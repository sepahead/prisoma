# Living protocol ledgers

This directory contains machine-readable **current-state** protocol and ecosystem ledgers. They
do not replace `grandplan.md`, preregister a real experiment, or turn software fixtures into
scientific evidence.

- `research_claim_registry_v1.json` maps EC1 and H1–H4 to their current executable artifacts,
  proof commands, blockers, and permitted claim language. Development/blinded-pilot nuisance and
  design parameters remain unfrozen instead of receiving invented values; minimum useful effects
  require separate domain and decision justification.
- `m0_preregistration_skeleton_v1.json` is an **unfrozen** branch-separated H1-A/H1-B/H2/H3/H4
  scaffold. `holdout_registry_v1.json` and its hash-chained access ledger currently say that no
  confirmatory holdout is registered; they do not prove historical or off-repository non-access.
  `transport_contamination_ledger_v1.json` is structure-only until source/target data are selected.
  `literature_screening_ledger_v1.json` imports the dated reference inventory with its missing
  query/candidate-decision provenance made explicit; it is not a systematic search. Validate this
  honest unfinished state with `just research-governance`. The stricter
  `python scripts/audit_research_governance.py --require-freeze-ready` is expected to fail until M0
  is genuinely frozen. That stricter mode is a completeness and integrity gate, not an automated
  judgment that the scientific choices or independent reviews are substantively correct. The v1
  scaffold is intentionally non-promotable: a real freeze requires a reviewed successor schema and
  validator with typed, content-bound receipts, not in-place replacement of these null fields.
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
