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

The observer speaks NCP wire 1.0 at the unreleased, release-blocked `1.0.0-rc.1` candidate.
It pins the candidate's exact commit `2819dae3b6338bb1df6d105ebb5b7433936a993d` (compact proto contract hash `163acc57d8a62b66`).
Wire 0.8 is retired; its last pin, `v0.8.0`, remains in the history.
In NCP's own ledger, tasks P01, P02, and P03 remain OPEN, not dependency-ready, and **NOT RUN**: the repin is local implementation work, not qualification.
P03 includes Prisoma observer-role qualification.
The [task ledger at the pinned commit](https://github.com/sepahead/NCP/blob/2819dae3b6338bb1df6d105ebb5b7433936a993d/evidence/implementation/task-ledger.v1.json) preserves that boundary.
These identities do not describe the separate native local capture package.
