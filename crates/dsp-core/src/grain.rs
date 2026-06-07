//! Granular cloud — a fixed, pre-allocated pool of windowed grains reading from
//! a caller-owned buffer. The pool is HARD-CAPPED (no per-grain allocation in
//! the audio thread); the caller is responsible for clamping the spawn rate (the
//! v1 "pollen" bug was an unbounded spawn loop). Grains use a Hann window.

use crate::util::TWO_PI;

pub const MAX_GRAINS: usize = 16;

#[derive(Clone, Copy)]
struct Grain {
    pos: f32, // fractional read head into the buffer
    inc: f32, // read increment (pitch)
    age: f32, // samples elapsed
    len: f32, // grain length in samples
    gain: f32,
    active: bool,
}

impl Grain {
    const OFF: Self = Self {
        pos: 0.0,
        inc: 1.0,
        age: 0.0,
        len: 1.0,
        gain: 0.0,
        active: false,
    };
}

pub struct GrainCloud {
    grains: [Grain; MAX_GRAINS],
}

impl Default for GrainCloud {
    fn default() -> Self {
        Self::new()
    }
}

impl GrainCloud {
    pub fn new() -> Self {
        Self {
            grains: [Grain::OFF; MAX_GRAINS],
        }
    }

    pub fn reset(&mut self) {
        self.grains = [Grain::OFF; MAX_GRAINS];
    }

    pub fn active(&self) -> usize {
        self.grains.iter().filter(|g| g.active).count()
    }

    /// Start a grain. Returns false if the pool is full (caller must not retry —
    /// dropping is the de-chaos behaviour). `start`/`len` in buffer samples,
    /// `pitch` is the read increment (1.0 = original speed).
    pub fn spawn(&mut self, start: f32, len: f32, pitch: f32, gain: f32) -> bool {
        for g in self.grains.iter_mut() {
            if !g.active {
                g.pos = start.max(0.0);
                g.inc = pitch.clamp(0.05, 8.0);
                g.age = 0.0;
                g.len = len.max(4.0);
                g.gain = gain;
                g.active = true;
                return true;
            }
        }
        false
    }

    #[inline]
    fn read(buf: &[f32], pos: f32) -> f32 {
        let n = buf.len();
        if n == 0 {
            return 0.0;
        }
        let i0 = (pos.floor() as usize) % n;
        let i1 = (i0 + 1) % n;
        let fr = pos - pos.floor();
        buf[i0] * (1.0 - fr) + buf[i1] * fr
    }

    /// Sum all active grains reading from `buf`, advancing + windowing each.
    /// Output is energy-normalised by the active count so density doesn't blow up.
    #[inline]
    pub fn process(&mut self, buf: &[f32]) -> f32 {
        let mut acc = 0.0;
        let mut n = 0u32;
        for g in self.grains.iter_mut() {
            if !g.active {
                continue;
            }
            let w = 0.5 - 0.5 * (TWO_PI * (g.age / g.len)).cos(); // Hann
            acc += Self::read(buf, g.pos) * w * g.gain;
            g.pos += g.inc;
            g.age += 1.0;
            if g.age >= g.len {
                g.active = false;
            }
            n += 1;
        }
        if n > 1 {
            acc /= (n as f32).sqrt();
        }
        acc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_is_hard_capped() {
        let mut c = GrainCloud::new();
        let mut ok = 0;
        for _ in 0..(MAX_GRAINS + 8) {
            if c.spawn(0.0, 1000.0, 1.0, 1.0) {
                ok += 1;
            }
        }
        assert_eq!(ok, MAX_GRAINS, "pool exceeded its hard cap");
        assert_eq!(c.active(), MAX_GRAINS);
    }

    #[test]
    fn grain_windows_in_and_out() {
        let buf = vec![1.0f32; 2000]; // constant buffer → output is just the window
        let mut c = GrainCloud::new();
        c.spawn(0.0, 100.0, 1.0, 1.0);
        let out: Vec<f32> = (0..100).map(|_| c.process(&buf)).collect();
        assert!(out[0].abs() < 0.05, "grain did not fade in");
        assert!(out[50] > 0.8, "grain not full at centre");
        assert!(out[99].abs() < 0.1, "grain did not fade out");
        assert_eq!(c.active(), 0, "grain did not retire");
    }

    #[test]
    fn output_is_finite_and_bounded() {
        let buf: Vec<f32> = (0..4000).map(|i| (i as f32 * 0.01).sin()).collect();
        let mut c = GrainCloud::new();
        for k in 0..MAX_GRAINS {
            c.spawn(k as f32 * 50.0, 800.0, 1.0 + k as f32 * 0.01, 1.0);
        }
        let mut peak = 0.0f32;
        for _ in 0..2000 {
            let y = c.process(&buf);
            assert!(y.is_finite());
            peak = peak.max(y.abs());
        }
        assert!(peak < 6.0, "grain cloud too loud: {peak}");
    }
}
