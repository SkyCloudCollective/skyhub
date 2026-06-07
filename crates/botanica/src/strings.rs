//! Strings — a bounded polyphonic choir/string bed for Botanica.
//!
//! Port of v1's `triggerStringChord` + `makeStringVoice` (the unused
//! `char.strings.*` genes), rebuilt as a real, tamed DSP module. A fixed voice
//! pool (cap 12) is fired on detected transients; each voice is a detuned
//! saw+triangle pair with a vibrato LFO, a short bowed-noise breath transient
//! (HP→BP), a shared body low-pass and an attack/decay amp envelope. Chords are
//! built from a major/minor triad on a chosen root, voiced with a *seeded* RNG
//! (deterministic, no `Math.random`), and allocated by voice-stealing — never by
//! allocating on the audio thread.
//!
//! De-chaos (port spec): one onset gate with hysteresis + a refractory time so a
//! burst of transients can't make the bed continuous; a hard probability ceiling;
//! bounded polyphony; bounded vibrato/detune; a tanh-limited bus.

use dsp_core::{clampf, flush_denormal, Adsr, EnvStage, Lfo, Oscillator, Svf, SvfMode, Waveform};

const MAX_VOICES: usize = 12;
const BURST_LEN: usize = 7200; // ~150 ms at 48 kHz (bowed-noise breath)

#[inline]
fn note_to_freq(midi: f32) -> f32 {
    440.0 * (2.0_f32).powf((midi - 69.0) / 12.0)
}

/// User-facing string-bed parameters (Copy; set once per block).
#[derive(Clone, Copy, Debug)]
pub struct StringParams {
    pub level: f32,           // overall bed mix 0..1
    pub probability: f32,     // base fire chance per onset 0..1
    pub probability_max: f32, // hard ceiling 0..1 (bed can't go continuous)
    pub airiness: f32,        // breath-transient amount 0..1
    pub detune_cents: f32,    // saw/triangle ensemble width 0..25
    pub vibrato_rate: f32,    // master vibrato rate (Hz)
    pub vibrato_depth: f32,   // 0..1 → hz * (0.001..0.008)
    pub attack: f32,          // bow-on attack (s)
    pub release: f32,         // tail length (s)
    pub max_voices: usize,    // 1..12
    pub body_tone: f32,       // 0..1 dark→bright body LP
    pub octave_base: f32,     // root MIDI (24..60)
    pub key: f32,             // root offset in semitones (0..11)
    pub minor: bool,          // triad quality
}

impl Default for StringParams {
    fn default() -> Self {
        Self {
            level: 0.45,
            probability: 0.30,
            probability_max: 0.80,
            airiness: 0.40,
            detune_cents: 6.0,
            vibrato_rate: 5.4,
            vibrato_depth: 0.4,
            attack: 0.20,
            release: 0.40,
            max_voices: 8,
            body_tone: 0.5,
            octave_base: 48.0,
            key: 0.0,
            minor: false,
        }
    }
}

#[derive(Clone, Copy)]
struct StringVoice {
    saw: Oscillator,
    tri: Oscillator,
    vib: Lfo,
    body: Svf,
    breath_hp: Svf,
    breath_bp: Svf,
    amp: Adsr,
    hz: f32,
    detune: f32,    // ratio for the triangle
    vib_ratio: f32, // per-voice ratio of the master vibrato rate
    burst: usize,
    noise: u32,
    active: bool,
    age: u64,
}

impl StringVoice {
    fn new(sr: f32) -> Self {
        Self {
            saw: Oscillator::new(sr),
            tri: Oscillator::new(sr),
            vib: Lfo::new(sr),
            body: Svf::new(sr),
            breath_hp: Svf::new(sr),
            breath_bp: Svf::new(sr),
            amp: Adsr::new(sr),
            hz: 220.0,
            detune: 1.0,
            vib_ratio: 1.0,
            burst: 0,
            noise: 0x1234_5678,
            active: false,
            age: 0,
        }
    }

    fn note_on(&mut self, hz: f32, p: &StringParams, seed: u32, age: u64) {
        self.hz = hz.clamp(20.0, 6000.0);
        self.detune = (2.0_f32).powf(clampf(p.detune_cents, 0.0, 25.0) / 1200.0);
        self.vib_ratio = 0.85 + 0.30 * ((seed >> 8 & 0xFF) as f32 / 255.0);
        self.noise = seed | 1;
        // body LP: dark→bright with body_tone; bounded
        let body_cut = clampf(
            (900.0 + self.hz * 2.0) * (0.6 + p.body_tone * 0.9),
            900.0,
            6000.0,
        );
        self.body.set(body_cut, 0.85);
        // bowed-noise breath path: HP 1800 → BP around the 2.4th harmonic
        self.breath_hp.set(1800.0, 0.707);
        self.breath_bp
            .set(clampf(self.hz * 2.4, 600.0, 4800.0), 3.5);
        self.body.reset();
        self.breath_hp.reset();
        self.breath_bp.reset();
        self.saw.reset();
        self.tri.reset();
        self.vib.reset(0.0);
        // AD shape: decay runs the full tail toward 0; sustain 0 frees the voice
        self.amp
            .set(clampf(p.attack, 0.02, 0.45), p.attack + p.release, 0.0, 0.1);
        self.amp.gate_on();
        self.burst = BURST_LEN;
        self.active = true;
        self.age = age;
    }

    #[inline]
    fn rng01(&mut self) -> f32 {
        self.noise = self
            .noise
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        (self.noise >> 9) as f32 / (1u32 << 23) as f32
    }

    #[inline]
    fn next(&mut self, master_vib: f32, vib_depth_norm: f32, airiness: f32) -> f32 {
        if !self.active {
            return 0.0;
        }
        let vdepth = self.hz * vib_depth_norm;
        let lfo = self.vib.next(master_vib * self.vib_ratio, Waveform::Sine);
        let f = self.hz + lfo * vdepth;

        let saw = self.saw.next(f, Waveform::Saw);
        let tri = self.tri.next(f * self.detune, Waveform::Triangle);
        let osc = 0.55 * saw + 0.45 * tri;
        let body = self.body.process(osc, SvfMode::Lowpass);

        let breath = if self.burst > 0 {
            let i = BURST_LEN - self.burst;
            let w = (1.0 - i as f32 / BURST_LEN as f32).powf(1.2);
            let nz = (self.rng01() * 2.0 - 1.0) * w;
            let hp = self.breath_hp.process(nz, SvfMode::Highpass);
            let bp = self.breath_bp.process(hp, SvfMode::Bandpass);
            self.burst -= 1;
            bp * airiness
        } else {
            0.0
        };

        let env = self.amp.next();
        // free the voice once the AD has decayed to ~0 (parks at Sustain=0)
        if env < 1e-4 && self.amp.stage() != EnvStage::Attack {
            self.active = false;
        }
        flush_denormal((body + breath) * env)
    }
}

/// The string bed: a bounded voice pool fired by transients.
pub struct StringBed {
    voices: [StringVoice; MAX_VOICES],
    sample_rate: f32,
    params: StringParams,
    // onset detector (hysteresis + refractory)
    energy: f32,
    prev_energy: f32,
    flux_mean: f32,
    armed: bool,
    cooldown: u32,
    // macro inputs (set per block)
    macro_i: f32,
    macro_b: f32,
    density: f32,
    char_prob: f32,
    vib_depth_norm: f32,
    rng: u32,
    age: u64,
}

impl StringBed {
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        Self {
            voices: core::array::from_fn(|_| StringVoice::new(sr)),
            sample_rate: sr,
            params: StringParams::default(),
            energy: 0.0,
            prev_energy: 0.0,
            flux_mean: 0.0,
            armed: true,
            cooldown: 0,
            macro_i: 0.55,
            macro_b: 0.65,
            density: 0.4,
            char_prob: 0.0,
            vib_depth_norm: 0.001 + 0.4 * 0.007,
            rng: 0x2545_F491,
            age: 0,
        }
    }

    /// Set the bed params + the macro inputs that bias firing and gain.
    pub fn set_params(
        &mut self,
        p: StringParams,
        intensity: f32,
        bloom: f32,
        density: f32,
        char_prob: f32,
    ) {
        self.params = p;
        self.macro_i = clampf(intensity, 0.0, 1.0);
        self.macro_b = clampf(bloom, 0.0, 1.0);
        self.density = clampf(density, 0.0, 1.0);
        self.char_prob = clampf(char_prob, 0.0, 1.0);
        // map vibrato_depth 0..1 → 0.001..0.008 of hz
        self.vib_depth_norm = 0.001 + clampf(p.vibrato_depth, 0.0, 1.0) * 0.007;
    }

    #[inline]
    fn rng01(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.rng >> 9) as f32 / (1u32 << 23) as f32
    }

    /// Onset detector with hysteresis: only re-arms after the flux falls back.
    #[inline]
    fn detect_onset(&mut self, x: f32) -> bool {
        let e = x * x;
        self.energy = e + (self.energy - e) * 0.99;
        let flux = (self.energy - self.prev_energy).max(0.0);
        self.prev_energy = self.energy;
        self.flux_mean = flux * 0.001 + self.flux_mean * 0.999;
        let hi = self.flux_mean * 3.0 + 1e-7;
        let lo = self.flux_mean * 1.2 + 1e-7;
        if self.armed && flux > hi {
            self.armed = false;
            true
        } else {
            if flux < lo {
                self.armed = true;
            }
            false
        }
    }

    fn alloc(&mut self) -> usize {
        self.age = self.age.wrapping_add(1);
        let cap = self.params.max_voices.clamp(1, MAX_VOICES);
        for i in 0..cap {
            if !self.voices[i].active {
                return i;
            }
        }
        // steal the oldest
        let mut oldest = 0;
        for i in 1..cap {
            if self.voices[i].age < self.voices[oldest].age {
                oldest = i;
            }
        }
        oldest
    }

    /// Fire a chord (probability-gated). Returns true if it fired.
    pub fn trigger(&mut self, strength: f32) -> bool {
        let p = clampf(
            self.params.probability
                + 0.42 * self.macro_i
                + 0.26 * self.density
                + 0.10 * strength.clamp(0.0, 1.0)
                + self.char_prob,
            0.0,
            self.params.probability_max.clamp(0.0, 0.98),
        );
        if self.rng01() >= p {
            return false;
        }
        let root = self.params.octave_base + (self.params.key.round() % 12.0);
        let third = if self.params.minor { 3.0 } else { 4.0 };
        // simple, bounded voicing: root, third, fifth, octave
        let intervals = [0.0, third, 7.0, 12.0];
        let n = (2 + (self.rng01() * 2.0) as usize).min(intervals.len()); // 2–3 notes
        let age = self.age;
        for &iv in intervals.iter().take(n) {
            let hz = note_to_freq(root + iv);
            let seed = self.rng;
            let i = self.alloc();
            self.voices[i].note_on(hz, &self.params, seed, age);
        }
        true
    }

    /// Process one sample; `dry` drives onset detection. Returns the bed bus.
    #[inline]
    pub fn process(&mut self, dry: f32) -> f32 {
        if self.cooldown > 0 {
            self.cooldown -= 1;
        }
        if self.detect_onset(dry) && self.cooldown == 0 {
            let strength = self.energy.sqrt().min(1.0);
            if self.trigger(strength) {
                self.cooldown = (self.sample_rate * 0.12) as u32; // 120 ms refractory
            }
        }

        let vib = clampf(self.params.vibrato_rate, 0.1, 12.0);
        let depth = self.vib_depth_norm;
        let air = clampf(self.params.airiness, 0.0, 1.0);
        let mut acc = 0.0;
        for v in self.voices.iter_mut() {
            acc += v.next(vib, depth, air);
        }
        // bounded bus, then the macro base-gain × the master level (level 0 = silent).
        let base_gain = 0.04 + 0.12 * self.macro_i + 0.08 * self.macro_b;
        (acc * 0.6).tanh() * base_gain * clampf(self.params.level, 0.0, 1.0)
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    pub fn reset(&mut self) {
        for v in self.voices.iter_mut() {
            v.active = false;
        }
        self.cooldown = 0;
        self.armed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn impulses(n: usize, period: usize) -> Vec<f32> {
        (0..n)
            .map(|i| if i % period == 0 { 0.9 } else { 0.0 })
            .collect()
    }

    fn rms(b: &[f32]) -> f32 {
        (b.iter().map(|x| x * x).sum::<f32>() / b.len() as f32).sqrt()
    }

    fn params(mut f: impl FnMut(&mut StringParams)) -> StringParams {
        let mut p = StringParams::default();
        f(&mut p);
        p
    }

    #[test]
    fn silent_when_level_zero() {
        let mut s = StringBed::new(48_000.0);
        s.set_params(
            params(|p| {
                p.level = 0.0;
                p.probability = 1.0;
            }),
            0.5,
            0.5,
            1.0,
            0.0,
        );
        let inp = impulses(48_000, 4800);
        let out: Vec<f32> = inp.iter().map(|&x| s.process(x)).collect();
        assert!(
            out.iter().all(|v| v.abs() < 1e-4),
            "bed not silent at level 0"
        );
    }

    #[test]
    fn fires_and_is_bounded() {
        let mut s = StringBed::new(48_000.0);
        s.set_params(
            params(|p| {
                p.probability = 1.0;
                p.probability_max = 0.98;
            }),
            0.7,
            0.7,
            1.0,
            0.2,
        );
        let inp = impulses(96_000, 6000);
        let mut peak = 0.0f32;
        let out: Vec<f32> = inp
            .iter()
            .map(|&x| {
                let y = s.process(x);
                assert!(y.is_finite());
                peak = peak.max(y.abs());
                y
            })
            .collect();
        assert!(rms(&out) > 1e-3, "bed produced no sound");
        assert!(peak <= 1.0, "bed bus not bounded: {peak}");
    }

    #[test]
    fn respects_voice_cap() {
        let mut s = StringBed::new(48_000.0);
        s.set_params(
            params(|p| {
                p.probability = 1.0;
                p.probability_max = 0.98;
                p.max_voices = 6;
                p.release = 4.0;
            }),
            1.0,
            1.0,
            1.0,
            0.5,
        );
        for i in 0..96_000 {
            s.process(if i % 800 == 0 { 0.9 } else { 0.0 });
            assert!(
                s.active_voices() <= 6,
                "exceeded cap: {}",
                s.active_voices()
            );
        }
    }

    #[test]
    fn probability_max_caps_firing() {
        // prob_max 0 → never fires regardless of macros
        let mut s = StringBed::new(48_000.0);
        s.set_params(
            params(|p| {
                p.probability = 1.0;
                p.probability_max = 0.0;
            }),
            1.0,
            1.0,
            1.0,
            1.0,
        );
        for _ in 0..200 {
            assert!(!s.trigger(1.0));
        }
        assert_eq!(s.active_voices(), 0);
    }

    #[test]
    fn minor_third_differs_from_major() {
        let third_hz = |minor: bool| {
            let mut s = StringBed::new(48_000.0);
            s.set_params(
                params(|p| {
                    p.probability = 1.0;
                    p.probability_max = 0.98;
                    p.minor = minor;
                }),
                1.0,
                1.0,
                1.0,
                1.0,
            );
            while !s.trigger(1.0) {}
            s.voices[1].hz // the second voice is the third of the triad
        };
        let maj = third_hz(false);
        let min = third_hz(true);
        assert!(
            min < maj,
            "minor third not lower than major: {min} vs {maj}"
        );
    }
}
