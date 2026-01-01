#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metric {
    /// Chebyshev / L∞ distance: max_i |a_i - b_i|
    Chebyshev,
}

impl Metric {
    #[inline]
    pub fn distance(&self, a: &[f64], b: &[f64]) -> f64 {
        match self {
            Metric::Chebyshev => chebyshev(a, b),
        }
    }
}

#[inline]
pub fn chebyshev(a: &[f64], b: &[f64]) -> f64 {
    debug_assert_eq!(a.len(), b.len());
    let mut max_abs = 0.0;
    for (&ai, &bi) in a.iter().zip(b.iter()) {
        let d = (ai - bi).abs();
        if d > max_abs {
            max_abs = d;
        }
    }
    max_abs
}
