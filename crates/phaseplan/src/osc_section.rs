//! The oscillator section — PhasePlan's generator stack.
//!
//! Two band-limited wavetable oscillators (A, B with coarse/fine detune and a
//! morph position each), a square sub one octave down, and a white-noise source,
//! summed by a small mixer. This is the raw material the filter then shapes.
//!
//! Everything here is allocation-free after construction and bounded: the
//! wavetables are additive (band-limited by construction) and the noise PRNG is
//! a stack-resident xorshift. v1's PhasePlan had a single naive oscillator with
//! no anti-aliasing and no morph — this is the deeper, stable replacement.

use dsp_core::{Oscillator, Waveform, Wavetable};

/// Oscillator-section parameters (snapshot; cheap to copy each block).
#[derive(Clone, Copy, Debug)]
pub struct OscParams {
    /// Osc A wavetable morph position, 0..1 (sine → triangle → saw → square).
    pub wt_pos_a: f32,
    /// Osc B wavetable morph position, 0..1.
    pub wt_pos_b: f32,
    /// Osc B coarse pitch offset in semitones (e.g. -24..24).
    pub coarse_b: f32,
    /// Osc B fine detune in cents (the beating/super-saw character).
    pub detune_cents: f32,
    /// A↔B blend, 0 = only A, 1 = only B.
    pub osc_mix: f32,
    /// Square sub-oscillator level (one octave below A), 0..1.
    pub sub_level: f32,
    /// White-noise level, 0..1.
    pub noise_level: f32,
}

impl Default for OscParams {
    fn default() -> Self {
        Self {
            wt_pos_a: 0.5,
            wt_pos_b: 0.66,
            coarse_b: 0.0,
            detune_cents: 8.0,
            osc_mix: 0.4,
            sub_level: 0.3,
            noise_level: 0.0,
        }
    }
}

/// Stack-resident xorshift32 — a deterministic, allocation-free noise source.
#[derive(Clone, Copy, Debug)]
struct Noise {
    state: u32,
}

impl Noise {
    fn new(seed: u32) -> Self {
        Self {
            state: seed | 1, // never zero (xorshift fixed point)
        }
    }

    #[inline]
    fn next(&mut self) -> f32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        // map u32 → [-1, 1]
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// The generator stack for one voice.
#[derive(Clone, Debug)]
pub struct OscSection {
    wt_a: Wavetable,
    wt_b: Wavetable,
    sub: Oscillator,
    noise: Noise,
}

impl OscSection {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            wt_a: Wavetable::basic(sample_rate),
            wt_b: Wavetable::basic(sample_rate),
            sub: Oscillator::new(sample_rate),
            noise: Noise::new(0x9E37_79B9),
        }
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.wt_a.set_sample_rate(sr);
        self.wt_b.set_sample_rate(sr);
        self.sub.set_sample_rate(sr);
    }

    /// Reset the oscillator phases (clean attack on a fresh note-on).
    pub fn reset(&mut self) {
        self.wt_a.reset();
        self.wt_b.reset();
        self.sub.reset();
    }

    /// One sample at fundamental `freq` Hz, given the section parameters.
    #[inline]
    pub fn next(&mut self, freq: f32, p: &OscParams) -> f32 {
        let a = self.wt_a.next(freq, p.wt_pos_a);

        let ratio_b = exp2_semis(p.coarse_b) * exp2_semis(p.detune_cents / 100.0);
        let b = self.wt_b.next(freq * ratio_b, p.wt_pos_b);

        let sub = self.sub.next(freq * 0.5, Waveform::Square);
        let noise = self.noise.next();

        let osc = a * (1.0 - p.osc_mix) + b * p.osc_mix;
        // Leave headroom so a fully-stacked voice (osc + sub + noise) stays ~[-1,1].
        (osc + sub * p.sub_level + noise * p.noise_level) * 0.7
    }
}

/// 2^(semitones/12) — pitch ratio for a semitone offset.
#[inline]
fn exp2_semis(semitones: f32) -> f32 {
    (2.0_f32).powf(semitones / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(b: &[f32]) -> f32 {
        (b.iter().map(|x| x * x).sum::<f32>() / b.len() as f32).sqrt()
    }

    fn render(osc: &mut OscSection, freq: f32, p: &OscParams, n: usize) -> Vec<f32> {
        (0..n).map(|_| osc.next(freq, p)).collect()
    }

    #[test]
    fn bounded_and_finite_across_positions() {
        let mut o = OscSection::new(48_000.0);
        for pos in [0.0, 0.33, 0.66, 1.0] {
            o.reset();
            let p = OscParams {
                wt_pos_a: pos,
                wt_pos_b: pos,
                sub_level: 0.5,
                noise_level: 0.2,
                ..OscParams::default()
            };
            let buf = render(&mut o, 220.0, &p, 4800);
            assert!(buf.iter().all(|v| v.is_finite() && v.abs() <= 1.2));
            assert!(rms(&buf) > 0.05, "section silent at pos {pos}");
        }
    }

    #[test]
    fn morph_position_changes_timbre() {
        let mut o = OscSection::new(48_000.0);
        let base = OscParams {
            osc_mix: 0.0, // isolate osc A
            sub_level: 0.0,
            noise_level: 0.0,
            ..OscParams::default()
        };
        let sine = render(
            &mut o,
            110.0,
            &OscParams {
                wt_pos_a: 0.0,
                ..base
            },
            2048,
        );
        o.reset();
        let saw = render(
            &mut o,
            110.0,
            &OscParams {
                wt_pos_a: 0.66,
                ..base
            },
            2048,
        );
        let diff: f32 = sine.iter().zip(&saw).map(|(a, b)| (a - b).abs()).sum();
        assert!(
            diff > 40.0,
            "wt position did not change the waveform: {diff}"
        );
    }

    #[test]
    fn detune_creates_beating() {
        // Two oscillators a few cents apart at an equal mix should beat: the
        // short-term envelope swells and dips rather than holding constant.
        let mut o = OscSection::new(48_000.0);
        let p = OscParams {
            osc_mix: 0.5,
            coarse_b: 0.0,
            detune_cents: 12.0,
            sub_level: 0.0,
            noise_level: 0.0,
            ..OscParams::default()
        };
        let buf = render(&mut o, 220.0, &p, 48_000);
        let win = 1024;
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        for c in buf.chunks(win) {
            let r = rms(c);
            lo = lo.min(r);
            hi = hi.max(r);
        }
        assert!(hi > lo * 1.3, "no beating from detune: lo {lo} hi {hi}");
    }

    #[test]
    fn sub_adds_low_end() {
        let mut o = OscSection::new(48_000.0);
        let no_sub = OscParams {
            sub_level: 0.0,
            noise_level: 0.0,
            ..OscParams::default()
        };
        let with_sub = OscParams {
            sub_level: 0.9,
            ..no_sub
        };
        let a = render(&mut o, 220.0, &no_sub, 4096);
        o.reset();
        let b = render(&mut o, 220.0, &with_sub, 4096);
        assert!(rms(&b) > rms(&a), "sub did not add energy");
    }
}
