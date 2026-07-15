from __future__ import annotations

import hashlib
import json
import os
import runpy
import shutil
import subprocess
import sys
from collections.abc import Callable
from pathlib import Path
from types import SimpleNamespace
from typing import Any

import pytest


ROOT = Path(__file__).resolve().parents[2]
REQUIREMENTS_DIR = ROOT / "release" / "0.9.0" / "requirements"
AUDITOR = ROOT / "scripts" / "audit_release_requirements.py"
GENERATOR = ROOT / "scripts" / "generate_release_requirements.py"
LEDGER_NAME = "19_MASTER_TASK_LEDGER.yaml"
LEDGER_SHA256 = "384f5540dcdb4709b8f9add57e355761c6e076a1c4b22e26e42482bd0c0c4f29"
PACKAGE_SHA256 = "05ff0e9c4292f630c003b116a9146155717b14359989d229d3b43fea2e936240"
ARTIFACT_MANIFEST = "artifact_manifest.json"
EXPECTED_NAMES = {
    ARTIFACT_MANIFEST,
    LEDGER_NAME,
    "handoff_package_manifest.json",
    "task_dispositions.baseline.json",
}


def _load_generator() -> dict[str, Any]:
    return runpy.run_path(os.fspath(GENERATOR))


def _load_auditor() -> dict[str, Any]:
    scripts = os.fspath(ROOT / "scripts")
    sys.path.insert(0, scripts)
    try:
        return runpy.run_path(os.fspath(AUDITOR))
    finally:
        sys.path.remove(scripts)


def _run_audit(requirements_dir: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            os.fspath(AUDITOR),
            "--requirements-dir",
            os.fspath(requirements_dir),
        ],
        check=False,
        capture_output=True,
        text=True,
    )


def _copy_requirements(tmp_path: Path) -> Path:
    destination = tmp_path / "requirements"
    shutil.copytree(REQUIREMENTS_DIR, destination)
    return destination


def _read(path: Path) -> dict[str, Any]:
    document = json.loads(path.read_bytes())
    assert isinstance(document, dict)
    return document


def _write(path: Path, document: dict[str, Any]) -> None:
    path.write_text(
        json.dumps(
            document, ensure_ascii=False, allow_nan=False, indent=2, sort_keys=True
        )
        + "\n",
        encoding="utf-8",
    )


def _mutate(
    requirements_dir: Path,
    name: str,
    mutation: Callable[[dict[str, Any]], None],
) -> None:
    path = requirements_dir / name
    document = _read(path)
    mutation(document)
    _write(path, document)


def _assert_rejected(requirements_dir: Path, code: str) -> None:
    result = _run_audit(requirements_dir)
    assert result.returncode == 3, (result.stdout, result.stderr)
    assert result.stdout == ""
    assert f"[{code}]" in result.stderr
    assert "Traceback" not in result.stderr


def test_requirements_baseline_passes_without_external_handoff_lookup() -> None:
    result = _run_audit(REQUIREMENTS_DIR)
    assert result.returncode == 0, result.stderr
    assert json.loads(result.stdout) == {
        "all_dispositions_open": True,
        "external_handoff_reverified": False,
        "lens_disposition_count": 4_800,
        "release_ready": False,
        "release_version": "0.9.0",
        "review_complete": False,
        "status": "pass",
        "task_count": 240,
    }


def test_generator_requires_explicit_handoff_directory(tmp_path: Path) -> None:
    output = tmp_path / "output"
    result = subprocess.run(
        [sys.executable, os.fspath(GENERATOR), "--output-dir", os.fspath(output)],
        check=False,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 2
    assert "--handoff-dir" in result.stderr
    assert not output.exists()


def test_exact_source_copy_and_package_boundary_are_bound() -> None:
    source = (REQUIREMENTS_DIR / LEDGER_NAME).read_bytes()
    assert len(source) == 2_281_617
    assert hashlib.sha256(source).hexdigest() == LEDGER_SHA256
    package = _read(REQUIREMENTS_DIR / "handoff_package_manifest.json")
    assert package["included_file_count"] == 43
    assert package["excluded_file_count"] == 2
    assert package["observed_file_count"] == 45
    assert package["package_identity_sha256"] == PACKAGE_SHA256
    assert [entry["path"] for entry in package["excluded_files"]] == [
        ".DS_Store",
        "repo_work/.DS_Store",
    ]
    assert all(entry["content_read"] is False for entry in package["excluded_files"])
    assert all(
        entry["content_identity_bound"] is False for entry in package["excluded_files"]
    )
    included_paths = [entry["path"] for entry in package["included_files"]]
    assert len(included_paths) == len(set(included_paths)) == 43
    assert LEDGER_NAME in included_paths
    assert "29_SHA256SUMS.txt" in included_paths


def test_all_full_requirements_and_dispositions_are_retained_open() -> None:
    document = _read(REQUIREMENTS_DIR / "task_dispositions.baseline.json")
    assert document["task_count"] == 240
    assert document["lens_count"] == 20
    assert document["lens_disposition_count"] == 4_800
    assert document["review"]["open_task_count"] == 240
    assert document["review"]["closed_task_count"] == 0
    assert document["review"]["open_lens_disposition_count"] == 4_800
    assert document["review"]["closed_lens_disposition_count"] == 0
    expected_ids = [f"T{index:03d}" for index in range(240)]
    assert [task["requirements"]["id"] for task in document["tasks"]] == expected_ids
    for task in document["tasks"]:
        requirements = task["requirements"]
        assert requirements["head_mismatch_rule"]
        assert requirements["preconditions"]
        assert requirements["procedure"]
        assert requirements["mandatory_adversarial_questions"]
        assert requirements["required_tests"]
        assert requirements["required_evidence"]
        assert requirements["completion_rule"]
        assert task["task_disposition"] == {
            "blockers": [],
            "claim_impact": None,
            "completed_at": None,
            "decision": None,
            "evidence_refs": [],
            "independent_reviewer": None,
            "owner": None,
            "reviewer": None,
            "status": "open",
        }
        assert len(task["lens_requirements"]) == 20
        assert len(task["lens_dispositions"]) == 20
        assert all(lens["status"] == "OPEN" for lens in task["lens_requirements"])
        assert all(lens["status"] == "open" for lens in task["lens_dispositions"])


def test_every_task_source_block_is_content_bound() -> None:
    raw_lines = (REQUIREMENTS_DIR / LEDGER_NAME).read_bytes().splitlines(keepends=True)
    document = _read(REQUIREMENTS_DIR / "task_dispositions.baseline.json")
    for task in document["tasks"]:
        source = task["requirements"]["source_block"]
        block = b"".join(raw_lines[source["line_start"] - 1 : source["line_end"]])
        assert len(block) == source["bytes"]
        assert hashlib.sha256(block).hexdigest() == source["sha256"]


def test_audit_rejects_false_task_completion(tmp_path: Path) -> None:
    requirements = _copy_requirements(tmp_path)

    def mutate(document: dict[str, Any]) -> None:
        disposition = document["tasks"][0]["task_disposition"]
        disposition["status"] = "complete"
        disposition["decision"] = "GO"
        disposition["evidence_refs"] = ["fabricated"]

    _mutate(requirements, "task_dispositions.baseline.json", mutate)
    _assert_rejected(requirements, "FALSE_TASK_COMPLETION")


def test_audit_rejects_false_lens_completion(tmp_path: Path) -> None:
    requirements = _copy_requirements(tmp_path)

    def mutate(document: dict[str, Any]) -> None:
        disposition = document["tasks"][17]["lens_dispositions"][4]
        disposition["status"] = "complete"
        disposition["finding"] = "unsupported"

    _mutate(requirements, "task_dispositions.baseline.json", mutate)
    _assert_rejected(requirements, "FALSE_LENS_COMPLETION")


def test_audit_rejects_omitted_full_requirement(tmp_path: Path) -> None:
    requirements = _copy_requirements(tmp_path)

    def mutate(document: dict[str, Any]) -> None:
        document["tasks"][239]["requirements"]["required_evidence"] = []

    _mutate(requirements, "task_dispositions.baseline.json", mutate)
    _assert_rejected(requirements, "REQUIREMENTS_OMITTED")


def test_audit_rejects_exact_source_copy_drift(tmp_path: Path) -> None:
    requirements = _copy_requirements(tmp_path)
    source = requirements / LEDGER_NAME
    source.write_bytes(source.read_bytes() + b"\n")
    _assert_rejected(requirements, "LEDGER_IDENTITY")


def test_audit_rejects_package_identity_drift(tmp_path: Path) -> None:
    requirements = _copy_requirements(tmp_path)

    def mutate(document: dict[str, Any]) -> None:
        document["included_files"][0]["sha256"] = "0" * 64

    _mutate(requirements, "handoff_package_manifest.json", mutate)
    _assert_rejected(requirements, "PACKAGE_IDENTITY")


def test_audit_rejects_false_package_review_completion(tmp_path: Path) -> None:
    requirements = _copy_requirements(tmp_path)

    def mutate(document: dict[str, Any]) -> None:
        document["review"]["all_files_read_for_substantive_review"] = True
        document["review"]["human_review_complete"] = True
        document["review"]["release_ready"] = True

    _mutate(requirements, "handoff_package_manifest.json", mutate)
    _assert_rejected(requirements, "FALSE_PACKAGE_REVIEW")


def test_audit_rejects_artifact_manifest_drift(tmp_path: Path) -> None:
    requirements = _copy_requirements(tmp_path)

    def mutate(document: dict[str, Any]) -> None:
        document["artifacts"][0]["sha256"] = "0" * 64

    _mutate(requirements, "artifact_manifest.json", mutate)
    _assert_rejected(requirements, "ARTIFACT_MANIFEST_DRIFT")


def test_audit_rejects_unmanifested_artifact(tmp_path: Path) -> None:
    requirements = _copy_requirements(tmp_path)
    (requirements / "UNMANIFESTED.txt").write_text("not allowed\n", encoding="utf-8")
    _assert_rejected(requirements, "REQUIREMENTS_FILE_SET")


def test_audit_rejects_source_symlink(tmp_path: Path) -> None:
    requirements = _copy_requirements(tmp_path)
    source = requirements / LEDGER_NAME
    target = tmp_path / "ledger.yaml"
    shutil.copyfile(source, target)
    source.unlink()
    source.symlink_to(target)
    _assert_rejected(requirements, "REQUIREMENTS_ENTRY")


def test_audit_rejects_duplicate_json_key(tmp_path: Path) -> None:
    requirements = _copy_requirements(tmp_path)
    path = requirements / "handoff_package_manifest.json"
    raw = path.read_text(encoding="utf-8")
    path.write_text(
        raw.replace("{\n", '{\n  "project": "duplicate",\n', 1),
        encoding="utf-8",
    )
    _assert_rejected(requirements, "JSON_DUPLICATE_KEY")


def test_audit_rejects_noncanonical_json(tmp_path: Path) -> None:
    requirements = _copy_requirements(tmp_path)
    path = requirements / "artifact_manifest.json"
    path.write_bytes(path.read_bytes() + b" ")
    _assert_rejected(requirements, "JSON_CANONICAL")


@pytest.mark.parametrize(
    "name",
    [
        LEDGER_NAME,
        "handoff_package_manifest.json",
        "task_dispositions.baseline.json",
    ],
)
def test_artifact_manifest_binds_exact_bytes(name: str) -> None:
    manifest = _read(REQUIREMENTS_DIR / "artifact_manifest.json")
    entry = next(item for item in manifest["artifacts"] if item["path"] == name)
    raw = (REQUIREMENTS_DIR / name).read_bytes()
    assert entry["bytes"] == len(raw)
    assert entry["sha256"] == hashlib.sha256(raw).hexdigest()


def test_bounded_readers_ignore_deceptive_path_stat_and_reject_oversize(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    generator = _load_generator()
    auditor = _load_auditor()
    source = tmp_path / "artifact.json"
    source.write_bytes(b"123456789")
    real_stat = Path.stat

    def deceptive_stat(self: Path, *args: Any, **kwargs: Any) -> Any:
        observed = real_stat(self, *args, **kwargs)
        if self == source:
            return SimpleNamespace(st_mode=observed.st_mode, st_size=1)
        return observed

    monkeypatch.setattr(Path, "stat", deceptive_stat)
    with pytest.raises(Exception) as generator_error:
        generator["_read_regular"](source, code="TEST_LIMIT", max_bytes=8)
    assert getattr(generator_error.value, "code", None) == "TEST_LIMIT"

    with pytest.raises(Exception) as auditor_error:
        auditor["_read_regular"](source, max_bytes=8)
    assert getattr(auditor_error.value, "code", None) == "ARTIFACT_TOO_LARGE"


def test_bounded_readers_reject_final_component_symlinks(tmp_path: Path) -> None:
    generator = _load_generator()
    auditor = _load_auditor()
    target = tmp_path / "target.json"
    target.write_bytes(b"{}\n")
    source = tmp_path / "artifact.json"
    source.symlink_to(target)

    with pytest.raises(Exception) as generator_error:
        generator["_read_regular"](source, code="TEST_SYMLINK", max_bytes=8)
    assert getattr(generator_error.value, "code", None) == "TEST_SYMLINK"

    with pytest.raises(Exception) as auditor_error:
        auditor["_read_regular"](source, max_bytes=8)
    assert getattr(auditor_error.value, "code", None) == "ARTIFACT_PATH"


@pytest.mark.skipif(not hasattr(os, "mkfifo"), reason="requires POSIX FIFOs")
def test_bounded_readers_reject_fifo_without_blocking(tmp_path: Path) -> None:
    generator = _load_generator()
    auditor = _load_auditor()
    source = tmp_path / "artifact.fifo"
    os.mkfifo(source)

    with pytest.raises(Exception) as generator_error:
        generator["_read_regular"](source, code="TEST_FIFO", max_bytes=8)
    assert getattr(generator_error.value, "code", None) == "TEST_FIFO"

    with pytest.raises(Exception) as auditor_error:
        auditor["_read_regular"](source, max_bytes=8)
    assert getattr(auditor_error.value, "code", None) == "ARTIFACT_PATH"


def test_handoff_traversal_rejects_entry_overflow(tmp_path: Path) -> None:
    generator = _load_generator()
    handoff = tmp_path / "handoff"
    handoff.mkdir()
    for index in range(generator["MAX_HANDOFF_ENTRIES"] + 1):
        (handoff / f"entry-{index:03d}").touch()

    with pytest.raises(Exception) as caught:
        generator["_observed_files"](handoff)
    assert getattr(caught.value, "code", None) == "HANDOFF_WALK_BUDGET"


def test_handoff_traversal_rejects_symlink_directory(tmp_path: Path) -> None:
    generator = _load_generator()
    handoff = tmp_path / "handoff"
    handoff.mkdir()
    target = handoff / "target"
    target.mkdir()
    (handoff / "alias").symlink_to(target, target_is_directory=True)

    with pytest.raises(Exception) as caught:
        generator["_observed_files"](handoff)
    assert getattr(caught.value, "code", None) == "HANDOFF_SYMLINK"


def test_stable_double_capture_rejects_late_payload_mutation(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    generator = _load_generator()
    handoff = tmp_path / generator["EXPECTED_DIRECTORY_NAME"]
    handoff.mkdir()
    checksum_path = handoff / generator["CHECKSUM_NAME"]
    checksum_path.write_bytes(b"synthetic checksum\n")
    payload_path = handoff / generator["LEDGER_NAME"]
    first_payload = b"first capture\n"
    payload_path.write_bytes(first_payload)
    (handoff / ".DS_Store").write_bytes(b"")
    (handoff / "repo_work").mkdir()
    (handoff / "repo_work" / ".DS_Store").write_bytes(b"")

    implementation = generator["build_package_manifest"].__globals__
    monkeypatch.setitem(
        implementation,
        "_parse_checksum_file",
        lambda _raw: [
            (generator["LEDGER_NAME"], hashlib.sha256(first_payload).hexdigest())
        ],
    )
    monkeypatch.setitem(implementation, "EXPECTED_INCLUDED_COUNT", 2)
    read_regular = generator["_read_regular"]
    payload_reads = 0

    def mutate_before_second_payload_read(
        path: Path, *, code: str, max_bytes: int
    ) -> bytes:
        nonlocal payload_reads
        if path == payload_path:
            payload_reads += 1
            if payload_reads == 2:
                payload_path.write_bytes(b"late mutation\n")
        return read_regular(path, code=code, max_bytes=max_bytes)

    monkeypatch.setitem(
        implementation, "_read_regular", mutate_before_second_payload_read
    )
    with pytest.raises(Exception) as caught:
        generator["build_package_manifest"](handoff)
    assert getattr(caught.value, "code", None) == "HANDOFF_RACE"
    assert payload_reads == 2


def test_requirements_writer_installs_manifest_last_and_preserves_it_on_failure(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    generator = _load_generator()
    artifacts = {
        name: (REQUIREMENTS_DIR / name).read_bytes() for name in EXPECTED_NAMES
    }
    output = tmp_path / "requirements"
    real_replace = os.replace
    real_sync = generator["_fsync_directory"]
    installed: list[str] = []
    sync_positions: list[int] = []

    def observe_replace(source: Path, destination: Path) -> None:
        installed.append(Path(destination).name)
        real_replace(source, destination)

    def observe_sync(path: Path) -> None:
        sync_positions.append(len(installed))
        real_sync(path)

    monkeypatch.setattr(generator["os"], "replace", observe_replace)
    monkeypatch.setitem(
        generator["write_artifacts"].__globals__, "_fsync_directory", observe_sync
    )
    generator["write_artifacts"](output, artifacts)
    assert installed == sorted(EXPECTED_NAMES - {ARTIFACT_MANIFEST}) + [
        ARTIFACT_MANIFEST
    ]
    assert sync_positions == [len(EXPECTED_NAMES) - 1, len(EXPECTED_NAMES)]

    old_manifest = (output / ARTIFACT_MANIFEST).read_bytes()
    changed = {name: raw + b" " for name, raw in artifacts.items()}
    attempted: list[str] = []

    def fail_second_replace(source: Path, destination: Path) -> None:
        attempted.append(Path(destination).name)
        if len(attempted) == 2:
            raise OSError("injected mid-refresh failure")
        real_replace(source, destination)

    monkeypatch.setattr(generator["os"], "replace", fail_second_replace)
    with pytest.raises(Exception) as caught:
        generator["write_artifacts"](output, changed)
    assert getattr(caught.value, "code", None) == "OUTPUT_WRITE"
    assert ARTIFACT_MANIFEST not in attempted
    assert (output / ARTIFACT_MANIFEST).read_bytes() == old_manifest
    assert not any(path.name.startswith(".") for path in output.iterdir())
