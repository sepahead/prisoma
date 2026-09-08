# Fit a complete Lance action snapshot

This reader fits the separately identified Lance action column through the existing sklearn scaler.
It does not establish equivalence with the original HDF5 training archive or the checkpoint's training normalizer.
Complete-snapshot fitting passed the source-bound experiment below.

The accepted snapshot contains 2,336,736 little-endian float32 pairs, totaling 18,693,888 bytes.
Its complete SHA-256 is `db62c4b5317b53e1e256140146b512419a79f27f805f411637a18be8bcc18011`.
The observed source is `galilai-group/lewm-pusht`, revision `ea321e392348e3c65a18ab0d685f00e57be2c3e0`, dataset `pusht_expert_train.lance`, version 1.
Rows retain fragment order and physical offsets within each fragment.
The three fragments contain 1,048,576, 1,048,576, and 239,584 rows.

The function accepts a local column snapshot and a new output directory.
It downloads nothing and accepts no caller-supplied statistics, expected digest, or verification receipt.
The source can have any filename.

In the [exact optional runtime](README.md#prepare-the-private-runtime), run:

```python
from pathlib import Path
from experiments.lewm.lance_actions import fit_pusht_lance_snapshot

scaler = fit_pusht_lance_snapshot(
    Path("actions.f32le"), Path("new-scaler-output")
)
print(scaler.statistics)
```

The reader checks the exact regular-file extent before reading.
It rejects final symlinks, changed input identity, extra bytes, missing bytes, and a wrong complete digest.
The fitting operation consumes an immutable private byte copy.
Changing the original file afterward cannot change the fitted rows.
Failed output directories remain available for inspection.

The reader uses the existing `complete-rows-64mib-v1` admission profile and one complete sklearn `fit`.
It preserves whole-row NaN exclusion, rejection of every infinity, and finite training values outside the candidate action box.
The [owned-reader contract](OWNED_DATASET_READER.md#storage-and-resource-bounds) defines the shared row and memory estimates.
Those estimates do not bound total process memory.

The output contains the exact `actions.f32le` snapshot, the scaler's inspection receipt, and a terminal transaction record.
Only the live returned object is an owner-issued scaler.
Copied output records cannot reconstruct that object or authorize command execution.

The source scope is `verified_frozen_lance_action_snapshot`.
The receipt keeps `original_hdf5_equivalence=false` and `training_normalization_qualified=false`.
Raw action conversion, empirical support, model quality, Agent Bridge execution, M2, and W1–W3 require their separate evidence.

## Observed complete-column fit

The September 8, 2026 experiment used Python 3.11.15, NumPy 2.4.6, and scikit-learn 1.9.0 on an M4 Max.
All 2,336,736 rows were finite and retained.
The owned read, private snapshot, fit, and output writes took 1.27 seconds.
The process observed 324.5 MiB peak RSS, below its frozen one-GiB observation threshold.
This single run does not establish a general latency or memory guarantee.

| Quantity | First coordinate | Second coordinate |
| --- | ---: | ---: |
| Mean | -0.007812564379916172 | 0.006860687229453032 |
| Population variance | 0.043458674726340484 | 0.04274499450737964 |
| Retained sklearn scale | 0.20846744284501714 | 0.20674862637362224 |

An independent `math.fsum` calculation reconstructed the mean and centered population variance.
Both coordinates passed the frozen absolute and relative tolerances of `1e-12`.
The [observation record](lance-action-fit.json) retains exact values, input identities, source hashes, and limits.
This comparison checks numerical agreement, not floating-point implementation proof or statistical population validity.

The exact optional runtime also passed all 13 snapshot controls.
The default environment passed 11 controls and ten subtests, with two optional sklearn tests skipped.
Controls cover source changes, size limits, symlinks, deadlines, descriptor failures, and failed terminal writes.
