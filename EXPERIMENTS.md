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
 
**Version:** docset v12.5
**Review date:** 2026-07-15
**Context:** This document specifies *task suites, data collection, and evaluation protocols* used to test the confirmatory claims in `grandplan.md`. `grandplan.md` defines estimator/measure validation and the analysis logic; this file focuses on what to run and log. The deterministic Agent Bridge/Rapier/Rerun/attribution slices are implemented groundwork, but the core+ecosystem conformance benchmark (M2) and the locked H1 experiment (M4) remain open; external video predictors and the fuller PID‑Splat environment remain specifications until built.

**Docset-wide final solution:** `grandplan.md` §16 is the decision log. Experimental evidence must flow through the canonical run log; the Agent Bridge is the only control plane; Rerun is a read-only diagnostic viewer; and Tauri/SparkJS is deferred until the run-log/replay/Rerun loop is reliable. Every VLA action, intervention, scene edit, pause/resume/step transition, and correction force must be recorded as an Agent Bridge command before execution. PID, observers, Zenoh, Rerun, and offline harnesses do not actuate the system.

> **Docset v12.5 migration note (read first).** This document retains the legacy `Exp0–Exp5` task
> labels and a historical `H1–H9` mapping. Read every legacy label through the
> v12.5 confirmatory registry (`grandplan.md` §4) and the S0–S7 gate sequence (`grandplan.md` §5.1):
> - **Exp0 estimator validation → the S1 estimator/measure-validation gate** (`grandplan.md` §7; the `exp0` binary implements part of §7).
> - **Exp1–Exp5 task suites → the §5 experimental programme**, analysed under the §6 statistical
>   analysis plan. The former **Exp6–Exp10 world-model sketches are retired and audited in §14**;
>   any successor is a new reviewed exploratory protocol, not a continuation of those hypotheses.
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

*(A non-operative audit of the retired Exp6–Exp10 world-model sketches follows in §14; the
appendices follow it. No retired sketch is an experiment specification or a reason to delay the
active programme.)*

---

## 0. Relationship to `grandplan.md`

- `grandplan.md` is the scientific contract: definitions, estimator/measure assumptions, identification and confound controls, and the S0–S7 gate sequence (`grandplan.md` §5.1) with the four PID gates (`grandplan.md` §7.1) — especially the S1 estimator/measure-validation gate.
- This document specifies *what to run and what to log* so those gates and confirmatory claims can actually be tested.
- The milestone build order and gate sequence in `grandplan.md` §12 (M0–M7) and §5.1 are binding for experiment tooling: do not replace run logs with GUI-only state, do not treat live transport as required, and do not introduce predictor-driven `Flow_pred` before simulator-derived `Flow_gt` is replayable.
- **Mapping (high-level):**
  - The legacy *Experiment 0* estimator/geometry gate ↔ this document §4 (synthetic validation + geometry checks), which implements part of the S1 gate (`grandplan.md` §7).
  - The legacy *Experiments 1–5* task suites ↔ datasets generated by this document’s Experiments 1–5, feeding the `grandplan.md` §5 experimental programme and analysed under the §6 statistical analysis plan.

The machine-readable M0 bundle under `protocols/` is deliberately **not freeze-ready**. It keeps
H1-A, H1-B, H2, H3, and H4 branches separate while recording that the primary H1 protocol remains
unselected and unfrozen, with no registered confirmatory holdout, no selected source/target dataset
pair, and only a legacy dated reference inventory without reproducible query/candidate-decision
provenance. Run `just research-governance` for structural integrity; do not open a holdout or start
evidentiary capture until a reviewed successor schema's freeze-ready audit genuinely passes.

### 0.1 Hypothesis Coverage Matrix

The following table is the **binding v12.5 registry**. A task suite is not itself a claim: its run
configuration must choose exactly one confirmatory protocol and use that protocol’s unit, treatment,
outcome, score, and language.

| Current claim / protocol | Candidate task suites | Primary evidence | Required controls and current blocker |
|---|---|---|---|
| **EC1 — provenance-complete replay** | Exp1 plus a structurally different adapter/environment | Contract-violation detection and exact/tolerance-bounded replay versus a conventional script/container baseline | Typed assignment/receipt/outcome lineage, fault injection, standard-format adapter, external benchmark; still open (`grandplan.md` §8.8) |
| **H1 Protocol A — paired frozen-snapshot algorithmic response** | Exp1 baseline cases; Exp3 manipulation constructions | Held-out direct prediction of the declared paired response functional, with calibration, response reliability/Monte Carlo error, and a locked feature-vs-baseline contrast. This is an algorithmic-sensitivity diagnostic, not a randomized or physical effect | Immutable clone state, pre-treatment moderator, instrumented/noninstrumented noninterference, declared random-number coupling, draw ledger, reverse-order/process controls; blocked on real capture and stochastic clone machinery (`grandplan.md` §6.3) |
| **H1 Protocol B — randomized closed-loop effect moderation** | Exp1/Exp3 randomized episodes | Overall ITT first; then held-out effect-specific loss, causal calibration, prioritization, and policy value/regret under recorded assignment probabilities | Randomization/receipt/reset/censoring ledger, cluster-aware inference, synthetic oracle and negative controls; blocked on capture and assignment runner (`grandplan.md` §6.3) |
| **H2 — prospective censoring-aware failure prediction** | Prespecified landmarks in Exp1/Exp2; later Exp5 transport | Held-out log loss or censoring-aware Brier score at the frozen horizon, plus calibration, event sensitivity at fixed false-alarm burden, nondetection-retaining lead time, and decision utility | A deterministic synthetic fixed-horizon/IPCW/alarm arithmetic reference is fixture-runnable (`just h2-reference`); the domain freeze, real prospective capture, full matched-access comparator frontier, and external/later-time validation remain blocked (`grandplan.md` §6.4) |
| **H3 — conditional incremental PID value** | Any H1/H2 dataset only after all four gates | Nested out-of-sample improvement from adding preregistered PID features to the strongest valid non-PID model under the active H1/H2 endpoint | Population/measure/estimator/application gates, train-reference local construction, abstention denominator; application gate currently blocked (`grandplan.md` §7) |
| **H4 — availability–use divergence** | Exp3 input/internal intervention pairs; Exp1 positive/negative controls | Prespecified discordance between held-out decodability and policy/execution ITT effects, conditional on engagement and support | At least two intervention constructions where feasible, positive/negative controls, equivalence margins; blocked on capture/intervention pilot (`grandplan.md` §6.3) |

**H1 estimand boundary.** Protocol A and Protocol B answer different questions and cannot be
combined into one endpoint. If exact policy-output distributions are available, Protocol A uses a
frozen divergence `S_i = d(Pi_i^(1), Pi_i^(0))`. If only samples are available, its estimand is
`S_i(C) = E_C[d(Atilde_i^(1), Atilde_i^(0)) | W_i]` under a prespecified coupling `C` of the
two policy random streams. Changing `C` can change the target,
not merely its variance; record it as part of the estimand, never pool estimates across couplings,
and include an independent-stream sensitivity analysis when feasible. A Protocol A result supports
only frozen-snapshot algorithmic-sensitivity language. Any claim about randomized closed-loop,
execution, or physical-outcome effect moderation requires Protocol B, with ITT and inference at the
randomization/interference unit.

The dependence on `C` is not cosmetic. If both treatment outputs are marginally
`Bernoulli(1/2)` and `d` is mismatch, a common coupling gives expected mismatch `0`, independent
streams give `1/2`, and an antithetic coupling gives `1`. The marginals are identical in all three
cases; the paired response is not.

**H4 identification boundary.** Availability is held-out decodability, not the best score obtained
after inspecting the evaluation set. Select layers, transforms, regularization, and probe capacity
inside grouped outer-training data; use a second untouched test partition, keep persistent cases in
one fold, report balanced/proper metrics and uncertainty, and capacity-match comparators. A transfer
claim needs a separately untouched target policy/task family. Use is a prespecified paired
frozen-snapshot algorithmic response or randomized execution effect, with engagement and specificity
controls and without blending their scopes.
Compare the two against useful equivalence/discordance margins; a significant decodability result
beside a nonsignificant intervention result is not evidence of divergence.

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
| **H9** Attribution probes triangulate PID claims | Exp1, Exp3, Exp4 | LRP/IG/DeepLIFT/Grad-CAM/TCAV/saliency/occlusion/SHAP-style scores on the same logged samples | Separately grounded relationships to held-out actions, paired algorithmic responses, or randomized effects; attribution and PID do not share an estimand or numerical scale, so their ordering agreement cannot validate either method | Model/data randomization sanity checks, baseline/background sensitivity, deletion/occlusion tests, attention-not-explanation caveat |

### 0.2 Runbook: What Is Executable Today vs Blocked (docset v12.5, 2026-07-13)

This table is the self-sufficient entry point: it maps the run order onto current tooling, expected outcomes, and blockers. Commands assume `just` (each recipe wraps plain `cargo` commands listed in `README.md`/`AGENTS.md`).

| Step | What | How (today) | Gate / expected outcome | Canonical reference |
|---|---|---|---|---|
| 0 | M0 analysis-governance integrity | `just research-governance`; separately run `python scripts/audit_research_governance.py --require-freeze-ready` only as a closed-gate check | The base audit passes the honest unfinished scaffold; freeze-ready mode must fail until protocol/domain choices, estimands, endpoints, multiplicity, useful margins, holdout commitments, transport/contamination/rights evidence, comparator dispositions, and fresh search provenance are frozen. It checks completeness/integrity, not scientific correctness. Evidentiary capture remains blocked. | `grandplan.md` §12 M0; `protocols/README.md` |
| 1 | Toolchain + estimator diagnostics (legacy Exp0 → S1 gate) | `cargo test`; `just exp0` / `just exp0-bin` / `just exp0-runlog`; opt-in uncertainty: `just exp0-uncertainty` (`--bootstrap`/`--permutation`). `--strict-gate` is a direct CLI opt-in that implies `--strict-band` and exits 3 unless the curated d=1 Gaussian **MI** grid is GO; it does not gate the default high-d sweep or continuous atoms | Split status: default high-d MI/coherence = **NO-GO**; continuous `I^sx_∩` atoms on real embeddings = **BLOCKED / NOT APPLICATION-VALIDATED** (the `pid-rs` pin does carry low-d additive-Gaussian oracle + discrete-SxPID reference evidence). The curated strict band and favourable-dimension UQ diagnostics do not override either status | `grandplan.md` §7.2, §7.5; `findings.md` |
| 2 | Run-log spine + replay + bridge smokes | `just runlog-demo`, `runlog-validate`, `runlog-replay`, `runlog-bridge-*`, `runlog-sim-verify`, `runlog-rerun` | `valid=true`, `errors=0`; deterministic replay; simulator-derived `Flow_gt` verified | `grandplan.md` §8.2, §8.5 |
| 3 | Labeled toy pipeline end-to-end | `just toy-harness` | Canonical labeled artifacts validate; not VLA evidence | `grandplan.md` §12 (milestone rehearsal) |
| 4 | Offline `(V,L,D,A)` harness, all three PID modes | `just offline-harness`, `offline-harness-require-*`, `offline-harness-strict`, `offline-harness-discrete`, `offline-harness-discrete-pls` | Strict mode exits nonzero on its implemented legacy geometry aggregate (software smoke only; not corrected scientific eligibility); discrete modes report `saturation_warning=true` on the tiny fixtures (by design — the `grandplan.md` §7.6 discrete PID gate) | `grandplan.md` §7.6, §6.2 |
| 5 | **First real VLA/task capture (downstream critical path; blocked on M0 freeze)** | `just safe-adapter` maps a bounded, content-addressed synthetic SAFE-format NPZ/JSON bundle to the harness `(V,L,D,A)`+labels contract with the `grandplan.md` §9.2 hook-probe; downloaded pickle is rejected by default. `just rapier-harness` exercises a real Rapier3D task with labels + `Flow_gt`. Still required: safe re-export and exact manifest of real data, source/split/rights review, model/task/hook selection, the prospective episode-local H1 feature path, and a scientifically adequate capture-sizing gate | Strict harness modes and adversarial ingress checks have been exercised on synthetic/local fixtures only. The implemented `pid-sim-power-gate` is an idealized endpoint-level sensitivity simulator; it omits the required family → task/case → episode nesting, PID/SSI measurement error, binary outcomes, severity allocation, and selected-design type-I error. Its first-run counts are withdrawn as capture requirements. Capture sizing is **NOT READY / NOT PASSED** | `grandplan.md` §9.1, §6.8 |
| 6 | Exp1–Exp5 protocols (§5–§9 of this document) | `just h1-preflight` exercises the schema-v2 representative-mechanism structural/noninterference contract; `just h1-protocol-a` exact-binds that pass and runs a deterministic synthetic finite-benchmark clone/response and fixed out-of-fold scoring primitive. `just h2-reference` exact-binds separate plan/ontology/feature/split artifacts and runs deterministic complete/censored fixed-horizon cumulative-incidence, grouped fitting, IPCW Brier, reliability, alarm, nondetection, and declared-utility arithmetic. All exercise readable failure paths and zero PID events. Real Protocol A capture/analysis, subprocess/stochastic controls, Protocol B assignment/effect scoring, real prospective H2 capture/comparators/external validation, and applicable estimator/measure gates remain blocked | Every checked number is a software-reference output only. H1 binds `synthetic_fixture_only=true`, `establishes_h1_evidence=false`; H2 additionally binds `establishes_h2_evidence=false`, `prospective_capture=false`, `external_validation=false`, and `comparator_frontier_complete=false`. The outputs establish neither scientific claim, physical effect, calibration validity, warning benefit, nor closed-loop robustness. Per-claim metrics + controls remain those in the binding §0.1 table; kill rules are in `grandplan.md` §3.8 and statistics in §6. No capture-size claim may be made from the current idealized simulator | `grandplan.md` §5, §6.3–§6.4 |

**Binding power status:** `cargo run --release -p pid-sim --bin pid-sim-power-gate` is implemented and useful as an idealized endpoint-sensitivity simulator. It is **not capture-ready**, and its 2026-07-10 first-run task/case/episode counts are withdrawn—not lower bounds, recommendations, or requirements. A replacement capture gate must simulate the nested family/task-or-case/episode design, PID/SSI measurement error, binary outcomes, severity allocation, and type-I error under the selected analysis, after the prospective H1 feature path exists.

Three discipline rules apply at every step: (a) each (PID measure, preprocessing, estimator config) tuple must be frozen as a distinct preregistered regime — continuous `I^sx_∩` and discrete Williams–Beer `I_min` results must never be pooled (`grandplan.md` §7.6); (b) non-PID baselines and uncertainty (block bootstrap, permutation nulls) accompany every PID number; (c) before activation, each confirmatory claim must freeze exactly one primary endpoint with a predicted direction plus its multiplicity procedure — all other endpoints remain exploratory, and the active estimator regime must be selected ex ante by the separate S1 estimator/measure gate (`grandplan.md` §6.6, §7). Those choices are not yet frozen in the checked M0 scaffold.

### 0.2.1 Data sources (the harness is source-agnostic)

Everything downstream consumes one `(V,L,D,A)`+labels contract (the `OfflineVldaDataset` JSON the offline harness reads), so capture sources are pluggable. In `(V,L,D,A)`, **D is the Dynamics / hidden-state axis, not depth** — defined per model (`grandplan.md` §9.1, §9.2).

| Source | Role | Standalone? | Status |
|---|---|---|---|
| `experiments/safe_adapter/` (released SAFE rollouts) | **Critical path** (S2/EC1 reference adapter, `grandplan.md` §8.7) | yes | Bounded NPZ/strict-JSON ingress, exact file hashes, operator-declared source/split/rights and model/hook/tensor receipts, converter, and hook-probe are local-fixture-validated; legacy pickle is default-off; real safe export/capture, rights review, and scientific gates remain open |
| `crates/pid-sim` fixtures + `pid-rapier-harness` / `pid-toy-harness` | Sim cross-checks | yes | Runnable software/physics smokes with physics-derived labels + `Flow_gt`; not VLA evidence |
| `crates/ncp-observer` (Engram/NEST over the Neuro-Cybernetic Protocol) | **Optional** external bridge (M2 ecosystem conformance, `grandplan.md` §8.9.5) | fixture observatory only | **Exploratory-only, read-only — below the S2/EC1 bar** |

The pure-PID stack (the table above minus NCP) builds and its software smokes run with **no NCP/Engram/Zenoh dependency** — `ncp-observer` is excluded from the default cargo workspace. That does not imply that the scientific estimator/capture gates pass. NCP is a read-only exploratory tap, not a controller and not part of grandplan's critical path. The observer performs full wire-0.8 `{epoch,seq}` joins, never uses recency fallback, preserves exact/conflicting receipt semantics across restarts, and commits the artifact/canonical-log pair with a harness-verified publication receipt. The deterministic fault observatory now supplies local E3-style evidence for a finite, hand-authored fixture only when its build/runtime revisions agree, both worktree states are clean, and its lockfile plus exact executable hashes are recorded; otherwise its typed evidence level is reproducibility-unqualified. This is a local reproducibility binding, not signing or remote attestation. Its 18 frozen omission/duplicate/reorder/mutation/truncation/declared-profile-label scenarios run twice through the shared route/raw-ingress seams, with strict per-replay outcome records and injection truth kept separate from native detection in a receipt-last report. The frozen inventory is 16 assessed cases (15 matched, one matched known limitation for whole-tick omission), two expected `not_assessable` guards (logical pause and security-profile claim), and zero mismatches; `all_expectations_matched=true` is not an 18/18 detection-rate claim. Its `capture_integrity` remains only a visible-receipt/join grade; the whole-tick omission case is explicitly a manifest-only blind spot. Logical slots are annotations that do not drive or measure timing, trace truncation is not a live disconnect, and the declared-profile case neither loads nor selects a security configuration. No receipt timing/QoS/reconnect, authentication/ACL, live noninterference, E4, EC1, live Engram, security, or PID-validity conclusion follows. NCP artifacts declare no inferred population support, so use `--pid-mode none` by default: continuous KSG/shared-exclusions requests abstain, while quantized discrete `I_min` is only a non-evidentiary diagnostic with population `NotEvaluated` and application `Blocked`. It remains below the S2/EC1 contract until a conforming live publisher plus honest `L`, `metadata.split`, `episode_id`, and `success` structure exist for the strict harness checks and the `grandplan.md` §4 H1 audit. See `NCP_DEV_PROMPT.md`.

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

**Safe-mode rule:** read-only Agent Bridge sessions allow status/replay queries while logging and rejecting mutating/file-writing/lifecycle-ending requests. The in-repo stdio bridge exposes this as `pid-sim-bridge-stdio --safe-mode`; TCP/WebSocket start there automatically and require explicit `--allow-mutations`. Both network binaries refuse non-loopback bind addresses, but forwarding, proxying, or tunnelling a loopback listener is not prevented. TCP/stdio JSONL lines are capped at 1 MiB; the WebSocket HTTP upgrade is capped at 16 KiB and each incoming client frame at 1 MiB; network reads/writes time out after 30 seconds per operation. There is no total request/session deadline, request-count cap, or aggregate-traffic limit, so progress-making trickle traffic can persist.

The accepted WebSocket upgrade is specifically `GET /bridge HTTP/1.1` with exactly one each of a nonempty `Host`, `Upgrade: websocket`, tokenized `Connection` containing `upgrade`, `Sec-WebSocket-Version: 13`, and a base64 key decoding to 16 bytes, and with no `Origin`; this does not claim that every malformed request is detected. The wire API is a single-request JSON-RPC 2.0 subset: batches are unsupported, missing-id notifications are silent and distinct from explicit `null`, parameters are omitted or named objects (not positional arrays), undeclared top-level method keys are rejected, and `sim.step` requires numeric `dt`. Profile-invalid parameters use `-32602`; handler/domain failures after validation use `-32000`. `log.replay` and `export.rerun` use non-adversarial canonical-path confinement below the session run-log directory, rejecting traversal, observed symlink components, non-regular/out-of-root inputs, missing output parents, and pre-existing outputs. It is not a security-grade sandbox against hardlinks, aliases, or concurrent local filesystem mutation. Transport run logs and Rerun outputs are no-replace. Export parses/manifests the same exact byte snapshot read from the source, encodes and hashes finalized RRD bytes, then stages, syncs, and persists them no-clobber. The executable transports use `File::sync_all` for the initial run-log prefix, every session flush before a wire response, and the terminal seal; generic `SimBridgeSession<W>` durability remains sink-defined. There is no parent-directory fsync, power-loss claim, or cross-file transaction joining the run log and export. Ordinary accepted-client errors seal `Failed` only while provenance storage is writable; a crash or storage failure can leave incomplete/unreadable provenance, an apparently complete terminal record with indeterminate status/durability, or an orphan RRD. Outside safe mode, `intervention.apply` supports deterministic `set_velocity`, `translate_object`, and `set_pose`, `log.stop` requests finalization, and `export.rerun` writes a new `.rrd`. These are local E0 controls only: there is no authentication, authorization, TLS, redaction, or remote-security assessment.

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
- Treat reconstruction quality as a measurement-quality nuisance: if large scene regions have high
  residual error or low view coverage, PID features can shift for perceptual rather than policy
  reasons. Do not call it a causal confounder without a prespecified causal graph supporting that role.
- Prospectively validate a per‑Gaussian quality/uncertainty map from held‑out view residuals and
  record scene-level stats (mean/median quality, coverage, fraction unreliable) as artifacts.
- Analyze the measurement only as a nuisance covariate, stratum, or exclusion sensitivity unless a
  different estimand is derived. Reconstruction uncertainty is not PID-estimator uncertainty.
- Optionally study uncertainty-guided **view selection** after defining a posterior/predictive
  observation law; route and log every accepted capture decision through the Agent Bridge.
- The weighted-KSG/PID expression retained in `GAUSS_MI_INTEGRATION.md` is quarantined E0 and must
  not be implemented as written; the covariate/view study remains optional E1 work.

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
The values below are an illustrative planning profile, not a frozen hardware selection or a safety
limit. Before use, bind the exact robot/URDF/controller revision and verify every limit against the
applicable official manual and local risk assessment.

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

These are synthetic-scene examples, not measured camera calibrations. Real or benchmark capture
must store the calibrated intrinsics, distortion, extrinsics, coordinate convention, resolution,
rate, exposure policy, calibration procedure, uncertainty, and content hash for the exact device/run.

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

The following arrays illustrate two candidate interfaces. The selected policy/controller defines
the actual action semantics, rate, units, clipping, saturation, and safety envelope; freeze and log
that contract rather than copying these nominal ranges.

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
1) estimator/measure gate (legacy Exp0 → S1), 2) harness bring-up with `Flow_gt`, 3) a selected
small baseline, 4) the M0/M2-selected primary VLA, then optional diffusion and predictor-driven
`Flow_pred` branches. Model names in the table are candidates, not a frozen selection.

**Model choice is an experimental variable.** Log `model_id`, revision/commit hash, preprocessing, and action parameterization for every run.

| Model | Role in this study | Minimum verified facts | Must verify before quantitative use |
|-------|---------------------|------------------------|-------------------------------------|
| **OpenVLA** | Candidate primary VLA after M0/M2 selection and freeze | arXiv:2406.09246: Llama‑2 7B + (DINOv2, SigLIP) + ~970k demos | Action representation; exact hook points for `V/L/D`; whether/where to export pre-attention states; licensing + checkpoint provenance |
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
    (500, 10, "noisy_shared_signal"),
    (500, 10, "unique_s1"),
    (500, 10, "xor_synergy"),
    (1000, 64, "independent_additive"),   # Target PCA dimension
    (1000, 64, "noisy_shared_signal"),
    (1000, 64, "xor_synergy"),
    (2000, 256, "independent_additive"),  # Stress test
]
 
def generate_synthetic_data(n: int, d: int, scenario: str, noise: float = 0.05):
    """Illustrative diagnostic laws; atom targets require a separate measure-specific oracle."""
    
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
        
    elif scenario == "noisy_shared_signal":
        # S1, S2, and T are conditionally independent noisy views of one latent base.
        # This is NOT a pure-redundancy atom oracle: independent sensor noise can make
        # each source complementary given the other. An exact continuous copy would
        # instead create a singular/tied sample that the continuous estimator rejects.
        base = rng.normal(0, 1, (n, 1))
        S1 = np.concatenate([base + noise * rng.normal(0, 1, (n, 1)),
                            rng.normal(0, 1, (n, d-1))], axis=1)
        S2 = np.concatenate([base + noise * rng.normal(0, 1, (n, 1)),
                            rng.normal(0, 1, (n, d-1))], axis=1)
        T = base + noise * rng.normal(0, 1, (n, 1))
        
    elif scenario == "unique_s1":
        # T = S1[0] + noise, S2 independent
        # The MI structure is I(S2;T)=0 and I(S1,S2;T)=I(S1;T); validate any atom
        # statement against the committed shared-exclusions oracle.
        S1 = rng.normal(0, 1, (n, d))
        S2 = rng.normal(0, 1, (n, d))
        T = S1[:, :1] + noise * rng.normal(0, 1, (n, 1))
        
    elif scenario == "xor_synergy":
        # T = sign(S1[0] * S2[0]) + noise
        # The individual source MIs vanish by symmetry while the joint MI is positive;
        # this supplies an MI/coherence control, not an assumed continuous-atom target.
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
- **Deletion/occlusion or perturbation tests:** compare top-attributed removal against prespecified
  random, low-attributed, and task-relevant controls under a justified replacement distribution.
  Zeroing can be out of distribution. Infer across independent cases and control draws; a threshold
  built from the mean and SEM of a few controls for one input is not a calibrated faithfulness test.
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

`pid-offline-harness` reads and hashes one bounded regular-file byte snapshot, then converts its `(V,L,D,A)` vectors into a canonical summary JSON plus run-log JSONL without reopening the mutable input path for provenance. The checked fixture lives at `crates/pid-sim/fixtures/offline_vlda_fixture.json`; each sample has a `sample_id`, optional `episode_id`, numeric `v`/`l`/`d`/`a` vectors, optional labels, and optional string metadata. An input with `source: "ncp"` additionally requires `capture_integrity` plus a committed `publication_receipt`; the reader verifies both artifact/run-log hashes, canonical-log validity/run id/exact dataset artifact identity, and a successful visible-receipt grade before analysis. The run log records `run_started`, `config_logged`, `frame_observed`, `label_observed`, `embedding_contract`, `embedding_captured`, two-source PID metrics for all `V/L/D→A` source pairs—`(V,L;A)`, `(V,D;A)`, and `(L,D;A)`—after deterministic per-variable standardization, plus train-split-only PID screens with train-only standardization when a recognized `metadata.split` exists, geometry diagnostics/gates over the standardized analysis space, evaluation metrics including deterministic sample-level, episode-grouped, and metadata-split held-out majority/1-NN/nearest-centroid **and SAFE-class logistic-regression (`heldout_logreg_vlda`; train-fit, held-out-scored)** success-label baselines when boolean `success` labels plus the relevant `episode_id`/`metadata.split` provenance are present, input/summary artifacts, and `run_ended`. A recognized held-out split uses `metadata.split=train`/`training` for training samples and `test`/`validation`/`val`/`eval`/`evaluation`/`heldout`/`holdout`/`held_out`/`hold_out` for held-out samples; summaries preserve split counts and sample IDs. Held-out baselines report accuracy and, when both held-out classes exist, balanced accuracy. Nearest-centroid baselines are train-standardized, train-only, require both success classes in the train split, and emit AUROC from the signed centroid-distance score when both held-out success classes are present. Summaries and run logs also include train-split PID status/provenance, held-out class-coverage status/counts, episode-disjointness status/counts for `episode_id` leakage, held-out per-sample prediction records in summaries/run logs, and failure-class confusion/rate diagnostics for majority, 1NN, and centroid baselines so missing train/held-out classes, split episodes that leak across train/held-out subsets, train-only PID availability, misclassified held-out samples, nearest train exemplars, centroid scores, failure recall, and false alarms can be audited. Replay summaries keep `*_metrics` as unique latest-by-name metric-name counts and add `*_metric_events` counters for total metric event volume.

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
is exploratory triangulation. A sampled Protocol A study must also freeze the random-number coupling
as part of its response estimand; a Protocol A success is not evidence for a Protocol B effect.

### 5.1 Task Definition
| Property               | Value                                                        |
| ---------------------- | ------------------------------------------------------------ |
| Task                   | Pick object A, place on target B                             |
| Instruction Format     | "Pick up the {color} {object} and place it on the {target}." |
| Success Criteria       | **TBD before capture from task utility and measurement error** |
| Timeout/censoring      | **TBD before capture with a frozen timeout and estimand rule** |
| Episodes per condition | **TBD by the frozen nested design and capture-sizing gate**   |

### 5.2 Experimental Conditions
```yaml
# experiments/configs/exp1_pick_place.yaml
# Non-executable design scaffold: every null count must be replaced by a frozen, justified count
# before capture. The current idealized power simulator does not supply that justification.
experiment_id: exp1_pick_place_v1
 
conditions:
  - name: baseline
    scene: scenes/simple_pick_place.yaml
    instruction: "Pick up the red cube and place it on the blue plate."
    perturbations: []
    n_episodes: null
    
  - name: lighting_intensity_variation
    scene: scenes/simple_pick_place.yaml
    instruction: "Pick up the red cube and place it on the blue plate."
    perturbations:
      - type: lighting
        params: {intensity_range: [0.3, 1.0], fixed_color_temp_k: 4500}
    n_episodes: null
    
  - name: distractor_objects
    scene: scenes/simple_pick_place.yaml
    instruction: "Pick up the red cube and place it on the blue plate."
    perturbations:
      - type: add_distractors
        params: {count: 3, objects: [blue_cylinder, green_sphere, ycb_spam]}
    n_episodes: null
    
  - name: novel_instruction
    scene: scenes/simple_pick_place.yaml
    instruction: "Grasp the crimson block and set it down on the azure dish."
    perturbations: []
    n_episodes: null
 
data_collection:
  cameras: [wrist_cam, overhead_cam]
  framerate: 30
  save_embeddings: true
  embedding_rate: 5  # Hz
  save_actions: true
  save_proprioception: true
```

**Language-identification boundary.** The scaffold above fixes one instruction inside each scene
condition. It therefore does not identify a within-condition language contribution, and a contrast
between `novel_instruction` and another condition is not by itself a V–L design. Use `(V,D;A)` as
the primary decomposition for this fixed-instruction version. A V–L endpoint requires a separately
frozen language factorial: sample multiple semantically controlled instruction variants independently
within every relevant scene/perturbation cell, randomize or otherwise justify their assignment,
demonstrate adequate occupancy/entropy, and keep the task target, constraints, visual scene, and
difficulty distribution fixed. Language variants and all repeated frames from one episode remain in
the same outer split.

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
    randomness_ledger_sha256: str
    
    # Outcome/event status (observed after prospective features at the declared label time)
    outcome_kind: str            # "success", named failure, "censored", or competing event
    success: Optional[bool]
    failure_mode: Optional[str]  # "miss_grasp", "drop", "miss_place", "timeout", "collision"
    outcome_time: Optional[float]
    censoring_time: Optional[float]
    label_observed_at: float
    
    # Trajectory (T timesteps at 30Hz)
    timestamps: np.ndarray                # (T,)
    images_wrist: np.ndarray              # (T, 480, 640, 3) uint8
    images_overhead: np.ndarray           # (T, 720, 1280, 3) uint8
    
    # Robot state
    joint_positions: np.ndarray           # (T, 7)
    joint_velocities: np.ndarray          # (T, 7)
    ee_poses: np.ndarray                  # (T, 7) [x,y,z,qw,qx,qy,qz]
    gripper_widths: np.ndarray            # (T,)
    
    # Model/controller-specific actions
    action_timestamps: np.ndarray          # (N_action,)
    actions_commanded: np.ndarray          # (N_action, d_A)
    action_tokens: Optional[np.ndarray]    # optional model-specific discrete representation
    
    # Model-specific embedding samples; dimensions/rates are declared in the run contract.
    embedding_timestamps: np.ndarray       # (N_embed,)
    embeddings_V: np.ndarray               # (N_embed, d_V)
    embeddings_L: np.ndarray               # (N_embed, d_L)
    embeddings_D: np.ndarray               # (N_embed, d_D)
    
    # Optional train-fitted derived representations, each bound to an exact transform hash.
    embeddings_V_reduced: Optional[np.ndarray]  # (N_embed, d_V_reduced)
    embeddings_L_reduced: Optional[np.ndarray]  # (N_embed, d_L_reduced)
    embeddings_D_reduced: Optional[np.ndarray]  # (N_embed, d_D_reduced)
    
    # Object tracking
    object_poses: Dict[str, np.ndarray]   # object_id -> (T, 7)
    grasp_events: List[Tuple[float, str]] # [(time, object_id), ...]
    
    # PID/CI is not embedded in this raw episode object. Derived reports carry computation status,
    # four scientific verdicts, provenance, uncertainty, and values only when produced.
```

Exploratory attribution artifacts use the implemented first-class `attribution_logged` event: method, target output, layer, modality, baseline, score hash, a legacy compatibility boolean named `faithfulness_check`, and artifact URI. `experiments/attribution/` currently produces epsilon-/AttnLRP and gradient×input evidence on a small reference model. Its frozen validation contract uses selection-disjoint and group-disjoint cases, compares deletion AOPC with bounded per-case random-ranking references, and aggregates group wins with an exact one-sided sign test. The boolean is true only on a typed gate pass, but that pass means only that the ranking found output-sensitive baseline replacements sooner under this declared design. It does not establish causal or mechanistic faithfulness; the baseline can be out of distribution and dependent features remain order-sensitive. Metadata binds the exact input/baseline set and complete relevance set and records the status, reason, group evidence, provenance, and limitations. The producer installs exact-byte content-addressed NumPy artifacts without replacement and replaces the run-log name last. The Rerun adapter always surfaces the recorded compatibility flag/provenance; only the standalone converter's explicit `--load-attribution-artifacts` mode reads a confined, regular, non-symlinked NumPy v1.0 `<f8` artifact, capped at 1024 finite values, and it requires the recorded exact SHA-256 and canonical shape to match before output. Bridge export keeps external loading disabled. Path checks do not protect against every concurrent filesystem race, and publication is not a cross-file transaction. Production VLA/LXT hooks and a production-model validation study remain future work.

### 5.4 Conditional PID/CI Analysis Contract

The former per-episode sliding-window Python sketch is retired. The stable Python wheel does not
expose continuous shared-exclusions PID, and tens of autocorrelated rows inside one episode do not
constitute an estimable population.

For any conditional H3 feature family:

1. declare the population law, observation/dependence model, source and target semantics, and exact
   preprocessing before looking at the holdout;
2. use V–D as primary for the fixed-instruction Exp1 scaffold; use V–L only after the independent
   language-factorial and occupancy gate in §5.2;
3. fit every standardizer/reducer on outer-training cases, serialize it, and reuse its exact hash;
4. construct each estimate from a prespecified pooled set of decorrelated rows across independent
   cases, keeping all rows from a case in one outer fold;
5. run recovery/MI coherence and the separate population, measure, estimator, and application gates
   for every source, target, and concatenation; and
6. publish a typed derived report. `produced` values include uncertainty and denominators;
   `produced_with_warning` values remain non-interpretable unless every interpretation gate passes;
   abstentions carry a stable reason and no zero, NaN, scalar, or metric event.

A pre-treatment H1 moderator may use only information available before assignment/application at its
frozen landmark. Post-treatment action/flow atoms and outcome-stratified estimates are exploratory
descriptions, not eligible H1 moderators or prospective H2 features.

### 5.5 Evaluation Units and Metrics

The earlier frame-level Mann–Whitney and variable-length episode-midpoint sketch is retired. It
treated autocorrelated windows as independent observations and defined a feature at a different
physical time whenever episode duration changed. Neither operation yields a valid confirmatory
endpoint.

Use this binding sequence instead:

1. Freeze the scientific unit and independence cluster before capture: snapshot/case for Protocol A,
   randomized case or reset/interference block for Protocol B, and event-free episode at a named
   landmark for H2. Frames and overlapping windows are repeated measurements, never extra units.
2. Define any temporal diagnostic at an absolute pre-treatment timestamp or a reproducible named
   task event available in real time. Fit its within-unit aggregation rule on outer-training data.
   Do not use a trajectory midpoint, completion time, future phase boundary, or outcome to locate it.
3. Keep every frame, window, clone replicate, landmark, and language variant from one persistent
   case in one outer fold. Estimate uncertainty by randomization unit or cluster; a within-episode
   block bootstrap may quantify measurement uncertainty but does not increase the independent sample
   count.
4. For Protocol A, score held-out predictions directly against the frozen `S_i` or replicate
   distribution and report absolute calibration, Monte Carlo error, clone-order/coupling sensitivity,
   and the locked design-only-versus-design-plus-diagnostic score contrast.
5. For Protocol B, report overall ITT first, then the frozen cross-fitted effect-specific loss,
   causal calibration, prioritization statistic, and policy value/regret. Do not score against
   fabricated per-case individual effects or use Protocol A output as their surrogate.
6. For H2, freeze landmark, horizon, competing-event/censoring rule, and alarm policy; use the
   prespecified censoring-aware proper score and retain nondetections. A generic AUROC is secondary.
7. Report eligible units, independent clusters, events/failures, abstentions, and exclusions for
   every metric. If any class, event, cluster, or gate requirement is unmet, abstain from the affected
   comparison rather than returning an unstable number.

Every protocol uses the `grandplan.md` §6.5 baseline frontier at matched information access and
compute. No
episode-level ΔAUROC, atom-sign contrast, or window-level test substitutes for these endpoints.

### 5.6 Attribution Baselines and Exploratory H4 Triangulation

For the same episodes, run a small prespecified set of attribution probes and method-specific sanity
tests if the model exposes the required gradients/layers. Do not promote the current reference
probe's event flag into a production-model faithfulness claim:

1. **Vision:** Grad-CAM or LRP/IG over visual features/patches; summarize relevance on task objects, distractors, target zones, and safety-critical regions.
2. **Language:** Integrated Gradients, DeepLIFT, LRP, or occlusion over instruction tokens/embeddings; summarize relevance on object, relation, negation, and constraint tokens.
3. **Concepts:** TCAV-style probes for human-defined concepts such as target color, object shape, distractor, collision-risk, or “avoid” constraints when concept examples can be defined without label leakage.
4. **Sensitivity maps:** vanilla gradient, Input×Gradient, SmoothGrad/VarGrad-style ensembles, or embedding-gradient probes as cheap baselines when differentiable hooks exist.
5. **Black-box/embedding baseline:** SHAP-style, permutation, or occlusion importance over reduced `V/L/D/A` features when gradients are unavailable.

Attribution and PID/CI do **not** estimate a common quantity. Attribution is local to a model call,
baseline, layer, and intervention; PID atoms are distributional functionals over declared random
variables. Their magnitudes and modality orderings therefore have no automatic common scale. With
only two modalities, an apparent ordering match is a single concordance indicator, not evidence of
rank correlation or validation.

Triangulate them only through separately estimated held-out consequences:

- preregister the attribution baseline, layer, sample-to-episode aggregation, modality mapping, and
  faithfulness endpoint; require deletion/randomization controls independently of PID;
- require all four PID gates independently and propagate atom abstention and uncertainty;
- estimate action consequences, paired frozen-snapshot algorithmic responses, or randomized
  execution effects under the applicable H1/H4 protocol, then ask whether each method predicts
  those consequences out of sample;
- for a synergy-motivated probe, use a frozen two-by-two intervention design (neither, V only, L
  only, both) and estimate its interaction on a declared outcome. Do not infer interaction from
  visually plausible heatmaps or from removing features chosen on the evaluation cases; and
- report agreement or disagreement descriptively after both methods pass their own controls. It may
  delimit method scope, but it cannot make either method faithful or causally valid.

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
| Success Criteria | **TBD before capture from task utility and measurement error**   |
| Timeout/censoring | **TBD before capture with a frozen timeout and estimand rule**   |

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

### 6.3 Prospective Phase-Event Contract

The former post-hoc threshold function is retired. It could read an uninitialized grasp time, merge
multiple events, and silently omit unresolved phases; more importantly, phase definitions tuned on
evaluation outcomes would leak future information.

Define a tested, versioned state machine before capture and emit its transitions into the canonical
run log as the episode unfolds. The contract must:

- use only contemporaneously available gripper, contact, pose, and controller events;
- freeze thresholds and debounce/hysteresis rules on pilot or outer-training data;
- assign stable event IDs and object IDs, require legal state transitions, and represent
  missing/ambiguous/duplicate events explicitly rather than guessing a boundary;
- keep automatic phase detection blinded to success/failure and later trajectory duration;
- preserve raw signals so a blinded audit can reproduce every transition; and
- report phase-detection disagreement/error on an independently annotated validation sample.

A phase-derived prospective H2 feature is eligible only after its event is observable at the declared
landmark. A completion-defined phase may be used for retrospective exploratory description, but not
as a predictor available before completion.

### 6.4 Temporal PID Analysis

The retired H5 temporal-atom endpoint remains exploratory. Its estimation unit is a phase/stage
window pooled across independent episodes, not a short within-episode window. For every stage:

1. freeze the online event or absolute elapsed-time boundary and within-window stride;
2. draw at most the declared number of decorrelated rows from each episode;
3. fit preprocessing on outer-training episodes and apply the exact frozen transform;
4. run the MI/coherence and measure-specific atom gates on the pooled analysis variables;
5. record the number of contributing episodes, rows per episode, outcomes, missing phases, and
   abstentions; and
6. obtain uncertainty by resampling episode/task-family clusters, recomputing preprocessing and the
   estimator inside each valid replicate.

Per-episode atom trajectories may be plotted only as explicitly non-evidentiary diagnostics. Do not
fit kNN PID to tens of autocorrelated rows, pool those point estimates in a t-test, or treat phases as
independent observations. The evidence-level trend is the preregistered contrast across pooled stage
estimates, with a mandatory MI/CI-only twin and episode-cluster uncertainty. A failure to reach the
validated independent-sample regime produces an abstention, not an atom curve.

### 6.5 Temporal-Synergy Windowing Preregistration (docset v12.5; frozen before capture)

Per `grandplan.md` §9.4 (token/temporal aggregation), the following parameters must be frozen in a
reviewed M0 successor **before the first real capture** and may not change after temporal-synergy data
are seen. Values remain TBD today; `grandplan.md` §6.8 simulation-based design analysis must justify
and bind them. Running evidentiary capture while any applicable value is TBD is a protocol violation.

| Parameter | Value | Rule |
|---|---|---|
| Window definition | prospectively emitted task phase where validated; otherwise absolute elapsed-time landmarks fixed from baseline | never use fractions of final episode duration or a future completion event |
| Window count | **TBD by `grandplan.md` §6.8 power/design analysis** | frozen before capture |
| Within-window stride | ≥ estimated decorrelation length (or cap ≤ m frames/episode/window; m TBD) | `grandplan.md` §2.5/§6.7 inherited fully — consecutive same-episode frames bias kNN point estimates |
| `N_win` (min pooled samples per window) | **TBD** — must be ≥ the S1-validated minimum for the active regime; counts **post-stride** samples | capture is scaled or the temporal-synergy analysis downgraded if unreachable |
| Episodes-contributing count | reported per window | mandatory |
| Outcome/missingness | report by outcome while retaining incomplete/censored phases | never select only completed episodes/phases or treat outcome as a pre-treatment covariate |

Primary construct: pooled windowed `Syn(V_t, D_t; A_t)` + mandatory CI-only twin (`grandplan.md` §3.8).
Secondary (exploratory only): `Syn(V_t, V_{t−h}; A_t)`. Endpoint: `grandplan.md` §6 statistical analysis plan (temporal-synergy trend).

---

## 7. Experiment 3: Instruction Perturbation (Robustness)
**Protocol role:** an H1 Protocol A/Protocol B manipulation-response pilot or an H4 availability–use study,
selected before capture. Protocol A uses paired frozen snapshots; Protocol B uses randomized
closed-loop assignment and ITT-first analysis. The legacy redundancy/unique rank-correlation rows
below are exploratory precursors to H3, not confirmatory endpoints. **Legacy H6 safety is deferred**
(`grandplan.md` §4): the constraint-content factor is logging-only unless process-level safety
outcomes and matched controls are separately preregistered.

### 7.1 Task Definition
| Property | Value |
|----------|-------|
| Task | Pick-and-Place (same as Exp1) |
| Variations | (i) a semantically controlled paraphrase factor; (ii) a separate constraint-content factor |
| Goal | Estimate responses to isolated language manipulations without mixing wording, task content, scene, or physical difficulty |

### 7.2 Experimental Conditions
```yaml
experiment_id: exp3_instruction_robustness
status: design_scaffold_not_frozen

# Factor 1 changes wording only. Scene, target, constraint set, and task semantics are fixed.
paraphrase_factor:
  semantics_id: red_cube_to_blue_plate
  variants:
    - {name: canonical, instruction: "Pick up the red cube and place it on the blue plate."}
    - {name: paraphrase_a, instruction: "Put the red cube onto the blue plate."}
    - {name: paraphrase_b, instruction: "Move the crimson cube to the blue plate."}

# Factor 2 changes constraint content and is analyzed as a distinct manipulation. Its matched
# scene contains the same blue cylinder in both arms; wording outside the constraint is fixed.
constraint_factor:
  scene: scenes/pick_place_with_blue_cylinder.yaml
  variants:
    - {name: unconstrained, instruction: "Pick up the red cube and place it on the blue plate."}
    - {name: avoid_cylinder, instruction: "Pick up the red cube and place it on the blue plate without touching the blue cylinder."}

assignment:
  unit: null                 # freeze episode/case/reset block before capture
  probabilities: null        # archive generated assignments for Protocol B
  episodes_per_cell: null    # set only by the scientifically adequate capture-sizing gate
```

Do not place the paraphrase and constraint factors in one severity ordering. Validate paraphrase
meaning without using evaluation outcomes; record tokenization and instruction occupancy. For each
factor, vary one treatment dimension at a time and hold scene, object poses, target, policy state,
decoder, and physical perturbations fixed or randomized independently. If a joint language×physical
interaction is scientifically required, declare and power the complete factorial before capture;
do not infer it from a few hand-picked combined cells.

### 7.3 PID Metric Analysis
**Exploratory legacy H2/H3 analyses (non-operative for confirmation):** first pass the instruction
diversity/occupancy/entropy gate; otherwise make V–D primary. Fit preprocessing once on disjoint
training data and freeze its hash. Redundancy-versus-severity slopes and unique-ordering-versus-
intervention-ordering correlations may be reported as exploratory H3 mechanism screens only after
the four PID gates. Strength matching is outcome-independent (for example equal embedding
displacement or equal MI destroyed), never equal-success-impact. These screens do not replace the
Protocol A direct-response score, the Protocol B effect-specific stack, or H2’s prospective proper
score.
**Exploratory (legacy-H6-adjacent, Deferred — no claim):** `Unq(L)` / `Syn(V,L;A)` contrasts between
the matched `avoid_cylinder` and `unconstrained` cells may be logged but are not evidence for a safety
claim. The constraint arm changes task content, not merely syntax, and requires its own engagement,
specificity, and physical-outcome checks.

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
 
### 8.2 3D Flow Extraction Contract (External; Planned)

No 2-D tracker/depth pipeline is implemented here. A future adapter must not clip an out-of-frame or
occluded track to the image boundary and pretend it is an observation. For every track-time point it
must retain a visibility/confidence status and represent missing depth explicitly.

Before any quantitative use, freeze and validate:

- camera intrinsics, distortion model, extrinsics, coordinate frame, units, and timestamp alignment;
- metric-depth calibration (relative depth cannot silently become metric 3-D motion);
- tracker identity switches, occlusion/re-entry rules, depth interpolation, and rejection thresholds;
- uncertainty or repeated-view error against held-out simulator/physical reference trajectories; and
- a train-fitted low-dimensional aggregation with its exact hash.

The output is a typed sequence of finite 3-D points or missing observations with source-frame and
calibration provenance—not merely an `N×T×3` array. `Flow_gt` from replayed simulator object poses is
the bring-up target; predictor-derived `Flow_pred` remains a separate, error-bearing measurement.

### 8.3 Flow PID Boundary

The ordinary `pid_core_rs` 1.0 wheel intentionally omits continuous shared-exclusions PID. Run any
continuous flow analysis through the feature-pinned Rust offline harness with explicit per-axis
population-support declarations, the exact preprocessing hash, and separate population, measure,
estimator, and application verdicts. Do not re-create removed scalar Python calls or use the
experimental migration module in an evidentiary workflow.

One short clip is not an independent kNN sample. Pool prespecified, phase-aligned observations across
independent clips/episodes, stride within episode by at least the estimated decorrelation length, and
keep every clip in one outer fold. Keep the flow target low-dimensional, but re-run recovery and all
four gates on every source, target, and concatenation after aggregation. If the required independent
sample count or any gate is unavailable, record an abstention with no numeric PID placeholder.

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
- **World-model diagnostic (embodiment-agnostic target):** estimate the transport change in CI/PID summaries for decompositions such as `(V, D_vla; Flow_gt)` or `(V, L; Flow_gt)` across robots on matched tasks/scenes. Do not presume stability; define a useful equivalence/noninferiority region and propagate estimator uncertainty before calling a feature transportable.
- **Policy/embodiment sensitivity:** compare how Flow relates to actions via `(V, Flow_gt; A_cmd)` and simple MI terms (e.g., `I(D_vla; A_cmd)`) across robots. The contrast describes the transported policy–adapter–controller system; it does not isolate embodiment by itself.

The same task name and nominal policy are insufficient for identification: adapters, camera law,
action coordinates, controller, reachable state distribution, and the operational `D` can all
change. Report overlap and each change explicitly; cross-embodiment differences are descriptive
transport results unless a separately randomized within-system intervention identifies a cause.
Avoid undefined atoms like `Syn(D; A_robot)`; synergy is a two-source construct.

---
 
## 10. Perturbation Library

**Implementation note (planned):** every perturbation below is an Agent Bridge call (e.g., `intervention.apply`); a GUI merely submits that same call. The bridge appends parameters and simulation/wall-clock timestamps before the perturbation handler runs. No renderer, observer, PID worker, or Zenoh subscriber applies a perturbation directly.

### 10.1 Visual Intervention Contract (Planned)

The fuller renderer interventions are not implemented. A future Agent Bridge adapter must encode each
visual treatment as a versioned command with factor name, dose, nominal-state hash, target, RNG
ledger reference, application time, receipt, and a post-application manipulation measurement.

| Factor | Dose | Must remain fixed |
|---|---|---|
| Illumination intensity | absolute or nominal-relative scalar | color temperature, source direction, geometry, exposure |
| Illumination color temperature | calibrated kelvin/source spectrum | intensity, direction, geometry, exposure |
| Distractor count | integer with a frozen placement generator | candidate locations, object family, target visibility |
| Texture/appearance | content-addressed asset variant | geometry, physics proxy, illumination |
| Occlusion | area/pose under a frozen generator | target, illumination, all physical properties |

Do not use renderer settings that also change physics, camera calibration, automatic exposure, or the
task target unless those are explicit factors in a complete factorial.

### 10.2 Physical Intervention Contract (Planned)

The current bridge's implemented local interventions are narrower than this planned library. A future
physics adapter must reset to a content-addressed nominal state before every dose and record the
backend-specific receipt plus a measured manipulation check.

| Factor | Dose | Required isolation |
|---|---|---|
| Object mass | scale applied to the frozen nominal mass | inertia rule, geometry, friction, initial pose |
| Friction | absolute coefficient/material pair | mass, contact geometry, solver settings |
| Translation noise | logged vector drawn from its own RNG stream | rotation fixed |
| Rotation noise | logged axis-angle drawn from its own RNG stream | translation fixed |
| Impulse | logged impulse vector at a named simulation time | do not describe it as a duration-bearing force |
| Force schedule | force vector plus integration interval and application schedule | distinct estimand from impulse |

Reject a cell if the requested factor is unsupported, the nominal-state hash differs, treatment
receipt is absent, or a supposedly isolated manipulation changes an undeclared factor.

### 10.3 Perturbation Schedules
```yaml
# experiments/configs/perturbation_schedules.yaml
 
# Retired-H4 exploratory generalization battery (not registry H4)
generalization_battery:
  - name: lighting_intensity_sweep
    type: lighting_shift
    fixed: {color_temp_k: 4500}
    variations:
      - {intensity_delta: -0.3, color_temp_k: 4500}
      - {intensity_delta: 0.0, color_temp_k: 4500}
      - {intensity_delta: 0.3, color_temp_k: 4500}

  - name: lighting_temperature_sweep
    type: lighting_shift
    fixed: {intensity_delta: 0.0}
    variations:
      - {intensity_delta: 0.0, color_temp_k: 3000}
      - {intensity_delta: 0.0, color_temp_k: 4500}
      - {intensity_delta: 0.0, color_temp_k: 6500}
  
  - name: distractor_density
    type: add_distractor
    variations:
      - {count: 0}
      - {count: 1, positions: [[0.5, 0.1, 0.04]]}
      - {count: 3, positions: [[0.5, 0.1, 0.04], [0.35, -0.05, 0.03], [0.55, 0.15, 0.05]]}
  
  - name: object_translation_noise
    type: position_noise
    variations:
      - {sigma_xy: 0.0, sigma_theta: 0.0}
      - {sigma_xy: 0.02, sigma_theta: 0.0}
      - {sigma_xy: 0.05, sigma_theta: 0.0}

  - name: object_rotation_noise
    type: position_noise
    variations:
      - {sigma_xy: 0.0, sigma_theta: 0.0}
      - {sigma_xy: 0.0, sigma_theta: 0.1}
      - {sigma_xy: 0.0, sigma_theta: 0.2}
 
# Exploratory embodiment-gap battery (legacy H7)
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

**Factor-isolation rule.** A one-factor sweep holds every other perturbation parameter at a declared
nominal value and restores the same validated pre-treatment state before each cell; doses are
computed from that frozen nominal, never compounded from the preceding cell. For example, do not
change illumination intensity and color temperature, or object
translation and rotation, in the same cell and then attribute the response to either component.
Distractor-count sweeps use a frozen placement generator so count is not silently confounded with
location. Mass, friction, correction-loop status, instruction content, and visual corruption remain
separate factors. A scientific interaction requires a complete randomized factorial (including all
main-effect cells), an interaction estimand, and cluster-aware sizing; an ad hoc combined stress-test
cell is descriptive only.

**Exploratory legacy logging only (non-operative for confirmation):** perturbation sweeps may log
per-severity `Syn`, uncertainty, instruction occupancy/entropy, a frozen transform hash, and SSI :=
−IQR(Syn) for descriptive continuity with the retired memorization/generalization analysis. This is
not registry H4, whose endpoint is the availability–use divergence defined in the binding table and
`grandplan.md` §4/§6.3. Legacy-H3 sweeps may likewise log an outcome-independent intervention-strength
yardstick (embedding displacement or MI destroyed), but they do not substitute for current H1/H3.

---
 
## 11. Data Formats & Storage

### 11.1 Episode Sidecars (Optional; Run Log Remains Canonical)

The earlier standalone HDF5 writer is retired. It could publish an analysis file without a canonical
run-log binding and wrote PID arrays unconditionally, which cannot represent abstention without a
numeric placeholder.

If HDF5 or another columnar container is used for throughput, treat it as an immutable derived
sidecar. Its schema must include:

- schema version, run ID, canonical run-log content hash, source-artifact hashes, and the exact
  adapter/model/config/transform identities;
- sample, episode, case, event, assignment, receipt, timestamp, and split keys sufficient to join
  every row back to canonical events without positional inference;
- raw-versus-derived status, dtype, shape, units, coordinate frame, rate, missingness, and
  preprocessing lineage for every array;
- label provenance and observation time so a future/outcome label cannot enter a prospective
  feature; and
- per-estimate computation status plus all four scientific verdicts. A numeric dataset exists only
  for `produced` or `produced_with_warning` estimates; an abstention has a reason and no numeric PID
  dataset, zero, NaN, or metric event.

Install sidecars without replacing an existing target, hash the finalized bytes, and record the
artifact event in the canonical log. A release check must reconstruct the declared samples from the
log/source artifacts and compare keys, shapes, hashes, labels, assignments, and produced values to
the sidecar. Passing HDF5 parsing alone is not provenance-complete replay.

### 11.2 Dataset Index (JSON)

Example schema (values below are illustrative placeholders; populate from actual run metadata and measured statistics):
```json
{
  "example_only": true,
  "dataset_id": "prisoma_exp1_v1",
  "created": "2026-01-15T10:30:00Z",
  "experiment": "exp1_pick_place",
  "n_episodes": 400,
  "conditions": ["baseline", "lighting_intensity_variation", "distractor_objects", "novel_instruction"],
  "episodes_per_condition": 100,
  
  "episodes": [
    {
      "episode_id": "exp1_baseline_001",
      "condition": "baseline",
      "filepath": "data/exp1/baseline/episode_001.h5",
      "outcome_kind": "success",
      "success": true,
      "outcome_time_s": 12.4,
      "label_observed_at_s": 12.4,
      "randomness_ledger_sha256": "<example-ledger-sha256>"
    }
  ],
  
  "statistics": {
    "overall_success_rate": 0.73,
    "by_condition": {
      "baseline": {"success_rate": 0.85, "mean_duration": 11.2},
      "lighting_intensity_variation": {"success_rate": 0.71, "mean_duration": 13.1},
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

### 13.1 Randomness, Coupling, and Assignment Ledger

A single master RNG or a function of episode index is not an adequate reproducibility contract. It
can create accidental aliases between case selection, treatment assignment, environment noise, and
policy sampling, and it cannot describe Protocol A's coupled potential computations.

Use independent, domain-separated streams for at least:

- task/case sampling and split construction;
- treatment assignment and run-order randomization;
- environment transitions and sensor corruption;
- policy decoding/sampling;
- intervention generation and placebo selection;
- learned-transform initialization/tuning; and
- attribution controls, bootstrap, and permutation procedures.

For every stream record its semantic role, generator algorithm and library version, seed/key,
counter/substream range, draw count or draw-ledger hash, owning unit, and whether replay is exact or
tolerance-bounded. Generate and archive Protocol B assignments before treatment, record planned and
realized probabilities, and never regenerate a favorable assignment after outcomes are visible.

For sampled Protocol A responses, the coupling of treatment-0 and treatment-1 policy streams is part
of S_i(C). Declare whether streams are common, antithetic, or independent; preserve per-side draw
ledgers; randomize clone order; and repeat a frozen subset with independent streams when feasible.
Do not pool different couplings as repeated measurements of one estimand.

Seed equality does not prove deterministic execution. Also bind model/checkpoint, preprocessing,
device, kernels, precision, thread/scheduler settings, simulator integrator, and reset state, then
measure replay divergence. Report failed resets, divergent draw counts, and nondeterministic runs
rather than silently dropping them.

### 13.2 Version Pinning
- **Python dependencies:** pinned in `uv.lock` (run `uv sync`; commit lockfile changes).
- **Rust dependencies:** pinned in `Cargo.lock` (run `cargo build`; commit lockfile changes).
- **Model code/weights:** record upstream repo URL + commit hash and checkpoint identifiers in the experiment manifest.
- **External stacks (optional):** if you use video generation / segmentation / tracking / depth packages, pin them in a dedicated environment and record their lockfiles/commits.

### 13.3 Experiment Manifest
Every experiment run should produce a manifest file. The values below are schema examples only;
they are not observed results, capture requirements, tested version pins, or a planned study size:
```yaml
# results/exp1_run_001/manifest.yaml
example_only: true
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
  
randomness:
  ledger_path: results/exp1_run_001/randomness.json
  ledger_sha256: <record-finalized-ledger-sha256>
  assignment_manifest_sha256: <record-assignment-manifest-sha256-or-null>
  protocol_a_coupling: <common-antithetic-independent-or-not-applicable>
  
config:
  experiment_config: experiments/configs/exp1_pick_place.yaml
  scene_config: scenes/simple_pick_place.yaml
  
results:
  n_episodes: 400
  success_rate: 0.73
  canonical_runlog: results/exp1_run_001/runlog.jsonl
  canonical_runlog_sha256: <record-finalized-runlog-sha256>
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

## 14. Retired World-Model Comparison Sketches (Non-Operative)

The former §§14–19 described five directional comparisons between ManiGaussian and PEGS as
H_WM1–H_WM5 and included executable-looking YAML/Python. Those sketches are retired. They were never
implemented in this repository, their fixed counts were not justified by the nested design, and
several proposed endpoints did not identify the stated scientific questions. They must not be run,
cited as preregistered hypotheses, or used to size a capture. Historical text remains available in
version control.

The two project names below are retained only to identify the retired sketches. Any successor must
reverify each external system's current code, checkpoint, supported object class, license, and
interface from primary sources at a frozen revision. Neither system is a validated dependency here.

### 14.1 Failure analysis

| Retired sketch | Why its proposed test was invalid | Minimum defensible replacement |
|---|---|---|
| **H_WM1: prediction fidelity via `I(Prediction; P_GT)`** | Mutual information measures dependence, not numerical prediction accuracy. An invertible but physically wrong remapping can preserve MI, while a noiseless deterministic continuous relationship may have no finite MI. Fitting transforms on evaluation trajectories and treating timesteps as independent add further leakage and pseudoreplication | Use physically scaled trajectory error for deterministic predictions; use a proper log score plus calibration for probabilistic predictions. Freeze horizons/tolerances from task utility, fit every transform on outer-training cases, pair systems on identical cases, and infer at the case/task-family level. MI may be a gated secondary diagnostic only |
| **H_WM2: novel-object stability from different PID atoms** | The sketch compared variances of different functionals, variables, targets, dimensions, and scales across systems. That contrast has no common estimand and cannot establish generalization | Give both systems the same supported cases, inputs, target, prediction horizon, scoring rule, and information budget. Estimate each system's familiar-to-novel score change and the paired system-by-shift interaction, with task-family-held-out uncertainty |
| **H_WM3: perturbation compensation** | KL divergences between unrelated atom vectors and a comparison of raw unique information with a ratio are not commensurate. Hard-coded percentage/correlation thresholds had no utility or uncertainty basis, and combined perturbations mixed mass, friction, and correction status | Randomize isolated mass, friction, visual corruption, and correction-loop factors. Use prediction/execution error and physically interpretable correction effort as primary outcomes. Declare any interaction factorial in advance; report uncertainty and multiplicity. Compare PID only under an identical validated regime and denominator |
| **H_WM4: temporal coherence** | The sketch assigned every phase `success=True`, making outcome correlation undefined; it reused one completion expression for both systems, treated within-episode phases as independent, and applied an unpaired test to paired cases | Record real outcomes and prespecified phase events, retain missing/censored phases, summarize a frozen within-episode temporal functional, and compare systems with paired or hierarchical inference at the episode/task-family cluster. A constant outcome cannot support correlation |
| **H_WM5: deformable manipulation** | Selecting a comparator known not to support the task predetermines the result and violates overlap. Analyzing only failures is post-outcome selection; fitting PCA/standardization on evaluation data leaks; high language-unique information does not prove fallback or mechanism | Compare performance only where both systems have declared support and matched access, or report a descriptive capability boundary with no performance hypothesis. Analyze all assigned cases, fit transforms on outer training data, use direct task/shape metrics with uncertainty, and require targeted interventions before any mechanism language |

These are logical identification failures, not matters that can be repaired by a larger frame count or
a smaller p-value. In particular, PID and attribution cannot turn incomparable prediction targets
into a common performance measure.

### 14.2 Requirements for any successor comparison

A new world-model study starts from a new, reviewed protocol rather than editing the retired
hypotheses in place. Before capture it must satisfy all of the following:

1. **Question and estimand:** state whether the target is deterministic prediction error,
   probabilistic forecast quality, distribution-shift degradation, correction effort, task outcome,
   or a randomized intervention effect. Give its unit, target population, time horizon, censoring
   rule, and minimum useful difference.
2. **Common support:** demonstrate that every comparator can receive the same admissible inputs and
   produce the same target for the analyzed cases. Separate an unsupported-capability inventory from
   a performance comparison; do not condition the latter on observed success or failure.
3. **Comparator parity:** freeze exact repositories, commits, checkpoints, licenses, preprocessing,
   observation/action budgets, tuning budget, compute budget, and allowed online corrections. Report
   structural differences rather than attributing a cross-family contrast to one architecture.
4. **Independent design:** pair systems on content-identical cases when possible; randomize run order
   and intervention assignment; block by task family; keep all timesteps, phases, seeds, and derived
   windows from one persistent case in one outer fold; and infer at the randomization/interference
   cluster.
5. **Leakage control:** fit PCA, standardization, feature extraction, thresholds, phase rules, and
   model selection only within outer-training data. Use an untouched task-family or later-time test
   set for the frozen final comparison.
6. **Proper primary scores:** use physical error/utility for deterministic trajectories and proper
   scoring rules plus calibration for predictive distributions. Define any prediction-horizon
   threshold from external task utility and handle right censoring explicitly. MI is not a fidelity
   score.
7. **Factor isolation:** manipulate mass, friction, visual evidence, object novelty, instruction,
   and correction status separately. A claimed interaction requires the full factorial and a
   prespecified interaction contrast; a combined stress cell is descriptive.
8. **Uncertainty and multiplicity:** size the nested family→case→episode design by simulation under
   plausible null, weak, nonlinear, and failure regimes. Freeze the primary contrast, smallest
   effect of interest, stopping rule, exclusions, and multiplicity family. No episode count is
   specified here.
9. **Conditional PID use:** PID is secondary and only admissible after population, measure,
   estimator, and application gates pass for the exact variables and preprocessing. Use the same
   estimand, functional, information scale, and uncertainty treatment across systems; otherwise do
   not compare atoms.
10. **Provenance and authority:** route all interventions through the Agent Bridge, reconstruct every
    analyzed case from the canonical run log, and bind source data, assignments, transforms,
    checkpoints, outputs, exclusions, and analysis code by content hash.

### 14.3 Reactivation gate

A successor may be added only after M0 has a genuinely frozen reviewed study, the relevant external
adapters and golden fixtures exist, a pilot demonstrates support and manipulation specificity, the
nested capture-sizing analysis passes, and an analysis dry run fails closed on leakage, unsupported
cases, missing clusters, and PID abstention. Until then, world-model comparison is a research
question—not an active experiment, result, or directional hypothesis.

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
