//! ADSR envelope — exponential-ish segments via per-stage one-pole targets.
//!
//! A modulation source for the matrix. Times are in seconds; levels in [0,1].

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Clone, Copy, Debug)]
pub struct Adsr {
    sample_rate: f32,
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    stage: EnvStage,
    level: f32,
}

impl Adsr {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate: sample_rate.max(1.0),
            attack: 0.005,
            decay: 0.1,
            sustain: 0.8,
            release: 0.2,
            stage: EnvStage::Idle,
            level: 0.0,
        }
    }

    #[inline]
    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sample_rate = sr.max(1.0);
    }

    /// Set ADSR. Times in seconds, sustain level in [0,1].
    pub fn set(&mut self, attack: f32, decay: f32, sustain: f32, release: f32) {
        self.attack = attack.max(0.0);
        self.decay = decay.max(0.0);
        self.sustain = sustain.clamp(0.0, 1.0);
        self.release = release.max(0.0);
    }

    #[inline]
    pub fn gate_on(&mut self) {
        self.stage = EnvStage::Attack;
    }

    #[inline]
    pub fn gate_off(&mut self) {
        if self.stage != EnvStage::Idle {
            self.stage = EnvStage::Release;
        }
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.stage != EnvStage::Idle
    }

    #[inline]
    pub fn stage(&self) -> EnvStage {
        self.stage
    }

    /// Per-sample one-pole coefficient reaching ~99.9% over `time_s`.
    #[inline]
    fn coeff(&self, time_s: f32) -> f32 {
        if time_s <= 0.0 {
            0.0
        } else {
            (-6.9 / (time_s * self.sample_rate)).exp() // ~ -60 dB over time_s
        }
    }

    /// Advance one sample, returning the current envelope level [0,1].
    /// (Named `next` for consistency with the other one-sample generators; it
    /// is not an `Iterator`.)
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn next(&mut self) -> f32 {
        match self.stage {
            EnvStage::Idle => {
                self.level = 0.0;
            }
            EnvStage::Attack => {
                let c = self.coeff(self.attack);
                self.level = 1.0 + (self.level - 1.0) * c;
                if self.level >= 0.999 {
                    self.level = 1.0;
                    self.stage = EnvStage::Decay;
                }
            }
            EnvStage::Decay => {
                let c = self.coeff(self.decay);
                self.level = self.sustain + (self.level - self.sustain) * c;
                if (self.level - self.sustain).abs() < 1e-3 {
                    self.level = self.sustain;
                    self.stage = EnvStage::Sustain;
                }
            }
            EnvStage::Sustain => {
                self.level = self.sustain;
            }
            EnvStage::Release => {
                let c = self.coeff(self.release);
                self.level *= c;
                if self.level <= 1e-4 {
                    self.level = 0.0;
                    self.stage = EnvStage::Idle;
                }
            }
        }
        self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_cycle_runs_through_stages() {
        let sr = 48_000.0;
        let mut e = Adsr::new(sr);
        e.set(0.01, 0.05, 0.5, 0.05);
        e.gate_on();
        // attack should climb toward 1
        let mut peak = 0.0_f32;
        for _ in 0..(sr as usize / 50) {
            peak = peak.max(e.next());
        }
        assert!(peak > 0.9, "attack did not reach peak: {peak}");
        // hold sustain
        for _ in 0..(sr as usize / 10) {
            e.next();
        }
        assert!((e.next() - 0.5).abs() < 0.05, "did not settle at sustain");
        // release back to idle
        e.gate_off();
        for _ in 0..sr as usize {
            e.next();
        }
        assert_eq!(e.stage(), EnvStage::Idle);
        assert!(!e.is_active());
    }

    #[test]
    fn idle_outputs_zero() {
        let mut e = Adsr::new(48_000.0);
        for _ in 0..10 {
            assert_eq!(e.next(), 0.0);
        }
    }
}
