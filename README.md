# PID-VLA

> **Docs (start here):**
> - `grandplan.md` — Canonical spec (definitions, gates, hypotheses, engineering plan)
> - `EXPERIMENTS.md` — What to run + what to log (protocols)
> - `ARCHITECTURE.md` — Target system design (PID‑Splat)
> - `DIAGRAMS.md` — Architecture + control plane diagrams
> - `pidsplatspecs.md` — Simulation/spec details (PID‑Splat)
> - `uidesigner/UI.md` — UI/UX spec (viewer-first; ordered by milestones)
> - `GAUSS_MI_INTEGRATION.md` — Optional: 3DGS uncertainty + view selection (spec)
> - `WORLD_WARP_INTEGRATION.md` — Optional: external world‑model baseline (spec)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

PID‑VLA is a research toolkit for diagnosing **Vision‑Language‑Action (VLA)** policies using **Partial Information Decomposition (PID)** (shared‑exclusions `I^sx_∩`) and related information‑theoretic controls. The project is **gate‑driven**: do not interpret PID atoms on real embeddings until the estimator + geometry gates pass.

## Hypotheses (Docset v10.0)

The canonical registry + falsification criteria live in `grandplan.md` (§14.1).

| Hypothesis | One‑line testable claim |
|---|---|
| **H1** | PID/CI features predict failure labels beyond strong baselines. |
| **H2** | Redundancy predicts robustness to single‑modality ablation (matched controls). |
| **H3** | Uniques predict intervention sensitivity (matched‑strength perturbations). |
| **H4** | Memorization vs generalization induces systematic PID/CI shifts. |
| **H5** | Long‑horizon failures correlate with temporal PID/CI degradation. |
| **H6** | Safety tasks show distinctive V–L integration patterns (only with proper labels/controls). |
| **H7** | Flow‑as‑Bridge enables stage‑wise diagnostics and embodiment‑agnostic comparisons. |
| **H8** | Geometry diagnostics determine which estimator regime is valid. |

## Experiments (Run Order)

Details and logging requirements live in `EXPERIMENTS.md`; estimator gates and confounds live in `grandplan.md`.

1. **Exp0** — Estimator + geometry gate (GO/PIVOT/NO‑GO).
2. **Exp1** — Pick‑and‑place + perturbations (H1–H4).
3. **Exp2** — Long‑horizon composition (H5).
4. **Exp3** — Instruction/visual/physics perturbations (H1–H6).
5. **Exp4** — Flow‑as‑Bridge bring‑up with simulator `Flow_gt` (H7).
6. **Exp5** — Cross‑embodiment replication (H4/H7).

## Repo Status (What Actually Exists)

- Implemented: `crates/pid-core`, `crates/pid-python` (`pid_core_rs`), and the Experiment 0 runner (`just exp0`, `just exp0-bin`).
- Specified (not yet implemented here): the PID‑Splat harness (run logs + replay, Agent Bridge, sim loop, UI, optional live transports/predictors). Start at `grandplan.md` §A.7.

## Quick Start (Exp0 Gate)

```bash
# optional: nix develop
cargo test
just exp0
just exp0-bin
```

If you don’t have `just`: `cargo test` and `cargo run -p pid-core --bin exp0`.

## Engineering Plan (To “Finish” the Project)

Build order + acceptance criteria are in `grandplan.md` §A.7 (M0–M8): run logs + replay → Agent Bridge → minimal sim + `Flow_gt` → viewer‑first UI → embedding harness → optional live transport/predictors → optional GauSS‑MI uncertainty + view selection.

## Citation

```bibtex
@software{pid_vla,
  title = {PID-VLA: Partial Information Decomposition for Vision-Language-Action Models},
  year = {2026},
  url = {https://github.com/your-org/pid-vla}
}
```
