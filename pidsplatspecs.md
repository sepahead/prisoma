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

**Version:** 10.6 (Rerun-First Architecture; implementation-status refresh)
**Date:** 2026-06-22
**Context:** Canonical implementation spec for the simulation layer defined in `grandplan.md` §10.8 and §10.10.

---

### 1. Executive Summary

This document specifies the target engineering implementation of the **PID-Splat** environment. It bridges photorealistic 3D Gaussian Splatting (3DGS) with modular rigid-body physics (**Rapier, MuJoCo, or Isaac Gym**) and Dream2Flow-style video→flow bridges to enable reproducible PID diagnostics for VLA models.

**Visualization Strategy (v10.1): "Rerun-First"**
To accelerate research iteration (Phases 1-3), the system prioritizes **Rerun** (https://rerun.io/) as the primary visualization, logging, and "time machine" engine. The custom Tauri/SparkJS frontend is deferred to Phase 4 for advanced interactive needs.

**Docset-wide final solution:** `grandplan.md` §A.8 is the decision record. The run log is the canonical state, Rerun is the research viewer, Tauri/SparkJS is a later control/editor/custom-rendering shell, and every control path must go through the Agent Bridge.

**Core Philosophy:** "Splat-First." We render reality (captured via 3DGS) and bind physics to it, while overlaying predicted “dreams” (video‑predicted 3D flow) to visualize what a policy *expects* to happen.

**Contact/collision reality check:** existing “3DGS-based” simulators still use conventional physics engines for contacts. Treat splats as the appearance layer; use explicit collision geometry (URDF/MJCF primitives/meshes) in the physics backend.

**Multi-engine note (v10.1):** treat the physics backend as a **per-run** choice for contact-rich scenes. A more practical differentiator is **cross-backend replay** (re-run the same action log in Rapier vs MuJoCo and report divergence) as a robustness/confound control (see `grandplan.md` §E.1).

---

### 2. Technology Stack & Versions

| Component | Technology | Version / Spec | License |
| :--- | :--- | :--- | :--- |
| **Run log** | `pid-rs/crates/pid-runlog` JSONL events + replay summary | Schema v1; M1 groundwork implemented; includes embedding/sim/bridge event types, validation, replay hash comparison, summary JSON with unique metric-name counts plus total metric-event counters, manifest JSON, and co-located sidecar writing/verification | MIT (project) |
| **Agent Bridge core** | `crates/pid-bridge` | Local request/response schema, dispatcher, JSON-RPC-shaped request/response conversion, run-log integration, bridge/run-log contract JSON export, safe-mode gates, and stdio/TCP/WebSocket sim transports | MIT (project) |
| **Deterministic sim smoke** | `crates/pid-sim` | Object-only fixed-step sim + simulator-derived `Flow_gt`; bridge demo, stdio/TCP/WebSocket JSON-RPC bridges, `log.replay`, `log.start`/`log.stop`, deterministic `intervention.apply`, `export.rerun`, flow verification CLI, deterministic action/intervention replay checks, toy labeled harness, and offline `(V,L,D,A)` artifact-to-runlog harness with all-pairs `V/L/D→A` PID screens plus train-split-only PID screens when a metadata split is present, standardization provenance, geometry diagnostics/gates, fail-closed strict label/geometry/held-out-split/held-out-class-coverage/held-out-episode-disjoint modes, sample-level, episode-grouped, plus metadata-split held-out majority/1-NN/nearest-centroid success-label baselines with accuracy, balanced accuracy, and centroid AUROC, plus held-out class-coverage and episode-disjointness reports, per-sample prediction records in summaries/run logs, replay-visible total metric event counts, and failure-class confusion/rate diagnostics; a `PhysicsBackend` trait with a null adapter and a real `rapier3d-f64` backend (gravity/contacts/friction, deterministic) + a scripted push-to-goal manipulation exists behind the optional `rapier` feature (box-collider geometry; mesh-collider ingestion and MuJoCo/Isaac adapters remain planned) | MIT (project) |
| **Attribution probes (H9)** | Offline explainer artifacts | Planned companion diagnostics only: LRP/IG/DeepLIFT/Grad-CAM/TCAV/saliency/occlusion/SHAP-style tensors or visualizations should be logged via existing artifact records with method/target/baseline/hash metadata until a stable first-class attribution schema is justified | Method/model-dependent; verify |
| **Visualization** | **Rerun** (Phases 1-3) / Tauri (Phase 4) | Rerun SDK 0.28.x in Cargo; run-log conversion includes summary/provenance/validation diagnostic tracks; Tauri version to pin when implemented | Rerun: MIT OR Apache-2.0; Tauri API package metadata: Apache-2.0 OR MIT |
| **Renderer** | Rerun native/WebViewer / SparkJS (Phase 4) | Pin exact package versions / git SHAs at implementation time | Rerun WebViewer: MIT; SparkJS package metadata: MIT; Three.js: MIT |
| **Splat Library** | gsplat | v1.0+ (via Nerfstudio for training) | Apache 2.0 |
| **Physics Engine** | Rapier3d / MuJoCo | Planned backend adapters; pin exact versions when added | Apache 2.0 |
| **Middleware** | Zenoh | Pub/sub transport; shared memory/zero-copy is config-dependent | Apache 2.0 |
| **Sensor Sim** | Gazebo | Harmonic (gz-sim 8.x) | Apache 2.0 |
| **Video predictor** | Video model (external service) | Model-dependent (pin revision) | verify |
| **Flow Tracker** | Point tracker (e.g., CoTracker) | Model-dependent (pin revision) | verify |
| **Agent Bridge (control plane)** | JSON‑RPC over WebSocket (+ optional MCP wrapper) | Versioned local API for live interventions + automation | MIT (project) |

---

### 3. Gaussian Splatting Specifics

#### 3.1 Pipeline & Formats
*   **`.SPZ` (Compressed):** Used for runtime/distribution.
*   **`.PLY` (Raw):** Used during editing/debugging.

**Training Pipeline:**
1.  **Capture:** Polycam/Luma (iOS) or DSLR video.
2.  **Process:** `ns-train splatfacto --data <data_dir>` (Nerfstudio/gsplat backend).
3.  **Export:** `ns-export gaussian-splat --load-config <config> --output-format spz`.

**Optional OpenUSD / USDZ interop (Isaac Sim / LeIsaac workflows):**
- Convert splat `.ply` → `.usdz` (packaged OpenUSD) using NVIDIA 3DGrut, then compose splats + collision mesh in Isaac Sim to export a single `.usd` background stage (see `grandplan.md` §C.1 and `DIAGRAMS.md` §9).

#### 3.2 LOD Strategy
*   **Starting range (benchmark-dependent):** ~0.5M–2M gaussians per scene.
*   **Distance-based culling:** Alpha cull threshold increases with distance.
*   **Frustum culling:** Octree-based spatial indexing.

---

### 4. Dream2Flow Integration (Flow-as-Bridge)

This section specifies the target "Unified Architecture" from `grandplan.md` §10.10.

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
    synergy: Vec<f32>,
}
```

#### 4.2 Video Predictor Integration Pipeline
Video prediction happens externally (e.g., Python/CUDA or a hosted API) and feeds into the visualization via Zenoh.

1.  **Trigger:** VLA (or the orchestrator) sends `(Image, Instruction)` to a video predictor service.
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

#### 5.1 Key Expressions

| Key Expression | Data Type | Frequency | Source → Dest |
| :--- | :--- | :--- | :--- |
| `sim/pose/{id}` | `[f32; 7]` | 60Hz | Physics → run log → Rerun (P1-3) / SparkJS (P4) |
| `scene/uncertainty` | `SceneUncertaintyMap` | Event | GauSS‑MI (optional) → run log → UI/PID |
| `dream/flow/{id}`| `DreamFlowTrajectory` | Event | Video predictor/flow extractor → run log → Rerun (P1-3) / SparkJS (P4) |
| `pid/metric/{id}` | `PidStruct` | 10Hz | pid-core → run log → Rerun (P1-3) / SparkJS (P4) |
| `vla/action` | `[f32; 7]` | ~10Hz | VLA → Physics |

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

**Core rule:** the UI uses the same control plane as external tools (no hidden manual paths). All API calls emit an **audit event** into the run log for reproducibility.

**Recommended transport:**
- **JSON‑RPC 2.0 over WebSocket** on `127.0.0.1` (the deterministic sim smoke has this transport; full UI integration still builds on the same surface).
- Optional: **MCP server wrapper** exposing the same methods as tool calls (thin adapter; no separate logic).

The in-repo deterministic bridge currently exposes status/reset/step, scene edits, deterministic interventions (`set_velocity`, `translate_object`, `set_pose`), `log.replay`, `log.start`/`log.stop`, and `export.rerun`; safe mode allows status/replay and blocks mutation, run-ending, or file-writing export requests.

---

### 6. Modular Physics Binding (PEGS)

#### 6.1 Splat-to-Physics Mapping
The target environment supports multiple physics backends (**Rapier, MuJoCo, Isaac Gym**) via a unified trait interface. The checked repo currently has the deterministic object-sim smoke plus a `PhysicsBackend` trait with a null adapter and a real `rapier3d-f64` backend (gravity/contacts/friction) + a scripted push-to-goal manipulation behind the optional `rapier` feature; MuJoCo/Isaac backend adapters remain planned.
*   **Manual Proxy:** User defines primitive colliders (Box, Sphere) in code/config (visualized in Rerun) or uses the Tauri editor (Phase 4) to match visual boundaries.
*   **Visual Forces:** If `Syn(V, Flow; A)` drops, we can optionally apply "correction forces" to nudge the physics simulation toward the Dream (counterfactual analysis).

---

### 7. VLA Integration Interface

#### 7.1 Integration Points
1.  **Observation:** VLA subscribes to `sim/camera/rgb`.
2.  **Action:** VLA publishes to `vla/action`.
3.  **Embedding Extraction:**
    *   **Publisher:** `vla/embeddings` sends `(timestamp, layer_id, vector_f32)`.
    *   **Used By:** `pid-core` to compute `Syn(V, D; Flow)`.

---

### 8. PID Computation Pipeline

*   **Target:** We compute PID on **Flow** trajectories (Euclidean) to bypass the manifold geometry issues of raw embeddings (Flow-as-bridge).
*   **Metric:** `Syn(V_embedding, D_embedding; Flow_trajectory)`.
*   **Windowing:** Rolling window of T=10 to T=50 timesteps.

---

### 9. Performance Budget & Targets

**Measurement-first:** treat latency/throughput as empirical properties of your hardware + scene + models. For interactive work, start with offline playback and progressively move components in-loop only after you have measured budgets and uncertainty (see `grandplan.md` §A and `EXPERIMENTS.md` §12).

---

### 10. Implementation Plan / Current Status

1.  **Implemented groundwork:**
    *   Rust workspace with `pid-bridge`, `pid-sim`, and `pid-rerun`; the `pid-core`, `pid-runlog`, and `pid-python` crates live in the [`pid-rs`](https://github.com/sepahead/pid-rs) submodule (single source of truth).
    *   Canonical run-log schema, replay validation, summaries/manifests, sidecar write-and-verify, and a run-log-to-Rerun adapter.
    *   Deterministic object-sim smoke with simulator-derived `Flow_gt`, constant-velocity `flow_pred`, Agent Bridge stdio/TCP/WebSocket smokes, toy labeled harness, and offline `(V,L,D,A)` artifact harness.

2.  **Planned next environment work:**
    *   Integrate a 3DGS loader/asset pipeline and pin all external asset/model versions.
    *   Add a real physics backend loop (Rapier or MuJoCo first) behind the same run-log and Agent Bridge contract.
    *   Connect external video/flow predictors only after simulator-derived `Flow_gt` is reliable.
    *   Visualize PID and attribution artifacts on Rerun entities first; defer custom SparkJS shaders to Phase 4.

### 11. Error Handling

*   **Video predictor failure:** If the predictor fails to generate flow, "Ghost Splats" do not appear; simulation continues with physics only.
*   **Rerun Disconnect:** Simulation continues running headless; logs are preserved.
*   **Zenoh Disconnect:** Physics pauses; UI shows "Reconnecting...".

---

## 12. Asset Library Specifications

### 12.1 Standard Mesh Assets

**Status:** This repo does not currently ship an `assets/` library. The items below are *planned conventions* for a shared asset pack; generate them (e.g., Blender) or pull from standard datasets (e.g., YCB) and keep large binaries out of git where appropriate.

The following OBJ and MTL definitions are intended as ground truth for physics proxies in later experiments. They use Z-up coordinate convention and meters as units.

#### 12.1.1 Hollow Cylinder (Tube)
**File:** `assets/meshes/hollow_cylinder.obj`
- **Dimensions:** Outer R=0.03m, Inner R=0.02m, Height=0.08m
- **Purpose:** Novel geometry challenge (Exp 7)
- **Material:** `hollow_cylinder.mtl` (Plastic)

#### 12.1.2 Dice Cup
**File:** `assets/meshes/dice_cup.obj`
- **Dimensions:** Outer R=0.04m, Height=0.06m
- **Purpose:** Containment target for weighted die (Exp 7)
- **Material:** `dice_cup.mtl` (Leather/Plastic)

#### 12.1.3 L-Shaped Block
**File:** `assets/meshes/l_block.obj`
- **Dimensions:** 0.08m x 0.04m x 0.04m (Horizontal), 0.04m x 0.04m x 0.08m (Vertical)
- **Purpose:** Compound geometry grasp planning (Exp 7)
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
- **Purpose:** Transparent object physics proxy (Exp 7)
- **Material:** `glass_cube.mtl` (Transparent, index of refraction 1.52)

### 12.2 Material Definitions (MTL)

Common material properties used in `assets/meshes/*.mtl`:

| Material | Ka (Ambient) | Kd (Diffuse) | Ks (Specular) | Ns (Shininess) | d (Opacity) |
|----------|--------------|--------------|---------------|----------------|-------------|
| Plastic  | 0.1 0.1 0.1 | 0.6 0.6 0.7 | 0.3 0.3 0.3 | 50.0 | 1.0 |
| Wood | 0.1 0.08 0.05 | 0.6 0.45 0.3 | 0.1 0.1 0.1 | 10.0 | 1.0 |
| Metal | 0.2 0.2 0.2 | 0.7 0.7 0.75 | 0.9 0.9 0.9 | 100.0 | 1.0 |
| Glass | 0.0 0.0 0.0 | 0.1 0.1 0.1 | 0.9 0.9 0.9 | 200.0 | 0.15 (Tr 0.85) |
