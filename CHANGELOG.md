# Changelog

## Unreleased

- Completed the discrete/PLS pipeline slice: discrete 3-source PID (`discrete_pid3`, `I_min` over the 18-antichain lattice), `pid-core` `pipeline.rs` composition helpers (`pls_project_then_pid3`, `pls_project_then_discrete_pid3`, `bootstrap_pid3` per-atom CIs, `permutation_pid3` single-source nulls, `pls_cv_select_components`, `screen_pid2_pairs`), offline-harness `--pid-mode continuous|discrete|discrete-pls` with `--discrete-bins`/`--pls-components`, per-pair `discrete_saturation` diagnostics implementing the grandplan §8.1.6 saturation gate, `I_min`-correct naming for the discrete redundancy (`discrete_imin_redundancy*` — Williams–Beer-style, not discrete `i^sx_∩`), provenance `pid` values `discrete_imin`/`pls_discrete_imin`, a crate `recursion_limit` fix, new `just offline-harness-discrete`/`offline-harness-discrete-pls` recipes, and unit/smoke coverage for all three PID modes.

- Docset v10.2 (2026-06-12): synced `grandplan.md` with the 2026-06-11 implementation slice (PLS, discrete PID, block bootstrap, physics stub, `attribution_logged`, `--strict-gate`, high-dim fixture, 14 Python bindings); added `grandplan.md` §8.1.6 documenting that the implemented discrete redundancy is a Williams–Beer-style `I_min` functional (not discrete `i^sx_∩`) with a saturation/occupancy gate, binning-sensitivity requirements, and a cross-measure extension of Warning 6; added supervised-projection (PLS) guidance with leakage rules (§8.2, §17.5.3); added action-chunking to the `V/L/D/A` contract (§10.10.13.1); resolved the OpenVLA-OFT citation placeholder (arXiv:2502.19645); refreshed DreamVLA/π0/GR00T status notes (hedged); added MI-estimation and PID references (arXiv:2410.10924, 2506.00330, 2409.13506, 2502.04550, 2506.18498, 2602.10098, 2603.19233); corrected the discrete-redundancy naming in `AGENTS.md`/`findings.md`; updated Rapier status wording in `ARCHITECTURE.md`/`pidsplatspecs.md`; bumped docset alignment markers to v10.2.

- Added PLS supervised dimensionality reduction (NIPALS-PLS2) to `pid-core` (`pls.rs`) addressing the key finding that unsupervised projections fail when signal variance ≈ noise variance; added discrete PID via quantization (`discrete_pid.rs`) as a kNN-geometry escape hatch; added block bootstrap uncertainty quantification (`bootstrap.rs`); expanded Python bindings from 8 to 14 functions (pid3, discrete_pid2, pls_transform, standardize, pca_transform, hash_project); added `attribution_logged` event to the run-log schema (H9 attribution probe provenance); added `PhysicsBackend` trait with null adapter and Rapier3D skeleton (behind `rapier` feature) in `pid-sim`; added `--strict-gate` flag to Exp0 binary (exit code 3 on non-GO); added high-dimensional synthetic VLDA fixture (v=128, l=64, d=32, a=7, 48 samples); strengthened meshmaker quarantine in `.gitignore`; added `just offline-harness-highdim` recipe.

- Added the ten-scientist consensus decision record in `grandplan.md` §A.8: canonical run log as source of truth, Rerun as Phases 1–3 diagnostic/time-machine viewer, Agent Bridge as the only control plane, and Tauri/SparkJS as the deferred Phase 4 control/editor/custom-rendering shell.
- Aligned first-party Markdown docs to the same Rerun/Tauri/SparkJS/licensing decision and clarified that optional live transport, external world models, GauSS-MI, and generated assets must emit canonical run-log artifacts.
- Updated the simulation spec license table for the checked Rerun/Tauri/SparkJS/Rerun WebViewer/Three.js package metadata and added release-governance reminders for local crate licenses, third-party notices, and model/data/asset audits.
- Added `crates/pid-runlog` as the first M1 implementation slice: versioned JSONL event schema, reader/writer, SHA-256 helpers, deterministic replay summary, and `pid-runlog-replay` CLI.
- Added bounded follow-up slices for the 10-step plan: embedding/sim/bridge run-log events, replay hash comparison, `crates/pid-bridge`, `crates/pid-sim`, a run-log-to-Rerun adapter/CLI, `just` smoke recipes, and `THIRD_PARTY_NOTICES.md` release-governance groundwork.
- Added the next validation/dispatch slice: run-log structural validation, payload-hash/monotonicity checks, CLI `--validate`, bridge dispatcher abstraction, sim bridge handler/session, `pid-sim-bridge-demo`, `Flow_gt` verification helpers, and CI run-log smoke commands.
- Added the next provenance/API slice: compact run-log summaries, manifest JSON generation, JSON-RPC-shaped bridge request conversion, `pid-sim-verify`, stricter run-log validation before Rerun conversion, and CI smoke coverage for summary/manifest/flow verification.
- Added canonical `evaluation_metric`, `label_observed`, and `embedding_contract` run-log events plus a deterministic toy VLA/task harness with success labels, a replay-linked toy `(V,L,D,A)` contract, PID/CI features, non-PID baseline metrics, summary JSON, canonical run-log export, `just toy-harness`, and CI validation smoke.
- Added Agent Bridge read-only safe-mode metadata/enforcement plus a stdio `--safe-mode` smoke path that logs blocked mutating requests as bridge error responses.
- Added deterministic sim backend/solver provenance config logging for sim demo, bridge demo, and stdio bridge run logs, with validation that logged `config_hash` values match canonical config JSON.
- Tightened replay/provenance gates: run-log validation now checks `run_started`/`config_logged` hash consistency, summaries/manifests expose `config_hash`, replay compare exits nonzero on mismatches, and the sim bridge implements safe-mode `log.replay`.
- Added a loopback TCP JSON-RPC Agent Bridge transport (`pid-sim-bridge-tcp`) for the deterministic sim, with canonical run-log emission and CI validation/replay smoke coverage.
- Added first-class `flow_pred` run-log events plus deterministic constant-velocity baseline predictions for sim run logs, replay summaries, Rerun conversion, and CI smoke assertions.
- Added a generic offline `(V,L,D,A)` embedding harness (`pid-offline-harness`) with checked fixture input, schema validation, PID/baseline metrics, canonical summary/run-log export, `just offline-harness`, and CI validation smoke.
- Added a localhost WebSocket JSON-RPC Agent Bridge transport (`pid-sim-bridge-ws`) with RFC6455 handshake/frame handling, canonical run-log provenance, `just runlog-bridge-ws`, bridge contract transport coverage, and CI smoke validation.
- Implemented Agent Bridge `export.rerun` for validated run logs, including `.rrd` artifact logging, safe-mode blocking, stdio/WebSocket smoke coverage, and `just runlog-bridge-export-rerun`.
- Implemented the remaining advertised deterministic sim bridge lifecycle/intervention methods: `log.start`, `log.stop`, and deterministic `intervention.apply` (`set_velocity`, `translate_object`, `set_pose`), with run-log finalization gates, intervention replay verification, and stdio/TCP/WebSocket smoke coverage.
- Strengthened the offline `(V,L,D,A)` embedding harness with deterministic leave-one-out 1-NN success-label baselines for raw `V`, `L`, `D`, `A`, and concatenated `VLDA`, emitted in both summary JSON and canonical run-log evaluation metrics.
- Added offline `(V,L,D,A)` preprocessing/geometry provenance: PID metrics now run in a deterministic per-variable standardized analysis space, summaries record standardizer hashes and geometry gate warnings, run logs emit first-class geometry metrics, and CI checks the geometry metric count.
- Added fail-closed offline geometry gating via `pid-offline-harness --require-geometry-pass` and `just offline-harness-strict`, which exits nonzero on geometry warnings while still writing a valid failed run log with provenance.
- Extended the offline `(V,L,D,A)` harness from a single `(V,L;A)` PID screen to all two-source `V/L/D→A` screens: `(V,L;A)`, `(V,D;A)`, and `(L,D;A)`, emitted in both summary JSON and canonical run-log PID metrics.
- Added leakage-resistant leave-one-episode-out success baselines to the offline `(V,L,D,A)` harness, emitted when all labeled samples carry `episode_id`, with run-log provenance for split/group key/classifier.
- Added fail-closed success-label enforcement via `pid-offline-harness --require-success-labels`, including valid failed run logs for unlabeled captures and CI coverage of the failure path.
- Added metadata-split held-out success baselines to the offline `(V,L,D,A)` harness, preserving train/held-out sample IDs in summaries/run logs and adding `pid-offline-harness --require-heldout-split` plus CI coverage for success/failure paths.
- Added train-standardized nearest-centroid held-out success baselines for raw `V`, `L`, `D`, `A`, and concatenated `VLDA`, giving the offline harness a deterministic learned non-PID baseline when the train split has both success classes.
- Added held-out balanced accuracy metrics for offline majority, 1-NN, and nearest-centroid baselines when both held-out success classes are present, reducing accuracy-only label-imbalance blind spots.
- Added held-out nearest-centroid AUROC metrics for raw `V`, `L`, `D`, `A`, and concatenated `VLDA`, using the train-standardized signed centroid-distance score so larger scores are more success-like.
- Added held-out per-sample prediction records to offline VLDA summaries, including majority/1-NN/centroid predictions, 1-NN nearest train sample provenance, and centroid discrimination scores for error auditing.
- Added held-out failure-class confusion/rate diagnostics for offline majority, 1-NN, and nearest-centroid baselines, exposing failure TP/FP/TN/FN counts plus precision, recall, specificity, and F1 in summaries and run logs.
- Added held-out class-coverage reporting and `pid-offline-harness --require-heldout-class-coverage`, requiring both success and failure labels in train and held-out subsets for fail-closed offline harness runs.
- Added held-out episode-disjointness reporting and `pid-offline-harness --require-heldout-episode-disjoint`, preventing `episode_id` leakage across train and held-out splits.
- Added train-split-only offline VLDA PID screens with train-only standardization and explicit run-log provenance, so held-out embeddings are excluded from the PID evidence reported alongside held-out baselines.
- Promoted held-out offline VLDA per-sample prediction records into canonical run-log evaluation events, preserving correctness, score, distance, nearest-train, classifier, and sample provenance outside the summary artifact.
- Split replay metric summary semantics so existing `*_metrics` fields remain unique latest-by-name metric counts while new `*_metric_events` fields report total metric event volume, including repeated held-out prediction metrics.
- Added run-log sidecar verification so validation, summary, and manifest JSON sidecars can be checked against the current JSONL run log instead of silently going stale.
- Integrated LRP and related attribution methods into the research docset as H9 companion diagnostics/baselines, with sanity-check requirements and explicit separation from PID/geometry gates.
- Refreshed repo-wide documentation consistency notes: clarified planned-vs-implemented physics adapters, marked mesh generation reports as historical snapshots, added the full tracked-Markdown audit command, and removed stale v10.0/current-pass wording.

## 10.1 (2026-01-08)

- Clarified the v10.1 “Rerun-First” sequencing vs the Phase 4+ target UI stack (scope notes + reading guide) and tightened “spec-only vs implemented” labeling in `grandplan.md`.
- Fixed minor doc drift and numbering in the system-architecture blueprint portion of `grandplan.md` (e.g., §C.1/§C.2 ordering; asset→collision-proxy wording; pseudocode semantics for splat edits).
- Tightened verification language to be offline-friendly: referenced the local `outputs/arxiv_ref_cache.json` cache instead of “arXiv API”, scrubbed unverified latency placeholders in historical notes, and removed unverifiable license specifics (e.g., InternVLA‑A1).
- Added `scripts/audit_grandplan.py`, `scripts/audit_grandplan_claims.py`, and `scripts/update_arxiv_ref_cache.py` to audit arXiv coverage/title drift, scan for high-risk doc drift, and (optionally) refresh the local arXiv metadata cache used for offline verification.
- Updated `.gitignore` so `outputs/arxiv_ref_cache.json` can be tracked while keeping other `outputs/` artifacts ignored.
- Updated docset alignment markers across the documentation set to v10.1 where applicable (`README.md`, `DIAGRAMS.md`, `EXPERIMENTS.md`, `ARCHITECTURE.md`, `uidesigner/UI.md`, and optional module specs).

## 10.0 (2026-01-05)

- Integrated the optional GauSS‑MI spec across the docset: reconstruction uncertainty maps, uncertainty‑aware diagnostics/weighting (optional), and active view selection as confound controls (`grandplan.md` §C.2, `GAUSS_MI_INTEGRATION.md`, `DIAGRAMS.md`).
- Slimmed `README.md` to a brief entrypoint that links to the canonical spec/protocol docs and the engineering plan.
- Bumped docset alignment references from v9.0 → v10.0 across the documentation set.
- Added `uidesigner/UI.md` and `uidesigner/prompt_loop.py` to iteratively design the viewer-first UI (M1→M2→M4→M8) using gpt‑image (via FAL) + Gemini critique loops (Vertex AI), with artifacts saved per UI part.
- Fixed Mermaid syntax robustness in `DIAGRAMS.md` (sequence diagram note formatting; expanded multi-input edges) to improve rendering in common Mermaid toolchains.
- Added LuckyRobots/Lucky World as an emerging simulator comparator and distilled ML-first simulator lessons (RL-style `reset/step`, WebSocket control planes, external-backend adapters that still emit canonical run logs) across `grandplan.md`, `ARCHITECTURE.md`, `EXPERIMENTS.md`, and `DIAGRAMS.md`.
- Added Physical Intelligence PI “π” series (`π0`, `π0.5`, `π0.6*`) as a vendor/black-box VLA comparator with explicit “verify access + embeddings” caveats (`grandplan.md`, `EXPERIMENTS.md`).

## 9.0 (2026-01-05)

- Promoted an explicit v9.0 execution sequence (M0–M7) with acceptance criteria in `grandplan.md` (§A.7) so engineering can begin without re-interpreting the spec.
- Restructured `README.md` to lead with hypotheses + experiments, then map directly to the engineering build order (gate-driven, contract-first).
- Bumped docset alignment to v9.0 across `ARCHITECTURE.md`, `DIAGRAMS.md`, `EXPERIMENTS.md`, and `pidsplatspecs.md`, and clarified offline-first run logs + replay vs optional live transports (Zenoh).
- Added a multi-engine physics reality check: per-object mixed backends are a co-simulation problem; prefer one backend per run plus optional cross-backend replay as a robustness/confound control (`grandplan.md` §E.1).

## 8.0 (2026-01-05)

- Corrected SparkJS assumptions: documented SparkJS “Spark” as a Three.js-integrated WebGL2 3DGS renderer (with links), and made renderer requirements backend-agnostic (WebGL2/WebGPU) where appropriate.
- Clarified contacts/collisions in 3DGS-based simulators: updated SplatSim (PyBullet physics backbone) and DISCOVERSE (MuJoCo physics backbone) notes, and made PID‑Splat’s default collision path explicitly mesh/URDF/MJCF-driven (with splat-field collision heuristics treated as optional research).
- Updated hypothesis set: added **H8** (geometry gate → estimator regime choice), narrowed **H2/H3** into falsifiable ablation/intervention claims, and softened optional world-model extension hypotheses (H_WM1–H_WM5) to avoid pre-committed outcomes.
- Expanded model/flow survey: added SmolVLA to the VLA reference list and added RAFT (arXiv:2003.12039) as a non-generative flow baseline for `Flow_obs`.

## 7.0 (2026-01-05)

- Scientific audit pass across the docset: removed or downgraded unsourced performance/hardware/roadmap claims; switched to measurement-first language.
- Reworked `grandplan.md` VLA integration into a contract-first framing (`V/L/D/A` must be defined and logged per checkpoint; no assumed layer names/shapes).
- Added a risk-reducing execution sequence: Exp0 → harness bring-up with simulator-derived `Flow_gt` → small baseline (e.g., SmolVLA) → primary VLA (e.g., OpenVLA) → optional diffusion/predictor-driven Flow.
- Clarified H1 as “PID features ↔ failure labels” (synergy sign is a candidate feature, not a definition of hallucination).
- Added/updated Agent Bridge requirements (GUI and automation share one control plane; JSON-RPC/MCP; all interventions logged and replayable).
- Added OpenUSD/USDZ interop notes (LeIsaac Marble tutorial; `.ply → .usdz` via NVIDIA 3DGrut) as an optional workflow.
- Added InternVLA‑A1 as an optional diffusion/flow-matching VLA candidate for stage-wise ablations (with explicit license/verification caveats).
