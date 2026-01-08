#!/usr/bin/env python3
import os
import json
from pathlib import Path
from collections import defaultdict

SCRIPT_DIR = Path(__file__).parent

# Output directories
OUTPUT_DIRS = [
    SCRIPT_DIR / "output_batch",
    SCRIPT_DIR / "output_vla_new",
    SCRIPT_DIR / "output_industrial",
    SCRIPT_DIR / "output_bundeswehr"
]

# Manifests to check against
MANIFESTS = [
    SCRIPT_DIR / "mesh_manifest_industrial.json",
    SCRIPT_DIR / "mesh_manifest_vla_new.json",
    SCRIPT_DIR / "mesh_manifest_bundeswehr.json",
    SCRIPT_DIR / "mesh_manifest_bundeswehr_infantry.json",
    SCRIPT_DIR / "mesh_manifest_combined.json", 
    SCRIPT_DIR / "mesh_manifest_robotics_expanded.json"
]

def load_manifest_ids():
    planned_ids = set()
    for mf in MANIFESTS:
        if mf.exists():
            try:
                with open(mf) as f:
                    data = json.load(f)
                    for m in data.get("meshes", []):
                        planned_ids.add(m["id"])
            except Exception as e:
                print(f"Warning: Could not load {mf.name}: {e}")
    return planned_ids

def scan_directories():
    report = {
        "on_disk_count": 0,
        "zero_byte_files": [],
        "small_files": [], # < 1KB, likely error text
        "valid_assets": set(),
        "by_category": defaultdict(int)
    }

    print("Scanning output directories...")
    
    for out_dir in OUTPUT_DIRS:
        if not out_dir.exists():
            continue
            
        for cat_dir in out_dir.iterdir():
            if not cat_dir.is_dir(): continue
                
            for item_dir in cat_dir.iterdir():
                if not item_dir.is_dir(): continue
                
                item_id = item_dir.name
                glb_path = item_dir / f"{item_id}.glb"
                
                if glb_path.exists():
                    size = glb_path.stat().st_size
                    report["on_disk_count"] += 1
                    
                    if size == 0:
                        report["zero_byte_files"].append(str(glb_path))
                    elif size < 1024: # Less than 1KB is suspicious for a mesh
                        report["small_files"].append(str(glb_path))
                    else:
                        report["valid_assets"].add(item_id)
                        report["by_category"][cat_dir.name] += 1

    return report

def main():
    planned_ids = load_manifest_ids()
    scan_results = scan_directories()
    
    valid_ids = scan_results["valid_assets"]
    missing_ids = planned_ids - valid_ids
    
    # Generate MD Report
    md = "# Final Asset Integrity Report\n\n"
    
    md += "## Summary\n"
    md += f"- **Unique IDs in Manifests:** {len(planned_ids)}\n"
    md += f"- **Files on Disk (GLB):** {scan_results['on_disk_count']}\n"
    md += f"- **Valid Assets (>1KB):** {len(valid_ids)}\n"
    md += f"- **Missing / Not Generated:** {len(missing_ids)}\n"
    
    md += "\n## Health Issues\n"
    if scan_results["zero_byte_files"]:
        md += f"**CRITICAL: Found {len(scan_results['zero_byte_files'])} zero-byte files:**\n"
        for p in scan_results["zero_byte_files"]:
            md += f"- `{p}`\n"
    else:
        md += "- No zero-byte files found.\n"
        
    if scan_results["small_files"]:
        md += f"**WARNING: Found {len(scan_results['small_files'])} suspiciously small files (<1KB):**\n"
        for p in scan_results["small_files"]:
            md += f"- `{p}`\n"
    else:
        md += "- No suspiciously small files found.\n"
        
    md += "\n## Category Breakdown (Valid Assets)\n"
    sorted_cats = sorted(scan_results["by_category"].items(), key=lambda x: x[1], reverse=True)
    for cat, count in sorted_cats:
        md += f"- **{cat}:** {count}\n"
        
    md += "\n## Missing Items (Sample)\n"
    md += "> Items that were planned but not generated (likely due to credit exhaustion).\n\n"
    missing_list = list(missing_ids)
    missing_list.sort()
    for mid in missing_list[:50]:
        md += f"- {mid}\n"
    if len(missing_list) > 50:
        md += f"... and {len(missing_list) - 50} more.\n"

    with open("FINAL_INTEGRITY_REPORT.md", "w") as f:
        f.write(md)
        
    print(f"Report generated: FINAL_INTEGRITY_REPORT.md")
    print(f"Valid Assets: {len(valid_ids)} / {len(planned_ids)} Planned")
    print(f"Missing: {len(missing_ids)}")

if __name__ == "__main__":
    main()