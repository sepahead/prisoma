<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo-dark.svg">
    <img alt="Prisoma logo — the summit prism from the sepahead project graph: incoming beams meet a rising mirror blade on a violet hill and disperse into their spectral component rays." src="assets/logo-light.svg" width="200">
  </picture>
</p>

# prisoma

> **Rust PID estimators + Python bindings live in [`pid-rs`](https://github.com/sepahead/pid-rs) — the single source of truth.**
> `pid-core`, `pid-runlog`, and the `pid-python` (`pid_core_rs`) bindings are **not** vendored here;
> they are pinned as the `pid-rs/` git submodule. After cloning: `git submodule update --init`.
> The local crates (`pid-sim`, `pid-rerun`, `pid-bridge`) path-depend into `pid-rs/crates/*`, and the
> estimator binaries run from the submodule, e.g.
> `cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all --bin exp0`.
> Build the Python module from the submodule: `maturin develop -m pid-rs/crates/pid-python/Cargo.toml`.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](#license)

prisoma is a research toolkit providing **auditable experiment semantics** for **intervention‑grounded diagnosis** of **Vision‑Language‑Action (VLA)** policies: a provenance‑complete capture–intervention–replay substrate for testing whether genuinely pre‑treatment diagnostics predict intervention response and future failure beyond strong baselines. **Partial Information Decomposition (PID)** — the shared‑exclusions measure `I^sx_∩` — is one **conditional** candidate diagnostic, central only if it passes preregistered population, measure, estimator, and application gates (`grandplan.md` §7.1). The project is **gate‑driven**: PID atoms are never interpreted on real embeddings until those gates pass; confirmatory claims are bound by the `grandplan.md` §4 claim registry (EC1, H1–H4), the §3.8 PID kill rules, and the §6 statistical analysis plan; and negative results are first‑class publishable outcomes.

## Documentation map

Read these in order of what you need. `grandplan.md` is canonical; the others are kept consistent with it.

| Document | What it is |
|---|---|
| `grandplan.md` | Canonical spec — definitions, gates, hypotheses, engineering plan |
| `EXPERIMENTS.md` | What to run + what to log (protocols; runbook = executable-today vs blocked) |
| `ARCHITECTURE.md` | Target system design (PID‑Splat) |
| `DIAGRAMS.md` | Architecture + control-plane diagrams (status dashboards up top) |
| `pidsplatspecs.md` | Simulation/spec details (PID‑Splat) |
| `findings.md` | Current estimator-status evidence (Exp0 results + interpretation) |
| `REVIEW_AND_TODO.md` | Whole-repo review, prioritized to-do list, current critical path |
| `docs/CAPABILITY_MATRIX.md` | Generated, content-bound current capability/evidence inventory |
| `AGENTS.md` | Ground rules + a detailed "what actually exists" inventory for contributors |
| `NCP_DEV_PROMPT.md` | Optional: dev handoff for the Engram/NCP `(V,L,D,A)` bridge |
| `uidesigner/UI.md` | UI/UX spec (viewer-first; ordered by milestones) |
| `GAUSS_MI_INTEGRATION.md`, `WORLD_WARP_INTEGRATION.md` | Optional add-on specs (3DGS reconstruction-quality study; external world-model baseline) |
| `THIRD_PARTY_NOTICES.md` | Release-governance notices/checklist |

## Prerequisites

- **Rust** — a stable toolchain new enough for the current local dependency graph (Rerun
  0.28 declares Rust 1.88). The root workspace does not currently declare an enforced MSRV;
  the separate `pid-rs` workspace targets 1.80. Install via [rustup](https://rustup.rs/).
- **Git submodule** — `git submodule update --init` after cloning (fetches `pid-rs`, the estimator core). There are no nested submodules, so `--recursive` is not required. The submodule URL is SSH (`git@github.com:sepahead/pid-rs.git`); if you cloned over HTTPS without SSH keys, either configure SSH or add a `git config --global url."https://github.com/".insteadOf git@github.com:` rewrite first.
- **Python 3.11+** with [`uv`](https://docs.astral.sh/uv/) — only for the `experiments/` (SAFE adapter + attribution probe) and the doc-audit scripts. `numpy` is the sole hard runtime dep; `uv sync` installs the dev/analysis groups.
- **`just`** (optional) — a task runner; every `just` recipe below has a plain `cargo`/`python` equivalent. Install with `cargo install just`.
- **`maturin`** (optional) — only to build the Python bindings (`pid_core_rs`) from the submodule.

Verify the toolchain and see the estimator gate fire:

```bash
git submodule update --init
cargo test --workspace
cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all --bin exp0   # prints the GO/PIVOT/NO-GO verdict
```

## Current Status & What To Do, In Order (docset v12.5, 2026-07-12)

**Status at a glance:**

- **Implemented, with passing baseline tests:** the Rust estimator, run-log/replay,
  bridge/sim/Rerun groundwork, offline `(V,L,D,A)` harness, Rapier manipulation, SAFE adapter,
  and reference attribution probe. Passing tests establish current behavior, not production or
  scientific validity; the dated code review lists unresolved integrity/security blockers.
- **Four separated PID gates** (`grandplan.md` §7.1: population, measure, estimator, application).
  The high-dimensional **MI/coherence (estimator) path is NO-GO**; continuous **`I^sx_∩` atoms on
  real embeddings are BLOCKED / NOT APPLICATION-VALIDATED**: default Experiment 0 includes a
  measure-mismatched zero-redundancy target, while `--strict-gate` enforces the curated
  low-dimensional MI band and only reports atoms. See `findings.md`; never quote the binary's
  aggregate label as an atom-validity verdict.
- **Docset v12.5 (2026-07-12)** adopts the second-round adversarial review: the thesis is reframed
  around auditable experiment semantics with PID as a conditional candidate, replacing the v10.7
  plan (archived at `docs/archive/grandplan-v10.7.md`; review bundle at
  `docs/reviews/2026-07-12-grandplan-v12.5/`). Key changes: the EC1 + H1–H4 confirmatory registry
  (Protocol A/B split for H1, censoring-aware H2, conditional H3, availability–use H4), the S0–S7
  gate sequence, M0–M7 milestones, four PID gates, an E0–E5 ecosystem evidence ladder, and a
  dependency firebreak. Earlier docset history (v10.4–v10.7) is in `CHANGELOG.md`.
- **Open critical path:** do **not** begin an evidentiary real-VLA capture yet. Required first (S0–S3):
  repair the upstream continuous application gate; implement leakage-safe episode-local H1 scores
  plus action-entropy and ensemble/temperature baselines; freeze transforms and task eligibility;
  and replace the implemented idealized power tool with the nested capture design in
  `grandplan.md` §6.8. The first power report is overall NOT PASSED and all of its first-run grid
  counts are withdrawn as capture requirements.

```mermaid
flowchart LR
    classDef run fill:#1b5e20,stroke:#2e7d32,color:#fff;
    classDef gate fill:#e65100,stroke:#ef6c00,color:#fff;
    classDef blocked fill:#7f1d1d,stroke:#b71c1c,color:#fff,stroke-dasharray:5 3;

    Exp0["S1 estimator gate (Exp0 diagnostics)<br/>MI: NO-GO; I^sx: BLOCKED<br/>(runnable: just exp0-bin)"]:::gate
    Harness["Offline (V,L,D,A) harness<br/>+ baselines + attribution<br/>+ axis-provenance gate ENFORCED<br/>(EC1 groundwork, runnable today)"]:::run
    Adapter["safe_adapter → contract (S2/EC1)<br/>bounded hash-manifest ingress<br/>honest provenance<br/>(runnable fixture: just safe-adapter)"]:::run
    Capture["OPEN CRITICAL PATH (S3→S4)<br/>gate + endpoint + power repairs,<br/>then real VLA capture"]:::blocked
    Exps["H1 / H2 / H4 studies<br/>(blocked on endpoint + capture work)"]:::blocked
    H3["H3 conditional PID increment<br/>(also blocked on all four PID gates)"]:::blocked

    Harness --> Adapter
    Adapter --> Capture
    Capture -. blocks .-> Exps
    Capture -. blocks .-> H3
    Exp0 -. gates PID only .-> H3
```

*Caption: Runnable plumbing is not a scientific pass. H1/H2/H4 remain blocked on their
protocol, endpoint, power, and capture prerequisites, but they do not wait for PID; H3 also waits
for all four PID gates.*

Each step gates the next; canonical depth is in `grandplan.md` at the cited sections.

1. **Verify the toolchain and inspect diagnostics:** `cargo test`, then `just exp0` /
   `just exp0-bin`. The printed aggregate is diagnostic output, not a valid `I^sx` verdict;
   the current split status is MI NO-GO / `I^sx` application gate BLOCKED (`grandplan.md`
   §7.2, §7.9; `findings.md`).
2. **Learn the measurement-regime rules before touching real data:** one (PID measure, preprocessing, estimator config) tuple = one preregistered regime; never pool or compare continuous `I^sx_∩` atoms with discrete `I_min` atoms as if they were one quantity — `--pid-mode discrete` is Williams–Beer `I_min`, not discrete `i^sx_∩` (`grandplan.md` §7.6); supervised projections (PLS) are fit on training samples only and re-gated (`grandplan.md` §6.2).
3. **Exercise plumbing on checked fixtures:** strict geometry and discrete fixtures intentionally
   warn/fail. Their thresholds are not validated scientific gates, and discrete saturation is
   currently advisory rather than a strict failure path.
4. **Prepare, but do not treat as evidentiary capture yet:** the SAFE adapter and Rapier path
   can exercise the EC1 contract on checked fixtures. SAFE ingress now requires a bounded
   NPZ/strict-JSON bundle plus exact file hashes and operator-declared source/split/rights and
   model/checkpoint/hook/tensor receipts; downloaded pickle
   is rejected by default. Real SAFE use still requires an isolated safe re-export where needed,
   exact revision and split receipts, and a rights review. H1/H4 wait for their protocol and capture blockers; H2 now has
   a synthetic fixed-horizon protocol-arithmetic reference, but real H2 still waits for its domain
   freeze, capture, comparator, and external-validation blockers. H3 also waits for all four PID
   gates. The harness supports `--pid-mode none` so non-PID work continues.
5. **Analyze only after gates exist:** geometry diagnostics do not currently select a valid
   regime. The m-out-of-n raw percentile output is a stability envelope at size m, not an
   n-sample confidence interval; endpoint inference must resample the correct outer units.
6. **Run the non-PID baselines every time:** majority/1-NN/centroid baselines *and* a SAFE-class logistic-regression internal-feature failure detector (surfaced under the `heldout_logreg_vlda_success_*` metric names) are built into the harness; add one faithfulness-checked attribution baseline (`experiments/attribution/`, an AttnLRP protocol, `grandplan.md` §6.10, §10.2; `just attribution-probe`). The preregistered PID kill rules (`grandplan.md` §3.8) decide whether PID atoms earn a place in any claim — a negative answer is a publishable outcome.
7. **Only then** run the H1–H4 study protocols in `EXPERIMENTS.md` (see its runbook for what is executable today vs blocked on step 4).

## Confirmatory claim registry (Docset v12.5)

The canonical registry and its claim-to-evidence matrix live in `grandplan.md` §4 (with the §3.8 PID kill rules); the preregistered statistical analysis plan (estimands, endpoints, multiplicity, power gates) is `grandplan.md` §6. The thesis holds no more than three confirmatory scientific claims; engineering acceptance (EC1) is separate.

| Claim | One‑line testable claim | Type | Status |
|---|---|---|---|
| **EC1** | **Provenance-complete replay** — the capture/intervention/replay contract records the declared causal + temporal variables, detects contract violations, and reproduces exact events or tolerance-bounded outcomes, benchmarked against conventional scripts and standard containers. | Engineering acceptance | Run-log/replay groundwork implemented; external benchmark pending |
| **H1** | Genuinely **pre-treatment** diagnostics predict intervention response — **Protocol A** (paired frozen-snapshot algorithmic sensitivity) and/or **Protocol B** (randomized closed-loop effect modification), scored by effect-specific criteria, not factual-outcome fit. | Confirmatory | Blocked on pilot + capture |
| **H2** | Diagnostics improve **prospective, censoring-aware** failure prediction beyond strong baselines (Tri-Info / SAFE / Hide-and-Seek / ActProbe / Rewind-IL / VLAConf / Foresight …) under a frozen alarm policy (with process-level safety cost as a decision-utility adjunct, not the headline claim). | Confirmatory | Synthetic protocol reference runnable; real claim blocked on domain freeze + capture + external validation |
| **H3** | PID adds **incremental value only inside its validated support envelope** (all four gates), vs MI/CMI, uncertainty, temporal, geometry, attribution, and learned baselines. | Conditional | Blocked on the estimator application gate |
| **H4** | Representational **availability** (held-out decodability) can diverge from causal **policy use** — the availability–use gap. Replaces H3 as a thesis paper if PID fails; a first-order problem, not a consolation prize. | Confirmatory / fallback | Blocked on capture |

**Exploratory:** memorization under structured perturbation; temporal transitions before failure; low-dimensional object/contact flow as a portable target; process-level safety cost; cross-embodiment transport of relationships (not raw atom magnitudes); diagnostic-guided intervention/fallback selection.

**Retired/deferred:** real-time continuous PID as an online safety monitor; PID safety certification; full three-source PID as a required analysis; atom signs as direct evidence of memorization/grounding/world-modeling; universal cross-model atom comparisons; a custom simulator/Tauri shell/SparkJS renderer/Gaussian-splat editor as a thesis dependency (`grandplan.md` §4).

PID is **forced nowhere**: `grandplan.md` §3.8 records the PID kill rules and §4's claim-to-evidence matrix records, per claim, the minimal evidence, replication requirement, and main disqualifier. Attribution methods are comparison evidence, not a shortcut around PID validity, and any reasoning/trace claim must be grounded in action and counterfactual effects (`grandplan.md` §10.2). Disagreement under controlled interventions is itself a diagnostic result.

## Experiments (Run Order)

Details and logging requirements live in `EXPERIMENTS.md`; estimator gates and confounds live in `grandplan.md`.

1. **Exp0 — PID population/measure/estimator/application diagnostics (S1).** GO/PIVOT/NO‑GO. *Runnable today* (`just exp0-bin`); current verdict on synthetic high‑d controls remains **NO‑GO** under the pinned pid-rs 1.0 environment (`findings.md`). No PID atom or H3 result is interpretable without all four gates; EC1 and the non-PID H1/H2/H4 paths continue with PID disabled (`grandplan.md` §7, §8.9.3).
2. **EC1 capture/replay + adapter (S2).** The offline `(V,L,D,A)` harness, bounded/content-addressed SAFE synthetic-bundle path, and sim/Rapier `Flow_gt` cross‑checks are *runnable today* (`just safe-adapter`, `just runlog-sim-verify`); real SAFE ingestion, the external infrastructure benchmark, and a second adapter are pending (`grandplan.md` §8.8).
3. **Intervention pilot (S3).** Dose / target‑engagement / placebo / OOD checks on one interpretable intervention. *Blocked on capture* (`grandplan.md` §5.4, §5.6).
4. **H1 — pre‑treatment diagnostics predict intervention response** (Protocol A paired and/or Protocol B randomized). The common preflight and deterministic synthetic Protocol A scoring reference are fixture-runnable, but neither real/evidentiary response protocol is implemented; scientific H1 remains *blocked on pilot + capture* (`grandplan.md` §6.3).
5. **H2 — prospective, censoring‑aware failure prediction** vs the comparator frontier. The
   deterministic synthetic fixed-horizon/IPCW/alarm reference is fixture-runnable
   (`just h2-reference`), while real/evidentiary H2 remains *blocked on domain freeze, capture,
   comparator completion, and external validation* (`grandplan.md` §6.4).
6. **H3 or H4 — conditional PID incremental value, or the availability–use gap.** *Blocked on the estimator application gate / capture* (`grandplan.md` §7, §4).
7. **Transport replication (S7)** — second task family, policy, simulator, or embodiment; mind the embodiment‑in‑`L` confound. *Blocked on capture* (`grandplan.md` §5.11).

## Doc Audits

- `python scripts/audit_grandplan.py` (validates the R1–R112 reference ledger: contiguous IDs, all defined + cited, no undefined/unused/duplicate, URLs)
- `python scripts/audit_grandplan_claims.py` (heuristic scan for unqualified venue/perf claims)
- `python scripts/audit_docset_claims.py` (same heuristic scan across the canonical docset + `findings.md`)
- Full tracked-Markdown sweep: `python scripts/audit_docset_claims.py --paths $(git ls-files '*.md')`
- With `just`: `just docs-audit` (runs the four repository audits, including pinned-dependency truth)

## What Actually Exists

The authoritative, detailed inventory is in **`AGENTS.md`** ("Repo reality"). In brief:

- **Implemented (Rust, in the pinned `pid-rs` 1.0 submodule):** `pid-core` (stable report-first KSG/categorical/quantized surfaces plus default-off continuous shared exclusions, PLS/pipelines, hierarchy, and hyperbolic research features; discrete SxPID `i^sx_∩` supports 2–4 sources but is *not yet wired into the offline harness*), `pid-python` (a typed stable `pid_core_rs` surface for `compute_mi_report`, categorical SxPID/`I_min`, fitted quantization, and diagnostics; pre-1.0 scalar calls exist only in the default-off migration module), and `pid-runlog` (the canonical EC1 JSONL run-log schema + replay/validate/compare/summary/manifest/sidecar CLI).
- **Implemented (Rust, local crates):** `pid-bridge` (Agent Bridge dispatch/JSON-RPC-shaped
  conversion/contract export), `pid-sim` (deterministic sim, real optional Rapier backend,
  manipulation harness, transports, offline VLDA screens, a fail-closed H1 common-preflight,
  validator/CLI for content-addressed fixture plumbing and diagnostic-instrumentation
  noninterference, a deterministic finite-benchmark Protocol A software-reference runner, and a
  PID-free deterministic H2 fixed-horizon cumulative-incidence/IPCW/alarm software reference),
  and `pid-rerun`. The reference runner exact-binds a passed schema-v2 representative-mechanism
  preflight, restores independent clone states, reverses order, records zero RNG draws, and performs
  fixed out-of-fold proper scoring. It is synthetic scoring plumbing—not a subprocess/stochastic
  audit, physical individual effect, Protocol B implementation, or H1 scientific evidence. The H2
  reference exact-binds separate plan/ontology/feature/split artifacts and exercises grouped
  weighted fitting, stratified reverse-KM IPCW, competing events, reliability bins, frozen alarm
  semantics, nondetection retention, and declared-payoff utility on synthetic fixtures. It is not
  prospective capture, validated calibration, the comparator frontier, or H2 scientific evidence. Implemented
  baselines are majority, 1-NN, nearest-centroid, and held-out logistic regression; action
  predictive entropy and ensemble/temperature uncertainty are still missing. The code review
  also identifies network-authentication, transactional logging, reconstructability, and
  artifact-integrity work before production use.
- **Source-agnostic capture:** the analysis consumes one `(V,L,D,A)`+labels contract, so producers are pluggable. The **reference producer is `experiments/safe_adapter/`** (the S2/EC1 adapter); its checked path is a finite synthetic canonical bundle, while real downloaded data remain a gated ingress/capture step. `pid-sim` fixtures + the Rapier/toy harnesses are standalone sim cross-checks. In `(V,L,D,A)`, **D is the hidden-state / dynamics axis, not depth**, and semantic labels require architecture evidence (`grandplan.md` §9.1, §3.5).
- **Optional NCP observer:** `crates/ncp-observer` is a read-only tap for a conforming NCP
  producer (an E2 dependency edge to NCP itself, `grandplan.md` §8.9), excluded from the default
  workspace and off the critical path. The public `sepahead/engram` repository remains a
  README-only placeholder; there is no public live Engram producer or Prisoma integration. The observer's
  integrity repair ships against wire 0.8, pinned to the immutable NCP `v0.8.0` release:
  full-`{epoch,seq}` V/L/D/A buffering, sensor-authorized transitions, immutable rows/events,
  complete-frame duplicate/conflict receipts, observer-owned raw fault accounting, finite
  resident/output ceilings, and a canonical artifact/run-log bundle committed by a verified
  publication receipt. Known failed/zero-row captures remain diagnostic and the CLI exits nonzero; the
  offline harness rejects uncommitted or failed NCP input. `capture_integrity` covers visible
  receipts/join state only—whole-plane gaps, receipt timing/QoS/reconnect evidence, and peer
  authentication remain unassessed. A deterministic, bounded `ncp-fault-observatory` now replays
  18 frozen wire-0.8 fixture scenarios twice through the shared route/raw-ingress seams and
  publishes strict per-replay outcome records plus a hash-bound, receipt-last report. It explicitly
  separates injection truth from native detection: whole-tick omission is a manifest-only known
  limitation, logical slots are annotations that do not drive or measure timing, trace truncation
  is not a live disconnect, and the security case guards only a declared-profile label without
  loading or selecting a configuration. This is local E3-style fixture evidence only—not E4,
  EC1 completion, live Engram validation, security validation, or a PID gate change. No population support is
  inferred: continuous KSG/shared-exclusions requests abstain, `--pid-mode none` requests nothing,
  and quantized discrete `I_min` remains a non-evidentiary diagnostic with population
  `NotEvaluated` and application `Blocked`. It remains exploratory because honest
  L/split/episode/label structure and a conforming live publisher are still required before it
  can be an S2/EC1 producer.
  The E3-style label is emitted only when build/runtime revisions agree, both worktree states are
  clean, and the lockfile plus exact executable hashes are recorded; otherwise the report uses a
  reproducibility-unqualified typed level. This is a local reproducibility binding, not signing or
  remote attestation. `--verify DIR` read-only snapshots an in-place receipt-bound bundle and every
  nested artifact without rerunning the suite; only explicit `--out-dir` recovery may discard the
  writer's reserved partial temporary files after reconstructing their targets. The frozen outcome
  inventory is 16 assessed (15 matched and one matched known limitation for whole-tick omission),
  two expected `not_assessable` guards (logical pause and security-profile claim), and zero
  mismatches. `all_expectations_matched=true` means those classifications held, not an 18/18
  detection rate.
- **Specified (not yet built):** a fuller Rerun-based diagnostic viewer and the deferred
  Tauri/SparkJS UI. Start at `grandplan.md` §12 (milestones) and §8.10 (current vs target).

## Quick Start — Exp0 Gate

```bash
# optional: nix develop
cargo test
just exp0        # estimator smoke tests
just exp0-bin    # prints the GO/PIVOT/NO-GO verdict
just exp0-runlog # exports + validates canonical Exp0 evidence
```

Without `just`: `cargo test`, then `cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all --bin exp0`. To export canonical Exp0 evidence:

```bash
cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all --bin exp0 -- \
  --summary-json outputs/exp0_summary.json --runlog outputs/exp0_runlog.jsonl
cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- \
  --validate outputs/exp0_runlog.jsonl
```

See `findings.md` for the latest repo-local Exp0 interpretation notes.

## Quick Start — Tiny Labeled Harness

```bash
just toy-harness
```

Without `just`: `cargo run -p pid-sim --bin pid-toy-harness -- --summary-json outputs/toy_vla_summary.json --runlog outputs/toy_vla_runlog.jsonl`, then validate with `pid-runlog-replay --validate outputs/toy_vla_runlog.jsonl`. This is a deterministic toy task, **not VLA evidence** — it exercises label events, a replay-linked `(V,L,D,A)` contract, PID/CI features, non-PID baselines, summary artifacts, and canonical run-log export end to end.

## Quick Start — H1 Common Preflight

```bash
just h1-preflight
```

The recipe runs a passing fixture plus semantic/artifact- and parse-rejection fixtures, validates every
resulting run log, checks deterministic output, and asserts zero PID events. Without `just`:

```bash
cargo run -p pid-sim --bin pid-h1-preflight -- \
  --input INPUT --summary-json SUMMARY --runlog RUNLOG
```

Artifact paths resolve below the input directory unless `--artifact-root` is supplied.
The CLI verifies declared artifact bytes and shared structural/noninterference requirements only.
It neither executes nor clears Protocol A or B. Readable invalid contracts produce canonical failed
logs; missing or unreadable input files remain ordinary CLI I/O errors.

## Quick Start — H1 Protocol A Software Reference

```bash
just h1-protocol-a
```

This first runs the common preflight, exact-binds its content-addressed chain, executes the checked
deterministic synthetic finite benchmark, verifies byte-repeatable canonical logs, and exercises
preflight-binding and parse failures. The emitted response and proper-score numbers validate the
software primitive only; `synthetic_fixture_only=true` and `establishes_h1_evidence=false` are
binding. Real Protocol A capture/analysis and all Protocol B execution remain unimplemented.

## Quick Start — H2 Fixed-Horizon Software Reference

```bash
just h2-reference
```

The recipe exact-binds four outcome-independent artifacts (analysis plan, event ontology, feature
contract, and split manifest), then runs complete-follow-up and censored synthetic artifacts plus
readable parse and semantic-lineage failures; the Rust suite adds a positive multi-landmark alarm
boundary fixture. The combined checks exercise task-family-held-out weighted logistic models, grouped
cross-fitted stratified reverse-Kaplan–Meier censoring weights, Horvitz–Thompson IPCW Brier scores,
competing events as observed non-target outcomes, reliability bins, frozen alarm accounting,
persistence/refractory/capacity and positive matching boundaries, detected/undetected records
without numeric lead-time placeholders for misses, and declared-payoff utility.
The censored fixture retains the censored landmark in the estimand denominator while emitting no
numeric row loss; alarm/utility accounting abstains when follow-up is incomplete. Every run log is
PID-free and contains no action or intervention event.

These are deterministic software-reference numbers only. The binding flags remain
`synthetic_fixture_only=true`, `establishes_h2_evidence=false`, `prospective_capture=false`,
`external_validation=false`, and `comparator_frontier_complete=false`. Real H2 still requires a
domain-specific estimand/ontology freeze, powered prospective capture, full calibration and
censoring sensitivity, the matched-access comparator frontier, and an untouched later/external
holdout.

## Quick Start — Offline (V,L,D,A) Embedding Harness

```bash
just offline-harness
just offline-harness-require-labels
just offline-harness-require-heldout
just offline-harness-require-heldout-class-coverage
just offline-harness-require-heldout-episode-disjoint
just offline-harness-strict            # asserts the expected geometry-gate failure
just offline-harness-highdim
just firebreak                       # --pid-mode none; asserts zero MI/PID events
just offline-harness-discrete
just offline-harness-discrete-pls
```

**PID estimator modes** (`--pid-mode`): `none` (skip every MI/PID estimate), `continuous`,
`discrete` (`I_min`, a different measure), and `discrete-pls`. PLS selection accepts
`--pls-components N|cv|cv:MAX`; all fitted
transforms still need a frozen train-fit/apply-held-out scientific path. Discrete saturation
warnings mark non-evidence but do not currently fail the CLI, so discrete mode is not an
active-regime gate. Permutation choices are `--permutation-scheme
full-shuffle|circular-shift`: full shuffle assumes IID/exchangeable rows; circular shift
requires an approximately stationary series and is not a grouped-episode null.

**Input schema.** A JSON object with optional `run_id`/`source`/`model`/`task` and a `samples` array. Each sample carries `sample_id`, optional `episode_id`, numeric `v`/`l`/`d`/`a` vectors, optional `labels`, and optional string `metadata`. `metadata.split` values recognized as **train**: `train`, `training`; as **held-out**: `test`, `validation`, `val`, `eval`, `evaluation`, `heldout`, `holdout`, `held_out`, `hold_out`.

**What it computes.** All two-source `V/L/D→A` screens — `(V,L;A)`, `(V,D;A)`, `(L,D;A)` — after deterministic per-variable standardization, with geometry diagnostics/gates over the standardized space. When a recognized metadata split is present, it also emits train-split-only PID screens (fit with train-only standardizers, so held-out embeddings are excluded from both preprocessing and PID evidence).

**Baselines (when every sample has a boolean `success` label).** Success rate + majority accuracy; sample-level leave-one-out 1-NN; leakage-resistant leave-one-episode-out majority/1-NN (when every sample has an `episode_id` and there are ≥2 distinct episodes); and true held-out majority/1-NN + train-standardized nearest-centroid + a SAFE-class held-out logistic-regression detector (when the split is present). Held-out baselines emit accuracy and balanced accuracy when both classes are present; centroid baselines also emit AUROC. The summary/run log preserve split counts, train/held-out IDs, class-coverage and episode-disjointness status, held-out per-sample prediction records, and failure-class confusion/rate diagnostics.

**Strict modes (fail closed).** `--require-success-labels`, `--require-heldout-split`, `--require-heldout-class-coverage`, `--require-heldout-episode-disjoint`, `--require-geometry-pass`, and `--require-axis-provenance-honest` each fail the run (while still writing a valid *failed* run log) when their invariant is violated.

Without `just`:

```bash
cargo run -p pid-sim --bin pid-offline-harness -- \
  --input crates/pid-sim/fixtures/offline_vlda_fixture.json \
  --summary-json outputs/offline_vlda_summary.json --runlog outputs/offline_vlda_runlog.jsonl
cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay -- \
  --validate outputs/offline_vlda_runlog.jsonl
```

The harness is an artifact-to-runlog converter for captured embeddings, **not** evidence from a real VLA by itself.

## Quick Start — M1 Run Log & Agent Bridge

```bash
just runlog-demo               # emit + a deterministic sim run log
just bridge-contract           # export the Agent Bridge JSON-RPC contract
just runlog-replay             # replay
just runlog-validate           # validate
just runlog-bridge-demo
just runlog-bridge-stdio       # drive the bridge over stdio JSON-RPC
just runlog-bridge-stdio-safe  # same, in read-only safe mode
just runlog-summary
just runlog-manifest
just runlog-sidecars
just runlog-sim-verify
just runlog-rerun              # convert a run log to a Rerun .rrd
just runlog-rerun-bridge
just runlog-bridge-export-rerun
```

> **Note:** `just runlog-bridge-tcp` and `just runlog-bridge-ws` start a server that **blocks waiting for one client to connect** — they do not self-terminate. Run them in a separate terminal and connect a client (the CI job in `.github/workflows/ci.yml` shows a minimal Python client for each). They are omitted from the list above for that reason.

> **Security boundary:** TCP/WebSocket remain development smokes, not hardened remote control
> planes. Their binaries refuse non-loopback bind addresses and start in read-only safe mode, but
> they do not prevent a loopback listener from being exposed through forwarding, proxying,
> tunnelling, or another local process. They provide no authentication, authorization, TLS,
> payload redaction, remote-deployment assessment, or authenticated actor identity.

**Safe mode and wire subset.** The Agent Bridge read-only safe mode allows only `sim.status` and
confined `log.replay`. Every mutating method — `sim.step`, `sim.reset`, `scene.set_object`,
`intervention.apply`, `log.start`, `log.stop`, and file-writing `export.rerun` — is recorded as a
blocked bridge response. TCP/WebSocket require explicit `--allow-mutations` to leave safe mode;
stdio remains a directly invoked local process whose existing `--safe-mode` flag selects the
policy. The wire protocol is a single-request JSON-RPC 2.0 subset: batches are unsupported, an
omitted `id` is a silent notification and remains distinct from an explicit `null` id.
Parameters may be omitted or supplied as a named JSON object; positional arrays and undeclared
top-level method keys are rejected. Individual method contracts still enforce required values:
for example, `sim.step` requires a numeric `dt` and never silently substitutes one. Profile-level
invalid parameters use `-32602`; handler/domain failures after that validation use the
implementation-defined `-32000` code.

TCP and stdio cap each JSONL request line at 1 MiB. WebSocket caps the HTTP upgrade at 16 KiB and
each incoming client frame at 1 MiB; network socket reads and writes have a 30-second timeout per
operation. These
are not total request, session-duration, request-count, or aggregate-traffic limits, so traffic
that keeps making progress (including a trickle client) can persist indefinitely. A WebSocket
upgrade specifically requires `GET /bridge HTTP/1.1`, exactly one each of a nonempty `Host`,
`Upgrade: websocket`, a tokenized `Connection` containing `upgrade`,
`Sec-WebSocket-Version: 13`, and a base64 key decoding to exactly 16 bytes; any `Origin` header is
rejected. This is the implemented check set, not a claim that every malformed HTTP/WebSocket
request is recognized. After upgrade, client application messages must be unfragmented, masked
UTF-8 text frames; ping, pong, and close control frames are supported, while binary frames,
fragmentation, and WebSocket extensions/RSV use are rejected.

File RPCs use non-adversarial canonical-path confinement beneath the canonical directory holding
the session run log. They reject parent traversal, observed symlink components, non-regular or
out-of-root inputs, missing output parents, and existing outputs; transport run logs and Rerun
outputs use no-replace creation. This is not a security-grade filesystem sandbox against
hardlinks, alternative aliases, or concurrent local filesystem mutation. `export.rerun` parses
and manifests the same exact bounded byte snapshot read from the source, encodes the finalized RRD
bytes, hashes those bytes, stages and syncs the file, and installs it without clobbering. It does
not fsync the parent directory or claim power-loss durability. The three executable transports use
a file-backed writer whose flush calls `File::sync_all`: the initial prefix, each session flush
before a wire response, and the terminal seal therefore sync run-log file contents/metadata. A
generic `SimBridgeSession<W>` has only its supplied sink's flush semantics. Neither path fsyncs the
run-log parent directory or makes the run log and exported artifact one atomic transaction.

Once provenance storage is writable, ordinary accepted-client protocol/transport failures are
sealed with a failed `run_ended`. A crash or provenance-storage/write failure can instead leave an
incomplete or unreadable run log, an apparently complete terminal record whose status/durability
is indeterminate, or an installed RRD without its final `artifact_logged` record; there is no
valid-log or orphan-free guarantee for those cases.

Without `just`: `cargo run -p pid-sim --bin pid-sim-demo -- outputs/demo_runlog.jsonl`, then validate/replay it with `pid-runlog-replay`. For sidecar provenance, use `--write-sidecars` followed by `--verify-sidecars`.

## Engineering Plan (To "Finish" the Project)

The research milestones and stop rules are `grandplan.md` §12 (**M0–M7**: freeze contracts →
version estimator gates → core + ecosystem conformance benchmark → intervention pilot → locked H1 →
locked H2 → H3/H4 → transport replication). The infrastructure that supports them is specified in
§8 (**infrastructure as a scientific contribution**, whose acceptance claim is EC1).

The concrete build order for the capture/intervention/replay substrate:

Exp0 estimator gate → canonical `pid-runlog` event schema → deterministic replay → Agent Bridge
control plane → minimal sim + `Flow_gt` → Rerun-based viewer → embedding-capture harness on a real
VLA (the S2/EC1 adapter, `experiments/safe_adapter`) → optional live transport + robot sim →
optional predictor-driven `Flow_pred` → optional Tauri+SparkJS UI.

The GauSS‑MI document now separates an **optional, pre-implementation E1 reconstruction-quality
covariate/active-view study** from an **E0 quarantined weighted-KSG/PID sketch** that has no derived
estimand and must not be implemented as written (`GAUSS_MI_INTEGRATION.md`; `grandplan.md` §8.9).
Neither is a milestone.

If you use an external simulator backend (Isaac/MuJoCo/etc.), treat it as an adapter that still emits the canonical run log, logs backend/solver config via `config_logged`, and is controlled via the Agent Bridge surface.

### Docset-wide final solution

The decision record lives in `grandplan.md` §16:

```text
run log      = source of truth
Agent Bridge = only control plane
Rerun        = read-only diagnostic/time-machine viewer
Tauri/SparkJS = optional control/editor/custom-rendering shell
```

Build path: (1) keep Exp0/geometry gates strict; (2) define the canonical `pid-runlog` event schema; (3) implement deterministic replay; (4) route all GUI/script/LLM actions through the Agent Bridge; (5) build the minimal object sim and simulator-derived `Flow_gt`; (6) convert run logs into Rerun recordings/blueprints; (7) connect the offline embedding harness to one small real VLA/task capture with labels, attribution probes, and non-PID baselines; (8) gate optional live transport and external `Flow_pred` services behind the same run-log schema; (9) add Tauri/SparkJS only after the Rerun workflow works; (10) add license/provenance automation for dependencies, models, datasets, generated assets, and sidecars.

## Citation

```bibtex
@software{prisoma,
  title  = {Prisoma: Intervention-Grounded Diagnostics for Sequential Embodied Policies},
  author = {Mahmoudian, Sepehr},
  year   = {2026},
  url    = {https://github.com/sepahead/prisoma}
}
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
