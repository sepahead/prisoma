# PID-VLA: Partial Information Decomposition for Vision-Language-Action Models

**Wibral-group shared-exclusions PID (I^sx_∩) for VLA diagnostics**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

PID-VLA is a research toolkit for analyzing **Vision-Language-Action (VLA)** robot policies using **Partial Information Decomposition (PID)**. It quantifies how visual and linguistic inputs contribute—redundantly, uniquely, or synergistically—to robot action outputs.

**Canonical Research Specification:** This project is governed by [`grandplan.md`](grandplan.md), which contains the complete theoretical, experimental, and implementation details.

### Key Research Questions (Hypotheses)

| Hypothesis | Description | Reference |
|------------|-------------|-----------|
| **H1** | Negative synergy indicates subadditive information (potential hallucination). | `grandplan.md` §1.2 |
| **H4** | PID signatures distinguish memorization from generalization. | `grandplan.md` §3.6.2 |
| **H5** | Compositional failure correlates with temporal synergy degradation. | `grandplan.md` §3.6.3 |
| **H7** | 3D object flow serves as an embodiment-agnostic integration diagnostic. | `grandplan.md` §3.6.6 |

## Quick Start

### Prerequisites

- **Rust** 1.75+ (via rustup or Nix)
- **Python** 3.11+
- **uv** (Python package manager)
- **just** (task runner)

### Installation

```bash
# Clone the repository
git clone https://github.com/your-org/pid-vla.git
cd pid-vla

# Option 1: Using Nix (recommended for reproducibility)
nix develop

# Option 2: Manual setup
cargo build
uv sync
```

### Running Experiment 0 (Validation Gate)

Before using PID-VLA on real VLA embeddings, you **MUST** run the validation suite (see `grandplan.md` §9.1):

```bash
# Run Rust-side validation tests
just exp0

# Run the full Experiment 0 binary
just exp0-bin

# CSV output for analysis
cargo run -p pid-core --bin exp0 -- --csv > exp0_results.csv
```

**GO/NO-GO Criteria (`grandplan.md` §9.1):**
*   **GO:** Error < 15% at d=256 after PCA, stable across seeds.
*   **PIVOT:** Error 15-30% at d=256; switch to discrete or CI-only screening.
*   **NO-GO:** Error > 30% at d=256; fundamental approach failure.

### Basic Usage (Rust)

```rust
use pid_core::{pid2_isx, MatRef, Pid2Config};

// Prepare your data: n samples × d dimensions
// OpenVLA standard dimensions (see EXPERIMENTS.md §3.1)
let n = 1000;
let d_vis = 1024; // Vision (SigLIP/DinoV2)
let d_lang = 4096; // Language (Llama 2 7B)
let d_act = 8;    // Action (7 joints + gripper)

// Dummy data for illustration
let vision_embeddings = vec![0.0; n * d_vis];
let language_embeddings = vec![0.0; n * d_lang];
let action_outputs = vec![0.0; n * d_act];

// Map VLA variables to PID Source/Target roles
let s1 = MatRef::new(&vision_embeddings, n, d_vis)?; // Source 1: Vision
let s2 = MatRef::new(&language_embeddings, n, d_lang)?; // Source 2: Language
let t = MatRef::new(&action_outputs, n, d_act)?; // Target: Action

let cfg = Pid2Config::default();
let result = pid2_isx(s1, s2, t, &cfg)?;

println!("Redundancy: {:.3} nats", result.redundancy);
println!("Unique Vision: {:.3} nats", result.unique_s1);
println!("Unique Language: {:.3} nats", result.unique_s2);
println!("Synergy: {:.3} nats", result.synergy);
```

## Architecture

### PID-Core Library

The `pid-core` crate implements the estimators defined in `grandplan.md`:

| Module | Description | Reference |
|--------|-------------|-----------|
| `ksg` | KSG mutual information estimator (Kraskov et al. 2004) | §8.1 |
| `isx` | Shared-exclusions redundancy I^sx_∩ (Ehrlich et al. 2024) | §2.2 |
| `pid2` | 2-source PID decomposition (Red, Unq1, Unq2, Syn) | §2.1 |
| `hierarchy` | Fast→slow hierarchical screening for many-source settings | §2.5.4 |
| `geometry` | Distance concentration and intrinsic dimension diagnostics | §16 |
| `hyperbolic` | Hyperbolic geometry utilities (Poincaré/Lorentz) | §16.7 |

### Information Flow

```
┌─────────────────┐     ┌─────────────────┐
│  Vision (V)     │     │  Language (L)   │
│ (n×1024)        │     │ (n×4096)        │
└────────┬────────┘     └────────┬────────┘
         │                       │
         ▼                       ▼
    ┌─────────┐             ┌─────────┐
    │   S1    │             │   S2    │
    └────┬────┘             └────┬────┘
         │                       │
         └───────────┬───────────┘
                     │
              ┌──────▼──────┐
              │  PID-Core   │
              │  Estimators │
              └──────┬──────┘
                     │
         ┌───────────┼───────────┐
         │           │           │
    ┌────▼────┐ ┌────▼────┐ ┌────▼────┐
    │ Red(V,  │ │ Unq(V)  │ │ Syn(V,  │
    │ L; A)   │ │         │ │ L; A)   │
    └─────────┘ └─────────┘ └─────────┘
```

## VLA Architecture Targets

See `grandplan.md` §7 for detailed analysis.

| VLA | Backbone | World Model (D) | Action Rep | Notes |
| --- | --- | --- | --- | --- |
| **OpenVLA** | Llama 2 7B | Implicit (Hidden states) | Discrete bins | Primary target; d=4096 |
| **DreamVLA** | GPT-2 var. | Explicit (<dream> tokens) | Diffusion | Ideal for V-D-A; dims unknown |
| **PixelVLA** | Llama 2 7B | Implicit + Pixel enc. | Continuous 7D | Multiscale visual features |
| **TraceVLA** | Llama 2 7B | Trace-augmented V | Discrete bins | Temporal history in V |

## Estimator Caveats

⚠️ **Read before using on VLA embeddings (`grandplan.md` v5.5 Warning):**

1. **Manifold Geometry:** The continuous I^sx_∩ estimator relies on Chebyshev (L∞) geometry. It **cannot** be applied directly to hyperbolic/Lorentz/manifold embeddings without mitigation. See `grandplan.md` §16.

2. **Flow-as-Bridge:** To sidestep manifold issues, `EXPERIMENTS.md` §7 uses 3D Object Flow (Euclidean) as the PID target rather than high-dimensional embeddings.

3. **Sample Size:** Theoretically, KSG requires $N \propto k^d$. In practice, `EXPERIMENTS.md` §4 validates performance at $N=1000$ for $d=64$, relying on the empirical validation in `grandplan.md` §8.6.

4. **i.i.d. Assumption:** VLA trajectories are autocorrelated. Use cross-trajectory sampling or large strides.

## References

**Core Methodology:**
- Makkeh, A., Gutknecht, A. J., & Wibral, M. (2021). Introducing a differentiable measure of pointwise shared information. *Phys Rev E*, 103:032149.
- Ehrlich, D. A., Schick-Poland, K., Makkeh, A., et al. (2024). Partial Information Decomposition for Continuous Variables based on Shared Exclusions. *Phys Rev E*, 110:014115.
- Gutknecht, A. J., et al. (2025). Shannon Invariants: A Scalable Approach to Information Decomposition. *arXiv:2504.15779*.

**Research Plan:**
- [grandplan.md](grandplan.md) - Full theoretical specification
- [EXPERIMENTS.md](EXPERIMENTS.md) - Detailed protocols
- [pidsplatspecs.md](pidsplatspecs.md) - Simulation environment spec

## Citation

```bibtex
@software{pid_vla,
  title = {PID-VLA: Partial Information Decomposition for Vision-Language-Action Models},
  year = {2026},
  url = {https://github.com/your-org/pid-vla}
}
```
