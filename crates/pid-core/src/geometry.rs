use crate::error::{PidError, PidResult};
use crate::matrix::MatRef;
use crate::metric::Metric;

#[derive(Debug, Clone)]
pub struct IntrinsicDimConfig {
    /// Number of nearest neighbors to use for the Levina–Bickel MLE-style estimator.
    ///
    /// Requirements: `k >= 2` and `n > k`.
    pub k: usize,
    pub metric: Metric,
}

impl Default for IntrinsicDimConfig {
    fn default() -> Self {
        Self {
            k: 10,
            metric: Metric::Chebyshev,
        }
    }
}

/// Estimate intrinsic dimension using a nearest-neighbor MLE-style estimator (Levina–Bickel).
///
/// This is a **diagnostic**, not a guarantee: it is useful for deciding whether kNN-based MI/PID
/// is even plausible at a given operating point.
///
/// For each sample `i`, let `T_j(i)` be the distance from `x_i` to its `j`-th nearest neighbor
/// (excluding itself) under `cfg.metric`, and let `k = cfg.k`. The pointwise estimate is:
///
/// `m_i = ( (1/(k-1)) * Σ_{j=1..k-1} ln( T_k(i) / T_j(i) ) )^{-1}`
///
/// and the returned estimate is the mean of `m_i` over all samples.
///
/// Notes:
/// - Duplicate points (zero distances) make the estimator ill-posed; add jitter or change
///   preprocessing.
/// - This implementation is brute-force O(n²) and intended for Experiment-0-scale diagnostics.
pub fn intrinsic_dimension_levina_bickel(
    x: MatRef<'_>,
    cfg: &IntrinsicDimConfig,
) -> PidResult<f64> {
    let n = x.nrows();
    let d = x.ncols();
    if n == 0 || d == 0 {
        return Err(PidError::InvalidConfig {
            context: "intrinsic_dimension_levina_bickel",
            message: "x must be non-empty (n,d >= 1)",
        });
    }

    let k = cfg.k;
    if k < 2 || n <= k {
        return Err(PidError::InvalidK { k, n_samples: n });
    }

    let kth = k - 1;
    let mut scratch = Vec::with_capacity(n.saturating_sub(1));
    let mut sum_m = 0.0f64;
    for i in 0..n {
        scratch.clear();
        let xi = x.row(i);
        for j in 0..n {
            if i == j {
                continue;
            }
            scratch.push(cfg.metric.distance(xi, x.row(j)));
        }

        scratch.select_nth_unstable_by(kth, |a, b| a.total_cmp(b));
        // The k smallest distances are in scratch[..k] (unordered).
        scratch[..k].sort_by(|a, b| a.total_cmp(b));
        let tk = scratch[kth];
        if tk <= 0.0 || !tk.is_finite() {
            return Err(PidError::NumericalInstability {
                context: "intrinsic_dimension_levina_bickel: kNN radius is non-positive; add jitter to break duplicates",
            });
        }

        let mut s = 0.0f64;
        for &tj in &scratch[..kth] {
            if tj <= 0.0 || !tj.is_finite() {
                return Err(PidError::NumericalInstability {
                    context: "intrinsic_dimension_levina_bickel: neighbor distance is non-positive; add jitter to break duplicates",
                });
            }
            // By construction tj <= tk, so ln(tk/tj) >= 0.
            s += (tk / tj).ln();
        }

        let denom = s / (kth as f64);
        if denom <= 0.0 || !denom.is_finite() {
            return Err(PidError::NumericalInstability {
                context: "intrinsic_dimension_levina_bickel: non-positive mean log distance ratio",
            });
        }

        sum_m += 1.0 / denom;
    }

    Ok(sum_m / (n as f64))
}
