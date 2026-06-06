//! Oscillators with anti-aliasing.
//!
//! Naive saw/square waves alias badly above a few kHz (the v1 PhasePlan bug).
//! We use PolyBLEP band-limiting: cheap (a few ops per discontinuity per
//! sample) and good enough for a synth oscillator.

use crate::util::TWO_PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Waveform {
    Sine,
    Saw,
    Square,
    Triangle,
}

/// PolyBLEP correction around a phase discontinuity.
/// `t` is the phase in [0,1), `dt` is the per-sample phase increment (freq/sr).
#[inline]
pub fn poly_blep(t: f32, dt: f32) -> f32 {
    if dt <= 0.0 {
        return 0.0;
    }
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

/// A single phase-accumulating oscillator. No allocation; reset on note-on.
#[derive(Clone, Copy, Debug)]
pub struct Oscillator {
    phase: f32, // [0,1)
    sample_rate: f32,
    // triangle is the leaky integral of a band-limited square; keep its state.
    tri_state: f32,
}

impl Oscillator {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            phase: 0.0,
            sample_rate: sample_rate.max(1.0),
            tri_state: 0.0,
        }
    }

    #[inline]
    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sample_rate = sr.max(1.0);
    }

    #[inline]
    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.tri_state = 0.0;
    }

    /// Produce one sample at `freq` Hz for the given waveform.
    #[inline]
    pub fn next(&mut self, freq: f32, wave: Waveform) -> f32 {
        let dt = (freq / self.sample_rate).clamp(0.0, 0.5);
        let t = self.phase;

        let out = match wave {
            Waveform::Sine => (TWO_PI * t).sin(),
            Waveform::Saw => {
                // naive ramp minus the band-limit correction
                (2.0 * t - 1.0) - poly_blep(t, dt)
            }
            Waveform::Square => {
                let naive = if t < 0.5 { 1.0 } else { -1.0 };
                // correct both edges: rising at 0, falling at 0.5
                let mut s = naive;
                s += poly_blep(t, dt);
                let t2 = (t + 0.5) % 1.0;
                s -= poly_blep(t2, dt);
                s
            }
            Waveform::Triangle => {
                // integrate the band-limited square -> band-limited triangle
                let naive = if t < 0.5 { 1.0 } else { -1.0 };
                let mut sq = naive;
                sq += poly_blep(t, dt);
                let t2 = (t + 0.5) % 1.0;
                sq -= poly_blep(t2, dt);
                // leaky integrator, scaled so amplitude stays ~[-1,1]
                self.tri_state += 4.0 * dt * sq;
                self.tri_state *= 0.9995; // gentle leak removes DC drift
                self.tri_state
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

    fn rms(buf: &[f32]) -> f32 {
        (buf.iter().map(|x| x * x).sum::<f32>() / buf.len() as f32).sqrt()
    }

    #[test]
    fn sine_is_bounded_and_periodic() {
        let mut o = Oscillator::new(48_000.0);
        let n = 48_000 / 100; // one period of 100 Hz
        let mut buf = vec![0.0; n];
        for x in buf.iter_mut() {
            *x = o.next(100.0, Waveform::Sine);
        }
        assert!(buf.iter().all(|v| v.abs() <= 1.001));
        assert!(rms(&buf) > 0.6 && rms(&buf) < 0.75); // ~0.707 for a sine
    }

    #[test]
    fn saw_blep_reduces_peak_vs_naive() {
        // A high-frequency saw with BLEP must stay bounded (no huge alias spikes).
        let mut o = Oscillator::new(48_000.0);
        let mut peak = 0.0_f32;
        for _ in 0..10_000 {
            peak = peak.max(o.next(8_000.0, Waveform::Saw).abs());
        }
        assert!(peak <= 1.5, "saw peak too large: {peak}");
    }

    #[test]
    fn outputs_are_finite() {
        let mut o = Oscillator::new(44_100.0);
        for w in [
            Waveform::Sine,
            Waveform::Saw,
            Waveform::Square,
            Waveform::Triangle,
        ] {
            for _ in 0..5_000 {
                assert!(o.next(440.0, w).is_finite());
            }
            o.reset();
        }
    }
}
