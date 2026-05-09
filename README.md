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
> - `THIRD_PARTY_NOTICES.md` — Release-governance starter notices/checklist

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

PID‑VLA is a research toolkit for diagnosing **Vision‑Language‑Action (VLA)** policies using **Partial Information Decomposition (PID)** (shared‑exclusions `I^sx_∩`) and related information‑theoretic controls. The project is **gate‑driven**: do not interpret PID atoms on real embeddings until the estimator + geometry gates pass.

## Hypotheses (Docset v10.1)

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

## Doc Audits

- `python scripts/audit_grandplan.py --check-italic-titles` (arXiv coverage + title drift; uses `outputs/arxiv_ref_cache.json`)
- `python scripts/audit_grandplan_claims.py` (heuristic scan for unqualified venue/perf claims)
- `python scripts/audit_docset_claims.py` (same heuristic scan across the canonical docset + `findings.md`)
- If you have `just`: `just docs-audit`

## Repo Status (What Actually Exists)

- Implemented: `crates/pid-core`, `crates/pid-python` (`pid_core_rs`), `crates/pid-runlog` (M1 JSONL schema + replay/validate/compare/summary/manifest/sidecar CLI), `crates/pid-bridge` (local Agent Bridge request/response dispatch core + JSON-RPC-shaped request/response conversion + contract export), `crates/pid-sim` (deterministic object sim + `Flow_gt`/bridge demos, stdio JSON-RPC bridge, flow verification, action replay checks, and a labeled toy VLA/task harness), `crates/pid-rerun` (prototype Rerun logging + validated run-log replay adapter with summary/provenance/validation diagnostics), and the Experiment 0 runner (`just exp0`, `just exp0-bin`, `just exp0-runlog`).
- Specified: A fuller Rerun-based diagnostic viewer (Phases 1-3) and deferred Tauri/SparkJS UI (Phase 4). Start at `grandplan.md` §A.7.

## Quick Start (Exp0 Gate)

```bash
# optional: nix develop
cargo test
just exp0
just exp0-bin
just exp0-runlog
```

If you don’t have `just`: `cargo test` and `cargo run -p pid-core --bin exp0`. To export canonical Exp0 evidence, run `cargo run -p pid-core --bin exp0 -- --summary-json outputs/exp0_summary.json --runlog outputs/exp0_runlog.jsonl`, then validate it with `cargo run -p pid-runlog --bin pid-runlog-replay -- --validate outputs/exp0_runlog.jsonl`.
See `findings.md` for the latest repo-local Exp0 interpretation notes.

## Quick Start (Tiny Labeled Harness)

```bash
just toy-harness
```

If you don’t have `just`: run `cargo run -p pid-sim --bin pid-toy-harness -- --summary-json outputs/toy_vla_summary.json --runlog outputs/toy_vla_runlog.jsonl`, then validate it with `cargo run -p pid-runlog --bin pid-runlog-replay -- --validate outputs/toy_vla_runlog.jsonl`. This is a deterministic toy task, not VLA evidence; it exists to exercise first-class label events, a replay-linked toy `(V,L,D,A)` embedding contract, PID/CI features, non-PID baselines, summary artifacts, and canonical run-log export end to end.

## Quick Start (M1 Run Log)

```bash
just runlog-demo
just bridge-contract
just runlog-replay
just runlog-validate
just runlog-bridge-demo
just runlog-bridge-stdio
just runlog-summary
just runlog-manifest
just runlog-sidecars
just runlog-sim-verify
just runlog-rerun
just runlog-rerun-bridge
```

If you don’t have `just`: run `cargo run -p pid-sim --bin pid-sim-demo -- outputs/demo_runlog.jsonl`, then `cargo run -p pid-runlog --bin pid-runlog-replay -- --validate outputs/demo_runlog.jsonl`, then `cargo run -p pid-runlog --bin pid-runlog-replay -- outputs/demo_runlog.jsonl`.

## Engineering Plan (To “Finish” the Project)

Build order + acceptance criteria are in `grandplan.md` §A.7 (M0–M8): run logs + replay → Agent Bridge → minimal sim + `Flow_gt` → Rerun-based viewer → embedding harness → optional live transport/predictors → optional GauSS‑MI uncertainty + view selection.
Note: Custom Tauri+SparkJS UI is deferred to Phase 4.
If you use an external simulator backend (Isaac/MuJoCo/etc.), treat it as an adapter that still emits the canonical run log and is controlled via the Agent Bridge surface.

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
7. Add one small VLA embedding harness with labels and non-PID baselines.
8. Gate optional live transport and external `Flow_pred` services behind the same run-log schema.
9. Add Tauri/SparkJS only after the Rerun workflow works.
10. Add license/provenance automation for dependencies, models, datasets, generated assets, and sidecars.

## Citation

```bibtex
@software{pid_vla,
  title = {PID-VLA: Partial Information Decomposition for Vision-Language-Action Models},
  year = {2026},
  url = {https://github.com/your-org/pid-vla}
}
```
