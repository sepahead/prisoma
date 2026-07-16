# Security policy

## Supported source line

Prisoma 0.9.0 is a public GitHub source prerelease and research-software preview,
not a stable or production-supported release line. Security fixes are applied to
`main`; a later 0.9.x preview supersedes earlier previews. Older development
snapshots are unsupported, and no security-support or response-time SLA is
offered. Prisoma is research software, not a certified safety, security, medical,
or production-control system.

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
- Rerun conversion does not read attribution artifacts by default. The standalone
  converter requires `--load-attribution-artifacts`, then accepts only bounded
  relative regular, non-symlinked NumPy files below the run-log directory and
  verifies each file's recorded exact SHA-256 and canonical shape before output;
  bridge export never enables this capability. Converter input must be a non-symlink
  regular file, and headless output is a finalized, staged, file-synced, no-clobber
  `.rrd`. Viewer-specific event, input-byte, and projected-log-call limits constrain
  output amplification. The path checks reduce accidental traversal but are not a
  security-grade defense against hardlinks, aliases, or every concurrent mutation;
  digest verification still rejects changed artifact bytes.
- `crates/ncp-observer` is an optional, read-only, workspace-excluded consumer.
  Its checked fixtures and fault observatory are not live security validation,
  authenticated producer evidence, EC1 evidence, or permission to deploy an
  open NCP profile. Its pinned Zenoh 1.9 graph retains `lz4_flex` 0.10.0, which is
  affected by the high-severity RUSTSEC-2026-0041 block-decompression information
  disclosure. The checked dependency profile does not enable Zenoh's
  `transport_compression`, so the affected `decompress_into` call is compiled out;
  CI fails if that feature appears. The vulnerable package nevertheless remains in
  the optional lock, so Prisoma makes no vulnerability-free or live-NCP security
  claim and must replace the pin before that profile can be release-qualified. The
  same graph also retains the unmaintained (not known vulnerable)
  `rustls-pemfile` 2.2.0 because no compatible replacement exists. The root
  all-features lock/CI graph retains the unmaintained `paste` 1.0.15 proc-macro
  through optional Rapier's nalgebra tree, and the observer graph retains it
  through Zenoh; the default-feature root build does not compile it. `deny.toml`
  records these temporary exceptions with their actual dependency scopes.
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
