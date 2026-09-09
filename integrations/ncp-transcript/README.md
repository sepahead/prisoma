# Modular NCP transcript

Capture original NCP exchanges in one bounded file, with one to sixteen selected peers.
No body, neural, or monitor role is mandatory.
Large sensor payloads remain sequences of bounded NCP frames.

![Request capture, dispatch, response capture, and acknowledgement](capture.svg)

[Direct SVG for browser zoom](https://raw.githubusercontent.com/sepahead/prisoma/main/integrations/ncp-transcript/capture.svg)

## Contract

The host supplies the peer bindings, installed application contracts, requests, and streams.
`Journal.exchange` synchronizes each request before dispatch.
It synchronizes the received response before returning its bytes.
The caller can then acknowledge the result or request the next buffer chunk.
The journal selects no action and launches no producer.

NCP's existing client verifies request, response, predecessor, result-query, and acknowledgement joins.
The read-only verifier replays those same rules with the exact installed application contracts.
The file preserves original frame bytes instead of reserializing the exchanged messages.

Every peer must acknowledge a committed `Finish` before the journal can finish.
An explicit abort records a closed prefix without declaring execution completion.
Plain close, missing responses, truncated records, and storage failures leave incomplete evidence.
A transport failure does not authorize retry.
The host remains responsible for retiring its producer processes.

## Use on trusted streams

Install this optional package in its own Python environment:

```bash
python -m pip install ./integrations/ncp-transcript
```

The manifest pins the public NCP SDK to immutable Git source `9ae64ac1a77c9cd0612284992a8711220428a6e3`.
Python 3.11 or newer on a POSIX host is required.
The root Prisoma environment and PID dependency remain independent of this package.

The following code uses a binding and application contract from an existing trusted host.
`reader` and `writer` are that host's private NCP streams.

```python
from ncp_local.modular_client import Client
from ncp_local.modular_wire import Outcome
from prisoma_ncp_transcript import Journal, Peer

client = Client(binding, Contract)
with Journal(
    "capture.ncp",
    (Peer(binding, Contract),),
    max_exchanges=204,
    quota_bytes=32 * 1024**2,
) as journal:
    def call(operation):
        raw = client.begin(operation)
        reply = journal.exchange(
            binding.endpoint_id, raw, reader, writer, deadline=deadline
        )
        result = client.observe(reply)
        if result.outcome is not Outcome.COMMITTED:
            raise RuntimeError("Operation did not commit")
        ack = client.acknowledgement()
        reply = journal.exchange(
            binding.endpoint_id, ack, reader, writer, deadline=deadline
        )
        acknowledged = client.observe_acknowledgement(reply)
        if acknowledged.outcome is not Outcome.ACKNOWLEDGED:
            raise RuntimeError("Acknowledgement did not complete")
        return result

    # Use call() for preparation, operations, and the final Finish.
    # Acknowledge only the outcomes permitted by the installed contract.
    # Retain source buffers until their captured reads succeed.
    # journal.finish() then requires every selected peer's terminal acknowledgement.
```

This example shows a host integration seam, not a complete simulator launcher.
A Prisoma experiment must dispatch mutations through Agent Bridge and bind its canonical events to the transcript.
This package does not create that experiment binding.

## Capacity and storage

Let `N` be the maximum exchange count.
One exchange contains one request and one response, including acknowledgement exchanges.
Each NCP frame contains at most 65,536 bytes.
Its transcript record adds a one-byte peer index and 45 framing bytes.

The maximum exchange cost is:

```text
P = 2 × (65,536 + 1 + 45) = 131,164 bytes.
Q >= 8 + 45 + H + N × P + 45 + 1,024.
```

`H` is the serialized header length in bytes.
`Q` is the caller's byte quota.
The final 1,024 bytes reserve the terminal payload.
Admission enforces both the 8,190-exchange count ceiling and the 1-GiB byte ceiling.
The byte ceiling can reject a count that otherwise passes its independent limit.

For 204 exchanges, frame reservations require 26,757,456 bytes before header and terminal space.
A 32-MiB quota admits the recorded one-peer NEST case.
The resulting file used 346,779 bytes.
Quota admission reserves logical capacity; it neither allocates disk blocks nor guarantees future free space.

The file must be new, private, regular, and owned by the current user.
The writer rejects an existing final component and synchronizes the parent directory.
The verifier rejects final symlinks, hard links, shared permissions, oversized files, and changed file identities.
The parent directory remains trusted host input.
This contract does not qualify an adversarial shared filesystem.

Records contain a kind byte, ordinal, payload length, original payload, and SHA-256 chain digest.
Each digest includes its predecessor and exact framing bytes.
A hash chain detects unaccounted changes; it is not a signature or producer attestation.

The [SMT model](quota.smt2) checks conditional frame-cost and remaining-quota arithmetic.
It includes feasible examples and counterexamples to intentionally false bounds.
It does not prove the Python implementation, operating system, or storage device.

## Read-only verification

```python
from prisoma_ncp_transcript import verify

report = verify("capture.ncp", (Peer(binding, Contract),))
print(report.store_completion, report.exchange_pairs)
```

The report separates unanswered requests, pending peer operations, and finished peers.
It always returns `application_completion_validated=false` and `scientific_validation=false`.
Typed protocol validity does not establish complete application semantics or experimental validity.
The journal cannot prove that a caller never bypassed its capture boundary.

This format is `prisoma.ncp.transcript.v1`.
The earlier [local causal journal](../../crates/ncp-local-capture/README.md) retains its separate compatibility contract.
Neither format replaces the canonical schema-2 Agent Bridge run log.

## Observed evidence

The source suite passes 29 tests on Python 3.11 and 3.14.
The same 29 tests pass after a fresh Python 3.14 installation from the pinned public Git dependency.
That cold installation took 353 seconds; it does not establish a fast installation path.
It covers private-stream I/O, sixteen peers, an 800-KiB frame sequence, and storage and transport failures.
Negative controls cover malformed responses, changed files, missing acknowledgements, reordering, truncation, quotas, and terminal semantics.

One source experiment captured an actual NEST 3.9.0 service with eight recurrently connected neurons.
It recorded 100 one-millisecond steps, 204 exchanges, and 408 original frames.
All 100 observations matched the retained direct-reference study.
The first observation is pending and has no numeric count or rate.
The 99 complete observations cover time through 99 ms, leaving an explicit one-millisecond unread tail.
All four observed processes exited.
The native runtime and selected source files remained unchanged.

The slowest captured step took 16.809041 ms.
The complete experiment was not a one-millisecond real-time loop.
Filesystem synchronization has no hard execution-time bound in this implementation.

The [evidence summary](evidence.json) binds the source, observations, and retained root review.
The [step table](observations.csv) contains statuses, measured counts, and durations.
Empty numeric cells preserve the pending observation; they do not represent zero activity.
This source experiment does not qualify an installed host, CREBAIN buffer release, or a complete embodied experiment.
Independent review remains pending.

After installation, run the focused source suite:

```bash
python -m unittest discover -s integrations/ncp-transcript/tests -v
z3 integrations/ncp-transcript/quota.smt2
```

Use exact Z3 4.16.0 for the retained arithmetic check.
The expected results are `unsat, sat, sat, unsat, sat, sat`.
The [design record](DESIGN.md) compares ten approaches and separates the unresolved integration requirements.
