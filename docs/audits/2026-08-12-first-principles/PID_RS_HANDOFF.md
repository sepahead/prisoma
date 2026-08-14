# `pid-rs` handoff from the Prisoma first-principles audit

Review window: 2026-08-12 through 2026-08-13.

Prisoma pin retained: `796c11e70f009634b853dc4ada6f565563d82f51`.

Public `pid-rs` main observed: `7473e62acef6077c2c1147e09d5d1297f2a2874b`.

Latest reviewed estimator-code anchor: `cb3f58f0b190454cb3f1090de8798261ec78f194`.

Prisoma exact-revision consumer test: `722d3abeb922fc4119ecb9f92d7fedca096c9f77`.

No submodule update was made. This file is both the internal adoption brief and a message that can
be sent to the `pid-rs` repository.

The separate [`PID_RS_EXTENSION_BRIEF.md`](PID_RS_EXTENSION_BRIEF.md) compares twelve upstream
extension paths across twenty scientific and engineering lenses. It contains the selected work
order and a shorter message for the upstream implementation agent.

## Executive decision

Do not update the Prisoma pin yet.

The current upstream tree is materially stronger than the pin. It has method-identity catalogs,
clearer categorical atom types, exact categorical certification, formal assurance, KSG arithmetic
work, support-change records, software identity, source-errata records, and a proposed scientific
outcome schema. Public main is 97 commits ahead of the Prisoma pin.

An isolated clean Prisoma consumer build and all-feature test run against exact revision
`722d3abe` passed. The interval after the earlier `00fce70d` check changes assurance artifacts,
schemas, executable verifier scripts, prose, and tests. It touches three `pid-core` Rust source
files only in documentation or comments. It changes no Cargo manifest, public Rust signature, or
executable Rust statement in that later interval. The exact-revision run establishes compatibility for
Prisoma's current compiled and tested Rust consumer surface. It does not prove report-schema
compatibility, untested behavioral equivalence, Python compatibility, or scientific added value.

Public head `bbdfda40` is one later assurance commit. The `722d3abe..bbdfda40` delta
changes workflows, assurance records, formal replay documents, and Python verifier scripts. It
does not change `crates/`, any Cargo manifest or lock, `rust-toolchain.toml`, or `pyproject.toml`.
The consumer test therefore covers the exact code and dependency bytes that Prisoma consumed at
that head. It does not validate the changed upstream assurance scripts.

Estimator-code anchor `cb3f58f0` adds one bounded KSG integration commit after `bbdfda40`. It changes
three `pid-core` source files. The only production behavior change classifies kd-tree coordinate-
span overflow as `NumericalInstability`, which matches the brute backend. The other Rust changes
add predecessor-radius and structured-error regression tests. Prisoma inspected the exact diff and
replayed its four predecessor fixtures plus the overflow fixture on current-head bytes. All five
passed. This is a narrow review, not a full consumer replay or authority to update the pin.
Upstream still marks the broader revision-4 KSG repository integration **NO-GO**.

Current public head `7473e62` is the direct child of `cb3f58f0`. It changes workflow, custody,
formal, evidence, and verifier-script surfaces. It changes no file under `crates/`, no Cargo input,
and no Rust or Python package metadata. Its own changelog grants no estimator, PID, scientific,
integration, or review credit. Exact-head CI run
[`31724449805`](https://github.com/sepahead/pid-rs/actions/runs/31724449805) failed two of 45
jobs: the exact-count SxPID2 reference and KSG arithmetic/phase-isolation jobs. The separate
[`Push on main` run 31724449083](https://github.com/sepahead/pid-rs/actions/runs/31724449083)
passed. A narrow push receipt does not substitute for the failed full CI workflow.

The full pin-to-head interval does change package metadata. `pid-core` adds build dependencies on
`serde`, `serde_json`, and `sha2`. `pid-python` adds `serde_json`, and `Cargo.lock` changes. The new
`pid-core` build path creates software-identity data. A later adoption review must therefore cover
source-package builds, cache behavior, release archives, and dependency policy. The clean consumer
run proves that this exact dependency graph resolves and compiles. It does not prove every package
or distribution route.

The [`bbdfda40` CI run](https://github.com/sepahead/pid-rs/actions/runs/31651702557)
completed successfully on 2026-08-13. All 45 jobs passed. The
[`bbdfda40` CodeQL run](https://github.com/sepahead/pid-rs/actions/runs/31651702504) also passed.
These provider results satisfy the upstream-CI check only for that earlier head. They do not bind
`cb3f58f0` or replace consumer-owned compatibility, schema, package, and scientific-value review.
Exact-head [CI run 31686107959](https://github.com/sepahead/pid-rs/actions/runs/31686107959)
is red. Its certified-SxPID2 job rejected a stale reviewed workflow digest. Exact-head
[CodeQL run 31686106737](https://github.com/sepahead/pid-rs/actions/runs/31686106737) passed.

## Evidence collected by Prisoma

Prisoma's clean starting revision was tested in a temporary isolated worktree with `pid-rs`
replaced by public revision `722d3abeb922fc4119ecb9f92d7fedca096c9f77`.

```text
cargo +1.93.0 check --locked --workspace --all-features
cargo +1.93.0 test --locked --workspace --all-features --no-run
cargo +1.93.0 test --locked --workspace --all-features
```

All three commands passed. The all-feature run executed 531 Rust unit and integration tests. The
checked-in submodule remained clean at `796c11e`. The temporary worktree is an audit-only input and
must be removed before the Prisoma change closes.

Current main `7473e62acef6077c2c1147e09d5d1297f2a2874b` is 97 commits beyond the Prisoma pin.
The full consumer run remains bound to `722d3abe`. The `722d3abe..bbdfda40` interval changes only
upstream assurance, workflow, script, and prose surfaces. The later `bbdfda40..cb3f58f0` interval
contains the bounded Rust change reviewed below.
The direct-child `cb3f58f0..7473e62` interval changes custody and assurance surfaces only. It does
not change estimator code or dependency inputs.

At the final 2026-08-13 observation, all 45 hosted CI jobs for `bbdfda40` had passed. Its CodeQL
workflow also passed. This closes the provider-CI row only for `bbdfda40`.

The exact `bbdfda40..cb3f58f0` interval contains one commit. Prisoma inspected its three changed
production Rust files and the upstream claim boundary. These current-head focused tests passed:

```text
ksg_strict_radius_predecessor_reaches_both_backends
ksg_strict_radius_predecessor_preserves_swapped_ordered_counts
xblocks_strict_radius_predecessor_reaches_both_backends
xblocks_strict_radius_predecessor_preserves_selected_bits_when_marginals_swap
overflowing_coordinate_span_returns_numerical_instability_on_both_backends
```

## Concrete migration surface

The following delta is directly relevant to Prisoma.

### Scientific-object wording at the retained pin

The retained pin has two documentation defects. They do not change the implementation that
Prisoma calls, but they can mislead consumers:

- `crates/pid-core/src/sxpid.rs` lines 14–16 call categorical MGW and continuous Ehrlich shared
  exclusions “one measure.” They are distinct scientific objects with different domains,
  formulas, support assumptions, and continuous gauge choices. Shared lineage does not supply a
  mapping theorem.
- The same module describes same-row equal-width quantization helpers as “not an exact categorical
  estimator.” The helpers do evaluate the categorical MGW functional exactly on the derived
  empirical categorical law. They do not estimate the Ehrlich continuous functional of the
  original numeric variables, and their result changes with the fitted quantizer.

Recommended upstream wording:

> MGW categorical shared exclusions and Ehrlich continuous shared exclusions are related but
> distinct functionals. Fitted quantization creates categorical variables. The plug-in result
> targets MGW shared exclusions on their empirical categorical law. It is not an estimator of the
> original continuous-law functional, and no cross-domain equivalence is implied.

Prisoma corrects this boundary in its own types and prose. It does not patch the pinned submodule.

The retained pin also overstates fitted-quantizer provenance. `DiscreteInputEncoding::FittedEqualWidth`
and the Williams–Beer equivalent say that fitting occurred outside or separately from evaluated
rows. The public APIs cannot enforce that claim, and the stable Python test fits and evaluates the
same arrays. Domain-separated train and transform hashes do not prove row separation because the
same bytes can receive different domain hashes. Record an explicit fit/evaluation relation such
as `same_rows`, `disjoint_rows`, or `caller_asserted_unknown`, plus row-set commitments. Do not
derive separation from the existence of a fitted object or from unequal domain hashes.

Two more pin-level descriptions need correction:

- `AGENTS.md` and `kdtree.rs` place both KSG and Ehrlich `i^sx` behind the exact kd-tree/brute
  backend. KSG selects those backends. The Ehrlich redundancy term performs its own quadratic
  distance scratch, selection, and count loops. PID2 also calls KSG for its three mutual-information
  terms, but that does not make the redundancy term a kd-tree consumer.
- Default-off same-sample helpers call numeric inputs “continuous,” then quantize and evaluate the
  categorical MGW empirical law. A separate default-off Python migration function recommends
  Williams–Beer `I_min` when continuous kNN fails. Rename the helpers by the derived categorical
  object, and describe `I_min` only as a separate cross-estimand analysis chosen in advance.

### Categorical shared-exclusions atoms

The earlier `SxAtom` shape is split by meaning:

- `SxPointwiseAtom` represents one distinct joint realization;
- `SxAveragedAtom` represents the empirical-PMF average.

Accessors now carry units through names such as `informative_nats()` and
`misinformative_nats()`. Serialized values also carry an interpretation envelope. This is a
valuable type-level defense against mixing pointwise and averaged atoms.

Realization reports replace the ambiguous `prob` field with:

- `empirical_count`;
- `empirical_probability`.

Prisoma must review every Rust, Python, JSON, and documentation consumer of those fields before a
pin update. Compilation alone does not test serialized downstream records.

### Python categorical surface

Python categorical outputs move from a generic `SxAtom` identity toward the averaged categorical
atom identity. Prisoma does not currently ship a registry wheel, but any future local Python
adapter must use the renamed typed surface and confirm JSON compatibility.

### Identity and outcome records

Upstream adds software identity and a large schema-3 scientific outcome foundation. The release
scope describes this as a proposed review boundary rather than a stable downstream runtime
contract. Prisoma must not import schema-3 roadmap language into its canonical run-log claims.

The software-identity implementation adds a `pid-core` build script and three build dependencies.
Its recorded source state is a bounded build-time observation. It is not runtime attestation, and
Cargo can reuse it from cache. Prisoma must preserve that distinction in any adopted provenance
field.

### Resampling and experimental pipeline records

The experimental row-resampling surface now records the original row count, a per-replicate row
index hash, and algorithm revision 2. The revision separates row schedules from optional
perturbation streams. Categorical shared-exclusions bootstrap output now uses an all-or-none typed
summary status. It labels the summary as descriptive resampling variability with no coverage
guarantee. These are report and deterministic-stream changes, not only additive diagnostics.

Prisoma's consumer review found that the pinned generic row APIs already provide the critical
fail-closed semantics. `RowBootstrapResult.stats` is absent if any requested replicate fails.
`RowPermutationStat.tail_fraction` is absent unless all requested transforms are valid. The
typed permutation record also separates a Monte Carlo p-value from an approximate
stationary-surrogate score. Prisoma now checks those contracts again at its publication boundary.
It also requires each sidecar atom's original-data value to match the main report exactly.

Two useful upstream extensions remain. First, add a neutral companion name for the generic
`permutation_rows_pvalue_*` family because the same API returns a non-p-value surrogate score for
circular shifts. Keep the existing names as compatibility wrappers. Second, consider a typed
group-aware schedule API that never treats rows from separate episodes as one stationary series.
Prisoma currently owns that policy and fails closed on mixed coverage or multiple dependent
episodes. Neither request is an estimator defect in the pinned revision.

### Estimator and assurance changes

The upstream delta includes:

- stricter KSG arithmetic and observation contracts;
- support and concentration evidence;
- categorical exact-count and interval certifiers;
- Lean formal statements for bounded categorical surfaces;
- method-catalog and scientific-evidence coherence checks;
- changes to bootstrap, pipeline, preprocessing, and report surfaces.

These changes can strengthen estimator assurance. They do not validate continuous PID on
high-dimensional VLA embeddings.

## Prisoma v13 scientific-object contract

Prisoma now uses one exact categorical route for descriptive diagnostic development:

1. select either the all-row descriptive screen or the optional train-only split screen;
2. fit equal-width quantizers on the rows in that screen;
3. transform those same rows and construct empirical categorical laws;
4. call pinned `pid_core::stable::quantized::fitted_quantized_sxpid2_with_budget`; and
5. report averaged two-source MGW informative, misinformative, and net components in nats.

The current harness does not fit on training rows and score held-out rows. Its all-row screen fits
all rows. Its optional split screen fits and estimates on training rows only. A future inferential
route must freeze a training-fitted quantizer and evaluate a separately declared held-out law.

The CLI names are `categorical-sx` and `categorical-sx-pls`. The former `discrete` names now
reject. Each Prisoma report binds the functional, quantizer, estimator route, fitted edges,
transform hashes, dimensions, occupancy, information units, and out-of-range policy.

The PLS route is a separate supervised same-row diagnostic. Every value carries a typed warning
and an estimator-blocked gate. It is not a held-out result or a high-dimensional escape hatch.

This route is not Williams–Beer `I_min`, BROJA, the continuous Ehrlich functional, or an
infomorphic objective. Do not emit a `wibral_lineage` result identity. Model the relationships as a
provenance and estimand graph. A finite-sample estimator maps observations to an estimate of one
named functional. A declared-law evaluator maps a specified law to that functional's value.
Neither is a peer PID definition. Every artifact must declare object kind, domain, defining
reference, functional, a typed sample-estimator or declared-law-evaluator route, input-law kind,
preprocessing, units,
aggregation scope, and any composition. No result transfers across those objects without a
mapping theorem.

The provenance and estimand graph needed by Prisoma is:

- Wibral, Priesemann, Kay, Lizier, and Phillips (2017) supply the generic neural goal-function
  coordinate idea. They do not define one universal PID estimator or implement an arbitrary
  neural circuit objective.
- Makkeh, Gutknecht, and Wibral (2021) define categorical pointwise shared exclusions. Prisoma's
  fitted categorical route targets its averaged two-source functional on a new empirical law.
- Gutknecht, Wibral, and Makkeh (2021) supplies the parthood and formal-logic foundation. It is a
  semantic foundation, not a finite-sample estimator.
- Ehrlich, Schick-Poland, Makkeh, Lanfermann, Wollstadt, and Wibral (2024) define a related
  continuous counterpart with a distinct analytic estimand. Its source-disjunction kNN algorithm
  is the finite-sample estimator edge. No categorical-to-continuous limit or general
  output-identification theorem is assumed.
- Makkeh, Graetz, Schneider, Ehrlich, Priesemann, and Wibral (2025) define the bivariate
  infomorphic objective framework. Schneider, Neuhaus, Ehrlich, Makkeh, Ecker, Priesemann, and
  Wibral (2025) extend it to trivariate local objectives. These compositions are neither PID
  measures nor estimators.

For the same fixed categorical alphabet, event map, lattice, and exact source marginal law, the MGW
informative averaged components depend only on the source law. They are therefore invariant across
conditions that reuse that exact source law. An empirical route needs the same source counts, not
merely samples from the same population. Any net-atom difference is then the negative of the
misinformative-component difference. “Misinformative” is the formal nonnegative
negative-surprisal component, not error, harm, or deception.

The first upstream repair is a nonbreaking catalog graph. Keep current method IDs canonical for
the estimator routes that they already identify. Add paper-defined functional identities in a
separate scientific-object registry or method-catalog v2. Link each route with a typed
`targets_functional_id` edge. The highest-value new scientific input is then a bounded sparse
empirical-count-law MGW API. It should share canonical table mechanics with the row API and avoid
row expansion. Bridge its fixed-width count view to the arbitrary-precision exact-count certifier
through a versioned lossless schema adapter. Then add a separate specified-rational-law schema
and, after bounded agreement checks, a declared binary64 finite-law evaluator.

The supporting contract work is:

- typed fitted-transform receipts that compute relations from caller-declared row identities;
- a public resource constructor and conservative checked composition for retained output plus
  transient callback scratch;
- a fixed-source-law fixture that checks MGW informative-atom invariance;
- generic group-aware schedules that cannot splice independent groups;
- nominal p-value and surrogate-tail types that cannot enter the same FDR API; and
- a tuple-level continuous Ehrlich assumption and gauge contract.

A typed infomorphic-objective record comes later. It must preserve atom coefficients,
conditional-entropy terms, binning, gradient stops, and numerical guards. It must not call the
objective a PID measure.

The declared-mass and infomorphic items are research requests. The current integer-count certifier
does not certify soft sigmoid-derived mass laws, adaptive bins, stopped binning gradients, or training
guards. Sampling a target would change the object to a Monte Carlo empirical law. Prisoma will not
use that substitution silently.

## Consumer acceptance matrix

| Review item | Current evidence | Required before pin change |
|---|---|---|
| Rust source compatibility | All-feature check and test-target build passed | Keep as an exact-revision CI fixture in Prisoma. |
| Rust behavioral compatibility | Not established | Replay Prisoma analytic, abstention, tie, support, and resource fixtures. |
| Serialized report compatibility | Not established | Diff canonical JSON and status contracts with typed migration expectations. |
| Python compatibility | Not established | Build the local extension and replay report-first examples across supported Python/NumPy pairs. |
| Build and package compatibility | Exact workspace source build passed with the new dependency graph | Test package archives, Cargo cache/source-state behavior, and dependency-policy gates. |
| Categorical exact assurance | Strong upstream evidence | Confirm exact scope, assumptions, count bounds, and consumer field mapping. |
| Continuous KSG assurance | Improved upstream evidence | Re-run low-dimensional analytic and external fixtures. Retain abstention and support checks. |
| Continuous `I^sx_∩` validity | No Prisoma application evidence | Keep measure, estimator, and application gates closed. |
| Run-log compatibility | Not established | Replay schema-1 and schema-2 Prisoma fixtures and reject schema confusion. |
| Upstream hosted CI | `bbdfda40` CI and CodeQL passed. `cb3f58f0` CodeQL passed, but its CI run `31686107959` failed. Current-head `7473e62` full CI run `31724449805` failed two jobs; only the narrower `Push on main` run `31724449083` passed. | Require a green full CI run bound to the exact adoption head. |
| Prisoma application value | Not established | Show a reviewed improvement that closes a named Prisoma obligation. |

## Scientific boundary

Prisoma retains four independent PID gates:

1. population support;
2. PID measure;
3. estimator validity and uncertainty;
4. target application validity.

Upstream work can materially improve gate 3. Categorical theorems and exact certifiers do not pass
continuous gate 3. No upstream software result passes gates 1, 2, or 4 for Prisoma.

An abstained estimate must still have no numeric placeholder. A failed continuous term must never
auto-route to MGW categorical shared exclusions, Williams–Beer `I_min`, BROJA, or another
functional. Each object must remain separately named and interpreted.

## What to ask upstream

`pid-rs` already has `MIGRATION.md` and release-scope ledgers. The useful missing artifact is not a
second generic migration guide. Prisoma needs a compact consumer delta for the exact interval
`796c11e..7473e62`, with estimator-code changes distinguished from custody-only descendants.

Please provide or confirm:

- the Rust and Python symbol rename map;
- the serialized-field and status-contract delta;
- the minimum replay fixture set for a downstream consumer;
- which changes are intentional breaks before 1.0;
- which guarantees are categorical only;
- which continuous guarantees cover KSG MI versus KSG-backed shared exclusions;
- the exact assumptions of the Lean statements and exact certifiers;
- the role and stability of schema-3 scientific outcome records;
- a green exact-head hosted CI result, bound to revision and run URL;
- whether a neutral row-transform-tail API name can supplement the p-value compatibility name;
- whether a future group-aware schedule can preserve episode boundaries by construction;
- how fitted-quantizer metadata will record same-row, disjoint-row, and unknown row relations
  without inferring separation from domain hashes;
- correction of the kd-tree scope, same-row categorical-helper taxonomy, and `I_min` fallback
  language at the retained pin;
- whether `pid-runlog-replay` can treat a closed stdout pipe as normal termination instead of
  panicking while it prints the multi-line replay summary;
- a statement that Prisoma compatibility and VLA application validity remain consumer-owned.

## Ready-to-send message

**Title:** Prisoma consumer-delta request for `pid-rs@796c11e..7473e62`

Prisoma opened a first-principles estimator adoption review on 2026-08-12 and refreshed it through
2026-08-13. It remains pinned to
`796c11e70f009634b853dc4ada6f565563d82f51`. Public main was observed at
`7473e62acef6077c2c1147e09d5d1297f2a2874b`; the latest reviewed estimator-code anchor is its
parent `cb3f58f0b190454cb3f1090de8798261ec78f194`.

We tested Prisoma's clean Rust workspace against exact revision `722d3abe` in an isolated tree.
Head `bbdfda40` has byte-identical crates, Cargo files, Rust toolchain, and Python package metadata.
Its one later commit after `722d3abe` changes only upstream assurance, workflow, script, and prose
surfaces. The
interval after our earlier `00fce70d` check changes assurance artifacts, schemas, executable
verifier scripts, prose, and tests. It touches three `pid-core` Rust files only in documentation or
comments. It changes no Cargo manifest, public Rust signature, or executable Rust statement in
that later interval.
`cargo +1.93.0 check --locked --workspace --all-features` and
`cargo +1.93.0 test --locked --workspace --all-features --no-run` passed. The actual all-feature
workspace tests also passed.

We are not asking upstream to claim Prisoma compatibility or to recommend an update. We need a
compact consumer delta for this exact revision interval. Please confirm the Rust and Python rename
map, serialized report and status changes, required consumer replay fixtures, categorical-only
formal and exact-certifier scope, continuous KSG and shared-exclusions scope, schema-3 stability,
and abstention semantics.

Prisoma now names its active fitted categorical route `categorical-sx`. It uses the pinned
two-source MGW empirical-law backend and records quantizer plus transform receipts. It does not use
`I_min` or BROJA to define an active hypothesis. Please keep categorical MGW, continuous Ehrlich,
finite-sample estimators, and infomorphic objectives as typed non-substitutable objects.
Prisoma does not ask for one generic “Wibral PID.” It needs the MGW categorical and Ehrlich
continuous objects under separate contracts, with the goal-function, parthood, and infomorphic
work recorded at their correct composition or semantic layer. Public `pid-rs` may retain its
existing method IDs, but each must bind the complete defining team and exact reference.

Please also correct three retained-pin metadata boundaries. Fitted-quantizer types currently claim
outside/separate fitting that the API does not enforce. The kd-tree documentation includes the
Ehrlich redundancy term even though that term uses its own quadratic scan. Default-off helpers call
quantize-then-MGW results continuous and recommend `I_min` when continuous kNN fails. We suggest
typed row-relation receipts, exact backend-specific wording, and separate cross-estimand language.

For future work, first split paper-defined functional identity from runtime estimator identity in
the catalog. Then add a bounded sparse empirical-count-law MGW API. Please bridge it to the
existing exact-count certifier. Add a separate specified-rational-law schema next. After bounded
agreement checks, add a declared binary64 finite-law evaluator. Supporting work should add
fit/evaluation row receipts, checked resource composition, fixed-source-law invariance tests, and
group-aware schedules. Please do not treat the current count-law certifier as assurance for soft
neural laws, adaptive binning, stopped gradients, or training guards.

The pinned generic resampling surface already withholds a bootstrap summary or transform tail
fraction after any requested replicate fails. It also types circular-shift output as an approximate
surrogate score. Prisoma now enforces these properties again when it publishes an uncertainty
sidecar. As follow-up API design, please consider a neutral companion name for the generic
`permutation_rows_pvalue_*` family and a group-aware schedule that cannot splice groups. These
are semantic and ergonomic requests, not claims of an estimator defect.

We also reproduced a CLI robustness defect at the retained pin. Piping a long replay summary to
an early-closing consumer such as `grep -q` can panic on `BrokenPipe` and exit 101. Prisoma now
drains replay output in its recipes and CI. Please make `pid-runlog-replay` treat a closed stdout
pipe as normal termination, and add a regression that closes the reader after the first match.

Estimator-code anchor `cb3f58f0` adds one further bounded KSG integration commit. We inspected its three
changed production Rust files. We also replayed its four predecessor-radius fixtures and structured coordinate-
span overflow fixture on exact current-head bytes. All five passed. Upstream still labels the
broader KSG revision-4 repository integration NO-GO. Please preserve that boundary and provide
exact-head hosted CI evidence before recommending adoption.

Exact-head CI run `31686107959` is currently red because the certified-SxPID2 job detects a stale
reviewed workflow digest. Exact-head CodeQL run `31686106737` passed. Please repair and rerun the
failed exact-head gate without weakening its custody rule.

Current public head `7473e62` is a custody-only direct child of that estimator anchor. It changes
no crate or Cargo input and grants no estimator or scientific credit. Its full CI run
`31724449805` still failed the exact-count SxPID2 and KSG arithmetic/phase-isolation jobs. Its
separate `Push on main` run `31724449083` passed. Please provide a green full exact-head run before
recommending adoption.

The atom split into `SxPointwiseAtom` and `SxAveragedAtom`, the `prob` replacement with
`empirical_count` and `empirical_probability`, and the interpretation envelopes are especially
relevant. The pin-to-head interval also adds `pid-core` build dependencies on `serde`,
`serde_json`, and `sha2`; adds `serde_json` to `pid-python`; revises resampling identities; and adds
an experimental schema-3 type foundation. Run `31651702557` completed with all 45 jobs passing
for `bbdfda40`. CodeQL run `31651702504` also passed for that earlier head. These are not exact-
head receipts for `7473e62`.

Prisoma will keep population, measure, estimator, and application gates separate. Upstream
categorical assurance will not be treated as continuous VLA validation. A failed continuous term
will not route to the categorical MGW path or `I_min`, and an abstention will not receive a numeric
placeholder.

## Prisoma follow-up

After upstream answers or publishes the exact delta, Prisoma should open a dedicated pin-update
change. That change must replay the acceptance matrix above, update generated capability bindings,
and preserve every scientific stop rule. It must not be folded into an unrelated documentation or
release commit.
