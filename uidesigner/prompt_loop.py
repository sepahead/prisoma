#!/usr/bin/env python3
"""
UI prompt optimizer (optional design utility)

Pipeline (per ui_part in uidesigner/UI.md):
  1) Generate image with gpt-image-1.5 (via FAL) from a prompt
  2) Ask Gemini Vision (Vertex AI) to score the image against UI.md requirements
  3) Ask Gemini to rewrite the prompt based on the critique
  4) Iterate until score >= threshold (or max iterations)

This script is designed to be run by a human with credentials configured:
  - FAL: set FAL_KEY and verify the endpoint name (default: fal-ai/gpt-image-1.5)
  - Vertex AI: set GOOGLE_CLOUD_PROJECT and GOOGLE_CLOUD_LOCATION and authenticate
    (ADC via gcloud or service account).

Nothing in this repo ships cloud credentials. Do not commit keys.

This remains an optional, operator-run design aid. Its size, timeout, redirect, path, and
schema checks reduce accidental misuse; they are not an adversarial-filesystem sandbox,
and the public-address DNS preflight does not guarantee protection from DNS rebinding,
proxy behavior, or a compromised remote service.
"""

from __future__ import annotations

import argparse
import base64
import dataclasses
import datetime as dt
import ipaddress
import json
import math
import os
import re
import selectors
import socket
import stat
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any, Literal


MAX_UI_MD_BYTES = 2 * 1024 * 1024
MAX_UI_PARTS = 64
MAX_REQUIREMENTS = 64
MAX_SHORT_TEXT_CHARS = 1_024
MAX_PROMPT_CHARS = 65_536
MAX_MODEL_RESPONSE_CHARS = 131_072
MAX_HTTP_JSON_REQUEST_BYTES = 48 * 1024 * 1024
MAX_HTTP_JSON_RESPONSE_BYTES = 4 * 1024 * 1024
MAX_IMAGE_BYTES = 32 * 1024 * 1024
MAX_INIT_IMAGE_BYTES = 32 * 1024 * 1024
MAX_OUTPUT_JSON_BYTES = 8 * 1024 * 1024
MAX_OUTPUT_TEXT_BYTES = 1024 * 1024
MAX_IMAGE_SIDE = 4_096
MAX_IMAGE_PIXELS = 16_777_216
MIN_IMAGE_SIDE = 256
MAX_ITERATIONS = 20
MAX_SLEEP_SECONDS = 300.0
HTTP_TIMEOUT_SECONDS = 180.0
GCLOUD_TIMEOUT_SECONDS = 30.0
MAX_SUBPROCESS_OUTPUT_BYTES = 64 * 1024

UI_PART_ID_RE = re.compile(r"[a-z][a-z0-9_]{0,63}\Z")
FAL_ENDPOINT_RE = re.compile(
    r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}"
    r"(?:/[A-Za-z0-9][A-Za-z0-9._-]{0,127}){1,3}\Z"
)
GCP_PROJECT_RE = re.compile(r"[a-z][a-z0-9-]{4,28}[a-z0-9]\Z")
GCP_LOCATION_RE = re.compile(r"[a-z][a-z0-9-]{0,62}\Z")
MODEL_NAME_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}\Z")
ENV_NAME_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*\Z")
SAFE_OUTPUT_SEGMENT_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}\Z")
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


@dataclasses.dataclass(frozen=True)
class UiPart:
    part_id: str
    title: str
    milestone: str
    requirements: list[str]
    prompt_seed: str
    negative_prompt: str | None
    image_width: int
    image_height: int
    score_threshold: float
    max_iterations: int
    allow_img2img: bool


@dataclasses.dataclass(frozen=True)
class Review:
    score: float
    pass_: bool
    issues: list[str]
    fixes: list[str]
    raw_text: str


def _now_stamp() -> str:
    return dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")


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


def _atomic_write_bytes(path: Path, data: bytes, *, max_bytes: int) -> None:
    if len(data) > max_bytes:
        raise ValueError(f"Output for {path} exceeds the {max_bytes}-byte limit")
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.parent.is_symlink() or not path.parent.is_dir():
        raise ValueError(f"Output parent must be a regular directory: {path.parent}")
    if path.exists() or path.is_symlink():
        metadata = path.lstat()
        if not stat.S_ISREG(metadata.st_mode):
            raise ValueError(
                f"Output target must be a regular, non-symlink file: {path}"
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


def _write_text(path: Path, text: str) -> None:
    _atomic_write_bytes(path, text.encode("utf-8"), max_bytes=MAX_OUTPUT_TEXT_BYTES)


def _write_json(path: Path, obj: Any) -> None:
    encoded = (
        json.dumps(obj, indent=2, sort_keys=True, allow_nan=False) + "\n"
    ).encode("utf-8")
    _atomic_write_bytes(path, encoded, max_bytes=MAX_OUTPUT_JSON_BYTES)


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"Duplicate JSON key: {key!r}")
        result[key] = value
    return result


def _reject_json_constant(value: str) -> Any:
    raise ValueError(f"Non-standard JSON numeric constant: {value}")


def _decode_json_object(data: bytes | str, *, label: str) -> dict[str, Any]:
    try:
        obj = json.loads(
            data,
            object_pairs_hook=_reject_duplicate_keys,
            parse_constant=_reject_json_constant,
        )
    except (UnicodeDecodeError, json.JSONDecodeError, RecursionError) as exc:
        raise ValueError(f"{label} is not valid UTF-8 JSON") from exc
    if not isinstance(obj, dict):
        raise ValueError(f"{label} must be a JSON object")
    return obj


def _validate_https_url(
    url: str,
    *,
    allowed_hosts: set[str] | None = None,
    require_public_destination: bool = False,
) -> urllib.parse.SplitResult:
    if len(url) > 8_192 or any(char.isspace() or ord(char) < 0x20 for char in url):
        raise ValueError("URL is invalid or exceeds the 8192-character limit")
    parsed = urllib.parse.urlsplit(url)
    if parsed.scheme.lower() != "https":
        raise ValueError("Only HTTPS URLs are permitted")
    if parsed.username is not None or parsed.password is not None:
        raise ValueError("URL credentials are not permitted")
    if parsed.fragment:
        raise ValueError("URL fragments are not permitted")
    host = (parsed.hostname or "").lower().rstrip(".")
    if not host:
        raise ValueError("URL host is required")
    try:
        port = parsed.port
    except ValueError as exc:
        raise ValueError("URL port is invalid") from exc
    if port not in (None, 443):
        raise ValueError("Only the default HTTPS port is permitted")
    if allowed_hosts is not None and host not in allowed_hosts:
        raise ValueError(f"HTTPS redirect or destination host is not permitted: {host}")

    if require_public_destination:
        try:
            results = socket.getaddrinfo(host, 443, type=socket.SOCK_STREAM)
        except socket.gaierror as exc:
            raise ValueError(f"Image host could not be resolved: {host}") from exc
        addresses = {item[4][0].split("%", 1)[0] for item in results}
        if not addresses:
            raise ValueError(f"Image host resolved to no addresses: {host}")
        for address in addresses:
            try:
                ip = ipaddress.ip_address(address)
            except ValueError as exc:
                raise ValueError(
                    f"Image host returned an invalid address: {address}"
                ) from exc
            if not ip.is_global:
                raise ValueError(
                    f"Image URL resolves to a non-public address and was rejected: {address}"
                )
    return parsed


class _ValidatedRedirectHandler(urllib.request.HTTPRedirectHandler):
    def __init__(
        self,
        *,
        allowed_hosts: set[str] | None,
        require_public_destination: bool,
    ) -> None:
        super().__init__()
        self._allowed_hosts = allowed_hosts
        self._require_public_destination = require_public_destination

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
        _validate_https_url(
            absolute_url,
            allowed_hosts=self._allowed_hosts,
            require_public_destination=self._require_public_destination,
        )
        return super().redirect_request(req, fp, code, msg, headers, absolute_url)


def _read_response_bytes(response: Any, *, max_bytes: int, label: str) -> bytes:
    raw_length = response.headers.get("Content-Length")
    if raw_length is not None:
        try:
            declared_length = int(raw_length)
        except ValueError as exc:
            raise ValueError(f"{label} has an invalid Content-Length") from exc
        if declared_length < 0 or declared_length > max_bytes:
            raise ValueError(f"{label} exceeds the {max_bytes}-byte limit")
    body = response.read(max_bytes + 1)
    if len(body) > max_bytes:
        raise ValueError(f"{label} exceeds the {max_bytes}-byte limit")
    return body


def _open_validated_url(
    request: urllib.request.Request,
    *,
    timeout_s: float,
    allowed_hosts: set[str] | None,
    require_public_destination: bool,
) -> Any:
    if (
        not math.isfinite(timeout_s)
        or timeout_s <= 0.0
        or timeout_s > MAX_SLEEP_SECONDS
    ):
        raise ValueError(
            f"HTTP timeout must be finite and in (0, {MAX_SLEEP_SECONDS:g}]"
        )
    _validate_https_url(
        request.full_url,
        allowed_hosts=allowed_hosts,
        require_public_destination=require_public_destination,
    )
    opener = urllib.request.build_opener(
        _ValidatedRedirectHandler(
            allowed_hosts=allowed_hosts,
            require_public_destination=require_public_destination,
        )
    )
    response = opener.open(request, timeout=timeout_s)
    try:
        _validate_https_url(
            response.geturl(),
            allowed_hosts=allowed_hosts,
            require_public_destination=require_public_destination,
        )
    except BaseException:
        response.close()
        raise
    return response


def _http_json_post(
    url: str,
    headers: dict[str, str],
    payload: dict[str, Any],
    timeout_s: float = 180,
) -> dict[str, Any]:
    data = json.dumps(payload, allow_nan=False).encode("utf-8")
    if len(data) > MAX_HTTP_JSON_REQUEST_BYTES:
        raise ValueError(
            f"HTTP JSON request exceeds the {MAX_HTTP_JSON_REQUEST_BYTES}-byte limit"
        )
    parsed = _validate_https_url(url)
    allowed_hosts = {(parsed.hostname or "").lower().rstrip(".")}
    req = urllib.request.Request(
        url,
        data=data,
        headers={
            **headers,
            "Accept": "application/json",
            "Accept-Encoding": "identity",
            "Content-Type": "application/json",
            "User-Agent": "prisoma-ui-designer/0.9",
        },
        method="POST",
    )
    with _open_validated_url(
        req,
        timeout_s=timeout_s,
        allowed_hosts=allowed_hosts,
        require_public_destination=False,
    ) as resp:
        content_type = (resp.headers.get("Content-Type") or "").split(";", 1)[0]
        if content_type.lower() != "application/json":
            raise ValueError(f"HTTP response is not JSON: {content_type or 'missing'}")
        body = _read_response_bytes(
            resp, max_bytes=MAX_HTTP_JSON_RESPONSE_BYTES, label="HTTP JSON response"
        )
    return _decode_json_object(body, label="HTTP JSON response")


def _http_get_image(url: str, timeout_s: float = 180) -> bytes:
    req = urllib.request.Request(
        url,
        headers={
            "Accept": "image/png",
            "Accept-Encoding": "identity",
            "User-Agent": "prisoma-ui-designer/0.9",
        },
        method="GET",
    )
    with _open_validated_url(
        req,
        timeout_s=timeout_s,
        allowed_hosts=None,
        require_public_destination=True,
    ) as resp:
        content_type = (resp.headers.get("Content-Type") or "").split(";", 1)[0]
        if content_type.lower() != "image/png":
            raise ValueError(
                f"FAL image response is not PNG: {content_type or 'missing'}"
            )
        body = _read_response_bytes(
            resp, max_bytes=MAX_IMAGE_BYTES, label="FAL image response"
        )
    if not body.startswith(PNG_SIGNATURE):
        raise ValueError("FAL image response does not have a PNG signature")
    return body


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


def _checked_int(value: Any, *, label: str, minimum: int, maximum: int) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise ValueError(f"{label} must be an integer")
    if not minimum <= value <= maximum:
        raise ValueError(f"{label} must be in [{minimum}, {maximum}]")
    return value


def _checked_number(value: Any, *, label: str, minimum: float, maximum: float) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"{label} must be a number")
    result = float(value)
    if not math.isfinite(result) or not minimum <= result <= maximum:
        raise ValueError(f"{label} must be finite and in [{minimum}, {maximum}]")
    return result


def load_ui_parts(ui_md_path: Path) -> list[UiPart]:
    raw_text = _read_regular_file(
        ui_md_path, max_bytes=MAX_UI_MD_BYTES, label="UI markdown"
    )
    try:
        text = raw_text.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise ValueError("UI markdown is not valid UTF-8") from exc
    blocks = re.findall(r"```json\s*(\{.*?\})\s*```", text, flags=re.S)
    if text.count("```json") != len(blocks):
        raise ValueError("UI markdown contains an incomplete or non-object JSON fence")
    if len(blocks) > MAX_UI_PARTS:
        raise ValueError(f"UI markdown exceeds the {MAX_UI_PARTS}-part limit")

    allowed_keys = {
        "type",
        "id",
        "title",
        "milestone",
        "requirements",
        "prompt_seed",
        "negative_prompt",
        "image",
        "score_threshold",
        "max_iterations",
        "allow_img2img",
    }
    parts: list[UiPart] = []
    seen_ids: set[str] = set()
    for index, raw in enumerate(blocks, start=1):
        obj = _decode_json_object(raw, label=f"UI JSON block {index}")
        if obj.get("type") != "ui_part":
            continue
        if set(obj) != allowed_keys:
            missing = sorted(allowed_keys - set(obj))
            extra = sorted(set(obj) - allowed_keys)
            raise ValueError(
                f"UI part {index} has an invalid schema; missing={missing}, extra={extra}"
            )

        part_id = _checked_text(obj["id"], label=f"UI part {index} id", max_chars=64)
        if UI_PART_ID_RE.fullmatch(part_id) is None:
            raise ValueError(
                f"UI part {index} id must be one safe lowercase path segment"
            )
        if part_id in seen_ids:
            raise ValueError(f"Duplicate UI part id: {part_id}")
        seen_ids.add(part_id)

        requirements_obj = obj["requirements"]
        if not isinstance(requirements_obj, list) or not requirements_obj:
            raise ValueError(f"UI part {part_id} requirements must be a nonempty list")
        if len(requirements_obj) > MAX_REQUIREMENTS:
            raise ValueError(
                f"UI part {part_id} exceeds the {MAX_REQUIREMENTS}-requirement limit"
            )
        requirements = [
            _checked_text(
                item,
                label=f"UI part {part_id} requirement {item_index}",
                max_chars=MAX_SHORT_TEXT_CHARS,
            )
            for item_index, item in enumerate(requirements_obj, start=1)
        ]

        image = obj["image"]
        if not isinstance(image, dict) or set(image) != {"width", "height"}:
            raise ValueError(f"UI part {part_id} image must contain width and height")
        width = _checked_int(
            image["width"],
            label=f"UI part {part_id} image width",
            minimum=MIN_IMAGE_SIDE,
            maximum=MAX_IMAGE_SIDE,
        )
        height = _checked_int(
            image["height"],
            label=f"UI part {part_id} image height",
            minimum=MIN_IMAGE_SIDE,
            maximum=MAX_IMAGE_SIDE,
        )
        if width * height > MAX_IMAGE_PIXELS:
            raise ValueError(
                f"UI part {part_id} image exceeds the {MAX_IMAGE_PIXELS}-pixel limit"
            )

        negative_obj = obj["negative_prompt"]
        if negative_obj is None:
            negative_prompt = None
        else:
            negative_prompt = _checked_text(
                negative_obj,
                label=f"UI part {part_id} negative_prompt",
                max_chars=MAX_PROMPT_CHARS,
            )
        allow_img2img = obj["allow_img2img"]
        if not isinstance(allow_img2img, bool):
            raise ValueError(f"UI part {part_id} allow_img2img must be a boolean")

        parts.append(
            UiPart(
                part_id=part_id,
                title=_checked_text(
                    obj["title"],
                    label=f"UI part {part_id} title",
                    max_chars=MAX_SHORT_TEXT_CHARS,
                ),
                milestone=_checked_text(
                    obj["milestone"],
                    label=f"UI part {part_id} milestone",
                    max_chars=MAX_SHORT_TEXT_CHARS,
                    allow_empty=True,
                ),
                requirements=requirements,
                prompt_seed=_checked_text(
                    obj["prompt_seed"],
                    label=f"UI part {part_id} prompt_seed",
                    max_chars=MAX_PROMPT_CHARS,
                ),
                negative_prompt=negative_prompt,
                image_width=width,
                image_height=height,
                score_threshold=_checked_number(
                    obj["score_threshold"],
                    label=f"UI part {part_id} score_threshold",
                    minimum=0.0,
                    maximum=10.0,
                ),
                max_iterations=_checked_int(
                    obj["max_iterations"],
                    label=f"UI part {part_id} max_iterations",
                    minimum=1,
                    maximum=MAX_ITERATIONS,
                ),
                allow_img2img=allow_img2img,
            )
        )
    if not parts:
        raise ValueError(f"No ui_part JSON blocks found in {ui_md_path}")
    return parts


def _validate_ui_part(part: UiPart) -> None:
    if not isinstance(part, UiPart):
        raise ValueError("UI part must be a UiPart value")
    if UI_PART_ID_RE.fullmatch(part.part_id) is None:
        raise ValueError("UI part id must be one safe lowercase path segment")
    _checked_text(part.title, label="UI part title", max_chars=MAX_SHORT_TEXT_CHARS)
    _checked_text(
        part.milestone,
        label="UI part milestone",
        max_chars=MAX_SHORT_TEXT_CHARS,
        allow_empty=True,
    )
    if not isinstance(part.requirements, list) or not part.requirements:
        raise ValueError("UI part requirements must be a nonempty list")
    if len(part.requirements) > MAX_REQUIREMENTS:
        raise ValueError(f"UI part exceeds the {MAX_REQUIREMENTS}-requirement limit")
    for index, requirement in enumerate(part.requirements, start=1):
        _checked_text(
            requirement,
            label=f"UI part requirement {index}",
            max_chars=MAX_SHORT_TEXT_CHARS,
        )
    _checked_text(
        part.prompt_seed, label="UI part prompt seed", max_chars=MAX_PROMPT_CHARS
    )
    if part.negative_prompt is not None:
        _checked_text(
            part.negative_prompt,
            label="UI part negative prompt",
            max_chars=MAX_PROMPT_CHARS,
        )
    width = _checked_int(
        part.image_width,
        label="UI part image width",
        minimum=MIN_IMAGE_SIDE,
        maximum=MAX_IMAGE_SIDE,
    )
    height = _checked_int(
        part.image_height,
        label="UI part image height",
        minimum=MIN_IMAGE_SIDE,
        maximum=MAX_IMAGE_SIDE,
    )
    if width * height > MAX_IMAGE_PIXELS:
        raise ValueError(f"UI part image exceeds the {MAX_IMAGE_PIXELS}-pixel limit")
    _checked_number(
        part.score_threshold,
        label="UI part score threshold",
        minimum=0.0,
        maximum=10.0,
    )
    _checked_int(
        part.max_iterations,
        label="UI part max iterations",
        minimum=1,
        maximum=MAX_ITERATIONS,
    )
    if not isinstance(part.allow_img2img, bool):
        raise ValueError("UI part allow_img2img must be a boolean")


class FalGptImageClient:
    """
    Minimal FAL client using HTTPS + JSON.

    NOTE: FAL model endpoints and response shapes can change; treat this as a template.
    Verify the endpoint name and the output JSON schema in FAL docs for your account.
    """

    def __init__(self, *, fal_key: str, endpoint: str) -> None:
        if len(fal_key) > 16_384 or any(char in fal_key for char in "\r\n\x00"):
            raise ValueError("FAL key is invalid or exceeds the 16384-character limit")
        if FAL_ENDPOINT_RE.fullmatch(endpoint) is None:
            raise ValueError(
                "FAL endpoint must contain 2-4 safe slash-separated path segments"
            )
        self._fal_key = fal_key
        self._endpoint = endpoint

    def generate(
        self,
        *,
        prompt: str,
        negative_prompt: str | None,
        width: int,
        height: int,
        init_image_path: Path | None = None,
        mode: Literal["text2img", "img2img"] = "text2img",
    ) -> tuple[bytes, dict[str, Any]]:
        if not self._fal_key:
            raise RuntimeError("A FAL key is required for image generation")
        _checked_text(prompt, label="FAL prompt", max_chars=MAX_PROMPT_CHARS)
        if negative_prompt is not None:
            _checked_text(
                negative_prompt,
                label="FAL negative prompt",
                max_chars=MAX_PROMPT_CHARS,
            )
        checked_width = _checked_int(
            width,
            label="FAL image width",
            minimum=MIN_IMAGE_SIDE,
            maximum=MAX_IMAGE_SIDE,
        )
        checked_height = _checked_int(
            height,
            label="FAL image height",
            minimum=MIN_IMAGE_SIDE,
            maximum=MAX_IMAGE_SIDE,
        )
        if checked_width * checked_height > MAX_IMAGE_PIXELS:
            raise ValueError(f"FAL image exceeds the {MAX_IMAGE_PIXELS}-pixel limit")
        if mode not in ("text2img", "img2img"):
            raise ValueError(f"Unsupported FAL generation mode: {mode!r}")
        if mode == "img2img" and init_image_path is None:
            raise ValueError("img2img mode requires an initial image")
        if mode == "text2img" and init_image_path is not None:
            raise ValueError("text2img mode must not include an initial image")

        url = f"https://fal.run/{self._endpoint}"
        headers = {"Authorization": f"Key {self._fal_key}"}

        payload: dict[str, Any] = {
            "prompt": prompt,
            "image_size": {"width": checked_width, "height": checked_height},
        }
        if negative_prompt:
            payload["negative_prompt"] = negative_prompt

        # Image-to-image support is endpoint-specific. Common patterns are:
        #  - "image_url": "https://..."  (requires hosting the image)
        #  - "image": "data:image/png;base64,..." (inline data URI)
        if mode == "img2img" and init_image_path is not None:
            init_bytes = _read_regular_file(
                init_image_path,
                max_bytes=MAX_INIT_IMAGE_BYTES,
                label="FAL initial image",
            )
            if not init_bytes.startswith(PNG_SIGNATURE):
                raise ValueError("FAL initial image does not have a PNG signature")
            payload["image"] = "data:image/png;base64," + base64.b64encode(
                init_bytes
            ).decode("ascii")

        result = _http_json_post(url, headers=headers, payload=payload)

        # Try to locate an image URL in common shapes.
        image_url: Any = None
        if isinstance(result.get("images"), list) and result["images"]:
            first_image = result["images"][0]
            if not isinstance(first_image, dict):
                raise ValueError("FAL images[0] must be an object")
            image_url = first_image.get("url") or first_image.get("image_url")
        if image_url is None and isinstance(result.get("image"), dict):
            image_url = result["image"].get("url") or result["image"].get("image_url")
        if image_url is None:
            raise RuntimeError(
                f"Could not find image URL in FAL response keys={list(result.keys())}"
            )
        if not isinstance(image_url, str) or not image_url.strip():
            raise ValueError("FAL image URL must be a nonempty string")

        image_bytes = _http_get_image(image_url)
        return image_bytes, result


def _run_command_bounded(
    command: list[str], *, timeout_s: float, max_output_bytes: int
) -> tuple[int, bytes]:
    if (
        not command
        or not all(
            isinstance(item, str) and item and "\x00" not in item for item in command
        )
        or not isinstance(timeout_s, (int, float))
        or isinstance(timeout_s, bool)
        or not math.isfinite(timeout_s)
        or timeout_s <= 0
        or timeout_s > MAX_SLEEP_SECONDS
        or isinstance(max_output_bytes, bool)
        or not isinstance(max_output_bytes, int)
        or max_output_bytes <= 0
    ):
        raise ValueError("Bounded subprocess parameters are invalid or outside limits")
    try:
        process = subprocess.Popen(
            command,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            close_fds=True,
        )
    except FileNotFoundError:
        raise
    assert process.stdout is not None
    output = bytearray()
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)
    deadline = time.monotonic() + timeout_s
    eof = False
    try:
        while not eof:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError(
                    f"Subprocess exceeded the {timeout_s:g}-second timeout"
                )
            events = selector.select(min(remaining, 0.25))
            for key, _ in events:
                chunk = os.read(key.fileobj.fileno(), 65_536)
                if not chunk:
                    eof = True
                    break
                room = max_output_bytes + 1 - len(output)
                output.extend(chunk[:room])
                if len(output) > max_output_bytes:
                    raise RuntimeError(
                        f"Subprocess output exceeds the {max_output_bytes}-byte limit"
                    )
        try:
            return_code = process.wait(timeout=max(0.0, deadline - time.monotonic()))
        except subprocess.TimeoutExpired as exc:
            raise TimeoutError(
                f"Subprocess exceeded the {timeout_s:g}-second timeout"
            ) from exc
    except BaseException:
        process.kill()
        process.wait()
        raise
    finally:
        selector.close()
        process.stdout.close()
    return return_code, bytes(output)


def _bounded_model_text(raw: Any, *, label: str) -> str:
    if not isinstance(raw, str):
        raise ValueError(f"{label} must be text")
    if len(raw) > MAX_MODEL_RESPONSE_CHARS:
        raise ValueError(
            f"{label} exceeds the {MAX_MODEL_RESPONSE_CHARS}-character limit"
        )
    if "\x00" in raw:
        raise ValueError(f"{label} contains a NUL character")
    return raw


class GeminiVertexClient:
    """
    Gemini client wrapper.

    Preferred path: `google-genai` with Vertex AI:
      pip install google-genai

    Fallback path (no python deps): `gcloud auth print-access-token` + REST.
    """

    def __init__(
        self, *, project: str, location: str, model_vision: str, model_text: str
    ) -> None:
        if project and GCP_PROJECT_RE.fullmatch(project) is None:
            raise ValueError("GCP project id has an invalid format")
        if GCP_LOCATION_RE.fullmatch(location) is None:
            raise ValueError("GCP location has an invalid format")
        if MODEL_NAME_RE.fullmatch(model_vision) is None:
            raise ValueError("Gemini vision model name has an invalid format")
        if MODEL_NAME_RE.fullmatch(model_text) is None:
            raise ValueError("Gemini text model name has an invalid format")
        self._project = project
        self._location = location
        self._model_vision = model_vision
        self._model_text = model_text

        self._mode: Literal["google_genai", "rest"] = "rest"
        self._genai = None
        self._types = None

        try:
            from google import genai  # type: ignore
            from google.genai import types  # type: ignore

            self._genai = genai
            self._types = types
            self._mode = "google_genai"
        except Exception:
            self._mode = "rest"

    def _google_client(self) -> Any:
        if self._mode != "google_genai":
            raise RuntimeError("google-genai client is unavailable")
        types = self._types
        return self._genai.Client(
            vertexai=True,
            project=self._project,
            location=self._location,
            http_options=types.HttpOptions(
                timeout=int(HTTP_TIMEOUT_SECONDS * 1_000),
                retry_options=types.HttpRetryOptions(
                    attempts=3, initial_delay=1.0, max_delay=4.0
                ),
            ),
        )

    def _gcloud_access_token(self) -> str:
        try:
            return_code, out = _run_command_bounded(
                ["gcloud", "auth", "print-access-token"],
                timeout_s=GCLOUD_TIMEOUT_SECONDS,
                max_output_bytes=MAX_SUBPROCESS_OUTPUT_BYTES,
            )
        except FileNotFoundError as e:
            raise RuntimeError(
                "Gemini REST fallback requires `gcloud` to be installed and authenticated, "
                "or install `google-genai` (recommended)."
            ) from e
        except TimeoutError as exc:
            raise RuntimeError("gcloud access-token command timed out") from exc
        if return_code != 0:
            detail = out.decode("utf-8", errors="replace").strip()
            raise RuntimeError(
                f"gcloud access-token command failed with status {return_code}: {detail}"
            )
        try:
            token = out.decode("utf-8").strip()
        except UnicodeDecodeError as exc:
            raise RuntimeError("gcloud returned a non-UTF-8 access token") from exc
        if (
            not token
            or len(token) > 16_384
            or any(char.isspace() or ord(char) < 0x21 for char in token)
        ):
            raise RuntimeError(
                "Invalid access token from gcloud; authenticate again before retrying."
            )
        return token

    def critique_ui(self, *, part: UiPart, image_bytes: bytes) -> Review:
        _validate_ui_part(part)
        if not self._project:
            raise RuntimeError("A GCP project id is required for Gemini requests")
        if len(image_bytes) > MAX_IMAGE_BYTES or not image_bytes.startswith(
            PNG_SIGNATURE
        ):
            raise ValueError("Gemini input must be a bounded PNG image")
        prompt = (
            "You are a strict UI/UX reviewer for a scientific desktop app.\n"
            "Evaluate the provided UI mockup image against the requirements list.\n"
            "Return ONLY valid JSON with keys:\n"
            "  score (0-10 number), pass (boolean), issues (string[]), fixes (string[])\n"
            "Scoring rule: 10 only if every requirement is clearly satisfied with legible labels.\n"
            "Requirements:\n" + "\n".join([f"- {r}" for r in part.requirements])
        )

        if self._mode == "google_genai":
            client = self._google_client()
            t = self._types
            contents = [
                t.Content(
                    role="user",
                    parts=[
                        t.Part.from_text(text=prompt),
                        t.Part.from_bytes(data=image_bytes, mime_type="image/png"),
                    ],
                )
            ]
            resp = client.models.generate_content(
                model=self._model_vision,
                contents=contents,
                config=t.GenerateContentConfig(
                    temperature=0.2,
                    max_output_tokens=1024,
                    response_mime_type="application/json",
                ),
            )
            raw = resp.text or ""
        else:
            raw = self._critique_via_rest(
                prompt=prompt, image_bytes=image_bytes, model=self._model_vision
            )

        raw = _bounded_model_text(raw, label="Gemini critique response")
        obj = _parse_json_from_model(raw)
        if set(obj) != {"score", "pass", "issues", "fixes"}:
            raise ValueError("Gemini critique response has an invalid schema")
        pass_value = obj["pass"]
        if not isinstance(pass_value, bool):
            raise ValueError("Gemini critique pass must be a boolean")
        score = _checked_number(
            obj["score"], label="Gemini critique score", minimum=0.0, maximum=10.0
        )
        threshold = _checked_number(
            part.score_threshold,
            label="UI part score threshold",
            minimum=0.0,
            maximum=10.0,
        )
        if pass_value and score < threshold:
            raise ValueError(
                "Gemini critique pass contradicts the configured score threshold"
            )
        return Review(
            score=score,
            pass_=pass_value,
            issues=_checked_model_string_list(obj["issues"], label="Gemini issues"),
            fixes=_checked_model_string_list(obj["fixes"], label="Gemini fixes"),
            raw_text=raw,
        )

    def improve_prompt(
        self,
        *,
        part: UiPart,
        previous_prompt: str,
        previous_negative: str | None,
        review: Review,
    ) -> tuple[str, str | None]:
        _validate_ui_part(part)
        if not isinstance(review, Review):
            raise ValueError("Previous review must be a Review value")
        if not self._project:
            raise RuntimeError("A GCP project id is required for Gemini requests")
        _checked_text(
            previous_prompt,
            label="Previous Gemini prompt",
            max_chars=MAX_PROMPT_CHARS,
        )
        if previous_negative is not None:
            _checked_text(
                previous_negative,
                label="Previous Gemini negative prompt",
                max_chars=MAX_PROMPT_CHARS,
            )
        _checked_number(
            review.score,
            label="Previous review score",
            minimum=0.0,
            maximum=10.0,
        )
        if not isinstance(review.pass_, bool):
            raise ValueError("Previous review pass must be a boolean")
        _checked_model_string_list(review.issues, label="Previous review issues")
        _checked_model_string_list(review.fixes, label="Previous review fixes")
        prompt = (
            "You are a prompt engineer for a UI mockup generator.\n"
            "Rewrite the prompt to fix the issues while staying faithful to the requirements.\n"
            "Return ONLY valid JSON with keys: prompt (string), negative_prompt (string, optional).\n\n"
            f"UI part: {part.title}\n"
            "Requirements:\n"
            + "\n".join([f"- {r}" for r in part.requirements])
            + "\n\nPrevious prompt:\n"
            + previous_prompt
            + (
                "\n\nPrevious negative prompt:\n" + previous_negative
                if previous_negative
                else ""
            )
            + "\n\nCritique issues:\n"
            + "\n".join([f"- {x}" for x in review.issues])
            + "\n\nConcrete fixes to implement:\n"
            + "\n".join([f"- {x}" for x in review.fixes])
        )

        if self._mode == "google_genai":
            client = self._google_client()
            t = self._types
            contents = [t.Content(role="user", parts=[t.Part.from_text(text=prompt)])]
            resp = client.models.generate_content(
                model=self._model_text,
                contents=contents,
                config=t.GenerateContentConfig(
                    temperature=0.4,
                    max_output_tokens=1024,
                    response_mime_type="application/json",
                ),
            )
            raw = resp.text or ""
        else:
            raw = self._generate_text_via_rest(prompt=prompt, model=self._model_text)

        raw = _bounded_model_text(raw, label="Gemini prompt response")
        obj = _parse_json_from_model(raw)
        if not {"prompt"} <= set(obj) <= {"prompt", "negative_prompt"}:
            raise ValueError("Gemini prompt response has an invalid schema")
        new_prompt = _checked_text(
            obj["prompt"], label="Gemini revised prompt", max_chars=MAX_PROMPT_CHARS
        ).strip()
        new_negative = obj.get("negative_prompt")
        if new_negative is not None:
            new_negative = _checked_text(
                new_negative,
                label="Gemini revised negative prompt",
                max_chars=MAX_PROMPT_CHARS,
            ).strip()
        return new_prompt, new_negative

    def _critique_via_rest(self, *, prompt: str, image_bytes: bytes, model: str) -> str:
        token = self._gcloud_access_token()
        url = (
            f"https://{self._location}-aiplatform.googleapis.com/v1/projects/{self._project}"
            f"/locations/{self._location}/publishers/google/models/{model}:generateContent"
        )
        headers = {"Authorization": f"Bearer {token}"}
        payload = {
            "contents": [
                {
                    "role": "user",
                    "parts": [
                        {"text": prompt},
                        {
                            "inlineData": {
                                "mimeType": "image/png",
                                "data": base64.b64encode(image_bytes).decode("ascii"),
                            }
                        },
                    ],
                }
            ],
            "generationConfig": {
                "temperature": 0.2,
                "maxOutputTokens": 1024,
                "responseMimeType": "application/json",
            },
        }
        result = _http_json_post(url, headers=headers, payload=payload)
        return _vertex_extract_text(result)

    def _generate_text_via_rest(self, *, prompt: str, model: str) -> str:
        token = self._gcloud_access_token()
        url = (
            f"https://{self._location}-aiplatform.googleapis.com/v1/projects/{self._project}"
            f"/locations/{self._location}/publishers/google/models/{model}:generateContent"
        )
        headers = {"Authorization": f"Bearer {token}"}
        payload = {
            "contents": [{"role": "user", "parts": [{"text": prompt}]}],
            "generationConfig": {
                "temperature": 0.4,
                "maxOutputTokens": 1024,
                "responseMimeType": "application/json",
            },
        }
        result = _http_json_post(url, headers=headers, payload=payload)
        return _vertex_extract_text(result)


def _checked_model_string_list(value: Any, *, label: str) -> list[str]:
    if not isinstance(value, list):
        raise ValueError(f"{label} must be a list")
    if len(value) > MAX_REQUIREMENTS:
        raise ValueError(f"{label} exceeds the {MAX_REQUIREMENTS}-item limit")
    return [
        _checked_text(
            item,
            label=f"{label} item {index}",
            max_chars=MAX_SHORT_TEXT_CHARS,
        )
        for index, item in enumerate(value, start=1)
    ]


def _vertex_extract_text(resp: dict[str, Any]) -> str:
    # Vertex response is typically: {candidates: [{content:{parts:[{text:"..."}]}}]}
    candidates = resp.get("candidates")
    if not isinstance(candidates, list) or not candidates:
        raise ValueError("Vertex response must contain at least one candidate")
    first = candidates[0]
    if not isinstance(first, dict) or not isinstance(first.get("content"), dict):
        raise ValueError("Vertex candidate content must be an object")
    parts = first["content"].get("parts")
    if not isinstance(parts, list) or not 1 <= len(parts) <= 64:
        raise ValueError("Vertex candidate parts must be a bounded nonempty list")
    texts: list[str] = []
    for index, part in enumerate(parts, start=1):
        if not isinstance(part, dict) or set(part) != {"text"}:
            raise ValueError(f"Vertex text part {index} has an invalid schema")
        texts.append(
            _checked_text(
                part["text"],
                label=f"Vertex text part {index}",
                max_chars=MAX_MODEL_RESPONSE_CHARS,
            )
        )
    return _bounded_model_text("\n".join(texts), label="Vertex text response")


def _parse_json_from_model(raw: str) -> dict[str, Any]:
    s = _bounded_model_text(raw, label="Model response").strip()
    if not s:
        raise ValueError("Model returned an empty response")
    if s.startswith("```"):
        fenced = re.fullmatch(r"```(?:json)?[ \t]*\r?\n(.*?)\r?\n```", s, flags=re.S)
        if fenced is None:
            raise ValueError("Model response has a malformed JSON code fence")
        s = fenced.group(1).strip()
    return _decode_json_object(s, label="Model response")


def _create_contained_directory(base: Path, segment: str) -> Path:
    if SAFE_OUTPUT_SEGMENT_RE.fullmatch(segment) is None or segment in {".", ".."}:
        raise ValueError("Output directory name must be one safe path segment")
    base.mkdir(parents=True, exist_ok=True)
    if base.is_symlink() or not base.is_dir():
        raise ValueError(
            f"Output base must be a regular, non-symlink directory: {base}"
        )
    resolved_base = base.resolve(strict=True)
    candidate = base / segment
    if candidate.exists() or candidate.is_symlink():
        raise FileExistsError(f"Refusing to reuse output directory: {candidate}")
    candidate.mkdir()
    resolved_candidate = candidate.resolve(strict=True)
    if resolved_candidate.parent != resolved_base:
        candidate.rmdir()
        raise ValueError("Output directory escaped its configured base")
    return candidate


def optimize_part(
    *,
    part: UiPart,
    fal: FalGptImageClient,
    gemini: GeminiVertexClient,
    out_dir: Path,
    dry_run: bool,
    sleep_s: float,
) -> None:
    _validate_ui_part(part)
    sleep_s = _checked_number(
        sleep_s,
        label="Iteration sleep",
        minimum=0.0,
        maximum=MAX_SLEEP_SECONDS,
    )
    part_dir = _create_contained_directory(out_dir, part.part_id)

    prompt = part.prompt_seed.strip()
    negative_prompt = part.negative_prompt
    best: tuple[float, Path, str, str | None] | None = (
        None  # score, image_path, prompt, negative_prompt
    )

    for i in range(1, part.max_iterations + 1):
        iter_tag = f"iter_{i:02d}"
        mode: Literal["text2img", "img2img"] = "text2img"
        init_path: Path | None = None
        if part.allow_img2img and best is not None:
            mode = "img2img"
            init_path = best[1]

        prompt_path = part_dir / f"{iter_tag}.prompt.txt"
        header = f"# {part.title}\n# prompt\n"
        if negative_prompt:
            header += "# negative_prompt\n" + negative_prompt + "\n"
        _write_text(prompt_path, header + prompt + "\n")

        if dry_run:
            print(
                f"[DRY RUN] {part.part_id} {iter_tag}: would call FAL ({mode}) + Gemini. Prompt saved to {prompt_path}"
            )
            break

        try:
            image_bytes, fal_meta = fal.generate(
                prompt=prompt,
                negative_prompt=negative_prompt,
                width=part.image_width,
                height=part.image_height,
                init_image_path=init_path,
                mode=mode,
            )
        except urllib.error.HTTPError as e:
            detail = _read_response_bytes(
                e, max_bytes=64 * 1024, label="FAL error response"
            ).decode("utf-8", "replace")
            raise RuntimeError(
                f"FAL request failed with HTTP {e.code}: {detail}"
            ) from e

        image_path = part_dir / f"{iter_tag}.png"
        _atomic_write_bytes(image_path, image_bytes, max_bytes=MAX_IMAGE_BYTES)
        _write_json(part_dir / f"{iter_tag}.fal.json", fal_meta)

        review = gemini.critique_ui(part=part, image_bytes=image_bytes)
        _write_json(
            part_dir / f"{iter_tag}.review.json",
            {
                "score": review.score,
                "pass": review.pass_,
                "issues": review.issues,
                "fixes": review.fixes,
                "raw_text": review.raw_text,
            },
        )

        if best is None or review.score > best[0]:
            best = (review.score, image_path, prompt, negative_prompt)
            _atomic_write_bytes(
                part_dir / "best.png", image_bytes, max_bytes=MAX_IMAGE_BYTES
            )
            best_header = f"# {part.title}\n# prompt\n"
            if negative_prompt:
                best_header += "# negative_prompt\n" + negative_prompt + "\n"
            _write_text(part_dir / "best.prompt.txt", best_header + prompt + "\n")
            _write_json(
                part_dir / "best.review.json",
                {
                    "score": review.score,
                    "pass": review.pass_,
                    "issues": review.issues,
                    "fixes": review.fixes,
                },
            )

        print(
            f"{part.part_id} {iter_tag}: score={review.score:.1f} pass={review.pass_}"
        )
        if review.pass_ or review.score >= part.score_threshold:
            break

        prompt, new_negative = gemini.improve_prompt(
            part=part,
            previous_prompt=prompt,
            previous_negative=negative_prompt,
            review=review,
        )
        if new_negative is not None:
            negative_prompt = new_negative

        if sleep_s > 0:
            time.sleep(sleep_s)


def main(argv: list[str] | None = None) -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--ui-md", type=Path, default=Path("uidesigner/UI.md"))
    ap.add_argument("--out", type=Path, default=Path("uidesigner/out"))
    ap.add_argument(
        "--only",
        type=str,
        default="",
        help="Comma-separated ui_part ids to run (default: all)",
    )
    ap.add_argument("--dry-run", action="store_true")

    ap.add_argument(
        "--fal-endpoint",
        type=str,
        default=os.environ.get("FAL_ENDPOINT", "fal-ai/gpt-image-1.5"),
    )
    ap.add_argument("--fal-key-env", type=str, default="FAL_KEY")

    ap.add_argument(
        "--gcp-project", type=str, default=os.environ.get("GOOGLE_CLOUD_PROJECT", "")
    )
    ap.add_argument(
        "--gcp-location",
        type=str,
        default=os.environ.get("GOOGLE_CLOUD_LOCATION", "us-central1"),
    )
    ap.add_argument(
        "--gemini-vision-model",
        type=str,
        default=os.environ.get("GEMINI_VISION_MODEL", "gemini-1.5-pro"),
    )
    ap.add_argument(
        "--gemini-text-model",
        type=str,
        default=os.environ.get("GEMINI_TEXT_MODEL", "gemini-1.5-pro"),
    )

    ap.add_argument("--sleep", type=float, default=1.0)
    args = ap.parse_args(argv)

    try:
        parts = load_ui_parts(args.ui_md)
    except (OSError, ValueError) as exc:
        ap.error(str(exc))
    only_items = [x.strip() for x in args.only.split(",") if x.strip()]
    if len(only_items) > MAX_UI_PARTS:
        ap.error(f"--only exceeds the {MAX_UI_PARTS}-id limit")
    if any(UI_PART_ID_RE.fullmatch(item) is None for item in only_items):
        ap.error("--only values must be safe lowercase UI-part ids")
    if len(set(only_items)) != len(only_items):
        ap.error("--only contains a duplicate UI-part id")
    only = set(only_items)
    if only:
        parts = [p for p in parts if p.part_id in only]
        matched = {part.part_id for part in parts}
        missing = sorted(only - matched)
        if missing:
            ap.error(f"--only contains unknown ids: {missing}")

    if ENV_NAME_RE.fullmatch(args.fal_key_env) is None:
        ap.error("--fal-key-env must be a valid environment-variable name")
    fal_key = os.environ.get(args.fal_key_env, "")
    if not args.dry_run and not fal_key:
        ap.error(f"Missing FAL key env var: {args.fal_key_env}")
    if not args.dry_run and not args.gcp_project:
        ap.error("Missing GOOGLE_CLOUD_PROJECT (or pass --gcp-project).")

    try:
        checked_sleep = _checked_number(
            args.sleep,
            label="--sleep",
            minimum=0.0,
            maximum=MAX_SLEEP_SECONDS,
        )
        fal = FalGptImageClient(fal_key=fal_key, endpoint=args.fal_endpoint)
        gemini = GeminiVertexClient(
            project=args.gcp_project,
            location=args.gcp_location,
            model_vision=args.gemini_vision_model,
            model_text=args.gemini_text_model,
        )
        session_dir = _create_contained_directory(args.out, _now_stamp())
    except (OSError, ValueError) as exc:
        ap.error(str(exc))

    _write_json(
        session_dir / "session.json",
        {
            "ui_md": str(args.ui_md),
            "fal_endpoint": args.fal_endpoint,
            "gcp_project": args.gcp_project,
            "gcp_location": args.gcp_location,
            "gemini_vision_model": args.gemini_vision_model,
            "gemini_text_model": args.gemini_text_model,
            "dry_run": bool(args.dry_run),
        },
    )

    for part in parts:
        optimize_part(
            part=part,
            fal=fal,
            gemini=gemini,
            out_dir=session_dir,
            dry_run=args.dry_run,
            sleep_s=checked_sleep,
        )


if __name__ == "__main__":
    main()
