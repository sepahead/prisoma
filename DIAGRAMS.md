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

**Docset alignment:** These diagrams are aligned to `grandplan.md` v10.7. Several components shown below (e.g., Tauri/SparkJS/Gazebo, optional Zenoh live transport, external video predictors, and the Agent Bridge control plane) are part of the *target architecture* and may be external or not yet implemented in this repository; check `grandplan.md` “Repo status” (§11.1), the v10.1 execution plan (`grandplan.md` §A.7), and the ten-scientist consensus decision record (`grandplan.md` §A.8) for what exists today and what to build next.

**Docset-wide final solution:** the diagrams should be read through `grandplan.md` §A.8: run log as source of truth, Agent Bridge as the only control plane, Rerun as the Phases 1–3 diagnostic viewer, and Tauri/SparkJS as the deferred Phase 4 shell.

## 0. Docset v10.7 Status Dashboard (Pipeline State)

This chart is the honest, gate-driven snapshot for the v10.7 cut: Exp0 reports **NO-GO** (the gate working, not a bug — stricter under pid-rs 0.4.0's bias-corrected diagnostics; PIVOT under 0.3.0), the offline analysis/adapter path is runnable today, the real-VLA capture is the **still-open critical path**, and Exp1–Exp5 stay blocked on that capture. Nothing here upgrades the research/experiment status, which is unchanged since v10.3 (the v10.6/v10.7 slices are correctness/robustness + spec-audit/statistics-plan only — see CHANGELOG). The `(v10.4)`/`(v10.5)` tags below mark when a component first landed.

```mermaid
flowchart TD
    classDef run fill:#1b5e20,stroke:#2e7d32,color:#fff;
    classDef gate fill:#e65100,stroke:#ef6c00,color:#fff;
    classDef blocked fill:#7f1d1d,stroke:#b71c1c,color:#fff,stroke-dasharray:5 3;

    Exp0["Exp0 estimator + geometry gate<br/>verdict = NO-GO on synthetic high-d (pid-rs 0.4.0)<br/>(runnable: just exp0 / just exp0-bin)"]:::gate

    subgraph Today["Runnable today (gate-passing analysis spine)"]
        Harness["Offline (V,L,D,A) harness<br/>PID screens + non-PID baselines"]:::run
        ProvGate["Axis-provenance honesty gate ENFORCED<br/>--require-axis-provenance-honest (v10.4)"]:::run
        Adapter["safe_adapter → contract<br/>honest {v,l,d,a}_provenance (v10.4)"]:::run
        Attr["attribution probe (H9, faithfulness-checked)"]:::run
        Obs["ncp-observer tap pinned NCP v0.5.3 (v10.5)<br/>exploratory, off critical path"]:::run
    end

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
    Capture -. blocks .-> E1 & E2 & E3 & E4 & E5
```

*Caption: v10.7 pipeline state — orange = Exp0 gate (NO-GO, runnable); green = runnable analysis/adapter/observer spine; red dashed = the still-open real-VLA capture and the Exp1–Exp5 protocols it blocks.*

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
    classDef active fill:#7f1d1d,stroke:#b71c1c,color:#fff,stroke-dasharray:5 3;
    classDef spec fill:#424242,stroke:#616161,color:#fff;

    M0["M0 Run logs + replay<br/>pid-runlog (implemented)"]:::done
    M1["M1 JSONL schema + validate/summary/manifest<br/>(implemented)"]:::done
    M2["M2 Agent Bridge control plane<br/>stdio/TCP/WS, safe mode (implemented)"]:::done
    M3["M3 Minimal sim + Flow_gt<br/>pid-sim, Rapier harness (implemented)"]:::done
    M4["M4 Rerun-based viewer adapter<br/>pid-rerun (implemented; full viewer specified)"]:::done
    M5["M5 Embedding harness on REAL capture<br/>safe_adapter ready; capture OPEN (not done)"]:::active
    M6["M6 Optional live transport / Flow_pred<br/>(specified)"]:::spec
    M7["M7 GauSS-MI uncertainty + view selection<br/>(specified, optional)"]:::spec
    M8["M8 License/provenance automation<br/>(partial: notices, audit scripts)"]:::spec

    M0 --> M1 --> M2 --> M3 --> M4 --> M5 --> M6 --> M7 --> M8
```

*Caption: M0–M8 roadmap — green = implemented in-repo, red dashed = M5 open critical path (adapter ready, real capture not done), grey = specified/optional. Engineering state only; the research verdict is still gated on M5.*

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

    subgraph "Simulation & Vis Layer (Rust/Rerun)"
        subgraph "Backend"
            Phys[Physics Engine]
            PID_Core[pid-core Estimator]
            Agent["Agent Bridge (JSON-RPC/MCP)"]
            
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
        
        subgraph "Frontend (Rerun Viewer / SparkJS)"
            Vis["Rerun Viewer (P1-3) / SparkJS (P4)"]
            Ghost["Ghost Splats (Rerun PointCloud)"]
            
            Spark_Bridge --> Vis
            Z_PID --> Ghost
            Ghost --> Vis
        end
    end

    subgraph "Sensor Support"
        Gazebo[Headless Gazebo]
        Gazebo -->|RGB-D/LiDAR| Z_SENS
    end
```

---

## 2. PID-Splat Simulation Loop

This diagram details the target "Splat-First" update loop, showing how a physics backend (Rapier shown as an example), canonical run-log events, and rendering are synchronized: Rerun consumes the replay stream in Phases 1–3, while SparkJS can consume the same events in Phase 4.

```mermaid
sequenceDiagram
    participant Agent as Agent Bridge / UI
    participant VLA as VLA Agent
    participant Zenoh as Zenoh Bus
    participant Phys as Physics (Rust)
    participant PID as PID-Core
    participant Vis as Rerun / SparkJS

    Note over Phys,Vis: Example frame budget (hardware-dependent)

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
        Zenoh->>Vis: Log Transforms
        Zenoh->>Vis: Log Ghost Splats (PID)
        Vis->>Vis: Render Timeline
        Vis->>Vis: Rasterize 3DGS
    end
```

---

## 3. Geometry-First Analysis Protocol

This flowchart implements the decision logic from `grandplan.md` §16.11, determining whether to use Euclidean, Manifold, or Hierarchical analysis methods.
For δ-hyperbolicity thresholds, use a normalized `δ_rel` (e.g., `δ_rel = 2δ / diam(X)`) rather than raw δ; see `grandplan.md` §16.7.

```mermaid
flowchart TD
    Start["Input embeddings (V, D, A)"] --> Diag[Step 0: Geometry diagnostics]

    subgraph "Diagnostics"
        Diag --> ID["Intrinsic dimension (Levina–Bickel / GRIDE)"]
        Diag --> DC["Distance concentration (pairwise CV, nn/mean)"]
        Diag --> Delta["δ-hyperbolicity (4-point sampling)"]
        Diag --> Flat["Local flatness / curvature proxy (e.g., neighborhood PCA residual; ORC if available)"]
    end

    DC --> ConcQ{Concentration?}
    ConcQ -- Yes --> Reduce[Reduce/quantize or MI-only]
    Reduce --> Note0[Re-run diagnostics + Experiment 0 after pivot]

    ConcQ -- No --> Tree{δ_rel very small?}
    Tree -- Yes --> Hier[Tree-like regime]
    Hier --> SI[Use Shannon invariants / MI-only screening]
    Hier --> Note1["Avoid interpreting continuous I^sx_∩ atoms (no non-Euclidean derivation)"]

    Tree -- No --> FlatQ{Locally flat-ish?}
    Flat --> FlatQ

    FlatQ -- Yes --> Euclid["PCA + L∞ I^sx_∩ (after Experiment 0 gate)"]
    Euclid --> Gate{Experiment 0 passes?}
    Gate -- No --> Pivot["Pivot: quantization (discrete PID) or MI-only"]

    FlatQ -- No --> Curved[High curvature, non-hierarchical]
    Curved --> Quant[Quantization → discrete PID]
    Curved --> Unroll["Manifold unrolling → L∞ estimator (then re-validate)"]
```

---

## 4. Modular Physics Backend Architecture

This diagram shows the composable backend system where rendering (Gaussian Splats) is decoupled from physics (swappable between Rapier, MuJoCo, Isaac Gym) and robot simulation (Gazebo or MuJoCo).

```mermaid
graph TB
    subgraph "Application Layer"
        App[Rust App / Rerun Logger]
        Config[pid-splat.toml]
    end

    subgraph "Rendering Layer (Fixed)"
        Splats[Gaussian Splats]
        Vis[Rerun / SparkJS]
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

    Config --> App
    App --> Vis
    App --> PhysTrait
    App --> RobotTrait
    
    PhysTrait <--> Zenoh
    RobotTrait <--> Zenoh
    Vis <--> Zenoh
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

    Cam[Shared camera + UI state] --> Vis
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

## 7. Experiment 0 Validation Gate (GO/PIVOT/NO-GO)

This diagram summarizes the required estimator/geometry validation loop before applying PID to real VLA embeddings (`grandplan.md` §9.1, §16; `EXPERIMENTS.md` §4).

```mermaid
flowchart TD
    Start["Choose representation (V/L/D/A/Flow)"] --> Geo[Run geometry diagnostics]
    Geo -->|OK| Exp0["Run Experiment 0 (synthetic validation)"]
    Geo -->|Flags non-Euclidean / concentration| PivotGeom[Pivot representation: reduce/quantize/Flow target]
    PivotGeom --> Geo

    Exp0 --> Gate{Meets coherence gates?}
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

The Agent Bridge is the “programmable face” of the simulator: a local control plane that exposes the same operations the GUI uses (scene editing, interventions, run control, replay, exports). It is designed to be called by scripts and LLM coding tools without introducing irreproducible “manual steps”.

**External backend note:** the Agent Bridge can also act as an *adapter surface* for third‑party simulators that already expose an RL-style `reset/step` API (or their own WebSocket/pubsub control plane). In that mode, the adapter must still write prisoma run‑log events so replay and analysis are identical across backends.

The deterministic in-repo bridge currently provides stdio/TCP/WebSocket JSON-RPC smokes for status, reset/step, scene edits, deterministic interventions, `log.replay`, `log.start`/`log.stop`, and `export.rerun`; safe mode permits status/replay and logs blocked mutation, run-ending, or file-writing export requests.

```mermaid
graph TB
    subgraph Clients
        UI["GUI (Tauri)"]
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

---

## 11. Cross-Backend Replay (Optional Robustness Control)

This diagram captures the v10.1 cross-backend replay idea (`grandplan.md` §E.1): replay the same run log under different physics backends (e.g., Rapier vs MuJoCo) and quantify divergence. This is a practical way to test whether PID findings (H1–H6) are sensitive to contact-model idiosyncrasies.

```mermaid
graph LR
    Log["Run log<br/>(initial state + actions + interventions)"] -->|Replay| R[Rapier backend]
    Log -->|Replay| M[MuJoCo backend]

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
    Capture[Scene capture views] --> Train["3DGS training<br/>(PLY/SPZ)"]
    Train --> Render[Render held-out views]
    Render --> Resid["Residuals<br/>(I_obs vs I_render)"]
    Resid --> UMap["SceneUncertaintyMap<br/>(per-Gaussian uncertainty)"]

    UMap --> UI["UI overlay<br/>(color by uncertainty)"]
    UMap --> Gate["Quality gate<br/>(N_eff, fraction unreliable)"]
    UMap --> Wt["Optional weighting<br/>(weighted MI/PID)"]

    Gate -->|Needs more coverage| Suggest[Suggest next viewpoints]
    Suggest --> Capture

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
    Intervene --> Run

    Compare --> Log[Artifact manifest\nmethod + target + baseline + score hash]
```
