//! Effects — drive, delay, reverb, chorus. Real-time-safe: ring buffers are
//! sized once at construction; nothing allocates in `process`. Feedback paths
//! flush denormals.

use crate::util::flush_denormal;

/// Soft saturation (tanh waveshaper) with dry/wet. `drive` >= 1.
#[derive(Clone, Copy, Debug)]
pub struct Drive {
    pub drive: f32,
    pub mix: f32,
}

impl Default for Drive {
    fn default() -> Self {
        Self {
            drive: 1.0,
            mix: 1.0,
        }
    }
}

impl Drive {
    #[inline]
    pub fn process(&self, x: f32) -> f32 {
        let d = self.drive.max(1.0);
        let wet = (x * d).tanh() / d.tanh();
        x + (wet - x) * self.mix.clamp(0.0, 1.0)
    }
}

/// A fractional delay line (linear interpolation). `read` before `write`.
#[derive(Clone, Debug)]
pub struct DelayLine {
    buf: Vec<f32>,
    idx: usize,
}

impl DelayLine {
    pub fn new(max_samples: usize) -> Self {
        Self {
            buf: vec![0.0; max_samples.max(1)],
            idx: 0,
        }
    }

    pub fn clear(&mut self) {
        for v in self.buf.iter_mut() {
            *v = 0.0;
        }
    }

    #[inline]
    pub fn read(&self, delay_samples: f32) -> f32 {
        let n = self.buf.len();
        let d = delay_samples.clamp(1.0, (n - 1) as f32);
        let read_pos = (self.idx as f32 - d + n as f32) % n as f32;
        let i0 = read_pos.floor() as usize % n;
        let i1 = (i0 + 1) % n;
        let frac = read_pos - read_pos.floor();
        self.buf[i0] * (1.0 - frac) + self.buf[i1] * frac
    }

    #[inline]
    pub fn write(&mut self, x: f32) {
        self.buf[self.idx] = x;
        self.idx = (self.idx + 1) % self.buf.len();
    }
}

/// Stereo-agnostic mono delay with feedback + a one-pole damping in the loop.
#[derive(Clone, Debug)]
pub struct Delay {
    line: DelayLine,
    sample_rate: f32,
    time_s: f32,
    pub feedback: f32,
    pub mix: f32,
    damp: f32,
    damp_state: f32,
}

impl Delay {
    pub fn new(sample_rate: f32, max_time_s: f32) -> Self {
        let max = (sample_rate * max_time_s).ceil() as usize + 4;
        Self {
            line: DelayLine::new(max),
            sample_rate: sample_rate.max(1.0),
            time_s: 0.25,
            feedback: 0.3,
            mix: 0.25,
            damp: 0.3,
            damp_state: 0.0,
        }
    }

    pub fn set(&mut self, time_s: f32, feedback: f32, mix: f32, damp: f32) {
        self.time_s = time_s.max(0.001);
        self.feedback = feedback.clamp(0.0, 0.98);
        self.mix = mix.clamp(0.0, 1.0);
        self.damp = damp.clamp(0.0, 0.99);
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let d = self.time_s * self.sample_rate;
        let delayed = self.line.read(d);
        // damp the feedback (one-pole low-pass)
        self.damp_state = delayed * (1.0 - self.damp) + self.damp_state * self.damp;
        let fb = flush_denormal(x + self.damp_state * self.feedback);
        self.line.write(fb);
        x + (delayed - x) * self.mix
    }
}

/// Schroeder comb with damping (one per parallel band of the reverb).
#[derive(Clone, Debug)]
struct Comb {
    line: DelayLine,
    feedback: f32,
    damp: f32,
    store: f32,
    delay: f32,
}

impl Comb {
    fn new(delay_samples: usize) -> Self {
        Self {
            line: DelayLine::new(delay_samples + 2),
            feedback: 0.84,
            damp: 0.2,
            store: 0.0,
            delay: delay_samples as f32,
        }
    }
    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let y = self.line.read(self.delay);
        self.store = flush_denormal(y * (1.0 - self.damp) + self.store * self.damp);
        self.line
            .write(flush_denormal(x + self.store * self.feedback));
        y
    }
}

#[derive(Clone, Debug)]
struct Allpass {
    line: DelayLine,
    delay: f32,
    feedback: f32,
}

impl Allpass {
    fn new(delay_samples: usize) -> Self {
        Self {
            line: DelayLine::new(delay_samples + 2),
            delay: delay_samples as f32,
            feedback: 0.5,
        }
    }
    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let buf = self.line.read(self.delay);
        let out = -x + buf;
        self.line.write(flush_denormal(x + buf * self.feedback));
        out
    }
}

/// Algorithmic reverb (4 parallel damped combs → 2 series allpass diffusers).
#[derive(Clone, Debug)]
pub struct Reverb {
    combs: [Comb; 4],
    allpasses: [Allpass; 2],
    pub mix: f32,
}

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        let s = sample_rate / 44100.0;
        let cd = [1116, 1188, 1277, 1356];
        let ad = [556, 441];
        Self {
            combs: [
                Comb::new((cd[0] as f32 * s) as usize),
                Comb::new((cd[1] as f32 * s) as usize),
                Comb::new((cd[2] as f32 * s) as usize),
                Comb::new((cd[3] as f32 * s) as usize),
            ],
            allpasses: [
                Allpass::new((ad[0] as f32 * s) as usize),
                Allpass::new((ad[1] as f32 * s) as usize),
            ],
            mix: 0.25,
        }
    }

    /// `size` 0..1 scales tail length (feedback), `damp` 0..1 darkens it.
    pub fn set(&mut self, size: f32, damp: f32, mix: f32) {
        let fb = 0.7 + size.clamp(0.0, 1.0) * 0.28;
        for c in self.combs.iter_mut() {
            c.feedback = fb;
            c.damp = damp.clamp(0.0, 0.99);
        }
        self.mix = mix.clamp(0.0, 1.0);
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let mut wet = 0.0;
        for c in self.combs.iter_mut() {
            wet += c.process(x);
        }
        wet *= 0.25;
        for a in self.allpasses.iter_mut() {
            wet = a.process(wet);
        }
        x + (wet - x) * self.mix
    }
}

/// Chorus — a short LFO-modulated delay summed with the dry signal.
#[derive(Clone, Debug)]
pub struct Chorus {
    line: DelayLine,
    sample_rate: f32,
    phase: f32,
    pub rate_hz: f32,
    pub depth_ms: f32,
    pub base_ms: f32,
    pub mix: f32,
}

impl Chorus {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            line: DelayLine::new((sample_rate * 0.05) as usize + 4),
            sample_rate: sample_rate.max(1.0),
            phase: 0.0,
            rate_hz: 0.5,
            depth_ms: 4.0,
            base_ms: 12.0,
            mix: 0.4,
        }
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let lfo = (core::f32::consts::TAU * self.phase).sin();
        self.phase += self.rate_hz / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        let ms = self.base_ms + lfo * self.depth_ms;
        let d = (ms * 0.001 * self.sample_rate).max(1.0);
        let wet = self.line.read(d);
        self.line.write(x);
        x + (wet - x) * self.mix.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn energy(b: &[f32]) -> f32 {
        b.iter().map(|x| x * x).sum()
    }

    #[test]
    fn drive_changes_signal_and_stays_bounded() {
        let d = Drive {
            drive: 5.0,
            mix: 1.0,
        };
        let mut peak = 0.0_f32;
        for i in 0..1000 {
            let x = (i as f32 * 0.01).sin();
            peak = peak.max(d.process(x).abs());
        }
        assert!(peak <= 1.05, "drive output unbounded: {peak}");
    }

    #[test]
    fn delay_echoes_an_impulse_later() {
        let sr = 48_000.0;
        let mut d = Delay::new(sr, 1.0);
        d.set(0.01, 0.5, 1.0, 0.2); // 10 ms
        let mut out = vec![0.0; 4800];
        for (i, o) in out.iter_mut().enumerate() {
            *o = d.process(if i == 0 { 1.0 } else { 0.0 });
        }
        // energy after the first few ms (the echo) must be non-trivial
        assert!(energy(&out[100..]) > 0.01, "no echo energy");
        assert!(out.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn reverb_tail_decays_and_is_finite() {
        let sr = 48_000.0;
        let mut r = Reverb::new(sr);
        r.set(0.5, 0.3, 1.0);
        let mut out = vec![0.0; sr as usize];
        for (i, o) in out.iter_mut().enumerate() {
            *o = r.process(if i < 16 { 1.0 } else { 0.0 });
        }
        assert!(out.iter().all(|v| v.is_finite()));
        let early = energy(&out[1000..6000]);
        let late = energy(&out[40000..45000]);
        assert!(
            late < early,
            "reverb tail did not decay: early {early} late {late}"
        );
    }

    #[test]
    fn chorus_is_finite_and_audible() {
        let sr = 48_000.0;
        let mut c = Chorus::new(sr);
        let mut moved = false;
        let mut prev = 0.0;
        for i in 0..4800 {
            let x = (core::f32::consts::TAU * 220.0 * i as f32 / sr).sin();
            let y = c.process(x);
            assert!(y.is_finite());
            if (y - prev).abs() > 1e-4 {
                moved = true;
            }
            prev = y;
        }
        assert!(moved);
    }
}
