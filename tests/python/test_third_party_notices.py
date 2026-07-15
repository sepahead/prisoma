"""Regression tests for deterministic direct-dependency notices."""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path
from types import SimpleNamespace


SCRIPT = (
    Path(__file__).resolve().parents[2] / "scripts" / "generate_third_party_notices.py"
)
SPEC = importlib.util.spec_from_file_location("prisoma_third_party_notices", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


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
        assert kwargs["check"] is True
        return SimpleNamespace(
            stdout=json.dumps(
                {"packages": [], "workspace_members": [], "resolve": {"nodes": []}}
            )
        )

    monkeypatch.setattr(MODULE.subprocess, "run", fake_run)
    assert MODULE.rust_direct_dependencies() == []
    assert observed == [
        "cargo",
        "metadata",
        "--locked",
        "--all-features",
        "--format-version",
        "1",
    ]
