# Prisoma Experiment Protocols

This document is the executable runbook for the world-model-first program and the preserved
diagnostic protocols. [`grandplan.md`](grandplan.md) is canonical when the two documents differ.
PID-related modes also follow
[`PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md`](PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md).

The repository contains protocol groundwork. It does not contain a completed confirmatory study.

## 0. Scope

### 0.1 Unfrozen claim-template registry

| Claim | Question | Runnable today | Current boundary |
|---|---|---|---|
| W1 | Does an action-conditioned model improve supported reference-state prediction? Does it also preserve action ranking under a separate secondary estimand? | Native decision-contract reference only | No learned M4 model, held-out dynamics, or frozen W1 study |
| W2 | Does the complete selector improve episode outcomes under one M4 budget? | Native multi-replan contract reference only | No randomized complete-policy comparison or resource result |
| W3 | Which dynamics, renderer, policy-response, or selector boundary explains failure? | Specification only | No matched mesh/3DGS and learned-model panel |
| EC1 | Can registered accepted events be reconstructed and replayed under frozen margins? | Local schema, replay, adapter, and fault fixtures | No external finite acceptance study |
| H1-A | Do pre-treatment diagnostics predict a paired frozen-snapshot response? | Common preflight and synthetic Protocol-A reference | Real policy, environment, dose, and estimand remain unfrozen |
| H1-B | Do pre-treatment diagnostics predict randomized closed-loop effect modification? | Specification only | No randomized implementation or evidence |
| H2 | Do pre-treatment features predict future failure under one scoring and censoring contract? | Synthetic fixed-horizon risk-estimator arithmetic reference | No prospective real capture or validated calibration |
| H3 | Does the full PID/abstention/exact same-fold M1 policy add value on the target population? | Estimator diagnostics and bounded harness | Not eligible; high-dimensional path is NO-GO |
| H4 | Does availability diverge from response to one tested intervention? | Exploratory attribution reference | No availability or tested-response evidence |

The machine-readable W1-W3 source is
[`protocols/world_model_claim_registry_v1.json`](protocols/world_model_claim_registry_v1.json).
The preserved EC1/H1-H4 source is
[`protocols/research_claim_registry_v1.json`](protocols/research_claim_registry_v1.json).

### 0.2 Executable runbook

Run the stages in this order:

| Stage | Command | Passing means | Passing does not mean |
|---|---|---|---|
| Repository quality | `just check` | The locked code, tests, docs, and generated notices agree | Scientific validity or release readiness |
| Formal abstractions | `just formal` | The stated SMT abstractions and countermodels hold | The implementation refines every abstraction |
| Diagnostic governance | `just research-governance` | Current unfinished EC1/H ledgers are structurally valid | WM0 or diagnostic freeze readiness |
| World-model contract | `just world-model-reference` | Exact fork, fixed pool, pre-label publication, reference labels, bridge execution, and replay reconstruct | Learned-model quality, physical truth, W1, or W2 |
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
implementation is `experiments/safe_adapter`. It is a candidate producer for the preserved
diagnostic program, not the primary W1/W2 path. Local simulator fixtures are cross-checks.

The NCP observer is optional and off the critical path. It is not a world-model adapter. Within
the preserved diagnostic family, it is also not a substitute for a real SAFE capture. `D` is a
declared source axis, not an assumed depth channel.

### 0.4 World-model-first protocol

#### 0.4.1 Native decision-contract reference

Run:

```bash
just world-model-reference
```

The reference learns a bounded affine transition from deterministic simulator rows. At every
decision, it:

1. captures one immutable simulator fork;
2. creates one ordered pool with at least two supported actions;
3. predicts and scores every candidate;
4. writes and flushes the complete commitment before oracle access;
5. executes only the selected candidate through the Agent Bridge;
6. commits the selected-execution receipt;
7. labels every candidate on an independent branch restored from the saved fork;
8. verifies the run log, bridge replay, `Flow_gt`, and decision semantics.

The typestate API exposes selected execution only after forecast publication succeeds. A second
typestate transition exposes oracle labeling only after the execution receipt exists. Execution
rejects a live session whose simulator state differs from the committed fork. A fixed-pool test
changes only the learned action response and requires the selected action to change. These are
software contract checks. The learner and reference labels use the same affine deterministic simulator.
This same-law fixture cannot provide independent model validation. It establishes no physical
validity, held-out forecast value, or closed-loop benefit.

Schema 2 has no neutral inline decision record. The reference carries forecast commitments and
execution receipts in strictly named `label_observed` compatibility envelopes. They are not
outcome labels. The verifier enforces their exact shape and order.

#### 0.4.2 W1 forecast study and secondary ranking study

Before capture, freeze:

- one reset-state population, one proposal-or-executed-action randomization level, and one
  supported design distribution;
- one declared later reference-state outcome and horizon, or one separately measured physical outcome;
- one proper primary score and positive useful margin;
- current-only, current-plus-action direct cost, kinematic, no-future, action-shuffled,
  future-shuffled, repeated-query, and proposal-headroom controls;
- one action-support distance and abstention rule;
- calibration, subgroup, dependence, and multiplicity rules;
- one M4 deadline, latency tails, memory cap, power proxy, and missed-deadline rule.

For the primary score, every method receives the same fork, randomized supported action assignment,
declared action fields, history, language, controller contract, and budget. Commit each prediction
and abstention before its shared
label. Keep proper-score improvement primary.

Ranking is secondary. Use either one precommitted ordered pool or one fully recorded adaptive
search. For an adaptive optimizer, record every proposal round, score, elite set, distribution
update, stopping state, and final recommendation. Separately score and commit a final CEM mean or
Nevergrad recommendation when it was not sampled. Treat Policy Top-1, inversion rate, and action
sensitivity as secondary unless one is promoted before holdout access.

#### 0.4.3 W2 complete-policy study

Randomize complete deployed policies across independent reset blocks. Include:

1. one frozen nominal policy;
2. a same-budget multiple-proposal direct policy;
3. a direct action-value or cost predictor;
4. simple dynamics or kinematic MPC;
5. the learned predictor with selection disabled;
6. the same predictor with frozen score-based selection.

Match proposal count, action support, observation history, controller, and deadline. Use one
episode-level primary endpoint and intention-to-treat analysis. Fork-local candidate-set regret is
a secondary same-pool selector diagnostic. It is not deployed-policy regret.

#### 0.4.4 W3 linked mesh/3DGS tomography

Use one content-bound ledger to link four panels:

- reference transition from the declared simulator;
- the exact state trajectory and camera rendered through mesh and 3DGS;
- the same frozen policy evaluated on those matched observations;
- the same candidate pool passed through the learned predictor and selector.

Rendering must never change collision geometry, dynamics, actions, state, timestamps, or camera.
Without paired real observations, mesh-versus-3DGS is an observation-substrate contrast, not a
rendering-error estimate. Report pixel/feature, immediate policy, closed-loop policy, forecast, and
selection effects as different estimands. Do not add them into one error budget.

Bind the authoritative state, every dynamic body and joint, contact state, controller state, and
body/link-to-representation map. Bind camera intrinsics, distortion, crop, resolution, clipping,
pose, exposure, color transform, tone mapping, shutter, motion blur, frame time, and synchronization.
Reset policy memory, KV cache, history, and randomness for matched immediate queries. A failed
identity check invalidates the treatment. A valid zero effect is a reportable negative result.

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
| `continuous` | KSG MI and the Ehrlich continuous shared-exclusions functional through its named kNN route | Distinct continuous functional and estimator; application-blocked |
| `categorical-sx` | Fitted equal-width quantization, then empirical-PMF MGW SxPID2 | New categorical variables; informative, misinformative, and net atoms |
| `categorical-sx-pls` | Scope-fit PLS, fitted quantization, then empirical-PMF MGW SxPID2 | Supervised same-row diagnostic with a typed warning; the split screen uses train rows only and never scores held-out rows |

The categorical modes call pinned
`pid_core::stable::quantized::fitted_quantized_sxpid2_with_budget`. They do not call `I_min` or
BROJA. They never rescue a failed continuous estimate. Do not pool fitted-categorical and
continuous outputs, even though both belong to the shared-exclusions literature.

Every `categorical-sx-pls` estimate is `produced_with_warning`. The PLS transform uses the same
target rows that the categorical screen analyzes. This route measures a fitted empirical law and
selection inflation. It is not a held-out estimate or an inferential high-dimensional escape hatch.

Do not emit a `wibral_lineage` result identity. Each current result must name its full-team
functional, exact cumulative or Möbius-inverted quantity, antichain coordinate, component,
aggregation scope, estimator revision, source and target variables, transform, units, and gate verdict.
A future direct-law or objective result must also bind evaluation kind, input-law kind,
aggregation scope, and composition. An objective instance additionally binds every input-quantity
identity, the complete coefficient vector, non-PID terms, sign convention, and optimization
direction. MGW categorical shared exclusions, continuous Ehrlich shared exclusions, a statistic
that estimates either, a direct declared-law evaluator, and an infomorphic training objective are
related but non-substitutable. Transfer requires a mapping theorem.
A net atom, informative atom, and misinformative atom also remain separate coordinates. Never
compare or pool them by matching the informal label alone.

Apply the complete method and mathematics review in
[`PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md`](PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md)
before adding a mode, sensitivity branch, or thesis claim. A method that is mathematically valid
but inapplicable to H3 remains a preserved research object. It must not be renamed as an existing
PID or used as an automatic fallback.

### 5.3 Admission and diagnostics

The harness checks all work before expensive analysis. It binds:

- Raw input bytes and strict JSON structure.
- Sample and axis scalar counts.
- Metadata entries, nodes, depth, and UTF-8 size.
- Main, uncertainty, and total distance projections.
- Distance-coordinate projections.
- Dense-solver projections.
- Fitted-quantization and categorical-SxPID operation projections.
- Applied resource limits.

It also records static majority, 1-NN, centroid, and held-out logistic baselines when labels and
splits permit them. Categorical results retain the estimator's occupancy, singleton, low-count,
coverage-indicator, and unseen-state caveat fields. These do not prove population support.
Geometry diagnostics do not replace any PID gate.

### 5.4 Activation rule

H3 remains inactive unless all four gates pass inside one frozen regime. One regime is one tuple
of measure, preprocessing, and estimator configuration. Its primary denominator is the complete
frozen target ledger. Each abstention uses the exact same-fold M1 output.

Freeze the source–target ancestry before H3 can activate. Freeze a target-specific prediction
landmark before target realization or availability. Each source must exist at that landmark and
must not contain its target. Reject an action-conditioned state as a source when its
input is the exact candidate action used as the PID target. Cross-fitting does not repair this
target injection. A downstream command, later declared reference-state outcome, or separately
measured physical outcome is eligible only when the matched baseline receives the same proposal.
Command or simulator-state prediction is not physical forecast validity.

If H3 activates, use the frozen matched-access comparator registry and predeclared selection or
ensemble rule. Measure incremental predictive or decision value out of fold. Report warning,
abstention, and same-fold M1 substitution rates. Define improvement so larger values are better. Require its
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

The capture adapter must declare population support per axis. Each continuous MI/PID tuple also
needs a separate `continuous_tuple_support` declaration. It asserts that all required marginal and
joint laws are regular, full-dimensional, absolutely continuous, and finite-information.
Per-axis continuity does not imply that tuple contract. Observed values cannot infer either one.

## 8. Resource configuration

Defaults are conservative availability ceilings, not workload recommendations. Raise them only
with an explicit reviewed `--resource-limits-json` file.

The file is strict JSON, bounded to 64 KiB, and rejects unknown or zero fields. The applied values
and projected usage enter the report configuration hash.

The same configuration binds report contract `prisoma.offline_vlda.report/5`. Publication rejects
an unversioned or unknown report contract instead of inferring compatibility from JSON shape.
Publication also verifies a private process-local digest over all serialized report fields. A
deserialized summary has no such authority. Rerun the analysis before publishing from saved data.

Optional uncertainty is admitted together with the main analysis. A large bootstrap or permutation
request fails before the main analysis begins. Schema 3 records tuple support, row topology, and null
calibration. Current resamplers fail closed when mixed episode identifiers or multiple
non-singleton episodes would force a block or shift across boundaries. Multi-row block
subsampling and circular shifts require one episode with a strictly increasing canonical decimal
`metadata.sequence_index`. An `episode_id` alone does not establish order. Rows without episode
identifiers support only unit-block subsampling and full shuffle under a declared
row-exchangeability assumption. The CLI requires an
explicit block size for every bootstrap and circular-shift request. It requires an explicit scheme
for every permutation request. A combined request cannot mix an exchangeable-row bootstrap with a
serial surrogate, or a block bootstrap with a full shuffle. Restricted circular shifts yield
approximate surrogate tail fractions, not p-values. The temporal screen computes
within-unit-step-run Pearson lag-1 correlations. Its axis means exclude columns that are undefined
after centering, including constant columns. They report their defined-dimension coverage. The report
emits no lag value without episode identities. Every non-singleton segment also needs a strict
canonical `sequence_index` receipt. Only adjacent rows whose index advances by one contribute. The
report counts excluded gaps. It centers both lagged vectors inside each contiguous run before
pooling residual products. A run needs at least three lag pairs. Two pairs force Pearson
correlation to positive or negative one. It reports admitted and correlation-eligible pair counts.
It derives no estimator sample size or block length. Justify either quantity independently.

A future group-aware schedule is only a schedule substrate. Whole-group sampling with replacement
duplicates every numeric row in a repeated episode. Occurrence IDs preserve provenance but do not
remove the coordinate ties that the pinned continuous KSG/Ehrlich route rejects. The first
continuous episode route must therefore use a separately justified without-replacement group
subsampling diagnostic, per-group statistic, or new weighted/cluster-aware estimator. It must not
claim bootstrap confidence-interval calibration. Categorical callbacks may admit repeated groups
under a separately frozen sampling estimand.

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

## 10. Primary world-model and optional studies

### 10.1 Reconstruction quality

[`GAUSS_MI_INTEGRATION.md`](GAUSS_MI_INTEGRATION.md) specifies a possible reconstruction-quality
covariate and active-view study. No implementation or validated observation law exists.

Do not implement the rejected GauSS-MI-weighted KSG sketch. It lacks a population functional and
consistency argument.

### 10.2 Learned world model and linked fidelity study

The native exact-fork reference is implemented. W1 and W2 require a pinned learned-model adapter,
matched action support, rights review, content-bound outputs, and measured M4 resource receipts.
The first port target is the compact LeWorldModel PushT CEM path frozen in `grandplan.md`.
Reproduce its 30-round, 300-sample, 30-elite, horizon-five, five-action-block search before
freezing a reduced-budget M4 arm. Its end-to-end upstream evaluator hard-codes CUDA and has no
verified MPS path. One exact-package synthetic probe ran direct prediction, latent rollout, and the
full CEM loop on MPS. This does not establish preprocessing, PushT, closed-loop, or planner parity.
The one-seed independent TwoRoom reproduction does not test PushT or M4. It found unconfigured
pipeline conventions, conflicting evaluation settings, and a separation between one-step error
and long-horizon planning. Audit the analogous PushT fields across paper, configuration, and code.
Freeze each unresolved feasible reading before outcomes. Fit action scaling on training rows only. Inverse-transform every proposal and enforce the frozen
raw-action support rule. It remains an MPS candidate until all gates pass. Use JEPA-WM as the
second planning benchmark after a separate noncommercial-rights decision.

W3 links identical authoritative state trajectories and cameras across mesh and 3DGS observations.
It binds body/link representations, camera and photometric parameters, frame timing, asset lineage,
and policy reset state. It links the same fork and action set across learned and reference dynamics.
It measures immediate frozen-policy response and downstream policy effects in separate designs.
Rendering must not change collision geometry, dynamics, reset state, or action execution.

[`WORLD_WARP_INTEGRATION.md`](WORLD_WARP_INTEGRATION.md) remains an optional legacy comparator
specification. It requires a pinned adapter, matched support, rights review, and content-bound
outputs.

Generated scenes are not causal ground truth. WorldWarp is off the critical path.

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

Do not screen \(D \to A^\pi\) when \(D\) was computed from that exact proposal. A controller or
executed-command target is eligible only with the same proposal in the matched baseline and cannot
establish physical forecast validity. Use a separately measured physical outcome for that claim
and give the matched baseline the same proposal. Alternatively, keep the forecast inside the frozen class-D/E
comparison. Bind each tensor's maximum ancestor time to the target-specific prediction landmark.

The full protocol and M4 qualification sequence are in the
[WAM frontier review](docs/audits/2026-08-12-first-principles/WORLD_ACTION_MODEL_FRONTIER.md).

### 10.3 Rendering and UI

The matched mesh-versus-3DGS treatment is planned W3 research. A generic Gaussian-splat runtime
and the Tauri/SparkJS shell remain deferred. Rendering and UI may consume evidence, but they cannot
become control authorities or prerequisites for the exact-fork decision path.

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
