#!/usr/bin/env python3
"""Fail-closed audit of the immutable Prisoma 0.9 handoff requirements."""

from __future__ import annotations

import argparse
import json
import os
import stat
import sys
from collections.abc import Sequence
from pathlib import Path
from typing import Any

from generate_release_requirements import (
    ARTIFACT_MANIFEST_NAME,
    CHECKSUM_NAME,
    DISPOSITIONS_NAME,
    EXCLUDED_PATHS,
    EXPECTED_CHECKSUM_BYTES,
    EXPECTED_CHECKSUM_SHA256,
    EXPECTED_DIRECTORY_NAME,
    EXPECTED_INCLUDED_COUNT,
    EXPECTED_LEDGER_BYTES,
    EXPECTED_LEDGER_SHA256,
    EXPECTED_LENS_COUNT,
    EXPECTED_LENS_DISPOSITION_COUNT,
    EXPECTED_OUTPUT_NAMES,
    EXPECTED_PACKAGE_IDENTITY_SHA256,
    EXPECTED_PAYLOAD_COUNT,
    EXPECTED_TASK_COUNT,
    LEDGER_NAME,
    PACKAGE_MANIFEST_NAME,
    PACKAGE_IDENTITY_SEMANTICS,
    PROJECT,
    RELEASE_VERSION,
    SCHEMA_VERSION,
    RequirementsError,
    _safe_relative_path,
    build_artifact_manifest,
    build_artifacts,
    build_disposition_baseline,
    parse_master_ledger,
    pretty_json_bytes,
    semantic_sha256,
    sha256_bytes,
)


MAX_JSON_BYTES = 32 * 1024 * 1024


def fail(code: str, message: str) -> None:
    raise RequirementsError(code, message)


def _reject_constant(value: str) -> Any:
    fail("JSON_NONFINITE", f"non-finite JSON number is forbidden: {value}")


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            fail("JSON_DUPLICATE_KEY", f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _read_regular(path: Path, *, max_bytes: int) -> bytes:
    if max_bytes < 0:
        fail("ARTIFACT_TOO_LARGE", f"invalid negative byte limit: {max_bytes}")
    flags = (
        os.O_RDONLY
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_NOFOLLOW", 0)
        | getattr(os, "O_NONBLOCK", 0)
    )
    try:
        descriptor = os.open(path, flags)
    except OSError as exc:
        fail("ARTIFACT_PATH", f"cannot open regular non-symlink artifact {path}: {exc}")
    try:
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode):
            fail("ARTIFACT_PATH", f"artifact must be a regular file: {path}")
        if before.st_size < 0 or before.st_size > max_bytes:
            fail("ARTIFACT_TOO_LARGE", f"artifact exceeds {max_bytes} bytes: {path}")

        raw = bytearray()
        while len(raw) <= max_bytes:
            chunk = os.read(descriptor, min(1024 * 1024, max_bytes + 1 - len(raw)))
            if not chunk:
                break
            raw.extend(chunk)
        if len(raw) > max_bytes:
            fail("ARTIFACT_TOO_LARGE", f"artifact exceeds {max_bytes} bytes: {path}")

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
            fail("ARTIFACT_CHANGED", f"artifact changed while it was read: {path}")
        if len(raw) != after.st_size:
            fail("ARTIFACT_CHANGED", f"artifact size changed while it was read: {path}")
        try:
            named = os.stat(path, follow_symlinks=False)
        except OSError as exc:
            fail("ARTIFACT_CHANGED", f"cannot verify artifact identity {path}: {exc}")
        if not stat.S_ISREG(named.st_mode) or any(
            getattr(named, field) != getattr(after, field) for field in stable_fields
        ):
            fail("ARTIFACT_CHANGED", f"artifact path changed while it was read: {path}")
        return bytes(raw)
    except OSError as exc:
        fail("ARTIFACT_READ", f"cannot read {path}: {exc}")
    finally:
        try:
            os.close(descriptor)
        except OSError:
            pass


def _read_json(path: Path) -> tuple[dict[str, Any], bytes]:
    raw = _read_regular(path, max_bytes=MAX_JSON_BYTES)
    try:
        document = json.loads(
            raw,
            object_pairs_hook=_unique_object,
            parse_constant=_reject_constant,
        )
    except RequirementsError:
        raise
    except (
        json.JSONDecodeError,
        UnicodeDecodeError,
        RecursionError,
        ValueError,
    ) as exc:
        fail("JSON_PARSE", f"cannot parse {path}: {exc}")
    if not isinstance(document, dict):
        fail("JSON_ROOT", f"artifact root must be an object: {path}")
    try:
        canonical = pretty_json_bytes(document)
    except (UnicodeError, RecursionError, TypeError, ValueError) as exc:
        fail("JSON_CANONICALIZE", f"cannot canonicalize {path}: {exc}")
    if raw != canonical:
        fail("JSON_CANONICAL", f"artifact is not deterministic pretty JSON: {path}")
    return document, raw


def _assert_keys(value: Any, expected: set[str], *, context: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != expected:
        actual = sorted(value) if isinstance(value, dict) else type(value).__name__
        fail("SCHEMA_KEYS", f"{context} has wrong keys/type: {actual}")
    return value


def _first_difference(expected: Any, actual: Any, path: str = "$") -> str | None:
    if type(expected) is not type(actual):
        return f"{path}: type {type(actual).__name__} != {type(expected).__name__}"
    if isinstance(expected, dict):
        if set(expected) != set(actual):
            return (
                f"{path}: keys differ; missing={sorted(set(expected) - set(actual))}, "
                f"extra={sorted(set(actual) - set(expected))}"
            )
        for key in sorted(expected):
            difference = _first_difference(expected[key], actual[key], f"{path}.{key}")
            if difference is not None:
                return difference
        return None
    if isinstance(expected, list):
        if len(expected) != len(actual):
            return f"{path}: length {len(actual)} != {len(expected)}"
        for index, (expected_item, actual_item) in enumerate(
            zip(expected, actual, strict=True)
        ):
            difference = _first_difference(
                expected_item, actual_item, f"{path}[{index}]"
            )
            if difference is not None:
                return difference
        return None
    if expected != actual:
        return f"{path}: {actual!r} != {expected!r}"
    return None


def _assert_equal(expected: Any, actual: Any, *, code: str, context: str) -> None:
    difference = _first_difference(expected, actual)
    if difference is not None:
        fail(code, f"{context} differs: {difference}")


def _require_exact_directory(review_dir: Path) -> None:
    if review_dir.is_symlink() or not review_dir.is_dir():
        fail(
            "REQUIREMENTS_DIRECTORY",
            f"requirements directory must be real: {review_dir}",
        )
    names: set[str] = set()
    try:
        for entry in review_dir.iterdir():
            if len(names) >= len(EXPECTED_OUTPUT_NAMES):
                fail(
                    "REQUIREMENTS_FILE_SET", "requirements directory has extra entries"
                )
            if entry.is_symlink() or not entry.is_file():
                fail("REQUIREMENTS_ENTRY", f"unexpected non-regular entry: {entry}")
            names.add(entry.name)
    except RequirementsError:
        raise
    except OSError as exc:
        fail("REQUIREMENTS_DIRECTORY", f"cannot read requirements directory: {exc}")
    if names != EXPECTED_OUTPUT_NAMES:
        fail(
            "REQUIREMENTS_FILE_SET",
            f"requirements file set differs; missing={sorted(EXPECTED_OUTPUT_NAMES - names)}, "
            f"extra={sorted(names - EXPECTED_OUTPUT_NAMES)}",
        )


def _validate_package_manifest(document: dict[str, Any]) -> None:
    _assert_keys(
        document,
        {
            "schema_version",
            "record_type",
            "project",
            "release_version",
            "source",
            "included_file_count",
            "excluded_file_count",
            "observed_file_count",
            "package_identity_sha256",
            "identity_semantics",
            "included_files",
            "excluded_files",
            "review",
        },
        context="handoff package manifest",
    )
    if (
        document["schema_version"] != SCHEMA_VERSION
        or document["record_type"] != "immutable_handoff_package_manifest"
        or document["project"] != PROJECT
        or document["release_version"] != RELEASE_VERSION
    ):
        fail("PACKAGE_IDENTITY", "package-manifest schema/project identity is wrong")
    source = _assert_keys(
        document["source"],
        {
            "operator_supplied_explicit_directory",
            "expected_directory_name",
            "absolute_path_recorded",
            "stable_double_capture_required",
            "checksum_manifest",
        },
        context="package source",
    )
    if (
        source["operator_supplied_explicit_directory"] is not True
        or source["absolute_path_recorded"] is not False
        or source["stable_double_capture_required"] is not True
        or source["expected_directory_name"] != EXPECTED_DIRECTORY_NAME
    ):
        fail("PACKAGE_SOURCE", "package source boundary is wrong")
    checksum = _assert_keys(
        source["checksum_manifest"],
        {"path", "sha256", "bytes", "payload_entry_count"},
        context="checksum identity",
    )
    if checksum != {
        "path": CHECKSUM_NAME,
        "sha256": EXPECTED_CHECKSUM_SHA256,
        "bytes": EXPECTED_CHECKSUM_BYTES,
        "payload_entry_count": EXPECTED_PAYLOAD_COUNT,
    }:
        fail("PACKAGE_CHECKSUM", "checksum-manifest identity is wrong")
    included = document["included_files"]
    if not isinstance(included, list) or len(included) != EXPECTED_INCLUDED_COUNT:
        fail("PACKAGE_INCLUDED", "included-file records have wrong type/count")
    paths: list[str] = []
    for index, entry in enumerate(included):
        _assert_keys(
            entry,
            {"path", "sha256", "bytes", "identity_source", "role"},
            context=f"included file {index}",
        )
        path = entry["path"]
        if not isinstance(path, str):
            fail("PACKAGE_PATH", f"included file {index} path is not text")
        _safe_relative_path(path, code="PACKAGE_PATH")
        if not isinstance(entry["sha256"], str) or len(entry["sha256"]) != 64:
            fail("PACKAGE_HASH", f"included file {path} hash is malformed")
        try:
            int(entry["sha256"], 16)
        except ValueError:
            fail("PACKAGE_HASH", f"included file {path} hash is malformed")
        if type(entry["bytes"]) is not int or entry["bytes"] < 0:
            fail("PACKAGE_BYTES", f"included file {path} byte count is malformed")
        paths.append(path)
    if len(paths) != len(set(paths)) or paths != sorted(
        paths, key=lambda item: item.encode("utf-8")
    ):
        fail("PACKAGE_ORDER", "included paths are duplicated or unsorted")
    ledger = next((entry for entry in included if entry["path"] == LEDGER_NAME), None)
    checksum_entry = next(
        (entry for entry in included if entry["path"] == CHECKSUM_NAME), None
    )
    if (
        ledger is None
        or ledger["sha256"] != EXPECTED_LEDGER_SHA256
        or ledger["bytes"] != EXPECTED_LEDGER_BYTES
    ):
        fail("PACKAGE_LEDGER", "package does not bind the exact master ledger")
    if (
        checksum_entry is None
        or checksum_entry["sha256"] != EXPECTED_CHECKSUM_SHA256
        or checksum_entry["bytes"] != EXPECTED_CHECKSUM_BYTES
    ):
        fail("PACKAGE_CHECKSUM", "package does not bind its checksum manifest")
    exclusions = document["excluded_files"]
    expected_exclusions = [
        {
            "path": path,
            "present_as_regular_file": True,
            "content_read": False,
            "content_identity_bound": False,
            "reason": "operating-system metadata excluded from the requirements corpus",
        }
        for path in EXCLUDED_PATHS
    ]
    _assert_equal(
        expected_exclusions,
        exclusions,
        code="PACKAGE_EXCLUSIONS",
        context="excluded OS metadata",
    )
    package_identity = semantic_sha256(
        {"included_files": included, "excluded_paths": list(EXCLUDED_PATHS)}
    )
    if (
        package_identity != EXPECTED_PACKAGE_IDENTITY_SHA256
        or document["package_identity_sha256"] != package_identity
    ):
        fail("PACKAGE_IDENTITY", "included handoff package identity differs")
    if (
        document["included_file_count"] != EXPECTED_INCLUDED_COUNT
        or document["excluded_file_count"] != len(EXCLUDED_PATHS)
        or document["observed_file_count"]
        != EXPECTED_INCLUDED_COUNT + len(EXCLUDED_PATHS)
    ):
        fail("PACKAGE_COUNTS", "package file counts are wrong")
    if document["identity_semantics"] != PACKAGE_IDENTITY_SEMANTICS:
        fail("PACKAGE_IDENTITY_SEMANTICS", "package identity semantics are wrong")
    expected_review = {
        "status": "package_identity_verified_requirements_not_completed",
        "all_payload_hashes_verified": True,
        "all_files_read_for_substantive_review": False,
        "human_review_complete": False,
        "independent_review_complete": False,
        "release_ready": False,
    }
    _assert_equal(
        expected_review,
        document["review"],
        code="FALSE_PACKAGE_REVIEW",
        context="package review boundary",
    )


def _validate_open_dispositions(document: dict[str, Any]) -> None:
    review = document.get("review")
    expected_false = (
        "all_tasks_closed",
        "human_review_complete",
        "independent_review_complete",
        "release_gate_passed",
        "scientific_claims_established",
    )
    if not isinstance(review, dict) or any(
        review.get(field) is not False for field in expected_false
    ):
        fail("FALSE_REQUIREMENTS_REVIEW", "requirements baseline claims completion")
    if (
        review.get("open_task_count") != EXPECTED_TASK_COUNT
        or review.get("closed_task_count") != 0
        or review.get("open_lens_disposition_count") != EXPECTED_LENS_DISPOSITION_COUNT
        or review.get("closed_lens_disposition_count") != 0
    ):
        fail("FALSE_REQUIREMENTS_COUNTS", "requirements completion counts are wrong")
    tasks = document.get("tasks")
    if not isinstance(tasks, list) or len(tasks) != EXPECTED_TASK_COUNT:
        fail("REQUIREMENTS_TASKS", "requirements task count is wrong")
    lens_total = 0
    for index, task in enumerate(tasks):
        if not isinstance(task, dict):
            fail("REQUIREMENTS_TASK", f"task {index} is not an object")
        disposition = task.get("task_disposition")
        if not isinstance(disposition, dict) or disposition.get("status") != "open":
            fail("FALSE_TASK_COMPLETION", f"task {index} is not open")
        if (
            disposition.get("decision") is not None
            or disposition.get("owner") is not None
            or disposition.get("evidence_refs") != []
            or disposition.get("blockers") != []
            or disposition.get("claim_impact") is not None
            or disposition.get("reviewer") is not None
            or disposition.get("independent_reviewer") is not None
            or disposition.get("completed_at") is not None
        ):
            fail(
                "FALSE_TASK_COMPLETION",
                f"task {index} has nonempty disposition evidence",
            )
        requirements = task.get("requirements")
        required_fields = (
            "head_mismatch_rule",
            "preconditions",
            "procedure",
            "mandatory_adversarial_questions",
            "required_tests",
            "required_evidence",
            "completion_rule",
        )
        if not isinstance(requirements, dict) or any(
            not requirements.get(field) for field in required_fields
        ):
            fail(
                "REQUIREMENTS_OMITTED", f"task {index} omits full closure requirements"
            )
        lens_requirements = task.get("lens_requirements")
        lens_dispositions = task.get("lens_dispositions")
        if (
            not isinstance(lens_requirements, list)
            or not isinstance(lens_dispositions, list)
            or len(lens_requirements) != EXPECTED_LENS_COUNT
            or len(lens_dispositions) != EXPECTED_LENS_COUNT
        ):
            fail("REQUIREMENTS_LENSES", f"task {index} has wrong lens count")
        for lens_index, (requirement, disposition) in enumerate(
            zip(lens_requirements, lens_dispositions, strict=True)
        ):
            if (
                requirement.get("status") != "OPEN"
                or requirement.get("finding") != ""
                or requirement.get("evidence") != ""
                or disposition.get("status") != "open"
                or disposition.get("finding") is not None
                or disposition.get("evidence_refs") != []
                or disposition.get("blockers") != []
                or disposition.get("reviewer") is not None
                or disposition.get("reviewed_at") is not None
            ):
                fail(
                    "FALSE_LENS_COMPLETION",
                    f"task {index} lens {lens_index} is not open",
                )
        lens_total += len(lens_dispositions)
    if lens_total != EXPECTED_LENS_DISPOSITION_COUNT:
        fail("REQUIREMENTS_LENS_TOTAL", "lens-disposition total is wrong")


def audit_requirements(
    requirements_dir: Path, *, handoff_dir: Path | None = None
) -> dict[str, Any]:
    _require_exact_directory(requirements_dir)
    ledger_raw = _read_regular(
        requirements_dir / LEDGER_NAME, max_bytes=16 * 1024 * 1024
    )
    if (
        len(ledger_raw) != EXPECTED_LEDGER_BYTES
        or sha256_bytes(ledger_raw) != EXPECTED_LEDGER_SHA256
    ):
        fail(
            "LEDGER_IDENTITY",
            "tracked source copy differs from the reviewed master ledger",
        )
    parsed = parse_master_ledger(ledger_raw)
    package_document, package_raw = _read_json(requirements_dir / PACKAGE_MANIFEST_NAME)
    _validate_package_manifest(package_document)
    disposition_document, disposition_raw = _read_json(
        requirements_dir / DISPOSITIONS_NAME
    )
    _validate_open_dispositions(disposition_document)
    expected_disposition = build_disposition_baseline(parsed)
    _assert_equal(
        expected_disposition,
        disposition_document,
        code="DISPOSITION_DRIFT",
        context="task/lens disposition baseline",
    )
    manifest_document, manifest_raw = _read_json(
        requirements_dir / ARTIFACT_MANIFEST_NAME
    )
    payloads = {
        LEDGER_NAME: ledger_raw,
        PACKAGE_MANIFEST_NAME: package_raw,
        DISPOSITIONS_NAME: disposition_raw,
    }
    expected_manifest = build_artifact_manifest(payloads)
    _assert_equal(
        expected_manifest,
        manifest_document,
        code="ARTIFACT_MANIFEST_DRIFT",
        context="requirements artifact manifest",
    )
    if manifest_raw != pretty_json_bytes(expected_manifest):
        fail("ARTIFACT_MANIFEST_BYTES", "artifact manifest bytes differ")
    external_reverified = False
    if handoff_dir is not None:
        external_artifacts = build_artifacts(handoff_dir)
        actual_artifacts = {**payloads, ARTIFACT_MANIFEST_NAME: manifest_raw}
        _assert_equal(
            external_artifacts,
            actual_artifacts,
            code="EXTERNAL_HANDOFF_DRIFT",
            context="explicit external handoff regeneration",
        )
        external_reverified = True
    return {
        "status": "pass",
        "release_version": RELEASE_VERSION,
        "task_count": EXPECTED_TASK_COUNT,
        "lens_disposition_count": EXPECTED_LENS_DISPOSITION_COUNT,
        "all_dispositions_open": True,
        "review_complete": False,
        "release_ready": False,
        "external_handoff_reverified": external_reverified,
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--requirements-dir",
        type=Path,
        default=Path("release/0.9.0/requirements"),
    )
    parser.add_argument(
        "--handoff-dir",
        type=Path,
        help="optional explicit external handoff directory for byte-for-byte regeneration",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        result = audit_requirements(args.requirements_dir, handoff_dir=args.handoff_dir)
    except RequirementsError as exc:
        print(f"requirements audit failed [{exc.code}]: {exc}", file=sys.stderr)
        return 3
    print(json.dumps(result, allow_nan=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
