from __future__ import annotations

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
REVIEW_DIR = ROOT / "release" / "0.9.0" / "review"
AUDITOR = ROOT / "scripts" / "audit_release_review.py"
GENERATOR = ROOT / "scripts" / "generate_release_review.py"
FROZEN_COMMIT = "0968128062f30da5c04f3f31c23f6ce8e0d95d36"
PID_RS_COMMIT = "ac4a7803c5a77408f5e9176c60cda71c65c38260"
MASTER_LEDGER = (
    ROOT / "release" / "0.9.0" / "requirements" / "19_MASTER_TASK_LEDGER.yaml"
)
MANIFEST_NAME = "artifact_manifest.json"
EXPECTED_NAMES = {
    MANIFEST_NAME,
    "intake.json",
    "master_task_ledger.normalized.json",
    "tracked_file_inventory.baseline.json",
}


def _load_generator() -> dict[str, Any]:
    return runpy.run_path(os.fspath(GENERATOR))


def _install_fake_git(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    if os.name != "posix":
        pytest.skip("fake executable and process-reaping checks require POSIX")
    if any(character.isspace() for character in sys.executable):
        pytest.skip("the active Python executable cannot be represented in a shebang")

    executable = tmp_path / "bin" / "git"
    executable.parent.mkdir()
    executable.write_text(
        f"#!{sys.executable}\n"
        "import os\n"
        "import pathlib\n"
        "import subprocess\n"
        "import sys\n"
        "import time\n"
        "pid_file = os.environ.get('FAKE_GIT_PID_FILE')\n"
        "if pid_file is not None:\n"
        "    with open(pid_file, 'w', encoding='ascii') as handle:\n"
        "        handle.write(str(os.getpid()))\n"
        "stdout_bytes = int(os.environ.get('FAKE_GIT_STDOUT_BYTES', '0'))\n"
        "stderr_bytes = int(os.environ.get('FAKE_GIT_STDERR_BYTES', '0'))\n"
        "if stdout_bytes:\n"
        "    sys.stdout.buffer.write(b'o' * stdout_bytes)\n"
        "    sys.stdout.buffer.flush()\n"
        "if stderr_bytes:\n"
        "    sys.stderr.buffer.write(b'e' * stderr_bytes)\n"
        "    sys.stderr.buffer.flush()\n"
        "descendant_marker = os.environ.get('FAKE_GIT_DESCENDANT_MARKER')\n"
        "if descendant_marker is not None:\n"
        "    subprocess.Popen(\n"
        "        [\n"
        "            sys.executable,\n"
        "            '-c',\n"
        "            'import pathlib, sys, time; time.sleep(0.2); '\n"
        "            \"pathlib.Path(sys.argv[1]).write_text('escaped', encoding='utf-8')\",\n"
        "            descendant_marker,\n"
        "        ],\n"
        "        stdin=subprocess.DEVNULL,\n"
        "        stdout=subprocess.DEVNULL,\n"
        "        stderr=subprocess.DEVNULL,\n"
        "    )\n"
        "time.sleep(float(os.environ.get('FAKE_GIT_SLEEP_SECONDS', '0')))\n"
        "marker = os.environ.get('FAKE_GIT_MARKER')\n"
        "if marker is not None:\n"
        "    pathlib.Path(marker).write_text('escaped', encoding='utf-8')\n",
        encoding="utf-8",
    )
    executable.chmod(0o755)
    monkeypatch.setenv(
        "PATH", f"{executable.parent}{os.pathsep}{os.environ.get('PATH', '')}"
    )
    return executable


def _run_audit(review_dir: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            os.fspath(AUDITOR),
            "--repo",
            os.fspath(ROOT),
            "--review-dir",
            os.fspath(review_dir),
        ],
        check=False,
        capture_output=True,
        text=True,
    )


def _run_generator_check(review_dir: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            os.fspath(GENERATOR),
            "--repo",
            os.fspath(ROOT),
            "--master-ledger",
            os.fspath(MASTER_LEDGER),
            "--output-dir",
            os.fspath(review_dir),
            "--check",
        ],
        check=False,
        capture_output=True,
        text=True,
    )


def _copy_review(tmp_path: Path) -> Path:
    destination = tmp_path / "review"
    shutil.copytree(REVIEW_DIR, destination)
    return destination


def _read(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_bytes())
    assert isinstance(value, dict)
    return value


def _write(path: Path, value: dict[str, Any]) -> None:
    path.write_text(
        json.dumps(value, ensure_ascii=False, allow_nan=False, indent=2, sort_keys=True)
        + "\n",
        encoding="utf-8",
    )


def _mutate_json(
    review_dir: Path,
    name: str,
    mutation: Callable[[dict[str, Any]], None],
) -> None:
    path = review_dir / name
    value = _read(path)
    mutation(value)
    _write(path, value)


def _assert_rejected(review_dir: Path, code: str | None = None) -> None:
    result = _run_audit(review_dir)
    assert result.returncode == 3, (result.stdout, result.stderr)
    assert result.stdout == ""
    assert "release review audit failed" in result.stderr
    if code is not None:
        assert f"[{code}]" in result.stderr
    assert "Traceback" not in result.stderr


def _canonical_sha(value: Any) -> str:
    raw = json.dumps(
        value,
        ensure_ascii=False,
        allow_nan=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode()
    return hashlib.sha256(raw).hexdigest()


def test_release_review_baseline_passes_without_external_handoff_lookup() -> None:
    result = _run_audit(REVIEW_DIR)
    assert result.returncode == 0, result.stderr
    payload = json.loads(result.stdout)
    assert payload == {
        "frozen_commit": FROZEN_COMMIT,
        "release_ready": False,
        "release_version": "0.9.0",
        "review_completion_claimed": False,
        "status": "pass",
        "task_count": 240,
        "tracked_entry_count": 175,
    }


def test_generator_requires_operator_to_name_external_master_ledger(
    tmp_path: Path,
) -> None:
    result = subprocess.run(
        [
            sys.executable,
            os.fspath(GENERATOR),
            "--repo",
            os.fspath(ROOT),
            "--output-dir",
            os.fspath(tmp_path / "out"),
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 2
    assert "--master-ledger" in result.stderr
    assert not (tmp_path / "out").exists()


def test_intake_records_author_09_override_and_no_publication_identity() -> None:
    intake = _read(REVIEW_DIR / "intake.json")
    assert intake["author"]["name"] == "Sepehr Mahmoudian"
    assert intake["release"] == {
        "doi": None,
        "doi_status": "not_assigned_user_will_add_later",
        "nominal_handoff_target": "1.0.0",
        "override": "user_requested_0.9.0_review_release_before_1.0",
        "published": False,
        "requested_release": "0.9.0",
        "status": "review_intake_open_not_release_ready",
        "zenodo_record": None,
        "zenodo_status": "not_created_user_will_add_later",
    }
    assert intake["review"]["all_tasks_closed"] is False
    assert intake["review"]["closed_task_ids"] == []
    assert intake["review"]["human_review_complete"] is False
    assert intake["review"]["independent_review_complete"] is False
    assert intake["review"]["release_ready"] is False


def test_normalized_graph_is_contiguous_and_content_bound() -> None:
    ledger = _read(REVIEW_DIR / "master_task_ledger.normalized.json")
    tasks = ledger["tasks"]
    phases = ledger["phases"]
    assert [task["id"] for task in tasks] == [f"T{index:03d}" for index in range(240)]
    assert [phase["id"] for phase in phases] == [f"P{index:02d}" for index in range(16)]
    assert all(
        task["review_status"] == "open_imported_instruction_not_review_completion"
        for task in tasks
    )
    assert all(
        phase["review_status"] == "open_imported_instruction_not_review_completion"
        for phase in phases
    )
    assert ledger["review"]["all_tasks_closed"] is False
    assert ledger["review"]["closed_task_ids"] == []
    assert ledger["task_graph_sha256"] == _canonical_sha(
        {"phases": phases, "tasks": tasks}
    )
    assert ledger["source"]["sha256"] == (
        "384f5540dcdb4709b8f9add57e355761c6e076a1c4b22e26e42482bd0c0c4f29"
    )
    assert ledger["source"]["frozen_commit"] == FROZEN_COMMIT


def test_inventory_independently_matches_every_frozen_git_tree_entry() -> None:
    inventory = _read(REVIEW_DIR / "tracked_file_inventory.baseline.json")
    tree = subprocess.check_output(
        ["git", "-C", os.fspath(ROOT), "ls-tree", "-r", "-z", FROZEN_COMMIT]
    )
    expected_tree: list[tuple[str, str, str, str]] = []
    for raw_entry in tree.rstrip(b"\x00").split(b"\x00"):
        metadata, raw_path = raw_entry.split(b"\t", 1)
        mode, object_type, object_id = metadata.decode("ascii").split()
        expected_tree.append((raw_path.decode("utf-8"), mode, object_type, object_id))

    entries = inventory["entries"]
    assert len(entries) == len(expected_tree) == 175
    for entry, (path, mode, object_type, object_id) in zip(
        entries, expected_tree, strict=True
    ):
        assert entry["path"] == path
        assert entry["mode"] == mode
        assert entry["object_type"] == object_type
        assert entry["git_object_id"] == object_id
        assert entry["is_symlink"] is (mode == "120000")
        assert entry["is_executable"] is (mode == "100755")
        assert entry["review_status"] == "inventory_only_unreviewed"
        assert entry["human_reviewed"] is False
        assert entry["independent_reviewed"] is False
        assert type(entry["generated"]) is bool
        assert type(entry["public_surface"]) is bool
        assert type(entry["security_sensitive"]) is bool
        assert type(entry["science_sensitive"]) is bool
        assert isinstance(entry["category"], str) and entry["category"]
        if object_type == "commit":
            assert path == "pid-rs"
            assert object_id == PID_RS_COMMIT
            assert entry["git_blob_id"] is None
            assert entry["gitlink_commit"] == PID_RS_COMMIT
            assert entry["content_sha256"] is None
            assert entry["bytes"] is None
            assert entry["line_count"] is None
        else:
            content = subprocess.check_output(
                ["git", "-C", os.fspath(ROOT), "cat-file", "blob", object_id]
            )
            assert entry["git_blob_id"] == object_id
            assert entry["gitlink_commit"] is None
            assert entry["content_sha256"] == hashlib.sha256(content).hexdigest()
            assert entry["bytes"] == len(content)
            expected_lines = content.count(b"\n") + int(
                bool(content) and not content.endswith(b"\n")
            )
            assert entry["line_count"] == expected_lines


@pytest.mark.parametrize(
    ("field", "value", "code"),
    [
        (("author", "name"), "Someone Else", "INTAKE_AUTHOR"),
        (("release", "requested_release"), "1.0.0", "INTAKE_RELEASE"),
        (("frozen_baseline", "commit"), "0" * 40, "INTAKE_HEAD"),
        (("frozen_baseline", "pid_rs_gitlink_commit"), "0" * 40, "INTAKE_SUBMODULE"),
        (("release", "doi"), "10.0000/not-assigned", "INTAKE_PUBLICATION"),
        (("release", "zenodo_record"), "not-created", "INTAKE_PUBLICATION"),
    ],
)
def test_audit_rejects_wrong_intake_identity(
    tmp_path: Path,
    field: tuple[str, str],
    value: str,
    code: str,
) -> None:
    review = _copy_review(tmp_path)

    def mutate(document: dict[str, Any]) -> None:
        document[field[0]][field[1]] = value

    _mutate_json(review, "intake.json", mutate)
    _assert_rejected(review, code)


def test_audit_rejects_false_review_and_task_completion(tmp_path: Path) -> None:
    review = _copy_review(tmp_path)

    def mutate(document: dict[str, Any]) -> None:
        document["review"]["all_tasks_closed"] = True
        document["review"]["closed_task_ids"] = [
            f"T{index:03d}" for index in range(240)
        ]
        document["review"]["human_review_complete"] = True
        document["review"]["independent_review_complete"] = True
        document["review"]["release_ready"] = True

    _mutate_json(review, "intake.json", mutate)
    _assert_rejected(review, "FALSE_REVIEW")


def test_audit_rejects_false_file_review_completion(tmp_path: Path) -> None:
    review = _copy_review(tmp_path)

    def mutate(document: dict[str, Any]) -> None:
        document["review"]["reviewed_file_count"] = 175
        document["review"]["human_review_complete"] = True
        document["entries"][0]["review_status"] = "review_complete"
        document["entries"][0]["human_reviewed"] = True

    _mutate_json(review, "tracked_file_inventory.baseline.json", mutate)
    _assert_rejected(review, "FALSE_FILE_REVIEW")


def test_audit_rejects_duplicate_task_ids(tmp_path: Path) -> None:
    review = _copy_review(tmp_path)

    def mutate(document: dict[str, Any]) -> None:
        document["tasks"][1]["id"] = "T000"

    _mutate_json(review, "master_task_ledger.normalized.json", mutate)
    _assert_rejected(review, "TASK_DUPLICATE")


def test_audit_rejects_task_gap(tmp_path: Path) -> None:
    review = _copy_review(tmp_path)

    def mutate(document: dict[str, Any]) -> None:
        del document["tasks"][117]

    _mutate_json(review, "master_task_ledger.normalized.json", mutate)
    _assert_rejected(review, "TASK_COUNT")


def test_audit_rejects_unsafe_task_path_even_if_json_is_canonical(
    tmp_path: Path,
) -> None:
    review = _copy_review(tmp_path)

    def mutate(document: dict[str, Any]) -> None:
        document["tasks"][42]["mandatory_path_scopes"] = ["../escape"]

    _mutate_json(review, "master_task_ledger.normalized.json", mutate)
    _assert_rejected(review, "UNSAFE_PATH_SCOPE")


def test_audit_rejects_task_graph_drift_even_with_recomputed_adjacent_digest(
    tmp_path: Path,
) -> None:
    review = _copy_review(tmp_path)

    def mutate(document: dict[str, Any]) -> None:
        document["tasks"][12]["title"] = "Unreviewed replacement title."
        document["task_graph_sha256"] = _canonical_sha(
            {"phases": document["phases"], "tasks": document["tasks"]}
        )

    _mutate_json(review, "master_task_ledger.normalized.json", mutate)
    _assert_rejected(review, "TASK_GRAPH_DRIFT")


def test_audit_rejects_source_hash_drift(tmp_path: Path) -> None:
    review = _copy_review(tmp_path)

    def mutate(document: dict[str, Any]) -> None:
        document["source"]["sha256"] = "0" * 64

    _mutate_json(review, "master_task_ledger.normalized.json", mutate)
    _assert_rejected(review, "TASK_SOURCE")


def test_audit_rejects_inventory_submodule_drift(tmp_path: Path) -> None:
    review = _copy_review(tmp_path)

    def mutate(document: dict[str, Any]) -> None:
        gitlink = next(
            entry for entry in document["entries"] if entry["path"] == "pid-rs"
        )
        gitlink["git_object_id"] = "0" * 40
        gitlink["gitlink_commit"] = "0" * 40

    _mutate_json(review, "tracked_file_inventory.baseline.json", mutate)
    _assert_rejected(review, "INVENTORY_DRIFT")


def test_audit_rejects_noncanonical_hash_drift(tmp_path: Path) -> None:
    review = _copy_review(tmp_path)
    path = review / "intake.json"
    path.write_bytes(path.read_bytes() + b" ")
    _assert_rejected(review, "JSON_CANONICAL")


def test_audit_rejects_manifest_hash_drift(tmp_path: Path) -> None:
    review = _copy_review(tmp_path)

    def mutate(document: dict[str, Any]) -> None:
        document["artifacts"][0]["sha256"] = "0" * 64

    _mutate_json(review, "artifact_manifest.json", mutate)
    _assert_rejected(review, "MANIFEST_DRIFT")


def test_audit_rejects_duplicate_json_keys(tmp_path: Path) -> None:
    review = _copy_review(tmp_path)
    path = review / "intake.json"
    raw = path.read_text(encoding="utf-8")
    path.write_text(
        raw.replace("{\n", '{\n  "project": "duplicate",\n', 1), encoding="utf-8"
    )
    _assert_rejected(review, "JSON_DUPLICATE_KEY")


def test_audit_rejects_lone_surrogate_without_traceback(tmp_path: Path) -> None:
    review = _copy_review(tmp_path)
    path = review / "intake.json"
    path.write_bytes(b'{"bad":"\\ud800"}\n')
    _assert_rejected(review, "JSON_VALUE")


def test_audit_rejects_artifact_symlink(tmp_path: Path) -> None:
    review = _copy_review(tmp_path)
    intake = review / "intake.json"
    target = tmp_path / "intake-target.json"
    shutil.copyfile(intake, target)
    intake.unlink()
    intake.symlink_to(target)
    _assert_rejected(review, "ARTIFACT_SET")


def test_audit_rejects_unmanifested_review_artifact(tmp_path: Path) -> None:
    review = _copy_review(tmp_path)
    (review / "unmanifested.json").write_text("{}\n", encoding="utf-8")
    _assert_rejected(review, "ARTIFACT_SET")


def test_generator_check_and_writer_reject_extra_output_entries(tmp_path: Path) -> None:
    review = _copy_review(tmp_path)
    extra = review / "extra.txt"
    extra.write_text("not part of the review baseline\n", encoding="utf-8")

    result = _run_generator_check(review)
    assert result.returncode == 3, (result.stdout, result.stderr)
    assert result.stdout == ""
    assert "[ARTIFACT_SET]" in result.stderr
    assert extra.exists()

    generator = _load_generator()
    artifacts = {name: (REVIEW_DIR / name).read_bytes() for name in EXPECTED_NAMES}
    with pytest.raises(generator["ReleaseReviewError"]) as caught:
        generator["write_artifacts"](review, artifacts)
    assert caught.value.code == "ARTIFACT_SET"
    assert extra.exists()


def test_review_writer_installs_manifest_last_and_preserves_it_on_failure(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    generator = _load_generator()
    artifacts = {name: (REVIEW_DIR / name).read_bytes() for name in EXPECTED_NAMES}
    output = tmp_path / "review"
    real_replace = os.replace
    installed: list[str] = []

    def observe_replace(source: Path, destination: Path) -> None:
        installed.append(Path(destination).name)
        real_replace(source, destination)

    monkeypatch.setattr(generator["os"], "replace", observe_replace)
    generator["write_artifacts"](output, artifacts)
    assert installed == sorted(EXPECTED_NAMES - {MANIFEST_NAME}) + [MANIFEST_NAME]

    old_manifest = (output / MANIFEST_NAME).read_bytes()
    changed = {name: raw + b" " for name, raw in artifacts.items()}
    attempted: list[str] = []

    def fail_second_replace(source: Path, destination: Path) -> None:
        attempted.append(Path(destination).name)
        if len(attempted) == 2:
            raise OSError("injected mid-refresh failure")
        real_replace(source, destination)

    monkeypatch.setattr(generator["os"], "replace", fail_second_replace)
    with pytest.raises(generator["ReleaseReviewError"]) as caught:
        generator["write_artifacts"](output, changed)
    assert caught.value.code == "OUTPUT_WRITE"
    assert MANIFEST_NAME not in attempted
    assert (output / MANIFEST_NAME).read_bytes() == old_manifest
    assert not any(path.name.startswith(".") for path in output.iterdir())


def test_audit_rejects_oversize_artifact_without_reading_it(tmp_path: Path) -> None:
    review = _copy_review(tmp_path)
    with (review / "intake.json").open("wb") as handle:
        handle.truncate(64 * 1024 * 1024 + 1)
    _assert_rejected(review, "ARTIFACT_TOO_LARGE")


def test_bounded_reader_rejects_path_replacement_during_snapshot(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    generator = _load_generator()
    source = tmp_path / "source.json"
    replacement = tmp_path / "replacement.json"
    source.write_bytes(b"first")
    replacement.write_bytes(b"other")
    real_read = os.read
    real_replace = os.replace
    swapped = False

    def replace_after_read(descriptor: int, count: int) -> bytes:
        nonlocal swapped
        chunk = real_read(descriptor, count)
        if not swapped:
            swapped = True
            real_replace(replacement, source)
        return chunk

    monkeypatch.setattr(generator["os"], "read", replace_after_read)
    with pytest.raises(generator["ReleaseReviewError"]) as caught:
        generator["_read_bounded_regular"](
            source,
            max_bytes=32,
            path_code="TEST_PATH",
            read_code="TEST_RACE",
            too_large_code="TEST_SIZE",
            description="test input",
        )
    assert caught.value.code == "TEST_RACE"


def test_run_git_accepts_stdout_at_exact_budget(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    generator = _load_generator()
    _install_fake_git(tmp_path, monkeypatch)
    boundary = 4096
    monkeypatch.setenv("FAKE_GIT_STDOUT_BYTES", str(boundary))

    raw = generator["_run_git"](tmp_path, ["boundary"], max_bytes=boundary)

    assert raw == b"o" * boundary


def test_run_git_rejects_stdout_over_budget(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    generator = _load_generator()
    _install_fake_git(tmp_path, monkeypatch)
    boundary = 4096
    monkeypatch.setenv("FAKE_GIT_STDOUT_BYTES", str(boundary + 1))

    with pytest.raises(generator["ReleaseReviewError"]) as caught:
        generator["_run_git"](tmp_path, ["stdout-overflow"], max_bytes=boundary)

    assert caught.value.code == "GIT_OUTPUT"


def test_run_git_rejects_stderr_over_budget(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    generator = _load_generator()
    _install_fake_git(tmp_path, monkeypatch)
    stderr_budget = generator["MAX_GIT_STDERR_BYTES"]
    monkeypatch.setenv("FAKE_GIT_STDERR_BYTES", str(stderr_budget + 1))

    with pytest.raises(generator["ReleaseReviewError"]) as caught:
        generator["_run_git"](tmp_path, ["stderr-overflow"], max_bytes=16)

    assert caught.value.code == "GIT_STDERR"


def test_run_git_timeout_kills_and_reaps_process(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    generator = _load_generator()
    _install_fake_git(tmp_path, monkeypatch)
    monkeypatch.setenv("FAKE_GIT_SLEEP_SECONDS", "30")
    processes: list[subprocess.Popen[bytes]] = []
    real_popen = generator["subprocess"].Popen

    def capturing_popen(*args: Any, **kwargs: Any) -> subprocess.Popen[bytes]:
        process = real_popen(*args, **kwargs)
        processes.append(process)
        return process

    monkeypatch.setattr(generator["subprocess"], "Popen", capturing_popen)

    started = time.monotonic()
    with pytest.raises(generator["ReleaseReviewError"]) as caught:
        generator["_run_git"](tmp_path, ["timeout"], max_bytes=16, timeout_seconds=0.5)
    elapsed = time.monotonic() - started

    assert caught.value.code == "GIT_TIMEOUT"
    assert elapsed < 3.0
    assert len(processes) == 1
    assert processes[0].returncode is not None
    pid = processes[0].pid
    with pytest.raises(ChildProcessError):
        os.waitpid(pid, os.WNOHANG)
    with pytest.raises(ProcessLookupError):
        os.kill(pid, 0)


def test_run_git_reaps_descendants_after_success(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    generator = _load_generator()
    _install_fake_git(tmp_path, monkeypatch)
    marker = tmp_path / "descendant-escaped"
    monkeypatch.setenv("FAKE_GIT_DESCENDANT_MARKER", os.fspath(marker))

    assert generator["_run_git"](tmp_path, ["descendant"], max_bytes=16) == b""

    time.sleep(0.5)
    assert not marker.exists()


def test_run_git_selector_setup_failure_reaps_process(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    generator = _load_generator()
    _install_fake_git(tmp_path, monkeypatch)
    marker = tmp_path / "setup-escaped"
    monkeypatch.setenv("FAKE_GIT_SLEEP_SECONDS", "0.2")
    monkeypatch.setenv("FAKE_GIT_MARKER", os.fspath(marker))

    def fail_selector() -> None:
        raise RuntimeError("injected selector failure")

    monkeypatch.setattr(generator["selectors"], "DefaultSelector", fail_selector)
    with pytest.raises(RuntimeError, match="injected selector failure"):
        generator["_run_git"](tmp_path, ["setup-failure"], max_bytes=16)

    time.sleep(0.5)
    assert not marker.exists()
