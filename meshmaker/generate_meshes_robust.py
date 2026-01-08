#!/usr/bin/env python3
"""
PID-VLA Robust Parallel Mesh Generation Script v2.0

Features:
- Parallel execution with multiple API keys
- Priority key support (xerotropy has 3-6x credits)
- Model tracking in metadata
- Resume/retry capability
- Improved prompts with quality enhancements
- Fault-tolerant with automatic recovery
- Skip already completed meshes

Usage:
    python generate_meshes_robust.py --dry-run           # Preview
    python generate_meshes_robust.py --test 5            # Test with 5 meshes
    python generate_meshes_robust.py                     # Full run
    python generate_meshes_robust.py --retry-failed      # Retry only failed meshes
"""

import json
import os
import sys
import time
import argparse
import requests
import threading
import hashlib
from pathlib import Path
from dataclasses import dataclass, field, asdict
from typing import Optional, List, Dict, Any, Set, Tuple
from datetime import datetime
from concurrent.futures import ThreadPoolExecutor, as_completed
from queue import Queue, PriorityQueue
import traceback
import random

try:
    import fal_client
except ImportError:
    print("ERROR: fal_client not installed. Run: pip install fal-client")
    sys.exit(1)


# ============================================================================
# CONFIGURATION
# ============================================================================

IMAGE_MODEL = "fal-ai/gpt-image-1.5"
MESH_MODELS = {
    "trellis": "fal-ai/trellis",
    "hunyuan3d": "fal-ai/hunyuan3d-v3/image-to-3d",
}
PRIMARY_MESH_MODEL = "trellis"
FALLBACK_MESH_MODEL = "hunyuan3d"

MAX_RETRIES = 3
RETRY_DELAY = 2
INTER_REQUEST_DELAY = 0.3
DOWNLOAD_TIMEOUT = 300

# Quality enhancement keywords for prompts
PROMPT_ENHANCEMENTS = [
    "single isolated object",
    "centered in frame",
    "pure white background",
    "professional product photography",
    "studio lighting",
    "high resolution 4K",
    "sharp focus",
    "no shadows on background",
    "clean edges",
    "photorealistic render"
]


# ============================================================================
# ENHANCED PROMPT SYSTEM
# ============================================================================

def enhance_prompt(base_prompt: str, category: str = "") -> str:
    """Enhance prompt for better 3D mesh generation quality."""
    # Category-specific enhancements
    category_hints = {
        "robot": "industrial design, mechanical precision, ",
        "gripper": "robotic end effector, mechanical precision, ",
        "fabric": "soft material texture, natural folds, ",
        "hanoi": "wooden toy, smooth finish, ",
        "ycb": "YCB benchmark style, realistic object, ",
        "tool": "professional tool, industrial quality, ",
        "kitchen": "kitchenware, clean design, ",
    }

    category_hint = ""
    for key, hint in category_hints.items():
        if key in category.lower():
            category_hint = hint
            break

    # Build enhanced prompt
    enhancements = ", ".join(PROMPT_ENHANCEMENTS)
    enhanced = f"{category_hint}{base_prompt}, {enhancements}"

    return enhanced


# ============================================================================
# API KEY MANAGER WITH PRIORITY
# ============================================================================

@dataclass
class APIKey:
    key: str
    name: str
    priority: int  # Lower = higher priority (more credits)
    exhausted: bool = False
    usage_count: int = 0
    last_used: float = 0


class PriorityKeyManager:
    """Manages API keys with priority support."""

    def __init__(self, keys_file: Path):
        self.keys: List[APIKey] = []
        self.lock = threading.Lock()
        self._load_keys(keys_file)

    def _load_keys(self, keys_file: Path):
        """Load keys from file with priority detection."""
        if not keys_file.exists():
            return

        current_name = "unknown"
        priority = 10  # Default priority

        with open(keys_file) as f:
            for line in f:
                line = line.strip()

                # Parse comments for key names
                if line.startswith("#"):
                    name = line[1:].strip().lower()
                    if name:
                        current_name = name
                        # Set priority based on key name
                        if "xerotropy" in name:
                            priority = 1  # Highest priority (most credits)
                        elif "infinitropy" in name:
                            priority = 3
                        else:
                            priority = 5
                    continue

                # Skip empty lines and special chars
                if not line or line.startswith("$"):
                    continue

                # Add valid key
                self.keys.append(APIKey(
                    key=line,
                    name=current_name,
                    priority=priority
                ))

                # Reset for next key
                current_name = "unknown"
                priority = 10

        # Sort by priority
        self.keys.sort(key=lambda k: k.priority)

        print(f"[KeyManager] Loaded {len(self.keys)} keys:")
        for k in self.keys:
            print(f"  - {k.name}: priority={k.priority}")

    def get_best_available_key(self) -> Optional[APIKey]:
        """Get the best available key (lowest priority number = most credits)."""
        with self.lock:
            for key in self.keys:
                if not key.exhausted:
                    key.usage_count += 1
                    key.last_used = time.time()
                    return key
            return None

    def mark_exhausted(self, key: APIKey):
        """Mark a key as exhausted."""
        with self.lock:
            key.exhausted = True
            available = sum(1 for k in self.keys if not k.exhausted)
            print(f"[KeyManager] Key '{key.name}' exhausted. {available} keys remaining.")

    def get_stats(self) -> Dict:
        """Get usage statistics."""
        with self.lock:
            return {
                "total": len(self.keys),
                "available": sum(1 for k in self.keys if not k.exhausted),
                "exhausted": sum(1 for k in self.keys if k.exhausted),
                "usage": {k.name: k.usage_count for k in self.keys}
            }


# ============================================================================
# MESH METADATA
# ============================================================================

@dataclass
class MeshMetadata:
    mesh_id: str
    status: str  # pending, generating, complete, failed
    prompt: str
    enhanced_prompt: str
    category: str
    image_model: str = IMAGE_MODEL
    mesh_model: str = ""
    generated_at: str = ""
    generation_time_seconds: float = 0
    api_key_used: str = ""
    files: Dict = field(default_factory=dict)
    error: str = ""
    retry_count: int = 0

    def save(self, output_dir: Path):
        """Save metadata to file."""
        metadata_path = output_dir / self.mesh_id / "metadata.json"
        metadata_path.parent.mkdir(parents=True, exist_ok=True)
        metadata_path.write_text(json.dumps(asdict(self), indent=2))

    @classmethod
    def load(cls, output_dir: Path, mesh_id: str) -> Optional['MeshMetadata']:
        """Load metadata from file."""
        metadata_path = output_dir / mesh_id / "metadata.json"
        if not metadata_path.exists():
            return None
        try:
            data = json.loads(metadata_path.read_text())
            return cls(**data)
        except:
            return None


# ============================================================================
# DOWNLOAD UTILITIES
# ============================================================================

def download_file(url: str, local_path: Path, timeout: int = DOWNLOAD_TIMEOUT) -> bool:
    """Download file with retry logic."""
    for attempt in range(3):
        try:
            response = requests.get(url, stream=True, timeout=timeout)
            response.raise_for_status()

            local_path.parent.mkdir(parents=True, exist_ok=True)

            with open(local_path, "wb") as f:
                for chunk in response.iter_content(chunk_size=8192):
                    f.write(chunk)

            # Verify file size
            if local_path.stat().st_size > 1000:  # At least 1KB
                return True
            else:
                local_path.unlink()

        except Exception as e:
            if attempt < 2:
                time.sleep(1)
            else:
                print(f"      Download failed after 3 attempts: {e}")

    return False


# ============================================================================
# API CALLS
# ============================================================================

class APIError(Exception):
    """Base API error."""
    pass

class KeyExhaustedError(APIError):
    """Key ran out of credits."""
    pass

class TemporaryError(APIError):
    """Temporary error, can retry."""
    pass


def generate_image(prompt: str, api_key: APIKey) -> str:
    """Generate image using GPT-Image-1.5. Returns image URL."""
    os.environ["FAL_KEY"] = api_key.key

    try:
        result = fal_client.subscribe(
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

        images = result.get("images", [])
        if images and images[0].get("url"):
            return images[0]["url"]
        raise APIError("No image URL in response")

    except Exception as e:
        error_str = str(e).lower()
        if any(x in error_str for x in ["credit", "quota", "limit", "exceeded"]):
            raise KeyExhaustedError(str(e))
        elif any(x in error_str for x in ["timeout", "503", "502", "rate"]):
            raise TemporaryError(str(e))
        raise APIError(str(e))


def image_to_mesh(image_url: str, api_key: APIKey, model: str = "trellis") -> Dict:
    """Convert image to mesh. Returns API response."""
    os.environ["FAL_KEY"] = api_key.key

    model_endpoint = MESH_MODELS.get(model, MESH_MODELS["trellis"])

    try:
        if model == "trellis":
            result = fal_client.subscribe(
                model_endpoint,
                arguments={"image_url": image_url},
            )
        elif model == "hunyuan3d":
            result = fal_client.subscribe(
                model_endpoint,
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
        if any(x in error_str for x in ["credit", "quota", "limit", "exceeded"]):
            raise KeyExhaustedError(str(e))
        elif any(x in error_str for x in ["timeout", "503", "502", "rate"]):
            raise TemporaryError(str(e))
        raise APIError(str(e))


def extract_mesh_url(result: Dict) -> Optional[str]:
    """Extract GLB URL from API response."""
    # Try different response formats
    for key in ["model_mesh", "model_glb", "glb"]:
        val = result.get(key, {})
        if isinstance(val, dict) and "url" in val:
            return val["url"]
        if isinstance(val, str) and val.startswith("http"):
            return val

    # Check model_urls
    model_urls = result.get("model_urls", {})
    if "glb" in model_urls:
        glb = model_urls["glb"]
        if isinstance(glb, dict) and "url" in glb:
            return glb["url"]
        if isinstance(glb, str):
            return glb

    return None


# ============================================================================
# MESH GENERATOR
# ============================================================================

class MeshGenerator:
    """Generates meshes with fault tolerance."""

    def __init__(self, output_dir: Path, key_manager: PriorityKeyManager):
        self.output_dir = output_dir
        self.key_manager = key_manager
        self.output_dir.mkdir(parents=True, exist_ok=True)

    def process_mesh(self, mesh: Dict, model: str = PRIMARY_MESH_MODEL) -> MeshMetadata:
        """Process a single mesh with full error handling."""
        mesh_id = mesh["id"]
        category = mesh.get("category", "")
        base_prompt = mesh.get("prompt", mesh.get("text_prompt", ""))
        enhanced_prompt = enhance_prompt(base_prompt, category)

        # Create metadata
        metadata = MeshMetadata(
            mesh_id=mesh_id,
            status="generating",
            prompt=base_prompt,
            enhanced_prompt=enhanced_prompt,
            category=category,
        )

        start_time = time.time()
        mesh_dir = self.output_dir / mesh_id
        mesh_dir.mkdir(parents=True, exist_ok=True)

        # Save prompt
        (mesh_dir / "prompt.txt").write_text(base_prompt)
        (mesh_dir / "enhanced_prompt.txt").write_text(enhanced_prompt)

        try:
            # Get API key
            api_key = self.key_manager.get_best_available_key()
            if not api_key:
                metadata.status = "failed"
                metadata.error = "No API keys available"
                return metadata

            metadata.api_key_used = api_key.name

            # Step 1: Generate image
            image_url = generate_image(enhanced_prompt, api_key)

            # Download reference image
            image_path = mesh_dir / f"{mesh_id}_reference.png"
            if not download_file(image_url, image_path):
                # Try to continue anyway
                pass
            else:
                metadata.files["reference_image"] = image_path.name

            time.sleep(INTER_REQUEST_DELAY)

            # Step 2: Convert to mesh (try primary, fallback to secondary)
            mesh_result = None
            used_model = model

            for try_model in [model, FALLBACK_MESH_MODEL]:
                try:
                    mesh_result = image_to_mesh(image_url, api_key, try_model)
                    used_model = try_model
                    break
                except TemporaryError:
                    time.sleep(RETRY_DELAY)
                    continue
                except KeyExhaustedError:
                    self.key_manager.mark_exhausted(api_key)
                    # Try with new key
                    api_key = self.key_manager.get_best_available_key()
                    if not api_key:
                        raise APIError("All keys exhausted")
                    metadata.api_key_used = api_key.name
                    continue

            if not mesh_result:
                raise APIError("All mesh models failed")

            metadata.mesh_model = MESH_MODELS[used_model]

            # Extract and download mesh
            mesh_url = extract_mesh_url(mesh_result)
            if not mesh_url:
                # Save response for debugging
                (mesh_dir / "api_response.json").write_text(json.dumps(mesh_result, indent=2))
                raise APIError("No mesh URL in response")

            mesh_path = mesh_dir / f"{mesh_id}.glb"
            if not download_file(mesh_url, mesh_path):
                raise APIError("Mesh download failed")

            metadata.files["glb"] = mesh_path.name
            metadata.status = "complete"
            metadata.generation_time_seconds = time.time() - start_time
            metadata.generated_at = datetime.now().isoformat()

        except KeyExhaustedError as e:
            self.key_manager.mark_exhausted(api_key)
            metadata.status = "failed"
            metadata.error = f"Key exhausted: {e}"

        except Exception as e:
            metadata.status = "failed"
            metadata.error = str(e)
            traceback.print_exc()

        # Save metadata
        metadata.save(self.output_dir)

        return metadata


# ============================================================================
# PROGRESS TRACKER
# ============================================================================

class ProgressTracker:
    """Thread-safe progress tracking with persistence."""

    def __init__(self, total: int, output_dir: Path):
        self.total = total
        self.output_dir = output_dir
        self.completed = 0
        self.failed = 0
        self.in_progress: Set[str] = set()
        self.lock = threading.Lock()
        self.start_time = time.time()
        self.results: Dict[str, Dict] = {}

    def start_job(self, mesh_id: str):
        with self.lock:
            self.in_progress.add(mesh_id)

    def complete_job(self, mesh_id: str, metadata: MeshMetadata):
        with self.lock:
            self.in_progress.discard(mesh_id)
            self.results[mesh_id] = asdict(metadata)

            if metadata.status == "complete":
                self.completed += 1
            else:
                self.failed += 1

            # Auto-save every 10 jobs
            if (self.completed + self.failed) % 10 == 0:
                self._save_progress_unlocked()

    def _save_progress_unlocked(self):
        """Save progress (must hold lock)."""
        log_path = self.output_dir / "generation_progress.json"
        data = {
            "timestamp": datetime.now().isoformat(),
            "total": self.total,
            "completed": self.completed,
            "failed": self.failed,
            "elapsed_seconds": time.time() - self.start_time,
            "results": self.results,
        }
        log_path.write_text(json.dumps(data, indent=2))

    def save_progress(self):
        with self.lock:
            self._save_progress_unlocked()

    def get_status(self) -> str:
        with self.lock:
            elapsed = time.time() - self.start_time
            done = self.completed + self.failed
            rate = done / elapsed * 60 if elapsed > 0 else 0
            remaining = self.total - done
            eta = remaining / (rate / 60) if rate > 0 else 0

            return (
                f"{done}/{self.total} "
                f"({self.completed} OK, {self.failed} fail) "
                f"| {rate:.1f}/min "
                f"| ETA {eta/60:.0f}m"
            )


# ============================================================================
# WORKER
# ============================================================================

def worker(
    worker_id: int,
    job_queue: Queue,
    generator: MeshGenerator,
    progress: ProgressTracker,
    mesh_model: str,
):
    """Worker thread."""
    while True:
        try:
            priority, mesh = job_queue.get_nowait()
        except:
            break

        mesh_id = mesh["id"]
        progress.start_job(mesh_id)

        print(f"[W{worker_id}] {mesh_id}")

        metadata = generator.process_mesh(mesh, mesh_model)
        progress.complete_job(mesh_id, metadata)

        status = "+" if metadata.status == "complete" else "x"
        print(f"[W{worker_id}] [{status}] {mesh_id} | {progress.get_status()}")

        job_queue.task_done()
        time.sleep(INTER_REQUEST_DELAY)


# ============================================================================
# MAIN
# ============================================================================

def get_completed_meshes(output_dir: Path) -> Set[str]:
    """Get set of already completed mesh IDs."""
    completed = set()
    if not output_dir.exists():
        return completed

    for subdir in output_dir.iterdir():
        if not subdir.is_dir():
            continue

        # Check for GLB file
        glb_files = list(subdir.glob("*.glb"))
        if glb_files and glb_files[0].stat().st_size > 1000:
            completed.add(subdir.name)

    return completed


def get_failed_meshes(output_dir: Path) -> Set[str]:
    """Get set of failed mesh IDs."""
    failed = set()
    if not output_dir.exists():
        return failed

    for subdir in output_dir.iterdir():
        if not subdir.is_dir():
            continue

        metadata_path = subdir / "metadata.json"
        if metadata_path.exists():
            try:
                data = json.loads(metadata_path.read_text())
                if data.get("status") == "failed":
                    failed.add(subdir.name)
            except:
                pass
        else:
            # Directory exists but no metadata - consider failed
            glb_files = list(subdir.glob("*.glb"))
            if not glb_files:
                failed.add(subdir.name)

    return failed


def main():
    parser = argparse.ArgumentParser(description="Robust parallel mesh generation")
    parser.add_argument("--test", type=int, default=0, help="Test with N meshes")
    parser.add_argument("--dry-run", action="store_true", help="Preview only")
    parser.add_argument("--workers", type=int, default=0, help="Number of workers")
    parser.add_argument("--model", default="trellis", choices=list(MESH_MODELS.keys()))
    parser.add_argument("--manifest", default="mesh_manifest_robotics_expanded.json")
    parser.add_argument("--output", default="output_robotics")
    parser.add_argument("--retry-failed", action="store_true", help="Only retry failed meshes")
    parser.add_argument("--skip-completed", action="store_true", default=True)
    parser.add_argument("--only", help="Only these mesh IDs (comma-separated)")
    args = parser.parse_args()

    script_dir = Path(__file__).parent
    manifest_path = script_dir / args.manifest
    output_dir = script_dir / args.output
    keys_file = script_dir / "api_keys.txt"

    # Load manifest
    if not manifest_path.exists():
        print(f"ERROR: Manifest not found: {manifest_path}")
        # Try to find other manifests
        manifests = list(script_dir.glob("mesh_manifest*.json"))
        if manifests:
            print(f"Available manifests: {[m.name for m in manifests]}")
        sys.exit(1)

    with open(manifest_path) as f:
        data = json.load(f)
    meshes = data.get("meshes", [])
    print(f"Loaded {len(meshes)} mesh definitions from {manifest_path.name}")

    # Filter meshes
    if args.only:
        only_ids = set(args.only.split(","))
        meshes = [m for m in meshes if m["id"] in only_ids]

    # Get completed and failed meshes
    completed = get_completed_meshes(output_dir)
    failed = get_failed_meshes(output_dir)

    print(f"Already completed: {len(completed)}")
    print(f"Previously failed: {len(failed)}")

    if args.retry_failed:
        # Only retry failed ones
        meshes = [m for m in meshes if m["id"] in failed]
        print(f"Retrying {len(meshes)} failed meshes")
    elif args.skip_completed:
        # Skip completed
        meshes = [m for m in meshes if m["id"] not in completed]
        print(f"Skipping {len(completed)} completed, {len(meshes)} remaining")

    if args.test > 0:
        meshes = meshes[:args.test]

    if not meshes:
        print("No meshes to generate!")
        return

    if args.dry_run:
        print(f"\n[DRY-RUN] Would generate {len(meshes)} meshes:")
        for m in meshes[:20]:
            print(f"  - {m['id']}")
        if len(meshes) > 20:
            print(f"  ... and {len(meshes) - 20} more")
        return

    # Load keys
    key_manager = PriorityKeyManager(keys_file)
    if key_manager.get_stats()["total"] == 0:
        print("ERROR: No API keys found")
        sys.exit(1)

    num_workers = args.workers if args.workers > 0 else key_manager.get_stats()["available"]
    num_workers = min(num_workers, len(meshes))

    print(f"\n{'='*60}")
    print(f"PID-VLA Robust Mesh Generator v2.0")
    print(f"{'='*60}")
    print(f"Meshes to generate: {len(meshes)}")
    print(f"Output directory: {output_dir}")
    print(f"Workers: {num_workers}")
    print(f"Mesh model: {args.model}")
    print(f"{'='*60}\n")

    output_dir.mkdir(parents=True, exist_ok=True)

    # Create components
    generator = MeshGenerator(output_dir, key_manager)
    progress = ProgressTracker(len(meshes), output_dir)

    # Create job queue with priorities
    job_queue = Queue()
    for i, mesh in enumerate(meshes):
        job_queue.put((i, mesh))

    # Run workers
    print(f"Starting {num_workers} workers...")

    with ThreadPoolExecutor(max_workers=num_workers) as executor:
        futures = []
        for worker_id in range(num_workers):
            future = executor.submit(
                worker,
                worker_id,
                job_queue,
                generator,
                progress,
                args.model,
            )
            futures.append(future)

        for future in as_completed(futures):
            try:
                future.result()
            except Exception as e:
                print(f"Worker error: {e}")

    # Save final progress
    progress.save_progress()

    print(f"\n{'='*60}")
    print(f"COMPLETE")
    print(f"{'='*60}")
    print(f"Final: {progress.get_status()}")
    print(f"Keys: {key_manager.get_stats()}")
    print(f"{'='*60}")


if __name__ == "__main__":
    main()
