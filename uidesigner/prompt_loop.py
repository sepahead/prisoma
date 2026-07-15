#!/usr/bin/env python3
"""
UI prompt optimizer (Docset v10.0)

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
"""

from __future__ import annotations

import argparse
import base64
import dataclasses
import datetime as dt
import json
import os
import re
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any, Literal


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


def _write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def _write_json(path: Path, obj: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def _http_json_post(
    url: str, headers: dict[str, str], payload: dict[str, Any], timeout_s: int = 180
) -> dict[str, Any]:
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        url,
        data=data,
        headers={**headers, "Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=timeout_s) as resp:
        body = resp.read()
    return json.loads(body.decode("utf-8"))


def _http_get_bytes(url: str, timeout_s: int = 180) -> bytes:
    with urllib.request.urlopen(url, timeout=timeout_s) as resp:
        return resp.read()


def load_ui_parts(ui_md_path: Path) -> list[UiPart]:
    text = ui_md_path.read_text(encoding="utf-8")
    blocks = re.findall(r"```json\s*(\{.*?\})\s*```", text, flags=re.S)
    parts: list[UiPart] = []
    for raw in blocks:
        try:
            obj = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if obj.get("type") != "ui_part":
            continue

        image = obj.get("image") or {}
        parts.append(
            UiPart(
                part_id=str(obj["id"]),
                title=str(obj.get("title") or obj["id"]),
                milestone=str(obj.get("milestone") or ""),
                requirements=[str(x) for x in obj.get("requirements") or []],
                prompt_seed=str(obj.get("prompt_seed") or ""),
                negative_prompt=(
                    str(obj["negative_prompt"]) if obj.get("negative_prompt") else None
                ),
                image_width=int(image.get("width") or 1536),
                image_height=int(image.get("height") or 1024),
                score_threshold=float(obj.get("score_threshold") or 9.0),
                max_iterations=int(obj.get("max_iterations") or 8),
                allow_img2img=bool(obj.get("allow_img2img") or False),
            )
        )
    if not parts:
        raise SystemExit(f"No ui_part JSON blocks found in {ui_md_path}")
    return parts


class FalGptImageClient:
    """
    Minimal FAL client using HTTPS + JSON.

    NOTE: FAL model endpoints and response shapes can change; treat this as a template.
    Verify the endpoint name and the output JSON schema in FAL docs for your account.
    """

    def __init__(self, *, fal_key: str, endpoint: str) -> None:
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
        url = f"https://fal.run/{self._endpoint}"
        headers = {"Authorization": f"Key {self._fal_key}"}

        payload: dict[str, Any] = {
            "prompt": prompt,
            "image_size": {"width": width, "height": height},
        }
        if negative_prompt:
            payload["negative_prompt"] = negative_prompt

        # Image-to-image support is endpoint-specific. Common patterns are:
        #  - "image_url": "https://..."  (requires hosting the image)
        #  - "image": "data:image/png;base64,..." (inline data URI)
        if mode == "img2img" and init_image_path is not None:
            init_bytes = init_image_path.read_bytes()
            payload["image"] = "data:image/png;base64," + base64.b64encode(
                init_bytes
            ).decode("ascii")

        result = _http_json_post(url, headers=headers, payload=payload)

        # Try to locate an image URL in common shapes.
        image_url = None
        if isinstance(result.get("images"), list) and result["images"]:
            image_url = result["images"][0].get("url") or result["images"][0].get(
                "image_url"
            )
        if image_url is None and isinstance(result.get("image"), dict):
            image_url = result["image"].get("url") or result["image"].get("image_url")
        if image_url is None:
            raise RuntimeError(
                f"Could not find image URL in FAL response keys={list(result.keys())}"
            )

        image_bytes = _http_get_bytes(str(image_url))
        return image_bytes, result


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

    def _gcloud_access_token(self) -> str:
        try:
            out = subprocess.check_output(
                ["gcloud", "auth", "print-access-token"], stderr=subprocess.STDOUT
            )
        except FileNotFoundError as e:
            raise RuntimeError(
                "Gemini REST fallback requires `gcloud` to be installed and authenticated, "
                "or install `google-genai` (recommended)."
            ) from e
        token = out.decode("utf-8").strip()
        if not token:
            raise RuntimeError(
                "Empty access token from gcloud; run `gcloud auth application-default login`."
            )
        return token

    def critique_ui(self, *, part: UiPart, image_bytes: bytes) -> Review:
        prompt = (
            "You are a strict UI/UX reviewer for a scientific desktop app.\n"
            "Evaluate the provided UI mockup image against the requirements list.\n"
            "Return ONLY valid JSON with keys:\n"
            "  score (0-10 number), pass (boolean), issues (string[]), fixes (string[])\n"
            "Scoring rule: 10 only if every requirement is clearly satisfied with legible labels.\n"
            "Requirements:\n" + "\n".join([f"- {r}" for r in part.requirements])
        )

        if self._mode == "google_genai":
            client = self._genai.Client(
                vertexai=True, project=self._project, location=self._location
            )
            t = self._types
            contents = [
                t.Content(
                    role="user",
                    parts=[
                        t.Part.from_text(prompt),
                        t.Part.from_bytes(data=image_bytes, mime_type="image/png"),
                    ],
                )
            ]
            resp = client.models.generate_content(
                model=self._model_vision, contents=contents
            )
            raw = resp.text or ""
        else:
            raw = self._critique_via_rest(
                prompt=prompt, image_bytes=image_bytes, model=self._model_vision
            )

        obj = _parse_json_from_model(raw)
        return Review(
            score=float(obj.get("score", 0.0)),
            pass_=bool(obj.get("pass", False)),
            issues=[str(x) for x in obj.get("issues") or []],
            fixes=[str(x) for x in obj.get("fixes") or []],
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
            client = self._genai.Client(
                vertexai=True, project=self._project, location=self._location
            )
            t = self._types
            contents = [t.Content(role="user", parts=[t.Part.from_text(prompt)])]
            resp = client.models.generate_content(
                model=self._model_text, contents=contents
            )
            raw = resp.text or ""
        else:
            raw = self._generate_text_via_rest(prompt=prompt, model=self._model_text)

        obj = _parse_json_from_model(raw)
        new_prompt = str(obj.get("prompt") or "").strip()
        if not new_prompt:
            raise RuntimeError(f"Gemini returned no prompt. Raw:\n{raw}")
        new_negative = obj.get("negative_prompt")
        if new_negative is not None:
            new_negative = str(new_negative).strip() or None
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
            "generationConfig": {"temperature": 0.2, "maxOutputTokens": 1024},
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
            "generationConfig": {"temperature": 0.4, "maxOutputTokens": 1024},
        }
        result = _http_json_post(url, headers=headers, payload=payload)
        return _vertex_extract_text(result)


def _vertex_extract_text(resp: dict[str, Any]) -> str:
    # Vertex response is typically: {candidates: [{content:{parts:[{text:"..."}]}}]}
    try:
        candidates = resp.get("candidates") or []
        if not candidates:
            return json.dumps(resp)
        parts = candidates[0]["content"]["parts"]
        texts = [p.get("text", "") for p in parts if isinstance(p, dict)]
        return "\n".join([t for t in texts if t])
    except Exception:
        return json.dumps(resp)


def _parse_json_from_model(raw: str) -> dict[str, Any]:
    s = raw.strip()
    if not s:
        return {}
    # strip code fences if present
    s = re.sub(r"^```json\s*", "", s)
    s = re.sub(r"^```\s*", "", s)
    s = re.sub(r"\s*```$", "", s)

    # best-effort: take first {...} block
    start = s.find("{")
    end = s.rfind("}")
    if start == -1 or end == -1 or end <= start:
        raise RuntimeError(f"Model did not return JSON. Raw:\n{raw}")
    snippet = s[start : end + 1]
    return json.loads(snippet)


def optimize_part(
    *,
    part: UiPart,
    fal: FalGptImageClient,
    gemini: GeminiVertexClient,
    out_dir: Path,
    dry_run: bool,
    sleep_s: float,
) -> None:
    part_dir = out_dir / part.part_id
    part_dir.mkdir(parents=True, exist_ok=True)

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
            raise RuntimeError(
                f"FAL request failed: {e.read().decode('utf-8', 'ignore')}"
            ) from e

        image_path = part_dir / f"{iter_tag}.png"
        image_path.write_bytes(image_bytes)
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
            (part_dir / "best.png").write_bytes(image_bytes)
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


def main() -> None:
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
    args = ap.parse_args()

    parts = load_ui_parts(args.ui_md)
    only = {x.strip() for x in args.only.split(",") if x.strip()}
    if only:
        parts = [p for p in parts if p.part_id in only]
        if not parts:
            raise SystemExit(
                f"--only matched nothing. Available: {[p.part_id for p in load_ui_parts(args.ui_md)]}"
            )

    fal_key = os.environ.get(args.fal_key_env, "")
    if not args.dry_run and not fal_key:
        raise SystemExit(f"Missing FAL key env var: {args.fal_key_env}")
    if not args.dry_run and not args.gcp_project:
        raise SystemExit("Missing GOOGLE_CLOUD_PROJECT (or pass --gcp-project).")

    fal = FalGptImageClient(fal_key=fal_key, endpoint=args.fal_endpoint)
    gemini = GeminiVertexClient(
        project=args.gcp_project,
        location=args.gcp_location,
        model_vision=args.gemini_vision_model,
        model_text=args.gemini_text_model,
    )

    session_dir = args.out / _now_stamp()
    session_dir.mkdir(parents=True, exist_ok=True)
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
            sleep_s=args.sleep,
        )


if __name__ == "__main__":
    main()
