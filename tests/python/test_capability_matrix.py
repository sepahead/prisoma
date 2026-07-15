"""Regression tests for the generated, content-bound capability matrix."""

from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import subprocess
import sys
from pathlib import Path

import pytest


_SCRIPT = (
    Path(__file__).resolve().parents[2] / "scripts" / "generate_capability_matrix.py"
)
_SPEC = importlib.util.spec_from_file_location("prisoma_capability_matrix", _SCRIPT)
assert _SPEC is not None and _SPEC.loader is not None
_MODULE = importlib.util.module_from_spec(_SPEC)
sys.modules[_SPEC.name] = _MODULE
_SPEC.loader.exec_module(_MODULE)

CatalogError = _MODULE.CatalogError
content_digest = _MODULE.content_digest
load_catalog = _MODULE.load_catalog
render_json = _MODULE.render_json
render_markdown = _MODULE.render_markdown
resolve_catalog = _MODULE.resolve_catalog

_FIXTURE_REQUIRED = frozenset({"fixture.tested"})


def _catalog() -> dict:
    return {
        "schema_version": 1,
        "as_of_date": "2026-07-13",
        "canonical_spec": {
            "path": "grandplan.md",
            "version": "12.5",
            "sections": ["§8.10"],
        },
        "scope": "unit-test fixture only",
        "rows": [
            {
                "feature_id": "fixture.tested",
                "feature": "Fixture-tested capability",
                "status": "tested",
                "revision_inputs": ["src", "Cargo.lock"],
                "external_revision": "pid-rs@0123456789abcdef0123456789abcdef01234567",
                "test_command": "just fixture",
                "evidence_artifacts": ["src/evidence.txt#fixture"],
                "known_limitations": "Synthetic unit-test evidence only.",
                "evidence_level": "E3",
                "evidence_basis": "build_tested_adapter",
                "evidence_scope": "local fixture",
                "thesis_dependency": "optional",
                "claim_ids": ["EC1"],
            }
        ],
    }


def _write_repo(tmp_path: Path, catalog: dict | None = None) -> Path:
    (tmp_path / "grandplan.md").write_text("# v12.5\n", encoding="utf-8")
    (tmp_path / "justfile").write_text("fixture:\n    true\n", encoding="utf-8")
    source = tmp_path / "src"
    source.mkdir(exist_ok=True)
    (source / "evidence.txt").write_text("evidence\n", encoding="utf-8")
    (tmp_path / "Cargo.lock").write_text(
        'version = 4\n\n[[package]]\nname = "fixture-upstream"\nversion = "1.0.0"\n'
        'source = "git+https://github.com/sepahead/pid-rs#'
        '0123456789abcdef0123456789abcdef01234567"\n',
        encoding="utf-8",
    )
    catalog_path = tmp_path / "catalog.json"
    catalog_path.write_text(
        json.dumps(_catalog() if catalog is None else catalog, indent=2) + "\n",
        encoding="utf-8",
    )
    return catalog_path


def test_real_catalog_resolves_deterministically_and_has_no_validated_claim() -> None:
    first = resolve_catalog()
    second = resolve_catalog()
    assert render_json(first) == render_json(second)
    assert first["row_count"] == len(first["rows"])
    assert first["status_counts"].get("validated", 0) == 0
    assert all(
        len(row["exact_revision"].rsplit(":", 1)[-1]) == 64 for row in first["rows"]
    )
    markdown = render_markdown(first)
    assert "| Feature | Status | Exact revision | Test command |" in markdown
    assert "not a scientific result" in markdown
    assert "**0** `validated` row(s)" in markdown


def test_resolved_matrix_binds_source_and_evidence_bytes(tmp_path: Path) -> None:
    path = _write_repo(tmp_path)
    before = resolve_catalog(
        path,
        root=tmp_path,
        required_feature_ids=_FIXTURE_REQUIRED,
    )
    assert before["rows"][0]["exact_revision"].startswith(
        "pid-rs@0123456789abcdef0123456789abcdef01234567; local-sha256:"
    )
    assert "justfile" in before["rows"][0]["revision_inputs"]
    assert len(before["rows"][0]["evidence_artifacts"][0]["sha256"]) == 64

    (tmp_path / "src/evidence.txt").write_text("changed\n", encoding="utf-8")
    after = resolve_catalog(
        path,
        root=tmp_path,
        required_feature_ids=_FIXTURE_REQUIRED,
    )
    assert before["rows"][0]["exact_revision"] != after["rows"][0]["exact_revision"]
    assert (
        before["rows"][0]["evidence_artifacts"][0]["sha256"]
        != after["rows"][0]["evidence_artifacts"][0]["sha256"]
    )

    (tmp_path / "justfile").write_text("fixture:\n    false\n", encoding="utf-8")
    changed_recipe = resolve_catalog(
        path,
        root=tmp_path,
        required_feature_ids=_FIXTURE_REQUIRED,
    )
    assert (
        after["rows"][0]["exact_revision"]
        != changed_recipe["rows"][0]["exact_revision"]
    )


@pytest.mark.parametrize(
    ("mutation", "message"),
    [
        (lambda row: row.update(status="specified"), "cannot exceed E1"),
        (lambda row: row.update(evidence_level="E2"), "requires E3"),
        (
            lambda row: row.update(evidence_basis="intention_or_local_only"),
            "requires E0",
        ),
        (lambda row: row.update(known_limitations="none"), "real boundary"),
        (lambda row: row.update(thesis_dependency="mandatory"), "thesis_dependency"),
        (lambda row: row.update(claim_ids=["H9"]), "unknown claims"),
        (
            lambda row: row.update(external_revision="upstream@main"),
            "exact bounded 40-hex",
        ),
        (
            lambda row: row.update(
                external_revision="upstream@0123456789abcdef0123456789abcdef012345678"
            ),
            "exact bounded 40-hex",
        ),
        (lambda row: row.update(test_command="just imaginary"), "unknown just recipe"),
        (lambda row: row.update(extra_field=True), "unknown"),
    ],
)
def test_catalog_rejects_semantic_overclaim_and_schema_drift(
    tmp_path: Path, mutation, message: str
) -> None:
    catalog = _catalog()
    mutation(catalog["rows"][0])
    path = _write_repo(tmp_path, catalog)
    with pytest.raises(CatalogError, match=message):
        load_catalog(
            path,
            root=tmp_path,
            required_feature_ids=_FIXTURE_REQUIRED,
        )


def test_catalog_rejects_missing_traversal_symlink_and_generated_inputs(
    tmp_path: Path,
) -> None:
    cases = [
        (["missing.txt"], "does not exist"),
        (["../outside.txt"], "escapes the repository"),
        (["protocols/capability_matrix_current_v1.json"], "generated outputs"),
    ]
    for revision_inputs, message in cases:
        catalog = _catalog()
        catalog["rows"][0]["revision_inputs"] = revision_inputs
        if revision_inputs[0].startswith("protocols/"):
            generated = tmp_path / revision_inputs[0]
            generated.parent.mkdir(exist_ok=True)
            generated.write_text("generated\n", encoding="utf-8")
        path = _write_repo(tmp_path, catalog)
        with pytest.raises(CatalogError, match=message):
            load_catalog(
                path,
                root=tmp_path,
                required_feature_ids=_FIXTURE_REQUIRED,
            )

    symlink = tmp_path / "linked"
    symlink.symlink_to(tmp_path / "src", target_is_directory=True)
    catalog = _catalog()
    catalog["rows"][0]["revision_inputs"] = ["linked"]
    path = _write_repo(tmp_path, catalog)
    with pytest.raises(CatalogError, match="symlink"):
        load_catalog(
            path,
            root=tmp_path,
            required_feature_ids=_FIXTURE_REQUIRED,
        )

    nested_link = tmp_path / "linked-parent"
    nested_link.symlink_to(tmp_path / "src", target_is_directory=True)
    catalog["rows"][0]["revision_inputs"] = ["linked-parent/evidence.txt"]
    path = _write_repo(tmp_path, catalog)
    with pytest.raises(CatalogError, match="traverse a symlink"):
        load_catalog(
            path,
            root=tmp_path,
            required_feature_ids=_FIXTURE_REQUIRED,
        )


def test_content_digest_ignores_build_and_cache_directories(tmp_path: Path) -> None:
    path = _write_repo(tmp_path)
    load_catalog(path, root=tmp_path, required_feature_ids=_FIXTURE_REQUIRED)
    baseline = content_digest(["src"], root=tmp_path)
    for ignored in ("target", "__pycache__", ".pytest_cache", ".ruff_cache", ".git"):
        directory = tmp_path / "src" / ignored
        directory.mkdir()
        (directory / "noise.bin").write_bytes(b"noise")
    assert content_digest(["src"], root=tmp_path) == baseline


def test_catalog_rows_must_be_stably_sorted(tmp_path: Path) -> None:
    catalog = _catalog()
    second = copy.deepcopy(catalog["rows"][0])
    second["feature_id"] = "alpha.first"
    second["feature"] = "Earlier capability"
    catalog["rows"].append(second)
    path = _write_repo(tmp_path, catalog)
    with pytest.raises(CatalogError, match="sorted"):
        load_catalog(
            path,
            root=tmp_path,
            required_feature_ids=frozenset({"alpha.first", "fixture.tested"}),
        )


@pytest.mark.parametrize("field", ["revision_inputs", "evidence_artifacts"])
def test_catalog_rejects_generated_outputs_through_directory_ancestors(
    tmp_path: Path, field: str
) -> None:
    generated = tmp_path / "protocols/capability_matrix_current_v1.json"
    generated.parent.mkdir(parents=True)
    generated.write_text("{}\n", encoding="utf-8")
    catalog = _catalog()
    catalog["rows"][0][field] = ["protocols"]
    path = _write_repo(tmp_path, catalog)
    with pytest.raises(CatalogError, match="generated outputs"):
        load_catalog(
            path,
            root=tmp_path,
            required_feature_ids=_FIXTURE_REQUIRED,
        )


def test_catalog_rejects_duplicate_json_keys(tmp_path: Path) -> None:
    path = _write_repo(tmp_path)
    raw = path.read_text(encoding="utf-8")
    path.write_text(raw.replace('"scope":', '"scope": "duplicate",\n  "scope":', 1))
    with pytest.raises(CatalogError, match="duplicate JSON key 'scope'"):
        load_catalog(
            path,
            root=tmp_path,
            required_feature_ids=_FIXTURE_REQUIRED,
        )


def test_catalog_rejects_missing_required_capability(tmp_path: Path) -> None:
    path = _write_repo(tmp_path)
    with pytest.raises(CatalogError, match="missing.required"):
        load_catalog(
            path,
            root=tmp_path,
            required_feature_ids=frozenset({"fixture.tested", "missing.required"}),
        )


@pytest.mark.parametrize(
    ("declaration", "command"),
    [
        ('fake := "value"', "just fake"),
        ('set shell := ["bash", "-cu"]', "just set"),
    ],
)
def test_just_variables_and_settings_are_not_recipes(
    tmp_path: Path, declaration: str, command: str
) -> None:
    catalog = _catalog()
    catalog["rows"][0]["test_command"] = command
    path = _write_repo(tmp_path, catalog)
    (tmp_path / "justfile").write_text(
        f"{declaration}\nfixture:\n    true\n",
        encoding="utf-8",
    )
    with pytest.raises(CatalogError, match="unknown just recipe"):
        load_catalog(path, root=tmp_path, required_feature_ids=_FIXTURE_REQUIRED)


def test_registry_revision_must_match_a_bound_cargo_lock(tmp_path: Path) -> None:
    checksum = "a" * 64
    catalog = _catalog()
    row = catalog["rows"][0]
    row["external_revision"] = f"fixture-crate@1.2.3#sha256:{checksum}"
    row["evidence_level"] = "E2"
    row["evidence_basis"] = "declared_immutable_dependency"
    row["revision_inputs"] = ["src", "Cargo.lock"]
    path = _write_repo(tmp_path, catalog)
    lockfile = tmp_path / "Cargo.lock"
    lockfile.write_text(
        'version = 4\n\n[[package]]\nname = "fixture-crate"\nversion = "1.2.3"\n'
        f'source = "registry+https://example.invalid/index"\nchecksum = "{checksum}"\n',
        encoding="utf-8",
    )
    load_catalog(path, root=tmp_path, required_feature_ids=_FIXTURE_REQUIRED)

    lockfile.write_text(lockfile.read_text().replace(checksum, "b" * 64))
    with pytest.raises(CatalogError, match="does not match any exact package"):
        load_catalog(path, root=tmp_path, required_feature_ids=_FIXTURE_REQUIRED)


def test_lockfile_package_schema_is_bounded_and_typed(tmp_path: Path) -> None:
    catalog = _catalog()
    path = _write_repo(tmp_path, catalog)
    (tmp_path / "Cargo.lock").write_text(
        'version = 4\npackage = "not-a-list"\n', encoding="utf-8"
    )
    with pytest.raises(
        CatalogError, match=r"Git-source lockfile\.package must be a list"
    ):
        load_catalog(path, root=tmp_path, required_feature_ids=_FIXTURE_REQUIRED)


def test_multiple_registry_revisions_are_each_bound_to_cargo_lock(
    tmp_path: Path,
) -> None:
    first_checksum = "a" * 64
    second_checksum = "b" * 64
    catalog = _catalog()
    row = catalog["rows"][0]
    row["external_revision"] = (
        f"first-crate@1.2.3#sha256:{first_checksum}; "
        f"second-crate@4.5.6#sha256:{second_checksum}"
    )
    row["evidence_level"] = "E2"
    row["evidence_basis"] = "declared_immutable_dependency"
    row["revision_inputs"] = ["src", "Cargo.lock"]
    path = _write_repo(tmp_path, catalog)
    (tmp_path / "Cargo.lock").write_text(
        'version = 4\n\n[[package]]\nname = "first-crate"\nversion = "1.2.3"\n'
        'source = "registry+https://example.invalid/index"\n'
        f'checksum = "{first_checksum}"\n\n'
        '[[package]]\nname = "second-crate"\nversion = "4.5.6"\n'
        'source = "registry+https://example.invalid/index"\n'
        f'checksum = "{second_checksum}"\n',
        encoding="utf-8",
    )
    load_catalog(path, root=tmp_path, required_feature_ids=_FIXTURE_REQUIRED)

    (tmp_path / "Cargo.lock").write_text(
        (tmp_path / "Cargo.lock").read_text().replace(second_checksum, "c" * 64),
        encoding="utf-8",
    )
    with pytest.raises(CatalogError, match="does not match any exact package"):
        load_catalog(path, root=tmp_path, required_feature_ids=_FIXTURE_REQUIRED)


def test_git_revision_must_match_a_bound_cargo_lock_source(tmp_path: Path) -> None:
    catalog = _catalog()
    path = _write_repo(tmp_path, catalog)
    load_catalog(path, root=tmp_path, required_feature_ids=_FIXTURE_REQUIRED)

    catalog["rows"][0]["external_revision"] = f"pid-rs@{'0' * 40}"
    path = _write_repo(tmp_path, catalog)
    with pytest.raises(CatalogError, match="canonical repository"):
        load_catalog(path, root=tmp_path, required_feature_ids=_FIXTURE_REQUIRED)


def test_git_revision_rejects_a_same_named_repository_at_another_url(
    tmp_path: Path,
) -> None:
    catalog = _catalog()
    path = _write_repo(tmp_path, catalog)
    lockfile = tmp_path / "Cargo.lock"
    lockfile.write_text(
        lockfile.read_text(encoding="utf-8").replace(
            "https://github.com/sepahead/pid-rs",
            "https://evil.invalid/pid_rs",
        ),
        encoding="utf-8",
    )
    with pytest.raises(CatalogError, match="canonical repository"):
        load_catalog(path, root=tmp_path, required_feature_ids=_FIXTURE_REQUIRED)


def test_git_revision_must_match_a_bound_tag_selector(tmp_path: Path) -> None:
    catalog = _catalog()
    revision = "0123456789abcdef0123456789abcdef01234567"
    catalog["rows"][0]["external_revision"] = f"pid-rs@v1.0.0#{revision}"
    path = _write_repo(tmp_path, catalog)
    lockfile = tmp_path / "Cargo.lock"
    lockfile.write_text(
        'version = 4\n\n[[package]]\nname = "fixture-upstream"\nversion = "1.0.0"\n'
        f'source = "git+https://github.com/sepahead/pid-rs?tag=v1.0.0#{revision}"\n',
        encoding="utf-8",
    )
    load_catalog(path, root=tmp_path, required_feature_ids=_FIXTURE_REQUIRED)

    lockfile.write_text(lockfile.read_text().replace("tag=v1.0.0", "tag=v1.0.1"))
    with pytest.raises(CatalogError, match="canonical repository"):
        load_catalog(path, root=tmp_path, required_feature_ids=_FIXTURE_REQUIRED)


def test_git_revision_may_bind_a_declared_submodule_gitlink(tmp_path: Path) -> None:
    source = tmp_path / "pid-rs"
    source.mkdir()
    (source / "evidence.txt").write_text("pinned\n", encoding="utf-8")
    subprocess.run(["git", "init", "-q", str(source)], check=True)
    subprocess.run(["git", "-C", str(source), "add", "evidence.txt"], check=True)
    subprocess.run(
        [
            "git",
            "-C",
            str(source),
            "-c",
            "user.name=Capability Test",
            "-c",
            "user.email=capability@example.invalid",
            "commit",
            "-qm",
            "fixture",
        ],
        check=True,
    )
    revision = subprocess.run(
        ["git", "-C", str(source), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()

    catalog = _catalog()
    row = catalog["rows"][0]
    row["external_revision"] = f"pid-rs@{revision}"
    row["revision_inputs"] = ["pid-rs", ".gitmodules"]
    (tmp_path / ".gitmodules").write_text(
        '[submodule "pid-rs"]\n'
        "\tpath = pid-rs\n"
        "\turl = git@github.com:sepahead/pid-rs.git\n",
        encoding="utf-8",
    )
    path = _write_repo(tmp_path, catalog)
    subprocess.run(["git", "init", "-q", str(tmp_path)], check=True)
    subprocess.run(
        [
            "git",
            "-C",
            str(tmp_path),
            "update-index",
            "--add",
            "--cacheinfo",
            f"160000,{revision},pid-rs",
        ],
        check=True,
    )
    load_catalog(path, root=tmp_path, required_feature_ids=_FIXTURE_REQUIRED)

    gitmodules = tmp_path / ".gitmodules"
    canonical_gitmodules = gitmodules.read_text(encoding="utf-8")
    gitmodules.write_text(
        canonical_gitmodules.replace(
            "git@github.com:sepahead/pid-rs.git",
            "https://evil.invalid/pid-rs.git",
        ),
        encoding="utf-8",
    )
    with pytest.raises(CatalogError, match="canonical repository"):
        load_catalog(path, root=tmp_path, required_feature_ids=_FIXTURE_REQUIRED)
    gitmodules.write_text(canonical_gitmodules, encoding="utf-8")

    catalog["rows"][0]["external_revision"] = f"pid-rs@{'0' * 40}"
    path = _write_repo(tmp_path, catalog)
    with pytest.raises(CatalogError, match="canonical repository"):
        load_catalog(path, root=tmp_path, required_feature_ids=_FIXTURE_REQUIRED)

    catalog["rows"][0]["external_revision"] = f"pid-rs@{revision}"
    path = _write_repo(tmp_path, catalog)
    (source / "evidence.txt").write_text("dirty\n", encoding="utf-8")
    with pytest.raises(CatalogError, match="canonical repository"):
        load_catalog(path, root=tmp_path, required_feature_ids=_FIXTURE_REQUIRED)


def test_catalog_parse_and_source_hash_share_one_bounded_snapshot(
    tmp_path: Path, monkeypatch
) -> None:
    path = _write_repo(tmp_path)
    original = _MODULE._read_regular_bytes
    catalog_reads = 0

    def counted(candidate: Path, **kwargs) -> bytes:
        nonlocal catalog_reads
        if candidate == path:
            catalog_reads += 1
        return original(candidate, **kwargs)

    monkeypatch.setattr(_MODULE, "_read_regular_bytes", counted)
    matrix = resolve_catalog(
        path,
        root=tmp_path,
        required_feature_ids=_FIXTURE_REQUIRED,
    )
    assert catalog_reads == 1
    assert (
        matrix["source_catalog"]["sha256"]
        == hashlib.sha256(path.read_bytes()).hexdigest()
    )


def test_content_digest_enforces_aggregate_byte_budget(
    tmp_path: Path, monkeypatch
) -> None:
    _write_repo(tmp_path)
    monkeypatch.setattr(_MODULE, "MAX_REVISION_TOTAL_BYTES", 4)
    with pytest.raises(CatalogError, match="aggregate 4-byte"):
        content_digest(["src"], root=tmp_path)


def test_capability_write_is_atomic_and_symlink_safe(
    tmp_path: Path, monkeypatch
) -> None:
    target = tmp_path / "matrix.json"
    target.write_text("old\n", encoding="utf-8")

    def fail_replace(_source, _destination) -> None:
        raise OSError("injected replacement failure")

    monkeypatch.setattr(_MODULE.os, "replace", fail_replace)
    with pytest.raises(OSError, match="injected replacement failure"):
        _MODULE._atomic_write(target, "new\n")
    assert target.read_text(encoding="utf-8") == "old\n"
    assert list(tmp_path.glob(".matrix.json.*")) == []

    monkeypatch.undo()
    real = tmp_path / "real.json"
    real.write_text("real\n", encoding="utf-8")
    target.unlink()
    target.symlink_to(real)
    with pytest.raises(CatalogError, match="non-symlink"):
        _MODULE._atomic_write(target, "replacement\n")
    assert real.read_text(encoding="utf-8") == "real\n"


def test_git_subprocess_has_output_and_time_budgets(tmp_path: Path) -> None:
    with pytest.raises(CatalogError, match="aggregate 32-byte"):
        _MODULE._run_bounded(
            [sys.executable, "-c", "import sys; sys.stdout.write('x' * 128)"],
            cwd=tmp_path,
            timeout_seconds=2,
            max_output_bytes=32,
        )
    with pytest.raises(subprocess.TimeoutExpired):
        _MODULE._run_bounded(
            [sys.executable, "-c", "import time; time.sleep(2)"],
            cwd=tmp_path,
            timeout_seconds=0.05,
            max_output_bytes=32,
        )


def test_catalog_git_validation_has_an_aggregate_command_budget(
    tmp_path: Path, monkeypatch
) -> None:
    monkeypatch.setattr(_MODULE, "MAX_GIT_COMMANDS", 1)
    with _MODULE._git_budget():
        first = _MODULE._run_bounded(
            [sys.executable, "-c", "print('ok')"], cwd=tmp_path
        )
        assert first.stdout == "ok\n"
        with pytest.raises(CatalogError, match="1-command aggregate limit"):
            _MODULE._run_bounded(
                [sys.executable, "-c", "print('not run')"], cwd=tmp_path
            )
