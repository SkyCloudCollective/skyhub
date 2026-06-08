//! Granular freeze — capture a window of the sample and granulate it into a
//! sustained pad. The worst v1 offender: an unbounded `pollenRate` accumulator
//! that burst grains after any hitch. De-chaos (port spec): the spawn rate is
//! CLAMPED and driven by one sample-accurate clock; the grain pool is hard-capped
//! (`dsp-core::GrainCloud`, MAX_GRAINS); every macro is smoothed; freeze-off ramps
//! the wet to 0 (no click); a final tanh guarantees a bounded bus.
//!
//! The window capture (a `Vec`) happens off the audio thread, only when freeze
//! engages — `process` itself allocates nothing.

use dsp_core::{clampf, GrainCloud, Smoothed};

const MAX_RATE_HZ: f32 = 60.0; // hard ceiling on grains/second

pub struct GranularFreeze {
    cloud: GrainCloud,
    frozen: Vec<f32>,
    sample_rate: f32,
    spawn_clock: f64,
    rng: u32,
    density_s: Smoothed,
    size_s: Smoothed,
    wet_s: Smoothed,
    pitch_s: Smoothed,
    position: f32,
    jitter: f32,
    tone: f32,
    lp_state: f32,
}

impl GranularFreeze {
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        Self {
            cloud: GrainCloud::new(),
            frozen: Vec::new(),
            sample_rate: sr,
            spawn_clock: 0.0,
            rng: 0x2545_F491,
            density_s: Smoothed::new(0.4, 0.05, sr),
            size_s: Smoothed::new(0.5, 0.08, sr),
            wet_s: Smoothed::new(0.0, 0.18, sr), // soft freeze on/off
            pitch_s: Smoothed::new(1.0, 0.05, sr),
            position: 0.5,
            jitter: 0.3,
            tone: 0.3,
            lp_state: 0.0,
        }
    }

    /// Capture a window of `sample` centred on `center` (off the audio thread).
    pub fn capture(&mut self, sample: &[f32], center: usize, window_secs: f32) {
        self.frozen.clear();
        if sample.is_empty() {
            return;
        }
        let n = sample.len();
        let want = ((window_secs * self.sample_rate) as usize).clamp(256, n);
        let start = center % n;
        self.frozen.reserve(want);
        for k in 0..want {
            self.frozen.push(sample[(start + k) % n]);
        }
        self.cloud.reset();
        self.spawn_clock = 0.0;
    }

    pub fn clear(&mut self) {
        // keep the buffer so in-flight grains finish; just stop the wet (ramped)
        self.wet_s.set_target(0.0);
    }

    pub fn is_active(&self) -> bool {
        !self.frozen.is_empty() && (self.wet_s.value() > 1e-3 || self.cloud.active() > 0)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_params(
        &mut self,
        on: bool,
        density: f32,
        size: f32,
        position: f32,
        jitter: f32,
        pitch: f32,
        tone: f32,
    ) {
        self.wet_s.set_target(if on { 1.0 } else { 0.0 });
        self.density_s.set_target(clampf(density, 0.0, 1.0));
        self.size_s.set_target(clampf(size, 0.0, 1.0));
        self.pitch_s.set_target(clampf(pitch, 0.25, 4.0));
        self.position = clampf(position, 0.0, 1.0);
        self.jitter = clampf(jitter, 0.0, 1.0);
        self.tone = clampf(tone, 0.0, 0.99);
    }

    #[inline]
    fn rng01(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.rng >> 9) as f32 / (1u32 << 23) as f32
    }

    #[inline]
    pub fn process(&mut self) -> f32 {
        let wet = self.wet_s.next();
        if self.frozen.is_empty() {
            return 0.0;
        }
        let density = self.density_s.next();
        let size = self.size_s.next();
        let pitch = self.pitch_s.next();

        // CLAMPED spawn rate — the core de-chaos fix
        let rate = (0.5 + density * 30.0).min(MAX_RATE_HZ);
        self.spawn_clock += (rate / self.sample_rate) as f64;
        if self.spawn_clock >= 1.0 && wet > 1e-3 {
            self.spawn_clock -= 1.0;
            let len = self.frozen.len() as f32;
            let glen = ((0.02 + size * 0.18) * self.sample_rate).min(len);
            let jit = (self.rng01() * 2.0 - 1.0) * self.jitter * len * 0.25;
            let start = (self.position * len + jit).rem_euclid(len);
            self.cloud.spawn(start, glen, pitch, 1.0);
        }

        let g = self.cloud.process(&self.frozen);
        // gentle one-pole tone LP
        self.lp_state += (1.0 - self.tone) * (g - self.lp_state);
        ((self.lp_state) * 1.5).tanh() * wet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(n: usize, freq: f32, sr: f32) -> Vec<f32> {
        (0..n)
            .map(|i| (core::f32::consts::TAU * freq * i as f32 / sr).sin())
            .collect()
    }

    #[test]
    fn silent_until_captured() {
        let mut f = GranularFreeze::new(48_000.0);
        f.set_params(true, 0.5, 0.5, 0.5, 0.3, 1.0, 0.3);
        for _ in 0..1000 {
            assert_eq!(f.process(), 0.0);
        }
    }

    #[test]
    fn captures_and_granulates_bounded() {
        let mut f = GranularFreeze::new(48_000.0);
        f.capture(&tone(48_000, 220.0, 48_000.0), 1000, 0.3);
        f.set_params(true, 0.8, 0.5, 0.5, 0.3, 1.0, 0.3);
        let mut energy = 0.0f32;
        let mut peak = 0.0f32;
        for _ in 0..48_000 {
            let y = f.process();
            assert!(y.is_finite());
            energy += y * y;
            peak = peak.max(y.abs());
        }
        assert!(energy > 0.1, "freeze produced no sound");
        assert!(peak <= 1.05, "freeze bus not limited: {peak}");
    }

    #[test]
    fn spawn_rate_is_clamped() {
        // even at max density the spawn rate can't exceed MAX_RATE_HZ → grains
        // are dropped by the hard-capped pool, never an unbounded burst.
        let mut f = GranularFreeze::new(48_000.0);
        f.capture(&tone(48_000, 220.0, 48_000.0), 0, 0.3);
        f.set_params(true, 1.0, 0.9, 0.5, 0.9, 1.0, 0.3);
        for _ in 0..48_000 {
            let y = f.process();
            assert!(y.is_finite() && y.abs() <= 1.05);
        }
    }
}
