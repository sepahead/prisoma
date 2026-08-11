# WorldWarp comparator specification

**Docset alignment:** v12.5

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

Until those steps pass, WorldWarp remains an off-critical-path E1 proposal.
