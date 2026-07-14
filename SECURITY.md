# Security policy

## Supported release

Security fixes are applied to the latest `0.9.x` source release and to `main`.
Older development snapshots are unsupported. Prisoma is research software, not
a certified safety, security, medical, or production-control system.

## Reporting a vulnerability

Use GitHub's private vulnerability-reporting form on the repository **Security**
tab. Do not open a public issue for a suspected vulnerability, credential,
private dataset, or exploit. Include, when available:

- the affected commit, platform, feature flags, and dependency lock;
- a minimal reproduction that contains no private data or live credentials;
- the expected and observed trust boundary;
- whether confidentiality, integrity, availability, provenance, or control-plane
  authority is affected; and
- any known workaround.

Do not test against infrastructure or data you do not own or have explicit
permission to assess. There is no bug-bounty programme.

## Current trust boundaries

- The Agent Bridge binaries are loopback-only development tools. They do not
  authenticate users, authorize roles, provide TLS, or prevent a local proxy or
  tunnel from exposing the listener. Their path confinement is not a
  security-grade filesystem sandbox.
- `crates/ncp-observer` is an optional, read-only, workspace-excluded consumer.
  Its checked fixtures and fault observatory are not live security validation,
  authenticated producer evidence, EC1 evidence, or permission to deploy an
  open NCP profile. Its pinned Zenoh 1.9 graph also retains the unmaintained
  (not known vulnerable) `rustls-pemfile` 2.2.0 because no compatible replacement
  exists; `deny.toml` records the narrow temporary exception.
- Run-log hashes and local ledgers provide integrity and reproducibility
  evidence within their stated threat model. They are not signatures, remote
  attestation, trusted timestamps, or proof of historical non-access.
- The SAFE adapter's strict formats reduce deserialization risk, but external
  datasets, model weights, checkpoints, and legacy conversions remain separate
  trust and rights boundaries.

The canonical limitations are in [LIMITATIONS.md](LIMITATIONS.md) and the
implementation-specific threat boundaries are in `grandplan.md` sections 8 and
16. A passing test or dependency scan establishes only the behavior that test
or scan covers.

## Response and release handling

The maintainer will validate the report, determine affected revisions, and
coordinate a fix or documented scope reduction. A release is withdrawn or
superseded when its distributed artifact cannot be reconstructed, a critical
provenance claim is false, a security boundary is materially misstated, or a
known exploitable dependency remains in the distributed path. Public disclosure
is coordinated after users have a reasonable opportunity to update.
