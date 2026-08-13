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

The project asks whether pre-treatment diagnostics predict intervention response and future
failure beyond strong baselines. It does not assume that PID will answer that question.
Negative gate results are valid results.

> Current scientific status: EC1 and H1–H4 are unfrozen claim templates, not completed
> hypotheses. Confirmatory claims remain blocked. The high-dimensional
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

content-bound capture --> (V,L,D,A) adapter --> bounded offline analysis
                                                --> baselines
                                                --> geometry diagnostics
                                                --> conditional PID screens
```

`D` is a declared source axis. It often represents dynamics or hidden state, but Prisoma
does not assign one universal meaning to it.

Prisoma also does not treat `VLA` and `WAM` as rival scientific classes. It classifies the
deployed directed graph. Predictive training, intended-future conditioning, coupled joint
generation, action-conditioned prediction, and candidate planning are different designs. A joint
density does not create an operational action-conditioned query. See the
[dated frontier review](docs/audits/2026-08-12-first-principles/WORLD_ACTION_MODEL_FRONTIER.md).

The local crates stay small in role:

| Component | Responsibility | Boundary |
|---|---|---|
| `pid-bridge` | Request contracts and run-log integration | No transport or physics ownership |
| `pid-sim` | Deterministic fixtures, bridge transports, protocol references, offline harness | Protocols, analysis, WebSocket, Rapier, and Rerun export are opt-in; not a general simulator product |
| `pid-rerun` | Bounded run-log-validating Rerun conversion | No control authority |
| `experiments/safe_adapter` | Reference `(V,L,D,A)` adapter implementation | Candidate real-data path; synthetic proof only until real capture |
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
term to discrete `I_min` automatically because that changes the measure and estimand.

See [findings.md](findings.md) for current estimator evidence. See
[grandplan.md](grandplan.md) for the canonical research specification.

## What is implemented

- Canonical schema-2 run-log validation, replay, manifests, and sidecars.
- A mutation-disabled-by-default local Agent Bridge over in-process, stdio, TCP, and WebSocket transports.
- A finite, paired, read-only Engram-host TCP profile with secret-possession proofs.
- Deterministic object and Rapier-backed manipulation fixtures.
- A bounded offline `(V,L,D,A)` harness with static baselines and explicit PID modes.
- Typed resource admission for samples, decoded metadata, distance work, and dense solvers.
- H1 common-preflight and Protocol-A synthetic software references.
- An H2 fixed-horizon synthetic IPCW risk-estimator arithmetic reference.
- A content-bound SAFE adapter and a bounded attribution reference probe.
- A run-log-validating Rerun conversion adapter.
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
- A Gaussian-splatting or world-model runtime.
- A qualified SLIM, LiLa-WAM, Flex-\(\pi\), or other WAM adapter.
- An MPS-validated predictive-policy pipeline.

Optional rendering and comparator studies remain separate proposals. They do not define the
core architecture and do not sit on the thesis critical path.

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
library. Continuous and quantized modes require explicit opt-in. They remain diagnostic and do not
clear the four scientific gates. New summary configurations bind report contract
`prisoma.offline_vlda.report/2`. Publication rejects an unversioned or unknown report contract.

## Resource and overhead model

Prisoma defaults to bounded work. The offline harness admits at most 64 MiB of input and 1,024
samples. It caps pairwise work at 50,000,000 evaluations and coordinate work at 100,000,000
units. Dense-solver work defaults to 100,000,000 projected operations. It also caps decoded
scalars, metadata, JSON depth, and output bytes. Stress fixtures use an explicit limits file.

The optional NCP crate is outside the default workspace. Protocol references, legacy sensitivity,
Rust analysis, WebSocket, Rapier, Rerun export, and optional Python groups are also outside the
minimum path. Enable `pid-sim/protocol-references` for the H1 and H2 reference CLIs. Enable
`pid-sim/analysis` for the toy or offline harness. Enable `pid-sim/rerun-export` only when a bridge
must serve `export.rerun`. The bridge uses a direct argument parser. File inputs use bounded
descriptor snapshots.

These limits are safety and availability controls. They are not performance claims. Measure
the exact workload before selecting hardware or raising a limit.

For one M4 Max, use the documented SmolVLA MPS path only as a baseline candidate. SLIM is the
first compact predictive-training candidate for the full VLDA contract. Efficient-WAM is a later
class-J code port. Its released attention helper rejects non-CUDA devices before its nominal
fallback. JEPA-WAM is another later MPS port candidate with released source and weights. LiLa-WAM
is a separate 0.5B no-language predictive ablation. Full video WAMs remain off the critical path.
No reviewed predictive model is currently a Prisoma dependency or a qualified MPS runtime. Any
future asynchronous chunk path must measure the full observation-to-command delay. Bind each
executed command to its source observation and chunk index.

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
