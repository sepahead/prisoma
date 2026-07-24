# Contributing to Prisoma

Prisoma welcomes small, reviewable changes that preserve its scientific and
provenance boundaries. By submitting a contribution, you agree that it may be
distributed under the repository's dual `MIT OR Apache-2.0` license.

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
maturin develop --manifest-path pid-rs/crates/pid-python/Cargo.toml
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
cargo fmt --all -- --check
cargo clippy --locked --workspace -- -D warnings
cargo test --locked --workspace
python scripts/audit_docset_claims.py --all-tracked-markdown
python scripts/audit_grandplan.py
```

Also run the checks appropriate to the files changed:

```bash
python -m pytest tests/python -q
ruff check .
python scripts/generate_third_party_notices.py --check
cargo test --locked --manifest-path crates/ncp-observer/Cargo.toml
cargo deny --locked check
cargo deny --locked --manifest-path crates/ncp-observer/Cargo.toml check
```

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
