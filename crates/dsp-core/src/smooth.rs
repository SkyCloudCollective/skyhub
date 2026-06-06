//! Parameter smoothing — the cheapest insurance against zipper noise/clicks.
//!
//! A one-pole smoother glides the current value toward a target over a chosen
//! time constant. Every user-facing parameter that reaches the audio thread
//! goes through one of these (the real-time contract, see crate docs).

/// One-pole exponential smoother.
#[derive(Clone, Copy, Debug)]
pub struct Smoothed {
    current: f32,
    target: f32,
    coeff: f32, // per-sample pole coefficient in [0,1)
}

impl Smoothed {
    /// Create a smoother that reaches ~63% of a step in `time_s` seconds.
    pub fn new(initial: f32, time_s: f32, sample_rate: f32) -> Self {
        let mut s = Self {
            current: initial,
            target: initial,
            coeff: 0.0,
        };
        s.set_time(time_s, sample_rate);
        s
    }

    /// Recompute the coefficient (call on sample-rate or time change; not in the hot loop).
    pub fn set_time(&mut self, time_s: f32, sample_rate: f32) {
        if time_s <= 0.0 || sample_rate <= 0.0 {
            self.coeff = 0.0; // instantaneous
        } else {
            // coeff = exp(-1 / (time * sr))
            self.coeff = (-1.0 / (time_s * sample_rate)).exp();
        }
    }

    /// Set the destination value (cheap; safe every sample/block).
    #[inline]
    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Jump instantly (e.g. on note-on reset), skipping the glide.
    #[inline]
    pub fn reset(&mut self, value: f32) {
        self.current = value;
        self.target = value;
    }

    /// Advance one sample and return the smoothed value.
    /// (Named `next` for consistency with the other generators; not an `Iterator`.)
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn next(&mut self) -> f32 {
        self.current = self.target + (self.current - self.target) * self.coeff;
        self.current
    }

    #[inline]
    pub fn value(&self) -> f32 {
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converges_toward_target() {
        let mut s = Smoothed::new(0.0, 0.01, 48_000.0);
        s.set_target(1.0);
        let mut last = 0.0;
        for _ in 0..48_000 {
            last = s.next();
        }
        assert!((last - 1.0).abs() < 1e-3, "did not converge: {last}");
    }

    #[test]
    fn zero_time_is_instant() {
        let mut s = Smoothed::new(0.0, 0.0, 48_000.0);
        s.set_target(0.7);
        assert!((s.next() - 0.7).abs() < 1e-6);
    }

    #[test]
    fn reset_skips_glide() {
        let mut s = Smoothed::new(0.0, 1.0, 48_000.0);
        s.reset(0.5);
        assert_eq!(s.value(), 0.5);
        assert!((s.next() - 0.5).abs() < 1e-6);
    }
}
