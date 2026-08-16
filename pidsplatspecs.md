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
| Exact-fork world-model decision reference | Implemented software conformance path |
| External learned-world-model adapter | Not implemented or MPS-qualified |
| H1 common preflight | Implemented by `pid-h1-preflight` |
| H1 Protocol-A reference | Synthetic finite benchmark only |
| H2 reference | Synthetic fixed-horizon arithmetic only |
| Real confirmatory capture | Not implemented |

No surface in this table establishes a confirmatory result.

## 2. Global invariants

Every conforming adapter must preserve these rules:

1. The canonical run log is the source of truth for accepted recorded events.
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
The stream cannot prove an upstream event that its capture boundary never observed.

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

An environment adapter receives only validated bridge operations. It returns a typed result or an
explicit domain error for the same request. It must declare and record its determinism or
stochasticity contract.

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

For predictive policies, the declaration must name the deployed graph. Use separate labels for
predictive-trained current context, intended future, and candidate-action-conditioned prediction.
Use a separate coupled-joint-sampler label when future and action slots update together without a
clamped action query. Do not infer that query by factorizing a joint density. Do not use
`counterfactual` without the randomized executed-action gate. Record policy proposal,
controller output, and executed action separately when they differ. An asynchronous chunk adapter
must also record observation capture, inference start and finish, committed-prefix indices,
dispatch, acknowledgement, and measured end-to-end delay.

Before H3 admission, bind one target-specific prediction landmark before target availability.
Bind the maximum observation time in each tensor's ancestry to that landmark. A source must not
contain its PID target. In particular, reject a
candidate-action-conditioned `D` when that exact proposal is the target. A downstream command,
later declared reference-state outcome, or separately measured physical outcome remains eligible
only when the matched baseline receives the same proposal. Command or simulator-state prediction
is not physical forecast validity. The current artifact schema does not yet
validate this receipt.

Strict held-out analysis uses a boolean `success` label and a recognized `split` value. Stronger
gates can require class coverage, episode disjointness, and accepted axis-provenance markers.

## 7. Offline harness contract

### 7.1 Admission

The harness admits raw bytes, decoded structure, projected distance work, coordinate work, dense
solver work, and fitted-categorical PID work before analysis. Limits apply to in-memory and file
entry points.

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

`none` requests no MI or PID estimates. `continuous` requests KSG-based Ehrlich continuous shared
exclusions. `categorical-sx` fits equal-width quantizers and estimates the averaged two-source MGW
shared-exclusions functional on the resulting empirical categorical laws. `categorical-sx-pls`
fits PLS toward `A` on the same rows used by the fitted categorical screen. Every such estimate
has a `produced_with_warning` status and an estimator-blocked same-row reason. The split screen
uses train rows only. It does not score held-out categorical rows. This route is a descriptive
selection-inflation diagnostic, not an inferential escape hatch.

These modes name different analysis contracts. The categorical routes are not Williams–Beer
`I_min`, BROJA, the continuous Ehrlich functional, or an infomorphic objective. The harness never
pools their atoms or uses one as an automatic fallback for another.

The full method identity and publication rules are in
[`PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md`](PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md).
Every new route must name one functional, one exact quantity or lattice coordinate, and one tagged
evaluator-or-estimator edge. The cumulative or Möbius-inverted construction, component,
aggregation, law kind, source count and order, target, transform, row relation, units, support or
gauge, validation status, and application verdict are mandatory. A mathematically valid extension
may remain a research route, but it cannot inherit the identity or evidence of another PID.

### 7.4 Reports

The report records:

- Report contract `prisoma.offline_vlda.report/5` in its hashed configuration.
- Per-axis support plus one explicit `continuous_tuple_support` value for every requested
  continuous MI/PID tuple. The tuple assertion covers all required marginal and joint laws and
  finite information. Per-axis continuity cannot substitute for it.
- One fitted-quantization receipt per axis for categorical modes. Each receipt binds the defining
  functional, quantizer, estimator route, canonical edge hash, transform hashes, dimensions,
  information units, occupancy, and out-of-range policy. The estimate outcome records nats
  separately.
- Per-pair empirical-PMF occupancy, singleton, low-count, coverage-indicator, and unseen-state
  caveat fields from the pinned MGW estimator. They do not prove population support.
- A private process-local seal over every serialized report field. Publication rejects a changed
  or deserialized report. The seal is mutation detection, not an external signature.
- Within-unit-step-run Pearson lag-1 means and defined-dimension coverage for each axis. These are
  descriptive only. Missing episode identities produce no lag pairs. Each non-singleton segment
  also needs a strict canonical sequence index. Only adjacent rows whose index advances by one
  contribute. The report counts excluded gaps. It centers each contiguous run before pooling
  residual products. A run needs at least three lag pairs because two pairs force Pearson
  correlation to positive or negative one. The screen produces no inferential sample-size or
  block-length suggestion. It reports admitted and correlation-eligible pair counts separately.
- Exact input identity.
- Estimator and measure identity.
- Preprocessing provenance.
- Computation outcomes and abstention reasons.
- Four independent scientific gates.
- Geometry and temporal diagnostics.
- Baseline records and failure diagnostics.
- Applied resource limits and projected usage.

Optional uncertainty stays in a separately content-bound sidecar. It cannot alter the main report
after publication. Sidecar schema 3 records tuple support, row topology, and calibration. It returns a typed skip
when current row transforms would cross dependent episode boundaries. Serial transforms require
one episode and a strictly increasing canonical decimal `metadata.sequence_index`. Restricted
circular shifts produce surrogate tail fractions, not p-values. A combined bootstrap and
permutation request must use one row-dependence class. An `episode_id` alone does not establish
order. The temporal Pearson lag-1 screen does not establish an estimator effective sample size or
a valid block length.

A later group-schedule API must record which callbacks admit repeated rows. Sampling an episode
with replacement duplicates its numeric coordinates. A new occurrence ID does not make those
coordinates distinct for continuous kNN. A continuous callback must abstain unless its own sample
contract admits the realized schedule. Without-replacement group subsampling is a diagnostic with
its own target, not an automatically calibrated bootstrap interval.

## 8. H1 and H2 reference contracts

`pid-h1-preflight` consumes one bounded schema-v2 fixture. It verifies timing, lineage, reset, RNG,
fold, clone, and instrumentation-noninterference obligations.

`pid-h1-protocol-a` consumes the exact passed preflight chain. It runs a deterministic finite
benchmark and emits no H1-A evidence. It cannot establish H1-B.

A real H1 study needs one typed protocol-specific primary decision. Improvement must favor the
diagnostic-augmented model, and the useful margin must be positive. The one-sided lower confidence
bound must exceed that margin under frozen uncertainty and multiplicity. Equivalence,
noninferiority, nonsignificance, factual-outcome fit, or a secondary endpoint cannot rescue
primary failure.

`pid-h2-reference` binds four planning artifacts and one bounded dataset. It exercises fixed-horizon
censoring and IPCW risk-estimator arithmetic. It emits no prospective H2 claim or proper
observed-data score.

Each reader preserves a typed failed artifact for readable invalid input when its CLI contract
promises one.

## 9. Viewer contract

The implemented Rerun adapter consumes a bounded run-log snapshot that passes validation. It
writes headless RRD output without replacement or streams to a matching viewer version.

Attribution artifact loading is opt-in. It validates bounded NumPy arrays, recorded hashes, shapes,
and local path confinement before the first viewer write.

The complete diagnostic panel set is not implemented. A future viewer must remain read-only and
must derive all state from canonical evidence.

## 10. World-model and rendering interfaces

The native exact-fork world-model reference uses this control and evidence spine. Learned-model
and linked mesh-versus-3DGS studies must also preserve it. Reconstruction covariates, the legacy
WorldWarp comparator, and custom product UI remain optional or deferred.

A world-model adapter must state whether prediction runs at deployment. A planning adapter must
record at least two proposals, their predictions and scores, and the score-caused selection.

If an optional adapter is added, it must:

1. Pin its implementation and model revision.
2. Record rights and input receipts.
3. Bind exact inputs and outputs.
4. State its support and observation law.
5. Route all mutations through the Agent Bridge.
6. Remain removable without breaking core capture, replay, or PID-disabled baselines.

## 11. Failure policy

Reject malformed, oversized, non-finite, or structurally ambiguous inputs before expensive work.
For a scientifically unsupported analysis path, emit a typed abstention and continue only with
independently valid paths. Do not invent a result to keep a pipeline running.

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
