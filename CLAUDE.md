# Claude instructions for Prisoma

[AGENTS.md](AGENTS.md) is the canonical operating contract.
Read it, [README.md](README.md), and the document that owns the requested change.
The research specification is [grandplan.md](grandplan.md), docset v13.0.

## Working boundary

Prisoma develops experiments for action-conditioned world models and embodied decisions.
The exact-fork affine reference and one-input LeWM CPU/MPS engineering path have distinct scopes.
Neither closes M2, W1, W2, W3, or the PID application gates.

Preserve the `pid-rs` submodule, its pin, sources, index, branches, and worktrees during ecosystem work.
Use only the pinned public consumer APIs. Do not copy or revise estimator code here.
Keep the Agent Bridge as the canonical experiment mutation plane.
Observers, analysis, and Rerun have no command authority.

Follow the ASD-STE100 Issue 9 writing policy in AGENTS.md.
Keep assumptions, units, missingness, exact identities, and negative results visible.
Do not add AI co-author trailers or generated-by markers to commits.

## Checks

```bash
uv sync --locked --group ui
just check
```

Use the applicable optional gates listed in AGENTS.md.
Regenerate source projections through their owning generators without promoting scientific statuses.

## Preserved legacy compatibility

The legacy observer remains pinned to `v0.8.0` and wire 0.8.
The separate August 13, 2026 review observed NCP at `1a04294c90c1b50eba06ae1c6afe9c951319250d`.
That source is the unreleased, release-blocked `1.0.0-rc.1` candidate, with compact proto contract hash `163acc57d8a62b66`.
Its P01, P02, and P03 tasks remain OPEN, not dependency-ready, and **NOT RUN** in the retained review.
P03 includes Prisoma observer-role qualification.
The [dated task ledger](https://github.com/sepahead/NCP/blob/1a04294c90c1b50eba06ae1c6afe9c951319250d/evidence/implementation/task-ledger.v1.json) preserves that boundary.
These identities do not describe the separate native local capture package.
