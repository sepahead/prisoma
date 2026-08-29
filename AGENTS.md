# Agent Notes (prisoma)

Operating rules and a ground-truth inventory for anyone — human or agent — working in this
repository. The purpose of this file is to prevent two failure modes: **hallucinated
capabilities** (claiming things exist that don't) and **doc drift** (statements that stop
being true as the code moves).

> **Estimator environment: `pid-rs` 0.9.0 post-tag review source (submodule `796c11e`).**
> This review surface makes continuous support **declared, never inferred** — a bare continuous
> config fails closed. It makes no 1.x compatibility promise and carries no published-wheel
> promise. Continuous shared
> exclusions, pipelines, hierarchy and hyperbolic paths are default-off `experimental-*` features.
> Datasets declare per-axis population support. Each continuous MI/PID call also needs one
> complete-tuple joint-law and finite-information declaration. Marginal continuity does not imply
> joint absolute continuity or finite MI. Computation status is `produced` /
> `produced_with_warning` / `abstained`, while separate population/measure/estimator/application
> verdicts govern interpretation. An **abstained estimate has
> no numeric placeholder** (no zero, no NaN, no metric event). Exact ties reject a *sample*, never
> the population law. Never auto-route a failed continuous term to a categorical route: different
> object, measure, estimator, and estimand, never pooled.
> Public `pid-rs` main was observed at `bc3aa80` on 2026-08-14. Its latest estimator-code anchor
> remains grandparent `cb3f58f0`; the two later commits change custody and assurance surfaces only.
> Its newer method catalogs, formal/categorical assurance work, source-errata registry, and exact-
> certifier surfaces remain unadopted. Full exact-head CI passed 45 jobs, and CodeQL passed four
> jobs. Provider green does not prove consumer compatibility, estimator validity, or application
> value. A consumer-owned review must prove those properties before a pin change.

> **Single source of truth for the Rust PID estimators: [`pid-rs`](https://github.com/sepahead/pid-rs).**
> `pid-core`, `pid-python`, and `pid-runlog` are **not** vendored here — do **not** re-add copies.
> They are pinned as the `pid-rs/` git submodule; the local crates path-depend into
> `pid-rs/crates/*`. Edit the estimator core upstream in `pid-rs` (then bump the submodule),
> never here. Run its binaries via
> `cargo run --locked --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all --bin exp0` and
> `cargo run --locked --manifest-path pid-rs/crates/pid-runlog/Cargo.toml --bin pid-runlog-replay`.

## Contents

- [Technical writing](#technical-writing)
- [Ground rules](#ground-rules)
- [Architecture invariants (docset-wide final solution)](#architecture-invariants-docset-wide-final-solution)
- [Repo reality — what actually exists](#repo-reality--what-actually-exists)
  - [Estimator core (`pid-rs/` submodule)](#estimator-core-pid-rs-submodule)
  - [Local crates (`crates/`)](#local-crates-crates)
  - [Machine-readable truth ledgers (`protocols/`)](#machine-readable-truth-ledgers-protocols)
  - [Python experiments (`experiments/`, tracked packages)](#python-experiments-experiments-tracked-packages)
  - [Attribution / mechanistic-probe tooling (H4 / exploratory)](#attribution--mechanistic-probe-tooling-h4--exploratory)
  - [NCP observer (`crates/ncp-observer`, optional)](#ncp-observer-cratesncp-observer-optional)
  - [Engram managed observer (`crates/engram-managed-observer`, optional)](#engram-managed-observer-cratesengram-managed-observer-optional)
  - [Specified but not built](#specified-but-not-built)
- [Gates before any PR or commit](#gates-before-any-pr-or-commit)
- [Useful commands](#useful-commands)

## Technical writing

Use [ASD-STE100 Issue 9](https://www.asd-ste100.org/assets/files/ASD-STE100_ISSUE9.pdf)
as the writing baseline for project-owned technical prose. Describe the prose as **STE-aligned**.
Do not claim formal compliance or certification without a qualified full-document check.

Apply these rules to new prose and to prose that you substantially revise:

- Use American English and short, common words.
- Use one term for one concept. Keep approved domain, API, command, and mathematical terms.
- Keep descriptive sentences to 25 words or fewer.
- Keep procedural sentences to 20 words or fewer.
- Use active voice. Use passive voice only when the actor is unknown or unimportant.
- Give one instruction per step. Use the imperative form and put the condition before the action.
- Use simple verb tenses. Do not use contractions or semicolons.
- Use a vertical list for three or more items or for complex conditions.
- Give each paragraph one topic and no more than six sentences.
- State safety commands directly. Then state the probable result of noncompliance.
- Verify commands, links, versions, pins, capabilities, and scientific status before publication.

Technical accuracy and fail-closed scientific meaning take priority over vocabulary restriction.
The limits do not apply to code, commands, paths, identifiers, literals, equations, or tables.
They also do not apply to exact quotations, historical records, licenses, or codes of conduct.
Do not rewrite immutable review intake, generated files, vendored files, or submodule documentation.

## Ground rules

1. **`grandplan.md` is canonical.** It is the research + engineering spec; keep `README.md`,
   `ARCHITECTURE.md`, `DIAGRAMS.md`, `EXPERIMENTS.md`, and `pidsplatspecs.md` consistent with
   it (current docset: **v13.0**). The primary world-model claim family is W1–W3. The preserved
   EC1/H1–H4 diagnostic family remains unfrozen. PID kill rules and the statistical plan remain
   canonical in `grandplan.md`. No real study is frozen.
2. **Gate discipline.** Do not interpret PID atoms on real embeddings. PID validity splits into
   four gates — population, measure, estimator, application (`grandplan.md` §7.1). The current
   high-d **MI/coherence path is NO-GO**; the continuous `I^sx_∩` **application gate is BLOCKED /
   NOT APPLICATION-VALIDATED**. Default Experiment 0 does not compare shared-exclusions redundancy
   with a zero target: it reports atom-measure validation as `not_adjudicated` and atom-estimator
   validation as `blocked`. `--strict-gate` gates the curated low-d analytic-MI band while only
   reporting atoms.
   Geometry diagnostics are not a substitute; sampled-mean δ is descriptive only. See
   `findings.md` and `grandplan.md` §7.2, §7.9. One (functional, exact output coordinate,
   preprocessing, evaluator-or-estimator route) tuple = one pre-outcome frozen regime. The
   `categorical-sx` route fits equal-width
   quantizers and estimates averaged two-source MGW shared exclusions on empirical categorical
   laws. It is not Williams–Beer `I_min`, BROJA, the continuous Ehrlich functional, or an
   infomorphic objective. Never pool or auto-route these objects.
   Apply [`PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md`](PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md)
   before adding, selecting, comparing, or interpreting any PID-related route. Keep the
   paper-defined functional, output quantity or lattice coordinate, declared-law evaluator, sample
   estimator, transform, certifier, validation artifact, objective composition, and application
   interpretation separate. Check the equations, law domain, source count, support or gauge,
   dependence, and oracle before deciding that a method applies. Functional identity alone is not
   enough. Also bind cumulative-versus-Möbius construction, antichain coordinate, pointwise-versus-
   averaged scope, and net, informative, or misinformative component.
   Preserve valid novel PID work as a typed research object with explicit origin and blockers.
   Removing a route from an active claim does not authorize deleting or relabeling its mathematics,
   software, fixtures, or negative evidence. No cross-functional transfer is permitted without a
   mapping theorem whose assumptions hold.
   Treat the PID decision process as evidence. Keep the canonical Markdown contract and its
   deterministic PDF publication view paired through a build receipt. The receipt must bind the
   source, renderer, and PDF hashes, exact command, extracted-text checks, page count, and visual
   review of every page. The PDF is a derived view. Never edit it as an independent authority.
   Every H1 result must say H1-A or H1-B. Do not use “H1 passed” alone. H1 success requires a
   positive useful margin and a one-sided lower confidence bound above that margin. Noninferiority,
   equivalence, nonsignificance, or a secondary endpoint cannot rescue the primary endpoint. For
   H2, keep three roles distinct: a complete-data proper score, an IPCW estimator of complete-data risk, and a proper
   observed-data score. The same arithmetic does not make those roles interchangeable. A
   forecast-independent censoring-adjusted horizon score can target scalar risk only under its
   exact conditional-censoring, positivity, and censoring-law assumptions. A right-censored
   likelihood instead requires the full event-time-and-type law.
3. **Honesty over roadmap.** No hard-coded performance, cost, or roadmap claims unless backed
   by a committed source or a clearly labeled in-repo measurement. Do not claim non-existent
   crates/scripts/assets are runnable unless they are added in the same change. The doc-audit
   scripts (`scripts/audit_*.py`) and generated capability-matrix check enforce this — run them
   before every PR.
4. **Source verification offline-first.** Network access may be restricted; prefer
   `outputs/arxiv_ref_cache.json` for citation verification when possible
   (`scripts/update_arxiv_ref_cache.py` refreshes it).
5. **No AI co-authors.** Never add Claude, AI assistants, or agents as commit/PR co-authors —
   no `Co-Authored-By:` trailer and no "Generated with Claude Code" / 🤖 marker in commit
   messages or pull-request descriptions.
6. **Candidate evidence stays separate from immutable intake.** The handoff baselines under
   `release/0.9.0/{review,requirements}` are never edited to imply progress. Candidate updates
   enter only through `release/0.9.0/candidate_progress.json`; regenerate the content-bound
   `release/0.9.0/candidate/` artifacts and run
   `uv run --no-sync python scripts/audit_candidate_release.py`.
   Passing that audit proves internal integrity, not task closure, release readiness, or a
   scientific claim. Progress schema 0.1 is deliberately non-promotable: it records only
   open/in-progress/blocked work, wave rework, and failed evidence. Positive terminal states
   require a reviewed successor schema with typed obligation coverage and authenticated CI
   attestations.
7. **Classify predictive policies by their deployed graph.** Do not treat `VLA` and `WAM` as
   exclusive scientific classes. Keep predictive co-training, intended-future conditioning,
   coupled joint generation, action-conditioned prediction, and candidate planning separate.
   A joint density does not by itself expose an action-conditioned query. Action conditioning does not
   establish an interventional transition. A planner must propose, predict, score, and select over
   at least two actions. See `grandplan.md` §9.2 and the dated WAM frontier review.
   Reject target injection. A candidate-action-conditioned state cannot be a PID source for that
   exact proposal target. A downstream command, later declared reference-state outcome, or
   separately measured physical outcome remains eligible only when its matched baseline gets the
   same proposal. Command or simulator-state prediction is not physical forecast validity. Freeze
   a target-specific prediction landmark before target availability. Bind each
   source to a tensor-ancestry receipt at that landmark.

## Architecture invariants (docset-wide final solution)

- The **run log is the source of truth for accepted recorded events**. Every sample admitted to an
  artifact must be reconstructable from canonical events. The log cannot prove an upstream event
  that the capture boundary never observed.
- The **Agent Bridge is the only control plane** — observers, harnesses, and viewers drive
  nothing.
- **Rerun** is the Phases 1–3 diagnostic/time-machine viewer; **Tauri/SparkJS** is the
  deferred Phase 4 UI/custom-rendering shell.

## Repo reality — what actually exists

### Estimator core (`pid-rs/` submodule)

- **`pid-core`** — KSG MI (with an optional exact, deterministic data-parallel `parallel`
  rayon feature), continuous `I^sx_∩` (`IsxMethod::EhrlichKsg` and baselines), 3-source
  SxPID, hierarchical screening, Shannon invariants (`invariants.rs`: r̄/v̄), PLS supervised
  dimensionality reduction (`pls.rs`, NIPALS-PLS2), a separately named fitted categorical
  Williams–Beer-style `I_min` comparator, categorical MGW shared exclusions for 2–4 sources,
  block resampling plus an m-out-of-n **stability envelope** (not an n-sample CI), and a `pipeline.rs`
  composition layer (PLS→PID3, per-atom resampling summaries, single-source permutation tests,
  LOO-CV PLS component selection, all-pairs PID2 screening, generic
  `bootstrap_rows_stats`/`permutation_rows_pvalue` row-resampling helpers), an
  L2-regularized logistic-regression classifier (`logistic.rs`, Newton-IRLS), geometry and
  intrinsic-dimension diagnostics, and the Experiment 0 runner (`bin/exp0.rs`) with a
  `--strict-gate` flag for curated low-d-band CI enforcement plus opt-in resampling and
  permutation diagnostics. Its current outputs keep the MI/coherence, atom-measure, and
  atom-estimator verdicts separate. Prisoma wires the fitted two-source MGW categorical route,
  not the `I_min` comparator.
- **`pid-python`** — typed PyO3 bindings (`pid_core_rs`) with a narrow 0.9 review surface proposed
  for later 1.0 review:
  report-first conditional KSG MI, categorical shared-exclusions PID for 2–4 sources, a separately
  named categorical `I_min` comparator, fitted equal-width quantization, resource budgets, and
  diagnostics. Legacy scalar/research calls are absent from the default namespace and live only in
  the default-off `experimental.migration` module. Local source builds are supported for review;
  no registry distribution or published wheel is claimed.
- **`pid-runlog`** — the canonical (EC1) run-log schema (`grandplan.md` §8.2) with
  validation/replay/summary/manifest/sidecar write-and-verify, the `attribution_logged` event
  schema for attribution/mechanistic probes (H4 / exploratory), and the wall-clock-excluded
  `logical_trace_hash`. Finalized schema-2 streams require every bridge request to have exactly
  one response; schema 1 retains the historical unresolved-request warning for compatibility.

### Local crates (`crates/`)

- **`pid-bridge`** — Agent Bridge dispatch, JSON-RPC request/response conversion, and
  bridge/run-log contract export.
- **`pid-sim`** — deterministic object sim with `Flow_gt` plus a baseline `flow_pred` bridge
  demo; stdio/TCP/WebSocket bridges implementing a single-request JSON-RPC 2.0 subset (silent
  missing-id notifications, explicit-`null` responses, named-object parameters with exact
  top-level method keys, required numeric `sim.step.dt`, no batch support, `-32602` for
  profile-invalid parameters, and `-32000` for post-validation handler/domain failures);
  TCP/WebSocket binaries refuse non-loopback binds
  and start with mutations disabled (`--allow-mutations` is explicit), but do not prevent
  forwarding or proxying of a
  loopback listener; TCP/stdio JSONL lines are capped at 1 MiB, WebSocket upgrades/incoming client
  frames at 16 KiB/1 MiB, and network read/write operations at 30 seconds. Standard profiles have
  no total session/request or aggregate-traffic deadline, so progress-making trickle traffic may
  persist. The optional `--engram-host` TCP profile forces an exact three-method read-only surface.
  It adds finite request-count, 64 KiB line, 8 MiB aggregate-input, 64 MiB run-log-byte,
  2,048-event, and 8-pairing-attempt limits. It also requires operator-paste pairing: one
  CSPRNG startup secret printed once on stderr. Mutual HMAC-SHA256 proofs bind the profile, the
  request ID's RFC 8785 JCS form, and fresh 32-byte nonces to one TCP connection. Eight failed
  connections latch the bridge. Pairing proves secret possession only, never process or build
  attestation.
  Run-log usage includes the TCP prefix, each request and response, and the
  terminal seal. Its `--unique-run-log-dir` option atomically creates one no-clobber log in an
  existing directory and prints that canonical path before listening. The
  WebSocket upgrade
  requires `GET /bridge HTTP/1.1`, exactly one each of a nonempty `Host`, `Upgrade: websocket`,
  tokenized `Connection` containing `upgrade`, version `13`, and a base64 16-byte key, and rejects
  `Origin`; this is not a claim to detect every malformed request. Client application messages
  are unfragmented, masked UTF-8 text frames; binary frames, fragmentation, and extensions/RSV
  use are unsupported. File RPCs use non-adversarial
  canonical confinement that rejects traversal, observed symlinks, non-regular/out-of-root inputs,
  missing parents, and pre-existing outputs; it is not a security-grade sandbox against hardlinks,
  aliases, or concurrent filesystem mutation. Run logs and Rerun outputs are no-replace. The
  default `pid-sim` build excludes protocol references, legacy sensitivity, analysis,
  Rerun/Arrow, Rapier, and WebSocket. The `protocol-references` feature enables the H1/H2 CLIs.
  The `analysis` feature enables the toy and offline harnesses. The `websocket` feature enables
  that transport. The default runtime contract omits `export.rerun`. The opt-in
  `rerun-export` feature parses and manifests the same exact source snapshot, then stages, syncs,
  and installs finalized RRD bytes/hash no-clobber.
  Executable transport run logs use `File::sync_all` for the
  initial prefix, each session flush before a wire response, and the terminal seal; generic
  `SimBridgeSession<W>` remains sink-defined. There is no parent-directory fsync, power-loss claim,
  or cross-file run-log/export transaction. Ordinary accepted-client errors seal `Failed` only while
  provenance remains writable; crashes/storage failures can leave incomplete/unreadable
  provenance, an apparently complete terminal record with indeterminate status/durability, or an
  orphan RRD. Standard profiles have **no** authentication, authorization, TLS, redaction, or
  remote-security assessment. Engram pairing proves only possession of its startup secret. It does
  not authenticate an identity or authorize remote deployment. These controls are local E0
  hardening. The crate also includes safe-mode
  `bridge.describe`/`bridge.session`/`sim.status`/`log.replay`; bridge
  `log.start`/`log.stop`, deterministic `intervention.apply`, and feature-gated `export.rerun`; flow
  checks and action/intervention replay verification; the feature-gated toy labeled harness; a
  `PhysicsBackend` trait with a null adapter and a **real `rapier3d-f64` backend**
  (gravity/contacts/friction, deterministic; behind the `rapier` feature) plus a scripted
  push-to-goal manipulation (`manipulation.rs`, `pid-rapier-harness`) emitting canonical
  run-log events with real `Flow_gt` and physics-derived labels; and the **offline
  `(V,L,D,A)` artifact-to-runlog harness** behind `analysis`, with: all-pairs `V/L/D→A` PID screens (plus
  train-split-only screens when a metadata split is present), standardization provenance,
  geometry diagnostics, strict fail-closed modes
  (label/held-out-split/class-coverage/episode-disjoint/axis-provenance),
  committed NCP-publication verification (dataset/run-log hashes, canonical-log artifact binding,
  and a successful `complete` or `complete_with_warning` visible-receipt grade;
  degraded/uncommitted NCP artifacts reject),
  deterministic sample-level, episode-grouped, and metadata-split held-out
  majority/1-NN/nearest-centroid baselines (accuracy, balanced accuracy, centroid AUROC),
  a SAFE-class held-out logistic-regression failure detector (`heldout_logreg_vlda`;
  train-fit, held-out-scored), held-out per-sample prediction records, failure-class
  confusion/rate diagnostics, default 64 MiB input, 1,024-sample, 50,000,000 pairwise-work,
  100,000,000 distance-coordinate, 100,000,000 aggregate dense-solver, and 500,000,000
  categorical-operation CLI caps, and a
  complete strict
  `--resource-limits-json` override. Producer caps do not imply harness
  admission. Report configuration binds the applied limits, distance projections, and dense-solver
  projection. Optional uncertainty remains an out-of-band schema-3 sidecar. It records row
  topology and separates exchangeability-based Monte Carlo p-values from circular-shift surrogate
  scores. The CLI requires an explicit permutation scheme. It also requires an explicit block size
  for each bootstrap or circular-shift request. A combined bootstrap and permutation request must
  declare the same row-dependence class. Current resamplers return a typed skip for mixed
  episode-id coverage or multiple episodes with repeated rows. Multi-row block subsampling and
  circular shifts require one episode plus a strictly increasing canonical decimal
  `metadata.sequence_index`. An `episode_id` alone does not prove order. Unit-block subsampling
  and full shuffle remain available under an explicit row-exchangeability assertion. The harness
  never concatenates episodes into one stationary series.
  Temporal output is a descriptive within-unit-step-run Pearson lag-1 screen. Rows without
  episode identities produce no lag pairs. Every non-singleton segment also needs a strict
  canonical `metadata.sequence_index` receipt. Only adjacent rows whose receipt advances by one
  contribute. The report counts excluded gaps. It centers both lagged vectors inside each run
  before pooling residual products. A run needs at least three lag pairs because two pairs force
  Pearson correlation to positive or negative one. It reports admitted and correlation-eligible
  pair counts. Axis means exclude columns that are undefined after centering. The report records
  defined-dimension coverage. It emits no effective-sample-size or block-length suggestion. Select inferential blocks
  from a separate pre-outcome justification. Publication requires the private process-local seal
  created by the analysis call. A deserialized summary is read-only evidence, not authority to
  mint a new summary or run log. The CLI computes the sidecar first. It then writes the summary,
  sidecar, and run log. A later write can leave that output prefix. No cross-file transaction is
  claimed.
  The crate also ships `pid-world-model-reference`. It learns a small action-conditioned affine
  transition on the deterministic fixture. It commits a fixed candidate pool and all forecasts
  before labels. It executes the selected action only through the Agent Bridge. After it commits
  the execution receipt, it labels independent branches restored from the saved fork and verifies
  replay. Schema 2 has no neutral decision record. Forecast commitments and execution receipts use
  strictly named `label_observed` compatibility envelopes, but they are not outcome labels. This
  is software conformance only, not W1 or W2 evidence. A requested future `pid-runlog` decision
  event is not adopted until the submodule pin and consumer adapter are explicitly migrated.
  Report contract `prisoma.offline_vlda.report/5` keeps fitted-quantizer receipts,
  empirical-PMF occupancy diagnostics, and
  categorical informative, misinformative, and net MGW components separate. These diagnostics do
  not prove population support or application validity. The harness supports
  `--pid-mode none|continuous|categorical-sx|categorical-sx-pls` (`none` is the default and
  requests zero MI/PID work; the opt-in analysis build still links shared `pid-core` code) with
  fitted-quantizer receipts and per-pair `categorical_saturation` diagnostics. Every
  `categorical-sx-pls` estimate is a typed, estimator-blocked same-row warning. It is a supervised
  selection-inflation diagnostic, not an inferential escape hatch. The crate also has a fail-closed typed H1 common-preflight validator/CLI with
  a schema-v2 input contract and schema-v3 result artifacts, a representative-mechanism scope,
  exact-byte content-addressed and strictly bound
  policy/instrumentation/manifests, clock/timing/lineage/fold checks,
  per-axis-scaled outputs, paired start/reset/RNG/input receipts, diagnostic-instrumentation
  noninterference, valid failed run logs for readable invalid inputs, and passing/failing fixtures;
  a deterministic finite-benchmark **Protocol A software reference** (`pid-h1-protocol-a`) that
  exact-binds that passed preflight chain, restores independent per-side clone state, reverses
  treatment order, records zero RNG draws, computes the frozen scaled response, and compares fixed
  design-only versus design+moderator ridge predictors out of outer fold. It is a synthetic scoring
  primitive only: no subprocess audit, stochastic-policy path, physical effect, Protocol B, or H1
  evidence is claimed; a deterministic synthetic **H2 fixed-horizon software reference**
  (`pid-h2-reference`) that exact-binds separately frozen analysis-plan, event-ontology,
  feature-contract, and split-manifest artifacts, then exercises task-family-held-out weighted
  fitting, grouped cross-fitted stratified reverse-KM IPCW, Horvitz–Thompson Brier risk-estimator arithmetic,
  competing-event classification, reliability bins, frozen alarm/nondetection accounting, and
  declared-payoff utility with explicit censoring abstentions. It is PID-free protocol arithmetic,
  not prospective capture, validated calibration, the comparator frontier, or H2 evidence; and a
  high-dimensional synthetic VLDA fixture
  (`offline_vlda_highdim_fixture.json`: v=128, l=64, d=32, a=7). That stress fixture uses the
  explicit `offline_vlda_highdim_limits.json` override. It does not declare complete-tuple
  continuous support. It is a PID-disabled and fitted-categorical stress fixture, not a continuous
  population-law fixture.
- **`pid-rerun`** — bounded run-log→Rerun conversion that validates its input snapshot, with
  summary/provenance/validation diagnostics; replay summaries distinguish unique metric
  names from total metric-event counts; surfaces `attribution_logged` events (see below).
  Converter input is a bounded, non-symlink regular-file snapshot; timestamps outside Rerun's
  signed-nanosecond range and viewer event/byte/log-call budgets fail before output. Headless
  `.rrd` saves are explicitly finalized, file-synced, and installed without replacement.

### Machine-readable truth ledgers (`protocols/`)

- `world_model_claim_registry_v1.json` records the unfrozen v13.0 W1-W3 claim family. W1 and W2
  are proposed primary claims. W3 is secondary or exploratory. The registry records current
  software artifacts, proof commands, blockers, and permitted and prohibited language. It is not
  a preregistration, freeze, result, or learned-model qualification.
- `research_claim_registry_v1.json` records current EC1/H1–H4 software artifacts, proof commands,
  blockers, and permitted and prohibited language. This preserved diagnostic registry is not a
  preregistration or result.
- The preserved diagnostic-governance bundle keeps the historical unfrozen v1 scaffold and the superseded,
  all-null v2 draft. Its active all-null typed v3 successor adds a role-typed H3 ancestry
  contract. It covers EC1 finite acceptance. H1-A binds one typed primary response
  contract, useful margin, comparator, uncertainty, calibration consequence, and replication
  scope. H1-B binds one primary effect endpoint, its hierarchy, mandatory design checks, and
  directional replication. It also covers H2
  target/censoring/one-primary-scoring-contract/success obligations, H3 full-population
  incremental-value superiority, warning dispositions, H3/H4 claim
  selection, and H4 target/transport/inference/power obligations. EC1 endpoint coverage
  is typed across every registered fault-adapter detection obligation and every supported adapter's
  replay-fidelity and valid-case false-positive obligations. Each fault-adapter pair requires its
  own absolute sensitivity floor, a pair-specific estimate, and mandatory passage; an aggregate
  detection rate cannot rescue a failed pair. It also has a registry stating that no confirmatory
  holdout is registered
  plus a hash-chained non-access genesis event, an empty dataset-pending
  transport/contamination ledger, and a legacy reference-inventory import with incomplete search
  provenance. `just research-governance` validates both honest unfinished schemas; it does not
  prove freeze readiness, historical/off-repository non-access, absence of contamination, or a
  systematic literature search. Both validators' `--require-freeze-ready` modes must fail until
  those scientific conditions are genuinely met; they check completeness and integrity, not the
  substantive correctness of scientific judgment or review. The v1 scaffold is deliberately
  non-promotable. The 2026-08-12 H2 correction reopened review. The 2026-08-13 ancestry change
  moved the active contract to schema v3. It remains unreviewed and is not a freeze candidate.
  Its five H3 role bindings are structural placeholders. Prisoma does not yet implement the
  source-target ancestry producer, consumer validator, or per-row receipt schema. The validator
  reports `M0_SUCCESSOR_H3_ANCESTRY_PRODUCER_CONSUMER_AND_RECEIPT_UNIMPLEMENTED` for an H3
  candidate and rejects an H3 `frozen` state. Do not treat role labels or file hashes as role
  adequacy.
  A future candidate populates only EC1, H2, the selected H1 protocol, and the selected H3/H4
  branch. Every inactive protocol slot stays null. A fresh-sample switch from H3 to H4 retains the
  frozen H3 contract only as history under the prespecified sequential-error rule.
- `ecosystem_evidence_current_v1.json` is the dated, manually network-refreshed overlay on the
  immutable review CSV. CI is offline and checks exact reviewed revisions/boundaries; it does not
  silently poll or promote upstream HEADs.
- `capability_catalog_v1.json` generates `capability_matrix_current_v1.json` and
  `docs/CAPABILITY_MATRIX.md`. Rows bind local inputs/evidence to exact content SHA-256 and enforce
  the schema for reviewed, orthogonal software-status and §8.9 evidence-basis labels. `tested` means
  a named local proof path; E3 is reserved for pinned producer/consumer golden-fixture adapters.
  Static generation verifies paths, hashes, and canonical pins but cannot infer that command text
  exercises every declared input; review plus CI execution supplies that check. The current matrix
  has no E4/E5 `validated` row.

### Python experiments (`experiments/`, tracked packages)

- **`safe_adapter/`** — the **reference `(V,L,D,A)` adapter implementation** for the preserved
  EC1/H diagnostic family. It is not the W1-W3 critical path. It converts
  released SAFE VLA rollouts into the
  `(V,L,D,A)`+labels harness contract with honest per-axis `{v,l,d,a}_provenance` markers and a
  layerwise physics-decodability hook probe. Its default ingress is a finite NPZ/strict-JSON
  bundle bound by exact file hashes plus operator-declared source/split/rights and
  model/checkpoint/hook/tensor receipts; downloaded pickle is rejected by
  default, and the explicit legacy path is manifest-hashed plus NumPy-only restricted.
  Filename/metadata conflicts, unlisted/mismatched files, resource overruns, object/non-finite
  arrays, and unverified rights fail closed unless the named rights override is explicit.
  Synthetic conversion proves software readiness only; real SAFE re-export/capture and rights
  review remain open. The generic instrumented-versus-uninstrumented preflight validator is
  implemented in `pid-sim`, but `safe_adapter` does not yet produce the real paired policy
  evaluations required to clear it.
- **`attribution/`** — attribution diagnostic (H4 / exploratory; a detached-attention,
  value-path-only epsilon-LRP baseline that is explicitly **not AttnLRP**, plus grad×input on a
  small reference model). Its content-bound, selection-disjoint/group-disjoint
  deletion-ranking-sensitivity gate carries predictor-determinism provenance, abstains on every
  exact magnitude tie, compares mean absolute deletion sensitivity with per-case random rankings,
  and aggregates independent-group wins with a conservative one-sided binomial tail. Exactly one
  predeclared primary method can set the legacy compatibility boolean. Complete-work preflight,
  reconstructable content-addressed evidence bundles, companion `artifact_logged` events, and
  schema-valid `attribution_logged` events are implemented. Evidence binds hashes of the exact
  package source bytes observed before the core modules load. It does not detect later source
  changes or attest loaded bytecode. This is never a
  causal or mechanistic faithfulness claim. Production VLAs should use a separately pinned and
  validated LXT/AttnLRP implementation where appropriate.

### Attribution / mechanistic-probe tooling (H4 / exploratory)

Attribution methods (LRP, Integrated Gradients, DeepLIFT, Grad-CAM, TCAV,
saliency/SmoothGrad, occlusion, SHAP-style probes) are **H4/exploratory companion
diagnostics/baselines**, never substitutes for PID gates. The `attribution_logged` run-log
event carries method, target_output, layer, modality, baseline, score_hash,
faithfulness_check, and artifact_uri. `faithfulness_check` is a legacy compatibility field; the
current producer sets it only when the one predeclared primary method passes the narrower typed
ranking-sensitivity result. Each probe method also publishes a content-addressed NumPy relevance
artifact and a canonical reconstructable JSON evidence bundle, both represented by companion
`artifact_logged` events. The `pid-rerun` adapter surfaces each event as a plottable **recorded
check** (1.0 pass / 0.0 otherwise), not a validated-faithfulness verdict, plus a provenance text
line at a hashed identity over method, target, layer, modality, and baseline. Other dynamic
run-log identifiers use injective single-segment percent encoding. The standalone
converter's explicit `--load-attribution-artifacts` mode additionally loads at most 1024 finite
relevance values
from an exact, bounded NumPy `.npy` file; it confines relative regular, non-symlink paths to
the run-log directory and fails closed on a missing/malformed file or a mismatch with the recorded
exact file SHA-256 and canonical shape. It preflights and retains every referenced array before
the first Rerun write under an 8 MiB aggregate cap. A conversion is also bounded to 100,000
events, 64 MiB of serialized event content, 64 MiB of application-generated entity paths,
250,000 projected Rerun log calls, and 16 MiB for a supplied compact manifest.
External loading is default-off, and bridge export never enables it. This is local best-effort
path confinement, not protection against every concurrent filesystem race. Multi-panel 2-D heatmap
blueprints remain
future work. Attribution agreement is an H4/exploratory diagnostic and must be grounded in
action and counterfactual effects, not treated as faithfulness by itself (`grandplan.md` §4,
§10.2).

### NCP observer (`crates/ncp-observer`, optional)

A **read-only** Neuro-Cybernetic Protocol tap for a conforming producer, intended to support a
future NEST/Engram session, that emits an `OfflineVldaDataset` artifact (for
`pid-offline-harness`) plus canonical run-log events
(`EmbeddingContract`/`EmbeddingCaptured`/`LabelObserved`). The named public
`sepahead/engram` repository remains a README-only placeholder. The executable Engram Neural
Labs host lives in `sepahead/Paper2Brain`. Prisoma has a digest-locked, read-only
headless-runtime descriptor. The generic host adapter reads only describe, session, and status.
No live Paper2Brain-to-Prisoma producer, NCP bridge, wire translator, or authority path exists.

- **Honors the three invariants:** the run log is the source of truth for accepted recorded events, the observer drives
  nothing (the Agent Bridge stays the only control plane), and all NCP-specific mapping
  lives in this crate.
- **Pinned dependency:** the manifest pins the latest immutable NCP `v0.8.0` release (wire
  0.8) and resolves from the published repository; no sibling checkout or path override is
  required. Official NCP main was observed at
  `1a04294c90c1b50eba06ae1c6afe9c951319250d` on 2026-08-13. That commit is the
  unreleased, release-blocked `1.0.0-rc.1` candidate (wire 1.0; compact proto contract
  hash `163acc57d8a62b66`). It uses a different wire.
  NCP ledger tasks `P01`, `P02`, and `P03` are OPEN, not dependency-ready, and **NOT RUN**.
  They cover the native-1.0 observer, missing-variable and research-claim semantics, and
  fault-observatory migration plus Prisoma observer-role qualification. Refined low-overhead
  architecture prose and the prepared-stream-monitor gap record are coordination-only. B01 remains
  `IN_PROGRESS` with no passing receipt.
  See the
  [verified NCP task ledger](https://github.com/sepahead/NCP/blob/1a04294c90c1b50eba06ae1c6afe9c951319250d/evidence/implementation/task-ledger.v1.json).
- **Workspace-excluded by design:** it is in `Cargo.toml` `exclude`, not a member, because a
  broken dependency in a *member* would fail manifest resolution for **every** `cargo`
  command (including `-p`-scoped ones). Exclusion keeps root workspace resolution/build/test
  independent of NCP; it does not change the scientific PID verdicts. Build it explicitly:
  `cargo build --locked --manifest-path crates/ncp-observer/Cargo.toml`.
- **Off the critical path:** an optional, read-only `(V,L,D,A)` source only — grandplan does
  not depend on Engram. `experiments/safe_adapter` is a preserved diagnostic adapter, not the
  W1-W3 real-data path. The core builds with NCP disabled and runs its
  static non-PID
  label-baseline smoke with PID requests disabled (`grandplan.md` §8.9.3). The analysis build still
  links shared `pid-core` code. This is
  groundwork for H1/H2, not either protocol. Workspace tests remain independent
  of NCP/Engram/Zenoh; the high-dimensional MI/coherence and application verdicts remain
  NO-GO/BLOCKED as stated above.
- **Integrity repair (2026-07-10; wire-0.8 migration reconciled 2026-07-13):** V, A, and D
  are joined only on the full driving-sensor `StreamPosition` (`{epoch, seq}`).
  `CommandFrame.source` and `ObservationFrame.source` must echo that position; a source-less
  command or plane observation is uncorrelatable and dropped (source absence is wire 0.8's
  replacement for the retired observation `seq == 0` sentinel). Pending V/A/D, closed receipts,
  and redelivery classification use the full key across epoch transitions; future-epoch
  passengers wait for a valid sensor to authorize transition. Complete validated-frame hashes
  make exact redelivery idempotent and conflicting evidence capture-invalid without mutating an
  emitted row/event. Raw decode accounting is observer-owned; duplicate JSON keys, invalid
  session/key routes, incomplete boundary state, aliased label channels, nonbinary `success`
  values, and out-of-range sensor clocks fail closed. Sensor time is truncated to unsigned nanoseconds only
  after validation. Each kept sample and capture event preserves that source value as
  `sensor_timestamp_ns`; the event clock is a nondecreasing projection. Finite
  raw/frame/axis/resident/sample/output limits
  also fail closed. Callback work crosses a bounded handoff to one owning worker. Finalization
  reconstructs and caps artifact + canonical-log bytes before no-replace/fsync installs, then
  commits their hashes with a publication receipt installed last; exact retries adopt only
  bounded byte-identical regular files at the original three canonical targets. `pid-offline-harness`
  hashes the exact parsed input snapshot and verifies the receipt, canonical log, exact dataset
  artifact identity, and visible-receipt grade; failed/uncommitted NCP input rejects. The CLI
  requires `--runlog`, exits nonzero for zero/degraded/invalid captures after preserving their
  diagnostic failed bundle, and library publication requires an explicit capture session plus a
  canonical run log before ingestion.
- **Deterministic protocol-fault observatory (fixture-only):**
  `ncp-fault-observatory` validates a bounded, complete, content-addressed wire-0.8 baseline,
  applies 18 frozen logical schedules, and replays every case twice through the same callback
  route/size classifier and raw ingress decoder as live capture. It keeps injection truth,
  native observer response, manifest-oracle comparison, logical replay equivalence, exact
  publication hashes, and path-independent semantic hashes separate, then commits the trace,
  strict per-replay `outcome.json` records, case bundles, report, and canonical run log with an
  outer receipt installed last. This is a reproducibility-bound local fixture execution only
  when the build/runtime Git revisions agree, both worktree states are clean, and the standalone
  lockfile plus exact executable hashes are recorded. Otherwise, the typed level records a local
  fixture execution without that binding. This local execution does not create producer-consumer
  E3. The NCP relationship remains E2. The binding is not signing or remote attestation.
  `--verify DIR` read-only snapshots the complete in-place publication without
  rerunning it; explicit `--out-dir` retry alone may clean the writer-reserved temporary namespace
  after reconstructing every target. The frozen outcome inventory is 16 assessed (15 matched, one
  matched known limitation for whole-tick omission), two expected `not_assessable` guards
  (logical pause and security-profile claim), and zero mismatches; `all_expectations_matched` is
  therefore not an 18/18 detection-rate claim.
- **Honesty boundary:** `capture_integrity` is a visible-receipt/join grade, not delivery
  completeness. Own-stream gap detection, receipt timing, reconnect/QoS/clock evidence, producer
  authentication, and live transport behavior remain unbuilt/unassessed. The observatory calls a
  whole-tick omission a manifest-only known limitation; logical slots are annotations that do not
  drive or measure timing; and trace truncation is not disconnect evidence. Its security case
  guards only a declared-profile label: no configuration is loaded or selected. The NCP artifact
  declares no population support: continuous KSG/shared-exclusions requests abstain,
  `--pid-mode none` requests nothing. The fitted categorical MGW route remains non-evidentiary
  with population `NotEvaluated` and application `Blocked`. Use PID-disabled
  diagnostics/baselines by default until a real producer supplies justified per-axis and
  complete-tuple declarations.
  This is not E4, EC1, live Engram validation, or security validation.
- **Still exploratory-only** (below the D2/EC1 adapter contract; optional conformance item) until
  a conforming external publisher stamps every plane observation with its driving sensor
  `source`, a language channel is present (so `L` is real, not excluded), and
  `metadata.split`/`episode_id`/
  `success` structure lands. See `crates/ncp-observer/README.md` and the developer handoff
  `NCP_DEV_PROMPT.md`.

### Engram managed observer (`crates/engram-managed-observer`, optional)

This workspace-excluded crate implements a Host API 2 read-only child.
It verifies exact Engram step, cleanup, transcript, and terminal receipt digests.

The lifecycle is `prepare`, zero or more ordered `observe` calls, then `finish`.
Preparation records one to 64 host-declared channels and an equal subject roster.
Engram source receipts do not authenticate that roster.
The maximum prepared step count is one to 1,024.
A failed or cancelled source run can finish with zero observed steps.

Every operation uses class `observation`, effect `none`, and compute grant `none`.
The child has no Agent Bridge command, PID result, NCP, artifact, filesystem operation, network operation, or actuation path.
The reviewed sandbox does not enforce filesystem isolation.
Its receipts are descriptive local observations only.
The child mirrors Engram's terminal durable-evidence profile.
It always reports `source_durable_evidence_verified=false`.
Validate a full NEST evidence bundle only in the external bounded summary tool.

Run `just engram-managed-observer-check` after any related change.
Run `just engram-managed-observer-observed-release <commit>` on Apple silicon before staging.
The observed release requires a clean checkout at the exact `origin/main` commit.
Never infer release provenance from the target path or executable mode.
Do not alter the Host API 1.1 manifest or lock through this package.
Do not claim a sealed installation or production manager execution without its exact receipt.
Regenerate source fixtures with Engram's builder after any source receipt change.
The tracked Engram-store v1 receipt is historical audit data only.
The Engram reviewed-development v2 launch gate remains `NOT RUN`.
`evidence/crebain-real-nest-observer-matrix.json` records a separate completed read-only review.
It binds one-, two-, and three-drone captures to exact CREBAIN, Engram, and Prisoma revisions.
It grants no production manager, publisher, NCP, physical, plant, or scientific authority.

### Specified but not built

The learned LeWorldModel M4 adapter, matched mesh-versus-3DGS study, fuller Rerun views, and
Tauri/SparkJS shell are not built. The native `pid-world-model-reference` is built. Do not promote
its deterministic contract proof to learned-model, physical, W1, or W2 evidence. The independent
LeWM reproduction is one-seed TwoRoom evidence, not PushT or M4 evidence. Before a LeWM port study,
bind paper, configuration, and executable-code protocol fields. Freeze each unresolved feasible
reading before outcomes.

## Gates before any PR or commit

```bash
just check
```

Run `uv sync --locked --group ui` before `just check`. The UI group is needed only because the
full suite tests the optional PNG utility. Ordinary use can keep the smaller default environment.
The aggregate verifies the no-default-feature Rust surface before all-target, all-feature Clippy
and tests. That surface excludes protocol references, legacy sensitivity, analysis, WebSocket,
Rapier, and Rerun export. The aggregate also runs warning-free all-feature rustdoc, Ruff checks,
generated-notice drift checks, and all offline truth audits. Use
`just test` for the default locked Cargo test only. Use `just docs-audit` for the complete
documentation, governance, release-integrity, capability, and repository-truth audit set. The
estimator gate itself is `just exp0-bin` (prints the GO/PIVOT/NO-GO verdict).

## Useful commands

- Search: `rg -n "pattern"`
- Required local gate: `just check`
- Rust tests only: `just test` (or `cargo test --locked` if `just` is not installed)
- Full Python tests only: `just python-test`
- Diagnostic-governance integrity (honest unfinished state, not a freeze): `just research-governance`
- NCP wire-0.8 deterministic fault suite: `just ncp-fault-observatory outputs/ncp_fault_observatory`
- Estimator gate:
  - `just exp0` (or `cargo test --locked --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all exp0 -- --nocapture`)
  - `just exp0-bin` (or `cargo run --locked --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all --bin exp0`)
  - `just exp0-runlog` (or `cargo run --locked --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all --bin exp0 -- --summary-json outputs/exp0_summary.json --runlog outputs/exp0_runlog.jsonl`)
- Toy labeled harness:
  - `just toy-harness` (or `cargo run --locked -p pid-sim --features analysis --bin pid-toy-harness -- --summary-json outputs/toy_vla_summary.json --runlog outputs/toy_vla_runlog.jsonl`)
- H1 common structural/noninterference preflight (fixture plumbing, not Protocol A/B evidence):
  - `just h1-preflight`
- H1-A deterministic Protocol A software reference (synthetic fixture/scoring primitive, not H1-A evidence):
  - `just h1-protocol-a`
- H2 deterministic fixed-horizon/IPCW/alarm software reference (synthetic protocol arithmetic, not H2 evidence):
  - `just h2-reference`
- Native exact-fork world-model contract reference (software semantics only):
  - `just world-model-reference`
- Offline VLDA embedding harness:
  - `just offline-harness` (or `cargo run --locked -p pid-sim --features analysis --bin pid-offline-harness -- --input crates/pid-sim/fixtures/offline_vlda_fixture.json --summary-json outputs/offline_vlda_summary.json --runlog outputs/offline_vlda_runlog.jsonl`)
  - `just offline-harness-require-labels` — exercises `--require-success-labels` on the labeled fixture.
  - `just offline-harness-require-heldout` — exercises `--require-heldout-split`; the checked fixture has `metadata.split=train/test` assignments and passes this strict path.
  - `just offline-harness-require-heldout-class-coverage` — exercises `--require-heldout-class-coverage`; the checked fixture has both classes in train/test subsets and passes.
  - `just offline-harness-require-heldout-episode-disjoint` — exercises `--require-heldout-episode-disjoint`; the checked fixture has disjoint train/test `episode_id` sets and passes.
  - Geometry output is descriptive. It records risk warnings but never acts as an estimator-validity gate.
  - `just offline-harness-highdim` — the high-dimensional synthetic fixture (v=128, l=64, d=32, a=7, 48 samples).
  - `just firebreak` — runs the non-PID prediction/geometry path with `--pid-mode none` and asserts zero MI/PID requests and events. It is an estimator-request check, not a link-time dependency claim.
  - `just offline-harness-categorical-sx` — `--pid-mode categorical-sx --categorical-bins 8` (fitted categorical MGW SxPID with signed components and fitted-quantizer receipts; results remain non-evidence while gates are blocked).
  - `just offline-harness-categorical-sx-pls` — `--pid-mode categorical-sx-pls --pls-components 2 --categorical-bins 8` on the high-dimensional fixture. Each screen fits PLS and quantization on the same rows it analyzes. Every estimate carries a typed warning. The optional split screen uses train rows only and does not score held-out categorical rows.
- Run-log smoke:
  - `just bridge-contract`
  - `just bridge-security` — local-only unit proof for bind/safe defaults, the enumerated wire
    caps/upgrade checks, JSON-RPC subset behavior, and canonical/no-replace file handling; it is
    not remote-security or adversarial-filesystem validation.
  - `just runlog-demo`
  - `just runlog-bridge-demo`
  - `just runlog-bridge-stdio-safe`
  - `just runlog-bridge-stdio`
  - `just runlog-bridge-tcp`
  - `just runlog-bridge-ws`
  - `just runlog-validate`
  - `just runlog-summary`
  - `just runlog-manifest`
  - `just runlog-sidecars`
  - `just runlog-sim-verify`
  - `just runlog-replay`
  - `just runlog-rerun`
  - `just runlog-rerun-bridge`
  - `just runlog-bridge-export-rerun`
