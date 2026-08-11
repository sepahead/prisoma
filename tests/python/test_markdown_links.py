"""Focused tests for Prisoma's offline Markdown link policy."""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "audit_markdown_links.py"
SPEC = importlib.util.spec_from_file_location("prisoma_audit_markdown_links", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


def _git_repo(tmp_path: Path, files: dict[str, str | bytes]) -> Path:
    root = tmp_path / "repo"
    root.mkdir()
    subprocess.run(["git", "init", "-q"], cwd=root, check=True)
    for relative, content in files.items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(content, bytes):
            path.write_bytes(content)
        else:
            path.write_text(content, encoding="utf-8")
    subprocess.run(["git", "add", "-A"], cwd=root, check=True)
    return root


def _audit(root: Path, allowances: tuple[object, ...] = ()) -> object:
    return MODULE.audit_repository(root, allowances=frozenset(allowances))


def test_current_docset_passes_with_one_exact_archive_allowance() -> None:
    report = MODULE.audit_repository(ROOT)
    assert report.markdown_files >= 47
    assert report.allowed_findings == 1
    assert report.heading_fragments > 0


def test_commonmark_parser_covers_project_forms_and_ignores_code() -> None:
    document = MODULE.parse_document(
        """\
# Source

[inline](target.md#target)
![image](asset.png)
[full][target-ref]
<a href="target.md#target">raw</a>
<img src=asset.png>
<https://invalid.example/not-fetched>
`[code](missing.md)`

```markdown
[fenced](missing.md)
```

[target-ref]: target.md#target
""",
        source="source.md",
    )
    forms = {link.form for link in document.links}
    assert forms == {
        "html_href",
        "html_src",
        "markdown_image",
        "markdown_link",
        "reference_definition",
    }
    assert all(link.destination != "missing.md" for link in document.links)
    assert document.headings == frozenset({"source"})


def test_github_heading_slugs_cover_markup_unicode_and_duplicates() -> None:
    document = MODULE.parse_document(
        """\
## Hybrid Rendering: Splats + Mesh + Physics Proxies
## Confirmatory Claims → Experimental Programme Map
## Agent Bridge Control Plane (LLM‑First)
## Estimator core (`pid-rs/` submodule)
## [Linked title](target.md)
## Repeat
## Repeat
""",
        source="source.md",
    )
    assert document.headings == frozenset(
        {
            "hybrid-rendering-splats--mesh--physics-proxies",
            "confirmatory-claims--experimental-programme-map",
            "agent-bridge-control-plane-llmfirst",
            "estimator-core-pid-rs-submodule",
            "linked-title",
            "repeat",
            "repeat-1",
        }
    )


def test_valid_local_targets_fragments_images_and_external_urls_pass(
    tmp_path: Path,
) -> None:
    root = _git_repo(
        tmp_path,
        {
            "README.md": """\
# Home

[root](./)
[heading](docs/target.md#repeated-1)
[encoded](docs/space%20name.md#encoded-heading)
[tree](docs/)
![image](assets/pixel.bin)
[external](https://invalid.example/not-fetched)
""",
            "docs/target.md": "# Repeated\n\n# Repeated\n",
            "docs/space name.md": "# Encoded heading\n",
            "assets/pixel.bin": b"pixel",
        },
    )
    report = _audit(root)
    assert report.local_links == 5
    assert report.heading_fragments == 2
    assert report.external_links == 1


@pytest.mark.parametrize(
    "destination",
    (
        "javascript:alert(1)",
        "data:text/html,unsafe",
        "file:///etc/passwd",
        "//example.invalid/path",
        "https:missing-authority",
        "https://user:secret@example.invalid/path",
        "https://example.invalid/bad%0Apath",
    ),
)
def test_unsafe_or_malformed_external_urls_fail_closed(
    tmp_path: Path, destination: str
) -> None:
    root = _git_repo(tmp_path, {"README.md": f"[bad]({destination})\n"})
    with pytest.raises(MODULE.MarkdownAuditError, match="invalid_external_url"):
        _audit(root)


def test_invalid_percent_escape_in_html_url_fails_closed(tmp_path: Path) -> None:
    root = _git_repo(
        tmp_path,
        {"README.md": '<a href="https://example.invalid/%ZZ">bad</a>\n'},
    )
    with pytest.raises(MODULE.MarkdownAuditError, match="invalid_external_url"):
        _audit(root)


def test_supported_external_url_schemes_are_counted_without_network(
    tmp_path: Path,
) -> None:
    root = _git_repo(
        tmp_path,
        {
            "README.md": (
                "[https](https://example.invalid/a%20b?q=one%20two#part)\n"
                "[http](http://example.invalid/)\n"
                "[mail](mailto:reviewer@example.invalid)\n"
            )
        },
    )
    assert _audit(root).external_links == 3


@pytest.mark.parametrize(
    ("destination", "kind"),
    [
        ("missing.md", "missing_target"),
        ("/absolute.md", "out_of_repo_target"),
        ("../../outside.md", "out_of_repo_target"),
        ("target.md#absent", "missing_heading_fragment"),
        ("untracked.md", "untracked_target"),
        ("target.md#bad%FF", "invalid_destination"),
        ("target%0Afile.md", "invalid_destination"),
        (r"target\file.md", "invalid_destination"),
    ],
)
def test_invalid_local_targets_fail_closed(
    tmp_path: Path, destination: str, kind: str
) -> None:
    root = _git_repo(
        tmp_path,
        {
            "docs/source.md": f"[bad]({destination})\n",
            "docs/target.md": "# Present\n",
        },
    )
    (root / "docs/untracked.md").write_text("untracked\n", encoding="utf-8")
    with pytest.raises(MODULE.MarkdownAuditError, match=kind):
        _audit(root)


def test_symlink_target_is_rejected(tmp_path: Path) -> None:
    root = _git_repo(tmp_path, {"README.md": "[bad](linked.txt)\n"})
    (root / "real.txt").write_text("real\n", encoding="utf-8")
    (root / "linked.txt").symlink_to("real.txt")
    subprocess.run(["git", "add", "linked.txt"], cwd=root, check=True)
    with pytest.raises(MODULE.MarkdownAuditError, match="non_regular_target"):
        _audit(root)


@pytest.mark.parametrize(
    "source",
    [
        "[full][missing]",
        "[collapsed][]",
        "![image][missing]",
        "[nested [label]][missing]",
        "[code `]` label][missing]",
        r"[escaped \[ label][missing]",
    ],
)
def test_explicit_undefined_references_fail_closed(tmp_path: Path, source: str) -> None:
    root = _git_repo(tmp_path, {"README.md": f"{source}\n"})
    with pytest.raises(MODULE.MarkdownAuditError, match="undefined_reference"):
        _audit(root)


def test_shortcut_text_and_adjacent_numeric_citations_are_not_links(
    tmp_path: Path,
) -> None:
    root = _git_repo(
        tmp_path,
        {"README.md": "An ordinary [phrase] and citations [2][3].\n"},
    )
    assert _audit(root).parsed_links == 0


def test_undefined_reference_policy_respects_commonmark_literal_contexts() -> None:
    document = MODULE.parse_document(
        r"""\[literal][missing]
`[code][missing]`
[inline](target.md?example=[not][a-reference])
<span data-example="[html][missing]">ordinary text</span>
""",
        source="source.md",
    )

    assert all(link.form != "undefined_reference" for link in document.links)


def test_undefined_reference_scanner_requires_an_exact_code_span_marker() -> None:
    document = MODULE.parse_document(
        "``one ``` two [literal][missing] three`` [real][missing]\n",
        source="source.md",
    )

    undefined = [
        (link.line, link.destination)
        for link in document.links
        if link.form == "undefined_reference"
    ]

    assert undefined == [(1, "missing")]


def test_gitlink_descendants_are_owned_by_the_pinned_submodule(tmp_path: Path) -> None:
    upstream = tmp_path / "upstream"
    upstream.mkdir()
    subprocess.run(["git", "init", "-q"], cwd=upstream, check=True)
    (upstream / "README.md").write_text("# Upstream\n", encoding="utf-8")
    subprocess.run(["git", "add", "README.md"], cwd=upstream, check=True)
    subprocess.run(
        [
            "git",
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-q",
            "-m",
            "fixture",
        ],
        cwd=upstream,
        check=True,
    )
    root = _git_repo(
        tmp_path,
        {"README.md": "[upstream](vendor/README.md#not-owned-here)\n"},
    )
    subprocess.run(
        [
            "git",
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "-q",
            str(upstream),
            "vendor",
        ],
        cwd=root,
        check=True,
    )
    assert _audit(root).local_links == 1


def test_allowance_must_match_one_exact_finding(tmp_path: Path) -> None:
    destination = "target.md#old-title"
    root = _git_repo(
        tmp_path,
        {"README.md": f"[old]({destination})\n", "target.md": "# New title\n"},
    )
    exact = MODULE.IssueKey(
        source="README.md",
        line=1,
        destination=destination,
        kind="missing_heading_fragment",
    )
    assert _audit(root, (exact,)).allowed_findings == 1
    stale = MODULE.IssueKey("README.md", 2, destination, "missing_heading_fragment")
    with pytest.raises(MODULE.MarkdownAuditError, match="allowance no longer matches"):
        _audit(root, (stale,))


def test_file_and_link_limits_are_enforced(tmp_path: Path, monkeypatch) -> None:
    root = _git_repo(
        tmp_path,
        {
            "README.md": "[one](one.txt) [two](two.txt)\n",
            "one.txt": "1",
            "two.txt": "2",
        },
    )
    monkeypatch.setattr(MODULE, "MAX_LINKS", 1)
    with pytest.raises(MODULE.MarkdownAuditError, match="link-count"):
        _audit(root)
