# NCP Observer — Developer Handoff Prompt

> Copy-pasteable brief for a developer (or coding agent) bringing the `ncp-observer`
> Engram bridge up to the standard the prisoma analysis requires. Self-contained;
> read it top to bottom before touching code.

## 1. Context (what this is and is not)

**prisoma** is a Partial Information Decomposition (PID) toolkit for Vision-Language-Action
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

**Why it matters anyway:** the payoff is a closed loop — prisoma's PID screens quantify,
per NCP channel, the unique / redundant / synergistic information about the action, which
becomes a *design-time* channel-prioritization policy for NCP's perception codec under a
low-bandwidth link (drop redundant, keep unique, bundle synergistic). See
`crates/ncp-observer/README.md` and `RESILIENCE.md` in <https://github.com/sepahead/NCP>.

## 2. The bar to clear ("done" = M5-grade)

An `ncp-observer` capture is "up to the task" when its artifact passes the offline
harness's **strict leakage gates** and carries **honest provenance**, i.e. it can be run
with all of:

```
cargo run -p pid-sim --bin pid-offline-harness -- --input <ncp_vlda.json> \
  --require-success-labels --require-heldout-split \
  --require-heldout-class-coverage --require-heldout-episode-disjoint \
  --require-axis-provenance-honest
```

(`--require-axis-provenance-honest` is the opt-in gate — mirroring `--require-geometry-pass` —
that fails the run on degraded or absent axis-provenance markers; the `just safe-adapter`
recipe already runs it alongside the three held-out gates.)

…exiting 0, and it can feed the **H1 necessity audit** (does PID/CI beat the SAFE-class
held-out logistic-regression failure detector — grandplan §14.1.1). Until then it is
fine for *exploratory* PID screens only.

## 3. Current state

Already correct (do not regress):
- **V↔A join on `seq`** — `CommandFrame.seq` echoes the `SensorFrame.seq` it was computed
  from, so a sample pairs the action with the sensor that produced it, never by arrival
  time (the perception plane's DROP QoS makes arrival-time pairing unsound). `seq == 0`
  sensor/command frames are treated as unstamped (the upstream convention) and are
  dropped + counted rather than merged into one bogus tick.
- Deterministic channel ordering (sorted `BTreeMap` keys), read-only, System actor, and
  canonical run-log emission (`EmbeddingContract` / `EmbeddingCaptured` / `LabelObserved`,
  plus a finalize-time `ArtifactLogged` registering the dataset artifact with its sha256).
  Run-log timestamps ride a monotonic clock (out-of-order sensor `t` values are clamped),
  so the emitted log passes `pid-runlog-replay --validate`.
- **Exact D alignment in prisoma, including arrival reordering**: readouts are stored in
  `d_by_seq[obs.seq]` and preferred over recency; completed ticks are held for a short
  reorder grace window so a readout arriving after its tick's command still claims its
  own tick, and later-still readouts patch the in-memory sample (`d_source = "seq_late"`,
  counted). It just needs the publisher to stamp the driving `seq` (see Gap 1).
- **Bounded, reset-safe in-flight state**: FIFO (insertion-order) eviction, session `seq`
  resets start a new epoch (state cleared so epochs never cross-pair; `sample_id =
  ncp-{epoch}-{seq}` stays unique), and every exclusion/eviction path is counted in the
  `ObserverStats` finalize report.

## 4. Workstreams (the three gaps, in priority order)

### Gap 1 — D alignment on `seq` (DONE in-repo; residual is purely external)
`ObservationFrame` **carries `seq`**, and the prisoma observer joins D on it:
`Observer::on_observation` stores each readout in `d_by_seq[obs.seq]`, completed ticks
are held for a reorder grace window so a readout that arrives *after* its tick's
command (the likely ordering: the action plane outranks the observation plane in QoS)
still claims its own tick, and a readout arriving after emission still patches the
in-memory sample (`d_source = "seq_late"`, counted in the finalize report). The
most-recent readout (`latest_d`) is the fallback only for an unstamped
(`obs.seq == 0`) frame. Tests cover the in-order, reordered-within-grace, and
post-emission-patch paths, so nothing in **this repo** biases D-involving atoms
anymore. What remains:
- **NCP side (external — the only runtime gap), <https://github.com/sepahead/NCP>:** the
  publisher must actually stamp each `ObservationFrame` with the driving sensor `seq` at
  emission time (and `NEURO_CYBERNETIC_PROTOCOL.md` should document it). Until it does, a
  live session sends `obs.seq == 0` and the observer falls back to recency — so this, not
  any prisoma code, is what still biases D-involving atoms in a real capture.
- **Acceptance (met in-repo):** a session where D readouts arrive out of order still pairs
  each sample's D with its own `seq`; the recency fallback is reached only for genuinely
  unstamped frames.

### Gap 2 — absent-L ticks are excluded, not fabricated (residual is the retention policy)
A tick with no language channel yields an empty (zero-length) `L`. grandplan's M5
contract is "never fabricated", and the observer honors it the strict way: **empty-axis
ticks are excluded from the artifact and counted** (`excluded_empty_l` in the
`ObserverStats` finalize report) — one empty axis would make `pid-offline-harness`
reject the whole dataset anyway. Kept samples always carry the honest
`metadata.l_source = "channel"` marker.
- **Residual:** decide whether absent-L ticks should be *retained* via a fixed-dim
  zero backfill stamped `l_source = "absent_zeroed"` (which the harness's
  `--require-axis-provenance-honest` gate rejects as degraded), or whether exclusion
  (the current behavior) is the permanent policy. Exclusion is safer; backfill only
  helps exploratory screens on mixed-language sessions.
- **Acceptance:** no sample ever reaches the artifact with a fabricated or empty L (met);
  the exclusion count makes the loss visible (met).

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
3. **The NCP-specific mapping lives in prisoma** (`crates/ncp-observer`), not in Engram.
4. **Do not fabricate axes** (Gap 2) and **do not touch the `pid-core` estimators** — this
   is a capture-adapter task, not an estimator change.
5. **D is hidden states, not depth.** Keep the mapping and docs consistent with that.
6. **Do not call any record-port a principled D without a probe.** Today D is
   first-available-ports in BTreeMap order; before any world-model claim, run a
   `grandplan.md` §7.6.3-style physics/world-model probe on the candidate ports —
   a pre-motor readout is the locus most at risk of measuring action formatting
   (see `RESEARCH_VLA_D_NCP.md` §6.1).
7. **Prefer exclusion over backfill for absent L.** The research memo's position
   (`RESEARCH_VLA_D_NCP.md` §6.2): if Engram never grows a real language channel,
   keep excluding absent-L ticks permanently and restrict Engram screens to
   D/V-involving atoms — do not accept any zero/hash proxy for L.

## 6. Build & run

`ncp-observer` is **kept off the default cargo workspace** (`Cargo.toml` `exclude`)
to keep NCP/Zenoh off the critical path; it git-depends on the published NCP repo
<https://github.com/sepahead/NCP> (tag `v0.5.3`). Build/test it explicitly:

```bash
# build + test (pulls NCP from https://github.com/sepahead/NCP, tag v0.5.3)
cargo build --manifest-path crates/ncp-observer/Cargo.toml
cargo test  --manifest-path crates/ncp-observer/Cargo.toml

# tap a live session, then analyze the artifact through the standard harness
# (the crate is workspace-excluded, so `-p ncp-observer` does NOT resolve from the
# repo root — always go through --manifest-path)
cargo run --manifest-path crates/ncp-observer/Cargo.toml --bin ncp-observe -- \
    --session <id> --out outputs/ncp_vlda.json --runlog outputs/ncp_runlog.jsonl
cargo run -p pid-sim --bin pid-offline-harness -- --input outputs/ncp_vlda.json \
    --summary-json outputs/ncp_summary.json --runlog outputs/ncp_pid_runlog.jsonl
```

The `ncp-core` / `ncp-zenoh` deps in `crates/ncp-observer/Cargo.toml` resolve from the
published NCP repo <https://github.com/sepahead/NCP> (tag `v0.5.3`), so no sibling
checkout is needed; the crate is kept off the default workspace to keep NCP/Zenoh off the
critical path.

## 7. References

- `crates/ncp-observer/README.md` — what it does + the closed-loop payoff.
- `crates/ncp-observer/src/lib.rs` — `Observer` (V↔A `seq` join, `d_by_seq`, `try_complete`).
- `NEURO_CYBERNETIC_PROTOCOL.md` in <https://github.com/sepahead/NCP> — the NCP spec (Gap 1 lives here).
- `experiments/safe_adapter/` — the gold-standard, gate-passing `(V,L,D,A)` producer to
  mirror for provenance and split/label structure.
- `EXPERIMENTS.md` §0.2 (runbook) and `grandplan.md` §14.1.1 (H1 kill criteria), §7.6.3
  (D hook selection), §11.4 (one-control-plane rule).

## 8. Out of scope

- Making prisoma depend on Engram/NCP for any core result (it must stay standalone).
- Changing PID estimators, the harness gate logic, or the run-log schema.
- Any write/control path from the observer.
