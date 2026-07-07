# Agent Notes (prisoma)

Operating rules and a ground-truth inventory for anyone — human or agent — working in this
repository. The purpose of this file is to prevent two failure modes: **hallucinated
capabilities** (claiming things exist that don't) and **doc drift** (statements that stop
being true as the code moves).

> **Single source of truth for the Rust PID estimators: [`pid-rs`](https://github.com/sepahead/pid-rs).**
> `pid-core`, `pid-python`, and `pid-runlog` are **not** vendored here — do **not** re-add copies.
> They are pinned as the `pid-rs/` git submodule; the local crates path-depend into
> `pid-rs/crates/*`. Edit the estimator core upstream in `pid-rs` (then bump the submodule),
> never here. Run its binaries via
> `cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --bin exp0` and
> `cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay`.

## Ground rules

1. **`grandplan.md` is canonical.** It is the research + engineering spec; keep `README.md`,
   `ARCHITECTURE.md`, `DIAGRAMS.md`, `EXPERIMENTS.md`, and `pidsplatspecs.md` consistent with
   it (current docset: **v10.7**). The Rerun/Tauri/SparkJS decision record is `grandplan.md`
   §A.8; the hypothesis registry and falsifiability contracts are §14.1; the preregistered
   statistical analysis plan is §14.8.
2. **Gate discipline.** Do not interpret PID atoms on real embeddings until the Exp0 +
   geometry gates pass (Exp0 currently reports **NO-GO** on synthetic high-d controls — stricter
   under pid-rs 0.4.0's bias-corrected diagnostics, PIVOT under 0.3.0; that
   is the gate working, see `findings.md`). One (PID measure, preprocessing, estimator
   config) tuple = one preregistered regime; never pool continuous `I^sx_∩` atoms with
   discrete `I_min` atoms (`grandplan.md` Warning 6 + §8.1.6).
3. **Honesty over roadmap.** No hard-coded performance, cost, or roadmap claims unless backed
   by a committed source or a clearly labeled in-repo measurement. Do not claim non-existent
   crates/scripts/assets are runnable unless they are added in the same change. The doc-audit
   scripts (`scripts/audit_*.py`) enforce this — run them before every PR.
4. **Source verification offline-first.** Network access may be restricted; prefer
   `outputs/arxiv_ref_cache.json` for citation verification when possible
   (`scripts/update_arxiv_ref_cache.py` refreshes it).
5. **No AI co-authors.** Never add Claude, AI assistants, or agents as commit/PR co-authors —
   no `Co-Authored-By:` trailer and no "Generated with Claude Code" / 🤖 marker in commit
   messages or pull-request descriptions.

## Architecture invariants (docset-wide final solution)

- The **run log is the source of truth** — every captured sample must be reconstructable from
  canonical run-log events.
- The **Agent Bridge is the only control plane** — observers, harnesses, and viewers drive
  nothing.
- **Rerun** is the Phases 1–3 diagnostic/time-machine viewer; **Tauri/SparkJS** is the
  deferred Phase 4 UI/custom-rendering shell.

## Repo reality — what actually exists

### Estimator core (`pid-rs/` submodule)

- **`pid-core`** — KSG MI (with an optional exact, deterministic data-parallel `parallel`
  rayon feature), continuous `I^sx_∩` (`IsxMethod::EhrlichKsg` and baselines), 3-source
  SxPID, hierarchical screening, Shannon invariants (`invariants.rs`: r̄/v̄), PLS supervised
  dimensionality reduction (`pls.rs`, NIPALS-PLS2), discrete 2- and 3-source PID via
  quantization with a Williams–Beer-style `I_min` minimum-specific-information redundancy —
  **not** discrete `i^sx_∩` (pid-core 0.3.0 additionally ships a genuine discrete `i^sx_∩`
  in `sxpid.rs` for 2–4 sources, but it is **not yet wired into the offline harness**; see
  `grandplan.md` §8.1.6) — block-bootstrap uncertainty quantification, a `pipeline.rs`
  composition layer (PLS→PID3, per-atom bootstrap CIs, single-source permutation tests,
  LOO-CV PLS component selection, all-pairs PID2 screening, generic
  `bootstrap_rows_stats`/`permutation_rows_pvalue` row-resampling helpers), an
  L2-regularized logistic-regression classifier (`logistic.rs`, Newton-IRLS), geometry and
  intrinsic-dimension diagnostics, and the Experiment 0 runner (`bin/exp0.rs`) with a
  `--strict-gate` flag for CI enforcement and the opt-in `--bootstrap`/`--permutation`
  uncertainty gate.
- **`pid-python`** — PyO3 bindings (`pid_core_rs`; 18 exported functions, including
  `compute_pid3`, `compute_discrete_pid2`/`3`, the three 0.3.0
  `compute_discrete_sxpid2`/`3`/`_n` (n = 2–4), `pls_transform`, `standardize`,
  `pca_transform`, `hash_project`).
- **`pid-runlog`** — M1 run-log schema with validation/replay/summary/manifest/sidecar
  write-and-verify, the `attribution_logged` event schema for H9 probes, and the
  wall-clock-excluded `logical_trace_hash`.

### Local crates (`crates/`)

- **`pid-bridge`** — Agent Bridge dispatch, JSON-RPC request/response conversion, and
  bridge/run-log contract export.
- **`pid-sim`** — deterministic object sim with `Flow_gt` plus a baseline `flow_pred` bridge
  demo; stdio/TCP/WebSocket JSON-RPC bridges; safe-mode `log.replay`; bridge
  `log.start`/`log.stop`, deterministic `intervention.apply`, and `export.rerun`; flow
  checks and action/intervention replay verification; the toy labeled harness; a
  `PhysicsBackend` trait with a null adapter and a **real `rapier3d-f64` backend**
  (gravity/contacts/friction, deterministic; behind the `rapier` feature) plus a scripted
  push-to-goal manipulation (`manipulation.rs`, `pid-rapier-harness`) emitting canonical
  run-log events with real `Flow_gt` and physics-derived labels; and the **offline
  `(V,L,D,A)` artifact-to-runlog harness** with: all-pairs `V/L/D→A` PID screens (plus
  train-split-only screens when a metadata split is present), standardization provenance,
  geometry diagnostics/gates, strict fail-closed modes
  (label/geometry/held-out-split/class-coverage/episode-disjoint/axis-provenance),
  deterministic sample-level, episode-grouped, and metadata-split held-out
  majority/1-NN/nearest-centroid baselines (accuracy, balanced accuracy, centroid AUROC),
  a SAFE-class held-out logistic-regression failure detector (`heldout_logreg_vlda`;
  train-fit, held-out-scored), held-out per-sample prediction records, failure-class
  confusion/rate diagnostics, `--pid-mode continuous|discrete|discrete-pls` with per-pair
  `discrete_saturation` diagnostics, and a high-dimensional synthetic VLDA fixture
  (`offline_vlda_highdim_fixture.json`: v=128, l=64, d=32, a=7).
- **`pid-rerun`** — run-log→Rerun conversion and a validated replay adapter with
  summary/provenance/validation diagnostics; replay summaries distinguish unique metric
  names from total metric-event counts; surfaces `attribution_logged` events (see below).

### Python experiments (`experiments/`, tracked packages)

- **`safe_adapter/`** — the **M5 critical-path producer**: converts released SAFE VLA
  rollouts into the `(V,L,D,A)`+labels harness contract with honest per-axis
  `{v,l,d,a}_provenance` markers and the §7.6.3 hook-probe.
- **`attribution/`** — faithfulness-checked H9 probe (epsilon-/AttnLRP + grad×input on a
  small reference model; deletion-AOPC vs random control) emitting `attribution_logged`
  events that pass `pid-runlog-replay --validate`. Production VLAs should swap the
  reference model for LXT/AttnLRP.

### Attribution / H9 tooling

Attribution methods (LRP, Integrated Gradients, DeepLIFT, Grad-CAM, TCAV,
saliency/SmoothGrad, occlusion, SHAP-style probes) are **H9 companion
diagnostics/baselines**, never substitutes for PID gates. The `attribution_logged` run-log
event carries method, target_output, layer, modality, baseline, score_hash,
faithfulness_check, and artifact_uri. The `pid-rerun` adapter surfaces each event as a
plottable faithfulness verdict (`attributions/faithfulness/{method}`: 1.0 pass / 0.0 fail),
a provenance text line, and — when `artifact_uri` points to a NumPy `.npy` as the probe
writes — the per-element relevance values (capped at 1024) as a `Scalars` series at
`attributions/relevance/{method}`, via a small dependency-free `.npy` parser (best-effort;
missing/unparseable artifacts are skipped). Multi-panel 2-D heatmap blueprints remain
future work. The quantitative H9 agreement criterion is preregistered in `grandplan.md`
§14.1.

### NCP observer (`crates/ncp-observer`, optional)

A **read-only** Neuro-Cybernetic Protocol tap that subscribes to a NEST/Engram session's
Zenoh data planes and emits an `OfflineVldaDataset` artifact (for `pid-offline-harness`)
plus canonical run-log events (`EmbeddingContract`/`EmbeddingCaptured`/`LabelObserved`).

- **Honours the three invariants:** the run log is the source of truth, the observer drives
  nothing (the Agent Bridge stays the only control plane), and all NCP-specific mapping
  lives in this crate.
- **Pinned dependency:** git-depends on the published NCP repo
  (<https://github.com/sepahead/NCP>, tag `v0.6.0`) and pulls Zenoh — no sibling checkout
  required.
- **Workspace-excluded by design:** it is in `Cargo.toml` `exclude`, not a member, because a
  broken dependency in a *member* would fail manifest resolution for **every** `cargo`
  command (including `-p`-scoped ones). Exclusion keeps the PID estimator gates green on
  fresh checkouts and CI. Build it explicitly:
  `cargo build --manifest-path crates/ncp-observer/Cargo.toml`.
- **Off the critical path:** an optional `(V,L,D,A)` source only — grandplan does not depend
  on Engram; the M5 critical path is `experiments/safe_adapter`, and the pure-PID stack
  builds/tests/gates green with no NCP/Engram/Zenoh dependency.
- **Current state (as of docset v10.6):** emits a **validating** run log (monotonic clock;
  contract deferred to the first fully-populated sample; empty-axis ticks excluded and
  counted), **registers the dataset artifact** (`ArtifactLogged` + sha256), handles D
  **arrival reordering** (grace window + `seq_late` patch), is **reset-safe** (FIFO
  eviction + epochs), finalizes on **SIGTERM/SIGHUP**, and reports capture quality
  (`ObserverStats`).
- **Still exploratory-only** (below the M5 contract) until the external Engram publisher
  stamps observation `seq` (so D is exactly aligned, not recency-fallback), a language
  channel is present (so `L` is real, not excluded), and `metadata.split`/`episode_id`/
  `success` structure lands. See `crates/ncp-observer/README.md` and the developer handoff
  `NCP_DEV_PROMPT.md`.

### Specified but not built

Several simulation/visualization components are specifications only (see `grandplan.md`
§A.7 milestones) — notably the fuller Rerun-based viewer phases and the deferred
Tauri/SparkJS shell. Do not describe them as runnable.

## Gates before any PR or commit

```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
python scripts/audit_docset_claims.py
python scripts/audit_grandplan.py --check-italic-titles
```

Or the wrappers: `just test` and `just docs-audit`. The estimator gate itself is
`just exp0-bin` (prints the GO/PIVOT/NO-GO verdict).

## Useful commands

- Search: `rg -n "pattern"`
- Tests: `just test` (or `cargo test` if `just` isn't installed)
- Estimator gate:
  - `just exp0` (or `cargo test --manifest-path pid-rs/crates/pid-core/Cargo.toml exp0 -- --nocapture`)
  - `just exp0-bin` (or `cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --bin exp0`)
  - `just exp0-runlog` (or `cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --bin exp0 -- --summary-json outputs/exp0_summary.json --runlog outputs/exp0_runlog.jsonl`)
- Toy labeled harness:
  - `just toy-harness` (or `cargo run -p pid-sim --bin pid-toy-harness -- --summary-json outputs/toy_vla_summary.json --runlog outputs/toy_vla_runlog.jsonl`)
- Offline VLDA embedding harness:
  - `just offline-harness` (or `cargo run -p pid-sim --bin pid-offline-harness -- --input crates/pid-sim/fixtures/offline_vlda_fixture.json --summary-json outputs/offline_vlda_summary.json --runlog outputs/offline_vlda_runlog.jsonl`)
  - `just offline-harness-require-labels` — exercises `--require-success-labels` on the labeled fixture.
  - `just offline-harness-require-heldout` — exercises `--require-heldout-split`; the checked fixture has `metadata.split=train/test` assignments and passes this strict path.
  - `just offline-harness-require-heldout-class-coverage` — exercises `--require-heldout-class-coverage`; the checked fixture has both classes in train/test subsets and passes.
  - `just offline-harness-require-heldout-episode-disjoint` — exercises `--require-heldout-episode-disjoint`; the checked fixture has disjoint train/test `episode_id` sets and passes.
  - `just offline-harness-strict` — exercises `--require-geometry-pass`; the checked fixture is *expected* to exit nonzero while writing a valid failed run log (fail-closed demonstration).
  - `just offline-harness-highdim` — the high-dimensional synthetic fixture (v=128, l=64, d=32, a=7, 48 samples).
  - `just offline-harness-discrete` — `--pid-mode discrete --discrete-bins 8` (quantized `I_min` PID with per-pair `discrete_saturation` diagnostics; expect `saturation_warning=true` on the tiny smoke fixtures — that is the §8.1.6 gate working).
  - `just offline-harness-discrete-pls` — `--pid-mode discrete-pls --pls-components 2 --discrete-bins 8` on the high-dim fixture (PLS-project sources toward `A`, then discrete PID).
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
