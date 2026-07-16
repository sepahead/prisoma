# PID-Splat Architecture: Components and Evaluation Boundaries

> **Documentation Cross-Reference**:
> - `grandplan.md` — Master plan and theoretical foundations
> - `pidsplatspecs.md` — Detailed simulation environment specifications
> - `EXPERIMENTS.md` — Experimental protocols for Rerun-first diagnostics, modular physics, and hypothesis testing
> - `DIAGRAMS.md` — Visual architecture diagrams
> - `README.md` — Quick start guide
> - `GAUSS_MI_INTEGRATION.md` — Optional 3DGS uncertainty + view selection (spec)
> - `WORLD_WARP_INTEGRATION.md` — Optional external world‑model baseline (spec)

---

**Docset alignment:** This document is aligned to `grandplan.md` docset v12.5 (scientific cut
2026-07-12). It mixes implemented groundwork and target design, so every section must retain its
status label. The generated capability matrix is the machine-readable status authority; a box,
example, or future-tense interface in this document is not implementation evidence.

**v10.7 → v12.5 migration note:** the legacy H1–H9 / Exp0–Exp10 scheme is retired. The confirmatory registry (`grandplan.md` §4) is now **EC1** (provenance-complete replay) plus **H1** (pre-treatment diagnostics predict intervention response; Protocol A paired vs Protocol B randomized), **H2** (censoring-aware failure prediction), **H3** (conditional PID incremental value), and **H4** (availability can diverge from the effect of one frozen tested intervention). Gates are the **S0–S7** sequence (§5.1); build order is **milestones M0–M7** (§12); estimator validation ("Experiment 0") is now the **S1 gate / §7** and is judged against four gates — population, measure, estimator, application (§7.1). Legacy H-labels below are remapped accordingly.

**Docset-wide final solution:** `grandplan.md` §16 is the decision log (see also §8.2, §8.11, §8.13, §15.4). The run log is the source of truth; the Agent Bridge is the **only control plane**; Rerun is the read-only Phases 1–3 diagnostic/time-machine viewer; and Tauri/SparkJS is the Phase 4 shell for controls, editors, and custom rendering. Every VLA action, scene edit, intervention, pause/resume/step transition, and correction-force command must enter through the Agent Bridge and be appended to the canonical run log before execution. PID workers and observers analyze data; Zenoh transports data; Rerun renders replayed data. None may actuate the simulator.

## 1. Core System Components

### 1.1 Strategy: "Rerun-First" (Phases 1–3)

**What it is:**
The target Phases 1–3 workflow uses **Rerun** as its diagnostic and replay viewer. The repository
currently implements a bounded validating run-log converter/adapter, not the complete viewer
blueprint. The canonical run log—not a Rerun recording—is the authoritative record.

**Why this decision (v10.1):**
- **Reuse boundary:** Rerun supplies timeline, camera, and multimodal visualization primitives. The
  local adapter pins the reviewed `re_sdk`, `re_sdk_types`, and recording-encoding line at exactly
  0.34.1; changing that pin requires converter and finalized-recording compatibility tests.
- **Replay inspection:** converted run-log streams can be inspected and scrubbed while the viewer
  remains read-only. Diagnostic utility and operator time savings still require a task benchmark.
- **3DGS boundary:** point/ellipsoid-like data can be represented through supported archetypes, but
  native 3DGS behavior and fidelity in the exact 0.34.1 stack must be verified before it carries a
  rendering claim.
- **Project focus:** Allows engineering effort to concentrate on estimator gates, physics bindings,
  provenance, and experiment logic rather than an early custom viewer.

**Target visualization pattern: Ghost Splats (not implemented as a complete viewer feature)**

Phase 4 reserves SparkJS for project-specific shader work. The current Rerun adapter has no
custom ghost-splat shader. A target fallback is to log **two** separate entities:

*   **Entities:**
    1.  `world/reality`: The captured scene splats (standard rendering).
    2.  `world/ghost`: A separate, lower-density point cloud or splat-set representing predicted
        flow. A PID-derived color is permitted only for a produced estimate whose four gates allow
        interpretation; abstention is rendered as an explicit status, never as a color-mapped zero.
*   **Evaluation obligation:** measure bytes, log calls, frame time, visual legibility, and operator
    error against the shader alternative; no bandwidth, storage, or simplicity advantage is assumed.

### 1.2 Tauri Application + SparkJS (Phase 4 Target)

**What it does:**
- A specialized, cross-platform desktop application shell (Tauri v2)
- Hosts the SparkJS renderer (Three.js/WebGL2) for custom shader-based visual effects
- Provides a dedicated "Agent Bridge" control panel for human-in-the-loop experiments

**Status:**
- **Deferred to Phase 4.** This is an optional authoring/presentation target for specialized tools
  such as custom shader overlays and recorded intervention editing. It is not contingent evidence
  that the research has been validated and is not required for the core claims.
- **Not a replacement for the run log or Rerun gate workflow.** Tauri should first launch/open Rerun recordings, then optionally embed the Rerun WebViewer, and only later add SparkJS panels for custom shaders or direct manipulation. All edits still go through the Agent Bridge and become run-log events.

**Stack (Phase 4):**
```
┌─────────────────────────────────────────────────────────┐
│                    Tauri v2 Shell                       │
├─────────────────────────────────────────────────────────┤
│  Frontend (React + Three.js)│  Backend (Rust)          │
│  ├─ SparkJS 3DGS Renderer   │  ├─ PID-Core estimators  │
│  ├─ PID Heatmap Shaders     │  ├─ Rerun SDK (optional) │
│  ├─ Control Panel           │  ├─ Agent Bridge (JSON-RPC)     │
│  └─ Timeline + replay UI    │  └─ ML inference hooks   │
└─────────────────────────────────────────────────────────┘
```

### 1.3 Physics Backends (Rapier Implemented; Other Adapters Specified)

**Target role:**
- Provide rigid body physics simulation via a pluggable backend system
- Retain the implemented optional **Rapier3D-f64** object backend and define separately tested
  adapters before naming **MuJoCo** or an **Isaac-family** backend as supported
- Handle collision detection, joint constraints, friction, restitution
- Rapier can run at low step times for small scenes; achievable control/step rates are hardware- and scene-dependent.

**Implemented slice:** `crates/pid-sim/src/physics.rs` defines `PhysicsBackend`, a null
constant-velocity adapter, and an optional single-threaded `rapier3d-f64` adapter with cuboid
bodies, gravity, contacts, friction, impulses, and snapshots. `pid-rapier-harness` exercises a
scripted push-to-goal fixture and emits physics-derived labels and `Flow_gt`. Same-binary/platform
deterministic replay is tested; cross-platform bit identity is not claimed. Mesh-collider ingestion,
robot articulation, MuJoCo, and Isaac adapters are not implemented.

**Why it matters:**
- **Determinism:** Rapier aims for deterministic replay under fixed dt/ordering, but bitwise determinism can break across platforms/CPUs; verify and log settings/seeds.
- **Modularity:** a common trait reduces call-site coupling, but each adapter still needs
  task-specific contact, timing, determinism, and replay validation.
- **Data movement:** the Rust interface avoids a language-process boundary for Rapier, but the
  repository has no zero-copy proof or buffer-identity benchmark. Future foreign backends must log
  serialization/copy boundaries and measured cost.
- **Multi-engine reality**: per-object “Rapier walls + MuJoCo cups” is a co-simulation problem for contact-rich scenes. The recommendation is to use **one physics backend per run**, plus optional **cross-backend replay** (Rapier ↔ MuJoCo) as a robustness/confound check (see `grandplan.md` §8.5 replay levels; §6.10 robustness/falsification).

### 1.4 Gazebo Harmonic (Robot Simulation)

**Target role (planned robot/sensor adapter; none exists in this repository):**
- Robot simulation with the required URDF/SDF subset
- Sensor simulation (RGB-D cameras, joint encoders, force/torque)
- Headless mode for batch experiments

**Integration architecture (Rerun-First):**
```
┌──────────────────┐   command   ┌────────────────┐   append first   ┌───────────────────┐
│ GUI / VLA / tool │────────────▶│  Agent Bridge  │────────────────▶│ Canonical run log │
└──────────────────┘             └───────┬────────┘                 └─────────┬─────────┘
                                         │ dispatch recorded command          │ replay
                                         ▼                                    ▼
                              ┌─────────────────────┐              ┌───────────────────┐
                              │ Gazebo / backend    │              │ Rerun adapter/view │
                              │ robot + sensors     │              │ (read-only)       │
                              └──────────┬──────────┘              └───────────────────┘
                                         └── observations/events ──▶ run log

                     Zenoh, when enabled, mirrors data only; it is not a control path.
```

**Robot vs object simulation (coupling constraints)**

| Component | Use Case | When to Use |
|-----------|----------|-------------|
| **Physics Engine** | Object manipulation physics | Object-object interactions and perturbations within its validated support |
| **Robot Sim** | Robot kinematics/dynamics, sensor simulation | Robot URDF loading, sensor data, cross-embodiment |

**Important coupling rule:** if the robot and manipulated objects are simulated in different engines, robot–object contacts are **not physically meaningful** unless you implement an explicit coupling layer (co-simulation). For most prisoma experiments, prefer one of:
- **Single-engine contact (recommended for manipulation):** simulate robot + objects together in **MuJoCo** (benchmark-aligned) or another single backend, and use PID‑Splat only for logging/overlays.
- **Harness bring-up (recommended for early engineering):** use the in-repo deterministic object sim for run-log/Agent Bridge/Rerun plumbing first, then add object-only Rapier or MuJoCo physics and a kinematic “end-effector proxy” for interventions/perturbations; add full robot dynamics later (see `grandplan.md` §12 milestones).
- **Advanced (optional):** multi-engine “physics islands” with restricted coupling; static colliders can be duplicated, but cross-island contacts require one solver (see `grandplan.md` §8.5).

**Per-claim backend boundary** (see `EXPERIMENTS.md`): the synthetic H1 Protocol-A and H2
fixed-horizon references test protocol arithmetic without establishing any robot or physics
requirement. A real H1/H2 study must freeze the environment needed by its target population and
interventions. H3 is available only after all four PID gates and useful non-PID evidence. H4 needs a
matched internal/input intervention with engagement, specificity, and outcome receipts; no
availability–use asymmetry is currently a finding. Flow extraction may omit a physics simulator only
when its target is explicitly model-predicted flow; `Flow_gt`, executed flow, and physical-validity
claims require simulator ground truth or independently calibrated observations.

### 1.5 Gaussian Splatting (3DGS) Pipeline

**Target pipeline (not implemented or frozen here):**
- Capture a scene/object under a declared camera, rights, calibration, and retention protocol
- Train a pinned Nerfstudio `splatfacto` configuration
- Export Nerfstudio Gaussian splats as `.ply`; a separately reviewed and pinned converter may then
  produce `.spz` for a renderer that requires it

**Pipeline:**
```bash
# 1. Capture (phone/DSLR video; e.g., Polycam; capture protocol is dataset- and scene-dependent)
# 2. Train
ns-train splatfacto --data ./captures/scene/

# 3. Export PLY with Nerfstudio
ns-export gaussian-splat \
    --load-config <config> \
    --output-dir <dir>

# 4. Optional SPZ conversion is a separate tool step.
# Pin the converter executable/revision and record its exact command plus PLY/SPZ hashes.

# 5. Load in Rerun (Phases 1-3) or SparkJS (Phase 4)
```

Nerfstudio's `gaussian-splat` exporter is the PLY-producing step above. Do not pass an invented Gaussian-count training flag or an `--output-format spz` exporter flag. Treat any PLY→SPZ conversion as a distinct dependency with its own version, license, command, and provenance.

**Asset specifications:**

| Object class | Physics proxy example |
|--------------|-----------------------|
| box-like object | Cuboid |
| cylindrical object | Cylinder |
| irregular rigid object | Validated convex hull or mesh |
| tabletop/background scene | Static collision geometry separate from the splats |

Measure and log the actual Gaussian count, renderer memory/time, collision geometry, and export hashes per asset; there is no universal target count.

**Why it matters:**
- **Captured appearance candidate**: splats can preserve some scene appearance, but reconstruction
  artifacts and policy-relevant domain shift must be measured on the frozen capture pipeline
- **Object-centric candidate:** separately reconstructed assets may support composition, but
  segmentation, coordinate registration, occlusion seams, and collision-proxy agreement must pass
- **Differentiability caveat**: 3DGS is differentiable in *training* frameworks; the visualization target here is not a differentiable training primitive.
 
**Caveat:** “Domain gap” and “photorealism” are benchmark-dependent; treat any sim2real claims as empirical until measured.

### 1.6 Dream2Flow-Style Target Pipeline (Specified, Not Integrated)

**Target steps:**
- Use a versioned external video predictor to produce model-conditioned future frames; plausibility
  and physical validity are not assumed
- Apply separately validated segmentation, tracking, depth, and coordinate transforms
- Register a precise predicted-flow variable as a candidate prediction endpoint and, only after all
  four gates pass, as a PID target

> **Flow-as-bridge boundary:** The implemented `EhrlichKsg` path only supports Chebyshev (L∞)
> geometry and has no hyperbolic/Lorentzian derivation. Under the four PID gates
> (`grandplan.md` §7.1), the high-dimensional MI/coherence path is **NO-GO**, while continuous
> shared-exclusions atoms on real embeddings are **BLOCKED / not application-validated**
> (`grandplan.md` §7.2; `findings.md`). Replacing a high-dimensional target with an explicit 3-D
> flow summary can reduce target-side burden, but it does not remove high-dimensional `V`/`D`
> source neighborhoods or joint source-target geometry. The complete tuple still needs recovery,
> support, dependence, concentration/tie, and local-geometry checks.

**Pipeline:**
```
Current Image + Instruction
         │
         ▼
    ┌─────────┐
    │ Video   │ (T frames @ fps; configurable; log frames/fps/seed)
    │  model  │
    └────┬────┘
         │
         ▼
    ┌─────────┐
    │ Segm.   │ (Segment objects in frame 0)
    └────┬────┘
         │
         ▼
  ┌───────────┐
  │ Tracker   │ (Track 2D points through video)
  └─────┬─────┘
         │
         ▼
┌─────────────────┐
│ Depth model     │ (Estimate per-frame depth)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 3D Object Flow  │ (Lift 2D tracks to 3D)
│  Trajectory     │
└─────────────────┘
```

**Why it may be useful (exploratory — `grandplan.md` §9.6):**
- **Potentially transportable target:** object/contact flow can be less tied to an action head, but
  transport still requires matched coordinates, correspondence, visibility, timing, and contact
  semantics across embodiments.
- **Low-dimensional target candidate:** a frozen Euclidean summary can reduce target-side
  dimension; it does not validate the sources or estimator.
- **Prediction comparator:** direct held-out flow error/calibration can test physical prediction.
  PID is a separate, measure-specific information diagnostic and is not a fidelity score.

---

## 2. Simulator and Renderer Selection Boundaries

### 2.1 Simulator Capability Notes (Not a Ranking)

This is a candidate inventory, not a capability or quality ranking. No external row below is an
implemented Prisoma adapter. Do not compare sim-to-real change, frame rate, latency, contact error,
or scale without an exact revision, matched hardware/assets, and a common protocol.

| Candidate | Possible study role | Required evidence before selection |
|---|---|---|
| **MuJoCo / robosuite** | Reproduce a benchmark that fixes this stack | Exact versions, assets, solver/contact validation, renderer/sensor/action compatibility, and license |
| **PyBullet** | Reproduce an existing Bullet-based protocol | Exact version plus task contact, sensor, determinism, and benchmark compatibility |
| **Isaac Sim/Lab or a supported successor** | OpenUSD/PhysX and hardware-accelerated experiments | Supported release, hardware, API migration, deterministic replay, sensor semantics, measured resources, and license |
| **Isaac Gym** | Legacy benchmark reproduction only when the benchmark requires it | Availability/support status, exact binary/API, hardware, determinism, and an adapter report |
| **Gazebo Harmonic** | Robot/sensor middleware candidate | Exact physics/render plugins, URDF/SDF subset, clocks, sensor semantics, and adapter conformance |
| **Habitat** | Navigation-family candidate | Task/action/sensor overlap; manipulation claims require separate support |
| **CARLA** | Driving-family transport candidate | Target-population relevance and vehicle-specific endpoint contract |
| **Rapier3D-f64** | In-repo cuboid object fixture | Implemented optional adapter; still validate contacts, timing, and platform determinism for the selected task |
| **Captured 3DGS** | Appearance layer, not a simulator or collision model | Reconstruction, calibration, renderer compatibility, policy effect, and proxy-geometry agreement |

### 2.2 Adapter Selection Is Claim-Relative

No simulator is assigned a universal quality ranking here. Choose a backend from the requirements
of the frozen claim: contact model, robot/task support, sensor and renderer access, determinism,
licensing, hardware, throughput, and whether the exact interventions can be implemented. A
backend's default assets or renderer may induce a visual-domain shift, but its magnitude and effect
must be measured under the selected policy/task. Navigation-, driving-, manipulation-, and
object-only systems have different support sets; absence of overlap makes a comparison ineligible
rather than evidence that one system is generally inferior.

### 2.3 Target Composition Boundary

```
┌─────────────────────────────────────────────────────────────┐
│                    PID-Splat Architecture                   │
├─────────────────────────────────────────────────────────────┤
│  RENDERING        │  PHYSICS           │  ROBOT SIMULATION  │
│  ────────         │  ───────           │  ────────────────  │
│  Rerun (P1-3) /   │  PhysicsBackend    │  robot adapter     │
│  SparkJS (P4)     │  Rapier: current   │  (not implemented) │
│  (target layers)  │  others: specified │                    │
└─────────────────────────────────────────────────────────────┘
```

**Design choice:** decouple rendering from physics. Captured Gaussian splats are one visual
candidate; a separately validated backend owns dynamics/contact. The decoupling increases
composition flexibility but creates calibration, coordinate-registration, latency, and
cross-backend-consistency obligations.

### 2.4 Modular Physics Backend Selection

Select a physics backend from the frozen claim and measured support:

| Use case | Candidate backend | Selection evidence needed |
|----------|---------------------|-----|
| **Local object-harness iteration** | Rapier3D | In-repo interface support plus measured determinism/contact/throughput on the scene |
| **Contact-rich robot manipulation** | MuJoCo | Task contact validation and exact benchmark/model compatibility |
| **GPU-parallel experiments** | Isaac-family backend | Available supported version, hardware, determinism, and measured throughput |
| **Robot kinematics/sensors** | Gazebo Harmonic | Required URDF/SDF, sensor, timing, and physics-plugin support |
| **Benchmark reproduction** | Benchmark's exact backend | Match revision, assets, solver, sensors, action timing, and evaluation protocol |

There is no repository-wide `pid-splat.toml` backend selector and no checked Franka asset. The
current Rapier runner is selected through Cargo features and binary arguments. A future selector
must reject unavailable adapters, serialize exact backend revision, dt, solver, contact, and
hardware settings into the run log, and bind every referenced asset by hash before dispatch.

### 2.5 Camera & Environment Simulation

| Requirement | Simulator adapter | Rerun (P1–3) |
|---|---|---|
| **Camera views** | Must expose the frozen viewpoints and timing | Replays only the logged geometry/images; it creates no missing observation |
| **Lighting and sensor effects** | Must implement or record the selected condition | Displays the recorded result; it does not re-simulate the sensor |
| **Intrinsics/extrinsics** | Must expose exact calibrated values | Logs and visualizes supplied calibration/provenance |
| **Blur/distortion/noise** | Capability is backend/version specific and must be verified | Displays supplied frames and metadata only |
| **New environments** | Measure asset-authoring and validation cost | Captured reconstruction has its own capture/train/quality-review cost |

---

## 3. Comparison Boundaries for Existing VLM-Based Robotics

### 3.1 Comparison with OpenVLA / PixelVLA / TraceVLA

| Aspect | OpenVLA et al. | PID-Splat |
|--------|----------------|-----------|
| **Simulation** | Model/benchmark-specific mesh or image environments | Candidate captured-splat rendering plus a separately validated physics backend |
| **Visual Fidelity** | Depends on assets, renderer, sensors, and policy | Captured splats may change the visual-domain gap; measure reconstruction and policy effects |
| **Analysis** | Benchmark-defined outcomes and whatever diagnostics the exact release exposes | Proposes gated information diagnostics and intervention-grounded tests; does not by itself reveal why |
| **Physical-state interface** | Model-specific; a hidden state is not assumed to be a world model | Optional flow extraction plus direct prediction/intervention tests |
| **Embodiment Transfer** | Model/benchmark dependent | Tests transport of a standardized flow endpoint after frame/correspondence/visibility checks |

**What PID may add (hypothesis; validate empirically):** typical benchmarks emphasize task success
and sometimes auxiliary diagnostics. A validated PID adds a measure-relative summary of
distributional information. It does not identify causal input use without interventions:
- **Failure signatures:** exploratory PID-atom correlations may become conditional H3 features for
  the prospective H2 endpoint only after all four PID gates; they are not the H1 Protocol A/B endpoint
- **Availability vs tested intervention effect:** combine decodability with a frozen matched
  intervention to test the bounded H4 divergence without inferring natural non-use
- **Long-horizon composition:** test whether temporal PID summaries degrade before failure (exploratory temporal analysis)

### 3.2 Comparison with VLA-Arena

| Aspect | VLA-Arena | PID-Splat |
|--------|----------|-----------|
| **Focus** | Benchmark suite and its reported diagnostics | Additional diagnostic framework with causal claims restricted to interventions |
| **Rendering** | Benchmark-defined simulators | Optional captured 3DGS whose quality and policy relevance must be measured |
| **Metrics** | Exact benchmark-defined outcomes; verify from the selected revision | Proper prediction/intervention outcomes plus conditional information diagnostics |
| **Scalability** | Model/task evaluations require their defined runs | Prisoma still requires model/task/family runs; no single analysis identifies modality contribution |

**Complementary positioning:** VLA-Arena provides standardized evaluation; PID-based analyses aim to add diagnostic signals on top of those evaluations (not replace them).

### 3.3 Comparison with Dream2Flow

| Aspect | Dream2Flow (original) | PID-Splat Dream2Flow |
|--------|----------------------|---------------------|
| **Purpose** | Verify from the exact Dream2Flow release selected for comparison | Candidate predicted-flow endpoint; no integration exists here |
| **Integration** | External system; freeze its exact role and inputs | Specified post-hoc prediction/information pipeline |
| **Diagnostic role** | Action generation | Candidate flow target for gated information and prediction diagnostics |

**Scope difference:** Dream2Flow uses flow in action generation. Prisoma may log a frozen flow
endpoint for direct prediction error and, separately, gated information diagnostics. Neither MI
nor a PID atom establishes physical consistency, a world model, or independence from the action
decoder; those require matched interventions and direct held-out effects.

### 3.4 DreamVLA as a Within-Model Stage-Analysis Candidate

| Stage-analysis question | Candidate variable / intervention |
|-------------------------|-----------------------------------|
| Are exposed world-knowledge channels informative about future motion? | Within the same DreamVLA checkpoint, test preregistered `D_explicit` channels against `Flow_gt` after the estimator gates pass |
| Does the policy causally use an exposed channel? | Ablate, shuffle, or replace that channel within the same model and compare logged action/outcome changes |
| Where does an end-to-end failure arise? | Keep the model, task, preprocessing, and checkpoint fixed while separating world-model, action-decoding, and physical-execution stages |

Do **not** treat a DreamVLA-versus-OpenVLA PID difference as a causal effect of "dreaming": their architectures, action heads, training data, and definitions of `D` differ. Cross-model results are descriptive replication only. The causal design is the within-model stage/channel intervention above.

### 3.5 Attribution Methods as Diagnostic Overlays

LRP, Integrated Gradients, DeepLIFT, Grad-CAM, TCAV, saliency/SmoothGrad, occlusion/permutation, and
SHAP-style attributions are candidate companion diagnostics rather than substitutes for PID.
PID summarizes measure-specific distributional relationships across logged samples; an attribution
method assigns or probes local relevance/sensitivity under its own target and baseline. Neither
branch identifies causal use without an intervention design.

**Rerun-first integration:**
- Log precomputed attribution artifacts as images, heatmaps, token bars, point/patch colors, or scalar time-series tracks alongside PID/CI metrics.
- Keep attribution metadata with the artifact: method, target output, layer/modality,
  baseline/background/concept set, preprocessing, score hash, typed ranking-sensitivity or sanity
  result, and limitations. The run-log field named `faithfulness_check` is a legacy compatibility
  boolean, not a broad faithfulness claim.
- Do not require Phase 4 custom shaders for attribution review; Phase 4 can add interactive overlays, but the canonical evidence remains the run log plus artifacts.

**Implemented slice:** `experiments/attribution/` runs a detached-attention, value-path-only
epsilon-LRP baseline that is explicitly not AttnLRP, plus gradient×input, on a small reference
model. It applies a content-bound deletion-ranking-sensitivity gate on a selection-disjoint,
group-disjoint validation set with declared predictor-determinism provenance and runtime
determinism checks. Every exact attribution-magnitude tie causes a typed abstention. For an
identified ranking, the gate compares mean absolute deletion sensitivity with a bounded per-case
random-ranking reference and aggregates independent-group wins with a conservative one-sided
binomial tail. This diagnostic asks whether the feature ordering finds output-sensitive baseline
replacements sooner; it does not establish causal or mechanistic faithfulness. Exactly one
predeclared primary method can set the legacy run-log boolean. The producer preflights the complete
method-plus-gate work, then emits first-class `attribution_logged` events and companion
`artifact_logged` events for immutable content-addressed relevance artifacts and canonical,
reconstructable JSON evidence bundles. Those bundles bind exact model parameters, the complete
gate and case set, every input/baseline/relevance array, decision evidence, software versions, and
source hashes. The `pid-rerun` adapter surfaces a recorded compatibility check and provenance
text—not a validated-faithfulness verdict. Its standalone converter can additionally surface at
most 1024 finite values from a narrowly framed, bounded NumPy relevance artifact only after the
operator passes
`--load-attribution-artifacts`; paths are confined to the run-log directory, and the recorded exact
file SHA-256 and canonical shape must match before output. Bridge export never grants this
file-reading capability. The path checks are local best-effort confinement, not protection against
every concurrent filesystem race, and artifact/run-log publication is not a cross-file
transaction. Production VLA adapters, production-model validation, and richer 2-D panels remain
future work.

**Interpretation rule:** PID atoms and attribution scores do not share an estimand by default, so
agreement cannot validate either one and disagreement is not a contradiction by itself. A
triangulation study must freeze a common output, sampling unit, baseline, intervention, and effect
contrast; report both branches, their eligibility/status, and direct counterfactual effects
separately.

---

## 3A. Representation Diagnostics: Identification Boundary

### 3A.1 Observable Diagnostic Gaps

At the interface level, many VLA policies can be summarized as:
```
Image + Instruction → LLM → Action Tokens
```

**Questions requiring evidence:**
1. **Representation semantics:** a hidden state is not a physical state or world model merely
   because a probe can decode a physical variable from it.
2. **Feasibility:** physical feasibility and scene alignment require direct outcome and constraint
   measurements, not confidence or information alone.
3. **Integration:** observational associations among V, L, D, and A do not identify which channel
   the policy causally uses; matched interventions are required.
4. **Transport:** camera, timing, action-head, task, and embodiment changes can all alter the
   estimand and must be separated.

### 3A.2 What the Proposed Diagnostics Can Test

Subject to the proposed gates—and to a future immutable study freeze—the architecture can support
the following tests:

**1. Testing representational availability:**
- `D` is a declared hidden-state hook, not an assumed internal simulation.
- A train-only capacity-matched probe and held-out proper prediction score can provide bounded
  evidence that a physical quantity is decodable on the tested population. A randomized or
  otherwise identified matched intervention is needed to estimate whether the policy uses it.
- `PID(V,D;Flow)` is only a measure-specific association after all four gates pass. It does not
  test physical validity.

**2. Diagnosing integration quality:**
```
I(V,L;A) = Red(V,L;A) + Unq(V) + Unq(L) + Syn(V,L;A)
```
- **Positive `Syn`**: a numerical atom of the frozen PID functional. Its mechanism meaning is not
  identified by sign or magnitude and remains measure-, source-, target-, and population-relative.
- **Negative `Syn`**: allowed under `I^sx_∩`; treat it as a signed candidate diagnostic only after
  the four separate gates, oracle controls, and uncertainty checks.
- **Large `Unq(L)`**: an observational atom under the chosen measure, not evidence of causal language
  reliance. Estimate reliance with randomized instruction interventions, engagement checks, and
  placebo/positive controls.

**3. Descriptive transport evaluation:**
- Compare a standardized object/contact-flow endpoint only after coordinate, visibility,
  correspondence, timing, task, and support overlap are established.
- Raw atoms are not directly comparable across representations or action decoders. Cross-robot
  results remain descriptive unless a causal transport design identifies the changed component.

**4. Longitudinal diagnostics:**
- Test prespecified episode-level or hierarchical summaries over long horizons without treating
  overlapping windows as independent.
- A temporal change in an atom is exploratory and may reflect occupancy, estimator drift, or
  representation change; it does not establish loss of world-model coherence.

### 3A.3 Why Gaussian Splats + Modular Physics Enable This

| Design axis | Evaluation requirement |
|-----------------|-----------|
| Captured splats vs synthetic renders | Measure reconstruction quality and policy-relevant domain shift; neither is assumed superior |
| Rapier vs MuJoCo vs another backend | Match task/contact support and benchmark accuracy, determinism, and cost on the exact scene |
| Offline vs live diagnostics | Use the same canonical run log; measure latency/throughput before making operational claims |

The load-bearing requirement is reproducible instrumentation: fixed preprocessing, declared
support, validated estimators, controlled interventions, independent inference units, and a
canonical run log. Gaussian splats, modular physics, and Rerun are candidate implementation
choices whose reconstruction quality, contact validity, throughput, latency, and transport must
be benchmarked before they support a scientific or operational claim.

---

## 4. Component Summary

| Component | Role | Rationale (design goals; benchmark-dependent) |
|-----------|------|--------------|
| **M0 governance ledgers** | Analysis-freeze scaffolding | Preserve the non-promotable historical v1 scaffold and machine-check an all-null typed v2 successor draft covering EC1/H1/H2/H3/H4 freeze obligations, including exact EC1 acceptance coverage with a mandatory pair-specific absolute detection-sensitivity floor for every registered fault–adapter pair and no aggregate rescue, plus H2's one-primary-score success hierarchy, alongside the no-registered-holdout state, pending transport/contamination work, and legacy-only literature inventory. Passing either local audit is integrity evidence, not scientific readiness. |
| **Run log** | Canonical data spine | Source of truth for replay, analysis, Rerun export, and Tauri sessions; summaries distinguish unique metric names from total metric events. |
| **Agent Bridge** | Only control plane | GUI, scripts, LLM tools, and VLA-policy adapters submit every mutating command through the same local API; the command is recorded in the run log before execution. |
| **Rerun** | **Read-only visualization & diagnostics** | **Primary P1-3 Tool.** Timeline, 3D scene, plots, ghost overlays, and replay from run logs; it never drives the simulator. |
| **Tauri+SparkJS** | Interactive App | **Deferred to P4.** For custom shaders, collider/edit tools, and complex intervention UI; never the canonical store. |
| **Physics** | Object physics | Modular (Rapier/MuJoCo/Isaac) |
| **Robot Sim** | Robot dynamics | Industry-standard (Gazebo/MuJoCo) |
| **3DGS Pipeline** | Scene capture | Captured-view reconstruction candidate; quality and policy relevance are empirical |
| **Dream2Flow** | Prediction/flow candidate | Frozen Euclidean flow endpoint with explicit transport assumptions |
| **PID-Core** | Read-only information analysis | Computes candidate diagnostics from logged/captured data; it never triggers actions, pauses, or corrections |
| **Attribution probes** | Local explanation baselines | Reference detached-attention value-path epsilon-LRP (not AttnLRP) + gradient×input probe, content-bound ranking-sensitivity gate/evidence bundles, and recorded-check Rerun adapter are implemented; other methods/production-VLA hooks remain extensions |

Current deterministic bridge smokes expose stdio/TCP/WebSocket JSON-RPC methods for status,
deterministic stepping, deterministic interventions, replay, run lifecycle stop, and
`export.rerun`. TCP/WebSocket binaries refuse non-loopback bind addresses and default to safe mode;
leaving it requires `--allow-mutations`. This does not stop forwarding, proxying, or tunnelling a
loopback listener. TCP/stdio JSONL lines are capped at 1 MiB; WebSocket upgrades and incoming
client frames are capped at 16 KiB and 1 MiB respectively; network reads and writes time out after
30 seconds per operation. There is no total request/session deadline, request-count cap, or
aggregate-traffic budget, and progress-making trickle traffic can persist.

The WebSocket upgrade check is deliberately narrow: `GET /bridge HTTP/1.1`; exactly one each of a
nonempty `Host`, `Upgrade: websocket`, tokenized `Connection` containing `upgrade`, version `13`,
and base64 key decoding to 16 bytes; and no `Origin`. It is not a general malformed-HTTP detector.
Client application messages are unfragmented, masked UTF-8 text frames; ping, pong, and close are
supported, while binary frames, fragmentation, and extensions/RSV use are rejected. The bridge
implements a single-request JSON-RPC 2.0 subset, not batches. Missing-id notifications are silent
and distinct from explicit `null`; parameters are omitted or named objects, not positional arrays;
undeclared top-level method keys are rejected; and `sim.step` requires numeric `dt`.
Profile-invalid parameters map to `-32602`; handler/domain failures after validation map to
`-32000`.

Replay/export paths have non-adversarial canonical confinement: traversal, observed symlink
components, non-regular/out-of-root reads, missing parents, and existing outputs are rejected.
Run logs and Rerun outputs are created no-replace. Export parses and manifests the same exact byte
snapshot read from the source, finalizes and hashes the RRD bytes, then stages, syncs, and installs
them no-clobber. This is not a security-grade filesystem sandbox against hardlinks, aliases, or
concurrent mutation. Executable transport run logs use `File::sync_all` for the initial prefix,
each session flush before a wire response, and the terminal seal; generic
`SimBridgeSession<W>` durability remains sink-defined. There is no parent-directory fsync,
power-loss claim, or cross-file transaction joining a run log to its export. Ordinary
accepted-client failures are sealed `Failed` only while provenance storage remains writable; a
crash or storage failure may leave incomplete/unreadable provenance, an apparently complete
terminal record with indeterminate status/durability, or an orphan RRD. This is local E0
hardening, with no
authentication, authorization, TLS, redaction, or remote-security assessment, and not completion
of the full M2 acceptance contract (all target UI/VLA/backend controls plus a versioned
subscription stream). Likewise, the validating run-log-to-Rerun converter is **partial M2/EC1
viewer groundwork**; the complete blueprint/viewer remains specified, not built.

---

## 5. Research Trajectory

| Phase | Goal | Key Deliverable | Visualization |
|-------|------|-----------------|---------------|
| **1** | Validate estimators (S1 gate / `grandplan.md` §7) | Four PID gates (population/measure/estimator/application, §7.1); current status is MI/coherence **NO-GO** on the high-d sweep and continuous shared-exclusions atoms on real embeddings **BLOCKED / not application-validated** | Rerun (Charts) |
| **2** | Apply to OpenVLA on LIBERO | Failure signature taxonomy | Rerun (Timeline + Logs) |
| **3** | Within-model stage/channel ablations | World-model-stage vs action-decoding vs execution diagnostics under fixed model/checkpoint/task | Rerun (3DGS + Ghost Splats) |
| **4** | Embodiment transfer via Flow-as-bridge | Cross-robot PID analysis | **Tauri + SparkJS** (Interactive) |

**Ultimate goal:** move from aggregate task outcomes toward intervention-grounded, reproducible
diagnostics of when a policy's logged representations and actions change. Claims about
“understanding” remain operational and must be tied to prediction and counterfactual effects.

---

## 6. Hardware and Storage Planning (No Universal Minimum)

The repository has no evidence-backed RAM, VRAM, disk, or device minimum for the target stack. Requirements depend on the selected VLA/video models, capture codec and retention policy, scene size, estimator regime, and whether inference is local or remote.

Before capture, benchmark the exact configuration and record peak RAM/VRAM, median/p95 latency, bytes per episode, temporary conversion space, and retained-artifact size. Size hardware and storage from those measurements plus an explicit safety margin; do not reuse illustrative machine specifications as requirements. See `EXPERIMENTS.md` §12.


---

## 7. Retired World-Model Head-to-Head Sketch

The former ManiGaussian-versus-PEGS comparison is retired and non-operative. It assigned
directional generalization/deformable-object claims without committed evidence, compared systems
with unequal capability support, and used MI/PID atoms as “fidelity” scores even though information
is not prediction error, calibration, or causal mechanism. The corresponding H_WM1–H_WM5 sketches
were removed from `EXPERIMENTS.md` §14.

A successor study must first show overlap/positivity: both frozen systems must support the same
objects, interventions, observations, horizons, and outcomes. It must compare proper prediction
errors or scores on an episode/family-disjoint test set, fit every transform on training data,
match information access and tuning/compute budgets, retain failures, quantify clustered
uncertainty, and isolate perturbation factors. PID may be an additional measure-specific
diagnostic only after all four gates pass; it cannot define prediction fidelity or establish that
a latent is a world model. No such comparison is implemented or preregistered in this repository.

---

## 8. SmolVLA (LeRobot) Integration

SmolVLA (LeRobot) is a candidate lightweight baseline (planned integration; verify model availability/APIs).

### 8.1 Architecture
- **Backbone:** Lightweight VLM baseline (LeRobot; verify exact architecture/backbone).
- **Action head / representation:** Implementation-specific; verify (continuous delta actions vs discretized tokens/bins).
- **Inference:** May support async pipelines (verify and measure on your stack).

### 8.2 Architectural Role in prisoma
- **Iteration Speed:** Smaller models can make the PID pipeline easier to iterate on (measure inference latency on your hardware).
- **Control Rate:** Async inference can raise effective control rates (benchmark; depends on policy and environment).
- **Fine-tuning:** Possible LeRobot dataset integration; verify exact dataset, checkpoint, action,
  preprocessing, and license interfaces before selecting it.

---

## 9. InternVLA‑A1 (Optional) Integration

InternVLA‑A1 is a candidate **diffusion / flow-matching** VLA for stage-wise ablations because it explicitly separates “understanding”, “generation”, and “action” experts (verify details and interfaces from its paper/repo before use).

### 9.1 Architectural Role in prisoma (Docset v10.1)
- **Hierarchical PID inside one model:** treat generation-expert outputs as `D_gen` (a candidate `D_explicit`) and test `(V,L;D_gen)` and `(V,D_gen;A)` under the same data/logging contract as other VLAs.
- **Flow comparisons:** if `D_gen` yields predicted frames/latents, derive a model-side `Flow_pred` and compare to simulator-derived `Flow_gt` under matched controls (do not conflate “Flow Matching” used to generate actions with this project’s geometric `Flow_*` variables).
- **License caution:** verify the upstream license/model card before depending on it (may be restrictive); avoid vendoring code into this dual-licensed **MIT OR Apache-2.0** repo unless license compatibility is confirmed.

### 9.2 Integration Notes (Verify)
- The repo describes patched HuggingFace Transformers modules; isolate integration in a separate service/environment and log the exact revision.
- Confirm how to export intermediates (`D_gen`) and the exact action parameterization (“delta actions”, etc.) before quantitative comparisons.
