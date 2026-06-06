//! Voice characters — the XY morph system that gives Botanica its control surface.
//!
//! Five "characters" sit on a ring; the XY puck picks an angle and interpolates
//! between the two nearest. Each character is a set of *deltas* that bias the
//! filter, FX wet amounts, resonance, arp and string layers. The same XY puck
//! also derives the effective macros (intensity / bloom / motion / depth) via an
//! "orb" mapping. v1's Rust silently dropped this whole layer (hence "too little
//! control"); this restores it, with the de-chaos bounds from the port spec.
//!
//! Faithful re-implementation of our own v1 values; sound design by **Tev**.

use dsp_core::{clampf, Lfo, Svf, SvfMode, Waveform};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterKind {
    Lowpass,
    Bandpass,
    Notch,
}

impl FilterKind {
    fn mode(self) -> SvfMode {
        match self {
            FilterKind::Lowpass => SvfMode::Lowpass,
            FilterKind::Bandpass => SvfMode::Bandpass,
            FilterKind::Notch => SvfMode::Notch,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Fx {
    pub wet: f32,
    pub delay: f32,
    pub verb: f32,
    pub chorus: f32,
    pub phaser: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Res {
    pub amount: f32,
    pub q: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Arp {
    pub level: f32,
    pub bright: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Strings {
    pub prob: f32,
    pub airy: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Filt {
    pub open: f32,
    pub favor: FilterKind,
}

#[derive(Clone, Copy, Debug)]
pub struct Character {
    pub name: &'static str,
    pub fx: Fx,
    pub res: Res,
    pub arp: Arp,
    pub strings: Strings,
    pub filt: Filt,
}

/// The five ring characters (Glass Orchid → Aurora Silk). Values are deltas.
pub const CHARACTERS: [Character; 5] = [
    Character {
        name: "Glass Orchid",
        fx: Fx {
            wet: 0.12,
            delay: 0.05,
            verb: 0.18,
            chorus: 0.18,
            phaser: 0.08,
        },
        res: Res {
            amount: 0.22,
            q: 6.0,
        },
        arp: Arp {
            level: 0.18,
            bright: 0.18,
        },
        strings: Strings {
            prob: 0.10,
            airy: 0.22,
        },
        filt: Filt {
            open: 1.25,
            favor: FilterKind::Lowpass,
        },
    },
    Character {
        name: "Moss Cathedral",
        fx: Fx {
            wet: 0.10,
            delay: -0.02,
            verb: 0.28,
            chorus: 0.10,
            phaser: -0.02,
        },
        res: Res {
            amount: 0.14,
            q: 2.0,
        },
        arp: Arp {
            level: -0.06,
            bright: -0.05,
        },
        strings: Strings {
            prob: 0.22,
            airy: 0.08,
        },
        filt: Filt {
            open: 0.95,
            favor: FilterKind::Bandpass,
        },
    },
    Character {
        name: "Pollen Spark",
        fx: Fx {
            wet: 0.06,
            delay: 0.12,
            verb: 0.06,
            chorus: 0.06,
            phaser: 0.14,
        },
        res: Res {
            amount: 0.10,
            q: 3.0,
        },
        arp: Arp {
            level: 0.22,
            bright: 0.10,
        },
        strings: Strings {
            prob: 0.08,
            airy: 0.05,
        },
        filt: Filt {
            open: 1.05,
            favor: FilterKind::Notch,
        },
    },
    Character {
        name: "Root Murmur",
        fx: Fx {
            wet: -0.05,
            delay: 0.02,
            verb: 0.10,
            chorus: -0.04,
            phaser: 0.06,
        },
        res: Res {
            amount: 0.06,
            q: 1.0,
        },
        arp: Arp {
            level: -0.02,
            bright: -0.10,
        },
        strings: Strings {
            prob: 0.14,
            airy: -0.05,
        },
        filt: Filt {
            open: 0.75,
            favor: FilterKind::Lowpass,
        },
    },
    Character {
        name: "Aurora Silk",
        fx: Fx {
            wet: 0.14,
            delay: 0.02,
            verb: 0.16,
            chorus: 0.26,
            phaser: 0.04,
        },
        res: Res {
            amount: 0.16,
            q: 4.0,
        },
        arp: Arp {
            level: 0.10,
            bright: 0.08,
        },
        strings: Strings {
            prob: 0.18,
            airy: 0.14,
        },
        filt: Filt {
            open: 1.10,
            favor: FilterKind::Bandpass,
        },
    },
];

/// A character continuously interpolated between two ring neighbours.
#[derive(Clone, Copy, Debug)]
pub struct Blended {
    pub name: &'static str,
    pub fx: Fx,
    pub res: Res,
    pub arp: Arp,
    pub strings: Strings,
    pub open: f32,
    pub favor: FilterKind,
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Map the XY puck to a blended character (ring interpolation). `favor` and
/// `name` switch at the midpoint, exactly like the v1 picker.
pub fn blend(xy_x: f32, xy_y: f32) -> Blended {
    let angle = xy_y.atan2(xy_x); // -PI..PI
    let u = (angle + core::f32::consts::PI) / dsp_core::TWO_PI; // 0..1
    let x = u * 5.0;
    let i0 = (x.floor() as usize) % 5;
    let i1 = (i0 + 1) % 5;
    let t = x - x.floor();
    let a = &CHARACTERS[i0];
    let b = &CHARACTERS[i1];
    let pick = if t >= 0.5 { b } else { a };
    Blended {
        name: pick.name,
        fx: Fx {
            wet: lerp(a.fx.wet, b.fx.wet, t),
            delay: lerp(a.fx.delay, b.fx.delay, t),
            verb: lerp(a.fx.verb, b.fx.verb, t),
            chorus: lerp(a.fx.chorus, b.fx.chorus, t),
            phaser: lerp(a.fx.phaser, b.fx.phaser, t),
        },
        res: Res {
            amount: lerp(a.res.amount, b.res.amount, t),
            q: lerp(a.res.q, b.res.q, t),
        },
        arp: Arp {
            level: lerp(a.arp.level, b.arp.level, t),
            bright: lerp(a.arp.bright, b.arp.bright, t),
        },
        strings: Strings {
            prob: lerp(a.strings.prob, b.strings.prob, t),
            airy: lerp(a.strings.airy, b.strings.airy, t),
        },
        open: lerp(a.filt.open, b.filt.open, t),
        favor: pick.filt.favor,
    }
}

/// The effective macros derived from the XY puck blended with the explicit knobs.
#[derive(Clone, Copy, Debug)]
pub struct EffMacros {
    pub intensity: f32,
    pub bloom: f32,
    pub motion: f32,
    pub depth: f32,
    pub radius: f32,
}

/// `mix` (0..1) = how much the XY "orb" biases the knobs (v1 used 0.45).
pub fn eff_macros(
    xy_x: f32,
    xy_y: f32,
    intensity_knob: f32,
    bloom_knob: f32,
    motion_knob: f32,
    mix: f32,
) -> EffMacros {
    let px = clampf(xy_x, -1.0, 1.0);
    let py = clampf(xy_y, -1.0, 1.0);
    let i = px * 0.5 + 0.5;
    let b = -py * 0.5 + 0.5;
    let rr = (px * px + py * py).sqrt().min(1.0);
    let orb_i = 0.18 + i * 0.82;
    let orb_b = 0.18 + b * 0.82;
    let orb_m = 0.15 + rr * 0.85;
    let orb_d = clampf(0.10 + i * 0.75 + rr * 0.25, 0.0, 1.0);
    let m = clampf(mix, 0.0, 1.0);
    EffMacros {
        intensity: clampf((1.0 - m) * intensity_knob + m * orb_i, 0.0, 1.0),
        bloom: clampf((1.0 - m) * bloom_knob + m * orb_b, 0.0, 1.0),
        motion: clampf((1.0 - m) * motion_knob + m * orb_m, 0.0, 1.0),
        depth: orb_d,
        radius: rr,
    }
}

/// The character-shaped multimode filter with a bounded slow cutoff LFO.
pub struct CharacterFilter {
    svf: Svf,
    lfo: Lfo,
    sample_rate: f32,
}

impl CharacterFilter {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            svf: Svf::new(sample_rate),
            lfo: Lfo::new(sample_rate),
            sample_rate: sample_rate.max(1.0),
        }
    }

    pub fn reset(&mut self) {
        self.svf.reset();
        self.lfo.reset(0.0);
    }

    /// `character_q` 0..1 pushes resonance; `lfo_rate` is bounded by the caller.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn process(
        &mut self,
        x: f32,
        b: &Blended,
        bloom: f32,
        motion: f32,
        intensity: f32,
        character_q: f32,
        lfo_rate: f32,
    ) -> f32 {
        let base = (350.0 + 6500.0 * bloom) * (0.55 + 0.45 * b.open);
        let lfo_v = self.lfo.next(clampf(lfo_rate, 0.02, 0.5), Waveform::Sine);
        let favor_scale = if b.favor == FilterKind::Lowpass {
            1.0
        } else {
            0.7
        };
        let depth = (120.0 + 1200.0 * motion) * (0.35 + intensity * 0.65) * favor_scale;
        let nyq = self.sample_rate * 0.45;
        let cutoff = clampf(base + lfo_v * depth, 20.0, nyq);
        let q = clampf(0.7 + b.res.q.max(0.0) * (0.3 + character_q), 0.5, 24.0);
        self.svf.set(cutoff, q);
        self.svf.process(x, b.favor.mode())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_is_continuous_and_wraps() {
        // small angle change -> small parameter change (no jumps except the
        // name/favor switch which is allowed at the midpoint)
        let a = blend(1.0, 0.0);
        let b = blend(1.0, 0.001);
        assert!((a.open - b.open).abs() < 0.01);
        // sweeping all the way round returns near the start
        let start = blend(1.0, 0.0);
        let round = blend((dsp_core::TWO_PI).cos(), (dsp_core::TWO_PI).sin());
        assert!((start.open - round.open).abs() < 0.05);
    }

    #[test]
    fn all_five_characters_are_reachable() {
        let mut seen = std::collections::HashSet::new();
        for k in 0..360 {
            let ang = k as f32 * core::f32::consts::PI / 180.0;
            seen.insert(blend(ang.cos(), ang.sin()).name);
        }
        assert_eq!(seen.len(), 5, "not all characters reachable: {seen:?}");
    }

    #[test]
    fn eff_macros_stay_in_range_and_track_xy() {
        for &(x, y) in &[(-1.0, -1.0), (1.0, 1.0), (0.0, 0.0), (0.7, -0.3)] {
            let m = eff_macros(x, y, 0.5, 0.5, 0.5, 0.45);
            for v in [m.intensity, m.bloom, m.motion, m.depth, m.radius] {
                assert!((0.0..=1.0).contains(&v), "macro out of range: {v}");
            }
        }
        // right side (px=+1) raises intensity vs left side
        let right = eff_macros(1.0, 0.0, 0.5, 0.5, 0.5, 1.0).intensity;
        let left = eff_macros(-1.0, 0.0, 0.5, 0.5, 0.5, 1.0).intensity;
        assert!(right > left);
    }

    #[test]
    fn character_filter_is_stable_and_bounded() {
        let mut f = CharacterFilter::new(48_000.0);
        let b = blend(0.5, 0.5);
        let mut peak = 0.0f32;
        for i in 0..48_000 {
            let x = (core::f32::consts::TAU * 220.0 * i as f32 / 48_000.0).sin();
            let y = f.process(x, &b, 0.8, 0.6, 0.7, 1.0, 0.12);
            assert!(y.is_finite());
            peak = peak.max(y.abs());
        }
        assert!(peak < 8.0, "character filter unstable: peak {peak}");
    }
}
