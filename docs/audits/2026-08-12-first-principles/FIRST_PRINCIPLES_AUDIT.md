# Prisoma first-principles audit, 2026-08-12

This memo records the review opened on 2026-08-12 and refreshed through 2026-08-13. It covers
Prisoma's hypotheses, claims, sources, architecture, implementation, and documentation. It is a
repository audit. It is not a preregistration, a scientific result, a systematic review, or
evidence that M0 is freeze-ready.

## Decision

Keep Prisoma. Narrow its claims. Freeze science before adding infrastructure.

The project has a coherent low-overhead architecture for recorded interventions, replay, bounded
analysis, and diagnostic comparison. Its strongest contribution is the discipline that separates:

- signal availability;
- response to a named frozen-state intervention;
- response to a randomized closed-loop intervention;
- prospective prediction before an outcome;
- PID estimator output;
- application-valid scientific interpretation.

No scientific PID gate opened in this audit. The current repository remains software groundwork
and preregistration scaffolding. It does not contain EC1 or H1–H4 scientific evidence.

The central correction is:

> Prisoma can test named interventions and evaluate pre-outcome predictors. It cannot infer
> unrestricted natural pathway use without a stronger identification design.

The most important statistical correction is:

> Under censoring, the prediction object and score must match. A forecast-independent conditional
> IPCW Brier construction can target scalar horizon risk under its exact assumptions. A
> right-censored likelihood instead requires the full event-time-and-type law.

The most important deployment correction is:

> H3 concerns the value of a complete deployed policy over the full target population. It does
> not concern only the cases where PID returns a number.

The most important architecture correction is:

> VLA and WAM are not exclusive scientific classes. Prisoma must classify the deployed graph,
> then test predictive accuracy, runtime use, action conditioning, and planning separately.

## Audit boundary

| Item | Bound value |
|---|---|
| Starting Prisoma revision | `6d6f895d57ec38feb417a6027cab8dcdf525ce2a` |
| Starting branch state | clean `main`, equal to `origin/main` |
| Pinned `pid-rs` | `796c11e70f009634b853dc4ada6f565563d82f51` |
| Pinned description | `v0.9.0-7-g796c11e` |
| Immutable `pid-rs` 0.9.0 tag commit | `a9a275157237999c8da6ab813130d74f6113dec9` |
| Public `pid-rs` main reviewed | `bbdfda40f0a49a2260b10eafdcb438fc61ae94e9` |
| Real-study or holdout access | none |
| Submodule update | none |

The review preserved immutable release intake, historical records, generated-file ownership, and
the estimator submodule boundary.

The final ecosystem refresh observed NCP main at
`1a04294c90c1b50eba06ae1c6afe9c951319250d` and Paper2Brain main at
`2648caf18d24075c4a36af81a6bb032bb551244e` on 2026-08-13. These revisions did not change
Prisoma's dependency or integration verdicts.

## Method: 500 claim views

The audit crossed five claim families with ten inference-chain questions and ten assurance lenses.
This gives 500 claim-question-lens views. H1-A and H1-B received separate answers inside every
relevant H1 view. The number describes the review matrix. It does not imply 500 independent
reviewers or 500 empirical tests.

The inference chain asks about the question, population, intervention or prediction time, data,
unit, estimand, assumptions, estimator, comparator, falsifier, conclusion, and evidence status.
The assurance lenses cover construct validity, causality, statistics, measurement, estimation,
time and transport, systems, provenance, deployment, and governance. The exact matrix is defined
in `AUDIT_LEDGER.md`.

The canonical plan also defines 50 concrete design lenses. They expand the ten assurance
categories into distinct failure tests. They do not change the matrix count or claim 50 reviewers.

Four rules governed every view:

1. A definition is not an empirical result.
2. A fixture proves only its named software behavior at its named bytes.
3. A valid estimator does not validate a population, measure, or application.
4. A run log cannot prove an upstream event that the capture boundary never observed.

## Hypothesis audit

### EC1: finite recorded-event acceptance

**Audit status:** software groundwork only.

EC1 can establish finite acceptance over a frozen universe. That universe must enumerate adapters,
events, variables, faults, endpoints, and replay obligations. Each fault-adapter pair needs its own
minimum detection requirement. An aggregate rate cannot rescue a missed mandatory pair.

The run log is the source of truth for accepted recorded events. It cannot establish universal
provenance completeness. It cannot prove that a producer emitted an event that no accepted capture
boundary observed. A receipt proves only its declared publication facts.

Permitted conclusion after a future pass:

> The named adapters and fixtures met every registered EC1 obligation under the frozen schema.

Still prohibited:

- “all faults are detected”;
- “the log is universally provenance-complete”;
- “a successful receipt proves upstream delivery completeness”;
- “local fixture execution is an external producer-consumer qualification.”

The current M0 bundle has the right finite structure. It has no confirmatory holdout and remains
unfrozen. The NCP fault observatory is local fixture evidence, not E3 producer-consumer evidence.

### H1-A: frozen-state algorithmic response

**Audit status:** synthetic scoring reference only.

H1-A asks whether pre-intervention diagnostics predict a declared response of a frozen software
state. Both cloned responses can be executed. The estimand is therefore an algorithmic response,
not an unobserved physical individual effect.

The implementation must bind the full executable state. This includes weights, adapters, caches,
preprocessing, precision, kernels, decoding, random state, and the intervention. Stochastic policies
need a frozen draw-coupling rule and Monte Carlo error accounting.

A future pass can support only the evaluated frozen-state response claim. It cannot establish
closed-loop robustness, physical effect moderation, or natural pathway use.

The earlier v2 machine contract typed only calibration-bin provenance. That was not enough. It
could not reject an opaque or weak success rule. The revised contract now freezes one response
functional, proper score, matched-access comparator, positive useful margin, one-sided superiority
rule, uncertainty, calibration consequence, multiplicity procedure, and finite-benchmark or
replication scope. A lower bound must exceed the useful margin. Noninferiority or a secondary
metric cannot establish success.

### H1-B: randomized closed-loop effect modification

**Audit status:** unimplemented.

H1-B asks whether frozen pre-treatment diagnostics moderate a randomized embodied effect. It needs
an actual policy, task, environment, assignment, intervention, outcome, and interference unit.

The primary endpoint must be effect-specific. Factual-outcome prediction is only a nuisance or
secondary check. The plan now requires one frozen primary from a causal validation stack, plus a
prespecified hierarchy for the remaining checks. The overall intent-to-treat effect, assignment
integrity, engagement, and nuisance diagnostics remain mandatory.

The revised machine contract also freezes a positive useful margin and one-sided superiority
decision for H1-B. It binds the complete effect-validation stack, ITT and design checks,
uncertainty, and directional replication. Factual fit or a secondary endpoint cannot rescue a
failed primary effect endpoint.

H1-A and H1-B cannot be pooled. They have different units, observations, estimands, uncertainties,
and permitted conclusions. A project result must always say H1-A or H1-B.

Randomization identifies the implemented treatment contrast under the declared design. It does not
identify unrestricted natural mechanism use. A weak effect of one intervention can reflect poor
engagement, compensation, saturation, off-manifold behavior, or the wrong intervention site.

### H2: prospective censoring-aware failure prediction

**Audit status:** synthetic protocol arithmetic only.

H2 is a prospective prediction claim. Every feature must be available at or before the landmark.
A global dataset PID atom is not an episode feature. Local information scores need a training-only
reference distribution, cross-fitting, eligibility, and a frozen aggregation.

The review found that the earlier prediction-object contract was too vague. An intermediate audit
draft then overcorrected it by requiring a full event-time-and-type forecast for every censored
score. Direct review of the primary scoring papers rejected that rule.

The final primary contract separates three cases:

1. With complete follow-up for the full frozen eligible ledger, use a proper score for the declared
   complete-data target.
2. A forecast-independent conditional IPCW Brier construction can properly score scalar horizon
   risk on its identifiable region. It can also estimate a declared complete-data risk. The exact
   role, censoring law, positivity assumptions, and nuisance fitting must be frozen.
3. A right-censored likelihood scores a full event-time-and-type law. It cannot replace a horizon
   risk score merely because both use censored observations.

The competing-event ontology must be complete. The prediction object, score, risk, censoring
construction, assumptions, minimum useful margin, and uncertainty method form one primary
contract. The labels `IPCW`, `Brier`, and `likelihood` do not establish propriety by themselves.

Comparator selection is also frozen. “Strongest applicable baseline” is not an executable rule.
The plan now requires exact implementations or a pre-outcome rule for matched analogues. All
comparisons must disclose supervision, white-box access, resampling, external models, latency,
compute, and actionability.

Random frame splits remain prohibited. A deployment claim needs an untouched external or later-time
target. Recalibration uses a separate split and cannot be hidden inside test evaluation.

### H3: value of a complete PID policy

**Audit status:** not eligible.

H3 is not “PID works where it reports a number.” It evaluates one complete deployed policy over the
full frozen target population. That policy includes:

- the population and sampling law;
- PID measure and preprocessing;
- estimator configuration;
- eligibility and support checks;
- warning and abstention rules;
- the exact non-PID fallback;
- latency, failure, and compute costs;
- the comparator and minimum useful effect.

The current high-dimensional MI/coherence path remains NO-GO. The continuous shared-exclusions
application gate remains BLOCKED and NOT APPLICATION-VALIDATED. Experiment 0 reports its atom
measure as not adjudicated and its atom estimator as blocked.

An abstention has no numeric placeholder. It is neither zero nor NaN. A failed continuous term
must not route to discrete `I_min`. Those are different measures and estimands.

Geometry can help define a warning or eligibility rule. It cannot substitute for measure,
estimator, or application validation. Eligible-only performance must be reported, but it cannot
serve as the confirmatory H3 estimand.

### H4: preselected probe and intervention branch

**Audit status:** unimplemented.

H4 is a preselected alternative and companion branch. It cannot become a post-result fallback after
H3 fails on the same holdout. Such a switch needs a fresh holdout and sequential error control.

Probe quality and intervention response are different claims. Attribution agreement, deletion
sensitivity, or prediction does not establish causal faithfulness. A tested intervention effect
describes the named contrast. It does not prove individual natural non-use.

The plan retains the stronger H4 construction. It requires a frozen probe-selection rule, target,
intervention, equivalence margin, transport target, multiplicity rule, and independent replication.
The current attribution package is an exploratory baseline only.

## Cross-hypothesis findings

### One claim cannot rescue another

- EC1 software acceptance cannot validate H1–H4 science.
- H1-A cannot stand in for H1-B.
- H2 prediction cannot establish causal pathway use.
- H3 abstention cannot become evidence for H4.
- H4 intervention evidence cannot retroactively validate PID atoms.

### The independent unit is not a row

Episodes, seeds, task families, policy checkpoints, persistent worlds, and operators can create
dependence. The outer split and uncertainty method must use the highest independent unit. Sample
count alone is not evidence of precision.

### Missingness is part of the policy

Missing sources, ineligible regimes, observer drops, censoring, warning states, and estimator
abstention must remain visible. Complete-case filtering changes the target. A future deployment
claim must score the fallback policy on those cases.

### Cost and timeliness are scientific comparison axes

A white-box detector, external world model, repeated action sampler, and simple kinematic monitor
do not have equal access or cost. Report a cost-accuracy-timeliness frontier when access cannot be
matched. Do not hide annotation burden, action calls, compute, latency, or recovery coupling.

## Architecture audit

The high-level architecture is sufficient for the current research stage.

1. The Agent Bridge remains the only control plane.
2. The run log remains authoritative for accepted recorded events.
3. Rerun remains the Phase 1–3 diagnostic and replay viewer.
4. Tauri and SparkJS remain deferred Phase 4 presentation work.
5. NCP remains an optional read-only source outside the root workspace.
6. `pid-rs` remains the sole estimator source of truth.

No new service, database, queue, control plane, or custom viewer is justified now. The binding
constraints are scientific freeze, rights-approved capture, comparator fidelity, and external
validation. More infrastructure would not close them.

“Low overhead” therefore means:

- bounded local files and processes;
- typed schemas and explicit limits;
- content-bound generated views;
- fail-closed admission;
- one canonical run log;
- no hidden authority;
- no component without a named decision or gate.

## Implementation findings

### NCP sample time

The observer previously allowed command timing to stand in for the driving sensor clock. That can
misstate lineage because V, A, and D are joined on the sensor position. The observer now derives
the sample timestamp from the driving sensor frame. It rejects negative, non-finite, and
out-of-range values before state mutation. Valid fractional nanoseconds truncate toward zero.

### NCP success label

A configured success channel is now an exact binary scalar contract. It must contain one `0` or
`1`. Zero means false. One means true. Absence means no label. Other finite values fail closed.

The success channel must not equal the language channel. Without that guard, a language embedding
could silently become an outcome label. This was a direct feature-outcome ontology leak.

Every capture ingress now validates the mapping before mutation. Each kept sample and capture
event preserves the exact converted sensor value as `sensor_timestamp_ns`. The run-log event clock
uses a separate nondecreasing projection.

### Temporal and resampling contracts

The harness formerly computed AR(1)-derived sample-size and block-length hints from unlabeled row
order. Row order alone does not identify a time series. An `episode_id` groups rows but does not
order them. Those values now require one declared episode that spans every row and a strictly
increasing canonical decimal `metadata.sequence_index`. The lag-1 row-order value remains descriptive and cannot set an
inferential denominator or block length.

The uncertainty API also allowed one request to combine a multi-row block bootstrap with a full
shuffle, or a unit-block bootstrap with a circular serial surrogate. Serial transforms also require
the same sequence-index receipt. Those pairs assert
incompatible row-dependence laws. The library and CLI now reject both combinations before main
analysis. This validates neither resampling law. It prevents one report from claiming both.

### H1 scaled-response arithmetic

The H1-A reference formerly computed an L2 response with a direct sum of squared deltas. That
calculation can overflow when every input and the represented Euclidean norm are finite. It could
also emit an infinite response from finite inputs.

The metric now scales opposite-sign operands before addition and accumulates the norm with
`hypot`. It retains a representable dimensionless delta even when the unscaled subtraction would
overflow. It rejects a non-finite scaled delta or final norm. Focused tests cover both boundaries.
This fixes numerical admission. It does not create H1-A evidence.

### Repository truth

The research-governance successor schema now enforces the corrected H2 prediction-object contract.
It also binds H3's positive useful-value margin, full-population score contrast, one-sided
superiority rule, PID-feature construction, dependence-aware uncertainty, support policy,
multiplicity, and replication target. Current prose distinguishes the exact H3 gate states.
Engram pairing is described as startup-secret possession, not process or build attestation. The
docs now define the run log only over accepted recorded events.

These changes improve contracts. They do not promote evidence.

## Current literature review

### Method

The review checked primary papers, official proceedings, DOI records, arXiv records, official
repositories, and the repository's offline arXiv cache. Searches covered PID, VLA monitoring,
counterfactual and mechanistic VLA work, censored scoring, competing risks, causal-effect
validation, and current estimator assurance.

X was searched for discovery. The results were aggregation or promotional posts. No X post was
used as a source, and no claim changed because of one.

This was a broad decision-focused source review. It was not a registered systematic review. Search
provenance is not complete enough to support that label.

### Sources that changed the plan

| Source family | Primary source fact | Prisoma consequence |
|---|---|---|
| PID non-uniqueness | Williams–Beer, shared exclusions, and conditional-independence PID define different measures. The 2026 review finds no universal choice. | Keep the measure gate. Never pool atoms across measures. |
| Continuous estimation | KSG is a nearest-neighbor estimator with difficult finite-sample behavior under strong dependence. | Keep dimension, tie, support, power, and external-fixture gates. |
| Gaussian multi-source PID | Lyu, Clark, and Raviv give covariance-based closed forms under a Gaussian model and a different measure family. | Treat it as a separately named sensitivity regime, not shared-exclusions validation. |
| Multimodal PID | Sensory PID applies conditional PID with modality and instruction interventions. | Generic multimodal PID is not novel. Prisoma must add sequential and claim-matched experiments. |
| VLA monitoring | SAFE, Tri-Info, Hide-and-Seek, ActProbe, VLAConf, and related work span strong supervised, action, latent, uncertainty, and calibration baselines. | Freeze matched-access comparators and report cost, latency, calibration, and transport. |
| VLA mechanisms | CofactVLA, Häon et al., and related work use counterfactual masks, sparse features, and steering. | Intervention novelty is narrow. Require engagement, specificity, and closed-loop replication. |
| Executable policy | Same weights can behave differently under robot, controller, and normalization changes. WAV adds future-state and value latents. | Capture executable state and stratify transport by policy architecture and embodiment. |
| Censored scoring | Rindt, Kvamme–Borgan, Jonkers et al., and Alberge et al. separate prediction objects, score roles, censoring laws, identifiable regions, and competing events. | Freeze one exact H2 scoring contract. Do not infer validity from a metric label. |
| Effect validation | Curth et al. and causal-calibration work show that factual fit is not effect-model validation. | Use effect-specific H1-B endpoints and external or randomized calibration. |
| Attribution | Adebayo et al. and causal-abstraction work separate saliency appearance from intervention-grounded behavior. | Keep attribution exploratory and test target engagement. |
| World-action models | An exact-phrase query found 36 August 2026 submissions. A broader-name search found the same graphs under VLA, WLA, and latent-dynamics labels. ForeWAM and Rift are one-pass class C. CoWAM, World Action Planner, and optional WLA-0 are class E. CheckVLA is a class-D verifier. Efficient-WAM is class J in its released code. Several predictive branches disappear at deployment. | Classify executable information flow. Do not infer an action-conditioned query from a joint density or causal dynamics and planning from branding. |
| Action grounding | MiraBench and the world-model hallucination audit separate plausible video from action-sensitive dynamics. | Require randomized executed actions, proper scores, calibration, and failure-heavy tests. |
| Representation grounding | XEWorld, PhyLatent, and PSG-JEPA expose embodiment, physical-state, and counterfactual-dynamics failures. | Forward prediction and global non-collapse are not admission tests. |
| Execution management | HarnessWAM and TempoWAM add task memory, progress checks, recovery, and adaptive replanning around finite-horizon policies. | Measure these external mechanisms separately from the world branch. |
| Local execution | SmolVLA has an upstream MPS path. SLIM has a plausible MPS surface but no qualification. Efficient-WAM asserts CUDA before its attention fallback and uses float64/complex RoPE. JEPA-WAM is a CUDA-tested, pickle-based port candidate. LiLa-WAM has no language input or MPS path. | Qualify SmolVLA first. Gate SLIM next. Treat Efficient-WAM and JEPA-WAM as later code ports. Use LiLa-WAM as a separate predictive ablation. |

### Primary links refreshed

- Williams and Beer, [Nonnegative Decomposition of Multivariate
  Information](https://doi.org/10.3390/e12040488), 2010.
- Makkeh, Gutknecht, and Wibral, [Differentiable pointwise shared
  information](https://doi.org/10.1103/PhysRevE.103.032149), 2021.
- Ehrlich, Makkeh, and Wibral, [Continuous shared-exclusions PID for neural
  representations](https://doi.org/10.1103/PhysRevE.110.014115), 2024.
- Liardi et al., [The Mathematical Landscape of Partial Information
  Decomposition](https://arxiv.org/abs/2603.06678), 2026.
- Kraskov, Stögbauer, and Grassberger, [Estimating mutual
  information](https://doi.org/10.1103/PhysRevE.69.066138), 2004.
- Gao, Ver Steeg, and Galstyan, [Efficient MI estimation under strong
  dependence](https://arxiv.org/abs/1411.2003), 2015.
- Lyu, Clark, and Raviv, [Closed-Form Gaussian Estimators for Multi-Source
  PID](https://arxiv.org/abs/2605.09919), 2026.
- Fang et al., [Sensory PID](https://arxiv.org/abs/2606.00959), 2026.
- Gu et al., [SAFE](https://arxiv.org/abs/2506.09937), 2025.
- Yang et al., [Tri-Info](https://arxiv.org/abs/2606.19998), 2026.
- Panda et al., [What Do They See?](https://arxiv.org/abs/2607.16938), 2026.
- Zhang et al., [CofactVLA](https://arxiv.org/abs/2608.04396), 2026.
- Häon et al., [Mechanistic Interpretability for Steering
  VLA Models](https://proceedings.mlr.press/v305/haon25a.html), 2025.
- Tai, [Same Weights, Different Robot](https://arxiv.org/abs/2606.03724), 2026.
- Tang et al., [Shifting Uncertainty to Critical
  Moments](https://arxiv.org/abs/2603.18342), 2026.
- Yoon et al., [RoboBRIDGE](https://arxiv.org/abs/2607.27881), 2026.
- Li et al., [World-Value-Action Model](https://arxiv.org/abs/2604.14732), 2026.
- Rindt et al., [Survival Regression with Proper Scoring
  Rules](https://proceedings.mlr.press/v151/rindt22a.html), 2022.
- Kvamme and Borgan, [The Brier Score under Administrative
  Censoring](https://www.jmlr.org/papers/v24/19-1030.html), 2023.
- Jonkers et al., [Proper Scoring Rules for Right-Censored Survival
  Data](https://arxiv.org/abs/2606.06393), 2026.
- Alberge et al., [Proper scoring with competing
  risks](https://proceedings.mlr.press/v258/alberge25a.html), 2025.
- Adebayo et al., [Sanity Checks for Saliency
  Maps](https://proceedings.neurips.cc/paper/2018/hash/294a8ed24b1ad22ec2e7efea049b8737-Abstract.html),
  2018.
- Geiger et al., [Causal Abstractions of Neural
  Networks](https://arxiv.org/abs/2106.02997), 2021.
- Yan et al., [Flex-\(\pi\)](https://arxiv.org/abs/2608.10860), 2026.
- Wang et al., [SLIM-0.5B](https://arxiv.org/abs/2608.09771), 2026.
- Yang et al., [LiLa-WAM](https://arxiv.org/abs/2608.03701), 2026.
- Bao et al., [Surgical WAM](https://arxiv.org/abs/2608.11204), 2026.
- Liu et al., [CoWAM](https://arxiv.org/abs/2608.02578), 2026.
- Lou et al., [DynamicWAM](https://arxiv.org/abs/2608.00793), 2026.
- Wang et al., [FlowPilot](https://arxiv.org/abs/2608.00635), 2026.
- Zhao et al., [SG-WAM](https://arxiv.org/abs/2608.01397), 2026.
- Qiu et al., [Vid2WAM](https://arxiv.org/abs/2608.08558), 2026.
- Yuan et al., [DreamWAM](https://arxiv.org/abs/2608.04996), 2026.
- Yan et al., [Robust-WAM](https://arxiv.org/abs/2608.05903), 2026.
- Fan et al., [MobileWAM](https://arxiv.org/abs/2608.04657), 2026.
- Tang et al., [World Tokens](https://arxiv.org/abs/2608.09730), 2026.
- Lin et al., [JEPA-WAM](https://arxiv.org/abs/2608.09381), 2026.
- Peng et al., [FACT](https://arxiv.org/abs/2608.10232), 2026.
- Zhou et al., [\(\tau0\)-WM](https://arxiv.org/abs/2606.01027), 2026.
- Yang et al., [MiraBench](https://arxiv.org/abs/2605.29360), 2026.
- Hansen and Wang, [World-model hallucination](https://arxiv.org/abs/2606.27326), 2026.
- Chen et al., [XEWorld](https://arxiv.org/abs/2608.05799), 2026.
- Zeng et al., [PhyLatent](https://arxiv.org/abs/2608.05720), 2026.
- Yan et al., [Physical State Grounding for JEPA World Models](https://arxiv.org/abs/2608.06799),
  2026.
- Gu et al., [HarnessWAM](https://arxiv.org/abs/2608.09516), 2026.
- Ye et al., [TempoWAM](https://arxiv.org/abs/2608.09492), 2026.
- Pan et al., [World-to-Wrist VLA](https://arxiv.org/abs/2608.05369), 2026.
- Li et al., [Efficient-WAM](https://arxiv.org/abs/2606.10040), 2026.
- Lyu et al., [LDA-1B](https://arxiv.org/abs/2602.12215), 2026.
- Yang et al., [WLA-0](https://arxiv.org/abs/2606.05979), 2026.
- Wang et al., [RepWAM](https://arxiv.org/abs/2606.13674), 2026.
- Kairos Team et al., [Kairos](https://arxiv.org/abs/2606.16533), 2026.

The deployed-graph matrix, artifact pins, causal gate, and M4 decision are in
[`WORLD_ACTION_MODEL_FRONTIER.md`](WORLD_ACTION_MODEL_FRONTIER.md).

The full numbered bibliography and its role in the plan remain in `grandplan.md`.

## `pid-rs` review

The reviewed upstream head is 95 commits beyond the local pin. It adds substantial assurance and
API work. Prisoma checked, built all test targets, and ran its all-feature Rust consumer suite
against exact revision `722d3abeb922fc4119ecb9f92d7fedca096c9f77` in an isolated clean tree.
Current head `bbdfda40f0a49a2260b10eafdcb438fc61ae94e9` changes only assurance, workflow,
script, and prose surfaces relative to that tested revision. The consumed crates, Cargo files,
toolchain, and `pyproject.toml` are byte-identical. Since the earlier `00fce70d` check, executable verifier scripts, schemas, assurance
artifacts, prose, and tests changed. Three `pid-core` Rust source files changed only in
documentation or comments. No Cargo manifest, public Rust signature, or executable Rust statement
changed in that later interval.

```text
cargo +1.93.0 check --locked --workspace --all-features
cargo +1.93.0 test --locked --workspace --all-features --no-run
cargo +1.93.0 test --locked --workspace --all-features
```

All three commands passed. This is compatibility evidence for the compiled and tested Rust
consumer surface only.

The upstream categorical API now distinguishes `SxPointwiseAtom` from `SxAveragedAtom`. It also
replaces an ambiguous probability field with empirical count and probability fields. Those changes
are scientifically useful. They require a serialized-data and Python migration review.

The upstream release scope explicitly does not claim downstream Prisoma compatibility or VLA
application validity. Current-head CI run `31651702557` completed with all 45 jobs passing on
2026-08-13. Current-head CodeQL run `31651702504` also passed. The provider-CI result does not
replace the open consumer review, so the pin remains fixed. `PID_RS_HANDOFF.md` contains the
acceptance matrix and ready-to-send message.

## Documentation reconciliation

The current docset now uses the same boundaries:

- `grandplan.md` remains canonical at v12.5.
- `AGENTS.md` and `CLAUDE.md` carry the same stop rules.
- `README.md`, `ARCHITECTURE.md`, `DIAGRAMS.md`, `EXPERIMENTS.md`, and `pidsplatspecs.md`
  preserve the Agent Bridge, run-log, Rerun, and deferred-UI decisions.
- `LIMITATIONS.md`, `findings.md`, and `THESIS_EVIDENCE_INDEX.md` preserve the exact gate states.
- NCP and Engram prose separates read-only observation, pairing, transport, and attestation.
- Research ledgers preserve the unfrozen and non-promotable state.
- Historical and immutable intake remains historical and immutable.
- Generated views are changed only through their catalog and generator.

The audit also corrected “lean” to “low overhead.” It removed language that implied broad Rerun
validation, universal replay completeness, or natural pathway identification.

## What remains blocked

- M0 is not freeze-ready.
- EC1 lacks its final finite corpus and independent producer evidence.
- H1-A lacks real frozen-policy capture and a frozen claim decision.
- H1-B lacks a randomized closed-loop implementation.
- H2 lacks prospective real capture and a frozen primary contract.
- H3 has not passed population, measure, estimator, or application gates.
- H4 lacks a frozen target, selection rule, intervention, and error plan.
- SAFE re-export and rights review remain open.
- `pid-rs` adoption lacks behavior, schema, Python, fixture, package, and scientific-value evidence.
  Current-head provider CI is green, but it does not close those consumer-owned gates.

## Priority order

1. Review and freeze the M0 v2 scientific choices. Do not promote the v1 scaffold.
2. Produce one rights-approved SAFE capture with exact model, split, hook, and label receipts.
3. Close the generic paired instrumentation preflight on that real capture path.
4. Freeze either H1-A or H1-B as the first H1 claim. Do not blend them.
5. Freeze one aligned H2 primary scoring contract and exact comparator registry.
6. Validate low-dimensional estimators and retain all four PID gates.
7. Run a pilot without accessing the confirmatory holdout.
8. Add UI or ecosystem integration only when it closes a named scientific or operator decision.

## Final conclusion

Prisoma does not need a larger architecture. It needs fewer ambiguous claims and stronger frozen
decisions. The revised plan now makes that distinction explicit.

Retain PID as a conditional candidate. Treat non-PID monitors as mandatory comparators and exact
fallbacks. Promote no scientific result until the registered population, identification,
estimator, application, replication, and authenticated evidence requirements all pass.
