from __future__ import annotations

import hashlib
import importlib.util
import os
import re
import shutil
import stat
import sys
import time
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


def test_formal_registry_binds_every_model_digest() -> None:
    assert set(formal.MODEL_SHA256) == set(formal.EXPECTED)
    for name, digest in formal.MODEL_SHA256.items():
        model = formal._read_regular_model(formal.FORMAL_DIR / name)
        assert hashlib.sha256(model).hexdigest() == digest


def test_registry_snapshot_rejects_final_symlink(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    target = tmp_path / "registry.json"
    target.write_text("{}", encoding="utf-8")
    link = tmp_path / "registry-link.json"
    link.symlink_to(target)
    monkeypatch.setattr(formal, "REGISTRY_PATH", link)
    with pytest.raises(RuntimeError, match="regular non-symlink"):
        formal._load_registry()


def test_registry_snapshot_rejects_oversized_input(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    registry = tmp_path / "registry.json"
    registry.write_bytes(b"x" * 33)
    monkeypatch.setattr(formal, "REGISTRY_PATH", registry)
    monkeypatch.setattr(formal, "MAX_MODEL_BYTES", 32)
    with pytest.raises(RuntimeError, match="exceeds"):
        formal._load_registry()


def test_registry_snapshot_rejects_duplicate_json_members(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    registry = tmp_path / "registry.json"
    registry.write_text(
        '{"schema_version":1,"schema_version":1,'
        '"z3_version":"Z3 version 4.16.0 - 64 bit","models":{}}',
        encoding="utf-8",
    )
    monkeypatch.setattr(formal, "REGISTRY_PATH", registry)
    with pytest.raises(RuntimeError, match="duplicate JSON member 'schema_version'"):
        formal._load_registry()


def test_typed_outcome_domains_track_the_rust_contract() -> None:
    rust_source = (
        formal.ROOT / "crates" / "pid-sim" / "src" / "offline_harness.rs"
    ).read_text(encoding="utf-8")

    def enum_variants(name: str) -> tuple[str, ...]:
        match = re.search(
            rf"pub enum {name}\s*\{{(?P<body>.*?)\n\}}",
            rust_source,
            flags=re.DOTALL,
        )
        assert match is not None
        return tuple(
            re.findall(
                r"^\s*([A-Z][A-Za-z0-9_]*)\s*,\s*$",
                match.group("body"),
                flags=re.MULTILINE,
            )
        )

    assert enum_variants("OfflineVldaEstimateStatus") == (
        "NotRequested",
        "Produced",
        "ProducedWithWarning",
        "Abstained",
    )
    assert enum_variants("OfflineVldaScientificGateVerdict") == (
        "Passed",
        "Conditional",
        "NotEvaluated",
        "Blocked",
        "NotApplicable",
    )

    model = (formal.FORMAL_DIR / "typed_outcome_publication.smt2").read_text(
        encoding="utf-8"
    )
    assert (
        "status: 0=not_requested, 1=produced, 2=produced_with_warning, 3=abstained"
        in model
    )
    assert (
        "0=passed, 1=conditional, 2=not_evaluated, 3=blocked, 4=not_applicable" in model
    )
    assert "(define-fun valid-status ((s Int)) Bool (and (<= 0 s) (<= s 3)))" in model
    assert "(define-fun valid-gate ((g Int)) Bool (and (<= 0 g) (<= g 4)))" in model
    assert "(all-gates-equal 4)" in model
    assert "(all-gates-equal 0)" in model


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
        f"#!{sys.executable}\n"
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


def _validate_unregistered(source: bytes, expected: tuple[str, ...]) -> None:
    formal._validate_model_source(
        source, expected, "unregistered.smt2", verify_digest=False
    )


def test_source_audit_rejects_output_spoofing_command(tmp_path: Path) -> None:
    model = tmp_path / "spoof.smt2"
    model.write_text(
        '(set-logic QF_LIA)\n(echo "unsat")\n',
        encoding="utf-8",
    )
    with pytest.raises(RuntimeError, match="disallowed top-level commands"):
        formal.check_model("not-invoked", model, ("unsat",), verify_digest=False)


@pytest.mark.parametrize(
    "source, message",
    [
        (
            b"(set-logic QF_LIA)\n(push)\n(check-sat)\n(pop 1)\n",
            "canonical single-level",
        ),
        (
            b"(set-logic QF_LIA)\n(push 1)\n(check-sat)\n(pop 0)\n",
            "canonical single-level",
        ),
        (
            b"(set-logic QF_LIA)\n(push 2)\n(check-sat)\n(pop 2)\n",
            "canonical single-level",
        ),
        (
            b"(set-logic QF_LIA extra)\n(check-sat)\n",
            "malformed set-logic",
        ),
        (
            b"(set-logic QF_LIA)\n(check-sat true)\n",
            "zero-argument",
        ),
    ],
)
def test_source_audit_rejects_command_arity(source: bytes, message: str) -> None:
    with pytest.raises(RuntimeError, match=message):
        _validate_unregistered(source, ("sat",))


def test_source_audit_rejects_pop_zero_scope_leakage() -> None:
    source = (
        b"(set-logic QF_LIA)\n"
        b"(push 1)\n(assert false)\n(check-sat)\n(pop 0)\n(check-sat)\n"
    )
    with pytest.raises(RuntimeError, match="canonical single-level"):
        _validate_unregistered(source, ("unsat", "sat"))


def test_source_audit_rejects_digest_drift() -> None:
    name = next(iter(formal.EXPECTED))
    model = formal._read_regular_model(formal.FORMAL_DIR / name) + b"\n; drift\n"
    with pytest.raises(RuntimeError, match="digest"):
        formal._validate_model_source(model, formal.EXPECTED[name], name)


def test_runner_rejects_success_with_stderr(tmp_path: Path) -> None:
    solver = _fake_solver(tmp_path, stdout="unsat\n", stderr="warning\n")
    with pytest.raises(RuntimeError, match="unexpected stderr"):
        formal.check_model(
            str(solver), _one_check_model(tmp_path), ("unsat",), verify_digest=False
        )


def test_runner_requires_exact_solver_results(tmp_path: Path) -> None:
    solver = _fake_solver(tmp_path, stdout="sat\n")
    with pytest.raises(RuntimeError, match="returned"):
        formal.check_model(
            str(solver), _one_check_model(tmp_path), ("unsat",), verify_digest=False
        )


def test_runner_enforces_combined_output_bound(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    solver = _fake_solver(tmp_path, stdout="unsat\n")
    monkeypatch.setattr(formal, "MAX_OUTPUT_BYTES", 5)
    with pytest.raises(RuntimeError, match="output exceeded"):
        formal.check_model(
            str(solver), _one_check_model(tmp_path), ("unsat",), verify_digest=False
        )


def test_runner_enforces_wall_clock_deadline(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    solver = _fake_solver(tmp_path, stdout="unsat\n", delay_seconds=1.0)
    monkeypatch.setattr(formal, "TIMEOUT_SECONDS", 0.01)
    monkeypatch.setattr(formal, "WALLCLOCK_GRACE_SECONDS", 0.01)
    with pytest.raises(RuntimeError, match="timed out"):
        formal.check_model(
            str(solver), _one_check_model(tmp_path), ("unsat",), verify_digest=False
        )


def test_runner_reports_nonzero_exit_without_diagnostics(tmp_path: Path) -> None:
    solver = _fake_solver(tmp_path, stdout="", code=7)
    with pytest.raises(RuntimeError, match="exit status 7 without diagnostics"):
        formal.check_model(
            str(solver), _one_check_model(tmp_path), ("sat",), verify_digest=False
        )


def test_runner_rejects_non_ascii_output(tmp_path: Path) -> None:
    solver = tmp_path / "fake-z3"
    solver.write_bytes(b"#!/bin/sh\nprintf '\\377\\n'\n")
    solver.chmod(solver.stat().st_mode | stat.S_IXUSR)
    with pytest.raises(RuntimeError, match="non-ASCII"):
        formal.check_model(
            str(solver), _one_check_model(tmp_path), ("sat",), verify_digest=False
        )


def test_version_gate_rejects_a_different_solver(tmp_path: Path) -> None:
    solver = _fake_solver(tmp_path, stdout="Z3 version 4.8.12 - 64 bit\n")
    with pytest.raises(RuntimeError, match="unsupported Z3 version"):
        formal._require_z3_version(str(solver))


@pytest.mark.skipif(os.name != "posix", reason="process-group check requires POSIX")
def test_version_query_reaps_descendants_and_setup_failures(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    descendant_marker = tmp_path / "version-descendant-escaped"
    solver = tmp_path / "fake-z3-version"
    solver.write_text(
        f"#!{sys.executable}\n"
        "import subprocess, sys\n"
        "subprocess.Popen(\n"
        "    [sys.executable, '-c', "
        "'import pathlib, sys, time; time.sleep(0.2); "
        'pathlib.Path(sys.argv[1]).write_text("escaped", encoding="utf-8")\', '
        f"{os.fspath(descendant_marker)!r}],\n"
        "    stdin=subprocess.DEVNULL,\n"
        "    stdout=subprocess.DEVNULL,\n"
        "    stderr=subprocess.DEVNULL,\n"
        ")\n"
        f"print({formal.REQUIRED_Z3_VERSION!r})\n",
        encoding="utf-8",
    )
    solver.chmod(solver.stat().st_mode | stat.S_IXUSR)
    formal._require_z3_version(str(solver))
    time.sleep(0.5)
    assert not descendant_marker.exists()

    setup_marker = tmp_path / "version-setup-escaped"
    delayed_solver = tmp_path / "fake-z3-version-delayed"
    delayed_solver.write_text(
        f"#!{sys.executable}\n"
        "import pathlib, time\n"
        "time.sleep(0.2)\n"
        f"pathlib.Path({os.fspath(setup_marker)!r}).write_text("
        "'escaped', encoding='utf-8')\n"
        f"print({formal.REQUIRED_Z3_VERSION!r})\n",
        encoding="utf-8",
    )
    delayed_solver.chmod(delayed_solver.stat().st_mode | stat.S_IXUSR)

    def fail_selector() -> None:
        raise RuntimeError("injected selector failure")

    monkeypatch.setattr(formal.selectors, "DefaultSelector", fail_selector)
    with pytest.raises(RuntimeError, match="injected selector failure"):
        formal._require_z3_version(str(delayed_solver))
    time.sleep(0.5)
    assert not setup_marker.exists()


@pytest.mark.skipif(os.name != "posix", reason="process-group check requires POSIX")
def test_model_runner_reaps_descendants_after_success(tmp_path: Path) -> None:
    marker = tmp_path / "solver-descendant-escaped"
    solver = tmp_path / "fake-z3"
    solver.write_text(
        f"#!{sys.executable}\n"
        "import subprocess, sys\n"
        "subprocess.Popen(\n"
        "    [sys.executable, '-c', "
        "'import pathlib, sys, time; time.sleep(0.2); "
        'pathlib.Path(sys.argv[1]).write_text("escaped", encoding="utf-8")\', '
        f"{os.fspath(marker)!r}],\n"
        "    stdin=subprocess.DEVNULL,\n"
        "    stdout=subprocess.DEVNULL,\n"
        "    stderr=subprocess.DEVNULL,\n"
        ")\n"
        "print('unsat')\n",
        encoding="utf-8",
    )
    solver.chmod(solver.stat().st_mode | stat.S_IXUSR)
    formal.check_model(
        str(solver), _one_check_model(tmp_path), ("unsat",), verify_digest=False
    )
    time.sleep(0.5)
    assert not marker.exists()


@pytest.mark.skipif(os.name != "posix", reason="process-group check requires POSIX")
def test_group_signal_precedes_ownership_guard_reap(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    solver = _fake_solver(tmp_path, stdout="unsat\n")
    real_start = formal._start_owned_tool
    real_kill_group = formal._kill_owned_process_group
    owned: dict[str, object] = {}
    observations: list[str] = []

    def capture_start(
        command: list[str],
    ) -> tuple[object, object | None]:
        process, anchor = real_start(command)
        owned["process"] = process
        owned["anchor"] = anchor
        return process, anchor

    def assert_owned_before_signal(
        pgid: int, *, allow_darwin_empty_group: bool = False
    ) -> None:
        process = owned["process"]
        anchor = owned["anchor"]
        if anchor is not None:
            assert pgid == anchor.pid
            assert formal.os.getpgid(anchor.pid) == anchor.pid
            observations.append("live-anchor")
        else:
            assert pgid == process.pid
            if formal._posix_waitid_available():
                observation = formal.os.waitid(
                    formal.os.P_PID,
                    process.pid,
                    formal.os.WEXITED | formal.os.WNOHANG | formal.os.WNOWAIT,
                )
                assert observation is not None
                assert observation.si_pid == process.pid
                observations.append("waitid-wnowait")
            else:
                assert process.returncode is None
                observations.append("unreaped-session-leader")
        real_kill_group(
            pgid,
            allow_darwin_empty_group=allow_darwin_empty_group,
        )

    monkeypatch.setattr(formal, "_start_owned_tool", capture_start)
    monkeypatch.setattr(formal, "_kill_owned_process_group", assert_owned_before_signal)
    formal.check_model(
        str(solver), _one_check_model(tmp_path), ("unsat",), verify_digest=False
    )
    assert observations in (
        ["live-anchor"],
        ["waitid-wnowait"],
        ["unreaped-session-leader"],
    )


@pytest.mark.skipif(os.name != "posix", reason="process-group check requires POSIX")
def test_posix_without_waitid_uses_unreaped_session_leader(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    marker = tmp_path / "fallback-descendant-escaped"
    solver = tmp_path / "fake-z3"
    solver.write_text(
        f"#!{sys.executable}\n"
        "import subprocess, sys\n"
        "subprocess.Popen(\n"
        "    [sys.executable, '-c', "
        "'import pathlib, sys, time; time.sleep(0.2); "
        'pathlib.Path(sys.argv[1]).write_text("escaped", encoding="utf-8")\', '
        f"{os.fspath(marker)!r}],\n"
        "    stdin=subprocess.DEVNULL,\n"
        "    stdout=subprocess.DEVNULL,\n"
        "    stderr=subprocess.DEVNULL,\n"
        ")\n"
        "print('unsat')\n",
        encoding="utf-8",
    )
    solver.chmod(solver.stat().st_mode | stat.S_IXUSR)
    real_start = formal._start_owned_tool
    real_kill_group = formal._kill_owned_process_group
    owned: dict[str, object] = {}
    signaled = False

    def capture_start(
        command: list[str],
    ) -> tuple[object, object | None]:
        process, anchor = real_start(command)
        owned["process"] = process
        owned["anchor"] = anchor
        return process, anchor

    def assert_session_leader_owns_group(
        pgid: int, *, allow_darwin_empty_group: bool = False
    ) -> None:
        nonlocal signaled
        process = owned["process"]
        anchor = owned["anchor"]
        assert anchor is None
        assert pgid == process.pid
        assert process.returncode is None
        signaled = True
        real_kill_group(
            pgid,
            allow_darwin_empty_group=allow_darwin_empty_group,
        )

    monkeypatch.setattr(formal, "_posix_waitid_available", lambda: False)
    monkeypatch.setattr(formal, "_start_owned_tool", capture_start)
    monkeypatch.setattr(
        formal, "_kill_owned_process_group", assert_session_leader_owns_group
    )
    formal.check_model(
        str(solver), _one_check_model(tmp_path), ("unsat",), verify_digest=False
    )
    assert signaled
    time.sleep(0.5)
    assert not marker.exists()


@pytest.mark.skipif(shutil.which("z3") is None, reason="Z3 is not installed")
@pytest.mark.parametrize(
    "name, old, new, changed_index",
    [
        (
            "bridge_log_before_dispatch.smt2",
            "(and (= op 2) request_accepted (not safe_mode)",
            "(and (= op 2) (not safe_mode)",
            1,
        ),
        (
            "coupling_nonidentification.smt2",
            "(assert (= disagreement_b 1.0))",
            "(assert (= disagreement_b 0.0))",
            0,
        ),
        (
            "h2_paired_brier_bounds.smt2",
            "(assert (or (= y 0) (= y 1)))",
            "(assert true)",
            7,
        ),
        (
            "h2_paired_brier_bounds.smt2",
            "(assert (>= weight_a 0))",
            "(assert true)",
            9,
        ),
        (
            "h3_full_population_fallback.smt2",
            "(assert (= deployed_loss baseline_loss))",
            "(assert true)",
            1,
        ),
        (
            "informative_censoring_nonidentification.smt2",
            "(assert (= hidden_targets_b censored))",
            "(assert (= hidden_targets_b hidden_targets_a))",
            0,
        ),
        (
            "individual_effect_prevalence_nonidentification.smt2",
            "(assert (= (+ p01_b p10_b) 1.0))",
            "(assert (= (+ p01_b p10_b) 0.0))",
            0,
        ),
        (
            "pid_nonidentification.smt2",
            "(assert (= red_b (/ 3.0 4.0)))",
            "(assert (= red_b (/ 1.0 2.0)))",
            0,
        ),
        (
            "receipt_last_publication.smt2",
            "(=> receipt_step_confirmed receipt_path_visible)",
            "(and (=> receipt_step_confirmed receipt_path_visible)\n"
            "            (=> receipt_path_visible receipt_step_confirmed))",
            1,
        ),
        (
            "receipt_last_publication.smt2",
            "(= core_receipt_dataset_digest (digest prepared_dataset_bytes))",
            "true",
            4,
        ),
        (
            "receipt_last_publication.smt2",
            "(and outer-receipt-binding\n"
            "       (= reread_dataset_bytes installed_dataset_bytes)",
            "(and outer-receipt-binding\n       true",
            9,
        ),
        (
            "shannon_two_source_identity.smt2",
            "(/ (+ (- joint i2) (- joint i1)) joint)",
            "(/ (- joint i2) joint)",
            1,
        ),
        (
            "typed_outcome_publication.smt2",
            "(= has_value (> metric_count 0))",
            "true",
            16,
        ),
        (
            "typed_outcome_publication.smt2",
            "(define-fun valid-gate ((g Int)) Bool (and (<= 0 g) (<= g 4)))",
            "(define-fun valid-gate ((g Int)) Bool (and (<= 0 g) (<= g 3)))",
            0,
        ),
        (
            "typed_outcome_publication.smt2",
            "(=> interpret (and (all-gates-equal 0) has_envelope))",
            "true",
            19,
        ),
    ],
)
def test_semantic_mutant_changes_registered_obligation(
    tmp_path: Path, name: str, old: str, new: str, changed_index: int
) -> None:
    source = (formal.FORMAL_DIR / name).read_text(encoding="utf-8")
    assert source.count(old) == 1
    mutant = source.replace(old, new)
    path = tmp_path / name
    path.write_text(mutant, encoding="utf-8")
    formal._validate_model_source(
        path.read_bytes(), formal.EXPECTED[name], name, verify_digest=False
    )
    z3 = shutil.which("z3")
    assert z3 is not None
    formal._require_z3_version(z3)
    actual = formal._run_model_results(
        z3,
        path,
        formal.EXPECTED[name],
        verify_digest=False,
    )
    assert len(actual) == len(formal.EXPECTED[name])
    assert actual[changed_index] != formal.EXPECTED[name][changed_index]


def test_model_snapshot_rejects_final_symlink(tmp_path: Path) -> None:
    target = _one_check_model(tmp_path)
    link = tmp_path / "linked.smt2"
    link.symlink_to(target)
    with pytest.raises(RuntimeError, match="regular non-symlink"):
        formal._read_regular_model(link)


def test_model_snapshot_rejects_oversized_input(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    model = tmp_path / "large.smt2"
    model.write_bytes(b"x" * 33)
    monkeypatch.setattr(formal, "MAX_MODEL_BYTES", 32)
    with pytest.raises(RuntimeError, match="exceeds"):
        formal._read_regular_model(model)


@pytest.mark.parametrize(
    "source, message",
    [
        (b"\xff", "valid UTF-8"),
        (b"(set-logic QF_LIA)\x00", "NUL byte"),
        (
            b"(set-logic QF_LIA)\n(declare-const |bad\\name| Bool)\n(check-sat)\n",
            "backslash in an SMT-LIB quoted symbol",
        ),
    ],
)
def test_source_audit_rejects_invalid_text(source: bytes, message: str) -> None:
    with pytest.raises(RuntimeError, match=message):
        _validate_unregistered(source, ("sat",))
