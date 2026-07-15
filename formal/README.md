# Machine-checked logical obligations

These SMT-LIB models make a narrow set of repository claims executable. They are
checked with Z3 by `python scripts/check_formal_models.py`. The runner snapshots each
bounded regular model, permits only a small non-output-spoofing SMT command subset,
requires one registered result per `check-sat`, caps combined process output, and
enforces both solver and wall-clock deadlines.

The registered `unsat` results establish either an inductive invariant of an abstract
transition relation or the impossibility of a forbidden mathematical/publication state:

- `bridge_log_before_dispatch.smt2`: a handler call and an accepted response append each imply
  that the generic `LocalBridge` writer previously accepted the request append. Its separate
  satisfiable paths cover ordinary handler dispatch and a safe-mode response with no handler
  call. Writer acceptance is not flush or fsync;
- `receipt_last_publication.smt2`: the core NCP finalizer can reach a visible receipt only after
  successful dataset and run-log install steps, and the receipt was constructed from the digests
  of the prepared byte buffers. It deliberately does **not** model a reread of a new core install,
  because that code performs none. It also exposes a satisfiable post-install state in which the
  receipt path is visible but a final fsync/recovery step and successful return have not occurred.
  A second transition system captures the fault observatory's
  pre-commit snapshot verification, receipt-last install, post-commit reread/verification, and
  successful return. A satisfiable intermediate state records that mere receipt visibility is
  weaker than the observatory's verified-success return;
- `typed_outcome_publication.smt2`: all four local offline-harness computation-status tags are
  reachable under their contracts. Not-requested and abstained outcomes cannot carry a complete
  finite numeric payload, produced and produced-with-warning outcomes cannot omit one, and the
  required detail/code/gate conditions are enforced; and
- `shannon_two_source_identity.smt2`: in exact real arithmetic on the positive-MI domain, the
  two-source definitions algebraically imply \(\bar r + \bar v = 2\). A satisfiable ordinary
  premise check prevents a contradictory domain from making the proof vacuous.

The registered `sat` results make transition premises/status tags non-vacuous or require a
deliberate countermodel. The two scientific countermodels are:

- `pid_nonidentification.smt2` supplies two explicit nonnegative PID witnesses with
  \(I_1=I_2=1\) and \(I_{12}=3/2\), but different redundancy and synergy. A redundancy measure is
  additional structure, not a consequence of the three consistency equations; and
- `coupling_nonidentification.smt2` supplies two explicit valid joint probability tables with
  identical Bernoulli(1/2) marginals and expected disagreement 0 versus 1. A sampled
  paired-response distance therefore includes the coupling in its estimand.

## Assurance boundary

These are small mathematical models, not verified implementations. Their proofs apply only
if the model's transition guards correspond to the code; ordinary Rust and Python tests sample,
but do not prove, that refinement boundary. Boolean states such as `receipt_bound` and
`postcommit_verified` abstract successful code paths; the SMT solver does not implement SHA-256,
serialization, syscall, or filesystem semantics. In particular, these obligations do not prove
power-loss durability, protection from concurrent mutation, persistence of path contents after a
successful check, estimator consistency, floating-point identities, causal identification,
application validity, or any empirical hypothesis. The output-bounded runner protects CI from
several accidental or local-input failure modes; it does not authenticate the Z3 binary or make
the models proof-carrying artifacts. Formal satisfiability can rule out a logical implication or
prove an invariant of an abstraction; it cannot turn missing data or unvalidated assumptions into
evidence.
