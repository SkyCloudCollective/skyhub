//! Botanica — a granular sample-morph instrument.
//!
//! Inspired by Synplant; sound design by **Tev**. Original implementation. Built
//! entirely on `dsp-core`, so the web (wasm) build and the native plugin run the
//! same DSP. This file is the **engine skeleton + voice + FX rack**; the ported
//! advanced modules (resonance, transient arp, granular freeze, string layers,
//! XY characters) slot in next, each behind a clear parameter (the v1 complaint
//! was "too little control + chaotic"). Real-time contract: no alloc/lock in
//! `process`; macros are smoothed.

#![forbid(unsafe_code)]

use dsp_core::{Chorus, Delay, Drive, Oscillator, Reverb, Smoothed, Waveform};

/// User-facing macros, all normalised 0..1. (Characters/seed expand this later.)
#[derive(Clone, Copy, Debug)]
pub struct BotanicaParams {
    pub blend: f32,     // tone/drive blend
    pub intensity: f32, // output level / energy
    pub bloom: f32,     // reverb size + mix
    pub motion: f32,    // chorus/movement
    pub xy_x: f32,      // character morph X (reserved → characters)
    pub xy_y: f32,      // character morph Y (reserved → characters)
    pub freeze: f32,    // 0/1 granular freeze (reserved → freeze module)
}

impl Default for BotanicaParams {
    fn default() -> Self {
        Self {
            blend: 0.4,
            intensity: 0.7,
            bloom: 0.3,
            motion: 0.3,
            xy_x: 0.5,
            xy_y: 0.5,
            freeze: 0.0,
        }
    }
}

/// One loaded sample, looped with linear interpolation.
#[derive(Clone, Debug, Default)]
struct SampleVoice {
    data: Vec<f32>,
    src_rate: f32,
    pos: f32,
}

impl SampleVoice {
    fn loaded(&self) -> bool {
        !self.data.is_empty()
    }

    /// Advance and read one sample at the engine rate. `hold` freezes position.
    #[inline]
    fn next(&mut self, engine_rate: f32, hold: bool) -> f32 {
        let n = self.data.len();
        if n == 0 {
            return 0.0;
        }
        let i0 = self.pos.floor() as usize % n;
        let i1 = (i0 + 1) % n;
        let fr = self.pos - self.pos.floor();
        let out = self.data[i0] * (1.0 - fr) + self.data[i1] * fr;
        if !hold {
            let step = (self.src_rate / engine_rate).max(0.0);
            self.pos += step;
            if self.pos >= n as f32 {
                self.pos -= n as f32;
            }
        }
        out
    }
}

pub struct Engine {
    sample_rate: f32,
    voice: SampleVoice,
    fallback: Oscillator, // a gentle synth bed when no sample is loaded
    drive: Drive,
    chorus: Chorus,
    delay: Delay,
    reverb: Reverb,
    // smoothed macros (no clicks on automation)
    gain: Smoothed,
    drive_amt: Smoothed,
    verb_mix: Smoothed,
    params: BotanicaParams,
}

impl Engine {
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        let p = BotanicaParams::default();
        let mut e = Self {
            sample_rate: sr,
            voice: SampleVoice::default(),
            fallback: Oscillator::new(sr),
            drive: Drive::default(),
            chorus: Chorus::new(sr),
            delay: Delay::new(sr, 1.0),
            reverb: Reverb::new(sr),
            gain: Smoothed::new(p.intensity, 0.02, sr),
            drive_amt: Smoothed::new(1.0, 0.02, sr),
            verb_mix: Smoothed::new(p.bloom * 0.6, 0.03, sr),
            params: p,
        };
        e.delay.set(0.24, 0.25, 0.18, 0.3);
        e.apply_params();
        e
    }

    pub fn set_sample(&mut self, data: Vec<f32>, src_rate: f32) {
        self.voice = SampleVoice {
            data,
            src_rate: src_rate.max(1.0),
            pos: 0.0,
        };
    }

    pub fn clear_sample(&mut self) {
        self.voice = SampleVoice::default();
    }

    pub fn set_params(&mut self, p: BotanicaParams) {
        self.params = p;
        self.apply_params();
    }

    fn apply_params(&mut self) {
        let p = &self.params;
        // blend → drive (1..6), motion → chorus rate/depth, bloom → reverb size+mix.
        self.drive_amt
            .set_target(1.0 + p.blend.clamp(0.0, 1.0) * 5.0);
        self.gain.set_target(p.intensity.clamp(0.0, 1.0));
        self.chorus.rate_hz = 0.1 + p.motion.clamp(0.0, 1.0) * 3.0;
        self.chorus.depth_ms = 1.0 + p.motion.clamp(0.0, 1.0) * 6.0;
        let bloom = p.bloom.clamp(0.0, 1.0);
        self.reverb.set(bloom, 0.3, bloom * 0.6);
        self.verb_mix.set_target(bloom * 0.6);
    }

    /// Produce one mono sample. (Named `next` like the other generators; not an `Iterator`.)
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn next(&mut self) -> f32 {
        let hold = self.params.freeze >= 0.5;
        let dry = if self.voice.loaded() {
            self.voice.next(self.sample_rate, hold)
        } else {
            // a soft two-octave bed so the instrument is never silent
            0.4 * self.fallback.next(110.0, Waveform::Triangle)
        };

        self.drive.drive = self.drive_amt.next();
        let mut s = self.drive.process(dry);
        s = self.chorus.process(s);
        s = self.delay.process(s);
        s = self.reverb.process(s);
        s * self.gain.next()
    }

    /// Fill a mono buffer (helper for offline render / tests).
    pub fn process(&mut self, out: &mut [f32]) {
        for x in out.iter_mut() {
            *x = self.next();
        }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(b: &[f32]) -> f32 {
        (b.iter().map(|x| x * x).sum::<f32>() / b.len() as f32).sqrt()
    }

    #[test]
    fn makes_finite_bounded_sound_without_a_sample() {
        let mut e = Engine::new(48_000.0);
        let mut buf = vec![0.0; 48_000];
        e.process(&mut buf);
        assert!(buf.iter().all(|v| v.is_finite() && v.abs() <= 2.0));
        assert!(rms(&buf) > 0.01, "engine was silent");
    }

    #[test]
    fn loads_and_loops_a_sample() {
        let mut e = Engine::new(48_000.0);
        // a 1 kHz tone as the "sample"
        let sample: Vec<f32> = (0..4800)
            .map(|i| (core::f32::consts::TAU * 1000.0 * i as f32 / 48_000.0).sin())
            .collect();
        e.set_sample(sample, 48_000.0);
        let mut buf = vec![0.0; 48_000];
        e.process(&mut buf);
        assert!(buf.iter().all(|v| v.is_finite()));
        assert!(rms(&buf) > 0.05);
    }

    #[test]
    fn freeze_holds_position() {
        let mut e = Engine::new(48_000.0);
        let sample: Vec<f32> = (0..1000).map(|i| (i as f32 / 1000.0) * 2.0 - 1.0).collect();
        e.set_sample(sample, 48_000.0);
        let mut p = BotanicaParams {
            freeze: 1.0,
            intensity: 1.0,
            bloom: 0.0,
            motion: 0.0,
            blend: 0.0,
            ..BotanicaParams::default()
        };
        e.set_params(p);
        // settle the FX, then with freeze the dry source position should not advance
        let pos_before = e.voice.pos;
        for _ in 0..500 {
            e.next();
        }
        assert!(
            (e.voice.pos - pos_before).abs() < 1e-3,
            "freeze did not hold position"
        );
        p.freeze = 0.0;
        e.set_params(p);
        for _ in 0..500 {
            e.next();
        }
        assert!(
            e.voice.pos != pos_before,
            "position should advance when not frozen"
        );
    }

    #[test]
    fn intensity_scales_output() {
        let mut e = Engine::new(48_000.0);
        let loud = {
            e.set_params(BotanicaParams {
                intensity: 1.0,
                ..BotanicaParams::default()
            });
            let mut b = vec![0.0; 24_000];
            e.process(&mut b);
            rms(&b[12_000..])
        };
        let quiet = {
            e.set_params(BotanicaParams {
                intensity: 0.1,
                ..BotanicaParams::default()
            });
            let mut b = vec![0.0; 24_000];
            e.process(&mut b);
            rms(&b[12_000..])
        };
        assert!(
            loud > quiet * 2.0,
            "intensity did not scale output: loud {loud} quiet {quiet}"
        );
    }
}
