# pid_vla

Engineering roadmap for implementing and validating Wibral-group shared-exclusions PID (`I^sx_∩`) for Vision-Language-Action (VLA) diagnostics.

This repo currently contains the research specification (`grandplan.md`). The engineering goal is to turn that spec into a reproducible, validated implementation + experiment suite.

## Platform target (this repo)

- **Primary (do first):** macOS on Apple Silicon (M4 Max). Use **MLX / CoreML / Metal** for VLA inference + embedding extraction where applicable.
- **Secondary (later):** Linux/NVIDIA/CUDA (optional). Treat as a port once the macOS pipeline is validated and stable.

## Getting started (engineering setup)

Minimum tools:
- Rust toolchain (`rustc`, `cargo`) + `rustfmt` and `clippy`
- Python 3.11+ (for experiment orchestration)

Recommended tools (once code exists):
- `just` (task runner) for `build`, `test`, `exp0`, …
- PyO3 + `maturin` (to expose Rust to Python)
- macOS: Xcode Command Line Tools (Metal toolchain) + optional Accelerate usage for linear algebra
- macOS: MLX + CoreML tooling (for VLA inference / embedding extraction on Apple Silicon)
- Optional: Nix (`flake.nix`) for fully pinned environments (more relevant for Linux/CUDA)

## Target repository layout (what to create)

The grandplan assumes a Rust core with Python experiment glue. A reasonable target layout:

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

## Expected commands (once scaffolded)

After M0, the repo should expose a small, stable set of commands:
- `just build` / `cargo build` — build Rust crates
- `just test` / `cargo test` — run Rust unit tests (and later Python tests)
- `just exp0` — run Experiment 0 validation (the gate before any VLA experiments)
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

1. **`pid-core` (Rust):** continuous KSG mutual information + continuous shared-exclusions redundancy `I^sx_∩` + 2-source PID atoms `{Red, Unq1, Unq2, Syn}`.
2. **Hierarchical “fast→slow” diagnostics path:**
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
- **Strong dependence is a separate failure mode.** Near-deterministic relationships can break kNN MI/PID even at low `d`; include strong-dependence sweeps (Gao et al. 2015) in Experiment 0 and do not over-interpret MI/PID on effectively noiseless signals.
- **Liang et al. estimators are not `I^sx_∩`.** Use them as baselines, not as evidence that shared-exclusions behaves similarly.
- **macOS-first implementation.** Don’t block progress on CUDA/NixOS; treat Linux/CUDA as a later port once the M4 Max pipeline is validated.

## Rust core: what “done” means (minimum spec)

The Rust implementation is the long-lived foundation of this project.

- **Measure:** shared-exclusions redundancy `I^sx_∩` (Wibral group). Do not substitute other PID measures.
- **Estimator:** continuous k-NN / KSG-style estimator per Ehrlich et al. (2024). Cross-check against the authors’ public reference implementation (`csxpid`, `https://gitlab.gwdg.de/wibral/continuouspidestimator`) where possible.
- **Status note:** `isx_redundancy` has multiple estimators (`IsxMethod`). `IsxMethod::EhrlichKsg` matches `csxpid` on a fixed small-d dataset test; other methods (e.g., the `grandplan.md` Appendix B.3.4 sketch) are heuristic. Still treat all `I^sx_∩` results as **untrusted** at your target `(N,d)` until Experiment 0 validates the operating regime.
- **KSG invariants that must not drift:** use Chebyshev/L∞ for neighbor search + marginal counting; apply the documented tie/strict-inequality rule; use a real digamma `ψ(·)`.
- **Units:** pick one and stick to it (recommended: nats internally; provide explicit conversion to bits for reporting).
- **Preprocessing is explicit:** standardization + (if used) dimensionality reduction must be recorded with results; do not silently change dimensionality. (`pid-core` currently provides `Standardizer`, `Jitter`, and a dependency-free `HashProjector` baseline.)
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

## Engineering plan (milestones)

M0. **Scaffold the project**
- Create a Cargo workspace with `crates/pid-core/` (and later `crates/pid-python/`).
- Add a task runner (`justfile`) with at least `build`, `test`, `exp0`.
- Acceptance: `cargo test` runs locally; deterministic seed plumbing exists.

M1. **Implement KSG mutual information (Rust)**
- Implement KSG MI with correct metric + tie handling + digamma; add unit tests.
- Acceptance: matches a known-good small-d reference within tolerance; stable across seeds.

M2. **Implement continuous `I^sx_∩` redundancy (Rust)**
- Implement continuous shared-exclusions redundancy per Ehrlich et al. (2024), factored so kNN backend can be swapped later.
- Acceptance: passes Experiment 0 synthetic scenarios at low dimension and does not exhibit obvious numerical pathologies.

M3. **Implement 2-source PID wrapper + invariants checks**
- Combine `I(S1;T)`, `I(S2;T)`, `I(S1,S2;T)`, and `I^sx_∩` into `{Red, Unq1, Unq2, Syn}` with optional bootstrap SE/CI.
- Acceptance: internal consistency checks pass (`MI ≈ Red+Unq1+Unq2+Syn` within tolerance).

M4. **Experiment 0 (mandatory gate)**
- Run synthetic validation across `{d,n,k}` (including “VLA-like” d, or demonstrate collapse and pivot to dim reduction).
- Acceptance (from spec): d=10 (<5% error), d=100 (<10%), d=1000 (<15%), d=4096 (<20% *or* require dim reduction).
- Decision: **GO** if stable at d=4096; **PIVOT** if only stable after PCA/random projection (e.g., d≈256); **NO-GO** if unstable even at d≈256.

M5. **Python bindings + experiment harness**
- Expose Rust to Python (PyO3/maturin) and implement repeatable experiment runners that record full configs + seeds.
- Acceptance: Python can call `pid2_isx` and reproduce Experiment 0 results.

M6. **VLA data + Experiments 1–4**
- Implement embedding extraction + dataset interfaces on macOS (prefer MLX/CoreML); run decomposition comparison, baseline comparison, dimensionality study, and causal intervention study.
- Acceptance: preregistered metrics computed; AUROC + significance tests implemented; full provenance recorded.

M7. **(Optional) Real-time monitoring integration**
- Build a Rust “PID monitor” process that consumes embeddings from the inference stack (or logs) and computes Level-1 co-information online.
- Acceptance: bounded latency and stable output on representative rollouts; logs include full config + provenance.

M8. **(Optional) Visualization**
- Add a lightweight visualization surface (e.g., Tauri/WebView or simple web dashboard) to inspect trajectories and PID metrics.
- Acceptance: can replay rollouts and overlay metrics for debugging/analysis without changing estimator semantics.

## Experiments (what to run and why)

- **Experiment 0 (mandatory first): Estimator validation at scale.** Synthetic systems embedded into increasing dimensionality. Measure error/variance/runtime/memory; apply GO/PIVOT/NO-GO.
- **Experiment 1: Decomposition comparison.** V-D-A vs V-L-A vs V-D-A* vs hierarchical pairwise for failure prediction.
- **Experiment 2: Baseline comparison.** Compare `I^sx_∩` synergy vs entropy/uncertainty baselines plus Liang et al. (BATCH/CVX); success requires statistically significant AUROC improvement (paired bootstrap, p < 0.05).
- **Experiment 3: Dimensionality study.** Raw vs PCA vs random projection vs learned projection vs intermediate-layer embeddings.
- **Experiment 4: Causal validation.** Intervene on D and test predicted synergy changes + failure-rate changes.

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

Reference repos (baselines/sanity checks; not the same estimator unless noted):
- `https://gitlab.gwdg.de/wibral/continuouspidestimator` — authors’ reference implementation of the continuous `I^sx_∩` kNN estimator (Ehrlich et al. 2024); primary cross-check target for `pid-core`.
- `https://github.com/Abzinger/SxPID` — discrete `I^sx_∩` (definitions/lattice sanity).
- `https://github.com/Abzinger/sae_analysis` — WIP toolbox for information-theoretic SAE analysis (Shannon-invariants-style redundancy/vulnerability from Gutknecht et al. 2025); not a continuous `I^sx_∩` implementation; treat as a reference/starting point only.
- `https://github.com/pliang279/PID` — Liang et al. BATCH/CVX estimators (baseline; NOT `I^sx_∩`).
- `https://github.com/pwollstadt/IDTxl` — information dynamics toolkit (baseline ideas/cross-checks).
