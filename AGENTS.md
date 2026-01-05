# Agent Notes (pid_vla)

These notes help future coding agents work on this repo without introducing accidental hallucinations or doc drift.

## Ground rules

- `grandplan.md` is the canonical research + engineering spec; keep `README.md`, `ARCHITECTURE.md`, `DIAGRAMS.md`, `EXPERIMENTS.md`, and `pidsplatspecs.md` consistent with it.
- Avoid hard-coded performance, cost, or roadmap claims unless they are backed by a committed source or a clearly labeled measurement in this repo.
- Network access may be restricted; prefer `outputs/arxiv_ref_cache.json` for source verification when possible.

## Repo reality

- Implemented today: Rust estimators in `crates/pid-core`, PyO3 bindings in `crates/pid-python`, and the Rust Experiment 0 runner (`just exp0`, `just exp0-bin`).
- Many simulation/visualization components are specifications only; do not claim non-existent crates/scripts/assets are runnable unless they are added in the same change.

## Useful commands

- Search: `rg -n "pattern"`
- Tests: `just test`
- Estimator gate: `just exp0` (tests) and `just exp0-bin` (binary)
