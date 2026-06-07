//! Transient arp — key-locked arpeggiated sparkles triggered on a clock.
//!
//! De-chaos (from the port spec): a SINGLE sample-accurate clock (no frame-rate
//! pollen accumulator); an optional sync to BPM; a transient gate with cooldown
//! so it can't machine-gun; roam clamped to ≤7 semitones and quantised to a key
//! mask; a hard-capped, pre-sized voice pool; envelopes clamped to 1.0 on
//! retrigger (no amplitude stacking); deterministic (seeded) note choice.

use dsp_core::{clampf, flush_denormal, Oscillator, Smoothed, Waveform};

const MAX_VOICES: usize = 16;
const PER_TICK: usize = 3; // never spawn more than 3 at once

#[derive(Clone, Copy)]
struct ArpVoice {
    osc: Oscillator,
    freq: f32,
    env: f32,
    active: bool,
}

pub struct Arp {
    voices: [ArpVoice; MAX_VOICES],
    sample_rate: f32,
    clock: f64,
    cooldown: u32,
    // transient detector state
    energy: f32,
    prev_energy: f32,
    flux_mean: f32,
    // params
    rate_hz: f32,
    sync: bool,
    bpm: f32,
    division: f32, // ticks per beat (e.g. 2 = eighths)
    roam: f32,     // semitone spread (clamped ≤7)
    bright: f32,
    decay: f32,
    n_voices: usize,
    gate_sens: f32,
    root_midi: f32,
    key_mask: u16, // 12-bit allowed-semitone mask
    rel: f32,
    last_decay: f32,
    rng: u32,
    level_s: Smoothed,
    density_s: Smoothed,
}

impl Arp {
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        let mut a = Self {
            voices: [ArpVoice {
                osc: Oscillator::new(sr),
                freq: 440.0,
                env: 0.0,
                active: false,
            }; MAX_VOICES],
            sample_rate: sr,
            clock: 0.0,
            cooldown: 0,
            energy: 0.0,
            prev_energy: 0.0,
            flux_mean: 0.0,
            rate_hz: 8.0,
            sync: false,
            bpm: 120.0,
            division: 2.0,
            roam: 5.0,
            bright: 0.2,
            decay: 0.18,
            n_voices: 6,
            gate_sens: 2.0,
            root_midi: 57.0,            // A3
            key_mask: 0b1010_1101_0101, // a major-ish scale
            rel: 0.0,
            last_decay: -1.0,
            rng: 0x9E37_79B9,
            level_s: Smoothed::new(0.0, 0.03, sr),
            density_s: Smoothed::new(0.4, 0.05, sr),
        };
        a.update_rel();
        a
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_params(
        &mut self,
        level: f32,
        density: f32,
        bright: f32,
        roam: f32,
        decay: f32,
        n_voices: usize,
    ) {
        self.level_s.set_target(clampf(level, 0.0, 1.0));
        self.density_s.set_target(clampf(density, 0.0, 1.0));
        self.bright = clampf(bright, 0.0, 1.0);
        self.roam = clampf(roam, 0.0, 7.0); // de-chaos: never leave the key
        let d = clampf(decay, 0.02, 2.0);
        if (d - self.last_decay).abs() > 1e-4 {
            self.decay = d;
            self.update_rel();
        }
        self.n_voices = n_voices.clamp(1, MAX_VOICES);
    }

    pub fn set_clock(&mut self, sync: bool, rate_hz: f32, bpm: f32, division: f32) {
        self.sync = sync;
        self.rate_hz = clampf(rate_hz, 0.1, 40.0);
        self.bpm = clampf(bpm, 20.0, 300.0);
        self.division = clampf(division, 0.25, 8.0);
    }

    pub fn set_key(&mut self, root_midi: f32, key_mask: u16) {
        self.root_midi = root_midi;
        self.key_mask = if key_mask == 0 { 0xFFF } else { key_mask };
    }

    fn update_rel(&mut self) {
        self.rel = (-6.9 / (self.decay * self.sample_rate)).exp();
        self.last_decay = self.decay;
    }

    #[inline]
    fn rng01(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.rng >> 9) as f32 / (1u32 << 23) as f32
    }

    #[inline]
    fn tick_rate(&self) -> f32 {
        if self.sync {
            (self.bpm / 60.0) * self.division
        } else {
            self.rate_hz
        }
    }

    /// Quantise a semitone offset to the nearest allowed key-mask degree.
    fn quantize(&self, semis: f32) -> f32 {
        let s = semis.round() as i32;
        for d in 0..12 {
            for &off in &[d, -d] {
                let n = ((s + off) % 12 + 12) % 12;
                if self.key_mask & (1 << n) != 0 {
                    return (s + off) as f32;
                }
            }
        }
        semis
    }

    fn spawn(&mut self) {
        let count = PER_TICK.min(self.n_voices);
        for _ in 0..count {
            let r = (self.rng01() * 2.0 - 1.0) * self.roam;
            let semis = self.quantize(r);
            let midi = self.root_midi + semis + 12.0 * (self.rng01() * 2.0).floor(); // 0/1 octave up
            let freq = 440.0 * (2.0_f32).powf((midi - 69.0) / 12.0);
            // find a free (or quietest) slot, bounded by n_voices
            let mut chosen = None;
            for i in 0..self.n_voices {
                if !self.voices[i].active {
                    chosen = Some(i);
                    break;
                }
            }
            let i = chosen.unwrap_or_else(|| {
                // steal the quietest
                let mut q = 0;
                for j in 1..self.n_voices {
                    if self.voices[j].env < self.voices[q].env {
                        q = j;
                    }
                }
                q
            });
            self.voices[i].freq = clampf(freq, 20.0, self.sample_rate * 0.45);
            self.voices[i].osc.reset();
            self.voices[i].env = 1.0_f32.min(self.voices[i].env.max(0.0) + 1.0); // clamp to 1.0
            self.voices[i].active = true;
        }
    }

    #[inline]
    fn detect_transient(&mut self, input: f32) -> bool {
        let e = input * input;
        self.energy = e + (self.energy - e) * 0.99; // smoothed short-term energy
        let flux = (self.energy - self.prev_energy).max(0.0);
        self.prev_energy = self.energy;
        self.flux_mean = flux * 0.001 + self.flux_mean * 0.999;
        flux > self.flux_mean * (1.0 + self.gate_sens) + 1e-7
    }

    /// Process one sample; `input` drives transient detection. Returns the arp bus.
    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let level = self.level_s.next();
        let density = self.density_s.next();

        let transient = self.detect_transient(input);
        if self.cooldown > 0 {
            self.cooldown -= 1;
        }

        // advance the single sample-accurate clock
        self.clock += (self.tick_rate() / self.sample_rate) as f64;
        let ticked = self.clock >= 1.0;
        if ticked {
            self.clock -= 1.0;
            // fire on a transient, or probabilistically when density is high
            let fire = transient || self.rng01() < density;
            if fire && self.cooldown == 0 {
                self.spawn();
                self.cooldown = (self.sample_rate * 0.02) as u32; // 20 ms min spacing
            }
        }

        // render active voices
        let mut acc = 0.0;
        for v in self.voices.iter_mut() {
            if !v.active {
                continue;
            }
            let mut s = v.osc.next(v.freq, Waveform::Sine);
            if self.bright > 0.0 {
                // a touch of 2nd partial for sparkle
                s += self.bright * 0.5 * (s * s * 2.0 - 1.0);
            }
            acc += s * v.env;
            v.env = flush_denormal(v.env * self.rel);
            if v.env < 1e-4 {
                v.active = false;
            }
        }
        // true tanh limiter (mandatory, last): |tanh| < 1 guarantees a bounded bus
        let out = (acc * 0.5).tanh();
        out * level
    }

    pub fn reset(&mut self) {
        for v in self.voices.iter_mut() {
            v.active = false;
            v.env = 0.0;
        }
        self.clock = 0.0;
        self.cooldown = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn impulses(n: usize, period: usize) -> Vec<f32> {
        (0..n)
            .map(|i| if i % period == 0 { 1.0 } else { 0.0 })
            .collect()
    }

    #[test]
    fn silent_when_level_zero() {
        let mut a = Arp::new(48_000.0);
        a.set_params(0.0, 1.0, 0.2, 5.0, 0.1, 6);
        let inp = impulses(8000, 480);
        let out: Vec<f32> = inp.iter().map(|&x| a.process(x)).collect();
        assert!(
            out.iter().all(|v| v.abs() < 1e-4),
            "arp not silent at level 0"
        );
    }

    #[test]
    fn fires_and_is_bounded() {
        let mut a = Arp::new(48_000.0);
        a.set_params(1.0, 1.0, 0.3, 5.0, 0.15, 6);
        let inp = impulses(48_000, 480);
        let mut energy = 0.0f32;
        let mut peak = 0.0f32;
        for &x in &inp {
            let y = a.process(x);
            assert!(y.is_finite());
            energy += y * y;
            peak = peak.max(y.abs());
        }
        assert!(energy > 0.1, "arp produced no output");
        assert!(peak <= 1.05, "arp output not limited: {peak}");
    }

    #[test]
    fn never_exceeds_voice_cap() {
        let mut a = Arp::new(48_000.0);
        a.set_params(1.0, 1.0, 7.0, 0.5, 1.0, 4);
        for i in 0..48_000 {
            a.process(if i % 50 == 0 { 1.0 } else { 0.0 });
            let active = a.voices.iter().filter(|v| v.active).count();
            assert!(active <= 4, "exceeded voice cap: {active}");
        }
    }

    #[test]
    fn roam_stays_in_key() {
        let a = Arp::new(48_000.0);
        // every quantised offset must land on an allowed key-mask degree
        for s in -7..=7 {
            let q = a.quantize(s as f32);
            let n = (((q as i32) % 12) + 12) % 12;
            assert!(a.key_mask & (1 << n) != 0, "quantised {s} -> {q} off-key");
        }
    }
}
