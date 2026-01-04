# System Architecture Diagrams

> **Documentation Cross-Reference**:
> - `grandplan.md` — Master plan and theoretical foundations
> - `pidsplatspecs.md` — Detailed simulation environment and PID specifications
> - `ARCHITECTURE.md` — Component breakdown and advantages over VLM-based robotics
> - `EXPERIMENTS.md` — Experimental protocols for SparkJS, Gazebo, Rapier setup and hypothesis testing
> - `README.md` — Quick start guide

This document contains visual representations of the PID-VLA system, the PID-Splat simulation environment, and the data processing pipelines.

## 1. High-Level System Overview

This diagram illustrates how the core components interact via the Zenoh middleware, separating the inference (VLA), simulation (PID-Splat), and analysis (PID-Core) layers.

```mermaid
graph TD
    subgraph "Inference Layer (Python/MLX)"
        VLA[OpenVLA / DreamVLA]
        WAN[WAN 2.2 Video Gen]
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
            Rapier[Rapier3D Physics]
            PID_Core[pid-core Estimator]
            
            Z_ACT --> Rapier
            Rapier -->|Pose| Spark_Bridge
            
            Z_EMB --> PID_Core
            Z_FLOW --> PID_Core
            PID_Core -->|Synergy/Red/Unq| Z_PID
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
    participant VLA as VLA Agent
    participant Zenoh as Zenoh Bus
    participant Phys as Rapier (Rust)
    participant PID as PID-Core
    participant Spark as SparkJS (WebGPU)

    Note over Phys, Spark: Frame T (16ms)

    par Physics Step
        VLA->>Zenoh: Publish Action (Joints)
        Zenoh->>Phys: Apply Forces
        Phys->>Phys: Step Simulation (dt=1/60)
        Phys->>Zenoh: Publish Object Poses
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

```mermaid
flowchart TD
    Start[Input Embeddings V, D, A] --> Diag[Run Geometry Diagnostics]
    
    subgraph "Step 0: Diagnostics"
        Diag --> ID[Intrinsic Dimension]
        Diag --> Delta[Gromov Delta]
        Diag --> Curve[Local Curvature]
    end
    
    ID --> CheckID{ID < 20?}
    Delta --> CheckHyp{Delta < 0.1?}
    
    CheckHyp -- Yes (Tree-like) --> Hyperbolic[Hyperbolic Pipeline]
    
    CheckID -- No (ID >= 20) --> Fail[NO-GO: Estimator Invalid]
    
    CheckID -- Yes --> CheckFlat{Locally Flat?}
    Curve --> CheckFlat
    
    CheckFlat -- Yes --> Euclidean[Euclidean Pipeline]
    CheckFlat -- No --> Manifold[Manifold Pipeline]
    
    subgraph "Pipelines"
        Hyperbolic --> P1[Hyperbolic Projection]
        P1 --> SI[Shannon Invariants CI/Omega]
        
        Euclidean --> P2[PCA / Random Proj]
        P2 --> KSG[Standard KSG I_sx_intersect]
        
        Manifold --> P3[Isomap / Unrolling]
        P3 --> KSG
    end
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
            Rapier["Rapier3D\n<1ms/step\nRust-native"]
            MuJoCo["MuJoCo\nGold-standard contacts\nFFI bindings"]
            Isaac["Isaac Gym\nGPU parallel\n10k+ envs"]
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
    
    Ready --> Render[Always: Gaussian Splats via SparkJS]
```

### Use Case Decision Tree

```mermaid
flowchart TD
    UseCase[What's your use case?] --> Speed{Need speed?}
    
    Speed -->|Yes, <1ms/step| Rapier[Use: physics.backend = rapier]
    Speed -->|No| Contact{Contact-rich manipulation?}
    
    Contact -->|Yes, precise grasping| MuJoCo[Use: physics.backend = mujoco]
    Contact -->|No| Batch{Large-scale experiments?}
    
    Batch -->|Yes, 10k+ episodes| Isaac[Use: physics.backend = isaac]
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

## 5. Dream2Flow Data Pipeline

Visualizing the specific integration of WAN video generation and 3D flow extraction (`pidsplatspecs.md` §4).

```mermaid
graph LR
    subgraph "Input"
        IMG[Current Image]
        TXT[Instruction]
    end

    subgraph "WAN Generation"
        IMG & TXT --> WAN[WAN 2.2 Model]
        WAN --> VIDEO[Generated Video 2s]
    end

    subgraph "Flow Extraction"
        VIDEO --> SAM[SAM3 Segmentation]
        VIDEO --> DEPTH[Depth-Anything v3]
        VIDEO --> TRACK[CoTracker3]
        
        SAM & DEPTH & TRACK --> LIFT[2D to 3D Lifting]
        LIFT --> TRAJ[3D Flow Trajectory]
    end

    subgraph "Analysis"
        TRAJ --> TARGET{PID Target}
        VLA_EMB[VLA Embeddings] --> SOURCE{PID Source}
        
        SOURCE & TARGET --> EST[PID Estimator]
        EST --> GHOST[Ghost Splat Viz]
    end
```
