#!/usr/bin/env python3
"""
Heuristic audit for `grandplan.md` to surface high-risk doc drift:

- Venue claims (NeurIPS/ICML/etc.) that are not explicitly marked "verify venue/status"
  (arXiv metadata does not include venues; these are easy to accidentally hallucinate).
- Performance/cost numbers (%, x, GB, hours, ms/fps) that are not clearly qualified
  as paper-reported / benchmark-dependent / illustrative / verify.

This is not a proof of correctness; it's a fast filter to guide a manual, first-principles pass.
"""

from __future__ import annotations

import argparse
import re
from dataclasses import dataclass
from pathlib import Path


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


@dataclass(frozen=True)
class Finding:
    kind: str
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


def audit(path: Path) -> list[Finding]:
    lines = path.read_text(encoding="utf-8").splitlines()
    findings: list[Finding] = []
    in_references = False

    for line_no, line in iter_non_code_lines(lines):
        if not line.strip():
            continue

        if REFERENCES_HEADING_RE.match(line):
            in_references = True

        has_arxiv = bool(ARXIV_ID_RE.search(line))

        # Venue claim check: if we mention a venue, require an explicit "verify venue/status".
        # The `# References` bibliography legitimately lists venues per citation; the reference
        # policy (grandplan §17) governs rechecking them, so it is exempt from the line check.
        if VENUE_RE.search(line) and not in_references:
            # Headings and conference-notes sections are not paper venue assertions.
            if line.lstrip().startswith("#"):
                continue
            if "observation" in line.lower() or "field notes" in line.lower():
                continue
            if ("§12.4" in line) and (not has_arxiv):
                continue
            if not re.search(r"\bverify\b", line, re.IGNORECASE):
                findings.append(Finding("venue_claim_needs_verify", line_no, line.rstrip()))

        # Numeric performance/cost claim check.
        has_perf_num = bool(PERCENT_RE.search(line) or MULT_RE.search(line) or UNITS_RE.search(line))
        has_perf_words = bool(PERF_WORD_RE.search(line))
        if has_perf_num and has_perf_words:
            if not QUALIFIER_RE.search(line):
                # Allow some cases where the number is directly part of an arXiv abstract claim line.
                # (But still prefer qualifiers; this is just to reduce noise.)
                if not has_arxiv:
                    findings.append(Finding("numeric_claim_unqualified", line_no, line.rstrip()))

    return findings


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--grandplan", type=Path, default=Path("grandplan.md"))
    args = parser.parse_args()

    if not args.grandplan.exists():
        raise SystemExit(f"Missing file: {args.grandplan}")

    findings = audit(args.grandplan)

    if not findings:
        print(f"OK: no high-risk claim patterns found in {args.grandplan}.")
        return 0

    print(f"Findings: {len(findings)}")
    for f in findings:
        print(f"- {f.kind}: {args.grandplan}:{f.line_no}")
        print(f"  {f.line}")

    print()
    print("Notes:")
    print("- This is heuristic; review the flagged lines in context.")
    print("- Prefer adding explicit qualifiers (paper-reported/benchmark-dependent/verify) or removing the claim.")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
