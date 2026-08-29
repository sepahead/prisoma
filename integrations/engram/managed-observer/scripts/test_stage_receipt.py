#!/usr/bin/env python3
"""Provider-free controls for the managed-observer package-stage receipt."""

from __future__ import annotations

import copy
import hashlib
import importlib.util
import tempfile
from pathlib import Path, PurePosixPath
from types import ModuleType
from typing import Any


SCRIPT = Path(__file__).with_name("stage-package.py")


def load_stage_module() -> ModuleType:
    specification = importlib.util.spec_from_file_location(
        "prisoma_observer_stage_package",
        SCRIPT,
    )
    if specification is None or specification.loader is None:
        raise AssertionError("stage-package module cannot be loaded")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


def expect_rejected(label: str, action: Any) -> None:
    try:
        action()
    except (OSError, SystemExit, ValueError):
        return
    raise AssertionError(f"hostile package-stage control was accepted: {label}")


def source_roster(module: ModuleType, seed: str) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for path in module.STAGE_SOURCE_PATHS:
        payload = f"synthetic stage source {seed}: {path.as_posix()}".encode()
        rows.append(
            {
                "path": path.as_posix(),
                "sha256": module.sha256(payload),
                "git_blob": hashlib.sha1(
                    payload,
                    usedforsecurity=False,
                ).hexdigest(),
                "byte_count": len(payload),
            }
        )
    return rows


def fixture(
    module: ModuleType,
    output: Path,
    seed: str,
) -> tuple[dict[str, Any], dict[str, Any], list[dict[str, Any]]]:
    executable = output / "bin" / "prisoma-engram-managed-observer"
    executable.parent.mkdir(parents=True)
    executable_payload = f"synthetic observer executable: {seed}".encode()
    executable.write_bytes(executable_payload)
    executable.chmod(0o700)
    for relative in module.EXPECTED_SCHEMAS.values():
        path = output.joinpath(*PurePosixPath(relative).parts)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(f"synthetic schema {seed}: {relative}\n", encoding="utf-8")
        path.chmod(0o600)
    repository = {
        "repository": "https://github.com/sepahead/prisoma.git",
        "commit": "a" * 40,
        "tree": "b" * 40,
        "origin_main": "a" * 40,
        "object_format": "sha1",
        "clean": True,
    }
    build_receipt_sha256 = module.sha256(f"build receipt: {seed}".encode())
    verification = {
        "repository": repository,
        "exact_sha256": module.sha256(f"build receipt bytes: {seed}".encode()),
        "document": {"receipt_sha256": build_receipt_sha256},
        "artifact": {
            "size_bytes": len(executable_payload),
            "sha256": module.sha256(executable_payload),
            "mode": "0700",
        },
    }
    sources = source_roster(module, seed)
    source_roster_sha256 = module.sha256(
        b"prisoma-observer-stage-source-roster-v1\0" + module.canonical(sources)
    )
    source_executable = verification["artifact"]
    inventory = module.package_inventory(
        output,
        PurePosixPath("bin/prisoma-engram-managed-observer"),
        [PurePosixPath(path) for path in module.EXPECTED_SCHEMAS.values()],
    )
    stage_input = {
        "repository": repository,
        "observed_build_receipt_exact_sha256": verification["exact_sha256"],
        "observed_build_receipt_sha256": build_receipt_sha256,
        "stage_source_roster_sha256": source_roster_sha256,
        "source_executable": source_executable,
    }
    document: dict[str, Any] = {
        "schema_version": module.STAGE_RECEIPT_VERSION,
        "observation_scope": "one-clean-source-package-stage-observation-v1",
        "repository": repository,
        "observed_build_receipt_exact_sha256": verification["exact_sha256"],
        "observed_build_receipt_sha256": build_receipt_sha256,
        "stage_source_roster": sources,
        "stage_source_roster_sha256": source_roster_sha256,
        "stage_input_identity_sha256": module.sha256(
            b"prisoma-observer-stage-input-v1\0" + module.canonical(stage_input)
        ),
        "source_executable": source_executable,
        "staged_executable": source_executable,
        "package_inventory": inventory,
        "package_inventory_sha256": module.sha256(module.canonical(inventory)),
        "authority": module.STAGE_AUTHORITY,
        "disclosure": "Synthetic package-stage receipt for provider-free tests.",
    }
    document["receipt_sha256"] = module.digest_without(document, "receipt_sha256")
    return document, verification, sources


def validate(
    module: ModuleType,
    document: dict[str, Any],
    verification: dict[str, Any],
    output: Path,
    sources: list[dict[str, Any]],
) -> None:
    module.validate_stage_receipt(
        document,
        verification=verification,
        expected_revision="a" * 40,
        output=output,
        source_roster=sources,
        verify_source_bytes=False,
    )


def main() -> int:
    module = load_stage_module()
    with tempfile.TemporaryDirectory(prefix="prisoma-stage-receipt-") as raw:
        root = Path(raw).resolve(strict=True)
        output = root / "package"
        document, verification, sources = fixture(module, output, "primary")
        validate(module, document, verification, output, sources)

        controls: list[tuple[str, Any]] = []
        hostile = copy.deepcopy(document)
        hostile["unreviewed"] = True
        hostile["receipt_sha256"] = module.digest_without(hostile, "receipt_sha256")
        controls.append(
            (
                "open receipt field",
                lambda hostile=hostile: validate(
                    module, hostile, verification, output, sources
                ),
            )
        )
        hostile = copy.deepcopy(document)
        hostile["authority"]["execution"] = True
        hostile["receipt_sha256"] = module.digest_without(hostile, "receipt_sha256")
        controls.append(
            (
                "execution authority",
                lambda hostile=hostile: validate(
                    module, hostile, verification, output, sources
                ),
            )
        )
        hostile = copy.deepcopy(document)
        hostile["observed_build_receipt_exact_sha256"] = "f" * 64
        hostile["receipt_sha256"] = module.digest_without(hostile, "receipt_sha256")
        controls.append(
            (
                "swapped build receipt",
                lambda hostile=hostile: validate(
                    module, hostile, verification, output, sources
                ),
            )
        )
        hostile = copy.deepcopy(document)
        hostile["stage_source_roster"][0]["path"] = "../hostile.py"
        hostile["receipt_sha256"] = module.digest_without(hostile, "receipt_sha256")
        controls.append(
            (
                "source path traversal",
                lambda hostile=hostile: validate(
                    module, hostile, verification, output, sources
                ),
            )
        )
        hostile = copy.deepcopy(document)
        hostile["package_inventory_sha256"] = "0" * 64
        hostile["receipt_sha256"] = module.digest_without(hostile, "receipt_sha256")
        controls.append(
            (
                "forged inventory digest",
                lambda hostile=hostile: validate(
                    module, hostile, verification, output, sources
                ),
            )
        )
        secondary_output = root / "secondary-package"
        secondary, _secondary_verification, _secondary_sources = fixture(
            module,
            secondary_output,
            "secondary",
        )
        hostile = copy.deepcopy(document)
        hostile["package_inventory"] = secondary["package_inventory"]
        hostile["package_inventory_sha256"] = secondary["package_inventory_sha256"]
        hostile["receipt_sha256"] = module.digest_without(hostile, "receipt_sha256")
        controls.append(
            (
                "swapped valid package inventory",
                lambda hostile=hostile: validate(
                    module, hostile, verification, output, sources
                ),
            )
        )
        for label, action in controls:
            expect_rejected(label, action)
        executable = output / "bin" / "prisoma-engram-managed-observer"
        executable.write_bytes(b"swapped staged executable")
        executable.chmod(0o700)
        expect_rejected(
            "swapped package bytes",
            lambda: validate(module, document, verification, output, sources),
        )

    print("OK: package-stage receipt accepted one fixture and rejected 7 controls")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
