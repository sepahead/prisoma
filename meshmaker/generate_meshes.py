#!/usr/bin/env python3
"""
PID-VLA Multi-Model Mesh Generation Script

Uses fal.ai APIs via official fal_client to generate 3D meshes.

Models:
- fal-ai/flux/schnell (Image generation)
- fal-ai/hunyuan3d-v3/image-to-3d (Image to 3D)
- fal-ai/trellis (Image to 3D)

Usage:
    python generate_meshes.py --test 5   # Test run with 5 meshes
    python generate_meshes.py            # Full generation
"""

import json
import os
import sys
import time
import argparse
import requests
from pathlib import Path
from dataclasses import dataclass, field
from typing import Optional, List, Dict, Any, Tuple
from datetime import datetime

# Use official fal_client
try:
    import fal_client
except ImportError:
    print("ERROR: fal_client not installed. Run: pip install fal-client")
    sys.exit(1)


# ============================================================================
# PROMPTS
# ============================================================================


def enhance_image_prompt(base_prompt: str) -> str:
    """Create prompt optimized for 3D conversion."""
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
# API KEY MANAGEMENT
# ============================================================================


class APIKeyRotator:
    """Manages rotation through multiple API keys."""

    def __init__(self, keys: List[str]):
        self.keys = [k.strip() for k in keys if k.strip() and not k.startswith("#")]
        self.current_index = 0
        self.exhausted = set()
        print(f"[KeyRotator] Loaded {len(self.keys)} API keys")

    def get_next_key(self) -> Optional[str]:
        """Get the next usable API key."""
        if not self.keys:
            return None

        attempts = 0
        while attempts < len(self.keys):
            key = self.keys[self.current_index]
            self.current_index = (self.current_index + 1) % len(self.keys)

            if key not in self.exhausted:
                return key
            attempts += 1
        return None

    def mark_exhausted(self, key: str):
        self.exhausted.add(key)

    def get_status(self) -> Dict[str, int]:
        return {
            "total": len(self.keys),
            "usable": len(self.keys) - len(self.exhausted),
            "exhausted": len(self.exhausted),
        }


# ============================================================================
# MESH GENERATOR
# ============================================================================


@dataclass
class GenerationResult:
    """Result from a generation attempt."""

    model_name: str
    success: bool
    mesh_path: Optional[Path] = None
    image_path: Optional[Path] = None
    error: Optional[str] = None


class MeshGenerator:
    """Main mesh generation orchestrator."""

    def __init__(self, key_rotator: APIKeyRotator, output_dir: Path):
        self.key_rotator = key_rotator
        self.output_dir = output_dir
        self.output_dir.mkdir(parents=True, exist_ok=True)

    def download_file(self, url: str, local_path: Path) -> bool:
        """Download a file from URL."""
        try:
            response = requests.get(url, stream=True, timeout=300)
            response.raise_for_status()
            local_path.parent.mkdir(parents=True, exist_ok=True)
            with open(local_path, "wb") as f:
                for chunk in response.iter_content(chunk_size=8192):
                    f.write(chunk)
            return True
        except Exception as e:
            print(f"      Download error: {e}")
            return False

    def generate_image(self, prompt: str, api_key: str) -> Optional[str]:
        """Generate image using Flux Schnell."""
        os.environ["FAL_KEY"] = api_key

        try:
            result = fal_client.subscribe(
                "fal-ai/flux/schnell",
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
            print(f"      Image gen error: {e}")
            if "credit" in str(e).lower() or "quota" in str(e).lower():
                self.key_rotator.mark_exhausted(api_key)
        return None

    def image_to_mesh_hunyuan(self, image_url: str, api_key: str) -> Optional[Dict]:
        """Convert image to mesh using Hunyuan3D."""
        os.environ["FAL_KEY"] = api_key

        try:
            result = fal_client.subscribe(
                "fal-ai/hunyuan3d-v3/image-to-3d",
                arguments={
                    "input_image_url": image_url,
                    "generate_type": "Normal",
                    "face_count": 50000,
                    "enable_pbr": True,
                },
            )
            return result
        except Exception as e:
            print(f"      Hunyuan error: {e}")
            if "credit" in str(e).lower() or "quota" in str(e).lower():
                self.key_rotator.mark_exhausted(api_key)
        return None

    def image_to_mesh_trellis(self, image_url: str, api_key: str) -> Optional[Dict]:
        """Convert image to mesh using Trellis."""
        os.environ["FAL_KEY"] = api_key

        try:
            result = fal_client.subscribe(
                "fal-ai/trellis",
                arguments={
                    "image_url": image_url,
                },
            )
            return result
        except Exception as e:
            print(f"      Trellis error: {e}")
            if "credit" in str(e).lower() or "quota" in str(e).lower():
                self.key_rotator.mark_exhausted(api_key)
        return None

    def extract_mesh_url(self, result: Dict) -> Optional[str]:
        """Extract GLB URL from API response."""
        # Trellis format: model_mesh.url
        model_mesh = result.get("model_mesh", {})
        if isinstance(model_mesh, dict) and "url" in model_mesh:
            return model_mesh["url"]

        # Hunyuan format: model_glb.url
        model_glb = result.get("model_glb", {})
        if isinstance(model_glb, dict) and "url" in model_glb:
            return model_glb["url"]
        if isinstance(model_glb, str) and model_glb.startswith("http"):
            return model_glb

        # Alternative: glb.url
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

    def process_mesh(self, mesh_id: str, prompt: str) -> List[GenerationResult]:
        """Process a single mesh through the pipeline."""
        results = []
        mesh_dir = self.output_dir / mesh_id
        mesh_dir.mkdir(parents=True, exist_ok=True)

        # Save prompt
        (mesh_dir / "prompt.txt").write_text(prompt)

        # Step 1: Generate image
        enhanced_prompt = enhance_image_prompt(prompt)
        print(f"    [flux] Generating image...")

        api_key = self.key_rotator.get_next_key()
        if not api_key:
            return [GenerationResult("flux", False, error="No API keys")]

        image_url = self.generate_image(enhanced_prompt, api_key)
        if not image_url:
            return [GenerationResult("flux", False, error="Image generation failed")]

        print(f"    [flux] Success")

        # Download reference image
        image_path = mesh_dir / f"{mesh_id}_reference.png"
        self.download_file(image_url, image_path)

        # Step 2: Convert to mesh with multiple models
        mesh_models = [
            ("trellis", self.image_to_mesh_trellis),  # Faster model first
            ("hunyuan3d", self.image_to_mesh_hunyuan),  # High quality but slower
        ]

        for model_name, method in mesh_models:
            print(f"    [{model_name}] Converting to mesh...")

            api_key = self.key_rotator.get_next_key()
            if not api_key:
                results.append(GenerationResult(model_name, False, error="No API keys"))
                continue

            try:
                result = method(image_url, api_key)
                if result:
                    mesh_url = self.extract_mesh_url(result)
                    if mesh_url:
                        mesh_path = mesh_dir / f"{mesh_id}_{model_name}.glb"
                        if self.download_file(mesh_url, mesh_path):
                            print(f"    [{model_name}] Success: {mesh_path.name}")
                            results.append(
                                GenerationResult(
                                    model_name,
                                    True,
                                    mesh_path=mesh_path,
                                    image_path=image_path,
                                )
                            )
                        else:
                            results.append(
                                GenerationResult(
                                    model_name, False, error="Download failed"
                                )
                            )
                    else:
                        results.append(
                            GenerationResult(
                                model_name, False, error="No mesh URL in response"
                            )
                        )
                        # Debug: save response
                        (mesh_dir / f"{model_name}_response.json").write_text(
                            json.dumps(result, indent=2)
                        )
                else:
                    results.append(
                        GenerationResult(model_name, False, error="API call failed")
                    )
            except Exception as e:
                results.append(GenerationResult(model_name, False, error=str(e)))
                print(f"    [{model_name}] Error: {e}")

            time.sleep(1)

        return results

    def run(self, meshes: List[Dict], limit: int = 0):
        """Run the mesh generation pipeline."""
        if limit > 0:
            meshes = meshes[:limit]

        print(f"\n{'=' * 60}")
        print(f"PID-VLA Mesh Generator")
        print(f"{'=' * 60}")
        print(f"Jobs: {len(meshes)}")
        print(f"Output: {self.output_dir}")
        print(f"API Keys: {self.key_rotator.get_status()}")
        print(f"{'=' * 60}\n")

        all_results = {}
        total_success = 0
        total_failed = 0

        for i, mesh in enumerate(meshes):
            mesh_id = mesh["id"]
            prompt = mesh.get("text_prompt", mesh.get("prompt", ""))

            print(f"\n[{i + 1}/{len(meshes)}] {mesh_id}")
            print(f"    Prompt: {prompt[:50]}...")

            status = self.key_rotator.get_status()
            if status["usable"] == 0:
                print("\n[STOP] All API keys exhausted!")
                break

            results = self.process_mesh(mesh_id, prompt)
            all_results[mesh_id] = results

            successes = sum(1 for r in results if r.success)
            failures = sum(1 for r in results if not r.success)
            total_success += successes
            total_failed += failures

            print(f"    Result: {successes} success, {failures} failed")

        # Save log
        log_path = self.output_dir / "generation_log.json"
        log_data = {
            "timestamp": datetime.now().isoformat(),
            "summary": {"success": total_success, "failed": total_failed},
            "results": {
                mesh_id: [
                    {
                        "model": r.model_name,
                        "success": r.success,
                        "mesh_path": str(r.mesh_path) if r.mesh_path else None,
                        "error": r.error,
                    }
                    for r in results
                ]
                for mesh_id, results in all_results.items()
            },
        }
        log_path.write_text(json.dumps(log_data, indent=2))

        print(f"\n{'=' * 60}")
        print(f"COMPLETE: {total_success} meshes, {total_failed} failures")
        print(f"Log: {log_path}")
        print(f"{'=' * 60}")


# ============================================================================
# MAIN
# ============================================================================


def main():
    parser = argparse.ArgumentParser(description="Generate 3D meshes for PID-VLA")
    parser.add_argument(
        "--test", type=int, default=0, help="Test mode: generate N meshes only"
    )
    parser.add_argument("--only", help="Comma-separated list of mesh IDs")
    parser.add_argument(
        "--dry-run", action="store_true", help="Preview without API calls"
    )
    args = parser.parse_args()

    script_dir = Path(__file__).parent
    manifest_path = script_dir / "mesh_manifest.json"
    output_dir = script_dir / "output"
    keys_file = script_dir / "api_keys.txt"

    # Load manifest
    if not manifest_path.exists():
        print(f"Error: Manifest not found: {manifest_path}")
        sys.exit(1)

    with open(manifest_path) as f:
        data = json.load(f)
    meshes = data.get("meshes", [])
    print(f"Loaded {len(meshes)} mesh definitions")

    # Filter
    if args.only:
        only_ids = set(args.only.split(","))
        meshes = [m for m in meshes if m["id"] in only_ids]

    if args.test > 0:
        meshes = meshes[: args.test]

    print(f"Will process {len(meshes)} meshes")

    if args.dry_run:
        print("\n[DRY RUN] Would generate:")
        for m in meshes:
            print(f"  - {m['id']}: {m.get('text_prompt', '')[:50]}...")
        return

    # Load API keys
    keys = []
    if keys_file.exists():
        with open(keys_file) as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith("#") and not line.startswith("$"):
                    keys.append(line)

    if not keys:
        print("Error: No API keys in api_keys.txt")
        sys.exit(1)

    print(f"Found {len(keys)} API keys")

    # Run
    key_rotator = APIKeyRotator(keys)
    generator = MeshGenerator(key_rotator, output_dir)
    generator.run(meshes, limit=args.test)


if __name__ == "__main__":
    main()
