#!/usr/bin/env python3
"""Audit tracked Markdown links without network access.

``markdown-it-py`` owns CommonMark parsing. This script adds only repository policy:
local targets must exist and be tracked, local Markdown fragments must resolve, and
one immutable archive defect has an exact allowance. External URLs are counted but
not fetched.

This is a documentation consistency check. It is not an adversarial-filesystem or
subprocess security boundary.
"""

from __future__ import annotations

import argparse
import html
import posixpath
import re
import stat
import subprocess
import sys
import unicodedata
from collections import Counter
from dataclasses import dataclass
from html.parser import HTMLParser
from pathlib import Path, PurePosixPath
from urllib.parse import SplitResult, unquote_to_bytes, urlsplit

from markdown_it import MarkdownIt
from markdown_it.common.utils import normalizeReference
from markdown_it.rules_inline.state_inline import StateInline
from markdown_it.token import Token

ROOT = Path(__file__).resolve().parents[1]
MAX_GIT_OUTPUT_BYTES = 4 * 1024 * 1024
MAX_MARKDOWN_FILES = 4_096
MAX_MARKDOWN_FILE_BYTES = 8 * 1024 * 1024
MAX_TOTAL_MARKDOWN_BYTES = 64 * 1024 * 1024
MAX_LINKS = 100_000
MAX_LINK_DESTINATION_CHARS = 4_096
GIT_TIMEOUT_SECONDS = 10.0
SCHEME_RE = re.compile(r"^[A-Za-z][A-Za-z0-9+.-]*:")
PERCENT_ESCAPE_RE = re.compile(r"%[0-9A-Fa-f]{2}")
ALLOWED_EXTERNAL_SCHEMES = frozenset({"http", "https", "mailto"})


class MarkdownAuditError(RuntimeError):
    """The documentation set violates the offline link contract."""


@dataclass(frozen=True, order=True)
class IssueKey:
    source: str
    line: int
    destination: str
    kind: str


@dataclass(frozen=True)
class LinkOccurrence:
    source: str
    line: int
    destination: str
    form: str


@dataclass(frozen=True)
class Finding:
    key: IssueKey
    detail: str


@dataclass(frozen=True)
class ParsedDocument:
    headings: frozenset[str]
    links: tuple[LinkOccurrence, ...]


@dataclass(frozen=True)
class AuditReport:
    markdown_files: int
    parsed_links: int
    local_links: int
    heading_fragments: int
    external_links: int
    allowed_findings: int


# Immutable historical intake. Do not alter archive bytes to repair this old TOC link.
HISTORICAL_ARCHIVE_ALLOWANCE = IssueKey(
    source="docs/archive/grandplan-v10.7.md",
    line=1316,
    destination="#16-why-pca-and-knn-are-suboptimal-for-manifold-valued-embeddings",
    kind="missing_heading_fragment",
)
DEFAULT_ALLOWANCES = frozenset({HISTORICAL_ARCHIVE_ALLOWANCE})


def _tracked_paths(root: Path) -> tuple[frozenset[str], frozenset[str]]:
    try:
        completed = subprocess.run(
            ["git", "-c", "core.quotePath=false", "ls-files", "-s", "-z", "--"],
            cwd=root,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=GIT_TIMEOUT_SECONDS,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise MarkdownAuditError(f"cannot enumerate tracked paths: {error}") from error
    if len(completed.stdout) + len(completed.stderr) > MAX_GIT_OUTPUT_BYTES:
        raise MarkdownAuditError("Git path inventory exceeds its output limit")
    if completed.returncode != 0:
        detail = completed.stderr.decode("utf-8", "replace").strip()
        raise MarkdownAuditError(f"cannot enumerate tracked paths: {detail}")

    tracked: set[str] = set()
    gitlinks: set[str] = set()
    for entry in completed.stdout.split(b"\0"):
        if not entry:
            continue
        try:
            metadata, raw_path = entry.split(b"\t", 1)
            mode = metadata.split(b" ", 1)[0]
            path = raw_path.decode("utf-8")
        except (ValueError, UnicodeDecodeError) as error:
            raise MarkdownAuditError(
                "Git returned an invalid path inventory"
            ) from error
        tracked.add(path)
        if mode == b"160000":
            gitlinks.add(path)
    return frozenset(tracked), frozenset(gitlinks)


def _read_markdown(path: Path) -> str:
    try:
        metadata = path.lstat()
    except OSError as error:
        raise MarkdownAuditError(
            f"cannot inspect tracked Markdown file {path}"
        ) from error
    if not stat.S_ISREG(metadata.st_mode):
        raise MarkdownAuditError(f"tracked Markdown path is not a regular file: {path}")
    if metadata.st_size > MAX_MARKDOWN_FILE_BYTES:
        raise MarkdownAuditError(
            f"tracked Markdown file exceeds {MAX_MARKDOWN_FILE_BYTES} bytes: {path}"
        )
    try:
        with path.open("rb") as stream:
            payload = stream.read(MAX_MARKDOWN_FILE_BYTES + 1)
        if len(payload) > MAX_MARKDOWN_FILE_BYTES:
            raise MarkdownAuditError(
                f"tracked Markdown file exceeds {MAX_MARKDOWN_FILE_BYTES} bytes: {path}"
            )
        return payload.decode("utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise MarkdownAuditError(f"cannot read tracked Markdown file {path}") from error


def _heading_text(inline: Token) -> str:
    output: list[str] = []
    for child in inline.children or ():
        if child.type in {"text", "code_inline"}:
            output.append(child.content)
        elif child.type == "image":
            output.append(child.content)
        elif child.type in {"softbreak", "hardbreak"}:
            output.append(" ")
        elif child.type == "html_inline":
            output.append(re.sub(r"<[^>]*>", "", child.content))
    return html.unescape("".join(output)).strip()


def github_heading_slug(value: str) -> str:
    """Return the GitHub heading slug form used by Prisoma's current docs."""

    kept: list[str] = []
    for char in value.strip().lower():
        category = unicodedata.category(char)
        if (category.startswith("P") or category.startswith("S")) and char not in {
            "-",
            "_",
        }:
            continue
        if category.startswith("C"):
            continue
        kept.append("-" if char.isspace() else char)
    return "".join(kept)


class _HtmlLinks(HTMLParser):
    def __init__(self, *, source: str, base_line: int) -> None:
        super().__init__(convert_charrefs=True)
        self.source = source
        self.base_line = base_line
        self.links: list[LinkOccurrence] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        wanted = (
            "href" if tag.lower() == "a" else "src" if tag.lower() == "img" else None
        )
        if wanted is None:
            return
        for name, value in attrs:
            if name.lower() == wanted and value is not None:
                self.links.append(
                    LinkOccurrence(
                        source=self.source,
                        line=self.base_line + self.getpos()[0],
                        destination=value,
                        form=f"html_{wanted}",
                    )
                )

    handle_startendtag = handle_starttag


def _undefined_explicit_references(
    inline: Token,
    *,
    source: str,
    references: frozenset[str],
    parser: MarkdownIt,
) -> list[LinkOccurrence]:
    """Find undefined full and collapsed references in the original inline source."""

    text = inline.content
    base_line = (inline.map[0] if inline.map else 0) + 1
    links: list[LinkOccurrence] = []
    environment = {
        "references": {
            label: {"href": "https://reference.invalid/", "title": ""}
            for label in references
        }
    }
    state = StateInline(text, parser, environment, [])
    while state.pos < state.posMax:
        start = state.pos
        image = text[start : start + 2] == "!["
        opening = start + 1 if image else start
        if text[opening : opening + 1] == "[":
            label_end = parser.helpers.parseLinkLabel(state, opening, True)
            if label_end >= 0:
                after_label = label_end + 1
                if text[after_label : after_label + 1] == "[":
                    reference_end = parser.helpers.parseLinkLabel(state, after_label)
                    if reference_end >= 0:
                        label = text[opening + 1 : label_end]
                        raw_reference = text[after_label + 1 : reference_end] or label
                        after_reference = reference_end + 1
                        if (
                            label
                            and len(label) <= 999
                            and len(raw_reference) <= 999
                            and not (label.isdecimal() and raw_reference.isdecimal())
                            and normalizeReference(raw_reference) not in references
                        ):
                            links.append(
                                LinkOccurrence(
                                    source,
                                    base_line + text.count("\n", 0, opening),
                                    raw_reference,
                                    "undefined_reference",
                                )
                            )
                        state.pos = after_reference
                        continue

        parser.inline.skipToken(state)
        if (
            state.pos <= start
        ):  # Defensive progress guard around third-party parser rules.
            state.pos = start + 1
    return links


def _inline_links(inline: Token, *, source: str) -> list[LinkOccurrence]:
    line = (inline.map[0] if inline.map else 0) + 1
    links: list[LinkOccurrence] = []
    for child in inline.children or ():
        if child.type == "link_open":
            destination = child.attrGet("href")
            if destination is not None:
                links.append(LinkOccurrence(source, line, destination, "markdown_link"))
        elif child.type == "image":
            destination = child.attrGet("src")
            if destination is not None:
                links.append(
                    LinkOccurrence(source, line, destination, "markdown_image")
                )
        elif child.type == "html_inline":
            parser = _HtmlLinks(source=source, base_line=line - 1)
            parser.feed(child.content)
            links.extend(parser.links)
    return links


def parse_document(text: str, *, source: str) -> ParsedDocument:
    parser = MarkdownIt("commonmark")

    # Parse every syntactically valid destination. Markdown-it otherwise drops unsafe schemes
    # before repository policy can report them. This script never renders the parsed HTML.
    def retain_destination_for_policy(_destination: str) -> bool:
        return True

    parser.validateLink = retain_destination_for_policy
    environment: dict[str, object] = {}
    tokens = parser.parse(text, environment)
    raw_references = dict(environment.get("references", {}))
    references = frozenset(str(key) for key in raw_references)
    links: list[LinkOccurrence] = []
    heading_bases: list[str] = []
    in_heading = False

    for token in tokens:
        if token.type == "heading_open":
            in_heading = True
        elif token.type == "heading_close":
            in_heading = False
        elif token.type == "inline":
            if in_heading:
                heading_bases.append(github_heading_slug(_heading_text(token)))
            links.extend(_inline_links(token, source=source))
            links.extend(
                _undefined_explicit_references(
                    token, source=source, references=references, parser=parser
                )
            )
        elif token.type in {"html_block", "html_inline"}:
            base_line = token.map[0] if token.map else 0
            html_parser = _HtmlLinks(source=source, base_line=base_line)
            html_parser.feed(token.content)
            links.extend(html_parser.links)

    for value in raw_references.values():
        if not isinstance(value, dict):
            continue
        destination = value.get("href")
        line_map = value.get("map")
        if isinstance(destination, str):
            line = line_map[0] + 1 if isinstance(line_map, list) and line_map else 1
            links.append(
                LinkOccurrence(source, line, destination, "reference_definition")
            )

    counts: Counter[str] = Counter()
    headings: set[str] = set()
    for base in heading_bases:
        suffix = counts[base]
        counts[base] += 1
        headings.add(base if suffix == 0 else f"{base}-{suffix}")
    return ParsedDocument(frozenset(headings), tuple(links))


def _decode_component(value: str, *, label: str) -> str:
    cursor = 0
    while cursor < len(value):
        if value[cursor] == "%":
            match = PERCENT_ESCAPE_RE.match(value, cursor)
            if match is None:
                raise ValueError(f"{label} contains an invalid percent escape")
            cursor = match.end()
        else:
            cursor += 1
    try:
        decoded = unquote_to_bytes(value).decode("utf-8")
    except UnicodeDecodeError as error:
        raise ValueError(f"{label} is not percent-encoded UTF-8") from error
    if any(unicodedata.category(char).startswith("C") for char in decoded):
        raise ValueError(f"{label} contains a control character")
    return decoded


def _external_destination_error(destination: str, split: SplitResult) -> str | None:
    scheme = split.scheme.lower()
    if not scheme:
        return "scheme-relative external URLs are not allowed"
    if scheme not in ALLOWED_EXTERNAL_SCHEMES:
        return f"external URL scheme {scheme!r} is not allowed"
    try:
        _decode_component(split.netloc, label="URL authority")
        _decode_component(split.path, label="URL path")
        _decode_component(split.query, label="URL query")
        _decode_component(split.fragment, label="URL fragment")
    except ValueError as error:
        return str(error)
    if any(char.isspace() for char in destination):
        return "external URL contains unescaped whitespace"
    if scheme in {"http", "https"}:
        try:
            hostname = split.hostname
            _ = split.port
        except ValueError as error:
            return f"external URL authority is invalid: {error}"
        if not split.netloc or hostname is None:
            return "HTTP URL must include a host"
        if split.username is not None or split.password is not None:
            return "HTTP URL must not embed credentials"
    elif split.netloc or not split.path:
        return "mailto URL must name a recipient without an authority"
    return None


def _finding(link: LinkOccurrence, kind: str, detail: str) -> Finding:
    return Finding(IssueKey(link.source, link.line, link.destination, kind), detail)


def _target_is_inside_gitlink(target: str, gitlinks: frozenset[str]) -> bool:
    return any(
        target == gitlink or target.startswith(f"{gitlink}/") for gitlink in gitlinks
    )


def _validate_link(
    link: LinkOccurrence,
    *,
    root: Path,
    tracked: frozenset[str],
    gitlinks: frozenset[str],
    headings: dict[str, frozenset[str]],
) -> tuple[Finding | None, bool, bool]:
    destination = link.destination.strip()
    if link.form == "undefined_reference":
        return (
            _finding(link, "undefined_reference", "reference label is not defined"),
            False,
            False,
        )
    if len(destination) > MAX_LINK_DESTINATION_CHARS or any(
        unicodedata.category(char).startswith("C") for char in destination
    ):
        return (
            _finding(link, "invalid_destination", "link destination is invalid"),
            False,
            False,
        )
    if "\\" in destination:
        return (
            _finding(
                link, "invalid_destination", "local links must use URL separators"
            ),
            False,
            False,
        )
    try:
        split = urlsplit(destination)
    except ValueError as error:
        return (
            _finding(link, "invalid_destination", f"cannot parse link: {error}"),
            False,
            False,
        )
    if (
        SCHEME_RE.match(destination)
        or split.scheme
        or split.netloc
        or destination.startswith("//")
    ):
        error = _external_destination_error(destination, split)
        if error is not None:
            return _finding(link, "invalid_external_url", error), False, False
        return None, False, False
    if split.path.startswith("/"):
        return (
            _finding(
                link,
                "out_of_repo_target",
                "root-relative link is outside the repository",
            ),
            True,
            False,
        )
    try:
        decoded_path = _decode_component(split.path, label="link path")
        fragment = _decode_component(split.fragment, label="link fragment")
    except ValueError as error:
        return _finding(link, "invalid_destination", str(error)), True, False
    if "\\" in decoded_path:
        return (
            _finding(
                link, "invalid_destination", "local links must use URL separators"
            ),
            True,
            False,
        )
    source = PurePosixPath(link.source)
    combined = source.parent / decoded_path if decoded_path else source
    normalized = PurePosixPath(posixpath.normpath(combined.as_posix()))
    if (
        normalized.is_absolute()
        or normalized.as_posix() == ".."
        or ".." in normalized.parts
    ):
        return (
            _finding(link, "out_of_repo_target", "local target escapes the repository"),
            True,
            False,
        )
    target = normalized.as_posix()
    target_path = root / Path(*normalized.parts)
    try:
        metadata = target_path.lstat()
    except OSError:
        return (
            _finding(link, "missing_target", f"local target does not exist: {target}"),
            True,
            False,
        )
    if stat.S_ISLNK(metadata.st_mode):
        return (
            _finding(
                link, "non_regular_target", f"local target is a symlink: {target}"
            ),
            True,
            False,
        )

    inside_gitlink = _target_is_inside_gitlink(target, gitlinks)
    if stat.S_ISREG(metadata.st_mode):
        if target not in tracked and not inside_gitlink:
            return (
                _finding(
                    link, "untracked_target", f"local target is not tracked: {target}"
                ),
                True,
                False,
            )
    elif stat.S_ISDIR(metadata.st_mode):
        prefix = "" if target == "." else f"{target.rstrip('/')}/"
        if not inside_gitlink and not any(path.startswith(prefix) for path in tracked):
            return (
                _finding(
                    link,
                    "untracked_target",
                    f"local directory has no tracked content: {target}",
                ),
                True,
                False,
            )
        if fragment:
            return (
                _finding(
                    link, "invalid_destination", "directory links cannot name a heading"
                ),
                True,
                False,
            )
    else:
        return (
            _finding(
                link,
                "non_regular_target",
                f"local target is not a file or directory: {target}",
            ),
            True,
            False,
        )

    heading_fragment = bool(
        fragment
        and PurePosixPath(target).suffix.lower() == ".md"
        and not inside_gitlink
    )
    if heading_fragment:
        accepted = fragment.removeprefix("user-content-")
        if accepted not in headings.get(target, frozenset()):
            return (
                _finding(
                    link,
                    "missing_heading_fragment",
                    f"Markdown target {target} has no heading slug {fragment!r}",
                ),
                True,
                True,
            )
    return None, True, heading_fragment


def audit_repository(
    root: Path = ROOT,
    *,
    allowances: frozenset[IssueKey] = DEFAULT_ALLOWANCES,
) -> AuditReport:
    root = root.resolve()
    allowances = frozenset(allowances)
    tracked, gitlinks = _tracked_paths(root)
    markdown = tuple(sorted(path for path in tracked if path.lower().endswith(".md")))
    if len(markdown) > MAX_MARKDOWN_FILES:
        raise MarkdownAuditError("tracked Markdown exceeds its file-count limit")
    documents: dict[str, ParsedDocument] = {}
    total_bytes = 0
    for relative in markdown:
        path = root / relative
        text = _read_markdown(path)
        total_bytes += len(text.encode("utf-8"))
        if total_bytes > MAX_TOTAL_MARKDOWN_BYTES:
            raise MarkdownAuditError(
                "tracked Markdown exceeds its aggregate byte limit"
            )
        documents[relative] = parse_document(text, source=relative)

    links = tuple(link for document in documents.values() for link in document.links)
    if len(links) > MAX_LINKS:
        raise MarkdownAuditError("tracked Markdown exceeds its link-count limit")
    headings = {path: document.headings for path, document in documents.items()}
    findings: list[Finding] = []
    used_allowances: set[IssueKey] = set()
    local = fragments = external = 0
    for link in links:
        finding, is_local, is_fragment = _validate_link(
            link,
            root=root,
            tracked=tracked,
            gitlinks=gitlinks,
            headings=headings,
        )
        local += int(is_local)
        fragments += int(is_fragment)
        external += int(not is_local and finding is None)
        if finding is None:
            continue
        if finding.key in allowances:
            used_allowances.add(finding.key)
        else:
            findings.append(finding)
    stale = allowances - used_allowances
    if stale:
        keys = ", ".join(f"{key.source}:{key.line}" for key in sorted(stale))
        raise MarkdownAuditError(
            f"Markdown allowance no longer matches a finding: {keys}"
        )
    if findings:
        details = "\n".join(
            f"{item.key.source}:{item.key.line}: {item.key.kind}: "
            f"{item.key.destination!r}: {item.detail}"
            for item in sorted(findings, key=lambda item: item.key)
        )
        raise MarkdownAuditError(f"Markdown link audit failed:\n{details}")
    return AuditReport(
        len(markdown), len(links), local, fragments, external, len(used_allowances)
    )


def _parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = _parse_args(argv)
    try:
        report = audit_repository(args.root)
    except MarkdownAuditError as error:
        print(error, file=sys.stderr)
        return 1
    print(
        "Markdown link audit passed: "
        f"files={report.markdown_files}, links={report.parsed_links}, "
        f"local={report.local_links}, heading_fragments={report.heading_fragments}, "
        f"external_skipped={report.external_links}, allowances={report.allowed_findings}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
