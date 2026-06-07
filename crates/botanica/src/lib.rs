//! Botanica — a granular sample-morph instrument.
//!
//! Inspired by Synplant; sound design by **Tev**. Original implementation. Built
//! entirely on `dsp-core`, so the web (wasm) build and the native plugin run the
//! same DSP. The XY-character control surface (this commit) restores what v1's
//! Rust silently dropped — the puck drives both the character ring AND the
//! effective macros, with the de-chaos bounds from the port spec. The remaining
//! advanced modules (resonance, transient arp, granular freeze, string layers)
//! slot in next, each behind clear parameters. Real-time contract: no alloc/lock
//! in `process`; the puck + macros are smoothed.

#![forbid(unsafe_code)]

pub mod arp;
pub mod freeze;
pub mod resonance;
pub mod strings;
pub mod voice_characters;

use arp::Arp;
use dsp_core::{Chorus, Delay, Drive, Oscillator, Reverb, Smoothed, Waveform};
use freeze::GranularFreeze;
use resonance::ResonanceBank;
use strings::{StringBed, StringParams};
use voice_characters::{blend, eff_macros, CharacterFilter};

/// User-facing parameters. Macros are normalised 0..1 unless noted.
#[derive(Clone, Copy, Debug)]
pub struct BotanicaParams {
    pub blend: f32,           // tone/drive blend
    pub intensity: f32,       // energy knob (blended with the XY orb)
    pub bloom: f32,           // brightness/reverb knob (blended with the orb)
    pub motion: f32,          // movement knob (blended with the orb)
    pub xy_x: f32,            // XY puck X in [-1,1] (ring angle + intensity bias)
    pub xy_y: f32,            // XY puck Y in [-1,1] (ring angle + bloom bias)
    pub freeze: f32,          // 0/1 granular freeze (holds the loop; floors gain)
    pub retune_semis: f32,    // sample-loop pitch, clamped ratio 0.35..2.5
    pub character_q: f32,     // 0..1 push the character filter resonance
    pub filter_lfo_rate: f32, // bounded cutoff-LFO rate (Hz)
    pub xy_macro_mix: f32,    // 0..1 how much the orb biases the knobs
    pub resonance: f32,       // 0..1 harmonic resonance bank amount
    pub resonance_tilt: f32,  // -1..1 resonance brightness
    pub arp_amount: f32,      // 0..1 transient-arp level
    pub arp_density: f32,     // 0..1 arp trigger density
    pub freeze_size: f32,     // 0..1 granular grain size
    pub freeze_spray: f32,    // 0..1 granular position jitter
    pub strings_level: f32,   // 0..1 choir/string bed mix (0 = off)
    pub strings_air: f32,     // 0..1 breath/bow transient amount
    pub strings_density: f32, // 0..1 chord-fire probability bias
    pub strings_tone: f32,    // 0..1 dark→bright body of the bed
}

impl Default for BotanicaParams {
    fn default() -> Self {
        Self {
            blend: 0.4,
            intensity: 0.55,
            bloom: 0.65,
            motion: 0.55,
            xy_x: 0.0,
            xy_y: 0.0,
            freeze: 0.0,
            retune_semis: 0.0,
            character_q: 0.3,
            filter_lfo_rate: 0.12,
            xy_macro_mix: 0.45,
            resonance: 0.25,
            resonance_tilt: 0.0,
            arp_amount: 0.3,
            arp_density: 0.4,
            freeze_size: 0.5,
            freeze_spray: 0.3,
            strings_level: 0.0,
            strings_air: 0.4,
            strings_density: 0.4,
            strings_tone: 0.5,
        }
    }
}

/// One loaded sample, looped with linear interpolation + pitch ratio.
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

    #[inline]
    fn next(&mut self, engine_rate: f32, hold: bool, pitch_ratio: f32) -> f32 {
        let n = self.data.len();
        if n == 0 {
            return 0.0;
        }
        let i0 = self.pos.floor() as usize % n;
        let i1 = (i0 + 1) % n;
        let fr = self.pos - self.pos.floor();
        let out = self.data[i0] * (1.0 - fr) + self.data[i1] * fr;
        if !hold {
            let step = (self.src_rate / engine_rate * pitch_ratio).max(0.0);
            self.pos += step;
            while self.pos >= n as f32 {
                self.pos -= n as f32;
            }
        }
        out
    }
}

pub struct Engine {
    sample_rate: f32,
    voice: SampleVoice,
    fallback: Oscillator,
    char_filter: CharacterFilter,
    res: ResonanceBank,
    arp: Arp,
    freeze_mod: GranularFreeze,
    strings: StringBed,
    frozen: bool,
    drive: Drive,
    chorus: Chorus,
    delay: Delay,
    reverb: Reverb,
    // smoothed controls (no clicks)
    xy_x_s: Smoothed,
    xy_y_s: Smoothed,
    int_s: Smoothed,
    bloom_s: Smoothed,
    motion_s: Smoothed,
    drive_amt: Smoothed,
    gain: Smoothed,
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
            char_filter: CharacterFilter::new(sr),
            res: ResonanceBank::new(sr),
            arp: Arp::new(sr),
            freeze_mod: GranularFreeze::new(sr),
            strings: StringBed::new(sr),
            frozen: false,
            drive: Drive::default(),
            chorus: Chorus::new(sr),
            delay: Delay::new(sr, 1.0),
            reverb: Reverb::new(sr),
            xy_x_s: Smoothed::new(p.xy_x, 0.008, sr),
            xy_y_s: Smoothed::new(p.xy_y, 0.008, sr),
            int_s: Smoothed::new(p.intensity, 0.025, sr),
            bloom_s: Smoothed::new(p.bloom, 0.025, sr),
            motion_s: Smoothed::new(p.motion, 0.025, sr),
            drive_amt: Smoothed::new(1.0 + p.blend * 5.0, 0.02, sr),
            gain: Smoothed::new(p.intensity, 0.02, sr),
            params: p,
        };
        e.delay.set(0.24, 0.25, 0.18, 0.3);
        e.retarget();
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
        // freeze edge: capture a window on the rising edge (off the audio thread)
        let now = p.freeze >= 0.5;
        if now && !self.frozen {
            if self.voice.loaded() {
                let center = self.voice.pos as usize;
                self.freeze_mod.capture(&self.voice.data, center, 0.3);
                self.frozen = true;
            }
        } else if !now && self.frozen {
            self.freeze_mod.clear();
            self.frozen = false;
        }
        self.retarget();
    }

    /// Push parameter values to the smoothers (called off the hot loop on change).
    fn retarget(&mut self) {
        let p = self.params; // Copy — no borrow of self while we touch sub-modules
        self.xy_x_s.set_target(p.xy_x.clamp(-1.0, 1.0));
        self.xy_y_s.set_target(p.xy_y.clamp(-1.0, 1.0));
        self.int_s.set_target(p.intensity.clamp(0.0, 1.0));
        self.bloom_s.set_target(p.bloom.clamp(0.0, 1.0));
        self.motion_s.set_target(p.motion.clamp(0.0, 1.0));
        self.drive_amt
            .set_target(1.0 + p.blend.clamp(0.0, 1.0) * 5.0);
        // resonance bank tracks the voice pitch
        let ratio = (2.0_f32).powf(p.retune_semis / 12.0).clamp(0.35, 2.5);
        self.res.set_pitch(110.0 * ratio);
    }

    #[inline]
    fn pitch_ratio(&self) -> f32 {
        (2.0_f32)
            .powf(self.params.retune_semis / 12.0)
            .clamp(0.35, 2.5)
    }

    /// Produce one mono sample.
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn next(&mut self) -> f32 {
        let hold = self.params.freeze >= 0.5;

        // smoothed puck + knobs → blended character + effective macros
        let x = self.xy_x_s.next();
        let y = self.xy_y_s.next();
        let ik = self.int_s.next();
        let bk = self.bloom_s.next();
        let mk = self.motion_s.next();
        let bl = blend(x, y);
        let eff = eff_macros(x, y, ik, bk, mk, self.params.xy_macro_mix);

        // voice → character filter
        let dry = if self.voice.loaded() {
            self.voice.next(self.sample_rate, hold, self.pitch_ratio())
        } else {
            0.4 * self.fallback.next(110.0, Waveform::Triangle)
        };
        let shaped = self.char_filter.process(
            dry,
            &bl,
            eff.bloom,
            eff.motion,
            eff.intensity,
            self.params.character_q,
            self.params.filter_lfo_rate,
        );

        // harmonic resonance bank (amount + Q follow the character)
        let res_amt = (self.params.resonance * (0.4 + bl.res.amount * 2.0)).clamp(0.0, 1.0);
        let res_q = (2.0 + bl.res.q + self.params.character_q * 6.0).clamp(0.5, 24.0);
        self.res.set_params(
            res_amt,
            res_q,
            self.params.resonance_tilt,
            1.0,
            80.0,
            9000.0,
        );
        let voiced = self.res.process(shaped);

        // transient arp (key-locked sparkle; level + brightness follow the character)
        let arp_lvl =
            (self.params.arp_amount * (0.4 + bl.arp.level.max(0.0) * 2.0)).clamp(0.0, 1.0);
        self.arp.set_params(
            arp_lvl,
            self.params.arp_density,
            (0.2 + bl.arp.bright).clamp(0.0, 1.0),
            5.0,
            0.18,
            6,
        );
        // granular freeze (sustained pad from a captured window; clamped grain rate)
        let frz = if self.frozen || self.freeze_mod.is_active() {
            self.freeze_mod.set_params(
                self.params.freeze >= 0.5,
                0.2 + eff.depth * 0.7,
                self.params.freeze_size,
                0.5,
                self.params.freeze_spray,
                self.pitch_ratio(),
                0.3,
            );
            self.freeze_mod.process()
        } else {
            0.0
        };

        // choir/string bed (fired by the same transients; the character's strings
        // gene biases probability + airiness; skipped entirely when level is 0)
        let strings_bus = if self.params.strings_level > 1e-4 || self.strings.active_voices() > 0 {
            self.strings.set_params(
                StringParams {
                    level: self.params.strings_level,
                    airiness: (self.params.strings_air * (0.6 + bl.strings.airy.max(0.0) * 3.0))
                        .clamp(0.0, 1.0),
                    body_tone: self.params.strings_tone,
                    key: self.params.retune_semis.rem_euclid(12.0),
                    ..StringParams::default()
                },
                eff.intensity,
                eff.bloom,
                self.params.strings_density,
                bl.strings.prob.max(0.0),
            );
            self.strings.process(dry)
        } else {
            0.0
        };

        let pre_fx = voiced + self.arp.process(dry) * 0.6 + frz + strings_bus;

        // FX amounts driven by the effective macros + the character deltas
        self.drive.drive = self.drive_amt.next();
        self.chorus.rate_hz = 0.1 + eff.motion * 3.0;
        self.chorus.depth_ms = (1.0 + eff.motion * 6.0) * (1.0 + bl.fx.chorus).max(0.2);
        let dfb = (0.25 + bl.fx.delay).clamp(0.0, 0.9);
        self.delay
            .set(0.24, dfb, (0.18 + bl.fx.delay * 0.4).clamp(0.0, 0.5), 0.3);
        let verb = (eff.bloom * 0.55 + bl.fx.verb).clamp(0.0, 0.9);
        self.reverb.set(eff.bloom, 0.3, verb);

        let mut s = self.drive.process(pre_fx);
        s = self.chorus.process(s);
        s = self.delay.process(s);
        s = self.reverb.process(s);

        // freeze floors the gain so the loop sustains rather than going silent
        let target_gain = if hold {
            eff.intensity.max(0.15)
        } else {
            eff.intensity
        };
        self.gain.set_target(target_gain);
        s * self.gain.next()
    }

    /// Fill a mono buffer (offline render / tests).
    pub fn process(&mut self, out: &mut [f32]) {
        for x in out.iter_mut() {
            *x = self.next();
        }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// The currently-active character name (for UI display).
    pub fn character_name(&self) -> &'static str {
        blend(self.xy_x_s.value(), self.xy_y_s.value()).name
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
        assert!(buf.iter().all(|v| v.is_finite() && v.abs() <= 4.0));
        assert!(rms(&buf) > 0.005, "engine was silent");
    }

    #[test]
    fn strings_bed_adds_energy_and_stays_bounded() {
        // a transient-rich sample so the bed's onset gate fires
        let sample: Vec<f32> = (0..48_000)
            .map(|i| if i % 6000 == 0 { 0.9 } else { 0.0 })
            .collect();
        let render = |level: f32| {
            let mut e = Engine::new(48_000.0);
            e.set_sample(sample.clone(), 48_000.0);
            e.set_params(BotanicaParams {
                strings_level: level,
                ..BotanicaParams::default()
            });
            let mut buf = vec![0.0; 96_000];
            e.process(&mut buf);
            assert!(buf.iter().all(|v| v.is_finite() && v.abs() <= 4.0));
            rms(&buf)
        };
        let off = render(0.0);
        let on = render(0.9);
        assert!(on > off, "strings bed added no energy: off {off} on {on}");
    }

    #[test]
    fn loads_and_loops_a_sample() {
        let mut e = Engine::new(48_000.0);
        let sample: Vec<f32> = (0..4800)
            .map(|i| (core::f32::consts::TAU * 1000.0 * i as f32 / 48_000.0).sin())
            .collect();
        e.set_sample(sample, 48_000.0);
        let mut buf = vec![0.0; 48_000];
        e.process(&mut buf);
        assert!(buf.iter().all(|v| v.is_finite()));
        assert!(rms(&buf) > 0.02);
    }

    #[test]
    fn freeze_holds_position() {
        let mut e = Engine::new(48_000.0);
        let sample: Vec<f32> = (0..1000).map(|i| (i as f32 / 1000.0) * 2.0 - 1.0).collect();
        e.set_sample(sample, 48_000.0);
        e.set_params(BotanicaParams {
            freeze: 1.0,
            ..BotanicaParams::default()
        });
        // run a bit so the smoothed puck settles, then sample the held position
        for _ in 0..2000 {
            e.next();
        }
        let pos_before = e.voice.pos;
        for _ in 0..500 {
            e.next();
        }
        assert!(
            (e.voice.pos - pos_before).abs() < 1e-3,
            "freeze did not hold position"
        );
    }

    #[test]
    fn retune_changes_playback_speed() {
        let mut e = Engine::new(48_000.0);
        let sample: Vec<f32> = (0..1000).map(|i| (i as f32 / 1000.0)).collect();
        e.set_sample(sample.clone(), 48_000.0);
        // +12 semitones => ~2x speed => position advances ~2x further
        e.set_params(BotanicaParams {
            retune_semis: 12.0,
            ..BotanicaParams::default()
        });
        for _ in 0..400 {
            e.next();
        }
        let fast = e.voice.pos;
        e.set_sample(sample, 48_000.0);
        e.set_params(BotanicaParams {
            retune_semis: 0.0,
            ..BotanicaParams::default()
        });
        for _ in 0..400 {
            e.next();
        }
        let normal = e.voice.pos;
        assert!(
            fast > normal * 1.5,
            "retune did not speed up: fast {fast} normal {normal}"
        );
    }

    #[test]
    fn xy_selects_a_character() {
        let mut e = Engine::new(48_000.0);
        e.set_params(BotanicaParams {
            xy_x: 1.0,
            xy_y: 0.0,
            ..BotanicaParams::default()
        });
        for _ in 0..1000 {
            e.next();
        }
        let a = e.character_name();
        e.set_params(BotanicaParams {
            xy_x: -1.0,
            xy_y: 0.0,
            ..BotanicaParams::default()
        });
        for _ in 0..1000 {
            e.next();
        }
        let b = e.character_name();
        assert_ne!(a, b, "opposite puck positions gave the same character");
    }
}
