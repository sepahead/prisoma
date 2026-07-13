# Changelog

## Unreleased

### NCP observer visible-receipt integrity prerequisites (2026-07-13)

- Reworked the optional NCP tap around full driving-sensor `{epoch, seq}` keys. Pre-active and
  future-epoch command/readout receipts no longer collide or authorize a restart; only a fully
  validated sensor advances the watermark/epoch, while already-buffered passengers for that epoch
  survive the transition. Complete bounded typed-frame receipts remain classifiable across retired
  epochs: exact redelivery is idempotent, conflicting evidence quarantines/invalidates without
  last-write-wins mutation, and later quarantined traffic is not mislabeled as an ordinary duplicate.
- Moved raw frame/byte budgets and decoder/medium integrity into observer-owned state so a caller
  cannot reset or forget diagnostic counters to obtain a successful log. The live raw-session path
  shares exact base-key routing and the strict decoder, rejects duplicate JSON keys and invalid NCP
  session segments, bounds frames before the handoff, and records callback oversize/route/handoff
  loss separately. Added finite global ceilings for retained closed receipts and resident in-flight
  elements, complete-before-capacity flushing, and explicit incomplete/unclaimed state accounting at
  transition/capacity/finalization without cloning all partial vectors.
- Made NCP bundle publication fail closed: artifact and run-log bytes are bounded and reconstructed
  before publication, installed no-replace + fsync, and committed by a hash-binding publication
  receipt installed last. Publication requires an explicitly bound session and UTF-8-representable
  canonical targets; retries pin all three bundle targets and verify bounded byte identity. The
  offline harness now reads/hashes one immutable regular-file input snapshot and verifies the
  receipt, both hashes, canonical run-log validity/run id/exact artifact identity, and a successful
  visible-receipt grade before accepting NCP input. Known degraded/invalid and zero-row captures
  preserve a failed diagnostic bundle but the CLI exits nonzero. NCP artifacts deliberately carry
  no inferred pid-rs 1.0 population-support declaration: continuous KSG/shared-exclusions requests
  abstain, `--pid-mode none` requests nothing, and quantized discrete `I_min` is non-evidentiary
  with population `NotEvaluated` and application `Blocked`. PID-disabled diagnostics remain
  available after a valid commit.
- Scope remains local exploratory integrity groundwork. `capture_integrity` does not detect wholly
  missing own-stream ticks or attest receipt time, reconnect/QoS/clock state, authenticated producer
  identity, live conformance, E4/EC1, or security. The deterministic protocol-fault observatory is
  still specified but not runnable.

### SAFE ingress integrity and provenance (2026-07-13)

- Replaced the reference adapter's default trust in downloaded `.pkl` files with a finite,
  content-addressed canonical ingress: action CSV + object-free NPZ arrays + strict JSON metadata,
  bound by `safe_bundle_manifest.json` with exact file sizes/SHA-256, immutable source revision,
  complete task universe/seen-role/origin/freeze/contamination receipt, model/checkpoint/hook/tensor
  contract receipt, and operator-declared rights-status/reference attestation. Synthetic fixtures now use
  that same safe bundle. Missing/extra/mixed or hash-mismatched files, filename/payload identity or
  outcome conflicts, non-finite/object/ragged arrays, and file/row/tensor/archive budget overruns
  fail closed before conversion; FIFOs and other non-regular inputs reject without a blocking
  open; the raw and manifest hashes flow into emitted sample lineage. Conversion also rejects
  missing lineage or pooling across different manifest/source/rights/split/capture receipts.
  The converter explicitly marks the source bundle as external and not archived by the converter,
  rather than inventing a resolvable artifact URI; real EC1 evidence still has to preserve and
  hash-verify that bundle and its tensor contract outside the converter.
- Raw pickle is rejected by default. The explicit legacy option first verifies the file against the
  manifest and uses a NumPy-only restricted unpickler that rejects arbitrary globals and Torch
  pickles, trailing objects, allocation-capable ndarray reconstruction, and files above the separate
  legacy ceiling; it is documented as a trust boundary, not a sandbox. Unverified rights are likewise
  default-off behind an explicit, non-authorizing override. Current upstream Torch-tensor rollouts
  therefore require review and safe re-export in a disposable constrained environment before the
  normal adapter path.
- Made the emitted contract JSON finite on read/write and atomic/fsync-durable with no-clobber
  installation by default. Added adversarial
  tests for code-execution gadgets, legacy-default refusal, content drift, metadata conflicts,
  type confusion, resource ceilings, and output overwrite policy. This is capture-ingress software readiness only:
  no real SAFE capture, rights determination, S2/EC1 completion, or H1/H2 evidence is claimed.

### H2 fixed-horizon software reference (2026-07-13)

- Added `pid-h2-reference`, a strict PID-free deterministic synthetic reference for one named
  fixed-horizon cumulative-incidence estimand. It exact-binds outcome-independent analysis-plan,
  event-ontology, feature-contract, and split-manifest artifacts; rejects future/unavailable
  features and episode/world/family fold leakage; fits paired task-family-held-out baseline and
  diagnostic weighted-logistic models with outer-training transforms; cross-fits frozen-stratum
  reverse-Kaplan–Meier censoring models inside training; and reports Horvitz–Thompson IPCW Brier
  arithmetic with target, competing, event-free, and censored outcomes kept distinct. Censored
  rows remain in the estimand denominator but carry no numeric weight or row-loss placeholder;
  unsupported strata and positivity failure abstain rather than clip.
- Added reliability-bin reference arithmetic, executable frozen-threshold persistence/refractory/
  capacity/event-matching semantics, tagged detected/undetected lead-time records and all-event
  lead curves, and a declared-payoff utility scenario. A focused multi-landmark boundary test
  covers positive matching, refractory suppression, capacity rejection, and nondetection. Alarm
  and utility accounting abstain on the censored artifact because follow-up is incomplete.
  Passing, censored, future-feature unit, semantic-lineage failure, and readable parse-failure
  paths produce deterministic canonical logs with zero PID, action, and intervention events. The
  binding flags explicitly deny prospective capture, H2 evidence,
  external validation, comparator completeness, and censoring-assumption validation.
- Replaced H1 Protocol A's potentially oversized full-input `ConfigLogged` value with a compact
  content-addressed receipt, keeping every accepted input below the run-log line limit while the
  exact input remains reconstructable from its logged artifact hash.

### H1 Protocol A software reference + current-truth ledgers (2026-07-13)

- Added a strict, fail-closed H1 common-preflight schema and `pid-h1-preflight` CLI. It binds the
  source run, analysis, population, split, blinding/order, output-metric, snapshot, moderator,
  start/reset/RNG/input receipts, memory, and cache artifacts by exact-byte verified SHA-256;
  parses the frozen manifests strictly; validates a common clock and strict pre-treatment order;
  keeps task families/interference clusters in one outer fold; applies the declared per-axis output
  scales; and checks blinded, counterbalanced instrumented/uninstrumented outputs, latency,
  controller offsets, paired starting state, memory, and cache with stable reason codes and explicit
  local-check denominators.
  Passing/failing fixtures and canonical schema-2 run logs exercise the contract in CI. This is
  structural software evidence only: no Protocol A/B response estimate or H1 scientific evidence
  is produced.
- Added `pid-h1-protocol-a`, a deterministic synthetic finite-benchmark execution/scoring reference.
  It exact-binds a separately passed preflight chain and its representative-mechanism scope, policy
  and instrumentation specs, clock and clone/reset/application boundaries, control/treated versions,
  and output contract. Per case it binds the exact moderator and clone-state values, restores
  independent treatment-side policy instances, exercises both orders with zero RNG draws, and writes
  canonical treatment receipts in actual execution order. Fixed leave-one-outer-fold-out ridge
  comparisons retain proper scores for homogeneous/null responses while calibration explicitly
  abstains on zero prediction variance. Bounded artifact reads, output-alias checks, dense-scoring
  work limits, deterministic replay, and readable invalid-chain/parse fixtures fail closed with no
  numeric or PID events. This remains a scoring primitive: no subprocess/stochastic audit, physical
  individual effect, Protocol B, real capture, or H1 evidence.
- Added living machine-readable current-state overlays in `protocols/` for ecosystem evidence and
  EC1/H1-H4 execution status, plus offline truth-audit checks. Corrected the named public Engram edge
  to its verified README-only placeholder revision and distinguished Haldir's reviewed revision from
  current public main; neither edge is promoted into Prisoma's critical path.
- Corrected the dependency-firebreak language: the current `--pid-mode none` smoke proves static
  factual-outcome label baselines run without PID atoms; it is not H1 response prediction or
  prospective H2. Withdrew the retired v10.7 power-grid counts as current capture requirements and
  corrected `pidsplatspecs.md` to schema 2 / partial-M2 truth.

### `pid-rs` 1.0 migration + estimate abstention subsystem (2026-07-12)

- **Bumped the `pid-rs` submodule `8a5a9dd` (0.4.0) → `ac4a780` (1.0.0).** This is an
  estimator-environment change, not a routine pin bump: 1.0 narrows its default surface (continuous
  shared exclusions, pipelines, hierarchy and hyperbolic paths are now default-off `experimental-*`
  features), restructures the flat `pid_core::*` namespace into `stable` / `diagnostics` /
  `experimental`, and makes continuous support **declared, never inferred** —
  `SupportContract::Unspecified` fails closed. `crates/pid-sim` and `crates/pid-rerun` now enable
  the features they use and construct configs through the explicit
  `assume_regular_full_dimensional()` population-law assertion.
- **Corrected scientific status, not a capability regression.** pid-rs 0.4 silently produced numbers
  for tuples outside its valid support contract. 1.0 exposes the mismatch by **failing closed**. The
  committed `offline_vlda_fixture.json` has a **binary `L`** (an instruction/condition indicator),
  so every continuous `(V,L)→A` / `(L,D)→A` screen and `MI(L;A)` had been running an
  absolutely-continuous KSG estimator over a two-valued variable. Those estimates were never valid;
  they are now reported as abstentions.
- **Estimate abstention subsystem** (`grandplan.md` §7.14). Datasets declare per-axis population
  support (`support: {v,l,d,a}`); support is **declared by the adapter, never inferred from observed
  cardinality**. Every candidate carries a typed computation outcome — `not_requested` /
  `produced` / `produced_with_warning` / `abstained` — with the requested measure, the exact estimator revision,
  the axes, a **stable reason code**, and the observed axis evidence (unique rows, multiplicities).
  Reason codes: `declared_support_incompatible_continuous`, `support_contract_unspecified`,
  `observed_sample_incompatible_exact_ties`, `ambiguous_neighbor_shell`,
  `estimator_requires_equal_source_dimensions`. **A value exists only when it was produced** — an
  abstained estimate emits no numeric placeholder, no zero, no NaN, and no `PidMetric` event; its
  status replays from the run log. Scientific population/measure/estimator/application verdicts
  remain separate from computation status and accompany every emitted numeric metric. Denominators
  (requested / declared-support-compatible / preflight-passed / estimated / warned /
  abstained-by-reason) are reported.
  Exact ties reject a **sample** for the continuous estimator; they never prove the population law
  is discrete. Failed continuous terms are **never** auto-routed to discrete `I_min` — that is a
  different measure with its own estimand identity, and the two are never pooled.
- **Fixtures.** `offline_vlda_fixture.json` is unchanged and preserved as the **mixed-support
  regression fixture**: it now declares `l: categorical` and proves that unsupported inputs produce
  a clean, auditable abstention. New `offline_vlda_continuous_fixture.json` (declared all-continuous,
  equal ambient source dimensions, tie-free) retains positive-path coverage for continuous
  KSG / `I^sx_∩` — `just offline-harness-continuous`.
- **Completed the consumer migration follow-ups.** `tests/python/test_pid_core_rs.py` now exercises
  the stable 1.0 report-first MI and categorical SxPID/`I_min` surfaces, verifies that legacy scalar
  calls are absent, and keeps the measures separate. Third-party notices and the excluded
  `ncp-observer` lock now resolve pid-core/pid-runlog 1.0; the observer emits a canonical SHA-256
  configuration hash over its locked NCP identity and effective capture settings and passes its
  locked suite.
- **Made the dependency firebreak real.** `pid-offline-harness --pid-mode none` runs geometry,
  labels, held-out prediction records, and every non-PID baseline while requesting/emitting zero
  MI/PID estimates. CI and the task recipe assert `requested=0`, `pid_metrics=0`, and the presence
  of the SAFE-class held-out logistic baseline.
- **Separated computation from scientific eligibility.** Estimate outcomes now say `produced`,
  `produced_with_warning`, `abstained`, or `not_requested` and carry explicit population, measure,
  estimator, and application verdicts. A produced diagnostic remains application-blocked unless a
  versioned support envelope passes; no numeric placeholder is emitted for abstention. Canonical
  numeric metric events carry the computation outcome, exact measure/revision, four verdicts,
  interpretation flag, and warning code; uncertainty sidecars carry the same gates. Legacy
  `eligible` / `eligible_with_warning` summaries and `support_eligible` denominators deserialize
  conservatively with the application gate blocked.
- **Repaired 1.0 command/fixture drift.** Exp0 commands enable the required `experimental-all`
  feature, and smoke assertions reflect the mixed-support fixture’s four surviving PID/MI events
  and 20 geometry metrics instead of the invalid pre-1.0 counts. The Rapier CI/task recipe now runs
  its physics and manipulation test filters as two valid Cargo commands instead of passing Cargo a
  rejected second positional filter.
- **Added dependency-truth drift prevention.** `scripts/audit_repo_truth.py` binds active Exp0
  commands, estimator stamps, the excluded observer lock, NCP provenance revision, firebreak
  assertions, and generated notices to the checked-out gitlinks/packages; CI and `just docs-audit`
  run it.

### Docset v12.5 — adopt the second-round intervention-grounded plan (2026-07-12)

- **Adopted `grandplan.md` docset v12.5**, replacing the v10.7 "Comprehensive prisoma
  Specification" with the intervention-grounded plan from the 2026-07-12 second-round
  adversarial review. The thesis is reframed around **auditable experiment semantics** with
  **PID as a conditional candidate**. New structure: the **EC1 + H1–H4** confirmatory claim
  registry (a Protocol A paired frozen-snapshot vs Protocol B randomized closed-loop split for
  H1; a censoring-aware H2 with the 2026 monitor comparator frontier and a frozen alarm policy;
  a conditional H3; an availability–use H4), the **S0–S7** gate sequence, **M0–M7** research
  milestones, **four separated PID gates** (population/measure/estimator/application), an
  **E0–E5** ecosystem evidence ladder (only `pid-rs` and NCP are direct edges), a **dependency
  firebreak**, and an **R1–R112** reference ledger. The outgoing plan is archived at
  `docs/archive/grandplan-v10.7.md`; the review bundle is vendored at
  `docs/reviews/2026-07-12-grandplan-v12.5/`. (The v11/v12 intermediate review versions were
  not adopted directly — v11 was the first-round replacement.)
- **Migrated the whole docset to v12.5.** Restamped and retargeted every companion doc
  (`README.md`, `AGENTS.md`, `CLAUDE.md`, `findings.md`, `ARCHITECTURE.md`, `DIAGRAMS.md`,
  `EXPERIMENTS.md`, `pidsplatspecs.md`, `NCP_DEV_PROMPT.md`, `RESEARCH_VLA_D_NCP.md`,
  `REVIEW_AND_TODO.md`, the optional-spec and observer READMEs) to the new section numbers, the
  EC1/H1–H4 registry, the four PID gates, and the E0–E5 ecosystem language — preserving the
  runnable "what exists today" content and the control-plane invariants (run log = source of
  truth; Agent Bridge = only control plane; observers/PID/Zenoh/Rerun never actuate).
- **Migrated the doc-audit scripts.** `scripts/audit_grandplan.py` now validates the R1–R112
  reference ledger (contiguous IDs, all defined + cited, no undefined/unused/duplicate) instead
  of arXiv-cache coverage; the venue heuristic in `audit_grandplan_claims.py` /
  `audit_docset_claims.py` is bibliography-aware; and `audit_docset_claims.py` treats
  `docs/archive/**` and `docs/reviews/**` as whole-file historical records.

### Added

- **§14.8.3 power gate implemented and first-run** (`crates/pid-sim/src/power.rs`,
  binary `pid-sim-power-gate`, artifacts in `docs/power-gate/`). This was the first run of the
  then-v10.7 **idealized endpoint model**, not the current Protocol-A/Protocol-B/prospective-H2
  procedures or nested capture design. It used an episode-level binormal ΔAUROC model,
  family-blocked Gaussian-copula task correlations, and a family-blocked matched-case Kendall
  model. Exact machine-readable verdicts (400 replicates/cell, 500 bootstraps, one-sided α = 0.05)
  were: **H1 did not reach a passing grid point at its registered minimum effect; H2 first passed
  at 64 tasks; H3 first passed at 40 matched cases; H4 first passed at 96 tasks.** H3's 30-case
  raw-power cell failed its same-n null-size screen. Raw H1 0.08-effect power was 0.795 at 640 and
  0.897 at 960 episodes, but those cells are not capture requirements. The dated unit tests cover
  quantile, AUROC/Spearman/Kendall, calibration, and grid-selection mechanics.

### Changed

- **Finalized the NCP wire-0.8 migration: re-pinned `ncp-observer` to the immutable NCP
  `v0.8.0` release (wire 0.8).** Moved `ncp-core`/`ncp-zenoh` from the
  `wire-0.8-stream-identity` branch to the `v0.8.0` tag and refreshed the crate lockfile; all
  active NCP-pin references across the docset were reconciled to `v0.8.0`/wire 0.8. The
  `meshmaker/` tombstone records its relocation to `relief-atlas`.

- **Re-pinned `ncp-observer` to immutable NCP `v0.7.1`.** This is a
  wire-identical patch on wire 0.7 (`CONTRACT_HASH = f05e328cad20959d`): observer
  behavior and exact-`seq` D alignment are unchanged. The manifest and lockfile
  now resolve the Rust 1.88-compatible release commit in lockstep, and active pin
  documentation (including the stale root workspace comment) is synchronized.

- **Repaired `ncp-observer` evidence integrity and adopted the immutable NCP
  `v0.7.0` / wire-0.7 release:** exact
  seq-only D with seq-0/late drops, bounded callback-to-worker handoff,
  `Keys::try_new`, explicit open vs fail-closed secure client modes, immutable
  sample/events, and atomic durable artifact + reconstructed run-log finalization.
  Append/hash/write failures now propagate while preserving retry state; focused
  tests prove deterministic reconstruction without duplicate events.

- **Re-pinned `ncp-observer` to NCP `v0.6.0` (the wire-0.6 enforcement cut) and
  enforced the wire contract on ingress** (commit `c48d70b`). Wire 0.6 is a
  *semantic* break with an unchanged serialization, so `CONTRACT_HASH` stays
  `24e8e6e31e1dec8a` — the `ncp_version` string (`0.5` → `0.6`), not the hash,
  gates compatibility. The three data-plane subscribers now decode through
  `ncp_core::decode_validated`, so a version-less, incompatible, or unstamped
  (`seq < 1`) frame is dropped and counted instead of being fed into the
  `(V, L, D, A)` join; a new `accept_observation` helper additionally counts
  valid-but-unstamped plane observations (`seq == 0`, recency-fallback D) so a
  D-degraded capture is diagnosable. **Gap 1 (D `seq`-alignment): the protocol
  half is now resolved** — wire 0.6 makes plane-published
  `ObservationFrame.seq >= 1` (echoing the driving `SensorFrame.seq`) normative
  and the observer enforces it; the residual is purely that the Engram publisher
  must stamp `obs.seq`. The pinned pid-rs submodule is unchanged.

## Docset v10.7 (2026-07-06)

First-principles spec audit + statistics-plan slice. **No research-conclusion
change** — the Exp0 gate still blocks continuous-atom interpretation on synthetic
high-d controls (verdict label now **NO-GO** under pid-rs 0.4.0, see the Addendum;
PIVOT under 0.3.0) and the open critical path is still the first real-VLA capture
(not done). Produced by a
five-agent audit of `grandplan.md` against the pid-rs code, the NCP repo, first-principles
mathematics, hostile study-design review, and citation verification, plus a 104-agent
adversarially-verified July-2026 literature sweep. No prisoma Rust code changed in this slice (the Addendum bumps the pid-rs submodule pointer + lockfile).

### Fixed (mathematical/precision errors in the docset)

- **`findings.md` δ-hyperbolicity direction was backwards** ("High δ → Hyperbolic"):
  trees have δ = 0; LOW δ indicates tree-like/hyperbolic structure. This contradicted
  `grandplan.md` §16.7 (which was correct). The one outright error found by the audit.
- Small-x digamma expansion corrected (`ψ(x) ≈ −1/x − γ`, not `−1/x − 1/(2x²)`);
  §15.1.2 retitled pole-not-underflow, with the counting-convention argument-≥1 guarantee.
- `r̄ − 1 = CoInfo/I` identity scoped to n=2; `r̄ = 1` reading narrowed to "additive MI"
  (independence neither implies nor is implied); findings.md Scenario-B inference softened.
- CI-sign convention ("negative = synergy-dominant") explicitly scoped to CI₂ (invalid at
  m=3 by the §5.3 parity-flip); "7 MI estimates" for the pairwise-CI trio corrected to 6.
- Distance-concentration statements conditioned on the Beyer et al. (1999) hypotheses in
  both files; strong-dependence sample complexity restated as exponential-in-MI (was
  "quadratic in 1/noise"); "PCA preserves Euclidean distances" softened (projection);
  mean-δ vs sup-δ estimator caveat added (§16.7.1); Ehrlich validation envelope in
  Warning 3 corrected from "~100 dimensions" to low-d (verify against the paper).

### Changed (hypothesis system / preregistration — grandplan.md)

- §14.1 registry now carries a falsifiability index covering H4–H7 (previously scattered);
  H7 split into H7a (method) / H7b (falsifiable claim); H6 unified as Deferred with an
  explicit in-section disproof; H4 refinement bounded to preregistered variants; H5
  operationalized (phase-aligned windows pooled across episodes; preregistered trend
  endpoint); H2's availability-vs-use gap named with the asymmetric outcome preregistered;
  H9 given a quantitative Kendall-τ/sign-agreement criterion.
- Discrete estimator regimes (I_min mode and, once wired, discrete SxPID) preregistered
  NOW as first-class H1–H5 formulations — closes the H2/H3 "untestable while Exp0 stays
  PIVOT" dead end without violating the anti-measure-switching kill criterion.
- **New §14.8 Preregistered Statistical Analysis Plan:** one primary endpoint per
  hypothesis, hierarchical gatekeeping + BH-FDR over the exploratory grid, a
  simulation-based power analysis as an M5 capture gate (with preregistered minimum
  effect sizes), and a mandatory minimal baseline set (SAFE-class logreg, predictive
  entropy, majority/1-NN/centroid, one ensemble/temperature variant).
- **New §9.7.2b:** dedicated H2/H3 ablation-sensitivity protocol on the V0–V4/W0–W4 axes.
- Decision rules repaired: SSI made sign-safe — final form (same-day addendum)
  `SSI := −IQR(Syn)` per preregistered perturbation axis, Tukey quartiles, in nats
  (the old `1 − Var/Mean` explodes when median Syn ≈ 0 or flips sign, and the
  intermediate `1 − IQR/(|median|+ε)` normalization was still unstable near zero); the 3-of-5 vs 4-of-6
  publication gates reconciled into one canonical rule (OOD + task-difficulty must-pass;
  embodiment-gap must-pass for H7b claims); the permutation placebo criterion
  replaced (final form in the Addendum: distribution-centered-at-chance test; the
  old fixed 0.55 threshold exceeds chance at small episode counts); §9.6
  "conditional success" pivots require held-out replication; §18.2.5 "unique
  interpretability" GO operationalized as a PID-vs-entropy guided-intervention test;
  memorization-index stratification restricted to pre-outcome quantities; VLA-Arena
  "sufficient statistical power" claim replaced with a §14.8 pointer.

### Changed (July-2026 literature refresh — verified against primary sources)

- **New §12.6 (checked 2026-07-06):** PID field-status review added
  (arXiv:2603.06678, Liardi et al. 2026); second impossibility line added
  (arXiv:2604.03869) with a position-against-both note; continuous-`I^sx_∩` validation
  ceiling re-confirmed current (still low-d synthetic only through mid-2026); §2.2.4
  gains two publication-critical negative-atom nuances (documented empirical negatives
  are *unique* atoms, not synergy; the authors sanction time-series clamping — prisoma
  must preregister clamped/unclamped reporting); r̄/v̄ anchored to arXiv:2504.15779
  Propositions 1–2 (and noted as matching `invariants.rs`).
- VLA landscape: world-model-VLA family recognized as surveyed (arXiv:2606.00113), with
  WorldVLA/FlowVLA/RynnVLA-002/3D-VLA noted in §7.5; VLA-Arena resolved to **ICML 2026** (re-verify at submission)
  and TraceVLA to ICLR 2025 (re-verify at submission); SmolVLA cited (arXiv:2506.01844; §7.8.1 updated to
  paper-reported vs still-verify); PixelVLA improvement range version-pinned (v1
  10.1–17.8% vs current revision 10.1–28.7%); missing arXiv IDs added (Ehrlich
  2311.06373, Liang 2302.12247, Schick-Poland 2106.12393, Makkeh 2002.03356); WAN title
  aligned; SIMA 2 / CQN-AS date mismatches fixed; PRE-HAL/GalaxeaVLA/GWM/VLA-Arena-asset
  hedging added. Novelty re-verified 2026-07-06: **no PID/information-decomposition
  analysis of VLA or robot policies found**; closest adjacent work is VLA-InfoEntropy
  (arXiv:2604.05323); "VLDA" is unclaimed as a term. (`arXiv:2512.16662`'s author
  "Matthias PH" verified correct — Philip Hendrik Matthias — do not "fix" it.)

### Fixed (repo-truth drift)

- §11.2 project tree redrawn to reality (pid-rs is a submodule — estimator crates are
  NOT under `crates/`; `experiments/{safe_adapter,attribution}` are tracked packages;
  `scripts/`, `tests/`, `meshmaker/`, `crates/ncp-observer` shown).
- "General n-source SxPID" corrected to **2–4 sources** at every current-state site (`discrete_sxpid_n`
  errors above 4); pid-rs 0.3.0 noted as a crate-version/commit pin (upstream has no
  v0.3.0 git tag); §15.2.1–15.2.2 SIMD/streaming sketches labeled **not implemented**
  (the shipped path is brute-force exact kNN + the optional `parallel` rayon feature);
  §8.3.2 marks which of its three recommendations exist.
- NCP pin prose synced to **v0.5.3** (manifest was re-pinned in `ea779fd` but eight files
  — 11 prose sites — still said v0.5.2: `README.md`, `AGENTS.md`, `NCP_DEV_PROMPT.md` ×3,
  `crates/ncp-observer/README.md` ×2, root `Cargo.toml` comment, `DIAGRAMS.md`,
  `RESEARCH_VLA_D_NCP.md`, `REVIEW_AND_TODO.md`). Wire-identical; `CONTRACT_HASH`
  unchanged `24e8e6e31e1dec8a`; v0.5.3 is NCP-local fail-closed safety hardening.
- `grandplan.md` B.7 version-history table had no v10.6 row and still marked 10.5 as
  "(Current)"; added 10.6 + 10.7 rows and moved the marker.

### Addendum (same day — triple-check pass)

Three independent verification passes (hunk-by-hunk diff verifier, upstream pid-rs
auditor, hostile referee review of the new statistical content) drove a second
round of repairs; no research-conclusion change.

- **Statistical repairs to the v10.7 additions themselves:** H2 primary endpoint
  re-unitized from task families (n≈4 — untestable) to **tasks** with family-blocked
  bootstrap; H3 endpoint redefined as **mean per-case Kendall τ** across ≥ 20 matched
  cases (a 3-item Spearman is not a statistic; minimum effect τ̄ ≥ 1/3); §9.7.2b
  strength matching made outcome-independent (equal-success-impact calibration was
  circular for H3); §14.2.2 permutation placebo re-based on the
  distribution-centered-at-chance test (the quantile-of-own-distribution version
  fired 2.5% always and missed wholesale leakage); H9 criterion fully specified
  (per-case τ, tie/Syn-dominance rules, family-blocked case bootstrap, negative CI =
  affirmative disconfirmation, argmax-identity wording); §14.8.2 gained the
  regime-multiplicity rule + an explicit error-rate statement; §14.8.3 scoped to
  H1–H4 with per-endpoint analysis units and a CI-based H1 futility bound;
  clamped/unclamped reporting scoped by claim type (negativity claims are
  unclamped-only by definition); H5 windowing now inherits §9.0 fully (within-window
  stride, post-stride `N_win`, outcome stratification); RoPE score notation fixed
  (`Var_pos/Var_sem`); directional α; §9.3.2 aligned to the incremental H1 contrast.
- **Verifier fixes:** three GFM-breaking table rows (raw pipes in cells: SSI, MAR,
  B.7 v10.2) escaped; Bonferroni cross-ref §14.5.2 → §14.5.1; survey date
  standardized (June 2026); H6 registry cell → "Deferred"; `ARCHITECTURE.md` /
  `EXPERIMENTS.md` stamps → v10.7; DIAGRAMS H7 node split + observer landing-tag
  restored (v10.5); PixelVLA range un-abbreviated in the notes; memorization
  indicators labeled (a)/(b)/(c).
- **pid-rs 0.4.0 adopted:** upstream audited clean of every docset error class;
  **v0.4.0 released** upstream (CountSketch sign fix, true moving-block bootstrap,
  exp0 `NegativeHandling::Allow`, bias-corrected Levina–Bickel, Python API cleanup —
  breaking under 0.x, hence 0.4.0) with retroactive v0.3.0 + release v0.4.0 tags;
  submodule re-pinned `7e8f16d` → v0.4.0; `repin-pidrs.sh` consumer default renamed
  pid_vla → prisoma. **Consequence: the Exp0 verdict on the synthetic high-d
  controls is now NO-GO** (3 invariant-bound violations surfaced, e.g. v̄ ≈ −26.6 on
  `unique_s1_pca` d=64; ID(t) 1.14 → 1.01 confirms the old value was estimator
  bias) — a stricter gate, not a science change; `findings.md` updated.
- **Docs:** `AGENTS.md`, `CLAUDE.md`, `README.md` professionally restructured
  (content-preserving; AGENTS.md's two ~600-word single-bullet inventories are now
  organized sections).
- **Every-markdown sweep (four review agents across both repos):**
  - **Deleted** `FINAL_INTEGRITY_REPORT.md` and `GENERATION_REPORT.md` — one-off,
    machine-generated inventories of untracked/quarantined `meshmaker/` asset outputs
    (zero inbound references; mutually contradictory counts; carried military-asset
    listings the governance review wanted out of scientific releases; git history
    preserves them; the run-log provenance rule lives in grandplan §A.8 and the
    `THIRD_PARTY_NOTICES.md` checklist).
  - **`THIRD_PARTY_NOTICES.generated.md` regenerated** for the pid-rs 0.4.0 re-pin
    (pid-core/pid-runlog rows 0.3.0 → 0.4.0; the CI `--check` gate had been failing).
  - **`EXPERIMENTS.md` aligned to v10.7:** §0.1 rows now carry the §14.8.1 primary
    endpoints with predicted directions (H1 incremental ΔAUROC; H2 across-tasks ρ;
    H3 mean per-case τ; H4 SSI := −IQR); H5 row + §6.4 rebased on the pooled-window
    rule (per-episode `analyze_temporal_pid` demoted to exploratory visualization);
    **new §6.5 H5 windowing preregistration stub** (the section grandplan §3.6.3
    promises); Exp3 rebadged H2/H3 (§9.7.2b) with H6 exploratory-only (Deferred);
    H7 split; the `independent_additive` "Red≈0, Syn≈0" ground-truth comment
    corrected (I^sx redundancy is genuinely positive there; additive ≠ independent);
    one-sided synergy test → two-sided; §0.2 rows gained the §14.8.3 power-capture
    gate; §4.5 now names the implemented `heldout_logreg_vlda` baseline; SmolVLA
    cited; perturbation-library logging rules for SSI/H3 added.
  - **Status/spec docs synced:** `REVIEW_AND_TODO.md` (P0–P2 done-annotations, new
    2026-07-05/06 pass section, MIT→dual-license fix, meshmaker-quarantine status,
    CI-dead-period note); `RESEARCH_VLA_D_NCP.md` (observer behavior updated to
    post-v10.6 reality: absent-L excluded not zeroed, `d_source` set corrected);
    `crates/ncp-observer/README.md` (`-p` → `--manifest-path` gate description;
    "internal simulation" downgraded to "world-model status untested — what the
    probe would test"); `NCP_DEV_PROMPT.md` (+2 hard constraints: no D claims
    without a §7.6.3-style probe; exclusion-over-backfill for absent L);
    `DIAGRAMS.md` (M0–M8 roadmap realigned to grandplan §A.7 — M0=Exp0 gate,
    M7=Flow_pred, M8=Tauri UI; duplicate `Vis` mermaid node id split; H6/H7 nodes
    relabeled); `ARCHITECTURE.md` (§4 table un-broken — a prose paragraph sat mid-
    table; "validated ISX" → NO-GO-aware wording; H6/H7 rows; 3DGS claim hedged);
    `pidsplatspecs.md` (v10.7 stamp; per-timestep synergy comment corrected to
    windowed-ensemble; Zenoh dual-license); `GAUSS_MI_INTEGRATION.md` (v10.7 stamp;
    weighted-KSG formula marked "heuristic ansatz, not a derived estimator";
    SparkJS next-step rewritten Rerun-first; submodule-rule note on the module
    tree); `WORLD_WARP_INTEGRATION.md` (v10.7 stamp); `uidesigner/UI.md` (v10.7
    stamp; GauSS-MI screen unmislabeled from M8 — it is an optional add-on, not a
    milestone; Phase-4 scope note; mock badge honesty).
  - **pid-rs markdown sweep** (upstream commit `d662b11`): SECURITY 0.4.x, scripts/
    README consumer rename, dead v0.1.0 changelog link, PR-template accuracy,
    AGENTS test-map completeness. All other pid-rs files verified clean, including
    a byte-for-byte check of AGENTS.md's expected example output.

## Docset v10.6 (2026-07-05)

Whole-repo review + correctness/robustness slice. **No research/estimator/experiment
status change** — Exp0 still reports PIVOT/NO-GO on synthetic high-d controls and the
open critical path is still the first real-VLA capture (not done). The discrete-harness
measure identity is unchanged (`--pid-mode discrete` remains Williams–Beer `I_min`).

### Fixed (CI / release-gate integrity)

- **CI had silently not run since 2026-06-13.** `.github/workflows/ci.yml` contained an
  unquoted `physics:: manipulation::` scalar (a `": "` sequence is illegal in a YAML
  plain scalar), so GitHub failed to parse the workflow and created **zero jobs** on
  every push — no fmt/clippy/test, no cargo-deny, no notices-drift check, no docs audit
  had executed for three weeks. The line is now quoted and the whole file parses. The
  `python-bindings` job also now creates a venv before `maturin develop` (which refuses
  to run outside one).
- **Third-party notices refreshed.** `THIRD_PARTY_NOTICES.generated.md` was stale after
  the pid-rs 0.1.0→0.3.0 adoption (listed `pid-core`/`pid-runlog` as 0.1.0); regenerated
  to 0.3.0 so the CI `--check` gate passes.
- **Docset audit gate green.** `arXiv:2306.02149` (Makkeh et al.) is now in
  `outputs/arxiv_ref_cache.json`, so `just docs-audit` / the CI docs job pass; two
  paper-reported figures in `RESEARCH_VLA_D_NCP.md` are now explicitly qualified.

### Fixed (correctness / robustness — Rust)

- **Offline harness run logs stayed valid at capture scale.** `ArtifactLogged` /
  `ErrorLogged` / `RunEnded` were stamped at fixed `metric_base + 10_000/19_900/20_000`
  offsets while metric-event timestamps grow one-per-event; past ~10,000 metric events
  (≈470 held-out samples) the fixed offsets were overtaken and the emitted run log failed
  its own `pid-runlog-replay --validate`. Timestamps now continue from the running
  metric-event counter. New test at ~12,000 events.
- **Agent Bridge `export.rerun` can no longer overwrite the live run log.** The handler
  wrote any client-supplied `output_uri` (`save_recording` truncates its target), so a
  socket client could destroy the session's own source-of-truth run log or any writable
  file. It now rejects an `output_uri` that resolves to the session's run log and
  requires a `.rrd` extension.
- **JSON-RPC ids honor the spec.** `BridgeRpcRequest.id` was typed `String`, so a numeric
  id (`"id": 1`, the most common client convention) was rejected as a parse error with a
  fabricated id. Ids are now `String | Number | Null`, echoed verbatim; structured ids →
  −32600, unparseable JSON → −32700 (id null), unknown method → −32601.
- **Rejected/malformed RPC traffic is now audited.** Parse/id/unknown-method failures
  previously never reached the run log; they now emit a failed `bridge_response` so the
  control-plane audit trail is complete.
- **Transports always seal the run log.** `stdio`/`tcp`/`ws` bridges skipped
  `finish_run` on any transport error, leaving a run log with no `run_ended` (fails
  validation). They now finalize with `RunStatus::Failed` on error. `dispatch_rpc_lines`
  also flushes per response (interactive clients no longer deadlock) and caps line length.
- **`sim.reset` after `sim.step` is refused** (it would regress run-log step/time and fail
  validation); **`verify_flow_gt`** was rewritten to pair flows with snapshots in stream
  order (a post-step intervention's second snapshot no longer causes false "flow mismatch"),
  and now rejects a non-finite/negative tolerance; **`sim.step`** rejects a non-numeric `dt`
  instead of silently substituting 0.1; **`--safe-mode`** with the path omitted no longer
  runs with safe mode off.

### Changed (NCP integration — ncp-observer brought up toward the M5 bar)

- **Emitted run log now validates.** `RunEnded` used `timestamp_ns: 0` after nonzero
  sample timestamps (violating the nondecreasing rule) and zero-dim `EmbeddingContract`/
  `EmbeddingCaptured` for absent L/D (schema errors). The observer now rides a monotonic
  clock, defers the contract to the first fully-populated sample, and **excludes empty-axis
  ticks** (counted) so the artifact passes both `pid-runlog-replay --validate` and
  `pid-offline-harness`'s `validate_dataset`.
- **Dataset artifact is registered** in the run log at finalize (`ArtifactLogged` with
  sha256), so the source-of-truth log can locate and verify the data (NCP_DEV_PROMPT hard
  constraint 1). The dataset writer is now flushed explicitly.
- **D reordering handled in-repo.** A reorder grace window lets a seq-stamped observation
  that arrives after its tick's command still claim its own tick (the likely ordering,
  since the action plane outranks the observation plane in QoS); a later-still readout
  patches the sample (`d_source = "seq_late"`). Provenance markers are mirrored into the
  run-log events.
- **Session/seq resets are safe.** Eviction is now FIFO (insertion-order), not lowest-key
  (which starved new low-seq ticks after a reset); a reset starts a new epoch with cleared
  state (no cross-epoch pairing) and epoch-qualified `sample_id`s. `EmbeddingCaptured`
  append no longer uses `let _ =`.
- **Clean shutdown.** `ncp-observe` finalizes on SIGTERM/SIGHUP too (not just SIGINT), so
  `docker stop` / systemd no longer lose an entire capture; undecodable frames are counted
  and reported. All exclusion/eviction paths surface in an `ObserverStats` finalize report.

### Fixed (correctness / robustness — Python + pid-rerun)

- **SAFE adapter step-count guard was dead code** (`a.shape[0] != n` compared a value to
  itself); it now validates the hidden-state-derived V/L/D against the action count and
  raises a clear per-episode contract error instead of silently truncating or dying with
  an `IndexError`.
- **`canonical_hash` matched Rust for non-ASCII** — `json.dumps` defaulted to
  `ensure_ascii=True` (escaping to `\uXXXX`) while serde_json writes raw UTF-8, so a
  non-ASCII config value would diverge and fail sidecar validation; now `ensure_ascii=False`.
- **Test `test_redundancy_rejects_unvalidated_hyperbolic_metric`** expected `RuntimeError`
  but pid-rs 0.3.0's bindings map `InvalidConfig` → `ValueError`; updated.
- **pid-rerun**: dropped the dead `hdf5`/`hdf5-data` optional dependency + feature (unused,
  abandoned upstream, dragged a duplicate ndarray into the lockfile); nanosecond timestamps
  now convert via `Duration::from_nanos` (no epoch-scale precision loss); the `.npy` dtype
  check is exact (`'descr': '<f8'`, not a substring that matched structured dtypes); relative
  attribution `artifact_uri`s resolve against the run-log directory and an unreadable
  artifact is surfaced as a WARN, not silently dropped; the LIBERO/OXE loader claim in
  `data.rs` (no such loaders exist) was corrected.

## Docset v10.5 (2026-07-01)

Repo-wide consistency + robustness slice. **No research/estimator/experiment status
change** — Exp0 still reports PIVOT/NO-GO on synthetic high-d controls and the open
critical path is still the first real-VLA capture (not done). The discrete-harness
**measure identity is unchanged**: `--pid-mode discrete` still uses the Williams–Beer
`I_min` functional (`discrete_pid.rs`), not discrete `i^sx_∩`.

### Integrations

- **pid-rs submodule advanced from `v0.2.0` to the 0.3.0 line** (adopt current upstream).
  NB: the pin is the commit `7e8f16d` — one commit past the `release: 0.3.0` commit, whose
  workspace version is `0.3.0`; upstream has **not** cut a `v0.3.0` git *tag* yet, so
  `git describe` reports `v0.2.0-22-g7e8f16d`. The crate version, not a tag, is 0.3.0. 0.3.0
  adds a *genuine* discrete shared-exclusions PID (`sxpid.rs`: `discrete_sxpid2` /
  `discrete_sxpid3`, plus n-source SxPID [sic — 2–4 sources; see v10.7]), numerical-stability hardening across
  the estimators, criterion benchmarks, and a run-log `logical_trace_hash` (wall-clock-
  excluded logical hash). prisoma's `crates/pid-rerun` was updated for the new
  `pid_runlog::RunManifest.logical_trace_hash` field; the whole workspace + the excluded
  `ncp-observer` build and test green against 0.3.0. NB: the new discrete `sxpid.rs`
  estimator is **not yet wired into the offline harness** (`--pid-mode discrete` remains
  `I_min`); wiring it in is tracked follow-up (see `grandplan.md` §8.1.6).
- **NCP observer pin doc propagation `v0.5.0` → `v0.5.2`.** The manifest/lockfile were
  bumped to NCP `v0.5.2` earlier; the doc set now matches. `v0.5.1`/`v0.5.2` are
  wire-identical patch bumps on the 0.5 stable wire cut at `v0.5.0`, so `NCP_VERSION`
  stays `0.5` and `CONTRACT_HASH` stays `24e8e6e31e1dec8a`. Read-only data-plane tap;
  no behavior change.

### Fixed (correctness / robustness)

- **`ncp-observer`: success label no longer leaks into V.** When a `success_channel`
  is configured (the `ncp-observe` binary sets `success`), that channel was being
  flattened into the V feature vector *and* emitted as the outcome label, so any
  PID(V;A) screen would have measured the label. V now excludes both the language and
  the success channels. New test `success_channel_is_excluded_from_v`.
- **`ncp-observer`: bounded in-flight state.** `pending` and `d_by_seq` are now capped
  (`MAX_INFLIGHT`, oldest-`seq` eviction) so a long-running tap with never-completing
  `seq`s cannot grow memory without bound. New test `inflight_maps_are_bounded`.
- **`ncp-observer`: run-log integrity + CLI robustness.** `emit_runlog` no longer
  swallows `RunLogWriter::append` errors with `let _ =` (a dropped "source of truth"
  event is now reported); `RunStarted` uses `pid_runlog::RUN_LOG_SCHEMA_VERSION` instead
  of a hard-coded `1`; the arg parser advances unknown flags by one (no longer swallows
  the next real flag) and reports a value-less trailing flag; `finalize` recovers a
  poisoned mutex instead of panicking away every buffered sample.
- **`pid-rerun`: episode outcome timestamp.** `log_episode` logged the success/failure
  annotation at the *relative* `duration()` on an absolute timeline; it now uses the
  failure time or the last frame's absolute timestamp. 3D point logging
  (`log_frame`/`log_flow`/`log_ghost_splat`) is panic-safe on positions arrays with
  fewer than 3 columns (`row_to_point3`).
- **`pid-sim`: silent held-out void surfaced.** A single sample with a missing/unrecognized
  `split` value voids the entire held-out analysis; the harness now warns (in non-strict
  mode) instead of dropping all held-out metrics silently.

### Docs (consistency / honesty)

- `THIRD_PARTY_NOTICES.md` now states the project is **dual-licensed MIT OR Apache-2.0**
  (crates declare `license = "MIT OR Apache-2.0"`), correcting a stale "MIT only" notice.
- `ncp-observer` honest provenance docs: the absent-language path emits an **empty** L
  vector (not a fixed-dim "all-zero" vector as the comments claimed); the
  `--require-axis-provenance-honest` gate doc now states it checks the *stamped* axes and
  does not (yet) require all four to be independently attested. Both are pre-existing
  behaviors now accurately documented (Gap 2 / per-axis coverage remain tracked follow-ups).

### Known follow-ups

- arXiv `2306.02149` is referenced in `grandplan.md` but not yet in
  `outputs/arxiv_ref_cache.json` (refresh with
  `python scripts/update_arxiv_ref_cache.py --ids 2306.02149`; needs network).
- `ncp-observer` fixed-dim zero backfill for absent-L, and a 4-axis provenance-coverage
  gate, remain behavioral follow-ups (docs updated to match current behavior).

## Docset v10.4 (2026-06-22)

- Naming: repo/package/crate renamed `pid_vla` → `prisoma` (complete repo-wide; no
  `pid_vla` references remain). No wire or behavior change. Research/estimator/experiment
  status is unchanged from v10.3: Exp0 still reports PIVOT/NO-GO on synthetic high-d
  controls, and the open critical path is still running on real downloaded VLA data.

- Science-honesty: `--require-axis-provenance-honest` ENFORCES axis provenance, not
  just reports it (2026-06-21). The offline VLDA harness already surfaced per-axis
  provenance status; this adds an opt-in gate (mirroring `--require-geometry-pass`)
  that FAILS the run if any V/L/D/A axis is `degraded` (`text_hash_proxy` /
  `absent_zeroed` / `recency_fallback` …) — and, crucially, if NO provenance markers
  were stamped at all (positive attestation: honesty cannot be vacuously passed on a
  dataset that carries no provenance). Threaded through `OfflineVldaRunlogOptions`
  (recorded in the run-log as `strict_axis_provenance_honest`), the bin arg
  parser/usage, and the reproducible `just safe-adapter` recipe (the honest synthetic
  SAFE dataset passes; a provenance-stripped copy fails). New unit test
  `axis_provenance_gate_fails_on_degraded_and_on_absent_markers`.

- Science-honesty: the offline VLDA harness now surfaces `safe_adapter` axis
  provenance (2026-06-21). `axis_provenance` previously recognized only the live
  `ncp-observer` markers (`l_source`/`d_source`); running the offline `safe_adapter`
  pipeline end-to-end (synth→convert→verify→`pid-offline-harness`) exposed that its
  samples carry `{v,l,d,a}_provenance` instead (`token_slice:*` / `hidden_state_pool`
  / `action_vector` = honest; `text_hash_proxy` = hash surrogate for a missing real
  feature = degraded), so the report came back with `axis_provenance: []` and a PID
  atom computed from a hash-proxy axis would have been reported as trustworthy. The
  harness now recognizes both capture conventions (extended `MARKERS` + a shared
  `DEGRADED_PROV` value set); on the honest synthetic SAFE dataset all four axes report
  `status=ok` with their `token_slice:*`/`action_vector` sources. New unit test
  `axis_provenance_recognizes_safe_adapter_markers`.

- Re-pinned `crates/ncp-observer` to NCP `v0.5.0` (wire `0.4` → `0.5`, the stable-wire
  cut: the command/sim `mode` strings are now proto enums (`Mode`/`SimMode`) and
  `CONTRACT_HASH` was recomputed `2cf0763ad61e4f1c` → `24e8e6e31e1dec8a`). The observer
  is a read-only data-plane tap (no session handshake) and the JSON wire is unchanged
  for known values, so the bump is the `ncp-core`/`ncp-zenoh` git tag + its standalone
  `Cargo.lock` and the doc pins; `ncp-observer` builds against the v0.5.0 tag and its
  tests pass unchanged.

- Science-honesty: `ncp-observer` per-sample provenance markers (2026-06-21). Every
  emitted `(V,L,D,A)` sample now carries `metadata.l_source` and `metadata.d_source`
  so a degenerate axis is never silently presented as real data. `l_source` is
  `"channel"` when the language channel was present or `"absent_zeroed"` when `L` is
  the fabricated all-zero vector (NCP_DEV_PROMPT Gap 2); `d_source` is `"seq"`
  (exact alignment), `"recency_fallback"` (publisher sent `obs.seq == 0`, so `D` is
  the most-recent readout rather than the driving one — Gap 1), or `"absent"`.
  Downstream can now filter on these; combined with the harness degenerate-axis gate
  below, a zeroed `L` cannot pass unflagged. New test
  `provenance_marks_recency_fallback_and_present_language`; `d_aligns_on_seq_not_recency`
  extended to assert the markers.

- Science-honesty: degenerate-axis geometry gate (2026-06-21). The offline VLDA harness
  now flags any variable whose every dimension is zero-variance (all-constant) as a
  geometry-gate warning, reusing the already-computed-but-previously-unused
  `zero_variance_dims`. An all-constant axis has zero variance, hence zero mutual
  information with anything by construction, so every PID atom involving it is degenerate
  rather than merely small — this is exactly the fabricated all-zero `L` case
  (`NCP_DEV_PROMPT.md` Gap 2, where an absent language channel `unwrap_or_default()`s to
  zeros). The warning sets the gates to `warn` (and so fails `--require-geometry-pass`),
  satisfying Gap 2's acceptance criterion that "the harness can filter on it." New unit
  test `geometry_gates_flag_all_constant_variable_as_degenerate`.

- Docset v10.3 (2026-06-13) — capture + analysis implementation slice. (1) **Exp0 uncertainty gate**: opt-in `--bootstrap`/`--permutation` on the Exp0 binary run subsample-bootstrap CIs (Politis–Romano, KSG-safe) and single-source permutation nulls, with a preregistered ground-truth marginal-significance check folded into the GO/PIVOT/NO-GO verdict (8/8 on the synthetic scenarios); `pid-core` gains `bootstrap_rows_stats` + `permutation_rows_pvalue`; default runs stay byte-identical (closes `REVIEW_AND_TODO.md` P0 item 1 end to end). (2) **Real Rapier3D backend**: the fake `RapierBackend` stub is replaced with a real `rapier3d-f64` pipeline (gravity/contacts/friction, deterministic), plus a scripted push-to-goal manipulation (`crates/pid-sim/src/manipulation.rs`) emitting canonical run-log events with real `Flow_gt` and physics-derived success labels (`pid-rapier-harness`; new `rapier` CI job). (3) **SAFE-class failure detector**: `pid-core` `logistic.rs` (Newton-IRLS L2 logistic regression) wired as the leakage-safe `heldout_logreg_vlda` offline-harness baseline (H1); fixture evaluation-metric counts move 139→142 / 220→223. (4) **M5 SAFE capture adapter** (`experiments/safe_adapter/`): converts released SAFE rollouts to the `(V,L,D,A)`+labels contract with honest provenance and the §7.6.3 layerwise physics-probe for `D_hidden[k]` selection; verified end to end into the real harness with all leakage gates passing. (5) **H9 attribution probe** (`experiments/attribution/`): epsilon-/AttnLRP + gradient×input, deletion-AOPC faithfulness check vs a random control, and schema-conformant `attribution_logged` run-log emission validated by `pid-runlog-replay`. (6) **Release governance**: `meshmaker/` quarantined out of tracking (non-destructive `git rm --cached` + tombstone README; `.gitignore` hardened); `scripts/generate_third_party_notices.py` generates a direct-dependency SBOM with a CI drift check. New `experiments` CI job cross-validates both Python pipelines against the real Rust binaries.

- First-principles review + correctness pass (2026-06-16). A multi-agent, adversarially-verified review of every crate + the Python experiments produced 18 confirmed findings, all fixed and verified (clippy/fmt clean; full test suite incl. `rapier`/`parallel` features; Python suites; all CI metric greps). Headline: **CI was red** — the new `ncp-observer` crate, a default workspace member, path-depends on the sibling `Paper2Brain/ncp` tree (absent on a fresh checkout), which fails manifest resolution for *every* cargo command; it is now **excluded from the default workspace** (build via `--manifest-path`), regenerating `Cargo.lock` without the Zenoh tree, and its two clippy lints + formatting are fixed. Correctness fixes: `discrete_pid` histograms now key on the bin vector instead of a base-`num_bins` integer that silently overflowed `usize` in high dimension (collisions corrupting every discrete entropy/MI/PID value; debug panic); the `.npy` reader uses checked arithmetic so a crafted shape returns `None` instead of aborting in `Vec::with_capacity`; the PLS LOO-CV no longer leaks the held-out target (`PlsProjector::predict` via the proper `B = W(PᵀW)⁻¹Cᵀ` regression, correct for k≥2); `block_bootstrap`/`_paired` are now a moving-block bootstrap (Künsch 1989; no tail-drop/grid-bias) with the correct citation; the redundant `cmi_violations` Exp0 gate (always equal to the monotonicity check) is dropped (7 gate metrics now, `pid_metrics` 8→7); `ksg` xblocks rejects non-Chebyshev metrics; the synthetic-data LCG shift is fixed (`>>33`→`>>32`); `pid-runlog-replay` prints the `attributions=` count; `PhysicsStepReport.timestamp_ns` accumulates per-step dt; the attribution faithfulness check is made statistically honest (seeded tie-breaking + a 3-standard-error threshold so an uninformative attribution reliably fails); plus geometry doc-drift / `Pcg32::next_u32` renaming and dead-code cleanup.
- Data-source boundary documented + NCP dev handoff (2026-06-16). Clarified across the docset (`README.md`, `EXPERIMENTS.md` §0.2.1, `AGENTS.md`, `grandplan.md` §11.4, `crates/ncp-observer/README.md`) that the analysis is **source-agnostic** over one `(V,L,D,A)`+labels contract: the **critical-path producer is `experiments/safe_adapter`** (M5), the sim harnesses are standalone cross-checks, and **`ncp-observer` (Engram/NEST) is an optional, non-critical-path bridge** — grandplan does not depend on Engram, and the pure-PID stack builds/gates green with no NCP/Engram/Zenoh dependency. NCP is exploratory-only (below the M5 contract) until three gaps close — precise D `seq`-alignment, honest (non-zeroed) `L`, and `metadata.split`/`episode_id`/`success` structure for the strict gates and the §14.1.1 H1 audit. Added `NCP_DEV_PROMPT.md`, a self-contained developer handoff for that work. Also reaffirmed that **D is the hidden-state / dynamics axis, not depth** (consistent across SAFE and NCP).

- External-source review addendum (2026-06-13): reviewed dimos / DimensionalOS (`dimensionalOS/dimos`, Apache-2.0 agentic multi-robot control OS) source-by-source and recorded it in `grandplan.md` §12.5 + §13.10 + the §11.4 interoperability note — integrated narrowly as independent external validation of the Agent Bridge one-control-plane / record→replay / every-action-a-run-log-event architecture (§A.7/§A.8/§11.4) and a concrete external-backend adapter target (LCM as another M6 typed-stream transport alongside Zenoh/ROS 2); ruled out as an M5 capture shortcut (no internal-activation or success/failure-label extraction per README → SAFE still dominates for `(V,L,D,A)`+labels) and as a PID/IT method contribution.

- Final research/improvement pass: restored CI parity (rustfmt on the offline harness; clippy/ruff/pytest verified locally), fixed the duplicate `grandplan.md` §2.5.4 header (→ §2.5.5), recorded a dated novelty check in §12.5 (no published PID-on-VLA application found; standing related-work check), identified the released SAFE rollout datasets/pipeline (`vla-safe/SAFE`) as the concrete M5 capture shortcut (README step 4, `EXPERIMENTS.md` §0.2, `REVIEW_AND_TODO.md` critical path), and added a hedged §13.1 pointer to the reported 2026 Wibral–Makkeh multivariate PID paper.

- Self-sufficiency pass on the entry-point docs: `README.md` gains a "Current Status & What To Do, In Order" section (status paragraph + 7 gated steps with commands and expected outcomes), status + real-robotics-problem columns on the hypotheses table, per-experiment runnable-today/blocked annotations, and `findings.md`/`REVIEW_AND_TODO.md` in the doc map; `EXPERIMENTS.md` gains a §0.2 executable-vs-blocked runbook table and an updated geometry-decision-tree note pointing at the implemented discrete PID modes; `findings.md` notes the harness `--pid-mode` wiring of both escape hatches with saturation diagnostics.

- Third v10.2 research batch: integrated World Pilot (arXiv:2606.12403; separable WAM steering pathways as model-native Exp4/H3 interventions), the video-world-model physics-interpretability study (arXiv:2602.07050; "Physics Emergence Zone" hook-point prior in `grandplan.md` §7.6.3), and the GEN-1 vendor post (black-box comparator; embodiment-gap instance); added a "Real robotics problem addressed" column and failure-regime framing to the hypothesis registry (§14.1.0); aligned `EXPERIMENTS.md` H2/H5 rows with the §14.1.1 necessity audit (graded-language probe option; mandatory CI-only ablation); appended a dated 2026-06-12 implementation/status section to `REVIEW_AND_TODO.md` marking block bootstrap (P0.1 item) and the discrete/quantized PID fallback (P2.11) as shipped and naming the first real VLA capture as the critical path; refreshed README repo-status wording and the arXiv cache.

- Research integration pass (docset v10.2, 2026-06-12): added `grandplan.md` §12.5 external-source review (integrated: Qwen-VLA, V-JEPA 2/-AC + a latent-predictive world-model taxonomy class, NVIDIA Cosmos 3 "World Action Model" notes, RoboLab-120 benchmark context, π0.7, AttnLRP, GSWorld, Zenoh-in-production via ROBOTIS Cyclo, world-model surveys; ruled out with citations: ProjectEdenGG, Trajectory, a DAIR.AI aggregator week, unverifiable X posts), §14.1.1 per-hypothesis PID-necessity audit with preregistered kill criteria, §14.7.1 concrete AttnLRP/LRP protocol for transformer VLAs, the embodiment-in-`L` confound (§14.5.7.3), LIBERO-saturation caution, and new D-candidate rows (Qwen-VLA, V-JEPA 2-AC, Cosmos3-Nano-Policy); refreshed the arXiv reference cache accordingly.

- Completed the discrete/PLS pipeline slice: discrete 3-source PID (`discrete_pid3`, `I_min` over the 18-antichain lattice), `pid-core` `pipeline.rs` composition helpers (`pls_project_then_pid3`, `pls_project_then_discrete_pid3`, `bootstrap_pid3` per-atom CIs, `permutation_pid3` single-source nulls, `pls_cv_select_components`, `screen_pid2_pairs`), offline-harness `--pid-mode continuous|discrete|discrete-pls` with `--discrete-bins`/`--pls-components`, per-pair `discrete_saturation` diagnostics implementing the grandplan §8.1.6 saturation gate, `I_min`-correct naming for the discrete redundancy (`discrete_imin_redundancy*` — Williams–Beer-style, not discrete `i^sx_∩`), provenance `pid` values `discrete_imin`/`pls_discrete_imin`, a crate `recursion_limit` fix, new `just offline-harness-discrete`/`offline-harness-discrete-pls` recipes, and unit/smoke coverage for all three PID modes.

- Docset v10.2 (2026-06-12): synced `grandplan.md` with the 2026-06-11 implementation slice (PLS, discrete PID, block bootstrap, physics stub, `attribution_logged`, `--strict-gate`, high-dim fixture, 14 Python bindings); added `grandplan.md` §8.1.6 documenting that the implemented discrete redundancy is a Williams–Beer-style `I_min` functional (not discrete `i^sx_∩`) with a saturation/occupancy gate, binning-sensitivity requirements, and a cross-measure extension of Warning 6; added supervised-projection (PLS) guidance with leakage rules (§8.2, §17.5.3); added action-chunking to the `V/L/D/A` contract (§10.10.13.1); resolved the OpenVLA-OFT citation placeholder (arXiv:2502.19645); refreshed DreamVLA/π0/GR00T status notes (hedged); added MI-estimation and PID references (arXiv:2410.10924, 2506.00330, 2409.13506, 2502.04550, 2506.18498, 2602.10098, 2603.19233); corrected the discrete-redundancy naming in `AGENTS.md`/`findings.md`; updated Rapier status wording in `ARCHITECTURE.md`/`pidsplatspecs.md`; bumped docset alignment markers to v10.2.

- Added PLS supervised dimensionality reduction (NIPALS-PLS2) to `pid-core` (`pls.rs`) addressing the key finding that unsupervised projections fail when signal variance ≈ noise variance; added discrete PID via quantization (`discrete_pid.rs`) as a kNN-geometry escape hatch; added block bootstrap uncertainty quantification (`bootstrap.rs`); expanded Python bindings from 8 to 14 functions (pid3, discrete_pid2, pls_transform, standardize, pca_transform, hash_project); added `attribution_logged` event to the run-log schema (H9 attribution probe provenance); added `PhysicsBackend` trait with null adapter and Rapier3D skeleton (behind `rapier` feature) in `pid-sim`; added `--strict-gate` flag to Exp0 binary (exit code 3 on non-GO); added high-dimensional synthetic VLDA fixture (v=128, l=64, d=32, a=7, 48 samples); strengthened meshmaker quarantine in `.gitignore`; added `just offline-harness-highdim` recipe.

- Added the ten-scientist consensus decision record in `grandplan.md` §A.8: canonical run log as source of truth, Rerun as Phases 1–3 diagnostic/time-machine viewer, Agent Bridge as the only control plane, and Tauri/SparkJS as the deferred Phase 4 control/editor/custom-rendering shell.
- Aligned first-party Markdown docs to the same Rerun/Tauri/SparkJS/licensing decision and clarified that optional live transport, external world models, GauSS-MI, and generated assets must emit canonical run-log artifacts.
- Updated the simulation spec license table for the checked Rerun/Tauri/SparkJS/Rerun WebViewer/Three.js package metadata and added release-governance reminders for local crate licenses, third-party notices, and model/data/asset audits.
- Added `crates/pid-runlog` as the first M1 implementation slice: versioned JSONL event schema, reader/writer, SHA-256 helpers, deterministic replay summary, and `pid-runlog-replay` CLI.
- Added bounded follow-up slices for the 10-step plan: embedding/sim/bridge run-log events, replay hash comparison, `crates/pid-bridge`, `crates/pid-sim`, a run-log-to-Rerun adapter/CLI, `just` smoke recipes, and `THIRD_PARTY_NOTICES.md` release-governance groundwork.
- Added the next validation/dispatch slice: run-log structural validation, payload-hash/monotonicity checks, CLI `--validate`, bridge dispatcher abstraction, sim bridge handler/session, `pid-sim-bridge-demo`, `Flow_gt` verification helpers, and CI run-log smoke commands.
- Added the next provenance/API slice: compact run-log summaries, manifest JSON generation, JSON-RPC-shaped bridge request conversion, `pid-sim-verify`, stricter run-log validation before Rerun conversion, and CI smoke coverage for summary/manifest/flow verification.
- Added canonical `evaluation_metric`, `label_observed`, and `embedding_contract` run-log events plus a deterministic toy VLA/task harness with success labels, a replay-linked toy `(V,L,D,A)` contract, PID/CI features, non-PID baseline metrics, summary JSON, canonical run-log export, `just toy-harness`, and CI validation smoke.
- Added Agent Bridge read-only safe-mode metadata/enforcement plus a stdio `--safe-mode` smoke path that logs blocked mutating requests as bridge error responses.
- Added deterministic sim backend/solver provenance config logging for sim demo, bridge demo, and stdio bridge run logs, with validation that logged `config_hash` values match canonical config JSON.
- Tightened replay/provenance gates: run-log validation now checks `run_started`/`config_logged` hash consistency, summaries/manifests expose `config_hash`, replay compare exits nonzero on mismatches, and the sim bridge implements safe-mode `log.replay`.
- Added a loopback TCP JSON-RPC Agent Bridge transport (`pid-sim-bridge-tcp`) for the deterministic sim, with canonical run-log emission and CI validation/replay smoke coverage.
- Added first-class `flow_pred` run-log events plus deterministic constant-velocity baseline predictions for sim run logs, replay summaries, Rerun conversion, and CI smoke assertions.
- Added a generic offline `(V,L,D,A)` embedding harness (`pid-offline-harness`) with checked fixture input, schema validation, PID/baseline metrics, canonical summary/run-log export, `just offline-harness`, and CI validation smoke.
- Added a localhost WebSocket JSON-RPC Agent Bridge transport (`pid-sim-bridge-ws`) with RFC6455 handshake/frame handling, canonical run-log provenance, `just runlog-bridge-ws`, bridge contract transport coverage, and CI smoke validation.
- Implemented Agent Bridge `export.rerun` for validated run logs, including `.rrd` artifact logging, safe-mode blocking, stdio/WebSocket smoke coverage, and `just runlog-bridge-export-rerun`.
- Implemented the remaining advertised deterministic sim bridge lifecycle/intervention methods: `log.start`, `log.stop`, and deterministic `intervention.apply` (`set_velocity`, `translate_object`, `set_pose`), with run-log finalization gates, intervention replay verification, and stdio/TCP/WebSocket smoke coverage.
- Strengthened the offline `(V,L,D,A)` embedding harness with deterministic leave-one-out 1-NN success-label baselines for raw `V`, `L`, `D`, `A`, and concatenated `VLDA`, emitted in both summary JSON and canonical run-log evaluation metrics.
- Added offline `(V,L,D,A)` preprocessing/geometry provenance: PID metrics now run in a deterministic per-variable standardized analysis space, summaries record standardizer hashes and geometry gate warnings, run logs emit first-class geometry metrics, and CI checks the geometry metric count.
- Added fail-closed offline geometry gating via `pid-offline-harness --require-geometry-pass` and `just offline-harness-strict`, which exits nonzero on geometry warnings while still writing a valid failed run log with provenance.
- Extended the offline `(V,L,D,A)` harness from a single `(V,L;A)` PID screen to all two-source `V/L/D→A` screens: `(V,L;A)`, `(V,D;A)`, and `(L,D;A)`, emitted in both summary JSON and canonical run-log PID metrics.
- Added leakage-resistant leave-one-episode-out success baselines to the offline `(V,L,D,A)` harness, emitted when all labeled samples carry `episode_id`, with run-log provenance for split/group key/classifier.
- Added fail-closed success-label enforcement via `pid-offline-harness --require-success-labels`, including valid failed run logs for unlabeled captures and CI coverage of the failure path.
- Added metadata-split held-out success baselines to the offline `(V,L,D,A)` harness, preserving train/held-out sample IDs in summaries/run logs and adding `pid-offline-harness --require-heldout-split` plus CI coverage for success/failure paths.
- Added train-standardized nearest-centroid held-out success baselines for raw `V`, `L`, `D`, `A`, and concatenated `VLDA`, giving the offline harness a deterministic learned non-PID baseline when the train split has both success classes.
- Added held-out balanced accuracy metrics for offline majority, 1-NN, and nearest-centroid baselines when both held-out success classes are present, reducing accuracy-only label-imbalance blind spots.
- Added held-out nearest-centroid AUROC metrics for raw `V`, `L`, `D`, `A`, and concatenated `VLDA`, using the train-standardized signed centroid-distance score so larger scores are more success-like.
- Added held-out per-sample prediction records to offline VLDA summaries, including majority/1-NN/centroid predictions, 1-NN nearest train sample provenance, and centroid discrimination scores for error auditing.
- Added held-out failure-class confusion/rate diagnostics for offline majority, 1-NN, and nearest-centroid baselines, exposing failure TP/FP/TN/FN counts plus precision, recall, specificity, and F1 in summaries and run logs.
- Added held-out class-coverage reporting and `pid-offline-harness --require-heldout-class-coverage`, requiring both success and failure labels in train and held-out subsets for fail-closed offline harness runs.
- Added held-out episode-disjointness reporting and `pid-offline-harness --require-heldout-episode-disjoint`, preventing `episode_id` leakage across train and held-out splits.
- Added train-split-only offline VLDA PID screens with train-only standardization and explicit run-log provenance, so held-out embeddings are excluded from the PID evidence reported alongside held-out baselines.
- Promoted held-out offline VLDA per-sample prediction records into canonical run-log evaluation events, preserving correctness, score, distance, nearest-train, classifier, and sample provenance outside the summary artifact.
- Split replay metric summary semantics so existing `*_metrics` fields remain unique latest-by-name metric counts while new `*_metric_events` fields report total metric event volume, including repeated held-out prediction metrics.
- Added run-log sidecar verification so validation, summary, and manifest JSON sidecars can be checked against the current JSONL run log instead of silently going stale.
- Integrated LRP and related attribution methods into the research docset as H9 companion diagnostics/baselines, with sanity-check requirements and explicit separation from PID/geometry gates.
- Refreshed repo-wide documentation consistency notes: clarified planned-vs-implemented physics adapters, marked mesh generation reports as historical snapshots, added the full tracked-Markdown audit command, and removed stale v10.0/current-pass wording.

## 10.1 (2026-01-08)

- Clarified the v10.1 “Rerun-First” sequencing vs the Phase 4+ target UI stack (scope notes + reading guide) and tightened “spec-only vs implemented” labeling in `grandplan.md`.
- Fixed minor doc drift and numbering in the system-architecture blueprint portion of `grandplan.md` (e.g., §C.1/§C.2 ordering; asset→collision-proxy wording; pseudocode semantics for splat edits).
- Tightened verification language to be offline-friendly: referenced the local `outputs/arxiv_ref_cache.json` cache instead of “arXiv API”, scrubbed unverified latency placeholders in historical notes, and removed unverifiable license specifics (e.g., InternVLA‑A1).
- Added `scripts/audit_grandplan.py`, `scripts/audit_grandplan_claims.py`, and `scripts/update_arxiv_ref_cache.py` to audit arXiv coverage/title drift, scan for high-risk doc drift, and (optionally) refresh the local arXiv metadata cache used for offline verification.
- Updated `.gitignore` so `outputs/arxiv_ref_cache.json` can be tracked while keeping other `outputs/` artifacts ignored.
- Updated docset alignment markers across the documentation set to v10.1 where applicable (`README.md`, `DIAGRAMS.md`, `EXPERIMENTS.md`, `ARCHITECTURE.md`, `uidesigner/UI.md`, and optional module specs).

## 10.0 (2026-01-05)

- Integrated the optional GauSS‑MI spec across the docset: reconstruction uncertainty maps, uncertainty‑aware diagnostics/weighting (optional), and active view selection as confound controls (`grandplan.md` §C.2, `GAUSS_MI_INTEGRATION.md`, `DIAGRAMS.md`).
- Slimmed `README.md` to a brief entrypoint that links to the canonical spec/protocol docs and the engineering plan.
- Bumped docset alignment references from v9.0 → v10.0 across the documentation set.
- Added `uidesigner/UI.md` and `uidesigner/prompt_loop.py` to iteratively design the viewer-first UI (M1→M2→M4→M8) using gpt‑image (via FAL) + Gemini critique loops (Vertex AI), with artifacts saved per UI part.
- Fixed Mermaid syntax robustness in `DIAGRAMS.md` (sequence diagram note formatting; expanded multi-input edges) to improve rendering in common Mermaid toolchains.
- Added LuckyRobots/Lucky World as an emerging simulator comparator and distilled ML-first simulator lessons (RL-style `reset/step`, WebSocket control planes, external-backend adapters that still emit canonical run logs) across `grandplan.md`, `ARCHITECTURE.md`, `EXPERIMENTS.md`, and `DIAGRAMS.md`.
- Added Physical Intelligence PI “π” series (`π0`, `π0.5`, `π0.6*`) as a vendor/black-box VLA comparator with explicit “verify access + embeddings” caveats (`grandplan.md`, `EXPERIMENTS.md`).

## 9.0 (2026-01-05)

- Promoted an explicit v9.0 execution sequence (M0–M7) with acceptance criteria in `grandplan.md` (§A.7) so engineering can begin without re-interpreting the spec.
- Restructured `README.md` to lead with hypotheses + experiments, then map directly to the engineering build order (gate-driven, contract-first).
- Bumped docset alignment to v9.0 across `ARCHITECTURE.md`, `DIAGRAMS.md`, `EXPERIMENTS.md`, and `pidsplatspecs.md`, and clarified offline-first run logs + replay vs optional live transports (Zenoh).
- Added a multi-engine physics reality check: per-object mixed backends are a co-simulation problem; prefer one backend per run plus optional cross-backend replay as a robustness/confound control (`grandplan.md` §E.1).

## 8.0 (2026-01-05)

- Corrected SparkJS assumptions: documented SparkJS “Spark” as a Three.js-integrated WebGL2 3DGS renderer (with links), and made renderer requirements backend-agnostic (WebGL2/WebGPU) where appropriate.
- Clarified contacts/collisions in 3DGS-based simulators: updated SplatSim (PyBullet physics backbone) and DISCOVERSE (MuJoCo physics backbone) notes, and made PID‑Splat’s default collision path explicitly mesh/URDF/MJCF-driven (with splat-field collision heuristics treated as optional research).
- Updated hypothesis set: added **H8** (geometry gate → estimator regime choice), narrowed **H2/H3** into falsifiable ablation/intervention claims, and softened optional world-model extension hypotheses (H_WM1–H_WM5) to avoid pre-committed outcomes.
- Expanded model/flow survey: added SmolVLA to the VLA reference list and added RAFT (arXiv:2003.12039) as a non-generative flow baseline for `Flow_obs`.

## 7.0 (2026-01-05)

- Scientific audit pass across the docset: removed or downgraded unsourced performance/hardware/roadmap claims; switched to measurement-first language.
- Reworked `grandplan.md` VLA integration into a contract-first framing (`V/L/D/A` must be defined and logged per checkpoint; no assumed layer names/shapes).
- Added a risk-reducing execution sequence: Exp0 → harness bring-up with simulator-derived `Flow_gt` → small baseline (e.g., SmolVLA) → primary VLA (e.g., OpenVLA) → optional diffusion/predictor-driven Flow.
- Clarified H1 as “PID features ↔ failure labels” (synergy sign is a candidate feature, not a definition of hallucination).
- Added/updated Agent Bridge requirements (GUI and automation share one control plane; JSON-RPC/MCP; all interventions logged and replayable).
- Added OpenUSD/USDZ interop notes (LeIsaac Marble tutorial; `.ply → .usdz` via NVIDIA 3DGrut) as an optional workflow.
- Added InternVLA‑A1 as an optional diffusion/flow-matching VLA candidate for stage-wise ablations (with explicit license/verification caveats).
