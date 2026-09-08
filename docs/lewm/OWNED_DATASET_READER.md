# Owned training-action reader

`experiments.lewm.dataset_actions` connects one verified compressed archive to an owned fitted action scaler.
Actual training-archive qualification remains `NOT RUN`.
Small synthetic archives test the transaction mechanics without receiving dataset authority.

The [Lance action-snapshot reader](LANCE_ACTION_SNAPSHOT.md) has a separate source identity.
Its fitted rows cannot establish original-HDF5 equivalence or training-normalizer compatibility.

## Exact dataset profile

The public `fit_pusht_training_scaler(archive, output)` function accepts two local paths.
It accepts no expected hash, verification flag, statistics, row array, or copied receipt.
The output directory must be new.
Its optional `row_profile` keyword selects one of two fixed resource profiles.
The default preserves `legacy-1m-rows-v1` and its original receipts.

| Field | Frozen value |
| --- | --- |
| Dataset repository | `quentinll/lewm-pusht` |
| Revision | `655cd446b9929369d7d406001da85c15d1457850` |
| Archive | `pusht_expert_train.h5.zst` |
| Compressed bytes | `13136247974` |
| Compressed SHA-256 | `7cfbd6d90fa2f27876379a5ff169715a36ed82edbda64f9e5b5bfa34d212f318` |
| Complete decoded bytes | `46300921856` |
| Action column | Root `/action`, complete original row order |
| Numeric extent | Native float32 or float64, shape `[N,2]`, with `2 <= N <= 1000000` |

The explicit `complete-rows-64mib-v1` profile replaces only the complete-row capacity with a 64-MiB byte ceiling.
It does not change the dataset identity, fit population, estimator, or action coordinates.
Neither profile guarantees that an unobserved archive satisfies its bounds.
Unknown selectors and caller-supplied numeric capacities reject.

The exact optional LeWM runtime is required before HDF5 or sklearn imports.
The reader imports neither Torch, the general upstream dataset package, nor HDF5 plugins.
It uses the same root-column slice as the pinned planning wheel.
The original standardized-only contracts, CEM, model projection, qualification plan, and PID pin remain separate.

## Ownership chain

1. Open the source as a bounded regular file without following its final symlink.
2. Copy the complete compressed bytes into a new private file while hashing them.
3. Reject source changes, incomplete extents, and a wrong complete digest.
4. Decode the verified private snapshot with the exact local Zstandard profile.
5. Bound stdout, stderr, elapsed decode time, decoder window memory, and free disk space.
6. Require the complete decoded extent, successful termination, group absence, and a reaped direct process.
7. Reopen the private decoded file through its retained file identity.
8. Check HDF5 metadata before allocating the action array.
9. Read and retain the exact action rows and their complete-row NaN mask.
10. Fit the actual pinned sklearn `StandardScaler` through the private issuer.

The caller receives an in-process `FittedActionScaler` only after terminal evidence persists.
Its `fit_rows` and `retained_row_mask` properties return views backed by immutable bytes.
Its inspection receipt binds the archive, decoder, decoded bytes, row metadata, fit recipe, and statistics.
Changing the original archive after private admission cannot replace the retained input.

The fit excludes each row containing any NaN, preserving the pinned `eval.py` rule.
Infinity anywhere rejects, including infinity in a row otherwise excluded for NaN.
At least two complete finite rows must remain.
Finite training values outside `[-1,1]` remain in the fit.
Candidate conversion separately rejects actual inverse-transformed float32 commands outside that raw action box.

## Storage and resource bounds

Root action must be a direct hard link to an allocated numeric dataset.
Soft links, external links, virtual datasets, external storage, and unallocated fill-only regions reject.
Contiguous and chunked storage are supported.
The admitted filters are built-in shuffle, deflate, and Fletcher32 with checked parameters.
Scale-offset and plugin filters remain unsupported.
Other columns are not read, including image arrays and unrelated links.

The legacy action array has a 16,000,000-byte extent limit and a separate one-million-row ceiling.
Each decoded HDF5 chunk retains its 16,000,000-byte limit under both profiles.
The HDF5 raw chunk cache has a four-MiB bound.
These limits do not describe total Python, sklearn, or native-parser memory.
HDF5 metadata parsing occurs before the action shape can be inspected.
This ordinary trusted workflow does not isolate a malicious native parser or arbitrary Python code.

For the explicit byte profile, let `N` be rows and `d` be bytes per coordinate, either four or eight.
The complete two-coordinate matrix has `A = 2*N*d` bytes.
Admission checks `N <= floor(67108864 / (2*d))` before reading or copying rows.
This permits at most 8,388,608 float32 rows or 4,194,304 float64 rows.
The fitted receipt records the selected profile, actual matrix bytes, and dtype-specific row limit.
The archive transaction declares both dtype limits without presenting the float32 limit as float64 admission.

The planning estimate `E = 8*A + 32*N` bytes accounts for row copies, filtered rows, masks, and sklearn scratch.
Its conditional maximum is 768 MiB for float32 and 640 MiB for float64.
These estimates do not prove allocator behavior, native-parser memory, or total process RSS.
The [integer obligations](../../formal/lewm_row_admission.smt2) check these formulas and their admitted and excessive boundaries.
Separate synthetic process measurements report observed memory under their frozen workload.

Both profiles retain one whole-array sklearn `fit` after the same complete-row NaN exclusion.
Splitting rows across `partial_fit` calls can change floating-point reduction order and fitted scales.
The larger resource profile grants no new dataset or execution authority.

The compressed-copy loop has a 900-second deadline checked between bounded file operations.
The decoder has a 1,800-second watchdog and a 256-MiB window-memory limit.
The window limit is not a process RSS limit.
Stdout cannot exceed the declared complete decoded size; stderr cannot exceed 65,536 bytes.
Reserve both new files plus a 24-GiB free-space floor before starting.
The reader checks the remaining space during copying and decoding.

The Zstandard executable and its non-system library closure have fixed local paths and byte identities.
The receipt also records the macOS version, build, and declared system-library linkage.
The decoder receives a small explicit environment without `DYLD_*` injection.
These local observations do not attest loaded machine code or Darwin shared-cache bytes.

## Failure and evidence

The reader retains failed directories and partial files.
`failure.json` and `DatasetReadError.receipt` distinguish the primary error from observed decoder cleanup.
Permission denial alone never proves that a process group disappeared.
A later verified group absence and leader reap can confirm cleanup while preserving the earlier signal observation.
The original decode or output failure remains a failure.

The Darwin decoder retains its direct child with `waitid(..., WNOWAIT)` while any group signal remains possible.
Every signal requires an unreaped child identity; an already-reaped child or lost ownership blocks signaling.
The terminal wait revokes signal authority before reaping.
Subsequent group checks observe absence and never signal a potentially reused group number.
The workflow requires default SIGCHLD handling and exclusive ownership of child waiting.
Its fixed decoder must remain within the new process group; this contract does not cover escaped or daemonized processes.
The exact optional Python lacks an `os.waitid` wrapper, so the reader prepares the native Darwin system API before creating a child.
Its reviewed structure layout and constants are checked, and a non-child identity must reject before launch.
The receipt names that API and ABI; it does not attest loaded system-library bytes.

Descriptor ownership is relinquished before each close attempt.
A close failure cannot suppress another owned descriptor's close or cause an uncertain descriptor number to be retried.
The primary operation failure and secondary close observations remain separate.
Snapshot read and copy failures remain primary when either owned file close also fails.
Their secondary errors remain in `compressed_snapshot.descriptor_close_errors`.
The retained snapshot probe closes its descriptor when identity or stream validation fails.
The decoder closes stdout, stderr, and its output descriptor independently after process cleanup.
Each close failure remains in `decode.resource_close_errors`; final decode counters remain available.
An original decoder failure stays primary. Any decoder close failure blocks scaler issuance.

`scaler.json` records the fitted handle's inspection receipt.
`transaction.json` records completed fitting only after that receipt persists.
Neither file can reconstruct an owner-issued scaler or authorize action execution.

`fit_control_archive_scaler()` exercises the same transaction with a bounded synthetic archive and declared decoded size.
Its compressed and decoded limits are four MiB and eight MiB.
It always issues `synthetic_control_only`, regardless of its source identifier or receipt text.
Both synthetic fit functions accept the same closed `row_profile` selector and preserve synthetic scope.

A successful reader supplies dataset-bound normalization only.
It supplies no Agent Bridge execution, training-support guarantee, model validation, physical calibration, M2, or W1–W3 completion.
Dataset rights and scientific population suitability remain separate review objects.
