//! §14.8.3 simulation-based **power gate** for the H1–H4 primary endpoints.
//!
//! `grandplan.md` §14.8.3 makes a power analysis a *capture gate*: before the
//! first real capture (M5) is analyzed, a simulation-based power analysis must
//! run — with the **actual grouped/episode-level bootstrap** and the **correct
//! analysis unit per endpoint** (episodes for H1, *tasks* for H2/H4, *matched
//! cases* for H3; adding episodes never adds tasks, so an episode-only power
//! calculation is a category error for the correlational endpoints).
//!
//! Preregistered minimum effect sizes (§14.8.3 — commitments, not estimates):
//! - **H1**: incremental held-out episode-level ΔAUROC ≥ 0.05
//!   ({baselines + PID/CI} over {baselines alone}; paired episode-level
//!   bootstrap; success = one-sided significance AND point ≥ 0.05; futility =
//!   95% CI upper bound < 0.02).
//! - **H2/H4**: |Spearman ρ| ≥ 0.3 across tasks (families are *clusters*,
//!   handled by family-blocked bootstrap — never the analysis unit).
//! - **H3**: mean per-case Kendall τ ≥ 1/3 across ≥ 20 matched cases
//!   (family-blocked case-resampling bootstrap).
//!
//! Each simulator draws data at the preregistered minimum effect (and at a
//! null cell to verify the test's size), runs the *preregistered statistical
//! procedure itself* — not an analytic shortcut — and reports empirical power
//! per candidate capture size. The H1 feature model is binormal, so the
//! injected incremental effect is *exact by construction*:
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
    idx.sort_by(|&a, &b| scores[a].0.partial_cmp(&scores[b].0).unwrap());
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
    idx.sort_by(|&a, &b| xs[a].partial_cmp(&xs[b]).unwrap());
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
                .max_by(|&r1, &r2| a[r1][col].abs().partial_cmp(&a[r2][col].abs()).unwrap())
                .unwrap();
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
    /// Analysis units in the simulated capture (episodes / tasks / cases).
    pub n_units: usize,
    /// Injected true effect on the endpoint's own scale (ΔAUROC / ρ / mean τ).
    pub effect: f64,
    /// Fraction of replicates achieving one-sided directional significance.
    pub significance_rate: f64,
    /// Fraction achieving the full preregistered success criterion (for H1:
    /// significance AND point estimate ≥ the minimum effect; for H2–H4 equal
    /// to `significance_rate`).
    pub power: f64,
    /// H1 only: fraction of replicates declared futile (95% CI upper < 0.02).
    pub futility_rate: f64,
    /// Mean point estimate across replicates (calibration check).
    pub mean_point_estimate: f64,
    pub replicates: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerGateConfig {
    /// Candidate H1 capture sizes (episodes).
    pub h1_episodes_grid: Vec<usize>,
    /// H1 injected incremental ΔAUROC values (must include the preregistered
    /// minimum 0.05).
    pub h1_delta_grid: Vec<f64>,
    /// Standalone AUROC of the pooled baseline feature set.
    pub h1_baseline_auroc: f64,
    /// Episode-level failure prevalence.
    pub h1_failure_rate: f64,
    /// Fraction of episodes held out for the endpoint contrast.
    pub h1_heldout_frac: f64,
    /// Candidate H2/H4 capture sizes (tasks).
    pub h2h4_tasks_grid: Vec<usize>,
    /// Injected Spearman ρ (preregistered minimum 0.3).
    pub h2h4_rho: f64,
    /// Tasks per family (families are resampling blocks).
    pub h2h4_family_size: usize,
    /// Between-family random-effect SD on the outcome variable.
    pub h2h4_family_sd: f64,
    /// Candidate H3 capture sizes (matched cases).
    pub h3_cases_grid: Vec<usize>,
    /// Injected mean per-case Kendall τ (preregistered minimum 1/3).
    pub h3_mean_tau: f64,
    /// Cases per family (families are resampling blocks).
    pub h3_family_size: usize,
    /// Bootstrap resamples per replicate.
    pub n_boot: usize,
    /// Monte-Carlo replicates per grid cell.
    pub replicates: usize,
    /// One-sided significance level.
    pub alpha: f64,
    /// H1 preregistered minimum effect (success needs point ≥ this).
    pub h1_min_effect: f64,
    /// H1 futility bound (95% CI upper < this ⇒ futile).
    pub h1_futility_bound: f64,
    /// Target power the gate requires at the preregistered minimum effect.
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

/// Verdict for one endpoint: the smallest simulated capture size whose power
/// at the preregistered minimum effect reaches the target, if any.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EndpointVerdict {
    pub endpoint: String,
    pub unit: String,
    pub min_effect: f64,
    /// Smallest n on the grid with power ≥ target at the minimum effect.
    pub min_units_for_target_power: Option<usize>,
    /// Empirical one-sided type-I rate at the null cell for the largest n
    /// (should be ≤ alpha plus Monte-Carlo slack).
    pub null_significance_rate: f64,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerGateReport {
    pub config: PowerGateConfig,
    pub h1: Vec<PowerCell>,
    pub h2h4: Vec<PowerCell>,
    pub h2h4_null: Vec<PowerCell>,
    pub h3: Vec<PowerCell>,
    pub h3_null: Vec<PowerCell>,
    pub verdicts: Vec<EndpointVerdict>,
    /// True iff every covered endpoint has a feasible capture size on the grid
    /// and every null cell's size is within Monte-Carlo slack of alpha.
    pub passed: bool,
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
        deltas.sort_by(|a, b| a.partial_cmp(b).unwrap());
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

/// One H2/H4 replicate at the task level with family clustering.
fn h2h4_replicate(
    rng: &mut Rng,
    tasks: usize,
    rho: f64,
    cfg: &PowerGateConfig,
) -> Option<(f64, bool)> {
    // Gaussian copula: Pearson r giving Spearman ρ under bivariate normality.
    let r = 2.0 * (std::f64::consts::PI * rho / 6.0).sin();
    let n_fam = tasks.div_ceil(cfg.h2h4_family_size);
    let mut fam_of = Vec::with_capacity(tasks);
    let mut xs = Vec::with_capacity(tasks);
    let mut ys = Vec::with_capacity(tasks);
    for f in 0..n_fam {
        let u_f = rng.next_gaussian() * cfg.h2h4_family_sd;
        for _ in 0..cfg.h2h4_family_size {
            if xs.len() == tasks {
                break;
            }
            let z = rng.next_gaussian();
            let e = rng.next_gaussian();
            xs.push(z);
            ys.push(r * z + (1.0 - r * r).sqrt() * e + u_f);
            fam_of.push(f);
        }
    }
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
    boots.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let significant = percentile(&boots, cfg.alpha) > 0.0;
    Some((point, significant))
}

// ─────────────────────────────── H3 simulator ───────────────────────────────

/// Calibrate the rank-noise SD so that E[per-case τ] over 3-modality cases
/// equals `target_tau` (deterministic Monte-Carlo bisection).
fn h3_calibrate_sigma(target_tau: f64, seed: u64) -> f64 {
    let mean_tau_at = |sigma: f64| -> f64 {
        let mut rng = Rng::new(seed ^ 0xCA11_B4A7E);
        let m = 40_000;
        let mut acc = 0.0;
        for _ in 0..m {
            let unq = [1.0, 2.0, 3.0];
            let eff: Vec<f64> = unq
                .iter()
                .map(|u| u + sigma * rng.next_gaussian())
                .collect();
            acc += kendall_tau(&unq, &eff);
        }
        acc / m as f64
    };
    let (mut lo, mut hi) = (0.05_f64, 20.0_f64);
    for _ in 0..40 {
        let mid = 0.5 * (lo + hi);
        if mean_tau_at(mid) > target_tau {
            lo = mid; // more noise still leaves τ above target → raise noise
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// One H3 replicate: mean per-case Kendall τ with family-blocked
/// case-resampling bootstrap.
fn h3_replicate(
    rng: &mut Rng,
    cases: usize,
    sigma: f64,
    cfg: &PowerGateConfig,
) -> Option<(f64, bool)> {
    let n_fam = cases.div_ceil(cfg.h3_family_size);
    let mut taus = Vec::with_capacity(cases);
    let mut fam_of = Vec::with_capacity(cases);
    for f in 0..n_fam {
        for _ in 0..cfg.h3_family_size {
            if taus.len() == cases {
                break;
            }
            let unq = [1.0, 2.0, 3.0];
            let eff: Vec<f64> = unq
                .iter()
                .map(|u| u + sigma * rng.next_gaussian())
                .collect();
            taus.push(kendall_tau(&unq, &eff));
            fam_of.push(f);
        }
    }
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
    boots.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let significant = percentile(&boots, cfg.alpha) > 0.0;
    Some((point, significant))
}

// ─────────────────────────────── the gate itself ────────────────────────────

/// Run the §14.8.3 power gate. Deterministic for a fixed config.
pub fn run_power_gate(cfg: &PowerGateConfig) -> PowerGateReport {
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
            h1.push(PowerCell {
                n_units: n,
                effect: delta,
                significance_rate: sig as f64 / valid.max(1) as f64,
                power: succ as f64 / valid.max(1) as f64,
                futility_rate: fut as f64 / valid.max(1) as f64,
                mean_point_estimate: points / valid.max(1) as f64,
                replicates: valid,
            });
        }
    }

    // H2/H4 surface over tasks at the preregistered ρ, plus a null row per n.
    let run_h2h4 = |rho: f64, tag: u64| -> Vec<PowerCell> {
        cfg.h2h4_tasks_grid
            .iter()
            .enumerate()
            .map(|(gi, &n)| {
                let mut sig = 0usize;
                let mut points = 0.0;
                let mut valid = 0usize;
                for rep in 0..cfg.replicates {
                    let mut rng = Rng::new(cfg.seed ^ tag ^ ((gi as u64) << 32) ^ rep as u64);
                    if let Some((point, s)) = h2h4_replicate(&mut rng, n, rho, cfg) {
                        valid += 1;
                        points += point;
                        sig += s as usize;
                    }
                }
                let rate = sig as f64 / valid.max(1) as f64;
                PowerCell {
                    n_units: n,
                    effect: rho,
                    significance_rate: rate,
                    power: rate,
                    futility_rate: 0.0,
                    mean_point_estimate: points / valid.max(1) as f64,
                    replicates: valid,
                }
            })
            .collect()
    };
    let h2h4 = run_h2h4(cfg.h2h4_rho, 0x22_0000);
    let h2h4_null = run_h2h4(0.0, 0x22_1111);

    // H3 surface over matched cases at mean τ = 1/3, plus null.
    let sigma_alt = h3_calibrate_sigma(cfg.h3_mean_tau, cfg.seed);
    let run_h3 = |sigma: f64, effect: f64, tag: u64| -> Vec<PowerCell> {
        cfg.h3_cases_grid
            .iter()
            .enumerate()
            .map(|(gi, &n)| {
                let mut sig = 0usize;
                let mut points = 0.0;
                let mut valid = 0usize;
                for rep in 0..cfg.replicates {
                    let mut rng = Rng::new(cfg.seed ^ tag ^ ((gi as u64) << 32) ^ rep as u64);
                    if let Some((point, s)) = h3_replicate(&mut rng, n, sigma, cfg) {
                        valid += 1;
                        points += point;
                        sig += s as usize;
                    }
                }
                let rate = sig as f64 / valid.max(1) as f64;
                PowerCell {
                    n_units: n,
                    effect,
                    significance_rate: rate,
                    power: rate,
                    futility_rate: 0.0,
                    mean_point_estimate: points / valid.max(1) as f64,
                    replicates: valid,
                }
            })
            .collect()
    };
    let h3 = run_h3(sigma_alt, cfg.h3_mean_tau, 0x33_0000);
    // Null: effects i.i.d. of the unq ordering ⇒ enormous noise.
    let h3_null = run_h3(1.0e6, 0.0, 0x33_1111);

    // Verdicts. Monte-Carlo slack on the null size check: 3 SEs of a binomial
    // at alpha with `replicates` draws.
    let slack = 3.0 * (cfg.alpha * (1.0 - cfg.alpha) / cfg.replicates as f64).sqrt();
    let min_units = |cells: &[PowerCell], effect: f64| -> Option<usize> {
        let mut ns: Vec<usize> = cells
            .iter()
            .filter(|c| (c.effect - effect).abs() < 1e-12 && c.power >= cfg.target_power)
            .map(|c| c.n_units)
            .collect();
        ns.sort_unstable();
        ns.first().copied()
    };
    let h1_null_rate = h1
        .iter()
        .filter(|c| c.effect == 0.0)
        .max_by_key(|c| c.n_units)
        .map(|c| c.significance_rate)
        .unwrap_or(f64::NAN);
    let h2h4_null_rate = h2h4_null
        .iter()
        .max_by_key(|c| c.n_units)
        .map(|c| c.significance_rate)
        .unwrap_or(f64::NAN);
    let h3_null_rate = h3_null
        .iter()
        .max_by_key(|c| c.n_units)
        .map(|c| c.significance_rate)
        .unwrap_or(f64::NAN);

    let mk = |endpoint: &str, unit: &str, min_effect: f64, mu: Option<usize>, null_rate: f64| {
        EndpointVerdict {
            endpoint: endpoint.to_string(),
            unit: unit.to_string(),
            min_effect,
            min_units_for_target_power: mu,
            null_significance_rate: null_rate,
            passed: mu.is_some() && null_rate <= cfg.alpha + slack,
        }
    };
    let verdicts = vec![
        mk(
            "H1 incremental ΔAUROC",
            "episodes",
            cfg.h1_min_effect,
            min_units(&h1, cfg.h1_min_effect),
            h1_null_rate,
        ),
        mk(
            "H2/H4 Spearman ρ",
            "tasks",
            cfg.h2h4_rho,
            min_units(&h2h4, cfg.h2h4_rho),
            h2h4_null_rate,
        ),
        mk(
            "H3 mean per-case Kendall τ",
            "matched cases",
            cfg.h3_mean_tau,
            min_units(&h3, cfg.h3_mean_tau),
            h3_null_rate,
        ),
    ];
    let passed = verdicts.iter().all(|v| v.passed);
    PowerGateReport {
        config: cfg.clone(),
        h1,
        h2h4,
        h2h4_null,
        h3,
        h3_null,
        verdicts,
        passed,
    }
}

/// Render the report as a compact markdown document (for `docs/`).
pub fn power_gate_markdown(r: &PowerGateReport) -> String {
    let mut s = String::new();
    s.push_str("# §14.8.3 Power Gate — simulation-based power analysis (H1–H4 primaries)\n\n");
    s.push_str(&format!(
        "Replicates/cell: {} · bootstrap: {} · one-sided α = {} · target power = {}\n\n",
        r.config.replicates, r.config.n_boot, r.config.alpha, r.config.target_power
    ));
    s.push_str("## Verdicts\n\n| Endpoint | Unit | Min effect | Min units for power ≥ target | Null sig. rate | Passed |\n|---|---|---|---|---|---|\n");
    for v in &r.verdicts {
        s.push_str(&format!(
            "| {} | {} | {:.3} | {} | {:.3} | {} |\n",
            v.endpoint,
            v.unit,
            v.min_effect,
            v.min_units_for_target_power
                .map(|n| n.to_string())
                .unwrap_or_else(|| "NOT REACHED ON GRID".to_string()),
            v.null_significance_rate,
            if v.passed { "✅" } else { "❌" }
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
    table(&mut s, "H2/H4 (tasks at ρ = min effect)", &r.h2h4, false);
    table(
        &mut s,
        "H2/H4 null (ρ = 0; size check)",
        &r.h2h4_null,
        false,
    );
    table(&mut s, "H3 (matched cases at mean τ = 1/3)", &r.h3, false);
    table(&mut s, "H3 null (size check)", &r.h3_null, false);
    s.push_str(&format!(
        "\n**Gate: {}**\n",
        if r.passed { "PASSED" } else { "NOT PASSED" }
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
    fn h3_noise_calibration_hits_target_tau() {
        let sigma = h3_calibrate_sigma(1.0 / 3.0, 7);
        let mut rng = Rng::new(99);
        let m = 20_000;
        let mut acc = 0.0;
        for _ in 0..m {
            let unq = [1.0, 2.0, 3.0];
            let eff: Vec<f64> = unq
                .iter()
                .map(|u| u + sigma * rng.next_gaussian())
                .collect();
            acc += kendall_tau(&unq, &eff);
        }
        let mean = acc / m as f64;
        assert!(
            (mean - 1.0 / 3.0).abs() < 0.03,
            "calibrated mean τ {mean:.3}"
        );
    }

    #[test]
    fn power_gate_runs_and_is_sane_on_a_small_grid() {
        let cfg = PowerGateConfig {
            h1_episodes_grid: vec![40, 240],
            h1_delta_grid: vec![0.0, 0.08],
            h2h4_tasks_grid: vec![8, 48],
            h3_cases_grid: vec![20, 60],
            n_boot: 200,
            replicates: 60,
            ..Default::default()
        };
        let r = run_power_gate(&cfg);
        let cell = |n: usize, e: f64| {
            r.h1.iter()
                .find(|c| c.n_units == n && (c.effect - e).abs() < 1e-12)
                .unwrap()
                .clone()
        };
        // Power grows with n and with effect.
        assert!(cell(240, 0.08).significance_rate > cell(40, 0.08).significance_rate - 0.05);
        assert!(cell(240, 0.08).significance_rate > cell(240, 0.0).significance_rate);
        // Null cells: one-sided size within generous Monte-Carlo slack.
        assert!(cell(240, 0.0).significance_rate <= 0.05 + 3.0 * (0.05f64 * 0.95 / 60.0).sqrt());
        let h2_large = r.h2h4.iter().find(|c| c.n_units == 48).unwrap();
        let h2_small = r.h2h4.iter().find(|c| c.n_units == 8).unwrap();
        assert!(h2_large.power >= h2_small.power - 0.05);
        // Markdown renders.
        let md = power_gate_markdown(&r);
        assert!(md.contains("Verdicts") && md.contains("H3"));
    }
}
