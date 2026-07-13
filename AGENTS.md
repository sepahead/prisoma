# Agent Notes (prisoma)

Operating rules and a ground-truth inventory for anyone — human or agent — working in this
repository. The purpose of this file is to prevent two failure modes: **hallucinated
capabilities** (claiming things exist that don't) and **doc drift** (statements that stop
being true as the code moves).

> **Estimator environment: `pid-rs` 1.0.0 (submodule `ac4a780`).** 1.0 makes continuous support
> **declared, never inferred** — a bare continuous config fails closed. Continuous shared
> exclusions, pipelines, hierarchy and hyperbolic paths are default-off `experimental-*` features.
> Datasets declare per-axis population support; computation status is `produced` /
> `produced_with_warning` / `abstained`, while separate population/measure/estimator/application
> verdicts govern interpretation. An **abstained estimate has
> no numeric placeholder** (no zero, no NaN, no metric event). Exact ties reject a *sample*, never
> the population law. Never auto-route a failed continuous term to discrete `I_min`: different
> measure, different estimand, never pooled.

> **Single source of truth for the Rust PID estimators: [`pid-rs`](https://github.com/sepahead/pid-rs).**
> `pid-core`, `pid-python`, and `pid-runlog` are **not** vendored here — do **not** re-add copies.
> They are pinned as the `pid-rs/` git submodule; the local crates path-depend into
> `pid-rs/crates/*`. Edit the estimator core upstream in `pid-rs` (then bump the submodule),
> never here. Run its binaries via
> `cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all --bin exp0` and
> `cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay`.

## Ground rules

1. **`grandplan.md` is canonical.** It is the research + engineering spec; keep `README.md`,
   `ARCHITECTURE.md`, `DIAGRAMS.md`, `EXPERIMENTS.md`, and `pidsplatspecs.md` consistent with
   it (current docset: **v12.5**). The Rerun/Tauri/SparkJS decision record is `grandplan.md`
   §16; the confirmatory claim registry (EC1, H1–H4) and PID kill rules are §4 and §3.8; the
   preregistered statistical analysis plan is §6.
2. **Gate discipline.** Do not interpret PID atoms on real embeddings. PID validity splits into
   four gates — population, measure, estimator, application (`grandplan.md` §7.1). The current
   high-d **MI/coherence path is NO-GO**; the continuous `I^sx_∩` **application gate is BLOCKED /
   NOT APPLICATION-VALIDATED**: default Experiment 0 includes a measure-mismatched zero-redundancy
   target, and `--strict-gate` gates the curated low-d MI band while only reporting atoms.
   Geometry diagnostics are not a substitute; sampled-mean δ is descriptive only. See
   `findings.md` and `grandplan.md` §7.2, §7.9. One (PID measure, preprocessing, estimator
   config) tuple = one preregistered regime; never pool continuous `I^sx_∩` atoms with discrete
   `I_min` atoms — `--pid-mode discrete` is Williams–Beer `I_min`, not discrete `i^sx_∩`
   (`grandplan.md` §7.6).
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
  **not** discrete `i^sx_∩` (pid-core additionally ships a genuine discrete `i^sx_∩`
  in `sxpid.rs` for 2–4 sources, but it is **not yet wired into the offline harness**; see
  `grandplan.md` §7.6) — block resampling plus an m-out-of-n **stability envelope** (not an
  n-sample CI), a `pipeline.rs`
  composition layer (PLS→PID3, per-atom bootstrap CIs, single-source permutation tests,
  LOO-CV PLS component selection, all-pairs PID2 screening, generic
  `bootstrap_rows_stats`/`permutation_rows_pvalue` row-resampling helpers), an
  L2-regularized logistic-regression classifier (`logistic.rs`, Newton-IRLS), geometry and
  intrinsic-dimension diagnostics, and the Experiment 0 runner (`bin/exp0.rs`) with a
  `--strict-gate` flag for curated low-d-band CI enforcement plus opt-in resampling and
  permutation diagnostics. Repair its separate MI/atom verdicts upstream in `pid-rs`.
- **`pid-python`** — typed PyO3 bindings (`pid_core_rs`) with a narrow stable 1.x surface:
  report-first conditional KSG MI, categorical shared-exclusions PID for 2–4 sources, a separately
  named categorical `I_min` comparator, fitted equal-width quantization, resource budgets, and
  diagnostics. Pre-1.0 scalar/research calls are absent from ordinary wheels and live only in the
  default-off `experimental.migration` module.
- **`pid-runlog`** — the canonical (EC1) run-log schema (`grandplan.md` §8.2) with
  validation/replay/summary/manifest/sidecar write-and-verify, the `attribution_logged` event
  schema for attribution/mechanistic probes (H4 / exploratory), and the wall-clock-excluded
  `logical_trace_hash`.

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
  committed NCP-publication verification (dataset/run-log hashes, canonical-log artifact binding,
  and a successful `complete` or `complete_with_warning` visible-receipt grade;
  degraded/uncommitted NCP artifacts reject),
  deterministic sample-level, episode-grouped, and metadata-split held-out
  majority/1-NN/nearest-centroid baselines (accuracy, balanced accuracy, centroid AUROC),
  a SAFE-class held-out logistic-regression failure detector (`heldout_logreg_vlda`;
  train-fit, held-out-scored), held-out per-sample prediction records, failure-class
  confusion/rate diagnostics, `--pid-mode none|continuous|discrete|discrete-pls` (`none` is the
  true zero-MI/PID dependency-firebreak path) with per-pair
  `discrete_saturation` diagnostics; a fail-closed typed H1 common-preflight validator/CLI with
  a schema-v2 representative mechanism scope, exact-byte content-addressed/strictly bound
  policy/instrumentation/manifests, clock/timing/lineage/fold checks,
  per-axis-scaled outputs, paired start/reset/RNG/input receipts, diagnostic-instrumentation
  noninterference, valid failed run logs for readable invalid inputs, and passing/failing fixtures;
  a deterministic finite-benchmark **Protocol A software reference** (`pid-h1-protocol-a`) that
  exact-binds that passed preflight chain, restores independent per-side clone state, reverses
  treatment order, records zero RNG draws, computes the frozen scaled response, and compares fixed
  design-only versus design+moderator ridge predictors out of outer fold. It is a synthetic scoring
  primitive only: no subprocess audit, stochastic-policy path, physical effect, Protocol B, or H1
  evidence is claimed; a deterministic synthetic **H2 fixed-horizon software reference**
  (`pid-h2-reference`) that exact-binds separately frozen analysis-plan, event-ontology,
  feature-contract, and split-manifest artifacts, then exercises task-family-held-out weighted
  fitting, grouped cross-fitted stratified reverse-KM IPCW, Horvitz–Thompson Brier arithmetic,
  competing-event classification, reliability bins, frozen alarm/nondetection accounting, and
  declared-payoff utility with explicit censoring abstentions. It is PID-free protocol arithmetic,
  not prospective capture, validated calibration, the comparator frontier, or H2 evidence; and a
  high-dimensional synthetic VLDA fixture
  (`offline_vlda_highdim_fixture.json`: v=128, l=64, d=32, a=7).
- **`pid-rerun`** — run-log→Rerun conversion and a validated replay adapter with
  summary/provenance/validation diagnostics; replay summaries distinguish unique metric
  names from total metric-event counts; surfaces `attribution_logged` events (see below).

### Python experiments (`experiments/`, tracked packages)

- **`safe_adapter/`** — the **reference `(V,L,D,A)` producer** for the confirmatory
  H-experiments (the S2/EC1 adapter contract): converts released SAFE VLA rollouts into the
  `(V,L,D,A)`+labels harness contract with honest per-axis `{v,l,d,a}_provenance` markers and a
  layerwise physics-decodability hook probe. Its default ingress is a finite NPZ/strict-JSON
  bundle bound by exact file hashes plus operator-declared source/split/rights and
  model/checkpoint/hook/tensor receipts; downloaded pickle is rejected by
  default, and the explicit legacy path is manifest-hashed plus NumPy-only restricted.
  Filename/metadata conflicts, unlisted/mismatched files, resource overruns, object/non-finite
  arrays, and unverified rights fail closed unless the named rights override is explicit.
  Synthetic conversion proves software readiness only; real safe re-export/capture and rights
  review remain open. The generic instrumented-versus-uninstrumented preflight validator is
  implemented in `pid-sim`, but `safe_adapter` does not yet produce the real paired policy
  evaluations required to clear it.
- **`attribution/`** — faithfulness-checked attribution/mechanistic probe (H4 / exploratory;
  epsilon-/AttnLRP + grad×input on a small reference model; deletion-AOPC vs random control)
  emitting `attribution_logged` events that pass `pid-runlog-replay --validate`. Production VLAs
  should swap the reference model for LXT/AttnLRP.

### Attribution / mechanistic-probe tooling (H4 / exploratory)

Attribution methods (LRP, Integrated Gradients, DeepLIFT, Grad-CAM, TCAV,
saliency/SmoothGrad, occlusion, SHAP-style probes) are **H4/exploratory companion
diagnostics/baselines**, never substitutes for PID gates. The `attribution_logged` run-log
event carries method, target_output, layer, modality, baseline, score_hash,
faithfulness_check, and artifact_uri. The `pid-rerun` adapter surfaces each event as a
plottable faithfulness verdict (`attributions/faithfulness/{method}`: 1.0 pass / 0.0 fail),
a provenance text line, and — when `artifact_uri` points to a NumPy `.npy` as the probe
writes — the per-element relevance values (capped at 1024) as a `Scalars` series at
`attributions/relevance/{method}`, via a small dependency-free `.npy` parser (best-effort;
missing/unparseable artifacts are skipped). Multi-panel 2-D heatmap blueprints remain
future work. Attribution agreement is an H4/exploratory diagnostic and must be grounded in
action and counterfactual effects, not treated as faithfulness by itself (`grandplan.md` §4,
§10.2).

### NCP observer (`crates/ncp-observer`, optional)

A **read-only** Neuro-Cybernetic Protocol tap for a conforming producer, intended to support a
future NEST/Engram session, that emits an `OfflineVldaDataset` artifact (for
`pid-offline-harness`) plus canonical run-log events
(`EmbeddingContract`/`EmbeddingCaptured`/`LabelObserved`). The named public `sepahead/engram`
repository is currently a README-only placeholder, so no public live Engram integration exists.

- **Honours the three invariants:** the run log is the source of truth, the observer drives
  nothing (the Agent Bridge stays the only control plane), and all NCP-specific mapping
  lives in this crate.
- **Pinned dependency:** the manifest pins the immutable NCP `v0.8.0` (wire 0.8) release and
  resolves from the published repository; no sibling checkout or path override is
  required.
- **Workspace-excluded by design:** it is in `Cargo.toml` `exclude`, not a member, because a
  broken dependency in a *member* would fail manifest resolution for **every** `cargo`
  command (including `-p`-scoped ones). Exclusion keeps root workspace resolution/build/test
  independent of NCP; it does not change the scientific PID verdicts. Build it explicitly:
  `cargo build --manifest-path crates/ncp-observer/Cargo.toml`.
- **Off the critical path:** an optional, read-only `(V,L,D,A)` source only — grandplan does
  not depend on Engram; the reference producer for the confirmatory H-experiments is
  `experiments/safe_adapter`, and the core builds with NCP disabled and runs its static non-PID
  label-baseline smoke with PID disabled (dependency firebreak, `grandplan.md` §8.9.3). This is
  groundwork for H1/H2, not either protocol. Workspace tests remain independent
  of NCP/Engram/Zenoh; the high-dimensional MI/coherence and application verdicts remain
  NO-GO/BLOCKED as stated above.
- **Integrity repair (2026-07-10; wire-0.8 migration reconciled 2026-07-13):** V, A, and D
  are joined only on the full driving-sensor `StreamPosition` (`{epoch, seq}`).
  `CommandFrame.source` and `ObservationFrame.source` must echo that position; a source-less
  command or plane observation is uncorrelatable and dropped (source absence is wire 0.8's
  replacement for the retired observation `seq == 0` sentinel). Pending V/A/D, closed receipts,
  and redelivery classification use the full key across epoch transitions; future-epoch
  passengers wait for a valid sensor to authorize transition. Complete validated-frame hashes
  make exact redelivery idempotent and conflicting evidence capture-invalid without mutating an
  emitted row/event. Raw decode accounting is observer-owned; duplicate JSON keys, invalid
  session/key routes, incomplete boundary state, and finite raw/frame/axis/resident/sample/output
  limits fail closed. Callback work crosses a bounded handoff to one owning worker. Finalization
  reconstructs and caps artifact + canonical-log bytes before no-replace/fsync installs, then
  commits their hashes with a publication receipt installed last; exact retries adopt only
  bounded byte-identical regular files at the original three canonical targets. `pid-offline-harness`
  hashes the exact parsed input snapshot and verifies the receipt, canonical log, exact dataset
  artifact identity, and visible-receipt grade; failed/uncommitted NCP input rejects. The CLI
  requires `--runlog`, exits nonzero for zero/degraded/invalid captures after preserving their
  diagnostic failed bundle, and library publication requires an explicit capture session plus a
  canonical run log before ingestion.
- **Honesty boundary:** `capture_integrity` is a visible-receipt/join grade, not delivery
  completeness. Own-stream gap detection, receipt timing, reconnect/QoS/clock evidence, producer
  authentication, and the deterministic protocol-fault observatory remain unbuilt. The NCP
  artifact declares no population support: continuous KSG/shared-exclusions requests abstain,
  `--pid-mode none` requests nothing, and quantized discrete `I_min` is at most a non-evidentiary
  diagnostic with population `NotEvaluated` and application `Blocked`. Use PID-disabled
  diagnostics/baselines by default until a real producer supplies justified per-axis declarations.
  This is not E4, EC1, live Engram validation, or security validation.
- **Still exploratory-only** (below the S2/EC1 adapter contract; optional M2 ecosystem item) until
  a conforming external publisher stamps every plane observation with its driving sensor
  `source`, a language channel is present (so `L` is real, not excluded), and
  `metadata.split`/`episode_id`/
  `success` structure lands. See `crates/ncp-observer/README.md` and the developer handoff
  `NCP_DEV_PROMPT.md`.

### Specified but not built

Several simulation/visualization components are specifications only (see `grandplan.md`
§12 milestones and §8.10 current-vs-target implementation) — notably the fuller Rerun-based
viewer phases and the deferred Tauri/SparkJS shell. Do not describe them as runnable.

## Gates before any PR or commit

```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
python scripts/audit_docset_claims.py
python scripts/audit_grandplan.py   # validates the R1-R112 reference ledger
```

Or the wrappers: `just test` and `just docs-audit`. The estimator gate itself is
`just exp0-bin` (prints the GO/PIVOT/NO-GO verdict).

## Useful commands

- Search: `rg -n "pattern"`
- Tests: `just test` (or `cargo test` if `just` isn't installed)
- Estimator gate:
  - `just exp0` (or `cargo test --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all exp0 -- --nocapture`)
  - `just exp0-bin` (or `cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all --bin exp0`)
  - `just exp0-runlog` (or `cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all --bin exp0 -- --summary-json outputs/exp0_summary.json --runlog outputs/exp0_runlog.jsonl`)
- Toy labeled harness:
  - `just toy-harness` (or `cargo run -p pid-sim --bin pid-toy-harness -- --summary-json outputs/toy_vla_summary.json --runlog outputs/toy_vla_runlog.jsonl`)
- H1 common structural/noninterference preflight (fixture plumbing, not Protocol A/B evidence):
  - `just h1-preflight`
- H1 deterministic Protocol A software reference (synthetic fixture/scoring primitive, not H1 evidence):
  - `just h1-protocol-a`
- H2 deterministic fixed-horizon/IPCW/alarm software reference (synthetic protocol arithmetic, not H2 evidence):
  - `just h2-reference`
- Offline VLDA embedding harness:
  - `just offline-harness` (or `cargo run -p pid-sim --bin pid-offline-harness -- --input crates/pid-sim/fixtures/offline_vlda_fixture.json --summary-json outputs/offline_vlda_summary.json --runlog outputs/offline_vlda_runlog.jsonl`)
  - `just offline-harness-require-labels` — exercises `--require-success-labels` on the labeled fixture.
  - `just offline-harness-require-heldout` — exercises `--require-heldout-split`; the checked fixture has `metadata.split=train/test` assignments and passes this strict path.
  - `just offline-harness-require-heldout-class-coverage` — exercises `--require-heldout-class-coverage`; the checked fixture has both classes in train/test subsets and passes.
  - `just offline-harness-require-heldout-episode-disjoint` — exercises `--require-heldout-episode-disjoint`; the checked fixture has disjoint train/test `episode_id` sets and passes.
  - `just offline-harness-strict` — exercises `--require-geometry-pass`; the checked fixture is *expected* to exit nonzero while writing a valid failed run log (fail-closed demonstration).
  - `just offline-harness-highdim` — the high-dimensional synthetic fixture (v=128, l=64, d=32, a=7, 48 samples).
  - `just firebreak` — runs the non-PID prediction/geometry path with `--pid-mode none` and asserts zero MI/PID requests and events.
  - `just offline-harness-discrete` — `--pid-mode discrete --discrete-bins 8` (quantized `I_min` PID with per-pair `discrete_saturation` diagnostics; expect `saturation_warning=true` on the tiny smoke fixtures. The warning does not fail the CLI, but its estimator verdict is blocked and the values are non-evidence).
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
