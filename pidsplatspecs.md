# PID-Splat Unified Simulation Environment Specification

> **Documentation Cross-Reference**:
> - `grandplan.md` — Master plan and theoretical foundations
> - `ARCHITECTURE.md` — Component breakdown and advantages over VLM-based robotics
> - `EXPERIMENTS.md` — Experimental protocols for Rerun-first diagnostics, modular physics, and hypothesis testing
> - `DIAGRAMS.md` — Visual architecture diagrams
> - `README.md` — Quick start guide
> - `GAUSS_MI_INTEGRATION.md` — Optional 3DGS uncertainty + view selection (spec)
> - `WORLD_WARP_INTEGRATION.md` — Optional external world‑model baseline (spec)
## Technical Blueprint for the "Splat-First" Research Platform

**Version:** 12.5 (2026-07-12 docset v12.5 alignment)
**Date:** 2026-07-12
**Context:** Canonical implementation spec for the infrastructure/simulation and visualization layer defined in `grandplan.md` §8 (Infrastructure as a scientific contribution — esp. §8.6 interoperability and §8.13 visualization/rendering).

---

### 1. Executive Summary

This document specifies the target engineering implementation of the **PID-Splat** environment. It bridges photorealistic 3D Gaussian Splatting (3DGS) with modular rigid-body physics (**Rapier, MuJoCo, or Isaac Gym**) and Dream2Flow-style video→flow bridges to enable reproducible PID diagnostics for VLA models.

**Visualization Strategy (v10.1): "Rerun-First"**
To accelerate research iteration (Phases 1-3), the system prioritizes **Rerun** (https://rerun.io/) as the primary visualization and "time machine" viewer. The canonical run log remains the authoritative record. The custom Tauri/SparkJS frontend is deferred to Phase 4 for advanced interactive needs.

**Docset-wide final solution:** `grandplan.md` §16 is the decision log (see also §8.2 event model, §8.11 control plane, §8.13 visualization). The run log is the canonical state, the Agent Bridge is the only control plane, Rerun is a read-only research viewer, and Tauri/SparkJS is a later control/editor/custom-rendering shell. Every VLA action, scene edit, intervention, pause/resume/step transition, and correction force must enter through the Agent Bridge and be recorded before execution. Observers and PID workers are read-only analyzers; Zenoh is optional data transport; none of them may actuate physics.

**Current estimator status (four-gate model, `grandplan.md` §7.1):** the high-dimensional **MI/coherence path is NO-GO** (population/measure/estimator gates fail on nuisance-dimension controls); continuous shared-exclusions atoms on **real embeddings are BLOCKED / NOT APPLICATION-VALIDATED** (application gate un-cleared). The `exp0` binary implements part of the §7 estimator/measure validation (S1 gate); its default aggregate is **not** an atom-validity verdict, and `--strict-gate` only enforces the curated d=1 Gaussian **MI** band, not the atoms or the high-dimensional sweep. `--pid-mode discrete` is Williams–Beer `I_min`, **not** discrete `i^sx_∩` (`grandplan.md` §7.6). See `grandplan.md` §7.2 and `findings.md`.

**Core Philosophy:** "Splat-First." We render reality (captured via 3DGS) and bind physics to it, while overlaying predicted “dreams” (video‑predicted 3D flow) to visualize what a policy *expects* to happen.

**Contact/collision reality check:** existing “3DGS-based” simulators still use conventional physics engines for contacts. Treat splats as the appearance layer; use explicit collision geometry (URDF/MJCF primitives/meshes) in the physics backend.

**Multi-engine note (v10.1):** treat the physics backend as a **per-run** choice for contact-rich scenes. A more practical differentiator is **cross-backend replay** (re-run the same action log in Rapier vs MuJoCo and report divergence) as a robustness/confound control (see `grandplan.md` §6.10 robustness/falsification and §8.5 replay levels).

---

### 2. Technology Stack & Versions

| Component | Technology | Version / Spec | License |
| :--- | :--- | :--- | :--- |
| **M0 governance** | Strict JSON/JSONL ledgers + offline validator | Implemented honesty scaffold only: unfrozen protocol branches, no registered confirmatory holdout, pending dataset/transport/contamination work, and legacy-only literature inventory. Not freeze-ready or scientific evidence | MIT OR Apache-2.0 (project) |
| **Run log** | `pid-rs/crates/pid-runlog` JSONL events + replay summary | Schema 2; partial M2/EC1 groundwork implemented. Current types cover embedding/sim/bridge/PID/attribution events, validation, replay hash comparison, summary/manifest JSON, and sidecar verification; the full typed causal/temporal event model and graded external conformance benchmark remain open | MIT OR Apache-2.0 (project) |
| **Agent Bridge core** | `crates/pid-bridge` | **Partial M2 groundwork:** local request/response schema, dispatcher, single-request JSON-RPC 2.0 subset, run-log integration, contract export, safe-mode gates, and stdio/TCP/WebSocket deterministic-sim transports. Network binaries refuse non-loopback binds and default safe; per-message/per-operation caps, an enumerated no-Origin WebSocket upgrade, non-adversarial canonical file confinement, and no-replace outputs are implemented. Forwarding/proxying, adversarial filesystem mutation, authentication/authorization/TLS/redaction, remote assessment, full target UI/VLA/backend coverage, and versioned subscriptions remain | MIT OR Apache-2.0 (project) |
| **Deterministic sim smoke** | `crates/pid-sim` | Object-only fixed-step sim + simulator-derived `Flow_gt`; bridge demo, stdio/TCP/WebSocket single-request JSON-RPC 2.0 subset, `log.replay`, `log.start`/`log.stop`, deterministic `intervention.apply`, `export.rerun`, flow verification CLI, deterministic action/intervention replay checks, toy labeled harness, and offline `(V,L,D,A)` artifact-to-runlog harness with all-pairs `V/L/D→A` PID screens plus train-split-only PID screens when a metadata split is present, standardization provenance, geometry diagnostics plus a legacy fail-closed aggregate (software smoke only; corrected scientific eligibility does not gate on sampled mean `δ_rel`), strict label/held-out-split/class-coverage/episode-disjoint modes, sample-level, episode-grouped, metadata-split held-out majority/1-NN/nearest-centroid/logistic baselines, prediction/confusion diagnostics, and replay-visible metric-event counts; a `PhysicsBackend` trait with a null adapter and a **real `rapier3d-f64` backend** (gravity/contacts/friction, deterministic) + a scripted push-to-goal manipulation exists behind the optional `rapier` feature (box-collider geometry; mesh-collider ingestion and MuJoCo/Isaac adapters remain planned) | MIT OR Apache-2.0 (project) |
| **Attribution probes (triangulation baseline; `grandplan.md` §6.5 Level 3 / §3.7)** | `experiments/attribution` + `attribution_logged` + `pid-rerun` | Implemented reference slice: epsilon-/AttnLRP and gradient×input on a small reference model, deletion-AOPC vs random faithfulness check, content-addressed no-replace artifact publication, first-class run-log events, and Rerun faithfulness/provenance. The standalone converter's default-off `--load-attribution-artifacts` capability can surface a confined, regular, exact-SHA/shape-bound NumPy `<f8` relevance series of at most 1024 finite values; bridge export keeps external loading off. Confinement is local best-effort, publication is not a cross-file transaction, and neither protects against every concurrent filesystem race. Production VLA/LXT hooks and richer panels remain planned | MIT OR Apache-2.0 (project); model-dependent inputs must be verified |
| **Visualization** | **Rerun** (Phases 1-3) / Tauri (Phase 4) | **Partial M2/EC1 viewer groundwork:** Rerun SDK 0.34.1 and validating run-log conversion with summary/provenance/validation and attribution tracks are implemented; the complete viewer blueprint remains specified. Tauri version to pin when implemented | Rerun: MIT OR Apache-2.0; Tauri API package metadata: Apache-2.0 OR MIT |
| **Renderer** | Rerun native/WebViewer / SparkJS (Phase 4) | Pin exact package versions / git SHAs at implementation time | Rerun WebViewer: MIT; SparkJS package metadata: MIT; Three.js: MIT |
| **Splat Library** | gsplat | v1.0+ (via Nerfstudio for training) | Apache 2.0 |
| **Physics Engine** | Rapier3d / MuJoCo | Real pinned `rapier3d-f64` backend implemented behind `rapier`; MuJoCo/Isaac adapters remain planned and must be pinned when added | Apache-2.0 |
| **Middleware** | Zenoh | Pub/sub transport; shared memory/zero-copy is config-dependent | EPL-2.0 OR Apache-2.0 |
| **Sensor Sim** | Gazebo | Harmonic (gz-sim 8.x) | Apache 2.0 |
| **Video predictor** | Video model (external service) | Model-dependent (pin revision) | verify |
| **Flow Tracker** | Point tracker (e.g., CoTracker) | Model-dependent (pin revision) | verify |
| **Agent Bridge (control plane)** | Single-request JSON‑RPC 2.0 subset over WebSocket (+ optional MCP wrapper) | Versioned local API for live interventions + automation; M2 acceptance remains partial | MIT OR Apache-2.0 (project) |

---

### 3. Gaussian Splatting Specifics

#### 3.1 Pipeline & Formats
*   **`.SPZ` (Compressed):** Used for runtime/distribution.
*   **`.PLY` (Raw):** Used during editing/debugging.

**Training Pipeline:**
1.  **Capture:** Polycam/Luma (iOS) or DSLR video.
2.  **Process:** `ns-train splatfacto --data <data_dir>` (Nerfstudio/gsplat backend).
3.  **Export PLY:** `ns-export gaussian-splat --load-config <config> --output-dir <dir>`.
4.  **Optional SPZ conversion:** use a separately selected and pinned PLY→SPZ converter. Record its executable/revision, license, exact command, and input/output hashes; SPZ is not an `ns-export gaussian-splat` output-format flag.

**Optional OpenUSD / USDZ interop (Isaac Sim / LeIsaac workflows):**
- Convert splat `.ply` → `.usdz` (packaged OpenUSD) using NVIDIA 3DGrut, then compose splats + collision mesh in Isaac Sim to export a single `.usd` background stage (see `grandplan.md` §8.6 interoperability and `DIAGRAMS.md` §9).

#### 3.2 LOD Strategy
*   **Asset-specific count:** measure the exported Gaussian count and renderer memory/time for each scene; do not impose an unsupported universal range.
*   **Distance-based culling:** Alpha cull threshold increases with distance.
*   **Frustum culling:** Octree-based spatial indexing.

---

### 4. Dream2Flow Integration (Flow-as-Bridge)

This section specifies the target unified infrastructure architecture from `grandplan.md` §8 (esp. §8.6 interoperability and §8.7 adapter contract).

**v10.1 sequencing note:** bring up Flow-as-Bridge using **simulator-derived `Flow_gt`** (from logged object poses) before introducing any stochastic video predictor.

#### 4.1 3D Flow Data Structure
We represent the "Dream" not just as a hidden state, but as explicit 3D trajectories extracted from predicted videos (Dream2Flow-style bridge).

```rust
/// Represents a single object's predicted path (the "Flow")
#[derive(Serialize, Deserialize, Clone)]
struct DreamFlowTrajectory {
    object_id: u32,
    /// Sequence of (x, y, z) points over time T
    points: Vec<[f32; 3]>,
    /// Confidence/Opacity per point (from the tracker)
    confidence: Vec<f32>,
    /// PID Synergy at each point Syn(V, D; Flow_t)
    /// Windowed ensemble estimate Syn(V, D; Flow_t) aligned to each timestep
    /// (NOT a single-sample pointwise PID; regime per grandplan §2.5/§4)
    synergy: Vec<f32>,
}
```

#### 4.2 Video Predictor Integration Pipeline
Video prediction happens externally (e.g., Python/CUDA or a hosted API). Results are registered in the canonical run log; Zenoh may mirror data to live consumers but is not a control plane.

1.  **Trigger:** an Agent Bridge request records and triggers the `(Image, Instruction)` predictor job; a VLA or orchestration client may originate that request but may not bypass the bridge.
2.  **Generate:** Video model generates a short clip (length is configurable; log FPS/frames/seed).
3.  **Extract:** Tracking + depth estimation extract `DreamFlowTrajectory` (model-specific; log versions).
4.  **Record:** Rust backend writes `dream/flow/{id}`-equivalent events into the canonical run log; optional Zenoh publication is only a live-transport mirror.

#### 4.3 "Ghost Splat" Visualization (Rerun Implementation)
In Rerun (Phases 1-3), we avoid complex custom shaders. Instead, we log **two distinct entities**:

1.  **`world/reality`**: The captured scene splats (standard rendering).
2.  **`world/ghost`**: A separate point cloud or splat-set representing the predicted flow.
    *   **Color Mapping:** Manually colored Red/Blue/Green in Rust before logging to represent PID values (Syn, Unq, Red).
    *   **Transparency:** Alpha value set by MI magnitude or confidence.

(In Phase 4/SparkJS, this will be upgraded to a GPU shader-based overlay).

---

### 5. Zenoh Middleware Protocol

**Note (v10.1 execution plan):** Zenoh is an optional live/distributed transport (M6). Early milestones should be able to run entirely offline by writing the same events to the run log (M1) and replaying them (M1/M4).

**Control boundary:** Zenoh key expressions carry observations, embeddings, metrics, and optional live mirrors. A Zenoh subscriber must never apply an action, intervention, pause, resume, step, or correction directly. Such a message is first submitted to the Agent Bridge, recorded as a canonical command event, and only then dispatched to a backend.

#### 5.1 Key Expressions

| Key Expression | Data Type | Frequency | Source → Dest |
| :--- | :--- | :--- | :--- |
| `sim/pose/{id}` | `[f32; 7]` | 60Hz | Physics → run log → Rerun (P1-3) / SparkJS (P4) |
| `scene/uncertainty` | `SceneUncertaintyMap` | Event | prospective quality study (optional) → run log → Rerun/nuisance analysis; never direct weighted PID |
| `dream/flow/{id}`| `DreamFlowTrajectory` | Event | Video predictor/flow extractor → run log → Rerun (P1-3) / SparkJS (P4) |
| `pid/metric/{id}` | `PidStruct` | 10Hz | pid-core → run log → Rerun (P1-3) / SparkJS (P4) |
| `vla/action` | model-specific action vector/chunk | Event | VLA adapter → Agent Bridge → canonical action event → Physics; optional Zenoh mirror is data-only |

#### 5.2 PID Message Schema
```rust
#[derive(Serialize, Deserialize, Clone)]
struct PidMetricMsg {
    timestamp_ns: u64,
    object_id: u32,
    synergy: f32,
    redundancy: f32,
    unique_v: f32,
    unique_l: f32,
}
```

#### 5.3 Agent Bridge (UI + Automation API)

The simulator must be **agent-native**: the GUI is not the only interface. Every operation that matters scientifically (scene edits, interventions, run control, replay, exports) must be callable through a stable API that works well with **Claude Code / Codex / opencode‑style tooling**.

**Core rule:** the UI, VLA-policy adapter, scripts, and external tools use the same control plane (no hidden manual paths). Every mutating request—including VLA actions, reset/step, pause/resume, scene edits, interventions, and correction forces—is appended to the canonical run log before dispatch. PID workers, observers, Zenoh, Rerun, and offline harnesses never issue control commands.

**Recommended transport:**
- **JSON‑RPC 2.0 over WebSocket** on `127.0.0.1` (the deterministic sim smoke implements the single-request subset described below; full UI integration still builds on the same surface).
- Optional: **MCP server wrapper** exposing the same methods as tool calls (thin adapter; no separate logic).

The in-repo deterministic bridge currently exposes status/reset/step, scene edits, deterministic interventions (`set_velocity`, `translate_object`, `set_pose`), `log.replay`, `log.start`/`log.stop`, and `export.rerun`. TCP/WebSocket binaries refuse non-loopback bind addresses and start in safe mode; `--allow-mutations` is an explicit local opt-in, but forwarding, proxying, or tunnelling a loopback listener is not prevented. TCP/stdio JSONL lines are capped at 1 MiB, WebSocket upgrades/incoming client frames at 16 KiB/1 MiB, and network reads/writes at 30 seconds per operation. No total request/session deadline, request-count cap, or aggregate-traffic limit exists, so progress-making trickle traffic can persist.

WebSocket accepts `GET /bridge HTTP/1.1` with exactly one each of a nonempty `Host`, `Upgrade: websocket`, tokenized `Connection` containing `upgrade`, version `13`, and a base64 key decoding to 16 bytes, and rejects `Origin`; this enumerated gate does not promise detection of every malformed request. Client application messages are unfragmented, masked UTF-8 text frames; ping, pong, and close control frames are supported, while binary frames, fragmentation, and extensions/RSV use are rejected. The wire protocol is a single-request JSON-RPC 2.0 subset: batches are unsupported, missing-id notifications are silent and distinct from explicit `null`, parameters are omitted or named objects (not positional arrays), undeclared top-level method keys are rejected, and `sim.step` requires numeric `dt`. Profile-invalid parameters use `-32602`; handler/domain failures after validation use `-32000`.

File methods use non-adversarial canonical confinement, rejecting traversal, observed symlinks, non-regular/out-of-root inputs, missing output parents, and existing outputs. Run logs and Rerun outputs are no-replace. Export parses/manifests the same exact byte snapshot read from the source, encodes/hashes finalized RRD bytes, and stages, syncs, and persists them no-clobber. Executable transport run logs call `File::sync_all` for the initial prefix, each session flush before a wire response, and the terminal seal; generic `SimBridgeSession<W>` durability is sink-defined. This is not a security-grade sandbox against hardlinks, aliases, or concurrent filesystem mutation, and there is no parent-directory fsync, power-loss claim, or cross-file run-log/export transaction. Ordinary accepted-client errors seal `Failed` only while provenance storage is writable, and a crash/storage failure may leave incomplete or unreadable provenance, an apparently complete terminal record with indeterminate status/durability, or an orphan RRD. These are local E0 controls, with no authentication, authorization, TLS, redaction, or remote-security assessment.

---

### 6. Modular Physics Binding (PEGS)

#### 6.1 Splat-to-Physics Mapping
The target environment supports multiple physics backends (**Rapier, MuJoCo, Isaac Gym**) via a unified trait interface. The checked repo currently has the deterministic object-sim smoke plus a `PhysicsBackend` trait with a null adapter and a real `rapier3d-f64` backend (gravity/contacts/friction) + a scripted push-to-goal manipulation behind the optional `rapier` feature; MuJoCo/Isaac backend adapters remain planned.
*   **Manual Proxy:** collider definitions from code/config are registered as canonical configuration events; Phase 4 Tauri edits submit the same Agent Bridge scene-edit request. Rerun only visualizes the resulting proxy/state.
*   **Visual Forces (target counterfactual):** a preregistered client may request a correction force, but PID never triggers it automatically. The request must pass through the Agent Bridge, be written to the canonical run log, and then be dispatched to physics.

---

### 7. VLA Integration Interface

#### 7.1 Integration Points
1.  **Observation:** VLA subscribes to `sim/camera/rgb`.
2.  **Action:** the VLA-policy adapter submits each action/chunk to the Agent Bridge. The bridge records the canonical action event before dispatching it to physics; optional `vla/action` publication is a data mirror only.
3.  **Embedding Extraction:**
    *   **Publisher:** `vla/embeddings` sends `(timestamp, layer_id, vector_f32)`.
    *   **Used By:** `pid-core` to compute `Syn(V, D; Flow)`.

---

### 8. PID Computation Pipeline

*   **Target:** We compute PID on **Flow** trajectories (Euclidean) to bypass the manifold geometry issues of raw embeddings (Flow-as-bridge).
*   **Metric:** `Syn(V_embedding, D_embedding; Flow_trajectory)`.
*   **Windowing:** Rolling window of T=10 to T=50 timesteps (illustrative; final windows per the future frozen statistical analysis plan specified in grandplan §6, with the unit-of-inference and dependence rules in §2.5/§6.7).

---

### 9. Performance Budget & Targets

**Measurement-first:** treat latency/throughput as empirical properties of your hardware + scene + models. For interactive work, start with offline playback and progressively move components in-loop only after you have measured budgets and uncertainty (see `grandplan.md` §12 milestones and `EXPERIMENTS.md` §12).

---

### 10. Implementation Plan / Current Status

1.  **Implemented groundwork:**
    *   Rust workspace with `pid-bridge`, `pid-sim`, and `pid-rerun`; the `pid-core`, `pid-runlog`, and `pid-python` crates live in the [`pid-rs`](https://github.com/sepahead/pid-rs) submodule (single source of truth).
    *   Canonical run-log schema, replay validation, summaries/manifests, sidecar write-and-verify, and a validating run-log-to-Rerun adapter. This is partial M2/EC1 groundwork; the complete typed causal/temporal model, conformance benchmark, and diagnostic blueprint/viewer are not built.
    *   Deterministic object-sim smoke with simulator-derived `Flow_gt`, constant-velocity `flow_pred`, Agent Bridge stdio/TCP/WebSocket smokes, toy labeled harness, offline `(V,L,D,A)` artifact harness, the content-addressed schema-v2 `pid-h1-preflight` fixture path, the exact-bound deterministic `pid-h1-protocol-a` synthetic finite-benchmark scoring reference, and the PID-free `pid-h2-reference` fixed-horizon cumulative-incidence/IPCW/alarm arithmetic reference. The H1/H2 paths are software primitives, not real Protocol A/B, prospective H2 capture, calibration validation, or scientific evidence. The bridge is partial M2; target-wide UI/VLA/backend control coverage and subscriptions are not complete.
    *   A real `rapier3d-f64` gravity/contact/friction backend and scripted push-to-goal harness behind the `rapier` feature.
    *   A faithfulness-checked reference attribution producer plus first-class run-log and Rerun attribution handling.

2.  **Planned next environment work:**
    *   Integrate a 3DGS loader/asset pipeline and pin all external asset/model versions.
    *   Extend the implemented Rapier path beyond its box-collider scripted harness; add and pin MuJoCo/Isaac adapters only behind the same run-log and Agent Bridge contract.
    *   Connect external video/flow predictors only after simulator-derived `Flow_gt` is reliable.
    *   Expand the existing attribution faithfulness/provenance/relevance tracks into richer Rerun panels; defer custom SparkJS shaders to Phase 4.

### 11. Target Failure Policies (Specified, Not Fully Implemented)

These are target policies, not claims about completed end-to-end failure handling:

*   **Video predictor failure:** record the error and missing-flow status; the target default is to continue physics without ghost-flow data unless a preregistered Agent Bridge policy requests and logs a pause.
*   **Rerun disconnect:** because Rerun is a read-only viewer, the target default is for the headless run and canonical logging to continue.
*   **Zenoh disconnect:** Zenoh itself must not pause physics. Record transport loss; any pause/resume/fail-closed decision is an explicit Agent Bridge command written to the run log before execution.

---

## 12. Asset Library Specifications

### 12.1 Standard Mesh Assets

**Status:** This repo does not currently ship an `assets/` library. The items below are *planned conventions* for a shared asset pack; generate them (e.g., Blender) or pull from standard datasets (e.g., YCB) and keep large binaries out of git where appropriate.

The following OBJ and MTL definitions are intended as ground truth for physics proxies in later experiments. They use Z-up coordinate convention and meters as units.

#### 12.1.1 Hollow Cylinder (Tube)
**File:** `assets/meshes/hollow_cylinder.obj`
- **Dimensions:** Outer R=0.03m, Inner R=0.02m, Height=0.08m
- **Purpose:** Novel geometry challenge (exploratory novel-geometry manipulation task)
- **Material:** `hollow_cylinder.mtl` (Plastic)

#### 12.1.2 Dice Cup
**File:** `assets/meshes/dice_cup.obj`
- **Dimensions:** Outer R=0.04m, Height=0.06m
- **Purpose:** Containment target for weighted die (exploratory novel-geometry manipulation task)
- **Material:** `dice_cup.mtl` (Leather/Plastic)

#### 12.1.3 L-Shaped Block
**File:** `assets/meshes/l_block.obj`
- **Dimensions:** 0.08m x 0.04m x 0.04m (Horizontal), 0.04m x 0.04m x 0.08m (Vertical)
- **Purpose:** Compound geometry grasp planning (exploratory novel-geometry manipulation task)
- **Material:** `l_block.mtl` (Wood)

#### 12.1.4 Target Platform
**File:** `assets/meshes/target_platform.obj`
- **Dimensions:** 0.08m x 0.08m x 0.01m
- **Purpose:** Standard placement target
- **Material:** `target_platform.mtl` (Matte)

#### 12.1.5 Metal Peg
**File:** `assets/meshes/metal_peg.obj`
- **Dimensions:** Radius=0.015m, Height=0.06m
- **Purpose:** Precision alignment target for hollow cylinder
- **Material:** `metal_peg.mtl` (Metal)

#### 12.1.6 Glass Cube
**File:** `assets/meshes/glass_cube.obj`
- **Dimensions:** 0.06m x 0.06m x 0.06m
- **Purpose:** Transparent object physics proxy (exploratory novel-geometry manipulation task)
- **Material:** `glass_cube.mtl` (Transparent, index of refraction 1.52)

### 12.2 Material Definitions (MTL)

Common material properties used in `assets/meshes/*.mtl`:

| Material | Ka (Ambient) | Kd (Diffuse) | Ks (Specular) | Ns (Shininess) | d (Opacity) |
|----------|--------------|--------------|---------------|----------------|-------------|
| Plastic  | 0.1 0.1 0.1 | 0.6 0.6 0.7 | 0.3 0.3 0.3 | 50.0 | 1.0 |
| Wood | 0.1 0.08 0.05 | 0.6 0.45 0.3 | 0.1 0.1 0.1 | 10.0 | 1.0 |
| Metal | 0.2 0.2 0.2 | 0.7 0.7 0.75 | 0.9 0.9 0.9 | 100.0 | 1.0 |
| Glass | 0.0 0.0 0.0 | 0.1 0.1 0.1 | 0.9 0.9 0.9 | 200.0 | 0.15 (Tr 0.85) |
