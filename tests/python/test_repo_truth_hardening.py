"""Fail-closed boundary tests for the repository-truth maintenance audit."""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

import pytest


SCRIPT = Path(__file__).resolve().parents[2] / "scripts" / "audit_repo_truth.py"
SPEC = importlib.util.spec_from_file_location("prisoma_audit_repo_truth", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


def test_git_subprocess_has_output_and_time_budgets(
    tmp_path: Path, monkeypatch
) -> None:
    monkeypatch.setattr(MODULE, "ROOT", tmp_path)
    with pytest.raises(MODULE.TruthAuditError, match="aggregate 32-byte"):
        MODULE._run_bounded(
            [sys.executable, "-c", "import sys; sys.stderr.write('x' * 128)"],
            timeout_seconds=2,
            max_output_bytes=32,
        )
    with pytest.raises(subprocess.TimeoutExpired):
        MODULE._run_bounded(
            [sys.executable, "-c", "import time; time.sleep(2)"],
            timeout_seconds=0.05,
            max_output_bytes=32,
        )


def test_json_reader_rejects_duplicate_constants_symlinks_and_oversize(
    tmp_path: Path, monkeypatch
) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"a": 1, "a": 2}\n', encoding="utf-8")
    with pytest.raises(MODULE.TruthAuditError, match="duplicate JSON key"):
        MODULE._json_object(duplicate, label="fixture")

    constant = tmp_path / "constant.json"
    constant.write_text('{"a": NaN}\n', encoding="utf-8")
    with pytest.raises(MODULE.TruthAuditError, match="invalid JSON constant"):
        MODULE._json_object(constant, label="fixture")

    link = tmp_path / "link.json"
    link.symlink_to(duplicate)
    with pytest.raises(MODULE.TruthAuditError, match="non-symlink"):
        MODULE._json_object(link, label="fixture")

    monkeypatch.setattr(MODULE, "MAX_REPO_FILE_BYTES", 4)
    with pytest.raises(MODULE.TruthAuditError, match="4-byte limit"):
        MODULE._json_object(duplicate, label="fixture")


def test_overlay_path_is_confined_and_symlink_free(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setattr(MODULE, "ROOT", tmp_path)
    inside = tmp_path / "inside.csv"
    inside.write_text("name\nvalue\n", encoding="utf-8")
    assert MODULE._repo_relative_path("inside.csv", label="fixture") == inside

    with pytest.raises(MODULE.TruthAuditError, match="escapes"):
        MODULE._repo_relative_path("../outside.csv", label="fixture")

    alias = tmp_path / "alias.csv"
    alias.symlink_to(inside)
    with pytest.raises(MODULE.TruthAuditError, match="symlink"):
        MODULE._repo_relative_path("alias.csv", label="fixture")


def test_toml_reader_rejects_malformed_input(tmp_path: Path) -> None:
    malformed = tmp_path / "bad.toml"
    malformed.write_text("[broken\n", encoding="utf-8")
    with pytest.raises(MODULE.TruthAuditError, match="cannot parse"):
        MODULE._toml_object(malformed, label="fixture")


def test_cli_converts_malformed_input_to_a_failed_audit(monkeypatch, capsys) -> None:
    monkeypatch.setattr(
        MODULE,
        "_audit",
        lambda: (_ for _ in ()).throw(MODULE.TruthAuditError("bad fixture")),
    )
    assert MODULE.main() == 1
    assert "audit input invalid or unavailable: bad fixture" in capsys.readouterr().out
