//! Discrete PID via quantization: an escape hatch for high-dimensional continuous data
//! where kNN-based MI estimation fails due to distance concentration.
//!
//! # Strategy
//!
//! 1. Quantize each continuous variable into `num_bins` equal-width bins per dimension.
//! 2. Compute discrete entropies by counting bin occupancies.
//! 3. Derive MI, co-information, and shared-exclusions redundancy from discrete entropies.
//! 4. Produce PID atoms (Red, Unq1, Unq2, Syn) using the same algebraic decomposition
//!    as the continuous path, but with counting-based estimation.
//!
//! This bypasses the kNN geometry problems entirely: discrete PID counts mass in
//! joint/marginal bins rather than measuring exclusion-ball volumes.
//!
//! # When to use
//!
//! - When the Experiment 0 geometry gate flags distance concentration or high intrinsic
//!   dimension in the continuous data.
//! - When `v̄ < 0` (monotonicity violation) blocks continuous PID interpretation.
//! - As a robustness check: compare discrete and continuous PID on the same data.
//!
//! # Limitations
//!
//! - Quantization destroys fine-grained information; results depend on `num_bins`.
//! - High-dimensional quantization is combinatorial (curse of dimensionality in bin counts).
//! - This module is designed for **low effective dimension** targets (after PLS/PCA reduction)
//!   or for scalar/low-d action spaces.

use crate::error::{PidError, PidResult};
use crate::matrix::MatRef;

/// Result of a discrete 2-source PID decomposition.
#[derive(Debug, Clone)]
pub struct DiscretePid2Result {
    pub redundancy: f64,
    pub unique_s1: f64,
    pub unique_s2: f64,
    pub synergy: f64,
    pub mi_s1_t: f64,
    pub mi_s2_t: f64,
    pub mi_s1s2_t: f64,
    pub num_bins: usize,
}

/// Quantize a continuous matrix into equal-width bins per dimension.
///
/// Each column is independently binned into `num_bins` equal-width bins spanning
/// the column's [min, max] range. Values exactly at `max` are placed in the last bin.
///
/// Returns a matrix of bin indices (nrows × ncols), stored row-major.
pub fn quantize_equal_width(x: MatRef<'_>, num_bins: usize) -> PidResult<Vec<Vec<usize>>> {
    if num_bins < 2 {
        return Err(PidError::InvalidConfig {
            context: "quantize_equal_width",
            message: "num_bins must be >= 2",
        });
    }
    let n = x.nrows();
    let d = x.ncols();

    // Compute column min/max.
    let mut col_min = vec![f64::INFINITY; d];
    let mut col_max = vec![f64::NEG_INFINITY; d];
    for i in 0..n {
        let row = x.row(i);
        for j in 0..d {
            if row[j] < col_min[j] {
                col_min[j] = row[j];
            }
            if row[j] > col_max[j] {
                col_max[j] = row[j];
            }
        }
    }

    let mut out = vec![vec![0usize; d]; n];
    for (i, out_row) in out.iter_mut().enumerate() {
        let row = x.row(i);
        for j in 0..d {
            let range = col_max[j] - col_min[j];
            let bin = if range < 1e-15 {
                0 // Constant column → all in bin 0.
            } else {
                let frac = (row[j] - col_min[j]) / range;
                let b = (frac * num_bins as f64) as usize;
                b.min(num_bins - 1) // Clamp max value into last bin.
            };
            out_row[j] = bin;
        }
    }
    Ok(out)
}

/// Encode a multi-dimensional bin assignment into a single integer key.
///
/// Each row of `bins` is a vector of per-dimension bin indices.
/// The key is `b[0] * num_bins^(d-1) + b[1] * num_bins^(d-2) + ... + b[d-1]`.
fn encode_bins(bins: &[usize], num_bins: usize) -> usize {
    let mut key = 0;
    for &b in bins {
        key = key * num_bins + b;
    }
    key
}

/// Compute discrete Shannon entropy H(X) from bin assignments.
///
/// `bins` is n×d_x; entropy is computed over the joint distribution of all columns.
/// Units: nats (natural logarithm).
pub fn discrete_entropy(bins: &[Vec<usize>], num_bins: usize) -> f64 {
    let n = bins.len();
    if n == 0 {
        return 0.0;
    }
    let mut counts = std::collections::HashMap::new();
    for row in bins {
        let key = encode_bins(row, num_bins);
        *counts.entry(key).or_insert(0usize) += 1;
    }
    let inv_n = 1.0 / n as f64;
    let mut h = 0.0;
    for &c in counts.values() {
        let p = c as f64 * inv_n;
        if p > 0.0 {
            h -= p * p.ln();
        }
    }
    h
}

/// Compute discrete mutual information I(X;Y) from quantized data.
///
/// `x_bins` is n×d_x, `y_bins` is n×d_y.
/// I(X;Y) = H(X) + H(Y) - H(X,Y).
pub fn discrete_mi(
    x_bins: &[Vec<usize>],
    y_bins: &[Vec<usize>],
    num_bins: usize,
) -> PidResult<f64> {
    if x_bins.len() != y_bins.len() {
        return Err(PidError::RowCountMismatch {
            context: "discrete_mi",
            left_rows: x_bins.len(),
            right_rows: y_bins.len(),
        });
    }
    let n = x_bins.len();

    let h_x = discrete_entropy(x_bins, num_bins);
    let h_y = discrete_entropy(y_bins, num_bins);

    // Joint entropy H(X,Y).
    let mut joint = Vec::with_capacity(n);
    for i in 0..n {
        let mut row = x_bins[i].clone();
        row.extend_from_slice(&y_bins[i]);
        joint.push(row);
    }
    let h_xy = discrete_entropy(&joint, num_bins);

    Ok(h_x + h_y - h_xy)
}

/// Compute discrete 2-source PID atoms via quantization + shared-exclusions redundancy.
///
/// Sources S1, S2 and target T are each quantized into `num_bins` equal-width bins.
/// Redundancy uses the discrete shared-exclusions formula:
///
/// `Red(S1,S2;T) = Σ_t p(t) min(i_spec(S1;t), i_spec(S2;t))`
///
/// where `i_spec(S;t) = log(p(t|s)/p(t))` is the pointwise specific information.
pub fn discrete_pid2(
    s1: MatRef<'_>,
    s2: MatRef<'_>,
    target: MatRef<'_>,
    num_bins: usize,
) -> PidResult<DiscretePid2Result> {
    if num_bins < 2 {
        return Err(PidError::InvalidConfig {
            context: "discrete_pid2",
            message: "num_bins must be >= 2",
        });
    }
    let n = s1.nrows();
    if s2.nrows() != n || target.nrows() != n {
        return Err(PidError::RowCountMismatch {
            context: "discrete_pid2",
            left_rows: n,
            right_rows: s2.nrows().min(target.nrows()),
        });
    }

    // 1. Quantize all three variables.
    let s1_bins = quantize_equal_width(s1, num_bins)?;
    let s2_bins = quantize_equal_width(s2, num_bins)?;
    let t_bins = quantize_equal_width(target, num_bins)?;

    // 2. Compute MI terms.
    let mi_s1_t = discrete_mi(&s1_bins, &t_bins, num_bins)?;
    let mi_s2_t = discrete_mi(&s2_bins, &t_bins, num_bins)?;

    // For joint MI: concatenate S1 and S2 bins.
    let mut s1s2_bins = Vec::with_capacity(n);
    for i in 0..n {
        let mut row = s1_bins[i].clone();
        row.extend_from_slice(&s2_bins[i]);
        s1s2_bins.push(row);
    }
    let mi_s1s2_t = discrete_mi(&s1s2_bins, &t_bins, num_bins)?;

    // 3. Compute shared-exclusions redundancy via pointwise specific information.
    let redundancy = discrete_isx_redundancy(&s1_bins, &s2_bins, &t_bins, num_bins);

    // 4. Derive PID atoms.
    let unique_s1 = mi_s1_t - redundancy;
    let unique_s2 = mi_s2_t - redundancy;
    let synergy = mi_s1s2_t - mi_s1_t - mi_s2_t + redundancy;

    Ok(DiscretePid2Result {
        redundancy,
        unique_s1,
        unique_s2,
        synergy,
        mi_s1_t,
        mi_s2_t,
        mi_s1s2_t,
        num_bins,
    })
}

/// Discrete shared-exclusions redundancy.
///
/// `Red(S1,S2;T) = Σ_t p(t) min(i_spec(S1;t), i_spec(S2;t))`
///
/// where `i_spec(S;t) = Σ_s p(s|t) log(p(t|s)/p(t))`.
fn discrete_isx_redundancy(
    s1_bins: &[Vec<usize>],
    s2_bins: &[Vec<usize>],
    t_bins: &[Vec<usize>],
    num_bins: usize,
) -> f64 {
    let n = s1_bins.len();
    if n == 0 {
        return 0.0;
    }
    let inv_n = 1.0 / n as f64;

    // Build marginal distributions and conditional distributions.
    // For each source S, compute p(s) and p(s|t) and p(t|s).
    let t_counts = count_dist(t_bins, num_bins);
    let s1_counts = count_dist(s1_bins, num_bins);
    let s2_counts = count_dist(s2_bins, num_bins);

    // Joint counts: (s, t) for each source.
    let s1t_counts = count_joint_dist(s1_bins, t_bins, num_bins);
    let s2t_counts = count_joint_dist(s2_bins, t_bins, num_bins);

    // Compute specific information for each (source, t) pair:
    // i_spec(S;t) = Σ_s p(s|t) log(p(t|s) / p(t))
    //             = Σ_s [p(s,t)/p(t)] log[p(s,t) * n / (p(s) * p(t) * n)]
    //             = Σ_s [count(s,t)/count(t)] log[count(s,t) * n / (count(s) * count(t))]
    let i_spec_s1 = specific_information(&s1t_counts, &s1_counts, &t_counts, n);
    let i_spec_s2 = specific_information(&s2t_counts, &s2_counts, &t_counts, n);

    // Red = Σ_t p(t) min(i_spec(S1;t), i_spec(S2;t))
    let mut red = 0.0;
    for (t_key, &ct) in &t_counts {
        let p_t = ct as f64 * inv_n;
        let is1 = i_spec_s1.get(t_key).copied().unwrap_or(0.0);
        let is2 = i_spec_s2.get(t_key).copied().unwrap_or(0.0);
        red += p_t * is1.min(is2);
    }

    red
}

/// Count the frequency of each bin combination.
fn count_dist(bins: &[Vec<usize>], num_bins: usize) -> std::collections::HashMap<usize, usize> {
    let mut counts = std::collections::HashMap::new();
    for row in bins {
        let key = encode_bins(row, num_bins);
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

/// Count the joint frequency of (x_bins, y_bins) pairs.
fn count_joint_dist(
    x_bins: &[Vec<usize>],
    y_bins: &[Vec<usize>],
    num_bins: usize,
) -> std::collections::HashMap<(usize, usize), usize> {
    let mut counts = std::collections::HashMap::new();
    for (xr, yr) in x_bins.iter().zip(y_bins) {
        let xk = encode_bins(xr, num_bins);
        let yk = encode_bins(yr, num_bins);
        *counts.entry((xk, yk)).or_insert(0) += 1;
    }
    counts
}

/// Compute specific information `i(S; t)` for each target bin `t`.
///
/// `i(S; t) = Σ_s p(s|t) log(p(s,t) * n / (p(s) * p(t) * n))`
///           = Σ_s [count(s,t)/count(t)] * log[count(s,t) * n / (count(s) * count(t))]
fn specific_information(
    st_counts: &std::collections::HashMap<(usize, usize), usize>,
    s_counts: &std::collections::HashMap<usize, usize>,
    t_counts: &std::collections::HashMap<usize, usize>,
    n: usize,
) -> std::collections::HashMap<usize, f64> {
    let mut result = std::collections::HashMap::new();

    // Group joint counts by t.
    let mut by_t: std::collections::HashMap<usize, Vec<(usize, usize)>> =
        std::collections::HashMap::new();
    for (&(sk, tk), &cst) in st_counts {
        by_t.entry(tk).or_default().push((sk, cst));
    }

    for (&tk, entries) in &by_t {
        let ct = t_counts.get(&tk).copied().unwrap_or(0);
        if ct == 0 {
            continue;
        }
        let mut is = 0.0;
        for &(sk, cst) in entries {
            let cs = s_counts.get(&sk).copied().unwrap_or(0);
            if cs == 0 || cst == 0 {
                continue;
            }
            // p(s|t) = cst / ct
            // log(p(s,t) / (p(s) * p(t))) = log(cst * n / (cs * ct))
            let log_ratio = ((cst as f64) * (n as f64) / ((cs as f64) * (ct as f64))).ln();
            is += (cst as f64 / ct as f64) * log_ratio;
        }
        result.insert(tk, is);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::MatRef;

    #[test]
    fn discrete_entropy_uniform() {
        // 4 equally likely bins → H = ln(4) ≈ 1.386
        let bins: Vec<Vec<usize>> = (0..400).map(|i| vec![i % 4]).collect();
        let h = discrete_entropy(&bins, 4);
        assert!(
            (h - 4.0f64.ln()).abs() < 0.05,
            "H(uniform 4 bins) should be ≈ ln(4); got {h}"
        );
    }

    #[test]
    fn discrete_mi_independent() {
        // Independent X and Y → I(X;Y) ≈ 0
        let n = 1000;
        let mut rng = crate::preprocess::SplitMix64::new(42);
        let mut x_bins = Vec::with_capacity(n);
        let mut y_bins = Vec::with_capacity(n);
        for _ in 0..n {
            x_bins.push(vec![(rng.next_u64() as usize) % 4]);
            y_bins.push(vec![(rng.next_u64() as usize) % 4]);
        }
        let mi = discrete_mi(&x_bins, &y_bins, 4).unwrap();
        assert!(
            mi.abs() < 0.05,
            "MI of independent vars should be ≈ 0; got {mi}"
        );
    }

    #[test]
    fn discrete_mi_copy() {
        // Y = X → I(X;Y) = H(X)
        let n = 500;
        let bins: Vec<Vec<usize>> = (0..n).map(|i| vec![i % 8]).collect();
        let mi = discrete_mi(&bins, &bins, 8).unwrap();
        let h = discrete_entropy(&bins, 8);
        assert!(
            (mi - h).abs() < 0.01,
            "MI(X;X) should equal H(X); MI={mi}, H={h}"
        );
    }

    #[test]
    fn discrete_pid2_redundant_copy() {
        // S1 = S2 = signal → Red ≈ MI, Unq ≈ 0, Syn ≈ 0
        let n = 500;
        let d = 1;
        let mut rng = crate::preprocess::SplitMix64::new(99);
        let mut s1_data = Vec::with_capacity(n * d);
        let mut s2_data = Vec::with_capacity(n * d);
        let mut t_data = Vec::with_capacity(n * d);
        for _ in 0..n {
            let sig = rng.normal();
            s1_data.push(sig);
            s2_data.push(sig + 0.01 * rng.normal()); // Near-copy
            t_data.push(sig + 0.1 * rng.normal());
        }
        let s1 = MatRef::new(&s1_data, n, d).unwrap();
        let s2 = MatRef::new(&s2_data, n, d).unwrap();
        let t = MatRef::new(&t_data, n, d).unwrap();

        let result = discrete_pid2(s1, s2, t, 10).unwrap();

        // Redundancy should dominate; unique should be small.
        assert!(
            result.redundancy > 0.5 * result.mi_s1_t,
            "Redundancy should be > 50% of MI for near-copies; Red={}, MI={}",
            result.redundancy,
            result.mi_s1_t
        );
        assert!(
            result.unique_s1.abs() < 0.3 * result.mi_s1_t,
            "Unique S1 should be small for near-copies; Unq1={}",
            result.unique_s1
        );
    }

    #[test]
    fn quantize_rejects_bad_bins() {
        let data = vec![0.0f64; 10];
        let m = MatRef::new(&data, 5, 2).unwrap();
        assert!(quantize_equal_width(m, 0).is_err());
        assert!(quantize_equal_width(m, 1).is_err());
    }
}
