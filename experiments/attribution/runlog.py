"""Emit ``attribution_logged`` run-log events conformant to the Rust schema.

The Rust ``RunLogEvent::AttributionLogged`` variant
(``pid-rs/crates/pid-runlog/src/lib.rs``) already exists; this module writes a canonical
JSONL run log (``run_started`` / ``config_logged`` / one ``attribution_logged`` per
probe / ``run_ended``) plus the attribution arrays as artifact files with sha256
provenance, so the result passes ``pid-runlog-replay --validate``.

Hash compatibility: the validator recomputes ``sha256(serde_json::to_vec(config))``
and checks it equals the logged ``config_hash``. serde_json (no ``preserve_order``)
serializes objects with **sorted keys, compact separators**, which
``json.dumps(..., sort_keys=True, separators=(",", ":"))`` reproduces — *provided
the config is composed only of objects, lists, strings, integers, and booleans* (we
never put floats or nulls in the config, to avoid cross-language formatting ambiguity).
The same canonical hash is used for
``run_started`` and ``config_logged`` so they agree.
"""

from __future__ import annotations

import hashlib
import io
import json
import os
import stat
import tempfile
import unicodedata
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

import numpy as np

SCHEMA_VERSION = 2
MAX_RERUN_RELEVANCE_VALUES = 1024
MAX_RERUN_RELEVANCE_FILE_BYTES = 12 * 1024
MAX_RERUN_PREPARED_ARTIFACT_BYTES = 8 * 1024 * 1024
MAX_RERUN_EVENTS = 100_000
MAX_RERUN_SERIALIZED_EVENT_BYTES = 64 * 1024 * 1024

# Keep the producer inside pid-runlog 1.0's default bounded-reader contract so
# every emitted line can be consumed by the canonical validator. These are
# intentionally duplicated here because the Python producer cannot import Rust
# constants at runtime; focused tests pin the cross-language boundary behavior.
MAX_RUNLOG_FILE_BYTES = 256 * 1024 * 1024
MAX_RUNLOG_LINE_BYTES = 4 * 1024 * 1024
MAX_RUNLOG_STRING_BYTES = 1024 * 1024
MAX_RUNLOG_ARRAY_LEN = 1_000_000
MAX_RUNLOG_OBJECT_ENTRIES = 100_000
MAX_RUNLOG_NESTING_DEPTH = 64


def _canonical_run_id(run_id: object) -> str:
    """Return a portable, normalization-stable run identifier."""
    if type(run_id) is not str or not run_id:
        raise ValueError("run_id must be a non-empty string")
    if run_id != run_id.strip():
        raise ValueError("run_id must not have leading or trailing whitespace")
    if unicodedata.normalize("NFC", run_id) != run_id:
        raise ValueError("run_id must use canonical NFC Unicode normalization")
    if any(ord(character) < 0x20 or ord(character) == 0x7F for character in run_id):
        raise ValueError("run_id must not contain control characters")
    return run_id


def _validate_canonical_value(
    value: object,
    path: str = "$",
    *,
    _depth: int = 0,
    _active_containers: set[int] | None = None,
) -> None:
    """Restrict cross-language hashes to JSON forms with identical number text."""
    if type(value) in (dict, list):
        depth = _depth + 1
        if depth > MAX_RUNLOG_NESTING_DEPTH:
            raise ValueError(
                "canonical JSON nesting exceeds the run-log limit "
                f"{MAX_RUNLOG_NESTING_DEPTH} at {path}"
            )
        active = _active_containers if _active_containers is not None else set()
        identity = id(value)
        if identity in active:
            raise ValueError(f"canonical JSON value at {path} contains a cycle")
        active.add(identity)
    else:
        depth = _depth
        active = _active_containers

    if type(value) is dict:
        if len(value) > MAX_RUNLOG_OBJECT_ENTRIES:
            raise ValueError(
                "canonical JSON object exceeds the run-log entry limit "
                f"{MAX_RUNLOG_OBJECT_ENTRIES} at {path}"
            )
        try:
            for key, child in value.items():
                if type(key) is not str:
                    raise ValueError(
                        f"canonical JSON object key at {path} must be a string"
                    )
                _validate_canonical_value(
                    child,
                    f"{path}.{key}",
                    _depth=depth,
                    _active_containers=active,
                )
        finally:
            active.remove(id(value))
        return
    if type(value) is list:
        if len(value) > MAX_RUNLOG_ARRAY_LEN:
            raise ValueError(
                "canonical JSON array exceeds the run-log length limit "
                f"{MAX_RUNLOG_ARRAY_LEN} at {path}"
            )
        try:
            for index, child in enumerate(value):
                _validate_canonical_value(
                    child,
                    f"{path}[{index}]",
                    _depth=depth,
                    _active_containers=active,
                )
        finally:
            active.remove(id(value))
        return
    if type(value) is int:
        if value < -(2**63) or value > 2**64 - 1:
            raise ValueError(
                f"canonical JSON value integer at {path} is outside serde_json's exact range"
            )
        return
    if type(value) in (str, bool):
        return
    raise ValueError(
        f"canonical JSON value at {path} must be a string, integer, boolean, list, or object"
    )


def canonical_hash(value: object) -> str:
    """Reproduce the Rust canonical hash for JSON built from exact shared forms."""
    # ``ensure_ascii=False`` is REQUIRED: serde_json writes raw UTF-8, while
    # Python's default escapes non-ASCII to ``\uXXXX`` — so a config value such as
    # ``I^sx_∩`` (which the docset uses freely) would otherwise hash differently
    # in Python than in Rust and fail ``pid-runlog-replay --validate``.
    _validate_canonical_value(value)
    payload = json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def sha256_file(path: str | Path) -> str:
    h = hashlib.sha256()
    with Path(path).open("rb") as handle:
        for chunk in iter(lambda: handle.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


_PRODUCER_METADATA_KEYS = frozenset({"artifact_sha256", "relevance_shape"})


def _validated_record_metadata(metadata: object, record_index: int) -> dict[str, str]:
    """Preserve metadata exactly; never coerce keys or values through ``str``."""
    if metadata is None:
        return {}
    if type(metadata) is not dict:
        raise ValueError(f"record {record_index} metadata must be a plain dictionary")

    normalized_keys: dict[str, str] = {}
    result: dict[str, str] = {}
    for key, value in metadata.items():
        if type(key) is not str or type(value) is not str:
            raise ValueError(
                f"record {record_index} metadata keys and values must be exact strings"
            )
        normalized = unicodedata.normalize("NFC", key)
        prior = normalized_keys.get(normalized)
        if prior is not None and prior != key:
            raise ValueError(
                f"record {record_index} metadata contains a Unicode normalization collision"
            )
        if normalized != key:
            raise ValueError(
                f"record {record_index} metadata keys must use canonical NFC normalization"
            )
        if key in _PRODUCER_METADATA_KEYS:
            raise ValueError(
                f"record {record_index} metadata key {key!r} is reserved by the producer"
            )
        normalized_keys[normalized] = key
        result[key] = value
    return result


def _validate_json_resource_bytes(payload: bytes, line_number: int) -> None:
    """Mirror pid-runlog's bounded JSON scanner for one serialized event."""
    if len(payload) > MAX_RUNLOG_LINE_BYTES:
        raise ValueError(
            f"run-log line {line_number} exceeds the {MAX_RUNLOG_LINE_BYTES}-byte limit"
        )

    # Each frame is [container byte, comma count, has content]. The payload was
    # produced by json.dumps, so syntax is already valid; this scan enforces the
    # canonical reader's encoded-string, nesting, and container budgets exactly.
    stack: list[list[int | bool]] = []
    in_string = False
    escaped = False
    string_bytes = 0
    for byte in payload:
        if in_string:
            if escaped:
                escaped = False
                string_bytes += 1
            elif byte == ord("\\"):
                escaped = True
                string_bytes += 1
            elif byte == ord('"'):
                in_string = False
            else:
                string_bytes += 1
            if string_bytes > MAX_RUNLOG_STRING_BYTES:
                raise ValueError(
                    f"run-log line {line_number} contains a JSON string exceeding "
                    f"the {MAX_RUNLOG_STRING_BYTES}-byte limit"
                )
            continue

        if byte == ord('"'):
            if stack:
                stack[-1][2] = True
            in_string = True
            string_bytes = 0
        elif byte in (ord("{"), ord("[")):
            if stack:
                stack[-1][2] = True
            depth = len(stack) + 1
            if depth > MAX_RUNLOG_NESTING_DEPTH:
                raise ValueError(
                    f"run-log line {line_number} exceeds the JSON nesting limit "
                    f"{MAX_RUNLOG_NESTING_DEPTH}"
                )
            stack.append([byte, 0, False])
        elif byte == ord(","):
            if stack:
                stack[-1][1] = int(stack[-1][1]) + 1
        elif byte in (ord("}"), ord("]")):
            frame = stack.pop()
            entries = int(frame[1]) + 1 if frame[2] else 0
            if frame[0] == ord("["):
                limit = MAX_RUNLOG_ARRAY_LEN
                kind = "array length"
            else:
                limit = MAX_RUNLOG_OBJECT_ENTRIES
                kind = "object entries"
            if entries > limit:
                raise ValueError(
                    f"run-log line {line_number} exceeds the JSON {kind} limit {limit}"
                )
        elif not chr(byte).isspace() and stack:
            stack[-1][2] = True


def _serialize_bounded_runlog(events: Sequence[dict]) -> bytes:
    if len(events) > MAX_RERUN_EVENTS:
        raise ValueError(
            f"run-log event count exceeds the {MAX_RERUN_EVENTS}-event viewer limit"
        )

    output = bytearray()
    serialized_event_bytes = 0
    for line_number, event in enumerate(events, start=1):
        payload = json.dumps(event, ensure_ascii=False, allow_nan=False).encode("utf-8")
        _validate_json_resource_bytes(payload, line_number)
        serialized_event_bytes += len(payload)
        if serialized_event_bytes > MAX_RERUN_SERIALIZED_EVENT_BYTES:
            raise ValueError(
                "run-log serialized-event aggregate exceeds the "
                f"{MAX_RERUN_SERIALIZED_EVENT_BYTES}-byte viewer limit"
            )
        projected_total = len(output) + len(payload) + 1
        if projected_total > MAX_RUNLOG_FILE_BYTES:
            raise ValueError(
                f"run-log aggregate bytes exceed the {MAX_RUNLOG_FILE_BYTES}-byte limit"
            )
        output.extend(payload)
        output.append(ord("\n"))
    return bytes(output)


def _artifact_filename(content_hash: str) -> str:
    return f"{content_hash}.npy"


def _lexical_absolute(path: str | Path) -> Path:
    """Return an absolute, normalized path without following symlinks."""
    return Path(os.path.abspath(os.fspath(path)))


def _validate_path_components(path: Path, *, leaf_kind: str) -> None:
    """Reject symlinked/non-directory ancestors and unsafe existing leaves."""
    current = Path(path.anchor)
    for part in path.parts[1:]:
        current /= part
        is_leaf = current == path
        try:
            metadata = current.lstat()
        except FileNotFoundError:
            return

        if stat.S_ISLNK(metadata.st_mode):
            raise ValueError(f"publication path must not contain symlinks: {current}")
        if not is_leaf:
            if not stat.S_ISDIR(metadata.st_mode):
                raise ValueError(
                    f"publication path ancestor must be a directory: {current}"
                )
            continue

        if leaf_kind == "directory":
            if not stat.S_ISDIR(metadata.st_mode):
                raise ValueError(f"artifact_dir must be a directory: {current}")
        elif leaf_kind == "file":
            if not stat.S_ISREG(metadata.st_mode):
                raise ValueError(f"runlog_path must be a regular file: {current}")
            if metadata.st_nlink != 1:
                raise ValueError(
                    "runlog_path must not have hard-link aliases before replacement"
                )
        else:  # pragma: no cover - internal programming error
            raise AssertionError(f"unknown leaf kind: {leaf_kind}")


def _validate_publication_topology(
    runlog_path: Path, artifact_dir: Path | None
) -> None:
    """Validate publication paths without creating or resolving through aliases."""
    if runlog_path == Path(runlog_path.anchor):
        raise ValueError("runlog_path must name a file, not a filesystem root")
    _validate_path_components(runlog_path, leaf_kind="file")

    if artifact_dir is None:
        return
    runlog_dir = runlog_path.parent
    try:
        relative_artifact_dir = artifact_dir.relative_to(runlog_dir)
    except ValueError as error:
        raise ValueError(
            "artifact_dir must be a strict descendant of the run log directory"
        ) from error
    if not relative_artifact_dir.parts:
        raise ValueError(
            "artifact_dir must be a strict descendant of the run log directory"
        )
    if artifact_dir == runlog_path or runlog_path in artifact_dir.parents:
        raise ValueError(
            "runlog_path and artifact_dir must not alias or contain each other"
        )
    _validate_path_components(artifact_dir, leaf_kind="directory")

    if runlog_dir.exists() and artifact_dir.exists():
        try:
            aliases_runlog_dir = os.path.samefile(runlog_dir, artifact_dir)
        except OSError as error:
            raise ValueError("unable to validate artifact_dir topology") from error
        if aliases_runlog_dir:
            raise ValueError("artifact_dir must not alias the run log directory")


def _stage_synced_bytes(directory: Path, payload: bytes) -> Path:
    """Write and file-sync one temporary file in its destination directory."""
    descriptor, raw_path = tempfile.mkstemp(
        prefix=".attribution-stage-", suffix=".tmp", dir=directory
    )
    staged_path = Path(raw_path)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
    except BaseException:
        staged_path.unlink(missing_ok=True)
        raise
    return staged_path


def _verify_existing_artifact(path: Path, expected_bytes: bytes) -> None:
    try:
        metadata = path.lstat()
    except FileNotFoundError as error:
        raise OSError(
            f"installed artifact disappeared before verification: {path}"
        ) from error
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        raise ValueError(
            f"artifact destination must be a regular non-symlink file: {path}"
        )
    if metadata.st_nlink != 1:
        raise ValueError(
            f"artifact destination must not have hard-link aliases: {path}"
        )
    if metadata.st_size != len(expected_bytes) or path.read_bytes() != expected_bytes:
        raise FileExistsError(
            f"content-addressed artifact exists with different bytes: {path}"
        )


def _preflight_artifact_destination(path: Path, expected_bytes: bytes) -> None:
    try:
        path.lstat()
    except FileNotFoundError:
        return
    _verify_existing_artifact(path, expected_bytes)


def _install_staged_artifact(
    staged_path: Path, artifact_path: Path, artifact_bytes: bytes
) -> None:
    """Install one staged artifact by a no-clobber hard-link operation."""
    try:
        os.link(staged_path, artifact_path, follow_symlinks=False)
    except FileExistsError:
        _verify_existing_artifact(artifact_path, artifact_bytes)
        return

    staged_path.unlink()
    _verify_existing_artifact(artifact_path, artifact_bytes)


@dataclass
class AttributionRecord:
    """One attribution result to log (mirrors the Rust event fields)."""

    method: str
    target_output: str
    relevance: np.ndarray
    faithfulness_passed: bool
    layer: str | None = None
    modality: str | None = None
    baseline: str | None = None
    metadata: dict[str, str] | None = None

    def __post_init__(self) -> None:
        if type(self.method) is not str or not self.method:
            raise ValueError(
                "method must be a non-empty string (the harness validator rejects it)"
            )
        if type(self.target_output) is not str or not self.target_output:
            raise ValueError("target_output must be a non-empty string")
        for field_name in ("layer", "modality", "baseline"):
            value = getattr(self, field_name)
            if value is not None and type(value) is not str:
                raise ValueError(f"{field_name} must be a string or None")
        if type(self.faithfulness_passed) is not bool:
            raise ValueError("faithfulness_passed must be a boolean")


def write_attribution_runlog(
    runlog_path: str | Path,
    records: Sequence[AttributionRecord],
    *,
    run_id: str = "attribution-probe",
    config: dict | None = None,
    artifact_dir: str | Path | None = None,
) -> Path:
    """Write a canonical run log for a batch of attribution records.

    Each record's ``relevance`` array is saved as an exact NumPy v1.0 little-endian
    ``f64`` C-order artifact (when ``artifact_dir`` is given). ``artifact_dir`` must
    resolve inside the run log's directory, and ``artifact_uri`` is emitted relative
    to that directory for the standalone converter's confined, explicit loader.
    ``artifact_sha256`` is retained in metadata; a ``score_hash`` over the rounded
    relevance is always recorded so identical attributions are detectable without
    the file. Artifacts are content-addressed and installed without replacing an
    existing name. Their file contents and the staged run log are synced before the
    run-log name is atomically replaced last.
    """
    run_id = _canonical_run_id(run_id)
    projected_event_count = len(records) + 3
    if projected_event_count > MAX_RERUN_EVENTS:
        raise ValueError(
            f"run-log event count exceeds the {MAX_RERUN_EVENTS}-event viewer limit"
        )

    requested_runlog_path = Path(runlog_path)
    publication_runlog_path = _lexical_absolute(requested_runlog_path)
    runlog_dir = publication_runlog_path.parent
    resolved_artifact_dir = None
    if artifact_dir is not None:
        resolved_artifact_dir = _lexical_absolute(artifact_dir)
    _validate_publication_topology(publication_runlog_path, resolved_artifact_dir)

    if config is not None and type(config) is not dict:
        raise ValueError("config must be a plain dictionary when provided")
    config = dict(config or {})
    config.setdefault("experiment", "attribution_probe")
    config.setdefault("n_records", str(len(records)))
    config_hash = canonical_hash(config)

    events: list[dict] = []
    prepared_artifacts: dict[str, tuple[Path, bytes]] = {}
    prepared_artifact_bytes = 0
    events.append(
        {
            "type": "run_started",
            "schema_version": SCHEMA_VERSION,
            "run_id": run_id,
            "timestamp_ns": 0,
            "config_hash": config_hash,
            "metadata": {"source": "attribution-probe"},
        }
    )
    events.append(
        {
            "type": "config_logged",
            "timestamp_ns": 0,
            "config_hash": config_hash,
            "config": config,
        }
    )

    ts = 1
    for i, rec in enumerate(records):
        if type(rec.method) is not str or not rec.method:
            raise ValueError(
                "method must be a non-empty string (the harness validator rejects it)"
            )
        if type(rec.target_output) is not str or not rec.target_output:
            raise ValueError("target_output must be a non-empty string")
        for field_name in ("layer", "modality", "baseline"):
            value = getattr(rec, field_name)
            if value is not None and type(value) is not str:
                raise ValueError(f"{field_name} must be a string or None")
        if type(rec.faithfulness_passed) is not bool:
            raise ValueError("faithfulness_passed must be a boolean")
        if type(rec.relevance) is not np.ndarray:
            raise ValueError("relevance must be a NumPy ndarray")
        if rec.relevance.size > MAX_RERUN_RELEVANCE_VALUES:
            raise ValueError(
                "relevance arrays must contain at most "
                f"{MAX_RERUN_RELEVANCE_VALUES} values"
            )
        rel = np.ascontiguousarray(rec.relevance, dtype=np.dtype("<f8"))
        if rel.size == 0:
            raise ValueError("relevance arrays must be non-empty")
        if not np.isfinite(rel).all():
            raise ValueError("relevance arrays must contain only finite values")
        # Stable score hash over the rounded relevance (order/precision-stable).
        score_hash = hashlib.sha256(
            np.round(rel, 8).tobytes(order="C") + rel.shape.__repr__().encode()
        ).hexdigest()
        metadata = _validated_record_metadata(rec.metadata, i)
        metadata["relevance_shape"] = "x".join(str(n) for n in rel.shape)

        artifact_uri = None
        if resolved_artifact_dir is not None:
            artifact_buffer = io.BytesIO()
            np.lib.format.write_array(
                artifact_buffer, rel, version=(1, 0), allow_pickle=False
            )
            artifact_bytes = artifact_buffer.getvalue()
            if len(artifact_bytes) > MAX_RERUN_RELEVANCE_FILE_BYTES:
                raise ValueError(
                    "relevance artifact exceeds the Rerun converter's "
                    f"{MAX_RERUN_RELEVANCE_FILE_BYTES}-byte limit"
                )
            artifact_hash = hashlib.sha256(artifact_bytes).hexdigest()
            artifact_path = resolved_artifact_dir / _artifact_filename(artifact_hash)
            artifact_uri = artifact_path.relative_to(runlog_dir).as_posix()
            metadata["artifact_sha256"] = artifact_hash
            prior_artifact = prepared_artifacts.get(artifact_hash)
            if prior_artifact is not None and prior_artifact[1] != artifact_bytes:
                raise RuntimeError("sha256 collision across attribution artifacts")
            if prior_artifact is None:
                prepared_artifact_bytes += len(artifact_bytes)
                if prepared_artifact_bytes > MAX_RERUN_PREPARED_ARTIFACT_BYTES:
                    raise ValueError(
                        "unique prepared relevance artifacts exceed the "
                        f"{MAX_RERUN_PREPARED_ARTIFACT_BYTES}-byte aggregate limit"
                    )
                prepared_artifacts[artifact_hash] = (artifact_path, artifact_bytes)

        events.append(
            {
                "type": "attribution_logged",
                "timestamp_ns": ts,
                "method": rec.method,
                "target_output": rec.target_output,
                "layer": rec.layer,
                "modality": rec.modality,
                "baseline": rec.baseline,
                "score_hash": score_hash,
                "faithfulness_check": bool(rec.faithfulness_passed),
                "artifact_uri": artifact_uri,
                "metadata": metadata,
            }
        )
        ts += 1

    events.append(
        {
            "type": "run_ended",
            "run_id": run_id,
            "timestamp_ns": ts,
            "status": "succeeded",
            "message": f"logged {len(records)} attribution record(s)",
        }
    )

    # Serialize the complete log with strict finite-number handling before touching
    # the filesystem. Together with the in-memory artifact framing above, this
    # prevents invalid content from leaving publication files behind.
    runlog_bytes = _serialize_bounded_runlog(events)

    # Existing content-addressed names are checked before any directory or staging
    # file is created, then checked again during the no-clobber installation.
    for artifact_path, artifact_bytes in prepared_artifacts.values():
        _preflight_artifact_destination(artifact_path, artifact_bytes)

    publication_runlog_path.parent.mkdir(parents=True, exist_ok=True)
    if resolved_artifact_dir is not None:
        resolved_artifact_dir.mkdir(parents=True, exist_ok=True)
    _validate_publication_topology(publication_runlog_path, resolved_artifact_dir)

    staged_artifacts: list[tuple[Path, Path, bytes]] = []
    staged_runlog: Path | None = None
    try:
        if resolved_artifact_dir is not None:
            for artifact_path, artifact_bytes in prepared_artifacts.values():
                staged_path = _stage_synced_bytes(resolved_artifact_dir, artifact_bytes)
                staged_artifacts.append((staged_path, artifact_path, artifact_bytes))
        staged_runlog = _stage_synced_bytes(runlog_dir, runlog_bytes)

        for staged_path, artifact_path, artifact_bytes in staged_artifacts:
            _install_staged_artifact(staged_path, artifact_path, artifact_bytes)

        # Recheck the named topology immediately before publishing the new log. The
        # run log is the final visible name changed by this function.
        _validate_publication_topology(publication_runlog_path, resolved_artifact_dir)
        os.replace(staged_runlog, publication_runlog_path)
        staged_runlog = None
    finally:
        for staged_path, _, _ in staged_artifacts:
            staged_path.unlink(missing_ok=True)
        if staged_runlog is not None:
            staged_runlog.unlink(missing_ok=True)

    return requested_runlog_path
