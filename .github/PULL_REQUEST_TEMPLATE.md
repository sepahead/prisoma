# Pull request checklist

See [CONTRIBUTING.md](https://github.com/sepahead/prisoma/blob/main/CONTRIBUTING.md) for the full contribution policy.

## Summary

Describe the change and its purpose in one short paragraph.

## Required checks

- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo clippy --locked --workspace -- -D warnings` passes.
- [ ] `cargo test --locked --workspace` passes.
- [ ] `python scripts/audit_docset_claims.py --all-tracked-markdown` passes.
- [ ] `python scripts/audit_grandplan.py` passes.
- [ ] `python scripts/audit_research_governance.py` passes.
- [ ] `python scripts/generate_capability_matrix.py --check` passes.

## Claim control

- [ ] No new capability claims without in-repo evidence.
- [ ] No edits to immutable `release/0.9.0` review or requirements intake.
- [ ] No hand edits to generated files (`docs/CAPABILITY_MATRIX.md`,
      `THIRD_PARTY_NOTICES.generated.md`,
      `protocols/capability_matrix_current_v1.json`).
- [ ] No AI co-author trailers.
