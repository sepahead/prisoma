# WorldWarp Integration

> **Documentation Cross-Reference**:
> - `grandplan.md` — Canonical spec and world-model positioning
> - `EXPERIMENTS.md` — When/why to treat external world models as staged variables
> - `DIAGRAMS.md` — Agent Bridge control plane (how services are invoked + logged)

**Docset alignment:** docset v12.5 (optional external comparator — evidence-ladder **E1** interface spec per grandplan §8.9; pre-implementation, not built in this repo today, not a direct ecosystem edge)
**Status:** Specification / integration notes (verify upstream claims at time of use)

**Docset-wide final solution:** `grandplan.md` §16 is the decision log. WorldWarp or any external world model is an optional external comparator (evidence-ladder E1, grandplan §8.9); it must emit versioned artifacts into the run log and be visualized through the same Rerun/Tauri split rather than becoming an unlogged side channel.

## Overview
WorldWarp (https://github.com/sepahead/WorldWarp) is an external framework for generating long-range, camera-conditioned scenes from a single image. In the prisoma context, it can be treated as an *optional* external world-model baseline (evaluative/generative) to compare against VLA internal representations. Verify model backbones, licenses, and reproducibility constraints from the upstream repo before using it in experiments.

## Key Features
- **As described upstream (verify):** asynchronous chunk-wise generation, explicit camera conditioning, and an online cache intended to improve geometric consistency across frames.
- **Interactive GUI (verify):** upstream mentions a GUI for rapid testing and parameter tuning (e.g., camera paths, generation strength).
- **Foundation model integration (verify):** WorldWarp may integrate one or more video/VLM backbones; confirm exact models/versions and licensing in the upstream documentation rather than assuming specific WAN/Qwen variants.

## Relevance to prisoma
WorldWarp aligns with the "Generative World Model" component of the prisoma architecture.
- **Environment Generation:** Can generate diverse, consistent background scenes for robot simulation from a single seed image.
- **Visual Forecasting:** Serves as a predictive model for what a robot *should* see after a camera movement, providing a candidate "D" (Dynamics / world-model axis — never depth) reference for PID comparison.
- **Data Augmentation:** Capable of creating novel viewpoints of existing datasets to robustify VLA training.

## Integration Points
1.  **Evaluative World Model:** Use WorldWarp to generate expected future frames based on robot camera motions. Treat the predicted frames/latents as an explicit staged variable (e.g., `D_worldwarp`) and analyze `(V_obs, D_worldwarp; A)` under the same contract-first logging rules as other world models.
2.  **Simulation Environment:** Record generated scenes as run-log artifacts, visualize them in Rerun during Phases 1–3, and integrate them into SparkJS/Three.js only for Phase 4 custom rendering.
3.  **Counterfactual Analysis:** Generate "what-if" scenarios by altering camera trajectories to test VLA robustness to viewpoint shifts.
4.  **Agent Bridge orchestration (planned):** invoke WorldWarp as an external service through the same control plane as the GUI (JSON‑RPC/MCP), and log prompts/camera paths/versions/seeds as first-class artifacts for replay.
