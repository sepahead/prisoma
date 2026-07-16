//! Historical v10.7 endpoint-sensitivity calculations.
//!
//! This module preserves a retired Monte-Carlo calculation for reproducibility. Its four
//! `legacy_v10_7_*` endpoint IDs do not denote the current EC1/H1–H4 registry, its finite-grid
//! comparisons are nonpromotable, and no output can establish a current hypothesis gate,
//! scientific success, capture requirement, or study readiness.
//!
//! The retired endpoint labels were: incremental held-out episode-level ΔAUROC, two task-level
//! Spearman correlations, and mean per-case Kendall τ. Their legacy thresholds were 0.05, ±0.3,
//! and 1/3 respectively. The calculations exercise the statistical procedures encoded at the
//! time and report idealized sensitivity over finite grids. The H1 feature model is binormal, so
//! the injected incremental effect is exact by construction:
//! `d = √2·Φ⁻¹(AUROC)`, and an independent PID feature with
//! `d₂ = √(d²_target − d²_base)` yields the target combined AUROC under the
//! optimal (here: logistic) combiner.

use serde::{Deserialize, Serialize};

// ───────────────────────────── deterministic RNG ────────────────────────────

/// splitmix64-seeded xorshift64*; deterministic per (seed, cell, replicate) so
/// every grid cell is independently reproducible.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // splitmix64 scramble so nearby seeds decorrelate.
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        Self(if z == 0 { 0x2545_F491_4F6C_DD1D } else { z })
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in [0, 1).
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Standard normal via Marsaglia polar (no cached spare: simple and branchy
    /// but plenty fast at these sizes).
    fn next_gaussian(&mut self) -> f64 {
        loop {
            let u = 2.0 * self.next_f64() - 1.0;
            let v = 2.0 * self.next_f64() - 1.0;
            let s = u * u + v * v;
            if s > 0.0 && s < 1.0 {
                return u * (-2.0 * s.ln() / s).sqrt();
            }
        }
    }

    /// Fisher–Yates in-place shuffle.
    fn shuffle<T>(&mut self, xs: &mut [T]) {
        for i in (1..xs.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            xs.swap(i, j);
        }
    }
}

// ─────────────────────────── normal CDF / quantile ──────────────────────────

/// Standard normal CDF (Abramowitz–Stegun 7.1.26 via erf; |err| < 1.5e-7).
#[cfg_attr(not(test), allow(dead_code))]
fn phi(x: f64) -> f64 {
    let z = x / std::f64::consts::SQRT_2;
    let t = 1.0 / (1.0 + 0.327_591_1 * z.abs());
    let poly = t
        * (0.254_829_592
            + t * (-0.284_496_736
                + t * (1.421_413_741 + t * (-1.453_152_027 + t * 1.061_405_429))));
    let erf_abs = 1.0 - poly * (-z * z).exp();
    let erf = if z >= 0.0 { erf_abs } else { -erf_abs };
    0.5 * (1.0 + erf)
}

/// Standard normal quantile (Acklam's rational approximation; |rel err| < 1.2e-9).
fn phi_inv(p: f64) -> f64 {
    assert!(p > 0.0 && p < 1.0, "phi_inv domain");
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    const PLOW: f64 = 0.024_25;
    if p < PLOW {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= 1.0 - PLOW {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        -phi_inv(1.0 - p)
    }
}

// ───────────────────────────── small statistics ─────────────────────────────

/// Rank-based AUROC with midrank tie handling; `None` if one class is absent.
fn auroc(scores: &[(f64, bool)]) -> Option<f64> {
    let n_pos = scores.iter().filter(|(_, y)| *y).count();
    let n_neg = scores.len() - n_pos;
    if n_pos == 0 || n_neg == 0 {
        return None;
    }
    let mut idx: Vec<usize> = (0..scores.len()).collect();
    idx.sort_by(|&a, &b| scores[a].0.total_cmp(&scores[b].0));
    let mut ranks = vec![0.0; scores.len()];
    let mut i = 0;
    while i < idx.len() {
        let mut j = i;
        while j + 1 < idx.len() && scores[idx[j + 1]].0 == scores[idx[i]].0 {
            j += 1;
        }
        let midrank = (i + j) as f64 / 2.0 + 1.0;
        for &k in &idx[i..=j] {
            ranks[k] = midrank;
        }
        i = j + 1;
    }
    let rank_sum_pos: f64 = scores
        .iter()
        .zip(&ranks)
        .filter(|((_, y), _)| *y)
        .map(|(_, r)| r)
        .sum();
    let u = rank_sum_pos - (n_pos * (n_pos + 1)) as f64 / 2.0;
    Some(u / (n_pos as f64 * n_neg as f64))
}

/// Average ranks (midranks for ties).
fn ranks_of(xs: &[f64]) -> Vec<f64> {
    let mut idx: Vec<usize> = (0..xs.len()).collect();
    idx.sort_by(|&a, &b| xs[a].total_cmp(&xs[b]));
    let mut r = vec![0.0; xs.len()];
    let mut i = 0;
    while i < idx.len() {
        let mut j = i;
        while j + 1 < idx.len() && xs[idx[j + 1]] == xs[idx[i]] {
            j += 1;
        }
        let midrank = (i + j) as f64 / 2.0 + 1.0;
        for &k in &idx[i..=j] {
            r[k] = midrank;
        }
        i = j + 1;
    }
    r
}

/// Spearman rank correlation (Pearson on midranks); `None` if degenerate.
fn spearman(xs: &[f64], ys: &[f64]) -> Option<f64> {
    if xs.len() < 3 {
        return None;
    }
    let (rx, ry) = (ranks_of(xs), ranks_of(ys));
    let n = xs.len() as f64;
    let (mx, my) = (rx.iter().sum::<f64>() / n, ry.iter().sum::<f64>() / n);
    let mut sxy = 0.0;
    let mut sxx = 0.0;
    let mut syy = 0.0;
    for i in 0..xs.len() {
        let (dx, dy) = (rx[i] - mx, ry[i] - my);
        sxy += dx * dy;
        sxx += dx * dx;
        syy += dy * dy;
    }
    if sxx <= 0.0 || syy <= 0.0 {
        return None;
    }
    Some(sxy / (sxx * syy).sqrt())
}

/// Kendall τ-a for tiny vectors (per-case orderings of 2–3 modalities).
fn kendall_tau(xs: &[f64], ys: &[f64]) -> f64 {
    let n = xs.len();
    let mut num = 0i64;
    let mut pairs = 0i64;
    for i in 0..n {
        for j in (i + 1)..n {
            let a = (xs[i] - xs[j]).signum();
            let b = (ys[i] - ys[j]).signum();
            num += (a * b) as i64;
            pairs += 1;
        }
    }
    num as f64 / pairs as f64
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let idx = p * (sorted.len() - 1) as f64;
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;
    let w = idx - lo as f64;
    sorted[lo] * (1.0 - w) + sorted[hi] * w
}

/// Ridge-regularized logistic regression via IRLS (intercept + features).
/// Returns coefficient vector [w0, w1, …]; deterministic; ridge keeps the
/// separable small-sample case finite.
#[allow(clippy::needless_range_loop)] // dense matrix algebra reads clearest indexed
fn logistic_fit(x: &[Vec<f64>], y: &[bool], ridge: f64) -> Vec<f64> {
    let n = x.len();
    let d = x[0].len() + 1;
    let mut w = vec![0.0; d];
    let xi = |i: usize, j: usize| if j == 0 { 1.0 } else { x[i][j - 1] };
    for _ in 0..30 {
        // Gradient and Hessian of the penalized log-likelihood.
        let mut g = vec![0.0; d];
        let mut h = vec![vec![0.0; d]; d];
        for i in 0..n {
            let eta: f64 = (0..d).map(|j| w[j] * xi(i, j)).sum();
            let p = 1.0 / (1.0 + (-eta).exp());
            let r = if y[i] { 1.0 } else { 0.0 } - p;
            let s = (p * (1.0 - p)).max(1e-9);
            for j in 0..d {
                g[j] += r * xi(i, j);
                for k in 0..d {
                    h[j][k] += s * xi(i, j) * xi(i, k);
                }
            }
        }
        for j in 1..d {
            g[j] -= ridge * w[j];
            h[j][j] += ridge;
        }
        h[0][0] += 1e-12;
        // Solve h Δ = g by Gaussian elimination with partial pivoting.
        let mut a = h.clone();
        let mut b = g.clone();
        for col in 0..d {
            let piv = (col..d)
                .max_by(|&r1, &r2| a[r1][col].abs().total_cmp(&a[r2][col].abs()))
                .unwrap_or(col);
            a.swap(col, piv);
            b.swap(col, piv);
            if a[col][col].abs() < 1e-12 {
                continue;
            }
            for row in (col + 1)..d {
                let f = a[row][col] / a[col][col];
                for k in col..d {
                    a[row][k] -= f * a[col][k];
                }
                b[row] -= f * b[col];
            }
        }
        let mut delta = vec![0.0; d];
        for row in (0..d).rev() {
            if a[row][row].abs() < 1e-12 {
                continue;
            }
            let mut s = b[row];
            for k in (row + 1)..d {
                s -= a[row][k] * delta[k];
            }
            delta[row] = s / a[row][row];
        }
        let step: f64 = delta.iter().map(|v| v * v).sum::<f64>().sqrt();
        for j in 0..d {
            w[j] += delta[j];
        }
        if step < 1e-8 {
            break;
        }
    }
    w
}

fn logistic_score(w: &[f64], features: &[f64]) -> f64 {
    let mut eta = w[0];
    for (j, f) in features.iter().enumerate() {
        eta += w[j + 1] * f;
    }
    eta
}

// ───────────────────────────────── reporting ────────────────────────────────

/// One grid cell of a power surface: `n_units` at injected `effect` → `power`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerCell {
    /// Analysis units in this idealized grid cell (episodes / tasks / cases).
    pub n_units: usize,
    /// Injected true effect on the endpoint's own scale (ΔAUROC / ρ / mean τ).
    pub effect: f64,
    /// Fraction of replicates achieving one-sided directional significance.
    pub significance_rate: f64,
    /// Fraction achieving the retired calculation's internal criterion (for legacy H1:
    /// significance AND point estimate ≥ its threshold; for the other legacy endpoints,
    /// equal to `significance_rate`).
    pub power: f64,
    /// H1 only: fraction of replicates declared futile (95% CI upper < 0.02).
    pub futility_rate: f64,
    /// Mean point estimate across replicates (calibration check).
    pub mean_point_estimate: f64,
    pub replicates: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerGateConfig {
    /// Candidate legacy-v10.7 H1 analysis-unit counts, not capture requirements.
    pub h1_episodes_grid: Vec<usize>,
    /// Injected legacy-v10.7 H1 incremental ΔAUROC values.
    pub h1_delta_grid: Vec<f64>,
    /// Standalone AUROC of the pooled baseline feature set.
    pub h1_baseline_auroc: f64,
    /// Episode-level failure prevalence.
    pub h1_failure_rate: f64,
    /// Fraction of episodes held out for the endpoint contrast.
    pub h1_heldout_frac: f64,
    /// Candidate legacy-v10.7 H2/H4 task counts, not capture requirements.
    pub h2h4_tasks_grid: Vec<usize>,
    /// Injected marginal Spearman |ρ| for the retired calculation.
    pub h2h4_rho: f64,
    /// Tasks per family (families are resampling blocks).
    pub h2h4_family_size: usize,
    /// Between-family random-effect SD on the outcome variable.
    pub h2h4_family_sd: f64,
    /// Candidate legacy-v10.7 H3 case counts, not capture requirements.
    pub h3_cases_grid: Vec<usize>,
    /// Injected mean per-case Kendall τ for the retired calculation.
    pub h3_mean_tau: f64,
    /// Cases per family (families are resampling blocks).
    pub h3_family_size: usize,
    /// Gaussian score-error ICC between cases in one H3 family. This is a
    /// latent-noise ICC, not the nonlinear ICC of the resulting Kendall taus.
    pub h3_family_icc: f64,
    /// Bootstrap resamples per replicate.
    pub n_boot: usize,
    /// Monte-Carlo replicates per grid cell.
    pub replicates: usize,
    /// One-sided significance level.
    pub alpha: f64,
    /// Retired H1 point threshold used by the historical internal criterion.
    pub h1_min_effect: f64,
    /// H1 futility bound (95% CI upper < this ⇒ futile).
    pub h1_futility_bound: f64,
    /// Target Monte-Carlo rate used by the historical finite-grid comparison.
    pub target_power: f64,
    pub seed: u64,
}

impl Default for PowerGateConfig {
    fn default() -> Self {
        Self {
            h1_episodes_grid: vec![40, 80, 160, 320, 480, 640, 960],
            h1_delta_grid: vec![0.0, 0.03, 0.05, 0.08],
            h1_baseline_auroc: 0.65,
            h1_failure_rate: 0.30,
            h1_heldout_frac: 0.5,
            h2h4_tasks_grid: vec![8, 12, 16, 24, 32, 48, 64, 96, 128],
            h2h4_rho: 0.3,
            h2h4_family_size: 4,
            h2h4_family_sd: 0.3,
            h3_cases_grid: vec![20, 30, 40, 60],
            h3_mean_tau: 1.0 / 3.0,
            h3_family_size: 4,
            h3_family_icc: 0.3,
            n_boot: 500,
            replicates: 400,
            alpha: 0.05,
            h1_min_effect: 0.05,
            h1_futility_bound: 0.02,
            target_power: 0.8,
            seed: 0x1483,
        }
    }
}

/// Configuration error detected before any simulation is run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerGateConfigError {
    pub field: String,
    pub reason: String,
}

impl PowerGateConfigError {
    fn new(field: &str, reason: impl Into<String>) -> Self {
        Self {
            field: field.to_string(),
            reason: reason.into(),
        }
    }
}

impl std::fmt::Display for PowerGateConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "legacy-sensitivity `{}`: {}", self.field, self.reason)
    }
}

impl std::error::Error for PowerGateConfigError {}

fn has_duplicate_usize(values: &[usize]) -> bool {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted.windows(2).any(|pair| pair[0] == pair[1])
}

fn approximately_equal(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12
}

impl PowerGateConfig {
    /// Rejects values that make a labeled DGP or its bootstrap undefined.
    pub fn validate(&self) -> Result<(), PowerGateConfigError> {
        if self.replicates == 0 {
            return Err(PowerGateConfigError::new(
                "replicates",
                "must be greater than zero",
            ));
        }
        if self.n_boot < 2 {
            return Err(PowerGateConfigError::new("n_boot", "must be at least two"));
        }
        if !self.alpha.is_finite() || self.alpha <= 0.0 || self.alpha >= 0.5 {
            return Err(PowerGateConfigError::new(
                "alpha",
                "must be finite and in (0, 0.5)",
            ));
        }
        if self.n_boot as f64 * self.alpha < 1.0 {
            return Err(PowerGateConfigError::new(
                "n_boot",
                "must provide at least one bootstrap draw in the alpha tail",
            ));
        }
        if self.replicates as f64 * self.alpha < 1.0 {
            return Err(PowerGateConfigError::new(
                "replicates",
                "must provide at least one Monte-Carlo replicate at the nominal alpha rate",
            ));
        }
        if !self.target_power.is_finite() || self.target_power <= 0.0 || self.target_power > 1.0 {
            return Err(PowerGateConfigError::new(
                "target_power",
                "must be finite and in (0, 1]",
            ));
        }

        if self.h1_episodes_grid.is_empty()
            || self.h1_episodes_grid.iter().any(|&n| n < 4)
            || has_duplicate_usize(&self.h1_episodes_grid)
        {
            return Err(PowerGateConfigError::new(
                "h1_episodes_grid",
                "must contain unique counts of at least four episodes",
            ));
        }
        if !self.h1_baseline_auroc.is_finite()
            || self.h1_baseline_auroc <= 0.0
            || self.h1_baseline_auroc >= 1.0
        {
            return Err(PowerGateConfigError::new(
                "h1_baseline_auroc",
                "must be finite and in (0, 1)",
            ));
        }
        if !self.h1_failure_rate.is_finite()
            || self.h1_failure_rate <= 0.0
            || self.h1_failure_rate >= 1.0
        {
            return Err(PowerGateConfigError::new(
                "h1_failure_rate",
                "must be finite and in (0, 1)",
            ));
        }
        if !self.h1_heldout_frac.is_finite()
            || self.h1_heldout_frac <= 0.0
            || self.h1_heldout_frac >= 1.0
        {
            return Err(PowerGateConfigError::new(
                "h1_heldout_frac",
                "must be finite and in (0, 1)",
            ));
        }
        if self.h1_episodes_grid.iter().any(|&n| {
            let n_train = (n as f64 * (1.0 - self.h1_heldout_frac)).round() as usize;
            n_train < 2 || n.saturating_sub(n_train) < 2
        }) {
            return Err(PowerGateConfigError::new(
                "h1_heldout_frac",
                "must leave at least two train and two held-out episodes at every grid count",
            ));
        }
        if self.h1_delta_grid.is_empty()
            || self
                .h1_delta_grid
                .iter()
                .any(|delta| !delta.is_finite() || *delta < 0.0)
        {
            return Err(PowerGateConfigError::new(
                "h1_delta_grid",
                "must contain finite non-negative effects",
            ));
        }
        let mut deltas = self.h1_delta_grid.clone();
        deltas.sort_by(f64::total_cmp);
        if deltas
            .windows(2)
            .any(|pair| approximately_equal(pair[0], pair[1]))
        {
            return Err(PowerGateConfigError::new(
                "h1_delta_grid",
                "must not contain duplicate effects",
            ));
        }
        if !self
            .h1_delta_grid
            .iter()
            .any(|&delta| approximately_equal(delta, 0.0))
            || !self
                .h1_delta_grid
                .iter()
                .any(|&delta| approximately_equal(delta, self.h1_min_effect))
        {
            return Err(PowerGateConfigError::new(
                "h1_delta_grid",
                "must contain both the null and h1_min_effect cells",
            ));
        }
        if !self.h1_min_effect.is_finite() || self.h1_min_effect <= 0.0 {
            return Err(PowerGateConfigError::new(
                "h1_min_effect",
                "must be finite and positive",
            ));
        }
        if self
            .h1_delta_grid
            .iter()
            .any(|delta| self.h1_baseline_auroc + delta >= 1.0)
        {
            return Err(PowerGateConfigError::new(
                "h1_delta_grid",
                "baseline AUROC plus every injected delta must be below one",
            ));
        }
        if !self.h1_futility_bound.is_finite()
            || self.h1_futility_bound < 0.0
            || self.h1_futility_bound >= self.h1_min_effect
        {
            return Err(PowerGateConfigError::new(
                "h1_futility_bound",
                "must be finite, non-negative, and below h1_min_effect",
            ));
        }

        if self.h2h4_tasks_grid.is_empty()
            || self.h2h4_tasks_grid.iter().any(|&n| n < 3)
            || has_duplicate_usize(&self.h2h4_tasks_grid)
        {
            return Err(PowerGateConfigError::new(
                "h2h4_tasks_grid",
                "must contain unique counts of at least three tasks",
            ));
        }
        if self.h2h4_family_size == 0 {
            return Err(PowerGateConfigError::new(
                "h2h4_family_size",
                "must be greater than zero",
            ));
        }
        if self
            .h2h4_tasks_grid
            .iter()
            .any(|&n| n.div_ceil(self.h2h4_family_size) < 2)
        {
            return Err(PowerGateConfigError::new(
                "h2h4_family_size",
                "must yield at least two family blocks at every task-grid count",
            ));
        }
        if !self.h2h4_family_sd.is_finite() || self.h2h4_family_sd < 0.0 {
            return Err(PowerGateConfigError::new(
                "h2h4_family_sd",
                "must be finite and non-negative",
            ));
        }
        if !self.h2h4_rho.is_finite() || self.h2h4_rho <= 0.0 || self.h2h4_rho >= 1.0 {
            return Err(PowerGateConfigError::new(
                "h2h4_rho",
                "must be a finite magnitude in (0, 1)",
            ));
        }
        calibrated_latent_pearson(self.h2h4_rho, self.h2h4_family_sd)?;

        if self.h3_cases_grid.is_empty()
            || self.h3_cases_grid.iter().any(|&n| n < 20)
            || has_duplicate_usize(&self.h3_cases_grid)
        {
            return Err(PowerGateConfigError::new(
                "h3_cases_grid",
                "must contain unique counts of at least 20 matched cases",
            ));
        }
        if self.h3_family_size == 0 {
            return Err(PowerGateConfigError::new(
                "h3_family_size",
                "must be greater than zero",
            ));
        }
        if self
            .h3_cases_grid
            .iter()
            .any(|&n| n.div_ceil(self.h3_family_size) < 2)
        {
            return Err(PowerGateConfigError::new(
                "h3_family_size",
                "must yield at least two family blocks at every case-grid count",
            ));
        }
        if !self.h3_family_icc.is_finite() || self.h3_family_icc < 0.0 || self.h3_family_icc >= 1.0
        {
            return Err(PowerGateConfigError::new(
                "h3_family_icc",
                "must be finite and in [0, 1)",
            ));
        }
        if !self.h3_mean_tau.is_finite() || self.h3_mean_tau <= 0.0 || self.h3_mean_tau >= 1.0 {
            return Err(PowerGateConfigError::new(
                "h3_mean_tau",
                "must be finite and in (0, 1) for this ordered-noise DGP",
            ));
        }
        h3_calibrate_sigma(self.h3_mean_tau)?;

        Ok(())
    }
}

/// One-sided direction encoded by the retired v10.7 calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PredictedDirection {
    Positive,
    Negative,
}

/// Diagnostic for one retired idealized endpoint surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegacyEndpointDiagnostic {
    pub endpoint_id: String,
    pub endpoint: String,
    pub dgp_tag: String,
    pub unit: String,
    pub predicted_direction: PredictedDirection,
    pub minimum_effect_magnitude: f64,
    /// Smallest evaluated grid n meeting both target power and its same-n null
    /// size tolerance. This is not a capture requirement or guarantee.
    pub smallest_passing_grid_n: Option<usize>,
    /// Empirical null rate for `smallest_passing_grid_n`, never another n.
    pub null_rate_at_smallest_passing_grid_n: Option<f64>,
    /// Maximum accepted null rate at the selected n (alpha + 3 MC SEs).
    pub null_size_tolerance_at_smallest_passing_grid_n: Option<f64>,
    /// Whether this surface met its retired finite-grid comparison only.
    pub legacy_internal_criterion_met: bool,
}

/// Whether a report may be promoted into current scientific evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacySensitivityPromotionStatus {
    Nonpromotable,
}

/// Evaluation status of the current EC1/H1–H4 hypothesis registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurrentHypothesisGateStatus {
    NotEvaluated,
}

/// Scientific-success status permitted for this retired calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurrentScientificSuccessStatus {
    NotEstablished,
}

/// Machine-readable caveat attached to every report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerGateLimitation {
    pub code: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerGateReport {
    pub schema_version: String,
    pub artifact_id: String,
    pub config: PowerGateConfig,
    #[serde(rename = "legacy_v10_7_h1_surface")]
    pub h1: Vec<PowerCell>,
    #[serde(rename = "legacy_v10_7_h2_surface")]
    pub h2: Vec<PowerCell>,
    #[serde(rename = "legacy_v10_7_h2_null_surface")]
    pub h2_null: Vec<PowerCell>,
    #[serde(rename = "legacy_v10_7_h3_surface")]
    pub h3: Vec<PowerCell>,
    #[serde(rename = "legacy_v10_7_h3_null_surface")]
    pub h3_null: Vec<PowerCell>,
    #[serde(rename = "legacy_v10_7_h4_surface")]
    pub h4: Vec<PowerCell>,
    #[serde(rename = "legacy_v10_7_h4_null_surface")]
    pub h4_null: Vec<PowerCell>,
    pub legacy_endpoint_diagnostics: Vec<LegacyEndpointDiagnostic>,
    /// True iff every retired surface met its own same-n finite-grid comparison.
    /// This is not a current scientific gate or success flag.
    pub legacy_internal_grid_criterion_met: bool,
    pub promotion_status: LegacySensitivityPromotionStatus,
    pub current_hypothesis_gate_status: CurrentHypothesisGateStatus,
    pub current_scientific_success_status: CurrentScientificSuccessStatus,
    pub limitations: Vec<PowerGateLimitation>,
}

// ─────────────────────────────── H1 simulator ───────────────────────────────

/// One H1 replicate: returns (point ΔAUROC, significant, success, futile),
/// or `None` if a degenerate split could not be avoided.
fn h1_replicate(
    rng: &mut Rng,
    episodes: usize,
    delta: f64,
    cfg: &PowerGateConfig,
) -> Option<(f64, bool, bool, bool)> {
    let d_base = std::f64::consts::SQRT_2 * phi_inv(cfg.h1_baseline_auroc);
    let target = (cfg.h1_baseline_auroc + delta).min(0.999);
    let d_comb = std::f64::consts::SQRT_2 * phi_inv(target);
    let d_pid = (d_comb * d_comb - d_base * d_base).max(0.0).sqrt();

    // Episode-level features: baseline score b, PID feature p (independent
    // signal). Class-conditional unit-variance Gaussians — binormal by
    // construction, so injected AUROCs are exact.
    let mut y = Vec::with_capacity(episodes);
    let mut feats = Vec::with_capacity(episodes);
    for _ in 0..episodes {
        let yi = rng.next_f64() < cfg.h1_failure_rate;
        let shift = if yi { 1.0 } else { 0.0 };
        let b = rng.next_gaussian() + d_base * shift;
        let p = rng.next_gaussian() + d_pid * shift;
        y.push(yi);
        feats.push(vec![b, p]);
    }

    // Episode-level split (train/held-out), both classes required on each side.
    let mut order: Vec<usize> = (0..episodes).collect();
    for _ in 0..20 {
        rng.shuffle(&mut order);
        let n_train = ((episodes as f64) * (1.0 - cfg.h1_heldout_frac)).round() as usize;
        let (tr, ho) = order.split_at(n_train);
        let has_both = |ix: &[usize]| ix.iter().any(|&i| y[i]) && ix.iter().any(|&i| !y[i]);
        if !(has_both(tr) && has_both(ho)) {
            continue;
        }

        let xa: Vec<Vec<f64>> = tr.iter().map(|&i| vec![feats[i][0]]).collect();
        let xb: Vec<Vec<f64>> = tr.iter().map(|&i| feats[i].clone()).collect();
        let yt: Vec<bool> = tr.iter().map(|&i| y[i]).collect();
        let wa = logistic_fit(&xa, &yt, 1e-6);
        let wb = logistic_fit(&xb, &yt, 1e-6);

        let ho_scores: Vec<(f64, f64, bool)> = ho
            .iter()
            .map(|&i| {
                (
                    logistic_score(&wa, &feats[i][..1]),
                    logistic_score(&wb, &feats[i]),
                    y[i],
                )
            })
            .collect();
        let sa: Vec<(f64, bool)> = ho_scores.iter().map(|&(a, _, yy)| (a, yy)).collect();
        let sb: Vec<(f64, bool)> = ho_scores.iter().map(|&(_, b, yy)| (b, yy)).collect();
        let (aa, ab) = (auroc(&sa)?, auroc(&sb)?);
        let point = ab - aa;

        // Paired episode-level bootstrap on the held-out episodes: the same
        // resample scores both models, so the Δ distribution is the paired one.
        let mut deltas = Vec::with_capacity(cfg.n_boot);
        for _ in 0..cfg.n_boot {
            let mut ra = Vec::with_capacity(ho.len());
            let mut rb = Vec::with_capacity(ho.len());
            for _ in 0..ho.len() {
                let k = (rng.next_u64() % ho.len() as u64) as usize;
                ra.push((ho_scores[k].0, ho_scores[k].2));
                rb.push((ho_scores[k].1, ho_scores[k].2));
            }
            if let (Some(ba), Some(bb)) = (auroc(&ra), auroc(&rb)) {
                deltas.push(bb - ba);
            }
        }
        if deltas.len() < cfg.n_boot / 2 {
            return None;
        }
        deltas.sort_by(f64::total_cmp);
        let q_alpha = percentile(&deltas, cfg.alpha);
        let q_hi = percentile(&deltas, 0.975);
        let significant = q_alpha > 0.0;
        let success = significant && point >= cfg.h1_min_effect;
        let futile = q_hi < cfg.h1_futility_bound;
        return Some((point, significant, success, futile));
    }
    None
}

// ───────────────────────────── H2/H4 simulator ──────────────────────────────

/// Conditional copula correlation required to retain `marginal_rho` after an
/// outcome-only Gaussian family random effect is added.
fn calibrated_latent_pearson(
    marginal_rho: f64,
    family_sd: f64,
) -> Result<f64, PowerGateConfigError> {
    let marginal_pearson = 2.0 * (std::f64::consts::PI * marginal_rho / 6.0).sin();
    let attenuation = (1.0 + family_sd * family_sd).sqrt();
    let latent_pearson = marginal_pearson * attenuation;
    if !latent_pearson.is_finite() || latent_pearson.abs() > 1.0 {
        let max_rho = 6.0 / std::f64::consts::PI * (1.0 / (2.0 * attenuation)).asin();
        return Err(PowerGateConfigError::new(
            "h2h4_rho",
            format!(
                "|rho|={:.6} is impossible with family_sd={:.6}; maximum marginal |rho| is {:.6}",
                marginal_rho.abs(),
                family_sd,
                max_rho
            ),
        ));
    }
    Ok(latent_pearson)
}

fn h2h4_task_sample(
    rng: &mut Rng,
    tasks: usize,
    latent_pearson: f64,
    cfg: &PowerGateConfig,
) -> (Vec<f64>, Vec<f64>, Vec<usize>) {
    let n_fam = tasks.div_ceil(cfg.h2h4_family_size);
    let mut fam_of = Vec::with_capacity(tasks);
    let mut xs = Vec::with_capacity(tasks);
    let mut ys = Vec::with_capacity(tasks);
    for f in 0..n_fam {
        let family_effect = rng.next_gaussian() * cfg.h2h4_family_sd;
        for _ in 0..cfg.h2h4_family_size {
            if xs.len() == tasks {
                break;
            }
            let x = rng.next_gaussian();
            let residual = rng.next_gaussian();
            xs.push(x);
            ys.push(
                latent_pearson * x
                    + (1.0 - latent_pearson * latent_pearson).max(0.0).sqrt() * residual
                    + family_effect,
            );
            fam_of.push(f);
        }
    }
    (xs, ys, fam_of)
}

fn directional_significance(
    sorted_bootstrap: &[f64],
    alpha: f64,
    direction: PredictedDirection,
) -> bool {
    match direction {
        PredictedDirection::Positive => percentile(sorted_bootstrap, alpha) > 0.0,
        PredictedDirection::Negative => percentile(sorted_bootstrap, 1.0 - alpha) < 0.0,
    }
}

/// One H2 or H4 replicate at the task level with family clustering.
fn h2h4_replicate(
    rng: &mut Rng,
    tasks: usize,
    latent_pearson: f64,
    direction: PredictedDirection,
    cfg: &PowerGateConfig,
) -> Option<(f64, bool)> {
    let n_fam = tasks.div_ceil(cfg.h2h4_family_size);
    let (xs, ys, fam_of) = h2h4_task_sample(rng, tasks, latent_pearson, cfg);
    let point = spearman(&xs, &ys)?;

    // Family-blocked bootstrap: resample families with replacement, pool tasks.
    let mut fam_tasks: Vec<Vec<usize>> = vec![Vec::new(); n_fam];
    for (t, &f) in fam_of.iter().enumerate() {
        fam_tasks[f].push(t);
    }
    let mut boots = Vec::with_capacity(cfg.n_boot);
    for _ in 0..cfg.n_boot {
        let mut bx = Vec::with_capacity(tasks);
        let mut by = Vec::with_capacity(tasks);
        for _ in 0..n_fam {
            let f = (rng.next_u64() % n_fam as u64) as usize;
            for &t in &fam_tasks[f] {
                bx.push(xs[t]);
                by.push(ys[t]);
            }
        }
        if let Some(sr) = spearman(&bx, &by) {
            boots.push(sr);
        }
    }
    if boots.len() < cfg.n_boot / 2 {
        return None;
    }
    boots.sort_by(f64::total_cmp);
    let significant = directional_significance(&boots, cfg.alpha, direction);
    Some((point, significant))
}

// ─────────────────────────────── H3 simulator ───────────────────────────────

/// Exact marginal E[per-case Kendall tau] for the three ordered scores under
/// independent equal-variance Gaussian score noise.
fn h3_expected_tau(sigma: f64) -> f64 {
    if sigma == 0.0 {
        return 1.0;
    }
    let pair_tau = |gap: f64| 2.0 * phi(gap / (std::f64::consts::SQRT_2 * sigma)) - 1.0;
    (2.0 * pair_tau(1.0) + pair_tau(2.0)) / 3.0
}

/// Calibrate score-noise SD to the target marginal mean tau. The H3 family
/// decomposition preserves this marginal variance for every configured ICC.
fn h3_calibrate_sigma(target_tau: f64) -> Result<f64, PowerGateConfigError> {
    if !target_tau.is_finite() || target_tau <= 0.0 || target_tau >= 1.0 {
        return Err(PowerGateConfigError::new(
            "h3_mean_tau",
            "must be finite and in (0, 1) for this ordered-noise DGP",
        ));
    }
    let mut hi = 1.0;
    while h3_expected_tau(hi) > target_tau {
        hi *= 2.0;
        if !hi.is_finite() {
            return Err(PowerGateConfigError::new(
                "h3_mean_tau",
                "could not calibrate a finite score-noise scale",
            ));
        }
    }
    let mut lo = 0.0;
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if h3_expected_tau(mid) > target_tau {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(0.5 * (lo + hi))
}

fn h3_case_sample(
    rng: &mut Rng,
    cases: usize,
    signal_scale: f64,
    sigma: f64,
    cfg: &PowerGateConfig,
) -> (Vec<f64>, Vec<usize>) {
    let n_fam = cases.div_ceil(cfg.h3_family_size);
    let family_weight = cfg.h3_family_icc.sqrt();
    let case_weight = (1.0 - cfg.h3_family_icc).sqrt();
    let unq = [1.0, 2.0, 3.0];
    let mut taus = Vec::with_capacity(cases);
    let mut fam_of = Vec::with_capacity(cases);
    for f in 0..n_fam {
        let family_noise: [f64; 3] = std::array::from_fn(|_| rng.next_gaussian());
        for _ in 0..cfg.h3_family_size {
            if taus.len() == cases {
                break;
            }
            let effect: [f64; 3] = std::array::from_fn(|modality| {
                signal_scale * unq[modality]
                    + sigma
                        * (family_weight * family_noise[modality]
                            + case_weight * rng.next_gaussian())
            });
            taus.push(kendall_tau(&unq, &effect));
            fam_of.push(f);
        }
    }
    (taus, fam_of)
}

/// One H3 replicate: mean per-case Kendall τ with family-blocked
/// case-resampling bootstrap.
fn h3_replicate(
    rng: &mut Rng,
    cases: usize,
    signal_scale: f64,
    sigma: f64,
    cfg: &PowerGateConfig,
) -> Option<(f64, bool)> {
    let n_fam = cases.div_ceil(cfg.h3_family_size);
    let (taus, fam_of) = h3_case_sample(rng, cases, signal_scale, sigma, cfg);
    let point = taus.iter().sum::<f64>() / taus.len() as f64;

    let mut fam_cases: Vec<Vec<usize>> = vec![Vec::new(); n_fam];
    for (c, &f) in fam_of.iter().enumerate() {
        fam_cases[f].push(c);
    }
    let mut boots = Vec::with_capacity(cfg.n_boot);
    for _ in 0..cfg.n_boot {
        let mut acc = 0.0;
        let mut n = 0usize;
        for _ in 0..n_fam {
            let f = (rng.next_u64() % n_fam as u64) as usize;
            for &c in &fam_cases[f] {
                acc += taus[c];
                n += 1;
            }
        }
        if n > 0 {
            boots.push(acc / n as f64);
        }
    }
    if boots.len() < cfg.n_boot / 2 {
        return None;
    }
    boots.sort_by(f64::total_cmp);
    let significant = percentile(&boots, cfg.alpha) > 0.0;
    Some((point, significant))
}

// ───────────────────── retired finite-grid comparison logic ─────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
struct PassingGridCell {
    n_units: usize,
    null_rate: f64,
    null_size_tolerance: f64,
}

fn null_size_tolerance(alpha: f64, replicates: usize) -> Option<f64> {
    (replicates > 0)
        .then(|| (alpha + 3.0 * (alpha * (1.0 - alpha) / replicates as f64).sqrt()).min(1.0))
}

fn select_smallest_passing_grid_n(
    cells: &[PowerCell],
    effect: f64,
    null_cells: &[PowerCell],
    cfg: &PowerGateConfig,
) -> Option<PassingGridCell> {
    let mut candidates: Vec<&PowerCell> = cells
        .iter()
        .filter(|cell| {
            approximately_equal(cell.effect, effect)
                && cell.power.is_finite()
                && cell.power >= cfg.target_power
        })
        .collect();
    candidates.sort_by_key(|cell| cell.n_units);
    candidates.into_iter().find_map(|candidate| {
        let null = null_cells.iter().find(|cell| {
            cell.n_units == candidate.n_units && approximately_equal(cell.effect, 0.0)
        })?;
        let tolerance = null_size_tolerance(cfg.alpha, null.replicates)?;
        (null.significance_rate.is_finite() && null.significance_rate <= tolerance).then_some(
            PassingGridCell {
                n_units: candidate.n_units,
                null_rate: null.significance_rate,
                null_size_tolerance: tolerance,
            },
        )
    })
}

struct LegacyEndpointSpec<'a> {
    endpoint_id: &'a str,
    endpoint: &'a str,
    dgp_tag: &'a str,
    unit: &'a str,
    predicted_direction: PredictedDirection,
    minimum_effect_magnitude: f64,
    signed_effect: f64,
}

fn legacy_endpoint_diagnostic(
    spec: LegacyEndpointSpec<'_>,
    cells: &[PowerCell],
    null_cells: &[PowerCell],
    cfg: &PowerGateConfig,
) -> LegacyEndpointDiagnostic {
    let selected = select_smallest_passing_grid_n(cells, spec.signed_effect, null_cells, cfg);
    LegacyEndpointDiagnostic {
        endpoint_id: spec.endpoint_id.to_string(),
        endpoint: spec.endpoint.to_string(),
        dgp_tag: spec.dgp_tag.to_string(),
        unit: spec.unit.to_string(),
        predicted_direction: spec.predicted_direction,
        minimum_effect_magnitude: spec.minimum_effect_magnitude,
        smallest_passing_grid_n: selected.map(|cell| cell.n_units),
        null_rate_at_smallest_passing_grid_n: selected.map(|cell| cell.null_rate),
        null_size_tolerance_at_smallest_passing_grid_n: selected
            .map(|cell| cell.null_size_tolerance),
        legacy_internal_criterion_met: selected.is_some(),
    }
}

fn require_complete_replicates(
    endpoint: &str,
    n_units: usize,
    effect: f64,
    valid: usize,
    requested: usize,
) -> Result<(), PowerGateConfigError> {
    if valid != requested {
        return Err(PowerGateConfigError::new(
            "replicates",
            format!(
                "{endpoint} n={n_units} effect={effect:.6} retained {valid}/{requested} replicates; refusing a conditionally filtered rate"
            ),
        ));
    }
    Ok(())
}

/// Run the retired v10.7 sensitivity calculation. Deterministic for a fixed config.
pub fn run_legacy_sensitivity_calculation(
    cfg: &PowerGateConfig,
) -> Result<PowerGateReport, PowerGateConfigError> {
    cfg.validate()?;

    // H1 surface over (episodes × delta), including the null column.
    let mut h1 = Vec::new();
    for (gi, &n) in cfg.h1_episodes_grid.iter().enumerate() {
        for (di, &delta) in cfg.h1_delta_grid.iter().enumerate() {
            let mut sig = 0usize;
            let mut succ = 0usize;
            let mut fut = 0usize;
            let mut points = 0.0;
            let mut valid = 0usize;
            for rep in 0..cfg.replicates {
                let mut rng = Rng::new(
                    cfg.seed ^ ((0x11_0000 + gi as u64) << 32) ^ ((di as u64) << 20) ^ rep as u64,
                );
                if let Some((point, s, ok, f)) = h1_replicate(&mut rng, n, delta, cfg) {
                    valid += 1;
                    points += point;
                    sig += s as usize;
                    succ += ok as usize;
                    fut += f as usize;
                }
            }
            require_complete_replicates("H1", n, delta, valid, cfg.replicates)?;
            h1.push(PowerCell {
                n_units: n,
                effect: delta,
                significance_rate: sig as f64 / valid as f64,
                power: succ as f64 / valid as f64,
                futility_rate: fut as f64 / valid as f64,
                mean_point_estimate: points / valid as f64,
                replicates: valid,
            });
        }
    }

    // H2 and H4 use separate directional surfaces and seeds. Their common
    // Gaussian copula is calibrated to the labeled marginal rho after the
    // outcome-only family effect is added.
    let latent_rho = calibrated_latent_pearson(cfg.h2h4_rho, cfg.h2h4_family_sd)?;
    let run_h2h4 = |effect: f64,
                    latent_pearson: f64,
                    direction: PredictedDirection,
                    tag: u64|
     -> Result<Vec<PowerCell>, PowerGateConfigError> {
        cfg.h2h4_tasks_grid
            .iter()
            .enumerate()
            .map(|(gi, &n)| {
                let mut sig = 0usize;
                let mut points = 0.0;
                let mut valid = 0usize;
                for rep in 0..cfg.replicates {
                    let mut rng = Rng::new(cfg.seed ^ tag ^ ((gi as u64) << 32) ^ rep as u64);
                    if let Some((point, s)) =
                        h2h4_replicate(&mut rng, n, latent_pearson, direction, cfg)
                    {
                        valid += 1;
                        points += point;
                        sig += s as usize;
                    }
                }
                require_complete_replicates("H2/H4", n, effect, valid, cfg.replicates)?;
                let rate = sig as f64 / valid as f64;
                Ok(PowerCell {
                    n_units: n,
                    effect,
                    significance_rate: rate,
                    power: rate,
                    futility_rate: 0.0,
                    mean_point_estimate: points / valid as f64,
                    replicates: valid,
                })
            })
            .collect()
    };
    let h2 = run_h2h4(
        cfg.h2h4_rho,
        latent_rho,
        PredictedDirection::Positive,
        0x22_0000,
    )?;
    let h2_null = run_h2h4(0.0, 0.0, PredictedDirection::Positive, 0x22_1111)?;
    let h4 = run_h2h4(
        -cfg.h2h4_rho,
        -latent_rho,
        PredictedDirection::Negative,
        0x44_0000,
    )?;
    let h4_null = run_h2h4(0.0, 0.0, PredictedDirection::Negative, 0x44_1111)?;

    // H3 surface over matched cases at mean τ = 1/3, plus null.
    let sigma_alt = h3_calibrate_sigma(cfg.h3_mean_tau)?;
    let run_h3 = |signal_scale: f64,
                  sigma: f64,
                  effect: f64,
                  tag: u64|
     -> Result<Vec<PowerCell>, PowerGateConfigError> {
        cfg.h3_cases_grid
            .iter()
            .enumerate()
            .map(|(gi, &n)| {
                let mut sig = 0usize;
                let mut points = 0.0;
                let mut valid = 0usize;
                for rep in 0..cfg.replicates {
                    let mut rng = Rng::new(cfg.seed ^ tag ^ ((gi as u64) << 32) ^ rep as u64);
                    if let Some((point, s)) = h3_replicate(&mut rng, n, signal_scale, sigma, cfg) {
                        valid += 1;
                        points += point;
                        sig += s as usize;
                    }
                }
                require_complete_replicates("H3", n, effect, valid, cfg.replicates)?;
                let rate = sig as f64 / valid as f64;
                Ok(PowerCell {
                    n_units: n,
                    effect,
                    significance_rate: rate,
                    power: rate,
                    futility_rate: 0.0,
                    mean_point_estimate: points / valid as f64,
                    replicates: valid,
                })
            })
            .collect()
    };
    let h3 = run_h3(1.0, sigma_alt, cfg.h3_mean_tau, 0x33_0000)?;
    // Removing the ordered signal gives an exact marginal null while retaining
    // the configured family dependence.
    let h3_null = run_h3(0.0, 1.0, 0.0, 0x33_1111)?;

    let legacy_endpoint_diagnostics = vec![
        legacy_endpoint_diagnostic(
            LegacyEndpointSpec {
                endpoint_id: "legacy_v10_7_h1_incremental_auroc",
                endpoint: "Legacy v10.7 H1 incremental ΔAUROC",
                dgp_tag: "legacy_v10_7_h1_binormal_incremental_auroc_v1",
                unit: "episodes",
                predicted_direction: PredictedDirection::Positive,
                minimum_effect_magnitude: cfg.h1_min_effect,
                signed_effect: cfg.h1_min_effect,
            },
            &h1,
            &h1,
            cfg,
        ),
        legacy_endpoint_diagnostic(
            LegacyEndpointSpec {
                endpoint_id: "legacy_v10_7_h2_red_ablation_spearman",
                endpoint: "Legacy v10.7 H2 Red vs ablation-slope Spearman rho",
                dgp_tag: "legacy_v10_7_h2_positive_marginal_spearman_family_outcome_re_v2",
                unit: "tasks",
                predicted_direction: PredictedDirection::Positive,
                minimum_effect_magnitude: cfg.h2h4_rho,
                signed_effect: cfg.h2h4_rho,
            },
            &h2,
            &h2_null,
            cfg,
        ),
        legacy_endpoint_diagnostic(
            LegacyEndpointSpec {
                endpoint_id: "legacy_v10_7_h3_case_kendall",
                endpoint: "Legacy v10.7 H3 mean per-case Kendall tau",
                dgp_tag: "legacy_v10_7_h3_ordered_gaussian_score_noise_family_icc_v2",
                unit: "matched cases",
                predicted_direction: PredictedDirection::Positive,
                minimum_effect_magnitude: cfg.h3_mean_tau,
                signed_effect: cfg.h3_mean_tau,
            },
            &h3,
            &h3_null,
            cfg,
        ),
        legacy_endpoint_diagnostic(
            LegacyEndpointSpec {
                endpoint_id: "legacy_v10_7_h4_ssi_degradation_spearman",
                endpoint: "Legacy v10.7 H4 SSI vs L0-to-L2 degradation Spearman rho",
                dgp_tag: "legacy_v10_7_h4_negative_marginal_spearman_family_outcome_re_v2",
                unit: "tasks",
                predicted_direction: PredictedDirection::Negative,
                minimum_effect_magnitude: cfg.h2h4_rho,
                signed_effect: -cfg.h2h4_rho,
            },
            &h4,
            &h4_null,
            cfg,
        ),
    ];
    let legacy_internal_grid_criterion_met = legacy_endpoint_diagnostics
        .iter()
        .all(|diagnostic| diagnostic.legacy_internal_criterion_met);
    let limitations = vec![
        PowerGateLimitation {
            code: "nonpromotable_retired_endpoint_schema".to_string(),
            detail: "These legacy-v10.7 endpoint calculations do not evaluate the current EC1/H1–H4 registry and cannot establish scientific success.".to_string(),
        },
        PowerGateLimitation {
            code: "grid_counts_not_capture_requirements".to_string(),
            detail: "Selected n values are the smallest passing points on finite idealized grids; they are not capture requirements or guarantees.".to_string(),
        },
        PowerGateLimitation {
            code: "idealized_endpoint_dgps".to_string(),
            detail: "H2/H4 use calibrated Gaussian-copula endpoint pairs and H3 uses ordered Gaussian score noise; real endpoint measurement error and estimator instability are not simulated.".to_string(),
        },
        PowerGateLimitation {
            code: "no_nested_capture_allocation".to_string(),
            detail: "The simulator has no family→task/case→episode→severity/window allocation, binomial outcomes, instruction-eligibility gate, or fitted-transform uncertainty; H2 and H4 still share one idealized copula family rather than endpoint-specific capture DGPs.".to_string(),
        },
        PowerGateLimitation {
            code: "coarse_monte_carlo_size_tolerance".to_string(),
            detail: "The same-n size screen uses alpha plus three binomial Monte-Carlo standard errors. It is a transparent simulation tolerance, not evidence that a real test is calibrated exactly at nominal alpha.".to_string(),
        },
        PowerGateLimitation {
            code: "h1_feature_path_not_implemented".to_string(),
            detail: "The H1 binormal feature model does not supply the train-reference local PID/CI scores, leakage tests, censoring rules, or missing mandatory baselines required by the scientific endpoint.".to_string(),
        },
        PowerGateLimitation {
            code: "pilot_dependence_calibration_required".to_string(),
            detail: "Family sizes, H2/H4 outcome random-effect SD, and H3 latent score-error ICC require pilot justification before capture planning.".to_string(),
        },
    ];
    Ok(PowerGateReport {
        schema_version: "3.0".to_string(),
        artifact_id: "legacy_v10_7_endpoint_sensitivity_calculation_v1".to_string(),
        config: cfg.clone(),
        h1,
        h2,
        h2_null,
        h3,
        h3_null,
        h4,
        h4_null,
        legacy_endpoint_diagnostics,
        legacy_internal_grid_criterion_met,
        promotion_status: LegacySensitivityPromotionStatus::Nonpromotable,
        current_hypothesis_gate_status: CurrentHypothesisGateStatus::NotEvaluated,
        current_scientific_success_status: CurrentScientificSuccessStatus::NotEstablished,
        limitations,
    })
}

/// Render the retired calculation as a compact historical markdown report.
pub fn legacy_sensitivity_markdown(r: &PowerGateReport) -> String {
    let mut s = String::new();
    s.push_str("# Historical v10.7 endpoint-sensitivity calculation\n\n");
    s.push_str(
        "**NONPROMOTABLE:** this retired calculation does not evaluate the current EC1/H1–H4 \
         registry and cannot establish scientific success, capture readiness, or a study gate.\n\n",
    );
    s.push_str(&format!(
        "Replicates/cell: {} · bootstrap: {} · one-sided α = {} · target power = {}\n\n",
        r.config.replicates, r.config.n_boot, r.config.alpha, r.config.target_power
    ));
    s.push_str("The grid counts below are historical idealized sensitivities, **not capture requirements or guarantees**. A count is selected only when its own same-n null cell meets the retired Monte-Carlo size tolerance.\n\n");
    s.push_str("## Legacy endpoint diagnostics\n\n| Legacy endpoint ID | Endpoint | DGP tag | Direction | Unit | Legacy threshold | Smallest matching grid n (not a requirement) | Same-n null rate / tolerance | Retired internal criterion |\n|---|---|---|---|---|---|---|---|---|\n");
    for v in &r.legacy_endpoint_diagnostics {
        let direction = match v.predicted_direction {
            PredictedDirection::Positive => "positive",
            PredictedDirection::Negative => "negative",
        };
        let selected = v
            .smallest_passing_grid_n
            .map(|n| n.to_string())
            .unwrap_or_else(|| "NOT REACHED WITH VALID SAME-n NULL".to_string());
        let null_check = v
            .null_rate_at_smallest_passing_grid_n
            .zip(v.null_size_tolerance_at_smallest_passing_grid_n)
            .map(|(rate, tolerance)| format!("{rate:.3} / {tolerance:.3}"))
            .unwrap_or_else(|| "n/a".to_string());
        s.push_str(&format!(
            "| `{}` | {} | `{}` | {} | {} | {:.3} | {} | {} | {} |\n",
            v.endpoint_id,
            v.endpoint,
            v.dgp_tag,
            direction,
            v.unit,
            v.minimum_effect_magnitude,
            selected,
            null_check,
            if v.legacy_internal_criterion_met {
                "met"
            } else {
                "not met"
            }
        ));
    }
    let table = |s: &mut String, title: &str, cells: &[PowerCell], h1: bool| {
        s.push_str(&format!("\n## {title}\n\n"));
        if h1 {
            s.push_str("| n | effect | power (sig ∧ point≥min) | sig. rate | futility rate | mean point |\n|---|---|---|---|---|---|\n");
        } else {
            s.push_str("| n | effect | power | mean point |\n|---|---|---|---|\n");
        }
        for c in cells {
            if h1 {
                s.push_str(&format!(
                    "| {} | {:.3} | {:.3} | {:.3} | {:.3} | {:.4} |\n",
                    c.n_units,
                    c.effect,
                    c.power,
                    c.significance_rate,
                    c.futility_rate,
                    c.mean_point_estimate
                ));
            } else {
                s.push_str(&format!(
                    "| {} | {:.3} | {:.3} | {:.4} |\n",
                    c.n_units, c.effect, c.power, c.mean_point_estimate
                ));
            }
        }
    };
    table(
        &mut s,
        "H1 (episodes × ΔAUROC; includes the null column)",
        &r.h1,
        true,
    );
    table(
        &mut s,
        "H2 (tasks; predicted marginal Spearman rho > 0)",
        &r.h2,
        false,
    );
    table(
        &mut s,
        "H2 null (marginal rho = 0; positive-tail size check)",
        &r.h2_null,
        false,
    );
    table(&mut s, "H3 (matched cases at mean τ = 1/3)", &r.h3, false);
    table(&mut s, "H3 null (size check)", &r.h3_null, false);
    table(
        &mut s,
        "H4 (tasks; predicted marginal Spearman rho < 0)",
        &r.h4,
        false,
    );
    table(
        &mut s,
        "H4 null (marginal rho = 0; negative-tail size check)",
        &r.h4_null,
        false,
    );
    s.push_str("\n## Machine-readable limitations\n\n");
    for limitation in &r.limitations {
        s.push_str(&format!("- `{}`: {}\n", limitation.code, limitation.detail));
    }
    s.push_str(&format!(
        "\n**Retired internal grid criterion: {}. Promotion status: NONPROMOTABLE. \
         Current EC1/H1–H4 gate: NOT EVALUATED. Scientific success: NOT ESTABLISHED.**\n",
        if r.legacy_internal_grid_criterion_met {
            "MET"
        } else {
            "NOT MET"
        }
    ));
    s
}

// ──────────────────────────────────── tests ─────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_quantile_roundtrips() {
        for &p in &[0.01, 0.05, 0.25, 0.5, 0.65, 0.95, 0.99] {
            assert!((phi(phi_inv(p)) - p).abs() < 1e-6, "p={p}");
        }
    }

    #[test]
    fn auroc_agrees_with_hand_computation() {
        // scores: pos {3, 2}, neg {1, 2} → pairs: (3>1)+(3>2)+(2>1)+(2==2)/2 = 3.5/4
        let s = [(3.0, true), (2.0, true), (1.0, false), (2.0, false)];
        assert!((auroc(&s).unwrap() - 3.5 / 4.0).abs() < 1e-12);
    }

    #[test]
    fn spearman_and_kendall_sanity() {
        let x = [1.0, 2.0, 3.0, 4.0];
        let y = [10.0, 20.0, 30.0, 40.0];
        assert!((spearman(&x, &y).unwrap() - 1.0).abs() < 1e-12);
        let yr: Vec<f64> = y.iter().rev().copied().collect();
        assert!((spearman(&x, &yr).unwrap() + 1.0).abs() < 1e-12);
        assert!((kendall_tau(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]) - 1.0).abs() < 1e-12);
        assert!((kendall_tau(&[1.0, 2.0, 3.0], &[3.0, 2.0, 1.0]) + 1.0).abs() < 1e-12);
    }

    #[test]
    fn h1_binormal_calibration_is_exact_in_expectation() {
        // With a huge simulated capture the achieved incremental ΔAUROC must
        // match the injected effect (binormal construction, optimal combiner).
        let cfg = PowerGateConfig {
            n_boot: 50,
            replicates: 20,
            ..Default::default()
        };
        let mut acc = 0.0;
        let mut n = 0;
        for rep in 0..20 {
            let mut rng = Rng::new(0xCAFE ^ rep);
            if let Some((point, _, _, _)) = h1_replicate(&mut rng, 4000, 0.08, &cfg) {
                acc += point;
                n += 1;
            }
        }
        let mean = acc / n as f64;
        assert!(
            (mean - 0.08).abs() < 0.02,
            "mean ΔAUROC {mean:.4} should approximate the injected 0.08"
        );
    }

    #[test]
    fn h2_family_effect_calibration_hits_labeled_marginal_spearman_rho() {
        let cfg = PowerGateConfig {
            h2h4_family_sd: 0.8,
            ..Default::default()
        };
        let latent = calibrated_latent_pearson(cfg.h2h4_rho, cfg.h2h4_family_sd).unwrap();
        let mut rng = Rng::new(0xCA11_BA7E);
        let (xs, ys, _) = h2h4_task_sample(&mut rng, 100_000, latent, &cfg);
        let realized = spearman(&xs, &ys).unwrap();
        assert!(
            (realized - cfg.h2h4_rho).abs() < 0.015,
            "realized marginal rho {realized:.4}"
        );
    }

    #[test]
    fn h2_calibration_rejects_effect_impossible_after_family_attenuation() {
        let cfg = PowerGateConfig {
            h2h4_rho: 0.9,
            h2h4_family_sd: 10.0,
            ..Default::default()
        };
        let error = cfg.validate().unwrap_err();
        assert_eq!(error.field, "h2h4_rho");
    }

    #[test]
    fn h4_direction_uses_the_negative_bootstrap_tail() {
        let negative_bootstrap = [-0.6, -0.4, -0.2];
        assert!(directional_significance(
            &negative_bootstrap,
            0.05,
            PredictedDirection::Negative
        ));
    }

    #[test]
    fn h3_family_clustering_preserves_target_marginal_tau() {
        let sigma = h3_calibrate_sigma(1.0 / 3.0).unwrap();
        let cfg = PowerGateConfig {
            h3_family_icc: 0.6,
            ..Default::default()
        };
        let mut rng = Rng::new(99);
        let (taus, _) = h3_case_sample(&mut rng, 40_000, 1.0, sigma, &cfg);
        let mean = taus.iter().sum::<f64>() / taus.len() as f64;
        assert!(
            (mean - 1.0 / 3.0).abs() < 0.02,
            "calibrated mean τ {mean:.3}"
        );
    }

    #[test]
    fn h3_family_clustering_induces_within_family_tau_dependence() {
        let sigma = h3_calibrate_sigma(1.0 / 3.0).unwrap();
        let cfg = PowerGateConfig {
            h3_family_icc: 0.6,
            ..Default::default()
        };
        let mut rng = Rng::new(0x1CC);
        let (taus, _) = h3_case_sample(&mut rng, 40_000, 1.0, sigma, &cfg);
        let first: Vec<f64> = taus.iter().step_by(cfg.h3_family_size).copied().collect();
        let second: Vec<f64> = taus
            .iter()
            .skip(1)
            .step_by(cfg.h3_family_size)
            .copied()
            .collect();
        let within_family_rho = spearman(&first, &second).unwrap();
        assert!(
            within_family_rho > 0.1,
            "within-family tau rho {within_family_rho:.3}"
        );
    }

    #[test]
    fn h3_clustered_zero_signal_dgp_has_zero_marginal_tau() {
        let cfg = PowerGateConfig {
            h3_family_icc: 0.6,
            ..Default::default()
        };
        let mut rng = Rng::new(0x0BAD_5EED);
        let (taus, _) = h3_case_sample(&mut rng, 40_000, 0.0, 1.0, &cfg);
        let mean = taus.iter().sum::<f64>() / taus.len() as f64;
        assert!(mean.abs() < 0.02, "clustered null mean tau {mean:.3}");
    }

    #[test]
    fn config_validation_rejects_undefined_family_and_icc_settings() {
        let invalid = [
            (
                PowerGateConfig {
                    h2h4_family_size: 0,
                    ..Default::default()
                },
                "h2h4_family_size",
            ),
            (
                PowerGateConfig {
                    h3_family_size: 0,
                    ..Default::default()
                },
                "h3_family_size",
            ),
            (
                PowerGateConfig {
                    h3_family_icc: 1.0,
                    ..Default::default()
                },
                "h3_family_icc",
            ),
        ];
        for (cfg, expected_field) in invalid {
            assert_eq!(cfg.validate().unwrap_err().field, expected_field);
        }
    }

    #[test]
    fn smallest_grid_n_requires_a_passing_null_cell_at_the_same_n() {
        let cell = |n_units: usize, effect: f64, power: f64, significance_rate: f64| PowerCell {
            n_units,
            effect,
            significance_rate,
            power,
            futility_rate: 0.0,
            mean_point_estimate: effect,
            replicates: 400,
        };
        let cfg = PowerGateConfig::default();
        let alternatives = [cell(20, 0.3, 0.9, 0.9), cell(40, 0.3, 0.85, 0.85)];
        let nulls = [cell(20, 0.0, 0.15, 0.15), cell(40, 0.0, 0.05, 0.05)];
        let selected = select_smallest_passing_grid_n(&alternatives, 0.3, &nulls, &cfg).unwrap();
        assert_eq!(selected.n_units, 40);
    }

    #[test]
    fn smallest_grid_n_reports_the_selected_cells_null_rate() {
        let cell = |n_units: usize, effect: f64, power: f64, significance_rate: f64| PowerCell {
            n_units,
            effect,
            significance_rate,
            power,
            futility_rate: 0.0,
            mean_point_estimate: effect,
            replicates: 400,
        };
        let cfg = PowerGateConfig::default();
        let alternatives = [cell(20, 0.3, 0.9, 0.9), cell(40, 0.3, 0.9, 0.9)];
        let nulls = [cell(20, 0.0, 0.04, 0.04), cell(40, 0.0, 0.2, 0.2)];
        let selected = select_smallest_passing_grid_n(&alternatives, 0.3, &nulls, &cfg).unwrap();
        assert!((selected.null_rate - 0.04).abs() < f64::EPSILON);
    }

    #[test]
    fn legacy_report_is_explicitly_nonpromotable_and_does_not_evaluate_current_hypotheses() {
        let cfg = PowerGateConfig {
            h1_episodes_grid: vec![40],
            h1_delta_grid: vec![0.0, 0.05],
            h2h4_tasks_grid: vec![20],
            h3_cases_grid: vec![20],
            n_boot: 100,
            replicates: 20,
            ..Default::default()
        };
        let r = run_legacy_sensitivity_calculation(&cfg).unwrap();
        assert_eq!(r.schema_version, "3.0");
        assert_eq!(
            r.artifact_id,
            "legacy_v10_7_endpoint_sensitivity_calculation_v1"
        );
        let endpoint_ids: Vec<&str> = r
            .legacy_endpoint_diagnostics
            .iter()
            .map(|diagnostic| diagnostic.endpoint_id.as_str())
            .collect();
        assert_eq!(
            endpoint_ids,
            [
                "legacy_v10_7_h1_incremental_auroc",
                "legacy_v10_7_h2_red_ablation_spearman",
                "legacy_v10_7_h3_case_kendall",
                "legacy_v10_7_h4_ssi_degradation_spearman",
            ]
        );
        assert!(r.h2[0].mean_point_estimate > 0.0);
        assert!(r.h4[0].mean_point_estimate < 0.0);
        assert_eq!(
            r.promotion_status,
            LegacySensitivityPromotionStatus::Nonpromotable
        );
        assert_eq!(
            r.current_hypothesis_gate_status,
            CurrentHypothesisGateStatus::NotEvaluated
        );
        assert_eq!(
            r.current_scientific_success_status,
            CurrentScientificSuccessStatus::NotEstablished
        );
        let serialized = serde_json::to_string(&r).expect("serialize legacy report");
        assert!(serialized.contains("\"promotion_status\":\"nonpromotable\""));
        assert!(serialized.contains("\"current_hypothesis_gate_status\":\"not_evaluated\""));
        assert!(serialized.contains("\"current_scientific_success_status\":\"not_established\""));
        assert!(serialized.contains("\"legacy_v10_7_h1_surface\""));
        assert!(!serialized.contains("\"h1\":["));
        assert!(!serialized.contains("\"idealized_sensitivity_gate_passed\""));
        assert!(!serialized.contains("\"capture_ready\""));
        let md = legacy_sensitivity_markdown(&r);
        assert!(md.contains("not capture requirements or guarantees"));
        assert!(md.contains("NONPROMOTABLE"));
        assert!(md.contains("Current EC1/H1–H4 gate: NOT EVALUATED"));
    }
}
