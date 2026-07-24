# `ncp-observer` — passive Neuro-Cybernetic Protocol tap for prisoma

Converts a conforming Neuro-Cybernetic Protocol producer into another `(V,L,D,A)` source for
prisoma's Partial Information Decomposition. The intended future producer is a NEST/Engram
session, but the named public `sepahead/engram` repository is currently a README-only placeholder
and supplies no executable publisher. This crate is a **read-only observer**: it subscribes to the
NCP data-plane keys over Zenoh and never drives anything (the Agent Bridge stays the only control
plane).

> **Compatibility boundary (rechecked 2026-07-24):** this crate pins immutable NCP `v0.8.0`
> and wire 0.8. Public NCP main at `10492c81` is an unreleased, release-blocked
> `1.0.0-rc.1` wire-1.0 candidate. It is not a compatible dependency update.

It uses the canonical Rust NCP SDK (`ncp-core` + `ncp-zenoh`) from the published
NCP repo **<https://github.com/sepahead/NCP>**. Spec: `NEURO_CYBERNETIC_PROTOCOL.md`
in that repo.

## Scope & status (read before relying on it)

This crate is **optional and exploratory-only**. It is **not** on grandplan's critical
path — grandplan does not depend on Engram, and the S2/EC1 reference `(V,L,D,A)` producer
is `experiments/safe_adapter/`. The core workspace builds and tests with NCP/Engram/Zenoh
absent, and the static factual-outcome baseline smoke runs with PID disabled; `ncp-observer` is **excluded from the
default cargo workspace** (build it with `--manifest-path`, see below).

It can support *exploratory* PID screens on a future conforming producer, but it is **below the
S2/EC1 conformance bar** (an optional M2 ecosystem item) until the gaps below close:

1. **D alignment — exact-only and immutable in-repo.** `ObservationFrame` carries
   a `source` echoing the driving `SensorFrame.stream`, and this observer joins D
   only on that exact `{epoch, seq}`. A short reorder grace window admits a matching
   readout that arrives after its command. Command/readout receipts that overtake
   the first sensor of a fresh epoch are buffered under the full `{epoch, seq}`
   and released only when that sensor authorizes the transition. Once a row's
   canonical event exists, complete validated-frame receipts are retained for the
   finite capture: exact redelivery is idempotent, while changed evidence marks
   the capture invalid without patching the row. An observation with **no
   `source`** is the pull/RPC form and is dropped (source ABSENCE, the wire-0.8
   successor to the retired `seq == 0` sentinel): there is no recency fallback or
   future-D pairing. A conforming producer must therefore stamp every published
   plane observation with its driving `source`.
2. **Honest `L` — absent-language ticks are excluded, never fabricated.** A tick with
   no language channel yields an empty (zero-length) `L`; such ticks are **excluded
   from the artifact and counted** (`excluded_empty_l` in the finalize report),
   because one empty axis would make `pid-offline-harness` reject the whole dataset.
   Kept samples always carry `metadata.l_source = "channel"`. A zero/hash backfill would
   fabricate a language axis and is not a conformance repair; exclusion remains the
   fail-closed policy until a genuine language channel exists.
3. **Finalization integrity — repaired.** Artifact and run-log bytes are bounded
   and completely reconstructed before either final path is installed. Each file
   uses a same-directory temporary file, fsync, and a no-replace hard-link install;
   a small hash-binding publication receipt is installed last as the commit
   marker. `pid-offline-harness` verifies the receipt, both hashes, the canonical
   run log, its exact dataset artifact identity, and the capture grade before
   accepting an NCP artifact. Publication requires an explicitly bound session
   and UTF-8-representable canonical targets. Append/hash/write/fsync failures
   preserve exact same-path retry state; retries pin all three canonical targets
   and adopt only byte-identical bounded regular files.
4. **Held-out structure** — no `metadata.split` / `episode_id` / required `success` labels
   by default, so the strict `--require-heldout-*` gates and H1/H2 protocol analyses cannot
   run. Passing these adapter gates would still not clear the four PID gates or implement
   H1 Protocol A/B, prospective H2, conditional H3, or H4 interventions.
5. **Protocol-fault observatory — deterministic fixture evidence only.** The
   `ncp-fault-observatory` binary replays a bounded, content-addressed, complete wire-0.8
   baseline through the same exact-route classifier and raw decoder as live capture. Its frozen
   logical schedules cover omission, duplicate/conflict, reorder, pause, late receipt, malformed
   input, version/identity/route/size faults, trace truncation, stream transition, and a
   declared-security-profile label guard. Each case runs twice and publishes path-independent
   semantic hashes, exact artifact hashes, a strict per-replay `outcome.json` record binding the
   counters/journal/finalization deltas and sample oracle, a typed oracle comparison, a canonical
   run log, and a receipt-last bundle. This is local, fixture-specific E3-style evidence, not
   live-producer conformance. The frozen inventory is 16 assessed cases (15 matched and one
   matched known limitation: whole-tick omission), two expected `not_assessable` guards (logical
   pause and the security-profile claim), and zero mismatches. Thus
   `all_expectations_matched=true` means the expected classifications held, not an 18/18 detection
   rate.

The in-repo work and external publisher requirements are tracked in
**`NCP_DEV_PROMPT.md`** at the repo root.

## What it does

Subscribes to `engram/ncp/session/{id}/{sensor,command,observation}` and converts
each closed-loop tick into an `OfflineVldaSample`, writing:

1. an **`OfflineVldaDataset` JSON artifact** — after receipt verification it can
   run through `pid-offline-harness` for diagnostics/baselines. It carries no
   inferred population-support declaration: continuous KSG/shared-exclusions
   requests abstain with `support_contract_unspecified`; `--pid-mode none`
   requests no estimates; and quantized discrete `I_min` can run only as a
   non-evidentiary diagnostic with population `NotEvaluated` and application
   `Blocked`, pending justified per-axis declarations and the remaining gates; and
2. **canonical run-log events** (the source of truth): one `EmbeddingContract`
   declaring the `(V,L,D,A)` variables, an `EmbeddingCaptured` per kept sample, a
   `LabelObserved` per success label, and — at finalize — an `ArtifactLogged`
   registering the dataset artifact (uri + sha256) so the run log can locate and
   verify the data it describes. A `.publication.json` receipt beside the dataset
   commits the durable pair last; failed finalization seals and preserves the
   in-memory sample/event source for an exact same-path retry.

Ticks that can never pass the harness (an empty axis: no language channel yet, no
observation yet) are **excluded and counted**, dims are held to the declared
contract, a sensor restart (a change of `stream.epoch`) starts a new incarnation
(`sample_id = ncp-{epoch}-{seq}`), and the finalize report (`ObserverStats`)
surfaces every locally observed exclusion/eviction path so a small artifact is
diagnosable rather than mysterious.

`capture_integrity` is deliberately a **visible-receipt/join grade**, not an
end-to-end delivery-completeness claim. The current observer does not detect an
entirely missing own-stream tick, record local receipt timestamps/reconnect history,
or attest negotiated QoS, clock sync, producer authentication, or ACL correctness.
The deterministic observatory preserves that distinction: its separate manifest oracle
identifies an entirely omitted tick as a known native blind spot rather than calling it an
observer detection.
Known degraded/invalid and zero-row captures end with `RunStatus::Failed`; the CLI
still preserves their diagnostic bundle but exits nonzero, and the offline harness
rejects it for analysis.

### (V, L, D, A) mapping
- **V** ← `SensorFrame` channels (all but the language channel), flattened.
- **L** ← the `instruction` `SensorFrame` channel (configurable).
- **D** ← `ObservationFrame` record-port readouts — the pre-motor neural state
  (world-model status **untested**: no architecture-evidence probe (grandplan §9.1 —
  a fused hidden state may not be called a "world model"/"dynamics" axis without it)
  has been run on these ports; "internal simulation" is what the `PID(V,D;A)` probe would *test*,
  not an established property). Note: in `(V,L,D,A)`, **D is the Dynamics /
  world-model axis**, not depth.
- **A** ← `CommandFrame` channels, flattened.

### Alignment (correctness)
V and A are joined on the driving sensor's **`StreamPosition` (`{epoch, seq}`)** —
wire 0.8's typed source-correlation key. A `SensorFrame` IS the origin, so it
contributes its OWN `stream`; a `CommandFrame.source` echoes the `SensorFrame.stream`
it was computed from, so a sample pairs the action with the sensor that produced
it, never by arrival time (the perception plane's DROP QoS makes arrival-time
pairing unsound) and never on the bare seq (a sensor restart reuses seqs under a
fresh epoch). `ObservationFrame.source` echoes the driving `SensorFrame.stream` too,
so D aligns on the full `{epoch, seq}` as well: readouts are stored tagged with
their `source.epoch`; completed ticks are held for a short reorder grace window so a
readout that arrives *after* its tick's command still claims its own tick.
Later-still exact readouts are idempotent; conflicting complete-frame evidence
invalidates the capture without changing an already-buffered artifact row or
run-log event. Every kept sample records `metadata.d_source = "source"`; ticks
with no exact readout are excluded and counted. A `SensorFrame` with an unset own
`stream`, and a `CommandFrame`/`ObservationFrame` with **no `source`**, are
uncorrelatable and are dropped + counted rather than merged into one bogus tick.
Each frame's payload `session_id` must equal the explicitly bound capture session.
Passenger generations are retained per key, while the live `session.generation`
is locked only by the first validated authorizing sensor; stale/foreign-session
frames are dropped and counted.

## Run

```bash
# tap a live session and write the artifact + run log on Ctrl-C / SIGTERM
# (ncp-observer is excluded from the default workspace; use --manifest-path)
cargo run --manifest-path crates/ncp-observer/Cargo.toml --bin ncp-observe -- \
    --open --session uav3 --out outputs/ncp_vlda.json --runlog outputs/ncp_runlog.jsonl
# then verify the committed bundle and run PID-disabled diagnostics/baselines
cargo run -p pid-sim --bin pid-offline-harness -- --input outputs/ncp_vlda.json \
    --pid-mode none --summary-json outputs/ncp_summary.json \
    --runlog outputs/ncp_baseline_runlog.jsonl

# run the complete deterministic offline wire-0.8 fault suite; an existing path
# is accepted only when every published artifact is exact and no entry is unbound
cargo run --locked --manifest-path crates/ncp-observer/Cargo.toml \
    --bin ncp-fault-observatory -- --out-dir outputs/ncp_fault_observatory
# later, revalidate that in-place publication without rerunning the scenarios
cargo run --locked --manifest-path crates/ncp-observer/Cargo.toml \
    --bin ncp-fault-observatory -- --verify outputs/ncp_fault_observatory
```

The second command verifies `outputs/ncp_vlda.json.publication.json` first. PID
mode is disabled intentionally: this adapter does not fabricate the current pid-rs
population-support declaration from observed cardinalities.

The observatory's built-in baseline is a finite hand-authored fixture. A supplied
`--trace FROZEN_V1_BASELINE.json` must be an exact typed-semantic variant of that baseline and
pass the same strict, bounded, current-wire validation before any output directory is created.
Logical slots are schedule annotations only: replay is sequential, and they neither drive nor
measure timing. In the truncation case, only the missing tick-7 command is observer-visible;
the missing tick-7 observation and later tail are manifest-only. Truncation is not a transport
disconnect. The security case only checks that a declared-profile label cannot become security
evidence; it does not load or select a configuration and tests neither authentication nor ACLs.
An offline code path that emits no control messages does not establish live control-timing
noninterference. The report hard-codes the corresponding nonclaims: it does not establish E4,
complete EC1, validate live Engram or security, or change any PID gate.

The E3-style evidence label additionally requires matching build-time and runtime Git revisions,
clean build-time and runtime worktree states, the standalone crate lockfile hash, and the exact
executable hash. This is a local reproducibility binding, not a signature or remote attestation.
Dirty, stale, or unknown source/build state still produces a verifiable diagnostic bundle, but its
typed evidence level is downgraded to reproducibility-unqualified. `--verify` is intentionally
read-only and in-place because publication receipts bind canonical paths; it snapshots every
compiled schedule, per-replay outcome record, dataset, inner run log/receipt, trace, report, outer
run log, and the receipt once for semantic/hash comparison. The exact
`.<allowed-target>.tmp-<pid>-<nonce>` namespace is reserved for the writer: an explicit `--out-dir`
retry may discard partial crash scratch only after independently reconstructing every target (or
computing the pending outer receipt), while `--verify` rejects and leaves such entries untouched.
New semantic/content projections use pid-runlog's lossless canonical JSON hash v2; exact raw and
artifact-byte SHA-256 identities remain separate.

## Possible future integration with NCP

The implemented direction is the read-only capture path. A possible later offline-to-online
workflow remains a research proposal, and both sides must stay non-invasive:

1. **Online, read-only tap (this crate).** Subscribe to the NCP data planes →
   `(V,L,D,A)` samples aligned on the driving sensor `{epoch, seq}` → run-log +
   offline analysis. A future conforming Engram publisher could be another
   `(V,L,D,A)` source; the observer drives nothing (Agent Bridge stays the only
   control plane).
2. **Future offline analysis → human-reviewed static configuration.** The current
   observer flattens channels into V/L/D/A axes and the harness runs axis-pair screens;
   it does **not** implement per-channel prioritization. A separate, scientifically gated
   per-channel analysis could test whether information summaries help choose static codec
   priorities under a poor link (see `RESILIENCE.md` in <https://github.com/sepahead/NCP>).
   Any adopted priorities belong in a versioned, human-reviewed NCP configuration, never
   in a write/control path from this observer.

Thus the only current arrow is NCP → read-only capture → offline analysis. PID is never a
per-tick runtime computation. Per-channel codec selection and a sim-vs-hardware fidelity
metric remain candidate studies that require their own estimand, gates, fixtures, and
conformance evidence (`NEUROMORPHIC.md` §5 in <https://github.com/sepahead/NCP>).

## Compatibility & versioning

The manifest and lockfile pin the immutable NCP **`v0.8.0`** tag
(`NCP_VERSION = 0.8`, `CONTRACT_HASH = d1b50a2d8a265276`) — the wire-0.8
stream-identity release — and resolve it from the published repository; no sibling
checkout or path override is required.
Wire 0.8 splits the overloaded top-level `seq` into a typed `stream` (this frame's
own position) and `source` (the frame that drove it), carries `session_id` +
`session.generation` on the data plane, and this tap drops/counts every
version-less, incompatible, wrong-kind, duplicate-key, source-less-plane, or
wrong-session frame rather than degrading D alignment. The live binary subscribes
at the raw session boundary so such failures remain countable, then accepts only
the three exact base-plane keys; named/unknown subkeys invalidate the visible-receipt
capture rather than being silently reclassified.

Finite software ceilings bound each worker-admitted raw frame (1 MiB), admitted
frame/byte lifetime totals (1,000,000 / 8 GiB), one axis (65,536 values), resident
in-flight values (1,000,000), globally retained closed receipts (50,000), samples
(25,000), retained sample values (10,000,000), and artifact/run-log outputs
(256 MiB each). These are safety ceilings, not measured performance or recommended
scientific sample sizes. Callback-side oversize, exact-route, and bounded-handoff
drops are counted separately; they are not misdescribed as worker-admission totals.

Connection mode is explicit: `--open` uses the unauthenticated/scouting-off NCP
client default and prints a warning; `--secure` calls `ZenohBus::open_secure` and
fails closed unless `NCP_ZENOH_CONFIG` names a strict TLS-only NCP client config.
Omitting both modes, combining them, or passing an unknown option is a startup
error. The realm is validated with `Keys::try_new` before either open path, and
every frame's payload `session_id` must match the subscribed session — wire 0.8
carries `session_id` + `session.generation` in the payload on all three planes
(transport-neutral identity), not only in the routing key.
`--secure` attests only selection of NCP's fail-closed client configuration; it is
not a producer-authentication or security-validation result. `--runlog` is
mandatory: the observer refuses to commit an artifact without the canonical log
that registers and reconstructs its evidence.
This observer reads only the generic data planes
(`SensorFrame`/`CommandFrame`/`ObservationFrame` → opaque value/time vectors), so it
is unaffected by NCP's neural enums. The one frame the old `v0.1.0` pin could not
handle was an `ObservationFrame` whose `Observation.observable` is the `#10` value
(`binary_state`), which would fail to deserialize and be silently dropped; the
current `v0.8.0` pin (proto-native wire) decodes it, so this observer now ingests `#10`
observations too. **Re-pin rule:** bump the `ncp-core`/`ncp-zenoh` tag in lockstep
with any future additive NCP wire extension *before* a producer emits it, rerun the
build/decoder/conformance checks, and update the mapping if the new fields change
capture semantics.

NCP's wire is **simulator-agnostic by design**. If a future public Engram/NEST
publisher and a later NEURON, Brian2, GeNN, or neuromorphic backend serve the same
wire, this observer's passive capture boundary should not need to change. A
sim-vs-hardware comparison remains a candidate study: it first needs a separate
estimand, fixtures, conformance evidence, and scientific validation.

## Build note

This crate git-depends on the NCP repo <https://github.com/sepahead/NCP>
(pinned to the immutable `v0.8.0` tag, wire 0.8) and pulls Zenoh, so it is heavier
than the pure-PID crates. The
estimator gates (`just exp0`, `just exp0-bin`, etc.) run the pid-rs crates via
`--manifest-path pid-rs/crates/...` and are unaffected.
