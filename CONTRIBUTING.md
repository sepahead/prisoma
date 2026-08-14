# Contributing to Prisoma

Prisoma welcomes small, reviewable changes that preserve its scientific and
provenance boundaries. By submitting a contribution, you agree that it may be
distributed under the repository's dual `MIT OR Apache-2.0` license.
Participation follows the [code of conduct](CODE_OF_CONDUCT.md).

## Set up the repository

```bash
git clone --recurse-submodules https://github.com/sepahead/prisoma.git
cd prisoma
git submodule update --init
uv sync --locked
cargo test --locked --workspace
```

The root is a source/research project rather than a PyPI package. Build the
canonical Python estimator binding from the pinned upstream workspace:

```bash
uv run --no-sync maturin develop --locked --manifest-path pid-rs/crates/pid-python/Cargo.toml
```

## Ownership and scope

- `grandplan.md` is the canonical research and engineering specification. Keep
  the active docset and machine-readable claim-template registry consistent with it.
- `pid-core`, `pid-python`, and `pid-runlog` are owned by the `pid-rs`
  repository. Change them upstream, release them there, and then update the
  Prisoma submodule pin with migration evidence.
- The run log governs accepted recorded events. It cannot prove an unseen upstream event.
  The Agent Bridge is the only control plane. Rerun/Tauri/SparkJS roles follow
  `grandplan.md` section 16.
- Do not add real participant, robot, customer, restricted, or secret data to
  the repository. Synthetic fixtures must identify themselves as synthetic.

## Scientific claim control

A documentation edit cannot change scientific status. Any proposed promotion
of W1, W2, or W3 must update the canonical specification and
`protocols/world_model_claim_registry_v1.json`. Any proposed promotion of M0, EC1, H1-A, H1-B,
H2, H3, or H4 must update `protocols/research_claim_registry_v1.json`. Each promotion must update
the relevant generated capability
views, and content-bound evidence in the same reviewed change. The corresponding
independent or accountable-human review must actually exist; names, signatures,
holdout custody, access history, ethics review, and data/model rights must never
be inferred or fabricated.

Every H1 result must name H1-A or H1-B. For H2, keep a complete-data proper score, an IPCW
complete-data risk estimator, and a proper observed-data score distinct. Freeze one contract that
binds the prediction object, score, target risk, censoring construction, assumptions, margin, and
uncertainty method. A right-censored likelihood requires the full event-time-and-type law. The
revised M0 v3 draft is unreviewed and all-null. The superseded v2 bytes remain historical.
In a future candidate, populate only EC1, H2, the selected H1 protocol, and the selected H3/H4
branch. Leave every inactive protocol slot null. A post-H3 switch to H4 requires a fresh untouched
sample and the frozen sequential-error rule.

PID interpretation additionally requires separate population, measure, estimator, and application
gates. Declare the object kind, domain, defining reference, estimand, estimator, units, and
composition. MGW categorical shared exclusions, Ehrlich continuous shared exclusions,
finite-sample estimators, Williams–Beer `I_min`, and infomorphic objectives are related but
non-substitutable objects. Never pool them, auto-route between them, or emit a numeric placeholder
for an abstention. Do not claim a cross-domain result without an explicit mapping theorem.

Classify every predictive policy by its deployed computation graph. Keep these cases distinct:

- a predictive target used only during training;
- an intended future generated before the action;
- a coupled joint future-action sampler without a clamped action query;
- an observational predictor conditioned on a proposed action; and
- a planner that proposes, predicts, scores, and selects at least two actions.

Do not infer causal dynamics from directed attention, action conditioning, video quality, or task
success. Causal action-consequence language requires randomized executed actions, execution
receipts, support checks, proper scores, and calibration. Planning language requires recorded
candidates, predictions, scores, selection, and a decision-flip test. Follow `grandplan.md`
section 9.2, "World-model experiments."

For any H3 source, freeze a target-specific prediction landmark before the target becomes
available. Bind each source's full tensor ancestry to that landmark. Reject a source that contains
its target or a descendant of that target. If a source consumes a candidate action and the target
is downstream, give the matched baseline the exact same proposal.

## Technical writing

Use the ASD-STE100 Issue 9 policy in `AGENTS.md` for project-owned technical prose.
Keep the scientific meaning exact. Do not rewrite literals, equations, licenses, immutable intake,
generated files, vendored files, or submodule documentation to satisfy a style preference.

## Required checks

Run before every commit or pull request:

```bash
uv sync --locked --group ui
just check
```

The `ui` group is required only because the full suite tests the optional PNG utility. Ordinary
Prisoma use can keep the smaller default environment. `just check` verifies the low-overhead default Rust
surface. It excludes protocol references, legacy sensitivity, analysis, WebSocket, Rapier, and
Rerun features. The gate then runs all-target, all-feature Clippy, tests, and rustdoc. It also runs
the Python suite, Ruff, notice drift checks, and the pre-commit offline truth audits.

The candidate release audit is commit-bound. Run `just release-candidate-audit` after the source
commit and candidate regeneration. CI runs this audit separately. Do not make an ordinary
pre-commit gate depend on evidence for a commit that does not yet exist.

For focused Python iteration, run:

```bash
uv run --no-sync pytest tests/python -q
uv run --no-sync ruff check .
uv run --no-sync ruff format --check .
```

Also run the checks appropriate to the files changed:

```bash
cargo test --locked --manifest-path crates/ncp-observer/Cargo.toml
cargo deny --locked check
cargo deny --locked --manifest-path crates/ncp-observer/Cargo.toml check
just formal
```

Run the NCP checks when its excluded crate or lock changes. Run both advisory checks when a Rust
manifest, lock, or policy changes. Run `just formal` when a formal model, its registry, or its
runner changes. It requires exact Z3 4.16.0.

Add positive, malformed/negative, boundary/resource, replay/timing/leakage, and
independent or property-based cases where applicable. Record exact commands and
exit statuses for release-affecting work.

## Changes and authorship

- Keep commits focused and use professional, descriptive messages.
- Preserve unrelated work in a dirty tree and coordinate changes to shared
  locks, submodule pins, schemas, generated files, and claim registries.
- Regenerate committed outputs with their checked generator; do not hand-edit
  generated views.
- Do not add an AI system, coding assistant, or agent as an author or commit/PR
  co-author. Do not add automated-generation markers to commit messages.
- Report vulnerabilities privately as described in [SECURITY.md](SECURITY.md).
