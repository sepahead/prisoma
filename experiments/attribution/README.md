# Attribution deletion ranking-sensitivity diagnostic (exploratory)

> Attribution is not a confirmatory claim in the v12.5 registry. This package is an
> exploratory diagnostic and never substitutes for the population, measure,
> estimator, or application gates on PID.

The package attributes a declared scalar model output to input features, then asks a
narrow validation question: **does that feature ranking identify explicit baseline
replacements that change the output sooner than uniformly random rankings on a
group-disjoint held-out set?** The measured absolute output change is deletion
*ranking sensitivity*. It is not causal or mechanistic faithfulness.

## Frozen validation contract

`RankingSensitivityGate` requires all design choices that used to be implicit:

* a baseline name and provenance statement;
* distinct method-selection and validation split names;
* grouping and predictor-determinism provenance statements;
* the method-selection group/unit identifiers, so overlap with validation fails
  closed;
* a predeclared `alpha`, minimum independent-group count, deletion-step count,
  deterministic seed, and random-ranking count.

The complete canonical gate manifest is hashed, and `frozen_gate_id` must equal its
content-derived `sha256:` identifier. An arbitrary human label is rejected. The
predictor is also evaluated repeatedly on each original input; a stateful or otherwise
nondeterministic result causes a typed abstention.

Every `AttributionValidationCase` declares the exact baseline tensor, case id,
independent group id, and underlying unit ids. Case, group, and underlying-unit reuse
across the validation set is rejected or abstained, rather than treated as additional
sample size. A well-formed set below the frozen group count abstains. **Any exact tie
in attribution magnitude causes a typed `ranking_ties_unresolved:<case_id>`
abstention**; the implementation does not invent a tie order or average over multiple
rankings. A constant attribution therefore abstains rather than failing as if an
identified ranking had been tested.

For each case, the implementation computes the **mean absolute deletion sensitivity**
of the declared attribution order and compares it with a seeded random-ranking
distribution. A group counts as a win only when the method sensitivity exceeds the
random-reference mean and its plus-one randomization-tail probability is below one
half. The final decision uses a conservative one-sided group-win binomial tail with
null win probability one half across independent groups. This is neither deletion
AOPC nor an exact sign test. There is no post-hoc effect margin or `mean + 3 SEM`
rule. Configuration is rejected when even unanimous group wins could not attain the
declared `alpha`, or when the random reference's worst-case binomial Monte Carlo
standard-error bound is wider than `alpha`.

The caller must name exactly one requested method as `primary_method`. Secondary
methods remain diagnostics even if their own ranking-sensitivity result passes; only
the predeclared primary method can set the legacy run-log compatibility boolean to
true.

The design still depends on declared independence and split provenance; it cannot
prove that upstream data collection honored them. Freeze and content-bind a real
split manifest before interpreting a production result.

## What is implemented

* `faithfulness.py` implements the bounded group-level ranking-sensitivity gate. Its
  `faithfulness_check` compatibility name exists only because the canonical run-log
  schema retains that field name; new code should call
  `ranking_sensitivity_check`.
* `probe.py` computes every requested attribution method on the same held-out cases,
  preflights the complete declared design structure and total method-plus-gate forward work
  against a fixed multiply-add budget, and binds one predeclared primary method.
  Before any attribution is evaluated it rejects malformed/leaky designs and work
  that exceeds the complete budget.
* `runlog.py` writes bounded `run_started` / `config_logged` /
  `artifact_logged` / `attribution_logged` / `run_ended` JSONL plus immutable
  content-addressed artifacts. Each method receives a compact NumPy relevance artifact
  and a canonical JSON evidence bundle containing the exact model parameters, gate
  manifest and hash, case/group/unit identities, inputs, baselines, every relevance
  array, group statistics, software versions, and source hashes. Companion
  `artifact_logged` events make both artifacts visible to the canonical manifest.
  Metadata records the typed status/reason, conservative group-win binomial tail,
  per-group contrasts and randomization probabilities, baseline/split/grouping
  provenance, primary/secondary role, random-reference resolution, and the
  limitations below.
* `attribute.py` provides a detached-attention, value-path-only epsilon-LRP baseline
  and a finite-difference gradient-times-input comparator for the small reference
  model. The epsilon-LRP baseline is explicitly **not AttnLRP**.

The NumPy `SmallTransformer` is a runnable stand-in, not a production VLA. A real
transformer integration should pin and validate its model/checkpoint and attribution
implementation independently; swapping in LXT/AttnLRP does not waive this gate.

## Limitations that remain even after a pass

* Baseline replacement can be out-of-distribution. A zero tensor is not assumed to be
  neutral merely because it is numerically simple.
* Dependent or redundant features make single-feature rankings intervention-order
  dependent; deletion does not identify unique feature effects.
* Absolute output change measures sensitivity to replacement, not direction,
  desirability, necessity, sufficiency, mediation, or a physical action effect.
* The random-ranking comparison cannot establish a causal pathway or mechanistic
  explanation, and it does not validate agreement with PID as a shared estimand.
* Method/layer/hyperparameter selection on the validation groups invalidates the
  gate. Use a separate selection set and a second untouched group-disjoint set.
* A synthetic demo pass is software evidence only. It is not evidence about a real
  policy, dataset, task family, or intervention distribution.

## Usage

The demo creates a deterministic multi-case validation set and an explicit
shape-matched zero baseline whose metadata says that distributional support is not
established:

```bash
python -m experiments.attribution demo \
    --runlog outputs/attribution_runlog.jsonl --artifacts outputs/attribution
cargo run --manifest-path pid-rs/crates/pid-runlog/Cargo.toml \
    --bin pid-runlog-replay -- --validate outputs/attribution_runlog.jsonl
cargo run -p pid-rerun --bin runlog-to-rerun -- \
    outputs/attribution_runlog.jsonl --load-attribution-artifacts \
    --save outputs/attribution_runlog.rrd
```

Each method prints `passed`, `failed`, or `abstained`, the typed reason, and the
conservative group-win binomial-tail probability when computed. Probe evidence
publication requires the confined artifact directory and remains bounded and
no-clobber. The Rerun adapter surfaces the recorded compatibility check and
provenance, not validated faithfulness; external relevance loading remains explicit
and exact-hash/shape checked.

Focused tests in `tests/python/test_attribution.py` cover informative, constant,
constant-output, and adversarial rankings; selection/validation and within-validation
leakage; insufficient groups; malformed/non-finite arrays and predictor outputs;
under-resolved or unattainable frozen gates; content-derived gate identity; predictor
determinism; exact-tie abstention; one-primary-method multiplicity; complete-work
budgets; relevance conservation; optional autograd agreement; reconstructable
evidence bundles; and run-log publication/validation behavior.
