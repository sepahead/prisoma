# Engram host integration

Prisoma remains a standalone system. Engram host API 1.1 adds one optional,
externally owned, read-only session profile. That profile requires
operator-paste pairing on every connection.

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
message binds the active profile, the exact JSON-RPC request-id text, and one
fresh 32-byte client nonce. Prisoma compares the proofs in constant time.

A successful result adds a `pairing` member with a fresh 32-byte server nonce
and a server proof. That proof binds the profile, the same request-id text,
both nonces, and the SHA-256 of the RFC 8785 JCS form of the session result
without its own `pairing` member. Engram verifies the server proof before it
trusts any session field.

A client nonce never repeats inside one connection. The first valid client
proof binds the secret to that single TCP connection. Each later
`bridge.session` refresh on the bound socket carries a fresh nonce and a fresh
proof.

### Rejection and latch behavior

Every rejected pairing returns JSON-RPC error `-32001` with the message
`pairing rejected`. The error carries no `data` member and no reason. Prisoma
then closes the socket. A replay of captured bytes on a new socket gets the
same rejection.

Each accepted connection consumes one unit of `max_pairing_attempts`. A
timeout, a wrong proof, and a post-binding connection each consume one unit.
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
