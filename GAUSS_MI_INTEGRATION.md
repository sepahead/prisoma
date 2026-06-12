# GauSS-MI Uncertainty Integration Specification

> **Documentation Cross-Reference**:
> - `grandplan.md` — Master plan
> - `pidsplatspecs.md` — Simulation environment (PID-Splat)
> - `DIAGRAMS.md` — System diagrams (GauSS‑MI overview)
> - `EXPERIMENTS.md` — Capture protocols + quality gates
> - `pid-core` — Rust implementation context

**Docset alignment:** v10.2 (this is an optional module spec; not implemented in this repo today)
**Spec version:** 1.0
**Date:** 2026-01-03
**Status:** Specification (Pre-Implementation)

**Docset-wide final solution:** `grandplan.md` §A.8 is the decision record. GauSS-MI is an optional confound-control module after the canonical run-log/replay/Rerun path exists; view-selection decisions must be Agent Bridge events and must not bypass replay/provenance.

---

## 1. Executive Summary

This specification details how to integrate **GauSS-MI (Gaussian Splatting Shannon Mutual Information)** uncertainty quantification with the `pid-core` library's PID estimators. The integration enables:

1. **Per-Gaussian uncertainty weighting** in PID computations
2. **Uncertainty-aware MI estimation** that down-weights unreliable visual features
3. **Active view selection** for improving PID estimate quality
4. **Diagnostic metrics** for identifying when visual uncertainty corrupts PID analysis
5. **Agent-native execution (planned):** expose view selection and uncertainty queries via the same Agent Bridge control plane as the GUI, so automated tools can request candidate viewpoints and the decisions are logged for replay

### Key Innovation
Standard PID estimators treat all samples equally. GauSS-MI integration allows uncertainty-weighted PID estimation where:
- High-uncertainty Gaussians contribute less to information measures
- Visual features from well-reconstructed regions dominate the analysis
- PID estimates include confidence intervals based on reconstruction quality

**Rigour note:** weighted kNN/KSG-style estimators and weighted PID atoms require a dedicated validation gate (analogous to Experiment 0). Treat all weighted-estimator outputs as *experimental* until they are validated on synthetic ground truth cases and stress-tested for bias/variance.

---

## 2. Background: GauSS-MI Framework

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

## 3. Integration Architecture

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
│   ├── gauss_mi/                 # NEW: GauSS-MI integration
│   │   ├── mod.rs                # Module exports
│   │   ├── uncertainty.rs        # Per-Gaussian uncertainty model
│   │   ├── weighted_ksg.rs       # Uncertainty-weighted KSG
│   │   ├── weighted_isx.rs       # Uncertainty-weighted ISX
│   │   ├── view_selection.rs     # Active view selection for PID
│   │   ├── diagnostics.rs        # Quality diagnostics
│   │   └── integration.rs        # High-level API
```

### 3.2 Data Flow
```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                         GauSS-MI + PID Pipeline                             │
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
│  │ Observations │     │ Extraction       │     │ Weighted Features  │      │
│  └──────────────┘     │ per Gaussian     │     │ V_weighted         │      │
│                       └──────────────────┘     └─────────┬──────────┘      │
│                                                          │                  │
│                                                          ▼                  │
│  ┌──────────────┐                              ┌────────────────────┐      │
│  │ Other        │─────────────────────────────►│ Weighted PID       │      │
│  │ Variables    │                              │ Estimator          │      │
│  │ (L, D, A)    │                              │                    │      │
│  └──────────────┘                              │ - Weighted KSG MI  │      │
│                                                │ - Weighted ISX Red │      │
│                                                │ - Confidence Ints  │      │
│                                                └─────────┬──────────┘      │
│                                                          │                  │
│                                                          ▼                  │
│                                                ┌────────────────────┐      │
│                                                │ PID Results +      │      │
│                                                │ Quality Metrics    │      │
│                                                │                    │      │
│                                                │ - Synergy ± CI     │      │
│                                                │ - Redundancy ± CI  │      │
│                                                │ - Unique ± CI      │      │
│                                                │ - Effective N      │      │
│                                                └────────────────────┘      │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Core Data Structures

### 4.1 Gaussian Uncertainty Representation
```rust
/// Per-Gaussian uncertainty from GauSS-MI
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
    /// Compute weight for PID estimation (higher weight = more certain)
    pub fn pid_weight(&self) -> f64 {
        // Inverse of composite uncertainty, normalized
        // w_i = 1 / (1 + σ²_i)
        1.0 / (1.0 + self.composite_score)
    }
    
    /// Check if Gaussian is reliable enough for PID
    pub fn is_reliable(&self, threshold: f64) -> bool {
        self.composite_score < threshold && self.observation_count >= 3
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

### 4.3 Weighted Sample Representation
```rust
/// A sample with associated uncertainty weight for PID estimation
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
 
/// Collection of weighted samples for PID estimation
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

## 5. Uncertainty-Weighted KSG Estimator

### 5.1 Weighted KSG Algorithm
The standard KSG estimator computes:
$$ I(X;Y) = \psi(k) + \psi(N) - \langle \psi(n_x + 1) + \psi(n_y + 1) \rangle $$

The weighted version modifies this to:
$$ I_w(X;Y) = \psi(k) + \psi(N_{eff}) - \frac{\langle w_i \cdot [\psi(n_x^w + 1) + \psi(n_y^w + 1)] \rangle}{\langle w_i \rangle} $$

Where:
- $N_{eff} = (\sum w_i)^2 / \sum w_i^2$ is effective sample size
- $n_x^w, n_y^w$ are weighted neighbor counts
- $w_i$ are sample weights from GauSS-MI uncertainty

### 5.2 Implementation Strategy
```rust
/// Configuration for uncertainty-weighted KSG estimator
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
    // 5. Compute bootstrap confidence intervals
}
```

---

## 6. Next Steps

1.  **Dependencies**: Add `gauss-mi-core` (hypothetical crate) or implement uncertainty logic in `pid-core`.
2.  **Validation**: Test weighted estimator against analytic ground truth for Gaussian channels with known heteroscedastic noise.
3.  **Integration**: Connect to SparkJS renderer to feed real-time uncertainty estimates into the PID loop.
