//! PhasePlan — a modular subtractive + wavetable synth with an open modulation
//! matrix.
//!
//! Inspired by Phase Plant; original implementation. Built entirely on
//! `dsp-core`, so the web (wasm) build and the native plugin run the same DSP.
//! v1's PhasePlan was a thin, aliasing, un-modulated stub; v2 rebuilds it as a
//! real instrument: anti-aliased dual wavetable oscillators ([`OscSection`]), a
//! TPT state-variable filter, two ADSRs and two LFOs feeding an open modulation
//! matrix ([`Voice`]), polyphony with voice-stealing, and a shared FX rack
//! (drive → chorus → delay → reverb) under a soft-clipped master ([`Engine`]).
//!
//! Real-time contract (see `dsp-core` crate docs): no alloc/lock/IO/panic in any
//! `process`/`next`; every user parameter that reaches the audio thread is
//! smoothed; the voice pool and effect buffers are sized once at construction;
//! modulation routes are configured off the audio thread.

#![forbid(unsafe_code)]

pub mod osc_section;
pub mod voice;

pub use osc_section::{OscParams, OscSection};
pub use voice::{Voice, VoiceParams};

use dsp_core::{Chorus, Delay, Drive, ModRoute, Reverb, Smoothed};

/// Default voice count (polyphony).
pub const POLYPHONY: usize = 8;

/// Instrument parameters: the per-voice patch plus the shared FX rack and
/// master. Copy, so the engine can fan it out to every voice cheaply.
#[derive(Clone, Copy, Debug)]
pub struct PhasePlanParams {
    pub voice: VoiceParams,
    /// Drive amount 0..1 (saturation + dry/wet).
    pub drive: f32,
    /// Chorus mix 0..1.
    pub chorus: f32,
    pub delay_time: f32,     // seconds
    pub delay_feedback: f32, // 0..1
    pub delay_mix: f32,      // 0..1
    pub reverb: f32,         // mix 0..1
    pub reverb_size: f32,    // 0..1
    pub master: f32,         // master gain 0..1
}

impl Default for PhasePlanParams {
    fn default() -> Self {
        Self {
            voice: VoiceParams::default(),
            drive: 0.0,
            chorus: 0.0,
            delay_time: 0.28,
            delay_feedback: 0.3,
            delay_mix: 0.0,
            reverb: 0.12,
            reverb_size: 0.5,
            master: 0.32,
        }
    }
}

/// The polyphonic instrument: a fixed voice pool + the shared FX rack.
pub struct Engine {
    sample_rate: f32,
    voices: Vec<Voice>,
    ages: Vec<u64>, // last note-on tick per voice (for stealing the oldest)
    age: u64,
    drive: Drive,
    chorus: Chorus,
    delay: Delay,
    reverb: Reverb,
    master: Smoothed,
    params: PhasePlanParams,
}

impl Engine {
    pub fn new(sample_rate: f32) -> Self {
        Self::with_voices(sample_rate, POLYPHONY)
    }

    pub fn with_voices(sample_rate: f32, voices: usize) -> Self {
        let sr = sample_rate.max(1.0);
        let n = voices.max(1);
        let p = PhasePlanParams::default();
        let mut e = Self {
            sample_rate: sr,
            voices: (0..n).map(|_| Voice::new(sr)).collect(),
            ages: vec![0; n],
            age: 0,
            drive: Drive::default(),
            chorus: Chorus::new(sr),
            delay: Delay::new(sr, 2.0),
            reverb: Reverb::new(sr),
            master: Smoothed::new(p.master, 0.01, sr),
            params: p,
        };
        e.apply_params();
        e
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        let sr = sr.max(1.0);
        self.sample_rate = sr;
        for v in self.voices.iter_mut() {
            v.set_sample_rate(sr);
        }
        self.chorus = Chorus::new(sr);
        self.delay = Delay::new(sr, 2.0);
        self.reverb = Reverb::new(sr);
        self.master = Smoothed::new(self.params.master, 0.01, sr);
        self.apply_params();
    }

    pub fn set_params(&mut self, p: PhasePlanParams) {
        self.params = p;
        self.apply_params();
    }

    /// Replace the modulation routes; fans out to every voice (off the audio thread).
    pub fn set_routes(&mut self, routes: &[ModRoute]) {
        for v in self.voices.iter_mut() {
            v.set_routes(routes);
        }
    }

    fn apply_params(&mut self) {
        let p = self.params;
        for v in self.voices.iter_mut() {
            v.set_params(p.voice);
        }
        // FX rack (cheap field writes / coefficient updates — off the hot loop)
        let drv = p.drive.clamp(0.0, 1.0);
        self.drive.drive = 1.0 + drv * 8.0;
        self.drive.mix = drv;
        self.chorus.mix = p.chorus.clamp(0.0, 1.0);
        self.chorus.rate_hz = 0.6;
        self.chorus.depth_ms = 4.0;
        self.delay
            .set(p.delay_time, p.delay_feedback, p.delay_mix, 0.3);
        self.reverb
            .set(p.reverb_size, 0.35, p.reverb.clamp(0.0, 1.0));
        self.master.set_target(p.master.clamp(0.0, 1.0));
    }

    /// Start a note. Reuses an idle voice if one exists, otherwise steals the
    /// oldest (its envelope re-gates and pitch snaps — click-free, no realloc).
    pub fn note_on(&mut self, note: f32, vel: f32) {
        self.age = self.age.wrapping_add(1);
        let idx = self
            .voices
            .iter()
            .position(|v| !v.is_active())
            .unwrap_or_else(|| {
                // steal the oldest active voice
                let mut oldest = 0;
                for i in 1..self.ages.len() {
                    if self.ages[i] < self.ages[oldest] {
                        oldest = i;
                    }
                }
                oldest
            });
        self.voices[idx].note_on(note, vel);
        self.ages[idx] = self.age;
    }

    /// Release every voice currently sounding `note`.
    pub fn note_off(&mut self, note: f32) {
        for v in self.voices.iter_mut() {
            if v.is_active() && (v.note() - note).abs() < 0.5 {
                v.note_off();
            }
        }
    }

    pub fn all_notes_off(&mut self) {
        for v in self.voices.iter_mut() {
            v.note_off();
        }
    }

    /// Number of voices currently producing sound (for the UI).
    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.is_active()).count()
    }

    #[inline]
    fn next_sample(&mut self) -> f32 {
        let mut s = 0.0;
        for v in self.voices.iter_mut() {
            s += v.next();
        }
        s = self.drive.process(s);
        s = self.chorus.process(s);
        s = self.delay.process(s);
        s = self.reverb.process(s);
        // soft-clip the summed polyphony so the master can never blow up
        (s * self.master.next()).tanh()
    }

    /// Fill a mono buffer (offline render / tests / the worklet block).
    pub fn process(&mut self, out: &mut [f32]) {
        for x in out.iter_mut() {
            *x = self.next_sample();
        }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::{SRC_LFO1, TGT_AMP};

    fn rms(b: &[f32]) -> f32 {
        (b.iter().map(|x| x * x).sum::<f32>() / b.len() as f32).sqrt()
    }

    #[test]
    fn chord_is_bounded_and_audible() {
        let mut e = Engine::new(48_000.0);
        for n in [48.0, 52.0, 55.0] {
            e.note_on(n, 0.9);
        }
        assert_eq!(e.active_voices(), 3);
        let mut buf = vec![0.0; 48_000];
        e.process(&mut buf);
        assert!(buf.iter().all(|s| s.is_finite() && s.abs() <= 1.0001));
        assert!(rms(&buf) > 0.02, "chord was silent");
    }

    #[test]
    fn more_notes_than_voices_is_safe() {
        let mut e = Engine::with_voices(48_000.0, 8);
        for n in 36..52 {
            e.note_on(n as f32, 0.8); // 16 notes onto 8 voices → steals
        }
        assert!(e.active_voices() <= 8);
        let mut buf = vec![0.0; 24_000];
        e.process(&mut buf); // must not panic and stays bounded
        assert!(buf.iter().all(|s| s.is_finite() && s.abs() <= 1.0001));
    }

    #[test]
    fn release_returns_to_silence() {
        let mut e = Engine::new(48_000.0);
        e.note_on(60.0, 1.0);
        let mut buf = vec![0.0; 24_000];
        e.process(&mut buf);
        assert!(rms(&buf) > 0.02);
        e.note_off(60.0);
        let mut tail = vec![0.0; 96_000];
        e.process(&mut tail);
        assert_eq!(e.active_voices(), 0, "note never released");
        assert!(
            rms(&tail[tail.len() - 8192..]) < 1e-3,
            "left an audible tail"
        );
    }

    #[test]
    fn master_gain_zero_is_silent() {
        let mut e = Engine::new(48_000.0);
        e.set_params(PhasePlanParams {
            master: 0.0,
            ..PhasePlanParams::default()
        });
        e.note_on(60.0, 1.0);
        let mut buf = vec![0.0; 48_000];
        e.process(&mut buf);
        assert!(rms(&buf[buf.len() - 8192..]) < 1e-4, "master 0 not silent");
    }

    #[test]
    fn routes_fan_out_to_voices() {
        // An LFO→amp route at high depth makes the output strongly non-stationary
        // (tremolo); without it the sustain is steady.
        let render = |routed: bool| {
            let mut e = Engine::new(48_000.0);
            e.set_params(PhasePlanParams {
                voice: VoiceParams {
                    amp_s: 1.0,
                    lfo1_rate: 5.0,
                    ..VoiceParams::default()
                },
                reverb: 0.0,
                ..PhasePlanParams::default()
            });
            if routed {
                e.set_routes(&[ModRoute {
                    source: SRC_LFO1,
                    target: TGT_AMP,
                    depth: 0.9,
                }]);
            }
            e.note_on(57.0, 1.0);
            let mut buf = vec![0.0; 48_000];
            e.process(&mut buf);
            let mut lo = f32::INFINITY;
            let mut hi = f32::NEG_INFINITY;
            for c in buf[4_000..].chunks(2048) {
                let r = rms(c);
                lo = lo.min(r);
                hi = hi.max(r);
            }
            (hi - lo) / hi.max(1e-6)
        };
        let steady = render(false);
        let tremolo = render(true);
        assert!(
            tremolo > steady + 0.2,
            "route had no effect: steady {steady} tremolo {tremolo}"
        );
    }
}
