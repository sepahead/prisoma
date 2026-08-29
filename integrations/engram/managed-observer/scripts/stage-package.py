#!/usr/bin/env python3
"""Stage one target-native Prisoma managed-observer authoring package."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import stat
from pathlib import Path, PurePosixPath
from typing import Any

from observed_build import BuildObservationError, verify_observed_build_receipt
from observed_build import canonical, digest_without
from source_provenance import (
    capture_repository_files,
    snapshot_regular_file,
    valid_git_object,
)


ROOT = Path(__file__).resolve().parents[4]
INTEGRATION = ROOT / "integrations" / "engram" / "managed-observer"
DEFAULT_BINARY = (
    ROOT
    / "crates"
    / "engram-managed-observer"
    / "target"
    / "release"
    / "prisoma-engram-managed-observer"
)
DEFAULT_OUTPUT = INTEGRATION / "package"
STAGE_RECEIPT_SCHEMA = (
    INTEGRATION / "evidence" / "observer-package-stage-receipt.schema.json"
)
STAGE_RECEIPT_VERSION = "prisoma.observer.package-stage-receipt.v1"
MAX_RECIPE_BYTES = 64 * 1024
MAX_EXECUTABLE_BYTES = 64 * 1024 * 1024
MAX_SCHEMA_BYTES = 1024 * 1024
EXPECTED_RECIPE_KEYS = {
    "schema_version",
    "manifest_template",
    "package_root",
    "configuration_path",
    "target",
    "executable",
    "schemas",
    "output_directory",
}
EXPECTED_TARGET = {
    "target_id": "macos-aarch64-darwin",
    "operating_system": "macos",
    "architecture": "aarch64",
    "abi": "darwin",
}
EXPECTED_SCHEMAS = {
    "engram.managed-runtime-ipc.v1": "contracts/managed-runtime-ipc.schema.json",
    "prisoma.observer.configuration.v1": "contracts/configuration.schema.json",
    "prisoma.observer.finish-request.v1": "contracts/finish-request.schema.json",
    "prisoma.observer.finish-response.v1": "contracts/finish-response.schema.json",
    "prisoma.observer.observe-request.v1": "contracts/observe-request.schema.json",
    "prisoma.observer.observe-response.v1": "contracts/observe-response.schema.json",
    "prisoma.observer.prepare-request.v1": "contracts/prepare-request.schema.json",
    "prisoma.observer.prepare-response.v1": "contracts/prepare-response.schema.json",
}
STAGE_AUTHORITY = {
    "observed_stage_only": True,
    "installation": False,
    "execution": False,
    "agent_bridge_command": False,
    "ncp": False,
    "music": False,
    "physical": False,
    "plant": False,
    "scientific": False,
}
STAGE_SOURCE_PATHS = tuple(
    sorted(
        {
            Path(
                "integrations/engram/managed-observer/"
                "authoring.macos-aarch64-darwin.json"
            ),
            Path("integrations/engram/managed-observer/configuration.json"),
            Path("integrations/engram/managed-observer/manifest.template.json"),
            Path(
                "integrations/engram/managed-observer/evidence/"
                "observer-package-stage-receipt.schema.json"
            ),
            Path("integrations/engram/managed-observer/scripts/source_provenance.py"),
            Path("integrations/engram/managed-observer/scripts/observed_build.py"),
            Path("integrations/engram/managed-observer/scripts/stage-package.py"),
            *(
                Path("integrations/engram/managed-observer") / relative
                for relative in EXPECTED_SCHEMAS.values()
            ),
        },
        key=lambda path: path.as_posix(),
    )
)
STAGE_RECEIPT_FIELDS = {
    "schema_version",
    "observation_scope",
    "repository",
    "observed_build_receipt_exact_sha256",
    "observed_build_receipt_sha256",
    "stage_source_roster",
    "stage_source_roster_sha256",
    "stage_input_identity_sha256",
    "source_executable",
    "staged_executable",
    "package_inventory",
    "package_inventory_sha256",
    "authority",
    "disclosure",
    "receipt_sha256",
}


def reject(reason: str) -> None:
    raise SystemExit(reason)


def safe_relative(value: Any) -> PurePosixPath:
    if not isinstance(value, str) or not value or "\\" in value or "\0" in value:
        reject("package path is not a nonempty POSIX string")
    relative = PurePosixPath(value)
    if relative.is_absolute() or any(
        part in {"", ".", ".."} for part in relative.parts
    ):
        reject(f"package path is unsafe: {value}")
    if str(relative) != value:
        reject(f"package path is not canonical: {value}")
    return relative


def open_regular(source: Path, max_bytes: int) -> tuple[int, os.stat_result]:
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        named = os.stat(source, follow_symlinks=False)
    except OSError as error:
        reject(f"source cannot be inspected without following links: {source}: {error}")
    try:
        descriptor = os.open(source, flags)
    except OSError as error:
        reject(f"source cannot be opened without following links: {source}: {error}")
    observed = os.fstat(descriptor)
    if (
        not stat.S_ISREG(observed.st_mode)
        or observed.st_uid != os.geteuid()
        or observed.st_size <= 0
        or observed.st_size > max_bytes
        or file_identity(named) != file_identity(observed)
    ):
        os.close(descriptor)
        reject(f"source is not a bounded owner-controlled regular file: {source}")
    return descriptor, observed


def file_identity(value: os.stat_result) -> tuple[int, ...]:
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


def load_closed_json(
    source: Path, max_bytes: int, label: str
) -> tuple[dict[str, Any], str]:
    descriptor, observed = open_regular(source, max_bytes)
    with os.fdopen(descriptor, "rb", closefd=True) as handle:
        payload = handle.read(observed.st_size + 1)
        after = os.fstat(handle.fileno())
    named_after = os.stat(source, follow_symlinks=False)
    if len(payload) != observed.st_size:
        reject(f"{label} changed while it was read: {source}")
    if file_identity(observed) != file_identity(after) or file_identity(
        after
    ) != file_identity(named_after):
        reject(f"{label} identity changed while it was read: {source}")

    def closed_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                reject(f"{label} contains a duplicate member: {key}")
            result[key] = value
        return result

    def reject_constant(value: str) -> None:
        reject(f"{label} contains a non-finite constant: {value}")

    try:
        document = json.loads(
            payload,
            object_pairs_hook=closed_object,
            parse_constant=reject_constant,
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        reject(f"{label} is not strict JSON: {source}: {error}")
    if not isinstance(document, dict):
        reject(f"{label} root must be an object")
    return document, hashlib.sha256(payload).hexdigest()


def load_recipe(source: Path) -> dict[str, Any]:
    document, _sha256 = load_closed_json(source, MAX_RECIPE_BYTES, "recipe")
    return document


def copy_regular(
    source: Path,
    destination: Path,
    mode: int,
    max_bytes: int,
    *,
    expected_sha256: str | None = None,
) -> None:
    source_descriptor, observed = open_regular(source, max_bytes)
    destination.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    destination_flags = (
        os.O_WRONLY
        | os.O_CREAT
        | os.O_EXCL
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_NOFOLLOW", 0)
    )
    try:
        destination_descriptor = os.open(destination, destination_flags, mode)
    except OSError:
        os.close(source_descriptor)
        raise
    copied = 0
    digest = hashlib.sha256()
    try:
        while copied < observed.st_size:
            chunk = os.read(
                source_descriptor, min(1024 * 1024, observed.st_size - copied)
            )
            if not chunk:
                reject(f"source changed while it was copied: {source}")
            digest.update(chunk)
            view = memoryview(chunk)
            while view:
                written = os.write(destination_descriptor, view)
                if written <= 0:
                    reject(f"destination write made no progress: {destination}")
                copied += written
                view = view[written:]
        if os.read(source_descriptor, 1):
            reject(f"source grew while it was copied: {source}")
        after = os.fstat(source_descriptor)
        named_after = os.stat(source, follow_symlinks=False)
        if file_identity(observed) != file_identity(after) or file_identity(
            after
        ) != file_identity(named_after):
            reject(f"source identity changed while it was copied: {source}")
        if expected_sha256 is not None and digest.hexdigest() != expected_sha256:
            reject(f"source bytes changed after validation: {source}")
        os.fchmod(destination_descriptor, mode)
        os.fsync(destination_descriptor)
    finally:
        os.close(source_descriptor)
        os.close(destination_descriptor)
    if copied != observed.st_size:
        reject(f"source copy length changed: {source}")


def digest_regular(path: Path, max_bytes: int) -> str:
    descriptor, observed = open_regular(path, max_bytes)
    digest = hashlib.sha256()
    try:
        remaining = observed.st_size
        while remaining:
            chunk = os.read(descriptor, min(1024 * 1024, remaining))
            if not chunk:
                reject(f"staged file changed while it was read: {path}")
            digest.update(chunk)
            remaining -= len(chunk)
        if os.read(descriptor, 1):
            reject(f"staged file grew while it was read: {path}")
        after = os.fstat(descriptor)
        named_after = os.stat(path, follow_symlinks=False)
        if file_identity(observed) != file_identity(after) or file_identity(
            after
        ) != file_identity(named_after):
            reject(f"staged file identity changed while it was read: {path}")
    finally:
        os.close(descriptor)
    return digest.hexdigest()


def sha256(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def valid_sha256(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


def stage_source_roster(expected_revision: str) -> list[dict[str, Any]]:
    try:
        return capture_repository_files(
            ROOT,
            expected_revision,
            STAGE_SOURCE_PATHS,
            MAX_SCHEMA_BYTES,
        )
    except (OSError, ValueError) as error:
        raise BuildObservationError(
            "package staging source does not reopen from committed Git bytes"
        ) from error


def executable_identity(path: Path) -> dict[str, Any]:
    observed = os.lstat(path)
    if (
        not stat.S_ISREG(observed.st_mode)
        or observed.st_uid != os.geteuid()
        or observed.st_nlink != 1
        or path.resolve(strict=True) != path
        or stat.S_IMODE(observed.st_mode) != 0o700
    ):
        raise BuildObservationError("staged executable identity differs")
    payload = snapshot_regular_file(path, MAX_EXECUTABLE_BYTES)
    return {
        "size_bytes": len(payload),
        "sha256": sha256(payload),
        "mode": "0700",
    }


def package_inventory(
    output: Path,
    executable_path: PurePosixPath,
    schema_paths: list[PurePosixPath],
) -> list[dict[str, Any]]:
    expected_roles = {
        executable_path.as_posix(): "executable",
        **{path.as_posix(): "contract" for path in schema_paths},
    }
    observed_paths: set[str] = set()
    for path in output.rglob("*"):
        observed = os.lstat(path)
        if stat.S_ISDIR(observed.st_mode):
            if path.resolve(strict=True) != path:
                raise BuildObservationError("staged package directory traverses a link")
            continue
        if not stat.S_ISREG(observed.st_mode) or path.resolve(strict=True) != path:
            raise BuildObservationError("staged package contains a non-regular file")
        observed_paths.add(path.relative_to(output).as_posix())
    if observed_paths != set(expected_roles):
        raise BuildObservationError("staged package file roster differs")
    rows: list[dict[str, Any]] = []
    for relative, role in sorted(expected_roles.items()):
        path = output.joinpath(*PurePosixPath(relative).parts)
        observed = os.lstat(path)
        expected_mode = 0o700 if role == "executable" else 0o600
        if (
            observed.st_uid != os.geteuid()
            or observed.st_nlink != 1
            or stat.S_IMODE(observed.st_mode) != expected_mode
        ):
            raise BuildObservationError("staged package file mode or owner differs")
        maximum = MAX_EXECUTABLE_BYTES if role == "executable" else MAX_SCHEMA_BYTES
        payload = snapshot_regular_file(path, maximum)
        rows.append(
            {
                "relative_path": relative,
                "size_bytes": len(payload),
                "sha256": sha256(payload),
                "mode": "0700" if role == "executable" else "0600",
                "role": role,
            }
        )
    return rows


def write_new_receipt(path: Path, payload: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.parent.resolve(strict=True) != path.parent:
        raise BuildObservationError("package-stage receipt parent traverses a link")
    flags = (
        os.O_WRONLY
        | os.O_CREAT
        | os.O_EXCL
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_NOFOLLOW", 0)
    )
    descriptor = os.open(path, flags, 0o600)
    try:
        view = memoryview(payload)
        while view:
            written = os.write(descriptor, view)
            if written <= 0:
                raise OSError("package-stage receipt write made no progress")
            view = view[written:]
        os.fsync(descriptor)
    except BaseException:
        os.close(descriptor)
        descriptor = -1
        path.unlink(missing_ok=True)
        raise
    finally:
        if descriptor >= 0:
            os.close(descriptor)


def validate_stage_receipt(
    document: dict[str, Any],
    *,
    verification: dict[str, Any],
    expected_revision: str,
    output: Path,
    source_roster: list[dict[str, Any]],
    verify_source_bytes: bool = True,
) -> None:
    if set(document) != STAGE_RECEIPT_FIELDS:
        raise BuildObservationError("package-stage receipt field roster differs")
    repository = document["repository"]
    object_format = (
        repository.get("object_format") if isinstance(repository, dict) else None
    )
    if (
        repository != verification["repository"]
        or object_format not in {"sha1", "sha256"}
        or not valid_git_object(repository.get("commit"), object_format)
        or not valid_git_object(repository.get("tree"), object_format)
        or repository.get("origin_main") != repository.get("commit")
        or repository.get("clean") is not True
    ):
        raise BuildObservationError("package-stage repository identity differs")
    if not isinstance(source_roster, list) or not 12 <= len(source_roster) <= 32:
        raise BuildObservationError("package-stage source roster length differs")
    expected_paths = [path.as_posix() for path in STAGE_SOURCE_PATHS]
    observed_paths: list[str] = []
    for row in source_roster:
        if not isinstance(row, dict) or set(row) != {
            "path",
            "sha256",
            "git_blob",
            "byte_count",
        }:
            raise BuildObservationError("package-stage source row differs")
        relative = safe_relative(row["path"])
        if (
            not valid_sha256(row["sha256"])
            or not valid_git_object(row["git_blob"], object_format)
            or isinstance(row["byte_count"], bool)
            or not isinstance(row["byte_count"], int)
            or not 1 <= row["byte_count"] <= MAX_SCHEMA_BYTES
        ):
            raise BuildObservationError("package-stage source identity differs")
        observed_paths.append(relative.as_posix())
    source_roster_sha256 = sha256(
        b"prisoma-observer-stage-source-roster-v1\0" + canonical(source_roster)
    )
    source_executable = {
        "size_bytes": verification["artifact"]["size_bytes"],
        "sha256": verification["artifact"]["sha256"],
        "mode": verification["artifact"]["mode"],
    }
    staged_executable = executable_identity(
        output / "bin" / "prisoma-engram-managed-observer"
    )
    inventory = package_inventory(
        output,
        PurePosixPath("bin/prisoma-engram-managed-observer"),
        [PurePosixPath(path) for path in EXPECTED_SCHEMAS.values()],
    )
    stage_input = {
        "repository": repository,
        "observed_build_receipt_exact_sha256": verification["exact_sha256"],
        "observed_build_receipt_sha256": verification["document"]["receipt_sha256"],
        "stage_source_roster_sha256": source_roster_sha256,
        "source_executable": source_executable,
    }
    if (
        document["schema_version"] != STAGE_RECEIPT_VERSION
        or document["observation_scope"]
        != "one-clean-source-package-stage-observation-v1"
        or document["observed_build_receipt_exact_sha256"]
        != verification["exact_sha256"]
        or document["observed_build_receipt_sha256"]
        != verification["document"]["receipt_sha256"]
        or observed_paths != expected_paths
        or document["stage_source_roster"] != source_roster
        or document["stage_source_roster_sha256"] != source_roster_sha256
        or document["stage_input_identity_sha256"]
        != sha256(b"prisoma-observer-stage-input-v1\0" + canonical(stage_input))
        or document["source_executable"] != source_executable
        or document["staged_executable"] != staged_executable
        or staged_executable != source_executable
        or document["package_inventory"] != inventory
        or document["package_inventory_sha256"] != sha256(canonical(inventory))
        or document["authority"] != STAGE_AUTHORITY
        or not isinstance(document["disclosure"], str)
        or not document["disclosure"]
        or document["disclosure"] != document["disclosure"].strip()
        or len(document["disclosure"].encode("utf-8")) > 4096
        or not valid_sha256(document["receipt_sha256"])
        or document["receipt_sha256"] != digest_without(document, "receipt_sha256")
    ):
        raise BuildObservationError("package-stage receipt lineage differs")
    if verify_source_bytes and stage_source_roster(expected_revision) != source_roster:
        raise BuildObservationError("package-stage source changed during re-open")


def stage(
    binary: Path,
    output: Path,
    recipe_path: Path,
    expected_binary_sha256: str,
) -> None:
    if output.exists() or output.is_symlink():
        reject(f"output already exists: {output}")
    if output.parent.resolve(strict=True) != output.parent:
        reject(f"output parent traverses a link: {output.parent}")
    recipe = load_recipe(recipe_path)
    workspace = recipe_path.parent
    executable = recipe.get("executable")
    schemas = recipe.get("schemas")
    if (
        set(recipe) != EXPECTED_RECIPE_KEYS
        or recipe.get("schema_version") != "1.0"
        or recipe.get("manifest_template") != "manifest.template.json"
        or recipe.get("package_root") != "package"
        or recipe.get("configuration_path") != "configuration.json"
        or recipe.get("output_directory") != "sealed/macos-aarch64-darwin"
        or recipe.get("target") != EXPECTED_TARGET
        or not isinstance(executable, dict)
        or set(executable) != {"package_relative_path", "launch_abi"}
        or executable.get("launch_abi") != "engram.managed-runtime-stdio.v1"
        or not isinstance(schemas, list)
        or len(schemas) != len(EXPECTED_SCHEMAS)
    ):
        reject("recipe closed authoring contract differs")
    if not isinstance(executable, dict) or not isinstance(schemas, list) or not schemas:
        reject("recipe executable and schema roster are required")
    executable_path = safe_relative(executable.get("package_relative_path"))
    if executable_path.parts[0] != "bin":
        reject("executable must be staged below bin/")

    schema_rows: list[tuple[Path, PurePosixPath, str]] = []
    schema_hashes: dict[str, str] = {}
    seen_paths: set[PurePosixPath] = set()
    seen_schema_ids: set[str] = set()
    for schema in schemas:
        if not isinstance(schema, dict) or set(schema) != {
            "schema_id",
            "package_relative_path",
        }:
            reject("schema row must be an object")
        schema_id = schema.get("schema_id")
        relative = safe_relative(schema.get("package_relative_path"))
        if (
            not isinstance(schema_id, str)
            or EXPECTED_SCHEMAS.get(schema_id) != relative.as_posix()
            or relative.parts[0] != "contracts"
            or relative in seen_paths
            or schema_id in seen_schema_ids
        ):
            reject("schema paths must be unique and below contracts/")
        seen_paths.add(relative)
        seen_schema_ids.add(schema_id)
        source = workspace.joinpath(*relative.parts)
        schema_document, schema_sha256 = load_closed_json(
            source,
            MAX_SCHEMA_BYTES,
            "schema",
        )
        expected_document_id = (
            "https://engram.local/schemas/engram.managed-runtime-ipc.v1.schema.json"
            if schema_id == "engram.managed-runtime-ipc.v1"
            else f"https://engram.local/extension-contracts/{schema_id}.json"
        )
        if (
            schema_document.get("$schema")
            != "https://json-schema.org/draft/2020-12/schema"
            or schema_document.get("$id") != expected_document_id
        ):
            reject(f"schema identity differs from recipe: {schema_id}")
        schema_hashes[schema_id] = schema_sha256
        schema_rows.append((source, relative, schema_sha256))
    if seen_schema_ids != set(EXPECTED_SCHEMAS):
        reject("recipe schema identifier roster differs")

    manifest_path = workspace / str(recipe["manifest_template"])
    manifest, _manifest_sha256 = load_closed_json(
        manifest_path,
        MAX_RECIPE_BYTES,
        "manifest template",
    )
    try:
        runtime = manifest["runtime"]
        references = [
            runtime["transport"]["contract"],
            runtime["configuration"]["schema"],
            *[
                reference
                for operation in runtime["operations"]
                for reference in (
                    operation["request_schema"],
                    operation["response_schema"],
                )
            ],
        ]
        manifest_target_id = runtime["reviewed_package"]["target_id"]
    except (KeyError, TypeError):
        reject("manifest template schema registry is incomplete")
    if (
        manifest.get("id") != "sepahead.prisoma.observer"
        or manifest_target_id != EXPECTED_TARGET["target_id"]
        or not isinstance(references, list)
        or len(references) != len(EXPECTED_SCHEMAS)
    ):
        reject("manifest template identity or schema roster differs")
    seen_manifest_ids: set[str] = set()
    for reference in references:
        if not isinstance(reference, dict) or set(reference) != {
            "schema_id",
            "schema_sha256",
        }:
            reject("manifest template schema reference is not closed")
        schema_id = reference["schema_id"]
        if (
            not isinstance(schema_id, str)
            or schema_id in seen_manifest_ids
            or schema_hashes.get(schema_id) != reference["schema_sha256"]
        ):
            reject("manifest template schema digest differs from staged bytes")
        seen_manifest_ids.add(schema_id)
    if seen_manifest_ids != set(EXPECTED_SCHEMAS):
        reject("manifest template schema identifier roster differs")

    configuration_path = safe_relative(recipe.get("configuration_path"))
    configuration = workspace.joinpath(*configuration_path.parts)
    configuration_descriptor, _ = open_regular(configuration, MAX_RECIPE_BYTES)
    os.close(configuration_descriptor)

    output.mkdir(parents=True, mode=0o700)
    try:
        copy_regular(
            binary,
            output.joinpath(*executable_path.parts),
            0o700,
            MAX_EXECUTABLE_BYTES,
            expected_sha256=expected_binary_sha256,
        )
        for source, relative, schema_sha256 in schema_rows:
            copy_regular(
                source,
                output.joinpath(*relative.parts),
                0o600,
                MAX_SCHEMA_BYTES,
                expected_sha256=schema_sha256,
            )
        staged_executable = output.joinpath(*executable_path.parts)
        if digest_regular(staged_executable, MAX_EXECUTABLE_BYTES) != (
            expected_binary_sha256
        ):
            reject("staged executable bytes differ from the observed build")
    except BaseException:
        shutil.rmtree(output)
        raise


def absolute_without_resolving_leaf(path: Path) -> Path:
    return Path(os.path.abspath(path))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=DEFAULT_BINARY)
    parser.add_argument("--binary-build-receipt", required=True, type=Path)
    parser.add_argument("--expected-prisoma-revision", required=True)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--stage-receipt", type=Path)
    parser.add_argument(
        "--recipe",
        type=Path,
        default=INTEGRATION / "authoring.macos-aarch64-darwin.json",
    )
    arguments = parser.parse_args()
    binary = absolute_without_resolving_leaf(arguments.binary)
    output = absolute_without_resolving_leaf(arguments.output)
    recipe = absolute_without_resolving_leaf(arguments.recipe)
    build_receipt = absolute_without_resolving_leaf(arguments.binary_build_receipt)
    stage_receipt = absolute_without_resolving_leaf(
        arguments.stage_receipt
        if arguments.stage_receipt is not None
        else output.parent / f"{output.name}.stage-receipt.json"
    )
    staged = False
    receipt_created = False
    try:
        expected_recipe = (
            INTEGRATION / "authoring.macos-aarch64-darwin.json"
        ).absolute()
        if (
            recipe != expected_recipe
            or recipe.resolve(strict=True) != recipe
            or stage_receipt == output
            or stage_receipt.exists()
            or stage_receipt.is_symlink()
        ):
            raise BuildObservationError(
                "package recipe or stage-receipt path differs from the closed authoring path"
            )
        verification = verify_observed_build_receipt(
            build_receipt,
            binary,
            arguments.expected_prisoma_revision,
        )
        source_before = stage_source_roster(arguments.expected_prisoma_revision)
        stage(
            binary,
            output,
            recipe,
            verification["artifact"]["sha256"],
        )
        staged = True
        final_verification = verify_observed_build_receipt(
            build_receipt,
            binary,
            arguments.expected_prisoma_revision,
        )
        if final_verification["payload"] != verification["payload"]:
            raise BuildObservationError(
                "observed-build receipt changed during package staging"
            )
        source_after = stage_source_roster(arguments.expected_prisoma_revision)
        if source_after != source_before:
            raise BuildObservationError("package-stage source changed during staging")
        source_executable = {
            "size_bytes": verification["artifact"]["size_bytes"],
            "sha256": verification["artifact"]["sha256"],
            "mode": verification["artifact"]["mode"],
        }
        staged_executable = executable_identity(
            output / "bin" / "prisoma-engram-managed-observer"
        )
        inventory = package_inventory(
            output,
            PurePosixPath("bin/prisoma-engram-managed-observer"),
            [PurePosixPath(path) for path in EXPECTED_SCHEMAS.values()],
        )
        source_roster_sha256 = sha256(
            b"prisoma-observer-stage-source-roster-v1\0" + canonical(source_before)
        )
        stage_input = {
            "repository": verification["repository"],
            "observed_build_receipt_exact_sha256": verification["exact_sha256"],
            "observed_build_receipt_sha256": verification["document"]["receipt_sha256"],
            "stage_source_roster_sha256": source_roster_sha256,
            "source_executable": source_executable,
        }
        document: dict[str, Any] = {
            "schema_version": STAGE_RECEIPT_VERSION,
            "observation_scope": "one-clean-source-package-stage-observation-v1",
            "repository": verification["repository"],
            "observed_build_receipt_exact_sha256": verification["exact_sha256"],
            "observed_build_receipt_sha256": verification["document"]["receipt_sha256"],
            "stage_source_roster": source_before,
            "stage_source_roster_sha256": source_roster_sha256,
            "stage_input_identity_sha256": sha256(
                b"prisoma-observer-stage-input-v1\0" + canonical(stage_input)
            ),
            "source_executable": source_executable,
            "staged_executable": staged_executable,
            "package_inventory": inventory,
            "package_inventory_sha256": sha256(canonical(inventory)),
            "authority": STAGE_AUTHORITY,
            "disclosure": (
                "This receipt records one clean-source package stage. "
                "It grants no installation, execution, Agent Bridge, NCP, MUSIC, "
                "physical, plant, or scientific authority."
            ),
        }
        document["receipt_sha256"] = digest_without(document, "receipt_sha256")
        validate_stage_receipt(
            document,
            verification=verification,
            expected_revision=arguments.expected_prisoma_revision,
            output=output,
            source_roster=source_before,
        )
        payload = canonical(document) + b"\n"
        write_new_receipt(stage_receipt, payload)
        receipt_created = True
        receipt_document, _ = load_closed_json(
            stage_receipt,
            MAX_RECIPE_BYTES,
            "package-stage receipt",
        )
        if (
            snapshot_regular_file(stage_receipt, MAX_RECIPE_BYTES) != payload
            or stat.S_IMODE(os.lstat(stage_receipt).st_mode) != 0o600
        ):
            raise BuildObservationError("package-stage receipt bytes or mode differ")
        validate_stage_receipt(
            receipt_document,
            verification=final_verification,
            expected_revision=arguments.expected_prisoma_revision,
            output=output,
            source_roster=source_before,
        )
    except (BuildObservationError, OSError, ValueError) as error:
        if (
            receipt_created
            and stage_receipt.exists()
            and not stage_receipt.is_symlink()
        ):
            stage_receipt.unlink()
        if staged and output.exists() and output.is_dir() and not output.is_symlink():
            shutil.rmtree(output)
        reject(f"observed release staging failed: {error}")
    print(
        f"OK: staged managed observer package at {output} with receipt {stage_receipt}"
    )


if __name__ == "__main__":
    main()
