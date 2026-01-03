# PID-Splat Unified Simulation Environment Specification
## Technical Blueprint for the "Splat-First" Research Platform

**Version:** 2.0 (Engineering Ready)
**Date:** 2026-01-03
**Context:** Canonical implementation spec for the simulation layer defined in `grandplan.md` §10.8.

---

### 1. Executive Summary

This document specifies the engineering implementation of the **PID-Splat** environment. It bridges photorealistic 3D Gaussian Splatting (3DGS) with deterministic rigid-body physics (Rapier) to enable real-time Partial Information Decomposition (PID) diagnostics for VLA models.

**Core Philosophy:** "Splat-First." We render reality (captured via 3DGS) and bind physics to it, rather than rendering physics proxies (meshes) and trying to make them look real.

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
| **Frontend UI** | React + Three.js | React 19, r160+ (WebGPU compatible) | MIT |
| **Shader Lang** | WGSL | WebGPU Shading Language | Open |

---

### 3. Gaussian Splatting Specifics

#### 3.1 Pipeline & Formats
We support two formats for distinct lifecycle stages:
*   **`.PLY` (Raw):** Used during editing/debugging. Contains full SH coefficients (float32). Heavy memory usage.
*   **`.SPZ` (Compressed):** Used for runtime/distribution. Quantized (int8/f16). 10x smaller.

**Training Pipeline:**
1.  **Capture:** Polycam/Luma (iOS) or DSLR video.
2.  **Process:** `ns-train splatfacto --data <data_dir>` (Nerfstudio/gsplat backend).
3.  **Export:** `ns-export gaussian-splat --load-config <config> --output-format spz`.

#### 3.2 Performance & LOD
*   **Target Count:** 500k - 2M gaussians per scene.
*   **Memory Budget:** < 2GB VRAM for rendering (allowing room for VLA inference).
*   **LOD Strategy:**
    *   **Distance-based culling:** Alpha cull threshold increases with distance ($d > 5m \implies \alpha_{cutoff} = 0.1$).
    *   **Frustum culling:** Octree-based spatial indexing (built-in to SparkJS).

---

### 4. Headless Gazebo Integration

Gazebo runs as a background process solely for generating sensor data that Rapier cannot (e.g., specific camera distortion models or LiDAR).

#### 4.1 Configuration (`gazebo_config.yaml`)
```yaml
gazebo_version: harmonic
physics:
  engine: dart  # Only for sensor interaction, not main dynamics
  step_size: 0.001 # 1kHz
sensors:
  - name: wrist_cam
    type: camera
    resolution: [224, 224] # Matches OpenVLA input
    fov: 1.57 # 90 degrees
    update_rate: 30
ros2_bridge:
  enabled: true
  topics:
    - /camera/rgb/image_raw
    - /camera/depth/image_raw
```

---

### 5. Zenoh Middleware Protocol

We use **Zenoh** for high-throughput, low-latency IPC between the Rust backend (Physics/PID), the Frontend (Renderer), and the VLA.

#### 5.1 Key Expressions & Serialization
Format: **Bincode** (Rust) or **CDR** (standard ROS 2 compatibility) for performance. JSON is strictly for control commands.

| Key Expression | Data Type | Frequency | Source → Dest |
| :--- | :--- | :--- | :--- |
| `sim/pose/{id}` | `[f32; 7]` (Pos+Quat) | 60Hz | Rapier → SparkJS |
| `pid/metric/{id}` | `PidStruct` | 10Hz | pid-core → SparkJS |
| `vla/action` | `[f32; 7]` (Joints/Gripper) | ~5-10Hz | VLA → Rapier |
| `sys/control` | `Json` (Load/Reset) | Event | UI → Backend |

#### 5.2 PID Message Schema (Rust)
```rust
#[derive(Serialize, Deserialize, Clone)]
struct PidMetricMsg {
    timestamp_ns: u64,
    object_id: u32,
    synergy: f32,
    redundancy: f32,
    unique_v: f32,
    unique_l: f32,
    total_mi: f32,
}
```

---

### 6. Rapier Physics Binding (PEGS)

#### 6.1 Splat-to-Physics Mapping
We do *not* mesh the splats for physics. We use **Collision Proxies**:
1.  **Automatic:** Compute Convex Hull of the Splat point cloud (heavy, offline).
2.  **Manual (Preferred):** User places primitive colliders (Box, Sphere, Capsule) in the Tauri editor to match visual boundaries.

#### 6.2 Simulation Config
```rust
let integration_parameters = IntegrationParameters {
    dt: 1.0 / 60.0, // 60Hz physics step (matches rendering for smoothness)
    min_ccd_dt: 1.0 / 60.0 / 100.0, // Continuous Collision Detection
    erp: 0.8, // Error Reduction Parameter
    ..Default::default()
};
```

---

### 7. VLA Integration Interface

The system treats the VLA as an external agent interacting via Zenoh.

#### 7.1 Integration Points
1.  **Observation:** VLA subscribes to `sim/camera/rgb` (rendered by SparkJS or Gazebo) or grabs frames directly if local.
2.  **Action:** VLA publishes to `vla/action`. Tauri backend translates this to Rapier forces/position targets.
3.  **Embedding Extraction (The "Hook"):**
    *   The VLA Inference Server (Python/MLX) must expose an endpoint or Zenoh publisher for internal embeddings (`D`, `V`, `L`).
    *   **Spec:** Publisher `vla/embeddings` sends `(timestamp, layer_id, vector_f32)`.

---

### 8. PID Computation Pipeline

PID is computationally heavy ($O(N^2)$ for exact KSG). We decouple it from the render loop.

*   **Windowing:** Rolling window of $T=10$ to $T=50$ timesteps (configurable).
*   **Update Rate:** 10Hz target (asynchronous).
*   **Fallback:** If computation exceeds 100ms, skip frames (drop older samples).
*   **Memory:** Circular buffers in Rust backend to avoid re-allocation.

---

### 9. Performance Budget & Targets

| Metric | Target | Minimum Acceptable |
| :--- | :--- | :--- |
| **Render FPS** | 60 FPS | 30 FPS |
| **Physics Step** | 16ms (60Hz) | 33ms (30Hz) |
| **PID Latency** | < 100ms | < 500ms |
| **E2E Latency** | < 50ms | < 150ms |
| **VRAM Usage** | < 4 GB | < 8 GB |
| **System RAM** | < 16 GB | < 32 GB |

---

### 10. Implementation Plan (Refined)

1.  **Infrastructure (Week 1-2):**
    *   Set up Tauri v2 + Rust workspace.
    *   Integrate `gsplat` (via SparkJS) WebGPU renderer.
    *   Establish Zenoh bus.

2.  **Physics (Week 3-4):**
    *   Implement Rapier loop in Rust thread.
    *   Create "Proxy Editor" (place cubes over splats).

3.  **Integration (Week 5-6):**
    *   Connect Python VLA harness to Zenoh.
    *   Visualize PID heatmaps on splats (WGSL shader).

### 11. Error Handling & Fallbacks

*   **WebGPU Failure:** If WebGPU unavailable, fall back to WebGL2 (SparkJS supports both, though WebGL2 is slower for sorting).
*   **Zenoh Disconnect:** Physics pauses; UI shows "Reconnecting..." overlay.
*   **NaN in PID:** Clamp output to 0.0, log warning (do not crash renderer). Visualization maps NaN to specific color (e.g., Magenta).