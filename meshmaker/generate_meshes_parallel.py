#!/usr/bin/env python3
"""
PID-VLA Parallel Mesh Generation Script

Runs multiple API keys in parallel for maximum throughput.
Each worker gets its own API key and processes jobs from a shared queue.

Usage:
    python generate_meshes_parallel.py --dry-run          # Preview without API calls
    python generate_meshes_parallel.py --test 3           # Test with 3 meshes
    python generate_meshes_parallel.py                    # Full generation
    python generate_meshes_parallel.py --workers 4        # Limit parallel workers
"""

import json
import os
import sys
import time
import argparse
import requests
import threading
from pathlib import Path
from dataclasses import dataclass, field
from typing import Optional, List, Dict, Any, Set
from datetime import datetime
from concurrent.futures import ThreadPoolExecutor, as_completed
from queue import Queue
import traceback

# Use official fal_client
try:
    import fal_client
except ImportError:
    print("ERROR: fal_client not installed. Run: pip install fal-client")
    sys.exit(1)


# ============================================================================
# CONFIGURATION
# ============================================================================

IMAGE_MODEL = "fal-ai/flux/schnell"
MESH_MODELS = [
    ("trellis", "fal-ai/trellis"),
    ("hunyuan3d", "fal-ai/hunyuan3d-v3/image-to-3d"),
]
DEFAULT_MESH_MODEL = "trellis"  # Primary model to use

RETRY_DELAY = 2  # seconds between retries
MAX_RETRIES = 2
INTER_REQUEST_DELAY = 0.5  # seconds between requests per worker


# ============================================================================
# PROMPT ENHANCEMENT
# ============================================================================

def enhance_prompt(base_prompt: str) -> str:
    """Enhance prompt for better 3D conversion results."""
    enhancements = [
        "single object centered in frame",
        "pure white background",
        "professional product photography",
        "studio lighting",
        "high resolution",
        "sharp focus",
        "isolated object",
    ]
    return f"{base_prompt}, {', '.join(enhancements)}"


# ============================================================================
# THREAD-SAFE PROGRESS TRACKER
# ============================================================================

class ProgressTracker:
    """Thread-safe progress tracking."""

    def __init__(self, total: int):
        self.total = total
        self.completed = 0
        self.failed = 0
        self.in_progress: Set[str] = set()
        self.lock = threading.Lock()
        self.start_time = time.time()
        self.results: Dict[str, Dict] = {}

    def start_job(self, mesh_id: str):
        with self.lock:
            self.in_progress.add(mesh_id)

    def complete_job(self, mesh_id: str, success: bool, result: Dict):
        with self.lock:
            self.in_progress.discard(mesh_id)
            self.results[mesh_id] = result
            if success:
                self.completed += 1
            else:
                self.failed += 1

    def get_status(self) -> str:
        with self.lock:
            elapsed = time.time() - self.start_time
            done = self.completed + self.failed
            rate = done / elapsed if elapsed > 0 else 0
            remaining = self.total - done
            eta = remaining / rate if rate > 0 else 0

            return (
                f"Progress: {done}/{self.total} "
                f"({self.completed} OK, {self.failed} failed) "
                f"| Rate: {rate:.2f}/s "
                f"| ETA: {eta/60:.1f}min "
                f"| Active: {len(self.in_progress)}"
            )

    def save_log(self, output_dir: Path):
        with self.lock:
            log_data = {
                "timestamp": datetime.now().isoformat(),
                "total": self.total,
                "completed": self.completed,
                "failed": self.failed,
                "elapsed_seconds": time.time() - self.start_time,
                "results": self.results,
            }
            log_path = output_dir / "generation_log.json"
            log_path.write_text(json.dumps(log_data, indent=2))


# ============================================================================
# API KEY MANAGER (Thread-Safe)
# ============================================================================

class APIKeyManager:
    """Thread-safe API key distribution and exhaustion tracking."""

    def __init__(self, keys: List[str]):
        self.keys = [k.strip() for k in keys if k.strip() and not k.startswith("#")]
        self.exhausted: Set[str] = set()
        self.lock = threading.Lock()
        self.usage_count: Dict[str, int] = {k: 0 for k in self.keys}
        print(f"[KeyManager] Loaded {len(self.keys)} API keys")

    def get_key_for_worker(self, worker_id: int) -> Optional[str]:
        """Get a dedicated key for a worker (round-robin assignment)."""
        with self.lock:
            available = [k for k in self.keys if k not in self.exhausted]
            if not available:
                return None
            return available[worker_id % len(available)]

    def mark_exhausted(self, key: str):
        """Mark a key as exhausted (out of credits)."""
        with self.lock:
            self.exhausted.add(key)
            remaining = len(self.keys) - len(self.exhausted)
            print(f"[KeyManager] Key exhausted. {remaining} keys remaining.")

    def record_usage(self, key: str):
        with self.lock:
            self.usage_count[key] = self.usage_count.get(key, 0) + 1

    def get_available_count(self) -> int:
        with self.lock:
            return len(self.keys) - len(self.exhausted)

    def get_stats(self) -> Dict:
        with self.lock:
            return {
                "total": len(self.keys),
                "available": len(self.keys) - len(self.exhausted),
                "exhausted": len(self.exhausted),
                "usage": dict(self.usage_count),
            }


# ============================================================================
# MESH GENERATION WORKER
# ============================================================================

@dataclass
class JobResult:
    mesh_id: str
    success: bool
    image_path: Optional[Path] = None
    mesh_path: Optional[Path] = None
    error: Optional[str] = None
    model_used: Optional[str] = None


def download_file(url: str, local_path: Path, timeout: int = 300) -> bool:
    """Download a file from URL."""
    try:
        response = requests.get(url, stream=True, timeout=timeout)
        response.raise_for_status()
        local_path.parent.mkdir(parents=True, exist_ok=True)
        with open(local_path, "wb") as f:
            for chunk in response.iter_content(chunk_size=8192):
                f.write(chunk)
        return True
    except Exception as e:
        print(f"      Download error: {e}")
        return False


def generate_image(prompt: str, api_key: str) -> Optional[str]:
    """Generate image using Flux Schnell. Returns image URL or None."""
    os.environ["FAL_KEY"] = api_key

    try:
        result = fal_client.subscribe(
            IMAGE_MODEL,
            arguments={
                "prompt": prompt,
                "image_size": {"width": 1024, "height": 1024},
                "num_images": 1,
                "enable_safety_checker": True,
            },
        )
        images = result.get("images", [])
        if images:
            return images[0].get("url")
    except Exception as e:
        error_str = str(e).lower()
        if "credit" in error_str or "quota" in error_str or "limit" in error_str:
            raise KeyExhaustedException(api_key, str(e))
        print(f"      Image generation error: {e}")
    return None


def image_to_mesh(image_url: str, api_key: str, model: str = "trellis") -> Optional[Dict]:
    """Convert image to mesh. Returns API result or None."""
    os.environ["FAL_KEY"] = api_key

    try:
        if model == "trellis":
            result = fal_client.subscribe(
                "fal-ai/trellis",
                arguments={"image_url": image_url},
            )
        elif model == "hunyuan3d":
            result = fal_client.subscribe(
                "fal-ai/hunyuan3d-v3/image-to-3d",
                arguments={
                    "input_image_url": image_url,
                    "generate_type": "Normal",
                    "face_count": 50000,
                    "enable_pbr": True,
                },
            )
        else:
            raise ValueError(f"Unknown model: {model}")
        return result
    except Exception as e:
        error_str = str(e).lower()
        if "credit" in error_str or "quota" in error_str or "limit" in error_str:
            raise KeyExhaustedException(api_key, str(e))
        print(f"      Mesh conversion error ({model}): {e}")
    return None


def extract_mesh_url(result: Dict) -> Optional[str]:
    """Extract GLB URL from API response."""
    # Trellis format
    model_mesh = result.get("model_mesh", {})
    if isinstance(model_mesh, dict) and "url" in model_mesh:
        return model_mesh["url"]

    # Hunyuan format
    model_glb = result.get("model_glb", {})
    if isinstance(model_glb, dict) and "url" in model_glb:
        return model_glb["url"]
    if isinstance(model_glb, str) and model_glb.startswith("http"):
        return model_glb

    # Alternative formats
    glb = result.get("glb", {})
    if isinstance(glb, dict) and "url" in glb:
        return glb["url"]
    if isinstance(glb, str) and glb.startswith("http"):
        return glb

    # model_urls format
    model_urls = result.get("model_urls", {})
    if "glb" in model_urls:
        glb_data = model_urls["glb"]
        if isinstance(glb_data, dict) and "url" in glb_data:
            return glb_data["url"]
        if isinstance(glb_data, str):
            return glb_data

    return None


class KeyExhaustedException(Exception):
    """Raised when an API key runs out of credits."""
    def __init__(self, key: str, message: str):
        self.key = key
        super().__init__(message)


def process_mesh_job(
    mesh: Dict,
    output_dir: Path,
    api_key: str,
    key_manager: APIKeyManager,
    mesh_model: str = "trellis",
) -> JobResult:
    """Process a single mesh job. Returns JobResult."""
    mesh_id = mesh["id"]
    prompt = mesh.get("prompt", mesh.get("text_prompt", ""))
    mesh_dir = output_dir / mesh_id
    mesh_dir.mkdir(parents=True, exist_ok=True)

    # Save prompt
    (mesh_dir / "prompt.txt").write_text(prompt)

    try:
        # Step 1: Generate image
        enhanced_prompt = enhance_prompt(prompt)
        image_url = generate_image(enhanced_prompt, api_key)

        if not image_url:
            return JobResult(mesh_id, False, error="Image generation failed")

        key_manager.record_usage(api_key)

        # Download reference image
        image_path = mesh_dir / f"{mesh_id}_reference.png"
        download_file(image_url, image_path)

        time.sleep(INTER_REQUEST_DELAY)

        # Step 2: Convert to mesh
        result = image_to_mesh(image_url, api_key, mesh_model)

        if not result:
            return JobResult(mesh_id, False, image_path=image_path, error="Mesh conversion failed")

        key_manager.record_usage(api_key)

        mesh_url = extract_mesh_url(result)
        if not mesh_url:
            # Save response for debugging
            (mesh_dir / "api_response.json").write_text(json.dumps(result, indent=2))
            return JobResult(mesh_id, False, image_path=image_path, error="No mesh URL in response")

        # Download mesh
        mesh_path = mesh_dir / f"{mesh_id}.glb"
        if download_file(mesh_url, mesh_path):
            return JobResult(mesh_id, True, image_path=image_path, mesh_path=mesh_path, model_used=mesh_model)
        else:
            return JobResult(mesh_id, False, image_path=image_path, error="Mesh download failed")

    except KeyExhaustedException as e:
        key_manager.mark_exhausted(e.key)
        return JobResult(mesh_id, False, error=f"API key exhausted: {e}")
    except Exception as e:
        traceback.print_exc()
        return JobResult(mesh_id, False, error=str(e))


# ============================================================================
# WORKER FUNCTION
# ============================================================================

def worker(
    worker_id: int,
    job_queue: Queue,
    output_dir: Path,
    key_manager: APIKeyManager,
    progress: ProgressTracker,
    mesh_model: str,
    dry_run: bool = False,
):
    """Worker thread that processes jobs from the queue."""

    while True:
        try:
            mesh = job_queue.get_nowait()
        except:
            break  # Queue empty

        mesh_id = mesh["id"]
        progress.start_job(mesh_id)

        # Get API key for this worker
        api_key = key_manager.get_key_for_worker(worker_id)

        if not api_key:
            print(f"[Worker {worker_id}] No API keys available, stopping")
            progress.complete_job(mesh_id, False, {"error": "No API keys"})
            job_queue.task_done()
            break

        if dry_run:
            print(f"[Worker {worker_id}] [DRY-RUN] Would generate: {mesh_id}")
            progress.complete_job(mesh_id, True, {"dry_run": True})
            job_queue.task_done()
            time.sleep(0.1)
            continue

        print(f"[Worker {worker_id}] Processing: {mesh_id}")

        # Process with retries
        for attempt in range(MAX_RETRIES + 1):
            result = process_mesh_job(mesh, output_dir, api_key, key_manager, mesh_model)

            if result.success:
                break

            # Check if we should retry
            if "exhausted" in (result.error or "").lower():
                # Get a new key
                api_key = key_manager.get_key_for_worker(worker_id)
                if not api_key:
                    break
            elif attempt < MAX_RETRIES:
                print(f"[Worker {worker_id}] Retrying {mesh_id} (attempt {attempt + 2})")
                time.sleep(RETRY_DELAY)

        # Record result
        result_dict = {
            "success": result.success,
            "image_path": str(result.image_path) if result.image_path else None,
            "mesh_path": str(result.mesh_path) if result.mesh_path else None,
            "error": result.error,
            "model": result.model_used,
        }
        progress.complete_job(mesh_id, result.success, result_dict)

        status_char = "+" if result.success else "x"
        print(f"[Worker {worker_id}] [{status_char}] {mesh_id} | {progress.get_status()}")

        job_queue.task_done()
        time.sleep(INTER_REQUEST_DELAY)


# ============================================================================
# MAIN
# ============================================================================

def main():
    parser = argparse.ArgumentParser(description="Parallel mesh generation for PID-VLA")
    parser.add_argument("--test", type=int, default=0, help="Test mode: generate N meshes only")
    parser.add_argument("--dry-run", action="store_true", help="Preview without API calls")
    parser.add_argument("--workers", type=int, default=0, help="Number of parallel workers (default: num keys)")
    parser.add_argument("--model", default="trellis", choices=["trellis", "hunyuan3d"], help="Mesh model to use")
    parser.add_argument("--manifest", default="mesh_manifest_1000.json", help="Manifest file to use")
    parser.add_argument("--output", default="output_parallel", help="Output directory")
    parser.add_argument("--only", help="Comma-separated list of mesh IDs to generate")
    parser.add_argument("--skip", help="Comma-separated list of mesh IDs to skip")
    parser.add_argument("--skip-existing", action="store_true", help="Skip meshes that already have .glb files")
    args = parser.parse_args()

    script_dir = Path(__file__).parent
    manifest_path = script_dir / args.manifest
    output_dir = script_dir / args.output
    keys_file = script_dir / "api_keys.txt"

    # Load manifest
    if not manifest_path.exists():
        print(f"ERROR: Manifest not found: {manifest_path}")
        sys.exit(1)

    with open(manifest_path) as f:
        data = json.load(f)
    meshes = data.get("meshes", [])
    print(f"Loaded {len(meshes)} mesh definitions from {manifest_path.name}")

    # Filter meshes
    if args.only:
        only_ids = set(args.only.split(","))
        meshes = [m for m in meshes if m["id"] in only_ids]

    if args.skip:
        skip_ids = set(args.skip.split(","))
        meshes = [m for m in meshes if m["id"] not in skip_ids]

    if args.skip_existing:
        existing = set()
        for mesh in meshes:
            glb_path = output_dir / mesh["id"] / f"{mesh['id']}.glb"
            if glb_path.exists():
                existing.add(mesh["id"])
        if existing:
            print(f"Skipping {len(existing)} existing meshes")
            meshes = [m for m in meshes if m["id"] not in existing]

    if args.test > 0:
        meshes = meshes[:args.test]

    if not meshes:
        print("No meshes to generate")
        return

    print(f"Will process {len(meshes)} meshes")

    # Load API keys
    keys = []
    if keys_file.exists():
        with open(keys_file) as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith("#") and not line.startswith("$"):
                    keys.append(line)

    if not keys and not args.dry_run:
        print("ERROR: No API keys found in api_keys.txt")
        sys.exit(1)

    num_keys = len(keys) if keys else 1
    num_workers = args.workers if args.workers > 0 else num_keys
    num_workers = min(num_workers, len(meshes))  # Don't spawn more workers than jobs

    print(f"\n{'='*60}")
    print(f"PID-VLA Parallel Mesh Generator")
    print(f"{'='*60}")
    print(f"Meshes to generate: {len(meshes)}")
    print(f"API keys available: {num_keys}")
    print(f"Parallel workers: {num_workers}")
    print(f"Mesh model: {args.model}")
    print(f"Output directory: {output_dir}")
    print(f"Dry run: {args.dry_run}")
    print(f"{'='*60}\n")

    if args.dry_run:
        print("[DRY-RUN] Would generate:")
        for m in meshes[:20]:
            print(f"  - {m['id']}: {m.get('prompt', '')[:50]}...")
        if len(meshes) > 20:
            print(f"  ... and {len(meshes) - 20} more")
        return

    # Create output directory
    output_dir.mkdir(parents=True, exist_ok=True)

    # Initialize components
    key_manager = APIKeyManager(keys)
    progress = ProgressTracker(len(meshes))

    # Create job queue
    job_queue = Queue()
    for mesh in meshes:
        job_queue.put(mesh)

    # Start workers
    print(f"Starting {num_workers} parallel workers...")

    with ThreadPoolExecutor(max_workers=num_workers) as executor:
        futures = []
        for worker_id in range(num_workers):
            future = executor.submit(
                worker,
                worker_id,
                job_queue,
                output_dir,
                key_manager,
                progress,
                args.model,
                args.dry_run,
            )
            futures.append(future)

        # Wait for all workers
        for future in as_completed(futures):
            try:
                future.result()
            except Exception as e:
                print(f"Worker error: {e}")

    # Save final log
    progress.save_log(output_dir)

    # Print summary
    print(f"\n{'='*60}")
    print(f"GENERATION COMPLETE")
    print(f"{'='*60}")
    print(progress.get_status())
    print(f"Key usage: {key_manager.get_stats()}")
    print(f"Log saved to: {output_dir / 'generation_log.json'}")
    print(f"{'='*60}")


if __name__ == "__main__":
    main()
