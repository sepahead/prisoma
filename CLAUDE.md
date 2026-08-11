# CLAUDE.md — Prisoma

**`AGENTS.md` is the source of truth for how to work in this repo.** Read it first; this
file restates the highest-leverage rules and adds Claude-Code-specific notes.

## What Prisoma is

A gate-driven research toolkit providing **auditable experiment semantics** for
intervention-grounded diagnosis of Vision-Language-Action (VLA) policies. Partial Information
Decomposition (shared-exclusions `I^sx_∩`) is one **conditional** candidate diagnostic — central
only if its measure, estimator, application regime, and incremental value pass preregistered
gates — not the thesis premise. The canonical spec is `grandplan.md` (**docset v12.5**);
`README.md` is the entry point. The Rust PID estimators live **upstream** in the
[`pid-rs`](https://github.com/sepahead/pid-rs) submodule (`pid-core`, `pid-runlog`, `pid-python`)
— **not** vendored here. Edit the estimator core upstream, then bump the submodule; never re-add
copies to this repo.

## Technical writing

Use [ASD-STE100 Issue 9](https://www.asd-ste100.org/assets/files/ASD-STE100_ISSUE9.pdf)
for project-owned technical prose. Call the result **STE-aligned**, not certified.

- Use American English and one consistent term for each concept.
- Keep descriptive sentences to 25 words or fewer.
- Keep procedural sentences to 20 words or fewer.
- Use active voice, simple tenses, and direct imperative steps.
- Put each condition before its action. Give one instruction per step.
- Do not use contractions or semicolons in technical prose.
- Keep one topic in each paragraph and no more than six sentences.
- Preserve exact domain, API, command, identifier, and mathematical terms.
- Verify every command, link, version, capability, and scientific-status statement.

Accuracy and fail-closed meaning take priority over vocabulary limits. Exempt code, commands,
paths, identifiers, literals, equations, tables, exact quotations, licenses, and historical records.
Do not rewrite immutable intake, generated files, vendored files, or submodule documentation.
Follow the complete policy and exception list in `AGENTS.md`.

## The rules you cannot get wrong

1. **Gate discipline.** Do not interpret PID atoms on real embeddings. PID validity is split
   into **four separate gates** — population, measure, estimator, application (`grandplan.md`
   §7.1). The high-d MI/coherence path is **NO-GO**; continuous shared-exclusions atoms on real
   VLA embeddings remain **BLOCKED / NOT APPLICATION-VALIDATED** (`grandplan.md` §3.2, §7.2)
   because default Experiment 0 reports atom-measure validation as `not_adjudicated` and
   atom-estimator validation as `blocked`, while the strict band gates analytic MI rather than
   atoms (`findings.md`). It never compares shared-exclusions redundancy with a zero target.
   Sampled-mean δ is descriptive, not a validity gate. One
   (PID measure, preprocessing, estimator config) tuple = one preregistered regime; never pool
   continuous `I^sx_∩` atoms with discrete `I_min` atoms — `--pid-mode discrete` is Williams–Beer
   `I_min`, **not** discrete `i^sx_∩` (`grandplan.md` §7.6). Confirmatory claims are bound by the
   §4 claim registry (EC1, H1–H4), the §3.8 PID kill rules, and the §6 statistical analysis plan.
2. **Honesty over roadmap.** Do not claim non-existent crates/scripts/assets are runnable.
   Avoid hard-coded performance/cost claims unless backed by a committed source or a clearly
   labelled in-repo measurement — the doc-audit scripts (`scripts/audit_*.py`) enforce this.
   Keep the docset version stamps consistent across `README.md` / `AGENTS.md` /
   `grandplan.md` / `DIAGRAMS.md` / `findings.md` (all **v12.5**).
3. **Run log = source of truth; Agent Bridge = only control plane.** Every captured sample
   must be reconstructable from canonical run-log events. Observers and harnesses drive
   nothing.

## Before you open a PR / commit

```bash
uv sync --locked --group ui
just check
```

The estimator gate: `just exp0-bin` (prints the GO/PIVOT/NO-GO verdict) — or the `cargo`
equivalents in `AGENTS.md`. `just test`, `just python-test`, and `just docs-audit` provide focused
subsets of the required local gate.

## Claude-specific

- **No AI co-authors.** No `Co-Authored-By:` trailer, no "Generated with Claude Code" line,
  no 🤖 marker in any commit or PR. (Global rule; restated here.)
- **pid-rs is a submodule.** After cloning, `git submodule update --init`. Estimator
  binaries run via `--manifest-path pid-rs/crates/pid-core/Cargo.toml`.
- **ncp-observer is workspace-excluded.** It git-depends on the published NCP repo (tag pin,
  currently `v0.8.0`, wire 0.8) and pulls Zenoh, so build it with
  `--manifest-path crates/ncp-observer/Cargo.toml`, never `-p` from the repo root. It is an
  optional, exploratory-only, **read-only** `(V,L,D,A)` source (E2 edge, `grandplan.md` §8.9) —
  part of the M2 ecosystem-conformance benchmark, not a critical-path dependency. The reference
  adapter for the confirmatory H-experiments is `experiments/safe_adapter`; the core must build
  with NCP disabled and H1/H2 must run without requesting PID atoms (dependency firebreak,
  §8.9.3).
- **NCP is a pinned git dependency**, currently the latest immutable release `v0.8.0` (wire
  0.8); no sibling checkout is required. Keep this legacy consumer frozen. A different wire
  requires a separate consumer surface, corpus, and qualification path. Official NCP main was
  observed at `1ffd3bf9a6c52d0279eb31a56e0664e4eec24d68` on 2026-08-11. That commit is the
  unreleased, release-blocked `1.0.0-rc.1` candidate (wire 1.0; compact proto contract hash
  `163acc57d8a62b66`). NCP ledger tasks `P01`, `P02`, and `P03` are OPEN, not dependency-ready,
  and **NOT RUN**. `P03` covers fault-observatory migration and Prisoma observer-role
  qualification. See the
  [verified NCP task ledger](https://github.com/sepahead/NCP/blob/1ffd3bf9a6c52d0279eb31a56e0664e4eec24d68/evidence/implementation/task-ledger.v1.json).
- **The estimator pin is deliberate.** Public `pid-rs` main at `cb351ad` has newer unadopted
  contracts and exact-certifier work. Keep `796c11e` until a consumer-owned compatibility and
  scientific-value review supports a pin change. New provenance surfaces do not open PID gates.
