# Prisoma Experiment Protocols

This document is the executable runbook for current software proofs and the boundary for future
scientific experiments. [`grandplan.md`](grandplan.md) is canonical when the two documents differ.

The repository contains protocol groundwork. It does not contain a completed confirmatory study.

## 0. Scope

### 0.1 Unfrozen claim-template registry

| Claim | Question | Runnable today | Current boundary |
|---|---|---|---|
| EC1 | Can registered accepted events be reconstructed and replayed under frozen margins? | Local schema, replay, adapter, and fault fixtures | No external finite acceptance study |
| H1-A | Do pre-treatment diagnostics predict a paired frozen-snapshot response? | Common preflight and synthetic Protocol-A reference | Real policy, environment, dose, and estimand remain unfrozen |
| H1-B | Do pre-treatment diagnostics predict randomized closed-loop effect modification? | Specification only | No randomized implementation or evidence |
| H2 | Do pre-treatment features predict future failure under one scoring and censoring contract? | Synthetic fixed-horizon risk-estimator arithmetic reference | No prospective real capture or validated calibration |
| H3 | Does the full PID/abstention/exact-fallback policy add value on the target population? | Estimator diagnostics and bounded harness | Not eligible; high-dimensional path is NO-GO |
| H4 | Does availability diverge from response to one tested intervention? | Exploratory attribution reference | No availability or tested-response evidence |

The machine-readable source is
[`protocols/research_claim_registry_v1.json`](protocols/research_claim_registry_v1.json).

### 0.2 Executable runbook

Run the stages in this order:

| Stage | Command | Passing means | Passing does not mean |
|---|---|---|---|
| Repository quality | `just check` | The locked code, tests, docs, and generated notices agree | Scientific validity or release readiness |
| Formal abstractions | `just formal` | The stated SMT abstractions and countermodels hold | The implementation refines every abstraction |
| M0 governance | `just research-governance` | Current unfinished ledgers are structurally valid | Freeze readiness |
| EC1 groundwork | `just runlog-sidecars-proof` and `just runlog-rerun-proof` | Local fixtures reconstruct and convert | External replay validation |
| PID firebreak | `just firebreak` | Static baselines run without PID atoms or NCP | H1 or H2 evidence |
| H1 preflight | `just h1-preflight` | The checked fixture satisfies the common contract | A physical response estimate |
| H1 Protocol A | `just h1-protocol-a` | Synthetic scoring arithmetic is deterministic | H1-A or H1-B evidence |
| H2 reference | `just h2-reference` | Synthetic censoring and risk-estimator arithmetic is deterministic | H2 evidence or a proper observed-data score |
| H3 diagnostics | `just exp0-bin` | Current diagnostic verdicts reproduce | Atom or application validity |
| Offline harness | `just offline-harness` | The fixture passes the selected software path | Real-data eligibility |
| H4 reference | `just attribution-probe` | Bounded reference artifacts reconstruct | Causal faithfulness |

The two `--require-freeze-ready` governance modes must fail until the scientific obligations are
genuinely complete. Do not weaken those checks to make them pass.

### 0.3 Data-source rule

The harness is source-agnostic over one strict `(V,L,D,A)` contract. The reference adapter
implementation is `experiments/safe_adapter`. It is the candidate critical-path real-data producer.
Local simulator fixtures are cross-checks.

The NCP observer is optional and off the critical path. It is not a substitute for a real SAFE
capture. `D` is a declared source axis, not an assumed depth channel.

## 1. Evidence rules shared by every experiment

### 1.1 Canonical capture

Record every accepted request, response, observation, action, intervention, label, and artifact in
the canonical run log. A sidecar can add derived data but cannot replace the event stream.

### 1.2 Pre-treatment timing

Freeze the feature timestamp and lineage before the treatment, response, failure window, or outcome.
Reject any feature that can contain post-treatment information.

### 1.3 Independent experimental unit

Declare the unit before capture. Keep train, calibration, and held-out groups disjoint at that unit.
Do not treat frames from one episode as independent episodes.

### 1.4 Content binding

Bind each input, planning artifact, output, and executable revision by exact content identity.
Record the command, toolchain, seed, and active feature flags.

### 1.5 Failure preservation

When a readable input fails validation, publish the typed failed result and a schema-valid failed
run log when the protocol promises one. Never convert a failed gate into missing data.

## 2. EC1 replay groundwork

### 2.1 Current proof

Run:

```bash
just runlog-sidecars-proof
just runlog-rerun-proof
just runlog-bridge-export-rerun
```

These paths check schema-2 request and response closure, replay, manifests, sidecars, conversion,
and exact-snapshot export behavior. The export recipe enables `pid-sim/rerun-export`; default
bridge builds omit that viewer dependency and method.

### 2.2 Remaining acceptance study

Before EC1 can pass, freeze:

- The supported adapter set.
- The finite fault universe.
- The replay oracle and tolerances.
- One absolute sensitivity floor for each registered fault-adapter pair.
- Valid-case false-positive obligations.
- Uncertainty and multiplicity rules.
- A conventional external baseline.
- A structurally different external adapter.

An aggregate detection rate cannot rescue a failed required fault-adapter pair.

## 3. H1 intervention-response protocol

### 3.1 Common preflight

Run:

```bash
just h1-preflight
```

`pid-h1-preflight` validates the structural, timing, lineage, fold, reset, RNG, clone, and
instrumentation-noninterference contract. The valid input is a representative-mechanism structural
fixture only.

The result establishes no response estimate and no H1-A evidence. It cannot establish H1-B.

### 3.2 Protocol-A software reference

Run:

```bash
just h1-protocol-a
```

The reference exact-binds a passed preflight chain. It restores independent per-side clone state,
reverses treatment order, records zero RNG draws, and scores fixed-design and moderator models out
of outer fold.

It is a deterministic finite benchmark. It is not a subprocess audit, physical effect, stochastic
policy result, Protocol B implementation, or H1-A evidence.

### 3.3 Required real protocol freeze

Before real capture, freeze:

1. The policy, environment, task, and experimental unit.
2. The intervention family, dose, engagement check, and specificity controls.
3. The pre-treatment feature whitelist and instrumentation comparison.
4. Protocol A or Protocol B.
5. One estimand and its direction.
6. One proper response score or one effect-specific primary endpoint.
7. A matched-access comparator and positive useful margin.
8. A one-sided lower-confidence-bound decision under frozen uncertainty and multiplicity.
9. Calibration acceptance and failure rules or the full effect-validation stack.
10. The testing hierarchy and non-rescuable primary endpoint.
11. The finite-benchmark or directional-replication scope.
12. Holdout, access, and contamination controls.

Do not select treatment strength from held-out response behavior.

## 4. H2 future-failure protocol

### 4.1 Software reference

Run:

```bash
just h2-reference
```

The reference binds separate analysis-plan, event-ontology, feature-contract, and split-manifest
artifacts. It exercises:

- Task-family-held-out fitting.
- Grouped cross-fitting.
- Stratified reverse-KM IPCW.
- Horvitz–Thompson IPCW Brier risk-estimator arithmetic.
- Competing-event classification.
- Reliability bins.
- Frozen alarm and nondetection accounting.
- Declared-payoff utility.
- Explicit censoring abstentions.

This is PID-free protocol arithmetic. It is not prospective capture, calibrated prediction,
comparator superiority, warning benefit, or H2 evidence.

### 4.2 Required real protocol freeze

Before real capture, freeze:

1. The landmark and prediction horizon.
2. One primary failure definition and competing events.
3. Censoring strata and sensitivity analyses.
4. The independent episode and task-family units.
5. One primary scoring contract that aligns the prediction object, score, risk, censoring,
   identification, and uncertainty.
   A forecast-independent censoring-adjusted Brier construction can target scalar horizon risk
   under its exact conditional-censoring and positivity assumptions. A right-censored likelihood
   requires a full event-time-and-type law. Freeze the complete competing-event ontology.
6. Calibration intercept, slope, and uncertainty requirements.
7. Alarm threshold selection using training information only.
8. The matched-access comparator frontier.
9. The external or later-time validation split.

Retain nondetections in the utility denominator. Do not omit cases without an alarm.

## 5. H3 estimator and incremental-value protocol

### 5.1 Experiment 0

Run the current estimator diagnostic:

```bash
cargo run --locked \
  --manifest-path pid-rs/crates/pid-core/Cargo.toml \
  --features experimental-all \
  --bin exp0
```

The high-dimensional MI/coherence path is NO-GO. The default atom-measure gate is
`not_adjudicated`. The atom-estimator gate is `blocked`. The continuous application gate is
`blocked_not_application_validated`.

`--strict-gate` enforces the curated low-dimensional analytic-MI band. It only reports atoms.
Never report this as an atom-recovery or high-dimensional pass.

### 5.2 Offline `(V,L,D,A)` harness

Run the estimator-request firebreak first:

```bash
cargo run --locked -p pid-sim --features analysis --bin pid-offline-harness -- \
  --input crates/pid-sim/fixtures/offline_vlda_fixture.json \
  --pid-mode none \
  --summary-json outputs/offline_summary.json \
  --runlog outputs/offline_runlog.jsonl
```

Available modes are:

| Mode | Computation | Scientific identity |
|---|---|---|
| `none` (default) | Baselines and geometry only | No MI or PID request |
| `continuous` | KSG MI and continuous shared exclusions | Conditional, currently application-blocked |
| `discrete` | Equal-width quantization and Williams–Beer `I_min` | Different measure and estimand |
| `discrete-pls` | Train-fit PLS followed by quantized `I_min` | Different measure with supervised projection |

Discrete modes never rescue a failed continuous estimate. Do not pool outputs across modes.

### 5.3 Admission and diagnostics

The harness checks all work before expensive analysis. It binds:

- Raw input bytes and strict JSON structure.
- Sample and axis scalar counts.
- Metadata entries, nodes, depth, and UTF-8 size.
- Main, uncertainty, and total distance projections.
- Distance-coordinate projections.
- Dense-solver projections.
- Applied resource limits.

It also records static majority, 1-NN, centroid, and held-out logistic baselines when labels and
splits permit them. Geometry diagnostics do not replace any PID gate.

### 5.4 Activation rule

H3 remains inactive unless all four gates pass inside one frozen regime. One regime is one tuple
of measure, preprocessing, and estimator configuration. Its primary denominator is the complete
frozen target ledger. Each abstention uses the exact same-fold M1 output.

If H3 activates, use the frozen matched-access comparator registry and predeclared selection or
ensemble rule. Measure incremental predictive or decision value out of fold. Report warning,
abstention, and fallback rates. Define improvement so larger values are better. Require its
one-sided lower confidence bound to exceed the positive useful margin under the frozen
multiplicity rule. Noninferiority, equivalence, and eligible-only performance cannot establish
added value.

## 6. H4 tested-intervention-effect protocol

### 6.1 Current attribution reference

Run:

```bash
just attribution-probe
```

The current package implements a detached-attention, value-path-only epsilon-LRP baseline and
grad-times-input on a small reference model. The LRP variant is not AttnLRP.

The ranking-sensitivity gate uses content-bound, selection-disjoint, group-disjoint deletion tests.
It abstains on exact magnitude ties. One predeclared primary method can set the legacy boolean.

Passing this reference is not a causal, mechanistic, production-VLA, or transport-faithfulness
claim.

### 6.2 Required confirmatory H4 design

Freeze one target population, sampling or transport contract, baseline-defined region rule,
weight vector, probe, intervention construction, effect endpoint, margins, and independent unit
before capture. Use simultaneous availability-superiority and effect-equivalence inference. Bind
uncertainty for estimated target weights, exact fixed finite-target weights, and joint design
power. A second construction is required before the
claim generalizes beyond the primary construction. Attribution agreement alone cannot define
success. A small tested effect does not establish natural policy non-use.

Production work should use a separately pinned and validated AttnLRP implementation when that
method is appropriate.

## 7. Splits, labels, and adapter contract

Each sample requires nonempty finite vectors for `v`, `l`, `d`, and `a`. Sample identifiers must
be unique. Optional episode identifiers must be nonempty when present.

The strict held-out path expects:

- A boolean `success` label on every sample.
- A recognized `split` value on every sample.
- Both train and held-out samples.
- Required class coverage when enabled.
- Episode disjointness when enabled.
- Honest per-axis provenance markers when enabled.

The capture adapter must declare population support per axis. Observed cardinality, ties, or
geometry cannot infer that declaration.

## 8. Resource configuration

Defaults are conservative availability ceilings, not workload recommendations. Raise them only
with an explicit reviewed `--resource-limits-json` file.

The file is strict JSON, bounded to 64 KiB, and rejects unknown or zero fields. The applied values
and projected usage enter the report configuration hash.

The same configuration binds report contract `prisoma.offline_vlda.report/2`. Publication rejects
an unversioned or unknown report contract instead of inferring compatibility from JSON shape.

Optional uncertainty is admitted together with the main analysis. A large bootstrap or permutation
request fails before the main analysis begins. Schema 2 records the row topology and null
calibration. Current resamplers fail closed when mixed episode identifiers or multiple
non-singleton episodes would force a block or shift across boundaries. Multi-row block
subsampling and circular shifts require one episode with a strictly increasing canonical decimal
`metadata.sequence_index`. An `episode_id` alone does not establish order. Rows without episode
identifiers support only unit-block subsampling and full shuffle under a declared
row-exchangeability assumption. The CLI requires an
explicit block size for every bootstrap and circular-shift request. It requires an explicit scheme
for every permutation request. A combined request cannot mix an exchangeable-row bootstrap with a
serial surrogate, or a block bootstrap with a full shuffle. Restricted circular shifts yield
approximate surrogate tail fractions, not p-values. The temporal AR(1) screen is descriptive. Its
derived hints require the same sequence-index receipt and cannot select a block length by itself.

## 9. Reproducibility checklist

Before any evidentiary run:

- Freeze the exact source commit and clean worktree state.
- Initialize and verify the `pid-rs` gitlink.
- Record Rust, Python, `uv`, and platform versions.
- Record every dependency lockfile and active feature.
- Hash raw inputs before parsing.
- Record rights and source receipts for real data.
- Record seeds and every RNG draw contract.
- Keep train, calibration, and held-out units disjoint.
- Preserve failed and abstained outcomes.
- Validate the final run log independently.
- Keep optional sidecars content-bound to the run.
- Record resource limits and observed usage.

For release work, require exact pushed-commit CI evidence. Local success is not a post-push
attestation.

## 10. Optional studies

### 10.1 Reconstruction quality

[`GAUSS_MI_INTEGRATION.md`](GAUSS_MI_INTEGRATION.md) specifies a possible reconstruction-quality
covariate and active-view study. No implementation or validated observation law exists.

Do not implement the rejected GauSS-MI-weighted KSG sketch. It lacks a population functional and
consistency argument.

### 10.2 External world-model comparator

[`WORLD_WARP_INTEGRATION.md`](WORLD_WARP_INTEGRATION.md) specifies an optional external comparator.
It requires a pinned adapter, matched support, rights review, and content-bound outputs.

Generated scenes are not causal ground truth. This comparator is off the critical path.

Classify each candidate by its deployed graph before capture. Keep predictive co-training,
intended-future conditioning, coupled joint generation, action-conditioned prediction, and
candidate planning separate. Do not infer an operational conditional query by factorizing a joint
density.

Use this matched six-arm mechanism ladder if the study is activated:

1. train and deploy an action-only direct policy;
2. add a future loss but deploy the direct policy;
3. expose an intended future to action generation;
4. jointly sample future and action slots without clamping a candidate action;
5. add action-conditioned prediction and scoring, but force execution of the frozen direct-policy
   proposal; and
6. enable score-based selection among at least two otherwise identical proposals.

Match data, backbone, optimizer, parameters, compute, controller, and evaluation. Match arms 5 and
6 on proposals, predictions, scores, and compute. Validate the action-conditioned predictor with
randomized executed actions. Require arm 6 to pass a fixed-proposal decision-flip test. Log every
proposal, prediction, score, selection, controller conversion, and execution receipt. If execution
overlaps inference, also bind observation capture, inference start and finish, committed-prefix
indices, dispatch, and acknowledgement. Compare delay tails and alignment error before comparing
smoothing methods.

The full protocol and M4 qualification sequence are in the
[WAM frontier review](docs/audits/2026-08-12-first-principles/WORLD_ACTION_MODEL_FRONTIER.md).

### 10.3 Rendering and UI

Gaussian-splat rendering and a Tauri/SparkJS shell are deferred surfaces. They may visualize
evidence, but they cannot become control authorities or prerequisites for the core protocols.

## 11. Interpretation checklist

Before writing a result, answer each question:

1. Was the claim registered before access to the relevant outcomes?
2. Did the unit and inference cluster match the sampling, assignment, and interference assumptions?
3. Were all features available before treatment or landmark?
4. Did the population gate pass?
5. Did the measure gate pass?
6. Did the estimator gate pass?
7. Did the application gate pass?
8. Did the required baseline comparison pass out of fold?
9. Did every required sensitivity analysis retain its status?
10. Does the wording stay inside the claim-template registry's permitted language?

If any required answer is no or unknown, keep the claim blocked.
