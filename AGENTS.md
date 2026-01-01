# AGENTS.md

This file is for Codex CLI agents and contributors working in this repo.

Canonical spec: `grandplan.md`.

## Step-by-step plan (project roadmap)

1. **Implement Wibral PID in Rust (I^sx_∩, continuous; “latest” in spec = Makkeh et al. 2021 + Ehrlich et al. 2024):**
   - Create a Rust core library (`pid-core`) implementing KSG MI, continuous shared-exclusions redundancy I^sx_∩, and the derived 2-source PID atoms.
   - Prioritize: correctness, numerical stability, reproducibility (seeded RNG, pinned deps), and a clean API that can later support 3-way PID and alternative estimators.
   - Include validation tests mirroring Experiment 0 scenarios (independent/additive, XOR-like synergy, redundant/copy, scaling).
2. **Adopt a hierarchical computation design (sound + adaptable):**
   - Level 1: fast screening via Shannon invariants (pairwise co-information from KSG MI).
   - Level 2: targeted full pairwise I^sx_∩ PID on suspicious pairs.
   - Level 3: optional 3-way PID (offline only) once pairwise behavior is understood.
3. **Implement dimensionality reduction + preprocessing hooks:**
   - Standardization/whitening; optional **invertible reparameterizations** (e.g., monotone marginal Gaussianization) to improve kNN geometry without changing true MI; PCA (retain variance target); random projections; interfaces for learned projections.
   - Add **geometry diagnostics** (intrinsic dimension + distance concentration) to decide whether kNN-based MI/`I^sx_∩` is even plausible in the chosen representation.
   - Make dim reduction explicit and logged so results are interpretable/reproducible.
4. **Build experiment harnesses and reproducibility scaffolding:**
   - Add a simple task runner (e.g., `justfile`) and/or scripts to run experiments deterministically.
   - Add benchmarking (runtime vs N,d,k; memory) to enforce “real-time” viability for Level 1.
5. **Experiment 0 gate (do before any VLA claims):**
   - Run synthetic validation across {N,d,k} grids up to VLA-like d (or demonstrate why dim reduction is required).
   - Include **strong-dependence sweeps** (near-deterministic/large-MI regimes) as a separate axis from “high `d`” (see Gao et al. 2015); do not treat low-`d` success as proof of robustness under strong dependence.
   - Include **geometry diagnostics** as a separate axis: intrinsic-dimension estimates and distance-concentration proxies; treat “kNN works after PCA” as unproven unless intrinsic dimension is also low/stable.
   - Record error/variance/cost; decide GO / PIVOT / NO-GO using the thresholds in `grandplan.md` (e.g., error targets: d=10 <5%, d=100 <10%, d=1000 <15%, d=4096 <20% or require dim reduction).
   - Decision rule of thumb from spec: **GO** if stable at d=4096; **PIVOT** if stable only after PCA/random projection (e.g., d≈256); **NO-GO** if unstable even at d≈256 (abandon I^sx_∩ for alternatives).
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
- **Makkeh A, Gutknecht AJ, Wibral M (2021)** — *Phys Rev E* 103:032149. “Introducing a differentiable measure of pointwise shared information.” Defines `I^sx_∩`. DOI: `https://doi.org/10.1103/PhysRevE.103.032149`
- **Ehrlich DA, Schick-Poland K, Makkeh A, Lanfermann F, Wollstadt P, Wibral M (2024)** — *Phys Rev E* 110:014115. “Partial Information Decomposition for Continuous Variables based on Shared Exclusions.” Continuous `I^sx_∩` estimator details. DOI: `https://doi.org/10.1103/PhysRevE.110.014115`
- **Kraskov A, Stögbauer H, Grassberger P (2004)** — *Phys Rev E* 69:066138. “Estimating mutual information.” KSG estimator details (max-norm usage, neighbor counting, bias/variance behavior). DOI: `https://doi.org/10.1103/PhysRevE.69.066138`

Context/guardrails:
- **Gutknecht AJ, Rosas FE, Ehrlich DA, Makkeh A, Mediano PAM, Wibral M (2025)** — arXiv:2504.15779. “Shannon Invariants: A Scalable Approach to Information Decomposition.” `https://arxiv.org/abs/2504.15779`
- **Matthias PH, Makkeh A, Wibral M, Gutknecht AJ (2025)** — arXiv:2512.16662. Impossibility/inconsistency results (explains why `I^sx_∩` can have negative atoms). `https://arxiv.org/abs/2512.16662`
- **Gao S, Ver Steeg G, Galstyan A (2015)** — arXiv:1411.2003. Strong-dependence sample complexity pathologies for kNN MI estimators (relevant to “near-deterministic” VLA variables). `https://arxiv.org/abs/1411.2003`
- **Gao S, Ver Steeg G, Galstyan A (2015)** — arXiv:1508.00536. Local Gaussian MI estimator (strong-dependence correction / MI baseline). `https://arxiv.org/abs/1508.00536`
- **Belghazi MI et al. (2018)** — arXiv:1801.04062. MINE (neural MI; treat as a separate validated MI pipeline for MI-only screening, not drop-in `I^sx_∩`). `https://arxiv.org/abs/1801.04062`
- **Mukherjee S, Asnani H, Kannan S (2019)** — arXiv:1906.01824. CCMI (classifier-based conditional MI; useful if conditioning becomes central; separate validated pipeline). `https://arxiv.org/abs/1906.01824`
- **Marx A, Fischer J (2021)** — arXiv:2110.13883. Geodesic kNN MI estimation on Riemannian manifolds (MI-only baseline if embeddings are curved/manifold-valued). `https://arxiv.org/abs/2110.13883`
- **Nickel M, Kiela D (2017)** — arXiv:1705.08039. Poincaré embeddings (hyperbolic geometry for hierarchies; optional learned projection). `https://arxiv.org/abs/1705.08039`
- **Nickel M, Kiela D (2018)** — arXiv:1806.03417. Lorentz (hyperboloid) model hyperbolic embeddings (often more stable than Poincaré ball). `https://arxiv.org/abs/1806.03417`
- **Ganea O-E, Bécigneul G, Hofmann T (2018)** — arXiv:1805.09112. Hyperbolic Neural Networks (background). `https://arxiv.org/abs/1805.09112`
- **Local note (conceptual only):** `Information Theory Meets Differential Geometry.pdf` (do not treat as an estimator spec or correctness oracle).
- **Liang PP et al. (2023)** — NeurIPS 2023. Multimodal “PID” estimators (BATCH/CVX) for baselines; not the same as `I^sx_∩`. Code: `https://github.com/pliang279/PID`
- **PixelVLA (2025)** — arXiv:2511.01571. Pixel-level understanding + visual prompting for VLAs (optional future target; see `grandplan.md` §7.3). `https://arxiv.org/abs/2511.01571`
- **TraceVLA (2024)** — arXiv:2412.10345. Visual trace prompting for spatial-temporal awareness (optional future target; see `grandplan.md` §7.4). `https://arxiv.org/abs/2412.10345`

In-repo pointers (use these to stay aligned with the spec):
- `grandplan.md` §2.2 (`I^sx_∩` definition), §2.3 (continuous extension), §8.1 (KSG details), §2.5.4 (hierarchical strategy), §9.1 (Experiment 0), Appendix B.3.4 (Rust estimator sketch + validation tests).
  - Current code locations: `crates/pid-core/src/ksg.rs` (KSG MI + local terms), `crates/pid-core/src/isx.rs` (`I^sx_∩` candidates via `IsxMethod`), `crates/pid-core/src/pid2.rs` (PID atoms wrapper), `crates/pid-core/src/ci.rs` (co-information), `crates/pid-core/src/nn.rs` (brute-force kNN helpers), `crates/pid-core/src/preprocess.rs` (standardization), `crates/pid-core/src/geometry.rs` (intrinsic-dimension diagnostic).

Reference code (for sanity checks and baselines; verify commit hashes when used):
- **Continuous `I^sx_∩` (authors’ reference impl):** `https://gitlab.gwdg.de/wibral/continuouspidestimator` (Python package `csxpid`; Ehrlich et al. 2024; uses KDTree/ball-tree variants + merging procedure for disjunction distances)
- **Discrete `I^sx_∩` (definitions/lattice sanity):** `https://github.com/Abzinger/SxPID`
- **Related (Shannon invariants on neural latents; NOT `I^sx_∩`):** `https://github.com/Abzinger/sae_analysis` (WIP; computes redundancy/vulnerability-style invariants for SAEs; uses submodules `sparsify`/`delphi`; treat as reference only)
- **Baseline estimators (NOT `I^sx_∩`):** `https://github.com/pliang279/PID` (Liang BATCH/CVX)
- **General info-dynamics toolkit:** `https://github.com/pwollstadt/IDTxl`

Quick comparison (to avoid false equivalences):
- `Abzinger/sae_analysis` is **discrete PMF / plug-in entropy**-based and targets **SAE latents** + **Shannon invariants** (degree of redundancy/vulnerability). It does **not** implement continuous KSG MI or continuous shared-exclusions `I^sx_∩`.
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

Rust workspace layout (minimum viable):
- `crates/pid-core/`
  - `src/lib.rs` (public API surface)
  - `src/ksg.rs` (KSG MI estimator)
  - `src/isx.rs` (continuous `I^sx_∩` redundancy estimator)
  - `src/pid2.rs` (2-source PID wrapper: calls KSG + `I^sx_∩`)
  - `src/preprocess.rs` (standardize/whiten; PCA/random projection hooks)
  - `src/metrics.rs` (L∞/L2 distance, tie handling)
  - `src/nn.rs` (kNN backend abstraction: brute force baseline + optional trees/ANN later)
  - `src/stats.rs` (digamma, bootstrap utilities, CI helpers)

Public API sketch (keep stable; allow internal refactors):
- `fn ksg_mi(x: ArrayView2<f64>, y: ArrayView2<f64>, cfg: &KsgConfig) -> Result<f64>`
- `fn isx_redundancy(s1: ArrayView2<f64>, s2: ArrayView2<f64>, t: ArrayView2<f64>, cfg: &IsxConfig) -> Result<f64>`
- `fn pid2_isx(s1: ArrayView2<f64>, s2: ArrayView2<f64>, t: ArrayView2<f64>, cfg: &Pid2Config) -> Result<Pid2Result>`
- `struct Pid2Result { redundancy, unique_s1, unique_s2, synergy, se/ci optional, meta }`

Implementation note: exact container types (`ndarray`, `nalgebra`, raw slices) are flexible, but keep a single canonical “row-major (n×d) float” convention and enforce it everywhere.

### Preprocessing + dimensionality reduction rules (avoid silent bugs)

- Always apply **the same preprocessing pipeline** to all variables involved in a computation, but **fit transforms without mixing variables**:
  - Fit/transform `S1`, `S2`, and `T` independently (no PCA on `[S1|S2|T]` concatenations).
  - Log/serialize the fitted transform (mean/std; PCA components; random projection matrix seed).
- Default preprocessing (minimum):
  - Per-dimension standardization (zero mean, unit variance) or min-max scaling.
  - Optional: monotone marginal Gaussianization (rank/CDF→Normal) as an **invertible** per-dimension reparameterization to improve kNN geometry (does not change true MI when applied per variable; still re-validate estimator behavior).
  - Optional: small jitter for duplicate points (seeded) to avoid kNN tie pathologies; record when enabled.
- Dimensionality reduction (only after Experiment 0 justifies it):
  - PCA: pick variance-retained target (e.g., 95%) or fixed component count; record the achieved dimension.
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
- Implement from the paper and **cross-check against the authors’ reference implementation** (`csxpid`, `gitlab.gwdg.de/wibral/continuouspidestimator`; vendored at `.external/repos/continuouspidestimator`); treat Appendix B.3.4 in `grandplan.md` as a sketch, not as a proof of correctness.
- Keep the estimator factored so we can swap kNN backends (exact vs approximate) without changing math.

High-dimensional regime handling:
- Expect **distance concentration** and estimator collapse at large `d`; do not hide this—detect it and trigger the Experiment 0 “PIVOT” path (dim reduction).
- Default approach: PCA to ~256 dims (variance retained target) + rerun Experiment 0 to re-establish accuracy.
- Strong dependence is a separate pathology from high `d`: large true MI (near-deterministic mappings) can break kNN MI/PID at low `d` unless sample sizes are enormous (Gao et al. 2015). Treat “noiseless” signals with extreme caution.
- Do not mix estimator families inside PID identities (e.g., do not combine MINE MI terms with disjunction-kNN redundancy in `Syn = I(S1,S2;T) − I(S1;T) − I(S2;T) + Red`).

### Result reporting (make downstream experiments reproducible)

Every experiment output that depends on the Rust estimator should record:
- estimator versions (crate git rev / crate version), configs (`k`, metric, log base, preprocessing, dim reduction),
- environment (OS/arch, `rustc --version`, BLAS/GPU backend if applicable),
- sample sizes and effective dimensions after reduction,
- random seeds for any stochastic step (jitter, bootstrap, random projection),
- warnings/diagnostics (e.g., distance concentration indicators, excessive ties, NaNs clamped/filtered).

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
