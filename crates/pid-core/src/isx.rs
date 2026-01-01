use crate::error::{PidError, PidResult};
use crate::matrix::MatRef;
use crate::metric::Metric;
use crate::stats::digamma;

#[derive(Debug, Clone)]
pub struct IsxConfig {
    pub k: usize,
    pub metric: Metric,
    pub tie_epsilon: f64,
}

impl Default for IsxConfig {
    fn default() -> Self {
        Self {
            k: 3,
            metric: Metric::Chebyshev,
            tie_epsilon: 1e-15,
        }
    }
}

/// Continuous shared-exclusions redundancy I^sx_∩(S1,S2;T).
///
/// This is the core Wibral-group PID quantity (Makkeh et al. 2021; Ehrlich et al. 2024).
///
/// NOTE: The current implementation follows the *project spec sketch* in `grandplan.md`
/// (Appendix B.3.4.2). It must be treated as **unvalidated** until Experiment 0 has been
/// run and acceptance criteria met.
pub fn isx_redundancy(
    s1: MatRef<'_>,
    s2: MatRef<'_>,
    t: MatRef<'_>,
    cfg: &IsxConfig,
) -> PidResult<f64> {
    if s1.nrows() != s2.nrows() || s1.nrows() != t.nrows() {
        return Err(PidError::RowCountMismatch {
            context: "isx_redundancy",
            left_rows: s1.nrows(),
            right_rows: t.nrows(),
        });
    }
    let n = s1.nrows();
    let k = cfg.k;
    if k == 0 || n <= k {
        return Err(PidError::InvalidK { k, n_samples: n });
    }

    // 1) Per-sample kNN radii in (S1,T), (S2,T), (S1,S2,T) joint spaces.
    let mut eps_s1_t = vec![0.0f64; n];
    let mut eps_s2_t = vec![0.0f64; n];
    let mut eps_s1_s2_t = vec![0.0f64; n];

    for i in 0..n {
        eps_s1_t[i] = kth_neighbor_distance_s_t(s1, t, i, k, cfg.metric);
        eps_s2_t[i] = kth_neighbor_distance_s_t(s2, t, i, k, cfg.metric);
        eps_s1_s2_t[i] = kth_neighbor_distance_s1_s2_t(s1, s2, t, i, k, cfg.metric);
    }

    // 2) Count neighbors in target space within the respective radii.
    let mut n_t_s1 = vec![0usize; n];
    let mut n_t_s2 = vec![0usize; n];
    let mut n_t_shared = vec![0usize; n];
    let mut _n_t_joint = vec![0usize; n];

    for i in 0..n {
        let e1 = (eps_s1_t[i] - cfg.tie_epsilon).max(0.0);
        let e2 = (eps_s2_t[i] - cfg.tie_epsilon).max(0.0);
        let es = (eps_s1_t[i].min(eps_s2_t[i]) - cfg.tie_epsilon).max(0.0);
        let ej = (eps_s1_s2_t[i] - cfg.tie_epsilon).max(0.0);

        n_t_s1[i] = count_neighbors_within(t, i, e1, cfg.metric);
        n_t_s2[i] = count_neighbors_within(t, i, e2, cfg.metric);
        n_t_shared[i] = count_neighbors_within(t, i, es, cfg.metric);
        _n_t_joint[i] = count_neighbors_within(t, i, ej, cfg.metric);
    }

    // 3) Spec-sketch estimator (grandplan Appendix B.3.4.2).
    let psi_k = digamma(k as f64);
    let psi_n = digamma(n as f64);

    let avg_term = (0..n)
        .map(|i| {
            let psi_shared = digamma((n_t_shared[i] + 1) as f64);
            let psi_s1 = digamma((n_t_s1[i] + 1) as f64);
            let psi_s2 = digamma((n_t_s2[i] + 1) as f64);
            psi_shared - 0.5 * (psi_s1 + psi_s2)
        })
        .sum::<f64>()
        / (n as f64);

    let redundancy = psi_k + psi_n + avg_term;
    Ok(redundancy.max(0.0))
}

fn kth_neighbor_distance_s_t(
    s: MatRef<'_>,
    t: MatRef<'_>,
    i: usize,
    k: usize,
    metric: Metric,
) -> f64 {
    let n = s.nrows();
    let si = s.row(i);
    let ti = t.row(i);

    let mut dists = Vec::with_capacity(n - 1);
    for j in 0..n {
        if i == j {
            continue;
        }
        let ds = metric.distance(si, s.row(j));
        let dt = metric.distance(ti, t.row(j));
        dists.push(OrderedF64(ds.max(dt)));
    }
    let kth = k - 1;
    dists.select_nth_unstable(kth);
    dists[kth].0
}

fn kth_neighbor_distance_s1_s2_t(
    s1: MatRef<'_>,
    s2: MatRef<'_>,
    t: MatRef<'_>,
    i: usize,
    k: usize,
    metric: Metric,
) -> f64 {
    let n = s1.nrows();
    let s1i = s1.row(i);
    let s2i = s2.row(i);
    let ti = t.row(i);

    let mut dists = Vec::with_capacity(n - 1);
    for j in 0..n {
        if i == j {
            continue;
        }
        let d1 = metric.distance(s1i, s1.row(j));
        let d2 = metric.distance(s2i, s2.row(j));
        let dt = metric.distance(ti, t.row(j));
        dists.push(OrderedF64(d1.max(d2).max(dt)));
    }
    let kth = k - 1;
    dists.select_nth_unstable(kth);
    dists[kth].0
}

fn count_neighbors_within(m: MatRef<'_>, i: usize, eps: f64, metric: Metric) -> usize {
    let n = m.nrows();
    let mi = m.row(i);
    let mut count = 0usize;
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
