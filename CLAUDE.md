# CLAUDE.md — prisoma

**`AGENTS.md` is the source of truth for how to work in this repo.** Read it first;
this file restates the highest-leverage rules and adds Claude-Code-specific notes.

## What prisoma is

A gate-driven research toolkit for diagnosing Vision-Language-Action (VLA) policies
with Partial Information Decomposition (`I^sx_∩`). The canonical spec is `grandplan.md`;
`README.md` is the entry point. The Rust PID estimators live **upstream** in the
[`pid-rs`](https://github.com/sepahead/pid-rs) submodule (`pid-core`, `pid-runlog`,
`pid-python`) — **not** vendored here. Edit the estimator core upstream, then bump the
submodule; never re-add copies to this repo.

## The rules you cannot get wrong

1. **Gate discipline.** Do not interpret PID atoms on real embeddings until the Exp0 +
   geometry gates pass. Exp0 currently reports **PIVOT** on synthetic high-d controls
   (`findings.md`) — that is the gate working. One (PID measure, preprocessing, estimator
   config) tuple = one preregistered regime; never pool continuous `I^sx_∩` atoms with
   discrete `I_min` atoms.
2. **Honesty over roadmap.** Do not claim non-existent crates/scripts/assets are runnable.
   Avoid hard-coded performance/cost claims unless backed by a committed source or a clearly
   labelled in-repo measurement — the doc-audit scripts (`scripts/audit_*.py`) enforce this.
   Keep the docset version stamps consistent across `README.md`/`AGENTS.md`/`grandplan.md`/etc.
3. **Run log = source of truth; Agent Bridge = only control plane.** Every captured sample
   must be reconstructable from canonical run-log events. The observer/harness drive nothing.

## Before you open a PR / commit

```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
python scripts/audit_docset_claims.py           # + audit_grandplan.py --check-italic-titles
```

The estimator gate: `just exp0-bin` (prints the GO/PIVOT/NO-GO verdict) — or the `cargo`
equivalents in `AGENTS.md`. `just test` / `just docs-audit` wrap the above.

## Claude-specific

- **No AI co-authors.** No `Co-Authored-By:` trailer, no "Generated with Claude Code" line,
  no 🤖 marker in any commit or PR. (Global rule; restated here.)
- **pid-rs is a submodule.** After cloning, `git submodule update --init`. Estimator
  binaries run via `--manifest-path pid-rs/crates/pid-core/Cargo.toml`.
- **ncp-observer is workspace-excluded.** It git-depends on the published NCP repo (tag pin)
  and pulls Zenoh, so build it with `--manifest-path crates/ncp-observer/Cargo.toml`, never
  `-p` from the repo root. It is an optional, exploratory-only `(V,L,D,A)` source — not on
  the M5 critical path (which is `experiments/safe_adapter`).
- **NCP is a sibling repo** (`../NCP`), pinned by tag. If you change the NCP wire, this
  repo's `ncp-observer` must be re-pinned and re-verified; see `../NCP/AGENTS.md`.
