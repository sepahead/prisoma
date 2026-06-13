# meshmaker — QUARANTINED (not part of the canonical PID-VLA repo)

This directory held cost-bearing, external asset-generation tooling (3D mesh /
asset generation via paid cloud APIs, batch/swarm launchers, and associated
prompts). It is **not** part of the PID-VLA scientific core — the estimators, run
log, bridge, sim, harnesses, and experiments — and has been **quarantined out of
version control**.

## What changed

- All `meshmaker/` scripts were removed from git tracking with `git rm --cached`
  (the working-tree files are **kept on disk**; nothing local was deleted).
- `.gitignore` now ignores everything under `meshmaker/` except this tombstone, so
  a fresh clone of the canonical repo does not contain the tooling, its prompts, or
  any generated output.
- `meshmaker/api_keys.txt` was already untracked and ignored; it must live outside
  the repository tree before any release (see the release checklist below).

## Why

The whole-repo review (`../REVIEW_AND_TODO.md`, Security/Governance perspective +
P0 item 3) flagged this tooling as:

- cost-bearing (paid generation APIs; a swarm launcher that can spawn many parallel
  cloud jobs);
- a secret-handling risk (`api_keys.txt` in the working tree);
- containing asset prompts unrelated to — and potentially distracting from — the
  PID-VLA diagnostics;

and recommended isolating it from the canonical project and from all lint / test /
release claims. (`grandplan.md` §A.8 already records that meshmaker is not on the
10-step critical path.)

## If you need it

The files are still on your disk under `meshmaker/` (just untracked). To develop it
further, move it to its own repository, e.g.:

```bash
cp -r meshmaker ../meshmaker-standalone && (cd ../meshmaker-standalone && git init)
```

To recover the previously tracked versions from history:

```bash
git log --oneline -- meshmaker/        # find a commit before the quarantine
git checkout <commit> -- meshmaker/    # restore tracked files into the working tree
```

## Release checklist (meshmaker)

- [ ] `meshmaker/` is absent from the released source tree / wheels / app bundles.
- [ ] `meshmaker/api_keys.txt` (and any credentials) live outside the repo tree.
- [ ] No generated assets, prompts, or logs from `meshmaker/` ship in a release.
