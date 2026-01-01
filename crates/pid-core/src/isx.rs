use crate::error::{PidError, PidResult};
use crate::ksg::{ksg_local_mi_terms, KsgConfig, NegativeHandling};
use crate::matrix::{concat_horiz, MatRef};
use crate::metric::Metric;
use crate::nn::{count_neighbors_within, kth_neighbor_distance_joint_max};
use crate::stats::digamma;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsxMethod {
    /// Heuristic/sketch from `grandplan.md` Appendix B.3.4.2.
    ///
    /// This exists to mirror the project spec text, but should be treated as
    /// untrusted until Experiment 0 validates it.
    GrandplanSketch,
    /// Approximate shared-exclusions redundancy by taking the samplewise minimum
    /// of KSG local MI terms for (S1,T) and (S2,T), then averaging.
    LocalMinKsg,
    /// Approximate shared-exclusions redundancy using the disjunction form:
    ///
    /// i^sx(s1,s2;t) = log( exp(i(s1;t)) + exp(i(s2;t)) - exp(i(s1,s2;t)) )
    ///
    /// with all pointwise terms estimated via KSG local MI contributions.
    DisjunctionFromLocalMi,
}

#[derive(Debug, Clone)]
pub struct IsxConfig {
    pub k: usize,
    pub metric: Metric,
    pub tie_epsilon: f64,
    pub method: IsxMethod,
}

impl Default for IsxConfig {
    fn default() -> Self {
        Self {
            k: 3,
            metric: Metric::Chebyshev,
            tie_epsilon: 1e-15,
            method: IsxMethod::LocalMinKsg,
        }
    }
}

/// Continuous shared-exclusions redundancy I^sx_∩(S1,S2;T).
///
/// This is the core Wibral-group PID quantity (Makkeh et al. 2021; Ehrlich et al. 2024).
///
/// By default (`IsxMethod::LocalMinKsg`), this matches `grandplan.md` §2.2.1 / §2.3.2:
/// compute pointwise MI terms via KSG (`ksg_local_mi_terms`), take the per-sample minimum
/// across sources, and average.
///
/// Other `IsxMethod` variants are included as explicit experimental baselines / cross-checks
/// against various sketches in `grandplan.md` and must not be trusted without validation.
pub fn isx_redundancy(
    s1: MatRef<'_>,
    s2: MatRef<'_>,
    t: MatRef<'_>,
    cfg: &IsxConfig,
) -> PidResult<f64> {
    match cfg.method {
        IsxMethod::GrandplanSketch => isx_redundancy_grandplan_sketch(s1, s2, t, cfg),
        IsxMethod::LocalMinKsg => isx_redundancy_local_min_ksg(s1, s2, t, cfg),
        IsxMethod::DisjunctionFromLocalMi => {
            isx_redundancy_disjunction_from_local_mi(s1, s2, t, cfg)
        }
    }
}

fn isx_redundancy_disjunction_from_local_mi(
    s1: MatRef<'_>,
    s2: MatRef<'_>,
    t: MatRef<'_>,
    cfg: &IsxConfig,
) -> PidResult<f64> {
    if s1.nrows() != s2.nrows() || s1.nrows() != t.nrows() {
        return Err(PidError::RowCountMismatch {
            context: "isx_redundancy_disjunction_from_local_mi",
            left_rows: s1.nrows(),
            right_rows: t.nrows(),
        });
    }
    let n = s1.nrows();
    let k = cfg.k;
    if k == 0 || n <= k {
        return Err(PidError::InvalidK { k, n_samples: n });
    }

    let ksg_cfg = KsgConfig {
        k: cfg.k,
        metric: cfg.metric,
        tie_epsilon: cfg.tie_epsilon,
        negative_handling: NegativeHandling::Allow,
    };

    let i1 = ksg_local_mi_terms(s1, t, &ksg_cfg)?;
    let i2 = ksg_local_mi_terms(s2, t, &ksg_cfg)?;
    let s1s2 = concat_horiz(s1, s2)?;
    let i12 = ksg_local_mi_terms(s1s2.as_ref(), t, &ksg_cfg)?;

    let mut sum = 0.0f64;
    for ((&a, &b), &c) in i1.iter().zip(i2.iter()).zip(i12.iter()) {
        // Compute: log(exp(a)+exp(b)-exp(c)) stably.
        let m = a.max(b).max(c);
        let sa = (a - m).exp();
        let sb = (b - m).exp();
        let sc = (c - m).exp();
        let s = sa + sb - sc;
        if !s.is_finite() || s <= 0.0 {
            return Err(PidError::NumericalInstability {
                context: "isx_redundancy_disjunction_from_local_mi",
            });
        }
        sum += m + s.ln();
    }

    Ok(sum / (n as f64))
}

fn isx_redundancy_local_min_ksg(
    s1: MatRef<'_>,
    s2: MatRef<'_>,
    t: MatRef<'_>,
    cfg: &IsxConfig,
) -> PidResult<f64> {
    if s1.nrows() != s2.nrows() || s1.nrows() != t.nrows() {
        return Err(PidError::RowCountMismatch {
            context: "isx_redundancy_local_min_ksg",
            left_rows: s1.nrows(),
            right_rows: t.nrows(),
        });
    }
    let n = s1.nrows();
    let k = cfg.k;
    if k == 0 || n <= k {
        return Err(PidError::InvalidK { k, n_samples: n });
    }

    let ksg_cfg = KsgConfig {
        k: cfg.k,
        metric: cfg.metric,
        tie_epsilon: cfg.tie_epsilon,
        negative_handling: NegativeHandling::Allow,
    };

    let local_s1 = ksg_local_mi_terms(s1, t, &ksg_cfg)?;
    let local_s2 = ksg_local_mi_terms(s2, t, &ksg_cfg)?;

    let red = local_s1
        .iter()
        .zip(local_s2.iter())
        .map(|(&a, &b)| a.min(b))
        .sum::<f64>()
        / (n as f64);

    Ok(red)
}

fn isx_redundancy_grandplan_sketch(
    s1: MatRef<'_>,
    s2: MatRef<'_>,
    t: MatRef<'_>,
    cfg: &IsxConfig,
) -> PidResult<f64> {
    if s1.nrows() != s2.nrows() || s1.nrows() != t.nrows() {
        return Err(PidError::RowCountMismatch {
            context: "isx_redundancy_grandplan_sketch",
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
        eps_s1_t[i] = kth_neighbor_distance_joint_max(&[s1, t], i, k, cfg.metric)?;
        eps_s2_t[i] = kth_neighbor_distance_joint_max(&[s2, t], i, k, cfg.metric)?;
        eps_s1_s2_t[i] = kth_neighbor_distance_joint_max(&[s1, s2, t], i, k, cfg.metric)?;
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
