#!/usr/bin/env python3
"""
Offline audit for `grandplan.md` paper references.

Goals:
  - Enumerate arXiv IDs referenced in `grandplan.md`
  - Check coverage against `outputs/arxiv_ref_cache.json`
  - Print a short, actionable report (no network required)

Exit codes:
  0: all referenced arXiv IDs are present in the cache
  1: at least one referenced arXiv ID is missing from the cache
"""

from __future__ import annotations

import argparse
import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


ARXIV_ID_RE = re.compile(r"arXiv:(\d{4}\.\d{5})")
ARXIV_ITALIC_TITLE_RE = re.compile(r"\*([^*]+)\*\s*arXiv:(\d{4}\.\d{5})")


@dataclass(frozen=True)
class MissingRef:
    arxiv_id: str
    line_no: int
    line: str


@dataclass(frozen=True)
class TitleMismatch:
    arxiv_id: str
    line_no: int
    doc_title: str
    cache_title: str
    line: str


def extract_arxiv_mentions(markdown: str) -> list[tuple[str, int, str]]:
    mentions: list[tuple[str, int, str]] = []
    for idx, line in enumerate(markdown.splitlines(), start=1):
        for match in ARXIV_ID_RE.finditer(line):
            mentions.append((match.group(1), idx, line.rstrip()))
    return mentions


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def unique_preserve_order(items: Iterable[str]) -> list[str]:
    seen: set[str] = set()
    out: list[str] = []
    for item in items:
        if item in seen:
            continue
        seen.add(item)
        out.append(item)
    return out


def normalize_title(title: str) -> str:
    title = title.lower().strip()
    title = re.sub(r"\s+", " ", title)
    # keep alnum + spaces for tolerant matching
    title = re.sub(r"[^a-z0-9 ]", "", title)
    return title


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--grandplan",
        type=Path,
        default=Path("grandplan.md"),
        help="Path to grandplan markdown (default: grandplan.md).",
    )
    parser.add_argument(
        "--cache",
        type=Path,
        default=Path("outputs/arxiv_ref_cache.json"),
        help="Path to arXiv cache JSON (default: outputs/arxiv_ref_cache.json).",
    )
    parser.add_argument(
        "--check-italic-titles",
        action="store_true",
        help="Check `*Title* arXiv:XXXX.XXXXX` lines against cached arXiv titles.",
    )
    args = parser.parse_args()

    if not args.grandplan.exists():
        raise SystemExit(f"Missing file: {args.grandplan}")
    if not args.cache.exists():
        raise SystemExit(f"Missing file: {args.cache}")

    grandplan_text = args.grandplan.read_text(encoding="utf-8")
    cache = load_json(args.cache)

    mentions = extract_arxiv_mentions(grandplan_text)
    ids_in_doc = unique_preserve_order([arxiv_id for (arxiv_id, _, _) in mentions])

    missing: list[MissingRef] = []
    for arxiv_id, line_no, line in mentions:
        if arxiv_id not in cache:
            missing.append(MissingRef(arxiv_id=arxiv_id, line_no=line_no, line=line))

    mismatches: list[TitleMismatch] = []
    if args.check_italic_titles:
        for idx, line in enumerate(grandplan_text.splitlines(), start=1):
            m = ARXIV_ITALIC_TITLE_RE.search(line)
            if not m:
                continue
            doc_title = m.group(1).strip().rstrip(".")
            arxiv_id = m.group(2)
            entry = cache.get(arxiv_id)
            if entry is None:
                continue
            cache_title = (entry.get("title") or "").strip()
            if not cache_title:
                continue
            if normalize_title(doc_title) != normalize_title(cache_title):
                mismatches.append(
                    TitleMismatch(
                        arxiv_id=arxiv_id,
                        line_no=idx,
                        doc_title=doc_title,
                        cache_title=cache_title,
                        line=line.rstrip(),
                    )
                )

    print(f"grandplan: {args.grandplan}")
    print(f"cache:     {args.cache}")
    print()
    print(f"arXiv IDs referenced: {len(ids_in_doc)}")
    print(f"cache entries:        {len(cache)}")
    print(f"missing from cache:   {len(set(m.arxiv_id for m in missing))}")
    if args.check_italic_titles:
        print(f"italic-title mismatches: {len(mismatches)}")

    if missing or mismatches:
        print()
        if missing:
            print("Missing arXiv IDs (first occurrence):")
            seen: set[str] = set()
            for m in missing:
                if m.arxiv_id in seen:
                    continue
                seen.add(m.arxiv_id)
                print(f"- arXiv:{m.arxiv_id} at {args.grandplan}:{m.line_no}")
                print(f"  {m.line}")

        if mismatches:
            print()
            print("Italic title mismatches (first occurrence):")
            for mm in mismatches:
                print(f"- arXiv:{mm.arxiv_id} at {args.grandplan}:{mm.line_no}")
                print(f"  doc:   {mm.doc_title}")
                print(f"  cache: {mm.cache_title}")
                print(f"  {mm.line}")

        print()
        print("Suggested next step:")
        if missing:
            print("  - Update `outputs/arxiv_ref_cache.json` to include the missing IDs (requires network).")
        if mismatches:
            print("  - Align the italicized titles to the cached arXiv titles (or remove the italics).")
        print("  - Then re-run this audit to confirm coverage.")

        return 1

    print()
    print("OK: all referenced arXiv IDs are present in the cache.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
