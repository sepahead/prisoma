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

Network refresh is deliberate: inspect advertised revisions and repository content, update the
dated overlay, reconcile dependent prose, and run `python scripts/audit_repo_truth.py`. Evidence
expires when an endpoint revision, schema, wire version, model, or adapter revision changes.
