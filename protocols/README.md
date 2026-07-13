# Living protocol ledgers

This directory contains machine-readable **current-state** protocol and ecosystem ledgers. They
do not replace `grandplan.md`, preregister a real experiment, or turn software fixtures into
scientific evidence.

- `research_claim_registry_v1.json` maps EC1 and H1–H4 to their current executable artifacts,
  proof commands, blockers, and permitted claim language. Pilot-derived quantities remain
  explicitly unfrozen instead of receiving placeholder numbers.
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
