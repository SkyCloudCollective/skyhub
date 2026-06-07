//! Transient arp — key-locked arpeggiated sparkles triggered on a clock.
//!
//! De-chaos (from the port spec): a SINGLE sample-accurate clock (no frame-rate
//! pollen accumulator); an optional sync to BPM; a transient gate with cooldown
//! so it can't machine-gun; roam clamped to ≤7 semitones and quantised to a key
//! mask; a hard-capped, pre-sized voice pool; envelopes clamped to 1.0 on
//! retrigger (no amplitude stacking); deterministic (seeded) note choice.
//!
//! Sound design by **Tev** (the plucky-sine sparkle). De-click (A3): every voice
//! onset rises through a short linear attack ramp (≈2 ms, sample-rate-aware)
//! instead of jumping the gain to its peak — Tev heard the original instant
//! onset as a "click" on the pluck. The ramp keeps the percussive character
//! (fast but *continuous*) and, because voice-stealing also restarts through the
//! ramp, a stolen voice can no longer step-discontinuity at full amplitude.

use dsp_core::{clampf, flush_denormal, Oscillator, Smoothed, Waveform};

const MAX_VOICES: usize = 16;
const PER_TICK: usize = 3; // never spawn more than 3 at once

/// Onset attack length. Short enough to stay percussive, long enough that the
/// rising edge has no full-scale step (no click). Converted to a per-sample
/// linear increment at the engine sample rate in `Arp::new`/`spawn`.
const ATTACK_SECS: f32 = 0.002; // ≈2 ms

#[derive(Clone, Copy)]
struct ArpVoice {
    osc: Oscillator,
    freq: f32,
    env: f32,  // decaying tail level (the voice's "peak" target)
    gain: f32, // applied amplitude — ramps 0→env over the attack, then == env
    attacking: bool,
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
    attack_inc: f32, // per-sample gain rise during a voice's onset ramp
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
                gain: 0.0,
                attacking: false,
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
            // rise from 0 to peak over ATTACK_SECS; at least 1 sample so a 0 SR
            // never divides by zero and the ramp is always finite.
            attack_inc: 1.0 / (ATTACK_SECS * sr).max(1.0),
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
            // Target peak: clamp to 1.0 (no amplitude stacking). The audible
            // amplitude (`gain`) keeps its current value and is ramped up to this
            // peak over the attack — so neither a fresh nor a stolen voice steps
            // to full scale, killing Tev's onset "click".
            self.voices[i].env = 1.0_f32.min(self.voices[i].env.max(0.0) + 1.0);
            self.voices[i].gain = self.voices[i].gain.clamp(0.0, self.voices[i].env);
            self.voices[i].attacking = true;
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
            // Attack ramp: rise the audible gain to the voice peak, then hand off
            // to the shared exponential decay. This keeps the onset continuous
            // (no full-scale step → no click) without softening the pluck.
            if v.attacking {
                v.gain += self.attack_inc;
                if v.gain >= v.env {
                    v.gain = v.env;
                    v.attacking = false;
                }
            } else {
                v.gain = v.env;
            }
            acc += s * v.gain;
            v.env = flush_denormal(v.env * self.rel);
            if !v.attacking {
                v.gain = v.env; // post-attack the gain tracks the decaying peak
            }
            if v.env < 1e-4 {
                v.active = false;
                v.attacking = false;
                v.gain = 0.0;
            }
        }
        // true tanh limiter (mandatory, last): |tanh| < 1 guarantees a bounded bus
        let out = (acc * 0.5).tanh();
        out * level
    }

    pub fn reset(&mut self) {
        for v in self.voices.iter_mut() {
            v.active = false;
            v.attacking = false;
            v.env = 0.0;
            v.gain = 0.0;
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
    fn onset_has_no_full_scale_step() {
        // A3 (Tev's "click" on the pluck onset): the attack ramp must keep the
        // sample-to-sample delta bounded — no instant jump to full amplitude.
        // We drive a dense, hard-firing arp and assert |x[n]-x[n-1]| stays well
        // under a full-scale step for the whole render.
        let mut a = Arp::new(48_000.0);
        a.set_params(1.0, 1.0, 0.0, 7.0, 0.2, 8);
        let inp = impulses(48_000, 240); // frequent transients → frequent onsets
        let mut prev = 0.0f32;
        let mut max_delta = 0.0f32;
        for &x in &inp {
            let y = a.process(x);
            assert!(y.is_finite());
            max_delta = max_delta.max((y - prev).abs());
            prev = y;
        }
        // A genuine click (env jumping to 1.0 against a non-zero osc value) would
        // push a per-sample delta toward ~1.0; the 2 ms ramp keeps it small.
        assert!(
            max_delta < 0.35,
            "onset stepped too hard (click): max delta {max_delta}"
        );
    }

    #[test]
    fn voice_gain_ramps_up_from_zero() {
        // Inspect a single freshly-spawned voice: its applied gain must climb
        // from ~0 through the attack rather than starting at the peak.
        let mut a = Arp::new(48_000.0);
        a.set_params(1.0, 1.0, 0.0, 0.0, 0.5, 1);
        // Force one spawn deterministically.
        a.spawn();
        let v0 = a.voices[0];
        assert!(v0.active && v0.attacking, "spawn did not arm the attack");
        assert!(v0.gain <= 1e-3, "fresh voice did not start near zero gain");
        // Run a couple of samples; the gain should rise toward the peak.
        let g_start = a.voices[0].gain;
        a.process(0.0);
        a.process(0.0);
        assert!(
            a.voices[0].gain > g_start,
            "attack did not raise the voice gain"
        );
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
