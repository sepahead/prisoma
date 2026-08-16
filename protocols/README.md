# Living protocol ledgers

This directory contains machine-readable **current-state** protocol and ecosystem ledgers. They
do not replace `grandplan.md`, preregister a real experiment, or turn software fixtures into
scientific evidence.
PID functional identity, method applicability, research preservation, and publication levels are
governed by [`PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md`](../PID_METHOD_SELECTION_AND_PUBLICATION_CONTRACT.md).

- `world_model_claim_registry_v1.json` is the unfrozen v13.0 W1–W3 registry. W1 and W2 are the
  only proposed primary scientific claims. W3 is secondary or exploratory. The registry records
  the native exact-fork software reference, the unimplemented learned-model studies, every major
  freeze obligation, and strict permitted/prohibited language. It also binds the reviewed LeWM
  code lock, exact platform wheels, checkpoint digest, rights boundary, and CEM contract as the
  first external candidate. It records the one-seed, TwoRoom-only independent reproduction as a
  protocol-identity warning, not PushT or M4 evidence. Every unresolved paper, configuration, or
  executable-code reading must become a pre-outcome frozen arm. JEPA-WM remains the second planning benchmark. The registry is not a dependency
  declaration, preregistration, holdout authorization, M4 qualification, or result.
- `research_claim_registry_v1.json` maps EC1 and H1–H4 to their current executable artifacts,
  proof commands, blockers, and permitted claim language. Development/blinded-pilot nuisance and
  design parameters remain unfrozen instead of receiving invented values; minimum useful effects
  require separate domain and decision justification. H3 records population, measure,
  atom-estimator, continuous-application, and high-dimensional MI/coherence states separately. The
  current H4 software artifact is a reference-model deletion-ranking-sensitivity diagnostic only.
  It does not establish natural pathway use, mechanism, or attribution faithfulness. Its current-state date
  may advance beyond the historical
  intake bundle only when it content-binds a typed v3 successor with the same date and status.
- `m0_preregistration_skeleton_v1.json` is an **unfrozen** branch-separated H1-A/H1-B/H2/H3/H4
  historical scaffold. Its exact checked bytes and intake-time artifact hashes are preserved;
  those historical hashes are records, not assertions that mutable current fixture paths still
  contain the same bytes. It remains intentionally non-promotable.
- `m0_preregistration_successor_draft_v3.json` is a revised **typed successor draft**, not a
  preregistration or freeze candidate. The 2026-08-12 first-principles correction reopened its
  scientific and statistical review. The 2026-08-13 ancestry correction kept it open and
  unreviewed. It exact-binds the historical v1 bytes and adds null,
  freeze-required obligations for EC1's finite adapter/fault/oracle/acceptance design, including
  exact fault-adapter detection coverage, separately estimated absolute sensitivity floors, and
  mandatory passage without distribution-average rescue, plus per-adapter replay-fidelity and
  false-positive coverage; H1-A's typed response functional, proper score, matched-access
  comparator, positive useful margin, one-sided superiority rule, uncertainty, calibration
  acceptance and failure consequence, multiplicity, and finite-benchmark or replication scope;
  H1-B's typed effect-specific endpoint kind, single primary endpoint, hierarchy, positive-margin
  one-sided success rule, mandatory validation stack, ITT and design checks, uncertainty, and
  directional replication, with factual-outcome loss restricted to a secondary descriptive
  outcome-model diagnostic;
  H2's landmark, target, censoring, comparator, and one-primary-scoring-contract. That contract
  binds the prediction object, score, risk, censoring law, assumptions, and uncertainty. A
  forecast-independent censoring-adjusted horizon score can target scalar risk under its exact
  conditional-censoring and positivity assumptions. A right-censored likelihood requires a full
  event-time-and-type prediction object. It cannot silently replace a fixed-horizon risk score.
  The contract also freezes the complete competing-event ontology. The
  no-censoring branch requires complete follow-up for the full frozen eligible population, never
  outcome-selected complete cases;
  calibration/actionability, external replication, multiplicity, and non-rescuable success
  contract; H3's full inherited target-ID ledger, exact same-fold M1 substitution, complete-population
  paired-scoring policy, fail-closed receipt/reporting rules, positive useful-value margin,
  one-sided superiority decision, PID-feature construction, dependence-aware uncertainty,
  multiplicity, replication target, support acceptance, warning-code dispositions, and a typed
  source-target ancestry contract. That contract must bind a target-specific prediction landmark
  before target availability. It must also bind the source-target inventory, producer, consumer
  validator, and per-row receipt schema. It rejects post-landmark observations and exact target
  injection. A downstream target also requires the same proposal in the matched baseline;
  H3/H4 exclusivity within a maximum three-claim family; and H4 target sampling, transport,
  one-tuple/one-outcome selection, simultaneous inference, uncertainty for estimated target
  weights, exact fixed weights for an enumerated finite target, and joint
  power. `scripts/audit_research_governance_successor.py` validates the draft and fails closed on
  unknown fields, malformed bindings, false freeze metadata, or incoherent filled candidates.
  A terminal `frozen` document additionally requires a typed receipt. To avoid a circular hash,
  the validator canonicalizes the complete document as the reviewed candidate by setting
  `status` to `freeze_candidate_under_review`, setting `freeze_receipt`, `freeze_revision`, and
  `frozen_at` to null, then hashing compact UTF-8 JSON with sorted keys and no trailing newline.
  `freeze_revision` and the receipt must contain that exact SHA-256; the receipt must also bind the
  same frozen timestamp and all four reviewed global freeze-slot artifacts. An arbitrary file,
  arbitrary digest, or post-review candidate edit cannot promote the document. This is
  content binding, not a signature, identity proof, or automated judgment that a review was
  independent or scientifically adequate.
  The current H3 role bindings are structural placeholders only. The ancestry producer,
  consumer validator, and per-row receipt schema do not exist. An H3 candidate therefore returns
  `M0_SUCCESSOR_H3_ANCESTRY_PRODUCER_CONSUMER_AND_RECEIPT_UNIMPLEMENTED`, and an H3 terminal
  `frozen` state is invalid. Role strings, distinct paths, and hashes do not prove role adequacy.
  A future candidate must populate EC1, H2, one selected H1 protocol, and one selected H3/H4
  branch. Every inactive protocol slot must remain null. A fresh-sample switch from H3 to H4 keeps
  the frozen H3 contract as history under the prespecified sequential-error rule.
  Every freeze-bearing value in the checked draft is null, so
  `python scripts/audit_research_governance_successor.py --require-freeze-ready` is expected to
  exit 3 with typed blockers. A future freeze requires a new, separately reviewed, content-bound
  candidate; editing v1 or merely filling this checked draft does not promote it.
- `m0_preregistration_successor_draft_v2.json` preserves the superseded, all-null v2 draft bytes.
  The v3 ancestry-contract change is not schema-compatible with v2. The active validator targets
  v3. The retained v2 file is historical process evidence, not a candidate or migration target.
- `holdout_registry_v1.json` and its hash-chained access ledger currently
  say that no confirmatory holdout is registered; they do not prove historical or off-repository
  non-access.
- `transport_contamination_ledger_v1.json` is structure-only until source/target data are selected.
- `literature_screening_ledger_v1.json` imports the dated reference inventory with its missing
  query/candidate-decision provenance made explicit; it is not a systematic search. Validate this
  state rather than completing the missing work.

Validate the honest unfinished preserved diagnostic bundle with `just research-governance`, which runs both governance
validators. `python scripts/audit_research_governance.py --require-freeze-ready` and the successor
validator's strict mode are expected to fail until M0 is genuinely frozen. Those modes are
completeness and integrity gates, not automated judgment that the scientific choices or
independent reviews are substantively correct. The v1 scaffold is intentionally non-promotable.
The v3 file remains an all-null, revised, unreviewed draft contract.
- `ecosystem_evidence_current_v1.json` is an offline overlay on the immutable 21-row public-
  ecosystem audit archived under `docs/reviews/`. Normal CI verifies the archived baseline hash,
  row count, current overrides, and matching canonical prose without contacting the network.
- `capability_catalog_v1.json` is the reviewed source for the repository-wide feature/status
  inventory. `capability_matrix_current_v1.json` and `docs/CAPABILITY_MATRIX.md` are generated,
  fail-closed views whose local revisions and evidence artifacts are bound to deterministic
  content hashes. Software status is independent of the §8.9 relationship ladder: `tested` means a
  named local proof path, E2 requires an immutable external dependency, and E3 requires pinned
  producer/consumer golden-fixture evidence. The generator checks schema, paths, hashes, and
  canonical pins; review plus CI execution verifies that commands exercise the declared proof.
  There are currently no E4 or E5 `validated` rows.

Network refresh is deliberate: inspect advertised revisions and repository content, update the
dated overlay, reconcile dependent prose, and run `python scripts/audit_repo_truth.py`. Evidence
expires when an endpoint revision, schema, wire version, model, or adapter revision changes.
Regenerate and verify the capability views with `just capability-matrix` and
`just capability-matrix-check`.
