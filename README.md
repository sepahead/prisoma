# PID-VLA: Partial Information Decomposition for Vision-Language-Action Models

**Wibral-group shared-exclusions PID (I^sx_∩) for VLA diagnostics**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

PID-VLA is a research toolkit for analyzing **Vision-Language-Action (VLA)** robot policies using **Partial Information Decomposition (PID)**. It quantifies how visual and linguistic inputs contribute—redundantly, uniquely, or synergistically—to robot action outputs.

**Canonical Research Specification:** This project is governed by [`grandplan.md`](grandplan.md), which contains the complete theoretical, experimental, and implementation details. All major architectural decisions should be cross-referenced against that document.

### Key Research Questions

1. **Redundancy**: What information about actions is available from *both* vision and language?
2. **Unique Vision**: What action information is available *only* from vision?
3. **Unique Language**: What action information is available *only* from language?
4. **Synergy**: What action information emerges *only* from the combination of vision and language?

### Core Components

| Component | Description |
|-----------|-------------|
| `pid-core` | Rust library implementing KSG mutual information and I^sx_∩ PID estimators |
| Simulation | Headless Gazebo environment for collecting VLA trajectory data |
| Visualization | Tauri + SparkJS frontend for real-time PID visualization |
| Analysis | Python scripts for statistical analysis and report generation |

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

### Basic Usage

```rust
use pid_core::{pid2_isx, MatRef, Pid2Config};

// Prepare your data: n samples × d dimensions
let vision_embeddings: Vec<f64> = /* ... */;
let language_embeddings: Vec<f64> = /* ... */;
let action_outputs: Vec<f64> = /* ... */;

let n = 1000; // number of samples
let d_vis = 768; // vision embedding dimension
let d_lang = 512; // language embedding dimension
let d_act = 7; // action dimension

let s1 = MatRef::new(&vision_embeddings, n, d_vis)?;
let s2 = MatRef::new(&language_embeddings, n, d_lang)?;
let t = MatRef::new(&action_outputs, n, d_act)?;

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
| `pid3` | Full 3-source SxPID with 18 atoms and Möbius inversion | §5.3 |
| `hierarchy` | Fast→slow hierarchical screening for many-source settings | §2.5.4 |
| `geometry` | Distance concentration and intrinsic dimension diagnostics | §16 |
| `preprocess` | Standardization, PCA, hash projection | §8.2 |
| `hyperbolic` | Hyperbolic geometry utilities (Poincaré/Lorentz) | §16.7 |

### Information Flow

```
┌─────────────────┐     ┌─────────────────┐
│  Vision Encoder │     │ Language Encoder│
│   (ViT, CLIP)   │     │  (T5, LLaMA)    │
└────────┬────────┘     └────────┬────────┘
         │                       │
         ▼                       ▼
    ┌─────────┐             ┌─────────┐
    │   S1    │             │   S2    │
    │ (n×d_v) │             │ (n×d_l) │
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
    │ Red(S1, │ │ Unq(S1) │ │ Syn(S1, │
    │ S2; T)  │ │ Unq(S2) │ │ S2; T)  │
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

## Project Structure

```
pid_vla/
├── crates/
│   └── pid-core/           # Core Rust library
│       ├── src/
│       │   ├── lib.rs      # Public API
│       │   ├── ksg.rs      # KSG MI estimator
│       │   ├── isx.rs      # I^sx_∩ redundancy
│       │   ├── pid2.rs     # 2-source PID
│       │   ├── pid3.rs     # 3-source SxPID
│       │   ├── hierarchy.rs # Hierarchical screening
│       │   ├── geometry.rs  # Geometric diagnostics
│       │   ├── preprocess.rs # Data preprocessing
│       │   └── bin/
│       │       └── exp0.rs  # Experiment 0 validation
│       └── tests/          # Integration tests
├── grandplan.md            # Comprehensive project specification
├── AGENTS.md               # Step-by-step implementation plan
├── Cargo.toml              # Workspace configuration
├── pyproject.toml          # Python dependencies
├── flake.nix               # Nix development environment
└── justfile                # Task runner commands
```

## Mathematical Background

### Partial Information Decomposition (PID)

For two sources S1, S2 and target T, the total mutual information I(S1,S2;T) decomposes into:

```
I(S1,S2;T) = Red(S1,S2;T) + Unq(S1;T) + Unq(S2;T) + Syn(S1,S2;T)
```

Where:
- **Red** (Redundancy): Information about T available from *either* S1 or S2 alone
- **Unq(S1)**: Information about T available *only* from S1
- **Unq(S2)**: Information about T available *only* from S2
- **Syn** (Synergy): Information about T available *only* from S1 and S2 together

### Shared-Exclusions Redundancy (I^sx_∩)

PID-VLA uses the Wibral-group shared-exclusions definition (Makkeh et al. 2021; Ehrlich et al. 2024):

```
I^sx_∩(S1,S2;T) = ψ(k) + ψ(n) - ⟨ψ(n_α(i)) + ψ(n_T(i))⟩_i
```

This is estimated using a KSG-style kNN algorithm with disjunction neighborhoods.

### Units

All information quantities are reported in **nats** (natural logarithm). To convert to bits, divide by ln(2) ≈ 0.693.

## Configuration

### KSG Estimator

```rust
use pid_core::{KsgConfig, Metric, NegativeHandling};

let cfg = KsgConfig {
    k: 3,                                    // Number of nearest neighbors
    metric: Metric::Chebyshev,               // L∞ norm (standard for KSG)
    tie_epsilon: 0.0,                        // Tie-breaking threshold
    negative_handling: NegativeHandling::ClampToZero,
};
```

### PID Configuration

```rust
use pid_core::{Pid2Config, IsxConfig, IsxMethod};

let cfg = Pid2Config {
    ksg: KsgConfig::default(),
    isx: IsxConfig {
        k: 3,
        metric: Metric::Chebyshev,
        tie_epsilon: 0.0,
        method: IsxMethod::EhrlichKsg,  // Paper-faithful estimator
    },
};
```

## Estimator Caveats

⚠️ **Read before using on VLA embeddings (`grandplan.md` v5.5 Warning):**

1. **Manifold Geometry:** The continuous I^sx_∩ estimator relies on Chebyshev (L∞) geometry for exact product-ball cancellations. It **cannot** be applied directly to hyperbolic/Lorentz/manifold embeddings without a new derivation. See `grandplan.md` §16 for mitigation strategies (Unrolling, Geodesic MI, Quantization).

2. **i.i.d. assumption**: KSG estimators assume independent samples. VLA trajectories violate this—subsample or decorrelate.

3. **High dimension**: kNN distances concentrate in high ambient dimension. Use dimensionality reduction (PCA, hash projection) or check `distance_concentration_stats()`.

4. **Strong dependence**: Near-deterministic relationships (very large MI) require prohibitive sample sizes.

5. **Duplicates**: Exact duplicates collapse kNN radius to 0. Add jitter if needed.

6. **Negative values**: I^sx_∩ and PID atoms can be negative—this is mathematically valid, not an error.

## Development

### Commands

```bash
just build      # Build all crates
just test       # Run all tests
just fmt        # Format code
just lint       # Run clippy
just exp0       # Run Experiment 0 tests
just exp0-bin   # Run Experiment 0 binary
```

### Python Environment

```bash
uv sync                          # Install dependencies
uv run pytest                    # Run Python tests
uv run python scripts/analyze.py # Run analysis scripts
```

## References

**Core Methodology:**
- Makkeh, A., Gutknecht, A. J., & Wibral, M. (2021). Introducing a differentiable measure of pointwise shared information. *Phys Rev E*, 103:032149.
- Ehrlich, D. A., Schick-Poland, K., Makkeh, A., et al. (2024). Partial Information Decomposition for Continuous Variables based on Shared Exclusions. *Phys Rev E*, 110:014115.
- Gutknecht, A. J., et al. (2025). Shannon Invariants: A Scalable Approach to Information Decomposition. *arXiv:2504.15779*.

**Reference Implementations:**
- [continuouspidestimator](https://gitlab.gwdg.de/wibral/continuouspidestimator) (Python reference for I^sx_∩)
- [infomorphic_networks](https://gitlab.gwdg.de/wibral/infomorphic_networks) (Wibral lab research code)

## License

MIT License - see [LICENSE](LICENSE) for details.

## Citation

```bibtex
@software{pid_vla,
  title = {PID-VLA: Partial Information Decomposition for Vision-Language-Action Models},
  year = {2024},
  url = {https://github.com/your-org/pid-vla}
}
```