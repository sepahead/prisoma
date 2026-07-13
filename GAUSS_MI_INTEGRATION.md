# Reconstruction-Quality Study and Rejected GauSS-MI Weighting Record

> **Documentation Cross-Reference**:
> - `grandplan.md` — Master plan
> - `pidsplatspecs.md` — Simulation environment (PID-Splat)
> - `DIAGRAMS.md` — System diagrams (GauSS‑MI overview)
> - `EXPERIMENTS.md` — Capture protocols + quality gates
> - `pid-rs/crates/pid-core` (submodule) — Rust implementation context (estimator changes land upstream in pid-rs, then the submodule is bumped; estimator code is never added to this repo directly)

**Docset alignment:** docset v12.5 (optional E1 covariate/view-study design; the weighted-PID sketch is quarantined E0, not an estimator interface or direct ecosystem edge)
**Spec version:** 1.1
**Originally proposed:** 2026-01-03
**Last reconciled:** 2026-07-13
**Status:** Specification (Pre-Implementation)

**Docset-wide final solution:** `grandplan.md` §16 is the decision log. A reconstruction-quality
covariate/view study is optional E1 work after the canonical run-log/replay/Rerun path exists; the
weighted-PID sketch is E0 and off-path. View-selection decisions must be Agent Bridge events and
must not bypass replay/provenance.

---

## 1. Executive Summary

This document is a **prospective research sketch**, not an available integration. It separates two
ideas that earlier wording blurred:

1. **Admissible E1 study design:** measure reconstruction quality/uncertainty and use it as a
   prespecified nuisance covariate, stratifier, exclusion sensitivity, or separately validated
   active-view objective. Any view-selection action would pass through the Agent Bridge and run log.
2. **Quarantined E0 estimator sketch:** uncertainty-weighted KSG/MI/PID. No population functional,
   importance-sampling law, neighbor-mass estimator, bias/consistency result, calibrated interval,
   or oracle evidence currently makes that sketch a scientific estimator. It must not be added to
   `pid-rs` or used to interpret atoms as written below.

### Safe research question
The safe near-term question is whether a measured reconstruction-quality variable explains or
stratifies diagnostic performance without changing the information estimand. Down-weighting samples
would instead change the sampling law and possibly the target functional; “more certain points count
more” is intuition, not a derivation. Reconstruction uncertainty also cannot be substituted for
estimator uncertainty or turned into a confidence interval.

**Fail-closed rule:** the weighted kNN/KSG and PID material in Sections 4–5 is retained only to make
the rejected heuristic auditable. It is pseudocode, not an API or implementation plan. Promotion
requires a named population functional, a derived estimator, support/positivity conditions, separate
measure and estimator gates, analytic or independently computed oracles, coverage and false-
eligibility tests, and a preregistered application regime under `grandplan.md` §7.

---

## 2. Prospective reconstruction-quality measurement sketch

### 2.1 Per-Gaussian Uncertainty Model
This spec proposes assigning uncertainty to each 3D Gaussian in a scene reconstruction by analyzing residual image loss:

$$ \sigma^2_i = \text{Var}[L(I_{obs}, I_{rendered}) \mid G_i \text{ contributes to pixel}] $$

Where:
- $\sigma^2_i$: uncertainty variance for Gaussian $i$
- $L(\cdot,\cdot)$: photometric loss (e.g., L1 or SSIM)
- $G_i$: the $i$-th Gaussian splat
- $I_{obs}, I_{rendered}$: observed and rendered images

### 2.2 Shannon Mutual Information for View Selection
This spec proposes using expected information gain for candidate viewpoints:

$$ MI(\Theta; Y_v) = H(\Theta) - H(\Theta \mid Y_v) $$

Where:
- $\Theta$: parameters of the Gaussian reconstruction (positions, covariances, colors)
- $Y_v$: observation from viewpoint $v$
- $H(\cdot)$: Shannon entropy

This identity is only a target definition. It does not provide a posterior over $\Theta$, a
predictive observation law for $Y_v$, or a usable estimator; those must be specified and validated
before an active-view policy exists.

### 2.3 Probabilistic Gaussian Representation
Each Gaussian is modeled as:
```rust
G_i = {
    μ_i ∈ R³,           # position (mean)
    Σ_i ∈ R³ˣ³,         # covariance (shape)
    c_i ∈ R^48,         # spherical harmonics (color)
    α_i ∈ [0,1],        # opacity
    σ²_μ_i ∈ R,         # position uncertainty
    σ²_Σ_i ∈ R,         # shape uncertainty  
    σ²_c_i ∈ R,         # color uncertainty
}
```

---

## 3. Prospective study architecture

### 3.1 Module Structure
```text
pid-core/
├── src/
│   ├── lib.rs
│   ├── ksg.rs                    # Existing KSG estimator
│   ├── isx.rs                    # Existing ISX redundancy
│   ├── pid2.rs                   # Existing 2-source PID
│   ├── pid3.rs                   # Existing 3-source PID
│   │
│   ├── gauss_mi/                 # HYPOTHETICAL ONLY; no such upstream module exists
│   │   ├── mod.rs                # Module exports
│   │   ├── uncertainty.rs        # Per-Gaussian uncertainty model
│   │   ├── weighted_ksg.rs       # QUARANTINED sketch; do not implement as written
│   │   ├── weighted_isx.rs       # QUARANTINED sketch; estimand not defined
│   │   ├── view_selection.rs     # Separately validated active-view study
│   │   ├── diagnostics.rs        # Quality diagnostics
│   │   └── integration.rs        # Prospective orchestration sketch
```

### 3.2 Data Flow
```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                Prospective reconstruction-quality study                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────┐     ┌──────────────────┐     ┌────────────────────┐      │
│  │ 3DGS Scene   │────►│ GauSS-MI         │────►│ Per-Gaussian       │      │
│  │ Reconstruction│     │ Uncertainty      │     │ Uncertainties      │      │
│  └──────────────┘     │ Estimation       │     │ {σ²_i}             │      │
│                       └──────────────────┘     └─────────┬──────────┘      │
│                                                          │                  │
│                                                          ▼                  │
│  ┌──────────────┐     ┌──────────────────┐     ┌────────────────────┐      │
│  │ Visual       │────►│ Feature          │────►│ Uncertainty-       │      │
│  │ Observations │     │ Extraction       │     │ Quality covariate  │      │
│  └──────────────┘     │ per Gaussian     │     │ quality_feature    │      │
│                       └──────────────────┘     └─────────┬──────────┘      │
│                                                          │                  │
│                                                          ▼                  │
│  ┌──────────────┐                              ┌────────────────────┐      │
│  │ Other        │─────────────────────────────►│ Admissible output  │      │
│  │ Variables    │                              │ quality metrics    │      │
│  │ (L, D, A)    │                              │                    │      │
│  └──────────────┘                              │ - quality strata   │      │
│                                                │ - covariate checks │      │
│                                                │ - sensitivity only │      │
│                                                │ - no PID CI claim  │      │
│                                                └────────────────────┘      │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │ ISOLATED REJECTED BRANCH — no input or output edge                  │  │
│  │ GauSS-MI-weighted KSG/PID sketch: no derivation, estimand, or CI    │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

The admissible path uses reconstruction quality only as a covariate, stratum, or sensitivity
diagnostic. The rejected weighting sketch is deliberately disconnected from that path.

---

## 4. Prospective pseudocode (not a shipped API)

### 4.1 Gaussian Uncertainty Representation
```rust
/// PROSPECTIVE PSEUDOCODE — not present in pid-rs or Prisoma.
/// Per-Gaussian reconstruction-quality proposal.
#[derive(Debug, Clone)]
pub struct GaussianUncertainty {
    /// Index of the Gaussian in the scene
    pub gaussian_idx: usize,
    
    /// Position uncertainty variance (meters²)
    pub position_variance: f64,
    
    /// Shape (covariance) uncertainty 
    pub shape_variance: f64,
    
    /// Color uncertainty (SH coefficient variance)
    pub color_variance: f64,
    
    /// Composite uncertainty score (normalized 0-1, lower = more certain)
    pub composite_score: f64,
    
    /// Number of observations contributing to this Gaussian
    pub observation_count: u32,
    
    /// Confidence in the uncertainty estimate itself
    pub meta_confidence: f64,
}
 
impl GaussianUncertainty {
    /// A prespecified reconstruction-quality sensitivity screen.
    /// This does not define an information-estimator weight.
    pub fn is_quality_eligible(&self, threshold: f64, minimum_observations: u32) -> bool {
        self.composite_score < threshold && self.observation_count >= minimum_observations
    }
}
```

### 4.2 Scene Uncertainty Map
```rust
/// Uncertainty information for entire 3DGS scene
#[derive(Debug, Clone)]
pub struct SceneUncertaintyMap {
    /// Per-Gaussian uncertainties
    pub gaussians: Vec<GaussianUncertainty>,
    
    /// Global scene uncertainty statistics
    pub mean_uncertainty: f64,
    pub median_uncertainty: f64,
    pub uncertainty_std: f64,
    
    /// Spatial uncertainty distribution
    pub spatial_grid: Option<SpatialUncertaintyGrid>,
    
    /// Timestamp of uncertainty computation
    pub computed_at: u64,
    
    /// Source viewpoints used for uncertainty estimation
    pub source_viewpoints: Vec<ViewpointInfo>,
}
 
/// Spatial grid for fast uncertainty lookup
#[derive(Debug, Clone)]
pub struct SpatialUncertaintyGrid {
    /// Grid resolution
    pub resolution: [usize; 3],
    
    /// Grid bounds (min, max) in world coordinates
    pub bounds_min: [f64; 3],
    pub bounds_max: [f64; 3],
    
    /// Per-cell aggregated uncertainty
    pub cells: Vec<f64>,
}
```

### 4.3 Rejected weighted-sample sketch

The following types record what the earlier proposal assumed. They have no approved sampling-law
semantics and must not be treated as a `pid-core` design.

```rust
/// REJECTED PSEUDOCODE — `weight` has no derived PID estimand.
#[derive(Debug, Clone)]
pub struct WeightedSample {
    /// Feature vector
    pub features: Vec<f64>,
    
    /// Uncertainty weight (0-1, higher = more reliable)
    pub weight: f64,
    
    /// Source Gaussian indices contributing to this sample
    pub source_gaussians: Vec<usize>,
    
    /// Spatial position (for spatial uncertainty lookup)
    pub position: Option<[f64; 3]>,
}
 
/// REJECTED PSEUDOCODE — effective N does not repair the missing estimand.
#[derive(Debug, Clone)]
pub struct WeightedSampleSet {
    pub samples: Vec<WeightedSample>,
    pub dim: usize,
    
    /// Effective sample size accounting for weights
    /// N_eff = (Σw_i)² / Σw_i²
    pub effective_n: f64,
}
```

---

## 5. Quarantined weighted-KSG sketch — do not implement as written

### 5.1 Rejected heuristic ansatz
The standard KSG estimator computes:
$$ I(X;Y) = \psi(k) + \psi(N) - \langle \psi(n_x + 1) + \psi(n_y + 1) \rangle $$

The earlier draft proposed this expression:
$$ I_w(X;Y) = \psi(k) + \psi(N_{eff}) - \frac{\langle w_i \cdot [\psi(n_x^w + 1) + \psi(n_y^w + 1)] \rangle}{\langle w_i \rangle} $$

Where:
- $N_{eff} = (\sum w_i)^2 / \sum w_i^2$ is effective sample size
- $n_x^w, n_y^w$ are weighted neighbor counts
- $w_i$ are sample weights from GauSS-MI uncertainty

**Status: rejected heuristic ansatz, not a derived estimator.** Substituting $\psi(N_{eff})$ for
$\psi(N)$ has no derivation (KSG's neighbor-count terms rest on a local probability-mass argument
that arbitrary reliability weighting breaks), $n_x^w$ is undefined, and no bias, consistency,
support, or target-measure analysis exists. A synthetic smoke cannot repair a missing estimand. Do
not return a number from this formula.

### 5.2 Non-implementation record
```rust
/// REJECTED PSEUDOCODE — retained to document the non-implementation boundary.
#[derive(Debug, Clone)]
pub struct WeightedKsgConfig {
    /// Base KSG configuration
    pub base: KsgConfig,
    /// Minimum weight to include sample (0-1)
    pub min_weight: f64,
    /// Whether to use weighted neighbor counting
    pub weighted_neighbors: bool,
    /// Method for combining weights in distance computation
    pub weight_combination: WeightCombination,
}

pub fn weighted_ksg_mi(
    x: &WeightedSampleSet,
    y: &WeightedSampleSet,
    cfg: &WeightedKsgConfig,
) -> PidResult<WeightedMIResult> {
    // 1. Filter samples by min_weight
    // 2. Compute effective sample size N_eff
    // 3. For each sample i:
    //    a. Find k-th nearest neighbor in joint space using metric
    //    b. Count weighted neighbors in marginal spaces (nx, ny) within radius
    //    c. Compute local MI contribution weighted by w_i
    // 4. Normalize by total weight
    // 5. STOP: no derived estimand/estimator or calibrated interval exists.
}
```

---

## 6. Promotion sequence

1. **Use the safe baseline first:** define a reconstruction-quality measurement contract, validate it
   independently of PID, log the artifact in the canonical run log, and analyze it as a nuisance
   covariate/stratifier or exclusion sensitivity. Visualize through Rerun; keep SparkJS deferred.
2. **Define the scientific target before code:** state whether weighting changes the population law,
   targets an importance-weighted distribution, handles measurement error, or estimates another
   explicit functional. Specify positivity, dependence, and missingness assumptions.
3. **Derive and independently review an estimator:** only then propose upstream `pid-rs` code. A
   hypothetical `gauss-mi-core` dependency is not approved merely because a crate could be named.
4. **Run separate gates:** analytic/independent oracles, nulls, heteroscedastic stress tests,
   bias/variance, interval coverage, abstention sensitivity/specificity, and the full population,
   measure, estimator, and application verdicts.
5. **Promote only after evidence:** until those steps pass, weighted MI/PID remains E0 and off the
   thesis path; active view selection remains a separate E1 study whose actions use the Agent Bridge.
