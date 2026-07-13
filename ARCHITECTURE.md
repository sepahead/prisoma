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

**Docset alignment:** This document is aligned to `grandplan.md` docset v12.5 (seventh adversarial revision; scientific cut 2026-07-12). It describes a *target architecture* (PID‑Splat) that evolves from a "Rerun-First" research prototype (Phases 1–3) to a specialized interactive application (Phase 4+).

**v10.7 → v12.5 migration note:** the legacy H1–H9 / Exp0–Exp10 scheme is retired. The confirmatory registry (`grandplan.md` §4) is now **EC1** (provenance-complete replay) plus **H1** (pre-treatment diagnostics predict intervention response; Protocol A paired vs Protocol B randomized), **H2** (censoring-aware failure prediction), **H3** (conditional PID incremental value), and **H4** (availability can diverge from causal policy use). Gates are the **S0–S7** sequence (§5.1); build order is **milestones M0–M7** (§12); estimator validation ("Experiment 0") is now the **S1 gate / §7** and is judged against four gates — population, measure, estimator, application (§7.1). Legacy H-labels below are remapped accordingly.

**Docset-wide final solution:** `grandplan.md` §16 is the decision log (see also §8.2, §8.11, §8.13, §15.4). The run log is the source of truth; the Agent Bridge is the **only control plane**; Rerun is the read-only Phases 1–3 diagnostic/time-machine viewer; and Tauri/SparkJS is the Phase 4 shell for controls, editors, and custom rendering. Every VLA action, scene edit, intervention, pause/resume/step transition, and correction-force command must enter through the Agent Bridge and be appended to the canonical run log before execution. PID workers and observers analyze data; Zenoh transports data; Rerun renders replayed data. None may actuate the simulator.

## 1. Core System Components

### 1.1 Strategy: "Rerun-First" (Phases 1–3)

**What it is:**
Instead of building a custom simulator frontend from scratch immediately, we utilize **Rerun** (https://rerun.io/) as the primary visualization and "time machine" viewer for the initial research phases. The canonical run log—not a Rerun recording—is the authoritative record.

**Why this decision (v10.1):**
- **Engineering Efficiency:** Building a custom 3D engine with timeline scrubbing, camera controls, and state management (Tauri+SparkJS) is a massive upfront cost. Rerun provides these "for free" via a simple SDK (`cargo add rerun`).
- **The "Time Machine":** Rerun can display converted run-log streams with replay/scrubbing. This is critical for diagnosing VLA failures (e.g., "rewind to 2 seconds before the drop") while keeping the viewer read-only.
- **3DGS Support:** splat data can be logged to Rerun (point clouds/ellipsoids); verify native 3DGS rendering in the pinned Rerun 0.28.x before relying on it.
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
- **Multi-engine reality**: per-object “Rapier walls + MuJoCo cups” is a co-simulation problem for contact-rich scenes. The recommendation is to use **one physics backend per run**, plus optional **cross-backend replay** (Rapier ↔ MuJoCo) as a robustness/confound check (see `grandplan.md` §8.5 replay levels; §6.10 robustness/falsification).

### 1.4 Gazebo Harmonic (Robot Simulation)

**Target role (planned robot/sensor adapter):**
- Industry-standard robot simulation (URDF/SDF support)
- Sensor simulation (RGB-D cameras, joint encoders, force/torque)
- Headless mode for batch experiments

**Integration architecture (Rerun-First):**
```
┌──────────────────┐   command   ┌────────────────┐   append first   ┌───────────────────┐
│ GUI / VLA / tool │────────────▶│  Agent Bridge  │────────────────▶│ Canonical run log │
└──────────────────┘             └───────┬────────┘                 └─────────┬─────────┘
                                         │ dispatch recorded command          │ replay
                                         ▼                                    ▼
                              ┌─────────────────────┐              ┌───────────────────┐
                              │ Gazebo / backend    │              │ Rerun adapter/view │
                              │ robot + sensors     │              │ (read-only)       │
                              └──────────┬──────────┘              └───────────────────┘
                                         └── observations/events ──▶ run log

                     Zenoh, when enabled, mirrors data only; it is not a control path.
```

**Robot vs object simulation (coupling constraints)**

| Component | Use Case | When to Use |
|-----------|----------|-------------|
| **Physics Engine** | Object manipulation physics (fast, deterministic) | Object-object interactions, perturbations, fast iteration |
| **Robot Sim** | Robot kinematics/dynamics, sensor simulation | Robot URDF loading, sensor data, cross-embodiment |

**Important coupling rule:** if the robot and manipulated objects are simulated in different engines, robot–object contacts are **not physically meaningful** unless you implement an explicit coupling layer (co-simulation). For most prisoma experiments, prefer one of:
- **Single-engine contact (recommended for manipulation):** simulate robot + objects together in **MuJoCo** (benchmark-aligned) or another single backend, and use PID‑Splat only for logging/overlays.
- **Harness bring-up (recommended for early engineering):** use the in-repo deterministic object sim for run-log/Agent Bridge/Rerun plumbing first, then add object-only Rapier or MuJoCo physics and a kinematic “end-effector proxy” for interventions/perturbations; add full robot dynamics later (see `grandplan.md` §12 milestones).
- **Advanced (optional):** multi-engine “physics islands” with restricted coupling; static colliders can be duplicated, but cross-island contacts require one solver (see `grandplan.md` §8.5).

**Per-claim engine usage** (see `EXPERIMENTS.md` for full details):

| Claim | Physics | Robot | Notes |
|------------|--------|--------|-------|
| H1 (pre-treatment diagnostics predict intervention response) | ✓ | ✓ | The deterministic synthetic Protocol A scoring reference is runnable, but real paired capture and the randomized Protocol B fork remain open (grandplan §6.3); synergy sign is a candidate feature, not a definition |
| H2 (censoring-aware prospective failure prediction) | ✓ | ✓ | Long-horizon contact physics; frozen alarm policy, lead time (grandplan §6.4) |
| H3 (conditional PID incremental value) | ✓ | | Only inside the validated support envelope (grandplan §7.14) |
| H4 (availability can diverge from causal policy use) | ✓ | | Mass/friction perturbations; availability-vs-use asymmetry reported as a finding |
| Flow-as-bridge (Exploratory — grandplan §9.6) | | | Flow from an external video predictor; no physics sim needed for flow extraction itself |
| Safety-aware V–L integration (Retired/deferred — grandplan §4) | ✓ | ✓ | Collision detection for safety; deferred until proper safety labels + matched controls exist |

### 1.5 Gaussian Splatting (3DGS) Pipeline

**What it does:**
- Captures real-world scenes/objects via photogrammetry (iPhone + Polycam)
- Trains neural radiance representation (Nerfstudio splatfacto)
- Exports Nerfstudio Gaussian splats as `.ply`; a separately selected and pinned converter may then produce `.spz` for a renderer that requires it

**Pipeline:**
```bash
# 1. Capture (phone/DSLR video; e.g., Polycam; capture protocol is dataset- and scene-dependent)
# 2. Train
ns-train splatfacto --data ./captures/scene/

# 3. Export PLY with Nerfstudio
ns-export gaussian-splat \
    --load-config <config> \
    --output-dir <dir>

# 4. Optional SPZ conversion is a separate tool step.
# Pin the converter executable/revision and record its exact command plus PLY/SPZ hashes.

# 5. Load in Rerun (Phases 1-3) or SparkJS (Phase 4)
```

Nerfstudio's `gaussian-splat` exporter is the PLY-producing step above. Do not pass an invented Gaussian-count training flag or an `--output-format spz` exporter flag. Treat any PLY→SPZ conversion as a distinct dependency with its own version, license, command, and provenance.

**Asset specifications:**

| Object class | Physics proxy example |
|--------------|-----------------------|
| box-like object | Cuboid |
| cylindrical object | Cylinder |
| irregular rigid object | Validated convex hull or mesh |
| tabletop/background scene | Static collision geometry separate from the splats |

Measure and log the actual Gaussian count, renderer memory/time, collision geometry, and export hashes per asset; there is no universal target count.

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

> **Why Flow-as-Bridge is Critical**: The implemented `EhrlichKsg` path only supports Chebyshev (L∞) geometry and does **not** currently have a derivation for hyperbolic/Lorentzian manifolds. Under the four PID gates (`grandplan.md` §7.1), the high-dimensional MI/coherence path is **NO-GO**, while continuous shared-exclusions atoms on real embeddings are **BLOCKED / not application-validated** (`grandplan.md` §7.2; `findings.md`). If high‑D embeddings exhibit problematic local geometry or distance concentration, shifting the diagnostic target to explicit 3D object flow can reduce the ambient-dimension burden; flow still needs estimator-recovery, intrinsic-dimension, concentration/tie, dependence, and local-flatness checks.

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

**Why it matters (flow-as-bridge; Exploratory — `grandplan.md` §9.6):**
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
| **Simulation** | MuJoCo/PyBullet (mesh-based) | Gaussian Splats + Rapier (photoreal capture + low-latency physics; benchmark-dependent) |
| **Visual Fidelity** | Synthetic renders can introduce domain gaps | Real-captured splats can reduce visual domain gaps (benchmark-dependent) |
| **Analysis** | Task success rate only | PID decomposition reveals *why* success/failure |
| **World Model** | Implicit in LLM hidden states | Explicit 3D flow extraction for validation |
| **Embodiment Transfer** | Per-robot fine-tuning | Flow-as-bridge tests embodiment-agnostic understanding |

**What PID adds (hypothesis; validate empirically):** typical benchmarks emphasize task success and sometimes auxiliary diagnostics; PID offers an additional, information-theoretic decomposition that *may* help localize which inputs drive decisions:
- **Failure signatures:** exploratory PID-atom correlations may become conditional H3 features for
  the prospective H2 endpoint only after all four PID gates; they are not the H1 Protocol A/B endpoint
- **Availability vs use:** test whether representational availability diverges from causal policy use across held-out compositions (H4)
- **Long-horizon composition:** test whether temporal PID summaries degrade before failure (exploratory temporal analysis)

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

### 3.4 DreamVLA as a Within-Model Stage-Analysis Candidate

| Stage-analysis question | Candidate variable / intervention |
|-------------------------|-----------------------------------|
| Are exposed world-knowledge channels informative about future motion? | Within the same DreamVLA checkpoint, test preregistered `D_explicit` channels against `Flow_gt` after the estimator gates pass |
| Does the policy causally use an exposed channel? | Ablate, shuffle, or replace that channel within the same model and compare logged action/outcome changes |
| Where does an end-to-end failure arise? | Keep the model, task, preprocessing, and checkpoint fixed while separating world-model, action-decoding, and physical-execution stages |

Do **not** treat a DreamVLA-versus-OpenVLA PID difference as a causal effect of "dreaming": their architectures, action heads, training data, and definitions of `D` differ. Cross-model results are descriptive replication only. The causal design is the within-model stage/channel intervention above.

### 3.5 Attribution Methods as Diagnostic Overlays

LRP, Integrated Gradients, DeepLIFT, Grad-CAM, TCAV, saliency/SmoothGrad, occlusion/permutation, and SHAP-style attributions are complementary to PID rather than substitutes for it. PID summarizes information relationships across logged samples; attribution localizes which features, tokens, regions, layers, or concepts influenced a selected output for a model call.

**Rerun-first integration:**
- Log precomputed attribution artifacts as images, heatmaps, token bars, point/patch colors, or scalar time-series tracks alongside PID/CI metrics.
- Keep attribution metadata with the artifact: method, target output, layer/modality, baseline/background/concept set, preprocessing, score hash, and faithfulness/sanity-check result.
- Do not require Phase 4 custom shaders for attribution review; Phase 4 can add interactive overlays, but the canonical evidence remains the run log plus artifacts.

**Implemented slice:** `experiments/attribution/` runs epsilon-/AttnLRP and gradient×input on a small reference model, checks deletion-AOPC against a random control, and emits first-class `attribution_logged` events. The `pid-rerun` adapter surfaces the faithfulness verdict, provenance text, and up to 1024 values from compatible NumPy relevance artifacts. Production VLA adapters and richer 2-D panels remain future work.

**Interpretation rule:** if PID claims `Unq(V)`, `Unq(L)`, or `Syn(V,L;A)` is diagnostic, attribution overlays should either provide a compatible local account under matched interventions or expose a disagreement that must be reported.

---

## 3A. World Model vs VLM-Based Robotics: The Core Argument

### 3A.1 The Problem with Pure VLM-Based Robotics

Current VLAs (OpenVLA, PixelVLA) are essentially:
```
Image + Instruction → LLM → Action Tokens
```

**Failure modes:**
1. **Implicit state:** policies often do not expose an explicit physical state representation (“world model”) as a first-class variable
2. **Grounding risk:** models can produce confident actions that are physically infeasible or misaligned with the scene
3. **Opaque integration:** it is often hard to attribute failures to V vs L vs internal state without targeted probes/interventions
4. **Embodiment coupling:** policies may overfit to specific camera viewpoints, control frequencies, or embodiments

### 3A.2 The World Model Advantage

PID-Splat enables **world model based robotics** by:

**1. Extracting implicit world models:**
- D (hidden states before action head) represents the VLA's "internal simulation"
- PID(V, D; Flow) tests if D predicts physically valid 3D trajectories

**2. Diagnosing integration quality:**
```
I(V,L;A) = Red(V,L;A) + Unq(V) + Unq(L) + Syn(V,L;A)
```
- **High Syn**: information about `A` is present only in the joint `(V,L)` beyond either alone (interpretation is task-dependent; validate under controls)
- **Negative Syn**: allowed under `I^sx_∩`; treat as a candidate diagnostic feature and rule out estimator/geometry artifacts via the S1 estimator/measure gate (`grandplan.md` §7) + perturbation controls
- **High Unq(L) in a visually dominated task**: can indicate language reliance; test with instruction perturbations and placebo controls

**3. Embodiment-agnostic evaluation:**
- 3D Flow is robot-independent
- Compare PID(V, D; Flow) across Franka vs UR5 vs mobile manipulator
- Same world model understanding → different action decoders

**4. Compositional verification:**
- Long-horizon tasks: Does synergy degrade over time? (exploratory temporal analysis)
- If yes → world model loses coherence over long plans

### 3A.3 Why Gaussian Splats + Modular Physics Enable This

| Traditional Sim | PID-Splat |
|-----------------|-----------|
| Synthetic renders → domain gap → VLA sees different inputs than real | Splat captures → photorealism → VLA sees real-like inputs |
| MuJoCo physics → slow, non-Rust → IPC overhead | Modular physics → Rust-native (Rapier) or FFI (MuJoCo) |
| Offline evaluation → no real-time feedback | **Rerun Time Machine** → scrub through failure cases instantly |

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
| **Agent Bridge** | Only control plane | GUI, scripts, LLM tools, and VLA-policy adapters submit every mutating command through the same local API; the command is recorded in the run log before execution. |
| **Rerun** | **Read-only visualization & diagnostics** | **Primary P1-3 Tool.** Timeline, 3D scene, plots, ghost overlays, and replay from run logs; it never drives the simulator. |
| **Tauri+SparkJS** | Interactive App | **Deferred to P4.** For custom shaders, collider/edit tools, and complex intervention UI; never the canonical store. |
| **Physics** | Object physics | Modular (Rapier/MuJoCo/Isaac) |
| **Robot Sim** | Robot dynamics | Industry-standard (Gazebo/MuJoCo) |
| **3DGS Pipeline** | Scene capture | Photorealistic captures; differentiable training pipelines exist (visualization here is non-differentiable) |
| **Dream2Flow** | World model probe | Euclidean flow target, embodiment-agnostic |
| **PID-Core** | Read-only information analysis | Computes candidate diagnostics from logged/captured data; it never triggers actions, pauses, or corrections |
| **Attribution probes** | Local explanation baselines | Reference epsilon-/AttnLRP + gradient×input probe and Rerun adapter are implemented; other methods/production-VLA hooks remain extensions |

Current deterministic bridge smokes expose stdio/TCP/WebSocket JSON-RPC methods for status, deterministic stepping, deterministic interventions, replay, run lifecycle stop, and `export.rerun`; safe mode keeps status/replay read-only and rejects mutation, run-ending, or file-writing exports. This is **partial M2 groundwork**, not completion of the full M2 acceptance contract (all target UI/VLA/backend controls plus a versioned subscription stream). Likewise, the validating run-log-to-Rerun converter is **partial M2/EC1 viewer groundwork**; the complete blueprint/viewer remains specified, not built.

---

## 5. Research Trajectory

| Phase | Goal | Key Deliverable | Visualization |
|-------|------|-----------------|---------------|
| **1** | Validate estimators (S1 gate / `grandplan.md` §7) | Four PID gates (population/measure/estimator/application, §7.1); current status is MI/coherence **NO-GO** on the high-d sweep and continuous shared-exclusions atoms on real embeddings **BLOCKED / not application-validated** | Rerun (Charts) |
| **2** | Apply to OpenVLA on LIBERO | Failure signature taxonomy | Rerun (Timeline + Logs) |
| **3** | Within-model stage/channel ablations | World-model-stage vs action-decoding vs execution diagnostics under fixed model/checkpoint/task | Rerun (3DGS + Ghost Splats) |
| **4** | Embodiment transfer via Flow-as-bridge | Cross-robot PID analysis | **Tauri + SparkJS** (Interactive) |

**Ultimate goal**: Move from "does this VLA work?" to "how does this VLA understand the world?" — enabling principled debugging and improvement of vision-language-action models.

---

## 6. Hardware and Storage Planning (No Universal Minimum)

The repository has no evidence-backed RAM, VRAM, disk, or device minimum for the target stack. Requirements depend on the selected VLA/video models, capture codec and retention policy, scene size, estimator regime, and whether inference is local or remote.

Before capture, benchmark the exact configuration and record peak RAM/VRAM, median/p95 latency, bytes per episode, temporary conversion space, and retained-artifact size. Size hardware and storage from those measurements plus an explicit safety margin; do not reuse illustrative machine specifications as requirements. See `EXPERIMENTS.md` §12.


---

## 7. World Model Comparison: ManiGaussian vs PEGS

The architecture supports a head-to-head comparison between two dominant world model paradigms for 3DGS-based robotics.

### 7.1 ManiGaussian (Learned Implicit Physics)
- **Paradigm:** End-to-end differentiable world model.
- **Mechanism:** Uses a 3D Variational Encoder to map 3DGS scenes to a latent space $Z$. Future states are predicted as $Z_{t+1} = f(Z_t, A_t)$.
- **PID Role:** Used to measure the fidelity of the latent world model: $Syn(V, L; Z)$.
- **Strength:** Captures complex, hard-to-model dynamics directly from data.
- **Weakness:** Poor generalization to novel objects (requires retraining) (reported/expected; verify against upstream papers).

### 7.2 PEGS (Explicit Particle-Based Physics)
- **Paradigm:** Hybrid explicit simulation with visual correction.
- **Mechanism:** Binds Gaussians to a PBD (Position-Based Dynamics) particle system. A target implementation may use "visual forces" to nudge particles toward photometric observations, but every correction command must be an Agent Bridge request recorded in the canonical run log before it reaches physics.
- **PID Role:** Used to measure the benefit of visual correction: $Syn(P_{pred}, V_{obs}; P_{corr})$.
- **Strength:** High generalization (physics laws don't change); handles deformables natively (reported/expected; verify against upstream papers).
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
- **License caution:** verify the upstream license/model card before depending on it (may be restrictive); avoid vendoring code into this dual-licensed **MIT OR Apache-2.0** repo unless license compatibility is confirmed.

### 9.2 Integration Notes (Verify)
- The repo describes patched HuggingFace Transformers modules; isolate integration in a separate service/environment and log the exact revision.
- Confirm how to export intermediates (`D_gen`) and the exact action parameterization (“delta actions”, etc.) before quantitative comparisons.
