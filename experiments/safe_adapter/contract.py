"""The ``(V, L, D, A)`` + labels contract consumed by ``pid-offline-harness``.

This mirrors the Rust ``OfflineVldaSample`` / ``OfflineVldaDataset`` schema in
``crates/pid-sim/src/offline_harness.rs`` exactly, so a dataset emitted here can be
fed straight into the offline harness:

* the top-level object has optional ``run_id`` / ``source`` / ``model`` / ``task``
  strings and a required ``samples`` array;
* each sample carries a non-empty ``sample_id``, an optional ``episode_id``,
  numeric ``v`` / ``l`` / ``d`` / ``a`` vectors of *fixed per-variable length*
  across the dataset, an optional ``labels`` object (we always set a boolean
  ``success``), and an optional flat string ``metadata`` map (we set
  ``metadata.split`` to a recognised train/held-out token plus per-variable
  provenance).

The dataclasses here are intentionally dependency-light (stdlib + the float
coercion below) so the adapter runs without the compiled extension.
"""

from __future__ import annotations

import json
import numbers
import os
import stat
import tempfile
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path

# Recognised metadata.split tokens, matching the Rust harness's accepted values.
TRAIN_SPLIT_TOKENS = ("train", "training")
HELDOUT_SPLIT_TOKENS = (
    "test",
    "validation",
    "val",
    "eval",
    "evaluation",
    "heldout",
    "holdout",
    "held_out",
    "hold_out",
)

# The contract loader is intentionally finite because verification currently
# materializes the full JSON object. Larger captures must be sharded or read by a
# future streaming verifier instead of risking an unbounded allocation here.
MAX_CONTRACT_JSON_BYTES = 512 * 1024 * 1024


def _unique_json_object(pairs: list[tuple[str, object]]) -> dict:
    value: dict[str, object] = {}
    for key, item in pairs:
        if key in value:
            raise ValueError(f"duplicate JSON object key {key!r}")
        value[key] = item
    return value


def _reject_json_constant(value: str) -> object:
    raise ValueError(f"non-finite JSON constant {value!r} is forbidden")


def _open_regular_readonly(path: Path) -> tuple[int, os.stat_result]:
    """Open without following links or blocking on special files, then bind the inode."""
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)
    try:
        fd = os.open(path, flags)
    except OSError as exc:
        raise ValueError(
            f"contract JSON must be a readable regular non-symlink file: {path}"
        ) from exc
    try:
        metadata = os.fstat(fd)
        if not stat.S_ISREG(metadata.st_mode):
            raise ValueError(f"contract JSON must be a regular file: {path}")
        return fd, metadata
    except BaseException:
        os.close(fd)
        raise


def _coerce_vector(name: str, values: Sequence[float]) -> list[float]:
    if isinstance(values, (str, bytes)) or not isinstance(values, Sequence):
        raise TypeError(f"{name} must be a sequence of real numbers")
    out: list[float] = []
    for v in values:
        if isinstance(v, bool) or not isinstance(v, numbers.Real):
            raise TypeError(f"{name} contains a non-real value: {v!r}")
        fv = float(v)
        # The harness rejects non-finite embedding values; fail loudly here.
        if fv != fv or fv in (float("inf"), float("-inf")):
            raise ValueError(f"{name} contains a non-finite value: {v!r}")
        out.append(fv)
    if not out:
        raise ValueError(f"{name} must be non-empty")
    return out


@dataclass
class VldaSample:
    """One ``(V, L, D, A)`` sample with a success label and provenance."""

    sample_id: str
    v: list[float]
    l: list[float]
    d: list[float]
    a: list[float]
    success: bool
    episode_id: str | None = None
    metadata: dict[str, str] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if not isinstance(self.sample_id, str) or not self.sample_id:
            raise ValueError("sample_id must be non-empty")
        if self.episode_id is not None and not isinstance(self.episode_id, str):
            raise TypeError("episode_id must be a string when present")
        if not isinstance(self.success, bool):
            raise TypeError("success must be boolean")
        if not isinstance(self.metadata, dict) or any(
            not isinstance(key, str) or not isinstance(value, str)
            for key, value in self.metadata.items()
        ):
            raise TypeError("metadata must map strings to strings")
        self.v = _coerce_vector("v", self.v)
        self.l = _coerce_vector("l", self.l)
        self.d = _coerce_vector("d", self.d)
        self.a = _coerce_vector("a", self.a)

    def to_json(self) -> dict:
        obj: dict = {
            "sample_id": self.sample_id,
            "v": self.v,
            "l": self.l,
            "d": self.d,
            "a": self.a,
            "labels": {"success": self.success},
        }
        if self.episode_id is not None:
            obj["episode_id"] = self.episode_id
        if self.metadata:
            # The harness metadata map is string -> string.
            obj["metadata"] = dict(self.metadata)
        return obj


@dataclass
class VldaDataset:
    """A full ``(V, L, D, A)`` dataset ready for the offline harness."""

    samples: list[VldaSample]
    run_id: str | None = None
    source: str | None = None
    model: str | None = None
    task: str | None = None

    def __post_init__(self) -> None:
        if not isinstance(self.samples, list) or any(
            not isinstance(sample, VldaSample) for sample in self.samples
        ):
            raise TypeError("samples must be a list of VldaSample objects")
        for name in ("run_id", "source", "model", "task"):
            value = getattr(self, name)
            if value is not None and not isinstance(value, str):
                raise TypeError(f"{name} must be a string when present")

    def dims(self) -> dict[str, int]:
        if not self.samples:
            raise ValueError("dataset has no samples")
        first = self.samples[0]
        return {
            "v": len(first.v),
            "l": len(first.l),
            "d": len(first.d),
            "a": len(first.a),
        }

    def validate(self) -> list[str]:
        """Return a list of contract violations (empty == valid).

        Mirrors the structural checks the Rust harness performs: non-empty,
        unique sample ids, and *fixed per-variable dimensionality* across all
        samples (the harness builds matrices and would reject ragged rows).
        """
        issues: list[str] = []
        if not self.samples:
            issues.append("dataset must contain at least one sample")
            return issues
        dims = self.dims()
        seen_ids: set[str] = set()
        for idx, s in enumerate(self.samples):
            if not s.sample_id:
                issues.append(f"sample {idx}: empty sample_id")
            if s.sample_id in seen_ids:
                issues.append(f"sample {idx}: duplicate sample_id {s.sample_id!r}")
            seen_ids.add(s.sample_id)
            for key, vec in (("v", s.v), ("l", s.l), ("d", s.d), ("a", s.a)):
                if len(vec) != dims[key]:
                    issues.append(
                        f"sample {idx} ({s.sample_id!r}): {key} has length "
                        f"{len(vec)}, expected {dims[key]}"
                    )
        return issues

    def to_json(self) -> dict:
        obj: dict = {"samples": [s.to_json() for s in self.samples]}
        for key in ("run_id", "source", "model", "task"):
            value = getattr(self, key)
            if value is not None:
                obj[key] = value
        return obj

    def write_json(self, path: str | Path, *, overwrite: bool = False) -> Path:
        path = Path(path)
        path.parent.mkdir(parents=True, exist_ok=True)
        if path.exists() and not overwrite:
            raise FileExistsError(f"refusing to overwrite existing dataset: {path}")
        issues = self.validate()
        if issues:
            raise ValueError(
                "refusing to write an invalid dataset:\n  " + "\n  ".join(issues)
            )
        fd, temp_name = tempfile.mkstemp(
            dir=path.parent,
            prefix=f".{path.name}.",
            suffix=".tmp",
        )
        temp_path = Path(temp_name)
        try:
            with os.fdopen(fd, "w", encoding="utf-8") as handle:
                json.dump(self.to_json(), handle, indent=2, allow_nan=False)
                handle.write("\n")
                handle.flush()
                output_bytes = os.fstat(handle.fileno()).st_size
                if output_bytes > MAX_CONTRACT_JSON_BYTES:
                    raise ValueError(
                        f"dataset JSON is {output_bytes} bytes; limit is "
                        f"{MAX_CONTRACT_JSON_BYTES} bytes"
                    )
                os.fsync(handle.fileno())
            if overwrite:
                os.replace(temp_path, path)
            else:
                # A same-directory hard link installs the fsynced inode only if
                # the destination is still absent; unlike rename on POSIX it
                # cannot silently overwrite a path created after the early check.
                os.link(temp_path, path)
                temp_path.unlink()
            _fsync_directory(path.parent)
        except BaseException:
            temp_path.unlink(missing_ok=True)
            raise
        return path


def split_token(seen: bool, *, train_if_seen: bool = True) -> str:
    """Map a SAFE seen/unseen flag to a recognised harness split token.

    SAFE evaluates zero-shot failure detection on *unseen* tasks, so the natural
    leakage-safe mapping is: seen tasks -> train, unseen tasks -> held-out.
    """
    is_train = seen if train_if_seen else not seen
    return TRAIN_SPLIT_TOKENS[0] if is_train else HELDOUT_SPLIT_TOKENS[0]


def load_dataset_json(
    path: str | Path,
    *,
    max_bytes: int = MAX_CONTRACT_JSON_BYTES,
) -> dict:
    """Load a bounded contract JSON file for round-trip checks.

    ``max_bytes`` is checked before the full document is materialized. This is a
    structural verifier, not a streaming reader; captures above the limit must be
    sharded rather than silently consuming unbounded memory.
    """
    path = Path(path)
    fd, metadata = _open_regular_readonly(path)
    with os.fdopen(fd, "rb") as handle:
        if metadata.st_size > max_bytes:
            raise ValueError(
                f"contract JSON {path} is {metadata.st_size} bytes; "
                f"limit is {max_bytes} bytes"
            )
        payload = handle.read(max_bytes + 1)
    if len(payload) != metadata.st_size:
        raise ValueError(f"contract JSON changed while snapshotting: {path}")
    value = json.loads(
        payload,
        object_pairs_hook=_unique_json_object,
        parse_constant=_reject_json_constant,
    )
    if not isinstance(value, dict):
        raise ValueError(f"contract JSON {path} must contain a top-level object")
    return value


def _fsync_directory(path: Path) -> None:
    """Durably install an atomically replaced file on platforms with directory fsync."""
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
    try:
        directory_fd = os.open(path, flags)
    except OSError:
        return
    try:
        os.fsync(directory_fd)
    finally:
        os.close(directory_fd)


def is_mapping(obj: object) -> bool:
    return isinstance(obj, Mapping)
