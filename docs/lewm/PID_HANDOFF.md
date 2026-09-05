# World-model evidence and the pinned PID boundary

Prisoma retains its substantive integration with `pid-rs`.
The local model probe does not change the submodule or any upstream PID source.
It does not produce an admissible PID dataset yet.

## Concrete variable map

| Captured object | Actual meaning | Permitted interpretation |
| --- | --- | --- |
| Input pixels | Current PushT RGB render | Observed visual input |
| Initial encoded latent | Encoder/projector output before candidate conditioning | Visual representation, not an independent modality |
| Candidate sequence | Five blocks of standardized two-dimensional actions | Proposed model input; raw execution remains disabled |
| Predicted latent sequence | Action-conditioned model forecast | Candidate-conditioned predictive state |
| Goal embedding | Encoded task target image | Declared conditioning input |
| Latent goal score | Deterministic squared distance between forecast and encoded goal | Planner objective; not an observed physical outcome |
| Selected recommendation | Returned CEM mean, separately scored | Proposed sequence; not an executed action |
| Future reference state | Not collected in the first probe | Absent; no substituted label |
| Language source | Not consumed by this LeWM route | Absent; no text hash or zero-vector replacement |

The model is visual and action-conditioned. It does not supply a language-conditioned `(V,L,D,A)` capture regime.
Prisoma must use an explicit variable roster for a later admissible experiment.
It must not fabricate missing axes to satisfy an existing four-axis consumer.

## Ancestry and target control

A prediction landmark must identify when all declared source inputs became available.
Each tensor receipt must bind its producer, exact weights, input artifacts, shape, dtype, device, and maximum ancestor time.
It must also bind pooling, masks, fitted transforms, and the exact target definition.

A candidate-conditioned latent contains information from its candidate action by construction.
It cannot become a PID source whose target is that same candidate action.
Cross-fitting cannot repair this target injection.

A later measured reference outcome can be a distinct target.
Every matched baseline must then receive the exact candidate action and the same permitted observations.
A controller-command target supports command prediction only. It cannot establish physical forecast accuracy.

## Sampling and estimator admission

The 9,000 adaptive CEM proposals are not 9,000 independent experimental units.
They share one observation and depend on earlier selection rounds.
The current single-state probe cannot identify a population PID quantity.

A future handoff must declare:

1. The target population, sampling unit, action-selection law, support, and cluster structure.
2. The measure, defining reference, functional, atom coordinate, domain, and units.
3. The estimator, projection, reference fit, hyperparameters, and diagnostic thresholds.
4. The application claim, matched baseline, prediction landmark, and exclusion rule.

Population, measure, estimator, and application gates remain separate.
High-dimensional continuous vectors do not automatically have a supported estimator or finite information.
Unsupported routes must return typed abstention and a reason. They must not emit numeric placeholder atoms.

The present probe exports original arrays and their content bindings.
A later reviewed consumer can construct an eligible dataset without modifying the upstream estimator.
Its source, action, target, and validity contracts must pass before invoking the pinned public PID API.

## Owner boundaries

Prisoma owns experiment ordering, variable provenance, eligibility, and the comparison population.
`pid-rs` owns its published estimator and run-log APIs.
Rerun supplies its existing data and visualization capabilities.
CREBAIN owns its separate world, dynamics, observations, and accepted checkpoint construction.
The Agent Bridge remains the canonical mutation plane for a complete Prisoma experiment.

No current scientific task status changes from this handoff proposal.
