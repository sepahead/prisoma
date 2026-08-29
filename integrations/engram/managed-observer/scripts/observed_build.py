"""Exact local-build observations for the Prisoma Engram managed observer."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import stat
import struct
import subprocess
from pathlib import Path, PurePosixPath
from typing import Any

from source_provenance import (
    capture_repository_identity,
    snapshot_regular_file,
    valid_git_object,
)


ROOT = Path(__file__).resolve().parents[4]
CRATE = ROOT / "crates" / "engram-managed-observer"
MANIFEST = CRATE / "Cargo.toml"
LOCK = CRATE / "Cargo.lock"
RELEASE_BINARY = CRATE / "target" / "release" / "prisoma-engram-managed-observer"
BUILD_RECEIPT_SCHEMA = (
    ROOT
    / "integrations"
    / "engram"
    / "managed-observer"
    / "evidence"
    / "observer-release-build-receipt.schema.json"
)
SCHEMA_VERSION = "prisoma.observer.release-build-receipt.v1"
MAX_SOURCE_BYTES = 16 * 1024 * 1024
MAX_SOURCE_FILES = 128
MAX_BINARY_BYTES = 64 * 1024 * 1024
MAX_TOOL_BYTES = 64 * 1024 * 1024
MAX_GIT_OUTPUT_BYTES = 8 * 1024 * 1024
MAX_VERSION_BYTES = 64 * 1024
MAX_RECEIPT_BYTES = 4 * 1024 * 1024
MACHO_MAGIC_64_LE = b"\xcf\xfa\xed\xfe"
MACHO_CPU_TYPE_ARM64 = 0x0100000C
MACHO_FILE_TYPE_EXECUTE = 2
MACHO_BUILD_VERSION_COMMAND = 0x32
MACHO_PLATFORM_MACOS = 1
HEX = frozenset("0123456789abcdef")
REPOSITORY_FIELDS = {
    "repository",
    "commit",
    "tree",
    "origin_main",
    "object_format",
    "clean",
}
SOURCE_ROW_FIELDS = {"path", "size_bytes", "sha256", "git_mode", "git_blob"}
SOURCE_FIELDS = {
    "manifest_path",
    "lock_path",
    "source_roster",
    "source_roster_sha256",
    "cargo_configuration_files",
}
TOOL_FIELDS = {
    "path",
    "resolved_path",
    "size_bytes",
    "sha256",
    "version_verbose",
    "version_verbose_sha256",
}
TOOLCHAIN_FIELDS = {
    "cargo",
    "rustc",
    "rustc_host",
    "rustc_release",
    "rustc_commit_hash",
    "rustc_commit_date",
    "llvm_version",
}
STREAM_FIELDS = {"size_bytes", "sha256"}
BUILD_FIELDS = {
    "argv",
    "profile",
    "locked",
    "offline",
    "incremental",
    "target_directory_isolated",
    "environment",
    "exit_code",
    "stdout",
    "stderr",
}
BUILD_ENVIRONMENT = {
    "CARGO_INCREMENTAL": "0",
    "LC_ALL": "C",
    "RUST_BACKTRACE": "0",
    "SOURCE_DATE_EPOCH": "0",
}
MACHO_FIELDS = {
    "magic",
    "bits",
    "endianness",
    "cpu_type",
    "cpu_subtype",
    "file_type",
    "load_command_count",
    "load_command_bytes",
    "flags",
    "reserved",
    "build_platform",
    "minimum_os",
    "sdk",
}
ARTIFACT_FIELDS = {
    "path",
    "size_bytes",
    "sha256",
    "mode",
    "owner_private",
    "executable",
    "link_count",
    "macho",
}
AUTHORITY = {
    "observed_local_build_only": True,
    "reproducible_build_attested": False,
    "publisher_authenticated": False,
    "loaded_bytes_attested": False,
    "external_dependency_closure_attested": False,
    "production_manager_execution": False,
    "ncp_authority": False,
    "music_authority": False,
    "physical_authority": False,
    "scientific_authority": False,
}
RECEIPT_FIELDS = {
    "schema_version",
    "observation_scope",
    "repository",
    "source",
    "toolchain",
    "build",
    "artifact",
    "authority",
    "disclosure",
    "receipt_sha256",
}


class BuildObservationError(ValueError):
    """A fail-closed observed-build validation error."""


def canonical(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        allow_nan=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def sha256(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def digest_without(value: dict[str, Any], field: str) -> str:
    return sha256(canonical({key: item for key, item in value.items() if key != field}))


def valid_sha256(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in HEX for character in value)
    )


def valid_git_oid(value: Any, object_format: str | None = None) -> bool:
    return valid_git_object(value, object_format)


def require_keys(value: Any, fields: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != fields:
        observed = sorted(value) if isinstance(value, dict) else type(value).__name__
        raise BuildObservationError(f"{label} field roster differs: {observed}")
    return value


def safe_relative(value: Any, label: str) -> PurePosixPath:
    if not isinstance(value, str) or not value or "\\" in value or "\0" in value:
        raise BuildObservationError(f"{label} is not a canonical relative path")
    relative = PurePosixPath(value)
    if (
        relative.is_absolute()
        or relative.as_posix() != value
        or any(part in {"", ".", ".."} for part in relative.parts)
    ):
        raise BuildObservationError(f"{label} is not a canonical relative path")
    return relative


def strict_json(payload: bytes, label: str) -> dict[str, Any]:
    def closed_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise BuildObservationError(f"{label} repeats JSON member {key}")
            result[key] = value
        return result

    def reject_constant(value: str) -> None:
        raise BuildObservationError(f"{label} contains non-finite value {value}")

    try:
        document = json.loads(
            payload,
            object_pairs_hook=closed_object,
            parse_constant=reject_constant,
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise BuildObservationError(f"{label} is not strict JSON: {error}") from error
    if not isinstance(document, dict):
        raise BuildObservationError(f"{label} root is not an object")
    return document


def git_output(root: Path, *arguments: str) -> bytes:
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
            *arguments,
        ],
        cwd=root,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=30,
        check=False,
        close_fds=True,
    )
    if (
        completed.returncode != 0
        or len(completed.stdout) > MAX_GIT_OUTPUT_BYTES
        or len(completed.stderr) > MAX_GIT_OUTPUT_BYTES
    ):
        diagnostic = completed.stderr.decode("utf-8", errors="replace")[-1024:]
        raise BuildObservationError(f"Git observation failed: {diagnostic}")
    return completed.stdout


def git_text(root: Path, *arguments: str) -> str:
    payload = git_output(root, *arguments)
    try:
        value = payload.decode("utf-8").strip()
    except UnicodeDecodeError as error:
        raise BuildObservationError("Git output is not UTF-8") from error
    if not value:
        raise BuildObservationError("Git output is empty")
    return value


def repository_identity(root: Path, expected_revision: str) -> dict[str, Any]:
    try:
        return capture_repository_identity(root, expected_revision)
    except (OSError, ValueError) as error:
        raise BuildObservationError(
            "observed build requires an exact clean origin/main checkout"
        ) from error


def cargo_configuration_files(root: Path) -> list[str]:
    search_roots = {root, *root.parents, CRATE, Path.home()}
    candidates = tuple(
        directory / ".cargo" / filename
        for directory in search_roots
        for filename in ("config", "config.toml")
    )
    found = [os.fspath(path) for path in candidates if path.exists()]
    if found:
        raise BuildObservationError(
            "observed build rejects repository or user Cargo configuration"
        )
    return []


def source_identity(root: Path) -> dict[str, Any]:
    object_format = git_text(root, "rev-parse", "--show-object-format")
    prefix = "crates/engram-managed-observer/"
    embedded_release_sources = {
        "integrations/engram/managed-observer/contracts/configuration.schema.json",
        "integrations/engram/managed-observer/contracts/finish-request.schema.json",
        "integrations/engram/managed-observer/contracts/finish-response.schema.json",
        "integrations/engram/managed-observer/contracts/managed-runtime-ipc.schema.json",
        "integrations/engram/managed-observer/contracts/observe-request.schema.json",
        "integrations/engram/managed-observer/contracts/observe-response.schema.json",
        "integrations/engram/managed-observer/contracts/prepare-request.schema.json",
        "integrations/engram/managed-observer/contracts/prepare-response.schema.json",
        "integrations/engram/managed-observer/evidence/observer-release-build-receipt.schema.json",
        "integrations/engram/managed-observer/scripts/build-release-observer.py",
        "integrations/engram/managed-observer/scripts/observed_build.py",
        "integrations/engram/managed-observer/scripts/source_provenance.py",
    }
    tracked = git_output(
        root,
        "ls-files",
        "--stage",
        "-z",
        "--",
        prefix,
        *sorted(embedded_release_sources),
    )
    records: dict[str, tuple[str, str]] = {}
    for raw in tracked.split(b"\0"):
        if not raw:
            continue
        try:
            metadata, path_bytes = raw.split(b"\t", 1)
            mode_bytes, blob_bytes, stage_bytes = metadata.split(b" ", 2)
            path = path_bytes.decode("utf-8")
            mode = mode_bytes.decode("ascii")
            blob = blob_bytes.decode("ascii")
            stage = stage_bytes.decode("ascii")
        except (ValueError, UnicodeDecodeError) as error:
            raise BuildObservationError("Git source roster is malformed") from error
        if stage != "0" or path in records:
            raise BuildObservationError("Git source roster has a staged collision")
        records[path] = (mode, blob)
    crate_paths = {path for path in records if path.startswith(prefix)}
    expected_paths = crate_paths | embedded_release_sources
    if (
        set(records) != expected_paths
        or f"{prefix}Cargo.toml" not in records
        or f"{prefix}Cargo.lock" not in records
        or not any(path.startswith(f"{prefix}src/") for path in expected_paths)
        or len(expected_paths) < 15
        or len(expected_paths) > MAX_SOURCE_FILES
    ):
        raise BuildObservationError("observed build source roster is incomplete")
    rows: list[dict[str, Any]] = []
    for relative in sorted(expected_paths):
        mode, blob = records[relative]
        if mode not in {"100644", "100755"} or not valid_git_oid(
            blob,
            object_format,
        ):
            raise BuildObservationError("observed build source Git identity differs")
        path = root.joinpath(*PurePosixPath(relative).parts)
        payload = snapshot_regular_file(path, MAX_SOURCE_BYTES)
        if not payload:
            raise BuildObservationError("observed build source file is empty")
        expected_blob = git_blob_id(payload, root)
        tree_row = git_output(root, "ls-tree", "-z", "HEAD", "--", relative)
        expected_tree_row = f"{mode} blob {blob}\t{relative}\0".encode()
        if expected_blob != blob or tree_row != expected_tree_row:
            raise BuildObservationError(
                "observed source bytes differ from the Git blob"
            )
        rows.append(
            {
                "path": relative,
                "size_bytes": len(payload),
                "sha256": sha256(payload),
                "git_mode": mode,
                "git_blob": blob,
            }
        )
    return {
        "manifest_path": f"{prefix}Cargo.toml",
        "lock_path": f"{prefix}Cargo.lock",
        "source_roster": rows,
        "source_roster_sha256": sha256(
            b"prisoma-observer-build-source-roster-v1\0" + canonical(rows)
        ),
        "cargo_configuration_files": cargo_configuration_files(root),
    }


def git_blob_id(payload: bytes, root: Path) -> str:
    completed = subprocess.run(
        ["git", "hash-object", "--stdin"],
        cwd=root,
        env={
            **{
                key: value
                for key, value in os.environ.items()
                if not key.startswith("GIT_")
            },
            "GIT_NO_REPLACE_OBJECTS": "1",
            "LC_ALL": "C",
        },
        input=payload,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=30,
        check=False,
        close_fds=True,
    )
    if completed.returncode != 0 or completed.stderr:
        raise BuildObservationError("Git cannot derive an observed source blob")
    try:
        value = completed.stdout.decode("ascii").strip()
    except UnicodeDecodeError as error:
        raise BuildObservationError("Git source blob is not ASCII") from error
    if not valid_git_oid(value):
        raise BuildObservationError("Git source blob identity differs")
    return value


def checked_tool(name: str, version_arguments: list[str]) -> dict[str, Any]:
    discovered = shutil.which(name)
    if discovered is None:
        raise BuildObservationError(f"required tool is absent: {name}")
    path = Path(discovered).absolute()
    lexical = os.lstat(path)
    if (
        not (stat.S_ISREG(lexical.st_mode) or stat.S_ISLNK(lexical.st_mode))
        or lexical.st_uid != os.geteuid()
        or path.parent.resolve(strict=True) != path.parent
    ):
        raise BuildObservationError(f"{name} invocation path is not owner controlled")
    resolved_path = path.resolve(strict=True)
    payload = snapshot_regular_file(resolved_path, MAX_TOOL_BYTES)
    completed = subprocess.run(
        [os.fspath(path), *version_arguments],
        env=sanitized_environment(),
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=30,
        check=False,
        close_fds=True,
    )
    if (
        completed.returncode != 0
        or completed.stderr
        or not completed.stdout
        or len(completed.stdout) > MAX_VERSION_BYTES
    ):
        raise BuildObservationError(f"{name} verbose version observation failed")
    try:
        version = completed.stdout.decode("utf-8").strip()
    except UnicodeDecodeError as error:
        raise BuildObservationError(f"{name} verbose version is not UTF-8") from error
    lexical_after = os.lstat(path)
    if (
        not version
        or completed.stdout != version.encode("utf-8") + b"\n"
        or (
            lexical.st_dev,
            lexical.st_ino,
            lexical.st_mode,
            lexical.st_nlink,
            lexical.st_uid,
            lexical.st_size,
            lexical.st_mtime_ns,
            lexical.st_ctime_ns,
        )
        != (
            lexical_after.st_dev,
            lexical_after.st_ino,
            lexical_after.st_mode,
            lexical_after.st_nlink,
            lexical_after.st_uid,
            lexical_after.st_size,
            lexical_after.st_mtime_ns,
            lexical_after.st_ctime_ns,
        )
        or path.resolve(strict=True) != resolved_path
        or snapshot_regular_file(resolved_path, MAX_TOOL_BYTES) != payload
    ):
        raise BuildObservationError(f"{name} tool identity changed during observation")
    return {
        "path": os.fspath(path),
        "resolved_path": os.fspath(resolved_path),
        "size_bytes": len(payload),
        "sha256": sha256(payload),
        "version_verbose": version,
        "version_verbose_sha256": sha256(completed.stdout),
    }


def sanitized_environment() -> dict[str, str]:
    allowed = {
        "HOME",
        "PATH",
        "TMPDIR",
        "USER",
        "LOGNAME",
        "SHELL",
        "TERM",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
    }
    environment = {key: value for key, value in os.environ.items() if key in allowed}
    environment.update(BUILD_ENVIRONMENT)
    return environment


def toolchain_identity() -> dict[str, Any]:
    cargo = checked_tool("cargo", ["--version", "--verbose"])
    rustc = checked_tool("rustc", ["-vV"])
    fields: dict[str, str] = {}
    for line in rustc["version_verbose"].splitlines():
        if ": " in line:
            key, value = line.split(": ", 1)
            fields[key] = value
    required = {
        "host",
        "release",
        "commit-hash",
        "commit-date",
        "LLVM version",
    }
    if set(fields) < required or not valid_git_oid(fields["commit-hash"]):
        raise BuildObservationError("rustc verbose version identity is incomplete")
    return {
        "cargo": cargo,
        "rustc": rustc,
        "rustc_host": fields["host"],
        "rustc_release": fields["release"],
        "rustc_commit_hash": fields["commit-hash"],
        "rustc_commit_date": fields["commit-date"],
        "llvm_version": fields["LLVM version"],
    }


def packed_version(value: int) -> str:
    major = (value >> 16) & 0xFFFF
    minor = (value >> 8) & 0xFF
    patch = value & 0xFF
    return f"{major}.{minor}.{patch}"


def parse_arm64_macho(payload: bytes) -> dict[str, Any]:
    if len(payload) < 32 or payload[:4] != MACHO_MAGIC_64_LE:
        raise BuildObservationError("observer release binary is not thin Mach-O 64")
    (
        magic,
        cpu_type,
        cpu_subtype,
        file_type,
        command_count,
        command_bytes,
        flags,
        reserved,
    ) = struct.unpack_from("<8I", payload, 0)
    if (
        magic != 0xFEEDFACF
        or cpu_type != MACHO_CPU_TYPE_ARM64
        or cpu_subtype & 0xFF000000
        or file_type != MACHO_FILE_TYPE_EXECUTE
        or not 1 <= command_count <= 4096
        or not 8 <= command_bytes <= min(MAX_SOURCE_BYTES, len(payload) - 32)
        or reserved != 0
    ):
        raise BuildObservationError("observer Mach-O arm64 header differs")
    cursor = 32
    end = cursor + command_bytes
    build_versions: list[tuple[int, int, int]] = []
    for _index in range(command_count):
        if cursor + 8 > end:
            raise BuildObservationError("observer Mach-O load command is truncated")
        command, size = struct.unpack_from("<2I", payload, cursor)
        if size < 8 or size % 8 or cursor + size > end:
            raise BuildObservationError("observer Mach-O load command size differs")
        if command == MACHO_BUILD_VERSION_COMMAND:
            if size < 24:
                raise BuildObservationError(
                    "observer Mach-O build version is truncated"
                )
            platform, minimum_os, sdk = struct.unpack_from("<3I", payload, cursor + 8)
            build_versions.append((platform, minimum_os, sdk))
        cursor += size
    if cursor != end or len(build_versions) != 1:
        raise BuildObservationError("observer Mach-O command roster differs")
    platform, minimum_os, sdk = build_versions[0]
    if (
        platform != MACHO_PLATFORM_MACOS
        or minimum_os == 0
        or sdk == 0
        or sdk < minimum_os
    ):
        raise BuildObservationError("observer Mach-O target platform is not macOS")
    return {
        "magic": "feedfacf",
        "bits": 64,
        "endianness": "little",
        "cpu_type": cpu_type,
        "cpu_subtype": cpu_subtype,
        "file_type": file_type,
        "load_command_count": command_count,
        "load_command_bytes": command_bytes,
        "flags": flags,
        "reserved": reserved,
        "build_platform": "macos",
        "minimum_os": packed_version(minimum_os),
        "sdk": packed_version(sdk),
    }


def artifact_identity(path: Path) -> dict[str, Any]:
    expected = RELEASE_BINARY.absolute()
    candidate = path.absolute()
    if candidate != expected:
        raise BuildObservationError(f"observer artifact path must equal {expected}")
    observed = os.lstat(candidate)
    if (
        not stat.S_ISREG(observed.st_mode)
        or observed.st_nlink != 1
        or observed.st_uid != os.geteuid()
        or candidate.resolve(strict=True) != candidate
        or stat.S_IMODE(observed.st_mode) != 0o700
    ):
        raise BuildObservationError("observer artifact is not one owner-private file")
    payload = snapshot_regular_file(candidate, MAX_BINARY_BYTES)
    return {
        "path": candidate.relative_to(ROOT).as_posix(),
        "size_bytes": len(payload),
        "sha256": sha256(payload),
        "mode": "0700",
        "owner_private": True,
        "executable": True,
        "link_count": 1,
        "macho": parse_arm64_macho(payload),
    }


def stream_identity(payload: bytes) -> dict[str, Any]:
    return {"size_bytes": len(payload), "sha256": sha256(payload)}


def validate_receipt_document(
    document: dict[str, Any],
    *,
    repository: dict[str, Any],
    source: dict[str, Any],
    toolchain: dict[str, Any],
    artifact: dict[str, Any],
) -> None:
    require_keys(document, RECEIPT_FIELDS, "observed-build receipt")
    require_keys(document["repository"], REPOSITORY_FIELDS, "observed repository")
    source_document = require_keys(document["source"], SOURCE_FIELDS, "observed source")
    toolchain_document = require_keys(
        document["toolchain"], TOOLCHAIN_FIELDS, "observed toolchain"
    )
    build = require_keys(document["build"], BUILD_FIELDS, "observed build")
    artifact_document = require_keys(
        document["artifact"], ARTIFACT_FIELDS, "observed artifact"
    )
    for row in source_document["source_roster"]:
        require_keys(row, SOURCE_ROW_FIELDS, "observed source row")
    for name in ("cargo", "rustc"):
        require_keys(toolchain_document[name], TOOL_FIELDS, f"observed {name}")
    require_keys(build["stdout"], STREAM_FIELDS, "observed build stdout")
    require_keys(build["stderr"], STREAM_FIELDS, "observed build stderr")
    require_keys(artifact_document["macho"], MACHO_FIELDS, "observed Mach-O")
    repository_document = document["repository"]
    if (
        repository_document != repository
        or not isinstance(repository_document["repository"], str)
        or not repository_document["repository"]
        or repository_document["object_format"] not in {"sha1", "sha256"}
        or not valid_git_oid(
            repository_document["commit"],
            repository_document["object_format"],
        )
        or not valid_git_oid(
            repository_document["tree"],
            repository_document["object_format"],
        )
        or not valid_git_oid(
            repository_document["origin_main"],
            repository_document["object_format"],
        )
        or repository_document["origin_main"] != repository_document["commit"]
        or repository_document["clean"] is not True
    ):
        raise BuildObservationError("observed repository semantics differ")
    source_rows = source_document["source_roster"]
    source_paths = [row["path"] for row in source_rows]
    if (
        source_paths != sorted(set(source_paths))
        or source_document["manifest_path"] not in source_paths
        or source_document["lock_path"] not in source_paths
        or source_document["cargo_configuration_files"] != []
        or any(
            safe_relative(row["path"], "observed source path").as_posix() != row["path"]
            or not isinstance(row["size_bytes"], int)
            or not 1 <= row["size_bytes"] <= MAX_SOURCE_BYTES
            or not valid_sha256(row["sha256"])
            or row["git_mode"] not in {"100644", "100755"}
            or not valid_git_oid(
                row["git_blob"],
                repository_document["object_format"],
            )
            for row in source_rows
        )
    ):
        raise BuildObservationError("observed source roster semantics differ")
    for name in ("cargo", "rustc"):
        tool = toolchain_document[name]
        if (
            not all(
                isinstance(tool[field], str)
                and Path(tool[field]).is_absolute()
                and os.fspath(Path(tool[field])) == tool[field]
                for field in ("path", "resolved_path")
            )
            or not isinstance(tool["size_bytes"], int)
            or not 1 <= tool["size_bytes"] <= MAX_TOOL_BYTES
            or not valid_sha256(tool["sha256"])
            or not isinstance(tool["version_verbose"], str)
            or not tool["version_verbose"]
            or tool["version_verbose_sha256"]
            != sha256(tool["version_verbose"].encode("utf-8") + b"\n")
        ):
            raise BuildObservationError(f"observed {name} semantics differ")
    rustc_fields: dict[str, str] = {}
    for line in toolchain_document["rustc"]["version_verbose"].splitlines():
        if ": " in line:
            key, value = line.split(": ", 1)
            rustc_fields[key] = value
    commit_date = toolchain_document["rustc_commit_date"]
    llvm_version = toolchain_document["llvm_version"]
    if (
        toolchain_document["rustc_host"] != "aarch64-apple-darwin"
        or not isinstance(toolchain_document["rustc_release"], str)
        or not toolchain_document["rustc_release"]
        or not valid_git_oid(toolchain_document["rustc_commit_hash"])
        or not isinstance(commit_date, str)
        or len(commit_date) != 10
        or commit_date[4:5] != "-"
        or commit_date[7:8] != "-"
        or not commit_date.replace("-", "").isdigit()
        or not isinstance(llvm_version, str)
        or not 2 <= len(llvm_version.split(".")) <= 3
        or any(not component.isdigit() for component in llvm_version.split("."))
        or rustc_fields.get("host") != toolchain_document["rustc_host"]
        or rustc_fields.get("release") != toolchain_document["rustc_release"]
        or rustc_fields.get("commit-hash") != toolchain_document["rustc_commit_hash"]
        or rustc_fields.get("commit-date") != commit_date
        or rustc_fields.get("LLVM version") != llvm_version
        or not toolchain_document["cargo"]["version_verbose"].startswith("cargo ")
    ):
        raise BuildObservationError("observed toolchain lineage differs")
    argv = build["argv"]
    if not isinstance(argv, list) or len(argv) != 11:
        raise BuildObservationError("observed build argv differs")
    target_directory = Path(argv[10])
    expected_prefix = (CRATE / "target").absolute()
    target_suffix = target_directory.name.removeprefix(".observed-build-")
    if (
        argv[:10]
        != [
            toolchain["cargo"]["path"],
            "build",
            "--locked",
            "--offline",
            "--release",
            "--manifest-path",
            os.fspath(MANIFEST.absolute()),
            "--bin",
            "prisoma-engram-managed-observer",
            "--target-dir",
        ]
        or target_directory.parent != expected_prefix
        or not target_directory.name.startswith(".observed-build-")
        or not target_suffix
        or len(target_suffix) > 128
        or any(
            character not in "abcdefghijklmnopqrstuvwxyz0123456789_"
            for character in target_suffix
        )
    ):
        raise BuildObservationError("observed build command path differs")
    if (
        document["schema_version"] != SCHEMA_VERSION
        or document["observation_scope"]
        != "one-local-clean-source-build-observation-v1"
        or document["repository"] != repository
        or source_document != source
        or toolchain_document != toolchain
        or source_document["source_roster_sha256"]
        != sha256(
            b"prisoma-observer-build-source-roster-v1\0"
            + canonical(source_document["source_roster"])
        )
        or build["profile"] != "release"
        or build["locked"] is not True
        or build["offline"] is not True
        or build["incremental"] is not False
        or build["target_directory_isolated"] is not True
        or build["environment"] != BUILD_ENVIRONMENT
        or build["exit_code"] != 0
        or any(
            not isinstance(build[stream]["size_bytes"], int)
            or not 0 <= build[stream]["size_bytes"] <= 16 * 1024 * 1024
            or not valid_sha256(build[stream]["sha256"])
            for stream in ("stdout", "stderr")
        )
        or artifact_document != artifact
        or artifact_document["path"]
        != "crates/engram-managed-observer/target/release/"
        "prisoma-engram-managed-observer"
        or artifact_document["mode"] != "0700"
        or artifact_document["owner_private"] is not True
        or artifact_document["executable"] is not True
        or artifact_document["link_count"] != 1
        or not isinstance(artifact_document["size_bytes"], int)
        or not 1 <= artifact_document["size_bytes"] <= MAX_BINARY_BYTES
        or not valid_sha256(artifact_document["sha256"])
        or document["authority"] != AUTHORITY
        or not isinstance(document["disclosure"], str)
        or not document["disclosure"]
        or document["disclosure"] != document["disclosure"].strip()
        or len(document["disclosure"].encode("utf-8")) > 4096
        or not valid_sha256(document["receipt_sha256"])
        or document["receipt_sha256"] != digest_without(document, "receipt_sha256")
    ):
        raise BuildObservationError("observed build receipt lineage differs")


def verify_observed_build_receipt(
    receipt_path: Path,
    binary_path: Path,
    expected_revision: str,
) -> dict[str, Any]:
    expected_receipt_path = RELEASE_BINARY.with_name(
        f"{RELEASE_BINARY.name}.observed-build.json"
    ).absolute()
    candidate_receipt_path = receipt_path.absolute()
    receipt_stat = os.lstat(candidate_receipt_path)
    if (
        candidate_receipt_path != expected_receipt_path
        or not stat.S_ISREG(receipt_stat.st_mode)
        or receipt_stat.st_uid != os.geteuid()
        or receipt_stat.st_nlink != 1
        or stat.S_IMODE(receipt_stat.st_mode) != 0o600
        or candidate_receipt_path.resolve(strict=True) != candidate_receipt_path
    ):
        raise BuildObservationError("observed-build receipt path or mode differs")
    payload = snapshot_regular_file(candidate_receipt_path, MAX_RECEIPT_BYTES)
    document = strict_json(payload, "observed-build receipt")
    repository = repository_identity(ROOT, expected_revision)
    source = source_identity(ROOT)
    toolchain = toolchain_identity()
    artifact = artifact_identity(binary_path)
    validate_receipt_document(
        document,
        repository=repository,
        source=source,
        toolchain=toolchain,
        artifact=artifact,
    )
    if payload != canonical(document) + b"\n":
        raise BuildObservationError("observed-build receipt bytes are not canonical")
    return {
        "document": document,
        "payload": payload,
        "exact_sha256": sha256(payload),
        "repository": repository,
        "source": source,
        "toolchain": toolchain,
        "artifact": artifact,
    }
