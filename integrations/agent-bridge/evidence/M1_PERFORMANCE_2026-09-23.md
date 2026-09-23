# M1 performance observations: September 23, 2026

The corrected campaign completed all 192 frozen executions with identical original sensor payload bytes across routes.
Every one of its 4,608 route deadlines and 4,608 export deadlines was missed.
These results establish coarse measurement and complete accounting for this workload, without real-time or release qualification.

The [receipt projection](native-m1-performance-2026-09-23.json) preserves exact nanosecond summaries, identities, counters, and limitations.
The [performance owner](../M1_PERFORMANCE.md) defines the maintained tools, schedule, bounds, and rerun prerequisites.

## Fixed workload and observed timing

Each execution used the same [24-tick M1 workload](../tests/fixtures/m1.workload.v1.json), including its seed, targets, RGB, thermal, and pressure sensors.
Each retained 44 payloads totaling 4,326,400 original bytes.
The 32 timing blocks each contained direct, NCP body, and canonical routes at minimal and detailed instrumentation levels.
The freeze rotated route order and reversed alternate six-block groups.

Each arm supplied 768 full-run samples and 576 samples from the declared warmed suffix, ticks 7–24.
The table reports route service time from actual tick start through accepted observation access, serialization, and required release.
The canonical interval also includes its required durable recording.
Values use milliseconds and nearest-rank quantiles.

| Arm | p50 ms | p99 ms | Maximum ms |
| --- | ---: | ---: | ---: |
| Direct / minimal | 16.508 | 33.764 | 37.383 |
| Direct / detailed | 16.847 | 34.185 | 35.471 |
| NCP body / minimal | 61.359 | 140.002 | 214.140 |
| NCP body / detailed | 41.635 | 94.010 | 101.224 |
| Canonical / minimal | 98.938 | 179.232 | 278.316 |
| Canonical / detailed | 117.018 | 191.270 | 226.059 |

At 576 samples, p99.9 equals the sample maximum.
Neither this warmed suffix nor these quantiles establish steady state, a population tail, or a worst-case bound.
The 120-Hz offers retained their original origin and deadlines when execution fell behind.
No deadline was rebased to hide backlog.

The receipt projection also reports offered-to-completion delay and paired whole-route differences.
Those differences pair matching block, tick, and instrumentation level before calculating quantiles.
They are not isolated protocol costs.
Detailed and minimal arms used separate executions; their differences include system noise.

## Completion and retirement

The campaign retained 8,448 payloads totaling 830,668,800 original bytes across its 192 executions.
The 128 NCP executions each recorded 476 exchanges, totaling 60,928 exchanges.
All canonical runs passed their maintained original-command and capture checks.
The outer campaign exited zero after approximately 1,397.851 seconds.

All 192 direct commands exited zero without timeout, forced termination, or output overflow.
The 64 standalone owners confirmed cleanup, and all 128 Python SDK process receipts confirmed cleanup.
Separately, the observer recorded 2,037 birth identities summed across cases and observed every recorded identity retire.
No observed identity remained, and no descendant signal was sent.
This process evidence covers sampled owned descendants, without exhaustive tracking or hostile-process containment.

## Failed first attempt remains part of the record

Campaign001 completed two direct cases before a body worker rejected its 32-character hexadecimal run identifier.
The rejection occurred before that body producer launched.
The frozen denominator remains two completed, one failed, and 189 unattempted cases.
The original 190 uncompleted cases were not replaced or rerun.

The maintained repair admitted canonical UUIDv4 strings through the actual installed NCP owner before effects.
Campaign002 used a separately reviewed freeze and retained the workload, measured intervals, case order, bounds, and installed artifacts.
The [historical attempt record](../M1_PERFORMANCE.md#first-retained-native-attempt) retains its original freeze and terminal digests.

## Source, custody, and limits

Campaign002 ran published Prisoma `d63eefed1a05fbecaa38718ed81484c3d0e442cc`, following source repair `51d4cc707beb4f252d78fa0ffa52ccfba9b2eed2`.
The installed CREBAIN client remained `a5037a23a8e55d39ca0f09da2c25853c5209467b`; its native runtime separately identified `d397905fe51c687b229a496af3b7dac572595e55`.
The NCP SDK remained `c0465d40f1f2b9df2caf9793183d11e65ac9ec74`.
The native runtime manifest retained SHA-256 `82f8bf3c0ef08394c87c3b9f94468d2fc4ef4a610de9cb91315996058d450457`.

The maintained source, workload, and summary are public.
Original timing rows, payloads, captures, inventories, command receipts, and process journals remain private, locally observed artifacts.
Their hashes identify retained bytes; they do not attest loaded code or replace originals for replay.
Postterminal checks rejoined the frozen inputs and all ten maintained source copies to their published Git bytes.

Other authorized work was active on this machine; this was not a dedicated-host benchmark.
Preparation intervals include different route-specific verification work.
The study does not measure complete allocations, copies, GPU resources, storage contention, long-run behavior, or supported scale.
Full W6 resource acceptance, scientific conclusions, and NCP v1 release remain separate gates.
