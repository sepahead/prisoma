# Prisoma Runtime and Adapter Contract

The filename is retained for stable links. Prisoma is not a “PID-Splat” simulator product.
This document defines the narrow contracts that external adapters and viewers must preserve.

[`grandplan.md`](grandplan.md) is canonical. [ARCHITECTURE.md](ARCHITECTURE.md) explains the
component design. [EXPERIMENTS.md](EXPERIMENTS.md) contains executable proof commands.

## 1. Contract status

| Surface | Current status |
|---|---|
| Run log | Schema 2; partial M2/EC1 groundwork |
| Agent Bridge | Implemented local control-plane groundwork |
| Rerun adapter | Implemented conversion groundwork |
| Complete Rerun diagnostic application | Specified, not implemented |
| Tauri/SparkJS shell | Deferred |
| `(V,L,D,A)` offline harness | Implemented bounded software path |
| H1 common preflight | Implemented by `pid-h1-preflight` |
| H1 Protocol-A reference | Synthetic finite benchmark only |
| H2 reference | Synthetic fixed-horizon arithmetic only |
| Real confirmatory capture | Not implemented |

No surface in this table establishes a confirmatory result.

## 2. Global invariants

Every conforming adapter must preserve these rules:

1. The canonical run log is the source of truth.
2. The Agent Bridge is the only control plane.
3. A read-only observer cannot issue actions or interventions.
4. Every evidence artifact binds exact source bytes.
5. Computation status and scientific gate status remain separate.
6. An abstained estimate has no numeric placeholder.
7. Optional integrations stay outside the critical path.

## 3. Run-log contract

The pinned `pid-runlog` crate owns the event schema. Prisoma consumes that schema and does not
define a competing log format.

A finalized schema-2 bridge stream requires:

- One `run_started` event.
- One content-bound configuration.
- Exactly one response for each bridge request.
- Monotone logical event order.
- One terminal event when finalization succeeds.

Every event needed to reconstruct a captured sample must enter this stream. An external database,
viewer cache, or sidecar cannot become the authoritative source.

Derived artifacts must record their URI, content hash, kind, and relevant shape or schema metadata.

## 4. Agent Bridge contract

### 4.1 Method surface

Canonical JSON-RPC methods use dotted names:

- `bridge.describe`
- `bridge.session`
- `sim.status`
- `sim.reset`
- `sim.step`
- `log.start`
- `log.stop`
- `log.replay`
- `scene.set_object`
- `intervention.apply`
- `export.rerun`

`export.rerun` requires the `pid-sim/rerun-export` feature. A default runtime omits it from
`bridge.describe` and `bridge.session`. Wire parsers reject underscore aliases. Legacy run-log
action aliases remain replay-compatible.

The offline and toy harnesses require `pid-sim/analysis`. WebSocket transport requires
`pid-sim/websocket`. These features keep estimator, linear-algebra, and SHA-1 dependencies outside
the default execution graph.

### 4.2 Request model

The bridge accepts one JSON-RPC object per message. It does not accept batches. Parameters are
named objects with exact top-level keys. Duplicate JSON object members reject before typed parsing.

A missing `id` is a notification and produces no wire response. An explicit `null` identifier is
a request and receives a response with `id: null`.

### 4.3 Safe mode

Safe mode permits only static read operations. Mutating methods fail before backend dispatch but
still receive an auditable response event when storage remains writable.

### 4.4 Transport limits

stdio and TCP use bounded JSONL records. WebSocket uses a finite upgrade and frame profile. Network
operations have deadlines. The standard profiles do not have a total progress-making session
deadline.

TCP and WebSocket listeners reject non-loopback bind addresses. This is not protection against a
forwarding proxy.

### 4.5 Engram-host profile

The optional Engram-host profile exposes exactly three read-only methods. Its constructor requires
a pairing secret, so an advertised paired profile cannot start without a pairing guard.

Mutual HMAC-SHA256 proofs bind the profile, connection, request identifier, and fresh nonces. The
profile caps pairing attempts, requests, aggregate input, events, and run-log bytes.

Pairing proves startup-secret possession only. It is not process, binary, build, or host attestation.

## 5. Environment adapter contract

An environment adapter receives only validated bridge operations. It returns deterministic state
or an explicit domain error for the same request.

The current repository includes:

- A deterministic object-state backend.
- A null `PhysicsBackend` adapter.
- A feature-gated `rapier3d-f64` backend.
- A scripted push-to-goal manipulation fixture.

These are local cross-checks. They do not select the future real study environment.

New backends must define:

- Reset semantics.
- Step and timestamp semantics.
- Action and intervention schemas.
- Observation and flow provenance.
- Determinism controls.
- Resource limits.
- Replay tolerances.
- Failure and durability behavior.

## 6. `(V,L,D,A)` producer contract

Each artifact contains a nonempty sample list. Every sample has:

- A unique nonempty `sample_id`.
- Nonempty finite `v`, `l`, `d`, and `a` arrays.
- Optional nonempty `episode_id`.
- A label map.
- A metadata map.

The producer declares population support separately for `V`, `L`, `D`, and `A`. The harness never
infers this support from the sample.

`D` is neutral in the shared contract. Each adapter must declare its meaning and provenance.

Strict held-out analysis uses a boolean `success` label and a recognized `split` value. Stronger
gates can require class coverage, episode disjointness, and accepted axis-provenance markers.

## 7. Offline harness contract

### 7.1 Admission

The harness admits raw bytes, decoded structure, projected distance work, coordinate work, and dense
solver work before analysis. Limits apply to in-memory and file entry points.

The default file reader accepts one bounded regular non-symlink snapshot on supported Unix hosts.
It verifies the same descriptor identity before and after the read and at the lexical path.

### 7.2 Baselines

When labels permit them, the harness computes:

- Overall majority accuracy.
- Leave-one-out 1-NN by axis and joint VLDA.
- Episode-excluded 1-NN.
- Train-split majority and 1-NN.
- Train-standardized nearest centroids.
- A held-out VLDA logistic baseline.

These are static factual-outcome baselines. They are not the H1 response or H2 prospective endpoint.

### 7.3 PID modes

`none` requests no MI or PID estimates. `continuous` requests KSG-based continuous shared
exclusions. `discrete` requests quantized Williams–Beer `I_min`. `discrete-pls` fits PLS before the
same quantized measure.

These modes name different analysis contracts. The harness never pools their atoms or uses one as
an automatic fallback for another.

### 7.4 Reports

The report records:

- Exact input identity.
- Estimator and measure identity.
- Preprocessing provenance.
- Computation outcomes and abstention reasons.
- Four independent scientific gates.
- Geometry and temporal diagnostics.
- Baseline records and failure diagnostics.
- Applied resource limits and projected usage.

Optional uncertainty stays in a separately content-bound sidecar. It cannot alter the main report
after publication.

## 8. H1 and H2 reference contracts

`pid-h1-preflight` consumes one bounded schema-v2 fixture. It verifies timing, lineage, reset, RNG,
fold, clone, and instrumentation-noninterference obligations.

`pid-h1-protocol-a` consumes the exact passed preflight chain. It runs a deterministic finite
benchmark and emits no H1 evidentiary claim.

`pid-h2-reference` binds four planning artifacts and one bounded dataset. It exercises fixed-horizon
censoring and score arithmetic. It emits no prospective H2 claim.

Each reader preserves a typed failed artifact for readable invalid input when its CLI contract
promises one.

## 9. Viewer contract

The implemented Rerun adapter consumes a bounded validated run-log snapshot. It writes headless RRD
output without replacement or streams to a matching viewer version.

Attribution artifact loading is opt-in. It validates bounded NumPy arrays, recorded hashes, shapes,
and local path confinement before the first viewer write.

The complete diagnostic panel set is not implemented. A future viewer must remain read-only and
must derive all state from canonical evidence.

## 10. Optional rendering and comparator interfaces

Gaussian splats, reconstruction-quality covariates, world-model comparators, and custom product UI
are optional studies or deferred surfaces. They are not required by the runtime contract.

If an optional adapter is added, it must:

1. Pin its implementation and model revision.
2. Record rights and input receipts.
3. Bind exact inputs and outputs.
4. State its support and observation law.
5. Route all mutations through the Agent Bridge.
6. Remain removable without breaking core capture, replay, or PID-disabled baselines.

## 11. Failure policy

Reject malformed, oversized, non-finite, ambiguous, or scientifically unsupported inputs before
expensive work. Preserve a typed reason. Do not invent a result to keep a pipeline running.

Storage failures can leave incomplete logs or partial output sets. The system does not claim a
multi-file transaction, parent-directory durability on every path, or crash-proof finalization.

## 12. Acceptance rule for contract changes

A contract change requires all of the following:

- Updated typed schemas and focused compatibility tests.
- Updated capability-catalog evidence inputs.
- Regenerated content-bound matrices.
- Updated current docs in the same change.
- Explicit migration notes for source or wire breaks.
- Full locked quality gates.

Do not preserve a stale interface only because this legacy filename once described a larger product.
