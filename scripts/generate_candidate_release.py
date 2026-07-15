#!/usr/bin/env python3
"""Generate a content-bound, deliberately unpublished Prisoma 0.9 candidate.

The immutable handoff requirements remain untouched. This program derives a live execution
ledger whose dispositions default to open, merges explicit candidate progress, and binds the
result to an explicit Git index and working-tree snapshot. Candidate artifacts exclude their
own output directory to avoid self-reference; the artifact manifest binds every generated
candidate file instead.
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
from collections import Counter
from collections.abc import Mapping, Sequence
from datetime import datetime
from pathlib import Path, PurePosixPath
from typing import Any


SCHEMA_VERSION = "prisoma.release-candidate/0.1.0"
PROJECT = "prisoma"
REPOSITORY = "https://github.com/sepahead/prisoma"
AUTHOR = "Sepehr Mahmoudian"
RELEASE_VERSION = "0.9.0"
NOMINAL_HANDOFF_VERSION = "1.0.0"
CANDIDATE_RELATIVE = "release/0.9.0/candidate"
PROGRESS_RELATIVE = "release/0.9.0/candidate_progress.json"
EVIDENCE_RELATIVE = "release/0.9.0/evidence"
PROGRESS_SCHEMA_VERSION = "prisoma.release-candidate-progress/0.1.0"
TERMINAL_PROMOTION_POLICY = "disabled_in_0.1_pending_typed_authenticated_evidence"
BASELINE_RELATIVE = "release/0.9.0/requirements/task_dispositions.baseline.json"
BASELINE_SHA256 = "43cb0b6e2e77557ccceed2736fa319c16358039918052943c3fd085c9298affc"
BASELINE_REQUIREMENTS_SHA256 = (
    "a83af23b486cc148fed6f194953cc7509b5f55f13dd7a5086fd5ebdb52672035"
)
EXPECTED_TASK_COUNT = 240
EXPECTED_LENS_COUNT = 20
EXPECTED_LENS_DISPOSITION_COUNT = EXPECTED_TASK_COUNT * EXPECTED_LENS_COUNT
MAX_FILE_BYTES = 256 * 1024 * 1024
MAX_BASELINE_BYTES = 16 * 1024 * 1024
MAX_PROGRESS_BYTES = 8 * 1024 * 1024
MAX_CANDIDATE_ARTIFACT_BYTES = 64 * 1024 * 1024
MAX_INVENTORY_ENTRIES = 512
MAX_INVENTORY_PATH_BYTES = 4 * 1024 * 1024
MAX_INVENTORY_CONTENT_BYTES = 2 * 1024 * 1024 * 1024
MAX_INVENTORY_LISTING_BYTES = MAX_INVENTORY_PATH_BYTES + MAX_INVENTORY_ENTRIES * 128
MAX_GIT_STDERR_BYTES = 1024 * 1024
GIT_COMMAND_TIMEOUT_SECONDS = 120.0
INVENTORY_GIT_DEADLINE_SECONDS = 120.0
RECURSIVE_GITLINK_PATHS = ("pid-rs",)

INVENTORY_NAME = "source_inventory.json"
TASK_LEDGER_NAME = "task_lens_ledger.json"
CLAIM_LEDGER_NAME = "claim_evidence_ledger.json"
DEFECT_REGISTER_NAME = "defect_register.json"
RECEIPTS_NAME = "evidence_receipts.json"
DRAFT_MANIFEST_NAME = "draft_release_manifest.json"
ARTIFACT_MANIFEST_NAME = "artifact_manifest.json"
ARTIFACT_NAMES = (
    INVENTORY_NAME,
    TASK_LEDGER_NAME,
    CLAIM_LEDGER_NAME,
    DEFECT_REGISTER_NAME,
    RECEIPTS_NAME,
    DRAFT_MANIFEST_NAME,
)
EXPECTED_OUTPUT_NAMES = frozenset((*ARTIFACT_NAMES, ARTIFACT_MANIFEST_NAME))

_SHA256_RE = re.compile(r"[0-9a-f]{64}\Z")
_OID_RE = re.compile(r"[0-9a-f]{40}\Z")
_TIMESTAMP_RE = re.compile(
    r"20[0-9]{2}-(?:0[1-9]|1[0-2])-(?:0[1-9]|[12][0-9]|3[01])"
    r"T(?:[01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]Z\Z"
)


class CandidateError(ValueError):
    """Controlled candidate-generation failure."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


def fail(code: str, message: str) -> None:
    raise CandidateError(code, message)


def canonical_json_bytes(value: Any) -> bytes:
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


def sha256_bytes(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def semantic_sha256(value: Any) -> str:
    return sha256_bytes(canonical_json_bytes(value))


def _stat_identity(value: os.stat_result) -> tuple[int, int, int, int, int, int]:
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
    size_code: str,
    description: str,
) -> tuple[bytes, os.stat_result]:
    """Read one stable regular-file snapshot without following its final symlink."""

    if max_bytes < 0:
        fail(size_code, f"invalid negative byte limit for {description}: {max_bytes}")
    try:
        named_before = os.stat(path, follow_symlinks=False)
    except OSError as exc:
        fail(path_code, f"cannot inspect {description} {path}: {exc}")
    if not stat.S_ISREG(named_before.st_mode):
        fail(path_code, f"{description} must be a regular non-symlink file: {path}")

    flags = (
        os.O_RDONLY
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_NOFOLLOW", 0)
        | getattr(os, "O_NONBLOCK", 0)
    )
    try:
        descriptor = os.open(path, flags)
    except OSError as exc:
        fail(path_code, f"cannot open {description} {path}: {exc}")
    try:
        try:
            opened = os.fstat(descriptor)
        except OSError as exc:
            fail(read_code, f"cannot inspect opened {description} {path}: {exc}")
        if not stat.S_ISREG(opened.st_mode):
            fail(path_code, f"{description} must be a regular file: {path}")
        if (named_before.st_dev, named_before.st_ino) != (opened.st_dev, opened.st_ino):
            fail(read_code, f"{description} path changed while it was opened: {path}")
        if opened.st_size < 0 or opened.st_size > max_bytes:
            fail(size_code, f"{description} exceeds {max_bytes} bytes: {path}")

        raw = bytearray()
        while len(raw) <= max_bytes:
            request = min(1024 * 1024, max_bytes + 1 - len(raw))
            try:
                chunk = os.read(descriptor, request)
            except OSError as exc:
                fail(read_code, f"cannot read {description} {path}: {exc}")
            if not chunk:
                break
            raw.extend(chunk)
        if len(raw) > max_bytes:
            fail(size_code, f"{description} exceeds {max_bytes} bytes: {path}")

        try:
            opened_after = os.fstat(descriptor)
            named_after = os.stat(path, follow_symlinks=False)
        except OSError as exc:
            fail(read_code, f"cannot verify {description} snapshot {path}: {exc}")
        if (
            not stat.S_ISREG(opened_after.st_mode)
            or not stat.S_ISREG(named_after.st_mode)
            or _stat_identity(opened) != _stat_identity(opened_after)
            or _stat_identity(opened_after) != _stat_identity(named_after)
            or len(raw) != opened_after.st_size
        ):
            fail(read_code, f"{description} changed while it was read: {path}")
        return bytes(raw), opened_after
    finally:
        try:
            os.close(descriptor)
        except OSError:
            pass


def _run_git(
    repo: Path,
    args: Sequence[str],
    *,
    input_bytes: bytes | None = None,
    max_bytes: int = 512 * 1024 * 1024,
    timeout_seconds: float = GIT_COMMAND_TIMEOUT_SECONDS,
) -> bytes:
    if max_bytes < 0:
        fail("GIT_OUTPUT", f"invalid negative Git output budget: {max_bytes}")
    if timeout_seconds <= 0:
        fail("GIT_TIMEOUT", "Git command has no remaining execution budget")
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
        fail("GIT_UNAVAILABLE", f"cannot execute git: {exc}")
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
        fail(
            "GIT_TIMEOUT",
            f"git {' '.join(args)} exceeded {timeout_seconds:g} seconds",
        )
    if overflow == "stdout":
        fail("GIT_OUTPUT", f"git {' '.join(args)} exceeded the output budget")
    if overflow == "stderr":
        fail("GIT_STDERR", f"git {' '.join(args)} exceeded the stderr budget")
    if returncode != 0:
        detail = bytes(stderr).decode("utf-8", errors="replace").strip()
        fail("GIT_COMMAND", f"git {' '.join(args)} failed: {detail}")
    return bytes(stdout)


def resolve_repo(repo: Path) -> Path:
    if repo.is_symlink() or not repo.is_dir():
        fail("REPOSITORY", f"repository must be a real directory: {repo}")
    raw = _run_git(repo, ["rev-parse", "--show-toplevel"], max_bytes=16 * 1024)
    try:
        top = Path(raw.decode("utf-8", errors="strict").strip()).resolve(strict=True)
    except (OSError, UnicodeDecodeError) as exc:
        fail("REPOSITORY", f"cannot resolve repository root: {exc}")
    return top


def _reject_constant(value: str) -> Any:
    fail("JSON_NONFINITE", f"non-finite JSON number is forbidden: {value}")


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            fail("JSON_DUPLICATE_KEY", f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _assert_exact_keys(value: Any, keys: set[str], *, context: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        actual = sorted(value) if isinstance(value, dict) else type(value).__name__
        fail("PROGRESS_SCHEMA", f"{context} has wrong keys/type: {actual}")
    return value


def _require_text(value: Any, *, context: str, nullable: bool = False) -> str | None:
    if nullable and value is None:
        return None
    if (
        not isinstance(value, str)
        or not value.strip()
        or value != " ".join(value.split())
        or any(ord(character) < 0x20 for character in value)
    ):
        fail("PROGRESS_TEXT", f"{context} must be normalized nonempty text")
    return value


def _require_timestamp(
    value: Any, *, context: str, nullable: bool = False
) -> str | None:
    if nullable and value is None:
        return None
    if not isinstance(value, str) or _TIMESTAMP_RE.fullmatch(value) is None:
        fail("PROGRESS_TIMESTAMP", f"{context} must be an exact UTC second timestamp")
    try:
        datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ")
    except ValueError:
        fail("PROGRESS_TIMESTAMP", f"{context} must be a valid UTC calendar timestamp")
    return value


def _require_distinct_reviewers(
    reviewer: str | None, independent_reviewer: str | None, *, context: str
) -> None:
    if (
        reviewer is not None
        and independent_reviewer is not None
        and reviewer == independent_reviewer
    ):
        fail(
            "PROGRESS_INDEPENDENT_REVIEW",
            f"{context} independent reviewer must differ from reviewer",
        )


def _require_string_list(
    value: Any,
    *,
    context: str,
    allow_empty: bool,
    sorted_unique: bool = True,
) -> list[str]:
    if not isinstance(value, list) or (not allow_empty and not value):
        fail("PROGRESS_LIST", f"{context} must be a nonempty list")
    result: list[str] = []
    for index, item in enumerate(value):
        normalized = _require_text(item, context=f"{context}[{index}]")
        assert isinstance(normalized, str)
        result.append(normalized)
    if len(result) != len(set(result)):
        fail("PROGRESS_LIST", f"{context} contains duplicates")
    if sorted_unique and result != sorted(result):
        fail("PROGRESS_LIST", f"{context} must be lexicographically sorted")
    return result


def _validate_progress_document(value: Any, raw: bytes) -> tuple[dict[str, Any], bytes]:
    _assert_exact_keys(
        value,
        {
            "schema_version",
            "record_type",
            "project",
            "release_version",
            "progress_revision",
            "terminal_promotion_policy",
            "task_updates",
            "lens_updates",
            "file_review_updates",
            "claim_updates",
            "defect_updates",
            "evidence_receipt_updates",
            "wave_receipts",
            "boundary",
        },
        context="candidate progress input",
    )
    if (
        value["schema_version"] != PROGRESS_SCHEMA_VERSION
        or value["record_type"] != "candidate_progress_overrides"
        or value["project"] != PROJECT
        or value["release_version"] != RELEASE_VERSION
        or type(value["progress_revision"]) is not int
        or value["progress_revision"] < 0
        or value["terminal_promotion_policy"] != TERMINAL_PROMOTION_POLICY
    ):
        fail("PROGRESS_IDENTITY", "candidate progress identity/revision is wrong")
    for field in (
        "task_updates",
        "lens_updates",
        "file_review_updates",
        "claim_updates",
        "defect_updates",
        "evidence_receipt_updates",
        "wave_receipts",
    ):
        if not isinstance(value[field], list):
            fail("PROGRESS_SCHEMA", f"candidate progress {field} must be a list")
    _require_text(value["boundary"], context="candidate progress boundary")
    if pretty_json_bytes(value) != raw:
        fail(
            "PROGRESS_CANONICAL",
            "candidate progress input is not canonical pretty JSON",
        )
    return value, raw


def _read_progress(repo: Path) -> tuple[dict[str, Any], bytes]:
    path = repo / PROGRESS_RELATIVE
    raw, _ = _read_bounded_regular(
        path,
        max_bytes=MAX_PROGRESS_BYTES,
        path_code="PROGRESS_PATH",
        read_code="PROGRESS_READ",
        size_code="PROGRESS_SIZE",
        description="candidate progress input",
    )
    try:
        value = json.loads(
            raw,
            object_pairs_hook=_unique_object,
            parse_constant=_reject_constant,
        )
    except CandidateError:
        raise
    except (
        json.JSONDecodeError,
        UnicodeDecodeError,
        RecursionError,
        ValueError,
    ) as exc:
        fail("PROGRESS_JSON", f"cannot parse candidate progress input: {exc}")
    return _validate_progress_document(value, raw)


def _decode_path(raw: bytes, *, context: str) -> str:
    try:
        value = raw.decode("utf-8", errors="strict")
    except UnicodeDecodeError as exc:
        fail("PATH_UTF8", f"{context} is not UTF-8: {exc}")
    if (
        not value
        or value.startswith("/")
        or "\\" in value
        or any(ord(character) < 0x20 for character in value)
    ):
        fail("PATH_UNSAFE", f"unsafe repository path in {context}: {value!r}")
    parts = PurePosixPath(value).parts
    if not parts or any(part in {"", ".", ".."} for part in parts):
        fail("PATH_UNSAFE", f"unsafe repository path in {context}: {value!r}")
    return value


def _is_self_excluded(path: str) -> bool:
    return path == CANDIDATE_RELATIVE or path.startswith(f"{CANDIDATE_RELATIVE}/")


def _parse_index(raw: bytes) -> list[dict[str, Any]]:
    entries: list[dict[str, Any]] = []
    path_bytes = 0
    for record in raw.rstrip(b"\0").split(b"\0") if raw else []:
        try:
            metadata, raw_path = record.split(b"\t", 1)
            mode, oid, raw_stage = metadata.decode("ascii").split(" ")
            stage = int(raw_stage, 10)
        except (UnicodeDecodeError, ValueError) as exc:
            fail("INDEX_PARSE", f"cannot parse Git index entry: {exc}")
        path = _decode_path(raw_path, context="Git index")
        if _is_self_excluded(path):
            continue
        if len(entries) >= MAX_INVENTORY_ENTRIES:
            fail("INVENTORY_BUDGET", "Git index exceeds the inventory entry budget")
        path_bytes += len(raw_path)
        if path_bytes > MAX_INVENTORY_PATH_BYTES:
            fail("INVENTORY_BUDGET", "Git index paths exceed the inventory path budget")
        if stage != 0:
            fail("INDEX_UNMERGED", f"unmerged index stage {stage} at {path}")
        if mode not in {"100644", "100755", "120000", "160000"}:
            fail("INDEX_MODE", f"unsupported index mode {mode} at {path}")
        if len(oid) != 40 or any(
            character not in "0123456789abcdef" for character in oid
        ):
            fail("INDEX_OID", f"invalid index object ID at {path}")
        entries.append({"path": path, "mode": mode, "oid": oid, "stage": stage})
    entries.sort(key=lambda entry: entry["path"])
    paths = [entry["path"] for entry in entries]
    if len(paths) != len(set(paths)):
        fail("INDEX_DUPLICATE", "Git index contains duplicate stage-zero paths")
    return entries


def _parse_head_tree(raw: bytes) -> dict[str, dict[str, str]]:
    result: dict[str, dict[str, str]] = {}
    path_bytes = 0
    for record in raw.rstrip(b"\0").split(b"\0") if raw else []:
        try:
            metadata, raw_path = record.split(b"\t", 1)
            mode, object_type, oid = metadata.decode("ascii").split(" ")
        except (UnicodeDecodeError, ValueError) as exc:
            fail("HEAD_TREE_PARSE", f"cannot parse Git tree entry: {exc}")
        path = _decode_path(raw_path, context="HEAD tree")
        if _is_self_excluded(path):
            continue
        if len(result) >= MAX_INVENTORY_ENTRIES:
            fail("INVENTORY_BUDGET", "HEAD tree exceeds the inventory entry budget")
        path_bytes += len(raw_path)
        if path_bytes > MAX_INVENTORY_PATH_BYTES:
            fail("INVENTORY_BUDGET", "HEAD tree paths exceed the inventory path budget")
        if path in result:
            fail("HEAD_TREE_DUPLICATE", f"duplicate HEAD tree path: {path}")
        result[path] = {"mode": mode, "object_type": object_type, "oid": oid}
    return result


def _parse_pinned_gitlink_tree(
    raw: bytes, *, gitlink_path: str
) -> dict[str, dict[str, str]]:
    """Parse a pinned gitlink tree into parent-relative, blob-only file rows."""

    result: dict[str, dict[str, str]] = {}
    path_bytes = 0
    for record in raw.rstrip(b"\0").split(b"\0") if raw else []:
        try:
            metadata, raw_path = record.split(b"\t", 1)
            mode, object_type, oid = metadata.decode("ascii").split(" ")
        except (UnicodeDecodeError, ValueError) as exc:
            fail("GITLINK_TREE_PARSE", f"cannot parse {gitlink_path} tree entry: {exc}")
        relative = _decode_path(raw_path, context=f"{gitlink_path} pinned tree")
        path = f"{gitlink_path}/{relative}"
        if mode not in {"100644", "100755", "120000"} or object_type != "blob":
            fail(
                "GITLINK_TREE_TYPE",
                f"unsupported recursive object {mode} {object_type} at {path}",
            )
        if _OID_RE.fullmatch(oid) is None:
            fail("GITLINK_TREE_OID", f"invalid pinned blob object ID at {path}")
        if len(result) >= MAX_INVENTORY_ENTRIES:
            fail("INVENTORY_BUDGET", f"{gitlink_path} tree exceeds the entry budget")
        path_bytes += len(path.encode("utf-8"))
        if path_bytes > MAX_INVENTORY_PATH_BYTES:
            fail("INVENTORY_BUDGET", f"{gitlink_path} paths exceed the path budget")
        if path in result:
            fail("GITLINK_TREE_DUPLICATE", f"duplicate pinned gitlink path: {path}")
        result[path] = {"mode": mode, "object_type": object_type, "oid": oid}
    return result


def _read_stable_file(path: Path) -> tuple[bytes, os.stat_result]:
    return _read_bounded_regular(
        path,
        max_bytes=MAX_FILE_BYTES,
        path_code="WORKTREE_TYPE",
        read_code="WORKTREE_RACE",
        size_code="WORKTREE_SIZE",
        description="working-tree file",
    )


def _remaining_capture_seconds(deadline: float) -> float:
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        fail(
            "INVENTORY_TIMEOUT",
            "candidate inventory Git subprocesses exceeded their aggregate deadline",
        )
    return min(GIT_COMMAND_TIMEOUT_SECONDS, remaining)


def _require_git_repository_root(
    path: Path, relative: str, *, capture_deadline: float
) -> None:
    try:
        metadata = path.lstat()
    except OSError as exc:
        fail("GITLINK_ROOT", f"cannot inspect gitlink repository {relative}: {exc}")
    if not stat.S_ISDIR(metadata.st_mode):
        fail("GITLINK_ROOT", f"gitlink repository must be a real directory: {relative}")
    raw_top = _run_git(
        path,
        ["rev-parse", "--show-toplevel"],
        max_bytes=16 * 1024,
        timeout_seconds=_remaining_capture_seconds(capture_deadline),
    )
    try:
        observed_top = Path(raw_top.decode("utf-8", errors="strict").strip()).resolve(
            strict=True
        )
        expected_top = path.resolve(strict=True)
    except (OSError, UnicodeDecodeError) as exc:
        fail("GITLINK_ROOT", f"cannot resolve gitlink repository {relative}: {exc}")
    if observed_top != expected_top:
        fail("GITLINK_ROOT", f"gitlink path is not its repository root: {relative}")


def _working_file_record(
    repo: Path, relative: str, *, capture_deadline: float
) -> dict[str, Any]:
    path = repo / relative
    try:
        metadata = path.lstat()
    except FileNotFoundError:
        return {
            "kind": "missing",
            "mode": None,
            "sha256": None,
            "bytes": None,
            "line_count": None,
            "link_target": None,
            "gitlink_head": None,
            "gitlink_status_sha256": None,
        }
    except OSError as exc:
        fail("WORKTREE_STAT", f"cannot stat working-tree path {path}: {exc}")
    if stat.S_ISLNK(metadata.st_mode):
        try:
            target = os.readlink(path)
        except OSError as exc:
            fail("WORKTREE_READLINK", f"cannot read symlink {path}: {exc}")
        try:
            raw = os.fsencode(target)
            normalized_target = os.fsdecode(raw)
        except (UnicodeError, ValueError) as exc:
            fail("WORKTREE_LINK", f"cannot encode symlink target {path}: {exc}")
        return {
            "kind": "symlink",
            "mode": "120000",
            "sha256": sha256_bytes(raw),
            "bytes": len(raw),
            "line_count": None,
            "link_target": normalized_target,
            "gitlink_head": None,
            "gitlink_status_sha256": None,
        }
    if stat.S_ISREG(metadata.st_mode):
        raw, stable = _read_stable_file(path)
        executable = bool(stable.st_mode & 0o111)
        return {
            "kind": "regular",
            "mode": "100755" if executable else "100644",
            "sha256": sha256_bytes(raw),
            "bytes": len(raw),
            "line_count": raw.count(b"\n") + int(bool(raw) and not raw.endswith(b"\n")),
            "link_target": None,
            "gitlink_head": None,
            "gitlink_status_sha256": None,
        }
    if stat.S_ISDIR(metadata.st_mode):
        _require_git_repository_root(path, relative, capture_deadline=capture_deadline)
        raw_head = _run_git(
            path,
            ["rev-parse", "HEAD"],
            max_bytes=1024,
            timeout_seconds=_remaining_capture_seconds(capture_deadline),
        )
        try:
            gitlink_head = raw_head.decode("ascii").strip()
        except UnicodeDecodeError as exc:
            fail("GITLINK_HEAD", f"invalid gitlink HEAD at {relative}: {exc}")
        if _OID_RE.fullmatch(gitlink_head) is None:
            fail("GITLINK_HEAD", f"invalid gitlink HEAD at {relative}")
        status_raw = _run_git(
            path,
            ["status", "--porcelain=v2", "-z", "--untracked-files=all"],
            max_bytes=MAX_INVENTORY_PATH_BYTES,
            timeout_seconds=_remaining_capture_seconds(capture_deadline),
        )
        return {
            "kind": "gitlink",
            "mode": "160000",
            "sha256": None,
            "bytes": None,
            "line_count": None,
            "link_target": None,
            "gitlink_head": gitlink_head,
            "gitlink_status_sha256": sha256_bytes(status_raw),
        }
    fail("WORKTREE_TYPE", f"unsupported working-tree object: {path}")


def _blob_record(repo: Path, oid: str, *, capture_deadline: float) -> dict[str, Any]:
    raw = _run_git(
        repo,
        ["cat-file", "blob", oid],
        max_bytes=MAX_FILE_BYTES,
        timeout_seconds=_remaining_capture_seconds(capture_deadline),
    )
    return {"sha256": sha256_bytes(raw), "bytes": len(raw)}


def _pinned_working_record(mode: str, raw: bytes, *, path: str) -> dict[str, Any]:
    link_target: str | None = None
    kind = "regular"
    line_count: int | None = raw.count(b"\n") + int(
        bool(raw) and not raw.endswith(b"\n")
    )
    if mode == "120000":
        kind = "symlink"
        line_count = None
        try:
            link_target = raw.decode("utf-8", errors="strict")
        except UnicodeDecodeError as exc:
            fail("GITLINK_SYMLINK", f"non-UTF-8 pinned symlink target at {path}: {exc}")
    return {
        "kind": kind,
        "mode": mode,
        "sha256": sha256_bytes(raw),
        "bytes": len(raw),
        "line_count": line_count,
        "link_target": link_target,
        "gitlink_head": None,
        "gitlink_status_sha256": None,
    }


def _project_pinned_gitlink(
    repo: Path,
    gitlink_path: str,
    commit: str,
    *,
    capture_deadline: float,
) -> tuple[dict[str, Any], list[dict[str, Any]], int]:
    """Read a bounded immutable commit projection from one initialized gitlink."""

    if gitlink_path not in RECURSIVE_GITLINK_PATHS or _OID_RE.fullmatch(commit) is None:
        fail(
            "GITLINK_PROJECTION", f"unsupported pinned gitlink identity: {gitlink_path}"
        )
    gitlink_repo = repo / gitlink_path
    _require_git_repository_root(
        gitlink_repo, gitlink_path, capture_deadline=capture_deadline
    )
    object_type = _run_git(
        gitlink_repo,
        ["cat-file", "-t", commit],
        max_bytes=32,
        timeout_seconds=_remaining_capture_seconds(capture_deadline),
    )
    if object_type != b"commit\n":
        fail("GITLINK_COMMIT", f"pinned gitlink object is not a commit: {gitlink_path}")
    raw_tree = _run_git(
        gitlink_repo,
        ["rev-parse", f"{commit}^{{tree}}"],
        max_bytes=1024,
        timeout_seconds=_remaining_capture_seconds(capture_deadline),
    )
    try:
        tree = raw_tree.decode("ascii").strip()
    except UnicodeDecodeError as exc:
        fail("GITLINK_TREE", f"invalid pinned tree at {gitlink_path}: {exc}")
    if _OID_RE.fullmatch(tree) is None:
        fail("GITLINK_TREE", f"invalid pinned tree at {gitlink_path}")
    tree_entries = _parse_pinned_gitlink_tree(
        _run_git(
            gitlink_repo,
            ["ls-tree", "-r", "-z", commit],
            max_bytes=MAX_INVENTORY_LISTING_BYTES,
            timeout_seconds=_remaining_capture_seconds(capture_deadline),
        ),
        gitlink_path=gitlink_path,
    )
    entries: list[dict[str, Any]] = []
    identity: list[dict[str, Any]] = []
    content_bytes = 0
    for path, tree_entry in sorted(tree_entries.items()):
        raw = _run_git(
            gitlink_repo,
            ["cat-file", "blob", tree_entry["oid"]],
            max_bytes=MAX_FILE_BYTES,
            timeout_seconds=_remaining_capture_seconds(capture_deadline),
        )
        content_bytes += len(raw) * 2
        if content_bytes > MAX_INVENTORY_CONTENT_BYTES:
            fail(
                "INVENTORY_BUDGET",
                f"{gitlink_path} projected content exceeds the content budget",
            )
        working = _pinned_working_record(tree_entry["mode"], raw, path=path)
        index = {
            "mode": tree_entry["mode"],
            "oid": tree_entry["oid"],
            "content_sha256": working["sha256"],
            "bytes": working["bytes"],
        }
        origin = {
            "kind": "pinned_gitlink_file",
            "gitlink_path": gitlink_path,
            "gitlink_commit": commit,
            "gitlink_tree": tree,
        }
        entry: dict[str, Any] = {
            "path": path,
            "inventory_origin": origin,
            "head": dict(tree_entry),
            "index": index,
            "index_state": "unchanged",
            "working_tree": working,
            "worktree_state": "unchanged",
        }
        entry["file_review"] = _default_file_review(entry)
        entries.append(entry)
        identity.append(
            {
                "path": path,
                "mode": tree_entry["mode"],
                "oid": tree_entry["oid"],
                "sha256": working["sha256"],
                "bytes": working["bytes"],
            }
        )
    source_record = {
        "path": gitlink_path,
        "commit": commit,
        "tree": tree,
        "entry_count": len(entries),
        "entries_sha256": semantic_sha256(identity),
    }
    return source_record, entries, content_bytes


def _classify_file_review(path: str) -> dict[str, Any]:
    name = PurePosixPath(path).name
    suffix = PurePosixPath(path).suffix.lower()
    if path == "pid-rs":
        language = "Gitlink"
    elif suffix == ".rs":
        language = "Rust"
    elif suffix == ".py":
        language = "Python"
    elif suffix == ".md":
        language = "Markdown"
    elif suffix in {".json", ".jsonl"}:
        language = "JSON"
    elif suffix in {".yaml", ".yml"}:
        language = "YAML"
    elif suffix == ".toml" or name in {"Cargo.lock", "uv.lock"}:
        language = "TOML"
    elif suffix == ".csv":
        language = "CSV"
    elif suffix in {".sh", ".bash", ".zsh"}:
        language = "Shell"
    elif suffix == ".nix":
        language = "Nix"
    else:
        language = "binary_or_unknown"

    if path.startswith(("crates/", "pid-rs/")) or path == "pid-rs":
        category = "rust_and_estimator_source"
    elif path.startswith("experiments/"):
        category = "python_experiment_source"
    elif path.startswith("tests/"):
        category = "test_source"
    elif path.startswith("scripts/"):
        category = "governance_and_build_script"
    elif path.startswith("protocols/"):
        category = "protocol_and_truth_ledger"
    elif path.startswith(".github/"):
        category = "continuous_integration"
    elif path.startswith("docs/"):
        category = "documentation_and_review_record"
    elif path.startswith("release/"):
        category = "release_governance"
    elif suffix == ".md":
        category = "root_documentation"
    else:
        category = "repository_configuration_or_asset"

    generated = path in {
        "THIRD_PARTY_NOTICES.generated.md",
        "docs/CAPABILITY_MATRIX.md",
        "protocols/capability_matrix_current_v1.json",
    } or path.startswith(("release/0.9.0/requirements/", "release/0.9.0/review/"))
    public_surface = (
        "/src/" in path
        or path.startswith(("experiments/", "protocols/", "release/"))
        or path
        in {
            "README.md",
            "ARCHITECTURE.md",
            "CHANGELOG.md",
            "CITATION.cff",
            "EXPERIMENTS.md",
            "LIMITATIONS.md",
            "SECURITY.md",
            "grandplan.md",
            "findings.md",
            "pidsplatspecs.md",
            "Cargo.toml",
            "pyproject.toml",
        }
    )
    security_critical = (
        path.startswith((".github/", "scripts/", "crates/pid-bridge/"))
        or path.startswith("crates/ncp-observer/")
        or "bridge" in path
        or path
        in {
            "Cargo.lock",
            "Cargo.toml",
            "SECURITY.md",
            "deny.toml",
            "pyproject.toml",
            "uv.lock",
        }
    )
    science_critical = (
        path == "pid-rs"
        or path.startswith("pid-rs/")
        or path.startswith(("experiments/", "protocols/"))
        or path
        in {
            "EXPERIMENTS.md",
            "findings.md",
            "grandplan.md",
            "pidsplatspecs.md",
            "THESIS_EVIDENCE_INDEX.md",
        }
    )
    return {
        "category": category,
        "language": language,
        "generated": generated,
        "public_surface": public_surface,
        "security_critical": security_critical,
        "science_critical": science_critical,
    }


def _default_file_review(entry: Mapping[str, Any]) -> dict[str, Any]:
    index = entry["index"]
    expected_blob = None if index is None or index["mode"] == "160000" else index["oid"]
    working = entry["working_tree"]
    return {
        "path": entry["path"],
        "git_blob_id": expected_blob,
        "sha256": working["sha256"],
        **_classify_file_review(entry["path"]),
        "reviewer": None,
        "independent_reviewer": None,
        "line_count": working["line_count"],
        "requirements": [],
        "defects": [],
        "tests": [],
        "evidence": [],
        "disposition": "OPEN_NOT_REVIEWED",
        "decision": "OPEN_NOT_REVIEWED",
        "updated_at": None,
        "completed_at": None,
        "notes": (
            "Inventory only; substantive file review and requirement closure are not claimed."
        ),
    }


def _index_state(
    index_entry: Mapping[str, Any] | None,
    head_entry: Mapping[str, Any] | None,
) -> str:
    if index_entry is None:
        return "untracked" if head_entry is None else "deleted"
    if head_entry is None:
        return "added"
    if index_entry["mode"] != head_entry["mode"]:
        return "type_changed"
    if index_entry["oid"] != head_entry["oid"]:
        return "modified"
    return "unchanged"


def _worktree_state(entry: Mapping[str, Any]) -> str:
    working = entry["working_tree"]
    if working["kind"] == "missing":
        return "deleted"
    if entry["index"] is None:
        return "untracked"
    index = entry["index"]
    if working["mode"] != index["mode"]:
        return "type_changed"
    if working["kind"] == "gitlink":
        if working["gitlink_head"] == index["oid"] and working[
            "gitlink_status_sha256"
        ] == sha256_bytes(b""):
            return "unchanged"
        return "modified"
    if working["sha256"] == index["content_sha256"]:
        return "unchanged"
    return "modified"


def _evidence_bindings_from_entries(
    entries: Sequence[Mapping[str, Any]], paths: Sequence[str], *, context: str
) -> list[dict[str, Any]]:
    by_path = {entry["path"]: entry for entry in entries}
    bindings: list[dict[str, Any]] = []
    for path in paths:
        entry = by_path.get(path)
        if entry is None:
            fail(
                "PROGRESS_EVIDENCE",
                f"{context} references an absent source path: {path}",
            )
        working = entry["working_tree"]
        if working["kind"] not in {"regular", "symlink"} or working["sha256"] is None:
            fail(
                "PROGRESS_EVIDENCE", f"{context} evidence is not a source file: {path}"
            )
        bindings.append(
            {
                "path": path,
                "sha256": working["sha256"],
                "bytes": working["bytes"],
            }
        )
    return bindings


def _apply_file_review_progress(
    entries: list[dict[str, Any]], progress: Mapping[str, Any]
) -> None:
    allowed = {
        "OPEN_NOT_REVIEWED",
        "IN_PROGRESS",
        "BLOCKED",
        "ACCEPT",
        "FIXED",
        "REMOVED",
        "NOT_CLAIMED",
    }
    final = {"ACCEPT", "FIXED", "REMOVED", "NOT_CLAIMED"}
    by_path = {entry["path"]: entry for entry in entries}
    seen: set[str] = set()
    for position, raw_update in enumerate(progress["file_review_updates"]):
        update = _assert_exact_keys(
            raw_update,
            {
                "path",
                "disposition",
                "reviewer",
                "independent_reviewer",
                "updated_at",
                "requirements",
                "defects",
                "tests",
                "evidence_paths",
                "notes",
            },
            context=f"file_review_updates[{position}]",
        )
        path = _require_text(
            update["path"], context=f"file_review_updates[{position}].path"
        )
        assert isinstance(path, str)
        if path in seen or path not in by_path:
            fail(
                "PROGRESS_FILE_REVIEW", f"file-review path is duplicate/absent: {path}"
            )
        seen.add(path)
        disposition = update["disposition"]
        if disposition not in allowed:
            fail("PROGRESS_FILE_REVIEW", f"unsupported file disposition at {path}")
        if disposition in final:
            fail(
                "TERMINAL_PROMOTION_DISABLED",
                f"candidate progress schema 0.1 cannot finalize file review: {path}",
            )
        reviewer = _require_text(
            update["reviewer"], context=f"{path}.reviewer", nullable=True
        )
        independent = _require_text(
            update["independent_reviewer"],
            context=f"{path}.independent_reviewer",
            nullable=True,
        )
        _require_distinct_reviewers(reviewer, independent, context=path)
        updated_at = _require_timestamp(
            update["updated_at"], context=f"{path}.updated_at", nullable=True
        )
        requirements = _require_string_list(
            update["requirements"], context=f"{path}.requirements", allow_empty=True
        )
        defects = _require_string_list(
            update["defects"], context=f"{path}.defects", allow_empty=True
        )
        tests = _require_string_list(
            update["tests"], context=f"{path}.tests", allow_empty=True
        )
        evidence_paths = _require_string_list(
            update["evidence_paths"],
            context=f"{path}.evidence_paths",
            allow_empty=True,
        )
        notes = _require_text(update["notes"], context=f"{path}.notes", nullable=True)
        if disposition == "OPEN_NOT_REVIEWED":
            if any(
                value
                for value in (
                    reviewer,
                    independent,
                    updated_at,
                    requirements,
                    defects,
                    tests,
                    evidence_paths,
                    notes,
                )
            ):
                fail(
                    "PROGRESS_FILE_REVIEW",
                    f"open file update carries promotion data: {path}",
                )
        else:
            if (
                reviewer is None
                or updated_at is None
                or not evidence_paths
                or notes is None
            ):
                fail(
                    "PROGRESS_FILE_REVIEW",
                    f"promoted file review lacks reviewer/timestamp/evidence/notes: {path}",
                )
        if disposition == "BLOCKED" and not defects:
            fail("PROGRESS_FILE_REVIEW", f"blocked file review lacks defects: {path}")
        if disposition in final:
            if independent is None or not requirements:
                fail(
                    "PROGRESS_FILE_REVIEW",
                    f"final file review lacks requirements/independent review: {path}",
                )
            if disposition in {"ACCEPT", "FIXED"} and not tests:
                fail("PROGRESS_FILE_REVIEW", f"accepted/fixed file lacks tests: {path}")
            if disposition == "FIXED" and not defects:
                fail("PROGRESS_FILE_REVIEW", f"fixed file review lacks defects: {path}")
        review = by_path[path]["file_review"]
        review.update(
            {
                "reviewer": reviewer,
                "independent_reviewer": independent,
                "requirements": requirements,
                "defects": defects,
                "tests": tests,
                "evidence": _evidence_bindings_from_entries(
                    entries, evidence_paths, context=f"file review {path}"
                ),
                "disposition": disposition,
                "decision": disposition,
                "updated_at": updated_at,
                "completed_at": updated_at if disposition in final else None,
                "notes": notes
                if notes is not None
                else (
                    "Inventory only; substantive file review and requirement closure are not "
                    "claimed."
                ),
            }
        )


def _capture_once(repo: Path) -> dict[str, Any]:
    capture_deadline = time.monotonic() + INVENTORY_GIT_DEADLINE_SECONDS

    def capture_git(
        args: Sequence[str], *, max_bytes: int = 512 * 1024 * 1024
    ) -> bytes:
        return _run_git(
            repo,
            args,
            max_bytes=max_bytes,
            timeout_seconds=_remaining_capture_seconds(capture_deadline),
        )

    progress, progress_raw = _read_progress(repo)
    try:
        head = (
            capture_git(["rev-parse", "HEAD"], max_bytes=1024).decode("ascii").strip()
        )
        head_tree_oid = (
            capture_git(["rev-parse", "HEAD^{tree}"], max_bytes=1024)
            .decode("ascii")
            .strip()
        )
    except UnicodeDecodeError as exc:
        fail("HEAD_IDENTITY", f"Git HEAD identity is not ASCII: {exc}")
    index_entries = _parse_index(
        capture_git(
            ["ls-files", "--stage", "-z"], max_bytes=MAX_INVENTORY_LISTING_BYTES
        )
    )
    head_entries = _parse_head_tree(
        capture_git(
            ["ls-tree", "-r", "-z", "HEAD"],
            max_bytes=MAX_INVENTORY_LISTING_BYTES,
        )
    )
    raw_untracked = capture_git(
        ["ls-files", "--others", "--exclude-standard", "-z"],
        max_bytes=MAX_INVENTORY_PATH_BYTES + MAX_INVENTORY_ENTRIES,
    )
    untracked: list[str] = []
    untracked_path_bytes = 0
    for raw_path in raw_untracked.rstrip(b"\0").split(b"\0") if raw_untracked else []:
        if not raw_path:
            continue
        path = _decode_path(raw_path, context="untracked file")
        if _is_self_excluded(path):
            continue
        if len(untracked) >= MAX_INVENTORY_ENTRIES:
            fail(
                "INVENTORY_BUDGET", "untracked files exceed the inventory entry budget"
            )
        untracked_path_bytes += len(raw_path)
        if untracked_path_bytes > MAX_INVENTORY_PATH_BYTES:
            fail("INVENTORY_BUDGET", "untracked paths exceed the inventory path budget")
        untracked.append(path)
    untracked.sort()
    index_by_path = {entry["path"]: entry for entry in index_entries}
    if set(untracked) & set(index_by_path):
        fail("INVENTORY_OVERLAP", "a path is both indexed and untracked")

    entries: list[dict[str, Any]] = []
    recursive_entries: list[dict[str, Any]] = []
    recursive_gitlinks: list[dict[str, Any]] = []
    parent_source_paths = set(head_entries) | set(index_by_path) | set(untracked)
    if len(parent_source_paths) > MAX_INVENTORY_ENTRIES:
        fail(
            "INVENTORY_BUDGET",
            f"source inventory exceeds {MAX_INVENTORY_ENTRIES} entries",
        )
    path_bytes = sum(len(path.encode("utf-8")) for path in parent_source_paths)
    if path_bytes > MAX_INVENTORY_PATH_BYTES:
        fail(
            "INVENTORY_BUDGET",
            f"source inventory paths exceed {MAX_INVENTORY_PATH_BYTES} bytes",
        )
    content_bytes = 0
    for path in sorted(parent_source_paths):
        raw_index = index_by_path.get(path)
        head_entry = head_entries.get(path)
        if raw_index is None:
            index_record = None
            index_state = _index_state(None, head_entry)
        else:
            blob = (
                {"sha256": None, "bytes": None}
                if raw_index["mode"] == "160000"
                else _blob_record(
                    repo, raw_index["oid"], capture_deadline=capture_deadline
                )
            )
            index_record = {
                "mode": raw_index["mode"],
                "oid": raw_index["oid"],
                "content_sha256": blob["sha256"],
                "bytes": blob["bytes"],
            }
            if blob["bytes"] is not None:
                content_bytes += blob["bytes"]
                if content_bytes > MAX_INVENTORY_CONTENT_BYTES:
                    fail(
                        "INVENTORY_BUDGET",
                        "indexed and working-tree content exceeds the inventory budget",
                    )
            index_state = _index_state(raw_index, head_entry)
        working = _working_file_record(repo, path, capture_deadline=capture_deadline)
        if working["bytes"] is not None:
            content_bytes += working["bytes"]
            if content_bytes > MAX_INVENTORY_CONTENT_BYTES:
                fail(
                    "INVENTORY_BUDGET",
                    "indexed and working-tree content exceeds the inventory budget",
                )
        if raw_index is not None and raw_index["mode"] == "160000":
            if working["kind"] != "gitlink":
                fail("GITLINK_TYPE", f"indexed gitlink is not a repository: {path}")
            if working["gitlink_head"] != raw_index["oid"] or working[
                "gitlink_status_sha256"
            ] != sha256_bytes(b""):
                fail(
                    "GITLINK_DIRTY",
                    f"non-exact or dirty gitlink cannot be content-bound: {path}",
                )
        entry: dict[str, Any] = {
            "path": path,
            "inventory_origin": {
                "kind": "parent_repository",
                "gitlink_path": None,
                "gitlink_commit": None,
                "gitlink_tree": None,
            },
            "head": None if head_entry is None else dict(head_entry),
            "index": index_record,
            "index_state": index_state,
            "working_tree": working,
        }
        entry["worktree_state"] = _worktree_state(entry)
        entry["file_review"] = _default_file_review(entry)
        entries.append(entry)
        if raw_index is not None and path in RECURSIVE_GITLINK_PATHS:
            if raw_index["mode"] != "160000":
                fail(
                    "GITLINK_TYPE", f"recursive inventory root is not a gitlink: {path}"
                )
            source_record, projected, projected_content_bytes = _project_pinned_gitlink(
                repo,
                path,
                raw_index["oid"],
                capture_deadline=capture_deadline,
            )
            recursive_gitlinks.append(source_record)
            recursive_entries.extend(projected)
            content_bytes += projected_content_bytes
            if content_bytes > MAX_INVENTORY_CONTENT_BYTES:
                fail(
                    "INVENTORY_BUDGET",
                    "indexed and working-tree content exceeds the inventory budget",
                )
    entries.extend(recursive_entries)
    entries.sort(key=lambda item: item["path"])
    all_paths = [entry["path"] for entry in entries]
    if len(all_paths) > MAX_INVENTORY_ENTRIES:
        fail(
            "INVENTORY_BUDGET",
            f"recursive source inventory exceeds {MAX_INVENTORY_ENTRIES} entries",
        )
    path_bytes = sum(len(path.encode("utf-8")) for path in all_paths)
    if path_bytes > MAX_INVENTORY_PATH_BYTES:
        fail(
            "INVENTORY_BUDGET",
            f"recursive source paths exceed {MAX_INVENTORY_PATH_BYTES} bytes",
        )
    if len(all_paths) != len(set(all_paths)):
        fail("INVENTORY_OVERLAP", "parent and recursive source paths overlap")
    _apply_file_review_progress(entries, progress)

    index_identity = [
        {
            "path": entry["path"],
            "mode": entry["mode"],
            "oid": entry["oid"],
            "stage": entry["stage"],
        }
        for entry in index_entries
    ]
    index_sha256 = semantic_sha256(index_identity)
    working_identity = [
        {
            "path": entry["path"],
            "inventory_origin": entry["inventory_origin"],
            "head": entry["head"],
            "index": entry["index"],
            "index_state": entry["index_state"],
            "working_tree": entry["working_tree"],
            "worktree_state": entry["worktree_state"],
        }
        for entry in entries
    ]
    worktree_sha256 = semantic_sha256(working_identity)
    parent_entries = [
        entry
        for entry in entries
        if entry["inventory_origin"]["kind"] == "parent_repository"
    ]
    modified_index_count = sum(
        entry["index_state"] != "unchanged" for entry in parent_entries
    )
    modified_worktree_count = sum(
        entry["worktree_state"] != "unchanged" for entry in parent_entries
    )
    untracked_count = sum(
        entry["index_state"] == "untracked" for entry in parent_entries
    )
    clean = modified_index_count == 0 and modified_worktree_count == 0
    state_material = {
        "head_commit": head,
        "head_tree": head_tree_oid,
        "index_entries_sha256": index_sha256,
        "worktree_entries_sha256": worktree_sha256,
        "recursive_gitlinks": recursive_gitlinks,
        "self_exclusions": [CANDIDATE_RELATIVE],
        "progress_snapshot": {
            "path": PROGRESS_RELATIVE,
            "sha256": sha256_bytes(progress_raw),
            "bytes": len(progress_raw),
            "semantic_sha256": semantic_sha256(progress),
            "document": progress,
        },
        "entries": entries,
    }
    candidate_state_sha256 = semantic_sha256(state_material)
    return {
        "schema_version": SCHEMA_VERSION,
        "record_type": "candidate_source_inventory",
        "project": PROJECT,
        "repository": REPOSITORY,
        "release_version": RELEASE_VERSION,
        "source": {
            "head_commit": head,
            "head_tree": head_tree_oid,
            "index_entries_sha256": index_sha256,
            "worktree_entries_sha256": worktree_sha256,
            "candidate_state_sha256": candidate_state_sha256,
            "clean": clean,
            "state": (
                "clean_source_snapshot"
                if clean
                else "dirty_uncommitted_source_snapshot"
            ),
            "explicit_source_arguments_required": True,
            "recursive_gitlinks": recursive_gitlinks,
        },
        "inventory_policy": {
            "basis": (
                "git_head_plus_index_plus_worktree_nonignored_untracked_and_pinned_"
                "gitlink_commit_trees"
            ),
            "self_excluded_paths": [CANDIDATE_RELATIVE],
            "self_exclusion_reason": (
                "candidate artifacts cannot inventory themselves; artifact_manifest.json "
                "binds every generated candidate artifact"
            ),
            "ignored_files_included": False,
            "dirty_gitlinks_permitted": False,
            "recursive_gitlink_paths": [
                record["path"] for record in recursive_gitlinks
            ],
            "recursive_rows_derived_from": "pinned_index_commit_git_objects",
            "stable_double_capture_required": True,
            "resource_limits": {
                "max_entry_count": MAX_INVENTORY_ENTRIES,
                "max_path_bytes": MAX_INVENTORY_PATH_BYTES,
                "max_git_object_bytes_per_blob": MAX_FILE_BYTES,
                "max_recursive_gitlink_count": len(RECURSIVE_GITLINK_PATHS),
                "max_index_and_worktree_content_bytes": MAX_INVENTORY_CONTENT_BYTES,
                "max_generated_artifact_bytes": MAX_CANDIDATE_ARTIFACT_BYTES,
                "git_subprocess_deadline_seconds_per_capture": (
                    INVENTORY_GIT_DEADLINE_SECONDS
                ),
            },
            "fixed_point_semantics": {
                "excluded_namespace": CANDIDATE_RELATIVE,
                "source_digest_excludes_candidate_outputs": True,
                "candidate_outputs_bound_by": ARTIFACT_MANIFEST_NAME,
                "regeneration_property": (
                    "For unchanged non-candidate source bytes, index identity, and pinned "
                    "gitlink commit objects, regeneration is byte-deterministic and changes "
                    "only the excluded candidate namespace."
                ),
                "review_boundary": (
                    "Candidate output files are governed artifacts, not source files silently "
                    "counted as reviewed."
                ),
            },
        },
        "progress_snapshot": state_material["progress_snapshot"],
        "summary": {
            "entry_count": len(entries),
            "parent_entry_count": len(parent_entries),
            "recursive_gitlink_entry_count": len(recursive_entries),
            "indexed_entry_count": len(index_entries),
            "untracked_entry_count": untracked_count,
            "index_changed_entry_count": modified_index_count,
            "worktree_changed_entry_count": modified_worktree_count,
            "clean": clean,
        },
        "file_review_template_contract": {
            "source_path": "repo_work/templates/FILE_REVIEW_LEDGER.csv",
            "source_sha256": (
                "273dd86b512ab55808ce9fe7911ae557d00a2da511e0235277bed5bde5d47342"
            ),
            "fields": [
                "path",
                "git_blob_id",
                "sha256",
                "category",
                "language",
                "generated",
                "public_surface",
                "security_critical",
                "science_critical",
                "reviewer",
                "line_count",
                "requirements",
                "defects",
                "tests",
                "evidence",
                "disposition",
                "completed_at",
                "notes",
            ],
            "extended_fields": [
                "independent_reviewer",
                "decision",
                "updated_at",
            ],
            "generated_defaults_unreviewed": True,
            "progress_update_count": len(progress["file_review_updates"]),
            "terminal_promotion_enabled": False,
            "terminal_promotion_policy": TERMINAL_PROMOTION_POLICY,
        },
        "entries": entries,
        "candidate_boundary": (
            "This is a content inventory of an explicit source snapshot. A dirty snapshot "
            "is not a commit, tag, published release, verification result, or readiness claim."
        ),
    }


def capture_stable_inventory(repo: Path) -> dict[str, Any]:
    first = _capture_once(repo)
    second = _capture_once(repo)
    if first != second:
        fail(
            "SOURCE_STATE_RACE",
            "repository state changed across the stable double capture",
        )
    return first


def source_state_arguments(inventory: Mapping[str, Any]) -> dict[str, Any]:
    source = inventory["source"]
    return {
        "source_head": source["head_commit"],
        "source_index_sha256": source["index_entries_sha256"],
        "source_worktree_sha256": source["worktree_entries_sha256"],
        "candidate_state_sha256": source["candidate_state_sha256"],
        "clean": source["clean"],
    }


def inventory_with_progress(
    inventory: Mapping[str, Any], progress: Mapping[str, Any]
) -> dict[str, Any]:
    """Return a pure synthetic inventory with a different canonical progress input.

    This supports deterministic model tests. Production generation always reads the real
    source-controlled progress file through ``capture_stable_inventory``.
    """

    raw = pretty_json_bytes(progress)
    validated_progress, _ = _validate_progress_document(
        json.loads(canonical_json_bytes(progress)), raw
    )
    progress = validated_progress
    updated = json.loads(canonical_json_bytes(inventory))
    entries = updated["entries"]
    progress_entry = next(
        (entry for entry in entries if entry["path"] == PROGRESS_RELATIVE), None
    )
    if progress_entry is None or progress_entry["working_tree"]["kind"] != "regular":
        fail("PROGRESS_INVENTORY", "candidate progress input is absent from inventory")
    working = progress_entry["working_tree"]
    working.update(
        {
            "sha256": sha256_bytes(raw),
            "bytes": len(raw),
            "line_count": raw.count(b"\n") + int(bool(raw) and not raw.endswith(b"\n")),
        }
    )
    progress_entry["worktree_state"] = _worktree_state(progress_entry)
    progress_entry["file_review"]["sha256"] = working["sha256"]
    progress_entry["file_review"]["line_count"] = working["line_count"]
    _apply_file_review_progress(entries, progress)
    snapshot = {
        "path": PROGRESS_RELATIVE,
        "sha256": sha256_bytes(raw),
        "bytes": len(raw),
        "semantic_sha256": semantic_sha256(progress),
        "document": progress,
    }
    updated["progress_snapshot"] = snapshot
    updated["file_review_template_contract"]["progress_update_count"] = len(
        progress["file_review_updates"]
    )
    working_identity = [
        {
            "path": entry["path"],
            "inventory_origin": entry["inventory_origin"],
            "head": entry["head"],
            "index": entry["index"],
            "index_state": entry["index_state"],
            "working_tree": entry["working_tree"],
            "worktree_state": entry["worktree_state"],
        }
        for entry in entries
    ]
    source = updated["source"]
    source["worktree_entries_sha256"] = semantic_sha256(working_identity)
    parent_entries = [
        entry
        for entry in entries
        if entry["inventory_origin"]["kind"] == "parent_repository"
    ]
    recursive_entries = [
        entry
        for entry in entries
        if entry["inventory_origin"]["kind"] == "pinned_gitlink_file"
    ]
    index_changed = sum(entry["index_state"] != "unchanged" for entry in parent_entries)
    worktree_changed = sum(
        entry["worktree_state"] != "unchanged" for entry in parent_entries
    )
    clean = index_changed == 0 and worktree_changed == 0
    source["clean"] = clean
    source["state"] = (
        "clean_source_snapshot" if clean else "dirty_uncommitted_source_snapshot"
    )
    updated["summary"].update(
        {
            "entry_count": len(entries),
            "parent_entry_count": len(parent_entries),
            "recursive_gitlink_entry_count": len(recursive_entries),
            "indexed_entry_count": sum(
                entry["index"] is not None for entry in parent_entries
            ),
            "untracked_entry_count": sum(
                entry["index_state"] == "untracked" for entry in parent_entries
            ),
            "index_changed_entry_count": index_changed,
            "worktree_changed_entry_count": worktree_changed,
            "clean": clean,
        }
    )
    state_material = {
        "head_commit": source["head_commit"],
        "head_tree": source["head_tree"],
        "index_entries_sha256": source["index_entries_sha256"],
        "worktree_entries_sha256": source["worktree_entries_sha256"],
        "recursive_gitlinks": source["recursive_gitlinks"],
        "self_exclusions": [CANDIDATE_RELATIVE],
        "progress_snapshot": snapshot,
        "entries": entries,
    }
    source["candidate_state_sha256"] = semantic_sha256(state_material)
    return updated


def _read_baseline(repo: Path) -> tuple[dict[str, Any], bytes]:
    path = repo / BASELINE_RELATIVE
    raw, _ = _read_bounded_regular(
        path,
        max_bytes=MAX_BASELINE_BYTES,
        path_code="BASELINE_PATH",
        read_code="BASELINE_READ",
        size_code="BASELINE_SIZE",
        description="immutable requirements baseline",
    )
    if sha256_bytes(raw) != BASELINE_SHA256:
        fail("BASELINE_SHA256", "immutable requirements baseline identity changed")
    try:
        value = json.loads(raw)
    except (json.JSONDecodeError, UnicodeDecodeError, RecursionError) as exc:
        fail("BASELINE_JSON", f"cannot parse immutable requirements baseline: {exc}")
    if not isinstance(value, dict):
        fail("BASELINE_JSON", "immutable requirements baseline root is not an object")
    if pretty_json_bytes(value) != raw:
        fail("BASELINE_CANONICAL", "immutable requirements baseline is not canonical")
    if (
        value.get("task_count") != EXPECTED_TASK_COUNT
        or value.get("lens_count") != EXPECTED_LENS_COUNT
        or value.get("lens_disposition_count") != EXPECTED_LENS_DISPOSITION_COUNT
        or value.get("source", {}).get("requirements_semantic_sha256")
        != BASELINE_REQUIREMENTS_SHA256
    ):
        fail(
            "BASELINE_SEMANTICS",
            "immutable requirements baseline counts or identity drifted",
        )
    return value, raw


def build_task_ledger(
    baseline: Mapping[str, Any],
    baseline_raw: bytes,
    inventory: Mapping[str, Any],
    progress: Mapping[str, Any],
    receipts: Mapping[str, Any],
) -> dict[str, Any]:
    phases = []
    for phase in baseline["phases"]:
        phases.append(
            {
                "id": phase["id"],
                "title": phase["title"],
                "execution_wave": phase["execution_wave"],
                "task_ids": phase["task_ids"],
                "status": "open",
            }
        )
    tasks = []
    for baseline_task in baseline["tasks"]:
        requirement = baseline_task["requirements"]
        lenses = []
        for lens in baseline_task["lens_requirements"]:
            lenses.append(
                {
                    "lens_id": lens["id"],
                    "name": lens["name"],
                    "baseline_requirement": lens,
                    "status": "open",
                    "finding": None,
                    "evidence_receipt_ids": [],
                    "evidence": [],
                    "blockers": [],
                    "reviewer": None,
                    "independent_reviewer": None,
                    "decision": None,
                    "updated_at": None,
                    "reviewed_at": None,
                }
            )
        tasks.append(
            {
                "id": requirement["id"],
                "phase_id": requirement["phase_id"],
                "title": requirement["title"],
                "priority": requirement["priority"],
                "dependencies": requirement["dependencies"],
                "baseline_requirements": requirement,
                "baseline_source_block_sha256": requirement["source_block"]["sha256"],
                "status": "open",
                "decision": None,
                "evidence_receipt_ids": [],
                "evidence": [],
                "blockers": [],
                "claim_impact": None,
                "owner": None,
                "reviewer": None,
                "independent_reviewer": None,
                "completed_at": None,
                "updated_at": None,
                "lenses": lenses,
            }
        )
    document = {
        "schema_version": SCHEMA_VERSION,
        "record_type": "candidate_live_task_lens_ledger",
        "project": PROJECT,
        "release_version": RELEASE_VERSION,
        "source_state_sha256": inventory["source"]["candidate_state_sha256"],
        "baseline": {
            "path": BASELINE_RELATIVE,
            "sha256": sha256_bytes(baseline_raw),
            "bytes": len(baseline_raw),
            "requirements_semantic_sha256": BASELINE_REQUIREMENTS_SHA256,
            "immutable": True,
        },
        "status_contract": {
            "task_statuses": [
                "open",
                "in_progress",
                "blocked",
                "closed",
                "claim_removed",
            ],
            "lens_statuses": [
                "open",
                "in_progress",
                "blocked",
                "closed",
                "claim_removed",
            ],
            "terminal_promotion_enabled": False,
            "terminal_promotion_policy": TERMINAL_PROMOTION_POLICY,
            "closure_rule": (
                "Terminal states are reserved for a successor schema that binds every typed "
                "task obligation to authenticated evidence; schema 0.1 records only open, "
                "in-progress, blocked, and wave-rework state"
            ),
        },
        "summary": {
            "phase_count": len(phases),
            "task_count": len(tasks),
            "open_task_count": len(tasks),
            "partial_task_count": 0,
            "blocked_task_count": 0,
            "closed_task_count": 0,
            "lens_disposition_count": len(tasks) * EXPECTED_LENS_COUNT,
            "open_lens_disposition_count": len(tasks) * EXPECTED_LENS_COUNT,
            "partial_lens_disposition_count": 0,
            "blocked_lens_disposition_count": 0,
            "closed_lens_disposition_count": 0,
            "all_tasks_closed": False,
            "release_gate_passed": False,
        },
        "phases": phases,
        "tasks": tasks,
        "boundary": (
            "This live ledger is derived from the immutable handoff baseline. Task and lens "
            "dispositions default to open, and only validated explicit progress changes them; "
            "no implementation, review, or closure is inferred."
        ),
    }
    _apply_task_lens_progress(document, inventory, progress, receipts)
    return document


def _apply_task_lens_progress(
    document: dict[str, Any],
    inventory: Mapping[str, Any],
    progress: Mapping[str, Any],
    receipts: Mapping[str, Any],
) -> None:
    task_updates = _updates_by_id(
        progress["task_updates"],
        id_field="task_id",
        expected_keys={
            "task_id",
            "status",
            "decision",
            "owner",
            "reviewer",
            "independent_reviewer",
            "updated_at",
            "evidence_paths",
            "evidence_receipt_ids",
            "blockers",
            "claim_impact",
        },
        context="task_updates",
    )
    tasks = {task["id"]: task for task in document["tasks"]}
    allowed = set(document["status_contract"]["task_statuses"])
    for task_id, update in task_updates.items():
        task = tasks.get(task_id)
        if task is None:
            fail("PROGRESS_TASK", f"unknown task update: {task_id}")
        status_value = update["status"]
        if status_value not in allowed:
            fail("PROGRESS_TASK", f"unsupported task status: {status_value}")
        if status_value in {"closed", "claim_removed"}:
            fail(
                "TERMINAL_PROMOTION_DISABLED",
                f"candidate progress schema 0.1 cannot finalize task: {task_id}",
            )
        decision = _require_text(
            update["decision"], context=f"{task_id}.decision", nullable=True
        )
        owner = _require_text(
            update["owner"], context=f"{task_id}.owner", nullable=True
        )
        reviewer = _require_text(
            update["reviewer"], context=f"{task_id}.reviewer", nullable=True
        )
        independent = _require_text(
            update["independent_reviewer"],
            context=f"{task_id}.independent_reviewer",
            nullable=True,
        )
        _require_distinct_reviewers(reviewer, independent, context=task_id)
        updated_at = _require_timestamp(
            update["updated_at"], context=f"{task_id}.updated_at", nullable=True
        )
        evidence_paths = _require_string_list(
            update["evidence_paths"],
            context=f"{task_id}.evidence_paths",
            allow_empty=True,
        )
        blockers = _require_string_list(
            update["blockers"], context=f"{task_id}.blockers", allow_empty=True
        )
        claim_impact = _require_text(
            update["claim_impact"], context=f"{task_id}.claim_impact", nullable=True
        )
        receipt_refs = _validated_receipt_refs(
            update["evidence_receipt_ids"],
            receipts,
            context=f"{task_id}.evidence_receipt_ids",
            require_passed=status_value == "closed",
            allow_empty=status_value != "closed",
            consumer_updated_at=updated_at,
        )
        if status_value == "open":
            if any(
                value
                for value in (
                    decision,
                    owner,
                    reviewer,
                    independent,
                    updated_at,
                    evidence_paths,
                    receipt_refs,
                    blockers,
                    claim_impact,
                )
            ):
                fail(
                    "PROGRESS_TASK",
                    f"open task update carries promotion data: {task_id}",
                )
        else:
            if (
                decision is None
                or reviewer is None
                or updated_at is None
                or not evidence_paths
            ):
                fail(
                    "PROGRESS_TASK",
                    f"promoted task lacks decision/reviewer/timestamp/evidence: {task_id}",
                )
        if status_value == "in_progress" and (
            decision != "WORK_STARTED" or owner is None
        ):
            fail("PROGRESS_TASK", f"in-progress task lacks owner/decision: {task_id}")
        if status_value == "blocked" and (decision != "BLOCKED" or not blockers):
            fail("PROGRESS_TASK", f"blocked task lacks blocker/decision: {task_id}")
        if status_value == "closed" and (
            decision != "ACCEPTED" or owner is None or independent is None
        ):
            fail(
                "PROGRESS_TASK",
                f"closed task lacks accepted independent review: {task_id}",
            )
        if status_value == "claim_removed" and (
            decision != "CLAIM_REMOVED" or independent is None or claim_impact is None
        ):
            fail("PROGRESS_TASK", f"claim-removed task lacks review/impact: {task_id}")
        task.update(
            {
                "status": status_value,
                "decision": decision,
                "owner": owner,
                "reviewer": reviewer,
                "independent_reviewer": independent,
                "updated_at": updated_at,
                "completed_at": (
                    updated_at if status_value in {"closed", "claim_removed"} else None
                ),
                "evidence": _inventory_evidence(inventory, evidence_paths),
                "evidence_receipt_ids": receipt_refs,
                "blockers": blockers,
                "claim_impact": claim_impact,
            }
        )

    seen_lenses: set[tuple[str, str]] = set()
    for position, raw_update in enumerate(progress["lens_updates"]):
        update = _assert_exact_keys(
            raw_update,
            {
                "task_id",
                "lens_id",
                "status",
                "decision",
                "reviewer",
                "independent_reviewer",
                "updated_at",
                "finding",
                "evidence_paths",
                "evidence_receipt_ids",
                "blockers",
            },
            context=f"lens_updates[{position}]",
        )
        task_id = _require_text(
            update["task_id"], context=f"lens_updates[{position}].task_id"
        )
        lens_id = _require_text(
            update["lens_id"], context=f"lens_updates[{position}].lens_id"
        )
        assert isinstance(task_id, str) and isinstance(lens_id, str)
        key = (task_id, lens_id)
        task = tasks.get(task_id)
        if task is None or key in seen_lenses:
            fail("PROGRESS_LENS", f"unknown/duplicate lens update: {task_id}.{lens_id}")
        seen_lenses.add(key)
        lens = next(
            (item for item in task["lenses"] if item["lens_id"] == lens_id), None
        )
        if lens is None:
            fail("PROGRESS_LENS", f"unknown lens update: {task_id}.{lens_id}")
        status_value = update["status"]
        if status_value not in document["status_contract"]["lens_statuses"]:
            fail("PROGRESS_LENS", f"unsupported lens status: {status_value}")
        if status_value in {"closed", "claim_removed"}:
            fail(
                "TERMINAL_PROMOTION_DISABLED",
                "candidate progress schema 0.1 cannot finalize lens: "
                f"{task_id}.{lens_id}",
            )
        decision = _require_text(
            update["decision"], context=f"{task_id}.{lens_id}.decision", nullable=True
        )
        reviewer = _require_text(
            update["reviewer"], context=f"{task_id}.{lens_id}.reviewer", nullable=True
        )
        independent = _require_text(
            update["independent_reviewer"],
            context=f"{task_id}.{lens_id}.independent_reviewer",
            nullable=True,
        )
        _require_distinct_reviewers(
            reviewer, independent, context=f"{task_id}.{lens_id}"
        )
        updated_at = _require_timestamp(
            update["updated_at"],
            context=f"{task_id}.{lens_id}.updated_at",
            nullable=True,
        )
        finding = _require_text(
            update["finding"], context=f"{task_id}.{lens_id}.finding", nullable=True
        )
        evidence_paths = _require_string_list(
            update["evidence_paths"],
            context=f"{task_id}.{lens_id}.evidence_paths",
            allow_empty=True,
        )
        blockers = _require_string_list(
            update["blockers"],
            context=f"{task_id}.{lens_id}.blockers",
            allow_empty=True,
        )
        receipt_refs = _validated_receipt_refs(
            update["evidence_receipt_ids"],
            receipts,
            context=f"{task_id}.{lens_id}.evidence_receipt_ids",
            require_passed=status_value == "closed",
            allow_empty=status_value != "closed",
            consumer_updated_at=updated_at,
        )
        if status_value == "open":
            if any(
                value
                for value in (
                    decision,
                    reviewer,
                    independent,
                    updated_at,
                    finding,
                    evidence_paths,
                    receipt_refs,
                    blockers,
                )
            ):
                fail("PROGRESS_LENS", f"open lens update carries promotion data: {key}")
        elif (
            decision is None
            or reviewer is None
            or updated_at is None
            or finding is None
            or not evidence_paths
        ):
            fail(
                "PROGRESS_LENS",
                f"promoted lens lacks decision/reviewer/timestamp/finding/evidence: {key}",
            )
        if status_value == "in_progress" and decision != "WORK_STARTED":
            fail("PROGRESS_LENS", f"in-progress lens decision is wrong: {key}")
        if status_value == "blocked" and (decision != "BLOCKED" or not blockers):
            fail("PROGRESS_LENS", f"blocked lens lacks blocker/decision: {key}")
        if status_value == "closed" and (decision != "ACCEPTED" or independent is None):
            fail("PROGRESS_LENS", f"closed lens lacks independent acceptance: {key}")
        if status_value == "claim_removed" and (
            decision != "CLAIM_REMOVED" or independent is None
        ):
            fail("PROGRESS_LENS", f"claim-removed lens lacks independent review: {key}")
        lens.update(
            {
                "status": status_value,
                "decision": decision,
                "reviewer": reviewer,
                "independent_reviewer": independent,
                "updated_at": updated_at,
                "reviewed_at": (
                    updated_at if status_value in {"closed", "claim_removed"} else None
                ),
                "finding": finding,
                "evidence": _inventory_evidence(inventory, evidence_paths),
                "evidence_receipt_ids": receipt_refs,
                "blockers": blockers,
            }
        )

    phases = {phase["id"]: phase for phase in document["phases"]}
    wave_receipts: list[dict[str, Any]] = []
    seen_waves: set[str] = set()
    for position, raw_receipt in enumerate(progress["wave_receipts"]):
        receipt = _assert_exact_keys(
            raw_receipt,
            {
                "wave_id",
                "decision",
                "reviewer",
                "independent_reviewer",
                "updated_at",
                "evidence_paths",
                "evidence_receipt_ids",
                "task_ids",
                "notes",
            },
            context=f"wave_receipts[{position}]",
        )
        wave_id = _require_text(
            receipt["wave_id"], context=f"wave_receipts[{position}].wave_id"
        )
        assert isinstance(wave_id, str)
        phase = phases.get(wave_id)
        if phase is None or wave_id in seen_waves:
            fail("PROGRESS_WAVE", f"unknown/duplicate wave receipt: {wave_id}")
        seen_waves.add(wave_id)
        decision = receipt["decision"]
        if decision not in {"WAVE_ACCEPTED", "WAVE_REWORK", "CLAIM_REMOVED"}:
            fail("PROGRESS_WAVE", f"unsupported wave decision: {decision}")
        if decision in {"WAVE_ACCEPTED", "CLAIM_REMOVED"}:
            fail(
                "TERMINAL_PROMOTION_DISABLED",
                f"candidate progress schema 0.1 cannot finalize wave: {wave_id}",
            )
        reviewer = _require_text(receipt["reviewer"], context=f"{wave_id}.reviewer")
        independent = _require_text(
            receipt["independent_reviewer"], context=f"{wave_id}.independent_reviewer"
        )
        assert isinstance(reviewer, str) and isinstance(independent, str)
        _require_distinct_reviewers(reviewer, independent, context=wave_id)
        updated_at = _require_timestamp(
            receipt["updated_at"], context=f"{wave_id}.updated_at"
        )
        notes = _require_text(receipt["notes"], context=f"{wave_id}.notes")
        evidence_paths = _require_string_list(
            receipt["evidence_paths"],
            context=f"{wave_id}.evidence_paths",
            allow_empty=False,
        )
        task_ids = _require_string_list(
            receipt["task_ids"], context=f"{wave_id}.task_ids", allow_empty=False
        )
        if task_ids != phase["task_ids"]:
            fail("PROGRESS_WAVE", f"wave task coverage differs: {wave_id}")
        receipt_refs = _validated_receipt_refs(
            receipt["evidence_receipt_ids"],
            receipts,
            context=f"{wave_id}.evidence_receipt_ids",
            require_passed=decision == "WAVE_ACCEPTED",
            allow_empty=decision != "WAVE_ACCEPTED",
            consumer_updated_at=updated_at,
        )
        members = [tasks[task_id] for task_id in task_ids]
        if decision == "WAVE_ACCEPTED" and not all(
            task["status"] in {"closed", "claim_removed"}
            and all(
                lens["status"] in {"closed", "claim_removed"} for lens in task["lenses"]
            )
            for task in members
        ):
            fail(
                "PROGRESS_WAVE", f"accepted wave has unfinished dispositions: {wave_id}"
            )
        if decision == "CLAIM_REMOVED" and not all(
            task["status"] == "claim_removed"
            and all(lens["status"] == "claim_removed" for lens in task["lenses"])
            for task in members
        ):
            fail(
                "PROGRESS_WAVE",
                f"removed wave has retained task or lens dispositions: {wave_id}",
            )
        wave_receipts.append(
            {
                "wave_id": wave_id,
                "decision": decision,
                "reviewer": reviewer,
                "independent_reviewer": independent,
                "updated_at": updated_at,
                "evidence": _inventory_evidence(inventory, evidence_paths),
                "evidence_receipt_ids": receipt_refs,
                "task_ids": task_ids,
                "notes": notes,
            }
        )
    document["wave_receipts"] = wave_receipts

    task_counts = Counter(task["status"] for task in document["tasks"])
    lens_counts = Counter(
        lens["status"] for task in document["tasks"] for lens in task["lenses"]
    )
    final_statuses = {"closed", "claim_removed"}
    all_tasks_final = all(
        task["status"] in final_statuses
        and all(lens["status"] in final_statuses for lens in task["lenses"])
        for task in document["tasks"]
    )
    for phase in document["phases"]:
        statuses = [tasks[task_id]["status"] for task_id in phase["task_ids"]]
        wave = next(
            (item for item in wave_receipts if item["wave_id"] == phase["id"]), None
        )
        if wave is not None and wave["decision"] == "CLAIM_REMOVED":
            phase["status"] = "claim_removed"
        elif wave is not None and wave["decision"] == "WAVE_ACCEPTED":
            phase["status"] = "closed"
        elif "blocked" in statuses:
            phase["status"] = "blocked"
        elif any(status != "open" for status in statuses):
            phase["status"] = "in_progress"
        else:
            phase["status"] = "open"
    document["summary"] = {
        "phase_count": len(document["phases"]),
        "task_count": len(document["tasks"]),
        "open_task_count": task_counts["open"],
        "in_progress_task_count": task_counts["in_progress"],
        "blocked_task_count": task_counts["blocked"],
        "closed_task_count": task_counts["closed"],
        "claim_removed_task_count": task_counts["claim_removed"],
        "lens_disposition_count": sum(lens_counts.values()),
        "open_lens_disposition_count": lens_counts["open"],
        "in_progress_lens_disposition_count": lens_counts["in_progress"],
        "blocked_lens_disposition_count": lens_counts["blocked"],
        "closed_lens_disposition_count": lens_counts["closed"],
        "claim_removed_lens_disposition_count": lens_counts["claim_removed"],
        "wave_receipt_count": len(wave_receipts),
        "wave_accepted_count": sum(
            receipt["decision"] == "WAVE_ACCEPTED" for receipt in wave_receipts
        ),
        "wave_rework_count": sum(
            receipt["decision"] == "WAVE_REWORK" for receipt in wave_receipts
        ),
        "claim_removed_wave_count": sum(
            receipt["decision"] == "CLAIM_REMOVED" for receipt in wave_receipts
        ),
        "all_tasks_closed_or_claim_removed": all_tasks_final,
        "release_gate_passed": False,
    }


def _inventory_evidence(
    inventory: Mapping[str, Any], paths: Sequence[str]
) -> list[dict[str, Any]]:
    entries = {entry["path"]: entry for entry in inventory["entries"]}
    evidence: list[dict[str, Any]] = []
    for path in paths:
        entry = entries.get(path)
        if entry is None:
            fail(
                "CLAIM_EVIDENCE_PATH",
                f"claim evidence is absent from inventory: {path}",
            )
        working = entry["working_tree"]
        if working["kind"] not in {"regular", "symlink"} or working["sha256"] is None:
            fail("CLAIM_EVIDENCE_TYPE", f"claim evidence is not a file: {path}")
        evidence.append(
            {
                "path": path,
                "sha256": working["sha256"],
                "bytes": working["bytes"],
                "inventory_entry_sha256": semantic_sha256(entry),
            }
        )
    return evidence


def _updates_by_id(
    raw_updates: Sequence[Any], *, id_field: str, expected_keys: set[str], context: str
) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for position, raw_update in enumerate(raw_updates):
        update = _assert_exact_keys(
            raw_update, expected_keys, context=f"{context}[{position}]"
        )
        identifier = _require_text(
            update[id_field], context=f"{context}[{position}].{id_field}"
        )
        assert isinstance(identifier, str)
        if identifier in result:
            fail("PROGRESS_DUPLICATE", f"{context} repeats {identifier}")
        result[identifier] = update
    return result


def _validated_receipt_refs(
    raw: Any,
    receipts: Mapping[str, Any],
    *,
    context: str,
    require_passed: bool,
    allow_empty: bool,
    consumer_updated_at: str | None,
) -> list[str]:
    references = _require_string_list(raw, context=context, allow_empty=allow_empty)
    by_id = {receipt["id"]: receipt for receipt in receipts["receipts"]}
    for receipt_id in references:
        receipt = by_id.get(receipt_id)
        if receipt is None:
            fail("PROGRESS_RECEIPT_REF", f"{context} references unknown {receipt_id}")
        if require_passed and receipt["status"] != "passed":
            fail("PROGRESS_RECEIPT_REF", f"{context} references unpassed {receipt_id}")
        completed_at = receipt.get("execution", {}).get("completed_at")
        if (
            consumer_updated_at is not None
            and completed_at is not None
            and consumer_updated_at < completed_at
        ):
            fail(
                "PROGRESS_RECEIPT_CHRONOLOGY",
                f"{context} predates referenced receipt completion: {receipt_id}",
            )
    return references


def build_claim_ledger(
    inventory: Mapping[str, Any],
    progress: Mapping[str, Any],
    receipts: Mapping[str, Any],
) -> dict[str, Any]:
    software_specs = [
        {
            "claim_id": "SW09-001",
            "title": "0.9 source-preview identity",
            "claim_text": (
                "Candidate metadata identifies Prisoma 0.9.0 as an unpublished source and "
                "software preview authored by Sepehr Mahmoudian, with no DOI or Zenodo record."
            ),
            "code_paths": [],
            "test_paths": ["tests/python/test_release_review.py"],
            "evidence_paths": ["CITATION.cff", "release/0.9.0/RELEASE_NOTES.md"],
            "receipts": ["RCP-DOCS", "RCP-POST-PUSH-CI"],
        },
        {
            "claim_id": "SW09-002",
            "title": "bounded Rerun conversion and no-replace save",
            "claim_text": (
                "The candidate source contains a bounded run-log-to-Rerun conversion path and "
                "a finalized local RRD save installed without replacing an existing path."
            ),
            "code_paths": [
                "Cargo.lock",
                "crates/pid-rerun/Cargo.toml",
                "crates/pid-rerun/src/lib.rs",
                "crates/pid-rerun/src/runlog.rs",
                "crates/pid-rerun/src/bin/runlog_to_rerun.rs",
            ],
            "test_paths": [
                "crates/pid-rerun/src/lib.rs",
                "crates/pid-rerun/src/runlog.rs",
            ],
            "evidence_paths": [],
            "receipts": ["RCP-RUST", "RCP-RERUN", "RCP-POST-PUSH-CI"],
        },
        {
            "claim_id": "SW09-003",
            "title": "content-bound attribution artifacts",
            "claim_text": (
                "The candidate source contains content-addressed attribution publication and "
                "opt-in Rerun loading that checks recorded digest and canonical shape."
            ),
            "code_paths": [
                "experiments/attribution/runlog.py",
                "crates/pid-rerun/src/adapters.rs",
            ],
            "test_paths": [
                "tests/python/test_attribution.py",
            ],
            "evidence_paths": ["experiments/attribution/README.md"],
            "receipts": ["RCP-PYTHON", "RCP-RERUN", "RCP-POST-PUSH-CI"],
        },
        {
            "claim_id": "SW09-004",
            "title": "fail-closed governance generators and audits",
            "claim_text": (
                "The candidate source contains deterministic governance generators and audits "
                "for the imported requirements, capability matrix, and unfinished M0 state."
            ),
            "code_paths": [
                "scripts/audit_release_requirements.py",
                "scripts/audit_research_governance.py",
                "scripts/generate_capability_matrix.py",
            ],
            "test_paths": [
                "tests/python/test_release_requirements.py",
                "tests/python/test_capability_matrix.py",
            ],
            "evidence_paths": [
                "protocols/capability_matrix_current_v1.json",
            ],
            "receipts": ["RCP-DOCS", "RCP-PYTHON", "RCP-POST-PUSH-CI"],
        },
    ]
    scientific_specs = [
        {
            "claim_id": "SCI-EC1",
            "title": "EC1 reconstructability claim",
            "claim_text": (
                "EC1 is not established by this candidate; real-study evidence remains absent."
            ),
            "evidence_paths": [
                "grandplan.md",
                "protocols/research_claim_registry_v1.json",
            ],
        },
        {
            "claim_id": "SCI-H1",
            "title": "H1 causal sensitivity claim",
            "claim_text": (
                "H1 is not established; software references are not causal study evidence."
            ),
            "evidence_paths": [
                "EXPERIMENTS.md",
                "protocols/research_claim_registry_v1.json",
            ],
        },
        {
            "claim_id": "SCI-H2",
            "title": "H2 prospective failure-prediction claim",
            "claim_text": (
                "H2 is not established; synthetic protocol arithmetic is not prospective evidence."
            ),
            "evidence_paths": [
                "EXPERIMENTS.md",
                "protocols/research_claim_registry_v1.json",
            ],
        },
        {
            "claim_id": "SCI-H3",
            "title": "H3 representation-shift claim",
            "claim_text": "H3 is not established and its study gate remains open.",
            "evidence_paths": [
                "grandplan.md",
                "protocols/research_claim_registry_v1.json",
            ],
        },
        {
            "claim_id": "SCI-H4",
            "title": "H4 mechanistic-attribution claim",
            "claim_text": (
                "H4 is exploratory and not established by the reference probe."
            ),
            "evidence_paths": ["experiments/attribution/README.md", "grandplan.md"],
        },
        {
            "claim_id": "SCI-PID-APPLICATION",
            "title": "continuous PID application validity",
            "claim_text": (
                "The high-dimensional MI/coherence path remains NO-GO and the continuous "
                "application gate remains blocked."
            ),
            "evidence_paths": ["findings.md", "grandplan.md"],
        },
    ]
    claims: list[dict[str, Any]] = []
    for spec in software_specs:
        all_paths = sorted(
            set(spec["code_paths"] + spec["test_paths"] + spec["evidence_paths"])
        )
        claims.append(
            {
                "claim_id": spec["claim_id"],
                "claim_text": spec["claim_text"],
                "claim_tier": "E0_LOCAL_SOURCE_PENDING_EXACT_VERIFICATION",
                "status": "source_evidenced_verification_pending",
                "code_paths": spec["code_paths"],
                "test_paths": spec["test_paths"],
                "evidence_paths": spec["evidence_paths"],
                "reviewer": None,
                "independent_reviewer": None,
                "updated_at": None,
                "permitted_language": spec["claim_text"],
                "prohibited_language": (
                    "Do not infer scientific validity, release readiness, deployment security, "
                    "or successful post-push verification from source presence."
                ),
                "residual_assumptions": [
                    "the recorded working-tree bytes become one exact pushed commit",
                    "all referenced receipts complete successfully for that commit",
                ],
                "decision": "PENDING_POST_PUSH_VERIFICATION",
                "claim_class": "software",
                "title": spec["title"],
                "content_bindings": _inventory_evidence(inventory, all_paths),
                "required_evidence_receipt_ids": spec["receipts"],
                "evidence_receipt_ids": spec["receipts"],
                "supplemental_evidence_receipt_ids": [],
                "blockers": ["all referenced post-push evidence receipts are pending"],
            }
        )
    for spec in scientific_specs:
        claims.append(
            {
                "claim_id": spec["claim_id"],
                "claim_text": spec["claim_text"],
                "claim_tier": "SCIENTIFIC_NOT_ESTABLISHED",
                "status": "blocked_not_established",
                "code_paths": [],
                "test_paths": [],
                "evidence_paths": spec["evidence_paths"],
                "reviewer": None,
                "independent_reviewer": None,
                "updated_at": None,
                "permitted_language": spec["claim_text"],
                "prohibited_language": (
                    "No confirmatory, causal, safety, validated-PID, or real-study conclusion "
                    "may be drawn from this candidate."
                ),
                "residual_assumptions": [
                    "protocol and evidence gates remain open",
                    "candidate contains no real-study result establishing this claim",
                ],
                "decision": "NOT_CLAIMED",
                "claim_class": "scientific",
                "title": spec["title"],
                "content_bindings": _inventory_evidence(
                    inventory, spec["evidence_paths"]
                ),
                "required_evidence_receipt_ids": [],
                "evidence_receipt_ids": [],
                "supplemental_evidence_receipt_ids": [],
                "blockers": [
                    "protocol and evidence gates remain open",
                    "candidate contains no real-study result establishing this claim",
                ],
            }
        )
    document = {
        "schema_version": SCHEMA_VERSION,
        "record_type": "candidate_claim_to_evidence_ledger",
        "project": PROJECT,
        "release_version": RELEASE_VERSION,
        "source_state_sha256": inventory["source"]["candidate_state_sha256"],
        "claim_template_contract": {
            "source_path": "repo_work/templates/CLAIM_TO_EVIDENCE.csv",
            "source_sha256": (
                "33d388820dd61d443bd957ecd34d0979ae2d7ed328a53476e06d8d41645e0df1"
            ),
            "fields": [
                "claim_id",
                "claim_text",
                "claim_tier",
                "status",
                "code_paths",
                "test_paths",
                "evidence_paths",
                "independent_reviewer",
                "permitted_language",
                "prohibited_language",
                "residual_assumptions",
                "decision",
            ],
        },
        "status_contract": {
            "software_statuses": [
                "source_evidenced_verification_pending",
                "verified_for_exact_candidate",
                "withdrawn",
            ],
            "scientific_statuses": ["blocked_not_established", "withdrawn"],
            "terminal_promotion_enabled": False,
            "terminal_promotion_policy": TERMINAL_PROMOTION_POLICY,
            "verification_rule": (
                "Schema 0.1 cannot verify or withdraw claims; a successor schema must bind "
                "typed claim clauses to authenticated exact-candidate evidence."
            ),
        },
        "claims": claims,
        "summary": {
            "software_claim_count": len(software_specs),
            "software_verified_count": 0,
            "software_verification_pending_count": len(software_specs),
            "scientific_claim_count": len(scientific_specs),
            "scientific_established_count": 0,
            "scientific_blocked_count": len(scientific_specs),
            "withdrawn_count": 0,
        },
    }
    _apply_claim_progress(document, inventory, progress, receipts)
    return document


def _apply_claim_progress(
    document: dict[str, Any],
    inventory: Mapping[str, Any],
    progress: Mapping[str, Any],
    receipts: Mapping[str, Any],
) -> None:
    updates = _updates_by_id(
        progress["claim_updates"],
        id_field="claim_id",
        expected_keys={
            "claim_id",
            "status",
            "decision",
            "reviewer",
            "independent_reviewer",
            "updated_at",
            "evidence_paths",
            "evidence_receipt_ids",
            "residual_assumptions",
        },
        context="claim_updates",
    )
    claims = {claim["claim_id"]: claim for claim in document["claims"]}
    for claim_id, update in updates.items():
        claim = claims.get(claim_id)
        if claim is None:
            fail("PROGRESS_CLAIM", f"unknown claim update: {claim_id}")
        status_value = update["status"]
        allowed = document["status_contract"][
            "software_statuses"
            if claim["claim_class"] == "software"
            else "scientific_statuses"
        ]
        if status_value not in allowed:
            fail("PROGRESS_CLAIM", f"claim status is non-promotable: {claim_id}")
        if status_value in {"verified_for_exact_candidate", "withdrawn"}:
            fail(
                "TERMINAL_PROMOTION_DISABLED",
                f"candidate progress schema 0.1 cannot finalize claim: {claim_id}",
            )
        decision = _require_text(update["decision"], context=f"{claim_id}.decision")
        reviewer = _require_text(update["reviewer"], context=f"{claim_id}.reviewer")
        independent = _require_text(
            update["independent_reviewer"], context=f"{claim_id}.independent_reviewer"
        )
        assert isinstance(reviewer, str) and isinstance(independent, str)
        _require_distinct_reviewers(reviewer, independent, context=claim_id)
        updated_at = _require_timestamp(
            update["updated_at"], context=f"{claim_id}.updated_at"
        )
        evidence_paths = _require_string_list(
            update["evidence_paths"],
            context=f"{claim_id}.evidence_paths",
            allow_empty=False,
        )
        residual = _require_string_list(
            update["residual_assumptions"],
            context=f"{claim_id}.residual_assumptions",
            allow_empty=False,
        )
        verify = status_value == "verified_for_exact_candidate"
        receipt_refs = _validated_receipt_refs(
            update["evidence_receipt_ids"],
            receipts,
            context=f"{claim_id}.evidence_receipt_ids",
            require_passed=verify,
            allow_empty=not verify,
            consumer_updated_at=updated_at,
        )
        required_receipt_ids = claim["required_evidence_receipt_ids"]
        combined_receipt_ids = sorted(set(required_receipt_ids + receipt_refs))
        if verify:
            receipt_status = {
                receipt["id"]: receipt["status"] for receipt in receipts["receipts"]
            }
            if any(
                receipt_status.get(receipt_id) != "passed"
                for receipt_id in required_receipt_ids
            ):
                fail(
                    "PROGRESS_CLAIM",
                    f"verified claim lacks every required passed receipt: {claim_id}",
                )
        if claim["claim_class"] == "scientific":
            if status_value == "blocked_not_established" and decision != "NOT_CLAIMED":
                fail(
                    "PROGRESS_CLAIM",
                    f"scientific claim decision is invalid: {claim_id}",
                )
            if status_value == "withdrawn" and decision != "CLAIM_REMOVED":
                fail(
                    "PROGRESS_CLAIM",
                    f"withdrawn scientific claim is invalid: {claim_id}",
                )
        elif status_value == "verified_for_exact_candidate":
            if (
                decision != "VERIFIED_FOR_EXACT_CANDIDATE"
                or "RCP-POST-PUSH-CI" not in combined_receipt_ids
            ):
                fail(
                    "PROGRESS_CLAIM",
                    f"verified software claim lacks exact CI: {claim_id}",
                )
        elif status_value == "withdrawn":
            if decision != "CLAIM_REMOVED":
                fail(
                    "PROGRESS_CLAIM", f"withdrawn software claim is invalid: {claim_id}"
                )
        elif decision not in {"PENDING_POST_PUSH_VERIFICATION", "REWORK"}:
            fail(
                "PROGRESS_CLAIM",
                f"pending software claim decision is invalid: {claim_id}",
            )
        combined_paths = sorted(set(claim["evidence_paths"] + evidence_paths))
        combined_bindings = _inventory_evidence(
            inventory,
            sorted(set(claim["code_paths"] + claim["test_paths"] + combined_paths)),
        )
        claim.update(
            {
                "status": status_value,
                "decision": decision,
                "reviewer": reviewer,
                "independent_reviewer": independent,
                "updated_at": updated_at,
                "evidence_paths": combined_paths,
                "content_bindings": combined_bindings,
                "evidence_receipt_ids": combined_receipt_ids,
                "supplemental_evidence_receipt_ids": receipt_refs,
                "residual_assumptions": residual,
                "blockers": (
                    []
                    if status_value == "verified_for_exact_candidate"
                    else ["claim remains unverified or removed in this candidate"]
                ),
            }
        )
    software = [
        claim for claim in document["claims"] if claim["claim_class"] == "software"
    ]
    scientific = [
        claim for claim in document["claims"] if claim["claim_class"] == "scientific"
    ]
    document["summary"] = {
        "software_claim_count": len(software),
        "software_verified_count": sum(
            claim["status"] == "verified_for_exact_candidate" for claim in software
        ),
        "software_verification_pending_count": sum(
            claim["status"] == "source_evidenced_verification_pending"
            for claim in software
        ),
        "scientific_claim_count": len(scientific),
        "scientific_established_count": 0,
        "scientific_blocked_count": sum(
            claim["status"] == "blocked_not_established" for claim in scientific
        ),
        "withdrawn_count": sum(
            claim["status"] == "withdrawn" for claim in document["claims"]
        ),
    }


def build_receipts(
    inventory: Mapping[str, Any], progress: Mapping[str, Any]
) -> dict[str, Any]:
    receipt_specs: list[tuple[str, str, list[list[str]], list[str]]] = [
        (
            "RCP-RUST",
            "Rust workspace gates",
            [
                ["cargo", "fmt", "--all", "--", "--check"],
                ["cargo", "clippy", "--locked", "--workspace", "--", "-D", "warnings"],
                ["cargo", "test", "--locked", "--workspace"],
            ],
            [],
        ),
        (
            "RCP-RERUN",
            "Rerun adapter and binary proof",
            [
                ["cargo", "test", "--locked", "-p", "pid-rerun"],
                ["just", "runlog-rerun-proof"],
            ],
            [],
        ),
        (
            "RCP-PYTHON",
            "Python lint and tests",
            [
                ["uv", "run", "--no-sync", "ruff", "check", "."],
                ["uv", "run", "--no-sync", "ruff", "format", "--check", "."],
                ["uv", "run", "--no-sync", "pytest", "tests/python", "-q"],
            ],
            [],
        ),
        (
            "RCP-DOCS",
            "documentation and governance audits",
            [
                ["just", "docs-audit"],
                ["python", "scripts/audit_candidate_release.py"],
            ],
            [],
        ),
        (
            "RCP-SUPPLY-CHAIN",
            "all-feature dependency policy",
            [
                ["cargo", "deny", "--locked", "--all-features", "check"],
                [
                    "cargo",
                    "deny",
                    "--manifest-path",
                    "crates/ncp-observer/Cargo.toml",
                    "--locked",
                    "check",
                ],
            ],
            [],
        ),
        (
            "RCP-POST-PUSH-CI",
            "post-push main-branch CI",
            [],
            [
                "all required jobs complete for one exact pushed main commit",
                "every required job conclusion is success",
                "workflow URL and immutable commit are recorded",
            ],
        ),
    ]
    receipts = []
    for receipt_id, title, commands, required_checks in receipt_specs:
        receipts.append(
            {
                "id": receipt_id,
                "title": title,
                "status": "pending_post_push_ci",
                "source_state_sha256": inventory["source"]["candidate_state_sha256"],
                "commands": [
                    {"argv": argv, "cwd": ".", "expected_exit_code": 0}
                    for argv in commands
                ],
                "required_checks": required_checks,
                "execution": {
                    "commit": None,
                    "started_at": None,
                    "completed_at": None,
                    "runner": None,
                    "workflow_run_url": None,
                    "exit_codes": [],
                },
                "evidence": {
                    "log_artifact": None,
                    "sha256": None,
                    "bytes": None,
                    "inventory_entry_sha256": None,
                },
                "conclusion": None,
                "reviewer": None,
                "independent_reviewer": None,
                "updated_at": None,
            }
        )
    document = {
        "schema_version": SCHEMA_VERSION,
        "record_type": "candidate_test_evidence_receipts",
        "project": PROJECT,
        "release_version": RELEASE_VERSION,
        "source_state_sha256": inventory["source"]["candidate_state_sha256"],
        "receipt_contract": {
            "statuses": [
                "pending_post_push_ci",
                "in_progress",
                "blocked",
                "failed",
                "passed",
            ],
            "success_requires_exact_pushed_commit": True,
            "success_commit_must_equal_source_head": True,
            "success_requires_log_sha256": True,
            "evidence_log_namespace": EVIDENCE_RELATIVE,
            "success_allows_only_progress_and_evidence_log_source_drift": True,
            "local_uncommitted_runs_are_completion_evidence": False,
            "terminal_promotion_enabled": False,
            "terminal_promotion_policy": TERMINAL_PROMOTION_POLICY,
        },
        "receipts": receipts,
        "summary": {
            "receipt_count": len(receipts),
            "pending_count": len(receipts),
            "passed_count": 0,
            "failed_count": 0,
            "all_required_evidence_passed": False,
        },
        "boundary": (
            "Schema 0.1 may record pending, running, blocked, or failed execution evidence, "
            "but cannot authenticate a passed receipt. Positive completion requires a "
            "reviewed successor schema and authenticated CI attestation."
        ),
    }
    updates = _updates_by_id(
        progress["evidence_receipt_updates"],
        id_field="receipt_id",
        expected_keys={
            "receipt_id",
            "status",
            "reviewer",
            "independent_reviewer",
            "updated_at",
            "execution",
            "evidence_path",
            "conclusion",
        },
        context="evidence_receipt_updates",
    )
    by_id = {receipt["id"]: receipt for receipt in document["receipts"]}
    allowed = set(document["receipt_contract"]["statuses"])
    for receipt_id, update in updates.items():
        receipt = by_id.get(receipt_id)
        if receipt is None:
            fail("PROGRESS_RECEIPT", f"unknown evidence receipt: {receipt_id}")
        status_value = update["status"]
        if status_value not in allowed:
            fail("PROGRESS_RECEIPT", f"unsupported receipt status: {status_value}")
        if status_value == "passed":
            fail(
                "TERMINAL_PROMOTION_DISABLED",
                f"candidate progress schema 0.1 cannot authenticate pass: {receipt_id}",
            )
        reviewer = _require_text(
            update["reviewer"], context=f"{receipt_id}.reviewer", nullable=True
        )
        independent = _require_text(
            update["independent_reviewer"],
            context=f"{receipt_id}.independent_reviewer",
            nullable=True,
        )
        _require_distinct_reviewers(reviewer, independent, context=receipt_id)
        updated_at = _require_timestamp(
            update["updated_at"], context=f"{receipt_id}.updated_at", nullable=True
        )
        conclusion = _require_text(
            update["conclusion"], context=f"{receipt_id}.conclusion", nullable=True
        )
        execution = _assert_exact_keys(
            update["execution"],
            {
                "commit",
                "started_at",
                "completed_at",
                "runner",
                "workflow_run_url",
                "exit_codes",
            },
            context=f"{receipt_id}.execution",
        )
        execution_commit = _require_text(
            execution["commit"],
            context=f"{receipt_id}.execution.commit",
            nullable=True,
        )
        if execution_commit is not None and _OID_RE.fullmatch(execution_commit) is None:
            fail(
                "PROGRESS_RECEIPT",
                f"receipt commit is not a Git OID: {receipt_id}",
            )
        execution_started_at = _require_timestamp(
            execution["started_at"],
            context=f"{receipt_id}.execution.started_at",
            nullable=True,
        )
        execution_completed_at = _require_timestamp(
            execution["completed_at"],
            context=f"{receipt_id}.execution.completed_at",
            nullable=True,
        )
        if execution_completed_at is not None and (
            execution_started_at is None
            or execution_completed_at < execution_started_at
        ):
            fail(
                "PROGRESS_RECEIPT_CHRONOLOGY",
                f"receipt completion precedes or lacks its start: {receipt_id}",
            )
        latest_execution_at = execution_completed_at or execution_started_at
        if (
            updated_at is not None
            and latest_execution_at is not None
            and updated_at < latest_execution_at
        ):
            fail(
                "PROGRESS_RECEIPT_CHRONOLOGY",
                f"receipt update precedes its execution state: {receipt_id}",
            )
        execution_runner = _require_text(
            execution["runner"],
            context=f"{receipt_id}.execution.runner",
            nullable=True,
        )
        workflow_url = _require_text(
            execution["workflow_run_url"],
            context=f"{receipt_id}.execution.workflow_run_url",
            nullable=True,
        )
        if workflow_url is not None and not workflow_url.startswith(
            "https://github.com/sepahead/prisoma/actions/runs/"
        ):
            fail(
                "PROGRESS_RECEIPT",
                f"receipt workflow URL is invalid: {receipt_id}",
            )
        exit_codes = execution["exit_codes"]
        expected_count = len(receipt["commands"]) or len(receipt["required_checks"])
        if (
            not isinstance(exit_codes, list)
            or len(exit_codes) > expected_count
            or any(type(code) is not int for code in exit_codes)
        ):
            fail(
                "PROGRESS_RECEIPT",
                f"receipt exit-code vector is invalid: {receipt_id}",
            )
        normalized_execution = {
            "commit": execution_commit,
            "started_at": execution_started_at,
            "completed_at": execution_completed_at,
            "runner": execution_runner,
            "workflow_run_url": workflow_url,
            "exit_codes": exit_codes,
        }
        evidence_path = _require_text(
            update["evidence_path"],
            context=f"{receipt_id}.evidence_path",
            nullable=True,
        )
        if status_value == "pending_post_push_ci":
            if (
                any(
                    value is not None
                    for value in (
                        reviewer,
                        independent,
                        updated_at,
                        conclusion,
                        evidence_path,
                        execution_commit,
                        execution_started_at,
                        execution_completed_at,
                        execution_runner,
                        workflow_url,
                    )
                )
                or exit_codes != []
            ):
                fail(
                    "PROGRESS_RECEIPT",
                    f"pending receipt carries evidence: {receipt_id}",
                )
        else:
            if reviewer is None or updated_at is None or conclusion is None:
                fail(
                    "PROGRESS_RECEIPT",
                    f"promoted receipt lacks reviewer/timestamp/conclusion: {receipt_id}",
                )
        if status_value in {"passed", "failed"}:
            if independent is None or evidence_path is None:
                fail(
                    "PROGRESS_RECEIPT",
                    f"final receipt lacks independent review/evidence: {receipt_id}",
                )
            if (
                execution_commit is None
                or execution_started_at is None
                or execution_completed_at is None
                or execution_runner is None
                or workflow_url is None
            ):
                fail(
                    "PROGRESS_RECEIPT",
                    f"final receipt execution identity is incomplete: {receipt_id}",
                )
            if len(exit_codes) != expected_count:
                fail(
                    "PROGRESS_RECEIPT",
                    f"receipt exit-code vector is invalid: {receipt_id}",
                )
            if status_value == "passed" and (
                any(code != 0 for code in exit_codes) or conclusion != "success"
            ):
                fail(
                    "PROGRESS_RECEIPT",
                    f"passed receipt has failing evidence: {receipt_id}",
                )
            if status_value == "failed" and (
                all(code == 0 for code in exit_codes) or conclusion != "failure"
            ):
                fail(
                    "PROGRESS_RECEIPT",
                    f"failed receipt lacks a failing check: {receipt_id}",
                )
        elif status_value == "in_progress":
            if (
                conclusion != "running"
                or execution_started_at is None
                or execution_completed_at is not None
            ):
                fail(
                    "PROGRESS_RECEIPT",
                    f"in-progress receipt execution state is invalid: {receipt_id}",
                )
        elif status_value == "blocked" and conclusion != "blocked":
            fail("PROGRESS_RECEIPT", f"blocked receipt decision is wrong: {receipt_id}")

        if evidence_path is None:
            evidence = {
                "log_artifact": None,
                "sha256": None,
                "bytes": None,
                "inventory_entry_sha256": None,
            }
        else:
            if not evidence_path.startswith(f"{EVIDENCE_RELATIVE}/"):
                fail(
                    "PROGRESS_RECEIPT_EVIDENCE_PATH",
                    f"receipt evidence must be under {EVIDENCE_RELATIVE}: {receipt_id}",
                )
            binding = _inventory_evidence(inventory, [evidence_path])[0]
            evidence_entry = next(
                entry
                for entry in inventory["entries"]
                if entry["path"] == evidence_path
            )
            if evidence_entry["working_tree"]["kind"] != "regular":
                fail(
                    "PROGRESS_RECEIPT_EVIDENCE_PATH",
                    f"receipt evidence must be a regular file: {receipt_id}",
                )
            evidence = {
                "log_artifact": binding["path"],
                "sha256": binding["sha256"],
                "bytes": binding["bytes"],
                "inventory_entry_sha256": binding["inventory_entry_sha256"],
            }
        receipt.update(
            {
                "status": status_value,
                "execution": normalized_execution,
                "evidence": evidence,
                "conclusion": conclusion,
                "reviewer": reviewer,
                "independent_reviewer": independent,
                "updated_at": updated_at,
            }
        )
    passed = [
        receipt for receipt in document["receipts"] if receipt["status"] == "passed"
    ]
    if passed:
        source_head = inventory["source"]["head_commit"]
        if any(receipt["execution"]["commit"] != source_head for receipt in passed):
            fail(
                "PROGRESS_RECEIPT_SOURCE_COMMIT",
                "every passed receipt must bind the exact candidate source HEAD",
            )
        permitted_drift_paths = {PROGRESS_RELATIVE}
        permitted_drift_paths.update(
            receipt["evidence"]["log_artifact"]
            for receipt in document["receipts"]
            if receipt["evidence"]["log_artifact"] is not None
        )
        unexpected_drift = [
            entry["path"]
            for entry in inventory["entries"]
            if entry["path"] not in permitted_drift_paths
            and (
                entry["index_state"] != "unchanged"
                or entry["worktree_state"] != "unchanged"
            )
        ]
        if unexpected_drift:
            preview = ", ".join(unexpected_drift[:8])
            suffix = "" if len(unexpected_drift) <= 8 else ", ..."
            fail(
                "PROGRESS_RECEIPT_SOURCE_DRIFT",
                "passed receipts coexist with source drift outside candidate progress and "
                f"referenced evidence logs: {preview}{suffix}",
            )
    counts = Counter(receipt["status"] for receipt in document["receipts"])
    document["summary"] = {
        "receipt_count": len(document["receipts"]),
        "pending_count": counts["pending_post_push_ci"],
        "in_progress_count": counts["in_progress"],
        "blocked_count": counts["blocked"],
        "passed_count": counts["passed"],
        "failed_count": counts["failed"],
        "all_required_evidence_passed": counts["passed"] == len(document["receipts"]),
    }
    return document


def build_defect_register(
    inventory: Mapping[str, Any],
    progress: Mapping[str, Any],
    receipts: Mapping[str, Any],
) -> dict[str, Any]:
    defects = [
        {
            "id": "DEF-P0-001",
            "priority": "P0",
            "status": "open",
            "title": "candidate source snapshot is dirty and has no exact pushed commit",
            "blocks_release": True,
            "evidence_receipt_ids": ["RCP-POST-PUSH-CI"],
            "resolution_rule": (
                "bind a clean exact candidate commit and successful post-push main CI"
            ),
        },
        {
            "id": "DEF-P0-002",
            "priority": "P0",
            "status": "open",
            "title": "all 240 imported tasks and 4,800 lens dispositions remain open",
            "blocks_release": True,
            "evidence_receipt_ids": [],
            "resolution_rule": (
                "review each obligation under its baseline closure rule or explicitly narrow "
                "the release scope without claiming completion"
            ),
        },
        {
            "id": "DEF-P0-003",
            "priority": "P0",
            "status": "open",
            "title": "exact test and post-push evidence receipts are pending",
            "blocks_release": True,
            "evidence_receipt_ids": [
                "RCP-RUST",
                "RCP-RERUN",
                "RCP-PYTHON",
                "RCP-DOCS",
                "RCP-SUPPLY-CHAIN",
                "RCP-POST-PUSH-CI",
            ],
            "resolution_rule": "attach exact successful receipts for the pushed candidate",
        },
        {
            "id": "DEF-P1-001",
            "priority": "P1",
            "status": "open",
            "title": "continuous PID application validity remains blocked",
            "blocks_release": False,
            "evidence_receipt_ids": [],
            "resolution_rule": (
                "retain the source-preview boundary and do not publish PID atoms as "
                "application-validated results"
            ),
        },
        {
            "id": "DEF-P1-002",
            "priority": "P1",
            "status": "open",
            "title": "latest Rerun integration lacks exact post-push matrix evidence",
            "blocks_release": True,
            "evidence_receipt_ids": ["RCP-RERUN", "RCP-POST-PUSH-CI"],
            "resolution_rule": "record successful exact-candidate Rerun and CI receipts",
        },
        {
            "id": "DEF-P2-001",
            "priority": "P2",
            "status": "open",
            "title": "fuller viewer phases and the deferred custom shell remain specifications",
            "blocks_release": False,
            "evidence_receipt_ids": [],
            "resolution_rule": "keep these components out of the 0.9 runnable capability claim",
        },
    ]
    for defect in defects:
        required_receipt_ids = list(defect["evidence_receipt_ids"])
        defect.update(
            {
                "required_evidence_receipt_ids": required_receipt_ids,
                "supplemental_evidence_receipt_ids": [],
                "decision": None,
                "reviewer": None,
                "independent_reviewer": None,
                "updated_at": None,
                "evidence": [],
                "residual_risk": None,
            }
        )
    document = {
        "schema_version": SCHEMA_VERSION,
        "record_type": "candidate_defect_register",
        "project": PROJECT,
        "release_version": RELEASE_VERSION,
        "source_state_sha256": inventory["source"]["candidate_state_sha256"],
        "status_contract": {
            "priorities": ["P0", "P1", "P2"],
            "statuses": ["open", "in_progress", "blocked", "mitigated", "closed"],
            "terminal_promotion_enabled": False,
            "terminal_promotion_policy": TERMINAL_PROMOTION_POLICY,
            "closure_rule": (
                "Schema 0.1 cannot mitigate or close defects; a successor schema must bind "
                "the exact resolution rule to authenticated evidence and reviewed residual risk"
            ),
        },
        "defects": defects,
        "summary": {
            "defect_count": len(defects),
            "P0": {"open": 3, "mitigated": 0, "closed": 0},
            "P1": {"open": 2, "mitigated": 0, "closed": 0},
            "P2": {"open": 1, "mitigated": 0, "closed": 0},
            "open_release_blocker_count": 4,
            "release_blocked": True,
        },
    }
    _apply_defect_progress(document, inventory, progress, receipts)
    return document


def _apply_defect_progress(
    document: dict[str, Any],
    inventory: Mapping[str, Any],
    progress: Mapping[str, Any],
    receipts: Mapping[str, Any],
) -> None:
    updates = _updates_by_id(
        progress["defect_updates"],
        id_field="defect_id",
        expected_keys={
            "defect_id",
            "status",
            "decision",
            "reviewer",
            "independent_reviewer",
            "updated_at",
            "evidence_paths",
            "evidence_receipt_ids",
            "residual_risk",
        },
        context="defect_updates",
    )
    defects = {defect["id"]: defect for defect in document["defects"]}
    allowed = set(document["status_contract"]["statuses"])
    for defect_id, update in updates.items():
        defect = defects.get(defect_id)
        if defect is None:
            fail("PROGRESS_DEFECT", f"unknown defect update: {defect_id}")
        status_value = update["status"]
        if status_value not in allowed:
            fail("PROGRESS_DEFECT", f"unsupported defect status: {status_value}")
        if status_value in {"mitigated", "closed"}:
            fail(
                "TERMINAL_PROMOTION_DISABLED",
                f"candidate progress schema 0.1 cannot finalize defect: {defect_id}",
            )
        decision = _require_text(
            update["decision"], context=f"{defect_id}.decision", nullable=True
        )
        reviewer = _require_text(
            update["reviewer"], context=f"{defect_id}.reviewer", nullable=True
        )
        independent = _require_text(
            update["independent_reviewer"],
            context=f"{defect_id}.independent_reviewer",
            nullable=True,
        )
        _require_distinct_reviewers(reviewer, independent, context=defect_id)
        updated_at = _require_timestamp(
            update["updated_at"], context=f"{defect_id}.updated_at", nullable=True
        )
        evidence_paths = _require_string_list(
            update["evidence_paths"],
            context=f"{defect_id}.evidence_paths",
            allow_empty=True,
        )
        residual_risk = _require_text(
            update["residual_risk"],
            context=f"{defect_id}.residual_risk",
            nullable=True,
        )
        final = status_value in {"mitigated", "closed"}
        receipt_refs = _validated_receipt_refs(
            update["evidence_receipt_ids"],
            receipts,
            context=f"{defect_id}.evidence_receipt_ids",
            require_passed=final,
            allow_empty=not final,
            consumer_updated_at=updated_at,
        )
        required_receipt_ids = defect["required_evidence_receipt_ids"]
        combined_receipt_ids = sorted(set(required_receipt_ids + receipt_refs))
        if final:
            receipt_status = {
                receipt["id"]: receipt["status"] for receipt in receipts["receipts"]
            }
            if any(
                receipt_status.get(receipt_id) != "passed"
                for receipt_id in required_receipt_ids
            ):
                fail(
                    "PROGRESS_DEFECT",
                    f"final defect lacks every required passed receipt: {defect_id}",
                )
        if status_value == "open":
            if any(
                value
                for value in (
                    decision,
                    reviewer,
                    independent,
                    updated_at,
                    evidence_paths,
                    receipt_refs,
                    residual_risk,
                )
            ):
                fail(
                    "PROGRESS_DEFECT",
                    f"open defect carries transition data: {defect_id}",
                )
        elif (
            decision is None
            or reviewer is None
            or updated_at is None
            or not evidence_paths
        ):
            fail(
                "PROGRESS_DEFECT",
                f"defect transition lacks decision/reviewer/timestamp/evidence: {defect_id}",
            )
        if status_value == "in_progress" and decision != "WORK_STARTED":
            fail(
                "PROGRESS_DEFECT", f"in-progress defect decision is wrong: {defect_id}"
            )
        if status_value == "blocked" and (
            decision != "BLOCKED" or residual_risk is None
        ):
            fail("PROGRESS_DEFECT", f"blocked defect lacks residual risk: {defect_id}")
        if status_value == "mitigated" and (
            decision != "MITIGATED" or independent is None or residual_risk is None
        ):
            fail(
                "PROGRESS_DEFECT",
                f"mitigated defect lacks reviewed residual risk: {defect_id}",
            )
        if status_value == "closed" and (decision != "CLOSED" or independent is None):
            fail(
                "PROGRESS_DEFECT",
                f"closed defect lacks independent closure: {defect_id}",
            )
        defect.update(
            {
                "status": status_value,
                "decision": decision,
                "reviewer": reviewer,
                "independent_reviewer": independent,
                "updated_at": updated_at,
                "evidence": _inventory_evidence(inventory, evidence_paths),
                "evidence_receipt_ids": combined_receipt_ids,
                "supplemental_evidence_receipt_ids": receipt_refs,
                "residual_risk": residual_risk,
                "blocks_release": defect["blocks_release"]
                and status_value not in {"mitigated", "closed"},
            }
        )
    summary: dict[str, Any] = {
        "defect_count": len(document["defects"]),
    }
    for priority in ("P0", "P1", "P2"):
        counts = Counter(
            defect["status"]
            for defect in document["defects"]
            if defect["priority"] == priority
        )
        summary[priority] = {
            status_value: counts[status_value]
            for status_value in document["status_contract"]["statuses"]
        }
    summary["open_release_blocker_count"] = sum(
        defect["blocks_release"] for defect in document["defects"]
    )
    summary["release_blocked"] = summary["open_release_blocker_count"] > 0
    document["summary"] = summary


def build_draft_manifest(
    inventory: Mapping[str, Any],
    task_ledger: Mapping[str, Any],
    claim_ledger: Mapping[str, Any],
    defects: Mapping[str, Any],
    receipts: Mapping[str, Any],
) -> dict[str, Any]:
    pid_entry = next(
        (entry for entry in inventory["entries"] if entry["path"] == "pid-rs"),
        None,
    )
    if pid_entry is None or pid_entry["working_tree"]["kind"] != "gitlink":
        fail("DRAFT_SUBMODULE", "pid-rs gitlink is absent from candidate inventory")
    claim_statuses = [
        {"claim_id": claim["claim_id"], "status": claim["status"]}
        for claim in claim_ledger["claims"]
    ]
    post_push = next(
        receipt
        for receipt in receipts["receipts"]
        if receipt["id"] == "RCP-POST-PUSH-CI"
    )
    exact_pushed_commit = (
        post_push["execution"]["commit"] if post_push["status"] == "passed" else None
    )
    all_tasks_final = task_ledger["summary"]["all_tasks_closed_or_claim_removed"]
    all_waves_final = task_ledger["summary"]["wave_accepted_count"] + task_ledger[
        "summary"
    ]["claim_removed_wave_count"] == len(task_ledger["phases"])
    all_file_reviews_final = all(
        entry["file_review"]["disposition"]
        in {"ACCEPT", "FIXED", "REMOVED", "NOT_CLAIMED"}
        for entry in inventory["entries"]
    )
    software_claims_final = all(
        claim["status"] in {"verified_for_exact_candidate", "withdrawn"}
        for claim in claim_ledger["claims"]
        if claim["claim_class"] == "software"
    )
    defects_resolved = defects["summary"]["release_blocked"] is False
    evidence_complete = receipts["summary"]["all_required_evidence_passed"]
    readiness_prerequisites_satisfied = all(
        (
            exact_pushed_commit is not None,
            all_tasks_final,
            all_waves_final,
            all_file_reviews_final,
            software_claims_final,
            defects_resolved,
            evidence_complete,
        )
    )
    residual_risks = [
        (
            "candidate schema 0.1 cannot authenticate terminal evidence or represent "
            "typed task-specific closure"
        ),
        "continuous PID application validity remains blocked",
        "Rerun output hardening is local software behavior, not a remote-security claim",
        "full viewer phases and the deferred custom shell are not runnable capabilities",
    ]
    if exact_pushed_commit is None:
        residual_risks.insert(
            0, "the recorded source state is not bound to a successful pushed candidate"
        )
    if not all_tasks_final or not all_waves_final:
        residual_risks.insert(1, "imported task/lens or wave review remains unfinished")
    if not all_file_reviews_final:
        residual_risks.insert(2, "source-file review dispositions remain unfinished")
    if not software_claims_final:
        residual_risks.insert(
            3, "retained software claims remain pending exact-candidate verification"
        )
    if not evidence_complete:
        residual_risks.insert(
            4, "exact-candidate test and CI receipts remain incomplete"
        )
    return {
        "manifest_schema": "prisoma.unpublished-candidate/0.1.0",
        "schema_version": SCHEMA_VERSION,
        "record_type": "unpublished_0_9_candidate_manifest",
        "project": PROJECT,
        "release_version": RELEASE_VERSION,
        "repository": REPOSITORY,
        "author": AUTHOR,
        "decision": "NO_GO",
        "terminal_promotion": {
            "enabled": False,
            "policy": TERMINAL_PROMOTION_POLICY,
            "readiness_prerequisites_satisfied": readiness_prerequisites_satisfied,
            "successor_schema_required": True,
        },
        "source": {
            "repository": REPOSITORY,
            "commit": inventory["source"]["head_commit"],
            "tree_clean": inventory["source"]["clean"],
            "commit_role": (
                "parent_HEAD_at_dirty_candidate_capture_not_release_commit"
                if not inventory["source"]["clean"]
                else "clean_HEAD_at_candidate_capture_not_publication_identity"
            ),
            "candidate_state_sha256": inventory["source"]["candidate_state_sha256"],
        },
        "submodules": [
            {
                "path": "pid-rs",
                "index_commit": pid_entry["index"]["oid"],
                "working_commit": pid_entry["working_tree"]["gitlink_head"],
                "dirty": False,
            }
        ],
        "toolchains": [
            {
                "name": "rust",
                "declared_requirement": "1.93",
                "identity_path": "Cargo.toml",
                "verification_status": "pending_post_push_ci",
            },
            {
                "name": "python",
                "declared_requirement": ">=3.11",
                "identity_path": "pyproject.toml",
                "verification_status": "pending_post_push_ci",
            },
            {
                "name": "uv",
                "declared_requirement": "==0.11.28",
                "identity_path": "pyproject.toml",
                "verification_status": "pending_post_push_ci",
            },
        ],
        "packages": [
            {"name": "prisoma", "version": RELEASE_VERSION, "published": False}
        ],
        "schemas": [SCHEMA_VERSION],
        "protocol_status": {
            "M0": "UNFROZEN",
            "EC1": "NOT_CLAIMED",
            "H1_A": "BLOCKED",
            "H1_B": "BLOCKED",
            "H2": "BLOCKED",
            "H3": "BLOCKED",
            "H4": "BLOCKED_EXPLORATORY_ONLY",
        },
        "claims": claim_statuses,
        "datasets": [],
        "models": [],
        "holdout": {
            "registered": False,
            "status": "not_registered_governance_open",
        },
        "evidence": [
            {"receipt_id": receipt["id"], "status": receipt["status"]}
            for receipt in receipts["receipts"]
        ],
        "security": {
            "sbom_sha256": None,
            "vulnerability_report_sha256": None,
            "privacy_review_sha256": None,
            "status": (
                "exact_candidate_receipts_complete"
                if evidence_complete
                else "exact_candidate_receipts_pending"
            ),
        },
        "independent_reviews": [],
        "cross_repository_qualification": [],
        "removed_claims": [],
        "signatures": [],
        "release": {
            "version": RELEASE_VERSION,
            "nominal_handoff_version": NOMINAL_HANDOFF_VERSION,
            "status": "draft_unpublished_not_release_ready",
            "published": False,
            "tag": None,
            "release_url": None,
            "doi": None,
            "zenodo_record": None,
            "one_point_zero_convergence_claimed": False,
        },
        "source_candidate": {
            "state_sha256": inventory["source"]["candidate_state_sha256"],
            "head_commit_at_capture": inventory["source"]["head_commit"],
            "clean": inventory["source"]["clean"],
            "state": inventory["source"]["state"],
            "exact_pushed_commit": exact_pushed_commit,
        },
        "component_semantic_sha256": {
            INVENTORY_NAME: semantic_sha256(inventory),
            TASK_LEDGER_NAME: semantic_sha256(task_ledger),
            CLAIM_LEDGER_NAME: semantic_sha256(claim_ledger),
            DEFECT_REGISTER_NAME: semantic_sha256(defects),
            RECEIPTS_NAME: semantic_sha256(receipts),
        },
        "candidate_gates": {
            "terminal_promotion_schema": "open",
            "clean_exact_commit": "closed" if exact_pushed_commit else "open",
            "all_task_and_lens_dispositions_closed": (
                "closed" if all_tasks_final else "open"
            ),
            "all_wave_receipts_accepted_or_removed": (
                "closed" if all_waves_final else "open"
            ),
            "all_file_reviews_final": "closed" if all_file_reviews_final else "open",
            "retained_software_claims_verified_or_withdrawn": (
                "closed" if software_claims_final else "open"
            ),
            "P0_and_blocking_P1_defects_resolved": (
                "closed" if defects_resolved else "open"
            ),
            "exact_test_receipts_complete": ("closed" if evidence_complete else "open"),
            "post_push_main_ci_success": (
                "closed" if post_push["status"] == "passed" else "open"
            ),
            "publication_authorized": "open",
        },
        "protocol_gates": {
            "population": "open_not_frozen",
            "measure": "blocked_for_default_zero_redundancy_target",
            "estimator_high_dimensional_mi_coherence": "no_go",
            "application_continuous_pid": "blocked_not_application_validated",
            "EC1": "open_not_established",
            "H1": "open_not_established",
            "H2": "open_not_established",
            "H3": "open_not_established",
            "H4": "exploratory_not_established",
        },
        "residual_risks": residual_risks,
        "decision_detail": {
            "status": "NO_GO_SCHEMA_0_1_NON_PROMOTABLE",
            "release_ready": False,
            "scientific_claims_established": False,
            "publication_ready": False,
        },
        "boundary": (
            "This manifest describes an unpublished 0.9 candidate only. It makes no 1.0 "
            "convergence, scientific completion, DOI, Zenodo, tag, or publication claim."
        ),
    }


def build_artifact_manifest(
    artifacts: Mapping[str, bytes], inventory: Mapping[str, Any]
) -> dict[str, Any]:
    if set(artifacts) != set(ARTIFACT_NAMES):
        fail("ARTIFACT_INPUT", "candidate artifact-manifest input set is wrong")
    draft = json.loads(artifacts[DRAFT_MANIFEST_NAME])
    release_ready = draft["decision_detail"]["release_ready"]
    if release_ready is not False:
        fail(
            "ARTIFACT_READINESS",
            "candidate schema 0.1 cannot emit a release-ready artifact manifest",
        )
    return {
        "schema_version": SCHEMA_VERSION,
        "record_type": "candidate_artifact_manifest",
        "project": PROJECT,
        "release_version": RELEASE_VERSION,
        "candidate_state_sha256": inventory["source"]["candidate_state_sha256"],
        "artifacts": [
            {
                "path": name,
                "sha256": sha256_bytes(artifacts[name]),
                "bytes": len(artifacts[name]),
            }
            for name in sorted(ARTIFACT_NAMES)
        ],
        "status": "integrity_manifest_for_unpublished_candidate",
        "release_ready": release_ready,
        "published": False,
    }


def build_artifacts_from_inventory(
    repo: Path, inventory: Mapping[str, Any]
) -> dict[str, bytes]:
    baseline, baseline_raw = _read_baseline(repo)
    progress = inventory["progress_snapshot"]["document"]
    receipts = build_receipts(inventory, progress)
    task_ledger = build_task_ledger(
        baseline, baseline_raw, inventory, progress, receipts
    )
    claim_ledger = build_claim_ledger(inventory, progress, receipts)
    defects = build_defect_register(inventory, progress, receipts)
    draft_manifest = build_draft_manifest(
        inventory, task_ledger, claim_ledger, defects, receipts
    )
    documents = {
        INVENTORY_NAME: inventory,
        TASK_LEDGER_NAME: task_ledger,
        CLAIM_LEDGER_NAME: claim_ledger,
        DEFECT_REGISTER_NAME: defects,
        RECEIPTS_NAME: receipts,
        DRAFT_MANIFEST_NAME: draft_manifest,
    }
    artifacts: dict[str, bytes] = {}
    for name, document in documents.items():
        raw = pretty_json_bytes(document)
        if len(raw) > MAX_CANDIDATE_ARTIFACT_BYTES:
            fail(
                "ARTIFACT_SIZE",
                f"generated candidate artifact exceeds "
                f"{MAX_CANDIDATE_ARTIFACT_BYTES} bytes: {name}",
            )
        artifacts[name] = raw
    manifest_raw = pretty_json_bytes(build_artifact_manifest(artifacts, inventory))
    if len(manifest_raw) > MAX_CANDIDATE_ARTIFACT_BYTES:
        fail("ARTIFACT_SIZE", "generated candidate artifact manifest is oversized")
    artifacts[ARTIFACT_MANIFEST_NAME] = manifest_raw
    return artifacts


def build_artifacts(
    repo: Path,
    *,
    source_head: str,
    source_index_sha256: str,
    source_worktree_sha256: str,
) -> dict[str, bytes]:
    inventory = capture_stable_inventory(repo)
    actual = source_state_arguments(inventory)
    expected = {
        "source_head": source_head,
        "source_index_sha256": source_index_sha256,
        "source_worktree_sha256": source_worktree_sha256,
    }
    for key, expected_value in expected.items():
        if actual[key] != expected_value:
            fail(
                "EXPLICIT_SOURCE_MISMATCH",
                f"{key} differs from explicit input; expected {expected_value}, "
                f"observed {actual[key]}",
            )
    return build_artifacts_from_inventory(repo, inventory)


def _directory_names(path: Path) -> set[str]:
    names: set[str] = set()
    try:
        for entry in path.iterdir():
            if len(names) >= len(EXPECTED_OUTPUT_NAMES):
                fail("OUTPUT_FILE_SET", "candidate output contains extra entries")
            if entry.is_symlink() or not entry.is_file():
                fail("OUTPUT_ENTRY", f"output contains a non-regular entry: {entry}")
            names.add(entry.name)
    except CandidateError:
        raise
    except OSError as exc:
        fail("OUTPUT_DIRECTORY", f"cannot read output directory: {exc}")
    return names


def _require_output_set(path: Path, *, allow_empty: bool) -> None:
    names = _directory_names(path)
    if allow_empty and not names:
        return
    if names != EXPECTED_OUTPUT_NAMES:
        fail(
            "OUTPUT_FILE_SET",
            f"candidate output set differs; missing={sorted(EXPECTED_OUTPUT_NAMES - names)}, "
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
        fail("OUTPUT_WRITE", f"cannot durably sync candidate directory {path}: {exc}")


def write_artifacts(output_dir: Path, artifacts: Mapping[str, bytes]) -> None:
    if set(artifacts) != EXPECTED_OUTPUT_NAMES:
        fail("OUTPUT_ARTIFACT_SET", "generated candidate artifact set is wrong")
    oversized = [
        name
        for name, raw in artifacts.items()
        if len(raw) > MAX_CANDIDATE_ARTIFACT_BYTES
    ]
    if oversized:
        fail(
            "ARTIFACT_SIZE", f"generated candidate artifacts are oversized: {oversized}"
        )
    if output_dir.exists():
        if output_dir.is_symlink() or not output_dir.is_dir():
            fail("OUTPUT_DIRECTORY", f"output must be a real directory: {output_dir}")
        _require_output_set(output_dir, allow_empty=True)
    else:
        try:
            output_dir.mkdir(parents=True, exist_ok=False)
        except OSError as exc:
            fail("OUTPUT_DIRECTORY", f"cannot create output directory: {exc}")
    # Install the integrity manifest last. An interrupted multi-file refresh is still
    # rejected by the auditor, while a visible new manifest never precedes the files it binds.
    write_order = sorted(name for name in artifacts if name != ARTIFACT_MANIFEST_NAME)
    write_order.append(ARTIFACT_MANIFEST_NAME)
    for name in write_order:
        if name == ARTIFACT_MANIFEST_NAME:
            _fsync_directory(output_dir)
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
            fail(
                "OUTPUT_WRITE", f"cannot write candidate artifact {destination}: {exc}"
            )
    _fsync_directory(output_dir)
    _require_output_set(output_dir, allow_empty=False)


def check_artifacts(output_dir: Path, artifacts: Mapping[str, bytes]) -> None:
    if output_dir.is_symlink() or not output_dir.is_dir():
        fail("OUTPUT_DIRECTORY", f"output must be a real directory: {output_dir}")
    _require_output_set(output_dir, allow_empty=False)
    for name, expected in artifacts.items():
        actual, _ = _read_bounded_regular(
            output_dir / name,
            max_bytes=MAX_CANDIDATE_ARTIFACT_BYTES,
            path_code="OUTPUT_READ",
            read_code="OUTPUT_READ",
            size_code="OUTPUT_READ",
            description="candidate output artifact",
        )
        if actual != expected:
            fail("OUTPUT_DRIFT", f"candidate artifact is stale: {name}")


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path("."))
    parser.add_argument("--output-dir", type=Path, default=Path(CANDIDATE_RELATIVE))
    parser.add_argument("--source-head")
    parser.add_argument("--source-index-sha256")
    parser.add_argument("--source-worktree-sha256")
    parser.add_argument(
        "--print-source-state",
        action="store_true",
        help="print the explicit arguments for a stable source snapshot and exit",
    )
    parser.add_argument("--check", action="store_true")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        repo = resolve_repo(args.repo)
        if args.print_source_state:
            inventory = capture_stable_inventory(repo)
            print(
                json.dumps(
                    source_state_arguments(inventory),
                    ensure_ascii=False,
                    allow_nan=False,
                    sort_keys=True,
                )
            )
            return 0
        missing = [
            name
            for name in (
                "source_head",
                "source_index_sha256",
                "source_worktree_sha256",
            )
            if getattr(args, name) is None
        ]
        if missing:
            print(
                "candidate generation requires explicit --source-head, "
                "--source-index-sha256, and --source-worktree-sha256",
                file=sys.stderr,
            )
            return 2
        output_dir = args.output_dir
        if not output_dir.is_absolute():
            output_dir = repo / output_dir
        artifacts = build_artifacts(
            repo,
            source_head=args.source_head,
            source_index_sha256=args.source_index_sha256,
            source_worktree_sha256=args.source_worktree_sha256,
        )
        if args.check:
            check_artifacts(output_dir, artifacts)
        else:
            write_artifacts(output_dir, artifacts)
    except CandidateError as exc:
        print(f"candidate generation failed [{exc.code}]: {exc}", file=sys.stderr)
        return 3
    mode = "current" if args.check else "generated"
    print(
        f"release candidate {mode}: artifacts={len(artifacts)}; "
        "release_ready=false; published=false"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
