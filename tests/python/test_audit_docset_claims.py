"""Regression tests for the cross-document drift checks."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

_AUDIT_PATH = Path(__file__).resolve().parents[2] / "scripts" / "audit_docset_claims.py"
_AUDIT_SPEC = importlib.util.spec_from_file_location(
    "prisoma_audit_docset_claims", _AUDIT_PATH
)
assert _AUDIT_SPEC is not None and _AUDIT_SPEC.loader is not None
_AUDIT_MODULE = importlib.util.module_from_spec(_AUDIT_SPEC)
sys.modules[_AUDIT_SPEC.name] = _AUDIT_MODULE
_AUDIT_SPEC.loader.exec_module(_AUDIT_MODULE)

audit_one = _AUDIT_MODULE.audit_one
build_section_catalog = _AUDIT_MODULE.build_section_catalog
ncp_manifest_pin = _AUDIT_MODULE.ncp_manifest_pin


def _write(tmp_path: Path, name: str, text: str) -> Path:
    path = tmp_path / name
    path.write_text(text, encoding="utf-8")
    return path


def _kinds(
    path: Path, *, catalog_paths: list[Path] | None = None, ncp_pin: str = ""
) -> list[str]:
    catalog = build_section_catalog(catalog_paths or [path])
    return [
        finding.kind
        for finding in audit_one(
            path, section_catalog=catalog, expected_ncp_pin=ncp_pin
        )
    ]


def test_mermaid_direct_control_edge_is_rejected_but_bridge_route_passes(
    tmp_path: Path,
):
    bad = _write(
        tmp_path,
        "DIAGRAMS.md",
        """\
# Architecture

```mermaid
flowchart LR
    UI["Tauri GUI"] --> Physics["Physics backend"]
```
""",
    )
    assert "agent_bridge_bypass_edge" in _kinds(bad)

    good = _write(
        tmp_path,
        "ARCHITECTURE.md",
        """\
# Architecture

```mermaid
flowchart LR
    UI["Tauri GUI"] --> Bridge["Agent Bridge"]
    Bridge --> Physics["Physics backend"]
    Observer["Read-only observer"] --> Log["Run log"]
```
""",
    )
    assert "agent_bridge_bypass_edge" not in _kinds(good)


def test_sequence_diagram_and_prose_control_bypasses_are_rejected(tmp_path: Path):
    diagram = _write(
        tmp_path,
        "DIAGRAMS.md",
        """\
# Architecture

```mermaid
sequenceDiagram
    participant P as VLA policy
    participant S as Physics simulator
    P->S: apply action
```
""",
    )
    assert "agent_bridge_bypass_edge" in _kinds(diagram)

    prose = _write(
        tmp_path,
        "ARCHITECTURE.md",
        "The VLA policy sends each action directly to the simulator.\n",
    )
    assert "agent_bridge_bypass_wording" in _kinds(prose)

    explicit_bypass = _write(
        tmp_path,
        "README.md",
        "The UI directly controls physics, bypassing the Agent Bridge.\n",
    )
    assert "agent_bridge_bypass_wording" in _kinds(explicit_bypass)

    noun_claim = _write(
        tmp_path,
        "pidsplatspecs.md",
        "The GUI exposes direct simulator controls.\n",
    )
    assert "agent_bridge_bypass_wording" in _kinds(noun_claim)


def test_unrelated_negation_history_or_bridge_mention_does_not_hide_bypass(
    tmp_path: Path,
):
    claims = [
        "The UI controls physics directly. The Agent Bridge only records telemetry.\n",
        "No observer is present, but the UI controls physics directly.\n",
        "Unlike the historical design, the UI controls physics directly.\n",
    ]
    for index, claim in enumerate(claims):
        path = _write(tmp_path, f"bypass-{index}.md", claim)
        assert "agent_bridge_bypass_wording" in _kinds(path)


def test_hard_wrapped_control_claim_is_rejected(tmp_path: Path):
    path = _write(
        tmp_path,
        "ARCHITECTURE.md",
        "The VLA sends each action directly\n  to the physics simulator.\n",
    )
    assert "agent_bridge_bypass_wording" in _kinds(path)


def test_mermaid_forward_declarations_are_resolved_before_edges(tmp_path: Path):
    path = _write(
        tmp_path,
        "DIAGRAMS.md",
        """\
```mermaid
flowchart LR
    A --> B
    A["UI client"]
    B["Physics backend"]
```
""",
    )
    assert "agent_bridge_bypass_edge" in _kinds(path)


def test_negative_and_agent_bridge_control_wording_passes(tmp_path: Path):
    path = _write(
        tmp_path,
        "ARCHITECTURE.md",
        """\
The UI must never control physics directly.
The VLA submits actions through the Agent Bridge before backend dispatch.
""",
    )
    assert "agent_bridge_bypass_wording" not in _kinds(path)


def test_invented_nerfstudio_spz_exports_are_rejected_even_in_code_fences(
    tmp_path: Path,
):
    command = _write(
        tmp_path,
        "ARCHITECTURE.md",
        """\
# Export

```bash
ns-export gaussian-splat --load-config outputs/config.yml --output-format spz
```
""",
    )
    assert "invented_nerfstudio_spz_export" in _kinds(command)

    continued_command = _write(
        tmp_path,
        "EXPERIMENTS.md",
        """\
```bash
ns-export gaussian-splat \\
  --output-format spz
```
""",
    )
    assert "invented_nerfstudio_spz_export" in _kinds(continued_command)

    prose = _write(
        tmp_path,
        "README.md",
        "Nerfstudio natively exports SPZ for the runtime.\n",
    )
    assert "invented_nerfstudio_spz_export" in _kinds(prose)

    false_contrast = _write(
        tmp_path,
        "pidsplatspecs.md",
        "Nerfstudio exports SPZ, not PLY.\n",
    )
    assert "invented_nerfstudio_spz_export" in _kinds(false_contrast)


def test_spz_disclaimer_and_historical_export_note_pass(tmp_path: Path):
    path = _write(
        tmp_path,
        "ARCHITECTURE.md",
        """\
# Current architecture

Do not pass an invented Nerfstudio `--output-format spz` flag; use a separate converter.

## Version History

The old draft used `ns-export gaussian-splat --output-format spz`.
""",
    )
    assert "invented_nerfstudio_spz_export" not in _kinds(path)

    accurate = _write(
        tmp_path,
        "pidsplatspecs.md",
        "Nerfstudio exports PLY; our separate converter writes SPZ.\n",
    )
    assert "invented_nerfstudio_spz_export" not in _kinds(accurate)

    accurate_conjunction = _write(
        tmp_path,
        "PIPELINE.md",
        "Nerfstudio exports PLY, and our converter writes SPZ.\n",
    )
    assert "invented_nerfstudio_spz_export" not in _kinds(accurate_conjunction)

    other_tool = _write(
        tmp_path,
        "OTHER.md",
        "converter --output-format spz\n",
    )
    assert "invented_nerfstudio_spz_export" not in _kinds(other_tool)


def test_dead_active_section_reference_is_rejected(tmp_path: Path):
    grandplan = _write(
        tmp_path,
        "grandplan.md",
        """\
# 10. World model

## 10.10 Unified architecture

The active integration instructions are in §10.11.
""",
    )
    kinds = _kinds(grandplan)
    assert kinds.count("dead_section_reference") == 1


def test_live_explicit_and_historical_section_references_pass(tmp_path: Path):
    grandplan = _write(
        tmp_path,
        "grandplan.md",
        """\
# 10. World model

## 10.10 Unified architecture
""",
    )
    experiments = _write(
        tmp_path,
        "EXPERIMENTS.md",
        """\
# Experiments

## 0.2 Runbook
""",
    )
    readme = _write(
        tmp_path,
        "README.md",
        "See `grandplan.md` §10.10 and the `EXPERIMENTS.md` §0.2 runbook.\n",
    )
    history = _write(
        tmp_path,
        "findings.md",
        """\
# Findings

## Version History

The old release pointed to §10.11.
""",
    )
    catalog_paths = [grandplan, experiments, readme, history]
    assert "dead_section_reference" not in _kinds(readme, catalog_paths=catalog_paths)
    assert "dead_section_reference" not in _kinds(history, catalog_paths=catalog_paths)

    post_qualified = _write(
        tmp_path,
        "POST.md",
        "See §0.2 in EXPERIMENTS.md before running the harness.\n",
    )
    catalog_paths.append(post_qualified)
    assert "dead_section_reference" not in _kinds(
        post_qualified, catalog_paths=catalog_paths
    )


def test_bare_appendix_references_are_checked(tmp_path: Path):
    grandplan = _write(
        tmp_path,
        "grandplan.md",
        """\
# Specification

## §A. Architecture

The live blueprint is §A, but §M does not exist.
""",
    )
    kinds = _kinds(grandplan)
    assert kinds.count("dead_section_reference") == 1

    experiments = _write(tmp_path, "EXPERIMENTS.md", "## 0.2 Runbook\n")
    mixed = _write(
        tmp_path,
        "MIXED.md",
        "See §0.2 in EXPERIMENTS.md; §A is live and §M is dead.\n",
    )
    kinds = _kinds(mixed, catalog_paths=[grandplan, experiments, mixed])
    assert kinds.count("dead_section_reference") == 1


def test_ncp_claim_is_compared_to_shared_manifest_tag(tmp_path: Path):
    manifest = _write(
        tmp_path,
        "Cargo.toml",
        """\
[dependencies]
ncp-core = { git = "https://github.com/sepahead/NCP", tag = "v0.6.0" }
ncp-zenoh = { git = "https://github.com/sepahead/NCP", tag = "v0.6.0" }
""",
    )
    pin, manifest_findings = ncp_manifest_pin(manifest)
    assert pin == "v0.6.0"
    assert manifest_findings == []

    current = _write(tmp_path, "README.md", "The optional NCP pin is v0.5.3.\n")
    assert "ncp_pin_mismatch" in _kinds(current, ncp_pin=pin)

    corrected = _write(tmp_path, "DIAGRAMS.md", "The NCP tap is pinned to v0.6.0.\n")
    assert "ncp_pin_mismatch" not in _kinds(corrected, ncp_pin=pin)


def test_historical_ncp_pins_do_not_conflict_with_current_manifest(tmp_path: Path):
    path = _write(
        tmp_path,
        "grandplan.md",
        """\
**v10.5 notes (historical cut):**
- NCP observer pin was v0.5.2.

# 1. Current design

The NCP observer is pinned to v0.6.0.
""",
    )
    assert "ncp_pin_mismatch" not in _kinds(path, ncp_pin="v0.6.0")


def test_hard_wrapped_ncp_pin_is_checked(tmp_path: Path):
    path = _write(
        tmp_path,
        "README.md",
        "The optional NCP dependency is pinned to\n  v0.5.3.\n",
    )
    assert "ncp_pin_mismatch" in _kinds(path, ncp_pin="v0.6.0")

    unrelated = _write(
        tmp_path,
        "INTEGRATION.md",
        "NCP integrates with pid-rs v0.4.0.\n",
    )
    assert "ncp_pin_mismatch" not in _kinds(unrelated, ncp_pin="v0.6.0")


def test_inconsistent_ncp_manifest_pins_are_reported(tmp_path: Path):
    manifest = _write(
        tmp_path,
        "Cargo.toml",
        """\
[dependencies]
ncp-core = { git = "https://github.com/sepahead/NCP", tag = "v0.6.0" }
ncp-zenoh = { git = "https://github.com/sepahead/NCP", tag = "v0.5.3" }
""",
    )
    pin, findings = ncp_manifest_pin(manifest)
    assert pin == ""
    assert [finding.kind for finding in findings] == ["ncp_manifest_pin_inconsistent"]


def test_changelog_is_excluded_from_new_invariant_checks(tmp_path: Path):
    changelog = _write(
        tmp_path,
        "CHANGELOG.md",
        """\
# Changelog

- Old UI directly controlled the physics simulator.
- Old docs used `--output-format spz` and claimed NCP v0.5.2.
- Old docs linked §10.11.
""",
    )
    protected_kinds = {
        "agent_bridge_bypass_wording",
        "invented_nerfstudio_spz_export",
        "ncp_pin_mismatch",
        "dead_section_reference",
    }
    assert protected_kinds.isdisjoint(_kinds(changelog, ncp_pin="v0.6.0"))
