# PRISOMA: Counterfactual Fidelity Tomography for Embodied World Models

- **Document:** canonical `grandplan.md` — **docset v13.0** (living spec; prior versions remain in git history; the reviewed v12.5 bundle is immutable under `docs/reviews/2026-07-12-grandplan-v12.5/`)
- **Reviewed scientific base:** 2026-07-12
- **Current first-principles reconciliation:** 2026-08-16
- **Repository snapshot reviewed:** `sepahead/prisoma@64bd881248463e7142d022bb95a5850bcf8fced2`; second-round review bundle preserved at `docs/reviews/2026-07-12-grandplan-v12.5/`
- **Status:** world-model-first research specification plus a preserved, unfrozen v12.5 diagnostic claim family; not a preregistration or empirical result
- **Repo-truth note (post-review):** the reviewed snapshot pinned NCP `v0.7.1`/wire 0.7 and `pid-rs@8a5a9dd`/0.4.0; the repository has since migrated to NCP **`v0.8.0`/wire 0.8** and the exact `pid-rs` **0.9.0 post-tag review source at `796c11e70f009634b853dc4ada6f565563d82f51`**, seven commits after the `v0.9.0` review tag, so active implementation-status statements below use the current pins while references to snapshot `64bd881…` remain historical; the 2026-08-13 network refresh rechecked `pid-rs`, NCP, and Paper2Brain public main, and the 2026-08-16 follow-up rechecked `pid-rs`; the Paper2Brain Prisoma descriptor bytes remained unchanged; none of these reviews supplied scientific conformance; this 0.9 review surface makes no 1.x compatibility or registry/wheel publication promise
- **Seventh adversarial revision:** paired-estimand separation, causal-heterogeneity scoring repair, expanded 2025–2026 monitor/calibration/safety comparators, estimator-status reconciliation, post-pin `pid-rs` development review, full public-repository ecosystem audit, alarm-policy specification, and reference deduplication completed 2026-07-12
- **Eighth first-principles reconciliation:** bounded claim language, H1 protocol identity, H2 scoring-rule and censoring-estimator separation, full-policy H3 semantics, tested-response H4 language, and an updated primary-source review completed 2026-08-12. This reconciliation reopens review of the all-null diagnostic successor draft. It does not authorize holdout access.
- **Ninth frontier correction:** deployed-graph WAM taxonomy, causal simulator admission, planner qualification, and the low-overhead M4 path reconciled through 2026-08-13. The review does not promote a model or open any scientific gate.
- **Tenth thesis reset:** exact-fork world-model decisions, linked dynamics/appearance/decision contrasts, and an M4-first execution ladder became the primary program on 2026-08-13. The prior EC1/H1–H4 family remains a secondary unfrozen diagnostic program. This reset creates no scientific result.
- **Eleventh PID-method reconciliation:** the functional, output-coordinate, law, evaluator, estimator, transform, certifier, objective, and interpretation layers are now governed by the typed [`PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md`](PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md). The reconciliation preserves valid research extensions while forbidding cross-PID fallback, atom pooling, and evidence transfer without a mapping theorem. It creates no estimator, application, or publication result.

> **Thesis in one sentence.** Prisoma is a low-overhead, content-bound laboratory that tests whether an action-conditioned world model preserves declared reference-state consequences, and separately measured physical consequences where available, across exact simulator forks, matched mesh/3DGS observations, frozen policies, and closed-loop execution.

---

## Executive decision

The scientifically defensible project is **not** “apply PID to Vision–Language–Action (VLA)
models and interpret the atoms.” It is also not “show photorealistic Gaussian-splat rollouts.”
Both formulations skip the decision contract and the reference outcome.

Prisoma should instead make three separable contributions:

1. **Exact-fork decision evidence.** One immutable pre-action state produces either one ordered
   feasible action pool or one fully recorded adaptive search. Every forecast and score commits
   before reference labels. Every candidate runs from an independent restored branch. Only the
   selected candidate reaches canonical execution through the Agent Bridge.
2. **Linked fidelity tomography.** Prisoma separates reference dynamics, learned prediction,
   observation substrate, frozen-policy response, and selection. It links matched panels by the
   same fork and action identity. It does not force an additive error decomposition or call the
   reference simulator physical truth.
3. **Conditional diagnostic science.** Calibration, action sensitivity, geometry, attribution,
   MI, PID, and prospective failure signals can explain or predict failures. PID is one optional
   family. It becomes central only after its population, measure, estimator, and application gates
   pass.

The project remains valuable if continuous PID fails, if 3DGS adds no decision value, or if one
world model does not beat a kinematic baseline. A negative result is useful when it localizes the
failure boundary and preserves every denominator. A polished simulator or dashboard without a
defensible decision contract is not a scientific contribution.

### Primary world-model question

> **Under one exact fork and one frozen supported-action or search contract, when does an action-conditioned world model preserve a declared later reference-state outcome well enough to improve closed-loop decisions over strong non-model baselines? Does its candidate ranking also improve under a separately frozen secondary estimand? Which dynamics, observation-substrate, policy-response, or selector boundary explains each failure?**

The primary target is the complete decision system. It includes proposal, prediction, score,
selection, controller conversion, execution, and later outcome. Forecast quality and policy success
remain separate endpoints.

### Secondary diagnostic question

> **Can pre-outcome diagnostics predict forecast error, ranking failure, intervention response, or future failure across held-out task, scene, policy, and embodiment families?**

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

The novelty case in v10.7 is too broad. By August 13, 2026:

- the official ICLR record, verified on 2026-08-12, lists a 2026 paper that applies PID across
  26 large vision–language models, tasks, layers, and training dynamics [R18];
- Sensory PID conditions on language, decomposes audio–video contributions, uses modality-shuffling and instruction interventions, and tests PID-guided reweighting [R113];
- a July 2026 multimodal foundation-model paper already uses a self-supervised PID-guided objective with counterfactual modality dropping and swapping, further narrowing any generic claim of novelty for “PID in multimodal learning” [R100];
- VLA failure prediction already includes explicit information-theoretic signals and cross-domain evaluation, making Tri-Info a mandatory baseline [R25];
- runtime VLA monitoring and calibration now includes SAFE, Hide-and-Seek, Rewind-IL, architecture-stratified black-box action monitors, Foresight, ActProbe, VLAConf, perturbation-based uncertainty, activation-warning studies, and temporal-difference calibration; together they span supervised internal features, coarsely supervised temporal localization, action-chunk self-consistency, kinematic signals, world-model latents, one-class confidence, perturbation disagreement, conformal calibration, simulation, and real robots [R95, R101–R105, R109–R112];
- VLA diagnosis already combines representation tracing, attention knockout, causal masking, object-removal counterfactuals, sparse-feature intervention, activation steering, and closed-loop behavior tracing [R26–R31, R114–R116];
- recent work treats action normalization and controller conventions as part of the executable policy, and it shows that identical weights can yield different robot behavior [R117];
- recent monitor and orchestration work adds transient-risk aggregation and recovery systems, which narrows any broad monitoring or integration novelty claim [R118–R119];
- a new conditional-independence Gaussian information hierarchy has closed-form covariance-law
  quantities and sample-covariance plug-in estimators. It provides two-source redundancy plus
  multi-source unique-information and order-specific synergy quantities, but deliberately assigns
  no redundancy for three or more sources and is therefore not a complete higher-source antichain
  PID. It is not a drop-in validation of shared exclusions [R126];
- world–value–action policies now plan through future-state and trajectory-value latents, which further widens the policy-state and architecture transport problem [R127];
- August systems now span predictive-training auxiliaries, intended-future policies,
  action-conditioned predictors, and candidate planners [R128–R137];
- Flex-\(\pi\)'s future stream cannot attend its action stream, so its generated future is not an
  action-conditioned transition despite its “Causal Joint Generation” label [R128];
- current evidence favors hybridization rather than replacement: several “WAM” systems remove
  their future branch at deployment, while others retain a direct VLA interface [R129–R134];
- direct policies remain active baselines rather than a dead class. OpenVLA, Octo, \(\pi_0\),
  \(\pi_{0.5}\), SmolVLA, and OpenVLA-OFT span generalist, flow-policy, compact, and fine-tuned
  designs [R21–R24, R49–R50];
- ForeWAM and Rift replace iterative future rollout with one-pass, action-independent
  future-position state that conditions action generation. Rift also intervenes on that state and
  measures closed-loop response [R165–R166];
- action and representation-grounding audits show that plausible video, forward prediction, and
  task success can hide action-insensitive or physically wrong state [R138–R139, R143–R145];
- execution frameworks now add external task memory, progress checks, recovery, and adaptive
  replanning around finite-horizon WAMs [R146–R147];
- World Action Planner proposes, predicts, ranks, and selects candidate actions. CheckVLA instead
  predicts the consequences of already committed actions to verify execution and trigger repair
  [R167–R168];
- a real-time WAM deployment study separates model inference from command scheduling and reports
  that timestamp alignment is a prerequisite for useful asynchronous chunk execution [R148];
- new benchmarks explicitly separate apparent capability from action-grounded use, test controlled physical reasoning, expose shortcutting or memorization, question whether task success identifies mechanism, and add process-level safety costs and risk-exposure time rather than relying on binary success alone [R27, R32–R36, R56];
- robotics ecosystems already provide timestamped multimodal containers, standardized episodic datasets, cross-embodiment corpora, visualization, and replay [R42–R48].

WorldSimProbe already separates action calibration, source preservation, grounding, and interaction
dynamics [R174]. Gaussian-splat simulators already separate appearance from dynamics and test
policies under matched controls [R175–R176]. CoWAM already uses fixed candidate pools,
pre-label commitments, abstention, and oracle proposal ceilings [R151]. World Action Planner
already proposes, predicts, scores, and selects [R167]. These are minimum comparators, not
unoccupied claims.

Prisoma should claim novelty only for a narrower **linked protocol**:

1. one content-bound fork and action identity follows every panel;
2. the declared simulator supplies reference transitions without being called physical truth;
3. the same state and camera are rendered through mesh and 3DGS without changing physics;
4. one frozen policy exposes immediate observation sensitivity before trajectories diverge;
5. one action-conditioned model exposes forecast and ranking error on the same action pool;
6. randomized complete-policy rollouts test downstream decision value under a frozen M4 budget;
7. optional diagnostics, including PID, must add held-out value beyond simpler signals.

This combination is a new integration and test protocol only if the final systematic search finds
no equivalent. It is not a claim that exact forks, Gaussian splats, world models, candidate-set
evaluation, provenance, or selective intervention are individually new.

## 1.2 Why the project matters even if niche

Embodied policies are sequential systems in which perception, instruction conditioning, internal state, action generation, controller filtering, and physical dynamics interact. Aggregate success can hide distinct failure mechanisms. An intervention-grounded diagnostic substrate can:

- distinguish “the model represented the relevant fact” from “the tested intervention changed the action pathway”;
- locate whether a failure entered through perception, conditioning, memory, action decoding, control, or execution;
- test whether an explanation predicts behavior under controlled perturbation;
- compare architectures without pretending that unlike hidden states are the same variable;
- evaluate prospective early-warning signals without temporal leakage;
- produce reusable, auditable experiment records.

The infrastructure contribution is not a “PID viewer.” It is an **experiment-semantics layer** binding interventions, internal-state provenance, replay, and estimands to standard robotics data.

## 1.3 Contribution counterfactual

Every infrastructure feature must answer:

> What scientific result becomes possible, more reliable, or cheaper because Prisoma exists, compared with MCAP/ROS bags, LeRobot/RLDS, Rerun, and an ordinary experiment script?

A feature counts as a research contribution only if it is externally benchmarked on a measurable
axis. Examples include timestamp alignment, dropped-event detection, assignment integrity, replay
fidelity, registered provenance-field coverage, adapter effort, estimator abstention, and
cross-policy portability.

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

## 2.2 Four target families

### Policy decision target

Examples: action-token distribution, continuous action density, denoising trajectory, action chunk, or a declared low-dimensional functional of the policy distribution. This asks what is associated with the learned policy’s decision before downstream control.

### Executed-action target

This includes inverse kinematics, clipping, collision checking, force limiting, smoothing, latency compensation, and human override. It asks what information survives into actuation.

### Declared reference-state target

This target is a later state or outcome from one versioned simulator or another declared reference
system. It can use physical units and state semantics. It is not measured physical truth, and it
does not identify the corresponding real-system transition.

### Measured physical-outcome target

Examples: next-state change, object flow, contact state, safety cost, progress, or success. This target mixes policy behavior with controller and environment dynamics.

Every analysis must name its target. A claim about policy output cannot be generalized silently to physical outcome.

## 2.3 Three estimand classes

### Observational information

For sources \(S_1,S_2\) and target \(Y\),

\[
I_{P_{\mathrm{obs}}}(S_1,S_2;Y)
\]

is a functional of the observational distribution induced by policy, task mixture, intervention
mixture, sampling, preprocessing, and temporal aggregation. It measures dependence in that regime.
It does not identify natural pathway use or response to an untested intervention.

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

## 2.4 Availability, tested response, and closed-loop effect

A representation may encode a fact that downstream action generation ignores. For a task-relevant variable \(Q\), define:

1. **Availability \(A_Q\):** held-out decodability of \(Q\) from \(R\).
2. **Tested policy response \(T_Q^{(k)}\):** the effect of exact intervention construction \(k\) on a declared policy output.
3. **Closed-loop effect \(E_Q^{(k)}\):** the effect of construction \(k\) on executed behavior or a physical outcome.

The object of interest is

\[
G_Q^{(k)}=(A_Q,T_Q^{(k)},E_Q^{(k)}),
\]

not any component alone. High \(A_Q\) with near-zero \(T_Q^{(k)}\) is an availability–tested-response gap for construction \(k\). High \(T_Q^{(k)}\) with low \(E_Q^{(k)}\) can indicate controller compensation or environmental irrelevance. Neither pattern identifies unrestricted natural pathway use. Recent action-grounding and mechanistic VLA studies make this distinction central [R26–R36, R114–R117].

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

The primary causal analysis is intention-to-treat (ITT) with the recorded assignment probability. Verify that assignment was generated before treatment, could not be overwritten by the policy or operator, and has support in every prespecified analysis stratum. Report realized treatment counts and probabilities by task family, checkpoint, block, and dose. Empty or near-empty cells require a pre-authorized coarser estimand or abstention. A flexible model does not restore missing support.

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

Do not emit a `wibral_lineage` result identity. An author name does not identify a functional,
estimator, evaluator, transform, or objective. Use a provenance and estimand graph when specifying
or interpreting a method. Current reports bind a full-team functional ID, estimator revision,
source and target roles, fitted-transform receipts, units, and gate status. A future declared-law
or objective schema must also bind evaluation kind, input-law kind, aggregation scope, and any
composition. Related objects remain typed and non-substitutable:

1. Wibral et al.'s neural goal-function work motivates PID coordinates. It does not define the
   later MGW shared-exclusions functional or implement an arbitrary objective [R169].
2. Makkeh–Gutknecht–Wibral categorical shared exclusions defines a pointwise functional on
   categorical probability laws [R05]. Its informative and misinformative parts are formal
   surprisal components. They do not mean benefit/harm, truth/error, or honesty/deception.
3. The mereological and formal-logic work states the event and parthood semantics. It does not
   turn one estimator into another [R170–R171].
4. Schick-Poland et al. and Ehrlich et al. develop distinct general and continuous
   shared-exclusions objects and a kNN route under their declared measure, support, and gauge
   assumptions [R06–R07]. They are inspired by the same exclusion principle. They are not the
   categorical functional applied after binning.
5. A plug-in empirical-PMF or kNN estimator maps observations to an estimate of one named
   functional. A declared-law evaluator maps a specified law to that functional's value. Neither
   is the functional itself.
6. An infomorphic objective is a coefficient-weighted composition of declared PID atoms, often
   with conditional entropy, through a stated binning and estimation route [R19, R172]. It is not
   a new PID functional or estimator. The pinned count-law API does not certify arbitrary soft
   weighted laws, adaptive bins, stopped binning gradients, or training guards.

Functional identity does not identify one scalar output. Each lattice-PID result must also bind the
exact cumulative or Möbius-inverted quantity, antichain coordinate, pointwise or averaged scope,
averaging law, and net, informative, or misinformative component. A non-lattice hierarchy result
instead binds its exact named quantity and output index; it must not invent a lattice coordinate.
A cumulative lattice value is not its atom. A net atom is not either nonnegative component. Use
typed graph edges for definition, targeting, evaluation, implementation, recovery on a stated
domain, motivation, validation, and composition. Shared authors, notation, or lineage do not
create an alias or mapping theorem.

No result transfers between these objects without an explicit mapping theorem. In particular,
quantization does not establish a discrete-to-continuous limit or an equivalence with the
continuous Ehrlich formulation.

Prisoma's fitted categorical route has one exact identity. Fitted equal-width quantizers define
new categorical variables. The pinned
`pid-rs@796c11e` function `fitted_quantized_sxpid2_with_budget` then evaluates the two-source
averaged MGW functional on the resulting empirical count law. It emits informative,
misinformative, and net atoms in nats. This route is neither generic “discrete PID,” Williams–Beer
`I_min`, BROJA, continuous shared exclusions, nor an infomorphic objective.

Other PID measures may remain inside `pid-rs` for upstream research and compatibility. Prisoma
does not use Williams–Beer `I_min` or BROJA to define an active hypothesis, substitute for a failed
shared-exclusions estimate, rescue a result, or form an active sensitivity branch. Those measures
may appear only as clearly excluded historical context. Their atoms are never pooled, relabeled,
or treated as replicated shared-exclusions evidence.

[`PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md`](PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md)
is the normative method-selection appendix for this plan. Before a PID-related object can enter a
claim, it must identify the paper-defined functional, law class, evaluator or estimator route,
transform, row relation, source count and lattice or hierarchy output structure, units, support or
gauge, validation level, and application verdict. A declared-law evaluator and a sample estimator
are different routes. A certifier and a validation fixture are evidence about a route, not
additional PIDs. An infomorphic objective is a downstream composition.

Novel valid extensions are retained as typed research objects. Higher-source categorical MGW,
specified rational and binary64 laws, mixed or conditional proposals, temporal constructions,
manifold estimators, and objective compositions are not discarded merely because they are outside
the active H3 path. Each needs its own definition, domain, axioms, estimator or evaluator, oracle,
failure contract, and publication level. Failure at an application gate removes an extension from
that application claim. It does not erase valid mathematics, software, or negative evidence.

The repository’s status must be stated at four levels. First, its high-dimensional MI/coherence checks are **NO-GO**. Second, the current pinned `pid-rs` review source has semi-analytic low-dimensional additive-Gaussian continuous-redundancy oracle checks with closed-form pointwise terms and discrete SxPID reference agreement, so it is inaccurate to say that no estimator validation exists. Third, current Experiment 0 never compares shared-exclusions redundancy with a zero target: its binary-default sweep covers 12 scenario–dimension cells over three deterministic seeds (36 case results), while the one-seed run-log recipe covers the same 12 cells once. Both report the high-dimensional MI/coherence verdict as **NO-GO**, atom-measure validation as `not_adjudicated`, and atom-estimator validation as `blocked`; the strict path gates only the curated analytic-MI band and reports atoms separately. Fourth, the repository now pins the exact `pid-rs` 0.9.0 post-tag review source at `796c11e70f009634b853dc4ada6f565563d82f51`, which includes the reproducible fixture against the authors’ public `csxpid` implementation, agreement within `1e-12` nats on that fixture, fail-closed continuous-support contracts, and report-first provenance. The migration changes what is runnable and which samples abstain; it does **not** validate dependent, high-dimensional VLA embeddings or make a 1.x compatibility promise. Continuous atoms on real embeddings therefore remain blocked [R61, R73].

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
- atoms do not predict the pre-outcome frozen paired algorithmic response or randomized closed-loop effect-modification endpoint;
- the episode/task-family sample size cannot support stable inference.

Infrastructure and non-PID science continue under these outcomes.

---

# 4. Confirmatory claim-template registry

Docset v13.0 has two registries with different roles. **W1 and W2 are the proposed primary thesis
claims.** EC1 and H1–H4 are the preserved v12.5 diagnostic claim family. No item is frozen.

The thesis has at most two primary scientific claims. Engineering acceptance claims remain
separate. A real study must freeze each population, action support, outcome, identification
assumption, comparator, resource budget, useful margin, multiplicity rule, and decision rule before
holdout access.

## W1 — supported forecast fidelity

**Question.** Under supported randomized actions from valid reset states, does a frozen
action-conditioned model improve proper prediction of one declared later reference-state outcome
beyond current-state, current-state-plus-action, and simple dynamics predictors of that same
outcome? If a separate physical system is measured, that physical-outcome estimand must be frozen
and reported separately. Candidate ranking is secondary unless it is promoted before holdout
access.

**Primary unit.** One immutable pre-action fork and one action assignment drawn from the frozen
supported design. Every method receives the same history, language, observation, declared action
fields, controller contract, and deadline. All predictions and abstentions commit before the shared
reference label. Candidate ranking is a separate secondary unit. It uses either one fixed ordered
pool or one fully recorded adaptive search trace.

Freeze whether the randomized treatment and predictor query use the policy proposal
\(A^\pi\), a controller-resolved \(A^{\mathrm{exec}}\), or both. Randomize at one declared level.
Bind the conversion receipt and retain every mismatch, override, hold, or rejection in the
intention-to-treat result. Every matched predictor receives the same declared action fields.

**Primary estimand.** Freeze one later outcome (Y_{i,a,h}), one predictive object, one scoring
rule, and one supported action distribution. If the object is a distribution, use a strictly proper
score. If it is a point functional, use a loss that is strictly consistent for that functional.
Orient the out-of-sample score difference so positive values favor the learned model over the
strongest frozen matched-access baseline. Ranking accuracy, Policy Top-1, pairwise inversion,
calibration, and action sensitivity are secondary unless one is promoted before holdout access.

**Required controls.** Include current-only, current-plus-action direct prediction of the frozen
outcome, known or kinematic dynamics, no-future, action-shuffled, future-shuffled, and
repeated-query drift. When candidate selection is evaluated, also report the oracle best among
the exact committed candidates and its denominator. Apply the same predictive object and score
whenever a control enters the primary contrast. Preserve failed, reversed, no-op, and low-support
actions. Abstain outside the frozen action-support envelope. A plausible video or latent is not a
passing forecast.

**Decision.** Orient improvement so positive is better. Success requires the one-sided lower
confidence bound to exceed a positive useful margin under clustered, dependence-aware uncertainty.
Freeze support, calibration, subgroup, and M4 resource gates as non-rescuable co-primary
conditions.

## W2 — closed-loop decision value under a local resource budget

**Question.** Under a fixed M4 Max observation-to-command deadline, does the complete
propose–predict–score–select policy improve episode-level task cost or success over the strongest
same-budget non-model policy?

**Unit and assignment.** Randomize complete deployed policies across independent reset blocks.
One arm cannot borrow labels, candidates, extra proposals, or extra compute from another arm.
Report intention-to-treat first. Keep fork-level selector evaluation separate from episode-level
policy evaluation.

**Primary estimand.** Freeze one episode-level cost, success, or constrained utility difference.
Do not sum fork-local candidate-set regret and call it deployed-policy regret. Report oracle
proposal headroom, selector error, and proposal failure as separate denominators.

**Required arms.** Include a frozen nominal policy, a same-budget multiple-proposal direct policy,
direct action-value or cost prediction, simple dynamics or kinematic MPC, the learned predictor with
selection disabled, and the complete learned selector. Match proposal count, controller, action
support, observation history, deadline, and evaluation.

**Low-overhead contract.** Freeze p50, p95, and p99 observation-to-command latency, peak unified
memory, energy or power proxy, missed-deadline rate, and abstention/fallback rate. “Runs on M4” and
“low overhead” are not results. Each requires a bound and a measured receipt.

## W3 — linked fidelity tomography (secondary or exploratory)

W3 localizes W1/W2 failure through linked matched panels. It is not a third independent causal
claim and it is not an additive decomposition.

1. **Reference-transition panel:** run the same fork and action through the declared simulator.
2. **Observation-substrate panel:** render the exact same state trajectory and camera through mesh
   and 3DGS paths. Rendering must never alter collision geometry or dynamics.
3. **Frozen-policy panel:** give matched observations to the same frozen policy. Estimate immediate
   paired action response on the common trajectory. Estimate downstream policy effects only in a
   separate randomized complete-policy rollout after trajectories can diverge.
4. **Learned-model panel:** query the same candidate pool, commit before labels, and link forecast
   errors to ranking, selection, and later policy outcomes.

Every W3 row binds one authoritative simulator state and one pure observation function. The state
receipt includes all dynamic bodies, joints, contacts, constraints, and controller state needed for
replay. A representation manifest maps each body and link to collision, mesh, and 3DGS assets. It
states how rigid, articulated, deformable, background, and robot elements move.

The camera receipt binds intrinsics, distortion, crop, resolution, near and far planes, exposure,
color transform, tone mapping, shutter, motion blur, world pose, and frame time. It also binds
drop, reorder, and synchronization rules. Matched policy queries reset recurrent state, KV cache,
history, and random state. Asset and reconstruction lineage define scene-family holdouts.

The simulator is a declared reference, not physical truth. Mesh and 3DGS are representation
treatments. Without paired real observations, their difference is not “rendering error.” Use linked
contrasts rather than a nominal full factorial unless every Cartesian cell has a comparable
estimand.

## Preserved diagnostic claim family

The v12.5 EC1/H1–H4 family remains useful for capture conformance, intervention response,
prospective failure, PID value, and attribution. It is secondary to W1/W2 in v13.0. Its current
machine-readable v1/v2/v3 governance files retain their identifiers and remain unfrozen.
They must not be described as the active thesis registration.

If a later diagnostic study activates the preserved family, its old selection rule permits H1,
H2, and exactly one of H3 or H4. H3 and H4 cannot both be confirmatory in that analysis family.

## EC1 — registered capture–replay fidelity and fault detection

For each preregistered policy–environment adapter, Prisoma records a finite declared inventory of
intervention, state, outcome, and temporal variables. It is tested against a finite, versioned
acceptance universe. That universe binds the supported adapter/capability matrix, valid-case controls, fault classes and
their injection distribution, oracle labels, conventional-script/container comparators, replay
endpoints, exact or tolerance-bounded margins, false-positive endpoint, uncertainty method,
multiplicity rule, and pass/fail decision. Every registered fault–adapter pair has a separately
estimated absolute sensitivity floor and is a mandatory acceptance gate: a distribution-average
sensitivity cannot substitute for, or rescue failure of, any pair, including a rare critical
fault. EC1 can therefore support only the claim that every registered pair passed its frozen floor
and the registered replay endpoints met their margins under the tested design; unregistered fault
classes and environments are not evaluated. A second
structurally different adapter and an external reproduction are required for the broad
infrastructure claim. Do not infer EC1 from implementation or happy-path fixtures alone.

## H1 family — pre-treatment diagnostics predict a named intervention response

**Question.** Do diagnostics available before intervention identify cases in which a policy will be sensitive to a declared manipulation?

**Mandatory design fork.** Two scientifically valid but non-interchangeable protocols are available. Every study must designate one before opening the confirmatory holdout. Every result must say **H1-A** or **H1-B**. “H1 passed” alone is prohibited because the protocols have different units, estimands, outcomes, and permitted conclusions.

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

**Protocol A analysis.** Predict \(S_i\) out of sample with task-family-blocked nested resampling.
Compare a design-only model, matched non-PID diagnostic models, and the same model plus the
candidate diagnostic family. Use identical outer splits and comparable capacity and tuning
budgets. Use a prespecified proper
predictive score—such as negative log predictive density or CRPS for a distributional predictor,
or squared error for a point predictor—and absolute calibration across bins of predicted
response. Bin boundaries are either prespecified on an outcome-independent scale or learned only
from outer-training predictions and then applied unchanged to held-out cases; held-out observed
responses never define or merge bins. Propagate replicate-level Monte Carlo uncertainty rather
than treating a noisy estimate of \(S_i\) as exact. A causal forest or treatment learner is
unnecessary when both algorithmic responses are directly computed.

### Protocol B estimand

For binary randomized assignment \(J\),

\[
\tau(d)=\mathbb{E}[Y(1)-Y(0)\mid D=d],
\]

or a prespecified low-dimensional projection/partition of \(D\). Report the population-average ITT effect even when heterogeneity is absent. For multiple doses, freeze a dose-response contrast or monotonicity functional rather than selecting the most favorable dose after inspection.

**Protocol B outcomes.** Keep three families distinct: (i) post-assignment policy-output change, (ii) executed-action/controller change, and (iii) progress, safety cost, or task outcome. Matched exogenous seeds are permitted only when the simulator’s reset and random-draw coupling preserve the target intervention; otherwise use randomized repeated trials. A policy-level effect cannot be silently upgraded to a physical-outcome effect.

**Protocol B analysis.** Fit the treatment-response learner inside nested, task-family-blocked cross-fitting. Compare a design-only effect model, matched non-PID diagnostic models, and the same model plus the candidate diagnostic family. Use identical outer splits and comparable capacity and tuning budgets. Freeze all inclusion and selection rules before the holdout. Candidate models may include a prespecified interaction model, causal forest, R-learner, doubly robust learner, or deliberately simple score, but model class and tuning budget are frozen before the outer holdout [R89–R91]. Because individual effects are unobserved, do not choose or validate an effect model solely by factual-outcome prediction: prognostic fit can improve while effect ranking worsens. Freeze a causal validation stack consisting of (i) a cross-fitted R-loss or doubly robust effect-prediction loss with nuisance diagnostics, (ii) causal calibration using train-defined prediction bins and held-out randomized contrasts, (iii) a rank-weighted average-treatment-effect or equivalent prioritization statistic when ranking is a goal, and (iv) policy value/regret under known assignment probabilities. Factual-outcome proper loss is only a secondary outcome-model diagnostic [R106–R108]. Do not score against naive same-data “individual effects”: physical individual treatment effects are not jointly observed.

Exactly one effect-specific member of that stack is frozen as the H1-B primary endpoint. The
remaining members are ordered in a prespecified gatekeeping hierarchy, with a complete familywise
error rule and explicit consequences for failure at each gate. The population-average ITT,
assignment integrity, engagement, and nuisance diagnostics remain mandatory regardless of which
effect-specific endpoint is primary; an attractive secondary endpoint cannot rescue a failed
primary endpoint.

A simple prespecified working model remains useful,

\[
Y_{ijk}=\alpha+b_{f(j)}+c_i+\beta J_{ijk}+\gamma^\top D_{ij}
+\delta^\top(J_{ijk}D_{ij})+\eta^\top X_{ij}+\varepsilon_{ijk},
\]

but coefficient significance is not the success criterion. Flexible and simple models use identical outer splits and comparable tuning budgets.

**Held-out endpoints.** Freeze endpoints separately by protocol.

- **Protocol A:** improvement in the primary predictive score for \(S_i\); calibration of predicted algorithmic response; stability across clone order, coupling, and valid output metrics; and decision value if the response prediction selects a diagnostic intervention or fallback.
- **Protocol B:** improvement in the one frozen primary effect-specific endpoint—chosen from causal
  effect-prediction loss, causal calibration, a prespecified rank/prioritization statistic, or
  value/regret of a prespecified treatment-allocation rule—with the other endpoints tested only
  through the frozen hierarchy; and randomization-based or cluster-aware uncertainty for the
  global no-effect-modification null. Factual-outcome fit alone is never primary. A model that
  predicts outcomes well but fails the primary effect-specific check does not pass H1.

**Prohibited endpoints.** Do not correlate diagnostics with a same-data per-case difference from non-cloned physical episodes and call it treatment-effect prediction. Do not discover and evaluate subgroups on the same units. Do not blend Protocol A and B scores into one endpoint or describe a Protocol A success as evidence of closed-loop robustness.

**Null.** Diagnostics do not improve the locked held-out endpoint beyond design variables and strong non-PID baselines by the minimum useful margin.

**Primary decision contract.** Canonicalize improvement so positive values favor the diagnostic-
augmented model. Freeze one positive minimum useful margin before holdout access. Success requires
the one-sided lower confidence bound for the primary improvement to exceed that margin under the
frozen dependence-aware uncertainty and multiplicity procedures. Noninferiority, equivalence, a
nonsignificant difference, or an attractive secondary endpoint cannot establish success. Protocol
A must also freeze the response functional, proper score, matched-access comparator, calibration
acceptance rule and failure consequence, and finite-benchmark or replication scope. Protocol B
must also pass the complete effect-validation stack, overall ITT, assignment, engagement,
specificity, and nuisance checks. It requires directional replication in another task family or
policy. Factual-outcome fit cannot establish H1-B success.

**Success and permitted language.** Protocol A success permits the bounded statement that
pre-treatment diagnostics improve prediction of the declared frozen-snapshot algorithmic response
by more than the frozen useful margin in the evaluated regime. A claim about embodied closed-loop
effect moderation requires Protocol B and every mandatory design check above. A significant
interaction without held-out utility, or a Protocol A result without a closed-loop test, is
insufficient for a physical-mechanism claim.

## H2 — diagnostics improve prospective, censoring-aware failure prediction

**Question.** Do signals available by landmark \(t_0\) predict a prespecified future failure type within horizon \(h\), beyond strong baselines, under the task mix and prevalence relevant to use?

**Unit.** Episode landmark. All landmarks from an episode, case seed, or persistent world state remain in one outer fold. Repeated landmarks are handled as longitudinal observations rather than independent rows.

**Time zero and eligibility.** For each landmark, freeze eligibility, feature cutoff, prediction horizon, competing events, and censoring rule before reading future data. A signal whose computation uses a future normalization constant, full-episode transform, final success label, or post-landmark intervention is leakage.

**Predictors.** Only timestamped data at or before \(t_0\). A global dataset PID atom is not an episode feature. Local information scores require a train-reference distribution, cross-fitting, an eligibility verdict, and a frozen episode/window aggregation. Missingness indicators may be predictors only when they would be observable at deployment.

**Outcome.** Use a mutually exclusive failure ontology where possible. For “failure by \(h\),” success, timeout, human takeover, reset, and other failure modes may be competing events rather than ordinary negatives. Report cause-specific and cumulative-incidence targets when the distinction changes the scientific question.

**Mandatory baselines.** Base rate; policy entropy/action uncertainty; ensemble or stochastic-pass disagreement when available; action smoothness/chunk inconsistency; state/dynamics prediction error; OOD distance; progress; and a capacity-matched learned latent baseline. Reproduce, or implement an input/supervision-matched analogue of, the strongest applicable families: SAFE-style supervised internal-state detection; Tri-Info signals; Hide-and-Seek temporal localization; ActProbe action-chunk magnitude and temporal-consistency signals; Rewind-IL/TIDE inter-chunk discrepancy; architecture-stratified black-box action features such as reversal, jerk, momentum coherence, and stall; VLAConf-style one-class internal-representation confidence; perturbation-induced action disagreement; activation-probe warning signals; Foresight-style or CheckVLA-style action-conditioned world-model verification; and temporal-difference success calibration when action probabilities are available [R25, R95, R101–R105, R109–R112, R168]. Before outcome access, freeze either exact implementations or a rule for selecting each input- and supervision-matched analogue. Outcome-informed selection of the “strongest” baseline is prohibited. Add simple time/task/checkpoint and action-head-family indicators so complex diagnostics do not receive credit for prevalence or architecture drift. Compare methods at matched information access, supervision, action resampling, external-model use, latency, and compute; otherwise report a cost–accuracy–timeliness Pareto frontier rather than a misleading single ranking.

**Validation.** Use leave-task-family-out, temporal, or external validation matching intended use. Hyperparameters, transforms, feature selection, censoring models, and calibration are fitted inside nested training folds. A deployment claim requires an untouched external or later-time test; random frame splits are prohibited.

**Primary scoring contract.** Freeze exactly one H2 primary scoring contract before the
confirmatory holdout. With complete follow-up for the full eligible population, fixed-horizon
binary log loss or Brier loss is proper for the declared horizon risk. Under censoring, state
whether the endpoint is a censoring-adjusted score for that horizon risk, an estimator of the
complete-data risk, or a score for a fuller event-time-and-type law. A forecast-independent
conditional IPCW Brier construction can properly score a scalar horizon risk on its identifiable
region when conditional independent censoring, positivity, and the censoring law are correct.
Estimated censoring laws need separate fitting and validation. Forecast-dependent weights can
break propriety [R92, R120–R122]. Thus, neither the label `IPCW` nor the label `Brier` establishes
the score's role or validity. A right-censored likelihood is aligned only when the full
event-time-and-type distribution is the frozen prediction object. It cannot replace a fixed-horizon
risk score merely because both use survival data. Competing risks require a mutually exclusive
event ontology and a score matched to the declared cause-specific risk, cumulative incidence, or
joint event-time-and-type law. A horizon-specific censoring-adjusted score can target a scalar
named-cause risk. A full competing-risk likelihood requires every modeled event type. Freeze the
identifiable region and assumptions [R123]. Do not use a generic integrated or time-dependent
Brier label as proof of propriety. Freeze the prediction object, estimand, score, censoring
construction, weight or censoring-law estimator, minimum useful margin, and uncertainty procedure
as one contract.
For a dynamically updated confidence sequence, also test temporal calibration or a locked
sequential scoring analogue. Do not average unrelated per-step calibration numbers. Temporal-
difference calibration is a comparator, not a guarantee of deployment validity [R112]. Secondary:
precision–recall AUC at stated prevalence; calibration intercept/slope
and reliability curve; event-level sensitivity at fixed false-alarm burden; alarms per episode or
operating hour; a lead-time distribution that explicitly retains undetected failures; and one
decision-utility analysis [R93–R94]. Decision utility is secondary gatekeeping or descriptive and
can never rescue failure of the one primary scoring contract. Conditional lead time among
detected failures alone is selection-biased and cannot rank monitors. ROC AUC alone is inadequate.
Converting repeated risk scores into alarms requires a frozen alarm policy—threshold,
persistence/debounce rule, refractory period, event-matching window, reset behavior, and
missing-score handling—tuned only in training data; otherwise false-alarm and lead-time comparisons
are underdefined. If a conformal warning set or threshold is used, also report empirical coverage,
set size or abstention, false-alarm burden, and subgroup/task coverage. Report uncertainty clustered
at the highest independent unit and publish independent episode, event, and task-family counts.

**Shift, conformal validity, and recalibration.** Evaluate performance by task family, policy checkpoint, failure type, sensor quality, and prevalence. Standard split-conformal marginal coverage relies on exchangeability; task, temporal, policy, or embodiment shift does not preserve that guarantee automatically. Use a method whose weighted, group-conditional, online, or sequential assumptions match the design, or describe target-domain coverage as empirical rather than guaranteed [R96]. When the test prevalence is artificial, report both sampled-population and target-prevalence metrics. Recalibration or conformal recalibration on target data is a separate procedure and data split, not a hidden test-set refit.

**Null.** Adding the diagnostic family does not improve the one frozen primary scoring contract by the
minimum useful margin over the strongest matched-access baseline under the locked external-validity
target.

**Success.** The diagnostic must exceed the frozen minimum useful improvement under that primary
scoring contract
under the prespecified uncertainty procedure and replicate on the declared external task family or
later-time block. Calibration must remain within its frozen tolerance or pass a prespecified
recalibration procedure on a separate split; the frozen alarm policy must meet its false-alarm and
warning-time actionability criteria; and prespecified subgroup degradation bounds must hold.
Failure of the primary endpoint cannot be rescued by calibration, utility, or another secondary
metric. A useful monitor may still fail to identify a mechanism; predictive and mechanistic claims
remain separate.

## H3 — a PID-with-abstention policy adds full-target-population value

H3 activates only after population, measure, estimator, and application gates in Section 7. The PID configuration is a tuple—not a generic method—containing source/target definitions, sampling law, measure, dimensionality, scaling/projection, estimator, neighborhood parameters, dependence treatment, local-score construction, and abstention rules.

**Source–target ancestry gate.** Freeze one target-specific prediction landmark and one
tensor-ancestry record before outcome access. The landmark must precede target realization or
availability. Every source must be available at that landmark. No source may contain the target,
read a post-landmark observation, or depend on the target by construction. In particular, a state
computed as \(q(F \mid H,L,A^\pi)\) is not an admissible source for PID whose target is that same
\(A^\pi\). Cross-fitting cannot remove this exact target injection. A controller command,
executed action, later declared reference-state outcome, or separately measured physical outcome
can be downstream without being contained in the source. For any such target, give the matched
baseline that exact candidate action. A command target can test controller or execution
prediction, not physical forecast validity. Use a separately measured physical outcome for that
claim, or use the state in the frozen class-D/E comparison in Section 9.2. A future-supervised
model may supply a source only through its deployed inference path. Bind
that source to the target-specific prediction landmark. Also bind the exact runtime inputs and
their maximum observation time.

Compare capacity- and tuning-budget-matched nested models:

- \(M_0\): design variables, assignment terms, base rate, and naive baselines;
- \(M_1\): \(M_0\) plus MI/CMI, co-information or Shannon-invariant screens, uncertainty, temporal, geometry, attribution, OOD, progress, and learned features;
- \(M_2\): \(M_1\) plus preregistered PID features generated only from training-reference fits that passed all gates.

**Common comparison population (frozen policy).** H3 uses full-target-population scoring with
exact same-fold \(M_1\) substitution whenever \(M_2\) abstains. Before any outer-holdout outcome is accessed,
bind one canonical ordered ledger of unique candidate IDs and inherit, unchanged, the target
population, unit, cluster, eligibility/time-zero rule, sampling weights, and outer split from
exactly one active parent H1 or H2 estimand. For every ledger ID, \(M_1\) must emit one held-out
prediction or decision. \(M_2\) must emit either one PID-augmented held-out output or one typed
abstention. Define the deployed \(M_2\) policy by
\[
\widetilde M_2(i)=
\begin{cases}
M_2(i), & \text{for a clean produced output, or an allowlisted warning with use-output disposition},\\
M_1(i), & \text{for an abstention, or a warning with the legacy fallback disposition}.
\end{cases}
\]
Any warning with a block disposition makes the affected primary comparison unavailable. An
unknown or malformed warning follows the fail-closed rule below.
The substituted value is the exact recorded \(M_1\) output for the same candidate and outer fold; it is not
a numeric placeholder for an abstained PID estimate. Score \(M_1\) and \(\widetilde M_2\) with the
same ordered IDs, outcomes, weights, clusters, folds, and primary endpoint. The primary denominator
is the complete frozen target ledger, never the subset on which PID happened to be eligible or
numerically successful. Under a decomposable loss, each substituted candidate contributes exactly
zero paired incremental score while retaining its target weight; for a nondecomposable endpoint,
apply the locked endpoint functional to both complete paired ledgers. Missing, duplicate, extra,
or misaligned IDs; a missing \(M_1\) output; an
untyped or internally contradictory \(M_2\) state; a substitution mismatch; post-outcome eligibility;
or drift in outcome, weight, cluster, fold, endpoint, or population binding blocks the primary H3
result. Publish content hashes for the frozen target ledger and the per-candidate paired-scoring
receipt. Report produced, produced-with-warning, and abstained counts by reason plus the substitution
count. An eligible-only comparison may be reported only as a labeled secondary diagnostic.
Before outcome access, freeze a registry of every possible `produced_with_warning` code and assign
each code exactly one disposition: permit the PID-augmented output, use the exact same-fold
\(M_1\) substitution, or block the comparison. Only explicitly allowlisted warning codes may use the
PID-augmented output. An absent, unknown, malformed, or unregistered warning code defaults
fail-closed to exact same-fold \(M_1\) substitution when the paired receipt remains valid, and otherwise blocks
the affected primary comparison. Warning frequencies and dispositions are reported by code; they
cannot be rewritten after outcomes are inspected.

**Primary endpoint.** Measure out-of-sample improvement of deployed \(\widetilde M_2\) over \(M_1\) with the active parent endpoint. Use the direct response-prediction score for H1-A. Use the frozen effect-specific endpoint for H1-B. Use the frozen primary scoring contract for H2. Apply nested cross-fitting and task-family-blocked uncertainty. Define improvement so larger values are better. Freeze the minimum useful margin before the holdout. H3 succeeds only when the one-sided lower confidence bound exceeds that margin under the frozen multiplicity rule. Noninferiority, equivalence, or a nonsignificant difference cannot establish added value.

**Secondary endpoints.** Mechanism discrimination on synthetic or controlled systems with matched lower-order dependence; calibration; stability under justified nuisance transformations; and the fraction of eligible deployment cases for which PID does not abstain.

**Local-feature validity.** Episode-local or window-local PID features may not be invented by running a global estimator on a handful of within-episode samples. The construction must be derived for the named measure or clearly labeled a surrogate, use a frozen train-reference population, and pass oracle and null tests for both local ranking and aggregate reconstruction. Fit, eligibility, and evaluation folds are disjoint.

**Shared-code limitation.** Prisoma and Galadriel using the same `pid-rs` implementation is reuse, not cross-implementation validation. Independent validation requires a mathematically equivalent implementation or reference calculation whose errors are not inherited from the same core [R72–R75].

**Kill criterion.** PID becomes a negative/methodological result when the gain is below the useful margin, the eligible support is too narrow for the intended use, abstention is excessive, conclusions reverse across equally justified measures or preprocessing regimes, or replication fails. The infrastructure and H1/H2 program continues unchanged.

## H4 — representational availability can diverge from response to one tested intervention

H4 is a preselected alternative to H3 and a useful companion diagnostic. It is not an
outcome-driven fallback. If H3 outcomes are inspected first, H4 requires a fresh untouched sample
and the frozen sequential error rule in this section. Its confirmatory conclusion concerns the
randomized effect of one engagement-validated intervention construction on one frozen outcome.
It does not identify natural pathway non-use.

**Target population and regions.** Freeze a target population \(P^\star\) and a finite partition
\(\mathcal C^\star\) of pre-assignment observable covariates, with target weights
\(w_c=P^\star(C=c)\) summing to one. Cells may be scientifically predeclared, or a
low-complexity conditional-average-treatment-effect (CATE) region rule may be learned using
discovery/training outcomes and then locked before an untouched randomized confirmatory sample. A
region discovered and evaluated on the same outcomes is exploratory.
Weights come from a declared probability-sampling design, an explicitly enumerated finite
benchmark, or a separately declared target-covariate sample independent of confirmatory outcomes.
Treatment randomization alone identifies treatment contrasts within the randomized sample; it
does not identify target-population sampling weights. When the randomized confirmatory sample is
not itself the target population, bind the sample source, selection indicator, sampling
probabilities where known, overlap diagnostics, effect modifiers, conditional effect-transport
assumptions, target-weight estimator, and weight-uncertainty procedure before outcomes are opened.
Every reported cell requires sampling support, treatment positivity, engagement, measurement
validity, and enough independent clusters; otherwise its result abstains.
Engagement, receipt, and specificity are cell-level validity gates for the frozen intervention
construction. Realized post-assignment engagement never defines, filters, or reweights the primary
ITT population; any per-protocol or receipt-conditioned analysis is separately identified and
secondary under §2.6.

For a task-relevant variable \(Q\), representation site \(R\), cell \(c\), intervention
construction \(k\), and one declared policy, execution, or physical outcome \(Y\), define:

- \(A_{Q,R,c}\): preregistered out-of-sample availability from a locked probe and metric, relative
  to a capacity-matched reference and oriented so larger values mean greater availability;
- \(\tau_{Q,c}^{(k,Y)}=\mathbb E[Y(1)-Y(0)\mid C=c]\): the randomized cell-average ITT effect of
  the exact intervention construction on the exact target;
- \(G_{Q,c}^{(k)}\): target-engagement, specificity, receipt, and support diagnostics.

Randomized marginal arm laws can identify a cell-average effect under the stated assumptions; they
do not identify the joint law of \((Y(0),Y(1))\), the prevalence of nonzero individual effects, or
which units have an effect. `formal/individual_effect_prevalence_nonidentification.smt2` gives an
explicit countermodel with identical arm marginals and ATE but different individual-effect
prevalence. H4 therefore concerns observable or held-out-identified **regions and their target
population mass**, never latent per-unit effect labels.

**One frozen primary tuple.** Before the confirmatory holdout, freeze exactly one tuple
\[
\Theta^\star=(Q,R,\text{probe},\text{availability metric/reference},k,\text{dose},
Y,\mathcal C^\star,w,\Delta_A,\delta_T,\pi_{\min}),
\]
including all preprocessing, time windows, support gates, and the direction of each comparison.
The tuple contains exactly one primary outcome \(Y\); no composite assembled after outcome access
or best-of-outcomes selection is permitted. Other layers, probes, intervention constructions,
doses, outcomes, region rules, and margins are secondary or exploratory. A second intervention
construction is strongly preferred as a
prespecified replication because a null under one construction can reflect poor engagement or
downstream compensation, but it is not silently pooled with the primary tuple. Include a
positive-control variable known to affect the target and a negative-control site or variable
expected not to; failure of either control blocks the affected H4 claim.

Probe and attribution selection also requires method sanity checks. Randomize model parameters and
labels where applicable. Use control tasks to measure probe selectivity. For a causal-abstraction
claim, freeze the high-level variable, correspondence, intervention map, and interchange test.
Activation change alone does not establish that abstraction [R124–R125].

**Intersection–union decision.** For each primary cell, availability must be superior by the
useful margin,
\[
H^{A}_{0,c}: A_{Q,R,c}-A_{\mathrm{ref},c}\leq\Delta_A,
\]
and the randomized mean effect must be equivalent to zero within the scientifically justified
region,
\[
H^{T}_{0,c}: \tau_{Q,c}^{(k,Y)}\leq-\delta_T
\quad\text{or}\quad
\tau_{Q,c}^{(k,Y)}\geq\delta_T.
\]
A cell is certified only when both null components are rejected: a simultaneous lower bound for
availability exceeds \(\Delta_A\) **and** a simultaneous interval for the effect lies wholly inside
\((-\delta_T,\delta_T)\). A significant probe beside a nonsignificant effect is not divergence.
Define the target-weighted divergence-region mass
\[
D^\star=\sum_{c\in\mathcal C^\star}w_c\,
\mathbf 1\{A_{Q,R,c}-A_{\mathrm{ref},c}>\Delta_A
\ \text{and}\ |\tau_{Q,c}^{(k,Y)}|<\delta_T\}.
\]
If the primary claim requires \(D^\star\geq\pi_{\min}\), use a conservative lower bound formed
from simultaneously certified cells and include target-weight uncertainty whenever weights are
estimated rather than fixed by an enumerated finite target. \(D^\star\) is a mass of
baseline-defined regions satisfying two population-level properties, not a prevalence of
individual causal effects.

**Multiplicity and language.** The complete primary cell family and both components of every
intersection–union test use a prespecified strong familywise-error procedure or simultaneous
confidence region at level \(\alpha\). Selection of cells, margins, or the primary construction
after seeing confirmatory outcomes invalidates the claim. False-discovery-rate control is allowed
only for explicitly secondary screens and cannot promote a secondary region into the confirmatory
result.

Permitted conclusion: a simultaneous lower bound establishes that at least the prespecified
fraction \(\pi_{\min}\) of the target population lies in baseline regions where \(Q\) is decodable
above the useful margin and the cell-average effect of the tested intervention on the declared
target is within the equivalence region after the frozen cell-level engagement, support, and
multiplicity gates pass, without conditioning the ITT analysis on realized receipt. Prohibited
conclusions include that the system never uses \(Q\), that any unit has zero
individual effect, that the divergence-region mass is individual-effect prevalence, that the probe
reveals the natural code, or that a patched activation is a modular causal variable.

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
| EC1 experiment semantics | finite preregistered adapter/capability and variable inventory; valid controls; injected fault universe/distribution; oracle; mandatory per-fault–adapter absolute sensitivity floors without aggregate rescue; exact/tolerance replay margins; false-positive endpoint; uncertainty, multiplicity, and replay comparison against a baseline stack | second independent adapter plus external reproduction | tested only on self-generated happy paths, distribution-average detection that hides a failed registered pair, or claims beyond the registered fault universe |
| Average intervention effect | assignment integrity, ITT contrast, manipulation check, cluster-aware uncertainty | second task family for broad language | post-assignment exclusion or treatment ambiguity |
| Paired algorithmic response | immutable pre-treatment snapshot, exact clone/reset contract, declared RNG coupling, direct paired response, outer-fold prediction | second construction or policy before language beyond the frozen construction and regime | mutable shared state, unquantified Monte Carlo error, or physical-effect language |
| Closed-loop effect moderation | pre-treatment feature, assignment integrity, outer-fold evaluation on randomized outcomes, calibration, useful-margin test | directional replication | post-treatment moderator, in-sample subgrouping, or paired-software contrast substituted for physical outcomes |
| Prospective monitor | landmark freeze, censoring/competing-risk handling, external/temporal holdout, calibration, decision utility | external task/time block | frame leakage or prevalence-obscured metric |
| PID incremental value | all four gates, matched baselines, nested cross-fitting, one frozen target-ID ledger, and full-population same-fold \(M_1\) substitution for every \(M_2\) abstention | second regime/policy | complete-case scoring, ID drift, unsupported local score, or shared-code “validation” |
| Availability–tested-intervention-effect divergence | one frozen tuple and outcome; held-out availability superiority; randomized cell-average effect equivalence; target sampling/transport contract and weights; engagement/support; weight uncertainty; simultaneous familywise control; joint design power | second construction or policy before language beyond the exact frozen construction | nonsignificant effect treated as equivalence, treatment randomization treated as target sampling, one construction generalized to natural non-use, in-sample region discovery, or region mass called individual-effect prevalence |
| Transport | named target population, overlap, effect-modifier audit, external data | another site/embodiment when claimed | “different benchmark” without transport assumptions |
| Safety relevance | process/outcome measure, failure coverage, intervention evaluation | operational context | certification language or unmeasured hazards |

A claim is downgraded automatically when any required cell is missing. Statistical significance cannot upgrade a design whose identifying assumptions failed.

# 5. Experimental program

## 5.1 Gate sequence

The program has one primary world-model ladder and one preserved diagnostic ladder. A diagnostic
result cannot rescue a failed world-model gate. A world-model result cannot validate a diagnostic.
Later results cannot rescue an earlier failed gate through post-hoc reinterpretation.

| Primary stage | Purpose | Required output | Gate |
|---|---|---|---|
| WM0 | Freeze W1/W2 variables, actions, outcomes, resources, and decision rules | world-model analysis specification and untouched holdout ledger | target, action support, unit, comparator, and margins are unambiguous |
| WM1 | Validate exact-fork and pre-oracle decision semantics | native reference, tamper tests, and replay receipt | no oracle leakage; bridge-only selected execution |
| WM2 | Qualify one learned predictor on the named M4 Max | exact pins, action-sensitivity, CPU/MPS parity, search trace, latency, memory, and fallback receipt | supported actions change forecasts; resource and parity gates pass |
| WM3 | Pilot W1 and linked W3 panels | support map, score calibration, nuisance controls, and decision-flip cases | at least one supported forecast regime and one discriminating matched panel |
| WM4 | Confirmatory W1 | locked fork-level forecast study | proper or consistent score clears its positive useful margin |
| WM5 | Confirmatory W2 | randomized complete-policy study | intention-to-treat utility clears its positive useful margin |
| WM6 | Replication or bounded transport | second task family, environment, model, or embodiment | relationship replicates within the named target population |

The diagnostic ladder remains available when it answers a separate question:

| Diagnostic stage | Purpose | Required output | Gate |
|---|---|---|---|
| D0 | Freeze diagnostic variables, estimands, and outcomes | diagnostic analysis specification | all targets and units are unambiguous |
| D1 | Validate the named measure, estimator, and preprocessing | oracle recovery, coverage, stability, and abstention map | at least one eligible diagnostic regime |
| D2 | Validate capture, timing, intervention, and replay | conformance and benchmark report | no unresolved corruption; replay tolerance met |
| D3 | Pilot interventions | engagement, dose, carryover, placebo, and OOD checks | nontrivial and interpretable intervention |
| D4 | Confirmatory H1 | locked intervention-response study | held-out family result and replication plan |
| D5 | Confirmatory H2 | prospective failure study | locked temporal or family holdout and calibration |
| D6 | Conditional H3 or H4 | locked incremental-PID or exact-construction divergence study | frozen branch decision passes |
| D7 | Diagnostic transport | cross-policy, simulator, embodiment, or robot replication | bounded claim of external validity |

## 5.2 Model, policy, and environment selection

Select the first learned world model for identifiable action-conditioned prediction on the named
M4 Max. Do not select it for benchmark prestige. It must have:

- exact, reviewable code, weights, preprocessing, and license terms;
- a deployed query that clamps an action and predicts a later declared reference target;
- a declared support set for every candidate action;
- controllable randomness or a repeatable inference path;
- a reconstructable propose–predict–score–select trace;
- a CPU path and a credible MPS port that never hides CPU fallback;
- tractable multi-replan compute under a frozen deadline and memory limit;
- a reproducible environment with meaningful action consequences.

The first external qualification candidate is the pinned LeWorldModel (LeWM) PushT stack in
Section 10.4 [R181, R182]. It remains a candidate until WM2 passes. Its compact action-conditioned
predictor and CEM planner are a better first M4 engineering target than the larger JEPA-WM stack.
The upstream evaluator still hard-codes CUDA. The repository must not call LeWM MPS-supported
before end-to-end parity, resource, and planner tests pass.

An independent reproduction concerns LeWM TwoRoom, not PushT or M4 [R182]. It used one seed and
does not qualify or disqualify the selected port. It does show that protocol identity is part of
the result. On the same 50 episodes and released checkpoint, protocol-sensitive success moved
from 84% to 8% through goal construction alone. The paper also found conflicts between released configuration and
appendix values. WM2 must bind preprocessing, action gathering, normalization, episode selection,
goal construction, step and replan budgets, and planner settings before outcome access. When
authoritative sources disagree, freeze and report each feasible reading. Do not select a reading
after observing its result.

Select the companion direct policy separately. It needs reviewable action semantics, a frozen
controller, and repeatable closed-loop execution. SmolVLA is a compact MPS-oriented direct-policy
candidate, not a world model [R140]. A policy with a training-only predictor is also not an
action-conditioned deployment model unless its deployed graph exposes that query.

The second learned model should change one scientific axis while preserving the environment,
action law, target, scorer, and resource budget. A comparison that also changes tasks, embodiment,
controller, and target is descriptive. Proprietary systems can support black-box external tests.
They cannot support internal-mechanism claims without access to the required variables.

## 5.3 Factorial task design

Where feasible, cross these factors:

- task family;
- scene/layout and initial condition;
- object identity and visual appearance;
- instruction semantics and paraphrase;
- learned world-model identity and checkpoint;
- supported action family and forecast horizon;
- predictor enabled, bypassed, or replaced by a matched baseline;
- renderer substrate when W3 is active;
- candidate-pool and search-budget regime;
- selector and scorer;
- intervention type and dose;
- policy checkpoint or training regime;
- action horizon/controller setting;
- embodiment only in the transport stage.

Use a balanced or documented fractional-factorial design rather than an unstructured collection of
failures. Do not call linked matched panels a full factorial unless all required cells have the same
estimand and support. Preserve independent variation among instruction, scene, state, prediction,
selection, control, and execution factors.

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

### 5.4.3 Prediction and selection interventions

Examples include:

- shuffle candidate actions while preserving the fork and pool size;
- replace the learned prediction with a matched persistence or oracle-free baseline;
- bypass the predictor while retaining the same proposal, scorer, and controller budget;
- change one candidate while preserving pool order and all other candidates;
- replace or perturb the scorer without changing predicted states;
- vary search depth, population, or stopping rule under a matched compute budget.

Every intervention must preserve the complete pre-oracle trace. Report action sensitivity,
ranking changes, selected-action changes, abstention, and decision flips separately. A changed
prediction without a changed choice is forecast sensitivity, not decision value. A changed choice
without a better randomized outcome is not policy improvement.

### 5.4.4 Decision and execution interventions

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
- handle reset failures as outcomes or censoring under a frozen primary rule. Any exclusion is secondary and needs an explicit missingness assumption.

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

### Forecast and selection outcomes

Evaluate each prediction against one declared later reference target. Use a strictly proper score
for a predictive distribution or a strictly consistent loss for its named point functional. Report
support violations, calibration, action sensitivity, and abstention before selection performance.

Evaluate ranking and selection as separate objects. Record candidate ranks, selected-candidate
regret against the shared oracle-labeled pool, coverage, abstention, and decision flips. These are
fork-level outcomes. They do not establish that the learned predictor is a physical transition or
that the complete policy improves closed-loop behavior.

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

Keep every action, forecast, score, and oracle label from one restored fork or adaptive search
trace in one fold. Group task, dynamics, asset-lineage, and scene families when the claim concerns
unseen families. Never split candidates from one pool across model-selection and evaluation data.

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

Changing any required scientific field after unblinding creates a new estimand and must be labeled exploratory. A software configuration file is not a substitute for the table because it rarely records causal assumptions or the permitted conclusion.

## 6.2 Leakage and fitted preprocessing

Any learned transform used for held-out, inferential, or predictive evaluation—including
normalization, PCA, PLS, SAE, clustering, codebook, probe, local-information reference
distribution, imputation, threshold, or feature selector—must be fit inside the training fold.
Nested cross-fitting is required when the transform and outcome model are both learned. A
same-row fit is eligible only for an explicit in-sample software screen. It cannot support a
held-out or application claim.

A transform record must include:

- training sample IDs and time cutoff;
- code/weights/configuration hash;
- fitted parameters or artifact hash;
- source tensor contract;
- random seed;
- intended reuse scope.

Using all episodes to fit PCA and then cross-validating a classifier is leakage even when labels were not explicitly used, because test-distribution geometry informed the features. For temporal claims, transforms must also respect time order.

## 6.3 W1 and W2 analysis: forecast fidelity is not policy value

### W1 — supported fork-level prediction

Let \(I\) index an independently sampled reset fork. Let \(R\sim g_I\) be the randomized action
assignment at the one frozen level: policy proposal or controller-resolved executed action. Let
\(Q_I(R)\) be the exact action-field tuple available to every predictor after the frozen controller
conversion. Let \(Y_{I,R,h}\) be the declared later reference outcome under the complete frozen
assignment-to-execution rule. For learned model \(m\), matched-access baseline \(b\), and frozen
loss \(\ell\), define

\[
\Delta_{W1}=\mathbb{E}_{I,R}\left[
  \ell\{b(H_I,Q_I(R)),Y_{I,R,h}\}-\ell\{m(H_I,Q_I(R)),Y_{I,R,h}\}
\right].
\]

Positive values favor the learned model. The expectation covers the declared fork population and
action-assignment design, not whichever candidates a proposer happens to prefer. If \(R\) is a
proposal assignment, analyze it by intention-to-treat and retain the resolved command, override,
hold, rejection, and executed-action receipts. If the model returns a distribution, \(\ell\) must
be strictly proper for that distribution. If it returns a declared point functional, \(\ell\) must
be strictly consistent for that functional. A Euclidean latent loss is an engineering target unless
its relation to the physical outcome is separately validated.

Use randomized supported executed actions where observational policy data do not identify the
required transitions. Simulator forks can label every supported candidate for a finite software
population, but they do not identify real-robot transitions. Cluster uncertainty at the independent
reset, scene, and task-family levels required by the claim. Keep all actions from one fork in the
same outer fold.

Candidate ranking is a distinct secondary estimand. On one frozen ordered pool \(C_I\), report
Policy Top-1, pairwise inversions, and selected-versus-best-in-pool loss. These condition on the
realized proposer and pool. They do not estimate proposal quality or deployed-policy value. Report
pool duplicates, infeasible proposals, support abstentions, and oracle headroom as separate
denominators.

### W2 — randomized complete-policy value

Let \(Z_I(p)\) be the frozen episode utility under complete deployed policy variant \(p\). A variant
includes proposal, prediction, selection, controller conversion, deadline handling, abstention, and
fallback. For the learned selector \(p_m\) and strongest frozen same-budget comparator \(p_b\),
define

\[
\Delta_{W2}=\mathbb{E}\left[Z_I(p_m)-Z_I(p_b)\right].
\]

Randomize complete variants across independent reset blocks. Analyze assignment by
intention-to-treat. Count crashes, missed deadlines, abstentions, and fallbacks in the assigned arm.
Do not replace this estimand with a sum of fork-level best-in-pool gaps along the selected policy's
visited states. That sum is a path-conditional selector diagnostic under changing state support.

Freeze one utility or task endpoint and one positive useful margin. Freeze latency, peak unified
memory, power proxy, deadline-miss, and fallback limits as non-rescuable acceptance gates. A
resource-normalized utility is allowed only when its units and trade-off weights are frozen before
outcome access. Always report the raw task and resource endpoints too.

### W3 — two matched designs, not one pseudo-factorial

The observation-substrate panel first replays one authoritative state trajectory through two pure
render functions. One function uses the mesh path. The other uses the 3DGS path. The panel estimates
paired pixel, feature, and immediate frozen-policy response while state and camera stay fixed. It
cannot estimate a closed-loop renderer effect because neither policy controls that common
trajectory.

Bind the full state, body-to-representation manifest, transforms, camera model, exposure and color
pipeline, shutter, frame time, synchronization, and asset lineage. Reset policy memory, KV cache,
history, and random state before each paired query. Separate rigid, articulated, deformable,
background, and robot rendering rules. A mismatch makes the treatment undefined. It is not a null
result.

Estimate downstream renderer effects in a separate randomized complete-policy design. After the
first policy-dependent action, trajectories can diverge and no longer form framewise pairs. Analyze
episode outcomes by assigned renderer pipeline. Keep collision geometry and physical dynamics
identical by construction. If either renderer changes those objects, the treatment is compound and
the observation-substrate claim fails.

A valid matched design can yield no useful decision effect. Report that as a negative W3 result.
Do not convert it into an identity failure or a reason to discard the panel.

W1 and W2 are the only proposed primary scientific claims. Freeze their joint gatekeeping or
familywise-error rule before holdout access. W3, pool-level selector measures, and diagnostic
localization remain secondary unless the claim registry is amended before any relevant outcome is
seen.

## 6.4 H1 analysis: paired algorithmic and randomized closed-loop response

The analysis begins with a common preflight and then follows exactly one primary protocol. The other protocol may be a hierarchically secondary replication, but their endpoints and claim language remain separate.

### Common preflight

1. Freeze the baseline-state boundary, moderator timestamp, treatment version, intervention site, dose, output metric, reset boundary, and target population.
2. Verify that diagnostic capture is observational: compare instrumented and uninstrumented policy outputs, latency, memory state, and controller timing on blinded fixtures.
3. Construct all moderators without treatment or outcome leakage. Unsupervised transforms use outer-training predictors only; supervised diagnostic learning is nested. Freeze missing-value handling and PID abstention.
4. Keep all snapshots, clone replicates, landmarks, and episodes from one persistent case or task-family cluster in the same outer fold.
5. Predeclare whether the scientific claim is frozen-snapshot algorithmic sensitivity or randomized closed-loop effect moderation.

**Current implementation boundary (2026-08-12).** `pid-h1-preflight` implements a strict,
content-addressed schema-v2 input validator for a declared representative-mechanism
structural/noninterference fixture. It binds exact policy and instrumentation specs, execution
context, clock, clone/reset/application boundaries, treatment pair, metric, and manifests; readable
rejected contracts write canonical valid failed run logs. Its result artifacts use schema 3 so an
oversized input has a typed rejection and no false whole-file digest. `pid-h1-protocol-a` implements
a deterministic synthetic finite-benchmark **software reference/scoring primitive**: it exact-binds
the separately passed preflight chain, restores independent per-side clone state, reverses treatment
order, records zero RNG draws, hashes the exact moderator/clone values, computes the frozen scaled
response, and compares fixed design-only versus design+moderator ridge models out of outer fold.
Homogeneous predictions retain proper scores while calibration explicitly abstains. This is not a
subprocess-placement audit, stochastic-policy/Monte-Carlo path, target-engagement study, real policy
capture, physical individual effect, complete binned-calibration analysis, Protocol B, or H1-A evidence.
Those scientific stages remain blocked on pilot, capture, freeze, and replication work.

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

Define the primary improvement as the diagnostic-augmented score minus the matched-access
comparator score after orienting the score so larger is better. Freeze a positive useful margin,
one-sided lower-confidence-bound rule, dependence unit, multiplicity procedure, calibration
acceptance rule, and the consequence of calibration failure. A secondary metric cannot rescue a
failed primary score. State whether the claim is finite-benchmark only or requires a frozen
replication target.

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

Orient the frozen primary contrast so positive values favor the diagnostic-augmented model. Freeze
a positive useful margin and require its one-sided lower confidence bound to exceed that margin.
Apply the frozen multiplicity rule. A secondary endpoint or factual-outcome fit cannot rescue a
failed primary endpoint. The overall ITT, assignment, engagement, specificity, nuisance, and
mandatory effect-validation checks must pass. Directional replication in another task family or
policy is required for the H1-B claim.

No single metric is universally reliable across data-generating regimes, and recent large-trial evidence shows that many causal-ML effect estimates fail internal and external validation; synthetic oracle studies and empirical negative controls are therefore mandatory before trusting a selected learner [R106, R108].

Never score against an unobserved physical “true individual effect.” Synthetic systems may use oracle effects for method validation; exact simulator clone pairs may be reported as Protocol A or under a separately declared paired-world target, not as ordinary parallel-arm truth. Secondary per-protocol or complier analyses must retain the ITT result and state their extra assumptions.

### Confirmatory contrast and multiplicity

The confirmatory contrast uses the locked diagnostic model and the frozen matched-access comparator
registry under identical outer folds, information access, and tuning budget. Freeze any comparator
selection or ensemble rule before holdout access. Report the score difference, interval, useful-
margin comparison, absolute calibration, and all abstentions. Broad model or hyperparameter search
belongs inside nested resampling.

For multiple treatments or doses, either use a prespecified multinomial/dose-response learner or define a small contrast family. Pairwise fishing across modalities, layers, doses, outcomes, metrics, couplings, and horizons is not one H1 test. If Protocol A and Protocol B are both run, specify a testing hierarchy; Protocol B is required before using language about closed-loop robustness or physical outcome moderation.

## 6.5 H2 analysis: prospective failure with time and censoring

Choose the prediction target before model selection:

- binary failure within a fixed horizon among units event-free at \(t_0\);
- cause-specific hazard for a named failure;
- cumulative incidence under competing risks;
- remaining time to failure;
- dynamic risk updated at prespecified landmarks.

The data pipeline must prevent future leakage. All landmarks from an episode stay together. Window normalization, reference distributions, feature selection, censoring weights, imputation, and calibration are fitted only in the outer training data.

For fixed-horizon binary targets with complete follow-up, log loss and Brier loss are proper for the
declared horizon risk. Freeze one primary scoring contract. Under censoring, a forecast-independent
conditional IPCW Brier construction can properly score that scalar risk on the identifiable region
when its conditional-censoring and positivity assumptions hold. The same arithmetic can instead be
used to estimate a declared complete-data risk. Freeze which object is primary. Fit and validate the
censoring law without using the forecast being scored. Report diagnostics and sensitivity analyses
for the censoring model, and state the identification assumptions that data alone cannot prove
[R92, R120–R122]. A right-censored likelihood
instead scores a full event-time-and-type distribution. For competing risks, bind the complete event
ontology and match the score to a cause-specific risk, cumulative incidence, or joint law [R123].
Treating every competing event as an ordinary nonfailure changes the estimand.

Here, complete follow-up means that every unit in the full frozen eligible target ledger is
observed through the horizon under the declared event ontology. Selecting the rows whose follow-up
happened to be observed is complete-case analysis, not the complete-follow-up branch, and is
ineligible as the primary H2 construction without a separately identified observed-data method.

Evaluate:

1. the frozen primary scoring contract and discrimination at the frozen horizon;
2. calibration-in-the-large, slope, and reliability by risk range;
3. event-level detection probability at fixed false-alarm burden, alarms per episode/time, and lead time with undetected failures retained explicitly rather than omitted, under a preregistered threshold/persistence/refractory/event-matching policy;
4. decision utility under explicit costs, fallback capacity, and intervention latency [R93];
5. robustness to task/prevalence shift and missing sensors;
6. external or later-time validation without refitting, followed separately by prespecified recalibration if needed.

Capacity-match learned baselines by training examples, labels, tuning trials, and compute budget. Reproduce applicable representatives of supervised internal-state monitoring, coarsely supervised temporal localization, pure action-space and inter-chunk monitoring, architecture-stratified kinematic monitoring, one-class internal confidence, perturbation disagreement, activation probes, information-theoretic signals, action-conditioned world-model latents, and sequential calibration—or state precisely why an interface is unavailable [R25, R95, R101–R105, R109–R112]. Compare not only predictive performance but annotation burden, white-box access, action-resampling cost, external-model cost, latency, warning time, recovery coupling, and conformal abstention/coverage. Report failure prevalence and independent episode, event, and family counts with every metric. Precision–recall summaries are interpreted at that prevalence [R94].

Conformal calibration is nested inside training/calibration folds. Report the exact nonconformity score, calibration unit, exchangeability or shift assumption, finite-sample correction, and whether repeated landmarks violate the nominal unit. Under task-family or temporal shift, coverage is an empirical transport result unless the chosen weighted/group/sequential method supplies a theorem whose assumptions were checked [R96].

**Current software reference (not H2 evidence).** `just h2-reference` now exercises one narrow
schema-v1 branch of this design on deterministic synthetic finite benchmarks: a named-failure
fixed-horizon cumulative-incidence target over scheduled landmarks that are event-free and
uncensored at entry; separately
content-addressed analysis-plan, event-ontology, feature-contract, and split-manifest artifacts;
task-family-held-out deterministic weighted logistic baseline/diagnostic models with outer-training
standardization; grouped inner-cross-fitted, frozen-stratum reverse-Kaplan–Meier censoring models;
and Horvitz–Thompson IPCW Brier risk-estimator arithmetic. Target and competing terminal events use
\(1/\widehat G(u^-)\), event-free rows use \(1/\widehat G(h)\), and censored-before-horizon rows
retain their place in the eligible-landmark denominator without a numeric row loss. The reference
requires each censoring stratum to be content-addressed and frozen by episode start. An explicit
censoring event at the inclusive horizon leaves the outcome unobserved, while administrative
follow-up completed through that horizon is event-free; this boundary is regression-tested. The
reference never clips a weight and abstains below its frozen positivity floor. Its checked
complete-follow-up artifact exercises reliability-bin arithmetic and no-alarm nondetection
accounting; a focused multi-landmark boundary test exercises externally frozen thresholds, persistence,
refractory/capacity rules, positive one-to-one event matching, and detected/undetected lead records.
Both paths exercise all-event lead curves and a declared-payoff utility scenario; the censored
fixture deliberately abstains from alarm and utility metrics when adjudication is incomplete.
Canonical logs contain zero PID, action, and intervention events.

The reference flags `synthetic_fixture_only=true`, `establishes_h2_evidence=false`,
`prospective_capture=false`, `external_validation=false`, and
`comparator_frontier_complete=false`. It does not validate the conditional censoring assumption,
does not implement calibration intercept/slope or uncertainty, does not supply the matched-access
comparator frontier, and does not freeze the real domain target or scientific minimum useful
effect. Real H2 remains blocked on its domain-specific estimand/ontology/landmark freeze,
powered prospective capture, censoring and missing-sensor sensitivity, full nested calibration and
threshold selection (or an independently justified external threshold), the comparator frontier,
and an untouched later-time or external holdout.

## 6.6 Baseline hierarchy

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

PID must be compared with every mandatory frozen comparator and the predeclared comparator
selection or ensemble rule, not merely entropy or majority class.

## 6.7 Multiplicity and researcher degrees of freedom

The analysis tree includes many source pairs, layers, targets, measures, estimators, dimensions, windows, horizons, tasks, and doses. Uncontrolled search makes nominal p-values meaningless.

Use hierarchical gatekeeping:

1. estimator eligibility;
2. one primary source/target contract;
3. one primary endpoint for the selected H1 protocol, one primary scoring contract for H2, and one
   complete frozen tuple for H4;
4. one locked PID functional, output coordinate, and evaluator-or-estimator regime for H3;
5. strong familywise-error control or simultaneous confidence regions over every confirmatory
   family, including all H4 primary cells and both availability/effect components;
6. false-discovery-rate procedures only for explicitly secondary families; and
7. all unregistered variants labeled exploratory.

For H4, each cell uses an intersection–union decision: both availability superiority and effect
equivalence must pass. Componentwise \(p<\alpha\) without a confirmatory-family correction is
insufficient, and FDR control cannot substitute for simultaneous confirmatory coverage. When
regions are learned, region discovery, tuning, and multiplicity calibration occur entirely before
the untouched randomized confirmation sample.

Do not select a layer, projection dimension, PID functional, output coordinate, route, or temporal
window because it maximizes the test statistic. A multiverse may be reported, but the confirmatory
result must remain the locked branch.

## 6.8 Uncertainty and dependence

Use uncertainty at the level supporting the claim:

- task-family block bootstrap for transfer claims;
- case/episode cluster bootstrap for repeated frames/windows;
- randomization inference for randomized treatment assignments where feasible;
- hierarchical-model intervals with small-cluster corrections or sensitivity checks;
- paired bootstrap for matched control/treatment cases;
- nested resampling when preprocessing or feature selection is fit.

A moving-block bootstrap over frames does not create new independent task families. Report the number of independent clusters and the distribution of cluster sizes.

The current offline harness does not implement episode-aware cluster resampling. Its schema-3 uncertainty sidecar fails closed when multiple episodes contain repeated rows or episode identifiers are only partly present. Serial resampling requires one episode and a strictly increasing canonical decimal `metadata.sequence_index`. An episode identifier groups rows but does not establish their order. Missing episode identifiers do not establish a continuous series. Unit-block subsampling and full shuffle remain available only under the declared row-exchangeability null. A combined bootstrap and permutation request must declare one row-dependence class. One global block subsample or circular shift must never splice episodes into a synthetic stationary series. A restricted circular-shift tail fraction is an approximate stationary surrogate score, not a randomization-test p-value. The harness's within-unit-step-run Pearson lag-1 output is a descriptive screen. It emits no lag pairs when episode identities are absent. Every non-singleton segment also needs a strict canonical `metadata.sequence_index` receipt. Only adjacent rows whose index advances by one contribute, and the report counts excluded gaps. The screen centers both lag vectors inside each contiguous run before pooling residual products. A run needs at least three lag pairs because two pairs force Pearson correlation to positive or negative one. It reports admitted and correlation-eligible pair counts. Each axis average excludes columns that are undefined after centering and reports the defined and total dimension counts. The report derives no effective sample size or block length. The frozen design must justify both quantities independently.

Offline report publication requires the private process-local seal created by the analysis call.
The seal covers every serialized report field. It detects mutation but provides no external
authentication. A deserialized summary is read-only evidence and cannot mint a new summary or run
log. Rerun the analysis from the exact input snapshot to publish again.

Estimator uncertainty and downstream prediction uncertainty must both be propagated. Treating an estimated PID atom as error-free can attenuate or destabilize downstream effects.

## 6.9 Power and design analysis

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

Define a minimum useful effect before simulation. Report operating characteristics across plausible nuisance parameters, not a single optimistic count. For H1 Protocol B, simulate effect-model selection and calibration under null, weak, nonlinear, and sign-changing heterogeneity rather than powering only the average effect. For H2, vary the number of independent failures, episodes, task families, censoring patterns, and false-alarm opportunities; the number of landmarks is not the event count. For H4, simulate the **joint** probability that the complete decision succeeds: availability exceeds its margin, effect equivalence is established, sampling and treatment support gates pass, engagement and both controls pass, estimated target weights carry their declared uncertainty, and the simultaneous intersection–union procedure certifies enough target mass. An exactly enumerated finite target has no weight-estimation uncertainty, but its fixed weights remain bound. Include global-null, availability-only, non-equivalent-effect, poor-overlap, weak-engagement, control-failure, sparse-cell, and weight-misspecification scenarios when weights are estimated, and verify familywise type-I error under the complete confirmatory procedure. Powering either availability or equivalence alone is insufficient. The final design must include enough independent families or embodiments for the claimed generalization level; more frames do not compensate for one family.

## 6.10 Missingness, crashes, and intervention failures

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

## 6.11 Robustness and falsification checks

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

## 6.12 Sensitivity analyses tied to assumptions

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
| PID regime validity | measure/preprocessing alternatives labeled as separate estimands |

A robustness result that changes the estimand must be reported as such, not as confirmation of the original one.

# 7. Estimator and measure validation

## 7.1 Separate four questions

Treat four different questions as four independent gates:

1. **Population gate:** is the intended quantity finite, defined, and scientifically meaningful?
2. **Measure gate:** does the chosen PID functional have the properties needed for the claim in the specified source/target class?
3. **Estimator gate:** does the implementation recover the functional with acceptable bias, uncertainty coverage, and failure detection at the planned regime?
4. **Application gate:** are the real embeddings and sampling process sufficiently close to a validated regime for interpretation?

Passing an MI coherence check does not validate a PID measure. Passing a low-dimensional PID fixture does not validate high-dimensional embeddings. Stability across seeds does not establish correctness.

These four labels describe the current sample-estimator path. A direct specified-law route must
call gate 3 an **evaluator-correctness gate**, not an estimator-validity gate. It checks the
declared law, canonicalization, arithmetic, implementation, and independent oracle. It grants no
finite-sample or population-sampling claim. An exact certifier covers only its admitted inputs and
named coordinates. See the method-selection contract for the complete route identity.

## 7.2 Current repository status

At the current repository state, with the historical review snapshot preserved separately and the
implementation reconciled against the exact `pid-rs` 0.9.0 post-tag review source:

- the high-dimensional MI/coherence path is **NO-GO** on nuisance-dimension controls;
- `pid-rs` has meaningful low-dimensional implementation evidence: continuous shared-exclusions redundancy is checked against a semi-analytic additive-Gaussian oracle with closed-form pointwise terms and a paired finite-sample Monte Carlo expectation, and categorical SxPID is checked bit-faithfully against reference values; these results validate named fixtures, not arbitrary embedding regimes [R73];
- Prisoma’s default Experiment 0 sweep never compares shared-exclusions redundancy with a zero target. Its default 12 cases yield a high-dimensional MI/coherence **NO-GO** while atom-measure validation remains `not_adjudicated` and atom-estimator validation remains `blocked`; the strict band gates analytically known MI terms rather than the full VLA application;
- the current exact `pid-rs` 0.9.0 post-tag review-source pin at `796c11e70f009634b853dc4ada6f565563d82f51` includes a committed `csxpid` fixture with agreement within `1e-12` nats, fail-closed declared-support contracts, and structured reports. Prisoma now requires per-axis support and a stronger declaration for each complete continuous estimator tuple. Marginal continuity does not imply joint absolute continuity or finite MI. The migration exposes unsupported tuples as abstentions [R73];
- continuous shared-exclusions for the intended high-dimensional, dependent, transformed VLA tensors remains **NOT APPLICATION-VALIDATED**: a low-dimensional cross-implementation fixture does not establish broad estimator consistency or application validity, atom components combine estimators with different bias profiles, uncertainty procedures have kNN-specific caveats, and the application-support envelope has not passed [R61, R73];
- no evidentiary real-VLA capture has yet passed all estimator, endpoint, power, and application-support prerequisites [R61].

The v13.0 plan preserves these distinctions. Low-dimensional oracle success is real evidence. It
is neither zero evidence nor permission to interpret high-dimensional representation atoms.

Any future preregistration must freeze one estimator environment. The current software review
environment pins `pid-rs` 0.9.0 post-tag source at
`796c11e70f009634b853dc4ada6f565563d82f51`; do not silently float to `main`. This repository pin
does not freeze the unfinished scientific protocol. The migration from the reviewed `8a5a9dd…`
environment records API/feature
changes, changed support rejections and result status in `CHANGELOG.md` and `findings.md`, retains
the frozen v12.5 review artifacts, and must keep root, Python, and excluded-consumer conformance
checks green. Every evidentiary artifact must record the exact revision and enabled experimental
features. The old environment remains a historical replay reference, not the active estimator;
the 0.9 review surface makes no 1.x compatibility or distribution promise, and the new
low-dimensional fixture does not upgrade the application verdict.

## 7.3 Synthetic validation matrix

Validation must span families chosen to isolate failure modes, not just familiar XOR/copy examples.

### A. Analytic or numerical-oracle families

- independent Gaussian channels with known MI;
- continuous shared-exclusions systems with a measure-specific analytic or numerical oracle;
- discrete copy, unique, XOR, AND/OR, noisy XOR, and mixtures;
- low-dimensional categorical MGW fixtures with exact rational or integer count laws;
- continuous mixtures with numerical integration or high-precision Monte Carlo oracle;
- mixed discrete–continuous targets when intended in application.

The first project-owned categorical case study must sit outside the world-model and `(V,L,D,A)`
harnesses. Freeze one alphabet, event map, two-source lattice, and canonical integer count table for
each condition. If conditions reuse exactly the same source counts, their averaged MGW informative
atom vectors are identical. Any net-atom difference then equals the negative of the corresponding
misinformative-component difference. Do not say “only misinformation changes.” The
misinformative components may also be equal, and their formal name does not imply error or harm.
Independent samples from one source population do not preserve exact empirical equality.

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

High-dimensional atom drift without an oracle is labeled **sensitivity**, not estimator validation.

## 7.6 Fitted categorical shared-exclusions gate

Fitted categorical shared exclusions is not an automatic escape from continuous geometry. The
quantizer creates new categorical variables. The empirical-PMF plug-in then targets the MGW
categorical functional on those variables. This is a different object from the continuous Ehrlich
functional, even when both use shared-exclusions language.

Required checks:

- an explicit fit/evaluation scope and a content-bound row-identity receipt;
- training-only fitting for every held-out, inferential, or predictive evaluation;
- an explicit `descriptive_same_rows` label when a same-row fit is used only for an in-sample
  software screen; that screen cannot support held-out, predictive, or application claims;
- exact functional or hierarchy, cumulative or Möbius-inverted quantity or named hierarchy index,
  antichain coordinate when applicable, aggregation, component, backend revision, log unit, source
  order, and target;
- minimum cell occupancy and effective support;
- held-out assignment stability;
- sensitivity to codebook size and seed;
- informative, misinformative, and net reconstruction for every atom;
- saturation diagnostics;
- explicit statement that fitted-categorical and continuous atoms are different quantities;
- no fallback to `I_min`, BROJA, or another PID functional.

For deliberately categorical mechanism variables, MGW may be the cleanest primary route. For
quantized hidden states, it is exploratory until the quantizer and empirical support pass the
application gate. PLS creates a supervised transformed estimand and requires a separately fitted,
leakage-safe regime.

The current `categorical-sx-pls` software screen is not that regime. Each screen fits PLS toward
the target and evaluates MGW on the same rows. The optional split screen uses train rows only. It
does not score held-out categorical rows. Every estimate must serialize as
`produced_with_warning`, with an estimator-blocked `supervised_same_row_preprocessing` reason.
When saturation also applies, use the combined typed reason. Treat both the observed and shuffled
screens as selection-inflation diagnostics. Neither can support H3 or rescue a continuous route.

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
- computation states (`not_requested`, `produced`, `produced_with_warning`, `abstained`) and
  separate population/measure/estimator/application verdicts;
- semantic version and exact implementation revision.

At application time, every candidate records whether an estimate was **not requested**, **produced**, **produced with a numerical or design warning**, or **abstained**. Computation is not eligibility: record the population, measure, estimator, and application verdicts separately, and permit interpretation only when all required gates pass. Report the denominator: total candidate cases/windows, requested estimates, declared-support-compatible tuples, cases reaching each diagnostic stage, successful computations, warnings, and abstentions by reason. Predictive performance among the small easiest subset is not deployment performance.

Continuous admission needs two distinct declarations. First, declare each axis population. Second,
declare the complete source-target tuple used by that estimator call. The tuple contract asserts
that every required marginal and joint law is regular, full-dimensional, absolutely continuous,
and finite-information. Per-axis continuity does not imply this. A deterministic relation such as
continuous (Y=X) has continuous marginals but a singular joint law and can have infinite MI.
Missing or incompatible tuple contracts must abstain before constructing the upstream KSG or
Ehrlich support assertion. A declaration is an input claim, not proof or gate passage.

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

The authoritative record for accepted recorded events is append-only and schema-versioned. It
cannot establish an upstream event that no capture boundary observed.
The implemented finalized schema-2 validator requires exactly one response for every bridge
request; a missing response is invalid. Schema 1 remains readable and preserves its historical
unresolved-request warning for compatibility.

Minimum event families are:

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

Prisoma’s canonical semantics can be stored in or alongside these formats. A custom JSONL log may remain the internal source of truth for accepted recorded events, but exporters/importers and conformance tests are required. No format can prove an event that no capture boundary observed.

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
- schema error detection sensitivity, estimated separately for every registered fault–adapter pair
  against its frozen absolute floor;
- intervention-assignment fidelity;
- timestamp alignment error;
- replay discrepancy;
- registered provenance-field coverage;
- time to answer a blinded audit question;
- proportion of invalid analyses automatically blocked.

The benchmark must include negative cases. A system that records valid runs but fails to reject invalid ones has not established scientific value.
No distribution-weighted or macro-average detection summary is an EC1 acceptance substitute:
every registered pair is a mandatory gate, while aggregate summaries remain descriptive.

## 8.9 Repository ecosystem: evidence, boundaries, and useful roles

The public `sepahead` profile depicts a broad project graph, but a profile diagram is architectural intent, not implementation evidence [R85]. Repository relationships are classified by auditable evidence:

- **E0 — intention:** profile, roadmap, issue, or prose says projects should connect;
- **E1 — interface specification:** schemas, adapter design, or integration document exists, but no build-tested adapter;
- **E2 — declared immutable dependency:** submodule, lockfile, exact git tag/revision, or consumer manifest creates a reproducible code relationship;
- **E3 — build-tested adapter:** producer and consumer compile/test together against golden fixtures at pinned revisions;
- **E4 — end-to-end scientific conformance:** live or replayed data traverse the boundary with schema, time, frame, intervention, provenance, fault-injection, and outcome checks;
- **E5 — independent replication:** another team or independently maintained implementation reproduces the integration and scientific result.

Use **connected** only for E2 or above, **integrated** only for E3 or above, and **validated integration** only for E4 or above. Shared ownership or shared code does not supply independence.

### 8.9.1 Audited relationship matrix, reconciled after the reviewed snapshot

The review snapshot remains the provenance baseline. This living matrix incorporates dated,
verified post-review changes. The NCP provider boundary was rechecked against official main on
13 August 2026. The `pid-rs` public-main comparison was rechecked on 16 August 2026. An external
project can mature without raising its **relationship to Prisoma**.

| Repository | Audited relationship to Prisoma | Evidence level | Scientifically useful role | Boundary / required next evidence |
|---|---|---:|---|---|
| `pid-rs` | Direct git submodule deliberately pinned to the 0.9.0 post-tag review source at `796c11e70f009634b853dc4ada6f565563d82f51`, seven commits after the immutable `v0.9.0` review tag at `a9a275157237999c8da6ab813130d74f6113dec9`; Prisoma crates path-depend on its estimator/run-log crates. Public main was observed at `bc3aa80fb6025e709c2906a08bce25a4fac40578` | E2; root and consumer conformance checks support E3 for the tested local paths | canonical estimator implementation, run-log schema, low-dimensional analytic/external fixtures, discrete references, fail-closed support contracts and reports | upstream main adds unadopted method catalogs, formal/categorical assurance work, support-change and concentration records, Lean 4.33 formal replay hardening, source-errata and evidence-boundary registries, and exact-certifier surfaces, but says Prisoma integration is not claimed. An isolated all-feature Prisoma check, test-target build, and 531-test run passed at `722d3abe`. Head `bbdfda40` changes only upstream assurance surfaces relative to that tested revision. Estimator-code anchor `cb3f58f0` adds one bounded KSG integration commit. Prisoma inspected its three changed production Rust files and replayed four predecessor-radius fixtures plus the structured overflow fixture. Head `7473e62` is a custody-only child whose full CI run `31724449805` failed two jobs. Current public head `bc3aa80` repairs that custody wiring and also changes no crate or Cargo input. Its full CI run `31773937366` passed 45 jobs, and CodeQL run `31773937102` passed four jobs. Provider green does not establish consumer compatibility or application validity. Upstream still marks broader revision-4 KSG repository integration NO-GO. Retain the reviewed pin until consumer-owned scientific-value and adoption checks pass. The fixtures do not establish VLA application validity [R72–R73] |
| `NCP` | Optional `ncp-observer`, excluded from the default workspace, pinned to immutable NCP `v0.8.0` at `2f5bd586` (wire 0.8). Verified upstream main `1a04294c90c1b50eba06ae1c6afe9c951319250d` is the unreleased, release-blocked `1.0.0-rc.1` candidate (wire 1.0; compact proto contract hash `163acc57d8a62b66`). Full-key visible-receipt integrity and receipt-last bundles are local-only. Receipt schema 1 binds the exact legacy identity. The bounded 18-case observatory replays one complete hand-authored trace twice through the shared route/raw decoder. It separates native response from its manifest oracle. | E2 dependency edge with a reproducibility-bound local fixture execution for the exact fixture and consumer; not producer-consumer E3 | optional source of versioned observations from neural or robotic systems; deterministic decoder, join, finalization, and replay fault checks | Retain the wire-0.8 read-only pin. Do not infer wire-1.0 compatibility. NCP ledger tasks `P01`, `P02`, and `P03` are OPEN, not dependency-ready, and **NOT RUN**. B01 remains `IN_PROGRESS`; its refined low-overhead architecture and prepared-stream-monitor gap record are coordination-only and have no passing receipt. Require a separate consumer, conforming live producer, migration corpus, own-stream/timing/QoS/reconnect/authentication evidence, interventions/outcomes, and an E4 report. Whole-tick omission remains a native blind spot. No PID population support is inferred [R72, R74]. |
| `galadriel` | Public revision `80506dd2ce52b33c3334c7d1760a8155c7631241` freezes 0.9.0 candidate inputs and adds a strict two-route consumer, lifecycle adapter, and bounded operational receiver. No direct Prisoma dependency or adapter exists | E0 between projects; E2 only to shared dependencies | external diagnostic comparator for cross-sensor consistency, NIS/CUSUM, signed correlation, and optional PID evidence | require a reciprocal pin, direct consumer-owned adapter, producer-consumer golden fixture, and receiver-verified deployment. Shared `pid-rs` results remain one correlated method family, not replication [R75–R76] |
| `crebain` | Public revision `7f6b3bdf4d20aba1b351b3ceacb259bd123c93a6` adds a restricted read-only Engram view to the 0.9.0 research prerelease. Embedded mode disables native, external telemetry, artifact exchange, NCP, and plant paths. NCP action/control commands stay unregistered, and no direct Prisoma reference exists | E0 between Crebain and Prisoma; the read-only Engram view does not create a direct edge | candidate non-manipulation embodiment, multimodal tracking/fusion testbed, timing/fault-injection or advisory-evidence producer | host messages do not attest a process or build. A local `put` does not prove receiver receipt. No live NCP control loop or direct Prisoma adapter exists [R76] |
| `manwe` | Public revision `6d73405bbf5365039ee1d0db9c466ed6346a9c57` includes numeric, I/O, and security hardening but still has no drop-in Prisoma adapter; schemas, tensors, clocks, frames, and statistical assumptions differ | E0/E1 | candidate perception producer and shift/latency testbed; useful negative example for adapter discipline | satisfy its documented promotion gates; never infer compatibility from Rust/Python or shared maintainer [R77] |
| `engram` | The named public `sepahead/engram` repository remains at placeholder revision `a4ce6ab9897dd3f1265b4cacc53f0afc349087cd`. It has no executable public producer or direct Prisoma adapter | E0 between projects | candidate future neural-state producer only after executable evidence exists | require an executable release, immutable producer revision, variable semantics, a compatible read-only transport, producer-consumer fixture, and E4 conformance [R74, R78] |
| `Paper2Brain` (Engram Neural Labs host) | Public revision `2648caf18d24075c4a36af81a6bb032bb551244e` retains the unchanged byte-locked generic Prisoma descriptor in its frontend and compiled Tauri catalog. Its generic JSON-RPC TCP adapter connects to a separately started `--engram-host` process and reads exact describe, session, and status methods. The descriptor declares target Engram wire 1.0 incompatible with Prisoma wire 0.8. NCP's provider inventory records a preserved in-progress Paper2Brain migration that targets candidate wire 1.0. It is not an installed or qualified integration. | E2 declared immutable consumer-manifest relationship plus local read-only transport evidence; not E3 because no producer-consumer scientific golden fixture exists | host-rendered review of bounded live status and one canonical Prisoma run log; candidate future neural-state producer | the active profile is finite and read-only, but loopback and peer reports do not attest a process. The host does not start Prisoma. The descriptor accepts only a canonical run log and produces no artifact. Rerun and offline VLDA remain standalone Prisoma surfaces. The live card and structural preview are not Prisoma validation. No wire translator, NCP bridge, validated artifact importer, authority path, or E4 report exists [R74, R78] |
| `melkor` | Public revision `529260f568c62250b0541a11f5c24b45767bf1cf` has a v2 development/release-candidate line, canonical scene model, KHR_gaussian_splatting GLB I/O, inspection/conversion paths, and resource hardening; no direct Prisoma reference or adapter exists | E0 | optional 3D reconstruction/uncertainty or scene-variation producer | separate reconstruction uncertainty from policy uncertainty; require calibrated geometry, model/tool license review, a consumer-owned adapter, and an E4 benchmark [R79] |
| `WorldWarp` | Prisoma has an optional integration specification; no implemented adapter was verified; upstream repository is a forked scene-generation system | E1 | external world-model/counterfactual-scene baseline under a bounded research question | high compute, generated-scene support, causal validity, and license/provenance must pass; never put on critical path [R80] |
| GauSS-MI concept | Prisoma now separates an E1 reconstruction-quality covariate/active-view study from a quarantined weighted-KSG/PID sketch | E1 for the covariate/view design; E0 for weighted PID | possible reconstruction-quality nuisance study or separately validated active-view experiment | the weighted expression lacks a named population functional, derivation, support conditions, or oracle evidence and must not be implemented as written; reconstruction uncertainty is not estimator uncertainty [R81] |
| `cobot-atlas` | No direct adapter/reference found; public mesh dataset and generation pipeline | E0 | asset diversity for controlled object/appearance/layout factors | freeze asset revision, physics/collision validation, lineage, near-duplicate groups, and per-asset license/provenance [R82] |
| `relief-atlas` | No direct adapter/reference found; large generated mesh collection with per-asset licensing caveat | E0 | optional stress-test domain for disaster-response scenes, not a primary manipulation benchmark | perform asset-level licensing, quality, collision, realism, and safety/ethics audit; avoid scope expansion [R83] |
| `cortexel` | Public revision `d29669e6d5b1766fd96e1eacefb02b3f43c5ce61` is a 0.9.0 prerelease with deterministic accessible SVG export across 19 stable visualization families; no direct Prisoma reference or adapter exists | E0 | possible read-only renderer of scientific artifacts after a stable consumer contract exists | no published package/DOI, external oracle, real adapter, or Prisoma contract exists. Visual agreement is not analysis validation, and Cortexel does not supersede the Rerun-first decision [R84] |
| `haldir` | Public revision `555108666cb82e8a36dcd4b08b5b30c62367a6f4` contains substantial internal P0/durability and release work, an opt-in exact NCP adapter, a narrow receiver-observed synthetic mTLS/ACL campaign, an off-by-default exact-route strict-publisher binding with a broader Called-lifecycle fault matrix, and fail-closed validation of a caller-declared live startup profile; no runnable service, direct Prisoma dependency, or application route exists | E0 between projects; Haldir’s internal tests are not Prisoma integration or security evidence | offline authority/receipt comparator for fail-closed decision semantics | the live-profile declaration is cooperative, process-local, bypassable through lower-level construction, and not durably bound; positive composition and result/fault cases use test-only seams, and no live Zenoh session executes the concrete method. Haldir originates command frames, so a direct route would conflict with the Agent Bridge-only control plane unless §16 were revised. Require credential custody, runnable service, Crebain application, immutable release, consumer-owned adapter, threat-model mapping, and E4 conformance before any security/integration claim [R86] |
| `brojapid-activationfunctions` | No software edge to Prisoma; a released BROJA-PID analysis of activation functions linked to prior reproduction work | E0 edge; public code/release evidence | excluded historical lineage only; no active Prisoma analysis role | BROJA unique information is a different PID measure; Prisoma does not import, relabel, compare, or use its atoms for an active hypothesis [R97] |
| `mahmoudian-2020-rescience` | No software edge to Prisoma; published ReScience C replication and archived code | E0 edge; publication/reproducibility lineage | evidence of prior reproducibility practice and a source of controlled transfer-function fixtures | does not validate `pid-rs`, continuous shared-exclusions, VLA hypotheses, or present infrastructure [R98] |
| `nest-simulator` | Public fork advertises PID/information-theoretic work on feature branches; no direct Prisoma root reference or pinned adapter was verified | E0 | candidate neural-state producer through a future read-only NCP adapter | pin the exact branch/commit, separate fork changes from upstream NEST, publish a fixture, and pass E4 semantics/security tests [R99] |

“No direct reference found” is a bounded statement about the reviewed public material, not proof that no private branch or unpublished adapter exists. Anonymous GitHub search is incomplete; therefore the evidence ledger records both positive evidence and search limitations.

### 8.9.2 Current implementation boundary

At current Prisoma HEAD, unchanged in kind from snapshot `64bd881…`, the only direct
code/dependency relationships inside Prisoma are:

1. `pid-rs` as the pinned canonical estimator/run-log submodule; and
2. NCP as an optional pinned dependency of the excluded read-only observer crate.

Paper2Brain's external consumer manifest is a direct declared relationship at its exact reviewed
revision, but it is not a Prisoma dependency or a producer-consumer scientific integration.

The core thesis must run with NCP disabled and must survive a PID NO-GO. WorldWarp, GauSS-MI, Engram, sibling visualization projects, generated-asset collections, and UAV testbeds are optional producers, comparators, or future transport settings—not prerequisites.

### 8.9.3 Dependency firebreak

A release candidate passes the firebreak only when:

- the capture/intervention/replay core builds and executes without NCP;
- the static factual-outcome label baselines execute with PID disabled and without `pid-rs`
  atoms; this dependency smoke is groundwork, not H1 response scoring or prospective H2;
- an ordinary local-file or standard-format adapter can replace every sibling repository;
- no private repository, unpublished model, personal token, or sibling checkout is required for the minimum viable thesis;
- an unqualified external model, 3D backend, viewer, or asset source can fail without changing
  assignments, outcomes, or provenance of already-recorded runs; W1/W2 execution begins only after
  one learned world-model path passes M2 and is frozen as a required study component;
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
9. replay equivalence and registered provenance-field coverage against the canonical event model;
10. a scientific conformance test showing the adapter does not change the estimand or silently fit on holdout information.

A status badge or successful `cargo build` is E3 evidence at most, and only for the tested revisions. E4 requires data and scientific semantics, not merely type compatibility.

### 8.9.5 NCP-specific boundary

Prisoma’s NCP component is a **read-only observation client**. It must never acquire command authority merely because NCP supports an action plane [R74]. For each session, record realm, route, NCP tag/wire/contract hash, peer identities, authorization mode, encryption/ACL profile, session ID, sequence numbers, source timestamps, local receipt times, synchronization uncertainty, drops/reorders/duplicates, payload schema, and disconnect/reconnect events.

Open/default transport is unsuitable for untrusted deployment; use an isolated realm or the documented secure profile and verify it. A local mode/TTL governor is defense in depth, not network authentication. Observer failure must not alter the robot/controller, and backpressure must drop or spool diagnostics according to a declared policy rather than perturb control timing.

### 8.9.6 `pid-rs`-specific boundary

`pid-rs` is the canonical implementation dependency, not external corroboration. Prisoma pins the
exact 0.9.0 post-tag review source at `796c11e70f009634b853dc4ada6f565563d82f51`, archives run
configuration and structured reports, records enabled default-off research features, and fails
closed when support is unspecified. The reviewed `8a5a9dd` pin remains historical evidence; the
deliberate review-source migration incorporates the public `csxpid` fixture and stricter
support/provenance contracts, regenerates locks, migrates consumer APIs, and reruns root, Python,
and excluded-consumer conformance gates. This 0.9 surface makes no 1.x compatibility, registry, or
published-wheel promise. Even at the new pin, a low-dimensional fixture is not independent
validation of broad estimator behavior or of the intended high-dimensional/dependent VLA
application. Mixed-dimensional continuous three-source analysis remains exploratory. Application
eligibility is decided by Section 7, not by a passing unit test or the fact that an API returns a
number [R73].

Public main at `bc3aa80fb6025e709c2906a08bce25a4fac40578` adds unadopted method catalogs,
software identity, support-change and concentration records, formal/categorical assurance work,
Lean 4.33 formal replay hardening, source-errata and evidence-boundary registries, outcome
contracts, and exact-certifier surfaces. These additions do not open the high-dimensional or
application gates. An isolated exact-revision all-feature Prisoma check, test-target build, and test
run passed against `722d3abe`. Head `bbdfda40` changes only assurance surfaces relative to that
tested revision. Estimator-code anchor `cb3f58f0` adds one bounded KSG integration commit. Prisoma inspected
its three changed production Rust files and replayed its four predecessor-radius fixtures plus the
structured overflow fixture. Head `7473e62` changes custody and assurance surfaces only. Its full
CI run `31724449805` failed two jobs. Current head `bc3aa80` repairs that custody wiring and also
changes no crate or Cargo input. Its full CI run `31773937366` passed 45 jobs, and CodeQL run
`31773937102` passed four jobs. A later pin requires consumer-owned compatibility, exact-reference,
and regression evidence against the frozen Prisoma contract. Upstream still marks broader revision-4 KSG repository
integration NO-GO. These provider results do not close the consumer review.

### 8.9.7 Ecosystem opportunity without thesis capture

Do not move a sibling integration onto the WM0–WM3 critical path. First complete one rights-approved
raw-to-run-log capture, a structurally independent EC1 adapter, and the conventional-stack
comparison. After those gates pass, Galadriel offers the highest-leverage diagnostic comparator.
NCP wire-1.0 migration, Cortexel, and Melkor remain optional until a consumer-owned contract and
fixture establish a direct edge.

The ecosystem can create notable experiments once the core is stable:

- use `crebain` or Manwe-derived streams as a transport test of timing, multimodal fusion, and non-manipulation embodiment;
- compare Prisoma’s prospective diagnostics with Galadriel’s consistency-monitor outputs while accounting for shared dependencies;
- use NCP to test protocol-version, sequence-loss, and provenance faults with a read-only observer;
- use cobot-atlas assets for prespecified object/appearance diversity after physics and duplication audits;
- evaluate reconstruction uncertainty from `melkor` as a covariate or nuisance factor, not as a replacement for estimator uncertainty;
- use an external world model such as WorldWarp only for a separate counterfactual-support study.

Each is optional. The scientific contribution is the ability to test such systems under one explicit experiment contract, not the number of sibling repositories shown in a graph.

### 8.9.8 Scientific lineage and candidate producers are not integrations

The public ecosystem also contains two information-theoretic lineage artifacts and a possible
neural-simulation producer. The 2020 ReScience C repository documents a replication of a
three-way information-theoretic transfer-function study. `brojapid-activationfunctions` applies
the BROJA unique-information measure to activation functions [R97–R98]. They document historical
questions and reproducibility practice only. Prisoma does not use either artifact as an active
estimator, comparator, fixture, or sensitivity branch. They do not validate shared exclusions,
`pid-rs`, a Prisoma estimand, or Prisoma software.

The `nest-simulator` fork is a plausible future producer of neural-state streams, especially through NCP, but repository-level mention of PID branches is E0 evidence only [R99]. Promotion requires an exact branch/commit, a delta against upstream NEST, executable model fixture, variable semantics, clock and sequence contract, read-only authority, and an E4 end-to-end report. None of these repositories belongs on the minimum thesis path.

## 8.10 Current versus target implementation

Maintain a generated capability matrix with columns: feature, status (`implemented`, `tested`, `validated`, `specified`, `deferred`), exact revision, test command, evidence artifact, known limitations, evidence level E0–E5, and thesis dependency. Documentation must not call a feature “integrated” unless its evidence level meets Section 8.9.

The reviewed source catalog is now `protocols/capability_catalog_v1.json`; the deterministic
machine-readable and human-readable views are `protocols/capability_matrix_current_v1.json` and
`docs/CAPABILITY_MATRIX.md`. Local revisions are exact SHA-256 content bindings rather than a
self-referential future Git commit. Generation fails closed on unknown fields, unsafe/missing paths,
duplicate keys, status/evidence-basis contradictions, missing canonical rows, generated-output
self-reference, missing limitations, and output drift. Status and Section 8.9 evidence are
orthogonal: `tested` records a named local proof path, while a local-only feature remains E0 for
relationship evidence; E2 requires an immutable external dependency and E3 requires pinned producer
and consumer revisions tested together against golden fixtures. `deferred` means E0 unavailable or
off-path and includes rejected records; it is not a delivery promise. Generation checks schema,
paths, hashes, and canonical pins but cannot infer command semantics; review plus CI execution must
confirm that each named proof exercises its declared inputs. The current table contains no
`validated` row: local software/fixture evidence cannot upgrade itself to E4 scientific conformance.

The deterministic Agent Bridge transports implement a bounded local fail-closed slice.
TCP/WebSocket binaries refuse non-loopback bind addresses, start in safe mode, and require explicit
mutation opt-in, but they cannot prevent forwarding, proxying, or tunnelling a loopback listener.
TCP/stdio JSONL lines are capped at 1 MiB, WebSocket HTTP upgrades at 16 KiB, and incoming client
WebSocket frames at 1 MiB. Network reads and writes time out after 30 seconds per operation.
Standard profiles have no total session-duration or aggregate-traffic budget, so progress-making
trickle traffic can persist. The separate `engram-host-read-only-v2` TCP profile exposes only
describe, session, and status. It requires operator-paste pairing on every accepted connection.
It adds finite request-count, line, aggregate-input, run-log-byte, run-log-event, and
pairing-attempt limits. Run-log accounting includes the TCP prefix and terminal seal. Those
limits bound one local session. Pairing proves startup-secret possession only. It does not add
process attestation or remote-deployment qualification.

The WebSocket gate requires exactly `GET /bridge HTTP/1.1`, exactly one each of a nonempty `Host`,
`Upgrade: websocket`, tokenized `Connection` containing `upgrade`, version `13`, and a base64 key
decoding to 16 bytes, and no `Origin`. This enumerated contract is not a claim that every malformed
HTTP/WebSocket request is detected. Client application messages are unfragmented, masked UTF-8
text frames; ping, pong, and close control frames are supported, while binary frames,
fragmentation, and extensions/RSV use are rejected. The bridge implements a single-request
JSON-RPC 2.0 subset: batches are unsupported, an omitted-id notification is silent and distinct
from an explicit `null` request id, parameters are omitted or named objects (not positional
arrays), undeclared top-level method keys are rejected, `sim.step` requires numeric `dt`, and a
profile-invalid parameter uses `-32602`. Handler/domain failures after validation use `-32000`.

Replay/export file methods use non-adversarial canonical confinement below the session run-log
directory. They reject traversal, observed symlink components, non-regular/out-of-root inputs,
missing output parents, and pre-existing outputs; transport run logs and Rerun outputs are
no-replace. The default `pid-sim` runtime omits `export.rerun` and its Rerun/Arrow dependencies.
The opt-in `rerun-export` feature parses and manifests the same exact source snapshot. It encodes
and hashes finalized RRD bytes, then stages, syncs, and persists them no-clobber. This is not a
security-grade filesystem sandbox against hardlinks, aliases, or concurrent local mutation, and
executable transport run logs use `File::sync_all` for the initial prefix, every session flush
before a wire response, and the terminal seal. Generic `SimBridgeSession<W>` durability remains
sink-defined; there is no parent-directory fsync, power-loss claim, or cross-file run-log/export
transaction. Ordinary accepted-client protocol/transport failures are sealed `Failed` only
while provenance storage remains writable; a crash or storage failure may leave incomplete or
unreadable provenance, an apparently complete terminal record with indeterminate
status/durability, or an installed RRD without its final provenance event. This is E0 local
hardening only. Caller IDs remain declarations rather than authenticated identities;
authentication, authorization, TLS, redaction, remote deployment, and external security assessment
remain unimplemented.

The reviewed snapshot has meaningful estimator/run-log and adapter groundwork, but current repository prose also records blocked scientific capture and invalidated high-dimensional regimes [R61, R72]. Treat passing unit tests as evidence of software behavior, not causal identification, estimator validity, deployment security, or paper-level novelty.

The local `pid-sim` implementation also includes two explicitly non-evidentiary protocol
references: the deterministic synthetic H1 Protocol-A runner and the deterministic synthetic H2
fixed-horizon/IPCW/alarm runner described in Sections 6.4–6.5. Both write canonical, replay-valid,
PID-free logs and fail closed on checked fixtures. Neither is real claim execution, external
validation, or a substitute for M4/M5.

## 8.11 Control plane and agent access

In the target remote-capable design, GUI, scripts, notebooks, and LLM agents must invoke the same
typed control plane. Every mutating request must produce:

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
- append-only audit records and content hashes checked against an authenticated or separately
  anchored manifest when tamper evidence is claimed;
- path sandboxing and refusal to overwrite source data;
- secrets never stored in run logs;
- configurable redaction for human video/audio and instruction text;
- dataset consent, retention, and deletion metadata;
- dependency/model/dataset/asset licenses tracked separately;
- model-generated interventions constrained by the preregistered design and safety envelope.

The system does not become safe because actions are logged. Logging enables audit; prevention requires independent controls.

## 8.13 Visualization and rendering

Rerun-first visualization is appropriate because it supports time-aware multimodal inspection without making a custom UI the scientific bottleneck [R45]. Tauri, SparkJS, WebGPU, Gaussian splatting, and editable scenes are optional presentation or experiment-authoring layers.

The pinned Rerun 0.34.1 Rust types expose meshes, pinhole cameras, transforms, images, arrows, and
points. Its PLY loader reduces Gaussian data to spherical points and ignores opacity. It does not
expose the newer unstable anisotropic `GaussianSplats3D` archetype found on Rerun main. The current
Prisoma adapter also drops `FrameObserved`, reduces flow vectors to magnitudes, and ignores
candidate metadata. Therefore it is a derived viewer, not the W1–W3 evidence model. Upgrade or add
a typed adapter only after a storage-compatibility and semantic review.

Rules:

- visualization consumes schema-checked, content-bound artifacts; it is not the source of truth;
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

Use an explicit predictive state, rollout, object flow, contact prediction, or next-state
distribution only when it is a documented model output. Evaluate it first against external
predictive error. A paper's “world knowledge” label does not define a Prisoma source.

Classify the deployed directed graph before naming `D`:

| Class | Deployed computation | Permitted description |
|---|---|---|
| A | \(\pi(A^\pi\mid H,L)\) | direct policy |
| B | predictive target in training; direct policy in deployment | predictive co-training |
| C | \(q(F\mid H,L)\,\pi(A^\pi\mid H,L,F)\) | intended-future conditioning |
| J | \(q(F,A^\pi\mid H,L)\) sampled jointly, with no exposed clamped-action query | coupled joint generation |
| D | \(q(F\mid H,L,A^\pi)\) | action-conditioned observational prediction |
| E_pool | predict, score, and select over one precommitted pool of at least two actions | fixed-pool predictive planning |
| E_opt | adaptively propose, predict, score, and return one final action recommendation | adaptive predictive optimization |

Treat these as different operational and statistical contracts:

\[
q_{\mathrm{intent}}(F\mid H,L),\quad
q_{\mathrm{joint}}(F,A^\pi\mid H,L),\quad
q_{\mathrm{query}}(F\mid H,L,A^\pi),\quad
p(F\mid H,L,\operatorname{do}(A^{exec})).
\]

Class J is operational, not algebraic. The existence of a conditional factorization of a joint
density does not mean the deployed system accepts a clamped candidate action and returns its
forecast.

Both class-E forms require a frozen score and a selection caused by that score. For `E_pool`,
record the ordered pool, every forecast and score, and the selected action. Require a decision-flip
test with the pool and randomness fixed.

For `E_opt`, record the initial search distribution, seed, every proposal round, forecasts, scores,
elite indices, distribution updates, stopping rule, and final optimizer state. CEM and Nevergrad can
return a mean or recommendation that was never sampled. Evaluate and commit that final action as a
new query before execution. A fixed-randomness intervention must change only the score signal and
show whether it changes the recommendation.

The graph class is not an evidence grade. Record a separate capability vector for candidate-action
queries, adaptive proposal, scoring, selection, deployed forecast use, and physical outcome
validation. Also record whether each fact comes from author prose, source inspection, execution
trace, or controlled intervention.

An action-conditioned predictor remains observational until its causal gate passes. Randomize
executed actions from valid reset states. Preserve failed, reversed, no-op, and low-support
actions. Record proposal, controller conversion, execution, holds, truncation, and overrides.
Use proper scores and calibration on declared later reference-state outcomes. Use physical-outcome
language only for a separately measured physical system. Test hidden-state aliases and abstain
outside declared action support [R136, R138–R139].

For `(V,L,D,A)`, name `D` from this graph. Flex-\(\pi\)'s generated state is an
`intended-future representation`, not action-conditioned dynamics [R128]. Training-only systems
provide a `predictive-trained current-context state`. Use `coupled joint-sampler state` for class
J. Reserve `candidate-action-conditioned predictive state` for an exact class-D or class-E query.
“Counterfactual” still requires the causal gate.

Use \(A^\pi\) for the proposal target. Store controller output and \(A^{exec}\) separately.
Do not treat RGB, DINO, pointmap, or other RGB-derived streams as independent modalities. A
post-decision `D` is not an H1 pre-treatment moderator.

Do not use \(q(F \mid H,L,A^\pi)\) as a source for PID with the same \(A^\pi\) target. The
candidate action is already an input to that state. Use a later declared reference-state outcome
to test simulator forecast value, with the candidate action also available to the matched
baseline, or use the state inside the matched class-D/E selection study. A controller output or
\(A^{exec}\) target can test command prediction only. It cannot establish physical forecast
validity. That claim requires a separately measured physical outcome. Do not confuse downstream
causal descent with source containment.

## 9.3 Target hierarchy

Targets should progress from most identifiable to most consequential. Keep each target family
separate:

1. discrete synthetic or task variable for estimator validation;
2. declared later simulator or reference-state outcome for action-conditioned forecast tests;
3. candidate ranking, selected action, and abstention for fork-level selector tests;
4. policy proposal, controller output, and executed action as three distinct command objects;
5. separately measured physical state change, object flow, or contact event;
6. downstream progress, failure, utility, and process-level safety outcome.

The simulator reference target is a benchmark label under a declared simulator. It is not a
measured physical outcome. A selected command is not evidence of forecast fidelity. A forecast
score is not evidence of complete-policy value. W1 and W2 test those claims separately.

A low-dimensional target can improve estimation but does not solve high-dimensional source geometry. A flow target may be embodiment-portable only after coordinate frame, object correspondence, visibility, and contact semantics are standardized.

## 9.4 Token and temporal aggregation

Pooling is a scientific choice. Candidate approaches include:

- a predefined token subset with architecture semantics;
- attention-independent masked means;
- a fixed learned projection trained only on development data;
- task-variable probes whose outputs, not hidden vectors, become interpretable low-dimensional sources;
- phase-aligned summaries.

Do not average tokens merely because it is convenient. Report sensitivity to a small preregistered set of plausible aggregations. Token selection based on the outcome or intervention effect must occur inside nested training folds.

For every row, bind the target-specific prediction landmark and the maximum timestamp of every
tensor ancestor. Require the landmark to precede target realization or availability. Reject a
source when any ancestor occurs after the landmark. Training-time future supervision does
not violate this rule when the deployed inference path reads only admissible inputs. A diagnostic
loss that rereads future frames does violate it, even when the model calls the result a prediction.

For temporal aggregation:

- use non-overlapping or explicitly dependent windows;
- align by observable task phase only if phase is defined without future outcome;
- avoid using terminal failure to retrospectively define “pre-failure” phase in the primary prospective analysis;
- record variable latency and action-chunk timing;
- record observation time, inference start and finish, committed-prefix indices, command dispatch,
  and measured rather than assumed end-to-end delay;
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

Closed-form conditional-independence Gaussian hierarchy evaluators and sample-covariance plug-in
estimators add a computationally attractive alternative under a jointly Gaussian covariance model
[R126]. For two sources the construction defines redundancy; for three or more it deliberately
does not, so its unique-information, narrow-synergy, total-synergy, and order-\(K\) synergistic-effect
outputs are named hierarchy quantities rather than a complete antichain PID. They do not certify
shared-exclusions atoms, BROJA/\(\sim\), the distinct Gaussian-restricted \(\sim_G\)-PID,
deficiency \(\delta\), Gaussian-channel-restricted \(\delta_G\), the convex-surrogate
\(\widehat{\delta}_G\)-PID, \(\delta^\lambda\), or I-PID [R183–R185], or unrestricted VLA
embedding laws.
Treat each exact quantity and route as a separately named sensitivity or comparator only after law,
measure, and estimator review.

Multimodal interaction decomposition was already developed for multimodal machine learning before this project, using measure choices that are not interchangeable with shared-exclusions PID [R20]. The closest recent precedents include the study of 26 large vision–language models whose official ICLR 2026 record was verified on 2026-08-12, and Sensory PID's conditional audio–video analysis [R18, R113]. BrainFIBRE additionally uses a self-supervised PID-guided multimodal objective with counterfactual modality dropping/swapping in neuroimaging [R100]. Prisoma must distinguish itself by sequential policies, policy/execution/outcome separation, paired and randomized interventions, prospective failure prediction, and estimator abstention—not by the generic use of PID or counterfactual modality perturbations.

## 10.2 VLA diagnosis and interpretability

Mandatory comparison families include:

| Work/family | What it already contributes | What Prisoma must add or test |
|---|---|---|
| Tri-Info [R25] | information-theoretic VLA failure prediction from action diversity, temporal consistency, and action–state coupling | independent implementation/benchmark; incremental value of intervention-grounded and PID features |
| SAFE [R110] | multitask supervised failure detection from VLA internal features across multiple policies and simulated/real settings | capacity- and supervision-matched internal-feature baseline; explicit outer-task holdout, calibration, censoring, and access-cost accounting |
| Hide-and-Seek [R95] | coarsely supervised temporal localization of VLA failure signals; runtime accuracy–timeliness analysis and conformal prediction across three policies, simulation benchmarks, and a real robot | matched-input and matched-cost H2 comparison; explicit censoring, calibration, transport, and conformal-assumption audit |
| Rewind-IL / black-box action monitoring / temporal-difference calibration [R109, R111–R112] | inter-chunk discrepancy and recovery, architecture-dependent kinematic monitor signatures, and explicitly sequential success calibration | separate detection from recovery effects; stratify by action architecture; compare sequential calibration, false alarms, lead time, and utility under identical trajectory access |
| Foresight / ActProbe / VLAConf / perturbation and activation monitors [R101–R105] | strong 2026 alternatives spanning action-conditioned world-model latents, pure action-space features, one-class internal confidence, hidden-activation perturbation disagreement, and activation probes | interface-matched comparator suite; separate gains from supervision, white-box access, resampling, external models, and compute; compare calibration, event recall, false alarms, lead time, and transport |
| CheckVLA [R168] | post-dispatch action-conditioned execution prediction, calibrated triggering, and latency-aware suffix repair with action-shuffle and observation-only controls | separate prediction, detection, and repair effects; match invocation, latency, memory, calibration unit, false alarms, rescue, and harm |
| VLA-Trace [R26] | multi-level tracing, CKA, attention knockout, rollout probes | common capture contract, prospective prediction, and external intervention validation |
| BeTTER [R27] | controlled physical-reasoning interventions and real-world validation | broader provenance/replay substrate and explicit availability–tested-response analysis |
| SAE/feature intervention studies [R28–R30] | sparse features, causal steering/ablation, closed-loop behavioral tests | intervention OOD checks, standardized outcome semantics, cross-policy replication |
| Embodied-reasoning faithfulness / Pinocchio [R31] | distinguishes functional performance from faithful reasoning traces and proposes a behavioral faithfulness critic | do not equate verbalized reasoning with mechanism; ground any trace claim in action and counterfactual effects |
| RoboSemanticBench / physical-reasoning identifiability work [R32–R33] | separates semantic grounding or benchmark success from evidence of action-level response and physical generalization | generalized availability–tested-response–closed-loop-effect benchmark across modalities and pathways |
| VLA-Arena, LIBERO-PRO, Colosseum V2 [R34–R36] | perturbation/generalization/shortcut benchmarks | internal-state and intervention provenance plus held-out predictive tests |

Prisoma should not claim to be the first VLA diagnostic framework. Its novelty depends on enforcing common experimental semantics and testing diagnostic claims against paired algorithmic responses, randomized closed-loop effects, and prospective external validation at the level appropriate to each claim.

### Developments indexed during the 8–9 July 2026 refresh

Four preprints indexed during that refresh sharpen the design without changing its core hypothesis:

- **LaMem-VLA** makes short- and long-term latent memory explicit, reinforcing that memory state and reset semantics should be first-class captured variables rather than hidden inside a generic `D` axis [R68].
- **TouchWorld** combines predictive tactile modeling with a faster reactive contact pathway, reinforcing the need to admit tactile/contact sources and to separate high-level policy decisions from low-level feedback control [R69].
- **LEEVLA** models task-relevant latent environment evolution. Its semantic label and response to
  a named intervention must be tested rather than inferred from architecture [R70].
- **Harness VLA** places a memory-guided agentic harness and retryable manipulation primitives around a frozen VLA under deployment perturbations, creating direct adjacent work for Prisoma’s monitor/intervention layer; Prisoma must distinguish itself through randomized experiment semantics, internal-state provenance, and auditable estimator abstention [R71].

These are new preprints, not settled evidence. Their role in this plan is to update the competing-system and variable-selection landscape, not to import their performance claims.

## 10.3 Embodied datasets and logging

RLDS, LeRobot, MCAP/rosbag2, Open X-Embodiment, DROID, and Rerun establish strong prior art for episodic datasets, streaming logs, cross-embodiment data, and visualization [R37–R48]. The project should build adapters and conformance tests rather than a closed proprietary format.

## 10.4 World models and flow

World-model VLAs and explicit predictive planners are a substantial 2026 research family
[R51–R52]. The term still spans incompatible definitions [R67]. Prisoma therefore uses the
classes in Section 9.2, "World-model experiments," rather than model branding.

The August review changes the prior-art boundary:

- Flex-\(\pi\) generates an intended future before action decoding. Its future cannot attend the
  action stream [R128].
- Reflective VLA conditions each decision on recorded past observation–action–consequence
  triplets. It is history conditioning, not simulation under a candidate current action [R66].
- SLIM, LiLa-WAM, World Tokens, VLA-JEPA, Fast-WAM, and arXiv:2608.09381 JEPA-WAM use future
  prediction for training while deploying a direct policy [R129–R133, R149]. The released
  LiLa-WAM inference loop ignores its returned shared tokens and never calls its future decoder.
  VLA-JEPA's predictor consumes learned VLM latent-action tokens, not clamped robot actions.
- The stage-level JEPA-WAM predicts an intended next-stage latent from observed history and
  language. It conditions short-horizon video and action generation on that latent [R142].
- ForeWAM and Rift create action-independent future-position state in one prefill and expose it to
  the action path. They are class C. Rift's paired cache interventions establish bounded use of
  the tested cache path, not physical correctness or a causal transition [R165–R166].
- SelfWAM and UWM expose optional action-conditioned predictors [R134–R135].
- FACT can score four proposed actions through an action-conditioned value. Its direct mode is
  not a planner [R136].
- \(\tau0\)-WM proposes, simulates, scores, and selects candidates [R137]. CoWAM is the clearest
  reviewed selective class-E design. It uses one fixed candidate pool, typed admission checks,
  calibrated scores, a nominal-action default, and an abstention path [R151]. Both use
  observational forecasts until the causal gate passes.
- World Action Planner is another explicit class-E system. It proposes and refines actions,
  predicts a grid of candidate outcomes, ranks them, and executes the selected candidate. Its
  reported simulation results do not validate the learned predictor as a causal transition
  [R167].
- SG-WAM, Vid2WAM, and MobileWAM remove their predictive teacher or branch at deployment.
  Robust-WAM removes its teacher and alignment head but keeps learned query tokens. They are class
  B because none exposes a callable transition query [R154–R157].
- DynamicWAM and FlowPilot jointly couple future and action streams at deployment. They are class
  J because neither exposes a clamped candidate-action query [R152–R153]. DreamWAM is class B in
  no-rollout mode and class J in joint mode [R158].
- Dyna-2 reports a matched internal predictive-objective comparison and a separate matched
  architecture-family comparison. Its deployed action field does not consume predicted video.
  Neither comparison identifies online future simulation or planning [R141].
- A broader-name search confirms that labels do not define the deployed graph. World-to-Wrist
  keeps the `VLA` label while actions consume an intended future wrist latent. WLA-0 is class B in
  its default mode and class E in its optional candidate-selection mode. LDA-1B is class B in
  policy mode and exposes a separate class-D forward query [R159, R161–R162].
- The released Efficient-WAM sampler uses bidirectional joint attention over future-video and
  action tokens. It is class J, not class C or D. Its source defaults to CUDA, asserts CUDA before
  its nominal attention fallback, and uses float64/complex RoPE. RepWAM had no released inference
  code or weights at the cutoff. Kairos is released, but its report calls direct closed-loop regret
  validation future work [R160, R163–R164].
- MiraBench and the world-model hallucination audit show that visual realism is not action
  grounding [R138–R139].
- XEWorld, PhyLatent, and PSG-JEPA show why forward prediction and global non-collapse are not
  enough. Test physical-state identity, change, action sensitivity, and embodiment transfer
  separately [R143–R145].
- HarnessWAM and TempoWAM add task state, progress checks, recovery, and adaptive replanning around
  finite-horizon policies. Treat those mechanisms as separate system components [R146–R147].
- CheckVLA uses a class-D predictor to verify already committed actions and trigger a suffix
  rewrite. It is a strong H2 monitor comparator, not a candidate planner [R168].
- a small real-time deployment study compares six chunk-scheduling and reconciliation methods on a
  10 Hz bimanual platform. Its strongest general lesson is narrower than its model ranking:
  incorrect observation-to-command alignment creates boundary errors that blending cannot repair
  [R148]. The study uses three tasks and five trials per method–task cell, so treat its method
  ordering as platform-specific evidence.
- Surgical WAM jointly samples future-video and action slots at deployment. Action tokens attend
  the future slots during denoising, but the deployed interface does not expose a clamped
  candidate-action forecast query. Prisoma therefore classifies it as class J, not class D. Its
  matched video-pretraining result is author-reported on four simulated surgical tasks. No
  official code or checkpoint was verified at the review cutoff [R150].

An exact-phrase arXiv query found 36 August 2026 “world action model” submissions through
13 August 2026. The dated frontier review gives every item a typed disposition. This is a bounded
search cohort, not a systematic-review claim.

These findings do not show that VLAs are dead. A VLA usually names the policy interface. A WAM can
name a training objective, backbone, prediction path, or planner. Current systems often combine
these roles.

For every claimed predictive state, ask:

- Is the variable externally accurate under a proper score?
- Does it respond to the proposed action?
- Does the policy use it at deployment?
- Does a named intervention change action generation?
- Does its relationship survive controller conversion and embodiment change?
- Does it beat state, action, kinematic, uncertainty, and cost-matched baselines?

The matched mechanism experiment has six arms:

1. action-only training and direct deployment;
2. action plus future loss and direct deployment;
3. intended-future-conditioned action deployment;
4. coupled joint future-action sampling without a clamped candidate-action query;
5. action-conditioned prediction and scoring that cannot alter the executed direct-policy
   proposal; and
6. the same candidate predictions and scores with frozen score-based selection among at least two
   proposals.

Match backbone, data, optimizer, parameter budget, compute, task, controller, and evaluation.
Match arms 5 and 6 on proposals, predictions, scores, and compute. In arm 5, execute the frozen
direct-policy proposal regardless of score. Validate the predictor under randomized executed
actions. Require arm 6 to pass a fixed-proposal decision-flip test. This separates predictive
training, runtime intended-future use, coupled joint generation, action-conditioned forecast
validity, and selection utility.

The local path is world-model-first. `pid-world-model-reference` is the zero-model-download first
rung. A clean Rust build can still fetch pinned Cargo dependencies.
It learns an affine action-conditioned transition, commits a fixed pool, labels independent
restored branches, executes only through the Agent Bridge, and verifies replay. Its training and
reference laws are the same deterministic simulator. It proves software semantics only. It does
not estimate W1, W2, model quality, physical truth, or planning benefit.

Run-log schema 2 has no neutral inline decision record. The reference therefore carries forecast
commitments and execution receipts in strictly named `label_observed` compatibility envelopes.
They are not outcome labels. A future upstream schema must separate these records before the path
becomes a general learned-model adapter.

The first external M4 target is the compact LeWM PushT path [R181, R182]. Pin code to
`Mengarr/lewm@8a2c595813d0eee85b2dbffa6f58ff0842f9e673` and bind its committed `uv.lock`
at SHA-256 `1bf638a080ce7717ee000f5b0be9de1ca327624025ba52433c7fbcbcc90d024e`.
That lock selects `stable-worldmodel==0.1.1` from wheel SHA-256
`00eaabd9e046e6364b3d1db47e5b365a0f628aea3a9376d6a407f75cbbbd2ef5`. The package's
source tag resolves to `15a5538d492ae524c64cb18cc56a2d70611e877e`. It also selects
`stable-pretraining==0.1.7` from wheel SHA-256
`60fc8fc3c9490e9a059aa7e038ab62cbe0505841e78c4165c18a99d8f599ec65`.
Pin weights to `quentinll/lewm-pusht@22b330c28c27ead4bfd1888615af1340e3fe9052`. The `weights.pt` artifact is
72,290,721 bytes with SHA-256
`48938400ae3464c9680731287f583a9cb516f55a8ec64ea13a91be47fb15b607`. Its deployed query accepts
current visual state plus action blocks, and its released evaluator performs CEM planning. The
paper reports about 15 million parameters. These are source and artifact facts, not M4 performance
or qualification claims.

Do not substitute current `stable-worldmodel` main for the locked package. Main revision
`9a66d7d020043c8efb507f45373e808714f0842d` takes a `cost` object in the planner constructor.
LeWM's pinned evaluator passes `model`. The pair does not compose without a reviewed migration.
Treat current main as a separate port target, not an exact reproduction dependency.

LeWM is the first port because it is smaller than the reviewed JEPA-WM runtime, has MIT-licensed
project source and model-card terms, and uses standard PyTorch attention in its core predictor.
The exact `stable-worldmodel` package metadata also declares MIT. Its reviewed source tree lacks a
license file, so rights review must resolve that discrepancy before adoption. Code, package,
checkpoint, data, and paper licenses remain separate.

The upstream end-to-end evaluator is not MPS-ready. It hard-codes CUDA, its documented conversion
uses unsafe pickle-enabled loading, and the official paper result does not include an M4 run.
Prisoma must instantiate the exact architecture, verify the digest, and load the state dictionary
with `weights_only=True`. No released stack in the reviewed cohort has an unmodified, verified MPS
propose–predict–score–select path. The Apple work remains a bounded compatibility port.

A 2026-08-14 local synthetic probe used the exact 0.1.1 wheel and checkpoint. Direct prediction,
latent rollout, and the published 30-by-300 CEM loop produced finite repeatable MPS outputs. The
probe did not run official image preprocessing, PushT, or closed-loop replanning. It did not test
CPU/MPS candidate-order parity. It therefore establishes only that these reviewed tensor paths can
execute on this M4 Max. It does not establish MPS support, WM2, or model quality.

A one-seed independent reproduction tested the smaller LeWM TwoRoom task [R182]. It did not test
PushT, MPS, or seed variance. It found four consequential pipeline conventions outside released
configuration files. It also found conflicting goal offsets, step budgets, and CEM iteration
counts across released sources. On three checkpoints, one-step prediction error did not order
long-horizon planning performance. These findings reinforce two boundaries. W1 forecast evidence
cannot substitute for W2 closed-loop value, and a model arm is undefined until its full evaluation
protocol is frozen.

The first parity port must preserve the published PushT configuration: 30 CEM rounds, 300 samples,
30 elites, horizon five, and action blocks of length five. Retain every proposal, forecast, score,
elite set, mean, and variance. Separately score and commit the final recommended action before
execution. First reconcile the paper, configuration, and executable code for preprocessing,
action gathering, action width, missing-value handling, normalization, goal creation, episode
selection, horizons, budgets, and replanning. Freeze every unresolved feasible reading as a
separate arm before outcomes. Port device and random-number ownership without changing planner
mathematics. A smaller search becomes a distinct, predeclared M4 arm only after exact reproduction.

The published evaluator fits each `StandardScaler` on the evaluation dataset. Its CEM solver uses
the action space only to obtain dimensions. It does not clamp sampled standardized actions to the
space bounds. Prisoma must not inherit either behavior in a confirmatory study. Freeze a
training-only scaler receipt. Inverse-transform every proposal, check raw-action support, and apply
one predeclared reject, project, or truncated-sampling rule. Report support violations. A bounded
or projected solver is a distinct port arm until exact-reproduction results are recorded. The
unmodified evaluator remains a descriptive reproduction arm, not a held-out W1 or W2 analysis.

The pinned JEPA-WM PushT stack remains the second planning benchmark [R173]. It uses a larger
39.7-million-parameter runtime and a 211,639,615-byte checkpoint. Its upstream agent and planners
hard-code CUDA, and its hub path discards a requested MPS device when CUDA is absent. Its
CC-BY-NC-4.0 terms also make it a weaker default foundation for a reusable local pipeline.

The port must make unrelated dataset imports lazy. Admission requires content hashes, zero network
access during execution, finite CPU and MPS outputs, action sensitivity above repeat-run drift,
CPU/MPS candidate-order parity under frozen tolerances, a reconstructable adaptive search trace,
more than one closed-loop replan, and measured resource receipts. Benchmark the upstream CEM first.
Only then freeze a reduced-budget CEM arm for M4 use. Never call that reduced arm an upstream
reproduction.

Measure cold start separately. For steady state, use at least 1,000 decisions or a larger
predeclared count justified for p99 precision. Record the exact M4 model, OS, PyTorch build, device,
power mode, thermal state, warmup, concurrency, and clock. Report empirical p50, p95, and p99 with
uncertainty, deadline misses, peak unified memory, and fallback. Until all gates pass, call the
stack an **MPS candidate**, not MPS support.

SmolVLA remains a small direct-policy baseline because LeRobot documents an MPS path [R140].
VLA-JEPA remains a predictive-co-training comparator. Its policy inference drops the predictor,
and its future loss does not expose a clamped physical-action transition query [R131]. Neither is
the primary world model. Large video WAMs and MLX rewrites remain later work. Weight conversion
alone cannot reproduce preprocessing, masks, rotary math, integrators, normalizers, planners, or
capture hooks.
The complete dated review is
[`WORLD_ACTION_MODEL_FRONTIER.md`](docs/audits/2026-08-12-first-principles/WORLD_ACTION_MODEL_FRONTIER.md).

## 10.5 Safety and correction

Current work studies safety benchmarks, safety-aware planning, correction, and replanning [R54–R58]. ForesightSafety-VLA further makes process-level cumulative safety cost, risk-exposure time, and safe/unsafe success/failure quadrants explicit across controlled visual, language, and scene variations [R56]. Prisoma may contribute process-level evidence and failure tracing. It must not claim certification or operational safety solely from diagnostic performance.

## 10.6 Internal repository ecosystem is context, not prior-art evidence

Prisoma exists within a public ecosystem of estimator, protocol, robotics, perception, asset, and visualization projects [R72–R86]. This can reduce engineering cost and create transport settings, but it must not inflate novelty or validation claims.

The direct audited dependencies are the pinned `pid-rs` submodule and the optional NCP observer.
Galadriel and Crebain share dependencies or protocol context but have no direct Prisoma edge.
Manwe documents that adaptation is required. The named `sepahead/engram` repository remains a
placeholder. The executable Engram Neural Labs host is `sepahead/Paper2Brain`. Its byte-locked
Prisoma manifest, generic live read-only adapter, and structural preview create an E2 consumer
edge, not E3 scientific conformance. The host does not start or attest Prisoma. No NCP producer,
bridge, wire translator, artifact-validation path, or authority path exists. WorldWarp and GauSS-MI
remain optional specifications. The ReScience/BROJA repositories are excluded historical
lineage. The
NEST fork is a candidate producer, not integration or validation. Mesh and visualization
repositories are candidate inputs or outputs.

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

The dated search must include at least: `partial information decomposition robot`, `information
decomposition vision language action`, `VLA failure diagnosis`, `embodied mechanistic
interpretability`, `causal intervention VLA`, `action grounding benchmark`, `robot policy internal
state logging`, `world action model`, `action conditioned world model`, `future conditioned robot
policy`, `robot world model hallucination`, `candidate action world model planning`, `robot
Gaussian splatting simulator`, `3DGS closed-loop policy evaluation`, `mesh Gaussian splat matched
rendering`, `real-to-sim robot policy evaluation`, and `world model simulator fidelity`.

---

# 11. Thesis architecture and publication strategy

## 11.1 Minimum viable thesis

The minimum defensible thesis is world-model-first. It does not require PID success or a 3DGS
benefit.

### Paper A — supported world-model decisions under a local budget

**Contribution:** a supported-action calibration and decision protocol that links proper forecast
scores, support-shift abstention, fixed-pool or adaptive search traces, and randomized complete-policy
value under a frozen M4 budget. Exact forks and hashes are enabling controls, not the novelty claim.

**Required evidence:** W1 and W2 frozen separately; held-out task and dynamics families; supported
randomized actions; direct-cost, kinematic, current-only, shuffled, no-future, and proposal-headroom
controls; calibrated abstention; randomized complete policies; and measured latency, memory, power
proxy, deadline, and fallback receipts. The design must state how it extends CoWAM and
WorldSimProbe rather than relabeling their fixed-pool and simulator-fidelity controls.

### Paper B — linked fidelity tomography

**Contribution:** linked matched contrasts that localize decision error across declared reference
dynamics, learned prediction, mesh-versus-3DGS observation substrate, frozen-policy response,
controller conversion, and selection.

**Required evidence:** identical physical trajectories and cameras across renderer treatments;
unchanged collision geometry; identical fork/action identities across learned and reference paths;
frozen policies; candidate-level and episode-level estimands kept separate; and replication on a
second task or scene family.

This is an integration protocol, not a priority claim. SplatSim, DISCOVERSE, GSWorld, Real-to-Sim
Robot Policy Evaluation, RoboSnap, WorldSimProbe, RoboWM-Bench, CoWAM, and World Action Planner
already occupy major parts of the broad idea [R151, R167, R174–R180]. The narrower contribution is
cross-substrate attribution under matched state and camera identity. It links renderer and learned-
model errors to frozen-policy response and downstream selection value.

### Paper C — conditional diagnostics and shared-exclusions study

**Contribution:** either validated, typed shared-exclusions quantities add reproducible
incremental diagnostic value, or a rigorous boundary shows where they abstain or lose to simpler
signals.

**Required evidence:** the scientific-object firewall; a project-owned exact categorical fixture;
measure-specific continuous validation where attempted; all four gates; strong non-PID baselines;
negative results; a second model or task-family replication; and one content-bound process packet.
That packet must preserve the method decision, mathematics, applicability, route, execution,
review, and publication stages. Its canonical Markdown and deterministic PDF view must have an
exact build and page-review receipt.

If Paper C cannot support a meaningful PID estimand, replace it with a dedicated availability–
tested-response–closed-loop-effect or estimator-abstention paper. This preserves thesis coherence
rather than forcing PID.

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

- W1/W2 variable dictionaries, units, supported action population, outcomes, useful margins,
  comparator frontier, resource bounds, and separate decision rules;
- W3 linked-panel treatment and nuisance map, with a proof that rendering cannot alter physics;
- causal graph, variable dictionary, treatment-version ontology, and interference/reset boundary;
- world-model source, action, prediction-landmark, reference-target, scorer, selector, controller,
  and outcome contracts;
- complete W1/W2 estimand table, target populations, and positive useful margins;
- pre-treatment feature whitelist and automated lineage rule;
- baseline, intervention, outcome, competing-event, and censoring definitions;
- transport/contamination ledger and dated literature ledger;
- ecosystem evidence ledger, dependency firebreak, and optional-component map.

**Stop:** unresolved ambiguity about treatment, target, time zero, unit, action support, target
population, causal interpretation, renderer/physics separation, or whether a component is a
dependency versus an optional testbed.

The current `world_model_claim_registry_v1.json` is only a fail-closed status and claim-language
ledger. It is not a preregistration, freeze candidate, holdout receipt, or scientific result. WM0
remains open until a reviewed world-model contract binds every item above and registers untouched
evaluation data.

**Preserved diagnostic-governance boundary (reconciled 2026-08-12).** The repository ships the historical
`unfrozen_draft` v1 scaffold unchanged as a non-promotable intake artifact and a separately
revised typed v3 successor draft whose freeze-bearing values are all null. The superseded v2
bytes remain as historical process evidence. The first-principles
H2 correction reopened its scientific and statistical review, so its current status is unreviewed.
The v3 draft content-binds v1 and encodes the missing EC1 finite acceptance protocol. A future
candidate populates EC1, H2, one selected H1 contract, and one selected H3-or-H4 contract. Every
inactive branch remains null. A fresh-sample H4 switch may retain the prior frozen H3 contract as
history. Its H1-A
contract types the response functional, proper score, matched-access comparator, positive useful
margin, one-sided superiority rule, uncertainty, calibration acceptance and failure consequence,
multiplicity, and finite-benchmark or replication scope. Its H1-B contract types the primary
effect-specific endpoint and hierarchy, positive-margin one-sided success rule, complete
effect-validation stack, ITT and design checks, uncertainty, and directional replication.
Factual-outcome loss remains a secondary descriptive diagnostic. The draft also types H2
target/censoring/one-primary-scoring-contract/success obligations,
H3 full-target-ledger exact same-fold \(M_1\)-substitution and warning-disposition rules, H3/H4 claim selection,
and H4 target/sampling/transport/tuple/inference/power obligations. EC1 detection acceptance is
pairwise: every registered fault–adapter pair must
have its own absolute sensitivity floor and pass independently, with no distribution-average
substitution. Both default validators pass only
their honest unfinished states; both strict freeze-ready modes remain closed. The bundle also
states **no confirmatory holdout is registered**, contains one hash-chained non-access genesis
event, has empty source/target-pending transport and contamination assessment arrays, and imports
a dated legacy reference inventory whose database queries, candidate universe, criteria, and
per-candidate decisions are absent. Passing the default audits means only that this unfinished
state is internally consistent. It does not supply a freeze receipt, prove historical or
off-repository non-access, establish absence of contamination, or constitute a systematic review.
The historical v1/holdout/transport/literature intake remains dated 2026-07-13. The preserved
diagnostic claim registry is dated 2026-08-13 and content-binds the revised, unreviewed typed
successor at that date;
the validator permits that current registry to advance only without predating or rewriting the
historical intake.
The freeze-ready modes must remain failed until the domain, protocol, estimands, minimum useful
effects, holdout commitments, transport evidence, comparator dispositions, and fresh reproducible
search are genuinely complete.
Even then, the machine audit establishes completeness, content binding, and internal consistency;
it cannot adjudicate the substantive adequacy of the scientific choices or independent reviews.
The checked v1 scaffold remains intentionally non-promotable. The checked v3 successor is a draft
contract, not a completed candidate: a real freeze requires a separately reviewed candidate that
fills its typed obligations with content-bound receipts and passes the strict validator rather
than editing v1 or treating the null draft as a preregistration.
The H3 ancestry bindings currently name structural placeholders only. No project-owned ancestry
producer, consumer validator, or per-row receipt schema exists. Therefore an H3 candidate carries
the stable `M0_SUCCESSOR_H3_ANCESTRY_PRODUCER_CONSUMER_AND_RECEIPT_UNIMPLEMENTED` blocker, and the
validator rejects an H3 terminal `frozen` state. A file extension, role label, distinct path, or
matching digest cannot clear this blocker. H3 can freeze only after executable role-specific
artifacts exist and the validator checks their semantics.
For a terminal `frozen` state, the validator removes circularity by hashing the complete reviewed
candidate with status normalized to `freeze_candidate_under_review` and all three terminal metadata
fields set to null, using sorted-key compact UTF-8 JSON. The typed receipt and `freeze_revision`
must bind that same digest, frozen timestamp, and the four reviewed global freeze-slot artifacts.
This rejects an arbitrary receipt file, invented digest, or post-review candidate edit. The hash is
not a digital signature or reviewer-identity attestation, and validator passage still cannot
establish review independence or scientific adequacy.

This diagnostic bundle does not block WM1–WM6. If the D4–D6 diagnostic family is activated, freeze
EC1, H2, the selected H1 protocol, and the selected H3-or-H4 branch before its holdout. Keep every
inactive branch null. Passing diagnostic governance cannot substitute for WM0.

## M1 — exact-fork decision contract

Deliver:

- native propose–predict–score–select reference;
- immutable fork and ordered candidate-pool hashes;
- typestate-enforced pre-oracle publication;
- independent restored candidate branches;
- bridge-only selected execution;
- decision-flip, tamper, replay, and resource-limit tests;
- generic Rerun conversion without control authority, treated only as an optional derived view.

**Boundary:** this milestone proves software semantics only. Do not place it in W1/W2 result tables.

## M2 — M4 learned-world-model qualification

Deliver:

- exact LeWM code, checkpoint, and dependency pins;
- a source-concordance ledger for preprocessing, action gathering, normalization, missing-value
  handling, goal construction, episode selection, horizons, budgets, and planner settings;
- pre-outcome frozen arms for each unresolved feasible paper, configuration, or code reading;
- reviewed weights-only checkpoint ingress and offline execution;
- CPU and MPS predictor parity under frozen tolerances;
- fixed-action sensitivity beyond repeat-run drift;
- exact published PushT CEM reproduction before any reduced-budget arm;
- a reconstructable adaptive-search ledger, including the separately scored final mean;
- one multi-replan PushT episode;
- cold-start and at least 1,000 steady-state decisions on the named M4 configuration;
- p50/p95/p99 latency with uncertainty, peak unified memory, power proxy, deadline, and fallback receipts;
- explicit rights receipts for LeWM, its weights, `stable-worldmodel`, and task data;
- a second-rung JEPA-WM comparison only after its separate noncommercial-rights decision.

**Stop port:** hidden CPU fallback, candidate-order instability, unsupported actions, missed frozen
resource limits, mutable network-loaded code, or no selection dependence on forecasts. Keep the
native contract oracle and evaluate another compact released model.

## M3 — W1 forecast pilot and secondary ranking study

Deliver:

- supported randomized-action capture from valid reset states;
- held-out dynamics and task families;
- current-only, current-plus-action cost, kinematic, no-future, action-shuffled,
  future-shuffled, and oracle-pool controls;
- proper-score, calibration, ranking, support-distance, and abstention diagnostics;
- design-analysis parameters for one frozen W1 primary score.

**Stop W1:** no support region with stable prediction, no useful margin over the strongest simple
baseline, or calibration/resource gates fail.

## M4 — W3 renderer and policy matched panels

Deliver:

- one physics trajectory and camera ledger rendered through mesh and 3DGS without changing state;
- authoritative state plus body/link-to-representation mapping;
- content-bound asset lineage, camera, renderer, frame, color, exposure, and timing identities;
- explicit rigid, articulated, deformable, background, and robot rendering rules;
- reset receipts for policy memory, KV cache, history, and randomness;
- pixel, feature, policy-action, and closed-loop contrasts with distinct estimands;
- learned-versus-reference forecasts on the same fork and action pool;
- linked error localization without additive-cause language.

**Invalidate renderer treatment:** treatments are not state-identical, collision geometry changes,
or representation, frame, camera, timing, or policy-reset binding fails.

**Valid null:** all identity gates pass but 3DGS has no useful decision effect. Report this negative
result. Do not relabel it as an invalid treatment.

## M5 — locked W1

Deliver:

- frozen W1 population, action support, later outcome, proper score, useful margin, baseline,
  support/calibration/resource gates, and cluster-aware uncertainty;
- untouched task-family holdout;
- error, inversion, subgroup, and abstention ledgers.

**Stop W1 claim:** lower confidence bound misses the useful margin or any non-rescuable gate fails.

## M6 — locked W2

Deliver a randomized comparison of complete policies on independent reset blocks. Freeze one
episode-level endpoint, same-budget comparators, proposal count, controller, deadline, fallback,
useful margin, and intention-to-treat analysis. Report fork-level selector quantities only as
secondary diagnostics.

**Stop W2 claim:** no useful complete-policy improvement, deadline/resource failure, or benefit
exists only after excluding abstentions, fallbacks, or failed executions.

## M7 — conditional diagnostic branch and transport

Activate the preserved H1/H2/H3/H4 branch only after W1/W2 define a useful diagnostic problem.
Shared-exclusions work first needs the exact categorical fixture and its applicable gates. Then use
a second task family, policy, simulator, embodiment, or real-robot setting. Do not generalize beyond
the variation actually replicated.

---

# 13. Fifty-lens adversarial review incorporated into the plan

This section records independent questions that must be answered before a claim survives. It is
not a vote or a claim of expert consensus. Each lens has a concrete failure condition. Overlap
between lenses does not let one favorable result rescue another failed requirement.

## Lens 1 — information theory

**Question:** Is the quantity defined, finite, measure-specific, and invariant only under transformations for which invariance is proved?

**Failure condition:** atom labels are treated as universal, deterministic continuous MI is assumed finite, or a different PID measure is substituted without changing the claim.

**Design consequence:** name the functional, exact output coordinate, and route; validate each at
its applicable level; make PID conditional.

## Lens 2 — causal inference

**Question:** What intervention or identification assumption connects an observational diagnostic to a named policy response?

**Failure condition:** correlation, decodability, or mutual information is described as natural pathway use.

**Design consequence:** exact paired responses are the reference for frozen-snapshot algorithmic
claims. Randomized effects are the reference for closed-loop claims. Availability, tested response,
and closed-loop effect remain separate.

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

**Design consequence:** locked landmarks, frozen scoring and censoring contracts, calibration,
decision utility, and TRIPOD+AI/PROBAST+AI review [R59–R60].

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

**Failure condition:** a custom renderer, world-model training from scratch, real-time PID, and
multiple robot stacks all become prerequisites for the first world-model paper.

**Design consequence:** Paper A/B/C structure, strict gates, optional adapters, and kill rules.

## Lens 20 — philosophy of science and falsifiability

**Question:** What result would make the project abandon, narrow, or reverse its favored explanation?

**Failure condition:** every null result is redescribed as evidence that PID needs refinement.

**Design consequence:** minimum useful effects, disconfirming outcomes, immutable primary endpoints, and a PID-independent success path.

## Lens 21 — metrology and construct validity

**Question:** Does each recorded variable measure the named construct at the declared scale and resolution?

**Failure condition:** a convenient tensor, proxy label, or clock value silently replaces the construct of interest.

**Design consequence:** define the measurement model, calibration chain, resolution, error sources,
and admissible interpretation for every primary variable.

## Lens 22 — missing data and censoring

**Question:** Which processes make values unavailable, censored, truncated, or undefined, and how do they alter the target?

**Failure condition:** complete cases are treated as representative, or one censoring correction is called a universally proper score.

**Design consequence:** type every missingness path. Freeze the target, score, nuisance model,
positivity check, and sensitivity analysis together.

## Lens 23 — transportability and external validity

**Question:** Which population differences can be adjusted, and which require a new experiment?

**Failure condition:** a task, simulator, robot, or policy result is generalized beyond observed support.

**Design consequence:** declare source and target populations, measured effect modifiers, support
checks, transport weights, and replication boundaries.

## Lens 24 — interference and shared environment

**Question:** Can one unit's treatment, cache, scene state, operator, or resource use affect another unit's outcome?

**Failure condition:** SUTVA is assumed while episodes share state, learning, queues, or human adaptation.

**Design consequence:** isolate units or model interference. Randomize at the level where spillover
occurs and record shared-state resets.

## Lens 25 — uncertainty quantification and sensitivity

**Question:** Does reported uncertainty include sampling, clustering, nuisance fitting, selection, numerical error, and design uncertainty?

**Failure condition:** one bootstrap interval is presented as total uncertainty under unsupported exchangeability.

**Design consequence:** name each uncertainty source, justify the resampling unit, and report
sensitivity to defensible alternative assumptions.

## Lens 26 — numerical analysis and floating-point semantics

**Question:** Are finite ranges, conditioning, tolerances, summation order, and platform differences bounded before computation?

**Failure condition:** finite inputs overflow, NaNs enter comparisons, or a tolerance becomes an unrecorded scientific parameter.

**Design consequence:** use checked arithmetic, stable formulas, explicit finite checks, recorded
tolerances, and cross-platform numerical fixtures.

## Lens 27 — algorithmic complexity and resource bounds

**Question:** Is time, memory, I/O, pairwise work, and output growth bounded for every public path?

**Failure condition:** a nominally small input triggers unbounded expansion, quadratic work, blocking I/O, or oversized publication.

**Design consequence:** project work before execution. Apply typed aggregate limits and test each
limit at every ingress and publication boundary.

## Lens 28 — hardware portability and accelerator semantics

**Question:** Does the same declared computation run on CPU, CUDA, MPS, or another target without silent semantic change?

**Failure condition:** unsupported dtypes, fallback, fused kernels, or device-specific randomness change the result without a receipt.

**Design consequence:** qualify each hardware path separately. Compare deterministic fixtures,
latency, memory, dtype, fallback, and output tolerance.

## Lens 29 — compiler, runtime, and language semantics

**Question:** Which compiler, interpreter, loader, optimization, and undefined behavior can affect execution?

**Failure condition:** source hashes are treated as proof of loaded code, or unsafe behavior varies across toolchains.

**Design consequence:** pin toolchains, bind runtime identity, deny warnings, test release builds,
and document the remaining execution-attestation boundary.

## Lens 30 — schema evolution and API compatibility

**Question:** Can producers and consumers distinguish compatible extension, migration, deprecation, and breaking change?

**Failure condition:** an additive-looking field changes meaning, or a compatibility alias weakens the canonical wire contract.

**Design consequence:** version schemas, deny unknown fields where required, test migrations, and
publish exact source and wire compatibility notes.

## Lens 31 — reliability and fault tolerance

**Question:** What state remains after crashes, partial writes, retries, storage loss, and restart?

**Failure condition:** a successful response lacks durable provenance, or retry adopts bytes not verified on the same descriptor.

**Design consequence:** define failure atomicity, no-clobber publication, sync boundaries, retry
identity, recovery behavior, and honest incomplete states.

## Lens 32 — observability and diagnostic coverage

**Question:** Can the system distinguish model, controller, transport, storage, timing, and operator failures?

**Failure condition:** one generic error or success metric hides the component and boundary that failed.

**Design consequence:** emit typed events, component-local counters, boundary receipts, and negative
tests for each supported failure mode.

## Lens 33 — data engineering and label lineage

**Question:** Can each row, label, split, transform, and exclusion be reconstructed without filename or directory inference?

**Failure condition:** order, episode identity, success, or provenance is guessed from storage layout.

**Design consequence:** require typed manifests, canonical row keys, exact split receipts, label
ontology, and conflict rejection.

## Lens 34 — adversarial robustness and distribution shift

**Question:** Which realistic corruptions, attacks, and support shifts can alter decisions or bypass gates?

**Failure condition:** nominal benchmark success is used as evidence against sensor, prompt, model, or artifact attacks.

**Design consequence:** freeze a threat model, test bounded perturbations and corruptions, retain
failed cases, and separate robustness from security.

## Lens 35 — optimization and training dynamics

**Question:** Are gains caused by the proposed mechanism rather than compute, initialization, schedule, regularization, or selection?

**Failure condition:** unmatched training budgets or best-run selection are attributed to one architectural component.

**Design consequence:** match budgets, log learning dynamics, use multiple seeds, freeze checkpoint
selection, and run mechanism-specific ablations.

## Lens 36 — evaluation contamination and leakage

**Question:** Could training, pretraining, tuning, retrieval, caching, or human review expose evaluation content or close variants?

**Failure condition:** a nominal holdout shares tasks, templates, assets, operators, or derived statistics with model selection.

**Design consequence:** audit lineage and similarity, freeze access rules, record holdout access,
and use fresh external families where contamination cannot be excluded.

## Lens 37 — decision theory and utility

**Question:** Does a predictive improvement change a prespecified decision under explicit costs and alternatives?

**Failure condition:** a statistically better score is called useful without thresholds, actions, harms, or opportunity costs.

**Design consequence:** freeze the decision rule, comparator, payoff table, abstention action, and
minimum useful improvement before outcomes open.

## Lens 38 — online adaptation and drift

**Question:** How do policy updates, operators, environments, sensors, and task prevalence change after deployment?

**Failure condition:** a frozen validation result is applied after the data-generating process or model has changed.

**Design consequence:** version every update, monitor declared drift signals, preserve pre-update
baselines, and require revalidation after material change.

## Lens 39 — calibration, selective prediction, and abstention

**Question:** Are confidence, risk, coverage, and abstention valid for the target population and decision rule?

**Failure condition:** a score rank or synthetic calibration curve is treated as calibrated probability or safe abstention.

**Design consequence:** predeclare calibration metrics, coverage targets, subgroup checks, and the
same-fold substitution behavior for every abstained case.

## Lens 40 — formal specification and model checking

**Question:** Which invariants can be stated precisely and checked over all relevant states or traces?

**Failure condition:** tests over examples are described as proof of a protocol or concurrency invariant.

**Design consequence:** separate executable tests, bounded checks, proof artifacts, assumptions,
and unproved claims. Bind each artifact to exact source.

## Lens 41 — independent replication

**Question:** Can a separate team reproduce the procedure and obtain a compatible result without hidden coordination?

**Failure condition:** rerunning the same code, data, and authors is called independent replication.

**Design consequence:** publish complete artifacts, preserve a clean-room path, and require a
different operator or site for replication claims.

## Lens 42 — selective reporting and publication bias

**Question:** Are all attempted families, outcomes, seeds, exclusions, and protocol deviations visible?

**Failure condition:** only favorable tasks, checkpoints, summaries, or analytic variants survive into the report.

**Design consequence:** register the analysis family, retain failed evidence, disclose deviations,
and distinguish confirmatory results from post-outcome exploration.

## Lens 43 — claim language and technical communication

**Question:** Can a reader map every important sentence to an estimand, artifact, result, limitation, or roadmap item?

**Failure condition:** terms such as world model, causal, validated, safe, or real-time change meaning across documents.

**Design consequence:** use one term per concept, keep a claim registry, state non-claims near
results, and audit the complete active docset.

## Lens 44 — maintainability and operational ownership

**Question:** Can another maintainer understand, test, update, and safely retire each component?

**Failure condition:** one person's memory, machine, credential, or unpublished script is required for a critical path.

**Design consequence:** document ownership, invariants, recovery, deprecation, and minimal runbooks.
Keep the core smaller than the evidence it protects.

## Lens 45 — dependency lifecycle and ecosystem governance

**Question:** Are upstream revisions, advisories, maintenance status, and compatibility boundaries reviewed continuously?

**Failure condition:** a pin is called safe because it is immutable, or a newer upstream is adopted without consumer review.

**Design consequence:** maintain a dependency ledger, advisory policy, update cadence, firebreak,
and exact consumer acceptance matrix.

## Lens 46 — artifact custody and authenticity

**Question:** Who produced, transferred, reviewed, and published each artifact, and what identity does the evidence establish?

**Failure condition:** a hash proves reviewer identity, build provenance, or trusted origin that it cannot establish.

**Design consequence:** separate integrity from authenticity. Bind custody events, signatures or
attestations, exact revisions, and reviewer roles where the claim requires them.

## Lens 47 — compute economics and environmental cost

**Question:** What compute, energy, storage, operator time, and hardware opportunity cost buys the reported improvement?

**Failure condition:** a low-parameter model is called low overhead while its backbone, decoding, data, or training cost is omitted.

**Design consequence:** report end-to-end resource accounting and matched budgets. Prefer the
smallest system that clears the frozen utility gate.

## Lens 48 — ecological validity and human-robot interaction

**Question:** Do laboratory tasks preserve the interruptions, ambiguity, latency, recovery, and human behavior relevant to use?

**Failure condition:** scripted, reset-heavy success is generalized to sustained operation with people or changing environments.

**Design consequence:** define the use context, include recovery and interruption scenarios, and
measure operator burden and unsafe surprises.

## Lens 49 — model selection and researcher degrees of freedom

**Question:** How many models, layers, measures, prompts, windows, thresholds, and summaries could have been selected?

**Failure condition:** selection occurs on the evaluation set or disappears from multiplicity accounting.

**Design consequence:** freeze selection rules, use nested evaluation, preserve the candidate
ledger, and control the correct family of claims.

## Lens 50 — counterfactual stress testing and red teaming

**Question:** Which minimal change should reverse the claimed mechanism, decision, or interpretation?

**Failure condition:** the result survives because the test never creates a discriminating counterfactual.

**Design consequence:** design negative controls, decision-flip tests, adversarial traces, and
near-boundary cases before accepting the preferred explanation.

---

# 14. Risk register

| Risk | Probability | Impact | Leading indicator | Mitigation | Decision |
|---|---:|---:|---|---|---|
| Learned predictor is action-insensitive or wrong on supported consequences | high | high | forecasts do not change with action; W1 score/calibration failure | narrow support, change model/target, or stop the world-model thesis | stop W1/W2 if no qualified model remains |
| M4 planner misses latency, memory, parity, or fallback gates | medium–high | high | CPU fallback, deadline misses, candidate-order drift, or memory pressure | reduce a prespecified search budget only after exact reproduction; change candidate | stop the port if WM2 fails |
| Simulator reference dynamics do not transfer to physical outcomes | high | high | rank reversals or support gaps across dynamics | bound W1 to the simulator; add matched dynamics panels and later physical validation | prohibit physical-world claims |
| Renderer substrate changes policy input rather than only measurement | medium | high | camera/state mismatch or policy response changes under nominally matched frames | content-bind state/camera trajectories; separate immediate and downstream panels | stop W3 decomposition |
| Prior work already contains the claimed protocol contribution | medium–high | high | collision with CoWAM, WorldSimProbe, or related controls | claim only the linked cross-substrate integration and measured residual | remove priority or novelty language |
| No meaningful finite PID estimand for chosen tensors | high | low | deterministic or near-deterministic path; oracle mismatch | use a non-PID diagnostic or stop H3 | kill H3; continue W1/W2 |
| Continuous estimator fails planned regime | high | low | bias, coverage, or abstention failure | stop that route; never auto-route the result; only start a separately preregistered categorical estimand on explicitly transformed variables, or use an MI-free diagnostic | continue W1/W2 and any eligible H1/H2/H4 work |
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
| Multiple-testing inflation | high | high | many layers/measures/windows | locked primary branch plus strong familywise/simultaneous confirmatory control; FDR only for secondary work | exploratory labels |
| Compute/runtime prevents required resampling | medium | medium | pilot exceeds budget | approximate only after validation; cache distances; narrow grid | reduce scope |
| Repository/spec drift | high | medium | docs disagree with tests/manifests | generated capability matrix, CI documentation checks | block release |
| Security/privacy incident | low–medium | high | remote mutation or personal data exposure | local defaults, access control, redaction, retention | halt affected collection |
| PhD scope expands into product building | high | high | optional UI or simulator blocks experiments | enforce milestone dependencies and paper deliverables | defer optional product work |

## 14.1 Top three existential risks

1. **The learned model does not preserve supported action consequences.** W1 must fail when proper
   forecast, calibration, action-sensitivity, or support gates fail. Visual plausibility cannot
   rescue it.
2. **Forecast gains do not improve complete closed-loop decisions under the M4 budget.** W2 must
   randomize complete policies and retain deadlines, abstentions, fallbacks, and failed executions.
3. **The linked protocol does not isolate a useful residual beyond prior work.** W3 must narrow to
   matched cross-substrate localization and downstream decision effects. Infrastructure volume,
   PID output, or a photorealistic renderer cannot create novelty.

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
- for each PID result, the method-stage ledger, complete object graph, mathematics and
  applicability decisions, and the paired canonical Markdown plus deterministic PDF receipt;
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
- For PID, report the exact functional or hierarchy, output coordinate, route, law, transform,
  source count, component, aggregation, and achieved `PID-P*` level.
- Publish the method-selection process, rejected routes, theorem locators, negative controls, and
  unresolved objections. Do not publish only the selected numeric result.
- Treat the Markdown as authority and the PDF as a derived view. Record source, renderer, and PDF
  hashes plus extracted-text and page-by-page visual checks.
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
- a PID publication packet conflates two scientific objects, omits a rejected route, or carries a
  stale Markdown-to-PDF build receipt;
- an analysis uses a nonlocked endpoint as “primary” or changes the target population silently;
- the world-model claim ledger changes W1/W2/W3 roles, status, blockers, artifacts, or permitted
  language without an exact reviewed registry update;
- a world-model result is called confirmatory without a separate reviewed freeze contract,
  registered holdout, and authenticated access history;
- the preserved diagnostic-governance, holdout, transport/contamination, literature, and claim registries disagree
  about freeze or access state, fail schema/content-hash validation, or break the recorded access
  hash chain;
- holdout exposure is recorded before a valid freeze and authorization, or an unfinished scaffold is
  promoted to freeze-ready;
- systematic, scoping, complete, or reproducible-review language appears without saved database
  sources, exact queries and dates, a candidate universe, criteria, and per-candidate decisions;
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
| Claim-matched intervention responses ground use claims | availability does not identify response; paired software and randomized closed-loop interventions identify different targets | only if a stronger identification design is justified |
| W1/W2 are the only proposed primary scientific claims | supported forecast fidelity and complete-policy value define the world-model thesis | pre-holdout amendment after M2/M3 shows no viable model, outcome, or resource regime |
| H1/H2 are preserved secondary diagnostics | they can localize response and prospective failure after W1/W2 define a useful target | activate only under a separately frozen diagnostic study |
| H4 is a preserved alternative/companion inside that diagnostic family | availability–tested-response gaps remain useful but do not define the primary thesis | activate only under the preserved family’s selection rule |
| Full three-source PID is exploratory | combinatorics and foundational limitations | new measure/estimator with relevant validation |
| No safety-certification language | diagnostics are one evidence source | formal assurance program with domain standards |
| Rerun/standard formats first | existing tools solve viewing/storage well | benchmark demonstrates a missing capability requiring custom work |
| Tauri/SparkJS remain optional; 3DGS is conditional | the custom shell is not needed; a 3DGS path is required only for W3/Paper B and never for W1/W2 | promote a renderer only after state/camera/physics identity checks pass |
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
- **[R10]** Lyu, A.; Clark, A.; Raviv, N. (2026). *Structural Impossibility of Antichain-Lattice Partial Information Decomposition*. arXiv:2604.03869v2. https://arxiv.org/abs/2604.03869
- **[R11]** Gutknecht, A. J.; Rosas, F. E.; Ehrlich, D. A.; Makkeh, A.; Mediano, P. A. M.; Wibral, M. (2025). *Shannon Invariants: A Scalable Approach to Information Decomposition*. arXiv:2504.15779. https://arxiv.org/abs/2504.15779
- **[R12]** Kraskov, A.; Stögbauer, H.; Grassberger, P. (2004). *Estimating Mutual Information*. **Physical Review E** 69:066138. https://doi.org/10.1103/PhysRevE.69.066138
- **[R13]** Gao, S.; Ver Steeg, G.; Galstyan, A. (2015). *Efficient Estimation of Mutual Information for Strongly Dependent Variables*. AISTATS. arXiv:1411.2003. https://arxiv.org/abs/1411.2003
- **[R14]** Amjad, R. A.; Geiger, B. C. (2019). *Learning Representations for Neural Network-Based Classification Using the Information Bottleneck Principle*. **IEEE TPAMI**. arXiv:1802.09766. https://arxiv.org/abs/1802.09766
- **[R15]** Goldfeld, Z.; van den Berg, E.; Greenewald, K.; Melnyk, I.; Nguyen, N.; Kingsbury, B.; Polyanskiy, Y. (2019). *Estimating Information Flow in Deep Neural Networks*. ICML. arXiv:1810.05728. https://arxiv.org/abs/1810.05728
- **[R16]** Song, J.; Ermon, S. (2020). *Understanding the Limitations of Variational Mutual Information Estimators*. AISTATS. arXiv:1910.06222. https://arxiv.org/abs/1910.06222
- **[R17]** Belghazi, M. I. et al. (2018). *Mutual Information Neural Estimation*. ICML. arXiv:1801.04062. https://arxiv.org/abs/1801.04062
- **[R18]** Xiu, Z.; Luo, Y.; Nakayama, H. (2026). *A Comprehensive Information-Decomposition Analysis of Large Vision-Language Models*. ICLR 2026. arXiv:2603.29676. https://arxiv.org/abs/2603.29676 ; official venue record: https://iclr.cc/virtual/2026/poster/10011370
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
- **[R48]** Chen, K. et al. (2025). *Robo-DM: Data Management For Large Robot Datasets*. arXiv:2505.15558. https://arxiv.org/abs/2505.15558
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
- **[R73]** `sepahead/pid-rs`. *Shared-exclusions partial information decomposition and mutual-information estimators in Rust*. Reviewed Prisoma pin `8a5a9dda601556443f956a2fba164cccc913ed2e`, contract-hardening revision `70b45f7b75fac06777ea215a73df01209490311a`, immutable `v0.9.0` review-tag commit `a9a275157237999c8da6ab813130d74f6113dec9`, current Prisoma pin `796c11e70f009634b853dc4ada6f565563d82f51`, estimator-code anchor `cb3f58f0b190454cb3f1090de8798261ec78f194`, and unadopted public main `bc3aa80fb6025e709c2906a08bce25a4fac40578`, rechecked 14 August 2026. The current pin includes the public-`csxpid` fixture, agreement within `1e-12` nats after recorded conversion, fail-closed population-support contracts, default-off research features, and bounded report-first APIs. Public main adds method catalogs, software identity, outcome/run-log contracts, formal/categorical assurance work, support-change and concentration records, Lean 4.33 formal replay hardening, source-errata and evidence-boundary registries, and exact-certifier surfaces. Its ecosystem file states that Prisoma integration is not claimed. An isolated all-feature Prisoma check, test-target build, and 531-test run passed at `722d3abe`. Prisoma inspected the one-commit `bbdfda40..cb3f58f0` estimator delta and replayed four predecessor-radius fixtures plus one structured overflow fixture. Head `7473e62` changes custody and assurance surfaces only, and its full CI run `31724449805` failed two jobs. Current head `bc3aa80` is one custody-repair child later and also changes no crate or Cargo input. Full CI run `31773937366` passed all 45 jobs, and CodeQL run `31773937102` passed all four jobs. Upstream still marks broader revision-4 KSG repository integration NO-GO. The 0.9 review source makes no 1.x compatibility, registry, or published-wheel promise. Neither revision establishes high-dimensional VLA application validity or independent corroboration. https://github.com/sepahead/pid-rs/tree/796c11e70f009634b853dc4ada6f565563d82f51 ; https://github.com/sepahead/pid-rs/tree/bc3aa80fb6025e709c2906a08bce25a4fac40578
- **[R74]** `sepahead/NCP`. *Neuro-Cybernetic Protocol*. The latest immutable release is `v0.8.0` at peeled commit `2f5bd586d4bb20c90362bb6f5698b7f64057ba4e`, wire 0.8. The provider boundary was verified through the official ref on 13 August 2026. Prisoma deliberately retains the immutable consumer pin. Verified upstream main `1a04294c90c1b50eba06ae1c6afe9c951319250d` is the unreleased, release-blocked `1.0.0-rc.1` candidate. It uses wire 1.0 and compact proto contract hash `163acc57d8a62b66`. It is incompatible with the pinned observer. NCP ledger tasks `P01`, `P02`, and `P03` are OPEN, not dependency-ready, and **NOT RUN**. B01 remains `IN_PROGRESS`; its refined low-overhead architecture and prepared-stream-monitor gap record are coordination-only and have no passing receipt. Prisoma remains a read-only observer with documented transport and security limits. https://github.com/sepahead/NCP/tree/v0.8.0 ; https://github.com/sepahead/NCP/tree/1a04294c90c1b50eba06ae1c6afe9c951319250d ; https://github.com/sepahead/NCP/blob/1a04294c90c1b50eba06ae1c6afe9c951319250d/evidence/implementation/task-ledger.v1.json
- **[R75]** `sepahead/galadriel`. *Fail-closed cross-sensor statistical-consistency monitoring in safe Rust*, public revision `80506dd2ce52b33c3334c7d1760a8155c7631241`, inspected 24 July 2026. The tree freezes 0.9.0 candidate inputs and includes a strict two-route consumer, lifecycle adapter, and bounded operational receiver. No reciprocal Prisoma pin, direct adapter, producer-consumer golden fixture, or deployed receiver-verified Crebain qualification exists. https://github.com/sepahead/galadriel/tree/80506dd2ce52b33c3334c7d1760a8155c7631241
- **[R76]** `sepahead/crebain`. *Multi-UAV simulation and airspace-awareness research testbed*, public revision `7f6b3bdf4d20aba1b351b3ceacb259bd123c93a6`, inspected 26 July 2026. The restricted read-only Engram view adds continued host challenges and a native-IPC accessibility probe. Embedded mode disables native, external telemetry, artifact exchange, NCP, and plant paths. The host must relock a stale frame. Host messages correlate a document but do not attest a process or build. NCP action/control commands remain unregistered. No live NCP control loop or direct Prisoma adapter exists. https://github.com/sepahead/crebain/tree/7f6b3bdf4d20aba1b351b3ceacb259bd123c93a6
- **[R77]** `sepahead/manwe`. *Airspace perception research workbench*, public revision `6d73405bbf5365039ee1d0db9c466ed6346a9c57`, inspected 24 July 2026. The repository adds numeric, I/O, and security hardening but still has no drop-in Prisoma adapter. It documents schema, tensor, clock, frame, and statistical-assumption gaps. https://github.com/sepahead/manwe/tree/6d73405bbf5365039ee1d0db9c466ed6346a9c57
- **[R78]** `sepahead/engram` and `sepahead/Paper2Brain`, inspected 13 August 2026. The named Engram repository remains a README-only placeholder at `a4ce6ab9897dd3f1265b4cacc53f0afc349087cd`. The executable Engram Neural Labs host lives in Paper2Brain at `2648caf18d24075c4a36af81a6bb032bb551244e`. Host API 1.1 retains the unchanged byte-locked Prisoma descriptor in its native catalog and uses one generic bounded JSON-RPC TCP adapter. Four observed commits changed adjacent-page caption binding, visual-parent disambiguation, and visual-grounding design and value gates. They did not change the Prisoma surfaces. Its Prisoma descriptor remains byte-identical to the local file at SHA-256 `006a6cc5fe46041fcc180d1890a36f821e8901768161952b143bbfc3c3fd70f9`. Prisoma implements the exact read-only profile with describe, session, and status only. This adds local live-presentation evidence to the E2 consumer relationship. It is not a producer-consumer scientific fixture, process attestation, Prisoma validation, NCP translation, artifact ingestion, authority, or E4 evidence. The descriptor declares target Engram wire 1.0 incompatible with Prisoma wire 0.8. NCP's provider inventory records a preserved in-progress Paper2Brain migration that targets candidate wire 1.0. It is not an installed or qualified integration. https://github.com/sepahead/engram/tree/a4ce6ab9897dd3f1265b4cacc53f0afc349087cd ; https://github.com/sepahead/Paper2Brain/tree/2648caf18d24075c4a36af81a6bb032bb551244e
- **[R79]** `sepahead/melkor`. *Gaussian splatting and reconstruction toolkit*, public revision `529260f568c62250b0541a11f5c24b45767bf1cf`, inspected 24 July 2026. The public v2 development/release-candidate line includes a canonical scene model, KHR_gaussian_splatting GLB I/O, inspection/conversion paths, and resource hardening. It has no direct Prisoma adapter or calibrated reconstruction-to-diagnostic uncertainty result. https://github.com/sepahead/melkor/tree/529260f568c62250b0541a11f5c24b45767bf1cf
- **[R80]** Prisoma repository. *WORLD_WARP_INTEGRATION.md*, snapshot `64bd881248463e7142d022bb95a5850bcf8fced2`; optional external world-model integration specification, not verified as implemented. https://github.com/sepahead/prisoma/blob/64bd881248463e7142d022bb95a5850bcf8fced2/WORLD_WARP_INTEGRATION.md
- **[R81]** Prisoma repository. *GAUSS_MI_INTEGRATION.md*, snapshot `64bd881248463e7142d022bb95a5850bcf8fced2`; status “Specification (Pre-Implementation)” and weighted KSG described as a heuristic requiring its own validation gate. https://github.com/sepahead/prisoma/blob/64bd881248463e7142d022bb95a5850bcf8fced2/GAUSS_MI_INTEGRATION.md
- **[R82]** `sepahead/cobot-atlas`. *3D mesh-generation pipeline and dataset*. Accessed 12 July 2026. Repository reports 2,024 GLB files in the hosted dataset. https://github.com/sepahead/cobot-atlas
- **[R83]** `sepahead/relief-atlas`. *10K+ 3D mesh assets for disaster relief and civil protection*. Accessed 12 July 2026. Repository reports 10,079 items and directs users to individual asset metadata for licensing. https://github.com/sepahead/relief-atlas
- **[R84]** `sepahead/cortexel`. Scientific-visualization contract, public revision `d29669e6d5b1766fd96e1eacefb02b3f43c5ce61`, inspected 24 July 2026. The public 0.9.0 prerelease provides deterministic accessible SVG export across 19 stable visualization families. It has no published package or DOI, external oracle or real-adapter evidence, or direct Prisoma contract. It does not supersede the Rerun-first decision. https://github.com/sepahead/cortexel/tree/d29669e6d5b1766fd96e1eacefb02b3f43c5ce61
- **[R85]** `sepahead` GitHub profile and six public repository-index pages. Accessed 12 July 2026; metadata for 174 public repositories were screened. The profile project graph is treated as architectural intention rather than executable integration evidence; anonymous GitHub code search required sign-in, so negative findings are bounded to public metadata and inspected repository surfaces. https://github.com/sepahead?tab=repositories
- **[R86]** `sepahead/haldir`, public revision `555108666cb82e8a36dcd4b08b5b30c62367a6f4`, inspected 24 July 2026. The tree contains substantial internal P0, durable-publication, audit, and release work. It also contains an opt-in exact NCP-0.8 adapter, synthetic mTLS/ACL evidence, strict-publisher bindings, and a Called-lifecycle fault matrix. The live-profile declaration remains cooperative, process-local, bypassable through lower-level construction, and not durably bound. Positive cases use test-only seams, and no live Zenoh session executes the concrete method. There is no runnable service, credential-custody proof, Crebain application, direct Prisoma route, or E4 report. Because Haldir originates commands, it remains an offline comparator under the Agent Bridge-only decision. https://github.com/sepahead/haldir/tree/555108666cb82e8a36dcd4b08b5b30c62367a6f4
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
- **[R113]** Fang, W.; Zhang, T.; Tao, W.; Chan, A. (2026). *Towards Understanding Modality Interaction in Multimodal Language Models via Partial Information Decomposition*. arXiv:2606.00959v2, 8 August 2026. https://arxiv.org/abs/2606.00959
- **[R114]** Panda, K.; Maia, W.; Agarwal, V.; Greer, R. (2026). *What Do They See? Interpreting Complex Road Scenarios Through the Eyes of Vision-Language-Action Models for Safe and Trustworthy Autonomous Vehicle Learning*. arXiv:2607.16938, 18 July 2026. https://arxiv.org/abs/2607.16938
- **[R115]** Zhang, Y.; Wu, Y.; Duan, H.; Han, J. (2026). *CofactVLA: Deconfounding Vision-Language-Action Models via Counterfactual Intervention*. arXiv:2608.04396, 5 August 2026. https://arxiv.org/abs/2608.04396
- **[R116]** Häon, B.; Stocking, K. C.; Chuang, I.; Tomlin, C. (2025). *Mechanistic Interpretability for Steering Vision-Language-Action Models*. Proceedings of the 9th Conference on Robot Learning, PMLR 305:2743–2762. https://proceedings.mlr.press/v305/haon25a.html
- **[R117]** Tai, J. (2026). *Same Weights, Different Robot: A Deployment Safety View of VLA Policies*. arXiv:2606.03724, 2 June 2026. https://arxiv.org/abs/2606.03724
- **[R118]** Tang, Y.; Wang, T.; Chen, Y.; Zhang, B.; Guan, Q.; Tang, R. (2026). *Shifting Uncertainty to Critical Moments: Towards Reliable Uncertainty Quantification for VLA Model*. arXiv:2603.18342, 18 March 2026. https://arxiv.org/abs/2603.18342
- **[R119]** Yoon, S.; Yoo, M.; Ahn, S.; Choi, S.; Woo, H. (2026). *RoboBRIDGE: A Modular Framework for Bridging Policies to Robust Real-World Robotic Agents*. arXiv:2607.27881, 30 July 2026. https://arxiv.org/abs/2607.27881
- **[R120]** Rindt, D.; Hu, R.; Steinsaltz, D.; Sejdinovic, D. (2022). *Survival Regression with Proper Scoring Rules and Monotonic Neural Networks*. AISTATS 2022, PMLR 151:1190–1205. https://proceedings.mlr.press/v151/rindt22a.html
- **[R121]** Kvamme, H.; Borgan, Ø. (2023). *The Brier Score under Administrative Censoring: Problems and a Solution*. **JMLR** 24(2):1–26. https://www.jmlr.org/papers/v24/19-1030.html
- **[R122]** Jonkers, J.; Van Wallendael, G.; Duchateau, L.; Van Hoecke, S. (2026). *Proper Scoring Rules for Right-Censored Survival Data*. arXiv:2606.06393, 4 June 2026. https://arxiv.org/abs/2606.06393
- **[R123]** Alberge, J.; Maladiere, V.; Grisel, O.; Abécassis, J.; Varoquaux, G. (2025). *Survival Models: Proper Scoring Rule and Stochastic Optimization with Competing Risks*. AISTATS 2025, PMLR 258:3619–3627. https://proceedings.mlr.press/v258/alberge25a.html
- **[R124]** Adebayo, J.; Gilmer, J.; Muelly, M.; Goodfellow, I.; Hardt, M.; Kim, B. (2018). *Sanity Checks for Saliency Maps*. NeurIPS 2018. https://proceedings.neurips.cc/paper/2018/hash/294a8ed24b1ad22ec2e7efea049b8737-Abstract.html
- **[R125]** Geiger, A.; Lu, H.; Icard, T.; Potts, C. (2021). *Causal Abstractions of Neural Networks*. arXiv:2106.02997. https://arxiv.org/abs/2106.02997
- **[R126]** Lyu, A.; Clark, A.; Raviv, N. (2026). *Closed-Form Gaussian Estimators for Multi-Source Partial Information Decomposition*. arXiv:2605.09919, 11 May 2026. The paper's conditional-independence hierarchy defines two-source redundancy and multi-source unique/synergy quantities; for \(N\geq3\) it deliberately assigns no redundancy and is not a complete antichain decomposition. https://arxiv.org/abs/2605.09919
- **[R127]** Li, R.; Zhang, H.; Jin, J.; Zeng, Q.; Zhuang, Z.; Tang, Y.; Lyu, S.; Wang, D. (2026). *World-Value-Action Model: Implicit Planning for Vision-Language-Action Systems*. arXiv:2604.14732v2, 19 April 2026. https://arxiv.org/abs/2604.14732
- **[R128]** Yan, G.; Liu, J.; Fan, Y.; Cai, L.; Liao, M.; Zhang, J.; Fox, D. (2026). *Flex-\(\pi\): A Multi-Stream World-Action Model with Compute Flexibility*. arXiv:2608.10860v1, 11 August 2026. https://arxiv.org/abs/2608.10860 ; https://flex-pi.github.io/
- **[R129]** Wang, J. et al. (2026). *SLIM-0.5B: Learning Action-Grounded Predictive Latents for Robot Manipulation*. arXiv:2608.09771v1, 10 August 2026. https://arxiv.org/abs/2608.09771 ; https://github.com/kzz1031/SLIM
- **[R130]** Tang, Q.; Zhuang, B.; Yuan, B.; Yu, X.; Guo, L.; Feng, J. (2026). *World Tokens: Enhancing Embodied Policies with Training-Time World Modeling*. arXiv:2608.09730v1, 10 August 2026. https://arxiv.org/abs/2608.09730
- **[R131]** Sun, J. et al. (2026). *VLA-JEPA: Enhancing Vision-Language-Action Model with Latent World Model*. arXiv:2602.10098v2, updated 14 February 2026. Reviewed LeRobot source `huggingface/lerobot@a16f34c085c9597fcbdb9fde395a3334d78df716`; reviewed model `lerobot/VLA-JEPA-LIBERO@735d9f692981e286ade093b5046627eda876e5d0`. https://arxiv.org/abs/2602.10098 ; https://github.com/huggingface/lerobot/tree/a16f34c085c9597fcbdb9fde395a3334d78df716 ; https://huggingface.co/lerobot/VLA-JEPA-LIBERO/tree/735d9f692981e286ade093b5046627eda876e5d0
- **[R132]** Yuan, T.; Dong, Z.; Liu, Y.; Zhao, H. (2026). *Fast-WAM: Do World Action Models Need Test-time Future Imagination?* arXiv:2603.16666v2. https://arxiv.org/abs/2603.16666
- **[R133]** Lin, Y. et al. (2026). *JEPA-WAM: Learning Vision-Language-Action Policies with Joint-Embedding World Modeling*. arXiv:2608.09381v1, 10 August 2026. Reviewed source `SpriteWithoutIce/JEPA_WAM@537830bee0d84d10266a14cad7f038b653b717d8`; model repository `CokeAnd1ce/JEPA_WAM@ca10ccbc191d8f56b4346487913e043b2722b6d2`. The main LIBERO PyTorch file has SHA-256 `e63285fb347048989f14a8a24962a2b921d787f7ada0176a0eacd6b256d57d23`. https://arxiv.org/abs/2608.09381 ; https://github.com/SpriteWithoutIce/JEPA_WAM ; https://huggingface.co/CokeAnd1ce/JEPA_WAM/tree/ca10ccbc191d8f56b4346487913e043b2722b6d2
- **[R134]** Pan, B.; Liu, F.; Lu, H.; Wang, J.; Shi, Y. (2026). *SelfWAM: A Self-Grounded Unified World Action Model for Fast Robot Control*. arXiv:2608.00725v1, 1 August 2026. https://arxiv.org/abs/2608.00725
- **[R135]** Zhu, C.; Yu, R.; Feng, S.; Burchfiel, B.; Shah, P.; Gupta, A. (2025). *Unified World Models: Coupling Video and Action Diffusion for Pretraining on Large Robotic Datasets*. arXiv:2504.02792v3. https://arxiv.org/abs/2504.02792
- **[R136]** Peng, Q.; Liang, Y.; Yan, R.; Hansen, N.; Wang, X. (2026). *FACT: Failure-Aware Causal Training for World-Action Models*. arXiv:2608.10232v1, 10 August 2026. The title's causal label does not establish identification. https://arxiv.org/abs/2608.10232 ; https://fact-wam.github.io/
- **[R137]** Zhou, P. et al. (2026). *\(\tau_0\)-WM: A Unified Video-Action World Model for Robotic Manipulation*. arXiv:2606.01027v1. https://arxiv.org/abs/2606.01027
- **[R138]** Yang, T. et al. (2026). *MiraBench: Evaluating Action-Conditioned Reliability in Robotic World Models*. arXiv:2605.29360v1. https://arxiv.org/abs/2605.29360
- **[R139]** Hansen, N.; Wang, X. (2026). *Hallucination in World Models Is Predictable and Preventable*. arXiv:2606.27326v1. https://arxiv.org/abs/2606.27326
- **[R140]** Shukor, M. et al. (2025). *SmolVLA: A Vision-Language-Action Model for Affordable and Efficient Robotics*. arXiv:2506.01844v1. https://arxiv.org/abs/2506.01844 ; https://github.com/huggingface/lerobot
- **[R141]** Dyna Robotics (2026). *Dyna-2*. Company technical report and model page, reviewed 13 August 2026. It reports finite internal results without public code, checkpoints, raw trials, or a complete independent protocol. https://www.dyna.co/dyna-2
- **[R142]** Liu, X. et al. (2026). *JEPA-WAM: Stage-Level Joint-Embedding Prediction for World-Action Models in Robot Manipulation*. arXiv:2608.10780v1, 11 August 2026. The deployed predictor estimates an intended next-stage latent from observed history and language. It does not predict consequences under candidate actions. https://arxiv.org/abs/2608.10780
- **[R143]** Chen, Y. et al. (2026). *XEWorld: Can Action-Conditioned World Models Generalize to Unseen Robot Embodiments?* arXiv:2608.05799v1, 6 August 2026. https://arxiv.org/abs/2608.05799
- **[R144]** Zeng, X.; Ren, H.; Song, Z. (2026). *PhyLatent: Learning Dynamics-Relevant Representations for JEPA World Models*. arXiv:2608.05720v1, 6 August 2026. https://arxiv.org/abs/2608.05720
- **[R145]** Yan, H. et al. (2026). *Is Forward Prediction Enough? Physical State Grounding for JEPA World Models*. arXiv:2608.06799v1, 7 August 2026. https://arxiv.org/abs/2608.06799
- **[R146]** Gu, Z. et al. (2026). *HarnessWAM: Bridging Prediction and Deliberation in World Action Models*. arXiv:2608.09516v1, 10 August 2026. https://arxiv.org/abs/2608.09516
- **[R147]** Ye, F. et al. (2026). *Rethink Before You Execute: Adaptive Execution for World Action Models*. arXiv:2608.09492v1, 10 August 2026. https://arxiv.org/abs/2608.09492
- **[R148]** Motubrain Team (2026). *World Action Models in Real Time: An Empirical Study of Smooth Execution via Asynchronous Deployment*. arXiv:2608.01880v1, 3 August 2026. The online comparison uses three tasks and five trials per method–task cell. https://arxiv.org/abs/2608.01880
- **[R149]** Yang, F. et al. (2026). *LiLa-WAM: Lightweight Latent Reasoning World-Action Model for Robotic Manipulation*. arXiv:2608.03701v1, 4 August 2026. Reviewed code `b6a2095d76927119bcfc0d2ca04eb5cea98d10d8`; ModelScope checkpoint revision `93ab191b2500aa37322244c4ae0e84eed1e848ee`. The paper reports a 0.5B model and single-24-GB-GPU training. The released inference loop ignores returned shared tokens and does not invoke the future decoder. The code has no repository license, uses no language input, and is not MPS-qualified. https://arxiv.org/abs/2608.03701 ; https://github.com/teee000/LiLa-WAM
- **[R150]** Bao, W.; Jiang, T.; Chen, Z.; Lim, S.-N.; Peng, P. D.; Shang, Y. (2026). *Surgical WAM: A World-Action Model for Data-Efficient Surgical Robot Learning*. arXiv:2608.11204v1, 11 August 2026. The paper reports joint video-action sampling and a matched video-pretraining ablation on four simulated surgical tasks. No official runnable code or checkpoint was verified at the review cutoff. https://arxiv.org/abs/2608.11204
- **[R151]** Liu, S.; Wen, Q.; Hao, S.; Luo, Q.; Zhang, C.; You, F.; Wu, C.; Su, N. (2026). *CoWAM: Coordination Contracts for Selective Policy Intervention with WAMs*. arXiv:2608.02578v1, 3 August 2026. Its selector uses a shared candidate pool and commits decisions before shared oracle labels. This design supports selective-decision evaluation. It does not identify a causal transition. https://arxiv.org/abs/2608.02578
- **[R152]** Lou, Y. et al. (2026). *DynamicWAM: Dual-Path Motion Conditioning for World-Action Models in Dynamic Manipulation*. arXiv:2608.00793v2, 6 August 2026. https://arxiv.org/abs/2608.00793
- **[R153]** Wang, R. et al. (2026). *FlowPilot: Real-Time World-Action Modeling for Agile UAV Navigation*. arXiv:2608.00635v1, 1 August 2026. The paper reports an end-to-end result on Jetson Orin NX for depth-only UAV navigation. This is author-reported task-specific efficiency evidence, not an M4 manipulation result. https://arxiv.org/abs/2608.00635
- **[R154]** Zhao, R. et al. (2026). *SG-WAM: Self-Guided World Modeling in Geometry-Aware Policy Space*. arXiv:2608.01397v1, 2 August 2026. Its action-conditioned latent predictor and geometry teacher are training branches. The deployed policy is direct. https://arxiv.org/abs/2608.01397
- **[R155]** Qiu, C. et al. (2026). *Vid2WAM: Distilling Video Diffusion Priors into World Action Models*. arXiv:2608.08558v1, 9 August 2026. The video teacher and inverse-dynamics model are removed at deployment. https://arxiv.org/abs/2608.08558
- **[R156]** Yan, H. et al. (2026). *Robust-WAM: Bridging Generative Pretraining and Semantic Foresight in World-Action Models*. arXiv:2608.05903v2, 7 August 2026. Its semantic-foresight objective aligns the action stream during training. https://arxiv.org/abs/2608.05903
- **[R157]** Fan, Z. et al. (2026). *MobileWAM: Bridging World Action Models to Mobile Manipulation with Chain-of-Foresight*. arXiv:2608.04657v2, 6 August 2026. Its foresight chain and video generation are removed at deployment. https://arxiv.org/abs/2608.04657
- **[R158]** Yuan, S. et al. (2026). *DreamWAM: Beyond RGB Future Prediction for World Action Models*. arXiv:2608.04996v1, 5 August 2026. It supports no-rollout and joint video-action inference modes. The released repository has no license file or declared GitHub license at the review cutoff. https://arxiv.org/abs/2608.04996 ; https://github.com/hustvl/DreamWAM
- **[R159]** Pan, Y. et al. (2026). *World-to-Wrist: Task-Conditioned Future Wrist Modeling for Fine-Grained Robot Manipulation*. arXiv:2608.05369v1, 5 August 2026. Its deployed action head consumes a future wrist latent predicted from current task context and wrist history. Reviewed source: `yyyyu120/W2-VLA@0a32385caf0abcb41dd42b46f24bdb6b6050f992`. https://arxiv.org/abs/2608.05369 ; https://github.com/yyyyu120/W2-VLA
- **[R160]** Li, J. et al. (2026). *Efficient-WAM: A 1B-Parameter World-Action Model with Low-Cost Future Imagination*. arXiv:2606.10040, updated 10 June 2026. Reviewed source: `jiajun613/Efficient-WAM@2bd75a8c56acfcd5754b98c7ed313176911ccae0`, Apache-2.0. Reviewed model repository: `jiajun0613/Efficient-WAM_RoboTwin@81280a79e8ac69dd6ffb9ce8698e00d122ec07fd`, with no model card or declared weight license. The released runtime uses bidirectional video-action attention, asserts CUDA before its nominal attention fallback, and loads a pickle-based checkpoint. https://arxiv.org/abs/2606.10040 ; https://github.com/jiajun613/Efficient-WAM ; https://huggingface.co/jiajun0613/Efficient-WAM_RoboTwin/tree/81280a79e8ac69dd6ffb9ce8698e00d122ec07fd
- **[R161]** Lyu, J. et al. (2026). *LDA-1B: Scaling Latent Dynamics Action Model via Universal Embodied Data Ingestion*. arXiv:2602.12215, updated 3 June 2026. Policy mode substitutes a visual register for the unobserved future. A separate forward task accepts current state and action to predict future DINO latents. Reviewed source: `jiangranlv/LDA-1B@06e6a274a9086cc26635a9fe663866335eb30fc5`. https://arxiv.org/abs/2602.12215 ; https://github.com/jiangranlv/LDA-1B
- **[R162]** Yang, Y. et al. (2026). *World-Language-Action Model for Unified World Modeling, Language Reasoning, and Action Synthesis*. arXiv:2606.05979v1, 4 June 2026. Default inference disables the world expert. Optional test-time scaling samples, predicts, scores, and selects among action candidates. Reviewed source: `SJTU-DENG-Lab/WLA@155ac94eaca8b3d1ae0789ae298fc55e37936081`. https://arxiv.org/abs/2606.05979 ; https://github.com/SJTU-DENG-Lab/WLA
- **[R163]** Wang, J. et al. (2026). *RepWAM: World Action Modeling with Representation Visual-Action Tokenizers*. arXiv:2606.13674, updated 13 June 2026. Reviewed repository `wdrink/RepWAM@ad32f52182662ade57699aacc9d146e1aef55975` still described inference code and weights as under inspection at the cutoff. https://arxiv.org/abs/2606.13674 ; https://github.com/wdrink/RepWAM
- **[R164]** Kairos Team et al. (2026). *Kairos: A Regret-Aware Native World-Action Model Stack for Physical AI*. arXiv:2606.16533v3, 3 July 2026. Reviewed source: `kairos-agi/kairos@661f93337e85e9a30470b109ca645744a1947a65`. Source and weights are released for a CUDA-oriented 4B stack. The report describes current evaluation as proxy evidence and leaves direct real-robot closed-loop regret validation to future work. https://arxiv.org/abs/2606.16533 ; https://github.com/kairos-agi/kairos
- **[R165]** Huang, J.; Wu, Z.; Zhang, Z.; Wang, Z.; You, S.; Huang, T. (2026). *Foresight Without Seeing: Latent Futures for World Action Models*. arXiv:2608.11605v1, 12 August 2026. ForeWAM performs one prefill over current context and stochastic future slots, then conditions action denoising on hidden K/V and dynamics-register state. The interface is action-independent. No official runnable code or checkpoint was verified at the cutoff. https://arxiv.org/abs/2608.11605
- **[R166]** Zhang, C.; Tong, J.; Li, X.; Wang, Y.; Li, H. (2026). *Keep the Future, Drop the Rollout: RIFT for World Action Models*. arXiv:2608.11521v1, 12 August 2026. Rift writes an action-independent future-position K/V cache in one prefill. The paper also reports paired closed-loop cache interventions. These tests support bounded use of the tested cache path, not physical correctness or causal-transition identification. No official runnable code or checkpoint was verified at the cutoff. https://arxiv.org/abs/2608.11521
- **[R167]** Zhang, X.; Du, Y. (2026). *World Action Planner: Generalizable Decision-Making with Action-Conditioned World Models*. arXiv:2607.27599v1, 30 July 2026. Its algorithm proposes actions, predicts a grid of candidate outcomes, ranks them, and executes the selected candidate. This satisfies the operational class-E planning definition. Its reported results are simulation evidence, and no official runnable artifact was verified in this review. https://arxiv.org/abs/2607.27599 ; https://worldactionplanner.github.io/
- **[R168]** Liu, Y. et al. (2026). *CheckVLA: Execution-Time Verification with Action-Conditioned World Model for Long-Horizon Mobile Manipulation*. arXiv:2607.26789v1, 29 July 2026. It predicts expected execution under committed actions, compares predictions with later observations, and triggers a latency-aware suffix rewrite. Its action-shuffle and observation-only controls make it a strong monitor comparator. This does not establish causal-transition validity or external transport. https://arxiv.org/abs/2607.26789
- **[R169]** Wibral, M.; Priesemann, V.; Kay, J. W.; Lizier, J. T.; Phillips, W. A. (2017). *Partial Information Decomposition as a Unified Approach to the Specification of Neural Goal Functions*. **Brain and Cognition** 112:25–38. The paper was first published online in 2015. It develops a coordinate language for neural goal functions. It does not define one universal PID functional or the later MGW shared-exclusions functional. https://doi.org/10.1016/j.bandc.2015.09.004
- **[R170]** Gutknecht, A. J.; Wibral, M.; Makkeh, A. (2021). *Bits and Pieces: Understanding Information Decomposition from Part-Whole Relationships and Formal Logic*. **Proceedings of the Royal Society A** 477:20210110. https://doi.org/10.1098/rspa.2021.0110
- **[R171]** Gutknecht, A. J.; Makkeh, A.; Wibral, M. (2025). *From Babel to Boole: The Logical Organization of Information Decompositions*. **Proceedings of the Royal Society A** 481:20240174. https://doi.org/10.1098/rspa.2024.0174
- **[R172]** Schneider, A. C.; Neuhaus, V.; Ehrlich, D. A.; Makkeh, A.; Ecker, A. S.; Priesemann, V.; Wibral, M. (2025). *What Should a Neuron Aim For? Designing Local Objective Functions Based on Information Theory*. ICLR 2025. The work composes PID quantities into three-input local learning objectives. It is not a PID functional or finite-sample certifier. https://proceedings.iclr.cc/paper_files/paper/2025/hash/87d8ed41d250c401a68f05100e0a4ef0-Abstract-Conference.html
- **[R173]** Terver, B.; Yang, T.-Y.; Ponce, J.; Bardes, A.; LeCun, Y. (2025/2026). *What Drives Success in Physical Planning with Joint-Embedding Predictive World Models?* arXiv:2512.24497. Reviewed code `facebookresearch/jepa-wms@13cf1d9c7e476f53c17714d2e0f1dc239a883ce0`; reviewed model revision `facebook/jepa-wms@9b9c41ef249466630dbf1a20e78391865d07b3b9`. The released evaluator performs predictive planning but is CUDA-oriented and licensed CC BY-NC 4.0. No upstream MPS end-to-end result was verified. https://arxiv.org/abs/2512.24497 ; https://github.com/facebookresearch/jepa-wms/tree/13cf1d9c7e476f53c17714d2e0f1dc239a883ce0
- **[R174]** Co, P. et al. (2026). *WorldSimProbe: Diagnosing Simulator Faithfulness in Action-Conditioned World Models for Embodied Manipulation*. arXiv:2608.09298v1, 10 August 2026. It separates action calibration, grounding, and interaction dynamics under controlled action-conditioned tests. https://arxiv.org/abs/2608.09298
- **[R175]** Jiang, G. et al. (2025). *GSWorld: Closed-Loop Photo-Realistic Simulation Suite for Robotic Manipulation*. arXiv:2510.20813. It combines 3D Gaussian splats, mesh/URDF assets, physics, and closed-loop policy evaluation. https://arxiv.org/abs/2510.20813
- **[R176]** Zhang, K. et al. (2025). *Real-to-Sim Robot Policy Evaluation with Gaussian Splatting Simulation of Soft-Body Interactions*. arXiv:2511.04665. It evaluates policy behavior in Gaussian-rendered, physics-based real-to-sim environments. https://arxiv.org/abs/2511.04665
- **[R177]** *RoboWM-Bench: A Benchmark for Evaluating World Models in Robotic Manipulation* (2026). arXiv:2604.19092. It grounds video-world-model evaluation through executable embodied outcomes and checks real-to-sim outcome consistency. https://arxiv.org/abs/2604.19092
- **[R178]** Qureshi, M. N. et al. (2024). *SplatSim: Zero-Shot Sim2Real Transfer of RGB Manipulation Policies Using Gaussian Splatting*. arXiv:2409.10161. It combines simulator physics with Gaussian-splat appearance for RGB-policy training and sim-to-real evaluation. https://arxiv.org/abs/2409.10161 ; https://splatsim.github.io/
- **[R179]** Jia, Y. et al. (2025). *DISCOVERSE: Efficient Robot Simulation in Complex High-Fidelity Environments*. arXiv:2507.21981. It combines MuJoCo, Gaussian-splat rendering, robot assets, sensor paths, and parallel simulation. This is direct prior art against a broad “first 3DGS robot simulator” claim. https://arxiv.org/abs/2507.21981 ; https://air-discoverse.github.io/
- **[R180]** Zhang, S. et al. (2026). *RoboSnap: One-Shot Real-to-Sim Scene Generation for Generalizable Robot Learning and Evaluation*. arXiv:2607.06699. It separates collision-aware foreground assets from a Gaussian-splat visual layer and evaluates reusable real-to-sim scenes. https://arxiv.org/abs/2607.06699
- **[R181]** Maes, L.; Le Lidec, Q.; Scieur, D.; LeCun, Y.; Balestriero, R. (2026). *LeWorldModel: Stable End-to-End Joint-Embedding Predictive Architecture from Pixels*. arXiv:2603.19312v3. Reviewed code `Mengarr/lewm@8a2c595813d0eee85b2dbffa6f58ff0842f9e673`; its exact lock selects `stable-worldmodel==0.1.1` (source tag `15a5538d492ae524c64cb18cc56a2d70611e877e`) and `stable-pretraining==0.1.7`; reviewed PushT model revision `quentinll/lewm-pusht@22b330c28c27ead4bfd1888615af1340e3fe9052`. The released predictor is action-conditioned and the evaluator uses CEM. Project source and model-card terms declare MIT. The `stable-worldmodel` wheel metadata declares MIT, but the wheel and reviewed source tag contain no license file. The upstream evaluator hard-codes CUDA and its documented model conversion uses pickle-enabled loading. A local synthetic tensor/CEM probe ran on MPS, but no end-to-end MPS qualification was verified. Current platform main `9a66d7d020043c8efb507f45373e808714f0842d` has an incompatible planner constructor and is a separate migration target. https://arxiv.org/abs/2603.19312 ; https://github.com/Mengarr/lewm/tree/8a2c595813d0eee85b2dbffa6f58ff0842f9e673 ; https://github.com/galilai-group/stable-worldmodel/tree/15a5538d492ae524c64cb18cc56a2d70611e877e ; https://huggingface.co/quentinll/lewm-pusht/tree/22b330c28c27ead4bfd1888615af1340e3fe9052
- **[R182]** Singh, J. (2026). *The Evaluation Protocol Determines the Result: An Independent Reproduction of LeWorldModel on TwoRoom*. arXiv:2608.10145v1. This one-seed, TwoRoom-only study reproduces the representation probe and one published planning reading. It reports four consequential pipeline conventions absent from configuration files, conflicts between appendix and configuration evaluation settings, and strong goal-construction sensitivity on identical episodes. Across three checkpoints, one-step prediction error did not order long-horizon planning performance. It does not test PushT, MPS, other environments, or seed variance. Reviewed public reproduction source at the cutoff: `joyjeet-singh/tinylab@f2f665411d79cd626096ec8d4271b355a2c0f550`. https://arxiv.org/abs/2608.10145 ; https://github.com/joyjeet-singh/tinylab/tree/f2f665411d79cd626096ec8d4271b355a2c0f550
- **[R183]** Venkatesh, P.; Schamberg, G. (2021/2022). *Partial Information Decomposition via Deficiency for Multivariate Gaussians*. The work starts from the bivariate deficiency-based \(\delta\)-PID, defines Gaussian-channel-restricted deficiency \(\delta_G\), and then defines the further convex-surrogate \(\widehat{\delta}_G\)-PID with proved bounds and extremal agreements. These are not BROJA/\(\sim\), \(\sim_G\), or shared exclusions. https://arxiv.org/abs/2105.00769
- **[R184]** Venkatesh, P.; Gurushankar, K.; Schamberg, G. (2023). *Capturing and Interpreting Unique Information*. The paper identifies the \(\sim\)-PID as BROJA, analyzes its relationship to the distinct deficiency \(\delta\)-PID, introduces a parameterized \(\delta^\lambda\) family, and separately defines the information-deficiency I-PID. Its displayed small-\(\lambda\) raw objective tends to zero when exact copying is feasible, so the paper's stated BROJA endpoint still needs an explicit normalization or lexicographic limit theorem before becoming an equality edge. It proves I-PID Blackwellian for jointly Gaussian laws and leaves the general statement conjectural. https://arxiv.org/abs/2302.11873
- **[R185]** Venkatesh, P.; Bennett, C.; Gale, S.; Ramirez, T. K.; Heller, G.; Durand, S.; Olsen, S.; Mihalas, S. (2023). *Gaussian Partial Information Decomposition: Bias Correction and Application to High-dimensional Data*. The paper defines \(\sim_G\) by restricting the BROJA/\(\sim\) coupling optimization to jointly Gaussian laws, so its atoms bound BROJA in general and equality for Gaussian input laws remains conjectural. It develops covariance-law computation, sample-covariance estimation, and a separate finite-sample bias-correction route for \(\sim_G\). https://arxiv.org/abs/2307.10515
---

# Appendix A. Minimal canonical event envelope

The following target envelope is illustrative. The implemented `pid-runlog` schema remains
authoritative for current accepted recorded events. Adopting this wider envelope requires a
versioned schema and conformance tests.

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

Before opening a W1 or W2 confirmatory holdout:

- [ ] W1 and W2 are registered separately with one population, unit, supported action law,
      outcome, primary score or utility, comparator, positive useful margin, uncertainty rule,
      and non-rescuable decision gates;
- [ ] every W1 forecast, score, selection, and abstention commits before shared reference labels;
- [ ] W1 uses a strictly proper score for a predictive distribution or a strictly consistent loss
      for its named point functional;
- [ ] W2 randomizes complete policies and freezes proposal count, action support, controller,
      deadline, fallback, resource limits, and intention-to-treat analysis;
- [ ] W3 remains secondary, binds identical fork/action/state/camera identities across panels, and
      keeps immediate common-trajectory response separate from downstream randomized policy effects;
- [ ] the world-model freeze contract is separate from the current status ledger and binds exact
      code, weights, preprocessing, action support, prediction landmark, reference target, scorer,
      candidate generator, search rule, selector, controller, resource limits, and fallback;
- [ ] every candidate and adaptive-search step from one fork stays in one fold and one immutable
      pre-oracle trace;
- [ ] causal diagram and target level frozen;
- [ ] unit of inference and cluster structure frozen;
- [ ] eligibility gates passed;
- [ ] treatment assignment and manipulation checks validated;
- [ ] all preprocessing fitted on training data and hashed;
- [ ] baselines, model capacities, and hyperparameter budgets frozen;
- [ ] minimum useful effect and primary protocol-specific score frozen;

If a preserved diagnostic study opens its own holdout, also require:

- [ ] EC1 supported adapters, finite fault/valid-case universe, oracle, endpoints, per-fault–
      adapter absolute sensitivity floors and mandatory-pass rules, replay margins, false-positive
      endpoint, uncertainty, multiplicity, and decision rule frozen when EC1 is evaluated; no
      distribution-average sensitivity can rescue a failed pair;
- [ ] H1-A has one typed primary response contract with a positive useful margin, matched-access
      comparator, one-sided superiority rule, uncertainty, calibration consequence, multiplicity,
      and finite-benchmark or replication scope; bins never use held-out observed responses;
- [ ] H1-B has one typed primary effect-specific endpoint, a positive useful margin, a one-sided
      superiority rule, and an explicit hierarchy for all secondary confirmatory endpoints;
- [ ] H1-B binds and passes the effect-validation stack, overall ITT, assignment, engagement,
      specificity, nuisance checks, and directional replication; factual fit cannot establish
      success;
- [ ] H3/H4 selection is frozen with no more than three scientific claims and no outcome-informed
      branch switch;
- [ ] H3 warning codes have a frozen allowlist and exact same-fold M1 substitution/block disposition;
- [ ] H3 binds a target-specific prediction landmark before target realization or availability,
      source/target ancestry, producer and consumer implementations, and fail-closed per-row
      receipts that exclude post-landmark observations and target injection;
- [ ] H4 primary tuple, region rule, target weights, superiority/equivalence margins, and
      simultaneous familywise procedure frozen when H4 is active;
- [ ] H4 sample source, selection/overlap, transport assumptions, weight uncertainty, exactly one
      primary outcome, and joint operating-characteristic simulation are frozen;
- [ ] missingness, exclusion, reset-failure, and censoring rules frozen;
- [ ] multiplicity family and exploratory labels frozen;
- [ ] simulation-based design analysis passed;
- [ ] code, container, and environment digests recorded;
- [ ] holdout access audited;
- [ ] independent pilot or development-split negative and positive controls passed;
- [ ] result interpretation table drafted before unblinding.

# Appendix C. Result-interpretation table

| Observed result | Permitted conclusion | Prohibited conclusion |
|---|---|---|
| Learned predictor beats the matched W1 baseline under all frozen gates | supported fork-level forecast fidelity improved for the named outcome, action law, population, and resource regime | the model is a causal transition, physical truth, or a better deployed policy |
| Randomized complete selector beats the same-budget W2 comparator | the complete deployed decision system improved the frozen episode endpoint in the tested regime | fork-local regret proves general planning value or forecast mechanism |
| Linked W3 panels localize one error boundary | the named dynamics, renderer, frozen-policy, or selector contrast changed under its matched design | an additive causal decomposition, first-ever protocol, or real-world rendering truth |
| Diagnostic predicts paired frozen-snapshot response | diagnostic is useful for algorithmic-sensitivity prediction under the declared clone/coupling contract | diagnostic atom is a physical mechanism or closed-loop effect moderator |
| Diagnostic predicts randomized closed-loop effect modification under effect-specific validation | diagnostic is useful for effect moderation in the evaluated regime | diagnostic atom is the causal mechanism or an observed individual effect |
| PID beats full baseline set and replicates | PID adds conditional empirical value under named measure/estimator | PID is universally superior or necessary |
| PID does not beat baselines | no demonstrated incremental value in the evaluated regime | PID theory is false |
| Simultaneously controlled probe superiority and randomized cell-average effect equivalence pass in frozen regions | availability–tested-intervention-effect divergence for the frozen construction, outcome, target population, and certified region mass | any unit has zero effect, certified region mass is individual-effect prevalence, the represented concept is never used, or another intervention would also have a small effect |
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
- [ ] the primary improvement is oriented so positive values favor the diagnostic model;
- [ ] the useful margin is positive and the one-sided lower confidence bound must exceed it;
- [ ] calibration bins, calibration-failure consequence, allocation rule, testing hierarchy,
      dependence-aware uncertainty, multiplicity, and replication scope are frozen;
- [ ] noninferiority, equivalence, nonsignificance, or secondary endpoints cannot rescue a failed
      primary H1 endpoint.

Before H2 landmarking:

- [ ] time zero, horizon, eligibility, and prediction update schedule are frozen;
- [ ] all feature computations stop at the landmark;
- [ ] repeated landmarks and persistent-world groups stay in one fold;
- [ ] failure types, competing events, censoring, and missingness are defined;
- [ ] test prevalence and target prevalence are recorded;
- [ ] any estimated censoring model, calibration map, and threshold are trained only inside outer
      folds; a fixed censoring law is bound and justified separately;
- [ ] the prediction object, score family, target risk, censoring construction, identification
      assumptions, identifiable region, nuisance fitting, and uncertainty method form one aligned
      primary contract;
- [ ] exactly one primary scoring contract and its useful margin are frozen; alternative scores and
      decision utility are secondary and cannot rescue primary failure;
- [ ] calibration tolerance/recalibration, warning-time actionability, subgroup degradation, and
      multiplicity gates are frozen;
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

1. **Protocol-fault observatory (fixture phase implemented; live phase open).** The runnable phase validates one bounded, complete, content-addressed wire-0.8 fixture. It applies 18 frozen logical fault schedules. Every case runs twice through the live observer's shared route classifier and raw decoder. Strict per-replay records bind counters, delivery and finalization deltas, sample oracles, and durable bundles. The report separates injector truth, native response, manifest-oracle match, and path-independent replay equivalence. The inventory has 16 assessed cases, including one matched known limitation for whole-tick omission. Two guards are expected `not_assessable`, and the inventory has zero mismatches. `all_expectations_matched=true` is not an 18/18 detection rate. Read-only `--verify` snapshots the complete in-place publication. A reproducibility-bound local fixture execution requires matching build/runtime revisions, clean worktrees, and recorded lockfile and executable hashes. Otherwise, the typed level records a local fixture execution without that binding. This does not create producer-consumer E3. The NCP relationship remains E2. It is not signing or remote attestation. Logical slots do not measure timing. Trace truncation is not disconnect evidence. Offline no-control execution is not live noninterference. The declared-security-profile case does not load a configuration or test authentication and ACL behavior. The live phase requires a conforming external producer, receipt clocks, own-stream gap/QoS/reconnect/security evidence, interventions, outcomes, and an E4 report.
2. **Cross-domain diagnostic transport.** Export temporally aligned Crebain or Manwe-style perception/fusion streams through an adapter and test whether H2 diagnostics retain calibration under a non-manipulation embodiment. The primary result is transport failure or success under named shifts, not a universal VLA claim.
3. **Independent-monitor comparison.** Compare Galadriel consistency signals, ordinary uncertainty/OOD signals, and Prisoma diagnostics on the same randomized faults. Treat shared `pid-rs` outputs as one method family, not independent votes.
4. **Asset-diversity stress test.** Use a quality-controlled, license-cleared, physically validated subset of cobot-atlas to create prespecified object-appearance and geometry shifts. Keep generated assets grouped by lineage to avoid near-duplicate leakage.
5. **Reconstruction uncertainty as nuisance.** Use Melkor-derived reconstruction quality as a measured nuisance/effect modifier. First ask whether it predicts diagnostic failure; do not jump to unvalidated uncertainty-weighted PID.
6. **World-model counterfactual support.** Compare generated scenes against simulator-ground-truth interventions under explicit support and realism metrics. WorldWarp/GauSS-MI remain separate exploratory work until their adapters and estimators pass E4 and Section 7 gates.
7. **Project-owned categorical shared-exclusions case study.** Specify canonical integer count laws
   on one fixed alphabet and MGW two-source lattice. Reuse exact source counts across conditions
   when testing informative-component invariance. Bind the defining paper, equations, log unit,
   source order, target, event map, positive support, antichain coordinates, and pinned `pid-rs`
   backend. Require atom reconstruction, nonnegative
   informative/misinformative components, and the exact relationship between net and component
   contrasts. Keep this outside the world-model and `(V,L,D,A)` harnesses. It exercises one
   functional and implementation. It does not validate continuous PID, quantized hidden states,
   causal meaning, or the Prisoma application. Treat the fixed-source-law relationship as a
   paper-derived algebraic identity checked by a project-defined fixture unless an exact primary-
   source theorem locator is pinned. The informative cumulative term depends on the matching
   source union-event probability. Exact source-marginal equality and fixed Möbius inversion then
   preserve the informative atom vector, so each net-atom contrast is the negative of its
   misinformative contrast. Preserve empirical-count and specified-law roles as different result
   types even when their normalized masses agree.
8. **Neural-simulation producer trial.** After pinning a specific NEST-fork branch and documenting its delta from upstream, export a small neural-state fixture through a read-only NCP path. Evaluate clock/sequence semantics, provenance, replay, and noninterference before any neuroscience or embodiment claim.

None of these experiments is required for a successful thesis. Their value is that the same event and estimand contract makes heterogeneous embodied-agent investigations comparable without pretending their representations, clocks, action spaces, or causal targets are identical.

*End of canonical v13.0 proposal.*
