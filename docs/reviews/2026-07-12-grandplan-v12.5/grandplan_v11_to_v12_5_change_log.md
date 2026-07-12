# Change log: `grandplan_v11.md` → `grandplan_v12_5.md`

**Date:** 2026-07-12
**Repository snapshot held fixed:** `sepahead/prisoma@64bd881248463e7142d022bb95a5850bcf8fced2`
**Purpose:** record substantive scientific, statistical, literature, and ecosystem changes made during the second-round adversarial review

## Quantitative summary

| Property | v11 | v12.5 |
|---|---:|---:|
| Bytes | 113,828 | 198,502 |
| Lines | 2,017 | 2,536 |
| Approximate word tokens | 14,405 | 25,078 |
| Headings | 187 | 222 |
| Defined references | 71 | 112 |
| Git textual change | — | 671 insertions, 152 deletions relative to v11 |

The increase is not a return to the original plan’s uncontrolled scope. Most additions make estimands, validation, comparators, ecosystem boundaries, and stop rules more explicit.

# 1. Scientific claim architecture

## Added explicit protocol split for H1

v11 treated intervention-grounded prediction as one broad claim. v12.5 separates:

- **Protocol A:** paired frozen-snapshot algorithmic response;
- **Protocol B:** randomized closed-loop response.

The protocols now have distinct units, target populations, treatments, outcomes, assumptions, scores, and permitted interpretations. A paired software contrast cannot be described as an observed physical individual treatment effect or closed-loop robustness result.

## Strengthened pre-treatment requirements

The primary moderator must be computed from the untreated baseline state with outer-training transformations only. Features from treated passes, treatment engagement, downstream controllers, or future frames are prohibited. Diagnostic instrumentation must pass a noninterference test.

## Repaired heterogeneous-effect validation

v11’s moderation analysis is replaced with a protocol-specific validation stack. For Protocol B, factual-outcome prediction and coefficient significance are secondary. Primary criteria now include:

- cross-fitted R-loss or doubly robust effect loss;
- causal calibration with training-defined groups and held-out randomized contrasts;
- a prespecified rank/prioritization statistic;
- treatment-policy value or regret;
- cluster-aware or randomization-based uncertainty for the global heterogeneity null.

Naive same-data individual-effect targets are explicitly prohibited.

## Clarified target populations

Finite benchmark, task-family superpopulation, and transported population claims are now distinct. Every result must state which one it targets.

# 2. Prospective failure prediction

## Expanded mandatory comparator frontier

v12.5 adds or strengthens direct comparison with:

- SAFE;
- Hide-and-Seek;
- ActProbe;
- Rewind-IL/TIDE;
- architecture-stratified black-box action monitoring;
- VLAConf;
- perturbation-based uncertainty;
- activation warning probes;
- Foresight action-conditioned world-model latents;
- temporal-difference calibration;
- Tri-Info and existing uncertainty, temporal, OOD, progress, and learned baselines.

Methods must be compared at matched information access, supervision, action resampling, external-model use, latency, and compute. Otherwise results are reported as a Pareto frontier.

## Added censoring and competing-risk precision

The plan now distinguishes fixed-horizon failure, cause-specific hazard, cumulative incidence, remaining time, and dynamic risk. Success, timeout, takeover, reset, and other failure modes may be competing events rather than ordinary negatives.

## Fixed alarm-policy underdefinition

Repeated scores are converted to alarms only through a frozen training-only policy specifying threshold, persistence/debounce, refractory period, event-matching window, reset behavior, and missing-score handling.

## Fixed lead-time selection bias

Lead time must retain undetected failures explicitly. Conditional lead time among detected failures alone cannot rank monitors.

## Strengthened calibration and shift language

Conformal and recalibration claims now state exchangeability or shift assumptions, calibration unit, repeated-landmark dependence, and whether coverage is theoretical or merely empirical under transport.

# 3. Safety outcomes

Binary success is no longer sufficient. v12.5 adds:

- cumulative safety cost;
- risk-exposure duration;
- safe success, unsafe success, safe failure, and unsafe failure quadrants;
- explicit separation between monitoring evidence and certification.

A duplicated ForesightSafety-VLA reference introduced during literature expansion was removed in the final pass; the remaining reference is unique.

# 4. PID and estimator status

## Reconciled pinned versus current `pid-rs`

v12.5 records four separate facts:

1. Prisoma pins `pid-rs@8a5a9dda601556443f956a2fba164cccc913ed2e`.
2. That pin has meaningful low-dimensional Gaussian-oracle and discrete-reference evidence.
3. At the pin, reproducible external continuous cross-validation was still documented as pending.
4. Later `pid-rs` main revision `70b45f7b75fac06777ea215a73df01209490311a` adds a public `csxpid` fixture, reported agreement within `1e-12` nats after recorded conversion, fail-closed population-support contracts, and stronger provenance.

The later revision is a candidate upgrade, not inherited evidence. A migration requires exact pinning, API/estimand review, lockfile regeneration, synthetic and adapter conformance reruns, result-delta reporting, and preservation of the old environment.

## Preserved application abstention

Neither low-dimensional oracle success nor external fixture agreement establishes high-dimensional, dependent, mixed-dimensional VLA application validity. Population, measure, estimator, and application verdicts remain separate.

## Strengthened local-feature validity

Global PID atoms cannot be treated as episode features. Local scores require a train-reference population, a named measure or explicit surrogate, cross-fitting, oracle/null tests, aggregate-reconstruction checks, and disjoint fit/eligibility/evaluation data.

# 5. Repository ecosystem

## Added full public-repository screen

Metadata for all 174 public repositories on the `sepahead` profile was screened. Relevant candidates received deeper review. Negative findings are bounded to public surfaces because private branches and authenticated code search were unavailable.

## Added E0–E5 evidence ladder

- E0: intention or adjacency;
- E1: interface specification;
- E2: immutable dependency;
- E3: build-tested adapter;
- E4: end-to-end scientific conformance;
- E5: independent replication.

The words “connected,” “integrated,” and “validated integration” are now evidence-gated.

## Added direct relationship boundary

Only two direct relationships are supported at the reviewed snapshot:

- `pid-rs` as pinned estimator/run-log submodule;
- NCP as optional pinned read-only observer dependency.

Galadriel, Crebain, Manwe, Engram, Melkor, WorldWarp, GauSS-MI, Cobot Atlas, Relief Atlas, Cortexel, Haldir, the NEST fork, and the ReScience/BROJA lineage are candidates, comparators, specifications, or fixtures—not current Prisoma integrations.

## Added dependency firebreak

The minimum thesis must:

- build with NCP disabled;
- run H1/H2 with PID disabled;
- require no private sibling repository or token;
- replace every sibling with a local-file or standard-format adapter;
- isolate optional viewer/world-model/asset failures;
- prevent producers from seeing outcomes, holdout membership, or future schedules;
- content-address and revision-pin cross-repository artifacts.

## Added adapter promotion contract

Every candidate adapter must document revisions, lockfiles, licenses, schemas, units, clocks, sequences, frames, transforms, assignment/receipt/outcome boundaries, security, fault tests, performance, replay equivalence, and estimand preservation before E4 wording is permitted.

# 6. Infrastructure and experimental semantics

v12.5 strengthens the distinction between:

- policy distribution;
- proposed action;
- controller transformation;
- executed command;
- environment transition;
- physical outcome.

It expands provenance requirements for recurrent memory, tactile state, predictive/world-model state, external monitors, low-level controllers, and protocol sources. This reflects July 8–9 developments in latent memory, tactile predictive/reactive control, latent environment evolution, and memory-guided VLA harnesses.

# 7. Literature and novelty

The novelty boundary is further narrowed. v12.5 no longer allows generic claims of:

- first multimodal PID;
- first VLA diagnostic;
- first VLA failure monitor;
- first intervention-based VLA explanation;
- generic logging, simulator, or viewer novelty.

The defensible contribution is the combination of availability–use–effect separation, claim-matched paired/randomized reference criteria, estimator abstention, prospective external validation, portable experiment semantics, and a fair incremental PID test.

References added since v11 cover repository evidence, causal inference, heterogeneous-effect model selection and calibration, censoring-aware prediction, decision analysis, conformal prediction beyond exchangeability, the latest monitoring methods, scientific lineage repositories, and July 2026 VLA developments.

# 8. Scope control

The minimum thesis remains:

- Paper A: experiment semantics and infrastructure benchmark;
- Paper B: intervention-grounded diagnostics;
- Paper C: conditional PID study only after gates pass.

NCP live integration, Engram, NEST, custom simulation, custom UI, Gaussian splatting, WorldWarp, GauSS-MI, large asset domains, and full continuous three-source PID remain optional. The plan explicitly protects Papers A and B from PID NO-GO and optional-repository failure.

# 9. Validation and editorial corrections

The final v12.5 candidate was checked for:

- UTF-8 validity and final newline;
- NUL and trailing whitespace;
- balanced code fences;
- heading duplicates and hierarchy jumps;
- table column consistency;
- malformed URLs;
- unmatched math delimiters;
- stale version markers and placeholders;
- contiguous reference identifiers;
- duplicate reference identifiers;
- undefined citation uses;
- unused reference definitions;
- duplicate URLs and duplicate literature entries.

The replacement and incremental patches are separately apply-tested in temporary Git repositories and compared byte-for-byte with the target files.
