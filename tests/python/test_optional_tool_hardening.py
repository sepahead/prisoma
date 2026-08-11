from __future__ import annotations

import importlib.util
import json
import os
import runpy
import selectors
import socket
import struct
import subprocess
import sys
import time
import urllib.parse
import urllib.request
import zlib
from pathlib import Path
from types import SimpleNamespace
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
<feed xmlns="http://www.w3.org/2005/Atom">{_atom_entry(arxiv_id, version=version)}
</feed>
"""


def _atom_entry(arxiv_id: str, *, version: str = "v2") -> str:
    return f"""
  <entry>
    <id>http://arxiv.org/abs/{arxiv_id}{version}</id>
    <updated>2015-03-05T22:10:18Z</updated>
    <published>2014-11-07T19:00:57Z</published>
    <title>A bounded title</title>
    <summary>A bounded summary.</summary>
    <author><name>A. Author</name></author>
  </entry>
"""


def _atom_many(ids: list[str]) -> str:
    entries = "".join(_atom_entry(arxiv_id) for arxiv_id in ids)
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<feed xmlns="http://www.w3.org/2005/Atom">'
        f"{entries}\n"
        "</feed>\n"
    )


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


def _png_chunk(
    chunk_type: bytes, payload: bytes, *, corrupt_crc: bool = False
) -> bytes:
    crc = zlib.crc32(chunk_type)
    crc = zlib.crc32(payload, crc) & 0xFFFFFFFF
    if corrupt_crc:
        crc ^= 1
    return (
        struct.pack(">I", len(payload)) + chunk_type + payload + struct.pack(">I", crc)
    )


def _png_ihdr(
    width: int,
    height: int,
    *,
    bit_depth: int = 8,
    color_type: int = 2,
    compression: int = 0,
    filter_method: int = 0,
    interlace: int = 0,
) -> bytes:
    return _png_chunk(
        b"IHDR",
        struct.pack(
            ">IIBBBBB",
            width,
            height,
            bit_depth,
            color_type,
            compression,
            filter_method,
            interlace,
        ),
    )


def _png(
    width: int = 2,
    height: int = 2,
    *,
    bit_depth: int = 8,
    color_type: int = 2,
    compression: int = 0,
    filter_method: int = 0,
    interlace: int = 0,
    scanline_filter: int = 0,
    before_idat: tuple[bytes, ...] = (),
    after_idat: tuple[bytes, ...] = (),
    compressed_payload: bytes | None = None,
    corrupt_idat_crc: bool = False,
) -> bytes:
    channels = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}.get(color_type, 1)
    row_bytes = (width * channels * max(bit_depth, 1) + 7) // 8
    raw = b"".join(bytes((scanline_filter,)) + bytes(row_bytes) for _ in range(height))
    compressed = (
        zlib.compress(raw) if compressed_payload is None else compressed_payload
    )
    return b"".join(
        (
            ui.PNG_SIGNATURE,
            _png_ihdr(
                width,
                height,
                bit_depth=bit_depth,
                color_type=color_type,
                compression=compression,
                filter_method=filter_method,
                interlace=interlace,
            ),
            *before_idat,
            _png_chunk(b"IDAT", compressed, corrupt_crc=corrupt_idat_crc),
            *after_idat,
            _png_chunk(b"IEND", b""),
        )
    )


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


def test_ui_reader_rejects_a_final_lexical_path_swap(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    path = tmp_path / "source.md"
    replacement = tmp_path / "replacement.md"
    path.write_text("stable\n", encoding="utf-8")
    replacement.write_text("stable\n", encoding="utf-8")
    real_lstat = ui.Path.lstat
    calls = 0

    def swapped_lstat(candidate: Path) -> object:
        nonlocal calls
        if candidate == path:
            calls += 1
            if calls == 2:
                return real_lstat(replacement)
        return real_lstat(candidate)

    monkeypatch.setattr(ui.Path, "lstat", swapped_lstat)
    with pytest.raises(ValueError, match="changed while it was read"):
        ui._read_regular_file(path, max_bytes=1024, label="fixture")


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

    outside_dir = tmp_path / "outside"
    outside_dir.mkdir()
    linked_parent = tmp_path / "linked-parent"
    linked_parent.symlink_to(outside_dir, target_is_directory=True)
    with pytest.raises(ValueError, match="parent must be a regular directory"):
        ui._atomic_write_bytes(
            linked_parent / "output.txt", b"replacement", max_bytes=64
        )
    assert list(outside_dir.iterdir()) == []


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

    valid = _png(width=2, height=3)
    info = ui._validate_png(valid, label="fixture", expected_size=(2, 3))
    assert (info.width, info.height, info.mode) == (2, 3, "RGB")

    set_response(valid, "image/png")
    assert (
        ui._http_get_image("https://images.example/output.png", expected_size=(2, 3))
        == valid
    )
    set_response(b"not-png", "image/png")
    with pytest.raises(ValueError, match="PNG signature"):
        ui._http_get_image("https://images.example/output.png")
    set_response(valid, "text/html")
    with pytest.raises(ValueError, match="not PNG"):
        ui._http_get_image("https://images.example/output.png")


@pytest.mark.parametrize(
    "malformed",
    (
        ui.PNG_SIGNATURE,
        _png()[:-1],
        _png(compressed_payload=b"not-a-zlib-stream"),
        _png(scanline_filter=5),
    ),
)
def test_png_preflight_rejects_signature_only_truncation_and_invalid_image_data(
    malformed: bytes,
) -> None:
    with pytest.raises(ValueError):
        ui._validate_png(malformed, label="fixture")


def test_png_validation_rejects_corrupt_or_unsupported_files() -> None:
    with pytest.raises(ValueError, match="valid decodable PNG"):
        ui._validate_png(_png(corrupt_idat_crc=True), label="fixture")


@pytest.mark.parametrize(
    "malformed",
    (
        _png(width=ui.MAX_IMAGE_SIDE + 1, height=1),
        _png(bit_depth=4, color_type=2),
        _png(interlace=2),
    ),
)
def test_png_preflight_rejects_extreme_dimensions_and_unsupported_ihdr(
    malformed: bytes,
) -> None:
    with pytest.raises(ValueError):
        ui._validate_png(malformed, label="fixture")


def test_png_preflight_rejects_wrong_requested_dimensions() -> None:
    with pytest.raises(ValueError, match="do not match the requested"):
        ui._validate_png(_png(width=2, height=3), label="fixture", expected_size=(3, 2))


def test_png_preflight_enforces_aggregate_pixel_budget(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(ui, "MAX_IMAGE_PIXELS", 3)
    with pytest.raises(ValueError, match="pixel limit"):
        ui._validate_png(_png(width=2, height=2), label="fixture")


def test_fal_backend_binds_download_and_initial_image_dimensions(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    client = ui.FalGptImageClient(fal_key="test-key", endpoint="fal-ai/test-model")
    requests: list[tuple[str, dict[str, Any]]] = []

    def fake_post(
        url: str, headers: dict[str, str], payload: dict[str, Any], **kwargs: Any
    ) -> dict[str, Any]:
        requests.append((url, payload))
        return {"images": [{"url": "https://images.example/out.png"}]}

    monkeypatch.setattr(ui, "_http_json_post", fake_post)

    response: dict[str, bytes] = {"image": _png(width=1024, height=1024)}
    monkeypatch.setattr(
        ui,
        "_open_validated_url",
        lambda *args, **kwargs: _Response(
            response["image"],
            url="https://images.example/out.png",
            content_type="image/png",
        ),
    )
    image, _ = client.generate(
        prompt="Render a bounded panel.",
        negative_prompt="No clutter.",
        width=1024,
        height=1024,
    )
    assert image == response["image"]
    assert requests[-1][0] == "https://fal.run/fal-ai/test-model"
    assert requests[-1][1]["image_size"] == "1024x1024"
    assert requests[-1][1]["output_format"] == "png"
    assert requests[-1][1]["num_images"] == 1
    assert requests[-1][1]["prompt"].endswith(
        "Avoid these visual elements: No clutter."
    )
    assert "negative_prompt" not in requests[-1][1]

    response["image"] = _png(width=2, height=2)
    with pytest.raises(ValueError, match="do not match the requested"):
        client.generate(
            prompt="Render a bounded panel.",
            negative_prompt=None,
            width=1024,
            height=1024,
        )

    init_image = tmp_path / "init.png"
    init_image.write_bytes(_png(width=2, height=2))
    with pytest.raises(ValueError, match="do not match the requested"):
        client.generate(
            prompt="Render a bounded panel.",
            negative_prompt=None,
            width=1024,
            height=1024,
            init_image_path=init_image,
            mode="img2img",
        )

    init_image.write_bytes(_png(width=1024, height=1024))
    response["image"] = _png(width=1024, height=1024)
    client.generate(
        prompt="Revise a bounded panel.",
        negative_prompt=None,
        width=1024,
        height=1024,
        init_image_path=init_image,
        mode="img2img",
    )
    assert requests[-1][0] == "https://fal.run/fal-ai/test-model/edit"
    assert requests[-1][1]["image_urls"][0].startswith("data:image/png;base64,")
    assert "image" not in requests[-1][1]

    with pytest.raises(ValueError, match="must be one of"):
        client.generate(
            prompt="Render an unsupported size.",
            negative_prompt=None,
            width=1600,
            height=1000,
        )


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
        image_width=256,
        image_height=256,
        score_threshold=9.0,
        max_iterations=2,
        allow_img2img=False,
    )
    monkeypatch.setattr(
        client,
        "_critique_via_rest",
        lambda **kwargs: '{"score":11,"pass":true,"issues":[],"fixes":[]}',
    )
    with pytest.raises(ValueError, match="valid decodable PNG"):
        client.critique_ui(part=part, image_bytes=ui.PNG_SIGNATURE)
    with pytest.raises(ValueError, match="finite and in"):
        client.critique_ui(part=part, image_bytes=_png(width=256, height=256))


def test_iteration_rejects_invalid_png_before_saving_or_forwarding(
    tmp_path: Path,
) -> None:
    part = ui.UiPart(
        part_id="safe_panel",
        title="Safe panel",
        milestone="optional",
        requirements=["Show a panel."],
        prompt_seed="Render a panel.",
        negative_prompt=None,
        image_width=256,
        image_height=256,
        score_threshold=9.0,
        max_iterations=1,
        allow_img2img=False,
    )

    class InvalidFal:
        def generate(self, **kwargs: object) -> tuple[bytes, dict[str, Any]]:
            return ui.PNG_SIGNATURE, {}

    class ForbiddenGemini:
        def critique_ui(self, **kwargs: object) -> ui.Review:
            pytest.fail("invalid image must not be forwarded")

    output = tmp_path / "out"
    output.mkdir()
    with pytest.raises(ValueError, match="valid decodable PNG"):
        ui.optimize_part(
            part=part,
            fal=InvalidFal(),  # type: ignore[arg-type]
            gemini=ForbiddenGemini(),  # type: ignore[arg-type]
            out_dir=output,
            dry_run=False,
            sleep_s=0,
        )
    assert not list(output.rglob("*.png"))


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
    with pytest.raises(ValueError, match="outside limits"):
        ui._run_command_bounded(
            [sys.executable, "-c", "pass"],
            timeout_s=2.0,
            max_output_bytes=ui.MAX_SUBPROCESS_OUTPUT_BYTES + 1,
        )


def test_bounded_subprocess_uses_an_isolated_process_group(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    real_popen = ui.subprocess.Popen
    observed: dict[str, object] = {}

    def recording_popen(*args: object, **kwargs: object) -> subprocess.Popen[bytes]:
        observed.update(kwargs)
        return real_popen(*args, **kwargs)

    monkeypatch.setattr(ui.subprocess, "Popen", recording_popen)
    return_code, output = ui._run_command_bounded(
        [sys.executable, "-c", "print('isolated')"],
        timeout_s=2.0,
        max_output_bytes=64,
    )

    assert return_code == 0
    assert output == b"isolated\n"
    assert observed["start_new_session"] is (os.name == "posix")


@pytest.mark.skipif(os.name != "posix", reason="POSIX descriptor behavior")
def test_bounded_subprocess_can_finish_after_closing_its_output_descriptors(
    tmp_path: Path,
) -> None:
    marker = tmp_path / "cleanup-finished.txt"
    code = (
        "import os,pathlib,sys,time;"
        "os.close(1);os.close(2);time.sleep(0.05);"
        "pathlib.Path(sys.argv[1]).write_text('finished', encoding='utf-8')"
    )

    return_code, output = ui._run_command_bounded(
        [sys.executable, "-c", code, str(marker)],
        timeout_s=2.0,
        max_output_bytes=64,
    )

    assert return_code == 0
    assert output == b""
    assert marker.read_text(encoding="utf-8") == "finished"


@pytest.mark.parametrize(
    ("relative_script", "helper_name"),
    [
        ("scripts/audit_repo_truth.py", "_terminate"),
        ("scripts/audit_docset_claims.py", "_terminate_process_group"),
        ("scripts/generate_candidate_release.py", "_terminate_process_group"),
        ("scripts/generate_release_review.py", "_terminate_process_group"),
        ("scripts/generate_third_party_notices.py", "_terminate"),
        ("scripts/generate_capability_matrix.py", "_terminate"),
    ],
)
def test_bounded_script_cleanup_never_signals_a_reaped_leader(
    relative_script: str,
    helper_name: str,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    root = Path(__file__).resolve().parents[2]
    namespace = runpy.run_path(os.fspath(root / relative_script))

    def forbidden_killpg(_pgid: int, _signal: int) -> None:
        raise AssertionError("reaped process group must not be signaled")

    monkeypatch.setattr(namespace["os"], "killpg", forbidden_killpg)
    namespace[helper_name](SimpleNamespace(pid=123_456, returncode=0))


def test_bounded_subprocess_cleans_up_when_selector_setup_fails(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    marker = tmp_path / "escaped.txt"

    def fail_selector() -> selectors.BaseSelector:
        raise RuntimeError("injected selector failure")

    monkeypatch.setattr(ui.selectors, "DefaultSelector", fail_selector)
    with pytest.raises(RuntimeError, match="injected selector failure"):
        ui._run_command_bounded(
            [
                sys.executable,
                "-c",
                (
                    "import pathlib,sys,time;"
                    "time.sleep(0.35);"
                    "pathlib.Path(sys.argv[1]).write_text("
                    "'escaped', encoding='utf-8')"
                ),
                str(marker),
            ],
            timeout_s=2.0,
            max_output_bytes=64,
        )
    time.sleep(0.5)
    assert not marker.exists()


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

    outside_dir = tmp_path / "outside"
    outside_dir.mkdir()
    linked_parent = tmp_path / "linked-parent"
    linked_parent.symlink_to(outside_dir, target_is_directory=True)
    with pytest.raises(ValueError, match="parent must be a regular directory"):
        arxiv.save_cache(linked_parent / "cache.json", expected)
    assert list(outside_dir.iterdir()) == []


def test_arxiv_reader_rejects_a_final_lexical_path_swap(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    cache_path = tmp_path / "cache.json"
    replacement = tmp_path / "replacement.json"
    cache_path.write_text("{}\n", encoding="utf-8")
    replacement.write_text("{}\n", encoding="utf-8")
    real_lstat = arxiv.Path.lstat
    calls = 0

    def swapped_lstat(candidate: Path) -> object:
        nonlocal calls
        if candidate == cache_path:
            calls += 1
            if calls == 2:
                return real_lstat(replacement)
        return real_lstat(candidate)

    monkeypatch.setattr(arxiv.Path, "lstat", swapped_lstat)
    with pytest.raises(ValueError, match="changed while it was read"):
        arxiv._read_regular_file(
            cache_path,
            max_bytes=arxiv.MAX_CACHE_BYTES,
            label="arXiv cache",
        )


def test_atom_parser_accepts_versioned_ids_and_rejects_active_or_excessive_content(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    entries = arxiv.parse_atom(_atom())
    assert [entry.arxiv_id for entry in entries] == ["1411.2003"]
    assert entries[0].title == "A bounded title"

    with pytest.raises(ValueError, match="DTD or entity"):
        arxiv.parse_atom('<!DOCTYPE feed [<!ENTITY x "boom">]><feed>&x;</feed>')
    with pytest.raises(ValueError, match="DTD or entity"):
        arxiv.parse_atom(
            '<!DOCTYPE feed [<!ENTITY x SYSTEM "file:///etc/passwd">]>'
            '<feed xmlns="http://www.w3.org/2005/Atom">&x;</feed>'
        )
    atom = _atom()
    entry_body = atom.split("<entry>", 1)[1].split("</entry>", 1)[0]
    duplicate = atom.replace("</feed>", f"<entry>{entry_body}</entry></feed>")
    with pytest.raises(ValueError, match="duplicate id"):
        arxiv.parse_atom(duplicate)
    monkeypatch.setattr(arxiv, "MAX_ATOM_ELEMENTS", 3)
    with pytest.raises(ValueError, match="element limit"):
        arxiv.parse_atom(atom)


def test_arxiv_fetch_uses_bounded_https_request_without_network(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    opened: dict[str, Any] = {"urls": []}

    class _Opener:
        def open(self, request: urllib.request.Request, *, timeout: float) -> _Response:
            opened["urls"].append(request.full_url)
            opened["timeout"] = timeout
            return _Response(
                _atom().encode("utf-8"),
                url=request.full_url,
                content_type="application/atom+xml; charset=utf-8",
            )

    monkeypatch.setattr(urllib.request, "build_opener", lambda *args: _Opener())
    assert "1411.2003" in arxiv.fetch_arxiv_atom(["1411.2003"])
    opened_url = opened["urls"][0]
    assert opened_url.startswith("https://export.arxiv.org/api/query?")
    query = urllib.parse.parse_qs(urllib.parse.urlsplit(opened_url).query)
    assert query == {"id_list": ["1411.2003"], "max_results": ["1"]}
    assert opened["timeout"] == arxiv.HTTP_TIMEOUT_SECONDS

    batch = [f"2501.{serial:05d}" for serial in range(1, 26)]
    arxiv.fetch_arxiv_atom(batch)
    batch_query = urllib.parse.parse_qs(urllib.parse.urlsplit(opened["urls"][1]).query)
    assert batch_query["id_list"] == [",".join(batch)]
    assert batch_query["max_results"] == ["25"]

    handler = arxiv._ArxivRedirectHandler()
    with pytest.raises(ValueError, match="must use HTTPS"):
        handler.redirect_request(
            urllib.request.Request(opened_url),
            None,
            302,
            "Found",
            {},
            "http://export.arxiv.org/api/query?id_list=1411.2003",
        )
    with pytest.raises(ValueError, match="remain on"):
        handler.redirect_request(
            urllib.request.Request(opened_url),
            None,
            302,
            "Found",
            {},
            "https://example.org/api/query?id_list=1411.2003",
        )
    with pytest.raises(ValueError, match="path or fragment"):
        handler.redirect_request(
            urllib.request.Request(opened_url),
            None,
            302,
            "Found",
            {},
            "https://export.arxiv.org/other?id_list=1411.2003",
        )
    with pytest.raises(ValueError, match="must equal"):
        handler.redirect_request(
            urllib.request.Request(opened_url),
            None,
            302,
            "Found",
            {},
            "https://export.arxiv.org/api/query?id_list=1411.2003&max_results=10",
        )

    oversized = _Response(
        b"12345",
        url=opened_url,
        content_type="application/atom+xml",
        content_length="5",
    )
    with pytest.raises(ValueError, match="exceeds"):
        arxiv._read_response_bytes(oversized, max_bytes=4)


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


def test_arxiv_cli_rejects_incomplete_batch_without_partial_cache_write(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    cache_path = tmp_path / "cache.json"
    arxiv.save_cache(cache_path, {"1411.2003": _cache_entry()})
    original = cache_path.read_bytes()
    requested = [f"2501.{serial:05d}" for serial in range(1, 26)]
    monkeypatch.setattr(
        arxiv,
        "fetch_arxiv_atom",
        lambda ids: _atom_many(ids[:10]),
    )

    with pytest.raises(ValueError, match="incomplete batch"):
        arxiv.main(
            [
                "--ids",
                *requested,
                "--batch-size",
                "25",
                "--cache",
                str(cache_path),
                "--sleep",
                "0",
            ]
        )

    assert cache_path.read_bytes() == original
    assert arxiv.load_cache(cache_path) == {"1411.2003": _cache_entry()}
    assert not list(tmp_path.glob(".cache.json.*"))
