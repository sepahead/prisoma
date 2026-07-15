#!/usr/bin/env python3
"""Fail-closed audit of the deterministic Prisoma 0.9.0 review baseline."""

from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Sequence
from pathlib import Path
from typing import Any

from generate_release_review import (
    ARTIFACT_NAMES,
    AUTHOR,
    EXPECTED_HANDOFF_BYTES,
    EXPECTED_HANDOFF_SHA256,
    EXPECTED_PHASE_COUNT,
    EXPECTED_TASK_COUNT,
    EXPECTED_TASK_GRAPH_SHA256,
    FROZEN_COMMIT,
    MANIFEST_NAME,
    MAX_ARTIFACT_BYTES,
    NOMINAL_HANDOFF_RELEASE,
    PID_RS_COMMIT,
    PROJECT,
    RELEASE_VERSION,
    REPOSITORY,
    SCHEMA_VERSION,
    ReleaseReviewError,
    _read_bounded_regular,
    _read_explicit_master_ledger,
    build_intake,
    build_inventory,
    build_manifest,
    build_task_ledger,
    pretty_json_bytes,
    resolve_repo,
    semantic_sha256,
    validate_normalized_task_graph,
)


def _fail(code: str, message: str) -> None:
    raise ReleaseReviewError(code, message)


def _reject_constant(value: str) -> Any:
    _fail("JSON_NONFINITE", f"non-finite JSON number is forbidden: {value}")


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            _fail("JSON_DUPLICATE_KEY", f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _read_json(path: Path) -> tuple[dict[str, Any], bytes]:
    raw = _read_bounded_regular(
        path,
        max_bytes=MAX_ARTIFACT_BYTES,
        path_code="ARTIFACT_PATH",
        read_code="ARTIFACT_READ",
        too_large_code="ARTIFACT_TOO_LARGE",
        description="release-review artifact",
    )
    try:
        document = json.loads(
            raw,
            object_pairs_hook=_unique_object,
            parse_constant=_reject_constant,
        )
    except ReleaseReviewError:
        raise
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        _fail("JSON_PARSE", f"cannot parse {path}: {exc}")
    except RecursionError as exc:
        _fail("JSON_DEPTH", f"JSON nesting is too deep in {path}: {exc}")
    except ValueError as exc:
        _fail("JSON_VALUE", f"invalid JSON value in {path}: {exc}")
    if not isinstance(document, dict):
        _fail("JSON_ROOT", f"artifact root must be an object: {path}")
    try:
        canonical = pretty_json_bytes(document)
    except (UnicodeError, RecursionError, ValueError) as exc:
        _fail("JSON_VALUE", f"cannot canonicalize JSON in {path}: {exc}")
    if raw != canonical:
        _fail(
            "JSON_CANONICAL",
            f"artifact is not in deterministic pretty-JSON form: {path}",
        )
    return document, raw


def _assert_exact_keys(value: Any, keys: set[str], *, context: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        actual = sorted(value) if isinstance(value, dict) else type(value).__name__
        _fail("SCHEMA_KEYS", f"{context} has wrong keys/type: {actual}")
    return value


def _first_difference(expected: Any, actual: Any, path: str = "$") -> str | None:
    if type(expected) is not type(actual):
        return f"{path}: type {type(actual).__name__} != {type(expected).__name__}"
    if isinstance(expected, dict):
        if set(expected) != set(actual):
            missing = sorted(set(expected) - set(actual))
            extra = sorted(set(actual) - set(expected))
            return f"{path}: keys differ; missing={missing}, extra={extra}"
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
        _fail(code, f"{context} differs: {difference}")


def _validate_task_ledger(document: dict[str, Any]) -> None:
    required_keys = {
        "schema_version",
        "record_type",
        "project",
        "source",
        "normalization",
        "phase_count",
        "task_count",
        "task_graph_sha256",
        "review",
        "phases",
        "tasks",
    }
    _assert_exact_keys(document, required_keys, context="normalized task ledger")
    if document["schema_version"] != SCHEMA_VERSION:
        _fail("TASK_SCHEMA_VERSION", "normalized task ledger schema version is wrong")
    if (
        document["record_type"] != "normalized_master_task_ledger"
        or document["project"] != PROJECT
    ):
        _fail("TASK_IDENTITY", "normalized task ledger identity is wrong")
    expected_source = {
        "operator_must_supply_explicit_cli_path": True,
        "expected_filename": "19_MASTER_TASK_LEDGER.yaml",
        "sha256": EXPECTED_HANDOFF_SHA256,
        "bytes": EXPECTED_HANDOFF_BYTES,
        "handoff_schema": "2.0.0",
        "declared_status": "NO_GO_PENDING_IMPLEMENTATION_AND_EVIDENCE",
        "nominal_release_target": NOMINAL_HANDOFF_RELEASE,
        "frozen_commit": FROZEN_COMMIT,
    }
    _assert_equal(
        expected_source, document["source"], code="TASK_SOURCE", context="task source"
    )
    expected_normalization = {
        "unicode": "NFC",
        "text_whitespace": "collapsed",
        "identifiers": "uppercase_fixed_width",
        "dependencies": "unique_numeric_sort",
        "path_scopes": (
            "unique_lexicographic_sort; repository metadata becomes @repository-metadata"
        ),
        "source_fields_retained": [
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
        ],
    }
    _assert_equal(
        expected_normalization,
        document["normalization"],
        code="TASK_NORMALIZATION",
        context="normalization contract",
    )
    if (
        document["phase_count"] != EXPECTED_PHASE_COUNT
        or document["task_count"] != EXPECTED_TASK_COUNT
    ):
        _fail("TASK_COUNTS", "normalized task/phase counts are wrong")
    expected_review = {
        "status": "normalized_intake_only_not_substantively_reviewed",
        "all_tasks_closed": False,
        "closed_task_ids": [],
        "human_review_complete": False,
        "independent_review_complete": False,
        "release_gate_passed": False,
    }
    _assert_equal(
        expected_review, document["review"], code="FALSE_REVIEW", context="task review"
    )
    validate_normalized_task_graph(document["phases"], document["tasks"])
    graph_sha256 = semantic_sha256(
        {"phases": document["phases"], "tasks": document["tasks"]}
    )
    if graph_sha256 != EXPECTED_TASK_GRAPH_SHA256:
        _fail(
            "TASK_GRAPH_DRIFT", f"normalized graph has unreviewed digest {graph_sha256}"
        )
    if document["task_graph_sha256"] != graph_sha256:
        _fail(
            "TASK_GRAPH_HASH",
            "stored normalized graph digest does not match its content",
        )


def _validate_intake_shape(document: dict[str, Any]) -> None:
    _assert_exact_keys(
        document,
        {
            "schema_version",
            "record_type",
            "project",
            "repository",
            "author",
            "release",
            "frozen_baseline",
            "handoff_binding",
            "review",
        },
        context="release intake",
    )
    if (
        document["schema_version"] != SCHEMA_VERSION
        or document["record_type"] != "release_review_intake"
    ):
        _fail("INTAKE_SCHEMA", "release intake schema identity is wrong")
    if document["project"] != PROJECT or document["repository"] != REPOSITORY:
        _fail("INTAKE_PROJECT", "release intake project/repository is wrong")
    author = _assert_exact_keys(
        document["author"], {"name", "basis", "scope"}, context="author"
    )
    if author["name"] != AUTHOR:
        _fail("INTAKE_AUTHOR", f"release author must be {AUTHOR}")
    release = _assert_exact_keys(
        document["release"],
        {
            "nominal_handoff_target",
            "requested_release",
            "override",
            "status",
            "doi",
            "doi_status",
            "zenodo_record",
            "zenodo_status",
            "published",
        },
        context="release override",
    )
    if release["nominal_handoff_target"] != NOMINAL_HANDOFF_RELEASE:
        _fail("INTAKE_NOMINAL_RELEASE", "handoff nominal release target is wrong")
    if release["requested_release"] != RELEASE_VERSION:
        _fail("INTAKE_RELEASE", f"requested release must be {RELEASE_VERSION}")
    if (
        release["doi"] is not None
        or release["zenodo_record"] is not None
        or release["published"] is not False
    ):
        _fail(
            "INTAKE_PUBLICATION",
            "0.9.0 intake must have no DOI, Zenodo record, or publication",
        )
    review = document["review"]
    if not isinstance(review, dict):
        _fail("INTAKE_REVIEW", "intake review state must be an object")
    forbidden_truths = (
        "all_tasks_closed",
        "human_review_complete",
        "independent_review_complete",
        "release_ready",
        "scientific_claims_established",
    )
    if any(review.get(field) is not False for field in forbidden_truths):
        _fail(
            "FALSE_REVIEW",
            "intake must not claim task, review, release, or scientific completion",
        )
    if review.get("closed_task_ids") != []:
        _fail("FALSE_TASK_CLOSURE", "baseline intake must contain no closed task ids")
    baseline = document["frozen_baseline"]
    if not isinstance(baseline, dict):
        _fail("INTAKE_BASELINE", "frozen baseline must be an object")
    if baseline.get("commit") != FROZEN_COMMIT:
        _fail("INTAKE_HEAD", "intake frozen head is wrong")
    if baseline.get("pid_rs_gitlink_commit") != PID_RS_COMMIT:
        _fail("INTAKE_SUBMODULE", "intake pid-rs gitlink identity is wrong")


def _validate_inventory_review_boundary(document: dict[str, Any]) -> None:
    review = document.get("review")
    expected_review = {
        "status": "inventory_only_unreviewed",
        "human_review_complete": False,
        "independent_review_complete": False,
        "reviewed_file_count": 0,
        "claim": "No file-review completion is asserted by this baseline inventory.",
    }
    _assert_equal(
        expected_review, review, code="FALSE_FILE_REVIEW", context="inventory review"
    )
    entries = document.get("entries")
    if not isinstance(entries, list):
        _fail("INVENTORY_ENTRIES", "inventory entries must be a list")
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            _fail("INVENTORY_ENTRY", f"inventory entry {index} is not an object")
        if (
            entry.get("review_status") != "inventory_only_unreviewed"
            or entry.get("human_reviewed") is not False
            or entry.get("independent_reviewed") is not False
        ):
            _fail(
                "FALSE_FILE_REVIEW",
                f"inventory entry {index} asserts review completion",
            )


def audit_review(
    repo: Path,
    review_dir: Path,
    *,
    master_ledger_raw: bytes | None = None,
) -> dict[str, Any]:
    repo = resolve_repo(repo)
    if review_dir.is_symlink() or not review_dir.is_dir():
        _fail(
            "REVIEW_DIRECTORY",
            f"review directory must be a real directory: {review_dir}",
        )
    expected_names = set(ARTIFACT_NAMES) | {MANIFEST_NAME}
    actual_names: set[str] = set()
    try:
        for entry in review_dir.iterdir():
            if len(actual_names) >= len(expected_names):
                _fail("ARTIFACT_SET", "review directory contains extra entries")
            if entry.is_symlink() or not entry.is_file():
                _fail("ARTIFACT_SET", f"review entry is not a regular file: {entry}")
            actual_names.add(entry.name)
    except ReleaseReviewError:
        raise
    except OSError as exc:
        _fail("REVIEW_DIRECTORY_READ", f"cannot list review directory: {exc}")
    if actual_names != expected_names:
        missing = sorted(expected_names - actual_names)
        extra = sorted(actual_names - expected_names)
        _fail(
            "ARTIFACT_SET",
            f"review artifact set differs; missing={missing}, extra={extra}",
        )
    documents: dict[str, dict[str, Any]] = {}
    raw_documents: dict[str, bytes] = {}
    for name in (*ARTIFACT_NAMES, MANIFEST_NAME):
        document, raw = _read_json(review_dir / name)
        documents[name] = document
        raw_documents[name] = raw

    task_document = documents["master_task_ledger.normalized.json"]
    _validate_task_ledger(task_document)
    if master_ledger_raw is not None:
        source_expected = build_task_ledger(master_ledger_raw)
        _assert_equal(
            source_expected,
            task_document,
            code="TASK_SOURCE_DRIFT",
            context="explicit source normalization",
        )

    inventory_document = documents["tracked_file_inventory.baseline.json"]
    _validate_inventory_review_boundary(inventory_document)
    inventory_expected = build_inventory(repo)
    _assert_equal(
        inventory_expected,
        inventory_document,
        code="INVENTORY_DRIFT",
        context="frozen Git inventory",
    )

    intake_document = documents["intake.json"]
    _validate_intake_shape(intake_document)
    intake_expected = build_intake(task_document, inventory_expected)
    _assert_equal(
        intake_expected, intake_document, code="INTAKE_DRIFT", context="release intake"
    )

    expected_payloads = {
        "intake.json": pretty_json_bytes(intake_expected),
        "master_task_ledger.normalized.json": pretty_json_bytes(task_document),
        "tracked_file_inventory.baseline.json": pretty_json_bytes(inventory_expected),
    }
    for name, expected in expected_payloads.items():
        if raw_documents[name] != expected:
            _fail("ARTIFACT_BYTES", f"{name} bytes differ from deterministic content")
    manifest_expected = build_manifest(expected_payloads)
    _assert_equal(
        manifest_expected,
        documents[MANIFEST_NAME],
        code="MANIFEST_DRIFT",
        context="artifact manifest",
    )
    if raw_documents[MANIFEST_NAME] != pretty_json_bytes(manifest_expected):
        _fail("MANIFEST_BYTES", "artifact manifest bytes are not deterministic")

    return {
        "status": "pass",
        "release_version": RELEASE_VERSION,
        "frozen_commit": FROZEN_COMMIT,
        "task_count": task_document["task_count"],
        "tracked_entry_count": inventory_document["tracked_entry_count"],
        "review_completion_claimed": False,
        "release_ready": False,
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo", type=Path, default=Path("."), help="Prisoma Git repository"
    )
    parser.add_argument(
        "--review-dir",
        type=Path,
        default=Path("release/0.9.0/review"),
        help="directory containing the tracked review baseline",
    )
    parser.add_argument(
        "--master-ledger",
        type=Path,
        help=(
            "optional explicit external 19_MASTER_TASK_LEDGER.yaml; no external path is "
            "searched or inferred when omitted"
        ),
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        raw = None
        if args.master_ledger is not None:
            raw = _read_explicit_master_ledger(args.master_ledger)
        result = audit_review(args.repo, args.review_dir, master_ledger_raw=raw)
    except ReleaseReviewError as exc:
        print(f"release review audit failed [{exc.code}]: {exc}", file=sys.stderr)
        return 3
    print(json.dumps(result, allow_nan=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
