use crate::error::{PidError, PidResult};
use crate::matrix::{concat_horiz, MatRef};
use crate::metric::Metric;
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
            negative_handling: NegativeHandling::Allow,
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
    if x.nrows() != y.nrows() {
        return Err(PidError::RowCountMismatch {
            context: "ksg_mi",
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

    let mut sum = 0.0;
    for i in 0..n {
        let eps = kth_neighbor_distance_joint(x, y, i, cfg)?;
        // Strict inequality for marginal counts, with optional numeric tie epsilon.
        let eps = (eps - cfg.tie_epsilon).max(0.0);

        let nx = count_neighbors_within(x, i, eps, cfg.metric);
        let ny = count_neighbors_within(y, i, eps, cfg.metric);

        sum += digamma((nx + 1) as f64) + digamma((ny + 1) as f64);
    }

    let mi = psi_k + psi_n - (sum / n as f64);
    Ok(match cfg.negative_handling {
        NegativeHandling::Allow => mi,
        NegativeHandling::ClampToZero => mi.max(0.0),
    })
}

fn kth_neighbor_distance_joint(
    x: MatRef<'_>,
    y: MatRef<'_>,
    i: usize,
    cfg: &KsgConfig,
) -> PidResult<f64> {
    let n = x.nrows();
    let k = cfg.k;
    let metric = cfg.metric;

    let xi = x.row(i);
    let yi = y.row(i);

    let mut dists = Vec::with_capacity(n - 1);
    for j in 0..n {
        if i == j {
            continue;
        }
        let dx = metric.distance(xi, x.row(j));
        let dy = metric.distance(yi, y.row(j));
        dists.push(OrderedF64(dx.max(dy)));
    }

    // dists.len() == n - 1, and n > k, so k-th neighbor exists.
    let kth = k - 1; // 0-based index
    dists.select_nth_unstable(kth);
    Ok(dists[kth].0)
}

fn count_neighbors_within(m: MatRef<'_>, i: usize, eps: f64, metric: Metric) -> usize {
    let n = m.nrows();
    let mi = m.row(i);
    let mut count = 0;
    for j in 0..n {
        if i == j {
            continue;
        }
        if metric.distance(mi, m.row(j)) < eps {
            count += 1;
        }
    }
    count
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

#[derive(Debug, Clone, Copy)]
struct OrderedF64(f64);

impl PartialEq for OrderedF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl PartialOrd for OrderedF64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for OrderedF64 {}

impl Ord for OrderedF64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}
