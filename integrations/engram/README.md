# Engram host integration

Prisoma remains a standalone system. Engram host API 1.1 adds one optional,
externally owned, read-only session profile. That profile requires
operator-paste pairing on every connection.

Host API 2 adds a separate managed observer candidate.
It verifies closed-loop receipt lineage through inherited private pipes.
It does not replace or modify the Host API 1.1 status bridge.

## Host API 2 managed observer

The observer implements `prepare`, `observe`, and `finish` operations.
All three operations use class `observation` and compute grant `none`.

`prepare` records one to 64 host-declared channels and an equal subject roster.
The checked fixture declares three channels and three drone identifiers.
Engram source receipts do not authenticate those identifiers.
`observe` verifies each complete Engram V2 step receipt in order.
It rejects a later input that differs from the prior accepted output.
`finish` verifies the V2 terminal receipt and V5 transcript, then clears retained state.
It joins the first input to the initial snapshot and rechecks the full chain.
It also checks logical clocks, neural execution bindings, and cleanup provider digests.
It mirrors the durable-evidence profile without validating a NEST evidence bundle.
The checked success vector declares the NEST bundle-v2 profile.
Every response still reports bundle verification as false.

A failed source run can finish with zero observed steps.
The prepared maximum remains between one and 1,024 steps.

The runtime has no Agent Bridge command, action, intervention, PID, NCP, or network operation.
It has no filesystem operation, artifact route, embodiment, plant, or actuation capability.
The reviewed sandbox does not enforce filesystem isolation.
Its receipts are descriptive local observations, not scientific evidence.
Every child response reports `source_durable_evidence_verified=false`.
An external tool can call Engram's exact bundle-to-run validator and emit a bounded summary.
The matrix harness accepts CREBAIN capture v2 and evidence-index v2 only.
It accepts installed-package proof v3 only.
It reviews one-, two-, and three-drone captures in fixed order.
It reopens exact Prisoma, CREBAIN, and Engram sources from clean pushed commits.
It reads the `real-nest-3.9-v2` evidence roster and retains the v1 input suite.
It separates the CREBAIN source commit, C0, from publication commit C1.
C1 must be C0's direct single-parent child.
C1 may add only the index and three capture blobs.
It validates CREBAIN tool, build, stage, pack, package, and installed-proof lineage.
The pack receipt binds committed `scripts/engram_extension.py` bytes and Git identity.
Each capture joins that tool row to its loaded host-module source closure.
This join does not attest Python bytecode, interpreter state, or dependency bytes.
Each run has a distinct runtime source closure and one common stable source roster.
It verifies each exact eight-row V5 receipt-store roster.
It rederives terminal, evidence, store metadata, and lock bytes.
It validates four sidecar paths and their opaque captured identities.
Capture v2 does not embed those four sidecar bodies.
It also rejoins V2 launch, preparation, capabilities, and step-attempt receipts.
It also joins 6N topology and guardian lifecycle.
Historical evidence-index v1 remains audit-only.
It runs the release observer and the external Engram validator for each capture.

The package template and authoring recipe are in `managed-observer/`.
Package staging requires the adjacent clean-source arm64 build receipt.
The receipt binds exact source, toolchain, and Mach-O bytes without granting runtime authority.
Staging emits a separate receipt for the exact build and package bytes.
The stage receipt grants no installation, execution, NCP, or MUSIC authority.
The real-binary fixture normalizes only runtime nonces and envelope message identifiers.
It preserves every deterministic semantic receipt.

A historical v1 receipt records one temporary-store launch and clean reap.
Its source state was a working-tree candidate.
The current v2 operational gate remains `NOT RUN`.
The v2 contract requires verified staging and guardian process-group closure.
No evidence grants production manager, publisher, NCP, physical, or scientific authority.
Read [the managed observer guide](managed-observer/README.md) before packaging.

## Start the hosted profile

Run this command from the Prisoma repository:

```bash
cargo run --locked -p pid-sim --bin pid-sim-bridge-tcp -- \
  --engram-host --unique-run-log-dir outputs
```

The `outputs` directory must exist. Prisoma atomically creates a new
`engram-host-*.jsonl` file in that directory. It does not replace an existing
file.

`--engram-host` also generates one 32-byte startup secret from the
operating-system CSPRNG. The process prints the run-log path and that secret
exactly once on its controlling stderr, before it announces the listener:

```text
run_log=outputs/engram-host-<run>.jsonl pairing=engp1_<43 chars>
listening 127.0.0.1:38472
```

The secret never appears in the run log, a response, or any file. The operator
pastes it into Engram. A new launch generates a new secret.

The process accepts loopback connections until one proves possession of the
secret and finishes. Engram does not start, restart, stop, or attest the
Prisoma process.

Disconnecting ends the bound session. The process then answers any queued
socket with a pairing rejection and exits. Run the same command to create a new
secret, a new log, and another session.

The run seals as `Failed` when no accepted connection ever paired.

## Generic contract

`manifest.json` declares these generic Engram contracts:

- entrypoint `host-rendered-jsonrpc-tcp`
- renderer `engram.bridge-status.v2`
- framing `json-rpc-2.0-ndjson`
- profile `engram-host-read-only-v2`
- methods `bridge.describe`, `bridge.session`, and `sim.status`
- pairing mechanism `operator-paste-psk-hmac-sha256-v1`
- pairing secret format `engp1-base64url-256`
- pairing scope `single-successful-tcp-connection`

The descriptor accepts only a canonical run log for Engram rendering. It
declares no produced artifacts. Rerun recordings and offline VLDA artifacts
remain standalone Prisoma surfaces.

The profile forces safe mode. `--engram-host` conflicts with
`--allow-mutations` in either argument order. The adapter exposes no simulation
step, reset, scene write, intervention, log write, export, replay, or NCP
method.
The parser rejects unknown request-envelope members and duplicate object keys
at every JSON depth.

## Operator-paste pairing

The first request on each accepted connection must be a paired
`bridge.session`. Its `params` member holds exactly one `pairing` object with
`mechanism`, `client_nonce`, and `client_proof`.

Engram derives `client_proof` with HMAC-SHA256 keyed by the pasted secret. That
message binds the active profile, the RFC 8785 JCS form of the JSON-RPC request
id, and one fresh 32-byte client nonce. Pairing accepts string and safe-integer
ids. Prisoma uses the RustCrypto HMAC verifier.

A successful result adds a `pairing` member with a fresh 32-byte server nonce
and a server proof. That proof binds the profile, the same canonical request id,
both nonces, and the SHA-256 of the RFC 8785 JCS form of the session result
without its own `pairing` member. Engram verifies the server proof before it
trusts any session field.

A client nonce never repeats inside one connection. Prisoma commits the
single-connection binding only after it records a successful paired session result. Each later
`bridge.session` refresh on the bound socket carries a fresh nonce and a fresh
proof.

### Rejection and latch behavior

Every rejected pairing returns JSON-RPC error `-32001` with the message
`pairing rejected`. The error carries no `data` member and no reason. Prisoma
then closes the socket. A replay of captured bytes on a new socket gets the
same rejection.

Each accepted connection consumes one unit of `max_pairing_attempts`. A
timeout and a wrong proof each consume one unit. A successful proof ends the listener after that
connection closes.
Eight failed connections latch the bridge. No further pairing is possible on
that launch. `bridge.session` reports `pairing_attempts` in `resource_usage`.

## Finite session policy

The process enforces the limits declared in the manifest:

| Resource | Limit |
| --- | ---: |
| Requests | 512 |
| JSON-RPC line | 65,536 bytes |
| Aggregate input | 8,388,608 bytes |
| Session run log | 67,108,864 bytes |
| Session run-log events | 2,048 |
| Pairing attempts | 8 |

`bridge.session` reports the active profile, exact method set, limits, and
current usage. Run-log usage includes the TCP prefix and current request. It
excludes the pending session response, as `observed_at` states. The terminal
seal also consumes the declared limit. Engram rejects a mismatch. The process
fails closed when a limit is reached.

An Engram connection uses three initial reads. Each status refresh uses two
more reads. At the current four-second nominal cadence, 512 requests provide
about 16 minutes 56 seconds of scheduled refresh coverage. Manual reads and
scheduling delays can reduce that time. This profile is not an unattended
study monitor.

## Evidence and authority boundary

Pairing proves possession of the startup secret. It is not same-user,
same-terminal, process, build, or commit attestation. An operator can forward
the pasted secret, and another local process can steal it or occupy the port
before Prisoma listens.

The session report is a peer assertion. It does not prove process identity,
source revision, executable digest, or transport authentication.

The canonical run log remains the source of truth for accepted recorded events.
The `prisoma.canonical-run-log.v2` renderer is a structural preview.
It is not Prisoma validation, replay evidence, NCP, a closed loop, or control
authority.

Prisoma pins the latest immutable NCP `v0.8.0` release and uses wire 0.8. Official NCP
main was observed at `1a04294c90c1b50eba06ae1c6afe9c951319250d` on 2026-08-13.
That commit is the unreleased, release-blocked `1.0.0-rc.1` candidate (wire 1.0;
compact proto contract hash `163acc57d8a62b66`). The manifest declares target Engram wire
1.0 and marks it incompatible with Prisoma wire 0.8. NCP's provider inventory records a
preserved in-progress Paper2Brain migration that targets candidate wire 1.0. It is not an
installed or qualified integration. No translation path exists.
NCP ledger tasks `P01`, `P02`, and `P03` are OPEN, not dependency-ready, and **NOT RUN**.
`P03` covers fault-observatory migration and Prisoma observer-role qualification. The refined
low-overhead architecture and prepared-stream-monitor gap record are coordination-only. B01
remains `IN_PROGRESS` with no passing receipt. See the
[verified NCP task ledger](https://github.com/sepahead/NCP/blob/1a04294c90c1b50eba06ae1c6afe9c951319250d/evidence/implementation/task-ledger.v1.json).

## Validation

Run these focused gates:

```bash
cargo test --locked -p pid-bridge
cargo test --locked -p pid-sim --lib
cargo test --locked -p pid-sim --bin pid-sim-bridge-tcp
cargo clippy --locked -p pid-bridge -p pid-sim --all-targets -- -D warnings
cargo fmt --all -- --check
```

The cross-repository design decisions and acceptance evidence are maintained in
Engram's `docs/EXTENSION_HOST_QUALITY_LEDGER.md`.
