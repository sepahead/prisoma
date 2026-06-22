# prisoma Whole-Repo Review and To-Do List

Date: 2026-05-09
Last consistency pass: 2026-06-12 (see "Implementation Pass Status: 2026-06-12" below)

This document records a whole-repo review of the prisoma repository from ten scientific/engineering perspectives, followed by a prioritized to-do list. It is intentionally direct and conservative: it distinguishes implemented functionality from specified/planned architecture and prioritizes scientific validity over roadmap optimism. The opening review has been updated after the follow-up implementation passes; older risks that were fixed are called out as fixed rather than left as current failures.

**Docset-wide final solution:** `grandplan.md` §A.8 records the current ten-scientist consensus. The run log is the source of truth, Rerun is the Phases 1–3 diagnostic/time-machine viewer, Agent Bridge is the only control plane, and Tauri/SparkJS is the deferred Phase 4 control/editor/custom-rendering shell.

## Executive Verdict

The repository is strongest as a **research specification plus a small, serious Rust estimator/run-log core**. The documentation is intentionally self-critical and correctly treats PID-on-VLA as a gated hypothesis rather than a proven method. The implemented Rust workspace, Python extension smoke tests, run-log/replay pipeline, deterministic bridge/sim smokes, toy harness, and offline `(V,L,D,A)` artifact harness all have automated coverage.

The main current risks are:

1. Experiment 0 is now stricter and visible, but the repo still needs a publishable measurement regime with uncertainty/block-bootstrap style evidence before downstream VLA claims.
2. There is still no real VLA capture/extraction/failure-label pipeline; the offline harness converts already-captured embeddings into canonical artifacts.
3. Python packaging is wired for local Maturin builds and tested, but the Python API is not yet a stable public interface.
4. `pid-rerun` is useful for validated run-log conversion and diagnostics, but remains a prototype viewer adapter rather than the full Phase 1–3 diagnostic product.
5. `meshmaker/` remains tracked legacy/auxiliary tooling and should be split or further quarantined before release.
6. The docs are now aligned to the current implementation, but the target architecture is still intentionally larger than the implemented repo.

## Verification Snapshot

Commands and outcomes from the most recent full implementation verification pass:

- `cargo fmt --all -- --check`: passed.
- `cargo test --workspace`: passed.
- `cargo clippy --workspace -- -D warnings`: passed.
- `uv run ruff check`: passed.
- `uv run pytest -q`: passed.
- `python scripts/audit_grandplan.py --check-italic-titles`: passed.
- `python scripts/audit_grandplan_claims.py`: passed.
- `python scripts/audit_docset_claims.py`: passed.
- Focused run-log sidecar smoke passed: `--write-sidecars` followed by `--verify-sidecars`, plus an exact-JSON drift failure check.

Additional documentation-only consistency checks from the 2026-05-10 pass:

- `python scripts/audit_grandplan.py --check-italic-titles`: passed.
- `python scripts/audit_grandplan_claims.py`: passed.
- `python scripts/audit_docset_claims.py`: passed.
- `python scripts/audit_docset_claims.py --paths $(git ls-files '*.md')`: passed.
- Local Markdown link-target check across tracked docs: passed.

## Ten-Perspective Review

### 1. Information Theorist

#### Strengths

- The repo correctly separates the distribution-level quantity from finite-sample estimators.
- The docs explicitly warn against the tempting but false claim that negative synergy literally means conflict or hallucination.
- Shared-exclusions PID is documented with the right high-level estimator form and counting convention.

#### Concerns

- The default KSG configuration clamps negative MI estimates to zero, which is practical for reports but risky for algebraic cancellation, co-information, and invariant checks.
- `Pid2Config` now rejects incoherent KSG/ISX settings for `k`, metric, and `tie_epsilon`; the remaining concern is documenting which estimator settings are appropriate for each evidence regime.
- `pid3_isx` now rejects non-Chebyshev metrics until research-gated support is explicitly added.

#### Judgment

The theoretical framing is good. Several invalid estimator combinations are now rejected; remaining API work is mostly about making validated regimes and reporting boundaries harder to misuse.

### 2. Statistician / Estimator Scientist

#### Strengths

- The docs explicitly identify high dimension, strong dependence, and trajectory autocorrelation as estimator failure modes.
- Tests cover Gaussian MI approximations, strong-dependence smoke tests, co-information checks, duplicate rejection, and fixed-data reference values.
- `findings.md` honestly reports current estimator incoherence and high-dimensional collapse.

#### Concerns

- The documented Experiment 0 acceptance criteria are strong; the implemented runner now exposes stricter monotonicity/CMI/invariant/geometry checks, but still needs uncertainty/resampling before it can serve as a publishable measurement protocol.
- `gromov_hyperbolicity` reports mean sampled delta, while gating usually needs worst-case or high-quantile behavior.
- Seed sweeps are implemented, but uncertainty quantification and block bootstrap/trajectory resampling remain open.

#### Judgment

The repo knows what the scientific gates should be, but the executable gate is not yet the scientific gate.

### 3. Robotics Experimentalist

#### Strengths

- The docs correctly require external targets, teacher actions, labels, interventions, or counterfactuals before making grounding claims.
- The recommended build order is sensible: Experiment 0, run logs/replay, bridge API, minimal simulator/ground-truth flow, Rerun viewer, then embeddings.

#### Concerns

- A deterministic simulator, Agent Bridge groundwork, run-log/replay system, toy harness, and offline embedding artifact converter now exist; physics-backed manipulation and real VLA data capture remain planned.
- `assets/` and `experiments/` are mostly scaffolding; a fresh clone cannot run later-stage experiments.
- The repo needs a minimal deterministic manipulation task before expanding into the larger architecture.

#### Judgment

Do not proceed to VLA claims yet. Build the early milestones before making new scientific conclusions.

### 4. VLA / ML Scientist

#### Strengths

- The hypothesis registry is falsifiable and baseline-aware rather than PID-triumphalist.
- The Flow-as-Bridge idea is a promising way to avoid raw high-dimensional embedding geometry problems.
- H9 now positions LRP, Integrated Gradients, DeepLIFT, Grad-CAM, TCAV, saliency/SmoothGrad, occlusion, and SHAP-style probes as attribution baselines/triangulation checks rather than replacements for PID.

#### Concerns

- There is no implemented model integration, embedding extraction job, real-task dataset loader, or failure-label pipeline; the offline `(V,L,D,A)` harness only converts already-captured embedding JSON into canonical artifacts, though it now includes deterministic all-pairs `V/L/D→A` PID screens plus train-split-only PID screens when a metadata split is present, standardization provenance, geometry diagnostics/gates, strict label/geometry/held-out-split/held-out-class-coverage/held-out-episode-disjoint modes, sample-level, episode-grouped, plus metadata-split held-out majority/1-NN/nearest-centroid success-label baselines with accuracy, balanced accuracy, and centroid AUROC, held-out class-coverage and episode-disjointness reports, held-out per-sample prediction records in summaries/run logs, and held-out failure-class confusion/rate diagnostics when labels/groups/splits are present.
- Attribution probes are documented but not implemented in the run-log schema or Rerun adapter yet; until then they should be artifact records with method/target/baseline/hash provenance.
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

- Rust formatting is now enforced and currently passes.
- Clippy is now enforced and currently passes.
- KSG/ISX/PID distance paths now reject non-finite or invalid distances instead of silently sorting `NaN`; continue adding tests when new metrics are introduced.
- `Pid3Config` now forbids non-Chebyshev metrics until mathematically validated.

#### Judgment

`pid-core` remains the repo's strongest implemented component; CI-level hygiene and several invalid-configuration guards are now in place, while publishable uncertainty protocols remain open.

### 6. Python Packaging / API Scientist

#### Strengths

- PyO3 bindings expose basic MI, redundancy, intrinsic dimension, hyperbolicity, and distance-statistics functions.

#### Concerns

- `pyproject.toml` now wires Maturin for local Rust-extension builds.
- Python users can now set estimator options needed by the Rust identity tests.
- Python extension smoke tests now exist.
- The Python API now exposes co-information, `compute_pid2`, and invariants; publishing/stability work remains.

#### Judgment

The Python bindings are useful for local experiments and now have local-build/test coverage, but they are not yet a stable public API.

### 7. Visualization / HCI Scientist

#### Strengths

- The Rerun-first strategy is pragmatic and avoids premature custom UI work.
- Entity paths are centralized in `pid-rerun`.

#### Concerns

- `vla_demo` now computes actual `pid2_isx` atoms for its synthetic window; it remains demo-only and not VLA evidence.
- `VlaEpisode` shape validation was added for malformed frames.
- The viewer assumes the external `rerun` CLI is installed.

#### Judgment

The visualization direction is good, but synthetic demos should remain clearly separated from real VLA diagnostics.

### 8. Reproducibility / MLOps Scientist

#### Strengths

- Docs consistently emphasize offline-first logs, replay, hashes, and a single control plane.
- Existing doc-audit scripts are useful and passed during review.

#### Concerns

- Historical `flake.lock` wording was removed or corrected; no tracked `flake.lock` is implied.
- `AGENTS.md`, README, and the broader docset now align to v10.1.
- CI now covers Rust formatting, Clippy, tests, doc audits, canonical Python lint, and run-log smokes.
- Experiment 0 outputs are not saved/versioned by default.

#### Judgment

The reproducibility plan is now backed by run logs, validation/replay, summaries/manifests, sidecar writing/verification, trace hashes, config hashes, and CI smokes; release-grade environment/data/model provenance is still future work.

### 9. Security / Safety / Governance Reviewer

#### Strengths

- The intended Agent Bridge has good local-only, token, read-only-default design language.
- `.gitignore` keeps local outputs and `meshmaker/api_keys.txt` out of git by default.

#### Concerns

- An ignored `meshmaker/api_keys.txt` exists in the workspace. It was not read during review and should live outside the repository tree.
- `meshmaker` is still cost-bearing external-generation tooling; main generators no longer disable the image safety checker, but upstream policy behavior and generated content still need release review.
- `meshmaker` includes military/weapon asset prompts unrelated to the core prisoma diagnostics.
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
- CI now catches formatting, Clippy, Rust tests, canonical Ruff drift, doc audits, and run-log smoke regressions.

#### Judgment

The repo needs a cleanup pass that separates the canonical project from local experiments and scratch artifacts.

## Prioritized To-Do List

### P0: Blockers Before Serious Downstream Claims

1. **Promote Experiment 0 from strict smoke gate to publishable measurement protocol.** *(Partially done 2026-06-11/12: block bootstrap + per-atom bootstrap CIs + single-source permutation tests now exist in `pid-core` (`bootstrap.rs`, `pipeline.rs`), and PLS defines a documented supervised-projection regime (grandplan §8.2.3). Open: wiring resampling/uncertainty into the Exp0 runner itself.)*
   - Keep the current monotonicity/CMI/invariant/geometry gates strict.
   - Add uncertainty quantification, block bootstrap or trajectory-level resampling, and clearly documented validated preprocessing regimes.
   - Keep treating current PIVOT/NO-GO outputs as limits evidence, not VLA conclusions.

2. **Build a real capture pipeline before making VLA claims.**
   - Add one model/task integration that extracts `(V,L,D,A)` embeddings plus externally meaningful success/failure labels.
   - Preserve sample IDs, episode IDs, train/held-out splits, config hashes, and artifacts in the canonical run log, with verified sidecars beside it.
   - Keep non-PID baselines mandatory.

3. **Quarantine or split `meshmaker/` before release.**
   - Keep it outside canonical lint/test claims unless it is cleaned up.
   - Move API keys and generated/cost-bearing workflows outside the repository tree.
   - Remove or isolate non-core asset-generation content from prisoma scientific releases.

4. **Define the first real VLA baseline and task.**
   - Pick one lightweight baseline, one task, one label definition, and one run-log contract.
   - Include at least one feasible attribution probe as a non-PID baseline if the model exposes the needed layers/gradients.
   - Avoid adding more optional model branches until that path works end to end.

### P1: Needed for Publishable Engineering

5. **Add physics-backed manipulation beyond the deterministic object smoke.**
   - Introduce a backend adapter only if it still emits canonical run-log events and can be replayed/verified.
   - Treat cross-backend replay as a robustness/confound check rather than a claim of physics truth.

6. **Harden release provenance.**
   - Generate dependency notices/SBOMs for Rust/Python and, later, npm/Tauri.
   - Record license/provenance for datasets, model checkpoints, generated meshes, prompts, 3DGS captures, and sidecars.

7. **Turn the Rerun workflow into the first serious diagnostic viewer.**
   - Preserve the current validated run-log-to-Rerun adapter.
   - Add richer run-level panels only after they are backed by run-log events, summaries, or manifests.

8. **Stabilize the Python API only after the Rust scientific contract settles.**
   - Keep local Maturin builds/tests passing.
   - Add versioned examples and packaging metadata before treating it as public.

9. **Prototype attribution artifact logging before adding schema surface.**
   - Start with `artifact_logged` records for attribution tensors/heatmaps plus method/target/baseline metadata.
   - Promote a first-class `attribution_computed` event only after one real VLA/task path demonstrates which fields are stable.

### P2: Later Improvements

10. Add approximate or parallel kNN only after correctness gates are stable. *(Exact parallel done 2026-06-14: `cargo … --manifest-path pid-rs/crates/pid-core/Cargo.toml --features parallel` runs the KSG kNN data-parallel via rayon, producing results identical to the serial path — the full pid-core suite incl. the independent cross-validation passes under the feature, validated by a dedicated CI job. Only the **approximate** kNN variant — which would change results — remains deferred per the "after correctness gates are stable" caution.)*
11. Add a discrete/quantized PID fallback. *(Done 2026-06-11/12: 2- and 3-source discrete PID with an `I_min`-style redundancy — explicitly not discrete `i^sx_∩`, see grandplan §8.1.6 — plus harness `--pid-mode discrete|discrete-pls` and per-pair saturation diagnostics.)*
12. Split `grandplan.md` into smaller maintainable specs while preserving one canonical index. *(Deferred: pure-maintainability refactor whose main risk is breaking the doc-audit tooling that scans `grandplan.md`; should be done with an explicit split plan, not autonomously.)*
13. Add benchmark fixtures comparing against external reference implementations where licensing permits. *(Done 2026-06-14, self-contained: `pid-rs/crates/pid-core/tests/cross_validation.rs` re-derives the Williams–Beer `I_min` PID by an independent route and checks `discrete_pid2` against it + the known logic-gate structure. The external SxPID-class refs live under gitignored `.external/` and are not CI-reproducible, so an in-test independent reference is used instead.)*

## Suggested Immediate Sequence

The next best move is not more architecture or more asset generation. A sensible execution order is:

1. Keep Exp0/geometry gates strict and add uncertainty/resampling.
2. Keep run-log sidecars verified and make deterministic replay a required artifact.
3. Connect the offline embedding harness to one real VLA/task capture with labels.
4. Add physics-backed manipulation only behind the same run-log/Agent Bridge contract.
5. Continue quarantining or splitting `meshmaker/`.
6. Stabilize Python packaging/API after the Rust evidence contract is stable.

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
- `pid-rerun` demo now computes actual `pid2_isx` atoms for its synthetic window instead of earlier placeholder proportions.
- `VlaEpisode` shape validation was added, and the demo no longer sleeps for an hour unless `--serve` is passed.
- `meshmaker/` remains in-repo but is quarantined from canonical Ruff lint; safety-checker disabling was removed from the main generators, and swarm launch now requires an explicit environment opt-in.
- Doc drift fixes were applied for docset version, Rerun SDK wording, Python binding status, and `flake.lock` wording.

Current residuals:

- The stricter Experiment 0 gate currently surfaces PIVOT/NO-GO-style failures on the synthetic diagnostics. This is useful: it exposes estimator monotonicity, CMI, invariant, and geometry failures that should block downstream VLA claims.
- `meshmaker/` is still tracked legacy/auxiliary tooling. Full removal or repository split would be a destructive organizational change and should be done only with explicit confirmation.
- `pid-rs/crates/pid-runlog` now provides M1 groundwork (JSONL event schema, reader/writer, validation, replay summary with unique metric-name counts plus total metric-event counters, manifest/summary JSON, sidecar writing/verification, hashes, config-hash consistency gates, first-class `Flow_gt`/`flow_pred`, and contract metadata), `crates/pid-bridge` provides M2 event-core/dispatcher/JSON-RPC request/response groundwork plus bridge/run-log contract export and read-only safe-mode gates, `crates/pid-sim` provides an M3 deterministic smoke sim with backend/solver config provenance, bridge demo, stdio/TCP/WebSocket JSON-RPC bridges, safe-mode `log.replay`, bridge `log.start`/`log.stop`, deterministic bridge `intervention.apply`, bridge `export.rerun`, simulator-derived `Flow_gt`, constant-velocity baseline `flow_pred`, flow verification, action/intervention replay checks, and an offline `(V,L,D,A)` artifact-to-runlog harness with all-pairs `V/L/D→A` PID screens plus train-split-only PID screens when a metadata split is present, standardization provenance, geometry diagnostics/gates, strict label/geometry/held-out-split/held-out-class-coverage/held-out-episode-disjoint modes, deterministic sample-level, episode-grouped, plus metadata-split held-out majority/1-NN/nearest-centroid success-label baselines with accuracy, balanced accuracy, and centroid AUROC, held-out class-coverage and episode-disjointness reports, held-out per-sample prediction records in summaries/run logs, replay-visible total metric event counts, and held-out failure-class confusion/rate diagnostics, and `crates/pid-rerun` converts run logs into Rerun recordings with summary/provenance/validation diagnostics; physics backend and real VLA/simulator data capture are still future work.
- A tiny deterministic labeled harness now exists in `pid-sim` (`pid-toy-harness` / `just toy-harness`) to exercise first-class success-label events, a replay-linked toy `(V,L,D,A)` embedding contract, PID/CI features, non-PID baselines, summary artifacts, and canonical run-log export before any real VLA claims.
- A generic offline embedding harness now exists in `pid-sim` (`pid-offline-harness` / `just offline-harness`) to ingest captured JSON samples with `v`, `l`, `d`, `a`, labels, and metadata, then emit canonical summary/run-log artifacts with config provenance, embedding contracts, standardized all-pairs `V/L/D→A` PID metrics plus train-split-only PID metrics when a metadata split is present, geometry diagnostics/gates, strict label/geometry/held-out-split/held-out-class-coverage/held-out-episode-disjoint modes, deterministic sample-level plus episode-grouped and metadata-split held-out majority/1-NN/nearest-centroid success-label baselines with accuracy, balanced accuracy, and centroid AUROC, held-out class-coverage and episode-disjointness reports, held-out per-sample prediction records in summaries/run logs, replay-visible total metric event counts, held-out failure-class confusion/rate diagnostics, and artifact records. `--require-geometry-pass` / `just offline-harness-strict` fails closed on geometry warnings while preserving a valid failed run log; `--require-heldout-split` fails closed if train/held-out baselines cannot be produced; `--require-heldout-class-coverage` fails closed if either subset lacks success or failure labels; `--require-heldout-episode-disjoint` fails closed if any `episode_id` crosses the train/held-out boundary.

## Implementation Pass Status: 2026-06-13

Completed since the 2026-06-12 pass (the "capture + analysis" slice — docset v10.3). This pass closes every estimator-side blocker the review previously listed *and* delivers the first real M5 capture path:

- **P0 item 1 (Exp0 → publishable measurement protocol) closed end-to-end.** The Exp0 binary gains opt-in `--bootstrap`/`--permutation` (+ `--block-size`/`--alpha`): subsample-bootstrap CIs (Politis–Romano, KSG-safe — the naive with-replacement bootstrap is documented as unreliable for kNN MI) and single-source permutation nulls at the favourable dimension, with a **preregistered, ground-truth-derived marginal-significance check** folded into the GO/PIVOT/NO-GO verdict (the permutation null must call a source significant iff it is marginally informative by construction; calibrated 8/8 on the four synthetic scenarios). `pid-core` gains generic `bootstrap_rows_stats`/`permutation_rows_pvalue`. Default runs stay byte-identical, so CI metric counts are unchanged.
- **P0 items 2 + 4 (the critical path) — first real capture path shipped.** `experiments/safe_adapter/` converts released SAFE rollouts into the `(V,L,D,A)`+labels harness contract, with honest per-variable provenance (D=hidden states, A=actions, labels are native; V/L are token-sliced / explicitly supplied / a labelled text proxy — never fabricated) and the §7.6.3 layerwise physics-probe for `D_hidden[k]` selection. Verified end to end into the real `pid-offline-harness` with `--require-heldout-split`/`-class-coverage`/`-episode-disjoint` all passing. The SAFE-class internal-feature failure detector (P0 item 4) is now a built-in harness baseline (`heldout_logreg_vlda`, leakage-safe), and the faithfulness-checked attribution probe (`experiments/attribution/`, §14.7.1) emits validated `attribution_logged` run logs.
- **P1 item 5 (physics-backed manipulation) shipped (no longer a stub).** A real single-threaded `rapier3d-f64` backend (gravity/contacts/friction, deterministic substepping) replaces the fake stub, plus a scripted push-to-goal manipulation emitting canonical run-log events with real `Flow_gt` and physics-derived success labels (`pid-rapier-harness`, `rapier` CI job). Honest scope: flow-consistent + deterministically self-reproducible, but **not** constant-velocity replayable (that model is for the kinematic sim), and no cross-platform bit-determinism claim.
- **P0 item 3 / P1 item 6 (governance).** `meshmaker/` is quarantined out of tracking (`git rm --cached` — local files kept; tombstone README; `.gitignore` hardened). `scripts/generate_third_party_notices.py` generates a direct-dependency SBOM (Rust licenses via `cargo metadata`, Python versions via `uv.lock`) with a CI drift check.
- **Verification:** `cargo fmt --all --check`, `cargo clippy --workspace`, `cargo test --workspace`, `cargo clippy/test -p pid-sim --features rapier`, `uv run ruff check .`, the SAFE-adapter + attribution pytest suites, and all three doc audits (incl. the full tracked-Markdown sweep) pass.

**Revised critical path:** the remaining blocker is purely a **data-pull step** — point `experiments/safe_adapter` at the real downloaded SAFE rollouts (verify tensors/coverage/licenses) and run the existing analysis (PID screens + the built-in non-PID baselines incl. the SAFE-class detector + the §14.7.1 attribution probe) under the geometry/uncertainty gates; the preregistered §14.1.1 kill criteria then decide whether PID atoms earn a place. No further estimator, harness, baseline, or attribution code is required for that first study.

> **Not on the critical path:** `crates/ncp-observer` (the Engram/NEST Neuro-Cybernetic-Protocol bridge) is an **optional** `(V,L,D,A)` source — grandplan does not depend on Engram, and the pure-PID stack builds/gates green with no NCP/Engram/Zenoh dependency (it is excluded from the default cargo workspace). It is exploratory-only until it meets the M5 contract (D `seq`-alignment, honest `L`, split/episode/label structure); bringing it up to bar is a self-contained task tracked in `NCP_DEV_PROMPT.md`. It does not block the SAFE data-pull above.

## Implementation Pass Status: 2026-06-12

Completed since the 2026-05-09 pass (commits `20fd4bc`, `7e2c515`, `951a348`, `0b9d4a7` + this pass):

- **Estimator escape hatches shipped:** PLS supervised dimensionality reduction (`pls.rs`; the fix for the Exp0 signal-in-noise finding), discrete 2- and 3-source PID via quantization (`discrete_pid.rs`), block-bootstrap uncertainty (`bootstrap.rs`), and a `pipeline.rs` composition layer (PLS→PID3, per-atom bootstrap CIs, single-source permutation tests, LOO-CV PLS component selection, all-pairs PID2 screening). This closes the library-level half of P0 item 1 and all of P2 item 11.
- **Measure identity corrected:** the discrete redundancy is a Williams–Beer-style `I_min` functional, not discrete `i^sx_∩`; code names (`discrete_imin_redundancy*`), provenance strings (`discrete_imin`/`pls_discrete_imin`), and the docset (grandplan §8.1.6, Warning 6 extension) now say so. Cross-mode comparisons are cross-measure comparisons.
- **Saturation gate implemented and empirically confirmed:** discrete modes emit per-pair `discrete_saturation` diagnostics; smoke fixtures show plug-in MI pinned at the `ln(n)` ceiling being flagged, as §8.1.6 predicts.
- **Harness modes:** `pid-offline-harness --pid-mode continuous|discrete|discrete-pls` with `--discrete-bins`/`--pls-components`; `just offline-harness-discrete` / `offline-harness-discrete-pls` recipes; unit + smoke coverage for all three modes.
- **Other June items:** `PhysicsBackend` trait + Rapier3D stub behind the `rapier` feature (P1 item 5 groundwork only — not a validated physics path), `attribution_logged` run-log event (schema only; the P1 item 9 caution about field stability still applies), Exp0 `--strict-gate`, high-dim synthetic fixture (CI smoke only), Python bindings 8→14.
- **Docset v10.2:** measure-identity + supervised-projection + action-chunking guidance; §12.5 external-source review (integrated and ruled-out, both cited); §14.1.1 per-hypothesis PID-necessity audit with preregistered kill criteria; §14.7.1 AttnLRP/LRP protocol; world-model taxonomy update (latent-predictive/JEPA + unified omnimodel/WAM classes); Physics-Emergence-Zone hook-point prior (§7.6.3); real-robotics-problem column + failure-regime framing in the hypothesis registry (§14.1.0).

Verification for this pass: `cargo test --workspace` green (35 suites), `cargo check` warning-free, all three doc audits pass, arXiv cache complete with zero italic-title drift.

**Revised critical path (unchanged in substance, sharper in justification):** the single highest-value next step remains **P0 items 2 + 4 — one real VLA/task capture with externally meaningful labels**, run through the offline harness with non-PID baselines and one faithfulness-checked attribution probe (AttnLRP per grandplan §14.7.1). Every estimator-side prerequisite this review previously listed as blocking (uncertainty quantification, supervised projection, a discrete fallback, regime documentation) now exists; the remaining blockers are data and labels, not estimators. Pick the capture target by the grandplan §10.10.13.3 decision matrix; the §7.6.3 Physics-Emergence-Zone probe procedure should choose the `D_hidden[k]` hook layer before geometry gating. **Concrete shortcut identified 2026-06-12:** the SAFE repo (`vla-safe/SAFE`, NeurIPS 2025) released rollout datasets (π0-FAST on Franka; OpenVLA on WidowX, with outcomes) and the code that generates them while extracting internal features — adapting that pipeline to emit this project's `(V,L,D,A)`+labels contract is likely cheaper than building capture from scratch (verify released tensors, per-step coverage, and licenses first). Secondary: wire bootstrap/permutation into the Exp0 runner (closing P0 item 1 end-to-end), and keep `meshmaker/` quarantine (P0 item 3) on the release checklist.

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
