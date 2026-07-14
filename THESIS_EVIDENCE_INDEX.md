# Prisoma 0.9 thesis-evidence index

**Release scope:** Prisoma 0.9.0 source and research-software preview

**Author:** Sepehr Mahmoudian
**Canonical research specification:** [`grandplan.md`](grandplan.md), docset v12.5

This index answers one question: **what can each artifact in Prisoma 0.9 actually support?** It
does not upgrade implementation or fixture tests into thesis evidence. The authoritative current
status is [`protocols/research_claim_registry_v1.json`](protocols/research_claim_registry_v1.json),
and the generated per-feature inventory is
[`docs/CAPABILITY_MATRIX.md`](docs/CAPABILITY_MATRIX.md). The latter currently contains no
`validated` rows.

Read this index with [`LIMITATIONS.md`](LIMITATIONS.md). When a claim is absent from this index, it
is not authorized by the 0.9 evidence bundle.

## Evidence vocabulary

| Term | Meaning in this repository | Does not mean |
|---|---|---|
| Specification | A declared design, interface, estimand, or acceptance rule. | Implemented, executed, reviewed, or valid. |
| Implementation | Code or a dependency exists. | The advertised path passed, is secure, or has scientific validity. |
| Local proof | A named command tests behavior on exact local inputs. | Independent replication, external validity, a hypothesis result, or deployment qualification. |
| E0–E5 evidence basis | The relationship-evidence scale in [`grandplan.md` §8.9](grandplan.md#89-repository-ecosystem-evidence-boundaries-and-useful-roles). | A substitute for the separate software status or scientific claim status. |
| Scientific evidence | Data and inference produced under a frozen, identified, leakage-safe protocol with its declared holdout and uncertainty. | A synthetic fixture, arithmetic reference, screenshot, or passing unit test. |
| Governance receipt | Content-bound human and custodian decisions for freeze, access, rights, review, or claim authorization. | A locally generated unsigned placeholder or inferred approval. |

The evidence order is fail-closed: variable and population definition precede identification;
identification precedes estimator validation; estimator validation precedes confirmatory
analysis; and confirmatory analysis precedes interpretation. Failure at an earlier level blocks
later language.

## Source-of-truth order

1. [`grandplan.md`](grandplan.md) defines claims, estimands, gates, and stop rules.
2. [`protocols/research_claim_registry_v1.json`](protocols/research_claim_registry_v1.json)
   records current execution/freeze status and permitted/prohibited language.
3. [`protocols/capability_catalog_v1.json`](protocols/capability_catalog_v1.json) generates the
   machine and human capability matrices.
4. [`docs/CAPABILITY_MATRIX.md`](docs/CAPABILITY_MATRIX.md) indexes exact software paths,
   revisions, proof commands, limitations, and E0–E5 evidence bases.
5. Run logs, summaries, fixtures, manifests, and command output prove only the bounded behavior
   their contracts name.

`just docs-audit`, `just research-governance`, and `just capability-matrix-check` test consistency
among these sources. They do not perform scientific or independent human review.

## Release-level status

| Item | Evidence present in 0.9 | Status for thesis use |
|---|---|---|
| M0 scientific freeze | Machine-checkable `unfrozen_draft` scaffolds and honest-unfinished-state validation. | **Not freeze-ready; not a preregistration.** |
| Confirmatory holdout | Registry says none is registered; access chain contains only a genesis event. | **Unavailable.** No independence or historical non-access proof. |
| EC1 | Local E0/E2/E3 software and fixture evidence across run-log, replay, Rerun, bridge, SAFE ingress, simulator, and NCP components. | **Not established.** External benchmark, second adapter, and independent reproduction are absent. |
| H1-A | Synthetic common preflight and deterministic finite Protocol-A scoring primitive. | **Software fixture only; no H1 evidence.** |
| H1-B | Canonical randomized closed-loop design. | **Blocked and unimplemented.** |
| H2 | Synthetic fixed-horizon/IPCW/alarm arithmetic reference. | **Software fixture only; no H2 evidence.** |
| H3 | Report-first estimator eligibility/abstention behavior and negative gate records. | **Not eligible; all four gates are blocked.** |
| H4 | Reference-model attribution logging and deletion-faithfulness control. | **Exploratory software groundwork only.** |
| NCP | Read-only wire-0.8 observer and deterministic local fault observatory. | **Experimental optional component; not live integration or scientific evidence.** |

## EC1 — provenance-complete replay

**Thesis role.** Engineering acceptance claim defined in
[`grandplan.md` §4](grandplan.md#ec1--provenance-complete-replay), separate from H1–H4.

**Current artifacts and local proofs**

| Artifact | Proof command | Bounded inference |
|---|---|---|
| Canonical run-log validation, replay, manifests, and sidecars in [`pid-rs/crates/pid-runlog`](pid-rs/crates/pid-runlog) plus the local consumer | `just runlog-sidecars-proof` | Exact checked schema-2 event/replay and sidecar behavior for the fixture path. |
| Validating run-log-to-Rerun converter in [`crates/pid-rerun`](crates/pid-rerun) | `just runlog-rerun-proof` | The checked conversion produces a nonempty Rerun artifact after run-log validation. |
| Agent Bridge in [`crates/pid-bridge`](crates/pid-bridge) and [`crates/pid-sim`](crates/pid-sim) | `just bridge-security` | Local tests for the enumerated single-request, bind/default, wire-cap, and non-adversarial file behavior. |
| Bounded SAFE canonical ingress in [`experiments/safe_adapter`](experiments/safe_adapter) | `just safe-adapter` | Synthetic content-addressed NPZ/JSON conversion and checked rejection behavior. |
| Offline `(V,L,D,A)` harness in [`crates/pid-sim/src/offline_harness.rs`](crates/pid-sim/src/offline_harness.rs) | `just offline-harness` | Deterministic local synthetic artifact-to-run-log behavior. |

**What is missing before EC1 language is permitted**

- a real content-addressed capture and reviewed rights receipt;
- the blinded comparison against an ordinary script/container and standard robotics containers;
- a structurally different second adapter rather than another self-generated fixture;
- the complete graded fault/replay report, including the declared corruption suite;
- an adapter-promotion report and external clean-room reconstruction.

Therefore, 0.9 may describe implemented local paths but may not say EC1 is complete, externally
validated, independently reproduced, portable, or deployment-ready.

## H1-A — paired frozen-snapshot intervention response

**Thesis role.** Predict an effect-specific paired algorithmic response using genuinely
pre-treatment diagnostics under a frozen clone/restoration contract. This is not a physical
individual treatment effect. See
[`grandplan.md` §4 H1](grandplan.md#h1--pre-treatment-diagnostics-predict-intervention-response)
and [`§6.3`](grandplan.md#63-h1-analysis-paired-algorithmic-and-randomized-closed-loop-response).

**Current artifacts and local proofs**

- [`crates/pid-sim/src/h1_preflight.rs`](crates/pid-sim/src/h1_preflight.rs) and its positive,
  semantic-failure, and parse-failure fixtures: `just h1-preflight`.
- [`crates/pid-sim/src/h1_protocol_a.rs`](crates/pid-sim/src/h1_protocol_a.rs) and
  [`crates/pid-sim/fixtures/h1_protocol_a_valid.json`](crates/pid-sim/fixtures/h1_protocol_a_valid.json):
  `just h1-protocol-a`.

These establish deterministic software behavior for a finite synthetic fixture: exact preflight
binding, paired clone-state restoration checks, treatment-order reversal, declared zero RNG draws,
a frozen scaled response, an outer-fold design-only versus design-plus-moderator scoring
calculation, canonical logging, and fail-closed rejection. They establish no H1 evidence.

**Missing scientific evidence**

- a frozen real policy, environment, intervention, target population, estimand, response scale,
  minimum useful effect, outer unit, and split;
- a real pilot covering dose, engagement, specificity, placebo/positive controls, reset, timing,
  stochastic-policy behavior, and safety handling;
- a real content-addressed capture with independent cases and task-family-blocked evaluation;
- a frozen-candidate holdout result with calibration, uncertainty, baseline comparison, and
  replication.

Factual outcome prediction, synthetic arithmetic, or a deterministic simulator contrast may not
be relabeled as real paired intervention-response evidence.

## H1-B — randomized closed-loop effect modification

**Thesis role.** Estimate a prespecified treatment-by-diagnostic interaction or stratum-specific
causal effect under prospective randomized assignment. Protocol B is not interchangeable with
Protocol A.

The design is specified in [`grandplan.md` §4](grandplan.md#h1--pre-treatment-diagnostics-predict-intervention-response)
and [`§6.3`](grandplan.md#63-h1-analysis-paired-algorithmic-and-randomized-closed-loop-response),
but 0.9 has no real assignment generator, concealment/receipt execution, randomized episode
capture, compliance/crossover/rescue record, effect learner, pilot, or held-out result. There is
no Protocol-B proof command because the protocol is unimplemented.

Before H1-B can be claimed, M0 must freeze the treatment version, assignment unit and
probability, interference/reset boundary, ITT estimand, timing, eligible baseline moderator,
outcomes, missingness, safety override, effect scale, multiplicity, power, and external or
transport target. Real randomized execution, manipulation checks, task-family-blocked inference,
and replication must then pass.

## H2 — prospective censoring-aware failure prediction

**Thesis role.** Test whether diagnostics improve prospective failure prediction beyond a
matched-access comparator frontier at a frozen landmark and horizon. See
[`grandplan.md` §4 H2](grandplan.md#h2--diagnostics-improve-prospective-censoring-aware-failure-prediction)
and [`§6.4`](grandplan.md#64-h2-analysis-prospective-failure-with-time-and-censoring).

**Current artifacts and local proofs**

- [`crates/pid-sim/src/h2_reference.rs`](crates/pid-sim/src/h2_reference.rs) and
  [`crates/pid-sim/fixtures/h2_reference`](crates/pid-sim/fixtures/h2_reference): `just
  h2-reference`.
- The PID- and NCP-independent static label-baseline smoke: `just firebreak`.

The H2 reference exercises deterministic synthetic fixed-horizon landmark eligibility,
task-family-held-out weighted fitting, grouped cross-fitted stratified reverse-KM IPCW,
Horvitz–Thompson Brier arithmetic, competing-event classification, reliability bins, frozen alarm
and nondetection accounting, and declared-payoff utility. The firebreak demonstrates only that a
local static baseline path can run without PID or NCP. Neither is prospective H2 evidence.

**Missing scientific evidence**

- a frozen real event ontology, landmark, horizon, target population, censoring/competing-event
  rules, feature availability contract, minimum useful effect, and decision payoff;
- prospective episode registration and enough independent failures, episodes, and task families;
- complete matched-access baselines, including named literature comparators after protocol review;
- training-only alarm selection or an independently justified external threshold;
- full calibration intercept/slope and uncertainty, censoring diagnostics and sensitivity,
  missing-sensor analysis, leakage audit, and subgroup/task-family variation;
- an untouched later-time or external holdout and independent replication.

No claim of prospective validity, warning benefit, calibration, comparator superiority, safety,
transport, or deployment follows from the synthetic reference.

## H3 — conditional incremental PID value

**Thesis role.** Compare a strong non-PID model M1 with M2 = M1 plus eligible PID features on the
same outer folds and budget. H3 is activated only inside a validated support envelope. See
[`grandplan.md` §4 H3](grandplan.md#h3--pid-adds-incremental-value-only-inside-its-validated-support-envelope)
and [`§7.1`](grandplan.md#71-separate-four-questions).

**Current artifacts and local proofs**

- [`findings.md`](findings.md) records the high-dimensional MI/coherence NO-GO and the blocked
  real-embedding application gate.
- `just exp0-bin` runs the pinned estimator gate and reports its actual verdict.
- `just estimate-report-contract` exercises positive and abstaining report-first fixture paths;
  abstentions have no numeric placeholder.
- `just offline-harness` exercises local synthetic artifact diagnostics; geometry output does not
  clear a PID gate.

The current population, measure, estimator, and application gates are all blocked for H3. The
`pid-rs` submodule is the canonical implementation dependency, not an independent implementation.
Quantized discrete `I_min` is a different estimand from continuous shared-exclusions and cannot be
used as an automatic fallback or pooled comparison.

**Missing scientific evidence**

- a content-bound matched-regime gate receipt covering the population law, exact measure,
  estimator configuration, dimensions, sample region, preprocessing, ties/refusal, dependence,
  coverage, and abstention;
- an eligible train-fitted episode-local information feature;
- an already useful non-PID H1 or H2 problem and a complete strong M1 baseline frontier;
- the nested, task-family-blocked, held-out M2-over-M1 comparison, useful-margin decision,
  sensitivity analysis, and second-model/family replication.

Nonzero atoms, attractive visualizations, geometry diagnostics, or in-sample association are not
H3 evidence. Gate failure or no useful incremental value is an admissible negative result; it must
not block H1, H2, or H4.

## H4 — representational availability versus causal use

**Thesis role.** Compare held-out availability from a locked probe with target-engaging
interventions at the same representational site. See
[`grandplan.md` §4 H4](grandplan.md#h4--representational-availability-can-diverge-from-causal-policy-use).

**Current artifacts and local proof**

- [`experiments/attribution`](experiments/attribution) and the canonical `attribution_logged`
  event path: `just attribution-probe`.

The small reference model, deletion-AOPC comparison with a random control, artifact generation,
and canonical event logging are exploratory software groundwork. They do not demonstrate
availability or causal use in a real policy.

**Missing scientific evidence**

- a frozen real target, representation, probe, task family, checkpoint, and availability metric;
- held-out probe performance with shuffle, temporal-shift, layer/hook, and leakage controls;
- at least two prespecified target-engaging intervention constructions where feasible, with
  dose-response, sham, off-target, orthogonal, and equivalent-norm random controls;
- action and counterfactual-effect outcomes, minimum useful effect, held-out analysis, and
  model/checkpoint plus task-family replication.

Probe accuracy, attribution maps, or agreement between explanation methods alone cannot establish
causal use or H4.

## M0 and human-governance evidence still absent

The following artifacts require real external state or human judgment and therefore cannot be
generated or inferred by repository automation:

| Required evidence | Required independent role or review | Current 0.9 state |
|---|---|---|
| Reviewed successor M0 schema and fully specified freeze candidate | Candidate, supervisor, causal/statistical reviewer, and H3 estimator reviewer if H3 is eligible | Absent; v1 is intentionally non-promotable and unfrozen. |
| Freeze signatures and amendment policy | Candidate, supervisor, independent reviewer | Absent; no approval is implied. |
| Holdout generation, commitment, custody, and access/reveal receipts | Independent holdout custodian | No confirmatory holdout is registered. |
| Real data/model rights, privacy, ethics, retention, and incident decisions | Data steward/controller and institutional review where applicable | Unresolved for real capture. Synthetic fixtures only. |
| Fresh reproducible prior-art search and comparator dispositions | Named reviewers under a saved search/screening protocol | Absent; current ledger is a legacy inventory. |
| EC1 external baseline and clean-room reconstruction | Independent reproducer using a structurally different adapter | Absent. |
| Real H1/H2 pilot and capture authorization | Domain, safety, policy/environment, and statistical reviewers | Absent. |
| First frozen-candidate holdout results and claim authorization | Custodian plus named scientific reviewers | Absent. No result or authorization may be invented. |

The local holdout access genesis event is evidence that the repository initialized a chain; it is
not evidence that no one accessed outcomes previously or elsewhere.

## Optional NCP evidence

[`crates/ncp-observer`](crates/ncp-observer) is excluded from the default workspace and pinned to
NCP wire 0.8. Its local proofs are:

```bash
cargo test --locked --manifest-path crates/ncp-observer/Cargo.toml
just ncp-fault-observatory
```

They support the bounded read-only consumer and deterministic fault-fixture behavior described in
[`crates/ncp-observer/README.md`](crates/ncp-observer/README.md). They do not establish a public
live Engram producer, final-version interoperability, delivery completeness, timing/QoS/reconnect
behavior, authentication/ACL security, E4 conformance, EC1, H1, H2, H3, or H4. NCP, Galadriel,
Haldir, Crebain, and other ecosystem projects are optional to the minimum path.

## Thesis-paper readiness map

| Thesis unit in [`grandplan.md` §11.1](grandplan.md#111-minimum-viable-thesis) | 0.9 contribution | Evidence still required |
|---|---|---|
| Paper A: experiment semantics and benchmark | Run-log/replay/bridge/Rerun/adapters and fault-test groundwork. | External conventional-stack baseline, blinded audit tasks, complete fault/replay grading, second adapter, portability measurement, and independent reproduction. |
| Paper B: intervention-grounded diagnostics | H1-A structural/scoring fixture and H2 arithmetic fixture; H4 attribution logging groundwork. | Real randomized intervention study, strong matched baselines, manipulation checks, hierarchical inference, availability–use analysis, held-out families, and replication. |
| Paper C: conditional information decomposition or rigorous negative boundary | Pinned report-first estimator interface, abstention behavior, and recorded NO-GO/blocked status. | Matched-regime gates, oracle evidence, eligible local features, M2-over-M1 holdout comparison, negative-result boundary, and second-model/family replication. |

None of these paper units is complete in 0.9. The software preview makes their current executable
groundwork and missing evidence auditable; it is not itself paper-level novelty or validation.

## Claim-language register

| Claim | Permitted statement | Prohibited statement |
|---|---|---|
| EC1 | The canonical run-log, local replay/conversion, and bounded content-addressed SAFE synthetic-ingress paths are implemented for tested fixtures. | EC1 is complete, externally validated, independently reproduced, or deployment-ready. |
| H1-A | The common preflight and deterministic finite-benchmark Protocol-A reference are fixture-runnable scoring primitives and establish no H1 evidence. | H1 passed, a physical individual effect was observed, or the fixture demonstrates real intervention sensitivity. |
| H1-B | The randomized closed-loop protocol is specified and blocked. | Protocol B exists as an executed study, or Protocol A establishes closed-loop effect modification. |
| H2 | The deterministic synthetic reference exercises named protocol arithmetic on checked fixtures only. | H2 passed, or prospective prediction, calibration validity, warning benefit, censoring validity, comparator superiority, safety, transport, or deployment has been shown. |
| H3 | PID outputs abstain or remain noninterpretable outside their named gates. | Geometry or emitted numbers establish real-embedding PID validity or incremental value. |
| H4 | The reference attribution path exercises logging and a deletion-faithfulness control. | Attribution agreement proves causal use or establishes H4. |

Any future evidence-changing revision must update the canonical plan, claim registry, capability
catalog/matrix, limitations, and this index together. A result should be added only with its exact
protocol revision, data and split identities, first-attempt holdout receipt, analysis environment,
proof command, uncertainty, reviewer authorization, and explicit claim impact.
