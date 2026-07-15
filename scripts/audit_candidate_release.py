#!/usr/bin/env python3
"""Audit the content-bound, deliberately unpublished Prisoma 0.9 candidate."""

from __future__ import annotations

import argparse
import json
import sys
import time
from collections import Counter
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any

from generate_candidate_release import (
    ARTIFACT_MANIFEST_NAME,
    ARTIFACT_NAMES,
    CANDIDATE_RELATIVE,
    CLAIM_LEDGER_NAME,
    DEFECT_REGISTER_NAME,
    DRAFT_MANIFEST_NAME,
    EVIDENCE_RELATIVE,
    EXPECTED_LENS_DISPOSITION_COUNT,
    EXPECTED_OUTPUT_NAMES,
    EXPECTED_TASK_COUNT,
    INVENTORY_NAME,
    INVENTORY_GIT_DEADLINE_SECONDS,
    MAX_CANDIDATE_ARTIFACT_BYTES,
    MAX_INVENTORY_CONTENT_BYTES,
    MAX_INVENTORY_ENTRIES,
    MAX_INVENTORY_LISTING_BYTES,
    MAX_INVENTORY_PATH_BYTES,
    MAX_FILE_BYTES,
    PROJECT,
    PROGRESS_RELATIVE,
    RECURSIVE_GITLINK_PATHS,
    RECEIPTS_NAME,
    RELEASE_VERSION,
    REPOSITORY,
    SCHEMA_VERSION,
    TASK_LEDGER_NAME,
    TERMINAL_PROMOTION_POLICY,
    CandidateError,
    _apply_file_review_progress,
    _classify_file_review,
    _index_state,
    _OID_RE,
    _parse_head_tree,
    _project_pinned_gitlink,
    _read_bounded_regular,
    _remaining_capture_seconds,
    _run_git,
    _SHA256_RE,
    _validate_progress_document,
    _worktree_state,
    build_artifact_manifest,
    build_artifacts_from_inventory,
    capture_stable_inventory,
    pretty_json_bytes,
    resolve_repo,
    semantic_sha256,
    sha256_bytes,
)


SHA256_EMPTY = sha256_bytes(b"")


def fail(code: str, message: str) -> None:
    raise CandidateError(code, message)


def _reject_constant(value: str) -> Any:
    fail("JSON_NONFINITE", f"non-finite JSON number is forbidden: {value}")


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            fail("JSON_DUPLICATE_KEY", f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _read_json(path: Path) -> tuple[dict[str, Any], bytes]:
    raw, _ = _read_bounded_regular(
        path,
        max_bytes=MAX_CANDIDATE_ARTIFACT_BYTES,
        path_code="ARTIFACT_PATH",
        read_code="ARTIFACT_READ",
        size_code="ARTIFACT_SIZE",
        description="candidate artifact",
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
        fail("JSON_PARSE", f"cannot parse candidate artifact {path}: {exc}")
    if not isinstance(value, dict):
        fail("JSON_ROOT", f"candidate artifact root must be an object: {path}")
    try:
        canonical = pretty_json_bytes(value)
    except (RecursionError, TypeError, UnicodeError, ValueError) as exc:
        fail("JSON_CANONICAL", f"cannot canonicalize candidate artifact {path}: {exc}")
    if raw != canonical:
        fail(
            "JSON_CANONICAL", f"candidate artifact is not canonical pretty JSON: {path}"
        )
    return value, raw


def _assert_keys(value: Any, expected: set[str], *, context: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != expected:
        actual = sorted(value) if isinstance(value, dict) else type(value).__name__
        fail("SCHEMA_KEYS", f"{context} has wrong keys/type: {actual}")
    return value


def _require_candidate_directory(candidate_dir: Path) -> None:
    if candidate_dir.is_symlink() or not candidate_dir.is_dir():
        fail(
            "CANDIDATE_DIRECTORY", f"candidate directory must be real: {candidate_dir}"
        )
    names: set[str] = set()
    try:
        for entry in candidate_dir.iterdir():
            if len(names) >= len(EXPECTED_OUTPUT_NAMES):
                fail("CANDIDATE_FILE_SET", "candidate directory contains extra entries")
            if entry.is_symlink() or not entry.is_file():
                fail(
                    "CANDIDATE_ENTRY",
                    f"unexpected non-regular candidate entry: {entry}",
                )
            names.add(entry.name)
    except CandidateError:
        raise
    except OSError as exc:
        fail("CANDIDATE_DIRECTORY", f"cannot read candidate directory: {exc}")
    if names != EXPECTED_OUTPUT_NAMES:
        fail(
            "CANDIDATE_FILE_SET",
            f"candidate file set differs; missing={sorted(EXPECTED_OUTPUT_NAMES - names)}, "
            f"extra={sorted(names - EXPECTED_OUTPUT_NAMES)}",
        )


def _validate_artifact_manifest(
    manifest: Mapping[str, Any], raw_by_name: Mapping[str, bytes]
) -> None:
    _assert_keys(
        manifest,
        {
            "schema_version",
            "record_type",
            "project",
            "release_version",
            "candidate_state_sha256",
            "artifacts",
            "status",
            "release_ready",
            "published",
        },
        context="candidate artifact manifest",
    )
    if (
        manifest["schema_version"] != SCHEMA_VERSION
        or manifest["record_type"] != "candidate_artifact_manifest"
        or manifest["project"] != PROJECT
        or manifest["release_version"] != RELEASE_VERSION
    ):
        fail("MANIFEST_IDENTITY", "candidate artifact-manifest identity is wrong")
    if (
        manifest["status"] != "integrity_manifest_for_unpublished_candidate"
        or manifest["release_ready"] is not False
        or manifest["published"] is not False
    ):
        fail(
            "FALSE_PUBLICATION",
            "candidate artifact manifest claims readiness/publication",
        )
    records = manifest["artifacts"]
    if not isinstance(records, list) or len(records) != len(ARTIFACT_NAMES):
        fail("MANIFEST_ARTIFACTS", "candidate artifact list has wrong type/count")
    expected_records = [
        {
            "path": name,
            "sha256": sha256_bytes(raw_by_name[name]),
            "bytes": len(raw_by_name[name]),
        }
        for name in sorted(ARTIFACT_NAMES)
    ]
    if records != expected_records:
        fail("MANIFEST_HASH", "candidate artifact hashes or byte counts differ")


def _default_file_review(entry: Mapping[str, Any]) -> dict[str, Any]:
    index = entry["index"]
    expected_blob = None if index is None or index["mode"] == "160000" else index["oid"]
    return {
        "path": entry["path"],
        "git_blob_id": expected_blob,
        "sha256": entry["working_tree"]["sha256"],
        **_classify_file_review(entry["path"]),
        "reviewer": None,
        "independent_reviewer": None,
        "line_count": entry["working_tree"]["line_count"],
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


def _validate_file_review(
    entry: Mapping[str, Any], expected_review: Mapping[str, Any]
) -> None:
    review = entry["file_review"]
    if not isinstance(review, dict) or review != expected_review:
        fail("FALSE_FILE_REVIEW", f"file-review merge differs at {entry['path']}")


def _validate_inventory_internal(
    repo: Path, inventory: Mapping[str, Any]
) -> dict[str, Any]:
    _assert_keys(
        inventory,
        {
            "schema_version",
            "record_type",
            "project",
            "repository",
            "release_version",
            "source",
            "inventory_policy",
            "progress_snapshot",
            "summary",
            "file_review_template_contract",
            "entries",
            "candidate_boundary",
        },
        context="candidate source inventory",
    )
    if (
        inventory["schema_version"] != SCHEMA_VERSION
        or inventory["record_type"] != "candidate_source_inventory"
        or inventory["project"] != PROJECT
        or inventory["repository"] != REPOSITORY
        or inventory["release_version"] != RELEASE_VERSION
    ):
        fail("INVENTORY_IDENTITY", "candidate inventory identity is wrong")
    source = _assert_keys(
        inventory["source"],
        {
            "head_commit",
            "head_tree",
            "index_entries_sha256",
            "worktree_entries_sha256",
            "candidate_state_sha256",
            "clean",
            "state",
            "explicit_source_arguments_required",
            "recursive_gitlinks",
        },
        context="candidate source identity",
    )
    if not isinstance(source["recursive_gitlinks"], list) or any(
        not isinstance(record, dict) for record in source["recursive_gitlinks"]
    ):
        fail("INVENTORY_GITLINK_SOURCE", "recursive gitlink sources are not records")
    policy = inventory["inventory_policy"]
    if (
        policy.get("basis")
        != (
            "git_head_plus_index_plus_worktree_nonignored_untracked_and_pinned_"
            "gitlink_commit_trees"
        )
        or policy.get("self_excluded_paths") != [CANDIDATE_RELATIVE]
        or policy.get("ignored_files_included") is not False
        or policy.get("dirty_gitlinks_permitted") is not False
        or policy.get("recursive_gitlink_paths")
        != [record.get("path") for record in source["recursive_gitlinks"]]
        or policy.get("recursive_rows_derived_from")
        != "pinned_index_commit_git_objects"
        or policy.get("stable_double_capture_required") is not True
        or policy.get("resource_limits")
        != {
            "max_entry_count": MAX_INVENTORY_ENTRIES,
            "max_path_bytes": MAX_INVENTORY_PATH_BYTES,
            "max_git_object_bytes_per_blob": MAX_FILE_BYTES,
            "max_recursive_gitlink_count": len(RECURSIVE_GITLINK_PATHS),
            "max_index_and_worktree_content_bytes": MAX_INVENTORY_CONTENT_BYTES,
            "max_generated_artifact_bytes": MAX_CANDIDATE_ARTIFACT_BYTES,
            "git_subprocess_deadline_seconds_per_capture": (
                INVENTORY_GIT_DEADLINE_SECONDS
            ),
        }
    ):
        fail("INVENTORY_POLICY", "candidate inventory policy is incomplete or changed")
    fixed_point = policy.get("fixed_point_semantics")
    if not isinstance(fixed_point, dict) or fixed_point != {
        "excluded_namespace": CANDIDATE_RELATIVE,
        "source_digest_excludes_candidate_outputs": True,
        "candidate_outputs_bound_by": ARTIFACT_MANIFEST_NAME,
        "regeneration_property": (
            "For unchanged non-candidate source bytes, index identity, and pinned gitlink "
            "commit objects, regeneration is byte-deterministic and changes only the "
            "excluded candidate namespace."
        ),
        "review_boundary": (
            "Candidate output files are governed artifacts, not source files silently counted "
            "as reviewed."
        ),
    }:
        fail("FIXED_POINT_POLICY", "candidate fixed-point semantics drifted")
    template = inventory["file_review_template_contract"]
    expected_template_fields = [
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
    ]
    if (
        not isinstance(template, dict)
        or template.get("fields") != expected_template_fields
        or template.get("extended_fields")
        != ["independent_reviewer", "decision", "updated_at"]
        or template.get("generated_defaults_unreviewed") is not True
        or template.get("terminal_promotion_enabled") is not False
        or template.get("terminal_promotion_policy") != TERMINAL_PROMOTION_POLICY
    ):
        fail("FILE_REVIEW_TEMPLATE", "file-review template contract is incomplete")

    progress_snapshot = _assert_keys(
        inventory["progress_snapshot"],
        {"path", "sha256", "bytes", "semantic_sha256", "document"},
        context="candidate progress snapshot",
    )
    progress_raw = pretty_json_bytes(progress_snapshot["document"])
    progress, _ = _validate_progress_document(
        progress_snapshot["document"], progress_raw
    )
    if (
        progress_snapshot["path"] != PROGRESS_RELATIVE
        or progress_snapshot["sha256"] != sha256_bytes(progress_raw)
        or progress_snapshot["bytes"] != len(progress_raw)
        or progress_snapshot["semantic_sha256"] != semantic_sha256(progress)
        or template.get("progress_update_count") != len(progress["file_review_updates"])
    ):
        fail("PROGRESS_SNAPSHOT", "embedded candidate progress identity differs")

    entries = inventory["entries"]
    if not isinstance(entries, list) or not entries:
        fail("INVENTORY_ENTRIES", "candidate inventory entries are absent")
    paths = [entry.get("path") for entry in entries if isinstance(entry, dict)]
    if (
        len(paths) != len(entries)
        or any(not isinstance(path, str) for path in paths)
        or paths != sorted(paths)
        or len(paths) != len(set(paths))
    ):
        fail("INVENTORY_PATHS", "candidate inventory paths are not unique and sorted")
    if any(
        path == CANDIDATE_RELATIVE or path.startswith(f"{CANDIDATE_RELATIVE}/")
        for path in paths
    ):
        fail("INVENTORY_SELF_REFERENCE", "candidate outputs occur in source inventory")
    progress_entry = next(
        (entry for entry in entries if entry["path"] == PROGRESS_RELATIVE), None
    )
    if (
        progress_entry is None
        or progress_entry["working_tree"]["kind"] != "regular"
        or progress_entry["working_tree"]["sha256"] != sha256_bytes(progress_raw)
        or progress_entry["working_tree"]["bytes"] != len(progress_raw)
        or progress_entry["working_tree"]["line_count"]
        != progress_raw.count(b"\n")
        + int(bool(progress_raw) and not progress_raw.endswith(b"\n"))
    ):
        fail("PROGRESS_SNAPSHOT", "progress snapshot differs from its inventory entry")

    expected_review_entries = json.loads(json.dumps(entries))
    for expected_entry in expected_review_entries:
        expected_entry["file_review"] = _default_file_review(expected_entry)
    _apply_file_review_progress(expected_review_entries, progress)
    expected_reviews = {
        entry["path"]: entry["file_review"] for entry in expected_review_entries
    }

    if (
        len(entries) > MAX_INVENTORY_ENTRIES
        or sum(len(path.encode("utf-8")) for path in paths) > MAX_INVENTORY_PATH_BYTES
    ):
        fail("INVENTORY_BUDGET", "candidate inventory exceeds its path/entry budget")

    audit_deadline = time.monotonic() + INVENTORY_GIT_DEADLINE_SECONDS

    def audit_git(target: Path, args: Sequence[str], *, max_bytes: int) -> bytes:
        return _run_git(
            target,
            args,
            max_bytes=max_bytes,
            timeout_seconds=_remaining_capture_seconds(audit_deadline),
        )

    head_tree = _parse_head_tree(
        audit_git(
            repo,
            ["ls-tree", "-r", "-z", source["head_commit"]],
            max_bytes=MAX_INVENTORY_LISTING_BYTES,
        )
    )
    observed_tree = (
        audit_git(
            repo,
            ["rev-parse", f"{source['head_commit']}^{{tree}}"],
            max_bytes=1024,
        )
        .decode("ascii")
        .strip()
    )
    if observed_tree != source["head_tree"]:
        fail(
            "INVENTORY_HEAD",
            "recorded HEAD tree does not match the Git object database",
        )
    parent_entries_by_path: dict[str, Mapping[str, Any]] = {}
    recorded_recursive_paths: set[str] = set()
    for entry in entries:
        origin = _assert_keys(
            entry.get("inventory_origin"),
            {"kind", "gitlink_path", "gitlink_commit", "gitlink_tree"},
            context=f"inventory origin {entry.get('path')}",
        )
        if origin["kind"] == "parent_repository":
            if any(
                origin[field] is not None
                for field in ("gitlink_path", "gitlink_commit", "gitlink_tree")
            ):
                fail(
                    "INVENTORY_ORIGIN",
                    f"parent inventory origin carries gitlink metadata: {entry['path']}",
                )
            parent_entries_by_path[entry["path"]] = entry
        elif origin["kind"] == "pinned_gitlink_file":
            recorded_recursive_paths.add(entry["path"])
        else:
            fail("INVENTORY_ORIGIN", f"unsupported origin at {entry['path']}")

    missing_head_paths = sorted(set(head_tree) - set(parent_entries_by_path))
    if missing_head_paths:
        fail(
            "INVENTORY_HEAD_COVERAGE",
            "candidate inventory omits HEAD path(s): "
            + ", ".join(missing_head_paths[:8]),
        )

    recursive_source = source["recursive_gitlinks"]
    if not isinstance(recursive_source, list):
        fail(
            "INVENTORY_GITLINK_SOURCE",
            "recursive gitlink source records are not a list",
        )
    recursive_source_paths: list[str] = []
    for record in recursive_source:
        _assert_keys(
            record,
            {"path", "commit", "tree", "entry_count", "entries_sha256"},
            context="recursive gitlink source record",
        )
        recursive_source_paths.append(record["path"])
    if (
        any(not isinstance(path, str) for path in recursive_source_paths)
        or recursive_source_paths != sorted(set(recursive_source_paths))
        or len(recursive_source_paths) > len(RECURSIVE_GITLINK_PATHS)
        or any(path not in RECURSIVE_GITLINK_PATHS for path in recursive_source_paths)
    ):
        fail("INVENTORY_GITLINK_SOURCE", "recursive gitlink source paths are invalid")
    expected_recursive_roots = sorted(
        path
        for path in RECURSIVE_GITLINK_PATHS
        if (
            path in parent_entries_by_path
            and isinstance(parent_entries_by_path[path].get("index"), dict)
            and parent_entries_by_path[path]["index"].get("mode") == "160000"
        )
    )
    if recursive_source_paths != expected_recursive_roots:
        fail(
            "INVENTORY_GITLINK_COVERAGE",
            "recursive gitlink source records omit or add a configured pinned root",
        )

    projected_by_path: dict[str, dict[str, Any]] = {}
    for record in recursive_source:
        parent_entry = parent_entries_by_path[record["path"]]
        parent_index = parent_entry["index"]
        if record["commit"] != parent_index["oid"]:
            fail(
                "INVENTORY_GITLINK_SOURCE",
                f"recursive commit differs from parent index at {record['path']}",
            )
        expected_record, projected, _ = _project_pinned_gitlink(
            repo,
            record["path"],
            record["commit"],
            capture_deadline=audit_deadline,
        )
        if record != expected_record:
            fail(
                "INVENTORY_GITLINK_SOURCE",
                f"recursive source binding differs at {record['path']}",
            )
        for projected_entry in projected:
            if projected_entry["path"] in projected_by_path:
                fail("INVENTORY_GITLINK_COVERAGE", "recursive projections overlap")
            projected_by_path[projected_entry["path"]] = projected_entry
    if recorded_recursive_paths != set(projected_by_path):
        missing = sorted(set(projected_by_path) - recorded_recursive_paths)
        extra = sorted(recorded_recursive_paths - set(projected_by_path))
        fail(
            "INVENTORY_GITLINK_COVERAGE",
            f"recursive rows differ; missing={missing[:8]}, extra={extra[:8]}",
        )

    index_identity = []
    working_identity = []
    content_bytes = 0
    for entry in entries:
        _assert_keys(
            entry,
            {
                "path",
                "inventory_origin",
                "head",
                "index",
                "index_state",
                "working_tree",
                "worktree_state",
                "file_review",
            },
            context=f"inventory entry {entry.get('path')}",
        )
        origin = entry["inventory_origin"]
        is_parent = origin["kind"] == "parent_repository"
        if is_parent:
            if entry["head"] != head_tree.get(entry["path"]):
                fail("INVENTORY_HEAD_ENTRY", f"HEAD binding differs at {entry['path']}")
        else:
            expected_entry = projected_by_path.get(entry["path"])
            if expected_entry is None or origin != expected_entry["inventory_origin"]:
                fail(
                    "INVENTORY_GITLINK_ENTRY",
                    f"recursive origin differs at {entry['path']}",
                )
            for field in (
                "head",
                "index",
                "index_state",
                "working_tree",
                "worktree_state",
            ):
                if entry[field] != expected_entry[field]:
                    fail(
                        "INVENTORY_GITLINK_ENTRY",
                        f"recursive {field} differs at {entry['path']}",
                    )
        working = _assert_keys(
            entry["working_tree"],
            {
                "kind",
                "mode",
                "sha256",
                "bytes",
                "line_count",
                "link_target",
                "gitlink_head",
                "gitlink_status_sha256",
            },
            context=f"working-tree entry {entry['path']}",
        )
        index = entry["index"]
        if index is not None:
            _assert_keys(
                index,
                {"mode", "oid", "content_sha256", "bytes"},
                context=f"index entry {entry['path']}",
            )
            if (
                index["mode"] not in {"100644", "100755", "120000", "160000"}
                or not isinstance(index["oid"], str)
                or _OID_RE.fullmatch(index["oid"]) is None
            ):
                fail("INVENTORY_INDEX", f"invalid index identity at {entry['path']}")
            if index["mode"] != "160000" and (
                not isinstance(index["content_sha256"], str)
                or _SHA256_RE.fullmatch(index["content_sha256"]) is None
                or type(index["bytes"]) is not int
                or index["bytes"] < 0
                or index["bytes"] > MAX_FILE_BYTES
            ):
                fail("INVENTORY_INDEX", f"invalid index content at {entry['path']}")
            if type(index["bytes"]) is int and index["bytes"] >= 0:
                content_bytes += index["bytes"]
            elif index["bytes"] is not None:
                fail("INVENTORY_CONTENT", f"invalid index size at {entry['path']}")
            if is_parent:
                index_identity.append(
                    {
                        "path": entry["path"],
                        "mode": index["mode"],
                        "oid": index["oid"],
                        "stage": 0,
                    }
                )
            if index["mode"] == "160000":
                if index["content_sha256"] is not None or index["bytes"] is not None:
                    fail(
                        "INVENTORY_GITLINK",
                        f"gitlink has blob fields at {entry['path']}",
                    )
                if (
                    working["kind"] != "gitlink"
                    or working["gitlink_head"] != index["oid"]
                    or working["gitlink_status_sha256"] != SHA256_EMPTY
                ):
                    fail(
                        "INVENTORY_GITLINK", f"gitlink is not clean at {entry['path']}"
                    )
            elif is_parent:
                blob = audit_git(
                    repo,
                    ["cat-file", "blob", index["oid"]],
                    max_bytes=MAX_FILE_BYTES,
                )
                if (
                    sha256_bytes(blob) != index["content_sha256"]
                    or len(blob) != index["bytes"]
                ):
                    fail(
                        "INVENTORY_INDEX_BLOB", f"index blob differs at {entry['path']}"
                    )
        if type(working["bytes"]) is int and 0 <= working["bytes"] <= MAX_FILE_BYTES:
            content_bytes += working["bytes"]
        elif working["bytes"] is not None:
            fail("INVENTORY_CONTENT", f"invalid working size at {entry['path']}")
        if content_bytes > MAX_INVENTORY_CONTENT_BYTES:
            fail("INVENTORY_BUDGET", "inventory content exceeds its aggregate budget")
        expected_index_state = _index_state(index, entry["head"])
        if entry["index_state"] != expected_index_state:
            fail(
                "INVENTORY_INDEX_STATE",
                f"index state differs from HEAD/index records at {entry['path']}",
            )
        expected_worktree_state = _worktree_state(entry)
        if entry["worktree_state"] != expected_worktree_state:
            fail(
                "INVENTORY_WORKTREE_STATE",
                f"worktree state differs from index/working records at {entry['path']}",
            )
        _validate_file_review(entry, expected_reviews[entry["path"]])
        working_identity.append(
            {
                "path": entry["path"],
                "inventory_origin": origin,
                "head": entry["head"],
                "index": index,
                "index_state": entry["index_state"],
                "working_tree": working,
                "worktree_state": entry["worktree_state"],
            }
        )

    if semantic_sha256(index_identity) != source["index_entries_sha256"]:
        fail("INVENTORY_INDEX_HASH", "candidate index semantic hash differs")
    if semantic_sha256(working_identity) != source["worktree_entries_sha256"]:
        fail("INVENTORY_WORKTREE_HASH", "candidate working-tree semantic hash differs")
    state_material = {
        "head_commit": source["head_commit"],
        "head_tree": source["head_tree"],
        "index_entries_sha256": source["index_entries_sha256"],
        "worktree_entries_sha256": source["worktree_entries_sha256"],
        "recursive_gitlinks": recursive_source,
        "self_exclusions": [CANDIDATE_RELATIVE],
        "progress_snapshot": progress_snapshot,
        "entries": entries,
    }
    if semantic_sha256(state_material) != source["candidate_state_sha256"]:
        fail("INVENTORY_STATE_HASH", "candidate source-state semantic hash differs")

    parent_entries = list(parent_entries_by_path.values())
    index_changed = sum(entry["index_state"] != "unchanged" for entry in parent_entries)
    worktree_changed = sum(
        entry["worktree_state"] != "unchanged" for entry in parent_entries
    )
    untracked = sum(entry["index_state"] == "untracked" for entry in parent_entries)
    clean = index_changed == 0 and worktree_changed == 0
    expected_summary = {
        "entry_count": len(entries),
        "parent_entry_count": len(parent_entries),
        "recursive_gitlink_entry_count": len(projected_by_path),
        "indexed_entry_count": len(index_identity),
        "untracked_entry_count": untracked,
        "index_changed_entry_count": index_changed,
        "worktree_changed_entry_count": worktree_changed,
        "clean": clean,
    }
    if inventory["summary"] != expected_summary:
        fail("INVENTORY_SUMMARY", "candidate inventory summary differs")
    if source["clean"] is not clean or source["state"] != (
        "clean_source_snapshot" if clean else "dirty_uncommitted_source_snapshot"
    ):
        fail("FALSE_CLEAN_STATE", "candidate inventory clean/dirty status differs")
    if source["explicit_source_arguments_required"] is not True:
        fail("EXPLICIT_SOURCE", "candidate inventory drops explicit source arguments")
    return progress


def _working_projection(inventory: Mapping[str, Any]) -> list[dict[str, Any]]:
    return [
        {
            "path": entry["path"],
            "inventory_origin": entry["inventory_origin"],
            "working_tree": entry["working_tree"],
        }
        for entry in inventory["entries"]
    ]


def _validate_live_coverage(repo: Path, recorded: Mapping[str, Any]) -> str:
    current = capture_stable_inventory(repo)
    if (
        current["source"] == recorded["source"]
        and current["entries"] == recorded["entries"]
    ):
        return "exact_source_state"
    if _working_projection(current) != _working_projection(recorded):
        fail(
            "LIVE_SOURCE_DRIFT",
            "current non-candidate source paths or bytes differ from the recorded candidate",
        )
    if current["source"]["clean"] is not True:
        fail(
            "LIVE_SOURCE_NOT_CONVERGED",
            "source differs from the recorded index state and is not a clean convergence",
        )
    completed = _run_git(
        repo,
        [
            "merge-base",
            "--is-ancestor",
            recorded["source"]["head_commit"],
            current["source"]["head_commit"],
        ],
        max_bytes=1024,
    )
    if completed != b"":
        fail("LIVE_SOURCE_ANCESTRY", "candidate convergence has unexpected Git output")
    return "committed_content_convergence"


def _validate_semantic_boundaries(documents: Mapping[str, Mapping[str, Any]]) -> None:
    inventory = documents[INVENTORY_NAME]
    task_ledger = documents[TASK_LEDGER_NAME]
    claims = documents[CLAIM_LEDGER_NAME]
    defects = documents[DEFECT_REGISTER_NAME]
    receipts = documents[RECEIPTS_NAME]
    draft = documents[DRAFT_MANIFEST_NAME]
    manifest = documents[ARTIFACT_MANIFEST_NAME]
    state_hash = inventory["source"]["candidate_state_sha256"]
    for name in (
        TASK_LEDGER_NAME,
        CLAIM_LEDGER_NAME,
        DEFECT_REGISTER_NAME,
        RECEIPTS_NAME,
    ):
        if documents[name].get("source_state_sha256") != state_hash:
            fail("SOURCE_BINDING", f"candidate source-state binding differs in {name}")
    if manifest.get("candidate_state_sha256") != state_hash:
        fail("SOURCE_BINDING", "candidate artifact manifest source binding differs")
    if (
        draft.get("source", {}).get("candidate_state_sha256") != state_hash
        or draft.get("source_candidate", {}).get("state_sha256") != state_hash
    ):
        fail("SOURCE_BINDING", "draft manifest source binding differs")
    receipt_rows = receipts.get("receipts")
    expected_receipt_statuses = {
        "pending_post_push_ci",
        "in_progress",
        "blocked",
        "failed",
        "passed",
    }
    if (
        not isinstance(receipt_rows, list)
        or set(receipts.get("receipt_contract", {}).get("statuses", []))
        != expected_receipt_statuses
        or receipts.get("receipt_contract", {}).get("terminal_promotion_enabled")
        is not False
        or receipts.get("receipt_contract", {}).get("terminal_promotion_policy")
        != TERMINAL_PROMOTION_POLICY
    ):
        fail("EVIDENCE_RECEIPT_STATUS", "candidate receipt contract differs")
    receipt_ids = [
        receipt.get("id") for receipt in receipt_rows if isinstance(receipt, dict)
    ]
    if (
        len(receipt_ids) != len(receipt_rows)
        or len(receipt_ids) != len(set(receipt_ids))
        or any(not isinstance(receipt_id, str) for receipt_id in receipt_ids)
    ):
        fail("EVIDENCE_RECEIPT_STATUS", "candidate receipt identifiers differ")
    receipt_status: dict[str, str] = {}
    for receipt in receipt_rows:
        status_value = receipt.get("status")
        if status_value not in expected_receipt_statuses:
            fail("EVIDENCE_RECEIPT_STATUS", "candidate receipt status is unsupported")
        if status_value == "passed":
            fail(
                "TERMINAL_PROMOTION_DISABLED",
                f"schema 0.1 contains a positive receipt outcome: {receipt['id']}",
            )
        receipt_id = receipt["id"]
        receipt_status[receipt_id] = status_value
        evidence_path = receipt.get("evidence", {}).get("log_artifact")
        if evidence_path is not None:
            evidence_entry = next(
                (
                    entry
                    for entry in inventory["entries"]
                    if entry["path"] == evidence_path
                ),
                None,
            )
            if (
                not isinstance(evidence_path, str)
                or not evidence_path.startswith(f"{EVIDENCE_RELATIVE}/")
                or evidence_entry is None
                or evidence_entry["working_tree"]["kind"] != "regular"
            ):
                fail(
                    "FALSE_EVIDENCE_RECEIPT",
                    f"receipt evidence path is outside the regular log namespace: {receipt_id}",
                )
        reviewer = receipt.get("reviewer")
        independent = receipt.get("independent_reviewer")
        if reviewer is not None and reviewer == independent:
            fail(
                "EVIDENCE_RECEIPT_REVIEW",
                f"receipt reviewers are not independent: {receipt_id}",
            )
        if status_value in {"passed", "failed"} and (
            not reviewer
            or not independent
            or not receipt.get("evidence", {}).get("sha256")
            or not receipt.get("execution", {}).get("commit")
        ):
            fail(
                "FALSE_EVIDENCE_RECEIPT",
                f"final receipt lacks exact independent evidence: {receipt_id}",
            )
        if status_value == "passed" and (
            receipt.get("conclusion") != "success"
            or receipt.get("execution", {}).get("commit")
            != inventory["source"]["head_commit"]
        ):
            fail(
                "FALSE_EVIDENCE_RECEIPT",
                f"passed receipt is not bound to the exact source HEAD: {receipt_id}",
            )
        if status_value == "failed" and receipt.get("conclusion") != "failure":
            fail(
                "FALSE_EVIDENCE_RECEIPT",
                f"failed receipt is inconsistent: {receipt_id}",
            )
        if status_value == "in_progress" and receipt.get("conclusion") != "running":
            fail(
                "FALSE_EVIDENCE_RECEIPT",
                f"running receipt is inconsistent: {receipt_id}",
            )
        if status_value == "blocked" and receipt.get("conclusion") != "blocked":
            fail(
                "FALSE_EVIDENCE_RECEIPT",
                f"blocked receipt is inconsistent: {receipt_id}",
            )
    passed_receipts = [
        receipt for receipt in receipt_rows if receipt["status"] == "passed"
    ]
    if passed_receipts:
        permitted_drift_paths = {PROGRESS_RELATIVE}
        permitted_drift_paths.update(
            receipt["evidence"]["log_artifact"]
            for receipt in receipt_rows
            if receipt.get("evidence", {}).get("log_artifact") is not None
        )
        if any(
            entry["path"] not in permitted_drift_paths
            and (
                entry["index_state"] != "unchanged"
                or entry["worktree_state"] != "unchanged"
            )
            for entry in inventory["entries"]
        ):
            fail(
                "FALSE_EVIDENCE_RECEIPT",
                "passed receipts coexist with non-evidence source drift",
            )
    receipt_counts = Counter(receipt_status.values())
    expected_receipt_summary = {
        "receipt_count": len(receipt_rows),
        "pending_count": receipt_counts["pending_post_push_ci"],
        "in_progress_count": receipt_counts["in_progress"],
        "blocked_count": receipt_counts["blocked"],
        "passed_count": receipt_counts["passed"],
        "failed_count": receipt_counts["failed"],
        "all_required_evidence_passed": receipt_counts["passed"] == len(receipt_rows),
    }
    if receipts.get("summary") != expected_receipt_summary:
        fail("EVIDENCE_RECEIPT_SUMMARY", "candidate receipt summary differs")

    task_rows = task_ledger.get("tasks")
    phase_rows = task_ledger.get("phases")
    wave_rows = task_ledger.get("wave_receipts")
    if (
        not isinstance(task_rows, list)
        or len(task_rows) != EXPECTED_TASK_COUNT
        or not isinstance(phase_rows, list)
        or not isinstance(wave_rows, list)
    ):
        fail("TASK_COUNT", "candidate task ledger has the wrong task/phase shape")
    task_ids = [task.get("id") for task in task_rows if isinstance(task, dict)]
    if len(task_ids) != len(task_rows) or len(task_ids) != len(set(task_ids)):
        fail("TASK_COUNT", "candidate task identifiers are invalid")
    tasks_by_id = {task["id"]: task for task in task_rows}
    required_task_requirement_fields = {
        "completion_rule",
        "current_head",
        "dependencies",
        "execution_wave",
        "head_mismatch_rule",
        "id",
        "mandatory_adversarial_questions",
        "mandatory_path_scope",
        "phase_id",
        "phase_title",
        "preconditions",
        "priority",
        "procedure",
        "required_evidence",
        "required_tests",
        "source_block",
        "subagent_lane",
        "title",
    }
    task_allowed = set(task_ledger.get("status_contract", {}).get("task_statuses", []))
    lens_allowed = set(task_ledger.get("status_contract", {}).get("lens_statuses", []))
    expected_statuses = {"open", "in_progress", "blocked", "closed", "claim_removed"}
    task_contract = task_ledger.get("status_contract", {})
    if (
        task_allowed != expected_statuses
        or lens_allowed != expected_statuses
        or task_contract.get("terminal_promotion_enabled") is not False
        or task_contract.get("terminal_promotion_policy") != TERMINAL_PROMOTION_POLICY
    ):
        fail("TASK_STATUS_CONTRACT", "candidate task/lens status contract differs")
    lens_rows: list[Mapping[str, Any]] = []
    for task in task_rows:
        baseline_requirements = task.get("baseline_requirements")
        if (
            not isinstance(baseline_requirements, dict)
            or set(baseline_requirements) != required_task_requirement_fields
            or baseline_requirements.get("id") != task.get("id")
            or baseline_requirements.get("source_block", {}).get("sha256")
            != task.get("baseline_source_block_sha256")
        ):
            fail(
                "TASK_REQUIREMENTS",
                f"task-specific closure requirements are incomplete: {task.get('id')}",
            )
        status_value = task.get("status")
        if status_value not in task_allowed:
            fail("TASK_STATUS", f"unsupported task status: {task.get('id')}")
        if status_value in {"closed", "claim_removed"}:
            fail(
                "TERMINAL_PROMOTION_DISABLED",
                f"schema 0.1 contains a final task: {task.get('id')}",
            )
        reviewer = task.get("reviewer")
        independent = task.get("independent_reviewer")
        if reviewer is not None and reviewer == independent:
            fail("TASK_REVIEW", f"task reviewers are not independent: {task.get('id')}")
        if status_value != "open" and (
            not task.get("decision")
            or not reviewer
            or not task.get("updated_at")
            or not task.get("evidence")
        ):
            code = (
                "FALSE_TASK_CLOSURE"
                if status_value in {"closed", "claim_removed"}
                else "TASK_PROMOTION"
            )
            fail(code, f"task promotion lacks prerequisites: {task.get('id')}")
        if status_value == "closed" and (
            task.get("decision") != "ACCEPTED"
            or not task.get("owner")
            or not independent
            or not task.get("evidence_receipt_ids")
            or any(
                receipt_status.get(receipt_id) != "passed"
                for receipt_id in task.get("evidence_receipt_ids", [])
            )
        ):
            fail(
                "FALSE_TASK_CLOSURE",
                f"closed task lacks passed evidence: {task.get('id')}",
            )
        if status_value == "claim_removed" and (
            task.get("decision") != "CLAIM_REMOVED"
            or not independent
            or not task.get("claim_impact")
        ):
            fail("FALSE_TASK_CLOSURE", f"removed task lacks review: {task.get('id')}")
        if status_value == "in_progress" and (
            task.get("decision") != "WORK_STARTED" or not task.get("owner")
        ):
            fail(
                "TASK_PROMOTION", f"in-progress task is inconsistent: {task.get('id')}"
            )
        if status_value == "blocked" and (
            task.get("decision") != "BLOCKED" or not task.get("blockers")
        ):
            fail("TASK_PROMOTION", f"blocked task is inconsistent: {task.get('id')}")
        lenses = task.get("lenses")
        if not isinstance(lenses, list):
            fail("LENS_COUNT", f"task lenses have the wrong type: {task.get('id')}")
        lens_rows.extend(lenses)
        if any(
            not isinstance(lens.get("baseline_requirement"), dict)
            or set(lens["baseline_requirement"])
            != {"evidence", "finding", "id", "name", "question", "status"}
            or lens["baseline_requirement"].get("id") != lens.get("lens_id")
            for lens in lenses
        ):
            fail("TASK_REQUIREMENTS", f"lens requirements differ: {task.get('id')}")
        lens_ids = [lens.get("lens_id") for lens in lenses if isinstance(lens, dict)]
        if len(lens_ids) != len(lenses) or len(lens_ids) != len(set(lens_ids)):
            fail("LENS_COUNT", f"task lens identifiers differ: {task.get('id')}")
    if len(lens_rows) != EXPECTED_LENS_DISPOSITION_COUNT:
        fail("LENS_COUNT", "candidate lens ledger has the wrong disposition count")
    for lens in lens_rows:
        status_value = lens.get("status")
        if status_value not in lens_allowed:
            fail("LENS_STATUS", f"unsupported lens status: {lens.get('lens_id')}")
        if status_value in {"closed", "claim_removed"}:
            fail(
                "TERMINAL_PROMOTION_DISABLED",
                f"schema 0.1 contains a final lens: {lens.get('lens_id')}",
            )
        reviewer = lens.get("reviewer")
        independent = lens.get("independent_reviewer")
        if reviewer is not None and reviewer == independent:
            fail(
                "LENS_REVIEW",
                f"lens reviewers are not independent: {lens.get('lens_id')}",
            )
        if status_value != "open" and (
            not lens.get("decision")
            or not reviewer
            or not lens.get("updated_at")
            or not lens.get("finding")
            or not lens.get("evidence")
        ):
            code = (
                "FALSE_LENS_CLOSURE"
                if status_value in {"closed", "claim_removed"}
                else "LENS_PROMOTION"
            )
            fail(code, f"lens promotion lacks prerequisites: {lens.get('lens_id')}")
        if status_value == "closed" and (
            lens.get("decision") != "ACCEPTED"
            or not independent
            or not lens.get("evidence_receipt_ids")
            or any(
                receipt_status.get(receipt_id) != "passed"
                for receipt_id in lens.get("evidence_receipt_ids", [])
            )
        ):
            fail(
                "FALSE_LENS_CLOSURE",
                f"closed lens lacks passed evidence: {lens.get('lens_id')}",
            )
        if status_value == "claim_removed" and (
            lens.get("decision") != "CLAIM_REMOVED" or not independent
        ):
            fail(
                "FALSE_LENS_CLOSURE",
                f"removed lens lacks review: {lens.get('lens_id')}",
            )
        if status_value == "in_progress" and lens.get("decision") != "WORK_STARTED":
            fail(
                "LENS_PROMOTION",
                f"in-progress lens is inconsistent: {lens.get('lens_id')}",
            )
        if status_value == "blocked" and (
            lens.get("decision") != "BLOCKED" or not lens.get("blockers")
        ):
            fail(
                "LENS_PROMOTION", f"blocked lens is inconsistent: {lens.get('lens_id')}"
            )

    phase_ids = [phase.get("id") for phase in phase_rows if isinstance(phase, dict)]
    wave_ids = [wave.get("wave_id") for wave in wave_rows if isinstance(wave, dict)]
    if (
        len(phase_ids) != len(phase_rows)
        or len(phase_ids) != len(set(phase_ids))
        or len(wave_ids) != len(wave_rows)
        or len(wave_ids) != len(set(wave_ids))
    ):
        fail("WAVE_SUMMARY", "candidate phase/wave identifiers differ")
    waves_by_id = {wave["wave_id"]: wave for wave in wave_rows}
    for wave in wave_rows:
        wave_id = wave["wave_id"]
        phase = next((item for item in phase_rows if item["id"] == wave_id), None)
        decision_value = wave.get("decision")
        if decision_value in {"WAVE_ACCEPTED", "CLAIM_REMOVED"}:
            fail(
                "TERMINAL_PROMOTION_DISABLED",
                f"schema 0.1 contains a final wave: {wave_id}",
            )
        reviewer = wave.get("reviewer")
        independent = wave.get("independent_reviewer")
        if (
            phase is None
            or wave.get("task_ids") != phase.get("task_ids")
            or reviewer is None
            or independent is None
            or reviewer == independent
        ):
            fail("FALSE_WAVE_CLOSURE", f"wave receipt is invalid: {wave_id}")
        members = [tasks_by_id.get(task_id) for task_id in wave["task_ids"]]
        if any(member is None for member in members):
            fail("FALSE_WAVE_CLOSURE", f"wave contains an unknown task: {wave_id}")
        if decision_value == "WAVE_ACCEPTED" and (
            not wave.get("evidence_receipt_ids")
            or any(
                receipt_status.get(receipt_id) != "passed"
                for receipt_id in wave.get("evidence_receipt_ids", [])
            )
            or not all(
                member["status"] in {"closed", "claim_removed"}
                and all(
                    lens["status"] in {"closed", "claim_removed"}
                    for lens in member["lenses"]
                )
                for member in members
            )
        ):
            fail("FALSE_WAVE_CLOSURE", f"accepted wave is unfinished: {wave_id}")
        if decision_value == "CLAIM_REMOVED" and not all(
            member["status"] == "claim_removed"
            and all(lens["status"] == "claim_removed" for lens in member["lenses"])
            for member in members
        ):
            fail("FALSE_WAVE_CLOSURE", f"removed wave retains work: {wave_id}")
        if decision_value not in {"WAVE_ACCEPTED", "WAVE_REWORK", "CLAIM_REMOVED"}:
            fail("FALSE_WAVE_CLOSURE", f"wave decision is unsupported: {wave_id}")
    for phase in phase_rows:
        if not isinstance(phase.get("task_ids"), list) or any(
            task_id not in tasks_by_id for task_id in phase["task_ids"]
        ):
            fail("WAVE_SUMMARY", f"phase task coverage differs: {phase.get('id')}")
        statuses = [tasks_by_id[task_id]["status"] for task_id in phase["task_ids"]]
        wave = waves_by_id.get(phase["id"])
        if wave is not None and wave["decision"] == "CLAIM_REMOVED":
            expected_phase_status = "claim_removed"
        elif wave is not None and wave["decision"] == "WAVE_ACCEPTED":
            expected_phase_status = "closed"
        elif "blocked" in statuses:
            expected_phase_status = "blocked"
        elif any(status != "open" for status in statuses):
            expected_phase_status = "in_progress"
        else:
            expected_phase_status = "open"
        if phase.get("status") != expected_phase_status:
            fail("WAVE_SUMMARY", f"phase status differs: {phase.get('id')}")

    task_counts = Counter(task["status"] for task in task_rows)
    lens_counts = Counter(lens["status"] for lens in lens_rows)
    final_statuses = {"closed", "claim_removed"}
    all_tasks_final = all(
        task["status"] in final_statuses
        and all(lens["status"] in final_statuses for lens in task["lenses"])
        for task in task_rows
    )
    expected_task_summary = {
        "phase_count": len(phase_rows),
        "task_count": len(task_rows),
        "open_task_count": task_counts["open"],
        "in_progress_task_count": task_counts["in_progress"],
        "blocked_task_count": task_counts["blocked"],
        "closed_task_count": task_counts["closed"],
        "claim_removed_task_count": task_counts["claim_removed"],
        "lens_disposition_count": len(lens_rows),
        "open_lens_disposition_count": lens_counts["open"],
        "in_progress_lens_disposition_count": lens_counts["in_progress"],
        "blocked_lens_disposition_count": lens_counts["blocked"],
        "closed_lens_disposition_count": lens_counts["closed"],
        "claim_removed_lens_disposition_count": lens_counts["claim_removed"],
        "wave_receipt_count": len(wave_rows),
        "wave_accepted_count": sum(
            wave["decision"] == "WAVE_ACCEPTED" for wave in wave_rows
        ),
        "wave_rework_count": sum(
            wave["decision"] == "WAVE_REWORK" for wave in wave_rows
        ),
        "claim_removed_wave_count": sum(
            wave["decision"] == "CLAIM_REMOVED" for wave in wave_rows
        ),
        "all_tasks_closed_or_claim_removed": all_tasks_final,
        "release_gate_passed": False,
    }
    if task_ledger.get("summary") != expected_task_summary:
        fail("TASK_SUMMARY", "candidate task/lens summary differs from dispositions")

    claim_rows = claims.get("claims")
    if not isinstance(claim_rows, list):
        fail("CLAIM_TEMPLATE", "candidate claims must be a list")
    claim_fields = set(claims.get("claim_template_contract", {}).get("fields", []))
    required_claim_fields = {
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
    }
    if claim_fields != required_claim_fields:
        fail("CLAIM_TEMPLATE", "claim-to-evidence template fields are incomplete")
    claim_contract = claims.get("status_contract", {})
    if (
        claim_contract.get("terminal_promotion_enabled") is not False
        or claim_contract.get("terminal_promotion_policy") != TERMINAL_PROMOTION_POLICY
    ):
        fail("CLAIM_TEMPLATE", "candidate claim terminal policy differs")
    claim_ids = [
        claim.get("claim_id") for claim in claim_rows if isinstance(claim, dict)
    ]
    if len(claim_ids) != len(claim_rows) or len(claim_ids) != len(set(claim_ids)):
        fail("CLAIM_TEMPLATE", "candidate claim identifiers differ")
    for claim in claim_rows:
        if not required_claim_fields.issubset(claim):
            fail(
                "CLAIM_TEMPLATE",
                f"claim fields are incomplete: {claim.get('claim_id')}",
            )
        required_receipts = claim.get("required_evidence_receipt_ids")
        supplemental_receipts = claim.get("supplemental_evidence_receipt_ids")
        effective_receipts = claim.get("evidence_receipt_ids")
        if (
            not isinstance(required_receipts, list)
            or not isinstance(supplemental_receipts, list)
            or not isinstance(effective_receipts, list)
            or set(effective_receipts)
            != set(required_receipts) | set(supplemental_receipts)
            or any(
                receipt_id not in receipt_status for receipt_id in effective_receipts
            )
        ):
            fail("CLAIM_TEMPLATE", "claim receipt obligations are incomplete")
        if claim["claim_class"] == "software":
            status_value = claim["status"]
            if status_value not in {
                "source_evidenced_verification_pending",
                "verified_for_exact_candidate",
                "withdrawn",
            }:
                fail("SOFTWARE_CLAIM_STATUS", "software claim status is unsupported")
            if status_value in {"verified_for_exact_candidate", "withdrawn"}:
                fail(
                    "TERMINAL_PROMOTION_DISABLED",
                    f"schema 0.1 contains a final software claim: {claim['claim_id']}",
                )
            if status_value == "verified_for_exact_candidate" and (
                claim["decision"] != "VERIFIED_FOR_EXACT_CANDIDATE"
                or "RCP-POST-PUSH-CI" not in claim["evidence_receipt_ids"]
                or not claim.get("independent_reviewer")
                or any(
                    receipt_status.get(receipt_id) != "passed"
                    for receipt_id in claim["evidence_receipt_ids"]
                )
            ):
                fail(
                    "SOFTWARE_CLAIM_VERIFICATION",
                    "verified claim lacks passed exact CI",
                )
            if status_value == "withdrawn" and (
                claim.get("decision") != "CLAIM_REMOVED"
                or not claim.get("independent_reviewer")
            ):
                fail("SOFTWARE_CLAIM_STATUS", "withdrawn software claim lacks review")
        elif claim["claim_class"] == "scientific":
            if claim["status"] == "blocked_not_established":
                if claim.get("decision") != "NOT_CLAIMED":
                    fail("FALSE_SCIENTIFIC_CLAIM", "scientific claim is promoted")
            elif claim["status"] == "withdrawn":
                fail(
                    "TERMINAL_PROMOTION_DISABLED",
                    f"schema 0.1 contains a withdrawn scientific claim: {claim['claim_id']}",
                )
                if claim.get("decision") != "CLAIM_REMOVED" or not claim.get(
                    "independent_reviewer"
                ):
                    fail(
                        "FALSE_SCIENTIFIC_CLAIM",
                        "scientific claim removal lacks review",
                    )
            else:
                fail("FALSE_SCIENTIFIC_CLAIM", "scientific claim is promoted")
        else:
            fail("CLAIM_TEMPLATE", "candidate claim class is unsupported")
        reviewer = claim.get("reviewer")
        independent = claim.get("independent_reviewer")
        if reviewer is not None and reviewer == independent:
            fail(
                "CLAIM_REVIEW",
                f"claim reviewers are not independent: {claim['claim_id']}",
            )
    software_claims = [
        claim for claim in claim_rows if claim["claim_class"] == "software"
    ]
    scientific_claims = [
        claim for claim in claim_rows if claim["claim_class"] == "scientific"
    ]
    expected_claim_summary = {
        "software_claim_count": len(software_claims),
        "software_verified_count": sum(
            claim["status"] == "verified_for_exact_candidate"
            for claim in software_claims
        ),
        "software_verification_pending_count": sum(
            claim["status"] == "source_evidenced_verification_pending"
            for claim in software_claims
        ),
        "scientific_claim_count": len(scientific_claims),
        "scientific_established_count": 0,
        "scientific_blocked_count": sum(
            claim["status"] == "blocked_not_established" for claim in scientific_claims
        ),
        "withdrawn_count": sum(claim["status"] == "withdrawn" for claim in claim_rows),
    }
    if claims.get("summary") != expected_claim_summary:
        fail("CLAIM_SUMMARY", "candidate claim summary differs")

    defect_rows = defects.get("defects")
    if not isinstance(defect_rows, list):
        fail("DEFECT_STATUS", "candidate defects must be a list")
    defect_contract = defects.get("status_contract", {})
    if (
        defect_contract.get("terminal_promotion_enabled") is not False
        or defect_contract.get("terminal_promotion_policy") != TERMINAL_PROMOTION_POLICY
    ):
        fail("DEFECT_STATUS", "candidate defect terminal policy differs")
    priorities = Counter(defect.get("priority") for defect in defect_rows)
    if set(priorities) != {"P0", "P1", "P2"}:
        fail("DEFECT_PRIORITIES", "candidate defect register lacks P0/P1/P2 records")
    for defect in defect_rows:
        if defect.get("status") not in {
            "open",
            "in_progress",
            "blocked",
            "mitigated",
            "closed",
        }:
            fail("DEFECT_STATUS", "candidate defect status is unsupported")
        if defect.get("status") in {"mitigated", "closed"}:
            fail(
                "TERMINAL_PROMOTION_DISABLED",
                f"schema 0.1 contains a final defect: {defect.get('id')}",
            )
        required_receipts = defect.get("required_evidence_receipt_ids")
        supplemental_receipts = defect.get("supplemental_evidence_receipt_ids")
        effective_receipts = defect.get("evidence_receipt_ids")
        if (
            not isinstance(required_receipts, list)
            or not isinstance(supplemental_receipts, list)
            or not isinstance(effective_receipts, list)
            or set(effective_receipts)
            != set(required_receipts) | set(supplemental_receipts)
            or any(
                receipt_id not in receipt_status for receipt_id in effective_receipts
            )
        ):
            fail("DEFECT_STATUS", "defect receipt obligations are incomplete")
        reviewer = defect.get("reviewer")
        independent = defect.get("independent_reviewer")
        if reviewer is not None and reviewer == independent:
            fail(
                "DEFECT_REVIEW",
                f"defect reviewers are not independent: {defect.get('id')}",
            )
        if defect.get("status") in {"mitigated", "closed"} and (
            not independent
            or not defect.get("evidence_receipt_ids")
            or any(
                receipt_status.get(receipt_id) != "passed"
                for receipt_id in defect.get("evidence_receipt_ids", [])
            )
        ):
            fail(
                "FALSE_DEFECT_CLOSURE", "final defect lacks passed independent evidence"
            )
    defect_statuses = ["open", "in_progress", "blocked", "mitigated", "closed"]
    expected_defect_summary: dict[str, Any] = {"defect_count": len(defect_rows)}
    for priority in ("P0", "P1", "P2"):
        counts = Counter(
            defect["status"] for defect in defect_rows if defect["priority"] == priority
        )
        expected_defect_summary[priority] = {
            status_value: counts[status_value] for status_value in defect_statuses
        }
    expected_defect_summary["open_release_blocker_count"] = sum(
        defect.get("blocks_release") is True for defect in defect_rows
    )
    expected_defect_summary["release_blocked"] = (
        expected_defect_summary["open_release_blocker_count"] > 0
    )
    if defects.get("summary") != expected_defect_summary:
        fail("DEFECT_SUMMARY", "candidate defect summary differs")

    template_keys = {
        "manifest_schema",
        "project",
        "release_version",
        "decision",
        "terminal_promotion",
        "source",
        "submodules",
        "toolchains",
        "packages",
        "schemas",
        "protocol_status",
        "claims",
        "datasets",
        "models",
        "holdout",
        "evidence",
        "security",
        "independent_reviews",
        "cross_repository_qualification",
        "removed_claims",
        "residual_risks",
        "signatures",
    }
    if not template_keys.issubset(draft):
        fail("DRAFT_TEMPLATE", "draft release manifest omits handoff template fields")
    release = draft.get("release", {})
    decision = draft.get("decision_detail", {})
    post_push = next(
        (receipt for receipt in receipt_rows if receipt["id"] == "RCP-POST-PUSH-CI"),
        None,
    )
    if post_push is None:
        fail("EVIDENCE_RECEIPT_STATUS", "post-push receipt is absent")
    exact_pushed_commit = (
        post_push["execution"]["commit"] if post_push["status"] == "passed" else None
    )
    all_waves_final = expected_task_summary[
        "wave_accepted_count"
    ] + expected_task_summary["claim_removed_wave_count"] == len(phase_rows)
    all_file_reviews_final = all(
        entry["file_review"]["disposition"]
        in {"ACCEPT", "FIXED", "REMOVED", "NOT_CLAIMED"}
        for entry in inventory["entries"]
    )
    software_claims_final = all(
        claim["status"] in {"verified_for_exact_candidate", "withdrawn"}
        for claim in software_claims
    )
    defects_resolved = expected_defect_summary["release_blocked"] is False
    evidence_complete = expected_receipt_summary["all_required_evidence_passed"]
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
    expected_decision = "NO_GO"
    expected_decision_status = "NO_GO_SCHEMA_0_1_NON_PROMOTABLE"
    expected_release_status = "draft_unpublished_not_release_ready"
    expected_candidate_gates = {
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
        "exact_test_receipts_complete": "closed" if evidence_complete else "open",
        "post_push_main_ci_success": (
            "closed" if post_push["status"] == "passed" else "open"
        ),
        "publication_authorized": "open",
    }
    if (
        draft.get("decision") != expected_decision
        or draft.get("terminal_promotion")
        != {
            "enabled": False,
            "policy": TERMINAL_PROMOTION_POLICY,
            "readiness_prerequisites_satisfied": readiness_prerequisites_satisfied,
            "successor_schema_required": True,
        }
        or draft.get("source", {}).get("tree_clean") is not inventory["source"]["clean"]
        or release.get("published") is not False
        or release.get("status") != expected_release_status
        or release.get("tag") is not None
        or release.get("release_url") is not None
        or release.get("doi") is not None
        or release.get("zenodo_record") is not None
        or release.get("one_point_zero_convergence_claimed") is not False
        or decision.get("status") != expected_decision_status
        or decision.get("release_ready") is not False
        or decision.get("scientific_claims_established") is not False
        or decision.get("publication_ready") is not False
        or draft.get("source_candidate", {}).get("exact_pushed_commit")
        != exact_pushed_commit
        or draft.get("candidate_gates") != expected_candidate_gates
        or any(
            package.get("published") is not False
            for package in draft.get("packages", [])
        )
    ):
        fail("FALSE_PUBLICATION", "draft manifest claims completion or publication")
    pending_software_risk = (
        "retained software claims remain pending exact-candidate verification"
    )
    if (
        pending_software_risk in draft.get("residual_risks", [])
    ) is software_claims_final:
        fail("DRAFT_RESIDUAL_RISK", "software-claim residual-risk summary differs")
    terminal_policy_risk = (
        "candidate schema 0.1 cannot authenticate terminal evidence or represent "
        "typed task-specific closure"
    )
    if terminal_policy_risk not in draft.get("residual_risks", []):
        fail("DRAFT_RESIDUAL_RISK", "terminal-policy residual risk is absent")
    expected_protocol_status = {
        "M0": "UNFROZEN",
        "EC1": "NOT_CLAIMED",
        "H1_A": "BLOCKED",
        "H1_B": "BLOCKED",
        "H2": "BLOCKED",
        "H3": "BLOCKED",
        "H4": "BLOCKED_EXPLORATORY_ONLY",
    }
    if draft.get("protocol_status") != expected_protocol_status:
        fail("FALSE_PROTOCOL_GATE", "draft manifest promotes a protocol gate")
    expected_component_hashes = {
        INVENTORY_NAME: semantic_sha256(inventory),
        TASK_LEDGER_NAME: semantic_sha256(task_ledger),
        CLAIM_LEDGER_NAME: semantic_sha256(claims),
        DEFECT_REGISTER_NAME: semantic_sha256(defects),
        RECEIPTS_NAME: semantic_sha256(receipts),
    }
    if draft.get("component_semantic_sha256") != expected_component_hashes:
        fail("COMPONENT_BINDING", "draft component semantic bindings differ")


def audit(
    repo: Path, candidate_dir: Path, *, validate_live_source: bool = True
) -> dict[str, Any]:
    _require_candidate_directory(candidate_dir)
    documents: dict[str, dict[str, Any]] = {}
    raw_by_name: dict[str, bytes] = {}
    for name in sorted(EXPECTED_OUTPUT_NAMES):
        document, raw = _read_json(candidate_dir / name)
        documents[name] = document
        raw_by_name[name] = raw
    manifest = documents[ARTIFACT_MANIFEST_NAME]
    _validate_artifact_manifest(manifest, raw_by_name)
    inventory = documents[INVENTORY_NAME]
    _validate_inventory_internal(repo, inventory)
    expected = build_artifacts_from_inventory(repo, inventory)
    _validate_semantic_boundaries(documents)
    for name in EXPECTED_OUTPUT_NAMES:
        if raw_by_name[name] != expected[name]:
            fail(
                "GENERATED_DRIFT",
                f"candidate artifact differs from deterministic source: {name}",
            )
    expected_manifest = build_artifact_manifest(
        {name: expected[name] for name in ARTIFACT_NAMES}, inventory
    )
    if manifest != expected_manifest:
        fail(
            "MANIFEST_SEMANTICS",
            "candidate artifact manifest differs from generated form",
        )
    draft = documents[DRAFT_MANIFEST_NAME]
    if manifest["release_ready"] is not draft["decision_detail"]["release_ready"]:
        fail("MANIFEST_READINESS", "artifact and draft readiness differ")
    source_match = (
        _validate_live_coverage(repo, inventory)
        if validate_live_source
        else "internal_snapshot_only"
    )
    task_summary = documents[TASK_LEDGER_NAME]["summary"]
    return {
        "status": "pass",
        "release_version": RELEASE_VERSION,
        "candidate_state_sha256": inventory["source"]["candidate_state_sha256"],
        "source_match": source_match,
        "inventory_entry_count": inventory["summary"]["entry_count"],
        "open_task_count": task_summary["open_task_count"],
        "in_progress_task_count": task_summary["in_progress_task_count"],
        "closed_task_count": task_summary["closed_task_count"],
        "open_lens_disposition_count": task_summary["open_lens_disposition_count"],
        "in_progress_lens_disposition_count": task_summary[
            "in_progress_lens_disposition_count"
        ],
        "pending_receipt_count": documents[RECEIPTS_NAME]["summary"]["pending_count"],
        "release_ready": draft["decision_detail"]["release_ready"],
        "published": False,
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path("."))
    parser.add_argument("--candidate-dir", type=Path, default=Path(CANDIDATE_RELATIVE))
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        repo = resolve_repo(args.repo)
        candidate_dir = args.candidate_dir
        if not candidate_dir.is_absolute():
            candidate_dir = repo / candidate_dir
        result = audit(repo, candidate_dir)
    except CandidateError as exc:
        print(f"candidate release audit failed [{exc.code}]: {exc}", file=sys.stderr)
        return 3
    print(json.dumps(result, ensure_ascii=False, allow_nan=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
