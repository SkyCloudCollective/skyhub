//! A single PhasePlan voice — the full mono signal path plus modulation.
//!
//! Signal: [`OscSection`] → state-variable filter → amplifier (amp ADSR · vel).
//! Modulation: two LFOs, a dedicated mod-envelope and the amp envelope feed an
//! open [`ModMatrix`]; routes add `depth · source` onto a small set of targets
//! (pitch, cutoff, wavetable positions, amp, resonance, osc-mix) every sample.
//! A keyboard-tracking filter and a hardwired filter-envelope amount make the
//! voice musical even before any matrix route is configured.
//!
//! Real-time contract: nothing here allocates or locks; the playing pitch is
//! glided through a one-pole smoother and the filter coefficients are recomputed
//! only when the modulated cutoff/resonance actually move.

use crate::osc_section::{OscParams, OscSection};
use dsp_core::{Adsr, Lfo, ModMatrix, ModRoute, Smoothed, Svf, SvfMode, Waveform};

// ── Modulation source indices (what the matrix reads) ────────────────────────
pub const SRC_LFO1: usize = 0;
pub const SRC_LFO2: usize = 1;
pub const SRC_MOD_ENV: usize = 2;
pub const SRC_AMP_ENV: usize = 3;
pub const SRC_VELOCITY: usize = 4;
pub const SRC_KEY: usize = 5;
pub const N_SOURCES: usize = 6;

// ── Modulation target indices (what the matrix writes) ───────────────────────
pub const TGT_PITCH: usize = 0; // semitones
pub const TGT_CUTOFF: usize = 1; // octaves
pub const TGT_WT_A: usize = 2; // 0..1 morph delta
pub const TGT_WT_B: usize = 3; // 0..1 morph delta
pub const TGT_AMP: usize = 4; // linear gain delta around 1
pub const TGT_RESONANCE: usize = 5; // 0..1 resonance delta
pub const TGT_OSC_MIX: usize = 6; // 0..1 A↔B delta
pub const N_TARGETS: usize = 7;

/// One voice's parameters (snapshot; Copy so the engine can fan it out cheaply).
#[derive(Clone, Copy, Debug)]
pub struct VoiceParams {
    pub osc: OscParams,
    /// Base filter cutoff in Hz (before tracking/envelope/matrix).
    pub cutoff: f32,
    /// Resonance 0..1 (mapped to filter Q).
    pub resonance: f32,
    pub filter_mode: SvfMode,
    /// Keyboard tracking 0..1 (cutoff follows the played note).
    pub filter_kbd: f32,
    /// Hardwired filter-envelope amount, -1..1 (octaves = amount · 6 · mod-env).
    pub filter_env: f32,
    pub amp_a: f32,
    pub amp_d: f32,
    pub amp_s: f32,
    pub amp_r: f32,
    pub mod_a: f32,
    pub mod_d: f32,
    pub mod_s: f32,
    pub mod_r: f32,
    pub lfo1_rate: f32,
    pub lfo1_wave: Waveform,
    pub lfo2_rate: f32,
    pub lfo2_wave: Waveform,
    /// Portamento time in seconds (glide between successive notes on a voice).
    pub glide: f32,
}

impl Default for VoiceParams {
    fn default() -> Self {
        Self {
            osc: OscParams::default(),
            cutoff: 2_200.0,
            resonance: 0.18,
            filter_mode: SvfMode::Lowpass,
            filter_kbd: 0.3,
            filter_env: 0.5,
            amp_a: 0.005,
            amp_d: 0.15,
            amp_s: 0.7,
            amp_r: 0.25,
            mod_a: 0.002,
            mod_d: 0.20,
            mod_s: 0.0,
            mod_r: 0.20,
            lfo1_rate: 4.0,
            lfo1_wave: Waveform::Sine,
            lfo2_rate: 0.6,
            lfo2_wave: Waveform::Triangle,
            glide: 0.0,
        }
    }
}

/// MIDI note number → frequency in Hz (A4 = 69 = 440 Hz).
#[inline]
fn note_to_freq(note: f32) -> f32 {
    440.0 * (2.0_f32).powf((note - 69.0) / 12.0)
}

/// Resonance 0..1 → filter Q (gentle at the bottom, sharp near the top).
#[inline]
fn q_from_resonance(r: f32) -> f32 {
    let r = r.clamp(0.0, 1.0);
    0.5 + r * r * 18.0
}

pub struct Voice {
    sample_rate: f32,
    osc: OscSection,
    filter: Svf,
    amp_env: Adsr,
    mod_env: Adsr,
    lfo1: Lfo,
    lfo2: Lfo,
    matrix: ModMatrix,
    pitch: Smoothed, // glided MIDI note
    note: f32,
    vel: f32,
    params: VoiceParams,
    // filter-coefficient memo (epsilon-gated recompute)
    last_cutoff: f32,
    last_q: f32,
}

impl Voice {
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        let p = VoiceParams::default();
        let mut v = Self {
            sample_rate: sr,
            osc: OscSection::new(sr),
            filter: Svf::new(sr),
            amp_env: Adsr::new(sr),
            mod_env: Adsr::new(sr),
            lfo1: Lfo::new(sr),
            lfo2: Lfo::new(sr),
            matrix: ModMatrix::new(),
            pitch: Smoothed::new(60.0, 0.0, sr),
            note: 60.0,
            vel: 0.0,
            params: p,
            last_cutoff: -1.0,
            last_q: -1.0,
        };
        v.apply_params();
        v
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        let sr = sr.max(1.0);
        self.sample_rate = sr;
        self.osc.set_sample_rate(sr);
        self.filter.set_sample_rate(sr);
        self.amp_env.set_sample_rate(sr);
        self.mod_env.set_sample_rate(sr);
        self.lfo1.set_sample_rate(sr);
        self.lfo2.set_sample_rate(sr);
        self.apply_params();
    }

    pub fn set_params(&mut self, p: VoiceParams) {
        self.params = p;
        self.apply_params();
    }

    /// Replace the modulation routes (off the audio thread).
    pub fn set_routes(&mut self, routes: &[ModRoute]) {
        self.matrix.set_routes(routes);
    }

    fn apply_params(&mut self) {
        let p = self.params;
        self.amp_env.set(p.amp_a, p.amp_d, p.amp_s, p.amp_r);
        self.mod_env.set(p.mod_a, p.mod_d, p.mod_s, p.mod_r);
        self.pitch.set_time(p.glide, self.sample_rate);
    }

    /// Start (or retrigger) a note. A fresh voice snaps to pitch and resets its
    /// generators for a clean attack; a legato retrigger glides.
    pub fn note_on(&mut self, note: f32, vel: f32) {
        let fresh = !self.amp_env.is_active();
        self.note = note;
        self.vel = vel.clamp(0.0, 1.0);
        if fresh {
            self.pitch.reset(note);
            self.osc.reset();
            self.filter.reset();
            self.lfo1.reset(0.0);
            self.lfo2.reset(0.0);
        }
        self.pitch.set_target(note);
        self.amp_env.gate_on();
        self.mod_env.gate_on();
    }

    pub fn note_off(&mut self) {
        self.amp_env.gate_off();
        self.mod_env.gate_off();
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.amp_env.is_active()
    }

    #[inline]
    pub fn note(&self) -> f32 {
        self.note
    }

    /// Produce one mono sample for this voice (0.0 when idle).
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn next(&mut self) -> f32 {
        if !self.amp_env.is_active() {
            return 0.0;
        }
        let p = self.params;

        // ── modulation sources (advance the per-sample generators once each) ──
        let lfo1 = self.lfo1.next(p.lfo1_rate, p.lfo1_wave);
        let lfo2 = self.lfo2.next(p.lfo2_rate, p.lfo2_wave);
        let menv = self.mod_env.next();
        let aenv = self.amp_env.next();
        let key = (self.note - 60.0) / 24.0;
        let srcs = [lfo1, lfo2, menv, aenv, self.vel, key];

        let mut tgt = [0.0_f32; N_TARGETS];
        self.matrix.apply(&srcs, &mut tgt);

        // ── pitch ──
        let play_note = self.pitch.next() + tgt[TGT_PITCH];
        let freq = note_to_freq(play_note);

        // ── oscillator section (morph + mix modulated) ──
        let mut op = p.osc;
        op.wt_pos_a = (op.wt_pos_a + tgt[TGT_WT_A]).clamp(0.0, 1.0);
        op.wt_pos_b = (op.wt_pos_b + tgt[TGT_WT_B]).clamp(0.0, 1.0);
        op.osc_mix = (op.osc_mix + tgt[TGT_OSC_MIX]).clamp(0.0, 1.0);
        let raw = self.osc.next(freq, &op);

        // ── filter (keyboard tracking + hardwired env + matrix, all in octaves) ──
        let oct =
            p.filter_kbd * (self.note - 60.0) / 12.0 + p.filter_env * menv * 6.0 + tgt[TGT_CUTOFF];
        let cutoff = (p.cutoff * (2.0_f32).powf(oct)).clamp(20.0, self.sample_rate * 0.49);
        let q = q_from_resonance(p.resonance + tgt[TGT_RESONANCE]);
        if (cutoff - self.last_cutoff).abs() > self.last_cutoff * 0.001 + 0.01
            || (q - self.last_q).abs() > 1e-3
        {
            self.filter.set(cutoff, q);
            self.last_cutoff = cutoff;
            self.last_q = q;
        }
        let filtered = self.filter.process(raw, p.filter_mode);

        // ── amplifier ──
        let amp = (1.0 + tgt[TGT_AMP]).max(0.0);
        filtered * aenv * self.vel * amp
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

    fn render(v: &mut Voice, n: usize) -> Vec<f32> {
        (0..n).map(|_| v.next()).collect()
    }

    #[test]
    fn note_lifecycle_and_silence_after_release() {
        let mut v = Voice::new(48_000.0);
        assert!(!v.is_active());
        v.note_on(60.0, 1.0);
        assert!(v.is_active());
        let buf = render(&mut v, 24_000);
        assert!(buf.iter().all(|s| s.is_finite() && s.abs() <= 2.0));
        assert!(rms(&buf) > 0.01, "voice was silent while gated");
        v.note_off();
        // run well past the release time → back to idle and silent
        let tail = render(&mut v, 48_000);
        assert!(!v.is_active(), "voice never released");
        assert!(
            rms(&tail[tail.len() - 4096..]) < 1e-4,
            "release left a tail"
        );
    }

    #[test]
    fn velocity_scales_level() {
        let render_vel = |vel: f32| {
            let mut v = Voice::new(48_000.0);
            v.note_on(60.0, vel);
            // measure during sustain (skip the attack transient)
            let _ = render(&mut v, 4_000);
            rms(&render(&mut v, 8_000))
        };
        let soft = render_vel(0.25);
        let loud = render_vel(1.0);
        assert!(
            loud > soft * 2.0,
            "velocity did not scale: soft {soft} loud {loud}"
        );
    }

    #[test]
    fn lower_cutoff_darkens() {
        let render_cut = |cutoff: f32| {
            let mut v = Voice::new(48_000.0);
            v.set_params(VoiceParams {
                osc: OscParams {
                    wt_pos_a: 0.66, // saw-ish: plenty of harmonics
                    osc_mix: 0.0,
                    sub_level: 0.0,
                    noise_level: 0.0,
                    ..OscParams::default()
                },
                cutoff,
                resonance: 0.0,
                filter_kbd: 0.0,
                filter_env: 0.0,
                ..VoiceParams::default()
            });
            v.note_on(45.0, 1.0); // low note → many harmonics under 8 kHz
            let _ = render(&mut v, 4_000);
            rms(&render(&mut v, 8_000))
        };
        let dark = render_cut(180.0);
        let bright = render_cut(9_000.0);
        assert!(
            bright > dark * 1.4,
            "cutoff did not darken: dark {dark} bright {bright}"
        );
    }

    #[test]
    fn lfo_to_cutoff_modulates_output() {
        let mut v = Voice::new(48_000.0);
        v.set_params(VoiceParams {
            osc: OscParams {
                wt_pos_a: 0.66,
                osc_mix: 0.0,
                sub_level: 0.0,
                noise_level: 0.0,
                ..OscParams::default()
            },
            cutoff: 600.0,
            resonance: 0.3,
            filter_kbd: 0.0,
            filter_env: 0.0,
            amp_s: 1.0, // steady sustain so only the LFO moves things
            lfo1_rate: 3.0,
            ..VoiceParams::default()
        });
        v.set_routes(&[ModRoute {
            source: SRC_LFO1,
            target: TGT_CUTOFF,
            depth: 2.5, // ±2.5 octaves of sweep
        }]);
        v.note_on(50.0, 1.0);
        let _ = render(&mut v, 4_000);
        let buf = render(&mut v, 48_000);
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        for c in buf.chunks(2048) {
            let r = rms(c);
            lo = lo.min(r);
            hi = hi.max(r);
        }
        assert!(
            hi > lo * 1.5,
            "LFO→cutoff did not modulate: lo {lo} hi {hi}"
        );
    }

    #[test]
    fn matrix_indices_are_in_range() {
        // guard against an index drift between the consts and the arrays
        for s in [
            SRC_LFO1,
            SRC_LFO2,
            SRC_MOD_ENV,
            SRC_AMP_ENV,
            SRC_VELOCITY,
            SRC_KEY,
        ] {
            assert!(s < N_SOURCES);
        }
        for t in [
            TGT_PITCH,
            TGT_CUTOFF,
            TGT_WT_A,
            TGT_WT_B,
            TGT_AMP,
            TGT_RESONANCE,
            TGT_OSC_MIX,
        ] {
            assert!(t < N_TARGETS);
        }
    }
}
