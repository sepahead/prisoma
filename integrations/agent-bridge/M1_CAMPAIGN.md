# Frozen sensor transfer campaign

This campaign compares installed body-only and canonical sensor execution.
It uses the public CREBAIN, NCP, and optional Prisoma APIs.
The [September 23 native evidence](evidence/M1_NATIVE_2026-09-23.md) records a passing 24-tick comparison, 15 rejecting controls, and two expected faults.
Qualification covers only its frozen engineering case and recorded runtime.
Both fault cases remain unhealthy, with incomplete captures and separate cleanup observations.
Synthetic controls establish software behavior only.
The separate [fixed-schedule baseline](M1_PERFORMANCE.md) owns prospective performance measurement without real-time or release qualification.

The maintained workload is [the original M1 input](tests/fixtures/m1.workload.v1.json).
Its 3,444 bytes have SHA-256 `f4b8cad4a9daf1f5b95abb9b03409040cd8caa67cf2e14c074a17dcc4a77ecae`.
The runner derives preparation, ordered targets, cadence, and transfer bounds from selected workload bytes.
It never treats one arm as the scientific input oracle.
Changing the workload requires a new freeze and separately reported qualification.

## Design decision

| Approach | Assumption and benefit | Failure mode | Decisive control |
| --- | --- | --- | --- |
| Restore the historical driver | Existing code reduces implementation work. | Missing private artifacts and stale source selections prevent reproducible use. | Reopen its complete source closure. |
| Put campaign policy in the installed bridge | Existing callbacks provide convenient checkpoints. | Experimental policy changes the execution API and previously selected wheels. | Install unchanged wheels in both arms. |
| Reimplement the protocol | Independent code appears to provide an independent oracle. | A second protocol implementation can accept incompatible requests. | Reconstruct bytes through the public typed client. |
| Compare shell summaries | Small scripts compare totals quickly. | Equal summaries can hide changed actions, ownership, or payload bytes. | Rehashed command and equal-but-wrong-workload controls. |
| Maintain a bounded external worker and comparator | Public APIs provide typed execution and replay. | Missing immutable joins or terminal receipts would weaken conclusions. | Full payload comparison, original-request reconstruction, and closed provenance controls. |

The selected external worker preserves the installed execution APIs.
Its body arm imports no Prisoma package.
Its canonical arm uses the owned Agent Bridge helper.
The comparator reopens exported bytes and reconstructs requests from the independently selected workload.
An external supervisor owns process observation and controlled faults.
The [command runner](M1_COMMAND_RUNNER.md) separately observes the supervisor's actual exit.

Five review lenses apply separately:

- Mathematics: derive exchange, chunk, payload, and cadence counts from typed inputs.
- Experimental validity: preserve both arms and every failed attempt without changing the selected workload.
- Authority: keep experimental ordering in Prisoma and producer execution in CREBAIN.
- Provenance: join exact freeze, source, wheels, installed files, runtime, case, and run identities.
- Operator understanding: distinguish application completion, stored evidence, and independently observed process retirement.

## Selection and checkpoints

The freeze uses schema `prisoma.m1-freeze.v1`.
It selects workload bytes, clean source, tool files, runtime, environments, deadlines, cases, and a private output root.
Each case creates a fresh child directory and persists `binding.json` before Prepare.
Bindings contain fresh UUID run, endpoint, and generation identities.
They also bind directory ownership and the exact freeze digest.

The worker uses two inherited pipes for bounded checkpoints.
It reports `prepared` after public Prepare and `tick1` after the first validated advance.
Each message uses `prisoma.m1-checkpoint.v1` and carries the freeze and complete case identity.
The supervisor replies with matching `prisoma.m1-continue.v1` fields.
Each message is one JSON object followed by LF, with a 4,096-byte limit.
EOF, timeout, extra fields, or identity drift fails the case.
Checkpoint waits consume the session budget.

The selected roster includes healthy body and canonical arms, caller loss, and renderer loss.
The observer records only captured descendant identities.
Emergency cleanup cannot establish natural retirement.
An expected-fault supervisor and command runner can exit successfully while the worker remains unhealthy.
Caller loss requires worker `-SIGKILL`; renderer loss requires a nonzero worker exit.
The worker preserves its public application cleanup receipt.

## Process observation and faults

| Approach | Assumption and benefit | Failure mode | Decisive control |
| --- | --- | --- | --- |
| Inspect and signal numeric PIDs | Shell tools provide a simple implementation. | PID reuse changes the target between inspection and signaling. | Reject a stale kernel process version. |
| Copy private structures through `ctypes` | Existing definitions reduce setup work. | An incorrect ABI layout corrupts identity observations. | Build against the installed SDK declarations. |
| Add simulator fault hooks | The simulator knows its child roles. | New hooks change the selected execution path. | Preserve the selected producer and client artifacts. |
| Use an operating-system containment service | A qualified service could track complete membership. | No such service is qualified for this Darwin selection. | Demonstrate the exact available service and ownership contract. |
| Combine SDK observation, direct ownership, and version-bound signaling | Separate mechanisms establish observed ownership and one selected fault. | Sampling misses short-lived descendants; SDK behavior can change. | SDK anomaly controls, owned-child retirement, and stale-token rejection. |

The selected observer never sends signals.
Its source uses installed SDK declarations rather than copied ABI layouts.
The installed `libproc.h` identifies its interface surface as private and subject to change.
Qualification applies only to the recorded Darwin version and SDK.

The renderer-loss case targets the main browser owner after exactly one validated tick.
The freeze binds its executable and its Node parent's executable separately.
The supervisor requires one active observed browser with that active observed parent.
The fault helper rejoins birth, parent, user identity, and executable before one fixed `SIGTERM`.
The kernel compares the selected audit-token process version during dispatch.
Signal acceptance supplies no application cleanup or retirement proof.
No ambiguous signal result authorizes a retry.

The comparator joins public runtime diagnostics to the selected browser and Node PIDs afterward.
It does not use diagnostic PIDs to select targets.
Executable paths and diagnostic strings do not attest loaded code or graphics hardware.
These controls do not contain hostile code or account for every descendant.

## Execute and verify

Publish the gated source before selecting an operational freeze.
Retain each attempt under a new private output root.
Do not replace failed attempts or change their selected deadlines.

The freeze closes workload, source, tool, runtime, browser, parent, environment, limit, output, and case fields.
Both environment inventories bind installed files, import origins, and their interpreter bytes.
Both arms select identical NCP and CREBAIN wheel bytes.
The canonical environment also selects the bridge and transcript wheels.
The body environment excludes Prisoma, NEST, and NumPy.
Wheel metadata and every packaged file must match the selected installed distribution.
The complete installed payload roster must match the archive, excluding four named installer-generated metadata files.
These selected wheels use direct package installation; an alternate `.data` installation scheme requires separate qualification.
Archive and payload limits apply to admitted bytes and entries.
The ZIP parser reads its central directory before the entry-count check.

Run the provider-free source and process controls with an installed canonical Python:

```text
just m1-campaign-check <installed-python> <new-external-gate-directory>
```

This gate runs the application suite and SDK observer, fault, supervisor, and command-runner controls.
Its native children are synthetic controls; they execute no simulator.
Run `uv sync --locked --group ui` and `just check` separately for the complete repository gate.
Candidate capture and its dedicated audit remain separate release requirements.

Use the selected canonical Python for inventories and readback commands:

```text
<python> -I -B <worker> inventory
<python> -I -B <runner> --freeze <freeze> --case <case-id>
<python> -I -B <worker> compare --freeze <freeze> --body <body-case> --canonical <canonical-case>
<python> -I -B <worker> verify-fault --freeze <freeze> --case <fault-case>
<python> -I -B <worker> copied-negatives --freeze <freeze> --case <canonical-case> --output <new-copy-directory>
<python> -I -B <worker> provenance-negatives --freeze <freeze> --body <body-case> --canonical <canonical-case> --output <new-control-directory>
```

Capture each environment's inventory with that environment's own Python.
The runner selects the case environment when it launches the supervisor.
The supervisor supplies the worker's two inherited checkpoint pipes.
Do not launch a native worker without that ownership chain.

Healthy comparison requires exact artifact rosters, original request reconstruction, complete payload equality, and all three terminal receipts.
Those receipts describe application cleanup, observed process retirement, and supervisor command completion separately.
Copied controls alter new files, reach deep verification failures, and reverify the original evidence.
Provenance controls retain selected readback variants and an altered workload copy.
They require exact rejection reasons for stale freezes, mixed artifacts, wrong identities, and mismatched ownership.
Both original arm records must reject the same altered workload during original-request reconstruction.
The controls preserve and reverify all original bytes without running either simulator again.

Fault verification requires the selected failure point and rejects any successful Finish or terminal run claim.
It joins the accepted prefix through public typed codecs and canonical execution receipts.
It checks exported payload bytes against their admitted manifests and canonical hashes.
A missing capture terminal remains a completeness failure.
Provisional transcript visitors do not become completed replay evidence.
The comparator preserves `cleanup_confirmed:false` even when every observed process retires.

## Evidence limits

The original workload expects 24 ticks, 44 payloads, 4,326,400 raw bytes, and 3,200 pressure samples.
Its expected transfer contains 168 chunks and 476 exchanges, with 36 exchanges on the heaviest call.
The canonical schedule contains 26 calls and 82 events.
These counts describe accounting, not measured performance or maximum supported scale.

The workload's historical reservation field describes a different quantity from the current transcript quota.
The owning capture API derives its reservation.
Neither quota arithmetic nor this campaign establishes process memory usage.

Matching payloads establish transfer equivalence for the selected engineering case.
They establish no real-time guarantee, controller stability, renderer oracle, learned-model benefit, or scientific validity.
Candidate source publication does not promote a release or satisfy native execution requirements.
