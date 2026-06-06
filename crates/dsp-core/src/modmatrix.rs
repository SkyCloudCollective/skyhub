//! Modulation matrix — route any modulation source onto any parameter.
//!
//! This is the heart of PhasePlan's "open modulation". Routes are configured
//! off the audio thread (they allocate); `process` is allocation-free: it reads
//! the current source values and adds `depth * source` to each target's base.
//!
//! Sources and targets are referenced by index; the instrument owns the
//! meaning of each index (e.g. source 0 = LFO1, target 3 = filter cutoff).

#[derive(Clone, Copy, Debug)]
pub struct ModRoute {
    pub source: usize,
    pub target: usize,
    pub depth: f32, // bipolar; added as depth * source_value
}

#[derive(Clone, Debug, Default)]
pub struct ModMatrix {
    routes: Vec<ModRoute>,
}

impl ModMatrix {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Replace all routes (off the audio thread).
    pub fn set_routes(&mut self, routes: &[ModRoute]) {
        self.routes.clear();
        self.routes.extend_from_slice(routes);
    }

    pub fn add(&mut self, source: usize, target: usize, depth: f32) {
        self.routes.push(ModRoute {
            source,
            target,
            depth,
        });
    }

    pub fn clear(&mut self) {
        self.routes.clear();
    }

    pub fn len(&self) -> usize {
        self.routes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// Apply every route: `targets[t] += depth * sources[s]`. Out-of-range
    /// indices are skipped (defensive — never panics in the audio thread).
    #[inline]
    pub fn apply(&self, sources: &[f32], targets: &mut [f32]) {
        for r in &self.routes {
            if r.source < sources.len() && r.target < targets.len() {
                targets[r.target] += r.depth * sources[r.source];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_sum_onto_targets() {
        let mut m = ModMatrix::new();
        m.add(0, 1, 0.5); // src0 -> tgt1 * 0.5
        m.add(1, 1, -0.25); // src1 -> tgt1 * -0.25 (sums)
        let sources = [2.0, 4.0];
        let mut targets = [10.0, 100.0];
        m.apply(&sources, &mut targets);
        assert_eq!(targets[0], 10.0); // untouched
        assert!((targets[1] - (100.0 + 1.0 - 1.0)).abs() < 1e-6); // 100 + 0.5*2 - 0.25*4
    }

    #[test]
    fn out_of_range_is_ignored() {
        let mut m = ModMatrix::new();
        m.add(9, 9, 1.0);
        let mut targets = [0.0];
        m.apply(&[1.0], &mut targets); // must not panic
        assert_eq!(targets[0], 0.0);
    }

    #[test]
    fn set_and_clear() {
        let mut m = ModMatrix::new();
        m.set_routes(&[ModRoute {
            source: 0,
            target: 0,
            depth: 1.0,
        }]);
        assert_eq!(m.len(), 1);
        m.clear();
        assert!(m.is_empty());
    }
}
