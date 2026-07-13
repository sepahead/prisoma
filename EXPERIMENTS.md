# prisoma Experimental Protocols

> **Documentation Cross-Reference**:
> - `grandplan.md` — Master plan and theoretical foundations
> - `pidsplatspecs.md` — Detailed simulation environment and PID specifications
> - `ARCHITECTURE.md` — Component breakdown (Rerun-first diagnostics, deferred Tauri, modular physics, 3DGS) and advantages over VLM-based robotics
> - `DIAGRAMS.md` — Visual architecture diagrams
> - `README.md` — Quick start guide
> - `GAUSS_MI_INTEGRATION.md` — Optional 3DGS uncertainty + view selection (spec)
> - `WORLD_WARP_INTEGRATION.md` — Optional external world‑model baseline (spec)

## Detailed Specifications for Reproducible Experiments
 
**Version:** docset v12.5 (aligned with the 2026-07-12 `grandplan.md` rewrite — seventh adversarial revision)
**Date:** 2026-07-12
**Context:** This document specifies *task suites, data collection, and evaluation protocols* used to test the confirmatory claims in `grandplan.md`. `grandplan.md` defines estimator/measure validation and the analysis logic; this file focuses on what to run and log. The deterministic Agent Bridge/Rapier/Rerun/attribution slices are implemented groundwork, but the core+ecosystem conformance benchmark (M2) and the locked H1 experiment (M4) remain open; external video predictors and the fuller PID‑Splat environment remain specifications until built.

**Docset-wide final solution:** `grandplan.md` §16 is the decision log. Experimental evidence must flow through the canonical run log; the Agent Bridge is the only control plane; Rerun is a read-only diagnostic viewer; and Tauri/SparkJS is deferred until the run-log/replay/Rerun loop is reliable. Every VLA action, intervention, scene edit, pause/resume/step transition, and correction force must be recorded as an Agent Bridge command before execution. PID, observers, Zenoh, Rerun, and offline harnesses do not actuate the system.

> **Docset v12.5 migration note (read first).** This document predates the v12.5 registry rewrite and is
> still organized around the retired `Exp0–Exp10` / `H1–H9` scheme. Read every legacy label through the
> v12.5 confirmatory registry (`grandplan.md` §4) and the S0–S7 gate sequence (`grandplan.md` §5.1):
> - **Exp0 estimator validation → the S1 estimator/measure-validation gate** (`grandplan.md` §7; the `exp0` binary implements part of §7).
> - **Exp1–Exp5 task suites → the §5 experimental programme**, analysed under the §6 statistical analysis plan; **Exp6–Exp10 world-model comparisons → §9.5 cross-model / §5 exploratory extensions** (must not block the core programme).
> - **H1 (grounding↔PID) → registry H1** (pre-treatment diagnostics predict intervention response; mandatory **Protocol A** paired algorithmic vs **Protocol B** randomized closed-loop fork, `grandplan.md` §6.3).
> - **H2/H3/H4/H5 (redundancy/uniques/memorization/temporal) → registry H3/H4** (PID adds incremental value only inside its validated support envelope; availability-vs-use divergence), estimated under §6 and gated by §7.
> - **H2 prospective failure detection → registry H2** (censoring-aware failure prediction, `grandplan.md` §6.4); **EC1** is the provenance-complete-replay engineering-acceptance claim.
> - **H6 (safety) / H7a–H7b (flow-as-bridge) / H8 (estimator diagnostics) / H9 (attribution) → Exploratory or retired** (`grandplan.md` §4 exploratory/retired lists; flow-as-bridge is `grandplan.md` §9.6).
> - Milestones are the research **M0–M7** (`grandplan.md` §12); PID admissibility is gated by **four** gates — population/measure/estimator/application (`grandplan.md` §7.1) — not the older two-gate language.
 
---
 
## Table of Contents
 
0. [Relationship to `grandplan.md`](#0-relationship-to-grandplanmd)
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
12. [Compute and Storage Planning](#12-compute-and-storage-planning)
13. [Reproducibility Checklist](#13-reproducibility-checklist)

*(Optional extensions §§14–19 — world-model comparisons Exp6–10 — and Appendices follow below; they must not block Experiments 0–4.)*

---

## 0. Relationship to `grandplan.md`

- `grandplan.md` is the scientific contract: definitions, estimator/measure assumptions, identification and confound controls, and the S0–S7 gate sequence (`grandplan.md` §5.1) with the four PID gates (`grandplan.md` §7.1) — especially the S1 estimator/measure-validation gate.
- This document specifies *what to run and what to log* so those gates and confirmatory claims can actually be tested.
- The milestone build order and gate sequence in `grandplan.md` §12 (M0–M7) and §5.1 are binding for experiment tooling: do not replace run logs with GUI-only state, do not treat live transport as required, and do not introduce predictor-driven `Flow_pred` before simulator-derived `Flow_gt` is replayable.
- **Mapping (high-level):**
  - The legacy *Experiment 0* estimator/geometry gate ↔ this document §4 (synthetic validation + geometry checks), which implements part of the S1 gate (`grandplan.md` §7).
  - The legacy *Experiments 1–5* task suites ↔ datasets generated by this document’s Experiments 1–5, feeding the `grandplan.md` §5 experimental programme and analysed under the §6 statistical analysis plan.

### 0.1 Hypothesis Coverage Matrix

The following table is the **binding v12.5 registry**. A task suite is not itself a claim: its run
configuration must choose exactly one confirmatory protocol and use that protocol’s unit, treatment,
outcome, score, and language.

| Current claim / protocol | Candidate task suites | Primary evidence | Required controls and current blocker |
|---|---|---|---|
| **EC1 — provenance-complete replay** | Exp1 plus a structurally different adapter/environment | Contract-violation detection and exact/tolerance-bounded replay versus a conventional script/container baseline | Typed assignment/receipt/outcome lineage, fault injection, standard-format adapter, external benchmark; still open (`grandplan.md` §8.8) |
| **H1 Protocol A — paired frozen-snapshot algorithmic response** | Exp1 baseline cases; Exp3 intervention constructions | Held-out direct prediction of the declared paired response functional, with calibration, response reliability/Monte Carlo error, and a locked feature-vs-baseline contrast | Immutable clone state, pre-treatment moderator, instrumented/noninstrumented noninterference, draw ledger, reverse-order/process controls; blocked on capture and clone machinery (`grandplan.md` §6.3) |
| **H1 Protocol B — randomized closed-loop effect moderation** | Exp1/Exp3 randomized episodes | Overall ITT first; then held-out effect-specific loss, causal calibration, prioritization, and policy value/regret under recorded assignment probabilities | Randomization/receipt/reset/censoring ledger, cluster-aware inference, synthetic oracle and negative controls; blocked on capture and assignment runner (`grandplan.md` §6.3) |
| **H2 — prospective censoring-aware failure prediction** | Prespecified landmarks in Exp1/Exp2; later Exp5 transport | Held-out log loss or censoring-aware Brier score at the frozen horizon, plus calibration, event sensitivity at fixed false-alarm burden, nondetection-retaining lead time, and decision utility | A deterministic synthetic fixed-horizon/IPCW/alarm arithmetic reference is fixture-runnable (`just h2-reference`); the domain freeze, real prospective capture, full matched-access comparator frontier, and external/later-time validation remain blocked (`grandplan.md` §6.4) |
| **H3 — conditional incremental PID value** | Any H1/H2 dataset only after all four gates | Nested out-of-sample improvement from adding preregistered PID features to the strongest valid non-PID model under the active H1/H2 endpoint | Population/measure/estimator/application gates, train-reference local construction, abstention denominator; application gate currently blocked (`grandplan.md` §7) |
| **H4 — availability–use divergence** | Exp3 input/internal intervention pairs; Exp1 positive/negative controls | Prespecified discordance between held-out decodability and policy/execution ITT effects, conditional on engagement and support | At least two intervention constructions where feasible, positive/negative controls, equivalence margins; blocked on capture/intervention pilot (`grandplan.md` §6.3) |

### 0.1.1 Retired pre-v12.5 mapping (historical and non-operative)

The legacy H1–H9 matrix below is retained only to explain the older Exp1–Exp5 prose and must not be
used to select a primary endpoint. In particular, its ΔAUROC, rank-correlation, and atom-sign rows
do not replace Protocol A, Protocol B, or prospective H2 above.

PID/CI hypotheses below inherit the four PID gates (`grandplan.md` §7.1): **population** (is the atom finite/defined/meaningful), **measure** (does the shared-exclusions functional have the properties the claim needs), **estimator** (does the implementation recover it with acceptable bias/coverage/failure-detection at the planned regime), and **application** (are the real embeddings/sampling close enough to a validated regime to interpret). Concretely: 1) the measure-independent MI/coherence gate passes for the exact pipeline, 2) a measure-specific oracle/cross-check validates the claimed atoms, and 3) recovery, intrinsic dimension, distance concentration/ties, dependence, and calibrated local-flatness diagnostics support every variable and concatenation passed to the estimator (see §4.0). Today the default high-dimensional MI/coherence path is **NO-GO**; the `pid-rs` pin does carry real low-dimensional additive-Gaussian oracle and discrete-SxPID reference evidence, but continuous shared-exclusions atoms on **real VLA embeddings are BLOCKED / NOT APPLICATION-VALIDATED** (`grandplan.md` §7.2, §7.5). Attribution-based comparisons add their own faithfulness/stability checks; they do not repair any of the four gates.

For every active V–L endpoint, preregister instruction diversity and pass an instruction occupancy/entropy gate; otherwise make V–D primary. Cross-task atom comparisons require a declared information-scale denominator with estimator uncertainty propagated. Fit all learned preprocessing once on disjoint V0/W0 training data, freeze it across perturbation cells, and record its transform hash.

| Legacy hypothesis (read via the v12.5 migration note) | Experiments | Variables (examples) | Primary metrics | Required controls (see `grandplan.md` §6) |
|-----------|-------------|----------------------|----------------|--------------------------------------------|
| **H1** Grounding failures ↔ PID features → historical precursor to registry **H1/H3** | Exp1, Exp3 | `(V,L;A)` or `(V,D;A)` | **Superseded exploratory endpoint:** held-out episode-level incremental ΔAUROC of {baselines + PID/CI} over {baselines alone}; not the current Protocol A/B response score | Label leakage checks, stratified splits, nested CV/held-out test, corrected sampling/geometry diagnostics + separate MI/atom gates; mandatory minimal baseline set (`grandplan.md` §6.5) |
| **H2** Redundancy ↔ robustness-to-ablation → historical precursor to registry **H3/H4** | Exp1, Exp3 | `(V,L;A)` under single-modality corruptions; optional graded-`L` probe via language-specificity levels (vague/default/specific, RoboLab-120-style; `grandplan.md` §5.3) | **Superseded exploratory endpoint:** rank association between pre-ablation `Red` and success-vs-severity slope | Instruction diversity/occupancy gate, matched difficulty, nuisance controls, frozen transform hash, separate MI/atom gates; availability-vs-use asymmetry reported only under the current H4 protocol |
| **H3** Uniques ↔ intervention sensitivity → historical precursor to registry **H1/H3** | Exp1, Exp3 | `(V,L;A)` under modality-isolated perturbations | **Superseded exploratory endpoint:** ordering agreement between `Unq` and matched-intervention effects | Instruction diversity/occupancy gate; perturbations isolated to one modality; outcome-independent strength matching; placebo perturbations; one frozen transform across cells |
| **H4** Memorization vs generalization → retired/exploratory | Exp1, Exp3, Exp5 + §10 | `(V,L;A)` across in-dist vs perturbed | **Superseded exploratory endpoint:** SSI := −IQR(Syn) and structured-perturbation degradation; this is not current registry H4 | Instruction diversity/occupancy gate, information-scale denominator with propagated estimator uncertainty, frozen transform hash, matched perturbation difficulty, seed controls |
| **H5** Temporal synergy degradation → Exploratory (`grandplan.md` §9.4) | Exp2 | windowed `Syn(V_t,D_t;A_t)` (primary) + mandatory CI-only twin; phase-aligned windows **pooled across episodes** (`grandplan.md` §9.4) | Trend (slope / early-vs-late contrast) vs composition-stage index; episode-level block bootstrap; predicted decline in failing vs succeeding episodes (`grandplan.md` §6) | Pooling across episodes (never per-trajectory windows), within-window stride ≥ decorrelation length, post-stride `N_win` ≥ a future measure-specific validated minimum, outcome stratification, phase definitions preregistered (§6.5); **mandatory CI-only ablation** (`grandplan.md` §3.8) |
| **H6** Safety constraints require V–L integration **(Deferred — `grandplan.md` §4 retired/deferred; no claims until safety labels + matched controls exist)** | Exp3 | safety vs baseline instructions | ΔUnq(L), ΔSyn(V,L;A); collision/near-miss rates | Matched task conditions, instruction-only changes, nuisance controls (lighting/distractors) |
| **H7a** (method) Flow-as-bridge enables stage-wise/cross-embodiment diagnostics; **H7b** (falsifiable) `Syn(V,D;A)` tracks world-model quality independent of execution success → Exploratory (`grandplan.md` §9.6) | Exp4, Exp5 | `(V,D;Flow)` and/or `(V,Flow;A)` | H7a: engineering acceptance (gates pass on flow targets). H7b (`grandplan.md` §6): difference in synergy–failure correlation between world-model-stage and execution-stage failures; predicted stronger for world-model-stage | Fixed flow pipeline across runs, low-d flow features, negative controls (shuffled flow / shuffled pairing), stage labels per `grandplan.md` §9.6 |
| **H8** Diagnostics choose estimator regime | Exp0 | recovery controls + geometry/dependence diagnostics | Separate MI/coherence and measure-specific atom verdicts for continuous PID vs a preregistered alternative | Analytic MI recovery; measure-specific atom oracle/cross-check; intrinsic dimension; concentration/ties; dependence; calibrated local flatness. Sampled mean `δ_rel` is descriptive only |
| **H9** Attribution probes triangulate PID claims | Exp1, Exp3, Exp4 | LRP/IG/DeepLIFT/Grad-CAM/TCAV/saliency/occlusion/SHAP-style scores on the same logged samples | Agreement or principled disagreement with PID uniques/synergy under controlled interventions; incremental failure-prediction value | Model/data randomization sanity checks, baseline/background sensitivity, deletion/occlusion tests, attention-not-explanation caveat |

### 0.2 Runbook: What Is Executable Today vs Blocked (docset v12.5, 2026-07-12)

This table is the self-sufficient entry point: it maps the run order onto current tooling, expected outcomes, and blockers. Commands assume `just` (each recipe wraps plain `cargo` commands listed in `README.md`/`AGENTS.md`).

| Step | What | How (today) | Gate / expected outcome | Canonical reference |
|---|---|---|---|---|
| 1 | Toolchain + estimator diagnostics (legacy Exp0 → S1 gate) | `cargo test`; `just exp0` / `just exp0-bin` / `just exp0-runlog`; opt-in uncertainty: `just exp0-uncertainty` (`--bootstrap`/`--permutation`). `--strict-gate` is a direct CLI opt-in that implies `--strict-band` and exits 3 unless the curated d=1 Gaussian **MI** grid is GO; it does not gate the default high-d sweep or continuous atoms | Split status: default high-d MI/coherence = **NO-GO**; continuous `I^sx_∩` atoms on real embeddings = **BLOCKED / NOT APPLICATION-VALIDATED** (the `pid-rs` pin does carry low-d additive-Gaussian oracle + discrete-SxPID reference evidence). The curated strict band and favourable-dimension UQ diagnostics do not override either status | `grandplan.md` §7.2, §7.5; `findings.md` |
| 2 | Run-log spine + replay + bridge smokes | `just runlog-demo`, `runlog-validate`, `runlog-replay`, `runlog-bridge-*`, `runlog-sim-verify`, `runlog-rerun` | `valid=true`, `errors=0`; deterministic replay; simulator-derived `Flow_gt` verified | `grandplan.md` §8.2, §8.5 |
| 3 | Labeled toy pipeline end-to-end | `just toy-harness` | Canonical labeled artifacts validate; not VLA evidence | `grandplan.md` §12 (milestone rehearsal) |
| 4 | Offline `(V,L,D,A)` harness, all three PID modes | `just offline-harness`, `offline-harness-require-*`, `offline-harness-strict`, `offline-harness-discrete`, `offline-harness-discrete-pls` | Strict mode exits nonzero on its implemented legacy geometry aggregate (software smoke only; not corrected scientific eligibility); discrete modes report `saturation_warning=true` on the tiny fixtures (by design — the `grandplan.md` §7.6 discrete PID gate) | `grandplan.md` §7.6, §6.2 |
| 5 | **First real VLA/task capture (open critical path; adapter groundwork implemented)** | `just safe-adapter` maps released SAFE rollouts to the harness `(V,L,D,A)`+labels contract with the `grandplan.md` §9.2 hook-probe; `just rapier-harness` exercises a real Rapier3D task with labels + `Flow_gt`. Still required: real licensed data capture/pull, model/task/hook selection, the prospective episode-local H1 feature path, and a scientifically adequate capture-sizing gate | Strict harness modes have been exercised on the synthetic SAFE fixture only. The implemented `pid-sim-power-gate` is an idealized endpoint-level sensitivity simulator; it omits the required family → task/case → episode nesting, PID/SSI measurement error, binary outcomes, severity allocation, and selected-design type-I error. Its first-run counts are withdrawn as capture requirements. Capture sizing is **NOT READY / NOT PASSED** | `grandplan.md` §9.1, §6.8 |
| 6 | Exp1–Exp5 protocols (§5–§9 of this document) | `just h1-preflight` exercises the schema-v2 representative-mechanism structural/noninterference contract; `just h1-protocol-a` exact-binds that pass and runs a deterministic synthetic finite-benchmark clone/response and fixed out-of-fold scoring primitive. `just h2-reference` exact-binds separate plan/ontology/feature/split artifacts and runs deterministic complete/censored fixed-horizon cumulative-incidence, grouped fitting, IPCW Brier, reliability, alarm, nondetection, and declared-utility arithmetic. All exercise readable failure paths and zero PID events. Real Protocol A capture/analysis, subprocess/stochastic controls, Protocol B assignment/effect scoring, real prospective H2 capture/comparators/external validation, and applicable estimator/measure gates remain blocked | Every checked number is a software-reference output only. H1 binds `synthetic_fixture_only=true`, `establishes_h1_evidence=false`; H2 additionally binds `establishes_h2_evidence=false`, `prospective_capture=false`, `external_validation=false`, and `comparator_frontier_complete=false`. The outputs establish neither scientific claim, physical effect, calibration validity, warning benefit, nor closed-loop robustness. Per-claim metrics + controls remain those in the binding §0.1 table; kill rules are in `grandplan.md` §3.8 and statistics in §6. No capture-size claim may be made from the current idealized simulator | `grandplan.md` §5, §6.3–§6.4 |

**Binding power status:** `cargo run --release -p pid-sim --bin pid-sim-power-gate` is implemented and useful as an idealized endpoint-sensitivity simulator. It is **not capture-ready**, and its 2026-07-10 first-run task/case/episode counts are withdrawn—not lower bounds, recommendations, or requirements. A replacement capture gate must simulate the nested family/task-or-case/episode design, PID/SSI measurement error, binary outcomes, severity allocation, and type-I error under the selected analysis, after the prospective H1 feature path exists.

Three discipline rules apply at every step: (a) each (PID measure, preprocessing, estimator config) tuple is a distinct preregistered regime — continuous `I^sx_∩` and discrete Williams–Beer `I_min` results must never be pooled (`grandplan.md` §7.6); (b) non-PID baselines and uncertainty (block bootstrap, permutation nulls) accompany every PID number; (c) each active confirmatory claim has exactly one preregistered primary endpoint with a predicted direction — everything else is exploratory under BH-FDR q = 0.10, and the active estimator regime is selected ex ante by the separate S1 estimator/measure gate (`grandplan.md` §7).

### 0.2.1 Data sources (the harness is source-agnostic)

Everything downstream consumes one `(V,L,D,A)`+labels contract (the `OfflineVldaDataset` JSON the offline harness reads), so capture sources are pluggable. In `(V,L,D,A)`, **D is the Dynamics / hidden-state axis, not depth** — defined per model (`grandplan.md` §9.1, §9.2).

| Source | Role | Standalone? | Status |
|---|---|---|---|
| `experiments/safe_adapter/` (released SAFE rollouts) | **Critical path** (S2/EC1 reference adapter, `grandplan.md` §8.7) | yes | Converter/hook-probe implemented and fixture-validated; real capture and scientific gates still open |
| `crates/pid-sim` fixtures + `pid-rapier-harness` / `pid-toy-harness` | Sim cross-checks | yes | Runnable software/physics smokes with physics-derived labels + `Flow_gt`; not VLA evidence |
| `crates/ncp-observer` (Engram/NEST over the Neuro-Cybernetic Protocol) | **Optional** external bridge (M2 ecosystem conformance, `grandplan.md` §8.9.5) | n/a | **Exploratory-only, read-only — below the S2/EC1 bar** |

The pure-PID stack (the table above minus NCP) builds and its software smokes run with **no NCP/Engram/Zenoh dependency** — `ncp-observer` is excluded from the default cargo workspace. That does not imply that the scientific estimator/capture gates pass. NCP is a read-only exploratory tap, not a controller and not part of grandplan's critical path. The observer now performs exact wire-0.8 source/sequence joins and never uses recency fallback, but it remains below the S2/EC1 contract until a conforming live publisher plus honest `L`, `metadata.split`, `episode_id`, and `success` structure exist for the strict harness checks and the `grandplan.md` §4 H1 audit. See `NCP_DEV_PROMPT.md`.

## 0.5 Physics and Robot Backend Usage: Modular Architecture

This table clarifies the intended backend choices across experiments. Treat “recommended” as design guidance, not a claim of superiority or current implementation: the checked repo includes the deterministic object sim/logging harness **and a real Rapier3D (`rapier3d-f64`) backend behind the `rapier` feature** (gravity/contacts/friction + a scripted push-to-goal manipulation with physics-derived labels and `Flow_gt`; `just rapier-harness`), while MuJoCo/Gazebo/Isaac-backed manipulation remains planned. The right choice depends on your benchmark, hardware, and what you are trying to validate.

| Component | Engine | Purpose | Experiments |
|-----------|--------|---------|-------------|
| **Object Manipulation** | Rapier / MuJoCo | Grasping, stacking, placing objects | Exp 1-5 |
| **Robot Kinematics** | Gazebo / MuJoCo | 7-DOF arm dynamics, joint limits | Exp 1-5 |
| **Sensor Simulation** | Gazebo / MuJoCo | RGB-D cameras, joint encoders | Exp 1-5 |
| **Physical Perturbations** | Rapier / MuJoCo | Mass/friction variations | Exp 1, 3, 5 |
| **Visual Perturbations** | Logged renderer/scene events first; SparkJS Dynos in Phase 4 | Lighting, textures | Exp 1 |
| **Cross-Embodiment** | Gazebo / MuJoCo | UR5e vs Franka URDFs | Exp 5 |

### Per-Protocol Engine Mapping

| Protocol | Primary Engine | Reason |
|------------|----------------|--------|
| **H1 Protocol A** paired algorithmic response | Deterministic snapshot/clone runner; physics optional | The estimand is policy response at a frozen state, not a physical individual effect |
| **H1 Protocol B** randomized closed-loop response | Physics + Robot | Assignment, receipt, controller, contact, recovery, and physical outcomes are part of the estimand |
| **H2** prospective failure | Same closed-loop environment as deployment target | Landmarks, failures, censoring, competing events, and alarm burden must be observed prospectively |
| **H3** conditional PID increment | Inherits the active H1/H2 environment | PID is a gated feature family, not an engine or independent task protocol |
| **H4** availability–use divergence | Snapshot runner plus closed-loop confirmation where claimed | Requires decodability plus validated policy/execution interventions and controls |
| Exploratory Flow-as-Bridge | Simulator `Flow_gt` first; external predictor later | Separates measurement bring-up from video-predictor confounds |

### Modular Physics Backend Configuration

The target PID-Splat stack supports swappable physics backends. Select based on your experiment needs once those adapters exist:

```toml
# pid-splat.toml - Physics backend configuration

[physics]
backend = "rapier"  # Options: "rapier", "mujoco", "isaac"

# Rapier: Fast iteration, Rust-native, deterministic
[physics.rapier]
step_hz = 1000
deterministic = true
gravity = [0.0, 0.0, -9.81]

# MuJoCo: strong contact-physics baseline, benchmark compatibility
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
backend = "none"    # Options: "gazebo", "mujoco", "none" (default for early bring-up)
urdf_path = "assets/robots/franka_panda.urdf"
```

**Backend Selection Guide:**

| Use Case | Backend | Rationale |
|----------|---------|----------|
| Fast prototyping | `rapier` | Low-latency, no external deps |
| Benchmark comparison (LIBERO, MetaWorld) | `mujoco` | Match paper baselines |
| Large-scale ablations (GPU) | `isaac` | GPU parallelism (if available) |
| Accurate robot kinematics | `gazebo` | Industry-standard URDFs |
| Contact-rich manipulation | `mujoco` | Strong contact solver baseline |

**Coupling note (important):** if `robot.backend` and `physics.backend` differ and the robot is expected to make physical contact with simulated objects, you are in a **co-simulation** regime. Without an explicit coupling layer, robot–object contacts will not be physically meaningful. For most prisoma claims, prefer:
- **Single-engine contact:** robot + objects together in **MuJoCo** (benchmark-aligned), or
- **Harness bring-up:** the in-repo deterministic object sim first, then object-only Rapier/MuJoCo with a kinematic end-effector proxy (then add a full robot backend later).

**Robustness control (recommended):** replay a subset of episodes under both `rapier` and `mujoco` (same initial conditions + action log) and report divergence metrics. This helps rule out physics-backend artifacts across the confirmatory claims (legacy H6 safety is deferred; see `grandplan.md` §6.10 robustness/falsification checks).

### When to Use Which

**Use Rapier3D when:**
- Simulating object-object and object-table interactions in a Rust-native environment
- Running many episodes quickly (step time depends on scene + hardware)
- Applying physical perturbations (mass, friction)
- Determinism is critical for reproducibility

**Use MuJoCo when:**
- Strong contact-physics baselines are required for manipulation
- Comparing results against standard VLA benchmarks (LIBERO, MetaWorld)
- Precise grasping or multi-body dynamics are the focus

**Use Headless Gazebo when:**
- Simulating robot arm kinematics/dynamics via URDF
- Generating sensor data (RGB-D, joint states)
- Testing cross-embodiment (different robot URDFs)
- Industry-standard robot fidelity is required

### 0.5.1 Agent Bridge + Live Intervention (Protocol Requirement)

The PID‑Splat environment is specified to have a strong GUI *and* an **agent-native automation interface** (“Agent Bridge”). Experiments should be runnable:
- manually via the GUI, and
- programmatically via scripts or LLM coding tools (Claude Code/Codex/opencode-style),
without changing the experimental semantics.

**Control and reproducibility rule:** the Agent Bridge is the only control plane. Every VLA action/action chunk, intervention, scene edit, reset/step, pause/resume transition, correction force, and lifecycle command must enter through it and be appended to the canonical run log before backend dispatch. Avoid direct GUI→physics, VLA→Zenoh→physics, observer→physics, PID-triggered correction, or other hidden paths. Zenoh is data transport; Rerun and NCP observers are read-only.

**Safe-mode rule:** read-only Agent Bridge sessions should allow status/replay queries while logging and rejecting mutating/file-writing/lifecycle-ending requests. The in-repo stdio bridge smoke exposes this as `pid-sim-bridge-stdio --safe-mode`; `sim.status` and `log.replay` are accepted, while `sim.step`, `intervention.apply`, `log.stop`, and `export.rerun` are recorded as blocked bridge error responses. Outside safe mode, `intervention.apply` supports deterministic `set_velocity`, `translate_object`, and `set_pose` interventions, `log.stop` finalizes the run log without trailing events, and `export.rerun` converts a validated run log to a `.rrd` recording and records the generated artifact. The same JSON-RPC surface is also available over loopback TCP JSONL (`pid-sim-bridge-tcp --bind 127.0.0.1:PORT`) and loopback WebSocket (`pid-sim-bridge-ws --bind 127.0.0.1:PORT`), with the same canonical run-log emission and replay validation requirements.

**External simulator backends (optional):** some simulators expose an RL-style `reset/step` surface (and may already have their own WebSocket/pubsub interface). Put that native interface *behind* an Agent Bridge adapter; it is not a second prisoma control plane. The adapter must append the same command events before dispatch and emit the same observation events afterward so replay + analysis remain identical and auditable across backends.

**Backend provenance rule:** every sim-backed run should log backend, integrator, solver/contact settings, determinism settings, transport, and planned fixed-step parameters via `config_logged`; the in-repo deterministic object sim uses `deterministic_object` with a constant-velocity Euler integrator, no contact solver, `Flow_gt = pose_delta`, and a logged `constant_velocity_baseline` `flow_pred` event stream. Validation rejects mismatches between `run_started.config_hash`, `config_logged.config_hash`, and the canonical config JSON hash; summaries and manifests expose the surviving `config_hash`. Implemented local transports now cover stdio JSONL, TCP JSONL, and WebSocket JSON-RPC.

Minimum provenance fields for any action event:
- `actor_type`: `vla_policy` | `human_gui` | `script` | `llm_tool`
- `actor_id`: stable identifier (e.g., OS user, script name, tool name)
- `session_id`: run-scoped ID for an interactive session
- `request_id`: unique per API call (idempotency key)
- `payload_hash`: hash of the structured request body
- optional `prompt_hash`: hash of the LLM prompt/context that produced the action (store full text only if policy allows)

**Live intervention guidance:**
- Prefer applying interventions at a **named checkpoint** (`pause → apply → resume` or `step`) so “when” is reproducible; each transition is a separate Agent Bridge request/event, never an observer- or transport-side side effect.
- Heavy computations (video prediction, flow extraction, large bootstraps) are offline-first but should still be orchestrated through the same control plane so the artifacts and provenance are logged.

The implemented stdio/TCP/WebSocket deterministic-sim surface is **partial M2 groundwork**. Full target UI/VLA/backend command coverage and a versioned subscription stream remain to be built.

### 0.5.2 Decomposition Choice (2-way vs 3-way vs hierarchical)

Experiments must preregister which decomposition is being used and which concrete representations instantiate `(V,L,D,A)` for the tested model. Use the recovery/geometry diagnostics plus the separate MI/coherence and measure-specific atom results to decide which analyses are publishable (see `grandplan.md` §7.9 geometry diagnostics and §7.3 synthetic validation matrix).

| Decomposition | Example variables | When to use | Notes |
|---|---|---|---|
| **2-way PID (`pid2`)** | `(V,L;A)` or `(V,D;A)` | Conditional H3 only after all four gates | Most tractable atom analysis; geometry alone never supplies eligibility |
| **3-way PID (`pid3`)** | `(V,L,D;A)` | Exploratory only, after source semantics and gates pass | Expensive and measure-sensitive; report full uncertainty/sensitivity and abstention |
| **Hierarchical / screening** | pairwise PID + CI/Ω | Exploratory screening inside a validated regime | If constituent MI/measure gates fail, screen abstains rather than serving as a fallback claim |

### 0.5.3 Attribution Methods as Companion Diagnostics

Layer-wise Relevance Propagation (LRP) and related attribution methods answer a different question from PID. PID/CI estimates distribution-level dependence among random variables across logged samples, e.g. whether target-relevant information is redundant, unique, or synergistic across `V`, `L`, `D`, and `Flow`. Attribution methods explain a particular model call, layer, token, feature, region, or concept direction. Use them as **baselines and triangulation probes**, not as replacements for Experiment 0 or the geometry gate.

| Method family | Useful for | Main caveat to log/control |
|---|---|---|
| LRP / Deep Taylor | Layer-wise relevance conservation over differentiable networks for a selected output | Rule choice and architecture support affect relevance flow |
| Vanilla gradients / Input×Gradient / SmoothGrad-style ensembles | First-pass local sensitivity maps for accessible differentiable inputs or embeddings | Noisy or visually plausible maps can be model-insensitive; require randomization and deletion tests |
| Integrated Gradients | Input or embedding attribution along a baseline-to-input path | Baseline/reference choice can dominate results |
| DeepLIFT | Difference-from-reference contributions, often cheaper than IG | Reference choice and supported nonlinearities matter |
| Grad-CAM / guided Grad-CAM | Coarse visual localization on convolutional/spatial layers | Usually not token-level; spatial resolution and chosen layer matter |
| TCAV / concept activation vectors | Human-defined concept sensitivity at a layer | Concept set, random counterexamples, and statistical stability are part of the claim |
| SHAP-style / occlusion / feature ablation | Additive or perturbation-based feature importance baselines | Background distribution and feature dependence can change scores |
| Attention maps | Debugging signal, not a faithful explanation by default | Treat as a weak baseline unless intervention tests validate it |

**Attribution companion rule:** when attribution and a gated PID diagnostic are compared, use the
same sample IDs, target, and matched interventions, and report principled disagreement. Compatible
does not mean identical: gradients, relevance, occlusion, and PID measure different objects and can
diverge for valid reasons such as saturation, correlated features, or deterministic decoders.

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
# Capture settings are scene/device-specific; record the actual device, views, codec,
# resolution, lighting, and hashes in the run manifest.

# Train
ns-train splatfacto --data ./captures/table_v1/

# Export PLY with Nerfstudio
ns-export gaussian-splat \
    --load-config <config> \
    --output-dir <dir>

# Optional SPZ is a separate conversion step. Pin the converter/revision and record
# its exact command, license, and PLY/SPZ hashes; do not pass --output-format spz here.
```

**Optional 3DGS quality diagnostic (GauSS‑MI; proposed):**
- Treat reconstruction quality as a confound: if large scene regions have high residual error / low view coverage, PID features can shift for purely perceptual reasons.
- Compute a per‑Gaussian uncertainty map from held‑out view residuals and record scene‑level stats (mean/median uncertainty, fraction unreliable, `N_eff`) as artifacts.
- Optionally use uncertainty‑guided **view selection** to decide which additional camera viewpoints to capture next (log the decision as an intervention via the Agent Bridge).
- Specification lives in `GAUSS_MI_INTEGRATION.md`; treat as optional until implemented and validated.

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
| Object ID      | Nominal dimensions | Nominal mass | Physics proxy |
| -------------- | ------------------ | ------------ | ------------- |
|  red_cube        | 5×5×5 cm        | 100g | Cuboid        |
|  blue_cylinder   | r=3cm, h=8cm    | 150g | Cylinder      |
|  green_sphere    | r=4cm           | 200g | Ball          |
|  ycb_mustard     | 19×6×6 cm       | 600g | Convex Hull   |
|  ycb_spam        | 9×8×6 cm        | 350g | Cuboid        |
|  ycb_bowl        | r=8cm, h=5cm    | 180g | Trimesh       |
|  blue_plate      | r=10cm, h=1cm   | 250g | Cylinder      |
|  wooden_block_A  | 10×5×3 cm       | 120g | Cuboid        |
|  wooden_block_B  | 8×4×4 cm        | 100g | Cuboid        |

Measure and log the actual exported Gaussian count, dimensions, mass, collision proxy, and renderer memory/time for every asset; none of the nominal values above is a universal capture requirement.

**Object Capture Protocol:**
```bash
# For each object, record an object-centric capture protocol and train:
ns-train splatfacto --data ./captures/red_cube/

# Export PLY with the same command contract shown above; convert to SPZ only with
# the separately pinned converter when the selected renderer requires SPZ.
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

### 3.1 Model Selection and Staging (docset v12.5)

Follow the risk-reducing sequence in the `grandplan.md` §5.1 gate sequence and §5.2 policy/environment selection:
1) estimator/measure gate (legacy Exp0 → S1), 2) harness bring-up with `Flow_gt`, 3) small baseline (e.g., SmolVLA), 4) primary VLA target (e.g., OpenVLA), then optional branches (diffusion-based VLAs and predictor-driven `Flow_pred`).

**Model choice is an experimental variable.** Log `model_id`, revision/commit hash, preprocessing, and action parameterization for every run.

| Model | Role in this study | Minimum verified facts | Must verify before quantitative use |
|-------|---------------------|------------------------|-------------------------------------|
| **OpenVLA** | Primary target for Aim 1/2 | arXiv:2406.09246: Llama‑2 7B + (DINOv2, SigLIP) + ~970k demos | Action representation; exact hook points for `V/L/D`; whether/where to export pre-attention states; licensing + checkpoint provenance |
| **SmolVLA** | Harness bring-up baseline | arXiv:2506.01844 (~450M; SmolVLM-2 backbone; flow-matching action expert — paper-reported; verify checkpoint revision) | Available intermediate dumps, licensing, revision-specific config |
| **InternVLA‑A1** | Diffusion / flow-matching ablation axis (optional) | Repo + project page describe a tripartite understanding/generation/action design; action generation via “Flow Matching” (verify) | License constraints (verify upstream; may be restrictive); what the generation expert outputs (`D_gen`) and how to export it; action parameterization (“delta actions”); patched Transformers constraints; do not confuse “Flow Matching” (a generative method) with this project’s geometric `Flow_*` variables |
| **TraceVLA** | Temporal/history axis | arXiv:2412.10345: finetuned OpenVLA; trace-based prompting; 150K trajectories; compact Phi‑3‑Vision variant | How traces are encoded and how to separate “image vs trace” variables in logs |
| **DreamVLA** | Within-model explicit-`D` stage/channel ablation axis | arXiv:2507.04447: world-knowledge forecasting (dynamic/spatial/semantic cues), block-wise structured attention, and diffusion-based action modeling (abstract; optionally keep a local PDF under `.external/papers/` for offline reading). Code/weights are referenced as `WenyaoZhang/DreamVLA` (verify availability + license at time of use). | Backbone family/dims and exportable tensors; output formats/dims of explicit channels; what is exposed per step; verify dynamic-region/spatial/semantic cue extraction points |
| **PixelVLA** | Pixel-aligned diagnostics (if integration supports it) | arXiv:2511.01571: pixel-level reasoning + multimodal prompting; Pixel‑160K; reported gains | What variables are exposed (pixel maps vs pooled); API/backbone; dataset access/licensing |
| **PI “π” series** (`π0`, `π0.5`, `π0.6*`) | Closed-source comparator (optional) | PI vendor papers/blog posts (see `grandplan.md` §10.2 VLA diagnosis / §5.2 policy selection). Treat training/perf claims as vendor claims until replicated | Access mode (API vs weights); whether per-step embeddings/hidden states can be exported (required for internal PID); determinism/replay; licensing/ToS constraints |

### 3.2 `V/L/D/A` Extraction Contract (Model-Agnostic)

Treat `grandplan.md` §9.1 (do not begin with the labels V, L, and D) and §9.2 (pathway-source experiments) as the contract: do not assume layer names or fixed tensor shapes.

**Per-run definitions (log them):**
- `V`: a pre-fusion vision representation (choose and pin a hook point).
- `L`: a text/instruction representation (token pool or layer summary).
- `D`: a world/plan representation. Candidate definitions include:
  - `D_explicit`: model-exposed world-knowledge cues (preferred when available).
  - `D_hidden[k]`: selected hidden state(s) (layer choice is part of the experiment).
  - `D_fused`: post-fusion representation mixing modalities.
- `A`: action output (continuous or discrete; representation is model-specific).

**Extraction rule:** export *multiple* candidate `D` definitions in early bring-up runs. Later, preregister which `D` is “primary” for each model.

**Causal-comparison rule:** do not interpret DreamVLA-versus-OpenVLA PID differences as the effect of an explicit world model. Model family, training data, action head, and even the operational definition of `D` all differ. Use within-checkpoint DreamVLA channel/stage ablations (or equivalent within-model interventions) for causal claims; treat cross-model comparisons as descriptive replication only.

**Minimal extraction sketch (pseudocode):**
```python
def run_inference(model, processor, image, instruction):
    inputs = processor(images=image, text=instruction)
    outputs = model(**inputs, output_hidden_states=True)

    action = extract_action(outputs)  # tokens, bins, or continuous vector (model-specific)

    reps = {
        # placeholders: adapt to your model and log the exact hook names
        "V": extract_vision_rep(outputs),
        "L": extract_language_rep(outputs),
        # export multiple candidates during bring-up
        "D_hidden_16": extract_hidden(outputs, layer=16),
        "D_fused": extract_fused_rep(outputs),
    }

    return action, reps
```

**Action representation note:** do not assume discretization or “256 bins”. If your model uses binning, log the binning scheme (range, clipping, bin centers) and treat it as part of the experimental setup.


### 3.3 Dimensionality Reduction for PID
Raw embeddings may be high-dimensional, so reduction is a candidate measurement regime—not an automatic repair. Fit it once on disjoint V0/W0 training data, freeze it across every perturbation cell, and log a hash of the serialized transform. Never z-score or fit PCA independently per episode/condition.

```python
from sklearn.decomposition import PCA
from sklearn.pipeline import make_pipeline
from sklearn.preprocessing import StandardScaler
from sklearn.random_projection import GaussianRandomProjection

class EmbeddingReducer:
    """One train-fitted, frozen transform per axis; illustrative pseudocode."""

    def __init__(self, target_dim: int, method: str, seed: int):
        self.target_dim = target_dim
        self.method = method
        self.seed = seed
        self.axes = {}

    def _pipeline(self, axis_seed: int):
        if self.method == "pca":
            reduce = PCA(n_components=self.target_dim)
        elif self.method == "random_projection":
            reduce = GaussianRandomProjection(
                n_components=self.target_dim,
                random_state=axis_seed,
            )
        else:
            raise ValueError(f"unknown method: {self.method}")
        return make_pipeline(StandardScaler(), reduce)

    def fit_on_disjoint_v0_w0(self, train_by_axis: dict):
        for i, axis in enumerate(("V", "L", "D", "A")):
            pipe = self._pipeline(self.seed + i)
            self.axes[axis] = pipe.fit(train_by_axis[axis])
        # Serialize scaler/reducer parameters and record their cryptographic hash.
        self.transform_hash = hash_serialized_transforms(self.axes)
        return self

    def transform(self, axis: str, rows):
        # Reuse this exact object/hash for baseline and every perturbation cell.
        return self.axes[axis].transform(rows)
```

PCA/random projection still require recovery and geometry/dependence validation on all resulting estimator inputs and concatenations. PLS or any other target-supervised transform additionally requires train-only fitting and nested selection; it defines a distinct preregistered regime.

---
 
## 4. Experiment 0: Estimator Validation
Purpose: separately validate measure-independent MI/coherence and measure-specific atoms for the exact target pipeline before interpreting real experiments.

### 4.0 Geometry, Dependence, and Recovery Diagnostics (REQUIRED)

**Critical:** before running PID estimation, diagnose every individual variable and every concatenation actually passed to the estimator. Geometry diagnostics are supporting evidence; recovery on controls and the separate MI/atom gates decide eligibility.

#### Why This Matters

The implemented KSG/ISX estimators use Chebyshev/L∞ neighborhoods. Their behavior depends on intrinsic dimension, concentration/ties, dependence, local geometry, sample size, and preprocessing. Sampled mean Gromov `δ_rel` describes tree-likeness; it does **not** establish that Euclidean kNN is invalid and is not a pass/fail gate.

#### Geometry Diagnostics

| Diagnostic | Method | Scientific use | Action on warning |
|------------|--------|----------------|-------------------|
| **Estimator recovery** | Synthetic controls passed through the exact frozen transform | Primary empirical evidence that the pipeline recovers analytic MI and measure-specific atoms | Pivot estimator/representation; re-run all diagnostics |
| **Intrinsic dimension** | Levina–Bickel (kNN MLE) plus sensitivity to `k` | Calibrate against recovery controls; no universal cutoff | Reduce/change representation or increase independent sampling |
| **Distance concentration and ties** | Pairwise/nearest-neighbor summaries, duplicate/tie counts | Detect neighborhood degeneration under the estimator metric | Pivot metric-compatible representation/estimator and re-test |
| **Dependence / effective sampling** | Episode structure, autocorrelation/decorrelation diagnostics | Prevent treating correlated frames as independent rows | Stride/block/group at the episode level and re-size capture |
| **Local flatness** | Calibrated neighborhood residual/curvature diagnostics | Supporting check for the chosen local metric | Change representation or estimator; validate on controls |
| **Sampled mean `δ_rel`** | Gromov four-point sampling under a stated normalization | Descriptive tree-likeness statistic only | Report it; never use it alone to pass/fail Euclidean kNN |

#### Running Geometry Diagnostics

```python
import pid_core_rs as pid

# Run this for V, L, D, A, Flow, and every source/target concatenation used.
# 1. Report intrinsic dimension (calibrate against recovery controls; no fixed cutoff)
id_report = pid.intrinsic_dimension_report(embeddings, k=10)
print(f"intrinsic dimension estimate: {id_report.estimate:.1f}")

# 2. Report concentration/ties and compare to the validated control envelope
distance_report = pid.distance_concentration_report(embeddings)
print(f"pairwise CV: {distance_report.pairwise_cv:.3f}")

# Hyperbolicity is not part of the ordinary pid_core_rs 1.0 wheel. The Rust harness records its
# sampled four-point summary as a descriptive diagnostic; do not invent a Python stable call or
# use that statistic as scientific eligibility.
```

#### Hyperbolic/Lorentzian Limitation

> **⚠️ IMPORTANT**: The implemented ISX estimator (`EhrlichKsg`) **only supports Chebyshev (L∞) metric**. Hyperbolic/Lorentzian PID estimation is not currently supported, and continuous-atom validation is not yet covered by a valid automated gate.
>
> **Mitigation**: Use **Flow-as-Bridge** (see Experiment 4, §8). A low-dimensional 3D object-flow target can reduce target-side geometry and ambient-dimension burden, but it does not validate the source variables or their concatenations. Re-run recovery, dependence, intrinsic-dimension, concentration/tie, and local-flatness checks; raw flow can itself be high-dimensional as \(\mathbb{R}^{3T}\).

#### Scientific Eligibility Summary

| Check | Required evidence | Fail action |
|-------|-------------------|-------------|
| MI/coherence | Analytic recovery and coherence on the exact frozen pipeline | NO-GO/PIVOT for that MI pipeline |
| Measure-specific atoms | Committed oracle plus pinned independent cross-check with uncertainty | Do not interpret continuous atoms |
| Sampling/geometry | Recovery-calibrated intrinsic dimension, concentration/ties, dependence, and local-flatness diagnostics on every estimator input/concatenation | Change representation/sampling/estimator and repeat |
| Sampled mean `δ_rel` | Report only | Never a stand-alone fail action |

The current harness's `--require-geometry-pass` is an implementation-level fail-closed switch, not sufficient scientific eligibility by itself. Reports must expose the component diagnostics; publication decisions follow the corrected criteria above, not a legacy aggregate that includes a `δ_rel` threshold.

### 4.1 Synthetic Test Cases
```python
import numpy as np

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
        # NOTE (docset v12.5): "additive" ≠ "independent contributions". Under I^sx the true
        # redundancy here is genuinely POSITIVE (~0.2 nats; oracle-confirmed upstream
        # in pid-rs) — the Red≈0 expectation is an MMI convention I^sx does not
        # satisfy; and at low noise the joint MI far exceeds I(S1;T)+I(S2;T), so
        # synergy is large, not ≈0. Use this scenario for MI-coherence gates (§4.2),
        # NOT as an atom-level ground truth.
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

### 4.2 Acceptance Criteria: Separate MI/Coherence and Atom Gates

Do not hard-code “safe” dimensions or generic %-error cutoffs. Validate the **exact frozen preprocessing pipeline** and keep measure-independent MI recovery separate from measure-specific atom recovery.

**MI/coherence gate criteria:**
- No `NaN`/`Inf` in the relevant MI terms/invariants.
- **Monotonicity:** `I(S₁,S₂;T) ≥ I(S₁;T)` and `I(S₁,S₂;T) ≥ I(S₂;T)`.
- **CMI nonnegativity (via identities):** `I(S₁;T|S₂)=I(S₁,S₂;T)−I(S₂;T) ≥ 0` and similarly for `I(S₂;T|S₁)`.
- **Shannon-invariant sanity (Gutknecht et al. 2025):** for `n` sources and nonzero `I(S₁…S_n;T)`, require `0 ≤ \bar{r} ≤ n` and `0 ≤ \bar{v} ≤ n`; treat `\bar{r}` or `\bar{v}` outside these bounds as “estimator incoherence”.
- **Independent-noise MI invariance:** in controls where nuisance dimensions are analytically irrelevant, the relevant Gaussian-channel MI terms should remain within measured uncertainty. This does **not** imply continuous shared-exclusions redundancy is invariant to those dimensions.

**Continuous-atom gate criteria:**
- Use a committed Gaussian `I^sx_∩` oracle in its supported low-dimensional regime and a pinned independent implementation.
- Remove the known-false additive-control `Red≈0` target: it is an MMI expectation, not an `I^sx_∩` truth.
- Treat high-dimensional atom drift as functional-plus-estimator sensitivity until a dimension-specific oracle exists.
- Propagate uncertainty and require agreement under the exact frozen transform; MI/coherence success alone cannot pass this gate.

**Decision rule:**
- **MI_GATE GO/PIVOT/NO-GO:** decide from analytic MI recovery and coherence only. The current default high-dimensional sweep is **NO-GO**.
- **ISX_GATE GO/PIVOT/NO-GO:** decide only from the measure-specific oracle/cross-check above. This valid automated gate is **not yet implemented**, so current continuous atoms remain uninterpretable on real embeddings.
- `--strict-gate` does not change that status: it implies `--strict-band` and exits with code 3 unless a curated d=1 Gaussian grid passes its **three measure-independent MI checks**. The lower-dimensional four-scenario diagnostics, default high-dimensional sweep, and atoms are non-gating for that flag.

#### 4.2.1 Attribution Probe Sanity Checks (Separate From PID Gates)

Attribution methods do not inherit the kNN geometry assumptions above, but they need their own faithfulness checks before they can serve as exploratory H4 companion evidence or a strong baseline for H1/H3:

- **Model randomization:** attribution maps/scores should change when model parameters are randomized; otherwise they may be input edge detectors rather than model explanations.
- **Data/random-label randomization:** a model trained on randomized labels should not yield the same explanatory structure as the trained task model.
- **Reference/background sensitivity:** report IG/DeepLIFT baselines, SHAP background sets, TCAV concept/counterexample sets, and LRP rules; repeat enough variants to show conclusions are not a single-reference artifact.
- **Smoothing/noise sensitivity:** for SmoothGrad/VarGrad-style maps, report perturbation distribution, noise scale, sample count, and whether the conclusion survives unsmoothed and smoothed variants.
- **Deletion/occlusion or perturbation tests:** removing or corrupting top-attributed pixels/tokens/features should change the target more than removing low-attributed or random features, under a controlled replacement distribution.
- **Jitter and seed stability:** scores should be stable enough under mild input jitter, bootstrap resampling, and attribution hyperparameter changes for the downstream comparison being claimed.
- **Attention caveat:** attention entropy/maps are allowed as weak baselines, but should not be called explanations unless intervention tests validate that changing attended features changes the model output.

These checks can pass even when PID gates fail, and vice versa. Treat that as a measurement-regime distinction, not a contradiction.

### 4.3 Running Experiment 0
Run the implemented Rust Experiment 0 diagnostics (this repo):

```bash
just exp0
just exp0-bin
just exp0-runlog

# Optional CSV output for analysis
cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all --bin exp0 -- --csv > exp0_results.csv

# Optional canonical evidence export
cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all --bin exp0 -- --summary-json outputs/exp0_summary.json --runlog outputs/exp0_runlog.jsonl
cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/exp0_runlog.jsonl
cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --write-sidecars outputs/exp0_runlog.jsonl
cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --verify-sidecars outputs/exp0_runlog.jsonl

# Optional curated d=1 Gaussian MI gate (not an atom/high-d gate)
cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all --bin exp0 -- --strict-gate
```

**Note:** A larger Python experiment harness is specified in `grandplan.md`/`EXPERIMENTS.md` but is not implemented in this repo yet; avoid citing non-existent scripts as runnable.

**Current results:** the default high-dimensional MI/coherence path is **NO-GO**. Continuous `I^sx_∩` atom validation has no valid automated gate yet, so the existing aggregate verdict must not be presented as an atom-validation verdict. See the current `grandplan.md` corrective addendum and `findings.md`.

### 4.4 Tiny Labeled Harness Smoke

This repo includes a deterministic toy VLA/task harness that produces first-class success-label events, a replay-linked toy `(V,L,D,A)` embedding contract, PID/CI features over `(vision, language; action)`, non-PID baseline accuracies, a summary JSON artifact, and a canonical run log. It is a software integration smoke, not evidence for a real VLA policy.

```bash
just toy-harness

# Equivalent without just
cargo run -p pid-sim --bin pid-toy-harness -- --summary-json outputs/toy_vla_summary.json --runlog outputs/toy_vla_runlog.jsonl
cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/toy_vla_runlog.jsonl
cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --write-sidecars outputs/toy_vla_runlog.jsonl
cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --verify-sidecars outputs/toy_vla_runlog.jsonl
```

### 4.5 Offline VLDA Embedding Harness Smoke

`pid-offline-harness` converts captured sample JSON with `(V,L,D,A)` vectors into a canonical summary JSON plus run-log JSONL. The checked fixture lives at `crates/pid-sim/fixtures/offline_vlda_fixture.json`; each sample has a `sample_id`, optional `episode_id`, numeric `v`/`l`/`d`/`a` vectors, optional labels, and optional string metadata. The run log records `run_started`, `config_logged`, `frame_observed`, `label_observed`, `embedding_contract`, `embedding_captured`, two-source PID metrics for all `V/L/D→A` source pairs—`(V,L;A)`, `(V,D;A)`, and `(L,D;A)`—after deterministic per-variable standardization, plus train-split-only PID screens with train-only standardization when a recognized `metadata.split` exists, geometry diagnostics/gates over the standardized analysis space, evaluation metrics including deterministic sample-level, episode-grouped, and metadata-split held-out majority/1-NN/nearest-centroid **and SAFE-class logistic-regression (`heldout_logreg_vlda`; train-fit, held-out-scored)** success-label baselines when boolean `success` labels plus the relevant `episode_id`/`metadata.split` provenance are present, input/summary artifacts, and `run_ended`. A recognized held-out split uses `metadata.split=train`/`training` for training samples and `test`/`validation`/`val`/`eval`/`evaluation`/`heldout`/`holdout`/`held_out`/`hold_out` for held-out samples; summaries preserve split counts and sample IDs. Held-out baselines report accuracy and, when both held-out classes exist, balanced accuracy. Nearest-centroid baselines are train-standardized, train-only, require both success classes in the train split, and emit AUROC from the signed centroid-distance score when both held-out success classes are present. Summaries and run logs also include train-split PID status/provenance, held-out class-coverage status/counts, episode-disjointness status/counts for `episode_id` leakage, held-out per-sample prediction records in summaries/run logs, and failure-class confusion/rate diagnostics for majority, 1NN, and centroid baselines so missing train/held-out classes, split episodes that leak across train/held-out subsets, train-only PID availability, misclassified held-out samples, nearest train exemplars, centroid scores, failure recall, and false alarms can be audited. Replay summaries keep `*_metrics` as unique latest-by-name metric-name counts and add `*_metric_events` counters for total metric event volume.

```bash
just offline-harness
just offline-harness-require-labels
just offline-harness-require-heldout
just offline-harness-require-heldout-class-coverage
just offline-harness-require-heldout-episode-disjoint
just offline-harness-strict

# Equivalent without just
cargo run -p pid-sim --bin pid-offline-harness -- --input crates/pid-sim/fixtures/offline_vlda_fixture.json --summary-json outputs/offline_vlda_summary.json --runlog outputs/offline_vlda_runlog.jsonl
cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --validate outputs/offline_vlda_runlog.jsonl
cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --write-sidecars outputs/offline_vlda_runlog.jsonl
cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- --verify-sidecars outputs/offline_vlda_runlog.jsonl
```

Use `--require-success-labels` for fail-closed labeled-analysis runs, `--require-heldout-split` for fail-closed train/held-out baseline runs, `--require-heldout-class-coverage` when both train and held-out subsets must contain boolean success/failure labels, `--require-heldout-episode-disjoint` when held-out episodes must be disjoint from train episodes, and `--require-geometry-pass` for fail-closed geometry-gated runs. In those modes the CLI exits nonzero if required labels, held-out split baselines, held-out class coverage, held-out episode disjointness, or geometry pass status are unavailable, but it still writes a canonical summary and a valid run log with `run_ended.status = failed` plus an `error_logged` record for provenance.

This harness is an artifact-to-runlog converter for embedding captures. It still requires a real model/task capture process and externally meaningful labels before it can support VLA claims.

---
 
## 5. Experiment 1: Pick-and-Place (Baseline)
**Protocol role (choose one before capture):** H1 **Protocol A** frozen-snapshot response prediction,
H1 **Protocol B** randomized closed-loop effect moderation, or H2 prospective failure prediction.
The same episodes may support hierarchically secondary analyses, but their units/endpoints and claim
language remain separate. PID is only a conditional H3 feature family after all four gates; attribution
is exploratory triangulation.

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
from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple

import numpy as np

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
    # Target: Action (diagnostic only; an H1/H2 use requires protocol-specific scoring)
    pid_action_synergy: np.ndarray        # (T/6,)
    pid_action_redundancy: np.ndarray     # (T/6,)
    pid_action_unique_v: np.ndarray       # (T/6,)
    pid_action_unique_d: np.ndarray       # (T/6,)
    
    # Target: 3D Flow (exploratory Flow-as-Bridge diagnostic)
    pid_flow_synergy: np.ndarray          # (T/6,)
    pid_flow_redundancy: np.ndarray       # (T/6,)
    pid_flow_unique_v: np.ndarray         # (T/6,)
    pid_flow_unique_d: np.ndarray         # (T/6,)
    
    pid_co_information: np.ndarray        # (T/6,)
```

Exploratory attribution artifacts use the implemented first-class `attribution_logged` event: method, target output, layer, modality, baseline, score hash, faithfulness check, and artifact URI. `experiments/attribution/` currently produces epsilon-/AttnLRP and gradient×input evidence on a small reference model with deletion-AOPC vs random; the Rerun adapter surfaces faithfulness/provenance and compatible NumPy relevance values (capped at 1024). Record any additional preprocessing/stability metadata in the linked artifact/manifest. Production VLA/LXT hooks remain future work.

### 5.4 PID Computation
```python
import numpy as np
import pid_core_rs as pid

def zscore(x: np.ndarray, eps: float = 1e-8) -> np.ndarray:
    mu = x.mean(axis=0, keepdims=True)
    sd = x.std(axis=0, keepdims=True) + eps
    return (x - mu) / sd

def pid2_isx_window(s1: np.ndarray, s2: np.ndarray, t: np.ndarray, k: int = 3) -> dict:
    """
    Deliberately non-runnable marker for the old pre-1.0 Python sketch.

    Ordinary pid_core_rs 1.0 wheels do not expose continuous shared-exclusions PID. Use the
    feature-pinned Rust offline harness, which records support declarations, computation status,
    four separate scientific-gate verdicts, and abstentions. Do not reconstruct atoms from legacy
    scalar Python calls or enable the migration module in an evidentiary workflow.
    """
    raise NotImplementedError("continuous PID runs through the Rust harness under pid-rs 1.0")

def compute_episode_pid(
    episode: PickPlaceEpisode,
    frozen_transforms: dict,
    window_size: int = 20,
    k: int = 3,
) -> dict:
    """
    Compute PID metrics over sliding windows.
    
    Computes two decompositions:
    1. Target = Action (diagnostic; H1 use requires Protocol A/B scoring)
    2. Target = Flow   (exploratory Flow-as-Bridge measurement)
    
    Warning (docset v12.5): 20-sample windows inside one autocorrelated episode are NOT an
    estimable kNN-PID regime (grandplan.md §2.5, §6.7). For any windowed claim, pool
    phase-aligned windows ACROSS episodes with a within-window stride ≥ the
    decorrelation length; per-episode windows are exploratory visualization only.
    """
    n_samples = len(episode.embeddings_V_reduced)
    
    # Prepare targets
    # 1. Action (Joint Velocities) - Downsampled to 5Hz
    actions = episode.actions_commanded[::6]
    
    # 2. 3D Flow (Object Position)
    target_object = "red_cube"
    flow_3d = episode.object_poses[target_object][:, :3]
    flow_3d = flow_3d[::6]
    
    # Apply transforms fitted once on disjoint V0/W0 training data. Never refit per
    # episode or perturbation cell; persist frozen_transforms[axis].hash in the run log.
    V = frozen_transforms["V"].transform(episode.embeddings_V_reduced).astype(np.float64)
    D = frozen_transforms["D"].transform(episode.embeddings_D_reduced).astype(np.float64)
    A = frozen_transforms["A"].transform(actions).astype(np.float64)
    T = frozen_transforms["Flow"].transform(flow_3d).astype(np.float64)

    # Report diagnostics on representative pooled batches for every estimator input and
    # concatenation (V, D, A, Flow, [V,D], ...), not per episode. Calibrate ID/CV/ties/
    # local-flatness readings against recovery controls; no universal numeric threshold.
    # Sampled mean δ_rel may also be reported, but is descriptive and never changes a gate.
    
    results = {
        "action": {"syn": [], "red": [], "unq_v": [], "unq_d": []},
        "flow":   {"syn": [], "red": [], "unq_v": [], "unq_d": []}
    }
    
    for i in range(0, n_samples - window_size, window_size // 2):
        # Window slicing (n_window × d)
        win_V = V[i : i + window_size]
        win_D = D[i : i + window_size]
        win_A = A[i : i + window_size]
        win_T = T[i : i + window_size]
        
        # 1. PID(V, D -> Action)
        res_a = pid2_isx_window(win_V, win_D, win_A, k=k)
        results["action"]["syn"].append(res_a["syn"])
        results["action"]["red"].append(res_a["red"])
        results["action"]["unq_v"].append(res_a["unq_s1"])
        results["action"]["unq_d"].append(res_a["unq_s2"])
        
        # 2. PID(V, D -> Flow)
        res_f = pid2_isx_window(win_V, win_D, win_T, k=k)
        results["flow"]["syn"].append(res_f["syn"])
        results["flow"]["red"].append(res_f["red"])
        results["flow"]["unq_v"].append(res_f["unq_s1"])
        results["flow"]["unq_d"].append(res_f["unq_s2"])
    
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
```

### 5.5 Evaluation Metrics
```python
def evaluate_exp1(episodes: List[PickPlaceEpisode]) -> dict:
    """Compute all evaluation metrics for Experiment 1"""
    
    # Basic performance
    success_rate = np.mean([e.success for e in episodes])
    
    # Separate by outcome
    success_eps = [e for e in episodes if e.success]
    failure_eps = [e for e in episodes if not e.success]
    
    # PID metrics (conditional H3 feature family only after all four gates)
    syn_success = np.concatenate([e.pid_action_synergy for e in success_eps])
    syn_failure = np.concatenate([e.pid_action_synergy for e in failure_eps])
    
    # Exploratory screen only; this does not implement H1 Protocol A/B or prospective H2.
    # Example: use synergy at 50% of episode as a simple predictor (replace with preregistered model).
    synergy_midpoint = []
    labels = []
    for e in episodes:
        mid_idx = len(e.pid_action_synergy) // 2
        synergy_midpoint.append(e.pid_action_synergy[mid_idx])
        labels.append(1 if e.success else 0)
    
    # AUROC: can synergy predict success?
    from sklearn.metrics import roc_auc_score  # requires scikit-learn
    auroc_synergy = roc_auc_score(labels, synergy_midpoint)
    
    # Statistical tests
    from scipy.stats import mannwhitneyu, ttest_ind
    stat, pvalue = mannwhitneyu(syn_success, syn_failure, alternative='two-sided')  # sign is a candidate feature, not an assumption (grandplan.md Warning 1)
    
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

**Binding note (docset v12.5):** the snippet above is illustrative. For Protocol A, freeze the
snapshot boundary and response functional, then score a train-fitted predictor directly against
held-out paired algorithmic responses with calibration and response-reliability reporting. For
Protocol B, report overall ITT before the locked effect-specific R-loss/doubly robust loss, causal
calibration, prioritization, and policy-value/regret stack. For H2, freeze the landmark/horizon,
censoring/competing-event rule, and alarm policy, then use the prespecified proper predictive score.
No generic episode-level ΔAUROC substitutes for these protocol-specific endpoints. Every report
uses the §6.5 mandatory baseline frontier at matched information access and compute.

### 5.6 Attribution Baselines and Exploratory H4 Triangulation

For the same episodes, run a small set of faithfulness-checked attribution probes if the model exposes the required gradients/layers:

1. **Vision:** Grad-CAM or LRP/IG over visual features/patches; summarize relevance on task objects, distractors, target zones, and safety-critical regions.
2. **Language:** Integrated Gradients, DeepLIFT, LRP, or occlusion over instruction tokens/embeddings; summarize relevance on object, relation, negation, and constraint tokens.
3. **Concepts:** TCAV-style probes for human-defined concepts such as target color, object shape, distractor, collision-risk, or “avoid” constraints when concept examples can be defined without label leakage.
4. **Sensitivity maps:** vanilla gradient, Input×Gradient, SmoothGrad/VarGrad-style ensembles, or embedding-gradient probes as cheap baselines when differentiable hooks exist.
5. **Black-box/embedding baseline:** SHAP-style, permutation, or occlusion importance over reduced `V/L/D/A` features when gradients are unavailable.

Compare attribution summaries against PID/CI features using preregistered tests:
- If `Unq(L;A)` is high, language token/embedding attribution should be stronger than matched visual attribution under language-sensitive conditions, unless an intervention shows PID is tracking correlated nuisance information.
- If `Unq(V;A)` is high, visual relevance should localize to task-relevant objects/regions more than to distractors.
- If `Syn(V,L;A)` is high, removing either modality’s top-attributed features should produce a larger action/failure change than expected from each modality alone; test this with paired ablations rather than heatmap inspection.
- If attribution and PID disagree, run targeted perturbations and report the disagreement as evidence about method scope, saturation, feature correlation, or estimator failure.

Do not choose PID preprocessing by looking at held-out attribution/failure labels. Any learned feature selection, concept classifier, PCA, SAE, or background distribution must be fit on the training split and replayed on held-out data with logged hashes.


---
 
## 6. Experiment 2: Long-Horizon Assembly (Temporal)
**Protocol role:** H2 prospective landmark prediction when a failure horizon, censoring/competing
events, and alarm policy are frozen; otherwise the temporal-synergy analysis is exploratory only.
The retired H5 atom-trend claim is not confirmatory and cannot be promoted without the H3 gates.

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

> **⚠️ docset v12.5 binding rule (`grandplan.md` §9.4, §2.5, §6.7):** the legacy-H5 (now Exploratory) endpoint is computed on
> phase-aligned windows **pooled across episodes** — never on per-window kNN estimates inside a
> single autocorrelated trajectory (N≈tens per window is not estimable). The per-episode
> function below is retained **only** as an exploratory visualization aid; its outputs are
> excluded from the temporal-synergy evidence. The preregistered statistic is the trend (slope /
> early-vs-late contrast) of pooled windowed `Syn(V_t,D_t;A_t)` — plus the mandatory CI-only
> twin (`grandplan.md` §3.8) — against composition-stage index, with episode-level block bootstrap CIs
> and outcome stratification. Windowing parameters are frozen in §6.5 before capture.

```python
def analyze_temporal_pid(episode: StackingEpisode) -> dict:
    """
    EXPLORATORY VISUALIZATION ONLY — not temporal-synergy evidence (see the §6.4 note /
    grandplan.md §9.4). Per-episode phase summaries for plotting; the temporal
    endpoint pools windows across episodes.
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

### 6.5 Temporal-Synergy Windowing Preregistration (docset v12.5; frozen before capture)

Per `grandplan.md` §9.4 (token/temporal aggregation), the following parameters are fixed here **before the first real
capture** and may not be changed after any temporal-synergy data is seen. Values marked TBD are set by the
`grandplan.md` §6.8 simulation-based power/design analysis and frozen (committed to this section) at that point;
running the capture with any value still TBD is a protocol violation.

| Parameter | Value | Rule |
|---|---|---|
| Window definition | task phase (approach/grasp/transport/place) where annotatable; else fixed fractions of normalized episode time | preregistered per task family |
| Window count | **TBD by `grandplan.md` §6.8 power/design analysis** | frozen before capture |
| Within-window stride | ≥ estimated decorrelation length (or cap ≤ m frames/episode/window; m TBD) | `grandplan.md` §2.5/§6.7 inherited fully — consecutive same-episode frames bias kNN point estimates |
| `N_win` (min pooled samples per window) | **TBD** — must be ≥ the S1-validated minimum for the active regime; counts **post-stride** samples | capture is scaled or the temporal-synergy analysis downgraded if unreachable |
| Episodes-contributing count | reported per window | mandatory |
| Outcome stratification | trend stratified (or covaried) by episode outcome | failed episodes stall/time out; late fixed-fraction windows are not phase-comparable otherwise |

Primary construct: pooled windowed `Syn(V_t, D_t; A_t)` + mandatory CI-only twin (`grandplan.md` §3.8).
Secondary (exploratory only): `Syn(V_t, V_{t−h}; A_t)`. Endpoint: `grandplan.md` §6 statistical analysis plan (temporal-synergy trend).

---

## 7. Experiment 3: Instruction Perturbation (Robustness)
**Protocol role:** an H1 Protocol A/Protocol B intervention pilot or an H4 availability–use study,
selected before capture. Protocol A uses paired frozen snapshots; Protocol B uses randomized
closed-loop assignment and ITT-first analysis. The legacy redundancy/unique rank-correlation rows
below are exploratory precursors to H3, not confirmatory endpoints. **Legacy H6 safety is deferred**
(`grandplan.md` §4): the `safety_constraint` condition is logging-only unless process-level safety
outcomes and matched controls are separately preregistered.

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
**Exploratory legacy H2/H3 analyses (non-operative for confirmation):** first pass the instruction
diversity/occupancy/entropy gate; otherwise make V–D primary. Fit preprocessing once on disjoint
training data and freeze its hash. Redundancy-versus-severity slopes and unique-ordering-versus-
intervention-ordering correlations may be reported as exploratory H3 mechanism screens only after
the four PID gates. Strength matching is outcome-independent (for example equal embedding
displacement or equal MI destroyed), never equal-success-impact. These screens do not replace the
Protocol A direct-response score, the Protocol B effect-specific stack, or H2’s prospective proper
score.
**Exploratory (legacy-H6-adjacent, Deferred — no claim):** `Unq(L)` / `Syn(V,L;A)` contrasts between the `safety_constraint` and baseline conditions may be logged but are not evidence for any safety claim.

---
 
## 8. Experiment 4: Dream2Flow Validation (Flow-as-Bridge)
**Protocol role:** exploratory Flow-as-Bridge engineering/measurement study (`grandplan.md` §9.6),
not a confirmatory H claim. Any later use as an H1 moderator or H2 predictor must be frozen before
capture and evaluated under that protocol’s endpoint.

### 8.0 Flow_gt-First Bring-Up (Recommended)

Before introducing a stochastic video predictor, validate the Flow-as-Bridge pipeline using **simulator-derived ground truth flow** (`Flow_gt`) computed from logged object poses. This isolates the core scientific question—whether flow is a useful Euclidean diagnostic target—from predictor confounds.

**Protocol (minimal):**
1. Run episodes in simulation and log per-object poses over time.
2. Define `Flow_gt` as a low-dimensional per-object summary (e.g., centroid position/velocity over a short window); log the aggregation method.
3. Compute CI screening and (if gates pass) targeted SxPID on candidate decompositions such as `(V, L; Flow_gt)`, `(V, D_vla; Flow_gt)`, and `(V, Flow_gt; A_cmd)` under matched controls.
4. Only after this passes, add predictor-driven `Flow_pred` below to study embodiment-gap and world-model questions.

### 8.1 Video Generation Setup (External; Optional)

This repository does **not** ship a WAN/video-model runner. Use any image+instruction→video model (local or API) and treat the model choice as an *experimental variable*.

**Required logging (per generated clip):**
- Model identifier + revision/commit hash + license
- Prompt template and conditioning parameters
- Seed(s)
- FPS, number of frames, resolution
- Any pre/post-processing applied to frames

Store the generated clip as `video: (T, H, W, 3) uint8` plus per-frame timestamps.
 
### 8.2 3D Flow Extraction Pipeline
```python
import numpy as np

def lift_tracks_to_3d(tracks_2d: np.ndarray, depths: np.ndarray, K: np.ndarray) -> np.ndarray:
    """
    Lift tracked 2D points into 3D trajectories using per-frame depth.

    Inputs:
    - tracks_2d: (N, T, 2) pixel coords (u, v) for N objects/points
    - depths:    (T, H, W) depth maps (relative or metric; document which)
    - K:         (3, 3) camera intrinsics matrix

    Output:
    - tracks_3d: (N, T, 3) 3D points in the camera frame (units follow depth units)
    """
    tracks_2d = np.asarray(tracks_2d, dtype=np.float64)
    depths = np.asarray(depths, dtype=np.float64)
    K = np.asarray(K, dtype=np.float64)

    fx, fy = K[0, 0], K[1, 1]
    cx, cy = K[0, 2], K[1, 2]

    n, t, _ = tracks_2d.shape
    _, h, w = depths.shape

    tracks_3d = np.zeros((n, t, 3), dtype=np.float64)
    for i in range(n):
        for j in range(t):
            u, v = tracks_2d[i, j]
            uu = int(np.clip(np.round(u), 0, w - 1))
            vv = int(np.clip(np.round(v), 0, h - 1))
            z = depths[j, vv, uu]
            x = (u - cx) * z / fx
            y = (v - cy) * z / fy
            tracks_3d[i, j] = (x, y, z)

    return tracks_3d
```

### 8.3 Dream2Flow PID Computation
```python
import numpy as np
import pid_core_rs as pid

def zscore(x: np.ndarray, eps: float = 1e-8) -> np.ndarray:
    x = np.asarray(x, dtype=np.float64)
    mu = x.mean(axis=0, keepdims=True)
    sd = x.std(axis=0, keepdims=True) + eps
    return (x - mu) / sd

def pid2_atoms_isx(s1: np.ndarray, s2: np.ndarray, t: np.ndarray, *, k: int = 3, metric: str = "chebyshev") -> dict:
    """
    Non-runnable boundary marker: the stable pid_core_rs 1.0 wheel intentionally omits
    continuous shared-exclusions PID. Run this analysis through pid-offline-harness with the exact
    pid-rs revision/features and support declarations recorded; do not call removed scalar APIs.
    """
    raise NotImplementedError("continuous PID runs through the Rust harness under pid-rs 1.0")
```

**Important:** KSG/ISX estimates are not reliable with very small `n` (e.g., a single 48‑frame clip). For the flow-as-bridge analysis you typically need to aggregate across many clips and/or subsample to reduce autocorrelation (see `grandplan.md` §6.7 uncertainty and dependence). Also keep the **flow target low-dimensional** (avoid raw 3×N×T trajectories); aggregate (centroids/velocities) or reduce, then re-run both the MI/coherence and measure-specific atom validation on the resulting representation.

---

## 9. Experiment 5: Cross-Embodiment (Generalization)
**Protocol role:** S7 transport replication for an already frozen H1 or H2 estimand, plus
exploratory Flow-as-Bridge diagnostics. It does not establish generic “generalization”: name the
source/target policy, embodiment, controller, task-family distribution, overlap assumptions, and
which transforms/recalibration are frozen.

### 9.1 Protocol
Compare PID signatures on the **same task** performed by two different robots (Franka Panda vs. UR5e) with the **same VLA policy** (using cross-embodiment training data or adapters).

### 9.2 Key Comparison
Use Flow-as-Bridge to separate “world understanding” from “actuation/embodiment”:
- **World-model diagnostic (embodiment-agnostic target):** compare CI/PID summaries for decompositions such as `(V, D_vla; Flow_gt)` or `(V, L; Flow_gt)` across robots on matched tasks/scenes. These should be more stable across embodiments if Flow captures task-relevant state changes.
- **Policy/embodiment sensitivity:** compare how Flow relates to actions via `(V, Flow_gt; A_cmd)` and simple MI terms (e.g., `I(D_vla; A_cmd)`) across robots; differences here are consistent with an embodiment gap.

Avoid undefined atoms like `Syn(D; A_robot)`; synergy is a two-source construct.

---
 
## 10. Perturbation Library

**Implementation note (planned):** every perturbation below is an Agent Bridge call (e.g., `intervention.apply`); a GUI merely submits that same call. The bridge appends parameters and simulation/wall-clock timestamps before the perturbation handler runs. No renderer, observer, PID worker, or Zenoh subscriber applies a perturbation directly.

### 10.1 Visual Perturbations
```python
class VisualPerturbations:
    """Logged visual perturbations; SparkJS Dynos are the Phase 4 renderer path"""
    
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

**Exploratory legacy logging only (non-operative for confirmation):** perturbation sweeps may log
per-severity `Syn`, uncertainty, instruction occupancy/entropy, a frozen transform hash, and SSI :=
−IQR(Syn) for descriptive continuity with the retired memorization/generalization analysis. This is
not registry H4, whose endpoint is the availability–use divergence defined in the binding table and
`grandplan.md` §4/§6.3. Legacy-H3 sweeps may likewise log an outcome-independent intervention-strength
yardstick (embedding displacement or MI destroyed), but they do not substitute for current H1/H3.

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
        # Action target (diagnostic; protocol assignment lives in the canonical run log)
        action = pid.create_group("action")
        action.create_dataset("synergy", data=episode.pid_action_synergy)
        action.create_dataset("redundancy", data=episode.pid_action_redundancy)
        action.create_dataset("unique_v", data=episode.pid_action_unique_v)
        action.create_dataset("unique_d", data=episode.pid_action_unique_d)
        
        # Flow target (exploratory Flow-as-Bridge)
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

Example schema (values below are illustrative placeholders; populate from actual run metadata and measured statistics):
```json
{
  "dataset_id": "prisoma_exp1_v1",
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
 
## 12. Compute and Storage Planning

### 12.1 Hardware Envelope (Measure Before Capture)

There is no evidence-backed universal CPU, GPU, RAM, VRAM, or storage minimum for these protocols. The envelope depends on the chosen model/checkpoint, local versus remote inference, capture codec/resolution/rate, retained artifacts, scene complexity, and estimator `(n,d,k)`.

Run a representative pilot and record peak RAM/VRAM, median/p95 latency, thermal/throttling behavior where relevant, bytes per episode, temporary conversion space, and retained size. Choose hardware and storage from those measurements plus a declared margin; illustrative development machines elsewhere in this document are provenance examples, not requirements.

### 12.2 Per-Component Resource Usage
This section is intentionally **measurement-first**. Do not report fixed “ms/frame” or “s/video” values without benchmarking your exact stack.

| Component | What to measure | Notes |
|----------|------------------|-------|
| VLA inference | latency per action (median/p95), peak memory | backend/framework + model variant matter |
| Video generation (if used) | seconds per clip, peak VRAM | log model ID, resolution, frames, steps, seed |
| Segmentation/tracking/depth | time per frame/clip, peak VRAM | log model versions; track failure cases |
| PID-Core (Rust) | time per window vs `(n,d,k)`, memory | run `just exp0-bin` and record timings |
| Physics | step time vs scene complexity, determinism settings | backend-dependent (Rapier/MuJoCo/Isaac) |
| Rendering | fps and end-to-end latency | depends on scene size and GPU |

### 12.3 Latency Budget
**Online vs offline:** If VLA inference dominates latency, treat the control loop as **quasi-static** (pick-and-place style tasks with stable intermediate states). If you add video-generation + flow extraction, assume it is **offline** unless you have demonstrated interactive throughput on your hardware.

For reporting, compute:
`control_hz ≈ 1 / (t_vla + t_phys + t_pid + t_render + t_overhead)`
and record measured `(median, p95)` for each term.

### 12.4 Storage Sizing

Do not use generic per-episode GB figures. Measure each enabled stream on a representative pilot using the exact codec, shape, dtype, rate, compression, and retention policy. For each stream, report:

`bytes_per_episode = measured_bytes / completed_pilot_episodes`

Then size the capture as `sum(bytes_per_episode × planned_episodes)` plus measured temporary conversion/cache space, replicas/backups, and an explicit margin. Recompute after the scientifically adequate nested capture design is selected; capture sizing is currently **NOT READY / NOT PASSED**.

---
 
## 13. Reproducibility Checklist

### 13.1 Random Seeds
```python
import random
import numpy as np

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
    """Set common RNG seeds for reproducibility (where available)."""
    random.seed(seed)
    np.random.seed(seed)

    try:
        import torch
    except ImportError:
        torch = None

    if torch is not None:
        torch.manual_seed(seed)
        if torch.cuda.is_available():
            torch.cuda.manual_seed_all(seed)

    # Physics determinism is engine-dependent; ensure fixed dt/solver params and log them.
```

### 13.2 Version Pinning
- **Python dependencies:** pinned in `uv.lock` (run `uv sync`; commit lockfile changes).
- **Rust dependencies:** pinned in `Cargo.lock` (run `cargo build`; commit lockfile changes).
- **Model code/weights:** record upstream repo URL + commit hash and checkpoint identifiers in the experiment manifest.
- **External stacks (optional):** if you use video generation / segmentation / tracking / depth packages, pin them in a dedicated environment and record their lockfiles/commits.

### 13.3 Experiment Manifest
Every experiment run should produce a manifest file:
```yaml
# results/exp1_run_001/manifest.yaml
experiment:
  id: exp1_run_001
  timestamp: 2026-01-15T10:30:00Z
  duration_hours: 4.5
 
environment:
  hostname: <record-host-or-anonymized-id>
  os: <record-os-and-version>
  hardware: <record-cpu-gpu-ram-vram>
  
software:
  prisoma_commit: abc123def456
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

### 13.4 Agent / Automation Provenance

If any experiment uses the Agent Bridge (scripts or LLM tools) for live interventions or scene edits, require a session manifest (one per run) that makes tool-driven actions auditable:

```json
{
  "session_id": "sess_2026-01-05T10:12:33Z_001",
  "actor_type": "llm_tool",
  "actor_id": "codex-cli",
  "client_version": "unknown",
  "capabilities": ["scene.edit", "intervention.apply", "sim.step", "log.export"],
  "policy": {
    "store_full_prompts": false,
    "store_prompt_hashes": true
  }
}
```

### 13.5 Live Intervention Replay Rule

To keep runs replayable:
- Log every intervention with a **simulation-time coordinate** (e.g., `t_step`, `t_seconds`) and a wall-clock timestamp.
- Prefer checkpointed application (`pause/step → apply → resume`) for interventions that change task difficulty; each transition and the intervention itself is a separate Agent Bridge request appended before dispatch.
- Record whether a run is “strict replayable” (deterministic + fixed ordering) vs “best-effort replayable” (nondeterminism tolerated but logged).

---
 

---

## 14. World Model Comparison: ManiGaussian vs PEGS

**Priority note (`grandplan.md` §5 experimental programme, §4 retired/deferred):** Experiments 6–10 (legacy H_WM1–H_WM5) are optional exploratory world-model extensions and must not block the core programme. ManiGaussian and PEGS are external world-model **comparators/candidates** on the ecosystem evidence ladder (E0/E1, `grandplan.md` §8.9), not integrated or validated dependencies.

**Control-plane rule:** every PEGS visual-correction toggle/force and every matched ManiGaussian intervention in §§14–19 is an Agent Bridge command appended to the canonical run log before execution. PID outputs may be analysis inputs to a preregistered client decision, but PID workers never trigger corrections directly.

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
    mani_action_unique_l: float          # Unq(L; A) - candidate feature (requires validation against failure labels/controls)
    
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

## Appendix A: Planned Commands (Spec Only)

This repo currently ships Rust estimator/Python/run-log/bridge/deterministic-sim/Rerun-adapter groundwork plus the Exp0, toy, and offline embedding harnesses described earlier in this document. The full PID-Splat manipulation environment, custom UI, and Python experiment harness scripts below are still future targets; treat the commands as planned unless/until the referenced binaries/scripts exist.
```bash
# 1. Validate estimators (Experiment 0)
# cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all --bin exp0
 
# 2. Launch PID-Splat environment
# (planned) cargo run -p pid-splat -- --scene scenes/simple_pick_place.yaml
 
# 3. Run Experiment 1
# (planned) python experiments/run_exp1.py --config ...
 
# 4. Analyze results
# (planned) python analysis/analyze_exp1.py --data ...
 
# 5. Generate report
# (planned) python reports/generate_report.py --experiment ...
```

---

## Appendix B: Validation Tools

### B.1 YAML Schema Validator
**Script (planned):** `scripts/validate_yaml_schemas.py`

A comprehensive validator for all scene (`scenes/*.yaml`), object (`objects/*.yaml`), and experiment (`configs/*.yaml`) configuration files.

**Features:**
- **Auto-detection:** Identifies file type (scene, object, deformable, config) based on content.
- **Physics Validation:** Checks mass, friction, and dimensions against realistic bounds.
- **Cross-Validation:** Ensures object IDs referenced in tasks actually exist in the scene.
- **Deformable Checks:** Validates PEGS particle configs (grid size, constraint types).

**Usage:**
```bash
# Validate single file
# (planned) python scripts/validate_yaml_schemas.py --file assets/objects/novel/l_block.yaml

# Validate entire project
# (planned) python scripts/validate_yaml_schemas.py --all

# CI Integration (JSON output)
# (planned) python scripts/validate_yaml_schemas.py --all --json --strict
```
