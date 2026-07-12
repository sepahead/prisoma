# PRISOMA: Intervention-Grounded Diagnostics for Sequential Embodied Policies

- **Document:** canonical `grandplan.md` — **docset v12.5** (living spec; prior versions in git history; the outgoing v10.7 is archived at `docs/archive/grandplan-v10.7.md`)
- **Scientific cut:** 2026-07-12
- **Repository snapshot reviewed:** `sepahead/prisoma@64bd881248463e7142d022bb95a5850bcf8fced2`; second-round review bundle preserved at `docs/reviews/2026-07-12-grandplan-v12.5/`
- **Status:** preregistration-grade research specification; not a claim of completed empirical validation
- **Repo-truth note (post-review):** the reviewed snapshot pinned NCP `v0.7.1`/wire 0.7; the repository has since migrated to NCP **`v0.8.0`/wire 0.8** (`crates/ncp-observer/Cargo.toml`), so the active pin references below are stated at the current pin
- **Seventh adversarial revision:** paired-estimand separation, causal-heterogeneity scoring repair, expanded 2025–2026 monitor/calibration/safety comparators, estimator-status reconciliation, post-pin `pid-rs` development review, full public-repository ecosystem audit, alarm-policy specification, and reference deduplication completed 2026-07-12

> **Thesis in one sentence.** Prisoma is a provenance-complete capture–intervention–replay system for testing whether genuinely pre-treatment diagnostics of embodied policies—including, but not limited to Partial Information Decomposition (PID)—predict frozen-snapshot algorithmic sensitivity, randomized closed-loop effect modification, and future failures beyond strong simpler baselines, while preserving enough experiment semantics to make each conclusion auditable and transportable.

---

## Executive decision

The scientifically defensible project is **not** “apply PID to Vision–Language–Action (VLA) models and interpret the atoms.” That formulation assumes four unproven propositions: that the population estimand is meaningful, that the finite-sample estimator recovers it in the intended regime, that observational information in a representation reflects causal use by the policy, and that PID adds value over simpler diagnostics. None may be assumed.

Prisoma should instead make three separable contributions:

1. **Experimental infrastructure.** A source-agnostic contract records synchronized observations, instructions, internal states, policy distributions, controller transformations, executed actions, interventions, and outcomes with sufficient provenance to reproduce and audit an embodied-agent experiment.
2. **Intervention-grounded science.** Exact paired software interventions establish bounded frozen-snapshot algorithmic responses, while randomized closed-loop interventions establish population-level behavioral effects. Observational diagnostics are judged against the reference appropriate to the claim and against prospective failures on held-out task families.
3. **Conditional PID contribution.** PID is one candidate diagnostic family. It becomes thesis-central only when the chosen measure and estimator pass preregistered gates and its features provide reproducible incremental value over mutual information, uncertainty, temporal, geometry, attribution, and learned baselines.

The project therefore remains valuable if continuous PID fails. A negative result can be scientifically strong when it identifies the failure regime, calibrates abstention, compares alternative diagnostics, and releases a reusable benchmark. Conversely, a visually polished simulator or dashboard without a defensible estimand, intervention design, and external comparison is not a scientific contribution.

### Primary question

> **Can a reproducible diagnostic system identify which input or internal pathways an embodied policy actually uses, and can pre-outcome signals predict intervention sensitivity and future failure under task, scene, and embodiment shifts?**

### Conditional PID question

> **After measure-specific and finite-sample validation, does PID provide information about joint-source organization that improves causal-effect prediction, failure prediction, or mechanism discrimination beyond simpler non-PID diagnostics?**

### Non-claims

Prisoma will not claim that:

- information encoded in a representation is necessarily used by the policy;
- a PID atom is a causal effect, mechanism, semantic concept, or safety certificate;
- redundancy necessarily causes robustness, uniqueness necessarily causes sensitivity, or synergy necessarily causes compositional competence;
- negative atoms mean hallucination, misinformation, or harmful behavior without measure-specific theory and intervention evidence;
- a geometry heuristic proves estimator validity;
- agreement between observational explanation methods proves faithfulness;
- a generic logger, viewer, dataset format, simulator, or renderer is novel by itself;
- one benchmark result establishes real-world reliability, safety, or universal generalization;
- a profile diagram, README statement, dependency declaration, successful build, or shared maintainer proves end-to-end integration;
- a post-treatment variable may be used as if it were a baseline effect moderator;
- physical or closed-loop individual treatment effects are directly observed in an ordinary parallel-arm experiment; exact paired software replays instead identify only the declared frozen-snapshot algorithmic contrast under their clone and random-number-coupling assumptions.

### Evidence hierarchy

Every claim must follow this order:

1. define the scientific variable and target;
2. define the population estimand and sampling regime;
3. state the target population and establish identification—consistency, assignment mechanism, positivity, timing, interference, missingness, and measurement—or label the result associational;
4. validate the estimator at the intended dimension, dependence structure, sample size, and preprocessing;
5. run leakage-safe and dependence-aware experiments;
6. compare against strong simpler alternatives;
7. interpret or operationalize only what survived the earlier gates.

Failure at an earlier level blocks later claims.

---

# 1. Scientific positioning and novelty

## 1.1 What the 2026 literature changes

The novelty case in v10.7 is too broad. By July 12, 2026:

- an ICLR 2026 paper (verify venue/status at submission) already applies PID across 26 large vision–language models, tasks, layers, and training dynamics [R18];
- a July 2026 multimodal foundation-model paper already uses a self-supervised PID-guided objective with counterfactual modality dropping and swapping, further narrowing any generic claim of novelty for “PID in multimodal learning” [R100];
- VLA failure prediction already includes explicit information-theoretic signals and cross-domain evaluation, making Tri-Info a mandatory baseline [R25];
- runtime VLA monitoring and calibration now includes SAFE, Hide-and-Seek, Rewind-IL, architecture-stratified black-box action monitors, Foresight, ActProbe, VLAConf, perturbation-based uncertainty, activation-warning studies, and temporal-difference calibration; together they span supervised internal features, coarsely supervised temporal localization, action-chunk self-consistency, kinematic signals, world-model latents, one-class confidence, perturbation disagreement, conformal calibration, simulation, and real robots [R95, R101–R105, R109–R112];
- VLA diagnosis already combines representation tracing, attention knockout, causal masking, sparse-feature intervention, and closed-loop behavior tracing [R26–R31];
- new benchmarks explicitly separate apparent capability from action-grounded use, test controlled physical reasoning, expose shortcutting or memorization, question whether task success identifies mechanism, and add process-level safety costs and risk-exposure time rather than relying on binary success alone [R27, R32–R36, R56];
- robotics ecosystems already provide timestamped multimodal containers, standardized episodic datasets, cross-embodiment corpora, visualization, and replay [R42–R48].

Prisoma should therefore claim novelty only for the **combination** of:

1. an explicit availability–use–effect distinction;
2. paired frozen-snapshot responses and randomized closed-loop effects as distinct, claim-matched reference criteria;
3. estimator validation and abstention tied to the exact regime;
4. prospective, leakage-safe prediction on held-out task families;
5. a portable capture/intervention/replay contract; and
6. a head-to-head test of PID’s incremental value.

This is a dated, documented-search statement, not an absolute priority claim. Before submission, rerun the search with saved queries, databases, inclusion criteria, screening decisions, and a machine-readable ledger.

## 1.2 Why the project matters even if niche

Embodied policies are sequential systems in which perception, instruction conditioning, internal state, action generation, controller filtering, and physical dynamics interact. Aggregate success can hide distinct failure mechanisms. An intervention-grounded diagnostic substrate can:

- distinguish “the model represented the relevant fact” from “the action pathway used it”;
- locate whether a failure entered through perception, conditioning, memory, action decoding, control, or execution;
- test whether an explanation predicts behavior under controlled perturbation;
- compare architectures without pretending that unlike hidden states are the same variable;
- evaluate prospective early-warning signals without temporal leakage;
- produce reusable, auditable experiment records.

The infrastructure contribution is not a “PID viewer.” It is an **experiment-semantics layer** binding interventions, internal-state provenance, replay, and estimands to standard robotics data.

## 1.3 Contribution counterfactual

Every infrastructure feature must answer:

> What scientific result becomes possible, more reliable, or cheaper because Prisoma exists, compared with MCAP/ROS bags, LeRobot/RLDS, Rerun, and an ordinary experiment script?

A feature counts as a research contribution only if it is externally benchmarked on a measurable axis such as timestamp alignment, dropped-event detection, intervention-assignment integrity, replay fidelity, provenance completeness, adapter effort, estimator abstention, or cross-policy portability.

---

# 2. First-principles model

## 2.1 Sequential causal system

Model an episode as a partially observed controlled dynamical system. At time \(t\):

- \(X_t\): latent physical/environment state;
- \(O_t^m\): observation in modality \(m\), e.g. RGB, depth, tactile, audio, or proprioception;
- \(L\): instruction, goal, or task specification;
- \(H_t\): observable history available to the policy;
- \(R_{t,\ell}^{q}\): internal representation at declared module/layer \(\ell\) and provenance axis \(q\);
- \(\Pi_t(\cdot\mid H_t,L)\): policy distribution over an action, token sequence, trajectory, or action chunk;
- \(A_t^{\pi}\): sampled or decoded policy proposal;
- \(C_t\): controller, safety filter, inverse kinematics, smoothing, chunk truncation, or post-processing state;
- \(A_t^{\mathrm{exec}}=g(A_t^{\pi},C_t)\): executed command;
- \(X_{t+1}\sim P(\cdot\mid X_t,A_t^{\mathrm{exec}},E_t)\): next state under exogenous factors \(E_t\);
- \(Z_{t:t+h}\): downstream outcome, such as contact, object motion, progress, collision, or failure;
- \(J_t\): assigned experimental intervention, including target, dose, block, assignment probability, and seed.

The schema must preserve these distinctions. **Policy output, executed command, and physical outcome are different targets.**

## 2.2 Three target families

### Policy decision target

Examples: action-token distribution, continuous action density, denoising trajectory, action chunk, or a declared low-dimensional functional of the policy distribution. This asks what is associated with the learned policy’s decision before downstream control.

### Executed-action target

This includes inverse kinematics, clipping, collision checking, force limiting, smoothing, latency compensation, and human override. It asks what information survives into actuation.

### Physical-outcome target

Examples: next-state change, object flow, contact state, safety cost, progress, or success. This target mixes policy behavior with controller and environment dynamics.

Every analysis must name its target. A claim about policy output cannot be generalized silently to physical outcome.

## 2.3 Three estimand classes

### Observational information

For sources \(S_1,S_2\) and target \(Y\),

\[
I_{P_{\mathrm{obs}}}(S_1,S_2;Y)
\]

is a functional of the observational distribution induced by policy, task mixture, intervention mixture, sampling, preprocessing, and temporal aggregation. It measures dependence in that regime. It does not establish causal use.

### Interventional effect

For treatment \(J\) and closed-loop outcome \(Y\), a causal estimand may be

\[
\tau(x)=\mathbb{E}[Y(1)-Y(0)\mid X=x].
\]

Identification of this quantity requires randomized assignment or explicit exchangeability, positivity, consistency, interference, and missingness assumptions. It is the reference for claims about the behavioral or physical effect of the implemented intervention in the declared population.

A **paired frozen-snapshot algorithmic response** is a different estimand: a divergence between policy outputs computed from two immutable software clones under a prespecified state-reset and random-number coupling contract. It may be directly computable for the instrumented software under that contract, but it is not an observed physical individual treatment effect and does not identify a closed-loop causal effect. It is the appropriate reference only for bounded claims about algorithmic sensitivity at the cloned state.

### Prospective prediction

For landmark \(t_0\) and horizon \(h\), define

\[
F_{t_0,h}=\mathbb{1}\{\text{failure in }(t_0,t_0+h]\}.
\]

Only information available by \(t_0\) may be used. This estimates predictive utility, not causality.

## 2.4 Availability, use, and effect

A representation may encode a fact that downstream action generation ignores. For a task-relevant variable \(Q\), define:

1. **Availability \(A_Q\):** held-out decodability of \(Q\) from \(R\);
2. **Policy use \(U_Q\):** causal effect of a targeted input/pathway/internal intervention on the policy output;
3. **Closed-loop effect \(E_Q\):** causal effect on executed behavior or physical outcome.

The object of interest is

\[
G_Q=(A_Q,U_Q,E_Q),
\]

not any component alone. High \(A_Q\) with near-zero \(U_Q\) is an availability–use gap. High \(U_Q\) with low \(E_Q\) can indicate controller compensation or environmental irrelevance. Recent action-grounding and mechanistic VLA studies make this distinction central [R26–R36].

## 2.5 Unit of inference

The default independent unit is the randomized **case** or **episode**, not a frame. A case is a reproducible tuple of task family, scene specification, initial condition, policy checkpoint, and intervention assignment. Frames and overlapping windows within a case are repeated measures.

Allowed units must be declared:

- task family for transfer claims;
- case/initial-condition seed for paired intervention claims;
- episode for outcome prediction;
- episode landmark only with clustered or survival methods;
- robot/embodiment for cross-embodiment claims.

Effective sample size is not the number of logged frames.

---

## 2.6 Identification assumptions are part of the estimand

Exact paired computational response requires an immutable clone, explicit cache/memory reset, a declared random-number coupling, and a deterministic evaluation-order contract. Randomized closed-loop effects require a separately declared experiment. For every confirmatory contrast, freeze the applicable assumptions and the evidence used to assess them [R87–R91].

### Treatment definition and consistency

Each intervention is a versioned operation, not a label such as “vision ablation.” Record target, site, timing, dose, replacement distribution, random seed, duration, downstream controller state, and treatment receipt. The potential outcome notation

\[
Y_i(j)
\]

is meaningful only when treatment version \(j\) is sufficiently well specified that two nominally identical assignments do not hide scientifically different manipulations. If multiple versions are intentionally pooled, define the mixture and its assignment probabilities. A claim applies to that intervention family, not to every conceivable way of changing the source.

### Randomization, exchangeability, and positivity

The primary causal analysis is intention-to-treat (ITT) with the recorded assignment probability. Verify that assignment was generated before treatment, could not be overwritten by the policy or operator, and has support in every prespecified analysis stratum. Report realized treatment counts and probabilities by task family, checkpoint, block, and dose. Empty or near-empty cells change the target or force pooling; they are not solved by a flexible model.

Randomization does not repair post-assignment exclusions. Crashes, reset failures, missing sensors, and policy timeouts are outcomes or censoring events until a prespecified rule says otherwise. A complete-case analysis requires an additional missingness assumption and is secondary.

### No anticipation and temporal ordering

A diagnostic used to moderate H1 must be available before the randomized intervention is assigned or applied, except for immutable design variables. Features computed from treated activations, post-intervention policy outputs, target engagement, downstream controller behavior, or future frames are post-treatment variables and cannot be primary baseline moderators. They may be manipulation checks, mediators, or outcomes under a separate estimand.

For sequential assignments, define decision times and eligibility before observing the current treatment. If treatment at time \(t\) changes eligibility, diagnostics, or outcomes at later times, use a longitudinal estimand rather than pretending repeated frames are independent parallel trials.

### Interference and shared state

The stable-unit assumption is not automatic in robotics. Interference can arise through persistent simulator state, object wear, battery/thermal state, shared maps, adaptive controllers, human learning, cached model state, network congestion, or simultaneous robots. Declare the interference unit and reset boundary. When episodes share state, randomize and infer at the independent cluster level or model the exposure mapping explicitly. “Same seed” is not proof of independent counterfactual worlds.

### Treatment receipt and noncompliance

Record assignment \(J\), attempted application, actual treatment receipt \(R\), target-engagement measures, and any downstream compensation. ITT remains primary. Per-protocol, complier, or dose-received effects are secondary because conditioning on receipt can destroy randomization. Such estimates require an explicit instrumental-variable, principal-stratum, or structural model and its assumptions; they must not be presented as a cleaner ITT.

### Measurement validity

The intervention outcome must measure the declared target. Policy divergence, executed-action change, contact, progress, safety cost, and task success answer different questions. Measurement error in source tensors, clocks, transforms, labels, and outcome detectors can attenuate or fabricate moderation. Every primary measure needs a versioned algorithm, calibration or validation evidence, and a blind error audit on held-out records.

### Identified versus mechanistic conclusions

A randomized perturbation identifies the effect of the implemented perturbation under the studied policy–environment distribution. It does not by itself prove that the perturbed variable is the unique natural mechanism, because an intervention may be off-support, non-modular, or compensated downstream. Mechanism claims require target engagement, specificity, dose response, alternative intervention constructions, positive and negative controls, and replication at another site or task family.

## 2.7 Three generalization targets

Every result must choose one of three targets and use language that matches it.

1. **Finite benchmark target.** The average over the exact sampled cases, seeds, tasks, policy checkpoint, controller, and software revisions. Randomization supports causal inference for this finite set, subject to execution integrity.
2. **Task-family superpopulation target.** An expectation over a declared sampling process for layouts, objects, instructions, initial states, and stochastic executions within named task families. Generalization requires that cases were sampled or weighted to represent that process and that uncertainty reflects the family hierarchy.
3. **Transport target.** The effect or predictive performance under a different policy, embodiment, simulator, sensor suite, controller, institution, or real environment. Transport requires explicit effect modifiers, overlap, a selection diagram or equivalent transport assumptions, and external validation. A second benchmark is not automatically representative of deployment.

Report both the empirical distribution and the intended target distribution. Where deployment prevalence, task mix, or sensor quality differs, evaluate reweighting and recalibration, and present unweighted results as the benchmark-specific quantity. Do not use “generalizes” without naming the axes varied, held fixed, and excluded.

# 3. PID: admissible claims and gates

## 3.1 PID is measure-relative

For two sources \(S_1,S_2\) and target \(Y\), a PID seeks atoms satisfying

\[
I(S_1,S_2;Y)=R+U_1+U_2+S,
\]
\[
I(S_1;Y)=R+U_1,\qquad I(S_2;Y)=R+U_2.
\]

These equations underdetermine redundancy \(R\), uniques \(U_i\), and synergy \(S\); a redundancy or uniqueness principle is required. Different measures embody different mathematical commitments and may disagree [R01–R10]. Results must therefore be named precisely, e.g. “shared-exclusions redundancy under estimator E and preprocessing P,” not “the redundancy.”

## 3.2 Shared-exclusions PID

Shared-exclusions PID defines redundancy through overlap in how source realizations exclude target outcomes. It has discrete, pointwise, measure-theoretic continuous, and kNN-estimated continuous formulations [R05–R07]. It is attractive because it is localizable and can represent informative and misinformative contributions. It is not uniquely forced by universal axioms, and signed values do not authorize semantic relabeling.

The repository’s status must be stated at four levels. First, its high-dimensional MI/coherence checks are **NO-GO**. Second, the `pid-rs` revision pinned by the reviewed Prisoma snapshot has semi-analytic low-dimensional additive-Gaussian continuous-redundancy oracle checks with closed-form pointwise terms and discrete SxPID reference agreement, so it is inaccurate to say that no estimator validation exists. Third, Prisoma’s current Experiment 0 aggregate cannot adjudicate atom validity because one default target is inappropriate for the selected measure and the strict path gates MI while only reporting atoms. Fourth, post-pin `pid-rs` main at `70b45f7b75fac06777ea215a73df01209490311a` reports a reproducible fixture against the authors’ public `csxpid` implementation, agreement within `1e-12` nats on that fixture, fail-closed continuous-support contracts, and stronger result provenance. Prisoma does not inherit those later changes while its submodule remains pinned to `8a5a9dda601556443f956a2fba164cccc913ed2e`; even after a reviewed upgrade, fixture agreement would not validate dependent, high-dimensional VLA embeddings. Continuous atoms on real embeddings therefore remain blocked [R61, R73].

## 3.3 Multisource PID is exploratory

Three or more sources introduce many antichain-indexed atoms, stronger ambiguity, and recent structural impossibility results. The 2025–2026 literature documents incompatibilities among desirable properties and challenges antichain-lattice formulations [R08–R10]. Therefore:

- full three-source PID is exploratory;
- pairwise or conditional analyses must map to a declared scientific question;
- Shannon invariants or co-information may screen high-order structure only after every constituent MI estimate passes validation [R11];
- no high-order scalar substitutes for intervention evidence.

## 3.4 Deterministic continuous mappings

For continuous \(X\) and a non-constant deterministic map \(f\), \(I(X;f(X))\) can be infinite. Neural representations and action heads often contain deterministic or near-deterministic paths [R14–R15]. A finite estimator output does not make the population quantity finite.

Admissible strategies are:

1. analyze a genuinely stochastic policy distribution or sampled output with declared randomness;
2. define an explicitly quantized estimand and report quantizer sensitivity;
3. define a fixed, scientifically justified noise-smoothed estimand;
4. use a discrete outcome target;
5. choose a different dependence or causal-effect estimand.

## 3.5 Source choice is part of the estimand

“Vision,” “language,” and “dynamics” are not natural variables waiting to be measured. They may be raw inputs, pre-fusion tokens, post-fusion residual states, action-expert states, memory, or learned projections. If \(D\) is downstream of \(V\) and \(L\), treating \(V\) and \(D\) as independent conceptual modalities is misleading. If \(L\) is constant within a task, task-local V–L PID is degenerate.

Every source requires:

- exact producer module and tensor site;
- timing relative to fusion and action generation;
- shape, dtype, mask, pooling, and aggregation;
- deterministic ancestry;
- preprocessing hash and fit split;
- occupancy/entropy eligibility checks;
- a semantic label no stronger than provenance supports.

Use neutral labels \(R^{(1)},R^{(2)}\) until semantic naming is justified.

## 3.6 Population invariance is not estimator invariance

Population MI is invariant to suitable invertible reparameterizations; finite-sample estimators generally are not. Layer normalization, scale, whitening, pooling, PCA, PLS, discretization, and learned projection can change estimates materially. Cross-layer and cross-model atom magnitudes are not directly comparable without frozen transformations, matched validation, and sensitivity analysis. Primary comparisons should be within a fixed representation site and across randomized conditions.

## 3.7 What PID must uniquely add

PID earns a central role only if it does at least one of the following beyond simpler quantities:

- distinguishes systems with similar lower-order MI but different source organization and predicts their intervention pattern;
- improves held-out prediction of paired algorithmic response or randomized closed-loop effect modification under the protocol-specific score;
- improves prospective failure prediction beyond strong baselines;
- yields a stable, measure-aware mechanism taxonomy that predicts closed-loop behavior;
- detects a joint-source phenomenon missed by individual MI, joint MI, co-information, uncertainty, temporal consistency, and learned features.

A visually intuitive decomposition is insufficient.

## 3.8 PID kill rules

Remove PID from a confirmatory claim when:

- the population estimand is undefined or scientifically unhelpful;
- oracle recovery or uncertainty coverage fails at the planned regime;
- conclusions reverse across reasonable measures without an a priori selection argument;
- results depend materially on unvalidated projection, binning, scale, or metric;
- simpler baselines meet or exceed performance within the minimum useful margin;
- atoms do not predict the preregistered paired algorithmic response or randomized closed-loop effect-modification endpoint;
- the episode/task-family sample size cannot support stable inference.

Infrastructure and non-PID science continue under these outcomes.

---

# 4. Confirmatory claim registry

The thesis should contain no more than three confirmatory scientific claims. Engineering acceptance claims are separate.

## EC1 — provenance-complete replay

For each supported policy–environment adapter, Prisoma records the declared causal and temporal variables, detects contract violations, and reproduces exact events or tolerance-bounded derived outcomes under replay. Test this against conventional scripts and standard containers; do not infer it from implementation alone.

## H1 — pre-treatment diagnostics predict intervention response

**Question.** Do diagnostics available before intervention identify cases in which a policy will be sensitive to a declared manipulation?

**Mandatory design fork.** Two scientifically valid but non-interchangeable protocols are available. Every study must designate one as primary before opening the confirmatory holdout.

1. **Protocol A — paired frozen-snapshot algorithmic response.** Clone or exactly replay one pre-treatment computational state and evaluate both treatment versions. This identifies a response of the policy computation under the declared snapshot and random-number coupling; it does not identify a physical-trajectory individual treatment effect.
2. **Protocol B — randomized closed-loop response.** Randomize treatment across independent episodes, case-periods, or valid reset blocks and estimate average or conditional effects on future policy, execution, or physical outcomes. Ordinary parallel-arm data do not reveal both physical potential outcomes for one unit.

**Unit and target population.** For Protocol A, the unit is a baseline snapshot or case defined before either clone is evaluated. For Protocol B, the unit is the randomized case or case-period defined before assignment, clustered at the interference/reset level. In both protocols, declare whether inference targets the finite benchmark, a task-family superpopulation, or a transport population.

**Treatment and timing.** The manipulation has a unique version, dose, target, placebo, positive control, manipulation checks, and receipt definition. Capture the primary moderator before treatment is assigned or applied. For Protocol B, record assignment probability, noncompliance, carryover, and reset diagnostics; ITT is primary. A time-varying treatment that changes later eligibility or diagnostics requires a longitudinal estimand rather than a static interaction model.

**Eligible moderators.** Only variables computed from the untreated baseline state, using train-only fitted transformations, may enter the primary moderator vector \(D_i\). A feature from a treated forward pass, treatment-engagement check, downstream controller state, or future frame is post-treatment and ineligible. Diagnostic extraction must itself be noninterfering: instrumented and uninstrumented baseline outputs and timing must agree within a frozen tolerance.

### Protocol A estimand

Let \(W_i\) be the immutable pre-treatment snapshot and let \(\Pi_i^{(j)}(\cdot\mid W_i)\) denote the policy output distribution under treatment version \(j\in\{0,1\}\). When the full distributions are available, define

\[
S_i=d\!\left(\Pi_i^{(1)}(\cdot\mid W_i),\Pi_i^{(0)}(\cdot\mid W_i)\right),
\]

using a preregistered divergence or physically scaled action functional. If only stochastic samples are available, define the target over a declared coupling \(C\) of policy random numbers,

\[
S_i(C)=\mathbb{E}_{C}\!\left[d\!\left(\widetilde A_i^{(1)},\widetilde A_i^{(0)}\right)\mid W_i\right],
\]

and estimate it with enough paired or independent replicates to quantify Monte Carlo error. Common random numbers are a variance-reduction device, not a neutral default: report both the coupling and a sensitivity analysis with independent streams when feasible. Randomize clone order and worker placement; reset caches, recurrent memory, samplers, and mutable hooks; and hash the starting state. A deterministic clone pair can make \(S_i\) directly observable as an algorithmic contrast, but it remains conditional on the frozen snapshot and treatment implementation.

**Protocol A analysis.** Predict \(S_i\) out of sample with task-family-blocked nested resampling. Compare a design-only model with the same model plus \(D_i\), using a prespecified proper predictive score—such as negative log predictive density or CRPS for a distributional predictor, or squared error for a point predictor—and absolute calibration across held-out response bins. Propagate replicate-level Monte Carlo uncertainty rather than treating a noisy estimate of \(S_i\) as exact. A causal forest or treatment learner is unnecessary when both algorithmic responses are directly computed.

### Protocol B estimand

For binary randomized assignment \(J\),

\[
\tau(d)=\mathbb{E}[Y(1)-Y(0)\mid D=d],
\]

or a prespecified low-dimensional projection/partition of \(D\). Report the population-average ITT effect even when heterogeneity is absent. For multiple doses, freeze a dose-response contrast or monotonicity functional rather than selecting the most favorable dose after inspection.

**Protocol B outcomes.** Keep three families distinct: (i) post-assignment policy-output change, (ii) executed-action/controller change, and (iii) progress, safety cost, or task outcome. Matched exogenous seeds are permitted only when the simulator’s reset and random-draw coupling preserve the target intervention; otherwise use randomized repeated trials. A policy-level effect cannot be silently upgraded to a physical-outcome effect.

**Protocol B analysis.** Fit the treatment-response learner inside nested, task-family-blocked cross-fitting. Candidate models may include a prespecified interaction model, causal forest, R-learner, doubly robust learner, or deliberately simple score, but model class and tuning budget are frozen before the outer holdout [R89–R91]. Because individual effects are unobserved, do not choose or validate an effect model solely by factual-outcome prediction: prognostic fit can improve while effect ranking worsens. Freeze a causal validation stack consisting of (i) a cross-fitted R-loss or doubly robust effect-prediction loss with nuisance diagnostics, (ii) causal calibration using train-defined prediction bins and held-out randomized contrasts, (iii) a rank-weighted average-treatment-effect or equivalent prioritization statistic when ranking is a goal, and (iv) policy value/regret under known assignment probabilities. Factual-outcome proper loss is only a secondary outcome-model diagnostic [R106–R108]. Do not score against naive same-data “individual effects”: physical individual treatment effects are not jointly observed.

A simple prespecified working model remains useful,

\[
Y_{ijk}=\alpha+b_{f(j)}+c_i+\beta J_{ijk}+\gamma^\top D_{ij}
+\delta^\top(J_{ijk}D_{ij})+\eta^\top X_{ij}+\varepsilon_{ijk},
\]

but coefficient significance is not the success criterion. Flexible and simple models use identical outer splits and comparable tuning budgets.

**Held-out endpoints.** Freeze endpoints separately by protocol.

- **Protocol A:** improvement in the primary predictive score for \(S_i\); calibration of predicted algorithmic response; stability across clone order, coupling, and valid output metrics; and decision value if the response prediction selects a diagnostic intervention or fallback.
- **Protocol B:** improvement in the frozen causal effect-prediction loss rather than factual-outcome fit alone; causal calibration via train-defined bins and held-out randomized contrasts; a prespecified rank/prioritization statistic when relevant; value or regret of a prespecified treatment-allocation rule; and randomization-based or cluster-aware uncertainty for the global no-effect-modification null. A model that predicts outcomes well but fails these effect-specific checks does not pass H1.

**Prohibited endpoints.** Do not correlate diagnostics with a same-data per-case difference from non-cloned physical episodes and call it treatment-effect prediction. Do not discover and evaluate subgroups on the same units. Do not blend Protocol A and B scores into one endpoint or describe a Protocol A success as evidence of closed-loop robustness.

**Null.** Diagnostics do not improve the locked held-out endpoint beyond design variables and strong non-PID baselines by the minimum useful margin.

**Success and permitted language.** Protocol A success permits the bounded statement that pre-treatment diagnostics predict frozen-snapshot algorithmic sensitivity in the evaluated regime. A claim about embodied closed-loop effect moderation requires Protocol B, acceptable manipulation/specificity evidence, and directional replication in another task family or policy. A significant interaction without held-out utility, or a Protocol A result without a closed-loop test, is insufficient for a physical-mechanism claim.

## H2 — diagnostics improve prospective, censoring-aware failure prediction

**Question.** Do signals available by landmark \(t_0\) predict a prespecified future failure type within horizon \(h\), beyond strong baselines, under the task mix and prevalence relevant to use?

**Unit.** Episode landmark. All landmarks from an episode, case seed, or persistent world state remain in one outer fold. Repeated landmarks are handled as longitudinal observations rather than independent rows.

**Time zero and eligibility.** For each landmark, freeze eligibility, feature cutoff, prediction horizon, competing events, and censoring rule before reading future data. A signal whose computation uses a future normalization constant, full-episode transform, final success label, or post-landmark intervention is leakage.

**Predictors.** Only timestamped data at or before \(t_0\). A global dataset PID atom is not an episode feature. Local information scores require a train-reference distribution, cross-fitting, an eligibility verdict, and a frozen episode/window aggregation. Missingness indicators may be predictors only when they would be observable at deployment.

**Outcome.** Use a mutually exclusive failure ontology where possible. For “failure by \(h\),” success, timeout, human takeover, reset, and other failure modes may be competing events rather than ordinary negatives. Report cause-specific and cumulative-incidence targets when the distinction changes the scientific question.

**Mandatory baselines.** Base rate; policy entropy/action uncertainty; ensemble or stochastic-pass disagreement when available; action smoothness/chunk inconsistency; state/dynamics prediction error; OOD distance; progress; and a capacity-matched learned latent baseline. Reproduce, or implement an input/supervision-matched analogue of, the strongest applicable families: SAFE-style supervised internal-state detection; Tri-Info signals; Hide-and-Seek temporal localization; ActProbe action-chunk magnitude and temporal-consistency signals; Rewind-IL/TIDE inter-chunk discrepancy; architecture-stratified black-box action features such as reversal, jerk, momentum coherence, and stall; VLAConf-style one-class internal-representation confidence; perturbation-induced action disagreement; activation-probe warning signals; Foresight-style action-conditioned world-model latents; and temporal-difference success calibration when action probabilities are available [R25, R95, R101–R105, R109–R112]. Add simple time/task/checkpoint and action-head-family indicators so complex diagnostics do not receive credit for prevalence or architecture drift. Compare methods at matched information access, supervision, action resampling, external-model use, latency, and compute; otherwise report a cost–accuracy–timeliness Pareto frontier rather than a misleading single ranking.

**Validation.** Use leave-task-family-out, temporal, or external validation matching intended use. Hyperparameters, transforms, feature selection, censoring models, and calibration are fitted inside nested training folds. A deployment claim requires an untouched external or later-time test; random frame splits are prohibited.

**Metrics.** Primary: held-out log loss or time-dependent Brier score with prespecified handling of censoring [R92]. For a dynamically updated confidence sequence, also test temporal calibration or a locked sequential proper-score analogue rather than averaging unrelated per-step calibration numbers; temporal-difference calibration is a comparator, not a guarantee of deployment validity [R112]. Secondary: precision–recall AUC at stated prevalence; calibration intercept/slope and reliability curve; event-level sensitivity at fixed false-alarm burden; alarms per episode or operating hour; a lead-time distribution that explicitly retains undetected failures; and one decision-utility analysis [R93–R94]. Conditional lead time among detected failures alone is selection-biased and cannot rank monitors. ROC AUC alone is inadequate. Converting repeated risk scores into alarms requires a frozen alarm policy—threshold, persistence/debounce rule, refractory period, event-matching window, reset behavior, and missing-score handling—tuned only in training data; otherwise false-alarm and lead-time comparisons are underdefined. If a conformal warning set or threshold is used, also report empirical coverage, set size or abstention, false-alarm burden, and subgroup/task coverage. Report uncertainty clustered at the highest independent unit and publish independent episode, event, and task-family counts.

**Shift, conformal validity, and recalibration.** Evaluate performance by task family, policy checkpoint, failure type, sensor quality, and prevalence. Standard split-conformal marginal coverage relies on exchangeability; task, temporal, policy, or embodiment shift does not preserve that guarantee automatically. Use a method whose weighted, group-conditional, online, or sequential assumptions match the design, or describe target-domain coverage as empirical rather than guaranteed [R96]. When the test prevalence is artificial, report both sampled-population and target-prevalence metrics. Recalibration or conformal recalibration on target data is a separate procedure and data split, not a hidden test-set refit.

**Null.** Adding the diagnostic family does not improve the primary proper score or decision utility by the minimum useful margin over the strongest baseline under the locked external-validity target.

**Success.** Improvement replicates on an external task family/time block, calibration remains within tolerance or succeeds under a prespecified recalibration protocol, warning is early enough to act, and subgroup degradation is bounded. A useful monitor may still fail to identify a mechanism; predictive and mechanistic claims remain separate.

## H3 — PID adds incremental value only inside its validated support envelope

H3 activates only after population, measure, estimator, and application gates in Section 7. The PID configuration is a tuple—not a generic method—containing source/target definitions, sampling law, measure, dimensionality, scaling/projection, estimator, neighborhood parameters, dependence treatment, local-score construction, and abstention rules.

Compare capacity- and tuning-budget-matched nested models:

- \(M_0\): design variables, assignment terms, base rate, and naive baselines;
- \(M_1\): \(M_0\) plus MI/CMI, co-information or Shannon-invariant screens, uncertainty, temporal, geometry, attribution, OOD, progress, and learned features;
- \(M_2\): \(M_1\) plus preregistered PID features generated only from training-reference fits that passed all gates.

**Primary endpoint.** Out-of-sample improvement of \(M_2\) over \(M_1\) under the endpoint appropriate to the active claim: direct response-prediction score for H1 Protocol A; prespecified causal effect-prediction loss, causal calibration, or policy value for H1 Protocol B; and a censoring-aware proper predictive score for H2. Use nested cross-fitting and task-family-blocked uncertainty. The minimum useful margin and a smallest effect of interest are frozen before the holdout. An equality or noninferiority region is reported; “not significant” is not evidence that methods are equivalent.

**Secondary endpoints.** Mechanism discrimination on synthetic or controlled systems with matched lower-order dependence; calibration; stability under justified nuisance transformations; and the fraction of eligible deployment cases for which PID does not abstain.

**Local-feature validity.** Episode-local or window-local PID features may not be invented by running a global estimator on a handful of within-episode samples. The construction must be derived for the named measure or clearly labelled a surrogate, use a frozen train-reference population, and pass oracle and null tests for both local ranking and aggregate reconstruction. Fit, eligibility, and evaluation folds are disjoint.

**Shared-code limitation.** Prisoma and Galadriel using the same `pid-rs` implementation is reuse, not cross-implementation validation. Independent validation requires a mathematically equivalent implementation or reference calculation whose errors are not inherited from the same core [R72–R75].

**Kill criterion.** PID becomes a negative/methodological result when the gain is below the useful margin, the eligible support is too narrow for the intended use, abstention is excessive, conclusions reverse across equally justified measures or preprocessing regimes, or replication fails. The infrastructure and H1/H2 programme continue unchanged.

## H4 — representational availability can diverge from causal policy use

H4 may replace H3 as a thesis paper if PID fails. It is not a consolation prize: the availability–use distinction is a first-order scientific problem for embodied agents.

For a task-relevant variable \(Q\), define:

- \(A_Q\): preregistered out-of-sample availability from a locked probe or decoder;
- \(U_Q^{(k)}\): the ITT effect of intervention construction \(k\) on a policy-decision target;
- \(E_Q^{(k)}\): the ITT effect of the same intervention on executed action or physical outcome;
- \(G_Q^{(k)}\): target engagement, specificity, and off-support diagnostics.

Use at least two intervention constructions where feasible—for example input-level counterfactual replacement and internal-state patching—because a null under one construction can reflect poor engagement or downstream compensation. Compare dose-response shape, affected layers/times, and policy-versus-physical effects. Include a positive-control variable known to affect the action and a negative-control site or variable expected not to.

The primary estimands are the prevalence and magnitude of prespecified discordant states, such as high \(A_Q\) with near-zero validated \(U_Q\), and the task, architecture, layer, memory, controller, or intervention factors that explain them. Thresholds for “high” and “near zero” are based on held-out probe performance and equivalence margins, not visual inspection.

Permitted conclusion: the representation contains decodable information while the tested intervention family produces little policy-use effect in the evaluated regime, conditional on engagement and support checks. Prohibited conclusion: the system never uses \(Q\), the probe reveals the natural code, or the patched activation is a modular causal variable.

## Exploratory questions

- generalization and memorization under structured perturbation;
- temporal transitions before failure under a fixed horizon;
- low-dimensional object/contact flow as a portable target;
- process-level safety costs under controlled benchmarks;
- cross-embodiment transport of relationships, not raw atom magnitudes;
- diagnostic-guided intervention or fallback selection in a prospective trial.

## Retired/deferred claims

- real-time continuous PID as an online safety monitor;
- PID-based safety certification;
- full three-source PID as a required analysis;
- atom signs as direct evidence of memorization, grounding, or world modeling;
- universal cross-model atom comparisons;
- PID as a reward before observational and intervention validity; infomorphic-network results show that local information-theoretic objectives can be trained in other settings, but they do not establish usefulness or stability for VLAs [R19];
- a custom simulator, Tauri shell, SparkJS renderer, or Gaussian-splat editor as a thesis dependency.

---

## Claim-to-evidence matrix

No prose claim may outrun this matrix. The final manuscript should instantiate one row per reported claim and link it to immutable artifacts.

| Claim class | Minimal evidence | Replication requirement | Main disqualifier |
|---|---|---|---|
| EC1 experiment semantics | schema conformance, injected faults, replay comparison, baseline stack benchmark | second independent adapter | tested only on self-generated happy paths |
| Average intervention effect | assignment integrity, ITT contrast, manipulation check, cluster-aware uncertainty | second task family for broad language | post-assignment exclusion or treatment ambiguity |
| Paired algorithmic response | immutable pre-treatment snapshot, exact clone/reset contract, declared RNG coupling, direct paired response, outer-fold prediction | second intervention construction or policy | mutable shared state, unquantified Monte Carlo error, or physical-effect language |
| Closed-loop effect moderation | pre-treatment feature, assignment integrity, outer-fold evaluation on randomized outcomes, calibration, useful-margin test | directional replication | post-treatment moderator, in-sample subgrouping, or paired-software contrast substituted for physical outcomes |
| Prospective monitor | landmark freeze, censoring/competing-risk handling, external/temporal holdout, calibration, decision utility | external task/time block | frame leakage or prevalence-obscured metric |
| PID incremental value | all four gates, matched baselines, nested cross-fitting, abstention denominator | second regime/policy | unsupported local score or shared-code “validation” |
| Mechanistic use | valid intervention, engagement/specificity, multiple constructions, policy-level effect | second site/construction | off-support perturbation with no specificity evidence |
| Transport | named target population, overlap, effect-modifier audit, external data | another site/embodiment when claimed | “different benchmark” without transport assumptions |
| Safety relevance | process/outcome measure, failure coverage, intervention evaluation | operational context | certification language or unmeasured hazards |

A claim is downgraded automatically when any required cell is missing. Statistical significance cannot upgrade a design whose identifying assumptions failed.

# 5. Experimental programme

## 5.1 Gate sequence

The programme is staged. Later results cannot rescue an earlier failed gate through post-hoc reinterpretation.

| Stage | Purpose | Required output | Gate |
|---|---|---|---|
| S0 | Freeze variables, estimands, outcomes, and causal assumptions | variable dictionary, causal graph, analysis specification | all targets and units are unambiguous |
| S1 | Validate information estimators and preprocessing | oracle recovery, coverage, stability, abstention map | at least one eligible diagnostic regime |
| S2 | Validate capture, timing, intervention, and replay | conformance and external benchmark report | no unresolved corruption; replay tolerance met |
| S3 | Pilot interventions | target-engagement, dose, carryover, placebo, OOD checks | nontrivial and interpretable intervention |
| S4 | Confirmatory H1 | locked intervention-response study | held-out family result and replication plan |
| S5 | Confirmatory H2 | prospective failure study | locked temporal/family holdout and calibration |
| S6 | Conditional H3 or H4 | incremental PID or availability–use result | second family/model replication |
| S7 | Transport study | cross-embodiment or real-robot replication | bounded claim of external validity |

## 5.2 Policy and environment selection

The first policy should maximize identifiability, not benchmark prestige. Required properties are:

- legally instrumentable weights and inference code;
- controllable randomness or sufficiently repeatable inference;
- a documented representation site before or at a meaningful fusion/action boundary;
- access to a policy distribution, logits, denoising path, or repeated samples rather than only one opaque action;
- tractable closed-loop compute;
- a reproducible environment with meaningful perturbations;
- sufficient task and instruction diversity.

The second policy should differ on one scientifically relevant axis—such as autoregressive action tokens versus continuous action chunks—while keeping tasks, outcomes, and instrumentation as comparable as possible. A comparison that changes architecture, data, embodiment, simulator, tasks, source definitions, and controller simultaneously is descriptive, not causal.

Candidate ecosystems include OpenVLA/OpenVLA-OFT, Octo, SmolVLA, and other open policies with stable hooks [R21–R24, R49–R51]. Final selection must follow a frozen access-and-instrumentation audit. Proprietary systems can support black-box external validation but not internal-mechanism claims without access to the relevant variables.

## 5.3 Factorial task design

Where feasible, cross these factors:

- task family;
- scene/layout and initial condition;
- object identity and visual appearance;
- instruction semantics and paraphrase;
- intervention type and dose;
- policy checkpoint or training regime;
- action horizon/controller setting;
- embodiment only in the transport stage.

Use a balanced or documented fractional-factorial design rather than an unstructured collection of failures. The design must preserve independent variation among instruction, scene, state, decision, and execution factors.

### Instruction-diversity gate

A language source is eligible only when the evaluated population has genuine instruction variation. Report:

- unique semantic goals and surface forms;
- empirical occupancy and entropy after the declared representation or quantization;
- paraphrase, negation, contradiction, and compositional balance;
- whether instruction is constant within the estimation unit;
- train/test separation of templates and semantic compositions.

When language is constant or nearly constant, V–L PID is ineligible. Use a population spanning instructions or a different source pair.

## 5.4 Intervention taxonomy

Every intervention is a treatment with a causal target, dose, assignment mechanism, placebo, manipulation check, and limitation statement.

### 5.4.1 Input interventions

Examples include:

- object- or region-specific visual masking with matched low-level statistics;
- illumination, texture, viewpoint, distractor, or occlusion changes;
- instruction paraphrases preserving intent;
- instruction substitutions or contradictions changing one semantic factor;
- proprioceptive noise, delay, dropout, or calibration shifts;
- tactile/contact perturbations for contact-rich tasks.

A black image is not a surgical removal of vision; it may create an extreme out-of-distribution input. Include naturalistic counterfactuals and explicit OOD diagnostics.

### 5.4.2 Internal interventions

Examples include:

- activation patching from a matched control case;
- component ablation with mean, resampled, or conditional replacement;
- sparse-feature steering or ablation;
- attention/pathway knockout;
- recurrent-memory reset or controlled truncation.

Required checks:

- intervention magnitude relative to the natural activation distribution;
- local-density or classifier-based divergence from natural states;
- specificity to the target site;
- effects on unrelated probes;
- dose–response behavior where expected;
- an equal-norm or equal-compute sham.

An intervention can change behavior yet remain mechanistically uninterpretable when it creates states far outside the model’s natural activation distribution. Intervention support, dose, and geometric stability must therefore be measured rather than assumed [R53].

### 5.4.3 Decision and execution interventions

Examples include:

- action-chunk truncation or a replanning trigger;
- controller gain, filter, or latency changes;
- safety-filter on/off in safe simulation conditions;
- bounded action remapping or noise;
- object displacement, contact perturbation, or execution disturbance.

These separate policy sensitivity from controller and environmental effects. Recent work on VLA correction and adaptive replanning supports treating horizon and execution dynamics as independent failure channels [R54–R55].

## 5.5 Randomization, pairing, and carryover

### Assignment integrity

- Generate assignments before execution from a versioned design file.
- Record randomization probability, block, seed, treatment, dose, and timestamp.
- Block on task family, scene, checkpoint, and initial-condition class.
- Conceal assignment from manual outcome annotators when feasible.
- Never reconstruct assignment from observed data when a direct log should exist.

### Pairing

In simulation, use common random numbers and identical initial-condition seeds when that preserves the intervention’s meaning. On physical robots, use randomized repeated trials, measured initial state, and randomized order.

### Carryover

For stateful policies or physical trials:

- define reset and washout criteria;
- randomize or counterbalance order;
- log model-memory reset, environment reset, and calibration state;
- test treatment-by-order interaction;
- exclude or model reset failures under a preregistered rule.

## 5.6 Manipulation checks and controls

Required checks are:

1. **Target engagement:** the intended input, activation, pathway, or controller variable changed by the planned amount.
2. **Specificity:** unrelated channels stayed within tolerance or their changes are modeled.
3. **Support:** classify the treatment as in-distribution, plausible deployment shift, or intentionally adversarial OOD.
4. **Dose calibration:** set dose from intervention mechanics or an independent pilot, not the outcome to be explained.
5. **Placebo:** use an equal-cost or equal-norm intervention not expected to target the mechanism.
6. **Positive control:** include a treatment known to change the policy or outcome.
7. **Negative-control outcome:** include an outcome that should not change on causal grounds.

“Matched behavioral impact” is circular when behavior is the endpoint. Match dose, low-level input distance, activation norm, or an independently measured nuisance effect instead.

## 5.7 Outcome definitions

### Immediate policy outcomes

Match the metric to the output:

- categorical actions: Jensen–Shannon divergence, cross-entropy shift, or probability assigned to preregistered action sets;
- continuous distributions: energy distance, Wasserstein distance, or symmetrized KL only when well defined;
- deterministic chunks: physically scaled trajectory distance plus sensitivity to noise/quantization definitions;
- denoising/flow paths: iteration-aligned integrated path deviation;
- sequences: time-aligned distance and first-deviation time.

Report physical units and scale choices. Do not combine translation, rotation, gripper, and force dimensions without a declared metric.

### Closed-loop outcomes

At minimum record:

- externally defined task success;
- progress/subgoal completion;
- collision, contact, cumulative process-level safety cost, and risk-exposure duration;
- object-state error;
- steps or time to completion;
- intervention/replanning count;
- recovery after perturbation.

Binary success alone is insufficient for mechanism diagnosis: a nominally successful episode may still be unsafe, and a failed episode may have very different exposure severity or duration. Safety work must report process-level outcomes, distinguish safe success, unsafe success, safe failure, and unsafe failure when applicable, and must not be called certification [R56–R58].

### Failure ontology

Labels should distinguish, when observable:

- target-selection or semantic failure;
- visual localization/grounding failure;
- state-memory failure;
- action-generation failure;
- controller or inverse-kinematics failure;
- contact/execution failure;
- safety-filter intervention;
- timeout or infrastructure failure;
- ambiguous/unresolved.

Diagnostic results must not be used to assign the ground-truth label being predicted.

## 5.8 Splits and replication

Use three disjoint levels:

1. **Development:** estimator calibration, feature engineering, intervention pilot, and code debugging.
2. **Locked internal test:** held-out task families, scenes, objects, and seeds.
3. **External or transport test:** second policy, simulator, embodiment, laboratory session, or dataset.

A random frame split is prohibited. A random episode split is insufficient for a claim about unseen task families when near-duplicate scenes or instructions cross folds.

Replication must predefine what is invariant. The strongest target is replication of the **relationship** between diagnostics and effects, not equality of raw atom values across architectures.

---

## 5.9 Stochastic policies, environments, and interference

Separate at least four random sources: case sampling, environment transition noise, policy sampling/decoding noise, and treatment assignment. Store their seeds and generator versions independently. Reusing one global seed can create accidental coupling or deterministic aliases that understate uncertainty.

For matched counterfactual simulation, common random numbers may improve precision only when treatment does not change the semantic meaning or number of subsequent random draws. Validate coupling with a draw ledger or counter-based random streams. If an intervention changes branch structure, report that the paired worlds are only approximately coupled and use repeated independent executions.

When policies are deterministic under fixed inputs, repeated identical runs do not increase the independent sample size. Vary a scientifically meaningful exogenous unit—initial condition, observation noise, environment stochasticity, or randomized case—not merely a logging seed. Conversely, stochastic decoding requires enough repeats to distinguish policy-distribution change from Monte Carlo noise.

Persistent memory, adaptive maps, human operators, shared robots, or thermal/battery state can couple nominal episodes. Define a washout/reset protocol, verify reset observables, and include reset failures in the flow diagram. If interference remains, randomize batches or sessions and use those as inference clusters.

## 5.10 Holdout contamination and near-duplicate audit

Task-family holdout is credible only when training and test cases are not near duplicates under the representation that matters. Before unblinding outcomes:

- hash exact assets, instructions, trajectories, scene graphs, and generated seeds;
- detect semantic paraphrase overlap, asset-family clones, mirrored layouts, trajectory subsequences, and model-training benchmark contamination where evidence is available;
- define an exclusion or grouping threshold using training data only;
- keep all members of a duplicate/lineage group in one fold;
- report performance as a function of distance from training support rather than a single “unseen” label;
- preserve a contamination ledger, including unresolved model-pretraining uncertainty.

A contamination audit does not prove absence of memorization in a foundation model. It limits known benchmark leakage and makes the remaining uncertainty explicit.

## 5.11 Transport and dataset-shift design

Before claiming cross-policy, cross-simulator, cross-embodiment, or real-world relevance, list variables that differ between source and target: morphology, action parameterization, camera geometry, controller, dynamics, instruction distribution, object set, failure prevalence, latency, and observation noise. Mark each as measured, harmonized, adjusted, intentionally varied, or unobserved.

Use a transport split that withholds the complete target domain during model selection. Evaluate overlap of prespecified effect modifiers and diagnostic support. Where overlap is weak, abstain or report target-restricted results rather than extrapolating. Any reweighting uses train/source data plus a separately defined target covariate sample; final target outcomes remain untouched until evaluation.

# 6. Statistical analysis plan

## 6.1 A complete estimand table is mandatory

Before collection, create one row per primary or secondary estimand with these fields:

| Field | Required content |
|---|---|
| Scientific question | one sentence, independent of method branding |
| Target population | finite benchmark, task-family superpopulation, or transport population |
| Unit / cluster | assignment unit, outcome unit, interference cluster, repeated-measure structure |
| Eligibility / time zero | when a unit enters and what is known then |
| Treatment or predictor | exact version, timing, dose, preprocessing, availability |
| Comparator | control, placebo, baseline model, or alternative diagnostic |
| Outcome | target level, horizon, algorithm/version, competing events |
| Potential-outcome or predictive estimand | mathematical definition and scale |
| Assignment / sampling mechanism | probabilities, blocks, case sampling, weighting |
| Identification assumptions | consistency, exchangeability, positivity, no anticipation, interference, censoring, measurement |
| Estimator | model, cross-fitting, weights, uncertainty, finite-sample correction |
| Missingness / receipt | ITT rule, crash/censoring handling, secondary per-protocol assumptions |
| Multiplicity family | primary family, hierarchy, correction or gatekeeping |
| Minimum useful effect | superiority/equivalence/noninferiority region |
| Validation target | outer holdout, external site/time/task, calibration plan |
| Abstention rule | unsupported cases and denominator |
| Permitted interpretation | exact conclusion if passed |

Changing any bold scientific field after unblinding creates a new estimand and must be labelled exploratory. A software configuration file is not a substitute for the table because it rarely records causal assumptions or the permitted conclusion.

## 6.2 Leakage and fitted preprocessing

Any learned transform—including normalization, PCA, PLS, SAE, clustering, codebook, probe, local-information reference distribution, imputation, threshold, or feature selector—must be fit inside the training fold. Nested cross-fitting is required when the transform and outcome model are both learned.

A transform record must include:

- training sample IDs and time cutoff;
- code/weights/configuration hash;
- fitted parameters or artifact hash;
- source tensor contract;
- random seed;
- intended reuse scope.

Using all episodes to fit PCA and then cross-validating a classifier is leakage even when labels were not explicitly used, because test-distribution geometry informed the features. For temporal claims, transforms must also respect time order.

## 6.3 H1 analysis: paired algorithmic and randomized closed-loop response

The analysis begins with a common preflight and then follows exactly one primary protocol. The other protocol may be a hierarchically secondary replication, but their endpoints and claim language remain separate.

### Common preflight

1. Freeze the baseline-state boundary, moderator timestamp, treatment version, intervention site, dose, output metric, reset boundary, and target population.
2. Verify that diagnostic capture is observational: compare instrumented and uninstrumented policy outputs, latency, memory state, and controller timing on blinded fixtures.
3. Construct all moderators without treatment or outcome leakage. Unsupervised transforms use outer-training predictors only; supervised diagnostic learning is nested. Freeze missing-value handling and PID abstention.
4. Keep all snapshots, clone replicates, landmarks, and episodes from one persistent case or task-family cluster in the same outer fold.
5. Predeclare whether the scientific claim is frozen-snapshot algorithmic sensitivity or randomized closed-loop effect moderation.

### Protocol A — paired algorithmic response

Clone from a content-addressed immutable snapshot after \(D_i\) is captured and immediately before the intervention site. Record model/checkpoint, weights, adapters, recurrent/cache state, preprocessing state, numerical precision, device/kernel versions, decoder state, policy RNG state, and all intervention code/configuration.

For deterministic policies or exact output distributions, compute both responses once after passing repeatability tests. For sampled or diffusion/flow policies, estimate the declared response functional with repeated draws. Use counter-based streams or a draw ledger; report whether streams are common, antithetic, or independent. Re-run a subset with reversed evaluation order, different worker/process placement, and independent streams to detect cache, scheduler, or state contamination.

Fit the response predictor only on outer-training cases and evaluate directly against held-out \(S_i\) or its replicate distribution. The primary score is frozen before inspection. Report:

- absolute and baseline-relative predictive score;
- calibration of predicted versus observed response across held-out bins;
- response reliability and Monte Carlo standard error;
- sensitivity to a second valid output metric and random-number coupling;
- performance by intervention type, task family, and response magnitude;
- failure and abstention denominators.

A same-snapshot paired contrast is unusually valuable because both computational responses can be executed, but its scope is correspondingly narrow. It establishes sensitivity of the declared algorithm under a frozen state; it does not include state-transition, controller, contact, or recovery effects.

### Protocol B — randomized closed-loop response

Reproduce assignment from the archived randomization ledger; compare planned and realized probabilities; and report assignment, attempted treatment, receipt, reset failures, crashes, censoring, exclusions, and outcomes by arm. Estimate the overall ITT effect before heterogeneity. Infer at the randomized/interference unit using randomization inference, cluster-robust methods, or a justified hierarchical model.

Fit candidate treatment-response models in outer training folds. Select and evaluate them with effect-specific criteria because factual outcome fit alone is not a valid proxy for heterogeneous-effect accuracy [R106]. The locked stack is:

- cross-fitted R-loss or a doubly robust effect-prediction loss, with propensity and outcome-nuisance diagnostics and truncation rules;
- causal calibration: define bins or a monotone calibrator without the outer test outcomes, then compare predicted effects with held-out randomized within-bin contrasts [R107];
- a rank-weighted average-treatment-effect/prioritization statistic when the use case ranks cases for intervention;
- treatment-policy value and regret relative to treat-all, treat-none, and design-only rules under the recorded assignment probabilities;
- stability across task-family blocks, seeds, and model classes;
- factual-outcome proper loss only as a secondary check of the nuisance/outcome model.

No single metric is universally reliable across data-generating regimes, and recent large-trial evidence shows that many causal-ML effect estimates fail internal and external validation; synthetic oracle studies and empirical negative controls are therefore mandatory before trusting a selected learner [R106, R108].

Never score against an unobserved physical “true individual effect.” Synthetic systems may use oracle effects for method validation; exact simulator clone pairs may be reported as Protocol A or under a separately declared paired-world target, not as ordinary parallel-arm truth. Secondary per-protocol or complier analyses must retain the ITT result and state their extra assumptions.

### Confirmatory contrast and multiplicity

The confirmatory contrast is the locked model family with diagnostic features versus the strongest design/non-PID baseline under identical outer folds, information access, and tuning budget. Report the score difference, interval, useful-margin comparison, absolute calibration, and all abstentions. Broad model or hyperparameter search belongs inside nested resampling.

For multiple treatments or doses, either use a prespecified multinomial/dose-response learner or define a small contrast family. Pairwise fishing across modalities, layers, doses, outcomes, metrics, couplings, and horizons is not one H1 test. If Protocol A and Protocol B are both run, specify a testing hierarchy; Protocol B is required before using language about closed-loop robustness or physical outcome moderation.

## 6.4 H2 analysis: prospective failure with time and censoring

Choose the prediction target before model selection:

- binary failure within a fixed horizon among units event-free at \(t_0\);
- cause-specific hazard for a named failure;
- cumulative incidence under competing risks;
- remaining time to failure;
- dynamic risk updated at prespecified landmarks.

The data pipeline must prevent future leakage. All landmarks from an episode stay together. Window normalization, reference distributions, feature selection, censoring weights, imputation, and calibration are fitted only in the outer training data.

For fixed-horizon binary targets with complete follow-up, use log loss and Brier score. Under censoring, use a prespecified valid approach such as inverse-probability-of-censoring weighted Brier score, with the censoring model cross-fitted and stress-tested [R92]. When competing events preclude the named failure, distinguish cause-specific risk from cumulative incidence; treating every competing event as an ordinary nonfailure changes the estimand.

Evaluate:

1. discrimination and proper scoring at the frozen horizon;
2. calibration-in-the-large, slope, and reliability by risk range;
3. event-level detection probability at fixed false-alarm burden, alarms per episode/time, and lead time with undetected failures retained explicitly rather than omitted, under a preregistered threshold/persistence/refractory/event-matching policy;
4. decision utility under explicit costs, fallback capacity, and intervention latency [R93];
5. robustness to task/prevalence shift and missing sensors;
6. external or later-time validation without refitting, followed separately by prespecified recalibration if needed.

Capacity-match learned baselines by training examples, labels, tuning trials, and compute budget. Reproduce applicable representatives of supervised internal-state monitoring, coarsely supervised temporal localization, pure action-space and inter-chunk monitoring, architecture-stratified kinematic monitoring, one-class internal confidence, perturbation disagreement, activation probes, information-theoretic signals, action-conditioned world-model latents, and sequential calibration—or state precisely why an interface is unavailable [R25, R95, R101–R105, R109–R112]. Compare not only predictive performance but annotation burden, white-box access, action-resampling cost, external-model cost, latency, warning time, recovery coupling, and conformal abstention/coverage. Report failure prevalence and independent episode, event, and family counts with every metric. Precision–recall summaries are interpreted at that prevalence [R94].

Conformal calibration is nested inside training/calibration folds. Report the exact nonconformity score, calibration unit, exchangeability or shift assumption, finite-sample correction, and whether repeated landmarks violate the nominal unit. Under task-family or temporal shift, coverage is an empirical transport result unless the chosen weighted/group/sequential method supplies a theorem whose assumptions were checked [R96].

## 6.5 Baseline hierarchy

Baselines must be built and frozen before examining PID’s confirmatory endpoint.

### Level 0: design and naive baselines

- prevalence-only predictor;
- task family, horizon, severity, and initial-state variables;
- last action or simple progress trend.

### Level 1: policy uncertainty and temporal baselines

- action entropy or sample dispersion;
- ensemble/stochastic-pass disagreement;
- action smoothness and chunk inconsistency;
- dynamics/world-model prediction error;
- OOD or representation-distance score;
- Tri-Info’s diagnostic families implemented as faithfully as access permits [R25];
- SAFE-style supervised internal-state scores and Hide-and-Seek-style coarsely supervised temporal localization when matched labels and interfaces exist [R95, R110];
- action-space TCE/ACM features (ActProbe family), Rewind-IL/TIDE inter-chunk discrepancy, architecture-stratified reversal/jerk/momentum/stall features, one-class internal-state confidence (VLAConf family), perturbation disagreement, and activation-probe warning scores when the required interface exists [R102–R105, R109, R111];
- action-conditioned world-model latent prediction error or features when a matched external model and compute budget are available (Foresight family) [R101];
- temporal-difference or other explicitly sequential calibration baseline when policy action probabilities and the required trajectory supervision are available [R112].

### Level 2: information baselines

- individual and joint MI/CMI only where validated;
- co-information or Shannon invariants only where constituent terms pass gates;
- simple cross-correlation, canonical correlation, or predictive likelihood;
- discrete contingency statistics for categorical targets.

### Level 3: learned baselines

- capacity-matched regularized classifier/regressor on frozen representations;
- a temporal model when the endpoint is temporal;
- attribution or intervention-derived features when available;
- VLA-Trace/BeTTER-style mechanism features where technically comparable [R26–R27].

PID must be compared with the strongest valid baseline, not merely entropy or majority class.

## 6.6 Multiplicity and researcher degrees of freedom

The analysis tree includes many source pairs, layers, targets, measures, estimators, dimensions, windows, horizons, tasks, and doses. Uncontrolled search makes nominal p-values meaningless.

Use hierarchical gatekeeping:

1. estimator eligibility;
2. one primary source/target contract;
3. one primary endpoint for H1 and H2;
4. one locked PID measure/regime for H3;
5. secondary families controlled with false-discovery-rate procedures or simultaneous intervals;
6. all unregistered variants labelled exploratory.

Do not select a layer, projection dimension, PID measure, or temporal window because it maximizes the test statistic. A multiverse may be reported, but the confirmatory result must remain the locked branch.

## 6.7 Uncertainty and dependence

Use uncertainty at the level supporting the claim:

- task-family block bootstrap for transfer claims;
- case/episode cluster bootstrap for repeated frames/windows;
- randomization inference for randomized treatment assignments where feasible;
- hierarchical-model intervals with small-cluster corrections or sensitivity checks;
- paired bootstrap for matched control/treatment cases;
- nested resampling when preprocessing or feature selection is fit.

A moving-block bootstrap over frames does not create new independent task families. Report the number of independent clusters and the distribution of cluster sizes.

Estimator uncertainty and downstream prediction uncertainty must both be propagated. Treating an estimated PID atom as error-free can attenuate or destabilize downstream effects.

## 6.8 Power and design analysis

Power is a capture gate, not a generic sample-count paragraph. Use simulation based on the complete nested design:

- task-family heterogeneity;
- case and episode random effects;
- treatment assignment and dose;
- outcome prevalence and severity;
- repeated measures and autocorrelation;
- missing/aborted runs;
- diagnostic measurement error;
- estimator abstention;
- selected hypothesis test or predictive comparison;
- multiplicity and planned validation split.

Define a minimum useful effect before simulation. Report operating characteristics across plausible nuisance parameters, not a single optimistic count. For H1 Protocol B, simulate effect-model selection and calibration under null, weak, nonlinear, and sign-changing heterogeneity rather than powering only the average effect. For H2, vary the number of independent failures, episodes, task families, censoring patterns, and false-alarm opportunities; the number of landmarks is not the event count. The final design must include enough independent families or embodiments for the claimed generalization level; more frames do not compensate for one family.

## 6.9 Missingness, crashes, and intervention failures

Create a run-status ontology before collection:

- completed and scorable;
- completed but outcome ambiguous;
- intervention not delivered;
- reset failure;
- sensor/log corruption;
- policy or simulator crash;
- human safety stop;
- infrastructure timeout.

Never silently delete crashes or safety stops: they can be outcome-related. Report all assignments, treatment receipt, exclusions, and a flow diagram. Use intention-to-treat as the primary causal analysis when assignment is randomized, with treatment-received analyses secondary and explicitly assumption-dependent.

## 6.10 Robustness and falsification checks

Required checks include:

- label and assignment permutation under the same cluster structure;
- negative-control source, treatment, and outcome;
- placebo interventions;
- alternative but justified metrics/scales;
- leave-one-family-out influence analysis;
- model/data randomization for attribution methods;
- intervention-dose and OOD sensitivity;
- sensitivity to task mixture and prevalence;
- replication after freezing all decisions;
- comparison of conclusions with and without high-leverage cases;
- direct reporting of null and contradictory results.

No robustness analysis may be used to replace the primary result post hoc.

---

## 6.11 Sensitivity analyses tied to assumptions

Sensitivity analyses are not an unbounded menu. Each addresses a named assumption:

| Assumption | Prespecified diagnostic or sensitivity analysis |
|---|---|
| treatment consistency | analyze intervention versions/doses separately; inspect target-engagement distributions |
| assignment integrity | exact randomization reconstruction; balance as a corruption check, not a validity test |
| positivity | assignment/support tables and effective sample size by stratum |
| no anticipation | automated timestamp lineage and feature-availability audit |
| no interference | alternative clustering/reset exclusions; batch/session analysis |
| missing at random / censoring | worst-case bounds, pattern-mixture or weighting sensitivity where defensible |
| outcome validity | blinded relabel sample and alternative locked detector |
| model specification | simple versus flexible learner under the same outer folds |
| transport overlap | support plots, target restriction, bounded extrapolation |
| PID regime validity | measure/preprocessing alternatives labelled as separate estimands |

A robustness result that changes the estimand must be reported as such, not as confirmation of the original one.

# 7. Estimator and measure validation

## 7.1 Separate four questions

The current documentation sometimes compresses distinct failures into one verdict. Use four independent gates:

1. **Population gate:** is the intended quantity finite, defined, and scientifically meaningful?
2. **Measure gate:** does the chosen PID functional have the properties needed for the claim in the specified source/target class?
3. **Estimator gate:** does the implementation recover the functional with acceptable bias, uncertainty coverage, and failure detection at the planned regime?
4. **Application gate:** are the real embeddings and sampling process sufficiently close to a validated regime for interpretation?

Passing an MI coherence check does not validate a PID measure. Passing a low-dimensional PID fixture does not validate high-dimensional embeddings. Stability across seeds does not establish correctness.

## 7.2 Current repository status

At the reviewed Prisoma snapshot and its pinned `pid-rs` revision:

- the high-dimensional MI/coherence path is **NO-GO** on nuisance-dimension controls;
- `pid-rs` has meaningful low-dimensional implementation evidence: continuous shared-exclusions redundancy is checked against a semi-analytic additive-Gaussian oracle with closed-form pointwise terms and a paired finite-sample Monte Carlo expectation, and discrete SxPID is checked bit-faithfully against reference values; these results validate named fixtures, not arbitrary embedding regimes [R73];
- Prisoma’s default Experiment 0 aggregate label is still **not** an atom-validity verdict because one legacy redundancy target is measure-mismatched and the strict band gates analytically known MI terms rather than the full VLA application;
- at the pinned Prisoma dependency, reproducible external continuous cross-implementation provenance was still documented as pending; post-pin `pid-rs` main at `70b45f7…` now reports a committed `csxpid` fixture with agreement within `1e-12` nats, but that later evidence is not part of the frozen Prisoma build until the submodule is deliberately upgraded and its integration suite rerun [R73];
- continuous shared-exclusions for the intended high-dimensional, dependent, transformed VLA tensors remains **NOT APPLICATION-VALIDATED**: a low-dimensional cross-implementation fixture does not establish broad estimator consistency or application validity, atom components combine estimators with different bias profiles, uncertainty procedures have kNN-specific caveats, and the application-support envelope has not passed [R61, R73];
- no evidentiary real-VLA capture has yet passed all estimator, endpoint, power, and application-support prerequisites [R61].

The v12.5 plan preserves these distinctions. Low-dimensional oracle success is real evidence; it is neither zero evidence nor permission to interpret high-dimensional VLA atoms.

The preregistration must freeze one estimator environment. The preferred path is to evaluate `pid-rs@70b45f7…` as a **candidate upgrade**, not silently float to `main`: first reproduce the new external fixture and support-contract behavior in an isolated branch; then compare old and new outputs on every preregistered synthetic family; finally record whether changed APIs, support rejections, preprocessing metadata, or atoms alter the estimand. Until that migration report passes, the reviewed `8a5a9dd…` environment remains the reproducibility reference and is ineligible for claims that depend on the later fixture.

## 7.3 Synthetic validation matrix

Validation must span families chosen to isolate failure modes, not just familiar XOR/copy examples.

### A. Analytic or numerical-oracle families

- independent Gaussian channels with known MI;
- correlated Gaussian systems with analytic MMI/BROJA-compatible comparisons where applicable;
- discrete copy, unique, XOR, AND/OR, noisy XOR, and mixtures;
- low-dimensional shared-exclusions examples reproduced from the reference implementation;
- continuous mixtures with numerical integration or high-precision Monte Carlo oracle;
- mixed discrete–continuous targets when intended in application.

### B. Geometry and nuisance families

- added independent nuisance dimensions;
- anisotropic scaling and rotations;
- nonlinear invertible warps;
- manifolds with known coordinates;
- duplicates, ties, quantization, and low-precision tensors;
- sparse and heavy-tailed distributions;
- mixtures with varying local dimension.

### C. Dependence families

- AR and state-space trajectories with controlled autocorrelation;
- phase-locked or overlapping windows;
- repeated episodes with family-level random effects;
- policy-like deterministic mappings with controlled stochasticity;
- covariate shift between transform-fit and evaluation distributions.

### D. Mechanism-discrimination families

Construct matched systems with similar marginal MI or prediction accuracy but different source organization. These are the strongest synthetic tests of whether a PID feature adds anything scientifically distinctive.

## 7.4 Validation outputs

For every cell and sample size report:

- point bias and relative error where the oracle is nonzero;
- root mean squared error;
- confidence-interval coverage and width;
- failure/abstention rate;
- sensitivity to \(k\), metric, jitter, scale, and seed;
- monotonicity only when guaranteed by the data-generating family;
- runtime and peak memory;
- cross-implementation agreement;
- whether the population quantity changed under the transformation.

A quantity that changes after adding source noise may reflect a change in the functional, estimator error, or both. Do not assume invariance without a theorem.

## 7.5 Continuous shared-exclusions gate

A continuous \(I_\cap^{sx}\) regime is eligible only when all of the following hold:

1. target and sources define a finite population problem;
2. the exact implementation matches the paper/reference code on committed fixtures;
3. the full atom vector, not just MI terms, is validated against a measure-specific oracle or independent implementation in the relevant low-dimensional family;
4. empirical coverage and abstention meet preregistered tolerances at the intended \(N,d,k\);
5. preprocessing is frozen and separately validated;
6. dependence-aware uncertainty is supported;
7. conclusions are stable across a narrow justified hyperparameter region;
8. no known numerical fallback silently substitutes another functional.

High-dimensional atom drift without an oracle is labelled **sensitivity**, not estimator validation.

## 7.6 Discrete PID gate

Discrete PID is not an automatic escape from continuous geometry. It changes the estimand and introduces discretization bias.

Required checks:

- codebook/binning fitted on training data only;
- minimum cell occupancy and effective support;
- held-out assignment stability;
- sensitivity to codebook size and seed;
- bias correction or Bayesian/smoothed estimates where justified;
- saturation diagnostics;
- cross-measure analysis when scientific conclusions depend on the PID measure;
- explicit statement that continuous and discrete atoms are different quantities.

For categorical outcomes and deliberately discrete mechanism variables, discrete PID may be the cleanest primary route. For arbitrary clustered hidden states, it is exploratory until stability and meaning are established.

## 7.7 Mutual information and Shannon-invariant gate

KSG-type estimators have useful asymptotic properties but can fail under high dimension, strong dependence, ties, anisotropy, and finite samples [R12–R13]. Shannon invariants avoid choosing a PID measure but still inherit every constituent MI-estimation problem [R11].

Required checks for each MI term:

- exact concatenation and metric diagnosed;
- finite population estimand;
- synthetic recovery at matched dimension and dependence;
- positive joint-MI denominator separated from numerical zero;
- uncertainty propagated through ratios;
- no bound violations attributed to “interesting negative structure” before estimator failure is ruled out.

If one constituent MI term fails, the derived invariant abstains.

## 7.8 Neural and variational estimators

MINE or other neural estimators may be used as sensitivity analyses, not as unquestioned ground truth. They require critic training, held-out evaluation, optimization diagnostics, multiple seeds, and awareness that variational bounds can be loose or unstable [R16–R17].

A neural estimator is eligible only when it:

- recovers the validation matrix at the planned regime;
- generalizes to held-out synthetic families;
- reports lower/upper-bound semantics correctly;
- avoids reusing outcome-test data for critic training;
- is compared with analytic/discrete/kNN alternatives;
- has a preregistered failure and early-stopping rule.

## 7.9 Geometry diagnostics are diagnostics, not proofs

Intrinsic dimension, distance concentration, neighbor ties, local linearity, subspace angles, and perturbation stability can identify risk. They do not prove consistency of a PID estimator. Sampled \(\delta\)-hyperbolicity is especially unsuitable as a hard Euclidean-validity gate: a Euclidean line is tree-like and has \(\delta=0\).

A geometry feature may enter a hard gate only after it predicts oracle-defined estimator validity on held-out synthetic families with calibrated error. Even then, it is an empirical abstention classifier limited to its training support.

## 7.10 Manifolds and metric substitution

Replacing Euclidean or max-norm distances with geodesic or hyperbolic distances inside a published estimator is a new estimator, not a harmless implementation option. Product-volume cancellation and neighborhood definitions may change. Such a method requires derivation and independent validation.

Manifold-aware MI may be explored where justified, but no resulting MI estimate licenses shared-exclusions atoms without a compatible measure/estimator derivation. Isomap, autoencoders, or hyperbolic heads are learned transformations and must be fit and validated inside training folds.

## 7.11 Local scores and prospective features

A global PID estimate for a dataset is not an episode-level predictor. To use local scores prospectively:

- define a train-reference distribution;
- fit all neighborhoods/densities/transforms using training data only;
- compute evaluation scores without future or peer-outcome information;
- define a fixed episode/window aggregation;
- validate score calibration and stability on synthetic data;
- propagate estimation error;
- prevent evaluation episodes from changing the reference structure.

Leave-one-out computation over the full evaluation set is not prospective when each test point influences the reference used for other test points.

## 7.12 Signed values and clamping

Do not clamp negative atoms in the primary analysis unless clamping is part of the published measure and estimand. Clamping changes the functional and can hide estimator failure.

Report:

- signed and, if scientifically motivated, separately decomposed informative/misinformative components;
- numerical tolerances near zero;
- sensitivity to bias correction;
- frequency and magnitude of negative values in oracle controls;
- whether a negative aggregate is permitted by the chosen measure;
- semantic interpretation only after intervention validation.

## 7.13 Minimum acceptance criteria

The preregistration must replace placeholders below with domain-justified values:

| Gate | Example criterion structure |
|---|---|
| Oracle bias | median absolute error below \(\epsilon_b\) over eligible cells |
| Coverage | empirical \((1-\alpha)\) coverage within \([c_{lo},c_{hi}]\) |
| Abstention | at least \(1-\epsilon_a\) sensitivity to known-invalid cells and bounded false abstention |
| Stability | conclusion invariant across locked neighboring \(k\)/seed settings |
| Cross-implementation | discrepancy below \(\epsilon_x\) on fixtures |
| Dependence | clustered interval retains nominal coverage in simulated trajectories |
| Application | real-data diagnostics fall inside validated support or analysis abstains |

Thresholds must be fixed before the real outcome analysis. “Looks stable” is not a gate.

---

## 7.14 Application-support envelope and abstention denominator

For each estimator configuration, publish a machine-readable support envelope containing:

- population-law assumptions and variable support type;
- permitted source/target dimensions and metric/scaling requirements;
- dependence and effective-sample-size conditions;
- validated sample-size and signal-strength grid;
- oracle bias, variance, coverage, false-positive, and ranking performance;
- preprocessing and observation-noise models;
- known failure signatures and structured reason codes;
- eligible, warning, and reject states;
- semantic version and exact implementation revision.

At application time, every requested estimate receives one of three statuses: **eligible**, **eligible with declared warning**, or **abstain**. Report the denominator: total candidate cases/windows, cases reaching each diagnostic stage, eligible cases, successful estimates, warnings, and abstentions by reason. Predictive performance among the small easiest subset is not deployment performance.

A support classifier trained on synthetic regimes is itself a predictive model. Validate it on held-out synthetic families and adversarial near-boundary cases; do not present geometry heuristics as ground truth. When no validated estimator covers the application regime, return no atom and continue with non-PID diagnostics.

# 8. Infrastructure as a scientific contribution

## 8.1 Design principle

Prisoma should be a thin, composable experiment-semantics layer rather than a replacement for simulators, dataset formats, viewers, or robot middleware. It should import/export standard ecosystems and enforce the pieces they do not define together:

- scientific variable provenance;
- randomized intervention assignment and treatment receipt;
- internal-state tensor contracts;
- policy-versus-execution separation;
- frozen-transform lineage;
- estimator eligibility/abstention;
- exact or tolerance-bounded replay;
- outcome and exclusion provenance.

## 8.2 Canonical event model

The authoritative record is append-only and schema-versioned. Minimum event families are:

### Run and environment

- `run_started`, `run_ended`, `run_status`;
- code, dependency, container, model, dataset, scene, and hardware identifiers;
- simulator/robot/controller versions and settings;
- wall-clock and monotonic clocks with synchronization metadata;
- random seeds and determinism mode.

### Sampling and task

- task family, semantic goal, instruction ID, scene ID, initial-condition ID;
- episode/case/landmark IDs;
- split assignment fixed before outcome analysis;
- policy checkpoint and adapter version.

### Observation and internal state

- timestamps, sensor frame IDs, calibration, masks, and dropped-frame flags;
- tensor-site ID, producer module, layer, pre/post-hook semantics;
- tensor shape, dtype, device, token mask, reduction, and artifact hash;
- deterministic ancestry and relation to fusion/action modules.

### Policy, controller, and execution

- policy distribution or samples where available;
- decoding/sampling configuration;
- proposed action/chunk;
- controller transformation and safety-filter decision;
- executed command and acknowledgement;
- observed state transition.

### Intervention

- assignment ID, block, probability, seed, target, dose, and planned time;
- treatment-delivery status and actual parameters;
- placebo/positive-control flag;
- manipulation-check artifacts;
- reset/washout status;
- operator or agent invocation provenance.

### Outcome and annotation

- process-level metrics and terminal outcome;
- annotation rubric, annotator/blinding metadata, disagreement;
- failure ontology and uncertainty;
- censoring, abort, crash, or safety-stop reason.

### Derived artifact and gate

- transform-fit record and hash;
- estimator configuration, software revision, fixture version;
- gate result with reason codes;
- derived feature lineage back to raw event IDs;
- analysis plan version and output hash.

## 8.3 Time and synchronization contract

Every stream must expose:

- source timestamp and clock domain;
- ingestion timestamp;
- sequence number;
- expected rate and tolerance;
- interpolation/alignment rule;
- late, duplicate, and dropped-event handling;
- synchronization quality estimate.

The system must detect impossible orderings, nonmonotonic timestamps, missing action acknowledgements, and intervention events outside declared checkpoints. “Nearest timestamp” is not a universal alignment rule; each variable requires a declared causal timing relationship.

## 8.4 Tensor provenance contract

For every extracted representation, store a machine-readable descriptor:

```yaml
tensor_contract:
  policy_id: open-policy@sha256:...
  module_path: model.action_expert.blocks.17
  hook: output_after_residual
  logical_role: candidate_action_state
  capture_time: before_action_sampling
  shape: [tokens, hidden]
  dtype: float16
  token_semantics: [vision, language, state, action_query]
  mask_artifact: sha256:...
  reduction:
    type: masked_mean
    fitted: false
  transform_artifact: sha256:...
  ancestry: [vision_encoder, language_encoder, fusion_stack]
```

Semantic labels such as “world model” or “dynamics” require architecture evidence. Otherwise use neutral module/site identifiers.

## 8.5 Replay levels

Replay must be graded rather than declared binary:

1. **Event replay:** reproduce the logged sequence and derived artifact graph.
2. **Policy replay:** same recorded inputs produce policy outputs within declared exact/tolerance criteria.
3. **Controller replay:** proposed actions reproduce executed commands.
4. **Simulator replay:** same initial condition/actions reproduce states within physical tolerances.
5. **Counterfactual replay:** a changed intervention is applied while all declared exogenous variables are held fixed.
6. **Physical repeatability:** repeated real trials quantify, rather than assume, irreducible variability.

Floating-point, GPU, physics, and asynchronous systems may prevent bitwise equality. Tolerances must be variable-specific, empirically justified, and versioned.

## 8.6 Interoperability, not reinvention

Use existing formats according to their strengths:

- **MCAP/rosbag2** for high-rate timestamped robotics streams and transport interoperability [R43, R46];
- **LeRobot Dataset v3** for episodic robot datasets, media, metadata, and Hub distribution [R44];
- **RLDS** for step/episode-oriented sequential datasets and dataset transformations [R42];
- **Rerun** for multimodal, time-aware visualization and recording [R45];
- **Open X-Embodiment-compatible schemas** for cross-dataset/embodiment mappings where useful [R37];
- **RO-Crate/W3C PROV-style provenance** for portable research-object metadata [R62].

Prisoma’s canonical semantics can be stored in or alongside these formats. A custom JSONL log may remain an internal source of truth, but exporters/importers and conformance tests are required.

## 8.7 Adapter contract

An adapter is accepted only if it passes:

- schema completeness for required variables;
- timestamp and sequence tests under load;
- dropped/duplicate-event injection tests;
- intervention assignment and receipt tests;
- policy/controller/execution separation;
- representation hook reproducibility;
- replay tests;
- deterministic fixture and failure-injection tests;
- licensing and model-access audit.

Adapter-specific omissions must be explicit capabilities, not null fields silently interpreted as data.

## 8.8 External benchmark for the infrastructure claim

Compare Prisoma with at least:

1. an ordinary model-specific experiment script plus files;
2. MCAP/rosbag2 logging with handwritten metadata;
3. a LeRobot/RLDS episodic export;
4. a Rerun-only visualization pipeline.

Use preregistered tasks such as:

- add a new policy and capture one internal site;
- run a blocked randomized intervention;
- trace one diagnostic feature back to source frames and transform fit;
- detect a deliberately dropped intervention event;
- replay a case and reproduce a summary;
- migrate a run between two supported storage formats;
- audit whether test data leaked into preprocessing.

Candidate endpoints:

- setup/adapter engineering effort under a fixed rubric;
- schema error detection rate;
- intervention-assignment fidelity;
- timestamp alignment error;
- replay discrepancy;
- provenance completeness;
- time to answer a blinded audit question;
- proportion of invalid analyses automatically blocked.

The benchmark must include negative cases. A system that records valid runs but fails to reject invalid ones has not established scientific value.

## 8.9 Repository ecosystem: evidence, boundaries, and useful roles

The public `sepahead` profile depicts a broad project graph, but a profile diagram is architectural intent, not implementation evidence [R85]. Repository relationships are classified by auditable evidence:

- **E0 — intention:** profile, roadmap, issue, or prose says projects should connect;
- **E1 — interface specification:** schemas, adapter design, or integration document exists, but no build-tested adapter;
- **E2 — declared immutable dependency:** submodule, lockfile, exact git tag/revision, or consumer manifest creates a reproducible code relationship;
- **E3 — build-tested adapter:** producer and consumer compile/test together against golden fixtures at pinned revisions;
- **E4 — end-to-end scientific conformance:** live or replayed data traverse the boundary with schema, time, frame, intervention, provenance, fault-injection, and outcome checks;
- **E5 — independent replication:** another team or independently maintained implementation reproduces the integration and scientific result.

Use **connected** only for E2 or above, **integrated** only for E3 or above, and **validated integration** only for E4 or above. Shared ownership or shared code does not supply independence.

### 8.9.1 Audited relationship matrix at the reviewed snapshot

| Repository | Audited relationship to Prisoma | Evidence level | Scientifically useful role | Boundary / required next evidence |
|---|---|---:|---|---|
| `pid-rs` | Direct git submodule at `8a5a9dda601556443f956a2fba164cccc913ed2e`; Prisoma crates path-depend on its estimator/run-log crates | E2; parts may be E3 within Prisoma CI | canonical estimator implementation, run-log schema, low-dimensional analytic oracle, discrete reference fixtures, abstention reports | the pin predates current main’s committed `csxpid` fixture and fail-closed support contracts; upgrade only through an explicit reviewed revision, then rerun all Prisoma conformance tests; fixture validation is not VLA application validation and shared code is not independent corroboration [R72–R73] |
| `NCP` | Optional `ncp-observer`, excluded from default workspace, pinned to immutable NCP `v0.8.0` (wire 0.8); observation-plane role only | E2; local tests may support E3 for fixtures | optional source of versioned observations from neural/robotic systems; test polyglot/protocol provenance | retain read-only authority; require conforming live producer, secure realm, sequence/drop tests, and E4 report [R72, R74] |
| `galadriel` | No direct Prisoma dependency found; it pins `pid-rs` and NCP but documents that no production publisher or end-to-end mTLS deployment test exists | E0 between projects; E2 to shared dependencies | external diagnostic comparator for cross-sensor consistency, NIS/CUSUM, signed correlation, and optional PID evidence | compilation is not live integration; shared `pid-rs` results are one correlated method family, not replication [R75] |
| `crebain` | No direct Prisoma reference found; its NCP surfaces are dormant and off by default, with no always-on Crebain↔Engram loop | E0 between projects; E2/E3 only for the specific dormant NCP build surfaces tested | candidate non-manipulation embodiment, multimodal tracking/fusion testbed, timing/fault-injection producer | define and test a read-only export adapter, frames, clocks, labels, safety boundaries, secure deployment, and golden trajectories before calling it integrated [R76] |
| `manwe` | Explicitly states no drop-in Prisoma adapter; schemas, tensors, clocks, frames, and statistical assumptions differ | E0/E1 | candidate perception producer and shift/latency testbed; useful negative example for adapter discipline | satisfy its documented promotion gates; never infer compatibility from Rust/Python or shared maintainer [R77] |
| `engram` | Public repository is a placeholder and says code will be opened after publication; NCP describes an illustrative commander role | E0 | future source of neural-state streams and memory/dynamics interventions if released and instrumentable | cannot be a thesis dependency; require public revision, license, executable fixture, variable semantics, and NCP E4 evidence [R74, R78] |
| `melkor` | No direct Prisoma reference found | E0 | optional 3D reconstruction/uncertainty or scene-variation producer | separate reconstruction uncertainty from policy uncertainty; require calibrated geometry, licenses, and an adapter benchmark [R79] |
| `WorldWarp` | Prisoma has an optional integration specification; no implemented adapter was verified; upstream repository is a forked scene-generation system | E1 | external world-model/counterfactual-scene baseline under a bounded research question | high compute, generated-scene support, causal validity, and license/provenance must pass; never put on critical path [R80] |
| GauSS-MI concept | Prisoma document explicitly labels the module pre-implementation and its weighted KSG proposal heuristic | E1 | possible reconstruction-quality covariate or active-view experiment | requires estimator derivation and separate oracle gate; it is not currently an estimator capability [R81] |
| `cobot-atlas` | No direct adapter/reference found; public mesh dataset and generation pipeline | E0 | asset diversity for controlled object/appearance/layout factors | freeze asset revision, physics/collision validation, lineage, near-duplicate groups, and per-asset license/provenance [R82] |
| `relief-atlas` | No direct adapter/reference found; large generated mesh collection with per-asset licensing caveat | E0 | optional stress-test domain for disaster-response scenes, not a primary manipulation benchmark | perform asset-level licensing, quality, collision, realism, and safety/ethics audit; avoid scope expansion [R83] |
| `cortexel` | No direct Prisoma reference found; visualization-oriented project | E0 | possible renderer of scientific artifacts after a stable schema exists | visual agreement is not analysis validation; add only through a versioned export contract [R84] |
| `haldir` | Public repository supplied insufficient implementation/metadata to verify a relation | E0/unknown | possible future security/attestation layer | no scientific or security reliance until code, threat model, contract, tests, and immutable release exist [R86] |
| `brojapid-activationfunctions` | No software edge to Prisoma; a released BROJA-PID analysis of activation functions linked to prior reproduction work | E0 edge; public code/release evidence | candidate discrete mechanism fixture and cross-measure sensitivity study | BROJA unique information is a different PID measure; atom names or magnitudes are not interchangeable with shared-exclusions [R97] |
| `mahmoudian-2020-rescience` | No software edge to Prisoma; published ReScience C replication and archived code | E0 edge; publication/reproducibility lineage | evidence of prior reproducibility practice and a source of controlled transfer-function fixtures | does not validate `pid-rs`, continuous shared-exclusions, VLA hypotheses, or present infrastructure [R98] |
| `nest-simulator` | Public fork advertises PID/information-theoretic work on feature branches; no direct Prisoma root reference or pinned adapter was verified | E0 | candidate neural-state producer through a future read-only NCP adapter | pin the exact branch/commit, separate fork changes from upstream NEST, publish a fixture, and pass E4 semantics/security tests [R99] |

“No direct reference found” is a bounded statement about the reviewed public material, not proof that no private branch or unpublished adapter exists. Anonymous GitHub search is incomplete; therefore the evidence ledger records both positive evidence and search limitations.

### 8.9.2 Current implementation boundary

At `prisoma@64bd881…`, the only repository relationships that should be described as direct are:

1. `pid-rs` as the pinned canonical estimator/run-log submodule; and
2. NCP as an optional pinned dependency of the excluded read-only observer crate.

The core thesis must run with NCP disabled and must survive a PID NO-GO. WorldWarp, GauSS-MI, Engram, sibling visualization projects, generated-asset collections, and UAV testbeds are optional producers, comparators, or future transport settings—not prerequisites.

### 8.9.3 Dependency firebreak

A release candidate passes the firebreak only when:

- the capture/intervention/replay core builds and executes without NCP;
- H1/H2 baselines execute with PID disabled and without `pid-rs` atoms;
- an ordinary local-file or standard-format adapter can replace every sibling repository;
- no private repository, unpublished model, personal token, or sibling checkout is required for the minimum viable thesis;
- optional world-model, 3D, viewer, and asset components can fail without changing assignments, primary outcomes, or provenance of already-recorded runs;
- producer repositories cannot read outcome labels, holdout membership, treatment schedules beyond their necessary command, or fitted analysis transforms;
- all cross-repository artifacts are content-addressed and revision-pinned.

### 8.9.4 Adapter promotion contract

A candidate ecosystem edge advances from E1/E2 to E3/E4 only after an integration report records:

1. exact revisions, lockfiles, licenses, SBOM, and build environment;
2. source and target schemas, units, dtypes, shapes, missingness, and allowed ranges;
3. clock domains, synchronization uncertainty, sequence semantics, buffering, and drop/duplicate/reorder behavior;
4. coordinate frames, transforms, calibration lineage, action convention, and embodiment identity;
5. assignment, treatment-attempt, treatment-receipt, and outcome boundaries;
6. authentication/authorization, transport security, least privilege, and data retention;
7. golden fixtures plus malformed, delayed, duplicated, reordered, truncated, incompatible-version, and crash-recovery tests;
8. latency/throughput/resource measurements at the scientific operating point;
9. replay equivalence and provenance completeness against the canonical event model;
10. a scientific conformance test showing the adapter does not change the estimand or silently fit on holdout information.

A status badge or successful `cargo build` is E3 evidence at most, and only for the tested revisions. E4 requires data and scientific semantics, not merely type compatibility.

### 8.9.5 NCP-specific boundary

Prisoma’s NCP component is a **read-only observation client**. It must never acquire command authority merely because NCP supports an action plane [R74]. For each session, record realm, route, NCP tag/wire/contract hash, peer identities, authorization mode, encryption/ACL profile, session ID, sequence numbers, source timestamps, local receipt times, synchronization uncertainty, drops/reorders/duplicates, payload schema, and disconnect/reconnect events.

Open/default transport is unsuitable for untrusted deployment; use an isolated realm or the documented secure profile and verify it. A local mode/TTL governor is defense in depth, not network authentication. Observer failure must not alter the robot/controller, and backpressure must drop or spool diagnostics according to a declared policy rather than perturb control timing.

### 8.9.6 `pid-rs`-specific boundary

`pid-rs` is the canonical implementation dependency, not external corroboration. Prisoma must pin the exact submodule revision, archive its run configuration and structured report, and fail closed when support is unspecified. The reviewed Prisoma pin has a real low-dimensional Gaussian-oracle check but predates the reproducible external `csxpid` fixture and stricter support/provenance contracts now documented on `pid-rs` main at `70b45f7…`. A deliberate upgrade should be treated as a scientific migration: review the API and estimand changes, regenerate lockfiles and reports, rerun synthetic and adapter conformance suites, and preserve the old environment for exact replay. Even after upgrade, a low-dimensional fixture is not independent validation of broad estimator behavior or of the intended high-dimensional/dependent VLA application. Mixed-dimensional continuous three-source analysis remains exploratory. Application eligibility is decided by Section 7, not by a passing unit test or the fact that an API returns a number [R73].

### 8.9.7 Ecosystem opportunity without thesis capture

The ecosystem can create notable experiments once the core is stable:

- use `crebain` or Manwe-derived streams as a transport test of timing, multimodal fusion, and non-manipulation embodiment;
- compare Prisoma’s prospective diagnostics with Galadriel’s consistency-monitor outputs while accounting for shared dependencies;
- use NCP to test protocol-version, sequence-loss, and provenance faults with a read-only observer;
- use cobot-atlas assets for prespecified object/appearance diversity after physics and duplication audits;
- evaluate reconstruction uncertainty from `melkor` as a covariate or nuisance factor, not as a replacement for estimator uncertainty;
- use an external world model such as WorldWarp only for a separate counterfactual-support study.

Each is optional. The scientific contribution is the ability to test such systems under one explicit experiment contract, not the number of sibling repositories shown in a graph.

### 8.9.8 Scientific lineage and candidate producers are not integrations

The public ecosystem also contains two information-theoretic lineage artifacts and a possible neural-simulation producer. The 2020 ReScience C repository documents a successful replication of a three-way information-theoretic transfer-function study, and `brojapid-activationfunctions` applies the BROJA unique-information measure to activation functions [R97–R98]. They are useful sources of controlled fixtures, reproducibility practices, and cross-measure disagreement tests. They do not validate shared-exclusions, `pid-rs`, the VLA estimand, or Prisoma’s software. Cross-measure comparisons must ask whether qualitative mechanism discrimination survives; they must not compare atom labels or magnitudes as though BROJA and \(I_\cap^{sx}\) were the same functional.

The `nest-simulator` fork is a plausible future producer of neural-state streams, especially through NCP, but repository-level mention of PID branches is E0 evidence only [R99]. Promotion requires an exact branch/commit, a delta against upstream NEST, executable model fixture, variable semantics, clock and sequence contract, read-only authority, and an E4 end-to-end report. None of these repositories belongs on the minimum thesis path.

## 8.10 Current versus target implementation

Maintain a generated capability matrix with columns: feature, status (`implemented`, `tested`, `validated`, `specified`, `deferred`), exact revision, test command, evidence artifact, known limitations, evidence level E0–E5, and thesis dependency. Documentation must not call a feature “integrated” unless its evidence level meets Section 8.9.

The reviewed snapshot has meaningful estimator/run-log and adapter groundwork, but current repository prose also records blocked scientific capture and invalidated high-dimensional regimes [R61, R72]. Treat passing unit tests as evidence of software behavior, not causal identification, estimator validity, deployment security, or paper-level novelty.

## 8.11 Control plane and agent access

GUI, scripts, notebooks, and LLM agents must invoke the same typed control plane. Every mutating request must produce:

- authenticated caller/session;
- method and validated parameters;
- current run state;
- assignment/protocol authorization;
- request/response timestamps;
- effect or rejection code;
- resulting event IDs.

Safe mode should be fail-closed. An LLM-accessible API is an automation feature, not a scientific contribution unless it improves reproducibility under benchmark.

## 8.12 Security, privacy, and governance

Minimum controls:

- localhost-only default for mutating control;
- explicit authentication and authorization before remote access;
- append-only audit records and tamper-evident artifact hashes;
- path sandboxing and refusal to overwrite source data;
- secrets never stored in run logs;
- configurable redaction for human video/audio and instruction text;
- dataset consent, retention, and deletion metadata;
- dependency/model/dataset/asset licenses tracked separately;
- model-generated interventions constrained by the preregistered design and safety envelope.

The system does not become safe because actions are logged. Logging enables audit; prevention requires independent controls.

## 8.13 Visualization and rendering

Rerun-first visualization is appropriate because it supports time-aware multimodal inspection without making a custom UI the scientific bottleneck [R45]. Tauri, SparkJS, WebGPU, Gaussian splatting, and editable scenes are optional presentation or experiment-authoring layers.

Rules:

- visualization consumes validated artifacts; it is not the source of truth;
- rendered colors must not imply calibrated uncertainty without a legend and scale;
- edits route through the intervention/control plane;
- screenshots are not evidence unless linked to run IDs and underlying data;
- 3D Gaussian splats are appearance representations, not collision geometry;
- renderer novelty is outside the core thesis unless separately benchmarked.

---

# 9. Source, representation, and target selection

## 9.1 Do not begin with the labels V, L, and D

Begin with a structural map of the model. For each candidate tensor, answer:

- what inputs are its deterministic ancestors?
- has multimodal fusion already occurred?
- can downstream modules bypass it?
- is it before or after recurrence, action conditioning, and decoding?
- does its token axis have stable semantics?
- is the same site available under every treatment?
- is the tensor comparable across time and checkpoints?

Only then assign a scientific role. A hidden state downstream of vision, language, and proprioception is a fused representation, not a pure “dynamics” source. “D” must not stand for depth in one analysis and dynamics in another.

## 9.2 Recommended source families

### A. Input-source experiments

Use raw or frozen-encoder summaries of instruction, vision, proprioception, tactile sensing, or history. These are easiest to perturb semantically, but observational PID can still reflect common causes and dataset structure.

### B. Pathway-source experiments

Use pre-fusion pathways or separately routed modules when architecture supports them. This is a better match to causal intervention, but only when bypass and residual connections are mapped.

### C. Temporal-source experiments

Compare short-history and long-memory states, current observation and recurrent memory, or predicted and observed state. The sampling and target horizon must prevent one source from containing the target by construction.

### D. Model–execution experiments

Compare a policy proposal representation with controller/execution state. This can separate learned decision error from downstream control error and may be more identifiable than assigning semantics to fused transformer layers.

### E. World-model experiments

Use an explicit predictive state, rollout, object flow, contact prediction, or next-state distribution only when it is a documented model output. Evaluate it first against external predictive error. A representation called “world knowledge” by a paper is not automatically a world-model state for Prisoma.

## 9.3 Target hierarchy

Primary targets should progress from most identifiable to most consequential:

1. discrete synthetic or task variable for estimator validation;
2. policy distribution or sampled action under controlled input;
3. executed action after controller transformation;
4. low-dimensional physical state change, object flow, or contact event;
5. progress/failure outcome.

A low-dimensional target can improve estimation but does not solve high-dimensional source geometry. A flow target may be embodiment-portable only after coordinate frame, object correspondence, visibility, and contact semantics are standardized.

## 9.4 Token and temporal aggregation

Pooling is a scientific choice. Candidate approaches include:

- a predefined token subset with architecture semantics;
- attention-independent masked means;
- a fixed learned projection trained only on development data;
- task-variable probes whose outputs, not hidden vectors, become interpretable low-dimensional sources;
- phase-aligned summaries.

Do not average tokens merely because it is convenient. Report sensitivity to a small preregistered set of plausible aggregations. Token selection based on the outcome or intervention effect must occur inside nested training folds.

For temporal aggregation:

- use non-overlapping or explicitly dependent windows;
- align by observable task phase only if phase is defined without future outcome;
- avoid using terminal failure to retrospectively define “pre-failure” phase in the primary prospective analysis;
- record variable latency and action-chunk timing;
- distinguish decision time from execution and observation feedback time.

## 9.5 Cross-model analysis

Cross-model claims should use one of three designs:

1. **Within-model replication:** test the same qualitative diagnostic–effect relationship separately in each model.
2. **Common external variable:** project each model to a shared, independently defined target such as object pose, action distribution, or task variable; validate each projection separately.
3. **Matched representational test:** use CKA/CCA or another similarity analysis only to characterize alignment, then run interventions in each model. Similar representation geometry does not guarantee intervention equivalence.

Do not concatenate hidden states from different models into one estimator or compare normalized atom magnitudes without a justified common scale and validation.

## 9.6 Flow as a bridge

Object/contact flow is useful when it is:

- defined in a shared world/object coordinate frame;
- derived from simulator ground truth or independently calibrated perception;
- accompanied by visibility and correspondence confidence;
- evaluated separately for rigid, articulated, deformable, and contact-rich motion;
- distinguished as predicted flow, desired flow, executed flow, and observed flow.

A scientifically useful decomposition is:

\[
\text{prediction error} \rightarrow \text{policy mapping error} \rightarrow
\text{controller/execution error} \rightarrow \text{outcome error}.
\]

The flow bridge is an optional measurement design, not proof of embodiment independence and not a reason to build a video world model as infrastructure.

---

# 10. Related work and dated prior-art boundary

## 10.1 Information decomposition

PID is a family of measure-relative decompositions originating with Williams and Beer and expanded through unique-information, common-change-in-surprisal, shared-exclusions, and continuous formulations [R01–R08]. The 2026 field review emphasizes the absence of a universally accepted measure [R08]. Recent inconsistency and structural-impossibility results further caution against treating high-order lattice atoms as uniquely determined natural objects [R09–R10]. Shannon invariants provide scalable measure-agnostic summaries but remain dependent on valid MI estimates [R11].

Multimodal interaction decomposition was already developed for multimodal machine learning before this project, using measure choices that are not interchangeable with shared-exclusions PID [R20]. The closest recent precedent is the ICLR 2026 study (verify venue/status at submission) applying PID to 26 large vision–language models [R18]. BrainFIBRE additionally uses a self-supervised PID-guided multimodal objective with counterfactual modality dropping/swapping in neuroimaging [R100]. Prisoma must cite these lines of work and distinguish itself by sequential policies, policy/execution/outcome separation, paired and randomized interventions, prospective failure prediction, and estimator abstention—not by the generic use of PID or counterfactual modality perturbations.

## 10.2 VLA diagnosis and interpretability

Mandatory comparison families include:

| Work/family | What it already contributes | What Prisoma must add or test |
|---|---|---|
| Tri-Info [R25] | information-theoretic VLA failure prediction from action diversity, temporal consistency, and action–state coupling | independent implementation/benchmark; incremental value of intervention-grounded and PID features |
| SAFE [R110] | multitask supervised failure detection from VLA internal features across multiple policies and simulated/real settings | capacity- and supervision-matched internal-feature baseline; explicit outer-task holdout, calibration, censoring, and access-cost accounting |
| Hide-and-Seek [R95] | coarsely supervised temporal localization of VLA failure signals; runtime accuracy–timeliness analysis and conformal prediction across three policies, simulation benchmarks, and a real robot | matched-input and matched-cost H2 comparison; explicit censoring, calibration, transport, and conformal-assumption audit |
| Rewind-IL / black-box action monitoring / temporal-difference calibration [R109, R111–R112] | inter-chunk discrepancy and recovery, architecture-dependent kinematic monitor signatures, and explicitly sequential success calibration | separate detection from recovery effects; stratify by action architecture; compare sequential calibration, false alarms, lead time, and utility under identical trajectory access |
| Foresight / ActProbe / VLAConf / perturbation and activation monitors [R101–R105] | strong 2026 alternatives spanning action-conditioned world-model latents, pure action-space features, one-class internal confidence, hidden-activation perturbation disagreement, and activation probes | interface-matched comparator suite; separate gains from supervision, white-box access, resampling, external models, and compute; compare calibration, event recall, false alarms, lead time, and transport |
| VLA-Trace [R26] | multi-level tracing, CKA, attention knockout, rollout probes | common capture contract, prospective prediction, and external intervention validation |
| BeTTER [R27] | controlled physical-reasoning interventions and real-world validation | broader provenance/replay substrate and explicit availability–use analysis |
| SAE/feature intervention studies [R28–R30] | sparse features, causal steering/ablation, closed-loop behavioral tests | intervention OOD checks, standardized outcome semantics, cross-policy replication |
| Embodied-reasoning faithfulness / Pinocchio [R31] | distinguishes functional performance from faithful reasoning traces and proposes a behavioral faithfulness critic | do not equate verbalized reasoning with mechanism; ground any trace claim in action and counterfactual effects |
| RoboSemanticBench / physical-reasoning identifiability work [R32–R33] | separates semantic grounding or benchmark success from evidence of action-level use and physical generalization | generalized availability–use–effect benchmark across modalities and pathways |
| VLA-Arena, LIBERO-PRO, Colosseum V2 [R34–R36] | perturbation/generalization/shortcut benchmarks | internal-state and intervention provenance plus held-out predictive tests |

Prisoma should not claim to be the first VLA diagnostic framework. Its novelty depends on enforcing common experimental semantics and testing diagnostic claims against paired algorithmic responses, randomized closed-loop effects, and prospective external validation at the level appropriate to each claim.

### Developments indexed 8–9 July 2026

Four last-week developments sharpen the design without changing its core hypothesis:

- **LaMem-VLA** makes short- and long-term latent memory explicit, reinforcing that memory state and reset semantics should be first-class captured variables rather than hidden inside a generic `D` axis [R68].
- **TouchWorld** combines predictive tactile modeling with a faster reactive contact pathway, reinforcing the need to admit tactile/contact sources and to separate high-level policy decisions from low-level feedback control [R69].
- **LEEVLA** models task-relevant latent environment evolution, adding another candidate internal predictive representation whose semantic label and causal use must be tested rather than inferred from architecture [R70].
- **Harness VLA** places a memory-guided agentic harness and retryable manipulation primitives around a frozen VLA under deployment perturbations, creating direct adjacent work for Prisoma’s monitor/intervention layer; Prisoma must distinguish itself through randomized experiment semantics, internal-state provenance, and auditable estimator abstention [R71].

These are new preprints, not settled evidence. Their role in this plan is to update the competing-system and variable-selection landscape, not to import their performance claims.

## 10.3 Embodied datasets and logging

RLDS, LeRobot, MCAP/rosbag2, Open X-Embodiment, DROID, and Rerun establish strong prior art for episodic datasets, streaming logs, cross-embodiment data, and visualization [R37–R48]. The project should build adapters and conformance tests rather than a closed proprietary format.

## 10.4 World models and flow

World-model VLAs and explicit predictive planners are a substantial 2026 research family [R51–R52]. Reflective VLA further illustrates why observation–action–consequence history can be a scientifically meaningful source rather than generic context [R66], while the July 2026 roadmap paper underscores that the term *world model* still spans incompatible definitions [R67]. Prisoma can instrument such systems, but a catalogue of model names is not a contribution. The relevant scientific questions are:

- is the predicted variable externally accurate?
- is it causally used by action generation?
- does its diagnostic relationship survive execution and embodiment changes?
- does it outperform simpler state-prediction or uncertainty baselines?

## 10.5 Safety and correction

Current work studies safety benchmarks, safety-aware planning, correction, and replanning [R54–R58]. ForesightSafety-VLA further makes process-level cumulative safety cost, risk-exposure time, and safe/unsafe success/failure quadrants explicit across controlled visual, language, and scene variations [R56]. Prisoma may contribute process-level evidence and failure tracing. It must not claim certification or operational safety solely from diagnostic performance.

## 10.6 Internal repository ecosystem is context, not prior-art evidence

Prisoma exists within a public ecosystem of estimator, protocol, robotics, perception, asset, and visualization projects [R72–R86]. This can reduce engineering cost and create transport settings, but it must not inflate novelty or validation claims.

The direct audited dependencies are the pinned `pid-rs` submodule and the optional NCP observer. Galadriel and Crebain share dependencies or protocol context but no direct Prisoma integration was verified; each repository explicitly exposes limits on live NCP integration. Manwe documents that adaptation is required. Engram is not publicly available as an executable implementation. WorldWarp and GauSS-MI are optional pre-implementation specifications. The ReScience/BROJA repositories are scientific lineage, and the NEST fork is a candidate producer—not integration or validation. Mesh and visualization repositories are candidate inputs or outputs, not scientific evidence.

For related-work purposes, cite the scientific or software artifact that actually supports a statement. The maintainer’s profile graph is evidence of intended architecture only. Shared authorship does not constitute independent replication, and shared estimator code creates correlated implementation risk.

## 10.7 Literature-search protocol

Before each submission:

1. search arXiv, OpenReview, Crossref, Semantic Scholar, Google Scholar, ACM DL, IEEE Xplore, and robotics conference proceedings;
2. save exact Boolean queries, dates, filters, and result counts;
3. screen titles/abstracts with two-person or independently replicated decisions for novelty-critical categories;
4. maintain inclusion/exclusion reasons;
5. identify newer versions, venues, retractions, code, and licenses;
6. distinguish primary papers from blogs, leaderboards, press releases, and social-media observations;
7. archive a machine-readable bibliography and evidence table.

The dated search must include at least: `partial information decomposition robot`, `information decomposition vision language action`, `VLA failure diagnosis`, `embodied mechanistic interpretability`, `causal intervention VLA`, `action grounding benchmark`, and `robot policy internal state logging`.

---

# 11. Thesis architecture and publication strategy

## 11.1 Minimum viable thesis

The minimum defensible thesis does not require PID success:

### Paper A — experiment semantics and benchmark

**Contribution:** an open capture/intervention/replay contract plus adapters and a benchmark showing when it prevents invalid embodied-agent analyses.

**Required evidence:** external baselines, fault injection, blinded audit tasks, replay tolerance, provenance completeness, and at least two policy/environment adapters.

### Paper B — intervention-grounded diagnostics

**Contribution:** a randomized study of whether observational diagnostics predict policy and closed-loop intervention effects on held-out task families.

**Required evidence:** strong baselines, manipulation checks, hierarchical inference, replication, and availability–use analysis.

### Paper C — conditional information-decomposition study

**Contribution:** either (a) validated PID adds reproducible incremental value, or (b) a rigorous map shows where PID fails or adds no value and which alternatives succeed.

**Required evidence:** measure/estimator gates, oracle matrix, negative results, and second-model/family replication.

If Paper C cannot support a meaningful PID estimand, replace it with a dedicated availability–use–effect or estimator-abstention paper. This preserves thesis coherence rather than forcing PID.

## 11.2 Stretch papers

Only after the core:

- flow/world-model stage diagnosis;
- cross-embodiment transport;
- prospective monitor/corrector trial;
- real-robot safety-process evidence;
- methodological work on a new continuous or manifold-compatible estimator.

## 11.3 Authorship-worthy infrastructure

Infrastructure is paper-worthy when it contains a generalizable abstraction, a credible comparison, and scientific evidence—not merely code volume. Candidate generalizable abstractions are:

- intervention assignment/receipt as first-class robotics data;
- internal-state provenance linked to action and outcome;
- graded replay semantics;
- estimator eligibility and abstention events;
- leakage-auditable transform lineage;
- policy/controller/execution separation.

## 11.4 Negative-result publication

A PID-negative result is publishable only if it is stronger than “our estimates were noisy.” It should establish:

- which population quantities were meaningful;
- which measures and estimators were tested;
- matched oracle regimes and sample sizes;
- exact failure modes and abstention performance;
- whether simpler diagnostics worked;
- whether intervention effects were predictable by other means;
- the boundary of generalization.

Release fixtures and failure cases so others can reproduce the boundary.

---

# 12. Milestones, gates, and stop rules

## M0 — freeze scientific and identification contracts

Deliver:

- causal graph, variable dictionary, treatment-version ontology, and interference/reset boundary;
- source/target tensor contracts and measurement-validation plan;
- complete H1/H2 estimand table, target populations, and minimum useful effects;
- pre-treatment feature whitelist and automated lineage rule;
- baseline, intervention, outcome, competing-event, and censoring definitions;
- transport/contamination ledger and dated literature ledger;
- ecosystem evidence ledger, dependency firebreak, and optional-component map.

**Stop:** unresolved ambiguity about treatment, target, time zero, unit, target population, causal interpretation, or whether a component is a dependency versus an optional testbed.

## M1 — repair and version estimator gates

Deliver:

- separate population, measure, estimator, and application verdicts;
- committed oracle fixtures;
- cross-implementation tests;
- matched-regime synthetic matrix;
- abstention reason codes.

**Stop PID path:** no regime passes at feasible sample size. Continue infrastructure and non-PID experiments.

## M2 — complete core and ecosystem conformance benchmark

Deliver:

- schema validators, sequence/time/frame checks, and fault injection;
- MCAP, LeRobot/RLDS, and Rerun adapters or justified substitutes;
- one non-sibling local/standard producer and one structurally different adapter;
- graded replay report and external baseline comparison;
- E0–E5 evidence ledger for every claimed repository edge;
- NCP read-only observer report if activated, including secure/open realm choice;
- dependency-firebreak test with NCP disabled and PID-disabled H1/H2 path;
- supply-chain manifest, exact revisions, licenses, and adapter promotion reports.

**Stop infrastructure-paper claim:** no material advantage over a simpler stack, or the minimum viable experiments require unpublished/private sibling components. Continue with the simpler stack and keep Prisoma as project software.

## M3 — intervention pilot

Deliver:

- dose curves;
- target engagement and specificity;
- placebo/positive controls;
- OOD and carryover diagnostics;
- preliminary design-analysis parameters.

**Stop treatment:** no nontrivial dose changes the intended mechanism without uncontrolled degradation.

## M4 — locked H1 experiment

Deliver:

- preregistration and immutable split manifest;
- completed assignment flow diagram;
- treatment-moderation result;
- held-out proper scoring and calibration;
- baseline comparison.

**Stop mechanism claim:** diagnostics fail to predict randomized effects or results do not replicate.

## M5 — locked H2 experiment

Deliver:

- prospective features and landmark rules;
- task-family/temporal holdout;
- proper scoring, calibration, and decision utility;
- leakage audit;
- error and subgroup analysis.

**Stop monitor claim:** no minimum useful improvement or unacceptable calibration.

## M6 — H3 or H4

Activate H3 only if PID passed M1 and non-PID experiments established a useful problem. Otherwise execute H4.

**Stop PID-central thesis:** no stable incremental PID value after replication. Publish the boundary and retain PID as a secondary result.

## M7 — transport replication

Use a second task family, policy, simulator, embodiment, or real-robot setting. Match the claimed population. Do not generalize beyond the variation actually replicated.

---

# 13. Twenty-lens adversarial review incorporated into the plan

This section records the independent questions that must be answered before a claim survives. It is not a rhetorical “expert consensus.” Each lens has a concrete failure condition.

## Lens 1 — information theory

**Question:** Is the quantity defined, finite, measure-specific, and invariant only under transformations for which invariance is proved?

**Failure condition:** atom labels are treated as universal, deterministic continuous MI is assumed finite, or a different PID measure is substituted without changing the claim.

**Design consequence:** name the measure; validate the exact functional; make PID conditional.

## Lens 2 — causal inference

**Question:** What intervention or identification assumption connects an observational diagnostic to policy use?

**Failure condition:** correlation, decodability, or mutual information is described as causal use.

**Design consequence:** randomized intervention effects are the reference; availability, use, and outcome effect are separate.

## Lens 3 — statistical estimation

**Question:** Does the estimator recover the target with calibrated uncertainty in the intended \(N,d\), dependence, support, and preprocessing regime?

**Failure condition:** synthetic stability or a low-dimensional fixture is extrapolated to high-dimensional real embeddings.

**Design consequence:** matched-regime oracle matrix and abstention.

## Lens 4 — experimental design

**Question:** Are treatments randomized, dosed, checked, counterbalanced, and compared with placebos/positive controls?

**Failure condition:** convenience perturbations confound mechanism, OOD degradation, and task difficulty.

**Design consequence:** assignment/receipt logs, manipulation checks, common random numbers, and carryover tests.

## Lens 5 — sequential decision-making

**Question:** Are history, policy distribution, chunk timing, feedback, and horizon represented correctly?

**Failure condition:** frames are treated as IID, future data enters a “real-time” score, or action chunks are collapsed without timing semantics.

**Design consequence:** landmarks, hazard/longitudinal models, and explicit decision/execution clocks.

## Lens 6 — control and robotics

**Question:** Is failure caused by the learned policy, post-processing/controller, execution, contact, or environment dynamics?

**Failure condition:** policy output and executed action are conflated.

**Design consequence:** log proposal, transformation, command, acknowledgement, and state transition separately.

## Lens 7 — representation learning

**Question:** What does a tensor’s architecture and ancestry justify calling it?

**Failure condition:** a fused hidden state is called “vision,” “language,” or “world model” for interpretive convenience.

**Design consequence:** neutral site IDs, tensor contracts, and architecture-specific causal maps.

## Lens 8 — mechanistic interpretability

**Question:** Does an intervention remain on-support and specifically affect the claimed mechanism?

**Failure condition:** a large steering vector changes behavior and is treated as faithful mechanistic proof.

**Design consequence:** divergence, dose, sham, specificity, and closed-loop tests; intervention-support and geometric-stability diagnostics [R53].

## Lens 9 — prediction science

**Question:** Is the prediction truly prospective, calibrated, externally validated, and useful relative to strong baselines?

**Failure condition:** global dataset atoms or future windows become episode features; AUROC alone supports deployment claims.

**Design consequence:** locked landmarks, proper scores, calibration, decision utility, and TRIPOD+AI/PROBAST+AI review [R59–R60].

## Lens 10 — benchmark science

**Question:** Does the benchmark vary the factors required to identify the claim and prevent template leakage or shortcutting?

**Failure condition:** near-duplicate scenes/instructions cross folds or one task family supports a general claim.

**Design consequence:** family, semantics, object, scene, severity, and temporal holdouts; compare with current perturbation benchmarks [R34–R36].

## Lens 11 — software architecture

**Question:** Is Prisoma the minimal layer that enforces scientific semantics while composing with existing tools?

**Failure condition:** custom storage, viewer, simulator, and renderer duplicate mature systems without measurable advantage.

**Design consequence:** adapters, conformance tests, thin core, and replaceable backends.

## Lens 12 — distributed systems and timing

**Question:** Are clocks, ordering, backpressure, dropped events, retries, and partial failures explicit?

**Failure condition:** timestamps are assumed synchronized or a GUI action is not part of the authoritative log.

**Design consequence:** clock domains, sequence numbers, bounded queues, append-only events, and fault injection.

## Lens 13 — reproducibility and provenance

**Question:** Can every derived value be traced to source events, transform fit, code, weights, data, and configuration?

**Failure condition:** a result depends on an unrecorded notebook, UI action, or mutable remote artifact.

**Design consequence:** content hashes, immutable manifests, provenance graph, and replay grades.

## Lens 14 — human factors and visualization

**Question:** Does the interface help a user detect uncertainty and invalidity rather than create false confidence?

**Failure condition:** colorful atom maps imply calibrated local explanations or hide abstention.

**Design consequence:** gate status, provenance, uncertainty, and noninterpretability warnings are first-class visual elements.

## Lens 15 — safety and assurance

**Question:** What concrete safety process/outcome is measured, and what evidence tier is justified?

**Failure condition:** diagnostic association is described as certification, assurance, or safe deployment.

**Design consequence:** process-level safety metrics, safety stops, negative outcomes retained, and claims limited to evidence generation.

## Lens 16 — security and agent governance

**Question:** Can an automated agent mutate experiments or files outside the authorized design?

**Failure condition:** remote or LLM control is enabled without authentication, capability limits, and complete audit.

**Design consequence:** fail-closed local defaults, typed authorization, sandboxing, and immutable assignment rules.

## Lens 17 — ethics, privacy, and data governance

**Question:** Are human video/audio, operator actions, instructions, and annotations collected and retained under clear governance?

**Failure condition:** a technically reproducible log violates consent, privacy, or deletion obligations.

**Design consequence:** consent/provenance fields, minimization, redaction, retention, and restricted exports.

## Lens 18 — licensing and supply chain

**Question:** Can code, weights, data, scenes, generated assets, and binaries be redistributed under their separate terms?

**Failure condition:** the project’s code license is assumed to cover models or datasets.

**Design consequence:** software bill of materials, pinned dependencies, notices, artifact-level licenses, and reproducible builds.

## Lens 19 — thesis scope and project management

**Question:** What is the smallest sequence that yields a defensible paper even when optional components fail?

**Failure condition:** custom rendering, simulator work, world-model training, real-time PID, and multiple robot stacks all become prerequisites.

**Design consequence:** Paper A/B/C structure, strict gates, optional adapters, and kill rules.

## Lens 20 — philosophy of science and falsifiability

**Question:** What result would make the project abandon, narrow, or reverse its favored explanation?

**Failure condition:** every null result is redescribed as evidence that PID needs refinement.

**Design consequence:** minimum useful effects, disconfirming outcomes, immutable primary endpoints, and a PID-independent success path.

---

# 14. Risk register

| Risk | Probability | Impact | Leading indicator | Mitigation | Decision |
|---|---:|---:|---|---|---|
| No meaningful finite PID estimand for chosen tensors | high | high | deterministic/near-deterministic path; oracle mismatch | change target, discretize by design, or use non-PID estimand | kill H3 if unresolved |
| Continuous estimator fails planned regime | high | high | bias/coverage/abstention failure | lower-dimensional scientifically defined variables; discrete or MI-free path | continue H1/H2/H4 |
| Intervention is OOD or nonspecific | high | high | activation/input divergence; broad probe changes | conditional replacements, naturalistic counterfactuals, dose/sham checks | reject treatment |
| Language source is degenerate | medium | high | low entropy/occupancy | redesign task/instruction population | V–L ineligible |
| Too few independent task families | high | high | design analysis shows cluster-limited power | narrow population claim; collect families, not frames | do not claim transfer |
| Temporal leakage | medium | high | features depend on future/reference test set | landmarks, train-reference fits, automated audit | invalidate affected result |
| Sim-to-real or embodiment transport fails | high | medium | relationship reverses under second platform | publish bounded simulation claim; analyze moderators | no universal claim |
| Strong baselines match PID | high | medium | M2≈M1 | publish negative/incremental-value boundary | PID secondary |
| Generic infrastructure offers no advantage | medium | high | benchmark parity with simple stack | simplify; contribute conformance spec only | no infrastructure novelty claim |
| Model access/hooks change | medium | medium | upstream API/weights unavailable | adapter abstraction; pin artifacts; second open model | drop opaque model |
| Outcome labels are unreliable | medium | high | low inter-rater agreement; ambiguous states | objective process metrics, adjudication, uncertainty labels | narrow endpoint |
| Crashes/safety stops cause informative missingness | medium | high | imbalance by treatment | intention-to-treat, explicit status, sensitivity analysis | report as outcomes |
| Multiple-testing inflation | high | high | many layers/measures/windows | gatekeeping, locked branch, FDR for secondary work | exploratory labels |
| Compute/runtime prevents required resampling | medium | medium | pilot exceeds budget | approximate only after validation; cache distances; narrow grid | reduce scope |
| Repository/spec drift | high | medium | docs disagree with tests/manifests | generated capability matrix, CI documentation checks | block release |
| Security/privacy incident | low–medium | high | remote mutation or personal data exposure | local defaults, access control, redaction, retention | halt affected collection |
| PhD scope expands into product building | high | high | optional UI/simulator blocks experiments | enforce milestone dependencies and paper deliverables | defer M8-style work |

## 14.1 Top three existential risks

1. **The observational quantity does not map to causal use.** H1/H4 interventions address this directly.
2. **The estimator cannot recover the quantity in the application regime.** S1 abstention prevents invalid interpretation.
3. **The project builds infrastructure without proving scientific advantage.** The external conformance benchmark makes the value claim falsifiable.

---

# 15. Reproducibility, reporting, and open science

## 15.1 Preregistration package

Commit and archive before confirmatory collection:

- research questions and claim hierarchy;
- causal graph and estimand table;
- inclusion/exclusion and failure ontology;
- intervention assignments and dose rules;
- source/target contracts;
- estimator/measure gates and thresholds;
- preprocessing and split manifests;
- baseline definitions and capacities;
- primary/secondary endpoints;
- power/design simulation code and assumptions;
- multiplicity rule;
- stopping, missingness, and deviation rules.

Use an immutable DOI-bearing archive when feasible. Amendments must be dated, justified, and separated from the original plan.

## 15.2 Reproducible artifact and ecosystem bundle

Each reported result must include:

- exact Prisoma revision, dirty-state flag, and patch/manifest hashes;
- exact revisions/tags and lockfiles for every sibling or external repository;
- dependency lockfiles, SBOM, container/Nix image digest, compiler/runtime versions;
- model, dataset, asset, and calibration revisions/checksums plus licenses;
- environment, simulator, controller, driver, hardware, and policy-decoding metadata;
- raw assignment, attempted-treatment, receipt, reset, run-status, censoring, and outcome ledgers;
- schema, event ontology, NCP wire/contract/security profile when used, and validator versions;
- clock domains, synchronization estimates, sequence/drop/reorder records, and frame transforms;
- fitted transforms with training IDs and feature-availability timestamps;
- estimator support verdicts, warnings, abstentions, and full candidate denominator;
- analysis command/configuration, nested split manifests, and randomization probabilities;
- generated tables/figures with source hashes and claim–evidence rows;
- known nondeterminism, counterfactual-coupling limits, and replay tolerances;
- adapter evidence level and E3/E4 conformance report where claimed;
- a machine-readable license/provenance manifest and disclosure of inaccessible/unpublished dependencies.

The archive must reproduce the reported result without access to a private sibling repository. Optional nonredistributable assets require a verifier and acquisition instructions, not an unrecorded local path.

## 15.3 Reporting standards

- Use a study flow diagram from assignments to analyzed units.
- Report all prespecified outcomes, including nulls and gate failures.
- Separate confirmatory, secondary, exploratory, and post-hoc analyses.
- Report effect sizes and uncertainty, not only significance.
- Report calibration and prevalence for prediction.
- Report independent cluster counts.
- Report estimator abstentions and excluded regimes.
- Include a limitations table mapping each claim to its unsupported extrapolations.
- Follow TRIPOD+AI/PROBAST+AI for prediction components [R59–R60].
- Provide model/data cards and a datasheet-style description for released artifacts [R63–R65].

## 15.4 Scientific integrity and ecosystem checks in CI

CI should fail when:

- a derived artifact lacks source lineage or a content hash;
- a test/holdout identifier appears in transform fitting, feature selection, calibration, or model tuning;
- a primary H1 moderator is timestamped after assignment or treatment application;
- assignment, attempted treatment, receipt, reset, censoring, or run-status events are missing or altered;
- policy proposal, controller output, executed action, and physical outcome are conflated;
- an analysis treats repeated frames as independent randomized units;
- a PID result is emitted after a support/eligibility gate failed or without the abstention denominator;
- an analysis uses a nonlocked endpoint as “primary” or changes the target population silently;
- a document claims `connected`, `integrated`, or `validated integration` below E2, E3, or E4 respectively;
- an optional sibling component becomes required by the core firebreak test;
- NCP observer code can publish commands or omits wire/contract/security/sequence provenance;
- a dependency/tag/submodule/consumer-manifest hash differs from the archived evidence ledger;
- a document claims a component is implemented without a passing capability test;
- a citation key is missing, duplicated, undefined, or unused;
- the capability/status/evidence table is stale relative to manifests and tests;
- the release ZIP fails hash, patch-application, byte-identity, schema, or archive-integrity validation.

CI cannot validate scientific truth, independence, or external validity. It can prevent many protocol, provenance, leakage, and status-inflation errors.

# 16. Decision log

| Decision | Rationale | Revisit condition |
|---|---|---|
| PID is conditional, not foundational | estimand, measure, estimator, and incremental value are unproven | validated regime plus replicated added value |
| Randomized effects ground pathway-use claims | availability is not causal use | only if a stronger identification design is justified |
| H1/H2 are primary | they produce useful science independent of PID | pilot shows no measurable treatment or prospective target |
| H4 is fallback/companion | current 2026 literature makes availability–use gap important | evidence shows near-universal alignment |
| Full three-source PID is exploratory | combinatorics and foundational limitations | new measure/estimator with relevant validation |
| No safety-certification language | diagnostics are one evidence source | formal assurance programme with domain standards |
| Rerun/standard formats first | existing tools solve viewing/storage well | benchmark demonstrates a missing capability requiring custom work |
| Tauri/SparkJS/3DGS optional | not required for scientific claims | separate HCI/rendering research question |
| Flow is a candidate target, not universal bridge | coordinate/contact/visibility assumptions | replicated cross-embodiment relationship |
| Cross-model raw atom comparisons avoided | variables and estimators are not matched | validated common representation/scale |
| Negative results are planned outcomes | prevents PID forcing and protects thesis coherence | never; only interpretation changes |
| Repository graph is not implementation evidence | avoids claiming integrations from profile/README intent | advance only with E2–E5 artifacts |
| `pid-rs` is dependency, not independent validation | shared implementation errors are correlated | external reference implementation/calculation exists |
| NCP observer remains read-only and optional | protects control timing, authority, and thesis scope | separate reviewed control research project |
| Pre-treatment moderators only for primary H1 | prevents post-treatment bias and leakage | separate mediation/longitudinal estimand |
| ITT is primary under nonreceipt | preserves randomization | explicit IV/principal-stratum assumptions justify secondary target |
| Generalization language names its target | benchmark, superpopulation, and transport claims differ | never; target may be expanded with evidence |

---

# 17. Reference policy

- Prefer the final peer-reviewed version when available; cite arXiv version/date when it contains the current technical record.
- For 2025–2026 work, record the version accessed and recheck venue/status at submission.
- Architectural claims require paper, official code, and model-card verification where possible.
- Vendor blogs and leaderboards may motivate a question but must not carry a scientific performance claim without a reproducible protocol.
- Software capabilities and licenses must be checked against the exact pinned revision; a README is evidence of a claim, while tests and artifacts determine its evidence level.
- Every quantitative claim in a manuscript needs a row in a claim–evidence ledger with source location, version, population, and caveat.


---

# References

References are version-pinned where the revision materially affects the claim. For 2025–2026 preprints, publication status and the cited version must be rechecked at manuscript submission. A preprint is evidence of prior art and reported results, not independent replication.

## Partial information decomposition and information estimation

- **[R01]** Williams, P. L.; Beer, R. D. (2010). *Nonnegative Decomposition of Multivariate Information*. arXiv:1004.2515. https://arxiv.org/abs/1004.2515
- **[R02]** Bertschinger, N.; Rauh, J.; Olbrich, E.; Jost, J.; Ay, N. (2014). *Quantifying Unique Information*. **Entropy** 16(4):2161–2183. https://doi.org/10.3390/e16042161
- **[R03]** Ince, R. A. A. (2017). *Measuring Multivariate Redundant Information with Pointwise Common Change in Surprisal*. **Entropy** 19(7):318. https://doi.org/10.3390/e19070318
- **[R04]** Finn, C.; Lizier, J. T. (2018). *Pointwise Partial Information Decomposition Using the Specificity and Ambiguity Lattices*. **Entropy** 20(4):297. https://doi.org/10.3390/e20040297
- **[R05]** Makkeh, A.; Gutknecht, A. J.; Wibral, M. (2021). *Introducing a Differentiable Measure of Pointwise Shared Information*. **Physical Review E** 103:032149. arXiv:2002.03356. https://doi.org/10.1103/PhysRevE.103.032149
- **[R06]** Schick-Poland, K.; Makkeh, A.; Gutknecht, A. J.; Wollstadt, P.; Wibral, M. (2021). *A Partial Information Decomposition for Discrete and Continuous Variables*. arXiv:2106.12393. https://arxiv.org/abs/2106.12393
- **[R07]** Ehrlich, D. A.; Schick-Poland, K.; Makkeh, A.; Lanfermann, F.; Wollstadt, P.; Wibral, M. (2024). *Partial Information Decomposition for Continuous Variables Based on Shared Exclusions*. **Physical Review E** 110:014115. arXiv:2311.06373. https://doi.org/10.1103/PhysRevE.110.014115
- **[R08]** Liardi, A.; Down, E.; Blackburne, G.; Neri, I.; Mediano, P. A. M. (2026). *The Mathematical Landscape of Partial Information Decomposition: A Comprehensive Review of Properties and Measures*. arXiv:2603.06678v2, 1 June 2026. https://arxiv.org/abs/2603.06678
- **[R09]** Matthias, P. H.; Makkeh, A.; Wibral, M.; Gutknecht, A. J. (2025). *Novel Inconsistency Results for Partial Information Decomposition*. arXiv:2512.16662. https://arxiv.org/abs/2512.16662
- **[R10]** Lyu, S. et al. (2026). *Structural Impossibility of Antichain-Lattice Partial Information Decomposition*. arXiv:2604.03869v2. https://arxiv.org/abs/2604.03869
- **[R11]** Gutknecht, A. J.; Rosas, F. E.; Ehrlich, D. A.; Makkeh, A.; Mediano, P. A. M.; Wibral, M. (2025). *Shannon Invariants: A Scalable Approach to Information Decomposition*. arXiv:2504.15779. https://arxiv.org/abs/2504.15779
- **[R12]** Kraskov, A.; Stögbauer, H.; Grassberger, P. (2004). *Estimating Mutual Information*. **Physical Review E** 69:066138. https://doi.org/10.1103/PhysRevE.69.066138
- **[R13]** Gao, S.; Ver Steeg, G.; Galstyan, A. (2015). *Efficient Estimation of Mutual Information for Strongly Dependent Variables*. AISTATS. arXiv:1411.2003. https://arxiv.org/abs/1411.2003
- **[R14]** Amjad, R. A.; Geiger, B. C. (2019). *Learning Representations for Neural Network-Based Classification Using the Information Bottleneck Principle*. **IEEE TPAMI**. arXiv:1802.09766. https://arxiv.org/abs/1802.09766
- **[R15]** Goldfeld, Z.; van den Berg, E.; Greenewald, K.; Melnyk, I.; Nguyen, N.; Kingsbury, B.; Polyanskiy, Y. (2019). *Estimating Information Flow in Deep Neural Networks*. ICML. arXiv:1810.05728. https://arxiv.org/abs/1810.05728
- **[R16]** Song, J.; Ermon, S. (2020). *Understanding the Limitations of Variational Mutual Information Estimators*. AISTATS. arXiv:1910.06222. https://arxiv.org/abs/1910.06222
- **[R17]** Belghazi, M. I. et al. (2018). *Mutual Information Neural Estimation*. ICML. arXiv:1801.04062. https://arxiv.org/abs/1801.04062
- **[R18]** Xiu, Z.; Luo, Y.; Nakayama, H. (2026). *A Comprehensive Information-Decomposition Analysis of Large Vision-Language Models*. ICLR 2026. arXiv:2603.29676. https://arxiv.org/abs/2603.29676
- **[R19]** Makkeh, A.; Graetz, M.; Schneider, A. C.; Ehrlich, D. A.; Priesemann, V.; Wibral, M. (2025). *A General Framework for Interpretable Neural Learning Based on Local Information-Theoretic Goal Functions*. **PNAS** 122:e2408125122. https://doi.org/10.1073/pnas.2408125122
- **[R20]** Liang, P. P. et al. (2023). *Quantifying & Modeling Multimodal Interactions: An Information Decomposition Framework*. NeurIPS 2023. arXiv:2302.12247v5. https://arxiv.org/abs/2302.12247

## VLA models, diagnostics, and embodied evaluation

- **[R21]** Kim, M. J. et al. (2024/2025). *OpenVLA: An Open-Source Vision-Language-Action Model*. CoRL 2024; arXiv:2406.09246. https://arxiv.org/abs/2406.09246
- **[R22]** Octo Model Team et al. (2024). *Octo: An Open-Source Generalist Robot Policy*. RSS 2024; arXiv:2405.12213. https://arxiv.org/abs/2405.12213
- **[R23]** Black, K. et al. (2024). *π0: A Vision-Language-Action Flow Model for General Robot Control*. arXiv:2410.24164. https://arxiv.org/abs/2410.24164
- **[R24]** Physical Intelligence et al. (2025). *π0.5: A Vision-Language-Action Model with Open-World Generalization*. arXiv:2504.16054. https://arxiv.org/abs/2504.16054
- **[R25]** Yang, J. et al. (2026). *Tri-Info: Generalizable, Interpretable Failure Prediction for VLA Models via Information Theory*. arXiv:2606.19998. https://arxiv.org/abs/2606.19998
- **[R26]** Shi, H. et al. (2026). *VLA-Trace: Diagnosing Vision-Language-Action Models through Representation and Behavior Tracing*. arXiv:2605.30117. https://arxiv.org/abs/2605.30117
- **[R27]** Xu, H. et al. (2026). *Unmasking the Illusion of Embodied Reasoning in Vision-Language-Action Models*. arXiv:2604.18000. https://arxiv.org/abs/2604.18000
- **[R28]** Grant, B.; Zhao, X.; Wang, P. (2026). *Not All Features Are Created Equal: A Mechanistic Study of Vision-Language-Action Models*. arXiv:2603.19233. https://arxiv.org/abs/2603.19233
- **[R29]** Zhang, H.; Xu, M.; Dhafer, A.; Yue, S.; Dong, H.; Hao, Z. D. (2026). *Embodied Interpretability: Linking Causal Understanding to Generalization in Vision-Language-Action Models*. arXiv:2605.00321. https://arxiv.org/abs/2605.00321
- **[R30]** Jin, X.; Chatterjee, A.; Kumar, P.; Paleja, R. (2026). *Event-Grounded Sparse Autoencoders for Vision-Language-Action Policies*. arXiv:2605.17204. https://arxiv.org/abs/2605.17204
- **[R31]** Foutter, M. et al. (2026). *Do Vision-Language-Action Models Mean What They Say? On the Role of Faithfulness in Embodied Reasoning*. arXiv:2607.04681, 6 July 2026. https://arxiv.org/abs/2607.04681
- **[R32]** Yu, B. et al. (2026). *RoboSemanticBench: Diagnosing Semantic Grounding in Action Prediction for VLA Models*. arXiv:2606.02277. https://arxiv.org/abs/2606.02277
- **[R33]** Chen, T.; Manchester, I.; Chen, H. (2026). *Position: Vision-Language-Action Models Cannot Be Verified to Perform Physical Reasoning*. arXiv:2606.30686. https://arxiv.org/abs/2606.30686
- **[R34]** Zhang, B. et al. (2026). *VLA-Arena: An Open-Source Framework for Benchmarking Vision-Language-Action Models*. ICML 2026; arXiv:2512.22539v3. https://arxiv.org/abs/2512.22539
- **[R35]** Zhou, X. et al. (2026). *LIBERO-PRO: Towards Robust and Fair Evaluation of Vision-Language-Action Models Beyond Memorization*. arXiv:2510.03827v2. https://arxiv.org/abs/2510.03827
- **[R36]** Morgan, J. et al. (2026). *Colosseum V2: Benchmarking Generalization for Vision Language Action Models*. arXiv:2605.27759. https://arxiv.org/abs/2605.27759

## Embodied datasets, formats, and infrastructure

- **[R37]** Open X-Embodiment Collaboration (2023/2024). *Open X-Embodiment: Robotic Learning Datasets and RT-X Models*. arXiv:2310.08864. https://arxiv.org/abs/2310.08864
- **[R38]** Khazatsky, A. et al. (2024). *DROID: A Large-Scale In-the-Wild Robot Manipulation Dataset*. arXiv:2403.12945. https://arxiv.org/abs/2403.12945
- **[R39]** Liu, B. et al. (2023). *LIBERO: Benchmarking Knowledge Transfer for Lifelong Robot Learning*. NeurIPS 2023; arXiv:2306.03310. https://arxiv.org/abs/2306.03310
- **[R40]** Nasiriany, S. et al. (2024). *RoboCasa: Large-Scale Simulation of Everyday Tasks for Generalist Robots*. arXiv:2406.02523. https://arxiv.org/abs/2406.02523
- **[R41]** Li, X. et al. (2024). *Evaluating Real-World Robot Manipulation Policies in Simulation* (SimplerEnv). arXiv:2405.05941. https://arxiv.org/abs/2405.05941
- **[R42]** Ramos, F. et al. (2021). *RLDS: An Ecosystem to Generate, Share, and Use Datasets in Reinforcement Learning*. arXiv:2111.02767. https://arxiv.org/abs/2111.02767
- **[R43]** Foxglove. *MCAP Specification*. Accessed 12 July 2026. https://mcap.dev/spec
- **[R44]** Hugging Face. *LeRobotDataset v3.0 Documentation*. Accessed 12 July 2026. https://huggingface.co/docs/lerobot/lerobot-dataset-v3
- **[R45]** Rerun. *Rerun Documentation*. Accessed 12 July 2026. https://rerun.io/docs
- **[R46]** ROS Tooling. *rosbag2_storage_mcap*. Accessed 12 July 2026. https://github.com/ros-tooling/rosbag2_storage_mcap
- **[R47]** robomimic contributors. *robomimic Dataset and Experiment Documentation*. Accessed 12 July 2026. https://robomimic.github.io/
- **[R48]** Zhang, T. et al. (2025). *Robo-DM: Data Management for Large-Scale Robot Learning*. arXiv:2505.15558. https://arxiv.org/abs/2505.15558
- **[R49]** Shukor, M. et al. (2025). *SmolVLA: A Vision-Language-Action Model for Affordable and Efficient Robotics*. arXiv:2506.01844. https://arxiv.org/abs/2506.01844
- **[R50]** Kim, M. J.; Finn, C.; Liang, P. (2025). *Fine-Tuning Vision-Language-Action Models: Optimizing Speed and Success* (OpenVLA-OFT). arXiv:2502.19645. https://arxiv.org/abs/2502.19645
- **[R51]** Zhang, J. et al. (2025). *DreamVLA: A Vision-Language-Action Model Dreamed with Comprehensive World Knowledge*. arXiv:2507.04447. https://arxiv.org/abs/2507.04447
- **[R52]** Wang, F. et al. (2026). *World Models for Robotic Manipulation: A Survey*. arXiv:2606.00113. https://arxiv.org/abs/2606.00113

## Intervention quality, correction, safety, and reporting

- **[R53]** Raju, P. C. (2026). *Geometric Stability: The Missing Axis of Representations*. arXiv:2601.09173v5, 6 July 2026. https://arxiv.org/abs/2601.09173
- **[R54]** Pan, Y. et al. (2026). *VLA-Corrector: Lightweight Detect-and-Correct Inference for Adaptive Action Horizon*. arXiv:2607.01804. https://arxiv.org/abs/2607.01804
- **[R55]** Feng, X. et al. (2026). *Denoising Tells When to Replan: Denoising-Variance Adaptive Chunking for Flow-Based Robot Policies*. arXiv:2606.03847. https://arxiv.org/abs/2606.03847
- **[R56]** Lyu, M. et al. (2026). *ForesightSafety-VLA: A Unified Diagnostic Safety Benchmark for Vision-Language-Action Models*. arXiv:2606.27079v2. https://arxiv.org/abs/2606.27079
- **[R57]** Cui, R. et al. (2026). *LIBERO-Safety: A Comprehensive Benchmark for Physical and Semantic Safety in Vision-Language-Action Models*. ECCV 2026; arXiv:2606.23686v2. https://arxiv.org/abs/2606.23686
- **[R58]** Panpatil, S. et al. (2026). *EgoSafetyBench: A Diagnostic Egocentric Video Benchmark for Evaluating Embodied VLMs as Runtime Safety Guards*. arXiv:2607.00218. https://arxiv.org/abs/2607.00218
- **[R59]** Collins, G. S. et al. (2024). *TRIPOD+AI Statement: Updated Guidance for Reporting Clinical Prediction Models that Use Regression or Machine Learning Methods*. **BMJ** 385:e078378. https://www.bmj.com/content/385/bmj-2023-078378
- **[R60]** Moons, K. G. M. et al. (2025). *PROBAST+AI: An Updated Quality, Risk-of-Bias, and Applicability Assessment Tool for Prediction Models Using Regression or Artificial Intelligence Methods*. **BMJ**. https://www.bmj.com/content/388/bmj-2024-082505
- **[R61]** Prisoma repository. *PID Experiment 0 Findings*, snapshot reviewed 12 July 2026. https://github.com/sepahead/prisoma/blob/64bd881248463e7142d022bb95a5850bcf8fced2/findings.md
- **[R62]** W3C. *PROV-O: The PROV Ontology*; RO-Crate Research Object Crate specification. https://www.w3.org/TR/prov-o/ ; https://www.researchobject.org/ro-crate/
- **[R63]** Mitchell, M. et al. (2019). *Model Cards for Model Reporting*. FAT* / FAccT. https://doi.org/10.1145/3287560.3287596
- **[R64]** Gebru, T. et al. (2021). *Datasheets for Datasets*. **Communications of the ACM** 64(12):86–92. https://doi.org/10.1145/3458723
- **[R65]** Pineau, J. et al. (2021). *Improving Reproducibility in Machine Learning Research: A Report from the NeurIPS 2019 Reproducibility Program*. **JMLR** 22(164):1–20. https://jmlr.org/papers/v22/20-303.html

## July 2026 additions

- **[R66]** Lian, Q.; Yu, K.; Zhang, L. (2026). *Reflective VLA: In-Context Action Consequences Make VLAs Generalize*. arXiv:2606.25215. https://arxiv.org/abs/2606.25215
- **[R67]** Chen, X. et al. (2026). *A Definition and Roadmap for World Models*. arXiv:2607.06401, 7 July 2026. https://arxiv.org/abs/2607.06401
- **[R68]** Qu, H. et al. (2026). *Dual Latent Memory in Vision-Language-Action Models for Robotic Manipulation*. arXiv:2607.07608, 8 July 2026. https://arxiv.org/abs/2607.07608
- **[R69]** Zhou, J. et al. (2026). *TouchWorld: A Predictive and Reactive Tactile Foundation Model for Dexterous Manipulation*. arXiv:2607.07287v2, 9 July 2026. https://arxiv.org/abs/2607.07287
- **[R70]** Lyu, Q. et al. (2026). *LEEVLA: Seeing What Matters in Latent Environment Evolution for Vision-Language-Action*. arXiv:2607.08182, 9 July 2026. https://arxiv.org/abs/2607.08182
- **[R71]** Zhang, Y. et al. (2026). *Harness VLA: Steering Frozen VLAs into Reliable Manipulation Primitives via Memory-Guided Agents*. arXiv:2607.08448, 9 July 2026. https://arxiv.org/abs/2607.08448

## Repository ecosystem and causal/predictive design additions

- **[R72]** Prisoma repository, snapshot `64bd881248463e7142d022bb95a5850bcf8fced2` (12 July 2026). Root workspace, `.gitmodules`, `.ncp-consumer`, README, and `crates/ncp-observer`; `pid-rs` submodule shown at `8a5a9dda601556443f956a2fba164cccc913ed2e`. https://github.com/sepahead/prisoma/tree/64bd881248463e7142d022bb95a5850bcf8fced2
- **[R73]** `sepahead/pid-rs`. *Shared-exclusions partial information decomposition and mutual-information estimators in Rust*. Prisoma-pinned revision `8a5a9dda601556443f956a2fba164cccc913ed2e` and post-pin main revision `70b45f7b75fac06777ea215a73df01209490311a`, accessed 12 July 2026. The pinned revision records a semi-analytic additive-Gaussian oracle for continuous redundancy, bit-faithful discrete SxPID reference checks, cross-estimator atom-bias caveats, high-dimensional/dependence warnings, and that reproducible external continuous cross-validation was pending. The later main revision reports a committed fixture generated with the authors’ public `csxpid` implementation at pinned commit `7bb984611a422cf7944ece68993fe3a27e2eadec`, agreement within `1e-12` nats after recorded unit conversion, fail-closed population-support contracts, and stronger structured provenance. These later changes are not inherited by the reviewed Prisoma submodule pin and do not establish high-dimensional VLA application validity. https://github.com/sepahead/pid-rs/tree/8a5a9dda601556443f956a2fba164cccc913ed2e ; https://github.com/sepahead/pid-rs/commit/70b45f7b75fac06777ea215a73df01209490311a
- **[R74]** `sepahead/NCP`, immutable release `v0.8.0`, wire 0.8. *Neuro-Cybernetic Protocol*. Accessed 12 July 2026 (reviewed snapshot pinned `v0.7.1`/wire 0.7; the repository has since migrated to `v0.8.0`/wire 0.8). Prisoma is described as a read-only observer; default/open action-plane security limitations are documented. https://github.com/sepahead/NCP/tree/v0.8.0
- **[R75]** `sepahead/galadriel`. *Fail-closed cross-sensor statistical-consistency monitoring in safe Rust*. Accessed 12 July 2026. Pins `pid-rs` and NCP and explicitly states compilation is not live-integration evidence. https://github.com/sepahead/galadriel
- **[R76]** `sepahead/crebain`. *Multi-UAV simulation and airspace-awareness research testbed*. Accessed 12 July 2026. Contains dormant, off-by-default NCP surfaces and no always-on Crebain↔Engram loop; no direct Prisoma reference was found in the reviewed public repository. https://github.com/sepahead/crebain
- **[R77]** `sepahead/manwe`. *Airspace perception research workbench*. Accessed 12 July 2026. The repository states that it does not ship a drop-in adapter for Prisoma and identifies schema, tensor, clock, frame, and statistical-assumption gaps. https://github.com/sepahead/manwe
- **[R78]** `sepahead/engram`. *Engram Neural Modeling Labs*. Accessed 12 July 2026. The public repository contains a placeholder README stating that code will be open sourced after publication. https://github.com/sepahead/engram
- **[R79]** `sepahead/melkor`. *Gaussian splatting pipelines and depth analysis for 3D reconstruction*. Accessed 12 July 2026. No direct Prisoma reference was found in the reviewed public repository. https://github.com/sepahead/melkor
- **[R80]** Prisoma repository. *WORLD_WARP_INTEGRATION.md*, snapshot `64bd881248463e7142d022bb95a5850bcf8fced2`; optional external world-model integration specification, not verified as implemented. https://github.com/sepahead/prisoma/blob/64bd881248463e7142d022bb95a5850bcf8fced2/WORLD_WARP_INTEGRATION.md
- **[R81]** Prisoma repository. *GAUSS_MI_INTEGRATION.md*, snapshot `64bd881248463e7142d022bb95a5850bcf8fced2`; status “Specification (Pre-Implementation)” and weighted KSG described as a heuristic requiring its own validation gate. https://github.com/sepahead/prisoma/blob/64bd881248463e7142d022bb95a5850bcf8fced2/GAUSS_MI_INTEGRATION.md
- **[R82]** `sepahead/cobot-atlas`. *3D mesh-generation pipeline and dataset*. Accessed 12 July 2026. Repository reports 2,024 GLB files in the hosted dataset. https://github.com/sepahead/cobot-atlas
- **[R83]** `sepahead/relief-atlas`. *10K+ 3D mesh assets for disaster relief and civil protection*. Accessed 12 July 2026. Repository reports 10,079 items and directs users to individual asset metadata for licensing. https://github.com/sepahead/relief-atlas
- **[R84]** `sepahead/cortexel`. Scientific-visualization project. Accessed 12 July 2026. No direct Prisoma reference was found in the reviewed public repository. https://github.com/sepahead/cortexel
- **[R85]** `sepahead` GitHub profile and six public repository-index pages. Accessed 12 July 2026; metadata for 174 public repositories were screened. The profile project graph is treated as architectural intention rather than executable integration evidence; anonymous GitHub code search required sign-in, so negative findings are bounded to public metadata and inspected repository surfaces. https://github.com/sepahead?tab=repositories
- **[R86]** `sepahead/haldir`. Public repository inspected 12 July 2026; insufficient public implementation metadata was available to establish a Prisoma relationship. https://github.com/sepahead/haldir
- **[R87]** Hernán, M. A.; Robins, J. M. (2020). *Causal Inference: What If*. Chapman & Hall/CRC. https://www.hsph.harvard.edu/miguel-hernan/causal-inference-book/
- **[R88]** Rubin, D. B. (1980). *Randomization Analysis of Experimental Data: The Fisher Randomization Test Comment*. **Journal of the American Statistical Association** 75(371):591–593. https://doi.org/10.1080/01621459.1980.10477512
- **[R89]** Imai, K.; King, G.; Stuart, E. A. (2008). *Misunderstandings Between Experimentalists and Observationalists about Causal Inference*. **Journal of the Royal Statistical Society: Series A** 171(2):481–502. https://doi.org/10.1111/j.1467-985X.2007.00527.x
- **[R90]** Chernozhukov, V. et al. (2018). *Double/Debiased Machine Learning for Treatment and Structural Parameters*. **The Econometrics Journal** 21(1):C1–C68. https://doi.org/10.1111/ectj.12097
- **[R91]** Kennedy, E. H. (2023). *Towards Optimal Doubly Robust Estimation of Heterogeneous Causal Effects*. **Electronic Journal of Statistics** 17(2):3008–3049. https://doi.org/10.1214/23-EJS2157
- **[R92]** Gerds, T. A.; Schumacher, M. (2006). *Consistent Estimation of the Expected Brier Score in General Survival Models with Right-Censored Event Times*. **Biometrical Journal** 48(6):1029–1040. https://doi.org/10.1002/bimj.200610301
- **[R93]** Vickers, A. J.; Elkin, E. B. (2006). *Decision Curve Analysis: A Novel Method for Evaluating Prediction Models*. **Medical Decision Making** 26(6):565–574. https://doi.org/10.1177/0272989X06295361
- **[R94]** Saito, T.; Rehmsmeier, M. (2015). *The Precision-Recall Plot Is More Informative than the ROC Plot When Evaluating Binary Classifiers on Imbalanced Datasets*. **PLoS ONE** 10(3):e0118432. https://doi.org/10.1371/journal.pone.0118432
- **[R95]** Park, S.; Li, W.; Oh, C.; Yeh, S.; Kira, Z.; Hagenow, M.; Li, S. (2026). *Hide-and-Seek in Trajectories: Discovering Failure Signals for VLA Runtime Monitoring*. arXiv:2605.30834, 29 May 2026. https://arxiv.org/abs/2605.30834
- **[R96]** Barber, R. F.; Candès, E. J.; Ramdas, A.; Tibshirani, R. J. (2023). *Conformal Prediction Beyond Exchangeability*. **The Annals of Statistics** 51(2):816–845. https://doi.org/10.1214/23-AOS2276
- **[R97]** `sepahead/brojapid-activationfunctions`. *BROJA Partial Information Decomposition analysis of neural activation functions*. Release lineage accessed 12 July 2026; uses the BROJA unique-information measure and cites the 2020 reproduction study. https://github.com/sepahead/brojapid-activationfunctions
- **[R98]** Mahmoudian, S. (2020). *[Re] Measures for Investigating the Contextual Modulation of Information Transmission*. **ReScience C** 6(3), article 2; code at `sepahead/mahmoudian-2020-rescience`. https://doi.org/10.5281/zenodo.3885793
- **[R99]** `sepahead/nest-simulator`. Public NEST simulator fork whose repository description points to feature branches for PID/information-theoretic work. Accessed 12 July 2026; no direct Prisoma adapter was verified. https://github.com/sepahead/nest-simulator
- **[R100]** Dong, Z.; Lin, Y.; Fang, J.; Zhou, J.; Ng, K. K.; Zhou, J. H. (2026). *BrainFIBRE: A Foundation Model via Information Decomposition for Brain Microstructure*. arXiv:2607.00573, 1 July 2026; ECCV 2026. https://arxiv.org/abs/2607.00573
- **[R101]** Zhang, H.; Lu, Y.; Wang, B.; Kang, X.; Kuo, Y.-L.; Cheng, Z.; Wang, M.; Jenkins, O. C. (2026). *Foresight: Failure Detection for Long-Horizon Robotic Manipulation with Action-Conditioned World Model Latents*. arXiv:2606.23085, 22 June 2026. https://arxiv.org/abs/2606.23085
- **[R102]** Huang, B.; Li, X.; Wang, X.; Mi, L.; Hao, Z.; Wang, W.; Wu, H.; Li, K.; Liu, Y.; Cao, T. (2026). *ActProbe: Action-Space Probe for Early Failure Detection of Generative Robot Policies*. arXiv:2606.08508, 7 June 2026. https://arxiv.org/abs/2606.08508
- **[R103]** Huang, D.; Gu, A.; Zhang, C.; Zou, B.; Dong, W.; Cen, Z.; Wang, Y.; Zhang, H. (2026). *VLAConf: Calibrated Task-Success Confidence for Vision-Language-Action Models*. arXiv:2605.29605, 28 May 2026. https://arxiv.org/abs/2605.29605
- **[R104]** Lee, Y.; Har, D. (2026). *Perturbation-Based Uncertainty for Failure Detection in Vision-Language-Action Models*. arXiv:2606.20754, 18 June 2026. https://arxiv.org/abs/2606.20754
- **[R105]** Mahato, D. T.; Ren, R. (2026). *Early Warning Signals for OpenVLA Failure under Visual Distribution Shift*. arXiv:2606.29699, 29 June 2026. https://arxiv.org/abs/2606.29699
- **[R106]** Curth, A.; van der Schaar, M. (2023). *In Search of Insights, Not Magic Bullets: Towards Demystification of the Model Selection Dilemma in Heterogeneous Treatment Effect Estimation*. ICML 2023, PMLR 202:6623–6642. https://proceedings.mlr.press/v202/curth23b.html
- **[R107]** van der Laan, L.; Ulloa-Pérez, E.; Carone, M.; Luedtke, A. (2023). *Causal Isotonic Calibration for Heterogeneous Treatment Effects*. ICML 2023, PMLR 202:34831–34854. https://proceedings.mlr.press/v202/van-der-laan23a.html
- **[R108]** Chen, H.; Aebersold, H.; Puhan, M. A.; Serra-Burriel, M. (2026). *Machine Learning Methods for Estimating Personalized Treatment Effects—Insights on Validity from Two Large Trials*. **American Journal of Epidemiology**. https://doi.org/10.1093/aje/kwag065
- **[R109]** Gupta, K. (2026). *How VLAs Fail Differently: Black-Box Action Monitoring Reveals Architecture-Specific Failure Signatures*. arXiv:2605.28726, 27 May 2026. https://arxiv.org/abs/2605.28726
- **[R110]** Gu, Q.; Ju, Y.; Sun, S.; Gilitschenski, I.; Nishimura, H.; Itkina, M.; Shkurti, F. (2025). *SAFE: Multitask Failure Detection for Vision-Language-Action Models*. arXiv:2506.09937, 11 June 2025. https://arxiv.org/abs/2506.09937
- **[R111]** Zheng, G.; Seenivasan, S.; Johnson-Roberson, M.; Zhi, W. (2026). *Rewind-IL: Online Failure Detection and State Respawning for Imitation Learning*. arXiv:2604.16683, 17 April 2026. https://arxiv.org/abs/2604.16683
- **[R112]** Francis-Meretzki, S.; Mutti, M.; Romano, Y.; Tamar, A. (2026). *Temporal Difference Calibration in Sequential Tasks: Application to Vision-Language-Action Models*. arXiv:2604.20472, 22 April 2026. https://arxiv.org/abs/2604.20472
---

# Appendix A. Minimal canonical event envelope

The following is illustrative. The repository schema remains authoritative only after conformance tests and versioning are implemented.

```json
{
  "schema_version": "prisoma.event/1.0",
  "run_id": "uuid",
  "event_id": "monotone-or-uuid",
  "event_type": "intervention.applied",
  "producer": {
    "component": "policy-adapter",
    "version": "git-sha-or-image-digest",
    "host_clock": "monotonic-clock-id"
  },
  "time": {
    "monotonic_ns": 0,
    "source_ns": 0,
    "episode_step": 0,
    "uncertainty_ns": 0
  },
  "causal": {
    "case_id": "case-id",
    "episode_id": "episode-id",
    "assignment_id": "assignment-id",
    "parent_event_ids": ["event-id"],
    "intervention_id": "intervention-id",
    "randomization_probability": 0.5
  },
  "artifact_refs": [
    {
      "uri": "artifacts/activations.zarr#tensor-key",
      "sha256": "...",
      "dtype": "float32",
      "shape": [1, 32, 4096],
      "semantic_site": "pre_action_fusion.layer_12",
      "preprocess_hash": "..."
    }
  ],
  "payload": {
    "target": "vision.region.object_3",
    "operation": "mask_with_matched_texture",
    "dose": 0.25,
    "sham": false
  }
}
```

Required properties:

- immutable event identity;
- explicit producer and schema version;
- monotonic and source time with synchronization uncertainty;
- case, episode, assignment, and causal parentage;
- content-addressed external tensors rather than giant inline arrays;
- exact representation site and transform hash;
- intervention dose and sham status;
- fail-closed validation for missing causal or provenance fields.

# Appendix B. Analysis-freeze checklist

Before opening a confirmatory holdout:

- [ ] causal diagram and target level frozen;
- [ ] unit of inference and cluster structure frozen;
- [ ] eligibility gates passed;
- [ ] treatment assignment and manipulation checks validated;
- [ ] all preprocessing fitted on training data and hashed;
- [ ] baselines, model capacities, and hyperparameter budgets frozen;
- [ ] minimum useful effect and primary protocol-specific score frozen;
- [ ] missingness, exclusion, reset-failure, and censoring rules frozen;
- [ ] multiplicity family and exploratory labels frozen;
- [ ] simulation-based design analysis passed;
- [ ] code, container, and environment digests recorded;
- [ ] holdout access audited;
- [ ] negative and positive controls passed;
- [ ] result interpretation table drafted before unblinding.

# Appendix C. Result-interpretation table

| Observed result | Permitted conclusion | Prohibited conclusion |
|---|---|---|
| Diagnostic predicts paired frozen-snapshot response | diagnostic is useful for algorithmic-sensitivity prediction under the declared clone/coupling contract | diagnostic atom is a physical mechanism or closed-loop effect moderator |
| Diagnostic predicts randomized closed-loop effect modification under effect-specific validation | diagnostic is useful for effect moderation in the evaluated regime | diagnostic atom is the causal mechanism or an observed individual effect |
| PID beats full baseline set and replicates | PID adds conditional empirical value under named measure/estimator | PID is universally superior or necessary |
| PID does not beat baselines | no demonstrated incremental value in the evaluated regime | PID theory is false |
| Probe decodes, intervention has no effect | availability–use gap under tested intervention validity | represented concept is never used anywhere |
| Estimator gate fails | abstain from the blocked quantitative claim | atom is zero or absent |
| Safety benchmark improves | evidence of benchmark-specific risk reduction | certification or deployment safety |
| Cross-embodiment relation replicates | transportability of the tested relation across named embodiments | embodiment invariance of raw representations |

# Appendix D. Repository integration evidence ledger

Create one version-controlled row for every claimed edge.

| Field | Meaning |
|---|---|
| edge ID | stable producer→consumer identifier |
| source / target | repositories and component paths |
| exact revisions | commit/tag/submodule/lockfile hashes |
| relationship claim | intended, specified, dependent, build-tested, end-to-end, replicated |
| evidence level | E0–E5 with date |
| data/control direction | observation, analysis artifact, command, bidirectional |
| authority | read-only, advisory, command-capable, safety-gating |
| schema / wire | version, contract hash, encoding |
| semantics | units, shapes, frames, clocks, missingness, labels |
| security | realm, authentication, ACL, encryption, threat boundary |
| fixtures | golden and adversarial fixture identifiers |
| conformance report | command, result, artifact hash |
| scientific impact | which estimand or benchmark the edge enables |
| independence | shared code/maintainer and correlated-error risks |
| license/provenance | code, model, data, and asset obligations |
| status caveat | strongest prohibited wording |
| owner / review date | accountable maintainer and expiry |

Evidence expires when either endpoint, schema, wire, model, or adapter revision changes. A new build may preserve E3 but E4 must be rerun whenever semantics or scientific operating conditions change.

# Appendix E. Causal and predictive preflight checklist

Before H1 execution:

- [ ] Protocol A or Protocol B is designated primary and their claims/endpoints are not blended;
- [ ] treatment versions, sites, doses, and baseline-state boundary are uniquely identified;
- [ ] primary moderators are provably pre-treatment and diagnostic capture is noninterfering;
- [ ] for Protocol A, clone state, cache/memory reset, RNG coupling, evaluation order, output metric, and Monte Carlo precision are frozen;
- [ ] for Protocol B, assignment probabilities and blocks are generated and archived before treatment;
- [ ] interference, carryover, and reset boundaries are tested;
- [ ] ITT outcome and treatment receipt are both recorded for Protocol B;
- [ ] policy, execution, and physical outcome families are separate;
- [ ] manipulation, specificity, positive-control, and placebo checks are frozen;
- [ ] response predictor or conditional-effect learner, effect-specific validation metric, and outer-fold scoring are locked; factual outcome fit alone is not used to select an effect model;
- [ ] no physical individual-treatment-effect proxy is used as observed truth;
- [ ] useful margin, calibration bins, allocation rule, testing hierarchy, and replication target are frozen.

Before H2 landmarking:

- [ ] time zero, horizon, eligibility, and prediction update schedule are frozen;
- [ ] all feature computations stop at the landmark;
- [ ] repeated landmarks and persistent-world groups stay in one fold;
- [ ] failure types, competing events, censoring, and missingness are defined;
- [ ] test prevalence and target prevalence are recorded;
- [ ] censoring model, calibration, and thresholds are trained only inside outer folds;
- [ ] proper score, calibration, warning time, and decision utility are frozen;
- [ ] external/temporal holdout remains untouched;
- [ ] recalibration data are distinct from final evaluation data;
- [ ] any conformal method records its calibration unit, exchangeability/shift assumptions, finite-sample correction, empirical coverage, set size/abstention, and subgroup/task coverage.

Before any transport claim:

- [ ] source and target populations are named;
- [ ] changed and invariant causal/measurement variables are listed;
- [ ] effect modifiers and support overlap are assessed;
- [ ] adapters pass frame, clock, schema, and outcome conformance;
- [ ] model/asset/dataset contamination and licensing are audited;
- [ ] claim language is bounded to the axes actually replicated.

# Appendix F. Ecosystem-specific experiment opportunities

These are optional experiments, ordered by scientific value rather than visual appeal.

1. **Protocol-fault observatory.** Feed a conforming read-only NCP producer through controlled delay, drop, reorder, duplicate, version-mismatch, disconnect, and security-profile conditions. Measure Prisoma’s detection, provenance, replay, and control noninterference. This can strengthen Paper A without requiring Engram.
2. **Cross-domain diagnostic transport.** Export temporally aligned Crebain or Manwe-style perception/fusion streams through an adapter and test whether H2 diagnostics retain calibration under a non-manipulation embodiment. The primary result is transport failure or success under named shifts, not a universal VLA claim.
3. **Independent-monitor comparison.** Compare Galadriel consistency signals, ordinary uncertainty/OOD signals, and Prisoma diagnostics on the same randomized faults. Treat shared `pid-rs` outputs as one method family, not independent votes.
4. **Asset-diversity stress test.** Use a quality-controlled, license-cleared, physically validated subset of cobot-atlas to create prespecified object-appearance and geometry shifts. Keep generated assets grouped by lineage to avoid near-duplicate leakage.
5. **Reconstruction uncertainty as nuisance.** Use Melkor-derived reconstruction quality as a measured nuisance/effect modifier. First ask whether it predicts diagnostic failure; do not jump to unvalidated uncertainty-weighted PID.
6. **World-model counterfactual support.** Compare generated scenes against simulator-ground-truth interventions under explicit support and realism metrics. WorldWarp/GauSS-MI remain separate exploratory work until their adapters and estimators pass E4 and Section 7 gates.
7. **Cross-measure mechanism fixtures.** Reproduce selected activation-function systems from the ReScience/BROJA lineage and compare qualitative mechanism discrimination under BROJA, shared-exclusions, MI/CMI, and intervention ground truth. Treat measure disagreement as a result; never equate atom labels or magnitudes across functionals.
8. **Neural-simulation producer trial.** After pinning a specific NEST-fork branch and documenting its delta from upstream, export a small neural-state fixture through a read-only NCP path. Evaluate clock/sequence semantics, provenance, replay, and noninterference before any neuroscience or embodiment claim.

None of these experiments is required for a successful thesis. Their value is that the same event and estimand contract makes heterogeneous embodied-agent investigations comparable without pretending their representations, clocks, action spaces, or causal targets are identical.

*End of canonical v12.5 proposal.*
