#!/usr/bin/env python3
"""Materialize one independent Cargo output for portable bootstrap checks only."""

from __future__ import annotations

import json
import os
import secrets
import stat
import sys
from pathlib import Path
from typing import Any

from observed_build import canonical, git_blob_id, git_output, git_text, sha256
from source_provenance import snapshot_regular_file

ROOT = Path(__file__).resolve().parents[4]
CRATE = ROOT / "crates/engram-managed-observer"
RELEASE_BINARY = CRATE / "target/release/prisoma-engram-managed-observer"
MAX_BINARY_BYTES = 64 * 1024 * 1024
MAX_SOURCE_BYTES = 1024 * 1024


def identity(value: os.stat_result) -> tuple[int, ...]:
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


def directory_identity(value: os.stat_result) -> tuple[int, ...]:
    # APFS changes a directory's link count when this helper creates its file.
    return value.st_dev, value.st_ino, value.st_mode, value.st_uid


def source_observation() -> dict[str, Any]:
    """Record current source and index separately; this does not attest a build."""
    integration = ROOT / "integrations/engram/managed-observer"
    paths = {
        CRATE / "Cargo.toml",
        CRATE / "Cargo.lock",
        Path(__file__),
        integration / "scripts/observed_build.py",
        integration / "scripts/source_provenance.py",
        *CRATE.joinpath("src").rglob("*.rs"),
        *integration.joinpath("contracts").glob("*.json"),
    }
    if not 10 <= len(paths) <= 128:
        raise ValueError("bootstrap source observation exceeds its roster bound")
    rows = []
    for path in sorted(paths):
        if path.resolve(strict=True) != path:
            raise ValueError("bootstrap source path traverses a link")
        relative = path.relative_to(ROOT).as_posix()
        payload = snapshot_regular_file(path, MAX_SOURCE_BYTES)
        index = git_output(ROOT, "ls-files", "--stage", "--", relative).decode()
        blob = git_blob_id(payload, ROOT)
        rows.append(
            {
                "path": relative,
                "bytes": len(payload),
                "sha256": sha256(payload),
                "current_blob": blob,
                "index_entry": index,
                "index_matches_current": index
                in (f"100644 {blob} 0\t{relative}\n", f"100755 {blob} 0\t{relative}\n"),
            }
        )
    return {"head": git_text(ROOT, "rev-parse", "HEAD"), "rows": rows}


def read_cargo_output(descriptor: int) -> tuple[bytes, tuple[int, ...]]:
    """Snapshot linked producer output; downstream unique-file rules are unchanged."""
    before = os.fstat(descriptor)
    if (
        not stat.S_ISREG(before.st_mode)
        or before.st_uid != os.geteuid()
        or before.st_nlink < 1
        or stat.S_IMODE(before.st_mode) & 0o022
        or not stat.S_IMODE(before.st_mode) & 0o100
        or not 1 <= before.st_size <= MAX_BINARY_BYTES
    ):
        raise ValueError("Cargo output is not a bounded owned executable")
    remaining = before.st_size
    chunks = []
    while remaining:
        chunk = os.read(descriptor, min(1024 * 1024, remaining))
        if not chunk:
            raise ValueError("Cargo output shrank during capture")
        chunks.append(chunk)
        remaining -= len(chunk)
    if os.read(descriptor, 1) or identity(os.fstat(descriptor)) != identity(before):
        raise ValueError("Cargo output changed during capture")
    return b"".join(chunks), identity(before)


def prepare_binary(path: Path) -> dict[str, Any]:
    """Replace only the fixed Cargo output; never infer observed-build authority."""
    candidate = path.absolute()
    if candidate != RELEASE_BINARY or candidate.resolve(strict=True) != candidate:
        raise ValueError("bootstrap binary must equal the canonical fixed Cargo output")
    if ROOT.resolve(strict=True) != ROOT:
        raise ValueError("bootstrap repository path traverses a link")
    # Hold every directory from the filesystem root. Opening the absolute parent
    # with O_NOFOLLOW alone would still follow a replaced intermediate component.
    context = source_observation()
    descriptors: list[int] = []
    bindings: list[tuple[int, str, int, tuple[int, ...]]] = []
    directory = -1
    created = False
    temporary = f".{candidate.name}.bootstrap-{secrets.token_hex(16)}"

    def rejoin_directories() -> None:
        for parent_fd, name, child_fd, admitted in bindings:
            if (
                directory_identity(os.fstat(child_fd)) != admitted
                or directory_identity(
                    os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
                )
                != admitted
            ):
                raise ValueError("bootstrap directory binding changed")

    try:
        directory = os.open("/", os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        descriptors.append(directory)
        parent = Path("/")
        for component in candidate.parent.parts[1:]:
            parent = parent / component
            observed = os.stat(component, dir_fd=directory, follow_symlinks=False)
            if not stat.S_ISDIR(observed.st_mode) or (
                (parent == ROOT or ROOT in parent.parents)
                and (
                    observed.st_uid != os.geteuid()
                    or stat.S_IMODE(observed.st_mode) & 0o022
                )
            ):
                raise ValueError("bootstrap parent is not an admitted directory")
            child = os.open(
                component,
                os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                dir_fd=directory,
            )
            descriptors.append(child)
            admitted = directory_identity(observed)
            if directory_identity(os.fstat(child)) != admitted:
                raise ValueError("bootstrap directory changed before open")
            bindings.append((directory, component, child, admitted))
            directory = child
        rejoin_directories()
        named = os.stat(candidate.name, dir_fd=directory, follow_symlinks=False)
        if not stat.S_ISREG(named.st_mode):
            raise ValueError("Cargo output is not a regular file")
        source = os.open(
            candidate.name,
            os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK,
            dir_fd=directory,
        )
        descriptors.append(source)
        payload, original = read_cargo_output(source)
        if identity(named) != original:
            raise ValueError("Cargo output changed before capture")
        output = os.open(
            temporary,
            os.O_RDWR | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
            0o600,
            dir_fd=directory,
        )
        descriptors.append(output)
        created = True
        remaining = memoryview(payload)
        while remaining:
            written = os.write(output, remaining)
            if written <= 0:
                raise OSError("bootstrap copy made no write progress")
            remaining = remaining[written:]
        os.fchmod(output, 0o700)
        os.fsync(output)
        os.lseek(output, 0, os.SEEK_SET)
        copied, sealed = read_cargo_output(output)
        if copied != payload or sealed[3] != 1 or sealed[:2] == original[:2]:
            raise ValueError("bootstrap copy is not an independent exact inode")
        if (
            identity(os.fstat(source)) != original
            or identity(
                os.stat(candidate.name, dir_fd=directory, follow_symlinks=False)
            )
            != original
            or source_observation() != context
        ):
            raise ValueError(
                "bootstrap input or source observation changed before publication"
            )
        rejoin_directories()
        os.replace(
            temporary, candidate.name, src_dir_fd=directory, dst_dir_fd=directory
        )
        os.fsync(directory)
        # Removing the original name legitimately changes the old inode's nlink/ctime.
        # Rejoin the new named inode instead of treating that expected unlink as drift.
        rejoin_directories()
        final = os.stat(candidate.name, dir_fd=directory, follow_symlinks=False)
        if (
            identity(final) != identity(os.fstat(output))
            or final.st_nlink != 1
            or stat.S_IMODE(final.st_mode) != 0o700
            or snapshot_regular_file(candidate, MAX_BINARY_BYTES) != payload
            or source_observation() != context
        ):
            raise ValueError(
                "published bootstrap artifact or source observation differs"
            )
        rejoin_directories()
        return {
            "scope": "portable-bootstrap-materialization-only",
            "path": str(candidate),
            "bytes": len(payload),
            "sha256": sha256(payload),
            "source_identity_before": original,
            "staged_identity": identity(final),
            "source_observation": context,
            "source_observation_sha256": sha256(canonical(context)),
            "compiled_source_attested": False,
            "observed_build_authority": False,
            "package_authority": False,
        }
    finally:
        # Publication may already have happened. Cleanup failure is an error,
        # never a successful receipt or proof that no replacement occurred.
        primary = sys.exception()
        cleanup_error: OSError | None = None
        if created:
            try:
                os.unlink(temporary, dir_fd=directory)
            except FileNotFoundError:
                pass
            except OSError as error:
                cleanup_error = error
        for descriptor in reversed(descriptors):
            try:
                os.close(descriptor)
            except OSError as error:
                # A failed close can already have released its descriptor.
                # Attempt every other cleanup once; never retry this number.
                if cleanup_error is None:
                    cleanup_error = error
        if cleanup_error is not None:
            if primary is not None:
                primary.add_note(f"bootstrap cleanup also failed: {cleanup_error}")
            else:
                raise cleanup_error


def main() -> int:
    if len(os.sys.argv) != 1:
        raise ValueError("bootstrap preparation accepts no caller-selected paths")
    print(json.dumps(prepare_binary(RELEASE_BINARY), sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as error:
        print(f"ERROR: {error}", file=os.sys.stderr)
        raise SystemExit(1) from error
