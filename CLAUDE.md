# CLAUDE.md — Prisoma

**`AGENTS.md` is the source of truth for how to work in this repo.** Read it first; this
file restates the highest-leverage rules and adds Claude-Code-specific notes.

## What Prisoma is

A low-overhead, world-model-first research toolkit for auditable supported decisions and matched
closed-loop fidelity studies. Partial Information Decomposition is one conditional diagnostic.
It is not the product or thesis premise. The canonical spec is `grandplan.md` (**docset v13.0**);
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
   (PID measure, preprocessing, estimator config) tuple = one pre-outcome frozen regime. The
   `categorical-sx` route fits equal-width quantizers and estimates averaged two-source MGW shared
   exclusions on the resulting empirical laws. It is not Williams–Beer `I_min`, BROJA, continuous
   Ehrlich shared exclusions, or an infomorphic objective. Never pool or auto-route these objects.
   Confirmatory claims are bound by the
   §4 claim-template registries (EC1, H1–H4 and W1–W3), the §3.8 PID kill rules, and the §6 statistical
   analysis plan. Every H1 result must say H1-A or H1-B. H1 success needs a positive useful
   margin and a one-sided lower confidence bound above it. Noninferiority, equivalence,
   nonsignificance, or a secondary endpoint cannot rescue the primary endpoint. For H2, keep a
   complete-data proper score, an IPCW estimator of complete-data risk, and a proper observed-data score distinct.
   Bind the prediction object, score, target risk, censoring construction, assumptions, and
   uncertainty as one contract. A forecast-independent censoring-adjusted horizon score can
   target scalar risk only under its exact assumptions. A right-censored likelihood requires the
   full event-time-and-type law.
   A future v3 freeze candidate populates EC1, H2, one selected H1 contract, and one selected
   H3-or-H4 contract. Keep every inactive protocol slot null. A post-H3 switch to H4 needs a fresh
   untouched sample and the frozen sequential-error rule.
2. **Honesty over roadmap.** Do not claim non-existent crates/scripts/assets are runnable.
   Avoid hard-coded performance/cost claims unless backed by a committed source or a clearly
   labeled in-repo measurement — the doc-audit scripts (`scripts/audit_*.py`) enforce this.
   Keep the active docset version stamps consistent across `README.md` / `AGENTS.md` /
   `grandplan.md` / `DIAGRAMS.md` / `findings.md` (all **v13.0**). Preserve immutable v12.5 intake.
3. **Run log = source of truth; Agent Bridge = only control plane.** Every sample admitted to an
   artifact must be reconstructable from canonical run-log events. The log governs accepted recorded
   events. It cannot prove an upstream event that the capture boundary never observed. Observers
   and harnesses drive nothing.
4. **Deployed graph beats model branding.** Do not treat `VLA` and `WAM` as exclusive classes.
   Separate predictive co-training, intended-future conditioning, coupled joint generation,
   action-conditioned prediction, and candidate planning. A joint sampler is not an
   action-conditioned query by algebra alone. Do not call an action-conditioned predictor causal without the
   randomized executed-action gate. See `grandplan.md` §9.2.
   Reject target injection. A state conditioned on a candidate action cannot be a PID source when
   the target is that exact proposal. A downstream command, later declared reference-state
   outcome, or separately measured physical outcome remains eligible only when the matched
   baseline receives the same proposal. Command or simulator-state prediction is not physical
   forecast validity. Freeze a target-specific prediction landmark before target
   availability. Bind each source to an ancestry receipt at that landmark.
5. **Low overhead is end-to-end.** Count dependency closure, safe loading, rights review, capture,
   memory, latency tails, and controller timing. Start with `just world-model-reference`. The first
   external target is the pinned compact LeWorldModel PushT planner. Its end-to-end upstream
   evaluator hard-codes CUDA and has no verified MPS path. JEPA-WM is the second planning
   benchmark. SmolVLA is the direct-policy MPS baseline. VLA-JEPA is a predictive-training
   comparator whose inference graph drops the predictor. None is a qualified runtime dependency.
   Treat the one-seed independent TwoRoom reproduction as a protocol-identity warning, not PushT
   evidence. Bind paper, configuration, and code readings before outcome access.

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
  an optional ecosystem-conformance benchmark, not a critical-path dependency. The reference
  adapter implementation is `experiments/safe_adapter`. It belongs to the preserved EC1/H
  diagnostic family, not the W1-W3 critical path. The core must build with NCP disabled. H1/H2
  must run without requesting PID atoms (dependency firebreak, §8.9.3).
- **NCP is a pinned git dependency**, currently the latest immutable release `v0.8.0` (wire
  0.8); no sibling checkout is required. Keep this legacy consumer frozen. A different wire
  requires a separate consumer surface, corpus, and qualification path. Official NCP main was
  observed at `1a04294c90c1b50eba06ae1c6afe9c951319250d` on 2026-08-13. That commit is the
  unreleased, release-blocked `1.0.0-rc.1` candidate (wire 1.0; compact proto contract hash
  `163acc57d8a62b66`). NCP ledger tasks `P01`, `P02`, and `P03` are OPEN, not dependency-ready,
  and **NOT RUN**. `P03` covers fault-observatory migration and Prisoma observer-role
  qualification. Refined low-overhead architecture prose and the prepared-stream-monitor gap
  record are coordination-only. B01 remains `IN_PROGRESS` with no passing receipt. See the
  [verified NCP task ledger](https://github.com/sepahead/NCP/blob/1a04294c90c1b50eba06ae1c6afe9c951319250d/evidence/implementation/task-ledger.v1.json).
- **The estimator pin is deliberate.** Public `pid-rs` main was observed at `7473e62` on
  2026-08-13. Its estimator-code parent is `cb3f58f0`; the child changes custody only. It has
  newer unadopted method catalogs, formal/categorical assurance work,
  source-errata records, and exact-certifier surfaces. Keep `796c11e` until a consumer-owned
  compatibility and scientific-value review supports a pin change. Full exact-head CI is red in
  two jobs, while a narrower push receipt passed. New provenance surfaces do not open PID gates.
