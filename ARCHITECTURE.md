# PID-Splat Architecture: Components & Comparative Advantages

> **Documentation Cross-Reference**:
> - `grandplan.md` — Master plan and theoretical foundations
> - `pidsplatspecs.md` — Detailed simulation environment specifications
> - `EXPERIMENTS.md` — Experimental protocols for Rerun-first diagnostics, modular physics, and hypothesis testing
> - `DIAGRAMS.md` — Visual architecture diagrams
> - `README.md` — Quick start guide
> - `GAUSS_MI_INTEGRATION.md` — Optional 3DGS uncertainty + view selection (spec)
> - `WORLD_WARP_INTEGRATION.md` — Optional external world‑model baseline (spec)

---

**Docset alignment:** This document is aligned to `grandplan.md` v10.4. It describes a *target architecture* (PID‑Splat) that evolves from a "Rerun-First" research prototype (Phases 1–3) to a specialized interactive application (Phase 4+).

**Docset-wide final solution:** `grandplan.md` §A.8 is the decision record. The run log is the source of truth; Rerun is the Phases 1–3 diagnostic/time-machine viewer; Tauri/SparkJS is the Phase 4 shell for controls, editors, and custom rendering; all clients share the Agent Bridge.

## 1. Core System Components

### 1.1 Strategy: "Rerun-First" (Phases 1–3)

**What it is:**
Instead of building a custom simulator frontend from scratch immediately, we utilize **Rerun** (https://rerun.io/) as the primary visualization, logging, and "time machine" engine for the initial research phases.

**Why this decision (v10.1):**
- **Engineering Efficiency:** Building a custom 3D engine with timeline scrubbing, camera controls, and state management (Tauri+SparkJS) is a massive upfront cost. Rerun provides these "for free" via a simple SDK (`cargo add rerun`).
- **The "Time Machine":** Rerun's core feature is recording streams of data and allowing instant replay/scrubbing. This is critical for diagnosing VLA failures (e.g., "rewind to 2 seconds before the drop").
- **3DGS Support:** Rerun supports logging Gaussian Splats directly.
- **Focus on Science:** Allows the team to focus on `pid-core`, physics bindings, and experiment logic (the novel parts) rather than boilerplate UI code.

**Implementation Detail: Ghost Splats in Rerun**
SparkJS (Phase 4) allows custom shaders for "Ghost Splats" (predictive flow overlays). Rerun does not support custom shaders.
*   **Workaround:** We log **two** separate entities:
    1.  `world/reality`: The captured scene splats (standard rendering).
    2.  `world/ghost`: A separate, lower-density point cloud or splat-set representing the predicted flow, colored Red/Blue based on PID values.
*   **Trade-off:** Slightly higher bandwidth/storage than a shader-based approach, but vastly simpler to implement.

### 1.2 Tauri Application + SparkJS (Phase 4 Target)

**What it does:**
- A specialized, cross-platform desktop application shell (Tauri v2)
- Hosts the SparkJS renderer (Three.js/WebGL2) for custom shader-based visual effects
- Provides a dedicated "Agent Bridge" control panel for human-in-the-loop experiments

**Status:**
- **Deferred to Phase 4.** This is the "Productization" target for when the research is validated and we need highly specific interactive tools (e.g., real-time "visual force" editing) that Rerun cannot support.
- **Not a replacement for the run log or Rerun gate workflow.** Tauri should first launch/open Rerun recordings, then optionally embed the Rerun WebViewer, and only later add SparkJS panels for custom shaders or direct manipulation. All edits still go through the Agent Bridge and become run-log events.

**Stack (Phase 4):**
```
┌─────────────────────────────────────────────────────────┐
│                    Tauri v2 Shell                       │
├─────────────────────────────────────────────────────────┤
│  Frontend (React + Three.js)│  Backend (Rust)          │
│  ├─ SparkJS 3DGS Renderer   │  ├─ PID-Core estimators  │
│  ├─ PID Heatmap Shaders     │  ├─ Rerun SDK (optional) │
│  ├─ Control Panel           │  ├─ Agent Bridge (JSON-RPC)     │
│  └─ Timeline + replay UI    │  └─ ML inference hooks   │
└─────────────────────────────────────────────────────────┘
```

### 1.3 Modular Physics Engine (Rapier, MuJoCo, Isaac Gym)

**Target role (planned physics adapters):**
- Provide rigid body physics simulation via a pluggable backend system
- Support **Rapier3D** (Rust-native default target), **MuJoCo** (industry standard), and **Isaac Gym** (GPU-parallel)
- Handle collision detection, joint constraints, friction, restitution
- Rapier can run at low step times for small scenes; achievable control/step rates are hardware- and scene-dependent.

**Target backend sketch (a `PhysicsBackend` trait with a null adapter and a **real `rapier3d-f64` backend** — gravity/contacts/friction, deterministic — exists in `crates/pid-sim` behind the optional `rapier` feature as of 2026-06-13, with a scripted push-to-goal manipulation emitting real `Flow_gt` + physics-derived labels; box-collider geometry only — mesh-collider ingestion and MuJoCo/Isaac adapters remain planned):**
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
- **Determinism:** Rapier aims for deterministic replay under fixed dt/ordering, but bitwise determinism can break across platforms/CPUs; verify and log settings/seeds.
- **Modularity:** Select an engine appropriate to your trade-offs (Rapier for speed, MuJoCo for contact fidelity)
- **Integration:** Native Rust (Rapier) = zero-copy data flow to PID-core; FFI for MuJoCo/Isaac
- **Multi-engine reality**: per-object “Rapier walls + MuJoCo cups” is a co-simulation problem for contact-rich scenes. v10.1 carries forward the recommendation to use **one physics backend per run**, plus optional **cross-backend replay** (Rapier ↔ MuJoCo) as a robustness/confound check (see `grandplan.md` §E.1).

### 1.4 Gazebo Harmonic (Robot Simulation)

**Target role (planned robot/sensor adapter):**
- Industry-standard robot simulation (URDF/SDF support)
- Sensor simulation (RGB-D cameras, joint encoders, force/torque)
- Headless mode for batch experiments

**Integration architecture (Rerun-First):**
```
┌─────────────────┐    Zenoh    ┌─────────────────────┐
│ Gazebo Harmonic │◄──────────►│  Rust Backend       │
│ (Headless)      │            │  ├─ PID-Core        │
│ ├─ Robot URDF   │            │  ├─ Controls        │
│ ├─ Sensors      │            │  └─ Rerun SDK       │
│ └─ ros_gz_bridge│            └──────────┬──────────┘
└─────────────────┘                       │
                                          ▼
                                   ┌──────────────┐
                                   │ Rerun Viewer │
                                   └──────────────┘
```

**Robot vs object simulation (coupling constraints)**

| Component | Use Case | When to Use |
|-----------|----------|-------------|
| **Physics Engine** | Object manipulation physics (fast, deterministic) | Object-object interactions, perturbations, fast iteration |
| **Robot Sim** | Robot kinematics/dynamics, sensor simulation | Robot URDF loading, sensor data, cross-embodiment |

**Important coupling rule:** if the robot and manipulated objects are simulated in different engines, robot–object contacts are **not physically meaningful** unless you implement an explicit coupling layer (co-simulation). For most prisoma experiments, prefer one of:
- **Single-engine contact (recommended for manipulation):** simulate robot + objects together in **MuJoCo** (benchmark-aligned) or another single backend, and use PID‑Splat only for logging/overlays.
- **Harness bring-up (recommended for early engineering):** use the in-repo deterministic object sim for run-log/Agent Bridge/Rerun plumbing first, then add object-only Rapier or MuJoCo physics and a kinematic “end-effector proxy” for interventions/perturbations; add full robot dynamics later (see `grandplan.md` §A.7).
- **Advanced (optional):** multi-engine “physics islands” with restricted coupling; static colliders can be duplicated, but cross-island contacts require one solver (see `grandplan.md` §E.1).

**Per-Hypothesis Engine Usage** (see `EXPERIMENTS.md` for full details):

| Hypothesis | Physics | Robot | Notes |
|------------|--------|--------|-------|
| H1 (PID features ↔ failure labels) | ✓ | ✓ | Object poses + robot state; synergy sign is a candidate feature, not a definition |
| H4 (Memorization vs generalization) | ✓ | | Mass/friction perturbations |
| H5 (Temporal degradation) | ✓ | ✓ | Long-horizon contact physics |
| H6 (Safety-aware V-L integration) | ✓ | ✓ | Collision detection for safety |
| H7 (Flow-as-bridge) | | | Flow from an external video predictor; no physics sim needed for flow extraction itself |

### 1.5 Gaussian Splatting (3DGS) Pipeline

**What it does:**
- Captures real-world scenes/objects via photogrammetry (iPhone + Polycam)
- Trains neural radiance representation (Nerfstudio splatfacto)
- Exports compressed `.spz` files for real-time rendering

**Pipeline:**
```bash
# 1. Capture (phone/DSLR video; e.g., Polycam; capture protocol is dataset- and scene-dependent)
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

# 4. Load in Rerun (Phases 1-3) or SparkJS (Phase 4)
```

**Asset specifications:**

| Object | Gaussian Count (illustrative; scene/export dependent) | Physics Proxy |
|--------|----------------|---------------|
| red_cube | ~15,000 | Cuboid |
| blue_cylinder | ~20,000 | Cylinder |
| ycb_mustard | ~40,000 | Convex Hull |
| tabletop_scene | ~800,000 | Static mesh |

**Why it matters:**
- **Real2Sim photorealism**: Captured splats look like real images (can reduce synthetic domain gaps; benchmark-dependent)
- **Object-centric assets**: Each manipulated object is a separate splat (compositional scenes)
- **Differentiability caveat**: 3DGS is differentiable in *training* frameworks; the visualization target here is not a differentiable training primitive.
 
**Caveat:** “Domain gap” and “photorealism” are benchmark-dependent; treat any sim2real claims as empirical until measured.

### 1.6 Dream2Flow Integration (World Model Bridge)

**What it does:**
- Uses a video generation model to "dream" plausible future trajectories (model choice is external)
- Segmentation + point tracking + depth estimation extract 3D object flow from dreamed videos
- 3D Flow becomes a **target variable** for PID analysis

> **Why Flow-as-Bridge is Critical**: The validated ISX estimator (`EhrlichKsg`) only supports Chebyshev (L∞) geometry and does **not** currently have a derivation for hyperbolic/Lorentzian manifolds. If high‑D embeddings exhibit tree‑like or curved geometry, shifting the diagnostic target to explicit 3D object flow can avoid non‑Euclidean metric issues; you still must validate dimensionality/distance concentration (flow is Euclidean but can be high‑D as \(\mathbb{R}^{3T}\)).

**Pipeline:**
```
Current Image + Instruction
         │
         ▼
    ┌─────────┐
    │ Video   │ (T frames @ fps; configurable; log frames/fps/seed)
    │  model  │
    └────┬────┘
         │
         ▼
    ┌─────────┐
    │ Segm.   │ (Segment objects in frame 0)
    └────┬────┘
         │
         ▼
  ┌───────────┐
  │ Tracker   │ (Track 2D points through video)
  └─────┬─────┘
         │
         ▼
┌─────────────────┐
│ Depth model     │ (Estimate per-frame depth)
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

### 2.1 Simulator Capability Notes (Not a Ranking)

Use this table as a qualitative capability map. Do not compare “sim2real %”, fps, or latency across platforms unless you run a matched benchmark + hardware + protocol.

| Simulator | Rendering | Physics | Availability / constraints | Notes for prisoma |
|-----------|-----------|---------|----------------------------|------------------|
| **MuJoCo / robosuite** | Raster (OpenGL) | MuJoCo | Cross-platform | Strong contact baseline; visuals are not photoreal by default |
| **PyBullet** | Raster (OpenGL) | Bullet | Cross-platform | Widely used but not state-of-the-art for contacts/visuals |
| **Isaac Sim/Lab** | RTX / OpenUSD | PhysX | NVIDIA GPU required | Strong USD tooling; heavy stack; PID harness is custom |
| **Isaac Gym** | (Varies) | GPU physics | NVIDIA GPU required | Good for scale; visuals depend on assets/renderer |
| **Gazebo Harmonic** | Raster (OGRE2) | Plugin-dependent | Cross-platform; ROS-centric | Strong robot/sensor ecosystem; PID harness is custom |
| **LuckyRobots / Lucky World** | “Hyperrealistic” (vendor claim); Vulkan renderer | **MuJoCo**-based (per README) | Proprietary world executable + Python API (**gRPC**, port 50051; WebSocket only inter-node); v0.1 lists Windows/Linux/macOS all “coming soon” (no public binary yet) | RL-style interface surface (`reset`/`step`, observations); PID harness still custom |
| **Habitat** | Mesh + neural | Limited (navigation focus) | Cross-platform | Good for nav; not a manipulation physics stack |
| **CARLA** | Unreal | Vehicle focus | Cross-platform | Driving-focused; not a manipulation stack |
| **Rapier3D** | Headless / debug | Rapier | Cross-platform | Fast iteration; contact fidelity depends on task and tuning |
| **3DGS (Gaussian splats)** | Photoreal views (capture-dependent) | N/A | Requires separate physics | Useful to reduce *visual* gaps when capture quality is good; does not replace physics |

### 2.2 Why Each Simulator Falls Short for VLA Diagnostics

| Simulator | Limitation for prisoma |
|-----------|------------------------|
| **MuJoCo/robosuite** | Synthetic visuals can create sim2real gaps for vision-heavy policies unless carefully randomized/photorealistic |
| **PyBullet** | Outdated rendering, poor visual fidelity |
| **Isaac Gym** | NVIDIA-only; rendering fidelity depends on the chosen renderer and assets |
| **Habitat** | Navigation-only, no manipulation |
| **CARLA** | Driving-only, no manipulation |
| **Gazebo** | Strong robot middleware support; visuals/physics trade-offs depend on plugins and assets; batch scaling may require additional tooling |

### 2.3 The PID-Splat Solution: Composable Backends

```
┌─────────────────────────────────────────────────────────────┐
│                    PID-Splat Architecture                   │
├─────────────────────────────────────────────────────────────┤
│  RENDERING        │  PHYSICS           │  ROBOT SIMULATION  │
│  ────────         │  ───────           │  ────────────────  │
│  Rerun (P1-3) /   │  Modular Backend   │  Gazebo (accurate) │
│  SparkJS (P4)     │  (Rapier, MuJoCo,  │  OR                │
│  (photorealistic) │   Isaac Gym)       │  MuJoCo (legacy)   │
└─────────────────────────────────────────────────────────────┘
```

**Key Insight**: Decouple rendering from physics. Use Gaussian splats for visuals (can reduce *visual* gaps when capture is good; benchmark-dependent) + a pluggable physics backend for dynamics/contact.

### 2.4 Modular Physics Backend Selection

Users can select physics backend based on their needs:

| Use Case | Recommended Backend | Why |
|----------|---------------------|-----|
| **Fast iteration / prototyping** | Rapier3D | Low-latency, Rust-native |
| **Accurate contact physics** | MuJoCo | Strong contact modeling baseline for manipulation |
| **GPU-parallel batch experiments** | Isaac Gym | Large parallel batches (GPU; hardware-dependent) |
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
backend = "rerun"  # Default: Rerun for P1-3, "spark" for P4

[robot]
backend = "none"    # Options: "gazebo", "mujoco", "none" (default for early bring-up)
urdf_path = "assets/robots/franka_panda.urdf"
```

### 2.5 Camera & Environment Simulation

| Feature | MuJoCo/robosuite | Isaac Gym | Rerun (P1-3) |
|---------|------------------|-----------|------------------|
| **Multi-view cameras** | ✓ Fixed viewpoints | ✓ Any viewpoint | ✓ Any viewpoint (Scrubbing) |
| **Lighting changes** | Re-render needed | Re-render needed | **N/A (Recorded)** |
| **Camera intrinsics** | Manual setup | Manual setup | **Logged from capture** |
| **Motion blur** | Not supported | Limited | **N/A** |
| **Lens distortion** | Not supported | Limited | **N/A** |
| **New environments** | Longer asset-authoring cycles | Longer asset-authoring cycles | Potentially faster capture/reconstruction (depends on setup/tooling) |

---

## 3. Advantages Over Existing VLM-Based Robotics

### 3.1 Comparison with OpenVLA / PixelVLA / TraceVLA

| Aspect | OpenVLA et al. | PID-Splat |
|--------|----------------|-----------|
| **Simulation** | MuJoCo/PyBullet (mesh-based) | Gaussian Splats + Rapier (photorealistic + fast) |
| **Visual Fidelity** | Synthetic renders can introduce domain gaps | Real-captured splats can reduce visual domain gaps (benchmark-dependent) |
| **Analysis** | Task success rate only | PID decomposition reveals *why* success/failure |
| **World Model** | Implicit in LLM hidden states | Explicit 3D flow extraction for validation |
| **Embodiment Transfer** | Per-robot fine-tuning | Flow-as-bridge tests embodiment-agnostic understanding |

**What PID adds (hypothesis; validate empirically):** typical benchmarks emphasize task success and sometimes auxiliary diagnostics; PID offers an additional, information-theoretic decomposition that *may* help localize which inputs drive decisions:
- **Grounding failure signatures:** candidate correlations between PID atoms and failures (H1/H2; see `grandplan.md` warnings + Experiment 0 gate)
- **Memorization vs generalization:** test whether PID patterns differ across held-out compositions (H4)
- **Long-horizon composition:** test whether temporal PID summaries degrade before failure (H5)

### 3.2 Comparison with VLA-Arena

| Aspect | VLA-Arena | PID-Splat |
|--------|----------|-----------|
| **Focus** | Benchmark suite (what) | Diagnostic framework (why) |
| **Rendering** | Standard simulators | 3DGS photorealism |
| **Metrics** | Task completion, collision rate | Information-theoretic decomposition |
| **Scalability** | N models × M tasks = N×M runs | Single analysis reveals modality contributions |

**Complementary positioning:** VLA-Arena provides standardized evaluation; PID-based analyses aim to add diagnostic signals on top of those evaluations (not replace them).

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
| **World Model** | Explicit world-knowledge forecasting (dynamic/spatial/semantic cues; verify) | D can be internal (hidden states) or external; Flow is used as a diagnostic intermediate |
| **Analysis** | Task performance | PID(V, D; Flow) reveals world model quality |
| **Architecture** | Fixed training objective | Post-hoc analysis, any VLA |

**Key advantage**: DreamVLA trains with dreaming; PID-Splat *analyzes* any VLA's implicit world model without retraining.

### 3.5 Attribution Methods as Diagnostic Overlays

LRP, Integrated Gradients, DeepLIFT, Grad-CAM, TCAV, saliency/SmoothGrad, occlusion/permutation, and SHAP-style attributions are complementary to PID rather than substitutes for it. PID summarizes information relationships across logged samples; attribution localizes which features, tokens, regions, layers, or concepts influenced a selected output for a model call.

**Rerun-first integration:**
- Log precomputed attribution artifacts as images, heatmaps, token bars, point/patch colors, or scalar time-series tracks alongside PID/CI metrics.
- Keep attribution metadata with the artifact: method, target output, layer/modality, baseline/background/concept set, preprocessing, score hash, and faithfulness/sanity-check result.
- Do not require Phase 4 custom shaders for attribution review; Phase 4 can add interactive overlays, but the canonical evidence remains the run log plus artifacts.

**Interpretation rule:** if PID claims `Unq(V)`, `Unq(L)`, or `Syn(V,L;A)` is diagnostic, attribution overlays should either provide a compatible local account under matched interventions or expose a disagreement that must be reported.

---

## 3. World Model vs VLM-Based Robotics: The Core Argument

### 3.1 The Problem with Pure VLM-Based Robotics

Current VLAs (OpenVLA, PixelVLA) are essentially:
```
Image + Instruction → LLM → Action Tokens
```

**Failure modes:**
1. **Implicit state:** policies often do not expose an explicit physical state representation (“world model”) as a first-class variable
2. **Grounding risk:** models can produce confident actions that are physically infeasible or misaligned with the scene
3. **Opaque integration:** it is often hard to attribute failures to V vs L vs internal state without targeted probes/interventions
4. **Embodiment coupling:** policies may overfit to specific camera viewpoints, control frequencies, or embodiments

### 3.2 The World Model Advantage

PID-Splat enables **world model based robotics** by:

**1. Extracting implicit world models:**
- D (hidden states before action head) represents the VLA's "internal simulation"
- PID(V, D; Flow) tests if D predicts physically valid 3D trajectories

**2. Diagnosing integration quality:**
```
I(V,L;A) = Red(V,L;A) + Unq(V) + Unq(L) + Syn(V,L;A)
```
- **High Syn**: information about `A` is present only in the joint `(V,L)` beyond either alone (interpretation is task-dependent; validate under controls)
- **Negative Syn**: allowed under `I^sx_∩`; treat as a candidate diagnostic feature and rule out estimator/geometry artifacts via Experiment 0 + perturbation controls
- **High Unq(L) in a visually dominated task**: can indicate language reliance; test with instruction perturbations and placebo controls

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
| Offline evaluation → no real-time feedback | **Rerun Time Machine** → scrub through failure cases instanty |

**The key insight**: To diagnose VLAs, we need:
1. **Photorealistic inputs** (so VLA behavior matches real-world)
2. **Fast physics** (for thousands of evaluation episodes)
3. **Real-time analysis** (live PID computation during rollouts)
4. **Reproducible instrumentation** (fixed preprocessing, validated estimators, and controlled interventions)

Gaussian splats + modular physics + a unified UI (Rerun for P1-3) are intended to support these goals; treat them as hypotheses until benchmarked.

---

## 4. Component Summary

| Component | Role | Rationale (design goals; benchmark-dependent) |
|-----------|------|--------------|
| **Run log** | Canonical data spine | Source of truth for replay, analysis, Rerun export, and Tauri sessions; summaries distinguish unique metric names from total metric events. |
| **Agent Bridge** | Shared control plane | GUI, scripts, and LLM tools call the same local API; every action is logged. |
| **Rerun** | **Visualization & diagnostics** | **Primary P1-3 Tool.** Timeline, 3D scene, plots, ghost overlays, and replay from run logs. |

Current deterministic bridge smokes expose stdio/TCP/WebSocket JSON-RPC methods for status, deterministic stepping, deterministic interventions, replay, run lifecycle stop, and `export.rerun`; safe mode keeps status/replay read-only and rejects mutation, run-ending, or file-writing exports.
| **Tauri+SparkJS** | Interactive App | **Deferred to P4.** For custom shaders, collider/edit tools, and complex intervention UI; never the canonical store. |
| **Physics** | Object physics | Modular (Rapier/MuJoCo/Isaac) |
| **Robot Sim** | Robot dynamics | Industry-standard (Gazebo/MuJoCo) |
| **3DGS Pipeline** | Scene capture | Photorealistic captures; differentiable training pipelines exist (visualization here is non-differentiable) |
| **Dream2Flow** | World model probe | Euclidean flow target, embodiment-agnostic |
| **PID-Core** | Information analysis | Decomposes V-L-A integration, diagnoses failures |
| **Attribution probes** | Local explanation baselines | LRP/IG/DeepLIFT/Grad-CAM/TCAV/saliency/SHAP-style artifacts triangulate PID claims when sanity checks pass |

---

## 5. Research Trajectory

| Phase | Goal | Key Deliverable | Visualization |
|-------|------|-----------------|---------------|
| **1** | Validate PID estimators (Experiment 0) | Exp0 gate status recorded (GO/PIVOT/NO‑GO) | Rerun (Charts) |
| **2** | Apply to OpenVLA on LIBERO | Failure signature taxonomy | Rerun (Timeline + Logs) |
| **3** | Compare DreamVLA vs OpenVLA | World model quality metrics | Rerun (3DGS + Ghost Splats) |
| **4** | Embodiment transfer via Flow-as-bridge | Cross-robot PID analysis | **Tauri + SparkJS** (Interactive) |

**Ultimate goal**: Move from "does this VLA work?" to "how does this VLA understand the world?" — enabling principled debugging and improvement of vision-language-action models.

---

## 6. Hardware Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| Apple Silicon | Any supported dev machine | More RAM helps for large datasets |
| NVIDIA GPU | Any CUDA GPU (if running models locally) | High-VRAM GPU if running heavy video/world models locally (otherwise use remote service) |
| RAM | 32GB | 64GB |
| Storage | 500GB SSD | 2TB NVMe |

**Latency note:** Any ms-level budget is hardware/model dependent; treat numbers as estimates until measured. For rigorous reporting, benchmark each component and report measured ranges (see `EXPERIMENTS.md` §12).


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

SmolVLA (LeRobot) is a candidate lightweight baseline (planned integration; verify model availability/APIs).

### 8.1 Architecture
- **Backbone:** Lightweight VLM baseline (LeRobot; verify exact architecture/backbone).
- **Action head / representation:** Implementation-specific; verify (continuous delta actions vs discretized tokens/bins).
- **Inference:** May support async pipelines (verify and measure on your stack).

### 8.2 Architectural Role in prisoma
- **Iteration Speed:** Smaller models can make the PID pipeline easier to iterate on (measure inference latency on your hardware).
- **Control Rate:** Async inference can raise effective control rates (benchmark; depends on policy and environment).
- **Fine-tuning:** Seamless integration with Hugging Face LeRobot datasets (SO-100, LIBERO).

---

## 9. InternVLA‑A1 (Optional) Integration

InternVLA‑A1 is a candidate **diffusion / flow-matching** VLA for stage-wise ablations because it explicitly separates “understanding”, “generation”, and “action” experts (verify details and interfaces from its paper/repo before use).

### 9.1 Architectural Role in prisoma (Docset v10.1)
- **Hierarchical PID inside one model:** treat generation-expert outputs as `D_gen` (a candidate `D_explicit`) and test `(V,L;D_gen)` and `(V,D_gen;A)` under the same data/logging contract as other VLAs.
- **Flow comparisons:** if `D_gen` yields predicted frames/latents, derive a model-side `Flow_pred` and compare to simulator-derived `Flow_gt` under matched controls (do not conflate “Flow Matching” used to generate actions with this project’s geometric `Flow_*` variables).
- **License caution:** verify the upstream license/model card before depending on it (may be restrictive); avoid vendoring code into this MIT-licensed repo unless license compatibility is confirmed.

### 9.2 Integration Notes (Verify)
- The repo describes patched HuggingFace Transformers modules; isolate integration in a separate service/environment and log the exact revision.
- Confirm how to export intermediates (`D_gen`) and the exact action parameterization (“delta actions”, etc.) before quantitative comparisons.
