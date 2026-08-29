#!/usr/bin/env python3
"""Exact Git and imported-source closure for Engram interoperability tools."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import re
import stat
import subprocess
import sys
from pathlib import Path
from typing import Any


MAX_IMPORTED_SOURCE_BYTES = 16 * 1024 * 1024
MAX_IMPORTED_SOURCE_COUNT = 256
MAX_IMPORTED_SOURCE_TOTAL_BYTES = 64 * 1024 * 1024
MAX_GIT_OUTPUT_BYTES = 128 * 1024 * 1024
MAX_GIT_COMMIT_BYTES = 1024 * 1024
IGNORED_REPOSITORY_DIRECTORIES = {
    ".git",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    ".venv",
    "node_modules",
    "target",
    "venv",
}
GIT_OBJECT_PATTERN = re.compile(r"(?:[0-9a-f]{40}|[0-9a-f]{64})")
GIT_OBJECT_LENGTHS = {"sha1": 40, "sha256": 64}
EVIDENCE_PUBLICATION_POLICY = "direct-child-exact-added-regular-blobs.v1"
EVIDENCE_PUBLICATION_ROSTER_DOMAIN = b"prisoma-evidence-publication-roster-v1\0"


def canonical(value: Any) -> bytes:
    """Encode one bounded provenance value deterministically."""

    return json.dumps(
        value,
        ensure_ascii=False,
        allow_nan=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def digest_bytes(payload: bytes) -> str:
    """Return a lowercase SHA-256 digest."""

    return hashlib.sha256(payload).hexdigest()


def _run_git(
    repository: Path,
    arguments: list[str],
    *,
    input_bytes: bytes | None = None,
) -> bytes:
    environment = {
        key: value for key, value in os.environ.items() if not key.startswith("GIT_")
    }
    environment.update(
        {
            "GIT_NO_REPLACE_OBJECTS": "1",
            "GIT_OPTIONAL_LOCKS": "0",
            "GIT_TERMINAL_PROMPT": "0",
            "LC_ALL": "C",
        }
    )
    completed = subprocess.run(
        [
            "git",
            "--no-replace-objects",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "-C",
            os.fspath(repository),
            *arguments,
        ],
        check=False,
        capture_output=True,
        env=environment,
        input=input_bytes,
        timeout=10,
    )
    if (
        completed.returncode != 0
        or completed.stderr
        or len(completed.stdout) > MAX_GIT_OUTPUT_BYTES
    ):
        raise ValueError("Git object cannot be resolved exactly")
    return completed.stdout


def _git_text(repository: Path, arguments: list[str], label: str) -> str:
    """Read one nonempty, single-line Git value."""

    try:
        payload = _run_git(repository, arguments).decode("utf-8")
    except UnicodeDecodeError as error:
        raise ValueError(f"{label} is not UTF-8") from error
    if not payload.endswith("\n"):
        raise ValueError(f"{label} lacks one terminal newline")
    value = payload[:-1]
    if (
        not value
        or "\n" in value
        or "\r" in value
        or value != value.strip()
        or any(ord(character) < 0x20 for character in value)
    ):
        raise ValueError(f"{label} is not one bounded Git value")
    return value


def valid_git_object(value: Any, object_format: str | None = None) -> bool:
    """Return true for one lowercase Git object in the declared format."""

    if not isinstance(value, str) or GIT_OBJECT_PATTERN.fullmatch(value) is None:
        return False
    if object_format is None:
        return True
    expected_length = GIT_OBJECT_LENGTHS.get(object_format)
    return expected_length is not None and len(value) == expected_length


def _resolved_repository(repository: Path) -> Path:
    lexical = Path(os.path.abspath(repository))
    resolved = lexical.resolve(strict=True)
    if lexical != resolved:
        raise ValueError("repository path traverses a link")
    return resolved


def _owner_controlled_directory(path: Path, *, label: str) -> Path:
    """Require one canonical owner-controlled directory."""

    lexical = Path(os.path.abspath(path))
    try:
        resolved = lexical.resolve(strict=True)
        observed = os.stat(lexical, follow_symlinks=False)
    except OSError as error:
        raise ValueError(f"{label} cannot be inspected") from error
    if (
        lexical != resolved
        or not stat.S_ISDIR(observed.st_mode)
        or stat.S_ISLNK(observed.st_mode)
        or observed.st_uid != os.geteuid()
        or stat.S_IMODE(observed.st_mode) & 0o022
    ):
        raise ValueError(f"{label} is not an owner-controlled directory")
    return lexical


def _owner_controlled_directory_chain(
    repository: Path,
    relative: Path,
    *,
    label: str,
) -> Path:
    """Require each repository-relative directory without following links."""

    current = _owner_controlled_directory(repository, label=label)
    for part in relative.parts:
        current = _owner_controlled_directory(current / part, label=label)
    return current


def _repository_layout(repository: Path) -> tuple[Path, Path]:
    """Rejoin Git's worktree, administrative directory, and common directory."""

    _owner_controlled_directory(repository, label="repository root")
    top_level = _git_text(
        repository,
        ["rev-parse", "--show-toplevel"],
        "Git worktree root",
    )
    if Path(os.path.abspath(top_level)) != repository:
        raise ValueError("Git worktree root differs from the repository")
    if (
        _git_text(
            repository,
            ["rev-parse", "--is-bare-repository"],
            "Git bare-repository state",
        )
        != "false"
        or _git_text(
            repository,
            ["rev-parse", "--is-inside-work-tree"],
            "Git worktree state",
        )
        != "true"
    ):
        raise ValueError("Git repository is not one non-bare worktree")

    git_directory = _owner_controlled_directory(
        Path(
            _git_text(
                repository,
                ["rev-parse", "--path-format=absolute", "--git-dir"],
                "Git administrative directory",
            )
        ),
        label="Git administrative directory",
    )
    common_directory = _owner_controlled_directory(
        Path(
            _git_text(
                repository,
                ["rev-parse", "--path-format=absolute", "--git-common-dir"],
                "Git common directory",
            )
        ),
        label="Git common directory",
    )
    marker = repository / ".git"
    try:
        marker_stat = os.stat(marker, follow_symlinks=False)
    except OSError as error:
        raise ValueError("Git worktree marker cannot be inspected") from error
    if stat.S_ISDIR(marker_stat.st_mode) and not stat.S_ISLNK(marker_stat.st_mode):
        marker_directory = _owner_controlled_directory(
            marker,
            label="Git worktree marker",
        )
        if marker_directory != git_directory or git_directory != common_directory:
            raise ValueError("Git directory does not rejoin the worktree root")
    elif stat.S_ISREG(marker_stat.st_mode) and not stat.S_ISLNK(marker_stat.st_mode):
        marker_payload = snapshot_regular_file(marker, 4096)
        if marker_payload != b"gitdir: " + os.fsencode(git_directory) + b"\n":
            raise ValueError("Git worktree marker differs from its directory")
        if (
            git_directory.parent != common_directory / "worktrees"
            or snapshot_regular_file(git_directory / "commondir", 4096) != b"../..\n"
            or snapshot_regular_file(git_directory / "gitdir", 4096)
            != os.fsencode(marker) + b"\n"
        ):
            raise ValueError("Git linked-worktree directories do not rejoin")
    else:
        raise ValueError("Git worktree marker is not a regular local object")

    shallow_path = Path(
        _git_text(
            repository,
            ["rev-parse", "--path-format=absolute", "--git-path", "shallow"],
            "Git shallow path",
        )
    )
    graft_path = Path(
        _git_text(
            repository,
            ["rev-parse", "--path-format=absolute", "--git-path", "info/grafts"],
            "Git graft path",
        )
    )
    if shallow_path != common_directory / "shallow" or graft_path != (
        common_directory / "info/grafts"
    ):
        raise ValueError("Git authority paths do not rejoin the common directory")
    if (
        _git_text(
            repository,
            ["rev-parse", "--is-shallow-repository"],
            "Git shallow-repository state",
        )
        != "false"
        or os.path.lexists(shallow_path)
        or os.path.lexists(graft_path)
        or _run_git(
            repository,
            ["for-each-ref", "--format=%(refname)", "refs/replace/"],
        )
    ):
        raise ValueError("Git history has a shallow, graft, or replacement override")
    return git_directory, common_directory


def _require_normal_index(repository: Path) -> None:
    """Reject assume-unchanged, skip-worktree, and other non-normal index rows."""

    raw = _run_git(repository, ["ls-files", "-v", "-z", "--"])
    rows = raw.split(b"\0")
    if rows[-1:] != [b""]:
        raise ValueError("Git index roster is not NUL-terminated")
    rows.pop()
    if not rows or any(len(row) < 3 or row[:2] != b"H " for row in rows):
        raise ValueError("Git index contains a non-normal tracked entry")


def capture_repository_identity(
    repository: Path,
    expected_revision: str,
) -> dict[str, Any]:
    """Bind one clean checkout to its pushed main commit and tree."""

    repository = _resolved_repository(repository)
    _repository_layout(repository)
    _require_normal_index(repository)
    object_format = _git_text(
        repository,
        ["rev-parse", "--show-object-format"],
        "Git object format",
    )
    if object_format not in GIT_OBJECT_LENGTHS:
        raise ValueError("Git object format is unsupported")
    if not valid_git_object(expected_revision, object_format):
        raise ValueError("expected revision is not a full lowercase Git object ID")
    resolved = _git_text(
        repository,
        ["rev-parse", "--verify", f"{expected_revision}^{{commit}}"],
        "resolved Git commit",
    )
    head = _git_text(
        repository,
        ["rev-parse", "--verify", "HEAD^{commit}"],
        "Git HEAD",
    )
    origin_main = _git_text(
        repository,
        ["rev-parse", "--verify", "refs/remotes/origin/main^{commit}"],
        "origin/main",
    )
    tree = _git_text(
        repository,
        ["rev-parse", "--verify", f"{expected_revision}^{{tree}}"],
        "Git tree",
    )
    origin = _git_text(
        repository,
        ["remote", "get-url", "origin"],
        "Git origin",
    )
    status = _run_git(
        repository,
        [
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
        ],
    )
    if (
        resolved != expected_revision
        or head != expected_revision
        or origin_main != expected_revision
        or status
        or not valid_git_object(tree, object_format)
    ):
        raise ValueError("repository is not clean at the required pushed main revision")
    return {
        "repository": origin,
        "commit": expected_revision,
        "tree": tree,
        "origin_main": origin_main,
        "object_format": object_format,
        "clean": True,
    }


def verify_repository_revision(repository: Path, expected_revision: str) -> str:
    """Require one clean checkout at the caller's pushed main commit."""

    return capture_repository_identity(repository, expected_revision)["commit"]


def snapshot_regular_file(
    path: Path,
    max_bytes: int,
    *,
    allow_empty: bool = False,
) -> bytes:
    """Read one bounded regular file without following its final path component."""

    payload, _identity = _snapshot_regular_file(
        path, max_bytes, allow_empty=allow_empty
    )
    return payload


def _snapshot_regular_file(
    path: Path,
    max_bytes: int,
    *,
    allow_empty: bool = False,
) -> tuple[bytes, tuple[int, ...]]:
    """Return stable bytes and identity for one unique owner-controlled file."""

    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    named_before = os.stat(path, follow_symlinks=False)
    descriptor = os.open(path, flags)
    try:
        before = os.fstat(descriptor)
        if (
            not stat.S_ISREG(before.st_mode)
            or before.st_uid != os.geteuid()
            or before.st_nlink != 1
            or stat.S_IMODE(before.st_mode) & 0o022
            or before.st_size < (0 if allow_empty else 1)
            or before.st_size > max_bytes
            or _regular_file_identity(named_before) != _regular_file_identity(before)
        ):
            raise ValueError(f"source is not a bounded owner-controlled file: {path}")
        chunks: list[bytes] = []
        remaining = before.st_size
        while remaining:
            chunk = os.read(descriptor, min(1024 * 1024, remaining))
            if not chunk:
                raise ValueError(f"source changed while it was read: {path}")
            chunks.append(chunk)
            remaining -= len(chunk)
        if os.read(descriptor, 1):
            raise ValueError(f"source grew while it was read: {path}")
        after = os.fstat(descriptor)
        named_after = os.stat(path, follow_symlinks=False)
        if not (
            _regular_file_identity(named_before)
            == _regular_file_identity(before)
            == _regular_file_identity(after)
            == _regular_file_identity(named_after)
        ):
            raise ValueError(f"source identity changed while it was read: {path}")
        return b"".join(chunks), _regular_file_identity(after)
    finally:
        os.close(descriptor)


def digest_regular_file(path: Path, max_bytes: int) -> str:
    """Digest one bounded regular file and reject identity drift."""

    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    named_before = os.stat(path, follow_symlinks=False)
    descriptor = os.open(path, flags)
    try:
        before = os.fstat(descriptor)
        if (
            not stat.S_ISREG(before.st_mode)
            or before.st_uid != os.geteuid()
            or before.st_nlink != 1
            or stat.S_IMODE(before.st_mode) & 0o022
            or before.st_size <= 0
            or before.st_size > max_bytes
            or _regular_file_identity(named_before) != _regular_file_identity(before)
        ):
            raise ValueError(f"source is not a bounded owner-controlled file: {path}")
        digest = hashlib.sha256()
        remaining = before.st_size
        while remaining:
            chunk = os.read(descriptor, min(1024 * 1024, remaining))
            if not chunk:
                raise ValueError(f"source changed while it was hashed: {path}")
            digest.update(chunk)
            remaining -= len(chunk)
        if os.read(descriptor, 1):
            raise ValueError(f"source grew while it was hashed: {path}")
        after = os.fstat(descriptor)
        named_after = os.stat(path, follow_symlinks=False)
        if not (
            _regular_file_identity(named_before)
            == _regular_file_identity(before)
            == _regular_file_identity(after)
            == _regular_file_identity(named_after)
        ):
            raise ValueError(f"source identity changed while it was hashed: {path}")
        return digest.hexdigest()
    finally:
        os.close(descriptor)


def _regular_file_identity(value: os.stat_result) -> tuple[int, ...]:
    return (
        value.st_dev,
        value.st_ino,
        value.st_mode,
        value.st_nlink,
        value.st_uid,
        value.st_size,
        value.st_mtime_ns,
        value.st_ctime_ns,
    )


def _module_source_paths(module: Any) -> tuple[Path, Path] | None:
    """Return lexical and resolved paths for one imported Python source."""

    module_file = getattr(module, "__file__", None)
    if (
        not isinstance(module_file, str)
        or module_file.startswith("<")
        or module_file.endswith(">")
    ):
        return None
    path = Path(module_file)
    if path.suffix in {".pyc", ".pyo"}:
        try:
            path = Path(importlib.util.source_from_cache(os.fspath(path)))
        except ValueError as error:
            raise ValueError("Engram bytecode source path is not canonical") from error
    lexical = Path(os.path.abspath(path))
    try:
        resolved = lexical.resolve(strict=True)
    except FileNotFoundError:
        resolved = lexical
    return lexical, resolved


def _is_engram_project_module(module_name: str) -> bool:
    return module_name == "backend" or module_name.startswith("backend.")


def _valid_engram_namespace_module(
    module_name: str,
    module: Any,
    repository: Path,
) -> bool:
    search_locations = getattr(module, "__path__", None)
    if search_locations is None:
        return False
    try:
        locations = tuple(search_locations)
    except TypeError:
        return False
    expected = repository.joinpath(*module_name.split("."))
    if len(locations) != 1 or not isinstance(locations[0], str):
        return False
    lexical = Path(os.path.abspath(locations[0]))
    try:
        resolved = lexical.resolve(strict=True)
        observed = resolved.stat()
    except OSError:
        return False
    return (
        lexical == resolved == expected
        and stat.S_ISDIR(observed.st_mode)
        and observed.st_uid == os.geteuid()
    )


def capture_repository_file(
    repository: Path,
    expected_revision: str,
    relative: Path,
    max_bytes: int,
    *,
    checkout_revision: str | None = None,
) -> dict[str, Any]:
    """Bind one current repository file to the same path at an exact commit."""

    return capture_repository_files(
        repository,
        expected_revision,
        [relative],
        max_bytes,
        checkout_revision=checkout_revision,
    )[0]


def capture_repository_files(
    repository: Path,
    expected_revision: str,
    relatives: list[Path] | tuple[Path, ...],
    max_bytes: int,
    *,
    checkout_revision: str | None = None,
) -> list[dict[str, Any]]:
    """Bind current files to one commit in a clean repository snapshot."""

    repository = _resolved_repository(repository)
    current_revision = checkout_revision or expected_revision
    identity = capture_repository_identity(repository, current_revision)
    if not valid_git_object(expected_revision, identity["object_format"]):
        raise ValueError("source revision differs from the Git object format")
    resolved_revision = _git_text(
        repository,
        ["rev-parse", "--verify", f"{expected_revision}^{{commit}}"],
        "source revision",
    )
    if resolved_revision != expected_revision:
        raise ValueError("source revision does not resolve exactly")
    if not isinstance(relatives, (list, tuple)) or not 1 <= len(relatives) <= 256:
        raise ValueError("repository source roster exceeds its file bound")
    relative_texts = [relative.as_posix() for relative in relatives]
    if relative_texts != sorted(set(relative_texts)):
        raise ValueError("repository source roster is not sorted and unique")
    rows: list[dict[str, Any]] = []
    snapshots: list[tuple[Path, bytes]] = []
    total_bytes = 0
    for relative in relatives:
        if (
            relative.is_absolute()
            or not relative.parts
            or any(part in {"", ".", ".."} for part in relative.parts)
            or not 1 <= len(relative.as_posix()) <= 512
        ):
            raise ValueError("repository source path is not a canonical relative path")
        source_path = repository / relative
        if source_path.resolve(strict=True) != source_path:
            raise ValueError(f"repository source path traverses a link: {relative}")
        payload = snapshot_regular_file(source_path, max_bytes, allow_empty=True)
        total_bytes += len(payload)
        if total_bytes > MAX_IMPORTED_SOURCE_TOTAL_BYTES:
            raise ValueError("repository source roster exceeds its byte bound")
        recorded_size_text = (
            _run_git(
                repository,
                ["cat-file", "-s", f"{expected_revision}:{relative.as_posix()}"],
            )
            .decode("ascii")
            .strip()
        )
        if not recorded_size_text.isascii() or not recorded_size_text.isdigit():
            raise ValueError(f"repository Git size differs: {relative}")
        recorded_size = int(recorded_size_text)
        if recorded_size != len(payload):
            raise ValueError(f"repository Git source size differs: {relative}")
        recorded_payload = _run_git(
            repository,
            ["show", f"{expected_revision}:{relative.as_posix()}"],
        )
        if recorded_payload != payload:
            raise ValueError(
                f"repository source differs from expected revision: {relative}"
            )
        recorded_blob = (
            _run_git(
                repository,
                ["rev-parse", f"{expected_revision}:{relative.as_posix()}"],
            )
            .decode("ascii")
            .strip()
        )
        current_blob = (
            _run_git(
                repository,
                ["hash-object", "--stdin"],
                input_bytes=payload,
            )
            .decode("ascii")
            .strip()
        )
        if recorded_blob != current_blob or not valid_git_object(
            recorded_blob,
            identity["object_format"],
        ):
            raise ValueError(f"repository Git blob differs: {relative}")
        rows.append(
            {
                "path": relative.as_posix(),
                "sha256": digest_bytes(payload),
                "git_blob": recorded_blob,
                "byte_count": len(payload),
            }
        )
        snapshots.append((source_path, payload))
    if capture_repository_identity(repository, current_revision) != identity:
        raise ValueError("repository identity changed during source capture")
    if any(
        snapshot_regular_file(path, max_bytes, allow_empty=True) != payload
        for path, payload in snapshots
    ):
        raise ValueError("repository source changed during independent re-open")
    return rows


def _canonical_repository_relative(relative: Path, *, label: str) -> str:
    """Return one canonical repository-relative POSIX path."""

    value = relative.as_posix()
    if (
        relative.is_absolute()
        or not relative.parts
        or any(part in {"", ".", ".."} for part in relative.parts)
        or not 1 <= len(value) <= 4096
        or "\\" in value
        or "\0" in value
    ):
        raise ValueError(f"{label} is not a canonical repository path")
    return value


def _require_raw_commit_parent(
    repository: Path,
    commit: str,
    expected_tree: str,
    expected_parent: str,
) -> None:
    """Read one raw commit object and require its sole literal parent."""

    payload = _run_git(repository, ["cat-file", "commit", commit])
    if (
        not 1 <= len(payload) <= MAX_GIT_COMMIT_BYTES
        or b"\0" in payload
        or b"\r" in payload
        or b"\n\n" not in payload
    ):
        raise ValueError("evidence publication commit object is malformed")
    header, _message = payload.split(b"\n\n", 1)
    lines = header.split(b"\n")
    fields: list[tuple[bytes, bytes]] = []
    previous_key: bytes | None = None
    for line in lines:
        if line.startswith(b" "):
            if (
                previous_key not in {b"gpgsig", b"gpgsig-sha256", b"mergetag"}
                or len(line) > 16_384
            ):
                raise ValueError("evidence publication commit header is malformed")
            continue
        try:
            key, value = line.split(b" ", 1)
        except ValueError as error:
            raise ValueError(
                "evidence publication commit header is malformed"
            ) from error
        if (
            re.fullmatch(rb"[a-z][a-z0-9-]{0,63}", key) is None
            or not value
            or len(value) > 16_384
            or any(byte < 0x20 or byte == 0x7F for byte in value)
        ):
            raise ValueError("evidence publication commit header is malformed")
        fields.append((key, value))
        previous_key = key
    trees = [value for key, value in fields if key == b"tree"]
    parents = [value for key, value in fields if key == b"parent"]
    authors = [value for key, value in fields if key == b"author"]
    committers = [value for key, value in fields if key == b"committer"]
    if (
        not fields
        or fields[0] != (b"tree", expected_tree.encode("ascii"))
        or len(fields) < 4
        or fields[1] != (b"parent", expected_parent.encode("ascii"))
        or trees != [expected_tree.encode("ascii")]
        or parents != [expected_parent.encode("ascii")]
        or len(authors) != 1
        or len(committers) != 1
    ):
        raise ValueError("evidence publication is not a direct single-parent child")


def _publication_snapshot(
    repository: Path,
    source_revision: str,
    publication_identity: dict[str, Any],
    relatives: tuple[Path, ...],
    max_file_bytes: int,
    max_total_bytes: int,
) -> dict[str, Any]:
    """Capture one complete evidence-only publication state."""

    publication_revision = publication_identity["commit"]
    object_format = publication_identity["object_format"]
    object_length = GIT_OBJECT_LENGTHS[object_format]
    source_commit = _git_text(
        repository,
        ["rev-parse", "--verify", f"{source_revision}^{{commit}}"],
        "evidence source commit",
    )
    source_tree = _git_text(
        repository,
        ["rev-parse", "--verify", f"{source_revision}^{{tree}}"],
        "evidence source tree",
    )
    if source_commit != source_revision or not valid_git_object(
        source_tree, object_format
    ):
        raise ValueError("evidence source revision does not resolve exactly")
    _require_raw_commit_parent(
        repository,
        publication_revision,
        publication_identity["tree"],
        source_revision,
    )

    expected_paths = tuple(
        _canonical_repository_relative(relative, label="evidence path")
        for relative in relatives
    )
    if expected_paths != tuple(sorted(set(expected_paths))):
        raise ValueError("evidence path roster is not sorted and unique")
    parents = {relative.parent for relative in relatives}
    if len(parents) != 1 or next(iter(parents)) == Path("."):
        raise ValueError("evidence files do not share one publication directory")
    evidence_directory = next(iter(parents))
    directory_text = _canonical_repository_relative(
        evidence_directory,
        label="evidence directory",
    )

    raw_delta = _run_git(
        repository,
        [
            "diff-tree",
            "--no-commit-id",
            "--raw",
            "-r",
            "-z",
            "--no-renames",
            "--full-index",
            source_revision,
            publication_revision,
            "--",
        ],
    )
    delta_parts = raw_delta.split(b"\0")
    if delta_parts[-1:] != [b""]:
        raise ValueError("evidence publication delta is not NUL-terminated")
    delta_parts.pop()
    if len(delta_parts) % 2 != 0:
        raise ValueError("evidence publication delta is malformed")
    delta_rows: list[tuple[str, str]] = []
    for position in range(0, len(delta_parts), 2):
        try:
            header = delta_parts[position].decode("ascii")
            path = delta_parts[position + 1].decode("utf-8")
            old_mode, new_mode, old_object, new_object, status = header.split(" ")
        except (UnicodeDecodeError, ValueError) as error:
            raise ValueError("evidence publication delta is malformed") from error
        if (
            old_mode != ":000000"
            or new_mode != "100644"
            or old_object != "0" * object_length
            or not valid_git_object(new_object, object_format)
            or status != "A"
        ):
            raise ValueError("evidence publication delta is not add-only regular data")
        delta_rows.append((path, new_object))
    if tuple(path for path, _object in delta_rows) != expected_paths:
        raise ValueError("evidence publication changed a path outside its exact roster")

    raw_tree = _run_git(
        repository,
        [
            "ls-tree",
            "-r",
            "-z",
            "--full-tree",
            publication_revision,
            "--",
            directory_text,
        ],
    )
    tree_entries = raw_tree.split(b"\0")
    if tree_entries[-1:] != [b""]:
        raise ValueError("evidence Git tree roster is not NUL-terminated")
    tree_entries.pop()
    tree_rows: list[tuple[str, str]] = []
    for entry in tree_entries:
        try:
            header, raw_path = entry.split(b"\t", 1)
            mode, kind, object_id = header.decode("ascii").split(" ")
            path = raw_path.decode("utf-8")
        except (UnicodeDecodeError, ValueError) as error:
            raise ValueError("evidence Git tree roster is malformed") from error
        if (
            mode != "100644"
            or kind != "blob"
            or not valid_git_object(object_id, object_format)
        ):
            raise ValueError("evidence Git tree contains a non-regular blob")
        tree_rows.append((path, object_id))
    if tree_rows != delta_rows:
        raise ValueError("evidence Git tree differs from the publication delta")

    lexical_directory = _owner_controlled_directory_chain(
        repository,
        evidence_directory,
        label="evidence publication directory",
    )
    with os.scandir(lexical_directory) as entries:
        observed_names = sorted(entry.name for entry in entries)
    expected_names = sorted(relative.name for relative in relatives)
    if observed_names != expected_names:
        raise ValueError("evidence publication directory roster differs")

    rows: list[dict[str, Any]] = []
    snapshots: list[tuple[Path, bytes]] = []
    total_bytes = 0
    tree_by_path = dict(tree_rows)
    file_identities: set[tuple[int, int]] = set()
    for relative, path in zip(relatives, expected_paths, strict=True):
        working_path = repository.joinpath(*relative.parts)
        if working_path.resolve(strict=True) != working_path:
            raise ValueError(f"evidence path traverses a link: {path}")
        payload, identity = _snapshot_regular_file(working_path, max_file_bytes)
        if stat.S_IMODE(identity[2]) & 0o111:
            raise ValueError(
                f"evidence filesystem mode differs from Git 100644: {path}"
            )
        inode = (identity[0], identity[1])
        if inode in file_identities:
            raise ValueError("evidence publication files do not have unique identities")
        file_identities.add(inode)
        total_bytes += len(payload)
        if total_bytes > max_total_bytes:
            raise ValueError("evidence publication exceeds its total byte bound")
        blob = tree_by_path[path]
        committed_payload = _run_git(repository, ["cat-file", "blob", blob])
        current_blob = _git_text(
            repository,
            ["hash-object", "--no-filters", "--", path],
            "evidence working-tree blob",
        )
        if payload != committed_payload or current_blob != blob:
            raise ValueError(f"evidence bytes differ from the Git blob: {path}")
        rows.append(
            {
                "path": path,
                "size_bytes": len(payload),
                "sha256": digest_bytes(payload),
                "git_mode": "100644",
                "git_blob": blob,
            }
        )
        snapshots.append((working_path, payload))
    return {
        "source": {
            "repository": publication_identity["repository"],
            "commit": source_revision,
            "tree": source_tree,
            "object_format": object_format,
        },
        "publication": {
            **publication_identity,
            "parent_commit": source_revision,
            "policy": EVIDENCE_PUBLICATION_POLICY,
            "evidence_directory": directory_text,
            "files": rows,
            "file_count": len(rows),
            "roster_sha256": digest_bytes(
                EVIDENCE_PUBLICATION_ROSTER_DOMAIN + canonical(rows)
            ),
        },
        "snapshots": snapshots,
    }


def capture_evidence_publication(
    repository: Path,
    source_revision: str,
    publication_revision: str,
    relatives: list[Path] | tuple[Path, ...],
    max_file_bytes: int,
    *,
    max_total_bytes: int = MAX_IMPORTED_SOURCE_TOTAL_BYTES,
) -> dict[str, Any]:
    """Bind one exact evidence-only child commit to its clean publication."""

    repository = _resolved_repository(repository)
    publication_identity = capture_repository_identity(
        repository,
        publication_revision,
    )
    object_format = publication_identity["object_format"]
    if (
        source_revision == publication_revision
        or not valid_git_object(source_revision, object_format)
        or isinstance(max_file_bytes, bool)
        or not isinstance(max_file_bytes, int)
        or max_file_bytes < 1
        or isinstance(max_total_bytes, bool)
        or not isinstance(max_total_bytes, int)
        or max_total_bytes < max_file_bytes
        or not isinstance(relatives, (list, tuple))
        or not 1 <= len(relatives) <= 64
    ):
        raise ValueError("evidence publication arguments differ")
    relative_tuple = tuple(relatives)
    first = _publication_snapshot(
        repository,
        source_revision,
        publication_identity,
        relative_tuple,
        max_file_bytes,
        max_total_bytes,
    )
    second_identity = capture_repository_identity(repository, publication_revision)
    second = _publication_snapshot(
        repository,
        source_revision,
        second_identity,
        relative_tuple,
        max_file_bytes,
        max_total_bytes,
    )
    if (
        second_identity != publication_identity
        or first["source"] != second["source"]
        or first["publication"] != second["publication"]
        or any(
            snapshot_regular_file(path, max_file_bytes) != payload
            for path, payload in first["snapshots"]
        )
    ):
        raise ValueError("evidence publication changed during independent re-open")
    return {
        "source": first["source"],
        "publication": first["publication"],
    }


def capture_imported_source_roster(
    repository: Path,
    expected_revision: str,
) -> list[dict[str, Any]]:
    """Bind every loaded, tracked Engram Python source to one exact commit."""

    repository = _resolved_repository(repository)
    identity = capture_repository_identity(repository, expected_revision)
    paths_to_modules: dict[Path, set[str]] = {}
    for module_name, module in tuple(sys.modules.items()):
        project_module = _is_engram_project_module(module_name)
        paths = _module_source_paths(module)
        if paths is None:
            if project_module and not _valid_engram_namespace_module(
                module_name,
                module,
                repository,
            ):
                raise ValueError("imported Engram module has no Python source path")
            continue
        lexical, resolved = paths
        try:
            lexical_relative = lexical.relative_to(repository)
        except ValueError:
            lexical_relative = None
        try:
            resolved_relative = resolved.relative_to(repository)
        except ValueError:
            resolved_relative = None
        if lexical_relative is not None and resolved_relative is None:
            raise ValueError("imported Engram source path escapes through a link")
        if lexical_relative is not None and lexical != resolved:
            raise ValueError("imported Engram source path traverses a link")
        relative = resolved_relative
        if relative is None:
            if project_module:
                raise ValueError(
                    "imported Engram module resolved outside the repository"
                )
            continue
        if relative.parts and relative.parts[0] in IGNORED_REPOSITORY_DIRECTORIES:
            if project_module:
                raise ValueError(
                    "imported Engram module resolved into an ignored directory"
                )
            continue
        if resolved.suffix != ".py":
            if project_module:
                raise ValueError(
                    "imported Engram module is not backed by Python source"
                )
            continue
        paths_to_modules.setdefault(relative, set()).add(module_name)
    if not paths_to_modules:
        raise ValueError("no imported Engram project sources were observed")
    if len(paths_to_modules) > MAX_IMPORTED_SOURCE_COUNT:
        raise ValueError("imported Engram source roster exceeds its file bound")

    roster: list[dict[str, Any]] = []
    total_bytes = 0
    for relative in sorted(paths_to_modules, key=lambda value: value.as_posix()):
        if not 1 <= len(relative.as_posix()) <= 512:
            raise ValueError("imported Engram source path exceeds its bound")
        source_path = repository / relative
        payload = snapshot_regular_file(
            source_path,
            MAX_IMPORTED_SOURCE_BYTES,
            allow_empty=True,
        )
        total_bytes += len(payload)
        if total_bytes > MAX_IMPORTED_SOURCE_TOTAL_BYTES:
            raise ValueError("imported Engram source roster exceeds its byte bound")
        recorded_size_text = (
            _run_git(
                repository,
                ["cat-file", "-s", f"{expected_revision}:{relative.as_posix()}"],
            )
            .decode("ascii")
            .strip()
        )
        if not recorded_size_text.isascii() or not recorded_size_text.isdigit():
            raise ValueError(f"imported Engram Git size differs: {relative}")
        recorded_size = int(recorded_size_text)
        if not 0 <= recorded_size <= MAX_IMPORTED_SOURCE_BYTES:
            raise ValueError(
                f"imported Engram Git source exceeds its bound: {relative}"
            )
        recorded_payload = _run_git(
            repository,
            ["show", f"{expected_revision}:{relative.as_posix()}"],
        )
        if len(recorded_payload) != recorded_size or payload != recorded_payload:
            raise ValueError(
                f"imported Engram source differs from expected revision: {relative}"
            )
        recorded_blob = (
            _run_git(
                repository,
                ["rev-parse", f"{expected_revision}:{relative.as_posix()}"],
            )
            .decode("ascii")
            .strip()
        )
        current_blob = (
            _run_git(
                repository,
                ["hash-object", "--stdin"],
                input_bytes=payload,
            )
            .decode("ascii")
            .strip()
        )
        if recorded_blob != current_blob or not valid_git_object(
            recorded_blob,
            identity["object_format"],
        ):
            raise ValueError(f"imported Engram Git blob differs: {relative}")
        module_names = sorted(paths_to_modules[relative])
        if (
            not module_names
            or len(module_names) > 16
            or any(not 1 <= len(module_name) <= 256 for module_name in module_names)
        ):
            raise ValueError(f"imported Engram module alias roster differs: {relative}")
        roster.append(
            {
                "path": relative.as_posix(),
                "sha256": digest_bytes(payload),
                "git_blob": recorded_blob,
                "byte_count": len(payload),
                "module_names": module_names,
            }
        )
    return roster


def imported_source_roster_sha256(roster: list[dict[str, Any]]) -> str:
    """Digest one ordered imported-source roster under a local domain."""

    return digest_bytes(
        b"prisoma-engram-imported-source-roster-v1\0" + canonical(roster)
    )


def verify_imported_source_roster_unchanged(
    repository: Path,
    expected_revision: str,
    expected_roster: list[dict[str, Any]],
) -> None:
    """Reject source bytes, imports, aliases, or revision drift."""

    if capture_imported_source_roster(repository, expected_revision) != expected_roster:
        raise ValueError("imported Engram source roster changed during execution")


def verify_committed_source_roster(
    repository: Path,
    expected_revision: str,
    rows: list[dict[str, Any]],
    *,
    path_field: str = "relative_path",
    size_field: str = "size_bytes",
    sha256_field: str = "sha256",
    mode_field: str | None = "git_mode",
    blob_field: str = "git_blob",
    max_files: int = 256,
    max_file_bytes: int = MAX_IMPORTED_SOURCE_BYTES,
    max_total_bytes: int = MAX_IMPORTED_SOURCE_TOTAL_BYTES,
    allow_empty: bool = False,
    checkout_revision: str | None = None,
) -> list[dict[str, Any]]:
    """Reopen and verify a generic committed source roster.

    A ``None`` mode field reopens the raw Git tree entry and accepts only a
    regular non-symlink blob mode. The bound commit and tree retain mode
    identity when a projection intentionally omits that field.
    """

    repository = _resolved_repository(repository)
    if mode_field is not None and (not isinstance(mode_field, str) or not mode_field):
        raise ValueError("committed source mode field differs")
    current_revision = checkout_revision or expected_revision
    identity = capture_repository_identity(repository, current_revision)
    if not valid_git_object(expected_revision, identity["object_format"]):
        raise ValueError("source revision differs from the Git object format")
    resolved_revision = _git_text(
        repository,
        ["rev-parse", "--verify", f"{expected_revision}^{{commit}}"],
        "source revision",
    )
    if resolved_revision != expected_revision:
        raise ValueError("source revision does not resolve exactly")
    if not isinstance(rows, list) or not 1 <= len(rows) <= max_files:
        raise ValueError("committed source roster exceeds its file bound")
    paths: list[str] = []
    total_bytes = 0
    snapshots: list[tuple[Path, bytes]] = []
    for row in rows:
        if not isinstance(row, dict):
            raise ValueError("committed source row is not an object")
        path_value = row.get(path_field)
        if (
            not isinstance(path_value, str)
            or not path_value
            or len(path_value) > 4096
            or "\\" in path_value
            or "\0" in path_value
        ):
            raise ValueError("committed source path is not canonical")
        relative = Path(path_value)
        if (
            relative.is_absolute()
            or relative.as_posix() != path_value
            or any(part in {"", ".", ".."} for part in relative.parts)
        ):
            raise ValueError("committed source path is not canonical")
        size = row.get(size_field)
        if (
            isinstance(size, bool)
            or not isinstance(size, int)
            or size < (0 if allow_empty else 1)
            or size > max_file_bytes
            or (
                mode_field is not None
                and row.get(mode_field) not in {"100644", "100755"}
            )
            or not isinstance(row.get(sha256_field), str)
            or re.fullmatch(r"[0-9a-f]{64}", row[sha256_field]) is None
            or not valid_git_object(row.get(blob_field), identity["object_format"])
        ):
            raise ValueError("committed source row identity differs")
        source = repository.joinpath(*relative.parts)
        if source.resolve(strict=True) != source:
            raise ValueError(f"committed source path traverses a link: {path_value}")
        payload = snapshot_regular_file(
            source,
            max_file_bytes,
            allow_empty=allow_empty,
        )
        tree_row = _run_git(
            repository,
            ["ls-tree", "-z", expected_revision, "--", path_value],
        )
        expected_modes = (
            (row[mode_field],) if mode_field is not None else ("100644", "100755")
        )
        expected_tree_rows = {
            f"{mode} blob {row[blob_field]}\t{path_value}\0".encode()
            for mode in expected_modes
        }
        current_blob = _git_text(
            repository,
            ["hash-object", "--no-filters", "--", path_value],
            "current source Git blob",
        )
        if (
            len(payload) != size
            or digest_bytes(payload) != row[sha256_field]
            or tree_row not in expected_tree_rows
            or current_blob != row[blob_field]
        ):
            raise ValueError(f"committed source bytes differ: {path_value}")
        paths.append(path_value)
        total_bytes += len(payload)
        snapshots.append((source, payload))
        if total_bytes > max_total_bytes:
            raise ValueError("committed source roster exceeds its byte bound")
    if paths != sorted(set(paths)):
        raise ValueError("committed source roster is not sorted and unique")
    if capture_repository_identity(repository, current_revision) != identity:
        raise ValueError("repository identity changed during source verification")
    for path, payload in snapshots:
        if (
            snapshot_regular_file(
                path,
                max_file_bytes,
                allow_empty=allow_empty,
            )
            != payload
        ):
            raise ValueError("committed source changed during independent re-open")
    return rows
