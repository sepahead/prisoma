#!/usr/bin/env python3 -u
"""
Spawn script specifically for Xerotropy key to generate Bundeswehr items.
"""
import sys
# sys.stdout.reconfigure(line_buffering=True) # Avoid this if it causes issues

import json
import os
import time
import requests
import multiprocessing as mp
from pathlib import Path
from datetime import datetime
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
OUTPUT_DIR = SCRIPT_DIR / "output_bundeswehr"
KEYS_FILE = SCRIPT_DIR / "api_keys.txt"
MANIFEST_FILE = SCRIPT_DIR / "mesh_manifest_bundeswehr.json"
QUEUE_FILE = SCRIPT_DIR / "mesh_queue_bundeswehr.json"
LOCK_FILE = SCRIPT_DIR / "mesh_queue_bundeswehr.lock"

# Load ONLY xerotropy key
def load_xerotropy_key():
    with open(KEYS_FILE) as f:
        lines = f.readlines()
    
    found_key = None
    next_is_key = False
    
    # Simple parsing to find key associated with "xerotropy"
    # Assuming format:
    # # xerotropy
    # key-string
    
    current_name = None
    for line in lines:
        line = line.strip()
        if not line: continue
        
        if line.startswith("#"):
            current_name = line.lstrip("#").strip().lower()
        elif ":" in line and not line.startswith("$"):
            if current_name and "xerotropy" in current_name:
                return line
    return None

def enhance_prompt(prompt):
    extras = "single isolated object, centered, pure white background, professional studio lighting, high resolution, sharp focus, photorealistic, detailed texture"
    return f"{prompt}, {extras}"

def get_next_mesh():
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
        return "fail", mesh_id, str(e)[:100]

def worker_process(worker_id, api_key, stats_queue):
    print(f"[W{worker_id}] Started (xerotropy)", flush=True)
    count = 0
    while True:
        mesh = get_next_mesh()
        if mesh is None:
            break
        
        mesh_id = mesh["id"]
        print(f"[W{worker_id}] {mesh_id}", flush=True)
        status, mid, info = generate_single_mesh(mesh, api_key)
        
        if status == "ok":
            print(f"[W{worker_id}] ✓ {mesh_id}", flush=True)
            stats_queue.put("ok")
        elif status == "skip":
            print(f"[W{worker_id}] skip {mesh_id}", flush=True)
            stats_queue.put("skip")
        else:
            print(f"[W{worker_id}] ✗ {mesh_id}: {info}", flush=True)
            stats_queue.put("fail")
            
    print(f"[W{worker_id}] Done", flush=True)

def main():
    xerotropy_key = load_xerotropy_key()
    if not xerotropy_key:
        print("Error: Could not find 'xerotropy' key in api_keys.txt")
        return

    print(f"Using key: ...{xerotropy_key[-8:]}")
    
    with open(MANIFEST_FILE) as f:
        meshes = json.load(f)["meshes"]
        
    OUTPUT_DIR.mkdir(exist_ok=True)
    existing = {glb.stem for glb in OUTPUT_DIR.glob("**/*.glb")}
    to_generate = [m for m in meshes if m["id"] not in existing]
    
    print(f"Total: {len(meshes)}, Remaining: {len(to_generate)}")
    
    with open(QUEUE_FILE, 'w') as f:
        json.dump(to_generate, f)
        
    LOCK_FILE.touch()
    
    workers = []
    stats_queue = mp.Queue()
    
    # Spawn 5 workers
    for i in range(5):
        p = mp.Process(target=worker_process, args=(i, xerotropy_key, stats_queue))
        workers.append(p)
        
    print("Spawning 5 workers...")
    for p in workers:
        p.start()
        
    for p in workers:
        p.join()
        
    print("Done.")
    QUEUE_FILE.unlink(missing_ok=True)
    LOCK_FILE.unlink(missing_ok=True)

if __name__ == "__main__":
    main()
