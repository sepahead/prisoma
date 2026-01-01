use crate::error::{PidError, PidResult};
use crate::matrix::MatRef;
use crate::metric::Metric;
use crate::nn::{
    count_neighbors_within, count_neighbors_within_joint_max,
    kth_neighbor_distance_joint_max_with_scratch, strict_radius,
};
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
    if x.ncols() == 0 || y.ncols() == 0 {
        return Err(PidError::InvalidConfig {
            context: "ksg_local_mi_terms",
            message: "x and y must have at least 1 column",
        });
    }
    if !cfg.tie_epsilon.is_finite() || cfg.tie_epsilon < 0.0 {
        return Err(PidError::InvalidConfig {
            context: "ksg_local_mi_terms",
            message: "tie_epsilon must be finite and >= 0",
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
    let mut scratch = Vec::with_capacity(n.saturating_sub(1));
    let blocks = [x, y];
    for i in 0..n {
        let eps = kth_neighbor_distance_joint_max_with_scratch(
            &blocks,
            i,
            cfg.k,
            cfg.metric,
            &mut scratch,
        )?;
        // Strict inequality for marginal counts.
        let eps = strict_radius(eps, cfg.tie_epsilon);
        if eps == 0.0 {
            return Err(PidError::NumericalInstability {
                context: "ksg_local_mi_terms: kNN radius is non-positive; add jitter to break duplicates",
            });
        }

        let nx = count_neighbors_within(x, i, eps, cfg.metric);
        let ny = count_neighbors_within(y, i, eps, cfg.metric);

        let li = psi_k + psi_n - digamma((nx + 1) as f64) - digamma((ny + 1) as f64);
        local.push(li);
    }

    Ok(local)
}

/// KSG local MI terms when the "X" variable is treated as a concatenation of multiple blocks.
///
/// With `Metric::Chebyshev`, treating the concatenation as a max-over-blocks distance is
/// equivalent to explicitly concatenating the vectors, but avoids allocating an `(n×(d1+d2+...))`
/// temporary matrix.
pub(crate) fn ksg_local_mi_terms_xblocks<'a>(
    x_blocks: &[MatRef<'a>],
    y: MatRef<'a>,
    cfg: &KsgConfig,
) -> PidResult<Vec<f64>> {
    if x_blocks.is_empty() {
        return Err(PidError::NotImplemented {
            feature: "ksg_local_mi_terms_xblocks with empty x_blocks",
        });
    }
    if y.ncols() == 0 {
        return Err(PidError::InvalidConfig {
            context: "ksg_local_mi_terms_xblocks",
            message: "y must have at least 1 column",
        });
    }
    if !cfg.tie_epsilon.is_finite() || cfg.tie_epsilon < 0.0 {
        return Err(PidError::InvalidConfig {
            context: "ksg_local_mi_terms_xblocks",
            message: "tie_epsilon must be finite and >= 0",
        });
    }
    let n = y.nrows();
    for b in x_blocks {
        if b.nrows() != n {
            return Err(PidError::RowCountMismatch {
                context: "ksg_local_mi_terms_xblocks",
                left_rows: n,
                right_rows: b.nrows(),
            });
        }
        if b.ncols() == 0 {
            return Err(PidError::InvalidConfig {
                context: "ksg_local_mi_terms_xblocks",
                message: "x blocks must have at least 1 column",
            });
        }
    }

    let k = cfg.k;
    if k == 0 || n <= k {
        return Err(PidError::InvalidK { k, n_samples: n });
    }

    let psi_k = digamma(k as f64);
    let psi_n = digamma(n as f64);

    let mut joint_blocks: Vec<MatRef<'a>> = Vec::with_capacity(x_blocks.len() + 1);
    joint_blocks.extend_from_slice(x_blocks);
    joint_blocks.push(y);

    let mut scratch = Vec::with_capacity(n.saturating_sub(1));
    let mut local = Vec::with_capacity(n);
    for i in 0..n {
        let eps_joint = kth_neighbor_distance_joint_max_with_scratch(
            &joint_blocks,
            i,
            cfg.k,
            cfg.metric,
            &mut scratch,
        )?;
        let eps = strict_radius(eps_joint, cfg.tie_epsilon);
        if eps == 0.0 {
            return Err(PidError::NumericalInstability {
                context: "ksg_local_mi_terms_xblocks: kNN radius is non-positive; add jitter to break duplicates",
            });
        }

        let nx = count_neighbors_within_joint_max(x_blocks, i, eps, cfg.metric)?;
        let ny = count_neighbors_within(y, i, eps, cfg.metric);

        local.push(psi_k + psi_n - digamma((nx + 1) as f64) - digamma((ny + 1) as f64));
    }

    Ok(local)
}

pub(crate) fn ksg_mi_xblocks<'a>(
    x_blocks: &[MatRef<'a>],
    y: MatRef<'a>,
    cfg: &KsgConfig,
) -> PidResult<f64> {
    let local = ksg_local_mi_terms_xblocks(x_blocks, y, cfg)?;
    let mi = local.iter().sum::<f64>() / (local.len() as f64);
    Ok(match cfg.negative_handling {
        NegativeHandling::Allow => mi,
        NegativeHandling::ClampToZero => mi.max(0.0),
    })
}

pub fn ksg_mi_concat_xy(
    x: MatRef<'_>,
    y: MatRef<'_>,
    t: MatRef<'_>,
    cfg: &KsgConfig,
) -> PidResult<f64> {
    ksg_mi_xblocks(&[x, y], t, cfg)
}

#[cfg(test)]
mod tests {
    use super::{ksg_mi, ksg_mi_concat_xy, KsgConfig};
    use crate::matrix::{concat_horiz, MatRef};

    #[test]
    fn concat_xy_matches_explicit_concatenation_for_chebyshev() {
        // For Chebyshev/L∞, computing distance as max-over-blocks is equivalent to explicit
        // concatenation. This test guards the allocation-avoidance optimization.
        let n = 40;
        let d1 = 3;
        let d2 = 2;
        let dt = 1;

        let mut x = Vec::with_capacity(n * d1);
        let mut y = Vec::with_capacity(n * d2);
        let mut t = Vec::with_capacity(n * dt);
        for i in 0..n {
            for j in 0..d1 {
                x.push((i as f64) * 0.1 + (j as f64) * 0.01);
            }
            for j in 0..d2 {
                y.push((i as f64) * 0.2 - (j as f64) * 0.03);
            }
            t.push((i as f64) * 0.15);
        }

        let x = MatRef::new(&x, n, d1).unwrap();
        let y = MatRef::new(&y, n, d2).unwrap();
        let t = MatRef::new(&t, n, dt).unwrap();
        let cfg = KsgConfig::default();

        let mi_blocks = ksg_mi_concat_xy(x, y, t, &cfg).unwrap();
        let xy = concat_horiz(x, y).unwrap();
        let mi_explicit = ksg_mi(xy.as_ref(), t, &cfg).unwrap();

        assert!(
            (mi_blocks - mi_explicit).abs() < 1e-12,
            "mi_blocks={mi_blocks} mi_explicit={mi_explicit}"
        );
    }
}
