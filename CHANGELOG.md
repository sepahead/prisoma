# Changelog

## Unreleased

- Science-honesty: `--require-axis-provenance-honest` ENFORCES axis provenance, not
  just reports it (2026-06-21). The offline VLDA harness already surfaced per-axis
  provenance status; this adds an opt-in gate (mirroring `--require-geometry-pass`)
  that FAILS the run if any V/L/D/A axis is `degraded` (`text_hash_proxy` /
  `absent_zeroed` / `recency_fallback` …) — and, crucially, if NO provenance markers
  were stamped at all (positive attestation: honesty cannot be vacuously passed on a
  dataset that carries no provenance). Threaded through `OfflineVldaRunlogOptions`
  (recorded in the run-log as `strict_axis_provenance_honest`), the bin arg
  parser/usage, and the reproducible `just safe-adapter` recipe (the honest synthetic
  SAFE dataset passes; a provenance-stripped copy fails). New unit test
  `axis_provenance_gate_fails_on_degraded_and_on_absent_markers`.

- Science-honesty: the offline VLDA harness now surfaces `safe_adapter` axis
  provenance (2026-06-21). `axis_provenance` previously recognized only the live
  `ncp-observer` markers (`l_source`/`d_source`); running the offline `safe_adapter`
  pipeline end-to-end (synth→convert→verify→`pid-offline-harness`) exposed that its
  samples carry `{v,l,d,a}_provenance` instead (`token_slice:*` / `hidden_state_pool`
  / `action_vector` = honest; `text_hash_proxy` = hash surrogate for a missing real
  feature = degraded), so the report came back with `axis_provenance: []` and a PID
  atom computed from a hash-proxy axis would have been reported as trustworthy. The
  harness now recognizes both capture conventions (extended `MARKERS` + a shared
  `DEGRADED_PROV` value set); on the honest synthetic SAFE dataset all four axes report
  `status=ok` with their `token_slice:*`/`action_vector` sources. New unit test
  `axis_provenance_recognizes_safe_adapter_markers`.

- Re-pinned `crates/ncp-observer` to NCP `v0.5.0` (wire `0.4` → `0.5`, the stable-wire
  cut: the command/sim `mode` strings are now proto enums (`Mode`/`SimMode`) and
  `CONTRACT_HASH` was recomputed `2cf0763ad61e4f1c` → `24e8e6e31e1dec8a`). The observer
  is a read-only data-plane tap (no session handshake) and the JSON wire is unchanged
  for known values, so the bump is the `ncp-core`/`ncp-zenoh` git tag + its standalone
  `Cargo.lock` and the doc pins; `ncp-observer` builds against the v0.5.0 tag and its
  tests pass unchanged.

- Science-honesty: `ncp-observer` per-sample provenance markers (2026-06-21). Every
  emitted `(V,L,D,A)` sample now carries `metadata.l_source` and `metadata.d_source`
  so a degenerate axis is never silently presented as real data. `l_source` is
  `"channel"` when the language channel was present or `"absent_zeroed"` when `L` is
  the fabricated all-zero vector (NCP_DEV_PROMPT Gap 2); `d_source` is `"seq"`
  (exact alignment), `"recency_fallback"` (publisher sent `obs.seq == 0`, so `D` is
  the most-recent readout rather than the driving one — Gap 1), or `"absent"`.
  Downstream can now filter on these; combined with the harness degenerate-axis gate
  below, a zeroed `L` cannot pass unflagged. New test
  `provenance_marks_recency_fallback_and_present_language`; `d_aligns_on_seq_not_recency`
  extended to assert the markers.

- Science-honesty: degenerate-axis geometry gate (2026-06-21). The offline VLDA harness
  now flags any variable whose every dimension is zero-variance (all-constant) as a
  geometry-gate warning, reusing the already-computed-but-previously-unused
  `zero_variance_dims`. An all-constant axis has zero variance, hence zero mutual
  information with anything by construction, so every PID atom involving it is degenerate
  rather than merely small — this is exactly the fabricated all-zero `L` case
  (`NCP_DEV_PROMPT.md` Gap 2, where an absent language channel `unwrap_or_default()`s to
  zeros). The warning sets the gates to `warn` (and so fails `--require-geometry-pass`),
  satisfying Gap 2's acceptance criterion that "the harness can filter on it." New unit
  test `geometry_gates_flag_all_constant_variable_as_degenerate`.

- First-principles review + correctness pass (2026-06-16). A multi-agent, adversarially-verified review of every crate + the Python experiments produced 18 confirmed findings, all fixed and verified (clippy/fmt clean; full test suite incl. `rapier`/`parallel` features; Python suites; all CI metric greps). Headline: **CI was red** — the new `ncp-observer` crate, a default workspace member, path-depends on the sibling `Paper2Brain/ncp` tree (absent on a fresh checkout), which fails manifest resolution for *every* cargo command; it is now **excluded from the default workspace** (build via `--manifest-path`), regenerating `Cargo.lock` without the Zenoh tree, and its two clippy lints + formatting are fixed. Correctness fixes: `discrete_pid` histograms now key on the bin vector instead of a base-`num_bins` integer that silently overflowed `usize` in high dimension (collisions corrupting every discrete entropy/MI/PID value; debug panic); the `.npy` reader uses checked arithmetic so a crafted shape returns `None` instead of aborting in `Vec::with_capacity`; the PLS LOO-CV no longer leaks the held-out target (`PlsProjector::predict` via the proper `B = W(PᵀW)⁻¹Cᵀ` regression, correct for k≥2); `block_bootstrap`/`_paired` are now a moving-block bootstrap (Künsch 1989; no tail-drop/grid-bias) with the correct citation; the redundant `cmi_violations` Exp0 gate (always equal to the monotonicity check) is dropped (7 gate metrics now, `pid_metrics` 8→7); `ksg` xblocks rejects non-Chebyshev metrics; the synthetic-data LCG shift is fixed (`>>33`→`>>32`); `pid-runlog-replay` prints the `attributions=` count; `PhysicsStepReport.timestamp_ns` accumulates per-step dt; the attribution faithfulness check is made statistically honest (seeded tie-breaking + a 3-standard-error threshold so an uninformative attribution reliably fails); plus geometry doc-drift / `Pcg32::next_u32` renaming and dead-code cleanup.
- Data-source boundary documented + NCP dev handoff (2026-06-16). Clarified across the docset (`README.md`, `EXPERIMENTS.md` §0.2.1, `AGENTS.md`, `grandplan.md` §11.4, `crates/ncp-observer/README.md`) that the analysis is **source-agnostic** over one `(V,L,D,A)`+labels contract: the **critical-path producer is `experiments/safe_adapter`** (M5), the sim harnesses are standalone cross-checks, and **`ncp-observer` (Engram/NEST) is an optional, non-critical-path bridge** — grandplan does not depend on Engram, and the pure-PID stack builds/gates green with no NCP/Engram/Zenoh dependency. NCP is exploratory-only (below the M5 contract) until three gaps close — precise D `seq`-alignment, honest (non-zeroed) `L`, and `metadata.split`/`episode_id`/`success` structure for the strict gates and the §14.1.1 H1 audit. Added `NCP_DEV_PROMPT.md`, a self-contained developer handoff for that work. Also reaffirmed that **D is the hidden-state / dynamics axis, not depth** (consistent across SAFE and NCP).
- Docset v10.3 (2026-06-13) — capture + analysis implementation slice. (1) **Exp0 uncertainty gate**: opt-in `--bootstrap`/`--permutation` on the Exp0 binary run subsample-bootstrap CIs (Politis–Romano, KSG-safe) and single-source permutation nulls, with a preregistered ground-truth marginal-significance check folded into the GO/PIVOT/NO-GO verdict (8/8 on the synthetic scenarios); `pid-core` gains `bootstrap_rows_stats` + `permutation_rows_pvalue`; default runs stay byte-identical (closes `REVIEW_AND_TODO.md` P0 item 1 end to end). (2) **Real Rapier3D backend**: the fake `RapierBackend` stub is replaced with a real `rapier3d-f64` pipeline (gravity/contacts/friction, deterministic), plus a scripted push-to-goal manipulation (`crates/pid-sim/src/manipulation.rs`) emitting canonical run-log events with real `Flow_gt` and physics-derived success labels (`pid-rapier-harness`; new `rapier` CI job). (3) **SAFE-class failure detector**: `pid-core` `logistic.rs` (Newton-IRLS L2 logistic regression) wired as the leakage-safe `heldout_logreg_vlda` offline-harness baseline (H1); fixture evaluation-metric counts move 139→142 / 220→223. (4) **M5 SAFE capture adapter** (`experiments/safe_adapter/`): converts released SAFE rollouts to the `(V,L,D,A)`+labels contract with honest provenance and the §7.6.3 layerwise physics-probe for `D_hidden[k]` selection; verified end to end into the real harness with all leakage gates passing. (5) **H9 attribution probe** (`experiments/attribution/`): epsilon-/AttnLRP + gradient×input, deletion-AOPC faithfulness check vs a random control, and schema-conformant `attribution_logged` run-log emission validated by `pid-runlog-replay`. (6) **Release governance**: `meshmaker/` quarantined out of tracking (non-destructive `git rm --cached` + tombstone README; `.gitignore` hardened); `scripts/generate_third_party_notices.py` generates a direct-dependency SBOM with a CI drift check. New `experiments` CI job cross-validates both Python pipelines against the real Rust binaries.

- External-source review addendum (2026-06-13): reviewed dimos / DimensionalOS (`dimensionalOS/dimos`, Apache-2.0 agentic multi-robot control OS) source-by-source and recorded it in `grandplan.md` §12.5 + §13.10 + the §11.4 interoperability note — integrated narrowly as independent external validation of the Agent Bridge one-control-plane / record→replay / every-action-a-run-log-event architecture (§A.7/§A.8/§11.4) and a concrete external-backend adapter target (LCM as another M6 typed-stream transport alongside Zenoh/ROS 2); ruled out as an M5 capture shortcut (no internal-activation or success/failure-label extraction per README → SAFE still dominates for `(V,L,D,A)`+labels) and as a PID/IT method contribution.

- Final research/improvement pass: restored CI parity (rustfmt on the offline harness; clippy/ruff/pytest verified locally), fixed the duplicate `grandplan.md` §2.5.4 header (→ §2.5.5), recorded a dated novelty check in §12.5 (no published PID-on-VLA application found; standing related-work check), identified the released SAFE rollout datasets/pipeline (`vla-safe/SAFE`) as the concrete M5 capture shortcut (README step 4, `EXPERIMENTS.md` §0.2, `REVIEW_AND_TODO.md` critical path), and added a hedged §13.1 pointer to the reported 2026 Wibral–Makkeh multivariate PID paper.

- Self-sufficiency pass on the entry-point docs: `README.md` gains a "Current Status & What To Do, In Order" section (status paragraph + 7 gated steps with commands and expected outcomes), status + real-robotics-problem columns on the hypotheses table, per-experiment runnable-today/blocked annotations, and `findings.md`/`REVIEW_AND_TODO.md` in the doc map; `EXPERIMENTS.md` gains a §0.2 executable-vs-blocked runbook table and an updated geometry-decision-tree note pointing at the implemented discrete PID modes; `findings.md` notes the harness `--pid-mode` wiring of both escape hatches with saturation diagnostics.

- Third v10.2 research batch: integrated World Pilot (arXiv:2606.12403; separable WAM steering pathways as model-native Exp4/H3 interventions), the video-world-model physics-interpretability study (arXiv:2602.07050; "Physics Emergence Zone" hook-point prior in `grandplan.md` §7.6.3), and the GEN-1 vendor post (black-box comparator; embodiment-gap instance); added a "Real robotics problem addressed" column and failure-regime framing to the hypothesis registry (§14.1.0); aligned `EXPERIMENTS.md` H2/H5 rows with the §14.1.1 necessity audit (graded-language probe option; mandatory CI-only ablation); appended a dated 2026-06-12 implementation/status section to `REVIEW_AND_TODO.md` marking block bootstrap (P0.1 item) and the discrete/quantized PID fallback (P2.11) as shipped and naming the first real VLA capture as the critical path; refreshed README repo-status wording and the arXiv cache.

- Research integration pass (docset v10.2, 2026-06-12): added `grandplan.md` §12.5 external-source review (integrated: Qwen-VLA, V-JEPA 2/-AC + a latent-predictive world-model taxonomy class, NVIDIA Cosmos 3 "World Action Model" notes, RoboLab-120 benchmark context, π0.7, AttnLRP, GSWorld, Zenoh-in-production via ROBOTIS Cyclo, world-model surveys; ruled out with citations: ProjectEdenGG, Trajectory, a DAIR.AI aggregator week, unverifiable X posts), §14.1.1 per-hypothesis PID-necessity audit with preregistered kill criteria, §14.7.1 concrete AttnLRP/LRP protocol for transformer VLAs, the embodiment-in-`L` confound (§14.5.7.3), LIBERO-saturation caution, and new D-candidate rows (Qwen-VLA, V-JEPA 2-AC, Cosmos3-Nano-Policy); refreshed the arXiv reference cache accordingly.

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
