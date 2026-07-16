"""SAFE rollout intermediate representation, loader, and a synthetic generator.

The real ``vla-safe/SAFE`` rollouts are stored as one ``task{N}--ep{M}--succ{0/1}.csv``
(per-step actions + hand-crafted uncertainty metrics) plus a matching ``.pkl`` dict
with keys ``hidden_states``, ``task_suite_name``, ``task_id``, ``task_description``,
``episode_idx`` (or the upstream ``eposide_idx`` typo) and ``episode_success``
(see ``failure_prob/data/openvla.py`` upstream). Because downloaded pickle is
executable, this module's default contract is a content-addressed CSV + object-free
NPZ + strict-JSON bundle. A restricted, manifest-hashed NumPy-only importer exists
for explicit legacy use; Torch/arbitrary globals remain rejected. The synthetic
generator writes the safe canonical layout so the whole adapter is testable
without the multi-GB downloads.
"""

from __future__ import annotations

import csv
import contextlib
import hashlib
import importlib
import io
import json
import math
import os
import pickle
import re
import struct
import stat
import tempfile
import zipfile
from collections.abc import Iterable, Iterator
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np

try:
    _NUMPY_MULTIARRAY = importlib.import_module("numpy._core.multiarray")
    _NUMPY_NUMERIC = importlib.import_module("numpy._core.numeric")
except ModuleNotFoundError:  # NumPy 1.x compatibility.
    _NUMPY_MULTIARRAY = importlib.import_module("numpy.core.multiarray")
    _NUMPY_NUMERIC = importlib.import_module("numpy.core.numeric")

_FILENAME_RE = re.compile(r"task(\d+)--ep(\d+)--succ([01])\.csv$")
_SHA256_RE = re.compile(r"[0-9a-f]{64}$")

MANIFEST_NAME = "safe_bundle_manifest.json"
ARRAYS_SUFFIX = ".arrays.npz"
METADATA_SUFFIX = ".metadata.json"
_RIGHTS_STATUSES = {"verified", "restricted", "unverified", "synthetic_generated"}
_SEMANTIC_STATUSES = {"unvalidated", "validated"}
_ARRAY_NAMES = {"hidden_states", "vision_features", "language_features"}


@dataclass(frozen=True)
class IngressLimits:
    """Finite resource ceilings for one SAFE bundle load.

    The defaults are conservative software-safety limits, not measurements or
    recommended scientific operating points. Callers may lower them for an
    adversarial fixture or raise them only as an explicit reviewed decision.
    """

    max_rollouts: int = 4_096
    max_total_bytes: int = 8 * 1024 * 1024 * 1024
    max_file_bytes: int = 1024 * 1024 * 1024
    max_metadata_bytes: int = 1024 * 1024
    max_manifest_bytes: int = 4 * 1024 * 1024
    max_csv_rows: int = 100_000
    max_tensor_elements: int = 25_000_000
    max_total_tensor_elements: int = 50_000_000
    max_array_dimension: int = 1_000_000
    max_npz_members: int = 4
    max_npz_uncompressed_bytes: int = 1024 * 1024 * 1024
    max_npy_header_bytes: int = 64 * 1024
    max_legacy_pickle_bytes: int = 64 * 1024 * 1024


# OpenVLA-style 7-D action column order used by the SAFE CSVs.
ACTION_COLUMNS = (
    "action/dx",
    "action/dy",
    "action/dz",
    "action/droll",
    "action/dpitch",
    "action/dyaw",
    "action/dgripper",
)


@dataclass
class SafeRollout:
    """One normalized SAFE rollout episode."""

    task_id: int
    episode_idx: int
    task_description: str
    episode_success: bool
    actions: np.ndarray  # (T, d_a)
    hidden_states: np.ndarray  # (T, d_h) pooled, or (T, n_token, d_h) raw
    seen: bool = True
    vision_features: np.ndarray | None = None  # (T, d_v), if separately extracted
    language_features: np.ndarray | None = None  # (T, d_l), if a text encoder was run
    token_groups: dict[str, tuple[int, int]] | None = None
    extra: dict = field(default_factory=dict)

    @property
    def n_steps(self) -> int:
        return int(self.actions.shape[0])

    def episode_id(self) -> str:
        return f"task{self.task_id}--ep{self.episode_idx}"


def parse_safe_filename(name: str) -> tuple[int, int, bool]:
    """Parse ``task{N}--ep{M}--succ{0/1}.csv`` -> (task_id, episode_idx, success)."""
    match = _FILENAME_RE.fullmatch(name)
    if not match:
        raise ValueError(f"unrecognised SAFE rollout filename: {name!r}")
    return int(match.group(1)), int(match.group(2)), bool(int(match.group(3)))


def _canonical_json_bytes(value: object) -> bytes:
    return json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _open_regular_readonly(
    path: Path, *, description: str
) -> tuple[int, os.stat_result]:
    """Open without following links or blocking on special files, then bind the inode."""
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)
    try:
        fd = os.open(path, flags)
    except OSError as exc:
        raise ValueError(
            f"{description} must be a readable regular non-symlink file: {path}"
        ) from exc
    try:
        metadata = os.fstat(fd)
        if not stat.S_ISREG(metadata.st_mode):
            raise ValueError(f"{description} must be a regular file: {path}")
        return fd, metadata
    except BaseException:
        os.close(fd)
        raise


def _hash_file(path: Path, limits: IngressLimits) -> tuple[int, str]:
    fd, metadata = _open_regular_readonly(path, description="bundle payload")
    size = metadata.st_size
    if size > limits.max_file_bytes:
        os.close(fd)
        raise ValueError(
            f"bundle payload {path} is {size} bytes; per-file limit is "
            f"{limits.max_file_bytes}"
        )
    digest = hashlib.sha256()
    observed = 0
    with os.fdopen(fd, "rb") as handle:
        while chunk := handle.read(1024 * 1024):
            observed += len(chunk)
            if observed > limits.max_file_bytes:
                raise ValueError(
                    f"bundle payload {path} exceeded {limits.max_file_bytes} bytes while reading"
                )
            digest.update(chunk)
    if observed != size:
        raise ValueError(f"bundle payload changed while hashing: {path}")
    return size, digest.hexdigest()


def _unique_json_object(pairs: list[tuple[str, object]]) -> dict:
    value: dict[str, object] = {}
    for key, item in pairs:
        if key in value:
            raise ValueError(f"duplicate JSON object key {key!r}")
        value[key] = item
    return value


def _reject_json_constant(value: str) -> object:
    raise ValueError(f"non-finite JSON constant {value!r} is forbidden")


def _read_json_object_with_hash(path: Path, *, max_bytes: int) -> tuple[dict, str]:
    fd, metadata = _open_regular_readonly(path, description="JSON input")
    with os.fdopen(fd, "rb") as handle:
        if metadata.st_size > max_bytes:
            raise ValueError(
                f"JSON input {path} is {metadata.st_size} bytes; limit is {max_bytes}"
            )
        payload = handle.read(max_bytes + 1)
    if len(payload) != metadata.st_size:
        raise ValueError(f"JSON input changed while snapshotting: {path}")
    value = json.loads(
        payload,
        object_pairs_hook=_unique_json_object,
        parse_constant=_reject_json_constant,
    )
    if not isinstance(value, dict):
        raise ValueError(f"JSON input {path} must contain a top-level object")
    return value, _sha256_bytes(payload)


def _read_json_object(path: Path, *, max_bytes: int) -> dict:
    return _read_json_object_with_hash(path, max_bytes=max_bytes)[0]


def _read_json_snapshot(
    handle: io.BufferedIOBase,
    path: Path,
    *,
    max_bytes: int,
) -> dict:
    handle.seek(0, os.SEEK_END)
    size = handle.tell()
    if size > max_bytes:
        raise ValueError(f"JSON input {path} is {size} bytes; limit is {max_bytes}")
    handle.seek(0)
    value = json.load(
        io.TextIOWrapper(handle, encoding="utf-8"),
        object_pairs_hook=_unique_json_object,
        parse_constant=_reject_json_constant,
    )
    if not isinstance(value, dict):
        raise ValueError(f"JSON input {path} must contain a top-level object")
    return value


@contextlib.contextmanager
def _verified_snapshot(
    path: Path,
    entry: dict,
    limits: IngressLimits,
) -> Iterator[io.BufferedRandom]:
    """Copy one verified inode into an owned snapshot, then parse only that snapshot."""
    fd, metadata = _open_regular_readonly(path, description="bundle payload")
    snapshot = tempfile.SpooledTemporaryFile(max_size=8 * 1024 * 1024, mode="w+b")
    try:
        with os.fdopen(fd, "rb") as source:
            if metadata.st_size != entry["size_bytes"]:
                raise ValueError(f"bundle payload size changed before snapshot: {path}")
            digest = hashlib.sha256()
            observed = 0
            while chunk := source.read(1024 * 1024):
                observed += len(chunk)
                if observed > limits.max_file_bytes:
                    raise ValueError(f"bundle payload exceeded the file limit: {path}")
                digest.update(chunk)
                snapshot.write(chunk)
        if observed != entry["size_bytes"] or digest.hexdigest() != entry["sha256"]:
            raise ValueError(
                f"bundle payload changed after manifest verification: {path}"
            )
        snapshot.seek(0)
        yield snapshot
    finally:
        snapshot.close()


def _fsync_directory(path: Path) -> None:
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
    try:
        directory_fd = os.open(path, flags)
    except OSError:
        return
    try:
        os.fsync(directory_fd)
    finally:
        os.close(directory_fd)


def _atomic_write_json(path: Path, value: object, *, overwrite: bool) -> None:
    if path.exists() and not overwrite:
        raise FileExistsError(f"refusing to overwrite existing manifest: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temp_name = tempfile.mkstemp(
        dir=path.parent,
        prefix=f".{path.name}.",
        suffix=".tmp",
    )
    temp_path = Path(temp_name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            json.dump(value, handle, sort_keys=True, indent=2, allow_nan=False)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        if overwrite:
            os.replace(temp_path, path)
        else:
            # Install without replacing a destination that may have appeared after
            # the early convenience check.  A hard link gives us rename-no-replace
            # semantics on the filesystems supported by this adapter.
            os.link(temp_path, path)
            temp_path.unlink()
        _fsync_directory(path.parent)
    except BaseException:
        temp_path.unlink(missing_ok=True)
        raise


def _safe_relative_name(value: object) -> str:
    if not isinstance(value, str) or not value or len(value) > 255:
        raise ValueError(
            f"manifest file path must be a non-empty bounded string: {value!r}"
        )
    path = Path(value)
    if path.is_absolute() or path.name != value or "/" in value or "\\" in value:
        raise ValueError(
            f"manifest payloads must be root-level relative files: {value!r}"
        )
    return value


def _split_receipt(
    task_ids: Iterable[int],
    seen_task_ids: Iterable[int],
    *,
    origin: str,
    frozen_before_outcomes: bool,
    contamination_review: str,
) -> tuple[dict, str]:
    tasks = sorted(set(task_ids))
    seen = sorted(set(seen_task_ids))
    if any(
        isinstance(value, bool) or not isinstance(value, int) or value < 0
        for value in [*tasks, *seen]
    ):
        raise ValueError("task ids must be distinct non-negative integers")
    if not set(seen).issubset(tasks):
        raise ValueError("seen task ids must be a subset of the bundle task universe")
    if not isinstance(origin, str) or not origin.strip() or len(origin) > 1024:
        raise ValueError("split origin must be a non-empty bounded string")
    if not isinstance(frozen_before_outcomes, bool):
        raise ValueError("frozen_before_outcomes must be boolean")
    if (
        not isinstance(contamination_review, str)
        or not contamination_review.strip()
        or len(contamination_review) > 4096
    ):
        raise ValueError("contamination review must be a non-empty bounded string")
    receipt = {
        "assignment_unit": "task_id",
        "task_ids": tasks,
        "seen_task_ids": seen,
        "seen_role": "train",
        "heldout_role": "test",
        "origin": origin.strip(),
        "frozen_before_outcomes": frozen_before_outcomes,
        "contamination_review": contamination_review.strip(),
    }
    return receipt, _sha256_bytes(_canonical_json_bytes(receipt))


def _bundle_payload_names(directory: Path, limits: IngressLimits) -> set[str]:
    """Return every root-level file except the manifest itself.

    Canonical bundles are deliberately isolated directories. Ignoring a README,
    video, second manifest, or executable would make the claimed exact file
    coverage false, so every such file is rejected unless the schema is extended
    to type and hash it explicitly.
    """
    names: set[str] = set()
    max_payload_entries = limits.max_rollouts * 3
    # `Path.iterdir()` may delegate to `listdir()` and materialize every name.
    # `scandir()` keeps enumeration streaming so the bound also protects the
    # directory walk itself.
    with os.scandir(directory) as entries:
        for entry in entries:
            if entry.name == MANIFEST_NAME:
                continue
            if len(names) >= max_payload_entries:
                raise ValueError(
                    "bundle root contains more payload entries than the finite "
                    f"{max_payload_entries}-file limit"
                )
            if not entry.is_file(follow_symlinks=False) and not entry.is_symlink():
                raise ValueError(
                    f"bundle contains an unsupported non-file entry: {entry.name!r}"
                )
            names.add(entry.name)
    return names


def _scan_bundle_payloads(
    directory: Path, limits: IngressLimits
) -> list[tuple[str, str]]:
    payloads: list[tuple[str, str]] = []
    expected: set[str] = set()
    formats: set[str] = set()
    episode_ids: set[tuple[int, int]] = set()
    payload_names = _bundle_payload_names(directory, limits)
    csv_paths = [
        directory / name for name in sorted(payload_names) if name.endswith(".csv")
    ]
    if not csv_paths:
        raise ValueError(f"no SAFE rollouts found under {directory}")
    for csv_path in csv_paths:
        task_id, episode_idx, _ = parse_safe_filename(csv_path.name)
        episode_id = (task_id, episode_idx)
        if episode_id in episode_ids:
            raise ValueError(
                f"duplicate SAFE episode identity task={task_id} episode={episode_idx}"
            )
        episode_ids.add(episode_id)
        stem = csv_path.name.removesuffix(".csv")
        arrays = directory / f"{stem}{ARRAYS_SUFFIX}"
        metadata = directory / f"{stem}{METADATA_SUFFIX}"
        legacy = directory / f"{stem}.pkl"
        canonical_present = arrays.exists() or metadata.exists()
        if (
            canonical_present
            and arrays.is_file()
            and metadata.is_file()
            and not legacy.exists()
        ):
            episode = [
                (csv_path.name, "action_csv"),
                (arrays.name, "arrays_npz"),
                (metadata.name, "metadata_json"),
            ]
            formats.add("canonical_npz_json_v1")
        elif legacy.is_file() and not canonical_present:
            episode = [(csv_path.name, "action_csv"), (legacy.name, "legacy_pickle")]
            formats.add("legacy_numpy_pickle_v1")
        else:
            raise ValueError(
                f"{stem}: require exactly CSV+{ARRAYS_SUFFIX}+{METADATA_SUFFIX} "
                "or CSV+.pkl, never an incomplete or mixed episode"
            )
        payloads.extend(episode)
        expected.update(name for name, _ in episode)
    if len(formats) != 1:
        raise ValueError(
            "a SAFE bundle may not mix canonical and legacy episode formats"
        )
    extras = sorted(payload_names - expected)
    if extras:
        raise ValueError(f"bundle contains unpaired or unlisted payloads: {extras}")
    return payloads


def write_safe_bundle_manifest(
    directory: str | Path,
    *,
    source_name: str,
    source_revision: str,
    rights_status: str,
    rights_reference: str,
    seen_task_ids: Iterable[int],
    overwrite: bool = False,
    split_origin: str = "operator_declared",
    split_frozen_before_outcomes: bool = False,
    contamination_review: str = "not_assessed",
    model_id: str = "unresolved",
    checkpoint_revision: str = "unresolved",
    hook_id: str = "unresolved",
    tensor_contract_sha256: str | None = None,
    semantic_validation_status: str = "unvalidated",
) -> Path:
    """Hash and atomically bind a prepared SAFE bundle without deserializing it."""
    directory = Path(directory)
    if not directory.is_dir() or directory.is_symlink():
        raise ValueError(f"bundle directory must be a regular directory: {directory}")
    if (
        not source_name.strip()
        or not source_revision.strip()
        or len(source_name) > 1024
        or len(source_revision) > 1024
    ):
        raise ValueError("source name and immutable revision must be non-empty")
    if (
        rights_status not in _RIGHTS_STATUSES
        or not rights_reference.strip()
        or len(rights_reference) > 4096
    ):
        raise ValueError(
            f"rights status must be one of {sorted(_RIGHTS_STATUSES)} with a non-empty reference"
        )
    for field_name, value in (
        ("model_id", model_id),
        ("checkpoint_revision", checkpoint_revision),
        ("hook_id", hook_id),
    ):
        if not isinstance(value, str) or not value.strip() or len(value) > 1024:
            raise ValueError(f"{field_name} must be a non-empty bounded string")
    if tensor_contract_sha256 is not None and (
        not isinstance(tensor_contract_sha256, str)
        or not _SHA256_RE.fullmatch(tensor_contract_sha256)
    ):
        raise ValueError("tensor_contract_sha256 must be null or lowercase SHA-256")
    if semantic_validation_status not in _SEMANTIC_STATUSES:
        raise ValueError(
            f"semantic validation status must be one of {sorted(_SEMANTIC_STATUSES)}"
        )
    limits = IngressLimits()
    payloads = _scan_bundle_payloads(directory, limits)
    rollout_count = sum(kind == "action_csv" for _, kind in payloads)
    if rollout_count > limits.max_rollouts:
        raise ValueError(
            f"bundle has {rollout_count} rollouts; limit is {limits.max_rollouts}"
        )
    task_ids = {
        parse_safe_filename(name)[0] for name, kind in payloads if kind == "action_csv"
    }
    split_receipt, split_sha256 = _split_receipt(
        task_ids,
        seen_task_ids,
        origin=split_origin,
        frozen_before_outcomes=split_frozen_before_outcomes,
        contamination_review=contamination_review,
    )
    files = []
    total_bytes = 0
    for name, kind in payloads:
        size, sha256 = _hash_file(directory / name, limits)
        total_bytes += size
        if total_bytes > limits.max_total_bytes:
            raise ValueError(
                f"bundle payloads total {total_bytes} bytes; limit is {limits.max_total_bytes}"
            )
        files.append({"path": name, "kind": kind, "size_bytes": size, "sha256": sha256})
    manifest = {
        "schema_version": 1,
        "source": {"name": source_name.strip(), "revision": source_revision.strip()},
        "capture": {
            "model_id": model_id.strip(),
            "checkpoint_revision": checkpoint_revision.strip(),
            "hook_id": hook_id.strip(),
            "tensor_contract_sha256": tensor_contract_sha256,
            "semantic_validation_status": semantic_validation_status,
        },
        "rights": {"status": rights_status, "reference": rights_reference.strip()},
        "split": {**split_receipt, "receipt_sha256": split_sha256},
        "files": sorted(files, key=lambda entry: entry["path"]),
    }
    manifest_bytes = (
        len(
            json.dumps(
                manifest,
                sort_keys=True,
                indent=2,
                allow_nan=False,
            ).encode("utf-8")
        )
        + 1
    )
    if manifest_bytes > limits.max_manifest_bytes:
        raise ValueError(
            f"manifest would be {manifest_bytes} bytes; limit is "
            f"{limits.max_manifest_bytes}"
        )
    manifest_path = directory / MANIFEST_NAME
    _atomic_write_json(manifest_path, manifest, overwrite=overwrite)
    return manifest_path


def _read_action_csv(
    handle: io.BufferedIOBase,
    path: Path,
    limits: IngressLimits,
) -> np.ndarray:
    """Read the 7 action columns from a SAFE rollout CSV into ``(T, d_a)``."""
    handle.seek(0)
    text = io.TextIOWrapper(handle, encoding="utf-8", newline="")
    try:
        reader = csv.DictReader(text)
        if reader.fieldnames is None:
            raise ValueError(f"{path}: empty CSV")
        missing = [c for c in ACTION_COLUMNS if c not in reader.fieldnames]
        if missing:
            raise ValueError(f"{path}: missing action columns {missing}")
        if len(set(reader.fieldnames)) != len(reader.fieldnames):
            raise ValueError(f"{path}: duplicate CSV column names are forbidden")
        rows: list[list[float]] = []
        for row_index, row in enumerate(reader):
            if row_index >= limits.max_csv_rows:
                raise ValueError(f"{path}: exceeds CSV row limit {limits.max_csv_rows}")
            values = [float(row[column]) for column in ACTION_COLUMNS]
            if not all(math.isfinite(value) for value in values):
                raise ValueError(
                    f"{path}: action row {row_index} contains non-finite values"
                )
            rows.append(values)
    finally:
        text.detach()
    if not rows:
        raise ValueError(f"{path}: no rows")
    return np.asarray(rows, dtype=np.float64)


def _validated_array(name: str, value: object, limits: IngressLimits) -> np.ndarray:
    array = np.asarray(value)
    if (
        array.dtype.hasobject
        or array.dtype.fields is not None
        or array.dtype.kind not in "iuf"
    ):
        raise ValueError(
            f"{name} must have a plain real numeric dtype, got {array.dtype}"
        )
    if array.ndim not in (2, 3):
        raise ValueError(f"{name} must be 2-D or 3-D, got shape {array.shape}")
    if any(dim <= 0 or dim > limits.max_array_dimension for dim in array.shape):
        raise ValueError(f"{name} has an invalid or oversized dimension: {array.shape}")
    if array.size > limits.max_tensor_elements:
        raise ValueError(
            f"{name} has {array.size} elements; limit is {limits.max_tensor_elements}"
        )
    converted = np.asarray(array, dtype=np.float64)
    if not np.isfinite(converted).all():
        raise ValueError(f"{name} contains non-finite values")
    return converted


def _hidden_states_to_array(value: object, limits: IngressLimits) -> np.ndarray:
    """Coerce restricted NumPy ``hidden_states`` into one finite array."""
    if isinstance(value, list):
        if not value or len(value) > limits.max_array_dimension:
            raise ValueError("hidden_states list is empty or exceeds the step limit")
        elements: list[np.ndarray] = []
        expected_shape: tuple[int, ...] | None = None
        total = 0
        for item in value:
            array = np.asarray(item)
            if (
                array.dtype.hasobject
                or array.dtype.fields is not None
                or array.dtype.kind not in "iuf"
                or array.ndim not in (1, 2)
                or any(
                    dim <= 0 or dim > limits.max_array_dimension for dim in array.shape
                )
            ):
                raise ValueError("hidden_states list contains an invalid numeric array")
            if expected_shape is None:
                expected_shape = array.shape
            elif array.shape != expected_shape:
                raise ValueError("hidden_states list is ragged")
            total += array.size
            if total > limits.max_tensor_elements:
                raise ValueError("hidden_states list exceeds the tensor element limit")
            elements.append(array)
        value = np.stack(elements, axis=0)
    return _validated_array("hidden_states", value, limits)


def _load_and_verify_manifest(
    directory: Path,
    limits: IngressLimits,
) -> tuple[dict, dict[str, dict], str]:
    manifest_path = directory / MANIFEST_NAME
    manifest, manifest_hash = _read_json_object_with_hash(
        manifest_path, max_bytes=limits.max_manifest_bytes
    )
    if set(manifest) != {
        "schema_version",
        "source",
        "capture",
        "rights",
        "split",
        "files",
    }:
        raise ValueError(f"{manifest_path}: unknown or missing top-level fields")
    if (
        isinstance(manifest["schema_version"], bool)
        or not isinstance(manifest["schema_version"], int)
        or manifest["schema_version"] != 1
    ):
        raise ValueError(f"{manifest_path}: unsupported schema_version")
    source = manifest["source"]
    capture = manifest["capture"]
    rights = manifest["rights"]
    split = manifest["split"]
    if not isinstance(source, dict) or set(source) != {"name", "revision"}:
        raise ValueError(f"{manifest_path}: invalid source receipt")
    if not all(
        isinstance(source.get(key), str)
        and source[key].strip()
        and len(source[key]) <= 1024
        for key in source
    ):
        raise ValueError(
            f"{manifest_path}: source name/revision must be non-empty strings"
        )
    if not isinstance(capture, dict) or set(capture) != {
        "model_id",
        "checkpoint_revision",
        "hook_id",
        "tensor_contract_sha256",
        "semantic_validation_status",
    }:
        raise ValueError(f"{manifest_path}: invalid capture receipt")
    for field_name in ("model_id", "checkpoint_revision", "hook_id"):
        value = capture.get(field_name)
        if not isinstance(value, str) or not value.strip() or len(value) > 1024:
            raise ValueError(f"{manifest_path}: invalid capture {field_name}")
    tensor_hash = capture.get("tensor_contract_sha256")
    if tensor_hash is not None and (
        not isinstance(tensor_hash, str) or not _SHA256_RE.fullmatch(tensor_hash)
    ):
        raise ValueError(f"{manifest_path}: invalid tensor-contract SHA-256")
    if capture.get("semantic_validation_status") not in _SEMANTIC_STATUSES:
        raise ValueError(f"{manifest_path}: invalid semantic validation status")
    if not isinstance(rights, dict) or set(rights) != {"status", "reference"}:
        raise ValueError(f"{manifest_path}: invalid rights receipt")
    if (
        rights.get("status") not in _RIGHTS_STATUSES
        or not isinstance(rights.get("reference"), str)
        or not rights["reference"].strip()
        or len(rights["reference"]) > 4096
    ):
        raise ValueError(f"{manifest_path}: invalid rights status/reference")
    if not isinstance(split, dict) or set(split) != {
        "assignment_unit",
        "task_ids",
        "seen_task_ids",
        "seen_role",
        "heldout_role",
        "origin",
        "frozen_before_outcomes",
        "contamination_review",
        "receipt_sha256",
    }:
        raise ValueError(f"{manifest_path}: invalid split receipt")
    if (
        split.get("assignment_unit") != "task_id"
        or not isinstance(split.get("seen_task_ids"), list)
        or not isinstance(split.get("task_ids"), list)
    ):
        raise ValueError(f"{manifest_path}: split must be task_id-based")
    split_receipt, expected_split_hash = _split_receipt(
        split["task_ids"],
        split["seen_task_ids"],
        origin=split.get("origin"),
        frozen_before_outcomes=split.get("frozen_before_outcomes"),
        contamination_review=split.get("contamination_review"),
    )
    if (
        split.get("seen_role") != "train"
        or split.get("heldout_role") != "test"
        or any(split.get(key) != value for key, value in split_receipt.items())
        or split.get("receipt_sha256") != expected_split_hash
    ):
        raise ValueError(f"{manifest_path}: noncanonical or mismatched split receipt")
    seen = split_receipt["seen_task_ids"]
    if (
        len(seen) > limits.max_rollouts
        or len(split_receipt["task_ids"]) > limits.max_rollouts
    ):
        raise ValueError(f"{manifest_path}: split receipt exceeds the rollout limit")
    files = manifest["files"]
    if not isinstance(files, list) or not files:
        raise ValueError(f"{manifest_path}: files must be a non-empty array")
    by_name: dict[str, dict] = {}
    total_bytes = 0
    allowed_kinds = {"action_csv", "arrays_npz", "metadata_json", "legacy_pickle"}
    expected_suffixes = {
        "action_csv": ".csv",
        "arrays_npz": ARRAYS_SUFFIX,
        "metadata_json": METADATA_SUFFIX,
        "legacy_pickle": ".pkl",
    }
    for index, entry in enumerate(files):
        if not isinstance(entry, dict) or set(entry) != {
            "path",
            "kind",
            "size_bytes",
            "sha256",
        }:
            raise ValueError(f"{manifest_path}: invalid file entry {index}")
        name = _safe_relative_name(entry["path"])
        if name in by_name:
            raise ValueError(f"{manifest_path}: duplicate file entry {name!r}")
        if entry["kind"] not in allowed_kinds:
            raise ValueError(f"{manifest_path}: invalid file kind for {name!r}")
        if not name.endswith(expected_suffixes[entry["kind"]]):
            raise ValueError(
                f"{manifest_path}: file kind/extension mismatch for {name!r}"
            )
        if entry["kind"] == "action_csv":
            parse_safe_filename(name)
        size = entry["size_bytes"]
        if isinstance(size, bool) or not isinstance(size, int) or size < 0:
            raise ValueError(f"{manifest_path}: invalid file size for {name!r}")
        if size > limits.max_file_bytes:
            raise ValueError(f"{manifest_path}: {name!r} exceeds the per-file limit")
        sha256 = entry["sha256"]
        if not isinstance(sha256, str) or not _SHA256_RE.fullmatch(sha256):
            raise ValueError(f"{manifest_path}: invalid SHA-256 for {name!r}")
        total_bytes += size
        if total_bytes > limits.max_total_bytes:
            raise ValueError(
                f"{manifest_path}: declared bytes exceed {limits.max_total_bytes}"
            )
        actual_size, actual_hash = _hash_file(directory / name, limits)
        if (actual_size, actual_hash) != (size, sha256):
            raise ValueError(f"{manifest_path}: content receipt mismatch for {name!r}")
        by_name[name] = entry
    actual_payloads = _bundle_payload_names(directory, limits)
    if set(by_name) != actual_payloads:
        missing = sorted(set(by_name) - actual_payloads)
        extra = sorted(actual_payloads - set(by_name))
        raise ValueError(
            f"{manifest_path}: payload coverage mismatch missing={missing} extra={extra}"
        )
    csv_names = [
        name for name, entry in by_name.items() if entry["kind"] == "action_csv"
    ]
    if len(csv_names) > limits.max_rollouts:
        raise ValueError(
            f"{manifest_path}: rollout count exceeds {limits.max_rollouts}"
        )
    expected_payloads: set[str] = set()
    formats: set[str] = set()
    episode_ids: set[tuple[int, int]] = set()
    for csv_name in csv_names:
        task_id, episode_idx, _ = parse_safe_filename(csv_name)
        episode_id = (task_id, episode_idx)
        if episode_id in episode_ids:
            raise ValueError(
                f"{manifest_path}: duplicate episode identity task={task_id} "
                f"episode={episode_idx}"
            )
        episode_ids.add(episode_id)
        stem = csv_name.removesuffix(".csv")
        arrays_name = f"{stem}{ARRAYS_SUFFIX}"
        metadata_name = f"{stem}{METADATA_SUFFIX}"
        legacy_name = f"{stem}.pkl"
        canonical = (
            by_name.get(arrays_name, {}).get("kind") == "arrays_npz"
            and by_name.get(metadata_name, {}).get("kind") == "metadata_json"
            and legacy_name not in by_name
        )
        legacy = (
            by_name.get(legacy_name, {}).get("kind") == "legacy_pickle"
            and arrays_name not in by_name
            and metadata_name not in by_name
        )
        if canonical:
            expected_payloads.update({csv_name, arrays_name, metadata_name})
            formats.add("canonical_npz_json_v1")
        elif legacy:
            expected_payloads.update({csv_name, legacy_name})
            formats.add("legacy_numpy_pickle_v1")
        else:
            raise ValueError(
                f"{manifest_path}: incomplete or mixed episode topology for {stem}"
            )
    if len(formats) != 1 or expected_payloads != set(by_name):
        raise ValueError(
            f"{manifest_path}: bundle topology contains orphan or mixed payloads"
        )
    task_ids = {parse_safe_filename(name)[0] for name in csv_names}
    if split_receipt["task_ids"] != sorted(task_ids):
        raise ValueError(f"{manifest_path}: split task universe differs from payloads")
    return manifest, by_name, manifest_hash


def _inspect_npy_member(
    archive: zipfile.ZipFile,
    info: zipfile.ZipInfo,
    limits: IngressLimits,
) -> None:
    with archive.open(info, "r") as member:
        version = np.lib.format.read_magic(member)
        if version == (1, 0):
            shape, _, dtype = np.lib.format.read_array_header_1_0(
                member, max_header_size=limits.max_npy_header_bytes
            )
        elif version in {(2, 0), (3, 0)}:
            shape, _, dtype = np.lib.format.read_array_header_2_0(
                member, max_header_size=limits.max_npy_header_bytes
            )
        else:
            raise ValueError(f"unsupported NPY version {version} in {info.filename}")
        data_offset = member.tell()
    if dtype.hasobject or dtype.fields is not None or dtype.kind not in "iuf":
        raise ValueError(
            f"{info.filename}: object/structured/non-real dtype {dtype} is forbidden"
        )
    if len(shape) not in (2, 3) or any(
        dim <= 0 or dim > limits.max_array_dimension for dim in shape
    ):
        raise ValueError(f"{info.filename}: invalid or oversized shape {shape}")
    elements = math.prod(shape)
    if elements > limits.max_tensor_elements:
        raise ValueError(
            f"{info.filename}: {elements} elements exceed {limits.max_tensor_elements}"
        )
    expected_data_bytes = elements * dtype.itemsize
    if (
        data_offset > info.file_size
        or expected_data_bytes != info.file_size - data_offset
    ):
        raise ValueError(
            f"{info.filename}: NPY member size does not exactly match its declared array"
        )


def _preflight_zip_directory(
    handle: io.BufferedIOBase,
    path: Path,
    limits: IngressLimits,
) -> None:
    """Reject multi-disk/ZIP64 or excessive-member archives before ZipFile allocates."""
    handle.seek(0, os.SEEK_END)
    size = handle.tell()
    tail_size = min(size, 65_557)
    handle.seek(size - tail_size)
    tail = handle.read(tail_size)
    signature = b"PK\x05\x06"
    offset = tail.rfind(signature)
    while offset >= 0:
        if offset + 22 <= len(tail):
            fields = struct.unpack_from("<4s4H2LH", tail, offset)
            (
                _,
                disk,
                start_disk,
                entries_disk,
                entries_total,
                cd_size,
                cd_offset,
                comment,
            ) = fields
            if offset + 22 + comment == len(tail):
                if disk != 0 or start_disk != 0 or entries_disk != entries_total:
                    raise ValueError(f"{path}: multi-disk ZIP archives are forbidden")
                if entries_total == 0xFFFF or cd_size == 0xFFFFFFFF:
                    raise ValueError(
                        f"{path}: ZIP64 archives are outside the ingress contract"
                    )
                if not 1 <= entries_total <= limits.max_npz_members:
                    raise ValueError(
                        f"{path}: NPZ member count must be 1..{limits.max_npz_members}"
                    )
                if cd_offset + cd_size > size:
                    raise ValueError(f"{path}: invalid ZIP central-directory bounds")
                return
        offset = tail.rfind(signature, 0, offset)
    raise ValueError(f"{path}: missing a valid ZIP end-of-central-directory record")


def _load_npz_arrays(
    handle: io.BufferedIOBase,
    path: Path,
    limits: IngressLimits,
) -> dict[str, np.ndarray]:
    _preflight_zip_directory(handle, path, limits)
    try:
        handle.seek(0)
        with zipfile.ZipFile(handle) as archive:
            infos = archive.infolist()
            if not infos or len(infos) > limits.max_npz_members:
                raise ValueError(
                    f"{path}: NPZ member count must be 1..{limits.max_npz_members}"
                )
            total_uncompressed = 0
            names: set[str] = set()
            for info in infos:
                if info.is_dir() or "/" in info.filename or "\\" in info.filename:
                    raise ValueError(
                        f"{path}: NPZ members must be root-level NPY files"
                    )
                if info.flag_bits & 0x1:
                    raise ValueError(f"{path}: encrypted NPZ members are forbidden")
                if info.compress_type not in {zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED}:
                    raise ValueError(f"{path}: unsupported NPZ compression method")
                if not info.filename.endswith(".npy"):
                    raise ValueError(f"{path}: unexpected NPZ member {info.filename!r}")
                name = info.filename.removesuffix(".npy")
                if name not in _ARRAY_NAMES or name in names:
                    raise ValueError(f"{path}: unexpected or duplicate array {name!r}")
                names.add(name)
                total_uncompressed += info.file_size
                if total_uncompressed > limits.max_npz_uncompressed_bytes:
                    raise ValueError(
                        f"{path}: uncompressed payload exceeds "
                        f"{limits.max_npz_uncompressed_bytes} bytes"
                    )
                _inspect_npy_member(archive, info, limits)
            if "hidden_states" not in names:
                raise ValueError(f"{path}: missing required hidden_states array")
    except zipfile.BadZipFile as exc:
        raise ValueError(f"{path}: invalid NPZ archive") from exc
    try:
        handle.seek(0)
        with np.load(
            handle,
            allow_pickle=False,
            max_header_size=limits.max_npy_header_bytes,
        ) as loaded:
            return {
                name: _validated_array(name, loaded[name], limits)
                for name in loaded.files
            }
    except (OSError, ValueError, EOFError) as exc:
        raise ValueError(
            f"{path}: could not load bounded numeric arrays: {exc}"
        ) from exc


class _RestrictedNumpyUnpickler(pickle.Unpickler):
    # Deliberately omit ndarray `_reconstruct`: it accepts an attacker-declared
    # shape and may allocate before post-load limits can run. `_frombuffer`
    # requires the declared shape to fit bytes already present in the bounded,
    # hash-verified pickle. Older NumPy pickle encodings may therefore abstain;
    # the canonical NPZ/JSON path is the compatibility target.
    _ALLOWED_GLOBALS = {
        ("numpy", "dtype"): np.dtype,
        ("numpy.core.multiarray", "scalar"): _NUMPY_MULTIARRAY.scalar,
        ("numpy._core.multiarray", "scalar"): _NUMPY_MULTIARRAY.scalar,
        ("numpy.core.numeric", "_frombuffer"): _NUMPY_NUMERIC._frombuffer,
        ("numpy._core.numeric", "_frombuffer"): _NUMPY_NUMERIC._frombuffer,
    }

    def find_class(self, module: str, name: str) -> object:
        value = self._ALLOWED_GLOBALS.get((module, name))
        if value is None:
            raise pickle.UnpicklingError(f"forbidden pickle global {module}.{name}")
        return value

    def persistent_load(self, pid: object) -> object:
        raise pickle.UnpicklingError(
            f"persistent pickle references are forbidden: {pid!r}"
        )


def _load_legacy_pickle(handle: io.BufferedIOBase, path: Path) -> dict:
    try:
        handle.seek(0)
        value = _RestrictedNumpyUnpickler(handle).load()
        if handle.read(1):
            raise pickle.UnpicklingError("trailing bytes after the first pickle object")
    except (pickle.UnpicklingError, EOFError, ValueError, TypeError) as exc:
        raise ValueError(f"restricted legacy pickle rejected {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise ValueError(f"legacy pickle {path} must contain a dictionary")
    return value


def _normalize_token_groups(
    value: object, *, n_tokens: int | None
) -> dict[str, tuple[int, int]] | None:
    if value is None:
        return None
    if not isinstance(value, dict) or len(value) > 128:
        raise ValueError("token_groups must be a bounded object")
    groups: dict[str, tuple[int, int]] = {}
    for name, bounds in value.items():
        if not isinstance(name, str) or not name or len(name) > 128:
            raise ValueError("token group names must be non-empty bounded strings")
        if not isinstance(bounds, (list, tuple)) or len(bounds) != 2:
            raise ValueError(f"token group {name!r} must contain [start, end]")
        start, end = bounds
        if any(isinstance(v, bool) or not isinstance(v, int) for v in (start, end)):
            raise ValueError(f"token group {name!r} bounds must be integers")
        if not 0 <= start < end or (n_tokens is not None and end > n_tokens):
            raise ValueError(f"token group {name!r} bounds are invalid: {bounds}")
        groups[name] = (start, end)
    return groups


def _validate_identity_metadata(
    meta: dict,
    *,
    path: Path,
    task_id: int,
    episode_idx: int,
    success: bool,
    strict: bool,
) -> tuple[str, str, dict[str, tuple[int, int]] | None]:
    allowed = {
        "schema_version",
        "task_suite_name",
        "task_id",
        "task_description",
        "episode_idx",
        "episode_success",
        "token_groups",
    }
    if strict and set(meta) - allowed:
        raise ValueError(
            f"{path}: unknown metadata fields {sorted(set(meta) - allowed)}"
        )
    if strict and (
        isinstance(meta.get("schema_version"), bool)
        or not isinstance(meta.get("schema_version"), int)
        or meta.get("schema_version") != 1
    ):
        raise ValueError(f"{path}: canonical metadata schema_version must be 1")
    required = {
        "schema_version",
        "task_suite_name",
        "task_id",
        "task_description",
        "episode_idx",
        "episode_success",
    }
    if strict and not required.issubset(meta):
        raise ValueError(
            f"{path}: canonical metadata is missing {sorted(required - set(meta))}"
        )
    if not strict and (
        "task_id" not in meta
        or "episode_success" not in meta
        or not ({"episode_idx", "eposide_idx"} & set(meta))
    ):
        raise ValueError(
            f"{path}: legacy metadata must explicitly carry task, episode, and outcome identity"
        )
    if "episode_idx" in meta and "eposide_idx" in meta:
        if meta["episode_idx"] != meta["eposide_idx"]:
            raise ValueError(f"{path}: episode_idx conflicts with legacy eposide_idx")
    supplied_episode = meta.get("episode_idx", meta.get("eposide_idx", episode_idx))
    checks = (
        ("task_id", meta.get("task_id", task_id), task_id),
        ("episode_idx", supplied_episode, episode_idx),
        ("episode_success", meta.get("episode_success", success), success),
    )
    for name, observed, expected in checks:
        if name == "episode_success":
            valid_type = isinstance(observed, bool) or (
                isinstance(observed, int)
                and not isinstance(observed, bool)
                and observed in (0, 1)
            )
            if not valid_type or bool(observed) != expected:
                raise ValueError(
                    f"{path}: {name}={observed!r} conflicts with filename value {expected!r}"
                )
        elif (
            isinstance(observed, bool)
            or not isinstance(observed, int)
            or observed != expected
        ):
            raise ValueError(
                f"{path}: {name}={observed!r} conflicts with filename value {expected!r}"
            )
    description = meta.get("task_description", f"task {task_id}")
    if not isinstance(description, str) or not description or len(description) > 16_384:
        raise ValueError(f"{path}: task_description must be a non-empty bounded string")
    suite = meta.get("task_suite_name")
    if suite is not None and (
        not isinstance(suite, str) or not suite or len(suite) > 1024
    ):
        raise ValueError(f"{path}: task_suite_name must be a non-empty bounded string")
    return description, suite or "unreported", meta.get("token_groups")


def load_safe_rollout_dir(
    directory: str | Path,
    *,
    seen_task_ids: Iterable[int] | None = None,
    allow_legacy_pickle: bool = False,
    allow_unverified_rights: bool = False,
    allow_unfrozen_split: bool = False,
    limits: IngressLimits | None = None,
) -> list[SafeRollout]:
    """Load a finite, content-addressed canonical SAFE rollout bundle.

    Legacy pickle is rejected unless explicitly enabled, hash-bound by the
    manifest, and accepted by the NumPy-only restricted unpickler.
    """
    limits = limits or IngressLimits()
    directory = Path(directory)
    if not directory.is_dir() or directory.is_symlink():
        raise ValueError(f"bundle directory must be a regular directory: {directory}")
    manifest, entries, manifest_hash = _load_and_verify_manifest(directory, limits)
    rights = manifest["rights"]
    if rights["status"] == "unverified" and not allow_unverified_rights:
        raise ValueError(
            "bundle rights status is unverified; review rights or explicitly pass "
            "allow_unverified_rights=True (which grants no rights)"
        )
    split_receipt = manifest["split"]
    split_unreviewed = not split_receipt["frozen_before_outcomes"] or split_receipt[
        "contamination_review"
    ].strip().lower() in {"not_assessed", "unassessed", "none"}
    if split_unreviewed and not allow_unfrozen_split:
        raise ValueError(
            "bundle split was not frozen before outcomes or contamination was not reviewed; "
            "explicit audit-only override required"
        )
    manifest_seen = set(manifest["split"]["seen_task_ids"])
    if seen_task_ids is not None and set(seen_task_ids) != manifest_seen:
        raise ValueError("caller seen_task_ids do not match the hashed split receipt")
    rollouts: list[SafeRollout] = []
    total_tensor_elements = 0
    for csv_name in sorted(
        name for name, entry in entries.items() if entry["kind"] == "action_csv"
    ):
        csv_path = directory / csv_name
        task_id, episode_idx, success = parse_safe_filename(csv_name)
        with _verified_snapshot(csv_path, entries[csv_name], limits) as snapshot:
            actions = _read_action_csv(snapshot, csv_path, limits)
        stem = csv_name.removesuffix(".csv")
        arrays_name = f"{stem}{ARRAYS_SUFFIX}"
        metadata_name = f"{stem}{METADATA_SUFFIX}"
        legacy_name = f"{stem}.pkl"
        if arrays_name in entries and metadata_name in entries:
            arrays_path = directory / arrays_name
            metadata_path = directory / metadata_name
            with _verified_snapshot(
                arrays_path, entries[arrays_name], limits
            ) as snapshot:
                arrays = _load_npz_arrays(snapshot, arrays_path, limits)
            with _verified_snapshot(
                metadata_path, entries[metadata_name], limits
            ) as snapshot:
                meta = _read_json_snapshot(
                    snapshot,
                    metadata_path,
                    max_bytes=limits.max_metadata_bytes,
                )
            ingest_format = "canonical_npz_json_v1"
            raw_metadata = entries[metadata_name]
            strict_meta = True
        elif legacy_name in entries:
            if not allow_legacy_pickle:
                raise ValueError(
                    f"legacy pickle {legacy_name} is disabled by default; safely re-export "
                    "to NPZ/JSON or explicitly opt into the restricted importer"
                )
            if entries[legacy_name]["size_bytes"] > limits.max_legacy_pickle_bytes:
                raise ValueError(
                    f"legacy pickle exceeds {limits.max_legacy_pickle_bytes} bytes"
                )
            arrays = {}
            legacy_path = directory / legacy_name
            with _verified_snapshot(
                legacy_path, entries[legacy_name], limits
            ) as snapshot:
                meta = _load_legacy_pickle(snapshot, legacy_path)
            if "hidden_states" not in meta:
                raise ValueError(f"legacy pickle {legacy_name} has no hidden_states")
            arrays["hidden_states"] = _hidden_states_to_array(
                meta["hidden_states"], limits
            )
            for optional_name in ("vision_features", "language_features"):
                if meta.get(optional_name) is not None:
                    arrays[optional_name] = _validated_array(
                        optional_name, meta[optional_name], limits
                    )
            ingest_format = "legacy_numpy_pickle_v1"
            raw_metadata = None
            strict_meta = False
            arrays_name = legacy_name
        else:
            raise ValueError(f"manifest has no complete data pair for {csv_name}")
        description, task_suite_name, raw_token_groups = _validate_identity_metadata(
            meta,
            path=directory / (metadata_name if strict_meta else legacy_name),
            task_id=task_id,
            episode_idx=episode_idx,
            success=success,
            strict=strict_meta,
        )
        hidden = _validated_array("hidden_states", arrays["hidden_states"], limits)
        if hidden.shape[0] != actions.shape[0]:
            raise ValueError(
                f"{stem}: hidden/action step-count mismatch "
                f"{hidden.shape[0]} vs {actions.shape[0]}"
            )
        vision = arrays.get("vision_features")
        language = arrays.get("language_features")
        for name, optional in (
            ("vision_features", vision),
            ("language_features", language),
        ):
            if optional is not None and optional.shape[0] != actions.shape[0]:
                raise ValueError(f"{stem}: {name}/action step-count mismatch")
        token_groups = _normalize_token_groups(
            raw_token_groups,
            n_tokens=hidden.shape[1] if hidden.ndim == 3 else None,
        )
        csv_entry = entries[csv_name]
        arrays_entry = entries[arrays_name]
        extra: dict[str, object] = {
            "ingest_format": ingest_format,
            "bundle_manifest_path": MANIFEST_NAME,
            "bundle_manifest_locator_status": "external_not_archived_by_converter",
            "bundle_manifest_sha256": manifest_hash,
            "source_name": manifest["source"]["name"],
            "source_revision": manifest["source"]["revision"],
            "rights_status": rights["status"],
            "rights_reference": rights["reference"],
            "rights_reference_sha256": _sha256_bytes(
                rights["reference"].encode("utf-8")
            ),
            "seen_split_receipt_sha256": manifest["split"]["receipt_sha256"],
            "split_origin": manifest["split"]["origin"],
            "split_origin_sha256": _sha256_bytes(
                manifest["split"]["origin"].encode("utf-8")
            ),
            "split_frozen_before_outcomes": manifest["split"]["frozen_before_outcomes"],
            "contamination_review": manifest["split"]["contamination_review"],
            "contamination_review_sha256": _sha256_bytes(
                manifest["split"]["contamination_review"].encode("utf-8")
            ),
            "seen_role": manifest["split"]["seen_role"],
            "heldout_role": manifest["split"]["heldout_role"],
            "split_scientific_eligibility": (
                "blocked_unfrozen_or_unreviewed"
                if split_unreviewed
                else "structural_split_ready"
            ),
            "task_suite_name": task_suite_name,
            "instruction_sha256": _sha256_bytes(description.encode("utf-8")),
            "model_id": manifest["capture"]["model_id"],
            "checkpoint_revision": manifest["capture"]["checkpoint_revision"],
            "hook_id": manifest["capture"]["hook_id"],
            "tensor_contract_sha256": manifest["capture"]["tensor_contract_sha256"],
            "semantic_validation_status": manifest["capture"][
                "semantic_validation_status"
            ],
            "raw_csv_path": csv_name,
            "raw_csv_size_bytes": csv_entry["size_bytes"],
            "raw_csv_sha256": csv_entry["sha256"],
            "raw_arrays_path": arrays_name,
            "raw_arrays_size_bytes": arrays_entry["size_bytes"],
            "raw_arrays_sha256": arrays_entry["sha256"],
        }
        if raw_metadata is not None:
            extra.update(
                {
                    "raw_metadata_path": metadata_name,
                    "raw_metadata_size_bytes": raw_metadata["size_bytes"],
                    "raw_metadata_sha256": raw_metadata["sha256"],
                }
            )
        rollout_elements = actions.size + hidden.size
        if vision is not None:
            rollout_elements += vision.size
        if language is not None:
            rollout_elements += language.size
        if total_tensor_elements + rollout_elements > limits.max_total_tensor_elements:
            raise ValueError(
                "bundle tensors exceed the total element limit "
                f"{limits.max_total_tensor_elements}"
            )
        total_tensor_elements += rollout_elements
        rollouts.append(
            SafeRollout(
                task_id=task_id,
                episode_idx=episode_idx,
                task_description=description,
                episode_success=success,
                actions=actions,
                hidden_states=hidden,
                seen=task_id in manifest_seen,
                vision_features=vision,
                language_features=language,
                token_groups=token_groups,
                extra=extra,
            )
        )
    if not rollouts:
        raise ValueError(f"no SAFE rollouts found under {directory}")
    return rollouts


def write_synthetic_safe_dir(
    directory: str | Path,
    *,
    n_tasks: int = 4,
    episodes_per_task: int = 4,
    n_steps: int = 12,
    n_tokens: int = 6,
    d_hidden: int = 8,
    d_action: int = 7,
    seed: int = 0,
    raw_token_states: bool = True,
) -> Path:
    """Write a synthetic canonical SAFE bundle (CSV + NPZ + JSON + manifest).

    The generated data has a *learnable* structure: a latent per-episode "skill"
    variable drives both the hidden states and the success outcome, so downstream
    PID/baselines/probes see real (not random) signal. With ``raw_token_states`` the
    NPZ arrays store raw ``(T, n_token, d)`` hidden states so token slicing is testable.
    """
    integer_fields = {
        "n_tasks": n_tasks,
        "episodes_per_task": episodes_per_task,
        "n_steps": n_steps,
        "n_tokens": n_tokens,
        "d_hidden": d_hidden,
        "d_action": d_action,
        "seed": seed,
    }
    if any(
        isinstance(value, bool) or not isinstance(value, int)
        for value in integer_fields.values()
    ):
        raise TypeError("synthetic counts, dimensions, and seed must be integers")
    if min(n_tasks, episodes_per_task, n_steps, n_tokens, d_hidden) <= 0:
        raise ValueError("synthetic dimensions and counts must be positive")
    if d_action != len(ACTION_COLUMNS):
        raise ValueError(
            f"synthetic SAFE actions must have {len(ACTION_COLUMNS)} columns"
        )
    if not isinstance(raw_token_states, bool):
        raise TypeError("raw_token_states must be boolean")
    if raw_token_states and n_tokens < 3:
        raise ValueError("raw token-state fixtures require at least three tokens")
    limits = IngressLimits()
    rollout_count = n_tasks * episodes_per_task
    hidden_elements = n_steps * d_hidden * (n_tokens if raw_token_states else 1)
    total_elements = rollout_count * (hidden_elements + n_steps * d_action)
    if rollout_count > limits.max_rollouts:
        raise ValueError(f"synthetic rollout count exceeds {limits.max_rollouts}")
    if n_steps > limits.max_csv_rows or hidden_elements > limits.max_tensor_elements:
        raise ValueError("synthetic episode exceeds row/tensor ingress limits")
    if any(
        dimension > limits.max_array_dimension
        for dimension in (n_steps, n_tokens, d_hidden, d_action)
    ):
        raise ValueError("synthetic array dimension exceeds the ingress limit")
    if total_elements > limits.max_total_tensor_elements:
        raise ValueError("synthetic bundle exceeds the total tensor ingress limit")
    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=True)
    if directory.is_symlink():
        raise ValueError("synthetic output directory may not be a symlink")
    recognized = _bundle_payload_names(directory, limits)
    manifest_path = directory / MANIFEST_NAME
    if recognized or manifest_path.exists():
        raise FileExistsError(
            "synthetic generation requires a new empty directory; existing evidence is never replaced"
        )
    rng = np.random.default_rng(seed)

    for task_id in range(n_tasks):
        for episode_idx in range(episodes_per_task):
            # Latent skill in [-1, 1]; higher skill -> more likely success.
            skill = rng.uniform(-1.0, 1.0)
            success = bool(skill + 0.2 * rng.standard_normal() > 0.0)

            # Actions: smooth trajectory modulated by skill + noise.
            t = np.linspace(0.0, 1.0, n_steps)
            base = np.outer(t, np.ones(d_action)) * (0.5 + 0.5 * skill)
            actions = base + 0.05 * rng.standard_normal((n_steps, d_action))

            if raw_token_states:
                hidden = rng.standard_normal((n_steps, n_tokens, d_hidden)) * 0.3
                # Vision tokens (first third) encode skill; language tokens (middle)
                # encode task identity; state tokens (last third) mix both.
                v_end = n_tokens // 3
                l_end = 2 * n_tokens // 3
                hidden[:, :v_end, 0] += skill
                hidden[:, v_end:l_end, 1] += task_id / n_tasks
                hidden[:, l_end:, 2] += skill + task_id / n_tasks
                token_groups = {
                    "vision": [0, max(1, v_end)],
                    "language": [max(1, v_end), max(2, l_end)],
                    "state": [max(2, l_end), n_tokens],
                }
            else:
                hidden = rng.standard_normal((n_steps, d_hidden)) * 0.3
                hidden[:, 0] += skill
                token_groups = None

            stem = f"task{task_id}--ep{episode_idx}--succ{int(success)}"
            _write_action_csv(directory / f"{stem}.csv", actions)
            meta = {
                "schema_version": 1,
                "task_suite_name": "synthetic",
                "task_id": task_id,
                "task_description": f"synthetic task {task_id}: move object {task_id}",
                "episode_idx": episode_idx,
                "episode_success": success,
                "token_groups": token_groups,
            }
            _write_npz_atomic(
                directory / f"{stem}{ARRAYS_SUFFIX}", hidden_states=hidden
            )
            _atomic_write_json(
                directory / f"{stem}{METADATA_SUFFIX}", meta, overwrite=True
            )
    synthetic_config = {
        "schema_version": 1,
        "n_tasks": n_tasks,
        "episodes_per_task": episodes_per_task,
        "n_steps": n_steps,
        "n_tokens": n_tokens,
        "d_hidden": d_hidden,
        "d_action": d_action,
        "seed": seed,
        "raw_token_states": raw_token_states,
    }
    synthetic_revision = _sha256_bytes(_canonical_json_bytes(synthetic_config))
    tensor_contract = {
        "schema_version": 1,
        "site": "synthetic_hidden_states",
        "shape": ["steps", "tokens" if raw_token_states else None, "hidden"],
        "dtype": "float64",
        "token_groups": "synthetic_declared_only" if raw_token_states else None,
    }
    write_safe_bundle_manifest(
        directory,
        source_name="prisoma/synthetic-safe",
        source_revision=synthetic_revision,
        rights_status="synthetic_generated",
        rights_reference="generated locally by experiments.safe_adapter",
        seen_task_ids=range(n_tasks // 2),
        overwrite=True,
        split_origin="deterministic_task_id_prefix_from_synthetic_config",
        split_frozen_before_outcomes=True,
        contamination_review="synthetic generator owns disjoint task ids; no external corpus",
        model_id="synthetic-generator",
        checkpoint_revision=synthetic_revision,
        hook_id="synthetic_hidden_states",
        tensor_contract_sha256=_sha256_bytes(_canonical_json_bytes(tensor_contract)),
        semantic_validation_status="unvalidated",
    )
    return directory


def _write_action_csv(path: Path, actions: np.ndarray) -> None:
    fd, temp_name = tempfile.mkstemp(
        dir=path.parent,
        prefix=f".{path.name}.",
        suffix=".tmp",
    )
    temp_path = Path(temp_name)
    try:
        with os.fdopen(fd, "w", newline="", encoding="utf-8") as handle:
            writer = csv.writer(handle)
            writer.writerow(ACTION_COLUMNS)
            for row in actions:
                writer.writerow(f"{v:.8g}" for v in row)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temp_path, path)
        _fsync_directory(path.parent)
    except BaseException:
        temp_path.unlink(missing_ok=True)
        raise


def _write_npz_atomic(path: Path, **arrays: np.ndarray) -> None:
    fd, temp_name = tempfile.mkstemp(
        dir=path.parent,
        prefix=f".{path.name}.",
        suffix=".tmp",
    )
    temp_path = Path(temp_name)
    try:
        with os.fdopen(fd, "wb") as handle:
            np.savez(handle, **arrays)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temp_path, path)
        _fsync_directory(path.parent)
    except BaseException:
        temp_path.unlink(missing_ok=True)
        raise
