# Machine-checked logical obligations

These SMT-LIB models make a narrow set of repository claims executable. They are
checked by `python scripts/check_formal_models.py`, which requires Z3 to report the
exact `Z3 version 4.16.0 - 64 bit` version string.
`model_registry.json` binds every model byte-for-byte to a SHA-256 digest, its ordered
results, and the solver version. The runner snapshots each bounded regular model,
parses complete top-level S-expressions, requires canonical single-level SMT-LIB
`(push 1)`/`(pop 1)` assertion-stack commands, permits only a small
non-output-spoofing command subset, requires one
registered result per `check-sat`, caps combined process output, kills the solver
process group on timeout or overflow, and enforces both solver and wall-clock deadlines.
CI downloads the official Linux x64 Z3 archive and verifies its published SHA-256
before use.

POSIX cleanup does not signal a process-group number after surrendering ownership of it.
When Python exposes `waitid`, the runner observes leader exit with `WNOWAIT`, signals the
group while the leader remains waitable, and only then reaps it. On supported POSIX Python
builds without `waitid`, a private live group leader anchors the PGID while the tool runs;
the tool is reaped, its anchored group is signaled, and the anchor is reaped last. Windows
retains leader-only termination. These controls cover ordinary inherited process-group
membership, not descendants that deliberately create a new session or process group, and
they are process-lifetime hygiene rather than a hostile-tool sandbox.

The registered `unsat` results establish either an inductive invariant of an abstract
transition relation or the impossibility of a forbidden mathematical/publication state:

- `bridge_log_before_dispatch.smt2`: a handler call and either typed response kind imply that
  the generic `LocalBridge` writer previously accepted the request append. Explicit normal and
  safe-mode states distinguish handler results from guarded rejections; a failed request append
  cannot transition to handler dispatch. Writer acceptance is not flush or fsync;
- `receipt_last_publication.smt2`: the core NCP finalizer can reach a visible receipt only after
  successful dataset and run-log install steps. Abstract byte and digest sorts structurally bind
  both receipt fields to the prepared byte buffers. This proves equality in the model, not
  collision resistance or SHA-256 implementation correctness. The core deliberately does
  **not** model a reread of a new install, because that code performs none. It also exposes a
  satisfiable post-install state in which the receipt path is visible but a final fsync/recovery
  step and successful return have not occurred. A second transition system captures the fault
  observatory's pre-commit snapshot verification, receipt-last install, post-commit byte equality
  and receipt binding, and successful return. A satisfiable intermediate state records that mere
  receipt visibility is weaker than the observatory's verified-success return;
- `typed_outcome_publication.smt2`: all four local offline-harness computation-status tags are
  reachable under their contracts. Each of the five Rust scientific-gate verdicts (`passed`,
  `conditional`, `not_evaluated`, `blocked`, and `not_applicable`) is explicit and reachable at
  the abstract publication boundary, and an interpretable produced state has an explicit
  satisfiability witness. Not-requested and abstained outcomes cannot carry a complete finite
  numeric payload or emit a numeric PID metric event; produced and produced-with-warning outcomes
  cannot omit one; and the required detail/code/gate conditions are enforced; and
- `shannon_two_source_identity.smt2`: in exact real arithmetic on the positive-MI domain, the
  two-source definitions algebraically imply \(\bar r + \bar v = 2\) and bound each normalized
  degree in \([0,2]\). A satisfiable ordinary premise check prevents a contradictory domain from
  making the proof vacuous; and
- `h3_full_population_fallback.smt2`: an exact same-fold baseline fallback has zero weighted
  paired loss improvement for every nonnegative target weight. Its counterexample shows that an
  average over PID-produced rows alone can nevertheless differ from the complete-ledger estimand,
  which is why H3 retains abstained candidates in the frozen target-population denominator; and
- `h2_paired_brier_bounds.smt2`: for fixed baseline and diagnostic probabilities in
  \([0,1]\), the baseline-minus-diagnostic Brier improvement for a missing binary outcome is
  enclosed by its two endpoint completions and remains in \([-1,1]\). A concrete nondegenerate
  pair attains both endpoints, so the rowwise interval cannot be narrowed without extra
  restrictions on the missing outcome. A final obligation proves closure under a two-block
  nonnegative weighted combination with positive total weight; repeated application yields finite
  nonnegative-weight aggregation by ordinary induction. The model deliberately holds predictions
  fixed and therefore says nothing about censoring assumptions used during fit.

The registered `sat` results make transition premises/status tags non-vacuous or require a
deliberate countermodel. The four scientific countermodels are:

- `pid_nonidentification.smt2` is narrowly an algebraic-consistency countermodel: it supplies two
  explicit locally positive decompositions
  with \(I_1=I_2=1\) and \(I_{12}=3/2\), but different redundancy and synergy. It does not prove
  that both decompositions arise from one globally defined redundancy functional on concrete
  probability laws. A redundancy measure is additional structure, not a consequence of the
  three local consistency equations; and
- `coupling_nonidentification.smt2` supplies two explicit valid joint probability tables with
  identical Bernoulli(1/2) marginals and expected disagreement 0 versus 1. A sampled
  paired-response distance can therefore depend on the coupling in its estimand; and
- `individual_effect_prevalence_nonidentification.smt2` supplies two valid binary
  potential-outcome joint laws with the same Bernoulli(1/2) control and treatment marginals and
  the same zero average treatment effect. One couples \(Y(0)\) and \(Y(1)\) identically and has
  zero prevalence of a nonzero individual effect; the other couples them antithetically and has
  prevalence one. Ordinary marginal arm laws therefore do not identify individual-effect
  prevalence without additional cross-world assumptions. This is why H4 is specified over
  baseline-observable regions and randomized region-average effects rather than latent per-unit
  effect labels; and
- `informative_censoring_nonidentification.smt2` is a finite-artifact missing-outcome
  countermodel, not a model of a censoring mechanism: it supplies two binary completions of the
  same eight-row observed H2 artifact, with two observed targets and two censored outcomes, whose
  target risks are one quarter and one half. A second obligation proves the conservative rowwise
  missing-outcome count bounds. It does not validate any conditionally independent censoring
  assumption or claim that episode-time structure cannot narrow the bounds.

## Assurance boundary

These are small mathematical models, not verified implementations. Their proofs apply only
if the model's transition guards correspond to the code; ordinary Rust and Python tests sample,
but do not prove, that refinement boundary. Boolean transition states still abstract successful
code paths. Uninterpreted byte/digest equalities prove only the binding relations asserted by the
abstraction; the SMT solver does not implement SHA-256, serialization, syscalls, or filesystem
semantics. In particular, these obligations do not prove power-loss durability, protection from
concurrent mutation, persistence of path contents after a successful check, estimator
consistency, floating-point identities, any positive causal-identification result, application
validity, or any empirical hypothesis. The output-bounded runner protects CI from several
accidental or local-input failure modes, but it does not impose a solver-memory limit. Outside the
pinned CI acquisition path, the version gate checks a solver's self-reported string and does not
authenticate its executable bytes. The pinned archive checksum authenticates the CI download only
to the published release checksum; it does not make the models proof-carrying artifacts. Formal
satisfiability can rule out a logical implication or prove an invariant of an abstraction; it
cannot turn missing data or unvalidated assumptions into evidence.
