use crate::error::{PidError, PidResult};
use crate::ksg::{ksg_local_mi_terms, ksg_local_mi_terms_xblocks, KsgConfig, NegativeHandling};
use crate::matrix::MatRef;
use crate::metric::Metric;
use crate::nn::{
    count_neighbors_within, kth_neighbor_distance_joint_max_with_scratch, strict_radius,
};
use crate::stats::digamma;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsxMethod {
    /// Paper-faithful kNN estimator for continuous shared-exclusions redundancy from:
    /// Ehrlich et al. (2024), arXiv:2311.06373v3.
    ///
    /// Implements the bivariate redundancy `I^sx_∩(S1,S2;T)` via the KSG-style estimator
    /// (Appendix H, Algorithms 3–6) under the L∞/Chebyshev metric:
    ///
    /// I^sx_∩ = ψ(k) + ψ(n) - ⟨ ψ(n_α(i)) + ψ(n_T(i)) ⟩_i
    ///
    /// where:
    /// - ε_i is the kNN radius in the joint (source-disjunction, target) space,
    /// - n_α(i) counts neighbors in the *source disjunction* within ε_i,
    /// - n_T(i) counts neighbors in target space within ε_i.
    ///
    /// Note: This is the default method for `IsxConfig`.
    EhrlichKsg,
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
            method: IsxMethod::EhrlichKsg,
        }
    }
}

/// Continuous shared-exclusions redundancy I^sx_∩(S1,S2;T).
///
/// This is the core Wibral-group PID quantity (Makkeh et al. 2021; Ehrlich et al. 2024).
///
/// By default (`IsxMethod::EhrlichKsg`), this uses the paper-faithful KSG-style kNN estimator
/// for continuous variables (Ehrlich et al. 2024, Appendix H).
///
/// Other `IsxMethod` variants are included only as explicit experimental baselines / cross-checks
/// against various sketches in `grandplan.md` and should not be trusted without validation.
pub fn isx_redundancy(
    s1: MatRef<'_>,
    s2: MatRef<'_>,
    t: MatRef<'_>,
    cfg: &IsxConfig,
) -> PidResult<f64> {
    if s1.ncols() == 0 || s2.ncols() == 0 || t.ncols() == 0 {
        return Err(PidError::InvalidConfig {
            context: "isx_redundancy",
            message: "inputs must have at least 1 column",
        });
    }
    if !cfg.tie_epsilon.is_finite() || cfg.tie_epsilon < 0.0 {
        return Err(PidError::InvalidConfig {
            context: "isx_redundancy",
            message: "tie_epsilon must be finite and >= 0",
        });
    }
    match cfg.method {
        IsxMethod::EhrlichKsg => isx_redundancy_ehrlich_ksg(s1, s2, t, cfg),
        IsxMethod::GrandplanSketch => isx_redundancy_grandplan_sketch(s1, s2, t, cfg),
        IsxMethod::LocalMinKsg => isx_redundancy_local_min_ksg(s1, s2, t, cfg),
        IsxMethod::DisjunctionFromLocalMi => {
            isx_redundancy_disjunction_from_local_mi(s1, s2, t, cfg)
        }
    }
}

fn isx_redundancy_ehrlich_ksg(
    s1: MatRef<'_>,
    s2: MatRef<'_>,
    t: MatRef<'_>,
    cfg: &IsxConfig,
) -> PidResult<f64> {
    if s1.nrows() != s2.nrows() || s1.nrows() != t.nrows() {
        return Err(PidError::RowCountMismatch {
            context: "isx_redundancy_ehrlich_ksg",
            left_rows: s1.nrows(),
            right_rows: t.nrows(),
        });
    }
    let n = s1.nrows();
    let k = cfg.k;
    if k == 0 || n <= k {
        return Err(PidError::InvalidK { k, n_samples: n });
    }

    // This is the bivariate antichain α = {{1},{2}}; the disjunction distance in source space is:
    // d_S_disj(i,j) = min( d(S1_i,S1_j), d(S2_i,S2_j) ).
    //
    // With Chebyshev/L∞ and a shared target ball, the joint disjunction distance is:
    // d_ST_disj(i,j) = max( d(T_i,T_j), d_S_disj(i,j) ).
    let psi_k = digamma(k as f64);
    let psi_n = digamma(n as f64);

    let mut scratch = Vec::with_capacity(n.saturating_sub(1));
    let mut sum = 0.0f64;
    for i in 0..n {
        scratch.clear();
        scratch.reserve(n.saturating_sub(1));

        for j in 0..n {
            if i == j {
                continue;
            }
            let ds1 = cfg.metric.distance(s1.row(i), s1.row(j));
            let ds2 = cfg.metric.distance(s2.row(i), s2.row(j));
            let dt = cfg.metric.distance(t.row(i), t.row(j));
            let d_joint = dt.max(ds1.min(ds2));
            scratch.push(d_joint);
        }

        let kth = k - 1;
        scratch.select_nth_unstable_by(kth, |a, b| a.total_cmp(b));
        let eps_raw = scratch[kth];
        let eps = strict_radius(eps_raw, cfg.tie_epsilon);
        if eps == 0.0 {
            return Err(PidError::NumericalInstability {
                context: "isx_redundancy_ehrlich_ksg: kNN radius is non-positive; add jitter to break duplicates",
            });
        }

        // Counts exclude self; the estimator needs counts including self.
        let n_t = count_neighbors_within(t, i, eps, cfg.metric) + 1;
        let n_alpha = count_neighbors_within_source_disjunction(s1, s2, i, eps, cfg.metric) + 1;

        sum += psi_k + psi_n - digamma(n_alpha as f64) - digamma(n_t as f64);
    }

    Ok(sum / (n as f64))
}

#[inline]
fn count_neighbors_within_source_disjunction(
    s1: MatRef<'_>,
    s2: MatRef<'_>,
    i: usize,
    eps: f64,
    metric: Metric,
) -> usize {
    let n = s1.nrows();
    let mut count = 0usize;
    for j in 0..n {
        if i == j {
            continue;
        }
        let d1 = metric.distance(s1.row(i), s1.row(j));
        let d2 = metric.distance(s2.row(i), s2.row(j));
        if d1.min(d2) < eps {
            count += 1;
        }
    }
    count
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
    let i12 = ksg_local_mi_terms_xblocks(&[s1, s2], t, &ksg_cfg)?;

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
                context: "isx_redundancy_disjunction_from_local_mi: disjunction argument is non-positive",
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

    let mut scratch = Vec::with_capacity(n.saturating_sub(1));
    for i in 0..n {
        eps_s1_t[i] =
            kth_neighbor_distance_joint_max_with_scratch(&[s1, t], i, k, cfg.metric, &mut scratch)?;
        eps_s2_t[i] =
            kth_neighbor_distance_joint_max_with_scratch(&[s2, t], i, k, cfg.metric, &mut scratch)?;
        eps_s1_s2_t[i] = kth_neighbor_distance_joint_max_with_scratch(
            &[s1, s2, t],
            i,
            k,
            cfg.metric,
            &mut scratch,
        )?;
    }

    // 2) Count neighbors in target space within the respective radii.
    let mut n_t_s1 = vec![0usize; n];
    let mut n_t_s2 = vec![0usize; n];
    let mut n_t_shared = vec![0usize; n];

    for i in 0..n {
        let e1 = strict_radius(eps_s1_t[i], cfg.tie_epsilon);
        let e2 = strict_radius(eps_s2_t[i], cfg.tie_epsilon);
        let es = strict_radius(eps_s1_t[i].min(eps_s2_t[i]), cfg.tie_epsilon);

        n_t_s1[i] = count_neighbors_within(t, i, e1, cfg.metric);
        n_t_s2[i] = count_neighbors_within(t, i, e2, cfg.metric);
        n_t_shared[i] = count_neighbors_within(t, i, es, cfg.metric);
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
