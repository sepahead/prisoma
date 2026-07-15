from __future__ import annotations

import importlib.util
import json
import socket
import sys
import urllib.request
from pathlib import Path
from typing import Any

import pytest

from uidesigner import prompt_loop as ui


SCRIPT = Path(__file__).resolve().parents[2] / "scripts" / "update_arxiv_ref_cache.py"
SPEC = importlib.util.spec_from_file_location("prisoma_update_arxiv_cache", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
arxiv = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = arxiv
SPEC.loader.exec_module(arxiv)


def _ui_part_obj(**updates: Any) -> dict[str, Any]:
    obj: dict[str, Any] = {
        "type": "ui_part",
        "id": "safe_panel",
        "title": "Safe panel",
        "milestone": "optional",
        "requirements": ["Show a bounded panel."],
        "prompt_seed": "Render a clear scientific panel.",
        "negative_prompt": None,
        "image": {"width": 1024, "height": 768},
        "score_threshold": 9.0,
        "max_iterations": 2,
        "allow_img2img": False,
    }
    obj.update(updates)
    return obj


def _write_ui_markdown(path: Path, obj: dict[str, Any]) -> None:
    path.write_text(f"```json\n{json.dumps(obj)}\n```\n", encoding="utf-8")


def _cache_entry(arxiv_id: str = "1411.2003") -> dict[str, Any]:
    return {
        "authors": ["A. Author"],
        "id": arxiv_id,
        "published": "2014-11-07T19:00:57Z",
        "summary": "A bounded summary.",
        "title": "A bounded title",
        "updated": "2015-03-05T22:10:18Z",
    }


def _atom(arxiv_id: str = "1411.2003", *, version: str = "v2") -> str:
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <id>http://arxiv.org/abs/{arxiv_id}{version}</id>
    <updated>2015-03-05T22:10:18Z</updated>
    <published>2014-11-07T19:00:57Z</published>
    <title>A bounded title</title>
    <summary>A bounded summary.</summary>
    <author><name>A. Author</name></author>
  </entry>
</feed>
"""


class _Response:
    def __init__(
        self,
        data: bytes,
        *,
        url: str,
        content_type: str,
        content_length: str | None = None,
    ) -> None:
        self._data = data
        self._url = url
        self.headers = {"Content-Type": content_type}
        if content_length is not None:
            self.headers["Content-Length"] = content_length

    def read(self, amount: int = -1) -> bytes:
        return self._data if amount < 0 else self._data[:amount]

    def geturl(self) -> str:
        return self._url

    def close(self) -> None:
        return None

    def __enter__(self) -> _Response:
        return self

    def __exit__(self, *args: object) -> None:
        self.close()


def test_checked_ui_document_loads_current_parts() -> None:
    root = Path(__file__).resolve().parents[2]
    parts = ui.load_ui_parts(root / "uidesigner" / "UI.md")
    assert len(parts) == 5
    assert all(ui.UI_PART_ID_RE.fullmatch(part.part_id) for part in parts)


@pytest.mark.parametrize(
    "update",
    [
        {"id": "../escape"},
        {"image": {"width": True, "height": 768}},
        {"score_threshold": "9"},
        {"score_threshold": float("nan")},
        {"max_iterations": 0},
        {"allow_img2img": 1},
        {"requirements": ["valid", 3]},
    ],
)
def test_ui_document_rejects_unsafe_schema_values(
    tmp_path: Path, update: dict[str, Any]
) -> None:
    path = tmp_path / "UI.md"
    _write_ui_markdown(path, _ui_part_obj(**update))
    with pytest.raises(ValueError):
        ui.load_ui_parts(path)


def test_ui_document_rejects_duplicate_keys_and_symlink_input(tmp_path: Path) -> None:
    duplicate = tmp_path / "duplicate.md"
    duplicate.write_text(
        '```json\n{"type":"ui_part","type":"ui_part"}\n```\n',
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="Duplicate JSON key"):
        ui.load_ui_parts(duplicate)

    target = tmp_path / "target.md"
    _write_ui_markdown(target, _ui_part_obj())
    link = tmp_path / "linked.md"
    link.symlink_to(target)
    with pytest.raises(ValueError, match="regular, non-symlink"):
        ui.load_ui_parts(link)


def test_output_directories_are_single_segment_and_do_not_follow_symlinks(
    tmp_path: Path,
) -> None:
    base = tmp_path / "out"
    base.mkdir()
    outside = tmp_path / "outside"
    outside.mkdir()
    (base / "safe_panel").symlink_to(outside, target_is_directory=True)

    with pytest.raises(FileExistsError):
        ui._create_contained_directory(base, "safe_panel")
    with pytest.raises(ValueError, match="one safe path segment"):
        ui._create_contained_directory(base, "../escape")
    assert list(outside.iterdir()) == []


def test_https_validation_rejects_downgrade_credentials_and_private_destinations(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    with pytest.raises(ValueError, match="Only HTTPS"):
        ui._validate_https_url("http://example.com/image.png")
    with pytest.raises(ValueError, match="credentials"):
        ui._validate_https_url("https://user@example.com/image.png")

    monkeypatch.setattr(
        socket,
        "getaddrinfo",
        lambda *args, **kwargs: [
            (socket.AF_INET, socket.SOCK_STREAM, 6, "", ("127.0.0.1", 443))
        ],
    )
    with pytest.raises(ValueError, match="non-public"):
        ui._validate_https_url(
            "https://images.example/image.png", require_public_destination=True
        )

    handler = ui._ValidatedRedirectHandler(
        allowed_hosts={"api.example"}, require_public_destination=False
    )
    request = urllib.request.Request("https://api.example/request")
    with pytest.raises(ValueError, match="Only HTTPS"):
        handler.redirect_request(
            request, None, 302, "Found", {}, "http://api.example/redirect"
        )
    with pytest.raises(ValueError, match="not permitted"):
        handler.redirect_request(
            request, None, 302, "Found", {}, "https://other.example/redirect"
        )


def test_http_and_atomic_output_limits_fail_closed(tmp_path: Path) -> None:
    response = _Response(
        b"12345",
        url="https://api.example/request",
        content_type="application/json",
        content_length="5",
    )
    with pytest.raises(ValueError, match="exceeds"):
        ui._read_response_bytes(response, max_bytes=4, label="fixture")

    outside = tmp_path / "outside.txt"
    outside.write_text("unchanged", encoding="utf-8")
    link = tmp_path / "output.txt"
    link.symlink_to(outside)
    with pytest.raises(ValueError, match="non-symlink"):
        ui._atomic_write_bytes(link, b"replacement", max_bytes=64)
    assert outside.read_text(encoding="utf-8") == "unchanged"


def test_image_download_requires_bounded_png_without_network(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def set_response(data: bytes, content_type: str) -> None:
        monkeypatch.setattr(
            ui,
            "_open_validated_url",
            lambda *args, **kwargs: _Response(
                data,
                url="https://images.example/output.png",
                content_type=content_type,
            ),
        )

    set_response(ui.PNG_SIGNATURE + b"fixture", "image/png")
    assert ui._http_get_image("https://images.example/output.png").startswith(
        ui.PNG_SIGNATURE
    )
    set_response(b"not-png", "image/png")
    with pytest.raises(ValueError, match="PNG signature"):
        ui._http_get_image("https://images.example/output.png")
    set_response(ui.PNG_SIGNATURE, "text/html")
    with pytest.raises(ValueError, match="not PNG"):
        ui._http_get_image("https://images.example/output.png")


def test_model_json_and_review_schema_are_strict(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    assert ui._parse_json_from_model('```json\n{"score": 9}\n```') == {"score": 9}
    for malformed in (
        'prefix {"score": 9}',
        '{"score": NaN}',
        '{"score": 9, "score": 8}',
    ):
        with pytest.raises(ValueError):
            ui._parse_json_from_model(malformed)

    client = ui.GeminiVertexClient(
        project="valid-project",
        location="us-central1",
        model_vision="gemini-1.5-pro",
        model_text="gemini-1.5-pro",
    )
    client._mode = "rest"
    part = ui.UiPart(
        part_id="safe_panel",
        title="Safe panel",
        milestone="optional",
        requirements=["Show a panel."],
        prompt_seed="Render a panel.",
        negative_prompt=None,
        image_width=1024,
        image_height=768,
        score_threshold=9.0,
        max_iterations=2,
        allow_img2img=False,
    )
    monkeypatch.setattr(
        client,
        "_critique_via_rest",
        lambda **kwargs: '{"score":11,"pass":true,"issues":[],"fixes":[]}',
    )
    with pytest.raises(ValueError, match="finite and in"):
        client.critique_ui(part=part, image_bytes=ui.PNG_SIGNATURE)


def test_subprocess_output_and_runtime_are_bounded() -> None:
    return_code, output = ui._run_command_bounded(
        [sys.executable, "-c", "print('ok')"], timeout_s=2.0, max_output_bytes=32
    )
    assert return_code == 0
    assert output == b"ok\n"

    with pytest.raises(RuntimeError, match="output exceeds"):
        ui._run_command_bounded(
            [sys.executable, "-c", "print('x' * 10000)"],
            timeout_s=2.0,
            max_output_bytes=64,
        )
    with pytest.raises(TimeoutError, match="timeout"):
        ui._run_command_bounded(
            [sys.executable, "-c", "import time; time.sleep(1)"],
            timeout_s=0.02,
            max_output_bytes=64,
        )


def test_arxiv_ids_accept_four_or_five_serial_digits_without_partial_matches() -> None:
    text = (
        "arXiv:1411.2003. arXiv:2503.20314v2, duplicate arXiv:1411.2003; "
        "invalid arXiv:2503.123456 and arXiv:2513.12345"
    )
    assert arxiv.extract_arxiv_ids_from_markdown(text) == [
        "1411.2003",
        "2503.20314",
    ]
    assert arxiv.validate_arxiv_id("1411.2003") == "1411.2003"
    assert arxiv.validate_arxiv_id("2503.20314") == "2503.20314"
    for invalid in (
        "arXiv:2503.20314",
        "2503.20314v2",
        " 2503.20314",
        "2513.20314",
        "0703.1234",
        "1411.12345",
        "2503.1234",
        "2503.00000",
        "2503.123",
        "2503.123456",
    ):
        with pytest.raises(ValueError):
            arxiv.validate_arxiv_id(invalid)


def test_arxiv_cache_is_schema_checked_atomic_and_symlink_safe(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    cache_path = tmp_path / "cache.json"
    expected = {"1411.2003": _cache_entry()}
    arxiv.save_cache(cache_path, expected)
    assert arxiv.load_cache(cache_path) == expected
    assert not list(tmp_path.glob(".cache.json.*"))

    original = cache_path.read_bytes()

    def fail_replace(source: object, target: object) -> None:
        raise OSError("injected failure")

    monkeypatch.setattr(arxiv.os, "replace", fail_replace)
    with pytest.raises(OSError, match="injected failure"):
        arxiv.save_cache(cache_path, expected)
    assert cache_path.read_bytes() == original
    assert not list(tmp_path.glob(".cache.json.*"))
    monkeypatch.undo()

    malformed = tmp_path / "malformed.json"
    malformed.write_text('{"1411.2003":{"id":"1411.2003"}}', encoding="utf-8")
    with pytest.raises(ValueError, match="invalid schema"):
        arxiv.load_cache(malformed)

    outside = tmp_path / "outside.json"
    outside.write_text("{}", encoding="utf-8")
    linked = tmp_path / "linked.json"
    linked.symlink_to(outside)
    with pytest.raises(ValueError, match="must not be a symlink"):
        arxiv.load_cache(linked)
    with pytest.raises(ValueError, match="non-symlink"):
        arxiv.save_cache(linked, expected)
    assert outside.read_text(encoding="utf-8") == "{}"


def test_atom_parser_accepts_versioned_ids_and_rejects_active_content() -> None:
    entries = arxiv.parse_atom(_atom())
    assert [entry.arxiv_id for entry in entries] == ["1411.2003"]
    assert entries[0].title == "A bounded title"

    with pytest.raises(ValueError, match="DTD or entity"):
        arxiv.parse_atom('<!DOCTYPE feed [<!ENTITY x "boom">]><feed>&x;</feed>')
    atom = _atom()
    entry_body = atom.split("<entry>", 1)[1].split("</entry>", 1)[0]
    duplicate = atom.replace("</feed>", f"<entry>{entry_body}</entry></feed>")
    with pytest.raises(ValueError, match="duplicate id"):
        arxiv.parse_atom(duplicate)


def test_arxiv_fetch_uses_bounded_https_request_without_network(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    opened: dict[str, Any] = {}
    response = _Response(
        _atom().encode("utf-8"),
        url="https://export.arxiv.org/api/query?id_list=1411.2003",
        content_type="application/atom+xml; charset=utf-8",
    )

    class _Opener:
        def open(self, request: urllib.request.Request, *, timeout: float) -> _Response:
            opened["url"] = request.full_url
            opened["timeout"] = timeout
            return response

    monkeypatch.setattr(urllib.request, "build_opener", lambda *args: _Opener())
    assert "1411.2003" in arxiv.fetch_arxiv_atom(["1411.2003"])
    assert opened["url"].startswith("https://export.arxiv.org/api/query?")
    assert opened["timeout"] == arxiv.HTTP_TIMEOUT_SECONDS

    handler = arxiv._ArxivRedirectHandler()
    with pytest.raises(ValueError, match="must use HTTPS"):
        handler.redirect_request(
            urllib.request.Request(opened["url"]),
            None,
            302,
            "Found",
            {},
            "http://export.arxiv.org/api/query?id_list=1411.2003",
        )


def test_arxiv_cli_rejects_invalid_ids_before_network(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setattr(
        arxiv,
        "fetch_arxiv_atom",
        lambda ids: pytest.fail("network helper must not be called"),
    )
    with pytest.raises(SystemExit) as exc_info:
        arxiv.main(
            [
                "--ids",
                "2503.123456",
                "--cache",
                str(tmp_path / "cache.json"),
            ]
        )
    assert exc_info.value.code == 2
    assert not (tmp_path / "cache.json").exists()


def test_arxiv_cli_offline_fetch_writes_valid_cache_atomically(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setattr(arxiv, "fetch_arxiv_atom", lambda ids: _atom(ids[0]))
    cache_path = tmp_path / "cache.json"
    assert (
        arxiv.main(
            [
                "--ids",
                "1411.2003",
                "--cache",
                str(cache_path),
                "--sleep",
                "0",
            ]
        )
        == 0
    )
    assert arxiv.load_cache(cache_path)["1411.2003"]["id"] == "1411.2003"
    assert not list(tmp_path.glob(".cache.json.*"))
