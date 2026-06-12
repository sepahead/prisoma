# Agent Notes (pid_vla)

These notes help future coding agents work on this repo without introducing accidental hallucinations or doc drift.

## Ground rules

- `grandplan.md` is the canonical research + engineering spec; keep `README.md`, `ARCHITECTURE.md`, `DIAGRAMS.md`, `EXPERIMENTS.md`, and `pidsplatspecs.md` consistent with it (current docset: v10.2). The Rerun/Tauri/SparkJS decision record is `grandplan.md` §A.8.
- Avoid hard-coded performance, cost, or roadmap claims unless they are backed by a committed source or a clearly labeled measurement in this repo.
- Network access may be restricted; prefer `outputs/arxiv_ref_cache.json` for source verification when possible.

## Repo reality

- Implemented: Rust estimators in `crates/pid-core` (PLS supervised dimensionality reduction, discrete PID via quantization with a Williams–Beer-style `I_min` minimum-specific-information redundancy — **not** discrete `i^sx_∩`; see `grandplan.md` §8.1.6 — block bootstrap uncertainty quantification, 3-source SxPID, hierarchical screening, Shannon invariants), PyO3 bindings in `crates/pid-python` (14 exported functions including `compute_pid3`, `compute_discrete_pid2`, `pls_transform`, `standardize`, `pca_transform`, `hash_project`), M1 run-log validation/replay/summary/manifest/sidecar write-and-verify groundwork in `crates/pid-runlog` (including `attribution_logged` event schema for H9 attribution probes), local Agent Bridge dispatch/JSON-RPC request/response and contract export groundwork in `crates/pid-bridge`, deterministic object-sim/`Flow_gt` plus baseline `flow_pred` bridge demo, stdio/TCP/WebSocket JSON-RPC bridges, safe-mode `log.replay`, bridge `log.start`/`log.stop`, deterministic bridge `intervention.apply`, bridge `export.rerun`, flow checks, action/intervention replay verification code, toy labeled harness, offline `(V,L,D,A)` artifact-to-runlog harness with all-pairs `V/L/D→A` PID screens plus train-split-only PID screens when a metadata split is present, standardization provenance, geometry diagnostics/gates, strict label/geometry/held-out-split/held-out-class-coverage/held-out-episode-disjoint modes, deterministic sample-level, episode-grouped, and metadata-split held-out majority/1-NN/nearest-centroid success-label baselines with accuracy, balanced accuracy, centroid AUROC, held-out class-coverage and episode-disjointness reports, held-out per-sample prediction records in summaries/run logs, held-out failure-class confusion/rate diagnostics in `crates/pid-sim`, physics backend trait with null adapter and Rapier3D stub (behind `rapier` feature flag), high-dimensional synthetic VLDA fixture (`offline_vlda_highdim_fixture.json` with v=128, l=64, d=32, a=7), replay summaries that distinguish unique metric names from total metric event counts, Rerun run-log conversion with summary/provenance/validation diagnostics in `crates/pid-rerun`, and the Rust Experiment 0 runner with `--strict-gate` flag for CI enforcement (`just exp0` / `just exp0-bin`, or run the equivalent `cargo` commands below).
- Many simulation/visualization components are specifications only (see `grandplan.md` §A.7 milestones); do not claim non-existent crates/scripts/assets are runnable unless they are added in the same change.
- Docset-wide final solution: run log is the source of truth, Agent Bridge is the only control plane, Rerun is the Phases 1–3 diagnostic viewer, and Tauri/SparkJS is deferred Phase 4 UI/custom rendering.
- Attribution methods (LRP, Integrated Gradients, DeepLIFT, Grad-CAM, TCAV, saliency/SmoothGrad, occlusion, SHAP-style probes) are documented as H9 companion diagnostics/baselines only. The `attribution_logged` run-log event now exists in the schema (method, target_output, layer, modality, baseline, score_hash, faithfulness_check, artifact_uri) but no Rerun attribution adapter is wired yet.

## Useful commands

- Search: `rg -n "pattern"`
- Tests: `just test` (or `cargo test` if `just` isn’t installed)
- Estimator gate:
  - `just exp0` (or `cargo test -p pid-core exp0 -- --nocapture`)
  - `just exp0-bin` (or `cargo run -p pid-core --bin exp0`)
  - `just exp0-runlog` (or `cargo run -p pid-core --bin exp0 -- --summary-json outputs/exp0_summary.json --runlog outputs/exp0_runlog.jsonl`)
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
