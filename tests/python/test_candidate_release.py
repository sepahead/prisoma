from __future__ import annotations

import copy
import hashlib
import json
import os
import runpy
import shutil
import subprocess
import sys
import time
from collections.abc import Callable
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parents[2]
CANDIDATE_DIR = ROOT / "release" / "0.9.0" / "candidate"
GENERATOR = ROOT / "scripts" / "generate_candidate_release.py"
AUDITOR = ROOT / "scripts" / "audit_candidate_release.py"
ARTIFACT_MANIFEST = "artifact_manifest.json"
CANDIDATE_RELATIVE = "release/0.9.0/candidate"
EXPECTED_NAMES = {
    ARTIFACT_MANIFEST,
    "source_inventory.json",
    "task_lens_ledger.json",
    "claim_evidence_ledger.json",
    "defect_register.json",
    "evidence_receipts.json",
    "draft_release_manifest.json",
}


def _read(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_bytes())
    assert isinstance(value, dict)
    return value


def _load_generator() -> dict[str, Any]:
    return runpy.run_path(os.fspath(GENERATOR))


def _load_auditor() -> dict[str, Any]:
    scripts = os.fspath(ROOT / "scripts")
    sys.path.insert(0, scripts)
    try:
        return runpy.run_path(os.fspath(AUDITOR))
    finally:
        sys.path.remove(scripts)


def _write_artifact_set(directory: Path, artifacts: dict[str, bytes]) -> None:
    directory.mkdir()
    for name, raw in artifacts.items():
        (directory / name).write_bytes(raw)


def _write(path: Path, value: dict[str, Any]) -> None:
    path.write_text(
        json.dumps(value, ensure_ascii=False, allow_nan=False, indent=2, sort_keys=True)
        + "\n",
        encoding="utf-8",
    )


def _run_audit(candidate_dir: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            os.fspath(AUDITOR),
            "--repo",
            os.fspath(ROOT),
            "--candidate-dir",
            os.fspath(candidate_dir),
        ],
        check=False,
        capture_output=True,
        text=True,
    )


def _copy_candidate(tmp_path: Path) -> Path:
    destination = tmp_path / "candidate"
    shutil.copytree(CANDIDATE_DIR, destination)
    return destination


def _rehash_manifest(candidate_dir: Path, name: str) -> None:
    raw = (candidate_dir / name).read_bytes()
    manifest_path = candidate_dir / ARTIFACT_MANIFEST
    manifest = _read(manifest_path)
    record = next(item for item in manifest["artifacts"] if item["path"] == name)
    record["sha256"] = hashlib.sha256(raw).hexdigest()
    record["bytes"] = len(raw)
    _write(manifest_path, manifest)


def _mutate(
    candidate_dir: Path,
    name: str,
    mutation: Callable[[dict[str, Any]], None],
) -> None:
    path = candidate_dir / name
    value = _read(path)
    mutation(value)
    _write(path, value)
    _rehash_manifest(candidate_dir, name)


def _assert_rejected(candidate_dir: Path, code: str | None = None) -> None:
    result = _run_audit(candidate_dir)
    assert result.returncode == 3, (result.stdout, result.stderr)
    assert result.stdout == ""
    assert "candidate release audit failed" in result.stderr
    if code is not None:
        assert f"[{code}]" in result.stderr
    assert "Traceback" not in result.stderr


def _git_paths() -> set[str]:
    head = subprocess.check_output(
        ["git", "-C", os.fspath(ROOT), "ls-tree", "-r", "-z", "--name-only", "HEAD"]
    )
    indexed = subprocess.check_output(["git", "-C", os.fspath(ROOT), "ls-files", "-z"])
    untracked = subprocess.check_output(
        [
            "git",
            "-C",
            os.fspath(ROOT),
            "ls-files",
            "--others",
            "--exclude-standard",
            "-z",
        ]
    )
    paths = {
        raw.decode("utf-8")
        for raw in head.rstrip(b"\0").split(b"\0")
        + indexed.rstrip(b"\0").split(b"\0")
        + untracked.rstrip(b"\0").split(b"\0")
        if raw
    }
    return {
        path
        for path in paths
        if path != CANDIDATE_RELATIVE and not path.startswith(f"{CANDIDATE_RELATIVE}/")
    }


def _pinned_pid_rs_paths() -> set[str]:
    raw_index = subprocess.check_output(
        ["git", "-C", os.fspath(ROOT), "ls-files", "--stage", "--", "pid-rs"]
    )
    metadata, path = raw_index.rstrip(b"\n").split(b"\t", 1)
    mode, commit, stage = metadata.decode("ascii").split(" ")
    assert (mode, stage, path) == ("160000", "0", b"pid-rs")
    raw_tree = subprocess.check_output(
        [
            "git",
            "-C",
            os.fspath(ROOT / "pid-rs"),
            "ls-tree",
            "-r",
            "-z",
            "--name-only",
            commit,
        ]
    )
    return {
        f"pid-rs/{raw.decode('utf-8', errors='strict')}"
        for raw in raw_tree.rstrip(b"\0").split(b"\0")
        if raw
    }


def _synthetic_in_progress_artifacts() -> tuple[dict[str, Any], dict[str, bytes]]:
    generator = _load_generator()
    inventory = _read(CANDIDATE_DIR / "source_inventory.json")
    progress = copy.deepcopy(inventory["progress_snapshot"]["document"])
    initial = generator["build_artifacts_from_inventory"](ROOT, inventory)
    task_ledger = json.loads(initial["task_lens_ledger.json"])
    claims = json.loads(initial["claim_evidence_ledger.json"])
    defects = json.loads(initial["defect_register.json"])
    receipts = json.loads(initial["evidence_receipts.json"])
    task = task_ledger["tasks"][0]
    lens = task["lenses"][0]
    scientific_claim = next(
        claim for claim in claims["claims"] if claim["claim_class"] == "scientific"
    )
    timestamp = "2026-07-14T12:00:00Z"
    progress.update(
        {
            "progress_revision": 1,
            "file_review_updates": [
                {
                    "path": "README.md",
                    "disposition": "IN_PROGRESS",
                    "reviewer": "Primary Reviewer",
                    "independent_reviewer": None,
                    "updated_at": timestamp,
                    "requirements": [],
                    "defects": [],
                    "tests": [],
                    "evidence_paths": ["README.md"],
                    "notes": "Review is underway; no final disposition is recorded.",
                }
            ],
            "task_updates": [
                {
                    "task_id": task["id"],
                    "status": "in_progress",
                    "decision": "WORK_STARTED",
                    "owner": "Task Owner",
                    "reviewer": "Primary Reviewer",
                    "independent_reviewer": None,
                    "updated_at": timestamp,
                    "evidence_paths": ["README.md"],
                    "evidence_receipt_ids": [],
                    "blockers": [],
                    "claim_impact": "No claim promotion is made.",
                }
            ],
            "lens_updates": [
                {
                    "task_id": task["id"],
                    "lens_id": lens["lens_id"],
                    "status": "in_progress",
                    "decision": "WORK_STARTED",
                    "reviewer": "Primary Reviewer",
                    "independent_reviewer": None,
                    "updated_at": timestamp,
                    "finding": "Review is underway and remains nonfinal.",
                    "evidence_paths": ["README.md"],
                    "evidence_receipt_ids": [],
                    "blockers": [],
                }
            ],
            "claim_updates": [
                {
                    "claim_id": scientific_claim["claim_id"],
                    "status": "blocked_not_established",
                    "decision": "NOT_CLAIMED",
                    "reviewer": "Primary Reviewer",
                    "independent_reviewer": "Independent Reviewer",
                    "updated_at": timestamp,
                    "evidence_paths": ["README.md"],
                    "evidence_receipt_ids": [],
                    "residual_assumptions": [
                        "The scientific gate remains blocked and unestablished."
                    ],
                }
            ],
            "defect_updates": [
                {
                    "defect_id": defects["defects"][0]["id"],
                    "status": "in_progress",
                    "decision": "WORK_STARTED",
                    "reviewer": "Primary Reviewer",
                    "independent_reviewer": None,
                    "updated_at": timestamp,
                    "evidence_paths": ["README.md"],
                    "evidence_receipt_ids": [],
                    "residual_risk": "The release-blocking condition remains unresolved.",
                }
            ],
            "evidence_receipt_updates": [
                {
                    "receipt_id": receipts["receipts"][0]["id"],
                    "status": "in_progress",
                    "reviewer": "Primary Reviewer",
                    "independent_reviewer": None,
                    "updated_at": timestamp,
                    "execution": {
                        "commit": None,
                        "started_at": timestamp,
                        "completed_at": None,
                        "runner": None,
                        "workflow_run_url": None,
                        "exit_codes": [],
                    },
                    "evidence_path": None,
                    "conclusion": "running",
                }
            ],
        }
    )
    synthetic_inventory = generator["inventory_with_progress"](inventory, progress)
    artifacts = generator["build_artifacts_from_inventory"](ROOT, synthetic_inventory)
    return generator, artifacts


def _passed_rerun_update(
    inventory: dict[str, Any], evidence_path: str, *, commit: str | None = None
) -> dict[str, Any]:
    return {
        "receipt_id": "RCP-RERUN",
        "status": "passed",
        "reviewer": "Primary Reviewer",
        "independent_reviewer": "Independent Reviewer",
        "updated_at": "2026-07-14T12:02:00Z",
        "execution": {
            "commit": commit or inventory["source"]["head_commit"],
            "started_at": "2026-07-14T12:00:00Z",
            "completed_at": "2026-07-14T12:01:00Z",
            "runner": "github-actions",
            "workflow_run_url": ("https://github.com/sepahead/prisoma/actions/runs/1"),
            "exit_codes": [0, 0],
        },
        "evidence_path": evidence_path,
        "conclusion": "success",
    }


def _failed_rerun_update(
    inventory: dict[str, Any], evidence_path: str, *, commit: str | None = None
) -> dict[str, Any]:
    update = _passed_rerun_update(inventory, evidence_path, commit=commit)
    update["status"] = "failed"
    update["execution"]["exit_codes"] = [0, 1]
    update["conclusion"] = "failure"
    return update


def _inventory_with_synthetic_evidence_log(
    inventory: dict[str, Any], path: str
) -> dict[str, Any]:
    updated = copy.deepcopy(inventory)
    source_entry = next(
        entry for entry in updated["entries"] if entry["path"] == "README.md"
    )
    evidence_entry = copy.deepcopy(source_entry)
    evidence_entry.update(
        {
            "path": path,
            "head": None,
            "index": None,
            "index_state": "untracked",
            "worktree_state": "untracked",
        }
    )
    evidence_entry["file_review"]["path"] = path
    updated["entries"].append(evidence_entry)
    return updated


def test_candidate_git_helper_enforces_exact_caps_and_timeout(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    generator = _load_generator()
    fake_bin = tmp_path / "bin"
    fake_bin.mkdir()
    fake_git = fake_bin / "git"
    fake_git.write_text(
        """#!/usr/bin/env python3
import os
import pathlib
import sys
import time

mode = os.environ["FAKE_GIT_MODE"]
if mode == "stdout":
    sys.stdout.buffer.write(b"abcd")
elif mode == "stderr":
    sys.stderr.buffer.write(b"abcd")
elif mode == "timeout":
    time.sleep(2)
    pathlib.Path(os.environ["FAKE_GIT_MARKER"]).write_text("escaped", encoding="utf-8")
""",
        encoding="utf-8",
    )
    fake_git.chmod(0o755)
    monkeypatch.setenv("PATH", f"{fake_bin}{os.pathsep}{os.environ['PATH']}")
    run_git = generator["_run_git"]

    monkeypatch.setenv("FAKE_GIT_MODE", "stdout")
    assert run_git(tmp_path, ["ignored"], max_bytes=4) == b"abcd"
    with pytest.raises(generator["CandidateError"]) as caught:
        run_git(tmp_path, ["ignored"], max_bytes=3)
    assert caught.value.code == "GIT_OUTPUT"

    monkeypatch.setenv("FAKE_GIT_MODE", "stderr")
    monkeypatch.setitem(run_git.__globals__, "MAX_GIT_STDERR_BYTES", 3)
    with pytest.raises(generator["CandidateError"]) as caught:
        run_git(tmp_path, ["ignored"], max_bytes=4)
    assert caught.value.code == "GIT_STDERR"

    marker = tmp_path / "escaped"
    monkeypatch.setenv("FAKE_GIT_MODE", "timeout")
    monkeypatch.setenv("FAKE_GIT_MARKER", os.fspath(marker))
    with pytest.raises(generator["CandidateError"]) as caught:
        run_git(tmp_path, ["ignored"], max_bytes=4, timeout_seconds=0.05)
    assert caught.value.code == "GIT_TIMEOUT"
    time.sleep(0.1)
    assert not marker.exists()


def test_candidate_audit_passes_but_reports_no_go_pending_state() -> None:
    result = _run_audit(CANDIDATE_DIR)
    assert result.returncode == 0, result.stderr
    payload = json.loads(result.stdout)
    assert payload["status"] == "pass"
    assert payload["release_version"] == "0.9.0"
    assert payload["source_match"] in {
        "exact_source_state",
        "committed_content_convergence",
    }
    assert payload["open_task_count"] == 240
    assert payload["open_lens_disposition_count"] == 4800
    assert payload["pending_receipt_count"] == 6
    assert payload["release_ready"] is False
    assert payload["published"] is False


def test_candidate_inventory_covers_index_and_all_nonignored_untracked_inputs() -> None:
    inventory = _read(CANDIDATE_DIR / "source_inventory.json")
    entries = inventory["entries"]
    parent_paths = {
        entry["path"]
        for entry in entries
        if entry["inventory_origin"]["kind"] == "parent_repository"
    }
    recursive_paths = {
        entry["path"]
        for entry in entries
        if entry["inventory_origin"]["kind"] == "pinned_gitlink_file"
    }
    assert parent_paths == _git_paths()
    assert recursive_paths == _pinned_pid_rs_paths()
    assert len(recursive_paths) == 148
    assert inventory["summary"]["parent_entry_count"] == len(parent_paths)
    assert inventory["summary"]["recursive_gitlink_entry_count"] == 148
    recursive_source = inventory["source"]["recursive_gitlinks"]
    assert len(recursive_source) == 1
    assert recursive_source[0]["path"] == "pid-rs"
    assert recursive_source[0]["entry_count"] == 148
    assert inventory["inventory_policy"]["recursive_gitlink_paths"] == ["pid-rs"]
    assert (
        inventory["inventory_policy"]["recursive_rows_derived_from"]
        == "pinned_index_commit_git_objects"
    )
    assert inventory["source"]["clean"] is False
    assert inventory["source"]["state"] == "dirty_uncommitted_source_snapshot"
    assert inventory["summary"]["untracked_entry_count"] > 0
    assert inventory["inventory_policy"]["self_excluded_paths"] == [CANDIDATE_RELATIVE]
    fixed_point = inventory["inventory_policy"]["fixed_point_semantics"]
    assert fixed_point["source_digest_excludes_candidate_outputs"] is True
    assert fixed_point["candidate_outputs_bound_by"] == ARTIFACT_MANIFEST
    for entry in entries:
        working = entry["working_tree"]
        if working["kind"] == "regular":
            raw = (ROOT / entry["path"]).read_bytes()
            assert working["sha256"] == hashlib.sha256(raw).hexdigest()
            assert working["bytes"] == len(raw)


def test_recursive_gitlink_omission_is_rejected_against_pinned_objects() -> None:
    generator = _load_generator()
    auditor = _load_auditor()
    inventory = generator["_capture_once"](ROOT)
    recursive = [
        entry
        for entry in inventory["entries"]
        if entry["inventory_origin"]["kind"] == "pinned_gitlink_file"
    ]
    assert len(recursive) == 148
    omitted_path = recursive[0]["path"]
    forged = copy.deepcopy(inventory)
    forged["entries"] = [
        entry for entry in forged["entries"] if entry["path"] != omitted_path
    ]
    forged["summary"]["entry_count"] -= 1
    forged["summary"]["recursive_gitlink_entry_count"] -= 1
    with pytest.raises(auditor["CandidateError"]) as caught:
        auditor["_validate_inventory_internal"](ROOT, forged)
    assert getattr(caught.value, "code", None) == "INVENTORY_GITLINK_COVERAGE"


def test_parent_head_omission_cannot_hide_behind_recursive_rows() -> None:
    generator = _load_generator()
    auditor = _load_auditor()
    inventory = generator["_capture_once"](ROOT)
    forged = copy.deepcopy(inventory)
    forged["entries"] = [
        entry for entry in forged["entries"] if entry["path"] != "README.md"
    ]
    forged["summary"]["entry_count"] -= 1
    forged["summary"]["parent_entry_count"] -= 1
    with pytest.raises(auditor["CandidateError"]) as caught:
        auditor["_validate_inventory_internal"](ROOT, forged)
    assert getattr(caught.value, "code", None) == "INVENTORY_HEAD_COVERAGE"


def test_recursive_inventory_rejects_a_dirty_gitlink(tmp_path: Path) -> None:
    generator = _load_generator()
    child = tmp_path / "child"
    child.mkdir()
    subprocess.run(["git", "init", "-q"], cwd=child, check=True)
    subprocess.run(["git", "config", "user.name", "Test Author"], cwd=child, check=True)
    subprocess.run(
        ["git", "config", "user.email", "test@example.invalid"],
        cwd=child,
        check=True,
    )
    (child / "estimator.rs").write_text("pub fn estimate() {}\n", encoding="utf-8")
    subprocess.run(["git", "add", "."], cwd=child, check=True)
    subprocess.run(["git", "commit", "-q", "-m", "fixture"], cwd=child, check=True)

    repo = tmp_path / "repo"
    progress_dir = repo / "release" / "0.9.0"
    progress_dir.mkdir(parents=True)
    shutil.copy2(
        ROOT / "release" / "0.9.0" / "candidate_progress.json",
        progress_dir / "candidate_progress.json",
    )
    subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
    subprocess.run(["git", "config", "user.name", "Test Author"], cwd=repo, check=True)
    subprocess.run(
        ["git", "config", "user.email", "test@example.invalid"],
        cwd=repo,
        check=True,
    )
    subprocess.run(
        [
            "git",
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "-q",
            os.fspath(child),
            "pid-rs",
        ],
        cwd=repo,
        check=True,
    )
    subprocess.run(["git", "add", "."], cwd=repo, check=True)
    subprocess.run(["git", "commit", "-q", "-m", "fixture"], cwd=repo, check=True)
    (repo / "pid-rs" / "estimator.rs").write_text(
        'pub fn estimate() { panic!("dirty") }\n', encoding="utf-8"
    )

    with pytest.raises(generator["CandidateError"]) as caught:
        generator["_capture_once"](repo)
    assert caught.value.code == "GITLINK_DIRTY"

    subprocess.run(["git", "restore", "estimator.rs"], cwd=repo / "pid-rs", check=True)
    subprocess.run(
        ["git", "config", "user.name", "Test Author"],
        cwd=repo / "pid-rs",
        check=True,
    )
    subprocess.run(
        ["git", "config", "user.email", "test@example.invalid"],
        cwd=repo / "pid-rs",
        check=True,
    )
    (repo / "pid-rs" / "estimator.rs").write_text(
        "pub fn estimate() { assert!(true) }\n", encoding="utf-8"
    )
    subprocess.run(["git", "add", "."], cwd=repo / "pid-rs", check=True)
    subprocess.run(
        ["git", "commit", "-q", "-m", "alternate fixture"],
        cwd=repo / "pid-rs",
        check=True,
    )
    assert (
        subprocess.check_output(["git", "status", "--porcelain"], cwd=repo / "pid-rs")
        == b""
    )
    with pytest.raises(generator["CandidateError"]) as caught:
        generator["_capture_once"](repo)
    assert caught.value.code == "GITLINK_DIRTY"


def test_internal_audit_recomputes_recorded_index_and_worktree_states() -> None:
    generator = _load_generator()
    auditor = _load_auditor()
    inventory = _read(CANDIDATE_DIR / "source_inventory.json")
    progress = inventory["progress_snapshot"]["document"]

    forged_index = copy.deepcopy(inventory)
    untracked = next(
        entry
        for entry in forged_index["entries"]
        if entry["index_state"] == "untracked"
    )
    untracked["index_state"] = "unchanged"
    forged_index = generator["inventory_with_progress"](forged_index, progress)
    with pytest.raises(Exception) as caught:
        auditor["_validate_inventory_internal"](ROOT, forged_index)
    assert getattr(caught.value, "code", None) == "INVENTORY_INDEX_STATE"

    forged_worktree = copy.deepcopy(inventory)
    modified = next(
        entry
        for entry in forged_worktree["entries"]
        if entry["worktree_state"] != "unchanged"
        and entry["path"] != "release/0.9.0/candidate_progress.json"
    )
    modified["worktree_state"] = "unchanged"
    forged_worktree = generator["inventory_with_progress"](forged_worktree, progress)
    with pytest.raises(Exception) as caught:
        auditor["_validate_inventory_internal"](ROOT, forged_worktree)
    assert getattr(caught.value, "code", None) == "INVENTORY_WORKTREE_STATE"


def test_inventory_captures_staged_deletions_from_head(tmp_path: Path) -> None:
    generator = _load_generator()
    repo = tmp_path / "repo"
    progress_dir = repo / "release" / "0.9.0"
    progress_dir.mkdir(parents=True)
    shutil.copy2(
        ROOT / "release" / "0.9.0" / "candidate_progress.json",
        progress_dir / "candidate_progress.json",
    )
    (repo / "tracked.txt").write_text("tracked\n", encoding="utf-8")
    for argv in (
        ["git", "init", "-q"],
        ["git", "config", "user.name", "Test Author"],
        ["git", "config", "user.email", "test@example.invalid"],
        ["git", "add", "."],
        ["git", "commit", "-q", "-m", "test fixture"],
        ["git", "rm", "-q", "tracked.txt"],
    ):
        subprocess.run(argv, cwd=repo, check=True)

    inventory = generator["capture_stable_inventory"](repo)
    deleted = next(
        entry for entry in inventory["entries"] if entry["path"] == "tracked.txt"
    )
    assert deleted["head"] is not None
    assert deleted["index"] is None
    assert deleted["index_state"] == "deleted"
    assert deleted["working_tree"]["kind"] == "missing"
    assert deleted["worktree_state"] == "deleted"
    assert inventory["summary"]["untracked_entry_count"] == 0
    assert inventory["summary"]["index_changed_entry_count"] == 1
    assert inventory["summary"]["clean"] is False


def test_file_review_records_preserve_every_handoff_field_without_false_review() -> (
    None
):
    inventory = _read(CANDIDATE_DIR / "source_inventory.json")
    expected_fields = inventory["file_review_template_contract"]["fields"]
    extended_fields = inventory["file_review_template_contract"]["extended_fields"]
    assert expected_fields == [
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
    for entry in inventory["entries"]:
        review = entry["file_review"]
        assert set(review) == set(expected_fields) | set(extended_fields)
        assert review["path"] == entry["path"]
        assert review["disposition"] == "OPEN_NOT_REVIEWED"
        assert review["reviewer"] is None
        assert review["independent_reviewer"] is None
        assert review["decision"] == "OPEN_NOT_REVIEWED"
        assert review["updated_at"] is None
        assert review["completed_at"] is None
        assert review["requirements"] == []
        assert review["defects"] == []
        assert review["tests"] == []
        assert review["evidence"] == []


def test_task_claim_defect_receipt_and_draft_boundaries_are_explicit() -> None:
    tasks = _read(CANDIDATE_DIR / "task_lens_ledger.json")
    assert tasks["summary"]["open_task_count"] == 240
    assert tasks["summary"]["open_lens_disposition_count"] == 4800
    assert tasks["summary"]["closed_task_count"] == 0
    assert all(task["status"] == "open" for task in tasks["tasks"])

    claim_ledger = _read(CANDIDATE_DIR / "claim_evidence_ledger.json")
    template_fields = set(claim_ledger["claim_template_contract"]["fields"])
    assert all(template_fields.issubset(claim) for claim in claim_ledger["claims"])
    assert all(
        claim["status"] == "source_evidenced_verification_pending"
        for claim in claim_ledger["claims"]
        if claim["claim_class"] == "software"
    )
    assert all(
        claim["status"] == "blocked_not_established"
        and claim["decision"] == "NOT_CLAIMED"
        for claim in claim_ledger["claims"]
        if claim["claim_class"] == "scientific"
    )

    defects = _read(CANDIDATE_DIR / "defect_register.json")
    assert {defect["priority"] for defect in defects["defects"]} == {"P0", "P1", "P2"}
    assert defects["summary"]["release_blocked"] is True
    assert all(defect["status"] == "open" for defect in defects["defects"])

    receipts = _read(CANDIDATE_DIR / "evidence_receipts.json")
    assert receipts["summary"]["pending_count"] == len(receipts["receipts"])
    assert receipts["summary"]["passed_count"] == 0
    assert all(
        receipt["status"] == "pending_post_push_ci" and receipt["conclusion"] is None
        for receipt in receipts["receipts"]
    )

    draft = _read(CANDIDATE_DIR / "draft_release_manifest.json")
    handoff_fields = {
        "manifest_schema",
        "project",
        "release_version",
        "decision",
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
    assert handoff_fields.issubset(draft)
    assert draft["decision"] == "NO_GO"
    assert draft["release"]["published"] is False
    assert draft["release"]["doi"] is None
    assert draft["release"]["zenodo_record"] is None
    assert draft["release"]["one_point_zero_convergence_claimed"] is False


def test_generator_requires_explicit_source_state_and_is_deterministic(
    tmp_path: Path,
) -> None:
    output_dir = tmp_path / "generated"
    missing = subprocess.run(
        [
            sys.executable,
            os.fspath(GENERATOR),
            "--repo",
            os.fspath(ROOT),
            "--output-dir",
            os.fspath(output_dir),
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    assert missing.returncode == 2
    assert "explicit --source-head" in missing.stderr
    assert not output_dir.exists()

    module = runpy.run_path(os.fspath(GENERATOR))
    inventory = _read(CANDIDATE_DIR / "source_inventory.json")
    first = module["build_artifacts_from_inventory"](ROOT, inventory)
    second = module["build_artifacts_from_inventory"](ROOT, inventory)
    assert first == second
    assert set(first) == EXPECTED_NAMES
    for name in EXPECTED_NAMES:
        assert first[name] == (CANDIDATE_DIR / name).read_bytes()


def test_generator_rejects_an_explicit_source_mismatch(tmp_path: Path) -> None:
    result = subprocess.run(
        [
            sys.executable,
            os.fspath(GENERATOR),
            "--repo",
            os.fspath(ROOT),
            "--output-dir",
            os.fspath(tmp_path / "out"),
            "--source-head",
            "0" * 40,
            "--source-index-sha256",
            "0" * 64,
            "--source-worktree-sha256",
            "0" * 64,
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 3
    assert "[EXPLICIT_SOURCE_MISMATCH]" in result.stderr
    assert not (tmp_path / "out").exists()


def test_explicit_in_progress_overlay_round_trips_without_claim_promotion(
    tmp_path: Path,
) -> None:
    _, artifacts = _synthetic_in_progress_artifacts()
    candidate = tmp_path / "candidate"
    _write_artifact_set(candidate, artifacts)
    auditor = _load_auditor()
    result = auditor["audit"](ROOT, candidate, validate_live_source=False)
    tasks = json.loads(artifacts["task_lens_ledger.json"])
    claims = json.loads(artifacts["claim_evidence_ledger.json"])
    defects = json.loads(artifacts["defect_register.json"])
    receipts = json.loads(artifacts["evidence_receipts.json"])
    draft = json.loads(artifacts["draft_release_manifest.json"])

    assert result["status"] == "pass"
    assert result["source_match"] == "internal_snapshot_only"
    assert tasks["summary"]["in_progress_task_count"] == 1
    assert tasks["summary"]["in_progress_lens_disposition_count"] == 1
    assert receipts["summary"]["in_progress_count"] == 1
    assert receipts["summary"]["pending_count"] == 5
    assert defects["summary"]["release_blocked"] is True
    assert claims["summary"]["scientific_established_count"] == 0
    assert all(
        claim["status"] in {"blocked_not_established", "withdrawn"}
        for claim in claims["claims"]
        if claim["claim_class"] == "scientific"
    )
    assert draft["decision"] == "NO_GO"
    assert draft["decision_detail"]["release_ready"] is False
    assert draft["release"]["published"] is False
    assert (
        draft["candidate_gates"]["retained_software_claims_verified_or_withdrawn"]
        == "open"
    )
    assert (
        "retained software claims remain pending exact-candidate verification"
        in draft["residual_risks"]
    )


def test_auditor_rejects_output_only_terminal_promotion_from_progress_candidate(
    tmp_path: Path,
) -> None:
    _, artifacts = _synthetic_in_progress_artifacts()
    candidate = tmp_path / "candidate"
    _write_artifact_set(candidate, artifacts)

    def close_without_evidence(value: dict[str, Any]) -> None:
        value["tasks"][0]["status"] = "closed"

    _mutate(candidate, "task_lens_ledger.json", close_without_evidence)
    auditor = _load_auditor()
    with pytest.raises(Exception) as caught:
        auditor["audit"](ROOT, candidate, validate_live_source=False)
    assert getattr(caught.value, "code", None) == "TERMINAL_PROMOTION_DISABLED"


def test_generator_rejects_nonindependent_or_unsupported_terminal_progress() -> None:
    generator, _ = _synthetic_in_progress_artifacts()
    inventory = _read(CANDIDATE_DIR / "source_inventory.json")
    progress = copy.deepcopy(inventory["progress_snapshot"]["document"])
    initial = generator["build_artifacts_from_inventory"](ROOT, inventory)
    claims = json.loads(initial["claim_evidence_ledger.json"])
    claim = next(
        item for item in claims["claims"] if item["claim_class"] == "scientific"
    )
    progress["claim_updates"] = [
        {
            "claim_id": claim["claim_id"],
            "status": "blocked_not_established",
            "decision": "NOT_CLAIMED",
            "reviewer": "Same Reviewer",
            "independent_reviewer": "Same Reviewer",
            "updated_at": "2026-07-14T12:00:00Z",
            "evidence_paths": ["README.md"],
            "evidence_receipt_ids": [],
            "residual_assumptions": ["The scientific gate remains blocked."],
        }
    ]
    updated_inventory = generator["inventory_with_progress"](inventory, progress)
    with pytest.raises(generator["CandidateError"]) as caught:
        generator["build_artifacts_from_inventory"](ROOT, updated_inventory)
    assert caught.value.code == "PROGRESS_INDEPENDENT_REVIEW"

    tasks = json.loads(initial["task_lens_ledger.json"])
    task = tasks["tasks"][0]
    progress = copy.deepcopy(inventory["progress_snapshot"]["document"])
    progress["task_updates"] = [
        {
            "task_id": task["id"],
            "status": "closed",
            "decision": "ACCEPTED",
            "owner": "Task Owner",
            "reviewer": "Primary Reviewer",
            "independent_reviewer": "Independent Reviewer",
            "updated_at": "2026-07-14T12:00:00Z",
            "evidence_paths": ["README.md"],
            "evidence_receipt_ids": ["RCP-RUST"],
            "blockers": [],
            "claim_impact": "The software scope is retained.",
        }
    ]
    updated_inventory = generator["inventory_with_progress"](inventory, progress)
    with pytest.raises(generator["CandidateError"]) as caught:
        generator["build_artifacts_from_inventory"](ROOT, updated_inventory)
    assert caught.value.code == "TERMINAL_PROMOTION_DISABLED"


def test_progress_timestamp_rejects_impossible_calendar_date() -> None:
    generator = _load_generator()
    inventory = _read(CANDIDATE_DIR / "source_inventory.json")
    progress = copy.deepcopy(inventory["progress_snapshot"]["document"])
    progress["evidence_receipt_updates"] = [
        {
            "receipt_id": "RCP-RERUN",
            "status": "in_progress",
            "reviewer": "Primary Reviewer",
            "independent_reviewer": None,
            "updated_at": "2026-07-14T12:00:00Z",
            "execution": {
                "commit": None,
                "started_at": "2026-02-31T12:00:00Z",
                "completed_at": None,
                "runner": None,
                "workflow_run_url": None,
                "exit_codes": [],
            },
            "evidence_path": None,
            "conclusion": "running",
        }
    ]
    with pytest.raises(generator["CandidateError"]) as caught:
        generator["build_receipts"](inventory, progress)
    assert caught.value.code == "PROGRESS_TIMESTAMP"


def test_final_receipt_rejects_reversed_execution_chronology() -> None:
    generator = _load_generator()
    inventory = _read(CANDIDATE_DIR / "source_inventory.json")
    progress = copy.deepcopy(inventory["progress_snapshot"]["document"])
    update = _failed_rerun_update(inventory, "release/0.9.0/evidence/not-reached.log")
    update["execution"]["started_at"] = "2026-07-14T12:01:00Z"
    update["execution"]["completed_at"] = "2026-07-14T12:00:00Z"
    progress["evidence_receipt_updates"] = [update]
    with pytest.raises(generator["CandidateError"]) as caught:
        generator["build_receipts"](inventory, progress)
    assert caught.value.code == "PROGRESS_RECEIPT_CHRONOLOGY"


def test_schema_0_1_rejects_task_lens_and_wave_terminal_promotions() -> None:
    generator = _load_generator()
    evidence_entry = {
        "path": "README.md",
        "working_tree": {"kind": "regular", "sha256": "1" * 64, "bytes": 1},
    }
    inventory = {"entries": [evidence_entry]}
    task = {
        "id": "TASK-1",
        "status": "open",
        "decision": None,
        "owner": None,
        "reviewer": None,
        "independent_reviewer": None,
        "updated_at": None,
        "completed_at": None,
        "evidence": [],
        "evidence_receipt_ids": [],
        "blockers": [],
        "claim_impact": None,
        "lenses": [
            {
                "lens_id": "LENS-1",
                "status": "open",
                "decision": None,
                "reviewer": None,
                "independent_reviewer": None,
                "updated_at": None,
                "reviewed_at": None,
                "finding": None,
                "evidence": [],
                "evidence_receipt_ids": [],
                "blockers": [],
            }
        ],
    }
    document = {
        "status_contract": {
            "task_statuses": [
                "open",
                "in_progress",
                "blocked",
                "closed",
                "claim_removed",
            ],
            "lens_statuses": [
                "open",
                "in_progress",
                "blocked",
                "closed",
                "claim_removed",
            ],
        },
        "phases": [{"id": "PHASE-1", "task_ids": ["TASK-1"], "status": "open"}],
        "tasks": [task],
    }
    task_update = {
        "task_id": "TASK-1",
        "status": "claim_removed",
        "decision": "CLAIM_REMOVED",
        "owner": None,
        "reviewer": "Primary Reviewer",
        "independent_reviewer": "Independent Reviewer",
        "updated_at": "2026-07-14T12:00:00Z",
        "evidence_paths": ["README.md"],
        "evidence_receipt_ids": [],
        "blockers": [],
        "claim_impact": "The associated claim is removed.",
    }
    wave = {
        "wave_id": "PHASE-1",
        "decision": "CLAIM_REMOVED",
        "reviewer": "Primary Reviewer",
        "independent_reviewer": "Independent Reviewer",
        "updated_at": "2026-07-14T12:00:00Z",
        "evidence_paths": ["README.md"],
        "evidence_receipt_ids": [],
        "task_ids": ["TASK-1"],
        "notes": "The complete wave claim is removed.",
    }
    task_terminal = {
        "task_updates": [task_update],
        "lens_updates": [],
        "wave_receipts": [],
    }
    with pytest.raises(generator["CandidateError"]) as caught:
        generator["_apply_task_lens_progress"](
            copy.deepcopy(document), inventory, task_terminal, {"receipts": []}
        )
    assert caught.value.code == "TERMINAL_PROMOTION_DISABLED"

    lens_terminal = {
        "task_updates": [],
        "lens_updates": [
            {
                "task_id": "TASK-1",
                "lens_id": "LENS-1",
                "status": "claim_removed",
                "decision": "CLAIM_REMOVED",
                "reviewer": "Primary Reviewer",
                "independent_reviewer": "Independent Reviewer",
                "updated_at": "2026-07-14T12:00:00Z",
                "finding": "The lens disposition is removed with the claim.",
                "evidence_paths": ["README.md"],
                "evidence_receipt_ids": [],
                "blockers": [],
            }
        ],
        "wave_receipts": [],
    }
    with pytest.raises(generator["CandidateError"]) as caught:
        generator["_apply_task_lens_progress"](
            copy.deepcopy(document), inventory, lens_terminal, {"receipts": []}
        )
    assert caught.value.code == "TERMINAL_PROMOTION_DISABLED"

    wave_terminal = {
        "task_updates": [],
        "lens_updates": [],
        "wave_receipts": [wave],
    }
    with pytest.raises(generator["CandidateError"]) as caught:
        generator["_apply_task_lens_progress"](
            copy.deepcopy(document), inventory, wave_terminal, {"receipts": []}
        )
    assert caught.value.code == "TERMINAL_PROMOTION_DISABLED"


def test_schema_0_1_rejects_file_claim_and_defect_terminal_promotions() -> None:
    generator = _load_generator()
    inventory = _read(CANDIDATE_DIR / "source_inventory.json")
    progress = copy.deepcopy(inventory["progress_snapshot"]["document"])
    progress["file_review_updates"] = [
        {
            "path": "README.md",
            "disposition": "ACCEPT",
            "reviewer": "Primary Reviewer",
            "independent_reviewer": "Independent Reviewer",
            "updated_at": "2026-07-14T12:00:00Z",
            "requirements": ["The exact file requirements were reviewed."],
            "defects": [],
            "tests": ["The exact file checks passed."],
            "evidence_paths": ["README.md"],
            "notes": "A terminal disposition is deliberately rejected by schema 0.1.",
        }
    ]
    with pytest.raises(generator["CandidateError"]) as caught:
        generator["_apply_file_review_progress"](
            copy.deepcopy(inventory["entries"]), progress
        )
    assert caught.value.code == "TERMINAL_PROMOTION_DISABLED"

    initial = generator["build_artifacts_from_inventory"](ROOT, inventory)
    claims = json.loads(initial["claim_evidence_ledger.json"])
    defects = json.loads(initial["defect_register.json"])
    receipts = json.loads(initial["evidence_receipts.json"])
    software_claim = next(
        claim for claim in claims["claims"] if claim["claim_class"] == "software"
    )
    claim_progress = {
        "claim_updates": [
            {
                "claim_id": software_claim["claim_id"],
                "status": "withdrawn",
                "decision": "CLAIM_REMOVED",
                "reviewer": "Primary Reviewer",
                "independent_reviewer": "Independent Reviewer",
                "updated_at": "2026-07-14T12:00:00Z",
                "evidence_paths": ["README.md"],
                "evidence_receipt_ids": [],
                "residual_assumptions": ["The claim is not retained."],
            }
        ]
    }
    with pytest.raises(generator["CandidateError"]) as caught:
        generator["_apply_claim_progress"](claims, inventory, claim_progress, receipts)
    assert caught.value.code == "TERMINAL_PROMOTION_DISABLED"

    defect_progress = {
        "defect_updates": [
            {
                "defect_id": defects["defects"][0]["id"],
                "status": "closed",
                "decision": "CLOSED",
                "reviewer": "Primary Reviewer",
                "independent_reviewer": "Independent Reviewer",
                "updated_at": "2026-07-14T12:00:00Z",
                "evidence_paths": ["README.md"],
                "evidence_receipt_ids": [],
                "residual_risk": None,
            }
        ]
    }
    with pytest.raises(generator["CandidateError"]) as caught:
        generator["_apply_defect_progress"](
            defects, inventory, defect_progress, receipts
        )
    assert caught.value.code == "TERMINAL_PROMOTION_DISABLED"


def test_progress_schema_requires_the_nonpromotable_policy() -> None:
    generator = _load_generator()
    inventory = _read(CANDIDATE_DIR / "source_inventory.json")
    progress = copy.deepcopy(inventory["progress_snapshot"]["document"])
    progress.pop("terminal_promotion_policy", None)
    with pytest.raises(generator["CandidateError"]) as caught:
        generator["_validate_progress_document"](
            progress, generator["pretty_json_bytes"](progress)
        )
    assert caught.value.code == "PROGRESS_SCHEMA"

    progress["terminal_promotion_policy"] = "enabled"
    with pytest.raises(generator["CandidateError"]) as caught:
        generator["_validate_progress_document"](
            progress, generator["pretty_json_bytes"](progress)
        )
    assert caught.value.code == "PROGRESS_IDENTITY"


def test_receipt_consumer_cannot_predate_failed_receipt_completion() -> None:
    generator = _load_generator()
    base_inventory = _read(CANDIDATE_DIR / "source_inventory.json")
    evidence_path = "release/0.9.0/evidence/rerun.log"
    inventory = _inventory_with_synthetic_evidence_log(base_inventory, evidence_path)
    progress = copy.deepcopy(inventory["progress_snapshot"]["document"])
    progress["evidence_receipt_updates"] = [
        _failed_rerun_update(inventory, evidence_path)
    ]
    receipts = generator["build_receipts"](inventory, progress)
    with pytest.raises(generator["CandidateError"]) as caught:
        generator["_validated_receipt_refs"](
            ["RCP-RERUN"],
            receipts,
            context="consumer receipt refs",
            require_passed=False,
            allow_empty=False,
            consumer_updated_at="2026-07-14T12:00:00Z",
        )
    assert caught.value.code == "PROGRESS_RECEIPT_CHRONOLOGY"


def test_draft_schema_0_1_remains_no_go_even_if_inputs_assert_finality() -> None:
    generator = _load_generator()
    inventory = copy.deepcopy(_read(CANDIDATE_DIR / "source_inventory.json"))
    initial = generator["build_artifacts_from_inventory"](ROOT, inventory)
    tasks = json.loads(initial["task_lens_ledger.json"])
    claims = json.loads(initial["claim_evidence_ledger.json"])
    defects = json.loads(initial["defect_register.json"])
    receipts = json.loads(initial["evidence_receipts.json"])
    for entry in inventory["entries"]:
        entry["file_review"]["disposition"] = "ACCEPT"
    tasks["summary"]["all_tasks_closed_or_claim_removed"] = True
    tasks["summary"]["wave_accepted_count"] = len(tasks["phases"])
    tasks["summary"]["claim_removed_wave_count"] = 0
    defects["summary"]["release_blocked"] = False
    receipts["summary"]["all_required_evidence_passed"] = True
    for receipt in receipts["receipts"]:
        receipt["status"] = "passed"
    post_push = next(
        receipt
        for receipt in receipts["receipts"]
        if receipt["id"] == "RCP-POST-PUSH-CI"
    )
    post_push["execution"]["commit"] = inventory["source"]["head_commit"]

    pending = generator["build_draft_manifest"](
        inventory, tasks, claims, defects, receipts
    )
    assert pending["decision"] == "NO_GO"
    assert (
        pending["candidate_gates"]["retained_software_claims_verified_or_withdrawn"]
        == "open"
    )
    assert (
        "retained software claims remain pending exact-candidate verification"
        in pending["residual_risks"]
    )

    for claim in claims["claims"]:
        if claim["claim_class"] == "software":
            claim["status"] = "withdrawn"
    ready = generator["build_draft_manifest"](
        inventory, tasks, claims, defects, receipts
    )
    assert ready["decision"] == "NO_GO"
    assert ready["decision_detail"]["release_ready"] is False
    assert ready["decision_detail"]["publication_ready"] is False
    assert ready["release"]["published"] is False
    assert ready["terminal_promotion"] == {
        "enabled": False,
        "policy": "disabled_in_0.1_pending_typed_authenticated_evidence",
        "readiness_prerequisites_satisfied": True,
        "successor_schema_required": True,
    }
    assert ready["candidate_gates"]["terminal_promotion_schema"] == "open"


def test_schema_0_1_rejects_every_positive_receipt() -> None:
    generator = _load_generator()
    inventory = _read(CANDIDATE_DIR / "source_inventory.json")
    progress = copy.deepcopy(inventory["progress_snapshot"]["document"])
    progress["evidence_receipt_updates"] = [
        _passed_rerun_update(inventory, "README.md")
    ]
    with pytest.raises(generator["CandidateError"]) as caught:
        generator["build_receipts"](inventory, progress)
    assert caught.value.code == "TERMINAL_PROMOTION_DISABLED"


def test_failed_receipt_cannot_whitelist_source_as_an_evidence_log() -> None:
    generator = _load_generator()
    inventory = _read(CANDIDATE_DIR / "source_inventory.json")
    progress = copy.deepcopy(inventory["progress_snapshot"]["document"])
    progress["evidence_receipt_updates"] = [
        _failed_rerun_update(inventory, "README.md")
    ]
    with pytest.raises(generator["CandidateError"]) as caught:
        generator["build_receipts"](inventory, progress)
    assert caught.value.code == "PROGRESS_RECEIPT_EVIDENCE_PATH"


def test_audit_rejects_false_task_closure_even_with_updated_artifact_hash(
    tmp_path: Path,
) -> None:
    candidate = _copy_candidate(tmp_path)

    def close_task(value: dict[str, Any]) -> None:
        value["tasks"][0]["status"] = "closed"

    _mutate(candidate, "task_lens_ledger.json", close_task)
    _assert_rejected(candidate, "TERMINAL_PROMOTION_DISABLED")


def test_audit_rejects_promoted_scientific_claim(tmp_path: Path) -> None:
    candidate = _copy_candidate(tmp_path)

    def promote(value: dict[str, Any]) -> None:
        claim = next(
            item for item in value["claims"] if item["claim_class"] == "scientific"
        )
        claim["status"] = "established"
        claim["decision"] = "RELEASED_CLAIM"

    _mutate(candidate, "claim_evidence_ledger.json", promote)
    _assert_rejected(candidate, "FALSE_SCIENTIFIC_CLAIM")


def test_audit_rejects_fabricated_receipt_or_defect_closure(tmp_path: Path) -> None:
    candidate = _copy_candidate(tmp_path)

    def pass_receipt(value: dict[str, Any]) -> None:
        value["receipts"][0]["status"] = "passed"
        value["receipts"][0]["conclusion"] = "success"

    _mutate(candidate, "evidence_receipts.json", pass_receipt)
    _assert_rejected(candidate, "TERMINAL_PROMOTION_DISABLED")

    candidate = _copy_candidate(tmp_path / "second")

    def close_defect(value: dict[str, Any]) -> None:
        value["defects"][0]["status"] = "closed"

    _mutate(candidate, "defect_register.json", close_defect)
    _assert_rejected(candidate, "TERMINAL_PROMOTION_DISABLED")


def test_audit_rejects_false_file_review_and_inventory_omission(tmp_path: Path) -> None:
    candidate = _copy_candidate(tmp_path)

    def claim_review(value: dict[str, Any]) -> None:
        value["entries"][0]["file_review"]["disposition"] = "ACCEPT"
        value["entries"][0]["file_review"]["reviewer"] = "unverified"

    _mutate(candidate, "source_inventory.json", claim_review)
    _assert_rejected(candidate, "FALSE_FILE_REVIEW")

    candidate = _copy_candidate(tmp_path / "second")

    def omit_path(value: dict[str, Any]) -> None:
        value["entries"].pop()

    _mutate(candidate, "source_inventory.json", omit_path)
    _assert_rejected(candidate, "INVENTORY_HEAD_COVERAGE")


def test_audit_rejects_publication_metadata_and_fixed_point_drift(
    tmp_path: Path,
) -> None:
    candidate = _copy_candidate(tmp_path)

    def publish(value: dict[str, Any]) -> None:
        value["release"]["published"] = True
        value["release"]["doi"] = "10.0000/not-a-real-candidate"

    _mutate(candidate, "draft_release_manifest.json", publish)
    _assert_rejected(candidate, "FALSE_PUBLICATION")

    candidate = _copy_candidate(tmp_path / "second")

    def remove_exclusion(value: dict[str, Any]) -> None:
        value["inventory_policy"]["self_excluded_paths"] = []

    _mutate(candidate, "source_inventory.json", remove_exclusion)
    _assert_rejected(candidate, "INVENTORY_POLICY")


def test_audit_rejects_symlink_and_duplicate_json_artifacts(tmp_path: Path) -> None:
    candidate = _copy_candidate(tmp_path)
    target = candidate / "source_inventory.json"
    raw = target.read_bytes()
    target.unlink()
    (candidate / "inventory-target.json").write_bytes(raw)
    target.symlink_to("inventory-target.json")
    _assert_rejected(candidate, "CANDIDATE_ENTRY")

    candidate = _copy_candidate(tmp_path / "second")
    path = candidate / "source_inventory.json"
    raw = path.read_text(encoding="utf-8")
    path.write_text(
        raw.replace(
            '{\n  "candidate_boundary"',
            '{\n  "project": "prisoma",\n  "candidate_boundary"',
            1,
        ),
        encoding="utf-8",
    )
    _assert_rejected(candidate, "JSON_DUPLICATE_KEY")


def test_candidate_readers_reject_oversize_and_fifo_without_blocking(
    tmp_path: Path,
) -> None:
    candidate = _copy_candidate(tmp_path)
    with (candidate / "source_inventory.json").open("wb") as handle:
        handle.truncate(64 * 1024 * 1024 + 1)
    _assert_rejected(candidate, "ARTIFACT_SIZE")

    if not hasattr(os, "mkfifo"):
        return
    generator = _load_generator()
    fifo = tmp_path / "bounded-reader.fifo"
    os.mkfifo(fifo)
    with pytest.raises(generator["CandidateError"]) as caught:
        generator["_read_bounded_regular"](
            fifo,
            max_bytes=8,
            path_code="TEST_PATH",
            read_code="TEST_READ",
            size_code="TEST_SIZE",
            description="test input",
        )
    assert caught.value.code == "TEST_PATH"


def test_candidate_writer_installs_manifest_last_and_fails_closed_mid_refresh(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    generator = _load_generator()
    artifacts = {name: (CANDIDATE_DIR / name).read_bytes() for name in EXPECTED_NAMES}
    output = tmp_path / "candidate"
    real_replace = os.replace
    installed: list[str] = []

    def observe_replace(source: Path, destination: Path) -> None:
        installed.append(Path(destination).name)
        real_replace(source, destination)

    monkeypatch.setattr(generator["os"], "replace", observe_replace)
    generator["write_artifacts"](output, artifacts)
    assert installed == sorted(EXPECTED_NAMES - {ARTIFACT_MANIFEST}) + [
        ARTIFACT_MANIFEST
    ]

    old_manifest = (output / ARTIFACT_MANIFEST).read_bytes()
    changed = {name: raw + b" " for name, raw in artifacts.items()}
    attempted: list[str] = []

    def fail_second_replace(source: Path, destination: Path) -> None:
        attempted.append(Path(destination).name)
        if len(attempted) == 2:
            raise OSError("injected mid-refresh failure")
        real_replace(source, destination)

    monkeypatch.setattr(generator["os"], "replace", fail_second_replace)
    with pytest.raises(generator["CandidateError"]) as caught:
        generator["write_artifacts"](output, changed)
    assert caught.value.code == "OUTPUT_WRITE"
    assert ARTIFACT_MANIFEST not in attempted
    assert (output / ARTIFACT_MANIFEST).read_bytes() == old_manifest
    assert not any(path.name.startswith(".") for path in output.iterdir())
