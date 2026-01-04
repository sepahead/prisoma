# PID-Splat Architecture: Components & Comparative Advantages

> **Documentation Cross-Reference**:
> - `grandplan.md` — Master plan and theoretical foundations
> - `pidsplatspecs.md` — Detailed simulation environment specifications
> - `EXPERIMENTS.md` — Experimental protocols for SparkJS and Modular Physics setup and hypothesis testing
> - `DIAGRAMS.md` — Visual architecture diagrams
> - `README.md` — Quick start guide

---

## 1. Core System Components

### 1.1 Tauri Application (Desktop Framework)

**What it does:**
- Provides the cross-platform desktop application shell
- Runs a Rust backend with native performance for PID computation
- Hosts the WebGPU-based SparkJS renderer for Gaussian Splat visualization
- Manages IPC (Inter-Process Communication) between:
  - Rust PID-core (computation)
  - React/Three.js frontend (visualization)
  - Zenoh bridge (simulation data streaming)

**Why it matters:**
- Native performance: Rust backend runs PID estimators at <100ms/window
- Real-time visualization: 16ms/frame rendering on M4 Max
- Unified interface: Single app combines simulation control, VLA inference monitoring, and PID analysis

**Stack:**
```
┌─────────────────────────────────────────────────────────┐
│                    Tauri v2 Shell                       │
├─────────────────────────────────────────────────────────┤
│  Frontend (React/Three.js)  │  Backend (Rust)          │
│  ├─ SparkJS 3DGS Renderer   │  ├─ PID-Core estimators  │
│  ├─ PID Heatmap Overlays    │  ├─ Zenoh subscriber     │
│  └─ Control Panel           │  └─ MLX inference hooks  │
└─────────────────────────────────────────────────────────┘
```

### 1.2 SparkJS (WebGPU Renderer)

**What it does:**
- Custom WebGPU renderer for 3D Gaussian Splatting (3DGS)
- Implements "Dynos" — WGSL shaders that modify splat color buffers based on PID metrics
- Real-time LOD (Level of Detail) for 60fps on consumer hardware

**Why it matters:**
- **Photorealistic rendering**: 3DGS provides camera-realistic views without expensive ray-tracing
- **Differentiable**: Splat rendering is fully differentiable, enabling gradient-based world model training
- **PID Overlay**: Dynos colorize splats based on information flow:
  - Red = High Synergy
  - Blue = High Unique Information
  - Green = High Redundancy

### 1.3 Modular Physics Engine (Rapier, MuJoCo, Isaac Gym)

**What it does:**
- Provides rigid body physics simulation via a pluggable backend system
- Supports **Rapier3D** (Rust-native, default), **MuJoCo** (industry standard), and **Isaac Gym** (GPU-parallel)
- Handles collision detection, joint constraints, friction, restitution
- Rapier runs at <1ms/step, enabling 1000Hz internal physics with 100Hz external control

**Key capabilities (Rapier implementation):**
```rust
// Table collider with realistic friction
let table_collider = ColliderBuilder::cuboid(0.60, 0.40, 0.375)
    .friction(0.4)
    .restitution(0.1)
    .build();

// Object with density-based mass
let cube_collider = ColliderBuilder::cuboid(0.025, 0.025, 0.025)
    .friction(0.5)
    .density(800.0)  // kg/m³, results in ~100g
    .build();
```

**Why it matters:**
- **Determinism**: Fully deterministic physics (critical for reproducibility in Rapier)
- **Modularity**: Select the best engine for the task (Rapier for speed, MuJoCo for contact accuracy)
- **Integration**: Native Rust (Rapier) = zero-copy data flow to PID-core; FFI for MuJoCo/Isaac

### 1.4 Gazebo Harmonic (Robot Simulation)

**What it does:**
- Industry-standard robot simulation (URDF/SDF support)
- Sensor simulation (RGB-D cameras, joint encoders, force/torque)
- Headless mode for batch experiments

**Integration architecture:**
```
┌─────────────────┐    Zenoh    ┌─────────────────┐
│ Gazebo Harmonic │◄──────────►│  Tauri App      │
│ (Headless)      │            │  ├─ SparkJS     │
│ ├─ Robot URDF   │            │  ├─ PID-Core    │
│ ├─ Sensors      │            │  └─ Controls    │
│ └─ ros_gz_bridge│            └─────────────────┘
└─────────────────┘
```

**Why separate Physics and Robot Simulation?**

| Component | Use Case | When to Use |
|-----------|----------|-------------|
| **Physics Engine** | Object manipulation physics (fast, deterministic) | Object-object interactions, perturbations, fast iteration |
| **Robot Sim** | Robot kinematics/dynamics, sensor simulation | Robot URDF loading, sensor data, cross-embodiment |

The "Splat-First Physics" approach:
- Robot Sim (Gazebo/MuJoCo) handles complex robot dynamics (Franka, UR5e URDFs)
- Physics Engine (Rapier/MuJoCo) handles object manipulation (grasping, stacking, placing)
- 3DGS provides visual rendering for both

**Per-Hypothesis Engine Usage** (see `EXPERIMENTS.md` for full details):

| Hypothesis | Physics | Robot | Notes |
|------------|--------|--------|-------|
| H1 (Synergy → hallucination) | ✓ | ✓ | Object poses + robot state |
| H4 (Memorization vs generalization) | ✓ | | Mass/friction perturbations |
| H5 (Temporal degradation) | ✓ | ✓ | Long-horizon contact physics |
| H6 (Safety-aware V-L integration) | ✓ | ✓ | Collision detection for safety |
| H7 (Flow-as-bridge) | | | Flow from WAN video gen, no physics sim needed |

### 1.5 Gaussian Splatting (3DGS) Pipeline

**What it does:**
- Captures real-world scenes/objects via photogrammetry (iPhone + Polycam)
- Trains neural radiance representation (Nerfstudio splatfacto)
- Exports compressed `.spz` files for real-time rendering

**Pipeline:**
```bash
# 1. Capture (iPhone 15 Pro, Polycam, 4K @30fps, 360° orbit)
# 2. Train
ns-train splatfacto \
    --data ./captures/scene/ \
    --max-num-iterations 30000 \
    --pipeline.model.num-gaussians 800000

# 3. Export
ns-export gaussian-splat \
    --load-config outputs/scene/splatfacto/config.yml \
    --output-dir ./assets/splats/ \
    --output-format spz

# 4. Load in SparkJS for real-time rendering
```

**Asset specifications:**

| Object | Gaussian Count | Physics Proxy |
|--------|----------------|---------------|
| red_cube | ~15,000 | Cuboid |
| blue_cylinder | ~20,000 | Cylinder |
| ycb_mustard | ~40,000 | Convex Hull |
| tabletop_scene | ~800,000 | Static mesh |

**Why it matters:**
- **Real2Sim photorealism**: Captured splats look like real images (domain gap minimization)
- **Object-centric assets**: Each manipulated object is a separate splat (compositional scenes)
- **Differentiable rendering**: Enables gradient flow through visual observations

### 1.6 Dream2Flow Integration (World Model Bridge)

**What it does:**
- Uses WAN 2.2 video generation to "dream" future trajectories
- SAM3 + CoTracker3 + Depth-Anything v3 extracts 3D object flow from dreamed videos
- 3D Flow becomes a **target variable** for PID analysis

> **Why Flow-as-Bridge is Critical**: The validated ISX estimator (`EhrlichKsg`) only supports Chebyshev (L∞) metric and **cannot** handle hyperbolic/Lorentzian geometry. VLA embeddings may lie on curved manifolds where standard PID estimation fails. By using 3D Object Flow (Euclidean R³) as the target, we sidestep manifold issues entirely.

**Pipeline:**
```
Current Image + Instruction
         │
         ▼
    ┌─────────┐
    │ WAN 2.2 │ (48 frames, 2 seconds @ 24fps)
    └────┬────┘
         │
         ▼
    ┌─────────┐
    │  SAM3   │ (Segment objects in frame 0)
    └────┬────┘
         │
         ▼
  ┌───────────┐
  │ CoTracker3│ (Track 2D points through video)
  └─────┬─────┘
         │
         ▼
┌─────────────────┐
│ Depth-Anything  │ (Estimate per-frame depth)
│      v3         │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 3D Object Flow  │ (Lift 2D tracks to 3D)
│  Trajectory     │
└─────────────────┘
```

**Why it matters (Critical for Hypothesis H7):**
- **Embodiment-agnostic**: 3D flow is independent of robot morphology
- **Euclidean target**: Avoids manifold geometry problems of high-D embeddings
- **World model probe**: Tests if VLA's internal model predicts physically consistent futures

---

## 2. Simulator Comparison: Why Gaussian Splats + Modular Physics

### 2.1 Comprehensive Simulator Comparison

| Simulator | Rendering | Physics | Domain Gap | Speed | VLA Suitability |
|-----------|-----------|---------|------------|-------|------------------|
| **MuJoCo** | Mesh-based, synthetic | ⭐⭐⭐⭐⭐ Excellent contacts | High | Fast | Good physics, poor visuals |
| **PyBullet** | Basic OpenGL | ⭐⭐⭐ Adequate | High | Medium | Legacy, limited |
| **Isaac Gym** | RTX ray-tracing | ⭐⭐⭐⭐ Good | Medium | Very Fast (GPU) | Best traditional sim |
| **Habitat** | Mesh + neural | ⭐⭐ Navigation only | Medium | Fast | Navigation tasks only |
| **CARLA** | Unreal Engine | ⭐⭐⭐ Vehicle physics | Low-Medium | Slow | Driving only |
| **robosuite** | MuJoCo backend | ⭐⭐⭐⭐⭐ MuJoCo | High | Fast | Standard benchmark |
| **Gazebo Harmonic** | OGRE2/Vulkan | ⭐⭐⭐⭐ Good robot sim | Medium | Medium | Industry standard |
| **Rapier3D** | None (headless) | ⭐⭐⭐ Good rigid body | N/A | Very Fast | Fast iteration |
| **Gaussian Splats** | Photorealistic | N/A (needs physics) | **Minimal** | 60fps | **Best for VLA visuals** |

### 2.2 Why Each Simulator Falls Short for VLA Diagnostics

| Simulator | Limitation for PID-VLA |
|-----------|------------------------|
| **MuJoCo/robosuite** | Synthetic visuals → domain gap → VLA behaves differently than on real images |
| **PyBullet** | Outdated rendering, poor visual fidelity |
| **Isaac Gym** | Requires NVIDIA GPU, complex setup, still not photorealistic |
| **Habitat** | Navigation-only, no manipulation |
| **CARLA** | Driving-only, no manipulation |
| **Gazebo** | Better visuals but still synthetic, slow for batch experiments |

### 2.3 The PID-Splat Solution: Composable Backends

```
┌─────────────────────────────────────────────────────────────┐
│                    PID-Splat Architecture                   │
├─────────────────────────────────────────────────────────────┤
│  RENDERING        │  PHYSICS           │  ROBOT SIMULATION  │
│  ────────         │  ───────           │  ────────────────  │
│  Gaussian Splats  │  Modular Backend   │  Gazebo (accurate) │
│  (photorealistic) │  (Rapier, MuJoCo,  │  OR                │
│                   │   Isaac Gym)       │  MuJoCo (legacy)   │
└─────────────────────────────────────────────────────────────┘
```

**Key Insight**: Decouple rendering from physics. Use Gaussian Splats for visuals (minimal domain gap) + pluggable physics backend for simulation.

### 2.4 Modular Physics Backend Selection

Users can select physics backend based on their needs:

| Use Case | Recommended Backend | Why |
|----------|---------------------|-----|
| **Fast iteration / prototyping** | Rapier3D | <1ms/step, Rust-native |
| **Accurate contact physics** | MuJoCo | Gold standard for manipulation |
| **GPU-parallel batch experiments** | Isaac Gym | 10,000+ parallel envs |
| **Robot kinematics/sensors** | Gazebo Harmonic | Industry-standard URDFs |
| **Benchmark comparison** | MuJoCo + robosuite | Match existing VLA papers |

**Configuration example:**

```toml
# pid-splat.toml
[physics]
backend = "rapier"  # Options: "rapier", "mujoco", "isaac"

[physics.rapier]
step_hz = 1000
deterministic = true

[physics.mujoco]
model_path = "assets/mujoco/franka.xml"
step_hz = 500

[physics.isaac]
gpu_id = 0
num_envs = 1024

[rendering]
backend = "splat"  # Always use Gaussian Splats for VLA studies

[robot]
backend = "gazebo"  # Options: "gazebo", "mujoco", "none"
urdf_path = "assets/robots/franka_panda.urdf"
```

### 2.5 Camera & Environment Simulation

| Feature | MuJoCo/robosuite | Isaac Gym | Gaussian Splats |
|---------|------------------|-----------|------------------|
| **Multi-view cameras** | ✓ Fixed viewpoints | ✓ Any viewpoint | ✓ Any viewpoint (within capture) |
| **Lighting changes** | Re-render needed | Re-render needed | **Real-time Dynos** |
| **Camera intrinsics** | Manual setup | Manual setup | **Real-time modification** |
| **Motion blur** | Not supported | Limited | **Post-process shader** |
| **Lens distortion** | Not supported | Limited | **Post-process shader** |
| **New environments** | Days (3D modeling) | Days (3D modeling) | **Minutes (iPhone capture)** |

---

## 3. Advantages Over Existing VLM-Based Robotics

### 3.1 Comparison with OpenVLA / PixelVLA / TraceVLA

| Aspect | OpenVLA et al. | PID-Splat |
|--------|----------------|-----------|
| **Simulation** | MuJoCo/PyBullet (mesh-based) | Gaussian Splats + Rapier (photorealistic + fast) |
| **Visual Fidelity** | Synthetic renders, domain gap | Real-captured splats, minimal domain gap |
| **Analysis** | Task success rate only | PID decomposition reveals *why* success/failure |
| **World Model** | Implicit in LLM hidden states | Explicit 3D flow extraction for validation |
| **Embodiment Transfer** | Per-robot fine-tuning | Flow-as-bridge tests embodiment-agnostic understanding |

**Key advantage**: OpenVLA/PixelVLA report success rates but cannot diagnose *why* a policy fails. PID-Splat decomposes information flow to identify:
- **Hallucination**: Negative synergy indicates subadditive integration (H1)
- **Memorization vs Generalization**: PID signatures distinguish these (H4)
- **Compositional failure**: Temporal synergy degradation in long-horizon tasks (H5)

### 3.2 Comparison with VLMarena

| Aspect | VLMarena | PID-Splat |
|--------|----------|-----------|
| **Focus** | Benchmark suite (what) | Diagnostic framework (why) |
| **Rendering** | Standard simulators | 3DGS photorealism |
| **Metrics** | Task completion, collision rate | Information-theoretic decomposition |
| **Scalability** | N models × M tasks = N×M runs | Single analysis reveals modality contributions |

**Key advantage**: VLMarena tells you *which* model performs better. PID-Splat tells you *why* one model integrates vision-language better than another.

### 3.3 Comparison with Dream2Flow

| Aspect | Dream2Flow (original) | PID-Splat Dream2Flow |
|--------|----------------------|---------------------|
| **Purpose** | Flow prediction for action | Flow as PID target for world model analysis |
| **Integration** | End-to-end training signal | Post-hoc diagnostic probe |
| **Novelty** | Action generation | Information decomposition of world model quality |

**Key advantage**: Original Dream2Flow uses flow for action generation. PID-Splat uses flow to *test* whether the VLA's world model (D) is consistent with physics, independent of the action decoder.

### 3.4 Comparison with DreamVLA

| Aspect | DreamVLA | PID-Splat |
|--------|----------|-----------|
| **World Model** | Explicit `<dream>` tokens | Explicit via Dream2Flow extraction |
| **Analysis** | Task performance | PID(V, D; Flow) reveals world model quality |
| **Architecture** | Fixed training objective | Post-hoc analysis, any VLA |

**Key advantage**: DreamVLA trains with dreaming; PID-Splat *analyzes* any VLA's implicit world model without retraining.

---

## 3. World Model vs VLM-Based Robotics: The Core Argument

### 3.1 The Problem with Pure VLM-Based Robotics

Current VLAs (OpenVLA, PixelVLA) are essentially:
```
Image + Instruction → LLM → Action Tokens
```

**Failure modes:**
1. **No physics grounding**: LLM has no explicit model of 3D physics
2. **Hallucination risk**: Language models can generate confident but wrong actions
3. **Opaque integration**: Unknown how V and L combine to produce A
4. **Embodiment brittleness**: Policy tied to specific robot morphology

### 3.2 The World Model Advantage

PID-Splat enables **world model based robotics** by:

**1. Extracting implicit world models:**
- D (hidden states before action head) represents the VLA's "internal simulation"
- PID(V, D; Flow) tests if D predicts physically valid 3D trajectories

**2. Diagnosing integration quality:**
```
I(V,L;A) = Red(V,L;A) + Unq(V) + Unq(L) + Syn(V,L;A)
```
- **High Syn**: Model requires both modalities (good integration)
- **Negative Syn**: Subadditive, potential hallucination
- **High Unq(L) in visual task**: Overreliance on language (brittleness)

**3. Embodiment-agnostic evaluation:**
- 3D Flow is robot-independent
- Compare PID(V, D; Flow) across Franka vs UR5 vs mobile manipulator
- Same world model understanding → different action decoders

**4. Compositional verification:**
- Long-horizon tasks: Does synergy degrade over time? (H5)
- If yes → world model loses coherence over long plans

### 3.3 Why Gaussian Splats + Modular Physics Enable This

| Traditional Sim | PID-Splat |
|-----------------|-----------|
| Synthetic renders → domain gap → VLA sees different inputs than real | Splat captures → photorealism → VLA sees real-like inputs |
| MuJoCo physics → slow, non-Rust → IPC overhead | Modular physics → Rust-native (Rapier) or FFI (MuJoCo) |
| Offline evaluation → no real-time feedback | Tauri dashboard → live PID overlay on running policy |

**The key insight**: To diagnose VLAs, we need:
1. **Photorealistic inputs** (so VLA behavior matches real-world)
2. **Fast physics** (for thousands of evaluation episodes)
3. **Real-time analysis** (live PID computation during rollouts)
4. **Differentiable rendering** (for future gradient-based probing)

Gaussian Splats + Modular Physics + Tauri provide all four.

---

## 4. Component Summary

| Component | Role | Why Superior |
|-----------|------|--------------|
| **Tauri** | Desktop app shell | Native Rust perf + cross-platform |
| **SparkJS** | 3DGS rendering | WebGPU photorealism + PID overlays |
| **Physics** | Object physics | Modular (Rapier/MuJoCo/Isaac) |
| **Robot Sim** | Robot dynamics | Industry-standard (Gazebo/MuJoCo) |
| **3DGS Pipeline** | Scene capture | Real2Sim photorealism, differentiable |
| **Dream2Flow** | World model probe | Euclidean flow target, embodiment-agnostic |
| **PID-Core** | Information analysis | Decomposes V-L-A integration, diagnoses failures |

---

## 5. Research Trajectory

| Phase | Goal | Key Deliverable |
|-------|------|-----------------|
| **1** | Validate PID estimators (Experiment 0) | GO/NO-GO gate passed |
| **2** | Apply to OpenVLA on LIBERO | Failure signature taxonomy |
| **3** | Compare DreamVLA vs OpenVLA | World model quality metrics |
| **4** | Embodiment transfer via Flow-as-bridge | Cross-robot PID analysis |

**Ultimate goal**: Move from "does this VLA work?" to "how does this VLA understand the world?" — enabling principled debugging and improvement of vision-language-action models.

---

## 6. Hardware Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| Apple Silicon | M2 Pro (16GB) | M4 Max (64GB) |
| NVIDIA GPU | RTX 3080 (10GB) | A100 (40GB) for WAN |
| RAM | 32GB | 64GB |
| Storage | 500GB SSD | 2TB NVMe |

**Latency Budget (Real-time loop, no WAN):**
```
Capture image          :    0ms (start)
VLA inference          : 2000ms
Action execution       :   10ms
Physics step           :    1ms
PID computation        :   50ms
Render                 :   16ms
─────────────────────────────────
Total                  : ~2100ms per action
Effective control Hz   : ~0.5 Hz (quasi-static tasks)
```


---

## 7. World Model Comparison: ManiGaussian vs PEGS

The architecture supports a head-to-head comparison between two dominant world model paradigms for 3DGS-based robotics.

### 7.1 ManiGaussian (Learned Implicit Physics)
- **Paradigm:** End-to-end differentiable world model.
- **Mechanism:** Uses a 3D Variational Encoder to map 3DGS scenes to a latent space $Z$. Future states are predicted as $Z_{t+1} = f(Z_t, A_t)$.
- **PID Role:** Used to measure the fidelity of the latent world model: $Syn(V, L; Z)$.
- **Strength:** Captures complex, hard-to-model dynamics directly from data.
- **Weakness:** Poor generalization to novel objects (requires retraining).

### 7.2 PEGS (Explicit Particle-Based Physics)
- **Paradigm:** Hybrid explicit simulation with visual correction.
- **Mechanism:** Binds Gaussians to a PBD (Position-Based Dynamics) particle system. Real-time "visual forces" nudge particles to match photometric observations.
- **PID Role:** Used to measure the benefit of visual correction: $Syn(P_{pred}, V_{obs}; P_{corr})$.
- **Strength:** High generalization (physics laws don't change); handles deformables natively.
- **Weakness:** Requires accurate manual proxy/mesh definitions for novel objects.

---

---

## 8. SmolVLA (LeRobot) Integration

SmolVLA (Jan 2025) is integrated as the primary lightweight baseline.

### 8.1 Architecture
- **Backbone:** SmolVLM-2 (SigLIP visual encoder + SmolLM2 language model).
- **Action Head:** **Flow-Matching Transformer** (continuous action generation).
- **Inference:** Asynchronous execution (perception/planning decoupled from control).
- **Parameters:** ~450M (15x smaller than OpenVLA).

### 8.2 Architectural Role in PID-VLA
- **Iteration Speed:** Enables rapid testing of the PID pipeline due to fast inference (~100ms on M4 Max vs ~2s for OpenVLA).
- **Control Rate:** Higher control frequency (~10Hz) due to async inference.
- **Fine-tuning:** Seamless integration with Hugging Face LeRobot datasets (SO-100, LIBERO).
