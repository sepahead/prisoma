# AGENTS.md

This file is for Codex CLI agents and contributors working in this repo.

Canonical spec: `grandplan.md` (v5.2, Jan 2026).

---

## Version Log

| Version | Date | Changes |
|---------|------|---------|
| **v5.2** | 2026-01-02 | **Explicit computation recipes + stage-wise validation:** implemented Level-0 discrete invariants (Red°/Vul°/Ω/CI) + exact toy tests; clarified how “entropy-only” is estimated in practice. |
| **v5.1** | 2026-01-02 | **Hypothesis coherence + manifold-first strategy:** unified H1–H4, elevated Shannon invariants (Red°, Vul°) as Level 0 for manifold/high-d regimes, updated §2.5 and §16.7. |
| **v5.0** | 2026-01-01 | **Final audit release:** Added PCA/kNN manifold limitations, confounding analysis, numerical stability, code audit. Grant-ready. |
| v4.0 | 2025-12-28 | Information geometry methods, intrinsic dimension, distance concentration |
| v3.0 | 2025-12-15 | 3-source PID, hierarchical screening improvements |
| v2.0 | 2025-12-01 | Preprocessing hooks, validation framework |
| v1.0 | 2025-11-15 | Initial KSG MI + `I^sx_∩` implementation |

---

## Progress Report (January 2026)

### Implementation Status

| Module | Status | Tests | Cross-check |
|--------|--------|-------|-------------|
| `ksg.rs` | ✅ Complete | ✅ Pass | vs sklearn KSG |
| `isx.rs` | ✅ Complete | ✅ Pass | vs csxpid (error < 1e-10) |
| `pid2.rs` | ✅ Complete | ✅ Pass | Identity constraints |
| `pid3.rs` | ✅ Complete | ✅ Pass | 18-atom Möbius inversion |
| `hierarchy.rs` | ✅ Complete | ✅ Pass | CI screening + triplet |
| `ci.rs` | ✅ Complete | ✅ Pass | |
| `invariants.rs` | ✅ Complete | ✅ Pass | Exact toy distributions (independent/redundant/XOR) |
| `geometry.rs` | ✅ Complete | ✅ Pass | ID scales correctly |
| `preprocess.rs` | ✅ Complete | ✅ Pass | |
| `bin/exp0.rs` | ✅ Complete | ✅ Pass | Synthetic + Gaussian channel |

### Remaining Work (Prioritized)

1. **[HIGH]** Python bindings (PyO3/maturin) for experiment harness
2. **[HIGH]** VLA embedding extraction on macOS (MLX/CoreML)
3. **[MEDIUM]** PCA implementation (Python-first, then optional Rust)
4. **[LOW]** SIMD/parallel acceleration (rayon)
5. **[LOW]** Ball-tree/KD-tree for low-d speedup

---

## Why PCA and kNN Are Suboptimal for Manifold-Valued Embeddings

**This section is critical for understanding the limitations of our approach.**

VLA embeddings empirically lie on **low-dimensional manifolds** embedded in high-dimensional space (~4096 dims). Standard Euclidean tools fail systematically on such data.

### The Core Problem

```
MANIFOLD STRUCTURE vs EUCLIDEAN ASSUMPTION
==========================================

What PCA/kNN assume:           Reality (VLA embeddings):

    •  •  •  •  •               ╭────────────────╮
    •  •  •  •  •              ╱   M ⊂ ℝ⁴⁰⁹⁶     ╲
    •  •  •  •  •             │  (curved manifold) │
    (uniform in ℝᵈ)           │   ID ≈ 50-200     │
                               ╲                  ╱
                                ╰────────────────╯
```

### Why PCA Fails

1. **Linear variance ≠ manifold structure:**
   - PCA finds directions of maximum **linear** variance
   - A curved manifold (e.g., Swiss roll) has high variance in multiple linear directions
   - PCA may retain 3 dimensions for a manifold with intrinsic dimension 1

2. **Geodesic distortion:**
   - Two points close along the manifold may be far in Euclidean space
   - PCA preserves Euclidean distances, not geodesic distances
   - After PCA projection, true neighbors may become separated

3. **High-curvature artifacts:**
   - Regions where the manifold curves sharply project to overlapping linear subspaces
   - Semantically distinct regions become indistinguishable

**When PCA is acceptable:**
- Manifold curvature is low (approximately linear)
- Variance retention is high (≥95%)
- Experiment 0 validates stability after PCA
- Intrinsic dimension is preserved (ID_after ≈ ID_before)

### Why Euclidean kNN Fails

1. **The shortcut problem:**
   ```
   Manifold path:              Euclidean shortcut:
       A ─────────╮                A
                  │                 ╲
      (geodesic)  │                  ╲ (through ambient space)
                  │                   ╲
       B ─────────╯                    B

   kNN declares A, B neighbors even though geodesically far
   ```

2. **Volume estimation error:**
   - KSG estimates density via hypersphere volumes
   - On curved manifolds, hyperspheres have wrong volume
   - Density estimates are systematically biased

3. **Compounding error:**
   - Bias grows exponentially with intrinsic dimension
   - At ID > 20, Euclidean kNN may be meaningless

### Diagnostic Protocol (Implemented)

Before running PID on VLA embeddings, always:

```rust
// 1. Estimate intrinsic dimension
let id = intrinsic_dimension_levina_bickel(data, &cfg)?;

// 2. Check distance concentration
let dc_stats = distance_concentration_stats(data, &cfg)?;

// 3. Decision logic
if id < ambient_dim / 10.0 {
    warn!("Manifold structure significant: ID={:.1} << d={}", id, ambient_dim);
}
if dc_stats.pairwise_cv < 0.2 {
    warn!("Distance concentration: CV={:.3}; kNN unreliable", dc_stats.pairwise_cv);
}
```

### Decision Flowchart

```
MANIFOLD METHODS DECISION TREE
==============================

1. Compute intrinsic dimension (ID)
   └── ID < ambient_dim / 10?
       ├── YES → Manifold effects likely significant → Step 2
       └── NO → Euclidean methods may suffice → Validate with Exp 0

2. Compute distance concentration (DC)
   └── CV of pairwise distances < 0.2?
       ├── YES → kNN unreliable → Step 3
       └── NO → Euclidean kNN may work → Validate with Exp 0

3. Attempt dimensionality reduction
   ├── PCA (95% variance) → Re-run Exp 0 → Stable?
   │   ├── YES → Use PCA-reduced data
   │   └── NO → Try random projection
   └── Random projection → Re-run Exp 0 → Stable?
       ├── YES → Use projected data
       └── NO → PIVOT: Use Shannon invariants (CI) only
```

### Alternatives to Standard Methods

| Method | When to Use | Limitations | Implemented? |
|--------|-------------|-------------|--------------|
| **PCA** | Low curvature, high variance retention | Distorts curved manifolds | Python (planned) |
| **Random projection** | Approximate distance preservation | No manifold awareness | ✅ `HashProjector` |
| **Isomap** | When geodesic structure matters | Sensitive to noise, holes | No |
| **Geodesic kNN MI** | Manifold-valued embeddings | O(n² log n), MI-only | No |
| **Shannon invariants (CI)** | When `I^sx_∩` is unstable | Not full PID | ✅ `ci.rs` |

### Practical Recommendations

1. **Always run geometry diagnostics first:**
   - Compute ID via `intrinsic_dimension_levina_bickel`
   - Compute distance concentration via `distance_concentration_stats`

2. **If ID << ambient dimension:**
   - Try PCA with 95% variance retention
   - Re-run Experiment 0 subset
   - Compare estimates before/after

3. **If estimates are unstable:**
   - Fall back to Shannon invariants (CI) for screening
   - Report instability as a finding
   - Do NOT claim `I^sx_∩` is valid in this regime

4. **Never silently apply transforms:**
   - Log all preprocessing steps
   - Record intrinsic dimension at each stage
   - Include geometry diagnostics in results

See `grandplan.md` §16 for full theoretical analysis and §15 for numerical stability guidance.

---

## Reproducibility (required; macOS-first)

- Use the Nix dev shell (`flake.nix`) for a pinned toolchain: `nix develop`.
- Use `uv` for Python **always** (never use `pip` directly):
  - Install/sync: `uv sync --frozen` (exactly reproduces `uv.lock`)
  - Run scripts: `uv run python …`
- Lockfiles are part of the contract:
  - Commit `uv.lock`.
  - Commit `flake.lock` (generate/update with `nix flake lock`).
  - Do not hand-edit lockfiles.

## Repo status snapshot (Jan 2026)

- Implemented (Rust): `crates/pid-core`
  - KSG MI (`ksg_mi`, Chebyshev/L∞ + strict-radius tie handling)
  - Continuous `I^sx_∩(S1,S2;T)` (`isx_redundancy`, `IsxMethod::EhrlichKsg`) + a fixed-data cross-check test vs `csxpid`
  - 2-source PID atoms (`pid2_isx`)
  - Hierarchical screening (`hierarchical_pairwise`, `hierarchical_triplet`)
  - Optional full 3-source continuous SxPID (`pid3_isx`, offline/expensive)
  - Preprocessing helpers (`Standardizer`, `Jitter`, `HashProjector`)
  - Geometry diagnostics:
    - intrinsic dimension (`intrinsic_dimension_levina_bickel`)
    - basic distance concentration proxies (`distance_concentration_stats`)
  - Quick synthetic runner (`cargo run -p pid-core --bin exp0`)
- Implemented (tooling): `justfile` with `build`, `test`, `exp0`, `exp0-bin`
- Missing (planned next): Python experiment harness + macOS-first VLA embedding extraction (MLX/CoreML)
- Current limitations (do not pretend otherwise):
  - kNN backend is brute-force `O(n²)` (reference correctness path)
  - `Metric` currently implements **Chebyshev only**
  - Intrinsic dimension is implemented; distance concentration has basic proxies, but may need expansion
  - PCA / marginal Gaussianization are not implemented in Rust (plan them in Python first)

## Step-by-step plan (project roadmap)

1. **Maintain and extend Wibral PID in Rust (I^sx_∩, continuous; “latest” in spec = Makkeh et al. 2021 + Ehrlich et al. 2024):**
   - `pid-core` already exists; keep estimator semantics stable while extending capabilities (bindings, harness, performance).
   - Prioritize: correctness, numerical stability, reproducibility (seeded RNG, pinned deps), and a clean API that can later support 3-way PID and alternative estimators.
   - Include validation tests mirroring Experiment 0 scenarios (independent/additive, XOR-like synergy, redundant/copy, scaling).
2. **Adopt a hierarchical computation design (sound + adaptable):**
   - Level 1: fast screening via Shannon invariants (pairwise co-information from KSG MI).
   - Level 2: targeted full pairwise I^sx_∩ PID on suspicious pairs.
   - Level 3: optional 3-way PID (offline only) once pairwise behavior is understood.
3. **Implement dimensionality reduction + preprocessing hooks:**
   - Already implemented in Rust: standardization, small jitter, and a dependency-free hash projection baseline.
   - Planned (prefer Python-first): PCA (retain variance target), random projections, optional invertible reparameterizations (e.g., monotone marginal Gaussianization).
   - Add **geometry diagnostics** to decide whether kNN-based MI/`I^sx_∩` is even plausible:
     - intrinsic dimension: implemented
     - distance concentration (basic proxies): implemented
   - Make dim reduction explicit and logged so results are interpretable/reproducible.
4. **Build experiment harnesses and reproducibility scaffolding:**
   - `justfile` already exists; extend it as the stable entrypoint for experiments.
   - Keep environments pinned with Nix (`flake.lock`) + uv (`uv.lock`); record `rustc --version`, Python version, and seeds in every run artifact.
   - Add benchmarking (runtime vs N,d,k; memory) to enforce “real-time” viability for Level 1.
5. **Experiment 0 gate (do before any VLA claims):**
   - Run synthetic validation across {N,d,k} grids up to VLA-like d (or demonstrate why dim reduction is required).
   - Include **strong-dependence sweeps** (near-deterministic/large-MI regimes) as a separate axis
     from “high `d`” (see Gao et al. 2015). Do not treat low-`d` success as proof of robustness
     under strong dependence.
   - Include **geometry diagnostics** as a separate axis: intrinsic-dimension estimates and
     distance-concentration proxies. Treat “kNN works after PCA” as unproven unless intrinsic
     dimension is also low/stable.
   - Record error/variance/cost; decide GO / PIVOT / NO-GO using the thresholds in `grandplan.md`
     (e.g., error targets: d=10 <5%, d=100 <10%, d=1000 <15%, d=4096 <20% or require dim
     reduction).
   - Decision rule of thumb from spec: **GO** if stable at d=4096; **PIVOT** if stable only after
     PCA/random projection (e.g., d≈256); **NO-GO** if unstable even at d≈256 (abandon `I^sx_∩` for
     alternatives).
   - Optional: use `sae_analysis` Shannon invariants (Red°, Vul°) as **heuristic screening / SAE-compression tooling** per `grandplan.md` Appendix B.3.3.5 (not a correctness oracle for `I^sx_∩`).
6. **VLA data plumbing:**
   - Define a stable on-disk format for rollouts + embeddings.
   - Implement extraction for (V, L, D, A) and optionally A* (“optimal action”) per the selected benchmark.
7. **Run Experiments 1–3 (comparative evaluation, baselines, dimensionality study):**
   - Predefine success criteria (AUROC + significance vs best baseline).
   - Track which decomposition is most predictive and which preprocessing is safest.
8. **Run Experiment 4 (causal validation):**
   - Controlled interventions on D; test expected synergy changes and corresponding failure-rate shifts.
9. **Only if Aim 1 succeeds:**
   - Aim 2: synergy dynamics (half-life) analysis.
   - Aim 3: RL fine-tuning using PID-derived intrinsic reward (treat as exploratory).
10. **(Optional) PixelVLA/TraceVLA + visualization integration:**
   - If targeting PixelVLA or TraceVLA, use `grandplan.md` §7.3–7.4 and §10.8.7 (PixelVLA + headless Gazebo + Tauri) as the integration sketch; treat as post-Experiment-0 work.

## Rust implementation specification (long-lived)

This is the engineering contract for the Rust core (`pid-core`). Keep it stable across the project so future agents can extend/optimize without changing the scientific meaning.

### Scope (what we must implement)

- **Two-source PID atoms** for continuous variables using Wibral-group shared-exclusions redundancy `I^sx_∩`:
  - Inputs: `S1` (n×d1), `S2` (n×d2), `T` (n×dt)
  - Outputs: `Red = I^sx_∩(S1,S2;T)`, `Unq1`, `Unq2`, `Syn`
  - Identity constraints (2-source PID):  
    - `Unq1 = I(S1;T) − Red`  
    - `Unq2 = I(S2;T) − Red`  
    - `Syn  = I(S1,S2;T) − I(S1;T) − I(S2;T) + Red`
- **Estimator building block:** KSG mutual information `I(X;Y)` for continuous variables (KSG-1 style), as required by the spec.
- **Hierarchical path (sound default):**
  - Level 1: fast screening via Shannon invariants / co-information computed from KSG MI (cheap; usable online)
  - Level 2: full pairwise `I^sx_∩` PID (slower; targeted)
  - Level 3: 3-way PID (offline; only after pairwise behavior is validated)

Level-1 co-information (for a pair of sources `X,Y` and target `T`) uses MI terms only:
- `CI(X,Y;T) = I(X;T) + I(Y;T) − I(X,Y;T)`  
  Sign convention used in the spec: **negative CI indicates synergy** (the joint provides more than the sum of parts).
  This is *not* `I^sx_∩` synergy, but it is fast and works as a screening/triage layer.

### Non-goals (until later)

- Do not implement alternative PID measures unless needed for baselines.
- Do not treat negative synergy as “bug”: `I^sx_∩` does not guarantee non-negativity; negative synergy is allowed and must be representable.
- Do not claim VLA conclusions until Experiment 0 validates the estimator regime.

### Required reading (sources + where they map into this repo)

Papers (authoritative):
- **Makkeh A, Gutknecht AJ, Wibral M (2021)** — *Phys Rev E* 103:032149.
  “Introducing a differentiable measure of pointwise shared information.” Defines `I^sx_∩`.
  DOI: `https://doi.org/10.1103/PhysRevE.103.032149`
- **Ehrlich DA, Schick-Poland K, Makkeh A, Lanfermann F, Wollstadt P, Wibral M (2024)** —
  *Phys Rev E* 110:014115. “Partial Information Decomposition for Continuous Variables based on
  Shared Exclusions.” Continuous `I^sx_∩` estimator details.
  DOI: `https://doi.org/10.1103/PhysRevE.110.014115`
- **Kraskov A, Stögbauer H, Grassberger P (2004)** — *Phys Rev E* 69:066138.
  “Estimating mutual information.” KSG estimator details (max-norm usage, neighbor counting,
  bias/variance behavior).
  DOI: `https://doi.org/10.1103/PhysRevE.69.066138`

Context/guardrails:
- **Gutknecht AJ, Rosas FE, Ehrlich DA, Makkeh A, Mediano PAM, Wibral M (2025)** —
  arXiv:2504.15779. “Shannon Invariants: A Scalable Approach to Information Decomposition.”
  `https://arxiv.org/abs/2504.15779`
- **Matthias PH, Makkeh A, Wibral M, Gutknecht AJ (2025)** — arXiv:2512.16662.
  Impossibility/inconsistency results (explains why `I^sx_∩` can have negative atoms).
  `https://arxiv.org/abs/2512.16662`
- **Gao S, Ver Steeg G, Galstyan A (2015)** — arXiv:1411.2003.
  Strong-dependence sample complexity pathologies for kNN MI estimators (relevant to
  “near-deterministic” VLA variables).
  `https://arxiv.org/abs/1411.2003`
- **Gao S, Ver Steeg G, Galstyan A (2015)** — arXiv:1508.00536. Local Gaussian MI estimator (strong-dependence correction / MI baseline). `https://arxiv.org/abs/1508.00536`
- **Belghazi MI et al. (2018)** — arXiv:1801.04062.
  MINE (neural MI; treat as a separate validated MI pipeline for MI-only screening, not drop-in
  `I^sx_∩`).
  `https://arxiv.org/abs/1801.04062`
- **Mukherjee S, Asnani H, Kannan S (2019)** — arXiv:1906.01824.
  CCMI (classifier-based conditional MI; useful if conditioning becomes central; separate
  validated pipeline).
  `https://arxiv.org/abs/1906.01824`
- **Marx A, Fischer J (2021)** — arXiv:2110.13883. Geodesic kNN MI estimation on Riemannian manifolds (MI-only baseline if embeddings are curved/manifold-valued). `https://arxiv.org/abs/2110.13883`
- **Nickel M, Kiela D (2017)** — arXiv:1705.08039. Poincaré embeddings (hyperbolic geometry for hierarchies; optional learned projection). `https://arxiv.org/abs/1705.08039`
- **Nickel M, Kiela D (2018)** — arXiv:1806.03417. Lorentz (hyperboloid) model hyperbolic embeddings (often more stable than Poincaré ball). `https://arxiv.org/abs/1806.03417`
- **Ganea O-E, Bécigneul G, Hofmann T (2018)** — arXiv:1805.09112. Hyperbolic Neural Networks (background). `https://arxiv.org/abs/1805.09112`
- Differential-geometry contingencies are covered in `grandplan.md` §8.1.5 (optional background; not an estimator spec).
- **Liang PP et al. (2023)** — NeurIPS 2023. Multimodal “PID” estimators (BATCH/CVX) for baselines; not the same as `I^sx_∩`. Code: `https://github.com/pliang279/PID`
- **PixelVLA (2025)** — arXiv:2511.01571. Pixel-level understanding + visual prompting for VLAs (optional future target; see `grandplan.md` §7.3). `https://arxiv.org/abs/2511.01571`
- **TraceVLA (2024)** — arXiv:2412.10345. Visual trace prompting for spatial-temporal awareness (optional future target; see `grandplan.md` §7.4). `https://arxiv.org/abs/2412.10345`

In-repo pointers (use these to stay aligned with the spec):
- `grandplan.md` §2.2 (`I^sx_∩` definition), §2.3 (continuous extension), §8.1 (KSG details),
  §2.5.4 (hierarchical strategy), §9.1 (Experiment 0), Appendix B.3.4 (validation notes).
  - Current code locations (Rust):
    - `crates/pid-core/src/ksg.rs` — KSG MI (+ local MI terms)
    - `crates/pid-core/src/isx.rs` — continuous `I^sx_∩(S1,S2;T)` (`IsxMethod::EhrlichKsg`)
    - `crates/pid-core/src/pid2.rs` — 2-source PID atoms wrapper
    - `crates/pid-core/src/ci.rs` — pairwise/triplet co-information helpers
    - `crates/pid-core/src/hierarchy.rs` — hierarchical screening + optional full 3-source PID
    - `crates/pid-core/src/pid3.rs` — full 3-source continuous SxPID (offline)
    - `crates/pid-core/src/geometry.rs` — intrinsic dimension + distance concentration diagnostics
    - `crates/pid-core/src/preprocess.rs` — standardization/jitter/hash projection
    - `crates/pid-core/src/matrix.rs`, `crates/pid-core/src/metric.rs`, `crates/pid-core/src/nn.rs`
      — data layout + metric + brute-force kNN utilities

Reference code (for sanity checks and baselines; verify commit hashes when used):
- **Continuous `I^sx_∩` (authors’ reference impl):**
  `https://gitlab.gwdg.de/wibral/continuouspidestimator` (Python package `csxpid`; Ehrlich et al.
  2024; uses KDTree/ball-tree variants + merging procedure for disjunction distances)
- **Discrete `I^sx_∩` (definitions/lattice sanity):** `https://github.com/Abzinger/SxPID`
- **Related (Shannon invariants on neural latents; NOT `I^sx_∩`):**
  `https://github.com/Abzinger/sae_analysis` (WIP; computes redundancy/vulnerability-style
  invariants for SAEs; uses submodules `sparsify`/`delphi`; treat as reference only)
- **Baseline estimators (NOT `I^sx_∩`):** `https://github.com/pliang279/PID` (Liang BATCH/CVX)
- **General info-dynamics toolkit:** `https://github.com/pwollstadt/IDTxl`

Quick comparison (to avoid false equivalences):
- `Abzinger/sae_analysis` is **discrete PMF / plug-in entropy**-based and targets **SAE latents**
  + **Shannon invariants** (degree of redundancy/vulnerability). It does **not** implement
  continuous KSG MI or continuous shared-exclusions `I^sx_∩`.
- `Abzinger/sae_analysis` uses `log2` (bits) throughout; `crates/pid-core` currently reports MI/PID in natural logs (nats).
- `crates/pid-core` in this repo is **continuous kNN/KSG**-based and targets **Wibral-group shared-exclusions PID** (`I^sx_∩`) + pairwise co-information screening.

### Engineering requirements (quality bar)

- **Reproducibility:** deterministic results for fixed `(seed, k, metric, preprocessing, dim-reduction)`; record configs with outputs.
- **Units:** pick one and be consistent (recommend nats internally + explicit `to_bits()` helper).
- **Preprocessing is explicit:** standardize/whiten and (if used) PCA/random projection must be recorded; never silently change dimensionality.
- **Robust error handling:** detect/return informative errors for shape mismatch, `n <= k`, NaNs/Infs, degenerate distances, etc.
- **Validation-first:** merging estimator changes requires rerunning Experiment 0 (or at least the representative subset) and updating expected tolerances.
- **API stability:** treat `pid-core` as a long-lived dependency; make breaking API changes deliberately (semver + migration notes).

### Mathematical definitions (do not reinterpret)

Keep these definitions consistent across Rust, Python bindings, and plots.

- **Pointwise mutual information (PMI):**  
  `i(s; t) = log( p(s,t) / (p(s) p(t)) )`
- **Shared-exclusions redundancy (Wibral group, conceptual):**  
  `i^sx_∩(s1,s2;t) = log( p(t | (S1=s1) ∨ (S2=s2)) / p(t) )` (Makkeh et al. 2021).  
  Continuous estimation uses the KSG-style kNN construction in Ehrlich et al. 2024 (Appendix H; Algorithms 3–6), implemented in this repo as `IsxMethod::EhrlichKsg`.
- **Two-source PID atoms derived from MI + redundancy:**  
  `Unq1 = I(S1;T) − Red`  
  `Unq2 = I(S2;T) − Red`  
  `Syn  = I(S1,S2;T) − I(S1;T) − I(S2;T) + Red`

Implementation detail: decide whether `log` is natural (nats) or base-2 (bits) and keep it consistent end-to-end.

### Suggested crate/API shape (so future work stays stable)

Current Rust workspace layout (authoritative; keep docs aligned with this):
- `crates/pid-core/src/lib.rs` — public API surface
- `crates/pid-core/src/error.rs` — error types (`PidError`, `PidResult`)
- `crates/pid-core/src/matrix.rs` — `MatRef`/`MatOwned` (row-major `n×d` floats)
- `crates/pid-core/src/metric.rs` — `Metric` (currently Chebyshev only)
- `crates/pid-core/src/nn.rs` — brute-force kNN helpers + strict-radius tie handling
- `crates/pid-core/src/stats.rs` — digamma utilities
- `crates/pid-core/src/ksg.rs` — KSG MI (`ksg_mi`, `ksg_mi_concat_xy`, local terms)
- `crates/pid-core/src/isx.rs` — continuous `I^sx_∩(S1,S2;T)` (`isx_redundancy`)
- `crates/pid-core/src/pid2.rs` — 2-source PID atoms wrapper (`pid2_isx`)
- `crates/pid-core/src/ci.rs` — co-information helpers (`co_information_pairwise`, etc.)
- `crates/pid-core/src/hierarchy.rs` — hierarchical screening + optional full 3-source PID
- `crates/pid-core/src/geometry.rs` — intrinsic-dimension diagnostic
- `crates/pid-core/src/distance_matrix.rs` — symmetric distances (used by `pid3_isx`)
- `crates/pid-core/src/pid3.rs` — full 3-source continuous SxPID (18 atoms; offline)
- `crates/pid-core/src/preprocess.rs` — standardization/jitter/hash projection
- `crates/pid-core/src/bin/exp0.rs` — quick synthetic runner

Public API sketch (keep stable; allow internal refactors):
- `fn ksg_mi(x: MatRef<'_>, y: MatRef<'_>, cfg: &KsgConfig) -> PidResult<f64>`
- `fn isx_redundancy(s1: MatRef<'_>, s2: MatRef<'_>, t: MatRef<'_>, cfg: &IsxConfig) -> PidResult<f64>`
- `fn pid2_isx(s1: MatRef<'_>, s2: MatRef<'_>, t: MatRef<'_>, cfg: &Pid2Config) -> PidResult<Pid2Result>`
- `fn hierarchical_pairwise(sources: &[MatRef<'_>], target: MatRef<'_>, cfg: &HierarchicalConfig) -> PidResult<Vec<PairwiseScreen>>`

Implementation note: keep a single canonical “row-major (`n×d`) float” convention and enforce it everywhere.

### Preprocessing + dimensionality reduction rules (avoid silent bugs)

- Always apply **the same preprocessing pipeline** to all variables involved in a computation, but **fit transforms without mixing variables**:
  - Fit/transform `S1`, `S2`, and `T` independently (no PCA on `[S1|S2|T]` concatenations).
  - Log/serialize the fitted transform (mean/std; PCA components; random projection matrix seed).
- Default preprocessing (minimum):
  - Per-dimension standardization (zero mean, unit variance). Implemented in Rust as `Standardizer`.
    (Min-max scaling is not currently implemented.)
  - Optional: monotone marginal Gaussianization (rank/CDF→Normal) as an **invertible**
    per-dimension reparameterization to improve kNN geometry (does not change true MI when applied
    per variable; still re-validate estimator behavior). (TODO: Python-first.)
  - Optional: small jitter for duplicate points (seeded) to avoid kNN tie pathologies. Implemented
    in Rust as `Jitter`; record when enabled.
- Dimensionality reduction (only after Experiment 0 justifies it):
  - PCA: pick variance-retained target (e.g., 95%) or fixed component count; record the achieved
    dimension. (TODO: Python-first.)
  - Random projection: use a seeded matrix; record seed + target dimension.
    - Current dependency-free baseline in this repo: `HashProjector` (feature hashing / CountSketch) in `crates/pid-core/src/preprocess.rs`.
  - Any reduction changes the quantity being estimated (non-invertible transform); always report it with results.

### kNN backend requirements (exact first, pluggable later)

- Implement a **correct brute-force kNN** path first (baseline truth for tests/benchmarks).
- Add optional acceleration only if it preserves semantics (same metric + tie rules):
  - KD-tree/ball-tree may help at low `d` but degrade at high `d`.
  - Approximate methods (HNSW/FAISS) are allowed only behind an explicit “approx” flag and require re-validation (Experiment 0 subset) to quantify bias.

### Algorithmic details that must not drift

KSG mutual information (continuous, k-NN):
- Use **Chebyshev / L∞ (max-norm)** for both (a) kNN search in joint space and (b) marginal counting (this is the standard KSG convention referenced in the spec).
- Use **strict inequality** when counting neighbors within `ε` (KSG tie-handling); document and test the tie rule because it affects bias.
- Implement/use a reliable **digamma** `ψ(·)` and avoid ad-hoc `log` substitutions.
- Always validate behavior on low-dimensional synthetic data before trusting high-dimensional runs.

Continuous `I^sx_∩` redundancy (Ehrlich et al. 2024):
- Implement from the paper and **cross-check against the authors’ reference implementation**
  (`csxpid`, `gitlab.gwdg.de/wibral/continuouspidestimator`; vendored at
  `.external/repos/continuouspidestimator`). Treat Appendix B.3.4 in `grandplan.md` as a sketch,
  not as a proof of correctness.
- Keep the estimator factored so we can swap kNN backends (exact vs approximate) without changing math.

High-dimensional regime handling:
- Expect **distance concentration** and estimator collapse at large `d`; do not hide this.
  Detect it via intrinsic-dimension estimates (implemented) and distance-concentration proxies
  (TODO), then trigger the Experiment 0 “PIVOT” path (dim reduction).
- Default approach: PCA to ~256 dims (variance retained target) + rerun Experiment 0 to re-establish accuracy.
- Strong dependence is a separate pathology from high `d`: large true MI (near-deterministic
  mappings) can break kNN MI/PID at low `d` unless sample sizes are enormous (Gao et al. 2015).
  Treat “noiseless” signals with extreme caution.
- Do not mix estimator families inside PID identities (e.g., do not combine MINE MI terms with disjunction-kNN redundancy in `Syn = I(S1,S2;T) − I(S1;T) − I(S2;T) + Red`).

### Result reporting (make downstream experiments reproducible)

Every experiment output that depends on the Rust estimator should record:
- estimator versions (crate git rev / crate version), configs (`k`, metric, log base, preprocessing, dim reduction),
- environment (OS/arch, `rustc --version`, BLAS/GPU backend if applicable),
- sample sizes and effective dimensions after reduction,
- random seeds for any stochastic step (jitter, bootstrap, random projection),
- warnings/diagnostics (e.g., intrinsic dimension, distance-concentration proxies (TODO),
  excessive ties, NaNs clamped/filtered).

### Validation obligations (what to test, always)

Experiment 0 (required gate; see `grandplan.md` §9.1):
- Synthetic generators with known qualitative structure:
  - Independent/additive (synergy ~ 0)
  - XOR-like (synergy > 0)
  - Redundant/copy (redundancy high, synergy ~ 0)
- Scaling sweeps across `{d, n, k}` up to the intended operating point (or show failure and pivot to dim reduction).
- Add a **strong-dependence sweep** (fixed small `d`, increasing MI / decreasing noise) to detect “kNN fails because MI is huge” regimes separately from “kNN fails because `d` is huge”.
- Report: mean estimate, variance across seeds, runtime, memory; classify GO/PIVOT/NO-GO.

Cross-checks (recommended):
- For small `d` and moderate `n`, compare MI estimates against a known-good Python implementation (e.g., SciPy/sklearn-based KSG) to catch off-by-one/tie bugs.
- For small `d`, cross-check `I^sx_∩` redundancy against `csxpid` (authors’ reference impl) to catch disjunction-distance/tie-rule bugs.
- Add invariants-based smoke tests:
  - `I(S1,S2;T)` should approximately equal `Red + Unq1 + Unq2 + Syn` (numerical tolerance)
  - `Unq1 + Red` should approximately equal `I(S1;T)` (same for S2)

### Performance targets (pragmatic)

- Level 1 (co-information screening): aim for “interactive” latency on moderate `n` (e.g., ~10ms–tens of ms range per pair, depending on `n,d`).
- Level 2 (full `I^sx_∩` PID): acceptable slower runtime, but must be benchmarked and profiled; optimize hotspots (distance calcs, neighbor counting) before adding GPU complexity.

## Experiments checklist (what to run)

- **Experiment 0 (mandatory):** synthetic validation at increasing dimensionality; go/no-go + dim-reduction pivot.
- **Experiment 1:** decomposition comparison (V-D-A vs V-L-A vs V-D-A* vs hierarchical pairwise).
- **Experiment 2:** baseline comparison (entropy/uncertainty, Liang BATCH/CVX, learned classifier, etc.).
- **Experiment 3:** dimensionality study (raw vs PCA vs random projection vs learned projection vs intermediate layers).
- **Experiment 4:** causal validation (intervene on D; measure synergy + failure rate effects).

## Skills (runtime-discovered)

These are the currently available Codex skills (paths are machine-local); use them when a task matches their `description`:

- `skill-creator`: Guide for creating effective skills. (file: `/Users/torusprime/.codex/skills/.system/skill-creator/SKILL.md`)
- `skill-installer`: Install Codex skills into `$CODEX_HOME/skills` from a curated list or a GitHub repo path. (file: `/Users/torusprime/.codex/skills/.system/skill-installer/SKILL.md`)

Trigger rules summary:
- Use a skill if the user names it (e.g., `$SkillName`) or if the request clearly matches its description.
- Keep context small; load only the minimum referenced files needed to execute the skill workflow.
