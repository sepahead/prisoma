# Prisoma agent contract

Prisoma develops auditable experiments for action-conditioned world models and embodied decisions.
This file is the canonical operating contract for maintainers and coding agents.
Every completion claim must identify the tested scope and remaining limits.

## Read before changing

Read [README.md](README.md) first.
Then read the documents that own the affected surface.
The canonical research and engineering specification is [grandplan.md](grandplan.md), docset v13.0.

| Change area | Required owner |
| --- | --- |
| Research question, target, model, baseline, or scientific claim | [Grand plan](grandplan.md) and [experimental runbook](EXPERIMENTS.md) |
| Component ownership, bridge, transport, replay, or resource bounds | [Architecture](ARCHITECTURE.md) and [adapter specification](pidsplatspecs.md) |
| World-model experiment ordering | [Workflow guide](docs/EXPERIMENT_WORKFLOW.md) and grandplan sections 9.2 and 12 |
| LeWM loading, candidates, CEM, or MPS | [LeWM owner](docs/lewm/README.md), [design](experiments/lewm/DESIGN.md), and [mathematics](docs/lewm/MATHEMATICS.md) |
| PID method, route, support, or interpretation | [PID method contract](PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md), grandplan section 7, and [findings](findings.md) |
| PID data from world-model outputs | [PID handoff](docs/lewm/PID_HANDOFF.md) and the target-specific ancestry rules |
| Native local NCP capture | [Capture contract](crates/ncp-local-capture/README.md) |
| Legacy NCP wire observer | [Observer contract](crates/ncp-observer/README.md) and [NCP development notes](NCP_DEV_PROMPT.md) |
| Engram Host API receipt observation | [Integration contract](integrations/engram/README.md) and [managed observer](integrations/engram/managed-observer/README.md) |
| Rerun or other visualizations | [UI specification](uidesigner/UI.md), [diagrams](DIAGRAMS.md), and grandplan section 16 |
| Release, dependencies, or security | [Contributing](CONTRIBUTING.md), [security](SECURITY.md), and [release notes](release/0.9.0/RELEASE_NOTES.md) |
| Capability or scientific-status projection | Owning catalog in `protocols/`, its generator, and its audit |

These documents remain requirements. This entrypoint does not replace their detailed contracts.

## Working method

1. Inspect the owning schema, implementation, tests, and evidence before editing.
2. Inventory staged changes, unstaged changes, branches, and worktrees before recovery work.
3. Preserve unrelated work and its staged state.
4. Compare five to ten credible approaches before a material design decision.
5. State assumptions, benefits, failure modes, and decisive controls for each approach.
6. Use independent council review for separable, consequential decisions.
7. Select the strongest compatible approach and record unresolved objections.
8. Implement generic behavior from declared schemas and capabilities.
9. Add a negative control for each new accept path.
10. Add a positive control for each new rejection path.
11. Run the applicable complete gate before publishing a small milestone.
12. Report exact commands, outcomes, limitations, and retained failures.

Review mathematics, experimental validity, authority, provenance, and operator understanding separately.
A majority vote cannot override a failed scientific, security, or provenance requirement.
Never branch on a paper title, author, filename, fixture identity, or expected result.

Freeze cases, seeds, exclusions, thresholds, source identities, and budgets before inspecting outcomes.
Keep random holdouts separate from selected challenge cases and synthetic controls.
Retain failed runs, abstentions, null results, and protocol deviations.
Do not tune a threshold or replace a case to improve an observed result.

## Protect ownership

`pid-rs` owns `pid-core`, `pid-python`, and `pid-runlog`.
Prisoma consumes their public APIs through the pinned submodule.
The active pin is 0.9.0 post-tag review source `796c11e70f009634b853dc4ada6f565563d82f51` (`796c11e`).
It makes **no 1.x compatibility promise** and no published-wheel promise.

Do not duplicate estimator implementations in Prisoma.
During ecosystem finalization, preserve all PID sources, pins, index state, branches, and worktrees.
An upstream request or public provider result does not authorize a consumer pin change.
A future adoption requires explicit scope and exact consumer compatibility evidence.

CREBAIN owns its world, dynamics, sensor production, and accepted checkpoint construction.
Prisoma owns experimental ordering and the comparison population.
Keep core reference workflows runnable independently of that selected embodied integration.
Do not alter active Engram KG, ingestion, model-provider, or shared runtime work.

## Experiment and authority invariants

The canonical run log governs accepted recorded events.
It cannot prove an upstream event that no capture boundary observed.
Schema-2 finalized logs require one response for every bridge request.
Sidecars, viewers, and summaries cannot replace canonical evidence.

The Agent Bridge is the only canonical experiment control plane.
It records accepted requests before dispatch and records effects and responses afterward.
Rerun, diagnostics, estimators, and observers have no mutation authority.
Logging an action does not make that action safe.

For an exact-fork experiment, preserve this order:

1. Freeze one complete pre-action state and the supported candidate pool.
2. Commit forecasts and scores before opening any reference label.
3. Select with the frozen rule.
4. Execute only the selected action through the Agent Bridge.
5. Commit its execution receipt.
6. Label each candidate on an independent restored branch.
7. Verify selection, execution, branch identity, and replay.

Adaptive search also binds every proposal round, forecast, score, elite set, update, and stopping rule.
A returned optimizer mean needs its own forecast and commitment.
The schema-2 reference uses named `label_observed` compatibility envelopes for commitments and execution receipts.
Those records are not outcome labels. Preserve historical replay and the pinned schema.

Declare policy proposal, controller output, executed command, and observed outcome separately.
Record units, frames, clocks, holds, saturation, overrides, and missingness.
A simulator reference outcome is not a measured physical outcome.
Checkpoint completeness requires every state component that can affect the declared future.
A displayed pose or rendered frame is not a complete checkpoint.

## World-model boundaries

Classify deployed computation before using a model label.
Separate predictive training, intended-future conditioning, joint generation, action-conditioned prediction, and scored candidate selection.
A joint density does not establish an executable clamped-action query.
An action-conditioned forecast is observational until its causal gate passes.

The affine reference proves software semantics only.
The maintained LeWM adapter supplies real pretrained one-input CPU/MPS engineering execution and complete search records.
It executes standardized candidate inputs, not raw actions, and collects no branch outcome labels.
One-input source concordance does not establish general implementation equivalence.
The upstream synthetic probe remains historical evidence with its original scope.

Keep LeWM source, weights, dependencies, preprocessing, and both source arms exact.
Preserve the first 30-round, 300-sample, 30-elite, horizon-five, five-action-block search.
A reduced search, alternate normalizer, or action projection is a separately frozen arm.
Use verified staged assets and the isolated optional runtime.
Do not load mutable remote code, enable hidden CPU fallback, or install model dependencies into shared services.
Code, checkpoint, data, and transitive rights remain separate review objects.

M2 and W1–W3 remain open.
W1 tests supported forecast fidelity under its frozen proper score and baseline.
W2 tests complete policies under its episode endpoint, resource bounds, and intention-to-treat design.
W3 compares linked fidelity boundaries with exact state, action, camera, and renderer identities.
A prediction score, command acknowledgement, or attractive render cannot substitute for those studies.

## PID and diagnostic science

Read the complete PID method contract before changing any scientific route.
Keep functional, quantity, coordinate, evaluator, estimator, transform, certifier, fixture, objective, and interpretation as distinct objects.
Freeze their exact identities, probability law, source order, target, aggregation, component, and units.
Do not pool different functionals or route a failed term into another method.
Preserve novel research with its own typed identity, blockers, and negative evidence.

Population, measure, estimator, and application are four separate interpretation gates.
A declared-law route substitutes evaluator correctness for estimator validity only.
High-dimensional MI/coherence remains **NO-GO**.
Continuous shared exclusions on the intended VLA tensors remains **BLOCKED / NOT APPLICATION-VALIDATED**.
Low-dimensional oracle success does not open those application gates.

Declare axis support and each complete continuous tuple's regular joint law and finite information.
Marginal continuity alone is insufficient.
Treat declarations as claims, not observed proof.
An abstention has no numeric placeholder, zero atom, NaN atom, or metric event.
Keep `not_requested`, `produced`, `produced_with_warning`, and `abstained` separate from interpretation verdicts.

The default `--pid-mode none` requests no MI or PID.
Its optional analysis feature still links shared geometry and logistic implementation.
Categorical MGW, continuous Ehrlich, `I_min`, and BROJA remain different scientific objects.
The same-row supervised `categorical-sx-pls` screen is an estimator-blocked selection-inflation diagnostic.
It provides no held-out categorical score or H3 evidence.

A candidate-conditioned state cannot predict that same candidate target in a PID analysis.
For a downstream target, give every matched baseline the same proposal and permitted observations.
Bind a prediction landmark before target availability and the maximum time of every tensor ancestor.
Reject future observations and target-containing sources.
The LeWM structural helper does not implement H3 ancestry attestation or supply missing language data.

Keep all rows from one fork or episode in the same declared split.
Fit transforms, thresholds, and model selection inside their permitted training folds.
Do not treat adaptive CEM proposals or repeated frames as independent experimental units.
Do not replace a complete target denominator with only successful estimates.

Every H1 result must name H1-A or H1-B.
Its primary lower confidence bound must exceed the frozen positive useful margin.
No secondary endpoint rescues a failed primary requirement.
For H2, distinguish a complete-data proper score, an IPCW risk estimator, and a proper observed-data score.
Freeze the target, censoring, score, assumptions, uncertainty, and missingness together.
Keep inactive diagnostic slots null and preserve the fresh-sample rule for an H3-to-H4 switch.

## Evidence and release

Preserve the immutable v12.5 intake and current v13.0 scientific design.
The release review retains 240 OPEN tasks and 4,800 OPEN lens dispositions.
Candidate schema 0.1 is non-promotable.
A generated source inventory describes current bytes. It cannot close scientific or publication requirements.
Regenerate committed projections with their checked generators after source changes settle.
Do not edit generated views by hand.

Bind claims to exact sources, commands, inputs, outputs, and evidence levels.
Hashes establish content identity, not producer authenticity or loaded-code attestation.
Same-code reruns are not independent replication.
A public source milestone grants no model, physical, provider, or scientific completion claim.

For PID publication, retain the complete PID-M0 through PID-M8 process and achieved PID-P evidence level.
Keep canonical Markdown paired with its deterministic PDF view.
Bind source, renderer, fonts, output bytes, extracted text, and every page's visual review.
Do not edit a PDF as an independent scientific authority.

## Technical writing

Use ASD-STE100 Issue 9 as the project writing baseline.
Describe the result as **STE-aligned**, without a formal compliance or certification claim.

- Use American English, active voice, and one term for one concept.
- Limit descriptive sentences to 25 words and procedural sentences to 20 words.
- Put each condition before its action and give one instruction per step.
- Use simple tenses without contractions or semicolons.
- Keep one topic per paragraph and at most six sentences.
- Define each mathematical symbol, unit, assumption, and operating bound.
- Keep equations, prose, diagrams, and rendered documents consistent.
- Provide accessible native SVGs, readable mobile views, and direct vector fallback links.
- Inspect actual renders at useful sizes and zoom levels.

Exact scientific meaning takes priority over vocabulary simplification.
Exempt code, commands, identifiers, paths, literals, equations, tables, quotations, licenses, and historical records from sentence limits.
Do not rewrite immutable intake, generated files, vendored code, or submodule documents for style.
Run the Markdown and documentation gates.

## Required checks

Read [CONTRIBUTING.md](CONTRIBUTING.md) and [justfile](justfile) for the complete commands.
Before publication, run:

```bash
uv sync --locked --group ui
just check
```

The `just check` gate includes Rust formatting, lean graph checks, all-feature Clippy, tests, rustdoc, Python, observers, firebreak, docs, and notices.
Run focused checks during iteration without calling them the complete gate.
Use these additional gates when their surfaces change:

| Surface | Gate |
| --- | --- |
| Documentation | `just docs-audit` |
| Candidate artifacts | `just release-candidate-audit`, after exact source capture and regeneration |
| Native NCP capture | `just ncp-local-capture-check` |
| Legacy wire observer | `just ncp-observer-test` |
| Engram managed observer | `just engram-managed-observer-check` |
| LeWM default admission and arithmetic | Existing `tests/python/test_lewm_*.py` tests |
| Actual LeWM model path | Opt-in command in [its owner guide](docs/lewm/README.md) |
| Formal model or runner | `just formal` with exact Z3 4.16.0 |
| Rust manifest, lock, or dependency policy | Applicable root and excluded-consumer `cargo deny --locked ... check` commands |

The candidate audit binds a source capture. It is not evidence for a future commit.
Distinguish a scoped source milestone from a complete immutable operational release.
Keep commits focused. Preserve unrelated work and do not add AI co-author trailers.

## Preserved legacy compatibility

The legacy observer remains pinned to `v0.8.0` and wire 0.8.
The separate August 13, 2026 review observed NCP at `1a04294c90c1b50eba06ae1c6afe9c951319250d`.
That source is the unreleased, release-blocked `1.0.0-rc.1` candidate, with compact proto contract hash `163acc57d8a62b66`.
Its P01, P02, and P03 tasks remain OPEN, not dependency-ready, and **NOT RUN** in the retained review.
P03 includes Prisoma observer-role qualification.
The [dated task ledger](https://github.com/sepahead/NCP/blob/1a04294c90c1b50eba06ae1c6afe9c951319250d/evidence/implementation/task-ledger.v1.json) preserves that boundary.
These identities do not describe the separate native local capture package.

The `sepahead/engram` repository remains a README-only placeholder.
The executable host lives in `sepahead/Paper2Brain`.
No live Paper2Brain-to-Prisoma producer is qualified by the legacy observer's retained review.
The separate native local capture and Host API packages retain their own contracts.
