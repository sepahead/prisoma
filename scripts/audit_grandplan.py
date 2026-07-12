#!/usr/bin/env python3
"""
Offline integrity audit for `grandplan.md` references (docset v12.5 reference ledger).

Docset v12.5 uses a self-contained `[R1]`..`[R112]` reference ledger (defined under the
`# References` section as `- **[R##]** ...`) instead of the older `arXiv:` cache-coverage
scheme. This audit validates that ledger the way the second-round review validated it:

  - reference definitions are contiguous `R1..RN` with no gaps or duplicates;
  - every cited reference ID is defined;
  - every defined reference ID is cited at least once in the body;
  - citation groups (`[R25]`, `[R61, R73]`, ranges like `[R89-R91]`/`[R01-R10]`) parse cleanly;
  - every reference definition carries at least one URL, and URLs are reported for review.

Exit codes:
  0: the reference ledger is internally consistent
  1: at least one integrity problem was found

The `--check-italic-titles` and `--cache` flags are accepted for backward compatibility with
existing `just docs-audit` / CLAUDE.md invocations; the arXiv cache is no longer required.
"""

from __future__ import annotations

import argparse
import re
from dataclasses import dataclass
from pathlib import Path


# `- **[R01]** ...` reference-definition marker at the start of a list item.
DEF_RE = re.compile(r"^\s*[-*]\s*\*\*\[R0*(\d{1,4})\]\*\*")
# A bracketed citation group beginning with an R-token, e.g. `[R18]`, `[R61, R73]`,
# `[R89-R91]`, `[R101-R105, R109-R112]`. Hyphen or en-dash ranges are both allowed.
BRACKET_RE = re.compile(
    r"\[(R0*\d{1,4}(?:\s*[–-]\s*R0*\d{1,4})?"
    r"(?:\s*,\s*R0*\d{1,4}(?:\s*[–-]\s*R0*\d{1,4})?)*)\]"
)
TOKEN_RE = re.compile(r"R0*(\d{1,4})\s*[–-]\s*R0*(\d{1,4})|R0*(\d{1,4})")
URL_RE = re.compile(r"https?://[^\s)>\]]+")
REFERENCES_HEADING_RE = re.compile(r"^#{1,6}\s+References\b", re.IGNORECASE)


@dataclass
class Ledger:
    definitions: list[int]          # ids in file order (to detect duplicates)
    cited: set[int]                 # ids cited outside their own definition marker
    def_urls: dict[int, list[str]]  # per-reference URLs
    urls: list[str]                 # all URLs in the document


def expand_group(content: str) -> list[int]:
    ids: list[int] = []
    for part in content.split(","):
        m = TOKEN_RE.search(part)
        if not m:
            continue
        if m.group(1) is not None and m.group(2) is not None:
            lo, hi = int(m.group(1)), int(m.group(2))
            if lo <= hi:
                ids.extend(range(lo, hi + 1))
        elif m.group(3) is not None:
            ids.append(int(m.group(3)))
    return ids


def parse_ledger(text: str) -> Ledger:
    definitions: list[int] = []
    cited: set[int] = set()
    def_urls: dict[int, list[str]] = {}
    urls: list[str] = []

    for line in text.splitlines():
        urls.extend(URL_RE.findall(line))
        def_match = DEF_RE.match(line)
        scan_line = line
        if def_match:
            ref_id = int(def_match.group(1))
            definitions.append(ref_id)
            def_urls.setdefault(ref_id, []).extend(URL_RE.findall(line))
            # Do not count a reference's own `**[R##]**` marker as a citation of itself.
            scan_line = line[def_match.end():]
        for grp in BRACKET_RE.finditer(scan_line):
            cited.update(expand_group(grp.group(1)))

    return Ledger(definitions=definitions, cited=cited, def_urls=def_urls, urls=urls)


def audit(text: str) -> tuple[list[str], Ledger]:
    ledger = parse_ledger(text)
    problems: list[str] = []

    defined = sorted(set(ledger.definitions))
    n = len(defined)

    # Duplicate definitions.
    seen: set[int] = set()
    dups = sorted({d for d in ledger.definitions if d in seen or seen.add(d)})
    if dups:
        problems.append(f"duplicate reference definitions: {['R%d' % d for d in dups]}")

    # Contiguity R1..RN.
    if not defined:
        problems.append("no reference definitions found (expected `- **[R##]** ...`)")
    else:
        expected = list(range(1, n + 1))
        if defined != expected:
            missing = sorted(set(expected) - set(defined))
            extra = sorted(set(defined) - set(expected))
            problems.append(
                "reference IDs are not contiguous R1..R%d: missing=%s extra=%s"
                % (n, ["R%d" % d for d in missing], ["R%d" % d for d in extra])
            )

    defined_set = set(defined)
    undefined = sorted(ledger.cited - defined_set)
    if undefined:
        problems.append(
            "citations without a definition: %s" % ["R%d" % d for d in undefined]
        )

    unused = sorted(defined_set - ledger.cited)
    if unused:
        problems.append(
            "reference definitions never cited in body: %s"
            % ["R%d" % d for d in unused]
        )

    missing_url = sorted(d for d in defined if not ledger.def_urls.get(d))
    if missing_url:
        problems.append(
            "reference definitions without a URL: %s" % ["R%d" % d for d in missing_url]
        )

    return problems, ledger


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--grandplan", type=Path, default=Path("grandplan.md"))
    # Accepted for backward compatibility; no longer load-bearing.
    parser.add_argument("--cache", type=Path, default=Path("outputs/arxiv_ref_cache.json"))
    parser.add_argument("--check-italic-titles", action="store_true")
    args = parser.parse_args()

    if not args.grandplan.exists():
        raise SystemExit(f"Missing file: {args.grandplan}")

    text = args.grandplan.read_text(encoding="utf-8")
    problems, ledger = audit(text)

    defined = sorted(set(ledger.definitions))
    print(f"grandplan:            {args.grandplan}")
    print(f"reference definitions: {len(defined)}"
          + (f" (R1..R{len(defined)})" if defined else ""))
    print(f"unique cited IDs:      {len(ledger.cited)}")
    print(f"URLs:                  {len(ledger.urls)} ({len(set(ledger.urls))} unique)")

    if problems:
        print()
        print("Problems:")
        for p in problems:
            print(f"- {p}")
        print()
        print("Suggested next step: fix the reference ledger so IDs are contiguous and")
        print("every reference is both defined and cited, then re-run this audit.")
        return 1

    print()
    print("OK: reference ledger is internally consistent.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
