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
(grandplan §9.1 warns against pre-labelling V/L/D; §9.2 pathway-source selection).

There are several producers of that contract:

| Producer | Role | Status |
|---|---|---|
| `experiments/safe_adapter/` (SAFE rollouts) | **Critical-path producer** (S2/EC1 reference adapter; grandplan §5.1, §8.7) | Implemented contract adapter with honest axis provenance; real capture and the diagnostic-noninterference preflight remain open |
| `crates/pid-sim` fixtures + Rapier/toy harnesses | Standalone sim sources | Software/conformance smokes, not scientific gate passes |
| **`crates/ncp-observer` (this)** | **Optional** Engram/NEST bridge | **Exploratory-only — below the S2/EC1 conformance bar (optional M2 ecosystem item)** |

`ncp-observer` is a **read-only passive tap**: it subscribes to a NEST/Engram session's
Neuro-Cybernetic Protocol (NCP) data planes over Zenoh and converts each closed-loop tick
into an `OfflineVldaSample`. It is **not on grandplan's critical path** (grandplan does
not depend on Engram at all), and root workspace resolution/build/test is independent of
NCP/Engram/Zenoh. That dependency firebreak does not imply a scientific PID gate pass.
Your job is to make this optional bridge *good enough to
feed the rigorous analysis*, not to make the project depend on it.

**Why it matters anyway:** a future, separately validated per-channel analysis could test
whether gated information summaries are useful for *design-time* NCP codec priorities under
a low-bandwidth link. The current observer flattens channels into V/L/D/A axes and the current
harness runs axis-pair screens; it does not yet implement a per-channel prioritization policy.
See
`crates/ncp-observer/README.md` and `RESILIENCE.md` in <https://github.com/sepahead/NCP>.

## 2. The adapter-side promotion bar (not EC1 completion)

An `ncp-observer` capture becomes a candidate for broader S2/EC1 conformance evaluation
when its artifact passes the offline harness's **strict leakage gates** and carries
**honest provenance**, i.e. it can be run with all of:

```
cargo run -p pid-sim --bin pid-offline-harness -- --input <ncp_vlda.json> \
  --require-success-labels --require-heldout-split \
  --require-heldout-class-coverage --require-heldout-episode-disjoint \
  --require-axis-provenance-honest
```

(`--require-axis-provenance-honest` is the opt-in gate — mirroring `--require-geometry-pass` —
that fails the run on degraded or absent axis-provenance markers; the `just safe-adapter`
recipe already runs it alongside the three held-out gates.)

…exiting 0. That establishes only the adapter-side prerequisites for H1/H2 baselines and the
**conditional H3 PID-necessity audit** (does gated PID/CI add value beyond the strongest valid
non-PID model; grandplan §3.8 PID kill rules, §6.5 baseline hierarchy). It does not clear the
population, measure, estimator, or application gates. Until then it is fine for *exploratory*
PID screens only; H1/H2 non-PID work may proceed with `--pid-mode none`.

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
  Run-log timestamps ride a monotonic clock (out-of-order sensor `t` values are clamped),
  so the emitted log passes `pid-runlog-replay --validate`.
- **Exact D alignment in prisoma, including arrival reordering**: plane-published
  `ObservationFrame.source` echoes the driving sensor `StreamPosition`; readouts are keyed
  by its `{epoch, seq}`. Completed ticks are held for a short reorder grace window so a
  matching readout arriving after its command still claims its own tick. A source-less
  pull/RPC observation and post-emission readouts are dropped/counted, never promoted by
  recency or patched into an already-logged row.
- **Bounded, reset-safe in-flight state**: FIFO (insertion-order) eviction and an explicit
  change of `stream.epoch` start a new incarnation (state cleared so epochs never cross-pair;
  `sample_id = ncp-{epoch}-{seq}` stays unique), and every exclusion/eviction path is counted in the
  `ObserverStats` finalize report.
- **Failure-safe finalization**: callbacks enqueue into a bounded handoff and one worker owns
  observer state. Canonical sample events remain buffered and immutable; artifact and run log
  are written via same-directory temporary files, flushed/fsynced, and atomically renamed.
  The first finalization attempt seals ingestion and binds its artifact path. Append/hash/write
  failures propagate without clearing samples, and exact same-path retries reconstruct the
  complete log without duplicates (including an exact install completed before a reported
  directory-fsync error).
- **Fail-closed ingress identity and transport**: `--secure` or `--open` must be chosen
  explicitly, unknown CLI options are rejected, realm/session key segments are validated, and
  each decoded `ObservationFrame.session_id` must equal the subscribed session (sensor/command
  session identity is carried only by the subscribed key). `--runlog` is mandatory, and the
  library refuses artifact finalization unless logging was attached before ingestion.

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
Tests cover in-order, reordered-within-grace, source-less pull-form drop, epoch isolation,
and immutable late-D paths.
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
- **Residual:** a conforming S2/EC1 producer must provide a genuine, dimensionally stable
  language channel for retained ticks. A zero/hash backfill is fabricated evidence and is
  not a conformance repair; keep exclusion as the permanent default.
- **Acceptance:** no sample ever reaches the artifact with a fabricated or empty L (met);
  the exclusion count makes the loss visible (met).

### Gap 3 — no held-out split / episode / label structure (MEDIUM; unlocks S2/EC1 analysis)
The artifact currently emits one optional `episode_id`, no `metadata.split`, and labels
only if a `success_channel` is configured — so the strict gates and the PID-necessity audit can't run.
- **Fix:** map NEST trials → `episode_id`; assign each sample a `metadata.split`
  (`train`/`test`, episode-disjoint); and source a real per-tick or per-episode `success`
  label from the task outcome (not a placeholder). Keep the split assignment deterministic
  and leakage-safe (no `episode_id` in both train and test).
- **Acceptance:** the five `--require-*` strict modes above all pass on a real session
  capture, and `heldout_logreg_vlda` (the learned H2-class baseline, grandplan §6.5) runs.
  This is necessary but not sufficient for H1–H4: protocol-specific capture/assignment and
  outcome machinery remain separate, and PID interpretation still requires all four gates.

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
   `grandplan.md` §9.2-style physics/world-model probe on the candidate ports —
   a pre-motor readout is the locus most at risk of measuring action formatting
   (see `RESEARCH_VLA_D_NCP.md` §6.1).
7. **Prefer exclusion over backfill for absent L.** The research memo's position
   (`RESEARCH_VLA_D_NCP.md` §6.2): if Engram never grows a real language channel,
   keep excluding absent-L ticks permanently and restrict Engram screens to
   D/V-involving atoms — do not accept any zero/hash proxy for L.

## 6. Build & run

`ncp-observer` is **kept off the default cargo workspace** (`Cargo.toml` `exclude`)
to keep NCP/Zenoh off the critical path; it git-depends on the published NCP repo
<https://github.com/sepahead/NCP> (tag `v0.8.0`). Build/test it explicitly:

```bash
# Build + test the workspace-excluded observer directly:
cargo build --manifest-path crates/ncp-observer/Cargo.toml
cargo test  --manifest-path crates/ncp-observer/Cargo.toml

# tap a live session, then analyze the artifact through the standard harness
# (the crate is workspace-excluded, so `-p ncp-observer` does NOT resolve from the
# repo root — always go through --manifest-path)
cargo run --manifest-path crates/ncp-observer/Cargo.toml --bin ncp-observe -- \
    --open --session <id> --out outputs/ncp_vlda.json --runlog outputs/ncp_runlog.jsonl
cargo run -p pid-sim --bin pid-offline-harness -- --input outputs/ncp_vlda.json \
    --summary-json outputs/ncp_summary.json --runlog outputs/ncp_pid_runlog.jsonl
```

The `ncp-core` / `ncp-zenoh` dependencies pin the immutable published `v0.8.0` tag in
lockstep. The crate stays off the default workspace so NCP/Zenoh resolution cannot break
root workspace resolution; scientific PID gates remain a separate question.

## 7. References

- `crates/ncp-observer/README.md` — what it does + the closed-loop payoff.
- `crates/ncp-observer/src/lib.rs` — `Observer` (full-`StreamPosition` source join,
  epoch-scoped `d_by_seq`, `emit_ready`).
- `NEURO_CYBERNETIC_PROTOCOL.md` in <https://github.com/sepahead/NCP> — the NCP spec (Gap 1 lives here).
- `experiments/safe_adapter/` — the reference `(V,L,D,A)` contract adapter to mirror for
  provenance and split/label structure; real capture and protocol preflights remain open.
- `EXPERIMENTS.md` §0.2 (runbook) and `grandplan.md` §3.8 (PID kill rules) + §6.5 (baseline
  hierarchy), §9.2 (pathway-source / D selection), §8.11 (one-control-plane rule).

## 8. Out of scope

- Making prisoma depend on Engram/NCP for any core result (it must stay standalone).
- Changing PID estimators, the harness gate logic, or the run-log schema.
- Any write/control path from the observer.
