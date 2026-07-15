from __future__ import annotations

import importlib.util
import shutil
import stat
import sys
from pathlib import Path

import pytest


SCRIPT = Path(__file__).resolve().parents[2] / "scripts" / "check_formal_models.py"
SPEC = importlib.util.spec_from_file_location("prisoma_check_formal_models", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
formal = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = formal
SPEC.loader.exec_module(formal)


@pytest.mark.skipif(shutil.which("z3") is None, reason="Z3 is not installed")
def test_registered_formal_obligations_hold() -> None:
    assert formal.main([]) == 0


def test_formal_registry_covers_every_smt_model() -> None:
    actual = {path.name for path in formal.FORMAL_DIR.glob("*.smt2")}
    assert actual == set(formal.EXPECTED)


def test_every_registered_source_has_exactly_the_registered_checks() -> None:
    for name, expected in formal.EXPECTED.items():
        model = formal._read_regular_model(formal.FORMAL_DIR / name)
        formal._validate_model_source(model, expected, name)
        commands = formal._top_level_commands(model)
        assert commands.count("check-sat") == len(expected)


def _fake_solver(
    tmp_path: Path,
    *,
    stdout: str,
    stderr: str = "",
    code: int = 0,
    delay_seconds: float = 0.0,
) -> Path:
    solver = tmp_path / "fake-z3"
    solver.write_text(
        "#!/usr/bin/env python3\n"
        "import sys, time\n"
        f"time.sleep({delay_seconds!r})\n"
        f"sys.stdout.write({stdout!r})\n"
        f"sys.stderr.write({stderr!r})\n"
        f"raise SystemExit({code})\n",
        encoding="utf-8",
    )
    solver.chmod(solver.stat().st_mode | stat.S_IXUSR)
    return solver


def _one_check_model(tmp_path: Path) -> Path:
    model = tmp_path / "one.smt2"
    model.write_text("(set-logic QF_LIA)\n(check-sat)\n", encoding="utf-8")
    return model


def test_source_audit_rejects_output_spoofing_command(tmp_path: Path) -> None:
    model = tmp_path / "spoof.smt2"
    model.write_text(
        '(set-logic QF_LIA)\n(echo "unsat")\n',
        encoding="utf-8",
    )
    with pytest.raises(RuntimeError, match="disallowed top-level commands"):
        formal.check_model("not-invoked", model, ("unsat",))


def test_runner_rejects_success_with_stderr(tmp_path: Path) -> None:
    solver = _fake_solver(tmp_path, stdout="unsat\n", stderr="warning\n")
    with pytest.raises(RuntimeError, match="unexpected stderr"):
        formal.check_model(str(solver), _one_check_model(tmp_path), ("unsat",))


def test_runner_requires_exact_solver_results(tmp_path: Path) -> None:
    solver = _fake_solver(tmp_path, stdout="sat\n")
    with pytest.raises(RuntimeError, match="returned"):
        formal.check_model(str(solver), _one_check_model(tmp_path), ("unsat",))


def test_runner_enforces_combined_output_bound(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    solver = _fake_solver(tmp_path, stdout="unsat\n")
    monkeypatch.setattr(formal, "MAX_OUTPUT_BYTES", 5)
    with pytest.raises(RuntimeError, match="output exceeded"):
        formal.check_model(str(solver), _one_check_model(tmp_path), ("unsat",))


def test_runner_enforces_wall_clock_deadline(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    solver = _fake_solver(tmp_path, stdout="unsat\n", delay_seconds=1.0)
    monkeypatch.setattr(formal, "TIMEOUT_SECONDS", 0.01)
    monkeypatch.setattr(formal, "WALLCLOCK_GRACE_SECONDS", 0.01)
    with pytest.raises(RuntimeError, match="timed out"):
        formal.check_model(str(solver), _one_check_model(tmp_path), ("unsat",))


def test_model_snapshot_rejects_final_symlink(tmp_path: Path) -> None:
    target = _one_check_model(tmp_path)
    link = tmp_path / "linked.smt2"
    link.symlink_to(target)
    with pytest.raises(RuntimeError, match="regular non-symlink"):
        formal._read_regular_model(link)
