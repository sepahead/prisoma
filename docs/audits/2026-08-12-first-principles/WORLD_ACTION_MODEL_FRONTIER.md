# World-action-model frontier and M4 Max decision, 2026-08-13 refresh

**Status:** dated primary-source review and implementation decision
**Cutoff:** 2026-08-13T05:06:43Z (`2026-08-13T07:06:43+02:00`)
**ArXiv snapshot:** the queried feed reported `updated=2026-08-13T05:06:43Z`
**Recheck:** 2026-08-13; the August cohort contained 36 submissions
**Scope:** deployed computation graphs, evidence quality, Prisoma variables, and low-overhead local execution
**Non-claim:** this review is not a systematic review, independent reproduction, or benchmark result

The search used the arXiv API and primary arXiv HTML, official project pages, official source
repositories, and official model hosts. It queried the exact phrase “world action model,” named
architectures, and references from the newest relevant papers. The supplied X post was used only
to locate its underlying report. This bounded search can miss work that uses different terms.

A second search covered papers that retain the `VLA`, `latent dynamics`, or `world-language-action`
name while adding predictive objectives or deployed future paths. This matters because naming a
model `VLA` does not make it reactive, and naming a model `WAM` does not make it a planner.

The sorted arXiv API query `all:"world action model"` returned 36 submissions with August 2026
submission dates through the cutoff. Section 10 disposes every result. This closes the earlier gap
where the method named the query but did not show all results. The review still does not claim
systematic recall outside this bounded search set.

## Decision

Vision-language-action policies are not dead. The question creates a false choice.

`VLA` usually names an input-output interface. `WAM` can name a backbone, training loss,
generated-future path, action-conditioned predictor, or planner. One system can be both.

The 2026 frontier is a convergence around six distinct deployed designs:

1. direct vision-language-action policies;
2. direct policies with predictive training targets;
3. policies conditioned on an intended future;
4. coupled joint future-action generators without a clamped action query;
5. action-conditioned observational predictors; and
6. planners that compare candidate actions through predicted consequences.

The sixth design has two contracts. A fixed-pool planner scores one precommitted ordered pool. An
adaptive optimizer, such as CEM or Nevergrad, changes later proposals from earlier scores. The
adaptive contract must retain every round and separately score its final recommendation.

Prisoma must classify the directed graph and deployed algorithm. It must not classify a system
from its paper title, marketing label, or generated video.

For one M4 Max, the immediate path is:

1. run Prisoma's native zero-model-download exact-fork reference;
2. port LeWorldModel PushT as the first external world-model candidate;
3. retain JEPA-WM PushT as the second planning benchmark;
4. qualify SmolVLA as the direct-policy MPS baseline;
5. retain VLA-JEPA as a predictive-training comparator, not a stock `D` producer;
6. qualify SLIM as a smaller predictive-training alternative after its rights and loader gates;
7. consider Efficient-WAM only as a later class-J Metal port after its concrete blockers close;
8. test LiLa-WAM as a separate no-language predictive ablation; and
9. keep full video WAMs off the critical path.

LeWorldModel (LeWM) is the first external target because it combines an action-conditioned latent
predictor with CEM planning in a much smaller released stack. The exact reviewed source is
`Mengarr/lewm@8a2c595813d0eee85b2dbffa6f58ff0842f9e673`. Its committed lock selects
`stable-worldmodel==0.1.1`, wheel SHA-256
`00eaabd9e046e6364b3d1db47e5b365a0f628aea3a9376d6a407f75cbbbd2ef5`, from source tag
`15a5538d492ae524c64cb18cc56a2d70611e877e`. It also selects
`stable-pretraining==0.1.7`, wheel SHA-256
`60fc8fc3c9490e9a059aa7e038ab62cbe0505841e78c4165c18a99d8f599ec65`. The PushT model is
`quentinll/lewm-pusht@22b330c28c27ead4bfd1888615af1340e3fe9052`. Its 72,290,721-byte
`weights.pt` has SHA-256
`48938400ae3464c9680731287f583a9cb516f55a8ec64ea13a91be47fb15b607`. The paper reports about
15 million parameters. Project source and the model card declare MIT. The exact
`stable-worldmodel` package metadata declares MIT, but its wheel and source tag have no license
file. The `stable-pretraining` wheel includes an MIT license file.

Current `stable-worldmodel` main at `9a66d7d020043c8efb507f45373e808714f0842d` is not the exact
LeWM dependency. Main now constructs CEM with a `cost` object. LeWM's pinned evaluator passes a
`model`. Using current main therefore needs a reviewed migration and creates a distinct port arm.

The upstream LeWM evaluator still hard-codes CUDA. Its documented conversion uses
`torch.load(..., weights_only=False)` and serializes a Python object. A Prisoma port must instead
instantiate the pinned architecture and load only the digest-bound state dictionary with
`weights_only=True`. These facts make LeWM a promising low-overhead port, not MPS support or model
qualification.

An independent one-seed reproduction covers LeWM TwoRoom, not PushT or M4. It reports four
consequential pipeline conventions outside released configuration files. It also reports conflicts
between appendix and configuration evaluation settings. On the same episodes and checkpoint,
protocol-sensitive success moved from 84% to 8% through goal construction alone. Across three checkpoints, one-step error did
not order long-horizon planning. This evidence does not reject the port. It requires a
source-concordance ledger and pre-outcome frozen protocol arms before any LeWM result is compared.

On 2026-08-14, an exact-package synthetic probe loaded the checkpoint with `weights_only=True` and
strict state-dictionary matching. Direct prediction, latent rollout, and the published 30-round,
300-sample CEM loop produced finite repeatable MPS outputs. CPU/MPS direct-prediction maximum
absolute difference was \(3.28 \times 10^{-6}\) on the tested tensor. This was one compatibility
observation, not a tolerance or performance benchmark. It omitted official preprocessing, PushT,
closed-loop replanning, and CPU/MPS candidate-order comparison. It does not establish MPS support
or WM2. See [`LEWM_M4_COMPATIBILITY_PROBE.md`](LEWM_M4_COMPATIBILITY_PROBE.md) for the exact
artifact identities, observations, and remaining gates. The probe code and synthetic input are
not committed, so the observations are not a repository-reproducible receipt.

The published evaluator fits `StandardScaler` instances on the evaluation dataset. The locked CEM
uses `Box` only to obtain action dimensions and does not enforce its bounds. A Prisoma study must
bind a train-only scaler, inverse-transform proposals, and check raw-action support. It must freeze
one reject, project, or truncated-sampling rule before outcomes. The unmodified route is therefore
a descriptive reproduction arm. A support-bounded solver is a distinct port arm.

JEPA-WM remains a second planning benchmark because its evaluator also performs
candidate-action-conditioned latent rollout. Its exact reviewed source is
`facebookresearch/jepa-wms@13cf1d9c7e476f53c17714d2e0f1dc239a883ce0`, and its PushT checkpoint is
`facebook/jepa-wms@9b9c41ef249466630dbf1a20e78391865d07b3b9`. Its 211,639,615-byte checkpoint
and code use CC-BY-NC-4.0. The upstream agent and planners hard-code CUDA. The hub loader also
changes a requested MPS device to CPU. These constraints make it a weaker first foundation.

VLA-JEPA remains a bounded MPS engineering comparator. Its official LeRobot implementation
does not call the training-only predictor during policy inference. It disables AMP on MPS. The
reviewed checkpoint uses safetensors, and its model card declares Apache-2.0. These facts lower
loader and device risk. They do not make the stock path an admissible Prisoma `D` source. Its
world-loss path requests observations at offsets 0 through 7. After tubelet encoding, the predictor
output mixes temporal positions with different tubelet ancestry. Later positions include post-
decision frame context. The method returns only a scalar loss. Its conditioning inputs are learned
Qwen latent-action tokens, not clamped robot actions. The pinned encoder uses two-frame tubelets,
so even the earliest context position reads frames `t` and `t+1`. No stock prediction position is
a row-`t` pre-action state. An inference-visible Qwen state is a
predictive-trained current-context candidate, not a predicted future. SLIM is smaller and remains
a useful predictive-training alternative. Its reviewed loader enables pickle and its rights are
nonstandard. LiLa-WAM is a compact no-language check, not a full-VLDA candidate.

## 1. First-principles taxonomy

Let:

- \(H_t\) be all policy-visible history at decision time;
- \(L\) be the instruction;
- \(F\) be a generated or predicted future;
- \(A^\pi\) be the policy proposal;
- \(A^{exec}\) be the command after controller conversion; and
- \(Y^{ref}\) be a later outcome from one declared simulator or reference system; and
- \(Y^{phys}\) be a separately measured physical-system outcome, when available.

These are different operational and statistical contracts:

\[
q_{\mathrm{intent}}(F\mid H_t,L),\quad
q_{\mathrm{joint}}(F,A^\pi\mid H_t,L),\quad
q_{\mathrm{query}}(F\mid H_t,L,A^\pi),\quad
p(F\mid H_t,L,\operatorname{do}(A^{exec})).
\]

The first predicts likely or intended progress. The second samples a coupled pair. The third is a
callable candidate-action forecast. The fourth is an interventional transition law.

| Class | Deployed graph | Defensible description | What it does not establish |
|---|---|---|---|
| A | \(\pi(A^\pi\mid H,L)\) | direct policy | predictive representation, simulation, or planning |
| B | predictive loss in training; direct policy in deployment | predictive co-training or representation regularization | runtime future use |
| C | \(q(F\mid H,L)\,\pi(A^\pi\mid H,L,F)\) | intended-future conditioning plus inverse policy | action-conditioned consequences |
| J | jointly sample \(q(F,A^\pi\mid H,L)\), without an exposed clamped-action query | coupled joint generation | an operational \(q(F\mid H,L,A^\pi)\) query |
| D | \(q(F\mid H,L,A^\pi)\) | action-conditioned observational prediction | an interventional transition or planner |
| E | propose \(A_1,\ldots,A_K\), predict, score, select | predictive candidate planner | causal validity of the learned simulator |

A class-E planner must satisfy all of these conditions:

- it creates at least two candidate actions;
- it predicts consequences under each candidate;
- it applies a frozen score or objective;
- it selects an action because of those scores;
- it records candidates, predictions, scores, and the selection; and
- it passes a decision-flip test with fixed proposals and changed predicted consequences.

Class J is defined by the deployed interface. A mathematical conditional factorization of a joint
density does not create a callable action-conditioned forecast. Joint generation is not sufficient.
Video prediction is not sufficient. A value head is not sufficient. An action-conditioned
simulator without candidate selection is class D.

## 2. Current architecture audit

The table reports the reviewed paper or official artifact. Reported benchmark values remain
author results unless this repository reproduces them.

| System | Deployed graph | Class | Evidence decision |
|---|---|---:|---|
| SmolVLA | compact direct VLA | A | MPS baseline candidate; no world-state claim |
| VLA-JEPA | learned latent-action tokens condition a training-only future objective; policy inference does not invoke the predictor | B | predictive-training evidence, not a clamped physical-action query; the two-frame tubelet context makes no stock prediction position a row-aligned pre-action `D` |
| LDA-1B policy mode | future DINO prediction is a co-training task; policy mode uses a visual register rather than a generated future | B | 1B dynamics backbone plus Qwen3-VL-4B and DINOv3; not a low-overhead M4 route |
| LDA-1B forward mode | action and current state can be supplied to a separate future-latent task | D | useful observational query; not an identified transition law or deployed planner |
| Dyna-2 scaling policy | video and action use separate marginal velocity fields; action does not consume predicted video | B | evidence for predictive co-training, not online imagination |
| Fast-WAM | future branch is removed at inference | B | direct policy at deployment |
| SLIM | action-conditioned future-latent prediction in training; deployment denoises actions from current features plus learned future-slot placeholders | B | compact predictive-training candidate; no explicit future is decoded or consumed at deployment |
| LiLa-WAM | bidirectional flow policy returns predictive-trained shared tokens; released inference ignores them and never calls the future decoder | B | 0.5B no-language predictive ablation; not a deployed simulator |
| World Tokens | training-time video denoiser supervises world tokens; denoiser is removed at inference | B | exclusive predictive-trained route, not deployed simulation |
| JEPA-WAM, arXiv:2608.09381 | shared latent predictor trains the policy; transition head is removed at deployment | B | current-context predictive training |
| SG-WAM, arXiv:2608.01397 | action-conditioned latent prediction trains policy tokens; predictor and geometry teacher are removed at deployment | B | compact predictive-training evidence, not a deployed transition query |
| Vid2WAM | offline video-teacher and inverse-dynamics targets train a 5B student; every prediction head is removed at deployment | B | distillation evidence, not online imagination |
| Robust-WAM | semantic foresight aligns the action stream during training; the teacher and alignment head are removed, while learned query tokens remain | B | predictive-trained query state, not a decoded future or transition query |
| MobileWAM | a future-latent chain supplies training supervision; foresight is removed at deployment | B | mobile-manipulation policy, not runtime simulation |
| WLA-0 efficient mode | a world expert supervises shared physical-dynamics queries during training, then is disabled | B | predictive and language co-training; no runtime forecast is used |
| ForeWAM | one video-backbone prefill maps current context and stochastic future slots to hidden K/V and dynamics-register state that conditions action denoising | C | one-pass intended-future conditioning; no decoded future, candidate-action query, or released artifact verified |
| Rift | one prefill maps current context and learned anticipation tokens to a fixed future-position K/V cache that conditions action denoising | C | future-cache use is tested by paired interventions; the cache remains action-independent and paper-only at the cutoff |
| Stage-level JEPA-WAM, arXiv:2608.10780 | predicts an intended next-stage latent from observed history and language, then conditions short-horizon video and action generation on it | C | deployed semantic-stage guidance, not action-conditioned consequence prediction |
| Flex-\(\pi\) | generated future cannot attend actions; actions can attend the generated future | C | intended-future policy, not action-conditioned dynamics |
| DreamZero | video prediction followed by inverse dynamics | C | generated internal goal, not candidate consequence comparison |
| World-to-Wrist VLA | predicts wrist futures from current task context and wrist history, then conditions actions on them | C | deployed intended-future latent path; not candidate-action conditioning |
| Efficient-WAM | jointly denoises future-video and action tokens with bidirectional attention, then reuses cached video keys and values during later action-only steps | J | released 1B coupled sampler; no clamped candidate-action query; CUDA-oriented and not MPS-qualified |
| SelfWAM, default | clean demonstrated action conditions future during training; deployed action path omits it | B | auxiliary action-conditioned learning |
| SelfWAM, optional rollout | proposed action conditions a generated future | D | observational action-conditioned query |
| UWM forward mode | predicts a next observation from current observation and action | D | learned expert-data transition, not causal identification |
| FACT | predicts future and value from a proposed action | D, or E with its optional four-candidate selector | planning only when candidate selection is enabled |
| Surgical WAM | jointly samples future-video and action slots at deployment; action slots attend future slots | J | coupled generation; no exposed clamped-action query, candidate comparison, or causal identification |
| DreamWAM | no-rollout mode uses predictive training only; joint mode denoises RGB futures and actions together | B or J | mode-dependent graph; structured-future gains do not identify a causal transition |
| DynamicWAM | mutually attends future-video and action streams; only actions are decoded for control | J | compact joint generation for moving targets, not candidate planning |
| FlowPilot | mutually denoises future depth and trajectories; the controller consumes only the trajectory | J | fast UAV joint generator, not a manipulation or M4 artifact |
| CoWAM | receives a fixed candidate pool with predicted futures, scores every candidate, then preserves, overrides, or abstains | E | conservative selector evidence; predicted futures remain observational |
| \(\tau0\)-WM ACVS | proposes, simulates, scores, and selects or rectifies actions | E | planning-class example with an integrated action-correction path |
| WLA-0 test-time scaling | samples at least two action chunks, predicts a future frame for each, scores them, and selects the top candidate | E | optional planner mode; value and forecast validity remain author evidence |
| World Action Planner | proposes and refines action plans, simulates a grid of action candidates, ranks imagined outcomes, and executes the selected candidate | E | explicit planning-class system; simulation evidence and a model-validity requirement remain |
| CheckVLA | predicts expected execution under already committed actions, compares prediction with new observations, and triggers a latency-aware suffix rewrite | D plus verifier/repair wrapper | strong monitor comparator; not candidate planning or a causal transition by architecture |

Two August papers use the name “Faster-WAM” for opposed designs. arXiv:2608.02365 removes deep
action computation and future generation at deployment. arXiv:2608.04404 retains inexpensive
inference-time future conditioning. This name collision is evidence that labels are not variables.

LiLa-WAM illustrates a second naming trap. The paper describes an action-conditioned future
latent. The reviewed release returns shared `cond_tokens` from each bidirectional flow step.
Its inference loop uses only `final_pred`. It never invokes the future decoder. Prisoma therefore
classifies the released path as predictive co-training, not runtime future use.

Surgical WAM is a relevant class-J system in the screened cohort. It jointly samples video and
actions with a Cosmos Policy backbone, then executes a short action prefix. Its paper states that
action tokens attend denoised future tokens at every diffusion step. It does not expose a clamped
candidate-action forecast query. The paper reports a matched video-pretraining ablation on four
simulated surgical tasks. No official runnable code or checkpoint was verified. The table therefore
places the deployed sampler in class J, not class D. It is frontier evidence, not an M4 candidate
or independent result.

CoWAM is the clearest newly screened class-E design. Its frozen proposer returns at least two
action-future pairs. A separate selector checks typed obligations, computes frozen scores, and
chooses preserve, override, or abstain. Its same-pool protocol separates proposal headroom from
selection quality. The paper reports simulation only, and it exposes no public runnable artifact.
Its forecasts are still observational. CoWAM improves the planner contract, not causal admission.

World Action Planner is a second explicit class-E design outside the exact-phrase August cohort.
Its algorithm proposes actions, predicts each grid-search candidate with an action-conditioned
world model, ranks the imagined outcomes, and executes the selected candidate. This satisfies the
operational planning definition. Its simulation results do not validate the world model as an
interventional transition law, and no official runnable artifact was verified in this review.

CheckVLA uses a different graph. It conditions a frozen world model on actions that the policy has
already committed. It checks later observations against the predicted execution, then triggers a
latency-aware suffix rewrite. This is a class-D prediction and verification loop, not candidate
planning. Its controlled action-shuffle and observation-only comparisons make it a strong H2 and
monitoring comparator. They do not establish causal transition validity or external transport.

ForeWAM and Rift appeared after the first 34-paper cutoff. Both preserve a future-position state
that actions can read, but both construct it from current context without a candidate action.
ForeWAM uses stochastic future slots, one video-backbone prefill, and latent-action-supervised
dynamics registers. Rift uses learned anticipation tokens and one prefill to write a fixed K/V
cache. Both are class C, not class D.

Rift also contributes a directly relevant intervention design. It records an action-independent
future cache, changes or masks the future values while holding the remaining inputs fixed, and
measures paired closed-loop execution. Those tests support bounded use of the tested cache path.
They do not show that its imagined state is physically correct, that another future interface is
used, or that the cache identifies a causal transition.

The compact papers do not all solve the same local problem. DynamicWAM reports 988.8 million
trainable stage-three parameters, but also needs UMT5-XXL and a Wan VAE. It trained on eight H100s.
FlowPilot reports an end-to-end Jetson result, but targets depth-only UAV navigation. SG-WAM reports
a 0.9B policy and removes its predictor at inference, but no official implementation was found.
These systems supply architecture tests. They do not displace a released, auditable M4 candidate.

The broader-name search further rejects a successor-era story. World-to-Wrist keeps the `VLA`
name while consuming a predicted wrist future at deployment. WLA-0 is class B in its default
efficient mode and class E in its optional test-time mode. LDA-1B is class B when invoked as a
policy and exposes a separate class-D forward task. The deployed graph changes while the paper
label stays fixed.

Efficient-WAM adds a second implementation-level warning. The paper emphasizes cheap future
imagination, but the released sampler concatenates video and action queries, keys, and values for
full joint attention. Video tokens can therefore depend on action tokens before later action steps
reuse cached video keys and values. The release is class J, not class C or D. A paper diagram or
factorized prose does not override executable information flow.

## 3. Flex-\(\pi\): exact correction

The [Flex-\(\pi\) paper](https://arxiv.org/html/2608.10860) states that no future visual token
attends the action stream. Action tokens can attend the generated future.

Its deployed full-mode graph is therefore:

\[
q(F\mid H,L,P,s)\;\pi(A^\pi\mid H,L,P,F,s),
\]

where \(P\) is proprioception and \(s\) is sampling state. It is not
\(q(F\mid H,L,A^\pi)\). The paper's phrase “Causal Joint Generation” describes directed
attention. It does not identify a causal estimand.

Important boundaries follow:

- changing a candidate action cannot change the generated future through the declared graph;
- generated futures can change the action proposal in full mode;
- this supports a future H1-A use test, not a counterfactual-dynamics claim;
- RGB, DINO, and pointmap streams share RGB ancestry;
- external forecast calibration is not reported quantitatively; and
- task success cannot substitute for action-consequence validity.

The paper reports a 6B system with a 5B Wan backbone and about 1B action expert. It reports 32
real-robot actions per chunk at 30 Hz. This is about 1.07 seconds of open-loop execution before
replanning. One-call latency is therefore not feedback rate.

The paper reports about 60 ms for action-only inference and 193 ms for full generation on an RTX
5090-class system. Paper-reported peak memory is about 15.8 GB for eager or compiled execution.
Paper-reported TensorRT peak memory is about 26.4 GB. The latency protocol uses three warmups and 20 synchronized
timed calls per configuration, then reports the mean. It does not report tail latency. These are
not MPS results.

The paper reports that full joint generation improves its selected real-robot task-completion
result over action-only inference. It uses several summary conventions across the main text and
appendix. Do not collapse those values into one unsupported success-rate pair. The reported finite
difference motivates a controlled runtime-use test. It does not prove a universal WAM advantage.

At the cutoff, the [official repository](https://github.com/geyan21/flex-pi) resolves to
`9f07a4c6ffecb5ae058879566cc0bb2fe9121703`. It contains a README and promises code and
checkpoints later. It has no repository license. Flex-\(\pi\) is therefore not an executable M4
port target.

## 4. The “VLAs are dead” claim

The slogan bundles three different hypotheses:

1. a vision-language pretrained backbone is an inefficient control backbone;
2. predictive video or latent supervision improves the learned policy representation; and
3. deployed action-conditioned prediction or candidate planning improves control.

Evidence for one hypothesis cannot establish the others. Most current low-overhead results test
the second hypothesis. They remove prediction heads or future generation at deployment.

The theme predates the supplied post. A
[June 2026 X reply](https://x.com/NGO275/status/2063522926390128676) attributes the slogan to the
need to predict action consequences. An
[NVIDIA technical explainer](https://developer.nvidia.com/blog/pretrained-to-imagine-fine-tuned-to-act-the-rise-of-world-action-models/)
instead defines the split by pretraining backbone: VLM for VLA and video or world model for WAM.
These are research programs and taxonomy choices. They are not a controlled comparison or field
consensus. NVIDIA also uses VLA language for current products while describing a WAM architecture
for a later generation.

The supplied [X post](https://x.com/CyberRobooo/status/2086999365789601933) is a discovery lead.
Its numbers point to the [Dyna-2 company report](https://www.dyna.co/dyna-2).

Dyna reports a 1.55-fold pooled mean-success ratio across seven tasks and three checkpoints. It
also reports winning 65 percent of 21 task-checkpoint cells. The public evidence does not expose
the raw independent trial units needed to audit this comparison.

More importantly, Dyna's action field does not consume its video field. The deployed policy is
reactive. Its result supports the value of a video-derived architecture and predictive co-training.
It does not show that an online simulator replaced a VLA.

The report contains two different internal comparisons.

Its same-Dyna-2 objective study compares action-only, joint action-plus-future training, and joint
training plus extra video-only data. It holds the architecture and action-data scale fixed. The
report says joint training wins all 39 tasks at each tested scale. This is useful matched evidence
for predictive co-training inside that system. It is company-reported, not independently
reproduced, and it does not test online future use or planning.

Its Dyna-2-versus-Dyna-1 comparison is matched on pretraining data, post-training data, training
hyperparameters, and three starting checkpoints per architecture. It is still not a causal test of
one world-model mechanism:

- the prior and new policies use different architecture families;
- the early Dyna-2 comparison omitted its later large pretraining stage;
- its action-only loss supervised the full early model;
- the reported 1.55-fold ratio pools trials and checkpoints; and
- code, checkpoints, raw trials, and a complete protocol are not public.

The strongest defensible conclusion is:

> Dyna reports a matched internal advantage for predictive co-training within Dyna-2 and a matched
> internal architecture-family advantage over Dyna-1. Neither result establishes that online
> world simulation or planning replaces direct VLA policies.

The broader field also contradicts a replacement story:

- JEPA-WAM adds predictive learning to an existing VLA path;
- VLA-JEPA does not invoke its predictor during policy inference;
- EgoWAM uses a training-time world objective;
- World Tokens removes its video denoiser at deployment;
- RoboTTT extends a VLA with test-time learning;
- \(\pi R^2\) adds fast proprioceptive control around slower VLA updates; and
- Flex-\(\pi\) exposes both action-only and generated-future modes.

The field is moving beyond a narrow, vision-language-only policy recipe. It is not abandoning the
vision-language-action interface.

## 5. Predictive validity and causal admission

An action-conditioned model may use causal action-consequence language only after it passes a
declared interventional gate.

1. Randomize actions from identical or valid reset states.
2. Include no-op, reversed, failed, and low-support actions.
3. Record \(A^\pi\), controller conversion, and \(A^{exec}\).
4. Record truncation, holds, latency, and safety overrides.
5. Use proper distributional scores and calibration.
6. Test hidden-state aliases for mass, friction, contact, and occlusion.
7. Measure support distance and abstain outside supported regions.
8. Preserve failure-heavy cases instead of selecting successful demonstrations.
9. Replicate on a held-out task family or embodiment.

[MiraBench](https://arxiv.org/html/2605.29360v1) directly motivates these tests. It reports that
visual fidelity can diverge from action fidelity. It also reports optimism under failure actions.

The [world-model hallucination audit](https://arxiv.org/abs/2606.27326) adds another warning.
Plausible pixels can ignore actions or violate dynamics. Image quality cannot validate control.

Three August studies sharpen the representation gate. XEWorld reports that reviewed
action-conditioned models follow visual similarity more than kinematic similarity on held-out
embodiments. PhyLatent reports physical-invariance, identifiability, and counterfactual-dynamics
collapse despite global non-collapse. PSG-JEPA argues that forward prediction alone does not
guarantee identifiable physical state or change. These are author results, but they directly
reject “it predicts forward” as a sufficient validity test.

HarnessWAM and TempoWAM also expose a systems lesson. A finite-horizon predictor does not supply
task memory, execution verification, recovery, or a correct replan schedule by itself. Their
external task-state and progress-monitor designs motivate a low-overhead monitor around a policy.
They do not make the underlying simulator causal or scientifically valid.

### 5.1 Runtime scheduling is a separate system axis

[World Action Models in Real Time](https://arxiv.org/abs/2608.01880) compares synchronous
execution and five asynchronous chunk-reconciliation strategies on one 10 Hz bimanual platform.
The policy emits 24-step chunks. The study schedules new inference after four executed steps and
uses an estimated eight-frame end-to-end delay.

The most portable result is not the reported method ranking. It is the timing contract. An
incoming chunk must be indexed to the physical state that exists when its commands execute. A
wrong delay estimate creates a chunk-boundary jump that blending cannot repair.

The online study uses three tasks and five trials for each method–task cell. Its reported preference
for prefix-conditioned generation is therefore platform-specific evidence, not a universal WAM
deployment rule. Prefix conditioning also changes the trained policy. It is not a free runtime
optimization.

Prisoma must record observation capture, inference start and finish, committed-prefix indices,
command dispatch, and execution acknowledgement. Low overhead does not justify a stale-action
shortcut. First measure the delay distribution and simple aligned action blending. Treat a
prefix-conditioned model as a separate intervention or model revision.

## 6. Consequences for Prisoma `(V,L,D,A)`

`D` must follow the computation graph. A paper's model name cannot define it. A learned token named
"action" is not a clamped robot action unless its value comes from the physical action interface.

| Architecture | Permitted `D` label | Prohibited label without more evidence |
|---|---|---|
| Flex-\(\pi\) future path | `generated intended-future representation` | action-conditioned or counterfactual state |
| Surgical WAM joint sampler | `coupled joint-sampler future/action state` | clamped candidate-action forecast |
| Flex-\(\pi\) shared trunk | `current/future-trained fused visual state` | independent dynamics source |
| VLA-JEPA, Dyna-2, Fast-WAM, SLIM, LiLa-WAM | `predictive-trained current-context state` | deployed future simulation |
| SelfWAM or UWM forward query | `candidate-action-conditioned predictive state` | interventional transition |
| \(\tau0\)-WM planner | `candidate-action-conditioned predictive state` plus planner records | causally valid simulator |

Use \(A^\pi\) as the action target for the policy proposal. Preserve controller output and
\(A^{exec}\) as separate targets.

Do not use a state computed from \(A^\pi\) as a PID source whose target is that exact
\(A^\pi\). This construction injects the target into the source. Cross-fitting does not repair
it. A controller command, executed action, later declared reference-state outcome, or separately
measured physical outcome remains eligible only when the matched baseline receives the same
proposal. Command and simulator-state prediction do not establish physical forecast validity. Use
a separately measured physical outcome for that claim, or keep the state inside the class-D/E
comparison.

Do not treat shared RGB-derived streams as independent conceptual modalities. Do not include a
task-constant language axis in a language-information claim. Do not use a post-decision `D` as an
H1 pre-treatment moderator.

Every adapter for this family must record:

- the exact attention and stream mask;
- layer and tensor-site identity;
- action and future-noise substreams;
- solver steps and seeds;
- candidate or demonstrated action conditions;
- proposal, controller output, and executed action;
- chunk start, hold, truncation, and replan times;
- checkpoint, encoder, preprocessor, and normalizer hashes; and
- whether the future branch exists at deployment.

It must also bind one target-specific prediction landmark before target availability. Bind the
maximum observation time in each tensor's ancestry. Reject any row whose source reads beyond that
landmark.

EC1 can prove capture and replay of these records. It cannot prove forecast validity, causality,
natural use, or planning.

## 7. Matched experiment that resolves the argument

Use one backbone, data set, optimizer, parameter budget, compute budget, and evaluation contract.

| Arm | Training | Deployment | Primary contrast |
|---|---|---|---|
| A | action only | direct policy | baseline |
| B | action plus future loss | direct policy | predictive-representation effect |
| C | action plus future loss | intended future visible to action | runtime intended-future effect |
| J | coupled future-action objective | jointly sample future and action slots without a clamped action query | coupled-generation effect |
| D | action-conditioned predictor and frozen scorer | compute proposals, forecasts, and scores, but execute the frozen direct-policy proposal | forecast validity without selection |
| E | same predictor, scorer, and candidate set as D | select among at least two proposals by the frozen score | selection effect |

Match task, embodiment, reset, controller, candidate set, latency, compute, and evaluation budgets.
Validate the action-conditioned predictor under randomized executed actions. Keep D's prediction
and score outside the control path. Require E to pass a fixed-proposal decision-flip test. The D–E
contrast is interpretable only when the proposals, predictions, scores, and non-selection code are
otherwise identical. Report task-level counts instead of one pooled ratio.

For a Flex-\(\pi\)-style H1-A study, use immutable clones. Keep action noise identical. Give
future generation a named, order-independent substream. Reset caches and solver state. Derive any
moderator from an untreated third clone.

For H1-B, randomize the complete deployed mode across episodes or reset blocks. Equalize latency
and holds, or define them as part of treatment. Otherwise, a physical effect cannot be assigned
to future semantics.

## 8. M4 Max low-overhead pipeline

The target is low overhead. It is not a claim that a large model is “lean.”

### 8.1 Candidate order

| Candidate | Exact reviewed artifact | Local decision |
|---|---|---|
| Native reference | current Prisoma source; `just world-model-reference` | zero-model-download decision-contract proof; Cargo dependencies still apply; not learned-model evidence |
| LeWM PushT | `Mengarr/lewm@8a2c595813d0eee85b2dbffa6f58ff0842f9e673`; locked `stable-worldmodel==0.1.1` wheel `00eaabd9…`, source tag `15a5538d…`; `quentinll/lewm-pusht@22b330c28c27ead4bfd1888615af1340e3fe9052` | first external world-model port; exact tensor and CEM paths ran in one synthetic MPS probe; upstream evaluator remains CUDA-hard-coded and end-to-end MPS is unqualified |
| JEPA-WM PushT | `facebookresearch/jepa-wms@13cf1d9c7e476f53c17714d2e0f1dc239a883ce0`; `facebook/jepa-wms@9b9c41ef249466630dbf1a20e78391865d07b3b9` | second planning benchmark; genuine planning interface, CC-BY-NC, CUDA-hard-coded upstream |
| SmolVLA | `huggingface/lerobot@a16f34c085c9597fcbdb9fde395a3334d78df716`; `lerobot/smolvla_base@c83c3163b8ca9b7e67c509fffd9121e66cb96205` | baseline candidate; upstream shows an MPS example |
| VLA-JEPA | `huggingface/lerobot@a16f34c085c9597fcbdb9fde395a3334d78df716`; `lerobot/VLA-JEPA-LIBERO@735d9f692981e286ade093b5046627eda876e5d0` | bounded MPS engineering smoke test; stock predictor is not a clamped robot-action query or a row-aligned `D` producer |
| SLIM | `kzz1031/SLIM@f3a544700c537e4bd720e8d0aa0d82599ec79e6b`; `kzzwang/SLIM-LIBERO@921b05ee80fb38fb5df84df4ff2db68aead8d15e` | smaller predictive-training alternative after loader and rights qualification |
| Efficient-WAM | `jiajun613/Efficient-WAM@2bd75a8c56acfcd5754b98c7ed313176911ccae0`; `jiajun0613/Efficient-WAM_RoboTwin@81280a79e8ac69dd6ffb9ce8698e00d122ec07fd` | released 1B class-J port candidate; behind SLIM because current MPS blockers are concrete |
| JEPA-WAM | `SpriteWithoutIce/JEPA_WAM@537830bee0d84d10266a14cad7f038b653b717d8`; `CokeAnd1ce/JEPA_WAM@ca10ccbc191d8f56b4346487913e043b2722b6d2` | released compact predictive candidate; CUDA stack and unsafe checkpoint format need port review |
| LiLa-WAM | `teee000/LiLa-WAM@b6a2095d76927119bcfc0d2ca04eb5cea98d10d8`; ModelScope `yangfan97/LiLa-WAM_RoboTwin2_0@93ab191b2500aa37322244c4ae0e84eed1e848ee` | no-language predictive ablation; port and rights review required |
| Light-WAM | `L1ziang/Light-WAM@b2785f66e13fd9987e94ae1ecc1c441d5059c9ae`; `l1ziang/lightwam-checkpoints@7cc8593fb95423a9cfbb93f82c95c2fa7d5357bd` | later port; current RoPE and device logic need work |
| Fast-WAM | code `45d8e1458921d83f8ad6cf9ce993d371208dabd0` | reject for the local critical path |
| Flex-\(\pi\) | code placeholder `9f07a4c6ffecb5ae058879566cc0bb2fe9121703` | not runnable at cutoff |

LeWM PushT uses a tiny Vision Transformer encoder and an action-conditioned autoregressive latent
predictor. The official model configuration uses five two-dimensional actions per block. The
published evaluation config uses 30 CEM rounds, 300 samples, 30 elites, horizon five, and action
blocks of length five. A faithful adapter must retain each adaptive round and separately score its
final recommendation before execution.

The independent reproduction studied the 18.03-million-parameter TwoRoom configuration, not this
PushT checkpoint. Its boundary still matters. It found dense frameskip action gathering,
programmatic action-encoder width, ImageNet pixel normalization, and action z-scoring with NaN
removal outside released configuration files. It also found configuration-versus-appendix
conflicts for goal offset, step budget, and CEM iterations. Prisoma must audit the analogous PushT
pipeline fields from paper, configuration, and executable code. Each unresolved feasible reading
becomes a separate frozen arm. An observed outcome cannot choose the arm.

The reviewed core model and platform planner accept an explicit device. The official evaluator
still sends the model to CUDA. Core portability therefore does not prove an end-to-end MPS path.
The port must verify CPU and MPS outputs, action sensitivity, candidate order, random-number
ownership, deadlines, and unified-memory use. It must not hide CPU fallback. The full PushT data
download and paper benchmark are not required for a bounded engineering smoke test, but such a
smoke test is not a paper reproduction.

JEPA-WM PushT uses a DINOv2 ViT-S/14 encoder and an action-conditioned latent predictor. The
optional decoder is not needed for planning. The exact checkpoint is 211,639,615 bytes. The
official repository and model card declare CC-BY-NC-4.0. Its checkpoint is a PyTorch archive, so
the port must verify the digest and extract weights through a reviewed loader before use.

The stock evaluator is not MPS-ready. `GC_Agent` creates `cuda:0` devices and generators. CEM,
MPPI, and gradient planners create CUDA devices. The hub loader sends every non-CUDA host to CPU,
even when the caller requested MPS. A narrow port can make device ownership follow the model,
keep the decoder absent, and use a device-following planner. This must be demonstrated, not
assumed.

The reviewed PushT configuration uses CEM, not Nevergrad, as its primary planner. It runs 30
rounds with 300 trajectories per round, 10 elites, and horizon six. CEM returns the final elite
mean. That mean need not be one sampled trajectory. A faithful port must retain every proposal and
score, then evaluate and commit the final mean before execution. Keep Nevergrad as a distinct
optimizer ablation.

JEPA-WAM at arXiv:2608.09381 now has source and weights, but it remains unqualified locally.
The stage-level JEPA-WAM at arXiv:2608.10780 is paper-only at this cutoff. No official runnable
code or checkpoint was verified for it. Neither system belongs in the M4 execution path until its
exact code, weights, rights, loader, memory use, and MPS behavior pass the same gates.

The reviewed JEPA-WAM release uses a 300M V-JEPA 2.1 encoder and Qwen2.5-0.5B policy backbone.
Its main LIBERO checkpoint is a 5,355,388,110-byte PyTorch file with SHA-256
`e63285fb347048989f14a8a24962a2b921d787f7ada0176a0eacd6b256d57d23`. The source and model
card declare MIT, but its V-JEPA, Qwen, data, and simulator dependencies retain separate rights. The tested
environment pins Python 3.10, CUDA 12.1, PyTorch 2.2, and FlashAttention. It is not an MPS setup.
The loader uses pickle-based `.pt` files. Require exact digests, a reviewed loader, and an
SDPA-based MPS path before the first local run.

SmolVLA has about 450 million parameters and a 906,712,520-byte weight file. Its Hugging Face
card does not declare a weight license. The Apache-2.0 LeRobot code license does not automatically
license the weights.

VLA-JEPA's reviewed checkpoint contains about 3.08 billion BF16 parameters in a
6,163,212,550-byte safetensors file. Its model card declares Apache-2.0. Its LeRobot inference path
does not call the training-only world predictor. The shared device utility disables AMP on MPS,
and the reviewed inference method contains no CUDA literal. These facts lower loader and device
risk for an engineering smoke test on this 128 GiB host. They do not establish that every
dependency runs on MPS or that the model meets latency, parity, memory, or hook gates. The stock
world-loss path loads offsets 0 through 7. It mixes prediction positions with different tubelet
ancestry. It
conditions on learned Qwen latent-action tokens, not clamped robot actions, and returns only a
scalar loss. The pinned two-frame tubelet encoder makes even the earliest context depend on `t+1`.
No stock prediction position is admissible as row `t`. A new adapter must use only inputs available
by a target-specific prediction landmark that precedes target availability.

SLIM reports about 472 million trainable parameters. Its checkpoint file is about 945 MB. Its
paper-reported mean inference is about 60.6 ms, with 4.26 GiB incremental memory on an H100. These
are not M4 measurements.

SLIM uses PyTorch scaled dot-product attention and float32 sinusoidal positions in the reviewed
source. It has no evident complex-float RoPE blocker. Its server accepts a device argument, but
the project documents CUDA and has no MPS qualification.

The reviewed SLIM loader calls `torch.load(..., weights_only=False)`. Do not load an unverified
checkpoint through that path. Verify the exact digest and use a reviewed state-dict-only loader.

SLIM's repository license adds nonstandard wording to an MIT-like text. Its Hugging Face model
metadata uses `other`. Complete a rights review before redistribution or release evidence.

Efficient-WAM publishes two roughly 1.98 GB PyTorch checkpoints. The RT checkpoint has SHA-256
`209ab4d6f897276633e4d0f36e6b0c573c4938cf0df99d974b05e754ab92340f`. Its model repository has
no model card or declared weight license. The Apache-2.0 source license does not supply one.

The reviewed inference path defaults to CUDA. Its shared attention helper asserts that every query
is on CUDA before reaching its nominal scaled-dot-product fallback. It also contains CUDA autocast
sites and Wan-derived float64/complex RoPE. The loader deserializes the stage-three checkpoint with
plain `torch.load`. The complete runtime also requires UMT5-XXL and a Wan VAE. A credible MPS port
must make the attention fallback reachable, replace the RoPE path with real float32 math, make
autocast device-aware, use a reviewed state-dict-only loader, and measure full dependency memory.
The smaller headline checkpoint does not make the complete runtime low overhead.

LiLa-WAM reports 0.5B total parameters, with 0.2B trainable. The authors report 14.7–21.3 GB of
training memory across query counts. They report about 110 RTX 5090 GPU-hours for 50 RoboTwin
tasks and 85 ms inference on an RTX 4090. These are author results, not M4 measurements.

The reviewed ModelScope checkpoint is a 1,272,132,693-byte LFS object with SHA-256
`40e8aba09a6caeb6de1f532e55dd98a3ace0ba9363e87d52634c9ee9adeaeff9`. Its minimal model card
declares Apache-2.0. The GitHub source repository has no license file. Do not infer source-code
rights from the separate checkpoint metadata.

LiLa-WAM also depends on gated DINOv3 ViT-L/16 weights under the separate `dinov3-license`.
The source repository has no locked dependency manifest. Training selects CUDA or CPU and calls
`torch.amp.autocast("cuda")`. Inference defaults to CUDA and loads the checkpoint with
`torch.load(..., weights_only=False)`. MPS therefore requires code changes, a safe state-dict
loader, dependency closure, and CPU/MPS parity tests.

LiLa-WAM has no language input. Its Visual Transition Token is a fixed task vector computed from
demonstration endpoints. It cannot supply Prisoma's language axis or support language claims.
Its best local role is a compact predictive-loss ablation under a separately named contract.

Light-WAM's reviewed code is MIT-licensed. Its reviewed LIBERO-10 checkpoint is a
3,720,363,717-byte PyTorch file, but its Hugging Face repository has no model-card license. The
code defaults to CUDA or CPU rather than MPS. Its video-transformer RoPE path creates float64 and
complex tensors on the selected device. Treat MPS support as port work, not configuration work.
Review the pickle-based checkpoint path and artifact rights before loading or redistribution.

### 8.2 Qualification sequence

1. Pin code, weights, encoders, tokenizer, normalizer, environment, and planner.
2. Record the license for each artifact separately.
3. Install inference dependencies without CUDA or DeepSpeed.
4. Verify checkpoint hashes before reviewed weights-only deserialization.
5. Run one CPU batch with synthetic finite inputs.
6. Run the same batch on MPS.
7. Compare shapes, forecast losses, and candidate ordering within frozen tolerances.
8. Show action sensitivity above repeated-query drift on supported actions.
9. Verify each Prisoma hook's count, order, shape, dtype, and ancestry.
10. Run with network access disabled after artifact staging.
11. Measure cold start separately.
12. Measure at least 1,000 warm decisions, or justify a larger count for p99 precision.
13. Report p50, p95, and p99 with uncertainty and the complete capture-to-command tail.
14. Record peak unified memory, allocator state, thermal state, and a declared power proxy.
15. Reconstruct every CEM round, its final scored mean, and more than one closed-loop replan.
16. Fail closed on deadline miss or unexpected MPS-to-CPU fallback.

Until this sequence passes, use “MPS candidate.” Do not use “MPS supported.”

### 8.3 Local environment observation

Local hardware inspection measured 128 GiB of unified memory on a `Mac16,5` Apple M4 Max. The host
runs macOS 26.5.1.
The repository virtual environment uses arm64 Python 3.11.15. The host Python is arm64 3.14.6.

Neither original environment had PyTorch installed at the 2026-08-13 check. No model weights were
downloaded during that dated frontier scan.

A separate arm64 Python 3.11.13 environment was created on 2026-08-14. It used PyTorch 2.12.1,
the exact checkpoint, the pinned LeWM classes, and the locked 0.1.1 CEM solver. Synthetic CPU and
MPS prediction, rollout, and full-budget MPS CEM checks passed their narrow finite and repeatability
checks. This follow-up is not a paper reproduction, timing benchmark, end-to-end PushT run, hook
result, or model qualification. The companion compatibility report records its exact boundary.

The 2026-08-10 independent preprint is a stronger warning about evaluation identity than about
model quality. It uses one seed and only TwoRoom. Its reported 84%-to-8% matched change comes from
goal construction on the same 50 episodes and released checkpoint. Its three-checkpoint comparison
also separates one-step prediction quality from long-horizon control. Prisoma therefore keeps W1
forecast fidelity and W2 complete-policy value as separate primary claims.

### 8.4 Prisoma overhead budget

Keep production capture optional and bounded:

- disable every hook by default;
- copy only declared tensor sites;
- down-project offline when the estimand permits it;
- use bounded queues and explicit drop accounting;
- write content-addressed batches outside the action loop;
- run MI, PID, geometry, and attribution offline;
- record capture time and bytes per sample; and
- require no measurable action change in the paired noninterference preflight.

For predictive research on one M4, run the native exact-fork reference first. Then port the compact
LeWM PushT planner with its exact published CEM configuration. Benchmark it before freezing any
reduced-budget arm. Retain JEPA-WM as the second planning benchmark. Compare each model with
current-only, action-only cost, kinematic, repeated-query, shuffled-action, and proposal-headroom
controls. Do not begin with a 5B video generator.

MLX is a later optimization. No reviewed candidate has an official MLX implementation. Weight
conversion alone does not preserve masks, flow integration, normalizers, hooks, or numerical
semantics. PyTorch MPS is the lower-risk first target.

World-to-Wrist, WLA-0, LDA-1B, RepWAM, and Kairos do not change this order. World-to-Wrist is
about 4.97B parameters. WLA-0 activates about 2B parameters. LDA-1B also requires a 4B VLM and a
DINO encoder. RepWAM had no released inference code or weights at the cutoff. Kairos is a released
4B CUDA-oriented stack whose own report still calls closed-loop regret validation future work.

## 9. Evidence and artifact boundary

Social media can surface a claim. It cannot promote that claim.

For every model, keep separate rows for:

- paper availability and license;
- source-code revision and license;
- checkpoint revision and license;
- data rights;
- runnable dependency closure;
- hardware qualification;
- reproduced metrics; and
- external replication.

The absence of code blocks execution. The presence of code does not validate a paper result. An
official checkpoint does not grant data or weight redistribution rights unless its license says so.

## 10. Complete August exact-phrase disposition

This table covers every August result from the sorted arXiv API query for the exact phrase
`"world action model"` through the cutoff. “Screen only” means the paper did not change Prisoma's
current implementation order. It does not mean the work lacks scientific value.

| arXiv | Screened role | Prisoma disposition |
|---|---|---|
| [2608.11605](https://arxiv.org/abs/2608.11605) | ForeWAM one-pass hidden future-slot conditioning | class C; paper-only intended-future interface; no local artifact |
| [2608.11521](https://arxiv.org/abs/2608.11521) | Rift one-pass future-position cache and future-cache intervention study | class C; bounded path-use evidence; no local artifact |
| [2608.11204](https://arxiv.org/abs/2608.11204) | Surgical WAM joint future-action sampler | class J; frontier evidence; no local artifact |
| [2608.10860](https://arxiv.org/abs/2608.10860) | Flex-\(\pi\) intended-future conditioning | class C; full review in §3; placeholder repository |
| [2608.10780](https://arxiv.org/abs/2608.10780) | stage-level JEPA-WAM intended-stage guidance | class C; paper-only architecture evidence |
| [2608.10232](https://arxiv.org/abs/2608.10232) | FACT action-conditioned prediction and optional selection | class D/E; failure data and selector design evidence |
| [2608.10107](https://arxiv.org/abs/2608.10107) | 4D-consistent driving WAM | driving task mismatch; screen only |
| [2608.09771](https://arxiv.org/abs/2608.09771) | SLIM 0.5B predictive latent policy | smaller full-VLDA alternative after qualification |
| [2608.09730](https://arxiv.org/abs/2608.09730) | World Tokens training-time world adapter | class B; 2B policy plus about 0.5B adapter; no local artifact review |
| [2608.09516](https://arxiv.org/abs/2608.09516) | HarnessWAM task-manager wrapper | systems evidence for memory, verification, and recovery |
| [2608.09492](https://arxiv.org/abs/2608.09492) | TempoWAM progress-based execution wrapper | systems evidence for adaptive replanning |
| [2608.09381](https://arxiv.org/abs/2608.09381) | JEPA-WAM latent predictive training | class B; released CUDA artifact; later MPS port candidate |
| [2608.08839](https://arxiv.org/abs/2608.08839) | text-grounded semantic foresight | class C architecture evidence; screen only |
| [2608.08558](https://arxiv.org/abs/2608.08558) | Vid2WAM offline teacher distillation | class B; 5B student; not a low-overhead target |
| [2608.08023](https://arxiv.org/abs/2608.08023) | trajectory-field supervision for WAMs | training method; deployed class inherits the chosen backbone |
| [2608.07468](https://arxiv.org/abs/2608.07468) | SimWAM driving policy with training-time video | class B; driving task mismatch |
| [2608.07267](https://arxiv.org/abs/2608.07267) | WNM-3D joint navigation generation | navigation task mismatch; screen only |
| [2608.06994](https://arxiv.org/abs/2608.06994) | PILOT transition-token reasoning | representation evidence; no local artifact review |
| [2608.06375](https://arxiv.org/abs/2608.06375) | \(\omega\)-0 latent whole-body prediction | humanoid embodiment mismatch; compact-latent evidence |
| [2608.06008](https://arxiv.org/abs/2608.06008) | Adaptive-WAM multi-exit driving planner | driving task mismatch; early-exit scheduling evidence |
| [2608.05903](https://arxiv.org/abs/2608.05903) | Robust-WAM semantic foresight alignment | class B; visual-shift training evidence |
| [2608.04996](https://arxiv.org/abs/2608.04996) | DreamWAM structured future supervision | class B/J by mode; 5B backbone; not an M4 critical-path model |
| [2608.04657](https://arxiv.org/abs/2608.04657) | MobileWAM chain-of-foresight training | class B; mobile-manipulation evidence; code promised later |
| [2608.04404](https://arxiv.org/abs/2608.04404) | Faster-WAM retained future conditioning | class C; paper-only efficiency evidence |
| [2608.03701](https://arxiv.org/abs/2608.03701) | LiLa-WAM compact latent prediction | class B; no-language ablation after qualification |
| [2608.03682](https://arxiv.org/abs/2608.03682) | PhyAI shared inference runtime | runtime engineering, not a new policy graph |
| [2608.03244](https://arxiv.org/abs/2608.03244) | UniNav joint visual-waypoint diffusion | navigation task mismatch; screen only |
| [2608.02578](https://arxiv.org/abs/2608.02578) | CoWAM fixed-pool selective intervention | class E; contract and evaluation design evidence |
| [2608.02365](https://arxiv.org/abs/2608.02365) | Faster-WAM shallow action module | class B; paper-only low-compute evidence |
| [2608.01880](https://arxiv.org/abs/2608.01880) | asynchronous WAM execution study | scheduling evidence; reviewed in §5.1 |
| [2608.01397](https://arxiv.org/abs/2608.01397) | self-guided geometry-aware latent prediction | class B; 0.9B paper-only candidate |
| [2608.01221](https://arxiv.org/abs/2608.01221) | EndoWAM endoscopic navigation | clinical task and validation mismatch; screen only |
| [2608.00793](https://arxiv.org/abs/2608.00793) | DynamicWAM motion-conditioned joint generator | class J; dynamic-manipulation architecture evidence |
| [2608.00725](https://arxiv.org/abs/2608.00725) | SelfWAM self-grounded prediction | class B by default and D in optional rollout mode |
| [2608.00635](https://arxiv.org/abs/2608.00635) | FlowPilot depth-trajectory joint generator | class J; UAV task mismatch; onboard efficiency evidence |
| [2608.00547](https://arxiv.org/abs/2608.00547) | oracle visuo-tactile future-interface study | controlled representation study, not a learned deployable WAM |

The complete cohort reinforces one conclusion. The field is splitting prediction across training,
conditioning, joint generation, verification, and selection. It is not moving from one uniform
“VLA” class to one uniform “WAM” class.

## 11. Primary sources

- Flex-\(\pi\): https://arxiv.org/abs/2608.10860 and https://flex-pi.github.io/
- LeWorldModel: https://arxiv.org/abs/2603.19312 and https://github.com/Mengarr/lewm
- Independent LeWorldModel TwoRoom reproduction: https://arxiv.org/abs/2608.10145 and
  https://github.com/joyjeet-singh/tinylab
- stable-worldmodel platform: https://github.com/galilai-group/stable-worldmodel
- JEPA-WM physical planning: https://arxiv.org/abs/2512.24497 and https://github.com/facebookresearch/jepa-wms
- ForeWAM: https://arxiv.org/abs/2608.11605
- Rift: https://arxiv.org/abs/2608.11521
- SLIM: https://arxiv.org/abs/2608.09771 and https://github.com/kzz1031/SLIM
- World Tokens: https://arxiv.org/abs/2608.09730
- FACT: https://arxiv.org/abs/2608.10232 and https://github.com/Bariona/FACT
- CoWAM: https://arxiv.org/abs/2608.02578
- DynamicWAM: https://arxiv.org/abs/2608.00793
- FlowPilot: https://arxiv.org/abs/2608.00635
- SG-WAM, self-guided latent prediction: https://arxiv.org/abs/2608.01397
- SG-WAM, semantic guidance: https://arxiv.org/abs/2608.08839
- Vid2WAM: https://arxiv.org/abs/2608.08558
- DreamWAM: https://arxiv.org/abs/2608.04996 and https://github.com/hustvl/DreamWAM
- Robust-WAM: https://arxiv.org/abs/2608.05903
- MobileWAM: https://arxiv.org/abs/2608.04657
- TempoWAM: https://arxiv.org/abs/2608.09492
- JEPA-WAM: https://arxiv.org/abs/2608.09381 and https://github.com/SpriteWithoutIce/JEPA_WAM
- intended-stage JEPA-WAM: https://arxiv.org/abs/2608.10780
- SelfWAM: https://arxiv.org/abs/2608.00725
- Faster-WAM, shallow action module: https://arxiv.org/abs/2608.02365
- Faster-WAM, future conditioning: https://arxiv.org/abs/2608.04404
- VLA-JEPA: https://arxiv.org/abs/2602.10098
- DreamZero: https://arxiv.org/abs/2602.15922
- Fast-WAM: https://arxiv.org/abs/2603.16666
- UWM: https://arxiv.org/abs/2504.02792
- \(\tau0\)-WM: https://arxiv.org/abs/2606.01027
- MiraBench: https://arxiv.org/abs/2605.29360
- world-model hallucination audit: https://arxiv.org/abs/2606.27326
- world-model definition roadmap: https://arxiv.org/abs/2607.06401
- XEWorld: https://arxiv.org/abs/2608.05799
- PhyLatent: https://arxiv.org/abs/2608.05720
- PSG-JEPA: https://arxiv.org/abs/2608.06799
- HarnessWAM: https://arxiv.org/abs/2608.09516
- Dyna-2 report: https://www.dyna.co/dyna-2
- SmolVLA: https://arxiv.org/abs/2506.01844 and https://github.com/huggingface/lerobot
- Light-WAM: https://arxiv.org/abs/2606.08242 and https://github.com/L1ziang/Light-WAM
- LiLa-WAM: https://arxiv.org/abs/2608.03701 and https://github.com/teee000/LiLa-WAM
- Surgical WAM: https://arxiv.org/abs/2608.11204
- real-time WAM chunk execution: https://arxiv.org/abs/2608.01880
- World-to-Wrist VLA: https://arxiv.org/abs/2608.05369 and https://github.com/yyyyu120/W2-VLA
- Efficient-WAM: https://arxiv.org/abs/2606.10040 and https://github.com/jiajun613/Efficient-WAM
- LDA-1B: https://arxiv.org/abs/2602.12215 and https://github.com/jiangranlv/LDA-1B
- WLA-0: https://arxiv.org/abs/2606.05979 and https://github.com/SJTU-DENG-Lab/WLA
- RepWAM: https://arxiv.org/abs/2606.13674 and https://github.com/wdrink/RepWAM
- Kairos: https://arxiv.org/abs/2606.16533 and https://github.com/kairos-agi/kairos
- World Action Planner: https://arxiv.org/abs/2607.27599 and https://worldactionplanner.github.io/
- CheckVLA: https://arxiv.org/abs/2607.26789

These sources support a dated architecture review. They do not clear Prisoma's population,
measure, estimator, application, causal-identification, or external-replication gates.
