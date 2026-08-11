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
  the active docset and machine-readable claim registry consistent with it.
- `pid-core`, `pid-python`, and `pid-runlog` are owned by the `pid-rs`
  repository. Change them upstream, release them there, and then update the
  Prisoma submodule pin with migration evidence.
- The run log is the source of truth, the Agent Bridge is the only control
  plane, and Rerun/Tauri/SparkJS roles follow `grandplan.md` section 16.
- Do not add real participant, robot, customer, restricted, or secret data to
  the repository. Synthetic fixtures must identify themselves as synthetic.

## Scientific claim control

A documentation edit cannot change scientific status. Any proposed promotion
of M0, EC1, H1-A, H1-B, H2, H3, or H4 must update the canonical specification,
`protocols/research_claim_registry_v1.json`, relevant generated capability
views, and content-bound evidence in the same reviewed change. The corresponding
independent or accountable-human review must actually exist; names, signatures,
holdout custody, access history, ethics review, and data/model rights must never
be inferred or fabricated.

PID interpretation additionally requires separate population, measure,
estimator, and application gates. Never pool continuous shared-exclusions atoms
with discrete Williams–Beer `I_min`, route a failed continuous term to a
different estimand, or emit a numeric placeholder for an abstention.

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
Prisoma use can keep the smaller default environment. `just check` verifies the lean default Rust
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
