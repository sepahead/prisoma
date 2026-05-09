# Agent Notes (pid_vla)

These notes help future coding agents work on this repo without introducing accidental hallucinations or doc drift.

## Ground rules

- `grandplan.md` is the canonical research + engineering spec; keep `README.md`, `ARCHITECTURE.md`, `DIAGRAMS.md`, `EXPERIMENTS.md`, and `pidsplatspecs.md` consistent with it (current docset: v10.1). The Rerun/Tauri/SparkJS decision record is `grandplan.md` §A.8.
- Avoid hard-coded performance, cost, or roadmap claims unless they are backed by a committed source or a clearly labeled measurement in this repo.
- Network access may be restricted; prefer `outputs/arxiv_ref_cache.json` for source verification when possible.

## Repo reality

- Implemented today: Rust estimators in `crates/pid-core`, PyO3 bindings in `crates/pid-python`, M1 run-log validation/replay/summary/manifest groundwork in `crates/pid-runlog`, local Agent Bridge dispatch/JSON-RPC request groundwork in `crates/pid-bridge`, deterministic object-sim/`Flow_gt` bridge demo and verification code in `crates/pid-sim`, and the Rust Experiment 0 runner (`just exp0` / `just exp0-bin`, or run the equivalent `cargo` commands below).
- Many simulation/visualization components are specifications only (see `grandplan.md` §A.7 milestones); do not claim non-existent crates/scripts/assets are runnable unless they are added in the same change.
- Docset-wide final solution: run log is the source of truth, Agent Bridge is the only control plane, Rerun is the Phases 1–3 diagnostic viewer, and Tauri/SparkJS is deferred Phase 4 UI/custom rendering.

## Useful commands

- Search: `rg -n "pattern"`
- Tests: `just test` (or `cargo test` if `just` isn’t installed)
- Estimator gate:
  - `just exp0` (or `cargo test -p pid-core exp0 -- --nocapture`)
  - `just exp0-bin` (or `cargo run -p pid-core --bin exp0`)
- Run-log smoke:
  - `just runlog-demo`
  - `just runlog-bridge-demo`
  - `just runlog-validate`
  - `just runlog-summary`
  - `just runlog-manifest`
  - `just runlog-sim-verify`
  - `just runlog-replay`
  - `just runlog-rerun`
