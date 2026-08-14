# WorldWarp comparator specification

**Docset alignment:** v13.0 (legacy optional comparator, not the W1/W2 critical path)

**Status:** E1 interface specification only

**Implementation:** Not built in Prisoma

`grandplan.md` section 16 is the decision record. WorldWarp is an optional external comparator.
It is not a Prisoma dependency, integration, producer, or thesis prerequisite.

## Current boundary

The candidate upstream repository is <https://github.com/sepahead/WorldWarp>. Prisoma has no
pinned WorldWarp revision, consumer adapter, executable fixture, or rights-approved model bundle.
Verify the upstream implementation, models, licenses, and compute needs before any adoption.

Generated scenes are not observations, simulator ground truth, or causal interventions. A
generated frame or latent becomes a candidate D source only after the section 9.1 mapping review.
D never means depth, dynamics by default, or natural policy use.

## Deployed-graph admission

Before adapter work, classify the exact upstream graph with the six classes in `grandplan.md`
section 9.2. Record whether prediction is training-only, intended-future conditioned, coupled to
action in a joint sampler, exposed as an action-conditioned query, or part of candidate selection.

Do not infer an operational action-conditioned query by factorizing a joint density.

Do not use `counterfactual` for an action-conditioned prediction without randomized executed-
action validation. Do not use `planner` unless the runtime proposes, predicts, scores, and selects
over at least two actions.

The current August review found that several WAMs remove their future branch at deployment.
Flex-\(\pi\)'s generated future cannot attend candidate actions. Model branding cannot replace
this graph audit. See the
[dated frontier review](docs/audits/2026-08-12-first-principles/WORLD_ACTION_MODEL_FRONTIER.md).

## Required adapter contract

If Prisoma adopts this comparator, the adapter must:

- use the Agent Bridge as its only request plane;
- record each request and result in the canonical run log;
- pin the upstream revision, model, checkpoint, configuration, and execution environment;
- bind prompts, source media, camera paths, seeds, outputs, and rights receipts by exact digest;
- keep observed inputs separate from generated predictions;
- declare population support and computation status for each emitted axis;
- enforce input, output, time, memory, and process limits; and
- remain optional when the external service is unavailable.

Rerun may show the recorded artifacts during diagnostic phases. A Phase 4 shell may render them
only from the same logged identities. No UI may create an unlogged control or evidence path.

## Admission before scientific use

Require all of the following before a WorldWarp result enters a study:

1. Complete the model, data, license, privacy, and redistribution review.
2. Implement a bounded, content-addressed adapter with replayable fixtures.
3. Test adapter conformance independently from the service implementation.
4. Freeze a separate counterfactual-support question and matched comparator.
5. Measure realism and support against observed or simulator-ground-truth interventions.
6. Apply the population, measure, estimator, and application gates to derived diagnostics.
7. For action-conditioned claims, pass the randomized executed-action causal gate.
8. For planning claims, pass the candidate-log and decision-flip tests.

Until those steps pass, WorldWarp remains an off-critical-path E1 proposal.
