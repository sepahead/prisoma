# E1 bounded pressure reference

The [reference module](../experiments/e1_reference.py) implements E1 numerical arithmetic and proposed forecast commitments.
It has no simulator, checkpoint, transport, or label-access capability.
Its tests use synthetic values and establish no native or scientific result.

The experiment identifier is `NCP-E1-ACTION-CONDITIONED-PRESSURE-FORECAST`.
This small learned reference remains separate from advertised product-model qualification.
W1–W3, the release review, and the candidate promotion states remain unchanged.

## Exact study and population

The [study fixture](../tests/fixtures/e1/study.v1.json) binds the numerical choices.
The [prospective seed roster](../tests/fixtures/e1/seed-roster.v1.json) binds every episode before native outcomes exist.
The module reconstructs both records without ambient randomness.

| Split | Episodes | Permitted use |
| --- | ---: | --- |
| Qualification | 8 | Separate checkpoint and execution controls |
| Training | 64 | Fit transforms, four ridges, and the constant baseline |
| Development | 16 | Select one trained ridge |
| Held out | 32 | Evaluate the fixed selection without refitting |

Every episode has distinct placement, runtime, and acoustic seeds.
All 360 seeds are globally unique and exclude bootstrap seed `104729`.
The declared SHA-256 procedure derives each unsigned 32-bit seed from its episode, role, and collision nonce.
Qualification episodes never enter training or evaluation.

The position projection uses the native unsigned 32-bit LCG recurrence:

$$s_{j+1}=(1664525s_j+1013904223)\bmod 2^{32}.$$

Here, $s_0$ is the placement seed, and $j$ identifies successive draws.
The explicit position is $(s_1/2^{32}-0.5,8,s_3/2^{32}-0.5)$ meters.
The fixed vertical coordinate still consumes the second draw.
These finite pseudorandom design points do not establish literal continuous independence.
The separate runtime seed does not imply identical random-number consumption after explicit-position preparation.

Native execution needs a new workload identity and a separate prospective freeze.
The scene has one body, no solids, and the inherited force-ground material, acoustic, thermal, and control semantics.
The study fixture records camera geometry, pressure placement, and the 120-Hz body clock.
The numerical module cannot verify the prepared scene from observation values alone.

## Inputs, actions, and target

The landmark follows completed tick 12.
It contains both complete due 160 × 120 camera frames and pressure from ticks 10–12.
The original pressure intervals are `[1200,1333)`, `[1333,1466)`, and `[1466,1600)`.
The three blocks contain exactly 400 samples.

The consumed metadata follows the published CREBAIN [RGBA owner](https://github.com/sepahead/crebain/blob/a5037a23a8e55d39ca0f09da2c25853c5209467b/integrations/ncp-force-ground-sensors/contracts/rgba8.semantic.v1.json) and [pressure owner](https://github.com/sepahead/crebain/blob/a5037a23a8e55d39ca0f09da2c25853c5209467b/integrations/ncp-force-ground-sensors/contracts/pressure.semantic.v1.json).
RGBA bytes use `u8`, `c_contiguous`, `rgba8-srgb`, and `bottom-left`.
Pressure uses `f64le`, `c_contiguous`, `16000` Hz, and `pascal`.
Source and availability ticks must match each declared block.
Missing, partial, future, foreign-declared, and nonfinite inputs fail their corresponding checks.

The eight observation features are six channel means, pressure mean, and pressure RMS.
Channel means divide exact integer channel sums by $19200\times255$.
They describe encoded RGB values, not physical radiance.
Alpha is ignored only after the exact RGBA layout passes.
The input identity still includes every original alpha byte.

The action order is neutral, positive pitch, negative pitch, positive roll, and negative roll.
Nonzero angles are $0.03$ radians with the declared sign.
Each target is armed `force_attitude_height`, heading zero, altitude eight meters.
It applies at tick 13 and holds through tick 36.
The 26 features contain eight observations, roll, pitch, and each observation's roll and pitch products.
The intercept is separate, and the study lists each feature's unit.

The target uses the original pressure samples from ticks 34–36.
Their intervals are `[4400,4533)`, `[4533,4666)`, and `[4666,4800)`.
The target-function semantic digest is `2587d58265dfec6decf9d2255a410ca195a337e1bbf1020ccaac4f37ee6c1505`.
CREBAIN owns native target evaluation and checkpoint completeness.

For ordered samples $p_i$ in pascals, define $m=\max_i|p_i|$.
If $m=0$, the result is positive zero.
Otherwise, sequential binary64 Neumaier accumulation sums $(p_i/m)^2$ in payload order.
The result is $m\sqrt{(s+c)/400}$, where $s$ is the running sum and $c$ is its correction.
The mean uses the same scaled accumulation over signed ratios.
Nonfinite samples or intermediates fail instead of changing the target.
Raw payload identity preserves signed zero separately from numerical output.
Frozen stress vectors qualify the tested runtime arithmetic, without claiming universal cross-platform bit identity.

## Training and numerical contract

The selected solver uses CPython 3.11, NumPy 2.4.6, and binary64 arrays.
The owning dependency lock selects that NumPy version for Python below 3.12.
The operational freeze must also bind the actual Python, NumPy wheel, installed inventory, and numerical backend.
A version string or supplied inventory digest does not attest loaded library bytes.

Training contains 320 rows: five actions for each of 64 episodes.
Only exact numeric constants are dropped.
Each retained column records its maximum absolute value, normalized mean, and normalized population scale.
This factored representation avoids overflowing raw sums and differences.
A nonconstant column whose scale becomes zero fails.
Development and held-out inputs never refit these decisions.

The objective is summed squared error plus $\lambda\sum_{j=1}^{d}\beta_j^2$.
Here, $d$ is the retained feature count, $\beta_0$ is the unpenalized intercept, and $\lambda$ is the ridge penalty.
The labels divide by their positive training maximum, or one when all training targets are zero.
Predictions multiply by that same target scale.
The scaling preserves the stated physical-unit objective and penalty choice.

The augmented least-squares matrix adds `diag(0, sqrt(lambda), …)` below the design matrix.
`numpy.linalg.lstsq` uses `rcond=1e-12` and must return finite coefficients with full augmented rank.
The implementation also checks the normal residual against its frozen relative bound:

$$\|A^T(A\beta-y)+\lambda P\beta\|_\infty
\le10^{-10}\left[\|A\|_1(\|A\|_\infty\|\beta\|_\infty+\|y\|_\infty)
+\lambda\|P\beta\|_\infty\right].$$

Here, $A$ is the normalized design including its intercept column, $y$ contains scaled targets, and $P=\operatorname{diag}(0,1,\ldots,1)$.
The matrix norms are induced norms, and the vector norms are maximum absolute values.
An intercept-only fit uses a zero penalized norm.
Finite negative predictions become zero before scoring and action selection.

The fixed penalties are `1e-6`, `1e-3`, `1`, and `1e3`.
Development selection minimizes mean episode MAE, with exact ties choosing the larger penalty.
There is no approximate tie threshold or training-plus-development refit.
The constant baseline uses the frozen 320-target training mean.
Persistence repeats the landmark RMS for all five actions.

## Stage commitments and authority

| Stage | Committed forecasts | Actual collection policy |
| --- | --- | --- |
| Training | Five persistence values | Neutral |
| Development | Five values from each frozen ridge and each baseline | Neutral |
| Held out | Five selected-ridge values and both baselines | Selected ridge's minimum, with fixed action-order ties |

Training cannot use a ridge that does not yet exist.
Development labels select a penalty only after all candidate forecasts exist.
The selected penalty cannot retroactively claim control of development collection.
The selected reference and selection artifact must freeze before the first held-out label becomes available.

`Commitment.record()` exposes the complete proposed forecast content.
Its digest can join C1's separate forecast-commitment field.
The collection policy and selected action remain distinct from every candidate's recommendation.
Closed canonical JSON loaders require selected byte identities and reject contract, feature, solver, and roster drift.
They use no pickle or executable model loader.

These records do not establish publication time, pre-label ordering, or action execution.
The future canonical adapter must join C1 decision and execution receipts to these exact bytes.
It must qualify retained checkpoints, independent restored owners, original payloads, common random numbers, and terminal closure.
The predictor receives observations and candidate actions, never checkpoint handles or evaluation capabilities.
This module neither implements that adapter nor invents a checkpoint API.

## Evaluation and null results

Episode MAE averages all five absolute forecast errors in pascals.
Both baseline contrasts use baseline MAE minus reference MAE.
Positive differences favor the reference.
All five branches remain one episode cluster.

The paired bootstrap uses 10,000 shared resamples of the same 32 episodes.
It uses `PCG64(104729)`, unsigned 32-bit indices, and NumPy's linear 2.5% and 97.5% quantiles.
The frozen little-endian index digest is `2f2e3aa4f7869010334213e1ac4bd7b0e25d2b7892ef3c80cff6208fccb1d203`.
The interval assumes exchangeable episodes from the declared design.
It supplies no power guarantee, posterior calibration, or exact finite-sample coverage guarantee.

Both lower interval endpoints must exceed 5% of the same frozen training-mean target.
Zero or unrepresentable positive margins produce an uninformative result.
No within-episode action variation across the entire held-out population also produces an uninformative result.
An interval below the threshold remains null or inconclusive.
Missing, failed, duplicated, or replaced episodes cannot produce a complete report.

`benefit_established` describes the arithmetic decision on supplied numerical rows only.
Every report retains false native-ancestry, canonical-order, scientific-validation, calibrated-posterior, and release-qualification fields.
Selected-action realized pressure needs a separate qualified join to actual execution and the restored neutral label.
Forecast error never supplies that policy contrast.

## Alternatives and decisive controls

| Approach | Assumption and benefit | Failure mode | Decisive check |
| --- | --- | --- | --- |
| Existing affine simulator reference | Cheap deterministic control example | Different state target and same-law labels | Preserve its separate contract gate |
| Existing LeWM PushT adapter | Available pretrained model | Incompatible action normalization, image path, and pressure target | Reject an unqualified units or action projection |
| New general learning stack | Flexible model capacity | Adds unreviewed tuning and dependencies | Require a separate model study and locked budget |
| NumPy normal-equation inverse | Short ridge implementation | Squares conditioning and hides rank loss | Compare singular and collinear controls |
| Selected bounded augmented least squares | Existing locked dependency and explicit contracts | Runtime numerical drift or unsupported native ancestry | Residual, exact artifact, paired-negative, and separate C1 gates |

The selected approach minimizes new machinery and keeps the solver contract inspectable.
Mathematical review checks arithmetic, units, conditioning, and the objective.
Validity review checks split custody, stage policies, pairing, thresholds, and null outcomes.
Authority review separates numerical records from canonical and native actions.
Provenance review binds original input bytes, training rows, source identities, and frozen artifacts.
Operator review checks clear failure codes, complete denominators, and honest result labels.

## Run the source controls

```bash
uv sync --locked --group ui
just e1-reference-check
just check
```

The [controls](../tests/python/test_e1_reference.py) include frozen stress vectors, independent decimal arithmetic, closed-form ridge, synthetic fitting, and paired bootstrap outcomes.
They also reject wrong layouts, late inputs, altered identities, stage changes, incomplete splits, and changed scores.
Native C1 qualification, immutable installation, prospective operational freeze, and the complete E1 campaign remain separate gates.
No native E1 episode is executed by these commands.
