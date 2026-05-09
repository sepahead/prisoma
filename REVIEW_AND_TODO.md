# PID-VLA Whole-Repo Review and To-Do List

Date: 2026-05-09

This document records a whole-repo review of the PID-VLA repository from ten scientific/engineering perspectives, followed by a prioritized to-do list. It is intentionally direct and conservative: it distinguishes implemented functionality from specified/planned architecture and prioritizes scientific validity over roadmap optimism.

**Docset-wide final solution:** `grandplan.md` §A.8 records the current ten-scientist consensus. The run log is the source of truth, Rerun is the Phases 1–3 diagnostic/time-machine viewer, Agent Bridge is the only control plane, and Tauri/SparkJS is the deferred Phase 4 control/editor/custom-rendering shell.

## Executive Verdict

The repository is strongest as a **research specification plus a small, serious Rust estimator core**. The documentation is unusually self-critical and correctly treats PID-on-VLA as a gated hypothesis rather than a proven method. The implemented `pid-core` crate has meaningful tests and reference-value checks, and `cargo test --workspace` passed during review.

The main risks are:

1. The implemented Experiment 0 gate is weaker than the documented scientific gate.
2. Rust formatting and Clippy checks currently fail.
3. Python packaging/bindings are local-development oriented, not yet a robust user-facing API.
4. `pid-rerun` is prototype-level, and one demo logs fixed PID proportions rather than actual PID atoms.
5. `meshmaker/` is operationally and ethically high-risk clutter relative to the PID-VLA research core.
6. The docs are mostly honest, but still contain minor drift and an overlarge target-architecture surface area.

No tracked files were changed during the review itself. Running `uv run ruff ...` created an ignored `.venv/` directory.

## Verification Snapshot

Commands and outcomes observed during the review:

- `cargo test --workspace -q`: passed.
- `python scripts/audit_grandplan.py --check-italic-titles`: passed.
- `python scripts/audit_grandplan_claims.py`: passed.
- `python scripts/audit_docset_claims.py`: passed.
- `cargo fmt --all -- --check`: failed due Rust formatting drift.
- `cargo clippy --workspace -- -D warnings`: failed on `clippy::neg_cmp_op_on_partial_ord` in `crates/pid-core/src/hyperbolic.rs`.
- `uv run ruff check scripts uidesigner meshmaker report_gen.py`: failed with 102 errors, mostly in `meshmaker/` and ignored `report_gen.py`.

## Ten-Perspective Review

### 1. Information Theorist

#### Strengths

- The repo correctly separates the distribution-level quantity from finite-sample estimators.
- The docs explicitly warn against the tempting but false claim that negative synergy literally means conflict or hallucination.
- Shared-exclusions PID is documented with the right high-level estimator form and counting convention.

#### Concerns

- The default KSG configuration clamps negative MI estimates to zero, which is practical for reports but risky for algebraic cancellation, co-information, and invariant checks.
- `Pid2Config` permits independent KSG and ISX configs without validating that `k`, metric, and tie handling are coherent.
- `pid3_isx` exposes a metric field but lacks the Chebyshev-only guard present in the two-source ISX path.

#### Judgment

The theoretical framing is good. The API should make invalid estimator combinations harder to express.

### 2. Statistician / Estimator Scientist

#### Strengths

- The docs explicitly identify high dimension, strong dependence, and trajectory autocorrelation as estimator failure modes.
- Tests cover Gaussian MI approximations, strong-dependence smoke tests, co-information checks, duplicate rejection, and fixed-data reference values.
- `findings.md` honestly reports current estimator incoherence and high-dimensional collapse.

#### Concerns

- The documented Experiment 0 acceptance criteria are strong, but the implemented `exp0` status only applies a weaker independent-additive redundancy threshold plus heuristic geometry warnings.
- `gromov_hyperbolicity` reports mean sampled delta, while gating usually needs worst-case or high-quantile behavior.
- There is no implemented uncertainty quantification, block bootstrap, or seed sweep in the executable gate.

#### Judgment

The repo knows what the scientific gates should be, but the executable gate is not yet the scientific gate.

### 3. Robotics Experimentalist

#### Strengths

- The docs correctly require external targets, teacher actions, labels, interventions, or counterfactuals before making grounding claims.
- The recommended build order is sensible: Experiment 0, run logs/replay, bridge API, minimal simulator/ground-truth flow, Rerun viewer, then embeddings.

#### Concerns

- The simulator, Agent Bridge, run-log/replay system, and actual VLA embedding harness are still mostly planned rather than implemented.
- `assets/` and `experiments/` are mostly scaffolding; a fresh clone cannot run later-stage experiments.
- The repo needs a minimal deterministic manipulation task before expanding into the larger architecture.

#### Judgment

Do not proceed to VLA claims yet. Build the early milestones before making new scientific conclusions.

### 4. VLA / ML Scientist

#### Strengths

- The hypothesis registry is falsifiable and baseline-aware rather than PID-triumphalist.
- The Flow-as-Bridge idea is a promising way to avoid raw high-dimensional embedding geometry problems.

#### Concerns

- There is no implemented model integration, embedding extraction, layer contract, dataset loader, failure-label pipeline, or baseline classifier.
- `pid-rerun` synthetic demos are useful visually but are not ML evidence.
- The project should define one first VLA baseline and one first task instead of keeping many optional model branches open.

#### Judgment

The ML research question is still upstream of data collection. The next high-value step is a tiny, logged baseline with real labels and strong non-PID baselines.

### 5. Rust / Software Engineer

#### Strengths

- `pid-core` is compact, mostly safe, and forbids unsafe code.
- The code uses explicit error types and finite-input validation at matrix construction.
- The Rust test suite passes.

#### Concerns

- `cargo fmt --check` fails.
- `cargo clippy --workspace -- -D warnings` fails in `hyperbolic.rs`.
- Hyperbolic distance validity relies mostly on caller discipline; invalid Lorentz rows can yield `NaN`, and KSG paths do not robustly reject non-finite distances.
- `Pid3Config` should forbid non-Chebyshev metrics until mathematically validated.

#### Judgment

`pid-core` is the repo's strongest implemented component, but CI-level hygiene and invalid-configuration guards need tightening.

### 6. Python Packaging / API Scientist

#### Strengths

- PyO3 bindings expose basic MI, redundancy, intrinsic dimension, hyperbolicity, and distance-statistics functions.

#### Concerns

- `pyproject.toml` does not wire a maturin or scikit-build style backend for the Rust extension.
- Python users cannot set `negative_handling` or `tie_epsilon`, so they cannot reproduce the Rust tests' estimator identity settings.
- There are no Python tests for the extension.
- The Python API lacks high-level `pid2`, co-information, invariants, and preprocessing bindings.

#### Judgment

The Python bindings are useful for local experiments but are not yet a stable public API.

### 7. Visualization / HCI Scientist

#### Strengths

- The Rerun-first strategy is pragmatic and avoids premature custom UI work.
- Entity paths are centralized in `pid-rerun`.

#### Concerns

- `vla_demo` logs PID atoms as fixed proportions of MI; this must stay unmistakably demo-only until it computes actual PID.
- `VlaEpisode` assumes consistent embedding/action dimensions and may panic on malformed frames.
- The viewer assumes the external `rerun` CLI is installed.

#### Judgment

The visualization direction is good, but the demo should not look like real diagnostics until it computes real PID.

### 8. Reproducibility / MLOps Scientist

#### Strengths

- Docs consistently emphasize offline-first logs, replay, hashes, and a single control plane.
- Existing doc-audit scripts are useful and passed during review.

#### Concerns

- The docs reference `flake.lock`, but no `flake.lock` is tracked.
- `AGENTS.md` says the current docset is v10.0 while `README.md` and `CHANGELOG.md` say v10.1.
- There is no CI configuration.
- Experiment 0 outputs are not saved/versioned by default.

#### Judgment

The reproducibility plan is good; the reproducibility implementation is still mostly future work.

### 9. Security / Safety / Governance Reviewer

#### Strengths

- The intended Agent Bridge has good local-only, token, read-only-default design language.
- `.gitignore` keeps local outputs and `meshmaker/api_keys.txt` out of git by default.

#### Concerns

- An ignored `meshmaker/api_keys.txt` exists in the workspace. It was not read during review and should live outside the repository tree.
- `meshmaker` contains scripts that disable a safety checker for image generation.
- `meshmaker` includes military/weapon asset prompts unrelated to the core PID-VLA diagnostics.
- `launch_swarm.sh` can spawn 10 background cloud-generation jobs with side effects and possible costs.

#### Judgment

`meshmaker/` should be quarantined or split out. It is not aligned with the scientific core and increases operational, cost, and safety risk.

### 10. Open-Source Maintainer

#### Strengths

- The README clearly distinguishes implemented components from planned/specification-only components.
- The license is MIT.
- The Rust workspace is small and understandable.

#### Concerns

- `grandplan.md` is valuable but very large and hard to maintain.
- Tracked `meshmaker/*.py` scripts are messy and fail linting.
- Ignored local artifacts, `.external/`, generated PDFs, RRD files, and mesh outputs make the workspace larger and noisier than the tracked repo.
- No CI means formatting, Clippy, and Ruff drift are easy.

#### Judgment

The repo needs a cleanup pass that separates the canonical project from local experiments and scratch artifacts.

## Prioritized To-Do List

### P0: Blockers Before Serious Downstream Claims

1. **Make implemented Experiment 0 match documented Experiment 0.**
   - Add monotonicity gates: `I(S1,S2;T) >= I(S1;T)` and `I(S1,S2;T) >= I(S2;T)`.
   - Add conditional MI nonnegativity gates.
   - Add invariant bounds for `r_bar` and `v_bar`.
   - Add seed sweeps and uncertainty intervals.
   - Save CSV/JSON outputs with config hashes.

2. **Fix Rust hygiene.**
   - Run `cargo fmt --all`.
   - Fix the Clippy issue in `crates/pid-core/src/hyperbolic.rs`.
   - Add CI for `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, and `cargo test --workspace`.

3. **Harden estimator APIs against invalid configurations.**
   - Validate `Pid2Config` consistency between KSG and ISX configs.
   - Forbid `Metric::HyperbolicLorentz` in `pid3_isx` unless explicitly marked experimental and separately named.
   - Reject non-finite distances inside KSG loops.
   - Consider defaulting identity-sensitive APIs to `NegativeHandling::Allow`, or make clamping opt-in at reporting boundaries.

4. **Bring Python bindings to parity with the scientific contract.**
   - Expose `negative_handling`, `tie_epsilon`, co-information, `pid2_isx`, invariants, and preprocessing.
   - Add a proper Rust-extension build backend such as `maturin` if Python distribution is intended.
   - Add Python tests that mirror core Rust tests.

5. **Quarantine `meshmaker/`.**
   - Move it to a separate repository or a clearly marked scratch area outside the canonical project.
   - Remove safety-checker disabling.
   - Move API keys outside the repository tree.
   - Add dry-run defaults and explicit cost controls if kept.
   - Remove or isolate military/weapon asset generation from the PID-VLA research repo.

### P1: Needed for Publishable Engineering

6. **Implement M1 run logs and replay.**
   - Add a versioned event schema.
   - Record config and artifact hashes.
   - Provide a deterministic replay CLI.
   - Log Experiment 0 results.

7. **Make `pid-rerun` scientifically honest by construction.**
   - Rename fake PID demo outputs or compute real `pid2_isx`.
   - Validate shapes in `VlaFrame` and `VlaEpisode`.
   - Avoid one-hour sleep as default non-save behavior, or add a clear `--serve` mode.

8. **Resolve doc drift.**
   - Update `AGENTS.md` from v10.0 to v10.1.
   - Remove the `flake.lock` reference or add a tracked `flake.lock`.
   - Fix "small sample assets" wording in `grandplan.md` unless those assets are actually tracked.
   - Align `pidsplatspecs.md` Rerun version language with the actual Cargo dependency.

9. **Add a minimal real experiment harness.**
   - Pick one toy/small VLA or deterministic policy.
   - Pick one task.
   - Pick one run-log format.
   - Pick one failure label.
   - Include non-PID baselines before making PID claims.

10. **Add provenance discipline for generated artifacts.**
    - Treat ignored `outputs/` reports as local artifacts unless promoted.
    - If promoted, add generation command, source inputs, commit hash, and audit date.

### P2: Later Improvements

11. Add approximate or parallel kNN only after correctness gates are stable.
12. Add block bootstrap or trajectory-level resampling.
13. Add a discrete/quantized PID fallback.
14. Split `grandplan.md` into smaller maintainable specs while preserving one canonical index.
15. Add benchmark fixtures comparing against external reference implementations where licensing permits.

## Suggested Immediate Sequence

The next best move is not more architecture or more asset generation. A sensible execution order is:

1. Fix Rust formatting and Clippy.
2. Add CI for Rust formatting, Clippy, and tests.
3. Strengthen Experiment 0 gates to match the documented scientific criteria.
4. Harden estimator configuration validation.
5. Decide whether `meshmaker/` belongs in this repository.
6. Bring Python bindings and tests closer to the Rust scientific contract.
7. Build run logs and deterministic replay.

## Implementation Pass Status: 2026-05-09

Completed in the follow-up implementation pass:

- Rust formatting and Clippy were fixed.
- CI was added for Rust formatting, Clippy, tests, doc audits, and canonical Python lint.
- KSG/ISX/PID distance paths now reject non-finite or invalid distances instead of silently sorting `NaN`.
- `Pid2Config` now rejects incoherent KSG/ISX settings for `k`, metric, and `tie_epsilon`.
- `pid3_isx` now rejects non-Chebyshev metrics until research-gated support is explicitly added.
- Experiment 0 now supports deterministic seed sweeps, stricter monotonicity/CMI/invariant gates, and optional JSON summaries with a config hash.
- Python bindings now expose estimator options plus `compute_co_information`, `compute_pid2`, and `compute_invariants`; Maturin is wired in `pyproject.toml`.
- Python tests were added for the `pid_core_rs` extension.
- `pid-rerun` demo now computes actual `pid2_isx` atoms for its synthetic window instead of fixed PID proportions.
- `VlaEpisode` shape validation was added, and the demo no longer sleeps for an hour unless `--serve` is passed.
- `meshmaker/` remains in-repo but is quarantined from canonical Ruff lint; safety-checker disabling was removed from the main generators, and swarm launch now requires an explicit environment opt-in.
- Doc drift fixes were applied for docset version, Rerun SDK wording, Python binding status, and `flake.lock` wording.

Current residuals:

- The stricter Experiment 0 gate currently reports `NO-GO` on the synthetic quick run. This is useful: it surfaces estimator monotonicity, CMI, invariant, and geometry failures that should block downstream VLA claims.
- `meshmaker/` is still tracked legacy/auxiliary tooling. Full removal or repository split would be a destructive organizational change and should be done only with explicit confirmation.
- `crates/pid-runlog` now provides M1 groundwork (JSONL event schema, reader/writer, validation, replay summary, manifest/summary JSON, sidecar writing, hashes, and contract metadata), `crates/pid-bridge` provides M2 event-core/dispatcher/JSON-RPC request/response groundwork plus bridge/run-log contract export and read-only safe-mode gates, `crates/pid-sim` provides an M3 deterministic smoke sim with backend/solver config provenance, bridge demo, stdio JSON-RPC bridge, simulator-derived `Flow_gt` verification, and action replay checks, and `crates/pid-rerun` converts run logs into Rerun recordings with summary/provenance/validation diagnostics; full network bridge, physics backend, and real VLA/simulator experiment harness are still future work.
- A tiny deterministic labeled harness now exists in `pid-sim` (`pid-toy-harness` / `just toy-harness`) to exercise first-class success-label events, a replay-linked toy `(V,L,D,A)` embedding contract, PID/CI features, non-PID baselines, summary artifacts, and canonical run-log export before any real VLA claims.

## Ten-Scientist Consensus Follow-Up: 2026-05-09

The review perspectives converge on one implementation answer rather than competing UI stacks:

1. Keep Exp0/geometry gates strict and visible.
2. Build `pid-runlog` before more UI.
3. Make deterministic replay a required artifact.
4. Route GUI, scripts, and LLM tools through the Agent Bridge.
5. Implement a minimal physics-backed object sim and `Flow_gt`.
6. Use Rerun as the first serious diagnostic viewer.
7. Add one small VLA/task harness with labels and non-PID baselines.
8. Add live transport and external `Flow_pred` only after replay works.
9. Add Tauri/SparkJS as a Phase 4 shell that opens/embeds Rerun and contributes custom editors/shaders without owning truth.
10. Treat dependency, sidecar, model, data, generated mesh, and 3DGS capture licenses as release-blocking provenance items.
