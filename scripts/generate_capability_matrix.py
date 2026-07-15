#!/usr/bin/env python3
"""Generate and drift-check Prisoma's current capability matrix.

The catalog in ``protocols/capability_catalog_v1.json`` is the reviewed source of row
semantics.  This script validates that catalog, resolves every evidence path, and binds each
row to deterministic SHA-256 content revisions.  It deliberately does not embed the current
Git commit: a generated file cannot contain the hash of the commit that will contain it without
creating a self-reference.

Usage::

    python scripts/generate_capability_matrix.py --write
    python scripts/generate_capability_matrix.py --check
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import selectors
import shlex
import signal
import stat
import subprocess
import sys
import tempfile
import time
import tomllib
import urllib.parse
from collections import Counter
from contextlib import contextmanager
from contextvars import ContextVar
from datetime import date
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[1]
CATALOG_PATH = REPO_ROOT / "protocols/capability_catalog_v1.json"
JSON_OUTPUT = REPO_ROOT / "protocols/capability_matrix_current_v1.json"
MARKDOWN_OUTPUT = REPO_ROOT / "docs/CAPABILITY_MATRIX.md"

ALLOWED_STATUSES = {"implemented", "tested", "validated", "specified", "deferred"}
ALLOWED_EVIDENCE_LEVELS = {f"E{level}" for level in range(6)}
EVIDENCE_BASIS_TO_LEVEL = {
    "intention_or_local_only": "E0",
    "interface_specification": "E1",
    "declared_immutable_dependency": "E2",
    "build_tested_adapter": "E3",
    "end_to_end_scientific_conformance": "E4",
    "independent_replication": "E5",
}
ALLOWED_DEPENDENCIES = {"required", "conditional", "optional", "not_on_thesis_path"}
ALLOWED_CLAIMS = {"EC1", "H1", "H2", "H3", "H4"}
EXCLUDED_TREE_PARTS = {
    ".git",
    ".pytest_cache",
    ".ruff_cache",
    "__pycache__",
    "target",
}
FEATURE_ID_RE = re.compile(r"^[a-z0-9]+(?:[._-][a-z0-9]+)*$")
JUST_RECIPE_RE = re.compile(r"^([A-Za-z0-9][A-Za-z0-9_-]*)(?:\s[^:]*)?:(?!=)")
GIT_REVISION_RE = re.compile(
    r"^(?P<name>[A-Za-z0-9_.-]+)@"
    r"(?:(?P<selector>[A-Za-z0-9][A-Za-z0-9._/+:-]*)#)?"
    r"(?P<commit>[0-9a-f]{40})$"
)
REGISTRY_REVISION_RE = re.compile(
    r"^(?P<name>[A-Za-z0-9_.-]+)@"
    r"(?P<version>[0-9]+\.[0-9]+\.[0-9]+(?:[-+][A-Za-z0-9_.-]+)?)"
    r"#sha256:(?P<checksum>[0-9a-f]{64})$"
)
GIT_REPOSITORY_URLS = {
    "pid-rs": frozenset(
        {
            "git@github.com:sepahead/pid-rs.git",
            "https://github.com/sepahead/pid-rs",
            "https://github.com/sepahead/pid-rs.git",
        }
    ),
    "NCP": frozenset(
        {
            "https://github.com/sepahead/NCP",
            "https://github.com/sepahead/NCP.git",
        }
    ),
}
GITLINK_REPOSITORY_PATHS = {"pid-rs": "pid-rs"}

REQUIRED_FEATURE_IDS = frozenset(
    {
        "analysis.estimate_abstention",
        "bridge.agent_control_plane",
        "capture.offline_vlda_harness",
        "capture.safe_reference_adapter",
        "docs.capability_matrix",
        "ecosystem.evidence_ledger",
        "experiment.ec1_external_benchmark",
        "experiment.h1_common_preflight",
        "experiment.h1_protocol_a_reference",
        "experiment.h1_protocol_b",
        "experiment.h2_real_prospective",
        "experiment.h2_reference",
        "experiment.h4_attribution_reference",
        "governance.m0_research_ledgers",
        "integration.standard_format_adapters",
        "integration.structurally_different_adapter",
        "observer.ncp_fault_observatory",
        "observer.ncp_read_only",
        "pid.estimator_core",
        "proposal.gauss_mi_covariate",
        "proposal.gauss_mi_weighted_pid",
        "proposal.worldwarp_external",
        "replay.canonical_runlog",
        "sim.rapier_backend",
        "ui.tauri_sparkjs_shell",
        "viewer.rerun_adapter",
        "viewer.rerun_full_blueprint",
    }
)

TOP_LEVEL_KEYS = {
    "schema_version",
    "as_of_date",
    "canonical_spec",
    "scope",
    "rows",
}
CANONICAL_SPEC_KEYS = {"path", "version", "sections"}
ROW_KEYS = {
    "feature_id",
    "feature",
    "status",
    "revision_inputs",
    "external_revision",
    "test_command",
    "evidence_artifacts",
    "known_limitations",
    "evidence_level",
    "evidence_basis",
    "evidence_scope",
    "thesis_dependency",
    "claim_ids",
}
MAX_CATALOG_BYTES = 4 * 1024 * 1024
MAX_LOCKFILE_BYTES = 16 * 1024 * 1024
MAX_TEXT_INPUT_BYTES = 16 * 1024 * 1024
MAX_REVISION_FILE_BYTES = 64 * 1024 * 1024
MAX_REVISION_TOTAL_BYTES = 256 * 1024 * 1024
MAX_REVISION_FILES = 20_000
MAX_TREE_ENTRIES = 50_000
MAX_GIT_OUTPUT_BYTES = 4 * 1024 * 1024
GIT_TIMEOUT_SECONDS = 30.0
MAX_GIT_AGGREGATE_OUTPUT_BYTES = 32 * 1024 * 1024
MAX_GIT_AGGREGATE_SECONDS = 120.0
MAX_GIT_COMMANDS = 256
MAX_GENERATED_OUTPUT_BYTES = 16 * 1024 * 1024
MAX_CATALOG_ROWS = 1_000
MAX_CONTENT_PATHS = 1_000

_GIT_DEADLINE: ContextVar[float | None] = ContextVar(
    "capability_git_deadline", default=None
)
_GIT_OUTPUT_USED: ContextVar[int] = ContextVar("capability_git_output_used", default=0)
_GIT_COMMANDS_USED: ContextVar[int] = ContextVar(
    "capability_git_commands_used", default=0
)


class CatalogError(ValueError):
    """The capability catalog is malformed, unsafe, or semantically inconsistent."""


def _read_regular_bytes(path: Path, *, max_bytes: int, context: str) -> bytes:
    try:
        before = path.lstat()
    except OSError as error:
        raise CatalogError(f"cannot inspect {context} {path}: {error}") from error
    if not stat.S_ISREG(before.st_mode):
        raise CatalogError(f"{context} must be a regular, non-symlink file: {path}")
    if before.st_size > max_bytes:
        raise CatalogError(f"{context} exceeds the {max_bytes}-byte limit: {path}")

    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(path, flags)
    except OSError as error:
        raise CatalogError(f"cannot open {context} {path}: {error}") from error
    try:
        opened = os.fstat(descriptor)
        if not stat.S_ISREG(opened.st_mode) or (opened.st_dev, opened.st_ino) != (
            before.st_dev,
            before.st_ino,
        ):
            raise CatalogError(f"{context} changed while opening: {path}")
        chunks: list[bytes] = []
        remaining = max_bytes + 1
        while remaining:
            chunk = os.read(descriptor, min(65_536, remaining))
            if not chunk:
                break
            chunks.append(chunk)
            remaining -= len(chunk)
        after = os.fstat(descriptor)
    finally:
        os.close(descriptor)
    payload = b"".join(chunks)
    if len(payload) > max_bytes:
        raise CatalogError(f"{context} exceeds the {max_bytes}-byte limit: {path}")
    if (opened.st_dev, opened.st_ino, opened.st_size, opened.st_mtime_ns) != (
        after.st_dev,
        after.st_ino,
        after.st_size,
        after.st_mtime_ns,
    ) or len(payload) != after.st_size:
        raise CatalogError(f"{context} changed while reading: {path}")
    return payload


def _read_regular_text(path: Path, *, max_bytes: int, context: str) -> str:
    try:
        return _read_regular_bytes(path, max_bytes=max_bytes, context=context).decode(
            "utf-8"
        )
    except UnicodeDecodeError as error:
        raise CatalogError(f"{context} is not valid UTF-8: {path}") from error


def _terminate(process: subprocess.Popen[bytes]) -> None:
    try:
        if os.name == "posix":
            os.killpg(process.pid, signal.SIGKILL)
        elif process.poll() is None:
            process.kill()
    except OSError:
        try:
            process.kill()
        except OSError:
            pass


@contextmanager
def _git_budget():
    if _GIT_DEADLINE.get() is not None:
        yield
        return
    deadline_token = _GIT_DEADLINE.set(time.monotonic() + MAX_GIT_AGGREGATE_SECONDS)
    output_token = _GIT_OUTPUT_USED.set(0)
    command_token = _GIT_COMMANDS_USED.set(0)
    try:
        yield
    finally:
        _GIT_COMMANDS_USED.reset(command_token)
        _GIT_OUTPUT_USED.reset(output_token)
        _GIT_DEADLINE.reset(deadline_token)


def _run_bounded(
    command: list[str],
    *,
    cwd: Path,
    timeout_seconds: float = GIT_TIMEOUT_SECONDS,
    max_output_bytes: int = MAX_GIT_OUTPUT_BYTES,
    text: bool = True,
) -> subprocess.CompletedProcess[str] | subprocess.CompletedProcess[bytes]:
    """Run Git with aggregate output and wall-time limits."""

    aggregate_deadline = _GIT_DEADLINE.get()
    if aggregate_deadline is not None:
        commands_used = _GIT_COMMANDS_USED.get()
        if commands_used >= MAX_GIT_COMMANDS:
            raise CatalogError(
                f"Git validation exceeds the {MAX_GIT_COMMANDS}-command aggregate limit"
            )
        _GIT_COMMANDS_USED.set(commands_used + 1)
    process = subprocess.Popen(
        command,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        start_new_session=os.name == "posix",
    )
    assert process.stdout is not None and process.stderr is not None
    selector = selectors.DefaultSelector()
    buffers = {"stdout": bytearray(), "stderr": bytearray()}
    total = 0
    deadline = time.monotonic() + timeout_seconds
    if aggregate_deadline is not None:
        deadline = min(deadline, aggregate_deadline)
    try:
        selector.register(process.stdout, selectors.EVENT_READ, "stdout")
        selector.register(process.stderr, selectors.EVENT_READ, "stderr")
        while selector.get_map():
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise subprocess.TimeoutExpired(command, timeout_seconds)
            for key, _events in selector.select(min(remaining, 0.25)):
                chunk = os.read(key.fd, min(65_536, max_output_bytes - total + 1))
                if not chunk:
                    selector.unregister(key.fileobj)
                    key.fileobj.close()
                    continue
                total += len(chunk)
                if total > max_output_bytes:
                    raise CatalogError(
                        "Git subprocess output exceeds the aggregate "
                        f"{max_output_bytes}-byte limit"
                    )
                if aggregate_deadline is not None:
                    aggregate_used = _GIT_OUTPUT_USED.get() + len(chunk)
                    if aggregate_used > MAX_GIT_AGGREGATE_OUTPUT_BYTES:
                        raise CatalogError(
                            "Git validation exceeds the aggregate "
                            f"{MAX_GIT_AGGREGATE_OUTPUT_BYTES}-byte output limit"
                        )
                    _GIT_OUTPUT_USED.set(aggregate_used)
                buffers[key.data].extend(chunk)
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise subprocess.TimeoutExpired(command, timeout_seconds)
        return_code = process.wait(timeout=remaining)
    except BaseException:
        _terminate(process)
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            pass
        raise
    finally:
        selector.close()
        for stream in (process.stdout, process.stderr):
            if not stream.closed:
                stream.close()

    stdout_bytes = bytes(buffers["stdout"])
    stderr_bytes = bytes(buffers["stderr"])
    if text:
        try:
            stdout: str | bytes = stdout_bytes.decode("utf-8")
            stderr: str | bytes = stderr_bytes.decode("utf-8")
        except UnicodeDecodeError as error:
            raise CatalogError("Git subprocess output is not valid UTF-8") from error
    else:
        stdout = stdout_bytes
        stderr = stderr_bytes
    if return_code != 0:
        raise subprocess.CalledProcessError(
            return_code, command, output=stdout, stderr=stderr
        )
    return subprocess.CompletedProcess(command, return_code, stdout, stderr)


def _object_without_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            raise CatalogError(f"duplicate JSON key {key!r}")
        value[key] = item
    return value


def _reject_json_constant(value: str) -> Any:
    raise CatalogError(f"invalid JSON numeric constant {value}")


def _require_exact_keys(
    value: dict[str, Any], expected: set[str], context: str
) -> None:
    missing = sorted(expected - value.keys())
    unknown = sorted(value.keys() - expected)
    if missing or unknown:
        details = []
        if missing:
            details.append(f"missing={missing}")
        if unknown:
            details.append(f"unknown={unknown}")
        raise CatalogError(f"{context} has invalid fields: {', '.join(details)}")


def _nonempty_string(value: Any, context: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise CatalogError(f"{context} must be a non-empty string")
    return value


def _toml_packages(lock: dict[str, Any], *, context: str) -> list[dict[str, Any]]:
    packages = lock.get("package")
    if not isinstance(packages, list):
        raise CatalogError(f"{context}.package must be a list")
    validated: list[dict[str, Any]] = []
    for index, package in enumerate(packages):
        if not isinstance(package, dict):
            raise CatalogError(f"{context}.package[{index}] must be a table")
        for field in ("name", "version"):
            if not isinstance(package.get(field), str) or not package[field]:
                raise CatalogError(
                    f"{context}.package[{index}].{field} must be a non-empty string"
                )
        for field in ("source", "checksum"):
            if package.get(field) is not None and not isinstance(package[field], str):
                raise CatalogError(
                    f"{context}.package[{index}].{field} must be null or a string"
                )
        validated.append(package)
    return validated


def _relative_path(raw: str, *, root: Path, context: str) -> Path:
    if "\\" in raw:
        raise CatalogError(
            f"{context} must use repository-relative POSIX separators: {raw!r}"
        )
    relative = Path(raw)
    if relative.is_absolute() or ".." in relative.parts:
        raise CatalogError(f"{context} escapes the repository: {raw!r}")

    candidate = root / relative
    component = root
    for part in relative.parts:
        component /= part
        if component.is_symlink():
            raise CatalogError(f"{context} may not traverse a symlink: {raw!r}")
    try:
        resolved = candidate.resolve(strict=True)
    except FileNotFoundError as error:
        raise CatalogError(f"{context} does not exist: {raw!r}") from error
    try:
        resolved.relative_to(root.resolve(strict=True))
    except ValueError as error:
        raise CatalogError(
            f"{context} resolves outside the repository: {raw!r}"
        ) from error
    return resolved


def _artifact_base(raw: str) -> str:
    if not isinstance(raw, str) or not raw.strip():
        raise CatalogError("evidence artifact paths must be non-empty strings")
    base, separator, fragment = raw.partition("#")
    if separator and not fragment:
        raise CatalogError(f"evidence artifact has an empty fragment: {raw!r}")
    if "?" in base:
        raise CatalogError(f"evidence artifact queries are not supported: {raw!r}")
    return base


def _tree_files(path: Path, *, root: Path, context: str) -> list[Path]:
    if path.is_file():
        return [path]
    if not path.is_dir():
        raise CatalogError(f"{context} is neither a regular file nor directory")

    files: list[Path] = []
    visited_entries = 0
    for directory, child_directories, filenames in os.walk(
        path, topdown=True, followlinks=False
    ):
        visited_entries += len(child_directories) + len(filenames)
        if visited_entries > MAX_TREE_ENTRIES:
            raise CatalogError(
                f"{context} exceeds the {MAX_TREE_ENTRIES}-entry traversal limit"
            )
        directory_path = Path(directory)
        retained_directories: list[str] = []
        for name in sorted(child_directories):
            candidate = directory_path / name
            relative = candidate.relative_to(root)
            if any(part in EXCLUDED_TREE_PARTS for part in relative.parts):
                continue
            if candidate.is_symlink():
                raise CatalogError(
                    f"{context} contains a symlink: {relative.as_posix()!r}"
                )
            retained_directories.append(name)
        child_directories[:] = retained_directories

        for name in sorted(filenames):
            candidate = directory_path / name
            relative = candidate.relative_to(root)
            if any(part in EXCLUDED_TREE_PARTS for part in relative.parts):
                continue
            if candidate.is_symlink():
                raise CatalogError(
                    f"{context} contains a symlink: {relative.as_posix()!r}"
                )
            if candidate.is_file():
                files.append(candidate)
                if len(files) > MAX_REVISION_FILES:
                    raise CatalogError(
                        f"{context} exceeds the {MAX_REVISION_FILES}-file limit"
                    )
    if not files:
        raise CatalogError(f"{context} contains no revisionable files")
    return files


def _content_files(raw_paths: list[str], *, root: Path) -> list[Path]:
    if (
        not isinstance(raw_paths, list)
        or not raw_paths
        or len(raw_paths) > MAX_CONTENT_PATHS
    ):
        raise CatalogError("content paths must be a non-empty list")
    for index, raw in enumerate(raw_paths):
        _nonempty_string(raw, f"content_paths[{index}]")
    if len(raw_paths) != len(set(raw_paths)):
        raise CatalogError("content paths must not contain duplicates")

    files: set[Path] = set()
    for raw in raw_paths:
        resolved = _relative_path(raw, root=root, context=f"revision input {raw!r}")
        files.update(
            _tree_files(resolved, root=root, context=f"revision input {raw!r}")
        )
        if len(files) > MAX_REVISION_FILES:
            raise CatalogError(
                f"content paths exceed the {MAX_REVISION_FILES}-file limit"
            )
    return sorted(files, key=lambda item: item.relative_to(root).as_posix())


def _reject_generated_content(
    raw_paths: list[str], *, root: Path, context: str
) -> None:
    generated_relatives = {
        JSON_OUTPUT.relative_to(REPO_ROOT),
        MARKDOWN_OUTPUT.relative_to(REPO_ROOT),
    }
    generated = {(root / relative).resolve() for relative in generated_relatives}
    resolved_roots = {
        _relative_path(raw, root=root, context=f"{context} path {raw!r}")
        for raw in raw_paths
    }
    included = {
        output
        for output in generated
        if any(
            output == candidate or output.is_relative_to(candidate)
            for candidate in resolved_roots
        )
    }
    included.update(set(_content_files(raw_paths, root=root)).intersection(generated))
    if included:
        names = sorted(path.relative_to(root.resolve()).as_posix() for path in included)
        raise CatalogError(f"{context} may not include generated outputs: {names}")


def content_digest(raw_paths: list[str], *, root: Path) -> str:
    """Hash the named files/directories with path and byte boundaries."""

    digest = hashlib.sha256(b"prisoma-capability-content-v1\0")
    aggregate_bytes = 0
    for path in _content_files(raw_paths, root=root):
        relative = path.relative_to(root).as_posix().encode("utf-8")
        payload = _read_regular_bytes(
            path,
            max_bytes=MAX_REVISION_FILE_BYTES,
            context="revision input",
        )
        aggregate_bytes += len(payload)
        if aggregate_bytes > MAX_REVISION_TOTAL_BYTES:
            raise CatalogError(
                "revision inputs exceed the aggregate "
                f"{MAX_REVISION_TOTAL_BYTES}-byte limit"
            )
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        digest.update(len(payload).to_bytes(8, "big"))
        digest.update(payload)
    return digest.hexdigest()


def _validate_registry_revision(
    match: re.Match[str],
    raw_paths: list[str],
    *,
    root: Path,
    context: str,
) -> None:
    lockfiles = [
        path
        for path in _content_files(raw_paths, root=root)
        if path.name == "Cargo.lock"
    ]
    if not lockfiles:
        raise CatalogError(
            f"{context} registry revision has no Cargo.lock revision input"
        )

    expected = (
        match.group("name"),
        match.group("version"),
        match.group("checksum"),
    )
    for lockfile in lockfiles:
        try:
            lock = tomllib.loads(
                _read_regular_text(
                    lockfile,
                    max_bytes=MAX_LOCKFILE_BYTES,
                    context="registry lockfile",
                )
            )
        except (OSError, tomllib.TOMLDecodeError, RecursionError) as error:
            raise CatalogError(
                f"cannot parse registry lockfile {lockfile}: {error}"
            ) from error
        for package in _toml_packages(lock, context="registry lockfile"):
            actual = (
                package.get("name"),
                package.get("version"),
                package.get("checksum"),
            )
            source = package.get("source")
            if (
                actual == expected
                and isinstance(source, str)
                and source.startswith("registry+")
            ):
                return
    raise CatalogError(
        f"{context} registry revision does not match any exact package in its Cargo.lock inputs"
    )


def _gitmodule_urls(*, root: Path) -> dict[str, str]:
    gitmodules = root / ".gitmodules"
    if not gitmodules.is_file() or gitmodules.is_symlink():
        return {}
    try:
        paths = _run_bounded(
            [
                "git",
                "config",
                "--file",
                str(gitmodules),
                "--get-regexp",
                r"^submodule\..*\.path$",
            ],
            cwd=root,
        ).stdout.splitlines()
    except (CatalogError, OSError, subprocess.SubprocessError):
        return {}

    urls: dict[str, str] = {}
    for line in paths:
        try:
            key, path = line.split(maxsplit=1)
            url_key = f"{key.removesuffix('.path')}.url"
            url = _run_bounded(
                ["git", "config", "--file", str(gitmodules), "--get", url_key],
                cwd=root,
            ).stdout.strip()
        except (CatalogError, ValueError, OSError, subprocess.SubprocessError):
            continue
        if path and url:
            urls[path] = url
    return urls


def _bound_gitlinks(raw_paths: list[str], *, root: Path) -> list[tuple[str, str, str]]:
    """Return tracked gitlinks that contain at least one declared revision input."""

    if ".gitmodules" not in raw_paths:
        return []
    configured_urls = _gitmodule_urls(root=root)

    try:
        result = _run_bounded(
            ["git", "-C", str(root), "ls-files", "--stage", "-z"],
            cwd=root,
            text=False,
        )
    except (CatalogError, OSError, subprocess.SubprocessError):
        return []

    gitlinks: list[tuple[str, str, str]] = []
    for record in result.stdout.split(b"\0"):
        if not record:
            continue
        try:
            metadata, raw_path = record.split(b"\t", 1)
            mode, commit, _stage = metadata.decode("ascii").split(" ", 2)
            path = raw_path.decode("utf-8")
        except (UnicodeDecodeError, ValueError):
            continue
        if mode != "160000":
            continue
        if not any(raw == path or raw.startswith(f"{path}/") for raw in raw_paths):
            continue
        try:
            checked_out = _run_bounded(
                ["git", "-C", str(root / path), "rev-parse", "HEAD"],
                cwd=root,
            ).stdout.strip()
        except (CatalogError, OSError, subprocess.SubprocessError):
            continue
        try:
            dirty = _run_bounded(
                [
                    "git",
                    "-C",
                    str(root / path),
                    "status",
                    "--porcelain=v1",
                    "--untracked-files=all",
                ],
                cwd=root,
            ).stdout
        except (CatalogError, OSError, subprocess.SubprocessError):
            continue
        configured_url = configured_urls.get(path)
        if checked_out == commit and not dirty and configured_url is not None:
            gitlinks.append((path, commit, configured_url))
    return gitlinks


def _git_lock_sources(raw_paths: list[str], *, root: Path) -> list[str]:
    sources: list[str] = []
    for lockfile in (
        path
        for path in _content_files(raw_paths, root=root)
        if path.name == "Cargo.lock"
    ):
        try:
            lock = tomllib.loads(
                _read_regular_text(
                    lockfile,
                    max_bytes=MAX_LOCKFILE_BYTES,
                    context="Git-source lockfile",
                )
            )
        except (OSError, tomllib.TOMLDecodeError, RecursionError) as error:
            raise CatalogError(
                f"cannot parse Git-source lockfile {lockfile}: {error}"
            ) from error
        for package in _toml_packages(lock, context="Git-source lockfile"):
            source = package.get("source")
            if isinstance(source, str) and source.startswith("git+"):
                sources.append(source)
    return sources


def _validate_git_revision(
    match: re.Match[str],
    raw_paths: list[str],
    *,
    root: Path,
    context: str,
) -> None:
    """Bind an advertised Git revision to a declared gitlink or Cargo.lock source."""

    expected_name = match.group("name")
    expected_commit = match.group("commit")
    selector = match.group("selector")
    allowed_urls = GIT_REPOSITORY_URLS.get(expected_name)
    if allowed_urls is None:
        raise CatalogError(
            f"{context} names an unknown canonical Git repository {expected_name!r}"
        )

    for path, commit, configured_url in _bound_gitlinks(raw_paths, root=root):
        if (
            selector is None
            and GITLINK_REPOSITORY_PATHS.get(expected_name) == path
            and commit == expected_commit
            and configured_url in allowed_urls
        ):
            return

    for source in _git_lock_sources(raw_paths, root=root):
        parsed = urllib.parse.urlsplit(source.removeprefix("git+"))
        repository_url = urllib.parse.urlunsplit(
            (parsed.scheme, parsed.netloc, parsed.path.rstrip("/"), "", "")
        )
        if repository_url not in allowed_urls or parsed.fragment != expected_commit:
            continue
        if selector is None:
            return
        query = urllib.parse.parse_qs(parsed.query, keep_blank_values=True)
        if selector in {
            value for key in ("tag", "rev", "branch") for value in query.get(key, [])
        }:
            return

    raise CatalogError(
        f"{context} does not match the canonical repository, commit, and selector in "
        "the declared clean checked-out gitlink or Cargo.lock revision inputs"
    )


def _validate_row(row: Any, *, index: int, root: Path) -> dict[str, Any]:
    context = f"rows[{index}]"
    if not isinstance(row, dict):
        raise CatalogError(f"{context} must be an object")
    _require_exact_keys(row, ROW_KEYS, context)

    feature_id = _nonempty_string(row["feature_id"], f"{context}.feature_id")
    if not FEATURE_ID_RE.fullmatch(feature_id):
        raise CatalogError(f"{context}.feature_id is not a stable lowercase identifier")
    _nonempty_string(row["feature"], f"{context}.feature")
    status = _nonempty_string(row["status"], f"{context}.status")
    if status not in ALLOWED_STATUSES:
        raise CatalogError(f"{context}.status is not one of {sorted(ALLOWED_STATUSES)}")

    evidence_level = _nonempty_string(
        row["evidence_level"], f"{context}.evidence_level"
    )
    if evidence_level not in ALLOWED_EVIDENCE_LEVELS:
        raise CatalogError(
            f"{context}.evidence_level is not one of {sorted(ALLOWED_EVIDENCE_LEVELS)}"
        )
    evidence_basis = _nonempty_string(
        row["evidence_basis"], f"{context}.evidence_basis"
    )
    expected_level = EVIDENCE_BASIS_TO_LEVEL.get(evidence_basis)
    if expected_level is None:
        raise CatalogError(
            f"{context}.evidence_basis is not one of {sorted(EVIDENCE_BASIS_TO_LEVEL)}"
        )
    if evidence_level != expected_level:
        raise CatalogError(
            f"{context}: evidence basis {evidence_basis!r} requires {expected_level}, "
            f"not {evidence_level}"
        )

    level = int(evidence_level[1:])
    if status == "deferred" and level != 0:
        raise CatalogError(f"{context}: deferred rows must remain E0")
    if status == "specified" and level > 1:
        raise CatalogError(f"{context}: specified rows cannot exceed E1")
    if status == "implemented" and level > 2:
        raise CatalogError(f"{context}: implemented-but-untested rows cannot exceed E2")
    if status == "tested" and level > 3:
        raise CatalogError(f"{context}: locally tested rows cannot claim E4 or E5")
    if status == "validated" and level not in {4, 5}:
        raise CatalogError(f"{context}: validated rows require E4 or E5 evidence")

    command = row["test_command"]
    if status in {"tested", "validated"}:
        command = _nonempty_string(command, f"{context}.test_command")
    elif status in {"implemented", "specified", "deferred"} and command is not None:
        raise CatalogError(
            f"{context}: rows without named test evidence may not advertise a test command"
        )
    elif command is not None:
        command = _nonempty_string(command, f"{context}.test_command")

    if command is not None:
        try:
            command_parts = shlex.split(command)
        except ValueError as error:
            raise CatalogError(
                f"{context}.test_command is not valid shell syntax"
            ) from error
        if command_parts and command_parts[0] == "just":
            if len(command_parts) < 2:
                raise CatalogError(f"{context}.test_command omits the just recipe")
            justfile = _relative_path(
                "justfile", root=root, context="test-command justfile"
            )
            recipes = {
                match.group(1)
                for line in _read_regular_text(
                    justfile,
                    max_bytes=MAX_TEXT_INPUT_BYTES,
                    context="test-command justfile",
                ).splitlines()
                if (match := JUST_RECIPE_RE.match(line)) is not None
            }
            if command_parts[1] not in recipes:
                raise CatalogError(
                    f"{context}.test_command names unknown just recipe {command_parts[1]!r}"
                )

    external_revision = row["external_revision"]
    git_matches: list[re.Match[str]] = []
    registry_matches: list[re.Match[str]] = []
    if external_revision is not None:
        external_revision = _nonempty_string(
            external_revision, f"{context}.external_revision"
        )
        revision_parts = external_revision.split("; ")
        if len(revision_parts) != len(set(revision_parts)) or len(revision_parts) > 16:
            raise CatalogError(
                f"{context}.external_revision contains duplicate or excessive identities"
            )
        for part in revision_parts:
            git_match = GIT_REVISION_RE.fullmatch(part)
            registry_match = REGISTRY_REVISION_RE.fullmatch(part)
            if git_match is None and registry_match is None:
                raise CatalogError(
                    f"{context}.external_revision must contain semicolon-separated exact "
                    "bounded 40-hex Git revisions or exact registry versions plus SHA-256"
                )
            if git_match is not None:
                git_matches.append(git_match)
            if registry_match is not None:
                registry_matches.append(registry_match)
    if level >= 2 and external_revision is None:
        raise CatalogError(
            f"{context}: E2 or higher requires an immutable external revision"
        )

    revision_inputs = row["revision_inputs"]
    if not isinstance(revision_inputs, list):
        raise CatalogError(f"{context}.revision_inputs must be a list")
    content_digest(revision_inputs, root=root)
    _reject_generated_content(
        revision_inputs,
        root=root,
        context=f"{context}.revision_inputs",
    )
    for registry_match in registry_matches:
        _validate_registry_revision(
            registry_match,
            revision_inputs,
            root=root,
            context=f"{context}.external_revision",
        )
    for git_match in git_matches:
        _validate_git_revision(
            git_match,
            revision_inputs,
            root=root,
            context=f"{context}.external_revision",
        )

    artifacts = row["evidence_artifacts"]
    if not isinstance(artifacts, list) or not artifacts:
        raise CatalogError(f"{context}.evidence_artifacts must be a non-empty list")
    for artifact_index, artifact in enumerate(artifacts):
        _nonempty_string(artifact, f"{context}.evidence_artifacts[{artifact_index}]")
    if len(artifacts) != len(set(artifacts)):
        raise CatalogError(f"{context}.evidence_artifacts must not contain duplicates")
    for artifact_index, artifact in enumerate(artifacts):
        base = _artifact_base(artifact)
        _relative_path(
            base,
            root=root,
            context=f"{context}.evidence_artifacts[{artifact_index}]",
        )
        _reject_generated_content(
            [base],
            root=root,
            context=f"{context}.evidence_artifacts[{artifact_index}]",
        )

    limitation = _nonempty_string(
        row["known_limitations"], f"{context}.known_limitations"
    )
    if limitation.strip().lower() in {"none", "n/a", "na"}:
        raise CatalogError(f"{context}.known_limitations must state a real boundary")
    _nonempty_string(row["evidence_scope"], f"{context}.evidence_scope")

    dependency = _nonempty_string(
        row["thesis_dependency"], f"{context}.thesis_dependency"
    )
    if dependency not in ALLOWED_DEPENDENCIES:
        raise CatalogError(
            f"{context}.thesis_dependency is not one of {sorted(ALLOWED_DEPENDENCIES)}"
        )

    claim_ids = row["claim_ids"]
    if not isinstance(claim_ids, list) or any(
        not isinstance(claim_id, str) for claim_id in claim_ids
    ):
        raise CatalogError(f"{context}.claim_ids must be a string list")
    if len(claim_ids) != len(set(claim_ids)):
        raise CatalogError(f"{context}.claim_ids must not contain duplicates")
    unknown_claims = sorted(set(claim_ids) - ALLOWED_CLAIMS)
    if unknown_claims:
        raise CatalogError(
            f"{context}.claim_ids contains unknown claims: {unknown_claims}"
        )
    return row


def _load_catalog(
    path: Path = CATALOG_PATH,
    *,
    root: Path = REPO_ROOT,
    required_feature_ids: frozenset[str] = REQUIRED_FEATURE_IDS,
    _payload: bytes | None = None,
) -> dict[str, Any]:
    try:
        payload = (
            _read_regular_bytes(
                path,
                max_bytes=MAX_CATALOG_BYTES,
                context="capability catalog",
            )
            if _payload is None
            else _payload
        )
        if len(payload) > MAX_CATALOG_BYTES:
            raise CatalogError(
                f"capability catalog exceeds the {MAX_CATALOG_BYTES}-byte limit"
            )
        catalog = json.loads(
            payload.decode("utf-8"),
            object_pairs_hook=_object_without_duplicate_keys,
            parse_constant=_reject_json_constant,
        )
    except (
        OSError,
        UnicodeDecodeError,
        ValueError,
        RecursionError,
    ) as error:
        raise CatalogError(f"cannot read capability catalog {path}: {error}") from error
    if not isinstance(catalog, dict):
        raise CatalogError("capability catalog must be a JSON object")
    _require_exact_keys(catalog, TOP_LEVEL_KEYS, "catalog")
    if catalog["schema_version"] != 1:
        raise CatalogError("capability catalog schema_version must be 1")
    try:
        date.fromisoformat(
            _nonempty_string(catalog["as_of_date"], "catalog.as_of_date")
        )
    except ValueError as error:
        raise CatalogError("catalog.as_of_date must be an ISO calendar date") from error
    _nonempty_string(catalog["scope"], "catalog.scope")

    canonical = catalog["canonical_spec"]
    if not isinstance(canonical, dict):
        raise CatalogError("catalog.canonical_spec must be an object")
    _require_exact_keys(canonical, CANONICAL_SPEC_KEYS, "catalog.canonical_spec")
    if canonical["path"] != "grandplan.md" or canonical["version"] != "12.5":
        raise CatalogError("catalog must bind the canonical grandplan.md v12.5 spec")
    sections = canonical["sections"]
    if not isinstance(sections, list) or not sections:
        raise CatalogError("catalog.canonical_spec.sections must be a non-empty list")
    for section_index, section in enumerate(sections):
        _nonempty_string(section, f"catalog.canonical_spec.sections[{section_index}]")
    _relative_path("grandplan.md", root=root, context="canonical spec")

    rows = catalog["rows"]
    if not isinstance(rows, list) or not rows or len(rows) > MAX_CATALOG_ROWS:
        raise CatalogError(f"catalog.rows must contain 1–{MAX_CATALOG_ROWS} entries")
    validated = [
        _validate_row(row, index=index, root=root) for index, row in enumerate(rows)
    ]
    identifiers = [row["feature_id"] for row in validated]
    names = [row["feature"] for row in validated]
    if len(identifiers) != len(set(identifiers)):
        raise CatalogError("catalog feature_id values must be unique")
    if len(names) != len(set(names)):
        raise CatalogError("catalog feature names must be unique")
    if identifiers != sorted(identifiers):
        raise CatalogError("catalog rows must be sorted by feature_id")
    missing_features = sorted(required_feature_ids - set(identifiers))
    if missing_features:
        raise CatalogError(f"catalog omits required capabilities: {missing_features}")
    return catalog


def load_catalog(
    path: Path = CATALOG_PATH,
    *,
    root: Path = REPO_ROOT,
    required_feature_ids: frozenset[str] = REQUIRED_FEATURE_IDS,
    _payload: bytes | None = None,
) -> dict[str, Any]:
    with _git_budget():
        return _load_catalog(
            path,
            root=root,
            required_feature_ids=required_feature_ids,
            _payload=_payload,
        )


def _effective_revision_inputs(row: dict[str, Any]) -> list[str]:
    inputs = list(row["revision_inputs"])
    command = row["test_command"]
    if (
        command is not None
        and shlex.split(command)[0] == "just"
        and "justfile" not in inputs
    ):
        inputs.append("justfile")
    return inputs


def resolve_catalog(
    catalog_path: Path = CATALOG_PATH,
    *,
    root: Path = REPO_ROOT,
    required_feature_ids: frozenset[str] = REQUIRED_FEATURE_IDS,
) -> dict[str, Any]:
    catalog_bytes = _read_regular_bytes(
        catalog_path,
        max_bytes=MAX_CATALOG_BYTES,
        context="capability catalog",
    )
    catalog = load_catalog(
        catalog_path,
        root=root,
        required_feature_ids=required_feature_ids,
        _payload=catalog_bytes,
    )
    source_relative = catalog_path.resolve().relative_to(root.resolve()).as_posix()
    resolved_rows: list[dict[str, Any]] = []

    for row in catalog["rows"]:
        effective_inputs = _effective_revision_inputs(row)
        local_digest = content_digest(effective_inputs, root=root)
        local_revision = f"local-sha256:{local_digest}"
        exact_revision = local_revision
        if row["external_revision"] is not None:
            exact_revision = f"{row['external_revision']}; {local_revision}"

        evidence = []
        for raw in row["evidence_artifacts"]:
            base = _artifact_base(raw)
            evidence.append(
                {
                    "path": raw,
                    "sha256": content_digest([base], root=root),
                }
            )

        resolved_rows.append(
            {
                "feature_id": row["feature_id"],
                "feature": row["feature"],
                "status": row["status"],
                "exact_revision": exact_revision,
                "revision_inputs": effective_inputs,
                "test_command": row["test_command"],
                "evidence_artifacts": evidence,
                "known_limitations": row["known_limitations"],
                "evidence_level": row["evidence_level"],
                "evidence_basis": row["evidence_basis"],
                "evidence_scope": row["evidence_scope"],
                "thesis_dependency": row["thesis_dependency"],
                "claim_ids": row["claim_ids"],
            }
        )

    return {
        "schema_version": 1,
        "as_of_date": catalog["as_of_date"],
        "canonical_spec": catalog["canonical_spec"],
        "scope": catalog["scope"],
        "source_catalog": {
            "path": source_relative,
            "sha256": hashlib.sha256(catalog_bytes).hexdigest(),
        },
        "hash_contract": (
            "SHA-256 over domain tag plus sorted repository-relative path and byte-length "
            "framed file contents; excluded tree parts are .git, target, __pycache__, "
            ".pytest_cache, and .ruff_cache; justfile is an implicit revision input for "
            "every advertised just recipe"
        ),
        "row_count": len(resolved_rows),
        "status_counts": dict(
            sorted(Counter(row["status"] for row in resolved_rows).items())
        ),
        "evidence_level_counts": dict(
            sorted(Counter(row["evidence_level"] for row in resolved_rows).items())
        ),
        "rows": resolved_rows,
    }


def _markdown_escape(value: str) -> str:
    return value.replace("\\", "\\\\").replace("|", "\\|").replace("\n", "<br>")


def _markdown_code(value: str) -> str:
    return f"<code>{_markdown_escape(value).replace('`', '&#96;')}</code>"


def render_markdown(matrix: dict[str, Any]) -> str:
    source = matrix["source_catalog"]
    validated_count = matrix["status_counts"].get("validated", 0)
    lines = [
        "<!-- generated by scripts/generate_capability_matrix.py; do not edit by hand -->",
        "",
        "# Current capability matrix",
        "",
        f"As of **{matrix['as_of_date']}**, generated offline from "
        f"[`{source['path']}`](../{source['path']}) "
        f"(SHA-256 `{source['sha256']}`). Regenerate with "
        "`python scripts/generate_capability_matrix.py --write`.",
        "",
        "This is a software/evidence inventory, not a scientific result. `tested` means "
        "behavior on the named local proof path only; it does not itself assign E3 relationship "
        "evidence or imply scientific conformance, deployment security, estimator application "
        "validity, or EC1/H1–H4 success. "
        f"The current matrix contains **{validated_count}** `validated` row(s).",
        "",
        "Status and evidence level are orthogonal. Status semantics are fail-closed: `implemented` "
        "= code/dependency without a named proof command; `tested` = named local proof; `validated` "
        "= E4/E5 end-to-end or independent evidence; `specified` = design/interface only; "
        "`deferred` = E0 unavailable/off-path, including rejected records, and is never a delivery "
        "promise. The E0–E5 column follows grandplan §8.9 strictly: local-only tested features stay "
        "E0, E2 requires an immutable external dependency, and E3 requires a pinned producer and "
        "consumer tested together against golden fixtures. Exact local revisions are content hashes, "
        "avoiding a self-referential future Git commit hash. The generator checks schema, canonical "
        "pins, paths, and hashes; catalog review and CI execution—not static command-text "
        "inspection—establish that a named proof actually exercises its declared inputs.",
        "",
        "| Feature | Status | Exact revision | Test command | Evidence artifact | Known limitations | Evidence level | Thesis dependency |",
        "|---|---|---|---|---|---|---|---|",
    ]

    for row in matrix["rows"]:
        artifacts = "<br>".join(
            f"[`{_markdown_escape(item['path'])}`](../{item['path']})"
            for item in row["evidence_artifacts"]
        )
        command = (
            "—" if row["test_command"] is None else _markdown_code(row["test_command"])
        )
        basis = row["evidence_basis"].replace("_", " ")
        evidence = (
            f"**{row['evidence_level']}** ({_markdown_escape(basis)}) — "
            f"{_markdown_escape(row['evidence_scope'])}"
        )
        dependency = _markdown_escape(row["thesis_dependency"].replace("_", " "))
        claims = row["claim_ids"]
        if claims:
            dependency += f"<br>claims: {', '.join(claims)}"
        lines.append(
            "| "
            + " | ".join(
                [
                    _markdown_escape(row["feature"]),
                    _markdown_code(row["status"]),
                    _markdown_code(row["exact_revision"]),
                    command,
                    artifacts,
                    _markdown_escape(row["known_limitations"]),
                    evidence,
                    dependency,
                ]
            )
            + " |"
        )
    lines.append("")
    return "\n".join(lines)


def render_json(matrix: dict[str, Any]) -> str:
    return json.dumps(matrix, indent=2, ensure_ascii=False) + "\n"


def _validate_output_target(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.parent.is_symlink() or not path.parent.is_dir():
        raise CatalogError(f"output parent must be a real directory: {path.parent}")
    if path.exists() or path.is_symlink():
        try:
            mode = path.lstat().st_mode
        except OSError as error:
            raise CatalogError(
                f"cannot inspect generated output {path}: {error}"
            ) from error
        if not stat.S_ISREG(mode):
            raise CatalogError(
                f"generated output must be a regular, non-symlink file: {path}"
            )


def _atomic_write(path: Path, content: str) -> None:
    payload = content.encode("utf-8")
    if len(payload) > MAX_GENERATED_OUTPUT_BYTES:
        raise CatalogError(
            "generated output exceeds the "
            f"{MAX_GENERATED_OUTPUT_BYTES}-byte limit: {path}"
        )
    _validate_output_target(path)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{path.name}.", dir=path.parent
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
        os.chmod(temporary, 0o644)
        os.replace(temporary, path)
        directory_fd = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def _check_output(path: Path, expected: str) -> str | None:
    if not path.exists() and not path.is_symlink():
        return f"{path.relative_to(REPO_ROOT)} is missing"
    try:
        actual = _read_regular_text(
            path,
            max_bytes=MAX_GENERATED_OUTPUT_BYTES,
            context="generated capability output",
        )
    except CatalogError as error:
        return str(error)
    if actual != expected:
        return f"{path.relative_to(REPO_ROOT)} is out of date"
    return None


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument(
        "--write", action="store_true", help="write both generated outputs"
    )
    group.add_argument(
        "--check", action="store_true", help="fail if either output has drifted"
    )
    args = parser.parse_args(argv)

    try:
        matrix = resolve_catalog()
        expected_json = render_json(matrix)
        expected_markdown = render_markdown(matrix)
    except CatalogError as error:
        print(f"capability catalog invalid: {error}", file=sys.stderr)
        return 1

    if args.write:
        try:
            # Preflight both targets before replacing either one. Each replacement is
            # independently atomic and durable; a subsequent --check detects a crash
            # between the two publications.
            _validate_output_target(JSON_OUTPUT)
            _validate_output_target(MARKDOWN_OUTPUT)
            _atomic_write(JSON_OUTPUT, expected_json)
            _atomic_write(MARKDOWN_OUTPUT, expected_markdown)
        except (CatalogError, OSError) as error:
            print(f"cannot write capability matrix: {error}", file=sys.stderr)
            return 1
        print(
            "wrote "
            f"{JSON_OUTPUT.relative_to(REPO_ROOT)} and "
            f"{MARKDOWN_OUTPUT.relative_to(REPO_ROOT)}"
        )
        return 0

    problems = [
        problem
        for problem in (
            _check_output(JSON_OUTPUT, expected_json),
            _check_output(MARKDOWN_OUTPUT, expected_markdown),
        )
        if problem is not None
    ]
    if problems:
        for problem in problems:
            print(f"{problem}; run with --write", file=sys.stderr)
        return 1
    print("capability matrix is current and content-bound")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
