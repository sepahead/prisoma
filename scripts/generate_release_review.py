#!/usr/bin/env python3
"""Generate the deterministic Prisoma 0.9.0 release-review intake baseline.

The external handoff is intentionally not discoverable by this program.  An operator must
name the reviewed ``19_MASTER_TASK_LEDGER.yaml`` with ``--master-ledger``.  The generated
records normalize that ledger and inventory the historical Git cut; they do not claim that
any task or file has received human or independent review.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import selectors
import stat
import subprocess
import sys
import tempfile
import time
import unicodedata
from collections.abc import Mapping, Sequence
from pathlib import Path, PurePosixPath
from typing import Any


SCHEMA_VERSION = "prisoma.release-review/0.1.0"
PROJECT = "prisoma"
REPOSITORY = "https://github.com/sepahead/prisoma"
AUTHOR = "Sepehr Mahmoudian"
RELEASE_VERSION = "0.9.0"
NOMINAL_HANDOFF_RELEASE = "1.0.0"
FROZEN_COMMIT = "0968128062f30da5c04f3f31c23f6ce8e0d95d36"
FROZEN_TREE = "d7ee5763cbdc5906c91ff4c82c5fc9a124c6aa84"
PID_RS_PATH = "pid-rs"
PID_RS_COMMIT = "ac4a7803c5a77408f5e9176c60cda71c65c38260"
EXPECTED_HANDOFF_SHA256 = (
    "384f5540dcdb4709b8f9add57e355761c6e076a1c4b22e26e42482bd0c0c4f29"
)
# Filled from canonical_json({"phases": phases, "tasks": tasks}) after reviewing the
# external source.  Updating the handoff requires an explicit code review, not merely
# regenerating a mutable digest beside changed task text.
EXPECTED_TASK_GRAPH_SHA256 = (
    "ac2926271314f48052c4b7d84862fff20c0f7f76a3e73816cc5018e3583fec71"
)
EXPECTED_HANDOFF_BYTES = 2_281_617
EXPECTED_TASK_COUNT = 240
EXPECTED_PHASE_COUNT = 16
EXPECTED_TRACKED_COUNT = 175
MAX_MASTER_LEDGER_BYTES = 16 * 1024 * 1024
MAX_ARTIFACT_BYTES = 64 * 1024 * 1024
MAX_GIT_STDERR_BYTES = 1024 * 1024
GIT_COMMAND_TIMEOUT_SECONDS = 120.0
INVENTORY_GIT_DEADLINE_SECONDS = 120.0
MAX_TREE_LISTING_BYTES = 4 * 1024 * 1024
MAX_FROZEN_PATH_BYTES = 1024 * 1024
MAX_FROZEN_AGGREGATE_BYTES = 512 * 1024 * 1024

ARTIFACT_NAMES = (
    "intake.json",
    "master_task_ledger.normalized.json",
    "tracked_file_inventory.baseline.json",
)
MANIFEST_NAME = "artifact_manifest.json"

_TASK_ID_RE = re.compile(r"T([0-9]{3})\Z")
_PHASE_ID_RE = re.compile(r"P([0-9]{2})\Z")
_OID_RE = re.compile(r"[0-9a-f]{40}\Z")
_TOP_FIELDS = (
    "handoff_schema",
    "project",
    "repository",
    "frozen_commit",
    "release_target",
    "status",
    "phase_count",
    "task_count",
)
_TASK_SCALAR_FIELDS = (
    "id",
    "phase_id",
    "phase_title",
    "title",
    "priority",
    "execution_wave",
    "subagent_lane",
    "current_head",
)

_GENERATED_PATHS = frozenset(
    {
        "THIRD_PARTY_NOTICES.generated.md",
        "docs/CAPABILITY_MATRIX.md",
        "docs/power-gate/POWER-GATE-2026-07-10.md",
        "docs/power-gate/power-gate-2026-07-10.json",
        "docs/reviews/2026-07-12-grandplan-v12.5/prisoma_review_manifest_v2.json",
        "docs/reviews/2026-07-12-grandplan-v12.5/validation_checks_v2.json",
        "outputs/arxiv_ref_cache.json",
        "protocols/capability_matrix_current_v1.json",
    }
)
_ROOT_PUBLIC_PATHS = frozenset(
    {
        "AGENTS.md",
        "ARCHITECTURE.md",
        "CHANGELOG.md",
        "CLAUDE.md",
        "Cargo.toml",
        "DIAGRAMS.md",
        "EXPERIMENTS.md",
        "LICENSE-APACHE",
        "LICENSE-MIT",
        "NCP_DEV_PROMPT.md",
        "README.md",
        "REVIEW_AND_TODO.md",
        "SECURITY.md",
        "THIRD_PARTY_NOTICES.generated.md",
        "THIRD_PARTY_NOTICES.md",
        "findings.md",
        "grandplan.md",
        "justfile",
        "pid-splat.toml",
        "pidsplatspecs.md",
        "pyproject.toml",
    }
)
_SECURITY_ROOT_PATHS = frozenset(
    {
        ".gitmodules",
        ".pre-commit-config.yaml",
        "Cargo.lock",
        "Cargo.toml",
        "SECURITY.md",
        "deny.toml",
        "flake.nix",
        "pyproject.toml",
        "uv.lock",
    }
)
_SCIENCE_ROOT_PATHS = frozenset(
    {
        "EXPERIMENTS.md",
        "findings.md",
        "grandplan.md",
        "pidsplatspecs.md",
        "RESEARCH_VLA_D_NCP.md",
    }
)


class ReleaseReviewError(ValueError):
    """A controlled, user-facing release-review validation failure."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


def _fail(code: str, message: str) -> None:
    raise ReleaseReviewError(code, message)


def _stat_identity(value: os.stat_result) -> tuple[int, int, int, int, int, int]:
    """Return metadata that must remain stable across one bounded file snapshot."""

    return (
        value.st_dev,
        value.st_ino,
        value.st_mode,
        value.st_size,
        value.st_mtime_ns,
        value.st_ctime_ns,
    )


def _read_bounded_regular(
    path: Path,
    *,
    max_bytes: int,
    path_code: str,
    read_code: str,
    too_large_code: str,
    description: str,
) -> bytes:
    """Read a stable, bounded regular-file snapshot without following a final symlink."""

    if max_bytes < 0:
        _fail(read_code, f"invalid negative read limit for {description}: {max_bytes}")
    try:
        path_before = os.stat(path, follow_symlinks=False)
    except OSError as exc:
        _fail(path_code, f"cannot inspect {description} {path}: {exc}")
    if not stat.S_ISREG(path_before.st_mode):
        _fail(path_code, f"{description} must be a regular non-symlink file: {path}")

    flags = (
        os.O_RDONLY
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_NOFOLLOW", 0)
        | getattr(os, "O_NONBLOCK", 0)
    )
    try:
        descriptor = os.open(path, flags)
    except OSError as exc:
        _fail(path_code, f"cannot open {description} {path}: {exc}")

    try:
        try:
            opened = os.fstat(descriptor)
        except OSError as exc:
            _fail(read_code, f"cannot inspect opened {description} {path}: {exc}")
        if not stat.S_ISREG(opened.st_mode):
            _fail(path_code, f"{description} must be a regular file: {path}")
        if (path_before.st_dev, path_before.st_ino) != (opened.st_dev, opened.st_ino):
            _fail(read_code, f"{description} path changed while it was opened: {path}")
        if opened.st_size > max_bytes:
            _fail(
                too_large_code,
                f"{description} exceeds the {max_bytes}-byte limit: {path}",
            )

        chunks: list[bytes] = []
        total = 0
        while total <= max_bytes:
            request = min(1024 * 1024, max_bytes + 1 - total)
            try:
                chunk = os.read(descriptor, request)
            except OSError as exc:
                _fail(read_code, f"cannot read {description} {path}: {exc}")
            if not chunk:
                break
            chunks.append(chunk)
            total += len(chunk)
        if total > max_bytes:
            _fail(
                too_large_code,
                f"{description} exceeds the {max_bytes}-byte limit: {path}",
            )
        raw = b"".join(chunks)

        try:
            closed_snapshot = os.fstat(descriptor)
            path_after = os.stat(path, follow_symlinks=False)
        except OSError as exc:
            _fail(read_code, f"cannot verify {description} snapshot {path}: {exc}")
        if (
            not stat.S_ISREG(closed_snapshot.st_mode)
            or not stat.S_ISREG(path_after.st_mode)
            or _stat_identity(opened) != _stat_identity(closed_snapshot)
            or _stat_identity(closed_snapshot) != _stat_identity(path_after)
            or len(raw) != closed_snapshot.st_size
        ):
            _fail(read_code, f"{description} changed while it was read: {path}")
        return raw
    finally:
        try:
            os.close(descriptor)
        except OSError:
            pass


def canonical_json_bytes(value: Any) -> bytes:
    """Return the sole canonical byte representation used for semantic digests."""

    return json.dumps(
        value,
        ensure_ascii=False,
        allow_nan=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")


def pretty_json_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, allow_nan=False, indent=2, sort_keys=True)
        + "\n"
    ).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def semantic_sha256(value: Any) -> str:
    return sha256_bytes(canonical_json_bytes(value))


def _run_git(
    repo: Path,
    args: Sequence[str],
    *,
    input_bytes: bytes | None = None,
    max_bytes: int = MAX_ARTIFACT_BYTES,
    timeout_seconds: float = GIT_COMMAND_TIMEOUT_SECONDS,
) -> bytes:
    if max_bytes < 0:
        _fail("GIT_OUTPUT", f"invalid negative Git output budget: {max_bytes}")
    if timeout_seconds <= 0:
        _fail("GIT_TIMEOUT", "Git command has no remaining execution budget")
    command = ["git", "-C", os.fspath(repo), *args]
    input_stream = None
    try:
        if input_bytes is not None:
            input_stream = tempfile.TemporaryFile()
            input_stream.write(input_bytes)
            input_stream.seek(0)
        process = subprocess.Popen(
            command,
            stdin=input_stream if input_stream is not None else subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except OSError as exc:
        if input_stream is not None:
            input_stream.close()
        _fail("GIT_UNAVAILABLE", f"cannot execute git: {exc}")
    assert process.stdout is not None and process.stderr is not None
    selector = selectors.DefaultSelector()
    stdout = bytearray()
    stderr = bytearray()
    overflow: str | None = None
    timed_out = False
    deadline = time.monotonic() + timeout_seconds
    try:
        for stream, label in ((process.stdout, "stdout"), (process.stderr, "stderr")):
            os.set_blocking(stream.fileno(), False)
            selector.register(stream, selectors.EVENT_READ, label)
        while selector.get_map():
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                timed_out = True
                break
            for key, _ in selector.select(min(remaining, 1.0)):
                target = stdout if key.data == "stdout" else stderr
                limit = max_bytes if key.data == "stdout" else MAX_GIT_STDERR_BYTES
                try:
                    chunk = os.read(key.fd, min(1024 * 1024, limit + 1 - len(target)))
                except BlockingIOError:
                    continue
                if not chunk:
                    selector.unregister(key.fileobj)
                    key.fileobj.close()
                    continue
                target.extend(chunk)
                if len(target) > limit:
                    overflow = key.data
                    break
            if overflow is not None:
                break
        if timed_out or overflow is not None:
            process.kill()
        remaining = max(0.0, deadline - time.monotonic())
        try:
            returncode = process.wait(timeout=max(1.0, remaining))
        except subprocess.TimeoutExpired:
            timed_out = True
            process.kill()
            process.wait(timeout=5.0)
            returncode = process.returncode
    finally:
        selector.close()
        for stream in (process.stdout, process.stderr):
            if not stream.closed:
                stream.close()
        if input_stream is not None:
            input_stream.close()
    if timed_out:
        _fail(
            "GIT_TIMEOUT",
            f"git {' '.join(args)} exceeded {timeout_seconds:g} seconds",
        )
    if overflow == "stdout":
        _fail("GIT_OUTPUT", f"git {' '.join(args)} exceeded the output budget")
    if overflow == "stderr":
        _fail("GIT_STDERR", f"git {' '.join(args)} exceeded the stderr budget")
    if returncode != 0:
        detail = bytes(stderr).decode("utf-8", errors="replace").strip()
        _fail("GIT_COMMAND_FAILED", f"git {' '.join(args)} failed: {detail}")
    return bytes(stdout)


def resolve_repo(repo: Path) -> Path:
    if not repo.exists() or not repo.is_dir():
        _fail("REPOSITORY_MISSING", f"repository directory does not exist: {repo}")
    raw = _run_git(repo, ["rev-parse", "--show-toplevel"], max_bytes=16 * 1024)
    try:
        top = Path(raw.decode("utf-8", errors="strict").strip()).resolve(strict=True)
    except (UnicodeDecodeError, OSError) as exc:
        _fail("REPOSITORY_INVALID", f"cannot resolve repository root: {exc}")
    return top


def _normalize_text(value: str, *, field: str) -> str:
    normalized = unicodedata.normalize("NFC", " ".join(value.split()))
    if not normalized or any(ord(char) < 0x20 for char in normalized):
        _fail(
            "LEDGER_TEXT_INVALID", f"{field} is empty or contains a control character"
        )
    return normalized


def _parse_yaml_scalar(raw: str, *, field: str) -> str:
    """Parse the narrow string-scalar subset used by the reviewed handoff fields."""

    value = raw.strip()
    if not value:
        _fail("LEDGER_SCALAR_MISSING", f"{field} has no scalar value")
    if value.startswith("'"):
        if len(value) < 2 or not value.endswith("'"):
            _fail("LEDGER_SCALAR_INVALID", f"{field} has an unterminated quote")
        value = value[1:-1].replace("''", "'")
    elif value.startswith('"'):
        try:
            decoded = json.loads(value)
        except (json.JSONDecodeError, UnicodeError) as exc:
            _fail("LEDGER_SCALAR_INVALID", f"{field} has invalid quoting: {exc}")
        if not isinstance(decoded, str):
            _fail("LEDGER_SCALAR_INVALID", f"{field} must decode to a string")
        value = decoded
    elif value[0] in "[{&*!>|%@`":
        _fail("LEDGER_SCALAR_UNSUPPORTED", f"{field} uses unsupported YAML syntax")
    return _normalize_text(value, field=field)


def _unique_line_value(lines: Sequence[str], prefix: str, *, field: str) -> str:
    matches = [line[len(prefix) :] for line in lines if line.startswith(prefix)]
    if len(matches) != 1:
        _fail(
            "LEDGER_FIELD_CARDINALITY",
            f"{field} must occur exactly once at its required indentation; found {len(matches)}",
        )
    return matches[0]


def _parse_string_list(block: Sequence[str], field: str) -> list[str]:
    prefixes = (f"  {field}:",)
    indexes = [index for index, line in enumerate(block) if line.startswith(prefixes)]
    if len(indexes) != 1:
        _fail(
            "LEDGER_FIELD_CARDINALITY",
            f"task {field} must occur exactly once; found {len(indexes)}",
        )
    index = indexes[0]
    suffix = block[index][len(prefixes[0]) :].strip()
    if suffix:
        if suffix == "[]":
            return []
        _fail("LEDGER_LIST_INVALID", f"task {field} must be a block list or []")
    values: list[str] = []
    cursor = index + 1
    while cursor < len(block) and block[cursor].startswith("  - "):
        values.append(
            _parse_yaml_scalar(block[cursor][4:], field=f"task {field}[{len(values)}]")
        )
        cursor += 1
    if not values:
        _fail("LEDGER_LIST_INVALID", f"task {field} is an empty implicit block list")
    return values


def _normalize_scope(value: str) -> str:
    if value == "repository metadata":
        return "@repository-metadata"
    if (
        value.startswith(("/", "-"))
        or "\\" in value
        or "\x00" in value
        or "//" in value
        or any(ord(char) < 0x20 for char in value)
    ):
        _fail("UNSAFE_PATH_SCOPE", f"unsafe task path scope: {value!r}")
    parts = PurePosixPath(value.rstrip("/")).parts
    if not parts or any(part in {"", ".", ".."} for part in parts):
        _fail("UNSAFE_PATH_SCOPE", f"unsafe task path scope: {value!r}")
    if any("[" in part or "]" in part or "?" in part for part in parts):
        _fail("UNSAFE_PATH_SCOPE", f"unsupported task path glob: {value!r}")
    return value


def validate_normalized_task_graph(phases: Any, tasks: Any) -> None:
    if not isinstance(tasks, list):
        _fail("TASKS_TYPE", "normalized tasks must be a list")
    if not isinstance(phases, list):
        _fail("PHASES_TYPE", "normalized phases must be a list")
    if len(tasks) != EXPECTED_TASK_COUNT:
        _fail("TASK_COUNT", f"expected {EXPECTED_TASK_COUNT} tasks, found {len(tasks)}")
    if len(phases) != EXPECTED_PHASE_COUNT:
        _fail(
            "PHASE_COUNT",
            f"expected {EXPECTED_PHASE_COUNT} phases, found {len(phases)}",
        )

    expected_task_ids = [f"T{index:03d}" for index in range(EXPECTED_TASK_COUNT)]
    actual_task_ids: list[str] = []
    seen_task_ids: set[str] = set()
    phase_to_tasks: dict[str, list[str]] = {}
    phase_titles: dict[str, str] = {}
    phase_waves: dict[str, int] = {}
    required_task_keys = {
        "id",
        "phase_id",
        "phase_title",
        "title",
        "priority",
        "dependencies",
        "execution_wave",
        "subagent_lane",
        "mandatory_path_scopes",
        "source_current_head",
        "review_status",
    }
    for position, task in enumerate(tasks):
        if not isinstance(task, dict) or set(task) != required_task_keys:
            _fail("TASK_SCHEMA", f"task at position {position} has the wrong fields")
        task_id = task["id"]
        if not isinstance(task_id, str) or _TASK_ID_RE.fullmatch(task_id) is None:
            _fail("TASK_ID", f"invalid task id at position {position}: {task_id!r}")
        if task_id in seen_task_ids:
            _fail("TASK_DUPLICATE", f"duplicate task id: {task_id}")
        seen_task_ids.add(task_id)
        actual_task_ids.append(task_id)
        if task_id != expected_task_ids[position]:
            _fail(
                "TASK_GAP_OR_ORDER",
                f"expected {expected_task_ids[position]}, found {task_id}",
            )

        phase_id = task["phase_id"]
        if not isinstance(phase_id, str) or _PHASE_ID_RE.fullmatch(phase_id) is None:
            _fail("PHASE_ID", f"invalid phase id for {task_id}: {phase_id!r}")
        expected_phase = f"P{position // 15:02d}"
        if phase_id != expected_phase:
            _fail(
                "TASK_PHASE",
                f"{task_id} must belong to {expected_phase}, not {phase_id}",
            )
        phase_title = task["phase_title"]
        title = task["title"]
        priority = task["priority"]
        if not all(
            isinstance(item, str) and item for item in (phase_title, title, priority)
        ):
            _fail("TASK_TEXT", f"{task_id} has an invalid title or priority")
        if task["source_current_head"] != FROZEN_COMMIT:
            _fail("TASK_HEAD", f"{task_id} is not bound to the frozen head")
        if task["review_status"] != "open_imported_instruction_not_review_completion":
            _fail("FALSE_TASK_REVIEW", f"{task_id} has an impermissible review status")

        wave = task["execution_wave"]
        lane = task["subagent_lane"]
        if type(wave) is not int or wave != position // 15:
            _fail("TASK_WAVE", f"{task_id} has invalid execution wave {wave!r}")
        if type(lane) is not int or lane not in {1, 2, 3}:
            _fail("TASK_LANE", f"{task_id} has invalid subagent lane {lane!r}")

        dependencies = task["dependencies"]
        if not isinstance(dependencies, list) or not all(
            isinstance(item, str) and _TASK_ID_RE.fullmatch(item)
            for item in dependencies
        ):
            _fail("TASK_DEPENDENCIES", f"{task_id} has invalid dependencies")
        if len(dependencies) != len(set(dependencies)):
            _fail("TASK_DEPENDENCY_DUPLICATE", f"{task_id} repeats a dependency")
        if dependencies != sorted(dependencies, key=lambda value: int(value[1:])):
            _fail("TASK_DEPENDENCY_ORDER", f"{task_id} dependencies are not normalized")
        if any(
            dependency not in seen_task_ids - {task_id} for dependency in dependencies
        ):
            _fail(
                "TASK_DEPENDENCY_GRAPH",
                f"{task_id} has a missing or forward dependency",
            )

        scopes = task["mandatory_path_scopes"]
        if not isinstance(scopes, list) or not scopes:
            _fail("TASK_PATH_SCOPES", f"{task_id} must have at least one path scope")
        if not all(isinstance(scope, str) for scope in scopes):
            _fail("TASK_PATH_SCOPES", f"{task_id} has a non-string path scope")
        normalized_scopes = sorted({_normalize_scope(scope) for scope in scopes})
        if scopes != normalized_scopes:
            _fail("TASK_PATH_SCOPE_ORDER", f"{task_id} path scopes are not normalized")

        previous_title = phase_titles.setdefault(phase_id, phase_title)
        if previous_title != phase_title:
            _fail("PHASE_TITLE_DRIFT", f"{phase_id} has inconsistent titles")
        previous_wave = phase_waves.setdefault(phase_id, wave)
        if previous_wave != wave:
            _fail("PHASE_WAVE_DRIFT", f"{phase_id} has inconsistent waves")
        phase_to_tasks.setdefault(phase_id, []).append(task_id)

    if actual_task_ids != expected_task_ids:
        _fail("TASK_GRAPH", "task identifiers are not the complete contiguous range")

    expected_phase_ids = [f"P{index:02d}" for index in range(EXPECTED_PHASE_COUNT)]
    actual_phase_ids: list[str] = []
    required_phase_keys = {"id", "title", "execution_wave", "task_ids", "review_status"}
    for index, phase in enumerate(phases):
        if not isinstance(phase, dict) or set(phase) != required_phase_keys:
            _fail("PHASE_SCHEMA", f"phase at position {index} has the wrong fields")
        phase_id = phase["id"]
        actual_phase_ids.append(phase_id)
        if phase_id != expected_phase_ids[index]:
            _fail(
                "PHASE_GAP_OR_ORDER",
                f"expected {expected_phase_ids[index]}, found {phase_id}",
            )
        if phase["title"] != phase_titles.get(phase_id):
            _fail(
                "PHASE_TITLE_DRIFT", f"phase summary for {phase_id} has the wrong title"
            )
        if phase["execution_wave"] != phase_waves.get(phase_id):
            _fail(
                "PHASE_WAVE_DRIFT", f"phase summary for {phase_id} has the wrong wave"
            )
        if phase["task_ids"] != phase_to_tasks.get(phase_id):
            _fail(
                "PHASE_TASK_DRIFT", f"phase summary for {phase_id} has the wrong tasks"
            )
        if phase["review_status"] != "open_imported_instruction_not_review_completion":
            _fail(
                "FALSE_PHASE_REVIEW", f"{phase_id} has an impermissible review status"
            )
    if actual_phase_ids != expected_phase_ids:
        _fail("PHASE_GRAPH", "phase identifiers are not the complete contiguous range")


def parse_master_ledger_bytes(
    raw: bytes,
) -> tuple[dict[str, str], list[dict[str, Any]]]:
    if len(raw) > MAX_MASTER_LEDGER_BYTES:
        _fail("LEDGER_TOO_LARGE", "master ledger exceeds the 16 MiB intake limit")
    if b"\x00" in raw:
        _fail("LEDGER_NUL", "master ledger contains a NUL byte")
    try:
        text = raw.decode("utf-8", errors="strict")
    except UnicodeDecodeError as exc:
        _fail("LEDGER_UTF8", f"master ledger is not UTF-8: {exc}")
    lines = text.splitlines()
    if lines.count("tasks:") != 1:
        _fail(
            "LEDGER_TASK_SECTION",
            "master ledger must contain exactly one top-level tasks section",
        )
    task_start = lines.index("tasks:") + 1
    metadata: dict[str, str] = {}
    for field in _TOP_FIELDS:
        raw_value = _unique_line_value(lines[:task_start], f"{field}:", field=field)
        metadata[field] = _parse_yaml_scalar(raw_value, field=field)

    starts = [
        index
        for index in range(task_start, len(lines))
        if lines[index].startswith("- id: T")
    ]
    if not starts:
        _fail("LEDGER_TASKS_MISSING", "master ledger contains no T-task records")
    tasks: list[dict[str, Any]] = []
    for task_index, start in enumerate(starts):
        end = starts[task_index + 1] if task_index + 1 < len(starts) else len(lines)
        block = lines[start:end]
        scalars: dict[str, str] = {}
        scalars["id"] = _parse_yaml_scalar(block[0][len("- id:") :], field="task id")
        for field in _TASK_SCALAR_FIELDS[1:]:
            value = _unique_line_value(
                block, f"  {field}:", field=f"{scalars['id']}.{field}"
            )
            scalars[field] = _parse_yaml_scalar(value, field=f"{scalars['id']}.{field}")
        try:
            execution_wave = int(scalars["execution_wave"], 10)
            subagent_lane = int(scalars["subagent_lane"], 10)
        except ValueError:
            _fail(
                "LEDGER_INTEGER_INVALID",
                f"{scalars['id']} has a non-integer wave or lane",
            )
        dependencies_raw = _parse_string_list(block, "dependencies")
        if not all(_TASK_ID_RE.fullmatch(value) for value in dependencies_raw):
            _fail(
                "LEDGER_DEPENDENCY_INVALID",
                f"{scalars['id']} has an invalid dependency",
            )
        dependencies = sorted(dependencies_raw, key=lambda value: int(value[1:]))
        scopes = sorted(
            {
                _normalize_scope(value)
                for value in _parse_string_list(block, "mandatory_path_scope")
            }
        )
        tasks.append(
            {
                "id": scalars["id"].upper(),
                "phase_id": scalars["phase_id"].upper(),
                "phase_title": _normalize_text(
                    scalars["phase_title"], field="phase_title"
                ),
                "title": _normalize_text(scalars["title"], field="title"),
                "priority": scalars["priority"],
                "dependencies": dependencies,
                "execution_wave": execution_wave,
                "subagent_lane": subagent_lane,
                "mandatory_path_scopes": scopes,
                "source_current_head": scalars["current_head"],
                "review_status": "open_imported_instruction_not_review_completion",
            }
        )
    return metadata, tasks


def _phases_from_tasks(tasks: Sequence[Mapping[str, Any]]) -> list[dict[str, Any]]:
    phases: list[dict[str, Any]] = []
    for phase_index in range(EXPECTED_PHASE_COUNT):
        phase_id = f"P{phase_index:02d}"
        members = [task for task in tasks if task["phase_id"] == phase_id]
        if not members:
            _fail("PHASE_TASKS_MISSING", f"no tasks found for {phase_id}")
        phases.append(
            {
                "id": phase_id,
                "title": members[0]["phase_title"],
                "execution_wave": phase_index,
                "task_ids": [task["id"] for task in members],
                "review_status": "open_imported_instruction_not_review_completion",
            }
        )
    return phases


def build_task_ledger(raw: bytes) -> dict[str, Any]:
    source_sha256 = sha256_bytes(raw)
    if source_sha256 != EXPECTED_HANDOFF_SHA256:
        _fail(
            "HANDOFF_HASH",
            f"master ledger SHA-256 {source_sha256} does not match the reviewed source",
        )
    if len(raw) != EXPECTED_HANDOFF_BYTES:
        _fail(
            "HANDOFF_BYTES",
            f"master ledger byte count {len(raw)} does not match reviewed source",
        )
    metadata, tasks = parse_master_ledger_bytes(raw)
    expected_metadata = {
        "project": PROJECT,
        "repository": REPOSITORY,
        "frozen_commit": FROZEN_COMMIT,
        "release_target": NOMINAL_HANDOFF_RELEASE,
        "phase_count": str(EXPECTED_PHASE_COUNT),
        "task_count": str(EXPECTED_TASK_COUNT),
    }
    for field, expected in expected_metadata.items():
        if metadata[field] != expected:
            _fail("HANDOFF_METADATA", f"master ledger {field} must be {expected!r}")
    phases = _phases_from_tasks(tasks)
    validate_normalized_task_graph(phases, tasks)
    graph = {"phases": phases, "tasks": tasks}
    graph_sha256 = semantic_sha256(graph)
    if graph_sha256 != EXPECTED_TASK_GRAPH_SHA256:
        _fail(
            "TASK_GRAPH_HASH", "normalized task graph differs from the reviewed graph"
        )
    return {
        "schema_version": SCHEMA_VERSION,
        "record_type": "normalized_master_task_ledger",
        "project": PROJECT,
        "source": {
            "operator_must_supply_explicit_cli_path": True,
            "expected_filename": "19_MASTER_TASK_LEDGER.yaml",
            "sha256": source_sha256,
            "bytes": len(raw),
            "handoff_schema": metadata["handoff_schema"],
            "declared_status": metadata["status"],
            "nominal_release_target": metadata["release_target"],
            "frozen_commit": metadata["frozen_commit"],
        },
        "normalization": {
            "unicode": "NFC",
            "text_whitespace": "collapsed",
            "identifiers": "uppercase_fixed_width",
            "dependencies": "unique_numeric_sort",
            "path_scopes": "unique_lexicographic_sort; repository metadata becomes @repository-metadata",
            "source_fields_retained": [
                "id",
                "phase_id",
                "phase_title",
                "title",
                "priority",
                "dependencies",
                "execution_wave",
                "subagent_lane",
                "mandatory_path_scope",
                "current_head",
            ],
        },
        "phase_count": len(phases),
        "task_count": len(tasks),
        "task_graph_sha256": graph_sha256,
        "review": {
            "status": "normalized_intake_only_not_substantively_reviewed",
            "all_tasks_closed": False,
            "closed_task_ids": [],
            "human_review_complete": False,
            "independent_review_complete": False,
            "release_gate_passed": False,
        },
        "phases": phases,
        "tasks": tasks,
    }


def _safe_git_path(path: str) -> None:
    if (
        not path
        or path.startswith(("/", "-"))
        or "\\" in path
        or "\x00" in path
        or "//" in path
        or any(ord(char) < 0x20 for char in path)
    ):
        _fail("UNSAFE_GIT_PATH", f"unsafe Git path: {path!r}")
    parts = PurePosixPath(path).parts
    if any(part in {"", ".", ".."} for part in parts):
        _fail("UNSAFE_GIT_PATH", f"unsafe Git path: {path!r}")


def _category(path: str, object_type: str) -> str:
    lower = path.lower()
    name = PurePosixPath(path).name.lower()
    suffix = PurePosixPath(path).suffix.lower()
    if object_type == "commit":
        return "submodule"
    if path in _GENERATED_PATHS:
        return "generated_artifact"
    if path.startswith(".github/workflows/"):
        return "automation"
    if path.startswith("tests/"):
        return "test"
    if "/fixtures/" in path or path.startswith("crates/pid-sim/fixtures/"):
        return "fixture"
    if name.startswith("license") or "notice" in name:
        return "legal"
    if suffix == ".rs":
        return "rust_source"
    if suffix == ".py":
        return "python_source"
    if suffix in {".md", ".rst"}:
        return "documentation"
    if suffix in {".svg", ".png", ".jpg", ".jpeg", ".gif"}:
        return "asset"
    if suffix in {".json", ".jsonl", ".csv"}:
        return "governance_or_data"
    if suffix in {".toml", ".yaml", ".yml", ".lock", ".nix"} or lower.startswith(
        ".git"
    ):
        return "configuration"
    if suffix == ".bin":
        return "binary_fixture"
    return "other"


def _public_surface(path: str) -> bool:
    return (
        path in _ROOT_PUBLIC_PATHS
        or path.startswith(("docs/", "protocols/", "scripts/"))
        or path.endswith("Cargo.toml")
        or "/src/bin/" in path
        or path.endswith("/src/lib.rs")
        or (
            path.startswith("experiments/")
            and PurePosixPath(path).name in {"README.md", "__init__.py", "__main__.py"}
        )
    )


def _security_sensitive(path: str) -> bool:
    lower = path.lower()
    tokens = (
        "audit",
        "bridge",
        "holdout",
        "manifest",
        "observer",
        "replay",
        "runlog",
        "security",
        "verify",
    )
    return (
        path in _SECURITY_ROOT_PATHS
        or path.startswith(
            (".github/", "crates/ncp-observer/", "experiments/safe_adapter/")
        )
        or any(token in lower for token in tokens)
    )


def _science_sensitive(path: str) -> bool:
    lower = path.lower()
    tokens = ("attribution", "h1_", "h2_", "offline", "pid", "power", "preregistration")
    return (
        path in _SCIENCE_ROOT_PATHS
        or path.startswith(("experiments/", "protocols/", "docs/power-gate/"))
        or any(token in lower for token in tokens)
    )


def _gitmodules_url(repo: Path, *, timeout_seconds: float) -> str:
    raw = _run_git(
        repo,
        ["show", f"{FROZEN_COMMIT}:.gitmodules"],
        max_bytes=64 * 1024,
        timeout_seconds=timeout_seconds,
    )
    try:
        text = raw.decode("utf-8", errors="strict")
    except UnicodeDecodeError as exc:
        _fail("GITMODULES_UTF8", f"frozen .gitmodules is not UTF-8: {exc}")
    matches = re.findall(r"(?m)^\s*url\s*=\s*(\S.*?)\s*$", text)
    if len(matches) != 1:
        _fail("GITMODULES_URL", "frozen .gitmodules must contain one submodule URL")
    return matches[0]


def build_inventory(repo: Path) -> dict[str, Any]:
    repo = resolve_repo(repo)
    git_deadline = time.monotonic() + INVENTORY_GIT_DEADLINE_SECONDS

    def inventory_git(args: Sequence[str], *, max_bytes: int) -> bytes:
        remaining = git_deadline - time.monotonic()
        if remaining <= 0:
            _fail(
                "INVENTORY_TIMEOUT",
                "review inventory Git subprocesses exceeded their aggregate deadline",
            )
        return _run_git(
            repo,
            args,
            max_bytes=max_bytes,
            timeout_seconds=min(GIT_COMMAND_TIMEOUT_SECONDS, remaining),
        )

    tree = inventory_git(["rev-parse", f"{FROZEN_COMMIT}^{{tree}}"], max_bytes=1024)
    try:
        tree_oid = tree.decode("ascii", errors="strict").strip()
    except UnicodeDecodeError as exc:
        _fail("TREE_OID", f"cannot decode frozen tree identity: {exc}")
    if tree_oid != FROZEN_TREE:
        _fail("FROZEN_TREE", f"frozen commit resolves to unexpected tree {tree_oid}")
    raw_tree = inventory_git(
        ["ls-tree", "-r", "-z", FROZEN_COMMIT], max_bytes=MAX_TREE_LISTING_BYTES
    )
    raw_entries = [] if not raw_tree else raw_tree.rstrip(b"\x00").split(b"\x00")
    entries: list[dict[str, Any]] = []
    submodules: list[dict[str, Any]] = []
    paths_seen: set[str] = set()
    path_bytes = 0
    aggregate_bytes = 0
    submodule_url = _gitmodules_url(
        repo,
        timeout_seconds=min(
            GIT_COMMAND_TIMEOUT_SECONDS, max(0.001, git_deadline - time.monotonic())
        ),
    )
    for raw_entry in raw_entries:
        if len(entries) >= EXPECTED_TRACKED_COUNT:
            _fail("TRACKED_COUNT", "frozen tree exceeds the reviewed entry count")
        try:
            metadata_raw, path_raw = raw_entry.split(b"\t", 1)
            mode, object_type, object_id = metadata_raw.decode(
                "ascii", errors="strict"
            ).split(" ")
            path = path_raw.decode("utf-8", errors="strict")
        except (ValueError, UnicodeDecodeError) as exc:
            _fail("TREE_ENTRY", f"cannot parse frozen tree entry: {exc}")
        _safe_git_path(path)
        path_bytes += len(path_raw)
        if path_bytes > MAX_FROZEN_PATH_BYTES:
            _fail("TREE_PATH_BUDGET", "frozen tree paths exceed the reviewed budget")
        if path in paths_seen:
            _fail("TREE_DUPLICATE_PATH", f"frozen tree repeats path {path}")
        paths_seen.add(path)
        if not _OID_RE.fullmatch(object_id):
            _fail("TREE_OBJECT_ID", f"invalid Git object id for {path}")
        if mode not in {"100644", "100755", "120000", "160000"}:
            _fail("TREE_MODE", f"unsupported Git mode {mode} for {path}")
        if object_type not in {"blob", "commit"}:
            _fail(
                "TREE_OBJECT_TYPE", f"unsupported object type {object_type} for {path}"
            )
        if object_type == "commit":
            if mode != "160000" or path != PID_RS_PATH or object_id != PID_RS_COMMIT:
                _fail("SUBMODULE_IDENTITY", f"unexpected gitlink {path}@{object_id}")
            content_sha256: str | None = None
            byte_count: int | None = None
            line_count: int | None = None
            is_binary: bool | None = None
            git_blob_id: str | None = None
            gitlink_commit: str | None = object_id
            submodules.append(
                {
                    "path": path,
                    "gitlink_commit": object_id,
                    "mode": mode,
                    "url_at_frozen_commit": submodule_url,
                    "review_status": "identity_recorded_content_not_reviewed_here",
                }
            )
        else:
            content = inventory_git(
                ["cat-file", "blob", object_id], max_bytes=MAX_ARTIFACT_BYTES
            )
            aggregate_bytes += len(content)
            if aggregate_bytes > MAX_FROZEN_AGGREGATE_BYTES:
                _fail(
                    "TREE_CONTENT_BUDGET",
                    "frozen blob content exceeds the reviewed aggregate budget",
                )
            content_sha256 = sha256_bytes(content)
            byte_count = len(content)
            line_count = content.count(b"\n") + int(
                bool(content) and not content.endswith(b"\n")
            )
            is_binary = b"\x00" in content
            git_blob_id = object_id
            gitlink_commit = None
        entries.append(
            {
                "path": path,
                "mode": mode,
                "object_type": object_type,
                "git_object_id": object_id,
                "git_blob_id": git_blob_id,
                "gitlink_commit": gitlink_commit,
                "content_sha256": content_sha256,
                "bytes": byte_count,
                "line_count": line_count,
                "is_binary": is_binary,
                "is_symlink": mode == "120000",
                "is_executable": mode == "100755",
                "category": _category(path, object_type),
                "generated": path in _GENERATED_PATHS,
                "public_surface": _public_surface(path),
                "security_sensitive": _security_sensitive(path),
                "science_sensitive": _science_sensitive(path),
                "review_status": "inventory_only_unreviewed",
                "human_reviewed": False,
                "independent_reviewed": False,
            }
        )
    if len(entries) != EXPECTED_TRACKED_COUNT:
        _fail(
            "TRACKED_COUNT",
            f"expected {EXPECTED_TRACKED_COUNT} tree entries, found {len(entries)}",
        )
    if len(submodules) != 1:
        _fail("SUBMODULE_COUNT", f"expected one gitlink, found {len(submodules)}")
    if entries != sorted(entries, key=lambda entry: entry["path"].encode("utf-8")):
        _fail("TREE_ORDER", "Git tree entries are not in canonical bytewise path order")
    blob_count = sum(entry["object_type"] == "blob" for entry in entries)
    flag_counts = {
        flag: sum(bool(entry[flag]) for entry in entries)
        for flag in (
            "generated",
            "public_surface",
            "security_sensitive",
            "science_sensitive",
            "is_symlink",
            "is_executable",
        )
    }
    return {
        "schema_version": SCHEMA_VERSION,
        "record_type": "frozen_tracked_file_inventory",
        "project": PROJECT,
        "source": {
            "commit": FROZEN_COMMIT,
            "tree": FROZEN_TREE,
            "inventory_source": "git ls-tree -r -z plus git cat-file blob",
            "working_tree_used_for_content": False,
        },
        "line_count_semantics": (
            "LF byte count plus one for a nonempty blob without a terminal LF; "
            "reported for binary blobs as byte framing, not source lines"
        ),
        "classification_boundary": {
            "purpose": "deterministic review prioritization only; flags do not establish review or risk",
            "generated_exact_paths": sorted(_GENERATED_PATHS),
            "public_surface_rule": (
                "reviewed root allowlist, docs/protocols/scripts prefixes, Cargo manifests, "
                "Rust lib/bin entry points, and experiment package/CLI entry points"
            ),
            "security_sensitive_rule": (
                "reviewed root allowlist, workflow/NCP/SAFE prefixes, and audit/bridge/holdout/"
                "manifest/observer/replay/runlog/security/verify path tokens"
            ),
            "science_sensitive_rule": (
                "reviewed root allowlist, experiments/protocols/power-gate prefixes, and "
                "attribution/H1/H2/offline/PID/power/preregistration path tokens"
            ),
        },
        "tracked_entry_count": len(entries),
        "tracked_blob_count": blob_count,
        "gitlink_count": len(submodules),
        "flag_counts": flag_counts,
        "inventory_entries_sha256": semantic_sha256(entries),
        "review": {
            "status": "inventory_only_unreviewed",
            "human_review_complete": False,
            "independent_review_complete": False,
            "reviewed_file_count": 0,
            "claim": "No file-review completion is asserted by this baseline inventory.",
        },
        "submodules": submodules,
        "entries": entries,
    }


def build_intake(
    task_ledger: Mapping[str, Any], inventory: Mapping[str, Any]
) -> dict[str, Any]:
    return {
        "schema_version": SCHEMA_VERSION,
        "record_type": "release_review_intake",
        "project": PROJECT,
        "repository": REPOSITORY,
        "author": {
            "name": AUTHOR,
            "basis": "explicit_user_instruction",
            "scope": "release authorship metadata; not an assertion of review completion",
        },
        "release": {
            "nominal_handoff_target": NOMINAL_HANDOFF_RELEASE,
            "requested_release": RELEASE_VERSION,
            "override": "user_requested_0.9.0_review_release_before_1.0",
            "status": "review_intake_open_not_release_ready",
            "doi": None,
            "doi_status": "not_assigned_user_will_add_later",
            "zenodo_record": None,
            "zenodo_status": "not_created_user_will_add_later",
            "published": False,
        },
        "frozen_baseline": {
            "commit": FROZEN_COMMIT,
            "tree": FROZEN_TREE,
            "pid_rs_gitlink_commit": PID_RS_COMMIT,
            "tracked_entry_count": inventory["tracked_entry_count"],
        },
        "handoff_binding": {
            "master_ledger_sha256": task_ledger["source"]["sha256"],
            "task_graph_sha256": task_ledger["task_graph_sha256"],
            "task_count": task_ledger["task_count"],
            "phase_count": task_ledger["phase_count"],
        },
        "review": {
            "status": "intake_and_inventory_only",
            "all_tasks_closed": False,
            "closed_task_ids": [],
            "human_review_complete": False,
            "independent_review_complete": False,
            "release_ready": False,
            "scientific_claims_established": False,
            "boundary": (
                "Normalization and inventory are mechanical evidence only; every task and "
                "substantive file review remains open until separately evidenced."
            ),
        },
    }


def build_manifest(artifacts: Mapping[str, bytes]) -> dict[str, Any]:
    if set(artifacts) != set(ARTIFACT_NAMES):
        _fail(
            "MANIFEST_INPUT", "manifest inputs do not match the required artifact set"
        )
    return {
        "schema_version": SCHEMA_VERSION,
        "record_type": "release_review_artifact_manifest",
        "project": PROJECT,
        "release_version": RELEASE_VERSION,
        "frozen_commit": FROZEN_COMMIT,
        "artifacts": [
            {
                "path": name,
                "sha256": sha256_bytes(artifacts[name]),
                "bytes": len(artifacts[name]),
            }
            for name in sorted(ARTIFACT_NAMES)
        ],
        "review_status": "integrity_manifest_only_not_review_completion",
    }


def build_artifacts(repo: Path, master_ledger_raw: bytes) -> dict[str, bytes]:
    task_ledger = build_task_ledger(master_ledger_raw)
    inventory = build_inventory(repo)
    intake = build_intake(task_ledger, inventory)
    documents = {
        "intake.json": pretty_json_bytes(intake),
        "master_task_ledger.normalized.json": pretty_json_bytes(task_ledger),
        "tracked_file_inventory.baseline.json": pretty_json_bytes(inventory),
    }
    documents[MANIFEST_NAME] = pretty_json_bytes(build_manifest(documents))
    return documents


def _safe_output_directory(path: Path) -> Path:
    if path.exists():
        if path.is_symlink() or not path.is_dir():
            _fail(
                "OUTPUT_DIRECTORY_UNSAFE",
                f"output path is not a real directory: {path}",
            )
    else:
        try:
            path.mkdir(parents=True, exist_ok=False)
        except OSError as exc:
            _fail(
                "OUTPUT_DIRECTORY_CREATE",
                f"cannot create output directory {path}: {exc}",
            )
    return path


def _directory_names(path: Path) -> set[str]:
    names: set[str] = set()
    expected_count = len(ARTIFACT_NAMES) + 1
    try:
        for entry in path.iterdir():
            if len(names) >= expected_count:
                _fail("ARTIFACT_SET", "review output contains extra entries")
            if entry.is_symlink() or not entry.is_file():
                _fail("OUTPUT_ENTRY", f"output contains a non-regular entry: {entry}")
            names.add(entry.name)
    except ReleaseReviewError:
        raise
    except OSError as exc:
        _fail("OUTPUT_DIRECTORY_READ", f"cannot list output directory {path}: {exc}")
    return names


def _require_exact_output_set(output_dir: Path, *, allow_empty: bool) -> None:
    expected_names = set(ARTIFACT_NAMES) | {MANIFEST_NAME}
    actual_names = _directory_names(output_dir)
    if allow_empty and not actual_names:
        return
    if actual_names != expected_names:
        _fail(
            "ARTIFACT_SET",
            "review artifact set differs; "
            f"missing={sorted(expected_names - actual_names)}, "
            f"extra={sorted(actual_names - expected_names)}",
        )


def _fsync_directory(path: Path) -> None:
    if os.name != "posix":
        return
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(path, flags)
        try:
            os.fsync(descriptor)
        finally:
            os.close(descriptor)
    except OSError as exc:
        _fail("OUTPUT_WRITE", f"cannot durably sync review directory {path}: {exc}")


def write_artifacts(output_dir: Path, artifacts: Mapping[str, bytes]) -> None:
    output_dir = _safe_output_directory(output_dir)
    expected_names = set(ARTIFACT_NAMES) | {MANIFEST_NAME}
    if set(artifacts) != expected_names:
        _fail("ARTIFACT_SET", "generated artifact set is incomplete or contains extras")
    _require_exact_output_set(output_dir, allow_empty=True)
    write_order = sorted(name for name in artifacts if name != MANIFEST_NAME)
    write_order.append(MANIFEST_NAME)
    for name in write_order:
        if name == MANIFEST_NAME:
            # Make every payload rename durable before publishing the manifest that binds it.
            _fsync_directory(output_dir)
        _safe_git_path(name)
        destination = output_dir / name
        if destination.is_symlink():
            _fail("OUTPUT_SYMLINK", f"refusing to replace symlink {destination}")
        temporary = output_dir / f".{name}.tmp.{os.getpid()}"
        if temporary.exists() or temporary.is_symlink():
            _fail("OUTPUT_TEMP_EXISTS", f"temporary output already exists: {temporary}")
        try:
            with temporary.open("xb") as handle:
                handle.write(artifacts[name])
                handle.flush()
                os.fsync(handle.fileno())
            os.replace(temporary, destination)
        except OSError as exc:
            try:
                temporary.unlink(missing_ok=True)
            except OSError:
                pass
            _fail("OUTPUT_WRITE", f"cannot write {destination}: {exc}")
    _fsync_directory(output_dir)
    _require_exact_output_set(output_dir, allow_empty=False)


def _read_explicit_master_ledger(path: Path) -> bytes:
    if path.name != "19_MASTER_TASK_LEDGER.yaml":
        _fail(
            "HANDOFF_FILENAME",
            "explicit master ledger must be named 19_MASTER_TASK_LEDGER.yaml",
        )
    return _read_bounded_regular(
        path,
        max_bytes=MAX_MASTER_LEDGER_BYTES,
        path_code="HANDOFF_PATH",
        read_code="HANDOFF_READ",
        too_large_code="LEDGER_TOO_LARGE",
        description="master ledger",
    )


def check_artifacts(output_dir: Path, artifacts: Mapping[str, bytes]) -> None:
    if output_dir.is_symlink() or not output_dir.is_dir():
        _fail(
            "OUTPUT_DIRECTORY_UNSAFE",
            f"output path is not a real directory: {output_dir}",
        )
    expected_names = set(ARTIFACT_NAMES) | {MANIFEST_NAME}
    if set(artifacts) != expected_names:
        _fail("ARTIFACT_SET", "generated artifact set is incomplete or contains extras")
    _require_exact_output_set(output_dir, allow_empty=False)
    for name, expected in artifacts.items():
        path = output_dir / name
        actual = _read_bounded_regular(
            path,
            max_bytes=MAX_ARTIFACT_BYTES,
            path_code="CHECK_ARTIFACT_MISSING",
            read_code="CHECK_ARTIFACT_READ",
            too_large_code="CHECK_ARTIFACT_TOO_LARGE",
            description="generated review artifact",
        )
        if actual != expected:
            _fail("CHECK_ARTIFACT_DRIFT", f"generated artifact is stale: {path}")


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--master-ledger",
        required=True,
        type=Path,
        help="explicit path to the reviewed external 19_MASTER_TASK_LEDGER.yaml",
    )
    parser.add_argument(
        "--repo", type=Path, default=Path("."), help="Prisoma Git repository"
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("release/0.9.0/review"),
        help="destination for deterministic review artifacts",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="compare generated bytes with output-dir without writing",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        repo = resolve_repo(args.repo)
        raw = _read_explicit_master_ledger(args.master_ledger)
        artifacts = build_artifacts(repo, raw)
        if args.check:
            check_artifacts(args.output_dir, artifacts)
        else:
            write_artifacts(args.output_dir, artifacts)
    except ReleaseReviewError as exc:
        print(f"release review generation failed [{exc.code}]: {exc}", file=sys.stderr)
        return 3
    mode = "current" if args.check else "generated"
    print(
        f"release review {mode}: {len(artifacts)} artifacts; "
        f"release={RELEASE_VERSION}; frozen_head={FROZEN_COMMIT}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
