# pid_vla

Engineering roadmap for implementing and validating Wibral-group shared-exclusions PID (SxPID, `I^sx_∩`) for Vision-Language-Action (VLA) diagnostics.

Canonical research specification: `grandplan.md` (v5.3, Jan 2026).

---

## Version Log

| Version | Date | Changes |
|---------|------|---------|
| **v5.3** | 2026-01-02 | **Docs sync + README consolidation:** updated canonical spec version; removed outdated “build-from-scratch” milestones so the README roadmap matches the current repo state. |
| **v5.2** | 2026-01-02 | **Shannon invariants (exact discrete):** added `pid-core` discrete invariants (`invariants.rs`: entropies, Red°, Vul°, Ω, discrete CI) + tests for XOR/redundant/independent toy systems to provide exact sanity checks for estimator/approximation changes. |
| **v5.1** | 2026-01-02 | **Coherence + manifold/hierarchy alignment:** clarified hypothesis↔aims framing, strengthened geometry-first + Shannon-invariants hierarchy workflow, and made exact-baseline validation requirements explicit for approximations/accelerations. |
| **v5.0** | 2026-01-01 | **Final audit release:** Added confounding factors analysis (§14), numerical stability guidance (§15), manifold/PCA/kNN limitations (§16). Information geometry methods integrated. Code audit complete. Grant-ready documentation. |
| v4.0 | 2025-12-28 | Added information geometry methods, intrinsic dimension diagnostics, distance concentration proxies |
| v3.0 | 2025-12-15 | Critical review and gameplan adjustments, 3-source PID implementation |
| v2.0 | 2025-12-01 | Hierarchical screening, preprocessing hooks |
| v1.0 | 2025-11-15 | Initial KSG MI + `I^sx_∩` implementation |

---

## Progress Report (January 2026)

### Completed (Ready for Validation)

| Component | Status | Location | Notes |
|-----------|--------|----------|-------|
| **KSG Mutual Information** | ✅ Complete | `crates/pid-core/src/ksg.rs` | Chebyshev/L∞, strict-radius ties, cross-checked |
| **Continuous I^sx_∩** | ✅ Complete | `crates/pid-core/src/isx.rs` | Multiple methods: EhrlichKsg (reference), LocalMinKsg, GrandplanSketch, DisjunctionFromLocalMi |
| **2-source PID** | ✅ Complete | `crates/pid-core/src/pid2.rs` | {Red, Unq1, Unq2, Syn} atoms |
| **3-source PID** | ✅ Complete | `crates/pid-core/src/pid3.rs` | 18 atoms, Möbius inversion, offline only |
| **Hierarchical screening** | ✅ Complete | `crates/pid-core/src/hierarchy.rs` | Fast CI → targeted PID, 3-source triplet |
| **Co-information** | ✅ Complete | `crates/pid-core/src/ci.rs` | Pairwise and triplet CI |
| **Shannon invariants (exact discrete)** | ✅ Complete | `crates/pid-core/src/invariants.rs` | Entropies + Red°/Vul°/Ω + discrete CI (toy-system exact baselines) |
| **Intrinsic dimension** | ✅ Complete | `crates/pid-core/src/geometry.rs` | Levina-Bickel MLE |
| **Distance concentration** | ✅ Complete | `crates/pid-core/src/geometry.rs` | CV, NN ratio diagnostics |
| **Preprocessing** | ✅ Complete | `crates/pid-core/src/preprocess.rs` | Standardizer, Jitter, HashProjector |
| **Experiment 0 runner** | ✅ Complete | `crates/pid-core/src/bin/exp0.rs` | Synthetic validation + Gaussian channel sweep + geometry diagnostics |

### In Progress

| Component | Status | Priority | Blocker |
|-----------|--------|----------|---------|
| Python bindings (PyO3) | 🔄 Planned | High | None |
| VLA embedding extraction (MLX) | 🔄 Planned | High | Requires Python harness |
| PCA implementation | 🔄 Planned (Python-first) | Medium | None |
| SIMD acceleration | 🔄 Optional | Low | Performance profiling needed |

### Validation Status

| Test | Result | Notes |
|------|--------|-------|
| `csxpid` cross-check (fixed data) | ✅ Pass | Error < 1e-10 |
| Synthetic generators (d≤10) | ✅ Pass | All scenarios (independent, redundant, unique, XOR) |
| High-d synthetic (d=256) | ⚠️ Partial | Requires hash projection; estimates drift with d |
| Gaussian channel (strong dependence) | ⚠️ Partial | Underestimates at σ < 0.03 (expected per Gao et al.) |
| Intrinsic dimension accuracy | ✅ Pass | Increases correctly with true dimension |

### Known Limitations (Be Honest)

1. **kNN is brute-force O(n²):** Acceptable for Experiment 0 but not real-time at n > 10k
2. **Only Chebyshev metric:** Euclidean/other metrics not implemented
3. **No PCA in Rust:** Must use Python or `HashProjector` baseline
4. **Strong dependence regime:** Estimates degrade when true MI > ~4 nats
5. **Manifold-aware estimation not implemented:** geometry diagnostics exist, but Euclidean kNN may fail on curved embeddings (see `grandplan.md` §16)
6. **No parallelization yet:** Single-threaded; rayon integration planned

---

## Critical Considerations for Grant Reviewers

### Why PCA/kNN May Be Suboptimal for VLA Embeddings

VLA embeddings lie on **low-dimensional manifolds** in high-dimensional space. Standard tools fail:

1. **PCA preserves linear variance, not geodesic structure:**
   - A spiral has high 3D variance but intrinsic dimension 1
   - PCA retains all 3 components, missing the 1D structure
   - After PCA, kNN may find "wrong" neighbors (Euclidean shortcuts)

2. **Euclidean kNN finds shortcuts through ambient space:**
   - Points far apart on the manifold may be close in Euclidean distance
   - This biases density estimates and MI/PID calculations
   - Bias compounds exponentially with intrinsic dimension

3. **When to suspect manifold effects:**
   - Intrinsic dimension << ambient dimension (e.g., ID=50 but d=4096)
   - Distance concentration coefficient of variation < 0.2
   - Estimates unstable across preprocessing choices

**Mitigation strategy (implemented):**
- Compute intrinsic dimension before estimating (§16.5 of grandplan.md)
- Check distance concentration as a "geometry health check"
- If manifold effects are significant, fall back to Shannon invariants (CI) for screening

See `grandplan.md` §16 for detailed analysis and decision flowcharts.

---

## Status (what exists today)

- Reproducible tooling scaffold:
  - `flake.nix` (dev shell), `pyproject.toml`, `uv.lock`
  - `flake.lock` should be generated and committed once Nix is installed (`nix flake lock`)
- Rust estimator core (`crates/pid-core`) is already implemented:
  - KSG mutual information (Kraskov et al. 2004): `crates/pid-core/src/ksg.rs`
  - Continuous `I^sx_∩(S1,S2;T)` (Ehrlich et al. 2024) + cross-check test vs `csxpid`: `crates/pid-core/src/isx.rs`
  - 2-source PID atoms `{Red, Unq1, Unq2, Syn}`: `crates/pid-core/src/pid2.rs`
  - Hierarchical “fast→slow” screening (CI → selected pairwise PID; optional full 3-source SxPID): `crates/pid-core/src/hierarchy.rs`
  - Optional full 3-source continuous SxPID (18 atoms; offline only): `crates/pid-core/src/pid3.rs`
  - Discrete Shannon invariants (exact): entropies + Red°/Vul°/Ω + discrete CI in `crates/pid-core/src/invariants.rs` (see `crates/pid-core/tests/invariants.rs`)
  - Preprocessing (dependency-free): `Standardizer`, `Jitter`, `HashProjector` in `crates/pid-core/src/preprocess.rs`
  - Geometry diagnostics: intrinsic dimension + basic distance concentration proxies in `crates/pid-core/src/geometry.rs`
  - Quick Experiment 0 runner: `cargo run -p pid-core --bin exp0` (prints a small synthetic sweep + geometry diagnostics)
- Not yet built (planned next): Python experiment harness (`python/`), macOS-first VLA embedding extraction (MLX/CoreML), run logging + plots.

## Platform target (this repo)

- **Primary (do first):** macOS on Apple Silicon (M4 Max). Use **MLX / CoreML / Metal** for VLA inference + embedding extraction where applicable.
- **Secondary (later):** Linux/NVIDIA/CUDA (optional). Treat as a port once the macOS pipeline is validated and stable.

## Getting started (engineering setup)

This repo aims to be **reproducible on macOS (M4 Max) from day 1**.

**Required path (macOS-first): Nix + uv**

1. Enter the pinned dev shell:
   - `nix develop`
   - If `flake.lock` is missing, generate it once with `nix flake lock` and commit it (this is what makes Nix reproducible).
2. Sync Python dependencies (never use `pip` directly):
   - `uv sync --frozen` (uses `uv.lock` exactly)
3. Build/test:
   - `just test` (includes analytic Gaussian MI sanity checks + `csxpid` cross-checks)
   - `just exp0-bin`

Notes:
- `flake.nix` provides `rustc/cargo/rustfmt/clippy`, `python`, `uv`, and `just`.
- macOS also needs Xcode Command Line Tools for Metal/Accelerate-related work (`xcode-select --install`).
- VLA inference/embedding extraction will be macOS-first via **MLX / CoreML / Metal** (Python deps live in `uv.lock`).

**Fallback (not recommended):** install Rust + `just` + Python + `uv` manually and accept that results may not be bit-for-bit reproducible across machines.

## Repository layout (current + target)

Current repo already includes the Rust core. The remaining planned layout is:

```
pid_vla/
├── Cargo.toml
├── justfile
├── crates/
│   ├── pid-core/        # Rust: KSG + I^sx_∩ + PID atoms
│   └── pid-python/      # Rust: PyO3 bindings (optional early)
├── python/
│   ├── pid_vla/         # Python package (thin wrappers + experiments utils)
│   └── experiments/     # exp0..exp4 scripts
├── data/                # synthetic + rollouts + embeddings (local)
└── results/             # metrics + plots + logs (local)
```

## Commands (current)

- `just build` / `cargo build` — build Rust crates
- `just test` / `cargo test` — run Rust unit tests (and later Python tests)
- `just exp0` — run the Rust Experiment 0 **test suite** (synthetic checks)
- `just exp0-bin` — run the Rust quick Experiment 0 runner (`cargo run -p pid-core --bin exp0`)

## Where to look in `grandplan.md` (implementation-critical)

- `§2.2` shared-exclusions PID measure (`I^sx_∩`)
- `§2.3` continuous-variable extension
- `§8.1` KSG estimator implementation notes (Chebyshev distance, counting rules)
- `§2.5.4` hierarchical “fast→slow” strategy (co-information screening)
- `§7.3–7.4` PixelVLA + TraceVLA (optional future VLA targets)
- `§10.8.7` PixelVLA + headless Gazebo + Tauri integration (optional future)
- `§9.1` Experiment 0 protocol + GO/PIVOT/NO-GO criteria
- `Appendix B.3.3–B.3.4` reference code availability (`csxpid`) + Rust implementation sketch + validation tests + `sae_analysis` cross-validation notes

## What we are building (deliverables)

1. **`pid-core` (Rust):** continuous KSG mutual information + continuous shared-exclusions redundancy `I^sx_∩` + PID atoms.
2. **Hierarchical “fast→slow” diagnostics path (Wibral/Gutknecht-style scaling in source count):**
   - Level 1 (fast): pairwise co-information `CI(X,Y;T) = I(X;T)+I(Y;T)−I(X,Y;T)` (KSG MI only; negative CI indicates synergy).
   - Level 2 (slower): full pairwise `I^sx_∩` PID on suspicious pairs.
   - Level 3 (offline): 3-way PID only after pairwise validation.
3. **Experiment harnesses + reproducibility:** mandatory Experiment 0 validation gate, benchmarks, result logging, seeded runs.
4. **Python integration:** bindings (PyO3/maturin) + experiment scripts/notebooks for analysis and plots.
5. **VLA plumbing (macOS-first):** embedding extraction (V, L, D, A, optionally A*) on Apple Silicon using **MLX/CoreML** (and Metal where helpful) + experiments 1–4 on a chosen benchmark/dataset.
6. **(Optional, later) Real-time monitor service:** ingest embeddings during rollout, compute Level-1 metrics online, export to logs/visualization.
7. **(Optional, later) Visualization UI:** a small app/dashboard to overlay PID metrics during rollouts (implementation details in `grandplan.md` §10.8 and §11).

## Guardrails (do not skip)

- **Experiment 0 is mandatory first.** If the estimator collapses at high `d`, all downstream VLA conclusions are invalid.
- **“Synergy < 0 ⇒ hallucination” is a hypothesis, not a definition.** Treat synergy sign as a feature to be evaluated against strong baselines (entropy, Liang BATCH/CVX, learned classifier, etc.).
- **High-dimensional kNN is fragile.** Expect distance concentration at `d≈4096`; plan for PCA/random projections and re-validation.
- **Strong dependence is a separate failure mode.**
  Near-deterministic relationships can break kNN MI/PID even at low `d`.
  Include strong-dependence sweeps (Gao et al. 2015) in Experiment 0 and do not over-interpret
  MI/PID on effectively noiseless signals.
- **Any acceleration/approximation must match exact baselines.**
  Treat KD/ball trees, approximate kNN, and GPU-accelerated distance code as new estimator variants: require agreement with brute-force on analytic MI baselines + `csxpid` cross-check data, and quantify bias via an Experiment 0 subset.
- **Geometry can invalidate kNN.**
  Track intrinsic dimension and distance-concentration proxies; if intrinsic dimension remains
  high/unstable even after reduction, treat kNN-based MI/`I^sx_∩` as invalid for that regime and
  pivot to MI-only baselines (e.g., geodesic kNN MI) or Shannon invariants.
- **Liang et al. estimators are not `I^sx_∩`.** Use them as baselines, not as evidence that shared-exclusions behaves similarly.
- **macOS-first implementation.** Don’t block progress on CUDA/NixOS; treat Linux/CUDA as a later port once the M4 Max pipeline is validated.

## Technical spec (minimal but precise)

This section is intentionally self-contained; `grandplan.md` has fuller discussion and citations.

### Quantities (always report units)

All information quantities in this repo are reported in **nats** (natural log).

- Mutual information: `I(X;T)`
- Pairwise co-information (targeted): `CI(X,Y;T) = I(X;T) + I(Y;T) − I((X,Y);T)`
- Shared-exclusions redundancy (Wibral group): `I^sx_∩(S1,S2;T)` (Makkeh et al. 2021)
- 2-source PID atoms (by definition once `Red = I^sx_∩` is chosen):
  - `Unq1 = I(S1;T) − Red`
  - `Unq2 = I(S2;T) − Red`
  - `Syn  = I(S1,S2;T) − I(S1;T) − I(S2;T) + Red`

Important:
- `I^sx_∩` and the derived PID atoms are **not guaranteed non-negative**; negative values are allowed and must be representable (see `grandplan.md` and Matthias et al. 2025).

### Estimators (what this repo commits to)

- KSG MI estimator (Kraskov et al. 2004):
  - Metric: **Chebyshev / L∞** (`Metric::Chebyshev`)
  - Tie semantics: strict-radius handling (`< ε_raw`) via `strict_radius` then inclusive counts (`<= ε`)
  - Digamma `ψ(·)` (no ad-hoc `log` substitutions)
- Continuous `I^sx_∩` estimator (Ehrlich et al. 2024):
  - Uses a KSG-style construction with the **source disjunction distance**
  - Paper-faithful path is `IsxMethod::EhrlichKsg` in `crates/pid-core/src/isx.rs`

### Assumptions & failure modes (call these out in every experiment write-up)

- **i.i.d. requirement:**
  kNN estimators assume independent samples; VLA trajectories are autocorrelated. Subsample,
  block-bootstrap, or explicitly model dependence (see `grandplan.md` §1.2 Warning 5).
- **Duplicates/quantization:** exact duplicates can collapse kNN radii; fix the upstream representation first. If you must add jitter, do it explicitly, seeded, and re-validate.
- **High dimension:** distance concentration can collapse kNN behavior even when sample count is large; use intrinsic-dimension diagnostics and (if needed) explicit dimensionality reduction.
- **Strong dependence:** near-deterministic relationships can make kNN MI require prohibitive samples even at low `d` (Gao et al. 2015). This is separate from “high `d`”.
- **Non-invertible projections change the quantity:**
  PCA/projection/learned embeddings are not “free”; treat them as part of the measurement
  definition and re-run Experiment 0 at the effective dimension.
- **Do not mix estimator families inside PID identities:** e.g., don’t combine MINE MI with disjunction-kNN redundancy in `Syn = ...`.

Baselines and their assumptions (useful for “PIVOT” paths; MI/CMI-only, not `I^sx_∩`):
- **Gaussian MI from correlation**: assumes joint Gaussian/elliptical; embeddings are typically non-Gaussian → use only as a sanity baseline.
- **Local Gaussian MI (Gao et al. 2015, arXiv:1508.00536)**: assumes local neighborhoods are approximately Gaussian; can help under strong dependence but still needs validation.
- **MINE (Belghazi et al. 2018)**: neural lower bound; sensitive to training/regularization; not directly plug-compatible with `I^sx_∩`.
- **CCMI (Mukherjee et al. 2019)**: classifier-based CMI; depends on negative-sample construction and calibration; validate on synthetic conditional systems first.

## Rust core: what “done” means (minimum spec)

The Rust implementation is the long-lived foundation of this project.

- **Measure:** shared-exclusions redundancy `I^sx_∩` (Wibral group). Do not substitute other PID measures.
- **Estimator:** continuous k-NN / KSG-style estimator per Ehrlich et al. (2024).
  Cross-check against the authors’ public reference implementation (`csxpid`,
  `https://gitlab.gwdg.de/wibral/continuouspidestimator`) where possible.
- **Status note:** `isx_redundancy` has multiple estimators (`IsxMethod`).
  `IsxMethod::EhrlichKsg` matches `csxpid` on a fixed small-`d` dataset test; other methods
  (including `IsxMethod::GrandplanSketch`) are heuristic. Still treat all `I^sx_∩` results as
  **untrusted** at your target `(N,d)` until Experiment 0 validates the operating regime.
- **KSG invariants that must not drift:** use Chebyshev/L∞ for neighbor search + marginal counting; apply the documented tie/strict-inequality rule; use a real digamma `ψ(·)`.
- **Units:** pick one and stick to it (recommended: nats internally; provide explicit conversion to bits for reporting).
- **Preprocessing is explicit:** standardization and any dimensionality reduction must be recorded
  with results; do not silently change dimensionality. (`pid-core` currently provides
  `Standardizer`, `Jitter`, and a dependency-free `HashProjector` baseline.)
- **Atom formulas (2-source PID):**
  - `Unq1 = I(S1;T) − Red`
  - `Unq2 = I(S2;T) − Red`
  - `Syn  = I(S1,S2;T) − I(S1;T) − I(S2;T) + Red`
- **Important:** `I^sx_∩` PID atoms (especially synergy) are not guaranteed non-negative; negative values must be representable and tested (this is not automatically a bug).

Implementation details (modules, API shape, preprocessing rules, kNN backend rules, validation obligations) are documented in `AGENTS.md`.

Suggested `pid-core` internal layout (so work can parallelize cleanly):
- `ksg.rs` — KSG mutual information
- `isx.rs` — continuous `I^sx_∩` redundancy estimator
- `pid2.rs` — 2-source PID wrapper (`{Red, Unq1, Unq2, Syn}`)
- `preprocess.rs` — standardization + jitter + hash projection (PCA later; explicit + logged)
- `nn.rs` — kNN backend abstraction (brute-force baseline first)
- `stats.rs` — digamma + bootstrap/CI utilities

## Roadmap (next engineering milestones)

The Rust estimator core (`pid-core`) is already implemented in this repo. The remaining critical work is to build the **Python experiment harness + macOS-first VLA embedding extraction**, then run Experiment 0 at VLA-relevant regimes (including geometry diagnostics + dimensionality-reduction pivots) before making any VLA claims.

- **Python harness / bindings**: PyO3/maturin (or thin wrapper) so experiments can call `pid-core` and log full provenance (configs, seeds, transforms).
- **macOS VLA embedding extraction**: MLX/CoreML pipeline + a stable on-disk format for `(V,L,D,A[,A*])`.
- **Dimensionality reduction pipeline (Python-first)**: PCA (variance target) + projection baselines; enforce train-only fitting to prevent leakage.
- **Performance (later)**: SIMD/rayon; optional exact low-d trees; explicit “approx” modes only after exact-baseline regression tests + Experiment 0 subset bias quantification.

## Experiments (actionable, step-by-step)

The experiments below are the “engineering contract” for reaching the `grandplan.md` goals.

### Experiment 0 (mandatory gate): estimator validation at scale

Purpose:
- Establish whether kNN/KSG-based MI and the continuous `I^sx_∩` estimator are trustworthy at the intended operating regime (ambient/intrinsic dimension and dependence strength).

Run now (Rust smoke subset):
1. `just test`
2. `just exp0` (runs the Rust Experiment 0-related tests)
3. `just exp0-bin` (prints a small synthetic sweep, geometry diagnostics, and a strong-dependence Gaussian-channel sweep)

What to implement next (Python full harness; see `grandplan.md` §9.1):
- Grid sweep over `{n, d, k, seeds}` with fixed synthetic generators:
  - independent/additive
  - redundant/copy
  - unique-only
  - XOR-like / interaction-only
- Separate axis: **strong-dependence** sweep at fixed small `d` (e.g., Gaussian channel with decreasing noise), comparing to analytic MI.
- Geometry diagnostics:
  - intrinsic dimension (implemented in Rust)
  - distance concentration proxies (basic ones implemented in Rust; expand if needed)

Acceptance criteria:
- Use the GO / PIVOT / NO-GO thresholds from `grandplan.md` §9.1.
- Record: bias/error, variance across seeds, runtime/memory, and all estimator/preprocessing config.

### Experiment 1: decomposition comparison (diagnostic signal)

Question:
- Which decomposition is most predictive of failures on real rollouts: `(V,D)→A*`, `(V,L,D)→A*`, or hierarchical pairwise screens?

Steps:
1. Define the sampling unit (per-timestep vs per-trajectory window) and the target label (`A*` or success/failure).
2. Extract embeddings on macOS (MLX/CoreML) with full provenance (model id, layer, normalization).
3. Run Level-1 CI screening across candidate pairs/windows; then Level-2 `I^sx_∩` PID on the selected subset.
4. Evaluate predictive power vs baselines (AUROC, calibration), using paired bootstrap across trajectories.

### Experiment 2: baseline comparison (does `I^sx_∩` add value?)

Question:
- Does `I^sx_∩` synergy/redundancy provide statistically significant improvement over strong baselines?

Baselines (minimum):
- entropy/uncertainty features (model confidence, logit entropy, etc.)
- Liang et al. BATCH/CVX estimators (baseline only; not `I^sx_∩`)
- a supervised classifier on the same embeddings (to bound achievable prediction)

Acceptance:
- preregistered metric (AUROC) and significance test (paired bootstrap, p < 0.05), with leakage controls (fit any projections on train only).

### Experiment 3: dimensionality study (representation + reduction)

Question:
- Which representation makes kNN/PID viable and stable?

Steps:
1. Compare raw embeddings vs intermediate layers vs explicit reductions (PCA / random projection / learned projection).
2. For every non-invertible transform, rerun a subset of Experiment 0 to quantify estimator drift.
3. Report intrinsic dimension estimates and any distance-concentration proxies alongside PID results.

### Experiment 4: causal validation (interventions)

Question:
- Do controlled interventions on `D` change PID terms in the predicted direction and correlate with improved outcomes?

Steps:
1. Design interventions + placebo controls (same action budget, no semantic change).
2. Run paired rollouts (same initial states/seeds) with/without interventions.
3. Test whether measured changes in synergy/redundancy predict changes in failure rates beyond baselines.

## Sources (papers + reference code)

Authoritative papers:
- Makkeh, Gutknecht, Wibral (2021) — *Phys Rev E* 103:032149. `I^sx_∩` definition. DOI: `https://doi.org/10.1103/PhysRevE.103.032149`
- Ehrlich et al. (2024) — *Phys Rev E* 110:014115. Continuous `I^sx_∩` estimator. DOI: `https://doi.org/10.1103/PhysRevE.110.014115`
- Kraskov, Stögbauer, Grassberger (2004) — *Phys Rev E* 69:066138. KSG MI estimator. DOI: `https://doi.org/10.1103/PhysRevE.69.066138`
- Gutknecht et al. (2025) — arXiv:2504.15779. Shannon invariants / scalable decomposition. `https://arxiv.org/abs/2504.15779`
- Matthias et al. (2025) — arXiv:2512.16662. Why negative PID atoms can occur (impossibility/inconsistency results). `https://arxiv.org/abs/2512.16662`
- PixelVLA (2025) — arXiv:2511.01571. Pixel-level understanding + visual prompting for VLAs (optional future). `https://arxiv.org/abs/2511.01571`
- TraceVLA (2024) — arXiv:2412.10345. Visual trace prompting for spatial-temporal awareness (optional future). `https://arxiv.org/abs/2412.10345`

MI/CMI estimation references (baselines; not `I^sx_∩`):
- Gao, Ver Steeg, Galstyan (2015) — sample complexity pathologies for kNN MI under strong dependence. arXiv:1411.2003. `https://arxiv.org/abs/1411.2003`
- Gao, Ver Steeg, Galstyan (2015) — local Gaussian MI estimator (strong dependence correction). arXiv:1508.00536. `https://arxiv.org/abs/1508.00536`
- Belghazi et al. (2018) — MINE (neural MI; lower-bound-style; treat as separate validated pipeline). arXiv:1801.04062. `https://arxiv.org/abs/1801.04062`
- Mukherjee, Asnani, Kannan (2019) — CCMI (classifier-based conditional MI; for conditioning-heavy baselines). arXiv:1906.01824. `https://arxiv.org/abs/1906.01824`

Differential geometry / manifold contingencies (MI-only baselines; not `I^sx_∩`):
- Marx, Fischer (2021) — geodesic kNN MI on Riemannian manifolds (useful if embeddings appear curved/manifold-valued). arXiv:2110.13883. `https://arxiv.org/abs/2110.13883`
- Nickel, Kiela (2017) — Poincaré embeddings for hierarchical representations (optional learned projection). arXiv:1705.08039. `https://arxiv.org/abs/1705.08039`
- Nickel, Kiela (2018) — Lorentz (hyperboloid) model for hyperbolic hierarchies (optional learned projection). arXiv:1806.03417. `https://arxiv.org/abs/1806.03417`
- Ganea, Bécigneul, Hofmann (2018) — Hyperbolic Neural Networks (optional background). arXiv:1805.09112. `https://arxiv.org/abs/1805.09112`

Reference repos (baselines/sanity checks; not the same estimator unless noted):
- `https://gitlab.gwdg.de/wibral/continuouspidestimator` — authors’ reference implementation of the continuous `I^sx_∩` kNN estimator (Ehrlich et al. 2024); primary cross-check target for `pid-core`.
- `https://github.com/Abzinger/SxPID` — discrete `I^sx_∩` (definitions/lattice sanity).
- `https://github.com/Abzinger/sae_analysis` — WIP toolbox for information-theoretic SAE analysis
  (Shannon-invariants-style redundancy/vulnerability from Gutknecht et al. 2025); not a
  continuous `I^sx_∩` implementation; treat as a reference/starting point only.
- `https://github.com/pliang279/PID` — Liang et al. BATCH/CVX estimators (baseline; NOT `I^sx_∩`).
- `https://github.com/pwollstadt/IDTxl` — information dynamics toolkit (baseline ideas/cross-checks).
