# `ncp-observer` — passive Neuro-Cybernetic Protocol tap for prisoma

Makes **Engram** (a NEST spiking network, exposed over the Neuro-Cybernetic Protocol)
another `(V,L,D,A)` source for prisoma's Partial Information Decomposition — the
same role `experiments/safe_adapter` plays for SAFE rollouts. It is a **read-only
observer**: it subscribes to the NCP data-plane keys over Zenoh and never drives
anything (the Agent Bridge stays the only control plane).

It uses the canonical Rust NCP SDK (`ncp-core` + `ncp-zenoh`) from the published
NCP repo **<https://github.com/sepahead/NCP>**. Spec: `NEURO_CYBERNETIC_PROTOCOL.md`
in that repo.

## Scope & status (read before relying on it)

This crate is **optional and exploratory-only**. It is **not** on grandplan's critical
path — grandplan does not depend on Engram, and the M5 critical-path `(V,L,D,A)` producer
is `experiments/safe_adapter/`. The pure-PID stack builds, tests, and clears the strict
gates with no NCP/Engram/Zenoh dependency, so `ncp-observer` is **excluded from the
default cargo workspace** (build it with `--manifest-path`, see below).

It is fine for *exploratory* PID screens on a live Engram session, but it is **below the
M5 contract** (gate-passing artifacts with honest provenance) until the gaps below close:

1. **D alignment — exact-only and immutable in-repo.** `ObservationFrame` carries
   a `source` echoing the driving `SensorFrame.stream`, and this observer joins D
   only on that exact `{epoch, seq}`. A short reorder grace window admits a matching
   readout that arrives after its command; once a row's canonical event exists,
   later readouts are dropped and counted, never patched. An observation with **no
   `source`** is the pull/RPC form and is dropped (source ABSENCE, the wire-0.8
   successor to the retired `seq == 0` sentinel): there is no recency fallback or
   future-D pairing. A conforming producer must therefore stamp every published
   plane observation with its driving `source`.
2. **Honest `L` — absent-language ticks are excluded, never fabricated.** A tick with
   no language channel yields an empty (zero-length) `L`; such ticks are **excluded
   from the artifact and counted** (`excluded_empty_l` in the finalize report),
   because one empty axis would make `pid-offline-harness` reject the whole dataset.
   Kept samples always carry `metadata.l_source = "channel"`. The tracked follow-up
   (Gap 2) is a fixed-dim zero *backfill* that would retain such ticks under the
   degraded `l_source = "absent_zeroed"` marker, which the harness's
   `--require-axis-provenance-honest` gate rejects.
3. **Finalization integrity — repaired.** Samples and canonical events remain
   buffered until a same-directory temporary artifact is flushed, fsynced,
   atomically renamed, and hashed. The complete run log is then reconstructed and
   installed the same way. Append/hash/write failures propagate without clearing
   samples; the first finalization attempt seals ingestion and binds its output
   path, and an exact retry reconstructs one event per sample with no duplicates.
4. **Held-out structure** — no `metadata.split` / `episode_id` / required `success` labels
   by default, so the strict `--require-heldout-*` gates and the grandplan §4 H1 confirmatory-claim audit can't run.

Bringing it up to bar is a self-contained task — see **`NCP_DEV_PROMPT.md`** at the repo root.

## What it does

Subscribes to `engram/ncp/session/{id}/{sensor,command,observation}` and converts
each closed-loop tick into an `OfflineVldaSample`, writing:

1. an **`OfflineVldaDataset` JSON artifact** — run it through `pid-offline-harness`
   (`V/L/D → A` PID screens), exactly like the SAFE adapter's output; and
2. **canonical run-log events** (the source of truth): one `EmbeddingContract`
   declaring the `(V,L,D,A)` variables, an `EmbeddingCaptured` per kept sample, a
   `LabelObserved` per success label, and — at finalize — an `ArtifactLogged`
   registering the dataset artifact (uri + sha256) so the run log can locate and
   verify the data it describes. Artifact and log publication are atomic/durable;
   failed finalization seals and preserves the in-memory sample/event source for
   an exact same-path retry.

Ticks that can never pass the harness (an empty axis: no language channel yet, no
observation yet) are **excluded and counted**, dims are held to the declared
contract, a sensor restart (a change of `stream.epoch`) starts a new incarnation
(`sample_id = ncp-{epoch}-{seq}`), and the finalize report (`ObserverStats`)
surfaces every exclusion/eviction path so a small artifact is diagnosable rather
than mysterious.

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
Later-still readouts are dropped without changing an already-buffered artifact row
or run-log event. Every kept sample records `metadata.d_source = "source"`; ticks
with no exact readout are excluded and counted. A `SensorFrame` with an unset own
`stream`, and a `CommandFrame`/`ObservationFrame` with **no `source`**, are
uncorrelatable and are dropped + counted rather than merged into one bogus tick.
Each frame's payload `session_id` (must equal the captured session) and
`session.generation` (the live incarnation, locked to the first seen) are validated;
a stale/foreign-session frame is dropped and counted.

## Run

```bash
# tap a live session and write the artifact + run log on Ctrl-C / SIGTERM
# (ncp-observer is excluded from the default workspace; use --manifest-path)
cargo run --manifest-path crates/ncp-observer/Cargo.toml --bin ncp-observe -- \
    --open --session uav3 --out outputs/ncp_vlda.json --runlog outputs/ncp_runlog.jsonl
# then run the PID screens on it
cargo run -p pid-sim --bin pid-offline-harness -- --input outputs/ncp_vlda.json \
    --summary-json outputs/ncp_summary.json --runlog outputs/ncp_pid_runlog.jsonl
```

## Best integration with NCP (the closed loop)

The recommended integration is **bidirectional**, and both directions are
non-invasive:

1. **Online, read-only tap (this crate).** Subscribe to the NCP data planes →
   `(V,L,D,A)` samples aligned on the driving sensor `{epoch, seq}` → run-log +
   offline PID. Engram is just
   another `(V,L,D,A)` source; the observer drives nothing (Agent Bridge stays the
   only control plane).
2. **Offline → online feedback (the payoff).** The PID screens here quantify, per
   sensor channel, the **unique / redundant / synergistic** information about the
   action. That is exactly the policy NCP's perception plane needs under a poor
   (low-bandwidth) link: **drop redundant channels, keep unique ones, bundle
   synergistic ones** (see `RESILIENCE.md` in <https://github.com/sepahead/NCP>). prisoma computes the
   policy *offline*; NCP's codec applies it *online* as static channel priorities.

So the loop closes: NCP streams `(V,L,D,A)` → prisoma decomposes it → a channel
priority/redundancy policy feeds back into the perception codec. PID is a
**design-time** tool (expensive, hard to estimate — prisoma's whole domain), never
a per-tick runtime computation. It also serves as a **sim-vs-hardware fidelity
metric** (`NEUROMORPHIC.md` §5 in <https://github.com/sepahead/NCP>): does a neuromorphic chip preserve
the same information flow as the NEST simulator?

## Compatibility & versioning

The manifest and lockfile pin the immutable NCP **`v0.8.0`** tag
(`NCP_VERSION = 0.8`, `CONTRACT_HASH = d1b50a2d8a265276`) — the wire-0.8
stream-identity release — and resolve it from the published repository; no sibling
checkout or path override is required.
Wire 0.8 splits the overloaded top-level `seq` into a typed `stream` (this frame's
own position) and `source` (the frame that drove it), carries `session_id` +
`session.generation` on the data plane, and this tap drops/counts every
version-less, incompatible, wrong-kind, source-less-plane, or wrong-session frame
rather than degrading D alignment.

Connection mode is explicit: `--open` uses the unauthenticated/scouting-off NCP
client default and prints a warning; `--secure` calls `ZenohBus::open_secure` and
fails closed unless `NCP_ZENOH_CONFIG` names a strict TLS-only NCP client config.
Omitting both modes, combining them, or passing an unknown option is a startup
error. The realm is validated with `Keys::try_new` before either open path, and
every frame's payload `session_id` must match the subscribed session — wire 0.8
carries `session_id` + `session.generation` in the payload on all three planes
(transport-neutral identity), not only in the routing key.
`--runlog` is mandatory: the observer refuses to publish an artifact without the
canonical log that registers and reconstructs its evidence.
This observer reads only the generic data planes
(`SensorFrame`/`CommandFrame`/`ObservationFrame` → opaque value/time vectors), so it
is unaffected by NCP's neural enums. The one frame the old `v0.1.0` pin could not
handle was an `ObservationFrame` whose `Observation.observable` is the `#10` value
(`binary_state`), which would fail to deserialize and be silently dropped; the
`v0.5.x` pin (proto-native wire) decodes it, so this observer now ingests `#10`
observations too. **Re-pin rule:** bump the `ncp-core`/`ncp-zenoh` tag in lockstep
with any future additive NCP wire extension *before* Engram emits it; no code change
is needed.

Engram is NEST today, but NCP's wire is **simulator-agnostic by design** — if a
future Engram backend (NEURON / Brian2 / GeNN / a neuromorphic chip) serves the
same wire, this observer is unchanged. That is exactly why it doubles as a
**sim-vs-hardware fidelity metric** (above): does the chip preserve the same
information flow the NEST simulator does?

## Build note

This crate git-depends on the NCP repo <https://github.com/sepahead/NCP>
(pinned to the immutable `v0.8.0` tag, wire 0.8) and pulls Zenoh, so it is heavier
than the pure-PID crates. The
estimator gates (`just exp0`, `just exp0-bin`, etc.) run the pid-rs crates via
`--manifest-path pid-rs/crates/...` and are unaffected.
