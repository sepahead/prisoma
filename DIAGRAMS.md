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

**Docset alignment:** These diagrams are aligned to `grandplan.md` docset v12.5 (seventh adversarial revision; scientific cut 2026-07-12). Several components shown below (e.g., Tauri/SparkJS/Gazebo, optional Zenoh live transport, and external video predictors) are part of the *target architecture* and may be external or not yet implemented in this repository; check `grandplan.md` current-versus-target implementation (§8.10), the research milestones M0–M7 (`grandplan.md` §12), and the decision log (`grandplan.md` §16) for what exists today and what to build next.

**v10.7 → v12.5 migration note:** the old H1–H9 / Exp0–Exp10 scheme is retired. The confirmatory registry is now **EC1** (provenance-complete replay) plus **H1–H4** (`grandplan.md` §4); the estimator/experiment ordering is the **S0–S7 gate sequence** (§5.1); build order is **milestones M0–M7** (§12). Legacy "Exp0" estimator validation is now the **S1 gate / §7**. These diagrams are retargeted accordingly.

**Docset-wide final solution:** the diagrams should be read through `grandplan.md` §16 (decision log; see also §8.2, §8.11, §8.13, §15.4): run log as source of truth, Agent Bridge as the only control plane, Rerun as the read-only Phases 1–3 diagnostic viewer, and Tauri/SparkJS as the deferred Phase 4 shell. VLA actions, interventions, pause/resume/step transitions, and correction forces always traverse **client → Agent Bridge → canonical command event → backend**. PID, observers, Zenoh, and Rerun never actuate the system.

## 0. Docset v12.5 Status Dashboard (Pipeline State)

This chart is the honest, gate-driven snapshot. Estimator/measure validation (the **S1 gate**, `grandplan.md` §7) is judged against four separate PID gates — population, measure, estimator, and application (§7.1). The high-dimensional **MI/coherence path is NO-GO** (nuisance-dimension controls); continuous shared-exclusions atoms on **real VLA embeddings are BLOCKED / not application-validated**; the `pid-rs` pin does carry real low-dimensional additive-Gaussian oracle and discrete SxPID reference evidence. The first real-VLA capture, the capture-sizing/power gate (§6.8), the intervention pilot (S3), and the episode-local H1 feature path remain open; the confirmatory EC1/H1–H4 claims therefore remain blocked.

```mermaid
flowchart TD
    classDef run fill:#1b5e20,stroke:#2e7d32,color:#fff;
    classDef gate fill:#e65100,stroke:#ef6c00,color:#fff;
    classDef blocked fill:#7f1d1d,stroke:#b71c1c,color:#fff,stroke-dasharray:5 3;

    S1["S1 estimator/measure gate (§7)<br/>four gates: population / measure / estimator / application<br/>MI/coherence = NO-GO on high-d<br/>continuous i^sx atoms on real embeddings = BLOCKED<br/>low-d Gaussian oracle + discrete SxPID reference = PASS"]:::gate

    subgraph Today["Runnable tooling today (not application-validated VLA evidence)"]
        Harness["Offline (V,L,D,A) harness<br/>PID screens + non-PID baselines"]:::run
        ProvGate["Axis-provenance honesty gate ENFORCED<br/>--require-axis-provenance-honest"]:::run
        Adapter["safe_adapter → contract<br/>bounded hash-manifest ingress<br/>honest {v,l,d,a}_provenance (S2/EC1 reference adapter)"]:::run
        Attr["attribution reference probe + Rerun adapter<br/>faithfulness/provenance/relevance implemented"]:::run
        Obs["ncp-observer + 18-case deterministic fixture observatory<br/>NCP v0.8.0 (wire 0.8); optional read-only, off critical path<br/>local E3-style fixture evidence only"]:::run
        H1Ref["synthetic H1 Protocol-A reference<br/>preflight + paired response scoring"]:::run
        H2Ref["synthetic H2 fixed-horizon reference<br/>IPCW Brier + alarm accounting"]:::run
    end

    Power["CAPTURE / POWER GATE NOT READY (§6.8)<br/>idealized power simulator exists;<br/>nested capture model + H1 prospective features missing"]:::blocked
    Capture["OPEN CRITICAL PATH<br/>real downloaded VLA capture + labels<br/>(NOT done)"]:::blocked

    subgraph Blocked["Blocked on capture + intervention pilot (S2/S3)"]
        EC1["EC1 provenance-complete replay"]:::blocked
        H1["H1 pre-treatment diagnostics predict intervention response"]:::blocked
        H2["H2 censoring-aware failure prediction"]:::blocked
        H3["H3 conditional PID incremental value"]:::blocked
        H4["H4 availability vs causal use"]:::blocked
    end

    S1 --> Harness
    Harness --> ProvGate
    Harness --> Adapter
    Harness --> Attr
    Harness --> H1Ref & H2Ref
    Adapter --> Capture
    Power -. blocks .-> Capture
    Capture -. blocks .-> EC1 & H1 & H2 & H3 & H4
```

*Caption: v12.5 pipeline state — orange = the S1 estimator/measure gate (four-gate status); green = runnable tooling, not application-validated atom evidence; red dashed = unresolved capture/power gates (§6.8) and the EC1/H1–H4 confirmatory claims they block.*

---

## 0.1 Confirmatory Claim Status (EC1, H1–H4)

Claims grouped by their `grandplan.md` §4 confirmatory-registry role (kill rules in §3.8; falsifiability in §13 Lens 20). All confirmatory tests remain blocked on the real-VLA capture and the intervention pilot. Estimator validation, attribution probes, and the explicitly non-evidentiary synthetic H1 Protocol-A and H2 fixed-horizon/IPCW/alarm software references run today on fixtures.

```mermaid
flowchart TB
    classDef core fill:#0d47a1,stroke:#1565c0,color:#fff;
    classDef eng fill:#1b5e20,stroke:#2e7d32,color:#fff;
    classDef cond fill:#4a148c,stroke:#6a1b9a,color:#fff;
    classDef defer fill:#424242,stroke:#616161,color:#fff;

    subgraph Engineering["Engineering acceptance"]
        EC1["EC1 provenance-complete replay"]:::eng
    end

    subgraph Confirmatory["Confirmatory"]
        H1["H1 pre-treatment diagnostics predict intervention response<br/>(Protocol A paired vs Protocol B randomized)"]:::core
        H2["H2 censoring-aware prospective failure prediction"]:::core
    end

    subgraph Conditional["Conditional (validated support envelope)"]
        H3["H3 PID adds incremental value only inside its validated envelope"]:::cond
        H4["H4 representational availability can diverge from causal use"]:::cond
    end

    subgraph ExplDefer["Exploratory / retired-deferred (§4)"]
        EXP["Exploratory questions (e.g. flow-as-bridge §9.6)"]:::defer
        RET["Retired/deferred legacy H-claims"]:::defer
    end
```

*Caption: EC1 + H1–H4 by role (engineering / confirmatory / conditional / exploratory-deferred) per `grandplan.md` §4. All confirmatory verdicts remain pending the open real-VLA capture and intervention pilot.*

---

## 0.2 Research Milestone / Critical-Path Roadmap (M0–M7)

Build order from `grandplan.md` §12 (research milestones M0–M7; gate sequence §5.1). The old repo used M1–M5 for *infrastructure* (run logs, Agent Bridge, sim, Rerun); those are now the event-model + control-plane parts of §8 and feed the research milestones as groundwork. "Implemented" reflects verified in-repo crates/harnesses; the real capture + intervention pilot (M3) is the open critical path; M4–M7 are downstream/specified. This is engineering state, not a research result.

```mermaid
flowchart TD
    classDef done fill:#1b5e20,stroke:#2e7d32,color:#fff;
    classDef partial fill:#e65100,stroke:#ef6c00,color:#fff;
    classDef active fill:#7f1d1d,stroke:#b71c1c,color:#fff,stroke-dasharray:5 3;
    classDef spec fill:#424242,stroke:#616161,color:#fff;

    Infra["Infrastructure groundwork (§8 event model + control plane)<br/>run logs + replay, Agent Bridge, pid-sim/Rapier, Rerun adapter — implemented"]:::done

    M0["M0 freeze scientific + identification contracts"]:::partial
    M1["M1 repair + version estimator gates (S1 / §7)<br/>MI/coherence high-d = NO-GO"]:::partial
    M2["M2 core + ecosystem conformance benchmark<br/>(incl. dependency firebreak: NCP-off + estimator-off H1/H2, §8.9.3)"]:::partial
    M3["M3 intervention pilot (S3)<br/>real capture OPEN (not done)"]:::active
    M4["M4 locked H1 experiment"]:::spec
    M5["M5 locked H2 experiment"]:::spec
    M6["M6 H3 or H4"]:::spec
    M7["M7 transport replication"]:::spec

    Infra --> M0 --> M1 --> M2 --> M3 --> M4 --> M5 --> M6 --> M7
```

*Caption: research milestones M0–M7 (`grandplan.md` §12) — green = implemented infrastructure groundwork (§8), orange = partially met research contracts, red dashed = M3 intervention pilot blocked on the open real capture, grey = specified/downstream. Engineering state only, not a research result.*

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

This flowchart implements the geometry/dependence decision logic from `grandplan.md` §7.9 (geometry diagnostics are diagnostics, not proofs; see also §7.10 on metric substitution). Every variable and every concatenation actually passed to an estimator is diagnosed. Sampled mean `δ_rel` is reported as a descriptive tree-likeness statistic only: it is **not** a Euclidean-validity pass/fail gate (a Euclidean line is the immediate counterexample).

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

This diagram captures the intended hybrid approach: use 3DGS splats for photoreal appearance, and meshes/URDFs for articulated robots, collision proxies, and precise interactive edits. This aligns with `grandplan.md` §8.13 (visualization and rendering) and §7.9 (geometry/diagnostics are independent of the renderer, but the renderer must support inspectable overlays).

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

Visualizing a model-agnostic Dream2Flow-style bridge: external video prediction → 3D flow extraction → PID targets (flow as a bridge; see `grandplan.md` §9.6). The video predictor is treated as an interchangeable, versioned service (no oracle framing).

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

## 7. Estimator/Measure Validation (S1): The Four Gates and Atom Validation

This diagram summarizes the estimator/measure validation loop — the **S1 gate** — before applying PID to real VLA embeddings (`grandplan.md` §7; the four gates population/measure/estimator/application in §7.1; continuous shared-exclusions gate §7.5; discrete PID gate §7.6). The aggregate estimator-validation label must not be presented as continuous shared-exclusions atom validation.

```mermaid
flowchart TD
    Start["Choose representation (V/L/D/A/Flow)"] --> Geo[Run geometry diagnostics]
    Geo -->|OK| S1["Run S1 synthetic validation matrix (§7.3)"]
    Geo -->|Recovery / ID / concentration / ties / local-flatness warnings| PivotGeom[Pivot representation: reduce/quantize/Flow target]
    PivotGeom --> Geo

    S1 --> MIGate{Measure-independent MI/coherence passes? (§7.7)}
    MIGate -->|NO-GO on high-d| StopMI[Stop/pivot this MI pipeline]
    MIGate -->|Passes after a validated pivot| AtomGate{Application gate: real-embedding regime near a validated support envelope? (§7.14)}
    AtomGate -->|BLOCKED / not application-validated today| StopAtoms[Do not interpret continuous i^sx atoms]
    AtomGate -->|Future pass| Proceed[Proceed to preregistered real-embedding analyses]

    StopMI --> PivotEst[Pivot estimator/representation]
    PivotEst --> Geo
```

---

## 8. Confirmatory Claims → Experimental Programme Map

```mermaid
graph LR
    EC1[EC1 provenance-complete replay] --> INFRA["§8.8 infrastructure conformance benchmark"]

    H1[H1 pre-treatment diagnostics predict intervention response] --> PA["§6.3 Protocol A paired algorithmic response"]
    H1 --> PB["§6.3 Protocol B randomized closed-loop response"]

    H2[H2 censoring-aware failure prediction] --> H2A["§6.4 prospective failure with time + censoring"]

    H3[H3 conditional PID incremental value] --> ENV["§7.14 application-support envelope"]
    H4[H4 availability vs causal use] --> ENV

    PA --> PROG["§5 experimental programme + §5.4 intervention taxonomy"]
    PB --> PROG
    H2A --> PROG
```

---

## 9. OpenUSD / USDZ Interop (Optional)

This diagram summarizes the LeIsaac/Isaac Sim interoperability pattern (interoperability, not reinvention; `grandplan.md` §8.6): convert splats to OpenUSD for composition/validation in USD tooling, then (optionally) bring the composed result back into the PID‑Splat workflow.

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

This diagram captures the cross-backend replay idea (`grandplan.md` §8.5 replay levels; robustness/falsification §6.10): replay the same run log under different physics backends (e.g., Rapier vs MuJoCo) and quantify divergence. This is a practical way to test whether PID findings (H1–H4) are sensitive to contact-model idiosyncrasies.

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

    PFeat --> Compare[Triangulation layer\nH4 / exploratory]
    AFeat --> Compare

    Compare --> Agree[Compatible under controls\nstronger diagnostic story]
    Compare --> Disagree[Disagreement\nrun targeted perturbations]
    Disagree --> Intervene[Agent Bridge intervention\nocclude / ablate / swap / shuffle]
    Intervene -->|Append command before dispatch| Run
    Intervene -->|Dispatch recorded command| Target[Model/backend perturbation handler]
    Target -->|New samples/results| Run

    Compare --> Log[Artifact manifest\nmethod + target + baseline + score hash]
```
