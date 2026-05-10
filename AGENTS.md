# Agent Notes (pid_vla)

These notes help future coding agents work on this repo without introducing accidental hallucinations or doc drift.

## Ground rules

- `grandplan.md` is the canonical research + engineering spec; keep `README.md`, `ARCHITECTURE.md`, `DIAGRAMS.md`, `EXPERIMENTS.md`, and `pidsplatspecs.md` consistent with it (current docset: v10.1). The Rerun/Tauri/SparkJS decision record is `grandplan.md` §A.8.
- Avoid hard-coded performance, cost, or roadmap claims unless they are backed by a committed source or a clearly labeled measurement in this repo.
- Network access may be restricted; prefer `outputs/arxiv_ref_cache.json` for source verification when possible.

## Repo reality

- Implemented today: Rust estimators in `crates/pid-core`, PyO3 bindings in `crates/pid-python`, M1 run-log validation/replay/summary/manifest/sidecar groundwork in `crates/pid-runlog`, local Agent Bridge dispatch/JSON-RPC request/response and contract export groundwork in `crates/pid-bridge`, deterministic object-sim/`Flow_gt` plus baseline `flow_pred` bridge demo, stdio/TCP/WebSocket JSON-RPC bridges, safe-mode `log.replay`, bridge `log.start`/`log.stop`, deterministic bridge `intervention.apply`, bridge `export.rerun`, flow checks, action/intervention replay verification code, toy labeled harness, and offline `(V,L,D,A)` artifact-to-runlog harness with deterministic success-label baselines in `crates/pid-sim`, Rerun run-log conversion with summary/provenance/validation diagnostics in `crates/pid-rerun`, and the Rust Experiment 0 runner (`just exp0` / `just exp0-bin`, or run the equivalent `cargo` commands below).
- Many simulation/visualization components are specifications only (see `grandplan.md` §A.7 milestones); do not claim non-existent crates/scripts/assets are runnable unless they are added in the same change.
- Docset-wide final solution: run log is the source of truth, Agent Bridge is the only control plane, Rerun is the Phases 1–3 diagnostic viewer, and Tauri/SparkJS is deferred Phase 4 UI/custom rendering.

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
