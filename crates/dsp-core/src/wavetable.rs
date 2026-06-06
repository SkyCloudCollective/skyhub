//! Wavetable oscillator — PhasePlan's marquee generator.
//!
//! A wavetable is a set of single-cycle frames; `position` (0..1) morphs between
//! adjacent frames, `phase` reads within a frame (both linearly interpolated).
//! The built-in frames are additive (finite harmonic sums), so the table content
//! is band-limited by construction. (Per-octave mip tables to fully kill
//! playback aliasing at very high notes are a later refinement.)

use crate::util::TWO_PI;

pub const FRAME_LEN: usize = 2048;

#[derive(Clone, Debug)]
pub struct Wavetable {
    frames: Vec<[f32; FRAME_LEN]>,
    phase: f32,
    sample_rate: f32,
}

fn additive(harmonics: &[(f32, f32)]) -> [f32; FRAME_LEN] {
    // harmonics = [(multiple, amplitude)], normalised to peak ~1.
    let mut f = [0.0f32; FRAME_LEN];
    let mut peak = 0.0f32;
    for (i, s) in f.iter_mut().enumerate() {
        let t = i as f32 / FRAME_LEN as f32;
        let mut v = 0.0;
        for (mult, amp) in harmonics {
            v += amp * (TWO_PI * mult * t).sin();
        }
        *s = v;
        peak = peak.max(v.abs());
    }
    if peak > 1e-6 {
        for s in f.iter_mut() {
            *s /= peak;
        }
    }
    f
}

impl Wavetable {
    pub fn new(frames: Vec<[f32; FRAME_LEN]>, sample_rate: f32) -> Self {
        let frames = if frames.is_empty() {
            vec![additive(&[(1.0, 1.0)])]
        } else {
            frames
        };
        Self {
            frames,
            phase: 0.0,
            sample_rate: sample_rate.max(1.0),
        }
    }

    /// Default morphing set: sine → triangle → saw → square (all band-limited).
    pub fn basic(sample_rate: f32) -> Self {
        const K: usize = 24;
        let sine = additive(&[(1.0, 1.0)]);
        let mut tri = Vec::new();
        let mut saw = Vec::new();
        let mut sq = Vec::new();
        for k in 1..=K {
            let kf = k as f32;
            saw.push((kf, 1.0 / kf));
            if k % 2 == 1 {
                sq.push((kf, 1.0 / kf));
                let sign = if (k / 2) % 2 == 0 { 1.0 } else { -1.0 };
                tri.push((kf, sign / (kf * kf)));
            }
        }
        Self::new(
            vec![sine, additive(&tri), additive(&saw), additive(&sq)],
            sample_rate,
        )
    }

    #[inline]
    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sample_rate = sr.max(1.0);
    }

    #[inline]
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    #[inline]
    fn sample_frame(frame: &[f32; FRAME_LEN], phase: f32) -> f32 {
        let p = phase * FRAME_LEN as f32;
        let i0 = p.floor() as usize % FRAME_LEN;
        let i1 = (i0 + 1) % FRAME_LEN;
        let fr = p - p.floor();
        frame[i0] * (1.0 - fr) + frame[i1] * fr
    }

    /// One sample at `freq` Hz, morphing to `position` in [0,1].
    #[inline]
    pub fn next(&mut self, freq: f32, position: f32) -> f32 {
        let nf = self.frames.len();
        let out = if nf == 1 {
            Self::sample_frame(&self.frames[0], self.phase)
        } else {
            let fp = position.clamp(0.0, 1.0) * (nf - 1) as f32;
            let a = fp.floor() as usize;
            let b = (a + 1).min(nf - 1);
            let fr = fp - fp.floor();
            let va = Self::sample_frame(&self.frames[a], self.phase);
            let vb = Self::sample_frame(&self.frames[b], self.phase);
            va * (1.0 - fr) + vb * fr
        };
        self.phase += (freq / self.sample_rate).clamp(0.0, 0.5);
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        out
    }

    pub fn num_frames(&self) -> usize {
        self.frames.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(b: &[f32]) -> f32 {
        (b.iter().map(|x| x * x).sum::<f32>() / b.len() as f32).sqrt()
    }

    #[test]
    fn produces_bounded_finite_sound() {
        let mut w = Wavetable::basic(48_000.0);
        let mut buf = vec![0.0; 4800];
        for x in buf.iter_mut() {
            *x = w.next(220.0, 0.0);
        }
        assert!(buf.iter().all(|v| v.is_finite() && v.abs() <= 1.05));
        assert!(rms(&buf) > 0.3); // sine frame ~0.7
    }

    #[test]
    fn position_morph_changes_timbre() {
        let mut w = Wavetable::basic(48_000.0);
        let mut sine_b = vec![0.0; 2048];
        for x in sine_b.iter_mut() {
            *x = w.next(110.0, 0.0);
        }
        w.reset();
        let mut saw_b = vec![0.0; 2048];
        for x in saw_b.iter_mut() {
            *x = w.next(110.0, 0.66); // toward saw
        }
        // a saw has more high-harmonic content -> different waveform than sine
        let diff: f32 = sine_b.iter().zip(&saw_b).map(|(a, b)| (a - b).abs()).sum();
        assert!(diff > 50.0, "position did not change the waveform: {diff}");
    }
}
