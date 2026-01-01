use crate::error::{PidError, PidResult};
use crate::matrix::MatRef;
use crate::metric::Metric;

#[derive(Debug, Clone)]
pub struct DistanceConcentrationConfig {
    pub metric: Metric,
}

impl Default for DistanceConcentrationConfig {
    fn default() -> Self {
        Self {
            metric: Metric::Chebyshev,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DistanceConcentrationStats {
    /// Count of pairwise distances (n*(n-1)/2).
    pub pairwise_count: u64,
    pub pairwise_min: f64,
    pub pairwise_max: f64,
    pub pairwise_mean: f64,
    pub pairwise_std: f64,
    /// Coefficient of variation: std/mean (unitless).
    pub pairwise_cv: f64,

    /// Per-point nearest-neighbor distance summary.
    pub nn_min: f64,
    pub nn_max: f64,
    pub nn_mean: f64,
    pub nn_std: f64,
    pub nn_cv: f64,

    /// Ratio of mean nearest-neighbor distance to mean pairwise distance.
    ///
    /// In high dimension with distance concentration, this ratio tends to approach 1.
    pub nn_over_pairwise_mean: f64,
}

#[derive(Clone, Copy, Debug)]
struct RunningMoments {
    n: u64,
    mean: f64,
    m2: f64,
}

impl RunningMoments {
    fn new() -> Self {
        Self {
            n: 0,
            mean: 0.0,
            m2: 0.0,
        }
    }

    fn update(&mut self, x: f64) {
        debug_assert!(x.is_finite());
        self.n += 1;
        let delta = x - self.mean;
        self.mean += delta / (self.n as f64);
        let delta2 = x - self.mean;
        self.m2 += delta * delta2;
    }

    fn mean(&self) -> f64 {
        self.mean
    }

    fn std_population(&self) -> f64 {
        if self.n == 0 {
            return f64::NAN;
        }
        (self.m2 / (self.n as f64)).sqrt()
    }
}

/// Distance concentration diagnostics for kNN validity checks.
///
/// This function computes simple, robust proxies that indicate whether distances are
/// becoming “nearly equal” (a common failure mode for kNN methods in high dimension):
/// - coefficient of variation of all pairwise distances (`pairwise_cv = std/mean`)
/// - ratio of mean nearest-neighbor distance to mean pairwise distance (`nn_over_pairwise_mean`)
///
/// Notes:
/// - This is a **diagnostic**, not a guarantee.
/// - Non-finite inputs (NaN/Inf) are rejected.
/// - Duplicate points are allowed (min distance can be 0), but fully-degenerate data
///   (all distances 0) is rejected.
/// - This implementation is brute-force O(n²) and intended for Experiment-0-scale diagnostics.
pub fn distance_concentration_stats(
    x: MatRef<'_>,
    cfg: &DistanceConcentrationConfig,
) -> PidResult<DistanceConcentrationStats> {
    let n = x.nrows();
    let d = x.ncols();
    if n < 2 || d == 0 {
        return Err(PidError::InvalidConfig {
            context: "distance_concentration_stats",
            message: "x must have at least 2 rows and 1 column",
        });
    }

    let mut pair_stats = RunningMoments::new();
    let mut pair_min = f64::INFINITY;
    let mut pair_max = 0.0f64;

    let mut nn = vec![f64::INFINITY; n];

    for i in 0..n {
        let xi = x.row(i);
        for j in (i + 1)..n {
            let dist = cfg.metric.distance(xi, x.row(j));
            if !dist.is_finite() || dist < 0.0 {
                return Err(PidError::NumericalInstability {
                    context: "distance_concentration_stats: non-finite or negative distance",
                });
            }
            if dist < pair_min {
                pair_min = dist;
            }
            if dist > pair_max {
                pair_max = dist;
            }
            pair_stats.update(dist);

            if dist < nn[i] {
                nn[i] = dist;
            }
            if dist < nn[j] {
                nn[j] = dist;
            }
        }
    }

    let pairwise_mean = pair_stats.mean();
    if !pairwise_mean.is_finite() || pairwise_mean <= 0.0 {
        return Err(PidError::NumericalInstability {
            context: "distance_concentration_stats: non-positive mean distance (degenerate data)",
        });
    }
    let pairwise_std = pair_stats.std_population();
    let pairwise_cv = pairwise_std / pairwise_mean;

    let mut nn_stats = RunningMoments::new();
    let mut nn_min = f64::INFINITY;
    let mut nn_max = 0.0f64;
    for &dnn in &nn {
        if !dnn.is_finite() || dnn < 0.0 {
            return Err(PidError::NumericalInstability {
                context:
                    "distance_concentration_stats: non-finite or negative nearest-neighbor distance",
            });
        }
        if dnn < nn_min {
            nn_min = dnn;
        }
        if dnn > nn_max {
            nn_max = dnn;
        }
        nn_stats.update(dnn);
    }

    let nn_mean = nn_stats.mean();
    if !nn_mean.is_finite() || nn_mean <= 0.0 {
        return Err(PidError::NumericalInstability {
            context: "distance_concentration_stats: non-positive nearest-neighbor mean distance (degenerate data)",
        });
    }
    let nn_std = nn_stats.std_population();
    let nn_cv = nn_std / nn_mean;

    Ok(DistanceConcentrationStats {
        pairwise_count: pair_stats.n,
        pairwise_min: pair_min,
        pairwise_max: pair_max,
        pairwise_mean,
        pairwise_std,
        pairwise_cv,
        nn_min,
        nn_max,
        nn_mean,
        nn_std,
        nn_cv,
        nn_over_pairwise_mean: nn_mean / pairwise_mean,
    })
}

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
