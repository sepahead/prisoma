# M1 command completion

The external command runner observes the selected supervisor through its actual exit.
It uses only Python's standard library and imports no simulator or installed experiment package.
Its command receipt supplements the [campaign evidence](M1_CAMPAIGN.md).

## Decision

| Approach | Assumption and benefit | Failure mode | Decisive control |
| --- | --- | --- | --- |
| Use shell redirection and an exit code | A short wrapper reduces code. | Unbounded output and incomplete source joins permit ambiguous evidence. | Overflow and selected-byte drift controls. |
| Let the supervisor report command success | The supervisor already owns execution. | It cannot observe its own eventual exit. | Publish an owner receipt, then exit unsuccessfully. |
| Use `communicate()` with a timeout | Standard helpers simplify pipe handling. | Captured output can exhaust memory before the timeout. | Emit more than the frozen stream limit. |
| Use a bounded external runner | A separate parent observes exit and selected bytes. | Incorrect cleanup could target unrelated processes or hide partial output. | Owned-child timeout and inherited-pipe controls. |
| Add a persistent process monitor | A separate service could retain observations across runner failure. | Additional authority, lifecycle, and deployment requirements exceed this campaign. | Remove that service and attempt standalone execution. |

The selected runner bounds both streams and waits for its direct child.
It does not introduce a persistent service or broaden process authority.

The mathematical lens requires exact integer limits and return codes.
The experimental lens preserves failures, overflow, missing receipts, and byte drift.
The authority lens permits signals only to the directly owned, unreaped supervisor.
The provenance lens joins selected source, freeze, command, output, and owner bytes.
The operator lens distinguishes supervisor exit from observed descendant retirement.

## Invocation and limits

Run the selected runner with isolated Python and disabled bytecode writes:

```text
<python> -I -B <selected-m1-run.py> --freeze <absolute-freeze.json> --case <case-id>
```

The runner derives the supervisor command from the selected case environment:

```text
<case-prefix>/bin/python -I -B <selected-supervisor.py> --freeze <absolute-freeze.json> --case <case-id>
```

The selected launcher retains its environment path.
Its resolved binary must match the frozen installed inventory.
This observation does not attest loaded interpreter or module bytes.

Let `s` be the frozen session limit, in seconds.
Let `c` be the frozen checkpoint limit, in seconds.
The freeze requires integer bounds `1 <= c <= 60`, `c < s`, and `s <= 600`.

| Bound | Formula | Maximum |
| --- | --- | --- |
| Command wall time | `s + 2*c + 30` seconds | 750 seconds |
| Grace after interruption | `c` seconds | 60 seconds |
| Retained bytes per stream | `65536 + 4096*(s+c)` bytes | 2,768,896 bytes |
| Wait after forced termination | 5 seconds | 5 seconds |

The wall limit covers the launched supervisor, including its preflight and terminal checks.
The runner's own file reads, source checks, and publication occur outside that subprocess limit.
Local filesystem operations do not supply hard real-time guarantees.

Timeout or overflow requests cleanup through `SIGINT` on the directly owned, unreaped supervisor.
After the grace period, the runner may send `SIGKILL` to that same direct child.
It never signals a process group or selects a descendant PID for cleanup.
An inherited pipe can remain open after supervisor exit.
That case fails the deadline and provides no descendant-retirement claim.

## Receipt and publication

Each attempt requires fresh `<case>.command` and `<case>.command.json` paths within the frozen output root.
The command directory contains bounded `stdout.bin` and `stderr.bin` files.
Overflow retains each stream's prefix and marks the attempt unsuccessful.

Schema `prisoma.m1-command.v1` contains exactly these fields:

| Field | Meaning |
| --- | --- |
| `schema` | Exact command receipt schema identifier. |
| `freeze` | Absolute freeze path, byte count, and SHA-256 digest. |
| `case` | Complete selected case row. |
| `runner`, `supervisor` | Exact frozen source file identities. |
| `argv` | Complete derived supervisor argument vector. |
| `limits` | Derived wall, grace, and stream bounds. |
| `returncode` | Actual integer status observed after supervisor reaping. |
| `timed_out`, `output_overflow` | Explicit deadline and retained-output failures. |
| `interrupted`, `forced_kill` | Runner interruption and attempted forced direct-child termination. |
| `failure` | A bounded failure code, or null. |
| `stdout`, `stderr` | Absolute retained stream identities. |
| `owner` | Exact post-exit owner file identity, or null. |

Each file identity contains only `path`, `bytes`, and `sha256`.
The runner reopens the freeze and selected files after supervisor exit.
It reads `owner.json` only after reaping that supervisor.
The comparator separately reopens owner bytes and validates their semantic joins.

Command acceptance requires return code zero, all four flags false, no failure code, and the selected owner identity.
An owner receipt alone does not establish command success.
Prelaunch failure or failure to reap cannot produce a completed command receipt.

Publication writes `.<case>.command.json.pending`.
It flushes and synchronizes that file, then reopens its exact bytes.
A final hard link publishes `<case>.command.json` without replacing an existing path.
The staging link remains available for inspection.
There is no fallible validation or cleanup after that commit point.
This contract makes no directory-fsync durability claim.

The synthetic controls establish command and receipt behavior only.
They do not qualify a renderer, simulator, native campaign, or scientific result.
