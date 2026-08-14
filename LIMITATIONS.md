# Prisoma 0.9 limitations

**Release scope:** Prisoma 0.9.0 public GitHub source prerelease and research-software preview

**Author:** Sepehr Mahmoudian
**Canonical research specification:** [`grandplan.md`](grandplan.md), docset v13.0

The Prisoma 0.9.0 source prerelease contains tested software groundwork and explicit research
protocols. Its internal candidate decision record remains NO-GO and `published:false`; that field
denies candidate-package and scientific promotion, not public source availability. This is not a
scientific-results release, a frozen preregistration, a validated safety system, or a
production-deployment qualification. Passing a command in this repository establishes only the
behavior named by that command on its checked inputs. It does not establish causal identification,
statistical validity, transportability, estimator application validity, or a thesis hypothesis.

The machine-readable W1-W3 state is
[`protocols/world_model_claim_registry_v1.json`](protocols/world_model_claim_registry_v1.json).
The preserved EC1/H1-H4 state is
[`protocols/research_claim_registry_v1.json`](protocols/research_claim_registry_v1.json). The
generated [`docs/CAPABILITY_MATRIX.md`](docs/CAPABILITY_MATRIX.md) is the software/evidence
inventory. It currently has no `validated` rows. If this summary and the canonical specification
ever differ, the more restrictive statement governs until the discrepancy is reviewed.

## Scientific status in this candidate

| Area | Current status | What 0.9 does not establish |
|---|---|---|
| Preserved diagnostic governance | **Not freeze-ready.** Historical v1 remains non-promotable. The active v3 successor is an all-null, unreviewed typed draft contract. Superseded v2 bytes remain historical. | WM0, a diagnostic preregistration, a completed freeze candidate, substantive scientific review, or permission to begin confirmatory capture. |
| Confirmatory holdout | **None registered.** The access ledger contains a genesis record only. | Historical or off-repository non-access, independent custody, blinding, or a first-attempt result. |
| W1 | A native exact-fork decision-contract reference is runnable. | Learned-model forecast validity, calibrated ranking, held-out support, or a useful-margin result. |
| W2 | The native reference reconstructs fixed-pool selection and several closed-loop replans. | A randomized complete-policy comparison, planning benefit, or an M4 resource result. |
| W3 | Linked dynamics, mesh/3DGS, policy-response, controller, and selection panels are specified. | An implemented renderer treatment, matched fidelity result, or new component-level priority claim. |
| EC1 | Partial run-log, replay, bridge, Rerun-conversion, and synthetic SAFE-ingress groundwork. | Registered capture–replay fidelity and fault detection as an externally benchmarked claim, or completeness beyond the frozen universe. |
| H1-A | A deterministic finite synthetic Protocol-A scoring reference and common preflight are runnable. | Real paired intervention-response evidence, a physical individual effect, or generalization beyond the fixture contract. |
| H1-B | The randomized closed-loop protocol is specified but unimplemented. | Randomized assignment, intention-to-treat or effect-modification evidence, or closed-loop robustness. |
| H2 | A deterministic synthetic fixed-horizon/IPCW/alarm arithmetic reference is runnable. | A proper observed-data score, a frozen aligned prediction-object contract, prospective prediction, calibration validity, warning benefit, comparator superiority, safety gain, or deployment validity. |
| H3 | **Not eligible.** Population is open/unfrozen. Measure is not adjudicated. The current atom-estimator and continuous-application gates are blocked. High-dimensional MI/coherence is NO-GO. | Interpretable PID atoms on real embeddings, eligible-only promotion, or full-target held-out incremental policy value. |
| H4 | A small reference-model attribution path exercises logging and a group-level deletion-ranking-sensitivity control. | Causal or mechanistic faithfulness, representational availability, natural policy use, or divergence between availability and the effect of a tested intervention in a real VLA. |
| NCP observer | Optional, workspace-excluded, read-only wire-0.8 experimental component. | Final protocol interoperability, a live Engram integration, transport completeness, security validation, EC1, or a scientific result. |

The detailed claim definitions and stop rules are in
[`grandplan.md` §4](grandplan.md#4-confirmatory-claim-template-registry),
[`§7`](grandplan.md#7-estimator-and-measure-validation), and
[`§12`](grandplan.md#12-milestones-gates-and-stop-rules).

## Public claim boundary

The following language is deliberately narrower than the project objectives.

| Claim | Permitted for 0.9 | Prohibited for 0.9 |
|---|---|---|
| W1 | The native affine reference verifies one deterministic exact-fork forecast-commit, branch-label, selection, bridge-execution, and replay contract. | “W1 passed,” learned-model quality, physical forecast validity, causal transition validity, supported ranking benefit, or M4 model qualification. |
| W2 | The native reference reconstructs one bounded multi-replan score-based selection fixture. | “W2 passed,” randomized complete-policy benefit, deployed-policy regret, deployment readiness, or an M4 resource result. |
| W3 | The linked matched-panel protocol is specified. | Implementation, validation, a complete factorial or additive causal decomposition, global priority, or physical truth from a mesh or Gaussian splat. |
| EC1 | Canonical run-log, local replay/conversion, and bounded content-addressed SAFE synthetic-ingress paths are implemented for the tested fixtures. | “EC1 is complete,” “provenance-complete beyond the registered universe,” “externally validated,” or “deployment ready.” |
| H1-A | The schema-v2 preflight input contract, schema-v3 result artifact, and deterministic finite-benchmark Protocol-A software reference are fixture-runnable scoring primitives. They establish no H1-A evidence and cannot establish H1-B. | Unqualified “H1 passed,” “a physical individual effect was observed,” or “closed-loop robustness was established.” |
| H1-B | A randomized closed-loop design is specified in the canonical plan; execution remains blocked. | Any statement that Protocol B was implemented, randomized, analyzed, or validated. Protocol A may not be substituted for Protocol B. |
| H2 | The deterministic synthetic reference exercises the named fixed-horizon, grouped fitting, IPCW risk-estimator, reliability-bin, alarm, nondetection, and declared-payoff arithmetic on checked fixtures only. | “H2 passed,” an `IPCW` label establishes propriety, a likelihood for another prediction object is interchangeable, or any claim of prospective prediction, calibration validity, warning benefit, censoring-assumption validity, comparator superiority, transport, safety gain, or deployment validity. |
| H3 | PID estimates abstain or remain noninterpretable outside their named population, measure, estimator, and application gates. The primary target is the full policy with exact M1 fallback. | Any claim that eligible-only performance, geometry, a nonzero atom, or an emitted number establishes real-embedding PID validity or incremental value. |
| H4 | The reference attribution path exercises canonical logging and a deletion-ranking-sensitivity control. | Any claim that the control establishes causal/mechanistic faithfulness, natural policy use, a tested-intervention effect, or H4. |

“Specified,” “implemented,” “tested,” and “validated” are different states. In particular, a
locally tested feature can remain E0 relationship evidence; an immutable external dependency can
support E2 without proving integration; and an E3 fixture does not become E4 independent
conformance by being maintained in the same project. See
[`docs/CAPABILITY_MATRIX.md`](docs/CAPABILITY_MATRIX.md) for the current per-feature labels.

## What the software proofs cover

The most direct local checks are:

```bash
just docs-audit
just research-governance
just capability-matrix-check
just runlog-sidecars-proof
just runlog-rerun-proof
just safe-adapter
just h1-preflight
just h1-protocol-a
just firebreak
just h2-reference
just world-model-reference
just estimate-report-contract
just exp0-bin
just attribution-probe
just bridge-security
```

These commands check deterministic fixtures, fail-closed parsing or invariants, local replay,
content bindings, or protocol arithmetic. They do not create missing data, assign treatments,
register a holdout, perform independent review, validate a population estimand, or authorize a
public scientific claim. `just research-governance` validates the honesty and internal
consistency of the unfinished M0 state; its freeze-ready mode is expected to fail. `just
exp0-bin` reports the estimator gate outcome, including the current negative or blocked regimes;
it is not a success criterion that must be forced to GO.

## M0, holdout, and external-evidence limitations

The real study cannot be frozen by filling null fields in the historical v1 scaffold. The checked
v3 successor draft now types and content-binds the missing contract surface, including EC1 finite
acceptance with complete detection/replay/false-positive coverage and a mandatory, separately
estimated absolute sensitivity floor for every registered fault–adapter pair—never an aggregate
substitute—H1-A's typed response, comparator, positive-margin one-sided success, calibration,
uncertainty, multiplicity, and scope contract; H1-B's primary endpoint, hierarchy,
positive-margin success, mandatory design checks, and directional replication; H2's aligned
prediction-object/target/censoring/one-primary-scoring-contract/non-rescuable-success contract,
H3's full-population fallback policy, positive incremental-value margin, one-sided superiority
decision, dependence-aware uncertainty, replication target, and warning dispositions, plus H3/H4
selection and a target-specific source-ancestry contract that excludes post-landmark observations
and target injection. It also types H4 target sampling, transport, one tuple/outcome, simultaneous inference,
uncertainty when target weights are estimated, exact fixed weights for an enumerated finite target,
and joint power. A future candidate populates EC1, H2, one selected H1 contract, and one selected
H3-or-H4 contract. Inactive protocol slots stay null. The 2026-08-12 scoring correction reopened
review. The ancestry-contract change moved the active draft from v2 to v3.
Every freeze-bearing v3 value remains null. A real freeze still requires a new review and a
separate completed candidate binding the target population, policy, embodiment,
environment, intervention, outcomes, time origin, units, estimands, minimum useful effects,
splits, multiplicity, power/precision design, missingness, rights, and analysis environment, plus
the required review decisions and immutable receipts. No such signatures or decisions are
represented in 0.9.

The H3 ancestry role bindings are structural placeholders, not implemented roles. Prisoma has no
ancestry producer, consumer validator, or per-row receipt schema. An H3 candidate therefore keeps
the stable implementation blocker, and the validator rejects an H3 `frozen` state. This prevents
ordinary prose or unrelated code from satisfying the ancestry contract by path and hash alone.

Before confirmatory analysis, an independent custodian must register and control a real holdout,
publish its commitment, and preserve the first frozen-candidate result including failures and
abstentions. The current local hash-chain cannot prove prior non-access. The literature ledger is
a legacy reference inventory, not a fresh reproducible search with saved queries, databases,
criteria, candidate universe, and screening decisions.

No real policy/environment/intervention pilot, real SAFE capture, real prospective H2 capture,
external or later-time holdout, independent EC1 reproduction, second structurally different EC1
adapter, or externally benchmarked conventional-stack comparison is included.

## Estimator and PID limitations

Prisoma pins the canonical `pid-rs` 0.9.0 post-tag review source at submodule commit `796c11e`.
That review surface makes no 1.x compatibility promise and carries no registry or published-wheel
promise. The exact pin is a dependency identity, not independent corroboration. The
high-dimensional MI/coherence route is NO-GO, and continuous shared-exclusions PID on real VLA
embeddings is not application-validated. An output may be computed only when declared support
permits it; an abstention has no numeric placeholder and must not be interpreted as zero.

Public `pid-rs` main was observed at `7473e62` on 2026-08-13. Its estimator-code parent is
`cb3f58f0`; the child changes custody and assurance surfaces only. Newer method catalogs,
formal/categorical assurance work, source-errata records, and exact-certifier surfaces remain
unadopted. Full exact-head CI is red in two jobs, while a narrower push receipt passed. Provenance
improvements do not establish application validity.

Population, measure, estimator, and application verdicts are separate. Geometry diagnostics and
sampled-mean delta are descriptive and cannot clear those gates. The fitted categorical route
constructs empirical categorical laws and targets the averaged two-source MGW shared-exclusions
functional. Continuous Ehrlich shared exclusions, MGW categorical shared exclusions,
Williams–Beer `I_min`, BROJA, finite-sample estimators, and infomorphic objectives are related but
non-interchangeable objects. Prisoma does not use `I_min` or BROJA for an active hypothesis.
Categorical reports retain empirical occupancy and a coverage warning indicator. Neither observed
coverage nor a low singleton fraction proves coverage of the population law or removes plug-in
bias.
Quantization, PLS, scaling, and other fitted transformations must be fitted inside training folds
for any future held-out comparison. H3 requires an eligible
episode-local feature and a task-family-blocked M2-over-M1 comparison after the non-PID H1 or H2
problem is established. Its primary comparison must retain the complete frozen target ledger and
use the exact same-fold M1 output for each abstention. Neither path exists in 0.9.

## World-action-model limitations

`VLA` and `WAM` do not define exclusive model classes. Prisoma classifies the deployed directed
graph. A predictive loss does not prove runtime future use. Intended-future conditioning does not
prove action-conditioned dynamics. Coupled joint generation does not expose a candidate-action
forecast query by algebraic factorization alone.

Action-conditioned prediction is not an interventional transition by architecture. It needs
randomized executed actions, support checks, execution receipts, proper scores, and calibration.
Generated-video quality and task success cannot replace these tests.

An action-conditioned state also cannot be a PID source for that exact proposal target. That design
injects the target into the source. Cross-fitting does not remove the defect. A downstream command,
later declared reference-state outcome, or separately measured physical outcome remains eligible
only when the matched baseline gets the same proposal. Command or simulator-state prediction is
not physical forecast validity. Each target needs a frozen prediction
landmark before that target becomes available. Each source needs an ancestry receipt that proves
availability by that landmark.
The current shared artifact schema and offline harness do not validate that receipt. H3 remains
blocked until a typed producer record and consumer check exist.

Forward prediction and global latent non-collapse do not prove physical-state identity,
action-sensitivity, or embodiment transfer. A finite-horizon world branch also does not supply
task memory, failure recovery, or a correct replan schedule.

Asynchronous chunk execution does not remove latency by itself. It changes command scheduling and
can execute stale or misindexed actions. Prisoma has no qualified asynchronous WAM controller.
Any future claim must bind observation, inference, dispatch, and execution times and test delay
tails rather than report model latency alone.

The native affine world-model reference proves only exact-fork decision-contract semantics. Its
learner and reference branches use the same deterministic law. It is not a qualified learned
world model and cannot establish W1 or W2.

Run-log schema 2 has no neutral inline decision-record event. The reference therefore stores its
forecast commitment and execution receipt in strictly named `label_observed` compatibility
envelopes. They are not outcome labels. Its verifier enforces their exact shapes and order. A
future `pid-runlog` schema should add a content-bound decision record before this path becomes a
general learned-model adapter.

No reviewed external WAM has a qualified Prisoma adapter. No predictive candidate has passed the
local MPS, parity, latency, memory, rights, checkpoint-loader, and hook gates. Flex-\(\pi\) had no
runnable code or checkpoint at the review cutoff. VLA-JEPA's LeRobot code contains MPS-specific
handling and safetensors branches, but that is not an end-to-end support claim. It has not passed the local
qualification gates. Its policy path does not call
the predictor. Its stock world-loss path also uses a future video window and learned Qwen latent-
action tokens rather than clamped robot actions. Its pinned two-frame tubelet encoder makes even
the earliest context depend on `t+1`. No stock prediction position is admissible as a row-aligned
pre-action `D`. The compact LeWorldModel PushT planner is the first external port candidate because
it exposes a candidate-action-conditioned latent query and CEM selection loop in a smaller stack.
Its reviewed evaluator hard-codes CUDA. The documented conversion uses pickle-enabled loading, so
Prisoma must instantiate the architecture and load a digest-bound state dictionary with
`weights_only=True`. A one-seed independent reproduction covers only TwoRoom. It reports
outcome-relevant conventions outside configuration files and conflicts among released evaluation
settings. It does not test PushT, M4, other tasks, or seed variance. This requires a frozen
paper/configuration/code concordance ledger. It is not evidence for or against W1 or W2. JEPA-WM
remains a larger, noncommercial second planning benchmark.
LiLa-WAM has no language input, no source-code license, and no MPS qualification. Its released
inference loop does not call its future decoder. Efficient-WAM has released source and weights,
but its reviewed joint sampler asserts CUDA before its nominal attention fallback and uses
float64/complex RoPE. Its model repository has no declared weight license.

Social-media claims are discovery leads only. The supplied Dyna-2 post points to a company report.
That report contains a matched internal objective study and a separate matched architecture study.
Neither is independently reproducible. Neither identifies online future simulation or planning.

## Data rights, privacy, and ethics limitations

The checked SAFE path uses synthetic canonical NPZ/JSON fixtures. It does not establish rights to
download, extract activations from, redistribute, or publish any real dataset, model,
checkpoint, prompt, image, language trace, annotation, or derived embedding. A public dataset
label alone does not grant those rights.

Before real capture, the study still needs documented controller/processor roles, source and
model licenses, redistribution terms, export restrictions, human-subject and personal-data
classification, institutional review where required, consent or another lawful basis, data
minimization, redaction and pseudonymization, embedding re-identification assessment, access and
encryption controls, retention/deletion/withdrawal rules, and incident response. The current
transport/contamination ledger is structurally present but has no selected real dataset or target
assessment.

Do not place secrets, private holdout membership, scoring answers, personal identifiers, or
unredacted sensitive media in run logs or generated artifacts.

## Security and deployment limitations

The Agent Bridge is the only intended control plane, but its present network transports are
local research tooling:

- TCP and WebSocket binaries refuse non-loopback bind addresses and default to safe mode, but
  forwarding, tunnelling, or proxying a loopback listener is not prevented.
- Standard profiles have no authentication. The Engram profile verifies possession of an
  operator-pasted startup secret only. It does not authenticate a user, process, build, commit, or
  actor identity. No profile provides authorization, TLS, credential custody, redaction, or a
  remote-security assessment. Caller identity is locally declared.
- TCP/stdio lines and WebSocket upgrades/frames have per-message caps. Network reads and writes
  have per-operation timeouts. Standard profiles have no total request/session, request-count,
  or aggregate-traffic limit. Progress-making trickle traffic can persist. The optional Engram
  profile adds finite request-count, aggregate-input, and run-log limits. It has no independent
  wall-clock deadline.
- The WebSocket and JSON-RPC implementations intentionally support narrow subsets. They are not
  general HTTP/WebSocket or JSON-RPC conformance claims.
- File RPCs reject observed traversal, symlinks, non-regular or out-of-root inputs, missing output
  parents, and existing outputs under a non-adversarial canonical-confinement model. This is not
  a security-grade sandbox against hardlinks, aliases, or concurrent filesystem mutation.
- Transport logs and export outputs use no-clobber staging and file synchronization on named
  paths, but there is no parent-directory fsync guarantee, power-loss guarantee, or cross-file
  transaction. A crash or storage failure can leave incomplete provenance or an orphan output.
- Logging an intervention does not make it safe. Physical safety, policy authorization,
  emergency handling, and independent deployment controls are outside the demonstrated scope.

`just bridge-security` is a local unit proof for the enumerated behavior above. It is not a
penetration test, adversarial-filesystem assessment, safety case, or authorization to expose the
bridge remotely.

## NCP and ecosystem limitations

The optional NCP observer is built separately against immutable NCP wire 0.8. The deterministic
fault observatory exercises a finite local fixture and records a known whole-tick-omission blind
spot; it does not measure live timing, delivery completeness, QoS, reconnect behavior,
authentication, ACL enforcement, or producer noninterference. The observer's visible-receipt
capture grade is a join/publication grade, not proof that every source event was delivered.
Schema-1 publication receipts accept only the frozen `v0.8.0` tag, revision, wire, and compact
hash; another wire needs a separately reviewed receipt schema and consumer.
Official NCP main was observed at `1a04294c90c1b50eba06ae1c6afe9c951319250d` on
2026-08-13. That commit is the unreleased, release-blocked `1.0.0-rc.1` candidate (wire
1.0; compact proto contract hash `163acc57d8a62b66`). The latest immutable release is `v0.8.0`,
which uses a different wire. NCP ledger tasks `P01`, `P02`, and `P03` are OPEN, not
dependency-ready, and **NOT RUN**. `P03` covers fault-observatory migration and Prisoma
observer-role qualification. The refined low-overhead architecture and prepared-stream-monitor
gap record are coordination-only. B01 remains `IN_PROGRESS` with no passing receipt. See the
[verified NCP task ledger](https://github.com/sepahead/NCP/blob/1a04294c90c1b50eba06ae1c6afe9c951319250d/evidence/implementation/task-ledger.v1.json).
The Zenoh 1.9 dependency graph retains `lz4_flex` 0.10.0, which is affected by the
high-severity RUSTSEC-2026-0041 block-decompression information disclosure. The checked profile
does not enable Zenoh's `transport_compression`, so the affected call is cfg-elided, and CI fails
if that feature appears. The vulnerable package is still present in the optional lock: this is
not a clean audit, does not qualify the NCP binary for release or live use, and must be removed by
a qualified NCP/Zenoh pin admitting `lz4_flex` 0.11.6 or newer. The graph also retains the
unmaintained (not known vulnerable) `rustls-pemfile` 2.2.0 because no compatible replacement
exists. The observer graph also retains the unmaintained `paste` 1.0.15 proc-macro through Zenoh.
Rapier 0.34 removed `paste` from the root graph. `deny.toml` records these temporary observer
exceptions.

No reviewed public live producer currently supplies the honest language, split, episode, and
outcome structure needed by the research path, and the public Engram repository is not a live
integration. This is a bounded statement about reviewed public sources. NCP remains optional and
outside the default workspace. Galadriel, Haldir, Crebain,
WorldWarp, and other ecosystem candidates are not required for the core claims and are not
integrations merely because they are named or share maintainers.

## Interoperability, visualization, and product limitations

The repository does not yet contain the required MCAP/rosbag2 or LeRobot/RLDS adapters, a second
independent EC1 adapter, or the external conventional-stack benchmark. Local JSON/NPZ fixtures do
not substitute for those deliverables. The run-log-to-Rerun converter is runnable, but the full
Phases 1–3 diagnostic viewer is not built. The Tauri/SparkJS shell and custom renderer are
deferred product surfaces. A successful conversion, displayed plot, or screenshot does not prove
outcome blinding, replay correctness, estimator validity, or a scientific claim.

External attribution-artifact loading is default-off and unavailable to bridge export. The
standalone converter's explicit opt-in enforces relative regular, non-symlink paths, bounded exact
NumPy framing, finite values, canonical shape metadata, exact SHA-256 binding, and batch preflight
before its first Rerun write. It rejects timestamps beyond Rerun's signed-nanosecond range and caps
one conversion at 100,000 events, 64 MiB of serialized event content, 64 MiB of
application-generated entity paths, 250,000 projected log calls, 16 MiB for a supplied compact
manifest, and 8 MiB of retained relevance values plus their in-memory shape and identity storage.
These viewer limits are stricter than the canonical run-log reader. Headless saves explicitly finalize the
encoder and install a staged, file-synced
`.rrd` without replacement, but do not fsync the parent directory. The Python attribution producer
uses no-clobber content-addressed relevance artifacts and reconstructable JSON evidence bundles,
with companion `artifact_logged` events, and replaces the run-log name last. The Rerun track is the
recorded compatibility check, not a validated-faithfulness verdict. A failed publication can leave
an unreferenced new artifact, and no cross-file transaction or power-loss guarantee is claimed.
Path confinement remains a local best-effort boundary rather than a security-grade defense against
hardlinks, aliases, or every concurrent filesystem race.

## Reproducibility and generalization limitations

Locks, exact submodule pins, content hashes, deterministic fixtures, and canonical run logs make
specific local behavior auditable. They do not guarantee identical behavior on every operating
system, hardware target, filesystem, dependency mirror, real robot, policy, simulator, task
family, or future dependency release. Tests written and run by this project are not independent
replication. Generalization may extend only to variation actually represented in a reviewed,
held-out, independently reproduced study.

The definitive evidence-to-claim map for this release is
[`THESIS_EVIDENCE_INDEX.md`](THESIS_EVIDENCE_INDEX.md).
