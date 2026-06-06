//! State-variable filter (Andrew Simper / Cytomic TPT topology).
//!
//! One structure yields low-pass, high-pass, band-pass and notch simultaneously
//! and stays stable under fast cutoff/resonance modulation — exactly what an
//! open modulation matrix needs. Coefficients are recomputed only when cutoff
//! or Q change, never per sample unless modulated.

use crate::util::flush_denormal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SvfMode {
    Lowpass,
    Highpass,
    Bandpass,
    Notch,
}

#[derive(Clone, Copy, Debug)]
pub struct Svf {
    sample_rate: f32,
    // coefficients
    g: f32,
    k: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    // state
    ic1eq: f32,
    ic2eq: f32,
}

impl Svf {
    pub fn new(sample_rate: f32) -> Self {
        let mut f = Self {
            sample_rate: sample_rate.max(1.0),
            g: 0.0,
            k: 0.0,
            a1: 0.0,
            a2: 0.0,
            a3: 0.0,
            ic1eq: 0.0,
            ic2eq: 0.0,
        };
        f.set(1_000.0, 0.707);
        f
    }

    #[inline]
    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sample_rate = sr.max(1.0);
    }

    /// Set cutoff (Hz) and resonance Q (>= 0.5). Clamped to a stable range.
    pub fn set(&mut self, cutoff_hz: f32, q: f32) {
        let nyq = self.sample_rate * 0.5;
        let fc = cutoff_hz.clamp(10.0, nyq * 0.99);
        let q = q.max(0.025);
        self.g = (core::f32::consts::PI * fc / self.sample_rate).tan();
        self.k = 1.0 / q;
        self.a1 = 1.0 / (1.0 + self.g * (self.g + self.k));
        self.a2 = self.g * self.a1;
        self.a3 = self.g * self.a2;
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ic1eq = 0.0;
        self.ic2eq = 0.0;
    }

    /// Process one sample, returning the requested response.
    #[inline]
    pub fn process(&mut self, x: f32, mode: SvfMode) -> f32 {
        let v3 = x - self.ic2eq;
        let v1 = self.a1 * self.ic1eq + self.a2 * v3;
        let v2 = self.ic2eq + self.a2 * self.ic1eq + self.a3 * v3;
        self.ic1eq = flush_denormal(2.0 * v1 - self.ic1eq);
        self.ic2eq = flush_denormal(2.0 * v2 - self.ic2eq);

        match mode {
            SvfMode::Lowpass => v2,
            SvfMode::Highpass => x - self.k * v1 - v2,
            SvfMode::Bandpass => v1,
            SvfMode::Notch => x - self.k * v1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(buf: &[f32]) -> f32 {
        (buf.iter().map(|x| x * x).sum::<f32>() / buf.len() as f32).sqrt()
    }

    fn tone(freq: f32, sr: f32, n: usize) -> Vec<f32> {
        let mut p = 0.0_f32;
        let dt = freq / sr;
        (0..n)
            .map(|_| {
                let s = (core::f32::consts::TAU * p).sin();
                p = (p + dt) % 1.0;
                s
            })
            .collect()
    }

    #[test]
    fn lowpass_attenuates_highs() {
        let sr = 48_000.0;
        let mut f = Svf::new(sr);
        f.set(500.0, 0.707);
        let hi = tone(10_000.0, sr, 4096);
        let out: Vec<f32> = hi.iter().map(|&x| f.process(x, SvfMode::Lowpass)).collect();
        assert!(rms(&out) < rms(&hi) * 0.25, "highs not attenuated");
    }

    #[test]
    fn lowpass_passes_lows() {
        let sr = 48_000.0;
        let mut f = Svf::new(sr);
        f.set(2_000.0, 0.707);
        let lo = tone(100.0, sr, 4096);
        // settle, then measure
        for &x in &lo {
            f.process(x, SvfMode::Lowpass);
        }
        let out: Vec<f32> = lo.iter().map(|&x| f.process(x, SvfMode::Lowpass)).collect();
        assert!(rms(&out) > rms(&lo) * 0.7, "lows wrongly attenuated");
    }

    #[test]
    fn stays_finite_under_high_resonance() {
        let sr = 48_000.0;
        let mut f = Svf::new(sr);
        f.set(800.0, 20.0);
        let mut last = 0.0;
        for i in 0..48_000 {
            last = f.process(if i == 0 { 1.0 } else { 0.0 }, SvfMode::Bandpass);
            assert!(last.is_finite());
        }
        let _ = last;
    }
}
