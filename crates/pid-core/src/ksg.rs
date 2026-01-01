use crate::error::{PidError, PidResult};
use crate::matrix::{concat_horiz, MatRef};
use crate::metric::Metric;
use crate::nn::{count_neighbors_within, kth_neighbor_distance_joint_max};
use crate::stats::digamma;

#[derive(Debug, Clone, Copy)]
pub enum NegativeHandling {
    Allow,
    ClampToZero,
}

#[derive(Debug, Clone)]
pub struct KsgConfig {
    /// Number of nearest neighbors (excluding self).
    ///
    /// KSG requires `n > k >= 1`.
    pub k: usize,
    /// Distance metric. For KSG, the standard choice is Chebyshev / L∞.
    pub metric: Metric,
    /// Strict-inequality tie handling: marginal neighbor counts use `< (eps - tie_epsilon)`.
    pub tie_epsilon: f64,
    /// Handling of small negative MI estimates due to finite-sample noise.
    pub negative_handling: NegativeHandling,
}

impl Default for KsgConfig {
    fn default() -> Self {
        Self {
            k: 3,
            metric: Metric::Chebyshev,
            tie_epsilon: 1e-15,
            negative_handling: NegativeHandling::ClampToZero,
        }
    }
}

/// KSG mutual information estimator (Algorithm 1 style).
///
/// - Uses a kNN search in joint space (X,Y) with the configured metric (default: L∞).
/// - Uses strict inequality for marginal counts (`< eps`) to reduce tie bias.
/// - Returns MI in nats (natural log).
///
/// This is a brute-force O(n²) reference implementation intended for correctness first.
pub fn ksg_mi(x: MatRef<'_>, y: MatRef<'_>, cfg: &KsgConfig) -> PidResult<f64> {
    let local = ksg_local_mi_terms(x, y, cfg)?;
    let mi = local.iter().sum::<f64>() / (local.len() as f64);
    Ok(match cfg.negative_handling {
        NegativeHandling::Allow => mi,
        NegativeHandling::ClampToZero => mi.max(0.0),
    })
}

/// Returns the per-sample local MI contributions whose average equals the KSG MI estimate.
///
/// local_i = ψ(k) + ψ(n) - ψ(n_x(i)+1) - ψ(n_y(i)+1)
///
/// This is useful for building shared-exclusions estimators based on pointwise terms.
pub fn ksg_local_mi_terms(x: MatRef<'_>, y: MatRef<'_>, cfg: &KsgConfig) -> PidResult<Vec<f64>> {
    if x.nrows() != y.nrows() {
        return Err(PidError::RowCountMismatch {
            context: "ksg_local_mi_terms",
            left_rows: x.nrows(),
            right_rows: y.nrows(),
        });
    }
    let n = x.nrows();
    let k = cfg.k;
    if k == 0 || n <= k {
        return Err(PidError::InvalidK { k, n_samples: n });
    }

    let psi_k = digamma(k as f64);
    let psi_n = digamma(n as f64);

    let mut local = Vec::with_capacity(n);
    for i in 0..n {
        let blocks = [x, y];
        let eps = kth_neighbor_distance_joint_max(&blocks, i, cfg.k, cfg.metric)?;
        // Strict inequality for marginal counts, with optional numeric tie epsilon.
        let eps = (eps - cfg.tie_epsilon).max(0.0);

        let nx = count_neighbors_within(x, i, eps, cfg.metric);
        let ny = count_neighbors_within(y, i, eps, cfg.metric);

        let li = psi_k + psi_n - digamma((nx + 1) as f64) - digamma((ny + 1) as f64);
        local.push(li);
    }

    Ok(local)
}

pub fn ksg_mi_concat_xy(
    x: MatRef<'_>,
    y: MatRef<'_>,
    t: MatRef<'_>,
    cfg: &KsgConfig,
) -> PidResult<f64> {
    let xy = concat_horiz(x, y)?;
    ksg_mi(xy.as_ref(), t, cfg)
}
