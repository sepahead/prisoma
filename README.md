# prisoma

> **Rust PID estimators + Python bindings live in [`pid-rs`](https://github.com/sepahead/pid-rs) — the single source of truth.**
> `pid-core`, `pid-runlog`, and the `pid-python` (`pid_core_rs`) bindings are **not** vendored here;
> they are pinned as the `pid-rs/` git submodule. After cloning: `git submodule update --init --recursive`.
> The local crates (`pid-sim`, `pid-rerun`, `pid-bridge`) path-depend into `pid-rs/crates/*`, and the
> estimator binaries run from the submodule, e.g.
> `cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --bin exp0`.
> Build the Python module from the submodule: `maturin develop -m pid-rs/crates/pid-python/Cargo.toml`.

> **Docs (start here):**
> - `grandplan.md` — Canonical spec (definitions, gates, hypotheses, engineering plan)
> - `EXPERIMENTS.md` — What to run + what to log (protocols)
> - `ARCHITECTURE.md` — Target system design (PID‑Splat)
> - `DIAGRAMS.md` — Architecture + control plane diagrams
> - `pidsplatspecs.md` — Simulation/spec details (PID‑Splat)
> - `uidesigner/UI.md` — UI/UX spec (viewer-first; ordered by milestones)
> - `GAUSS_MI_INTEGRATION.md` — Optional: 3DGS uncertainty + view selection (spec)
> - `WORLD_WARP_INTEGRATION.md` — Optional: external world‑model baseline (spec)
> - `THIRD_PARTY_NOTICES.md` — Release-governance starter notices/checklist
> - `findings.md` — Current estimator-status evidence (Exp0 results + interpretation)
> - `REVIEW_AND_TODO.md` — Whole-repo review, prioritized to-do list, and the current critical path
> - `NCP_DEV_PROMPT.md` — Optional: dev handoff for the Engram/NCP `(V,L,D,A)` bridge

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](#license)

prisoma is a research toolkit for diagnosing **Vision‑Language‑Action (VLA)** policies using **Partial Information Decomposition (PID)** (shared‑exclusions `I^sx_∩`) and related information‑theoretic controls, with local attribution methods treated as baselines/triangulation probes. The project is **gate‑driven**: do not interpret PID atoms on real embeddings until the estimator + geometry gates pass.

## Current Status & What To Do, In Order (v10.3, 2026-06-13)

**Status in one paragraph:** the Rust estimator core, run-log/replay/bridge/sim/Rerun groundwork, and the offline `(V,L,D,A)` harness are implemented with passing tests. Experiment 0 reports **PIVOT/NO-GO** on synthetic high-dimensional controls — that is the gate *working*, not a bug (see `findings.md`): continuous kNN PID atoms are not interpretable on raw high-d embeddings. As of v10.3 the estimator/analysis side is wired end to end: Exp0 has an opt-in uncertainty gate (`--bootstrap`/`--permutation` with a preregistered marginal-significance check), the offline harness carries a SAFE-class logistic-regression failure detector alongside the other non-PID baselines, a real Rapier3D manipulation produces physics-derived labels + `Flow_gt`, `experiments/safe_adapter/` converts released SAFE VLA rollouts into the harness contract (the M5 capture shortcut), and `experiments/attribution/` is a faithfulness-checked H9 attribution probe. **The open critical path is now running this on real downloaded VLA data** — the adapter and the full analysis/baseline/attribution path are in place (`REVIEW_AND_TODO.md`).

Each step gates the next; canonical depth is in `grandplan.md` at the cited sections.

1. **Verify the toolchain and see the gate fire:** `cargo test`, then `just exp0` / `just exp0-bin`. Expect a `PIVOT` verdict on the synthetic high-d diagnostics and read `findings.md` for why. Gate criteria: `grandplan.md` §9.1.
2. **Learn the measurement-regime rules before touching real data:** one (PID measure, preprocessing, estimator config) tuple = one preregistered regime; never pool or compare continuous `I^sx_∩` atoms with discrete `I_min` atoms as if they were one quantity (`grandplan.md` Warning 6 + §8.1.6); supervised projections (PLS) are fit on training samples only and re-gated (§8.2.3 step 5).
3. **Exercise the full pipeline on checked fixtures:** `just toy-harness`, `just offline-harness`, then the strict and discrete variants (`offline-harness-strict`, `offline-harness-discrete`, `offline-harness-discrete-pls`). The strict run exits nonzero and the discrete runs show `saturation_warning=true` on the tiny fixtures — both **by design**; they demonstrate the gates you must respect on real data.
4. **Capture real data (open critical path; M5):** pick one model + one task via the decision matrix in `grandplan.md` §10.10.13.3; choose the `D` hook layer by the layerwise physics-probe procedure (§7.6.3) *before* geometry gating; log `(V,L,D,A)` with success labels, `episode_id`s, and a train/test split in the offline-harness input schema (below). The fastest path is now implemented: `experiments/safe_adapter/` converts released SAFE rollouts (`vla-safe/SAFE`; OpenVLA + π0-FAST, with outcomes — verify tensors/licenses) into this contract, with honest per-variable provenance and the §7.6.3 hook-probe (`python -m experiments.safe_adapter`, or `just safe-adapter`). For a physics task instead, the real Rapier3D manipulation (`just rapier-harness`) emits labels + `Flow_gt`.
5. **Analyze under the gates:** run the harness on the capture; geometry + coherence gates select continuous vs discrete vs MI-only screening (H8); report **all** regimes attempted; quantify uncertainty with the built-in opt-in `--bootstrap N --permutation N` flags (subsample-bootstrap CIs + single-source permutation p-values on the continuous `(V,L)/(V,D)/(L,D)→A` atoms, written to a dedicated `--uncertainty-json` file so the canonical run-log counts are untouched), or call the `pid-core` helpers (`bootstrap_rows_stats`/`permutation_rows_pvalue`) directly.
6. **Run the non-PID baselines every time:** majority/1-NN/centroid baselines *and* a SAFE-class logistic-regression internal-feature failure detector (`heldout_logreg_vlda`) are built into the harness; add one faithfulness-checked attribution probe (`experiments/attribution/`, the §14.7.1 AttnLRP protocol; `just attribution-probe`). The preregistered kill criteria (§14.1.1) decide whether PID atoms earn a place in any claim — a negative answer is a publishable outcome.
7. **Only then** run the Exp1–Exp5 protocols in `EXPERIMENTS.md` (see its §0.2 runbook for what is executable today vs blocked on step 4).

## Hypotheses (Docset v10.3)

The canonical registry + falsification criteria live in `grandplan.md` (§14.1).

| Hypothesis | One‑line testable claim | Status | Real robotics problem addressed |
|---|---|---|---|
| **H1** | PID/CI features predict failure labels beyond strong baselines. | Core | Failure triage at fleet scale (frontier generalists still fail most episodes; teams triage by watching videos) |
| **H2** | Redundancy predicts robustness to single‑modality ablation (matched controls). | Exploratory | Forecasting which skills degrade when a sensor/modality degrades, before it happens in the field |
| **H3** | Uniques predict intervention sensitivity (matched‑strength perturbations). | Exploratory | Targeted data collection: spend teleop budget on the modality that actually moves behavior |
| **H4** | Memorization vs generalization induces systematic PID/CI shifts. | Core | Pre-deployment generalization certification (VLA-Arena: current VLAs memorize) |
| **H5** | Long‑horizon failures correlate with temporal PID/CI degradation. | Core (CI-only ablation mandatory) | Early warning for compositional drift in multi-stage tasks (kitting/assembly) |
| **H6** | Safety tasks show distinctive V–L integration patterns (only with proper labels/controls). | Deferred | Safety-case evidence (needs proper labels first) |
| **H7** | Flow‑as‑Bridge enables stage‑wise diagnostics and embodiment‑agnostic comparisons. | Core (method) | Cross-embodiment porting diagnosis: world-model failure vs execution failure |
| **H8** | Geometry diagnostics determine which estimator regime is valid. | Core (method) | Trustworthy metrics: don't ship estimator artifacts to dashboards |
| **H9** | Faithfulness-checked attribution probes (LRP/IG/DeepLIFT/Grad-CAM/TCAV/saliency/occlusion/SHAP-style) should triangulate, or falsify, PID-derived modality/stage claims. | Triangulation | Audit-grade incident explanations from converging evidence |

PID is **forced nowhere**: `grandplan.md` §14.1.1 records, per hypothesis, the cheapest non-PID alternative, what PID distinctively adds, and the preregistered kill criteria that downgrade or drop PID-atom claims when simpler quantities suffice.

## Experiments (Run Order)

Details and logging requirements live in `EXPERIMENTS.md`; estimator gates and confounds live in `grandplan.md`.

1. **Exp0** — Estimator + geometry gate (GO/PIVOT/NO‑GO). *Runnable today* (`just exp0`); current verdict on synthetic high-d controls: **PIVOT** (`findings.md`). Nothing downstream is interpretable without this gate.
2. **Exp1** — Pick‑and‑place + perturbations (H1–H4). *Blocked on the first real capture* (step 4 above); the offline harness + baselines that will analyze it are runnable today.
3. **Exp2** — Long‑horizon composition (H5; windowed PID/CI with block bootstrap; CI-only ablation mandatory). *Blocked on capture.*
4. **Exp3** — Instruction/visual/physics perturbations (H1–H6; matched-strength controls + placebos). *Blocked on capture.*
5. **Exp4** — Flow‑as‑Bridge bring‑up with simulator `Flow_gt` (H7). *Sim-side `Flow_gt` + verification runnable today* (`just runlog-sim-verify`); VLA-side blocked on capture.
6. **Exp5** — Cross‑embodiment replication (H4/H7; mind the embodiment-in-`L` confound, `grandplan.md` §14.5.7.3). *Blocked on capture.*

Attribution methods are comparison evidence, not a shortcut around PID validity: LRP and related methods explain one model call or concept direction, while PID/CI estimates distribution-level information across logged samples. If attribution probes disagree with PID signatures under controlled interventions, treat the disagreement as a diagnostic result.

## Doc Audits

- `python scripts/audit_grandplan.py --check-italic-titles` (arXiv coverage + title drift; uses `outputs/arxiv_ref_cache.json`)
- `python scripts/audit_grandplan_claims.py` (heuristic scan for unqualified venue/perf claims)
- `python scripts/audit_docset_claims.py` (same heuristic scan across the canonical docset + `findings.md`)
- Full tracked-Markdown sweep: `python scripts/audit_docset_claims.py --paths $(git ls-files '*.md')`
- If you have `just`: `just docs-audit`

## Repo Status (What Actually Exists)

- Implemented: `pid-rs/crates/pid-core` (KSG MI — with an optional `parallel` feature for exact, deterministic data-parallel kNN — continuous `I^sx_∩`, PLS supervised reduction, discrete 2-/3-source PID with an `I_min`-style redundancy + saturation diagnostics — see `grandplan.md` §8.1.6 — block bootstrap, generic `bootstrap_rows_stats`/`permutation_rows_pvalue` uncertainty helpers, an L2 logistic-regression classifier (`logistic.rs`), and PLS→PID3/bootstrap/permutation pipeline helpers), `pid-rs/crates/pid-python` (`pid_core_rs`; 15 functions), `pid-rs/crates/pid-runlog` (M1 JSONL schema + replay/validate/compare/summary/manifest/sidecar write-and-verify CLI), `crates/pid-bridge` (local Agent Bridge request/response dispatch core + JSON-RPC-shaped request/response conversion + contract export), `crates/pid-sim` (deterministic object sim + a **real `rapier3d-f64` physics backend** (behind the `rapier` feature) + a scripted push-to-goal manipulation with physics-derived labels and `Flow_gt` (`pid-rapier-harness`), `Flow_gt`/baseline `flow_pred` bridge demos, stdio/TCP/WebSocket JSON-RPC bridges, safe-mode `log.replay`, bridge `log.start`/`log.stop`, deterministic `intervention.apply`, bridge `export.rerun`, flow verification, action/intervention replay checks, a labeled toy VLA/task harness, and a generic offline `(V,L,D,A)` embedding harness with all-pairs `V/L/D→A` PID screens plus train-split-only PID screens when a metadata split is present, standardization provenance, geometry diagnostics/gates, strict label/geometry/held-out-split/held-out-class-coverage/held-out-episode-disjoint modes, deterministic sample-level, episode-grouped, and metadata-split held-out majority/1-NN/nearest-centroid **and SAFE-class logistic-regression** success-label baselines with accuracy, balanced accuracy, centroid AUROC, held-out class-coverage and episode-disjointness reports, held-out per-sample prediction records in summaries/run logs, and held-out failure-class confusion/rate diagnostics), `crates/pid-rerun` (prototype Rerun logging + validated run-log replay adapter with summary/provenance/validation diagnostics), the Experiment 0 runner (`just exp0`, `just exp0-bin`, `just exp0-runlog`, plus an opt-in `--bootstrap`/`--permutation` uncertainty gate), and the Python `experiments/` (`safe_adapter` — released-SAFE-rollout → `(V,L,D,A)` contract converter with the §7.6.3 hook-probe; `attribution` — faithfulness-checked H9 attribution probe emitting `attribution_logged` run logs).
- Source-agnostic capture: the analysis consumes one `(V,L,D,A)`+labels contract, so producers are pluggable. The **critical-path producer is `experiments/safe_adapter/`** (released SAFE rollouts → contract, gate-passing); `crates/pid-sim` fixtures + the Rapier/toy harnesses are standalone sim cross-checks. In `(V,L,D,A)`, **D is the hidden-state / dynamics axis, not depth** (`grandplan.md` §7.6.3).
- Optional Engram bridge: `crates/ncp-observer` is a read-only Neuro-Cybernetic-Protocol tap that turns an Engram/NEST session into another `(V,L,D,A)` source. It is **not on grandplan's critical path** (grandplan does not depend on Engram), is **kept off the default cargo workspace** to keep NCP/Zenoh off the critical path (it git-depends on the published NCP repo <https://github.com/sepahead/NCP> (v0.5.0); build via `cargo build --manifest-path crates/ncp-observer/Cargo.toml`), and is **exploratory-only** until its provenance gaps close (honest `L`, split/episode/label structure; D `seq`-alignment is already wired observer-side). The pure-PID stack builds and gates green with **no NCP/Engram/Zenoh dependency**. Bringing it up to the M5 bar is a self-contained task — `NCP_DEV_PROMPT.md`.
- Specified: A fuller Rerun-based diagnostic viewer (Phases 1-3) and the deferred Tauri/SparkJS UI (Phase 4). The H9 attribution probe exists in `experiments/attribution/` (faithfulness-checked, with `attribution_logged` emission validated by `pid-runlog-replay`), and the `pid-rerun` adapter surfaces those events as a plottable faithfulness verdict, a provenance line, and the per-element relevance values from the `.npy` artifact (a multi-value `Scalars` series); multi-panel 2-D heatmap blueprints remain future work, and production VLAs should swap the reference model for LXT/AttnLRP. Start at `grandplan.md` §A.7.

## Quick Start (Exp0 Gate)

```bash
# optional: nix develop
cargo test
just exp0
just exp0-bin
just exp0-runlog
```

If you don’t have `just`: `cargo test` and `cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --bin exp0`. To export canonical Exp0 evidence, run `cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --bin exp0 -- --summary-json outputs/exp0_summary.json --runlog outputs/exp0_runlog.jsonl`, then validate it with `cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/exp0_runlog.jsonl`.
See `findings.md` for the latest repo-local Exp0 interpretation notes.

## Quick Start (Tiny Labeled Harness)

```bash
just toy-harness
```

If you don’t have `just`: run `cargo run -p pid-sim --bin pid-toy-harness -- --summary-json outputs/toy_vla_summary.json --runlog outputs/toy_vla_runlog.jsonl`, then validate it with `cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/toy_vla_runlog.jsonl`. This is a deterministic toy task, not VLA evidence; it exists to exercise first-class label events, a replay-linked toy `(V,L,D,A)` embedding contract, PID/CI features, non-PID baselines, summary artifacts, and canonical run-log export end to end.

## Quick Start (Offline VLDA Embedding Harness)

```bash
just offline-harness
just offline-harness-require-labels
just offline-harness-require-heldout
just offline-harness-require-heldout-class-coverage
just offline-harness-require-heldout-episode-disjoint
just offline-harness-strict
just offline-harness-highdim
just offline-harness-discrete
just offline-harness-discrete-pls
```

PID estimator modes: `--pid-mode continuous` (default; KSG + continuous `I^sx_∩`), `--pid-mode discrete` (equal-width quantization + a Williams–Beer-style `I_min` redundancy — a different PID measure; see `grandplan.md` §8.1.6), and `--pid-mode discrete-pls` (PLS-project `V/L/D` toward `A`, then discrete PID; `--pls-components N`). Discrete modes attach per-pair `discrete_saturation` diagnostics; pairs with `saturation_warning=true` are estimator artifacts, not evidence (expected on the tiny checked fixtures).

If you don’t have `just`: run `cargo run -p pid-sim --bin pid-offline-harness -- --input crates/pid-sim/fixtures/offline_vlda_fixture.json --summary-json outputs/offline_vlda_summary.json --runlog outputs/offline_vlda_runlog.jsonl`, then validate it with `cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/offline_vlda_runlog.jsonl`. Add `--require-success-labels` when a run must include boolean `success` labels for every sample, add `--require-heldout-split` when a run must include train/held-out split baselines, add `--require-heldout-class-coverage` when train and held-out subsets must each include boolean `success=true` and `success=false` samples, add `--require-heldout-episode-disjoint` when no `episode_id` may appear in both train and held-out subsets, and add `--require-geometry-pass` when the run should fail closed if the standardized geometry gate warns; strict modes still write canonical failed run logs. The input schema is a JSON object with optional `run_id`/`source`/`model`/`task` fields and a `samples` array; each sample carries `sample_id`, optional `episode_id`, numeric `v`, `l`, `d`, and `a` vectors, optional `labels`, and optional string `metadata`. Current PID atoms include all two-source `V/L/D→A` screens—`(V,L;A)`, `(V,D;A)`, and `(L,D;A)`—computed after deterministic per-variable standardization and accompanied by geometry diagnostics/gates over the standardized analysis space. When a recognized metadata split is present, the summary and run log also emit train-split-only PID screens under explicit `metadata_split_train` provenance, fit with train-only standardizers so held-out embeddings are excluded from both preprocessing and PID evidence. If every sample has a boolean `success` label, the harness also logs success rate, majority accuracy, sample-level leave-one-out 1-NN success baselines, leakage-resistant leave-one-episode-out majority/1-NN baselines when all samples carry `episode_id`, and true held-out majority/1-NN plus train-standardized nearest-centroid baselines when every sample has `metadata.split` set to a recognized train value (`train`/`training`) or held-out value (`test`/`validation`/`val`/`eval`/`evaluation`/`heldout`/`holdout`/`held_out`/`hold_out`). Held-out baselines emit both accuracy and balanced accuracy when both held-out success classes are present; nearest-centroid baselines are emitted only when the train split contains both success classes and also emit AUROC from the signed centroid-distance score when both held-out classes are present. The summary and run log preserve split counts, train/held-out sample IDs, class-coverage counts/status, episode-disjointness counts/status, held-out per-sample prediction records with 1NN provenance plus centroid scores, and failure-class confusion/rate diagnostics; replay summaries keep `*_metrics` as unique latest-by-name counts and add `*_metric_events` counters so repeated per-sample prediction metrics are counted as events. The harness is an artifact-to-runlog converter for captured embeddings, not evidence from a real VLA by itself.

## Quick Start (M1 Run Log)

```bash
just runlog-demo
just bridge-contract
just runlog-replay
just runlog-validate
just runlog-bridge-demo
just runlog-bridge-stdio-safe
just runlog-bridge-stdio
just runlog-bridge-tcp
just runlog-bridge-ws
just runlog-summary
just runlog-manifest
just runlog-sidecars
just runlog-sim-verify
just runlog-rerun
just runlog-rerun-bridge
just runlog-bridge-export-rerun
```

`runlog-bridge-stdio-safe` exercises the Agent Bridge read-only safe mode: `sim.status`/`log.replay` are allowed, while `sim.step`, `intervention.apply`, `log.stop`, and file-writing `export.rerun` requests are logged as blocked bridge error responses. Outside safe mode, `intervention.apply` supports deterministic `set_velocity`, `translate_object`, and `set_pose` interventions; `log.stop` finalizes the run log without trailing events; `export.rerun` converts a validated run log to a `.rrd` recording and logs the generated artifact. `pid-sim-bridge-tcp` exposes the newline-delimited JSON-RPC protocol on localhost for one client connection; `pid-sim-bridge-ws` exposes JSON-RPC over a local RFC6455 WebSocket connection. Both write canonical run logs.

If you don’t have `just`: run `cargo run -p pid-sim --bin pid-sim-demo -- outputs/demo_runlog.jsonl`, then `cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/demo_runlog.jsonl`, then `cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- outputs/demo_runlog.jsonl`. For sidecar provenance, use `cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --write-sidecars outputs/demo_runlog.jsonl` followed by `cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --verify-sidecars outputs/demo_runlog.jsonl`.

## Engineering Plan (To “Finish” the Project)

Build order + acceptance criteria are in `grandplan.md` §A.7 (M0–M8): run logs + replay → Agent Bridge → minimal sim + `Flow_gt` → Rerun-based viewer → embedding harness → optional live transport/predictors → optional GauSS‑MI uncertainty + view selection.
Note: Custom Tauri+SparkJS UI is deferred to Phase 4.
If you use an external simulator backend (Isaac/MuJoCo/etc.), treat it as an adapter that still emits the canonical run log, logs backend/solver config via `config_logged`, and is controlled via the Agent Bridge surface. Replay/provenance gates validate `run_started`/`config_logged` config-hash consistency, expose the surviving `config_hash` in summary/manifest sidecars, verify sidecars against current run logs, and allow read-only `log.replay` in bridge safe mode.

## Docset-Wide Final Solution

The ten-scientist consensus decision record lives in `grandplan.md` §A.8. The short version is:

```text
run log = source of truth
Agent Bridge = only control plane
Rerun = Phases 1-3 diagnostic/time-machine viewer
Tauri/SparkJS = Phase 4 control/editor/custom-rendering shell
```

Final 10-step build path:

1. Keep Exp0/geometry gates strict.
2. Define the canonical `pid-runlog` event schema.
3. Implement deterministic replay.
4. Route all GUI/script/LLM actions through the Agent Bridge.
5. Build the minimal object sim and simulator-derived `Flow_gt`.
6. Convert run logs into Rerun recordings/blueprints.
7. Connect the offline embedding harness to one small real VLA/task capture with labels, attribution probes, and non-PID baselines.
8. Gate optional live transport and external `Flow_pred` services behind the same run-log schema.
9. Add Tauri/SparkJS only after the Rerun workflow works.
10. Add license/provenance automation for dependencies, models, datasets, generated assets, and sidecars.

## Citation

```bibtex
@software{prisoma,
  title = {Prisoma: Partial Information Decomposition for Vision-Language-Action Models},
  year = {2026},
  url = {https://github.com/sepahead/prisoma}
}
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
