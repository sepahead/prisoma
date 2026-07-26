# Engram host integration

Prisoma remains a standalone system. Engram host API 1.0 adds one optional,
externally owned, read-only session profile.

## Start the hosted profile

Run this command from the Prisoma repository:

```bash
cargo run --locked -p pid-sim --bin pid-sim-bridge-tcp -- \
  --engram-host --unique-run-log-dir outputs
```

The `outputs` directory must exist. Prisoma atomically creates a new
`engram-host-*.jsonl` file in that directory. It prints the canonical path
before it listens. It does not replace an existing file.

The process waits for one loopback client. Engram connects and disconnects its
socket. Engram does not start, restart, stop, authenticate, or attest the
Prisoma process.

Disconnecting ends the one-client server session. Run the same command to
create a new log and start another session.

## Generic contract

`manifest.json` declares these generic Engram contracts:

- entrypoint `host-rendered-jsonrpc-tcp`;
- renderer `engram.bridge-status.v1`;
- framing `json-rpc-2.0-ndjson`;
- profile `engram-host-read-only-v1`; and
- methods `bridge.describe`, `bridge.session`, and `sim.status`.

The descriptor accepts only a canonical run log for Engram rendering. It
declares no produced artifacts. Rerun recordings and offline VLDA artifacts
remain standalone Prisoma surfaces.

The profile forces safe mode. `--engram-host` conflicts with
`--allow-mutations` in either argument order. The adapter exposes no simulation
step, reset, scene write, intervention, log write, export, replay, or NCP
method.
The parser rejects unknown request-envelope members and duplicate object keys
at every JSON depth.

## Finite session policy

The process enforces the limits declared in the manifest:

| Resource | Limit |
| --- | ---: |
| Requests | 512 |
| JSON-RPC line | 65,536 bytes |
| Aggregate input | 8,388,608 bytes |
| Session run log | 67,108,864 bytes |
| Session run-log events | 2,048 |

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

The loopback endpoint is unauthenticated. A local process can occupy the port.
The session report is a peer assertion. It does not prove process identity,
source revision, executable digest, or transport authentication.

The canonical run log remains the source of truth.
The `prisoma.canonical-run-log.v2` renderer is a structural preview.
It is not Prisoma validation, replay evidence, NCP, a closed loop, or control
authority.

Prisoma uses NCP wire 0.8. Engram uses candidate wire 1.0. The manifest marks
them incompatible. No translation path exists.

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
