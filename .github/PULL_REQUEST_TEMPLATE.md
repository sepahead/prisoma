# Pull request checklist

See [CONTRIBUTING.md](https://github.com/sepahead/prisoma/blob/main/CONTRIBUTING.md) for the full contribution policy.

## Summary

Describe the change and its purpose in one short paragraph.

## Required checks

- [ ] `just check` passes the locked Rust, Python, formatting, notice, and truth-audit gates.
- [ ] If NCP or its lock changed, its explicit test and `cargo deny` checks pass.
- [ ] If `formal/`, its registry, or its runner changed, run `just formal` with Z3 4.16.0.

## Claim control

- [ ] No new capability claims without in-repo evidence.
- [ ] No edits to immutable `release/0.9.0` review or requirements intake.
- [ ] No hand edits to generated files (`docs/CAPABILITY_MATRIX.md`,
      `THIRD_PARTY_NOTICES.generated.md`,
      `protocols/capability_matrix_current_v1.json`).
- [ ] Predictive-policy claims name the deployed graph. Causal and planning terms satisfy
      `grandplan.md` section 9.2, "World-model experiments."
- [ ] Any H3 source binds a target-specific prediction landmark before target availability and
      proves source ancestry, target exclusion, and matched-proposal access.
- [ ] No AI co-author trailers.
