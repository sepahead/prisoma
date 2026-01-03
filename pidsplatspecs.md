# PID-Splat Unified Simulation Environment Specification
## Technical Blueprint for the "Splat-First" Research Platform

**Version:** 3.0 (Dream2Flow Integrated)
**Date:** 2026-01-03
**Context:** Canonical implementation spec for the simulation layer defined in `grandplan.md` §10.8 and §10.10.

---

### 1. Executive Summary

This document specifies the engineering implementation of the **PID-Splat** environment. It bridges photorealistic 3D Gaussian Splatting (3DGS) with deterministic rigid-body physics (Rapier) and **generative video flow (Dream2Flow)** to enable real-time Partial Information Decomposition (PID) diagnostics for VLA models.

**Core Philosophy:** "Splat-First." We render reality (captured via 3DGS) and bind physics to it, while overlaying generative "dreams" (WAN-derived flow) to visualize what the VLA expects to happen.

---

### 2. Technology Stack & Versions

| Component | Technology | Version / Spec | License |
| :--- | :--- | :--- | :--- |
| **Frontend Shell** | Tauri | v2.0+ (Rust backend, WebView frontend) | MIT |
| **Renderer** | SparkJS | 2025 Release (WebGPU backend) | MIT |
| **Splat Library** | gsplat | v1.0+ (via Nerfstudio for training) | Apache 2.0 |
| **Physics Engine** | Rapier3d | v0.18+ (Rust native) | Apache 2.0 |
| **Middleware** | Zenoh | v1.0 (Zero-copy, shared memory) | Apache 2.0 |
| **Sensor Sim** | Gazebo | Harmonic (gz-sim 8.x) | Apache 2.0 |
| **Video Gen** | WAN | v2.2 (for Dream2Flow) | Apache 2.0 |
| **Flow Tracker** | CoTracker3 | v3.0 (Meta) | CC-BY-NC |

---

### 3. Gaussian Splatting Specifics

#### 3.1 Pipeline & Formats
*   **`.SPZ` (Compressed):** Used for runtime/distribution.
*   **`.PLY` (Raw):** Used during editing/debugging.

**Training Pipeline:**
1.  **Capture:** Polycam/Luma (iOS) or DSLR video.
2.  **Process:** `ns-train splatfacto --data <data_dir>` (Nerfstudio/gsplat backend).
3.  **Export:** `ns-export gaussian-splat --load-config <config> --output-format spz`.

#### 3.2 LOD Strategy
*   **Target Count:** 500k - 2M gaussians per scene.
*   **Distance-based culling:** Alpha cull threshold increases with distance.
*   **Frustum culling:** Octree-based spatial indexing.

---

### 4. Dream2Flow Integration (New in v3.0)

This section implements the "Unified Architecture" from `grandplan.md` §10.10.

#### 4.1 3D Flow Data Structure
We represent the "Dream" not just as a hidden state, but as explicit 3D trajectories extracted from WAN-generated videos.

```rust
/// Represents a single object's predicted path (the "Flow")
#[derive(Serialize, Deserialize, Clone)]
struct DreamFlowTrajectory {
    object_id: u32,
    /// Sequence of (x, y, z) points over time T
    points: Vec<[f32; 3]>,
    /// Confidence/Opacity per point (from CoTracker3)
    confidence: Vec<f32>,
    /// PID Synergy at each point Syn(V, D; Flow_t)
    synergy: Vec<f32>,
}
```

#### 4.2 WAN Integration Pipeline
The WAN video generation happens externally (Python/CUDA) and feeds into the visualization via Zenoh.

1.  **Trigger:** VLA sends `(Image, Instruction)` to WAN Service.
2.  **Generate:** WAN 2.2 generates 2s video.
3.  **Extract:** CoTracker3 + Depth-Anything v3 extracts `DreamFlowTrajectory`.
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
| `sim/pose/{id}` | `[f32; 7]` | 60Hz | Rapier → SparkJS |
| `dream/flow/{id}`| `DreamFlowTrajectory` | Event | WAN → SparkJS |
| `pid/metric/{id}` | `PidStruct` | 10Hz | pid-core → SparkJS |
| `vla/action` | `[f32; 7]` | ~10Hz | VLA → Rapier |

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

---

### 6. Rapier Physics Binding (PEGS)

#### 6.1 Splat-to-Physics Mapping
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

| Metric | Target | Minimum Acceptable |
| :--- | :--- | :--- |
| **Render FPS** | 60 FPS | 30 FPS |
| **Physics Step** | 16ms (60Hz) | 33ms (30Hz) |
| **Flow Viz** | < 5ms (instanced) | < 10ms |
| **E2E Latency** | < 50ms | < 150ms |

---

### 10. Implementation Plan

1.  **Infrastructure (Week 1-2):**
    *   Set up Tauri v2 + Rust workspace.
    *   Integrate `gsplat` (via SparkJS) WebGPU renderer.

2.  **Physics (Week 3-4):**
    *   Implement Rapier loop.
    *   Create "Proxy Editor".

3.  **Dream2Flow (Week 5-6):**
    *   Implement `DreamFlowTrajectory` struct.
    *   Create "Ghost Splat" shader in SparkJS.
    *   Connect WAN output stream.

4.  **Integration (Week 7-8):**
    *   Visualize PID heatmaps on both Real and Ghost splats.

### 11. Error Handling

*   **WAN Failure:** If WAN fails to generate flow, "Ghost Splats" do not appear; simulation continues with physics only.
*   **WebGPU Failure:** Fall back to WebGL2.
*   **Zenoh Disconnect:** Physics pauses; UI shows "Reconnecting...".
