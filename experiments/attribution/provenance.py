"""Bind attribution evidence to one import-time source manifest.

The package imports this module before its implementation modules. The manifest
records the exact source bytes present at that boundary. It is provenance, not
loaded-code, dependency, interpreter, or process attestation.
"""

from __future__ import annotations

import hashlib
import json
import os
import platform
import stat
import sys
from pathlib import Path

import numpy as np

_SOURCE_NAMES = (
    "__init__.py",
    "__main__.py",
    "attribute.py",
    "faithfulness.py",
    "model.py",
    "probe.py",
    "provenance.py",
    "runlog.py",
)
_PACKAGE_DIRECTORY = Path(__file__).resolve().parent
_MAX_SOURCE_FILE_BYTES = 2 * 1024 * 1024
_MAX_SOURCE_TOTAL_BYTES = 8 * 1024 * 1024


def _identity(metadata: os.stat_result) -> tuple[int, int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_size,
        metadata.st_mtime_ns,
        metadata.st_ctime_ns,
    )


def _read_source(path: Path) -> bytes:
    """Read one bounded package source from a stable regular-file identity."""

    try:
        before = path.lstat()
    except OSError as error:
        raise RuntimeError(f"cannot inspect attribution source: {path}") from error
    if not stat.S_ISREG(before.st_mode):
        raise RuntimeError(f"attribution source is not a regular file: {path}")
    if before.st_size > _MAX_SOURCE_FILE_BYTES:
        raise RuntimeError(f"attribution source exceeds its byte limit: {path}")

    flags = os.O_RDONLY
    flags |= getattr(os, "O_CLOEXEC", 0)
    flags |= getattr(os, "O_NOFOLLOW", 0)
    flags |= getattr(os, "O_NONBLOCK", 0)
    try:
        descriptor = os.open(path, flags)
    except OSError as error:
        raise RuntimeError(f"cannot open attribution source: {path}") from error
    try:
        opened_before = os.fstat(descriptor)
        if not stat.S_ISREG(opened_before.st_mode):
            raise RuntimeError(f"attribution source is not a regular file: {path}")
        if _identity(before) != _identity(opened_before):
            raise RuntimeError(
                f"attribution source changed before it was opened: {path}"
            )
        with os.fdopen(descriptor, "rb", closefd=False) as source:
            payload = source.read(_MAX_SOURCE_FILE_BYTES + 1)
        opened_after = os.fstat(descriptor)
        after = path.lstat()
        if (
            not stat.S_ISREG(after.st_mode)
            or _identity(opened_before) != _identity(opened_after)
            or _identity(opened_after) != _identity(after)
            or len(payload) != opened_after.st_size
        ):
            raise RuntimeError(f"attribution source changed while being read: {path}")
        if len(payload) > _MAX_SOURCE_FILE_BYTES:
            raise RuntimeError(f"attribution source exceeds its byte limit: {path}")
        return payload
    except OSError as error:
        raise RuntimeError(f"cannot read attribution source: {path}") from error
    finally:
        os.close(descriptor)


def _capture_manifest_json() -> bytes:
    observed = tuple(
        sorted(
            path.name for path in _PACKAGE_DIRECTORY.iterdir() if path.suffix == ".py"
        )
    )
    if observed != tuple(sorted(_SOURCE_NAMES)):
        raise RuntimeError(
            "attribution source inventory changed; update the provenance inventory"
        )

    source_hashes: dict[str, str] = {}
    total_bytes = 0
    for name in _SOURCE_NAMES:
        payload = _read_source(_PACKAGE_DIRECTORY / name)
        total_bytes += len(payload)
        if total_bytes > _MAX_SOURCE_TOTAL_BYTES:
            raise RuntimeError("attribution sources exceed their aggregate byte limit")
        source_hashes[name] = hashlib.sha256(payload).hexdigest()

    manifest = {
        "python": platform.python_version(),
        "numpy": np.__version__,
        "python_implementation": platform.python_implementation(),
        "python_cache_tag": sys.implementation.cache_tag or "not_available",
        "python_optimize": sys.flags.optimize,
        "source_sha256": source_hashes,
        "source_binding": "import_time_source_snapshot_v1",
    }
    return json.dumps(
        manifest,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")


def attribution_software_manifest() -> dict[str, object]:
    """Return a detached copy of the package's import-time source manifest."""

    manifest = json.loads(_IMPORT_MANIFEST_JSON)
    if type(manifest) is not dict:
        raise RuntimeError("attribution source manifest is not a JSON object")
    return manifest


# ``experiments.attribution.__init__`` imports this module first.
_IMPORT_MANIFEST_JSON = _capture_manifest_json()
