# Fixed-schedule M1 performance baseline

Status: maintained source and synthetic controls. Native performance execution remains **NOT RUN**.
Scope: prospective installed M1 engineering measurements on one selected Darwin host.
This study cannot grant NCP v1 release or real-time qualification.
The owner-approved broader performance and resource profile remains unavailable.

## Owning contracts

The [M1 campaign](M1_CAMPAIGN.md) owns the original workload and installed execution joins.
The [architecture](../../ARCHITECTURE.md) owns component and authority boundaries.
The [recorded execution guide](GUIDE.md) defines the canonical transfer arithmetic.
The earlier native M1 evidence supplies no per-tick latency distribution or allocation count.

The independent direct baseline uses CREBAIN's standalone owner.
The canonical arm uses Prisoma's Agent Bridge and its original NCP capture.
Neither standalone arm becomes a canonical Prisoma experiment record.

## Compared approaches

| Approach | Assumption | Benefit | Failure mode | Decisive experiment |
| --- | --- | --- | --- | --- |
| Transport echo microbenchmark | Small messages represent sensor transfer | Cheap isolated framing timing | Omits native work, payloads, and durability | Compare its cost coverage with actual M1 exchange spans |
| Whole-session elapsed time | Startup and operation are separable afterward | No internal modifications | Cannot recover per-tick deadlines or stages | Record preparation, each advance, and retirement separately |
| Public-boundary spans on installed paths | Added clocks have measurable overhead | Exact workload and source identities | Instrumentation can dominate short operations | Pair minimal and detailed instrumentation with fixed order |
| New native runtime trace hooks | Rebuilt instrumented runtime preserves behavior | Fine-grained physics, graphics, and serialization costs | New artifact needs full gate and installed parity | Rebuild, qualify, and compare unchanged observation bytes |
| System allocation profiler | Tool observes all selected processes and allocators | Native allocation and process memory evidence | Missing descendants, privileges, or GPU coverage | A declared allocation canary must be observed before interpretation |
| Python allocation snapshots | Python retained blocks represent total allocation | Easy client-only memory evidence | Misses freed, native, Rust, JavaScript, and GPU allocations | Report only its actual domain; never substitute it for full allocation counts |
| Static operation accounting | Source loops determine all costs | Exact byte/chunk/exchange expectations | Does not measure hidden copies, allocations, or latency | Join static counts to actual frames and retain unmeasured domains |
| Shared-memory prototype | Copy cost dominates the residual budget | Potential copy reduction | Adds lease, crash, authority, and layout obligations prematurely | Require measured bottleneck and benefit before implementation |

Select public-boundary paired measurement plus actual exchange accounting.
Retain allocation instrumentation as a separately gated instrument qualification.
Do not rebuild or alter the published native runtime for this baseline.
This selection measures coarse stages and exposes remaining fine-grained gaps.

## Frozen experiment shape

Use the original 24-tick M1 workload with seed 4 and targets at ticks 1 and 13.
Keep 320×240 RGB every second tick and 160×120 thermal every third tick.
Keep one 16-kHz pressure microphone on every tick.
The expected original bytes remain 4,326,400 per completed route.

Select the prior M1 runtime through an explicitly supplied, independently reviewed M1 freeze.
Use the exact independently installed body and canonical Python environments selected by that freeze.
Record the runtime, installed distributions, scripts, and input identities before execution.
Recheck their bytes after the campaign.

Run 32 blocks with six fixed arms per block.
The arms are direct, NCP body, and NCP canonical, each at two instrumentation levels.
Rotate the six-arm order by block index.
Reverse alternate six-block groups.
Seal the exact expanded order before running any case.

All arms share the same specification, action schedule, sensor roster, and original sample bytes.
They use fresh owners, bindings, and graphics generations.
No arm accesses another arm's live handles or mutable state.

Every arm records minimal boundary clocks and complete payload hashes.
Detailed arms additionally record bounded exchange and public function spans.
Compare complete payload bytes at each matching tick and sensor.
Store all original payload bytes under a predeclared quota.

Use 6 GiB for the complete 192-case artifact envelope.
Reserve 64 MiB for case artifacts and 8 MiB for command streams before each launch.
Release unused reservations only after measuring the retained case bytes.
Stop before launch when the remaining campaign quota cannot cover that reservation.
The quota need not admit every case at its worst-case reservation size.

Bound each worker's benchmark exports to 48 MiB.
Bound retained original wire frames to 16 MiB and reserve each reply before its request.
The maintained canonical owner separately reserves its declared capture extent.

Recheck the complete case directory against 64 MiB after process exit.
Require the existing 24-GiB free-disk floor before each case.
This disk check makes no RAM claim.

The first six ticks in every case are the declared warmup.
Keep those ticks in the full offered-load and failure report.
Report ticks 7–24 separately as the selected warmed suffix.
Do not claim steady state from this warmup choice.

Across 32 cases, the warmed suffix has 576 samples per arm.
Report p50, p99, p99.9, and maximum with nearest-rank quantiles.
At this sample size, p99.9 equals the sample maximum.
It is not a reliable estimate of a population tail or a worst-case bound.

## Clocks, scheduling, and costs

Each process uses its own monotonic nanosecond clock.
Never subtract timestamps from different processes.
The origin `t0` is the process-local preparation completion time, measured in nanoseconds.
Preparation includes runtime verification and owner admission.

Record the exact verification scope with each route's preparation interval.
The direct interval includes its manifest/source join and installed native imports.
The Python intervals include complete `InstalledRuntime.open` verification.
The launcher performs complete installed-runtime verification before and after the campaign.

These preparation intervals are route-specific costs, not equivalent-work microbenchmarks.
Record process command duration independently.
Record explicit Finish, canonical finalization, and context retirement separately.

Start the offered clock after preparation.
Tick k has scheduled offer time t0 + (k - 1) / 120 seconds.
Its deadline is t0 + k / 120 seconds.
Let integer `k` identify a body tick, from 1 through 24.

The stored offer is `t0 + ceil((k - 1) × 1,000,000,000 / 120)` nanoseconds.
The stored deadline numerator is `120 × t0 + k × 1,000,000,000`.
A completion at nanosecond `n` misses that deadline exactly when `120 × n` exceeds its numerator.
Use integer rational arithmetic for nanosecond schedule projection.

Wait only until the fixed offer time when ahead.
When late, keep the original offer time and execute sequentially.
Never shift later offer times to hide backlog.
No mutable tick can start before its predecessor has a known accepted outcome.

For every planned tick, retain scheduled time, actual start, accepted observation, and release completion.
The direct arm additionally records schedule, native advance, observation read/parse, and release.
The body arm records advance/read validation separately from source-buffer release.
The canonical arm times its actual durable advance call and exact Finish.

Detailed NCP spans classify actual framing writes/reads and SDK contract/canonical recording work.
Frame spans include remote service, scheduling, and backpressure.
They do not isolate transport cost.
They cannot count copies or allocations or alone justify batching.

Nested spans must retain inclusive and exclusive meaning.
Never sum overlapping durations.
Request/response byte counts come from actual frame objects.
Record route_complete_ns after complete observation access, serialization, and required release.
For canonical operations, route_complete_ns also includes their required durable recording.

Record export_complete_ns after hashing and synchronizing every original payload and its export row.
Apply identical benchmark hashing and export requirements in every arm.
The primary route deadline uses route_complete_ns.
Report export deadline misses separately.

Both deadlines retain the same original offered schedule.
Export delays also delay subsequent offers in this sequential workload.
Preserve that backlog.

Report startup, operation, release, export, and retirement costs separately.
Record offered-to-complete delay and deadline misses for every tick.
No missing or non-finite measurement can become a zero latency.

Paired differences use matching block, tick, and instrumentation level.
Compute difference samples first.
Never subtract unrelated percentile summaries.

These are empirical whole-route differences, not isolated protocol overhead.
Compare detailed versus minimal arms as instrumentation-cost observations.
Their different executions and system noise preclude exact overhead subtraction.

## Outcomes and limitations

A completed accounting case requires all 24 ticks, 44 payloads, and exact paired original bytes.
The NCP paths must retain the expected 476 exchanges and per-call grouping.
Canonical results must satisfy the maintained capture and command verifier.

A failure remains in its original arm and block.
No replacement trials or threshold adjustment are allowed.
Uncertain cleanup stops future native launches.
All remaining planned cases are reported as uncompleted.

Use 180 seconds for each owner's session and the existing separate 205-second cleanup grace.
The direct native owner keeps its existing 30-second graphics and 45-second retirement bounds.
Use a finite 45-minute campaign limit.
An outer timeout grants no claim that descendants retired.
The process runner must keep exact original errors and observed ownership evidence.

Use the reviewed birth-bound Darwin observer without signaling descendants.
Record any direct-child timeout, interruption, forced termination, and output overflow separately.
An observation failure remains sticky and stops further native launches.

The outer runner interrupts its owned child after the declared session interval.
Its separate cleanup grace precedes any forced termination of that same unreaped child.
Observed disappearance establishes only retirement of identities actually seen.

The 120-Hz deadline comes from the offered workload, not a newly invented release budget.
A measured violation is a failed deadline observation.
Passing every sampled deadline still cannot establish real-time qualification.
The baseline remains incomplete for full W6 resource acceptance until allocation, copy, GPU,
storage contention, long-run, and approved platform/resource policy gates are satisfied.

Optimize only after inspecting stage evidence.
A direct service cost above 8.333 milliseconds records a missed deadline for that standalone sample.
It cannot prove a zero-transport lower bound for another execution.
Before attributing avoidable exchange cost, require separately qualified instrumentation on the actual service path.
If that evidence identifies avoidable exchange cost, evaluate bounded typed batches first.
Do not pipeline unresolved mutable operations or introduce shared memory from arithmetic alone.

## Required controls before execution

Verify schedule arithmetic at exact offer/deadline boundaries and after deliberate overruns.
Reject non-finite, reversed, foreign-clock, duplicate, missing, and extra timing rows.
Verify nearest-rank quantiles and pairing before evaluating native outcomes.
Reject incorrect payload bytes, sensor/tick joins, frame counts, and source identities.
Preserve failure rows and planned-but-uncompleted rows.

A detailed wrapper must forward arguments, return values, and original exceptions unchanged.
Verify span stack closure and explicit overflow before applying wrappers to installed paths.
Runtime and package identities must match both sides of the study.

## Maintained location

| Approach | Assumption and benefit | Failure mode | Decisive control |
| --- | --- | --- | --- |
| Existing integration script siblings | The integration owns the paired measurement and can reuse its helpers. | Private path inference survives an incomplete migration. | Select every external input explicitly. |
| New performance subdirectory | Grouped files improve discovery. | Additional parent-path logic encourages helper copies. | Reopen one maintained helper identity. |
| Repository-root scripts | General tools have a familiar location. | Application requirements become separated from their owner. | Find all prerequisites from the entrypoint. |
| New installed performance package | Installed commands simplify distribution. | Packaging expands the body environment's dependencies. | Verify unchanged installed dependency boundaries. |
| Private archive and public summary | Original experiment bytes remain preserved. | Readers cannot discover maintained rerun tooling. | Reconstruct a source-bound run from public instructions. |

The selected scripts reuse [the M1 helper](scripts/m1_campaign.py) and [process observer](scripts/owned_observer.py).
No helper, producer, estimator, or installed bridge implementation is copied.
The body environment remains independent of Prisoma packages.

The maintained successor preserves the reviewed private cut's measurement logic.
Its deliberate changes cover file locations, explicit inputs, input admission, formatting, controls, and documentation.
The shared file reader also closes descriptors when stream construction rejects an input.
It preserves the original read failure and each additional stream or descriptor cleanup failure.
The prior private cuts remain preimplementation evidence with no native performance outcomes.
The published successor requires its own prospective freeze and terminal evidence.

## Source controls

Run the complete repository gate before source publication:

```text
uv sync --locked --group ui
just check
```

Run the integration gate with explicit installed tools and a new external output directory:

```text
just m1-performance-check <installed-canonical-python> <selected-bun> <new-external-gate-directory>
```

This gate checks the application bridge, fixed clocks, framing, report consumer, input admission, diagnostic bounds, and selected Darwin observer.
Its process controls launch synthetic Python children, without a simulator or graphics owner.
The selected-manifest control requires explicit `PRISOMA_PERFORMANCE_MANIFEST` and `PRISOMA_PERFORMANCE_MANIFEST_SHA256` environment values.
Set both before the integration gate to qualify that selected metadata file.
Without the path selection, that one file-specific control is skipped.
The root gate and commit-bound candidate audit remain separate requirements.

## Freeze, execute, and report

Publish the gated source before creating an operational freeze.
Select the prior M1 freeze, current design, and independent review by direct absolute paths.
Every selected input must be a regular file without symlinks, containing at most 8 MiB.
Outer JSON readers reject duplicate decoded keys and non-finite numbers.
The prior freeze also rejects another profile.

File admission uses bounded, nonblocking, no-follow reads and rejects a changed descriptor or path identity.
These input admission failures occur before runtime inspection or output creation.
The design and review selections bind original bytes without interpreting approval or runtime qualification.

Python metadata uses the existing M1 parser and its unchanged structural limits.
A finite-value check also rejects exponent overflow, such as `1e400`.
Python preparation obtains executable identities from the already verified public `InstalledRuntime` object.

The Bun metadata scanner preserves native `JSON.parse` values.
It detects decoded duplicate keys before full value construction.
Its token cursor only advances.
An unterminated string cannot restart a search at escaped quotes.
Its separate envelope permits 262,144 value nodes, depth 32, and 20,000 keys or items per container.

Every container and primitive value counts as one node.
Keys are not additional value nodes.
Each key already owns a value node.
Each nonroot value has one parent object key or array position.
Thus the global key count cannot exceed the value count minus one.

The selected preimplementation manifest contains 150,722 value nodes and 124,696 keys within 6,198,942 bytes.
Its depth is four, and its largest array has 15,656 items.
This observation explains the separate metadata envelope without changing M1 or protocol parser limits.
The runtime-manifest read and scan remain part of the direct route's declared preparation cost.
Measured owner and tick operations retain their original decoders and timing boundaries.

The caller must separately join source publication, prior native evidence, and the current review before execution.

Use the selected canonical Python for these commands:

```text
<python> -I -B integrations/agent-bridge/scripts/perf_campaign.py freeze <new-output> \
  --m1-freeze <absolute-prior-M1-freeze> \
  --design <absolute-M1_PERFORMANCE.md> \
  --review <absolute-current-source-review>
<python> -I -B integrations/agent-bridge/scripts/perf_campaign.py run <new-output>/freeze.json
<python> -I -B integrations/agent-bridge/scripts/perf_report.py <new-output>
```

The command runner owns each worker and selects its frozen interpreter or Bun executable.
Do not launch a native worker directly.
The freeze records exact helper, observer, design, review, runtime, executable, environment, workload, case, and limit identities.
The `design_closure` field stores the selected review artifact's identity, without an approval claim.

Keep outputs outside the source checkout.
Retain each failed attempt and its remaining case roster.
Report generation requires complete accounting and all 192 original cases.
It reopens selected bytes and rejects invalid lifecycle clocks before calculating summaries.
A report is a derived measurement view, without loaded-code attestation or release authority.

Publish a dated summary only after independent terminal readback.
Classify private original artifacts separately from public source and summaries.
A later source change requires a new freeze for any rerun.
