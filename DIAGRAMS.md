# System Architecture Diagrams

> **Documentation Cross-Reference**:
> - `grandplan.md` — Master plan and theoretical foundations
> - `pidsplatspecs.md` — Detailed simulation environment and PID specifications
> - `ARCHITECTURE.md` — Component breakdown and advantages over VLM-based robotics
> - `EXPERIMENTS.md` — Experimental protocols for Rerun-first diagnostics, modular physics, and hypothesis testing
> - `README.md` — Quick start guide
> - `GAUSS_MI_INTEGRATION.md` — Optional 3DGS uncertainty + view selection (spec)
> - `WORLD_WARP_INTEGRATION.md` — Optional external world‑model baseline (spec)

This document contains visual representations of the prisoma system, the PID-Splat simulation environment, and the data processing pipelines.

**Docset alignment:** These diagrams are aligned to the current 2026-07-10 corrective addendum in `grandplan.md` v10.7. Several components shown below (e.g., Tauri/SparkJS/Gazebo, optional Zenoh live transport, and external video predictors) are part of the *target architecture* and may be external or not yet implemented in this repository; check `grandplan.md` “Repo status” (§11.1), the execution plan (`grandplan.md` §A.7), and the decision record (`grandplan.md` §A.8) for what exists today and what to build next.

**Docset-wide final solution:** the diagrams should be read through `grandplan.md` §A.8: run log as source of truth, Agent Bridge as the only control plane, Rerun as the read-only Phases 1–3 diagnostic viewer, and Tauri/SparkJS as the deferred Phase 4 shell. VLA actions, interventions, pause/resume/step transitions, and correction forces always traverse **client → Agent Bridge → canonical command event → backend**. PID, observers, Zenoh, and Rerun never actuate the system.

## 0. Docset v10.7 Status Dashboard (Pipeline State)

This chart is the honest, gate-driven snapshot after the corrective audit. Exp0 has a **split status**: its default high-dimensional MI/coherence sweep is **NO-GO**, while continuous `I^sx_∩` atom validation has **no valid automated gate yet**. The offline tooling is runnable, but it is not a gate-passing atom-analysis spine. The first real-VLA capture, the nested capture-sizing gate, and the episode-local H1 feature path remain open; Exp1–Exp5 therefore remain blocked.

```mermaid
flowchart TD
    classDef run fill:#1b5e20,stroke:#2e7d32,color:#fff;
    classDef gate fill:#e65100,stroke:#ef6c00,color:#fff;
    classDef blocked fill:#7f1d1d,stroke:#b71c1c,color:#fff,stroke-dasharray:5 3;

    Exp0["Exp0 split status<br/>MI/coherence = NO-GO on default high-d sweep<br/>continuous-atom gate = NOT VALID YET<br/>(runner is executable)"]:::gate

    subgraph Today["Runnable tooling today (not gate-passing VLA evidence)"]
        Harness["Offline (V,L,D,A) harness<br/>PID screens + non-PID baselines"]:::run
        ProvGate["Axis-provenance honesty gate ENFORCED<br/>--require-axis-provenance-honest (v10.4)"]:::run
        Adapter["safe_adapter → contract<br/>honest {v,l,d,a}_provenance (v10.4)"]:::run
        Attr["attribution reference probe + Rerun adapter<br/>faithfulness/provenance/relevance implemented"]:::run
        Obs["ncp-observer tap pinned NCP v0.7.0<br/>exploratory, off critical path"]:::run
    end

    Power["CAPTURE GATE NOT READY / NOT PASSED<br/>idealized power simulator exists;<br/>nested capture model + H1 prospective features missing"]:::blocked
    Capture["OPEN CRITICAL PATH<br/>real downloaded VLA capture + labels<br/>(NOT done)"]:::blocked

    subgraph Blocked["Blocked on first real capture"]
        E1["Exp1 pick-and-place (H1–H4)"]:::blocked
        E2["Exp2 long-horizon (H5)"]:::blocked
        E3["Exp3 perturbations (H1–H6)"]:::blocked
        E4["Exp4 Flow-as-Bridge (H7)"]:::blocked
        E5["Exp5 cross-embodiment (H4/H7)"]:::blocked
    end

    Exp0 --> Harness
    Harness --> ProvGate
    Harness --> Adapter
    Harness --> Attr
    Adapter --> Capture
    Power -. blocks .-> Capture
    Capture -. blocks .-> E1 & E2 & E3 & E4 & E5
```

*Caption: corrected v10.7 pipeline state — orange = executable Exp0 with split scientific status; green = runnable tooling, not validated atom evidence; red dashed = unresolved capture design/data gates and the Exp1–Exp5 protocols they block.*

---

## 0.1 Hypothesis Status (H1–H9)

Hypotheses grouped by their `grandplan.md` §14.1 status. Status is unchanged at v10.7; all hypothesis tests remain blocked on the real-VLA capture (only H8 geometry diagnostics and the H9 probe machinery run today on fixtures/synthetic).

```mermaid
flowchart TB
    classDef core fill:#0d47a1,stroke:#1565c0,color:#fff;
    classDef expl fill:#4a148c,stroke:#6a1b9a,color:#fff;
    classDef defer fill:#424242,stroke:#616161,color:#fff;
    classDef tri fill:#1b5e20,stroke:#2e7d32,color:#fff;

    subgraph Core["Core"]
        H1["H1 PID/CI predicts failure beyond baselines"]:::core
        H4["H4 Memorization vs generalization PID shifts"]:::core
        H5["H5 Long-horizon temporal PID/CI degradation<br/>(CI-only ablation mandatory)"]:::core
        H7["H7a method + H7b hypothesis<br/>(Flow-as-Bridge; §14.1 v10.7 split)"]:::core
        H8["H8 Geometry diagnostics select estimator regime (method)"]:::core
    end

    subgraph Exploratory["Exploratory"]
        H2["H2 Redundancy predicts ablation robustness"]:::expl
        H3["H3 Uniques predict intervention sensitivity"]:::expl
    end

    subgraph Deferred["Deferred"]
        H6["H6 Safety-task V–L integration (needs proper labels)"]:::defer
    end

    subgraph Triangulation["Triangulation"]
        H9["H9 Faithfulness-checked attribution triangulates/falsifies PID"]:::tri
    end
```

*Caption: H1–H9 by status (Core / Exploratory / Deferred / Triangulation). Status is unchanged at v10.7; all hypothesis verdicts remain pending the open real-VLA capture.*

---

## 0.2 Milestone / Critical-Path Roadmap (M0–M8)

Build order from `grandplan.md` §A.7. "Implemented" reflects verified in-repo crates/harnesses; M5 capture is the open critical path; M6–M8 are specified/optional. This is engineering state, not a research result.

```mermaid
flowchart TD
    classDef done fill:#1b5e20,stroke:#2e7d32,color:#fff;
    classDef partial fill:#e65100,stroke:#ef6c00,color:#fff;
    classDef active fill:#7f1d1d,stroke:#b71c1c,color:#fff,stroke-dasharray:5 3;
    classDef spec fill:#424242,stroke:#616161,color:#fff;

    M0["M0 Exp0 runner implemented<br/>MI/coherence high-d = NO-GO;<br/>continuous-atom gate not valid yet"]:::partial
    M1["M1 Run logs + replay<br/>pid-runlog: JSONL schema, validate/summary/manifest (implemented)"]:::done
    M2["M2 Agent Bridge control plane<br/>stdio/TCP/WS + safe mode implemented;<br/>full target control/subscription contract PARTIAL"]:::partial
    M3["M3 Minimal sim + Flow_gt<br/>pid-sim, Rapier harness (implemented)"]:::done
    M4["M4 Rerun adapter implemented<br/>validation + attribution tracks;<br/>full viewer blueprint PARTIAL"]:::partial
    M5["M5 Embedding harness on REAL capture<br/>safe_adapter ready; capture OPEN (not done)"]:::active
    M6["M6 Optional live transport + robot sim<br/>(specified)"]:::spec
    M7["M7 Optional predictor-driven Flow_pred<br/>(specified)"]:::spec
    M8["M8 Custom Tauri+SparkJS UI (Phase 4)<br/>(specified, deferred)"]:::spec

    M0 --> M1 --> M2 --> M3 --> M4 --> M5 --> M6 --> M7 --> M8
```

*Caption: M0–M8 roadmap — green = implemented acceptance slice, orange = implemented groundwork with an unmet scientific/milestone contract, red dashed = M5 open critical path, grey = specified/optional. Engineering state only.*

---

## 1. High-Level System Overview

This diagram illustrates the target interaction pattern. The canonical Phases 1–3 data spine is **run log → replay → Rerun**; Zenoh/live middleware is optional Phase 6 transport and must still emit the same run-log events.

```mermaid
graph TD
    subgraph "Automation Clients"
        Claude[Claude Code / Codex / opencode]
        Scripts["Scripts (Python/Rust)"]
    end

    subgraph "Inference Layer (External)"
        VLA["Target VLA (e.g., SmolVLA/OpenVLA/DreamVLA/InternVLA‑A1)"]
        WAN["Video Gen Model (WAN-like)"]
        VFM[Vision Foundation Models]

        VLA -->|Action request| Agent
        VLA -->|Embeddings| Z_EMB[Zenoh: vla/embeddings]
        WAN --> VFM
        VFM -->|3D Flow| Z_FLOW[Zenoh: dream/flow]
    end

    subgraph "Optional data transport (Zenoh; never control)"
        Z_EMB
        Z_FLOW
        Z_SENS[Zenoh: sim/sensors]
        Z_PID[Zenoh: pid/metrics]
    end

    subgraph "Simulation & Vis Layer (Rust/Rerun)"
        subgraph "Backend"
            Phys[Physics Engine]
            PID_Core["pid-core Estimator<br/>(read-only analysis)"]
            Agent["Agent Bridge (JSON-RPC/MCP)"]
            Log["Canonical run log<br/>(source of truth)"]

            Agent -->|Append command before execution| Log
            Agent -->|Dispatch only after log append| Phys
            Phys -->|Pose / contact / Flow_gt events| Log

            Z_EMB -->|Captured data| Log
            Z_FLOW -->|Captured data| Log
            Log -->|Samples only| PID_Core
            PID_Core -->|Analysis metric events| Log
            PID_Core -->|Optional live mirror| Z_PID

            Claude --> Agent
            Scripts --> Agent
        end

        subgraph "Frontend"
            Replay["Run-log replay / Rerun adapter"]
            Rerun["Rerun Viewer (P1-3, read-only)"]
            Spark["Tauri/SparkJS shell (P4)"]
            Ghost["Ghost Splats (Rerun PointCloud)"]

            Log --> Replay --> Rerun
            Replay --> Spark
            Log --> Ghost
            Ghost --> Rerun
            Ghost --> Spark
            Spark -->|Control requests only| Agent
        end
    end

    subgraph "Sensor Support"
        Gazebo[Headless Gazebo]
        Gazebo -->|RGB-D/LiDAR| Z_SENS
        Z_SENS -->|Captured observations| Log
        Z_SENS -->|Observation data| VLA
    end
```

---

## 2. PID-Splat Simulation Loop

This diagram details the target "Splat-First" update loop, showing how a physics backend (Rapier shown as an example), canonical run-log events, and rendering are synchronized: Rerun consumes the replay stream in Phases 1–3, while SparkJS can consume the same events in Phase 4.

```mermaid
sequenceDiagram
    participant Client as UI / script client
    participant VLA as VLA Agent
    participant Bridge as Agent Bridge
    participant Log as Canonical Run Log
    participant Phys as Physics (Rust)
    participant Zenoh as Zenoh Data Bus
    participant PID as PID-Core (read-only)
    participant Vis as Rerun (read-only) / SparkJS

    Note over Bridge,Phys: Every mutating command is logged before backend dispatch

    par Physics Step
        VLA->>Bridge: Submit action / action chunk
        Bridge->>Log: Append canonical action event
        Bridge->>Phys: Dispatch recorded action
        Phys->>Phys: Step Simulation (dt=1/60)
        Phys->>Log: Append poses / contacts / Flow_gt
        Phys->>Zenoh: Optional pose-data mirror
        Client->>Bridge: Request pause / step / intervention / correction
        Bridge->>Log: Append canonical control event
        Bridge->>Phys: Dispatch recorded control
    and PID Computation
        VLA->>Zenoh: Publish Embeddings (V, D)
        Zenoh->>Log: Append captured embedding events
        Log->>PID: Read analysis samples
        PID->>PID: Compute I_sx_intersect
        PID->>Log: Append analysis metrics (Syn, Red, Unq)
        PID->>Zenoh: Optional metric-data mirror
    end

    par Read-only Rendering
        Log->>Vis: Replay converted transforms / metrics / artifacts
        Vis->>Vis: Render Timeline
        Vis->>Vis: Rasterize 3DGS
    end
```

---

## 3. Geometry-First Analysis Protocol

This flowchart implements the corrected decision logic from `grandplan.md` §16.11. Every variable and every concatenation actually passed to an estimator is diagnosed. Sampled mean `δ_rel` is reported as a descriptive tree-likeness statistic only: it is **not** a Euclidean-validity pass/fail gate (a Euclidean line is the immediate counterexample).

```mermaid
flowchart TD
    Start["Input embeddings and every estimator concatenation"] --> Diag[Step 0: Geometry / dependence diagnostics]

    subgraph "Diagnostics"
        Diag --> ID["Intrinsic dimension (Levina–Bickel / GRIDE)"]
        Diag --> DC["Distance concentration (pairwise CV, nn/mean)"]
        Diag --> Ties["Ties / duplicate distances / dependence"]
        Diag --> Delta["Sampled mean δ_rel<br/>(descriptive only)"]
        Diag --> Flat["Calibrated local-flatness diagnostics"]
    end

    Delta --> DeltaNote["Report; never use alone to pass/fail Euclidean kNN"]
    ID --> GeoGate{Recovery-supporting geometry?}
    DC --> GeoGate
    Ties --> GeoGate
    Flat --> GeoGate

    GeoGate -- No --> Reduce[Reduce/quantize or use a different MI pipeline]
    Reduce --> Note0[Re-run diagnostics + Experiment 0 after pivot]
    GeoGate -- Yes --> Exp0["Run measure-independent MI recovery checks<br/>and a measure-specific atom oracle"]
    Exp0 --> MI{MI/coherence gate passes?}
    MI -- No --> Pivot["NO-GO/PIVOT for this pipeline"]
    MI -- Yes --> Atom{Valid measure-specific atom gate passes?}
    Atom -- No or unavailable --> NoAtoms["Do not interpret continuous atoms"]
    Atom -- Yes --> Euclid["Preregistered continuous I^sx_∩ regime may proceed"]
```

---

## 4. Modular Physics Backend Architecture

This diagram shows the composable backend system where rendering (Gaussian Splats) is decoupled from physics (swappable between Rapier, MuJoCo, Isaac Gym) and robot simulation (Gazebo or MuJoCo).

```mermaid
graph TB
    subgraph "Application Layer"
        Bridge["Agent Bridge<br/>(only control plane)"]
        Log["Canonical run log"]
        App["Run-log replay / Rerun adapter"]
        Config[pid-splat.toml]
        Controls["Tauri/SparkJS control UI (P4)"]
    end

    subgraph "Rendering Layer (Fixed)"
        Splats[Gaussian Splats]
        Vis["Rerun (read-only) / SparkJS panels"]
        Ghost[Ghost Splats]
        
        Splats --> Vis
        Ghost --> Vis
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

    Config --> Bridge
    Controls -->|Control requests| Bridge
    Bridge -->|Append command| Log
    Bridge -->|Dispatch only after append| PhysTrait
    Bridge -->|Dispatch only after append| RobotTrait
    PhysTrait -->|State / contact events| Log
    RobotTrait -->|State / sensor events| Log
    Log --> App --> Vis

    Log -->|Optional data mirror| Zenoh
    Zenoh -->|Captured external observations only| Log
```

### Backend Selection Logic

```mermaid
flowchart TD
    Start[Read pid-splat.toml] --> Bridge[Agent Bridge validates configuration]
    Bridge --> ConfigLog[Append canonical config event]
    ConfigLog --> CheckPhys{physics.backend?}
    
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
    
    Ready --> Render[Default P1-3: log/replay via Rerun; P4: optional SparkJS]
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
        Splats["3DGS Splats<br/>(static background / captured assets)"]
        Vis[Rerun / SparkJS\nSplat Renderer]
        Splats --> Vis
    end

    subgraph "Dynamics Scene (Geometry)"
        Mesh["Meshes/URDFs<br/>(robots + collision proxies)"]
        Three["Three.js (WebGL2/WebGPU)<br/>Mesh Renderer"]
        Mesh --> Three
    end

    subgraph "Physics"
        Phys["Physics Engine<br/>(Rapier/MuJoCo)"]
        Mesh -->|Collision shapes| Phys
        Phys -->|Pose/Transforms| Mesh
    end

    subgraph "Diagnostics"
        PID["pid-core metrics<br/>(Syn/Red/Unq, CI/Ω)"]
        PID --> Overlay["GPU overlays<br/>(Dynos / heatmaps)"]
        Overlay --> Vis
        Overlay --> Three
    end

    Cam[Viewer-only camera + UI state] --> Vis
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
        IMG --> VP[Video Predictor Service]
        TXT --> VP
        VP --> VIDEO["Predicted Video Clip (T frames)"]
    end

    subgraph "Flow Extraction"
        VIDEO --> SAM["Segmentation (model-agnostic)"]
        VIDEO --> DEPTH["Depth (relative or metric)"]
        VIDEO --> TRACK["Tracking (model-agnostic)"]
        
        SAM --> LIFT[2D to 3D Lifting]
        DEPTH --> LIFT
        TRACK --> LIFT
        LIFT --> TRAJ[3D Flow Trajectory]
    end

    subgraph "Analysis"
        TRAJ --> TARGET{PID Target}
        VLA_EMB[VLA Embeddings] --> SOURCE{PID Source}
        
        SOURCE --> EST[PID Estimator]
        TARGET --> EST
        EST --> VIZ["PID Overlays (Splats/Mesh)"]
    end
```

---

## 7. Experiment 0: Separate MI/Coherence and Atom-Validation Gates

This diagram summarizes the corrected validation loop before applying PID to real VLA embeddings (`grandplan.md` corrective addendum, §9.1, §16; `EXPERIMENTS.md` §4). The existing aggregate Exp0 label must not be presented as continuous-atom validation.

```mermaid
flowchart TD
    Start["Choose representation (V/L/D/A/Flow)"] --> Geo[Run geometry diagnostics]
    Geo -->|OK| Exp0["Run Experiment 0 (synthetic validation)"]
    Geo -->|Recovery / ID / concentration / ties / local-flatness warnings| PivotGeom[Pivot representation: reduce/quantize/Flow target]
    PivotGeom --> Geo

    Exp0 --> MIGate{Measure-independent MI/coherence passes?}
    MIGate -->|NO-GO on current default high-d sweep| StopMI[Stop/pivot this MI pipeline]
    MIGate -->|Passes after a validated pivot| AtomGate{Measure-specific atom oracle + pinned cross-check pass?}
    AtomGate -->|Unavailable today| StopAtoms[Do not interpret continuous I^sx atoms]
    AtomGate -->|Future pass| Proceed[Proceed to preregistered real-embedding analyses]

    StopMI --> PivotEst[Pivot estimator/representation]
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

    H6["H6 Safety-aware integration (Deferred)"] --> E3

    H7["H7a/H7b Flow-as-bridge (split v10.7)"] --> E4[Exp4 Dream2Flow validation]
    H7 --> E5

    H9[H9 Attribution triangulation] --> E1
    H9 --> E3
    H9 --> E4
```

---

## 9. OpenUSD / USDZ Interop (Optional)

This diagram summarizes the LeIsaac/Isaac Sim interoperability pattern referenced in `grandplan.md` §C.1: convert splats to OpenUSD for composition/validation in USD tooling, then (optionally) bring the composed result back into the PID‑Splat workflow.

```mermaid
graph LR
    PLY["3DGS Splats (.ply)"] --> GRUT[NVIDIA 3DGrut\nply_to_usd]
    GRUT --> USDZ["USDZ (packaged OpenUSD)"]

    MESH["Collision mesh (.glb/.gltf)"] --> ISAAC[Isaac Sim / LeIsaac\nUSD stage composition]
    USDZ --> ISAAC

    ISAAC --> USD["Composed background scene (.usd/.usda/.usdc)"]

    USD --> NOTE[Optional: validate alignment/colliders\nin USD tooling]
    USD --> IMPORT["Optional: convert/import into<br/>PID‑Splat scene graph (planned)"]
```

---

## 10. Agent Bridge Control Plane (LLM‑First)

The Agent Bridge is the **only** programmable control plane: it exposes the same operations to the GUI, VLA-policy adapter, scripts, and LLM coding tools (actions, scene editing, interventions, pause/resume/step, correction forces, replay, and exports). Each mutating request is appended to the canonical run log before backend dispatch.

**External backend note:** the Agent Bridge is also the *adapter surface* for third‑party simulators that expose an RL-style `reset/step` API (or their own WebSocket/pubsub interface). Their native interface sits behind the bridge; it is not a second prisoma control plane. The adapter records prisoma command events before dispatch so replay and analysis are identical across backends.

The deterministic in-repo bridge currently provides stdio/TCP/WebSocket JSON-RPC smokes for status, reset/step, scene edits, deterministic interventions, `log.replay`, `log.start`/`log.stop`, and `export.rerun`; safe mode permits status/replay and logs blocked mutation, run-ending, or file-writing export requests.

```mermaid
graph TB
    subgraph Clients
        UI["GUI (Tauri)"]
        VLA["VLA-policy adapter"]
        LLM[Claude Code / Codex / opencode]
        Script["Scripts (Python/Rust)"]
    end

    subgraph ControlPlane
        RPC["Agent Bridge<br/>(JSON-RPC over WebSocket)"]
        MCP["Optional MCP wrapper<br/>(thin adapter)"]
    end

    subgraph Core
        Sim["Deterministic sim loop<br/>(threaded)"]
        Scene["Scene graph<br/>(splats+meshes+URDF)"]
        Intervene["Intervention engine<br/>(perturb/apply/undo/branch)"]
        Log["Run log + replay<br/>(artifacts + audit)"]
        PID["PID workers<br/>(CI/Ω/SxPID)"]
        Events["Event stream<br/>(state/metrics/frames)"]
    end

    UI --> RPC
    VLA --> RPC
    Script --> RPC
    LLM --> MCP --> RPC

    RPC -->|Append request/response audit first| Log
    RPC -->|Dispatch recorded command| Sim
    RPC -->|Dispatch recorded command| Scene
    RPC -->|Dispatch recorded command| Intervene

    Sim --> Events
    Log -->|Captured samples| PID
    PID -->|Analysis metrics only| Log
    PID --> Events
    Log --> Events

    Events --> UI
    Events --> Script
    Events --> LLM
```

---

## 11. Cross-Backend Replay (Optional Robustness Control)

This diagram captures the v10.1 cross-backend replay idea (`grandplan.md` §E.1): replay the same run log under different physics backends (e.g., Rapier vs MuJoCo) and quantify divergence. This is a practical way to test whether PID findings (H1–H6) are sensitive to contact-model idiosyncrasies.

```mermaid
graph LR
    Client["UI / script client"] -->|log.replay request| Bridge[Agent Bridge]
    Log["Run log<br/>(initial state + actions + interventions)"] -->|Replay data| Bridge
    Bridge -->|Dispatch recorded replay| R[Rapier backend]
    Bridge -->|Dispatch recorded replay| M[MuJoCo backend]

    R --> TR[State/contact trace]
    M --> TM[State/contact trace]

    TR --> D["Diff + divergence metrics<br/>(state, contacts, success)"]
    TM --> D

    D --> Report["Sensitivity report<br/>(PID vs backend)"]
```

---

## 12. GauSS‑MI Uncertainty + Active View Selection (Optional)

This diagram summarizes the proposed GauSS‑MI integration (`GAUSS_MI_INTEGRATION.md`): treat 3DGS reconstruction uncertainty as a confound/diagnostic signal, optionally down‑weight unreliable visual features, and (if you are still capturing scenes) use uncertainty‑guided view selection to reduce uncertainty.

```mermaid
graph TB
    Capture[Scene capture views] --> Train["3DGS training + Nerfstudio export<br/>(PLY)"]
    Train --> Convert["Optional separately pinned<br/>PLY → SPZ converter"]
    Train --> Render[Render held-out views]
    Convert --> Render
    Render --> Resid["Residuals<br/>(I_obs vs I_render)"]
    Resid --> UMap["SceneUncertaintyMap<br/>(per-Gaussian uncertainty)"]

    UMap --> UI["UI overlay<br/>(color by uncertainty)"]
    UMap --> Gate["Quality gate<br/>(N_eff, fraction unreliable)"]
    UMap --> Wt["Optional weighting<br/>(weighted MI/PID)"]

    Gate -->|Needs more coverage| Suggest[Suggest next viewpoints]
    Suggest --> Accept[Human/script accepts proposal]
    Accept --> Bridge[Agent Bridge records capture decision]
    Bridge --> Capture

    Wt --> PID["pid-core<br/>(MI/CI/PID)"]
    PID --> Log["Run log artifacts<br/>(metric events + provenance)"]
```

---

## 13. Attribution Probes as Companion Diagnostics

This diagram places LRP/Integrated Gradients/DeepLIFT/Grad-CAM/TCAV/saliency/SHAP-style methods beside PID. The two branches answer different questions and should be compared only through logged samples, common targets, and matched interventions.

```mermaid
graph TB
    Run[Canonical run log\nsamples + embeddings + targets] --> PID[PID/CI branch\nRed / Unq / Syn / CI]
    Run --> Attr[Attribution branch\nLRP / IG / DeepLIFT / Grad-CAM / TCAV / saliency / SHAP-style]

    PID --> PFeat[Per-window / per-episode\ninformation features]
    Attr --> AFeat[Heatmaps / token scores\nconcept scores / feature rankings]

    PFeat --> Compare[Triangulation layer\nH9]
    AFeat --> Compare

    Compare --> Agree[Compatible under controls\nstronger diagnostic story]
    Compare --> Disagree[Disagreement\nrun targeted perturbations]
    Disagree --> Intervene[Agent Bridge intervention\nocclude / ablate / swap / shuffle]
    Intervene -->|Append command before dispatch| Run
    Intervene -->|Dispatch recorded command| Target[Model/backend perturbation handler]
    Target -->|New samples/results| Run

    Compare --> Log[Artifact manifest\nmethod + target + baseline + score hash]
```
