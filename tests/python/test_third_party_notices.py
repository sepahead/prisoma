"""Regression tests for deterministic direct-dependency notices."""

from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from types import SimpleNamespace

import pytest


SCRIPT = (
    Path(__file__).resolve().parents[2] / "scripts" / "generate_third_party_notices.py"
)
SPEC = importlib.util.spec_from_file_location("prisoma_third_party_notices", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


def test_notice_snapshot_uses_nofollow_nonblocking_open(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    source = tmp_path / "source.txt"
    source.write_bytes(b"bounded\n")
    real_open = MODULE.os.open
    captured_flags: list[int] = []

    def recording_open(path: object, flags: int, mode: int = 0o777) -> int:
        captured_flags.append(flags)
        return real_open(path, flags, mode)

    monkeypatch.setattr(MODULE.os, "open", recording_open)
    assert (
        MODULE._read_regular_bytes(source, max_bytes=32, label="fixture")
        == b"bounded\n"
    )
    assert len(captured_flags) == 1
    assert captured_flags[0] & MODULE.os.O_NOFOLLOW
    assert captured_flags[0] & MODULE.os.O_NONBLOCK


def test_notice_snapshot_rejects_final_lexical_path_swap(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    source = tmp_path / "source.txt"
    replacement = tmp_path / "replacement.txt"
    source.write_bytes(b"bounded\n")
    replacement.write_bytes(b"bounded\n")
    real_lstat = MODULE.Path.lstat
    source_calls = 0

    def swapped_lstat(candidate: Path) -> object:
        nonlocal source_calls
        if candidate == source:
            source_calls += 1
            if source_calls == 2:
                return real_lstat(replacement)
        return real_lstat(candidate)

    monkeypatch.setattr(MODULE.Path, "lstat", swapped_lstat)
    with pytest.raises(MODULE.NoticeGenerationError, match="changed while reading"):
        MODULE._read_regular_bytes(source, max_bytes=32, label="fixture")


def test_numpy_marker_split_is_not_collapsed() -> None:
    rows = dict(MODULE.python_direct_dependencies())
    assert rows["numpy"] == (
        "2.4.6 [python_full_version < '3.12']; 2.5.1 [python_full_version >= '3.12']"
    )


def test_cargo_metadata_is_lockfile_constrained_and_resolves_all_features(
    monkeypatch,
) -> None:
    observed: list[str] = []

    def fake_run(command, **kwargs):
        observed.extend(command)
        assert kwargs["cwd"] == MODULE.REPO_ROOT
        return SimpleNamespace(
            stdout=json.dumps(
                {"packages": [], "workspace_members": [], "resolve": {"nodes": []}}
            )
        )

    monkeypatch.setattr(MODULE, "_run_bounded", fake_run)
    assert MODULE.rust_direct_dependencies() == []
    assert observed == [
        "cargo",
        "metadata",
        "--locked",
        "--all-features",
        "--format-version",
        "1",
    ]


def test_maintenance_subprocess_has_output_and_time_budgets(tmp_path: Path) -> None:
    with pytest.raises(MODULE.NoticeGenerationError, match="aggregate 32-byte"):
        MODULE._run_bounded(
            [sys.executable, "-c", "import sys; sys.stdout.write('x' * 128)"],
            cwd=tmp_path,
            timeout_seconds=2,
            max_output_bytes=32,
        )

    with pytest.raises(subprocess.TimeoutExpired):
        MODULE._run_bounded(
            [sys.executable, "-c", "import time; time.sleep(2)"],
            cwd=tmp_path,
            timeout_seconds=0.05,
            max_output_bytes=32,
        )


@pytest.mark.skipif(os.name != "posix", reason="descriptor-close check requires POSIX")
def test_maintenance_subprocess_allows_cleanup_after_both_pipes_close(
    tmp_path: Path,
) -> None:
    marker = tmp_path / "cleanup-finished"
    program = (
        "import os, pathlib, sys, time; "
        "os.close(1); os.close(2); time.sleep(0.05); "
        "pathlib.Path(sys.argv[1]).write_text('done', encoding='utf-8')"
    )

    result = MODULE._run_bounded(
        [sys.executable, "-c", program, os.fspath(marker)],
        cwd=tmp_path,
        timeout_seconds=2,
        max_output_bytes=32,
    )

    assert result.returncode == 0
    assert marker.read_text(encoding="utf-8") == "done"


@pytest.mark.skipif(os.name != "posix", reason="process-group check requires POSIX")
def test_maintenance_subprocess_reaps_process_group_after_setup_failure(
    tmp_path: Path, monkeypatch
) -> None:
    setup_marker = tmp_path / "setup-escaped"
    delayed_marker = (
        "import pathlib, sys, time; time.sleep(0.2); "
        "pathlib.Path(sys.argv[1]).write_text('escaped', encoding='utf-8')"
    )

    def fail_selector() -> None:
        raise RuntimeError("injected selector failure")

    monkeypatch.setattr(MODULE.selectors, "DefaultSelector", fail_selector)
    with pytest.raises(RuntimeError, match="injected selector failure"):
        MODULE._run_bounded(
            [sys.executable, "-c", delayed_marker, os.fspath(setup_marker)],
            cwd=tmp_path,
            timeout_seconds=2,
            max_output_bytes=32,
        )
    time.sleep(0.5)
    assert not setup_marker.exists()


def test_notice_write_is_atomic_and_rejects_symlink_targets(
    tmp_path: Path, monkeypatch
) -> None:
    target = tmp_path / "notices.md"
    target.write_text("old\n", encoding="utf-8")

    def fail_replace(_source, _destination) -> None:
        raise OSError("injected replacement failure")

    monkeypatch.setattr(MODULE.os, "replace", fail_replace)
    with pytest.raises(OSError, match="injected replacement failure"):
        MODULE._atomic_write(target, "new\n")
    assert target.read_text(encoding="utf-8") == "old\n"
    assert list(tmp_path.glob(".notices.md.*")) == []

    monkeypatch.undo()
    real = tmp_path / "real.md"
    real.write_text("real\n", encoding="utf-8")
    target.unlink()
    target.symlink_to(real)
    with pytest.raises(MODULE.NoticeGenerationError, match="non-symlink"):
        MODULE._atomic_write(target, "replacement\n")
    assert real.read_text(encoding="utf-8") == "real\n"


def test_notice_write_opens_parent_as_nofollow_nonblocking_directory(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    target = tmp_path / "notices.md"
    real_open = MODULE.os.open
    captured: list[tuple[Path, int]] = []

    def recording_open(path: object, flags: int, mode: int = 0o777) -> int:
        captured.append((Path(path), flags))
        return real_open(path, flags, mode)

    monkeypatch.setattr(MODULE.os, "open", recording_open)
    MODULE._atomic_write(target, "notices\n")

    directory_flags = [flags for path, flags in captured if path == tmp_path]
    assert len(directory_flags) == 1
    assert directory_flags[0] & MODULE.os.O_DIRECTORY
    assert directory_flags[0] & MODULE.os.O_NOFOLLOW
    assert directory_flags[0] & MODULE.os.O_NONBLOCK


def test_cargo_metadata_rejects_duplicate_keys_and_bad_schema(monkeypatch) -> None:
    monkeypatch.setattr(
        MODULE,
        "_run_bounded",
        lambda *_args, **_kwargs: SimpleNamespace(
            stdout='{"packages": [], "packages": [], "workspace_members": [], '
            '"resolve": {"nodes": []}}'
        ),
    )
    with pytest.raises(MODULE.NoticeGenerationError, match="duplicate key"):
        MODULE._cargo_metadata()

    monkeypatch.setattr(
        MODULE,
        "_run_bounded",
        lambda *_args, **_kwargs: SimpleNamespace(
            stdout='{"packages": {}, "workspace_members": [], "resolve": {"nodes": []}}'
        ),
    )
    with pytest.raises(MODULE.NoticeGenerationError, match="top-level schema"):
        MODULE._cargo_metadata()


def test_python_dependency_toml_schema_fails_closed(
    tmp_path: Path, monkeypatch
) -> None:
    (tmp_path / "pyproject.toml").write_text(
        '[project]\ndependencies = "numpy"\n', encoding="utf-8"
    )
    monkeypatch.setattr(MODULE, "REPO_ROOT", tmp_path)
    with pytest.raises(MODULE.NoticeGenerationError, match="string list"):
        MODULE.python_direct_dependencies()
