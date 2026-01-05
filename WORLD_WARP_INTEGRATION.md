# WorldWarp Integration

## Overview
WorldWarp (https://github.com/sepehrmn/WorldWarp) is an external framework for generating long-range, camera-conditioned scenes from a single image. In the PID‑VLA context, it can be treated as an *optional* external world-model baseline (evaluative/generative) to compare against VLA internal representations. Verify model backbones, licenses, and reproducibility constraints from the upstream repo before using it in experiments.

## Key Features
- **Asynchronous Chunk-Wise Autoregressive Diffusion:** Enables the generation of extended, long-range views efficiently.
- **Explicit Camera Conditioning:** Allows precise control over camera rotation and translation, critical for simulating robot viewpoints.
- **Online 3D Cache:** Maintains geometric consistency across generated frames, reducing geometric drift/artifacts in scene structure.
- **Interactive GUI:** Gradio-based interface for rapid testing and parameter tuning (camera paths, generation strength).
- **Foundation model integration (verify):** WorldWarp may integrate one or more video/VLM backbones; confirm exact models/versions and licensing in the upstream documentation rather than assuming specific WAN/Qwen variants.

## Relevance to PID-VLA
WorldWarp aligns with the "Generative World Model" component of the PID-VLA architecture.
- **Environment Generation:** Can generate diverse, consistent background scenes for robot simulation from a single seed image.
- **Visual Forecasting:** Serves as a predictive model for what a robot *should* see after a camera movement, providing a "D" (Dream) reference for PID comparison.
- **Data Augmentation:** Capable of creating novel viewpoints of existing datasets to robustify VLA training.

## Integration Points
1.  **Evaluative World Model:** Use WorldWarp to generate expected future frames based on robot camera motions. Compute PID synergy `Syn(V_obs, V_pred_WorldWarp; A)` to assess if the VLA's internal state matches a geometrically consistent external predictor.
2.  **Simulation Environment:** Integrate generated scenes into the SparkJS/Three.js rendering pipeline as dynamic skyboxes or environment maps.
3.  **Counterfactual Analysis:** Generate "what-if" scenarios by altering camera trajectories to test VLA robustness to viewpoint shifts.
4.  **Agent Bridge orchestration (planned):** invoke WorldWarp as an external service through the same control plane as the GUI (JSON‑RPC/MCP), and log prompts/camera paths/versions/seeds as first-class artifacts for replay.
