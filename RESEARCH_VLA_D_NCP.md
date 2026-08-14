# VLA and world-action models, the `D` axis, and the NCP sufficiency verdict

**Author:** Sepehr Mahmoudian · **Original memo:** 1 July 2026 · **Reconciled:** 13 August 2026

> **Repository facts synced 13 August 2026.** The observer pins immutable NCP `v0.8.0`, wire
> 0.8. Official NCP main was observed at `1a04294c90c1b50eba06ae1c6afe9c951319250d`.
> That commit is the incompatible, unreleased, release-blocked `1.0.0-rc.1` candidate on
> wire 1.0. Its compact proto contract hash is `163acc57d8a62b66`. NCP ledger tasks
> `P01`, `P02`, and `P03` are OPEN, not dependency-ready, and **NOT RUN**. `P03` covers
> fault-observatory migration and Prisoma observer-role qualification. `pid-rs` remains at the
> reviewed `796c11e` pin. Exp0 MI/coherence remains NO-GO.

> **Provenance and evidence limit.** The original workflow description reports an
> automated research run on 2026-07-01. It reports approximately 87 agent contexts,
> 10 search angles, and 16 of 16 selected claims surviving its internal adversarial
> vote.
>
> This repository retains the memo and its citations. It does not retain a
> machine-readable work ledger, per-claim ballots, prompts, transcripts, artifact
> hashes, or independent receipts for those counts. Treat the counts and the reported
> zero-refutation result as author-reported process notes. They are not reproducible
> evidence or independent review.
>
> Repository facts about Prisoma and NCP were grounded in the codebase. External
> VLA and PID claims carry inline `[n]` citations to the Sources list (§8). Verify
> specific model names and benchmark numbers against those sources before reuse.
> Section 7 separates external claims, repository facts, and author judgment.

## Bottom line (reconciled 13 August 2026)

The internal-dynamics, hidden-state, or world-model axis "D" is not depth. Several
cited 2026 policies use latent world models, and cited interpretability studies report
probeable or steerable structure in VLA hidden states [2][3][10][11][13][14][15].

VLAs are not dead. `VLA` usually names an input-output interface. `WAM` can name a
predictive training loss, a backbone, a generated-future path, an action-conditioned predictor,
or a planner. Many reviewed systems remain direct VLA policies at deployment [86]–[96][107]–[122].

Prisoma therefore classifies the deployed directed graph. It does not classify a system from its
paper title. Flex-\(\pi\), for example, generates an intended future that actions can read. Its
future cannot read candidate actions [86]. It is not an action-conditioned transition model.

The target must also stay outside every source's ancestry. A state computed from a candidate
action cannot be a PID source when that exact proposal is the target. That association is target
injection, not evidence that the state predicted or caused the decision. A downstream command,
later declared reference-state outcome, or separately measured physical outcome remains eligible
only when the matched baseline receives the same proposal. Command or simulator-state prediction
is not physical forecast validity.

ForeWAM and Rift use the same class-C direction with lower deployment overhead [123][124]. Each
creates action-independent future-position state in one prefill. Rift's paired cache interventions
show that the tested action path uses that state. They do not establish physical correctness or an
interventional transition.

World Action Planner is class E because it proposes, predicts, ranks, and selects candidates
[125]. CheckVLA is different. It predicts the execution of committed actions and can repair the
remaining action suffix [126]. This is a class-D verifier and repair wrapper, not candidate
planning.

The preserved Prisoma diagnostic family treats D as a source-agnostic input. It asks what D adds
about a named downstream variable. It also tests whether a declared predictor forecasts later
physical state. These are separate questions. PID is a conditional candidate for the first
question, not an assumed result [17][18]. W1-W3 are now the primary world-model claim family.

The NCP spiking-network bridge is an auxiliary D-source candidate. It is **not
sufficient today** for VLA studies or the (V,L,D,A) contract. All four review lenses
return **INSUFFICIENT**: scientific validity, estimator adequacy, engineering
completeness, and value relative to the SAFE reference adapter.

Some engineering and statistical gaps can be repaired. The scientific mismatch is
more structural. No real language stream, architecture-evidenced state selection, or
qualified world-model-bearing readout exists. NCP therefore remains exploratory,
fail-closed, and off the critical path. SAFE is the preserved diagnostic family's reference source
for real (V,L,D,A) data. It is not the W1-W3 path. Real capture and noninterference evidence remain
open.

---

## 1. What "D" means — and why it is not depth

In the Prisoma VLDA contract, V, L, and D are declared source axes, and A is the action target.
**D is not depth.** Its exact producer, timing, ancestry, and probe evidence govern every
semantic label. World-model, dynamics, internal-state, or planning semantics require separate
evidence. The contract enumerates three candidate forms (repo-internal): `D_explicit`,
`D_hidden[k]`, and `D_fused`. These names record the selected tensor role and provenance. They
do not establish natural policy use, a response to any untested intervention, or an internal
simulation.

A recent survey defines a **World Action Model (WAM)** through a predicted future that helps
produce, choose, or check an action [5]. This wide definition joins distinct deployed graphs.
Such a future is one candidate `D` source. It is not `D` by definition.

Prisoma tests whether a declared `D` source predicts a named later variable. A positive result
supports only that predictive statement. It does not establish natural use, intervention response,
causal dynamics, or planning.

## 2. VLA and D: state of the art through 13 August 2026

### 2.1 Deployed graph, not WAM branding

Let \(H\) contain all policy-visible history. Let \(F\) be a generated future. Let \(A^\pi\)
be the policy proposal. Let \(A^{exec}\) be the executed command.

Prisoma treats these as different operational and statistical contracts:

\[
q_{\mathrm{intent}}(F\mid H,L),\quad
q_{\mathrm{joint}}(F,A^\pi\mid H,L),\quad
q_{\mathrm{query}}(F\mid H,L,A^\pi),\quad
p(F\mid H,L,\operatorname{do}(A^{exec})).
\]

| Class | Deployed graph | Meaning |
|---|---|---|
| A | \(\pi(A^\pi\mid H,L)\) | direct policy |
| B | predictive training target; direct deployment | predictive co-training |
| C | \(q(F\mid H,L)\,\pi(A^\pi\mid H,L,F)\) | intended-future conditioning |
| J | jointly sample \(q(F,A^\pi\mid H,L)\), without a clamped-action query | coupled joint generation |
| D | \(q(F\mid H,L,A^\pi)\) | action-conditioned observational prediction |
| E | propose, predict, score, and select | predictive candidate planning |

Flex-\(\pi\) is class C in full mode [86]. DreamZero uses the same broad future-then-inverse
factorization [87]. The stage-level JEPA-WAM predicts an intended next-stage latent and uses it to
condition local video and action generation, so it is class C-like [92]. VLA-JEPA, Fast-WAM,
SLIM, LiLa-WAM, World Tokens, Dyna-2's scaling policy, and arXiv:2608.09381 JEPA-WAM are class B
at deployment [2][88]–[91][99][107]. Their predictive target can improve the policy without
becoming a runtime simulator. LiLa-WAM's released inference loop returns shared predictive-trained
tokens, but it ignores them and never calls the future decoder.

SelfWAM's optional rollout and UWM's forward mode expose class-D queries [93][94]. FACT is class D
in direct mode and class E when its optional candidate ranking runs [95]. \(\tau0\)-WM and CoWAM
are reviewed class-E systems [96][109]. CoWAM is the clearest selective-decision design in the
August cohort. It scores one fixed candidate pool, preserves or overrides the nominal action, and
abstains when its contracts do not admit a candidate. Neither system identifies an interventional
transition from architecture alone.

Surgical WAM, DynamicWAM, and FlowPilot are class J [108][110][111]. They jointly couple future
and action streams, but do not expose a clamped candidate-action forecast. DreamWAM is class B in
its no-rollout mode and class J in its joint mode [114]. A conditional factorization of a joint
density does not create a clamped-action operation.

SG-WAM, Vid2WAM, and MobileWAM remove their predictive teacher or branch at deployment
[112][113][116]. Robust-WAM removes its teacher and alignment head but keeps learned query tokens
[115]. These systems are class B. Their results test predictive training or distillation, not a
callable transition query. An exact-phrase arXiv screen found 36 August 2026 “world action model”
submissions through 13 August. Every item has a typed disposition in the dated frontier review.

A broader-name search reaches the same conclusion. World-to-Wrist keeps the VLA label while a
predicted wrist future conditions action generation, so it is class C [117]. LDA-1B is class B in
policy mode and exposes a separate class-D forward task [119]. WLA-0 is class B by default and
class E when its optional test-time path samples, predicts, scores, and selects candidates [120].
Efficient-WAM is class J in the released code. Its video and action tokens use bidirectional joint
attention, and no clamped candidate-action query is exposed [118]. Model branding does not select
the class.

ForeWAM and Rift extend the class-C branch with one-pass future-position state [123][124]. World
Action Planner is an explicit class-E design [125]. CheckVLA instead uses a class-D forecast to
verify committed actions and trigger a suffix repair [126]. These systems strengthen different
comparators. They do not collapse conditioning, transition identification, verification, and
planning into one claim.

A planner must compare at least two candidate actions. It must predict and score each candidate.
Its frozen score must cause the selection. It must log proposals, predictions, scores, and the
selected action. It must pass a decision-flip test with fixed proposals.

Action-conditioned causal language requires randomized executed actions, valid resets, action
support, proposal-to-execution consistency, proper scores, calibration, failure preservation, and
hidden-state-alias tests [97][98]. Generated pixel quality is not enough.

The phrase “VLAs are dead” is therefore not a testable architecture claim. The 2026 frontier is a
hybrid. Predictive objectives, state models, memory, and fast feedback increasingly augment the
vision-language-action interface.

A separate real-time WAM study compares six chunk-execution strategies on a 10 Hz bimanual
platform [106]. It reports that incorrect observation-to-command alignment causes boundary errors
that blending cannot repair. Its online comparison has three tasks and five trials per
method–task cell. The timing lesson is directly relevant. The reported method ordering is not a
universal deployment result.

The supplied X post is a discovery lead, not an independent source [100]. Dyna reports a
1.55-fold pooled success ratio over seven tasks and three checkpoints [99]. That internal
Dyna-2-versus-Dyna-1 comparison matches data, training hyperparameters, and three starting
checkpoints per architecture. The architectures still differ, and the ratio pools trials and
checkpoints. A separate same-Dyna-2 study compares action-only, joint, and video-co-trained
objectives under matched action data and steps. Dyna reports that joint training wins all 39 tasks
at each tested scale. This supports predictive co-training inside Dyna-2. It does not identify
online future simulation or planning, because the deployed action field does not consume the
predicted video field.

### 2.2 Latent world models are a prominent D candidate

A prominent 2026 candidate is a learned latent world model rather than a pixel reconstructor.
**VLA-JEPA** describes a JEPA-style latent world objective with a current-observation VLM path and
future latent targets [2][8]. Its paper calls this leakage-free because future frames do not enter
the VLM backbone. That is narrower than a row-level capture guarantee. The reviewed LeRobot
training implementation encodes one video window from `t` through `t+7`. It gives all but the last
encoded temporal position to a predictor and uses the shifted sequence as the target. The
predictor's so-called action inputs are learned Qwen latent-action tokens from the current
observation, not clamped robot actions. Its full output mixes temporal positions whose contexts
contain different video tubelets. The pinned encoder uses two-frame tubelets, so even the earliest
context position reads frames `t` and `t+1`. The method returns only the scalar alignment loss.
No stock prediction position is a row-`t` pre-action `D`. A future adapter must change the
capture or predictor path. Every input must exist by a target-specific prediction landmark that
precedes target availability.
Policy inference does not invoke that predictor. The paper reports the highest LIBERO average,
97.2%, against 97.1% for OpenVLA-OFT and 96.9% for pi0.5. It also reports leading results on five
of seven LIBERO-Plus perturbation dimensions and on SimplerEnv Google Robot. These are author
results. They support predictive training. They do not prove a deployed future state, natural use,
or a row-aligned Prisoma source.

All figures in this subsection are paper-reported and protocol-dependent. Prisoma did not
independently measure them.

**LaWAM** predicts compact latent visual subgoals in a frozen DINOv3 space. It infers latent
actions and reuses a forward decoder instead of reconstructing pixels or video. The paper reports
98.6% on LIBERO and 91.22% on RoboTwin at 187 ms per action chunk. It also reports up to 24-fold
lower latency than pixel-space WAMs. Its 2.3B policy contains a 230M-parameter LaWM [11].

**LiLa-WAM** uses a frozen DINOv3 encoder and a 0.5B bidirectional flow policy. The paper reports
single-24-GB-GPU training and an action-conditioned latent probe [107]. The released inference
loop consumes only the action velocity. It ignores the returned shared tokens and does not call
the future decoder. The source has no language input, no repository license, no locked dependency
manifest, and no MPS qualification. It is a low-overhead predictive ablation, not a full VLDA
source or deployed simulator.

**Efficient-WAM** releases a 1B class-J coupled sampler and two roughly 1.98 GB checkpoints [118].
The source is Apache-2.0, but the model repository has no card or declared weight license. The
runtime asserts CUDA before its nominal attention fallback. It also uses float64/complex RoPE,
plain `torch.load`, UMT5-XXL, and a Wan VAE. It is later Metal port work, not a low-overhead MPS
configuration.

**Surgical WAM** jointly samples future-video and action slots with a Cosmos Policy backbone
[108]. Its paper reports a matched action-free-video pretraining ablation on four simulated
surgical tasks. No official runnable code or checkpoint was verified by the review cutoff. It is
current architecture evidence, not an executable local candidate or independent result.

**AHEAD** adds a 4.9M-parameter motion-aware latent model to frozen 7B OpenVLA. It derives
per-token velocity and acceleration from optical flow. The paper reports 79–97% across 20 dynamic
simulation scenarios. Its best reported baseline range is 31–58%. On an xArm 7, it reports 29 or
30 successes on conveyor and rolling-ball tasks. It reports 19 of 30 on projectile catching, where
the listed baselines report zero [3].

**ALAM** learns algebraically consistent latent transitions from action-free video. Its declared
objective encourages composition and reversal consistency. The paper reports 25–85-fold lower
additivity or reversibility error. It also reports transfer gains from 47.9% to 85.0% on MetaWorld
MT50 and from 94.1% to 98.1% on LIBERO [10].

**World-Value-Action** gives a theoretical argument under its stated search model. Feasible
action-space trajectory probability can decay exponentially with horizon [6]. This result does not
prove that a latent `D` universally improves action-space planning.

The taxonomy work distinguishes render-and-decode, latent-only, and video-generation-free WAMs
[5]. A direct robustness study reports stronger results for several WAMs under its visual and
language perturbations. It reports 74.2% for LingBot-VA on RoboTwin 2.0-Plus and 82.2% for
Cosmos-Policy on LIBERO-Plus [9]. These are paper-reported results, not independent measurements.
A broader robot-world-model survey places the comparison in its architectural context [80].

At the frontier, **NVIDIA Cosmos 3** unifies language, image, video, audio, and action in one
mixture-of-transformers [4]. **GR00T N1.6** uses an internal Cosmos-2B VLM and a diffusion-
transformer action head [7][72]. Earlier V-JEPA, latent-action, and world-action work frames this
lineage [19]–[27].

### 2.3 Hidden states are probeable, value-like, and steerable

Another literature reads D from an existing policy instead of designing it.
**Frozen-VLA probing** uses linear probes on frozen OpenVLA, Pi0.5, DINOv2, and
CLIP features. The paper reports approximately 92–94% Pi0.5 pairwise ordering in
matched comparisons, versus approximately 50% for shuffled controls. The
paper-reported action-prefix selector result raises push-plate success from 26.7%
to 44.3% [14]. The paper-reported difference is 10.67 percentage points above
random and 17.67 points above greedy. The earlier 16.67-point value came from an
illustrative panel, not the formal table.

A mechanistic study covers six VLA models, four benchmarks, and more than 394,000
episodes. It reports that the visual pathway dominates action generation across
the tested architectures. Cross-task activation injection also steers X-VLA
toward source-task positions in 99.8% of reported episodes. In multi-pathway
models, expert pathways encode motor programs while VLM pathways encode goal
semantics [13].

TopK+AuxK sparse autoencoders on pi0.5 residual streams report approximately 79%
interpretable sampled features. The reported memorized-feature rates are 97.38%
for pi0.5-LIBERO and 89.19% for pi0.5-DROID. Closed-loop steering changes behavior
in the reported examples [15]. Event-grounded sparse autoencoders use behavioral
keyframes and report stronger OpenVLA interventions. Their usefulness still
depends on architecture and intervention site [16][31].

Related work covers emergent representations, symbolic-state probes, steering,
and VLA-Trace diagnostics [28][29][30][12]. Newer direct precedents remove objects from driving
scenes, compare internal responses, suppress visual shortcuts through a counterfactual branch,
and steer VLA activations in simulation and on a robot [82][83][84]. These studies motivate
Prisoma's physics-probe-then-gate rule. They do not make one intervention a universal test of
natural pathway use. A readout can carry decision-relevant structure and still reflect visual
entanglement or near-output action formatting.

New negative evidence makes forward prediction an inadequate admission test. XEWorld reports
that reviewed action-conditioned world models generalize by visual similarity more than physical
kinematic similarity on held-out embodiments [101]. PhyLatent reports three latent failures despite
global non-collapse: physical invariance, physical identifiability, and counterfactual dynamics
[102]. PSG-JEPA reports that forward prediction alone does not ensure that individual latents or
latent changes identify robot state [103]. These author results strengthen the need for external
physical-state, change, action-sensitivity, and held-out-embodiment tests.

HarnessWAM and TempoWAM also separate world prediction from execution management [104][105]. They
add task state, progress monitoring, event-triggered deliberation, recovery, or adaptive replanning
around finite-horizon policies. Prisoma should measure these external mechanisms separately. A
world branch does not supply them automatically.

### 2.4 Information-theoretic and PID analysis of multimodal internal state

A third, smaller literature applies PID directly. One study profiles 26 large vision–language
models across tasks, layers, and training stages [17]. Sensory PID conditions on language,
decomposes audio–video contributions, and adds modality-shuffling and instruction interventions
[85]. In LLMs, a reported synergistic core concentrates in middle layers [18]. These studies are
direct antecedents for Prisoma's layerwise `D_hidden[k]` screens. They do not validate Prisoma's
population, measure, estimator, or application regime. Complementary multimodal-interaction
methods map the surrounding design space [32][33][34][35][36].

### 2.5 PID estimator SOTA and its limits

Prisoma pins several non-substitutable objects in `pid-rs`. MGW categorical shared exclusions is a
paper-defined functional on categorical probability laws [127]. Prisoma's `categorical-sx` route
fits equal-width quantizers, forms empirical categorical laws, and evaluates averaged two-source
MGW components in nats. Ehrlich continuous shared exclusions is a distinct continuous formulation
with different support, gauge, and estimator assumptions [39]. No binning, limit, or cross-domain
equivalence is implied without a mapping theorem. KSG is a finite-sample mutual-information
estimator. An infomorphic objective is a coefficient-weighted composition of named PID atoms and
other terms, not another PID measure or estimator [128][129][130]. The current integer-count route
does not certify general soft weighted laws, adaptive bins, stopped binning gradients, or optimizer
guards.

The 2026 estimator frontier matters for what NCP could ever clear. A closed-form **Gaussian
multi-source PID** estimator gives explicit formulas without iterative optimization and extends
beyond two sources [1]. Its authors prove plug-in consistency and report controlled numerical
stability studies in finite-sample Gaussian regimes. Those results do not validate arbitrary VLA
or spiking data. The theory is simultaneously getting harder. A comprehensive review catalogs PID
properties and measures [37]. Multiple 2025–2026 results identify multivariate inconsistencies
[38][65][66]. Related MI advances bear on Prisoma's small-n, high-dimensional, autocorrelated
regimes [42][43][44][45]. Exp0 remains NO-GO on its synthetic high-dimensional controls. Any new D
source inherits that fragile regime.

### 2.6 Failure diagnosis from internal state, and neuromorphic control

A fourth literature reads internal features to *predict failure*. **SAFE** does multitask failure
detection for VLAs from internal features [46]. A wave of 2026 work extends this direction
[47][48][49][50][51][52][53][54]. SAFE is therefore the reference adapter for Prisoma's preserved
EC1/H diagnostic family. It is not the W1-W3 critical path. It can declare selected hidden states
as D, actions as A, and success or failure labels with explicit provenance. That mapping does not
establish dynamics, natural pathway use, or response to an intervention.

Finally, the **neuromorphic/SNN** literature is where NCP would have to land as a *legitimate* D source. There now exist spiking world models with multicompartment neurons for model-based RL [55], SNNs for continuous control via end-to-end model-based learning [56], neuromodulation-based spiking controllers [57], analog spiking arm control [58], spike-driven decision transformers [59], spiking diffusion policy [60], HPC-scale embodied SNN simulation [61], and methods bridging discrete spikes to continuous control (Proxy Target, surrogate-gradient representation analysis) [62][63]. PID studies report measure- and regime-specific redundancy or synergy estimates in biological, spiking, and oscillator systems [64][68][69][70][67]. Gaussian-PID bias-correction work also targets high-dimensional neural data [71]. These results motivate a study of a spiking D, but they do not establish a mechanism, a unique decomposition, or validity for this SNN.

## 3. Where Prisoma sits

Prisoma is not another VLA. It is a low-overhead, world-model-first experiment system for supported
decisions and linked closed-loop fidelity studies. W1-W3 are primary. The preserved EC1/H family
makes D a source-agnostic axis and requires architecture evidence before a stronger semantic role.
Its PID and mutual-information screens are conditional diagnostics, not headline outputs. A
separate physical forecast target asks whether a declared predictor forecasts later trajectory
quantities. It cannot prove natural pathway use.

The population, measure, estimator, and application gates govern every atom interpretation.
Geometry can support a warning or eligibility rule, but it cannot replace those gates. The current
high-dimensional MI/coherence path is NO-GO. Continuous atoms on real embeddings remain
application-blocked (repo-internal). EC1 and H1–H4 are unfrozen claim templates: finite registered
capture–replay fidelity and fault detection; H1-A frozen-snapshot response prediction or H1-B
randomized closed-loop effect modification; prospective H2 failure prediction; full-target H3 PID
incremental value with exact non-PID fallback; and H4 divergence between availability and the
cell-average response to one tested intervention. PID kill rules can retire H3 while H1 and H2
continue with PID disabled. Flow and attribution remain exploratory companions.

## 4. How NCP fits

The observer manifest pins the latest immutable NCP `v0.8.0` release, wire 0.8. NCP is an
external Zenoh pub/sub protocol. Its three data planes can expose a conforming sensorimotor
producer: perception (`SensorFrame`), action (`CommandFrame`), and neural observation
(`ObservationFrame`).

Official NCP main was observed at `1a04294c90c1b50eba06ae1c6afe9c951319250d` on
2026-08-13. That commit is the unreleased, release-blocked `1.0.0-rc.1` candidate. It uses
wire 1.0 and compact proto contract hash `163acc57d8a62b66`. Wire 1.0 is incompatible with
this observer.
NCP ledger tasks `P01`, `P02`, and `P03` are OPEN, not dependency-ready, and **NOT RUN**. `P03`
covers fault-observatory migration and Prisoma observer-role qualification. Refined low-overhead
architecture prose and the prepared-stream-monitor gap record are coordination-only. B01 remains
`IN_PROGRESS` with no passing receipt. See the
[verified NCP task ledger](https://github.com/sepahead/NCP/blob/1a04294c90c1b50eba06ae1c6afe9c951319250d/evidence/implementation/task-ledger.v1.json).

The intended future producer is **Engram**, described as a NEST spiking network. The named public
`sepahead/engram` repository remains a README-only placeholder. The executable Engram Neural Labs
host lives in `sepahead/Paper2Brain`. NCP's provider inventory records a preserved in-progress
Paper2Brain migration that targets candidate wire 1.0. It is not an installed or qualified
integration. Prisoma has a digest-locked, read-only headless-runtime descriptor for that host. Its
generic adapter reads only describe, session, and status. This live status path is not NCP, an
artifact validator, or a control path. The descriptor starts no process and grants no authority.
The review found that no compatible live wire-0.8 publisher exists in the public surface at the
2026-08-13 cutoff.

The `ncp-observer` crate is therefore a producer-agnostic **read-only passive tap**. It drives nothing; the Agent Bridge is the only control plane. It maps NCP onto (V,L,D,A): V ← `SensorFrame` channels minus language/success; L ← a named `SensorFrame` channel (default `instruction`); absent-language ticks are excluded from the artifact and counted (`excluded_empty_l`), never zeroed; **D ← `ObservationFrame` record-port readouts = pre-motor neural state** (world-model status untested); and A ← `CommandFrame` channels. Wire 0.8 correlates the planes with the full driving-sensor `StreamPosition`: a sensor contributes its own `stream`, while `CommandFrame.source` and a plane-published `ObservationFrame.source` echo the same `{epoch, seq}`. The observer never joins on arrival time or bare `seq`. Every kept sample carries `l_source = "channel"` and exact `d_source = "source"`; a source-less pull/RPC observation or missing readout is dropped or excludes the tick instead of being paired by recency.

Full validated-frame receipts remain bounded across epoch retirement. Exact post-emission redelivery is idempotent, and changed evidence invalidates the visible-receipt capture without patching the row or event. A publication receipt commits and hash-binds the artifact plus canonical log. Receipt schema 1 also binds the exact legacy tag, revision, wire, and compact hash. The harness rejects missing, failed, or different-wire NCP receipts. A deterministic 18-case fixture observatory exercises the shared route/raw-ingress seams and publishes replay/oracle evidence, but it reproduces a wholly missing tick as a manifest-only native blind spot. It does not establish receipt timing/QoS/reconnect, producer authentication, live noninterference, E4, EC1, security validation, or PID application validity (repo-internal).

The intended payoff is a research hypothesis, not a shipped control loop. The current observer flattens NCP channels into V/L/D/A axes, and the current harness runs axis-pair screens; it does not quantify or rank every sensor channel. A future, separately validated per-channel analysis could test whether gated information summaries help choose **design-time** codec priorities under a low-bandwidth link or compare simulator and neuromorphic information flow. Any adopted priority would be a versioned, human-reviewed NCP configuration; the observer would remain read-only and PID would never run per tick (repo-internal). This is a defensible candidate engineering study even if the world-model story never holds, and it is consistent with the neuromorphic spike-encoding / information-maximization literature [68][64]. Critically, NCP is **OPTIONAL, exploratory-only, off the critical path, and excluded from the default cargo workspace**; the grandplan does not depend on Engram, and root workspace resolution/build/test plus the static factual-outcome label-baseline smoke with PID disabled remain independent of NCP/Engram/Zenoh. That firebreak is dependency groundwork, not H1/H2 execution, and it does not turn the scientific PID gates green (repo-internal).

## 5. Is NCP sufficient?

### 5.1 Scientific validity — **INSUFFICIENT** (largely structural, not merely uncollected)

Is Engram a legitimate VLA analog, and is its pre-motor ObservationFrame readout a legitimate "D" comparable to a transformer VLA hidden state? The honest answer is no, today, and mostly *not by adding data*. Three disanalogies are structural (repo-internal). First, **no genuine live language stream has been demonstrated**: L is a named SensorFrame channel, and the observer excludes any tick where it is absent. That fail-closed behavior prevents fabrication, but it cannot make a non-language system support the I(V,L;A) decomposition. Second, the **"world model" label is unearned for a fixed pre-motor port**: grandplan §9.1–9.2 requires architecture mapping and external predictive evidence before assigning a scientific role. A pre-*motor* readout is exactly the near-output locus most at risk of action formatting rather than world content; physics decodability often peaks at intermediate depth and degrades toward output layers [13], and the cleanest D comes from models whose predicted latents are decision variables by construction [2][21], a guarantee an extracted SNN readout cannot offer. Third, the observer performs no architecture-evidence port selection — it concatenates configured record ports in deterministic BTreeMap order. An SNN may expose meaningful state families, but they require their own documented structure and probe/intervention evidence rather than being treated as a transformer layer stack (repo-internal). These are architectural properties, not mere data-collection gaps.

What *is* scientifically sound is Prisoma's **handling** of the weak analogy. Missing axes are excluded rather than fabricated, exact source correlation is mandatory, support declarations and computation outcomes are explicit, and `--require-axis-provenance-honest` is available. Geometry diagnostics can expose degeneracy, but they do not themselves grant scientific eligibility; population, measure, estimator, and application verdicts remain separate. The crate is workspace-excluded and is not eligible for a registered D2/EC1 evaluation. The primary W1/W2 program and preserved H1/H2 diagnostics can run with NCP absent and PID disabled (repo-internal). The integrity of the *program* holds; the *analogy* does not. Verdict on the literal lens claim: INSUFFICIENT.

### 5.2 Statistical / estimator adequacy — **INSUFFICIENT TODAY, FIXABLE in part**

No NCP data exists to test, so the question is answered structurally (repo-internal). As shaped today, an absent-L tick is excluded before artifact publication; a tick without an exact source-stamped D readout is excluded too. Missing `metadata.split`/`episode_id`/`success` means no leakage-resistant held-out evaluation and no majority, 1-NN, nearest-centroid, or logistic failure-label baseline can be computed under the strict modes. Even a structurally complete artifact would only make the baselines runnable: continuous KSG/shared-exclusions computation would still need declared support; fitted categorical MGW shared exclusions would remain a non-evidentiary diagnostic with population `NotEvaluated` and application `Blocked`; and interpretation would still require population, measure, estimator, and application gates. This is the honesty machinery working — the claim is not "NCP passes" but "NCP fails closed until its provenance and study structure are sufficient," which is the correct default.

There is one genuine potential advantage: **low dimensionality**. Exp0's NO-GO is driven by high-dimensional estimator incoherence, with historical intrinsic-dimension and MI-collapse diagnostics documenting the regime. A NEST closed loop with a handful of V channels, a few D ports, and low-dimensional A is *plausibly* easier, but low dimensionality alone passes none of the four gates. Three risks remain: (i) spike-count D is categorical/atomic by construction, so the current pid-rs support contract correctly rejects it for a declared continuous KSG/`I^sx` estimand rather than treating ties as a repairable nuisance; (ii) a steady-state SNN can produce a degenerate near-constant axis; and (iii) temporal autocorrelation reduces independent information even with exact wire-0.8 source joins. The descriptive within-unit-step-run Pearson lag-1 screen does not quantify the effective sample size of a nonlinear PID estimator. The retired recency fallback is no longer implemented. `MAX_INFLIGHT=4096` remains an ingress bound, not a sample count. A Gaussian-PID path for a separately justified approximately Gaussian continuous readout is an adoption recommendation, not current capability. Verdict: INSUFFICIENT today; transport alignment is repaired, while population/measure/estimator/application validity remains unresolved.

### 5.3 Engineering completeness — **INSUFFICIENT TODAY, FIXABLE**

The three documented NCP gaps before it can enter a registered D2/EC1 evaluation are all engineering, and all repairable in principle (repo-internal): (1) precise D source correlation — wire 0.8 defines `ObservationFrame.source` as the driving sensor's full `StreamPosition`, and the observer enforces/counts source absence on ingress; a conforming external plane publisher must actually stamp it; (2) honest L — a real language channel must be instrumented, while absent-L ticks remain excluded rather than backfilled; and (3) `metadata.split`/`episode_id`/`success` structure so held-out baselines and episode-disjoint splits exist. The passive-tap architecture, `{epoch, seq}` source-join discipline, DROP-QoS-aware pairing, bounded in-flight state, and provenance stamping are sound engineering. This lens is the most tractable, but a zero/hash language proxy is fabricated evidence, and adapter completeness cannot manufacture population/measure/estimator/application validity. Verdict: INSUFFICIENT today, fixable in-repo only after the external publisher supplies the required source and study channels.

### 5.4 Value versus the SAFE adapter — **SAFE is the preserved diagnostic reference**

SAFE rollouts can provide genuine VLA hidden states, actions, instructions, and outcomes.
`experiments/safe_adapter` implements the reference mapping with declared axis provenance. It is
not ready for confirmation and is not the W1-W3 real-data path.
Real capture and the diagnostic-noninterference preflight remain open. The H1/H2 protocol work
also remains open. SAFE provides the language and architecture structure required by §9.1. Its
literature also supports failure detection from internal state [46][47][48][49][50][51][52][53].

NCP is not a VLA substitute. Its narrower candidate roles are design-time codec prioritization
and an exploratory simulation-to-neuromorphic fidelity test [68][64]. Both roles remain off the
headline path. Verdict: SAFE has the required variable types in principle. Its implementation is
the reference adapter for this diagnostic comparison. NCP remains complementary and exploratory.

### 5.5 Integrated verdict

**NCP is not sufficient, today, for VLA studies or the (V,L,D,A)/VLDA PID contract.** The scientific lens is the binding constraint and is largely structural: no demonstrated language stream, no architecture-evidenced state selection, and a pre-motor D at high risk of action formatting rather than world-model content [13][2]. The statistical and engineering lenses are INSUFFICIENT-but-fixable, with two external/structural caveats (publisher-side `ObservationFrame.source` stamping and the impossibility of synthesizing a real L). Against SAFE, NCP loses as a source for the preserved VLDA diagnostic. It retains narrower candidate value for a future design-time codec study or a separately validated fidelity comparison. The correct posture — which the repo already enforces — is **exploratory, four-gate governed, non-headline**.

## 6. Recommendations

1. **Do not call Engram's pre-motor readout a "world model" or "internal simulation" in any result** until grandplan §9.1–9.2 architecture mapping and external predictive evidence support that role. Use "pre-motor neural state (world-model status untested)."
2. **Treat Engram as a neural-dynamics source, not a VLA analog.** Missing-L ticks are currently excluded. Drop the I(V,L;A) target for this source unless a genuine language channel is instrumented; do not accept a hash/zero proxy as L.
3. **Apply explicit architecture evidence before calling any port "D":** map record ports, probe preregistered physical/task variables on held-out data, and use frozen interventions where possible [82][84]; otherwise document D as configured pre-motor ports, not a principled world-model state.
4. **Resolve the estimator regime for spike data:** separate analog-rate ports from categorical spike-count ports and do not concatenate heterogeneous units into one continuous D. Preregister one measure, preprocessing, and estimator tuple. Never auto-route a failed continuous term to the fitted categorical MGW route or pool the two. Evaluate a Gaussian-PID path only for separately justified low-dimensional approximately Gaussian readouts [1][41][71], and use block/group resampling for autocorrelated sessions [44].
5. **Keep the honesty scaffolding load-bearing:** never let an Engram atom into a comparison table without its `l_source`/`d_source`, computation outcome, and population/measure/estimator/application verdicts attached; keep `--require-axis-provenance-honest` mandatory on Engram artifacts.
6. **If the preserved EC1/H branch is activated, use SAFE as its first real-VLA adapter.** Complete
   real capture, diagnostic-noninterference preflight, and protocol-specific structure. NCP remains
   an optional conformance item.
7. **Test NCP's candidate payoff rather than assuming it:** add a separate per-channel estimand and validated analysis before considering a human-reviewed static codec policy or sim-vs-neuromorphic fidelity comparison [68][64] — never add per-tick PID or an observer control path.
8. **Classify every WAM by its deployed graph.** Do not use a generated future, training loss, or
   paper name as evidence of action-conditioned dynamics or planning
   [86]–[96][108]–[116][123]–[126].
9. **Use the matched six-arm mechanism ladder.** Separate direct policy, predictive co-training,
   intended-future use, coupled joint generation, action-conditioned forecast validity, and
   score-based candidate selection.
   Match data, backbone, compute, controller, and evaluation. Keep prediction and scoring present
   but noncontrolling in arm 5. Enable selection only in arm 6.
   Never use an action-conditioned state as a PID source for that exact proposal target. Bind a
   target-specific prediction landmark before target availability. Bind each source's tensor
   ancestry to that landmark. For downstream commands or outcomes, give the matched baseline the
   same proposal. Use a separately measured physical outcome for physical forecast validation.
10. **Keep the M4 path low overhead.** Start with the native exact-fork reference. Port the compact
    LeWorldModel PushT planner next because it exposes a clamped candidate-action query and CEM
    selection loop in a smaller stack. Its upstream evaluator hard-codes CUDA, so require CPU/MPS
    parity, action sensitivity,
    multi-replan reconstruction, and measured resource receipts before support language. Treat
    the one-seed TwoRoom reproduction as a protocol-identity warning, not PushT evidence. Audit
    paper, configuration, and executable-code fields. Freeze each unresolved feasible reading
    before outcomes. Treat
    JEPA-WM as the second planning benchmark after a separate rights decision. Treat SmolVLA as
    the direct-policy MPS baseline. Treat VLA-JEPA as a predictive-training comparator.
    Do not use its stock world-loss output as a row-aligned `D`. Policy inference does not call its
    predictor. The world-loss path reads frames from `t` through `t+7`, mixes positions with
    different tubelet ancestry, and returns only a scalar loss. Its conditioning tokens are learned
    latent actions, not physical robot actions. Treat SLIM as a smaller alternative after loader
    and rights review. Treat Efficient-WAM as a later class-J code port. Test LiLa-WAM only as a
    separate no-language predictive ablation.
    Keep multi-billion-parameter video WAMs off the critical path [77][88][91][107][118].
    Measure the full observation-to-command delay distribution before adding asynchronous chunk
    execution [106]. Use one-pass class-C models only after their hidden-state path beats matched
    direct-policy and predictive-training controls [123][124].

## 7. Currency and confidence (reconciled 13 August 2026)

**Externally sourced facts.** All benchmark figures are paper-reported. Verify each cited source
before reuse. The review covers these areas:

- latent VLA and WAM results [2]–[11], [77], [86]–[99], and [101]–[126];
- deployed computation graphs and artifact status [86]–[99] and [101]–[126];
- action, representation, and execution-management warnings [97], [98], and [101]–[105];
- probing, counterfactual, mechanistic, and sparse-autoencoder results [13]–[16] and [82]–[84];
- PID and multimodal-information work [1], [17], [18], [37]–[45], [65], [66], [85], and
  [127]–[130];
- SAFE and failure-detection work [46]–[53]; and
- neuromorphic, spiking-control, and neural-PID work [55]–[71].

**Repo-internal facts** (authoritative for what Prisoma and NCP do, verified against the named code revisions, not independent scientific evidence): the VLDA contract and source-role rules; the pid-rs estimator core; the high-dimensional MI/coherence NO-GO and continuous-atom application block; the `ncp-observer` V/L/D/A mapping, provenance markers, full-`StreamPosition` source join, missing-L/source exclusion, bounded visible-receipt/conflict accounting, `MAX_INFLIGHT=4096` plus finite resident/output ceilings, and committed publication receipt; the explicit absence of own-stream gap/timing/QoS/authentication evidence; the three NCP gaps; NCP's optional-M2/off-critical-path/workspace-excluded status; the SAFE adapter's implemented contract mapping and remaining preflight/capture gaps; the current unfrozen EC1/H1–H4 claim-template registry; and the four PID gates.

**Author judgment** (my synthesis, not a citation): that `D` is one useful organizing axis,
but only after deployed-graph classification; that the NCP disanalogy is *binding and largely
structural* while some statistical and engineering deficits are *fixable*; the integrated
INSUFFICIENT verdict; the SAFE-versus-NCP diagnostic judgment; and the recommendation order.

**Could not be verified by 13 August 2026:** the physical semantics of Engram's configured record
ports, whether an architecture-evidence or physics probe was attempted upstream, whether the
current external publisher stamps wire-0.8 observation sources, or whether a genuine language
channel can be instrumented. No NCP data exists to run through the four gates. All statistical NCP
claims are therefore structural predictions, not measurements. Low dimensionality is plausible
but untested. No universal intrinsic-dimension cutoff establishes eligibility.

**Overall confidence:** medium-high on the external literature synthesis and on the integrated verdict; medium on the specific estimator-regime predictions for spike data (untested); lower on any counterfactual about a *future* NCP that instruments a real L and passes a physics probe.

## 8. Sources

[1] Closed-Form Gaussian Estimators for Multi-Source PID — https://arxiv.org/pdf/2605.09919
[2] VLA-JEPA: Enhancing VLA with Latent World Model — https://arxiv.org/abs/2602.10098
[3] Intercepting the Future: Latent-Space Predictive World Model for Dynamic VLA Manipulation (AHEAD) — https://arxiv.org/abs/2606.02486
[4] Cosmos 3: Omnimodal World Models for Physical AI — https://research.nvidia.com/labs/cosmos-lab/cosmos3/technical-report.pdf
[5] World Action Models: A Survey — Dream Less, Act More — https://arxiv.org/html/2606.20781
[6] World-Value-Action Model: Implicit Planning for VLA Systems — https://arxiv.org/abs/2604.14732
[7] GR00T N1.6: An Improved Open Foundation Model for Generalist Humanoid Robots — https://research.nvidia.com/labs/gear/gr00t-n1_6/
[8] VLA-JEPA (PDF) — https://arxiv.org/pdf/2602.10098
[9] Do World Action Models Generalize Better than VLAs? A Robustness Study — https://arxiv.org/abs/2603.22078
[10] ALAM: Algebraically Consistent Latent Action Model for VLAs — https://arxiv.org/pdf/2605.10819
[11] LaWAM: Latent World Action Models for Efficient Dynamics-Aware Robot Policies — https://arxiv.org/pdf/2606.15768
[12] VLA-Trace: Diagnosing VLAs through Representation and Behavior Tracing — https://arxiv.org/abs/2605.30117
[13] Not All Features Are Created Equal: A Mechanistic Study of VLA Models — https://arxiv.org/abs/2603.19233
[14] What Frozen VLAs Already Know About Success — https://arxiv.org/pdf/2605.28527
[15] Sparse Autoencoders Reveal Interpretable and Steerable Features in VLA Models — https://arxiv.org/html/2603.19183v1
[16] Event-Grounded Sparse Autoencoders for VLA Policies — https://arxiv.org/abs/2605.17204
[17] A Comprehensive Information-Decomposition Analysis of Large VLMs — https://arxiv.org/abs/2603.29676
[18] A Brain-like Synergistic Core in LLMs Drives Behaviour and Learning — https://arxiv.org/abs/2601.06851
[19] DreamVLA — https://arxiv.org/abs/2507.04447
[20] WorldVLA: Towards Autoregressive Action World Model — https://arxiv.org/abs/2506.21539
[21] V-JEPA 2 — https://arxiv.org/html/2506.09985v1
[22] GR00T N1.5 (FLARE) — https://research.nvidia.com/labs/gear/gr00t-n1_5/
[23] F1: A VLA Model Bridging Understanding and Generation to Actions — https://arxiv.org/pdf/2509.06951
[24] Latent Action Pretraining from Videos (LAPA) — https://arxiv.org/abs/2410.11758
[25] CLAM: Continuous Latent Action Models — https://arxiv.org/abs/2505.04999
[26] UniVLA: Learning to Act Anywhere with Task-centric Latent Actions — https://huggingface.co/papers/2505.06111
[27] Genie: Generative Interactive Environments — https://arxiv.org/html/2402.15391v1
[28] Emergent World Representations in OpenVLA — https://arxiv.org/abs/2509.24559
[29] Mechanistic Interpretability for Steering VLA Models — https://arxiv.org/abs/2509.00328
[30] Probing a VLA Model for Symbolic States and Integration into a Cognitive Architecture — https://arxiv.org/abs/2502.04558
[31] Event-Grounded Sparse Autoencoders for VLA Policies (PDF) — https://arxiv.org/pdf/2605.17204
[32] SynIB: Informational Bottleneck for Maximizing Synergy — https://arxiv.org/pdf/2606.09853
[33] Quantifying Modality Contributions via Disentangling Multimodal Representations — https://arxiv.org/abs/2511.19470
[34] Capability and Robustness Cannot Both Be Free: An Information-Theoretic Bound for VLAs — https://arxiv.org/pdf/2605.25889
[35] Investigating Redundancy in Multimodal LLMs (Conditional Utilization Rate) — https://arxiv.org/pdf/2507.03262
[36] Efficient Quantification of Multimodal Interaction at Sample Level — https://arxiv.org/pdf/2506.17248
[37] The Mathematical Landscape of Partial Information Decomposition — https://arxiv.org/abs/2603.06678
[38] Multivariate PID: Constructions, Inconsistencies, and Alternative Measures — https://arxiv.org/abs/2508.05530
[39] PID for Continuous Variables based on Shared Exclusions — https://arxiv.org/abs/2311.06373
[40] PID for Discrete Target and Continuous Source Random Variables — https://link.aps.org/doi/10.1103/58bg-5n9s
[41] Closed-Form Gaussian Estimators for Multi-Source PID — https://arxiv.org/abs/2605.09919
[42] Accurate Estimation of Mutual Information in High Dimensional Data — https://arxiv.org/abs/2506.00330
[43] MIST: Mutual Information Estimation via Supervised Training — https://arxiv.org/html/2511.18945
[44] Partial Information Rate Decomposition — https://arxiv.org/abs/2502.04550
[45] Mutual Information and Task-Relevant Latent Dimensionality — https://arxiv.org/html/2602.08105
[46] SAFE: Multitask Failure Detection for VLA Models — https://arxiv.org/abs/2506.09937
[47] Perturbation-Based Uncertainty for Failure Detection in VLAs — https://arxiv.org/pdf/2606.20754
[48] Failure Prediction at Runtime for Generative Robot Policies (FIPER) — https://arxiv.org/abs/2510.09459
[49] Hide-and-Seek in Trajectories: Discovering Failure Signals for VLA Runtime Monitoring — https://arxiv.org/pdf/2605.30834
[50] Uncertainty Quantification for Flow-Based VLA Models — https://arxiv.org/pdf/2606.18043
[51] Shifting Uncertainty to Critical Moments — https://arxiv.org/abs/2603.18342
[52] ActProbe: Action-Space Probe for Early Failure Detection — https://arxiv.org/pdf/2606.08508
[53] ReconVLA: Uncertainty-Guided and Failure-Aware VLA Framework — https://arxiv.org/pdf/2604.16677
[54] VLA Models: Concepts, Progress, Applications and Challenges — https://arxiv.org/pdf/2505.04769
[55] Spiking World Model with Multicompartment Neurons for Model-Based RL — https://www.pnas.org/doi/10.1073/pnas.2513319122
[56] Spiking Neural Networks for Continuous Control via End-to-End Model-Based Learning — https://arxiv.org/abs/2509.05356
[57] SpikeAEC: Neuromodulation-Based Spiking Controller — https://www.frontiersin.org/journals/neurorobotics/articles/10.3389/fnbot.2026.1757795/full
[58] Spiking Analog Hardware Trajectory Interpolation for Closed-Loop Arm Control — https://arxiv.org/html/2501.17172v1
[59] Spike-Driven Transformer for Decision Making — https://cvpr.thecvf.com/virtual/2025/poster/32864
[60] L-SDPPO: Spiking Diffusion Policy for Intra-vehicular Manipulation — https://arxiv.org/pdf/2606.06049
[61] Deploying Embodied Large-Scale SNNs on HPC Infrastructure — https://www.ncbi.nlm.nih.gov/pmc/articles/PMC9160925/
[62] Proxy Target: Bridging Discrete SNNs and Continuous Control — https://arxiv.org/pdf/2505.24161
[63] Uncovering the Representation of SNNs Trained with Surrogate Gradient — https://arxiv.org/pdf/2304.13098
[64] Specialized Structure of Neural Population Codes in Parietal Cortex Outputs — https://www.nature.com/articles/s41593-025-02095-x
[65] Novel Inconsistency Results for Partial Information Decomposition — https://arxiv.org/pdf/2512.16662
[66] The Whole Is Less than the Sum of Parts: Subsystem Inconsistency in PID — https://arxiv.org/pdf/2510.14864
[67] Time-Varying Synergy/Redundancy Dominance in the Human Cerebral Cortex — https://www.ncbi.nlm.nih.gov/pmc/articles/PMC12366633/
[68] Maximizing Information in Neuron Populations for Neuromorphic Spike Encoding — https://iopscience.iop.org/article/10.1088/2634-4386/ada8d4
[69] Redundant and Synergistic Interactions in Transistor Chaotic Oscillators and Neurophysiological Recordings — https://arxiv.org/html/2512.13570v1
[70] PID Reveals Synergistic Neural Integration Downstream of Recurrent Flow in Cortical Cultures — https://pmc.ncbi.nlm.nih.gov/articles/PMC8297941/
[71] Gaussian PID: Bias Correction and Application to High-dimensional Data — https://arxiv.org/pdf/2307.10515
[72] Building Generalist Humanoid Capabilities with NVIDIA Isaac GR00T N1.6 — https://developer.nvidia.com/blog/building-generalist-humanoid-capabilities-with-nvidia-isaac-gr00t-n1-6-using-a-sim-to-real-workflow/
[73] pi0.5: A VLA Model with Open-World Generalization — https://arxiv.org/abs/2504.16054
[74] Gemini Robotics 1.5 — https://arxiv.org/pdf/2510.03342
[75] Helix: A VLA Model for Generalist Humanoid Control — https://www.figure.ai/news/helix
[76] Fine-Tuning VLA Models: Optimizing Speed and Success — https://arxiv.org/abs/2502.19645
[77] SmolVLA — https://arxiv.org/pdf/2506.01844
[80] World Model for Robot Learning: A Comprehensive Survey — https://arxiv.org/html/2605.00080
[82] What Do They See? Interpreting Complex Road Scenarios Through the Eyes of Vision-Language-Action Models for Safe and Trustworthy Autonomous Vehicle Learning — https://arxiv.org/abs/2607.16938
[83] CofactVLA: Deconfounding Vision-Language-Action Models via Counterfactual Intervention — https://arxiv.org/abs/2608.04396
[84] Mechanistic Interpretability for Steering Vision-Language-Action Models — https://proceedings.mlr.press/v305/haon25a.html
[85] Towards Understanding Modality Interaction in Multimodal Language Models via Partial Information Decomposition — https://arxiv.org/abs/2606.00959
[86] Flex-\(\pi\): A Multi-Stream World-Action Model with Compute Flexibility — https://arxiv.org/abs/2608.10860 and https://flex-pi.github.io/
[87] DreamZero: Learning Robot Control from Video Generation — https://arxiv.org/abs/2602.15922
[88] SLIM-0.5B: Learning Action-Grounded Predictive Latents for Robot Manipulation — https://arxiv.org/abs/2608.09771 and https://github.com/kzz1031/SLIM
[89] World Tokens: Enhancing Embodied Policies with Training-Time World Modeling — https://arxiv.org/abs/2608.09730
[90] Fast-WAM: Do World Action Models Need Test-time Future Imagination? — https://arxiv.org/abs/2603.16666
[91] JEPA-WAM: Learning VLA Policies with Joint-Embedding World Modeling — https://arxiv.org/abs/2608.09381
[92] JEPA-WAM: Stage-Level Joint-Embedding Prediction for World-Action Models — https://arxiv.org/abs/2608.10780
[93] SelfWAM: A Self-Grounded Unified World Action Model for Fast Robot Control — https://arxiv.org/abs/2608.00725
[94] Unified World Models: Coupling Video and Action Diffusion — https://arxiv.org/abs/2504.02792
[95] FACT: Failure-Aware Causal Training for World-Action Models — https://arxiv.org/abs/2608.10232 and https://fact-wam.github.io/
[96] \(\tau0\)-WM: A Unified Video-Action World Model for Robotic Manipulation — https://arxiv.org/abs/2606.01027
[97] MiraBench: Evaluating Action-Conditioned Reliability in Robotic World Models — https://arxiv.org/abs/2605.29360
[98] Hallucination in World Models Is Predictable and Preventable — https://arxiv.org/abs/2606.27326
[99] Dyna-2 technical report — https://www.dyna.co/dyna-2
[100] Supplied X discovery lead for Dyna-2 — https://x.com/CyberRobooo/status/2086999365789601933
[101] XEWorld: Can Action-Conditioned World Models Generalize to Unseen Robot Embodiments? — https://arxiv.org/abs/2608.05799
[102] PhyLatent: Learning Dynamics-Relevant Representations for JEPA World Models — https://arxiv.org/abs/2608.05720
[103] Is Forward Prediction Enough? Physical State Grounding for JEPA World Models — https://arxiv.org/abs/2608.06799
[104] HarnessWAM: Bridging Prediction and Deliberation in World Action Models — https://arxiv.org/abs/2608.09516
[105] Rethink Before You Execute: Adaptive Execution for World Action Models (TempoWAM) — https://arxiv.org/abs/2608.09492
[106] World Action Models in Real Time: An Empirical Study of Smooth Execution via Asynchronous Deployment — https://arxiv.org/abs/2608.01880
[107] LiLa-WAM: Lightweight Latent Reasoning World-Action Model for Robotic Manipulation — https://arxiv.org/abs/2608.03701 and https://github.com/teee000/LiLa-WAM
[108] Surgical WAM: A World-Action Model for Data-Efficient Surgical Robot Learning — https://arxiv.org/abs/2608.11204
[109] CoWAM: Coordination Contracts for Selective Policy Intervention with WAMs — https://arxiv.org/abs/2608.02578
[110] DynamicWAM: Dual-Path Motion Conditioning for World-Action Models in Dynamic Manipulation — https://arxiv.org/abs/2608.00793
[111] FlowPilot: Real-Time World-Action Modeling for Agile UAV Navigation — https://arxiv.org/abs/2608.00635
[112] SG-WAM: Self-Guided World Modeling in Geometry-Aware Policy Space — https://arxiv.org/abs/2608.01397
[113] Vid2WAM: Distilling Video Diffusion Priors into World Action Models — https://arxiv.org/abs/2608.08558
[114] DreamWAM: Beyond RGB Future Prediction for World Action Models — https://arxiv.org/abs/2608.04996 and https://github.com/hustvl/DreamWAM
[115] Robust-WAM: Bridging Generative Pretraining and Semantic Foresight in World-Action Models — https://arxiv.org/abs/2608.05903
[116] MobileWAM: Bridging World Action Models to Mobile Manipulation with Chain-of-Foresight — https://arxiv.org/abs/2608.04657
[117] World-to-Wrist: Task-Conditioned Future Wrist Modeling for Fine-Grained Robot Manipulation — https://arxiv.org/abs/2608.05369 and https://github.com/yyyyu120/W2-VLA
[118] Efficient-WAM: A 1B-Parameter World-Action Model with Low-Cost Future Imagination — https://arxiv.org/abs/2606.10040 and https://github.com/jiajun613/Efficient-WAM
[119] LDA-1B: Scaling Latent Dynamics Action Model via Universal Embodied Data Ingestion — https://arxiv.org/abs/2602.12215 and https://github.com/jiangranlv/LDA-1B
[120] World-Language-Action Model for Unified World Modeling, Language Reasoning, and Action Synthesis — https://arxiv.org/abs/2606.05979 and https://github.com/SJTU-DENG-Lab/WLA
[121] RepWAM: World Action Modeling with Representation Visual-Action Tokenizers — https://arxiv.org/abs/2606.13674 and https://github.com/wdrink/RepWAM
[122] Kairos: A Regret-Aware Native World-Action Model Stack for Physical AI — https://arxiv.org/abs/2606.16533 and https://github.com/kairos-agi/kairos
[123] Foresight Without Seeing: Latent Futures for World Action Models (ForeWAM) — https://arxiv.org/abs/2608.11605
[124] Keep the Future, Drop the Rollout: RIFT for World Action Models — https://arxiv.org/abs/2608.11521
[125] World Action Planner: Generalizable Decision-Making with Action-Conditioned World Models — https://arxiv.org/abs/2607.27599 and https://worldactionplanner.github.io/
[126] CheckVLA: Execution-Time Verification with Action-Conditioned World Model for Long-Horizon Mobile Manipulation — https://arxiv.org/abs/2607.26789
[127] Makkeh, Gutknecht, and Wibral, Introducing a differentiable measure of pointwise shared information — https://arxiv.org/abs/2002.03356
[128] Wibral et al., Partial information decomposition as a unified approach to the specification of neural goal functions — https://pubmed.ncbi.nlm.nih.gov/26475739/
[129] Makkeh et al., A generalized framework for infomorphic neural networks — https://pmc.ncbi.nlm.nih.gov/articles/PMC11912414/
[130] Infomorphic neural networks: learning efficiently with modularity and explicit control over embodied information — https://proceedings.iclr.cc/paper_files/paper/2025/hash/87d8ed41d250c401a68f05100e0a4ef0-Abstract-Conference.html
The complete architecture and M4 artifact audit is
[`WORLD_ACTION_MODEL_FRONTIER.md`](docs/audits/2026-08-12-first-principles/WORLD_ACTION_MODEL_FRONTIER.md).
