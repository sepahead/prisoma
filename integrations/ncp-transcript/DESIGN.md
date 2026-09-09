# Modular transcript design

Status: source implementation with native capture evidence. Release and scientific gates remain separate.

The current causal journal requires neural, body, and monitor producers.
Its 64-KiB step limit cannot retain the measured 385,072-byte CREBAIN sensor batch.
A larger serialized step also conflicts with NCP's bounded control frames.

## Alternatives

| Approach | Assumption and benefit | Failure mode | Decisive control |
| --- | --- | --- | --- |
| Increase the old step limit | One object remains easy to inspect | Oversized control frames and mandatory peers remain | Capture a full multimodal batch |
| Drop sensor bytes | Summaries suffice for analysis | Lost evidence cannot support later comparisons | Reconstruct each original payload |
| Compress each step | Compression reduces typical storage | Worst-case bounds and latency remain uncertain | Incompressible input |
| Reuse the schema-2 experiment log directly | Existing authority avoids another event schema | Requires a separately reviewed PID contract change | Exact pinned compatibility gate |
| Store one file per sensor | Payloads stay separate | Cross-file publication and orphan recovery expand the contract | Crash between file commits |
| Use SQLite | Transactions and queries are established | File admission, journals, dependencies, and quotas need separate qualification | Storage failure and replay controls |
| Make Rerun canonical | A viewer already records data | A derived projection cannot supply missing bridge events | Reconstruct original request bytes |
| Capture after simulation | Producer interfaces stay unchanged | A crash can erase evidence of an executed request | Fail before postprocessing |
| Add capture to the NCP core | Every client gains one hook | Experiment storage becomes a protocol dependency | Standalone dependency check |
| Capture bounded exchanges at the host | Existing frames and optional peers compose | Host coverage and application completion still need their own gates | Durable request before dispatch; response before acknowledgement |

Select host capture as a separate optional package.
Preserve the old causal journal and the canonical schema-2 experiment log.
Use NCP's installed contracts and client state machine for transcript replay.
Do not duplicate its digest, acknowledgement, or predecessor rules.

## Ten review lenses

Correctness: preserve original request and response bytes, including rejected exchanges.
Composition: admit one to sixteen explicitly selected peers without required project roles.
Authority: the host supplies requests and streams; capture selects no action or executable.
Experiment validity: Agent Bridge remains the canonical experiment control plane.
Durability: synchronize each request before dispatch and each response before returning it.
Resources: reserve the maximum two-frame cost for every admitted exchange before opening the run.
Failure: retire on storage or transport failure; never retry an uncertain request.
Security: admit a new private regular file and reject final symlinks, shared permissions, and oversized records.
Maintenance: use one append-only file, fixed framing, SHA-256, and the existing NCP decoder.
Evidence: require terminal peer acknowledgements for transcript completion, without asserting application completion.

These alternatives received local review informed by prior release councils.
Independent review remains pending and supplies no release authority.

## Boundaries

The stream records what its supplied host boundary observes.
It cannot prove that a caller did not bypass that boundary.
Its digests establish content identity, not producer authenticity.
Typed decoding does not validate an application's complete causal history.
File synchronization does not promise power-loss behavior for every filesystem or storage device.
Logical quota admission does not allocate disk blocks or guarantee future free space.
This sequential local package establishes no remote security or real-time guarantee.

The separate [native CREBAIN run](README.md#crebain-sensor-capture) verifies payload reconstruction and capture before source-buffer release for one fixed workload.
It uses the ordinary host exchange hook and requires no NEST or monitor peer.
An embodied experiment must additionally bind Agent Bridge events to this transcript.
Combined execution, native lifetime faults, installed-host qualification, and scientific interpretation retain separate gates.
