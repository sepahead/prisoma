#!/usr/bin/env python3 -u
"""
Batch mesh generation v2 - with hunyuan3d fallback.
Uses fal-ai/gpt-image-1.5 for images, trellis primary + hunyuan3d fallback for meshes.
Skips already completed meshes from any previous run.
"""

import sys
sys.stdout.reconfigure(line_buffering=True)
sys.stderr.reconfigure(line_buffering=True)

import json
import os
import time
import requests
import threading
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed
from queue import Queue
from datetime import datetime

try:
    import fal_client
except ImportError:
    print("ERROR: pip install fal-client")
    sys.exit(1)

# Configuration
IMAGE_MODEL = "fal-ai/gpt-image-1.5"
MESH_MODELS = {
    "trellis2": {
        "id": "fal-ai/trellis-2",
        "param": "image_url",
        "extra": {"resolution": 1024, "texture_size": 2048}
    },
    "hunyuan3d": {
        "id": "fal-ai/hunyuan3d-v3/image-to-3d",
        "param": "input_image_url",
        "extra": {"enable_pbr": False, "face_count": 500000}
    },
}
PRIMARY_MESH = "trellis2"
FALLBACK_MESH = "hunyuan3d"

SCRIPT_DIR = Path(__file__).parent
OUTPUT_DIR = SCRIPT_DIR / "output_batch"
KEYS_FILE = SCRIPT_DIR / "api_keys.txt"

def load_keys():
    keys = []
    with open(KEYS_FILE) as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith('#') and not line.startswith('$'):
                if ':' in line:
                    keys.append(line)
    return keys

def enhance_prompt(prompt, category=""):
    extras = "single isolated object, centered, pure white background, professional studio lighting, ultra high resolution 8K, extremely sharp focus, photorealistic, detailed texture, clean edges"
    return f"{prompt}, {extras}"

def generate_mesh(mesh_def, api_key, results_queue):
    mesh_id = mesh_def["id"]
    category = mesh_def.get("category", "")
    prompt = enhance_prompt(mesh_def["prompt"], category)

    output_path = OUTPUT_DIR / category / mesh_id
    glb_file = output_path / f"{mesh_id}.glb"

    if glb_file.exists():
        results_queue.put(("skip", mesh_id))
        return

    os.environ["FAL_KEY"] = api_key
    print(f"[START] {mesh_id}", flush=True)

    try:
        # Generate image with gpt-image-1.5
        img_result = fal_client.subscribe(
            IMAGE_MODEL,
            arguments={
                "prompt": prompt,
                "image_size": "1024x1024",
                "num_images": 1,
                "background": "opaque",
                "quality": "high",
                "output_format": "png",
            },
        )

        images = img_result.get("images", [])
        if not images or not images[0].get("url"):
            results_queue.put(("fail", mesh_id, "No image URL"))
            return

        image_url = images[0]["url"]

        # Generate mesh - try trellis first, then hunyuan3d fallback
        mesh_model_used = None
        glb_url = None

        for model_key in [PRIMARY_MESH, FALLBACK_MESH]:
            model_cfg = MESH_MODELS[model_key]
            try:
                args = {model_cfg["param"]: image_url}
                args.update(model_cfg.get("extra", {}))
                mesh_result = fal_client.subscribe(
                    model_cfg["id"],
                    arguments=args,
                )
                glb_url = (
                    mesh_result.get("glb_url") or
                    mesh_result.get("model_mesh", {}).get("url") or
                    mesh_result.get("model_glb", {}).get("url")
                )
                if glb_url:
                    mesh_model_used = model_key
                    break
            except Exception as mesh_err:
                if model_key == PRIMARY_MESH:
                    continue
                raise mesh_err

        if not glb_url:
            results_queue.put(("fail", mesh_id, "No GLB URL from either model"))
            return

        output_path.mkdir(parents=True, exist_ok=True)

        # Save image
        img_resp = requests.get(image_url, timeout=120)
        with open(output_path / f"{mesh_id}.png", "wb") as f:
            f.write(img_resp.content)

        # Save mesh
        glb_resp = requests.get(glb_url, timeout=120)
        with open(glb_file, "wb") as f:
            f.write(glb_resp.content)

        # Save metadata
        with open(output_path / "metadata.json", "w") as f:
            json.dump({
                "id": mesh_id,
                "category": category,
                "prompt": prompt,
                "image_model": IMAGE_MODEL,
                "mesh_model": MESH_MODELS[mesh_model_used]["id"],
                "mesh_model_key": mesh_model_used,
                "timestamp": datetime.now().isoformat(),
            }, f, indent=2)

        results_queue.put(("ok", mesh_id, mesh_model_used))

    except Exception as e:
        error_msg = str(e)
        if any(x in error_msg.lower() for x in ["credit", "quota", "exceeded"]):
            results_queue.put(("exhausted", mesh_id, api_key))
        else:
            results_queue.put(("fail", mesh_id, error_msg[:100]))

def main():
    manifest_files = [
        "mesh_manifest_robotics_expanded.json",
        "mesh_manifest_combined.json",
    ]

    all_meshes = []
    seen_ids = set()

    for mf in manifest_files:
        path = SCRIPT_DIR / mf
        if path.exists():
            with open(path) as f:
                data = json.load(f)
                for m in data.get("meshes", []):
                    if m["id"] not in seen_ids:
                        all_meshes.append(m)
                        seen_ids.add(m["id"])

    print(f"Loaded {len(all_meshes)} unique mesh definitions")

    OUTPUT_DIR.mkdir(exist_ok=True)
    existing = set()
    for glb in OUTPUT_DIR.glob("**/*.glb"):
        existing.add(glb.stem)

    to_generate = [m for m in all_meshes if m["id"] not in existing]
    print(f"Already complete: {len(existing)}, remaining: {len(to_generate)}")

    if not to_generate:
        print("All meshes complete!")
        return

    keys = load_keys()
    print(f"Loaded {len(keys)} API keys")
    print(f"Using: {IMAGE_MODEL} -> {MESH_MODELS[PRIMARY_MESH]['id']} (fallback: {MESH_MODELS[FALLBACK_MESH]['id']})")

    exhausted_keys = set()
    results_queue = Queue()
    stats = {"ok": 0, "fail": 0, "skip": 0, "trellis2": 0, "hunyuan3d": 0}
    start_time = time.time()

    key_idx = [0]
    key_lock = threading.Lock()

    def get_key():
        with key_lock:
            available = [k for k in keys if k not in exhausted_keys]
            if not available:
                return None
            key = available[key_idx[0] % len(available)]
            key_idx[0] += 1
            return key

    def process_results():
        while True:
            try:
                result = results_queue.get(timeout=1)
                if result is None:
                    break

                status = result[0]
                mesh_id = result[1]

                if status == "ok":
                    stats["ok"] += 1
                    model_used = result[2] if len(result) > 2 else "unknown"
                    if model_used in stats:
                        stats[model_used] += 1
                elif status == "skip":
                    stats["skip"] += 1
                elif status == "fail":
                    stats["fail"] += 1
                    print(f"[FAIL] {mesh_id}: {result[2] if len(result) > 2 else 'unknown'}")
                elif status == "exhausted":
                    exhausted_keys.add(result[2])
                    print(f"[KEY EXHAUSTED] {len(exhausted_keys)}/{len(keys)} keys exhausted")

                total = stats["ok"] + stats["fail"]
                elapsed = time.time() - start_time
                rate = total / elapsed * 60 if elapsed > 0 else 0
                remaining = len(to_generate) - total - stats["skip"]
                eta = remaining / (rate / 60) / 60 if rate > 0 else 0

                if total % 10 == 0 and total > 0:
                    print(f"[{datetime.now().strftime('%H:%M:%S')}] {total}/{len(to_generate)} ({stats['ok']} ok, {stats['fail']} fail) | trellis2:{stats['trellis2']} hunyuan:{stats['hunyuan3d']} | {rate:.1f}/min | ETA {eta:.1f}m")

            except:
                continue

    result_thread = threading.Thread(target=process_results, daemon=True)
    result_thread.start()

    print(f"\nSubmitting {len(to_generate)} tasks with {len(keys)} workers...")

    with ThreadPoolExecutor(max_workers=len(keys)) as executor:
        futures = []
        for mesh in to_generate:
            key = get_key()
            if key is None:
                print("All keys exhausted!")
                break
            futures.append(executor.submit(generate_mesh, mesh, key, results_queue))

        for future in as_completed(futures):
            try:
                future.result()
            except Exception as e:
                print(f"Task error: {e}")

    results_queue.put(None)
    result_thread.join(timeout=5)

    elapsed = time.time() - start_time
    print(f"\n{'='*60}")
    print(f"COMPLETE")
    print(f"{'='*60}")
    print(f"Generated: {stats['ok']} (trellis2: {stats['trellis2']}, hunyuan3d: {stats['hunyuan3d']})")
    print(f"Failed: {stats['fail']}")
    print(f"Skipped: {stats['skip']}")
    print(f"Time: {elapsed/60:.1f} minutes")
    print(f"Keys exhausted: {len(exhausted_keys)}/{len(keys)}")
    print(f"{'='*60}")

if __name__ == "__main__":
    main()
