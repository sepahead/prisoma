# Prisoma 0.9 limitations

**Release scope:** Prisoma 0.9.0 source and research-software preview

**Author:** Sepehr Mahmoudian
**Canonical research specification:** [`grandplan.md`](grandplan.md), docset v12.5

Prisoma 0.9 packages tested software groundwork and explicit research protocols. It is not a
scientific-results release, a frozen preregistration, a validated safety system, or a
production-deployment qualification. Passing a command in this repository establishes only the
behavior named by that command on its checked inputs. It does not establish causal
identification, statistical validity, transportability, estimator application validity, or a
thesis hypothesis.

The machine-readable current state is
[`protocols/research_claim_registry_v1.json`](protocols/research_claim_registry_v1.json). The
generated [`docs/CAPABILITY_MATRIX.md`](docs/CAPABILITY_MATRIX.md) is the software/evidence
inventory. It currently has no `validated` rows. If this summary and the canonical specification
ever differ, the more restrictive statement governs until the discrepancy is reviewed.

## Scientific status at release

| Area | Current status | What 0.9 does not establish |
|---|---|---|
| M0 governance | **Not freeze-ready.** The v1 files are an intentionally non-promotable, unfrozen scaffold. | A preregistration, substantive scientific review, or permission to begin confirmatory capture. |
| Confirmatory holdout | **None registered.** The access ledger contains a genesis record only. | Historical or off-repository non-access, independent custody, blinding, or a first-attempt result. |
| EC1 | Partial run-log, replay, bridge, Rerun-conversion, and synthetic SAFE-ingress groundwork. | Provenance-complete replay as an externally benchmarked infrastructure claim. |
| H1-A | A deterministic finite synthetic Protocol-A scoring reference and common preflight are runnable. | Real paired intervention-response evidence, a physical individual effect, or generalization beyond the fixture contract. |
| H1-B | The randomized closed-loop protocol is specified but unimplemented. | Randomized assignment, intention-to-treat or effect-modification evidence, or closed-loop robustness. |
| H2 | A deterministic synthetic fixed-horizon/IPCW/alarm arithmetic reference is runnable. | Prospective prediction, calibration validity, warning benefit, comparator superiority, safety gain, or deployment validity. |
| H3 | **Not eligible.** Population, measure, estimator, and application gates remain blocked. | Interpretable PID atoms on real embeddings or held-out incremental PID value. |
| H4 | A small reference-model attribution path exercises logging and one deletion-faithfulness control. | Representational availability, causal policy use, or an availability–use divergence in a real VLA. |
| NCP observer | Optional, workspace-excluded, read-only wire-0.8 experimental component. | Final protocol interoperability, a live Engram integration, transport completeness, security validation, EC1, or a scientific result. |

The detailed claim definitions and stop rules are in
[`grandplan.md` §4](grandplan.md#4-confirmatory-claim-registry),
[`§7`](grandplan.md#7-estimator-and-measure-validation), and
[`§12`](grandplan.md#12-milestones-gates-and-stop-rules).

## Public claim boundary

The following language is deliberately narrower than the project objectives.

| Claim | Permitted for 0.9 | Prohibited for 0.9 |
|---|---|---|
| EC1 | Canonical run-log, local replay/conversion, and bounded content-addressed SAFE synthetic-ingress paths are implemented for the tested fixtures. | “EC1 is complete,” “externally validated,” or “deployment ready.” |
| H1-A | The schema-v2 common preflight and deterministic finite-benchmark Protocol-A software reference are fixture-runnable scoring primitives and establish no H1 evidence. | “H1 passed,” “a physical individual effect was observed,” or “closed-loop robustness was established.” |
| H1-B | A randomized closed-loop design is specified in the canonical plan; execution remains blocked. | Any statement that Protocol B was implemented, randomized, analyzed, or validated. Protocol A may not be substituted for Protocol B. |
| H2 | The deterministic synthetic reference exercises the named fixed-horizon, grouped fitting, IPCW, reliability-bin, alarm, nondetection, and declared-payoff arithmetic on checked fixtures only. | “H2 passed,” or any claim of prospective prediction, calibration validity, warning benefit, censoring-assumption validity, comparator superiority, transport, safety gain, or deployment validity. |
| H3 | PID estimates abstain or remain noninterpretable outside their named population, measure, estimator, and application gates. | Any claim that geometry, a nonzero atom, or an emitted number establishes real-embedding PID validity. |
| H4 | The reference attribution path exercises canonical logging and a deletion-faithfulness control. | Any claim that attribution agreement proves causal use or establishes H4. |

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

The real study cannot be frozen by filling null fields in the current v1 scaffolds. A reviewed
successor schema and validator must content-bind the target population, policy, embodiment,
environment, intervention, outcomes, time origin, units, estimands, minimum useful effects,
splits, multiplicity, power/precision design, missingness, rights, and analysis environment. It
must then receive the required candidate, supervisor, and independent-review decisions. No such
signatures or decisions are represented in 0.9.

Before confirmatory analysis, an independent custodian must register and control a real holdout,
publish its commitment, and preserve the first frozen-candidate result including failures and
abstentions. The current local hash-chain cannot prove prior non-access. The literature ledger is
a legacy reference inventory, not a fresh reproducible search with saved queries, databases,
criteria, candidate universe, and screening decisions.

No real policy/environment/intervention pilot, real SAFE capture, real prospective H2 capture,
external or later-time holdout, independent EC1 reproduction, second structurally different EC1
adapter, or externally benchmarked conventional-stack comparison is included.

## Estimator and PID limitations

Prisoma pins the canonical `pid-rs` 1.0.0 implementation at submodule commit `ac4a780`. That pin
is a dependency identity, not independent corroboration. The high-dimensional MI/coherence route
is NO-GO, and continuous shared-exclusions PID on real VLA embeddings is not
application-validated. An output may be computed only when declared support permits it; an
abstention has no numeric placeholder and must not be interpreted as zero.

Population, measure, estimator, and application verdicts are separate. Geometry diagnostics and
sampled-mean delta are descriptive and cannot clear those gates. Continuous shared-exclusions
atoms and quantized discrete Williams–Beer `I_min` atoms are different estimands and must never be
pooled or silently substituted. Quantization, PLS, scaling, and other fitted transformations must
be fitted inside training folds for any future held-out comparison. H3 requires an eligible
episode-local feature and a task-family-blocked M2-over-M1 comparison after the non-PID H1 or H2
problem is established; neither exists in 0.9.

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
- There is no authentication, authorization, TLS, credential custody, redaction, remote-security
  assessment, or authenticated actor identity. Caller identity is locally declared.
- TCP/stdio lines and WebSocket upgrades/frames have per-message caps, and network reads/writes
  have per-operation timeouts. There is no total request/session deadline, request-count cap, or
  aggregate-traffic budget; progress-making trickle traffic may persist.
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
The Zenoh 1.9 dependency graph retains the unmaintained (not known vulnerable)
`rustls-pemfile` 2.2.0 because no compatible replacement exists; `deny.toml` records the narrow
temporary exception, which must be removed when a qualified upstream pin permits it.

No conforming public live producer currently supplies the honest language, split, episode, and
outcome structure needed by the research path, and the public Engram repository is not a live
integration. NCP remains optional and outside the default workspace. Galadriel, Haldir, Crebain,
WorldWarp, and other ecosystem candidates are not required for the core claims and are not
integrations merely because they are named or share maintainers.

## Interoperability, visualization, and product limitations

The repository does not yet contain the required MCAP/rosbag2 or LeRobot/RLDS adapters, a second
independent EC1 adapter, or the external conventional-stack benchmark. Local JSON/NPZ fixtures do
not substitute for those deliverables. The run-log-to-Rerun converter is runnable, but the full
Phases 1–3 diagnostic viewer is not built. The Tauri/SparkJS shell and custom renderer are
deferred product surfaces. A successful conversion, displayed plot, or screenshot does not prove
outcome blinding, replay correctness, estimator validity, or a scientific claim.

## Reproducibility and generalization limitations

Locks, exact submodule pins, content hashes, deterministic fixtures, and canonical run logs make
specific local behavior auditable. They do not guarantee identical behavior on every operating
system, hardware target, filesystem, dependency mirror, real robot, policy, simulator, task
family, or future dependency release. Tests written and run by this project are not independent
replication. Generalization may extend only to variation actually represented in a reviewed,
held-out, independently reproduced study.

The definitive evidence-to-claim map for this release is
[`THESIS_EVIDENCE_INDEX.md`](THESIS_EVIDENCE_INDEX.md).
