<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo-dark.svg">
    <img alt="Prisoma logo" src="assets/logo-light.svg" width="200">
  </picture>
</p>

# Prisoma

Prisoma is a low-overhead research toolkit for auditable experiments on embodied policies.
It provides capture, intervention, replay, protocol, and evidence-groundwork components.
Partial information decomposition (PID) is one conditional diagnostic, not the product.

[![CI](https://github.com/sepahead/prisoma/actions/workflows/ci.yml/badge.svg)](https://github.com/sepahead/prisoma/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](#license)

The primary project asks when an action-conditioned world model improves a supported decision,
not only a forecast. It also asks where prediction, rendering, policy, controller, and selection
errors enter one matched closed loop. PID is a conditional diagnostic study. Negative gate
results are valid results.

> Current scientific status: W1–W3 and the preserved EC1/H1–H4 family are unfrozen claim
> templates, not results. Confirmatory claims remain blocked. The high-dimensional
> MI/coherence path is **NO-GO**. The continuous shared-exclusions application gate is
> **BLOCKED / NOT APPLICATION-VALIDATED**.

## Design in one page

Prisoma is an experiment-semantics layer. It does not replace a policy, simulator, robot,
viewer, or estimator library.

Three invariants define the architecture:

1. The canonical run log is the source of truth for accepted recorded events. It does not prove
   that an upstream source emitted an event the capture boundary never observed.
2. The Agent Bridge is the only control plane.
3. Scientific interpretation requires separate population, measure, estimator, and application gates.

```text
client or policy
      |
      v
Agent Bridge ----append before dispatch----> canonical run log
      |                                           |
      v                                           +--> replay / validation
physics or environment                            +--> Rerun adapter
      |
      +---------------- observations ------------>+

exact restored fork --> candidate actions --> forecast commit --> selection --> bridge execution
                                                |                                      |
                                                +--> saved fork --> reference labels <--+ execution receipt

content-bound capture --> (V,L,D,A) adapter --> bounded offline diagnostics
```

`D` is a declared source axis. It often represents dynamics or hidden state, but Prisoma
does not assign one universal meaning to it.

Prisoma does not treat `VLA` and `WAM` as rival scientific classes. It classifies the
deployed directed graph. Predictive training, intended-future conditioning, coupled joint
generation, action-conditioned prediction, and candidate planning are different designs. A joint
density does not create an operational action-conditioned query. The reviewed evidence does not
show that VLAs are dead. Many current systems retain a VLA action interface while adding a world
objective, a future-conditioning path, or a planner. See the
[dated frontier review](docs/audits/2026-08-12-first-principles/WORLD_ACTION_MODEL_FRONTIER.md).
Prisoma also rejects target injection. A state conditioned on a candidate action cannot be a PID
source when the target is that exact proposal. A downstream command, later declared
reference-state outcome, or separately measured physical outcome remains eligible only when the
matched baseline receives the same proposal. Command prediction and simulator-state prediction do
not establish physical forecast validity. A future H3 adapter must freeze a target-specific prediction
landmark before target realization or availability. It must bind every source ancestor to that
landmark. The current shared artifact schema does not yet enforce this receipt.

The local crates stay small in role:

| Component | Responsibility | Boundary |
|---|---|---|
| `pid-bridge` | Request contracts and run-log integration | No transport or physics ownership |
| `pid-sim` | Deterministic fixtures, bridge transports, protocol references, offline harness | Protocols, analysis, WebSocket, Rapier, and Rerun export are opt-in; not a general simulator product |
| `pid-rerun` | Bounded run-log-validating Rerun conversion | No control authority |
| `experiments/safe_adapter` | Reference `(V,L,D,A)` adapter implementation | Preserved diagnostic path; synthetic proof only until real capture |
| `experiments/attribution` | Bounded exploratory attribution probe | No causal-faithfulness claim |
| `crates/ncp-observer` | Optional read-only wire-0.8 observer | Excluded from the main workspace |

The estimator source of truth is the pinned
[`pid-rs`](https://github.com/sepahead/pid-rs) submodule at `796c11e`.
Prisoma 0.9.0 reviews that post-tag source and makes no 1.x compatibility promise.
It also makes no published-wheel promise.

## Scientific gate status

The computation status of a number never authorizes its interpretation.

| Gate | Current status | Meaning |
|---|---|---|
| Population | Open and unfrozen | A producer declaration is not a validated population law |
| Measure | Not adjudicated for the default atom path | The required measure-level validation is incomplete |
| Atom estimator | Blocked | Current atom recovery does not clear the estimator gate |
| Continuous application | Blocked | No application-valid support envelope exists for real VLA embeddings |
| High-dimensional MI/coherence | NO-GO | Current nuisance tests fail the reviewed analysis route |

An abstained estimate has no zero, NaN, or metric placeholder. Exact ties can reject a
sample. They cannot redefine its population law. Prisoma never routes a failed continuous
term to a categorical route automatically because that changes the object and estimand.

See [findings.md](findings.md) for current estimator evidence. See
[grandplan.md](grandplan.md) for the canonical research specification.

## What is implemented

- Canonical schema-2 run-log validation, replay, manifests, and sidecars.
- A mutation-disabled-by-default local Agent Bridge over in-process, stdio, TCP, and WebSocket transports.
- A finite, paired, read-only Engram-host TCP profile with secret-possession proofs.
- A bounded Host API 2 observer for exact Engram closed-loop receipt lineage.
- A clean-source arm64 build receipt for that observer's release staging path.
- Deterministic object and Rapier-backed manipulation fixtures.
- A zero-model-download exact-fork world-model decision reference with a fixed candidate pool,
  pre-label forecast publication, bridge-only selected execution, post-receipt independent branch
  labels, and replay.
- A bounded offline `(V,L,D,A)` harness with static baselines and explicit PID modes.
- Typed resource admission for samples, decoded metadata, distance, dense-solver, and categorical
  work.
- H1 common-preflight and Protocol-A synthetic software references.
- An H2 fixed-horizon synthetic IPCW risk-estimator arithmetic reference.
- A content-bound SAFE adapter and a bounded attribution reference probe.
- A narrow run-log-validating Rerun conversion adapter. It is a derived view, not W1–W3 evidence.
- Machine-readable claim-template, capability, governance, and release-truth ledgers.

These are software proofs for stated fixtures. They are not EC1 validation, a frozen
preregistration, a confirmatory result, a safety result, or deployment evidence.

## What is not implemented

- A real confirmatory capture or registered holdout.
- A validated high-dimensional continuous PID application path.
- A live Paper2Brain-to-Prisoma producer or NCP wire translator.
- A production remote-security boundary.
- The complete Rerun diagnostic application.
- A Tauri/SparkJS product shell.
- A Gaussian-splatting runtime or external learned-world-model adapter.
- A qualified JEPA-WM, VLA-JEPA, Flex-\(\pi\), or other reviewed model adapter.
- An MPS-validated learned-world-model planning pipeline.

The native world-model reference is the first software rung. Linked mesh-versus-3DGS fidelity
tomography is a planned study. Neither is empirical evidence yet.

## Install

Required tools:

- Rust 1.93 for the local workspace.
- Python 3.11 or newer.
- [`uv`](https://docs.astral.sh/uv/) 0.11.28.
- Git with submodule support.
- `just` 1.56.0 for the documented recipes, if desired.

The pinned `pid-rs` workspace declares Rust 1.89. The local workspace declares Rust 1.93.

```bash
git clone --recurse-submodules https://github.com/sepahead/prisoma.git
cd prisoma
git submodule update --init
uv sync --locked
cargo build --locked -p pid-bridge -p pid-sim
```

The default Python environment contains NumPy, developer tools, and Markdown support.
Install an optional group only for its operator task:

```bash
uv sync --locked --group analysis
uv sync --locked --group ui
```

Build the full viewer and export surface only when needed:

```bash
cargo build --locked --workspace --all-features
```

Build the Python estimator bindings from the submodule:

```bash
uv run --no-sync maturin develop --locked \
  --manifest-path pid-rs/crates/pid-python/Cargo.toml
```

## Run the software proofs

Start with the full local gate:

```bash
uv sync --locked --group ui
just check
```

The UI group is required only because the full suite tests the optional PNG utility. The default
environment remains sufficient for ordinary capture, protocol, and attribution work.

Run focused evidence paths when their surface changes:

```bash
just exp0-bin
just firebreak
just h1-preflight
just h1-protocol-a
just h2-reference
just world-model-reference
just toy-harness
just rapier-test
just safe-adapter
just attribution-probe
just runlog-rerun-proof
just formal
```

The firebreak checks that static factual-outcome baselines request no PID atoms or NCP data.
It does not implement the H1 response or prospective H2 endpoint.

Run the offline harness directly:

```bash
cargo run --locked -p pid-sim --features analysis --bin pid-offline-harness -- \
  --input crates/pid-sim/fixtures/offline_vlda_fixture.json \
  --pid-mode none \
  --summary-json outputs/offline_summary.json \
  --runlog outputs/offline_runlog.jsonl

cargo run --locked \
  --manifest-path pid-rs/crates/pid-runlog/Cargo.toml \
  --bin pid-runlog-replay -- \
  --validate outputs/offline_runlog.jsonl
```

The default `--pid-mode none` is the estimator-request firebreak. It emits no MI or PID request.
The opt-in `analysis` build still links `pid-core` because geometry and logistic code reuse that
library. `continuous`, `categorical-sx`, and `categorical-sx-pls` require explicit opt-in. The
categorical routes fit equal-width bins and estimate the averaged two-source MGW
shared-exclusions functional on the resulting empirical categorical laws. They are not `I_min`,
BROJA, the continuous Ehrlich functional, or an infomorphic objective. Every report binds
fitted-transform receipts, transform hashes, estimator identity, and units. `categorical-sx-pls`
learns a target-supervised projection from the same rows it analyzes. Every such estimate is an
estimator-blocked `produced_with_warning` selection-inflation diagnostic. It does not score
held-out categorical rows and cannot rescue an inferential PID claim. These modes remain
diagnostic and do not
clear the four scientific gates. Reports also retain the estimator's empirical-PMF occupancy,
singleton, low-count, coverage-indicator, and unseen-state caveat fields. These diagnostics do not
prove population support. Continuous requests also require a complete-tuple joint-law and
finite-information declaration. Per-axis continuity alone is insufficient. New summary
configurations bind report contract `prisoma.offline_vlda.report/5`. Publication rejects an
unversioned or unknown report contract.

Use [`PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md`](PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md)
before selecting or interpreting any PID-related route. It separates functionals, declared-law
evaluators, sample estimators, transforms, certifiers, validation artifacts, objective
compositions, and application interpretations. It also separates cumulative lattice values,
Möbius-inverted atoms, antichain coordinates, pointwise and averaged quantities, and net,
informative, and misinformative components. It preserves higher-source and novel PID work as typed
research objects without letting one PID rescue, pool with, or masquerade as another.
It also defines the `PID-M0` through `PID-M8` process. That process preserves intake, checks the
mathematics and applicability, freezes the route, records execution and review, and builds a
paired Markdown/PDF publication packet. The PDF is a deterministic derived view, not a second
scientific authority.
Publication also requires the private in-process seal created by the analysis call. Treat a saved
summary as read-only evidence. Rerun the analysis to publish a new summary or run log.

## Resource and overhead model

Prisoma defaults to bounded work. The offline harness admits at most 64 MiB of input and 1,024
samples. It caps pairwise work at 50,000,000 evaluations and coordinate work at 100,000,000
units. Dense-solver work defaults to 100,000,000 projected operations. It also caps decoded
scalars, metadata, JSON depth, and output bytes. Fitted categorical PID work defaults to
500,000,000 projected operations. Stress fixtures use an explicit limits file.

The optional NCP crate is outside the default workspace. Protocol references, legacy sensitivity,
Rust analysis, WebSocket, Rapier, Rerun export, and optional Python groups are also outside the
minimum path. Enable `pid-sim/protocol-references` for the H1 and H2 reference CLIs. Enable
`pid-sim/analysis` for the toy or offline harness. Enable `pid-sim/rerun-export` only when a bridge
must serve `export.rerun`. The bridge uses a direct argument parser. File inputs use bounded
descriptor snapshots.

These limits are safety and availability controls. They are not performance claims. Measure
the exact workload before selecting hardware or raising a limit.

For one M4 Max, start with `just world-model-reference`. It is CPU-only, needs no model download,
and is small. A clean Rust build can still fetch pinned Cargo dependencies.
The first external target is the compact LeWorldModel PushT CEM planner at the exact revisions in
`grandplan.md`. Its reviewed configuration uses 30 rounds, 300 samples, 30 elites, horizon five,
and action blocks of length five. Its upstream evaluator hard-codes CUDA, so it is only an **MPS
candidate**. One exact-package synthetic probe ran the predictor, rollout, and full-budget CEM on
MPS. It did not run PushT or closed-loop replanning. Reproduce the exact CEM path before freezing a
reduced-budget arm. A one-seed independent TwoRoom reproduction found outcome-relevant pipeline
conventions outside configuration files and conflicting evaluation settings. It did not test
PushT or M4. Bind a paper/configuration/code concordance ledger and freeze each unresolved feasible
reading before outcomes. Fit action scaling on training rows only. Check raw-action support after inverse
transformation. Admit the port only after CPU/MPS parity, action sensitivity, adaptive-search
reconstruction, multi-replan execution, and measured tail-latency, memory, power, and deadline
receipts. JEPA-WM is the second planning benchmark.
SmolVLA is the direct-policy MPS baseline. VLA-JEPA is a predictive-training comparator. No
reviewed external model is a current Prisoma dependency or a qualified MPS runtime.

## NCP and Engram boundary

Prisoma pins the optional observer to NCP `v0.8.0` and wire 0.8. The crate is excluded from
the default workspace so NCP and Zenoh remain off the critical path.

Official NCP main was observed at
`1a04294c90c1b50eba06ae1c6afe9c951319250d` on 2026-08-13. That source is the release-blocked
`1.0.0-rc.1` candidate and uses wire 1.0. Its compact proto contract hash
`163acc57d8a62b66` remains unadopted. Ledger tasks
[`P01`, `P02`, and `P03`](https://github.com/sepahead/NCP/blob/1a04294c90c1b50eba06ae1c6afe9c951319250d/evidence/implementation/task-ledger.v1.json)
are OPEN, not dependency-ready, and NOT RUN. They include Prisoma observer-role qualification.
The refined low-overhead architecture and prepared-stream-monitor gap record are coordination
artifacts. B01 remains in progress.

The named `sepahead/engram` repository remains a README-only placeholder. The executable host
is `sepahead/Paper2Brain`. Its provider record describes a preserved in-progress Paper2Brain
migration that targets candidate wire 1.0. It is not an installed or qualified integration.
This is not an NCP producer, wire translator, artifact validator, or authority path.

The local Engram-host profile proves possession of one startup secret. It does not attest a
process, binary, build, producer, or remote host.

The separate Host API 2 child verifies closed-loop step and terminal receipts.
Its three operations use observation class and no compute grant.
The checked fixture records a host-declared projection of three drone subjects.
Engram source receipts do not authenticate those subject identifiers.
Its success vector declares NEST bundle-v2 while every child response reports verification as false.
The child has no NCP, PID, Agent Bridge command, artifact, network, or physical authority.
A historical v1 receipt records one Engram store launch and clean child reap.
Its source state was a working-tree candidate, so it grants no current runtime claim.
The Engram reviewed-development v2 launch gate remains `NOT RUN`.
The separate CREBAIN matrix records a completed read-only review of three real-NEST captures.
No state establishes production manager execution or publisher identity.

## Release and evidence status

The public 0.9.0 source is a prerelease research preview. It is not a stable release or a
scientific-results release.

`release/0.9.0/review` and `release/0.9.0/requirements` preserve immutable intake.
Candidate progress enters through `release/0.9.0/candidate_progress.json`. Generated candidate
artifacts remain content-bound to an exact source state.

Candidate schema 0.1 is deliberately non-promotable. It can record open, in-progress, blocked,
or failed work. A reviewed successor schema and authenticated exact-commit CI evidence are
required for terminal promotion.

```bash
uv run --no-sync python scripts/audit_candidate_release.py
```

A passing audit proves internal consistency only. It does not prove release readiness or close
any scientific claim.

## Documentation

Read these documents in this order:

| Document | Purpose |
|---|---|
| [grandplan.md](grandplan.md) | Canonical research and engineering specification |
| [PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md](PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md) | PID mathematics, route selection, research preservation, and publication contract |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Current system design, boundaries, and overhead model |
| [EXPERIMENTS.md](EXPERIMENTS.md) | Executable proof paths and blocked scientific protocols |
| [DIAGRAMS.md](DIAGRAMS.md) | Compact architecture and evidence-flow diagrams |
| [pidsplatspecs.md](pidsplatspecs.md) | Stable Prisoma interface contract; legacy filename |
| [findings.md](findings.md) | Current estimator evidence and verdicts |
| [LIMITATIONS.md](LIMITATIONS.md) | Claim, security, and deployment limits |
| [THESIS_EVIDENCE_INDEX.md](THESIS_EVIDENCE_INDEX.md) | Claim-to-evidence and blocker map |
| [WORLD_ACTION_MODEL_FRONTIER.md](docs/audits/2026-08-12-first-principles/WORLD_ACTION_MODEL_FRONTIER.md) | Dated WAM taxonomy, causal gate, and M4 decision |
| [RESEARCH_VLA_D_NCP.md](RESEARCH_VLA_D_NCP.md) | D-axis, VLA, and optional NCP literature synthesis |
| [docs/CAPABILITY_MATRIX.md](docs/CAPABILITY_MATRIX.md) | Generated content-bound capability inventory |
| [docs/audits/2026-08-12-first-principles/FIRST_PRINCIPLES_AUDIT.md](docs/audits/2026-08-12-first-principles/FIRST_PRINCIPLES_AUDIT.md) | Dated hypothesis, source, and repository audit |
| [docs/audits/2026-08-12-first-principles/PID_RS_HANDOFF.md](docs/audits/2026-08-12-first-principles/PID_RS_HANDOFF.md) | Consumer-owned `pid-rs` migration and assurance handoff |
| [docs/audits/2026-08-12-first-principles/PID_RS_EXTENSION_BRIEF.md](docs/audits/2026-08-12-first-principles/PID_RS_EXTENSION_BRIEF.md) | Ranked `pid-rs` extension request with scientific-object boundaries |
| [AGENTS.md](AGENTS.md) | Contributor ground truth and gate rules |

`GAUSS_MI_INTEGRATION.md` and `WORLD_WARP_INTEGRATION.md` are optional study specifications.
They are not active runtime architecture.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md) before editing.
Keep technical prose STE-aligned. Verify commands, pins, links, and scientific status.

Run these gates before a commit:

```bash
just check
just formal
just research-governance
cargo deny --locked check
```

Build the optional NCP crate separately when it changes:

```bash
cargo test --locked --manifest-path crates/ncp-observer/Cargo.toml --all-targets
```

Report security issues through the private process in [SECURITY.md](SECURITY.md).

## Citation

No archival paper citation is available yet. Cite the exact repository revision and describe
the gate status that applied to your run.

## License

Prisoma is available under either the MIT License or Apache License 2.0. See
[LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
