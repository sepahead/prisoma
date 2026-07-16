# Living protocol ledgers

This directory contains machine-readable **current-state** protocol and ecosystem ledgers. They
do not replace `grandplan.md`, preregister a real experiment, or turn software fixtures into
scientific evidence.

- `research_claim_registry_v1.json` maps EC1 and H1–H4 to their current executable artifacts,
  proof commands, blockers, and permitted claim language. Development/blinded-pilot nuisance and
  design parameters remain unfrozen instead of receiving invented values; minimum useful effects
  require separate domain and decision justification. The current H4 software artifact is a
  reference-model deletion-ranking-sensitivity diagnostic only; it does not establish causal use,
  mechanism, or attribution faithfulness. Its current-state date may advance beyond the historical
  intake bundle only when it content-binds a typed v2 successor with the same date and status.
- `m0_preregistration_skeleton_v1.json` is an **unfrozen** branch-separated H1-A/H1-B/H2/H3/H4
  historical scaffold. Its exact checked bytes and intake-time artifact hashes are preserved;
  those historical hashes are records, not assertions that mutable current fixture paths still
  contain the same bytes. It remains intentionally non-promotable.
- `m0_preregistration_successor_draft_v2.json` is the reviewed **typed successor draft**, not a
  preregistration or freeze candidate. It exact-binds the historical v1 bytes and adds null,
  freeze-required obligations for EC1's finite adapter/fault/oracle/acceptance design, including
  exact fault-adapter detection coverage, separately estimated absolute sensitivity floors, and
  mandatory passage without distribution-average rescue, plus per-adapter replay-fidelity and
  false-positive coverage; H1-A calibration bins; H1-B's typed effect-specific endpoint kind,
  single primary endpoint, and hierarchy, with factual-outcome loss restricted to a secondary
  descriptive outcome-model diagnostic;
  H2's landmark, target, censoring, comparator, one-primary-proper-score (where the
  no-censoring branch requires complete follow-up for the full frozen eligible population, never
  outcome-selected complete cases),
  calibration/actionability, external replication, multiplicity, and non-rescuable success
  contract; H3's full inherited target-ID ledger, exact same-fold M1 fallback, complete-population
  paired-scoring policy, fail-closed receipt/reporting rules, and warning-code dispositions;
  H3/H4 exclusivity within a maximum three-claim family; and H4 target sampling, transport,
  one-tuple/one-outcome selection, simultaneous inference, target-weight uncertainty, and joint
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
  Every freeze-bearing value in the checked draft is null, so
  `python scripts/audit_research_governance_successor.py --require-freeze-ready` is expected to
  exit 3 with typed blockers. A future freeze requires a separately reviewed, content-bound
  candidate; editing v1 or merely filling this checked draft does not promote it.
- `holdout_registry_v1.json` and its hash-chained access ledger currently
  say that no confirmatory holdout is registered; they do not prove historical or off-repository
  non-access.
- `transport_contamination_ledger_v1.json` is structure-only until source/target data are selected.
- `literature_screening_ledger_v1.json` imports the dated reference inventory with its missing
  query/candidate-decision provenance made explicit; it is not a systematic search. Validate this
  state rather than completing the missing work.

Validate the honest unfinished bundle with `just research-governance`, which runs both governance
validators. `python scripts/audit_research_governance.py --require-freeze-ready` and the successor
validator's strict mode are expected to fail until M0 is genuinely frozen. Those modes are
completeness and integrity gates, not automated judgment that the scientific choices or
independent reviews are substantively correct. The v1 scaffold is intentionally non-promotable,
and the v2 file remains an all-null draft contract.
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
