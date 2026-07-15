#!/usr/bin/env python3
"""
Update `outputs/arxiv_ref_cache.json` by fetching missing arXiv metadata.

This script *requires network access* (arXiv API).

Usage:
  - Update cache for all arXiv IDs referenced in grandplan.md:
      python scripts/update_arxiv_ref_cache.py

  - Update cache for explicit arXiv IDs:
      python scripts/update_arxiv_ref_cache.py --ids 2503.20314 2412.10345
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import time
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET
from dataclasses import dataclass
from pathlib import Path


ARXIV_ID_RE = re.compile(r"arXiv:(\d{4}\.\d{5})")
ATOM_NS = {"atom": "http://www.w3.org/2005/Atom"}


@dataclass(frozen=True)
class ArxivEntry:
    arxiv_id: str
    title: str
    summary: str
    authors: list[str]
    published: str
    updated: str


def extract_arxiv_ids_from_markdown(text: str) -> list[str]:
    ids: list[str] = []
    seen: set[str] = set()
    for line in text.splitlines():
        for match in ARXIV_ID_RE.finditer(line):
            arxiv_id = match.group(1)
            if arxiv_id in seen:
                continue
            seen.add(arxiv_id)
            ids.append(arxiv_id)
    return ids


def load_cache(path: Path) -> dict:
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def save_cache(path: Path, cache: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(cache, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


def fetch_arxiv_atom(ids: list[str]) -> str:
    # arXiv API guidance suggests batching; keep modest to avoid 414s.
    id_list = ",".join(ids)
    query = urllib.parse.urlencode({"id_list": id_list})
    url = f"http://export.arxiv.org/api/query?{query}"
    with urllib.request.urlopen(url, timeout=30) as resp:
        return resp.read().decode("utf-8", errors="replace")


def parse_atom(xml_text: str) -> list[ArxivEntry]:
    root = ET.fromstring(xml_text)
    entries: list[ArxivEntry] = []
    for e in root.findall("atom:entry", ATOM_NS):
        arxiv_id_full = (
            e.findtext("atom:id", default="", namespaces=ATOM_NS) or ""
        ).strip()
        # Example: http://arxiv.org/abs/2507.04447v1
        arxiv_id = arxiv_id_full.rsplit("/", 1)[-1].split("v", 1)[0]

        title = (e.findtext("atom:title", default="", namespaces=ATOM_NS) or "").strip()
        summary = (
            e.findtext("atom:summary", default="", namespaces=ATOM_NS) or ""
        ).strip()
        published = (
            e.findtext("atom:published", default="", namespaces=ATOM_NS) or ""
        ).strip()
        updated = (
            e.findtext("atom:updated", default="", namespaces=ATOM_NS) or ""
        ).strip()
        authors = [
            (a.findtext("atom:name", default="", namespaces=ATOM_NS) or "").strip()
            for a in e.findall("atom:author", ATOM_NS)
        ]
        authors = [a for a in authors if a]

        if not arxiv_id or not title:
            continue
        entries.append(
            ArxivEntry(
                arxiv_id=arxiv_id,
                title=" ".join(title.split()),
                summary=" ".join(summary.split()),
                authors=authors,
                published=published,
                updated=updated,
            )
        )
    return entries


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
        "--ids",
        nargs="*",
        default=None,
        help="Explicit arXiv IDs to fetch (default: extract from grandplan.md).",
    )
    parser.add_argument(
        "--batch-size",
        type=int,
        default=25,
        help="Max arXiv IDs per API call (default: 25).",
    )
    parser.add_argument(
        "--sleep",
        type=float,
        default=3.0,
        help="Seconds to sleep between API calls (default: 3.0).",
    )
    args = parser.parse_args()

    cache = load_cache(args.cache)

    if args.ids is not None:
        requested = args.ids
    else:
        if not args.grandplan.exists():
            raise SystemExit(f"Missing file: {args.grandplan}")
        requested = extract_arxiv_ids_from_markdown(
            args.grandplan.read_text(encoding="utf-8")
        )

    requested = [x.strip() for x in requested if x and x.strip()]
    requested = sorted(set(requested))

    missing = [x for x in requested if x not in cache]
    if not missing:
        print("No missing arXiv IDs; cache is up to date.")
        return 0

    print(f"Cache path: {args.cache}")
    print(f"Requested IDs: {len(requested)}")
    print(f"Missing IDs:   {len(missing)}")

    batch_size = max(1, args.batch_size)
    fetched = 0
    for start in range(0, len(missing), batch_size):
        batch = missing[start : start + batch_size]
        print(
            f"Fetching batch {start // batch_size + 1}: {', '.join(batch)}",
            file=sys.stderr,
        )
        xml_text = fetch_arxiv_atom(batch)
        entries = parse_atom(xml_text)
        by_id = {e.arxiv_id: e for e in entries}
        for arxiv_id in batch:
            entry = by_id.get(arxiv_id)
            if entry is None:
                print(
                    f"WARNING: arXiv:{arxiv_id} not found in API response",
                    file=sys.stderr,
                )
                continue
            cache[arxiv_id] = {
                "authors": entry.authors,
                "id": entry.arxiv_id,
                "published": entry.published,
                "summary": entry.summary,
                "title": entry.title,
                "updated": entry.updated,
            }
            fetched += 1
        time.sleep(max(0.0, args.sleep))

    save_cache(args.cache, cache)
    print(f"Updated cache with {fetched} entries.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
