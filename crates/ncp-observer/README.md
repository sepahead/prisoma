# `ncp-observer` — passive Neuro-Cybernetic Protocol tap for pid_vla

Makes **Engram** (a NEST spiking network, exposed over the Neuro-Cybernetic Protocol)
another `(V,L,D,A)` source for pid_vla's Partial Information Decomposition — the
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

1. **D alignment — done in-repo, pending an external runtime stamp.** `ObservationFrame`
   **now carries `seq`**, and this observer already joins D on it: `on_observation` stores
   each readout in `d_by_seq[obs.seq]` and `try_complete` prefers it over recency (test
   `d_aligns_on_seq_not_recency`). Recency is only the fallback for an unstamped
   (`obs.seq == 0`) frame, so the lone remaining piece is external — the Engram publisher
   must stamp each observation with its driving sensor `seq`.
2. **Honest `L`** — an absent language channel currently yields an all-zero `L`; provenance
   must be explicit (real `L`, or a marker so zeroed-`L` samples are excluded), never
   fabricated.
3. **Held-out structure** — no `metadata.split` / `episode_id` / required `success` labels
   by default, so the strict `--require-heldout-*` gates and the §14.1.1 H1 audit can't run.

Bringing it up to bar is a self-contained task — see **`NCP_DEV_PROMPT.md`** at the repo root.

## What it does

Subscribes to `engram/ncp/session/{id}/{sensor,command,observation}` and converts
each closed-loop tick into an `OfflineVldaSample`, writing:

1. an **`OfflineVldaDataset` JSON artifact** — run it through `pid-offline-harness`
   (`V/L/D → A` PID screens), exactly like the SAFE adapter's output; and
2. **canonical run-log events** (the source of truth): one `EmbeddingContract`
   declaring the `(V,L,D,A)` variables, an `EmbeddingCaptured` per sample, and a
   `LabelObserved` per success label.

### (V, L, D, A) mapping
- **V** ← `SensorFrame` channels (all but the language channel), flattened.
- **L** ← the `instruction` `SensorFrame` channel (configurable).
- **D** ← `ObservationFrame` record-port readouts — the neural state *before* the
  motor head (the "internal simulation" the `PID(V,D;A)` probe targets). Note: in
  `(V,L,D,A)`, **D is the Dynamics / world-model axis**, not depth.
- **A** ← `CommandFrame` channels, flattened.

### Alignment (correctness)
V and A are joined on **`seq`** — a `CommandFrame.seq` echoes the `SensorFrame.seq`
it was computed from, so a sample pairs the action with the sensor that produced
it, never by arrival time (the perception plane's DROP QoS makes arrival-time
pairing unsound). `ObservationFrame` **now carries `seq` too** (it echoes the
driving `SensorFrame.seq`), so D aligns on `seq` as well: this observer stores each
readout in `d_by_seq[obs.seq]` and prefers it, falling back to recency only for an
unstamped (`obs.seq == 0`) frame. The lone remaining D-alignment gap is external —
the Engram publisher must stamp observations with the driving `seq`.

## Run

```bash
# tap a live session and write the artifact + run log on Ctrl-C
cargo run -p ncp-observer --bin ncp-observe -- \
    --session uav3 --out outputs/ncp_vlda.json --runlog outputs/ncp_runlog.jsonl
# then run the PID screens on it
cargo run -p pid-sim --bin pid-offline-harness -- --input outputs/ncp_vlda.json \
    --summary-json outputs/ncp_summary.json --runlog outputs/ncp_pid_runlog.jsonl
```

## Best integration with NCP (the closed loop)

The recommended integration is **bidirectional**, and both directions are
non-invasive:

1. **Online, read-only tap (this crate).** Subscribe to the NCP data planes →
   `(V,L,D,A)` samples aligned on `seq` → run-log + offline PID. Engram is just
   another `(V,L,D,A)` source; the observer drives nothing (Agent Bridge stays the
   only control plane).
2. **Offline → online feedback (the payoff).** The PID screens here quantify, per
   sensor channel, the **unique / redundant / synergistic** information about the
   action. That is exactly the policy NCP's perception plane needs under a poor
   (low-bandwidth) link: **drop redundant channels, keep unique ones, bundle
   synergistic ones** (see `RESILIENCE.md` in <https://github.com/sepahead/NCP>). pid_vla computes the
   policy *offline*; NCP's codec applies it *online* as static channel priorities.

So the loop closes: NCP streams `(V,L,D,A)` → pid_vla decomposes it → a channel
priority/redundancy policy feeds back into the perception codec. PID is a
**design-time** tool (expensive, hard to estimate — pid_vla's whole domain), never
a per-tick runtime computation. It also serves as a **sim-vs-hardware fidelity
metric** (`NEUROMORPHIC.md` §5 in <https://github.com/sepahead/NCP>): does a neuromorphic chip preserve
the same information flow as the NEST simulator?

## Compatibility & versioning

Pinned to NCP **`v0.5.0`** (`Cargo.toml`), excluded from the default workspace.
This observer reads only the generic data planes
(`SensorFrame`/`CommandFrame`/`ObservationFrame` → opaque value/time vectors), so it
is unaffected by NCP's neural enums. The one frame the old `v0.1.0` pin could not
handle was an `ObservationFrame` whose `Observation.observable` is the `#10` value
(`binary_state`), which would fail to deserialize and be silently dropped; the
`v0.5.0` pin (proto-native wire) decodes it, so this observer now ingests `#10`
observations too. **Re-pin rule:** bump the `ncp-core`/`ncp-zenoh` tag in lockstep
with any future additive NCP wire extension *before* Engram emits it; no code change
is needed.

Engram is NEST today, but NCP's wire is **simulator-agnostic by design** — if a
future Engram backend (NEURON / Brian2 / GeNN / a neuromorphic chip) serves the
same wire, this observer is unchanged. That is exactly why it doubles as a
**sim-vs-hardware fidelity metric** (above): does the chip preserve the same
information flow the NEST simulator does?

## Build note

This crate git-depends on the published NCP repo <https://github.com/sepahead/NCP>
(tag `v0.5.0`) and pulls Zenoh, so it is heavier than the pure-PID crates. The
estimator gates (`just exp0`, etc.) target specific crates with `-p` and are
unaffected.
