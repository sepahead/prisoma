"""Fit the separately identified, complete Lance action-column snapshot.

This issuer verifies actual row bytes. It accepts no asserted statistics or
verification receipt. Lance-to-original-HDF5 equivalence remains unestablished.
"""

from __future__ import annotations

from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import stat
import time

import numpy as np

from .supported_actions import (
    COMPLETE_ROW_PROFILE,
    FittedActionScaler,
    _admit_row_shape,
    _fit_owned_rows,
)


@dataclass(frozen=True)
class _Column:
    rows: int
    sha256: str
    scope: str
    source: dict


_LANCE = _Column(
    2_336_736,
    "db62c4b5317b53e1e256140146b512419a79f27f805f411637a18be8bcc18011",
    "verified_frozen_lance_action_snapshot",
    {
        "repository": "galilai-group/lewm-pusht",
        "revision": "ea321e392348e3c65a18ab0d685f00e57be2c3e0",
        "dataset": "pusht_expert_train.lance",
        "version": 1,
        "column": "action",
        "fragment_rows": [1_048_576, 1_048_576, 239_584],
        "row_order": "fragment_order_then_physical_row_offset",
    },
)


def _identity(value: os.stat_result) -> tuple:
    return (
        value.st_dev,
        value.st_ino,
        value.st_size,
        value.st_mtime_ns,
        value.st_ctime_ns,
    )


def _read_column(path: Path, profile: _Column) -> bytes:
    admission = _admit_row_shape((profile.rows, 2), 4, COMPLETE_ROW_PROFILE)
    size = admission["complete_row_bytes"]
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    primary = None
    try:
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode) or before.st_size != size:
            raise ValueError("Action snapshot requires its exact regular-file extent")
        deadline = time.monotonic() + 60
        blocks, count, digest = [], 0, hashlib.sha256()
        while True:
            if time.monotonic() >= deadline:
                raise TimeoutError("Action snapshot read exceeded its deadline")
            block = os.read(descriptor, min(1024 * 1024, size - count + 1))
            if not block:
                break
            count += len(block)
            if count > size:
                raise ValueError("Action snapshot grew beyond its admitted extent")
            digest.update(block)
            blocks.append(block)
        if count != size or digest.hexdigest() != profile.sha256:
            raise ValueError("Action snapshot bytes differ from the frozen column")
        if _identity(os.fstat(descriptor)) != _identity(before):
            raise ValueError("Action snapshot changed during its private read")
        return b"".join(blocks)
    except BaseException as error:
        primary = error
        raise
    finally:
        # Relinquish this descriptor exactly once, including a failed close.
        try:
            os.close(descriptor)
        except BaseException as error:
            if primary is None:
                raise
            primary.add_note(f"Action snapshot close also failed: {error!r}")


def _save(path: Path, data: bytes) -> None:
    with path.open("xb") as stream:
        if stream.write(data) != len(data):
            raise OSError("Private snapshot write was incomplete")
        stream.flush()
        os.fsync(stream.fileno())


def _json(value: dict) -> bytes:
    return (
        json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + "\n"
    ).encode()


def _fit_snapshot(source: Path, output: Path, profile: _Column) -> FittedActionScaler:
    _admit_row_shape((profile.rows, 2), 4, COMPLETE_ROW_PROFILE)
    output.mkdir(mode=0o700)
    try:
        data = _read_column(source, profile)
        _save(output / "actions.f32le", data)
        rows = np.frombuffer(data, dtype="<f4").reshape(profile.rows, 2)
        fitted = _fit_owned_rows(
            rows,
            {
                "schema": "prisoma.lewm.lance-action-snapshot.v1",
                "scope": profile.scope,
                "source": profile.source,
                "source_sha256": profile.sha256,
                "original_hdf5_equivalence": False,
                "training_normalization_qualified": False,
            },
            row_profile=COMPLETE_ROW_PROFILE,
        )
        record = fitted.receipt()
        if record["rows"]["sha256"] != profile.sha256:
            raise ValueError("Retained fitted rows differ from the verified snapshot")
        _save(output / "scaler.json", _json(record))
        _save(
            output / "transaction.json",
            _json(
                {
                    "schema": "prisoma.lewm.lance-action-transaction.v1",
                    "status": "complete",
                    "scope": profile.scope,
                    "scaler_sha256": fitted.sha256,
                    "source_sha256": profile.sha256,
                    "raw_actions_executed": False,
                }
            ),
        )
        return fitted
    except BaseException as error:
        error.add_note(f"Failed action-snapshot output remains at {output}")
        raise


def fit_pusht_lance_snapshot(source: Path, output: Path) -> FittedActionScaler:
    """Fit one complete pinned action snapshot into a new private directory.

    source contains little-endian float32 pairs in observed physical row order.
    It can have any filename. The profile fixes its extent and complete digest.
    The returned handle does not establish the original checkpoint's normalizer.
    """
    return _fit_snapshot(Path(source), Path(output), _LANCE)
