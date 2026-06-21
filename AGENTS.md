# Agent Notes (pid_vla)

These notes help future coding agents work on this repo without introducing accidental hallucinations or doc drift.

> **Single source of truth for the Rust PID estimators: [`pid-rs`](https://github.com/sepahead/pid-rs).**
> `pid-core` and `pid-runlog` are **not** vendored in this repo — do **not** re-add copies. They are
> pinned as the `pid-rs/` git submodule; the other crates path-depend into `pid-rs/crates/*`. Edit
> the estimator core upstream in `pid-rs` (then bump the submodule), never here. Run its binaries via
> `cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --bin exp0` /
> `--manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay`.

## Ground rules

- `grandplan.md` is the canonical research + engineering spec; keep `README.md`, `ARCHITECTURE.md`, `DIAGRAMS.md`, `EXPERIMENTS.md`, and `pidsplatspecs.md` consistent with it (current docset: v10.3). The Rerun/Tauri/SparkJS decision record is `grandplan.md` §A.8.
- Avoid hard-coded performance, cost, or roadmap claims unless they are backed by a committed source or a clearly labeled measurement in this repo.
- Network access may be restricted; prefer `outputs/arxiv_ref_cache.json` for source verification when possible.
- Do not add Claude, AI assistants, or agents as commit/PR co-authors — no `Co-Authored-By:` trailer and no "Generated with Claude Code" / 🤖 line in commit messages or pull-request descriptions.

## Repo reality

- Implemented: Rust estimators in `pid-rs/crates/pid-core` (PLS supervised dimensionality reduction, discrete 2- and 3-source PID via quantization with a Williams–Beer-style `I_min` minimum-specific-information redundancy — **not** discrete `i^sx_∩`; see `grandplan.md` §8.1.6 — block bootstrap uncertainty quantification, a `pipeline.rs` composition layer (PLS→PID3, per-atom bootstrap CIs, single-source permutation tests, LOO-CV PLS component selection, all-pairs PID2 screening, plus generic `bootstrap_rows_stats`/`permutation_rows_pvalue` row-resampling uncertainty helpers), an L2-regularized logistic-regression classifier (`logistic.rs`, Newton-IRLS), an optional `parallel` feature (rayon) for exact, deterministic data-parallel KSG kNN, 3-source SxPID, hierarchical screening, Shannon invariants), PyO3 bindings in `pid-rs/crates/pid-python` (15 exported functions including `compute_pid3`, `compute_discrete_pid2`, `compute_discrete_pid3`, `pls_transform`, `standardize`, `pca_transform`, `hash_project`), M1 run-log validation/replay/summary/manifest/sidecar write-and-verify groundwork in `pid-rs/crates/pid-runlog` (including `attribution_logged` event schema for H9 attribution probes), local Agent Bridge dispatch/JSON-RPC request/response and contract export groundwork in `crates/pid-bridge`, deterministic object-sim/`Flow_gt` plus baseline `flow_pred` bridge demo, stdio/TCP/WebSocket JSON-RPC bridges, safe-mode `log.replay`, bridge `log.start`/`log.stop`, deterministic bridge `intervention.apply`, bridge `export.rerun`, flow checks, action/intervention replay verification code, toy labeled harness, offline `(V,L,D,A)` artifact-to-runlog harness with all-pairs `V/L/D→A` PID screens plus train-split-only PID screens when a metadata split is present, standardization provenance, geometry diagnostics/gates, strict label/geometry/held-out-split/held-out-class-coverage/held-out-episode-disjoint modes, deterministic sample-level, episode-grouped, and metadata-split held-out majority/1-NN/nearest-centroid success-label baselines with accuracy, balanced accuracy, centroid AUROC, held-out class-coverage and episode-disjointness reports, a SAFE-class held-out logistic-regression failure-detector baseline (`heldout_logreg_vlda`; train-fit, held-out-scored), held-out per-sample prediction records in summaries/run logs, held-out failure-class confusion/rate diagnostics in `crates/pid-sim`, a physics backend trait with a null adapter and a **real `rapier3d-f64` backend** (gravity/contacts/friction, deterministic; behind the `rapier` feature) plus a scripted push-to-goal manipulation (`manipulation.rs`, `pid-rapier-harness`) emitting canonical run-log events with real `Flow_gt` and physics-derived labels, the Exp0 opt-in `--bootstrap`/`--permutation` uncertainty gate, the Python `experiments/safe_adapter` (released-SAFE-rollout → `(V,L,D,A)` converter + §7.6.3 hook-probe) and `experiments/attribution` (faithfulness-checked H9 probe + `attribution_logged` emission), high-dimensional synthetic VLDA fixture (`offline_vlda_highdim_fixture.json` with v=128, l=64, d=32, a=7), replay summaries that distinguish unique metric names from total metric event counts, Rerun run-log conversion with summary/provenance/validation diagnostics in `crates/pid-rerun`, and the Rust Experiment 0 runner with `--strict-gate` flag for CI enforcement (`just exp0` / `just exp0-bin`, or run the equivalent `cargo` commands below).
- NCP observer (`crates/ncp-observer`): a read-only Neuro-Cybernetic Protocol tap that subscribes to a NEST/Engram session's Zenoh data planes and emits an `OfflineVldaDataset` artifact (for `pid-offline-harness`) plus canonical run-log events (`EmbeddingContract`/`EmbeddingCaptured`/`LabelObserved`). It honours the three rules — run log is the source of truth, the observer drives nothing (Agent Bridge stays the only control plane), and the NCP-specific mapping lives here. It git-depends on the published NCP repo (<https://github.com/sepahead/NCP>, tag `v0.5.0`) and pulls Zenoh. To keep NCP/Zenoh off fresh checkouts/CI, `ncp-observer` is **excluded** from the default workspace (`Cargo.toml` `exclude`), not a member — a broken dependency in a member would fail manifest resolution for *every* `cargo` command (including `-p`-scoped ones), so excluding it keeps the PID estimator gates green. Build it explicitly when the sibling is present: `cargo build --manifest-path crates/ncp-observer/Cargo.toml`. It is an **optional, non-critical-path** `(V,L,D,A)` source — grandplan does not depend on Engram; the M5 critical path is `experiments/safe_adapter`, and the pure-PID stack builds/tests/gates green with no NCP/Engram/Zenoh dependency. It is **exploratory-only** (below the M5 contract) until precise D `seq`-alignment, honest (non-zeroed) `L`, and `metadata.split`/`episode_id`/`success` structure land. See `crates/ncp-observer/README.md` and the developer handoff `NCP_DEV_PROMPT.md`.
- Many simulation/visualization components are specifications only (see `grandplan.md` §A.7 milestones); do not claim non-existent crates/scripts/assets are runnable unless they are added in the same change.
- Docset-wide final solution: run log is the source of truth, Agent Bridge is the only control plane, Rerun is the Phases 1–3 diagnostic viewer, and Tauri/SparkJS is deferred Phase 4 UI/custom rendering.
- Attribution methods (LRP, Integrated Gradients, DeepLIFT, Grad-CAM, TCAV, saliency/SmoothGrad, occlusion, SHAP-style probes) are H9 companion diagnostics/baselines. The `attribution_logged` run-log event exists in the schema (method, target_output, layer, modality, baseline, score_hash, faithfulness_check, artifact_uri) and `experiments/attribution/` now emits it from a faithfulness-checked probe (epsilon-/AttnLRP + grad×input on a small reference model; deletion-AOPC vs random control) that passes `pid-runlog-replay --validate`. Production VLAs should swap the reference model for LXT/AttnLRP. The `pid-rerun` run-log→Rerun adapter surfaces each `attribution_logged` event as a plottable faithfulness verdict (`attributions/faithfulness/{method}` scalar, 1.0 pass / 0.0 fail) a provenance text line (method/target/layer/modality/baseline/score), and — when the `artifact_uri` points to a NumPy `.npy` (as the probe writes) — the actual per-element relevance values (capped at 1024) as a `Scalars` series at `attributions/relevance/{method}`, read by a small dependency-free `.npy` parser (best-effort; missing/unparseable artifacts are skipped). Multi-panel 2-D heatmap blueprints remain future work.

## Useful commands

- Search: `rg -n "pattern"`
- Tests: `just test` (or `cargo test` if `just` isn’t installed)
- Estimator gate:
  - `just exp0` (or `cargo test --manifest-path pid-rs/crates/pid-core/Cargo.toml exp0 -- --nocapture`)
  - `just exp0-bin` (or `cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --bin exp0`)
  - `just exp0-runlog` (or `cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --bin exp0 -- --summary-json outputs/exp0_summary.json --runlog outputs/exp0_runlog.jsonl`)
- Toy labeled harness:
  - `just toy-harness` (or `cargo run -p pid-sim --bin pid-toy-harness -- --summary-json outputs/toy_vla_summary.json --runlog outputs/toy_vla_runlog.jsonl`)
- Offline VLDA embedding harness:
  - `just offline-harness` (or `cargo run -p pid-sim --bin pid-offline-harness -- --input crates/pid-sim/fixtures/offline_vlda_fixture.json --summary-json outputs/offline_vlda_summary.json --runlog outputs/offline_vlda_runlog.jsonl`)
  - `just offline-harness-require-labels` exercises `--require-success-labels` on the labeled fixture.
  - `just offline-harness-require-heldout` exercises `--require-heldout-split`; the checked fixture has `metadata.split=train/test` assignments and should pass this strict path.
  - `just offline-harness-require-heldout-class-coverage` exercises `--require-heldout-class-coverage`; the checked fixture has both success and failure labels in train/test subsets and should pass this strict path.
  - `just offline-harness-require-heldout-episode-disjoint` exercises `--require-heldout-episode-disjoint`; the checked fixture has disjoint train/test `episode_id` sets and should pass this strict path.
  - `just offline-harness-strict` exercises `--require-geometry-pass`; the checked fixture is expected to exit nonzero while writing a valid failed run log.
  - `just offline-harness-highdim` runs the high-dimensional synthetic fixture (v=128, l=64, d=32, a=7, 48 samples).
  - `just offline-harness-discrete` exercises `--pid-mode discrete --discrete-bins 8` (quantized `I_min` PID with per-pair `discrete_saturation` diagnostics; expect `saturation_warning=true` on the tiny smoke fixtures — that is the §8.1.6 gate working).
  - `just offline-harness-discrete-pls` exercises `--pid-mode discrete-pls --pls-components 2 --discrete-bins 8` on the high-dim fixture (PLS-project sources toward `A`, then discrete PID).
- Run-log smoke:
  - `just bridge-contract`
  - `just runlog-demo`
  - `just runlog-bridge-demo`
  - `just runlog-bridge-stdio-safe`
  - `just runlog-bridge-stdio`
  - `just runlog-bridge-tcp`
  - `just runlog-bridge-ws`
  - `just runlog-validate`
  - `just runlog-summary`
  - `just runlog-manifest`
  - `just runlog-sidecars`
  - `just runlog-sim-verify`
  - `just runlog-replay`
  - `just runlog-rerun`
  - `just runlog-rerun-bridge`
  - `just runlog-bridge-export-rerun`
