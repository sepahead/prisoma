#!/usr/bin/env python3
"""Generate the immutable Prisoma 0.9 handoff-requirements baseline.

The handoff directory is always supplied explicitly.  This generator copies the exact
master ledger, binds every checksummed handoff payload, and derives an all-open task/lens
disposition baseline.  It records imported requirements; it never asserts review,
implementation, scientific validity, or release readiness.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
import sys
import unicodedata
from collections.abc import Mapping, Sequence
from pathlib import Path, PurePosixPath
from typing import Any


SCHEMA_VERSION = "prisoma.release-requirements/0.1.0"
PROJECT = "prisoma"
REPOSITORY = "https://github.com/sepahead/prisoma"
AUTHOR = "Sepehr Mahmoudian"
RELEASE_VERSION = "0.9.0"
NOMINAL_RELEASE_VERSION = "1.0.0"
EXPECTED_DIRECTORY_NAME = "PRISOMA_V1_0_CURRENT_HEAD_MAX_EFFORT_STANDALONE_HANDOFF"
LEDGER_NAME = "19_MASTER_TASK_LEDGER.yaml"
CHECKSUM_NAME = "29_SHA256SUMS.txt"
EXPECTED_LEDGER_SHA256 = (
    "384f5540dcdb4709b8f9add57e355761c6e076a1c4b22e26e42482bd0c0c4f29"
)
EXPECTED_LEDGER_BYTES = 2_281_617
EXPECTED_CHECKSUM_SHA256 = (
    "78f18f0f3cf107b495ba6cb9dec2fdcc2e9484b6d0d79019f32ed97ef95afbc4"
)
EXPECTED_CHECKSUM_BYTES = 4_200
EXPECTED_PAYLOAD_COUNT = 42
EXPECTED_INCLUDED_COUNT = 43
EXPECTED_TASK_COUNT = 240
EXPECTED_PHASE_COUNT = 16
EXPECTED_LENS_COUNT = 20
EXPECTED_LENS_DISPOSITION_COUNT = EXPECTED_TASK_COUNT * EXPECTED_LENS_COUNT
EXPECTED_FROZEN_COMMIT = "0968128062f30da5c04f3f31c23f6ce8e0d95d36"
EXPECTED_PID_RS_COMMIT = "ac4a7803c5a77408f5e9176c60cda71c65c38260"
# Filled after first generation from the canonical included-file records.  A handoff
# replacement requires an explicit source and code review, not an adjacent digest edit.
EXPECTED_PACKAGE_IDENTITY_SHA256: str | None = (
    "05ff0e9c4292f630c003b116a9146155717b14359989d229d3b43fea2e936240"
)
EXPECTED_REQUIREMENTS_SEMANTIC_SHA256 = (
    "a83af23b486cc148fed6f194953cc7509b5f55f13dd7a5086fd5ebdb52672035"
)
PACKAGE_IDENTITY_SEMANTICS = (
    "SHA-256 over canonical JSON containing all 43 complete included-file records "
    "and the two excluded relative paths; excluded file bytes are deliberately unbound"
)

PACKAGE_MANIFEST_NAME = "handoff_package_manifest.json"
DISPOSITIONS_NAME = "task_dispositions.baseline.json"
ARTIFACT_MANIFEST_NAME = "artifact_manifest.json"
ARTIFACT_NAMES = (LEDGER_NAME, PACKAGE_MANIFEST_NAME, DISPOSITIONS_NAME)
EXPECTED_OUTPUT_NAMES = frozenset((*ARTIFACT_NAMES, ARTIFACT_MANIFEST_NAME))
EXCLUDED_PATHS = (".DS_Store", "repo_work/.DS_Store")
MAX_HANDOFF_ENTRIES = 128
MAX_HANDOFF_DEPTH = 16
MAX_HANDOFF_PATH_BYTES = 128 * 1024

_SHA256_RE = re.compile(r"[0-9a-f]{64}\Z")
_TASK_ID_RE = re.compile(r"T[0-9]{3}\Z")
_PHASE_ID_RE = re.compile(r"P[0-9]{2}\Z")
_LENS_ID_RE = re.compile(r"L[0-9]{2}\Z")
_TASK_FIELD_RE = re.compile(r"^  ([a-z][a-z0-9_]*):(.*)$")
_LENS_FIELD_RE = re.compile(r"^      ([a-z][a-z0-9_]*):(.*)$")
_GLOBAL_LENS_FIELD_RE = re.compile(r"^  ([a-z][a-z0-9_]*):(.*)$")

_TOP_FIELDS = (
    "handoff_schema",
    "project",
    "repository",
    "frozen_commit",
    "release_target",
    "status",
    "lead_agents",
    "max_concurrent_subagents",
    "phase_count",
    "task_count",
)
_TASK_FIELDS = frozenset(
    {
        "phase_id",
        "phase_title",
        "title",
        "priority",
        "dependencies",
        "execution_wave",
        "subagent_lane",
        "mandatory_path_scope",
        "current_head",
        "head_mismatch_rule",
        "preconditions",
        "procedure",
        "mandatory_adversarial_questions",
        "required_tests",
        "required_evidence",
        "twenty_lens_review",
        "completion_rule",
    }
)
_TASK_LIST_FIELDS = frozenset(
    {
        "dependencies",
        "mandatory_path_scope",
        "preconditions",
        "procedure",
        "mandatory_adversarial_questions",
        "required_tests",
        "required_evidence",
    }
)
_TASK_INTEGER_FIELDS = frozenset({"execution_wave", "subagent_lane"})
_LENS_FIELDS = frozenset({"name", "question", "finding", "evidence", "status"})


class RequirementsError(ValueError):
    """Controlled, user-facing requirements-package failure."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


def fail(code: str, message: str) -> None:
    raise RequirementsError(code, message)


def sha256_bytes(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        allow_nan=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")


def semantic_sha256(value: Any) -> str:
    return sha256_bytes(canonical_json_bytes(value))


def pretty_json_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, allow_nan=False, indent=2, sort_keys=True)
        + "\n"
    ).encode("utf-8")


def _normalized_text(parts: Sequence[str], *, field: str) -> str:
    value = unicodedata.normalize("NFC", " ".join(" ".join(parts).split()))
    if any(ord(character) < 0x20 for character in value):
        fail("SOURCE_CONTROL_CHARACTER", f"{field} contains a control character")
    return value


def _decode_scalar(parts: Sequence[str], *, field: str) -> str:
    value = _normalized_text(parts, field=field)
    if not value:
        fail("SOURCE_SCALAR_EMPTY", f"{field} is unexpectedly empty")
    if value.startswith("'"):
        if len(value) < 2 or not value.endswith("'"):
            fail("SOURCE_QUOTE", f"{field} has an unterminated single quote")
        return value[1:-1].replace("''", "'")
    if value.startswith('"'):
        try:
            decoded = json.loads(value)
        except json.JSONDecodeError as exc:
            fail("SOURCE_QUOTE", f"{field} has invalid double quoting: {exc}")
        if not isinstance(decoded, str):
            fail("SOURCE_SCALAR_TYPE", f"{field} must decode to text")
        return decoded
    return value


def _decode_optional_scalar(parts: Sequence[str], *, field: str) -> str:
    value = _normalized_text(parts, field=field)
    if value == "''":
        return ""
    return _decode_scalar([value], field=field)


def _leading_spaces(line: str) -> int:
    return len(line) - len(line.lstrip(" "))


def _safe_relative_path(value: str, *, code: str) -> str:
    if (
        not value
        or value.startswith(("/", "-"))
        or "\\" in value
        or "\x00" in value
        or "//" in value
        or any(ord(character) < 0x20 for character in value)
    ):
        fail(code, f"unsafe relative path: {value!r}")
    parts = PurePosixPath(value).parts
    if not parts or any(part in {"", ".", ".."} for part in parts):
        fail(code, f"unsafe relative path: {value!r}")
    return value


def _field_ranges(
    block: Sequence[str],
    pattern: re.Pattern[str],
    *,
    expected: frozenset[str],
    context: str,
) -> dict[str, tuple[int, int, str]]:
    starts: list[tuple[str, int, str]] = []
    for index, line in enumerate(block):
        match = pattern.fullmatch(line)
        if match is not None:
            starts.append((match.group(1), index, match.group(2).strip()))
    names = [name for name, _, _ in starts]
    if len(names) != len(set(names)):
        fail("SOURCE_FIELD_DUPLICATE", f"{context} repeats a field")
    if set(names) != expected:
        fail(
            "SOURCE_FIELD_SET",
            f"{context} fields differ; missing={sorted(expected - set(names))}, "
            f"extra={sorted(set(names) - expected)}",
        )
    result: dict[str, tuple[int, int, str]] = {}
    for position, (name, start, suffix) in enumerate(starts):
        end = starts[position + 1][1] if position + 1 < len(starts) else len(block)
        result[name] = (start, end, suffix)
    return result


def _parse_scalar_range(
    block: Sequence[str], value_range: tuple[int, int, str], *, field: str
) -> str:
    start, end, suffix = value_range
    parts = [suffix]
    for line in block[start + 1 : end]:
        if line.strip():
            parts.append(line.strip())
    return _decode_scalar(parts, field=field)


def _parse_list_range(
    block: Sequence[str], value_range: tuple[int, int, str], *, field: str
) -> list[str]:
    start, end, suffix = value_range
    if suffix:
        if suffix == "[]":
            return []
        fail("SOURCE_LIST", f"{field} must use a block list or []")
    values: list[str] = []
    current: list[str] | None = None
    for line in block[start + 1 : end]:
        if line.startswith("  - "):
            if current is not None:
                values.append(_decode_scalar(current, field=f"{field}[{len(values)}]"))
            current = [line[4:]]
        elif line.strip():
            if current is None or _leading_spaces(line) < 4:
                fail("SOURCE_LIST", f"{field} contains malformed list content")
            current.append(line.strip())
    if current is not None:
        values.append(_decode_scalar(current, field=f"{field}[{len(values)}]"))
    if not values:
        fail("SOURCE_LIST", f"{field} is an empty implicit list")
    return values


def _parse_global_lenses(lines: Sequence[str]) -> list[dict[str, str]]:
    starts = [index for index, line in enumerate(lines) if line.startswith("- id: L")]
    if len(starts) != EXPECTED_LENS_COUNT:
        fail("SOURCE_GLOBAL_LENSES", f"expected 20 global lenses, found {len(starts)}")
    lenses: list[dict[str, str]] = []
    for position, start in enumerate(starts):
        end = starts[position + 1] if position + 1 < len(starts) else len(lines)
        block = lines[start:end]
        lens_id = _decode_scalar([block[0][len("- id:") :]], field="global lens id")
        ranges = _field_ranges(
            block[1:],
            _GLOBAL_LENS_FIELD_RE,
            expected=frozenset({"name", "question"}),
            context=lens_id,
        )
        lenses.append(
            {
                "id": lens_id,
                "name": _parse_scalar_range(
                    block[1:], ranges["name"], field=f"{lens_id}.name"
                ),
                "question": _parse_scalar_range(
                    block[1:], ranges["question"], field=f"{lens_id}.question"
                ),
            }
        )
    return lenses


def _parse_task_lenses(
    block: Sequence[str], value_range: tuple[int, int, str], *, task_id: str
) -> list[dict[str, str]]:
    start, end, suffix = value_range
    if suffix:
        fail("SOURCE_TASK_LENSES", f"{task_id}.twenty_lens_review has inline data")
    nested = block[start + 1 : end]
    starts = [
        index
        for index, line in enumerate(nested)
        if re.fullmatch(r"    L[0-9]{2}:", line)
    ]
    if len(starts) != EXPECTED_LENS_COUNT:
        fail("SOURCE_TASK_LENSES", f"{task_id} has {len(starts)} lens records")
    lenses: list[dict[str, str]] = []
    for position, lens_start in enumerate(starts):
        lens_end = starts[position + 1] if position + 1 < len(starts) else len(nested)
        lens_block = nested[lens_start:lens_end]
        lens_id = lens_block[0].strip()[:-1]
        ranges = _field_ranges(
            lens_block[1:],
            _LENS_FIELD_RE,
            expected=_LENS_FIELDS,
            context=f"{task_id}.{lens_id}",
        )
        record = {"id": lens_id}
        for field in ("name", "question", "finding", "evidence", "status"):
            field_start, field_end, field_suffix = ranges[field]
            parts = [field_suffix]
            for line in lens_block[1:][field_start + 1 : field_end]:
                if line.strip():
                    parts.append(line.strip())
            record[field] = _decode_optional_scalar(
                parts, field=f"{task_id}.{lens_id}.{field}"
            )
        lenses.append(record)
    return lenses


def parse_master_ledger(raw: bytes) -> dict[str, Any]:
    if len(raw) != EXPECTED_LEDGER_BYTES:
        fail("LEDGER_BYTES", f"master ledger has unexpected byte count {len(raw)}")
    digest = sha256_bytes(raw)
    if digest != EXPECTED_LEDGER_SHA256:
        fail("LEDGER_SHA256", f"master ledger has unexpected SHA-256 {digest}")
    try:
        text = raw.decode("utf-8", errors="strict")
    except UnicodeDecodeError as exc:
        fail("LEDGER_UTF8", f"master ledger is not UTF-8: {exc}")
    if "\r" in text or "\x00" in text:
        fail("LEDGER_FRAMING", "master ledger must use LF framing and contain no NUL")
    kept_lines = text.splitlines(keepends=True)
    lines = [line.removesuffix("\n") for line in kept_lines]
    if lines.count("twenty_lenses:") != 1 or lines.count("tasks:") != 1:
        fail(
            "LEDGER_SECTIONS",
            "master ledger must contain one lens and one task section",
        )
    lens_index = lines.index("twenty_lenses:")
    task_section_index = lines.index("tasks:")
    if task_section_index <= lens_index:
        fail("LEDGER_SECTIONS", "task section precedes lens section")

    metadata: dict[str, Any] = {}
    top_lines = lines[:lens_index]
    for field in _TOP_FIELDS:
        matches = [
            line.split(":", 1)[1].strip()
            for line in top_lines
            if line.startswith(f"{field}:")
        ]
        if len(matches) != 1:
            fail("SOURCE_TOP_FIELD", f"{field} must occur exactly once")
        value = _decode_scalar(matches, field=field)
        metadata[field] = (
            int(value)
            if field
            in {"lead_agents", "max_concurrent_subagents", "phase_count", "task_count"}
            else value
        )
    expected_top_lines = len(_TOP_FIELDS)
    if len([line for line in top_lines if line.strip()]) != expected_top_lines:
        fail("SOURCE_TOP_FIELD", "master ledger has unrecognized top-level metadata")

    global_lenses = _parse_global_lenses(lines[lens_index + 1 : task_section_index])
    task_starts = [
        index
        for index in range(task_section_index + 1, len(lines))
        if lines[index].startswith("- id: T")
    ]
    if len(task_starts) != EXPECTED_TASK_COUNT:
        fail("SOURCE_TASK_COUNT", f"expected 240 tasks, found {len(task_starts)}")
    tasks: list[dict[str, Any]] = []
    for position, absolute_start in enumerate(task_starts):
        absolute_end = (
            task_starts[position + 1] if position + 1 < len(task_starts) else len(lines)
        )
        block = lines[absolute_start:absolute_end]
        task_id = _decode_scalar([block[0][len("- id:") :]], field="task id")
        ranges = _field_ranges(
            block[1:], _TASK_FIELD_RE, expected=_TASK_FIELDS, context=task_id
        )
        task: dict[str, Any] = {"id": task_id}
        for field in (
            "phase_id",
            "phase_title",
            "title",
            "priority",
            "current_head",
            "head_mismatch_rule",
            "completion_rule",
        ):
            task[field] = _parse_scalar_range(
                block[1:], ranges[field], field=f"{task_id}.{field}"
            )
        for field in _TASK_INTEGER_FIELDS:
            raw_integer = _parse_scalar_range(
                block[1:], ranges[field], field=f"{task_id}.{field}"
            )
            try:
                task[field] = int(raw_integer, 10)
            except ValueError:
                fail("SOURCE_INTEGER", f"{task_id}.{field} is not an integer")
        for field in _TASK_LIST_FIELDS:
            task[field] = _parse_list_range(
                block[1:], ranges[field], field=f"{task_id}.{field}"
            )
        task["twenty_lens_review"] = _parse_task_lenses(
            block[1:], ranges["twenty_lens_review"], task_id=task_id
        )
        source_block = "".join(kept_lines[absolute_start:absolute_end]).encode("utf-8")
        task["source_block"] = {
            "line_start": absolute_start + 1,
            "line_end": absolute_end,
            "sha256": sha256_bytes(source_block),
            "bytes": len(source_block),
        }
        tasks.append(task)

    document = {
        "metadata": metadata,
        "twenty_lenses": global_lenses,
        "tasks": tasks,
    }
    validate_parsed_requirements(document)
    return document


def validate_parsed_requirements(document: Mapping[str, Any]) -> None:
    metadata = document["metadata"]
    expected_metadata = {
        "handoff_schema": "2.0.0",
        "project": PROJECT,
        "repository": REPOSITORY,
        "frozen_commit": EXPECTED_FROZEN_COMMIT,
        "release_target": NOMINAL_RELEASE_VERSION,
        "status": "NO_GO_PENDING_IMPLEMENTATION_AND_EVIDENCE",
        "lead_agents": 1,
        "max_concurrent_subagents": 3,
        "phase_count": EXPECTED_PHASE_COUNT,
        "task_count": EXPECTED_TASK_COUNT,
    }
    if metadata != expected_metadata:
        fail(
            "SOURCE_METADATA",
            "master-ledger metadata differs from reviewed requirements",
        )
    lenses = document["twenty_lenses"]
    if [lens["id"] for lens in lenses] != [f"L{index:02d}" for index in range(1, 21)]:
        fail("SOURCE_GLOBAL_LENSES", "global lens IDs are not L01 through L20")
    tasks = document["tasks"]
    for position, task in enumerate(tasks):
        task_id = f"T{position:03d}"
        if task["id"] != task_id or _TASK_ID_RE.fullmatch(task["id"]) is None:
            fail("SOURCE_TASK_IDS", f"expected {task_id}, found {task['id']!r}")
        expected_phase = f"P{position // 15:02d}"
        if (
            task["phase_id"] != expected_phase
            or _PHASE_ID_RE.fullmatch(task["phase_id"]) is None
        ):
            fail("SOURCE_PHASE", f"{task_id} has wrong phase")
        if task["execution_wave"] != position // 15:
            fail("SOURCE_WAVE", f"{task_id} has wrong execution wave")
        if task["subagent_lane"] not in {1, 2, 3}:
            fail("SOURCE_LANE", f"{task_id} has wrong subagent lane")
        expected_dependencies = [] if position == 0 else [f"T{position - 1:03d}"]
        if task["dependencies"] != expected_dependencies:
            fail(
                "SOURCE_DEPENDENCY",
                f"{task_id} does not depend exactly on its predecessor",
            )
        if task["priority"] != "P0_RELEASE_BLOCKER":
            fail("SOURCE_PRIORITY", f"{task_id} is not a P0 release blocker")
        if task["current_head"] != EXPECTED_FROZEN_COMMIT:
            fail("SOURCE_HEAD", f"{task_id} has a different reviewed head")
        for field in _TASK_LIST_FIELDS - {"dependencies"}:
            if not task[field]:
                fail("SOURCE_REQUIREMENTS", f"{task_id}.{field} is empty")
        if not task["head_mismatch_rule"] or not task["completion_rule"]:
            fail("SOURCE_REQUIREMENTS", f"{task_id} lacks a closure rule")
        task_lenses = task["twenty_lens_review"]
        if [lens["id"] for lens in task_lenses] != [lens["id"] for lens in lenses]:
            fail("SOURCE_TASK_LENSES", f"{task_id} lens IDs differ from global lenses")
        for global_lens, task_lens in zip(lenses, task_lenses, strict=True):
            if (
                task_lens["name"] != global_lens["name"]
                or task_lens["question"] != global_lens["question"]
            ):
                fail("SOURCE_TASK_LENSES", f"{task_id}.{task_lens['id']} text drifts")
            if (
                task_lens["finding"] != ""
                or task_lens["evidence"] != ""
                or task_lens["status"] != "OPEN"
            ):
                fail(
                    "FALSE_IMPORTED_REVIEW",
                    f"{task_id}.{task_lens['id']} is not open and empty",
                )


def _read_regular(path: Path, *, code: str, max_bytes: int) -> bytes:
    if max_bytes < 0:
        fail(code, f"invalid negative byte limit for {path}: {max_bytes}")
    flags = (
        os.O_RDONLY
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_NOFOLLOW", 0)
        | getattr(os, "O_NONBLOCK", 0)
    )
    try:
        descriptor = os.open(path, flags)
    except OSError as exc:
        fail(code, f"cannot open regular non-symlink file {path}: {exc}")
    try:
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode):
            fail(code, f"expected regular non-symlink file: {path}")
        if before.st_size < 0 or before.st_size > max_bytes:
            fail(code, f"file exceeds {max_bytes} bytes: {path}")

        raw = bytearray()
        while len(raw) <= max_bytes:
            chunk = os.read(descriptor, min(1024 * 1024, max_bytes + 1 - len(raw)))
            if not chunk:
                break
            raw.extend(chunk)
        if len(raw) > max_bytes:
            fail(code, f"file exceeds {max_bytes} bytes: {path}")

        after = os.fstat(descriptor)
        stable_fields = (
            "st_dev",
            "st_ino",
            "st_mode",
            "st_size",
            "st_mtime_ns",
            "st_ctime_ns",
        )
        if any(
            getattr(before, field) != getattr(after, field) for field in stable_fields
        ):
            fail(code, f"file changed while it was read: {path}")
        if len(raw) != after.st_size:
            fail(code, f"file size changed while it was read: {path}")
        try:
            named = os.stat(path, follow_symlinks=False)
        except OSError as exc:
            fail(code, f"cannot verify opened file identity for {path}: {exc}")
        if not stat.S_ISREG(named.st_mode) or any(
            getattr(named, field) != getattr(after, field) for field in stable_fields
        ):
            fail(code, f"file path changed while it was read: {path}")
        return bytes(raw)
    except OSError as exc:
        fail(code, f"cannot read {path}: {exc}")
    finally:
        try:
            os.close(descriptor)
        except OSError:
            pass


def _parse_checksum_file(raw: bytes) -> list[tuple[str, str]]:
    if (
        len(raw) != EXPECTED_CHECKSUM_BYTES
        or sha256_bytes(raw) != EXPECTED_CHECKSUM_SHA256
    ):
        fail("CHECKSUM_IDENTITY", "29_SHA256SUMS.txt differs from the reviewed package")
    try:
        text = raw.decode("utf-8", errors="strict")
    except UnicodeDecodeError as exc:
        fail("CHECKSUM_UTF8", f"checksum manifest is not UTF-8: {exc}")
    if not text.endswith("\n") or "\r" in text or "\x00" in text:
        fail("CHECKSUM_FRAMING", "checksum manifest has invalid framing")
    entries: list[tuple[str, str]] = []
    for index, line in enumerate(text.splitlines(), start=1):
        match = re.fullmatch(r"([0-9a-f]{64})  (.+)", line)
        if match is None:
            fail("CHECKSUM_LINE", f"invalid checksum line {index}")
        digest, path = match.groups()
        _safe_relative_path(path, code="CHECKSUM_PATH")
        entries.append((path, digest))
    if len(entries) != EXPECTED_PAYLOAD_COUNT:
        fail("CHECKSUM_COUNT", f"expected 42 checksum entries, found {len(entries)}")
    paths = [path for path, _ in entries]
    if len(paths) != len(set(paths)) or paths != sorted(
        paths, key=lambda item: item.encode("utf-8")
    ):
        fail("CHECKSUM_ORDER", "checksum paths are duplicated or not bytewise sorted")
    if CHECKSUM_NAME in paths or any(path in EXCLUDED_PATHS for path in paths):
        fail(
            "CHECKSUM_BOUNDARY", "checksum manifest includes itself or an excluded file"
        )
    return entries


def _observed_files(root: Path) -> set[str]:
    observed: set[str] = set()
    stack = [(root, 0)]
    entry_count = 0
    path_bytes = 0
    while stack:
        directory, depth = stack.pop()
        try:
            with os.scandir(directory) as iterator:
                for item in iterator:
                    entry_count += 1
                    if entry_count > MAX_HANDOFF_ENTRIES:
                        fail(
                            "HANDOFF_WALK_BUDGET",
                            f"handoff exceeds {MAX_HANDOFF_ENTRIES} filesystem entries",
                        )
                    candidate = Path(item.path)
                    relative = candidate.relative_to(root).as_posix()
                    _safe_relative_path(relative, code="HANDOFF_PATH")
                    path_bytes += len(relative.encode("utf-8"))
                    if path_bytes > MAX_HANDOFF_PATH_BYTES:
                        fail(
                            "HANDOFF_WALK_BUDGET",
                            f"handoff paths exceed {MAX_HANDOFF_PATH_BYTES} bytes",
                        )
                    if item.is_symlink():
                        fail(
                            "HANDOFF_SYMLINK",
                            f"handoff contains a symlink: {relative}",
                        )
                    if item.is_file(follow_symlinks=False):
                        observed.add(relative)
                    elif item.is_dir(follow_symlinks=False):
                        if depth >= MAX_HANDOFF_DEPTH:
                            fail(
                                "HANDOFF_WALK_BUDGET",
                                f"handoff exceeds directory depth {MAX_HANDOFF_DEPTH}",
                            )
                        stack.append((candidate, depth + 1))
                    else:
                        fail(
                            "HANDOFF_SPECIAL_FILE",
                            f"handoff contains a special file: {relative}",
                        )
        except RequirementsError:
            raise
        except OSError as exc:
            fail("HANDOFF_WALK", f"cannot enumerate handoff directory: {exc}")
    return observed


def build_package_manifest(handoff_dir: Path) -> tuple[dict[str, Any], bytes]:
    if handoff_dir.is_symlink() or not handoff_dir.is_dir():
        fail("HANDOFF_DIRECTORY", f"handoff must be a real directory: {handoff_dir}")
    if handoff_dir.name != EXPECTED_DIRECTORY_NAME:
        fail(
            "HANDOFF_DIRECTORY_NAME",
            f"handoff directory must be named {EXPECTED_DIRECTORY_NAME}",
        )
    checksum_raw = _read_regular(
        handoff_dir / CHECKSUM_NAME, code="CHECKSUM_FILE", max_bytes=64 * 1024
    )
    checksum_entries = _parse_checksum_file(checksum_raw)
    expected_files = (
        {path for path, _ in checksum_entries} | {CHECKSUM_NAME} | set(EXCLUDED_PATHS)
    )
    observed_files = _observed_files(handoff_dir)
    if observed_files != expected_files:
        fail(
            "HANDOFF_FILE_SET",
            f"handoff file set differs; missing={sorted(expected_files - observed_files)}, "
            f"extra={sorted(observed_files - expected_files)}",
        )
    included: list[dict[str, Any]] = []
    ledger_raw: bytes | None = None
    for path, expected_digest in checksum_entries:
        raw = _read_regular(
            handoff_dir / path, code="HANDOFF_PAYLOAD", max_bytes=16 * 1024 * 1024
        )
        actual_digest = sha256_bytes(raw)
        if actual_digest != expected_digest:
            fail("HANDOFF_PAYLOAD_HASH", f"handoff payload hash differs: {path}")
        if path == LEDGER_NAME:
            ledger_raw = raw
        included.append(
            {
                "path": path,
                "sha256": actual_digest,
                "bytes": len(raw),
                "identity_source": CHECKSUM_NAME,
                "role": "master_task_ledger"
                if path == LEDGER_NAME
                else "handoff_payload",
            }
        )
    included.append(
        {
            "path": CHECKSUM_NAME,
            "sha256": sha256_bytes(checksum_raw),
            "bytes": len(checksum_raw),
            "identity_source": "reviewed generator constant; checksum file cannot list itself",
            "role": "checksum_manifest",
        }
    )
    included.sort(key=lambda entry: entry["path"].encode("utf-8"))
    if len(included) != EXPECTED_INCLUDED_COUNT or ledger_raw is None:
        fail("HANDOFF_INCLUDED_COUNT", "handoff included-file count is wrong")
    if _observed_files(handoff_dir) != observed_files:
        fail("HANDOFF_RACE", "handoff file set changed during the first capture")
    second_checksum_raw = _read_regular(
        handoff_dir / CHECKSUM_NAME, code="CHECKSUM_FILE", max_bytes=64 * 1024
    )
    if second_checksum_raw != checksum_raw:
        fail("HANDOFF_RACE", "checksum manifest changed across stable capture")
    first_by_path = {entry["path"]: entry for entry in included}
    for path, _ in checksum_entries:
        raw = _read_regular(
            handoff_dir / path,
            code="HANDOFF_PAYLOAD",
            max_bytes=16 * 1024 * 1024,
        )
        first = first_by_path[path]
        if len(raw) != first["bytes"] or sha256_bytes(raw) != first["sha256"]:
            fail("HANDOFF_RACE", f"handoff payload changed across capture: {path}")
    if _observed_files(handoff_dir) != observed_files:
        fail("HANDOFF_RACE", "handoff file set changed during stable recapture")
    exclusions = [
        {
            "path": path,
            "present_as_regular_file": True,
            "content_read": False,
            "content_identity_bound": False,
            "reason": "operating-system metadata excluded from the requirements corpus",
        }
        for path in EXCLUDED_PATHS
    ]
    identity_material = {
        "included_files": included,
        "excluded_paths": list(EXCLUDED_PATHS),
    }
    package_identity = semantic_sha256(identity_material)
    if (
        EXPECTED_PACKAGE_IDENTITY_SHA256 is not None
        and package_identity != EXPECTED_PACKAGE_IDENTITY_SHA256
    ):
        fail("PACKAGE_IDENTITY", f"package identity differs: {package_identity}")
    manifest = {
        "schema_version": SCHEMA_VERSION,
        "record_type": "immutable_handoff_package_manifest",
        "project": PROJECT,
        "release_version": RELEASE_VERSION,
        "source": {
            "operator_supplied_explicit_directory": True,
            "expected_directory_name": EXPECTED_DIRECTORY_NAME,
            "absolute_path_recorded": False,
            "stable_double_capture_required": True,
            "checksum_manifest": {
                "path": CHECKSUM_NAME,
                "sha256": EXPECTED_CHECKSUM_SHA256,
                "bytes": EXPECTED_CHECKSUM_BYTES,
                "payload_entry_count": EXPECTED_PAYLOAD_COUNT,
            },
        },
        "included_file_count": len(included),
        "excluded_file_count": len(exclusions),
        "observed_file_count": len(observed_files),
        "package_identity_sha256": package_identity,
        "identity_semantics": PACKAGE_IDENTITY_SEMANTICS,
        "included_files": included,
        "excluded_files": exclusions,
        "review": {
            "status": "package_identity_verified_requirements_not_completed",
            "all_payload_hashes_verified": True,
            "all_files_read_for_substantive_review": False,
            "human_review_complete": False,
            "independent_review_complete": False,
            "release_ready": False,
        },
    }
    return manifest, ledger_raw


def build_disposition_baseline(parsed: Mapping[str, Any]) -> dict[str, Any]:
    phases: list[dict[str, Any]] = []
    for phase_index in range(EXPECTED_PHASE_COUNT):
        members = parsed["tasks"][phase_index * 15 : (phase_index + 1) * 15]
        phases.append(
            {
                "id": f"P{phase_index:02d}",
                "title": members[0]["phase_title"],
                "execution_wave": phase_index,
                "task_ids": [task["id"] for task in members],
                "disposition_status": "open",
            }
        )
    tasks: list[dict[str, Any]] = []
    for source_task in parsed["tasks"]:
        requirements = {
            key: source_task[key]
            for key in (
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
                "head_mismatch_rule",
                "preconditions",
                "procedure",
                "mandatory_adversarial_questions",
                "required_tests",
                "required_evidence",
                "completion_rule",
                "source_block",
            )
        }
        lens_requirements = [dict(lens) for lens in source_task["twenty_lens_review"]]
        lens_dispositions = [
            {
                "lens_id": lens["id"],
                "status": "open",
                "finding": None,
                "evidence_refs": [],
                "blockers": [],
                "reviewer": None,
                "reviewed_at": None,
            }
            for lens in lens_requirements
        ]
        tasks.append(
            {
                "requirements": requirements,
                "lens_requirements": lens_requirements,
                "task_disposition": {
                    "status": "open",
                    "decision": None,
                    "owner": None,
                    "evidence_refs": [],
                    "blockers": [],
                    "claim_impact": None,
                    "reviewer": None,
                    "independent_reviewer": None,
                    "completed_at": None,
                },
                "lens_dispositions": lens_dispositions,
            }
        )
    requirement_material = {
        "metadata": parsed["metadata"],
        "twenty_lenses": parsed["twenty_lenses"],
        "tasks": [
            {
                "requirements": task["requirements"],
                "lens_requirements": task["lens_requirements"],
            }
            for task in tasks
        ],
    }
    requirements_semantic_sha256 = semantic_sha256(requirement_material)
    if requirements_semantic_sha256 != EXPECTED_REQUIREMENTS_SEMANTIC_SHA256:
        fail(
            "REQUIREMENTS_SEMANTIC_IDENTITY",
            f"parsed requirements semantic identity differs: {requirements_semantic_sha256}",
        )
    return {
        "schema_version": SCHEMA_VERSION,
        "record_type": "handoff_task_disposition_baseline",
        "project": PROJECT,
        "repository": REPOSITORY,
        "author": AUTHOR,
        "release": {
            "nominal_handoff_target": NOMINAL_RELEASE_VERSION,
            "requested_release": RELEASE_VERSION,
            "doi": None,
            "zenodo_record": None,
        },
        "source": {
            "path": LEDGER_NAME,
            "sha256": EXPECTED_LEDGER_SHA256,
            "bytes": EXPECTED_LEDGER_BYTES,
            "frozen_commit": EXPECTED_FROZEN_COMMIT,
            "pid_rs_gitlink_commit": EXPECTED_PID_RS_COMMIT,
            "requirements_semantic_sha256": requirements_semantic_sha256,
        },
        "imported_metadata": parsed["metadata"],
        "twenty_lenses": parsed["twenty_lenses"],
        "phase_count": EXPECTED_PHASE_COUNT,
        "task_count": EXPECTED_TASK_COUNT,
        "lens_count": EXPECTED_LENS_COUNT,
        "lens_disposition_count": EXPECTED_LENS_DISPOSITION_COUNT,
        "review": {
            "status": "immutable_import_all_dispositions_open",
            "open_task_count": EXPECTED_TASK_COUNT,
            "closed_task_count": 0,
            "open_lens_disposition_count": EXPECTED_LENS_DISPOSITION_COUNT,
            "closed_lens_disposition_count": 0,
            "all_tasks_closed": False,
            "human_review_complete": False,
            "independent_review_complete": False,
            "release_gate_passed": False,
            "scientific_claims_established": False,
            "boundary": (
                "This immutable baseline preserves imported obligations and empty disposition "
                "fields. It is not a live execution ledger and establishes no task completion."
            ),
        },
        "phases": phases,
        "tasks": tasks,
    }


def build_artifact_manifest(artifacts: Mapping[str, bytes]) -> dict[str, Any]:
    if set(artifacts) != set(ARTIFACT_NAMES):
        fail("ARTIFACT_INPUT", "artifact manifest input set is wrong")
    return {
        "schema_version": SCHEMA_VERSION,
        "record_type": "release_requirements_artifact_manifest",
        "project": PROJECT,
        "release_version": RELEASE_VERSION,
        "artifacts": [
            {
                "path": name,
                "sha256": sha256_bytes(artifacts[name]),
                "bytes": len(artifacts[name]),
            }
            for name in sorted(ARTIFACT_NAMES)
        ],
        "review_status": "integrity_only_no_completion_claim",
    }


def build_artifacts(handoff_dir: Path) -> dict[str, bytes]:
    package_manifest, ledger_raw = build_package_manifest(handoff_dir)
    parsed = parse_master_ledger(ledger_raw)
    disposition = build_disposition_baseline(parsed)
    artifacts = {
        LEDGER_NAME: ledger_raw,
        PACKAGE_MANIFEST_NAME: pretty_json_bytes(package_manifest),
        DISPOSITIONS_NAME: pretty_json_bytes(disposition),
    }
    artifacts[ARTIFACT_MANIFEST_NAME] = pretty_json_bytes(
        build_artifact_manifest(artifacts)
    )
    return artifacts


def _directory_names(path: Path) -> set[str]:
    names: set[str] = set()
    try:
        for entry in path.iterdir():
            if len(names) >= len(EXPECTED_OUTPUT_NAMES):
                fail("OUTPUT_FILE_SET", "requirements output contains extra entries")
            if entry.is_symlink() or not entry.is_file():
                fail("OUTPUT_ENTRY", f"output contains a non-regular entry: {entry}")
            names.add(entry.name)
    except RequirementsError:
        raise
    except OSError as exc:
        fail("OUTPUT_DIRECTORY_READ", f"cannot read output directory: {exc}")
    return names


def _require_exact_output_set(path: Path, *, allow_empty: bool) -> None:
    names = _directory_names(path)
    if allow_empty and not names:
        return
    if names != EXPECTED_OUTPUT_NAMES:
        fail(
            "OUTPUT_FILE_SET",
            f"requirements output set differs; missing={sorted(EXPECTED_OUTPUT_NAMES - names)}, "
            f"extra={sorted(names - EXPECTED_OUTPUT_NAMES)}",
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
        fail(
            "OUTPUT_WRITE", f"cannot durably sync requirements directory {path}: {exc}"
        )


def write_artifacts(output_dir: Path, artifacts: Mapping[str, bytes]) -> None:
    if output_dir.exists():
        if output_dir.is_symlink() or not output_dir.is_dir():
            fail("OUTPUT_DIRECTORY", f"output must be a real directory: {output_dir}")
        _require_exact_output_set(output_dir, allow_empty=True)
    else:
        try:
            output_dir.mkdir(parents=True, exist_ok=False)
        except OSError as exc:
            fail("OUTPUT_DIRECTORY", f"cannot create output directory: {exc}")
    if set(artifacts) != EXPECTED_OUTPUT_NAMES:
        fail("OUTPUT_ARTIFACT_SET", "generated artifact set is wrong")
    # Publish the integrity manifest only after every file that it binds has been
    # installed and the directory entries have been synced. An interrupted refresh
    # therefore cannot expose a new manifest for a partial payload set.
    write_order = sorted(name for name in artifacts if name != ARTIFACT_MANIFEST_NAME)
    write_order.append(ARTIFACT_MANIFEST_NAME)
    for name in write_order:
        if name == ARTIFACT_MANIFEST_NAME:
            _fsync_directory(output_dir)
        _safe_relative_path(name, code="OUTPUT_NAME")
        destination = output_dir / name
        temporary = output_dir / f".{name}.tmp.{os.getpid()}"
        if temporary.exists() or temporary.is_symlink():
            fail("OUTPUT_TEMP", f"temporary output already exists: {temporary}")
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
            fail("OUTPUT_WRITE", f"cannot write {destination}: {exc}")
    _fsync_directory(output_dir)
    _require_exact_output_set(output_dir, allow_empty=False)


def check_artifacts(output_dir: Path, artifacts: Mapping[str, bytes]) -> None:
    if output_dir.is_symlink() or not output_dir.is_dir():
        fail("OUTPUT_DIRECTORY", f"output must be a real directory: {output_dir}")
    _require_exact_output_set(output_dir, allow_empty=False)
    for name, expected in artifacts.items():
        actual = _read_regular(
            output_dir / name, code="OUTPUT_ARTIFACT", max_bytes=32 * 1024 * 1024
        )
        if actual != expected:
            fail("OUTPUT_DRIFT", f"generated requirements artifact is stale: {name}")


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--handoff-dir",
        required=True,
        type=Path,
        help="explicit path to PRISOMA_V1_0_CURRENT_HEAD_MAX_EFFORT_STANDALONE_HANDOFF",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("release/0.9.0/requirements"),
        help="destination for immutable requirements artifacts",
    )
    parser.add_argument("--check", action="store_true", help="compare without writing")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        artifacts = build_artifacts(args.handoff_dir)
        if args.check:
            check_artifacts(args.output_dir, artifacts)
        else:
            write_artifacts(args.output_dir, artifacts)
    except RequirementsError as exc:
        print(f"requirements generation failed [{exc.code}]: {exc}", file=sys.stderr)
        return 3
    mode = "current" if args.check else "generated"
    print(
        f"release requirements {mode}: tasks={EXPECTED_TASK_COUNT}; "
        f"lens_dispositions={EXPECTED_LENS_DISPOSITION_COUNT}; artifacts={len(artifacts)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
