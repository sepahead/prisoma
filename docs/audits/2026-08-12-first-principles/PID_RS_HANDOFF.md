# `pid-rs` handoff from the Prisoma first-principles audit

Review window: 2026-08-12 through 2026-08-13.

Prisoma pin retained: `796c11e70f009634b853dc4ada6f565563d82f51`.

Public `pid-rs` main reviewed: `bbdfda40f0a49a2260b10eafdcb438fc61ae94e9`.

Prisoma exact-revision consumer test: `722d3abeb922fc4119ecb9f92d7fedca096c9f77`.

No submodule update was made. This file is both the internal adoption brief and a message that can
be sent to the `pid-rs` repository.

## Executive decision

Do not update the Prisoma pin yet.

The current upstream tree is materially stronger than the pin. It has method-identity catalogs,
clearer categorical atom types, exact categorical certification, formal assurance, KSG arithmetic
work, support-change records, software identity, source-errata records, and a proposed scientific
outcome schema. Public main is 95 commits ahead of the Prisoma pin.

An isolated clean Prisoma consumer build and all-feature test run against exact revision
`722d3abe` passed. The interval after the earlier `00fce70d` check changes assurance artifacts,
schemas, executable verifier scripts, prose, and tests. It touches three `pid-core` Rust source
files only in documentation or comments. It changes no Cargo manifest, public Rust signature, or
executable Rust statement in that later interval. The exact-revision run establishes compatibility for
Prisoma's current compiled and tested Rust consumer surface. It does not prove report-schema
compatibility, untested behavioral equivalence, Python compatibility, or scientific added value.

Current public head `bbdfda40` is one later assurance commit. The `722d3abe..bbdfda40` delta
changes workflows, assurance records, formal replay documents, and Python verifier scripts. It
does not change `crates/`, any Cargo manifest or lock, `rust-toolchain.toml`, or `pyproject.toml`.
The consumer test therefore covers the exact code and dependency bytes that Prisoma consumes at
the observed head. It does not validate the changed upstream assurance scripts.

The full pin-to-head interval does change package metadata. `pid-core` adds build dependencies on
`serde`, `serde_json`, and `sha2`. `pid-python` adds `serde_json`, and `Cargo.lock` changes. The new
`pid-core` build path creates software-identity data. A later adoption review must therefore cover
source-package builds, cache behavior, release archives, and dependency policy. The clean consumer
run proves that this exact dependency graph resolves and compiles. It does not prove every package
or distribution route.

The [current-head CI run](https://github.com/sepahead/pid-rs/actions/runs/31651702557)
completed successfully on 2026-08-13. All 45 jobs passed. The
[current-head CodeQL run](https://github.com/sepahead/pid-rs/actions/runs/31651702504) also passed.
These provider results satisfy the upstream-CI check. They do not replace the consumer-owned
compatibility, schema, package, and scientific-value review required before a pin change.

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

Current main `bbdfda40f0a49a2260b10eafdcb438fc61ae94e9` is 95 commits beyond the Prisoma pin.
The final upstream commit removes post-commit path authority from the source-state verifier. Since `00fce70d`, executable
Python verifier scripts and their tests changed. Three `pid-core` Rust source files changed only in
documentation or comments. This audit inspected that exact delta and then tested exact current
consumed code at `722d3abe`. It does not classify verifier-script changes as compiled consumer
behavior.

At the final 2026-08-13 observation, all 45 current-head hosted CI jobs had passed. The
current-head CodeQL workflow also passed. This closes the provider-CI row only.

## Concrete migration surface

The following delta is directly relevant to Prisoma.

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

Two useful upstream extensions remain. First, add a neutral alias for the generic
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
| Upstream current-head CI | Run `31651702557` completed with all 45 jobs passing; CodeQL run `31651702504` passed | Closed for observed revision `bbdfda40`; rerun for any later revision. |
| Prisoma application value | Not established | Show a reviewed improvement that closes a named Prisoma obligation. |

## Scientific boundary

Prisoma retains four independent PID gates:

1. population support;
2. PID measure;
3. estimator validity and uncertainty;
4. VLA application validity.

Upstream work can materially improve gate 3. Categorical theorems and exact certifiers do not pass
continuous gate 3. No upstream software result passes gates 1, 2, or 4 for Prisoma.

An abstained estimate must still have no numeric placeholder. A failed continuous term must never
auto-route to discrete `I_min`. Continuous shared exclusions, categorical shared exclusions, and
Williams–Beer `I_min` must remain separately named and separately interpreted.

## What to ask upstream

`pid-rs` already has `MIGRATION.md` and release-scope ledgers. The useful missing artifact is not a
second generic migration guide. Prisoma needs a compact consumer delta for the exact interval
`796c11e..bbdfda40`.

Please provide or confirm:

- the Rust and Python symbol rename map;
- the serialized-field and status-contract delta;
- the minimum replay fixture set for a downstream consumer;
- which changes are intentional breaks before 1.0;
- which guarantees are categorical only;
- which continuous guarantees cover KSG MI versus KSG-backed shared exclusions;
- the exact assumptions of the Lean statements and exact certifiers;
- the role and stability of schema-3 scientific outcome records;
- the final current-head hosted CI result, bound to revision and run URL;
- whether a neutral row-transform-tail API name can supplement the p-value compatibility name;
- whether a future group-aware schedule can preserve episode boundaries by construction;
- whether `pid-runlog-replay` can treat a closed stdout pipe as normal termination instead of
  panicking while it prints the multi-line replay summary;
- a statement that Prisoma compatibility and VLA application validity remain consumer-owned.

## Ready-to-send message

**Title:** Prisoma consumer-delta request for `pid-rs@796c11e..bbdfda40`

Prisoma opened a first-principles estimator adoption review on 2026-08-12 and refreshed it through
2026-08-13. It remains pinned to
`796c11e70f009634b853dc4ada6f565563d82f51` and reviewed public main at
`bbdfda40f0a49a2260b10eafdcb438fc61ae94e9`.

We tested Prisoma's clean Rust workspace against exact revision `722d3abe` in an isolated tree.
Current head `bbdfda40` has byte-identical crates, Cargo files, Rust toolchain, and Python package
metadata. Its one later commit changes only upstream assurance, workflow, script, and prose
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

The pinned generic resampling surface already withholds a bootstrap summary or transform tail
fraction after any requested replicate fails. It also types circular-shift output as an approximate
surrogate score. Prisoma now enforces these properties again when it publishes an uncertainty
sidecar. As follow-up API design, please consider a neutral alias for the generic
`permutation_rows_pvalue_*` family and an episode-aware schedule that cannot splice groups. These
are semantic and ergonomic requests, not claims of an estimator defect.

We also reproduced a CLI robustness defect at the retained pin. Piping a long replay summary to
an early-closing consumer such as `grep -q` can panic on `BrokenPipe` and exit 101. Prisoma now
drains replay output in its recipes and CI. Please make `pid-runlog-replay` treat a closed stdout
pipe as normal termination, and add a regression that closes the reader after the first match.

The atom split into `SxPointwiseAtom` and `SxAveragedAtom`, the `prob` replacement with
`empirical_count` and `empirical_probability`, and the interpretation envelopes are especially
relevant. The pin-to-head interval also adds `pid-core` build dependencies on `serde`,
`serde_json`, and `sha2`; adds `serde_json` to `pid-python`; revises resampling identities; and adds
an experimental schema-3 type foundation. Current-head run `31651702557` completed with all 45
jobs passing. Current-head CodeQL run `31651702504` also passed. The run links are in this handoff.

Prisoma will keep population, measure, estimator, and application gates separate. Upstream
categorical assurance will not be treated as continuous VLA validation. A failed continuous term
will not route to discrete `I_min`, and an abstention will not receive a numeric placeholder.

## Prisoma follow-up

After upstream answers or publishes the exact delta, Prisoma should open a dedicated pin-update
change. That change must replay the acceptance matrix above, update generated capability bindings,
and preserve every scientific stop rule. It must not be folded into an unrelated documentation or
release commit.
