# NCP Observer — Developer Handoff Prompt

> Copy-pasteable brief for a developer (or coding agent) bringing the `ncp-observer`
> Engram bridge up to the standard the pid_vla analysis requires. Self-contained;
> read it top to bottom before touching code.

## 1. Context (what this is and is not)

**pid_vla** is a Partial Information Decomposition (PID) toolkit for Vision-Language-Action
(VLA) policies. Its analysis is **source-agnostic**: everything downstream consumes
`(V,L,D,A)`+labels artifacts in one schema (the `OfflineVldaDataset` JSON the
`pid-offline-harness` reads). In `(V,L,D,A)`, **D is the Dynamics / internal-state
("hidden states") axis, not depth** — defined per model as an experimental variable
(grandplan §10.10.13, §7.6.3).

There are several producers of that contract:

| Producer | Role | Status |
|---|---|---|
| `experiments/safe_adapter/` (SAFE rollouts) | **Critical path** (grandplan M5) | Gate-passing, honest provenance |
| `crates/pid-sim` fixtures + Rapier/toy harnesses | Standalone sim sources | Gate-passing |
| **`crates/ncp-observer` (this)** | **Optional** Engram/NEST bridge | **Exploratory-only — below the M5 bar** |

`ncp-observer` is a **read-only passive tap**: it subscribes to a NEST/Engram session's
Neuro-Cybernetic Protocol (NCP) data planes over Zenoh and converts each closed-loop tick
into an `OfflineVldaSample`. It is **not on grandplan's critical path** (grandplan does
not depend on Engram at all), and the pure-PID stack builds/tests/gates green with no
NCP/Engram/Zenoh dependency. Your job is to make this optional bridge *good enough to
feed the rigorous analysis*, not to make the project depend on it.

**Why it matters anyway:** the payoff is a closed loop — pid_vla's PID screens quantify,
per NCP channel, the unique / redundant / synergistic information about the action, which
becomes a *design-time* channel-prioritization policy for NCP's perception codec under a
low-bandwidth link (drop redundant, keep unique, bundle synergistic). See
`crates/ncp-observer/README.md` and `Paper2Brain/ncp/RESILIENCE.md`.

## 2. The bar to clear ("done" = M5-grade)

An `ncp-observer` capture is "up to the task" when its artifact passes the offline
harness's **strict leakage gates** and carries **honest provenance**, i.e. it can be run
with all of:

```
cargo run -p pid-sim --bin pid-offline-harness -- --input <ncp_vlda.json> \
  --require-success-labels --require-heldout-split \
  --require-heldout-class-coverage --require-heldout-episode-disjoint
```

…exiting 0, and it can feed the **H1 necessity audit** (does PID/CI beat the SAFE-class
held-out logistic-regression failure detector — grandplan §14.1.1). Until then it is
fine for *exploratory* PID screens only.

## 3. Current state

Already correct (do not regress):
- **V↔A join on `seq`** — `CommandFrame.seq` echoes the `SensorFrame.seq` it was computed
  from, so a sample pairs the action with the sensor that produced it, never by arrival
  time (the perception plane's DROP QoS makes arrival-time pairing unsound).
- Deterministic channel ordering (sorted `BTreeMap` keys), read-only, System actor, and
  canonical run-log emission (`EmbeddingContract` / `EmbeddingCaptured` / `LabelObserved`).
- An **exact D-alignment path already exists in pid_vla**: `Observer` keeps `d_by_seq` and
  `try_complete` prefers it over the most-recent fallback — it just needs the publisher to
  stamp the driving `seq` (see Gap 1).

## 4. Workstreams (the three gaps, in priority order)

### Gap 1 — D alignment is best-effort (HIGH; this is the one that biases the PID)
`ObservationFrame` carries no `seq` today, so D — the internal-state axis the whole
`PID(V,D;A)` probe targets — is paired with the **most recent** observation
(`try_complete` falls back to `latest_d`), not the observation produced for the driving
`seq`. That biases every D-involving atom.
- **Paper2Brain/ncp side (the real fix):** stamp `ObservationFrame` with the driving
  sensor `seq` (publisher emits the `seq` of the tick the readout belongs to). Update
  `Paper2Brain/NEURO_CONTROL_PROTOCOL.md`.
- **pid_vla side:** already wired — `Observer::on_observation` stores `d_by_seq[obs.seq]`
  and `try_complete` consumes it. Confirm with a test once observations carry `seq`.
- **Acceptance:** a session where D readouts arrive out of order still pairs each sample's
  D with its own `seq` (extend the existing `d_aligns_on_seq_not_recency` test to a
  realistic interleaving); document any remaining best-effort fallback honestly.

### Gap 2 — L can be fabricated as zeros (MEDIUM; provenance)
`on_sensor` does `channels.get(language_channel).map(...).unwrap_or_default()`, so an
absent language channel yields an **all-zero L** — a degenerate axis. grandplan's M5
contract is "never fabricated."
- **Fix:** make the L policy explicit — either (a) require the language channel and skip /
  flag samples without it, or (b) carry an honest provenance marker in `metadata`
  (e.g. `l_source = "instruction" | "absent_zeroed" | "text_proxy"`) so downstream can
  exclude zeroed-L samples from L-atoms. Do **not** silently emit zeros as if they were a
  language embedding.
- **Acceptance:** no sample carries a zero L without an explicit provenance marker; the
  harness can filter on it.

### Gap 3 — no held-out split / episode / label structure (MEDIUM; unlocks the gates + H1)
The artifact currently emits one optional `episode_id`, no `metadata.split`, and labels
only if a `success_channel` is configured — so the strict gates and the H1 audit can't run.
- **Fix:** map NEST trials → `episode_id`; assign each sample a `metadata.split`
  (`train`/`test`, episode-disjoint); and source a real per-tick or per-episode `success`
  label from the task outcome (not a placeholder). Keep the split assignment deterministic
  and leakage-safe (no `episode_id` in both train and test).
- **Acceptance:** the four `--require-*` strict modes above all pass on a real session
  capture, and `heldout_logreg_vlda` (the H1 baseline) runs.

## 5. Hard constraints (do not violate)

1. **Run log is the source of truth.** Every captured sample must be reconstructable from
   the canonical run-log events; the JSON artifact is a convenience view.
2. **The observer drives nothing.** It is read-only; the Agent Bridge stays the *only*
   control plane. Never add a publish/command path here.
3. **The NCP-specific mapping lives in pid_vla** (`crates/ncp-observer`), not in Engram.
4. **Do not fabricate axes** (Gap 2) and **do not touch the `pid-core` estimators** — this
   is a capture-adapter task, not an estimator change.
5. **D is hidden states, not depth.** Keep the mapping and docs consistent with that.

## 6. Build & run

`ncp-observer` is **excluded from the default cargo workspace** (`Cargo.toml` `exclude`)
because it path-depends on the sibling `Paper2Brain/ncp` tree, absent on a fresh checkout.
Build/test it explicitly when the sibling is present:

```bash
# build + test (sibling Paper2Brain/ncp must be adjacent on disk)
cargo build --manifest-path crates/ncp-observer/Cargo.toml
cargo test  --manifest-path crates/ncp-observer/Cargo.toml

# tap a live session, then analyze the artifact through the standard harness
cargo run -p ncp-observer --bin ncp-observe -- \
    --session <id> --out outputs/ncp_vlda.json --runlog outputs/ncp_runlog.jsonl
cargo run -p pid-sim --bin pid-offline-harness -- --input outputs/ncp_vlda.json \
    --summary-json outputs/ncp_summary.json --runlog outputs/ncp_pid_runlog.jsonl
```

For an external standalone build (no sibling checkout), switch the `ncp-core` / `ncp-zenoh`
path deps in `crates/ncp-observer/Cargo.toml` to a git/crates.io dependency — at which
point `ncp-observer` can rejoin the default workspace and regain CI coverage.

## 7. References

- `crates/ncp-observer/README.md` — what it does + the closed-loop payoff.
- `crates/ncp-observer/src/lib.rs` — `Observer` (V↔A `seq` join, `d_by_seq`, `try_complete`).
- `Paper2Brain/NEURO_CONTROL_PROTOCOL.md` — the NCP spec (Gap 1 lives here).
- `experiments/safe_adapter/` — the gold-standard, gate-passing `(V,L,D,A)` producer to
  mirror for provenance and split/label structure.
- `EXPERIMENTS.md` §0.2 (runbook) and `grandplan.md` §14.1.1 (H1 kill criteria), §7.6.3
  (D hook selection), §11.4 (one-control-plane rule).

## 8. Out of scope

- Making pid_vla depend on Engram/NCP for any core result (it must stay standalone).
- Changing PID estimators, the harness gate logic, or the run-log schema.
- Any write/control path from the observer.
