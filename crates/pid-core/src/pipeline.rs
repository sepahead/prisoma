//! Pipeline functions that compose PLS projection, PID decomposition, and bootstrap
//! uncertainty quantification.
//!
//! These are convenience entry points for the common VLA analysis workflow:
//!
//! 1. `pls_project_then_pid3` — fit PLS on high-dimensional embeddings, project into a
//!    low-dimensional task-relevant subspace, then run full 3-source SxPID.
//! 2. `bootstrap_pid3` — block-bootstrap resample rows of (V,L,D,A) jointly, recompute PID
//!    on each resample, and return percentile CIs on every PID atom.

use crate::bootstrap::BootstrapConfig;
use crate::concat_horiz;
use crate::discrete_pid::discrete_pid3;
use crate::error::{PidError, PidResult};
use crate::matrix::{MatOwned, MatRef};
use crate::pid2::{pid2_isx, Pid2Config, Pid2Result};
use crate::pid3::{pid3_isx, Antichain3, Pid3Config, Pid3Result};
use crate::pls::PlsProjector;
use crate::preprocess::SplitMix64;

// ── PLS → PID3 ─────────────────────────────────────────────────────────────

/// Configuration for [`pls_project_then_pid3`].
#[derive(Debug, Clone)]
pub struct PlsPid3Config {
    /// Number of PLS latent components to extract (applied to each source and target).
    pub pls_components: usize,
    /// PID3 estimator configuration (k, metric, tie_epsilon).
    pub pid_cfg: Pid3Config,
}

/// Output of [`pls_project_then_pid3`].
#[derive(Debug, Clone)]
pub struct PlsPid3Result {
    /// PID decomposition on the PLS-projected embeddings.
    pub pid: Pid3Result,
    /// Number of PLS components used.
    pub pls_components: usize,
    /// Input column counts for V, L, D, A before projection.
    pub input_dims: [usize; 4],
    /// Output column count after projection (= pls_components).
    pub projected_dim: usize,
}

/// Fit per-source PLS projectors (each source → A) to reduce dimensionality, then
/// run 3-source SxPID on the projected embeddings.
///
/// Each of V, L, D is projected through its own PLS model fitted with A as target.
/// A is projected through a PLS fitted with the concatenated VLD as target.
/// All four projections yield `pls_components`-dimensional representations.
///
/// The three sources (V, L, D) must share the same row count `n`, and A must also have `n` rows.
///
/// # Leakage warning
///
/// This function fits PLS on **all** provided data. For proper train/test separation,
/// call [`PlsProjector::fit`] on training data only, then [`PlsProjector::transform`]
/// on each split, and finally [`pid3_isx`] on the projected matrices.
pub fn pls_project_then_pid3(
    v: MatRef<'_>,
    l: MatRef<'_>,
    d: MatRef<'_>,
    a: MatRef<'_>,
    cfg: &PlsPid3Config,
) -> PidResult<PlsPid3Result> {
    let n = v.nrows();
    if l.nrows() != n || d.nrows() != n || a.nrows() != n {
        return Err(PidError::RowCountMismatch {
            context: "pls_project_then_pid3",
            left_rows: n,
            right_rows: l.nrows().min(d.nrows()).min(a.nrows()),
        });
    }

    // Fit a per-source PLS projector: each source S_i → A.
    // This gives each source its own low-d task-relevant representation.
    let v_proj = PlsProjector::fit(v, a, cfg.pls_components)?.transform(v)?;
    let l_proj = PlsProjector::fit(l, a, cfg.pls_components)?.transform(l)?;
    let d_proj = PlsProjector::fit(d, a, cfg.pls_components)?.transform(d)?;
    // For A, fit a PLS using the concatenated VLD as target so that the
    // projected target captures task-relevant variance from the sources.
    let vld = concat_horiz(concat_horiz(v, l)?.as_ref(), d)?;
    let a_proj = PlsProjector::fit(a, vld.as_ref(), cfg.pls_components)?.transform(a)?;

    let pid = pid3_isx(
        v_proj.as_ref(),
        l_proj.as_ref(),
        d_proj.as_ref(),
        a_proj.as_ref(),
        &cfg.pid_cfg,
    )?;

    Ok(PlsPid3Result {
        pid,
        pls_components: cfg.pls_components,
        input_dims: [v.ncols(), l.ncols(), d.ncols(), a.ncols()],
        projected_dim: cfg.pls_components,
    })
}

// ── Bootstrap PID3 ─────────────────────────────────────────────────────────

/// Per-atom bootstrap confidence interval for a 3-source PID decomposition.
#[derive(Debug, Clone)]
pub struct Pid3BootstrapAtom {
    /// The antichain identifying this atom on the PID lattice.
    pub antichain: Antichain3,
    /// Point estimate on the original (un-resampled) data.
    pub point_estimate: f64,
    /// Mean of the bootstrap distribution.
    pub boot_mean: f64,
    /// Standard error (std of bootstrap distribution).
    pub boot_se: f64,
    /// Lower percentile CI bound.
    pub ci_low: f64,
    /// Upper percentile CI bound.
    pub ci_high: f64,
}

/// Result of [`bootstrap_pid3`].
#[derive(Debug, Clone)]
pub struct BootstrapPid3Result {
    /// Point estimate PID result on the original data.
    pub point_estimate: Pid3Result,
    /// Bootstrap CIs for each atom (same canonical order as `point_estimate.atoms`).
    pub atoms: Vec<Pid3BootstrapAtom>,
    /// Number of bootstrap resamples used.
    pub n_boot: usize,
    /// Block size used.
    pub block_size: usize,
}

/// Block-bootstrap confidence intervals on every atom of a 3-source PID decomposition.
///
/// Rows of (V, L, D, A) are resampled jointly (same block indices across all four matrices),
/// preserving any cross-variable dependence. `pid3_isx` is recomputed on each resample, and
/// percentile CIs are extracted for each of the 18 atoms.
///
/// # Panics
///
/// Panics if `n < block_size`, `block_size == 0`, or `n_boot == 0`.
pub fn bootstrap_pid3(
    v: MatRef<'_>,
    l: MatRef<'_>,
    d: MatRef<'_>,
    a: MatRef<'_>,
    pid_cfg: &Pid3Config,
    boot_cfg: &BootstrapConfig,
) -> PidResult<BootstrapPid3Result> {
    let n = v.nrows();
    assert!(
        l.nrows() == n && d.nrows() == n && a.nrows() == n,
        "all inputs must have the same row count"
    );
    assert!(boot_cfg.block_size > 0, "block_size must be > 0");
    assert!(boot_cfg.block_size <= n, "block_size must be <= n");
    assert!(boot_cfg.n_boot > 0, "n_boot must be > 0");

    let dv = v.ncols();
    let dl = l.ncols();
    let dd = d.ncols();
    let da = a.ncols();
    let n_blocks = n / boot_cfg.block_size;
    assert!(n_blocks > 0, "n / block_size must be > 0");

    // Point estimate on original data.
    let point_estimate = pid3_isx(v, l, d, a, pid_cfg)?;
    let n_atoms = point_estimate.atoms.len();

    let mut rng = SplitMix64::new(boot_cfg.seed);
    // boot_values[atom_idx][boot_idx]
    let mut boot_values: Vec<Vec<f64>> = vec![Vec::with_capacity(boot_cfg.n_boot); n_atoms];

    for _ in 0..boot_cfg.n_boot {
        // Build resample index set by sampling n_blocks blocks with replacement.
        let mut indices = Vec::with_capacity(n_blocks * boot_cfg.block_size);
        for _ in 0..n_blocks {
            let block_start = (rng.next_u64() as usize % n_blocks) * boot_cfg.block_size;
            for j in 0..boot_cfg.block_size {
                indices.push(block_start + j);
            }
        }

        let resample = |mat: MatRef<'_>, dim: usize| -> MatOwned {
            let mut data = Vec::with_capacity(indices.len() * dim);
            for &i in &indices {
                data.extend_from_slice(mat.row(i));
            }
            MatOwned::new(data, indices.len(), dim).expect("resample data should be finite")
        };

        let vr = resample(v, dv);
        let lr = resample(l, dl);
        let dr = resample(d, dd);
        let ar = resample(a, da);

        match pid3_isx(vr.as_ref(), lr.as_ref(), dr.as_ref(), ar.as_ref(), pid_cfg) {
            Ok(result) => {
                for (idx, atom) in result.atoms.iter().enumerate() {
                    boot_values[idx].push(atom.value);
                }
            }
            Err(_) => {
                // PID failed on this resample (e.g. degenerate geometry); push NaN.
                for bv in &mut boot_values {
                    bv.push(f64::NAN);
                }
            }
        }
    }

    // Build per-atom bootstrap summaries.
    let alpha = boot_cfg.alpha;
    let atoms: Vec<Pid3BootstrapAtom> = point_estimate
        .atoms
        .iter()
        .enumerate()
        .map(|(idx, atom)| {
            let vals = &boot_values[idx];
            // Filter out NaN entries from failed resamples.
            let mut finite: Vec<f64> = vals.iter().copied().filter(|x| x.is_finite()).collect();
            if finite.is_empty() {
                return Pid3BootstrapAtom {
                    antichain: atom.antichain,
                    point_estimate: atom.value,
                    boot_mean: f64::NAN,
                    boot_se: f64::NAN,
                    ci_low: f64::NAN,
                    ci_high: f64::NAN,
                };
            }
            finite.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let m = finite.len();
            let mean = finite.iter().sum::<f64>() / m as f64;
            let var = finite.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / m as f64;
            let se = var.sqrt();
            let lo_idx = ((alpha / 2.0) * m as f64).floor() as usize;
            let hi_idx = (((1.0 - alpha / 2.0) * m as f64).ceil() as usize)
                .saturating_sub(1)
                .min(m - 1);
            Pid3BootstrapAtom {
                antichain: atom.antichain,
                point_estimate: atom.value,
                boot_mean: mean,
                boot_se: se,
                ci_low: finite[lo_idx],
                ci_high: finite[hi_idx],
            }
        })
        .collect();

    Ok(BootstrapPid3Result {
        point_estimate,
        atoms,
        n_boot: boot_cfg.n_boot,
        block_size: boot_cfg.block_size,
    })
}

// ── Permutation test ───────────────────────────────────────────────────────────

/// Result of a permutation test on PID atoms.
#[derive(Debug, Clone)]
pub struct PermutationPid3Atom {
    pub antichain: Antichain3,
    pub observed: f64,
    pub p_value: f64,
    pub n_perm: usize,
}

/// Result of [`permutation_pid3`].
#[derive(Debug, Clone)]
pub struct PermutationPid3Result {
    pub atoms: Vec<PermutationPid3Atom>,
    pub n_perm: usize,
    pub source_shuffled: usize,
}

/// Permutation test for PID atoms: shuffles rows of a single source to build a null
/// distribution, then computes one-sided p-values for each atom.
///
/// `source_idx` selects which source to shuffle (0=V, 1=L, 2=D). Under H0 (source carries
/// no information about target), the shuffled PID atoms should be ~0.
#[allow(clippy::too_many_arguments)]
pub fn permutation_pid3(
    v: MatRef<'_>,
    l: MatRef<'_>,
    d: MatRef<'_>,
    a: MatRef<'_>,
    pid_cfg: &Pid3Config,
    n_perm: usize,
    source_idx: usize,
    seed: u64,
) -> PidResult<PermutationPid3Result> {
    if source_idx > 2 {
        return Err(PidError::InvalidConfig {
            context: "permutation_pid3",
            message: "source_idx must be 0, 1, or 2",
        });
    }
    let n = v.nrows();
    assert!(n_perm > 0, "n_perm must be > 0");

    // Observed PID on real data.
    let observed = pid3_isx(v, l, d, a, pid_cfg)?;

    let mut rng = SplitMix64::new(seed);
    let n_atoms = observed.atoms.len();
    // perm_values[atom_idx][perm_idx]
    let mut perm_values: Vec<Vec<f64>> = vec![Vec::with_capacity(n_perm); n_atoms];

    let dv = v.ncols();
    let dl = l.ncols();
    let dd = d.ncols();

    for _ in 0..n_perm {
        // Build a permutation of row indices.
        let mut perm: Vec<usize> = (0..n).collect();
        // Fisher-Yates shuffle.
        for i in (1..n).rev() {
            let j = (rng.next_u64() as usize) % (i + 1);
            perm.swap(i, j);
        }

        let shuffle = |mat: MatRef<'_>, dim: usize| -> MatOwned {
            let mut data = Vec::with_capacity(n * dim);
            for &i in &perm {
                data.extend_from_slice(mat.row(i));
            }
            MatOwned::new(data, n, dim).expect("shuffle data should be finite")
        };

        let copy_mat = |mat: MatRef<'_>, dim: usize| -> MatOwned {
            let mut data = Vec::with_capacity(n * dim);
            for i in 0..n {
                data.extend_from_slice(mat.row(i));
            }
            MatOwned::new(data, n, dim).expect("copy data should be finite")
        };

        // Only shuffle the selected source; keep others and target intact.
        let vp = if source_idx == 0 {
            shuffle(v, dv)
        } else {
            copy_mat(v, dv)
        };
        let lp = if source_idx == 1 {
            shuffle(l, dl)
        } else {
            copy_mat(l, dl)
        };
        let dp = if source_idx == 2 {
            shuffle(d, dd)
        } else {
            copy_mat(d, dd)
        };

        match pid3_isx(vp.as_ref(), lp.as_ref(), dp.as_ref(), a, pid_cfg) {
            Ok(result) => {
                for (idx, atom) in result.atoms.iter().enumerate() {
                    perm_values[idx].push(atom.value);
                }
            }
            Err(_) => {
                for pv in &mut perm_values {
                    pv.push(f64::NAN);
                }
            }
        }
    }

    let atoms: Vec<PermutationPid3Atom> = observed
        .atoms
        .iter()
        .enumerate()
        .map(|(idx, atom)| {
            let vals = &perm_values[idx];
            let finite: Vec<f64> = vals.iter().copied().filter(|x| x.is_finite()).collect();
            let n_valid = finite.len();
            // One-sided p-value: fraction of permuted values >= observed.
            let p_value = if n_valid == 0 {
                f64::NAN
            } else {
                finite.iter().filter(|&&x| x >= atom.value).count() as f64 / n_valid as f64
            };
            PermutationPid3Atom {
                antichain: atom.antichain,
                observed: atom.value,
                p_value,
                n_perm: n_valid,
            }
        })
        .collect();

    Ok(PermutationPid3Result {
        atoms,
        n_perm,
        source_shuffled: source_idx,
    })
}

// ── PLS cross-validation ───────────────────────────────────────────────────────

/// Result of PLS cross-validation for component selection.
#[derive(Debug, Clone)]
pub struct PlsCvResult {
    /// Predictive power Q² for each candidate component count.
    pub q2: Vec<f64>,
    /// Optimal number of components (maximizing Q²).
    pub best_components: usize,
    /// Total number of candidate components tested.
    pub max_components: usize,
}

/// Leave-one-out cross-validation to select the optimal number of PLS components.
///
/// For each candidate `k` in 1..=max_components, this computes Q² = 1 - PRESS/SS_total,
/// where PRESS is the sum of squared prediction errors from LOO-CV and SS_total is the
/// total sum of squares of the target.
///
/// `x` is the source matrix (n×d_x) and `y` is the target (n×d_y).
pub fn pls_cv_select_components(
    x: MatRef<'_>,
    y: MatRef<'_>,
    max_components: usize,
) -> PidResult<PlsCvResult> {
    let n = x.nrows();
    let d_x = x.ncols();
    let d_y = y.ncols();
    if y.nrows() != n {
        return Err(PidError::RowCountMismatch {
            context: "pls_cv_select_components",
            left_rows: n,
            right_rows: y.nrows(),
        });
    }
    let max_out = d_x.min(n.saturating_sub(1));
    let max_components = max_components.min(max_out);
    if max_components == 0 {
        return Err(PidError::InvalidConfig {
            context: "pls_cv_select_components",
            message: "max_components must be >= 1 after clipping",
        });
    }

    // Compute SS_total.
    let mut y_mean = vec![0.0f64; d_y];
    for i in 0..n {
        let row = y.row(i);
        for (j, ym) in y_mean.iter_mut().enumerate() {
            *ym += row[j];
        }
    }
    for m in &mut y_mean {
        *m /= n as f64;
    }
    let ss_total: f64 = {
        let ym = &y_mean;
        (0..n)
            .flat_map(|i| (0..d_y).map(move |j| (y.row(i)[j] - ym[j]).powi(2)))
            .sum()
    };

    let mut q2 = Vec::with_capacity(max_components);
    for k in 1..=max_components {
        let mut press = 0.0f64;
        // LOO-CV: for each held-out sample, fit PLS on the rest and predict.
        for held_out in 0..n {
            // Build train set (n-1 samples).
            let train_n = n - 1;
            let mut x_train_data = Vec::with_capacity(train_n * d_x);
            let mut y_train_data = Vec::with_capacity(train_n * d_y);
            for i in 0..n {
                if i == held_out {
                    continue;
                }
                x_train_data.extend_from_slice(x.row(i));
                y_train_data.extend_from_slice(y.row(i));
            }
            let x_train =
                MatOwned::new(x_train_data, train_n, d_x).expect("train data should be finite");
            let y_train =
                MatOwned::new(y_train_data, train_n, d_y).expect("train data should be finite");

            match PlsProjector::fit(x_train.as_ref(), y_train.as_ref(), k) {
                Ok(pls) => {
                    // Predict for held-out sample.
                    let x_ho =
                        MatRef::new(x.row(held_out), 1, d_x).expect("held-out row should be valid");
                    match pls.transform(x_ho) {
                        Ok(t_ho) => {
                            // Reconstruct y from PLS scores: y_hat = T C^T + y_mean.
                            let t_row = t_ho.as_ref().row(0);
                            let ho_row = y.row(held_out);
                            for (j, &ym_j) in y_mean.iter().enumerate() {
                                let mut y_hat_j = ym_j;
                                for (comp, &t_c) in t_row.iter().enumerate().take(k) {
                                    let c_j = pls.y_weights()[comp * d_y + j];
                                    y_hat_j += t_c * c_j;
                                }
                                press += (ho_row[j] - y_hat_j).powi(2);
                            }
                        }
                        Err(_) => {
                            press += f64::NAN;
                        }
                    }
                }
                Err(_) => {
                    press += f64::NAN;
                }
            }
        }
        let q2_k = if ss_total > 0.0 && press.is_finite() {
            1.0 - press / ss_total
        } else {
            f64::NEG_INFINITY
        };
        q2.push(q2_k);
    }

    // Select best k (max Q²).
    let best_idx = q2
        .iter()
        .enumerate()
        .max_by(|(_, &a), (_, &b)| a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0);
    let best_components = best_idx + 1;

    Ok(PlsCvResult {
        q2,
        best_components,
        max_components,
    })
}

// ── PLS → Discrete PID3 ──────────────────────────────────────────────────────

/// Configuration for [`pls_project_then_discrete_pid3`].
#[derive(Debug, Clone)]
pub struct PlsDiscretePid3Config {
    /// Number of PLS latent components to extract.
    pub pls_components: usize,
    /// Number of equal-width bins for discrete PID.
    pub num_bins: usize,
}

/// Result of [`pls_project_then_discrete_pid3`].
#[derive(Debug, Clone)]
pub struct PlsDiscretePid3Result {
    pub pid: crate::discrete_pid::DiscretePid3Result,
    pub pls_components: usize,
    pub num_bins: usize,
    pub input_dims: [usize; 4],
    pub projected_dim: usize,
}

/// Fit per-source PLS projectors, project all four matrices into a low-dimensional
/// task-relevant subspace, then run discrete PID3 on the quantized projections.
///
/// This is the recommended escape hatch when continuous kNN-based PID fails due to
/// high ambient dimension or distance concentration.
pub fn pls_project_then_discrete_pid3(
    v: MatRef<'_>,
    l: MatRef<'_>,
    d: MatRef<'_>,
    a: MatRef<'_>,
    cfg: &PlsDiscretePid3Config,
) -> PidResult<PlsDiscretePid3Result> {
    let n = v.nrows();
    if l.nrows() != n || d.nrows() != n || a.nrows() != n {
        return Err(PidError::RowCountMismatch {
            context: "pls_project_then_discrete_pid3",
            left_rows: n,
            right_rows: l.nrows().min(d.nrows()).min(a.nrows()),
        });
    }

    // Per-source PLS projectors.
    let v_proj = PlsProjector::fit(v, a, cfg.pls_components)?.transform(v)?;
    let l_proj = PlsProjector::fit(l, a, cfg.pls_components)?.transform(l)?;
    let d_proj = PlsProjector::fit(d, a, cfg.pls_components)?.transform(d)?;
    let vld = concat_horiz(concat_horiz(v, l)?.as_ref(), d)?;
    let a_proj = PlsProjector::fit(a, vld.as_ref(), cfg.pls_components)?.transform(a)?;

    let pid = discrete_pid3(
        v_proj.as_ref(),
        l_proj.as_ref(),
        d_proj.as_ref(),
        a_proj.as_ref(),
        cfg.num_bins,
    )?;

    Ok(PlsDiscretePid3Result {
        pid,
        pls_components: cfg.pls_components,
        num_bins: cfg.num_bins,
        input_dims: [v.ncols(), l.ncols(), d.ncols(), a.ncols()],
        projected_dim: cfg.pls_components,
    })
}

// ── Multi-pair PID2 screening ──────────────────────────────────────────────────

/// A single PID2 screening result for a pair of sources.
#[derive(Debug, Clone)]
pub struct Pid2ScreenEntry {
    /// Source pair indices (i, j) into the sources list.
    pub source_i: usize,
    pub source_j: usize,
    pub result: Pid2Result,
}

/// Screen all pairs of sources with PID2, returning one entry per pair.
///
/// `sources` is a slice of matrices, each n×d_i. `target` is the target matrix.
/// This computes PID2 for all C(n_sources, 2) pairs and sorts them by descending
/// synergy.
pub fn screen_pid2_pairs(
    sources: &[MatRef<'_>],
    target: MatRef<'_>,
    cfg: &Pid2Config,
) -> PidResult<Vec<Pid2ScreenEntry>> {
    let n = target.nrows();
    let n_src = sources.len();
    let mut entries = Vec::with_capacity(n_src * (n_src.saturating_sub(1)) / 2);

    for i in 0..n_src {
        if sources[i].nrows() != n {
            return Err(PidError::RowCountMismatch {
                context: "screen_pid2_pairs",
                left_rows: n,
                right_rows: sources[i].nrows(),
            });
        }
        for j in (i + 1)..n_src {
            if sources[j].nrows() != n {
                continue;
            }
            match pid2_isx(sources[i], sources[j], target, cfg) {
                Ok(result) => {
                    entries.push(Pid2ScreenEntry {
                        source_i: i,
                        source_j: j,
                        result,
                    });
                }
                Err(_) => {
                    // Skip pairs that fail (e.g. degenerate geometry).
                }
            }
        }
    }

    // Sort by descending synergy.
    entries.sort_by(|a, b| {
        b.result
            .synergy
            .partial_cmp(&a.result.synergy)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(entries)
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preprocess::SplitMix64;

    /// Helper: generate synthetic (V, L, D, A) data where V and L share signal about A,
    /// D is pure noise.
    fn make_vlda(n: usize, seed: u64) -> (MatOwned, MatOwned, MatOwned, MatOwned) {
        let mut rng = SplitMix64::new(seed);
        let mut v_data = Vec::with_capacity(n * 3);
        let mut l_data = Vec::with_capacity(n * 3);
        let mut d_data = Vec::with_capacity(n * 2);
        let mut a_data = Vec::with_capacity(n);
        for _ in 0..n {
            let signal = rng.normal();
            // V carries signal in dim 0, noise in dims 1,2
            v_data.push(signal + 0.1 * rng.normal());
            v_data.push(rng.normal());
            v_data.push(rng.normal());
            // L carries signal in dim 0, noise in dims 1,2
            l_data.push(signal + 0.1 * rng.normal());
            l_data.push(rng.normal());
            l_data.push(rng.normal());
            // D is pure noise
            d_data.push(rng.normal());
            d_data.push(rng.normal());
            // A = signal + small noise
            a_data.push(signal + 0.05 * rng.normal());
        }
        let v = MatOwned::new(v_data, n, 3).unwrap();
        let l = MatOwned::new(l_data, n, 3).unwrap();
        let d = MatOwned::new(d_data, n, 2).unwrap();
        let a = MatOwned::new(a_data, n, 1).unwrap();
        (v, l, d, a)
    }

    #[test]
    fn pls_project_then_pid3_runs_and_returns_18_atoms() {
        let (v, l, d, a) = make_vlda(60, 42);
        let cfg = PlsPid3Config {
            pls_components: 1,
            pid_cfg: Pid3Config::default(),
        };
        let result =
            pls_project_then_pid3(v.as_ref(), l.as_ref(), d.as_ref(), a.as_ref(), &cfg).unwrap();
        // The PID result has 18 atoms for 3 sources.
        assert_eq!(result.pid.atoms.len(), 18);
        assert_eq!(result.pls_components, 1);
        assert_eq!(result.projected_dim, 1);
        assert_eq!(result.input_dims, [3, 3, 2, 1]);
    }

    #[test]
    fn pls_project_then_pid3_rejects_mismatched_rows() {
        let v = MatOwned::new(vec![0.0; 30], 10, 3).unwrap();
        let l = MatOwned::new(vec![0.0; 15], 5, 3).unwrap(); // Wrong row count
        let d = MatOwned::new(vec![0.0; 20], 10, 2).unwrap();
        let a = MatOwned::new(vec![0.0; 10], 10, 1).unwrap();
        let cfg = PlsPid3Config {
            pls_components: 1,
            pid_cfg: Pid3Config::default(),
        };
        assert!(
            pls_project_then_pid3(v.as_ref(), l.as_ref(), d.as_ref(), a.as_ref(), &cfg,).is_err()
        );
    }

    #[test]
    fn bootstrap_pid3_returns_ci_for_each_atom() {
        let (v, l, d, a) = make_vlda(80, 77);
        let pid_cfg = Pid3Config::default();
        let boot_cfg = BootstrapConfig {
            n_boot: 20, // Small for test speed
            block_size: 10,
            seed: 42,
            alpha: 0.1,
        };
        let result = bootstrap_pid3(
            v.as_ref(),
            l.as_ref(),
            d.as_ref(),
            a.as_ref(),
            &pid_cfg,
            &boot_cfg,
        )
        .unwrap();

        // 18 atoms for 3-source PID.
        assert_eq!(result.atoms.len(), 18);
        assert_eq!(result.point_estimate.atoms.len(), 18);
        assert_eq!(result.n_boot, 20);
        assert_eq!(result.block_size, 10);

        // Each atom's CI should bracket the point estimate (for most atoms with finite CIs).
        let finite_atoms: Vec<_> = result
            .atoms
            .iter()
            .filter(|a| a.ci_low.is_finite() && a.ci_high.is_finite())
            .collect();
        assert!(
            !finite_atoms.is_empty(),
            "at least some bootstrap atoms should have finite CIs"
        );
        for atom in &finite_atoms {
            assert!(
                atom.ci_low <= atom.ci_high,
                "CI low ({}) must be <= CI high ({})",
                atom.ci_low,
                atom.ci_high
            );
        }
    }

    #[test]
    fn bootstrap_pid3_is_deterministic() {
        let (v, l, d, a) = make_vlda(60, 123);
        let pid_cfg = Pid3Config::default();
        let boot_cfg = BootstrapConfig {
            n_boot: 10,
            block_size: 10,
            seed: 99,
            alpha: 0.05,
        };
        let r1 = bootstrap_pid3(
            v.as_ref(),
            l.as_ref(),
            d.as_ref(),
            a.as_ref(),
            &pid_cfg,
            &boot_cfg,
        )
        .unwrap();
        let r2 = bootstrap_pid3(
            v.as_ref(),
            l.as_ref(),
            d.as_ref(),
            a.as_ref(),
            &pid_cfg,
            &boot_cfg,
        )
        .unwrap();

        // Same seed → same bootstrap results.
        for (a1, a2) in r1.atoms.iter().zip(r2.atoms.iter()) {
            assert_eq!(a1.point_estimate, a2.point_estimate);
            if a1.boot_mean.is_finite() {
                assert!(
                    (a1.boot_mean - a2.boot_mean).abs() < 1e-12,
                    "bootstrap must be deterministic"
                );
            }
        }
    }

    #[test]
    fn bootstrap_pid3_point_estimate_matches_direct() {
        let (v, l, d, a) = make_vlda(60, 55);
        let pid_cfg = Pid3Config::default();
        let boot_cfg = BootstrapConfig {
            n_boot: 5,
            block_size: 10,
            seed: 0,
            alpha: 0.05,
        };
        let result = bootstrap_pid3(
            v.as_ref(),
            l.as_ref(),
            d.as_ref(),
            a.as_ref(),
            &pid_cfg,
            &boot_cfg,
        )
        .unwrap();

        // Point estimate should match a direct pid3_isx call.
        let direct = pid3_isx(v.as_ref(), l.as_ref(), d.as_ref(), a.as_ref(), &pid_cfg).unwrap();
        for (boot_atom, direct_atom) in result.point_estimate.atoms.iter().zip(direct.atoms.iter())
        {
            assert_eq!(boot_atom.antichain, direct_atom.antichain);
            assert!(
                (boot_atom.value - direct_atom.value).abs() < 1e-12,
                "point estimate must match direct pid3_isx"
            );
        }
    }

    #[test]
    fn permutation_pid3_produces_p_values() {
        let (v, l, d, a) = make_vlda(60, 42);
        let pid_cfg = Pid3Config::default();
        let result = permutation_pid3(
            v.as_ref(),
            l.as_ref(),
            d.as_ref(),
            a.as_ref(),
            &pid_cfg,
            10, // Small for test speed.
            2,  // Shuffle D (noise source → p-values should be high).
            42,
        )
        .unwrap();
        assert_eq!(result.atoms.len(), 18);
        assert_eq!(result.n_perm, 10);
        assert_eq!(result.source_shuffled, 2);
        // D is pure noise, so permuting it should yield high p-values.
        let finite_atoms: Vec<_> = result
            .atoms
            .iter()
            .filter(|a| a.p_value.is_finite())
            .collect();
        assert!(!finite_atoms.is_empty());
    }

    #[test]
    fn permutation_pid3_rejects_bad_source_idx() {
        let (v, l, d, a) = make_vlda(60, 42);
        let pid_cfg = Pid3Config::default();
        assert!(permutation_pid3(
            v.as_ref(),
            l.as_ref(),
            d.as_ref(),
            a.as_ref(),
            &pid_cfg,
            5,
            3, // Invalid source index.
            0
        )
        .is_err());
    }

    #[test]
    fn pls_cv_selects_at_least_one_component() {
        let n = 50;
        let mut rng = SplitMix64::new(77);
        let mut x_data = Vec::with_capacity(n * 5);
        let mut y_data = Vec::with_capacity(n);
        for _ in 0..n {
            let sig = rng.normal();
            x_data.push(sig + 0.1 * rng.normal());
            for _ in 1..5 {
                x_data.push(rng.normal());
            }
            y_data.push(sig);
        }
        let x = MatRef::new(&x_data, n, 5).unwrap();
        let y = MatRef::new(&y_data, n, 1).unwrap();
        let result = pls_cv_select_components(x, y, 3).unwrap();
        assert_eq!(result.q2.len(), 3);
        assert!(result.best_components >= 1);
        assert!(result.best_components <= 3);
    }

    #[test]
    fn pls_project_then_discrete_pid3_runs() {
        let (v, l, d, a) = make_vlda(60, 42);
        let cfg = PlsDiscretePid3Config {
            pls_components: 1,
            num_bins: 8,
        };
        let result =
            pls_project_then_discrete_pid3(v.as_ref(), l.as_ref(), d.as_ref(), a.as_ref(), &cfg)
                .unwrap();
        assert_eq!(result.pid.atoms.len(), 18);
        assert_eq!(result.pls_components, 1);
        assert_eq!(result.num_bins, 8);
        assert_eq!(result.projected_dim, 1);
        assert_eq!(result.input_dims, [3, 3, 2, 1]);
    }

    #[test]
    fn screen_pid2_pairs_returns_all_pairs() {
        let n = 60;
        let mut rng = SplitMix64::new(42);
        let mut s0_data = Vec::with_capacity(n * 2);
        let mut s1_data = Vec::with_capacity(n * 2);
        let mut s2_data = Vec::with_capacity(n);
        let mut t_data = Vec::with_capacity(n);
        for _ in 0..n {
            let sig = rng.normal();
            s0_data.push(sig + 0.1 * rng.normal());
            s0_data.push(rng.normal());
            s1_data.push(sig + 0.1 * rng.normal());
            s1_data.push(rng.normal());
            s2_data.push(rng.normal());
            t_data.push(sig + 0.05 * rng.normal());
        }
        let s0 = MatOwned::new(s0_data, n, 2).unwrap();
        let s1 = MatOwned::new(s1_data, n, 2).unwrap();
        let s2 = MatOwned::new(s2_data, n, 1).unwrap();
        let t = MatOwned::new(t_data, n, 1).unwrap();
        let sources: Vec<MatRef<'_>> = vec![s0.as_ref(), s1.as_ref(), s2.as_ref()];
        let cfg = Pid2Config::default();
        let entries = screen_pid2_pairs(&sources, t.as_ref(), &cfg).unwrap();
        // 3 sources → C(3,2) = 3 pairs.
        assert_eq!(entries.len(), 3);
        // Sorted by descending synergy.
        for w in entries.windows(2) {
            assert!(w[0].result.synergy >= w[1].result.synergy);
        }
    }
}
