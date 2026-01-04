# PID-VLA Experimental Protocols

> **Documentation Cross-Reference**:
> - `grandplan.md` — Master plan and theoretical foundations
> - `pidsplatspecs.md` — Detailed simulation environment and PID specifications
> - `ARCHITECTURE.md` — Component breakdown (Tauri, Modular Physics, 3DGS) and advantages over VLM-based robotics
> - `DIAGRAMS.md` — Visual architecture diagrams
> - `README.md` — Quick start guide

## Detailed Specifications for Reproducible Experiments
 
**Version:** 1.1 (Full 5-Experiment Suite)  
**Date:** 2026-01-03  
**Context:** This document provides exact specifications for reproducing all experiments defined in `grandplan.md` §9, using the PID-Splat environment from `pidsplatspecs.md`.
 
---
 
## Table of Contents
 
1. [Physical Environment Specifications](#1-physical-environment-specifications)
2. [Robot Configuration](#2-robot-configuration)
3. [VLA Model Setup](#3-vla-model-setup)
4. [Experiment 0: Estimator Validation](#4-experiment-0-estimator-validation)
5. [Experiment 1: Pick-and-Place (Baseline)](#5-experiment-1-pick-and-place-baseline)
6. [Experiment 2: Long-Horizon Assembly (Temporal)](#6-experiment-2-long-horizon-assembly-temporal)
7. [Experiment 3: Instruction Perturbation (Robustness)](#7-experiment-3-instruction-perturbation-robustness)
8. [Experiment 4: Dream2Flow Validation (Flow-as-Bridge)](#8-experiment-4-dream2flow-validation-flow-as-bridge)
9. [Experiment 5: Cross-Embodiment (Generalization)](#9-experiment-5-cross-embodiment-generalization)
10. [Perturbation Library](#10-perturbation-library)
11. [Data Formats & Storage](#11-data-formats--storage)
12. [Compute Requirements](#12-compute-requirements)
13. [Reproducibility Checklist](#13-reproducibility-checklist)

---

## Physics and Robot Backend Usage: Modular Architecture

This table clarifies when to use each backend across experiments. While **Rapier** and **Gazebo** are the recommended defaults for performance and industry standards, the system is fully modular and supports **MuJoCo** as a first-class alternative.

| Component | Engine | Purpose | Experiments |
|-----------|--------|---------|-------------|
| **Object Manipulation** | Rapier / MuJoCo | Grasping, stacking, placing objects | Exp 1-5 |
| **Robot Kinematics** | Gazebo / MuJoCo | 7-DOF arm dynamics, joint limits | Exp 1-5 |
| **Sensor Simulation** | Gazebo / MuJoCo | RGB-D cameras, joint encoders | Exp 1-5 |
| **Physical Perturbations** | Rapier / MuJoCo | Mass/friction variations | Exp 1, 3, 5 |
| **Visual Perturbations** | SparkJS Dynos | Lighting, textures | Exp 1 |
| **Cross-Embodiment** | Gazebo / MuJoCo | UR5e vs Franka URDFs | Exp 5 |

### Per-Hypothesis Engine Mapping

| Hypothesis | Primary Engine | Reason |
|------------|----------------|--------|
| **H1** (Synergy → hallucination) | Physics + Robot | Need accurate object poses for PID(V,D;A) |
| **H4** (Memorization vs generalization) | Physics | Perturbation library uses physics for mass/friction |
| **H5** (Temporal synergy degradation) | Physics + Robot | Long-horizon stacking needs precise contact physics |
| **H6** (Safety-aware V-L integration) | Physics + Robot | Collision detection for safety constraints |
| **H7** (Flow-as-bridge) | N/A (WAN video gen) | 3D flow extracted from WAN video, no physics sim needed |

### Modular Physics Backend Configuration

PID-Splat supports swappable physics backends. Select based on your experiment needs:

```toml
# pid-splat.toml - Physics backend configuration

[physics]
backend = "rapier"  # Options: "rapier", "mujoco", "isaac"

# Rapier: Fast iteration, Rust-native, deterministic
[physics.rapier]
step_hz = 1000
deterministic = true
gravity = [0.0, 0.0, -9.81]

# MuJoCo: Gold-standard contact physics, benchmark compatibility
[physics.mujoco]
model_path = "assets/mujoco/franka_tabletop.xml"
step_hz = 500
solver_iterations = 50

# Isaac Gym: GPU-parallel batch experiments
[physics.isaac]
gpu_id = 0
num_envs = 1024
use_gpu_pipeline = true

[robot]
backend = "gazebo"  # Options: "gazebo", "mujoco", "none"
urdf_path = "assets/robots/franka_panda.urdf"
```

**Backend Selection Guide:**

| Use Case | Backend | Rationale |
|----------|---------|----------|
| Fast prototyping | `rapier` | <1ms/step, no external deps |
| Benchmark comparison (LIBERO, MetaWorld) | `mujoco` | Match paper baselines |
| Large-scale ablations (10k+ episodes) | `isaac` | GPU parallelism |
| Accurate robot kinematics | `gazebo` | Industry-standard URDFs |
| Contact-rich manipulation | `mujoco` | Best contact solver |

### When to Use Which

**Use Rapier3D when:**
- Simulating object-object and object-table interactions in a Rust-native environment
- Running many episodes quickly (< 1ms/step)
- Applying physical perturbations (mass, friction)
- Determinism is critical for reproducibility

**Use MuJoCo when:**
- Gold-standard contact physics are required for manipulation
- Comparing results against standard VLA benchmarks (LIBERO, MetaWorld)
- Precise grasping or multi-body dynamics are the focus

**Use Headless Gazebo when:**
- Simulating robot arm kinematics/dynamics via URDF
- Generating sensor data (RGB-D, joint states)
- Testing cross-embodiment (different robot URDFs)
- Industry-standard robot fidelity is required
 
---
 
## 1. Physical Environment Specifications
 
### 1.1 Tabletop Scene (Primary Environment)
 
#### Table
| Property | Value | Notes |
|----------|-------|-------|
| Dimensions | 120cm × 80cm × 75cm | Standard lab table |
| Material | Wood laminate | Friction coefficient μ=0.4 |
| Color | Light oak | For visual contrast |
 
**3DGS Capture Protocol:**
```bash
# Capture
# Device: iPhone 15 Pro with Polycam app
# Method: 360° orbit around table, 2 minutes, 4K @30fps
# Lighting: Diffuse overhead (avoid harsh shadows) 
 
# Training
ns-train splatfacto \
    --data ./captures/table_v1/ \
    --max-num-iterations 30000 \
    --pipeline.model.num-gaussians 800000
 
# Export
ns-export gaussian-splat \
    --load-config outputs/table_v1/splatfacto/config.yml \
    --output-dir ./assets/splats/ \
    --output-format spz
```

**Physics Proxy (Rapier3D):**
```rust
// Table collider - static body
let table_collider = ColliderBuilder::cuboid(0.60, 0.40, 0.375)
    .translation(vector![0.0, 0.0, 0.375])
    .friction(0.4)
    .restitution(0.1)
    .build();

let table_body = RigidBodyBuilder::fixed()
    .translation(vector![0.0, 0.0, 0.0])
    .build();
```

#### Manipulation Objects
| Object ID      | Real Dimensions | Mass | Gaussian Count | Physics Proxy |
| -------------- | --------------- | ---- | -------------- | ------------- |
|  red_cube        | 5×5×5 cm        | 100g | ~15,000        | Cuboid        |
|  blue_cylinder   | r=3cm, h=8cm    | 150g | ~20,000        | Cylinder      |
|  green_sphere    | r=4cm           | 200g | ~25,000        | Ball          |
|  ycb_mustard     | 19×6×6 cm       | 600g | ~40,000        | Convex Hull   |
|  ycb_spam        | 9×8×6 cm        | 350g | ~30,000        | Cuboid        |
|  ycb_bowl        | r=8cm, h=5cm    | 180g | ~35,000        | Trimesh       |
|  blue_plate      | r=10cm, h=1cm   | 250g | ~20,000        | Cylinder      |
|  wooden_block_A  | 10×5×3 cm       | 120g | ~18,000        | Cuboid        |
|  wooden_block_B  | 8×4×4 cm        | 100g | ~16,000        | Cuboid        |

**Object Capture Protocol:**
```bash
# For each object:
# 1. Place on turntable with neutral background
# 2. Capture 360° video (iPhone, 1 minute, 4K)
# 3. Train with object-centric settings:

ns-train splatfacto \
    --data ./captures/red_cube/ \
    --max-num-iterations 15000 \
    --pipeline.model.num-gaussians 20000 \
    --pipeline.model.background-color white
```

**Physics Proxy Definitions:**
```rust
// Red cube
let red_cube_collider = ColliderBuilder::cuboid(0.025, 0.025, 0.025)
    .friction(0.5)
    .restitution(0.2)
    .density(800.0)  // kg/m³, results in ~100g
    .build();
 
// Blue cylinder
let blue_cylinder_collider = ColliderBuilder::cylinder(0.04, 0.03)
    .friction(0.4)
    .restitution(0.15)
    .density(663.0)  // ~150g
    .build();
 
// YCB Mustard - convex hull from mesh
let mustard_mesh = load_obj("./assets/meshes/ycb_mustard.obj");
let mustard_collider = ColliderBuilder::convex_hull(&mustard_mesh.vertices)
    .unwrap()
    .friction(0.3)
    .density(877.0)  // ~600g
    .build();
```

### 1.2 Scene Configurations

**Scene A: Simple Pick-and-Place**
```yaml
# scenes/simple_pick_place.yaml
scene_id: simple_pick_place_v1
environment: tabletop
 
objects:
  - id: red_cube
    splat: assets/splats/red_cube.spz
    initial_pose:
      position: [0.45, 0.10, 0.025]  # x, y, z (meters)
      orientation: [1.0, 0.0, 0.0, 0.0]  # qw, qx, qy, qz
    physics:
      type: cuboid
      half_extents: [0.025, 0.025, 0.025]
      mass: 0.1
 
  - id: blue_plate
    splat: assets/splats/blue_plate.spz
    initial_pose:
      position: [0.45, -0.15, 0.005]
      orientation: [1.0, 0.0, 0.0, 0.0]
    physics:
      type: cylinder
      radius: 0.10
      half_height: 0.005
      fixed: true  # Plate doesn't move
 
  - id: distractor_cylinder
    splat: assets/splats/blue_cylinder.spz
    initial_pose:
      position: [0.55, 0.0, 0.04]
      orientation: [1.0, 0.0, 0.0, 0.0]
    physics:
      type: cylinder
      radius: 0.03
      half_height: 0.04
      mass: 0.15

lighting:
  ambient: 0.3
  directional:
    direction: [-0.5, -0.5, -1.0]
    intensity: 0.7
    color: [1.0, 0.98, 0.95]  # Slightly warm
```

**Scene B: Multi-Object Sorting**
```yaml
# scenes/multi_object_sort.yaml
scene_id: multi_object_sort_v1
environment: tabletop
 
objects:
  # Objects to sort
  - id: red_cube_1
    splat: assets/splats/red_cube.spz
    initial_pose:
      position: [0.40, 0.15, 0.025]
      orientation: [1.0, 0.0, 0.0, 0.0]
    physics: {type: cuboid, half_extents: [0.025, 0.025, 0.025], mass: 0.1}
 
  - id: red_cube_2
    splat: assets/splats/red_cube.spz
    initial_pose:
      position: [0.50, 0.10, 0.025]
      orientation: [0.924, 0.0, 0.0, 0.383]  # 45° rotation
    physics: {type: cuboid, half_extents: [0.025, 0.025, 0.025], mass: 0.1}
 
  - id: blue_cylinder_1
    splat: assets/splats/blue_cylinder.spz
    initial_pose:
      position: [0.45, 0.05, 0.04]
      orientation: [1.0, 0.0, 0.0, 0.0]
    physics: {type: cylinder, radius: 0.03, half_height: 0.04, mass: 0.15}
 
  # Target zones
  - id: red_zone
    splat: assets/splats/red_zone_marker.spz
    initial_pose:
      position: [0.35, -0.20, 0.001]
      orientation: [1.0, 0.0, 0.0, 0.0]
    physics: {type: cylinder, radius: 0.08, half_height: 0.001, fixed: true}
 
  - id: blue_zone
    splat: assets/splats/blue_zone_marker.spz
    initial_pose:
      position: [0.55, -0.20, 0.001]
      orientation: [1.0, 0.0, 0.0, 0.0]
    physics: {type: cylinder, radius: 0.08, half_height: 0.001, fixed: true}
```

**Scene C: Stacking Challenge**
```yaml
# scenes/stacking.yaml
scene_id: stacking_v1
environment: tabletop
 
objects:
  - id: block_base
    splat: assets/splats/wooden_block_A.spz
    initial_pose:
      position: [0.45, 0.0, 0.015]
      orientation: [1.0, 0.0, 0.0, 0.0]
    physics: {type: cuboid, half_extents: [0.05, 0.025, 0.015], mass: 0.12, fixed: true}
 
  - id: block_mid
    splat: assets/splats/wooden_block_B.spz
    initial_pose:
      position: [0.35, 0.15, 0.02]
      orientation: [1.0, 0.0, 0.0, 0.0]
    physics: {type: cuboid, half_extents: [0.04, 0.02, 0.02], mass: 0.1}
 
  - id: block_top
    splat: assets/splats/red_cube.spz
    initial_pose:
      position: [0.55, 0.15, 0.025]
      orientation: [1.0, 0.0, 0.0, 0.0]
    physics: {type: cuboid, half_extents: [0.025, 0.025, 0.025], mass: 0.1}
```

---
 
## 2. Robot Configuration

### 2.1 Franka Emika Panda
| Property          | Value                                       |
| ----------------- | ------------------------------------------- |
| DOF               | 7 joints + 2 finger gripper                 |
| Reach             | 855mm                                       |
| Payload           | 3kg max                                     |
| Repeatability     | ±0.1mm                                      |
| Control Frequency | 1000Hz (internal), 100Hz (external command) |

**URDF Source:**
```bash
# Install Franka ROS packages
sudo apt install ros-humble-franka-description
 
# URDF location
/opt/ros/humble/share/franka_description/robots/panda/panda.urdf.xacro
```

**Joint Limits:**
```python
PANDA_JOINT_LIMITS = {
    "panda_joint1": {"lower": -2.8973, "upper": 2.8973, "velocity": 2.1750, "effort": 87.0},
    "panda_joint2": {"lower": -1.7628, "upper": 1.7628, "velocity": 2.1750, "effort": 87.0},
    "panda_joint3": {"lower": -2.8973, "upper": 2.8973, "velocity": 2.1750, "effort": 87.0},
    "panda_joint4": {"lower": -3.0718, "upper": -0.0698, "velocity": 2.1750, "effort": 87.0},
    "panda_joint5": {"lower": -2.8973, "upper": 2.8973, "velocity": 2.6100, "effort": 12.0},
    "panda_joint6": {"lower": -0.0175, "upper": 3.7525, "velocity": 2.6100, "effort": 12.0},
    "panda_joint7": {"lower": -2.8973, "upper": 2.8973, "velocity": 2.6100, "effort": 12.0},
    "panda_finger_joint1": {"lower": 0.0, "upper": 0.04, "velocity": 0.2, "effort": 20.0},
    "panda_finger_joint2": {"lower": 0.0, "upper": 0.04, "velocity": 0.2, "effort": 20.0},
}
```

**Default Home Configuration:**
```python
PANDA_HOME_JOINTS = [0.0, -0.785, 0.0, -2.356, 0.0, 1.571, 0.785]  # radians
# This places the end-effector roughly at [0.3, 0.0, 0.5] looking down
```

### 2.2 Camera Configuration

**Wrist Camera (Eye-in-Hand)**
```yaml
camera_id: wrist_cam
parent_link: panda_link8
transform:
  position: [0.05, 0.0, 0.05]  # meters, relative to link8
  orientation: [0.5, -0.5, 0.5, -0.5]  # qw,qx,qy,qz - looking forward

intrinsics:
  width: 640
  height: 480
  fx: 462.1  # focal length x (pixels)
  fy: 462.1  # focal length y (pixels)
  cx: 320.0  # principal point x
  cy: 240.0  # principal point y

specs:
  fov_horizontal: 69.4  # degrees
  fov_vertical: 55.0
  framerate: 30
  format: RGB8
```

**Overhead Camera (Global View)**
```yaml
camera_id: overhead_cam
parent_link: world
transform:
  position: [0.45, 0.0, 1.2]  # meters, above table center
  orientation: [0.0, 0.707, 0.707, 0.0]  # looking straight down

intrinsics:
  width: 1280
  height: 720
  fx: 924.3
  fy: 924.3
  cx: 640.0
  cy: 360.0

specs:
  fov_horizontal: 90.0
  fov_vertical: 58.7
  framerate: 30
  format: RGB8
```

**Side Camera (Evaluation/Recording)**
```yaml
camera_id: side_cam
parent_link: world
transform:
  position: [1.0, 0.0, 0.8]
  orientation: [0.653, 0.271, 0.271, 0.653]  # 45° angle

intrinsics:
  width: 1920
  height: 1080
  fx: 1385.5
  fy: 1385.5
  cx: 960.0
  cy: 540.0

specs:
  fov_horizontal: 60.0
  fov_vertical: 36.9
  framerate: 30
  format: RGB8
```

### 2.3 Action Space Definitions

**Action Space A: Joint Velocity Control (Default)**
```python
from dataclasses import dataclass
import numpy as np
 
@dataclass
class JointVelocityAction:
    """7-DOF joint velocities + gripper command"""
    
    joint_velocities: np.ndarray  # shape (7,), rad/s
    gripper_command: float        # 0.0 = close, 1.0 = open
    
    # Normalization ranges (for VLA tokenization)
    JOINT_VEL_MIN: float = -1.0   # rad/s (normalized)
    JOINT_VEL_MAX: float = 1.0
    
    def to_array(self) -> np.ndarray:
        """Flatten to 8-dim vector for VLA"""
        return np.concatenate([self.joint_velocities, [self.gripper_command]])
    
    @classmethod
    def from_array(cls, arr: np.ndarray) -> "JointVelocityAction":
        return cls(
            joint_velocities=arr[:7],
            gripper_command=arr[7]
        )
```

**Action Space B: End-Effector Delta Pose**
```python
@dataclass
class EEDeltaAction:
    """End-effector delta pose + gripper"""
    
    delta_position: np.ndarray    # shape (3,), meters
    delta_rotation: np.ndarray    # shape (3,), axis-angle (radians)
    gripper_command: float        # 0.0 = close, 1.0 = open
    
    # Ranges
    POS_DELTA_MAX: float = 0.05   # meters per step
    ROT_DELTA_MAX: float = 0.1    # radians per step
    
    def to_array(self) -> np.ndarray:
        return np.concatenate([
            self.delta_position,
            self.delta_rotation,
            [self.gripper_command]
        ])
```

### 2.4 Observation Space
```python
@dataclass
class RobotObservation:
    """Complete observation at timestep t"""
    
    # Proprioception
    joint_positions: np.ndarray     # (7,) radians
    joint_velocities: np.ndarray    # (7,) rad/s
    joint_torques: np.ndarray       # (7,) Nm
    
    # End-effector state
    ee_position: np.ndarray         # (3,) meters in world frame
    ee_orientation: np.ndarray      # (4,) quaternion [qw, qx, qy, qz]
    ee_linear_velocity: np.ndarray  # (3,) m/s
    ee_angular_velocity: np.ndarray # (3,) rad/s
    
    # Gripper
    gripper_width: float            # meters, 0.0 to 0.08
    gripper_force: float            # Newtons, 0.0 to 70.0
    gripper_is_grasping: bool       # True if object detected
    
    # Images
    wrist_rgb: np.ndarray           # (480, 640, 3) uint8
    overhead_rgb: np.ndarray        # (720, 1280, 3) uint8
    
    # Timestamp
    timestamp_ns: int               # Nanoseconds since episode start
    
    def get_proprio_vector(self) -> np.ndarray:
        """Flatten proprioception for VLA input"""
        return np.concatenate([
            self.joint_positions,
            self.joint_velocities,
            self.ee_position,
            self.ee_orientation,
            [self.gripper_width, self.gripper_force]
        ])  # (24,)
```

---
 
## 3. VLA Model Setup

### 3.1 OpenVLA (7B) - Primary Model
| Property         | Value                            |
| ---------------- | -------------------------------- |
| Base LLM         | Llama-2 7B                       |
| Vision Encoder   | SigLIP-SO400M + DinoV2-L (fused) |
| Total Parameters | ~7.6B                            |
| Hidden Dimension | 4096                             |
| Action Bins      | 256 per dimension                |
| Context Length   | 2048 tokens                      |

**Model Loading (MLX on Apple Silicon):**
```python
import mlx.core as mx
from openvla import OpenVLAModel, OpenVLAProcessor
 
# Load model
model = OpenVLAModel.from_pretrained(
    "openvla/openvla-7b",
    dtype=mx.float16,
    device="mps"
)
processor = OpenVLAProcessor.from_pretrained("openvla/openvla-7b")
 
# Inference
def run_inference(image: np.ndarray, instruction: str) -> Tuple[np.ndarray, dict]:
    inputs = processor(
        images=image,
        text=instruction,
        return_tensors="mlx"
    )
    
    with mx.no_grad():
        outputs = model(**inputs, output_hidden_states=True)
    
    # Extract action tokens
    action_tokens = outputs.action_logits.argmax(dim=-1)  # (8,)
    
    # Extract embeddings for PID
    embeddings = {
        "V": outputs.vision_hidden_states[-1].mean(dim=1),  # (1, 1024)
        "L": outputs.language_hidden_states[-1].mean(dim=1),  # (1, 4096)
        "D": outputs.fused_hidden_states[-1],  # (1, 4096)
    }
    
    # Detokenize action
    action = detokenize_action(action_tokens.numpy())
    
    return action, embeddings
```

**Action Tokenization:**
```python
def tokenize_action(continuous: np.ndarray, n_bins: int = 256) -> np.ndarray:
    """
    Convert continuous action [-1, 1] to discrete tokens [0, 255].
    
    OpenVLA uses uniform binning across the action range.
    """
    # Clip to valid range
    clipped = np.clip(continuous, -1.0, 1.0)
    
    # Map to [0, n_bins-1]
    tokens = ((clipped + 1.0) / 2.0 * (n_bins - 1)).astype(np.int32)
    
    return tokens
 
def detokenize_action(tokens: np.ndarray, n_bins: int = 256) -> np.ndarray:
    """
    Convert discrete tokens [0, 255] back to continuous action [-1, 1].
    """
    # Map to bin centers
    continuous = (tokens.astype(np.float32) / (n_bins - 1)) * 2.0 - 1.0
    
    return continuous
```

### 3.2 Embedding Extraction Points
| Layer Name                 | Tensor Shape       | Description             | Use in PID      |
| -------------------------- | ------------------ | ----------------------- | --------------- |
|  vision_encoder.patch_embed  | (B, 256, 1024)     | Raw ViT patch tokens    | —               |
| `vision_encoder.output` | (n_patches, 1024) | Post-attention visual tokens | `vla/emb/vision` |
| `language_encoder.output` | (n_tokens, 4096) | Instruction embedding | `vla/emb/language` |
| `llm.residual_pre_attn` | (1, 4096) | Pre-attention state (Mitigates RoPE) | `vla/emb/clean_d` |
| `fusion_layer.output` | (1, 4096) | Fused V+L representation | `vla/emb/fused` |
|  action_head.input           | (B, 1, 4096)       | Pre-action hidden state | D (World Model) |
|  action_head.logits          | (B, 8, 256)        | Action distribution     | A (argmax)      |

**Extraction Hook:**
```python
class EmbeddingExtractor:
    def __init__(self, model):
        self.model = model
        self.embeddings = {}
        
        # Register hooks
        model.vision_encoder.register_forward_hook(
            lambda m, inp, out: self.embeddings.update({"V_raw": out})
        )
        model.language_encoder.register_forward_hook(
            lambda m, inp, out: self.embeddings.update({"L_raw": out})
        )
        model.action_head.register_forward_hook(
            lambda m, inp, out: self.embeddings.update({"D_raw": inp[0]})
        )
    
    def get_pid_embeddings(self) -> dict:
        """Return processed embeddings for PID computation"""
        return {
            "V": self.embeddings["V_raw"].mean(dim=1).squeeze().numpy(),  # (1024,)
            "L": self.embeddings["L_raw"].mean(dim=1).squeeze().numpy(),  # (4096,)
            "D": self.embeddings["D_raw"].squeeze().numpy(),              # (4096,)
        }
```

### 3.3 Dimensionality Reduction for PID
Since raw embeddings are high-dimensional (1024-4096), we reduce before PID:
```python
from sklearn.decomposition import PCA
from pid_core import Standardizer, HashProjector
 
class EmbeddingReducer:
    """Reduce embedding dimensions for tractable PID estimation"""
    
    def __init__(self, target_dim: int = 64, method: str = "pca"):
        self.target_dim = target_dim
        self.method = method
        self.fitted = False
        
        if method == "pca":
            self.reducer_V = PCA(n_components=target_dim)
            self.reducer_L = PCA(n_components=target_dim)
            self.reducer_D = PCA(n_components=target_dim)
        elif method == "hash":
            # Deterministic random projection
            self.reducer_V = HashProjector(1024, target_dim, seed=0xA11CE)
            self.reducer_L = HashProjector(4096, target_dim, seed=0xB22CE)
            self.reducer_D = HashProjector(4096, target_dim, seed=0xC33CE)
    
    def fit(self, V_batch: np.ndarray, L_batch: np.ndarray, D_batch: np.ndarray):
        """Fit reducers on calibration data"""
        if self.method == "pca":
            self.reducer_V.fit(V_batch)
            self.reducer_L.fit(L_batch)
            self.reducer_D.fit(D_batch)
        self.fitted = True
    
    def transform(self, V: np.ndarray, L: np.ndarray, D: np.ndarray) -> tuple:
        """Reduce dimensions"""
        V_reduced = self.reducer_V.transform(V.reshape(1, -1)).flatten()
        L_reduced = self.reducer_L.transform(L.reshape(1, -1)).flatten()
        D_reduced = self.reducer_D.transform(D.reshape(1, -1)).flatten()
        return V_reduced, L_reduced, D_reduced
```

---
 
## 4. Experiment 0: Estimator Validation
Purpose: Validate that PID estimators work at the target dimensions before running real experiments.

### 4.0 Geometry Validation Gate (REQUIRED)

**Critical**: Before running PID estimation on VLA embeddings, you MUST validate the geometric assumptions.

#### Why This Matters

The KSG/ISX estimators assume Euclidean-like geometry (specifically Chebyshev/L∞). VLA embeddings may lie on curved manifolds (hyperbolic, Lorentzian) where these estimators produce invalid results.

#### Geometry Diagnostics

| Diagnostic | Method | Pass Criterion | Interpretation | Action if Fail |
|------------|--------|----------------|----------------|----------------|
| **Intrinsic Dimension** | Levina-Bickel (MLE) | d_intrinsic < 20 | Lower = better for KSG | Reduce dimension further |
| **δ-Hyperbolicity** | Gromov 4-point sampling | δ > 0.1 | Higher δ = more Euclidean-like; lower δ = more tree-like/hyperbolic | Use Flow-as-Bridge workaround |
| **Distance Concentration** | CV of pairwise distances | CV > 0.1 | Higher CV = healthier distance spread | Reduce dimension or increase N |

#### Running Geometry Diagnostics

```python
from pid_core import intrinsic_dimension_levina_bickel, estimate_gromov_delta, distance_concentration_stats

# 1. Check intrinsic dimension
d_hat = intrinsic_dimension_levina_bickel(embeddings, k=5)
if d_hat > 20:
    print(f"WARNING: Intrinsic dimension {d_hat:.1f} too high for KSG")
    # ACTION: Apply more aggressive PCA or use Flow-as-Bridge

# 2. Check hyperbolicity
delta = estimate_gromov_delta(embeddings, n_samples=1000)
if delta < 0.1:
    print(f"WARNING: Data is tree-like (δ={delta:.3f}), hyperbolic geometry suspected")
    # ACTION: Use Flow-as-Bridge (3D Euclidean target) instead of raw embeddings

# 3. Check distance concentration
stats = distance_concentration_stats(embeddings)
if stats.pairwise_cv < 0.1:
    print(f"WARNING: Distance concentration detected (CV={stats.pairwise_cv:.3f})")
    # ACTION: Reduce dimension or collect more samples
```

#### Hyperbolic/Lorentzian Limitation

> **⚠️ IMPORTANT**: The validated ISX estimator (`EhrlichKsg`) **only supports Chebyshev (L∞) metric**. Hyperbolic/Lorentzian PID estimation is NOT currently supported.
>
> **Workaround**: Use **Flow-as-Bridge** (see Experiment 4, §8). By using 3D Object Flow as the PID target instead of high-dimensional embeddings, you sidestep manifold geometry issues entirely because 3D flow lives in Euclidean R³.

#### Geometry Gate Pass/Fail

| Check | Pass Criterion | Interpretation | Fail Action |
|-------|----------------|----------------|-------------|
| Intrinsic Dim | < 20 | Low intrinsic dim = KSG works | Reduce to d < 20 via PCA |
| δ-hyperbolicity | > 0.1 | High δ = Euclidean-like geometry | Use Flow-as-Bridge |
| Distance CV | > 0.1 | High CV = good distance spread | Increase N or reduce d |
| **All three pass** | All criteria met | Geometry is KSG-compatible | Fix geometry issues first |

### 4.1 Synthetic Test Cases
```python
# Test configurations
EXP0_CONFIGS = [
    # (n_samples, dimension, scenario)
    (500, 10, "independent_additive"),
    (500, 10, "redundant_copy"),
    (500, 10, "unique_s1"),
    (500, 10, "xor_synergy"),
    (1000, 64, "independent_additive"),   # Target PCA dimension
    (1000, 64, "redundant_copy"),
    (1000, 64, "xor_synergy"),
    (2000, 256, "independent_additive"),  # Stress test
]
 
def generate_synthetic_data(n: int, d: int, scenario: str, noise: float = 0.05):
    """Generate synthetic data with known PID ground truth"""
    
    rng = np.random.default_rng(seed=42)
    
    if scenario == "independent_additive":
        # T = S1 + S2 + noise
        # Expected: Red ≈ 0, Unq1 ≈ Unq2 ≈ I(S;T)/2, Syn ≈ 0
        S1 = rng.normal(0, 1, (n, d))
        S2 = rng.normal(0, 1, (n, d))
        T = S1[:, :1] + S2[:, :1] + noise * rng.normal(0, 1, (n, 1))
        
    elif scenario == "redundant_copy":
        # T = S1[0] = S2[0] + noise
        # Expected: Red ≈ I(S1;T), Unq1 ≈ Unq2 ≈ 0, Syn ≈ 0
        base = rng.normal(0, 1, (n, 1))
        S1 = np.concatenate([base + noise * rng.normal(0, 1, (n, 1)),
                            rng.normal(0, 1, (n, d-1))], axis=1)
        S2 = np.concatenate([base + noise * rng.normal(0, 1, (n, 1)),
                            rng.normal(0, 1, (n, d-1))], axis=1)
        T = base + noise * rng.normal(0, 1, (n, 1))
        
    elif scenario == "unique_s1":
        # T = S1[0] + noise, S2 independent
        # Expected: Red ≈ 0, Unq1 ≈ I(S1;T), Unq2 ≈ 0, Syn ≈ 0
        S1 = rng.normal(0, 1, (n, d))
        S2 = rng.normal(0, 1, (n, d))
        T = S1[:, :1] + noise * rng.normal(0, 1, (n, 1))
        
    elif scenario == "xor_synergy":
        # T = sign(S1[0] * S2[0]) + noise
        # Expected: Red ≈ 0, Unq1 ≈ Unq2 ≈ 0, Syn > 0
        S1 = rng.normal(0, 1, (n, d))
        S2 = rng.normal(0, 1, (n, d))
        xor_signal = np.sign(S1[:, :1] * S2[:, :1])
        T = xor_signal + noise * rng.normal(0, 1, (n, 1))
    
    return S1, S2, T
```

### 4.2 Acceptance Criteria (from grandplan.md §9.1)
| Dimension            | Max Relative Error | Decision               |
| -------------------- | ------------------ | ---------------------- |
| d ≤ 10               | < 5%               | GO                     |
| d ≤ 100              | < 10%              | GO                     |
| d ≤ 256              | < 15%              | GO with caution        |
| d > 256              | < 20%              | PIVOT to dim reduction |
| d > 256, error > 20% | —                  | NO-GO                  |

### 4.3 Running Experiment 0
```bash
# Rust-native runner (fast)
# cargo run -p pid-core --bin exp0 -- --csv > results/exp0_rust.csv
 
# Full validation with diagnostics
# python experiments/exp0_full.py \
#     --configs experiments/configs/exp0.yaml \
#     --output results/exp0_full/ \
#     --seed 42
```

---
 
## 5. Experiment 1: Pick-and-Place (Baseline)
Hypothesis Tested: H1 (Negative synergy indicates subadditive information/potential hallucination), H2 (Modality contribution)

### 5.1 Task Definition
| Property               | Value                                                        |
| ---------------------- | ------------------------------------------------------------ |
| Task                   | Pick object A, place on target B                             |
| Instruction Format     | "Pick up the {color} {object} and place it on the {target}." |
| Success Criteria       | Object center within 2cm of target center, stable for 1s     |
| Timeout                | 60 seconds                                                   |
| Episodes per condition | 100                                                          |

### 5.2 Experimental Conditions
```yaml
# experiments/configs/exp1_pick_place.yaml
experiment_id: exp1_pick_place_v1
 
conditions:
  - name: baseline
    scene: scenes/simple_pick_place.yaml
    instruction: "Pick up the red cube and place it on the blue plate."
    perturbations: []
    n_episodes: 100
    
  - name: lighting_variation
    scene: scenes/simple_pick_place.yaml
    instruction: "Pick up the red cube and place it on the blue plate."
    perturbations:
      - type: lighting
        params: {intensity_range: [0.3, 1.0], color_temp_range: [3000, 6500]}
    n_episodes: 100
    
  - name: distractor_objects
    scene: scenes/simple_pick_place.yaml
    instruction: "Pick up the red cube and place it on the blue plate."
    perturbations:
      - type: add_distractors
        params: {count: 3, objects: [blue_cylinder, green_sphere, ycb_spam]}
    n_episodes: 100
    
  - name: novel_instruction
    scene: scenes/simple_pick_place.yaml
    instruction: "Grasp the crimson block and set it down on the azure dish."
    perturbations: []
    n_episodes: 100
 
data_collection:
  cameras: [wrist_cam, overhead_cam]
  framerate: 30
  save_embeddings: true
  embedding_rate: 5  # Hz
  save_actions: true
  save_proprioception: true
```

### 5.3 Episode Data Structure
```python
@dataclass
class PickPlaceEpisode:
    """Complete data for one pick-and-place episode"""
    
    # Metadata
    episode_id: str
    condition: str
    scene_config: str
    instruction: str
    seed: int
    
    # Outcome
    success: bool
    failure_mode: Optional[str]  # "miss_grasp", "drop", "miss_place", "timeout", "collision"
    completion_time: float       # seconds
    
    # Trajectory (T timesteps at 30Hz)
    timestamps: np.ndarray                # (T,)
    images_wrist: np.ndarray              # (T, 480, 640, 3) uint8
    images_overhead: np.ndarray           # (T, 720, 1280, 3) uint8
    
    # Robot state
    joint_positions: np.ndarray           # (T, 7)
    joint_velocities: np.ndarray          # (T, 7)
    ee_poses: np.ndarray                  # (T, 7) [x,y,z,qw,qx,qy,qz]
    gripper_widths: np.ndarray            # (T,)
    
    # Actions
    actions_commanded: np.ndarray         # (T, 8)
    action_tokens: np.ndarray             # (T, 8) int
    
    # Embeddings (at 5Hz = T/6 samples)
    embeddings_V: np.ndarray              # (T/6, 1024)
    embeddings_L: np.ndarray              # (T/6, 4096)
    embeddings_D: np.ndarray              # (T/6, 4096)
    
    # Reduced embeddings (at 5Hz)
    embeddings_V_reduced: np.ndarray      # (T/6, 64)
    embeddings_L_reduced: np.ndarray      # (T/6, 64)
    embeddings_D_reduced: np.ndarray      # (T/6, 64)
    
    # Object tracking
    object_poses: Dict[str, np.ndarray]   # object_id -> (T, 7)
    grasp_events: List[Tuple[float, str]] # [(time, object_id), ...]
    
    # PID metrics (computed post-hoc)
    # Target: Action (Tests H1/H2 - Policy Integration)
    pid_action_synergy: np.ndarray        # (T/6,)
    pid_action_redundancy: np.ndarray     # (T/6,)
    pid_action_unique_v: np.ndarray       # (T/6,)
    pid_action_unique_d: np.ndarray       # (T/6,)
    
    # Target: 3D Flow (Tests H7 - World Model Consistency)
    pid_flow_synergy: np.ndarray          # (T/6,)
    pid_flow_redundancy: np.ndarray       # (T/6,)
    pid_flow_unique_v: np.ndarray         # (T/6,)
    pid_flow_unique_d: np.ndarray         # (T/6,)
    
    pid_co_information: np.ndarray        # (T/6,)
```

### 5.4 PID Computation
```python
from pid_core import pid2_isx, MatRef, Pid2Config, KsgConfig, IsxConfig, IsxMethod
 
def compute_episode_pid(episode: PickPlaceEpisode, window_size: int = 20) -> dict:
    """
    Compute PID metrics over sliding windows.
    
    Computes two decompositions:
    1. Target = Action (Tests H1: Does integration predict policy success?)
    2. Target = Flow   (Tests H7: Is the world model consistent with physics?)
    
    Warning: Window size of 20 samples is small for high-dim PID. 
    Ensure embeddings are aggressively reduced (e.g. PCA to 8-16 dims) for stability
    or use larger windows/whole-episode analysis.
    """
    n_samples = len(episode.embeddings_V_reduced)
    
    # Prepare targets
    # 1. Action (Joint Velocities) - Downsampled to 5Hz
    actions = episode.actions_commanded[::6]
    
    # 2. 3D Flow (Object Position)
    target_object = "red_cube"
    flow_3d = episode.object_poses[target_object][:, :3]
    flow_3d = flow_3d[::6]
    
    # Standardize inputs (Z-score)
    V = Standardizer.fit_transform(episode.embeddings_V_reduced)
    D = Standardizer.fit_transform(episode.embeddings_D_reduced)
    A = Standardizer.fit_transform(actions)
    T = Standardizer.fit_transform(flow_3d)
    
    # TODO: Insert Geometry Diagnostics here (Intrinsic Dim / Delta-Hyp)
    # if intrinsic_dim(V) > threshold: warn("Manifold violation")
    
    # Config
    cfg = Pid2Config(
        ksg=KsgConfig(k=3, metric="chebyshev"),
        isx=IsxConfig(method=IsxMethod.EhrlichKsg)
    )
    
    results = {
        "action": {"syn": [], "red": [], "unq_v": [], "unq_d": []},
        "flow":   {"syn": [], "red": [], "unq_v": [], "unq_d": []}
    }
    
    for i in range(0, n_samples - window_size, window_size // 2):
        # Window slicing
        win_V = MatRef(V[i:i+window_size])
        win_D = MatRef(D[i:i+window_size])
        win_A = MatRef(A[i:i+window_size])
        win_T = MatRef(T[i:i+window_size])
        
        # 1. PID(V, D -> Action)
        res_a = pid2_isx(win_V, win_D, win_A, cfg)
        results["action"]["syn"].append(res_a.synergy)
        results["action"]["red"].append(res_a.redundancy)
        results["action"]["unq_v"].append(res_a.unique_s1)
        results["action"]["unq_d"].append(res_a.unique_s2)
        
        # 2. PID(V, D -> Flow)
        res_f = pid2_isx(win_V, win_D, win_T, cfg)
        results["flow"]["syn"].append(res_f.synergy)
        results["flow"]["red"].append(res_f.redundancy)
        results["flow"]["unq_v"].append(res_f.unique_s1)
        results["flow"]["unq_d"].append(res_f.unique_s2)
    
    return {
        "pid_action_synergy": np.array(results["action"]["syn"]),
        "pid_action_redundancy": np.array(results["action"]["red"]),
        "pid_action_unique_v": np.array(results["action"]["unq_v"]),
        "pid_action_unique_d": np.array(results["action"]["unq_d"]),
        "pid_flow_synergy": np.array(results["flow"]["syn"]),
        "pid_flow_redundancy": np.array(results["flow"]["red"]),
        "pid_flow_unique_v": np.array(results["flow"]["unq_v"]),
        "pid_flow_unique_d": np.array(results["flow"]["unq_d"]),
    }


### 5.5 Evaluation Metrics
```python
def evaluate_exp1(episodes: List[PickPlaceEpisode]) -> dict:
    """Compute all evaluation metrics for Experiment 1"""
    
    # Basic performance
    success_rate = np.mean([e.success for e in episodes])
    
    # Separate by outcome
    success_eps = [e for e in episodes if e.success]
    failure_eps = [e for e in episodes if not e.success]
    
    # PID metrics (Using ACTION synergy for failure prediction H1)
    syn_success = np.concatenate([e.pid_action_synergy for e in success_eps])
    syn_failure = np.concatenate([e.pid_action_synergy for e in failure_eps])
    
    # Hypothesis H1: Negative synergy predicts failure
    # Use synergy at 50% of episode as predictor
    synergy_midpoint = []
    labels = []
    for e in episodes:
        mid_idx = len(e.pid_action_synergy) // 2
        synergy_midpoint.append(e.pid_action_synergy[mid_idx])
        labels.append(1 if e.success else 0)
    
    # AUROC: can synergy predict success?
    from sklearn.metrics import roc_auc_score
    auroc_synergy = roc_auc_score(labels, synergy_midpoint)
    
    # Statistical tests
    from scipy.stats import mannwhitneyu, ttest_ind
    stat, pvalue = mannwhitneyu(syn_success, syn_failure, alternative='greater')
    
    return {
        "success_rate": success_rate,
        "mean_synergy_success": np.mean(syn_success),
        "mean_synergy_failure": np.mean(syn_failure),
        "auroc_synergy_predicts_success": auroc_synergy,
        "mannwhitney_pvalue": pvalue,
        "n_success": len(success_eps),
        "n_failure": len(failure_eps),
    }
```

---
 
## 6. Experiment 2: Long-Horizon Assembly (Temporal)
Hypothesis Tested: H5 (Compositional Failure Correlates with Temporal Synergy Degradation)

### 6.1 Task Definition
| Property         | Value                                                            |
| ---------------- | ---------------------------------------------------------------- |
| Task             | Stack 3 blocks in specified order                                |
| Instruction      | "Stack the blocks: red on bottom, blue in middle, green on top." |
| Subtasks         | 6 phases (grasp1, place1, grasp2, place2, grasp3, place3)        |
| Success Criteria | All 3 blocks stacked, stable for 2s, correct order               |
| Timeout          | 180 seconds                                                      |

### 6.2 Scene Configuration
```yaml
# scenes/stacking_3block.yaml
scene_id: stacking_3block_v1
 
objects:
  - id: red_block
    splat: assets/splats/red_cube.spz
    initial_pose: {position: [0.35, 0.15, 0.025], orientation: [1,0,0,0]}
    physics: {type: cuboid, half_extents: [0.025, 0.025, 0.025], mass: 0.1}
    
  - id: blue_block  
    splat: assets/splats/blue_cube.spz
    initial_pose: {position: [0.45, 0.20, 0.025], orientation: [1,0,0,0]}
    physics: {type: cuboid, half_extents: [0.025, 0.025, 0.025], mass: 0.1}
    
  - id: green_block
    splat: assets/splats/green_cube.spz
    initial_pose: {position: [0.55, 0.15, 0.025], orientation: [1,0,0,0]}
    physics: {type: cuboid, half_extents: [0.025, 0.025, 0.025], mass: 0.1}
    
  - id: target_base
    splat: assets/splats/target_marker.spz
    initial_pose: {position: [0.45, -0.10, 0.001], orientation: [1,0,0,0]}
    physics: {type: cylinder, radius: 0.04, half_height: 0.001, fixed: true}
```

### 6.3 Phase Detection
```python
def detect_task_phases(episode: StackingEpisode) -> List[Tuple[float, float, str]]:
    """
    Detect subtask phases from trajectory.
    
    Returns: [(start_time, end_time, phase_name), ...]"""
    phases = []
    
    # Phase detection based on gripper state and object contacts
    gripper_widths = episode.gripper_widths
    object_heights = {
        obj_id: poses[:, 2] for obj_id, poses in episode.object_poses.items()
    }
    
    # Detect grasp events (gripper closing while object rising)
    for obj_id in ["red_block", "blue_block", "green_block"]:
        heights = object_heights[obj_id]
        
        for t in range(1, len(heights)):
            # Grasp start: gripper closing + object starts rising
            if (gripper_widths[t] < gripper_widths[t-1] - 0.005 and
                heights[t] > heights[t-1] + 0.01):
                grasp_start = episode.timestamps[t]
                
            # Place end: gripper opening + object stationary
            if (gripper_widths[t] > gripper_widths[t-1] + 0.01 and
                abs(heights[t] - heights[t-1]) < 0.001):
                place_end = episode.timestamps[t]
                phases.append((grasp_start, place_end, f"manipulate_{obj_id}"))
    
    return sorted(phases, key=lambda x: x[0])
```

### 6.4 Temporal PID Analysis
```python
def analyze_temporal_pid(episode: StackingEpisode) -> dict:
    """
    Analyze how PID metrics evolve across task phases. 
    
    Tests H5: Does synergy degrade over time in long-horizon tasks?
    """
    phases = detect_task_phases(episode)
    
    phase_metrics = []
    for start, end, phase_name in phases:
        # Get PID values in this phase
        mask = (episode.pid_timestamps >= start) & (episode.pid_timestamps <= end)
        
        phase_metrics.append({
            "phase": phase_name,
            "start_time": start,
            "end_time": end,
            "mean_synergy": episode.pid_synergy[mask].mean(),
            "std_synergy": episode.pid_synergy[mask].std(),
            "mean_redundancy": episode.pid_redundancy[mask].mean(),
            "synergy_trend": np.polyfit(
                episode.pid_timestamps[mask],
                episode.pid_synergy[mask],
                deg=1
            )[0],  # Slope: positive = increasing, negative = degrading
        })
    
    # Cross-phase analysis
    synergy_by_phase_order = [m["mean_synergy"] for m in phase_metrics]
    
    return {
        "phase_metrics": phase_metrics,
        "overall_synergy_trend": np.polyfit(
            range(len(synergy_by_phase_order)),
            synergy_by_phase_order,
            deg=1
        )[0],
        "first_phase_synergy": synergy_by_phase_order[0] if synergy_by_phase_order else None,
        "last_phase_synergy": synergy_by_phase_order[-1] if synergy_by_phase_order else None,
    }
```

---

## 7. Experiment 3: Instruction Perturbation (Robustness)
Hypothesis Tested: **H6** (Safety-Aware Behavior Requires Specific V-L Integration) & General Robustness

### 7.1 Task Definition
| Property | Value |
|----------|-------|
| Task | Pick-and-Place (same as Exp1) |
| Variations | Instructions are semantically equivalent but syntactically distinct |
| Goal | Measure robustness of V-L-A PID signatures to language variation |

### 7.2 Experimental Conditions
```yaml
experiment_id: exp3_instruction_robustness
conditions:
  - name: baseline
    instruction: "Pick up the red cube."
  - name: verbose
    instruction: "I would like you to grasp the small red cube located on the table."
  - name: safety_constraint
    instruction: "Carefully pick up the red cube without hitting the blue cylinder."
  - name: abstract
    instruction: "Retrieve the crimson object."
```

### 7.3 PID Metric Analysis
**Focus:** Compare `Unq(L)` (Unique Language Information) and `Syn(V,L;A)` across conditions.
**Prediction:** Safety constraints should increase `Unq(L)` or `Syn(V,L;A)` compared to baseline if the model is attending to the constraint.

---
 
## 8. Experiment 4: Dream2Flow Validation (Flow-as-Bridge)
Hypothesis Tested: **H7** (3D Object Flow serves as an embodiment-agnostic integration diagnostic)

### 8.1 WAN Video Generation Setup
```python
# WAN 2.2 configuration
WAN_CONFIG = {
    "model": "wan-2.2-base",
    "num_frames": 48,           # 2 seconds at 24fps
    "width": 1280,
    "height": 720,
    "guidance_scale": 7.5,
    "num_inference_steps": 50,
    "seed": None,               # Set per-episode
}
 
def generate_dream_video(
    current_image: np.ndarray,
    instruction: str,
    seed: int
) -> np.ndarray:
    """
    Generate a 2-second video prediction using WAN.
    
    Returns: (48, 720, 1280, 3) uint8 array
    """
    from wan import WanPipeline
    
    pipe = WanPipeline.from_pretrained("wan-ai/wan-2.2-base")
    
    # Format prompt for video generation
    prompt = f"A robot arm {instruction}. Smooth motion, overhead camera view, realistic lighting."
    
    video = pipe(
        prompt=prompt,
        image=current_image,
        num_frames=WAN_CONFIG["num_frames"],
        width=WAN_CONFIG["width"],
        height=WAN_CONFIG["height"],
        guidance_scale=WAN_CONFIG["guidance_scale"],
        num_inference_steps=WAN_CONFIG["num_inference_steps"],
        generator=torch.Generator().manual_seed(seed),
    ).frames[0]
    
    return np.array(video)
```

### 8.2 3D Flow Extraction Pipeline
```python
def extract_3d_flow(
    video: np.ndarray,
    object_prompts: List[str],
    camera_intrinsics: np.ndarray
) -> Dict[str, np.ndarray]:
    """
    Extract 3D flow trajectories from generated video.
    
    Pipeline: SAM3 segmentation -> CoTracker3 tracking -> Depth lifting
    
    Returns: {object_name: (T, 3) array of 3D positions}
    """
    from segment_anything_3 import SAM3
    from cotracker3 import CoTracker3
    from depth_anything_v3 import DepthAnythingV3
    
    # 1. Segment objects in first frame
    sam = SAM3()
    first_frame = video[0]
    masks = {}
    for prompt in object_prompts:
        mask = sam.predict(first_frame, text_prompt=prompt)
        masks[prompt] = mask
    
    # 2. Get initial centroids
    initial_points = {}
    for name, mask in masks.items():
        y, x = np.where(mask)
        initial_points[name] = np.array([x.mean(), y.mean()])
    
    # 3. Track through video with CoTracker3
    tracker = CoTracker3()
    queries = np.stack(list(initial_points.values()))  # (n_objects, 2)
    tracks_2d, confidence = tracker.track(
        video=video,
        queries=queries,
        backward_tracking=False
    )  # tracks_2d: (n_objects, T, 2)
    
    # 4. Estimate depth for each frame
    # Note: Use DepthAnythingV3 for general scenes. 
    # Use DKT (Diffusion Knows Transparency) if scene contains glass/plastic (per grandplan §10.4.3).
    depth_model = DepthAnythingV3()
    depths = np.stack([depth_model(frame) for frame in video])  # (T, H, W)
    
    # 5. Lift 2D tracks to 3D
    K = camera_intrinsics  # (3, 3)
    fx, fy = K[0, 0], K[1, 1]
    cx, cy = K[0, 2], K[1, 2]
    
    flows_3d = {}
    for i, name in enumerate(initial_points.keys()):
        track_2d = tracks_2d[i]  # (T, 2)
        trajectory_3d = []
        
        for t in range(len(track_2d)):
            u, v = track_2d[t]
            z = depths[t, int(v), int(u)]
            
            # Unproject
            x = (u - cx) * z / fx
            y = (v - cy) * z / fy
            
            trajectory_3d.append([x, y, z])
        
        flows_3d[name] = np.array(trajectory_3d)
    
    return flows_3d
```

### 8.3 Dream2Flow PID Computation
```python
@dataclass
class DreamFlowTrajectory:
    """3D flow trajectory extracted from WAN-generated video"""
    object_id: str
    points: np.ndarray         # (T, 3)
    confidence: np.ndarray     # (T,)
    source_video_seed: int
 
def compute_dream_pid(
    V_embeddings: np.ndarray,   # (T, dim_v)
    D_embeddings: np.ndarray,   # (T, dim_d)
    dream_flow: DreamFlowTrajectory,
    reducer: EmbeddingReducer,
) -> dict:
    """
    Compute PID using 3D flow as target: I^sx(V, D; Flow)
    
    This sidesteps manifold geometry issues because Flow is 3D Euclidean.
    """
    # Reduce embeddings
    V_reduced = reducer.transform_batch(V_embeddings, "V")  # (T, 64)
    D_reduced = reducer.transform_batch(D_embeddings, "D")  # (T, 64)
    
    # Resample flow to match embedding timestamps
    flow_resampled = resample_trajectory(
        dream_flow.points,
        source_fps=24,
        target_fps=5,
        target_len=len(V_reduced)
    )  # (T, 3)
    
    # Standardize
    V_std = Standardizer.fit_transform(V_reduced)
    D_std = Standardizer.fit_transform(D_reduced)
    flow_std = Standardizer.fit_transform(flow_resampled)
    
    # Compute PID
    cfg = Pid2Config(ksg=KsgConfig(k=3), isx=IsxConfig(method=IsxMethod.EhrlichKsg))
    
    result = pid2_isx(
        MatRef(V_std),
        MatRef(D_std),
        MatRef(flow_std),
        cfg
    )
    
    return {
        "synergy": result.synergy,
        "redundancy": result.redundancy,
        "unique_v": result.unique_s1,
        "unique_d": result.unique_s2,
        "total_mi": result.redundancy + result.unique_s1 + result.unique_s2 + result.synergy,
    }
```

---

## 9. Experiment 5: Cross-Embodiment (Generalization)
Hypothesis Tested: **H7** (Embodiment Gap separation)

### 9.1 Protocol
Compare PID signatures on the **same task** performed by two different robots (Franka Panda vs. UR5e) with the **same VLA policy** (using cross-embodiment training data or adapters).

### 9.2 Key Comparison
Compute `Syn(V, D; A_robot)` for both robots.
*   **Prediction:** `Syn(V, D; Flow)` (World Model) should be similar across embodiments. `Syn(D; A_robot)` should vary if one embodiment is less familiar to the policy.

---
 
## 10. Perturbation Library

### 10.1 Visual Perturbations
```python
class VisualPerturbations:
    """Real-time visual perturbations via SparkJS Dynos"""
    
    @staticmethod
    def lighting_shift(scene, intensity_delta: float, color_temp_k: int):
        """
        Shift scene lighting. 
        
        Args:
            intensity_delta: -0.5 to 0.5 (relative change)
            color_temp_k: 2700 (warm) to 6500 (cool)
        """
        # Convert color temp to RGB multipliers
        rgb = kelvin_to_rgb(color_temp_k)
        
        scene.set_lighting(
            ambient=scene.ambient + intensity_delta,
            directional_color=rgb,
        )
    
    @staticmethod
    def add_distractor(scene, object_type: str, position: np.ndarray):
        """Add a distractor object to the scene"""
        distractor = scene.spawn_object(
            splat=f"assets/splats/{object_type}.spz",
            position=position,
            physics=True,
        )
        return distractor.id
    
    @staticmethod
    def texture_swap(scene, target_id: str, new_texture: str):
        """Swap object texture (requires variant splats)"""
        scene.objects[target_id].set_splat(f"assets/splats/{target_id}_{new_texture}.spz")
    
    @staticmethod  
    def add_occlusion(scene, position: np.ndarray, size: float):
        """Add floating occluder between camera and workspace"""
        scene.spawn_object(
            splat="assets/splats/black_panel.spz",
            position=position,
            scale=size,
            physics=False,  # Floating
        )
```

### 10.2 Physical Perturbations
```python
class PhysicalPerturbations:
    """Physics-based perturbations via Rapier"""
    
    @staticmethod
    def mass_variation(physics_world, object_id: str, scale: float):
        """
        Scale object mass. 
        
        Args:
            scale: 0.5 to 2.0 (multiplier)
        """
        body = physics_world.get_rigid_body(object_id)
        original_mass = body.mass()
        body.set_mass(original_mass * scale)
    
    @staticmethod
    def friction_variation(physics_world, object_id: str, new_friction: float):
        """
        Change surface friction. 
        
        Args:
            new_friction: 0.1 to 1.0
        """
        colliders = physics_world.get_colliders(object_id)
        for c in colliders:
            c.set_friction(new_friction)
    
    @staticmethod
    def position_noise(physics_world, object_id: str, sigma_xy: float, sigma_theta: float):
        """Add random noise to object pose"""
        body = physics_world.get_rigid_body(object_id)
        pos = body.translation()
        
        noise_xy = np.random.normal(0, sigma_xy, 2)
        noise_theta = np.random.normal(0, sigma_theta)
        
        body.set_translation(vector![pos.x + noise_xy[0], pos.y + noise_xy[1], pos.z])
        
        current_rot = body.rotation()
        delta_rot = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), noise_theta)
        body.set_rotation(current_rot * delta_rot)
    
    @staticmethod
    def external_force(physics_world, object_id: str, force: np.ndarray, duration: float):
        """Apply external force impulse"""
        body = physics_world.get_rigid_body(object_id)
        body.apply_impulse(vector![force[0], force[1], force[2]], true)
```

### 10.3 Perturbation Schedules
```yaml
# experiments/configs/perturbation_schedules.yaml
 
# H4: Generalization testing
generalization_battery:
  - name: lighting_sweep
    type: lighting_shift
    variations:
      - {intensity_delta: -0.3, color_temp_k: 3000}
      - {intensity_delta: 0.0, color_temp_k: 4500}
      - {intensity_delta: 0.3, color_temp_k: 6500}
  
  - name: distractor_density
    type: add_distractor
    variations:
      - {count: 0}
      - {count: 1, positions: [[0.5, 0.1, 0.04]]}
      - {count: 3, positions: [[0.5, 0.1, 0.04], [0.35, -0.05, 0.03], [0.55, 0.15, 0.05]]}
  
  - name: object_position_noise
    type: position_noise
    variations:
      - {sigma_xy: 0.0, sigma_theta: 0.0}
      - {sigma_xy: 0.02, sigma_theta: 0.1}
      - {sigma_xy: 0.05, sigma_theta: 0.2}
 
# H7: Embodiment gap testing
embodiment_battery:
  - name: mass_mismatch
    type: mass_variation
    target: red_cube
    variations: [0.5, 0.75, 1.0, 1.5, 2.0]
  
  - name: friction_mismatch
    type: friction_variation
    target: table
    variations: [0.2, 0.4, 0.6, 0.8]
```

---
 
## 11. Data Formats & Storage

### 11.1 Episode Storage (HDF5)
```python
import h5py
 
def save_episode(episode: PickPlaceEpisode, filepath: str):
    """Save episode to HDF5 format"""
    
    with h5py.File(filepath, 'w') as f:
        # Metadata
        meta = f.create_group("metadata")
        meta.attrs["episode_id"] = episode.episode_id
        meta.attrs["condition"] = episode.condition
        meta.attrs["instruction"] = episode.instruction
        meta.attrs["success"] = episode.success
        meta.attrs["failure_mode"] = episode.failure_mode or ""
        meta.attrs["seed"] = episode.seed
        
        # Trajectory
        traj = f.create_group("trajectory")
        traj.create_dataset("timestamps", data=episode.timestamps, compression="gzip")
        traj.create_dataset("joint_positions", data=episode.joint_positions, compression="gzip")
        traj.create_dataset("joint_velocities", data=episode.joint_velocities, compression="gzip")
        traj.create_dataset("ee_poses", data=episode.ee_poses, compression="gzip")
        traj.create_dataset("gripper_widths", data=episode.gripper_widths, compression="gzip")
        traj.create_dataset("actions", data=episode.actions_commanded, compression="gzip")
        
        # Images (compressed)
        imgs = f.create_group("images")
        imgs.create_dataset("wrist", data=episode.images_wrist, 
                           compression="gzip", compression_opts=4)
        imgs.create_dataset("overhead", data=episode.images_overhead,
                           compression="gzip", compression_opts=4)
        
        # Embeddings
        emb = f.create_group("embeddings")
        emb.create_dataset("V", data=episode.embeddings_V, compression="gzip")
        emb.create_dataset("L", data=episode.embeddings_L, compression="gzip")
        emb.create_dataset("D", data=episode.embeddings_D, compression="gzip")
        emb.create_dataset("V_reduced", data=episode.embeddings_V_reduced, compression="gzip")
        emb.create_dataset("L_reduced", data=episode.embeddings_L_reduced, compression="gzip")
        emb.create_dataset("D_reduced", data=episode.embeddings_D_reduced, compression="gzip")
        
        # PID metrics
        pid = f.create_group("pid")
        # Action target (H1)
        action = pid.create_group("action")
        action.create_dataset("synergy", data=episode.pid_action_synergy)
        action.create_dataset("redundancy", data=episode.pid_action_redundancy)
        action.create_dataset("unique_v", data=episode.pid_action_unique_v)
        action.create_dataset("unique_d", data=episode.pid_action_unique_d)
        
        # Flow target (H7)
        flow = pid.create_group("flow")
        flow.create_dataset("synergy", data=episode.pid_flow_synergy)
        flow.create_dataset("redundancy", data=episode.pid_flow_redundancy)
        flow.create_dataset("unique_v", data=episode.pid_flow_unique_v)
        flow.create_dataset("unique_d", data=episode.pid_flow_unique_d)
        
        # Object tracking
        objects = f.create_group("objects")
        for obj_id, poses in episode.object_poses.items():
            objects.create_dataset(obj_id, data=poses, compression="gzip")
```

### 11.2 Dataset Index (JSON)
```json
{
  "dataset_id": "pid_vla_exp1_v1",
  "created": "2026-01-15T10:30:00Z",
  "experiment": "exp1_pick_place",
  "n_episodes": 400,
  "conditions": ["baseline", "lighting_variation", "distractor_objects", "novel_instruction"],
  "episodes_per_condition": 100,
  
  "episodes": [
    {
      "episode_id": "exp1_baseline_001",
      "condition": "baseline",
      "filepath": "data/exp1/baseline/episode_001.h5",
      "success": true,
      "duration_s": 12.4,
      "seed": 42001
    }
  ],
  
  "statistics": {
    "overall_success_rate": 0.73,
    "by_condition": {
      "baseline": {"success_rate": 0.85, "mean_duration": 11.2},
      "lighting_variation": {"success_rate": 0.71, "mean_duration": 13.1},
      "distractor_objects": {"success_rate": 0.68, "mean_duration": 14.5},
      "novel_instruction": {"success_rate": 0.66, "mean_duration": 15.2}
    }
  }
}
```

---
 
## 12. Compute Requirements

### 12.1 Hardware Specifications
| Component     | Minimum         | Recommended         |
| --------------- | --------------- | ------------------- |
| Apple Silicon | M2 Pro (16GB)   | M4 Max (64GB)       |
| NVIDIA GPU    | RTX 3080 (10GB) | A100 (40GB) for WAN |
| RAM           | 32GB            | 64GB                |
| Storage       | 500GB SSD       | 2TB NVMe            |

### 12.2 Per-Component Resource Usage
| Component              | Device     | VRAM/RAM     | Compute Time  |
| ---------------------- | ---------- | ------------ | ------------- |
| OpenVLA 7B (inference) | M4 Max MLX | 16GB unified | ~2s/action    |
| WAN 2.2 (video gen)    | A100       | 40GB         | ~30s/video    |
| CoTracker3             | M4 Max MPS | 4GB          | ~5s/video     |
| Depth-Anything v3      | M4 Max MPS | 2GB          | ~0.5s/frame   |
| SAM3                   | M4 Max MPS | 4GB          | ~1s/image     |
| PID-Core (Rust)        | M4 Max CPU | 1GB          | <100ms/window |
| Rapier Physics         | M4 Max CPU | 0.5GB        | <1ms/step     |
| SparkJS Render         | M4 Max GPU | 4GB          | 16ms/frame    |

12.3 Latency Budget
Real-time Loop (no WAN):
Capture image          :    0ms (start)
VLA inference          : 2000ms
Action execution       :   10ms
Physics step           :    1ms
PID computation        :   50ms
Render                 :   16ms
─────────────────────────────────
Total                  : ~2100ms per action
Effective control Hz   : ~0.5 Hz

**Note: Quasi-Static Control Regime**
The ~0.5 Hz control loop restricts experiments to quasi-static manipulation tasks (e.g., pick-and-place with stable intermediate states). Dynamic tasks requiring high-frequency feedback (e.g., balancing, catching) are out of scope for this hardware configuration.

Offline Analysis (with WAN):
```
Episode recording      : Variable (task duration)
WAN video generation   : 30s per trigger point
Flow extraction        : 6s per video
Full PID analysis      : 10s per episode
─────────────────────────────────
Post-processing        : ~50s per episode
```

### 12.4 Storage Estimates
| Data Type                 | Per Episode | 100 Episodes |
| ------------------------- | ----------- | ------------ |
| Wrist images (30fps, 30s) | ~1.5 GB     | 150 GB       |
| Overhead images           | ~3.0 GB     | 300 GB       |
| Embeddings (5Hz)          | ~50 MB      | 5 GB         |
| Trajectories              | ~5 MB       | 500 MB       |
| PID metrics               | ~1 MB       | 100 MB       |
| Total (compressed)        | ~1 GB       | 100 GB       |

---
 
## 13. Reproducibility Checklist

### 13.1 Random Seeds
```python
# Master seed configuration
REPRODUCIBILITY_CONFIG = {
    "master_seed": 42,
    "numpy_seed": 42,
    "torch_seed": 42,
    "physics_seed": 42,
    
    # Per-episode seeds derived from master
    "episode_seed_fn": lambda master, episode_idx: master + episode_idx * 1000,
}
 
def set_all_seeds(seed: int):
    """Set all random seeds for reproducibility"""
    import random
    import numpy as np
    import torch
    import os
    
    random.seed(seed)
    np.random.seed(seed)
    torch.manual_seed(seed)
    if torch.cuda.is_available():
        torch.cuda.manual_seed_all(seed)
    
    # Rapier determinism
    os.environ["RAPIER_DETERMINISTIC"] = "1"
```

### 13.2 Version Pinning
```toml
# pyproject.toml
[project]
dependencies = [
    "numpy==1.26.4",
    "torch==2.2.0",
    "transformers==4.38.0",
    "openvla==0.1.0",
    "segment-anything-3==1.0.0",
    "cotracker3==3.0.0",
    "depth-anything-v3==3.0.0",
]
```

```toml
# Cargo.toml
[dependencies]
rapier3d = "0.18.0"
nalgebra = "0.33.0"
```

### 13.3 Experiment Manifest
Every experiment run should produce a manifest file:
```yaml
# results/exp1_run_001/manifest.yaml
experiment:
  id: exp1_run_001
  timestamp: 2026-01-15T10:30:00Z
  duration_hours: 4.5
 
environment:
  hostname: research-mac-01
  os: macOS 15.2
  hardware: Apple M4 Max (64GB)
  
software:
  pid_vla_commit: abc123def456
  python_version: 3.11.7
  rust_version: 1.75.0
  
seeds:
  master: 42
  numpy: 42
  torch: 42
  
config:
  experiment_config: experiments/configs/exp1_pick_place.yaml
  scene_config: scenes/simple_pick_place.yaml
  
results:
  n_episodes: 400
  success_rate: 0.73
  data_path: results/exp1_run_001/episodes/
  analysis_path: results/exp1_run_001/analysis/
```

---
 

---

## 14. World Model Comparison: ManiGaussian vs PEGS

### 14.1 Overview and Motivation

This experiment suite compares two fundamentally different approaches to integrating physics with 3D Gaussian Splatting for robotic manipulation:

| Approach     | Physics Type       | Visual Integration                            | Key Innovation                                    |
| ------------ | ------------------ | --------------------------------------------- | ------------------------------------------------- |
| ManiGaussian | Learned (implicit) | 3D Variational Encoder → Gaussian World Model | Predicts future Z embedding conditioned on action |
| PEGS         | Explicit (PBD)     | Dual Gaussian-Particle + Visual Forces        | Real-time correction via photometric error        |

**Core Research Questions:**
1. Which approach captures more information about ground truth physics? (Measured by I(Prediction; P_GT))
2. How do PID signatures differ between learned and explicit physics?
3. Which approach generalizes better to novel objects/physics perturbations?
4. How does temporal coherence degrade in long-horizon tasks?

### 14.2 Unified PID Metric Framework

```python
@dataclass
class WorldModelPIDMetrics:
    'Unified PID metrics for comparing ManiGaussian vs PEGS'
    
    # === ManiGaussian-specific ===
    # World model construction: PID(V, L; Z)
    mani_world_model_synergy: float      # Syn(V, L; Z)
    mani_world_model_redundancy: float   # Red(V, L; Z)
    mani_world_model_unique_v: float     # Unq(V; Z)
    mani_world_model_unique_l: float     # Unq(L; Z)
    
    # Dynamics prediction: I(Z'; P_GT)
    mani_prediction_mi: float            # MI between predicted embedding and ground truth
    
    # Action prediction: PID(Z, L; A)
    mani_action_synergy: float           # Syn(Z, L; A)
    mani_action_unique_z: float          # Unq(Z; A) - world model contribution
    mani_action_unique_l: float          # Unq(L; A) - hallucination risk indicator
    
    # === PEGS-specific ===
    # Correction quality: PID(P_pred, V_obs; P_corr)
    pegs_correction_synergy: float       # Syn(P_pred, V_obs; P_corr)
    pegs_correction_redundancy: float    # Red(P_pred, V_obs; P_corr)
    pegs_correction_unique_phys: float   # Unq(P_pred; P_corr) - physics dominance
    pegs_correction_unique_vis: float    # Unq(V_obs; P_corr) - visual dominance
    
    # Physics prediction accuracy: I(P_pred; P_GT)
    pegs_prediction_mi: float            # MI between PBD prediction and ground truth
    
    # Visual force magnitude (scalar)
    pegs_visual_force_magnitude: float   # ||F_visual|| - correction effort
    
    # === Comparative (both systems) ===
    # Flow consistency: I(WorldModel; Flow_GT)
    flow_consistency_mi: float           # How well does world model predict 3D flow?
    
    # Temporal stability
    synergy_variance: float              # Var(Syn) over episode
    synergy_slope: float                 # Trend of Syn over time (negative = degradation)
```

---

## 15. Experiment 6: World Model Prediction Fidelity

**Hypothesis H_WM1:** ManiGaussian achieves higher I(Prediction; P_GT) on in-distribution scenarios, but PEGS maintains stable accuracy across distribution shifts.

### 15.1 Task Definition

| Property     | Value                                                |
| ------------ | ---------------------------------------------------- |
| Task         | Open-loop trajectory prediction (no robot execution) |
| Input        | Initial scene state + 10-step action sequence        |
| Output       | Predicted object trajectories from both systems      |
| Ground Truth | Rapier3D physics simulation                          |
| Metric       | Prediction horizon (steps until error > threshold)   |
| Episodes     | 200 per condition                                    |

### 15.2 Experimental Protocol

```yaml
# experiments/configs/exp6_prediction_fidelity.yaml
experiment_id: exp6_world_model_prediction_v1
 
conditions:
  - name: single_object_push
    scene: scenes/simple_push.yaml
    action_type: push_sequence
    prediction_horizon: 20  # steps
    n_episodes: 200
    
  - name: multi_object_collision
    scene: scenes/collision_chain.yaml
    action_type: contact_cascade
    prediction_horizon: 30
    n_episodes: 200
    
  - name: stacking_dynamics
    scene: scenes/stacking.yaml
    action_type: place_sequence
    prediction_horizon: 25
    n_episodes: 200
 
world_models:
  - name: manigaussian
    type: learned
    checkpoint: models/manigaussian_v1.pth
    inference_mode: rollout
    
  - name: pegs
    type: explicit
    physics_engine: pbd
    pbd_iterations: 10
    visual_correction: false  # Pure physics prediction
    
  - name: pegs_corrected
    type: explicit
    physics_engine: pbd
    visual_correction: true
    correction_rate: 0.1
 
ground_truth:
  engine: rapier3d
  step_hz: 1000
  substeps: 10
```

### 15.3 Scene Configurations

**Scene: Single Object Push**
```yaml
# scenes/simple_push.yaml
scene_id: simple_push_v1
environment: tabletop
 
objects:
  - id: target_cube
    splat: assets/splats/red_cube.spz
    initial_pose:
      position: [0.45, 0.0, 0.025]
      orientation: [1.0, 0.0, 0.0, 0.0]
    physics:
      type: cuboid
      half_extents: [0.025, 0.025, 0.025]
      mass: 0.1
      friction: 0.5
      restitution: 0.2
 
action_sequence:
  type: push
  direction: [0.0, 1.0, 0.0]  # Push along Y axis
  force_magnitude: 2.0  # Newtons
  duration_steps: 5
```

**Scene: Multi-Object Collision Chain**
```yaml
# scenes/collision_chain.yaml
scene_id: collision_chain_v1
environment: tabletop
 
objects:
  - id: striker
    splat: assets/splats/green_sphere.spz
    initial_pose:
      position: [0.30, 0.0, 0.04]
      orientation: [1.0, 0.0, 0.0, 0.0]
    physics:
      type: ball
      radius: 0.04
      mass: 0.2
      friction: 0.3
      restitution: 0.8
 
  - id: target_1
    splat: assets/splats/red_cube.spz
    initial_pose:
      position: [0.45, 0.0, 0.025]
      orientation: [1.0, 0.0, 0.0, 0.0]
    physics:
      type: cuboid
      half_extents: [0.025, 0.025, 0.025]
      mass: 0.1
 
  - id: target_2
    splat: assets/splats/blue_cube.spz
    initial_pose:
      position: [0.52, 0.0, 0.025]
      orientation: [1.0, 0.0, 0.0, 0.0]
    physics:
      type: cuboid
      half_extents: [0.025, 0.025, 0.025]
      mass: 0.1
 
  - id: target_3
    splat: assets/splats/yellow_cube.spz
    initial_pose:
      position: [0.59, 0.0, 0.025]
      orientation: [1.0, 0.0, 0.0, 0.0]
    physics:
      type: cuboid
      half_extents: [0.025, 0.025, 0.025]
      mass: 0.1
 
action_sequence:
  type: impulse
  target: striker
  direction: [1.0, 0.0, 0.0]
  impulse_magnitude: 0.5  # N·s
```

### 15.4 PID Decompositions for Experiment 6

```python
@dataclass
class Exp6PIDAnalysis:
    'PID analysis for world model prediction fidelity'
    
    # ManiGaussian-specific
    # Sources: V (visual input), L (language), Z (Gaussian embedding)
    # Target: P_GT (ground truth positions)
    manigaussian_mi_z_pgt: float          # I(Z'; P_GT) - embedding captures physics
    manigaussian_pid_v_l_z: Pid2Result    # PID(V, L; Z) - how V,L form world model
    
    # PEGS-specific  
    # Sources: P_pred (PBD prediction), V_obs (visual observation)
    # Target: P_GT (ground truth positions)
    pegs_mi_pred_pgt: float               # I(P_pred; P_GT) - physics prediction
    pegs_mi_corr_pgt: float               # I(P_corrected; P_GT) - after correction
    pegs_pid_pred_vobs_pcorr: Pid2Result  # PID(P_pred, V_obs; P_corr)
    
    # Comparative metrics
    prediction_horizon_manigaussian: int  # Steps until error > threshold
    prediction_horizon_pegs: int
    prediction_horizon_pegs_corrected: int
 
 
def compute_exp6_pid(
    manigaussian_predictions: np.ndarray,  # (T, n_objects, 3)
    pegs_predictions: np.ndarray,          # (T, n_objects, 3)
    pegs_corrected: np.ndarray,            # (T, n_objects, 3)
    ground_truth: np.ndarray,              # (T, n_objects, 3)
    visual_observations: np.ndarray,       # (T, H, W, 3)
    manigaussian_embeddings: np.ndarray,   # (T, embed_dim)
    cfg: Pid2Config,
) -> Exp6PIDAnalysis:
    '
    Compute PID metrics comparing world model predictions.
    
    Key question: Which representation captures more information 
    about ground truth physics?
    '
    T = len(ground_truth)
    n_obj = ground_truth.shape[1]
    
    # Flatten object positions for MI computation
    gt_flat = ground_truth.reshape(T, -1)  # (T, n_objects * 3)
    mg_flat = manigaussian_predictions.reshape(T, -1)
    pegs_flat = pegs_predictions.reshape(T, -1)
    pegs_corr_flat = pegs_corrected.reshape(T, -1)
    
    # Standardize
    gt_std = Standardizer.fit_transform(MatRef.from_numpy(gt_flat))
    mg_std = Standardizer.fit_transform(MatRef.from_numpy(mg_flat))
    pegs_std = Standardizer.fit_transform(MatRef.from_numpy(pegs_flat))
    pegs_corr_std = Standardizer.fit_transform(MatRef.from_numpy(pegs_corr_flat))
    z_std = Standardizer.fit_transform(MatRef.from_numpy(manigaussian_embeddings))
    
    # 1. ManiGaussian: I(Z'; P_GT)
    mi_z_pgt = ksg_mi(z_std, gt_std, cfg.ksg)
    
    # 2. PEGS: I(P_pred; P_GT) and I(P_corr; P_GT)
    mi_pred_pgt = ksg_mi(pegs_std, gt_std, cfg.ksg)
    mi_corr_pgt = ksg_mi(pegs_corr_std, gt_std, cfg.ksg)
    
    # 3. PEGS PID: How does visual correction contribute?
    # PID(P_pred, V_features; P_corrected)
    v_features = extract_visual_features(visual_observations)  # CNN/ViT features
    v_std = Standardizer.fit_transform(MatRef.from_numpy(v_features))
    
    pegs_pid = pid2_isx(pegs_std, v_std, pegs_corr_std, cfg)
    
    # 4. Compute prediction horizons
    error_threshold = 0.02  # 2cm position error
    
    mg_horizon = compute_prediction_horizon(mg_flat, gt_flat, error_threshold)
    pegs_horizon = compute_prediction_horizon(pegs_flat, gt_flat, error_threshold)
    pegs_corr_horizon = compute_prediction_horizon(pegs_corr_flat, gt_flat, error_threshold)
    
    return Exp6PIDAnalysis(
        manigaussian_mi_z_pgt=mi_z_pgt,
        pegs_mi_pred_pgt=mi_pred_pgt,
        pegs_mi_corr_pgt=mi_corr_pgt,
        pegs_pid_pred_vobs_pcorr=pegs_pid,
        prediction_horizon_manigaussian=mg_horizon,
        prediction_horizon_pegs=pegs_horizon,
        prediction_horizon_pegs_corrected=pegs_corr_horizon,
    )
 
 
def compute_prediction_horizon(
    predictions: np.ndarray,
    ground_truth: np.ndarray,
    threshold: float,
) -> int:
    'Find first timestep where prediction error exceeds threshold'
    errors = np.linalg.norm(predictions - ground_truth, axis=1)
    exceeds = np.where(errors > threshold)[0]
    return exceeds[0] if len(exceeds) > 0 else len(predictions)
```

### 15.5 Evaluation Metrics

```python
def evaluate_exp6(results: List[Exp6PIDAnalysis]) -> dict:
    'Aggregate evaluation for Experiment 6'
    
    # Prediction accuracy comparison
    mg_horizons = [r.prediction_horizon_manigaussian for r in results]
    pegs_horizons = [r.prediction_horizon_pegs for r in results]
    pegs_corr_horizons = [r.prediction_horizon_pegs_corrected for r in results]
    
    # MI comparison
    mg_mi = [r.manigaussian_mi_z_pgt for r in results]
    pegs_mi = [r.pegs_mi_pred_pgt for r in results]
    pegs_corr_mi = [r.pegs_mi_corr_pgt for r in results]
    
    # Visual correction benefit (PEGS)
    correction_benefit = [
        r.pegs_pid_pred_vobs_pcorr.synergy for r in results
    ]
    
    # Statistical tests
    from scipy.stats import wilcoxon, ttest_rel
    
    # H_WM1: Is ManiGaussian MI > PEGS MI on in-distribution?
    stat_mi, pval_mi = ttest_rel(mg_mi, pegs_mi)
    
    # H_WM1b: Does visual correction help PEGS?
    stat_corr, pval_corr = ttest_rel(pegs_corr_mi, pegs_mi)
    
    return {
        "mean_horizon_manigaussian": np.mean(mg_horizons),
        "mean_horizon_pegs": np.mean(pegs_horizons),
        "mean_horizon_pegs_corrected": np.mean(pegs_corr_horizons),
        "mean_mi_manigaussian": np.mean(mg_mi),
        "mean_mi_pegs": np.mean(pegs_mi),
        "mean_mi_pegs_corrected": np.mean(pegs_corr_mi),
        "mean_correction_synergy": np.mean(correction_benefit),
        "h_wm1_pvalue": pval_mi,
        "h_wm1_significant": pval_mi < 0.05,
        "visual_correction_benefit_pvalue": pval_corr,
    }
```

---

## 16. Experiment 7: Novel Object Generalization

**Hypothesis H_WM2:** PEGS maintains stable PID signatures (Syn(P_pred, V_obs; P_corr)) on novel objects because physics generalizes; ManiGaussian's Syn(V, L; Z) degrades significantly on objects outside training distribution.

### 16.1 Task Definition

| Property         | Value                                                          |
| ---------------- | -------------------------------------------------------------- |
| Task             | Pick-and-place with novel objects                              |
| Novel Geometries | L-shaped blocks, hollow cylinders, torus                       |
| Novel Materials  | Glass-like (transparent), metallic (reflective)                |
| Novel Mass       | Weighted dice (asymmetric CoM), hollow vs solid                |
| Control          | Same objects captured as 3DGS but NOT in ManiGaussian training |

### 16.2 Novel Object Library

```yaml
# assets/novel_objects/object_library.yaml
novel_objects:
  # Geometry novelty
  - id: l_block
    description: "L-shaped wooden block"
    splat: assets/splats/novel/l_block.spz
    physics:
      type: compound  # Multiple cuboids
      components:
        - {type: cuboid, half_extents: [0.04, 0.02, 0.02], offset: [0, 0, 0]}
        - {type: cuboid, half_extents: [0.02, 0.04, 0.02], offset: [0.03, 0.02, 0]}
      mass: 0.15
    in_mani_training: false
    
  - id: hollow_cylinder
    description: "Hollow metal cylinder (pipe section)"
    splat: assets/splats/novel/hollow_cylinder.spz
    physics:
      type: trimesh
      mesh_path: assets/meshes/hollow_cylinder.obj
      mass: 0.08
    in_mani_training: false
    
  - id: torus
    description: "Rubber torus (donut shape)"
    splat: assets/splats/novel/torus.spz
    physics:
      type: trimesh
      mesh_path: assets/meshes/torus.obj
      mass: 0.05
    in_mani_training: false
    
  # Material novelty (same geometry as training, different appearance)
  - id: glass_cube
    description: "Transparent glass cube (same size as red_cube)"
    splat: assets/splats/novel/glass_cube.spz  # Captured with DKT for transparency
    physics:
      type: cuboid
      half_extents: [0.025, 0.025, 0.025]
      mass: 0.15  # Glass is denser
    in_mani_training: false
    visual_challenge: transparency
    
  - id: chrome_sphere
    description: "Reflective chrome sphere"
    splat: assets/splats/novel/chrome_sphere.spz
    physics:
      type: ball
      radius: 0.04
      mass: 0.5  # Metal
    in_mani_training: false
    visual_challenge: reflection
    
  # Mass distribution novelty
  - id: weighted_die
    description: "Cube with asymmetric weight distribution"
    splat: assets/splats/novel/weighted_die.spz
    physics:
      type: cuboid
      half_extents: [0.025, 0.025, 0.025]
      mass: 0.2
      center_of_mass: [0.01, 0.01, -0.015]  # Off-center
    in_mani_training: false
    physics_challenge: asymmetric_com
```

### 16.3 Experimental Conditions

```yaml
# experiments/configs/exp7_novel_objects.yaml
experiment_id: exp7_novel_object_generalization_v1
 
conditions:
  # Baseline: familiar objects (sanity check)
  - name: familiar_red_cube
    scene: scenes/novel_object_test.yaml
    target_object: red_cube
    instruction: "Pick up the red cube and place it on the blue plate."
    n_episodes: 50
    
  # Geometry novelty
  - name: novel_l_block
    scene: scenes/novel_object_test.yaml
    target_object: l_block
    instruction: "Pick up the L-shaped block and place it on the blue plate."
    n_episodes: 50
    
  - name: novel_hollow_cylinder
    scene: scenes/novel_object_test.yaml
    target_object: hollow_cylinder
    instruction: "Pick up the hollow cylinder and place it on the blue plate."
    n_episodes: 50
    
  - name: novel_torus
    scene: scenes/novel_object_test.yaml
    target_object: torus
    instruction: "Pick up the rubber ring and place it on the blue plate."
    n_episodes: 50
    
  # Material novelty
  - name: novel_glass_cube
    scene: scenes/novel_object_test.yaml
    target_object: glass_cube
    instruction: "Pick up the glass cube and place it on the blue plate."
    n_episodes: 50
    note: "Tests visual encoder on transparency (use DKT-captured splat)"
    
  - name: novel_chrome_sphere
    scene: scenes/novel_object_test.yaml
    target_object: chrome_sphere
    instruction: "Pick up the shiny metal ball and place it on the blue plate."
    n_episodes: 50
    note: "Tests visual encoder on reflections"
    
  # Physics novelty
  - name: novel_weighted_die
    scene: scenes/novel_object_test.yaml
    target_object: weighted_die
    instruction: "Pick up the die and place it on the blue plate."
    n_episodes: 50
    note: "Asymmetric CoM causes unexpected rotation during grasp"
 
analysis:
  primary_metrics:
    - "pid_stability"  # Variance of Syn across episode
    - "success_rate"
    - "grasp_success_rate"
  
  pid_decomposition:
    manigaussian:
      - "Syn(V, L; Z)"
      - "Unq(V; Z)"
      - "Unq(L; Z)"
    pegs:
      - "Syn(P_pred, V_obs; P_corr)"
      - "Unq(P_pred; P_corr)"
      - "Unq(V_obs; P_corr)"
```

### 16.4 PID Analysis for Generalization

```python
def analyze_novel_object_generalization(
    mani_episodes: List[NovelObjectEpisode],
    pegs_episodes: List[NovelObjectEpisode]
) -> dict:
    '
    Test H_WM2: PEGS maintains stable PID; ManiGaussian degrades on novel objects
    '
    
    # Group by object novelty type
    novelty_types = ["familiar", "geometry", "material", "physics"]
    
    results = {}
    for novelty in novelty_types:
        mani_eps = [e for e in mani_episodes if e.novelty_type == novelty]
        pegs_eps = [e for e in pegs_episodes if e.novelty_type == novelty]
        
        # Compute PID variance (stability metric)
        mani_syn_values = []
        for ep in mani_eps:
            syn_trajectory = compute_manigaussian_synergy_trajectory(ep)
            mani_syn_values.extend(syn_trajectory)
        
        pegs_syn_values = []
        for ep in pegs_eps:
            syn_trajectory = compute_pegs_synergy_trajectory(ep)
            pegs_syn_values.extend(syn_trajectory)
        
        results[novelty] = {
            "manigaussian": {
                "mean_synergy": np.mean(mani_syn_values),
                "std_synergy": np.std(mani_syn_values),
                "success_rate": np.mean([e.success for e in mani_eps]),
            },
            "pegs": {
                "mean_synergy": np.mean(pegs_syn_values),
                "std_synergy": np.std(pegs_syn_values),
                "success_rate": np.mean([e.success for e in pegs_eps]),
            }
        }
    
    # H_WM2 test: Compare stability drop from familiar to novel
    mani_familiar_std = results["familiar"]["manigaussian"]["std_synergy"]
    mani_novel_stds = [results[n]["manigaussian"]["std_synergy"] for n in ["geometry", "material", "physics"]]
    mani_stability_drop = np.mean(mani_novel_stds) / mani_familiar_std
    
    pegs_familiar_std = results["familiar"]["pegs"]["std_synergy"]
    pegs_novel_stds = [results[n]["pegs"]["std_synergy"] for n in ["geometry", "material", "physics"]]
    pegs_stability_drop = np.mean(pegs_novel_stds) / pegs_familiar_std
    
    return {
        "per_novelty_results": results,
        "h_wm2_test": {
            "hypothesis": "PEGS more stable than ManiGaussian on novel objects",
            "mani_stability_drop_ratio": mani_stability_drop,
            "pegs_stability_drop_ratio": pegs_stability_drop,
            "supported": pegs_stability_drop < mani_stability_drop,
        }
    }
```

---

## 17. Experiment 8: Physics Perturbation Sensitivity

**Hypothesis H_WM3:** PEGS's visual correction mechanism compensates for physics mismatch (stable PID, increased ||F_visual||); ManiGaussian shows PID distribution shift (increased Unq(V; Z) as it falls back to visual dominance).

### 17.1 Task Definition

| Property      | Value                                                                   |
| ------------- | ----------------------------------------------------------------------- |
| Task          | Standard pick-and-place                                                 |
| Perturbations | Mass ×{0.5, 0.75, 1.0, 1.5, 2.0}, Friction ×{0.3, 0.5, 1.0, 1.5, 2.0}   |
| Metric        | PID stability under physics mismatch                                    |
| Episodes      | 50 per perturbation level × 2 systems = 500 total per perturbation type |

### 17.2 Perturbation Matrix

```yaml
# experiments/configs/exp8_physics_perturbation.yaml
experiment_id: exp8_physics_perturbation_sensitivity_v1
 
perturbation_matrix:
  mass_scale: [0.5, 0.75, 1.0, 1.5, 2.0]
  friction_scale: [0.3, 0.5, 1.0, 1.5, 2.0]
  
# Generate all combinations
conditions:
  # Mass perturbations (friction = 1.0)
  - name: mass_0.5x
    perturbations:
      - {type: mass_variation, target: red_cube, scale: 0.5}
    n_episodes: 50
    
  - name: mass_0.75x
    perturbations:
      - {type: mass_variation, target: red_cube, scale: 0.75}
    n_episodes: 50
    
  # ... etc for all mass values
  
  # Friction perturbations (mass = 1.0)
  - name: friction_0.3x
    perturbations:
      - {type: friction_variation, target: table, scale: 0.3}
    n_episodes: 50
    
  # ... etc for all friction values
  
  # Combined perturbations (stress test)
  - name: combined_heavy_slippery
    perturbations:
      - {type: mass_variation, target: red_cube, scale: 2.0}
      - {type: friction_variation, target: table, scale: 0.3}
    n_episodes: 50
 
analysis:
  key_metrics:
    - "pid_distribution_shift"  # KL divergence from baseline
    - "visual_force_magnitude"  # PEGS correction effort
    - "unique_v_ratio"         # ManiGaussian visual dominance
```

### 17.3 PID Sensitivity Analysis

```python
def analyze_physics_perturbation_sensitivity(
    baseline_mani: List[Episode],
    baseline_pegs: List[Episode],
    perturbed_mani: Dict[str, List[Episode]],  # perturbation_name -> episodes
    perturbed_pegs: Dict[str, List[Episode]],
) -> dict:
    '
    Test H_WM3: PEGS compensates via visual forces; ManiGaussian shifts to visual dominance
    '
    
    # Compute baseline PID distributions
    baseline_mani_pid = extract_pid_distribution(baseline_mani, "manigaussian")
    baseline_pegs_pid = extract_pid_distribution(baseline_pegs, "pegs")
    
    results = {"manigaussian": {}, "pegs": {}}
    
    for pert_name, mani_eps in perturbed_mani.items():
        pegs_eps = perturbed_pegs[pert_name]
        
        # ManiGaussian analysis
        mani_pid = extract_pid_distribution(mani_eps, "manigaussian")
        mani_kl = compute_kl_divergence(baseline_mani_pid["synergy"], mani_pid["synergy"])
        mani_unique_v_ratio = np.mean(mani_pid["unique_v"]) / (
            np.mean(mani_pid["unique_v"]) + np.mean(mani_pid["unique_l"]) + 1e-8
        )
        
        results["manigaussian"][pert_name] = {
            "kl_from_baseline": mani_kl,
            "unique_v_ratio": mani_unique_v_ratio,
            "mean_synergy": np.mean(mani_pid["synergy"]),
            "success_rate": np.mean([e.success for e in mani_eps]),
        }
        
        # PEGS analysis
        pegs_pid = extract_pid_distribution(pegs_eps, "pegs")
        pegs_kl = compute_kl_divergence(baseline_pegs_pid["synergy"], pegs_pid["synergy"])
        mean_visual_force = np.mean([
            np.linalg.norm(e.pegs_visual_forces, axis=-1).mean() 
            for e in pegs_eps
        ])
        
        results["pegs"][pert_name] = {
            "kl_from_baseline": pegs_kl,
            "mean_visual_force": mean_visual_force,
            "mean_synergy": np.mean(pegs_pid["synergy"]),
            "success_rate": np.mean([e.success for e in pegs_eps]),
        }
    
    # H_WM3 tests
    # 1. PEGS should have lower KL divergence (more stable PID)
    mani_kls = [r["kl_from_baseline"] for r in results["manigaussian"].values()]
    pegs_kls = [r["kl_from_baseline"] for r in results["pegs"].values()]
    
    # 2. PEGS visual force should increase with perturbation severity
    perturbation_severity = [0.5, 0.75, 1.0, 1.5, 2.0]  # mass scale
    visual_forces = [results["pegs"][f"mass_{s}x"]["mean_visual_force"] for s in perturbation_severity]
    force_correlation = np.corrcoef(
        [abs(1 - s) for s in perturbation_severity],  # deviation from nominal
        visual_forces
    )[0, 1]
    
    # 3. ManiGaussian unique_v_ratio should increase under perturbation
    mani_unique_v_baseline = extract_pid_distribution(baseline_mani, "manigaussian")["unique_v"].mean()
    mani_unique_v_perturbed = np.mean([r["unique_v_ratio"] for r in results["manigaussian"].values()])
    
    return {
        "per_perturbation_results": results,
        "h_wm3_tests": {
            "pegs_more_stable": {
                "mani_mean_kl": np.mean(mani_kls),
                "pegs_mean_kl": np.mean(pegs_kls),
                "supported": np.mean(pegs_kls) < np.mean(mani_kls),
            },
            "pegs_force_compensates": {
                "force_perturbation_correlation": force_correlation,
                "supported": force_correlation > 0.5,  # positive correlation
            },
            "mani_visual_fallback": {
                "baseline_unique_v_ratio": mani_unique_v_baseline,
                "perturbed_unique_v_ratio": mani_unique_v_perturbed,
                "supported": mani_unique_v_perturbed > mani_unique_v_baseline * 1.2,  # 20% increase
            }
        }
    }
```

---

## 18. Experiment 9: Temporal Coherence in Long-Horizon Tasks

**Hypothesis H_WM4:** ManiGaussian shows faster synergy decay over long-horizon tasks; PEGS maintains coherence via continuous visual correction.

### 18.1 Task Definition

| Property         | Value                                 |
| ---------------- | ------------------------------------- |
| Task             | 5-block tower stacking                |
| Phases           | 10 subtasks (5 grasps + 5 placements) |
| Success Criteria | All 5 blocks stacked, stable for 3s   |
| Timeout          | 300 seconds                           |
| Episodes         | 50                                    |

### 18.2 Scene Configuration

```yaml
# scenes/tower_5_blocks.yaml
scene_id: tower_5_blocks_v1
environment: tabletop
 
objects:
  - id: block_1
    splat: assets/splats/red_cube.spz
    initial_pose: {position: [0.30, 0.15, 0.025], orientation: [1,0,0,0]}
    physics: {type: cuboid, half_extents: [0.025, 0.025, 0.025], mass: 0.1}
    stack_order: 1
    
  - id: block_2
    splat: assets/splats/blue_cube.spz
    initial_pose: {position: [0.35, 0.20, 0.025], orientation: [1,0,0,0]}
    physics: {type: cuboid, half_extents: [0.025, 0.025, 0.025], mass: 0.1}
    stack_order: 2
    
  - id: block_3
    splat: assets/splats/green_cube.spz
    initial_pose: {position: [0.40, 0.15, 0.025], orientation: [1,0,0,0]}
    physics: {type: cuboid, half_extents: [0.025, 0.025, 0.025], mass: 0.1}
    stack_order: 3
    
  - id: block_4
    splat: assets/splats/yellow_cube.spz
    initial_pose: {position: [0.45, 0.20, 0.025], orientation: [1,0,0,0]}
    physics: {type: cuboid, half_extents: [0.025, 0.025, 0.025], mass: 0.1}
    stack_order: 4
    
  - id: block_5
    splat: assets/splats/purple_cube.spz
    initial_pose: {position: [0.50, 0.15, 0.025], orientation: [1,0,0,0]}
    physics: {type: cuboid, half_extents: [0.025, 0.025, 0.025], mass: 0.1}
    stack_order: 5
    
  - id: target_base
    splat: assets/splats/target_platform.spz
    initial_pose: {position: [0.45, -0.10, 0.001], orientation: [1,0,0,0]}
    physics: {type: cuboid, half_extents: [0.04, 0.04, 0.001], fixed: true}
 
instruction: "Stack all five blocks in order: red, blue, green, yellow, purple from bottom to top."
```

### 18.3 Phase Detection and Temporal Analysis

```python
def detect_stacking_phases(episode: StackingEpisode) -> List[TaskPhase]:
    'Detect grasp and place phases from episode data'
    phases = []
    phase_id = 0
    
    gripper_widths = episode.gripper_widths
    timestamps = episode.timestamps
    
    # State machine for phase detection
    state = "searching"  # "searching", "approaching", "grasping", "lifting", "placing"
    current_object = None
    phase_start = None
    
    for t in range(1, len(timestamps)):
        dt = timestamps[t] - timestamps[t-1]
        
        # Detect grasp initiation (gripper closing rapidly)
        if state == "searching" and gripper_widths[t] < gripper_widths[t-1] - 0.005:
            state = "grasping"
            phase_start = timestamps[t]
            current_object = identify_nearest_object(episode, t)
            
        # Detect grasp completion (object lifted)
        elif state == "grasping":
            obj_height = episode.object_poses[current_object][t, 2]
            if obj_height > 0.05:  # Lifted threshold
                phases.append(TaskPhase(
                    phase_id=phase_id,
                    phase_type="grasp",
                    target_object=current_object,
                    start_time=phase_start,
                    end_time=timestamps[t],
                    success=True,
                ))
                phase_id += 1
                state = "placing"
                phase_start = timestamps[t]
                
        # Detect place completion (gripper opening, object stationary)
        elif state == "placing" and gripper_widths[t] > gripper_widths[t-1] + 0.01:
            obj_vel = np.linalg.norm(
                episode.object_velocities[current_object][t]
            )
            if obj_vel < 0.01:  # Stationary threshold
                phases.append(TaskPhase(
                    phase_id=phase_id,
                    phase_type="place",
                    target_object=current_object,
                    start_time=phase_start,
                    end_time=timestamps[t],
                    success=True,
                ))
                phase_id += 1
                state = "searching"
                current_object = None
                
    return phases
```

### 18.4 Temporal Coherence Evaluation

```python
def evaluate_exp9(results: List[Exp9PIDAnalysis]) -> dict:
    '
    Evaluate temporal coherence over long-horizon tasks.
    
    Key metrics:
    - Synergy degradation slope: negative = degrading, 0 = stable
    - Cumulative error growth rate
    - Correlation between synergy and phase success
    '
    
    # Aggregate synergy slopes
    mg_slopes = [r.synergy_slope_mg for r in results if not np.isnan(r.synergy_slope_mg)]
    pegs_slopes = [r.synergy_slope_pegs for r in results if not np.isnan(r.synergy_slope_pegs)]
    
    # Test H_WM4: ManiGaussian has more negative slope
    from scipy.stats import ttest_ind
    stat, pval = ttest_ind(mg_slopes, pegs_slopes)
    
    # Phase-by-phase synergy curves
    max_phases = max(len(r.phase_synergies_mg) for r in results)
    
    mg_synergy_curve = []
    pegs_synergy_curve = []
    
    for phase_idx in range(max_phases):
        mg_vals = [r.phase_synergies_mg[phase_idx] 
                  for r in results if phase_idx < len(r.phase_synergies_mg)]
        pegs_vals = [r.phase_synergies_pegs[phase_idx]
                   for r in results if phase_idx < len(r.phase_synergies_pegs)]
        
        mg_synergy_curve.append({
            "phase": phase_idx,
            "mean": np.nanmean(mg_vals),
            "std": np.nanstd(mg_vals),
        })
        pegs_synergy_curve.append({
            "phase": phase_idx,
            "mean": np.nanmean(pegs_vals),
            "std": np.nanstd(pegs_vals),
        })
    
    # Task completion rates
    mg_completion = np.mean([r.phases_completed / r.total_phases for r in results])
    pegs_completion = np.mean([r.phases_completed / r.total_phases for r in results])
    
    # Correlation: synergy with success
    all_phase_synergies_mg = []
    all_phase_successes = []
    for r in results:
        for phase, syn in zip(r.phases, r.phase_synergies_mg):
            if not np.isnan(syn):
                all_phase_synergies_mg.append(syn)
                all_phase_successes.append(1 if phase.success else 0)
    
    from scipy.stats import pointbiserialr
    corr, corr_pval = pointbiserialr(all_phase_successes, all_phase_synergies_mg)
    
    return {
        "mean_synergy_slope_mg": np.mean(mg_slopes),
        "mean_synergy_slope_pegs": np.mean(pegs_slopes),
        "h_wm4_pvalue": pval,
        "h_wm4_mg_degrades_more": np.mean(mg_slopes) < np.mean(pegs_slopes),
        "mg_synergy_curve": mg_synergy_curve,
        "pegs_synergy_curve": pegs_synergy_curve,
        "mg_phase_completion_rate": mg_completion,
        "pegs_phase_completion_rate": pegs_completion,
        "synergy_success_correlation": corr,
        "synergy_success_pvalue": corr_pval,
    }
```

---

## 19. Experiment 10: Deformable Object Manipulation

**Hypothesis H_WM5:** PEGS handles deformables via explicit constraints; ManiGaussian fails (high Unq(L; A)).

### 19.1 Task Definition

| Property     | Value                                        |
| ------------ | -------------------------------------------- |
| Task A       | Rope arrangement (form a circle)             |
| Task B       | Cloth folding (fold in half)                 |
| Physics      | PEGS uses particle chains / mesh constraints |
| ManiGaussian | No deformable support (expected failure)     |
| Episodes     | 30 per task                                  |

### 19.2 Deformable Object Specifications

```yaml
# objects/deformables.yaml
deformable_objects:
  - id: rope_1m
    description: "1m flexible rope"
    type: rope
    
    splat: assets/splats/deformable/rope_1m.spz
    
    pegs_config:
      particle_count: 100
      segment_length: 0.01  # meters
      bending_stiffness: 0.5
      stretch_stiffness: 1.0
      particle_mass: 0.001  # kg per particle
      
    physics_proxy:
      type: particle_chain
      constraints:
        - type: distance
          rest_length: 0.01
          compliance: 0.0001
        - type: bending
          rest_angle: 0.0
          compliance: 0.001
          
    manigaussian_support: false
    
  - id: cloth_30x30
    description: "30cm x 30cm cloth"
    type: cloth
    
    splat: assets/splats/deformable/cloth_30x30.spz
    
    pegs_config:
      particle_grid: [30, 30]  # 30x30 particles
      rest_spacing: 0.01  # meters
      stretch_stiffness: 1.0
      shear_stiffness: 0.5
      bend_stiffness: 0.1
      particle_mass: 0.0001
      
    physics_proxy:
      type: particle_mesh
      constraints:
        - type: stretch
          compliance: 0.0001
        - type: shear
          compliance: 0.001
        - type: bend
          compliance: 0.01
          
    manigaussian_support: false
```

### 19.3 Deformable Task Configurations

```yaml
# experiments/configs/exp10_deformables.yaml
experiment_id: exp10_deformable_manipulation
 
tasks:
  - name: rope_circle
    object: rope_1m
    instruction: "Arrange the rope into a circle on the table."
    success_criteria:
      shape: circle
      radius_tolerance: 0.05  # meters
      closure_gap: 0.02  # max gap at ends
    n_episodes: 30
    
  - name: rope_line
    object: rope_1m
    instruction: "Straighten the rope into a line."
    success_criteria:
      shape: line
      straightness: 0.95  # correlation coefficient
    n_episodes: 30
    
  - name: cloth_fold_half
    object: cloth_30x30
    instruction: "Fold the cloth in half."
    success_criteria:
      fold_type: half
      alignment_error: 0.02  # meters
      fold_crispness: 0.8
    n_episodes: 30
    
  - name: cloth_flatten
    object: cloth_30x30
    instruction: "Flatten the wrinkled cloth on the table."
    success_criteria:
      flatness: 0.95  # max height variance
    n_episodes: 30
 
world_model_configs:
  pegs:
    deformable_solver: position_based_dynamics
    solver_iterations: 20
    substeps: 5
    visual_correction: true
    
  manigaussian:
    # Expected to fail - rigid body assumption
    fallback_mode: rigid_approximation
```

### 19.4 PID Analysis for Deformables

```python
def compute_exp10_pid(
    episode: DeformableEpisode,
    cfg: Pid2Config,
) -> Exp10PIDAnalysis:
    '
    Analyze deformable manipulation performance.
    
    Key hypothesis H_WM5:
    - PEGS handles deformables via explicit constraints
    - ManiGaussian fails, falls back to language (high Unq(L; A))
    '
    
    # PEGS analysis
    if episode.pegs_data is not None:
        # Particle positions from PBD
        P_particles = episode.pegs_data.particle_positions  # (T, n_particles, 3)
        V_features = episode.visual_features
        P_corrected = episode.pegs_data.corrected_particles
        
        # Flatten particles for PID
        T = len(P_particles)
        P_flat = P_particles.reshape(T, -1)
        P_corr_flat = P_corrected.reshape(T, -1)
        
        # Reduce dimensionality (too many particles)
        from sklearn.decomposition import PCA
        pca = PCA(n_components=64)
        P_reduced = pca.fit_transform(P_flat)
        P_corr_reduced = pca.transform(P_corr_flat)
        
        P_std = Standardizer.fit_transform(MatRef.from_numpy(P_reduced))
        V_std = Standardizer.fit_transform(MatRef.from_numpy(V_features))
        P_corr_std = Standardizer.fit_transform(MatRef.from_numpy(P_corr_reduced))
        
        pegs_pid = pid2_isx(P_std, V_std, P_corr_std, cfg)
        
        # Constraint analysis
        constraint_violations = episode.pegs_data.constraint_violations
        mean_violation = np.mean(constraint_violations)
        
        # Particle tracking coverage
        particle_coverage = episode.pegs_data.tracking_coverage
        
    else:
        pegs_pid = None
        mean_violation = None
        particle_coverage = None
    
    # ManiGaussian failure analysis
    if episode.manigaussian_data is not None:
        mg_data = episode.manigaussian_data
        
        # Check if fallback was triggered
        fallback_triggered = mg_data.used_rigid_fallback
        
        # Language dominance analysis
        V = mg_data.visual_embeddings
        L = mg_data.language_embeddings
        D = mg_data.decoder_embeddings
        A = mg_data.actions
        
        V_std = Standardizer.fit_transform(MatRef.from_numpy(V))
        L_std = Standardizer.fit_transform(MatRef.from_numpy(L))
        A_std = Standardizer.fit_transform(MatRef.from_numpy(A))
        
        # PID(V, L; A) to check language dominance
        mg_pid = pid2_isx(V_std, L_std, A_std, cfg)
        total_mi = (mg_pid.redundancy + mg_pid.unique_s1 + 
                   mg_pid.unique_s2 + mg_pid.synergy)
        language_dominance = mg_pid.unique_s2 / total_mi if total_mi > 0 else 0
        
        # Action variance (uncertainty indicator)
        action_variance = np.mean(np.var(A, axis=0))
        
    else:
        fallback_triggered = None
        language_dominance = None
        action_variance = None
    
    return Exp10PIDAnalysis(
        task_name=episode.task_name,
        object_type=episode.object_type,
        pegs_pid_particles_v_corr=pegs_pid,
        pegs_constraint_violations=mean_violation,
        pegs_particle_coverage=particle_coverage,
        mg_attempted=episode.manigaussian_data is not None,
        mg_fallback_triggered=fallback_triggered,
        mg_language_dominance=language_dominance,
        mg_action_variance=action_variance,
        success_pegs=episode.pegs_success,
        success_mg=episode.manigaussian_success,
        shape_error_pegs=episode.pegs_shape_error,
        shape_error_mg=episode.manigaussian_shape_error,
    )
```

### 19.5 Deformable Manipulation Evaluation

```python
def evaluate_exp10(results: List[Exp10PIDAnalysis]) -> dict:
    '
    Evaluate deformable manipulation capabilities.
    
    Expected outcome: PEGS succeeds, ManiGaussian fails with high Unq(L; A)
    '
    
    # Success rates
    pegs_success = np.mean([r.success_pegs for r in results if r.success_pegs is not None])
    mg_success = np.mean([r.success_mg for r in results if r.success_mg is not None])
    
    # By object type
    rope_results = [r for r in results if r.object_type == "rope"]
    cloth_results = [r for r in results if r.object_type == "cloth"]
    
    pegs_rope_success = np.mean([r.success_pegs for r in rope_results])
    pegs_cloth_success = np.mean([r.success_pegs for r in cloth_results])
    
    # ManiGaussian failure analysis
    mg_failures = [r for r in results if not r.success_mg and r.mg_attempted]
    
    if mg_failures:
        mean_language_dominance = np.mean([r.mg_language_dominance for r in mg_failures])
        mean_action_variance = np.mean([r.mg_action_variance for r in mg_failures])
        fallback_rate = np.mean([r.mg_fallback_triggered for r in mg_failures])
    else:
        mean_language_dominance = None
        mean_action_variance = None
        fallback_rate = None
    
    # PEGS constraint analysis
    pegs_results = [r for r in results if r.pegs_pid_particles_v_corr is not None]
    
    mean_constraint_violation = np.mean([r.pegs_constraint_violations for r in pegs_results])
    mean_particle_coverage = np.mean([r.pegs_particle_coverage for r in pegs_results])
    mean_pegs_synergy = np.mean([r.pegs_pid_particles_v_corr.synergy for r in pegs_results])
    
    return {
        "pegs_overall_success": pegs_success,
        "mg_overall_success": mg_success,
        "pegs_rope_success": pegs_rope_success,
        "pegs_cloth_success": pegs_cloth_success,
        "mg_language_dominance_on_failure": mean_language_dominance,
        "mg_action_variance_on_failure": mean_action_variance,
        "mg_fallback_rate": fallback_rate,
        "pegs_mean_constraint_violation": mean_constraint_violation,
        "pegs_mean_particle_coverage": mean_particle_coverage,
        "pegs_mean_synergy": mean_pegs_synergy,
        "h_wm5_pegs_handles_deformables": pegs_success > 0.5,
        "h_wm5_mg_fails_deformables": mg_success < 0.2,
    }
```

---

## Appendix A: Quick Start Commands
```bash
# 1. Validate estimators (Experiment 0)
# just exp0-bin
 
# 2. Launch PID-Splat environment
# cargo run -p pid-splat -- --scene scenes/simple_pick_place.yaml
 
# 3. Run Experiment 1
# python experiments/run_exp1.py \
#     --config experiments/configs/exp1_pick_place.yaml \
#     --output results/exp1_$(date +%Y%m%d)/ \
#     --seed 42
 
# 4. Analyze results
# python analysis/analyze_exp1.py \
#     --data results/exp1_20260115/ \
#     --output results/exp1_20260115/analysis/
 
# 5. Generate report
# python reports/generate_report.py \
#     --experiment exp1 \
#     --data results/exp1_20260115/ \
#     --output reports/exp1_report.pdf
```

---

## Appendix B: Validation Tools

### B.1 YAML Schema Validator
**Script:** `scripts/validate_yaml_schemas.py`

A comprehensive validator for all scene (`scenes/*.yaml`), object (`objects/*.yaml`), and experiment (`configs/*.yaml`) configuration files.

**Features:**
- **Auto-detection:** Identifies file type (scene, object, deformable, config) based on content.
- **Physics Validation:** Checks mass, friction, and dimensions against realistic bounds.
- **Cross-Validation:** Ensures object IDs referenced in tasks actually exist in the scene.
- **Deformable Checks:** Validates PEGS particle configs (grid size, constraint types).

**Usage:**
```bash
# Validate single file
python scripts/validate_yaml_schemas.py --file assets/objects/novel/l_block.yaml

# Validate entire project
python scripts/validate_yaml_schemas.py --all

# CI Integration (JSON output)
python scripts/validate_yaml_schemas.py --all --json --strict
```
