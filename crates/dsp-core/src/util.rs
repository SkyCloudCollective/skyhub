//! Small shared helpers. No allocation, no panics.

pub const TWO_PI: f32 = core::f32::consts::TAU;

/// Branch-free clamp (NaN maps to `lo`).
#[inline]
pub fn clampf(x: f32, lo: f32, hi: f32) -> f32 {
    if x < lo || x.is_nan() {
        lo
    } else if x > hi {
        hi
    } else {
        x
    }
}

/// Convert decibels to a linear gain.
#[inline]
pub fn db_to_gain(db: f32) -> f32 {
    // 10^(db/20)
    10.0_f32.powf(db * 0.05)
}

/// Flush denormals to zero. Denormal floats stall feedback paths (filters,
/// reverb tails, delays) on some CPUs; this keeps the audio thread cheap.
#[inline]
pub fn flush_denormal(x: f32) -> f32 {
    if x.abs() < 1.0e-20 {
        0.0
    } else {
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_bounds_and_nan() {
        assert_eq!(clampf(5.0, 0.0, 1.0), 1.0);
        assert_eq!(clampf(-5.0, 0.0, 1.0), 0.0);
        assert_eq!(clampf(0.3, 0.0, 1.0), 0.3);
        assert_eq!(clampf(f32::NAN, 0.0, 1.0), 0.0);
    }

    #[test]
    fn db_zero_is_unity() {
        assert!((db_to_gain(0.0) - 1.0).abs() < 1e-6);
        assert!((db_to_gain(-6.0) - 0.5012).abs() < 1e-3);
    }

    #[test]
    fn denormals_flush() {
        assert_eq!(flush_denormal(1.0e-30), 0.0);
        assert_eq!(flush_denormal(0.5), 0.5);
    }
}
