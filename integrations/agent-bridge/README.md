# Application Agent Bridge

Dispatch declared application commands through Prisoma's Rust Agent Bridge.
The optional Python package uses the pinned PID-owned writer, schema, hashes, and replay validator.
It adds no estimator, simulator, sensor, or model dependency to the default Rust workspace.

The [execution guide](GUIDE.md) explains the command, transcript, timing, and sensor joins with a worked example.
The [frozen M1 campaign](M1_CAMPAIGN.md) owns the maintained transfer comparator and controlled process faults.
The [fixed-schedule performance baseline](M1_PERFORMANCE.md) supplies prospective measurement tools and synthetic controls.
Native performance execution remains **NOT RUN**.
The transfer campaign's [September 23 native evidence](evidence/M1_NATIVE_2026-09-23.md) compares 44 payloads totaling 4,326,400 bytes across body-only and canonical execution.
The earlier eight-command, 1,542,400-byte case remains historical evidence.
Neither case supplies learned-policy, exact-fork, statistical, or real-time qualification.

<picture>
  <source media="(max-width: 640px)" srcset="execution-mobile.svg">
  <img src="execution.svg" width="800" alt="Synchronize the canonical request, capture NCP execution, join the receipt, then synchronize the response before returning observations.">
</picture>

[Illustrated PDF](../../output/pdf/Recorded_Sensor_Execution.pdf) ·
[Mobile SVG](execution-mobile.svg) ·
[Direct vector for browser zoom](https://raw.githubusercontent.com/sepahead/prisoma/main/integrations/agent-bridge/execution.svg) ·
[Historical September 10 receipt](evidence/native-2026-09-10.json)

## Use with CREBAIN

Initialize Prisoma's pinned submodule before building the Rust extension.
Install the optional packages together in a separate Python 3.11-or-newer POSIX environment:

```bash
python -m pip install ./integrations/ncp-transcript './integrations/agent-bridge[crebain]'
```

The installation includes CREBAIN's Python sensor client and installed body launcher.
The producer, Bun, and optional graphics resources require the separate [runtime installation](https://github.com/sepahead/crebain/tree/main/integrations/ncp-force-ground-sensors/python#install-and-run-a-body-session).
Use that launcher for standalone sensor sessions and optional exchange capture.

`SensorExperiment` below retains the host-owned stream interface.
The host supplies trusted streams, an admitted CREBAIN binding and preparation, an initial target, and an absolute monotonic deadline.
It owns producer startup and verified process retirement.
The following code owns only the sensor session and its two new sibling evidence files:

```python
from prisoma_agent_bridge.crebain import SensorExperiment, verify_sensor_run

with SensorExperiment(
    "run.jsonl", "capture.ncp", reader, writer, binding, prepare,
    deadline=deadline,
) as experiment:
    for tick in range(prepare.planned_ticks):
        observation = experiment.advance(initial_target if tick == 0 else None)
        consume(observation)
    result = experiment.finish()

report = verify_sensor_run("run.jsonl")
```

The example holds the initial target after its first accepted command.
`advance` returns immutable observations after captured reads, source-buffer release, and canonical response synchronization.
Invalid targets and premature `finish` calls reject before dispatch.
Transport failures retain CREBAIN's original `SessionError`. The adapter retires and preserves the incomplete evidence.

Camera and microphone counts come from the selected preparation.
Each source keeps its own identity, cadence, acquisition time, and availability declaration.
The adapter adds no compulsory modality, neural peer, monitor, model, or PID estimator.
CREBAIN remains usable without Prisoma.

The optional dependency pins CREBAIN to `a5037a23a8e55d39ca0f09da2c25853c5209467b`.
Both sensor and transcript packages select NCP `c0465d40f1f2b9df2caf9793183d11e65ac9ec74`.
The current installation command requires both local Prisoma package paths.
This source milestone makes no published-PyPI-wheel promise.

The [historical native case](evidence/native-2026-09-10.json) retains CREBAIN `0e16f30f4970066e30c15f079b29c0e18f00cb25` and NCP `9ae64ac1a77c9cd0612284992a8711220428a6e3`.

## Own an installed producer

`owned_sensor_experiment` combines the installed body launcher with canonical command recording.
Supply the runtime admitted by CREBAIN's installer and the same typed preparation used by `SensorExperiment`.
The helper enters producer ownership inside the synchronized canonical Prepare callback.
It infers no executable path and registers no additional runtime.

```python
from prisoma_agent_bridge.crebain import owned_sensor_experiment

with owned_sensor_experiment(
    runtime, prepare, "run.jsonl", "capture.ncp", timeout_s=180,
) as experiment:
    for tick in range(prepare.planned_ticks):
        observation = experiment.advance(initial_target if tick == 0 else None)
        consume(observation)
    result = experiment.finish()
```

Call `finish()` explicitly after every planned tick.
A normal exit without canonical finalization becomes an exceptional owner exit.
Caught dispatch failures remain retained and prevent successful context completion.
Distinct execution, recording, and cleanup failures remain available as original exception objects.

Failure deduplication searches exception-group membership only.
Causes and contexts do not erase separately retained roots.
Each search inspects at most 4,096 root or child references, including repeated references.
Temporary traversal storage follows that bound instead of copying a complete child roster.
An exhausted search retains uncertain roots and sets `experiment.failure_graph_truncated=true`.
This flag describes the search, without truncating the original exception groups.
Diagnostic properties and exception formatting do not execute during that search.

After context exit, inspect `process_exit`, `diagnostics`, and `diagnostics_truncated` for the producer observations.
Those observations retain the installed launcher's scope and limitations.
The separate `failure_graph_truncated` flag reports incomplete failure-membership analysis.
Synthetic lifecycle controls do not qualify native producer retirement or scientific results.

## Readback

`verify_sensor_run` validates the canonical log through the pinned Rust reader.
It rejoins each execution receipt to its response hash and declared transcript span.
It then reconstructs each command through the installed sensor client, using captured frames as read-only inputs.
Each generated NCP request must equal the original captured bytes.
Every sensor payload must reconstruct under its producer manifest and match the canonical observation receipt.

The final artifact digest, complete planned tick count, all acknowledgements, and exact command roster must agree.
Readback starts no producer and selects no action.
It reports `sensor_session_replayed=true`, with `scientific_validation=false` and `producer_process_retirement_verified=false`.
The host must establish process retirement separately.

`inspect_sensor_run` exposes the same reconstructed observations through a streaming visitor.
Each frozen `SensorStep` includes the binding, preparation, returned catalog, requested target or hold, sensor batch, and exact transcript span.
The requested target is the recorded command. It is not a new action or an independent outcome label.
All source identities, tensor declarations, original payload bytes, and not-due slots remain available.

```python
from prisoma_agent_bridge.crebain import inspect_sensor_run

bytes_by_sensor = {}

def count_bytes(step):
    for reading in step.observation.readings:
        sensor_id = reading.manifest.sensor_id
        bytes_by_sensor[sensor_id] = (
            bytes_by_sensor.get(sensor_id, 0) + len(reading.payload)
        )

report = inspect_sensor_run("run.jsonl", count_bytes)
print(bytes_by_sensor)  # Use derived output only after successful return.
```

Visitor effects remain provisional until the complete inspection returns successfully.
A later malformed record, failed terminal join, or canonical-file mutation rejects the inspection.
Visitor exceptions propagate and stop further visits. Local replay resources close on every path.
The caller owns any retained observations, derived-output memory, and final publication transaction.
This interface supplies no feature transform, source grouping, target, PID estimate, or model-quality verdict.

One call's captured frames are retained during readback.
Its memory admission follows the selected scene's maximum exchange count and NCP's frame limit.
The historical six-tick example reserves 6,553,600 frame bytes for its largest call.
This bound excludes Python objects, reconstructed payloads, and canonical replay state.

## Recording contract

The caller supplies a finite method roster and application configuration before opening a new private run log.
Every request is appended and synchronized before its Python callback runs.
The callback consumes the recorded payload JSON and returns bounded result JSON.
The bridge records that result and its canonical response before returning.

Schema 2 stores the result in the named `prisoma.application_result.v1` compatibility envelope.
Its event type is `label_observed`, with `record_role=execution_receipt` and `is_outcome_label=false`.
This record is an execution receipt, not an outcome label.
The response hash must rejoin its exact result.

A callback exception or uncertain recording failure retires the adapter.
The incomplete prefix remains available for diagnosis.
The adapter supplies no retry, rollback, or protection against calls made outside its boundary.
An ordinary input rejection before dispatch leaves the session usable.

Host event timestamps use elapsed monotonic nanoseconds from bridge creation.
Response time is sampled after the callback returns and before response synchronization.
These timestamps are separate from simulation ticks, camera cadence, and microphone sample windows.

## Limits and files

One session admits 1–16 methods and at most 1,024 calls.
Input configuration, request objects, and result objects each have a 65,536-byte encoded JSON limit.
The callback owns any memory it allocates before returning.
These limits bound recorded data, not arbitrary installed Python code or process memory.

The caller supplies an existing trusted parent directory.
Files must be private regular files with one link and the current effective user as owner.
Creation rejects an existing path. Readers reject final symlinks and file changes during observation.
Synchronization calls the local filesystem. It does not guarantee every device's power-loss behavior.

Let `C` be the admitted call count and `J = 65,536` bytes.
The bridge admits at most `3C + 4` events and `J + 8,192 + C(2J + 4,096)` bytes.
The prefix contains two events. Each successful call adds a request, receipt, and response.
One optional artifact and the final run event occupy the remaining two slots.
The canonical writer independently enforces its own limits.

The optional terminal artifact is one relative sibling filename, with at most 1 GiB of observed bytes.
Its digest establishes content identity. Finalization does not make two files one atomic transaction.
An application reader must reopen the artifact and verify its recorded identity.

`inspect_runlog` uses the pinned canonical validator, then visits the original event sequence through its owned descriptor.
Visitor effects remain provisional until the function returns successfully.
Inspection retains the canonical replay state. It does not promise constant memory.
The generic inspector does not validate application completion or scientific meaning.

## Reproduce the checks

The source distribution includes the two required path dependencies and their pinned run-log implementation.
Exact Maturin 1.13.1 builds its wheel from a fresh extraction with `--sdist`.
CI checks this path, Rust diagnostics, installed Python controls, formal arithmetic, and the excluded crate's dependency graph.

```bash
just application-bridge-check /path/to/installed/python
python scripts/check_formal_models.py
cd integrations/agent-bridge
maturin build --sdist --release --locked --interpreter python --out /path/to/new/dist
```

The native case uses a host-managed CREBAIN construction and graphics runtime.
The [producer construction guide](https://github.com/sepahead/crebain/tree/main/integrations/ncp-force-ground-sensors#construction-and-qualification) owns those prerequisites.
The package does not infer producer paths or launch a renderer during import.

The canonical Markdown owns the publication text.
The [renderer](../../scripts/render_sensor_execution_pdf.py) and [publication receipt](publication.json) bind its PDF, fonts, and vector figures.
The tested publication runtime uses ReportLab 4.4.10 and svglib 1.5.1 with the receipt's exact macOS fonts.
From the repository root, use the isolated publication runtime:

```bash
python scripts/render_sensor_execution_pdf.py --check
```
