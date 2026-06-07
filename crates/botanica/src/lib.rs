//! Botanica — a granular sample-morph instrument.
//!
//! Inspired by Synplant; sound design by **Tev**. Original implementation. Built
//! entirely on `dsp-core`, so the web (wasm) build and the native plugin run the
//! same DSP. The XY-character control surface (this commit) restores what v1's
//! Rust silently dropped — the puck drives both the character ring AND the
//! effective macros, with the de-chaos bounds from the port spec. The remaining
//! advanced modules (resonance, transient arp, granular freeze, string layers)
//! slot in next, each behind clear parameters. Real-time contract: no alloc/lock
//! in `process`; the puck + macros are smoothed.

#![forbid(unsafe_code)]

pub mod arp;
pub mod freeze;
pub mod resonance;
pub mod strings;
pub mod voice_characters;

use arp::Arp;
use dsp_core::{Chorus, Delay, Drive, Oscillator, Reverb, Smoothed, Waveform};
use freeze::GranularFreeze;
use resonance::ResonanceBank;
use strings::{StringBed, StringParams};
use voice_characters::{blend, eff_macros, CharacterFilter};

/// User-facing parameters. Macros are normalised 0..1 unless noted.
#[derive(Clone, Copy, Debug)]
pub struct BotanicaParams {
    pub blend: f32,           // tone/drive blend
    pub intensity: f32,       // energy knob (blended with the XY orb)
    pub bloom: f32,           // brightness/reverb knob (blended with the orb)
    pub motion: f32,          // movement knob (blended with the orb)
    pub xy_x: f32,            // XY puck X in [-1,1] (ring angle + intensity bias)
    pub xy_y: f32,            // XY puck Y in [-1,1] (ring angle + bloom bias)
    pub freeze: f32,          // 0/1 granular freeze (holds the loop; floors gain)
    pub retune_semis: f32,    // sample-loop pitch, clamped ratio 0.35..2.5
    pub character_q: f32,     // 0..1 push the character filter resonance
    pub filter_lfo_rate: f32, // bounded cutoff-LFO rate (Hz)
    pub xy_macro_mix: f32,    // 0..1 how much the orb biases the knobs
    pub resonance: f32,       // 0..1 harmonic resonance bank amount
    pub resonance_tilt: f32,  // -1..1 resonance brightness
    pub arp_amount: f32,      // 0..1 transient-arp level
    pub arp_density: f32,     // 0..1 arp trigger density
    pub freeze_size: f32,     // 0..1 granular grain size
    pub freeze_spray: f32,    // 0..1 granular position jitter
    pub strings_level: f32,   // 0..1 choir/string bed mix (0 = off)
    pub strings_air: f32,     // 0..1 breath/bow transient amount
    pub strings_density: f32, // 0..1 chord-fire probability bias
    pub strings_tone: f32,    // 0..1 dark→bright body of the bed
    // Spark layer (Tev A2): the "plucky sine" sparkle as an explicitly
    // switchable layer. When `spark_on` flips to 0 the whole layer's gain
    // ramps to silence (declick) so there is no click "in the plucks absence".
    pub spark_on: f32,   // 0/1 master enable for the spark/pluck layer
    pub spark_rate: f32, // Hz free-running pluck rate (the "constant" pollen)
}

impl Default for BotanicaParams {
    fn default() -> Self {
        Self {
            blend: 0.4,
            intensity: 0.55,
            bloom: 0.65,
            motion: 0.55,
            xy_x: 0.0,
            xy_y: 0.0,
            freeze: 0.0,
            retune_semis: 0.0,
            character_q: 0.3,
            filter_lfo_rate: 0.12,
            xy_macro_mix: 0.45,
            resonance: 0.25,
            resonance_tilt: 0.0,
            arp_amount: 0.3,
            arp_density: 0.4,
            freeze_size: 0.5,
            freeze_spray: 0.3,
            strings_level: 0.0,
            strings_air: 0.4,
            strings_density: 0.4,
            strings_tone: 0.5,
            spark_on: 1.0,   // on by default — it is Botanica's signature shimmer
            spark_rate: 2.4, // Hz; v1's pollen sat ~0.2..5 Hz
        }
    }
}

/// MIDI note the sample loop plays at its natural (retune-only) pitch. Playing
/// this note = unison with the drone; other notes transpose around it. C3.
const ROOT_REFERENCE_MIDI: f32 = 48.0;

/// 12-bit allowed-semitone mask the spark arp quantises to, relative to the
/// played root: a major scale (root, 2, 4, 5, 7, 9, 11). Bit i = semitone i.
const MAJOR_KEY_MASK: u16 = 0b1010_1101_0101;

/// Last-note-priority held-note stack. Botanica is a single morphing texture,
/// not a polyphonic sampler, so playing it is *mono*: the most-recently pressed
/// (and still-held) note steers the whole instrument's pitch + key. A fixed
/// array (never allocates) tracks the held order; releasing the top note falls
/// back to the previous one, like a classic mono synth.
#[derive(Clone, Copy)]
struct NoteStack {
    notes: [u8; Self::CAP],
    len: usize,
}

impl NoteStack {
    const CAP: usize = 16;

    fn new() -> Self {
        Self {
            notes: [0; Self::CAP],
            len: 0,
        }
    }

    fn push(&mut self, note: u8) {
        // de-dup, then append (most recent last). Drop the oldest if full.
        self.remove(note);
        if self.len == Self::CAP {
            self.notes.copy_within(1..Self::CAP, 0);
            self.len -= 1;
        }
        self.notes[self.len] = note;
        self.len += 1;
    }

    fn remove(&mut self, note: u8) {
        if let Some(i) = self.notes[..self.len].iter().position(|&n| n == note) {
            for j in i..self.len - 1 {
                self.notes[j] = self.notes[j + 1];
            }
            self.len -= 1;
        }
    }

    fn clear(&mut self) {
        self.len = 0;
    }

    fn top(&self) -> Option<u8> {
        if self.len == 0 {
            None
        } else {
            Some(self.notes[self.len - 1])
        }
    }
}

/// One loaded sample, looped with linear interpolation + pitch ratio.
#[derive(Clone, Debug, Default)]
struct SampleVoice {
    data: Vec<f32>,
    src_rate: f32,
    pos: f32,
}

impl SampleVoice {
    fn loaded(&self) -> bool {
        !self.data.is_empty()
    }

    #[inline]
    fn next(&mut self, engine_rate: f32, hold: bool, pitch_ratio: f32) -> f32 {
        let n = self.data.len();
        if n == 0 {
            return 0.0;
        }
        let i0 = self.pos.floor() as usize % n;
        let i1 = (i0 + 1) % n;
        let fr = self.pos - self.pos.floor();
        let out = self.data[i0] * (1.0 - fr) + self.data[i1] * fr;
        if !hold {
            let step = (self.src_rate / engine_rate * pitch_ratio).max(0.0);
            self.pos += step;
            while self.pos >= n as f32 {
                self.pos -= n as f32;
            }
        }
        out
    }
}

pub struct Engine {
    sample_rate: f32,
    voice: SampleVoice,
    fallback: Oscillator,
    char_filter: CharacterFilter,
    res: ResonanceBank,
    arp: Arp,
    freeze_mod: GranularFreeze,
    strings: StringBed,
    frozen: bool,
    drive: Drive,
    chorus: Chorus,
    delay: Delay,
    reverb: Reverb,
    // smoothed controls (no clicks)
    xy_x_s: Smoothed,
    xy_y_s: Smoothed,
    int_s: Smoothed,
    bloom_s: Smoothed,
    motion_s: Smoothed,
    drive_amt: Smoothed,
    gain: Smoothed,
    spark_gain: Smoothed, // declick gate for the spark/pluck layer (A2)
    // ── playability (A1): the keyboard steers pitch + key, Cube-style ──
    held: NoteStack,        // mono last-note priority
    note_semis_s: Smoothed, // smoothed played-note transpose (no pitch zipper)
    root_midi: f32,         // last-played root the generative layers key to
    note_vel: f32,          // velocity of the active note (drives intensity bias)
    params: BotanicaParams,
}

impl Engine {
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        let p = BotanicaParams::default();
        let mut e = Self {
            sample_rate: sr,
            voice: SampleVoice::default(),
            fallback: Oscillator::new(sr),
            char_filter: CharacterFilter::new(sr),
            res: ResonanceBank::new(sr),
            arp: Arp::new(sr),
            freeze_mod: GranularFreeze::new(sr),
            strings: StringBed::new(sr),
            frozen: false,
            drive: Drive::default(),
            chorus: Chorus::new(sr),
            delay: Delay::new(sr, 1.0),
            reverb: Reverb::new(sr),
            xy_x_s: Smoothed::new(p.xy_x, 0.008, sr),
            xy_y_s: Smoothed::new(p.xy_y, 0.008, sr),
            int_s: Smoothed::new(p.intensity, 0.025, sr),
            bloom_s: Smoothed::new(p.bloom, 0.025, sr),
            motion_s: Smoothed::new(p.motion, 0.025, sr),
            drive_amt: Smoothed::new(1.0 + p.blend * 5.0, 0.02, sr),
            gain: Smoothed::new(p.intensity, 0.02, sr),
            // ~12 ms gate: fast enough to feel responsive when toggling, slow
            // enough that the layer fades in/out instead of clicking.
            spark_gain: Smoothed::new(if p.spark_on >= 0.5 { 1.0 } else { 0.0 }, 0.012, sr),
            held: NoteStack::new(),
            // ~18 ms pitch glide: legato/last-note transitions are smooth, not zippered
            note_semis_s: Smoothed::new(0.0, 0.018, sr),
            root_midi: ROOT_REFERENCE_MIDI,
            note_vel: 0.0,
            params: p,
        };
        e.delay.set(0.24, 0.25, 0.18, 0.3);
        e.retarget();
        e
    }

    pub fn set_sample(&mut self, data: Vec<f32>, src_rate: f32) {
        self.voice = SampleVoice {
            data,
            src_rate: src_rate.max(1.0),
            pos: 0.0,
        };
    }

    pub fn clear_sample(&mut self) {
        self.voice = SampleVoice::default();
    }

    /// Play a note (A1). Mono, last-note priority: the most recent held note
    /// transposes the sample loop and becomes the root the generative layers
    /// (arp, strings, resonance) lock to — Cube-style, the note you play decides
    /// the key. RT-safe: no alloc, just a fixed-array push + smoother retarget.
    pub fn note_on(&mut self, midi: f32, vel: f32) {
        let m = midi.clamp(0.0, 127.0);
        self.held.push(m as u8);
        self.note_vel = vel.clamp(0.0, 1.0);
        self.update_pitch_target();
    }

    /// Release a note. If notes remain, fall back to the previous one (legato);
    /// if none remain, the loop glides back to the retune drone but the last
    /// root is kept so the generative layers stay musically coherent.
    pub fn note_off(&mut self, midi: f32) {
        let m = midi.clamp(0.0, 127.0) as u8;
        self.held.remove(m);
        self.update_pitch_target();
    }

    /// Release every note (panic button / instrument switch).
    pub fn all_notes_off(&mut self) {
        self.held.clear();
        self.update_pitch_target();
    }

    /// Number of currently-held notes (for UI display).
    pub fn held_notes(&self) -> usize {
        self.held.len
    }

    /// Re-point the pitch glide + generative root from the held-note stack.
    /// When a note is held it sets both; when none is held the transpose glides
    /// back to 0 (the retune drone) while the root memory is preserved.
    fn update_pitch_target(&mut self) {
        match self.held.top() {
            Some(note) => {
                let n = note as f32;
                self.root_midi = n;
                self.note_semis_s.set_target(n - ROOT_REFERENCE_MIDI);
            }
            None => {
                // graceful drone: return the loop to its retune pitch; keep the
                // last root so arp/strings/resonance don't lurch off-key.
                self.note_semis_s.set_target(0.0);
            }
        }
        // resonance bank re-tracks the (possibly new) root immediately.
        self.update_resonance_pitch();
    }

    /// Resonance fundamental follows the played root + retune (A1).
    fn update_resonance_pitch(&mut self) {
        let semis = self.root_offset_semis() + self.params.retune_semis;
        let ratio = (2.0_f32).powf(semis / 12.0).clamp(0.25, 4.0);
        self.res.set_pitch(110.0 * ratio);
    }

    /// Semitone offset of the active root relative to the reference (0 when no
    /// note has ever been played).
    #[inline]
    fn root_offset_semis(&self) -> f32 {
        self.root_midi - ROOT_REFERENCE_MIDI
    }

    pub fn set_params(&mut self, p: BotanicaParams) {
        self.params = p;
        // freeze edge: capture a window on the rising edge (off the audio thread)
        let now = p.freeze >= 0.5;
        if now && !self.frozen {
            if self.voice.loaded() {
                let center = self.voice.pos as usize;
                self.freeze_mod.capture(&self.voice.data, center, 0.3);
                self.frozen = true;
            }
        } else if !now && self.frozen {
            self.freeze_mod.clear();
            self.frozen = false;
        }
        self.retarget();
    }

    /// Push parameter values to the smoothers (called off the hot loop on change).
    fn retarget(&mut self) {
        let p = self.params; // Copy — no borrow of self while we touch sub-modules
        self.xy_x_s.set_target(p.xy_x.clamp(-1.0, 1.0));
        self.xy_y_s.set_target(p.xy_y.clamp(-1.0, 1.0));
        self.int_s.set_target(p.intensity.clamp(0.0, 1.0));
        self.bloom_s.set_target(p.bloom.clamp(0.0, 1.0));
        self.motion_s.set_target(p.motion.clamp(0.0, 1.0));
        self.drive_amt
            .set_target(1.0 + p.blend.clamp(0.0, 1.0) * 5.0);
        // resonance bank tracks the played root + retune (A1)
        self.update_resonance_pitch();
        // spark layer: declick gate target + free-running pluck rate (A2)
        self.spark_gain
            .set_target(if p.spark_on >= 0.5 { 1.0 } else { 0.0 });
        let rate = p.spark_rate.clamp(0.1, 8.0);
        self.arp.set_clock(false, rate, 120.0, 2.0);
        // generative layers lock to the played root (A1): the spark arp arps a
        // major-ish scale around the root; the string bed is built on the same
        // key. This is the "always in key" feel of v1, anchored on the note the
        // player holds — not a hidden algorithm.
        self.arp.set_key(self.root_midi, MAJOR_KEY_MASK);
    }

    /// Sample-loop playback ratio for a given live note transpose (semitones).
    /// Folds the retune knob + the played-note transpose, clamped to the loop's
    /// safe range. Kept a touch wider than the original 0.35..2.5 so a 2-octave
    /// keyboard span doesn't pin at the edges.
    #[inline]
    fn pitch_ratio(&self, note_semis: f32) -> f32 {
        (2.0_f32)
            .powf((self.params.retune_semis + note_semis) / 12.0)
            .clamp(0.25, 4.0)
    }

    /// Produce one mono sample.
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn next(&mut self) -> f32 {
        let hold = self.params.freeze >= 0.5;

        // smoothed puck + knobs → blended character + effective macros
        let x = self.xy_x_s.next();
        let y = self.xy_y_s.next();
        let ik = self.int_s.next();
        let bk = self.bloom_s.next();
        let mk = self.motion_s.next();
        let bl = blend(x, y);
        let eff = eff_macros(x, y, ik, bk, mk, self.params.xy_macro_mix);

        // played-note transpose (A1), smoothed so legato glides without zipper.
        let note_semis = self.note_semis_s.next();
        let ratio = self.pitch_ratio(note_semis);

        // voice → character filter. The loop transposes with the note; the
        // built-in fallback tone is likewise playable (110 Hz at the reference
        // note), so Botanica responds to the keyboard with or without a sample.
        let dry = if self.voice.loaded() {
            self.voice.next(self.sample_rate, hold, ratio)
        } else {
            let f = (110.0 * ratio).clamp(20.0, self.sample_rate * 0.45);
            0.4 * self.fallback.next(f, Waveform::Triangle)
        };
        let shaped = self.char_filter.process(
            dry,
            &bl,
            eff.bloom,
            eff.motion,
            eff.intensity,
            self.params.character_q,
            self.params.filter_lfo_rate,
        );

        // harmonic resonance bank (amount + Q follow the character)
        let res_amt = (self.params.resonance * (0.4 + bl.res.amount * 2.0)).clamp(0.0, 1.0);
        let res_q = (2.0 + bl.res.q + self.params.character_q * 6.0).clamp(0.5, 24.0);
        self.res.set_params(
            res_amt,
            res_q,
            self.params.resonance_tilt,
            1.0,
            80.0,
            9000.0,
        );
        let voiced = self.res.process(shaped);

        // transient arp (key-locked sparkle; level + brightness follow the character)
        let arp_lvl =
            (self.params.arp_amount * (0.4 + bl.arp.level.max(0.0) * 2.0)).clamp(0.0, 1.0);
        self.arp.set_params(
            arp_lvl,
            self.params.arp_density,
            (0.2 + bl.arp.bright).clamp(0.0, 1.0),
            5.0,
            0.18,
            6,
        );
        // granular freeze (sustained pad from a captured window; clamped grain rate)
        let frz = if self.frozen || self.freeze_mod.is_active() {
            self.freeze_mod.set_params(
                self.params.freeze >= 0.5,
                0.2 + eff.depth * 0.7,
                self.params.freeze_size,
                0.5,
                self.params.freeze_spray,
                ratio,
                0.3,
            );
            self.freeze_mod.process()
        } else {
            0.0
        };

        // choir/string bed (fired by the same transients; the character's strings
        // gene biases probability + airiness; skipped entirely when level is 0)
        let strings_bus = if self.params.strings_level > 1e-4 || self.strings.active_voices() > 0 {
            self.strings.set_params(
                StringParams {
                    level: self.params.strings_level,
                    airiness: (self.params.strings_air * (0.6 + bl.strings.airy.max(0.0) * 3.0))
                        .clamp(0.0, 1.0),
                    body_tone: self.params.strings_tone,
                    // bed root follows the played note (A1): the chord is built
                    // on the held key, clamped into the bed's natural register.
                    octave_base: self.root_midi.clamp(24.0, 60.0),
                    key: 0.0,
                    ..StringParams::default()
                },
                eff.intensity,
                eff.bloom,
                self.params.strings_density,
                bl.strings.prob.max(0.0),
            );
            self.strings.process(dry)
        } else {
            0.0
        };

        // Spark layer: always advance the arp (its clock/voices keep state so a
        // re-enable is seamless) but multiply its bus by the declick gate. When
        // `spark_on` is 0 the gate ramps to 0, so the layer fades to true silence
        // with no click "in the plucks absence" (Tev A2).
        let spark = self.arp.process(dry) * 0.6 * self.spark_gain.next();
        let pre_fx = voiced + spark + frz + strings_bus;

        // FX amounts driven by the effective macros + the character deltas
        self.drive.drive = self.drive_amt.next();
        self.chorus.rate_hz = 0.1 + eff.motion * 3.0;
        self.chorus.depth_ms = (1.0 + eff.motion * 6.0) * (1.0 + bl.fx.chorus).max(0.2);
        let dfb = (0.25 + bl.fx.delay).clamp(0.0, 0.9);
        self.delay
            .set(0.24, dfb, (0.18 + bl.fx.delay * 0.4).clamp(0.0, 0.5), 0.3);
        let verb = (eff.bloom * 0.55 + bl.fx.verb).clamp(0.0, 0.9);
        self.reverb.set(eff.bloom, 0.3, verb);

        let mut s = self.drive.process(pre_fx);
        s = self.chorus.process(s);
        s = self.delay.process(s);
        s = self.reverb.process(s);

        // freeze floors the gain so the loop sustains rather than going silent
        let target_gain = if hold {
            eff.intensity.max(0.15)
        } else {
            eff.intensity
        };
        self.gain.set_target(target_gain);
        s * self.gain.next()
    }

    /// Fill a mono buffer (offline render / tests).
    pub fn process(&mut self, out: &mut [f32]) {
        for x in out.iter_mut() {
            *x = self.next();
        }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// The currently-active character name (for UI display).
    pub fn character_name(&self) -> &'static str {
        blend(self.xy_x_s.value(), self.xy_y_s.value()).name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(b: &[f32]) -> f32 {
        (b.iter().map(|x| x * x).sum::<f32>() / b.len() as f32).sqrt()
    }

    #[test]
    fn makes_finite_bounded_sound_without_a_sample() {
        let mut e = Engine::new(48_000.0);
        let mut buf = vec![0.0; 48_000];
        e.process(&mut buf);
        assert!(buf.iter().all(|v| v.is_finite() && v.abs() <= 4.0));
        assert!(rms(&buf) > 0.005, "engine was silent");
    }

    #[test]
    fn strings_bed_adds_energy_and_stays_bounded() {
        // a transient-rich sample so the bed's onset gate fires
        let sample: Vec<f32> = (0..48_000)
            .map(|i| if i % 6000 == 0 { 0.9 } else { 0.0 })
            .collect();
        let render = |level: f32| {
            let mut e = Engine::new(48_000.0);
            e.set_sample(sample.clone(), 48_000.0);
            e.set_params(BotanicaParams {
                strings_level: level,
                ..BotanicaParams::default()
            });
            let mut buf = vec![0.0; 96_000];
            e.process(&mut buf);
            assert!(buf.iter().all(|v| v.is_finite() && v.abs() <= 4.0));
            rms(&buf)
        };
        let off = render(0.0);
        let on = render(0.9);
        assert!(on > off, "strings bed added no energy: off {off} on {on}");
    }

    #[test]
    fn loads_and_loops_a_sample() {
        let mut e = Engine::new(48_000.0);
        let sample: Vec<f32> = (0..4800)
            .map(|i| (core::f32::consts::TAU * 1000.0 * i as f32 / 48_000.0).sin())
            .collect();
        e.set_sample(sample, 48_000.0);
        let mut buf = vec![0.0; 48_000];
        e.process(&mut buf);
        assert!(buf.iter().all(|v| v.is_finite()));
        assert!(rms(&buf) > 0.02);
    }

    #[test]
    fn freeze_holds_position() {
        let mut e = Engine::new(48_000.0);
        let sample: Vec<f32> = (0..1000).map(|i| (i as f32 / 1000.0) * 2.0 - 1.0).collect();
        e.set_sample(sample, 48_000.0);
        e.set_params(BotanicaParams {
            freeze: 1.0,
            ..BotanicaParams::default()
        });
        // run a bit so the smoothed puck settles, then sample the held position
        for _ in 0..2000 {
            e.next();
        }
        let pos_before = e.voice.pos;
        for _ in 0..500 {
            e.next();
        }
        assert!(
            (e.voice.pos - pos_before).abs() < 1e-3,
            "freeze did not hold position"
        );
    }

    #[test]
    fn retune_changes_playback_speed() {
        let mut e = Engine::new(48_000.0);
        let sample: Vec<f32> = (0..1000).map(|i| (i as f32 / 1000.0)).collect();
        e.set_sample(sample.clone(), 48_000.0);
        // +12 semitones => ~2x speed => position advances ~2x further
        e.set_params(BotanicaParams {
            retune_semis: 12.0,
            ..BotanicaParams::default()
        });
        for _ in 0..400 {
            e.next();
        }
        let fast = e.voice.pos;
        e.set_sample(sample, 48_000.0);
        e.set_params(BotanicaParams {
            retune_semis: 0.0,
            ..BotanicaParams::default()
        });
        for _ in 0..400 {
            e.next();
        }
        let normal = e.voice.pos;
        assert!(
            fast > normal * 1.5,
            "retune did not speed up: fast {fast} normal {normal}"
        );
    }

    #[test]
    fn spark_toggle_fades_without_a_click() {
        // A2: turning the spark layer off must ramp to silence (no step). We
        // crank the spark, let it ring, flip it off and assert the per-sample
        // delta never jumps full-scale across the transition.
        let mut e = Engine::new(48_000.0);
        e.set_params(BotanicaParams {
            spark_on: 1.0,
            arp_amount: 1.0,
            arp_density: 1.0,
            // mute everything else so we isolate the spark layer's tail
            intensity: 0.0,
            bloom: 0.0,
            resonance: 0.0,
            strings_level: 0.0,
            ..BotanicaParams::default()
        });
        for _ in 0..24_000 {
            e.next();
        }
        // Flip spark off mid-ring and capture the transition.
        e.set_params(BotanicaParams {
            spark_on: 0.0,
            arp_amount: 1.0,
            arp_density: 1.0,
            intensity: 0.0,
            bloom: 0.0,
            resonance: 0.0,
            strings_level: 0.0,
            ..BotanicaParams::default()
        });
        let mut prev = e.next();
        let mut max_delta = 0.0f32;
        for _ in 0..48_000 {
            let s = e.next();
            assert!(s.is_finite());
            max_delta = max_delta.max((s - prev).abs());
            prev = s;
        }
        assert!(
            max_delta < 0.2,
            "spark toggle-off stepped (click): max delta {max_delta}"
        );
    }

    #[test]
    fn spark_off_is_quieter_than_on() {
        // With intensity/resonance neutral, disabling spark must lower energy
        // (the layer is actually gated, not just renamed).
        let render = |on: f32| {
            let mut e = Engine::new(48_000.0);
            e.set_params(BotanicaParams {
                spark_on: on,
                arp_amount: 1.0,
                arp_density: 1.0,
                intensity: 0.0,
                bloom: 0.0,
                resonance: 0.0,
                strings_level: 0.0,
                ..BotanicaParams::default()
            });
            let mut buf = vec![0.0; 96_000];
            e.process(&mut buf);
            assert!(buf.iter().all(|v| v.is_finite()));
            rms(&buf)
        };
        let on = render(1.0);
        let off = render(0.0);
        assert!(off < on, "spark off was not quieter: on {on} off {off}");
    }

    #[test]
    fn note_on_transposes_the_loop() {
        // A1: playing a note above the reference must speed the loop up.
        let sample: Vec<f32> = (0..1000).map(|i| i as f32 / 1000.0).collect();
        let advance = |midi: f32| {
            let mut e = Engine::new(48_000.0);
            e.set_sample(sample.clone(), 48_000.0);
            e.note_on(midi, 0.9);
            // let the pitch glide settle, then measure travel over a fixed span
            for _ in 0..2000 {
                e.next();
            }
            let p0 = e.voice.pos;
            for _ in 0..400 {
                e.next();
            }
            (e.voice.pos - p0 + 1000.0) % 1000.0
        };
        let low = advance(ROOT_REFERENCE_MIDI); // unison
        let high = advance(ROOT_REFERENCE_MIDI + 12.0); // +1 octave ≈ 2×
        assert!(
            high > low * 1.5,
            "note did not transpose the loop: low {low} high {high}"
        );
    }

    #[test]
    fn mono_last_note_priority() {
        // Holding two notes tracks the most recent; releasing it falls back to
        // the earlier one (classic mono synth), and the root follows.
        let mut e = Engine::new(48_000.0);
        e.note_on(48.0, 1.0);
        assert_eq!(e.root_midi, 48.0);
        e.note_on(55.0, 1.0);
        assert_eq!(e.root_midi, 55.0);
        assert_eq!(e.held_notes(), 2);
        e.note_off(55.0);
        assert_eq!(e.root_midi, 48.0, "did not fall back to the held note");
        assert_eq!(e.held_notes(), 1);
    }

    #[test]
    fn all_notes_off_returns_to_drone_but_keeps_root() {
        // Releasing everything glides the loop transpose back to 0 (the retune
        // drone) but keeps the last root so the layers stay in key.
        let mut e = Engine::new(48_000.0);
        e.note_on(60.0, 1.0);
        for _ in 0..1000 {
            e.next();
        }
        e.all_notes_off();
        assert_eq!(e.held_notes(), 0);
        assert_eq!(e.root_midi, 60.0, "root memory was lost");
        for _ in 0..6000 {
            e.next();
        }
        // transpose has glided back toward unison
        assert!(
            e.note_semis_s.value().abs() < 0.5,
            "did not return to the drone pitch: {}",
            e.note_semis_s.value()
        );
    }

    #[test]
    fn playing_stays_finite_and_bounded() {
        let mut e = Engine::new(48_000.0);
        let sample: Vec<f32> = (0..4800)
            .map(|i| (core::f32::consts::TAU * 220.0 * i as f32 / 48_000.0).sin())
            .collect();
        e.set_sample(sample, 48_000.0);
        for n in [36.0, 48.0, 60.0, 67.0, 72.0] {
            e.note_on(n, 0.9);
            let mut buf = vec![0.0; 9600];
            e.process(&mut buf);
            assert!(buf.iter().all(|v| v.is_finite() && v.abs() <= 4.0));
            e.note_off(n);
        }
    }

    #[test]
    fn note_stack_dedups_and_caps() {
        let mut s = NoteStack::new();
        s.push(60);
        s.push(60); // de-dup
        assert_eq!(s.len, 1);
        for n in 0..(NoteStack::CAP as u8 + 4) {
            s.push(100 + n);
        }
        assert_eq!(s.len, NoteStack::CAP, "stack overflowed its fixed cap");
        // top is the most recent push
        assert_eq!(s.top(), Some(100 + NoteStack::CAP as u8 + 3));
    }

    #[test]
    fn xy_selects_a_character() {
        let mut e = Engine::new(48_000.0);
        e.set_params(BotanicaParams {
            xy_x: 1.0,
            xy_y: 0.0,
            ..BotanicaParams::default()
        });
        for _ in 0..1000 {
            e.next();
        }
        let a = e.character_name();
        e.set_params(BotanicaParams {
            xy_x: -1.0,
            xy_y: 0.0,
            ..BotanicaParams::default()
        });
        for _ in 0..1000 {
            e.next();
        }
        let b = e.character_name();
        assert_ne!(a, b, "opposite puck positions gave the same character");
    }
}
