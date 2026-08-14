# Prisoma first-principles audit ledger, 2026-08-12

This file is durable audit memory. It records scope, evidence, decisions, and remaining gates for
the review opened on 2026-08-12 and refreshed through 2026-08-13. It is not a preregistration, a
scientific result, or a systematic review.

## Audit identity

- Starting Prisoma revision: `6d6f895d57ec38feb417a6027cab8dcdf525ce2a`.
- Starting state: clean `main`, equal to `origin/main`.
- Starting `pid-rs` pin: `796c11e70f009634b853dc4ada6f565563d82f51`.
- Current public `pid-rs` main observed during review:
  `7473e62acef6077c2c1147e09d5d1297f2a2874b`.
- Canonical document on entry: `grandplan.md`, docset v12.5.
- Canonical document after reconciliation: `grandplan.md`, docset v13.0.
- Primary new family: W1 supported forecast fidelity with secondary ranking, W2 complete-policy selection, and W3 linked
  fidelity tomography. All remain unfrozen and have no empirical result.
- Review window: 2026-08-12 through 2026-08-13.
- Holdout and real-study access: not authorized and not performed.
- Submodule update: not performed.

## Inventory

The starting tree had 254 tracked entries outside the submodule boundary:

- 47 Markdown files;
- 42 Rust files;
- 52 Python files;
- 58 JSON files.

The inventory includes generated, archived, historical, and immutable review files. Those classes
have different edit rules.

| Class | Audit treatment |
|---|---|
| Current project prose | Read against `grandplan.md`, code, schemas, and current sources. Reconcile when false or ambiguous. |
| Generated truth | Edit its source only. Regenerate with the repository command after source freeze. |
| Historical or archived material | Preserve historical statements. Require an unambiguous historical label. |
| Immutable release intake | Preserve bytes. Do not rewrite it to imply progress. |
| `pid-rs/` submodule | Inspect the pin and upstream delta. Do not edit it in this repository. |

## The 500-view audit

The audit uses 500 distinct claim-question-assurance views. This is a structured matrix, not a
claim that 500 independent reviewers examined the project.

The entry audit crossed five preserved claim families: EC1, H1, H2, H3, and H4. H1 must answer every relevant view separately
for H1-A and H1-B. Each family is crossed with ten inference-chain questions and ten assurance
lenses. Thus, `5 × 10 × 10 = 500` views.

The v13.0 reconciliation applies the same questions and lenses to W1, W2, and W3. It does not
relabel the original 500-view matrix or claim extra reviewers.

`grandplan.md` §13 separately defines 50 concrete adversarial design lenses. Those lenses
operationalize this audit's ten assurance categories. They do not inflate the 500-view count or
imply independent reviewers.

### Ten inference-chain questions

1. What exact scientific question is asked?
2. What population and sampling law define the target?
3. What exposure, intervention, prediction time, or policy is evaluated?
4. What data are observable, and what is the independent inference unit?
5. What estimand is identified under which assumptions?
6. What estimator and uncertainty procedure target that estimand?
7. What comparator and minimum useful effect define practical success?
8. What falsifier, warning, abstention, and stop rule applies?
9. What conclusion is permitted if the test passes or fails?
10. What implementation state and evidence grade support the claim?

### Ten assurance lenses

1. construct and scientific meaning;
2. causal identification and interference;
3. statistical validity and multiplicity;
4. measurement, ontology, and missingness;
5. estimator and numerical validity;
6. time, leakage, transport, and shift;
7. systems, security, and resource bounds;
8. provenance, replay, and reproducibility;
9. deployment, cost, latency, and human actionability;
10. governance, falsification, claim language, and evidence promotion.

A fixture can answer an implementation question for the named bytes. It cannot answer a population,
identification, application-validity, or external-validity question.

### Factorized disposition of the 500 views

The table records each claim-question result after all ten assurance lenses were applied. The
codes are `D` for defined, `S` for a software primitive, `O` for open or unfrozen, and `B` for
blocked or not eligible. A combined code preserves both facts. H1-A and H1-B remain one registered
claim family. They need separate dispositions because they have different estimands.

| Claim | Q1 | Q2 | Q3 | Q4 | Q5 | Q6 | Q7 | Q8 | Q9 | Q10 |
|---|---|---|---|---|---|---|---|---|---|---|
| EC1 | D | O | D | O | D | S | O | D | D | S |
| H1-A | D | O | D | S+O | D | S | O | D | D | S |
| H1-B | D | O | O | O | D+O | B | O | D | D | B |
| H2 | D | O | D | O | D+O | S | O | D | D | S |
| H3 | D | O | D | O | D | B | O | D | D | B |
| H4 | D | O | O | O | D+O | B | O | D | D | B |

The assurance conclusions repeat across many cells. The audit stores them once rather than
inflating repeated text into 500 rows.

The additional v13 review records the primary family separately:

| Claim | Question | Current software | Scientific status |
|---|---|---|---|
| W1 | Supported reference-state forecast; same-pool ranking is secondary | Native same-law contract fixture | Open and unfrozen; no learned-model evidence |
| W2 | Complete-policy selection value under one M4 budget | Native multi-replan contract fixture | Open and unfrozen; no randomized policy result |
| W3 | Linked dynamics, renderer, policy, controller, and selector fidelity | Protocol specification | Open and unfrozen; no matched mesh/3DGS panel |

| Assurance lens | Cross-claim disposition |
|---|---|
| Construct | Each claim now names its construct. Real populations, margins, and claim selections remain unfrozen. |
| Causal | Only a future randomized H1-B design can identify its named closed-loop contrast. No current artifact identifies natural pathway use. |
| Statistical | Score, hierarchy, multiplicity, and uncertainty rules exist. Power inputs and final decisions remain open. |
| Measurement | Event and label contracts are typed. Rights-approved real capture and complete outcome ontology remain open. |
| Estimation | Synthetic and low-dimensional software checks exist. High-dimensional MI is NO-GO and continuous PID application use is BLOCKED. |
| Time and transport | Leakage, source–target ancestry, and split rules are explicit. Each target needs a pre-availability landmark. A source conditioned on a proposal cannot target that proposal. Downstream targets require matched proposal access. Command prediction is not physical forecast validity. No external or later-time replication exists. |
| Systems | Inputs and work are bounded. A run log cannot prove an upstream event that capture did not observe. |
| Provenance | Content binding and replay exist for named artifacts. They are not process, identity, build, or remote attestation. |
| Deployment | Costs and fallbacks are part of H3. No WAM or MPS candidate is locally qualified. |
| Governance | The active v3 draft exposes missing choices and blocks H3 freeze on unimplemented ancestry roles. It remains unreviewed, unfrozen, and non-promotable. The superseded v2 bytes remain historical. |

## Claim decisions

| Family | Final audit status | Governing correction |
|---|---|---|
| W1 | Same-law software reference only | Qualify a learned action-conditioned model, supported actions, proper forecast score, and same-pool ranking contract. |
| W2 | Software decision contract only | Compare complete randomized policies under one frozen M4 resource budget. |
| W3 | Specified only | Use linked matched panels. Do not claim a full factorial or global priority. |
| EC1 | Software groundwork only | Define a finite registered acceptance universe. Do not claim universal provenance completeness. |
| H1-A | Synthetic scoring reference only | Freeze one positive-margin, one-sided primary response-prediction contract. This is not a physical effect claim. |
| H1-B | Unimplemented | Freeze one positive-margin, one-sided effect-specific endpoint plus every mandatory design check. Randomization does not establish unrestricted natural pathway use. |
| H2 | Synthetic arithmetic reference only | Freeze one prediction object, score, risk, censoring law, assumptions, and uncertainty contract. |
| H3 | Not eligible | Evaluate the full deployed eligibility-warning-abstention-fallback policy, not only cases where PID returns a number. |
| H4 | Exploratory reference only | Preselect the alternative and companion branch. Do not switch to it after inspecting H3 on the same holdout. |

No scientific PID gate opened during this audit.

## Material first-principles corrections

### Identification and claim language

- Replaced unrestricted natural “use” claims with exact tested-intervention or prediction claims.
- Split H1-A and H1-B by unit, estimand, observation, endpoint, and permitted conclusion.
- Replaced opaque H1 success bindings with typed positive-margin, one-sided contracts. Secondary
  endpoints, equivalence, noninferiority, and factual-outcome fit cannot rescue primary failure.
- Bounded EC1 to a frozen adapter, event, variable, fault, endpoint, and replay universe.
- Made H4 a preselected branch rather than an outcome-driven fallback.
- Required exact comparator implementations or a pre-outcome analogue-selection rule.

### H2 prediction law

An intermediate audit draft overcorrected H2. It said that earlier censoring always requires a full
event-time-and-type forecast. A direct reread of Rindt et al. and Jonkers et al. disproved that
statement. A forecast-independent conditional IPCW Brier construction can properly score scalar
horizon risk on its identifiable region under correct censoring and positivity assumptions.

The final contract keeps three distinct objects separate:

- a complete-follow-up proper score;
- a censoring-adjusted horizon score or complete-data risk estimator under explicit assumptions;
- a proper score for a fuller event-time-and-type prediction object.

The score name alone does not select among those roles. A right-censored likelihood requires the
full event-time-and-type law. A horizon-specific adjusted score can target scalar named-cause risk.
The event ontology must still include every competing event that changes the target.

### H3 deployment estimand

H3 now targets the full frozen population value of one deployed policy. The policy includes:

- eligibility;
- PID measure and preprocessing;
- estimator and diagnostics;
- warnings and abstention;
- exact non-PID fallback;
- compute and latency effects.

Eligible-only performance is descriptive. It cannot establish the confirmatory H3 claim.

### PID scientific-object firewall

Do not emit a `wibral_lineage` result identity. Use a provenance and estimand graph. Each artifact
must declare its object kind, domain, defining reference, functional, evaluation kind, input-law
kind, implementation, preprocessing, units, aggregation scope, and composition.

MGW categorical shared exclusions is a functional on categorical probability laws. Ehrlich
continuous shared exclusions is a distinct formulation with different support, gauge, and
estimator assumptions. A finite-sample plug-in or kNN algorithm is an estimator, not the
functional. Direct evaluation of a specified law is not a sample estimator. An infomorphic goal is
a weighted composition of named atoms and other terms, not a new PID measure. Williams-Beer
`I_min` and BROJA are separate measures and have no active Prisoma hypothesis. No result transfers
across these objects without an explicit mapping theorem.

### NCP observer semantics

The NCP review found three mapping assumptions that were unsafe to leave implicit:

- the sample clock must come from the driving sensor frame, not a command-frame substitute;
- a configured success channel must contain exactly one binary scalar (`0` or `1`);
- the success channel must not alias the language channel.

The observer now rejects invalid values before state mutation. Zero maps to false. One maps to
true. Other finite values are invalid rather than silently recoded. Mapping validation applies to
every capture ingress. Each sample and capture event preserves raw converted sensor time separately
from the nondecreasing event clock.
These are software-contract fixes, not H1-A, H1-B, or H2 evidence.

### Low overhead

"Lean" now means low runtime and operator overhead. The preferred architecture uses small typed
contracts, bounded readers, exact ledgers, explicit abstention, and existing generators. It does
not add a service, database, control plane, or viewer when a file or local process suffices. Model
qualification also measures end-to-end capture-to-command latency and tails, peak unified memory,
model calls, artifact staging, fallback cost, and controller timing. Parameter count alone is not
an overhead metric.

### Temporal and uncertainty assumptions

The line-level implementation pass found two inferential ambiguities that broader gate runs had
not exposed.

- Missing `episode_id` values do not identify one time series. The harness reports only descriptive
  within-segment lag pairs. Rows without episode identities now produce none. The report emits no
  derived sample-size or block-length heuristic. Each non-singleton segment also needs a strict
  canonical sequence-index receipt. Only unit index advances contribute. The report counts gaps.
- The lag statistic is a within-unit-step-run Pearson correlation. Axis summaries exclude columns
  that are undefined after centering. Each axis records defined and total column counts. An
  all-undefined axis emits no lag-1 value. Each run needs at least three lag pairs because two
  pairs force Pearson correlation to positive or negative one.
- The current report schema 5 retains a private process-local mutation seal over every serialized
  field.
  Publication verifies the seal. A deserialized report is read-only evidence, and the seal is not
  a signature or process attestation.
- One uncertainty request cannot assert incompatible row laws. A unit-block bootstrap may pair
  with full shuffle. A multi-row block bootstrap may pair with a dependence-preserving circular
  surrogate only with the same sequence-index receipt. A block-shuffle request must use the same
  block size as the bootstrap. The library and CLI reject inconsistent combinations before main
  analysis.

These changes do not validate either resampling law. They make the caller's assumptions coherent
and machine-visible.

### H1 scaled-response arithmetic

The L2 response path used a direct sum of squares. Large finite deltas could overflow that
intermediate even when the true norm was finite. The implementation now uses scaled `hypot`
accumulation and rejects a non-finite derived norm. Tests cover both sides of this boundary. This
is a numerical software correction, not H1-A evidence.

### Replay-summary pipeline robustness

The integration pass reproduced a pinned `pid-runlog-replay` CLI failure that unit tests did not
expose. An early-closing stdout consumer such as `grep -q` can cause a `BrokenPipe` panic and exit
101 while the CLI prints a long replay summary. Prisoma's recipes and CI now use literal matches
that drain the stream. The repository-truth audit rejects an early-closing replay pipeline. The
`pid-rs` handoff asks upstream to handle a closed stdout pipe without a panic.

### World-action-model taxonomy

The audit rejects the binary “VLA versus WAM” frame. It records six deployed classes: direct
policy, predictive co-training, intended-future conditioning, coupled joint generation,
action-conditioned prediction, and candidate planning. A joint density does not expose a clamped
action query by algebraic factorization alone.

Flex-\(\pi\)'s future stream cannot attend actions. It is an intended-future policy, not an
action-conditioned transition model. Dyna-2's deployed action field does not consume predicted
video. Its reported comparison supports predictive co-training, not VLA obsolescence.

The broader-name search finds the same graph classes under different labels. World-to-Wrist keeps
the `VLA` label while consuming a predicted wrist future. WLA-0 is class B by default and class E
in optional candidate-selection mode. LDA-1B is class B in policy mode and exposes a separate
class-D forward task. Efficient-WAM is class J. Its released code gives video and action tokens
bidirectional joint attention. It does not expose a clamped candidate-action forecast.

An exact-phrase arXiv query found 36 August 2026 “world action model” submissions through
13 August. Every item has a typed disposition in `WORLD_ACTION_MODEL_FRONTIER.md`. CoWAM is the
strongest new class-E contract because it scores a fixed candidate pool and can preserve,
override, or abstain. Its forecasts remain observational. DynamicWAM, FlowPilot, and one DreamWAM
mode are class J. SG-WAM, Vid2WAM, and MobileWAM remove predictive branches at deployment.
Robust-WAM keeps learned query tokens but removes its teacher and alignment head. These four are
class B because none exposes a callable transition query.

The final feed refresh added ForeWAM and Rift. Both write action-independent future-position state
in one prefill and are class C. Rift also tests future-cache use through paired closed-loop
interventions. World Action Planner is an explicit class-E planner. CheckVLA is a class-D
execution verifier with a repair loop. None establishes an interventional transition law.

The local decision keeps full video WAMs off the M4 critical path. Prisoma runs its native contract
first. The pinned LeWorldModel PushT planner is the first external world-model port because it
exposes a candidate-action query and CEM selection loop in a smaller stack. Its upstream evaluator
hard-codes CUDA, so MPS remains unqualified. A one-seed independent LeWM reproduction covers only
TwoRoom. It adds a protocol-concordance obligation and no PushT, M4, W1, or W2 evidence. JEPA-WM
is the second planning benchmark. SmolVLA is the matched direct-policy MPS baseline. VLA-JEPA is a bounded
predictive-training comparator because its official LeRobot path has MPS-specific handling and uses
safetensors. It is not a stock offline `D` producer. Policy inference does not call the predictor.
The world-loss path mixes positions with different video-tubelet ancestry and conditions on learned
latent-action tokens, not robot actions. Its full predicted tensor cannot enter a row-aligned
pre-action `D`. The pinned two-frame tubelet encoder makes even the earliest context depend on
`t+1`. SLIM is a smaller alternative after rights and safe-loading gates pass. LiLa-WAM is a
separate 0.5B no-language predictive ablation. Its released inference loop ignores returned shared
tokens and does not call the future decoder.

The released JEPA-WAM source and model revisions are now bound. Its main checkpoint is a
5,355,388,110-byte pickle-based artifact from a CUDA-tested stack. It is an MPS port candidate,
not an MPS-qualified model. SmolVLA, VLA-JEPA, and SLIM remain ahead of it in the local sequence.

Efficient-WAM's reviewed source and model revisions are also bound. The source is Apache-2.0, but
the model repository has no card or declared weight license. Its roughly 1.98 GB checkpoint omits
the runtime cost of UMT5-XXL and the Wan VAE. The code asserts CUDA before its nominal attention
fallback and uses float64/complex RoPE. Treat it as later class-J port work, not a low-overhead MPS
configuration.

The exact Light-WAM code revision is MIT-licensed. Its exact reviewed LIBERO checkpoint is a
3,720,363,717-byte pickle-based PyTorch artifact with no model-card license. Its CUDA-or-CPU
device resolver and float64/complex RoPE path make MPS a port, not a documented setting.

## Literature and source pass

The review used primary papers, proceedings, official project repositories, and the local arXiv
cache. It also refreshed arXiv and public web discovery through 2026-08-13. X was searched only as a
discovery surface. No X post was promoted to evidence, and no conclusion depends on one.

The new source pass covered:

- PID definition, non-uniqueness, shared exclusions, Gaussian alternatives, and KSG limits;
- VLA modality PID, failure prediction, confidence, action monitoring, and temporal calibration;
- counterfactual and mechanistic VLA work;
- heterogeneous treatment-effect validation;
- right-censored and competing-risk scoring rules;
- saliency sanity checks and causal abstractions;
- all 36 exact-phrase August WAM submissions through the final cutoff, with deeper deployed-graph
  review for Flex-\(\pi\),
  SLIM, LiLa-WAM, World Tokens, JEPA-WAM, SelfWAM, FACT, Surgical WAM, CoWAM, DynamicWAM,
  FlowPilot, SG-WAM, Vid2WAM, DreamWAM, Robust-WAM, MobileWAM, and \(\tau0\)-WM;
- broader-name graph and release review for World-to-Wrist, Efficient-WAM, LDA-1B, WLA-0,
  RepWAM, and Kairos;
- action-grounding, world-model hallucination, and candidate-planner tests;
- exact code, checkpoint, rights, and MPS boundaries for the local model shortlist.

The source pass changed comparator and prediction-contract requirements. It did not supply the
missing Prisoma population law, VLA PID application validation, closed-loop H1-B experiment, or
prospective H2 capture.

## `pid-rs` adoption review

The submodule pin remains fixed. A temporary isolated clean consumer worktree at exact public
revision `722d3abeb922fc4119ecb9f92d7fedca096c9f77` passed these commands:

```text
cargo +1.93.0 check --locked --workspace --all-features
cargo +1.93.0 test --locked --workspace --all-features --no-run
cargo +1.93.0 test --locked --workspace --all-features
```

Current public main `7473e62acef6077c2c1147e09d5d1297f2a2874b` is 97 commits beyond the pin.
The consumer run covers the compiled and tested Prisoma Rust surface at `722d3abe`. Head
`bbdfda40` changes only assurance, workflow, script, and prose surfaces relative
to that tested revision. Estimator-code anchor `cb3f58f0` adds one bounded KSG integration commit. Prisoma
inspected its three changed production Rust files and replayed four predecessor-radius fixtures
plus the structured overflow fixture.
Current head `7473e62` changes custody and assurance files only. Its full CI run `31724449805`
failed two jobs. The narrower `Push on main` run `31724449083` passed.

This result establishes compatibility for the compiled and tested Rust consumer surface. It does
not establish behavioral compatibility, schema migration, fixture equivalence, or scientific
value. Exact-head CI run `31686107959` is red on a stale certified-SxPID2 workflow digest.
Exact-head CodeQL run `31686106737` passed. Upstream still marks broader revision-4 KSG repository
integration NO-GO. Consumer compatibility, schema, package, and scientific-value review remain
adoption blockers.

## External integration refresh

Paper2Brain public main was observed at `2648caf18d24075c4a36af81a6bb032bb551244e` on
2026-08-13. Four changes beyond the previous review point concern adjacent-page captions,
visual-parent disambiguation, and visual-grounding design and value gates.
Its `integrations/extensions/prisoma/manifest.json` remained byte-identical to Prisoma's local
descriptor at SHA-256 `006a6cc5fe46041fcc180d1890a36f821e8901768161952b143bbfc3c3fd70f9`.
This preserves only the E2 consumer-manifest relationship. It adds no producer, translator,
authority, process attestation, golden fixture, or scientific conformance.

NCP public main was observed at `1a04294c90c1b50eba06ae1c6afe9c951319250d` on
2026-08-13. Two commits after the prior review refine the maintainer-side low-overhead architecture
and record a prepared-stream-monitor gap. They do not change the release decision. B01 remains
`IN_PROGRESS` with no passing receipt. P01, P02, and P03 remain open and not run.

## File-class decisions

- Current Markdown was reconciled against the final claim boundaries.
- `AGENTS.md` and `CLAUDE.md` now carry the same scientific stop rules.
- The superseded M0 v2 draft remains historical. The active v3 successor remains all-null,
  unreviewed, and non-promotable. H3 freeze is blocked on real ancestry producer, consumer, and
  per-row receipt artifacts.
- The v1 historical scaffold remains historical.
- Capability views are regenerated from the catalog at source freeze. Any later bound-source edit
  requires another regeneration and check.
- Candidate release artifacts remain separate until the source commit is clean and pushed.
- Immutable release review and requirements inputs remain unchanged.
- `pid-rs/` remains clean at the pinned commit.

## Historical verification record

The checklist below records the completed source-audit cycle already present in repository
history. It is not a claim that a later working-tree refresh is complete. The cycle began at
`6d6f895d57ec38feb417a6027cab8dcdf525ce2a` and ended at
`58e8156f19df87ed79965fddbf8f1e869ea0583b`.

- [x] Inventory and starting-state capture.
- [x] Personal line-by-line read of `grandplan.md`.
- [x] EC1 and H1-H4 inference reconstruction.
- [x] Statistical analysis and gate reconstruction.
- [x] Primary-source and current-literature refresh.
- [x] Current Markdown classification and consistency review.
- [x] Machine-readable claim and successor-schema review.
- [x] Pinned and current-upstream `pid-rs` review.
- [x] Isolated current-upstream consumer compile and test-target build.
- [x] NCP mapping implementation corrections and focused tests.
- [x] pid-rs-facing handoff draft.
- [x] Deployed-graph WAM taxonomy and primary-source artifact review.
- [x] Fifty-lens adversarial design framework in the canonical plan.
- [x] Local MPS environment record: Apple M4 Max with 128 GiB. The 2026-08-13 environments lacked
  PyTorch. A separate 2026-08-14 exact-artifact probe produced finite repeatable synthetic LeWM
  tensor and full-budget CEM outputs on MPS. It did not run PushT or establish qualification.
- [x] Regenerated capability views and checked the research-governance bindings. The final
  capability regeneration followed the last bound-source edit.
- [x] Full source Rust, Python, docs, governance, release-intake, and adversarial gates. Before
  candidate regeneration, the full Python run had 593 passes, one optional skip, and only the two
  expected stale-candidate binding failures.
- [x] Personal final diff, current-Markdown, generated-file, immutable-intake, submodule, and
  source-tree review.
- [x] Core source-audit commit `03b46396dc8908b35381d2779cc6e903f2e03652` pushed to
  `origin/main` and fetched back at the same revision.
- [x] Candidate artifacts rebound to the audited source in
  `df9e67ec1c7979529fbb8191801e9d30cfe90ff3`.
- [x] Candidate artifacts rebound after the governance CI repair in
  `58e8156f19df87ed79965fddbf8f1e869ea0583b`.
- [x] The next review opened with one `main` worktree, no stale local branch, a clean submodule,
  and local `main` equal to `origin/main` at `58e8156f19df87ed79965fddbf8f1e869ea0583b`.

Do not edit a source-bound audit record only to predict its own later publication. For later
refreshes, use candidate artifacts, Git history, the fetched remote revision, and the worktree
inventory as the authoritative completion evidence.

## Promotion rule

Do not promote a scientific claim because this audit closes. Promotion requires the exact frozen
protocol, typed obligation coverage, independent evidence, and authenticated post-push CI defined
by a reviewed successor schema. Progress schema 0.1 cannot encode that terminal state.
