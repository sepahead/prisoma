import importlib.util
import os
import sys
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[2] / "scripts" / "audit_ci_pins.py"
SPEC = importlib.util.spec_from_file_location("prisoma_ci_pins", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)

audit_text = MODULE.audit_text
audit_reproducibility_text = MODULE.audit_reproducibility_text
audit_workflows = MODULE.audit_workflows


def test_repository_actions_are_commit_pinned() -> None:
    root = Path(__file__).resolve().parents[2]
    count, errors = audit_workflows(root)
    assert count > 0
    assert errors == []


def test_reproducibility_policy_rejects_mutable_runtimes_and_unlocked_commands() -> (
    None
):
    pin = "0123456789abcdef0123456789abcdef01234567"
    text = f"""
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@{pin}
      - uses: dtolnay/rust-toolchain@{pin}
      - uses: actions/setup-python@{pin}
        with:
          python-version: "3.11"
      - uses: astral-sh/setup-uv@{pin}
        with:
          version: "0.11"
      - run: cargo test --workspace
      - run: uv sync
      - run: maturin develop
      - run: rustup toolchain install stable
      - run: curl --location https://example.invalid/latest/tool
      - run: python -c 'assert True'
"""
    errors = audit_reproducibility_text(Path("ci.yml"), text)
    assert any("static versioned image" in error for error in errors)
    assert any("python-version must pin an exact patch" in error for error in errors)
    assert any("cargo test must use --locked" in error for error in errors)
    assert any("uv sync must use --locked" in error for error in errors)
    assert any("maturin develop must use --locked" in error for error in errors)
    assert any("rustup must install an exact toolchain" in error for error in errors)
    assert any("disable self-update" in error for error in errors)
    assert any("bounded HTTPS flags" in error for error in errors)
    assert any("strict SHA-256" in error for error in errors)
    assert any("mutable latest URL" in error for error in errors)
    assert any("rust-toolchain action must pin" in error for error in errors)
    assert any("setup-uv must pin" in error for error in errors)
    assert any("checkout must disable" in error for error in errors)
    assert any("may not use Python `assert`" in error for error in errors)


def test_reproducibility_policy_accepts_exact_runtimes_and_lock_enforcement() -> None:
    pin = "0123456789abcdef0123456789abcdef01234567"
    text = f"""
jobs:
  test:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@{pin}
        with:
          persist-credentials: false
      - uses: dtolnay/rust-toolchain@{pin}
        with:
          toolchain: 1.93.0
      - uses: actions/setup-python@{pin}
        with:
          python-version: "3.11.15"
      - uses: astral-sh/setup-uv@{pin}
        with:
          version: "0.11.28"
      - run: cargo test --locked --workspace
      - run: uv sync --locked
      - run: maturin develop --locked
      - run: rustup toolchain install 1.93.0 --no-self-update
      - run: python -c 'raise SystemExit(0)'
      - run: |
          curl --fail --location --retry 5 --retry-all-errors --connect-timeout 30 --max-time 300 --retry-max-time 600 --max-filesize 10000000 --proto '=https' --proto-redir '=https' --tlsv1.2 --output "$RUNNER_TEMP/tool" https://example.invalid/releases/download/1.2.3/tool
          echo "0000000000000000000000000000000000000000000000000000000000000000  $RUNNER_TEMP/tool" | sha256sum --check --strict
"""
    assert audit_reproducibility_text(Path("ci.yml"), text) == []


def test_rejects_tags_branches_short_shas_and_dynamic_revisions() -> None:
    text = """
steps:
  - uses: actions/checkout@v7
  - uses: owner/action@main
  - uses: owner/action@0123456789abcdef
  - uses: owner/action@${{ github.sha }}
"""
    count, errors = audit_text(Path("ci.yml"), text)
    assert count == 4
    assert len(errors) == 4


def test_accepts_full_commit_pins_and_rejects_unaudited_local_actions() -> None:
    text = """
steps:
  - uses: actions/checkout@0123456789abcdef0123456789abcdef01234567
  - uses: ./local-action
"""
    count, errors = audit_text(Path("ci.yml"), text)
    assert count == 2
    assert len(errors) == 1
    assert "local actions are forbidden" in errors[0]


def test_rejects_noncanonical_uses_key_forms() -> None:
    pin = "0123456789abcdef0123456789abcdef01234567"
    text = f"""
steps:
  - uses : owner/action@{pin}
  - "uses": owner/action@{pin}
  - 'uses': owner/action@{pin}
  - {{ uses: owner/action@{pin} }}
  ? uses
  : owner/action@{pin}
"""
    count, errors = audit_text(Path("ci.yml"), text)
    assert count == 5
    assert len(errors) == 5
    assert all("uses key must use canonical block form" in error for error in errors)


def test_accepts_quoted_external_values_but_rejects_quoted_local_actions() -> None:
    pin = "0123456789abcdef0123456789abcdef01234567"
    text = f"""
steps:
  - uses: "owner/action@{pin}"
  - uses: './local-action'
"""
    count, errors = audit_text(Path("ci.yml"), text)
    assert count == 2
    assert len(errors) == 1
    assert "local actions are forbidden" in errors[0]


def test_local_composite_cannot_hide_an_unpinned_external_action() -> None:
    text = """
steps:
  - uses: ./.github/actions/local
"""
    count, errors = audit_text(Path("ci.yml"), text)
    assert count == 1
    assert len(errors) == 1
    assert "local actions are forbidden" in errors[0]


def test_rejects_yaml_decoded_or_composed_uses_keys() -> None:
    fixtures = [
        'steps:\n  - "u\\u0073es": owner/action@main\n',
        "steps:\n  - !!str uses: owner/action@main\n",
        'steps:\n  - {"u\\x73es": owner/action@main}\n',
        "action-key: &action-key uses\nsteps:\n  - *action-key: owner/action@main\n",
        "steps:\n  - ? |-\n      uses\n    : owner/action@main\n",
        ('steps: [ { !<tag:yaml.org,2002:str> "uses": owner/action@main } ]\n'),
        'steps: [ { ? "uses" : owner/action@main } ]\n',
    ]
    for text in fixtures:
        count, errors = audit_text(Path("ci.yml"), text)
        assert count > 0
        assert errors
        assert any("canonical block" in error for error in errors)


def test_ignores_action_like_text_inside_block_scalars() -> None:
    pin = "0123456789abcdef0123456789abcdef01234567"
    text = f"""
steps:
  - name: Shell fixture
    run: |
      printf '%s' '"u\\u0073es": owner/action@main'
      printf '%s' '- {{uses: owner/action@main}}'
  - uses: owner/action@{pin}
"""
    count, errors = audit_text(Path("ci.yml"), text)
    assert count == 1
    assert errors == []


def test_compact_block_scalar_cannot_hide_a_sibling_uses_key() -> None:
    fixtures = [
        """
steps:
  - run: |
      echo fixture
    "u\\u0073es": owner/action@main
""",
        """
steps:
  -   run: |
        echo fixture
      "u\\u0073es": owner/action@main
""",
    ]
    for text in fixtures:
        count, errors = audit_text(Path("ci.yml"), text)
        assert count == 1
        assert len(errors) == 1
        assert "canonical block" in errors[0]


def test_workflow_reader_rejects_symlinks_fifos_and_oversized_files(
    tmp_path: Path, monkeypatch: object
) -> None:
    workflow_dir = tmp_path / ".github" / "workflows"
    workflow_dir.mkdir(parents=True)
    target = tmp_path / "target.yml"
    target.write_text("steps: []\n", encoding="utf-8")
    (workflow_dir / "symlink.yml").symlink_to(target)
    if hasattr(os, "mkfifo"):
        os.mkfifo(workflow_dir / "pipe.yaml")
    oversized = workflow_dir / "oversized.yml"
    oversized.write_bytes(b"x" * 17)
    monkeypatch.setattr(MODULE, "MAX_WORKFLOW_BYTES", 16)

    count, errors = audit_workflows(tmp_path)
    assert count == 0
    assert any("regular non-symlink" in error for error in errors)
    assert any("exceeds 16 bytes" in error for error in errors)
