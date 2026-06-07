//! Resonance bank — tuned band-pass resonators that add harmonic richness.
//!
//! A fixed bank of `N` `dsp-core` `Svf` band-passes tuned to a harmonic series
//! of the voice pitch, framed by an HP/LP pair, summed and mixed back by an
//! `amount`. De-chaos (from the port spec): the v1 read-modify-write Q-drift bug
//! is gone (Q is a single smoothed value, never accumulated); Q is bounded
//! [0.5, 24]; coefficients are only recomputed when the smoothed Q moves past an
//! epsilon; band states + accumulator flush denormals. Pre-sized in `new`.

use dsp_core::{clampf, flush_denormal, Smoothed, Svf, SvfMode};

const N: usize = 8;

pub struct ResonanceBank {
    bands: [Svf; N],
    band_hz: [f32; N],
    lo: Svf, // framing high-pass (res_lo)
    hi: Svf, // framing low-pass  (res_hi)
    sample_rate: f32,
    base_freq: f32,
    spread: f32,
    tilt: f32,
    lo_hz: f32,
    hi_hz: f32,
    amount_s: Smoothed,
    q_s: Smoothed,
    last_q: f32,
}

impl ResonanceBank {
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        let mut b = Self {
            bands: [Svf::new(sr); N],
            band_hz: [0.0; N],
            lo: Svf::new(sr),
            hi: Svf::new(sr),
            sample_rate: sr,
            base_freq: 110.0,
            spread: 1.0,
            tilt: 0.0,
            lo_hz: 80.0,
            hi_hz: 9000.0,
            amount_s: Smoothed::new(0.0, 0.03, sr),
            q_s: Smoothed::new(4.0, 0.12, sr), // de-chaos: smooth Q at the morph
            last_q: -1.0,
        };
        b.lo.set(b.lo_hz, 0.707);
        b.hi.set(b.hi_hz, 0.707);
        b.retune();
        b
    }

    /// Set the fundamental the bank tracks (off the hot loop).
    pub fn set_pitch(&mut self, freq: f32) {
        self.base_freq = clampf(freq, 20.0, self.sample_rate * 0.4);
        self.retune();
    }

    /// Frame + character. `amount`/`q` smooth toward target; structure params
    /// retune the bands immediately (called off the hot loop on param change).
    pub fn set_params(
        &mut self,
        amount: f32,
        q: f32,
        tilt: f32,
        spread: f32,
        lo_hz: f32,
        hi_hz: f32,
    ) {
        self.amount_s.set_target(clampf(amount, 0.0, 1.0));
        self.q_s.set_target(clampf(q, 0.5, 24.0));
        self.tilt = clampf(tilt, -1.0, 1.0);
        let new_spread = clampf(spread, 0.5, 2.0);
        let new_lo = clampf(lo_hz, 20.0, 2000.0);
        let new_hi = clampf(hi_hz, 1000.0, self.sample_rate * 0.45);
        if (new_spread - self.spread).abs() > 1e-4 {
            self.spread = new_spread;
            self.retune();
        }
        if (new_lo - self.lo_hz).abs() > 0.5 {
            self.lo_hz = new_lo;
            self.lo.set(self.lo_hz, 0.707);
        }
        if (new_hi - self.hi_hz).abs() > 0.5 {
            self.hi_hz = new_hi;
            self.hi.set(self.hi_hz, 0.707);
        }
    }

    fn retune(&mut self) {
        let q = self.q_s.value().clamp(0.5, 24.0);
        for k in 0..N {
            // harmonic series, mildly stretched by `spread` for inharmonic bells
            let mult = (k as f32 + 1.0).powf(self.spread);
            let f = clampf(self.base_freq * mult, self.lo_hz, self.hi_hz);
            self.band_hz[k] = f;
            self.bands[k].set(f, q);
        }
        self.last_q = q;
    }

    pub fn reset(&mut self) {
        for b in self.bands.iter_mut() {
            b.reset();
        }
        self.lo.reset();
        self.hi.reset();
    }

    #[inline]
    fn band_gain(&self, k: usize) -> f32 {
        // tilt > 0 brightens (favours higher bands), < 0 darkens
        let pos = k as f32 / (N as f32 - 1.0); // 0..1
        (1.0 + self.tilt * (pos - 0.5) * 2.0).max(0.0)
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let amount = self.amount_s.next();
        let q = self.q_s.next();
        // recompute Q only when it has actually moved (de-chaos: bounded churn)
        if (q - self.last_q).abs() > 0.05 {
            for k in 0..N {
                self.bands[k].set(self.band_hz[k], q);
            }
            self.last_q = q;
        }
        if amount <= 1e-4 {
            return x;
        }
        let framed = self
            .hi
            .process(self.lo.process(x, SvfMode::Highpass), SvfMode::Lowpass);
        let mut acc = 0.0;
        for k in 0..N {
            acc += self.bands[k].process(framed, SvfMode::Bandpass) * self.band_gain(k);
        }
        acc = flush_denormal(acc / N as f32);
        x + acc * amount * 2.0 // wet make-up so band-passes are audible against dry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(b: &[f32]) -> f32 {
        (b.iter().map(|x| x * x).sum::<f32>() / b.len() as f32).sqrt()
    }

    fn noise(n: usize) -> Vec<f32> {
        // deterministic pseudo-noise (no rng dependency)
        let mut s: u32 = 0x1234_5678;
        (0..n)
            .map(|_| {
                s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (s >> 9) as f32 / (1u32 << 23) as f32 * 2.0 - 1.0
            })
            .collect()
    }

    #[test]
    fn amount_zero_is_transparent() {
        let mut r = ResonanceBank::new(48_000.0);
        r.set_params(0.0, 6.0, 0.0, 1.0, 80.0, 9000.0);
        let inp = noise(2000);
        let out: Vec<f32> = inp.iter().map(|&x| r.process(x)).collect();
        // with amount→0 the output tracks the input closely (after the smoother settles)
        let diff: f32 = inp[500..]
            .iter()
            .zip(&out[500..])
            .map(|(a, b)| (a - b).abs())
            .sum();
        assert!(diff < 1.0, "amount=0 not transparent: {diff}");
    }

    #[test]
    fn resonance_adds_coloration() {
        let mut dry = ResonanceBank::new(48_000.0);
        dry.set_params(0.0, 6.0, 0.0, 1.0, 80.0, 9000.0);
        let mut wet = ResonanceBank::new(48_000.0);
        wet.set_pitch(220.0);
        wet.set_params(0.8, 10.0, 0.0, 1.0, 80.0, 9000.0);
        let inp = noise(8000);
        let do_: Vec<f32> = inp.iter().map(|&x| dry.process(x)).collect();
        let wo: Vec<f32> = inp.iter().map(|&x| wet.process(x)).collect();
        assert!(wo.iter().all(|v| v.is_finite()));
        // the resonated signal differs measurably from dry
        let d: f32 = do_[2000..]
            .iter()
            .zip(&wo[2000..])
            .map(|(a, b)| (a - b).abs())
            .sum();
        assert!(d > 5.0, "resonance changed nothing: {d}");
    }

    #[test]
    fn stable_at_max_q() {
        let mut r = ResonanceBank::new(48_000.0);
        r.set_pitch(440.0);
        r.set_params(1.0, 24.0, 0.5, 1.0, 80.0, 12000.0);
        let mut peak = 0.0f32;
        for i in 0..48_000 {
            let x = if i < 32 { 1.0 } else { 0.0 };
            let y = r.process(x);
            assert!(y.is_finite());
            peak = peak.max(y.abs());
        }
        assert!(peak < 20.0, "resonance bank unstable: peak {peak}");
    }
}
