<p align="center">
  <img src="assets/prisoma-logo.svg" width="120" alt="Prisoma information prism">
</p>

<p align="center"><a href="assets/archive/logos/README.md">Logo design archive</a></p>

# Prisoma

**Experiments for action-conditioned world models and embodied decisions.**

Prisoma compares proposed actions before their outcomes are known.
It binds forecasts to the same pre-action state, records the selected execution, and checks later outcomes against matched baselines.
Partial information decomposition (PID) provides a separate, gated analysis of declared sources and targets.

The current release is a **0.9.0 research source prerelease**.
Runnable engineering components are available. M2 and the W1–W3 scientific studies remain open.
The [v13.0 grand plan](grandplan.md) owns the research design.

<picture>
  <source media="(max-width: 640px)" srcset="assets/experiment-workflow-mobile.svg">
  <img src="assets/experiment-workflow.svg" width="1200" alt="Prisoma experiment: freeze one state and candidate actions, commit forecasts, execute the selection through Agent Bridge, then label independent restored branches. LeWM inference and CREBAIN integration have separate evidence boundaries.">
</picture>

[Read the workflow and mathematics](docs/EXPERIMENT_WORKFLOW.md) ·
[Wide SVG](assets/experiment-workflow.svg) ·
[Mobile SVG](assets/experiment-workflow-mobile.svg) ·
[Direct vector for browser zoom](https://raw.githubusercontent.com/sepahead/prisoma/main/assets/experiment-workflow.svg)

## What this adds

Logging records events. An experiment also needs an ordering rule and a fair comparison.
Prisoma's exact-fork reference enforces this sequence:

1. Save one immutable simulator state.
2. Freeze an ordered pool containing at least two distinct, supported actions.
3. Commit every forecast and score before reference labels become available.
4. Execute only the selected action through the Agent Bridge.
5. Commit its execution receipt.
6. Obtain each candidate outcome from an independent branch restored from the saved state.
7. Verify the commitments, selection, execution, outcomes, and replay.

This makes changes to candidates, predictions, and execution detectable in the tested reference.
The reference uses a small affine model and the same deterministic law for training and simulation.
It proves software ordering, not learned forecast accuracy or planning benefit.

[Rerun](https://rerun.io/docs/overview/what-is-rerun) already provides substantial recording, storage, query, and visualization capabilities.
Prisoma builds experimental ordering and scientific interpretation above that layer.
Its current Rerun adapter produces a derived view and does not map every world-model event.
See the [adapter boundaries](ARCHITECTURE.md#34-viewer-layer-pid-rerun).

## What runs today

| Component | What you can do | Evidence boundary |
| --- | --- | --- |
| Exact-fork decision reference | Compare a fixed action pool, execute the selection, label restored branches, and verify replay | Deterministic affine software control, not W1 or W2 |
| [LeWM engineering adapter](docs/lewm/README.md) | Run verified pretrained weights on real PushT images and inspect complete CPU/MPS search traces | One frozen input, no raw action execution or outcome labels |
| [Offline analysis](EXPERIMENTS.md) | Evaluate strict artifacts with simple baselines and optional named PID routes | Support, split, measure, estimator, and application limits remain explicit |
| [NCP capture](integrations/ncp-transcript/README.md) | Preserve original exchanges for independently selected peers, including large frame sequences | Separate native NEST and CREBAIN source runs; complete embodied experiment remains open |
| [Agent Bridge](ARCHITECTURE.md#32-contract-layer-pid-bridge) | Dispatch typed local operations and record accepted requests and responses | Local profiles, not a qualified remote-security system |
| [Rerun export](ARCHITECTURE.md#34-viewer-layer-pid-rerun) | Inspect schema-checked run-log projections | Derived inspection, not scientific validation |

The [capability matrix](docs/CAPABILITY_MATRIX.md) records the reviewed evidence for each cataloged surface.
Successful computation does not establish model quality, causal validity, or deployment safety.

## Start with the decision reference

Use Rust 1.93.0 or newer and the locked dependencies.
The pinned estimator workspace has its separate Rust 1.89.0 minimum.
Python utilities require Python 3.11 or newer and exact `uv==0.11.28`.
Recipes use `just==1.56.0`.

For a new checkout:

```bash
git clone --recurse-submodules https://github.com/sepahead/prisoma.git
cd prisoma
uv sync --locked
cargo build --locked -p pid-bridge -p pid-sim
just world-model-reference
```

The reference downloads no model weights. An uncached build can fetch pinned Cargo dependencies.
The recipe also validates the canonical run log and creates a temporary headless Rerun export.
It removes its temporary artifacts after verification.
Read [EXPERIMENTS.md](EXPERIMENTS.md) for commands that retain outputs for inspection.

The default core excludes NCP, Zenoh, the learned model runtime, and optional analysis features.
Enable only the surface you need. The full development gate deliberately exercises additional features.

## Run LeWM on the M4 Max

The maintained adapter executed real pretrained action-conditioned inference and CEM search on CPU and MPS.
It used one frozen PushT observation/goal input and two separately identified source constructions.
Their observed concordance is limited to that input and the recorded numerical controls.

The frozen search uses 30 rounds, 300 samples per round, 30 elites, horizon five, and five-action blocks.
Every round records proposals, predictions, scores, elite membership, and distribution updates.
The final mean receives its own forecast and score.

Follow the [offline staging and execution guide](docs/lewm/README.md).
The ordinary environment installs no Torch and downloads no weights.
The optional runtime verifies exact source, wheel, checkpoint, and input identities.

Candidate values are **standardized model inputs**, not raw PushT commands.
The current adapter executes no raw action and obtains no future branch label.
The [Lance snapshot reader](docs/lewm/LANCE_ACTION_SNAPSHOT.md) fits all 2.34 million observed action rows through the existing scaler.
Checkpoint-matched normalization, supported raw actions, multiple replans, and resource qualification remain open.
The adapter remains an **MPS candidate**, not completed M2 or W1–W3 evidence.

Read the [mathematics](docs/lewm/MATHEMATICS.md),
[vector PDF](output/pdf/LeWM_Mathematics_and_Evidence.pdf), and
[PID handoff](docs/lewm/PID_HANDOFF.md) for the exact meaning of each output.

## CREBAIN and the ecosystem

CREBAIN is the selected environment integration for the embodied rollout path.
It owns its world, dynamics, sensor production, and accepted checkpoint construction.
Prisoma owns candidate identity, forecast commitments, experiment ordering, comparison rules, and outcome lineage.
The complete Prisoma-to-CREBAIN embodied experiment remains under qualification.
A working environment component alone does not establish that integration.

The independent reference workflows remain runnable without CREBAIN.
This preserves the [dependency firebreak](grandplan.md#893-dependency-firebreak).
Each future environment path must bind units, frames, clocks, action application, missingness, and checkpoint semantics.
Simulator reference outcomes must remain distinct from measured physical outcomes.

NCP capture is a separate optional component.
Its modular transcript preserves original NCP JSON payload bytes and checks exchange order and terminal acknowledgements.
Its standalone manifest pins the `ncp-local` SDK to immutable public Git source, with no sibling checkout requirement.
CREBAIN's optional host hook captures sensor reads before releasing their source buffers.
One [native body run](integrations/ncp-transcript/README.md#crebain-sensor-capture) verified all 44 RGB, thermal, and pressure payloads.
CREBAIN remains usable without Prisoma.
The capture package also works without a body peer.
Application completion and scientific validity remain separate checks.
Installed ecosystem qualification remains a separate gate.
It does not turn NCP into a broker or grant Prisoma command authority.

<details>
<summary>Legacy NCP and Engram compatibility</summary>

The legacy observer remains pinned to `v0.8.0` and wire 0.8.
The separate August 13, 2026 review observed NCP at `1a04294c90c1b50eba06ae1c6afe9c951319250d`.
That source is the unreleased, release-blocked `1.0.0-rc.1` candidate, with compact proto contract hash `163acc57d8a62b66`.
Its P01, P02, and P03 tasks remain OPEN, not dependency-ready, and **NOT RUN** in the retained review.
P03 includes Prisoma observer-role qualification.
The [dated task ledger](https://github.com/sepahead/NCP/blob/1a04294c90c1b50eba06ae1c6afe9c951319250d/evidence/implementation/task-ledger.v1.json) preserves that boundary.
These identities do not describe the separate native local capture package.

The `sepahead/engram` repository remains a README-only placeholder.
The executable Engram Neural Labs host is `sepahead/Paper2Brain`.
The August review retains a preserved in-progress Paper2Brain migration that targets candidate wire 1.0.
That recorded migration is not an installed or qualified integration.

The separate Host API observer provides bounded, read-only presentation of source receipts.
This is not an NCP producer, wire translator, artifact validator, or authority path.
Read its [owning contract](integrations/engram/README.md) before using that historical compatibility surface.

</details>

## PID remains substantive and gated

Prisoma consumes `pid-core`, `pid-runlog`, and `pid-python` through the pinned `pid-rs` submodule.
The current pin is 0.9.0 post-tag review source `796c11e70f009634b853dc4ada6f565563d82f51` (`796c11e`).
This surface makes **no 1.x compatibility promise** and no published-wheel promise.

Prisoma owns variable definitions, sampling, transform lineage, eligibility, and comparisons.
`pid-rs` owns the estimator and run-log implementation.
The same estimator used by two projects is shared evidence, not independent replication.

The first planned embodied PID case uses RGB and acoustic observations, with thermal optional.
Start with two through four declared source variables; a larger count requires a supported estimator and separate statistical bounds.
These counts constrain the experiment, not the NCP protocol.
Neither capture nor estimation requires every ecosystem project or sensor modality.

A sensor instance is distinct from a modality and an experimental variable.
Two cameras or microphones retain separate identities, even when they share a modality.
Prisoma must record whether their features form separate variables or one joint source.
Each case must bind the target, prediction landmark, timing, feature encoding, sampling assumptions, and statistical bounds before evaluation.
Acoustic pressure samples require an explicit time window and transform; arrival order does not establish synchronization.
The planned comparison tests whether the full MGW decomposition adds information beyond MI, CMI, and task loss.
Keep its exact functional, source count, and statistical bounds fixed.
This is a design requirement, not a completed PID result or a change to the pinned PID-rs implementation.

PID interpretation requires four separate gates: population, measure, estimator, and application.
High-dimensional MI/coherence remains **NO-GO**.
Continuous shared-exclusions analysis of the intended VLA embeddings remains **BLOCKED / NOT APPLICATION-VALIDATED**.
A failed or unsupported estimate must abstain without a numeric placeholder.

The default offline `--pid-mode none` makes no MI or PID request.
Its optional analysis build still links shared `pid-core` geometry and logistic code.
Named categorical and continuous routes are different estimands. They never substitute for each other.

A candidate-conditioned latent cannot be a PID source whose target is that same candidate action.
A downstream target requires the same proposal in the matched baseline and a valid prediction landmark with source ancestry.
The current LeWM helper performs structural checks only. It does not attest H3 ancestry or produce a PID dataset.

Use the [method-selection and publication contract](PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md) before interpreting any PID output.
It defines the mathematical identity, route, gates, and paired Markdown/PDF evidence packet.

## Develop and review

Read [AGENTS.md](AGENTS.md) and [CONTRIBUTING.md](CONTRIBUTING.md) before changing the repository.
Run the complete local development gate before publication:

```bash
uv sync --locked --group ui
just check
```

The `ui` group supports the optional PNG utility tests.
Use `just docs-audit` for a focused documentation check.
The commit-bound candidate audit follows its [release procedure](release/0.9.0/RELEASE_NOTES.md).
Source publication does not close a scientific gate.

The release review retains **240 OPEN tasks and 4,800 OPEN lens dispositions**.
Candidate schema 0.1 remains non-promotable.
The frozen intake, scientific registries, holdout boundaries, and negative evidence remain unchanged.

| Read | Purpose |
| --- | --- |
| [Experiment workflow](docs/EXPERIMENT_WORKFLOW.md) | Ownership, ordering, a small numerical example, and current limits |
| [Grand plan](grandplan.md) | Canonical v13.0 research design and stop rules |
| [Experiments](EXPERIMENTS.md) | Runnable proofs, baselines, and evidence commands |
| [Architecture](ARCHITECTURE.md) | Component, authority, resource, and trust contracts |
| [System diagrams](DIAGRAMS.md) | Detailed current and proposed paths |
| [Findings](findings.md) | Retained measurements and negative results |
| [Limitations](LIMITATIONS.md) | Unsupported claims and operating boundaries |
| [Security](SECURITY.md) | Reporting and current threat boundaries |

## License and citation

Prisoma source uses [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
External code, weights, data, and assets retain their separate terms.
See [third-party notices](THIRD_PARTY_NOTICES.md) and the LeWM guide's unresolved historical notice boundary.
Use [CITATION.cff](CITATION.cff) when citing the software.
