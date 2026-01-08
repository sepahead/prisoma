#!/usr/bin/env python3 -u
"""
Multi-process mesh generation for VLA - each key runs in its own process(es).
Xerotropy gets 5 processes, others get 1 each.
"""

import sys
sys.stdout.reconfigure(line_buffering=True)

import json
import os
import time
import requests
import multiprocessing as mp
from pathlib import Path
from datetime import datetime
from filelock import FileLock
import fcntl

try:
    import fal_client
except ImportError:
    print("ERROR: pip install fal-client")
    sys.exit(1)

# Configuration
IMAGE_MODEL = "fal-ai/gpt-image-1.5"
MESH_MODELS = {
    "trellis2": {"id": "fal-ai/trellis-2", "param": "image_url", "extra": {"resolution": 1024, "texture_size": 2048}},
    "hunyuan3d": {"id": "fal-ai/hunyuan3d-v3/image-to-3d", "param": "input_image_url", "extra": {}},
}
PRIMARY_MESH = "trellis2"
FALLBACK_MESH = "hunyuan3d"

SCRIPT_DIR = Path(__file__).parent
OUTPUT_DIR = SCRIPT_DIR / "output_vla_new"
KEYS_FILE = SCRIPT_DIR / "api_keys.txt"
QUEUE_FILE = SCRIPT_DIR / "mesh_queue_vla.json"
LOCK_FILE = SCRIPT_DIR / "mesh_queue_vla.lock"

def load_keys():
    keys = []
    current_name = "unknown"
    with open(KEYS_FILE) as f:
        for line in f:
            line = line.strip()
            if line.startswith('#'):
                # Extract name from comment (e.g., "# xerotropy" or "#sepehrmn")
                name = line.lstrip('#').strip()
                if name and not name.startswith('=') and not name.startswith('Add') and not name.startswith('Get') and not name.startswith('The') and not name.startswith('Paste') and not name.startswith('Example') and 'fal' not in name:
                    current_name = name
            elif line and not line.startswith('$') and ':' in line:
                keys.append({"key": line, "name": current_name})
    return keys

def enhance_prompt(prompt):
    extras = "single isolated object, centered, pure white background, professional studio lighting, high resolution, sharp focus, photorealistic, detailed texture"
    return f"{prompt}, {extras}"

def get_next_mesh():
    """Get next mesh from queue (process-safe)."""
    lock_path = str(LOCK_FILE)
    with open(lock_path, 'w') as lock_file:
        fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX)
        try:
            with open(QUEUE_FILE) as f:
                queue = json.load(f)

            if not queue:
                return None

            mesh = queue.pop(0)

            with open(QUEUE_FILE, 'w') as f:
                json.dump(queue, f)

            return mesh
        finally:
            fcntl.flock(lock_file.fileno(), fcntl.LOCK_UN)

def generate_single_mesh(mesh_def, api_key):
    """Generate a single mesh."""
    mesh_id = mesh_def["id"]
    category = mesh_def.get("category", "")
    prompt = enhance_prompt(mesh_def["prompt"])

    output_path = OUTPUT_DIR / category / mesh_id
    glb_file = output_path / f"{mesh_id}.glb"

    if glb_file.exists():
        return "skip", mesh_id, None

    os.environ["FAL_KEY"] = api_key

    try:
        # Generate image
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
            return "fail", mesh_id, "No image URL"

        image_url = images[0]["url"]

        # Generate mesh
        mesh_model_used = None
        glb_url = None

        for model_key in [PRIMARY_MESH, FALLBACK_MESH]:
            model_cfg = MESH_MODELS[model_key]
            try:
                args = {model_cfg["param"]: image_url}
                args.update(model_cfg.get("extra", {}))
                mesh_result = fal_client.subscribe(model_cfg["id"], arguments=args)
                glb_url = (
                    mesh_result.get("glb_url") or
                    mesh_result.get("model_mesh", {}).get("url") or
                    mesh_result.get("model_glb", {}).get("url")
                )
                if glb_url:
                    mesh_model_used = model_key
                    break
            except:
                continue

        if not glb_url:
            return "fail", mesh_id, "No GLB URL"

        output_path.mkdir(parents=True, exist_ok=True)

        # Download
        img_resp = requests.get(image_url, timeout=120)
        with open(output_path / f"{mesh_id}.png", "wb") as f:
            f.write(img_resp.content)

        glb_resp = requests.get(glb_url, timeout=120)
        with open(glb_file, "wb") as f:
            f.write(glb_resp.content)

        with open(output_path / "metadata.json", "w") as f:
            json.dump({
                "id": mesh_id,
                "category": category,
                "prompt": prompt,
                "image_model": IMAGE_MODEL,
                "mesh_model": MESH_MODELS[mesh_model_used]["id"],
                "timestamp": datetime.now().isoformat(),
            }, f, indent=2)

        return "ok", mesh_id, mesh_model_used

    except Exception as e:
        err = str(e).lower()
        if any(x in err for x in ["credit", "quota", "exceeded"]):
            return "exhausted", mesh_id, str(e)
        return "fail", mesh_id, str(e)[:100]

def worker_process(worker_id, key_name, api_key, stats_queue):
    """Worker process - continuously pulls from queue and generates meshes."""
    print(f"[W{worker_id}] Started ({key_name})", flush=True)

    count = 0
    while True:
        mesh = get_next_mesh()
        if mesh is None:
            print(f"[W{worker_id}] Queue empty, exiting", flush=True)
            break

        mesh_id = mesh["id"]
        print(f"[W{worker_id}] {mesh_id}", flush=True)

        status, mid, info = generate_single_mesh(mesh, api_key)

        if status == "ok":
            count += 1
            print(f"[W{worker_id}] ✓ {mesh_id} ({info}) #{count}", flush=True)
            stats_queue.put(("ok", worker_id, key_name))
        elif status == "skip":
            print(f"[W{worker_id}] skip {mesh_id}", flush=True)
            stats_queue.put(("skip", worker_id, key_name))
        elif status == "exhausted":
            print(f"[W{worker_id}] KEY EXHAUSTED - stopping", flush=True)
            stats_queue.put(("exhausted", worker_id, key_name))
            break
        else:
            print(f"[W{worker_id}] ✗ {mesh_id}: {info}", flush=True)
            stats_queue.put(("fail", worker_id, key_name))

    print(f"[W{worker_id}] Done - generated {count} meshes", flush=True)

def main():
    # Load manifests
    manifest_files = ["mesh_manifest_vla_new.json"]
    all_meshes = []
    seen_ids = set()

    for mf in manifest_files:
        path = SCRIPT_DIR / mf
        if path.exists():
            with open(path) as f:
                for m in json.load(f).get("meshes", []):
                    if m["id"] not in seen_ids:
                        all_meshes.append(m)
                        seen_ids.add(m["id"])

    print(f"Loaded {len(all_meshes)} mesh definitions")

    # Filter out completed
    OUTPUT_DIR.mkdir(exist_ok=True)
    existing = {glb.stem for glb in OUTPUT_DIR.glob("**/*.glb")}
    to_generate = [m for m in all_meshes if m["id"] not in existing]

    print(f"Already complete: {len(existing)}, remaining: {len(to_generate)}")

    if not to_generate:
        print("All done!")
        return

    # Write queue
    with open(QUEUE_FILE, 'w') as f:
        json.dump(to_generate, f)

    # Create lock file
    LOCK_FILE.touch()

    # Load keys
    keys = load_keys()
    print(f"Loaded {len(keys)} keys")

    # Spawn workers: xerotropy gets 5, others get 1
    workers = []
    stats_queue = mp.Queue()
    worker_id = 0

    for key_info in keys:
        key_name = key_info["name"]
        api_key = key_info["key"]

        # Xerotropy gets 5 workers
        num_workers = 5 if "xerotropy" in key_name.lower() else 1

        for i in range(num_workers):
            p = mp.Process(target=worker_process, args=(worker_id, key_name, api_key, stats_queue))
            workers.append(p)
            worker_id += 1

    print(f"\nSpawning {len(workers)} worker processes...")
    print("=" * 60)

    start_time = time.time()

    # Start all workers
    for p in workers:
        p.start()

    # Monitor stats
    stats = {"ok": 0, "fail": 0, "skip": 0, "exhausted": 0}

    while any(p.is_alive() for p in workers):
        try:
            while not stats_queue.empty():
                status, wid, kname = stats_queue.get_nowait()
                stats[status] = stats.get(status, 0) + 1

            elapsed = time.time() - start_time
            total = stats["ok"] + stats["fail"]
            rate = total / elapsed * 60 if elapsed > 0 else 0
            remaining = len(to_generate) - total - stats["skip"]
            eta = remaining / (rate / 60) / 60 if rate > 0 else 0

            glb_count = len(list(OUTPUT_DIR.glob("**/*.glb")))

            print(f"\r[{datetime.now().strftime('%H:%M:%S')}] GLB:{glb_count} | ok:{stats['ok']} fail:{stats['fail']} skip:{stats['skip']} | {rate:.1f}/min | ETA:{eta:.0f}m", end="", flush=True)

            time.sleep(5)
        except KeyboardInterrupt:
            print("\nStopping workers...")
            for p in workers:
                p.terminate()
            break

    # Wait for all workers
    for p in workers:
        p.join(timeout=5)

    # Final stats
    elapsed = time.time() - start_time
    glb_count = len(list(OUTPUT_DIR.glob("**/*.glb")))

    print(f"\n\n{'='*60}")
    print("COMPLETE")
    print(f"{ '='*60}")
    print(f"Total GLB files: {glb_count}")
    print(f"Generated: {stats['ok']}, Failed: {stats['fail']}, Skipped: {stats['skip']}")
    print(f"Keys exhausted: {stats['exhausted']}")
    print(f"Time: {elapsed/60:.1f} minutes")
    print(f"{ '='*60}")

    # Cleanup
    QUEUE_FILE.unlink(missing_ok=True)
    LOCK_FILE.unlink(missing_ok=True)

if __name__ == "__main__":
    main()
