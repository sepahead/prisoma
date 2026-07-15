# Draft release notes — Prisoma 0.9.0

Prisoma 0.9.0 is an unpublished source and research-software preview candidate authored by
Sepehr Mahmoudian. It prepares the repository's implemented, testable infrastructure while keeping every
scientific gate and imported handoff disposition explicit.

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

This candidate is not a frozen preregistration, confirmatory result, EC1 validation,
production deployment, or validated PID application to real embeddings. The current PID
application gate remains blocked. No DOI or Zenodo record is assigned.

See `CHANGELOG.md`, `LIMITATIONS.md`, and `THESIS_EVIDENCE_INDEX.md` for the exact scope.
