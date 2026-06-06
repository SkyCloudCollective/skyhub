//! LFO — a slow modulation source for the matrix.
//!
//! Phase can be reset (key-sync) or free-running. Rate is in Hz; a host can
//! pre-multiply a tempo-synced rate before calling. Output is bipolar [-1,1];
//! callers scale/offset for the target parameter.

use crate::osc::{poly_blep, Waveform};
use crate::util::TWO_PI;

#[derive(Clone, Copy, Debug)]
pub struct Lfo {
    phase: f32,
    sample_rate: f32,
}

impl Lfo {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            phase: 0.0,
            sample_rate: sample_rate.max(1.0),
        }
    }

    #[inline]
    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sample_rate = sr.max(1.0);
    }

    #[inline]
    pub fn reset(&mut self, phase: f32) {
        self.phase = phase.rem_euclid(1.0);
    }

    /// One bipolar [-1,1] sample at `rate_hz`. Saw/square use light band-limiting
    /// (an LFO at audio-ish rates shouldn't spit aliasing into a modulation path).
    #[inline]
    pub fn next(&mut self, rate_hz: f32, wave: Waveform) -> f32 {
        let dt = (rate_hz / self.sample_rate).clamp(0.0, 0.5);
        let t = self.phase;
        let out = match wave {
            Waveform::Sine => (TWO_PI * t).sin(),
            Waveform::Triangle => 1.0 - 4.0 * (t - 0.5).abs(), // exact bipolar triangle
            Waveform::Saw => (2.0 * t - 1.0) - poly_blep(t, dt),
            Waveform::Square => {
                let naive = if t < 0.5 { 1.0 } else { -1.0 };
                naive + poly_blep(t, dt) - poly_blep((t + 0.5) % 1.0, dt)
            }
        };
        self.phase += dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bipolar_and_bounded() {
        let mut l = Lfo::new(48_000.0);
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        for _ in 0..48_000 {
            let v = l.next(5.0, Waveform::Sine);
            lo = lo.min(v);
            hi = hi.max(v);
            assert!(v.is_finite());
        }
        assert!(lo < -0.9 && hi > 0.9, "range too small: {lo}..{hi}");
    }

    #[test]
    fn triangle_is_exact_bipolar() {
        let mut l = Lfo::new(1000.0);
        // peak at phase 0.5 should be ~ +1, trough at 0.0 ~ -1
        l.reset(0.5);
        assert!((l.next(0.0, Waveform::Triangle) - 1.0).abs() < 1e-3);
    }
}
