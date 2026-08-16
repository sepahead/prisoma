# NCP Observer — Developer Handoff Prompt

> Copy-pasteable brief for a developer (or coding agent) bringing the `ncp-observer`
> consumer and artifact adapter up to the standard that Prisoma analysis requires. The intended
> future NCP producer
> is a NEST/Engram session. The named public `sepahead/engram` repository remains a
> README-only placeholder. The executable Engram Neural Labs host lives in
> `sepahead/Paper2Brain`.
> NCP's provider inventory records a preserved in-progress Paper2Brain migration.
> It targets candidate wire 1.0. It is not an installed or qualified integration.
> Prisoma has a read-only artifact descriptor. It has no compatible live publisher or bridge.
> This brief is self-contained. Read it top to
> bottom before touching code.

> **Compatibility boundary (rechecked 2026-08-13):** keep the latest immutable NCP
> `v0.8.0` release / wire 0.8. Official NCP main was observed at
> `1a04294c90c1b50eba06ae1c6afe9c951319250d` during this check. That commit is the
> unreleased, release-blocked `1.0.0-rc.1` candidate (wire 1.0;
> compact proto contract hash `163acc57d8a62b66`). It uses a different wire.
> NCP ledger tasks `P01`, `P02`, and `P03` are OPEN, not dependency-ready, and
> **NOT RUN**. They cover the native-1.0 observer, missing-variable and research-claim
> semantics, and fault-observatory migration plus Prisoma observer-role qualification.
> New low-overhead architecture prose is coordination-only. B01 remains `IN_PROGRESS` with no
> passing receipt. See the [verified NCP task ledger](https://github.com/sepahead/NCP/blob/1a04294c90c1b50eba06ae1c6afe9c951319250d/evidence/implementation/task-ledger.v1.json).

## 1. Context (what this is and is not)

**prisoma** builds auditable experiment semantics for supported world-model decisions and linked
closed-loop fidelity studies. Its preserved VLA/PID work is a conditional diagnostic family, not
the system's identity or an assumed result. That analysis is **source-agnostic**: downstream tools consume
`(V,L,D,A)`+labels artifacts in one schema (the `OfflineVldaDataset` JSON that the
`pid-offline-harness` reads). In `(V,L,D,A)`, **D is a declared source axis, not depth**.
Its producer, timing, ancestry, and scientific role are experimental variables.
`grandplan.md` §9.1 forbids premature V/L/D semantic labels, and §9.2 defines
pathway-source selection.

The analysis receives that contract through several adapters:

| Adapter or source | Role | Status |
|---|---|---|
| `experiments/safe_adapter/` (SAFE rollouts) | Reference adapter for the preserved EC1/H diagnostic family; not the W1-W3 critical path | Implemented contract adapter with declared axis provenance; real capture and the diagnostic-noninterference preflight remain open |
| `crates/pid-sim` fixtures + Rapier/toy harnesses | Standalone sim sources | Software/conformance smokes, not scientific gate passes |
| **`crates/ncp-observer` (this)** | **Optional** read-only NCP consumer/observer and Prisoma artifact adapter for a future conforming Engram/NEST producer | **Exploratory-only — not eligible for a registered D2/EC1 evaluation; no compatible live Paper2Brain publisher** |

`ncp-observer` is a **read-only passive tap**. It subscribes to a conforming producer's
Neuro-Cybernetic Protocol (NCP) data planes over Zenoh. It converts each eligible, complete,
source-correlated tick into an `OfflineVldaSample`. It is **not on grandplan's critical path** (grandplan does
not depend on Engram at all), and root workspace resolution/build/test is independent of
NCP/Engram/Zenoh. That dependency firebreak does not imply a scientific PID gate pass.
Your job is to make this optional bridge *good enough to
feed the rigorous analysis*, not to make the project depend on it.

**Why it matters anyway:** a future, separately validated per-channel analysis could test
whether gated information summaries are useful for *design-time* NCP codec priorities under
a low-bandwidth link. The current observer flattens channels into V/L/D/A axes and the current
harness runs axis-pair screens; it does not yet implement a per-channel prioritization policy.
See `crates/ncp-observer/README.md` and the wire-0.8
[`RESILIENCE.md`](https://github.com/sepahead/NCP/blob/v0.8.0/RESILIENCE.md).

## 2. The adapter-side promotion bar (not EC1 completion)

An `ncp-observer` capture becomes a candidate for broader D2/EC1 conformance evaluation
when its artifact passes the offline harness's **strict leakage gates** and carries
**honest provenance**, i.e. it can be run with all of:

```
cargo run --locked -p pid-sim --features analysis --bin pid-offline-harness -- --input outputs/ncp_vlda.json \
  --require-success-labels --require-heldout-split \
  --require-heldout-class-coverage --require-heldout-episode-disjoint \
  --require-axis-provenance-honest
```

(`--require-axis-provenance-honest` is the opt-in gate that fails the run on degraded or absent
axis-provenance markers. Geometry diagnostics are descriptive and never gate validity. The `just safe-adapter`
recipe already runs it alongside the three held-out gates.)

…exiting 0. That establishes only the adapter-side prerequisites for H1/H2 baselines and the
**conditional H3 PID-necessity audit** (does the full PID/abstention/exact same-fold M1 policy add value
beyond the frozen matched-access comparator registry; grandplan §3.8 PID kill rules, §6.5 baseline
hierarchy). It does not clear the
population, measure, estimator, or application gates. The current NCP artifact deliberately
declares no population support or complete-tuple contract. Continuous requests abstain;
`--pid-mode none` requests nothing; and fitted categorical MGW shared exclusions can produce only
a non-evidentiary diagnostic with population `NotEvaluated` and application `Blocked`. Empirical
PMF occupancy diagnostics do not establish population support. H1/H2
non-PID work may proceed with `--pid-mode none` after publication verification.

## 3. Current state

Already correct (do not regress):
- **V↔A join on the full driving-sensor `StreamPosition`** — a `SensorFrame` contributes
  its own `stream`, while `CommandFrame.source` echoes that `{epoch, seq}`. A sample pairs
  the action with the exact sensor that produced it, never by arrival time (the perception
  plane's DROP QoS makes arrival-time pairing unsound) and never on bare `seq`. An unset
  sensor `stream` or source-less command is dropped and counted as uncorrelatable.
- Deterministic channel ordering (sorted `BTreeMap` keys), read-only, System actor, and
  canonical run-log emission (`EmbeddingContract` / `EmbeddingCaptured` / `LabelObserved`,
  plus a finalize-time `ArtifactLogged` registering the dataset artifact with its sha256).
  A sensor clock must be finite, nonnegative, and within the unsigned run-log nanosecond range.
  Conversion truncates fractional nanoseconds toward zero. The observer rejects an invalid clock
  before it changes capture state. It clamps only a valid out-of-order clock to preserve monotonic
  run-log time. Each kept sample and capture event preserves the raw converted value as
  `sensor_timestamp_ns`.
- **Exact D alignment in prisoma, including arrival reordering**: plane-published
  `ObservationFrame.source` echoes the driving sensor `StreamPosition`; readouts are keyed
  by its full `{epoch, seq}` even before a sensor establishes the active epoch. Completed
  ticks are held for a short reorder grace window; future-epoch command/D receipts wait in
  bounded isolation until that epoch's sensor authorizes transition. A source-less pull/RPC
  observation is never promoted by recency. Complete validated-frame receipts remain
  globally bounded across retired epochs: exact redelivery is idempotent and conflicting
  evidence invalidates the capture without patching an already-logged row.
- **Bounded, reset-safe in-flight state**: FIFO (insertion-order) eviction, a global resident
  in-flight element ceiling, finite raw/admitted-frame/closed-receipt/sample/output ceilings,
  and sensor-authorized epoch changes prevent cross-pairing and unbounded retention;
  `sample_id = ncp-{epoch}-{seq}` stays unique. Incomplete V/A and unclaimed D are classified
  at epoch transition, capacity seal, and finalization without cloning all partial vectors.
- **Failure-safe finalization**: callbacks enqueue into a bounded handoff and one worker owns
  observer state. Artifact/run-log bytes are reconstructed and size-checked before publication;
  each uses a same-directory no-replace hard-link install plus fsync, and a hash-binding
  `.publication.json` receipt commits the pair last. The harness verifies the marker, both
  hashes, the canonical log and its exact dataset artifact identity, and a successful `complete` or
  `complete_with_warning` visible-receipt grade. The first
  finalization attempt seals ingestion and binds all three canonical bundle targets. Failures
  preserve exact same-path retry state; retries adopt only bounded byte-identical regular files.
- **Fail-closed ingress identity and transport**: `--secure` or `--open` must be chosen
  explicitly, unknown CLI options are rejected, realm/session key segments are validated, and
  every decoded data-plane frame's payload `session_id` must equal the subscribed session.
  A raw session subscription preserves decoder failures; only the exact three base-plane keys
  are accepted. `--secure` proves configuration selection, not producer authentication or a
  security audit. `--runlog` is mandatory, and library publication requires an explicitly bound
  capture session plus canonical logging before ingestion.

The machine-readable `capture_integrity` grade is deliberately limited to **visible receipts
and join state**. Whole-plane seq gaps, local receipt timestamps, reconnect/QoS history, clock
sync, and authenticated peer identity remain unassessed. The deterministic wire-trace
protocol-fault observatory in `grandplan.md` Appendix F is runnable for its bounded, complete,
hand-authored fixture and frozen logical fault registry. It distinguishes manifest-oracle truth
from native observer response and publishes strict per-replay outcome records plus
replay-equivalence/report evidence. Its reproducibility-bound local fixture execution level
requires matching build-time and runtime revisions, clean worktrees, and recorded lockfile and
executable hashes. This local execution does not create producer-consumer E3. The NCP
relationship remains E2. The binding is not signing or remote attestation. The fixed inventory
has 16 assessed cases: 15 matched cases and one matched known limitation for whole-tick
omission. It also has two expected `not_assessable` guards (logical pause and security-profile
claim) and zero mismatches. `all_expectations_matched` is not an 18/18 detection rate. Logical
slots are annotations that do not drive or measure timing, and the
declared-security-profile case does not load or select a configuration. It does not test wall-clock
latency, a live disconnect/reconnect, authentication/ACLs, or live control timing; do not promote
this fixture evidence to E4, EC1, live Engram validation, security validation, or a PID gate pass.

## 4. Workstreams (the three gaps, in priority order)

### Gap 1 — D alignment on `StreamPosition` (exact-only in-repo; residual is the live producer)

**Update (NCP `v0.8.0`, wire 0.8):** a plane-published `ObservationFrame.source` echoes
the driving `SensorFrame.stream` as the full `{epoch, seq}` correlation key. A source-less
observation is the valid pull/RPC form, but it has no exact driving tick and is therefore
dropped and counted by this plane observer. The manifest and lockfile pin the immutable
`v0.8.0` release.

`Observer::on_observation` stores each readout under its source `StreamPosition`; completed ticks
are held for a reorder grace window so a matching readout that arrives *after* its
command can still claim its own tick. Once emitted, a sample and its canonical event
are immutable. There is no `latest_d`, recency fallback, or post-emission patch path.
Tests cover in-order, reordered-within-grace, source-less pull-form drop, pre-active/future
epoch isolation, exact/conflicting redelivery across retirement, and immutable late-D paths.
What remains:
- **Live producer (external — the remaining alignment gap):** the wire contract requires the
  publisher to stamp each plane `ObservationFrame` with the driving sensor `source` at
  emission time. A source-less pull/RPC observation is dropped from plane capture, and the
  corresponding tick is excluded for missing D. That prevents biased atoms but can leave
  no analyzable rows.
- **Acceptance (met in-repo):** a session where D readouts arrive out of order still pairs
  each sample's D with its own `{epoch, seq}`; source-less/future D is never paired.

### Gap 2 — absent-L ticks are excluded, not fabricated (residual is a genuine channel)
A tick with no language channel yields an empty (zero-length) `L`. grandplan's adapter
contract (§8.7) is "never fabricated", and the observer honors it the strict way: **empty-axis
ticks are excluded from the artifact and counted** (`excluded_empty_l` in the
`ObserverStats` finalize report) — one empty axis would make `pid-offline-harness`
reject the whole dataset anyway. Kept samples always carry the honest
`metadata.l_source = "channel"` marker.
- **Residual:** a conforming D2/EC1 producer must provide a genuine, dimensionally stable
  language channel for retained ticks. A zero/hash backfill is fabricated evidence and is
  not a conformance repair; keep exclusion as the permanent default.
- **Acceptance:** no sample ever reaches the artifact with a fabricated or empty L (met);
  the exclusion count makes the loss visible (met).

### Gap 3 — no held-out split / episode / label structure (MEDIUM; unlocks D2/EC1 analysis)
The artifact currently emits one optional `episode_id`, no `metadata.split`, and labels
only if a `success_channel` is configured — so the strict gates and the PID-necessity audit can't run.
- **Fix:** map NEST trials → `episode_id`; assign each sample a `metadata.split`
  (`train`/`test`, episode-disjoint); and source a real per-tick or per-episode `success`
  label from the task outcome (not a placeholder). Keep the split assignment deterministic
  and leakage-safe (no `episode_id` in both train and test).
- **Acceptance:** the five `--require-*` strict modes above all pass on a real session
  capture, and `heldout_logreg_vlda` (a static factual-outcome baseline, not H2) runs.
  This is necessary but not sufficient for H1–H4: protocol-specific capture/assignment and
  outcome machinery remain separate, and PID interpretation still requires all four gates. The
  static held-out logistic path is not an H2 protocol or prospective result.

## 5. Hard constraints (do not violate)

1. **Run log is the source of truth for accepted recorded events.** Every captured sample must be
   reconstructable from canonical events. The JSON artifact is a convenience view. The log cannot
   prove an upstream event that the observer never received.
2. **The observer drives nothing.** It is read-only; the Agent Bridge stays the *only*
   control plane. Never add a publish/command path here.
3. **The NCP-specific mapping lives in prisoma** (`crates/ncp-observer`), not in Engram.
4. **Do not fabricate axes** (Gap 2) and **do not touch the `pid-core` estimators** — this
   is a capture-adapter task, not an estimator change.
5. **D is hidden states, not depth.** Keep the mapping and docs consistent with that.
6. **Do not call any record-port a principled D without a probe.** Today D is
   first-available-ports in BTreeMap order; before any world-model claim, run a
   `grandplan.md` §9.2-style physics/world-model probe on the candidate ports —
   a pre-motor readout is the locus most at risk of measuring action formatting
   (see `RESEARCH_VLA_D_NCP.md` recommendation 1).
7. **Prefer exclusion over backfill for absent L.** The research memo's position
   (`RESEARCH_VLA_D_NCP.md` recommendation 2): if Engram never grows a real language channel,
   keep excluding absent-L ticks permanently and restrict Engram screens to
   D/V-involving atoms — do not accept any zero/hash proxy for L.

## 6. Build & run

`ncp-observer` is **kept off the default cargo workspace** (`Cargo.toml` `exclude`)
to keep NCP/Zenoh off the critical path; it git-depends on the published NCP repo
<https://github.com/sepahead/NCP> (tag `v0.8.0`). Build/test it explicitly:

```bash
# Build + test the workspace-excluded observer directly:
cargo build --locked --manifest-path crates/ncp-observer/Cargo.toml
cargo test --locked --manifest-path crates/ncp-observer/Cargo.toml

# deterministic offline wire-0.8 fault suite; published artifacts must reconstruct
# exactly; only writer-reserved partial crash scratch may be cleaned on explicit retry
cargo run --locked --manifest-path crates/ncp-observer/Cargo.toml \
    --bin ncp-fault-observatory -- --out-dir outputs/ncp_fault_observatory
cargo run --locked --manifest-path crates/ncp-observer/Cargo.toml \
    --bin ncp-fault-observatory -- --verify outputs/ncp_fault_observatory

# tap a live session, then analyze the artifact through the standard harness
# (the crate is workspace-excluded, so `-p ncp-observer` does NOT resolve from the
# repo root — always go through --manifest-path)
cargo run --locked --manifest-path crates/ncp-observer/Cargo.toml --bin ncp-observe -- \
    --open --session uav3 --out outputs/ncp_vlda.json --runlog outputs/ncp_runlog.jsonl
cargo run --locked -p pid-sim --features analysis --bin pid-offline-harness -- --input outputs/ncp_vlda.json \
    --pid-mode none --summary-json outputs/ncp_summary.json \
    --runlog outputs/ncp_baseline_runlog.jsonl
```

The harness verifies `outputs/ncp_vlda.json.publication.json` and rejects degraded/invalid
captures. Schema-1 receipts must bind the exact legacy `v0.8.0` tag, revision, wire, and
compact hash; missing or different-wire identity fails closed. This command requests no PID
because the adapter declares no population support.
Continuous KSG/shared-exclusions requests would abstain rather than infer support from the
observed sample. Fitted categorical MGW shared exclusions would remain non-evidentiary with
population `NotEvaluated` and application `Blocked`.

The `ncp-core` / `ncp-zenoh` dependencies pin the immutable published `v0.8.0` tag in
lockstep. The crate stays off the default workspace so NCP/Zenoh resolution cannot break
root workspace resolution; scientific PID gates remain a separate question.

## 7. References

- `crates/ncp-observer/README.md` — what it does + the closed-loop payoff.
- `crates/ncp-observer/src/lib.rs` — `Observer` (full-`StreamPosition` source join,
  full-key `d_by_key`, retained receipts, `emit_ready`).
- Wire-0.8 [`NEURO_CYBERNETIC_PROTOCOL.md`](https://github.com/sepahead/NCP/blob/v0.8.0/NEURO_CYBERNETIC_PROTOCOL.md)
  — the pinned NCP specification for this consumer (Gap 1 lives here).
- `experiments/safe_adapter/` — the reference `(V,L,D,A)` contract adapter to mirror for
  provenance and split/label structure; real capture and protocol preflights remain open.
- `EXPERIMENTS.md` §0.2 (runbook) and `grandplan.md` §3.8 (PID kill rules) + §6.5 (baseline
  hierarchy), §9.2 (pathway-source / D selection), §8.11 (one-control-plane rule).

## 8. Out of scope

- Making prisoma depend on Engram/NCP for any core result (it must stay standalone).
- Changing PID estimators, the harness gate logic, or the run-log schema.
- Any write/control path from the observer.
