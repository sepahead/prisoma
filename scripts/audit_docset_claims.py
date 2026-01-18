#!/usr/bin/env python3
"""
Heuristic audit for the canonical docset to surface high-risk doc drift.

This extends the `audit_grandplan_claims.py` idea beyond `grandplan.md` by scanning:
- grandplan.md (canonical spec)
- README.md, ARCHITECTURE.md, EXPERIMENTS.md, DIAGRAMS.md, pidsplatspecs.md (canonical companion docs)
- findings.md (repo-local results summary; should still avoid unqualified claims)

It is intentionally conservative and *not* a proof of correctness. Use it to guide a
manual, first-principles review.
"""

from __future__ import annotations

import argparse
import re
from dataclasses import dataclass
from pathlib import Path


ARXIV_ID_RE = re.compile(r"arXiv:(\d{4}\.\d{5})")
VENUE_RE = re.compile(r"\b(NeurIPS|ICML|CoRL|ICLR|CVPR|ECCV|AAAI)\b")

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


@dataclass(frozen=True)
class Finding:
    kind: str
    path: Path
    line_no: int
    line: str


def iter_non_code_lines(lines: list[str]):
    in_fence = False
    for i, line in enumerate(lines, start=1):
        if line.lstrip().startswith("```"):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        yield i, line


def audit_one(path: Path) -> list[Finding]:
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    findings: list[Finding] = []

    for line_no, line in iter_non_code_lines(lines):
        if not line.strip():
            continue

        has_arxiv = bool(ARXIV_ID_RE.search(line))

        # Venue claim check: if we mention a venue, require an explicit "verify" marker.
        if VENUE_RE.search(line):
            # Headings and "field notes" style sections are not necessarily venue assertions.
            if line.lstrip().startswith("#"):
                continue
            if "observation" in line.lower() or "field notes" in line.lower():
                continue
            if not re.search(r"\bverify\b", line, re.IGNORECASE):
                findings.append(Finding("venue_claim_needs_verify", path, line_no, line.rstrip()))

        # Numeric performance/cost claim check.
        has_perf_num = bool(PERCENT_RE.search(line) or MULT_RE.search(line) or UNITS_RE.search(line))
        has_perf_words = bool(PERF_WORD_RE.search(line))
        if has_perf_num and has_perf_words:
            if not QUALIFIER_RE.search(line):
                # Allow some cases where the number is directly part of an arXiv abstract claim line.
                if not has_arxiv:
                    findings.append(Finding("numeric_claim_unqualified", path, line_no, line.rstrip()))

    return findings


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--paths",
        type=Path,
        nargs="*",
        default=[
            Path("grandplan.md"),
            Path("README.md"),
            Path("ARCHITECTURE.md"),
            Path("EXPERIMENTS.md"),
            Path("DIAGRAMS.md"),
            Path("pidsplatspecs.md"),
            Path("findings.md"),
        ],
        help="Markdown files to audit (defaults to the canonical docset + findings).",
    )
    args = parser.parse_args()

    all_findings: list[Finding] = []
    missing: list[Path] = []
    for path in args.paths:
        if not path.exists():
            missing.append(path)
            continue
        all_findings.extend(audit_one(path))

    if missing:
        raise SystemExit(f"Missing file(s): {', '.join(str(p) for p in missing)}")

    if not all_findings:
        print("OK: no high-risk claim patterns found in the scanned docset.")
        return 0

    print(f"Findings: {len(all_findings)}")
    for f in all_findings:
        print(f"- {f.kind}: {f.path}:{f.line_no}")
        print(f"  {f.line}")

    print()
    print("Notes:")
    print("- This is heuristic; review the flagged lines in context.")
    print("- Prefer adding explicit qualifiers (paper-reported/benchmark-dependent/verify) or removing the claim.")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
