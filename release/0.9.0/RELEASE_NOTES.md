# Release notes — Prisoma 0.9.0

Prisoma 0.9.0 is a public GitHub source prerelease and research-software preview authored by
Sepehr Mahmoudian, released on 2026-07-16. It presents the repository's implemented, testable
infrastructure while keeping every scientific gate and imported handoff disposition explicit.

Highlights:

- pins the diagnostic stack to the reviewed Rerun SDK release 0.34.1, with an exact
  viewer-version guard and finalized no-clobber `.rrd` saves;
- hardens run-log conversion, VLA adapters, attribution artifacts, timestamps, and
  output-amplification limits with fail-before-write tests;
- binds the supplied 240-task handoff ledger and 4,800 task/lens dispositions without
  claiming that open scientific work is complete;
- keeps candidate schema 0.1 non-promotable: it accepts review comments, blockers, active
  work, and failed evidence, while rejecting every positive terminal outcome;
- pins CI actions and the Python environment, checks the minimum Rust toolchain, and
  expands dependency and notice audits to optional workspace features.

This source prerelease is not a frozen preregistration, confirmatory result, EC1 validation,
production deployment, or validated PID application to real embeddings. The current PID
application gate remains blocked. `published:false` in the candidate decision manifest means
that candidate-package and scientific promotion are not authorized; it does not deny public
availability of this source prerelease. No DOI, Zenodo record, or archive identifier is assigned.

See `CHANGELOG.md`, `LIMITATIONS.md`, and `THESIS_EVIDENCE_INDEX.md` for the exact scope.
