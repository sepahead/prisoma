#!/usr/bin/env python3
"""
Update `outputs/arxiv_ref_cache.json` by fetching missing arXiv metadata.

This script *requires network access* (arXiv API).

The updater is a manual cache-maintenance utility. It uses a fixed HTTPS API origin,
bounded responses, strict local schemas, and atomic cache replacement; those checks do
not authenticate publication metadata or replace source review.

Usage:
  - Update cache for all arXiv IDs referenced in grandplan.md:
      python scripts/update_arxiv_ref_cache.py

  - Update cache for explicit arXiv IDs:
      python scripts/update_arxiv_ref_cache.py --ids 2503.20314 2412.10345
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import math
import os
import re
import stat
import sys
import tempfile
import time
import urllib.parse
import urllib.request

# The parser is byte/structure bounded and rejects DTD/entity declarations below.
import xml.etree.ElementTree as ET  # nosec B405
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ARXIV_ID_PATTERN = r"\d{2}(?:0[1-9]|1[0-2])\.\d{4,5}"
ARXIV_ID_RE = re.compile(
    rf"(?<![A-Za-z0-9])arXiv:({ARXIV_ID_PATTERN})(?:v[1-9]\d*)?"
    rf"(?![A-Za-z0-9]|\.\d)"
)
ARXIV_EXPLICIT_ID_RE = re.compile(rf"{ARXIV_ID_PATTERN}\Z")
ARXIV_ATOM_ID_RE = re.compile(rf"({ARXIV_ID_PATTERN})(?:v[1-9]\d*)?\Z")
RFC3339_UTC_RE = re.compile(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?Z\Z")
ATOM_NS = {"atom": "http://www.w3.org/2005/Atom"}
ARXIV_API_HOST = "export.arxiv.org"
ARXIV_API_URL = f"https://{ARXIV_API_HOST}/api/query"

MAX_MARKDOWN_BYTES = 8 * 1024 * 1024
MAX_CACHE_BYTES = 16 * 1024 * 1024
MAX_ATOM_BYTES = 8 * 1024 * 1024
MAX_IDS = 2_000
MAX_CACHE_ENTRIES = 10_000
MAX_BATCH_SIZE = 100
MAX_SLEEP_SECONDS = 60.0
HTTP_TIMEOUT_SECONDS = 30.0
MAX_TITLE_CHARS = 4_096
MAX_SUMMARY_CHARS = 131_072
MAX_AUTHORS = 512
MAX_AUTHOR_CHARS = 1_024
MAX_TIMESTAMP_CHARS = 64
MAX_ATOM_DEPTH = 32
MAX_ATOM_ELEMENTS = 1 + MAX_BATCH_SIZE * (2 * MAX_AUTHORS + 64)
EXPECTED_CACHE_FIELDS = {"authors", "id", "published", "summary", "title", "updated"}


@dataclass(frozen=True)
class ArxivEntry:
    arxiv_id: str
    title: str
    summary: str
    authors: list[str]
    published: str
    updated: str


class _BoundedTreeBuilder(ET.TreeBuilder):
    """Stop excessive XML structure while the bounded Atom tree is being built."""

    def __init__(self, *, max_depth: int, max_elements: int) -> None:
        super().__init__()
        self._max_depth = max_depth
        self._max_elements = max_elements
        self._depth = 0
        self._elements = 0

    def start(self, tag: str, attrs: dict[str, str]) -> Any:
        next_depth = self._depth + 1
        next_elements = self._elements + 1
        if next_depth > self._max_depth:
            raise ValueError(
                f"Atom response exceeds the {self._max_depth}-level depth limit"
            )
        if next_elements > self._max_elements:
            raise ValueError(
                f"Atom response exceeds the {self._max_elements}-element limit"
            )
        self._depth = next_depth
        self._elements = next_elements
        return super().start(tag, attrs)

    def end(self, tag: str) -> Any:
        element = super().end(tag)
        self._depth -= 1
        return element


def validate_arxiv_id(value: str) -> str:
    if not isinstance(value, str) or ARXIV_EXPLICIT_ID_RE.fullmatch(value) is None:
        raise ValueError(
            f"Invalid arXiv id {value!r}; expected YYMM.NNNN or YYMM.NNNNN"
        )
    year_month, serial = value.split(".", 1)
    if int(year_month) < 704:
        raise ValueError(f"Invalid arXiv id {value!r}; modern ids begin at 0704")
    if (int(year_month) < 1501 and len(serial) != 4) or (
        int(year_month) >= 1501 and len(serial) != 5
    ):
        raise ValueError(
            f"Invalid arXiv id {value!r}; serial width does not match its era"
        )
    if int(serial) == 0:
        raise ValueError(f"Invalid arXiv id {value!r}; serial must be positive")
    return value


def _read_regular_file(path: Path, *, max_bytes: int, label: str) -> bytes:
    try:
        metadata = path.lstat()
    except FileNotFoundError as exc:
        raise ValueError(f"{label} does not exist: {path}") from exc
    if not stat.S_ISREG(metadata.st_mode):
        raise ValueError(f"{label} must be a regular, non-symlink file: {path}")
    if metadata.st_size > max_bytes:
        raise ValueError(f"{label} exceeds the {max_bytes}-byte limit: {path}")

    fd = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
    try:
        opened = os.fstat(fd)
        if not stat.S_ISREG(opened.st_mode):
            raise ValueError(f"{label} must be a regular file: {path}")
        if (opened.st_dev, opened.st_ino) != (metadata.st_dev, metadata.st_ino):
            raise ValueError(f"{label} changed while it was being opened: {path}")
        if opened.st_size > max_bytes:
            raise ValueError(f"{label} exceeds the {max_bytes}-byte limit: {path}")
        chunks: list[bytes] = []
        remaining = max_bytes + 1
        while remaining:
            chunk = os.read(fd, min(65_536, remaining))
            if not chunk:
                break
            chunks.append(chunk)
            remaining -= len(chunk)
        data = b"".join(chunks)
    finally:
        os.close(fd)
    if len(data) > max_bytes:
        raise ValueError(f"{label} exceeds the {max_bytes}-byte limit: {path}")
    return data


def _atomic_write(path: Path, data: bytes) -> None:
    if len(data) > MAX_CACHE_BYTES:
        raise ValueError(f"Cache output exceeds the {MAX_CACHE_BYTES}-byte limit")
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.parent.is_symlink() or not path.parent.is_dir():
        raise ValueError(f"Cache parent must be a regular directory: {path.parent}")
    if path.exists() or path.is_symlink():
        if not stat.S_ISREG(path.lstat().st_mode):
            raise ValueError(
                f"Cache target must be a regular, non-symlink file: {path}"
            )

    fd, tmp_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    tmp_path = Path(tmp_name)
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(tmp_path, path)
    except BaseException:
        tmp_path.unlink(missing_ok=True)
        raise


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"Duplicate JSON key: {key!r}")
        result[key] = value
    return result


def _reject_json_constant(value: str) -> Any:
    raise ValueError(f"Non-standard JSON numeric constant: {value}")


def _checked_text(
    value: Any, *, label: str, max_chars: int, allow_empty: bool = False
) -> str:
    if not isinstance(value, str):
        raise ValueError(f"{label} must be a string")
    if len(value) > max_chars:
        raise ValueError(f"{label} exceeds the {max_chars}-character limit")
    if "\x00" in value:
        raise ValueError(f"{label} contains a NUL character")
    if any(ord(char) < 0x20 and char not in "\t\n\r" for char in value):
        raise ValueError(f"{label} contains a disallowed control character")
    if not allow_empty and not value.strip():
        raise ValueError(f"{label} must not be empty")
    return value


def _validate_timestamp(value: Any, *, label: str) -> str:
    text = _checked_text(value, label=label, max_chars=MAX_TIMESTAMP_CHARS)
    if RFC3339_UTC_RE.fullmatch(text) is None:
        raise ValueError(f"{label} must be an RFC 3339 UTC timestamp")
    try:
        parsed = dt.datetime.fromisoformat(text[:-1] + "+00:00")
    except ValueError as exc:
        raise ValueError(f"{label} must be an RFC 3339 UTC timestamp") from exc
    if parsed.tzinfo != dt.timezone.utc:
        raise ValueError(f"{label} must use UTC")
    return text


def _timestamp_value(value: str) -> dt.datetime:
    return dt.datetime.fromisoformat(value[:-1] + "+00:00")


def _validate_cache_entry(key: str, value: Any) -> dict[str, Any]:
    validate_arxiv_id(key)
    if not isinstance(value, dict) or set(value) != EXPECTED_CACHE_FIELDS:
        raise ValueError(f"Cache entry {key} has an invalid schema")
    entry_id = validate_arxiv_id(value["id"])
    if entry_id != key:
        raise ValueError(f"Cache entry {key} has mismatched id {entry_id}")
    authors_obj = value["authors"]
    if not isinstance(authors_obj, list) or not 1 <= len(authors_obj) <= MAX_AUTHORS:
        raise ValueError(f"Cache entry {key} authors must be a bounded nonempty list")
    authors = [
        _checked_text(
            author,
            label=f"Cache entry {key} author {index}",
            max_chars=MAX_AUTHOR_CHARS,
        )
        for index, author in enumerate(authors_obj, start=1)
    ]
    published = _validate_timestamp(
        value["published"], label=f"Cache entry {key} published"
    )
    updated = _validate_timestamp(value["updated"], label=f"Cache entry {key} updated")
    if _timestamp_value(updated) < _timestamp_value(published):
        raise ValueError(f"Cache entry {key} updated precedes published")
    return {
        "authors": authors,
        "id": entry_id,
        "published": published,
        "summary": _checked_text(
            value["summary"],
            label=f"Cache entry {key} summary",
            max_chars=MAX_SUMMARY_CHARS,
            allow_empty=True,
        ),
        "title": _checked_text(
            value["title"], label=f"Cache entry {key} title", max_chars=MAX_TITLE_CHARS
        ),
        "updated": updated,
    }


def extract_arxiv_ids_from_markdown(text: str) -> list[str]:
    ids: list[str] = []
    seen: set[str] = set()
    for line in text.splitlines():
        for match in ARXIV_ID_RE.finditer(line):
            arxiv_id = validate_arxiv_id(match.group(1))
            if arxiv_id in seen:
                continue
            seen.add(arxiv_id)
            ids.append(arxiv_id)
    return ids


def load_cache(path: Path) -> dict[str, dict[str, Any]]:
    if path.is_symlink():
        raise ValueError(f"Cache must not be a symlink: {path}")
    if not path.exists():
        return {}
    data = _read_regular_file(path, max_bytes=MAX_CACHE_BYTES, label="arXiv cache")
    try:
        obj = json.loads(
            data,
            object_pairs_hook=_reject_duplicate_keys,
            parse_constant=_reject_json_constant,
        )
    except (UnicodeDecodeError, json.JSONDecodeError, RecursionError) as exc:
        raise ValueError("arXiv cache is not valid UTF-8 JSON") from exc
    if not isinstance(obj, dict):
        raise ValueError("arXiv cache must be a JSON object")
    if len(obj) > MAX_CACHE_ENTRIES:
        raise ValueError(f"arXiv cache exceeds the {MAX_CACHE_ENTRIES}-entry limit")
    return {key: _validate_cache_entry(key, value) for key, value in obj.items()}


def save_cache(path: Path, cache: dict[str, dict[str, Any]]) -> None:
    if len(cache) > MAX_CACHE_ENTRIES:
        raise ValueError(f"arXiv cache exceeds the {MAX_CACHE_ENTRIES}-entry limit")
    validated = {key: _validate_cache_entry(key, value) for key, value in cache.items()}
    data = (
        json.dumps(validated, indent=2, sort_keys=True, allow_nan=False) + "\n"
    ).encode("utf-8")
    _atomic_write(path, data)


def _validate_arxiv_api_url(url: str) -> None:
    if len(url) > 8_192 or any(char.isspace() or ord(char) < 0x20 for char in url):
        raise ValueError("arXiv API URL is invalid or too long")
    parsed = urllib.parse.urlsplit(url)
    if parsed.scheme.lower() != "https":
        raise ValueError("arXiv API requests and redirects must use HTTPS")
    if parsed.username is not None or parsed.password is not None:
        raise ValueError("arXiv API URL credentials are not permitted")
    if (parsed.hostname or "").lower().rstrip(".") != ARXIV_API_HOST:
        raise ValueError("arXiv API redirects must remain on export.arxiv.org")
    try:
        port = parsed.port
    except ValueError as exc:
        raise ValueError("arXiv API URL port is invalid") from exc
    if port not in (None, 443):
        raise ValueError("arXiv API URL must use the default HTTPS port")
    if parsed.path != "/api/query" or parsed.fragment:
        raise ValueError("arXiv API URL path or fragment is invalid")
    try:
        query = urllib.parse.parse_qs(
            parsed.query,
            keep_blank_values=True,
            strict_parsing=True,
            max_num_fields=2,
        )
    except ValueError as exc:
        raise ValueError("arXiv API URL query is invalid") from exc
    if (
        set(query) != {"id_list", "max_results"}
        or len(query["id_list"]) != 1
        or len(query["max_results"]) != 1
    ):
        raise ValueError(
            "arXiv API URL must contain exactly one id_list and max_results parameter"
        )
    ids = query["id_list"][0].split(",")
    if not ids or len(ids) > MAX_BATCH_SIZE or any(not value for value in ids):
        raise ValueError("arXiv API URL id_list is empty or exceeds the batch limit")
    for arxiv_id in ids:
        validate_arxiv_id(arxiv_id)
    raw_max_results = query["max_results"][0]
    if re.fullmatch(r"[1-9]\d{0,2}", raw_max_results) is None:
        raise ValueError("arXiv API URL max_results is invalid")
    max_results = int(raw_max_results)
    if max_results > MAX_BATCH_SIZE or max_results != len(ids):
        raise ValueError(
            "arXiv API URL max_results must equal the requested id_list length"
        )


class _ArxivRedirectHandler(urllib.request.HTTPRedirectHandler):
    def redirect_request(
        self,
        req: urllib.request.Request,
        fp: Any,
        code: int,
        msg: str,
        headers: Any,
        newurl: str,
    ) -> urllib.request.Request | None:
        absolute_url = urllib.parse.urljoin(req.full_url, newurl)
        _validate_arxiv_api_url(absolute_url)
        return super().redirect_request(req, fp, code, msg, headers, absolute_url)


def _read_response_bytes(response: Any, *, max_bytes: int) -> bytes:
    raw_length = response.headers.get("Content-Length")
    if raw_length is not None:
        try:
            declared_length = int(raw_length)
        except ValueError as exc:
            raise ValueError("arXiv response has an invalid Content-Length") from exc
        if declared_length < 0 or declared_length > max_bytes:
            raise ValueError(f"arXiv response exceeds the {max_bytes}-byte limit")
    data = response.read(max_bytes + 1)
    if len(data) > max_bytes:
        raise ValueError(f"arXiv response exceeds the {max_bytes}-byte limit")
    return data


def fetch_arxiv_atom(ids: list[str]) -> str:
    # arXiv API guidance suggests batching; keep modest to avoid 414s.
    if not ids or len(ids) > MAX_BATCH_SIZE:
        raise ValueError(f"arXiv API batch must contain 1-{MAX_BATCH_SIZE} ids")
    if len(set(ids)) != len(ids):
        raise ValueError("arXiv API batch contains duplicate ids")
    for arxiv_id in ids:
        validate_arxiv_id(arxiv_id)
    id_list = ",".join(ids)
    query = urllib.parse.urlencode({"id_list": id_list, "max_results": len(ids)})
    url = f"{ARXIV_API_URL}?{query}"
    _validate_arxiv_api_url(url)
    request = urllib.request.Request(
        url,
        headers={
            "Accept": "application/atom+xml, application/xml;q=0.9",
            "Accept-Encoding": "identity",
            "User-Agent": "prisoma-arxiv-cache/0.9 (+https://github.com/sepahead/prisoma)",
        },
        method="GET",
    )
    opener = urllib.request.build_opener(_ArxivRedirectHandler())
    with opener.open(request, timeout=HTTP_TIMEOUT_SECONDS) as response:
        _validate_arxiv_api_url(response.geturl())
        content_type = (response.headers.get("Content-Type") or "").split(";", 1)[0]
        if content_type.lower() not in {
            "application/atom+xml",
            "application/xml",
            "text/xml",
        }:
            raise ValueError(
                f"arXiv API returned an unexpected content type: {content_type or 'missing'}"
            )
        data = _read_response_bytes(response, max_bytes=MAX_ATOM_BYTES)
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise ValueError("arXiv API response is not valid UTF-8") from exc


def parse_atom(xml_text: str) -> list[ArxivEntry]:
    if not isinstance(xml_text, str):
        raise ValueError("Atom response must be text")
    try:
        xml_bytes = xml_text.encode("utf-8")
    except UnicodeEncodeError as exc:
        raise ValueError("Atom response contains invalid Unicode") from exc
    if len(xml_bytes) > MAX_ATOM_BYTES:
        raise ValueError(f"Atom response exceeds the {MAX_ATOM_BYTES}-byte limit")
    lowered = xml_text.lower()
    if "<!doctype" in lowered or "<!entity" in lowered:
        raise ValueError("Atom response must not contain DTD or entity declarations")
    try:
        parser = ET.XMLParser(  # nosec B314
            target=_BoundedTreeBuilder(
                max_depth=MAX_ATOM_DEPTH,
                max_elements=MAX_ATOM_ELEMENTS,
            )
        )
        # Input is UTF-8/byte-bounded, declarations are rejected, and the target
        # aborts excessive depth/elements before finishing the tree.
        root = ET.fromstring(xml_text, parser=parser)  # nosec B314
    except ET.ParseError as exc:
        raise ValueError("arXiv API returned malformed Atom XML") from exc
    if root.tag != f"{{{ATOM_NS['atom']}}}feed":
        raise ValueError("arXiv API response root must be an Atom feed")
    atom_entries = root.findall("atom:entry", ATOM_NS)
    if len(atom_entries) > MAX_BATCH_SIZE:
        raise ValueError(f"Atom response exceeds the {MAX_BATCH_SIZE}-entry limit")

    entries: list[ArxivEntry] = []
    seen_ids: set[str] = set()
    for index, entry_node in enumerate(atom_entries, start=1):
        arxiv_id_full = _checked_text(
            entry_node.findtext("atom:id", default="", namespaces=ATOM_NS),
            label=f"Atom entry {index} id",
            max_chars=512,
        ).strip()
        parsed_id = urllib.parse.urlsplit(arxiv_id_full)
        scheme = parsed_id.scheme.lower()
        try:
            port = parsed_id.port
        except ValueError as exc:
            raise ValueError(f"Atom entry {index} has an invalid arXiv id URL") from exc
        if (
            scheme not in {"http", "https"}
            or (parsed_id.hostname or "").lower().rstrip(".") != "arxiv.org"
            or parsed_id.username is not None
            or parsed_id.password is not None
            or port not in (None, 80 if scheme == "http" else 443)
            or parsed_id.query
            or parsed_id.fragment
        ):
            raise ValueError(f"Atom entry {index} has an invalid arXiv id URL")
        path_prefix = "/abs/"
        if not parsed_id.path.startswith(path_prefix):
            raise ValueError(f"Atom entry {index} id URL must use /abs/")
        match = ARXIV_ATOM_ID_RE.fullmatch(parsed_id.path[len(path_prefix) :])
        if match is None:
            raise ValueError(f"Atom entry {index} contains an invalid modern arXiv id")
        arxiv_id = validate_arxiv_id(match.group(1))
        if arxiv_id in seen_ids:
            raise ValueError(f"Atom response contains duplicate id {arxiv_id}")
        seen_ids.add(arxiv_id)

        title = _checked_text(
            entry_node.findtext("atom:title", default="", namespaces=ATOM_NS),
            label=f"Atom entry {arxiv_id} title",
            max_chars=MAX_TITLE_CHARS,
        )
        summary = _checked_text(
            entry_node.findtext("atom:summary", default="", namespaces=ATOM_NS),
            label=f"Atom entry {arxiv_id} summary",
            max_chars=MAX_SUMMARY_CHARS,
            allow_empty=True,
        )
        published = _validate_timestamp(
            (
                entry_node.findtext("atom:published", default="", namespaces=ATOM_NS)
            ).strip(),
            label=f"Atom entry {arxiv_id} published",
        )
        updated = _validate_timestamp(
            (
                entry_node.findtext("atom:updated", default="", namespaces=ATOM_NS)
            ).strip(),
            label=f"Atom entry {arxiv_id} updated",
        )
        if _timestamp_value(updated) < _timestamp_value(published):
            raise ValueError(f"Atom entry {arxiv_id} updated precedes published")
        author_nodes = entry_node.findall("atom:author", ATOM_NS)
        if not 1 <= len(author_nodes) <= MAX_AUTHORS:
            raise ValueError(
                f"Atom entry {arxiv_id} authors must be a bounded nonempty list"
            )
        authors = [
            _checked_text(
                author.findtext("atom:name", default="", namespaces=ATOM_NS),
                label=f"Atom entry {arxiv_id} author {author_index}",
                max_chars=MAX_AUTHOR_CHARS,
            ).strip()
            for author_index, author in enumerate(author_nodes, start=1)
        ]

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


def main(argv: list[str] | None = None) -> int:
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
        nargs="+",
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
    args = parser.parse_args(argv)

    if not 1 <= args.batch_size <= MAX_BATCH_SIZE:
        parser.error(f"--batch-size must be in [1, {MAX_BATCH_SIZE}]")
    if (
        not math.isfinite(args.sleep)
        or args.sleep < 0.0
        or args.sleep > MAX_SLEEP_SECONDS
    ):
        parser.error(f"--sleep must be finite and in [0, {MAX_SLEEP_SECONDS:g}]")

    if args.ids is not None:
        try:
            requested = [validate_arxiv_id(value) for value in args.ids]
        except ValueError as exc:
            parser.error(str(exc))
        if len(set(requested)) != len(requested):
            parser.error("--ids contains a duplicate arXiv id")
    else:
        try:
            markdown_bytes = _read_regular_file(
                args.grandplan,
                max_bytes=MAX_MARKDOWN_BYTES,
                label="grandplan markdown",
            )
            markdown = markdown_bytes.decode("utf-8")
        except UnicodeDecodeError:
            parser.error("grandplan markdown is not valid UTF-8")
        except (OSError, ValueError) as exc:
            parser.error(str(exc))
        try:
            requested = extract_arxiv_ids_from_markdown(markdown)
        except ValueError as exc:
            parser.error(str(exc))

    requested = sorted(requested)
    if len(requested) > MAX_IDS:
        parser.error(f"Requested arXiv ids exceed the {MAX_IDS}-id limit")
    try:
        cache = load_cache(args.cache)
    except (OSError, ValueError) as exc:
        parser.error(str(exc))

    missing = [x for x in requested if x not in cache]
    if not missing:
        print("No missing arXiv IDs; cache is up to date.")
        return 0

    print(f"Cache path: {args.cache}")
    print(f"Requested IDs: {len(requested)}")
    print(f"Missing IDs:   {len(missing)}")

    batch_size = args.batch_size
    fetched = 0
    batches = [
        missing[start : start + batch_size]
        for start in range(0, len(missing), batch_size)
    ]
    for batch_index, batch in enumerate(batches, start=1):
        print(
            f"Fetching batch {batch_index}: {', '.join(batch)}",
            file=sys.stderr,
        )
        xml_text = fetch_arxiv_atom(batch)
        entries = parse_atom(xml_text)
        by_id = {e.arxiv_id: e for e in entries}
        unexpected = sorted(set(by_id) - set(batch))
        if unexpected:
            raise ValueError(f"arXiv API returned unexpected ids: {unexpected}")
        absent = sorted(set(batch) - set(by_id))
        if absent:
            raise ValueError(
                "arXiv API returned an incomplete batch; absent requested ids: "
                f"{absent}"
            )
        for arxiv_id in batch:
            entry = by_id[arxiv_id]
            cache[arxiv_id] = {
                "authors": entry.authors,
                "id": entry.arxiv_id,
                "published": entry.published,
                "summary": entry.summary,
                "title": entry.title,
                "updated": entry.updated,
            }
            fetched += 1
        if batch_index < len(batches) and args.sleep > 0:
            time.sleep(args.sleep)

    if fetched:
        save_cache(args.cache, cache)
    print(f"Updated cache with {fetched} entries.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
