#!/usr/bin/env python3
"""
Heuristic audit for the canonical docset to surface high-risk doc drift.

This extends the `audit_grandplan_claims.py` idea beyond `grandplan.md` by scanning:
- grandplan.md (canonical spec)
- README.md, ARCHITECTURE.md, EXPERIMENTS.md, DIAGRAMS.md, pidsplatspecs.md (canonical companion docs)
- findings.md (repo-local results summary; should still avoid unqualified claims)

In addition to claim wording, the audit protects a few docset-wide invariants that are easy
to break during large edits:

- mutating clients and observers may not bypass the Agent Bridge;
- Nerfstudio's Gaussian-splat exporter produces PLY, not native SPZ;
- active section references must resolve to a real heading; and
- active NCP pin claims must match `crates/ncp-observer/Cargo.toml`.

The checks are intentionally conservative and *not* a proof of correctness. Clearly marked
version-history/changelog text is excluded from the architecture/pin drift checks so that an
honest record of an old design or pin does not become a false positive.
"""

from __future__ import annotations

import argparse
import re
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


DEFAULT_PATHS = [
    Path("grandplan.md"),
    Path("README.md"),
    Path("ARCHITECTURE.md"),
    Path("EXPERIMENTS.md"),
    Path("DIAGRAMS.md"),
    Path("pidsplatspecs.md"),
    Path("findings.md"),
]
DEFAULT_NCP_MANIFEST = Path("crates/ncp-observer/Cargo.toml")
SUPPLEMENTAL_NCP_DOCS = [
    Path("AGENTS.md"),
    Path("CLAUDE.md"),
    Path("NCP_DEV_PROMPT.md"),
    Path("RESEARCH_VLA_D_NCP.md"),
    Path("REVIEW_AND_TODO.md"),
    Path("crates/ncp-observer/README.md"),
]

ARXIV_ID_RE = re.compile(r"arXiv:(\d{4}\.\d{5})")
VENUE_RE = re.compile(r"\b(NeurIPS|ICML|CoRL|ICLR|CVPR|ECCV|AAAI)\b")
REFERENCES_HEADING_RE = re.compile(r"^#{1,6}\s+References\b", re.IGNORECASE)

# Performance/cost-ish signals. Keep these narrow to reduce false positives.
PERCENT_RE = re.compile(r"\b\d+(?:\.\d+)?%")
MULT_RE = re.compile(r"\b\d+(?:\.\d+)?x\b|\b\d+(?:\.\d+)?×\b")
UNITS_RE = re.compile(
    r"\b(\d+(?:\.\d+)?)\s*(ms|fps|GB|GiB|TB|hours?|mins?|seconds?)\b", re.IGNORECASE
)

PERF_WORD_RE = re.compile(
    r"\b("
    r"accuracy|success|auroc|roc|score|outperform|improv|state-of-the-art|sota|"
    r"latency|throughput|speed|memory|vram|cost|budget"
    r")\b",
    re.IGNORECASE,
)

QUALIFIER_RE = re.compile(
    r"\b("
    r"verify|paper-reported|abstract|benchmark|protocol-sensitive|"
    r"illustrative|example|heuristic|depends|variable|order-of-magnitude|"
    r"approx|lower bound|not a guarantee|do not treat|"
    r"do not assume|do not cite|measure|measured"
    r")\b",
    re.IGNORECASE,
)

HEADING_RE = re.compile(r"^(?P<marks>#{1,6})\s+(?P<title>.+?)\s*$")
SECTION_ID_PATTERN = r"(?:[A-Z](?:\.\d+[a-z]?)*|\d+[A-Z]?(?:\.\d+[a-z]?)*|\d+)"
SECTION_HEADING_RE = re.compile(
    rf"^(?:§\s*)?(?P<section>{SECTION_ID_PATTERN})\b", re.IGNORECASE
)
SECTION_REF_RE = re.compile(
    rf"§{{1,2}}\s*(?P<section>{SECTION_ID_PATTERN})\b", re.IGNORECASE
)
DOC_NAME_RE = re.compile(
    r"(?P<doc>grandplan\.md|README\.md|ARCHITECTURE\.md|EXPERIMENTS\.md|"
    r"DIAGRAMS\.md|pidsplatspecs\.md|findings\.md)",
    re.IGNORECASE,
)

HISTORICAL_HEADING_RE = re.compile(
    r"\b(version history|change ?log|historical (?:notes?|record)|release notes?|"
    r"implementation pass status)\b",
    re.IGNORECASE,
)
HISTORICAL_LINE_RE = re.compile(
    r"^\s*(?:[-*+>]\s*)?(?:\*\*)?(?:"
    r"historical(?:ly)?|legacy note|previous risk|superseded|previously|formerly|"
    r"retired (?:section|number)|earlier drafts?|prior versions?|at-cut notes?"
    r")\b|\bold\s+[`*]*v\d+\.\d+\.\d+[`*]*\s+pin\b",
    re.IGNORECASE,
)
VERSION_NOTES_START_RE = re.compile(
    r"^\s*\*\*v?\d+(?:\.\d+)+(?:\s+FINAL)?\s+"
    r"(?:notes?|updates?|slice|cut)\b",
    re.IGNORECASE,
)

CONTROL_SOURCE_RE = re.compile(
    r"\b("
    r"ui|gui|clients?|vla|polic(?:y|ies)|scripts?|tools?|llms?|claude|mcp|"
    r"tauri|spark(?:js)?|zenoh|rerun|viewers?|observers?|"
    r"pid(?:\s+(?:worker|core|estimator))?"
    r")\b",
    re.IGNORECASE,
)
PROSE_CONTROL_SOURCE_RE = re.compile(
    r"\b("
    r"ui|gui|clients?|vla|polic(?:y|ies)|scripts?|tools?|llms?|claude|mcp|"
    r"tauri|spark(?:js)?|zenoh|rerun|viewers?|observers?|"
    r"pid(?![-‑])(?:\s+(?:worker|core|estimator))?"
    r")\b",
    re.IGNORECASE,
)
CONTROL_SINK_RE = re.compile(
    r"\b("
    r"physics|phys|sim|simulator|simulation|backend|rapier|gazebo|mujoco|isaac|"
    r"scene(?:\s+(?:edit|mutation))?|intervention|actuator|robot"
    r")\b",
    re.IGNORECASE,
)
PROSE_CONTROL_SINK_RE = re.compile(
    r"\b("
    r"physics|phys|sim|simulator|simulation|backend|rapier|gazebo|mujoco|isaac|"
    r"actuator|robot"
    r")\b",
    re.IGNORECASE,
)
CONTROL_VERB_RE = re.compile(
    r"\b("
    r"control(?:s|led|ling)?|drive(?:s|n)?|actuat(?:e|es|ed|ing)|"
    r"dispatch(?:es|ed|ing)?|apply|applies|applied|applying|"
    r"send(?:s|ing)?|issue(?:s|d|ing)?|trigger(?:s|ed|ing)?|"
    r"command(?:s|ed|ing)?|mutat(?:e|es|ed|ing)|edit(?:s|ed|ing)?|"
    r"step(?:s|ped|ping)?|reset(?:s|ting)?|pause(?:s|d|ing)?|resume(?:s|d|ing)?"
    r")\b",
    re.IGNORECASE,
)
NEGATED_CONTROL_RE = re.compile(
    r"\b(?:avoid|forbid(?:s|den)?|prohibit(?:s|ed)?|never|cannot|can't|"
    r"must\s+not|may\s+not|does\s+not|do\s+not|should\s+not|"
    r"is\s+not|are\s+not|no|none|neither)\b[^.;:]{0,80}" + CONTROL_VERB_RE.pattern,
    re.IGNORECASE,
)
NEGATED_PATH_RE = re.compile(
    r"\b(?:avoid|forbid(?:s|den)?|prohibit(?:s|ed)?|never|cannot|can't|"
    r"must\s+not|may\s+not|does\s+not|do\s+not|should\s+not|"
    r"is\s+not|are\s+not|no|none|neither)\b",
    re.IGNORECASE,
)
BRIDGE_BYPASS_RE = re.compile(
    r"\b(?:bypass(?:es|ed|ing)?|without|outside)\b[^.;]{0,50}\bAgent Bridge\b|"
    r"\bAgent Bridge\b[^.;]{0,50}\b(?:bypass(?:es|ed|ing)?|without|outside)\b",
    re.IGNORECASE,
)
BRIDGE_ROUTING_RE = re.compile(
    r"\b(?:through|via|into|to)\b[^.;:]{0,35}\bAgent[- ]Bridge\b|"
    r"\b(?:submits?|routes?)\b[^.;:]{0,35}\bAgent[- ]Bridge\b",
    re.IGNORECASE,
)
AGENT_BRIDGE_RE = re.compile(r"\bAgent[- ]Bridge\b", re.IGNORECASE)
BRIDGE_ANAPHORA_RE = re.compile(r"\bthrough\s+(?:it|the\s+bridge)\b", re.IGNORECASE)
CLAUSE_SPLIT_RE = re.compile(
    r"(?<=[.;!?])\s+|,\s*(?:but|however|while|whereas|yet)\s+",
    re.IGNORECASE,
)
CONTROL_PLANE_SAFETY_RE = re.compile(
    r"\b(?:same|only)\s+control plane\b|\bno hidden\b[^.;]{0,25}\bpaths?\b|"
    r"\bcanonical run log\b[^.;]{0,35}\bbefore dispatch\b",
    re.IGNORECASE,
)

DIRECT_CONTROL_FORWARD_RE = re.compile(
    PROSE_CONTROL_SOURCE_RE.pattern
    + r"[^.;:]{0,70}"
    + CONTROL_VERB_RE.pattern
    + r"[^.;:]{0,70}"
    + PROSE_CONTROL_SINK_RE.pattern,
    re.IGNORECASE,
)
DIRECT_CONTROL_REVERSE_RE = re.compile(
    PROSE_CONTROL_SINK_RE.pattern
    + r"[^.;:]{0,70}"
    + CONTROL_VERB_RE.pattern
    + r"[^.;:]{0,25}\bby\s+"
    + PROSE_CONTROL_SOURCE_RE.pattern,
    re.IGNORECASE,
)
DIRECT_CONTROL_ARROW_RE = re.compile(
    PROSE_CONTROL_SOURCE_RE.pattern + r"\s*(?:→|->|-->)\s*" + CONTROL_SINK_RE.pattern,
    re.IGNORECASE,
)
DIRECT_CONTROL_NOUN_RE = re.compile(
    PROSE_CONTROL_SOURCE_RE.pattern
    + r"[^.;:]{0,50}\bdirect(?:ly)?\b[^.;:]{0,35}"
    + r"(?:\b(?:controls?|commands?|actuation)\b[^.;:]{0,25}"
    + CONTROL_SINK_RE.pattern
    + r"|"
    + CONTROL_SINK_RE.pattern
    + r"[^.;:]{0,25}\b(?:controls?|commands?|actuation)\b)",
    re.IGNORECASE,
)
DIRECT_MUTATION_RE = re.compile(
    PROSE_CONTROL_SOURCE_RE.pattern
    + r"\s+(?:(?:can|may|will|must|directly)\s+)*"
    + r"(?:edits?|appl(?:y|ies)|triggers?|issues?)\b[^.;:]{0,45}"
    + r"\b(?:scene|intervention)\b",
    re.IGNORECASE,
)

# Mermaid flowchart and sequence-diagram edges. The lookahead permits overlapping edges in
# chains such as `UI --> Sim --> Log`.
DIAGRAM_EDGE_RE = re.compile(
    r"(?=(?P<src>[A-Za-z_][\w-]*)"
    r"(?:\s*(?:\[[^\]\n]*\]|\([^\)\n]*\)|\{[^}\n]*\}))?\s*"
    r"(?P<edge>--[^>\n]*-->|-->|==>|-\.[^>\n]*\.->|-\.->|-->>|->>|->)\s*"
    r"(?:\|(?P<label>[^|]*)\|\s*)?"
    r"(?P<dst>[A-Za-z_][\w-]*))"
)
NODE_DECL_RE = re.compile(
    r"\b(?P<node>[A-Za-z_][\w-]*)\s*"
    r"(?:\[\s*[\"']?(?P<bracket>[^\]\n]+?)[\"']?\s*\]|"
    r"\(\s*[\"']?(?P<round>[^)\n]+?)[\"']?\s*\)|"
    r"\{\s*[\"']?(?P<brace>[^}\n]+?)[\"']?\s*\})"
)
PARTICIPANT_RE = re.compile(
    r"^\s*participant\s+(?P<node>[A-Za-z_][\w-]*)\s+as\s+(?P<label>.+?)\s*$",
    re.IGNORECASE,
)

SPZ_FLAG_RE = re.compile(r"--output-format(?:\s+|=)[`'\"]?spz\b", re.IGNORECASE)
NERFSTUDIO_RE = re.compile(
    r"\b(?:Nerfstudio|ns-export(?:\s+gaussian-splat)?)\b", re.IGNORECASE
)
NERFSTUDIO_SPZ_CLAIM_RE = re.compile(
    r"\b(?:Nerfstudio|ns-export(?:\s+gaussian-splat)?)\b[^.;]{0,80}"
    r"\b(?:native(?:ly)?|direct(?:ly)?|built[ -]in|supports?|exports?|"
    r"output(?: format)?|produces?|writes?)\b[^.;]{0,50}(?:\bSPZ\b|\.spz\b)|"
    r"(?:\bSPZ\b|\.spz\b)[^.;]{0,50}\b(?:native(?:ly)?|direct(?:ly)?|"
    r"built[ -]in|export(?:s|ed)?|output)\b[^.;]{0,50}"
    r"\b(?:by|from|of)\s+(?:Nerfstudio|ns-export)\b",
    re.IGNORECASE,
)
NERFSTUDIO_PLY_CONVERTER_RE = re.compile(
    r"\b(?:Nerfstudio|ns-export(?:\s+gaussian-splat)?)\b[^.;]{0,60}"
    r"\b(?:exports?|produces?|writes?|output)\b[^.;]{0,35}\bPLY\b"
    r"[^.;]{0,60}\bconverter\b[^.;]{0,45}\bSPZ\b",
    re.IGNORECASE,
)
SPZ_DISCLAIMER_RE = re.compile(
    r"\b(?:do\s+not|never)\s+(?:pass|use|claim)|"
    r"\b(?:does\s+not|cannot|can't)\b[^.;]{0,35}"
    r"\b(?:export|support|produce|write)|"
    r"\b(?:invalid|invented|unsupported)\b|"
    r"\bseparate(?:ly)?\b[^.;]{0,35}\bconverter\b|"
    r"\bdistinct\b[^.;]{0,35}\b(?:dependency|step|tool)\b|"
    r"\bSPZ\b[^.;]{0,30}\bis\s+not\b[^.;]{0,35}\b(?:output|export|flag)|"
    r"\bnot\s+(?:native\s+)?SPZ\b",
    re.IGNORECASE,
)

NCP_NAME_RE = re.compile(r"\b(?:NCP|Neuro[- ]Cybernetic Protocol)\b", re.IGNORECASE)
SEMVER_TAG_RE = re.compile(r"\bv\d+\.\d+\.\d+\b", re.IGNORECASE)
NCP_PIN_CUE_RE = re.compile(
    r"\b(?:pin(?:ned)?|tag|dependency|version|tap|repo|synced|currently|update)\b",
    re.IGNORECASE,
)


@dataclass(frozen=True)
class Finding:
    kind: str
    path: Path
    line_no: int
    line: str


@dataclass(frozen=True)
class LineContext:
    line_no: int
    line: str
    in_fence: bool
    fence_language: str
    historical: bool


def iter_line_contexts(path: Path, lines: list[str]) -> Iterable[LineContext]:
    """Yield Markdown lines with fence and clearly-historical context attached."""

    in_fence = False
    fence_language = ""
    heading_stack: list[tuple[int, bool]] = []
    version_notes_block = False
    whole_file_history = path.name.casefold() in {"changelog.md", "history.md"}

    for line_no, line in enumerate(lines, start=1):
        stripped = line.lstrip()
        if stripped.startswith("```"):
            if in_fence:
                context = LineContext(
                    line_no,
                    line,
                    True,
                    fence_language,
                    whole_file_history or version_notes_block,
                )
                yield context
                in_fence = False
                fence_language = ""
            else:
                fence_language = stripped[3:].strip().casefold()
                in_fence = True
                yield LineContext(
                    line_no,
                    line,
                    True,
                    fence_language,
                    whole_file_history or version_notes_block,
                )
            continue

        if not in_fence:
            heading_match = HEADING_RE.match(line)
            if heading_match:
                version_notes_block = False
                level = len(heading_match.group("marks"))
                while heading_stack and heading_stack[-1][0] >= level:
                    heading_stack.pop()
                parent_historical = heading_stack[-1][1] if heading_stack else False
                heading_historical = parent_historical or bool(
                    HISTORICAL_HEADING_RE.search(heading_match.group("title"))
                )
                heading_stack.append((level, heading_historical))
            elif (
                VERSION_NOTES_START_RE.search(line) and "current" not in line.casefold()
            ):
                # grandplan keeps old release notes in a pre-ToC block. Once one starts, all
                # following prose remains historical until the next real Markdown heading.
                version_notes_block = True

        heading_historical = heading_stack[-1][1] if heading_stack else False
        line_historical = bool(HISTORICAL_LINE_RE.search(line))
        yield LineContext(
            line_no=line_no,
            line=line,
            in_fence=in_fence,
            fence_language=fence_language,
            historical=(
                whole_file_history
                or version_notes_block
                or heading_historical
                or line_historical
            ),
        )


def logical_text_contexts(contexts: Iterable[LineContext]) -> list[LineContext]:
    """Join hard-wrapped Markdown prose and backslash-continued shell commands."""

    output: list[LineContext] = []
    buffer: list[LineContext] = []

    def flush() -> None:
        if not buffer:
            return
        first = buffer[0]
        output.append(
            LineContext(
                line_no=first.line_no,
                line=" ".join(item.line.strip() for item in buffer),
                in_fence=first.in_fence,
                fence_language=first.fence_language,
                historical=first.historical,
            )
        )
        buffer.clear()

    for context in contexts:
        stripped = context.line.strip()
        if context.line.lstrip().startswith("```"):
            flush()
            continue
        if context.fence_language == "mermaid":
            flush()
            continue

        if context.in_fence:
            continuing = bool(buffer and buffer[-1].line.rstrip().endswith("\\"))
            if buffer and not continuing:
                flush()
            buffer.append(context)
            if not context.line.rstrip().endswith("\\"):
                flush()
            continue

        if (
            not stripped
            or HEADING_RE.match(context.line)
            or re.fullmatch(r"-{3,}", stripped)
        ):
            flush()
            continue

        starts_new_item = bool(re.match(r"^\s*(?:[-*+]\s+|\d+[.)]\s+)", context.line))
        if buffer and (
            starts_new_item
            or context.historical != buffer[0].historical
            or context.in_fence != buffer[0].in_fence
        ):
            flush()
        buffer.append(context)
        if stripped.startswith("|"):
            flush()

    flush()
    return output


def iter_non_code_lines(lines: list[str]):
    """Backwards-compatible iterator used by the original claim checks."""

    in_fence = False
    for i, line in enumerate(lines, start=1):
        if line.lstrip().startswith("```"):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        yield i, line


def extract_sections(path: Path) -> set[str]:
    """Return numeric/alphanumeric section labels declared by real Markdown headings."""

    lines = path.read_text(encoding="utf-8").splitlines()
    sections: set[str] = set()
    for context in iter_line_contexts(path, lines):
        if context.in_fence:
            continue
        heading_match = HEADING_RE.match(context.line)
        if not heading_match:
            continue
        section_match = SECTION_HEADING_RE.match(heading_match.group("title"))
        if section_match:
            sections.add(section_match.group("section").casefold())
    return sections


def build_section_catalog(paths: Iterable[Path]) -> dict[str, set[str]]:
    catalog: dict[str, set[str]] = {}
    for path in paths:
        if not path.exists():
            continue
        sections = extract_sections(path)
        catalog[path.as_posix().casefold()] = sections
        basename = path.name.casefold()
        # Explicit `README.md`-style references mean the root canonical document. Preserve
        # that entry when a full tracked-Markdown sweep later encounters nested READMEs.
        if basename not in catalog or len(path.parts) == 1:
            catalog[basename] = sections
    return catalog


def _nearest_explicit_doc(
    line: str, reference_start: int, reference_end: int
) -> str | None:
    """Find a nearby document name before or after a section reference."""

    preceding_start = max(0, reference_start - 60)
    preceding = line[preceding_start:reference_start]
    if not line.lstrip().startswith("|"):
        boundaries = list(re.finditer(r"[;!?]|\.(?=\s|$)", preceding))
        if boundaries:
            preceding = preceding[boundaries[-1].end() :]
    preceding_matches = list(DOC_NAME_RE.finditer(preceding))
    if preceding_matches:
        return preceding_matches[-1].group("doc").casefold()

    following = line[reference_end : min(len(line), reference_end + 60)]
    following_match = re.match(
        r"\s*(?:in|of|from)\s+[`*(]*" + DOC_NAME_RE.pattern,
        following,
        re.IGNORECASE,
    )
    if following_match:
        return following_match.group("doc").casefold()
    return None


def _section_reference_findings(
    path: Path,
    context: LineContext,
    section_catalog: dict[str, set[str]],
) -> list[Finding]:
    if context.in_fence or context.historical or HEADING_RE.match(context.line):
        return []

    source_name = path.as_posix().casefold()
    canonical_name = "grandplan.md"
    source_sections = section_catalog.get(
        source_name, section_catalog.get(path.name.casefold(), set())
    )
    canonical_sections = section_catalog.get(canonical_name, set())
    findings: list[Finding] = []

    for match in SECTION_REF_RE.finditer(context.line):
        section = match.group("section").casefold()
        explicit_doc = _nearest_explicit_doc(context.line, match.start(), match.end())
        if explicit_doc is not None:
            target_sections = section_catalog.get(explicit_doc)
            # A caller may intentionally audit a subset that does not contain the referenced
            # document. Do not invent a failure without its heading index.
            if target_sections is None or section in target_sections:
                continue
        else:
            # Unqualified companion-doc references are common shorthand for either the local
            # document or canonical grandplan. Only fail when neither possible target exists.
            if section in source_sections or section in canonical_sections:
                continue

        findings.append(
            Finding(
                "dead_section_reference", path, context.line_no, context.line.rstrip()
            )
        )

    return findings


def _node_text(node: str, labels: dict[str, str]) -> str:
    label = labels.get(node, "")
    return f"{node} {label}".replace("<br/>", " ").replace("\\n", " ")


def _update_mermaid_nodes(line: str, labels: dict[str, str]) -> None:
    participant = PARTICIPANT_RE.match(line)
    if participant:
        labels[participant.group("node")] = participant.group("label")
    for match in NODE_DECL_RE.finditer(line):
        label = (
            match.group("bracket") or match.group("round") or match.group("brace") or ""
        )
        labels[match.group("node")] = label


def _diagram_control_findings(
    path: Path,
    context: LineContext,
    labels: dict[str, str],
) -> list[Finding]:
    if context.historical:
        return []
    if context.fence_language not in {"mermaid", ""} and not DIAGRAM_EDGE_RE.search(
        context.line
    ):
        return []

    findings: list[Finding] = []
    for match in DIAGRAM_EDGE_RE.finditer(context.line):
        source = _node_text(match.group("src"), labels)
        sink = _node_text(match.group("dst"), labels)
        if (
            not AGENT_BRIDGE_RE.search(source)
            and CONTROL_SOURCE_RE.search(source)
            and CONTROL_SINK_RE.search(sink)
        ):
            findings.append(
                Finding(
                    "agent_bridge_bypass_edge",
                    path,
                    context.line_no,
                    context.line.rstrip(),
                )
            )
    return findings


def _all_diagram_control_findings(
    path: Path, contexts: list[LineContext]
) -> list[Finding]:
    """Audit Mermaid blocks after collecting all node/participant labels in each block."""

    findings: list[Finding] = []
    block: list[LineContext] = []
    in_mermaid_block = False
    outside_labels: dict[str, str] = {}

    def flush_block() -> None:
        if not block:
            return
        labels: dict[str, str] = {}
        for item in block:
            _update_mermaid_nodes(item.line, labels)
        for item in block:
            findings.extend(_diagram_control_findings(path, item, labels))
        block.clear()

    for context in contexts:
        if context.line.lstrip().startswith("```"):
            if context.fence_language == "mermaid":
                if in_mermaid_block:
                    flush_block()
                in_mermaid_block = not in_mermaid_block
            continue
        if in_mermaid_block:
            block.append(context)
            continue
        if not context.in_fence:
            if re.match(
                r"^\s*(?:flowchart|graph|sequenceDiagram)\b",
                context.line,
                re.IGNORECASE,
            ):
                outside_labels.clear()
            _update_mermaid_nodes(context.line, outside_labels)
            findings.extend(_diagram_control_findings(path, context, outside_labels))

    flush_block()
    return findings


def _prose_control_finding(path: Path, context: LineContext) -> Finding | None:
    if context.in_fence or context.historical:
        return None
    line = context.line
    for clause in (part.strip() for part in CLAUSE_SPLIT_RE.split(line)):
        if not clause:
            continue
        relation_matches = [
            match
            for pattern in (
                DIRECT_CONTROL_FORWARD_RE,
                DIRECT_CONTROL_REVERSE_RE,
                DIRECT_CONTROL_ARROW_RE,
                DIRECT_CONTROL_NOUN_RE,
                DIRECT_MUTATION_RE,
            )
            for match in pattern.finditer(clause)
        ]
        for match in relation_matches:
            nearby = clause[
                max(0, match.start() - 80) : min(len(clause), match.end() + 60)
            ]
            if NEGATED_CONTROL_RE.search(nearby) or (
                DIRECT_CONTROL_ARROW_RE.search(match.group(0))
                and NEGATED_PATH_RE.search(nearby)
            ):
                continue
            if BRIDGE_BYPASS_RE.search(nearby):
                return Finding(
                    "agent_bridge_bypass_wording", path, context.line_no, line.rstrip()
                )
            if BRIDGE_ROUTING_RE.search(nearby):
                continue
            if AGENT_BRIDGE_RE.search(match.group(0)):
                continue
            if BRIDGE_ANAPHORA_RE.search(nearby) and AGENT_BRIDGE_RE.search(line):
                continue
            if CONTROL_PLANE_SAFETY_RE.search(clause):
                continue
            return Finding(
                "agent_bridge_bypass_wording", path, context.line_no, line.rstrip()
            )
    return None


def _spz_finding(path: Path, context: LineContext) -> Finding | None:
    if context.historical:
        return None
    line = context.line
    for clause in (part.strip() for part in CLAUSE_SPLIT_RE.split(line)):
        if not clause:
            continue
        if SPZ_FLAG_RE.search(clause) and NERFSTUDIO_RE.search(clause):
            if SPZ_DISCLAIMER_RE.search(clause):
                continue
            return Finding(
                "invented_nerfstudio_spz_export", path, context.line_no, line.rstrip()
            )
        if NERFSTUDIO_PLY_CONVERTER_RE.search(clause):
            continue
        if NERFSTUDIO_SPZ_CLAIM_RE.search(clause) and not SPZ_DISCLAIMER_RE.search(
            clause
        ):
            return Finding(
                "invented_nerfstudio_spz_export", path, context.line_no, line.rstrip()
            )
    return None


def ncp_manifest_pin(path: Path) -> tuple[str, list[Finding]]:
    """Read the shared NCP git tag and report malformed/inconsistent dependency pins."""

    data = tomllib.loads(path.read_text(encoding="utf-8"))
    dependencies = data.get("dependencies", {})
    pins: dict[str, str] = {}
    for name in ("ncp-core", "ncp-zenoh"):
        dependency = dependencies.get(name)
        if not isinstance(dependency, dict) or not isinstance(
            dependency.get("tag"), str
        ):
            finding = Finding(
                "ncp_manifest_pin_missing",
                path,
                1,
                f"{name} must be a git dependency with an explicit tag",
            )
            return "", [finding]
        pins[name] = dependency["tag"]

    unique_pins = set(pins.values())
    if len(unique_pins) != 1:
        return "", [
            Finding(
                "ncp_manifest_pin_inconsistent",
                path,
                1,
                ", ".join(f"{name}={tag}" for name, tag in pins.items()),
            )
        ]
    return next(iter(unique_pins)), []


def _ncp_pin_finding(
    path: Path, context: LineContext, expected_pin: str
) -> Finding | None:
    if context.historical or not expected_pin or not NCP_NAME_RE.search(context.line):
        return None

    claimed_tags: set[str] = set()
    tag_matches = list(SEMVER_TAG_RE.finditer(context.line))
    for ncp_match in NCP_NAME_RE.finditer(context.line):
        for tag_match in tag_matches:
            if tag_match.start() >= ncp_match.end():
                between = context.line[ncp_match.end() : tag_match.start()]
            elif tag_match.end() <= ncp_match.start():
                between = context.line[tag_match.end() : ncp_match.start()]
            else:
                between = ""
            if len(between) > 100:
                continue
            markup_stripped = re.sub(r"[`*_()<>{}\[\]]", "", between).strip()
            if len(markup_stripped) <= 12 or NCP_PIN_CUE_RE.search(between):
                claimed_tags.add(tag_match.group(0).casefold())

    if claimed_tags and expected_pin.casefold() not in claimed_tags:
        return Finding("ncp_pin_mismatch", path, context.line_no, context.line.rstrip())
    return None


def audit_ncp_pins(path: Path, expected_pin: str) -> list[Finding]:
    """Run only the manifest-backed NCP claim check on one Markdown document."""

    lines = path.read_text(encoding="utf-8").splitlines()
    findings: list[Finding] = []
    contexts = list(iter_line_contexts(path, lines))
    for context in logical_text_contexts(contexts):
        finding = _ncp_pin_finding(path, context, expected_pin)
        if finding:
            findings.append(finding)
    return findings


def audit_one(
    path: Path,
    *,
    section_catalog: dict[str, set[str]] | None = None,
    expected_ncp_pin: str = "",
) -> list[Finding]:
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    findings: list[Finding] = []

    # Preserve the original non-code claim checks unchanged.
    in_references = False
    for line_no, line in iter_non_code_lines(lines):
        if not line.strip():
            continue

        if REFERENCES_HEADING_RE.match(line):
            in_references = True

        has_arxiv = bool(ARXIV_ID_RE.search(line))

        # Venue claim check: if we mention a venue, require an explicit "verify" marker.
        # A `# References` bibliography legitimately lists venues per citation and is governed
        # by the reference policy (grandplan §17), so it is exempt from the per-line check.
        if VENUE_RE.search(line) and not in_references:
            # Headings and "field notes" style sections are not necessarily venue assertions.
            if line.lstrip().startswith("#"):
                continue
            if "observation" in line.lower() or "field notes" in line.lower():
                continue
            if not re.search(r"\bverify\b", line, re.IGNORECASE):
                findings.append(
                    Finding("venue_claim_needs_verify", path, line_no, line.rstrip())
                )

        # Numeric performance/cost claim check.
        has_perf_num = bool(
            PERCENT_RE.search(line) or MULT_RE.search(line) or UNITS_RE.search(line)
        )
        has_perf_words = bool(PERF_WORD_RE.search(line))
        if has_perf_num and has_perf_words:
            if not QUALIFIER_RE.search(line):
                # Allow some cases where the number is directly part of an arXiv abstract claim line.
                if not has_arxiv:
                    findings.append(
                        Finding(
                            "numeric_claim_unqualified", path, line_no, line.rstrip()
                        )
                    )

    if section_catalog is None:
        catalog_paths = [path]
        canonical_path = Path("grandplan.md")
        if path.name.casefold() != "grandplan.md" and canonical_path.exists():
            catalog_paths.append(canonical_path)
        catalog = build_section_catalog(catalog_paths)
    else:
        catalog = section_catalog
    contexts = list(iter_line_contexts(path, lines))
    findings.extend(_all_diagram_control_findings(path, contexts))
    for context in logical_text_contexts(contexts):
        prose_finding = _prose_control_finding(path, context)
        if prose_finding:
            findings.append(prose_finding)
        spz_finding = _spz_finding(path, context)
        if spz_finding:
            findings.append(spz_finding)
        findings.extend(_section_reference_findings(path, context, catalog))
        pin_finding = _ncp_pin_finding(path, context, expected_ncp_pin)
        if pin_finding:
            findings.append(pin_finding)

    return findings


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--paths",
        type=Path,
        nargs="*",
        default=DEFAULT_PATHS,
        help="Markdown files to audit (defaults to the canonical docset + findings).",
    )
    parser.add_argument(
        "--ncp-manifest",
        type=Path,
        default=DEFAULT_NCP_MANIFEST,
        help="Manifest whose NCP git tag is authoritative (default: crates/ncp-observer/Cargo.toml).",
    )
    args = parser.parse_args()

    missing = [path for path in args.paths if not path.exists()]
    if missing:
        raise SystemExit(f"Missing file(s): {', '.join(str(p) for p in missing)}")
    if not args.ncp_manifest.exists():
        raise SystemExit(f"Missing file: {args.ncp_manifest}")

    # Always index canonical grandplan when available, even for a targeted companion-doc audit,
    # because unqualified companion references commonly use it as their implicit target.
    catalog_paths = list(args.paths)
    canonical_path = Path("grandplan.md")
    if canonical_path.exists() and canonical_path not in catalog_paths:
        catalog_paths.append(canonical_path)
    section_catalog = build_section_catalog(catalog_paths)

    expected_ncp_pin, manifest_findings = ncp_manifest_pin(args.ncp_manifest)
    all_findings: list[Finding] = list(manifest_findings)
    for path in args.paths:
        all_findings.extend(
            audit_one(
                path,
                section_catalog=section_catalog,
                expected_ncp_pin=expected_ncp_pin,
            )
        )
    scanned_paths = {path.resolve() for path in args.paths}
    for path in SUPPLEMENTAL_NCP_DOCS:
        if path.exists() and path.resolve() not in scanned_paths:
            all_findings.extend(audit_ncp_pins(path, expected_ncp_pin))

    if not all_findings:
        print(
            "OK: no high-risk claim patterns or protected-invariant drift found in the scanned docset."
        )
        return 0

    print(f"Findings: {len(all_findings)}")
    for finding in all_findings:
        print(f"- {finding.kind}: {finding.path}:{finding.line_no}")
        print(f"  {finding.line}")

    print()
    print("Notes:")
    print("- This is heuristic; review the flagged lines in context.")
    print(
        "- Prefer explicit qualifiers, live section links, and Agent-Bridge-routed diagrams."
    )
    print(
        "- Nerfstudio Gaussian-splat export is PLY; SPZ requires a separate pinned converter."
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
