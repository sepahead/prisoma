#!/usr/bin/env python3
"""Build and receipt one clean-source arm64 Mach-O observer release binary."""

from __future__ import annotations

import argparse
import os
import stat
import subprocess
import tempfile
from pathlib import Path

from observed_build import (
    AUTHORITY,
    BUILD_ENVIRONMENT,
    CRATE,
    MANIFEST,
    MAX_BINARY_BYTES,
    MAX_RECEIPT_BYTES,
    RELEASE_BINARY,
    ROOT,
    SCHEMA_VERSION,
    BuildObservationError,
    artifact_identity,
    canonical,
    digest_without,
    parse_arm64_macho,
    repository_identity,
    sanitized_environment,
    sha256,
    source_identity,
    stream_identity,
    toolchain_identity,
    validate_receipt_document,
    verify_observed_build_receipt,
)
from source_provenance import snapshot_regular_file


MAX_BUILD_STREAM_BYTES = 16 * 1024 * 1024
BUILD_TIMEOUT_SECONDS = 20 * 60


def parser() -> argparse.ArgumentParser:
    command = argparse.ArgumentParser(description=__doc__)
    command.add_argument("--expected-prisoma-revision", required=True)
    command.add_argument("--output-receipt", required=True, type=Path)
    command.add_argument("--verify", action="store_true")
    return command


def replace_regular(path: Path, payload: bytes, mode: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{path.name}.",
        dir=path.parent,
    )
    temporary = Path(temporary_name)
    try:
        os.fchmod(descriptor, mode)
        with os.fdopen(descriptor, "wb", closefd=True) as output:
            output.write(payload)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, path)
    finally:
        if temporary.exists():
            temporary.unlink()


def observed_build(
    expected_revision: str,
    output_receipt: Path,
) -> tuple[dict[str, object], bytes]:
    expected_receipt = RELEASE_BINARY.with_name(
        f"{RELEASE_BINARY.name}.observed-build.json"
    ).absolute()
    receipt_path = output_receipt.absolute()
    if receipt_path != expected_receipt:
        raise BuildObservationError(
            f"observed build receipt path must equal {expected_receipt}"
        )
    repository_before = repository_identity(ROOT, expected_revision)
    source_before = source_identity(ROOT)
    toolchain_before = toolchain_identity()
    if toolchain_before["rustc_host"] != "aarch64-apple-darwin":
        raise BuildObservationError(
            "observed release build requires the aarch64-apple-darwin host toolchain"
        )
    (CRATE / "target").mkdir(mode=0o700, parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(
        prefix=".observed-build-",
        dir=CRATE / "target",
    ) as raw_target:
        target_directory = Path(raw_target).absolute()
        argv = [
            toolchain_before["cargo"]["path"],
            "build",
            "--locked",
            "--offline",
            "--release",
            "--manifest-path",
            os.fspath(MANIFEST.absolute()),
            "--bin",
            "prisoma-engram-managed-observer",
            "--target-dir",
            os.fspath(target_directory),
        ]
        completed = subprocess.run(
            argv,
            cwd=ROOT,
            env=sanitized_environment(),
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=BUILD_TIMEOUT_SECONDS,
            check=False,
            close_fds=True,
        )
        if (
            completed.returncode != 0
            or len(completed.stdout) > MAX_BUILD_STREAM_BYTES
            or len(completed.stderr) > MAX_BUILD_STREAM_BYTES
        ):
            diagnostic = completed.stderr.decode("utf-8", errors="replace")[-2048:]
            raise BuildObservationError(f"observed release build failed: {diagnostic}")
        built_path = target_directory / "release" / "prisoma-engram-managed-observer"
        built_stat = os.lstat(built_path)
        if (
            not stat.S_ISREG(built_stat.st_mode)
            or built_stat.st_nlink != 1
            or built_stat.st_uid != os.geteuid()
            or built_path.resolve(strict=True) != built_path
        ):
            raise BuildObservationError("isolated Cargo output is not one regular file")
        built_payload = snapshot_regular_file(built_path, MAX_BINARY_BYTES)
        parse_arm64_macho(built_payload)
        replace_regular(RELEASE_BINARY, built_payload, 0o700)
    repository_after = repository_identity(ROOT, expected_revision)
    source_after = source_identity(ROOT)
    toolchain_after = toolchain_identity()
    artifact = artifact_identity(RELEASE_BINARY)
    if (
        repository_after != repository_before
        or source_after != source_before
        or toolchain_after != toolchain_before
        or artifact["sha256"] != sha256(built_payload)
        or artifact["size_bytes"] != len(built_payload)
    ):
        raise BuildObservationError("observed build inputs changed during execution")
    build = {
        "argv": argv,
        "profile": "release",
        "locked": True,
        "offline": True,
        "incremental": False,
        "target_directory_isolated": True,
        "environment": BUILD_ENVIRONMENT,
        "exit_code": completed.returncode,
        "stdout": stream_identity(completed.stdout),
        "stderr": stream_identity(completed.stderr),
    }
    document: dict[str, object] = {
        "schema_version": SCHEMA_VERSION,
        "observation_scope": "one-local-clean-source-build-observation-v1",
        "repository": repository_before,
        "source": source_before,
        "toolchain": toolchain_before,
        "build": build,
        "artifact": artifact,
        "authority": AUTHORITY,
        "disclosure": (
            "This receipt records one local clean-source build. "
            "It does not attest reproducibility, loaded bytes, dependencies, "
            "publisher identity, NCP, MUSIC, production execution, or scientific "
            "validity."
        ),
        "receipt_sha256": "",
    }
    document["receipt_sha256"] = digest_without(document, "receipt_sha256")
    validate_receipt_document(
        document,
        repository=repository_before,
        source=source_before,
        toolchain=toolchain_before,
        artifact=artifact,
    )
    payload = canonical(document) + b"\n"
    if len(payload) > MAX_RECEIPT_BYTES:
        raise BuildObservationError("observed-build receipt exceeds its byte bound")
    return document, payload


def main() -> int:
    arguments = parser().parse_args()
    try:
        if arguments.verify:
            verification = verify_observed_build_receipt(
                arguments.output_receipt.absolute(),
                RELEASE_BINARY,
                arguments.expected_prisoma_revision,
            )
            print(
                "OK: verified observed release build receipt "
                f"{verification['exact_sha256']}"
            )
            return 0
        _document, payload = observed_build(
            arguments.expected_prisoma_revision,
            arguments.output_receipt,
        )
        replace_regular(arguments.output_receipt.absolute(), payload, 0o600)
        verify_observed_build_receipt(
            arguments.output_receipt.absolute(),
            RELEASE_BINARY,
            arguments.expected_prisoma_revision,
        )
        print(f"OK: wrote observed release build receipt at {arguments.output_receipt}")
        return 0
    except (BuildObservationError, OSError, subprocess.TimeoutExpired) as error:
        print(f"ERROR: {error}", file=os.sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
