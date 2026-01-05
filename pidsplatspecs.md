# PID-Splat Unified Simulation Environment Specification

> **Documentation Cross-Reference**:
> - `grandplan.md` — Master plan and theoretical foundations
> - `ARCHITECTURE.md` — Component breakdown and advantages over VLM-based robotics
> - `EXPERIMENTS.md` — Experimental protocols for SparkJS and Modular Physics setup and hypothesis testing
> - `DIAGRAMS.md` — Visual architecture diagrams
> - `README.md` — Quick start guide
## Technical Blueprint for the "Splat-First" Research Platform

**Version:** 7.0 (Aligned with docset; PID‑Splat target spec)
**Date:** 2026-01-05
**Context:** Canonical implementation spec for the simulation layer defined in `grandplan.md` §10.8 and §10.10.

---

### 1. Executive Summary

This document specifies the engineering implementation of the **PID-Splat** environment. It bridges photorealistic 3D Gaussian Splatting (3DGS) with modular rigid-body physics (**Rapier, MuJoCo, or Isaac Gym**) and Dream2Flow-style video→flow bridges to enable reproducible PID diagnostics for VLA models (offline-first; optionally interactive if measured budgets allow).

**Core Philosophy:** "Splat-First." We render reality (captured via 3DGS) and bind physics to it, while overlaying predicted “dreams” (video‑predicted 3D flow) to visualize what a policy *expects* to happen (treat predictors as experimental variables; no oracle framing).

---

### 2. Technology Stack & Versions

| Component | Technology | Version / Spec | License |
| :--- | :--- | :--- | :--- |
| **Frontend Shell** | Tauri | v2.0+ (Rust backend, WebView frontend) | MIT |
| **Renderer** | SparkJS (or equivalent WebGPU 3DGS renderer) | Pin version / git SHA | verify |
| **Splat Library** | gsplat | v1.0+ (via Nerfstudio for training) | Apache 2.0 |
| **Physics Engine** | Rapier3d / MuJoCo | Rapier v0.18+ / MuJoCo v3.0+ | Apache 2.0 |
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
- Example conversion command (from LeIsaac Marble tutorial):
  - `python -m threedgrut.export.scripts.ply_to_usd path/to/your/splats.ply --output_file path/to/output.usdz`

#### 3.2 LOD Strategy
*   **Target Count:** 500k - 2M gaussians per scene.
*   **Distance-based culling:** Alpha cull threshold increases with distance.
*   **Frustum culling:** Octree-based spatial indexing.

---

### 4. Dream2Flow Integration (Flow-as-Bridge)

This section implements the "Unified Architecture" from `grandplan.md` §10.10.

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
4.  **Publish:** Rust backend receives `dream/flow/{id}` via Zenoh.

#### 4.3 "Ghost Splat" Visualization
SparkJS renders these flows as **animated ghost splats** overlaying the real physics simulation.

*   **Visual Style:** Semi-transparent, glowing trails.
*   **Color Mapping:**
    *   **Red:** High Synergy (VLA "Dream" matches Reality).
    *   **Blue:** Unique V (Reality diverges from Dream).
    *   **Pulsing:** Opacity pulses with the beat of the flow.

---

### 5. Zenoh Middleware Protocol

#### 5.1 Key Expressions

| Key Expression | Data Type | Frequency | Source → Dest |
| :--- | :--- | :--- | :--- |
| `sim/pose/{id}` | `[f32; 7]` | 60Hz | Physics → SparkJS |
| `dream/flow/{id}`| `DreamFlowTrajectory` | Event | Video predictor/flow extractor → SparkJS |
| `pid/metric/{id}` | `PidStruct` | 10Hz | pid-core → SparkJS |
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

**Recommended transport (planned):**
- **JSON‑RPC 2.0 over WebSocket** on `127.0.0.1` (simple from Rust/TS/Python).
- Optional: **MCP server wrapper** exposing the same methods as tool calls (thin adapter; no separate logic).

**Minimum method surface (versioned; sketch):**
- `sim.start|pause|step|reset|status|set_seed`
- `scene.load|save|list|inspect` (introspection must be complete enough for agents to plan edits)
- `scene.add_object|remove_object|set_transform|set_material|set_physics`
- `camera.get|set|keyframe|render_snapshot`
- `intervention.apply|list|undo|branch` (all interventions are log events with timestamps and actor IDs)
- `log.start|stop|export|replay|attach_artifact`

**Live intervention semantics:**
- Interventions can be applied while the sim is running, but the system must enforce **backpressure** (bounded queues) and expose “paused/stepping” modes to keep interventions reproducible.
- Heavy jobs (video prediction, flow extraction, large PID bootstraps) are offline-first but still orchestrated through the same control plane so the provenance is logged.

**Safety defaults:**
- Local-only by default; explicit opt‑in for remote binding.
- Capability gating for destructive operations (e.g., deleting assets, overwriting logs).
- Option for read-only sessions (inspection without mutation).

---

### 6. Modular Physics Binding (PEGS)

#### 6.1 Splat-to-Physics Mapping
The environment supports multiple physics backends (**Rapier, MuJoCo, Isaac Gym**) via a unified trait interface.
*   **Manual Proxy:** User places primitive colliders (Box, Sphere) in the Tauri editor to match visual boundaries.
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

### 10. Implementation Plan

1.  **Infrastructure (Week 1-2):**
    *   Set up Tauri v2 + Rust workspace.
    *   Integrate `gsplat` (via SparkJS) WebGPU renderer.

2.  **Physics (Week 3-4):**
    *   Implement Physics loop (Rapier by default, MuJoCo optional).
    *   Create "Proxy Editor".

3.  **Dream2Flow (Week 5-6):**
    *   Implement `DreamFlowTrajectory` struct.
    *   Create "Ghost Splat" shader in SparkJS.
    *   Connect external video predictor output stream.

4.  **Integration (Week 7-8):**
    *   Visualize PID heatmaps on both Real and Ghost splats.

### 11. Error Handling

*   **Video predictor failure:** If the predictor fails to generate flow, "Ghost Splats" do not appear; simulation continues with physics only.
*   **WebGPU Failure:** Fall back to WebGL2.
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
