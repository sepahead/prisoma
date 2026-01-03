# PID-VLA Implementation Agents & Roadmap

**Status:** Canonical Implementation Plan (linked to `grandplan.md` v6.7)
**Date:** 2026-01-03

This document defines the step-by-step implementation strategy for the PID-VLA project, identifying critical blockers, evaluating architectural approaches, and prescribing a 4-phase execution plan.

---

## A. Identified Blockers (10 Critical Issues)

These issues must be addressed for the project to succeed. They are prioritized by risk to the core scientific validity.

1.  **Manifold Geometry Problem (High Risk):** The continuous `I^sx_∩` estimator relies on Chebyshev (L∞) geometry. VLA embeddings (OpenVLA, DreamVLA) likely lie on curved manifolds (hyperbolic/Lorentzian). Naive application will yield invalid results.
    *   *Mitigation:* `geometry.rs` diagnostics (intrinsic dimension, δ-hyperbolicity) + Manifold Unrolling or Quantization (`grandplan.md` §16).

2.  **Dimensionality Curse (High Risk):** At d=4096, kNN distance concentration makes neighborhoods meaningless.
    *   *Mitigation:* Dimensionality reduction (PCA, SAE) validated via Experiment 0.

3.  **Strong Dependence Regime (Medium Risk):** VLA actions are near-deterministic functions of inputs (`A = f(V, L)`). This causes unbounded Mutual Information estimates in continuous kNN.
    *   *Mitigation:* Add calibrated noise or use discrete targets (`grandplan.md` §1.2 Warning 4).

4.  **Non-i.i.d. Trajectories (Medium Risk):** VLA rollouts are strongly autocorrelated. Treating frames as independent samples biases estimators.
    *   *Mitigation:* Cross-trajectory sampling or large-stride subsampling.

5.  **PyO3/Python Bindings (Engineering):** The high-performance Rust core (`pid-core`) is not yet callable from the Python experimental harness.
    *   *Mitigation:* Implement `crates/pid-python`.

6.  **MLX/CoreML Pipeline (Engineering):** No pipeline exists to extract embeddings from VLAs on Apple Silicon (primary dev target).
    *   *Mitigation:* `mlx_inference.py` implementation.

7.  **O(n²) Brute-Force kNN (Performance):** Current Rust implementation is exact O(n²). This will not scale to real-time monitoring.
    *   *Mitigation:* k-d tree / ball tree implementation or GPU acceleration.

8.  **Headless Gazebo Setup (Infrastructure):** No established method for running Gazebo Harmonic headless + streaming via Zenoh to Tauri.
    *   *Mitigation:* `pidsplatspecs.md` implementation plan.

9.  **Gaussian Splatting Integration (Novelty):** Binding Rapier physics to 3DGS visual entities ("Splat-First Physics") is uncharted territory.
    *   *Mitigation:* `pidsplatspecs.md` PEGS architecture.

10. **Real-time Monitoring (Performance):** Latency constraints for "live" PID visualization (<30ms) are tight.
    *   *Mitigation:* Use Shannon Invariants (CI) for screening (fast) and full PID only on demand.

---

## B. 10 Implementation Approaches Evaluated

We evaluated 10 potential strategies for executing this project.

| # | Approach | Best For | Risk | Verdict |
| :--- | :--- | :--- | :--- | :--- |
| 1 | **Sequential Milestones** | Safe, validated results | Slow execution | **Too slow** |
| 2 | **Parallel Tracks** | Speed | Integration hell | **Feasible** |
| 3 | **Visualization-First** | Early demos/funding | Unstable foundation | **High risk** |
| 4 | **Simulation-First** | Data pipeline health | High complexity | **Heavy start** |
| 5 | **Hybrid Local/Cloud** | Scaling | Infra overhead | **Premature** |
| 6 | **Microservices** | Modularity | Over-engineering | **No** |
| 7 | **Monolithic MVP** | Fast prototyping | Technical debt | **Hard to scale** |
| 8 | **Research-Prod Split** | Clean separation | Duplication | **Double work** |
| 9 | **WASM Core** | Web deployment | Complexity/Perf | **Maybe later** |
| 10 | **Incremental Validation** | Scientific rigor | Low risk | **RECOMMENDED** |

**Selection:** **Approach 10 (Incremental + Continuous Validation)** with elements of **Parallel Tracks**. We must validate the *estimator* (Experiment 0) before building the *visualizer* (Tauri), but infrastructure work can proceed in parallel once the math is safe.

---

## C. Recommended Plan: 4 Phases

### Phase 1: Validation Gate (Weeks 1-2)
**Goal:** Prove `pid-core` works at d=4096 (or find the working dimension).
*   **Action:** Complete `crates/pid-python` bindings.
*   **Action:** Run `exp0_validation.py` on synthetic Gaussian/XOR data.
*   **Gate:**
    *   **GO:** Error < 15% at d=256 (PCA).
    *   **PIVOT:** Use discrete PID if continuous fails.
    *   **NO-GO:** If even low-d fails, debug `pid-core`.

### Phase 2: Infrastructure (Weeks 3-6)
**Goal:** Build the three pillars of the platform concurrently.

*   **Track A: Analysis Harness (Python/Rust)**
    *   Implement `pid-python` fully.
    *   Port `mlx_inference.py` for OpenVLA embedding extraction on M4 Max.
    *   Implement `geometry.rs` diagnostics (Intrinsic Dim, δ-Hyperbolicity).

*   **Track B: Simulation (Gazebo)**
    *   Set up Headless Gazebo Harmonic.
    *   Implement Zenoh bridge for sensor data (RGB-D + Pose).
    *   Create "Splat-First" scene loader (`.ply` → Rapier Collider).

*   **Track C: Visualization (Tauri)**
    *   Scaffold Tauri v2 app.
    *   Implement SparkJS (WebGPU) renderer for 3DGS.
    *   Create IPC layer for streaming PID metrics.

### Phase 3: Integration (Weeks 7-10)
**Goal:** Connect the pillars to run Experiments 1-2.
*   **Integration:** Feed OpenVLA embeddings (Track A) into `pid-core`.
*   **Integration:** Stream results to Tauri (Track C).
*   **Experiment 1:** Run V-L-A decomposition on standard benchmarks (LIBERO).
*   **Experiment 2:** Compare PID features vs. Entropy baselines.

### Phase 4: Production & Advanced Features (Weeks 11-14)
**Goal:** Real-time capability and novel research.
*   **Optimization:** SIMD acceleration for `pid-core` distances.
*   **Real-time:** Live "Information Heatmap" overlay in Tauri.
*   **Experiment 3:** Dimensionality robustness study.
*   **Experiment 4:** Causal interventions (if DreamVLA available).

---

## D. Detailed Technical Specifications

### 1. Gaussian Splatting Setup (Visualization)
*   **Renderer:** SparkJS (custom WebGPU renderer) or port `gsplat.js`.
*   **Pipeline:**
    1.  Capture scene (Polycam/Luma).
    2.  Train Splat (Nerfstudio).
    3.  Export `.spz` (compressed splat).
    4.  Load in Tauri via SparkJS.
*   **Optimization:** Dynamic LOD (Level of Detail) to maintain 60fps on M4 Max.

### 2. Tauri + SparkJS Frontend (Application)
*   **Stack:** Tauri v2 (Rust backend) + React/Three.js (Frontend).
*   **IPC:** Zenoh for high-bandwidth data (video/splats), Tauri Commands for control.
*   **Dynos:** Custom WGSL shaders that modify Splat color buffers based on `pid-core` streams (Red=Synergy, Blue=Unique).

### 3. Headless Gazebo Simulation (Physics)
*   **Version:** Gazebo Harmonic (gz-sim).
*   **Mode:** Headless (no GUI, rendering via sensors only).
*   **Bridge:** `ros_gz_bridge` or direct Zenoh plugin.
*   **Scene:** "Invisible" collision meshes matching the Gaussian Splat visual geometry.

### 4. Custom VLA Solution (Inference)
*   **Base:** OpenVLA (7B) running on MLX (Apple Silicon).
*   **Hooks:**
    *   `vision_encoder`: Output of SigLIP/DinoV2.
    *   `language_model`: Hidden states at Layer 16 and 32.
    *   `action_head`: Logits/Tokens.
*   **Preprocessing:** Real-time PCA projection (matrix multiplication) before sending to `pid-core` to satisfy dimensionality constraints.
