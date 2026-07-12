# Second-round deep scientific and technical review of `prisoma/grandplan.md`

**Review date:** 2026-07-12
**Repository snapshot:** `sepahead/prisoma@64bd881248463e7142d022bb95a5850bcf8fced2`
**First replacement:** `grandplan_v11.md`
**Second-round canonical candidate:** `grandplan_v12_5.md`
**Scientific cut:** July 12, 2026
**Review stance:** adversarial, claim-matched, falsification-first, and independent of whether PID succeeds

---

# 1. Executive verdict

The v11 replacement made the decisive conceptual correction: Prisoma should be a programme for intervention-grounded diagnosis of sequential embodied policies, with PID as a conditional candidate rather than the thesis premise. The second round found that v11 still needed substantial repair in four areas.

First, its intervention hypothesis could still be read as if paired software replays, randomized closed-loop experiments, and individual physical treatment effects were interchangeable. They are not. `grandplan_v12_5.md` now separates a **paired frozen-snapshot algorithmic-response protocol** from a **randomized closed-loop effect-modification protocol**, assigns each a different estimand and score, and prohibits upgrading one into the other.

Second, the original H1 validation path risked selecting heterogeneous-treatment-effect models through factual-outcome prediction or correlations with noisy pseudo-individual effects. A model can predict outcomes well while estimating effect heterogeneity badly, and ordinary parallel-arm experiments do not reveal both physical potential outcomes for a unit. v12.5 therefore requires effect-specific outer-fold criteria: cross-fitted R-loss or doubly robust effect loss, causal calibration using training-defined groups and held-out randomized contrasts, a prespecified prioritization statistic, and policy value or regret. Factual-outcome loss is secondary.

Third, the prospective failure-prediction baseline frontier changed quickly in 2025–2026. v11 had Tri-Info and broad uncertainty baselines, but omitted or underweighted several direct competitors. v12.5 now treats SAFE, Hide-and-Seek, ActProbe, Rewind-IL/TIDE, architecture-stratified black-box action monitoring, VLAConf, perturbation-induced disagreement, activation probes, Foresight, and temporal-difference calibration as mandatory applicable comparators. It also fixes alarm-policy underdefinition, lead-time selection bias, censoring, competing risks, conformal validity under shift, and process-level safety outcomes.

Fourth, the surrounding `sepahead` repository ecosystem required an evidence audit. At the reviewed snapshot, only `pid-rs` and NCP are direct repository relationships. Galadriel, Crebain, Manwe, Engram, Melkor, WorldWarp, GauSS-MI, asset repositories, visualization projects, and information-theoretic lineage repositories are scientifically useful but not presently integrated with Prisoma. v12.5 adds an E0–E5 evidence ladder, a dependency firebreak, an adapter promotion contract, and explicit project boundaries.

A further final pass found and removed a duplicate ForesightSafety-VLA reference. The canonical candidate now has 112 unique, defined, and cited references.

## Final thesis framing

The strongest defensible thesis is:

> **Prisoma provides auditable experiment semantics for testing whether pre-treatment diagnostics of embodied policies predict frozen-snapshot algorithmic sensitivity, randomized closed-loop effect modification, and future failure beyond strong baselines; PID is thesis-central only if its measure, estimator, application regime, and incremental utility pass preregistered gates.**

This framing remains publishable under all scientifically plausible PID outcomes:

- PID is invalid or unstable in the intended regime;
- PID is estimable but not causally informative;
- PID is predictive but adds nothing beyond simpler signals;
- only a narrow discrete or low-dimensional PID regime survives;
- the infrastructure and availability–use gap are the strongest contributions.

# 2. Major second-round corrections

## 2.1 Paired algorithmic sensitivity and closed-loop causal effects are different estimands

A software clone can expose both policy computations for one frozen baseline state. It can therefore make a declared algorithmic contrast directly computable, subject to immutable-state, cache, recurrent-memory, evaluation-order, and random-number-coupling assumptions. It does not expose both physical trajectories that would have occurred for one embodied episode under two treatments.

A randomized closed-loop experiment instead identifies average effects and, with stronger modeling and overlap conditions, conditional effect modification in a target population. It generally does not identify unit-level physical treatment effects.

v12.5 names these:

- **Protocol A:** paired frozen-snapshot algorithmic response;
- **Protocol B:** randomized closed-loop response.

Each protocol has its own unit, target population, treatment contract, outcome family, analysis, endpoint, and permitted language. Protocol A may support “predicts algorithmic sensitivity under the declared snapshot and coupling.” Closed-loop or physical-effect language requires Protocol B.

## 2.2 The moderator must be genuinely pre-treatment

A treatment moderator cannot be computed from the treated forward pass, treatment engagement, downstream controller response, future frame, or outcome. Instrumentation itself can alter timing or output and therefore also requires a noninterference check.

v12.5 restricts primary moderators to untreated baseline variables transformed using outer-training data only. It requires an automated lineage rule, a pre-treatment whitelist, and instrumented-versus-uninstrumented equivalence tolerances.

This matters acutely for PID. A global dataset atom is not an episode moderator. An episode-local information feature needs a train-reference population, a mathematically defined local score or explicitly labelled surrogate, cross-fitting, aggregation, oracle/null validation, and an eligibility verdict.

## 2.3 Heterogeneous-effect model selection now uses causal criteria

The earlier plan could be read as validating effect prediction through ordinary outcome fit, coefficient significance, or a correlation with a same-data per-case effect estimate. Those are inadequate.

v12.5 requires a frozen stack appropriate to randomized heterogeneity:

1. cross-fitted R-loss or doubly robust effect-prediction loss;
2. causal calibration using bins formed in training data and treatment contrasts estimated in held-out randomized data;
3. a rank-weighted average treatment effect or equivalent prioritization statistic when ranking is the use case;
4. policy value or regret under known assignment probabilities;
5. randomization-based or cluster-aware uncertainty for the global no-effect-modification null.

The plan explicitly states that factual-outcome proper loss is a secondary nuisance/outcome-model diagnostic. A model that predicts outcomes but fails effect-specific checks does not pass H1.

## 2.4 Prospective monitoring now reflects the 2026 comparator frontier

Failure prediction cannot be evaluated only against entropy, OOD distance, or a generic learned baseline. The competitive frontier includes methods with different access and supervision:

- supervised internal-state detection;
- coarsely supervised temporal failure localization;
- action-chunk magnitude, smoothness, and temporal-consistency probes;
- inter-chunk discrepancy and state respawning;
- architecture-specific reversal, jerk, momentum, coherence, and stall features;
- one-class internal-representation confidence;
- perturbation-induced action disagreement;
- activation warning probes;
- action-conditioned world-model latents;
- information-theoretic signals;
- sequential success calibration and conformal thresholds.

v12.5 requires matched information access, labels, action resampling, external-model use, compute, and latency. When these cannot be matched, the output is a cost–accuracy–timeliness Pareto frontier, not a single league table.

## 2.5 Alarm evaluation is now operationally well defined

Repeated risk scores do not determine alarms without a policy. Threshold, persistence or debounce, refractory period, event-matching window, reset behavior, and missing-score handling all affect false alarms, detection, and lead time. Tuning these on test data would invalidate the comparison.

v12.5 freezes the alarm policy in training data and reports:

- event-level detection at fixed false-alarm burden;
- alarms per episode or operating hour;
- lead time with undetected failures retained explicitly;
- decision utility under intervention latency and fallback capacity;
- conformal coverage, set size or abstention, and subgroup/task coverage where applicable.

Conditional lead time among detected failures alone is identified as selection-biased.

## 2.6 Safety outcomes are no longer collapsed into binary success

A successful episode can be unsafe; a failed episode can have little or severe risk exposure. v12.5 requires cumulative safety cost, risk-exposure duration, and safe-success/unsafe-success/safe-failure/unsafe-failure quadrants where applicable. Diagnostics remain monitoring evidence, not certification.

## 2.7 `pid-rs` status is reconciled without flattening evidence

Two statements are simultaneously true:

- the Prisoma-pinned `pid-rs` revision contains real low-dimensional oracle and discrete-reference evidence but still documented pending external continuous cross-validation;
- a later `pid-rs` main revision adds a reproducible public `csxpid` fixture and stronger support/provenance contracts.

The latter does not automatically change the reviewed Prisoma dependency. v12.5 treats the later revision as a candidate upgrade requiring a scientific migration report. Neither revision establishes high-dimensional dependent VLA application validity.

## 2.8 Repository relationships are now evidence-graded

The plan previously risked allowing a profile diagram, sibling status, or integration specification to imply an implemented ecosystem. v12.5 introduces:

- E0 intention;
- E1 interface specification;
- E2 immutable dependency;
- E3 build-tested adapter;
- E4 end-to-end scientific conformance;
- E5 independent replication.

At the reviewed snapshot:

- `pid-rs`: direct E2 dependency;
- NCP: direct optional E2 observer dependency;
- all other inspected repositories: E0/E1 candidates or lineage unless a narrower fixture-specific relationship is documented.

# 3. Twenty-lens adversarial review

## Lens 1 — estimand clarity

**Failure risk.** Terms such as “action,” “information,” “sensitivity,” “failure,” and “robustness” can hide different variables and populations.

**v12.5 repair.** The plan separates policy distribution, proposed action, controller state, executed command, and physical outcome. It distinguishes observational information, paired algorithmic response, randomized causal effect, and prospective prediction. Every analysis requires an estimand table with unit, target population, treatment, outcome, time zero, horizon, sampling mechanism, and missingness.

**Residual risk.** Real VLA interfaces may expose only decoded actions, making policy-distribution targets unavailable. This must reduce claim scope rather than motivate invented proxies.

## Lens 2 — causal identification

**Failure risk.** Randomized assignment can be undermined by noncompliance, carryover, interference, treatment versions, post-treatment adjustment, or shared simulator state.

**v12.5 repair.** ITT is primary; treatment attempt and receipt are logged separately. Reset blocks, interference clusters, assignment probabilities, and manipulation/placebo controls are explicit. Post-treatment variables cannot enter baseline moderation.

**Residual risk.** Internal activation interventions may be off-support or affect several pathways. Randomization identifies the implemented perturbation, not a unique natural mechanism.

## Lens 3 — temporal and sequential validity

**Failure risk.** Overlapping windows, recurrent memory, future normalization, repeated landmarks, and action chunks create leakage and dependence.

**v12.5 repair.** All landmarks from an episode or persistent world state stay in one outer fold. Time zero, feature cutoff, horizon, competing events, censoring, memory reset, and controller state are explicit. Sequential calibration is a separate comparator.

**Residual risk.** Long-horizon policy/environment feedback may make static landmark models inadequate. Time-varying treatments may require longitudinal causal methods beyond the minimum plan.

## Lens 4 — intervention fidelity

**Failure risk.** Perturbations can alter task difficulty, timing, numerical stability, or support rather than the claimed pathway.

**v12.5 repair.** Every intervention has a unique version, dose, target, placebo, positive control, manipulation checks, receipt, specificity tests, and support diagnostics. Clone order and worker placement are randomized; mutable state is reset and hashed.

**Residual risk.** There may be no nontrivial dose that changes the intended mechanism without broad degradation. The plan correctly treats that as a stop condition.

## Lens 5 — measurement validity

**Failure risk.** “Vision,” “language,” and “dynamics” can be arbitrary tensors; safety and progress labels can be noisy; source timestamps and transformations can be wrong.

**v12.5 repair.** Source variables are selected by documented semantics, not labels. The event model records raw and transformed provenance, clocks, transforms, policy/controller/execution stages, and outcome definitions. Failure ontology and process-level safety costs are preregistered.

**Residual risk.** Some internal states lack stable cross-model meaning. Cross-model claims must be made at functional interfaces or model-specific strata.

## Lens 6 — PID axioms and measure choice

**Failure risk.** PID is underdetermined; different measures can disagree; atom labels can acquire unjustified universal semantics.

**v12.5 repair.** Every result names its measure, source/target population, and estimator. Full three-source continuous PID is exploratory. Cross-measure disagreement is reported, not selected away. BROJA lineage is not treated as shared-exclusions validation.

**Residual risk.** The chosen measure may still lack properties desired for a specific claim. Measure-property tests remain distinct from estimator recovery.

## Lens 7 — finite-sample estimator validity

**Failure risk.** Low-dimensional Gaussian or discrete fixtures do not characterize biased, dependent, anisotropic, high-dimensional VLA embeddings.

**v12.5 repair.** Validation spans analytic/numerical oracles, nuisance geometry, dependence, mechanism discrimination, sample size, dimension, preprocessing, and abstention. Population, measure, estimator, and application verdicts are separate.

**Residual risk.** A realistic oracle for high-dimensional dependent shared-exclusions may be unavailable. In that case, the application gate remains NOT ESTABLISHED rather than inferred from geometry.

## Lens 8 — high-dimensional geometry

**Failure risk.** KNN estimators can fail under concentration, duplicates, anisotropy, manifolds, mixed dimensions, or aggressive projection.

**v12.5 repair.** Geometry measures are warnings/stratifiers, not proof. Projection is train-fitted, hashed, stability-tested, and part of the estimand. Mixed-dimensional continuous three-source analysis is exploratory.

**Residual risk.** A projection selected for action prediction can distort the information decomposition. Alternative projections and an explicit scientific target are required.

## Lens 9 — uncertainty and multiplicity

**Failure risk.** Modalities, layers, doses, targets, horizons, metrics, estimators, and preprocessing create a vast researcher-degree-of-freedom surface.

**v12.5 repair.** Only three confirmatory scientific claims remain. Families, hierarchy, useful margins, smallest effects of interest, and outer holdouts are frozen. Cluster/task-family uncertainty replaces frame-level pseudo-replication.

**Residual risk.** Small numbers of task families can make asymptotic cluster inference fragile. Randomization inference, family-level sensitivity, and transparent counts are necessary.

## Lens 10 — predictive validation and calibration

**Failure risk.** Random frame splits inflate performance; ROC AUC hides prevalence and calibration; conformal guarantees fail under shift.

**v12.5 repair.** Task-family, temporal, or external validation is mandatory. Proper scores, calibration, target-prevalence reporting, alarm policy, false-alarm burden, lead time with nondetections, and decision utility are required. Shifted conformal coverage is described as empirical unless assumptions are checked.

**Residual risk.** External task families may be too few or too different to support stable recalibration. The plan should publish uncertainty and abstain from deployment claims.

## Lens 11 — heterogeneous-treatment-effect model selection

**Failure risk.** Factual outcome fit and noisy pseudo-ITE correlation are poor effect-model selectors.

**v12.5 repair.** Effect-specific R/DR loss, causal calibration, prioritization, and policy value/regret are primary. Models use identical outer splits and comparable tuning budgets.

**Residual risk.** Effect metrics themselves can disagree and be noisy. The plan appropriately freezes a stack and treats disagreement as evidence rather than selecting the best after inspection.

## Lens 12 — benchmark and transport validity

**Failure risk.** A finite benchmark may not support claims about new tasks, policies, embodiments, or physical systems.

**v12.5 repair.** Finite-population, task-family superpopulation, and transport targets are named separately. Task diversity, near-duplicate audits, and policy/embodiment transport stages are explicit.

**Residual risk.** Dataset construction by one lab can encode unrecognized shortcuts. External adapters and later-time tests are essential.

## Lens 13 — embodied dynamics and controller separation

**Failure risk.** A policy-output change may disappear in a controller, while controller or environment dynamics may create outcome differences unrelated to policy information.

**v12.5 repair.** Policy proposal, controller transformation, executed action, state transition, and outcome are separate event types and outcome families. Claims cannot silently move between them.

**Residual risk.** Proprietary controllers may be opaque. The plan must then limit itself to observed execution and outcome effects.

## Lens 14 — safety and human factors

**Failure risk.** Diagnostic scores can be misread as causal truth or safety certification; alarm burden and operator latency can make a statistically good monitor unusable.

**v12.5 repair.** Visualizations expose gate status, uncertainty, provenance, and noninterpretability warnings. Decision utility includes fallback capacity and latency. Process-level safety outcomes replace binary success alone.

**Residual risk.** Human-in-the-loop studies are deferred. Any operator-facing claim requires separate usability and response-policy evaluation.

## Lens 15 — leakage and contamination

**Failure risk.** Global transforms, full-episode statistics, near-duplicate tasks, shared random seeds, model pretraining overlap, and sibling repositories can leak holdout information.

**v12.5 repair.** Train-only fitting, outer-fold grouping, near-duplicate audits, source lineage, holdout access logs, and producer firewalls are explicit. Candidate producers cannot see outcomes or future schedules.

**Residual risk.** Pretraining contamination may be unknowable for some policies. This must be reported as a limitation and tested through controlled novel factors where possible.

## Lens 16 — infrastructure and replay

**Failure risk.** A logger or viewer can be mistaken for scientific infrastructure without showing that it changes result reliability.

**v12.5 repair.** EC1 is externally benchmarked on timestamp alignment, dropped events, assignment integrity, replay fidelity, provenance completeness, adapter effort, and abstention. Replay levels distinguish exact events, deterministic computation, bounded derived outcomes, and semantic replay.

**Residual risk.** Exact physical replay is impossible. The plan correctly avoids promising it and records tolerances and stochasticity.

## Lens 17 — software security and supply chain

**Failure risk.** Agent bridges, WebSockets, NCP action planes, model artifacts, generated assets, and sibling checkouts can introduce authority or provenance failures.

**v12.5 repair.** Least privilege, read-only observer boundaries, secure/open transport profiles, content addressing, SBOMs, license checks, malformed-input tests, and dependency firebreaks are required.

**Residual risk.** Security evidence is repository- and deployment-specific. No diagnostic system should be called secure because its data types are safe or its CI passes.

## Lens 18 — interoperability and ecosystem evidence

**Failure risk.** Shared ownership, diagrams, or build compatibility can be presented as integration.

**v12.5 repair.** E0–E5 levels bind wording to evidence. Only `pid-rs` and NCP are direct at the snapshot. Every candidate adapter must document schema, time, frames, assignments, security, faults, replay, and estimand preservation.

**Residual risk.** Public-surface review cannot rule out private work. The audit uses bounded negative language.

## Lens 19 — novelty and prior art

**Failure risk.** “First PID for multimodal/VLA” and “first VLA diagnostic” claims are no longer defensible.

**v12.5 repair.** Novelty is the combination of availability–use–effect separation, paired and randomized reference criteria, estimator abstention, prospective external validation, portable experiment semantics, and a fair incremental PID test. Search claims are dated and require rerunning before submission.

**Residual risk.** Rapid 2026 publication velocity may narrow the gap further. The thesis should emphasize executed benchmark evidence, not priority rhetoric.

## Lens 20 — feasibility and falsifiability

**Failure risk.** A project spanning PID theory, VLA interpretability, world models, custom simulation, 3D assets, security, and multiple sibling integrations can become unfinishable.

**v12.5 repair.** The minimum thesis is Papers A and B plus conditional Paper C. NCP, world models, custom UIs, Gaussian splatting, large asset domains, NEST/Engram, and three-source PID are optional. Kill rules preserve a PID-free path.

**Residual risk.** Even the minimum programme is substantial. Milestone gates must stop attractive side work until one real policy–environment capture, one external adapter, and one locked randomized experiment exist.

# 4. Repository ecosystem findings that change the science plan

## 4.1 Direct dependencies

`pid-rs` and NCP receive revision-specific provenance, migration, security, and noninterference requirements. The core must work without NCP and H1/H2 must work without PID.

## 4.2 Candidate comparators

Galadriel is valuable because it offers non-PID consistency evidence, but shared `pid-rs` use prevents an independence claim. A fair comparison should use common data, equal latency/information budgets, and separate shared versus independent components.

## 4.3 Candidate producers

Crebain, Manwe, Melkor, Cobot Atlas, and eventually a NEST/Engram source can stress different parts of the contract. They should enter one at a time through E4 conformance, not as a simultaneous ecosystem integration milestone.

## 4.4 Pre-implementation concepts

WorldWarp and GauSS-MI are explicitly outside the critical path. Generated counterfactual scenes need support and identity-preservation tests; weighted information estimators need a mathematical estimand and oracle gate.

## 4.5 Scientific lineage

The ReScience and BROJA repositories can seed controlled fixtures and cross-measure studies. They are not present validation and must not be presented as independent evidence for shared-exclusions.

# 5. Residual scientific risks after v12.5

## 5.1 No estimator gate can guarantee application truth

Synthetic families can falsify an estimator and bound support; they cannot prove that a real VLA embedding lies in a benign regime. Application eligibility therefore remains an abstention decision under uncertainty.

## 5.2 Treatment heterogeneity may be weak or nontransportable

Diagnostics may predict average outcomes but not differential response, or effect patterns may reverse across tasks. That is a valuable negative result if the design has power and effect-specific validation.

## 5.3 Diagnostic extraction may perturb the policy

Hooks, activation capture, repeated sampling, or cloned evaluation can change memory, timing, GPU kernels, or stochastic streams. Baseline instrumentation equivalence and timing audits are non-negotiable.

## 5.4 Strong baselines may dominate PID

This is not failure of the PhD. It would establish the value of intervention-grounded benchmark science and prevent an unnecessary complex diagnostic from being overinterpreted.

## 5.5 External validation may be capacity-limited

Real robot time, multiple policies, and task-family diversity may be scarce. The design should prioritize independent units and task families over dense frame counts.

## 5.6 The infrastructure claim could remain too broad

To be publishable, EC1 must show measurable improvements over ordinary scripts and standard containers on injected faults and cross-policy portability. Feature count is not evidence.

## 5.7 Fast-moving literature can change novelty

The dated search should be rerun at preregistration, submission, rebuttal, and camera-ready stages. New work should update comparators without changing the locked primary endpoint after data access.

# 6. Recommended 90-day execution order

## Days 1–15: freeze contracts and dependency decision

- adopt v12.5 as the single scientific source of truth;
- decide whether to retain the Prisoma `pid-rs` pin or migrate to `70b45f7…` through a recorded report;
- freeze the event schema, time/sequence semantics, treatment record, outcome ontology, and pre-treatment lineage rules;
- ensure the core builds with NCP disabled and H1/H2 run with PID disabled;
- create the exact claim-to-artifact matrix and preregistration skeleton.

## Days 16–30: estimator and instrumentation gates

- rerun low-dimensional continuous and discrete reference fixtures;
- add matched dimension/dependence/noise/preprocessing synthetic families;
- measure abstention coverage and false eligibility;
- test hook/non-hook output and timing equivalence;
- define one policy interface and one environment with accessible distributions or repeated samples.

## Days 31–45: infrastructure benchmark

- implement a conventional-script/standard-container baseline;
- inject timestamp, sequence, assignment, crash, partial-write, schema, and replay faults;
- run NCP in a read-only isolated or secured profile if used;
- publish machine-readable conformance reports and failure cases;
- select one external adapter challenge, preferably a documented incompatibility rather than an easy sibling path.

## Days 46–60: intervention pilot

- choose one scientifically interpretable input or internal intervention;
- define dose, placebo, positive control, receipt, and support checks;
- run Protocol A clone/coupling pilot and a small Protocol B randomized pilot;
- estimate variance components, carryover, reset reliability, and manipulation specificity;
- stop or redesign if no dose changes the target without broad degradation.

## Days 61–75: lock H1 and H2 analysis

- simulate power and model selection under weak, null, nonlinear, and sign-changing heterogeneity;
- freeze outer task-family splits, HTE validation stack, proper scores, useful margins, and multiplicity hierarchy;
- implement the full H2 comparator frontier with matched access labels;
- freeze censoring, competing-risk, alarm-policy, lead-time, calibration, and decision-utility definitions.

## Days 76–90: execute the first locked evidence block

- run the blinded/held-out H1 or H2 block without changing primary models;
- produce a complete audit trail, including abstentions and failed runs;
- evaluate whether PID is eligible for H3;
- write Paper A around the infrastructure benchmark even if PID remains blocked;
- update the risk register based on observed rather than imagined bottlenecks.

# 7. Acceptance criteria for using v12.5 as the canonical plan

The document is ready to serve as the canonical research specification when:

1. every active experiment names Protocol A, Protocol B, or prospective prediction;
2. no primary moderator is post-treatment;
3. HTE models are selected through effect-specific outer-fold criteria;
4. alarm conversion is fully frozen;
5. all PID results report population, measure, estimator, application support, and abstention;
6. the PID-disabled thesis path is executable;
7. the NCP-disabled core path is executable;
8. only evidence-graded repository relationships are described as connected or integrated;
9. an external adapter and conventional baseline are included in EC1;
10. the literature search and comparator registry are archived with dates and screening decisions.

# 8. Audit limitations

This second round is a rigorous plan review, not empirical validation of the proposed hypotheses. It examined the supplied repository snapshot and public web evidence available on July 12, 2026. It did not execute unavailable real VLA policies, physical robots, private repositories, or unpublished adapters. Public documentation can differ from code, and current branches can change after the scientific cut.

The plan’s mathematics and statistical design have been made substantially more defensible, but specialist review remains valuable in four areas before preregistration: the chosen PID measure and continuous estimator, heterogeneous-treatment-effect validation, competing-risk/sequential monitoring, and robotics intervention fidelity. Such review should test specific frozen choices rather than reopen the entire thesis scope.

# 9. Final scientific decision

v12.5 is materially stronger than v11 because it no longer lets one kind of evidence stand in for another. It distinguishes information available in a representation, information used by a policy computation, randomized effects on embodied behavior, and prospective warning utility. It gives PID a difficult but fair route to unique value and a complete route to a publishable negative result. It turns the surrounding ecosystem into a source of adversarial transport and conformance tests rather than a dependency web.

The plan is not “perfect,” and no plan can guarantee a positive PhD result. It is now structured so that negative findings, estimator abstention, baseline dominance, weak heterogeneity, and failed integrations produce interpretable scientific evidence rather than collapse the thesis. That is the relevant form of rigor.
