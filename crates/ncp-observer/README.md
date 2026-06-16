# `ncp-observer` — passive Neuro-Control Protocol tap for pid_vla

Makes **Engram** (a NEST spiking network, exposed over the Neuro-Control Protocol)
another `(V,L,D,A)` source for pid_vla's Partial Information Decomposition — the
same role `experiments/safe_adapter` plays for SAFE rollouts. It is a **read-only
observer**: it subscribes to the NCP data-plane keys over Zenoh and never drives
anything (the Agent Bridge stays the only control plane).

It uses the canonical Rust NCP SDK (`ncp-core` + `ncp-zenoh`) from the sibling
**`Paper2Brain/ncp`** workspace. Spec: `Paper2Brain/NEURO_CONTROL_PROTOCOL.md`.

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
pairing unsound). `ObservationFrame` carries no `seq` yet, so D is paired with the
most recent observation (best-effort); precise D alignment (stamp observations
with the driving `seq`) is a noted protocol enhancement.

## Run

```bash
# tap a live session and write the artifact + run log on Ctrl-C
cargo run -p ncp-observer --bin ncp-observe -- \
    --session uav3 --out outputs/ncp_vlda.json --runlog outputs/ncp_runlog.jsonl
# then run the PID screens on it
cargo run -p pid-sim --bin pid-offline-harness -- --input outputs/ncp_vlda.json \
    --summary-json outputs/ncp_summary.json --runlog outputs/ncp_pid_runlog.jsonl
```

## Build note

This crate depends on the sibling `Paper2Brain/ncp` workspace (path dependency) and
pulls Zenoh, so it is heavier than the pure-PID crates. The estimator gates
(`just exp0`, etc.) target specific crates with `-p` and are unaffected. For a
standalone build, switch the `ncp-core`/`ncp-zenoh` path deps to a git/crates.io
dependency.
