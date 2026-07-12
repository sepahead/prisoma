# meshmaker — RELOCATED (no longer in this repo)

The `meshmaker/` tooling and its generated assets are **no longer part of the
prisoma tree**. They were cost-bearing, external 3D-asset-generation tooling (paid
cloud APIs, batch/swarm launchers, prompts) and ~35 GB of generated mesh output —
unrelated to the prisoma scientific core (estimators, run log, bridge, sim,
harnesses, experiments).

## Where it went

Moved on 2026-07-10 to the **`relief-atlas`** repository, alongside the other
atlas asset tooling:

```
../relief-atlas/meshmaker/
```

(`cobot-atlas` already holds its own published share of the mesh assets; the
remaining generation tooling and outputs were consolidated into `relief-atlas`.)

The move was an on-disk relocation of the working-tree files only — nothing was
regenerated and no generated data was lost. Credentials (`api_keys.txt`) and the
large `output*/` directories are git-ignored in `relief-atlas` and must never be
committed.

## History

Prior to this move, `meshmaker/` was *quarantined* out of prisoma version control
(only this tombstone was tracked; see `grandplan.md` §A.8 and the git history of
this file for the original quarantine rationale). Historical references to
`meshmaker/` elsewhere in this repo (CHANGELOG, audits, grandplan) are retained as
records and are not rewritten.
