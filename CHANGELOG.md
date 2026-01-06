# Changelog

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
