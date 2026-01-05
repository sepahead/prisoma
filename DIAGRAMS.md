# System Architecture Diagrams

> **Documentation Cross-Reference**:
> - `grandplan.md` — Master plan and theoretical foundations
> - `pidsplatspecs.md` — Detailed simulation environment and PID specifications
> - `ARCHITECTURE.md` — Component breakdown and advantages over VLM-based robotics
> - `EXPERIMENTS.md` — Experimental protocols for SparkJS and Modular Physics setup and hypothesis testing
> - `README.md` — Quick start guide

This document contains visual representations of the PID-VLA system, the PID-Splat simulation environment, and the data processing pipelines.

**Docset alignment:** These diagrams are aligned to `grandplan.md` v7.0. Several components shown below (e.g., Tauri/SparkJS/Gazebo/Zenoh, external video predictors, and the Agent Bridge control plane) are part of the *target architecture* and may be external or not yet implemented in this repository; check `grandplan.md` “Repo status” for what exists today.

## 1. High-Level System Overview

This diagram illustrates how the core components interact via the Zenoh middleware, separating the inference (VLA), simulation (PID-Splat), and analysis (PID-Core) layers.

```mermaid
graph TD
    subgraph "Automation Clients"
        Claude[Claude Code / Codex / opencode]
        Scripts[Scripts (Python/Rust)]
    end

    subgraph "Inference Layer (External)"
        VLA[OpenVLA / DreamVLA]
        WAN[Video Gen Model (WAN-like)]
        Vis[Vision Foundation Models]
        
        VLA -->|Action| Z_ACT[Zenoh: vla/action]
        VLA -->|Embeddings| Z_EMB[Zenoh: vla/embeddings]
        WAN --> Vis
        Vis -->|3D Flow| Z_FLOW[Zenoh: dream/flow]
    end

    subgraph "Middleware (Zenoh)"
        Z_ACT
        Z_EMB
        Z_FLOW
        Z_SENS[Zenoh: sim/sensors]
        Z_PID[Zenoh: pid/metrics]
    end

    subgraph "Simulation & Vis Layer (Tauri/Rust)"
        subgraph "Backend"
            Phys[Physics Engine]
            PID_Core[pid-core Estimator]
            Agent[Agent Bridge (JSON-RPC/MCP)]
            
            Z_ACT --> Phys
            Phys -->|Pose| Spark_Bridge
            
            Z_EMB --> PID_Core
            Z_FLOW --> PID_Core
            PID_Core -->|Synergy/Red/Unq| Z_PID

            Claude --> Agent
            Scripts --> Agent
            Agent -->|Scene edits / interventions| Phys
            Agent -->|Compute requests| PID_Core
        end
        
        subgraph "Frontend (WebGPU)"
            SparkJS[SparkJS Renderer]
            Dynos[PID Dyno Shaders]
            
            Spark_Bridge --> SparkJS
            Z_PID --> Dynos
            Dynos --> SparkJS
        end
    end

    subgraph "Sensor Support"
        Gazebo[Headless Gazebo]
        Gazebo -->|RGB-D/LiDAR| Z_SENS
    end
```

---

## 2. PID-Splat Simulation Loop

This diagram details the "Splat-First" update loop, showing how physics (Rapier) and rendering (SparkJS) are synchronized and how PID metrics modulate the visual output.

```mermaid
sequenceDiagram
    participant Agent as Agent Bridge / UI
    participant VLA as VLA Agent
    participant Zenoh as Zenoh Bus
    participant Phys as Physics (Rust)
    participant PID as PID-Core
    participant Spark as SparkJS (WebGPU)

    Note over Phys, Spark: Example frame budget (hardware-dependent)

    par Physics Step
        VLA->>Zenoh: Publish Action (Joints)
        Zenoh->>Phys: Apply Forces
        Phys->>Phys: Step Simulation (dt=1/60)
        Phys->>Zenoh: Publish Object Poses
        Agent->>Phys: Apply intervention (pause/step-safe)
    and PID Computation
        VLA->>Zenoh: Publish Embeddings (V, D)
        Zenoh->>PID: Update Buffer
        PID->>PID: Compute I_sx_intersect
        PID->>Zenoh: Publish (Syn, Red, Unq)
    end

    par Rendering
        Zenoh->>Spark: Update Proxy Transforms
        Zenoh->>Spark: Update Splat Colors (PID)
        Spark->>Spark: Run Dyno Shaders
        Spark->>Spark: Rasterize 3DGS
    end
```

---

## 3. Geometry-First Analysis Protocol

This flowchart implements the decision logic from `grandplan.md` §16.11, determining whether to use Euclidean, Manifold, or Hierarchical analysis methods.
For δ-hyperbolicity thresholds, use a normalized `δ_rel` (e.g., `δ_rel = 2δ / diam(X)`) rather than raw δ; see `grandplan.md` §16.7.

```mermaid
flowchart TD
    Start[Input embeddings (V, D, A)] --> Diag[Step 0: Geometry diagnostics]

    subgraph "Diagnostics"
        Diag --> ID[Intrinsic dimension (Levina–Bickel / GRIDE)]
        Diag --> DC[Distance concentration (pairwise CV, nn/mean)]
        Diag --> Delta[δ-hyperbolicity (4-point sampling)]
        Diag --> Flat[Local flatness / curvature proxy (e.g., neighborhood PCA residual; ORC if available)]
    end

    DC --> ConcQ{Concentration?}
    ConcQ -- Yes --> Reduce[Reduce/quantize or MI-only]
    Reduce --> Note0[Re-run diagnostics + Experiment 0 after pivot]

    ConcQ -- No --> Tree{δ_rel very small?}
    Tree -- Yes --> Hier[Tree-like regime]
    Hier --> SI[Use Shannon invariants / MI-only screening]
    Hier --> Note1[Avoid interpreting continuous I^sx_∩ atoms (no non-Euclidean derivation)]

    Tree -- No --> FlatQ{Locally flat-ish?}
    Flat --> FlatQ

    FlatQ -- Yes --> Euclid[PCA + L∞ I^sx_∩ (after Experiment 0 gate)]
    Euclid --> Gate{Experiment 0 passes?}
    Gate -- No --> Pivot[Pivot: quantization (discrete PID) or MI-only]

    FlatQ -- No --> Curved[High curvature, non-hierarchical]
    Curved --> Quant[Quantization → discrete PID]
    Curved --> Unroll[Manifold unrolling → L∞ estimator (then re-validate)]
```

---

## 4. Modular Physics Backend Architecture

This diagram shows the composable backend system where rendering (Gaussian Splats) is decoupled from physics (swappable between Rapier, MuJoCo, Isaac Gym) and robot simulation (Gazebo or MuJoCo).

```mermaid
graph TB
    subgraph "Application Layer"
        Tauri[Tauri App]
        Config[pid-splat.toml]
    end

    subgraph "Rendering Layer (Fixed)"
        Splats[Gaussian Splats]
        SparkJS[SparkJS WebGPU]
        Dynos[PID Dyno Shaders]
        
        Splats --> SparkJS
        Dynos --> SparkJS
    end

    subgraph "Physics Layer (Swappable)"
        direction TB
        PhysTrait[PhysicsBackend Trait]
        
        subgraph "Implementations"
            Rapier["Rapier3D\n(low-latency; hardware-dependent)\nRust-native"]
            MuJoCo["MuJoCo\nstrong contact modeling\nFFI bindings"]
            Isaac["Isaac Gym\nGPU-parallel (if available)\n(batch scale)"]
        end
        
        PhysTrait --> Rapier
        PhysTrait --> MuJoCo
        PhysTrait --> Isaac
    end

    subgraph "Robot Layer (Swappable)"
        direction TB
        RobotTrait[RobotBackend Trait]
        
        subgraph "Robot Implementations"
            GazeboRobot["Gazebo Harmonic\nIndustry URDFs\nSensor sim"]
            MuJoCoRobot["MuJoCo Robot\nLegacy support\nBenchmark compat"]
        end
        
        RobotTrait --> GazeboRobot
        RobotTrait --> MuJoCoRobot
    end

    subgraph "Middleware"
        Zenoh[Zenoh Bus]
    end

    Config --> Tauri
    Tauri --> SparkJS
    Tauri --> PhysTrait
    Tauri --> RobotTrait
    
    PhysTrait <--> Zenoh
    RobotTrait <--> Zenoh
    SparkJS <--> Zenoh
```

### Backend Selection Logic

```mermaid
flowchart TD
    Start[Read pid-splat.toml] --> CheckPhys{physics.backend?}
    
    CheckPhys -->|rapier| Rapier[Initialize Rapier3D]
    CheckPhys -->|mujoco| MuJoCo[Initialize MuJoCo FFI]
    CheckPhys -->|isaac| Isaac[Initialize Isaac Gym]
    
    Rapier --> CheckRobot{robot.backend?}
    MuJoCo --> CheckRobot
    Isaac --> CheckRobot
    
    CheckRobot -->|gazebo| Gazebo[Launch Headless Gazebo]
    CheckRobot -->|mujoco| MuJoCoR[Use MuJoCo Robot]
    CheckRobot -->|none| NoRobot[Object-only Simulation]
    
    Gazebo --> Ready[Simulation Ready]
    MuJoCoR --> Ready
    NoRobot --> Ready
    
    Ready --> Render[Default: Gaussian splats via SparkJS]
```

### Use Case Decision Tree

```mermaid
flowchart TD
    UseCase[What's your use case?] --> Speed{Need speed?}
    
    Speed -->|Yes, prioritize speed| Rapier[Use: physics.backend = rapier]
    Speed -->|No| Contact{Contact-rich manipulation?}
    
    Contact -->|Yes, precise grasping| MuJoCo[Use: physics.backend = mujoco]
    Contact -->|No| Batch{Large-scale experiments?}
    
    Batch -->|Yes, large-scale batch runs| Isaac[Use: physics.backend = isaac]
    Batch -->|No| Benchmark{Comparing to papers?}
    
    Benchmark -->|Yes, LIBERO/MetaWorld| MuJoCo
    Benchmark -->|No| Rapier
    
    Rapier --> Robot{Need robot sim?}
    MuJoCo --> Robot
    Isaac --> Robot
    
    Robot -->|Yes, accurate kinematics| Gazebo[Use: robot.backend = gazebo]
    Robot -->|No, objects only| None[Use: robot.backend = none]
```

---

## 5. Hybrid Rendering: Splats + Mesh + Physics Proxies

This diagram captures the intended hybrid approach: use 3DGS splats for photoreal appearance, and meshes/URDFs for articulated robots, collision proxies, and precise interactive edits. This aligns with `grandplan.md` §A and §16 (geometry/diagnostics are independent of the renderer, but the renderer must support inspectable overlays).

```mermaid
graph TB
    subgraph "Visual Scene (Appearance)"
        Splats[3DGS Splats\n(static background / captured assets)]
        Spark[SparkJS (WebGPU)\nSplat Renderer]
        Splats --> Spark
    end

    subgraph "Dynamics Scene (Geometry)"
        Mesh[Meshes/URDFs\n(robots + collision proxies)]
        Three[Three.js/WebGPU\nMesh Renderer]
        Mesh --> Three
    end

    subgraph "Physics"
        Phys[Physics Engine\n(Rapier/MuJoCo)]
        Mesh -->|Collision shapes| Phys
        Phys -->|Pose/Transforms| Mesh
    end

    subgraph "Diagnostics"
        PID[pid-core metrics\n(Syn/Red/Unq, CI/Ω)]
        PID --> Overlay[GPU overlays\n(Dynos / heatmaps)]
        Overlay --> Spark
        Overlay --> Three
    end

    Cam[Shared camera + UI state] --> Spark
    Cam --> Three
```

---

## 6. Dream2Flow Data Pipeline

Visualizing a model-agnostic Dream2Flow-style bridge: external video prediction → 3D flow extraction → PID targets (see `grandplan.md` §9.7.7, §10.10). The video predictor is treated as an interchangeable, versioned service (no oracle framing).

```mermaid
graph LR
    subgraph "Input"
        IMG[Current Image]
        TXT[Instruction]
    end

    subgraph "Video Prediction (External)"
        IMG & TXT --> VP[Video Predictor Service]
        VP --> VIDEO[Predicted Video Clip (T frames)]
    end

    subgraph "Flow Extraction"
        VIDEO --> SAM[Segmentation (model-agnostic)]
        VIDEO --> DEPTH[Depth (relative or metric)]
        VIDEO --> TRACK[Tracking (model-agnostic)]
        
        SAM & DEPTH & TRACK --> LIFT[2D to 3D Lifting]
        LIFT --> TRAJ[3D Flow Trajectory]
    end

    subgraph "Analysis"
        TRAJ --> TARGET{PID Target}
        VLA_EMB[VLA Embeddings] --> SOURCE{PID Source}
        
        SOURCE & TARGET --> EST[PID Estimator]
        EST --> VIZ[PID Overlays (Splats/Mesh)]
    end
```

---

## 7. Experiment 0 Validation Gate (GO/PIVOT/NO-GO)

This diagram summarizes the required estimator/geometry validation loop before applying PID to real VLA embeddings (`grandplan.md` §9.1, §16; `EXPERIMENTS.md` §4).

```mermaid
flowchart TD
    Start[Choose representation (V/L/D/A/Flow)] --> Geo[Run geometry diagnostics]
    Geo -->|OK| Exp0[Run Experiment 0 (synthetic validation)]
    Geo -->|Flags non-Euclidean / concentration| PivotGeom[Pivot representation: reduce/quantize/Flow target]
    PivotGeom --> Geo

    Exp0 --> Gate{Meets accuracy/stability thresholds?}
    Gate -->|GO| Proceed[Proceed to real embeddings + preregistered analyses]
    Gate -->|PIVOT| PivotEst[Pivot estimator/representation; re-run Geo + Exp0]
    Gate -->|NO-GO| Stop[Stop: do not interpret PID atoms]

    PivotEst --> Geo
```

---

## 8. Hypotheses → Experiments Map

```mermaid
graph LR
    H1[H1 Grounding failures] --> E1[Exp1 Pick-and-place]
    H1 --> E3[Exp3 Instruction perturbation]

    H4[H4 Memorization vs generalization] --> E1
    H4 --> E3
    H4 --> E5[Exp5 Cross-embodiment]

    H5[H5 Temporal synergy degradation] --> E2[Exp2 Long-horizon assembly]

    H6[H6 Safety-aware integration] --> E3

    H7[H7 Flow-as-bridge] --> E4[Exp4 Dream2Flow validation]
    H7 --> E5
```

---

## 9. OpenUSD / USDZ Interop (Optional)

This diagram summarizes the LeIsaac/Isaac Sim interoperability pattern referenced in `grandplan.md` §C.1: convert splats to OpenUSD for composition/validation in USD tooling, then (optionally) bring the composed result back into the PID‑Splat workflow.

```mermaid
graph LR
    PLY[3DGS Splats (.ply)] --> GRUT[NVIDIA 3DGrut\nply_to_usd]
    GRUT --> USDZ[USDZ (packaged OpenUSD)]

    MESH[Collision mesh (.glb/.gltf)] --> ISAAC[Isaac Sim / LeIsaac\nUSD stage composition]
    USDZ --> ISAAC

    ISAAC --> USD[Composed background scene (.usd/.usda/.usdc)]

    USD --> NOTE[Optional: validate alignment/colliders\nin USD tooling]
    USD --> IMPORT[Optional: convert/import into\nPID‑Splat scene graph (planned)]
```

---

## 10. Agent Bridge Control Plane (LLM‑First)

The Agent Bridge is the “programmable face” of the simulator: a local control plane that exposes the same operations the GUI uses (scene editing, interventions, run control, replay, exports). It is designed to be called by scripts and LLM coding tools without introducing irreproducible “manual steps”.

```mermaid
graph TB
    subgraph Clients
        UI[GUI (Tauri)]
        LLM[Claude Code / Codex / opencode]
        Script[Scripts (Python/Rust)]
    end

    subgraph ControlPlane
        RPC[Agent Bridge\n(JSON-RPC over WebSocket)]
        MCP[Optional MCP wrapper\n(thin adapter)]
    end

    subgraph Core
        Sim[Deterministic sim loop\n(threaded)]
        Scene[Scene graph\n(splats+meshes+URDF)]
        Intervene[Intervention engine\n(perturb/apply/undo/branch)]
        Log[Run log + replay\n(artifacts + audit)]
        PID[PID workers\n(CI/Ω/SxPID)]
        Events[Event stream\n(state/metrics/frames)]
    end

    UI --> RPC
    Script --> RPC
    LLM --> MCP --> RPC

    RPC --> Sim
    RPC --> Scene
    RPC --> Intervene
    RPC --> Log
    RPC --> PID

    Sim --> Events
    PID --> Events
    Log --> Events

    Events --> UI
    Events --> Script
    Events --> LLM
```
