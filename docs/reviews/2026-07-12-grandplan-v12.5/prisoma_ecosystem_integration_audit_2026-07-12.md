# Prisoma ecosystem integration audit

**Audit date:** 2026-07-12
**Prisoma snapshot:** `sepahead/prisoma@64bd881248463e7142d022bb95a5850bcf8fced2`
**Canonical plan informed by this audit:** `grandplan_v12_5.md`
**Scope:** public repositories and public repository surfaces only
**Decision rule:** repository prose is evidence of intention; an immutable dependency, executable adapter, or end-to-end conformance artifact is required for stronger language

---

## 1. Executive verdict

The public `sepahead` ecosystem contains many projects that are scientifically adjacent to Prisoma, but only two repository relationships are sufficiently concrete at the reviewed snapshot to be called direct:

1. **`pid-rs`** is a pinned Git submodule and the canonical estimator/run-log implementation used by Prisoma.
2. **NCP** is an optional, immutable dependency of Prisoma’s excluded-by-default, read-only observation client.

No inspected public evidence justified describing Galadriel, Crebain, Manwe, Engram, Melkor, WorldWarp, GauSS-MI, Cobot Atlas, Relief Atlas, Cortexel, Haldir, the NEST fork, or the information-theoretic lineage repositories as presently integrated with Prisoma. Several are useful candidate producers, comparators, fixtures, or transport settings. That usefulness should be preserved without converting architectural intention into implementation evidence.

This distinction materially improves the PhD plan. It prevents the thesis from depending on unpublished or pre-implementation siblings, reduces correlated-error claims, and makes Prisoma’s defensible infrastructure contribution clearer: a portable experiment-semantics layer that can admit heterogeneous producers after they satisfy a common scientific conformance contract.

The ecosystem should therefore be used in three ways:

- **direct dependencies:** revision-pin, reproduce, test, and audit;
- **candidate adapters:** promote only through an explicit evidence ladder;
- **scientific lineage or stress-test sources:** use as fixtures, comparators, or transport tests without claiming software integration or validation.

## 2. Audit method and limits

The audit screened the public metadata for all 174 repositories shown on the `sepahead` GitHub profile on July 12, 2026. Repositories with names, descriptions, documentation, dependency declarations, or scientific roles plausibly related to Prisoma were then inspected more deeply. The review prioritized:

- direct references to `prisoma`;
- Git submodules, lockfiles, exact Git revisions, release tags, and consumer manifests;
- adapters, schemas, fixtures, and end-to-end examples;
- statements about current versus planned integration;
- data, clock, coordinate-frame, intervention, provenance, security, and licensing contracts;
- explicit limitations that constrain scientific claims.

The audit is bounded in four ways.

First, it cannot inspect private repositories, local branches, unpublished artifacts, or unpushed commits. Second, anonymous GitHub code search is incomplete and sometimes requires authentication; therefore “no direct reference found” means no reference was verified in the reviewed public surfaces, not proof of global absence. Third, repository documentation can be stale relative to code. Fourth, successful compilation or a shared maintainer is not evidence that scientific variables retain the same meaning across a boundary.

These limitations are recorded rather than hidden. Positive relationships require positive evidence; negative findings remain explicitly bounded.

## 3. Evidence ladder

Every ecosystem edge is assigned the strongest level supported by public evidence.

| Level | Name | Minimum evidence | Permitted wording |
|---|---|---|---|
| E0 | Intention or adjacency | profile graph, README mention, roadmap, issue, common topic, or shared maintainer | “candidate,” “adjacent,” or “planned” |
| E1 | Interface specification | schema, adapter design, handoff document, or integration specification without a build-tested implementation | “specified” or “designed,” not integrated |
| E2 | Immutable dependency | submodule, exact Git revision, release tag, lockfile, or consumer manifest establishing reproducible code provenance | “connected” or “dependency” |
| E3 | Build-tested adapter | producer and consumer compile/test together against pinned revisions and golden fixtures | “integrated for the tested fixture/revision” |
| E4 | End-to-end scientific conformance | live or replayed data cross the boundary with validated schema, units, time, frames, interventions, provenance, faults, replay, and outcomes | “validated integration” for the declared regime |
| E5 | Independent replication | another team or independently maintained implementation reproduces the integration and scientific result | “independently replicated” |

Two cautions are essential. Shared code can raise software maturity while lowering independence: Prisoma and Galadriel using the same `pid-rs` implementation are not independent confirmations. Also, E3 is not automatically E4. Type compatibility does not prove that timestamps, coordinate frames, treatments, outcomes, or sampling populations preserve the scientific estimand.

## 4. Verified direct relationships

### 4.1 `pid-rs`: direct canonical implementation dependency

Prisoma’s reviewed root tree includes `pid-rs` as a Git submodule at `8a5a9dda601556443f956a2fba164cccc913ed2e`. Prisoma’s own documentation states that estimator and run-log crates live there as the single source of truth, and local crates path-depend into the submodule. This is unambiguous E2 evidence. Portions may reach E3 where Prisoma CI actually builds or tests the pinned crates together, but that status must be tied to a specific workflow and revision rather than inferred globally.

The scientific interpretation requires a pin-versus-current distinction. At the Prisoma pin, `pid-rs` already contains meaningful low-dimensional evidence: a semi-analytic additive-Gaussian redundancy oracle and discrete shared-exclusions reference checks. The pinned documentation also records that reproducible external continuous cross-validation was still pending and warns about high-dimensional and dependent regimes.

A later `pid-rs` main revision, `70b45f7b75fac06777ea215a73df01209490311a`, reports a committed fixture generated with the public `csxpid` implementation, agreement within `1e-12` nats after documented unit conversion, stronger population-support contracts, and richer provenance. Those are genuine developments, but Prisoma does not inherit them merely because they exist on `pid-rs` main. An upgrade is a scientific migration, not a routine floating dependency update.

Required migration procedure:

1. pin the proposed new revision explicitly;
2. archive old and new lockfiles, compiler/toolchain, and structured estimator reports;
3. review measure, population, preprocessing, API, and default changes;
4. reproduce the external fixture independently in an isolated environment;
5. rerun all synthetic validation families and application-support gates;
6. rerun Prisoma adapter, run-log, replay, and abstention tests;
7. preserve the old environment for exact reproduction;
8. report every changed result, not only successful ones.

Even after upgrade, low-dimensional agreement does not establish the validity of high-dimensional, dependent, mixed-dimensional VLA applications. `pid-rs` is an implementation dependency, not external corroboration. Application eligibility remains a separate empirical verdict.

### 4.2 NCP: direct optional read-only observation dependency

Prisoma’s `crates/ncp-observer` is documented as an optional, excluded-by-default, read-only tap pinned to immutable NCP release `v0.7.1`, wire 0.7. NCP’s public documentation independently describes Prisoma as an observer client on its observation plane. This supports E2. Fixture-level local tests may support E3 for exactly those fixtures, but an E4 claim would require a conforming live producer and full scientific conformance evidence.

The boundary must remain read-only. NCP also exposes a safety-gated action plane, but the existence of that plane is not permission for Prisoma to acquire command authority. A diagnostic observer should not silently become part of the control loop, change timing, or become a safety actuator.

Every NCP-backed record should preserve:

- NCP tag, wire version, and contract hash;
- realm and route;
- session and producer identities;
- authorization and transport-security profile;
- sequence number, source timestamp, local receipt timestamp, and clock uncertainty;
- drop, duplicate, reorder, gap, reconnect, and restart events;
- payload schema, units, shapes, and provenance;
- observer queue depth, backpressure behavior, and loss policy;
- explicit evidence that observer failure does not alter control.

The open/default realm is not authenticated network security. For any non-isolated deployment, the documented ACL and mutual-TLS profile must be enabled and verified. A mode/TTL governor is defense in depth, not a substitute for authentication and authorization.

The minimum thesis must build and run with NCP disabled. NCP is a scientifically useful transport and fault-injection opportunity, not a required dependency of the core contribution.

## 5. Project-by-project evidence matrix

| Project | Relationship at audit date | Level | Scientifically useful role | Claim that is not justified | Promotion requirement |
|---|---|---:|---|---|---|
| `prisoma` | reviewed focal repository | — | capture–intervention–replay substrate and diagnostic benchmark | completed scientific validation | execute preregistered experiments and external replication |
| `pid-rs` | pinned submodule; path dependency | E2, with fixture-specific E3 possible | canonical MI/PID estimator and run-log implementation | independent validation or high-dimensional VLA eligibility | deliberate pin upgrade, external reference, matched-regime gates |
| `NCP` | optional pinned observer dependency | E2, with fixture-specific E3 possible | versioned observation transport, protocol/provenance fault testing | secure live integration or control authority | live conforming producer, secure profile, E4 report, read-only proof |
| `galadriel` | no direct Prisoma edge verified; shares `pid-rs` and NCP | E0 between projects | comparator for NIS/CUSUM, signed correlation, cross-sensor consistency, and optional PID evidence | independent PID confirmation or live Crebain/Prisoma integration | common data contract, independent baselines, recorded E4 experiment |
| `crebain` | no direct Prisoma reference; NCP surfaces dormant/off by default | E0 between projects | multimodal embodiment, tracking/fusion, timing and fault-injection producer | always-on Engram/NCP/Prisoma loop | read-only export adapter, clock/frame contract, secure E4 replay/live report |
| `crebain-native` | archived/native lineage of Crebain | E0 | historical implementation comparison or lightweight simulator source | current supported Prisoma adapter | exact revision, status review, maintained schema and conformance suite |
| `manwe` | explicitly says no drop-in Prisoma adapter | E0/E1 | candidate perception producer, domain-shift and latency testbed | compatibility from common language or maintainer | satisfy documented schema/tensor/clock/frame/statistical promotion gates |
| `engram` | public placeholder; implementation unavailable | E0 | future neural-state/memory/dynamics producer | current executable dependency | public immutable release, license, fixture, semantics, NCP E4 evidence |
| `melkor` | no direct Prisoma edge verified | E0 | scene/reconstruction producer; measured reconstruction-quality nuisance | calibrated policy/PID uncertainty or implemented Prisoma integration | versioned adapter, geometry calibration, license review, benchmark |
| WorldWarp fork | Prisoma contains an optional integration specification; implementation not verified | E1 | external generated-scene or world-model counterfactual baseline | critical-path integration or causal ground truth | support/realism study, pinned upstream, adapter, compute/license/provenance gates |
| GauSS-MI concept | Prisoma specification labels it pre-implementation; weighted KSG is heuristic | E1 | reconstruction-quality covariate or active-view research question | valid weighted PID estimator | mathematical estimand, oracle families, finite-sample gate, independent review |
| `cobot-atlas` | no direct adapter verified | E0 | controlled asset diversity for object/appearance/layout factors | physics-valid benchmark by virtue of mesh count | frozen asset revision, collision/scale audit, duplicate groups, licenses, adapter |
| `relief-atlas` | no direct adapter verified | E0 | optional disaster-response scene stress test | primary manipulation benchmark or blanket licensing | per-asset rights, geometry/physics/quality audit, ethical scope review |
| `cortexel` | no direct edge verified | E0 | possible scientific-artifact renderer | validation through visual agreement | stable export schema, render tests, provenance and warning fidelity |
| `silmaril-vision-studio` | no direct edge verified | E0 | possible manual inspection/vision-model prototyping surface | reproducible scientific analysis | versioned artifact import/export and automated conformance tests |
| `haldir` | public evidence insufficient to establish a role | E0/unknown | possible future security or attestation component | present security assurance | public code, threat model, immutable release, independent review and tests |
| `brojapid-activationfunctions` | no software edge; prior BROJA-PID analysis | E0 lineage | discrete mechanism fixtures and cross-measure sensitivity | validation of shared-exclusions or atom interchangeability | reproduce fixtures, state different measure, compare qualitative claims only |
| `mahmoudian-2020-rescience` | no software edge; published replication lineage | E0 lineage | reproducibility practice and controlled transfer-function fixtures | current Prisoma or `pid-rs` validation | port explicit fixtures and compare against present estimands separately |
| `nest-simulator` fork | no direct adapter verified; PID/information-theory branch intent | E0 | future neural-state producer through a read-only NCP path | current neural simulator integration | exact branch/commit, upstream delta, executable model, E4 contract |
| `rerun` fork | no direct Prisoma-specific edge verified in the inspected public surface | E0 | upstream-family visualization/storage context | proof that Prisoma’s scientific semantics are solved | use standard Rerun interface first; benchmark any missing semantic layer |
| `molmospaces` fork | no direct edge verified | E0 | possible external robot-learning ecosystem or transport setting | supported benchmark integration | exact use case, adapter, license/model/data provenance, E4 conformance |

## 6. Important negative findings

### 6.1 The profile graph is not an integration graph

A profile image or README diagram can be valuable architecture communication, but it does not establish a dependency, compatible schema, executable data path, or validated scientific result. The plan now treats such diagrams as E0 evidence. This is particularly important when several repositories are maintained by the same person: shared intent and naming make overinterpretation easy.

### 6.2 Galadriel is a comparator, not independent corroboration

Galadriel is scientifically relevant because it offers a different monitoring stack—NIS/CUSUM, signed correlation, conservative fusion, and optional PID evidence. It also documents unusually honest limitations: current Crebain data lack a common projection, missingness is informative, compilation is not live integration, and current evidence is synthetic or fixture-bounded.

However, Galadriel and Prisoma share `pid-rs`. Agreement in their PID outputs can reflect the same implementation. A comparison should treat shared PID results as one correlated method family and focus on genuinely distinct evidence: signed correlation, NIS/CUSUM, independent intervention outcomes, and non-PID baselines.

### 6.3 Crebain’s NCP surfaces do not create an end-to-end loop

Crebain documents dormant, opt-in NCP surfaces and explicitly states there is no always-on Crebain–Engram loop. No direct Prisoma reference was verified. Crebain is still valuable as a candidate producer because it offers multimodal tracking, fusion, and timing complexity that can stress the experiment contract. The correct first experiment is a read-only export/replay adapter with injected sequence, frame, and latency faults—not a claim that an ecosystem loop already exists.

### 6.4 Manwe is a useful incompatibility case

Manwe explicitly says it does not ship a drop-in adapter for Prisoma and lists mismatches in schemas, tensors, clocks, coordinate frames, and statistical assumptions. This is positive evidence for the need for Prisoma’s adapter contract. A successful Manwe-to-Prisoma conformance exercise could make a strong infrastructure benchmark precisely because compatibility is not assumed.

### 6.5 Engram cannot be a thesis dependency

The public Engram repository is a placeholder stating that code will be open sourced after publication. NCP describes an illustrative commander role, but that is not an executable public dependency. Any PhD path that requires Engram before publication inherits an unacceptable availability and reproducibility risk. Engram may become a later producer after a public immutable release and E4 conformance; it cannot block Papers A or B.

### 6.6 WorldWarp and GauSS-MI are specifications, not implemented capabilities

Prisoma’s documents describe optional integration ideas, and the GauSS-MI document labels itself pre-implementation. World-model generation and reconstruction-weighted information estimation also introduce major support, realism, compute, licensing, and estimator-validity questions. They belong in separate exploratory studies after the core intervention and prediction programme works.

### 6.7 Scientific lineage is not present validation

The ReScience replication and BROJA activation-function work demonstrate relevant experience and provide candidate fixtures. They use different questions and, in the BROJA case, a different PID measure. They do not validate continuous shared-exclusions, `pid-rs`, VLA source/target definitions, or Prisoma’s infrastructure. Their strongest use is a cross-measure mechanism-discrimination study with explicit ground truth.

## 7. Dependency firebreak

The following must pass before a thesis release can claim that the core is independent of the surrounding ecosystem:

1. the capture/intervention/replay core builds and executes without NCP;
2. H1 and H2 run with PID disabled and without PID atoms;
3. a local-file or established standard-format adapter can replace every sibling repository;
4. no private repository, unpublished model, personal token, or sibling checkout is needed;
5. optional viewers, assets, world models, and reconstruction systems may fail without changing treatment assignment, primary outcomes, or existing provenance;
6. candidate producers cannot read holdout labels, future treatment schedules, or fitted analysis transforms;
7. every cross-repository artifact is content-addressed and revision-pinned;
8. dependency licenses, model terms, and data rights are machine-readable where possible;
9. CI includes a PID-disabled and NCP-disabled minimum path;
10. documentation renders unavailable optional components as unavailable, not nominal.

## 8. Adapter promotion contract

A candidate adapter may advance to E3/E4 only after one versioned report supplies all of the following:

### 8.1 Provenance and supply chain

- exact producer and consumer revisions;
- lockfiles, toolchains, build flags, containers or environment manifests;
- SBOM, licenses, model/data terms, and integrity hashes;
- clean/dirty worktree status and patch provenance.

### 8.2 Data semantics

- source and target schemas;
- units, dtypes, shapes, ranges, missingness and sentinel values;
- identity semantics for episode, object, sensor, policy, model, and embodiment;
- source population and sampling law.

### 8.3 Time and sequence

- clock domains and synchronization method;
- measured uncertainty and drift;
- sequence semantics and process-epoch identity;
- buffering, batching, reordering, duplication, gap, and drop policies;
- restart and reconnect behavior.

### 8.4 Geometry and control

- coordinate frames and transform lineage;
- calibration provenance;
- action conventions, controller transformations, and execution feedback;
- separation of policy proposal, controller output, executed command, and physical result.

### 8.5 Experimental semantics

- assignment, treatment attempt, receipt, manipulation check, and outcome boundaries;
- which variables are pre-treatment;
- prevention of holdout and future leakage;
- explicit statement that the adapter preserves or changes the estimand.

### 8.6 Security and noninterference

- authentication, authorization, transport security, least privilege, and retention;
- read-only proof for observers;
- queue/backpressure limits and control-timing noninterference;
- threat model for malformed, replayed, oversized, delayed, and unauthorized inputs.

### 8.7 Tests and performance

- golden fixtures;
- malformed, delayed, duplicated, reordered, truncated, version-incompatible, and crash-recovery cases;
- replay-equivalence checks;
- latency, throughput, memory, disk, and loss at the scientific operating point;
- deterministic or tolerance-bounded expected outputs.

Passing a build is not enough. E4 requires scientific semantics and a result-bearing end-to-end path.

## 9. High-value ecosystem experiments

The ecosystem can create distinctive experiments without capturing the thesis scope.

### 9.1 Protocol-fault observatory

Use a conforming read-only NCP producer and inject controlled delay, loss, reorder, duplicate, version mismatch, disconnect, restart, identity collision, and security-profile faults. Measure detection, provenance completeness, replay behavior, data loss, and control noninterference. This directly benchmarks Prisoma’s infrastructure claim.

### 9.2 Cross-monitor comparison

On the same randomized faults and held-out task families, compare:

- ordinary uncertainty/OOD and temporal baselines;
- Galadriel’s NIS/CUSUM and signed-correlation evidence;
- Prisoma diagnostic families;
- PID only where eligible.

Use identical information access and latency budgets. Treat shared `pid-rs` outputs as one implementation family.

### 9.3 Manwe adapter challenge

Use Manwe’s documented incompatibilities as a prospective adapter benchmark. Measure engineering effort, schema defects detected, frame/clock errors prevented, replay fidelity, and whether the scientific estimand changes. This is more defensible than showcasing an easy sibling integration.

### 9.4 Asset-controlled manipulation benchmark

Select a frozen, license-cleared subset of Cobot Atlas assets. Audit scale, collision geometry, visual duplicates, provenance, and physical plausibility. Construct controlled appearance, shape, layout, and occlusion factors while keeping task semantics fixed. Relief Atlas should remain a later transport/stress domain because its size and humanitarian context add quality, licensing, and ethical complexity.

### 9.5 Reconstruction uncertainty as a nuisance variable

Use Melkor to generate or inspect scenes, but measure reconstruction quality independently. Test whether reconstruction error predicts diagnostic failure or modifies intervention effects. Do not call reconstruction uncertainty PID uncertainty, and do not weight information estimators without a separately validated estimand.

### 9.6 Cross-measure mechanism fixtures

Reproduce selected controlled systems from the ReScience/BROJA lineage. Compare BROJA, shared-exclusions, MI/CMI, and direct interventions. The endpoint is qualitative mechanism discrimination under known ground truth—not agreement of atom names or numerical magnitude across inequivalent measures.

### 9.7 Neural-stream producer trial

After selecting and pinning a specific NEST-fork branch, export a small documented neural-state fixture through a read-only NCP path. Validate variable meaning, clock and sequence semantics, provenance, replay, and observer noninterference before any neuroscience or embodied-agent conclusion.

### 9.8 World-model support study

Treat WorldWarp as an external generator in a separate study. Compare generated interventions with simulator-ground-truth interventions using explicit support, identity preservation, realism, and downstream-policy shift metrics. Generated counterfactuals are not causal ground truth by default.

## 10. Priority order

The rational order is:

1. freeze Prisoma’s canonical event and intervention contracts;
2. upgrade or deliberately retain the `pid-rs` pin through a recorded migration decision;
3. prove the PID-disabled and NCP-disabled core path;
4. execute the NCP protocol-fault observatory;
5. implement one difficult but bounded external adapter, preferably Manwe or a standard robotics container;
6. compare diagnostic families on randomized interventions and held-out failures;
7. add Galadriel as an independent non-PID comparator where its inputs are valid;
8. add controlled assets and reconstruction nuisances;
9. consider NEST/Engram and world-model studies only after E4 contracts exist.

This order maximizes publishable evidence per dependency and minimizes the risk that the PhD becomes an integration programme for sibling repositories.

## 11. Required wording in papers and documentation

Use language proportional to evidence:

- “Prisoma pins `pid-rs` as its estimator implementation” is supported.
- “Prisoma includes an optional read-only NCP observer pinned to `v0.7.1`” is supported.
- “Galadriel is a relevant comparator sharing `pid-rs` and NCP” is supported.
- “Crebain and Manwe are candidate producers requiring adapters” is supported.
- “WorldWarp and GauSS-MI are optional specifications” is supported.
- “The ecosystem is integrated end to end” is not supported.
- “Multiple repositories independently validate PID” is not supported.
- “The profile graph proves interoperability” is not supported.
- “Passing CI proves scientific validity or deployment safety” is not supported.

## 12. Source register

Primary public sources inspected include:

- Prisoma snapshot: https://github.com/sepahead/prisoma/tree/64bd881248463e7142d022bb95a5850bcf8fced2
- `pid-rs` pinned revision: https://github.com/sepahead/pid-rs/tree/8a5a9dda601556443f956a2fba164cccc913ed2e
- `pid-rs` post-pin revision: https://github.com/sepahead/pid-rs/commit/70b45f7b75fac06777ea215a73df01209490311a
- NCP `v0.7.1`: https://github.com/sepahead/NCP/tree/v0.7.1
- Galadriel: https://github.com/sepahead/galadriel
- Crebain: https://github.com/sepahead/crebain
- Manwe: https://github.com/sepahead/manwe
- Engram: https://github.com/sepahead/engram
- Melkor: https://github.com/sepahead/melkor
- Cobot Atlas: https://github.com/sepahead/cobot-atlas
- Relief Atlas: https://github.com/sepahead/relief-atlas
- Cortexel: https://github.com/sepahead/cortexel
- Haldir: https://github.com/sepahead/haldir
- BROJA activation-function project: https://github.com/sepahead/brojapid-activationfunctions
- ReScience reproduction repository: https://github.com/sepahead/mahmoudian-2020-rescience
- NEST fork: https://github.com/sepahead/nest-simulator
- public repository index: https://github.com/sepahead?tab=repositories

---

## Final decision

The ecosystem strengthens Prisoma when it is treated as a set of adversarial adapter and transport opportunities under one auditable contract. It weakens the thesis when diagrams, shared ownership, or planned interfaces are presented as completed integration. The v12.5 plan adopts the stronger interpretation: only `pid-rs` and NCP are direct at the reviewed snapshot; every other edge must earn promotion through reproducible software and scientific conformance evidence.
