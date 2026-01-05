# Comprehensive PID-VLA Specification

> **Documentation Set Cross-Reference**: This is the master plan. See also:
> - `pidsplatspecs.md` — Detailed simulation environment and PID specifications
> - `ARCHITECTURE.md` — Component breakdown (Tauri, Modular Physics, 3DGS) and advantages over VLM-based robotics
> - `EXPERIMENTS.md` — Experimental protocols for SparkJS and Modular Physics setup and hypothesis testing
> - `DIAGRAMS.md` — Visual architecture diagrams
> - `README.md` — Quick start guide
## Partial Information Decomposition for Vision-Language-Action Model Diagnostics
### A Critical Technical Analysis with Full Discussion of Approaches, Limitations, and Open Questions

**Version:** 7.0 FINAL (Scientific audit + citation verification + structural cleanup)
**Date:** 2026-01-05
**Status:** Research Specification + Implementation Blueprint (v7.0 audited)
**Canonical:** Living spec; prior versions live in git history.

**v7.0 FINAL notes (Scientific audit & verification pass):**
- Cross-checked arXiv IDs referenced in this document against cached arXiv API metadata (`outputs/arxiv_ref_cache.json`).
- Added DOI metadata for core references (Phys Rev E, PNAS); re-check DOI resolution before publication (no Crossref cache is committed here).
- Corrected at least one incorrect arXiv citation (Wan-Move) and tightened language around unsourced performance/latency/hardware “requirements” (measurement-first).
- Added an agent-native control plane requirement for the PID‑Splat UI stack: the same interventions and inspections exposed in the GUI must be callable via a stable local API (for Claude Code/Codex/opencode-style tooling), with full audit logging and replay.
- Clarified repo reality in §11 (current layout vs planned), updated Python binding examples to the real `pid_core_rs` API, and aligned task-runner docs.
- Added OpenUSD/USDZ interoperability notes based on the LeIsaac Marble tutorial (`.ply → .usdz → .usd`) as an optional workflow.
- Strengthened scientific rigor in §A and §16: clarified δ normalization (`δ_rel`), removed unverified numeric/roadmap claims, and added an explicit geometry→estimator/decomposition decision matrix (2-way vs 3-way vs hierarchical).

> **⚠️ Key caveat (estimator geometry):** The continuous `I^sx_∩` estimator (Ehrlich et al. 2024) relies on Chebyshev (L∞) geometry for exact product-ball cancellations. It **cannot** be applied directly to hyperbolic/Lorentz/manifold embeddings without a **new mathematical derivation** of the disjunction neighborhoods and volume forms in that geometry. Do not simply plug manifold distances into the current estimator.

> **Geometry mitigations (choose one, then re-run the Geometry Gate + Experiment 0):**
> 
> 1. **The "Manifold Unrolling" Approach (Isomap/AE → Standard Estimator):**
>    Use Isomap or Contractive Autoencoders to flatten the manifold into a lower-dimensional Euclidean space (e.g., d=32), then run the standard Ehrlich `I^sx_∩` estimator. This "unrolls" the geometry so L∞ distances become valid proxies.
> 
> 2. **The "Geodesic MI" Approach (Manifold kNN → Shannon Invariants):**
>    Prefer MI/CMI-based screening (CI/Ω / co-information) rather than continuous PID atoms. Use a manifold-aware MI estimator (e.g., Marx & Fischer 2021) with geodesic distances, and treat results as **screening** unless validated under controls.
> 
> 3. **The "Linear Projection" Approach (PCA → Standard Estimator):**
>    Pragmatic baseline. Use PCA to reduce to ~256 dims. PCA is a linear rotation, preserving the "box" volume logic of the Chebyshev estimator better than nonlinear warping, provided the manifold is locally flat enough.
> 
> 4. **The "Quantization" Approach (Clustering → Discrete PID):**
>    Map continuous embeddings to discrete cluster IDs (k=100..1000) using k-means/VQ. Use the classic Discrete `I^sx_∩` estimator (Makkeh et al. 2021), effectively bypassing geometry issues by counting mass instead of volumes.
> 
> 5. **The "Copula Transform" Approach (Rank Transform → Standard Estimator):**
>    Apply empirical CDF transform to every dimension to force Uniform marginals. This mitigates "empty space" issues in high-d L∞ metrics and maximizes estimator efficiency, though it ignores dependencies during the transform.
>
> **Excluded approaches (why they are not default):**
> 
> *   **Kernel Density Estimation (KDE):** Excluded because KDE becomes impractical in very high dimension (e.g., `d≈4096`) due to the curse of dimensionality, and because numerically integrating the `I^sx_∩` disjunction neighborhoods is not tractable at VLA scale.
> *   **Harmonic/Spectral Methods (Diffusion Maps):** Excluded due to cost (exact eigendecomposition is typically `O(N^3)`; scalable approximations exist) and because spectral embeddings can distort local volumes in ways that are hard to correct for PID atoms.
> *   **Naive Geodesic kNN (for PID atoms):** **Violates the v5.5 Warning.** The Ehrlich estimator relies on Euclidean product-volume cancellation ($Vol_{XY} \approx Vol_X \cdot Vol_Y$). Curvature breaks this exact cancellation, making atom estimates invalid. (Contrast with Method 2, which restricts itself to MI/CI where this cancellation is not required).

**v6.7 FINAL notes (Unified Splat-First Simulation Environment — Complete Architecture):**

> **🎯 Design Goal:** A hardware-free simulation environment for VLA data collection, training, and PID analysis, optimized for fast iteration and splat-first rendering; benchmark against Isaac Sim / Omniverse / Gazebo on matched tasks (avoid “outperforms” claims without a shared protocol).

---

### §A. Critical Analysis: Why This Architecture Over Competitors

**From first principles, what does *PID-based VLA diagnosis* require (beyond “a simulator”)?**
1. **Closed-loop instrumentation** → time-synchronized logging of `(obs, state, actions, embeddings, interventions)` with reproducible replay
2. **Controlled interventions** → perturb vision/language/physics/world-model inputs *without* changing task identity (placebo + nuisance controls)
3. **External targets/labels** → `A*` (teacher/optimal action), success/failure labels, and counterfactual hooks to ground “failure” claims
4. **Rendering + physics appropriate to the task** → visual realism helps *reduce* domain gaps; contact fidelity matters for manipulation; both must be benchmarked
5. **Iteration speed + cross-platform** → rapid ablations on researcher hardware; avoid locking the entire pipeline to one vendor/runtime
6. **Hybrid scene representations** → splats for photoreal views, meshes/URDFs for collisions/robots (and for “what-if” edits)
7. **World-model integration** → a clean interface to plug in WAN-like video models and flow extraction as *experimental variables*
8. **Agent-native control plane** → the GUI is not a dead-end: provide a stable automation API for live scene edits, interventions, and inspection (designed for LLM coding tools), and log every remote action as a first-class event for reproducibility

**Honest comparison with existing platforms:**

**Note on comparisons:** Use these as capability notes, not a performance ranking. Any “sim2real %”, fps, latency, footprint, or “better physics” claim must be tied to a specific benchmark + hardware + protocol.

| Platform | Rendering | Physics | Availability / constraints | PID-centric diagnostic loop |
|----------|-----------|---------|----------------------------|-----------------------------|
| **Isaac Sim/Lab** | Omniverse RTX / OpenUSD | PhysX | NVIDIA GPU required (verify supported OS/driver) | No (requires custom harness) |
| **Omniverse (general)** | Omniverse RTX / OpenUSD | PhysX | NVIDIA GPU required | No (not PID-centric) |
| **MuJoCo** | OpenGL raster | MuJoCo | Cross-platform | No (instrumentation is DIY) |
| **Gazebo (Classic/Ignition)** | OGRE raster | ODE/DART (varies) | Cross-platform; ROS-centric | No (instrumentation is DIY) |
| **SplatSim** | 3DGS rendering | Not the main contribution (see paper) | Research code; verify constraints | No (not a PID harness) |
| **DISCOVERSE** | 3DGS rendering | MuJoCo (paper) | Research code; verify constraints | No (not a PID harness) |
| **Proposed (Modular)** | 3DGS + mesh overlays | Modular (Rapier + baselines) | Cross-platform target | Yes (explicit goal) |

**Do current systems provide the closed-loop PID diagnostics needed here?**
- **They can be instrumented, but it is not “turn-key”.** Isaac Sim/Lab, MuJoCo, and Gazebo all support logging and Python integration, but none ship a PID-centric experiment harness with geometry gates, preregistered interventions, and synchronized `(V,L,D,A)` embedding capture as a first-class feature.
- **This project’s goal is not “a better simulator”; it is a better *diagnostic loop*.** The architectural differentiation is the explicit contract for (a) interventions and provenance, (b) synchronized logging/replay, and (c) in-loop computation/visualization of information-theoretic diagnostics.

**WAN-like video models and Dream2Flow-style bridges**
- Most simulators do not include “world model → flow → diagnostic target” pipelines out of the box. Here they are treated as external services with pinned versions, and as experimental variables rather than infrastructure assumptions.

**Hybrid splats + meshes (and why Three.js/WebGPU matters)**
- 3DGS is strong for photoreal *views* of captured scenes; meshes/URDFs remain the practical choice for articulated robots, collision proxies, and precise interactive edits. The proposed stack assumes hybrid rendering/physics: splats for appearance, meshes for dynamics and contact.

**Key advantages of proposed architecture:**
1. **Splat-first rendering (visual realism when capture is good)** — 3DGS can reduce *visual* domain gaps on some tasks; SplatSim reports 86.25% average zero-shot sim2real success on their benchmark (arXiv:2409.10161). Treat transfer rates as benchmark-specific.
2. **Hybrid splats + meshes** — Use splats for photoreal views and meshes/URDFs for collisions, robots, and precise “what-if” edits (Three.js/WebGPU can render both).
3. **Programmable overlays (Dynos)** — GPU-side splat recoloring/annotation makes PID diagnostics inspectable in real time (treat SparkJS implementation details as external).
4. **Modular physics choice** — Rapier (fast iteration, deterministic) vs MuJoCo/PhysX-class engines (contact baselines); avoid claiming “better” without matched evaluation.
5. **Closed-loop PID instrumentation as a first-class requirement** — explicit hooks for perturbations, provenance, and synchronized logs to make PID hypotheses testable.
6. **World-model integration is explicit** — WAN-like video models + flow extraction are treated as plug-in experimental variables, not baked into the simulator.
7. **Lower operational footprint (by design)** — a Tauri/WebGPU app (Rust backend + system WebView) can be substantially lighter to install/run than full Omniverse/Isaac stacks; exact size/throughput is packaging- and hardware-dependent, and other shells (Electron/Qt/Unity) remain viable.
8. **UI + automation are the same surface** — every action possible in the GUI (spawn/move objects, apply perturbations, scrub timeline, export logs) is also exposed via a stable local API (“Agent Bridge”), enabling reproducible live interventions by scripts and LLM tools (no hidden manual steps).

**Honest disadvantages:**
- **Not a GPU-parallel RL factory** — Isaac Gym/Lab can run O(10³) environments; this architecture prioritizes instrumented, reproducible runs (interactive or small-batch).
- **Physics validity is an empirical question** — Rapier is not the gold standard for contact-rich manipulation; use MuJoCo/PhysX baselines when physics fidelity is central.
- **Rendering trade-offs** — 3DGS is typically rasterized and does not automatically provide RTX-grade lighting/material effects; dynamic/interactive edits require hybrid representations.
- **Dependency/roadmap risk** — do not justify the research plan on third-party roadmap or marketing claims (e.g., “500M splats”, “multiplayer”); treat as unverified until benchmarked.

#### §A.1 Value Proposition (and when it is *not* justified)

This project’s scientific contribution is PID‑based diagnostics under controlled interventions—not “a new simulator”. A custom stack is justified only if it enables experiments that are otherwise impractical or error-prone to reproduce. The minimum bar for justification is:
1. **Intervention + replay as a first-class primitive** (episode-level, not just logs)
2. **Synchronized capture of** `(V,L,D,A)` **representations** alongside simulator state, prompts, and labels
3. **One control plane for both GUI and automation** so manual runs and LLM/tool-driven runs are comparable and auditable

If these properties can be achieved with acceptable effort on Isaac Sim/Lab, MuJoCo, or Gazebo for the target tasks, then PID‑VLA should use those systems and focus engineering effort on the experiment harness.

#### §A.2 Why Tauri (and why SparkJS is optional)

- **Tauri (shell):** provides a cross-platform desktop host with a Rust backend (for PID computation, run logging, and the Agent Bridge) and a WebView frontend (for a modern GPU UI).
- **Renderer (pluggable):** SparkJS is a candidate WebGPU 3DGS renderer; Three.js/WebGPU or other engines are acceptable if they meet interface requirements (version pinning, stable camera model, mesh overlays, and metric‑driven overlays).
- **Do not rely on ecosystem narratives:** “future-proof” arguments based on third‑party funding/roadmaps are not scientific evidence. Pin revisions, benchmark, and design for swap‑ability.

#### §A.3 Hybrid scene geometry: splats + meshes (+ anything WebGPU can draw)

- **Splats are appearance, not physics.** Use 3DGS for photoreal views of captured scenes; use meshes/URDFs/primitive colliders for contacts, robots, and interactive edits.
- **Compositing is a requirement:** 3DGS background + mesh robots/objects + optional point clouds/voxels/SDF debug views + annotation gizmos (selection, transforms, constraints).
- **Why this matters for PID hypotheses:** it enables controlled *visual* perturbations without changing physical state, controlled *physics* perturbations without changing appearance, and “what‑if” edits whose provenance is logged and replayable.

#### §A.4 Expert-perspective design review (10 lenses)

| Expert lens | Likely objection | Design response / constraint |
|---|---|---|
| Robotics simulation engineer | “You’ll reinvent Isaac/MuJoCo poorly.” | Do not compete on generality; benchmark on matched tasks; keep physics swappable; keep experiments the focus. |
| Graphics/renderer engineer | “3DGS can’t handle dynamics/materials.” | Treat splats as a static appearance layer; dynamic entities are meshes; overlays are diagnostic, not photoreal PT. |
| Control/robotics researcher | “Where is the ground truth / teacher?” | Require `A*` or external labels where causal claims are made; stage-wise logging prevents misattribution. |
| PID / information theory expert | “Estimator validity dominates conclusions.” | Enforce Geometry Gate + Experiment 0; downgrade to MI/CI/Ω/discrete PID when geometry fails. |
| VLA researcher | “D is ill-defined across models.” | Define D per model (explicit vs implicit); prefer decompositions that match operational variables; treat D extraction as an experimental variable. |
| Systems/performance engineer | “Real-time budgets are fantasy.” | Default to offline-first; measure budgets; make in-loop components opt-in and logged. |
| MLOps/reproducibility engineer | “Your runs won’t be replayable.” | Event-sourced run logs; deterministic seeds/settings; artifact hashes; same control plane for GUI and API. |
| Security/privacy engineer | “LLM tooling + local control plane is risky.” | Localhost-only by default; explicit auth token; audit logs; capability gating; safe-mode that disallows file/network actions. |
| HCI/UX designer | “Diagnostics will be unreadable.” | Timeline-first UI; semantic overlays; consistent encodings; every visualization maps to an exported numeric artifact. |
| Open-source maintainer | “Scope creep will kill the project.” | Keep simulator minimal; prioritize Experiment 0→3; treat world-model extras as optional modules with clear boundaries. |

#### §A.5 Utility beyond PID (why the broader community might care)

Even if PID results are negative, the infrastructure can remain useful as a **general VLA diagnostics harness**: stage-wise logging/replay, intervention tooling, hybrid 3DGS+mesh visualization, and an agent-native control plane for automated debugging and dataset generation.

#### §A.6 Recommended sequencing (risk-reducing “expert consensus”)

The lowest-risk path is to add complexity only after each prior layer is validated:

1. **Estimator gate (Exp0):** validate the estimator regime on synthetic data (including strong-dependence cases) before touching VLA embeddings.
2. **Harness bring-up (no predictor):** build logging/interventions/replay and compute **`Flow_gt`** from simulator object poses (no video model; no oracle claims).
3. **Small baseline (SmolVLA or toy policy):** validate embedding extraction + run metadata + geometry gate behavior on a cheap, reproducible model.
4. **Primary VLA target (OpenVLA or equivalent):** run Aim 1/2 experiments under matched controls; treat diffusion-based VLAs as a separate axis to test *after* baseline replication.
5. **Predictor-driven Flow (Dream2Flow/WAN):** integrate external video→flow only when you need embodiment-gap studies or when you want to compare `Flow_pred` to `Flow_gt` as an additional diagnostic layer.

**Why not start with video predictors?** They add major confounds (stochastic generation, heavy compute, opaque failure modes). Starting with `Flow_gt` tests the PID hypotheses about integration with a clean Euclidean variable before introducing an additional model.

---

### §B. Complete System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                           PID-VLA UNIFIED SIMULATION ENVIRONMENT                         │
│                              (Tauri + SparkJS + Modular Physics)                         │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                          │
│  ┌─────────────────────────────────────────────────────────────────────────────────────┐│
│  │                              TAURI APPLICATION SHELL                                 ││
│  │  ┌─────────────────────────────────────────────────────────────────────────────────┐││
│  │  │               SPARKJS RENDERING LAYER (WebGPU; fallback WebGL2)                 │││
│  │  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐  ┌─────────────────┐  │││
│  │  │  │  3DGS Splats  │  │  Mesh Overlay │  │  UI Overlays  │  │  PID Heatmaps   │  │││
│  │  │  │  (PLY/SPZ/    │  │  (GLB/GLTF)   │  │  (Three.js)   │  │  (Syn/Red/Unq)  │  │││
│  │  │  │   SPLAT/SOG)  │  │  Collision    │  │  Camera feeds │  │  Color-coded    │  │││
│  │  │  └───────────────┘  └───────────────┘  └───────────────┘  └─────────────────┘  │││
│  │  │                              ↕ Dyno Pipeline                                    │││
│  │  │  ┌─────────────────────────────────────────────────────────────────────────────┐│││
│  │  │  │  PROCEDURAL SPLAT SYSTEM (SparkJS Dynos)                                    ││││
│  │  │  │  • Weather effects (rain, fog, dust as dynamic splats)                      ││││
│  │  │  │  • Lighting changes (time-of-day, shadows via splat opacity)                ││││
│  │  │  │  • Domain randomization (procedural texture/color variation)                 ││││
│  │  │  │  • Real-time splat editing (select, move, scale, delete, clone)             ││││
│  │  │  └─────────────────────────────────────────────────────────────────────────────┘│││
│  │  └─────────────────────────────────────────────────────────────────────────────────┘││
│  │                                         ↕ IPC                                        ││
│  │  ┌─────────────────────────────────────────────────────────────────────────────────┐││
│  │  │                           RUST BACKEND (Tauri Core)                             │││
│  │  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐  ┌─────────────────┐  ┌─────────────────┐  │││
│  │  │  │ Physics Engine │  │  pid-core     │  │  Asset Manager│  │  Zenoh Client   │  │  Agent Bridge   │  │││
│  │  │  │ (Rapier/MuJoCo)│  │  (I^sx_∩)     │  │  (PLY/SPZ/GLB)│  │  (ROS 2 bridge) │  │ (MCP/JSON-RPC)  │  │││
│  │  │  │ • Collision  │  │  • Estimator  │  │  • Import     │  │  • Pub/Sub      │  │  • Live control │  │││
│  │  │  │ • Dynamics   │  │  • Bootstrap  │  │  • Convert    │  │  • Zero-copy    │  │  • Introspection│  │││
│  │  │  │ • Raycasting │  │  • Stream     │  │  • LOD        │  │  • Latency: measure ││  • Audit log    │  │││
│  │  │  └───────┬───────┘  └───────┬───────┘  └───────┬───────┘  └────────┬────────┘  └────────┬────────┘  │││
│  │  │          │                  │                  │                   │                   │           │││
│  │  │          └──────────────────┴──────────────────┴───────────────────┴───────────────────┘           │││
│  │  │                                    ↕                                            │││
│  │  │  ┌─────────────────────────────────────────────────────────────────────────────┐│││
│  │  │  │  PEGS-STYLE DUAL REPRESENTATION                                             ││││
│  │  │  │  ┌─────────────────────────────┐  ┌─────────────────────────────────────┐   ││││
│  │  │  │  │  PARTICLE LAYER (Physics)   │  │  GAUSSIAN LAYER (Rendering)         │   ││││
│  │  │  │  │  • Rapier rigid bodies      │←→│  • SparkJS splats (visual)          │   ││││
│  │  │  │  │  • Collision shapes         │  │  • Attached to particles            │   ││││
│  │  │  │  │  • Joint constraints        │  │  • "Visual forces" correction       │   ││││
│  │  │  │  │  • Contact forces           │  │  • Predicted vs observed sync       │   ││││
│  │  │  │  └─────────────────────────────┘  └─────────────────────────────────────┘   ││││
│  │  │  └─────────────────────────────────────────────────────────────────────────────┘│││
│  │  └─────────────────────────────────────────────────────────────────────────────────┘││
│  └─────────────────────────────────────────────────────────────────────────────────────┘│
│                                          ↕ Zenoh (latency: measure)                      │
│  ┌─────────────────────────────────────────────────────────────────────────────────────┐│
│  │                         HEADLESS GAZEBO (Physics + Sensors)                          ││
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐  ┌─────────────────────┐  ││
│  │  │  Robot URDF   │  │  Sensor Sim   │  │  World State  │  │  ROS 2 Interface    │  ││
│  │  │  • Franka     │  │  • RGB-D      │  │  • Poses      │  │  • /joint_states    │  ││
│  │  │  • WidowX     │  │  • LiDAR      │  │  • Velocities │  │  • /cmd_vel         │  ││
│  │  │  • GR1       │  │  • Force/Torque│  │  • Contacts   │  │  • /camera/image    │  ││
│  │  │  • Custom    │  │  • IMU        │  │  • Scene graph│  │  • /tf              │  ││
│  │  └───────────────┘  └───────────────┘  └───────────────┘  └─────────────────────┘  ││
│  └─────────────────────────────────────────────────────────────────────────────────────┘│
│                                          ↕ Zenoh                                        │
│  ┌─────────────────────────────────────────────────────────────────────────────────────┐│
│  │                              VLA INFERENCE (External)                                ││
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐  ┌─────────────────────┐  ││
│  │  │  OpenVLA      │  │  DreamVLA     │  │  PixelVLA     │  │  Custom Policy      │  ││
│  │  │  (7B, CUDA)   │  │  (D states)   │  │  (visual)     │  │  (external)         │  ││
│  │  └───────────────┘  └───────────────┘  └───────────────┘  └─────────────────────┘  ││
│  └─────────────────────────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

---

### §C. Asset Pipeline and Format Support

**Target formats (renderer support varies; verify with chosen stack):**

| Format | Type | Use Case | Import Method |
|--------|------|----------|---------------|
| **.PLY** | Point cloud / Splats | 3DGS scenes, COLMAP output | Renderer-native (SparkJS or equivalent) |
| **.SPZ** | Compressed splats | Optimized 3DGS storage | Renderer-native (SparkJS or equivalent) |
| **.SPLAT** | Raw splats | Fast loading | Renderer-native (SparkJS or equivalent) |
| **.KSPLAT** | Keyframed splats | Animated splats | Renderer-native (SparkJS or equivalent) |
| **.SOG** | Structured splats | Semantic splats | Renderer-native (SparkJS or equivalent) |
| **.GLB/.GLTF** | Mesh + materials | Collision geometry, robots | Three.js loader → collision mesh |
| **.URDF** | Robot description | Gazebo robots | Gazebo native |
| **.USD/.USDA/.USDC** | OpenUSD scene | Isaac Sim / LeIsaac interop; scene composition | Isaac Sim native; optional converter into PID‑Splat scene graph (planned) |
| **.USDZ** | Packaged OpenUSD | Bundle USD + textures/assets for distribution | Unpack to `.usda/.usdc` (e.g., LeIsaac workflow); import method depends on tooling |

**Asset workflow:**
```
Real Scene Capture    3DGS Reconstruction    Renderer Import   Physics Binding
─────────────────    ──────────────────    ──────────────    ──────────────────
iPhone/DSLR video → COLMAP + gsplat/nerfstudio → .PLY/.SPZ → Rapier collision mesh
                                                    ↓
                                              Dyno pipeline
                                              (edit, randomize)
```

**OpenUSD/Isaac Sim interop note (LeIsaac/Marble-style workflow):**
- LeIsaac’s Marble tutorial converts Gaussian splat `.ply` → `.usdz` (packaged OpenUSD) using NVIDIA 3DGrut, then combines the splat scene with a collision mesh inside Isaac Sim and exports a single `.usd` “background scene”.
- This project can adopt the same interoperability pattern when Isaac Sim/LeIsaac tooling is useful (e.g., to validate collision proxies or reuse OpenUSD scene composition), while still using SparkJS for splat rendering and mesh/URDF for physics in the PID‑Splat stack.

#### C.1 Optional: PLY → USDZ → USD (Isaac Sim / LeIsaac Interop)

LeIsaac’s Marble tutorial illustrates a concrete OpenUSD bridge for Gaussian splats:
1. Convert splats to USDZ (packaged OpenUSD) using 3DGrut:
   - `python -m threedgrut.export.scripts.ply_to_usd path/to/your/splats.ply --output_file path/to/output.usdz`
2. Unpack the `.usdz` (it contains a `default.usda` stage) and load it in Isaac Sim for rendering.
3. Reference a collision mesh (e.g., a decimated `.glb`) under `/World/Xform`, align transforms so splats and mesh overlap, and configure colliders.
4. Export a single `.usd` stage as the “background scene” for downstream task composition.

This is optional in PID‑VLA: it is an interoperability path for OpenUSD ecosystems, not a requirement for the Tauri/SparkJS/Gazebo stack.

---

### §D. PEGS-Style Dual Gaussian-Particle Representation

**Implementation in Modular Physics + SparkJS:**

```rust
// Rust backend: Particle-Physics layer (Generic Trait over Rapier/MuJoCo)
pub struct PhysicsParticle {
    body_handle: RigidBodyHandle,      // Generic handle
    collider_handle: ColliderHandle,   // Generic handle
    splat_indices: Vec<u32>,           // Attached SparkJS splat IDs
    mass: f32,
    material: PhysicsMaterial,
}

pub struct GaussianBinding {
    particle_id: u32,
    local_offset: Vec3,                // Offset from particle center
    scale_factor: f32,                 // Scale relative to particle
}

// Sync loop: Physics → Rendering
pub fn sync_particles_to_splats(
    physics: &dyn PhysicsBackend,      // Modular backend trait
    splat_transforms: &mut SplatTransformBuffer,
) {
    for particle in physics.particles().iter() {
        let transform = physics.get_body_transform(particle.body_handle);
        for &splat_idx in &particle.splat_indices {
            splat_transforms[splat_idx] = transform * particle.bindings[splat_idx].local_offset;
        }
    }
}
```

```typescript
// SparkJS frontend: Visual correction (PEGS-style "visual forces")
class VisualForceCorrector {
    computeVisualForces(predicted: RenderBuffer, observed: CameraFrame): ForceField {
        // Compare predicted render vs actual camera feed
        const diff = this.imageDifference(predicted, observed);

        // Convert pixel differences to 3D forces on splats
        const forces = this.backprojectToForces(diff, this.camera);

        // Send corrections to Rust backend
        return forces;
    }
}
```

---

### §E. Collision Detection on Gaussian Splats

**FOCI-inspired approach (Field Overlap Collision Integral):**

```rust
// Gaussian-Gaussian collision via overlap integral
pub fn gaussian_overlap_collision(
    g1: &GaussianSplat,
    g2: &GaussianSplat,
) -> Option<CollisionContact> {
    // Compute overlap integral between two Gaussians
    let overlap = compute_gaussian_overlap(
        g1.mean, g1.covariance,
        g2.mean, g2.covariance,
    );

    if overlap > COLLISION_THRESHOLD {
        // Penetration depth proportional to overlap
        let depth = overlap.ln() / COLLISION_SENSITIVITY;
        let normal = (g2.mean - g1.mean).normalize();

        Some(CollisionContact {
            point: (g1.mean + g2.mean) / 2.0,
            normal,
            depth,
        })
    } else {
        None
    }
}

// Hybrid approach: Gaussian for proximity, mesh for precision
pub struct HybridCollider {
    gaussian_cloud: Vec<GaussianSplat>,  // Fast broad-phase
    collision_mesh: TriMesh,              // Precise narrow-phase (from GLB)
}
```

**Raycasting on splat fields:**

```rust
pub fn raycast_splats(
    ray: Ray,
    splats: &SplatCloud,
    max_hits: usize,
) -> Vec<SplatHit> {
    let mut hits = Vec::new();

    for (idx, splat) in splats.iter().enumerate() {
        // Ray-ellipsoid intersection (splat as 3D Gaussian ellipsoid)
        if let Some(t) = ray_gaussian_intersection(&ray, splat) {
            hits.push(SplatHit {
                splat_index: idx,
                distance: t,
                point: ray.origin + ray.direction * t,
                density: splat.opacity_at(ray.origin + ray.direction * t),
            });
        }
    }

    hits.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
    hits.truncate(max_hits);
    hits
}
```

---

### §F. Environment Simulation (Lighting, Weather, Domain Randomization)

**SparkJS Dyno-based procedural effects:**

```typescript
// Weather as dynamic splat modification
class WeatherDyno extends SparkDyno {
    // Rain: spawn temporary splats with downward velocity
    rain(intensity: number): void {
        const rainSplats = this.generateRainDrops(intensity);
        this.splats.addTemporary(rainSplats, { lifetime: 2.0 });
    }

    // Fog: modify all splat opacities based on distance
    fog(density: number, falloff: number): void {
        this.splats.forEach(splat => {
            const distance = splat.position.distanceTo(this.camera.position);
            splat.opacity *= Math.exp(-density * distance / falloff);
        });
    }

    // Dust: particle system using splats
    dust(windDirection: Vec3, particleCount: number): void {
        const dustSplats = this.generateDustParticles(particleCount);
        dustSplats.forEach(s => s.velocity = windDirection.multiplyScalar(Math.random()));
        this.splats.addDynamic(dustSplats);
    }
}

// Lighting as splat color/opacity modification
class LightingDyno extends SparkDyno {
    timeOfDay(hour: number): void {
        const sunAngle = (hour - 6) * Math.PI / 12;  // 6am = 0, 6pm = π
        const sunColor = this.computeSunColor(sunAngle);
        const shadowDirection = new Vec3(Math.cos(sunAngle), -Math.sin(sunAngle), 0);

        this.splats.forEach(splat => {
            // Ambient occlusion approximation
            const occlusion = this.computeOcclusion(splat, shadowDirection);
            splat.color = splat.baseColor.multiply(sunColor).multiply(1 - occlusion * 0.5);
        });
    }
}

// Domain randomization for sim2real
class DomainRandomizationDyno extends SparkDyno {
    randomizeTextures(variance: number): void {
        this.splats.forEach(splat => {
            splat.color.r += (Math.random() - 0.5) * variance;
            splat.color.g += (Math.random() - 0.5) * variance;
            splat.color.b += (Math.random() - 0.5) * variance;
        });
    }

    randomizeLighting(variance: number): void {
        const perturbation = new Vec3(
            (Math.random() - 0.5) * variance,
            (Math.random() - 0.5) * variance,
            (Math.random() - 0.5) * variance,
        );
        this.lightDirection = this.baseLightDirection.add(perturbation).normalize();
    }
}
```

---

### §G. Camera System and Streaming

```typescript
// Virtual camera with raycast-based depth
class VirtualCamera {
    position: Vec3;
    orientation: Quat;
    fov: number;
    resolution: [number, number];

    // Render to offscreen buffer
    render(scene: SparkScene): ImageBuffer {
        return this.renderer.renderToBuffer(scene, this);
    }

    // Raycast depth (on splat field, not mesh)
    getDepthMap(): Float32Array {
        const depth = new Float32Array(this.resolution[0] * this.resolution[1]);
        for (let y = 0; y < this.resolution[1]; y++) {
            for (let x = 0; x < this.resolution[0]; x++) {
                const ray = this.pixelToRay(x, y);
                const hits = raycast_splats(ray, this.scene.splats, 1);
                depth[y * this.resolution[0] + x] = hits[0]?.distance ?? Infinity;
            }
        }
        return depth;
    }

    // Stream via Zenoh to ROS 2
    stream(zenohSession: ZenohSession, topic: string): void {
        setInterval(() => {
            const frame = this.render(this.scene);
            zenohSession.put(topic, frame.toBytes());
        }, 1000 / this.fps);
    }
}

// Camera placement via raycasting
class CameraPlacementTool {
    placeCamera(clickPosition: Vec2, scene: SparkScene): VirtualCamera {
        const ray = this.screenToRay(clickPosition);
        const hit = raycast_splats(ray, scene.splats, 1)[0];

        if (hit) {
            const camera = new VirtualCamera();
            camera.position = hit.point.add(hit.normal.multiplyScalar(0.5));
            camera.lookAt(hit.point);
            return camera;
        }
        return null;
    }
}
```

---

### §H. Real-Time Splat Editing

```typescript
// Selection and manipulation
class SplatEditor {
    selectedSplats: Set<number> = new Set();

    // Box select
    selectBox(min: Vec2, max: Vec2): void {
        this.scene.splats.forEach((splat, idx) => {
            const screenPos = this.camera.project(splat.position);
            if (screenPos.x >= min.x && screenPos.x <= max.x &&
                screenPos.y >= min.y && screenPos.y <= max.y) {
                this.selectedSplats.add(idx);
            }
        });
    }

    // Raycast select
    selectRaycast(screenPos: Vec2): void {
        const ray = this.camera.screenToRay(screenPos);
        const hits = raycast_splats(ray, this.scene.splats, 1);
        if (hits.length > 0) {
            this.selectedSplats.add(hits[0].splat_index);
        }
    }

    // Transform selected
    translate(delta: Vec3): void {
        this.selectedSplats.forEach(idx => {
            this.scene.splats[idx].position.add(delta);
        });
        this.syncToPhysics();
    }

    scale(factor: number): void {
        this.selectedSplats.forEach(idx => {
            this.scene.splats[idx].scale.multiplyScalar(factor);
        });
    }

    delete(): void {
        this.selectedSplats.forEach(idx => {
            this.scene.splats.markDeleted(idx);
        });
        this.scene.splats.compact();  // Remove deleted
        this.selectedSplats.clear();
    }

    clone(): void {
        const newSplats = [];
        this.selectedSplats.forEach(idx => {
            newSplats.push(this.scene.splats[idx].clone());
        });
        this.scene.splats.addAll(newSplats);
    }

    // Sync edits to Rapier physics
    syncToPhysics(): void {
        this.tauriBackend.invoke('update_splat_physics', {
            changes: this.pendingChanges
        });
    }
}

// Mesh editing (for collision geometry)
class MeshEditor {
    selectedMesh: GLBMesh | null = null;

    importGLB(file: File): Promise<GLBMesh> {
        return this.gltfLoader.load(file).then(gltf => {
            const mesh = new GLBMesh(gltf);
            this.scene.addMesh(mesh);
            // Auto-generate collision shape
            mesh.collider = this.generateConvexHull(mesh);
            return mesh;
        });
    }

    adjustCollider(mesh: GLBMesh, type: 'convex' | 'trimesh' | 'box'): void {
        switch (type) {
            case 'convex':
                mesh.collider = this.generateConvexHull(mesh);
                break;
            case 'trimesh':
                mesh.collider = this.generateTriMesh(mesh);
                break;
            case 'box':
                mesh.collider = this.generateBoundingBox(mesh);
                break;
        }
        this.syncColliderToRapier(mesh);
    }
}
```

---

### §I. Sim2Real Gap Closure Strategy

**Multi-layered approach motivated by 3DGS-based simulation work (SplatSim, arXiv:2409.10161; DISCOVERSE, arXiv:2507.21981) and PEGS-style explicit physics:**

| Layer | Strategy | Implementation | What to Measure (avoid pre-committing %) |
|-------|----------|----------------|---------------------|
| **Visual** | 3DGS rendering | SparkJS splat-first | Δ zero-shot transfer vs mesh baseline on the *same tasks*; report CIs |
| **Domain Randomization** | Dyno-based procedural | Weather, lighting, texture variance | Robustness under perturbation suites; ablation vs no-randomization |
| **Physics Alignment** | PEGS visual forces | Predicted vs observed correction | Trajectory/contact error vs reference; downstream effect on transfer |
| **Sensor Noise** | Gazebo sensor models | Realistic camera/depth noise | Sensitivity to sensor corruption; calibration against real sensors |
| **Action Noise** | Randomized execution | Torque/velocity perturbations | Robustness to execution noise; transfer across controllers |

**Outcome target:** define success metrics on a named benchmark and report improvements via controlled ablations; do not treat the layers as additively contributing “% improvements” unless measured.

---

### §J. Complete Data Collection Pipeline

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        DATA COLLECTION WORKFLOW                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  1. SCENE SETUP                                                              │
│     ├── Import 3DGS scene (.PLY/.SPZ from real capture or synthetic)        │
│     ├── Import robot URDF (Gazebo)                                          │
│     ├── Place virtual cameras (raycast click-to-place)                      │
│     └── Configure domain randomization (weather, lighting, textures)        │
│                                                                              │
│  2. TASK DEFINITION                                                          │
│     ├── Define goal states (object positions, gripper states)               │
│     ├── Specify reward function (for RL) or demonstration protocol          │
│     └── Set episode length and termination conditions                       │
│                                                                              │
│  3. DATA GENERATION                                                          │
│     ├── Teleoperation: SpaceNav/keyboard → Robot Sim → Physics sync        │
│     ├── OR scripted policies: Predefined trajectories with noise           │
│     ├── OR RL training: PPO/SAC with PID-VLA reward shaping                │
│     └── Domain randomization applied per-episode                            │
│                                                                              │
│  4. DATA RECORDING                                                           │
│     ├── RGB frames: SparkJS render @ 30Hz → Zenoh → HDF5                   │
│     ├── Depth: Splat raycast depth @ 30Hz                                   │
│     ├── Actions: Joint positions/velocities @ 100Hz                         │
│     ├── States: Object poses, contacts @ 100Hz                              │
│     ├── VLA embeddings: V, D, A per timestep                                │
│     └── PID metrics: Syn(V,D;A) computed inline                             │
│                                                                              │
│  5. EXPORT                                                                   │
│     ├── RLDS format (TensorFlow Datasets) for OpenVLA fine-tuning          │
│     ├── HDF5 for custom training                                            │
│     └── Zarr for large-scale storage                                        │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

### §K. VLA Paper Simulation Setups Analysis

**How existing VLAs collect data and train:**

| VLA | Primary Simulator | Physics | Rendering | Data Format | Notes |
|-----|-------------------|---------|-----------|-------------|-------|
| **OpenVLA** | SimplerEnv/LIBERO | MuJoCo | OpenGL raster | RLDS | Large RLDS-scale dataset (paper; verify counts before quoting) |
| **DreamVLA** | Unspecified | Unspecified | Unspecified | Unspecified | Focus on world model, not sim |
| **PixelVLA** | Pixel-160K dataset | Various | Various | Custom | Visual prompting focus |
| **TraceVLA** | SimplerEnv | MuJoCo | OpenGL raster | RLDS | Visual trace overlays |
| **RT-2** | Proprietary | Proprietary | Photorealistic | Proprietary | Google internal |
| **SpatialVLA** | SimplerEnv | MuJoCo | OpenGL raster | RLDS | Spatial reasoning focus |
| **Dream2Flow** | Real robot + sim | Unknown | Video generation | Unknown | Video→flow→action |

**Gap analysis:**
- Many widely-used public VLA benchmarks/pipelines (e.g., LIBERO/SimplerEnv) use MuJoCo + raster rendering, but simulator/renderer choices vary across papers; verify per model before making “all” claims.
- 3DGS-based rendering is not yet a default in mainstream VLA training pipelines (as of the papers reviewed here). Treat SplatSim’s transfer numbers as benchmark-specific evidence, not a guarantee.
- Interactive scene composition/editing exists in some simulators (e.g., OpenUSD tooling in Omniverse/Isaac Sim), but is not typically integrated as a PID-centric *intervention harness* for controlled experiments.
- PID/SxPID diagnostics are not standard evaluation metrics in open-source VLA training loops; this work makes them first-class (after Experiment 0 + geometry gates).

**Proposed system fills these gaps.**

---

### §L. Hardware Requirements

Hardware needs are workload-dependent (scene size, splat count, physics engine, and whether you run external video/world models locally). Treat the table below as planning guidance; measure and report on your actual hardware.

| Profile | CPU/GPU | RAM | Storage | Use Case |
|---------|---------|-----|---------|----------|
| **Estimator dev (Experiment 0)** | CPU-only OK; GPU optional | 16GB+ | modest | `cargo test`, `just exp0`, geometry diagnostics |
| **Renderer/sim prototyping** | WebGPU-capable GPU (Metal/Vulkan/DX12) | 32GB+ | fast SSD | Interactive splat+mesh visualization; small-batch rollouts |
| **Video/world-model experiments** | High-VRAM CUDA GPU *or* remote service | 32–64GB+ | large SSD | WAN-like predictors, Dream2Flow-style generation/flow caching |

**Comparison with Isaac Sim:**
- Isaac Sim/Lab typically requires an NVIDIA RTX-class GPU and substantial disk footprint; use NVIDIA’s official requirements for your target version.
- This repo’s **implemented** core (PID estimators + geometry gates) runs on CPU and is macOS-friendly; GPU-heavy components (rendering large splat scenes, video predictors) are optional and can be offloaded or run on Linux/CUDA.
- **Footprint note:** treat any “× smaller” comparisons as benchmark-dependent; measure on your deployment.

---

**v6.6 notes (3DGS Integration + Video Model Selection + Tauri/SparkJS/Gazebo Architecture):**

- **Where 3DGS fits in the PID-VLA pipeline — 4 distinct roles:**
  | Role | What It Does | When To Use |
  |------|--------------|-------------|
  | **1. PID Visualization** | Color splats by (Syn, Red, Unq); opacity = MI magnitude | Recommended for debugging/paper figures |
  | **2. Spatial Failure Localization** | Project PID metrics onto 3D scene geometry | When failures have spatial structure |
  | **3. World Model Representation (GWM)** | 3DGS as internal state for Gaussian World Model | Analytical comparison with VLA D |
  | **4. 3D memory (Spatia-like; optional)** | 3D scene memory as context for long-horizon prediction | When spatial consistency across long clips matters (paper-verify) |

- **Candidate video/flow predictors for Dream2Flow-style pipelines (examples; verify):**
  | Approach | Speed | Spatial Consistency | Conditioning | Use (hypothesis; verify) |
  |-------|-------------------|---------------------|-------------|--------------------------|
  | **WAN (arXiv:2503.20314)** | benchmark | unknown; measure | text/image conditioning (paper) | Baseline predictor for offline Flow extraction |
  | **Spatia** (arXiv:2512.15716) | benchmark | high (paper claim) | conditioned on spatial memory | Long-horizon spatial consistency studies |
  | **Motion-controlled predictors** (e.g., Wan‑Move, arXiv:2512.08765) | benchmark | benchmark | motion/trajectory guidance | Counterfactual “what‑if” motion probes (verify applicability) |

- **Acceleration techniques (optional; verify):**
  - Some work reports inference acceleration for diffusion video models (e.g., distillation/consistency, attention optimizations, sparsity). Treat as engineering exploration; benchmark end‑to‑end throughput on your hardware.

- **Spatia (arXiv:2512.15716) — For spatial coherence (paper-verify details):**
  - Described as maintaining a 3D point-cloud “spatial memory” used during generation
  - Described as updating this representation over time (method details depend on the paper/implementation)
  - Described as enabling explicit camera control during generation
  - **Relevance to PID-VLA:**
    - Alternative to WAN for spatially-consistent video generation
    - Natural integration with 3DGS visualization pipeline
    - Camera control enables systematic viewpoint variation for PID robustness testing
  - **Trade-off:** Speed vs spatial consistency is benchmark-dependent; measure end-to-end throughput and quality under your protocol.

- **Video4Spatial — For spatial reasoning evaluation:**
  - Tests visuospatial intelligence in video models
  - Object grounding (focus on specific objects) and scene navigation
  - **Limited relevance:** More useful for evaluating VLA spatial understanding than for generation

- **Recommended video model configuration for PID-VLA:**
  ```
  PRIMARY PIPELINE (Interactive debugging; benchmark-dependent):
  ───────────────────────────────────────────────────
  Gazebo RGB-D → video predictor (+ optional acceleration) → 3D Flow → PID → SparkJS

  SPATIAL CONSISTENCY PIPELINE (Long-horizon, offline analysis):
  ────────────────────────────────────────────────────────────────
  Gazebo RGB-D → 3D-memory predictor (Spatia-like; optional) → 3D Flow → PID → SparkJS

  COUNTERFACTUAL PIPELINE (Action-conditioned "what-if"):
  ────────────────────────────────────────────────────────
  Gazebo RGB-D + A* → motion/trajectory-conditioned predictor (if supported) → 3D Flow → PID comparison
  ```

- **Tauri + SparkJS + Headless Gazebo architecture integration:**
  ```
  ┌─────────────────────────────────────────────────────────────────────────┐
  │                         INTEGRATED ARCHITECTURE                          │
  ├─────────────────────────────────────────────────────────────────────────┤
  │                                                                          │
  │  Headless Gazebo ──Zenoh──→ Tauri Backend ──→ SparkJS Frontend          │
  │                    (Rust)            (WebGPU)                             │
  │                                │                    │                    │
  │                     ┌──────────┴──────────┐        │                    │
  │                     │                     │        │                    │
  │                     ▼                     ▼        ▼                    │
  │         Video/Flow Service*      PID Analysis   3DGS Render             │
  │         ┌──────────────────┐     ┌──────────┐   ┌──────────┐            │
  │         │ WAN-like model    │     │ pid-core │   │ SparkJS  │            │
  │         │ + flow extraction │     │ (Rust)   │   │ (WebGPU) │            │
  │         └──────────────────┘     └──────────┘   └──────────┘            │
  │                     │                     │              │              │
  │                     └──────────┬──────────┘              │              │
  │                                │                         │              │
  │                                ▼                         ▼              │
  │                  3D Flow (aggregated; d≤30)        PID-colored           │
  │                        + PID metrics               3DGS splats           │
  │                                                                          │
  └─────────────────────────────────────────────────────────────────────────┘
  ```

  \* Runs out-of-process (e.g., Python/CUDA). Tauri orchestrates requests, caches artifacts (video/flow), and logs model versions/seeds for reproducibility.

- **3DGS visualization encoding for PID:**
  | Splat Property | PID Mapping | Interpretation |
  |----------------|-------------|----------------|
  | **Color R** | Synergy | Red = high synergy (V,D cooperate) |
  | **Color G** | Redundancy | Green = high redundancy (V,D overlap) |
  | **Color B** | Unique(V) | Blue = vision-only information |
  | **Opacity** | MI magnitude | Transparent = low information |
  | **Size** | Uncertainty (bootstrap σ) | Large = uncertain estimate |

- **Hardware planning note:** Requirements depend on (a) splat scene size, (b) physics/contact workload, and (c) whether external video/world models run locally. Avoid “minimum VRAM” tables; measure peak VRAM/RAM and end‑to‑end latency on your deployment.

- **Implementation priority for Tauri+SparkJS+Gazebo setup:**
  1. **Phase 1:** Headless simulator → IPC → basic visualization + synchronized logging (no 3DGS)
  2. **Phase 2:** Add 3DGS splat rendering + static PID overlays (offline → playback first)
  3. **Phase 3:** Add external “Video Predictor Service*” + flow extraction + stage outcome labels (optional)
  4. **Phase 4:** Add additional world-model baselines (optional) for matched comparisons (no oracle framing)
  5. **Phase 5:** Stream PID overlays from live inference if latency budgets are met; otherwise keep PID computation offline

**v6.5 notes (Hierarchical 3-Way PID Applicability to Dream2Flow — Corrected after first-principles review):**

> **⚠️ Scientific Corrections (v6.5.1):** This section was revised after rigorous first-principles verification. Key corrections: (1) "bridge variable" claim was misleading and removed; (2) 3D flow dimensionality now properly specified; (3) execution-stage PID removed (undefined variable); (4) v5.5 vs curse-of-d distinction clarified.

- **Hierarchical Pairwise PID (§5.3 Option 3) maps to Dream2Flow stages** — but with important caveats:
  - `Syn(V, D_wan; Flow)` — World model quality (video generation stage)
  - `Syn(V, Flow; A)` — Flow-to-action translation (flow extraction + policy stage)
  - ~~`Syn(A_cmd, Sim; A_out)`~~ — **REMOVED: "Sim" was undefined; execution-stage PID requires further specification (robot state? simulation state?)**

- **3D Flow Dimensionality — Precise Definition Required:**
  - **Full representation:** N objects × 3 coordinates × T frames = 3NT dimensions (e.g., 10 objects × 24 frames = 720 dimensions — NOT "6-30")
  - **For PID-tractable representation, must aggregate:** single-object centroid trajectory (d=3T), mean flow vector (d=3), or principal flow statistics (d≤10)
  - **Claim "d≈6-30" is valid ONLY for:** single-object centroid over 2-10 frames, OR aggregated flow statistics
  - **If using full multi-object trajectories:** dimension can be 100s-1000s; v5.6 mitigations still required

- **v5.5 vs Curse of Dimensionality — Separate Issues:**
  | Issue | What It Is | When It Applies | How to Address |
  |-------|-----------|-----------------|----------------|
  | **v5.5 (Geometric)** | Chebyshev volume cancellation requires flat Euclidean space | Curved manifolds (hyperbolic, Lorentz) | Cannot apply L∞ I^sx_∩; use geodesic MI or quantization |
  | **Curse of d (Statistical)** | kNN becomes unreliable as d increases (distance concentration) | High-dimensional Euclidean (d>100-256) | Dimensionality reduction (PCA, SAE, quantization) |
  - **Low-d Euclidean (d<50):** Neither issue applies; I^sx_∩ estimator is valid
  - **High-d Euclidean (d>256):** v5.5 is satisfied (geometry is flat), but curse of d makes kNN unreliable → need reduction
  - **Curved manifold (any d):** v5.5 is violated regardless of dimension → cannot use L∞ I^sx_∩

- **Disjunction neighborhood does NOT "save" high-d sources:**
  - The I^sx_∩ estimator computes: `d_S_disj(i,j) = min(d(S₁), d(S₂))`
  - Counting `n_α(i)` uses the UNION of balls in both source spaces
  - If one source (e.g., V at d=4096) has distance concentration, it affects the union ball count even if the other source (Flow) is well-behaved
  - **Conclusion:** V must be preprocessed (PCA→256) regardless of Flow's dimensionality; Flow doesn't "rescue" V

- **Corrected v5.6 applicability per variable:**
  | Variable | Dimension | v5.5 (Geometry) | Curse of d | Mitigation Required |
  |----------|-----------|-----------------|------------|---------------------|
  | **Aggregated Flow** | d=3-30 | ✓ OK (Euclidean) | ✓ OK | None |
  | **Full Flow (N×T)** | d=100-1000 | ✓ OK (Euclidean) | ✗ Problem | PCA or time-windowing |
  | **A** (actions) | d=7 | ✓ OK | ✓ OK | None |
  | **V** (vision) | d=4096 | ⚠️ Check manifold | ✗ Problem | PCA→256 + geometry check |
  | **D_wan** | d=4096+ | ⚠️ Check manifold | ✗ Problem | Quantization or unrolling |

- **Corrected recommendations for Dream2Flow PID:**
  - `Syn(PCA(V)→256, D_wan_quantized; Flow_agg)`: All variables preprocessed; Flow aggregated to d≤30
  - `Syn(PCA(V)→256, Flow_agg; A)`: V reduced, Flow aggregated, A native — **most tractable**
  - **Execution stage:** Requires defining what "Sim" means before PID can be specified

- **What Experiment 0 must validate for Dream2Flow:**
  1. I^sx_∩ estimator reliability at d=256 (after PCA) with mixed source dimensions
  2. Stability under the specific aggregation method chosen for Flow
  3. Whether joint estimation (one source reduced, one native low-d) introduces systematic bias

- **Latent action diffusion remains complementary:**
  - Operates on A (policy output), not D (world model intermediate)
  - Doesn't solve the diagnostic problem of measuring information in D
  - Both approaches can coexist in same system

**v6.4 notes (VLM→World Model Transition + 3D Flow vs Latent Action Analysis):**
- **New §10.11: The VLM→World Model Paradigm Shift** — Documents the emerging transition in robotics foundation models:
  - **Generation 1 (VLM-based VLAs):** OpenVLA, RT-2, PaLM-E (and smaller open baselines such as SmolVLA) — language-model backbones with largely **implicit** world knowledge in weights; action parameterization varies by implementation (do not assume “action tokens appended to language output” universally).
  - **Generation 2 (World Model-based):** Dream2Flow, DreamVLA, Motus, UniSim — explicit world model with video/flow intermediate representations
  - Key architectural difference: where physics/dynamics knowledge is encoded
- **New lightweight baseline:** SmolVLA (LeRobot) is treated as a low-resource baseline for rapid Experiment 0/1 iteration and pipeline debugging; do not assume its internal variable semantics match larger VLAs (§7.8).
- **New analysis: 3D Object Flow vs Latent Action Space Diffusion** — Addresses the question "why not diffusion on latent actions instead of 3D positional space?":
  - **3D Object Flow (Dream2Flow):** Operates on D (world model); **explicitly Euclidean** (typically \(\mathbb{R}^{3T}\) before aggregation); embodiment-agnostic; can improve estimator tractability when summarized into low‑D object-level features; useful for cross-stage failure attribution
  - **Latent Action Diffusion:** Operates on A (policy head); compact learned representation; embodiment-specific; doesn't solve the D-side estimator validity problem
  - **For PID-VLA specifically:** 3D flow is more useful because the bottleneck is measuring information in high-dimensional D/V embeddings, not action space. 3D flow can be a “geometry escape hatch” (avoid non‑Euclidean metrics) when represented/aggregated in a way that passes the same Experiment 0 + geometry gates.
  - **Both can coexist:** Use latent action diffusion in the VLA being studied, but use 3D flow as an external D validation to diagnose where failures occur
- **Updated Hypothesis H7 context:** The choice of 3D flow over latent actions is deliberate — it enables the decomposition of failures into video→flow→execution stages, which maps directly to PID diagnostic goals

**v6.3 notes (Manifold-Geometry Integration + VLA Compatibility Matrix + Updated Vision Models):**
- **New §10.10.12: Manifold Geometry Considerations** — Connects v5.5/v5.6 manifold challenges to Dream2Flow pipeline:
  - Geometry analysis at each pipeline stage (WAN D_wan vs 3D Flow vs VLA embeddings)
  - **Key insight:** 3D object flow is an explicitly Euclidean target; after aggregation to low‑D features it can reduce reliance on non‑Euclidean latent distances (still validate dimension/concentration)
  - How v5.6 approaches (Isomap, Geodesic MI, PCA, Quantization, Copula) apply to each stage
  - Hyperbolic/Lorentzian connection: "Flow-as-bridge" — use flow targets to reduce dependence on `D_wan` geometry; avoid interpreting continuous PID atoms on non‑Euclidean embeddings without new derivations
  - Hierarchical PID and 3-source scaling with SAE integration (§16.8)
- **New §10.10.13: VLA Integration Matrix** — Per-VLA integration details:
  - v7.0 reframes this section as **contract-first**: define `V/L/D/A` explicitly per checkpoint and avoid assuming internal module names/shapes.
  - “Per‑VLA details” are restricted to abstract-supported facts and explicit verification; everything else is treated as “verify”.
  - The decision matrix is expressed in terms of analysis constraints (what variables are exposed, whether pixel-aligned variables exist, whether explicit `D` is available) rather than assumed architecture internals.
- **Updated vision foundation model placeholders (verify availability/licensing):**
  - **Segmentation:** SAM2 (promptable segmentation) or equivalent; use newer successors if/when released.
  - **Point tracking:** CoTracker (or equivalent point tracker).
  - **Depth:** a monocular depth model such as Depth-Anything v2 (relative depth) and/or a metric-depth baseline (e.g., Metric3D); calibrate if absolute scale is required.
  - Do not assume specific speed/latency improvements; benchmark your pipeline.
- **Cross-references added:** §16 geometry sections now connected to §10.10 pipeline

**v6.2 notes (Unified Architecture: Dream2Flow + WAN + PID + Gaussian Splatting):**
- **New §10.10: Unified Architecture** — Complete integration stack combining:
  - **Dream2Flow** pipeline (video → flow → action) with a WAN-like *local* video model as a replacement candidate for proprietary video APIs (do not assume equivalence; benchmark/validate)
  - **Vision foundation models:** segmentation (e.g., SAM2), point tracking (e.g., CoTracker), and depth estimation (e.g., Depth-Anything v2 / metric-depth baseline) for 3D flow extraction
  - **PID analysis at 4 stages:** World model quality, flow extraction, policy integration, embodiment gap
  - **Gaussian Splatting visualization:** PID-colored 3D splats where RGB = (Syn, Red, Unq)
  - **Tauri + SparkJS** frontend for interactive flow visualization and debugging
- **New §17.17: Dream2Flow Integration Requirements** — Measurement-first compute/data requirements:
  - Record per-clip wall-clock time, peak VRAM/RAM, and artifact sizes for *your* chosen models/hardware
  - Treat per-call API costs (if any) as a variable input; do not hard-code pricing into scientific claims
- **Novel concept:** Gaussian splats as PID visualization — encode synergy/redundancy/uniqueness as splat colors, MI magnitude as opacity, uncertainty as size
- **Optional motion control conditioning:** Wan-Move-style latent trajectory guidance can be used for counterfactual motion control *if* the chosen video model supports it; verify compatibility per paper/release.
- **Research payoff:** Localize VLA failures to specific stages, compare VLA internal D vs WAN-derived flows, visualize information integration in 3D

**v6.1 notes (Dream2Flow Integration + Embodiment-Agnostic Analysis):**
- **Dream2Flow integration (arXiv:2512.24766):** Added Dream2Flow (Dharmarajan et al. 2025) as a related paradigm demonstrating that video generation models encode implicit world knowledge that can be extracted via **3D object flow** as an intermediate representation. Key findings:
  - Video generation models can often synthesize plausible *object motion* even when robot–object interaction details are wrong; treat this as a paper-motivated premise to be validated on the chosen model/task distribution.
  - **Embodiment gap bypass:** Separating "what should move" (object) from "how to move it" (actuator) enables zero-shot transfer
  - The paper reports both simulation and real-world experiments across multiple object categories and embodiments; refer to the paper for specific tasks/robots and measured success rates (do not assume fixed rates in this spec).
- **New §10.9: Dream2Flow and Video-to-Flow Paradigm:** Detailed analysis of how video generation as implicit world model relates to VLA internal world models
- **New Hypothesis H7 (§3.6.6):** 3D object flow as an embodiment-agnostic intermediate representation may correlate with PID synergy patterns when V-D integration is successful
- **New confound §14.5.7:** Embodiment gap confound — PID on (V,D,A) may conflate world model quality with action-execution failures
- **Updated §9.7:** Dream2Flow failure taxonomy as structured framework for PID failure mode analysis
- **Updated §13.4:** Added Dream2Flow citation with project URL

**v6.0 notes (Critical Blockers Analysis + Training/Compute Requirements):**
- **New §17: Training, Compute, and Data Requirements Analysis:** Comprehensive audit of all methods in the document, classifying each by training requirements (none, low, medium, high, extreme), compute needs (inference-only vs training), and data availability. Covers:
  - §17.1: Executive classification table (25+ methods assessed)
  - §17.2-§17.3: Core PID estimators and VLA embedding extraction (no training required)
  - §17.4: VLA fine-tuning costs (LoRA: ~$50-150, Full: ~$50K+)
  - §17.5-§17.6: Dimensionality reduction (PCA vs learned SAE/VAE)
  - §17.7: Neural MI estimators (MINE, CCMI) with per-task training costs
  - §17.8-§17.9: World models and PRMs (extreme compute, infeasible for this project)
  - §17.10-§17.12: Failure classifiers, depth estimation, synthetic data
  - §17.13-§17.16: Data strategy, compute budget recommendations, licensing, and critical challenges
- **New §18: Critical Blockers and Risk Analysis:** Systematic risk assessment with explicit Go/No-Go decision criteria:
  - **5 Show-Stopper Blockers (Category 1):** Experiment 0 failure at d≤256, DreamVLA unavailability, strong dependence (unbounded MI), i.i.d. assumption violation, baselines always winning
  - **7 Major Blockers (Category 2):** Geodesic kNN not implemented, Ollivier-Ricci not implemented, no hyperbolic `I^sx_∩`, Pixel-160K access TBD, GRM weights TBD, no ground truth for "world model quality", scope exceeds PhD timeline
  - **8 Minor Blockers (Category 3):** SAE training, MINE retraining, Isomap cost, macOS CUDA limits, etc.
  - Risk mitigation timeline (Month 1-7 decision gates)
  - Fallback scope hierarchy (Core → Important → Stretch → Future Work)
- **Verification of key blockers (v7.0 audit level = arXiv abstracts + repo reality):**
  - DreamVLA: abstract does not specify backbone dims; weights availability must be checked upstream.
  - OpenVLA: abstract claims open-source checkpoints/code; verify repository/weights availability and license before depending on it.
  - VLA-Arena: verify benchmark availability, data access, and licensing before using it as a primary evaluation source.
  - Geodesic kNN MI and Ollivier-Ricci curvature: not implemented in `pid-core`.
- **Scientific Rigor Notes:** All blockers have explicit detection criteria, mitigation options, and Go/No-Go decision frameworks. Honest assessment: project is HIGH risk but tractable if Experiment 0 succeeds.
- **New §14.6: RoPE What-Where Entanglement Confound:** Based on Gopalakrishnan et al. (2025, arXiv:2509.10534), documents how RoPE-based VLAs (OpenVLA, Llama-based architectures) entangle content and position in embeddings. This creates a confound where PID estimates may reflect positional structure rather than pure semantic integration. Includes 5 mitigation strategies and publication requirements.

**v5.8 notes (VLA-Arena Deep Integration + Memorization/Generalization Analysis):**
- **VLA-Arena as Primary Evaluation Framework:** Deep integration of VLA-Arena (arXiv:2512.22539) as the recommended benchmark for PID-VLA experiments (§9.7.1). VLA-Arena's 170 tasks with structured difficulty axes directly align with PID diagnostic goals.
- **New Testable Hypothesis — Memorization vs Generalization (§3.6):** VLA-Arena's key finding that VLAs exhibit "memorization over generalization" motivates a new, falsifiable hypothesis: PID signatures (specifically, the stability of synergy under input perturbations) may distinguish memorized from generalized task performance. This is treated as a candidate sub-hypothesis requiring empirical validation, not an a priori claim.
- **Perturbation-Based PID Robustness Protocol (§9.7.2):** VLA-Arena's orthogonal V0-V4 (visual) and W0-W4 (language) perturbation axes provide a principled framework for testing whether PID estimates are robust to controlled distribution shifts. Asymmetric robustness patterns (V-perturbations affecting PID differently than L-perturbations) would provide evidence for modality-specific integration failures.
- **Expanded Confound Analysis (§14.5):** Added VLA-Arena-derived confounds including task difficulty stratification (L0/L1/L2), perturbation-induced distribution shift, memorization/generalization confound, modality asymmetry confound, and compositional failure confound. These must be controlled before attributing PID patterns to "integration quality."
- **New §9.7: VLA-Arena Alignment and Experimental Mapping:** Complete subsection mapping VLA-Arena's 4 task dimensions (Safety, Distractor, Extrapolation, Long-Horizon) to specific PID predictions, with falsifiability criteria and expected null results.
- **Long-Horizon and Compositional Failure Analysis (§3.6.3):** VLA-Arena's finding that VLAs cannot compose learned skills for long-horizon tasks suggests that temporal synergy dynamics (synergy half-life, synergy stability across task phases) may be diagnostically valuable. This extends Aim 2 with concrete experimental targets.
- **Safety Dimension Integration (§3.6.4):** VLA-Arena's Safety task axis (collision avoidance, constraint satisfaction) is integrated as a potential PID test case: safety-aware behavior may require specific V-L integration patterns that differ from goal-oriented behavior.
- **Expanded §13.2.1:** VLA-Arena benchmark details including task structure, perturbation protocols, and VLA-Arena-S/M/L dataset specifications.
- **Scientific Rigor Notes:** All VLA-Arena-derived hypotheses are explicitly marked as requiring Experiment 0 validation + controlled experiments. The "memorization over generalization" phenomenon is treated as an empirical observation from VLA-Arena, not as a definitional property of PID.

**v5.7 notes (changes without deleting prior work):**
- **Closed v5.6 as stable.** All v5.6 manifold approaches remain valid; this version adds empirical validation methods and VLA-specific guidance.
- **VLA claim status:** Cross-checked key arXiv abstracts (see §7.6; treat non-abstract specifics as derived/unverified unless cited):
  - OpenVLA (arXiv:2406.09246): abstract states 7B parameters, Llama 2 backbone, and visual features from DINOv2 + SigLIP.
  - DreamVLA (arXiv:2507.04447): abstract describes world-knowledge forecasting and a diffusion-based transformer; backbone dims are unspecified in the abstract.
  - PixelVLA: details remain unverified here; add a primary citation before using numeric specs.
  - TraceVLA (arXiv:2412.10345): abstract states it fine-tunes OpenVLA and mentions a 4B Phi‑3‑Vision compact variant.
- **First-Principles Geometry Analysis (§16.6-§16.11):**
  - §16.6: 4 empirically validated local flatness testing methods
  - §16.7: δ-hyperbolicity testing with Gromov 4-point condition
  - §16.8: SAE analysis for VLA (e.g., VLM SAE work; arXiv:2504.02821)
  - §16.9: Chebyshev/PixelVLA geometry transition analysis
  - §16.10: GPT-2 vs modern LLMs hierarchy evidence
  - §16.11: Unified Geometry-First Protocol + NanoGPT foundational study
- **Authoritative Code Sources:** Added Wibral GitLab repos (infomorphic_networks, continuouspidestimator) to §13
- **New VLA Research Integration:**
  - VLA-Arena benchmark: "memorization over generalization" finding (arXiv:2512.22539)
  - GenieReasoner/FACT tokenizer: flow-matching action discretization (arXiv:2512.24125)
  - Hierarchical geometry of cognitive states in transformer embeddings (arXiv:2512.22227)
- **Hyperbolic Training Guidance:** Added explicit guidance on when/where hyperbolic embedding training is needed (§16.7.4)

**v5.6 notes (changes without deleting prior work):**
- **Added Top 5 Manifold Solutions:** Explicitly listed "Manifold Unrolling", "Geodesic MI", "Linear Projection", "Quantization", and "Copula Transform" as practical engineering paths to address the v5.5 geometry warning.
- **Documented Exclusions:** Explicitly noted why KDE, Harmonic Math, and Naive Geodesic kNN are suboptimal or dangerous in this context.

**v5.6 Architecture notes (superseded by §7.6):**
  - Older “✓ verified” claims should be treated as historical notes unless a primary citation (paper section/code commit/model card) is added.
- **Added §7.6 Architecture Claim Status:** Tracks what is abstract-verified vs derived vs unverified, to avoid overstating certainty.
- **First-Principles Geometry Analysis (Jan 2026):** Major additions to §16:
  - **§16.6 Local Flatness Testing:** 4 empirically validated methods (manifold curvature via subspace angles, Ollivier-Ricci curvature, DLME constraint, curvature-adjusted PCA)
  - **§16.7 δ-Hyperbolicity Testing:** Gromov 4-point condition; literature pointers on tree-likeness diagnostics (replicate on your embeddings; do not transplant values)
  - **§16.8 SAE Analysis for VLA:** Application of Sparse Autoencoders to VLM/VLA components (e.g., arXiv:2504.02821), concrete protocol for PID analysis
  - **§16.9 Chebyshev/PixelVLA Analysis:** Geometry transition analysis showing where L∞ is appropriate vs hierarchical methods
  - **§16.10 GPT-2 vs Modern LLMs:** Architectural differences affecting geometry, layer-wise hierarchy evidence
  - **§16.11 Unified Geometry-First Protocol:** Complete decision framework integrating all diagnostics + NanoGPT foundational study protocol

**v5.5 notes (changes without deleting prior work):**
- **Critical Documentation Fix:** Explicitly documented that Wibral PID (`I^sx_∩`) on manifolds/Lorentz spaces requires new derivations (volume forms/disjunctions).
- Added top-level warning to prevent naive application of Euclidean estimators to curved spaces.

**v5.4 notes (VLA integration):**
- Verified key VLA + Shannon-invariants citations via **arXiv API** (titles/authors/dates):
  - OpenVLA — arXiv:2406.09246
  - DreamVLA — arXiv:2507.04447
  - Dream-VL & Dream-VLA — arXiv:2512.22615
  - PixelVLA — arXiv:2511.01571
  - TraceVLA — arXiv:2412.10345
  - Shannon invariants — arXiv:2504.15779
- Clarified how OpenVLA/DreamVLA/PixelVLA/TraceVLA affect **what variables exist** (what “D” can mean) and therefore which decompositions are scientifically clean (§6.1, §7).
- Clarified the **primary hypothesis** vs. **candidate sub-hypotheses/features** and made the hypothesis↔aims mapping explicit (§1.3, §3.3).
- Tightened the “hierarchy vs geometry” story: Shannon invariants/hierarchical screening address **source-count scaling**, while manifold/high‑d diagnostics address **estimator validity at (N,d)** (§8.1.5, §16).
- Tightened dimensionality-reduction language so the table cannot be misread as “random projection fixes manifolds” or “hyperbolic is drop‑in” (§8.2, §16.4).

**v5.1–v5.3 notes (restored):**
- **v5.3 (Hierarchy vs Geometry):** Distinguished source-count scaling (hierarchy) from estimator validity (geometry).
- **v5.2 (PixelVLA & TraceVLA):** Added citations and scope for visual prompting and trace-based architectures.
- **v5.1 (OpenVLA & DreamVLA):** Clarified variable definitions and world model ("D") extraction.

**v5.0 final audit notes (changes without deleting prior work):**
- Added confounding factors analysis (§14)
- Added numerical stability guidance (§15)
- Added manifold/PCA/kNN limitations section (§16) with detailed diagnostics and decision flowcharts
- Integrated information geometry methods and intrinsic dimension estimation
- Code audit complete — implementation cross-checked against reference implementations
- Grant-ready documentation with full provenance tracking

**Reference verification status (important):**
- Core `I^sx_∩` / KSG papers: verified by DOI metadata; local copies exist under `.external/papers/`.
- arXiv IDs in this document: verified via arXiv API (title/authors/date).
- Detailed architecture claims, runtime/latency numbers, and some “ecosystem” descriptions are treated as **unverified unless explicitly sourced**; keep them as ideas/design sketches, not facts.

---

# Table of Contents

1. [Executive Summary and Critical Warnings](#1-executive-summary-and-critical-warnings)
2. [Theoretical Foundations](#2-theoretical-foundations)
3. [The Core Research Questions](#3-the-core-research-questions)
4. [Decomposition Strategies: What Variables to Analyze](#4-decomposition-strategies-what-variables-to-analyze)
5. [Three-Way PID: I(V, L, D; A)](#5-three-way-pid-iv-l-d-a)
6. [Discarded Approaches and Why](#6-discarded-approaches-and-why)
7. [VLA Architecture Analysis](#7-vla-architecture-analysis)
8. [Estimation and Implementation](#8-estimation-and-implementation)
9. [Experimental Design](#9-experimental-design)
10. [World Model Integration (WAN, GWM, 3DGS)](#10-world-model-integration-wan-gwm-3dgs)
11. [Technical Implementation](#11-technical-implementation)
12. [Open Questions and Future Directions](#12-open-questions-and-future-directions)
13. [References](#13-references)
14. [Confounding Factors Analysis: Proving and Disproving the Hypotheses](#14-confounding-factors-analysis-proving-and-disproving-the-hypotheses)
15. [Numerical Stability and Optimization: Technical Guidance](#15-numerical-stability-and-optimization-technical-guidance)
16. [Why PCA and kNN Are Suboptimal for Manifold-Valued Embeddings](#16-why-pca-and-knn-are-suboptimal-for-manifold-valued-embeddings)
17. [Training, Compute, and Data Requirements Analysis](#17-training-compute-and-data-requirements-analysis)
18. [Critical Blockers and Risk Analysis](#18-critical-blockers-and-risk-analysis)
A. [Appendix A: Glossary](#appendix-a-glossary)
B. [Appendix B: Decision Log and Implementation Reference](#appendix-b-decision-log-and-implementation-reference)
C. [Appendix C: Modern Rendering Stack (SparkJS and WebGPU)](#appendix-c-modern-rendering-stack-sparkjs-and-webgpu)

---

# 1. Executive Summary and Critical Warnings

## 1.1 What This Document Is

This document provides a comprehensive specification for applying Partial Information Decomposition (PID), specifically the shared-exclusions measure I^sx_∩ from the Wibral group at Göttingen, to diagnose grounding failures ("hallucinations") in Vision-Language-Action (VLA) models.

**Scope constraint (PhD-critical):**
- This document is intentionally anchored on the **Wibral/Göttingen line of work**: shared-exclusions PID (`I^sx_∩`, “SxPID”) and the related **Shannon-invariants** program (Gutknecht et al. 2025).
- Other PID measures/tools may be mentioned for contrast or baselines, but **they are not the scientific object of this project**.

**First-principles epistemic split (do not blur):**
- The **quantity** `I^sx_∩` is a mathematical functional of the data-generating distribution.
- Any **estimator** (kNN/KSG, variational, etc.) is a finite-sample algorithm with bias/variance/failure modes. Most “surprising” effects at VLA scale are more likely estimator/pathology than new science unless ruled out by Experiment 0.

**Units (avoid silent mismatches):**
- Papers often use `log2` (bits). This repo’s Rust implementation uses natural `log` (nats).
- Convert via: `bits = nats / ln(2)` and `nats = bits * ln(2)`.

## 1.2 ⚠️ CRITICAL WARNINGS: Read Before Proceeding

This project underwent extensive first-principles review that revealed **fundamental conceptual issues** that must be understood before any implementation:

### Warning 1: The Core Hypothesis May Be Unfounded

**Claim in original proposal:** "Negative synergy (Syn < 0) indicates hallucination because V and D conflict"

**What the mathematics actually says:** Negative synergy under I^sx_∩ means **subadditive information**—combining sources V and D provides less predictive power about target A than expected from their individual contributions. This is NOT the same as "conflict."

Negative synergy could arise from:
- Estimation artifacts at high dimensions (curse of dimensionality)
- High correlation between V and D (double-counting effects)
- General model uncertainty (unrelated to hallucination)
- Pointwise misinformation (observing sources makes target less likely)

**Status:** This is a HYPOTHESIS requiring empirical validation, not a definitional truth. In this spec it is treated as a *candidate sub-hypothesis / feature* inside the primary evaluation aim (see §3.3), not as the project’s sole thesis.

### Warning 2: The V-D-A Decomposition May Be Degenerate

In a VLA model:
```python
action = vla_forward(vision, dream_state, language_instruction)
```

The action A is **deterministically computed** from (V, D, L). This creates problems:

1. **Triviality when conditioning on all inputs:** If `A = f(V,D,L)` deterministically (fixed weights + deterministic inference), then `I(V,D,L;A) = H(A)` (up to inference stochasticity). A 3-source PID of `(V,L,D)→A` decomposes `H(A)` and does not by itself validate grounding/correctness.
2. **Pairwise MI depends on what varies:** `I(V,D;A)` can be informative when `L` varies across samples, but it can approach `H(A)` if `L` is constant (or effectively redundant) in the dataset. Always report the sampling unit and which inputs are included in the decomposition.
3. **Grounding/failure diagnosis needs an external target or counterfactual:** Prefer `A*` (teacher/optimal action), a success/failure label, or controlled interventions/counterfactuals. Otherwise you risk measuring only the model’s internal consistency rather than “hallucination.”

### Warning 3: The KSG Estimator May Fail at VLA Scale

The continuous I^sx_∩ estimator (Ehrlich et al., 2024) was validated on:
- Low-dimensional systems (~100 dimensions)
- Thousands of samples
- Well-behaved distributions

VLA embeddings are:
- 4096+ dimensions
- Hundreds of samples per trajectory
- Unknown distributional properties

At d=4096, k-NN methods suffer from the curse of dimensionality: "nearest neighbors" become nearly equidistant.

### Warning 4: Strong Dependence Can Break kNN MI Even at Low Dimension

There is a separate (often missed) failure mode from “high dimension”: **strong statistical dependence** (very large true MI, e.g., near-deterministic relationships) can make KSG MI estimators require **prohibitively many samples** even when `d` is small.

Gao, Ver Steeg, and Galstyan (AISTATS 2015; arXiv:1411.2003) show that popular KSG MI estimators can have sample complexity that scales **exponentially in the true MI** for strongly dependent variables, due to an implicit local-uniformity assumption. They propose corrections/alternatives (e.g., local non-uniformity correction; local Gaussian approximations, arXiv:1508.00536) that can reduce bias in this regime.

Why this matters here:
- VLA pipelines often contain **near-deterministic mappings** (e.g., `A = f(V,D,L)`; deterministic decoders; cached embeddings; quantized actions).
- If variables are treated as continuous, **MI may be effectively unbounded** in the deterministic/noiseless limit. Estimator output can be dominated by numerical/finite-precision effects rather than meaningful “information integration.”

Design implication:
- Experiment 0 must include **strong-dependence** synthetic cases (not just “high `d`”) and explicitly test estimator stability under near-determinism.
- When using MI/PID on VLA signals, you must be explicit about the noise model / discretization / stochasticity that makes the quantity finite and interpretable.

### Warning 5: kNN Estimators Assume i.i.d. Samples (Trajectory Autocorrelation Is a Confound)

The KSG family (and the Ehrlich et al. `I^sx_∩` estimator built on it) is typically analyzed under an **i.i.d. sample** assumption.

But VLA data is usually collected as **trajectories**:
- Adjacent timesteps are strongly autocorrelated → “N frames” is not “N independent samples”.
- Some variables are constant within a trajectory (e.g., instruction `L`) → within-trajectory MI/PID can be degenerate or misleading.

**Implication:** Treating every frame as an i.i.d. sample can inflate apparent sample size, distort variance estimates, and change neighbor geometry. Any “real-time” claims must state the sampling unit (frames vs windows vs trajectories) and the effective sample size.

**Mitigations (design choices, not afterthoughts):**
- Prefer **across-trajectory** datasets where each sample is an episode/timepoint chosen by a reproducible rule (or use large stride subsampling).
- Use **block bootstrap** / trajectory-level resampling for uncertainty estimates when temporal dependence is unavoidable.

### Warning 6: Liang et al. (2023) Use DIFFERENT PID Measures

Their robotics results do NOT validate I^sx_∩ specifically. They use:
- "Batch estimator" based on variational bounds
- "CVX estimator" using convex optimization over discrete clusters

Neither uses the shared-exclusions definition. Their success doesn't transfer automatically to our approach.

## 1.3 Recommended Approach

Given these warnings, the recommended approach is:

1. **Run Experiment 0 FIRST:** validate the estimator on synthetic data at target dimensionality before any VLA experiments.
2. **Bring up the harness on the simplest stack:** start with simulator-derived `Flow_gt` (from logged object poses) and a small open baseline (e.g., SmolVLA, or even a toy policy) to validate logging, interventions, replay, and embedding extraction *without* adding a video predictor dependency.
3. **Scale to the primary VLA target next (e.g., OpenVLA):** only after (1)–(2) are stable; treat diffusion-based VLAs and predictor-driven Flow as optional branches, not prerequisites.
4. **Include strong baselines:** compare against entropy, OOD scores, PRMs/GRMs, and learned classifiers.
5. **Pre-register success criteria:** specify AUROC/effect-size targets, statistical tests, and which decompositions are “primary”.
6. **Plan for negative results:** if baselines match or beat PID features, that is a valid (publishable) outcome with confound analysis.
7. **Test multiple decompositions:** do not commit to V-D-A alone; test V-L-A and hierarchical pairwise variants, and add 3-way only when pairwise screening indicates value.

**Coherence note (why §1 highlights “one hypothesis” but §3 has multiple aims):**
- The original “Syn < 0 ⇒ hallucination” claim is *not* the project thesis; it is one candidate feature/sub-hypothesis.
- The project thesis is broader: under a **validated estimator regime**, a **feature set** derived from Shannon invariants (Gutknecht et al. 2025) and (where feasible) SxPID (`I^sx_∩`) should add predictive/diagnostic value beyond strong uncertainty baselines.
- §3.3 rewrites the aims to match this hierarchy and makes the gating explicit.

## 1.4 How the Pieces Fit Together (VLA Architecture × Hierarchy × Manifolds)

This project has three *separable* axes that are easy to conflate:

1. **What variables exist (model/architecture):** what “V”, “L”, “D”, and “A” mean depends on the VLA.
   - **DreamVLA (arXiv:2507.04447):** provides explicit world‑knowledge predictions → “D” is operationalizable (and interventionable).
   - **OpenVLA (arXiv:2406.09246):** no explicit “dream/world model” output → any “D” is a hidden-state extraction (definition choice).
   - **PixelVLA (arXiv:2511.01571):** introduces **multiscale V** and **visual prompts** → many candidate “sources” (hierarchy becomes useful).
   - **TraceVLA (arXiv:2412.10345):** injects history via **visual traces** → temporal information is partly “inside V,” blurring V/D boundaries.

2. **How we scale to many sources (hierarchy):** Shannon invariants / hierarchical screening (Gutknecht et al., arXiv:2504.15779) address **combinatorial explosion in source count**, not high‑dimensional geometry.
   - Level 1: MI-only invariants (CI/Ω) to screen many candidate sources/windows.
   - Level 2: targeted pairwise SxPID (`I^sx_∩`) where meaningful.
   - Level 3: optional full 3-way SxPID (18 atoms) offline.

3. **Whether estimation is valid at all (geometry/manifolds):** kNN/KSG and disjunction‑kNN `I^sx_∩` can collapse at high effective dimension or under strong dependence.
   - Always run geometry diagnostics (intrinsic dimension + distance concentration proxies) and the Experiment 0 gate **after** any projection/preprocessing.
   - If kNN-based `I^sx_∩` is invalid even after reduction, restrict claims to Shannon invariants / MI-only baselines and treat them as a different pipeline (not “`I^sx_∩` results”).

---

# 2. Theoretical Foundations

## 2.1 Partial Information Decomposition (PID)

PID addresses the question: Given two (or more) source variables S₁, S₂ and a target variable T, how can we decompose the total mutual information I(S₁, S₂; T) into components that capture:

- **Redundancy:** Information available from EITHER source alone
- **Unique Information:** Information available from ONE source but not the other
- **Synergy:** Information available ONLY from both sources together

For two sources, the decomposition is:
```
I(S₁, S₂; T) = Red(S₁, S₂; T) + Unq(S₁; T) + Unq(S₂; T) + Syn(S₁, S₂; T)
```

### 2.1.1 The Problem: PID is Underdetermined

Shannon's information theory doesn't uniquely specify how to compute these atoms. Multiple PID measures exist, each with different properties and trade-offs.

## 2.2 The I^sx_∩ (Shared-Exclusions) Measure

We adopt I^sx_∩ from Makkeh, Gutknecht, and Wibral (2021), extended to continuous variables by Ehrlich et al. (2024).

### 2.2.1 Definition

The shared-exclusions redundancy `I^sx_∩` is defined via **exclusions of probability mass** and can be written as a **local mutual information** induced by an auxiliary “statement” variable `W` (Makkeh et al. 2021, Eq. 17):
```
i^sx_∩(t : s₁; s₂) := i(t : W_{s₁,s₂}=1) = log[ p(t | W_{s₁,s₂}=1) / p(t) ]

I^sx_∩(S₁, S₂; T) := E_{t,s₁,s₂}[ i^sx_∩(t : s₁; s₂) ]
```

Where `i(·;·)` is the **pointwise mutual information**:
```
i(s; t) = log[p(s, t) / (p(s)·p(t))]
```

**Subtle but important (do not gloss over):**
- The *local* term `i(t : W_{s₁,s₂}=1)` is a pointwise mutual information with an auxiliary statement variable.
- The *global* redundancy `I^sx_∩(S₁,S₂;T)` is **not** the mutual information `I(T;W)` because the expectation is taken over `p(t,s₁,s₂)` (Makkeh et al. 2021 note under Eq. 17), not over `p(t,W)`.
- Consequence: even the **redundancy itself** can be negative at the distribution level; negative values are not automatically “bugs,” but they do require careful interpretation and estimator validation.

Ehrlich et al. (2024) derive a **kNN/KSG-style estimator** for the continuous case by replacing conjunction (intersection) neighborhoods with disjunction (union) neighborhoods; see §8.1.3 for the concrete estimator form.

**Important:** Do **not** confuse `I^sx_∩` with Williams & Beer’s `I_min`, which is defined using a minimum over “specific information” terms; `I_min` is a different redundancy measure.

### 2.2.2 Key Properties

| Property | I^sx_∩ | Implication |
|----------|--------|-------------|
| **Differentiability (distribution-level)** | ✓ | Differentiable as a functional of probabilities; **gradient-based training still requires a differentiable estimator** (the kNN/KSG estimator is not). |
| **Target Chain Rule** | ✓ | Atoms sum to total MI |
| **Atom non-negativity** | ✗ | Some atoms can be negative (including `I^sx_∩` itself; not just synergy/unique) |
| **Transformation Invariance** | ✗ | Traded for TCR |

### 2.2.3 Why Negative Synergy is Possible

The impossibility results from Matthias et al. (2025, arXiv:2512.16662) prove that:

> Non-negativity + Target Chain Rule + Transformation Invariance are **mutually incompatible**

I^sx_∩ satisfies the Target Chain Rule by sacrificing all-atom non-negativity. This means synergy CAN be negative. Whether negative-synergy regimes correlate with grounding failures is an empirical question that must be tested under controlled validation and strong baselines.

### 2.2.4 What Negative Synergy Actually Means (Mathematically)

When Syn < 0:
```
Syn = I(S₁, S₂; T) - Red - Unq₁ - Unq₂ < 0
```

This means: Red + Unq₁ + Unq₂ > I(S₁, S₂; T)

**Interpretation options:**
1. **Redundancy-leaning allocation under `I^sx_∩`:** relative to other PID measures, `I^sx_∩` can allocate more (or even negative) redundancy to satisfy its axioms; the synergy term adjusts accordingly via the PID identities.
2. **Subadditivity in the chosen decomposition:** combining sources yields less *net* information about `T` than suggested by their individual terms once redundancy is accounted for (a statement about the decomposition, not about “conflict”).
3. **Pointwise misinformation:** at specific points, observing `(s₁,s₂)` can make `t` less likely than marginally expected (negative local information), which can propagate into negative PID atoms depending on the measure.
4. **Estimator/pathology:** high dimension, strong dependence, ties/quantization, and trajectory autocorrelation can all produce artifactual negative atoms; treat “unexpected signs” as a prompt to run controls, not as a conclusion.

**NOT a valid interpretation:** "The sources are fighting each other" or "conflict" in any intuitive sense. This is a seductive but potentially misleading metaphor.

## 2.3 Continuous Variable Extension

Ehrlich et al. (2024) extended I^sx_∩ to continuous variables using k-nearest neighbor (k-NN) estimation, building on the KSG estimator (Kraskov et al., 2004).

### 2.3.1 KSG Estimator

The KSG formula for mutual information:
```
I(X; Y) = ψ(k) + ψ(N) - ⟨ψ(n_x + 1) + ψ(n_y + 1)⟩
```

Where:
- ψ is the digamma function
- k is the number of neighbors
- N is sample size
- n_x, n_y are marginal counts within the k-th neighbor distance
- **Maximum norm (Chebyshev distance)** is used for BOTH k-NN search AND marginal counting

### 2.3.2 Extension to I^sx_∩

Ehrlich et al. (2024) derive a **KSG-style kNN estimator** for continuous `I^sx_∩`. It is **not** “take the minimum of pointwise MI terms.”

The key adaptation is that the shared-exclusions “OR” in Makkeh et al. (2021) becomes a **disjunction neighborhood** in source space. Under Chebyshev/L∞:

- `d_S_disj(i,j) = min( d(S₁ᵢ,S₁ⱼ), d(S₂ᵢ,S₂ⱼ) )`
- `d_ST_disj(i,j) = max( d(Tᵢ,Tⱼ), d_S_disj(i,j) )`

For each sample `i`, let `εᵢ` be the distance to the `k`-th nearest neighbor under `d_ST_disj`. Count `n_α(i)` neighbors in the source-disjunction ball and `n_T(i)` neighbors in target space within `εᵢ`, then estimate:

```
Î^sx_∩ = ψ(k) + ψ(N) − (1/N) Σ_i [ ψ(n_α(i)) + ψ(n_T(i)) ]
```

**Counting convention (make this explicit; it affects off-by-one bugs):**
- In many KSG-style presentations, neighbor counts exclude the sample itself and the formula uses `ψ(n_x(i)+1)` / `ψ(n_y(i)+1)`.
- In other (equivalent) presentations, counts **include** the sample itself and the `+1` is absorbed into the count.

This document (and `crates/pid-core`) uses the **include-self** convention for `n_α(i)` and `n_T(i)` so the digamma arguments are the inclusive counts.

See §8.1.3 for concrete implementation notes (tie handling and “strict radius” rules matter).

## 2.4 Infomorphic Networks (Optional / Exploratory)

Makkeh et al. (2025, PNAS; DOI `10.1073/pnas.2408125122`) describe “infomorphic networks”: using local information-theoretic terms as **learning objectives** rather than post-hoc analysis tools. This is *conceptually adjacent* but **not required** for Aim 1 (implementing/validating `I^sx_∩` as a diagnostic).

In infomorphic networks, neurons optimize:
```
L_local = α·Redundancy + β·Unique + γ·Synergy
```

This motivates Aim 3 as an exploratory direction *only after* Experiment 0 and the diagnostic experiments succeed, and only with a differentiable estimator (the kNN/KSG estimators in this spec are not differentiable).

## 2.5 Shannon Invariants as Problem-Solvers

Gutknecht et al. (2025, arXiv:2504.15779) introduced Shannon invariants to address PID's scalability limitations. Here we explore how they can solve specific problems in our VLA application.

### 2.5.1 The Scalability Problem

- Full PID grows super-exponentially with the number of variables
- For 2 sources: 4 atoms
- For 3 sources: 18 atoms
- For 4 sources: 166 atoms

For three variables (V, L, D), estimating all 18 atoms is computationally expensive and many atoms are hard to interpret.

### 2.5.2 What Makes an Invariant "Shannon"?

**Key Insight from Gutknecht et al. (2025):** A "Shannon invariant" is a quantity that:
1. Captures meaningful properties of information decomposition
2. **Depends only on Shannon's entropy definition** (not on which PID measure you choose)
3. Can be computed efficiently from standard MI estimates

**Why This Matters:** Different PID measures (I^sx_∩, I_min, I_BROJA, etc.) give different values for redundancy and synergy. But Shannon invariants have the **same value regardless of which PID measure you use**. This makes them theoretically robust and practically useful.

**Units note (bits vs nats):**
- Gutknecht et al. (arXiv:2504.15779) primarily report in **bits** (`log2`).
- This repo’s Rust estimators report in **nats** (`ln`).
- Changing log base multiplies all MI/entropy/PID quantities by a constant: `bits = nats / ln(2)`.
  - Signs (e.g., `CI < 0`) and rank-order comparisons are unchanged.
  - Any numeric thresholds (e.g., “MI > 4 nats”) must be converted when comparing across papers.

**The Key Example - Co-Information:**

For any bivariate PID measure, the following identity holds:

```
CI(X₁, X₂; Y) = I(X₁;Y) + I(X₂;Y) - I(X₁,X₂;Y) = Red - Syn
```

This equals Redundancy minus Synergy for **any** valid PID measure. The individual values of Red and Syn depend on your measure choice, but their difference is invariant.

### 2.5.3 Shannon Invariants: Scalar Summaries

Instead of computing all PID atoms, Shannon invariants provide interpretable numbers:

**Co-Information (Interaction Information) for 3 Variables:**
```
CI(V, L, D; A) = I(V;A) + I(L;A) + I(D;A) 
              - I(V,L;A) - I(V,D;A) - I(L,D;A) 
              + I(V,L,D;A)
```

**Interpretation:**
This is the natural higher-order extension of the pairwise “interaction information” with a distinguished target:

```
CI_m(X₁,…,X_m; Y) := Σ_{∅≠S⊆{1..m}} (-1)^{|S|+1} I(X_S; Y)
```

For `m=2`, this reduces to `CI_2(X₁,X₂;Y)=I(X₁;Y)+I(X₂;Y)-I(X₁,X₂;Y)=Red−Syn` (a Shannon invariant for any bivariate PID).

**Sign convention warning:** literature flips signs and names (co-information vs interaction information) depending on author. In this document, **negative CI is treated as “synergy-dominant”** (i.e., `Syn > Red` for the corresponding bivariate PID), and **positive CI** as “redundancy-dominant”.

**Important:** `CI_m` is a Shannon-invariant summary computed from MI terms. It is **not** a PID and it conflates multiple PID atoms for `m≥3`. Use it as a screening statistic, not as a substitute for `I^sx_∩`.

**O-Information (for n > 3 variables):**

O-information (Rosas et al., 2019) is a **synergy-vs-redundancy bias** scalar defined for a *set of variables* (no distinguished target). A standard entropy-form definition is:

```
Ω(X₁,…,Xₙ) = (n-2)·H(X₁,…,Xₙ) + Σᵢ H(Xᵢ) − Σᵢ H(X_{-i})
```

where `X_{-i}` denotes the collection of all variables except `Xᵢ`. Equivalently, `Ω = TC − DTC` (total correlation minus dual total correlation).

For `n=3`, `Ω(X,Y,Z)` equals the (3-variable) co-information / interaction information (up to sign conventions).

**How to use it here (and where it does *not* help):**
- It can summarize whether a *chosen small set* (e.g., `{V,L,D,A}` or `{V,L,D}`) is globally synergy-leaning (`Ω<0`) vs redundancy-leaning (`Ω>0`).
- It is **not automatically scalable** to “hundreds of attention heads” in the raw sense: estimating the required high-order entropies/CMIs in high-dimensional continuous spaces can be harder than PID itself unless you introduce strong structure (coarse-graining, factorization, parametric assumptions, or dedicated estimators).
- Treat `Ω` as a screening/description statistic that may motivate where to apply the hierarchical SxPID pipeline (Level 1 CI → Level 2 targeted `I^sx_∩`).

**Application to VLA:** `Ω` can be useful once you have **a small, well-defined set of variables** (or a **coarse-grained** representation of many units). It is not a “free lunch” for hundreds of raw attention heads unless you add structure (clustering/SAE, factor models, or other dimensionality reduction) and re-validate estimator behavior.

### 2.5.4 How Shannon Invariants Solve Our Problems

#### Problem 1: Combinatorial Explosion in 3-Way PID

**Problem:** Computing I(V, L, D; A) decomposition requires 18 atoms.

**Shannon Invariant Solution:** Co-information gives a single summary statistic that captures whether the overall interaction is synergistic or redundant, without needing all 18 atoms.

```python
def co_information(V, L, D, A, k=5):
    """Efficient 3-way summary using only pairwise MI estimates."""
    I_V_A = ksg_mi(V, A, k)
    I_L_A = ksg_mi(L, A, k)
    I_D_A = ksg_mi(D, A, k)
    I_VL_A = ksg_mi(np.hstack([V, L]), A, k)
    I_VD_A = ksg_mi(np.hstack([V, D]), A, k)
    I_LD_A = ksg_mi(np.hstack([L, D]), A, k)
    I_VLD_A = ksg_mi(np.hstack([V, L, D]), A, k)
    
    return I_V_A + I_L_A + I_D_A - I_VL_A - I_VD_A - I_LD_A + I_VLD_A
```

**Cost:** 7 MI estimates instead of 18 PID atoms.

#### Problem 2: Which Decomposition to Use (V-D-A vs V-L-A)?

**Problem:** We don't know a priori whether V-D-A or V-L-A is more informative.

**Shannon Invariant Solution:** Compare co-information across decompositions:

```python
CI_VD = co_information_2way(V, D, A)  # Standard: I(V;A) + I(D;A) - I(V,D;A)
CI_VL = co_information_2way(V, L, A)
CI_LD = co_information_2way(L, D, A)
```

**Interpretation (cautious):**
- CI is a Shannon-invariant summary: for any bivariate PID, `CI = Red − Syn`.
- If CI_VL is strongly negative relative to CI_VD, the V–L pair is more “synergy-dominant” than V–D (a candidate to prioritize in deeper analysis).
- If CI_VD is strongly negative relative to CI_VL, the V–D pair is more “synergy-dominant” than V–L (a candidate to prioritize in deeper analysis).
- If all are similar, either (a) the system is genuinely symmetric, or (b) the estimator regime is too noisy to differentiate pairs.

#### Problem 3: Localizing Failure Mode Without Full 3-Way PID

**Problem:** We want to know WHERE the failure is (V-L? V-D? L-D?) without computing all 18 atoms.

**Shannon Invariant Solution:** Hierarchical pairwise co-information pattern:

```python
# Compute pairwise co-information (simpler than full PID)
CI_VL = I(V;A) + I(L;A) - I(V,L;A)  # Negative = synergistic
CI_VD = I(V;A) + I(D;A) - I(V,D;A)
CI_LD = I(L;A) + I(D;A) - I(L,D;A)
```

**Heuristic interpretation (requires validation):**

| CI_VL | CI_VD | CI_LD | Interpretation (heuristic) |
|-------|-------|-------|----------------|
| < 0 | < 0 | < 0 | All pairs synergy-dominant (Syn > Red for each pair) |
| > 0 | < 0 | < 0 | V–L redundancy-dominant (Red > Syn) while others are synergy-dominant |
| < 0 | > 0 | < 0 | V–D redundancy-dominant while others are synergy-dominant |
| < 0 | < 0 | > 0 | L–D redundancy-dominant while others are synergy-dominant |
| > 0 | > 0 | < 0 | Both V–L and V–D are redundancy-dominant relative to L–D |
| > 0 | < 0 | > 0 | V–L and L–D are redundancy-dominant relative to V–D |
| < 0 | > 0 | > 0 | V–D and L–D are redundancy-dominant relative to V–L |
| > 0 | > 0 | > 0 | All pairs redundancy-dominant (Red > Syn for each pair) |

**Note:** This uses classical interaction information (MI-based), not I^sx_∩. While less fine-grained, it's much cheaper to compute.

#### Problem 4: Real-Time Monitoring

**Problem:** Full PID is too slow for real-time intervention (~seconds per sample).

**Shannon Invariant Solution:** Pre-compute PID on training data to learn a mapping from CI to failure probability:

```python
# Offline: Learn relationship
training_data = [(CI_VL, CI_VD, CI_LD, failure_label) for trajectory in training_set]
ci_to_failure_model = train_classifier(training_data)

# Online: Fast inference using only CI (7 MI estimates)
ci_vec = [co_info_2way(V, L, A), co_info_2way(V, D, A), co_info_2way(L, D, A)]
failure_prob = ci_to_failure_model.predict(ci_vec)
```

**Speed improvement:** CI requires only MI terms (7 KSG runs for a triplet); full `I^sx_∩` PID adds an additional disjunction-kNN redundancy estimator with per-sample kNN radii + neighbor counts, which is typically more expensive.

### 2.5.4 Recommended Strategy: Hierarchical Approach

**Level 1 (Fast screening; MI-only):** Compute CI_VL, CI_VD, CI_LD using KSG. Use for triage / monitoring.

**Level 2 (Targeted, slower):** Compute full pairwise `I^sx_∩` PID for the most suspicious pair (identified by Level 1).

**Level 3 (Slow, offline):** Compute full 3-way decomposition or run detailed analysis for failure diagnosis.

**Latency note (do not oversell):** Wall-clock time depends strongly on `(N, d, k)` and on the kNN backend. A brute-force O(N²) implementation is not “real-time” at N in the thousands; any ms-level budgets are design targets that presume aggressive dimensionality reduction and/or accelerated kNN.

This hierarchical approach balances speed with interpretability, using Shannon invariants as a fast screening layer.

---

# 3. The Core Research Questions

## 3.1 Primary Question

**Do shared-exclusions PID (SxPID / `I^sx_∩`) features provide statistically reliable signal for VLA failure detection and diagnosis beyond strong uncertainty baselines?**

This is the critical validation gate. The “synergy sign” is one candidate feature, but we should treat it as a hypothesis rather than a privileged statistic. If SxPID features do not significantly outperform baselines under a validated estimator regime, we report negative results.

**Diagnostic-first stance (recommended):**
- It is scientifically reasonable to treat PID primarily as a **diagnostic / interpretability** tool (explaining *which sources matter and how*), even if it does not beat entropy/PRMs on AUROC.
- It is *not* scientifically justified to reduce PID to “synergy only” a priori. In practice, the most reliable signal may come from a **feature set** that includes MI terms, CI, redundancy, uniques, synergy, and derived summaries—then letting Experiments 1–2 determine what actually predicts failures.

## 3.2 Secondary Questions

1. **Which decomposition is most predictive?** V-D-A? V-L-A? Three-way? Something else?
2. **At what dimensionality does the estimator work?** Raw 4096-dim? PCA to 256? Learned projections?
3. **Is synergy causal or merely correlated with failure?** Can interventions on D cause synergy changes?
4. **Does synergy dynamics predict success?** Does synergy half-life correlate with task completion?

## 3.3 Specific Aims

**Non-negotiable gate:** All aims below assume Experiment 0 establishes a validated estimator regime (possibly only **after** explicit dimensionality reduction). If Experiment 0 is **NO-GO** even after reduction (e.g., at \(d \approx 256\)), we do not claim results about kNN-based `I^sx_∩` on VLA embeddings; we pivot to Shannon-invariant screening and non-`I^sx_∩` baselines.

### Aim 1 (Primary): Comparative Evaluation (Experiments 1–2)

**Primary hypothesis (falsifiable; pre-register):** Under a validated estimator regime, a feature set derived from Shannon invariants (CI/Ω) and (where feasible) SxPID atoms from plausible decompositions (e.g., V–D–A, V–L–A, hierarchical pairwise) contains predictive information about failure labels beyond the best baseline.

**Candidate sub-hypothesis (not privileged):** The “synergy sign / frequency of negative-synergy windows” contributes additional signal beyond MI/entropy alone; it may also fail entirely (estimator/pathology or irrelevance).

**Baselines:**
1. Predictive entropy: H(A|V, L)
2. Semantic entropy (VL-Uncertainty)
3. Snapshot ensemble variance
4. Cross-modal attention entropy
5. Learned failure classifier
6. Liang et al. Batch/CVX estimators (different PID family; baseline only)
7. **Process Reward Model (GRM):** Progress-based failure detection (Robo-Dopamine)

**Success criteria:** statistically significant improvement over best baseline (paired bootstrap or matched test; p < 0.05) with a practically meaningful effect size (pre-registered), OR a well-supported negative result (no improvement) with analysis of failure causes (estimator regime, confounds, variable choice).

### Aim 2: Regime Mapping for High‑d / Manifold‑Valued Embeddings (Experiment 3 + Exp0 subsets)

**Question:** At what effective dimensionality and preprocessing does the estimator become stable enough to support Aim 1?

**Deliverable:** a regime map and a single recommended measurement pipeline (e.g., Raw vs PCA95 vs random/Hash projection), with geometry diagnostics (intrinsic dimension, distance concentration proxies) recorded at each stage and explicit GO/PIVOT/NO-GO outcomes.

### Aim 3: Causal Validation for Diagnosis (Experiment 4)

**Question:** Are decomposition signatures merely correlational, or do controlled interventions / counterfactual targets (`A*`) produce predictable, reproducible changes?

**Design note:** causal claims require interventions (on `D`/`V`/`L`) or external targets; otherwise VLA self-consistency can masquerade as “information integration.”

### Optional extensions (only if Aim 1 succeeds AND Aim 2 yields a stable regime)

- **Synergy dynamics:** test whether time-resolved summaries (e.g., a “synergy half-life” under explicit windowing/stride + dependence-aware uncertainty) add signal beyond static features.
- **RL fine-tuning (exploratory):** kNN/KSG estimators are not differentiable and are unlikely to be safe as direct rewards. If pursued, follow Wibral-group “infomorphic networks” framing (Makkeh et al. 2025) or train an offline differentiable surrogate to predict SxPID-derived features; treat generic PRM/SRL methods as baseline controls.

## 3.4 Where PID Provides Unique Value (Six Use Cases)

**Important framing:** PID may not outperform entropy for pure AUROC on failure detection. Its unique value lies in **interpretable decomposition**. Here are six specific use cases where decomposition provides value that entropy cannot:

### Use Case 1: Failure Mode Diagnosis (Post-Hoc)

**Scenario:** Robot fails a task. We want to know WHY.

**What entropy tells us:** "The model was uncertain."

**What PID decomposition may suggest (hypotheses that require validation):**

| Pattern (estimated) | Hypothesis | Next Check |
|---------|-----------|-------------------|
| High Unq(V), low Unq(D) | Policy relies mostly on V for T | Intervention on D should have limited effect |
| Low Unq(V), high Unq(D) | Policy relies mostly on D for T | Intervention on V (occlusion/corruption) should strongly affect |
| High Syn (positive) | Joint V–D interaction contributes | Check whether synergy tracks task phase or known fusion modules |
| Low/negative Syn | Subadditivity / possible integration anomaly | Distinguish (a) estimator pathology, (b) redundancy inflation, (c) true “misinformation” via controls |
| High Unq(L) (V–L–A) | Language heavily determines T | Check instruction perturbations / paraphrases |

### Use Case 2: Architecture Design Feedback

**Scenario:** Comparing two VLA architectures on the same task.

**What entropy tells us:** "Architecture A is more certain than B."

**What PID decomposition tells us:**
- "Architecture A shows higher Syn, suggesting better multimodal fusion"
- "Architecture B shows higher Unq(V), suggesting it relies more on vision"
- "Architecture A shows higher Red, suggesting V and D encode similar information (potential redundancy)"

This informs which architectural choices improve integration vs. reliance on individual modalities.

### Use Case 3: Training Curriculum Design

**Scenario:** Designing a training curriculum for VLA fine-tuning.

**What entropy tells us:** "Train on examples where the model is uncertain."

**What PID decomposition tells us:**
- *(Hypotheses; require a validated estimator regime + controls for confounds like task difficulty and distribution shift.)*
- "Model shows persistently low/unstable Syn on manipulation tasks → candidate integration weakness → prioritize targeted manipulation data"
- "Model shows high Unq(D) and low Unq(V) (relative to validated baselines) → candidate over-reliance on internal state → add visual grounding data"
- "Model shows atypical V–L interaction signatures (e.g., CI_VL strongly negative or `Syn_VL` consistently extreme) → candidate language–vision alignment issue → add alignment data"

**Proposed curriculum objective:**
```python
curriculum_priority = α*|Syn| + β*imbalance(Unq_V, Unq_D) + γ*task_importance
```

### Use Case 4: Targeted Data Collection

**Scenario:** Limited budget for collecting new robot demonstrations.

**What entropy tells us:** "Collect data for high-uncertainty scenarios."

**What PID decomposition tells us:**
- *(Hypotheses; PID atoms can be negative under `I^sx_∩` and can be estimator-sensitive at high `d`.)*
- "Model needs visual grounding data (e.g., Unq(V) systematically low relative to Unq(D) under validated preprocessing)"
- "Model needs language–vision alignment data (e.g., V–L pair shows abnormal CI/PID signatures compared to controls)"
- "Model needs action diversity / disambiguation (e.g., redundancy-dominant signatures across task variants; verify with controlled task splits)"

This enables **targeted** data collection rather than blanket uncertainty-based collection.

### Use Case 5: Real-Time Intervention Selection

**Scenario:** Robot is about to fail. What help should we provide?

**What entropy tells us:** "Robot is uncertain → request help."

**What PID decomposition tells us:**
- *(Heuristic; only meaningful if the estimator regime is validated and the mapping is learned/calibrated on held-out data.)*
- High Unq(D), low Unq(V) → "Show me what you see" (visual confirmation)
- High Unq(V), low Unq(D) → "What do you expect to happen?" (prediction query)  
- V–L signatures suggest mismatch → "Did you understand the instruction?" (language clarification)

**Note:** This requires diagnostics fast enough for live intervention, which is currently challenging for full SxPID on high‑d signals. A practical approach is to compute PID offline and deploy only lightweight screening (CI/Ω) or a learned classifier online.

### Use Case 6: Interpretability for Safety Certification

**Scenario:** Certifying a VLA for deployment in safety-critical settings.

**What entropy tells us:** "Model uncertainty stays below threshold X."

**What PID decomposition tells us:**
- "Model avoids extreme negative-atom regimes outside those seen in validated controls" (a stability check, not a guarantee of “coherence”)
- "Model shows stable information signatures across task variations under fixed preprocessing" (robustness evidence if replicated)
- "Failure modes are *more traceable* to specific information sources" (interpretability hypothesis; validate against intervention tests)

This provides an **audit trail** for safety certification that entropy alone cannot provide.

### Summary: When to Use PID vs. Entropy

| Goal | Use Entropy | Use PID |
|------|-------------|---------|
| Simple failure detection | ✓ (faster, comparable AUROC) | |
| Understanding WHY failure occurred | | ✓ (decomposition) |
| Comparing architectures | | ✓ (multimodal integration metrics) |
| Training curriculum design | | ✓ (targeted improvement) |
| Data collection prioritization | | ✓ (specific capability gaps) |
| Safety certification | | ✓ (audit trail) |

**Positioning Statement:** PID is complementary to entropy, not a replacement. Use entropy for speed and simplicity; use PID for interpretability and actionable insights.

## 3.5 PID vs. Process Reward Models (PRMs)

### 3.5.1 What Are Process Reward Models?

Process Reward Models (PRMs) are vision-language models trained to predict task progress from visual observations. Unlike outcome reward models (ORMs) that only provide sparse binary success/failure signals, PRMs provide dense, step-by-step progress estimates.

**Recent Example: Robo-Dopamine (arXiv:2512.23703)**

Robo-Dopamine introduces a General Reward Model (GRM) trained on 35M samples from 3,400+ hours of video:
- **Step-wise Reward Discretization:** Hop-based relative progress labels
- **Multi-Perspective Progress Fusion:** Combines incremental, forward-anchored, and backward-anchored predictions
- **Policy-Invariant Reward Shaping:** Avoids "semantic trap" where agent stagnates in high-progress states
- **Results:** 92.8% progress accuracy, 0.953 VOC score, policy improves from ~0% to 95% in 150 rollouts *(paper-reported; verify evaluation protocol if used for quantitative comparisons).*

### 3.5.2 Comparison: PID vs. PRM

| Aspect | PID (I^sx_∩) | PRM (e.g., GRM) |
|--------|--------------|-----------------|
| **What it measures** | Information structure between V, D, A | Task progress toward goal |
| **Output** | Syn, Red, Unq decomposition | Progress estimate Φ ∈ [0,1] |
| **Interpretability** | WHY failure occurred | HOW FAR along task |
| **Computational cost** | O(n² × d) per pair | O(1) forward pass |
| **Training required** | None (estimator-based) | 35M+ samples, 3400+ hours |
| **Multi-view support** | Implicit in embeddings | Explicit (GRM uses multi-view fusion) |
| **Real-time feasible** | Shannon invariants only (fastest; depends on n,d and kNN backend) | Yes (single forward pass; hardware-dependent) |

### 3.5.3 When to Use Each

| Scenario | Use PID | Use PRM |
|----------|---------|---------|
| Diagnosing WHY failure occurred | ✓ | |
| Dense reward for RL fine-tuning | | ✓ |
| Comparing multimodal fusion quality | ✓ | |
| One-shot task adaptation | | ✓ (GRM adapts from 1 demo) |
| Architecture design feedback | ✓ | |
| Policy learning efficiency | | ✓ |
| Understanding V-D integration | ✓ | |
| Progress monitoring in deployment | | ✓ |

### 3.5.4 Potential Synergies

PID and PRMs can be complementary:

1. **PRM-guided PID sampling:** Use GRM progress estimates to identify critical transitions, then apply PID for detailed diagnosis
2. **PID-augmented rewards:** Add PID-based synergy term to PRM rewards for multimodal coherence:
   ```
   r_combined = r_GRM + α·Syn(V,D;A)
   ```
3. **OOD detection fusion:** GRM's consistency checking (forward vs backward anchored disagreement) + PID's synergy sign could provide robust failure detection
4. **Multi-Perspective Fusion analogy:** GRM's fusion of {incremental, forward-anchored, backward-anchored} predictions mirrors how we might fuse {Syn_VD, Syn_VL, Syn_LD} in hierarchical PID

### 3.5.5 Key Insight from Robo-Dopamine: The Semantic Trap

Robo-Dopamine identifies a critical failure mode in naive reward shaping:

**Problem:** Using r(s,a,s') = Φ(s') - Φ(s) as dense reward creates perverse incentives. The agent learns to reach high-progress states and stagnate rather than complete tasks.

**Their Solution:** Policy-Invariant Reward Shaping:
```
r_GRM = r_gold + γΦ(s') - Φ(s)
```
where r_gold = 1 at task completion. This telescopes to a boundary term, preserving optimal policy.

**Relevance to PID:** A similar trap could occur if using Syn as intrinsic reward. Our proposed:
```
r_intrinsic = α·Syn(V,D;A) - γ·max(0, -Syn)
```
should be analyzed for policy invariance properties.

## 3.6 Memorization vs Generalization: A VLA-Arena-Derived Hypothesis (v5.8)

VLA-Arena (arXiv:2512.22539) identifies a critical limitation of current VLAs: **"a strong tendency toward memorization over generalization."** This finding has direct implications for PID-based diagnostics and motivates new testable hypotheses.

### 3.6.1 The Phenomenon (Empirical Observation, Not PID Theory)

VLA-Arena's structured difficulty levels (L0→L1→L2) reveal that VLAs:
- Perform well on L0 tasks (training distribution)
- Degrade significantly on L1/L2 tasks (distribution shifts requiring generalization)
- Show **asymmetric robustness**: visual perturbations (V0→V4) and language perturbations (W0→W4) affect performance differently

**Critical epistemic note:** This is an empirical observation about current VLA architectures, not a theorem about information theory. The following hypotheses translate this observation into PID-testable predictions, but they require validation and may be false.

### 3.6.2 Hypothesis H4: PID Signatures Differ Between Memorization and Generalization Regimes

**Claim (falsifiable):** When a VLA memorizes a task, PID signatures on training-distribution inputs will differ systematically from PID signatures on OOD/generalization-requiring inputs.

**Theoretical motivation (speculative, requiring validation):**

| Regime | Information-Theoretic Characterization | Expected PID Pattern |
|--------|---------------------------------------|---------------------|
| **Memorization** | `(V,L)→A` approximates a lookup table; model has stored specific input-output mappings | High `I(V;A\|L)` and `I(L;A\|V)` on training examples (each input independently "indexes" the answer); potentially high redundancy (both sources point to same stored action) |
| **Generalization** | Model has learned compositional/abstract mappings that transfer to novel inputs | Synergy may be higher (V and L must be *combined* in a meaningful way, not just matched); synergy should be *stable* under perturbations |

**Predicted empirical signatures (testable):**

1. **Synergy stability under perturbation:** A generalizing model should show relatively stable `Syn(V,L;A)` across V0→V2 and W0→W2 perturbations (within some tolerance). A memorizing model should show rapid synergy degradation because the lookup fails.

2. **Cross-task synergy consistency:** A generalizing model should show similar synergy patterns across L0/L1/L2 difficulty levels for the same task family. A memorizing model should show high synergy only on L0 (exact match to training).

3. **Redundancy patterns:** Memorization may paradoxically show high redundancy on training data (both V and L independently "recall" the same stored action) but near-zero MI on OOD data. Generalization should show more consistent MI across distributions.

**How to disprove H4:**
- If synergy stability under perturbation does not correlate with L0→L1→L2 performance degradation, H4 is false or requires refinement.
- If simpler metrics (entropy, confidence) predict memorization/generalization equally well, PID adds no value for this diagnostic.
- If estimator variance at VLA scale is too large to distinguish regimes, H4 is untestable with current methods.

### 3.6.3 Hypothesis H5: Compositional Failure Correlates with Temporal Synergy Degradation

**Claim (falsifiable):** VLA-Arena's finding that VLAs "cannot compose learned skills for long-horizon tasks" should manifest as synergy degradation over time within a trajectory.

**Theoretical motivation:**
- Compositional skill requires maintaining context and integrating current observations with task history
- If the model fails to compose, `Syn(V_t, D_t; A_t)` (or `Syn(V_t, V_{t-history}; A_t)`) should degrade as the task progresses and requires more composition
- Alternatively, temporal synergy (synergy between current and past states) should be low when composition fails

**Predicted empirical signatures:**
1. **Synergy half-life:** On long-horizon tasks, measure how synergy evolves over timesteps. Models that fail to compose should show earlier synergy degradation than models that succeed.
2. **Phase-specific synergy:** For tasks with distinct phases (approach, grasp, place), synergy patterns should differ by phase if the model composes skills vs. executes a single memorized trajectory.

**How to disprove H5:**
- If synergy dynamics do not predict long-horizon task success beyond simple trajectory length, H5 is false.
- If all VLAs show similar temporal synergy patterns regardless of compositional ability, H5 lacks discriminative power.

### 3.6.4 Hypothesis H6: Safety-Aware Behavior Requires Specific V-L Integration

**Claim (exploratory, lower confidence):** VLA-Arena's Safety task axis (collision avoidance, constraint satisfaction) may require distinctive V-L integration patterns.

**Motivation:**
- Safety often requires integrating visual perception of hazards with language-specified constraints
- Unlike goal-directed tasks where V and L reinforce the same action, safety may involve V signaling "danger" while L specifies "avoid"
- This could manifest as specific PID patterns (e.g., higher unique information from V when safety is relevant)

**Status:** This is an exploratory hypothesis with lower confidence than H4/H5. It is included for completeness but should not be prioritized over the core validation experiments.

### 3.6.6 Hypothesis H7: 3D Object Flow as Embodiment-Agnostic Integration Diagnostic (Dream2Flow-Inspired)

**Source:** Dream2Flow (Dharmarajan et al. 2025, arXiv:2512.24766) demonstrates that video generation models encode implicit world knowledge that can be extracted via **3D object flow** as an intermediate representation.

**Key empirical observation (paper summary):**
Dream2Flow reports that, given an initial image and task instruction, video generation models often synthesize **sensible object motions** even though converting those motions into robot actions is non-trivial. This motivates treating *object motion* as a diagnostic intermediate and treating the “embodiment gap” (realizing state changes with a specific robot/controller) as a distinct failure mode from world prediction.

**Claim (falsifiable):** When a VLA's internal world model (D) correctly predicts object-level dynamics, `Syn(V,D;A)` should be higher than when D fails to predict plausible object motion — *independent of whether the final action execution succeeds*.

**Theoretical motivation:**
- Dream2Flow separates "what should move" (object flow) from "how to move it" (robot policy)
- This decoupling suggests that **failures in (V,D)→A may conflate two distinct phenomena**:
  1. World model quality (does D predict plausible object dynamics?)
  2. Action execution (does the policy translate good D into correct A?)
- PID on (V,D;A) measures information integration but cannot distinguish these sources of failure without additional structure

**Predicted empirical signatures:**
1. **Object flow as D proxy:** If 3D object flow can be extracted from VLA's D representation (or from an external world model conditioned on D), the correlation between flow quality and `Syn(V,D;A)` should be positive.
2. **Embodiment-independent synergy:** When comparing the same task across different robot embodiments (Dream2Flow reports cross-embodiment applicability; verify the exact embodiments), `Syn(V,D;A*)` computed against *optimal* action should be more stable than `Syn(V,D;A)` against actual action.
3. **Stage-wise alignment (no fixed percentages):** Dream2Flow’s staged perspective implies separable failure sources: video generation / world prediction, flow reconstruction, and robot control/execution. If PID synergy is meant to track world-model quality, it should correlate more with **flow-quality metrics** (world prediction and reconstruction) than with downstream execution errors, once you condition on/stratify by stage outcomes.

**How to disprove H7:**
- If `Syn(V,D;A)` correlates equally with all failure stages (generation, extraction, execution), the hypothesis that synergy specifically tracks world model quality is unsupported.
- If object flow quality (as a measurable intermediate) does not correlate with PID estimates, the conceptual link between PID and Dream2Flow's paradigm is weak.
- If embodiment has no effect on the synergy-failure relationship, the "embodiment gap" confound is negligible.

**Relevance to PID-VLA:**
This hypothesis does NOT require implementing Dream2Flow. Rather, it motivates:
1. Using `A*` (optimal action) as target when possible (reduces embodiment confound)
2. Adding embodiment/robot type as a covariate in failure analysis
3. Considering object-level predictions as a D operationalization in addition to hidden states

### 3.6.7 Relationship to Existing Aims

| New Hypothesis | Maps to Aim | Experimental Locus |
|----------------|-------------|-------------------|
| H4 (Mem vs Gen) | Aim 1 (Comparative Evaluation) | VLA-Arena L0/L1/L2 stratification |
| H5 (Compositional Failure) | Aim 2 (Synergy Dynamics) | VLA-Arena Long-Horizon tasks |
| H6 (Safety) | Exploratory (Aim 1 extension) | VLA-Arena Safety tasks |
| H7 (Embodiment Gap) | Aim 3 (Causal Validation) | Cross-embodiment comparison; A* vs A targets |

**Critical constraint:** All new hypotheses inherit the Experiment 0 gate. If the estimator is invalid at VLA scale, these hypotheses cannot be tested with kNN-based `I^sx_∩`. In that case, fall back to Shannon invariants (CI screening) and treat H4-H7 as future directions contingent on estimator improvements.

---

# 4. Decomposition Strategies: What Variables to Analyze

## 4.1 The Original Proposal: V-D-A

```
I(V, D; A) = Red(V,D;A) + Unq(V;A) + Unq(D;A) + Syn(V,D;A)
```

Where:
- **V** = Vision (observed scene, from vision encoder)
- **D** = Dream (internal world model state)
- **A** = Action (motor output)

### 4.1.1 Problems with V-D-A

1. **Potential degeneracy:** because `A` is computed from `(V,D,L)`, `I(V,D;A)` can become close to `H(A)` when `L` is constant/redundant and inference is near-deterministic. Treat this as a *dataset- and inference-protocol-dependent risk*, not an identity (see Warning 2 in §1.2).
2. **L is ignored:** Language instruction is not in the decomposition
3. **D is often implicit:** In autoregressive VLAs like OpenVLA, there's no explicit "dream" state

## 4.2 Alternative: V-L-A (Vision-Language-Action)

```
I(V, L; A) = Red(V,L;A) + Unq(V;A) + Unq(L;A) + Syn(V,L;A)
```

### 4.2.1 Advantages of V-L-A

| Advantage | Explanation |
|-----------|-------------|
| **L is (usually) available** | Language is typically provided externally; no need to extract hidden states |
| **L is externally specified intent** | It encodes what the human requested (often the closest available “ground truth” for intent, but can be ambiguous) |
| **Language grounding failures are common** | "Pick up red cup" → picks blue |
| **Often more interpretable than D** | Language is more interpretable than an implicit “dream” state, but negative synergy still requires careful controls/validation |

### 4.2.2 Interpretation of V-L-A Atoms

| PID Atom | Interpretation |
|----------|----------------|
| Unq(L;A) | Action determined purely by instruction (ignoring scene) |
| Unq(V;A) | Action determined purely by visual scene (ignoring instruction) |
| Syn(V,L;A) > 0 | Joint V–L interaction appears important (candidate “integration” signal) |
| **Syn(V,L;A) < 0** | Subadditivity / potential mismatch; distinguish from estimator artifacts and redundancy inflation via controls |

### 4.2.3 Why V-L-A Might Be Better Than V-D-A

Many VLA failures are specifically **language grounding failures**:
- "Pick up the red cup" → robot picks up blue cup
- "Place it on the left" → robot places on right
- Instruction ambiguity → wrong interpretation

V-D-A cannot distinguish these from other V–D internal mismatch hypotheses without language-side controls.

## 4.3 The Question of Ignoring L

### 4.3.1 Arguments FOR Ignoring L (Original Approach)

1. **L is static within a trajectory** - doesn't change mid-execution
2. **D encodes L-conditioned predictions** - already incorporated
3. **Simplicity** - three-variable PID is tractable

### 4.3.2 Arguments AGAINST Ignoring L

1. **Language grounding failures are major failure mode** - can't detect them without L
2. **V-L-A decomposition is more interpretable** - external signals only
3. **D may not be cleanly separable** - especially in autoregressive models
4. **Liang et al. include language** - validated approach includes L

### 4.3.3 Recommendation

**Elevate V-L-A to co-primary status with V-D-A.** Test both and compare predictive power.

## 4.4 Other Decomposition Options

| Decomposition | Sources | Target | Hypothesis |
|---------------|---------|--------|------------|
| V-D-A | Vision, Dream | Action | V–D mismatch may correlate with certain failures (requires controls) |
| V-D-A* | Vision, Dream | Optimal Action | Measures error, not tautology |
| V-L-A | Vision, Language | Action | V–L mismatch may correlate with language-grounding failures (requires controls) |
| D_t-D_{t-1}-A | Current Dream, Previous Dream | Action | Temporal inconsistency → failure |
| V-A*-Error | Vision, Optimal Action | Prediction Error | Directly predicts failure magnitude |

---

# 5. Three-Way PID: I(V, L, D; A)

## 5.1 Motivation

Rather than choosing between V-D-A or V-L-A, we could analyze all three sources simultaneously:

```
I(V, L, D; A) = ?
```

This would capture:
- Vision–language mismatches
- Vision–dream mismatches  
- Language–dream mismatches (e.g., instruction misinterpretation vs world-state representation)
- Three-way synergies and redundancies

## 5.2 The Problem: Combinatorial Explosion

For two sources, PID has 4 atoms: {Red, Unq₁, Unq₂, Syn}

For three sources, the partial information lattice has **18 distinct antichains** (atoms):

```
                        {VLD}                    ← Full synergy (all three needed)
                       /  |  \
                  {VL} {VD} {LD}                 ← Pairwise synergies
                 / | \ / | \ / | \
              {V} {L} {D}                        ← Unique information
                 \ | / \ | / \ | /
                  {VL∩} {VD∩} {LD∩}              ← Pairwise redundancies  
                       \  |  /
                        {VLD∩}                   ← Full redundancy (any one suffices)
```

Estimating 18 quantities is expensive and many are hard to interpret.

## 5.3 Practical Options for Three-Way Analysis

### Option 1: Full 3-Source PID

Compute all 18 atoms.

**Pros:** Complete picture  
**Cons:** Expensive, hard to interpret, estimation variance multiplies

### Option 2: Shannon Invariants / Co-Information

Compute a summary statistic:

```python
CI(V, L, D; A) = I(V;A) + I(L;A) + I(D;A) 
              - I(V,L;A) - I(V,D;A) - I(L,D;A) 
              + I(V,L,D;A)
```

This is the "interaction information" or "co-information":
- Negative = synergistic (three-way cooperation)
- Positive = redundant (three-way overlap)

**Pros:** Single interpretable number, cheap  
**Cons:** Loses fine-grained structure

### Option 3: Hierarchical Pairwise (RECOMMENDED)

Compute three separate 2-source PIDs:

```
PID(V, L; A)  → Syn_VL  (vision-language coherence)
PID(V, D; A)  → Syn_VD  (vision-dream coherence)
PID(L, D; A)  → Syn_LD  (language-dream coherence)
```

**Diagnostic Matrix:**

| Syn_VL | Syn_VD | Syn_LD | Hypothesis (requires validation) |
|--------|--------|--------|----------------|
| + | + | + | Pairwise synergies appear positive (suggests interaction-dominant regime) |
| - | + | + | V–L interaction appears weak/subadditive relative to other pairs (check language perturbations) |
| + | - | + | V–D interaction appears weak/subadditive (check D corruption / vision occlusion controls) |
| + | + | - | L–D interaction appears weak/subadditive (check instruction changes and D dependence) |
| - | - | + | V appears atypical relative to (L,D) (could be occlusion/OOD; check estimator stability) |
| - | + | - | L appears atypical relative to (V,D) (could be instruction ambiguity; check paraphrase robustness) |
| + | - | - | D appears atypical relative to (V,L) (could be world-model mismatch; check D interventions) |
| - | - | - | Broad subadditivity across pairs (could be estimator breakdown; run controls + Experiment 0-style checks) |

**Pros:**
- Only 3× the cost of single PID
- Each pairwise synergy is interpretable
- Pattern across all three is diagnostic
- Localizes failure mode

**Cons:**
- Doesn't capture true 3-way synergy
- Some redundant computation

### Option 4: Conditional PID

Compute PID conditioned on the third variable:

```
PID(V, D; A | L)  → "Given the instruction, how do vision and dream interact?"
```

**Pros:** Controls for task variation  
**Cons:** Requires more samples per conditioning value; conditional MI estimation is itself hard in high dimension. If conditioning becomes central, consider dedicated conditional-MI estimators as baselines (e.g., CCMI, arXiv:1906.01824), but treat that as a separate validated estimator pipeline (not automatically compatible with `I^sx_∩`).

## 5.4 Recommendation

**Start with Option 3 (Hierarchical Pairwise)**, with co-information (Option 2) as a summary.

The pattern {Syn_VL, Syn_VD, Syn_LD} can help generate and localize **testable hypotheses**:
- All negative → broad subadditivity across pairs (could be genuine mismatch or estimator breakdown; investigate with controls)
- Only Syn_VL negative → candidate V–L integration issue (validate with language-side perturbations)
- Only Syn_VD negative → candidate V–D mismatch (validate with D/V interventions and estimator controls)
- Only Syn_LD negative → candidate L–D mismatch (validate with instruction changes and D dependence)

---

# 6. Discarded Approaches and Why

## 6.1 OpenVLA vs DreamVLA Architectural Comparison

### 6.1.1 The Original Idea

Compare PID profiles between:
- **OpenVLA (arXiv:2406.09246; Kim et al. 2024):** Llama 2‑based VLA with no explicit “dream/world model” prediction head; the abstract states a fused DINOv2+SigLIP visual encoder and 7B parameters. Action parameterization details should be treated as **paper/code details** (verify before using as a variable definition).
- **DreamVLA (arXiv:2507.04447; Zhang et al. 2025):** VLA framework with explicit world-knowledge forecasting channels (dynamic-region-guided prediction plus spatial and semantic cues) and diffusion-based action modeling; the abstract does not specify backbone family/dimensions, so do not assume “GPT‑2” without a primary citation.
  - Related but distinct: **Dream‑VL & Dream‑VLA (arXiv:2512.22615; Ye et al. 2025)** uses a diffusion language-model backbone; do not conflate its architectural details with DreamVLA unless explicitly matched.

**Hypothesis (weaker / testable):** Architectures with explicit predicted world-knowledge channels may yield different PID signatures than those without such channels, *under matched variable definitions and matched targets*. Whether those differences correlate with “grounding failures” remains empirical.

### 6.1.2 Why It Was Discarded

#### Reason 1: The Core Hypothesis Became Questionable

During first-principles review, we discovered that "negative synergy = hallucination" is not mathematically rigorous. It's a hypothesis, not a definition.

#### Reason 2: Too Many Confounding Variables

| Aspect | OpenVLA | DreamVLA |
|--------|---------|----------|
| Backbone | Llama 2 (abstract) | Unspecified in abstract (verify paper/code) |
| Action representation | Unspecified in abstract (verify) | Diffusion-based transformer (abstract); exact action representation verify |
| World model | Implicit/none (no explicit prediction heads) | Explicit world-knowledge forecasting (dynamic/spatial/semantic cues; abstract) |
| Vision encoder | Fused DINOv2 + SigLIP features (abstract) | Unspecified in abstract (verify paper/code) |
| Attention | Causal (autoregressive) | Block-wise structured |
| Training data | 970k real-world demos (abstract) | Unspecified in abstract (verify) |

If we observe different PID profiles, we CANNOT attribute the difference to "world model quality" because too many variables differ.

#### Reason 3: “D” Exists Explicitly in One Model but Not the Other (Definition Mismatch)

DreamVLA explicitly predicts world‑knowledge via dedicated channels/tokens. This makes a **concrete “D”** operationalization plausible *within DreamVLA* (and supports targeted interventions on D).

OpenVLA does not provide an explicit “dream/world model” output channel by default. Any “D” you define in OpenVLA is necessarily an **extracted hidden state**, which changes the scientific question (and makes cross‑model comparisons fragile).

As a result, an OpenVLA↔DreamVLA PID comparison risks becoming circular or uninterpretable: observed differences may reflect **variable-definition choices**, not “world model quality.”

#### Reason 4: "D" is Ill-Defined for OpenVLA

DreamVLA has explicit "dream" outputs (dynamic region, depth, semantics). For OpenVLA, "D" would need to be extracted from intermediate hidden states, which is:
- Arbitrary (which layer? which tokens?)
- Not comparable to DreamVLA's explicit D
- May not represent world model at all

### 6.1.3 Why It Might Still Be Interesting

Despite these issues, the comparison could be valuable IF:
1. We first validate PID on a single architecture
2. We carefully control for confounds
3. We interpret results cautiously

**Recommendation:** Defer until after core validation (Experiments 0-3).

## 6.2 Using WAN for Analytical (Not Just Visualization) Purposes

### 6.2.1 The Original Idea

WAN's 3D Causal VAE (Wan-VAE) is itself a learned world model. We could:
1. Use it as a **proxy** for what a "good" world model should predict
2. Compare VLA's synergy against WAN's synergy
3. Treat large, systematic gaps (e.g., `Syn_VLA << Syn_WAN` under *matched variable definitions*) as a **hypothesis** about failure modes, not a diagnostic; validate with labels and controlled interventions.

### 6.2.2 WAN Ecosystem Overview (Conservative, Source‑Bound)

This document uses “WAN” to refer to the open Wan video foundation model family described in arXiv:2503.20314. Many “version” labels circulate publicly; do not assume features/speeds beyond what you can cite and reproduce.

| Component | Source | What is safe to claim here |
|----------|--------|----------------------------|
| **Wan** | arXiv:2503.20314 | Open video foundation models built on diffusion transformer paradigm; paper mentions 1.3B and 14B models and an associated public code/model release |
| **VACE** | arXiv:2503.07598 | All‑in‑one framework for video creation/editing with a unified conditioning interface (VCU) |
| **Wan‑Move** | arXiv:2512.08765 | Motion control for video generation via latent trajectory guidance (dense point trajectories) |

### 6.2.3 Can WAN Be Made Action‑ or Motion‑Conditioned?

Action conditioning is not a single switch; it is a design choice about *what you condition on* (robot actions, object trajectories, constraints) and what the predictor is expected to produce (video, flow, latent plans). Treat it as a separate engineering project from PID estimation.

| Approach | Related work | PID‑VLA relevance |
|----------|--------------|------------------|
| **Embodiment adaptation + action recovery** | DreamGen (arXiv:2505.12705) | Generates synthetic videos (“neural trajectories”) and recovers pseudo‑actions via latent action model or IDM; useful context, not a direct PID estimator |
| **Unified creation/editing conditioning** | VACE (arXiv:2503.07598) | Provides a structured conditioning interface; may be useful for constructing controlled counterfactual video edits (verify suitability for robot‑state conditioning) |
| **Motion control via trajectory guidance** | Wan‑Move (arXiv:2512.08765) | Encodes desired motion with dense point trajectories; useful for counterfactual motion probes, not a closed‑loop robot simulator |
| **Unified models** | Motus (arXiv:2512.13030) | Integrates understanding, video generation, and action experts; consider as a separate baseline rather than assuming it is “WAN inside” |

### 6.2.4 Why Original Concerns Remain Partially Valid

#### Concern 1: Distribution Mismatch (Still a Primary Risk)

Video foundation models are typically trained on broad internet video, not contact‑rich robot manipulation. This can create distribution mismatch (contacts, tool use, occlusion patterns, embodiment geometry). Adaptation may require task‑ or domain‑specific conditioning/fine‑tuning; treat that as an empirical dependency and report the data/protocol you used.

#### Concern 2: Latent Space Incompatibility (Still Valid)

Policy latents and video‑model latents are not naturally aligned. For PID, prefer analyzing variables that you can define cleanly and validate (e.g., VLA latents after preprocessing + Experiment 0; or explicit Flow targets), and treat WAN primarily as an external predictor/visualization tool unless you can justify a specific intermediate as a scientifically meaningful variable.

#### Concern 3: Computational Cost (Often Offline‑Only)

Video generation is often expensive (seconds→minutes per clip, depending on model/hardware/settings). For PID‑VLA, assume **offline** unless you have measured an interactive loop on your hardware. Use caching and precompute Flow targets for analysis runs.

#### Concern 4: Circular Reasoning Risk (STILL VALID BUT MANAGEABLE)

If we fine-tune WAN on robot data, it learns similar biases. Mitigation:
- Use WAN fine-tuned on **different** robot datasets than VLA
- Use WAN only for visualization, not analytical comparison
- Use independent world models (GWM, Cosmos) for synergy comparison

### 6.2.5 Recommended Alternative: GWM (Gaussian World Model)

GWM (Gaussian World Model; see the corresponding paper/code and verify venue/status) may be more appropriate for **analytical** purposes:

| Property | WAN (base) | WAN (fine-tuned) | GWM |
|----------|------------|------------------|-----|
| Trained on robot data | No | Yes (LoRA) | **Yes (native)** |
| 3D representation | No (2D video) | No | **Yes (3DGS)** |
| Action-conditioned | No | **Yes** | **Yes** |
| Latent space alignment | Poor | Medium | **High** |
| Inference speed | Slow | Slow | Model/implementation-dependent (benchmark) |

### 6.2.6 When to Use WAN vs GWM vs Neither

| Use Case | Recommendation |
|----------|----------------|
| Core PID validation (Aims 1-2) | **Neither** - compute PID on VLA latents only |
| Debugging specific failures | **GWM** - 3D spatial localization |
| Paper figures / demos | **WAN** - video visualization (benchmark-dependent) |
| Training data augmentation | **VACE** or **GWM** - controlled edits / 3D-aware augmentation (verify) |
| Unified world model baseline | **Motus** - separate integrated model baseline (see paper) |
| Real-time intervention | **Neither** - too slow, use entropy |

### 6.2.7 Key Resources

```
WAN Official:
- GitHub (per paper): https://github.com/Wan-Video/Wan2.1
- Paper: arXiv:2503.20314

Extensions:
- Wan-Move: arxiv.org/abs/2512.08765 (motion control)
- VACE: arxiv.org/abs/2503.07598 (all-in-one editing)
- Motus: arxiv.org/abs/2512.13030 (unified latent action world model)
- DreamGen: arxiv.org/abs/2505.12705 (robot learning via neural trajectories)
```

## 6.3 Using Full 3-Source PID from the Start

### 6.3.1 Why It Was Discarded

- 18 atoms to estimate (expensive)
- Many atoms are hard to interpret
- Estimation variance multiplies
- Hierarchical pairwise gives most of the benefit

### 6.3.2 Why It's Still Potentially Interesting

True three-way synergy (information requiring ALL THREE of V, L, D) might be important for complex tasks. Worth exploring after pairwise validation.

## 6.4 Using Raw 4096-dim Embeddings

### 6.4.1 Why It's Problematic

Curse of dimensionality: at d=4096, k-NN methods fail because nearest neighbors become nearly equidistant.

### 6.4.2 Mitigation

Test dimensionality reduction:
1. PCA to 256-dim (retaining 95% variance)
2. Learned projections to 64-dim
3. Use intermediate VLA layers instead of final embeddings

---

# 7. VLA Architecture Analysis

## 7.0 Conceptual Framing: Dual-Process Theory Analogy (With Caveats)

### 7.0.1 The Analogy

An intriguing conceptual parallel exists between our V-D decomposition and dual-process theories of cognition (Kahneman, 2011). In cognitive psychology:

| System | Characteristics | Proposed VLA Analogue |
|--------|----------------|----------------------|
| **System 1** | Fast, automatic, reactive, feedforward | **V** (Vision): Direct perceptual features from early layers |
| **System 2** | Slow, deliberate, predictive, requires working memory | **D** (Dream): World model predictions requiring temporal integration |

Under this framing:
- **High synergy** might indicate coherent integration between reactive perception and predictive reasoning—analogous to healthy System 1/2 coordination
- **Low/negative synergy** might indicate a failure to integrate—analogous to the cognitive conflict when "gut feeling" contradicts deliberation

This parallel is particularly apt for **DreamVLA**, which explicitly separates:
- Vision encoder (feedforward, "System 1-like")
- World-knowledge forecasting components (dynamic/spatial/semantic cues; "System 2-like" in this loose analogy)
- Action head that conditions on the predicted knowledge (paper reports a diffusion-based action model; verify details)

### 7.0.2 Why This Analogy Is LIMITED (Important Caveats)

**We emphasize this is a loose conceptual analogy, NOT a mechanistic claim:**

1. **Timescales don't match:** System 1 and 2 differ by 100-1000× in processing speed in humans. In VLAs, V and D are computed in the same forward pass with similar latency.

2. **Architecture doesn't match:** Human dual-process theory involves distinct neural circuits (e.g., Default Mode Network vs. Prefrontal Cortex). VLAs have a single unified architecture.

3. **We are NOT testing dual-process theory:** Our decomposition is grounded in information theory (PID), not cognitive architecture. We make no claims about VLAs "implementing" System 1/2.

4. **The analogy could mislead:** Reviewers familiar with cognitive science may object to loose application of these terms.

### 7.0.3 When This Framing IS Useful

- **Grant motivation:** Helps non-technical reviewers understand the intuition
- **Discussion section:** Situates findings in broader cognitive science context
- **Future directions:** Could motivate architectures with explicit fast/slow pathways

### 7.0.4 When to AVOID This Framing

- **Core hypothesis:** Don't claim "PID measures System 1/2 integration"
- **Technical sections:** Use precise information-theoretic language
- **Wibral group review:** They are mathematically rigorous; lead with PID formalism

### 7.0.5 The Scientifically Defensible Claim

**What we CAN say:**

> "Our V-D decomposition separates early visual features from later integrated representations that incorporate world model predictions. PID quantifies how these two information streams combine to determine actions. This is conceptually analogous to—though mechanistically distinct from—the integration of fast reactive processing with slower deliberative reasoning in dual-process cognitive theories."

**What we CANNOT say:**

> "PID measures System 1/2 integration in VLAs" ❌

## 7.1 OpenVLA (arXiv:2406.09246)

### 7.1.1 What the abstract claims (minimum verification bar)

OpenVLA (arXiv:2406.09246) is described in the arXiv abstract as:
- a **7B‑parameter** open-source VLA,
- built on a **Llama 2** language model,
- with a visual encoder that fuses pretrained features from **DINOv2** and **SigLIP**,
- trained on **970k** real‑world robot demonstrations (paper-reported),
- released with checkpoints and a PyTorch codebase (paper-reported; verify availability/licensing before depending on it).

**Not verified in the abstract (do not assume without a source):** exact action representation (tokens/bins/continuous), exact fusion/projection architecture, vision encoder parameter counts, and patch/tokenization details.

### 7.1.2 Key Properties for PID Analysis

- **No explicit world model:** "D" must be inferred from hidden states
- **Causal attention:** Each token only attends to previous tokens
- **Hidden states:** if the backbone is Llama 2 7B, hidden states are 4096‑dim across 32 layers by the standard config (derived prior; verify in implementation)
- **Layer-specific encoding:** object-state vs action-state localization is *often* reported in probing studies for large transformers, but treat any specific layer claim here as **unverified until you cite a concrete probing result for OpenVLA**.

**Backbone priors (derived from Llama 2 7B config; verify in code if you cite them):**
| Component | Value |
|-----------|-------|
| Hidden size | 4096 |
| Transformer layers | 32 |
| Attention heads | 32 |

### 7.1.3 Where to Extract "D"?

Options:
1. **Layer 16 (middle):** candidate “mid-level” representation (heuristic; requires model-specific probing)
2. **Layer 24:** candidate “late” representation (heuristic; requires model-specific probing)
3. **Average across layers:** Lose layer-specific information
4. **Don't use D at all:** Focus on V-L-A decomposition

## 7.2 DreamVLA (arXiv:2507.04447)

### 7.2.1 What the abstract claims (minimum verification bar)

DreamVLA (arXiv:2507.04447) is described in the arXiv abstract as a VLA framework that:
- integrates **world-knowledge forecasting** (dynamic-region-guided prediction plus spatial and semantic cues) to support a perception–prediction–action loop,
- uses **block-wise structured attention** to mitigate interference among dynamic/spatial/semantic information (verify the exact masking scheme before claiming disentanglement),
- uses a **diffusion-based transformer** to model the conditional distribution over future actions (verify the action representation used in your analysis),
- reports success on real-robot tasks and CALVIN ABC‑D (paper-reported; protocol-sensitive).

**Not verified in the abstract:** the exact backbone family/dimensions, the exact prediction heads/targets, and weight availability/licensing.
- **Key architectural idea:** prevent interference/leakage between dynamic/spatial/semantic streams via structured attention masks
- **Vision encoder:** see paper/code (do not assume a specific pretraining method without verification)

**⚠️ Dimension caveat (abstract-level verification):**
The DreamVLA arXiv abstract does **not** specify the backbone family, hidden dimensions, or layer counts. Treat any dimensionality claims as **unverified** until you cite the paper section, code commit, or model card that states them.

**What you can and cannot assume from the abstract alone:**
| Claim | Status | Source |
|------|--------|--------|
| Backbone family unspecified + block-wise structured attention | Paper-reported | arXiv abstract |
| World knowledge forecasting includes dynamic + spatial + semantic information | Paper-reported | arXiv abstract |
| Diffusion-based transformer for action distribution | Paper-reported | arXiv abstract |
| Backbone dims/layers, exact prediction targets, query lengths, diffusion steps | Not specified in abstract | Verify paper/code |

**Diffusion parameterization note (optional, but estimator-relevant):**
Diffusion models differ in whether they predict *noise/noised quantities* vs. *clean data*. Li & He (2025, arXiv:2511.13720) argue that predicting clean data can better respect the manifold assumption. For PID/MI estimation, this matters because it may change:
- the intrinsic dimension and local geometry of latents,
- the degree of apparent determinism between latents and outputs.
If you analyze diffusion-model internal representations (DreamVLA actions or predicted world knowledge), record the model’s diffusion parameterization and which representation you treat as the variable in PID.

**Dream-VL & Dream-VLA (arXiv:2512.22615, Ye et al. 2025)** is a related but distinct line:
- Uses a **diffusion LLM backbone** (“dLLM”) for VL/VLA, emphasizing bidirectionality and parallel generation.
- Reports strong LIBERO/SimplerEnv results; treat performance numbers as benchmark-dependent and verify protocols before using as “ground truth” comparisons.

**Implementation note:** If you need a small, controllable model for end-to-end pipeline validation (logging/interventions/estimators), see §7.7; do not infer DreamVLA’s backbone choice from this document without a primary citation.

### 7.2.2 Key Properties for PID Analysis

- **Explicit D (operationalizable):** world-knowledge predictions provide a concrete candidate “D” variable (dynamic/spatial/semantic outputs and/or intermediate “world embedding”)
- **Partial stream separation:** block-wise attention is intended to reduce cross-talk between predicted knowledge components (verify exact masking scheme before treating as “disentangled”)
- **Action structure:** the action model may predict sequences/chunks depending on implementation; verify what “A” is (single-step vs chunk) before interpreting MI/PID across time
- **Caveat:** “designed for PID” is too strong; the claim we can defend is only that the architecture makes D extraction less arbitrary than in models without explicit prediction heads/tokens.

### 7.2.3 Why DreamVLA is Better for V-D-A Analysis

Relative to models with no explicit world-knowledge outputs, DreamVLA can be **more amenable** to V–D–A analysis because:
1. “D” can be defined as an explicit predicted representation (rather than “some hidden state we decided to call D”).
2. You can test interventions that specifically corrupt predicted knowledge (e.g., corrupt depth vs corrupt semantics) and see whether PID features move as expected (§9.5).
3. You can probe whether the model actually uses predicted knowledge by comparing PID features with/without access to that channel (ablation-style).

This still does not remove the degeneracy/strong-dependence concerns in §1.2: if `A` is effectively deterministic and continuous, you must define the noise/discretization model that makes the information quantities finite and interpretable.

## 7.3 PixelVLA (Pixel-Level Understanding; arXiv:2511.01571)

### 7.3.1 What the abstract claims (minimum verification bar)

PixelVLA (arXiv:2511.01571) is described in the arXiv abstract as:
- supporting **pixel-level reasoning** and **multimodal prompting** (text + visual inputs),
- integrating a **multiscale pixel-aware encoder** with a **visual prompting encoder**,
- proposing a two-stage automated pipeline that generates **Pixel‑160K** (pixel-level annotations derived from existing robot data),
- improving manipulation success rates by **10.1%–17.8%** over OpenVLA on three benchmarks while requiring **1.5%** of its pretraining cost (paper-reported; protocol-sensitive),
- releasing dataset and code as open source (paper-reported; verify availability/licensing).

**Not verified in the abstract (do not assume without a source):** the exact backbone family/dimensions, action representation (discrete/continuous), prompt encoder provenance (e.g., SAM), and any specific LoRA settings.

### 7.3.2 PID-relevant implications (conditional)

- PixelVLA introduces an explicit *visual prompt* input channel. Decide whether to treat this as part of **V** or as a separate source **P** (and preregister that choice) before running PID.
- Pixel-level and multiscale representations can have different geometry than pooled “global” embeddings; re-run the Geometry Gate at the exact extraction points you will publish.
- The scientific question is not “PixelVLA is better”, but whether adding prompt channels and pixel-level structure changes redundancy/synergy patterns under matched controls (Exp1/Exp3).

## 7.4 TraceVLA (Visual Trace Prompting; arXiv:2412.10345)

**TraceVLA** (arXiv:2412.10345, December 2024; venue/status should be verified) enhances VLAs with spatial-temporal awareness by overlaying visual state-action trajectories:

```
Current Image + Historical Trace Overlay → VLA → Action
```

- Fine-tuned from OpenVLA (7B parameters) on 150K trajectories with visual traces
- Dual visual streams: current observation + trace-overlaid image, separated by special token
- Reported gains: ~10% on SimplerEnv and ~3.5× on real-robot tasks (paper-reported; protocol-sensitive)
- Also released as a compact TraceVLA‑Phi3 variant (4B parameters, Phi‑3‑Vision backbone; paper-reported).

**Claim status (based on arXiv abstracts + backbone configuration priors; verify in code if needed):**
| Component | Dimension | Source |
|-----------|-----------|--------|
| Backbone | OpenVLA (Llama 2 7B) | arXiv:2412.10345 (abstract) |
| Parameters | 7B | arXiv:2412.10345 (abstract) |
| Hidden size | 4096 | Derived from Llama 2 7B config (verify in implementation) |
| Transformer layers | 32 | Derived from Llama 2 7B config (verify in implementation) |
| Action discretization | (unverified in abstract) | Check OpenVLA/TraceVLA code/paper before citing |
| Compact variant | TraceVLA‑Phi3 (4B) | arXiv:2412.10345 (abstract) |

**PID Relevance:** TraceVLA encodes temporal history visually. This means V implicitly contains D-like information (past states). The V-D boundary becomes blurred—interesting for testing whether PID can detect this encoding.

## 7.5 Other VLAs (For Future Reference)

| VLA | Backbone | World Model | Action Representation | Notes |
|-----|----------|-------------|----------------------|-------|
| **OpenVLA-OFT** | (unverified) | (unverified) | (unverified) | Earlier-draft placeholder; add a concrete citation before using |
| **GR00T N1** | (see paper) | Planner-style | Continuous | NVIDIA et al. (2025), arXiv:2503.14734 |
| **TinyVLA** | Smaller | None | Discrete | Efficient |
| **π₀** | (see paper) | (see paper) | (see paper) | Mentioned as a baseline in Dream-VLA/Dream-VLA-related work; add citation when used |
| **MemoryVLA** | VLM + memory bank | Working + long-term | Continuous | Shi et al. (2025), arXiv:2508.19236 |
| **CoT-VLA** | 7B + visual CoT | Predicts visual goals | Mixed | Zhao et al. (2025), arXiv:2503.22020 (performance deltas are benchmark-dependent; verify protocol) |

## 7.6 Architecture Verification Summary (Jan 2026)

This section tracks *claim status* for VLA architecture details. Treat arXiv abstracts as the minimum verification bar; treat any non-abstract details as “derived from known backbone configs” or “unverified” unless you cite a concrete source (paper section, code commit, model card).

### 7.6.1 Verified Dimension Summary

| VLA | Hidden Dim | Layers | Action Type | Verification Status |
|-----|------------|--------|-------------|---------------------|
| **OpenVLA** | 4096 (Llama 2 7B prior) | 32 (Llama 2 7B prior) | (unverified in abstract) | Abstract verifies: 7B + Llama 2 + (DINOv2, SigLIP); other specifics require a citation |
| **DreamVLA** | Unknown | Unknown | Diffusion-based transformer (paper-reported) | Abstract does not specify backbone dims; do not assume GPT‑2 sizes |
| **PixelVLA** | Unknown (abstract) | Unknown (abstract) | Unknown (abstract) | Abstract verifies pixel-level reasoning + multimodal prompting + Pixel‑160K; numeric architecture details require paper/code |
| **TraceVLA** | 4096 (inherits OpenVLA) | 32 (inherits OpenVLA) | (unverified in abstract) | Abstract verifies it fine-tunes OpenVLA; action discretization requires a citation |

### 7.6.2 Implications for Geometry Analysis

**Practical prior (from reported backbones, not from geometry):** Some VLAs targeted here are reported to use a Llama 2 7B language backbone (e.g., OpenVLA arXiv:2406.09246; TraceVLA arXiv:2412.10345). If so, the LM hidden size is 4096 by the Llama 2 7B configuration. This is not a substitute for the Geometry Gate: the *effective* dimension and local geometry of the extracted representations must still be measured.

Implications for estimator planning:
- Treat **4096** as a plausible **upper bound** for certain extraction points; pooled/projected embeddings can be lower-dimensional.
- DreamVLA’s backbone dimensionality is **not stated in the abstract** (arXiv:2507.04447); do not assume GPT‑2 sizes without checking the paper/code.
- PCA (or other reduction) is a common starting point, but must be validated with the Geometry Gate + Experiment 0 on the exact pipeline you will publish.

### 7.6.3 Intrinsic Dimension Research (Transformer Embeddings)

Selected references on transformer embedding geometry (use as motivation; do not transplant numeric claims across models/datasets):

| Finding | Source |
|---------|--------|
| ID shows **bell-shaped curve** across layers (peak in early-middle) | [The Shape of Learning, arXiv:2311.05928](https://arxiv.org/abs/2311.05928) |
| ID **increases** during early training, then **compresses** | [Comparative Study, arXiv:2412.06245](https://arxiv.org/abs/2412.06245) |
| "Sustained drop in local dimension predicts improved generalization" | [Less is More, arXiv:2506.01034](https://arxiv.org/abs/2506.01034) |
| In-context learning induces **higher ID** than supervised fine-tuning | [Comparative Study, arXiv:2412.06245](https://arxiv.org/abs/2412.06245) |
| ID can be measured using GRIDE, Levina-Bickel MLE, MoM estimators | [Measuring ID, arXiv:2503.02142](https://arxiv.org/abs/2503.02142) |

**Implication for PID-VLA**: The intrinsic dimension of VLA embeddings is **layer-dependent**, **training-dependent**, and likely **much lower** than d=4096. However, the exact ID for VLA-specific embeddings is **not yet measured** and should be part of Experiment 0 diagnostics.

### 7.6.4 Geometry Mitigation Options (when d is large / geometry fails)

Given the confirmed d=4096 for most VLAs:

| Approach | Assessment | Notes |
|----------|------------|-------|
| **Manifold unrolling (Isomap/AE)** | When geometry fails but you still want continuous `I^sx_∩` | Requires validating that the learned embedding supports L∞ neighborhoods; re-run gates. |
| **Geodesic MI / CI screening** | When you need geometry-respecting *screening* | Prefer MI/CI/Ω over continuous PID atoms; do not claim atom-level conclusions. |
| **Linear projection (PCA)** | Baseline if representation is locally flat-ish | Valid only if the Geometry Gate + Experiment 0 pass on the exact preprocessing. |
| **Quantization → discrete PID** | When high‑D/concentration makes kNN unstable | Trades geometric fidelity for count-based robustness; report sensitivity to k. |
| **Copula / rank transform** | Preprocessing for L∞ estimators | Can mitigate empty-space artifacts; validate empirically (can also destroy structure). |

**Key point:** Treat these as *options*, not guarantees. The Geometry Gate + Experiment 0 determine which (if any) are publishable for a given representation.

## 7.7 Optional: Small Custom Model for Pipeline Validation (Fallback)

This is a contingency plan if (a) a target VLA is unavailable or too expensive to run for rapid iteration, and (b) you need a controllable model to validate logging, intervention semantics, and PID plumbing.

### 7.7.1 Motivation

If **DreamVLA weights are unavailable** (§18.2.2) and running OpenVLA (arXiv:2406.09246; 7B parameters per abstract) is too heavy for rapid iteration on your hardware, consider training a small, explicitly documented model solely for **pipeline validation**. This is not required for the scientific core of PID‑VLA unless you plan to publish the model and its training data.

### 7.7.2 Design Goals (what matters for this project)

Any “small custom VLA” used here should:
- expose well-defined `(V,L,D,A)` (or a justified subset) for logging,
- be cheap enough to run repeatedly for Experiment 0/1 plumbing,
- have weights/code that can be redistributed (or at minimum, reproducibly rebuilt),
- be evaluated only as a baseline/debugging target unless it is benchmarked under matched protocols.

### 7.7.3 Training Sketch (high level; verify against your chosen backbone)

- Keep the architecture simple (frozen encoders + small trainable projection + action head) so that changes in PID statistics are interpretable.
- Prefer supervised imitation on a small task suite first; do not jump to RL until the estimator gates and logging are stable.
- Record *everything* needed to reproduce training (data hashes, code commit, hyperparameters); otherwise the model is not a credible scientific object.

### 7.7.4 Advantages for PID‑VLA

- Faster iteration on the end-to-end experiment harness (logging, interventions, replay, diagnostics).
- Cleaner ablations (fewer moving parts) if you keep encoders frozen and only change fusion/projection.

### 7.7.5 Disadvantages / risks

- No guarantee of relevance to large VLAs; treat as a plumbing check unless benchmarked.
- Risk of confounding: PID patterns can reflect projection/head idiosyncrasies rather than modality integration (see §14.7).

### 7.7.6 Compute and budget note (measurement-first)

Do not cite fixed time/cost estimates here. Compute depends on model choice, dataset size, optimizer, precision, and hardware. If this fallback is used, report **measured** training time, peak memory, and throughput for your exact configuration.

### 7.7.7 Relevance to blockers

This can mitigate “model unavailable” and “iteration too slow” blockers, but it does **not** solve estimator validity, geometry pathologies, or the need for interventions/labels.

### 7.7.8 Decision criteria (when to use it)

Use a small custom model only if it materially accelerates Experiment 0/1 engineering and you can document it well enough to be scientifically defensible. Otherwise, prioritize running the target VLA(s) with aggressive measurement-first profiling and caching.

### 7.7.9 Summary

**Status:** Optional fallback for pipeline validation. Not required for the core PID study unless you intend to publish the model + training artifacts.

## 7.8 SmolVLA (LeRobot)

**Source:** LeRobot “SmolVLA” (model card / repo; verify at time of use)

SmolVLA is treated here as a **lightweight baseline** for fast iteration and pipeline debugging. It is not used as primary evidence about the internals of large VLAs.

### 7.8.1 What must be verified (do not assume)
- Backbone (vision encoder + language model), hidden sizes, context length
- Action representation (discrete tokens vs continuous vector) and control-rate assumptions
- Training data (datasets, episodes, embodiments) and licensing
- Whether it supports asynchronous inference, and what that means operationally (e.g., perception at lower rate than control)
- Which intermediate representations can be exported reproducibly for `V/L/D` and how they align to PID variables

### 7.8.2 How it is used in this study
- **Primary role:** Exercise the end-to-end harness (logging, interventions, replay, geometry gate, Experiment 0/1) on a model that is cheaper to run than 7B‑class VLAs.
- **Not a substitute:** Do not generalize PID patterns from SmolVLA to OpenVLA/DreamVLA without explicit cross‑model replication.

### 7.8.3 Minimal integration contract (same as any VLA)
- Inputs: `(obs_rgb[, obs_depth], instruction[, state])`
- Outputs: action `A` + optional representation dumps: `V`, `L`, and one or more candidate `D` definitions (layer IDs / named hooks)
- Log: model id/revision, seed, preprocessing, layer choice, and any throttling/async semantics as part of the run metadata

SmolVLA is a practical low-resource baseline for Experiment 0/1 iteration, not a core scientific object unless you preregister its training/setup and publish artifacts.

---

# 8. Estimation and Implementation

## 8.1 The KSG Estimator in Detail

### 8.1.1 Algorithm

```python
def ksg_mutual_information(X, Y, k=3):
    """
    KSG estimator for I(X; Y).
    Uses maximum norm (Chebyshev distance) for BOTH k-NN search AND counting.
    """
    N = len(X)
    XY = np.hstack([X, Y])
    
    # Build k-NN tree using Chebyshev (max norm) distance
    tree = KDTree(XY, metric='chebyshev')
    
    # For each point, find distance to k-th neighbor
    distances, _ = tree.query(XY, k=k+1)  # k+1 because point is its own neighbor
    eps_raw = distances[:, k]  # k-th neighbor distance for each point
    # KSG uses strict inequality (< eps_raw) for marginal counts; many radius queries are <=.
    # Implement strictness by shrinking the radius in floating point.
    eps = np.nextafter(eps_raw, 0.0)
    
    # Count points in marginal balls
    tree_x = KDTree(X, metric='chebyshev')
    tree_y = KDTree(Y, metric='chebyshev')
    
    n_x = np.array([len(tree_x.query_ball_point(X[i], eps[i])) - 1 for i in range(N)])
    n_y = np.array([len(tree_y.query_ball_point(Y[i], eps[i])) - 1 for i in range(N)])
    
    # KSG formula
    from scipy.special import digamma
    I = digamma(k) + digamma(N) - np.mean(digamma(n_x + 1) + digamma(n_y + 1))
    
    return I
```

### 8.1.2 Critical Implementation Notes

1. **Use Chebyshev (max norm) for EVERYTHING:** Both k-NN search and marginal counting
2. **The digamma function:** scipy.special.digamma, NOT np.log
3. **Handle edge cases:** n_x or n_y could be 0 at boundary points
4. **Normalize inputs:** Scale each dimension to [0, 1] or standardize
5. **Tie handling matters:** document whether you implement strict `< eps_raw` via `eps = nextafter(eps_raw, 0)` + inclusive `<= eps` counting (recommended), and test it.
6. **Duplicates/quantization:** if many points are identical (or nearly so), kNN radii can collapse to 0. Detect this and either (a) add small seeded jitter, or (b) reject the run and change preprocessing.

### 8.1.3 Extension to I^sx_∩

The continuous `I^sx_∩` estimator in **Ehrlich et al. (2024)** is **not** implemented as “take the minimum of pointwise MI terms.”

Instead, it adapts KSG by replacing conjunction (intersection) neighborhoods with the **disjunction (union)** neighborhoods implied by shared exclusions.

For **two sources** `S₁,S₂` and a target `T`, under Chebyshev/L∞:

1. For each sample `i`, compute the joint disjunction distance to every other sample `j`:
   - `d_S_disj(i,j) = min( d(S₁ᵢ,S₁ⱼ), d(S₂ᵢ,S₂ⱼ) )`
   - `d_ST_disj(i,j) = max( d(Tᵢ,Tⱼ), d_S_disj(i,j) )`
2. Let `εᵢ_raw` be the distance to the `k`-th nearest neighbor under `d_ST_disj`.
   - Use **strict** semantics for marginal counts (`< εᵢ_raw`) via `εᵢ = nextafter(εᵢ_raw, 0)` (or an equivalent strict-radius rule).
3. Count neighbors within `εᵢ`:
   - `n_α(i)` = number of samples within `εᵢ` of the **source disjunction** (`d_S_disj(i,j) <= εᵢ`), including the query point
   - `n_T(i)` = number of samples within `εᵢ` in target space (`d(Tᵢ,Tⱼ) <= εᵢ`), including the query point
4. Estimate redundancy:
   - `Î^sx_∩ = ψ(k) + ψ(N) − (1/N) Σ_i [ ψ(n_α(i)) + ψ(n_T(i)) ]`

This matches the authors’ reference implementation (`gitlab.gwdg.de/wibral/continuouspidestimator`, Python package `csxpid`) and is implemented in this repo as `crates/pid-core/src/isx.rs` (`IsxMethod::EhrlichKsg`).

### 8.1.4 Beyond KSG: Alternative MI/CMI Estimators (MINE, CCMI, Gao-LNC / Local Gaussian)

This project’s *scientific object* is **Wibral-group shared-exclusions redundancy** `I^sx_∩` and the derived PID atoms. For continuous variables, the only paper-faithful estimator in scope is the **Ehrlich et al. (2024) disjunction-kNN/KSG-style estimator** (§8.1.3).

However, two realities force us to consider additional estimators as **baselines / contingency options**:
1. **High dimension** (distance concentration) can break kNN geometry.
2. **Strong dependence** (very large true MI; near-deterministic relationships) can break kNN MI even at low dimension (Gao et al., arXiv:1411.2003).

It is crucial to keep roles separate:
- **`I^sx_∩` redundancy** is *not* obtainable from a generic MI estimator unless you implement the shared-exclusions logic (statement-variable / disjunction neighborhoods).
- **Shannon invariants / co-information screening** depend only on MI/CMI terms, so in principle they can be computed with *any* MI/CMI estimator (but estimator bias can still change conclusions).

#### A) Gao et al.: kNN Robustness for Strong Dependence (still nonparametric)

Gao, Ver Steeg, and Galstyan show that common KSG MI estimators can require sample sizes scaling exponentially in the **true MI** for strongly dependent variables, due to local-uniformity assumptions (arXiv:1411.2003). They propose improved estimators that account for **local non-uniformity**.

Follow-up work by the same authors proposes a **local Gaussian approximation** MI estimator (arXiv:1508.00536), which locally fits a Gaussian around each sample to better approximate densities.

How this fits here:
- **Pros:** Targets exactly one of our biggest conceptual confounds: near-determinism / very strong dependence (common in learned models).
- **Cons:** Does not remove the curse of dimensionality; still relies on local neighborhoods; integration into the disjunction-kNN `I^sx_∩` estimator is **non-trivial** (the elegance of KSG-style cancellation relies on specific ball/rectangle volume terms).

Recommendation:
- Treat Gao-style estimators as **MI baselines** (for CI/O-information screening) and as a diagnostic tool for “KSG is failing because MI is huge,” not as a drop-in replacement for `I^sx_∩`.

#### B) MINE (Belghazi et al., 2018): Neural MI Estimation (variational)

MINE (arXiv:1801.04062) estimates MI by optimizing a neural critic over samples (Donsker–Varadhan-style variational bounds).

How this fits here:
- **Pros:** Scales to high-dimensional inputs; does not explicitly depend on nearest-neighbor geometry; can be trained with minibatches.
- **Cons (PhD-critical):** Optimization instability, estimator bias/variance trade-offs, dependence on architecture/regularization, and reproducibility challenges. MINE estimates are typically **lower bounds** and can be sensitive to training protocol; “same data, different seed” can change the number unless carefully controlled.

Recommendation:
- Use MINE as an **optional baseline** for MI-only invariants when kNN collapses in high `d` (PIVOT path).
- Do **not** mix estimator families inside a PID identity (e.g., do not compute `Syn = I(S1,S2;T) - I(S1;T) - I(S2;T) + Red` with `I(·;·)` from MINE and `Red` from disjunction-kNN).

#### C) CCMI / Neural CMI: Conditional MI in High Dimension (classifier-based)

Conditional MI is relevant when conditioning on confounders (e.g., “given instruction L, how do V and D interact?”) or when using conditional PID variants.

Classifier-based CMI estimators such as CCMI (Mukherjee et al., arXiv:1906.01824) train a classifier to distinguish samples from the joint distribution vs. a product distribution to estimate KL divergences, then assemble CMI.

How this fits here:
- **Pros:** Can handle high-dimensional `Z` (conditioning variable) where kNN CMI struggles.
- **Cons:** Requires careful negative-sample construction and classifier calibration; adds another training loop; provides an estimator of CMI, not `I^sx_∩`.

Recommendation:
- If conditional analyses become central (e.g., PID conditioned on L), prefer to treat CCMI/CMI-NN estimators as **separate baseline pipelines** and validate them with synthetic conditional systems before using on VLA data.

#### Relationship to the Wibral/Gutknecht “hierarchical” strategy

The Wibral/Gutknecht strategy (Shannon invariants + hierarchical screening) primarily addresses **scaling in number of sources** (avoiding 18+ atoms unless needed). It does **not** by itself solve high-dimensional or strong-dependence estimator pathologies.

Therefore, a scientifically clean hierarchy is:
1. **Estimator validity gate (Experiment 0):** determine what MI estimator family is trustworthy at your `(N,d)` and dependence regime.
2. **Variable-count hierarchy:** use Shannon invariants/co-information for screening across many candidate sources/windows.
3. **Full `I^sx_∩` PID:** only where (1) and (2) indicate it is meaningful, and only with paper-faithful `I^sx_∩` estimation.

### 8.1.5 Differential Geometry / Manifold-Aware Contingencies (When kNN/Hierarchical PID Fail)

This section integrates differential-geometry ideas **only where they produce actionable changes** for this project: diagnosing when kNN estimators are invalid, designing safer preprocessing, and (optionally) using manifold-aware MI estimators as MI-only baselines.

It is important to separate:
- **Scientific object (fixed):** Wibral-group shared-exclusions redundancy `I^sx_∩` (Makkeh 2021) + its continuous disjunction-kNN estimator (Ehrlich 2024).
- **Estimator geometry (variable):** how we choose coordinates/metrics/projections to make finite-sample estimation behave.
- **Metaphor vs method:** differential-geometry analogies (e.g., Lorentzian rigidity ↔ PID axiom rigidity) can be useful intuition pumps, but they are **not** evidence and do not directly yield a new `I^sx_∩` estimator. Treat them as background intuition, not as a correctness source.

#### A) First principles: what transformations are truly “free”

For continuous variables, **mutual information is invariant under per-variable diffeomorphisms** (invertible, differentiable reparameterizations applied separately):
- `I(X;Y) = I(f(X); g(Y))` for invertible smooth `f`, `g` (and similarly for multivariate blocks), even though *differential entropies* change by Jacobian terms.

Practical consequence:
- Prefer **invertible** preprocessing steps (standardization, whitening, monotone marginal Gaussianization) before resorting to non-invertible dimension reduction, because invertible reparameterizations can improve kNN geometry **without changing the true MI**.

For PID:
- **Do not assume “free invariance” the way you can for MI.** In discrete settings, `I^sx_∩` is trivially invariant under relabelings (permutations). In continuous settings, the *estimator* (and the “treat sources on equal footing” convention in Ehrlich et al. 2024) introduces metric/scale choices; even invertible reparameterizations can change finite-sample behavior and can effectively redefine what you are measuring unless carefully controlled.
- Practical rule: treat preprocessing as part of the measurement definition; keep it explicit, keep it fixed across runs, and re-validate after substantial changes (Experiment 0 subset).

Hard constraint (do not violate):
- Do **not** apply transforms that mix variables (no PCA/ICA on `[S1|S2|T]` concatenations). Mixing can change the target quantity and can also change what “source” means scientifically.

#### B) Manifold hypothesis: intrinsic dimension matters more than ambient dimension

The “curse of dimensionality” for kNN is controlled by the **intrinsic** dimension of the support, not the raw embedding size:
- Many learned representations empirically lie near a **lower-dimensional manifold** embedded in ℝᵈ.
- kNN estimators can still fail if intrinsic dimension is high, or if curvature/noise makes the local-neighborhood assumption false.

Actionable integration (add to Experiment 0, not post-hoc):
- Measure **intrinsic dimension estimates** for each variable block (`V`, `D`, `L`, `A`, and their joint concatenations used for MI) on your intended sampling unit.
- Track **distance concentration diagnostics** (e.g., nearest-neighbor distance ratios, coefficient of variation of pairwise distances) as a “geometry health check.”

Interpretation rules (scientific hygiene):
- If intrinsic dimension is still large (or unstable across subsamples), treat KSG-based MI/`I^sx_∩` as likely invalid at that operating point, even if `d_total` was reduced by PCA.
- If intrinsic dimension is low and stable, kNN may be viable *after* Experiment 0 establishes quantitative accuracy.

#### C) Riemannian / geodesic kNN MI as a contingency baseline (MI-only, not `I^sx_∩`)

If the representation is plausibly **manifold-valued** (curved support where Euclidean distances are a poor proxy for neighborhood volumes), consider manifold-aware MI estimators as **separate baseline pipelines** for MI-only screening:
- Marx & Fischer (2021, arXiv:2110.13883) propose **geodesic kNN** MI estimation on Riemannian manifolds.

Scope and limitations for PID-VLA:
- This can support **Shannon-invariant screening** (CI/O-information-style terms) in curved settings.
- It does **not** automatically provide `I^sx_∩`, because the disjunction-neighborhood construction would need to be re-derived for Riemannian/hyperbolic spaces (volume forms and product-neighborhood cancellations are nontrivial).

#### D) Hyperbolic geometry for hierarchical structure (Poincaré / Lorentz model) — optional, research-gated

Hyperbolic spaces (constant negative curvature) can represent tree-like/hierarchical structures with low distortion, motivating their use as **learned low-dimensional projections** when hierarchies are central:
- Nickel & Kiela (2017, arXiv:1705.08039): Poincaré embeddings for hierarchies.
- Nickel & Kiela (2018, arXiv:1806.03417): efficient training in the **Lorentz (hyperboloid) model**.
- Ganea et al. (2018, arXiv:1805.09112): hyperbolic neural networks.

Why Lorentzian geometry shows up here (mathematically, not physically):
- The Lorentz model represents hyperbolic space as a Riemannian manifold embedded in a Minkowski space with a **Lorentzian** bilinear form (signature `(-,+,...,+)`), which makes optimization and distance computation numerically convenient.

How this could help (hypotheses; must be tested):
- As a **hierarchy-friendly projection**, hyperbolic embeddings may capture coarse semantic structure with fewer dimensions than Euclidean PCA (useful if “hierarchy” is the relevant inductive bias).

How this could fail:
- Any non-invertible projection (including hyperbolic embedding to low dimension) changes the information quantities. Treat it like a learned projection: re-run Experiment 0-style validation and report it as a different measurement regime.
- Hyperbolic embeddings come with a **non-Euclidean distance** (Poincaré/Lorentz). Feeding hyperbolic coordinates into a Euclidean/Chebyshev kNN estimator is not principled; treat “hyperbolic + MI/PID” as a **separate estimator pipeline** (research-gated), not a drop-in preprocessing step.

#### E) Differential-geometry analogies: audit and safe usage (Jan 2026)

This repo-local PDF is best read as a *conceptual synthesis note*, not as a technical specification. Below is a line-by-line-level **classification** of its major claims into: (i) correct math, (ii) plausible but not directly useful here, and (iii) speculative/unsupported.

What is solid (mathematics, broadly standard):
- **Lorentzian vs Riemannian metrics:** signature `(-,+,...,+)` vs `(+, +, ..., +)` and the induced timelike/null/spacelike classification.
- **Conformal maps preserve causal structure** (light cones) in Lorentzian geometry; they preserve “possibility of influence” but not distances.
- **PID impossibility results exist:** it is correct that Matthias–Makkeh–Wibral–Gutknecht (2025, arXiv:2512.16662) establish strong inconsistency/impossibility statements that force trade-offs among desirable PID axioms.

What is plausible background but not an actionable method for PID-VLA (needs careful scoping):
- **Rigidity-theorem analogy:** comparing “axiom rigidity” (PID) to “symmetry/curvature rigidity” (Lorentzian conformal geometry) can be a useful intuition pump, but it does not produce estimator-level guarantees for `I^sx_∩` on embeddings.
- **Lorentz (hyperboloid) model link:** emphasis on Lorentzian signatures is indirectly relevant because modern **hyperbolic embedding** methods often use the Lorentz model, but that is a representational choice, not a proof about PID atoms.

What is speculative / not currently supported for this project (treat as hypotheses at best):
- **Direct identification of PID atoms with timelike/null/spacelike geometry:** mapping {Red, Unq, Syn} onto Lorentzian causal classes is metaphorical; PID is defined on probability distributions, not spacetime intervals.
- **“Synergy requires spacelike separation”** or similar causal-geometry necessity claims: synergy/redundancy are statistical/functional properties and can arise in many causal graph configurations; Lorentzian geometry is not a general constraint in VLA inference.
- **Claims about Wibral-lab using Lorentzian PSD fits + “spectral PID” as a core method:** may be true in some neuroscience contexts, but this is **not cited to a specific Wibral-group PID paper** and is not part of the validated `I^sx_∩` estimator line (Makkeh 2021; Ehrlich 2024; Gutknecht 2025).
- **Consciousness interpretations (redundancy↔unconscious, synergy↔conscious):** outside scope for PID-VLA; treat as speculative neuroscience interpretation, not an engineering objective.

How we use it safely:
- Keep it as *conceptual background* and as motivation to (i) treat invariances carefully, and (ii) explicitly measure geometry/intrinsic dimension before trusting kNN at scale.
- Do not treat it as evidence about `I^sx_∩` on VLA embeddings, and do not borrow its metaphors as “explanations” for observed PID signs without controlled experiments.

## 8.2 Dimensionality Reduction Strategies

### 8.2.1 Why Dimensionality Reduction is Necessary

At d=4096, k-NN suffers from:
- **Distance concentration:** All points become nearly equidistant
- **Exponential sample requirements:** Sample needs grow rapidly with intrinsic dimension; at `d≈4096`, naive kNN is typically unusable without strong low-dimensional structure and/or explicit dimensionality reduction.
- **Computational cost:** O(N² d) for naive k-NN

### 8.2.2 Options

Before non-invertible dimensionality reduction, consider **invertible reparameterizations**:
- For **MI-only terms** (KSG MI, CI screening), per-variable invertible transforms can improve kNN geometry **without changing the true MI**.
- For **`I^sx_∩`**, treat such transforms as an explicit part of the measurement definition (the estimator has metric/scale conventions); keep them fixed and validate them.

These are “geometry fixes,” not “information fixes,” and still require Experiment 0 validation.

| Method | Dimensions | Properties |
|--------|------------|------------|
| **Invertible per-variable reparameterization** (standardize; marginal Gaussianization) | 4096 | Preserves true MI; can improve kNN geometry; still validate for `I^sx_∩` |
| **Raw embeddings** | 4096 | Often unusable (distance concentration / curvature) |
| **PCA (95% variance)** | ~256 | Linear; **changes the quantity** (non-invertible); often stabilizes Euclidean kNN; re-validate |
| **Random projection (JL)** | 64–256 | Preserves **ambient Euclidean** distances; does **not** recover geodesics; changes the quantity; re-validate |
| **Hash projection (CountSketch)** | 64–256 | Fast baseline (`HashProjector`); approximate; changes the quantity; re-validate |
| **Learned projection (AE/contrastive)** | 64 | Task-specific; changes the quantity; requires training + leakage controls |
| **Hyperbolic embedding (Poincaré/Lorentz)** | ~2–64 | Non-Euclidean metric; **not drop-in** for Euclidean kNN/`I^sx_∩`; treat as a separate estimator pipeline |
| **Intermediate layers** | 4096 but different | Alternative variables (not reduction); may encode different information |

### 8.2.3 Recommendation

1. **Run geometry diagnostics first** (intrinsic dimension + distance concentration + local flatness + δ-hyperbolicity); use them to justify whether kNN/PID is plausible at all. **See §16.6-§16.7 for empirically validated testing methods.**
2. If dimensionality reduction is needed, **start with PCA** (e.g., retain 95% variance) and treat ~256 dims as an initial engineering target, not a law. **⚠️ Caveat**: PCA requires local flatness assumption; test with methods in §16.6.4.
3. Compare against a random projection baseline.
4. If normalized δ-hyperbolicity is very small (e.g., δ_rel < 0.1 under an explicit normalization), treat the space as tree-like: prefer MI-only screening / quantization (and treat hyperbolic projection as optional feature engineering that must be re-validated). See §16.7.3.
5. Consider **SAE decomposition** before PID — may yield lower effective dimension with interpretable features. See §16.8.
6. If needed, train learned projections optimized for the downstream diagnostic objective (and re-run Experiment 0 at the resulting dimension).

**Updated Decision Framework**: See §16.11 for the unified Geometry-First Protocol that integrates all diagnostics.

## 8.3 Computational Considerations

### 8.3.1 Complexity

For `N` samples, `d` dimensions, and `k` neighbors, kNN-based estimators can range from “toy-problem fast” to completely infeasible depending on intrinsic dimension and backend:

- **Brute-force exact kNN (current reference path):** `O(N²·d)` distance work per estimate.
- **Tree-based exact kNN (KD/ball tree):** typically `O(N log N)` build + `O(N log N)` queries at *low intrinsic dimension*, but degrades toward brute force as `d` grows.
- **Approximate kNN (e.g., HNSW/FAISS-style):** potentially sub-quadratic, but introduces estimator bias; only acceptable behind an explicit “approx” mode + re-validation (subset of Experiment 0).

### 8.3.2 Rust Implementation

For real-time use, implement in Rust with:
- SIMD for distance calculations
- Ball trees for efficient counting
- Parallelization across samples

### 8.3.3 Expected Latency

Do not treat any ms-level numbers as “spec truth”: wall-clock depends strongly on `(N, d, k)` and on the kNN backend (exact vs approximate, CPU vs GPU, and whether dimensionality reduction is applied).

Engineering posture:
- Use brute-force exact kNN for **Experiment 0 + correctness**.
- Treat any “real-time monitoring” goal as **Level 0/Level 1 only** (Shannon invariants / co-information), and only after aggressive dimensionality reduction and benchmarking on your target hardware.

## 8.4 Validation Strategy

### 8.4.1 Synthetic Data with Known PID

Be explicit about what is “known”:

1. **Discrete, definition-level sanity checks (lattice bookkeeping):**
   - XOR / copy / unique toy systems have clear *qualitative* structure (redundant vs synergistic vs unique-dominant), but **numeric atom values depend on the PID measure** (and some measures allow negative atoms even in simple systems).
   - Use these to sanity-check antichain ordering, atom identities, and qualitative behavior via a *discrete* SxPID implementation (e.g., `Abzinger/SxPID`).
   - These do **not** validate the continuous kNN estimator.

2. **Continuous estimator validation (the actual Experiment 0 gate):**
   - Use i.i.d. *continuous* synthetic systems where at least some MI terms are analytic (e.g., correlated Gaussians), and where adding independent noise dimensions provably leaves the true quantities unchanged.
   - Cross-check continuous `I^sx_∩` redundancy against the authors’ reference implementation (`csxpid`) on fixed datasets.
   - See §9.1 for the full protocol.

### 8.4.2 Scaling Test

Test estimator accuracy at:
- d = 10, 100, 1000, 4096
- N = 100, 1000, 10000
- k = 3, 10, 30

**Go/No-Go:** Use the Experiment 0 gate criteria in §9.1. If estimates collapse at d=4096, pivot to dimensionality reduction and re-validate.

### 8.4.3 Temporal Dependence and Sampling (Trajectory Data)

Most kNN/KSG estimators are analyzed under an **i.i.d. sample** assumption. Robotics/VLA data is naturally **temporal** (trajectories), so sampling design is part of estimator validity:

- **Do not treat “frames” as i.i.d. by default.** Adjacent timesteps are autocorrelated; effective sample size can be far smaller than raw frame count.
- **Prefer cross-trajectory sampling** when possible (e.g., one sample per rollout at a fixed phase, or a large-stride subsample) to reduce dependence.
- **If time-resolved PID is desired**, compute on explicit windows and report window size/stride; interpret as descriptive unless causal controls support it.
- **Uncertainty estimates must respect dependence:** prefer trajectory-level resampling or block bootstrap over naive per-frame bootstrap.

---

# 9. Experimental Design

## 9.0 Sampling Unit, Pointwise Outputs, and Autocorrelation (Read Before Running AUROCs)

Information estimators require multiple samples. In VLA settings, it is easy to accidentally compute a quantity that is *mathematically well-defined* but *experimentally meaningless* because the sampling unit is wrong.

Key distinctions:
- **Estimation dataset:** the collection of samples used to estimate MI / `I^sx_∩` / PID atoms (kNN geometry depends on this).
- **Prediction target:** what you want to predict (often a trajectory-level failure label).

Common pitfalls (and fixes):
1. **Within-trajectory estimation vs. i.i.d. assumptions**
   - Treating every timestep as an i.i.d. sample can be misleading due to autocorrelation.
   - Mitigation: large-stride subsampling, explicit windows, and trajectory-level/block bootstrap for uncertainty.
2. **Per-trajectory prediction needs per-trajectory features**
   - A single global PID computed “across all trajectories” is not directly usable for AUROC per trajectory.
   - For per-trajectory diagnostics, use either:
     - **Within-trajectory PID on windows** (produces a time series of atoms), and summarize (mean/min/%negative/etc.), or
     - **Pointwise/local contributions** (PPID-style): compute per-sample local MI / local redundancy contributions and derive local atoms.
3. **Static variables inside a trajectory**
   - Instruction `L` is often constant within a rollout; within-trajectory MI(L;·) is degenerate.
   - For V–L analyses, prefer cross-trajectory designs (different `L` across samples) or define a target `T` that is trajectory-level (with appropriate estimators).

This section’s experiments should explicitly state the sampling unit (frames vs windows vs trajectories) and how per-trajectory features are derived.

## 9.1 Experiment 0: Estimator Validation (MANDATORY FIRST)

### 9.1.1 Purpose

Validate that I^sx_∩ estimation works at VLA scale before any VLA experiments.

### 9.1.2 Protocol

Design principle: create regimes where the *true* information quantities are unchanged by adding nuisance dimensions, so “ground truth” is well-defined without relying on uncheckable high-d claims.

1. **Generate i.i.d. synthetic systems** (not trajectories) with clear qualitative structure:
   - **Redundant/copy-like:** both sources observe (noisy) versions of the same latent that drives `T`
   - **Unique:** `T` depends on only one source
   - **Synergy/XOR-like:** `T` depends on an interaction (e.g., discrete XOR; or continuous “XOR-like” via thresholded signs)
2. **Define a low-dimensional “signal” representation** (e.g., `d_signal = 1..10`) for each system.
3. **Embed into high dimension by concatenating independent noise features**:
   - `S1' = [S1_signal | N1]`, `S2' = [S2_signal | N2]`, with `N1,N2` independent of everything (and of each other).
   - This preserves the *true* information about `T` but stresses the kNN geometry.
4. Sweep:
   - `d_total ∈ {10, 100, 1000, 4096}` via noise concatenation,
   - `N ∈ {100, 1000, 10000}` (and higher if feasible),
   - `k ∈ {3, 10, 30}`.
4b. **Strong-dependence sweep (separate axis from “high d”):**
   - Even at low dimension, kNN MI can fail when the *true MI is large* (Gao et al., arXiv:1411.2003).
   - Add a 1D (or low-d) Gaussian-channel family where the analytic MI is known and controllable:
     - Example: `X ~ N(0,1)`, `Y = X + σ·N`, `N~N(0,1)`, so `I(X;Y) = 0.5 ln(1 + 1/σ²)` and grows without bound as `σ→0`.
   - Sweep `σ` logarithmically (e.g., `σ ∈ {1, 0.3, 0.1, 0.03, 0.01, 0.003, ...}`) at fixed `N,k`.
   - Goal: empirically map the **safe MI regime** for KSG and for the continuous `I^sx_∩` estimator (and/or show that the noiseless/near-noiseless regime is fundamentally ill-posed for continuous targets).
4c. **Geometry diagnostics (separate axis from “high d” and “strong dependence”):**
   - Estimate **intrinsic dimension** of each variable block (and of the joint spaces used in MI) using nearest-neighbor-based intrinsic-dimension estimators (e.g., Levina–Bickel MLE; TwoNN-style estimators; or other validated ID estimators).
   - Compute **distance concentration** proxies (e.g., nearest-neighbor distance ratio distributions; coefficient of variation of pairwise distances).
   - Use these as a “geometry health check”:
     - Low, stable intrinsic dimension is a prerequisite for believing kNN results after dimensionality reduction.
     - If intrinsic dimension remains large/unstable, treat KSG-based MI/`I^sx_∩` as likely invalid at that operating point (even if `d_total` was reduced).
   - Optional (MI-only baseline): if the representation is plausibly manifold-valued/curved, compare MI terms against **geodesic kNN MI** (Marx & Fischer, arXiv:2110.13883). Treat this as a separate estimator pipeline; do not claim it estimates `I^sx_∩`.
5. For each setting, measure:
   - estimate mean + variance across random seeds,
   - runtime and peak memory,
   - failure modes (ties/duplicate points, NaNs/Infs, implausible drift with `d_total`).
6. **Cross-check correctness where possible:**
   - MI terms: compare against analytic Gaussian-channel MI in low dimensions.
   - `I^sx_∩` redundancy: compare against `csxpid` (authors’ reference implementation) for small `d_total` and fixed datasets.
7. **Optional estimator baselines (keep separate from `I^sx_∩` correctness):**
   - If you implement or adopt them, compare MI-only terms against:
     - Gao et al. strong-dependence corrections (LNC / local Gaussian MI; arXiv:1411.2003, arXiv:1508.00536),
     - MINE (arXiv:1801.04062) for high-dimensional MI (treat as a trained estimator; record architecture/seed/training steps),
     - CCMI / neural CMI (arXiv:1906.01824, arXiv:1911.02277) for conditional MI when conditioning becomes central.
   - Use these baselines to decide whether MI-only **screening** can be made reliable when kNN collapses; do not treat them as “estimating `I^sx_∩`.”

### 9.1.3 Success Criteria

Define “reference” values using the low-dimensional signal system (and cross-check with `csxpid`/analytic MI where available). Because added noise dimensions are independent, the *true* MI/PID quantities should remain constant as `d_total` increases; any systematic drift is estimator pathology.

| Dimensionality (d_total) | Required Accuracy vs Reference |
|--------------------------|-------------------------------|
| d = 10 | Error < 5% |
| d = 100 | Error < 10% |
| d = 1000 | Error < 15% |
| d = 4096 | Error < 20% **or** require dim reduction (PIVOT) |

**Error definition:** use relative error when the reference magnitude is non-trivial; use absolute error thresholds for atoms expected near zero (to avoid meaningless relative blow-ups).

### 9.1.4 If Validation Fails

Before “PIVOT” decisions, diagnose *why* validation failed (high intrinsic dimension vs strong dependence vs ties/quantization vs curvature):

1. **Run geometry + dependence diagnostics:** inspect the strong-dependence sweep (4b) and geometry diagnostics (4c) to distinguish “MI is huge” vs “intrinsic dimension is huge/unstable” vs “duplicate/tie pathology”.
2. **Try invertible geometry fixes (still same true MI):** re-run after per-variable standardization/whitening and (optionally) monotone marginal Gaussianization. If conclusions change wildly, treat the kNN estimator regime as unstable.
3. **Use PCA to reduce to 256-dim** (or a dimension justified by the intrinsic-dimension diagnostics).
4. **Re-validate at the reduced dimension**
5. **If still fails, use learned projections** (explicitly trained for the downstream objective; report as a different measurement regime).
6. **If still fails, abandon kNN-based `I^sx_∩`** for this regime and pivot to validated alternatives (Shannon invariants as primary; or non-KSG MI estimators for MI-only screening), clearly reporting that `I^sx_∩` was not estimable.

Additional contingency (MI-only screening, not full `I^sx_∩`):
- If the disjunction-kNN `I^sx_∩` estimator is unusable at your `(N,d)` even after dimensionality reduction, you may still be able to run **Shannon-invariant** screening (CI/O-information) with non-KSG MI estimators (e.g., MINE / classifier-based MI), but treat this as a *different scientific pipeline* and do not claim results about `I^sx_∩` without a validated `I^sx_∩` estimator.
- Optional geometry-aware MI-only baseline: geodesic kNN MI (Marx & Fischer, arXiv:2110.13883) for manifold-valued variables; treat as a separate validated pipeline.

## 9.2 Experiment 1: Decomposition Comparison

### 9.2.1 Purpose

Determine which decomposition best predicts VLA failures.

### 9.2.2 Decompositions to Test

1. **V-D-A:** Vision, Dream → Action
2. **V-L-A:** Vision, Language → Action
3. **V-D-A*:** Vision, Dream → Optimal Action
4. **Hierarchical:** All three pairwise PIDs

### 9.2.3 Protocol

1. Collect rollouts (e.g., LIBERO-10), with clear success/failure labels and enough coverage of failure modes.
2. Decide the **sampling unit** per decomposition (see §9.0):
   - V–D–A and V–D–A*: typically windowed within-trajectory (V,D,A vary over time).
   - V–L–A: `L` is often constant within a trajectory, so prefer cross-trajectory designs or a trajectory-level target.
3. Extract embeddings (V, L, D, A, and optionally A*) with explicit pooling rules and logged preprocessing.
   - **Leakage rule (critical):** any fitted preprocessing (PCA, learned projection, SAE, normalization learned from data) must be fit on the training split only, then applied to validation/test. Never fit PCA on the full dataset if you report predictive performance.
4. Compute features at multiple fidelity levels:
   - Level 0: co-information / Shannon invariants (fastest; usable broadly).
   - Level 1/2: pairwise `I^sx_∩` PID on selected windows/episodes (expensive; targeted).
5. Convert time series into per-trajectory features (e.g., mean/min/quantiles/%negative/peak magnitude, plus duration-above-threshold).
6. Train and evaluate a predictor (logistic regression / small MLP) using **grouped** cross-validation; report AUROC + calibration + confidence intervals.
   - **Grouping rule:** do not let windows/timesteps from the same trajectory appear in both train and test folds.
   - If you evaluate across multiple tasks/instructions, consider grouping by task family or instruction template to test generalization (not just memorization).

### 9.2.4 Expected Outcome

Report which decomposition achieves highest AUROC.

## 9.3 Experiment 2: Baseline Comparison (Rigorous)

### 9.3.1 Baselines

| Baseline | Description |
|----------|-------------|
| Action predictive entropy | Entropy of the model’s action distribution/logits (for deterministic policies, use stochastic decoding temperature and/or ensembles) |
| Semantic uncertainty (VL-Uncertainty-style) | Uncertainty signals derived from multimodal semantics (as in VL-Uncertainty / related work) |
| Ensemble variance | 4 checkpoint ensemble |
| Attention entropy | Mean cross-modal attention entropy |
| Learned classifier | MLP on (V, D) features |
| Liang et al. Batch PID | Their variational estimator |
| Liang et al. CVX PID | Their convex optimization estimator |
| Process Reward Model (GRM) | Progress-based failure detection (Robo-Dopamine) |

### 9.3.2 Success Criteria

SxPID-derived features achieve AUROC **statistically significantly** > best baseline (paired bootstrap, p < 0.05) with a preregistered effect size, OR yield a well-supported negative result with clear analysis.

**Evaluation hygiene (avoid overclaiming):** select hyperparameters and “best baseline” variants using training/validation only (nested CV or a held-out test set for the final claim).

## 9.4 Experiment 3: Dimensionality Study

### 9.4.1 Purpose

Determine optimal dimensionality for PID estimation.

### 9.4.2 Conditions

1. Raw embeddings (4096-dim)
2. PCA to 256-dim
3. PCA to 64-dim
4. Random projection to 256-dim
5. Learned projection to 64-dim
6. Intermediate VLA layers (layer 16, layer 24)

### 9.4.3 Metric

Primary: AUROC for failure detection at each dimensionality.

Also report (because “best AUROC” can hide estimator collapse):
- estimator diagnostics (tie rate / zero radii, distance concentration proxies, intrinsic-dimension estimates),
- variance across seeds (bootstrap / repeated splits),
- runtime and memory (so “best” is not infeasible).

## 9.5 Experiment 4: Causal Validation

### 9.5.1 Purpose

Test whether PID-derived signals respond to controlled interventions in a way consistent with a causal interpretation (not merely correlation).

### 9.5.2 Protocol

This experiment is only meaningful if **D is operationally interventionable** (e.g., an explicit predicted channel in DreamVLA) and if you define a target that avoids the “A is deterministic” tautology (prefer `A*` or an external failure/success label).

1. **Paired rollout design (reduce confounds):** for each initial state/instruction seed, run a baseline rollout and an intervention rollout that differs only in the D-intervention (same environment seed when possible).
2. **Intervention family (explicitly enumerate):**
   - **Ablation:** drop/mask a D channel (e.g., depth tokens) before fusion.
   - **Noise injection:** add calibrated noise to D (sweep noise level).
   - **Permutation (dependence-breaking) control:** randomly permute D across samples/episodes to break dependence on V while preserving D’s marginal distribution (offline analysis; or online if architecture allows swapping).
3. **Measurement:** compute the relevant PID/Shannon-invariant features under a fixed preprocessing pipeline:
   - If using `A*`: compute PID on `(V,D)→A*` (or on error `E=A−A*`) so the target is external.
   - If using a failure label: treat this as a mixed discrete/continuous setting; either discretize appropriately or use MI-only screening features as the primary statistic (do not pretend continuous kNN PID applies unchanged).
4. **Predictions (pre-register):**
   - D-degrading interventions should reduce `I(D;A*)` and shift the corresponding PID signatures (e.g., Unq(D) and/or Syn(V,D;A*) depending on architecture).
   - Dependence-breaking interventions (permutation) should collapse D-related terms toward 0 under ideal estimation (a strong estimator sanity check).
5. **Outcomes:** compare paired failure rates and PID feature shifts with paired statistical tests (e.g., paired bootstrap over seeds/episodes).
6. **Controls (“placebo”):** include at least one intervention expected to be task-irrelevant (e.g., perturb a D subspace empirically shown to be unused by the policy) and verify it does not systematically change either PID features or failure rate.

**Critical caveat:** an intervention can change PID features without being “the cause of failure” if it induces broader distribution shift. Interpret this experiment as *intervention consistency evidence*, not as full causal identification.

### 9.5.3 Expected Outcome

If PID-derived signals have causal relevance: interventions that degrade `D` in task-relevant ways should shift the corresponding PID metrics in the predicted direction *and* increase failure rates; controls should not.

## 9.6 Success Criteria Summary

| Outcome | Interpretation | Action |
|---------|----------------|--------|
| SxPID-derived features outperform the best baseline with statistical significance (paired bootstrap, p < 0.05) and a preregistered effect size | **Strong success** | Proceed with Aim 2, 3 |
| SxPID-derived features are competitive but gains are small/unstable across seeds/splits | **Moderate / uncertain** | Proceed cautiously; refine sampling/preprocessing and re-test |
| A different PID feature (not synergy) is consistently best | **Conditional success** | Pivot to the best-performing atom/summary |
| Baselines match or beat SxPID (with clear significance) | **Negative result** | Prefer simpler methods; write up limits/lessons |

## 9.7 VLA-Arena Integration and Experimental Alignment (v5.8)

VLA-Arena (arXiv:2512.22539) provides a structured benchmark that directly aligns with PID diagnostic goals. This section specifies how PID experiments should leverage VLA-Arena's framework.

### 9.7.1 VLA-Arena Structure Overview

**Three Orthogonal Difficulty Axes:**

| Axis | Levels | Description | PID Relevance |
|------|--------|-------------|---------------|
| **Task Structure** | L0→L1→L2 | Fine-tuning on L0 only; L1/L2 test generalization | Tests H4 (memorization vs generalization) |
| **Language (W)** | W0→W4 | Perturbation levels for language commands | Tests V-L synergy robustness |
| **Visual (V)** | V0→V4 | Perturbation levels for visual observations | Tests V-D synergy robustness |

**Four Task Dimensions (170 total tasks):**

| Dimension | Description | PID Prediction (Hypothesis) |
|-----------|-------------|---------------------------|
| **Safety** | Collision avoidance, constraint satisfaction | H6: Safety-aware integration may show distinct V-L patterns |
| **Distractor** | Irrelevant objects/information in scene | Robust synergy should ignore distractors; fragile synergy should degrade |
| **Extrapolation** | Novel object/scene configurations | Tests generalization; synergy stability predicts success (H4) |
| **Long Horizon** | Multi-step compositional tasks | Tests temporal synergy dynamics (H5) |

### 9.7.2 Experimental Protocol: Perturbation-Based PID Robustness

**Goal:** Determine whether PID estimates are robust to controlled distribution shifts and whether robustness predicts task performance.

**Protocol:**

```
FOR each task family in VLA-Arena:
    1. Run VLA on L0 tasks (training distribution)
       - Extract embeddings (V, L, D, A)
       - Compute PID features: Syn, Red, Unq_V, Unq_L, CI
       - Record success rate
    
    2. Run VLA on same tasks with V-perturbations (V1, V2, V3)
       - Compute PID features at each perturbation level
       - Record success rate
       - Measure: |ΔPID|/|ΔSuccess| (sensitivity ratio)
    
    3. Run VLA on same tasks with W-perturbations (W1, W2, W3)
       - Same measurements as step 2
    
    4. Compare V-perturbation sensitivity vs W-perturbation sensitivity
       - Asymmetric robustness → modality-specific integration weakness
       - Symmetric robustness → balanced integration
    
    5. Test L1, L2 difficulty levels
       - Compare PID patterns to L0
       - If PID stable but performance drops → generalization failure unrelated to integration
       - If PID degrades and performance drops → potential integration-based explanation
```

**Metrics to Report:**

| Metric | Definition | Interpretation |
|--------|------------|----------------|
| **Synergy Stability Index (SSI)** | `1 - Var(Syn)/Mean(Syn)` across perturbation levels | Higher = more robust integration |
| **Modality Asymmetry Ratio (MAR)** | `|ΔSyn_V|/|ΔSyn_W|` for matched perturbation severity | >1 = V-sensitive; <1 = W-sensitive |
| **Generalization Synergy Gap (GSG)** | `Syn_L0 - Syn_L2` | Positive = synergy degrades with generalization demand |

### 9.7.3 Mapping VLA-Arena Dimensions to PID Predictions

**Dimension: Safety**

| VLA-Arena Observation | PID Prediction | Falsification Criterion |
|-----------------------|----------------|------------------------|
| VLAs ignore safety constraints | Safety tasks may require integrating "avoid" signals from V with "constraint" signals from L | If Syn(V,L;A) is indistinguishable on safety vs non-safety tasks, H6 is unsupported |
| Collision avoidance failures | High `Unq(V)` may indicate model sees danger but doesn't integrate with task | If collision failures don't correlate with PID patterns, simpler metrics suffice |

**Dimension: Distractor**

| VLA-Arena Observation | PID Prediction | Falsification Criterion |
|-----------------------|----------------|------------------------|
| Performance degrades with distractors | Robust integration should show stable synergy despite distractors | If synergy degrades proportionally to distractor count but success doesn't, synergy is noise-sensitive |
| Models attend to irrelevant objects | `Unq(V)` may increase (visual information not integrated with language) | If attention-based metrics predict distractor failures better than PID, prefer attention |

**Dimension: Extrapolation**

| VLA-Arena Observation | PID Prediction | Falsification Criterion |
|-----------------------|----------------|------------------------|
| VLAs fail to extrapolate to novel configurations | Memorizing models: synergy stable on training, unstable on novel | If synergy stability doesn't predict extrapolation success, H4 is wrong |
| "Memorization over generalization" | PID patterns may differ systematically between L0 (memorized) and L2 (requires generalization) | If PID patterns are identical on L0 and L2, this hypothesis fails |

**Dimension: Long Horizon**

| VLA-Arena Observation | PID Prediction | Falsification Criterion |
|-----------------------|----------------|------------------------|
| Cannot compose learned skills | Temporal synergy may degrade as task progresses | If temporal synergy is stable but composition still fails, synergy is not the bottleneck |
| Multi-step tasks fail | Phase-specific synergy patterns may reveal where composition breaks | If all phases show similar synergy but some fail, synergy lacks diagnostic power |

### 9.7.4 VLA-Arena Datasets for PID Experiments

VLA-Arena provides three dataset scales for fine-tuning:

| Dataset | Size | Recommended Use for PID |
|---------|------|------------------------|
| **VLA-Arena-S** | Small | Initial Experiment 0 validation + rapid iteration |
| **VLA-Arena-M** | Medium | Primary experiments (Experiments 1-3) |
| **VLA-Arena-L** | Large | Full-scale validation if M shows positive results |

**Fine-tuning Protocol (matching VLA-Arena):**
- Fine-tune only on L0 tasks
- Evaluate on L0 (in-distribution), L1, L2 (generalization)
- Apply V and W perturbations independently
- This ensures PID experiments match VLA-Arena's evaluation protocol for comparability

### 9.7.5 Expected Null Results (Pre-Registration)

For scientific rigor, we pre-register expected null results:

1. **PID may not distinguish memorization from generalization** if the phenomenon is purely about lookup vs. computation, not information integration.

2. **Synergy stability may not predict perturbation robustness** if robustness is dominated by low-level perceptual factors.

3. **Long-horizon failures may not correlate with temporal synergy** if composition failures are architectural (context length, recurrence) rather than information-theoretic.

4. **Safety task patterns may not differ** if safety is handled by the same integration mechanisms as goal-directed behavior.

Observing these null results is **valid scientific outcome** that would redirect focus to simpler baselines.

### 9.7.6 Success Criteria for VLA-Arena Integration

| Outcome | Interpretation | Next Steps |
|---------|----------------|------------|
| Synergy stability predicts L0→L2 performance drop | **Strong support for H4** | Publish memorization/generalization diagnostic |
| V/W asymmetry in PID correlates with asymmetric robustness | **Support for modality-specific diagnosis** | Develop modality-specific interventions |
| Temporal synergy predicts long-horizon success | **Support for H5** | Develop synergy-based curriculum |
| None of the above | **Null result** | Report limits; prefer simpler baselines |

### 9.7.7 Dream2Flow Stage Taxonomy as PID Diagnostic Framework (v7.0)

Dream2Flow (arXiv:2512.24766) motivates a methodology that is directly useful for PID: if you can expose intermediate artifacts (generated video, extracted flow, executed trajectory), you can avoid misattributing “end‑to‑end failure” to the wrong mechanism. PID‑VLA adopts the same *stage-wise logging* idea whenever a Dream2Flow‑style bridge is used.

**Stage-wise logging schema (required when using Flow-as-Bridge):**

| Stage | Artifact(s) to persist | Typical failure modes (examples) | PID interpretation / action |
|-------|-------------------------|----------------------------------|-----------------------------|
| **S1: Video prediction** | Prompt, seed, generated clip(s), per-frame metadata/uncertainty if available | Identity drift, object morphing, instruction mismatch, implausible contacts | Treat the predictor output as a *proposal*, not ground truth; do not interpret PID deltas as “the VLA is wrong” until S1 plausibility is checked |
| **S2: Flow reconstruction** | Masks/segments, 2D tracks + confidence, depth/spatial cues + calibration metadata, lifted 3D trajectories | Track drift, occlusion failure, depth scale ambiguity, segmentation leakage | Perception/measurement confound: if S2 fails, Flow targets are unreliable and PID results are not interpretable without controls (§14.5, §10.4) |
| **S3: Control / execution** | Controller inputs, planned trajectory, executed trajectory, constraint violations | Actuator limits, contact mismatch, planner infeasibility | Embodiment gap: compare against `A*`/planned trajectories when possible; avoid blaming “world model quality” for execution infeasibility |

**How to use the taxonomy for PID analysis (examples):**
1. **Attribute failures before interpreting atoms:** label each episode with `(S1_ok, S2_ok, S3_ok)` and stratify all PID plots by these labels.
2. **Separate “prediction quality” from “control realizability”:** analyze `Syn(V, D_ext; Flow)` (prediction agreement) separately from `Syn(V, Flow; A)` / `I(Flow; A)` (realizability).
3. **Avoid “oracle” framing:** the generated video/flow is *not* ground truth; validate Flow plausibility against simulator/sensor trajectories when available.

**Caveats:**
- Dream2Flow motivates stage separation, but stage outcomes depend on the chosen video model, tracker, and controller; report your measured stage rates rather than assuming fixed success rates.
- If you cannot log intermediate artifacts, this taxonomy is inapplicable and failures collapse back to standard end‑to‑end evaluation.

## 9.8 Extended Comparisons: World Models & Deformables (Experiments 6-10)

**Priority note:** These are optional extensions and should not block Experiments 0–4.

This section extends the experimental suite to address advanced world model capabilities and deformable object manipulation, comparing **ManiGaussian** (learned/implicit) vs **PEGS** (explicit/PBD).

| Exp | Name | Hypothesis | Key Metric |
|-----|------|------------|------------|
| **6** | **Prediction Fidelity** | H_WM1: Learned models win in-distribution; explicit models win OOD | `I(Prediction; P_GT)` |
| **7** | **Novel Object Generalization** | H_WM2: Explicit physics generalizes to novel geometry/mass better | Success rate drop |
| **8** | **Physics Perturbation** | H_WM3: Visual correction (PEGS) adapts to mass/friction changes | Synergy variance |
| **9** | **Temporal Coherence** | H_WM4: Explicit state maintains coherence in long tasks | Synergy degradation slope |
| **10** | **Deformable Objects** | H_WM5: Particle-based physics handles rope/cloth; rigid assumptions fail | Task success rate |

**See `EXPERIMENTS.md` Sections 14-19 for detailed protocols.**

---

# 10. World Model Integration (WAN, GWM, 3DGS)

## 10.1 Overview

External world models can serve as:
1. **Visualization tools:** Render what the VLA "thinks" will happen
2. **Analytical baselines:** Compare VLA predictions against reference predictions
3. **Data augmentation:** Generate synthetic training data
4. **Training environment generation:** Create unlimited simulation scenarios

**Scope / verification note:** this entire section is *optional* and contains a mix of (a) paper-reported capabilities and (b) engineering design sketches. Treat architecture diagrams and runtime/latency numbers as **to-be-verified on your hardware and data**, and do not let this section block the core PID validation (Experiment 0 + Experiments 1–3).

### 10.1.1 World Model Taxonomy

Understanding the different roles of "world models" is critical for proper integration:

| Type | Example | Role in Pipeline | Action-Conditioned? |
|------|---------|------------------|---------------------|
| **Internal (VLA)** | DreamVLA world-knowledge forecasting (dynamic/spatial/semantic; arXiv:2507.04447) | Predicts world knowledge used for action decisions | Yes (in the sense of conditioning action on predicted knowledge; verify exact conditioning) |
| **Evaluative** | WAN, GWM | Visualize/validate VLA predictions | Via LoRA/VACE fine-tuning |
| **Generative (Environment)** | Genie 3, Isaac Sim | Create training environments for agents | Yes (responds to agent actions) |
| **Perceptual Foundation** | DKT, Depth-Anything | Improve visual input quality | N/A (perception preprocessing) |
| **Video-to-Flow (v6.1)** | Dream2Flow (arXiv:2512.24766) | Extract object dynamics from video models; embodiment-agnostic intermediate representation | Via trajectory optimization or RL |

**Key Insight:** PID analysis operates on the **internal** world model (D) within a VLA. External world models (WAN, GWM, Genie 3, Dream2Flow) can support PID analysis by:
1. Providing reference predictions to compare against VLA's D
2. Generating training environments where PID patterns can be studied
3. Improving visual input quality (V) for more interpretable PID results

### 10.1.2 Genie-like environment generators (optional; out of scope)

Some work explores generative models that produce **interactive environments** (e.g., “Genie”-style systems). They may be useful for synthetic data generation or RL fine-tuning, but they are not required for PID‑VLA and introduce additional confounds (the generator’s physics validity becomes part of the experimental condition).

If such a generator is used, treat it as another **versioned experimental variable** with the same provenance requirements (pinned revision, logged prompts/actions, and explicit “no oracle” framing).

## 10.2 WAN (Wan Video Model Family)

### 10.2.1 Architecture (High‑Level; Source‑Bound)

Wan (arXiv:2503.20314) is described as a diffusion‑transformer family for video generation with a VAE‑style video autoencoder. Treat detailed latent shapes/compression ratios and any “versioned” architecture claims as implementation‑specific; verify directly from the paper/release you use before relying on them for scientific arguments.

**Reported model sizes (paper):** 1.3B and 14B. The paper reports the 1.3B model can run with ~8.19 GB VRAM in their setting; benchmark on your hardware/runtime.

### 10.2.2 Extensions Relevant to Robotics

| Extension | Paper | Capability |
|-----------|-------|------------|
| **VACE** | arXiv:2503.07598 | All-in-one video creation/editing with Video Condition Unit |
| **Wan‑Move** | arXiv:2512.08765 | Motion control via latent trajectory guidance |
| **DreamGen** | arXiv:2505.12705 | Robot learning via neural trajectories (video‑model‑driven synthetic data + action recovery) |
| **Motus** | arXiv:2512.13030 | Unified latent action world model integrating understanding/video generation/action experts (do not assume it is “WAN inside”) |

### 10.2.3 Potential Uses (Revised)

| Use Case | Feasibility (indicative) | Notes |
|----------|-------------|-------|
| Dream visualization | Medium‑High | Render predicted futures (benchmark-dependent) |
| Hallucination visualization | Medium‑High | Compare predicted vs actual under controlled conditions |
| Analytical baseline | Medium | Use as *reference predictor* only; avoid oracle framing |
| Motion/action conditioning | Medium | Requires method support and/or training (verify) |
| Unified world model baseline | Medium | Consider Motus/VideoVLA/Dream‑VLA as separate baselines |
| Real-time intervention | Low | Video prediction is usually too slow for tight control loops |

### 10.2.4 Limitations (Revised)

**Base model limitations (relevance to PID‑VLA):**
- Not trained specifically on robot manipulation (domain mismatch is likely).
- Not natively a closed‑loop, action‑conditioned simulator; extensions may support motion guidance or structured conditioning.
- Latent spaces are not aligned with VLA representations; avoid direct latent comparisons unless you justify the variable definition.
- Use in PID‑VLA is primarily as a *reference predictor* and as a generator of artifacts for Flow reconstruction (stage‑wise logged; §9.7.7).

### 10.2.5 Conditioning Approaches (Pseudocode‑Level)

Implementation details vary rapidly across releases and libraries. Treat the following as **conceptual categories**, not guaranteed APIs:
1. **Adapter/fine‑tuning:** add lightweight adapters and fine‑tune on robot trajectories (if licensing/data allow).
2. **Structured conditioning/editing:** use creation/editing frameworks (e.g., VACE) to apply controlled edits/conditions.
3. **Trajectory guidance:** apply motion control mechanisms (e.g., Wan‑Move) to impose desired point‑trajectory motion.

## 10.3 GWM (Gaussian World Model)

### 10.3.1 Why GWM is Better for Analysis

| Property | WAN (base) | WAN (fine-tuned/VACE) | GWM |
|----------|------------|----------------------|-----|
| Trained on robot data | No | Partially (LoRA) | **Yes (native)** |
| 3D representation | No (2D video) | No | **Yes (3DGS)** |
| Action-conditioned | No | **Yes** | **Yes** |
| Latent space alignment | Low | Medium | **High** |
| Inference speed | Slow | Slow | **Faster** |
| Modification required | None | LoRA + data | None |

**When to Choose:**
- **GWM:** Analytical comparison, synergy baseline, 3D spatial reasoning
- **WAN (fine-tuned):** Visualization, data augmentation, unified models (Motus)
- **WAN (base):** Paper figures, demos only

### 10.3.2 Integration Architecture

```
Current Frame → Shared Vision Encoder → V
                       ↓
            ┌─────────────────────┐
            ↓                     ↓
       VLA World Model      GWM World Model
            ↓                     ↓
         D_vla                 D_gwm
            ↓                     ↓
            └─────────────────────┘
                       ↓
               PID Comparison
          Syn(V, D_vla; A) vs Syn(V, D_gwm; A)
```

## 10.4 Depth Perception Methods

### 10.4.1 Monocular Depth Estimation

| Method (examples; verify current versions) | Output | Notes |
|--------|--------|-------|
| **Depth-Anything (v2 or similar)** | Relative depth | Widely used; scale ambiguity unless calibrated |
| **Metric3D (or similar)** | Metric/scale-aware depth | Better for absolute geometry if calibrated; often slower/heavier |
| **Video depth models** | Temporally consistent depth | Useful when depth flicker is a confound |
| **Diffusion-based depth (optional)** | Depth with strong priors | Potentially robust in hard cases; benchmark carefully |

**Recommendation:** Start with a well-supported relative-depth model (e.g., Depth-Anything v2) and validate on your scenes. If you need absolute scale, use a metric-depth baseline (e.g., Metric3D) or calibrate relative depth; benchmark for accuracy and latency on your deployment.

### 10.4.2 Stereo Vision (StereoVLA Approach)

**Key Insight from StereoVLA (arXiv:2512.21970):** Rather than relying on monocular depth estimation, stereo vision provides direct 3D geometry from binocular disparity.

| Approach | Pros | Cons |
|----------|------|------|
| Monocular depth | Single camera, any setup | Estimated, not measured |
| Stereo vision | True geometry, accurate | Requires calibrated stereo pair |
| RGB-D sensor | Direct depth | Limited range, sensor cost |

**Integration with PID-VLA:**
```
Stereo Pair → Disparity Estimation → 3D Point Cloud → VLA Visual Encoder
                                                    ↓
                                              Enhanced V with native depth
```

StereoVLA shows improved spatial reasoning by providing the VLA with native 3D information rather than requiring the model to infer depth from monocular cues.

### 10.4.3 Transparent Object Depth (DKT)

**Problem:** Standard depth methods (RGB-D sensors, stereo, monocular estimation) fail on transparent, translucent, and reflective objects due to:
- Light refraction through glass/plastic
- Specular reflections on shiny surfaces
- Time-of-flight sensors receive corrupted signals
- Stereo correspondence fails on textureless transparent regions

**The “Diffusion Knows Transparency” principle (interpret cautiously):**

DKT (arXiv:2512.23705) argues that strong video diffusion priors can help infer depth for transparent/reflective objects. Interpret this as **learned statistical regularities** that are often consistent with light transport (refraction/reflection), not as evidence that the model “understands physics” in a mechanistic sense.

**Implementation note:** The base model, datasets, outputs (depth/normals), and runtime characteristics are paper-specific. Do not copy architecture diagrams or speed numbers into this spec without a primary citation and a local benchmark; treat DKT as an optional perception preprocessor whose value must be validated on your scenes and tasks.

**PID Relevance: Why Transparent Object Depth Matters for V-D Analysis**

This is a genuine connection to PID diagnostics:

1. **V quality affects interpretability:** if the visual representation `V` is dominated by perception artifacts (e.g., transparent-object depth failures), the resulting MI/PID quantities are still mathematically well-defined but can become **semantically uninterpretable** for “integration quality” questions:
   ```
   Perception artifact → V no longer tracks scene geometry → PID reflects the artifact regime
   ```

2. **V-D Mismatch from Perception Failure:** When standard depth sensors fail on glass:
   - V contains incorrect geometric information
   - D (world model) may predict correct physics
   - This creates *apparent* V-D mismatch that is actually a perception failure, not a world model failure
   
   **Testable hypothesis:** Using a better depth representation for transparent objects can change PID features by reducing perception-driven noise in V. Whether this increases or decreases synergy is empirical and must be measured under controls (Exp3 perturbations + ablations).

3. **Failure Mode Attribution:** Without accurate transparent object depth:
   - Low Syn(V,D;A) could indicate either:
     a) World model failure (D is wrong about physics)
     b) Perception failure (V is garbage)
   - DKT removes (b) from the equation, enabling cleaner diagnosis

4. **Avoid over-interpretation:** Even if DKT improves depth, this does not imply “physics understanding”. Treat it as a statistical prior that can reduce a confound in PID analysis.

**When to Use DKT in PID-VLA Pipeline:**
| Scenario | Recommendation |
|----------|----------------|
| Tasks involving glass/plastic | Use DKT for V preprocessing |
| Diagnosing transparent object failures | Essential for valid PID |
| General manipulation (opaque) | A standard depth estimator (e.g., Depth-Anything v2 or similar) may suffice (validate on your scenes) |
| Speed-critical real-time | DKT may be too slow; consider stereo/RGB-D or simpler depth baselines |

## 10.5 3DGS (3D Gaussian Splatting)

### 10.5.1 Role in Pipeline

- **SHARP:** Single-image to 3DGS conversion (latency is hardware/model dependent; benchmark)
- **Depth model:** Depth estimation (relative or metric; calibrate if absolute scale is required)
- **SparkJS:** 3DGS rendering in browser

### 10.5.2 When 3DGS Adds Value

- Debugging spatial reasoning failures
- Visualizing occlusion/depth errors
- Training data for 3D-aware policies

### 10.5.3 When 3DGS is Overkill

- Core PID diagnostics (2D sufficient)
- Real-time intervention (too slow)
- Most failure modes don't require 3D

## 10.6 Recommendation

| Task | Tool | Notes |
|------|------|-------|
| Core PID (Aims 1-2) | None (VLA latents only) | Avoid world model confounds |
| Failure debugging | GWM | 3D spatial localization |
| Paper figures | WAN (base) | Highest visual quality |
| Action-conditioned visualization | WAN VACE / Motus | Via conditioning pipeline |
| Data augmentation | WAN (fine-tuned) or GWM | Both support actions |
| Unified world model baseline | Motus | Integrated video+action baseline (see arXiv:2512.13030; do not assume underlying video model) |
| 3D spatial analysis | SHARP + 3DGS | Single-image 3D |
| Real-time monitoring | Entropy (not PID) | <100ms requirement |
| Simulation + visualization | Headless Gazebo + Tauri | Low-latency interactive |
| Transparent object depth | DKT | Essential for glass/plastic manipulation |
| RL environment generation | Genie 3 | Unlimited interactive training scenarios |
| VLA pre-training environments | Genie 3 + SIMA 2 | Procedural world generation |

## 10.7 World Model Paradigms and PID Implications

### 10.7.1 Theoretical Framework

Different world models serve different roles. Understanding this prevents category errors in PID analysis:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                       WORLD MODEL PARADIGMS                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  1. INTERNAL (D in PID)        2. EVALUATIVE              3. GENERATIVE │
│  ─────────────────────         ────────────              ───────────────│
│  Lives inside VLA              External reference         Creates envs   │
│  Predicts s' from (s,a)        Validates predictions      For training   │
│                                                                          │
│  Examples:                     Examples:                  Examples:      │
│  • DreamVLA world-knowledge    • WAN video gen           • Genie 3      │
│  • Hidden states               • GWM 3D prediction       • Isaac Sim    │
│  • Implicit in attention       • DKT depth               • Gazebo       │
│                                                                          │
│  PID measures THIS       →     Can compare with D   →    D learns from  │
│         ↓                              ↓                        ↓        │
│  Syn(V, D_internal; A)         Syn(V, D_internal; A)    Quality affects │
│                                vs Syn(V, D_external; A)  what D learns  │
└─────────────────────────────────────────────────────────────────────────┘
```

### 10.7.2 How External World Models Affect Internal D

**Training in Genie 3 → Effect on VLA's D:**

When a VLA trains in Genie 3-generated environments:
1. The VLA's internal world model D learns from Genie 3's emergent physics
2. Genie 3's physics are learned via self-supervision (not hardcoded)
3. If Genie 3's physics diverge from reality, D learns incorrect dynamics

**PID Prediction:**
```
VLA trained in Genie 3         VLA trained in real world
────────────────────────       ─────────────────────────
D_genie                        D_real

When deployed in reality:
• If Genie 3 physics ≈ real physics: Syn(V,D_genie;A) ≈ Syn(V,D_real;A)
• If Genie 3 physics ≠ real physics: Syn(V,D_genie;A) < Syn(V,D_real;A)
                                     (V shows real physics, D expects Genie physics)
```

**Testable Hypothesis:** VLAs trained in Genie 3 will show lower synergy on tasks where Genie 3's emergent physics differ most from reality (e.g., precise contact dynamics, fluid interactions).

### 10.7.3 Generative Priors as a Testable Premise (Not an Assumption)

Large generative models (video predictors, depth estimators, simulators with learned dynamics, etc.) may encode priors that resemble aspects of physical dynamics. PID‑VLA treats this as a **testable premise**, not a fact: “plausible video” is not the same as physically correct state transitions.

**Implication for PID (conservative):**
1. If a VLA (or auxiliary predictor) encodes useful dynamics priors, predicted‑future quality (or flow plausibility) should correlate with task success under preregistered metrics and persist under controlled perturbations.
2. If not, treat the predictor as a visualization instrument only; avoid attributing PID differences to “physics understanding.”

**Exploratory hypothesis (requires matched protocols):**
Policies with explicit prediction loops (predicting intermediate future cues/flows) may show different PID signatures on physics‑heavy tasks than purely reactive policies, but this must be tested on the same tasks, with the same variable definitions, after the Experiment 0 + geometry gates.

### 10.7.4 Perception Quality as PID Prerequisite

**The DKT lesson:** Before attributing low synergy to world model failure, verify perception quality.

```
Failure Mode Diagnostic Tree:
                         Low Syn(V,D;A)
                              │
                 ┌────────────┴────────────┐
                 │                         │
        V is accurate?              V is corrupted?
        (use DKT/stereo)            (depth sensor failure)
                 │                         │
          D is wrong                 Fix V first
      (true world model failure)    (not a D problem)
```

**Practical Protocol:**
1. For transparent/reflective objects: Prefer DKT-style preprocessing (if available) and validate depth quality
2. For stereo setups: Verify calibration before PID analysis
3. Log V quality metrics alongside PID measurements
4. If V quality degrades, discount PID findings

## 10.8 Headless Gazebo + Tauri Visualization System

### 10.8.1 Architecture Overview

A low-latency simulation and visualization system optimized for robotics research:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              TAURI APP                                   │
├─────────────────────────────────────────────────────────────────────────┤
│                      SparkJS / Three.js                                  │
│                   (WebGPU → Metal or Vulkan)                            │
├─────────────────────────────────────────────────────────────────────────┤
│                         Rust Backend                                     │
│  ┌─────────────────┬─────────────────┬─────────────────┐                │
│  │  Zenoh Client   │  ML Inference   │  Platform Utils │                │
│  │  (cross-plat)   │  (abstracted)   │                 │                │
│  └────────┬────────┴────────┬────────┴─────────────────┘                │
│           │                 │                                            │
│     ┌─────▼─────┐     ┌─────▼─────┐                                     │
│     │  macOS    │     │  Linux    │                                     │
│     │ Backend   │     │ Backend   │                                     │
│     │• CoreML   │     │• CUDA     │                                     │
│     │• MLX      │     │• TensorRT │                                     │
│     │• Metal    │     │• cuDNN    │                                     │
│     └───────────┘     └───────────┘                                     │
└─────────────────────────────────────────────────────────────────────────┘
```

### 10.8.2 Latency Path

**Note:** The numbers below are *illustrative budgets* (order-of-magnitude). Measure end-to-end latency on your hardware/configuration before making any real-time claims.

```
Gazebo (headless)                    Tauri + SparkJS
─────────────────                    ────────────────

                      Zenoh
Physics     ─────────(measure)──────→ State update ──→ Three.js
step rate (configurable)        shared mem/zero-copy (config-dependent)      render @ interactive rate

Camera      ─────────(measure)──────→ Texture update ─→ Three.js
frame rate (configurable)        zero-copy (config-dependent)                plane/quad

Sensors     ─────────(measure)──────→ Process ──→ Overlay ──→ render

Total input lag (report measured): data path + render path + OS scheduling jitter
```

### 10.8.3 Why This Architecture for PID-VLA

| Benefit | Explanation |
|---------|-------------|
| **Target low-latency UI loop (goal; measure)** | Enables interactive debugging of VLA decisions |
| **Zenoh middleware (optional)** | Pub/sub transport; can bridge to ROS 2; shared-memory/zero-copy behavior is configuration-dependent |
| **SparkJS for 3DGS (or equivalent)** | Renders Gaussian splats via WebGPU |
| **Platform abstraction (goal)** | Keep ML inference pluggable (macOS/Metal vs Linux/CUDA); do not assume identical throughput |
| **Headless sim option** | Decouple physics stepping from rendering; actual Hz depends on world complexity |
| **Three.js flexibility** | Overlay PID diagnostics, trajectories, flow vectors, and mesh/URDF debug geometry |

### 10.8.4 Integration with PID Monitoring

```rust
// Rust backend receives VLA embeddings via Zenoh
async fn pid_monitor_loop(zenoh_session: &Session) {
    let subscriber = zenoh_session
        .declare_subscriber("vla/embeddings")
        .await
        .unwrap();
    
    while let Ok(sample) = subscriber.recv_async().await {
        // Decode embeddings (zero-copy from shared memory)
        let embeddings: VLAEmbeddings = deserialize(&sample.payload);
        
        // Compute fast Shannon invariants (fastest; runtime depends on n,d and kNN backend)
        let ci = co_information_pairwise(
            &embeddings.vision,
            &embeddings.dream,
            &embeddings.action,
        );
        
        // Publish to visualization
        zenoh_session
            .put("pid/co_information", serialize(&ci))
            .await
            .unwrap();
    }
}
```

### 10.8.5 Visualization Overlays

The Tauri + Three.js frontend can overlay:

1. **Real-time PID metrics** (co-information, synergy estimates)
2. **Attention heatmaps** from VLA transformer layers
3. **Depth estimation** (a monocular depth model or stereo disparity)
4. **3DGS point clouds** rendered via SparkJS
5. **Action trajectory predictions** from world model

### 10.8.6 Hardware Requirements

Hardware requirements are benchmark-dependent and should not be stated as fixed “minimum/recommended” tables in this document. Measure end-to-end latency and peak memory for your exact stack (scene size, physics backend, rendering settings, and any external video/flow models).

### 10.8.7 PixelVLA Integration with Headless Gazebo + Tauri

Pixel-level prompting is treated as a **controlled intervention**, not a UI-only feature. Some VLAs support multimodal prompting with visual inputs (e.g., PixelVLA arXiv:2511.01571). For PID‑VLA, the critical requirements are:
- visual prompt actions are routed through the **Agent Bridge** control plane (so they are scriptable and logged),
- prompt semantics are preregistered (is the prompt part of V, or a separate source P?),
- and any model-specific tensor shapes/keys are isolated behind a per‑model adapter (do not bake unverified internals into the simulator spec).

**High-level data flow (conceptual; model-specific shapes/topics vary):**
1. Simulator renders observation(s) and publishes them to the VLA interface.
2. VLA consumes observation + instruction (+ optional visual prompt) and emits action plus any representations chosen for analysis.
3. PID‑core computes diagnostics on preregistered representations (offline-first; in-loop only if measured budgets allow).
4. UI overlays diagnostics and exposes prompt tools; every prompt event is logged with provenance and is replayable.

**Conceptual example: prompt routed through Agent Bridge (so external tools can reproduce it):**
```typescript
await agentBridge.call("prompt.set", {
  kind: "point",
  x: point.x,
  y: point.y,
  tag: "region_of_interest"
});
```

## 10.9 Dream2Flow: Video→Flow Bridge for Manipulation (v7.0)

### 10.9.1 Overview

**Dream2Flow** (Dharmarajan et al. 2025, arXiv:2512.24766) introduces a paradigm that is conceptually relevant to PID-VLA: using **3D object flow** as an intermediate representation to bridge video generation models and robotic manipulation.

**Key idea (paper-motivated; validate on your distribution):** video predictors can often synthesize plausible *object motion* even when robot–object interaction details are wrong. Dream2Flow proposes using **3D object flow** as an explicit intermediate so “world dynamics” and “embodiment/control” can fail separately, which is exactly the separation PID‑VLA needs for hypothesis H7 and confound control (§9.7.7, §14.5.7).

### 10.9.2 Architecture and Pipeline

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     DREAM2FLOW PIPELINE                                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  1. VIDEO GENERATION                                                     │
│     Input: image + task instruction (+ optional depth/metadata)          │
│     Model: video predictor (choice is an experimental variable)          │
│     Output: generated clip(s) + metadata                                 │
│                                                                          │
│  2. 3D FLOW EXTRACTION                                                   │
│     - SAM / open-vocab segmentation → Object masks                       │
│     - Video depth estimation (Depth-Anything/DKT)                        │
│     - 2D point tracking (CoTracker / TAPIR)                             │
│     → Lift 2D tracks to 3D → Object flow trajectories                   │
│                                                                          │
│  3. ROBOT POLICY                                                         │
│     - Trajectory optimization (MPC-style) OR                             │
│     - Reinforcement learning with 3D flow as reward                      │
│     → Executable low-level actions                                       │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 10.9.3 Empirical Findings (How to Use Them Safely)

Dream2Flow reports simulation and real-world results and motivates 3D object flow as a general interface for converting video predictions into manipulation objectives. PID‑VLA does **not** assume Dream2Flow’s measured success rates transfer to other predictors/trackers/robots; instead, it adopts the *stage separation* as an instrumentation requirement (§9.7.7).

**Requirement:** when using Flow‑as‑Bridge (H7), log stage outcomes (S1/S2/S3) and intermediate artifacts, and stratify PID analyses by these labels before interpreting any “world model quality” claims.

### 10.9.4 Relationship to VLA Internal World Models

| Aspect | VLA Internal D | Dream2Flow 3D Flow |
|--------|----------------|-------------------|
| **Source** | Learned within VLA | Extracted from pre-trained video model |
| **Format** | Hidden state (high‑D; model-dependent) | 3D trajectories (explicit geometry) |
| **Training** | End-to-end with policy | Predictor + reconstruction + control pipeline (may include optimization/RL; typically no task-specific demos) |
| **Interpretability** | Low (implicit) | High (visualizable trajectories) |
| **Embodiment** | Tied to specific robot | Agnostic |

**Key comparison:** VLA's "D" (dream/world model state) is an **implicit** representation learned end-to-end. Dream2Flow's 3D object flow is an **explicit** intermediate representation extracted from a separate video model.

### 10.9.5 Implications for PID-VLA

#### Implication 1: Decoupling World Model Quality from Action Execution

Dream2Flow's staged pipeline (video → flow → action) enables attributing failures to specific components. This motivates a similar decomposition for PID analysis:

```
PID on (V, D; A)  ← Conflates world model quality with execution
        vs
PID on (V, D; Flow)  ← Isolates world model contribution
PID on (Flow; A)     ← Isolates execution contribution
```

**Testable hypothesis (H7, §3.6.6):** If object flow can be extracted as an intermediate variable, PID on (V,D;Flow) should correlate with video generation success, while PID on (Flow;A) should correlate with execution success.

#### Implication 2: Video Models as D Proxies

For VLAs without explicit D (e.g., OpenVLA), a pre-trained video model could provide a **reference D**:

1. Condition video model on VLA's current state + instruction
2. Extract predicted object flow as "what the video model thinks should happen"
3. Compare to VLA's implicit predictions via PID

This is related to the GWM comparison approach (§10.3) but uses video-derived flow instead of 3DGS-derived predictions.

#### Implication 3: Flow as Reward Signal for PID-Informed RL

Dream2Flow formulates manipulation as object trajectory tracking and can optimize or learn controllers against a flow‑tracking objective. For PID‑VLA Aim 3 (RL fine-tuning), treat flow‑tracking rewards as a baseline objective; PID‑derived rewards are optional and require careful estimator/gradient surrogates.

```python
# Dream2Flow-style: reward from object flow
r_flow = -||extracted_flow - target_flow||

# PID-informed alternative: reward from synergy
r_pid = alpha * Syn(V, D; A*) - gamma * max(0, -Syn)

# Potential combination
r_combined = beta1 * r_flow + beta2 * r_pid
```

**Caution:** The PID-based reward requires a differentiable surrogate (kNN estimators are not differentiable).

### 10.9.6 When to Consider Dream2Flow-Style Analysis

| Scenario | Recommendation |
|----------|---------------|
| **Core PID validation (Experiments 0-3)** | Do NOT use Dream2Flow; compute PID on VLA latents directly |
| **Failure mode attribution** | Consider Dream2Flow's staged analysis as a template |
| **Embodiment-independent D proxy** | Consider video-derived flow if VLA has no explicit D |
| **RL reward design (Aim 3)** | Consider flow-based reward as a baseline for PID-based reward |
| **Visualization / interpretability** | Use Dream2Flow's 3D flow for intuitive failure visualization |

### 10.9.7 Limitations and Caveats

1. **Video model access/licensing:** some predictors are API-only or have restrictive licenses; open alternatives may differ in quality and failure modes. Treat the predictor choice as an experimental variable and report versions/settings.

2. **Flow extraction fidelity:** The 2D-to-3D lifting depends on depth estimation quality (see DKT discussion in §10.4.3).

3. **Not a PID estimator:** Dream2Flow does not compute information-theoretic quantities. Its relevance is conceptual (embodiment gap, failure taxonomy) rather than methodological.

4. **Object-centric assumption:** Dream2Flow's flow extraction assumes objects can be segmented. This may not apply to deformable/granular materials without extensions.

### 10.9.8 Code and Resources

- **Project website:** https://dream2flow.github.io/
- **Paper:** arXiv:2512.24766 (Dec 2025)
- Code/models: see the project website for current availability.

## 10.10 Unified Architecture: Dream2Flow + WAN + PID + Gaussian Splatting (v6.2)

### 10.10.1 Vision: A Complete PID-Aware Robotics Stack

This section describes an ambitious but tractable integration that combines:
- **Dream2Flow** (§10.9): 3D object flow extraction from video generation
- **WAN** (§10.2): Open-source video/world model (replaces proprietary APIs)
- **Wibral PID** (§2): Partial information decomposition for diagnostics
- **Gaussian Splatting** (§10.5): 3D scene representation
- **SparkJS/Tauri** (§10.8): Browser-based visualization
- **Gazebo** (§10.8): Headless robot simulation

**Key insight:** Dream2Flow’s staged architecture provides natural intervention points for PID analysis. A locally runnable predictor (e.g., Wan, arXiv:2503.20314) can improve reproducibility and artifact logging, but the architecture is intentionally model‑agnostic: treat the predictor as a plug‑in experimental variable.

### 10.10.2 Full Stack Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│              PID-DREAM2FLOW-VIDEO/FLOW-GAUSSIANSPLAT INTEGRATION STACK           │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  STAGE 1: INPUT + VIDEO GENERATION                                               │
│  ─────────────────────────────────                                               │
│  ┌───────────────┐    ┌────────────────────────────────────┐                    │
│  │ Gazebo        │    │       Video Predictor Service*     │                    │
│  │ (RGB-D +      │───▶│  • Image + instruction → clip(s)   │                    │
│  │  task instr)  │    │  • Optional intermediates (if any) │                    │
│  └───────────────┘    │  • Local or API inference          │                    │
│                       └───────────────┬────────────────────┘                    │
│                                       │                                          │
│  STAGE 2: VISION FOUNDATION MODELS    │                                          │
│  ─────────────────────────────────    ▼                                          │
│  ┌────────────────────────────────────────────────────────────────────┐         │
│  │                                                                     │         │
│  │   Segmentation      Point tracking       Depth estimation          │         │
│  │   (e.g., SAM2)      (e.g., CoTracker)    (depth model / DKT opt.)  │         │
│  │   (versions/APIs vary; benchmark VRAM/latency on your hardware)    │         │
│  │                                                                     │         │
│  └────────────────────────────────┬───────────────────────────────────┘         │
│                                   │                                              │
│  STAGE 3: 3D FLOW EXTRACTION      ▼                                              │
│  ───────────────────────────────────                                             │
│  ┌────────────────────────────────────────────────────────────────────┐         │
│  │           3D Object Flow Reconstruction                             │         │
│  │                                                                     │         │
│  │   2D tracks + Depth → 3D point trajectories                        │         │
│  │   Output: Per-object flow fields (Gaussian Splat representation)   │         │
│  │                                                                     │         │
│  │   Novel: Represent flows as animated Gaussian Splats where:        │         │
│  │   • Splat position = point in flow trajectory                      │         │
│  │   • Splat color = PID signature (RGB = Syn, Red, Unq)             │         │
│  │   • Splat opacity = MI magnitude / confidence                      │         │
│  │   • Splat size = variance / uncertainty                            │         │
│  └────────────────────────────────┬───────────────────────────────────┘         │
│                                   │                                              │
│  STAGE 4: DIAGNOSTICS (RUST)      │                                              │
│  ───────────────────────────      ▼                                              │
│  ┌────────────────────────────────────────────────────────────────────┐         │
│  │                    Wibral PID Analysis Layer                        │         │
│  │                                                                     │         │
│  │   PID Decomposition Points:                                         │         │
│  │                                                                     │         │
│  │   ① Flow quality (preferred):                                      │         │
│  │      quality(Flow_pred, Flow_gt) → predictor correctness            │         │
│  │      (only when Flow_gt exists; no oracle claims otherwise)         │         │
│  │                                                                     │         │
│  │   ② Integration summaries (screen → targeted PID):                  │         │
│  │      CI/PID on (V, L, D_vla, Flow_pred; A_cmd or A*)                │         │
│  │      "Which sources are used, redundantly or synergistically?"      │         │
│  │                                                                     │         │
│  │   ③ Execution outcome (not PID by default):                         │         │
│  │      success/failure labels + state deltas + contact events         │         │
│  │      "Did the environment/state change as intended?"                │         │
│  │                                                                     │         │
│  │   ④ Temporal diagnostics:                                           │         │
│  │      synergy/CI half-life, phase-wise summaries, bootstrapped CIs   │         │
│  │      (only after Exp0 + geometry gates for the chosen variables)    │         │
│  │                                                                     │         │
│  └────────────────────────────────┬───────────────────────────────────┘         │
│                                   │                                              │
│  STAGE 5: VISUALIZATION           ▼                                              │
│  ──────────────────────────────────                                              │
│  ┌────────────────────────────────────────────────────────────────────┐         │
│  │              Tauri + SparkJS + Three.js Visualization               │         │
│  │                                                                     │         │
│  │   ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐   │         │
│  │   │ Gaussian Splat  │  │ 3D Object Flow  │  │ PID Diagnostic  │   │         │
│  │   │ Scene View      │  │ Animation       │  │ Overlays        │   │         │
│  │   │ (SparkJS/WebGPU)│  │ (trajectories)  │  │ (synergy maps)  │   │         │
│  │   └─────────────────┘  └─────────────────┘  └─────────────────┘   │         │
│  │                                                                     │         │
│  │   Interactive Features:                                             │         │
│  │   • Scrub through flow timeline                                     │         │
│  │   • Click points to see local PID values                           │         │
│  │   • Compare WAN-generated vs VLA-generated flows                    │         │
│  │   • Overlay failure mode annotations                                │         │
│  │                                                                     │         │
│  └────────────────────────────────┬───────────────────────────────────┘         │
│                                   │                                              │
│  STAGE 6: ROBOT EXECUTION         ▼                                              │
│  ─────────────────────────────────                                               │
│  ┌────────────────────────────────────────────────────────────────────┐         │
│  │                 Gazebo Simulation / Real Robot                      │         │
│  │                                                                     │         │
│  │   • Trajectory optimization (MPC) on 3D flow targets               │         │
│  │   • RL with flow-derived + PID-derived rewards                     │         │
│  │   • Closed-loop execution with PID monitoring                      │         │
│  │                                                                     │         │
│  └────────────────────────────────────────────────────────────────────┘         │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 10.10.3 Why Prefer Local/Open Predictors Over API‑Only Services

Dream2Flow can be run with either API‑hosted video predictors or locally runnable/open‑weights predictors. For PID‑VLA, the core scientific need is **reproducibility + artifact logging**, not allegiance to a specific vendor/model.

| Dimension | API‑only services | Local/open predictors (when available) |
|----------|--------------------|----------------------------------------|
| **Reproducibility** | Versioning can change; seeds/metadata may be limited | You can pin weights/commits and record full settings |
| **Artifact logging** | May restrict intermediate outputs | Full control over clips, seeds, failure cases, caching |
| **Cost/latency** | Variable; depends on provider/network | Variable; depends on hardware/model |
| **“Access to D”** | Usually unavailable | Sometimes possible *if* the implementation exposes intermediates (optional; not required for Flow‑as‑Bridge) |

**PID note:** Flow‑as‑Bridge does not require hidden states from the video predictor. If intermediates are exposed, you may treat them as a candidate external “D” for exploratory comparisons, but all such analyses must still pass the Experiment 0 + geometry gates and avoid oracle framing (§9.7.7, §16.11).

### 10.10.4 Gaussian Splatting as PID Representation

**Novel visualization concept:** Represent 3D object flows as animated Gaussian splats where visual properties encode PID quantities:

```
GAUSSIAN SPLAT ↔ PID MAPPING
════════════════════════════

Splat Property          PID Quantity              Interpretation
────────────────────────────────────────────────────────────────────
Position (x,y,z)        Flow trajectory point     Where in 3D space
Color (RGB)             (Syn⁺, Red, Unq(V))      Information structure
  R channel             Synergy (clipped)         Joint integration signal (show negative synergy separately)
  G channel             Redundancy                Overlapping information
  B channel             Unique(V)                 Vision-only contribution (use alternate view for Unq(D))
Opacity (α)             |I(V,D;Flow)|            Total information
Size (σ)                Variance / uncertainty    Estimation confidence

Animation over time:
  • Splat moves along flow trajectory
  • Color changes as PID changes
  • Opacity pulses on significant MI events
  • Size grows when uncertainty increases
```

**Implementation sketch (SparkJS/Three.js):**

```typescript
// PID-colored Gaussian Splat renderer
interface PIDSplat {
  position: [number, number, number];  // 3D flow point
  synergy: number;      // [-1, 1] normalized
  redundancy: number;   // [0, 1] normalized  
  unique_v: number;     // [0, 1] normalized
  mi_magnitude: number; // [0, ∞) → mapped to opacity
  variance: number;     // [0, ∞) → mapped to size
}

function pidToColor(splat: PIDSplat): THREE.Color {
  // Minimal convention: R=Syn⁺, G=Red, B=Unq(V) (align docset color semantics).
  const r = Math.max(0, splat.synergy);
  const g = Math.max(0, splat.redundancy);
  const b = Math.max(0, splat.unique_v);
  return new THREE.Color(r, g, b);
}

function renderPIDFlow(splats: PIDSplat[], timestamp: number) {
  const geometry = new SplatGeometry(splats.length);
  
  for (let i = 0; i < splats.length; i++) {
    geometry.setPosition(i, splats[i].position);
    geometry.setColor(i, pidToColor(splats[i]));
    geometry.setOpacity(i, Math.tanh(splats[i].mi_magnitude));
    geometry.setSize(i, 0.01 + 0.05 * Math.sqrt(splats[i].variance));
  }
  
  // SparkJS handles WebGPU rendering
  sparkRenderer.render(geometry, camera);
}
```

### 10.10.5 PID Analysis Points in the Pipeline

The staged architecture supports **stage-wise diagnostics**, but not all stages should be framed as PID. v7.0 adopts a **no-oracle, contract-first** stance: use explicit quality metrics where possible, and apply PID only to variables that are well-defined for your run and that pass the Geometry Gate + Experiment 0.

**Per-run variable definitions (log them):**
- `Flow_pred`: reconstructed object-level 3D trajectories from a predictor (Dream2Flow-style).
- `Flow_gt`: object-level 3D trajectories from simulator logs (when available).
- `A_cmd`: policy output action command.
- `A*`: teacher/optimal action (when available).
- `D_vla`: chosen VLA internal representation (explicit or hidden; layer choice is part of the experiment).
- `D_pred` (optional): predictor internal representation, *only if* it is actually exposed (rare; do not assume).

**Recommended stage metrics (minimum viable):**
| Stage | Primary metric(s) | What it measures | Notes |
|-------|-------------------|------------------|-------|
| **1. Predictor** | `quality(Flow_pred, Flow_gt)` (if `Flow_gt` exists) | World-model correctness | Use trajectory error metrics + confidence; if no `Flow_gt`, treat as qualitative/heuristic only |
| **2. Flow extraction** | failure rate + confidence distributions | Pipeline health | Track “could not reconstruct”, lost tracks, depth failures; stratify later analysis by these codes |
| **3. Policy integration** | hierarchical CI/PID on `(V, L, D_vla, Flow_pred; A_cmd or A*)` | Integration diagnostics | Apply only after preprocessing + Exp0 gate; start with CI screening, then targeted SxPID |
| **4. Execution** | task success + state deltas | Embodiment/physics/control error | Do not force a PID definition here unless all variables are rigorously defined and justified |

**Optional (exploratory) PID points (only if variables exist):**
- `Syn(V, L; D_pred)` or `Syn(V, D_pred; Flow_pred)` to probe predictor internals **if** exposed.
- `Syn(V, D_vla; Flow_gt)` when you can compute `Flow_gt` from simulator logs (bypasses video predictor geometry entirely).

**Failure localization protocol (v7.0):** do not hard-code synergy thresholds. Instead, learn or calibrate a stage classifier on held-out tasks using the stage metrics above (and report uncertainty).

```python
def localize_failure(trial: Trial, clf) -> FailureStage:
    feats = {
        # Predictor quality (if GT exists)
        "flow_err": flow_error(trial.Flow_pred, trial.Flow_gt) if trial.Flow_gt is not None else None,
        "flow_conf_mean": float(np.mean(trial.Flow_pred.confidence)) if trial.Flow_pred is not None else None,

        # Extraction / reconstruction health
        "recon_failed": int(trial.stage_codes.flow_recon_failed),
        "track_drop_rate": float(trial.stage_codes.track_drop_rate),

        # Integration summaries (screening-friendly)
        "ci_vl_a": trial.CI_VL_A,
        "ci_vflow_a": trial.CI_VFlow_A,
        # ... optionally: selected PID atoms, only if Exp0+geometry gates pass
    }
    return clf.predict(feats)
```

### 10.10.6 WAN Action Conditioning for Counterfactual Analysis

WAN can be made motion/action-conditioned via Wan-Move (arXiv:2512.08765). The exact conditioning mechanism (guidance vs LoRA fine-tuning vs other adapters) is implementation-specific; verify the paper details before treating the conditioning signal as “actions”. This enables **counterfactual PID analysis**:

```
COUNTERFACTUAL ANALYSIS PROTOCOL
════════════════════════════════

1. Generate baseline video:
   V_base = WAN(image, instruction)
   Flow_base = extract_flow(V_base)

2. Generate action-conditioned video:
   V_action = WAN_Move(image, instruction, A_proposed)
   Flow_action = extract_flow(V_action)

3. Compute counterfactual PID:
   ΔSyn = Syn(V, D_wan; Flow_action) - Syn(V, D_wan; Flow_base)

4. Interpretation:
   Treat ΔSyn as a candidate feature, not a ground-truth utility signal:
   - ΔSyn may correlate with better predicted flow quality or downstream success, or it may not.
   - Any use of ΔSyn for “action selection” requires calibration on held-out tasks, uncertainty estimates, and ablations that rule out trivial confounds (e.g., action magnitude).
```

**Use case (optional / exploratory):** before executing a VLA-proposed action, simulate it in a predictor and compute *changes in diagnostics* (ΔCI/ΔPID/Δflow-quality). Do not assume the predictor is an oracle; validate against simulator or sensor-derived trajectories when making “improves/degrades” claims.

### 10.10.7 Computational Requirements Summary

**Per-trial breakdown (benchmark on your hardware; avoid treating any numbers as guarantees):**

| Component | What to measure | Notes |
|-----------|-----------------|------|
| Video generation (local or API) | wall-clock time, peak VRAM | record model name/revision, frames/resolution, prompt, seed |
| Segmentation | time/image, VRAM | often run once per clip; propagate masks if supported |
| Point tracking | time/clip, VRAM | depends on number of points and clip length |
| Depth estimation | time/frame, VRAM | relative vs metric depth; calibrate if absolute scale is required |
| 3D flow reconstruction | time/clip, CPU/GPU | unprojection + filtering; log failures and confidence |
| PID analysis (4 stages) | time/window, memory | depends on `(n,d,k)` and kNN backend; run Exp0 gate first |
| Rendering (optional) | fps, end-to-end latency | depends on scene complexity and GPU |
| **Total per trial** | sum | report separately for online loop vs offline analysis |

**Hardware note (v7.0):** this document does not prescribe a “minimum GPU”. The implemented core (`crates/pid-core`) runs on CPU. Any additional requirements are determined by the specific VLA checkpoint(s), video predictor(s), and scene complexity you choose. Prefer offline-first pipelines and cache artifacts so that interactive diagnostics do not depend on real-time video generation.

### 10.10.8 Comparison: VLA Internal D vs External Predictor‑Derived Flow

This architecture enables direct comparison between VLA internal representations and an external predictor’s reconstructed Flow targets (when available):

```
┌─────────────────────────────────────────────────────────────────────────────┐
│              VLA vs EXTERNAL PREDICTOR (FLOW) COMPARISON                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Same input (V, L)                                                           │
│       │                                                                      │
│       ├──────────────────────┬────────────────────────┐                     │
│       ▼                      ▼                        ▼                     │
│  ┌─────────────┐      ┌─────────────┐         ┌─────────────┐              │
│  │   OpenVLA   │      │  DreamVLA   │         │ Video/Flow  │              │
│  │             │      │             │         │  Predictor  │              │
│  └──────┬──────┘      └──────┬──────┘         └──────┬──────┘              │
│         │                    │                       │                      │
│         ▼                    ▼                       ▼                      │
│  D_openvla (implicit)  D_dreamvla (predicted world knowledge)  Flow_3D (explicit)│
│         │                    │                       │                      │
│         └────────────────────┴───────────────────────┘                      │
│                              │                                               │
│                              ▼                                               │
│                    PID Comparison Matrix:                                    │
│                                                                              │
│  Syn(V, D_openvla; A)  vs  Syn(V, D_dreamvla; A)  vs  Syn(V, Flow; A)       │
│                                                                              │
│  Questions answered:                                                         │
│  • Which “D” proxy is more informative about A under matched controls?      │
│  • Does an explicit flow intermediate provide a cleaner diagnostic target   │
│    than high‑D latent “D” proxies (after geometry + Experiment 0 gates)?    │
│  • Do synergy/CI patterns correlate with task success under preregistered   │
│    confound controls?                                                      │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 10.10.9 Research Payoffs

If this integration succeeds:

1. **Diagnostic power:** Localize VLA failures to specific stages (world model vs action decoder vs embodiment)
2. **Reference predictor:** Use WAN-like video models as a comparison baseline for world prediction (not a ground-truth oracle)
3. **Visualization:** Intuitive 3D visualization of information integration via PID-colored Gaussian splats
4. **Counterfactual analysis:** Test "what if" scenarios with action-/motion-conditioned predictors (when supported)
5. **Reward design:** Use PID + flow as combined reward signal for RL fine-tuning

### 10.10.10 Limitations and Caveats

1. **Computational cost:** Often offline-scale (tens of seconds+ per trial depending on the video model and vision stack). Pre-compute flows for offline analysis and report measured runtimes.

2. **Predictor quality variance:** predictor failure modes vary widely. Validate Flow reconstruction quality and bias before drawing conclusions.

3. **Not a replacement for Experiment 0:** This architecture is post-Experiment-0 work. The core PID estimator must be validated on synthetics first.

4. **Embodiment transfer is empirical:** do not assume PID patterns transfer across embodiments; evaluate under matched task/scene controls.

5. **Action/motion conditioning may require training:** treat conditioning fine-tunes as a separate, data/compute‑heavy project; only pursue if it materially improves the diagnostic question.

### 10.10.11 Implementation Roadmap

| Phase | Components | Timeline | Dependencies |
|-------|------------|----------|--------------|
| **0** | Experiment 0 validation | Month 1-2 | pid-core complete |
| **1** | Video predictor service (local or API) | Month 3 | Predictor available + logging/caching |
| **2** | Vision foundation model integration | Month 3-4 | Segmentation + tracking + depth models (versions vary; benchmark) |
| **3** | 3D flow extraction | Month 4 | Phase 2 complete |
| **4** | PID analysis on flows | Month 4-5 | Phase 3 + pid-core bindings |
| **5** | Gaussian splat visualization | Month 5-6 | SparkJS/Tauri setup |
| **6** | Counterfactual analysis (optional) | Month 6+ | Motion/action conditioning method (verify) |

**Go/No-Go gates:**
- After Phase 0: If Experiment 0 fails, entire integration is blocked
- After Phase 3: If flow extraction quality is poor, fall back to VLA-only PID
- After Phase 4: If PID on flows shows no signal, reconsider integration value

### 10.10.12 Manifold Geometry Considerations (v6.3)

The Dream2Flow + WAN + PID pipeline operates on high-dimensional embeddings where manifold structure matters. This section connects the v5.5/v5.6 manifold challenges to the unified architecture.

#### 10.10.12.1 Geometry at Each Pipeline Stage

| Stage | Representation | Dimension | Geometry Concern | Mitigation |
|-------|---------------|-----------|------------------|------------|
| **WAN hidden states (D_wan)** | Transformer activations | High‑D (model-specific; verify) | May be anisotropic/concentrated/hierarchical | Run §16 diagnostics first; avoid assuming Euclidean validity |
| **3D Object Flow** | Point trajectories | 3×T (Euclidean; can be high-d) | No non-Euclidean metric issue, but high-d concentration/autocorrelation can still bite | Aggregate to low-d object features; run §16 diagnostics + Experiment 0 |
| **VLA embeddings (D_vla)** | Policy/world-model latents | High‑D (model-specific; verify) | High-d anisotropy/concentration and representation-dependent geometry | Apply §16 diagnostics; reduce/quantize as needed |
| **PID-colored splats** | Visualization only | 3D + color | N/A | No estimation, just rendering |

**Key insight:** The 3D object flow target (Stage 3) is **explicitly Euclidean**. When represented as low‑dimensional aggregated object trajectories (rather than a full \(3NT\) tensor), it can reduce reliance on non‑Euclidean latent distances; you still must run geometry diagnostics + Experiment 0 on the chosen flow representation.

#### 10.10.12.2 How v5.6 Manifold Approaches Apply

| v5.6 Approach | Applicable To | Integration Notes |
|---------------|--------------|-------------------|
| **Manifold Unrolling (Isomap/AE)** | D_wan, D_vla | Apply before comparing to Flow |
| **Geodesic MI** | D_wan↔Flow, D_vla↔Flow | Use for MI-only comparisons |
| **Linear Projection (PCA)** | D_wan, D_vla | Test local flatness first (§16.6) |
| **Quantization** | D_wan, D_vla | Maps to discrete clusters; bypasses geometry |
| **Copula Transform** | D_wan, D_vla | Mitigates empty-space at d=4096 |

**Recommended protocol:**
```
GEOMETRY-AWARE DREAM2FLOW-PID PROTOCOL
═══════════════════════════════════════

1. Extract `D_wan` (if exposed) from the predictor run (dimension is model-specific)
2. Extract 3D Flow from vision models (Euclidean representation; may still be high‑D before aggregation)
3. Run geometry diagnostics on D_wan:
   ├── Intrinsic dimension (Levina-Bickel)
   ├── δ-hyperbolicity (Gromov 4-point)
   ├── Local curvature (Ollivier-Ricci)
   └── Decision (no universal thresholds):
       ├── If diagnostics suggest locally flat-ish + not strongly concentrated → PCA/whitening may make continuous SxPID plausible (still requires Experiment 0 on the full pipeline)
       └── Otherwise → MI-only screening / quantization / Flow-as-Bridge (avoid “continuous PID atom” claims on raw embeddings)

4. PID Analysis:
   ├── Syn(V, D_wan_reduced; Flow)  ← D_wan after transform
   ├── Syn(V, Flow; A)               ← Flow is already low-d
   └── Full I^sx_∩ valid for Flow-based analysis
```

#### 10.10.12.3 The Hyperbolic/Lorentzian Connection

Some studies report evidence of **hierarchical/hyperbolic structure** in modern embeddings (§16.7, §16.10); treat this as a measurable property, not a premise:

| Evidence | Source | Implication |
|----------|--------|-------------|
| Some studies report hyperbolic structure in token embeddings | HypLoRA (arXiv:2410.04010) | WAN `D_wan` may have hierarchical geometry |
| Some studies report low δ-hyperbolicity / ultrametric tendencies | arXiv:2512.20926 | Consider hyperbolic/hierarchical diagnostics (verify the exact setup/metric) |
| HELM explores hyperbolic LLMs; improvements are benchmark-dependent | arXiv:2505.24722 | Hyperbolic projection is plausible but must be validated |

**If D_wan is hyperbolic:**
1. **For MI-only screening (CI):** Use geodesic MI estimator (Marx & Fischer 2021)
2. **For full I^sx_∩:** Currently blocked — no hyperbolic I^sx_∩ exists (§16.4.2)
3. **Workaround:** Use Flow as the bridge — it's Euclidean and connects V to A

**The Flow-as-bridge insight:**
```
                 EUCLIDEAN          HYPERBOLIC?         EUCLIDEAN
                 (3D points)        (4096d manifold)    (3D points)
                     │                    │                  │
    V ───────────────┼────→ D_wan ───────┼────→ Flow ───────┼────→ A
    (image)          │    (WAN hidden)   │  (3D trajectory) │   (robot)
                     │                    │                  │
                     └────────────────────┴──────────────────┘
                              Hyperbolic issues
                              contained to D_wan
                              
    By computing PID on (V, Flow; A) instead of (V, D_wan; A),
    we bypass the hyperbolic geometry challenge entirely.
```

#### 10.10.12.4 Hierarchical PID and 3-Source Scaling

The Dream2Flow pipeline enables **natural 3-source decompositions** that relate to §16 hierarchy discussions:

**3-Source candidates:**
- `(V, D_wan, Flow; A)` — Vision + WAN world model + explicit flow → Action
- `(V_coarse, V_fine, Flow; A)` — Multiscale vision + flow (PixelVLA-style)
- `(V, Flow_object, Flow_robot; A)` — Separate object vs robot motion

**Hierarchy implications:**
1. **Level 1 screening:** Compute CI on all pairs — O(n²) per pair, fast
2. **Level 2 targeted PID:** Full I^sx_∩ on suspicious pairs
3. **Level 3 full 3-way:** Only if 2-way shows interesting patterns

**Connection to §16.8 SAE analysis:**
- Apply SAE to D_wan before PID
- Reduce to interpretable features (e.g., 64 SAE latents)
- Compute PID on (V, SAE_latents; Flow) — lower dimension, more interpretable

### 10.10.13 VLA Integration Matrix (v7.0)

This section maps how each VLA architecture integrates with the Dream2Flow + WAN + PID pipeline. v7.0 uses a **contract-first** framing: avoid assuming internal module names or fixed tensor shapes unless you have verified them for your exact checkpoint.

#### 10.10.13.1 VLA ↔ PID Variable Contract (model-agnostic)

For every run, define and log the analysis variables explicitly:
- `V`: a **pre-fusion** vision representation (e.g., pooled vision tokens or a chosen vision-layer summary).
- `L`: a **text/instruction** representation (token pool or selected layer summary).
- `D`: a **world/plan** representation. This can be:
  - **Explicit** (`D_explicit`): predicted world-knowledge channels/cues, if the model exposes them (preferred when available).
  - **Implicit** (`D_hidden[k]`): selected hidden state(s) of the policy backbone (treat layer choice as an experimental variable).
  - **Fused** (`D_fused`): a post-fusion representation that mixes vision + language.
- `A`: the action output (continuous or discrete; representation is model-specific).

**Hard requirement:** the harness must log `model_id`, revision/commit hash, preprocessing, seed(s), layer IDs for any extracted representations, and timestamp alignment with simulator state.

#### 10.10.13.2 Candidate VLAs and Operational “D” Definitions (known vs verify)

| VLA | Primary source | `D` for PID (candidate) | Best-fit hypotheses / decompositions | Verify before using |
|-----|----------------|-------------------------|--------------------------------------|---------------------|
| **OpenVLA** | arXiv:2406.09246 | `D_hidden[k]` (selected backbone state) or `D_fused` | Baseline 2-way: `(V,D;A)`; Flow-as-bridge: `(V,D;Flow)` and `(V,Flow;A)`; optional 3-way/hierarchical with `Flow` | Action parameterization; where to hook `V/L/D`; geometry gate results for chosen reps |
| **TraceVLA** | arXiv:2412.10345 | `D_hidden[k]` plus an explicit **trace input** channel (treat as an additional source) | Temporal hypotheses: H5-style “synergy half-life”; 3-way candidates: `(V_now, V_trace, Flow;A)` or `(V_with_trace, D_wan;Flow_future)` | How traces are encoded; how to separate “trace vs image” in logged variables |
| **DreamVLA** | arXiv:2507.04447 | `D_explicit` (world-knowledge forecasting outputs) if exposed; else `D_hidden[k]` | Cleanest for ablations: compare `(V, D_explicit; Flow)` against `(V, D_wan; Flow)`; channel-wise ablations for dynamic/spatial/semantic cues | Exact output formats/dims; weights/code availability; whether explicit channels are logged per-step |
| **PixelVLA** | arXiv:2511.01571 | Model-dependent: pixel-/prompt-conditioned representation (verify) | Spatial PID when pixel-aligned variables exist: `(V_region, Flow_region;A)`; hierarchical: region-level CI screening → targeted PID | What is actually exposed (pixel maps vs pooled); backbone/API; dataset access (Pixel‑160K) |
| **SmolVLA** | LeRobot (verify) | Whatever is accessible (`D_hidden[k]` / `D_fused`) | Harness/debug baseline: run Experiment 0/1 quickly; do not over-interpret cross-model semantics | Everything: backbone, action rep, licensing, async semantics, hook points |

**Geometry note:** do not assume “d=4096” or “RoPE entanglement” for every model. If you use a RoPE-based backbone (e.g., Llama-family), consider exporting a **pre-attention residual stream** representation as one candidate `D_hidden[k]`, but treat this as an empirical mitigation and validate via the Geometry Gate + Experiment 0.

#### 10.10.13.3 Decision Matrix: Which VLA for Which Analysis (v7.0)

| Analysis goal | Suggested VLA | Reason (constraint-first) |
|---------------|---------------|---------------------------|
| **Harness + estimator bring-up** | SmolVLA (or any small open baseline) | Faster iteration for logging/interventions/geometry gating before scaling up |
| **Core PID study on a widely used target** | OpenVLA | Most likely to be runnable and comparable; `D` is implicit but extractable |
| **Explicit world-model ablations** | DreamVLA (if available) | Operational `D_explicit` makes interventions and interpretation cleaner |
| **Temporal dynamics / history effects** | TraceVLA | Trace input explicitly manipulates history dependence |
| **Spatial / pixel-aligned diagnostics** | PixelVLA (if integration supports it) | Pixel-level variables enable localized PID/CI analyses |

#### 10.10.13.4 Vision Model Placeholders (v7.0 note; verify)

Earlier drafts referenced specific “v3/3” releases for segmentation/tracking/depth. Treat those as **placeholders** for whichever *current, available* models you can actually run and license. For scientific rigor, log the exact model name, version/commit hash, and license for every run.

| Category | Example choices (non-exhaustive) | Notes |
|----------|----------------------------------|-------|
| Segmentation | SAM2 or equivalent promptable segmenter | Validate mask quality and temporal consistency for your scenes |
| Tracking | CoTracker (or equivalent point tracker) | Log number of points, confidence, and failure modes |
| Depth | Depth-Anything v2 (relative) and/or a metric-depth baseline (e.g., Metric3D), plus RGB-D when available | Calibrate if absolute scale is required; handle transparents separately (e.g., DKT) |

**Performance:** Do not assume fixed ms/frame latencies or “2× speedups”. Benchmark your pipeline on your hardware and report measured ranges.

#### 10.10.13.5 Integration Contract: Tauri ↔ “Dream” Services (Dream2Flow / DreamVLA / Similar)

To keep the diagnostics scientifically interpretable and the system maintainable, treat world models and flow extractors as **external, versioned services**. The Tauri app should orchestrate requests, cache artifacts, and provide synchronized playback/overlays; it should not silently “bake in” a particular video model as ground truth.

**Minimum request payload (inputs):**
- `obs_rgb` (+ optional `obs_depth`), camera intrinsics/extrinsics, timestamp
- `instruction` (text) and task metadata
- optional `state` (robot joints, gripper, object poses if privileged)
- `model_id` / revision + `seed` (for reproducibility)

**Minimum response payload (outputs):**
- optional `video` (frames + metadata) for qualitative inspection
- `flow` in an explicit Euclidean form (prefer **object-level trajectories** or other low‑D summaries over full \(3NT\) tensors)
- optional `world_knowledge` (DreamVLA-style): dynamic-region cue + spatial cue(s) + semantic cue(s), plus any intermediate “world embedding” if exposed
- per-stage confidence/error codes (generation / reconstruction / control), so analysis can stratify by stage outcome

**PID/analysis requirements:**
- Define `D` explicitly for each run (what tensor is “D”? predicted cues, intermediate embedding, hidden state layer, etc.).
- Run geometry diagnostics + Experiment 0 on **the exact variables used** (`V`, `D`, `Flow`, and their joint concatenations after preprocessing).
- Treat generated video/flow as a **predictor output**, not a ground-truth oracle; validate “flow quality” against simulator logs or real sensor-derived trajectories when making claims about world-model correctness.

---

# 11. Technical Implementation

## 11.1 Technology Stack

### 11.1.1 Core Components

| Component | In repo (v7.0) | Status | Notes |
|-----------|-----------------|--------|------|
| Continuous MI + SxPID (2-way/3-way) | `crates/pid-core` | Implemented | KSG MI + continuous `I^sx_∩` (`IsxMethod::EhrlichKsg`) + `pid2`/`pid3` wrappers |
| Geometry gates + screening | `crates/pid-core` | Implemented | Intrinsic dimension, distance concentration, δ/δ_rel; hierarchy helpers |
| Python bindings (PyO3) | `crates/pid-python` → `pid_core_rs` | Implemented (extension crate) | Wheel/publish workflow is not wired yet; treat as local-dev bindings |
| Minimal “Experiment 0” runner | `crates/pid-core/src/bin/exp0.rs` + `just exp0` | Implemented | Smoke/validation subset; expand protocol in `EXPERIMENTS.md` |
| Visualization app | `crates/pid-tauri` | Planned | UI + WebGPU renderer (SparkJS/Three.js or equivalent) |
| Simulator + asset pipeline | `assets/`, `experiments/` | Planned | Currently empty; will be populated by scripts or external datasets (likely gitignored) |
| World-model / video predictor | Out-of-process service | Planned | Versioned external dependency (no oracle framing); logs seeds/artifacts |

### 11.1.2 Python Bindings

The repo ships a PyO3 extension crate (`crates/pid-python`) that builds a Python module named `pid_core_rs` exposing:
- `compute_mi`
- `compute_redundancy` (continuous `I^sx_∩` redundancy for 2 sources)
- `estimate_intrinsic_dimension`
- `estimate_gromov_delta`
- `distance_stats`

Packaging into a pip-installable wheel is planned; treat the snippet below as the target API once installed in your Python environment.

```python
import numpy as np
import pid_core_rs as pid

# Two-source SxPID sketch (V,D -> A). Shapes: (N,dv), (N,dd), (N,da)
I_v_a = pid.compute_mi(V, A, k=3)
I_d_a = pid.compute_mi(D, A, k=3)
I_vd_a = pid.compute_mi(np.concatenate([V, D], axis=1), A, k=3)
R = pid.compute_redundancy(V, D, A, k=3)
syn = I_vd_a - I_v_a - I_d_a + R
```

## 11.2 Project Structure

This section distinguishes the **current repo layout** from the **target layout** described in the broader system spec.

**Current (in this repo, v7.0):**

```
pid_vla/
├── Cargo.toml
├── Cargo.lock
├── crates/
│   ├── pid-core/
│   │   ├── src/
│   │   │   ├── bin/exp0.rs
│   │   │   └── (estimators + diagnostics)
│   │   └── tests/
│   └── pid-python/
│       └── src/lib.rs
├── justfile
├── flake.nix
├── flake.lock
├── pyproject.toml
├── uv.lock
└── *.md (docs)
```

**Repo status (v7.0):**
- Implemented: `crates/pid-core` (KSG MI, continuous `I^sx_∩` via `IsxMethod::EhrlichKsg`, 2-way and 3-way wrappers, preprocessing hooks, intrinsic-dimension diagnostics, geometry diagnostics, distance concentration, and a Rust `exp0` runner) and `crates/pid-python` (PyO3 bindings).
- Planned: `crates/pid-tauri` (visualization app) and a Python experiment harness (location TBD: `python/` or `experiments/`), plus asset/tooling scripts to populate `assets/` and local datasets.

## 11.3 Reproducibility

**Canonical (repo truth):** `flake.nix`, `flake.lock`, `pyproject.toml`, and `uv.lock` at the repo root.

If the examples below diverge from the repo files, **prefer the repo files**. (This document is a spec; the repo is the executable artifact.)

### 11.3.1 Nix Flake

```nix
{
  description = "pid_vla (macOS-first): reproducible dev shell for Rust + Python (uv)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            just

            # Rust toolchain (pin via flake.lock).
            rustc
            cargo
            rustfmt
            clippy

            # Python + uv (pin via flake.lock; pin deps via uv.lock).
            python311
            uv
          ];

          # Prefer a system/Nix-provided Python; do not auto-download Pythons.
          UV_NO_MANAGED_PYTHON = "1";
          UV_PYTHON_DOWNLOADS = "never";
        };
      }
    );
}
```

**Lockfile requirement:** commit `flake.lock` (generate/update with `nix flake lock`).

### 11.3.2 uv for Python

```toml
[project]
name = "pid-vla"
version = "0.1.0"
description = "Wibral-group shared-exclusions PID (I^sx_∩) for VLA diagnostics"
readme = "README.md"
requires-python = ">=3.11"

# Keep base dependencies minimal; add groups as needed.
dependencies = []

[dependency-groups]
dev = ["pytest>=8.0", "ruff>=0.6"]
analysis = ["numpy>=1.26", "scipy>=1.11", "pandas>=2.2", "matplotlib>=3.8", "seaborn>=0.13"]
report = ["reportlab>=4.0"]

[tool.uv]
default-groups = ["dev", "analysis"]
```

**Lockfile requirement:** commit `uv.lock` and use `uv sync --frozen` for deterministic installs.

---

## 11.4 Agent Bridge (Automation + Live Intervention)

The PID‑Splat simulator is intended to be **interactive** (GUI) *and* **programmable** (automation). A key design requirement is that the GUI is not a dead end: every operation that matters for experiments (scene edits, perturbations, run control, data export, replay) must be callable through a stable interface that works well with **Claude Code / Codex / opencode‑style tooling**.

**Design goals:**
- **Single source of truth:** the UI uses the same API as external tools (no “hidden clicks” that break reproducibility).
- **Live intervention:** apply perturbations while the sim is running (with bounded latency and backpressure); heavy computations remain offline-first.
- **Auditability:** every remote action is appended to the run log with `actor`, `client_id`, and payload hash; this makes “LLM-in-the-loop” experiments reproducible.
- **Safety:** local-only by default; explicit opt-in for remote access; capability/permission gating for destructive operations.

**Recommended transport (planned):**
- **Control plane:** JSON‑RPC 2.0 over WebSocket on `127.0.0.1` (easy from Rust/TS/Python).
- **LLM tooling:** optional MCP server wrapper that exposes the same methods as tool calls (thin adapter; no separate logic).

**Minimum method surface (sketch; versioned):**
- `sim.start|pause|step|reset|status`
- `scene.load|save|list|add_object|set_transform|set_material|remove_object`
- `camera.get|set|keyframe|render_snapshot`
- `intervention.apply|list|undo|branch` (all interventions are log events)
- `log.start|stop|export|replay`

**Async/concurrency sketch (Rust-side):**
- Keep physics stepping deterministic in a single “sim thread” or single-threaded task; apply interventions only at explicit checkpoints (`pause → apply → step/resume`).
- Run I/O-heavy components (Zenoh pub/sub, JSON‑RPC WebSocket server, file export) on an async runtime and communicate with the sim loop via bounded channels (backpressure instead of unbounded queues).
- Write the run log via an append-only writer task so GUI/LLM actions are recorded even if the UI crashes.

This control plane is an *engineering enabler* for the scientific goals: it makes the preregistered intervention protocol (§9/§14) executable, reviewable, and repeatable.

# 12. Open Questions and Future Directions

## 12.1 Theoretical Open Questions

### Q1: What Does Negative Synergy REALLY Mean for VLAs?

The mathematical definition (subadditive information) doesn't directly map to "hallucination." We need empirical validation that Syn < 0 correlates with human-labeled failures.

**Possible outcomes:**
- Strong correlation → proceed with PID
- Weak correlation → entropy may suffice
- No correlation → abandon PID approach

### Q2: Is the V-D-A Decomposition Fundamental?

Why these three variables? Alternatives:
- Proprioception vs Vision
- Short-term vs Long-term memory
- Task-specific vs General features

### Q3: Can PID Be Used for Training (Not Just Diagnosis)?

Infomorphic networks use PID as a training objective. Can we do the same for VLAs?
- Requires differentiable PID
- May be too noisy for gradient-based optimization
- Aim 3 explores this

## 12.2 Empirical Open Questions

### Q4: At What Scale Does PID Work?

The estimator is validated at ~100 dimensions. VLAs use 4096+. What's the practical limit?

### Q5: How Much Data is Needed?

PID estimation requires many samples. Is a single trajectory enough? Do we need thousands of rollouts?

### Q6: Is Real-Time PID Feasible?

For live intervention, diagnostics must be low-latency and stable. Is this achievable on the target hardware and scene/model sizes, or should “real-time” be restricted to CI/Ω screening with offline PID?

## 12.3 Future Directions

### Direction 1: Multi-Task PID Profiles

Characterize PID signatures across tasks:
- Do some tasks naturally have higher synergy?
- Can PID predict which tasks a VLA will struggle with?

### Direction 2: PID-Guided Data Collection

Use PID to identify high-synergy demonstrations for training data augmentation.

### Direction 3: Hierarchical PID

Apply PID at multiple levels:
- Token-level
- Timestep-level
- Trajectory-level
- Task-level

### Direction 4: Cross-Architecture Transfer

Can PID profiles predict how well a policy will transfer across:
- Embodiments
- Environments
- Task distributions

---

# 13. References

## 13.1 Core Wibral Group PID Work

### 13.1.1 Papers

- **Makkeh A, Gutknecht AJ, Wibral M (2021).** Introducing a differentiable measure of pointwise shared information. *Phys Rev E* 103:032149. DOI: `10.1103/PhysRevE.103.032149`. [Defines I^sx_∩]

- **Ehrlich DA, Schick-Poland K, Makkeh A, Lanfermann F, Wollstadt P, Wibral M (2024).** Partial Information Decomposition for Continuous Variables based on Shared Exclusions. *Phys Rev E* 110:014115. DOI: `10.1103/PhysRevE.110.014115`. [Continuous extension]

- **Makkeh A, Graetz M, Schneider AC, Ehrlich DA, Priesemann V, Wibral M (2025).** A General Framework for Interpretable Neural Learning based on Local Information-Theoretic Goal Functions. *PNAS* 122:e2408125122. DOI: `10.1073/pnas.2408125122`. [Infomorphic networks]

- **Gutknecht AJ, Rosas FE, Ehrlich DA, Makkeh A, Mediano PAM, Wibral M (2025).** Shannon Invariants: A Scalable Approach to Information Decomposition. arXiv:2504.15779. [Scalability]

- **Matthias PH, Makkeh A, Wibral M, Gutknecht AJ (2025).** Novel Inconsistency Results for Partial Information Decomposition. arXiv:2512.16662. [Impossibility theorems]

### 13.1.2 Authoritative Code Repositories (v5.7)

| Repository | Description | License | Status |
|------------|-------------|---------|--------|
| **[continuouspidestimator](https://gitlab.gwdg.de/wibral/continuouspidestimator)** (`csxpid`) | Reference implementation of continuous `I^sx_∩` estimator (Ehrlich et al. 2024) | BSD-3 | ✓ Canonical reference |
| **[infomorphic_networks](https://gitlab.gwdg.de/wibral/infomorphic_networks)** | Experiments with infomorphic networks; learning rule code in "PIDnets" repo (Abed) | GPL-3.0+ | Research code |
| **[SxPID](https://github.com/Abzinger/SxPID)** | Discrete `I^sx_∩` reference implementation | — | ✓ Canonical reference |
| **[sae_analysis](https://github.com/Abzinger/sae_analysis)** | Shannon invariants for SAE latents (Red°, Vul°) | — | Experimental |

**Note:** `infomorphic_networks` delegates core learning rules to "PIDnets" (Abed's repository). Use `continuouspidestimator` for validating continuous `I^sx_∩` estimates.

## 13.2 VLA Models

- **GalaxeaVLA:** Open-source VLA model and platform. [GitHub](https://github.com/OpenGalaxea/GalaxeaVLA)
- **π* (Pi-star) 0.6:** Foundation model for general-purpose robotics. [Pi Blog](https://www.pi.website/blog/pistar06)
- **OpenVLA:** Kim et al. (2024). *OpenVLA: An Open-Source Vision-Language-Action Model.* arXiv:2406.09246.
- **DreamVLA:** Zhang et al. (2025). *DreamVLA: A Vision-Language-Action Model Dreamed with Comprehensive World Knowledge.* arXiv:2507.04447. (World-knowledge forecasting + inverse dynamics; diffusion-style framing in the abstract.)
- **Dream-VL & Dream-VLA (diffusion LLM backbone):** Ye et al. (2025). *Dream-VL & Dream-VLA: Open Vision-Language and Vision-Language-Action Models with Diffusion Language Model Backbone.* arXiv:2512.22615.
  - **Legacy note:** earlier drafts referenced “HKU NLP (2024), 97.2% LIBERO” without a stable citation; treat any such performance claims as unverified unless traced to a specific paper/benchmark protocol.
- **OpenVLA-OFT:** (Unverified label in earlier drafts; likely a fine-tuning / decoding variant; add a concrete citation before treating as a distinct model family.)
- **GR00T N1:** NVIDIA et al. (2025). arXiv:2503.14734.
- **PixelVLA:** Liang et al. (2025). *PixelVLA: Advancing Pixel-level Understanding in Vision-Language-Action Model.* arXiv:2511.01571. Pixel-level understanding with multiscale encoder and visual prompting.
- **TraceVLA:** Zheng et al. (2024). *TraceVLA: Visual Trace Prompting Enhances Spatial-Temporal Awareness for Generalist Robotic Policies.* arXiv:2412.10345. Visual trace prompting for spatial-temporal awareness.
- **MemoryVLA:** Shi et al. (2025). arXiv:2508.19236. Perceptual-cognitive memory for long-horizon manipulation.
- **CoT-VLA:** Zhao et al. (2025). arXiv:2503.22020. Visual chain-of-thought reasoning for VLA.
- **Related (VLM reasoning; optional background for "L"/reasoning traces):** Deng et al. (2025). *OpenVLThinker: Complex Vision-Language Reasoning via Iterative SFT-RL Cycles.* arXiv:2503.17352. (Not a VLA policy paper per se, but relevant to how RL fine-tuning affects visual grounding and intermediate reasoning traces.)
- **GenieReasoner/FACT:** Liu et al. (2025). *Unified Embodied VLM Reasoning with Robotic Action via Autoregressive Discretized Pre-training.* arXiv:2512.24125. [FACT tokenizer: flow-matching action discretization; ERIQ benchmark for embodied reasoning]

## 13.2.1 VLA Benchmarks and Evaluation (v5.8)

### VLA-Arena (Primary Recommended Benchmark for PID-VLA)

**Citation:** Zhang et al. (2025). *VLA-Arena: An Open-Source Framework for Benchmarking Vision-Language-Action Models.* arXiv:2512.22539.

**Why VLA-Arena is recommended for PID experiments:**

| Property | VLA-Arena Value | PID-VLA Benefit |
|----------|-----------------|-----------------|
| **Structured difficulty** | L0/L1/L2 levels | Enables memorization vs generalization testing (H4) |
| **Orthogonal perturbation axes** | V0-V4 (visual), W0-W4 (language) | Decoupled testing of V and L integration robustness |
| **Task dimensions** | Safety, Distractor, Extrapolation, Long-Horizon | Maps directly to PID hypotheses (H4, H5, H6) |
| **Scale** | 170 tasks | Sufficient statistical power for PID signature analysis |
| **Open-source** | Full toolchain provided | Reproducibility and community adoption |

**VLA-Arena Task Structure (Detailed):**

```
VLA-Arena Task Organization:
├── Safety (collision avoidance, constraint satisfaction)
│   ├── L0: Training distribution
│   ├── L1: Mild generalization
│   └── L2: Strong generalization
├── Distractor (irrelevant objects in scene)
│   ├── L0, L1, L2 difficulty levels
│   └── Cross-product with V0-V4, W0-W4
├── Extrapolation (novel configurations)
│   ├── L0, L1, L2 difficulty levels
│   └── Key test for memorization vs generalization
└── Long Horizon (multi-step compositional tasks)
    ├── L0, L1, L2 difficulty levels
    └── Key test for temporal synergy dynamics
```

**Perturbation Protocols (V0-V4, W0-W4):**

| Level | Visual (V) Perturbations | Language (W) Perturbations |
|-------|-------------------------|---------------------------|
| **0** | Clean observation | Original instruction |
| **1** | Minor noise/lighting | Synonym substitution |
| **2** | Moderate occlusion | Paraphrasing |
| **3** | Significant viewpoint change | Instruction simplification/elaboration |
| **4** | Severe corruption | Ambiguous/underspecified instructions |

*Note: Exact perturbation specifications should be verified against VLA-Arena documentation.*

**VLA-Arena Datasets:**

| Dataset | Description | Recommended PID Use |
|---------|-------------|---------------------|
| **VLA-Arena-S** | Small-scale fine-tuning set | Experiment 0 validation, rapid prototyping |
| **VLA-Arena-M** | Medium-scale fine-tuning set | Primary experiments (Experiments 1-4) |
| **VLA-Arena-L** | Large-scale fine-tuning set | Full-scale validation, publication-ready results |

**Key Findings Relevant to PID-VLA:**

1. **"Memorization over generalization"**: VLAs show strong tendency to memorize training tasks rather than learning generalizable skills. This motivates H4 (§3.6.2).

2. **Asymmetric robustness**: V-perturbations and W-perturbations affect VLA performance differently. This suggests modality-specific integration weaknesses detectable by PID.

3. **Compositional failure on long-horizon tasks**: VLAs cannot compose learned skills. This motivates temporal synergy analysis (H5, §3.6.3).

4. **Safety constraint ignorance**: VLAs often fail to consider safety constraints. This motivates H6 (§3.6.4).

**Resources:**
- Website: https://vla-arena.github.io
- Leaderboard: Available at project website
- Code: Full end-to-end toolchain from task definition to automated evaluation

### Other VLA Benchmarks

- **ERIQ:** Liu et al. (2025). Embodied Reasoning Intelligence Quotient benchmark, 6000+ QA pairs. (Part of GenieReasoner work, arXiv:2512.24125). Useful for reasoning-focused VLA evaluation but less structured for PID analysis than VLA-Arena.

- **SimplerEnv:** Lightweight simulation benchmark. Useful for rapid iteration but lacks VLA-Arena's structured difficulty axes.

- **LIBERO:** Standard manipulation benchmark. Well-established but doesn't provide the perturbation structure needed for robustness testing.

**Benchmark Selection Guidance:**

| Research Question | Recommended Benchmark | Rationale |
|-------------------|----------------------|-----------|
| Memorization vs generalization | VLA-Arena | L0/L1/L2 structure directly tests this |
| V-L integration robustness | VLA-Arena | V0-V4 / W0-W4 perturbation axes |
| Temporal synergy dynamics | VLA-Arena (Long Horizon) | Multi-step tasks with clear phase structure |
| Rapid PID validation | SimplerEnv or VLA-Arena-S | Lower computational cost |
| Publication-ready results | VLA-Arena-M/L | Community standard, reproducible |

## 13.3 Multimodal PID

- **Liang PP, Cheng Y, Fan X, Ling CK, et al. (2023).** Quantifying & Modeling Multimodal Interactions: An Information Decomposition Framework. NeurIPS 2023. [Uses BATCH/CVX estimators, NOT I^sx_∩. Code: github.com/pliang279/PID]

- **IDTxl:** Wollstadt P, Lizier JT, et al. (2019). IDTxl: The Information Dynamics Toolkit xl. JOSS 4(34):1081. [Comprehensive PID toolkit. Code: github.com/pwollstadt/IDTxl]

- **SxPID:** Discrete `I^sx_∩` reference implementation (Python). [Code: https://github.com/Abzinger/SxPID]

- **sae_analysis:** WIP toolbox for Shannon-invariants-style analysis of SAE latents (degree of redundancy / vulnerability from Gutknecht et al. 2025). [Code: https://github.com/Abzinger/sae_analysis; experimental/not yet fully validated]

## 13.4 World Models

- **Dream2Flow:** Dharmarajan et al. (2025). *Dream2Flow: Bridging Video Generation and Open-World Manipulation with 3D Object Flow.* arXiv:2512.24766. [3D object flow as intermediate representation; embodiment-agnostic; zero-shot video-to-action. Website: https://dream2flow.github.io/]
- **GWM:** Gaussian World Model (3DGS + diffusion for robotics; verify venue/status).
- **Physically Embodied Gaussian Splatting:** (paper; verify venue/status). Real-time correctable world model.
- **WAN:** Wanxiang Video Model, Alibaba 2025. arXiv:2503.20314
- **WAN VACE:** Video All-in-one Creation and Editing. arXiv:2503.07598
- **Wan-Move:** Motion-controllable Video Generation. arXiv:2512.08765 (verify venue/status)
- **Motus:** Unified Latent Action World Model. arXiv:2512.13030
- **DreamGen:** Robot Learning via Neural Trajectories. arXiv:2505.12705
- **VideoVLA:** VideoVLA: Video Generators Can Be Generalizable Robot Manipulators. arXiv:2512.06963
- **Scalable Policy Evaluation:** Action-conditioned video for policy eval. arXiv:2511.11520
- **Genie 3:** DeepMind blog/release (verify details; listed as background inspiration, not a dependency of this project).
- **Genie 2:** DeepMind blog/release (verify details; background only).
- **Genie 1:** Bruce et al. (Feb 2024). Generative Interactive Environments. arXiv:2402.15391
- **SIMA 2:** DeepMind (Nov 2025). Gemini-powered generalist agent for 3D virtual worlds. arXiv:2512.04797
- **Diffusion modeling note (background; optional):** Li & He (2025). *Back to Basics: Let Denoising Generative Models Denoise.* arXiv:2511.13720. (Relevant to how “denoising” vs “noise prediction” parameterizations can change representation geometry; treat as background for diffusion-based world models, not a PID paper.)

## 13.5 Uncertainty & Hallucination Detection

- **VL-Uncertainty:** Zhang et al. (2024). arXiv:2411.11919
- **SAFE:** Multitask VLA failure detection. arXiv:2506.09937
- **PRE-HAL:** Dempster-Shafer for VLM hallucination

## 13.6 Process Reward Models (PRMs)

- **Robo-Dopamine:** Tan et al. (2025). arXiv:2512.23703. [GRM for step-aware progress rewards]
- **GVL:** Vision-language in-context value learners. Ma et al. (2024). [Progress prediction]
- **VLAC:** Vision-language action critic. Zhai et al. (2025). arXiv:2509.15937
- **SARM:** Stage-aware reward modeling. Chen et al. (2025). arXiv:2509.25358
- **LIV:** Language-image representations for rewards. Ma et al. (2023). ICML

## 13.7 Information Theory

- **KSG Estimator:** Kraskov et al. (2004). *Phys Rev E* 69:066138. DOI: `10.1103/PhysRevE.69.066138`.
- **O-information (Ω; synergy-vs-redundancy bias for a set of variables):** introduced by Rosas et al. (2019). *(Bibliographic details should be verified; included as optional background, not part of the Wibral-group `I^sx_∩` line.)*
- **kNN MI under strong dependence (limitations + fixes):**
  - Gao, Ver Steeg, Galstyan (2015). *Efficient Estimation of Mutual Information for Strongly Dependent Variables.* arXiv:1411.2003.
  - Gao, Ver Steeg, Galstyan (2015). *Estimating Mutual Information by Local Gaussian Approximation.* arXiv:1508.00536.
- **Neural / classifier-based MI estimation (baselines for MI/CMI; not `I^sx_∩`):**
  - Belghazi et al. (2018). *MINE: Mutual Information Neural Estimation.* arXiv:1801.04062.
  - Mukherjee, Asnani, Kannan (2019). *CCMI: Classifier based Conditional Mutual Information Estimation.* arXiv:1906.01824.
  - Molavipour, Bassi, Skoglund (2019). *Conditional Mutual Information Neural Estimator.* arXiv:1911.02277.
- **Williams & Beer (2010).** Original PID formulation

## 13.8 Scalable PID Methods

- **Shannon Invariants:** Gutknecht et al. (2025). arXiv:2504.15779. [Scalable summaries]
- **Gaussian PID:** Barrett et al. (2023). NeurIPS. [Bias-corrected high-d estimation]
- **Normalizing-flow PID in latent Gaussian space:** Zhao et al. (2025). arXiv:2510.04417. (Earlier drafts referred to this as “Thin-PID”; the arXiv title is *Partial Information Decomposition via Normalizing Flows in Latent Gaussian Distributions*.)
- **Representational Complexity:** Ehrlich et al. (2022). Trans. ML Res. [Coarse-graining]
- **dit Library:** Python library for discrete information theory (dit.distributions)
- **IDTxl:** Comprehensive information dynamics toolkit (pwollstadt/IDTxl)

## 13.9 Depth Estimation & 3D Perception

- **Depth-Anything v2/v3:** Yang et al. (2024-2025). Monocular depth foundation models.
- **Video Depth Anything:** Temporally consistent video depth estimation.
- **RollingDepth:** Video depth without video models. arXiv:2411.19189. [LDM-based]
- **StereoVLA:** Deng et al. (2025). arXiv:2512.21970. [Stereo vision for VLAs]
- **DKT (Diffusion Knows Transparency):** arXiv:2512.23705. [Transparent object depth via WAN]
- **Metric3D v2:** Absolute depth with metric scale recovery.
- **SHARP:** Single-image to 3DGS conversion.

## 13.10 Simulation & Middleware

- **Gazebo Harmonic:** ROS 2 compatible physics simulator
- **SplatSim:** *SplatSim: Zero-Shot Sim2Real Transfer of RGB Manipulation Policies Using Gaussian Splatting.* arXiv:2409.10161.
- **DISCOVERSE:** *DISCOVERSE: Efficient Robot Simulation in Complex High-Fidelity Environments.* arXiv:2507.21981.
- **Zenoh:** Zero-overhead pub/sub middleware (eclipse-zenoh.io)
- **Tauri:** Rust + WebView desktop apps (tauri.app)
- **SparkJS:** 3DGS rendering in browser via WebGPU
- **Three.js:** WebGL/WebGPU 3D rendering library

## 13.11 Training Infrastructure

- **NanoGPT:** Karpathy. GPT-2 reproduction in ~600 lines. (github.com/karpathy/nanoGPT)
- **nanochat:** Karpathy (2025). Full-stack ChatGPT training, ~$100. (github.com/karpathy/nanochat)
- **llm.c:** C/CUDA LLM training, 7% faster than PyTorch. (github.com/karpathy/llm.c)
- **modded-nanogpt:** Speedrun benchmark for LLM training optimization
- **SRL (step-wise reasoning training; optional):** Deng et al. (2025). *Supervised Reinforcement Learning: From Expert Trajectories to Step-wise Reasoning.* arXiv:2510.25992. (Potentially relevant to Aim 3 / PRM-style training loops; not PID-specific.)

## 13.12 Differential Geometry & Non-Euclidean Representation (Optional)

- Differential-geometry contingency notes are integrated into §8.1.5 (optional background; not a correctness source).
- **Manifold-aware MI estimation:** Marx, Fischer (2021). *Estimating Mutual Information via Geodesic kNN.* arXiv:2110.13883. (Riemannian/geodesic kNN MI; useful as MI-only baseline in curved settings.)
- **Hyperbolic embeddings for hierarchies:**
  - Nickel, Kiela (2017). *Poincaré Embeddings for Learning Hierarchical Representations.* arXiv:1705.08039.
  - Nickel, Kiela (2018). *Learning Continuous Hierarchies in the Lorentz Model of Hyperbolic Geometry.* arXiv:1806.03417.
  - Ganea, Bécigneul, Hofmann (2018). *Hyperbolic Neural Networks.* arXiv:1805.09112.
  - Yang et al. (2022). *Hyperbolic Graph Neural Networks: A Review of Methods and Applications.* arXiv:2202.13852.
- **Hyperbolic LLMs and fine-tuning (v5.7):**
  - **HELM:** First billion-scale hyperbolic LLM. arXiv:2505.24722.
  - **HypLoRA:** Hyperbolic fine-tuning for LLMs; reports evidence of hierarchical/hyperbolic structure in some embedding settings (check metric/setup). arXiv:2410.04010.
  - **Hypformer:** Efficient hyperbolic transformer with linear complexity. arXiv:2407.01290.
  - **Hierarchical Mamba:** Projects Mamba2 representations into Poincaré/Lorentz manifolds. arXiv:2505.18973.
- **Hierarchical structure in LLM embeddings (v5.7):**
  - **δ-hyperbolicity analysis:** arXiv:2512.20926. (Uses δ-hyperbolicity + ultrametricity to compare tree-likeness across embedding spaces; replicate on your embeddings—do not transplant exact values.)
  - **Cognitive state hierarchy:** Zhao (2025). *Hierarchical Geometry of Cognitive States in Transformer Embedding Spaces.* arXiv:2512.22227. [Demonstrates decodable hierarchical structure aligned with cognitive attributes]
- **Intrinsic dimension estimation (geometry diagnostics for kNN validity):**
  - Levina, Bickel (2005). *Maximum likelihood estimation of intrinsic dimension.* (Foundational intrinsic-dimension estimator; use as a diagnostic, not a guarantee.)
  - Gomtsyan et al. (2019). *Geometry-Aware Maximum Likelihood Estimation of Intrinsic Dimension.* arXiv:1904.06151.
- **Lorentzian conformal rigidity (background; mostly analogy-level for this project):**
  - Melnick, Pecastaing (2025). *A local Lorentzian Ferrand-Obata theorem for conformal vector fields.* arXiv:2511.03713.
  - Pecastaing (2019). *The conformal group of a compact simply connected Lorentzian manifold.* arXiv:1911.06251.
  - Frances (2025). *Conformal quotients of plane waves, and Lichnerowicz conjecture in a locally homogeneous setting.* arXiv:2503.08614.

---

# 14. Confounding Factors Analysis: Proving and Disproving the Hypotheses

This section addresses how confounding factors could be studied and removed to rigorously prove or disprove the core hypotheses of PID-VLA. Grant reviewers will scrutinize whether observed correlations reflect genuine causal relationships or are artifacts of confounding variables.

## 14.1 Core Hypotheses and Their Falsifiability

### 14.1.0 Hypothesis Registry (v7.0)

This project treats hypotheses as **falsifiable contracts**, not slogans. Status labels are about *priority and interpretability* given current estimator and logging constraints.

| Hypothesis | Status | Rationale / notes |
|------------|--------|-------------------|
| **H1** PID features ↔ failure labels | **Core** | Primary evaluative claim; must beat strong baselines under controls; synergy sign is a candidate feature, not a definition |
| **H2** redundancy ↔ robustness | Exploratory | Redundancy is easily confounded; only meaningful under matched difficulty + nuisance controls |
| **H3** uniques ↔ modality contribution | Exploratory | Useful for targeted interventions; requires symmetry in preprocessing to avoid estimator artifacts |
| **H4** memorization vs generalization | Core | Motivated by VLA-Arena framing; tests whether PID signatures change under structured distribution shifts |
| **H5** temporal synergy degradation | Core | Operationalizable with windowed summaries + block bootstrap; tests long-horizon composition failures |
| **H6** safety-aware integration | Exploratory | Lower confidence; include only if safety labels and matched controls are available |
| **H7** Flow-as-Bridge | Core (method + hypothesis) | Makes a Euclidean diagnostic target explicit; enables stage-wise attribution and cross-embodiment comparisons |

**Deprecated / ruled-out framing (kept for transparency):**
- “`Syn < 0` ⇒ hallucination” as a definitional claim is **rejected** (see §1.2 Warning 1). Negative synergy is mathematically meaningful (subadditive information) but requires empirical validation to map onto failure semantics.

### Hypothesis H1: SxPID-derived features correlate with VLA failures (including negative synergy)
**Claim (falsifiable):** Under a validated estimator regime, a **feature set** derived from Shannon invariants (CI/Ω) and (where feasible) SxPID atoms contains predictive information about failure labels beyond strong uncertainty baselines. The **synergy sign** (including negative synergy frequency) is one candidate feature, not a definition of “hallucination”.

**Confounds to rule out:**
1. **Task difficulty confound:** Negative synergy might correlate with inherently harder tasks (longer horizons, more object interactions), not with model failure per se.
2. **Distribution shift confound:** Negative synergy might arise when inputs are out-of-distribution, which also causes failures—but the failure is due to OOD inputs, not synergy.
3. **Embedding quality confound:** If embeddings are poorly learned, both synergy estimates and task performance degrade together, creating spurious correlation.

**How to disprove:**
- Control for task difficulty by stratifying experiments (same task family, varying synergy).
- Add explicit OOD detection baselines and test whether synergy provides signal beyond OOD scores.
- Test on multiple VLA architectures; if synergy-failure correlation appears only in one, it may be architecture-specific rather than fundamental.

### Hypothesis H2: High redundancy indicates robust information integration
**Claim:** High `Red_{V,D;A}` suggests the model has multiple pathways to correct action.
**Status:** Exploratory. Redundancy can increase for trivial reasons (task simplicity, dataset artifacts, representation leakage), so H2 is only meaningful under matched controls and alongside generalization tests (H4).

**Confounds:**
1. **Triviality confound:** If the task is trivial (e.g., "do nothing"), all sources may redundantly encode the same null information.
2. **Overfitting confound:** High redundancy in training data might indicate memorization rather than generalization.

### Hypothesis H3: Unique information identifies modality-specific contributions
**Claim:** `Unq_V` vs `Unq_D` vs `Unq_L` indicates which modality dominates decision-making.
**Status:** Exploratory but operationally useful for interventions (e.g., corrupt one modality and observe which uniques move). Interpreting uniques requires careful symmetry in preprocessing and dimensionality reduction across modalities to avoid estimation artifacts.

**Confounds:**
1. **Representation bias:** If one modality has higher-dimensional embeddings, it may have artificially higher unique information due to estimation artifacts.
2. **Preprocessing asymmetry:** Different preprocessing per modality can shift apparent unique contributions.

## 14.2 Experimental Controls for Confound Removal

### 14.2.1 Matched Control Experiments

For every "synergy predicts failure" claim, implement:

```
CONTROL DESIGN MATRIX
=====================

Primary comparison (within-task):
┌──────────────────────────────────────────────────────────────┐
│  Same task template    Same initial state seed               │
│  Same language instruction    Same environment physics       │
│  Different: VLA internal state / D representation            │
│                                                              │
│  Measure: ΔSynergy vs ΔFailure rate                          │
│  Prediction: Correlation should persist after matching       │
└──────────────────────────────────────────────────────────────┘

Task-difficulty control:
- Bin tasks by objective difficulty metrics (horizon length, object count, precision required)
- Test synergy-failure correlation WITHIN each difficulty bin
- If correlation disappears within bins, task difficulty is the true predictor

Distribution-shift control:
- Compute OOD score (e.g., Mahalanobis distance in embedding space, uncertainty calibration)
- Test whether synergy provides INCREMENTAL predictive power beyond OOD score
- Regression: Failure ~ OOD_score + Synergy + OOD_score×Synergy
```

### 14.2.2 Placebo Tests (Sanity Checks)

**Null intervention test:**
- Apply a "placebo" intervention that should NOT change synergy (e.g., add imperceptible noise to V)
- If measured synergy changes significantly, the estimator is sensitive to irrelevant variations

**Permutation test for spurious correlation:**
- Randomly permute trajectory labels within each task family
- Re-compute synergy-failure AUROC
- The permuted AUROC should be ~0.5 (no better than chance)
- If permuted AUROC > 0.55, there is label leakage or confounding

**Temporal shuffling test:**
- Shuffle timesteps within trajectories
- Re-estimate PID terms
- If estimates remain stable despite broken temporal structure, the estimator may not capture meaningful dynamics

### 14.2.3 Causal Identification Strategy

**Instrumental variable approach (if feasible):**
- Find a variable Z that affects D but not A directly (except through D)
- Example: Randomized perturbation to the world model training procedure
- Use Z as an instrument to estimate causal effect of D-quality on synergy

**Regression discontinuity design:**
- If there's a threshold in training (e.g., model checkpoint at step N), test whether synergy changes discontinuously at the threshold
- Sharp changes at arbitrary thresholds suggest overfitting to checkpoint artifacts

## 14.3 Alternative Interpretations of Results

### 14.3.1 If Negative Synergy Does NOT Predict Failure

**Interpretation 1: Synergy is architecture-dependent, not failure-predictive**
- Action: Report as valid negative result; pivot to simpler entropy/confidence baselines

**Interpretation 2: Estimator is broken at VLA scale**
- Action: Verify via Experiment 0; if estimator collapsed, negative result is uninformative

**Interpretation 3: Task distribution lacks sufficient failure diversity**
- Action: Expand benchmark to include more failure modes; re-test

### 14.3.2 If Positive Results Appear

**Alternative explanation 1: Confounding by entropy**
- Test: Include action entropy as covariate; if synergy becomes non-significant, entropy suffices

**Alternative explanation 2: Confounding by model uncertainty**
- Test: Include ensemble variance or explicit uncertainty estimate as covariate

**Alternative explanation 3: P-hacking through feature selection**
- Mitigation: Pre-register primary analysis; report ALL synergy variants tested, not just significant ones

## 14.4 Robustness Checks Required for Publication

| Check | Description | Pass Criterion |
|-------|-------------|----------------|
| **Seed robustness** | Run with 10+ random seeds | Effect size stable (CV < 30%) |
| **K robustness** | Test k ∈ {3, 5, 7, 10} | Direction consistent, magnitude within 2× |
| **Preprocessing robustness** | With/without standardization, jitter | Conclusions unchanged |
| **Dimensionality robustness** | Raw vs PCA-256 vs PCA-64 | At least one regime shows effect |
| **Temporal sampling** | Different stride/window sizes | Effect persists across reasonable ranges |
| **Cross-architecture** | Test on 2+ VLA architectures | Effect appears in majority |
| **Cross-benchmark** | Test on 2+ task distributions | Effect generalizes |

## 14.5 VLA-Arena-Derived Confounds

VLA-Arena (arXiv:2512.22539) provides systematic evidence of VLA behavioral patterns that introduce specific confounds for PID analysis. These must be controlled before attributing PID patterns to "integration quality."

### 14.5.1 Task Difficulty Stratification Confound (L0/L1/L2)

**The confound:** VLA-Arena defines three task structure levels that correlate with both failure rate and expected synergy:

| Level | Structure | Example | Failure Expectation | Synergy Confound |
|-------|-----------|---------|---------------------|------------------|
| **L0** | Single primitive | "Pick up the apple" | Low failure | Simple V-L mapping; synergy naturally low |
| **L1** | Conditioned primitive | "Pick the red object, not the blue" | Medium failure | Disambiguation requires synergy |
| **L2** | Composed actions | "Move X to Y, then Z to W" | High failure | Temporal composition; synergy across time |

**Control protocol:**
```
TASK-LEVEL STRATIFIED ANALYSIS
==============================

1. Stratify all trajectories by L-level (L0, L1, L2)
2. Compute PID terms WITHIN each level
3. Test synergy-failure correlation WITHIN each level separately
4. Only claim "synergy predicts failure" if effect persists within L1 and L2
   (L0 may have floor effects due to low failure rates)

Null hypothesis per level:
- H0(L1): Within L1 tasks, synergy is uncorrelated with failure (ρ = 0)
- H0(L2): Within L2 tasks, synergy is uncorrelated with failure (ρ = 0)

Pre-registered adjustment: Bonferroni correction for 2 independent tests (α = 0.025)
```

**Why this matters:** If PID synergy only predicts failure when comparing L0 vs L2 tasks (trivially different difficulty), the finding is confounded. The PID metric must provide signal **within** difficulty strata to be useful.

### 14.5.2 Perturbation-Induced Distribution Shift Confound

**The confound:** VLA-Arena applies systematic perturbations (W0-W4 for language, V0-V4 for visual) that shift inputs out of distribution. Both synergy and failure may increase due to OOD inputs, not due to integration failure.

**Perturbation taxonomy:**

| Axis | Levels | Examples | OOD Severity |
|------|--------|----------|--------------|
| **Language (W)** | W0-W4 | Original → Synonym → Paraphrase → Typos → Irrelevant | Increasing |
| **Visual (V)** | V0-V4 | Original → Lighting → Texture → Background → Distractor | Increasing |

**Control protocol:**
```
OOD-ADJUSTED SYNERGY ANALYSIS
=============================

Step 1: Quantify distribution shift
- Compute embedding-space OOD score for each (V, L) pair
- Methods: Mahalanobis distance, k-NN density, calibrated uncertainty

Step 2: Regression with OOD control
  Failure ~ OOD_score_V + OOD_score_L + Synergy + interactions

Step 3: Interpretation matrix
┌─────────────────────────────────────────────────────────────────┐
│ If Synergy significant after OOD control: Integration effect   │
│ If Synergy non-significant after OOD control: OOD confound     │
│ If interaction (Synergy × OOD) significant: Context-dependent  │
└─────────────────────────────────────────────────────────────────┘

Step 4: Report incremental R² from synergy beyond OOD baseline
```

**The VLA-Arena finding to verify:** VLAs show asymmetric robustness (more robust to V than L perturbations). PID should detect this as:
- `Unq_L` increases under L perturbation (language becomes less redundantly encoded)
- `Red_{V,L;A}` decreases under L perturbation (integration breaks down)

### 14.5.3 Memorization vs Generalization Confound

**The confound:** VLA-Arena's key finding is "memorization over generalization"—VLAs perform well on training-similar tasks but fail on novel compositions. Synergy patterns may simply reflect memorization confidence rather than integration quality.

**Diagnostic signatures:**

| Behavior | Memorized Response | Generalized Response |
|----------|-------------------|----------------------|
| Synergy pattern | Low variance (stable) | High variance (uncertain) |
| Response to perturbation | Abrupt failure | Graceful degradation |
| PID interpretation | Overfit to training synergy patterns | True integration |

**Control protocol:**
```
MEMORIZATION DETECTION PROTOCOL
===============================

1. Identify "memorization indicators" from VLA-Arena:
   - Task similarity to training set (embedding distance to training tasks)
   - Response stereotype (action sequence similarity to training)
   - Confidence-calibration gap (high confidence on failures)

2. Stratify by memorization score:
   - High memorization: Tasks similar to training, stereotyped responses
   - Low memorization: Novel compositions, variable responses

3. Test PID-failure correlation within each stratum:
   - If effect only in "high memorization": Synergy tracks overfitting
   - If effect only in "low memorization": Synergy tracks true integration
   - If effect in both: Synergy is robust across regimes

4. Report memorization index alongside all PID results
```

**Critical insight:** A VLA that has memorized a perfect V-L-A mapping will show high redundancy and low synergy (the signature of "robust integration"), but this is an artifact of overfitting, not genuine understanding.

### 14.5.4 Asymmetric Modality Robustness Confound

**The confound:** VLA-Arena shows VLAs are more robust to visual perturbations than language perturbations. This asymmetry could create systematic biases in PID decomposition that reflect architecture bias rather than task structure.

**Observed asymmetry (from VLA-Arena):**
```
┌────────────────────────────────────────────────────────────────┐
│              Perturbation Robustness Asymmetry                 │
├────────────────────────────────────────────────────────────────┤
│   Visual perturbations (V1-V4):                                │
│   - VLAs maintain performance longer                           │
│   - Synergy degrades gradually                                 │
│                                                                │
│   Language perturbations (W1-W4):                              │
│   - VLAs fail more abruptly                                    │
│   - Synergy shows discontinuous drops                          │
│                                                                │
│   Implication for PID:                                         │
│   - Unq_V may appear stable (robust pathway)                   │
│   - Unq_L may appear critical (fragile pathway)                │
│   - This is ARCHITECTURE-DRIVEN, not task-driven               │
└────────────────────────────────────────────────────────────────┘
```

**Control protocol:**
1. Normalize PID terms by baseline modality contribution (from unperturbed samples)
2. Report relative change: `ΔUnq_V / Unq_V(baseline)` vs `ΔUnq_L / Unq_L(baseline)`
3. Compare asymmetry across architectures (OpenVLA vs π₀ vs others)
4. If asymmetry pattern is identical across architectures, it reflects training data bias; if different, it reflects architecture

**Pre-registration:** State expected direction of asymmetry before running experiments. VLA-Arena predicts L > V fragility; if PID shows opposite, either the estimator or the hypothesis is wrong.

### 14.5.5 Compositional Failure Confound (Long-Horizon Tasks)

**The confound:** VLA-Arena shows VLAs cannot compose learned skills for long-horizon tasks. This creates a confound where synergy degradation over time might reflect:
- (a) True integration failure (the hypothesis)
- (b) Simple action-sequence length effects (confound)
- (c) Compounding error from early mistakes (confound)

**Control protocol:**
```
COMPOSITIONAL FAILURE ANALYSIS
==============================

1. Segment long-horizon tasks into sub-goals (if ground truth available)
2. Compute PID terms per sub-goal segment, not just per trajectory
3. Test whether synergy degradation:
   - Occurs at sub-goal boundaries (compositional failure)
   - Accumulates gradually (compounding error)
   - Correlates with sub-goal novelty (generalization failure)

4. Control for trajectory position:
   - Regression: Synergy ~ timestep + sub_goal_index + error_so_far
   - If timestep explains all variance, it's a length confound

5. Matched comparison:
   - Compare same-length trajectories with different composition requirements
   - L1 (single primitive, 50 steps) vs L2 (composed, 50 steps)
   - If synergy patterns differ, composition matters
```

### 14.5.6 Summary: Required Controls Before Publication

| Confound | Control Method | Failure Criterion |
|----------|----------------|-------------------|
| **Task difficulty (L0/L1/L2)** | Stratified analysis | Effect disappears within strata |
| **Distribution shift (OOD)** | OOD score regression | Synergy non-significant after OOD control |
| **Memorization** | Memorization index stratification | Effect only in high-memorization stratum |
| **Modality asymmetry** | Baseline-normalized ΔUnq | Pattern identical across architectures |
| **Compositional length** | Segment-level analysis + matched comparison | Timestep explains all variance |

**Publication gate:** At least 3 of 5 confound controls must be passed (effect persists after controlling) before claiming "PID synergy predicts VLA failure."

### 14.5.7 Embodiment Gap Confound (Dream2Flow-Derived)

**Source:** Dharmarajan et al. (2025), "Dream2Flow: Leveraging Video Generative Models for Embodied Action Planning" (arXiv:2512.24766)

#### 14.5.7.1 The Problem: Conflating World Model Quality with Execution Failure

When computing PID on VLA variables `(V, D, A)`, we implicitly assume that failures in `A` reflect failures in information integration. However, Dream2Flow's staged analysis reveals that **failures can occur at multiple decoupled stages**:

| Stage | What Fails | PID Manifestation | True Cause |
|-------|-----------|-------------------|------------|
| **World model** | D encodes incorrect dynamics | Low `I(D;A)` and `Syn(V,D;A)` | Internal representation error |
| **Action decoding** | Correct D → wrong A | Low `I(D;A)` despite correct D | Decoder/head failure |
| **Physical execution** | Correct A command → wrong outcome | PID looks normal, task still fails | Robot/environment mismatch |

**Dream2Flow empirical evidence (qualitative; verify numbers in the paper if you need them):**
Dream2Flow reports that stage-wise success can be substantially higher than end-to-end success, highlighting that failures arise in **multiple decoupled stages** and that errors can compound and interact. For PID‑VLA, treat this as a requirement to **log and analyze stage outcomes explicitly** rather than attributing all failures to “integration quality”.

#### 14.5.7.2 Why This Confounds PID Analysis

When a VLA fails, PID analysis on `(V, D, A)` cannot distinguish:

1. **D is wrong** (world model failure): PID correctly identifies integration failure
2. **D is right but A is wrong** (action decoder failure): PID on D shows high synergy, but A fails anyway
3. **A is right but outcome is wrong** (embodiment gap): PID shows success signature, task fails

**Critical implication:** Correlating low `Syn(V,D;A)` with failure conflates these three distinct failure modes. A VLA could have a perfect world model (high synergy at D) but fail due to action decoding or embodiment mismatch.

#### 14.5.7.3 Control Strategies

**Strategy 1: Multi-Stage PID Analysis**
```
Compute PID at multiple stages along the VLA pipeline:

1. Early layers: Syn(V,L;D_early) — sensory integration
2. Middle layers: Syn(V,D;D_late) — world model formation  
3. Output layers: I(D;A) — action decoding quality (a simple dependence check, not a PID atom)
4. Execution: Compare A_commanded vs A_achieved (if measurable)

If failure correlates with stage 3 but not stages 1-2, the world model
is intact but action decoding fails.
```

**Strategy 2: Counterfactual Action Evaluation**
- For failed trials, extract the internal representation D at the failure point
- Use an oracle policy (privileged access or human teleop) to determine optimal A*
- Compute `I(D;A*)` vs `I(D;A_actual)`
- If `I(D;A*) >> I(D;A_actual)`, the D representation was correct but action decoding failed

**Strategy 3: Embodiment-Matched Comparison**
- Compare the same VLA checkpoint across different robots (if available)
- Embodiment-independent PID patterns (same across robots) → reflect world model quality
- Embodiment-dependent PID patterns → may reflect action decoder overfitting

**Strategy 4: Simulation-to-Real Gap Analysis**
- Compute PID in simulation (where embodiment gap is zero by construction)
- Compare to PID on same policy in real environment
- Divergence indicates embodiment confound magnitude

#### 14.5.7.4 Dream2Flow as Diagnostic Ground Truth

Dream2Flow's staged architecture provides natural ablations:

| Comparison | What It Reveals |
|------------|-----------------|
| Video correct, flow incorrect | Representation bottleneck (not world model) |
| Flow correct, execution incorrect | Embodiment gap (action execution failure) |
| Video incorrect, everything else N/A | World model failure (true D deficiency) |

**Recommendation:** When ground-truth video predictions are available (Dream2Flow, video prediction VLAs, or world model benchmarks), use video accuracy as an **upper bound** on world model quality. If video is correct but actions fail, the failure is downstream of D.

#### 14.5.7.5 Updated Confound Control Table

| Confound | Control Method | Failure Criterion |
|----------|----------------|-------------------|
| **Task difficulty (L0/L1/L2)** | Stratified analysis | Effect disappears within strata |
| **Distribution shift (OOD)** | OOD score regression | Synergy non-significant after OOD control |
| **Memorization** | Memorization index stratification | Effect only in high-memorization stratum |
| **Modality asymmetry** | Baseline-normalized ΔUnq | Pattern identical across architectures |
| **Compositional length** | Segment-level analysis | Timestep explains all variance |
| **Embodiment gap** | Multi-stage PID + counterfactual A* | Effect localizes to action decoder, not D |

**Updated publication gate:** At least 4 of 6 confound controls must be passed before claiming "PID synergy predicts VLA failure." The embodiment gap control is particularly important for any claim about world model quality.

## 14.6 Positional Encoding Confound (RoPE What-Where Entanglement)

**Source:** Gopalakrishnan et al. (2025), "Decoupling the 'What' and 'Where' With Polar Coordinate Positional Embeddings" (arXiv:2509.10534)

### 14.6.1 The Problem: Content-Position Entanglement in VLA Embeddings

Most modern VLAs (OpenVLA, TraceVLA, and any Llama-based architecture) use **Rotary Position Embeddings (RoPE)** for attention. Gopalakrishnan et al. (2025) demonstrate empirically that RoPE **entangles content ("what") and position ("where")** in attention scores.

**Mathematical basis (from the paper):**

In RoPE, the attention score between query at position t and key at position s is:
```
a_ts^RoPE = Σ_c μ_q_tc · μ_k_sc · cos((s-t)θ_c + φ_k_sc - φ_q_tc)
```

The interaction term `φ_k_sc - φ_q_tc` means that **both the key and query influence the effective phase (position)**. Content and position are confounded in the same representation.

**Empirical evidence:**
- On an "Indirect Indexing" diagnostic task requiring position-only or content-only matching:
  - RoPE: **11% accuracy** (fails to separate what/where)
  - PoPE (decoupled): **95% accuracy**
- This entanglement persists across model scales (124M to 774M parameters)

### 14.6.2 Implications for PID Analysis

| PID Component | How RoPE Confound Affects It |
|---------------|------------------------------|
| **I(V;A)** | Vision has spatial structure; RoPE conflates "what object" with "where in image/sequence" |
| **I(L;A)** | Language instruction position (word order) is entangled with word semantics |
| **I(V,L;A) synergy** | May reflect position-content joint encoding, not pure semantic integration |
| **Trajectory-level PID** | Action at timestep t has position t entangled with action content |

**Critical concern:** When computing `I(V,D;A)` across a trajectory, we may be measuring:
1. True semantic integration (intended)
2. Positional structure of the trajectory (confound)
3. A mixture of both (likely reality)

### 14.6.3 Why This Matters for VLA Specifically

VLA data has **multiple position axes** that RoPE entangles:

1. **Token position within context window:** Standard RoPE position
2. **Timestep within trajectory:** Action sequences are temporally ordered
3. **Spatial position in visual input:** ViT patch positions (often also use positional encoding)
4. **Word position in instruction:** Language has syntactic position

All of these positions are entangled with their respective content. PID analysis cannot cleanly separate "does the model integrate V and L semantically?" from "does the model use V and L positions jointly?"

### 14.6.4 Control and Mitigation Strategies

**Strategy 1: Use Pre-Attention Embeddings**
- Extract embeddings **before** RoPE rotation is applied
- In Llama-style architectures: use residual stream before attention, not after
- Limitation: Loses information about how the model uses position

**Strategy 2: Cross-Position Averaging**
- Compute PID for the same semantic content at different trajectory positions
- If PID estimates are stable across positions → position confound is minor
- If PID estimates vary with position → position confound is significant

**Strategy 3: Position-Matched Controls**
- Compare success vs failure cases **at the same trajectory timestep**
- This controls for position effects when comparing PID signatures

**Strategy 4: Explicit Position Regression**
- Include trajectory timestep as a covariate in statistical analysis
- Test whether PID effects persist after controlling for position
- `Syn ~ failure + timestep + (failure × timestep)`

**Strategy 5: Use Non-RoPE Models (If Available)**
- If a target VLA uses non‑RoPE positional embeddings (e.g., learned absolute embeddings), this confound may be reduced (verify in the actual implementation)
- Compare PID patterns between RoPE-based (OpenVLA) and non-RoPE models
- Difference in patterns may indicate RoPE-specific artifacts

### 14.6.5 Diagnostic: Position-Content Entanglement Score

**Proposed metric:** Measure how much PID estimates vary with trajectory position for semantically matched samples.

```
Entanglement Score = Var(PID | position) / Var(PID | semantics)
```

- High score: position dominates; RoPE confound is severe
- Low score: semantics dominates; RoPE confound is minor
- Calibrate thresholds using synthetic controls and report sensitivity

### 14.6.6 Publication Requirement

Before claiming "synergy predicts failure," must demonstrate one of:
1. **Position control:** Effect persists after controlling for trajectory timestep
2. **Cross-position stability:** Same semantic content shows stable PID across positions
3. **Architecture comparison:** Effect replicates in non-RoPE architecture
4. **Pre-attention extraction:** Effect observed in pre-RoPE embeddings

**Reference:** Gopalakrishnan A, Csordás R, Schmidhuber J, Mozer MC (2025). Decoupling the "What" and "Where" With Polar Coordinate Positional Embeddings. arXiv:2509.10534.

---

# 15. Numerical Stability and Optimization: Technical Guidance

This section documents known numerical issues, failure modes, and optimization strategies for making the estimators robust at scale.

## 15.1 Known Numerical Failure Modes

### 15.1.1 kNN Radius Collapse (Most Common)

**Symptom:** `PidError::NumericalInstability: kNN radius is non-positive`

**Causes:**
1. **Duplicate points:** Identical samples in the dataset
2. **Quantization:** Low-precision embeddings creating effective duplicates
3. **Constant dimensions:** Columns with zero variance

**Solutions (in order of preference):**
```rust
// 1. FIRST: Check for and remove exact duplicates
fn remove_duplicates(data: &mut Vec<Vec<f64>>) -> usize {
    let original_len = data.len();
    data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    data.dedup();
    original_len - data.len()
}

// 2. SECOND: Add small jitter (ONLY if duplicates cannot be avoided)
// Use Jitter::new(std, seed) with std ≈ 1e-10 to 1e-8
// WARNING: Jitter changes the quantity being estimated; re-validate

// 3. THIRD: Increase k if sample size permits (reduces tie sensitivity)
// But this increases bias; trade-off depends on n/d ratio
```

**⚠️ WARNING:** Do NOT silently add jitter. Always log when jitter is applied and quantify its effect on estimates.

### 15.1.2 Digamma Underflow for Small Arguments

**Symptom:** NaN or Inf in MI estimates when counts are very small

**Cause:** `digamma(x)` diverges as x → 0; if neighbor counts approach 0 due to sparse data, results become unstable.

**Solution (implemented in `stats.rs`):**
```rust
// Use the asymptotic expansion for small x:
// ψ(x) ≈ -1/x - 1/(2x²) for small x (but we shouldn't reach x < 1 in practice)

// Better: Ensure n > k + 1 always, and use a precomputed table for digamma(1..n)
pub fn digamma_int_table(n: usize) -> Vec<f64> {
    // Precompute ψ(1), ψ(2), ..., ψ(n) using the recurrence:
    // ψ(x+1) = ψ(x) + 1/x
    // ...
}
```

### 15.1.3 Distance Concentration at High Dimension

**Symptom:** MI estimates collapse to near-zero or become highly variable as d increases.

**Mathematical basis:** In high dimensions, the ratio of nearest-neighbor distance to average distance converges to 1 (Beyer et al., 1999). This makes kNN neighborhoods meaningless.

**Diagnostic (implemented in `geometry.rs`):**
```rust
// Compute the coefficient of variation of pairwise distances
// If CV < 0.1, distances are concentrated and kNN is likely unreliable
let stats = distance_concentration_stats(data, &cfg)?;
if stats.pairwise_cv < 0.1 {
    warn!("Distance concentration detected (CV={:.3}); kNN estimates may be unreliable", stats.pairwise_cv);
}

// Also check: nn_over_pairwise_mean should be << 1 for kNN to work
// If nn/pairwise_mean > 0.5, neighbors are not meaningfully "near"
```

**Solutions:**
1. Reduce dimensionality via PCA/projection BEFORE estimating
2. Use intrinsic dimension estimate to set appropriate k
3. Accept that kNN-based `I^sx_∩` may be invalid above some d threshold

### 15.1.4 Strong Dependence Pathology

**Symptom:** MI estimates have huge variance or are biased at low noise levels (high true MI).

**Cause:** When X nearly determines Y (or vice versa), the nearest neighbors in joint space are the same as in marginal space, breaking the KSG estimator's assumptions (Gao et al., 2015).

**Diagnostic:**
```rust
// Compute the empirical correlation or a proxy for dependence strength
// If |corr(X, Y)| > 0.95, warn about strong-dependence regime

// Better: Check if the 1-NN distance in joint space equals the marginal 1-NN distance
// for a large fraction of points (indicates near-determinism)
```

**Solutions:**
1. For MI-only: Use local Gaussian MI estimator (Gao et al., 2015, arXiv:1508.00536)
2. For `I^sx_∩`: Accept that noiseless signals may not be estimable; add explicit noise floor to target
3. Increase sample size significantly (quadratic in 1/noise for strongly dependent pairs)

## 15.2 Optimization Strategies

### 15.2.1 Memory-Efficient Distance Computation

For large n, storing the full n×n distance matrix is prohibitive. Use on-the-fly computation:

```rust
// Instead of: let distances = pairwise_distances(data); // O(n²) memory

// Use streaming kNN that computes distances row-by-row:
fn streaming_knn(data: MatRef<'_>, k: usize, metric: Metric) -> Vec<(Vec<usize>, Vec<f64>)> {
    let n = data.nrows();
    let mut results = Vec::with_capacity(n);

    for i in 0..n {
        // Compute distances from point i to all other points
        let mut dists: Vec<(usize, f64)> = (0..n)
            .filter(|&j| j != i)
            .map(|j| (j, metric.distance(data.row(i), data.row(j))))
            .collect();

        // Partial sort to find k smallest
        dists.select_nth_unstable_by(k - 1, |a, b| a.1.partial_cmp(&b.1).unwrap());

        let (indices, distances): (Vec<_>, Vec<_>) = dists[..k].iter().cloned().unzip();
        results.push((indices, distances));
    }
    results
}
```

### 15.2.2 SIMD Acceleration for Distance Computation

The distance computation hotloop benefits significantly from SIMD:

```rust
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[inline]
#[target_feature(enable = "avx2")]
unsafe fn chebyshev_distance_avx2(a: &[f64], b: &[f64]) -> f64 {
    debug_assert_eq!(a.len(), b.len());
    let n = a.len();
    let mut max_diff = _mm256_setzero_pd();

    let chunks = n / 4;
    for i in 0..chunks {
        let va = _mm256_loadu_pd(a.as_ptr().add(i * 4));
        let vb = _mm256_loadu_pd(b.as_ptr().add(i * 4));
        let diff = _mm256_sub_pd(va, vb);
        let abs_diff = _mm256_andnot_pd(_mm256_set1_pd(-0.0), diff); // abs via sign bit clear
        max_diff = _mm256_max_pd(max_diff, abs_diff);
    }

    // Horizontal max reduction
    let mut arr = [0.0f64; 4];
    _mm256_storeu_pd(arr.as_mut_ptr(), max_diff);
    let mut result = arr.iter().cloned().fold(0.0, f64::max);

    // Handle remainder
    for i in (chunks * 4)..n {
        result = result.max((a[i] - b[i]).abs());
    }
    result
}
```

### 15.2.3 Parallelization Strategy

kNN computation is embarrassingly parallel across query points:

```rust
use rayon::prelude::*;

fn parallel_knn_mi(x: MatRef<'_>, y: MatRef<'_>, cfg: &KsgConfig) -> PidResult<f64> {
    let n = x.nrows();

    // Compute per-point contributions in parallel
    let contributions: Vec<f64> = (0..n)
        .into_par_iter()
        .map(|i| {
            // Compute kNN contribution for point i
            compute_point_contribution(i, x, y, cfg)
        })
        .collect();

    // Aggregate (sum + normalization)
    Ok(contributions.iter().sum::<f64>() / (n as f64))
}
```

**Caution:** Ensure thread-local RNG states if any stochastic element is involved.

### 15.2.4 Approximate kNN (Use With Extreme Caution)

For very large n, exact kNN becomes infeasible. Approximate methods (HNSW, FAISS) introduce bias:

```
APPROXIMATE kNN DECISION TREE
=============================

Is n > 100,000 AND d < 100?
├── YES: Consider ball-tree (exact but faster)
└── NO: Continue

Is n > 1,000,000?
├── YES: Consider approximate kNN with validation
│   └── REQUIRED: Run Experiment 0 subset with exact vs approx
│   └── REQUIRED: Report approximation error bound
│   └── REQUIRED: Use conservative recall target (≥0.99)
└── NO: Use brute-force (it's fast enough)

NEVER use approximate kNN without explicit validation.
NEVER silently switch from exact to approximate based on n.
```

## 15.3 Numerical Precision Recommendations

| Operation | Recommended Precision | Rationale |
|-----------|----------------------|-----------|
| Distance computation | f64 | Avoid cancellation in differences |
| Distance storage (if needed) | f64 | Sorting/comparison sensitivity |
| Digamma evaluation | f64 | Series expansion needs precision |
| Final MI/PID output | f64 | But report with appropriate sig figs |
| Random projection matrix | f32 sufficient | Johnson-Lindenstrauss doesn't need f64 |

## 15.4 Debugging Checklist for Numerical Issues

```
WHEN ESTIMATES LOOK WRONG, CHECK:
================================

1. [ ] Are there NaN or Inf values in input data?
   → Use: data.iter().all(|x| x.is_finite())

2. [ ] Are there duplicate rows?
   → Count unique rows; if < n, investigate source

3. [ ] Are any columns constant?
   → Check column variance; remove or warn

4. [ ] Is k appropriate for n?
   → Rule of thumb: k << sqrt(n), and n > 10*k minimum

5. [ ] Is d appropriate for n?
   → If d > n/10, expect degradation; check intrinsic dim

6. [ ] Is the true MI huge (strong dependence)?
   → Add noise to target and check if estimates stabilize

7. [ ] Are preprocessing parameters logged?
   → Verify standardization was applied, check for NaN in mean/std

8. [ ] Is the random seed fixed for reproducibility?
   → Run twice with same seed; results must be identical
```

---

# 16. Why PCA and kNN Are Suboptimal for Manifold-Valued Embeddings

This section provides rigorous analysis of why standard dimensionality reduction and nearest-neighbor methods fail on manifold-structured data, and what alternatives exist.

## 16.1 The Manifold Hypothesis for Neural Embeddings

Modern neural embeddings (including VLA representations) empirically lie near **low-dimensional manifolds** embedded in high-dimensional ambient space. This creates a mismatch with standard Euclidean tools:

```
MANIFOLD STRUCTURE ILLUSTRATION
===============================

True data geometry:         What PCA/kNN assume:

    ╭─────────────╮              •  •  •  •
   ╱               ╲             •  •  •  •
  │    M ⊂ ℝᵈ      │            •  •  •  •
  │  (curved)      │            (uniform in ℝᵈ)
   ╲               ╱
    ╰─────────────╯

Geodesic distance ≠ Euclidean distance
Manifold dimension << ambient dimension
```

## 16.2 Why PCA Fails on Manifolds

### 16.2.1 Mathematical Failure Mode

PCA finds directions of maximum **linear** variance. On curved manifolds, this can:

1. **Conflate intrinsic and extrinsic variance:**
   - A spiral in 3D has high variance in all 3 axes but intrinsic dimension 1
   - PCA retains all 3 components, failing to discover the 1D structure

2. **Distort local neighborhoods:**
   - Two points close in geodesic distance may be far in Euclidean distance
   - PCA preserves Euclidean distances, not geodesic distances
   - After PCA, kNN may find "wrong" neighbors

3. **Introduce artifacts at high curvature:**
   - Regions of high curvature project onto overlapping linear subspaces
   - Distinct manifold regions become indistinguishable

### 16.2.2 Empirical Diagnostic

```rust
// Test for PCA inadequacy:
// 1. Estimate intrinsic dimension before and after PCA
// 2. If PCA dimension >> intrinsic dimension, PCA is overkill but safe
// 3. If PCA dimension < intrinsic dimension, PCA destroys structure

let id_raw = intrinsic_dimension_levina_bickel(raw_data, &cfg)?;
let id_pca = intrinsic_dimension_levina_bickel(pca_data, &cfg)?;

if pca_dims < id_raw * 0.8 {
    warn!("PCA may destroy manifold structure: ID_raw={:.1}, ID_pca={:.1}, PCA_dims={}",
          id_raw, id_pca, pca_dims);
}
```

### 16.2.3 When PCA Is Acceptable

PCA is acceptable when:
1. The manifold is approximately **linear** (low curvature everywhere)
2. The retained variance is >> 95% (minimal information loss)
3. Experiment 0 re-validation shows stable estimates after PCA
4. Intrinsic dimension is preserved (ID_after ≈ ID_before)

## 16.3 Why Euclidean kNN Fails on Manifolds

### 16.3.1 The Shortcut Problem

kNN with Euclidean distance finds "shortcuts" through the ambient space that do not exist on the manifold:

```
SHORTCUT PROBLEM
================

Manifold path (geodesic):      Euclidean path:
    A ───────╮                    A
             │                     ╲
    (long geodesic)                 ╲ (short Euclidean)
             │                       ╲
    B ───────╯                        B

kNN may declare A and B as neighbors even though
they are far apart on the manifold.
```

### 16.3.2 Impact on MI/PID Estimation

1. **Neighbor misidentification:** kNN finds "wrong" neighbors, leading to incorrect density estimates
2. **Volume estimation error:** The KSG estimator uses neighborhood volumes; Euclidean balls have wrong volume on curved manifolds
3. **Bias compounds with dimension:** Error grows exponentially with intrinsic dimension

### 16.3.3 Quantifying the Problem

A practical “shortcut distortion” diagnostic is to compare Euclidean distances to approximate geodesic distances on a kNN graph:

1. Build a kNN graph (Euclidean) with `k_graph` neighbors.
2. For a small set of anchor points (or random pairs), compute shortest-path distances on this graph (Dijkstra).
3. Compare ratios \(d_\text{geo}(i,j) / d_\text{euc}(i,j)\):
   - Large ratios (e.g., >2 on many pairs) indicate severe shortcut distortion (Euclidean neighbors are unreliable).

**Status:** This graph-geodesic distortion diagnostic is not implemented in `pid-core` yet; today we rely on intrinsic-dimension + distance-concentration proxies (§16.5).

## 16.4 Alternatives to PCA and Euclidean kNN

### 16.4.1 For Dimensionality Reduction

| Method | When to Use | Limitations |
|--------|-------------|-------------|
| **UMAP/t-SNE** | Visualization only | Non-invertible, distorts global structure |
| **Isomap** | When geodesic structure matters | Sensitive to noise, holes in manifold |
| **Diffusion Maps** | Multi-scale manifold structure | Computational cost, parameter sensitivity |
| **Autoencoders (VAE)** | Learned nonlinear projection | Changes the quantity; requires re-validation |
| **Hyperbolic embeddings** | Hierarchical / tree-like structure | Non-Euclidean metric; would require a new MI/`I^sx_∩` estimator (not drop-in) |

**Recommendation for PID-VLA:**
1. **First:** Try PCA with high variance retention (≥95%) + Experiment 0 re-validation
2. **If PCA fails:** Use random projections / feature hashing (preserves ambient Euclidean distances; not a geodesic fix) + re-validation
3. **If random projection fails:** Consider Isomap + re-validation, or accept that kNN-based PID is invalid

### 16.4.2 For Manifold-Aware MI Estimation

**Geodesic kNN MI (Marx & Fischer, 2021):**
- Replace Euclidean distances with geodesic distances
- Requires manifold to be explicitly estimated or approximated
- Computational cost: O(n² log n) for geodesic computation
- Does NOT directly provide `I^sx_∩`; use for MI-only screening

```python
# Pseudocode for geodesic kNN MI (not Rust; research prototype)
def geodesic_knn_mi(X, Y, k):
    # 1. Build k-NN graph on X
    # 2. Compute shortest-path geodesic distances
    # 3. Use geodesic distances in KSG estimator
    # 4. Repeat for Y and (X,Y) joint
    pass
```

**⚠️ WARNING:** Geodesic kNN MI is not implemented in `pid-core`. If manifold effects are suspected, treat this as a research direction, not a ready tool.

#### Hyperbolic embeddings: a concrete MI-only estimator pipeline (implemented; research-gated)

If you use **hyperbolic embeddings** (Poincaré/Lorentz) as a learned projection, you must also change the estimator’s notion of “neighborhood” to the **hyperbolic geodesic distance**. A minimal, defensible *MI-only* pipeline is:

1. **Represent points in the Lorentz (hyperboloid) model** of \(\mathbb{H}^d\) (constant curvature \(-1\)):
   - Points live in \(\mathbb{R}^{d+1}\) with Minkowski bilinear form \(\langle x,y\rangle_L = -x_0y_0 + \sum_{i=1}^d x_i y_i\)
   - Valid points satisfy \(\langle x,x\rangle_L = -1\) and \(x_0>0\)
2. **Use geodesic distance** \(d_\mathbb{H}(x,y) = \operatorname{arcosh}(-\langle x,y\rangle_L)\).
3. **Estimate MI terms using KSG with a product (L∞) joint metric**:
   - For MI `I(X;Y)`, use the joint distance \(d((x,y),(x',y')) = \max(d_\mathbb{H}(x,x'), d_\mathbb{H}(y,y'))\), then KSG counts in the marginals using the same \(\varepsilon_i\) radius (standard KSG structure).
4. **Compute Shannon-invariant screening terms** (CI/Ω) from these MI estimates.

**Status in this repo:** `pid-core` now provides an **experimental** hyperbolic geodesic distance via `Metric::HyperbolicLorentz`, so you can run:
- MI via `ksg_mi(…, KsgConfig { metric: Metric::HyperbolicLorentz, … })`
- CI via `co_information_pairwise` / `co_information_triplet` with the same metric

**Important limitations (do not overclaim):**
- This is an MI/CI pipeline only. It does **not** make the continuous shared-exclusions `I^sx_∩` estimator “hyperbolic-correct” automatically; the Ehrlich et al. (2024) estimator is validated under the Euclidean/L∞ convention. Treat “hyperbolic + `I^sx_∩`” as research, requiring a re-derivation + a new Experiment 0 gate.
- A learned hyperbolic projection is non-invertible and therefore **changes the measured quantity**; report it as a different measurement regime.

**Paper check (important): why we treat this as research-gated**
- Kraskov et al. (KSG MI) and the continuous shared-exclusions estimator of Ehrlich et al. explicitly use the **maximum norm / L∞** construction so that a joint-space “ball” factorizes into a product of marginal balls and the relevant volume terms cancel in KSG-style expressions.
- Ehrlich et al. also note that other *Euclidean* norms can yield asymptotically consistent density estimates under standard “nicely shrinking” conditions, but the exact KSG-style cancellation logic (and our cross-checks vs `csxpid`) are tied to the L∞ convention at finite sample sizes.
- Hyperbolic geodesic neighborhoods are not covered by that Euclidean-norm argument; curvature changes local volume elements and the disjunction-neighborhood construction would need to be re-derived. Therefore, we do **not** claim `I^sx_∩` on hyperbolic embeddings without a fresh derivation + Experiment 0 validation.

## 16.5 Determining Whether Manifold Methods Are Necessary

### 16.5.1 Decision Flowchart

```
MANIFOLD METHODS DECISION TREE
==============================

1. Estimate intrinsic dimension (ID)
   └── ID < ambient_dim / 10?
       ├── YES: Manifold structure likely significant
       │   └── Continue to step 2
       └── NO: Euclidean methods may suffice
           └── Proceed with PCA/Euclidean kNN

2. Compute distance concentration (DC)
   └── CV of pairwise distances < 0.2?
       ├── YES: Distance concentration; Euclidean kNN unreliable
       │   └── Continue to step 3
       └── NO: Euclidean kNN may work
           └── Validate with Experiment 0

3. Compute manifold distortion (if implemented)
   └── Max geodesic/Euclidean ratio > 2?
       ├── YES: Manifold structure critical
       │   └── PIVOT to manifold-aware methods OR
       │   └── Accept that kNN-based I^sx_∩ is invalid
       └── NO: Euclidean approximation acceptable
           └── Proceed with caution + Experiment 0 validation

4. Always: Re-run Experiment 0 after any dimensionality reduction
```

### 16.5.2 Practical Checklist for VLA Embeddings

```
MANIFOLD ANALYSIS CHECKLIST
===========================

Before running PID on VLA embeddings:

[ ] Compute intrinsic dimension estimate
    → Record: ID_V, ID_L, ID_D, ID_A, and joint IDs

[ ] Check distance concentration
    → Record: pairwise CV for each variable

[ ] If ID << ambient dim:
    [ ] Compare PCA-reduced ID to original ID
    [ ] If PCA destroys structure, consider alternatives

[ ] If using PCA:
    [ ] Record variance retained
    [ ] Re-run Experiment 0 subset
    [ ] Compare estimates before/after

[ ] If estimates are unstable across methods:
    [ ] Report instability as a finding
    [ ] Consider that kNN-based I^sx_∩ may not be appropriate
    [ ] Fall back to Shannon invariants (CI screening)
```

## 16.6 Local Flatness Testing: Empirically Validated Methods (Jan 2026)

The "locally flat" assumption underpins PCA and standard kNN MI estimation. This section documents **empirically validated methods** to test whether this assumption holds for VLA embeddings.

### 16.6.1 Method 1: Manifold Curvature via Subspace Angles ([IEEE 2023](https://ieeexplore.ieee.org/document/10020561/))

Compute weighted angles between local subspaces at each data point:

```python
def manifold_curvature_estimate(X, k=20, pca_dims=10):
    """
    Estimate manifold curvature at each point.
    Returns per-point curvature and global average.
    """
    N = len(X)
    curvatures = []

    for i in range(N):
        # 1. Find k nearest neighbors
        neighbors_i = knn(X, X[i], k)

        # 2. Compute local PCA subspace at point i
        S_i = local_pca(X[neighbors_i], n_components=pca_dims)

        # 3. For each neighbor j, compute subspace S_j
        angles = []
        for j in neighbors_i:
            neighbors_j = knn(X, X[j], k)
            S_j = local_pca(X[neighbors_j], n_components=pca_dims)

            # 4. Principal angle between subspaces
            angle = subspace_angle(S_i, S_j)
            weight = 1.0 / distance(X[i], X[j])
            angles.append(weight * angle)

        # 5. Curvature = minimum weighted angle
        curvatures.append(min(angles))

    return curvatures, np.mean(curvatures)
```

**What the paper reports (paraphrase; verify on your setting):** curvature proxies based on local subspace-angle statistics can decrease across layers during training for the models/tasks studied.

**Interpretation**:
- Low curvature (< 0.1 radians) → locally flat, PCA acceptable
- High curvature (> 0.5 radians) → manifold methods needed

### 16.6.2 Method 2: Ollivier-Ricci Curvature ([Nature Comm. 2021](https://www.nature.com/articles/s41467-021-24884-1))

Ollivier-Ricci curvature (ORC) is a discrete curvature notion with convergence results in some regimes; it is **not unique** among discrete curvature proposals, and its practical behavior depends strongly on graph construction (e.g., kNN graph quality) and the choice of neighborhood measures.

```
ORC(x, y) = 1 - W₁(μ_x, μ_y) / d(x, y)

Where:
- W₁ = Wasserstein-1 distance between neighborhood distributions
- μ_x = uniform distribution over k-NN of x
- d(x,y) = distance between x and y
```

**Interpretation**:
- ORC ≈ 0: locally flat (grid-like) → Euclidean methods valid
- ORC > 0: positively curved (sphere-like, clustered)
- ORC < 0: negatively curved (hyperbolic, tree-like) → consider hyperbolic methods

**Implementation status**: Not in `pid-core` yet. Python reference: `GraphRicciCurvature` package.

### 16.6.3 Method 3: DLME Local Flatness Constraint ([arXiv:2207.03160](https://arxiv.org/abs/2207.03160))

The Deep Local-flatness Manifold Embedding adds a second-order curvature penalty:

```
L_flatness = Σᵢ ||∇²f(x_i)||²_F

Where ∇²f is the Hessian of the embedding function
```

**Application to VLA**: Can be used to **train** flat embeddings, not just diagnose.

### 16.6.4 Method 4: Curvature-Adjusted PCA Diagnostic

Standard local PCA assumes flatness. Test the assumption:

```python
def local_flatness_diagnostic(X, k_values=[10, 20, 50, 100]):
    """
    If ID estimate increases with k, local flatness is violated.
    """
    id_estimates = []
    for k in k_values:
        id_k = intrinsic_dimension_levina_bickel(X, k=k)
        id_estimates.append(id_k)

    # Flatness violation if ID increases >20% with k
    if id_estimates[-1] > id_estimates[0] * 1.2:
        return "VIOLATED: larger neighborhoods capture global curvature"
    else:
        return "ACCEPTABLE: local flatness assumption holds"
```

**Key point (from the discussion in [arXiv:2510.15141](https://arxiv.org/abs/2510.15141); re-check the paper for exact statements):** when the data are curved/nonlinear, estimates that assume local linearity can change systematically with neighborhood size because larger neighborhoods “see” more global geometry.

## 16.7 δ-Hyperbolicity: Testing for Hierarchical Structure (Jan 2026)

> **Cross-reference (v6.3):** For application to Dream2Flow pipeline, see §10.10.12.3 ("The Hyperbolic/Lorentzian Connection"). The "Flow-as-bridge" idea can reduce reliance on non-Euclidean `D_wan` embeddings by using an explicitly Euclidean flow representation as the diagnostic target; you still must check flow dimensionality and distance concentration before interpreting kNN-based estimates.

### 16.7.1 The Gromov δ-Hyperbolicity Measure

δ-hyperbolicity measures how "tree-like" a metric space is. Trees have δ = 0; higher δ indicates deviation from tree structure.

**4-point (quadrilateral) form** (equivalent, and what `pid-core` implements):
For any four points \(a,b,c,d\), define:
```
s1 = d(a,b) + d(c,d)
s2 = d(a,c) + d(b,d)
s3 = d(a,d) + d(b,c)
```
Let \(L ≥ M ≥ S\) be these three sums sorted. Then the per-quadruple value is:
```
δ(a,b,c,d) = (L - M) / 2
```
The space is δ-hyperbolic if \(δ(a,b,c,d)\) is uniformly bounded over all quadruples.

**Implementation note (`pid-core`)**: `gromov_hyperbolicity(...)` samples quadruples, computes \(δ(a,b,c,d)\) using the chosen `Metric` (default Chebyshev/L∞), and returns the **mean raw δ** over samples. Raw δ is scale-dependent.

**Recommended normalization (scale-invariant reporting):**
```
δ_rel = 2 δ / diam(X)
diam(X) ≈ max_{i<j} d(x_i, x_j)   (under the same metric)
```
Heuristic thresholds (e.g., “δ_rel < 0.1”) only make sense in terms of \(δ_rel\) plus a clearly stated metric and preprocessing.

### 16.7.2 What Literature Uses δ For (and How to Use It Here)

[arXiv:2512.20926](https://arxiv.org/abs/2512.20926) uses δ-hyperbolicity, ultrametricity, and neighbor-joining tree fits to probe hierarchical structure in embedding spaces. Their quantitative values depend on the metric, normalization, sampling scheme, and preprocessing; for PID‑VLA, treat this paper as a **method template** and recompute the statistics on your own embeddings rather than transplanting numbers.

**Implication for PID‑VLA (careful version):**
- If \(δ_rel\) is very small, the embedding distances behave in a strongly tree‑like way. This flags a regime where the **Euclidean/Chebyshev volume logic** underlying the validated continuous `I^sx_∩` estimator is not currently justified.
- In that regime, prefer **Shannon invariants (MI-only)**, **quantization → discrete PID**, or **Flow-as-Bridge** rather than interpreting continuous PID atoms on raw embeddings.

### 16.7.3 When to Use Hyperbolic vs Euclidean

```
HYPERBOLICITY DECISION TREE
============================

1. Compute δ-hyperbolicity on sample (n=1000-5000)
   └── δ_rel < 0.1?  (report metric + normalization)
       ├── YES: Strong hierarchy
       │   ├── Use MI-only screening (CI/Ω) and/or quantization → discrete PID
       │   ├── Treat hyperbolic projections as optional feature engineering (re-validate)
       │   └── Do not interpret continuous `I^sx_∩` atoms on hyperbolic distances (no derivation)
       └── NO: Continue to step 2

2. δ_rel ∈ [0.1, 0.3]?
   ├── YES: Moderate hierarchy
   │   ├── Compare Euclidean PCA vs hyperbolic projection
   │   └── Choose based on Experiment 0 validation
   └── NO: δ_rel > 0.3, weak/no hierarchy
       └── Euclidean methods acceptable (with flatness check)
```

### 16.7.4 Do You Need to Train a Hyperbolic Embedding Model? (v5.7)

**Short answer:** Usually NO for PID-VLA. Here's the decision framework:

| Scenario | Train Hyperbolic Model? | Recommendation |
|----------|------------------------|----------------|
| **Using pre-trained VLA (OpenVLA, PixelVLA, TraceVLA)** | ❌ NO | Embeddings already exist; just compute δ-hyperbolicity to decide analysis method |
| **Dimensionality reduction for PID** | ⚠️ MAYBE | If δ_rel is very small (e.g., < 0.1 under a stated normalization), consider HypLoRA-style projection; otherwise use PCA |
| **Shannon invariant screening (CI)** | ❌ NO | CI works with any MI estimator; no hyperbolic training needed |
| **Full `I^sx_∩` on Llama hidden states** | ❌ NO | Use Experiment 0 to validate L∞ estimator; if fails, use quantization |
| **Custom VLA from scratch** | ⚠️ MAYBE | Consider HELM/Hypformer architecture if hierarchy is central |

**Where hyperbolic training IS needed:**

1. **If you want a hyperbolic projection layer** for dimensionality reduction:
   - Train a Poincaré/Lorentz projection head on top of frozen VLA
   - Use HypLoRA ([arXiv:2410.04010](https://arxiv.org/abs/2410.04010)) for efficient fine-tuning
   - Target: ~64-256 hyperbolic dimensions

2. **If you want to compare Euclidean vs Hyperbolic representations:**
   - Train parallel projection heads (one Euclidean, one hyperbolic)
   - Compare downstream PID diagnostics
   - This is a research experiment, not a requirement

**Where hyperbolic training is NOT needed:**

1. **For geometry diagnostics** (δ-hyperbolicity, curvature): Just compute on existing embeddings
2. **For Shannon invariants (CI, Ω)**: Works with standard MI estimators
3. **For SAE analysis**: SAEs operate in Euclidean space
4. **For full `I^sx_∩`**: The L∞ estimator is Euclidean; hyperbolic `I^sx_∩` doesn't exist yet

**Practical recommendation for PID-VLA:**
```
1. Extract embeddings from pre-trained VLA (OpenVLA, PixelVLA, etc.)
2. Compute δ-hyperbolicity
3. Convert to δ_rel using an explicit normalization (e.g., diameter)
4. If δ_rel is very small: Use Shannon invariants (CI/Ω) for screening; report tree-like structure and avoid continuous PID atoms on raw embeddings
5. If δ_rel is not very small: Use standard PCA + L∞ `I^sx_∩` (with Experiment 0 validation)
6. Training hyperbolic models is OPTIONAL and only for comparative research
```

## 16.8 SAE Analysis for VLA Embeddings (Jan 2026)

### 16.8.1 What Sparse Autoencoders Reveal

Sparse Autoencoders (SAEs) decompose polysemantic activations into sparse, more interpretable feature dictionaries. Recent work ([arXiv:2504.02821](https://arxiv.org/abs/2504.02821)) extends SAE analysis to vision-language models (e.g., CLIP) and evaluates monosemanticity with a user-study-derived benchmark.

**Key findings**:
- **Multi-scale structure** in SAE feature spaces (small-scale “crystals”, intermediate “lobes”) has been reported in LLM SAE dictionaries ([arXiv:2410.19750](https://arxiv.org/abs/2410.19750)); validate whether analogous structure appears in VLA/VLM components.
- **Geometric regularities** (e.g., parallelogram/trapezoid relations generalizing classic word-embedding analogies) appear in some SAE feature dictionaries ([arXiv:2410.19750](https://arxiv.org/abs/2410.19750)).
- **Steering capability**: intervening on SAE latents in a VLM vision encoder can steer multimodal LLM outputs (e.g., LLaVA) without modifying the underlying LLM ([arXiv:2504.02821](https://arxiv.org/abs/2504.02821)).

### 16.8.2 SAE Application to VLA Components

| VLA Component | SAE Applicability | Benefit |
|---------------|-------------------|---------|
| **SigLIP/CLIP-like vision encoder** | ✓ Demonstrated for VLMs (arXiv:2504.02821; validate for your exact encoder/layer) | Decompose V into more monosemantic visual features; targeted interventions |
| **DinoV2-like vision features** | ✓ Plausible but unverified | Feature separation; dimensionality reduction targets |
| **LLM hidden states** | ✓ Used in mechanistic-interpretability SAE work (validate for your model/activation point) | Interpretable L/D representations; feature-based ablations |
| **Action decoder / policy head** | ? Unclear | May reveal action primitives, but depends on architecture and supervision |

### 16.8.3 SAE for PID Analysis: Concrete Protocol

```python
# 1. Train SAE on vision encoder (e.g., SigLIP layer in OpenVLA)
sae = SparseAutoencoder(
    d_input=1024,      # SigLIP output dim
    expansion=16,       # 1024 → 16384 sparse features
    sparsity_penalty=0.04
)
sae.train(vision_embeddings)

# 2. Extract sparse features
V_sparse = sae.encode(vision_embedding)  # Sparse, ~100 active features

# 3. Compute PID on SAE features
# - Lower effective dimension (only active features)
# - More interpretable decomposition
# - Can identify WHICH features drive actions

# 4. Feature ablation for failure diagnosis
for feature_idx in top_active_features:
    V_ablated = ablate_feature(V_sparse, feature_idx)
    action_change = model.forward(V_ablated) - model.forward(V_sparse)
    if action_change > threshold:
        print(f"Feature {feature_idx} drives action prediction")
```

### 16.8.4 Geometric Implications of SAE

SAE features have structure at three scales ([arXiv:2410.19750](https://arxiv.org/abs/2410.19750)):

1. **Atomic scale**: "Crystals" — parallelogram/trapezoid faces (analogy relations)
2. **Intermediate scale**: "Lobes" — modular clustering (math, code, language)
3. **Global scale**: Hierarchical organization of concept space

**Implication**: SAE features may have LOWER effective dimensionality and MORE hierarchical structure than raw embeddings, making them better candidates for:
- Shannon invariant screening (CI)
- Hyperbolic projection
- Interpretable PID decomposition

## 16.9 Chebyshev Distance and PixelVLA: Geometry Transition Analysis (Jan 2026)

### 16.9.1 Chebyshev in Image Processing

Chebyshev distance (L∞) is natural for pixel operations:

| Operation | Distance Metric | Structuring Element |
|-----------|-----------------|---------------------|
| **8-connected dilation/erosion** | Chebyshev (L∞) | Square (3×3) |
| **4-connected dilation/erosion** | Manhattan (L1) | Cross/Diamond |
| **Edge detection (8-neighbor)** | Chebyshev | Square kernel |
| **Pattern recognition** | Often L∞ | Square windows |

### 16.9.2 Geometry Transition in VLA Pipeline

```
GEOMETRY TRANSITION IN VLAs
============================

              PIXEL SPACE                    SEMANTIC SPACE
    ─────────────────────────────────────────────────────────

    Vision Encoder                              LLM Backbone
    (DinoV2, SigLIP)                           (Llama 2 7B)

    • L∞ neighborhoods match 8-connectivity      • Geometry is empirical: may be anisotropic,
      on pixel grids                              concentrated, or hierarchical
    • Learned features: do not assume           • Diagnose via ID/DC/δ_rel (do not assume)
      “L∞ is natural” post-encoder
    • ~1024 dim (example)                       • 4096 dim (example)

    APPROPRIATE:                                APPROPRIATE:
    L∞ estimator (only after Exp0 + geometry)   MI-only screening (CI/Ω) or discrete PID
    PCA may work if locally flat                Hyperbolic claims must be measured
    SAE for feature decomposition               Flow-as-Bridge when available
```

### 16.9.3 Where Chebyshev Is Appropriate in PixelVLA

This table is about whether an **L∞ neighborhood shape** is a reasonable heuristic for kNN queries at that stage. It is *not* a proof of estimator validity; the Experiment 0 + geometry gates still apply.

| Stage | Geometry | L∞ neighborhood heuristic? |
|-------|----------|------------------|
| **Raw image input** | Pixel grid | Yes (8-connectivity), but PID is not usually run on raw pixels |
| **DinoV2 patches** | Patch embeddings | Unknown; measure ID/DC/δ_rel and validate |
| **SigLIP output** | Global features | Unknown; measure + validate |
| **Multiscale encoder** | Hierarchical features | Unknown; measure + validate |
| **MLP projector output** | LLM-aligned | Unknown; often high-d; measure + validate |
| **LLM hidden states** | Semantic space | Unknown; often high-d/concentrated; expect MI-only/quantization to be safer unless gates pass |
| **Action decoder** | Continuous actions | Often low-d; likely acceptable if locally flat (still validate) |

### 16.9.4 Recommendation for PixelVLA PID Analysis

1. **Choose the representation first, then validate**: run geometry diagnostics + Experiment 0 on the exact stage you plan to analyze (patches vs pooled features vs projector output).
2. **If the gates pass (after reduction if needed)**: PCA→(≤256) + L∞ `I^sx_∩` is a candidate for two-source PID on that representation.
3. **If the gates fail on high‑D stages (common for LLM-aligned features)**: prefer MI-only screening (CI/Ω), quantization → discrete PID, or Flow‑as‑Bridge rather than interpreting continuous PID atoms.
4. **Actions** are usually low-dimensional; they are often the safest target variable, but still require i.i.d./autocorrelation controls.

## 16.10 Hierarchical Structure: GPT-2 vs Modern LLMs (Jan 2026)

This subsection is a cautionary note: architecture differences can change anisotropy, intrinsic dimension, and tree-likeness, but the direction is not reliably predictable. Treat “model X is more hierarchical than model Y” as a **measurable hypothesis**, not a premise.

### 16.10.1 Architecture Differences That Might Affect Geometry (Hypotheses)

| Feature | GPT-2 | Llama 2 | Why it might matter (hypothesis; must be measured) |
|---------|-------|---------|-----------------------------------------------------|
| **Position encoding** | Absolute (learned) | RoPE (rotary) | Changes similarity structure across positions; effect on δ_rel/ID is empirical |
| **MLP nonlinearity** | GELU (typical GPT-2) | SwiGLU | Different nonlinearities can affect anisotropy and effective dimension |
| **Attention** | Multi-head (MHA) | Grouped-query (GQA) | Changes parameterization and may change representational geometry |
| **Context length** | ~1k (common configs) | ~4k (common configs) | Longer context can change representation mixing and long-range structure |
| **Depth (example)** | 12 layers (GPT-2 small) | 32 layers (Llama 2 7B) | Depth changes compositional capacity; geometry may evolve across layers |

### 16.10.2 Empirical Evidence for Hierarchy Evolution

| Evidence | Source | Finding |
|----------|--------|---------|
| **Token embeddings** | [HypLoRA](https://arxiv.org/abs/2410.04010) | Paper reports high hyperbolicity in token embeddings (verify metric/setting) |
| **δ-hyperbolicity** | [arXiv:2512.20926](https://arxiv.org/abs/2512.20926) | Paper reports lower δ / more hierarchical structure in some modern models vs older baselines (verify the exact numbers and sampling method) |
| **Brain alignment** | [arXiv:2502.14671](https://arxiv.org/html/2502.14671v1) | Paper reports layer-wise differences in brain alignment (verify dataset/metric) |
| **Hyperbolic LLMs** | [HELM](https://arxiv.org/abs/2505.24722) | Hyperbolic LLM variants; improvements are benchmark-dependent and need replication |
### 16.10.3 What to Do in PID‑VLA (Instead of Assuming)

For any candidate VLA (GPT‑2‑backed or Llama‑backed):
1. Measure geometry **per layer and per representation** (ID, distance concentration, δ_rel) on the embeddings you actually plan to analyze.
2. Apply the Experiment 0 gate on the chosen preprocessing (e.g., z‑score + PCA→256).
3. Only then decide whether continuous PID atoms are interpretable, or whether you should pivot to MI‑only / discrete / Flow‑as‑Bridge.

## 16.11 Unified Geometry-First Protocol (Jan 2026)

Based on the first-principles analysis, here is the recommended protocol:

> **Cross-reference (v6.3):** For Dream2Flow + WAN integration, see §10.10.12 which applies this protocol to specific pipeline stages. Note that 3D object flow lives in Euclidean \(\mathbb{R}^{3T}\) (though it can still be high-dimensional for large \(T\)), so it avoids *non-Euclidean metric* issues but not the curse-of-dimensionality or autocorrelation pitfalls.

### 16.11.1 What To Compute (Implemented Diagnostics + Optional Extensions)

Before interpreting any PID atoms on a representation \(X\) (and especially before moving from 2‑way → 3‑way PID), compute diagnostics on:
- each marginal \(V, L, D, A\) you will use, and
- the **joint concatenations** that appear in your estimator calls (e.g., \([V;L]\), \([V;D]\), \([V;L;D]\)).

**Implemented in `pid-core` (Experiment‑0 scale, O(n²)):**
- **Intrinsic dimension** \(\hat d\) (Levina–Bickel): `intrinsic_dimension_levina_bickel`
- **Distance concentration** (pairwise CV, `nn_over_pairwise_mean`): `distance_concentration_stats`
- **δ-hyperbolicity** (4‑point sampling): `gromov_hyperbolicity` (raw δ); report `δ_rel = 2δ / diam(X)` with `diam(X)` measured under the same metric

**Optional external methods (not implemented here):**
- Neighborhood PCA residuals / subspace-angle curvature proxies (§16.6.1)
- Graph-based curvatures like Ollivier–Ricci on a kNN graph (§16.6.2), noting that poor kNN graphs in high‑D can make these unreliable

### 16.11.2 Geometry → Estimator / Decomposition Decision Matrix

| Diagnostic regime (heuristic) | What it means for estimation | Recommended estimation strategy | Decomposition strategy | Hypotheses you can still test cleanly |
|---|---|---|---|---|
| **Modest \(\hat d\)**, **no strong concentration** (pairwise CV not tiny), **δ_rel not very small**, locally flat-ish | Euclidean/Chebyshev neighborhood logic is at least plausible | PCA/whitening → continuous `I^sx_∩` (L∞) + KSG MI, *after Experiment 0 passes on that pipeline* | **Primary:** 2‑way PID (`pid2`). **Optional:** 3‑way PID (`pid3`) only offline and only after MI/CI stability checks | H1–H6 on `(V,L;A)` or `(V,D;A)`; H7 on flow targets if available |
| **Strong distance concentration** (very low CV; `nn_over_pairwise_mean → 1`) | kNN neighborhoods become unstable; variance/bias dominate | Reduce dimensionality aggressively; increase N; if still concentrated → MI-only or discrete | Prefer **hierarchical screening** (CI/Ω, pairwise MI/PID) | H4/H5/H6 as *comparative* diagnostics (ΔCI/ΔMI) under perturbations; avoid fine-grained atom claims |
| **Very small δ_rel** (tree-like distances) | Continuous `I^sx_∩` derivation is not justified in this geometry | MI-only screening (CI/Ω); quantization → discrete PID; Flow-as-Bridge when possible | Prefer **hierarchical pairwise** over full 3‑way atoms; treat hyperbolic projections as optional feature engineering (re-validate) | H7 stage attribution; H4/H6 as MI/CI shifts; avoid “continuous PID atom” conclusions on raw embeddings |
| **Strong dependence / heavy tails / autocorrelation dominates** | KSG/ISX can break even at low d | Use strong-dependence MI estimators (e.g., Gao–Ver Steeg–Galstyan) as a check; enforce block bootstrap / trajectory controls | Keep decomposition simple; emphasize uncertainty and robustness | Hypotheses become primarily about *robustness of invariants* under controls, not absolute atom values |

### 16.11.3 Minimal Sanity-Checks With Small Models (Optional)

Use a small, fast model (NanoGPT‑class) to validate the *plumbing* of geometry diagnostics and logging before spending effort on VLAs:

```python
# NanoGPT-based geometry sanity-check (conceptual)
model = NanoGPT(d_model=256, n_layers=6, n_heads=4)
model.train(action_prediction_dataset)

for layer in range(model.n_layers):
    emb = model.get_hidden(validation_set, layer)
    d_hat = levina_bickel_mle(emb, k=10)
    dc = distance_concentration_stats(emb)
    delta = gromov_hyperbolicity(emb, n_samples=1000)
    # Convert to δ_rel using an explicit diameter estimate.
```

Do not assume monotonic trends (e.g., “curvature decreases with layer”) without measuring them; use this to confirm that diagnostics behave sensibly on controlled data and are stable across seeds.

## 16.12 Theoretical Limitations (Fundamental, Not Fixable)

Some limitations are fundamental to kNN-based estimation on manifolds:

1. **Volume-form mismatch:** The KSG estimator assumes uniform volume elements; on curved manifolds, volume elements vary with curvature. This introduces bias even with geodesic distances.

2. **Intrinsic dimension heterogeneity:** If the intrinsic dimension varies across the manifold (e.g., lower near boundaries), kNN-based ID and MI estimates become inconsistent.

3. **Non-compact manifolds:** If the manifold is unbounded or has holes, geodesic distances can be undefined or infinite.

**Implication for PID-VLA:** Accept that there may be regimes where no kNN-based estimator works reliably. In such cases:
- Use Shannon invariants (CI, O-information) as the primary diagnostic
- Report kNN-based `I^sx_∩` with explicit caveats
- Consider neural MI estimators (MINE, etc.) as cross-checks

---

# 17. Training, Compute, and Data Requirements Analysis (v5.9)

This section provides a comprehensive, critical analysis of all components in the PID-VLA project that require training, with explicit compute cost estimates, data requirements, and guidance on obtaining or generating necessary data.

## 17.1 Executive Summary: Training Requirements Classification

| Category | Components | Training Required? | Data-Heavy? | Compute-Heavy? |
|----------|------------|-------------------|-------------|----------------|
| **Core PID Estimators** | KSG MI, `I^sx_∩`, Shannon invariants | ❌ No training | ❌ No | ⚠️ Moderate (kNN is O(N²d)) |
| **VLA Models (inference only)** | OpenVLA, DreamVLA, PixelVLA, TraceVLA | ❌ No training (use pre-trained) | ❌ No | ⚠️ Moderate (7B inference) |
| **VLA Fine-tuning** | LoRA adaptation | ✅ Yes | ✅ Yes | ✅ Yes |
| **Dimensionality Reduction** | PCA, Random Projection, Hash Projection | ❌ No training | ❌ No | ❌ Minimal |
| **Learned Projections** | Autoencoders, Contrastive | ✅ Yes | ⚠️ Moderate | ⚠️ Moderate |
| **SAE (Sparse Autoencoders)** | Vision/LLM decomposition | ✅ Yes | ✅ Yes | ✅ Yes |
| **Hyperbolic Embeddings** | Poincaré/Lorentz projection | ✅ Yes | ⚠️ Moderate | ⚠️ Moderate |
| **Neural MI Estimators** | MINE, CCMI | ✅ Yes | ⚠️ Moderate | ⚠️ Moderate |
| **World Models** | WAN, GWM, Genie 3 | ✅ Yes (if fine-tuning) | ✅ Yes | ✅✅ Very High |
| **Process Reward Models** | GRM (Robo-Dopamine) | ✅ Yes | ✅✅ Very High | ✅✅ Very High |
| **Failure Classifiers** | Learned baselines | ✅ Yes | ⚠️ Moderate | ⚠️ Moderate |
| **Depth Estimation** | DKT, Depth-Anything | ✅ Yes (if fine-tuning) | ✅ Yes | ✅ Yes |

## 17.2 Core PID Estimators (No Training Required)

### 17.2.1 KSG Mutual Information

**Training:** None — the KSG estimator is a non-parametric algorithm.

**Compute requirements (measurement-first):**
- Naive exact kNN scales as `O(N² d)`; storing a full pairwise distance matrix is `O(N²)` memory (avoid at large `N`).
- Tree/graph methods can help when intrinsic dimension is modest; approximate kNN can help but changes estimator behavior (validate under Experiment 0).
- Report measured wall-clock and peak memory for your chosen `(N,d,k)` and backend (exact vs approximate; CPU vs GPU).

**Sample size:** requirements grow rapidly with intrinsic dimension and dependence strength; treat any “works at (N,d)” claim as empirical until validated (Experiment 0 + Geometry Gate).

### 17.2.2 Continuous `I^sx_∩` Redundancy

**Training:** None — Ehrlich et al. (2024) estimator is non-parametric.

**Compute Requirements:**
- Same as KSG MI but with additional disjunction-distance computation
- Typically higher than standard KSG MI due to additional disjunction-distance computation (measure on your backend and `(N,d,k)`)
- Full pairwise distance matrix required: O(N²) memory or streaming computation

**Data Requirements:**
- Experiment 0 validation: synthetic data spanning your planned `d` and dependence regimes (see §9.1 and `EXPERIMENTS.md` §4)
- VLA analysis: extract embeddings from trajectories (see §17.3)

### 17.2.3 Shannon Invariants (CI, Ω)

**Training:** None — computed from MI terms.

**Compute requirements (measurement-first):**
- Computed from a small fixed set of MI/CMI terms (often substantially cheaper than estimating full multi-source PID atoms).
- Still inherits the MI estimator’s scaling and geometry pathologies; use as **screening** and validate stability under controls.

**Why This Matters:** Shannon invariants are the recommended "Level 0" screening layer precisely because they require NO training and have moderate compute cost.

## 17.3 VLA Embedding Extraction (Pre-trained Inference)

### 17.3.1 OpenVLA / PixelVLA / TraceVLA (Llama 2 7B backbone)

**Training:** None required for embedding extraction — use pre-trained weights.

**Inference compute (benchmark-dependent):**
- **Memory:** weights-only memory is approximately `params × bytes_per_param` (e.g., fp16 ≈ 2 bytes/param). For a 7B-parameter model this is ~14GB for weights alone; runtime overhead and KV cache can add substantially.
- **Latency/throughput:** depends on runtime (CUDA/Metal/CoreML), quantization, sequence length, batch size, and kernel choices. Do not report fixed ms numbers without measurement on your exact setup.

**Data sources / benchmarks (examples; verify sizes/licensing before citing):**
| Source | Purpose | Notes |
|--------|---------|-------|
| Open‑X Embodiment | Pretraining provenance / broad evaluation | Verify access terms and exact subsets used |
| VLA‑Arena | Perturbation axes for robustness tests | Verify dataset availability and protocol |
| LIBERO | Standard manipulation benchmark | Verify task definitions and splits |
| SimplerEnv | Lightweight iteration / sanity checks | Verify task parity with your study |
| Pixel‑160K (PixelVLA) | Pixel-level prompting analysis | Abstract says “will be released”; verify availability |

**Estimated data volume (illustrative; measure on your logging schema):**
- `10k trajectories × 100 timesteps × 4096 dims × 4 bytes ≈ 16GB` per variable if you store dense float32 arrays (V/L/D/A) without compression.
- In practice, you should log only the representations you analyze, prefer float16/quantized storage where valid, and compress/chunk data formats (see `EXPERIMENTS.md` §11).

### 17.3.2 DreamVLA (backbone dims unspecified in abstract)

**Training:** None for embedding extraction *if* weights and an inference API are available.

**Critical gap:** The DreamVLA arXiv abstract does not specify backbone family/dimensions or hidden sizes. Treat all such details as unknown until verified from a primary source (paper/code/model card). Do not assume it is “smaller than Llama 2 7B” without measurement.

**Inference compute:** model- and runtime-dependent; report measured latency/throughput and peak memory on your deployment.

## 17.4 VLA Fine-tuning (High Compute, High Data)

### 17.4.1 LoRA Adaptation of OpenVLA

**When Needed:** Task-specific fine-tuning for new domains, benchmarks, or custom robots.

**Training requirements (measurement-first):**
- Pre-trained base weights + a reproducible fine-tuning codepath.
- A declared LoRA/adapter configuration (rank, target modules, precision) recorded in the run manifest.
- Domain/task data with explicit licensing and provenance.
- Measured training throughput, peak memory, and wall-clock on your hardware/runtime (do not cite generic “X hours on Y GPU” numbers).

**Data sources (examples; verify availability and licensing):**
| Source | Notes |
|--------|------|
| Existing benchmarks (e.g., LIBERO, SimplerEnv) | Useful for matched comparisons; ensure protocol parity |
| Collect in simulation | Cheap iteration; validate sim realism for your claim scope |
| Collect on real robot | High fidelity; higher operational overhead and safety constraints |
| Perturbation benchmarks (e.g., VLA‑Arena) | Useful for robustness analyses; verify access terms |

### 17.4.2 Full VLA Pre-training (Extreme Compute)

**When Needed:** Only if training VLA from scratch (not recommended for PID-VLA project).

**Recommendation (scope discipline):** Use pre-trained VLAs. Full pre-training is out of scope for PID‑VLA unless the study explicitly targets training dynamics. If you do pre-train, report measured compute, data, and costs for your exact setup and treat it as a separate project with its own reproducibility package.

### 17.4.3 Moondream-Inspired Small VLA (Alternative to DreamVLA)

**When Needed:** If DreamVLA unavailable AND a small model is required for rapid iteration (see §7.7 for full architecture details).

**Scope note:** This is an *engineering contingency* for pipeline validation (logging/interventions/estimator plumbing), not a primary scientific target. If used, it must be documented and reproducible enough to be a credible object of study (see §7.7).

**Training requirements (measurement-first):**
- Choose a small open backbone and a simple, explicit fusion/projection mechanism.
- Record data provenance/licensing, commit hashes, hyperparameters, and seeds.
- Report measured training/inference throughput and peak memory for your configuration; do not cite generic “X days on Y GPUs” estimates.

**Advantages:** faster iteration for engineering and cleaner ablations if encoders are frozen.

**Disadvantages:** not automatically representative of large VLAs; cannot substitute for DreamVLA/OpenVLA claims unless benchmarked under matched protocols.

**Recommendation:** Use only if it materially accelerates Experiment 0/1 engineering; otherwise prioritize pre-trained VLAs.

## 17.5 Dimensionality Reduction (No Training)

### 17.5.1 PCA (Linear Projection)

**Training:** None — SVD on data matrix.

**Compute (measurement-first):** exact SVD is expensive at large `N`/`d`; randomized/incremental PCA is often preferred. Benchmark your PCA implementation and record runtime/memory for your dataset size and target dimension.

**Data Requirements:** Same embeddings used for PID (no additional data).

### 17.5.2 Random Projection / Hash Projection

**Training:** None — random matrix generation (seeded).

**Compute:** O(Nd × d_target) — matrix multiply, very fast.

**Data Requirements:** None.

## 17.6 Learned Dimensionality Reduction (Training Required)

### 17.6.1 Sparse Autoencoders (SAE)

**When Needed:**
- Interpretable feature decomposition (§16.8)
- Lower effective dimension for PID
- Monosemantic feature identification

**Training requirements (measurement-first):**
- Choose an SAE architecture/expansion factor and record it (do not assume a “typical” default is optimal).
- Training data are activation vectors from frozen VLA forward passes; report the extraction point(s) and dataset provenance.
- Report measured training throughput and peak memory; avoid fixed hour/GB claims.

**Data Sources:**
- Run frozen VLA on existing trajectory datasets
- Extract activations at target layer(s)
- No new robot data collection required

**Compute note:** SAE cost depends strongly on activation dimension, dataset size, expansion factor, optimizer settings, and hardware. Treat cost as empirical: benchmark on a small subset, then scale.

**Code Resources:**
- [SAELens](https://github.com/jbloomAus/SAELens) — well-maintained SAE training library
- [sae_analysis](https://github.com/Abzinger/sae_analysis) — Makkeh's Shannon invariants for SAE

### 17.6.2 Contractive/Variational Autoencoders

**Training requirements (measurement-first):**
- Choose an encoder/decoder architecture and objective (contractive/variational/reconstruction) and record it in the manifest.
- Use embedding vectors from the frozen VLA stage you analyze (same source as PID inputs).
- Report measured training throughput and peak memory for your setup; do not cite generic hour/GB figures.

**Data Sources:** Same as SAE — run frozen VLA on trajectory datasets.

### 17.6.3 Hyperbolic Projection Heads (HypLoRA-style)

**When Needed:**
- δ_rel is very small under an explicit normalization (tree-like distances)
- Hierarchy-preserving dimensionality reduction
- Research comparison with Euclidean methods

**Training requirements (measurement-first):**
- Define the projection geometry (e.g., Lorentz/Poincaré) and loss; record it.
- Use embeddings with an explicit structure-preserving objective (contrastive/reconstruction); report dataset provenance.
- Report measured training/inference cost for your implementation; avoid fixed hour/GB claims.

**⚠️ Critical Note:** Hyperbolic MI/PID estimators are research-gated (§16.4.2, §16.7.4). Training a hyperbolic projection changes the measured quantity and requires re-validation.

## 17.7 Neural MI Estimators (Training Required)

### 17.7.1 MINE (Mutual Information Neural Estimation)

**When Needed:**
- KSG fails at high dimension (Experiment 0 NO-GO)
- MI-only screening (not full `I^sx_∩`)
- Cross-check for kNN estimates

**Training requirements (measurement-first):**
- A critic network + optimizer; record architecture, objective variant, and early-stopping criteria.
- Training data is the same samples used by kNN MI (no extra data), but optimization stability can vary widely; report variance across seeds.
- Report measured per‑estimate time and peak memory for your configuration.

**Critical Challenge:** MINE must be **retrained for each MI estimate**. This makes it expensive for full PID decomposition (which requires multiple MI terms).

**Data Sources:** Same embeddings as kNN-based methods (no additional data).

### 17.7.2 CCMI (Conditional MI)

**Training requirements (measurement-first):**
- A classifier-based estimator for conditional MI; record architecture/objective/negative sampling scheme.
- Conditional estimation can be brittle; report stability checks and sensitivity to hyperparameters.

**When Needed:** Conditional analyses (e.g., I(V;A|L)).

## 17.8 World Models (Very High Compute/Data)

### 17.8.1 WAN (Wan) Fine-tuning

**When Needed:**
- Action-conditioned video generation for VLA visualization
- Data augmentation for VLA training

**Pre-trained inference (no training):**
- The Wan paper (arXiv:2503.20314) reports 1.3B and 14B models and claims the 1.3B model can run with ~8.19 GB VRAM in their setting.
- Inference time/VRAM depend strongly on runtime, precision, resolution, and clip length; measure on your hardware and report configuration details.

**Fine-tuning/adapters for robot domain or conditioning:**
- Treat as a separate, high‑compute project: data requirements, training time, and cost depend on the conditioning scheme and target tasks.
- If you fine‑tune, report dataset provenance and consider circularity/confound risks (a fine‑tuned predictor may learn the same biases as the VLA you are diagnosing).

**Data Sources:**
| Source | Availability | Robot-specific? |
|--------|--------------|-----------------|
| Open-X Embodiment videos | Public | ✅ Yes |
| RoboMimic datasets | Public | ✅ Yes |
| Simulate in Isaac Sim/Gazebo | Generate yourself | ✅ Yes |
| Internet video | Public (varies) | ❌ No |

### 17.8.2 GWM (Gaussian World Model)

**Compute trade-off:** can be lighter than large diffusion video models in some implementations, but requires 3DGS scene reconstruction; benchmark on your hardware and report measured ranges.

| Component | Compute | Notes |
|-----------|---------|-------|
| 3DGS scene fit | Benchmark | Depends on capture quality, views, and training config |
| GWM inference | Benchmark | Runtime/model dependent; do not assume “faster than WAN” without matched settings |
| Training GWM | Benchmark | Depends on dataset scale and architecture |

### 17.8.3 Genie 3 / SIMA 2 (DeepMind)

Treat environment generators as an optional, separate track. Availability and capabilities vary; verify access/licensing and treat the generator’s physics fidelity as part of the experimental condition.

## 17.9 Process Reward Models (Very High Data Requirements)

### 17.9.1 GRM (General Reward Model — Robo-Dopamine)

**Scope note:** Training large process reward models is typically data- and compute-intensive and is out of scope for PID‑VLA. Use pre-trained weights as a baseline only if they are available under an acceptable license.

**Data Source for Reproduction:**
- The GRM paper uses diverse robot manipulation videos
- Collecting 35M samples is a major data engineering effort
- Alternative: Use pre-trained GRM weights if released

**For PID-VLA (Baseline Use Only):**
- Use pre-trained GRM for failure prediction baseline
- Do NOT train GRM from scratch (out of scope)

## 17.10 Failure Classifiers (Moderate Requirements)

### 17.10.1 Learned Failure Predictor (Baseline)

**Training requirements (measurement-first):**
- A lightweight supervised model (e.g., logistic regression/MLP) over logged representations and/or PID features.
- A labeled dataset with clear success/failure definitions and split protocol.
- Report measured training cost and calibration/robustness metrics; avoid generic time/memory claims.

**Data Sources:**
| Approach | Effort | Labels |
|----------|--------|--------|
| LIBERO benchmark | Low | Automatic (task completion) |
| VLA-Arena | Low | Automatic (structured tasks) |
| Manual annotation | High | Human-labeled failure modes |
| Simulation auto-label | Medium | Physics-based success criteria |

## 17.11 Depth Estimation (High Compute for Training)

### 17.11.1 Depth-Anything (Inference Only)

**Training:** None — use pre-trained.

**Inference (benchmark on your hardware):**
| Model | Notes |
|-------|-------|
| Depth-Anything v2 (or similar) | Relative depth; validate whether calibration is required for your tasks |
| Metric depth baseline (e.g., Metric3D) | Use if absolute scale is required; validate and benchmark |

### 17.11.2 DKT (Diffusion Knows Transparency)

Treat diffusion-based depth for transparent/reflective objects as an optional perception preprocessor. Use pre-trained models if available and benchmark on your scenes; avoid committing to base-model/dataset/training-time claims in this document without a primary citation and a local benchmark.

## 17.12 Synthetic Data Generation

### 17.12.1 When Synthetic Data Is Sufficient

| Use Case | Synthetic OK? | Notes |
|----------|---------------|-------|
| **Experiment 0 (Estimator Validation)** | ✅ Yes | Required synthetic validation |
| **PID estimator development** | ✅ Yes | Gaussian/XOR systems |
| **VLA fine-tuning** | ⚠️ Partial | Sim-to-real gap |
| **Failure detection training** | ⚠️ Partial | Need diverse failure modes |
| **World model training** | ⚠️ Partial | Physics fidelity matters |

### 17.12.2 Synthetic Data Generation Tools

| Tool | Purpose | Compute | Realism |
|------|---------|---------|---------|
| **Isaac Sim** | Robot simulation | High (GPU required) | High |
| **Gazebo** | Robot simulation | Moderate | Moderate |
| **Mujoco** | Physics simulation | Low | Moderate |
| **Blender/Cycles** | Synthetic rendering | Moderate | Very High |
| **Procedural generators** | Domain randomization | Low | Variable |

### 17.12.3 Experiment 0 Synthetic Data Protocol

**Purpose:** Validate PID estimators at target dimensionality.

**Data Generation:**
```python
# No external data needed — generate in-memory
for d_total in [10, 100, 1000, 4096]:
    for n_samples in [1000, 5000, 10000, 50000]:
        # Signal variables (low-d, known structure)
        s1_signal = generate_redundant_source(d=5)
        s2_signal = generate_xor_source(d=5)
        target = generate_target(s1_signal, s2_signal)
        
        # Embed in high-d by concatenating noise
        s1 = concat([s1_signal, noise(d=d_total-5)])
        s2 = concat([s2_signal, noise(d=d_total-5)])
        
        # Run estimator validation
        ...
```

**Compute:** benchmark-dependent; record measured runtime and peak memory for your chosen grid size and backend.

## 17.13 Data Collection Strategy Summary

### 17.13.1 Recommended Data Collection Priority

| Priority | Data Type | Source | Effort | Required For |
|----------|-----------|--------|--------|--------------|
| **1** | Synthetic validation | Generate yourself | Very Low | Experiment 0 |
| **2** | VLA-Arena trajectories | Public download | Low | Experiments 1-4 |
| **3** | LIBERO/SimplerEnv | Public download | Low | Baseline comparison |
| **4** | Open-X Embodiment subset | Public download | Low-Medium | VLA fine-tuning |
| **5** | SAE training activations | Run frozen VLA | Medium | SAE analysis (§16.8) |
| **6** | Custom robot demos | Collect yourself | High | Domain-specific |

### 17.13.2 Total Data Requirements Estimate

Data volume is highly dependent on what you log (raw video vs embeddings vs flows), representation precision, compression, and episode count. Treat storage as an engineering variable: prefer chunked, compressed formats and log only the representations you analyze (see `EXPERIMENTS.md` §11).

## 17.14 Compute Budget Recommendations

### 17.14.1 Minimum Viable Setup (PhD Project)

**Measurement-first recommendation:** start with the hardware you have, run Experiment 0, and measure the actual bottlenecks (PID kNN, embedding extraction, video prediction, flow extraction). Upgrade only for the bottleneck you cannot mitigate via caching, reduction, or offline scheduling.

### 17.14.2 Compute Time Estimates

Do not cite generic time/cost estimates in this document. Record measured runtime ranges (median/p95), peak memory, and configuration for each pipeline stage and report those in your experiment manifests.

### 17.14.3 What NOT to Train (Out of Scope)

| Component | Why Out of Scope | Alternative |
|-----------|-----------------|-------------|
| VLA from scratch | Training dynamics are not the object here | Use pre-trained VLAs |
| Large PRM/GRM from scratch | Data/compute heavy; separate research program | Use pre-trained baseline if available |
| Video foundation model from scratch | Data/compute heavy; separate research program | Use pre-trained predictor; treat as experimental variable |
| Environment generators | Adds confounds and scope | Treat as optional future work |

## 17.15 Data Access and Licensing

Do not assume licenses from memory. For every dataset/model you use, record:
- the license (or lack thereof),
- access conditions/registration,
- exact version/commit and checksum,
- and whether redistribution is permitted.
If you cannot legally redistribute an artifact required to reproduce the study, treat that as a publication blocker.

## 17.16 Critical Challenges and Mitigations

### 17.16.1 Challenge: kNN at d=4096

**Problem:** Brute-force kNN is O(N²d), prohibitive at d=4096.

**Mitigations:**
1. PCA to d=256 (recommended first attempt)
2. GPU-accelerated kNN (cuML, FAISS)
3. Approximate kNN (with Experiment 0 re-validation)
4. SAE decomposition (reduce effective dimension)

### 17.16.2 Challenge: Strong Dependence (Near-Deterministic VLA)

**Problem:** VLA `A = f(V,D,L)` is nearly deterministic; MI can be huge/undefined.

**Mitigations:**
1. Add calibrated noise to action predictions
2. Use temperature in VLA decoding
3. Target external labels (`A*`, success/failure) instead of raw action
4. Strong-dependence synthetic sweep in Experiment 0

### 17.16.3 Challenge: Trajectory Autocorrelation

**Problem:** Consecutive timesteps are correlated; "N frames" ≠ "N i.i.d. samples".

**Mitigations:**
1. Cross-trajectory sampling (one sample per rollout)
2. Large-stride subsampling (every 10th frame)
3. Block bootstrap for uncertainty estimation
4. Trajectory-level PID features (mean/min over windows)

## 17.17 Dream2Flow Integration Requirements (v6.2)

This section provides detailed computational and data requirements for the unified Dream2Flow + WAN + PID + Gaussian Splatting architecture described in §10.10.

### 17.17.1 Vision Foundation Models (Inference Only)

This stage uses three model *classes*: segmentation, point tracking, and depth. Specific implementations change quickly; treat any named models as examples and benchmark on your hardware.

| Category | Example models (verify availability/licensing) | Outputs to log | Notes |
|----------|-----------------------------------------------|----------------|-------|
| Segmentation | SAM2 or equivalent promptable segmenter | masks, prompts, per-object confidence | Often run once per clip; propagate masks if supported |
| Point tracking | CoTracker (or equivalent) | 2D tracks, confidence, failure cases | Runtime depends on number of points and clip length |
| Depth | Depth-Anything v2 (relative) and/or a metric-depth baseline (e.g., Metric3D), plus RGB-D when available | depth maps + calibration metadata | Metric depth typically needs calibration; handle transparents separately |
| Transparent depth (optional) | DKT (or other transparency-aware depth) | depth + uncertainty | Only needed for glass/plastic/etc. scenes |

**Measurement protocol:** For every run, record wall-clock time per frame/clip, peak memory, and model version/commit hash. Do not report fixed ms/frame values without measurement.

### 17.17.2 Video Prediction (Local or API)

Video prediction is typically the dominant compute in a Dream2Flow‑style pipeline and is treated as **offline** for PID‑VLA (unless you have verified real‑time capability on your hardware).

Model choice (and whether inference is local or API‑hosted) is an **experimental variable**; do not assume any particular latency/cost/quality.

**What to log (minimum):**
- Predictor name + version/commit; weights checksum (local) or provider/model-id (API)
- Conditioning inputs (image(s), instruction, optional action), prompts, seeds, sampler settings, clip length/resolution
- Wall‑clock time, peak VRAM/RAM, failure modes (OOM/timeouts), output hashes for caching
- If API‑hosted: request IDs, rate limits, costs, and any provider‑side versioning metadata

**Reproducibility guidance:**
- Prefer pinned local inference when possible; otherwise cache returned clips and treat the provider as non‑stationary.
- Treat the generated clip as a *proposal*, not a label; validate downstream Flow plausibility against simulator/sensor trajectories when available (§9.7.7).

**Optional motion/action conditioning:**
- Motion control methods (e.g., latent trajectory guidance) can support counterfactual “what‑if” generation if supported by the chosen predictor; verify compatibility per method (e.g., Wan‑Move, arXiv:2512.08765) before committing engineering time.

### 17.17.3 3D Flow Reconstruction (Segmentation/Tracking/Depth)

Flow reconstruction is a **measurement pipeline**. Its failure modes are perception confounds that must be logged and stratified (do not interpret PID results if Flow targets are unreliable).

**Minimum steps (model-agnostic):**
1. Segment objects (or select points) to define object identities.
2. Track points through the generated clip; log confidence and track failures.
3. Estimate depth/spatial cues (or use RGB‑D if available); record calibration/scale handling.
4. Lift 2D tracks to 3D trajectories and aggregate into a Flow target representation (e.g., low‑D per‑object statistics vs full \(3T\) trajectories).

**Performance note:** runtime depends on clip length, number of tracked points, and chosen models; report measured wall‑clock and peak memory.

### 17.17.4 PID Analysis (Rust, CPU)

PID runtime depends on sample size \(n\), working dimension \(d\), kNN backend, and how many terms you compute. Do not report fixed ms/estimate without measured hardware + configuration details.

**Practical guidance:**
- Use Shannon invariants (CI/Ω) for continuous screening; trigger full `I^sx_∩` decomposition only on selected windows/episodes.
- Treat any Flow representation as a new target variable: re-run geometry diagnostics + Experiment 0 on the exact preprocessing pipeline you plan to publish (§16.11, §9.1).

### 17.17.5 Gaussian Splat Visualization (Real-Time)

| Component | Target | Notes |
|-----------|--------|-------|
| SparkJS rendering | Interactive frame rate | WebGPU-capable GPU; benchmark on your scene complexity and splat count |
| PID-colored overlays | Interactive for small scenes; offline for heavy runs | GPU budget depends on splat count + shader complexity + update rate |
| Timeline scrubbing | Responsive interaction | Depends on caching strategy + IO; measure end-to-end |
| Multi-view | Responsive interaction | Scene complexity and GPU memory are the limiting factors |

**Browser requirements:**
- Recent Chrome/Edge/Firefox with WebGPU support enabled
- Sufficient GPU VRAM for your scene size (benchmark-dependent)
- Sufficient system RAM for cached assets and logs (benchmark-dependent)

### 17.17.6 Full Pipeline Per-Trial Summary

Do not treat any per‑trial time/VRAM/cost numbers as stable. For every run, log:
- stage-level wall-clock time (median/p95),
- peak memory (RAM/VRAM),
- and configuration (model IDs, resolution, frames, seeds, tracker settings).
Assume offline-first unless you have demonstrated interactive throughput on your hardware.

### 17.17.7 Hardware Recommendations

Hardware requirements depend on which components you run locally:
- **Estimator-only (PID on embeddings):** CPU + enough RAM for kNN workloads; GPU not required.
- **Embedding extraction / video prediction / flow extraction:** typically benefits from GPU acceleration; exact VRAM requirements depend on the chosen models and resolutions.
- **Visualization:** any WebGPU-capable GPU can work, but large scenes require more VRAM.
Prefer profiling-first: run a small pilot, record peak memory and latency, then scale.

### 17.17.8 Data Requirements

Minimum required artifacts are:
- observations (RGB or RGB‑D),
- task instructions/prompts,
- success/failure labels or external targets,
- executed trajectories,
- and the extracted representations used for PID.
Exact volumes depend on episode length, frame rate, representation precision, and how much you cache; measure and report.

### 17.17.9 What NOT to Build (Out of Scope)

| Component | Why Out of Scope | Alternative |
|-----------|------------------|-------------|
| Custom video model | Separate high-compute research program | Use a pre-trained predictor; treat as experimental variable |
| Custom depth model | Not core contribution | Use Depth-Anything/DKT |
| Custom tracking model | Not core contribution | Use CoTracker (or equivalent) |
| Real robot experiments | High cost, safety, time | Use Gazebo simulation |
| Large-scale RL training | Separate research program | Small-scale validation only |

---

# 18. Critical Blockers and Risk Analysis

This section provides a systematic assessment of blockers that could prevent project success, organized by severity. Each blocker includes explicit mitigation strategies and Go/No-Go decision criteria.

## 18.1 Executive Summary: Risk Classification

| Category | Count | Project Impact |
|----------|-------|----------------|
| **Show-Stoppers (Cat 1)** | 5 | Could kill project entirely |
| **Major Blockers (Cat 2)** | 7 | Require significant scope pivots |
| **Minor Blockers (Cat 3)** | 8 | Workarounds exist; manageable |

**Overall Risk Assessment:** HIGH but tractable. Success depends critically on Experiment 0 outcomes and DreamVLA availability.

---

## 18.2 Category 1: Show-Stopper Blockers

These blockers could terminate the project if unmitigated. Each requires explicit Go/No-Go decision before proceeding.

### 18.2.1 Experiment 0 Fails at d ≤ 256

**Risk:** kNN-based `I^sx_∩` may be fundamentally unreliable even after dimensionality reduction.

**Evidence for Concern:**
- Distance concentration is exponential in dimension
- No published validation of continuous `I^sx_∩` at d > 100
- Ehrlich et al. 2024 validation was on low-d synthetic data only

**Detection:**
- Experiment 0 error > 20% at d=256 after PCA
- Synthetic XOR/redundancy patterns unrecoverable
- Variance across seeds exceeds signal magnitude

**Mitigation Options:**
1. **PIVOT to Shannon invariants only:** Use CI = I(X;T) + I(Y;T) - I(X,Y;T) for screening; abandon `I^sx_∩` atoms
2. **PIVOT to discrete PID:** Quantize embeddings to k=100-1000 clusters; use discrete `I^sx_∩` (Makkeh et al. 2021)
3. **PIVOT to neural MI:** Use MINE/CCMI for MI estimation; compute CI from neural MI (but NOT `I^sx_∩`)

**Go/No-Go Decision:**
- **GO:** Error < 15% at d=256 after PCA, stable across seeds
- **PIVOT:** Error 15-30% at d=256; switch to discrete or CI-only
- **NO-GO:** Error > 30% at d=256; fundamental approach failure → publish as negative result

**Timeline Risk:** This gate must be cleared in Month 1-2. Failure here invalidates all downstream experiments.

---

### 18.2.2 DreamVLA Weights/Architecture Unavailable

**Risk:** DreamVLA (arXiv:2507.04447) is the only VLA with explicitly extractable world model states ("D"). If unavailable, the V-D-A decomposition becomes impossible.

**Evidence for Concern:**
- Abstract does not specify backbone family/dimensions or hidden sizes; implementation details may be insufficient for re-implementation.
- Weight release/hosting status is unknown; verify upstream availability and licensing before depending on DreamVLA.

**Detection:**
- Unable to download/access DreamVLA weights
- Authors unresponsive to access requests
- Architecture details insufficient for re-implementation

**Mitigation Options:**
1. **PIVOT to OpenVLA hidden states:** Use internal layer activations as "D" proxy (less clean, but extractable)
2. **PIVOT to V-L-A only:** Abandon world model decomposition; focus on language grounding (still scientifically valuable)
3. **Re-implement DreamVLA:** Only if the architecture and training recipe are sufficiently specified and you can publish a reproducibility package
4. **Contact authors directly:** Request pre-publication access for research purposes

**Go/No-Go Decision:**
- **GO:** Weights accessible OR architecture fully specified for re-implementation
- **PIVOT:** Use OpenVLA hidden states as D proxy (weaker but tractable)
- **NO-GO:** No V-D-A decomposition possible → reframe project as V-L-A analysis only

**Timeline Risk:** Must resolve by Month 1. Downstream experiment design depends on this.

---

### 18.2.3 Strong Dependence Makes MI Unbounded

**Risk:** VLA action outputs are nearly deterministic functions of inputs. For deterministic f, I(X; f(X)) = H(X) which can be arbitrarily large or undefined for continuous X.

**Evidence for Concern:**
- VLA decoding is typically argmax (deterministic)
- Even with temperature, entropy of action distribution may be very low
- Gao et al. 2015 shows KSG MI estimators fail catastrophically in strong-dependence regimes

**Detection:**
- MI estimates grow without bound as k decreases
- Estimates are unstable across seeds (high variance)
- Strong-dependence synthetic sweep in Experiment 0 fails

**Mitigation Options:**
1. **Add calibrated noise:** Inject noise to action outputs before MI estimation (but this changes the quantity being estimated)
2. **Use temperature decoding:** Force VLA to output distributions, not argmax
3. **Target external labels:** Estimate I(V,L,D; Success) where Success is a binary external label (avoids determinism)
4. **Use binned/discrete actions:** Discretize action space to 256-1000 bins (as in OpenVLA); use discrete MI

**Go/No-Go Decision:**
- **GO:** Strong-dependence sweep in Experiment 0 shows bounded MI with actionable mitigations
- **PIVOT:** Switch to discrete targets (Success/Failure) or binned actions
- **NO-GO:** Continuous action-based PID fundamentally impossible → reframe to classification/reward prediction

**Timeline Risk:** Detectable in Experiment 0 (Month 1-2).

---

### 18.2.4 i.i.d. Assumption Fundamentally Violated

**Risk:** PID estimators assume i.i.d. samples. VLA rollouts are temporally correlated; consecutive frames are nearly identical.

**Evidence for Concern:**
- Adjacent frames in a trajectory share >90% visual content
- Action sequences are smooth (kinematic constraints)
- "10,000 frames" may represent only 100 effective i.i.d. samples

**Detection:**
- Effective sample size (ESS) << nominal N
- Block bootstrap CIs are 10x wider than naive CIs
- Estimates change dramatically with subsampling stride

**Mitigation Options:**
1. **Cross-trajectory sampling:** One sample per rollout (approximately independent if rollouts are independently reset/seeded)
2. **Large-stride subsampling:** Every 10th-30th frame within trajectory
3. **Block bootstrap:** Use trajectory-aware blocks for uncertainty quantification
4. **Trajectory-level features:** Compute mean/min/max PID over windows; treat trajectory as single sample

**Go/No-Go Decision:**
- **GO:** ESS/N > 0.1 with reasonable subsampling
- **PIVOT:** Switch to cross-trajectory only (requires more rollouts)
- **NO-GO:** Insufficient independent samples available → collect more data or abandon temporal claims

**Timeline Risk:** Detectable after first VLA embedding extraction (Month 3-4).

---

### 18.2.5 All Baselines Beat PID

**Risk:** Simpler baselines (entropy, uncertainty, GRM reward) may outperform PID for failure prediction, rendering the contribution trivial.

**Evidence for Concern:**
- Entropy baselines are well-established and cheap
- GRM (Robo-Dopamine) already achieves strong failure prediction
- PID adds complexity without guaranteed benefit

**Detection:**
- PID-based classifier AUROC ≤ baseline + 0.02
- No statistically significant improvement
- PID atoms do not provide interpretable signal

**Mitigation Options:**
1. **Reframe as diagnostic tool:** Even if not predictive, PID may explain *why* failures occur (interpretability value)
2. **Combine PID + baselines:** Ensemble approach where PID provides additional signal
3. **Focus on modality attribution:** PID tells *which modality* failed even if overall prediction is similar to baselines
4. **Publish as negative result:** Valuable contribution if rigorously conducted

**Go/No-Go Decision:**
- **GO:** PID provides statistically significant improvement OR unique interpretability
- **PIVOT:** Reframe as diagnostic/interpretability tool rather than predictor
- **NO-GO:** No advantage over baselines AND no interpretability → publish negative result

**Timeline Risk:** Discoverable only after Experiment 2 (Month 5-7).

---

## 18.3 Category 2: Major Blockers

These blockers require significant scope pivots but do not terminate the project.

### 18.3.1 Geodesic kNN MI Not Implemented

**Status:** NOT implemented in `pid-core`

**Impact:** Cannot test manifold-aware MI estimation directly. Limited to Euclidean approximations.

**Mitigation:** Use Isomap/contractive AE to "unroll" manifold first, then apply standard KSG. OR use PCA and accept linear approximation. Geodesic kNN is a future enhancement.

**Priority:** LOW for v1.0 experiments.

---

### 18.3.2 Ollivier-Ricci Curvature Not Implemented

**Status:** NOT implemented in `pid-core`

**Impact:** Cannot directly test local curvature of embedding spaces. Must rely on δ-hyperbolicity and indirect diagnostics.

**Mitigation:** Use δ-hyperbolicity (4-point condition) as proxy. Ollivier-Ricci is computationally expensive (O(n²) optimal transport per edge) anyway.

**Priority:** LOW for v1.0 experiments.

---

### 18.3.3 No Hyperbolic `I^sx_∩` Estimator Exists

**Status:** NO mathematical derivation exists for `I^sx_∩` in hyperbolic geometry

**Impact:** Cannot directly apply hyperbolic projections for full PID analysis. The v5.5 warning explicitly forbids this.

**Mitigation:** 
1. Use hyperbolic geometry for hierarchy visualization ONLY
2. Use hyperbolic projections for MI-only screening (geodesic MI is valid)
3. Apply "unrolling" approaches before `I^sx_∩` estimation

**Priority:** HIGH conceptual barrier; must be clearly communicated in all publications.

---

### 18.3.4 Pixel-160K Dataset Access TBD

**Status:** Access not publicly announced (as of Jan 2026)

**Impact:** Cannot replicate PixelVLA experiments with original training data.

**Mitigation:** Use alternative VLAs (OpenVLA, TraceVLA) on publicly available benchmarks (LIBERO, VLA-Arena).

**Priority:** MEDIUM; affects PixelVLA-specific experiments only.

---

### 18.3.5 GRM Weights May Not Be Released

**Status:** Robo-Dopamine paper does not guarantee weight release

**Impact:** Cannot use GRM as baseline without retraining (prohibitive: 35M samples).

**Mitigation:**
1. Contact authors for weights
2. Use simpler reward model baselines (GVL, VLAC)
3. Train lightweight reward classifier on smaller data

**Priority:** MEDIUM; affects Experiment 2 baseline comparisons.

---

### 18.3.6 No Ground Truth for "World Model Quality"

**Status:** No external metric for validating world model representations

**Impact:** Cannot prove that PID measures "world model quality" rather than something else.

**Mitigation:**
1. Use controlled interventions (change D, observe behavior changes)
2. Correlate PID with downstream task success
3. Treat "world model quality" as latent construct, not ground truth

**Priority:** HIGH conceptual issue; requires careful framing in publications.

---

### 18.3.7 Document Scope Exceeds PhD Timeline

**Status:** grandplan.md describes ~5 years of work

**Impact:** Risk of scope creep, never finishing, burnout.

**Mitigation:**
1. Strict prioritization: Experiments 0-2 are core; 3-4 are stretch goals
2. Aim 3 (RL fine-tuning) is explicitly optional
3. VLA-Arena integration is additive, not required
4. World model integration (§10) is future work

**Priority:** HIGH; requires active scope management.

---

## 18.4 Category 3: Minor Blockers

These blockers have known workarounds and are manageable.

| Blocker | Status | Workaround |
|---------|--------|------------|
| SAE training for VLA | Not implemented | Use pre-trained SAE (Jiang et al. 2025) |
| MINE must be retrained per estimate | Known limitation | Pre-train on VLA distribution once |
| Isomap expensive (O(n³)) | Known | Use sparse Isomap or landmarks |
| Full VLA pre-training infeasible | Known | Use LoRA fine-tuning only |
| Some arXiv papers not peer-reviewed | Known | Verify claims independently |
| macOS primary limits CUDA | Known | NixOS secondary target available |
| PyO3 bindings not implemented | Planned | Python-first for experiments |
| Ball-tree/KD-tree not implemented | Known | Brute-force kNN is reference-correct |

---

## 18.5 Risk Mitigation Timeline

| Month | Gate | Critical Decision |
|-------|------|-------------------|
| 1 | DreamVLA access | GO (available) / PIVOT (OpenVLA hidden states) |
| 1-2 | Experiment 0 | GO (< 15% error) / PIVOT (CI-only) / NO-GO |
| 2-3 | Strong dependence | GO (bounded) / PIVOT (discrete targets) |
| 3-4 | ESS estimation | GO (ESS/N > 0.1) / PIVOT (cross-trajectory) |
| 5-7 | Baseline comparison | GO (PID adds value) / PIVOT (interpretability) / NO-GO (negative result) |

---

## 18.6 Fallback Scope Hierarchy

If blockers force scope reduction, reduce in this order (highest priority first):

1. **Core (MUST complete):**
   - Experiment 0: Estimator validation
   - V-L-A decomposition on OpenVLA
   - Basic failure prediction comparison

2. **Important (SHOULD complete):**
   - V-D-A decomposition (if DreamVLA available)
   - VLA-Arena integration
   - Baseline comparison (Experiment 2)

3. **Stretch (COULD complete):**
   - Experiment 3: Dimensionality study
   - Experiment 4: Causal validation
   - PixelVLA/TraceVLA analysis

4. **Future work (WON'T in PhD):**
   - Aim 3: RL fine-tuning with PID reward
   - Full 3-way PID (18 atoms)
   - Custom world model training

---

## 18.7 Decision Framework Summary

```
BLOCKER RESOLUTION PROTOCOL
===========================

1. Identify blocker category (1/2/3)
2. Check detection criteria
3. If detected:
   a. Cat 1 → Immediate Go/Pivot/No-Go decision
   b. Cat 2 → Apply mitigation; document scope change
   c. Cat 3 → Apply workaround; continue
4. Document all decisions in Appendix B
5. Update timeline and deliverables
```

**Final Note:** This analysis assumes honest scientific inquiry. If Experiment 0 fails, that is a legitimate (and publishable) finding about the limits of continuous `I^sx_∩` estimation. The goal is truth, not confirmation of hypotheses.

---

# Appendix A: Glossary

| Term | Definition |
|------|------------|
| **VLA** | Vision-Language-Action model |
| **PID** | Partial Information Decomposition |
| **I^sx_∩** | Shared-exclusions redundancy measure |
| **Synergy** | Information available only from multiple sources together |
| **Redundancy** | Information available from any single source |
| **KSG** | Kraskov-Stögbauer-Grassberger MI estimator |
| **3DGS** | 3D Gaussian Splatting |
| **GWM** | Gaussian World Model |
| **WAN** | Wanxiang video generation model (Alibaba) |
| **VACE** | Video All-in-one Creation and Editing (WAN extension) |
| **MoE** | Mixture of Experts architecture |
| **DiT** | Diffusion Transformer |
| **CI** | Co-Information (Shannon invariant) |
| **Ω** | O-Information (generalized co-information) |
| **GPID** | Gaussian Partial Information Decomposition |
| **LoRA** | Low-Rank Adaptation (fine-tuning method) |
| **Zenoh** | Zero-overhead pub/sub middleware for robotics |
| **NanoGPT** | Minimal GPT-2 training codebase (Karpathy) |
| **StereoVLA** | VLA enhanced with stereo vision |
| **DKT** | Diffusion Knows Transparency (transparent object depth) |
| **PRM** | Process Reward Model (dense progress-based rewards) |
| **GRM** | General Reward Model (Robo-Dopamine's step-aware PRM) |
| **ORM** | Outcome Reward Model (sparse success/failure rewards) |
| **VOC** | Value-Order Consistency (PRM evaluation metric) |
| **PBRS** | Potential-Based Reward Shaping |
| **Genie 3** | DeepMind's general-purpose interactive world model |
| **SIMA 2** | Scalable Instructable Multiworld Agent (DeepMind) |
| **TransPhy3D** | Synthetic transparent object video dataset (11k scenes) |
| **Emergent Physics** | Physics learned via self-supervision, not hardcoded |
| **PixelVLA** | VLA with pixel-level understanding and visual prompting |
| **TraceVLA** | VLA with visual trace prompting for spatial-temporal awareness |
| **Multiscale Pixel-Aware Encoder** | PixelVLA component for pixel-level feature injection |
| **Visual Prompting Encoder** | PixelVLA component for processing points, masks, regions |
| **Pixel-160K** | PixelVLA's pixel-annotated visuomotor dataset (160K trajectories) |
| **sae_analysis** | Makkeh's Shannon invariant toolkit for SAE analysis |
| **Red° (Degree of Redundancy)** | Shannon invariant: avg. extent info accessible from multiple sources |
| **Vul° (Degree of Vulnerability)** | Shannon invariant: avg. extent info lost when sources removed |
| **Dream2Flow** | Framework bridging video generation and robot control via 3D object flow |
| **3D Object Flow** | Explicit 3D point trajectories extracted from video (embodiment-agnostic) |
| **SAM2** | Segment Anything Model 2 (Meta) — promptable segmentation; use for video mask initialization |
| **CoTracker** | Point tracking model — tracks 2D points through video frames (use the latest available release) |
| **Wan-Move** | WAN LoRA fine-tuning for robot action conditioning |
| **PID-Colored Splats** | Gaussian splats with RGB = (Synergy, Redundancy, Unique) for visualization |
| **Embodiment Gap** | Mismatch between intended action and physical execution due to robot differences |
| **D_wan** | WAN's internal hidden states used as world model proxy |

---

# Appendix B: Decision Log and Implementation Reference

**Scope note:** This appendix contains decision history and engineering sketches. Treat any hardware, performance, or cost numbers as historical placeholders unless they are tied to a primary citation or a committed benchmark in this repo; benchmark on your setup before using.

## B.1 Decision Log (Detailed)

### Decision 1: Discard OpenVLA vs DreamVLA Comparison

| Attribute | Value |
|-----------|-------|
| **Date** | December 2025 |
| **Status** | REJECTED |
| **Category** | Experimental Design |

**Original Proposal:**
Compare PID decomposition signatures between OpenVLA (no explicit world-model output) and DreamVLA (explicit world‑knowledge prediction outputs) to argue that architectures with explicit “D” have higher synergy.

**Why Rejected:**

1. **Confounds are insurmountable:** The architectures differ in backbone (Llama 2 vs backbone unspecified in DreamVLA’s abstract), training data, action representation, and attention/conditioning design. Any observed PID difference could be attributed to any of these factors.

2. **Circular reasoning risk:** If we define "D" differently for each architecture (hidden states for OpenVLA, explicit world‑knowledge outputs for DreamVLA), we may end up measuring the operationalization rather than an intrinsic property.

3. **No ground truth:** We have no independent measure of "world model quality" to validate against. We'd be correlating one unknown (PID signature) with another unknown (implicit world model strength).

4. **Publication risk:** Reviewers would correctly identify these confounds and reject the comparison as methodologically unsound.

**Alternative Adopted:**
Focus on within-architecture analysis wherever possible. Treat DreamVLA-style explicit world‑knowledge prediction as a strong candidate when available (because “D” is more operationalizable), but do not rely on model availability; maintain a V‑L‑A and/or Flow‑as‑Bridge path as the primary fallbacks.

---

### Decision 2: Elevate V-L-A to Co-Primary Status

| Attribute | Value |
|-----------|-------|
| **Date** | December 2025 |
| **Status** | ADOPTED |
| **Category** | Decomposition Strategy |

**Original Proposal:**
V-D-A (Vision-Dream-Action) as primary decomposition, with V-L-A as secondary.

**Why Changed:**

1. **L is externally specified intent:** Language instructions are human-provided and are often the closest available “ground truth” for task intent, but they can still be ambiguous/underspecified. “Ignoring L” must be operationalized carefully (dataset semantics, annotation policy, and task context).

2. **D is model-internal:** The "Dream" representation is whatever the model learned. It might be wrong, incomplete, or encode biases. Using D as a reference conflates model failures with reference failures.

3. **Language grounding failures are common:** Empirical observation shows VLAs often execute plausible-but-wrong actions that ignore instruction specifics (e.g., "pick up the RED cup" → picks up nearest cup regardless of color).

4. **Direct interpretability:** Low Syn_{V,L→A} immediately suggests "model isn't integrating vision with language instruction." Low Syn_{V,D→A} is harder to interpret because D is opaque.

**Current Status:**
- V-L-A: Co-primary (recommended starting point)
- V-D-A: Co-primary (for DreamVLA specifically)
- V-L-D-A: Three-way analysis after pairwise validation

---

### Decision 3: Recommend Hierarchical Pairwise PID

| Attribute | Value |
|-----------|-------|
| **Date** | December 2025 |
| **Status** | ADOPTED |
| **Category** | Estimation Strategy |

**Original Proposal:**
Compute full three-way PID I(V, L, D; A) with 18 atoms from the start.

**Why Changed:**

1. **Estimation cost:** 18 atoms require many kNN-based estimates. With exact/brute-force kNN this scales at least like O(n²·d) per estimate; at VLA scale (d≈4096), this becomes prohibitively expensive without aggressive dimensionality reduction and/or accelerated kNN.

2. **Interpretation burden:** Most of the 18 atoms have no clear operational meaning. What does "information uniquely provided by V, but redundantly available in L and D" mean for robot control?

3. **Variance multiplication:** Each additional atom adds estimation variance. With 18 atoms, confidence intervals become uselessly wide.

4. **Pairwise captures most value:** The key insights (which source dominates? is there synergy or subadditivity?) are available from pairwise decompositions.

**Recommended Hierarchical Strategy:**

```
Level 0: Shannon invariants (fastest; MI-only)
├── Compute CI_VL, CI_VD, CI_LD (co-information)
├── Use for: Real-time monitoring, screening
└── Proceed to Level 1 if: Any CI is suspicious (outside normal range)

Level 1: Pairwise PID (slower; targeted)
├── Compute full I^sx_∩(V, L; A) or I^sx_∩(V, D; A)
├── Use for: Failure diagnosis, architecture comparison
└── Proceed to Level 2 if: Need three-way interactions

Level 2: Three-way PID (offline only)
├── Compute full I^sx_∩(V, L, D; A)
├── Use for: Detailed post-hoc analysis, publication figures
└── Only after pairwise validation complete
```

**Latency note:** Any ms-level budgets depend strongly on `(n,d,k)` and on the kNN backend. Brute-force exact kNN is not real-time at large `n`; treat timings as design targets, not guarantees.

---

### Decision 4: Mandate Experiment 0 First

| Attribute | Value |
|-----------|-------|
| **Date** | December 2025 |
| **Status** | MANDATORY |
| **Category** | Validation Protocol |

**Original Proposal:**
Start with VLA experiments immediately, validate estimator in parallel.

**Why Changed:**

1. **Unknown operating regime:** The continuous I^sx_∩ estimator (Ehrlich et al., 2024) was validated on d≤100. VLA embeddings are d=4096. We have no evidence it works at this scale.

2. **Garbage in, garbage out:** If the estimator produces nonsense at d=4096, all downstream conclusions are invalid. We'd waste months chasing artifacts.

3. **Fast validation:** Synthetic data experiments take days, not months. The cost of validation is low; the cost of skipping validation is potentially the entire project.

4. **Publishable regardless of outcome:** If Experiment 0 shows the estimator fails at high dimensions, that's a valid contribution to the PID literature.

**Experiment 0 Protocol:**

```python
# Experiment 0 is about estimator validity, not a priori “truth” claims at d=4096.
# Use i.i.d. synthetic systems + noise-dimension embeddings where true information
# quantities are invariant to added nuisance dimensions.

for dim in [64, 256, 1024, 4096]:
    for n_samples in [1000, 5000, 10000, 50000]:
        # 1) Generate low-d "signal" variables (e.g., 1–10 dims).
        # 2) Concatenate independent noise dims to reach `dim`.
        # 3) Compare estimates against reference values computed on the signal system
        #    (cross-checked with `csxpid` for redundancy and analytic MI where available).
        pass
```

**Go/No-Go Criteria:**
- **GO:** Stable estimates under noise-dimension embeddings up to d=4096 with acceptable variance/runtime.
- **PIVOT:** Stable only after dimensionality reduction (e.g., PCA to ~256) → adopt reduction + re-validate and proceed.
- **NO-GO:** Unstable even after reduction (or contradicts `csxpid` at low d) → treat kNN-based `I^sx_∩` as invalid for this regime and pivot to alternative diagnostics (e.g., Shannon invariants as primary).

---

### Decision 5: Recommend GWM over WAN for Analysis

| Attribute | Value |
|-----------|-------|
| **Date** | December 2025 |
| **Status** | ADOPTED (with caveats) |
| **Category** | World Model Integration |

**Context:**
Both GWM (Gaussian World Model) and WAN (Wanxiang) were considered for providing ground-truth world state predictions to validate against VLA internal representations.

**Comparison (capability notes; avoid declaring a “winner” without a matched benchmark):**

| Criterion | WAN (video foundation model) | GWM (Gaussian world model; if built) |
|-----------|------------------------------|--------------------------------------|
| Training distribution | Broad video (paper; not robot-specialized by default) | Robot/scene dependent (you choose) |
| Representation | 2D video frames | 3D (e.g., 3DGS / scene state) |
| Action conditioning | Possible via fine-tuning/conditioning schemes (verify) | Native by design (predicts next state from action) |
| Runtime / footprint | Benchmark-dependent | Benchmark-dependent |
| Visual quality | Often high (paper claims; benchmark-dependent) | Scene/capture dependent |
| Weights/code | Paper claims open release (verify licensing) | Research/implementation dependent |

**When to Use Each:**

- **GWM:** Core analysis, failure localization, training data augmentation
- **WAN:** Paper figures, demos, qualitative visualization
- **Neither:** Real-time intervention (both too slow; use entropy)

**Implementation Note:**
GWM integration requires 3DGS scene reconstruction, which adds pipeline complexity. For initial experiments, compute PID on VLA latents alone without external world model reference.

---

## B.2 Platform Implementation Reference

**v7.0 scope change:** earlier drafts included detailed, hardware-specific implementation sketches (Apple Silicon targets, CUDA targets, MLX/Metal/CoreML acceleration ideas). Those blocks were removed in v7.0 because they:
- contained unsourced hardware/performance assertions,
- described non-existent modules/scripts in this repository,
- and risked being cited as “requirements” despite being unbenchmarked.

**Canonical repo reality (authoritative):**
- Tooling: `flake.nix` / `flake.lock` and `justfile`
- Python deps: `pyproject.toml` / `uv.lock`
- Implemented code: `crates/pid-core` (estimators + geometry gates) and `crates/pid-python` (`pid_core_rs` bindings)

**Engineering guidance (measurement-first):**
- The PID estimators and geometry diagnostics run on CPU.
- GPU acceleration (CUDA/Metal) and deployment backends (CoreML/TensorRT) are optional engineering work and should only be specified once there is a committed implementation + benchmark protocol in this repo.

If you need the historical platform notes, consult git history prior to v7.0 and treat them as unverified until benchmarked.

## B.3 MLX Framework Integration

**Status (v7.0):** Not part of the implemented repo and not required for the scientific claims.

MLX integration is a reasonable future direction for running certain models locally on Apple Silicon, but any integration should be tracked as an engineering proposal with:
- a pinned dependency/version,
- a benchmark + correctness protocol,
- and a clear mapping to the experiment harness (logging/interventions/replay).

Until then, treat MLX as an external option and do not cite performance expectations from this document.

## B.4 Metal Compute for PID Estimation

**Status (v7.0):** Not implemented here.

GPU kernels for kNN/KSG/PID are plausible but easy to get subtly wrong (precision, tie handling, radius conventions). If pursued, add them as a separately tested backend with parity tests against `crates/pid-core` and publish benchmark scripts + measured results in-repo.

## B.5 CoreML for Quantized Inference

**Status (v7.0):** Not implemented here.

CoreML (and similar deployment runtimes) are relevant only once the project includes a committed inference stack. For PID‑VLA, the scientific bottleneck is estimator validity + experiment design, not a specific inference runtime.

## B.6 Nix Configuration for Reproducibility

**Avoid divergence:** the authoritative reproducibility config lives in the repo root:
- `flake.nix`, `flake.lock`
- `pyproject.toml`, `uv.lock`
- `justfile`

Earlier drafts embedded large “kitchen sink” Nix flakes and task runners inside this document. Those were removed in v7.0 so the documentation cannot drift away from the executable repo. If you need expanded Nix/CUDA scaffolding, add it to the repo as real files and reference them here.

## B.7 Version History

**Note:** Older entries may contain legacy performance/latency/cost figures and engineering sketches that were later removed or rewritten. Treat those as historical placeholders unless they are tied to a primary citation or a committed benchmark; consult git history for removed appendices; v7.0 adopts measurement-first language.

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | Dec 2025 | Initial specification |
| 2.0 | Dec 2025 | Comprehensive revision with critical analysis, discarded approaches, three-way PID discussion |
| 2.1 | Dec 2025 | Added six use cases, Shannon invariants section, dual-process theory framing |
| 2.2 | Dec 2025 | Added complete Apple M4 implementation reference (Appendix B) |
| 2.3 | Dec 2025 | Added existing PID code availability analysis, complete Rust I^sx_∩ implementation with 5 validation test scenarios, verified all content based on Wibral's PID (not older Williams & Beer I_min) |
| 2.4 | Dec 2025 | **Major update:** (1) Clarified WAN-related scope and conditioning options (VACE, Wan‑Move) without hard‑coding unverified “version”/speed claims. (2) Added comprehensive §B.3.5 on scaling 3‑way PID: Shannon invariants, Gaussian PID, NF‑PID (“Thin‑PID” legacy) via normalizing flows, coarse‑graining approaches. (3) Added new references for scalable PID methods and video‑based robotics baselines (Motus, DreamGen, VideoVLA) without asserting shared backbones. |
| 2.5 | Dec 2025 | **Additions:** (1) Added §10.4 Depth Perception Methods: monocular depth (Depth-Anything v2/v3, Metric3D v2, RollingDepth), stereo vision (StereoVLA approach from arXiv:2512.21970), transparent object depth (DKT). (2) Added Headless Gazebo + Tauri Visualization System with Zenoh middleware, SparkJS/Three.js rendering, ~25-30ms latency path, cross-platform ML backends (CoreML/MLX/Metal on macOS, CUDA/TensorRT on Linux). (3) Added NanoGPT/nanochat note to DreamVLA backbone section - clarified GPT-2 refers to pretrained architecture, NanoGPT useful for custom training. (4) Expanded references: Depth Estimation & 3D Perception, Simulation & Middleware, Training Infrastructure. (5) Updated glossary with Zenoh, NanoGPT, StereoVLA, DKT. |
| 2.6 | Jan 2026 | **Process Reward Models integration:** (1) Added §3.5 PID vs. Process Reward Models (PRMs) - comprehensive comparison of PID approach with Robo-Dopamine's General Reward Model (GRM), including when to use each, potential synergies, and the "semantic trap" insight for reward shaping. (2) Added GRM as baseline #7 in experimental design. (3) Added §13.6 Process Reward Models references (Robo-Dopamine, GVL, VLAC, SARM, LIV). (4) Updated glossary with PRM, GRM, ORM, VOC, PBRS terms. |
| 2.7 | Jan 2026 | **World model paradigms & DKT deep dive:** (1) Added §10.1 world model taxonomy (Internal/Evaluative/Generative) with Genie 3 as environment generator. (2) Expanded §10.4.3 DKT section with "Diffusion Knows Transparency" principle, technical details, robot grasping results, and genuine PID relevance (perception quality as prerequisite for valid PID). (3) Added §10.7 World Model Paradigms and PID Implications: theoretical framework for how external world models (Genie 3, WAN) affect internal D; "Diffusion Knows Physics" principle; perception quality diagnostic tree. (4) Added Genie 3, SIMA 2, Genie 2 to world models references. (5) Updated glossary with Genie 3, SIMA 2, TransPhy3D, Emergent Physics. (6) Renumbered sections 10.7→10.8 for Gazebo+Tauri. |
| 2.8 | Jan 2026 | **NixOS CUDA secondary target:** (1) Restructured §B.2 as "Platform Implementation Reference" with primary (Apple M4) and secondary (NixOS + CUDA) targets. (2) Added §B.2.4 NixOS + CUDA Implementation with complete configuration.nix for NVIDIA drivers, flake.nix with CUDA-enabled PyTorch and Rust toolchain, CUDA software stack diagram. (3) Added GPU-accelerated PID implementation: CUDAKSGEstimator and CUDAPIDEstimator classes with chunked distance computation for OOM prevention. (4) Added NixOS troubleshooting guide and multi-GPU configuration (NCCL). (5) Fixed §B.3 subsection numbering: B.3.5→B.3.3, B.3.6→B.3.4, B.3.7→B.3.5 with correct heading levels. |
| 2.9 | Jan 2026 | **PixelVLA integration & sae_analysis notes:** (1) Added §7.3 PixelVLA architecture: multiscale pixel-aware encoder, visual prompting encoder, continuous action decoder, Pixel-160K dataset. (2) Added §7.4 TraceVLA: visual trace prompting for spatial-temporal awareness. (3) Added §10.8.7 PixelVLA + Headless Gazebo + Tauri integration: data flow diagram, visual prompting in Tauri (TypeScript), PixelVLA-specific PID analysis (Rust), latency budget (~86ms interactive). (4) Added §B.3.3.2 Abzinger/sae_analysis: Shannon invariants (Red°, Vul°) for SAE analysis, comparison with our approach. (5) Updated §B.3.3.5 to clarify sae_analysis is **not** an `I^sx_∩` estimator; added implementation-level definitions of Red°/Vul° and safe integration guidance (SAE compression + screening), not a correctness validation for `I^sx_∩`. (6) Updated §7.5 with MemoryVLA, CoT-VLA. (7) Added PixelVLA, TraceVLA, sae_analysis to references (§13.2, §13.3). (8) Updated glossary with PixelVLA, TraceVLA, Red°, Vul°, multiscale pixel-aware encoder, Pixel-160K. |
| 3.0 | Jan 2026 | **First-principles audit pass:** (1) Reframed “synergy sign” as a falsifiable hypothesis (not a definition); clarified deterministic-target degeneracy in VLA decompositions and the need for external targets/counterfactuals. (2) Tightened estimator risk framing and strengthened Experiment 0 as a scientific gate before any VLA claims. (3) Added/expanded i.i.d. vs trajectory autocorrelation guidance (sampling unit, block bootstrap). |
| 4.0 (Draft) | Jan 2026 | **Audited + citation-verified pass:** (1) Added explicit reference verification policy and downgraded unsourced architecture/latency statements to “unverified sketches”. (2) Added strong-dependence warning (Gao et al. 2015) and integrated a Gaussian-channel strong-dependence sweep into Experiment 0. (3) Added MI/CMI estimator comparison section (Gao-LNC/local Gaussian, MINE, CCMI) strictly as MI/CMI baselines (do not mix estimator families inside SxPID identities). (4) Verified key VLA citations (notably DreamVLA) and added optional background papers (OpenVLThinker, SRL, diffusion parameterization). (5) Cleaned up NF-PID (“Thin-PID” legacy) naming and other citation/notation fixes. (6) Corrected/clarified Shannon-invariant definitions (CI sign conventions; Ω vs target co-information) and reconciled scaling sketches. (7) Aligned reproducibility guidance with repo-canonical `flake.nix` + `uv.lock` workflow (macOS-first). (8) Integrated differential-geometry contingencies into §8.1.5 without relying on a repo-local PDF. |
| 5.0 | Jan 2026 | **Final audit release:** Added confounding factors analysis (§14), numerical stability guidance (§15), manifold/PCA/kNN limitations (§16). Integrated information geometry methods and intrinsic dimension estimation. Code audit complete (implementation cross-checked). Grant-ready documentation with full provenance tracking. |
| 5.1-5.3 | Jan 2026 | **Refinements:** Clarified variable definitions for OpenVLA/DreamVLA, added scope for visual prompting/trace architectures, and distinguished source-count scaling (hierarchy) from estimator validity (geometry). |
| 5.4 | Jan 2026 | **VLA Integration:** Verified key VLA + Shannon-invariants citations (OpenVLA, DreamVLA, PixelVLA, TraceVLA). Clarified primary hypothesis vs. candidate sub-hypotheses and mapped them to aims. |
| 5.5 | Jan 2026 | **Critical Geometry Fix:** Documented that Wibral PID (`I^sx_∩`) on manifolds/Lorentz spaces requires new derivations. Added top-level warning against naive Euclidean application. |
| 5.6 | Jan 2026 | **Manifold Approaches (WIP):** Added Top 5 manifold-compatible engineering approaches (Unrolling, Geodesic MI, Linear Projection, Quantization, Copula Transform) to address the v5.5 discovery. |
| **5.7** | Jan 2026 | **First-Principles Geometry Analysis + VLA Claim Status:** (1) Added/updated claim-status tracking for OpenVLA/DreamVLA/PixelVLA/TraceVLA (avoid overstating non-abstract details as “verified”). (2) Added §16.6-§16.11: local flatness testing, δ-hyperbolicity testing, SAE analysis for VLA, and a unified Geometry-First Protocol with a small-model sanity-check sketch. (3) Added Wibral GitLab repos as authoritative code sources. (4) Integrated VLA-Arena as a benchmark context (protocol-sensitive). (5) Added explicit hyperbolic-geometry cautions (no drop-in `I^sx_∩`). |
| **5.8** | Jan 2026 | **VLA-Arena Deep Integration + Memorization/Generalization Analysis:** (1) VLA-Arena as primary evaluation framework (§9.7.1). (2) New §3.6: Memorization vs Generalization hypotheses (H4-H6). (3) Perturbation-based PID robustness protocol (§9.7.2). (4) Expanded confound analysis (§14.5). (5) Long-horizon and compositional failure analysis. (6) Safety dimension integration. |
| **6.0** | Jan 2026 | **Critical Blockers Analysis + Training/Compute Requirements:** (1) New §17: Training, Compute, and Data Requirements Analysis covering 25+ methods, VLA fine-tuning costs, compute budget recommendations. (2) New §18: Critical Blockers and Risk Analysis with 5 show-stoppers, 7 major blockers, 8 minor blockers, Go/No-Go decision frameworks, and fallback scope hierarchy. (3) Verified DreamVLA architecture gaps, OpenVLA availability, VLA-Arena accessibility. (4) Risk assessment: HIGH but tractable if Experiment 0 succeeds. |
| **6.1** | Jan 2026 | **Dream2Flow Integration + Embodiment-Agnostic Analysis:** (1) Dream2Flow (arXiv:2512.24766) integration as related paradigm for 3D object flow extraction. (2) New §10.9: Dream2Flow and Video-to-Flow Paradigm. (3) New Hypothesis H7: 3D object flow as embodiment-agnostic intermediate. (4) New §14.5.7: Embodiment gap confound. (5) Updated §9.7: Dream2Flow failure taxonomy. |
| **6.2** | Jan 2026 | **Unified Architecture: Dream2Flow + Video Predictors + PID + Gaussian Splatting:** (1) New §10.10: complete integration stack. (2) Treated video prediction as plug‑in (local or API) with measurement‑first logging/caching (no hard‑coded cost/latency). (3) Segmentation/tracking/depth model classes for 3D Flow reconstruction. (4) PID analysis at staged intervention points. (5) Gaussian splat visualization concept. (6) Added §17.17 measurement-first integration requirements. |
| **6.3** | Jan 2026 | **Manifold-Geometry Integration + VLA Compatibility Matrix:** (1) New §10.10.12: Manifold geometry per pipeline stage. (2) Key insight: 3D flow is low-dim Euclidean — bypasses manifold issues. (3) New §10.10.13: VLA integration matrix. (4) Updated vision-model placeholders (segmentation/tracking/depth; versions vary). |
| **6.4** | Jan 2026 | **VLM→World Model Transition + 3D Flow vs Latent Action Analysis:** (1) Documented paradigm shift from VLM-based VLAs (Gen 1: OpenVLA, RT-2) to World Model-based (Gen 2: Dream2Flow, DreamVLA, Motus). (2) Analyzed 3D Object Flow vs Latent Action Space Diffusion: 3D flow operates on D (world model) enabling PID validity; latent action diffusion operates on A (policy) and doesn't solve D-side estimation. (3) For PID-VLA, 3D flow serves as "geometry escape hatch." (4) Both approaches can coexist. |
| **6.5** | Jan 2026 | **Hierarchical 3-Way PID for Dream2Flow Analysis (with v6.5.1 corrections):** (1) Hierarchical Pairwise PID (§5.3) maps to Dream2Flow stages: `Syn(V,D_wan;Flow)`, `Syn(V,Flow;A)`. (2) **CORRECTED:** Execution-stage PID removed ("Sim" was undefined). (3) **CORRECTED:** 3D flow dimensionality properly specified — full representation is 3NT (can be 100s-1000s dims); "d≈6-30" only valid for aggregated single-object statistics. (4) **CORRECTED:** v5.5 (geometric validity) vs curse-of-d (statistical reliability) are separate issues; low-d Euclidean addresses both, high-d Euclidean only violates curse-of-d. (5) **CORRECTED:** "Bridge variable" claim removed — disjunction neighborhood doesn't rescue high-d sources; V requires PCA→256 regardless of Flow dimension. (6) Experiment 0 must validate mixed-dimension joint estimation. (7) Latent action diffusion remains complementary (policy) not competing (diagnostic). |
| **6.6** | Jan 2026 | **3DGS Integration + Video Model Selection + Tauri/SparkJS/Gazebo Architecture:** (1) Defined 4 roles for 3DGS in pipeline: PID visualization, spatial failure localization, GWM representation, and spatial‑memory‑style context. (2) Compared candidate predictor families (Wan, Spatia) and optional acceleration methods (treat all speedups as benchmark‑dependent). (3) Integrated Tauri+SparkJS+Gazebo architecture diagram with external Video/Flow service placement. (4) Standardized 3DGS‑PID visualization encoding (R=Syn, G=Red, B=Unq(V), opacity=MI, size=uncertainty). (5) Updated implementation priority and hardware guidance as benchmark‑dependent targets. |
| **6.7 FINAL** | Jan 2026 | **Unified Splat-First Simulation Environment — Complete Architecture:** (1) **Critical comparison with competitors** (§A): Analysis vs Isaac Sim, Omniverse, MuJoCo, Gazebo, SplatSim, DISCOVERSE (benchmark required; avoid cross-task “sim2real %” comparisons). (2) **Complete system architecture** (§B): Tauri+SparkJS+Modular Physics stack diagram with PEGS-style dual Gaussian-Particle representation. (3) **Asset pipeline** (§C): PLY/SPZ/SPLAT/KSPLAT/SOG/GLB/URDF support with COLMAP→SparkJS workflow. (4) **PEGS-style physics** (§D): Rust implementation of particle-Gaussian binding with visual force correction. (5) **FOCI collision detection** (§E): Gaussian overlap integral for splat-splat collision + hybrid mesh approach + splat raycasting. (6) **Environment simulation** (§F): SparkJS Dyno-based weather/lighting/domain randomization. (7) **Camera system** (§G): Virtual cameras with raycast depth, Zenoh streaming, click-to-place. (8) **Real-time editing** (§H): Splat selection (box/raycast), transform, delete, clone + mesh collision adjustment. (9) **Sim2Real strategy** (§I): Multi-layer approach with measurement/ablation plan (avoid pre-committed transfer percentages). (10) **Data collection pipeline** (§J): Workflow from scene setup to RLDS/HDF5/Zarr export. (11) **VLA setup analysis** (§K): Gap analysis of open-source VLA simulation stacks. (12) **Hardware requirements** (§L): Footprint claims are hardware/config-dependent; benchmark for your deployment. |
| **6.8 FINAL** | Jan 2026 | **SmolVLA baseline + world-model comparison (draft; details later revised):** (1) Added SmolVLA (LeRobot) as a lightweight baseline for harness bring-up and pipeline debugging (§7.8; model internals must be verified). (2) Added a conceptual ManiGaussian vs PEGS comparison as a framing device for “learned implicit” vs “explicit physics + correction” world models (verify any implementation claims before use). (3) Sketched additional experiment ideas (Exps 6–10) but did not treat them as core aims. |
| **7.0 FINAL** | Jan 2026 | **Scientific audit + docset alignment:** (1) Reworked architecture/competitor comparisons into capability notes (removed star ratings and roadmap/marketing framing). (2) Removed or labeled unverified performance/latency/hardware “requirements”; made measurement-first language consistent across the docset. (3) Clarified repo reality in §11 (current layout vs planned), fixed Python binding examples to use the real `pid_core_rs` API, and updated task-runner documentation. (4) Made Dream2Flow/WAN integration model-agnostic (“Video Predictor Service”), removed hard-coded clip lengths and vendor-specific assertions, and tightened “no oracle” language. (5) Added OpenUSD/USDZ interop notes and diagrams (LeIsaac Marble tutorial; NVIDIA 3DGrut `.ply → .usdz` bridge) as an optional workflow. (6) Added/updated SmolVLA mentions as a lightweight baseline. (7) Docset consistency sweep across `README.md`, `DIAGRAMS.md`, `EXPERIMENTS.md`, `ARCHITECTURE.md`, and `pidsplatspecs.md`, plus repo guidance via `AGENTS.md`. (8) Added an agent-native control plane requirement (“Agent Bridge” via JSON‑RPC/MCP) so GUI actions and live interventions are reproducible and callable from LLM coding tools. |

---

# Appendix C: Modern Rendering Stack (SparkJS and WebGPU)

**Context:** The simulation environment described in §10.8 relies on a novel "Splat-First" rendering stack. This appendix details the technical specifications for the visualization layer.

### C.1 SparkJS Architecture

This spec assumes a **WebGPU 3D Gaussian Splatting renderer** (referred to as “SparkJS” throughout, but replace with your chosen implementation). The core requirements for PID‑Splat visualization are:
1. **Low-latency splat updates**: per-frame updates to positions/colors/scales without expensive CPU→GPU copies.
2. **Deterministic, inspectable overlay stages** (“Dynos” concept): a programmable pass that can map PID metrics → visual encodings.
3. **Scalable sorting/LOD strategy**: enough throughput to remain interactive at the splat counts required by your scenes (benchmark on target hardware).

Do not rely on vendor roadmaps or marketing claims for performance; treat renderer choice as an interchangeable component behind a stable interface.

### C.2 Dyno Shader Specification

**Concept:** A "Dyno" is a compute shader stage that runs before sorting. It takes PID metrics and environmental state as input and outputs modified splat attributes.

**PID Heatmap Dyno (WGSL):**
```wgsl
struct PidMetric {
    synergy: f32,
    redundancy: f32,
    unique_v: f32,
}

@group(0) @binding(0) var<storage, read> pid_buffer: array<PidMetric>;
@group(0) @binding(1) var<storage, read_write> splat_colors: array<vec4f>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let pid = pid_buffer[i];
    
    // Diverging colormap: Red (Syn) -> Gray (Red) -> Blue (Unq)
    let syn_color = vec3f(1.0, 0.0, 0.0);
    let red_color = vec3f(0.5, 0.5, 0.5);
    let unq_color = vec3f(0.0, 0.0, 1.0);
    
    // Mix based on dominant metric
    var final_color = red_color;
    if (pid.synergy > pid.redundancy && pid.synergy > pid.unique_v) {
        final_color = mix(red_color, syn_color, pid.synergy);
    } else if (pid.unique_v > pid.redundancy) {
        final_color = mix(red_color, unq_color, pid.unique_v);
    }
    
    splat_colors[i] = vec4f(final_color, 1.0);
}
```

---

*End of Document*
