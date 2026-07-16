"""Fail-closed boundary tests for the repository-truth maintenance audit."""

from __future__ import annotations

import importlib.util
import os
import subprocess
import sys
import time
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


@pytest.mark.skipif(os.name != "posix", reason="process-group check requires POSIX")
def test_git_subprocess_reaps_descendants_and_setup_failures(
    tmp_path: Path, monkeypatch
) -> None:
    monkeypatch.setattr(MODULE, "ROOT", tmp_path)
    descendant_marker = tmp_path / "descendant-escaped"
    spawn_descendant = """
import subprocess
import sys

subprocess.Popen(
    [
        sys.executable,
        "-c",
        "import pathlib, sys, time; time.sleep(0.2); "
        "pathlib.Path(sys.argv[1]).write_text('escaped', encoding='utf-8')",
        sys.argv[1],
    ],
    stdin=subprocess.DEVNULL,
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
)
"""
    MODULE._run_bounded(
        [sys.executable, "-c", spawn_descendant, os.fspath(descendant_marker)],
        timeout_seconds=2,
        max_output_bytes=32,
    )
    time.sleep(0.5)
    assert not descendant_marker.exists()

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
            timeout_seconds=2,
            max_output_bytes=32,
        )
    time.sleep(0.5)
    assert not setup_marker.exists()


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


def test_gitlink_revision_requires_one_exact_stage_zero_entry(monkeypatch) -> None:
    revision = "5" * 40
    monkeypatch.setattr(
        MODULE,
        "git_output",
        lambda *args: f"160000 {revision} 0\tpid-rs",
    )
    assert MODULE.gitlink_revision() == revision

    for malformed in (
        "",
        f"100644 {revision} 0\tpid-rs",
        f"160000 {revision} 1\tpid-rs",
        f"160000 {'g' * 40} 0\tpid-rs",
        f"160000 {revision} 0\tpid-rs\n160000 {revision} 0\tpid-rs",
    ):
        monkeypatch.setattr(MODULE, "git_output", lambda *args, value=malformed: value)
        with pytest.raises(MODULE.TruthAuditError, match="pid-rs"):
            MODULE.gitlink_revision()


def test_root_release_identity_rejects_author_and_archive_drift(
    tmp_path: Path, monkeypatch
) -> None:
    monkeypatch.setattr(MODULE, "ROOT", tmp_path)
    (tmp_path / "release" / "0.9.0").mkdir(parents=True)
    (tmp_path / "scripts").mkdir()
    (tmp_path / "pyproject.toml").write_text(
        """
[project]
version = "0.9.0"
authors = [{ name = "Sepehr Mahmoudian" }]
[project.urls]
Homepage = "https://github.com/sepahead/prisoma"
Repository = "https://github.com/sepahead/prisoma"
[tool.uv]
required-version = "==0.11.28"
""".lstrip(),
        encoding="utf-8",
    )
    (tmp_path / "Cargo.toml").write_text(
        """
[workspace]
[workspace.package]
version = "0.9.0"
authors = ["Sepehr Mahmoudian"]
rust-version = "1.93"
repository = "https://github.com/sepahead/prisoma"
""".lstrip(),
        encoding="utf-8",
    )
    (tmp_path / ".github" / "workflows").mkdir(parents=True)
    ci = tmp_path / ".github" / "workflows" / "ci.yml"
    ci_text = """
steps:
  - uses: astral-sh/setup-uv@0123456789abcdef0123456789abcdef01234567
    with:
      version: "0.11.28"
  - run: |
      rustup toolchain install 1.93.0 --no-self-update
      rustc --version | grep -F 'rustc 1.93.0 '
""".lstrip()
    ci.write_text(ci_text, encoding="utf-8")
    citation = (
        "version: 0.9.0\n"
        "authors:\n"
        "  - family-names: Mahmoudian\n"
        "    given-names: Sepehr\n"
        'repository-code: "https://github.com/sepahead/prisoma"\n'
        "date-released: 2026-07-16\n"
        "abstract: public GitHub source prerelease\n"
    )
    (tmp_path / "CITATION.cff").write_text(citation, encoding="utf-8")
    release_notes = tmp_path / "release" / "0.9.0" / "RELEASE_NOTES.md"
    release_notes.write_text(
        "Prisoma 0.9.0 is a public GitHub source prerelease authored by "
        "Sepehr Mahmoudian and released on 2026-07-16. "
        "`published:false` in the candidate decision manifest means non-promotion; "
        "it does not deny public availability of this source prerelease. "
        "No DOI, Zenodo record, or archive identifier is assigned.\n",
        encoding="utf-8",
    )
    (tmp_path / "scripts" / "generate_candidate_release.py").write_text(
        'REPOSITORY = "https://github.com/sepahead/prisoma"\n'
        'AUTHOR = "Sepehr Mahmoudian"\n'
        'RELEASE_VERSION = "0.9.0"\n',
        encoding="utf-8",
    )
    assert MODULE.root_release_identity_problems() == []

    ci.write_text(ci_text.replace("1.93.0", "1.94.0"), encoding="utf-8")
    problems = MODULE.root_release_identity_problems()
    assert any("workspace rust-version" in problem for problem in problems)
    ci.write_text(ci_text, encoding="utf-8")

    (tmp_path / "CITATION.cff").write_text(
        citation + "doi: 10.0000/not-assigned\n", encoding="utf-8"
    )
    release_notes.write_text(
        "Prisoma 0.9.0 is a public GitHub source prerelease authored by a different "
        "author and released on 2026-07-16. `published:false` in the candidate decision "
        "manifest means non-promotion; it does not deny public availability of this "
        "source prerelease. No DOI, Zenodo record, or archive identifier is assigned.\n",
        encoding="utf-8",
    )
    problems = MODULE.root_release_identity_problems()
    assert any("assigns a DOI" in problem for problem in problems)
    assert any("Sepehr Mahmoudian" in problem for problem in problems)

    (tmp_path / "CITATION.cff").write_text(
        citation
        + "identifiers:\n"
        + "  - type: swh\n"
        + "    value: swh:1:rel:0000000000000000000000000000000000000000\n",
        encoding="utf-8",
    )
    problems = MODULE.root_release_identity_problems()
    assert any("archive identifier metadata" in problem for problem in problems)


def test_justfile_reproducibility_rejects_unlocked_unquoted_and_optimized_checks(
    tmp_path: Path, monkeypatch
) -> None:
    monkeypatch.setattr(MODULE, "ROOT", tmp_path)
    justfile = tmp_path / "justfile"
    guard = (
        "sys.flags.optimize == 0 or sys.exit("
        '"recipe checks require unoptimized Python")'
    )
    justfile.write_text(
        "\n".join(
            (
                'set shell := ["bash", "-euo", "pipefail", "-c"]',
                "safe value:",
                "    cargo run --locked --bin demo -- {{ quote(value) }}",
                f"    python -c 'import sys; {guard}; assert True'",
                "",
            )
        ),
        encoding="utf-8",
    )
    assert MODULE.justfile_reproducibility_problems() == []

    justfile.write_text(
        "\n".join(
            (
                'set shell := ["sh", "-cu"]',
                "unsafe value:",
                "    cargo test -p demo",
                "    command --input {{ value }}",
                "    python -c 'assert True'",
                "",
            )
        ),
        encoding="utf-8",
    )
    problems = MODULE.justfile_reproducibility_problems()
    assert any("omits `--locked`" in problem for problem in problems)
    assert any("without `quote(...)`" in problem for problem in problems)
    assert any("optimization-mode guard" in problem for problem in problems)
    assert any("strict-mode shell with pipefail" in problem for problem in problems)


def test_exp0_documentation_distinguishes_cells_from_seeded_case_results(
    tmp_path: Path, monkeypatch
) -> None:
    monkeypatch.setattr(MODULE, "ROOT", tmp_path)
    pid_rs = tmp_path / "pid-rs"
    monkeypatch.setattr(MODULE, "PID_RS", pid_rs)
    source = pid_rs / "crates" / "pid-core" / "src" / "bin" / "exp0.rs"
    source.parent.mkdir(parents=True)
    source.write_text(
        """
let mut seeds = 3usize;
let dims = [10usize, 64, 256];
for name in [
    "independent_additive",
    "redundant_copy",
    "unique_s1",
    "xor_like",
] {}
""".lstrip(),
        encoding="utf-8",
    )
    (tmp_path / "justfile").write_text(
        'exp0-runlog path="outputs/exp0_runlog.jsonl" '
        'summary="outputs/exp0_summary.json" seeds="1":\n',
        encoding="utf-8",
    )
    (tmp_path / "grandplan.md").write_text(
        "The sweep has 12 scenario–dimension cells over three deterministic "
        "seeds (36 case results).\n",
        encoding="utf-8",
    )
    findings = (
        "The binary has 36 case results from 12 scenario–dimension cells over "
        "three deterministic seeds, nine geometry warnings, zero geometry "
        "abstentions, nine monotonicity violations, three normalized-invariant "
        "bound violations. The `just exp0-runlog` recipe deliberately passes "
        "one seed, so the corresponding counts are 12, three, zero, three, and one.\n"
    )
    findings_path = tmp_path / "findings.md"
    findings_path.write_text(findings, encoding="utf-8")
    assert MODULE.exp0_documentation_problems() == []

    findings_path.write_text(
        findings.replace("36 case results", "12 case results", 1),
        encoding="utf-8",
    )
    problems = MODULE.exp0_documentation_problems()
    assert any("deterministic Exp0 count boundary" in problem for problem in problems)

    findings_path.write_text(findings, encoding="utf-8")
    source.write_text(
        source.read_text(encoding="utf-8").replace(
            "let mut seeds = 3usize;",
            "let mut seeds = 1usize;",
        ),
        encoding="utf-8",
    )
    problems = MODULE.exp0_documentation_problems()
    assert any("sweep contract changed" in problem for problem in problems)


def test_readme_reproducibility_rejects_unlocked_cargo_examples(
    tmp_path: Path, monkeypatch
) -> None:
    monkeypatch.setattr(MODULE, "ROOT", tmp_path)
    readme = tmp_path / "README.md"
    readme.write_text(
        "`cargo test --locked`\n"
        "`cargo run --locked -p demo --bin demo`\n"
        "`cargo install just --version 1.56.0 --locked`\n",
        encoding="utf-8",
    )
    assert MODULE.readme_reproducibility_problems() == []

    readme.write_text(
        "`cargo test`\n`cargo run -p demo --bin demo`\n",
        encoding="utf-8",
    )
    problems = MODULE.readme_reproducibility_problems()
    assert len(problems) == 2
    assert all("omits `--locked`" in problem for problem in problems)


def test_flake_reproducibility_requires_one_minimal_content_addressed_input(
    tmp_path: Path, monkeypatch
) -> None:
    monkeypatch.setattr(MODULE, "ROOT", tmp_path)
    revision = "5" * 40
    nar_hash = "sha256-" + ("A" * 43) + "="
    (tmp_path / "flake.nix").write_text(
        """
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
  outputs = { nixpkgs, ... }:
    let
      pkgs = import nixpkgs { system = "aarch64-darwin"; };
      mkPinnedArchiveTool = value: value;
      uvPinned = mkPinnedArchiveTool {
        version = "0.11.28";
        repository = "astral-sh/uv";
      };
      justPinned = mkPinnedArchiveTool {
        version = "1.56.0";
        repository = "casey/just";
      };
    in
    assert pkgs.lib.versionAtLeast pkgs.rustc.version "1.93.0";
    assert pkgs.python311.version == "3.11.15";
    {
      packages = with pkgs; [
        justPinned
        uvPinned
      ];
      cargoHint = "cargo test --locked";
      uvHint = "uv sync --locked";
    };
}
""".lstrip(),
        encoding="utf-8",
    )
    (tmp_path / "flake.lock").write_text(
        f"""
{{
  "nodes": {{
    "nixpkgs": {{
      "locked": {{
        "lastModified": 1780000000,
        "narHash": "{nar_hash}",
        "owner": "NixOS",
        "repo": "nixpkgs",
        "rev": "{revision}",
        "type": "github"
      }},
      "original": {{
        "owner": "NixOS",
        "ref": "nixos-26.05",
        "repo": "nixpkgs",
        "type": "github"
      }}
    }},
    "root": {{
      "inputs": {{
        "nixpkgs": "nixpkgs"
      }}
    }}
  }},
  "root": "root",
  "version": 7
}}
""".lstrip(),
        encoding="utf-8",
    )
    assert MODULE.flake_reproducibility_problems() == []

    lock_text = (tmp_path / "flake.lock").read_text(encoding="utf-8")
    (tmp_path / "flake.lock").write_text(
        lock_text.replace(f'"rev": "{revision}"', '"rev": "mutable"').replace(
            f'"narHash": "{nar_hash}"', '"narHash": "sha256-unbound"'
        ),
        encoding="utf-8",
    )
    (tmp_path / "flake.nix").write_text(
        """
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  outputs = { nixpkgs, ... }: {
    cargoHint = "cargo test";
    uvHint = "uv sync";
  };
}
""".lstrip(),
        encoding="utf-8",
    )
    problems = MODULE.flake_reproducibility_problems()
    assert any("nixos-26.05" in problem for problem in problems)
    assert any("flake-utils" in problem for problem in problems)
    assert any("40-hex commit" in problem for problem in problems)
    assert any("SHA-256 SRI" in problem for problem in problems)
    assert any("Cargo command omits `--locked`" in problem for problem in problems)
    assert any("uv sync omits `--locked`" in problem for problem in problems)
    assert any("pinned toolchain fragment" in problem for problem in problems)


def test_precommit_reproducibility_rejects_mutable_or_unpinned_tools(
    tmp_path: Path, monkeypatch
) -> None:
    monkeypatch.setattr(MODULE, "ROOT", tmp_path)
    config = tmp_path / ".pre-commit-config.yaml"
    config.write_text(
        f"""
# With the repository-required uv 0.11.28, install once with
# `uv tool install pre-commit=={MODULE.PRE_COMMIT_VERSION} && pre-commit install`.
repos:
  - repo: https://github.com/gitleaks/gitleaks
    # Immutable commit for the verified v{MODULE.GITLEAKS_VERSION} release.
    rev: {MODULE.GITLEAKS_REVISION}
    hooks:
      - id: gitleaks
""".lstrip(),
        encoding="utf-8",
    )
    assert MODULE.precommit_reproducibility_problems() == []

    config.write_text(
        """
# Install with pip install pre-commit.
repos:
  - repo: https://github.com/gitleaks/gitleaks
    rev: v8.30.1
    hooks:
      - id: gitleaks
""".lstrip(),
        encoding="utf-8",
    )
    problems = MODULE.precommit_reproducibility_problems()
    assert any("pre-commit==4.6.0" in problem for problem in problems)
    assert any("unpinned pip installation" in problem for problem in problems)
    assert any("40-hex commits" in problem for problem in problems)


def test_cli_converts_malformed_input_to_a_failed_audit(monkeypatch, capsys) -> None:
    monkeypatch.setattr(
        MODULE,
        "_audit",
        lambda: (_ for _ in ()).throw(MODULE.TruthAuditError("bad fixture")),
    )
    assert MODULE.main() == 1
    assert "audit input invalid or unavailable: bad fixture" in capsys.readouterr().out
