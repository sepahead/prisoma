# LeWorldModel M4 compatibility probe

Probe date: 2026-08-14.

Status: local engineering observation only.

This probe does not qualify LeWorldModel, MPS, PushT, W1, or W2. It does not establish model
quality, closed-loop value, latency, memory fitness, or action-support validity.

## Exact reviewed inputs

| Input | Exact identity |
|---|---|
| LeWM source | `Mengarr/lewm@8a2c595813d0eee85b2dbffa6f58ff0842f9e673` |
| LeWM dependency lock | `uv.lock` SHA-256 `1bf638a080ce7717ee000f5b0be9de1ca327624025ba52433c7fbcbcc90d024e` |
| Planning package | `stable-worldmodel==0.1.1` wheel SHA-256 `00eaabd9e046e6364b3d1db47e5b365a0f628aea3a9376d6a407f75cbbbd2ef5` |
| Planning package source tag | `15a5538d492ae524c64cb18cc56a2d70611e877e` |
| Pretraining package | `stable-pretraining==0.1.7` wheel SHA-256 `60fc8fc3c9490e9a059aa7e038ab62cbe0505841e78c4165c18a99d8f599ec65` |
| PushT checkpoint | `quentinll/lewm-pusht@22b330c28c27ead4bfd1888615af1340e3fe9052/weights.pt` |
| Checkpoint bytes | `72,290,721` |
| Checkpoint SHA-256 | `48938400ae3464c9680731287f583a9cb516f55a8ec64ea13a91be47fb15b607` |

The test loaded the checkpoint with `torch.load(..., weights_only=True)`. It required an exact
state-dictionary match. It instantiated `JEPA` and `ARPredictor` from the pinned LeWM source.
It used only the CEM solver from the locked platform wheel.

This distinction matters. The wheel also contains `stable_worldmodel.wm.lewm.LeWM`. Its rollout
implementation differs from the pinned repository's `lewm.jepa.JEPA`. Both accept the checkpoint
with strict key matching. A strict weight load therefore cannot prove implementation identity.
The port must bind both source bytes and weights.

Current `stable-worldmodel` main was reviewed at
`9a66d7d020043c8efb507f45373e808714f0842d`. Its CEM constructor takes a `cost` object. The pinned
LeWM evaluator passes `model`. Current main is a migration target, not the released evaluator's
exact dependency.

## Host and probe environment

| Field | Observation |
|---|---|
| Host | Locally measured: Apple M4 Max, arm64, 128 GiB unified memory |
| Operating system | macOS 26.5.1 |
| Probe Python | CPython 3.11.13, arm64 |
| PyTorch | 2.12.1 |
| MPS | built and available |
| Constructed parameter count | 18,034,478 |

The paper reports about 15 million parameters. The local count includes every parameter in the
instantiated encoder, predictor, action encoder, projector, and prediction projector. Treat the
difference as a counting-boundary question until the authors' convention is reproduced.

## Predictor and rollout observation

The probe used deterministic synthetic finite tensors. It ran one direct prediction and one
four-candidate latent rollout on CPU and MPS.

| Check | Observation |
|---|---:|
| CPU prediction finite | yes |
| MPS prediction finite | yes |
| CPU rollout finite | yes |
| MPS rollout finite | yes |
| CPU repeat maximum absolute drift | `0` |
| MPS repeat maximum absolute drift | `0` |
| CPU action-change prediction L2 | `0.3433798254` |
| MPS action-change prediction L2 | `0.3433809876` |
| CPU/MPS prediction maximum absolute difference | `3.2782555e-6` |
| CPU/MPS rollout maximum absolute difference | `3.5762787e-6` |

These values show finite execution and action sensitivity on one synthetic input. They do not set
a parity tolerance. They do not test official preprocessing, a real observation, or candidate
ranking.

The probe script and synthetic input are not committed. These values are not a
repository-reproducible receipt.

## Exact-budget CEM observation

The second probe used the locked 0.1.1 CEM implementation and the pinned LeWM model. It used:

- 30 CEM rounds;
- 300 samples per round;
- 30 elites;
- horizon five;
- five two-dimensional actions per action block; and
- MPS seed 3072.

Two independent constructions produced identical final actions, final cost, and full recorded
round traces. Every recorded candidate, cost, mean, and scale was finite. The final action digest
was `2874b668f2e34205e6db2611d6c96a717e20cd6001e0e0b680b2fbefd9cac27a`.

No timing value from this two-run probe is admissible as a benchmark. It lacks warmup, a stable
thermal protocol, sufficient repetitions, the official preprocessing path, and the PushT loop.

## Direct evaluator findings

The released evaluator creates each `StandardScaler` from the evaluation dataset. It then selects
evaluation starts from that same dataset. This is acceptable as a descriptive reproduction of the
released script. It is not a held-out transform for a Prisoma confirmatory study.

The locked CEM solver accepts a `Box`, but it uses that object only to obtain the action dimension.
It samples an unbounded Gaussian in standardized coordinates. It does not clip or reject by
`Box.low` and `Box.high`.

In the synthetic unit-box diagnostic, the first CEM round placed 4,785 of 15,000 standardized
candidate scalars outside `[-1, 1]`. The final round placed 5,727 outside. These counts do not prove
raw PushT action violations. The published policy inverse-transforms standardized actions before
execution. They do prove that the solver itself does not enforce the supplied `Box` bounds.

The solver returns the final mean, final scale, and the last mean elite cost. It does not, by
default, retain every proposal and forecast. It also does not separately score its returned final
mean after the last update.

## Independent reproduction boundary

Singh's 2026-08-10 independent preprint reproduces LeWM on TwoRoom with one seed. It does not test
PushT, MPS, other environments, or seed variance. It reports four outcome-relevant pipeline
conventions that were absent from released configuration files:

- dense action gathering across each frameskip block;
- programmatic action-encoder width;
- ImageNet pixel normalization; and
- action z-scoring with NaN-row removal.

The preprint also reports conflicts between the appendix and released configuration. These include
goal offset 100 versus 25, step budget 150 versus 50, and CEM iterations 10 versus 30. On the same
50 episodes and released checkpoint, protocol-sensitive success moved from 84% to 8% through goal construction alone.
Across three checkpoints, one-step prediction error did not order long-horizon planning.

These author-reported reproduction findings do not establish a PushT defect. They do establish a
qualification requirement. The PushT adapter must bind each preprocessing, action, normalization,
goal, start-state, episode-selection, horizon, budget, and replanning convention to exact source
bytes. If paper, configuration, and code disagree, freeze each feasible reading before outcomes.
Report all frozen readings. Do not choose one because its result is favorable.

Primary source: https://arxiv.org/abs/2608.10145. Public reproduction source observed on the review
date: `joyjeet-singh/tinylab@f2f665411d79cd626096ec8d4271b355a2c0f550`.

## Prisoma decision

Keep two arms distinct:

1. The descriptive reproduction arm preserves the released dependency lock and evaluator
   behavior. It cannot serve as held-out W1 or W2 evidence.
2. The Prisoma port arm fits scaling on training rows only. It content-binds that fit. It checks
   every inverse-transformed proposal against raw action support. It uses one frozen reject,
   projection, or truncated-sampling rule.

The port arm must also retain every proposal, prediction, score, elite set, mean, and scale. It
must separately score and commit the final recommendation before execution. A support-bounded CEM
is not the exact released solver. Report it as a distinct arm.

## Remaining qualification gates

The following work remains open:

1. Build a paper, configuration, and executable-code concordance ledger for the PushT route.
2. Freeze each unresolved feasible protocol reading before outcome access.
3. Reproduce official image preprocessing on CPU.
4. Run the same content-bound inputs on MPS.
5. Freeze numeric and candidate-order tolerances before broad evaluation.
6. Bind a train-only normalizer and raw action-support rule.
7. Run PushT with restored states and more than one closed-loop replan.
8. Capture every adaptive CEM round and score the final recommendation.
9. Compare CPU and MPS candidate ordering under identical random inputs.
10. Disable the network after artifact staging.
11. Measure cold start and at least 1,000 warm decisions under a thermal protocol.
12. Record p50, p95, p99, missed deadlines, peak unified memory, and CPU fallback.
13. Complete checkpoint, package, model, data, and transitive-rights review.
14. Run the W1 and W2 controls in `grandplan.md`.

Until these gates pass, use **MPS candidate**. Do not use **MPS supported** or **MPS qualified**.
