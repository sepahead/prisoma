# Detailed Experimental Protocols (PID-VLA)

**Context:** This document provides the exact physical, computational, and procedural specifications required to reproduce the experiments defined in `grandplan.md`.

---

## 1. Physical Environment Specifications

### 1.1 Table (Static Object)
- **Physical Dimensions:** 120cm × 80cm × 75cm (standard lab table)
- **Material:** Wood/laminate (for realistic friction μ=0.4)
- **3DGS Capture:**
  - Device: iPhone 15 Pro with Polycam app
  - Capture: 360° orbit, 2-minute video, 4K@30fps
  - Training: `ns-train splatfacto --data ./table_capture --max-num-iterations 30000`
  - Output: `table.spz` (~50MB, ~800K gaussians)
- **Physics Proxy (Rapier):**
  - Type: `ColliderBuilder::cuboid(0.6, 0.4, 0.375)` (half-extents in meters)
  - Position: `[0.0, 0.0, 0.375]` (centered at table surface height)
  - Flags: `RigidBodyType::Fixed` (static body)

### 1.2 Manipulation Objects
| Object | Real Dimensions | Gaussian Count | Physics Proxy | Mass |
|--------|-----------------|----------------|---------------|------|
| Red Cube | 5cm³ | ~15K | `cuboid(0.025, 0.025, 0.025)` | 0.1kg |
| Blue Cylinder | r=3cm, h=8cm | ~20K | `cylinder(0.03, 0.04)` | 0.15kg |
| YCB Mustard Bottle | 19×6×6cm | ~40K | Convex hull (auto) | 0.6kg |
| YCB Spam Can | 9×8×6cm | ~30K | `cuboid(0.045, 0.04, 0.03)` | 0.35kg |

---

## 2. Robot Specifications

### 2.1 Physical Configuration
- **Model:** Franka Emika Panda
- **DOF:** 7 joints + 2 finger gripper
- **Workspace:** 855mm reach
- **Payload:** 3kg max
- **URDF Source:** `franka_description` ROS package

### 2.2 Camera Configuration
| Camera | Position | Resolution | FOV | Purpose |
|--------|----------|------------|-----|---------|
| Wrist (eye-in-hand) | End-effector mount | 640×480 | 69° | Manipulation view |
| Overhead | [0, 0, 1.5m] looking down | 1280×720 | 90° | Global scene |
| Side | [1.0, 0, 0.8m] | 1280×720 | 60° | Evaluation recording |

### 2.3 Action Space Definition
```python
class FrankaAction:
    """7-DOF joint velocities + gripper"""
    joint_velocities: np.ndarray  # shape (7,), rad/s, range [-1.0, 1.0]
    gripper_width: float          # meters, range [0.0, 0.08]
    
    # Alternative: End-effector delta pose
    ee_delta_pos: np.ndarray     # shape (3,), meters, range [-0.05, 0.05]
    ee_delta_rot: np.ndarray     # shape (3,), axis-angle, range [-0.1, 0.1]
```

### 2.4 Proprioception (State Observation)
```python
class FrankaState:
    joint_positions: np.ndarray   # shape (7,), radians
    joint_velocities: np.ndarray  # shape (7,), rad/s
    joint_torques: np.ndarray     # shape (7,), Nm
    ee_pose: np.ndarray           # shape (7,), [x,y,z,qw,qx,qy,qz]
    gripper_width: float          # meters
    gripper_force: float          # Newtons
```

---

## 3. VLA Model Configuration

### 3.1 Model Architecture
- **Model:** OpenVLA (7B)
- **Base:** Llama-2 7B backbone
- **Vision Encoder:** SigLIP + DinoV2 (fused 600M params)
- **Hidden Dimension:** 4096
- **Action Head:** 256-bin discretization per DOF

### 3.2 Embedding Extraction Points
| Layer | Dimension | Description | Zenoh Key |
|-------|-----------|-------------|-----------|
| `vision_encoder.output` | (n_patches, 1024) | Raw visual tokens | `vla/emb/vision` |
| `language_encoder.output` | (n_tokens, 4096) | Instruction embedding | `vla/emb/language` |
| `fusion_layer.output` | (1, 4096) | Fused V+L representation | `vla/emb/fused` |
| `action_head.input` | (1, 4096) | Pre-action hidden state (D) | `vla/emb/action_input` |

### 3.3 Action Tokenization
```python
# OpenVLA uses 256-bin discretization
def tokenize_action(continuous_action: np.ndarray) -> List[int]:
    """Convert continuous action to discrete tokens"""
    bins = np.linspace(-1.0, 1.0, 256)
    tokens = np.digitize(continuous_action, bins) - 1
    return tokens.tolist()  # 7 tokens for joints + 1 for gripper

def detokenize_action(tokens: List[int]) -> np.ndarray:
    """Convert discrete tokens back to continuous"""
    bins = np.linspace(-1.0, 1.0, 256)
    bin_centers = (bins[:-1] + bins[1:]) / 2
    return bin_centers[tokens]
```

### 3.4 Inference Pipeline
```python
# Full inference loop
def vla_inference_step(image: np.ndarray, instruction: str) -> Tuple[Action, Embeddings]:
    # 1. Encode vision
    vision_tokens = vision_encoder(image)  # (256, 1024)
    
    # 2. Encode instruction
    lang_tokens = language_encoder(instruction)  # (n, 4096)
    
    # 3. Fuse
    fused = fusion_layer(vision_tokens, lang_tokens)  # (1, 4096)
    
    # 4. Predict action
    action_logits = action_head(fused)  # (8, 256)
    action_tokens = action_logits.argmax(dim=-1)
    
    # 5. Extract embeddings for PID
    embeddings = {
        'V': vision_tokens.mean(dim=0),  # (1024,)
        'L': lang_tokens.mean(dim=0),    # (4096,)
        'D': fused.squeeze(),            # (4096,)
    }
    
    return detokenize_action(action_tokens), embeddings
```

---

## 4. Specific Experiment Protocols

### Experiment 1: Pick-and-Place (Hypothesis H1, H2)

#### 4.1 Task Definition
- **Instruction:** "Pick up the red cube and place it on the blue plate."
- **Success Criteria:** Cube center within 2cm of plate center, cube stable for 1s
- **Failure Modes:**
  - Miss grasp (gripper closes on air)
  - Drop during transport
  - Placement miss (>5cm from target)
  - Collision with obstacle

#### 4.2 Scene Configuration
```yaml
scene:
  table: table.spz
  objects:
    - id: red_cube
      splat: red_cube.spz
      initial_pose: [0.4, 0.1, 0.025, 0, 0, 0, 1]  # x,y,z,qw,qx,qy,qz
      physics: cuboid
    - id: blue_plate
      splat: blue_plate.spz
      initial_pose: [0.4, -0.2, 0.01, 0, 0, 0, 1]
      physics: cylinder
    - id: distractor_cylinder
      splat: yellow_cylinder.spz
      initial_pose: [0.5, 0.0, 0.04, 0, 0, 0, 1]
      physics: cylinder

robot:
  model: franka_panda
  base_pose: [0, 0, 0, 0, 0, 0, 1]
  initial_joint_config: [0, -0.785, 0, -2.356, 0, 1.571, 0.785]

cameras:
  wrist:
    parent: panda_link8
    offset: [0.05, 0, 0.05]
  overhead:
    pose: [0.4, 0, 1.5, 0.707, 0, 0.707, 0]  # looking down
```

#### 4.3 Data Collection Protocol
```python
# Per-episode data structure
class EpisodeData:
    episode_id: str
    instruction: str
    success: bool
    failure_mode: Optional[str]
    
    # Per-timestep data (T timesteps)
    timestamps: List[float]           # T
    images_wrist: List[np.ndarray]    # T × (480, 640, 3)
    images_overhead: List[np.ndarray] # T × (720, 1280, 3)
    
    # Robot state
    joint_positions: np.ndarray       # T × 7
    ee_poses: np.ndarray              # T × 7
    actions_commanded: np.ndarray     # T × 8
    actions_executed: np.ndarray      # T × 8
    
    # VLA embeddings (extracted at 10Hz)
    embeddings_V: np.ndarray          # T/6 × 1024
    embeddings_L: np.ndarray          # T/6 × 4096  
    embeddings_D: np.ndarray          # T/6 × 4096
    
    # Object tracking (from CoTracker3)
    object_tracks_2d: Dict[str, np.ndarray]  # object_id → T × 2
    object_poses_3d: Dict[str, np.ndarray]   # object_id → T × 7
    
    # PID metrics (computed post-hoc or streaming)
    pid_synergy: np.ndarray           # T/6
    pid_redundancy: np.ndarray        # T/6
    pid_unique_v: np.ndarray          # T/6
    pid_unique_l: np.ndarray          # T/6
```

#### 4.4 Evaluation Metrics
```python
# Primary metrics
success_rate = n_success / n_episodes
grasp_success_rate = n_successful_grasps / n_grasp_attempts

# PID-based metrics
mean_synergy_success = pid_synergy[success_episodes].mean()
mean_synergy_failure = pid_synergy[failure_episodes].mean()
synergy_auroc = roc_auc_score(success_labels, -pid_synergy)  # Negative synergy predicts failure

# Correlation analysis
failure_prediction_corr = pearsonr(pid_synergy, time_to_failure)
```

#### 4.5 Perturbation Experiments (H4: Generalization)
```yaml
perturbations:
  visual:
    - name: lighting_shift
      params: {intensity_delta: [-0.3, 0.3], color_temp: [3000K, 6500K]}
    - name: distractor_injection
      params: {objects: [random_ycb], count: [1, 3], positions: random}
    - name: texture_swap
      params: {target: table, textures: [wood, metal, cloth]}
  
  physical:
    - name: mass_variation
      params: {target: red_cube, scale: [0.5, 2.0]}
    - name: friction_variation
      params: {target: table, mu: [0.2, 0.8]}
    - name: object_position_noise
      params: {sigma_xy: 0.02, sigma_theta: 0.1}  # meters, radians
```

---

## 5. Dream2Flow Experiment Setup

### Experiment 3: Dream2Flow Validation (H5, H6)

#### 5.1 WAN Video Generation
```python
# Input to WAN 2.2
wan_input = {
    "image": current_frame,           # 720×1280×3
    "prompt": f"A robot arm {instruction}. Camera: stationary overhead view.",
    "num_frames": 48,                 # 2 seconds at 24fps
    "guidance_scale": 7.5,
    "seed": episode_seed,
}

# Output
generated_video: np.ndarray  # (48, 720, 1280, 3)
```

#### 5.2 3D Flow Extraction Pipeline
```python
# Step 1: Object segmentation with SAM3
masks = sam3.segment(current_frame, prompts=["red cube", "blue plate"])

# Step 2: Depth estimation
depth_maps = depth_anything_v3(generated_video)  # (48, 720, 1280)

# Step 3: 2D tracking with CoTracker3
tracks_2d = cotracker3.track(
    video=generated_video,
    queries=mask_centroids,  # Initial object centers
)  # (n_objects, 48, 2)

# Step 4: 2D → 3D lifting
camera_intrinsics = K  # 3×3 matrix
tracks_3d = []
for t in range(48):
    for obj_idx, (u, v) in enumerate(tracks_2d[:, t]):
        z = depth_maps[t, int(v), int(u)]
        x = (u - K[0,2]) * z / K[0,0]
        y = (v - K[1,2]) * z / K[1,1]
        tracks_3d.append([x, y, z])

# Result: DreamFlowTrajectory
dream_flow = DreamFlowTrajectory(
    object_id=obj_id,
    points=tracks_3d,        # List of [x,y,z]
    confidence=cotracker3.confidence,
    synergy=[],              # Computed later
)
```

#### 5.3 PID Computation on Flow
```python
# Target: 3D flow trajectory (low-dimensional, Euclidean)
# Sources: V embedding, D embedding

def compute_flow_pid(
    V_embeddings: np.ndarray,    # (T, 1024) - reduced via PCA
    D_embeddings: np.ndarray,    # (T, 256) - reduced via PCA
    flow_trajectory: np.ndarray, # (T, 3) - 3D positions
    k: int = 3,
) -> PidResult:
    """
    Compute I^sx_∩(V, D; Flow) using pid-core.
    
    This sidesteps the manifold geometry problem because:
    1. Flow is 3D Euclidean (no manifold issues)
    2. V and D are PCA-reduced to tractable dimensions
    """
    # Standardize
    V_std = standardize(V_embeddings)
    D_std = standardize(D_embeddings)
    flow_std = standardize(flow_trajectory)
    
    # Call pid-core
    result = pid_core.pid2_isx(
        s1=V_std,
        s2=D_std,
        t=flow_std,
        config=Pid2Config(k=k, method=IsxMethod.EhrlichKsg)
    )
    
    return result  # {synergy, redundancy, unique_s1, unique_s2}
```

---

## 6. Hardware & Compute Requirements

### 6.1 Per-Component
| Component | Hardware | VRAM/RAM | Typical Load |
|-----------|----------|----------|--------------|
| VLA Inference (OpenVLA 7B) | M4 Max (MLX) | 32GB unified | ~2s per action |
| WAN 2.2 Video Gen | A100 (remote) | 40GB | ~30s per 2s video |
| CoTracker3 | M4 Max (MPS) | 8GB | ~5s per video |
| Depth-Anything v3 | M4 Max (MPS) | 4GB | ~1s per frame |
| PID-Core | M4 Max (CPU) | 2GB | <100ms per window |
| Rapier Physics | M4 Max (CPU) | <1GB | <1ms per step |
| SparkJS Rendering | M4 Max (GPU) | 4GB | 60fps stable |

### 6.2 Full Pipeline Latency (Real-time Mode)
```
Image capture      : 0ms (start)
VLA inference      : 2000ms
Action execution   : 16ms
PID computation    : 50ms
Visualization      : 16ms
───────────────────────────
Total loop time    : ~2100ms per action
```

### 6.3 Full Pipeline Latency (With Dream2Flow, Offline Analysis)
```
WAN generation     : 30000ms (async/batched)
Flow extraction    : 6000ms
PID on flow        : 100ms
───────────────────────────
Additional latency : ~36s per episode (post-hoc)
```

---

## 7. Data Formats

### 7.1 HDF5/Zarr Schema
We use Zarr for efficient, chunked storage of trajectory data.

```
/experiment_id
  /metadata
    instruction: string
    success: bool
  /trajectory
    /images (T, H, W, 3)
    /joint_states (T, 7)
    /actions (T, 7)
    /embeddings
      /vision (T, 1024)
      /language (T, 4096)
      /dream (T, 4096)
    /pid_metrics
      /synergy (T)
      /redundancy (T)
```

---

## 8. Reproducibility

- **Seeds:** All stochastic components (VLA sampling, WAN generation, kNN jitter) must be seeded.
- **Versioning:** Record Git commit hash of `pid-vla` repo in experiment metadata.
- **Checkpoints:** Use specific, pinned versions of foundation models (e.g., `openvla-7b-v1.0`, `wan-2.2-v1.0`).
