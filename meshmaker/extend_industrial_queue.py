#!/usr/bin/env python3
import json
import random
import fcntl
from pathlib import Path

# Paths
SCRIPT_DIR = Path("meshmaker")
QUEUE_FILE = SCRIPT_DIR / "mesh_queue_industrial.json"
LOCK_FILE = SCRIPT_DIR / "mesh_queue_industrial.lock"
MANIFEST_FILE = SCRIPT_DIR / "mesh_manifest_industrial.json"

# Output dirs to check for existence
OUTPUT_DIRS = [
    SCRIPT_DIR / "output_batch",
    SCRIPT_DIR / "output_vla_new",
    SCRIPT_DIR / "output_industrial"
]

def get_existing_ids():
    existing = set()
    for d in OUTPUT_DIRS:
        if d.exists():
            for glb in d.glob("**/*.glb"):
                existing.add(glb.stem)
    return existing

def generate_extra_meshes(existing_ids):
    meshes = []
    
    # 1. Pneumatics (approx 80)
    pneu_types = ["air_cylinder_iso6432", "solenoid_valve_5_2", "push_in_fitting_elbow", "pneumatic_muffler_sintered", "air_preparation_frl"]
    for pt in pneu_types:
        for i in range(16):
            mid = f"pneumatic_{pt}_{i}"
            if mid not in existing_ids:
                meshes.append({
                    "id": mid,
                    "category": "industrial_pneumatics",
                    "prompt": f"Industrial pneumatic component, {pt.replace('_', ' ')}, variation {i}, metal and blue plastic, studio lighting, white background"
                })

    # 2. Hydraulics (approx 80)
    hyd_types = ["hydraulic_hose_crimped", "hydraulic_pump_gear", "hydraulic_cylinder_heavy_duty", "quick_coupler_female", "pressure_gauge_liquid_filled"]
    for ht in hyd_types:
        for i in range(16):
            mid = f"hydraulic_{ht}_{i}"
            if mid not in existing_ids:
                meshes.append({
                    "id": mid,
                    "category": "industrial_hydraulics",
                    "prompt": f"Heavy duty hydraulic part, {ht.replace('_', ' ')}, variation {i}, oily metal, yellow zinc plating, white background"
                })

    # 3. Safety Equipment (approx 60)
    safety_items = ["lockout_tagout_hasp", "emergency_stop_button_box", "safety_relay_module", "warning_light_tower", "industrial_fuse_link"]
    for si in safety_items:
        for i in range(12):
            mid = f"safety_{si}_{i}"
            if mid not in existing_ids:
                meshes.append({
                    "id": mid,
                    "category": "industrial_safety",
                    "prompt": f"Industrial safety equipment, {si.replace('_', ' ')}, variation {i}, bright safety colors, red/yellow, white background"
                })

    # 4. Lab & Test (approx 80)
    lab_items = ["oscilloscope_probe_tip", "bnc_connector_panel", "test_tube_rack_industrial", "petri_dish_glass", "pipette_tip_box"]
    for li in lab_items:
        for i in range(16):
            mid = f"lab_{li}_{i}"
            if mid not in existing_ids:
                meshes.append({
                    "id": mid,
                    "category": "industrial_lab",
                    "prompt": f"Scientific lab equipment, {li.replace('_', ' ')}, variation {i}, clean, sterile look, white background"
                })

    # 5. Raw Materials (approx 100)
    raw_mats = ["aluminum_ingot", "copper_coil_wire", "steel_rebar_section", "carbon_fiber_sheet_sample", "brass_hex_bar_cut"]
    for rm in raw_mats:
        for i in range(20):
            mid = f"material_{rm}_{i}"
            if mid not in existing_ids:
                meshes.append({
                    "id": mid,
                    "category": "industrial_materials",
                    "prompt": f"Raw industrial material sample, {rm.replace('_', ' ')}, variation {i}, realistic texture, white background"
                })

    random.shuffle(meshes)
    return meshes

def update_files(new_meshes):
    # 1. Update Manifest (Append)
    with open(MANIFEST_FILE, 'r+') as f:
        data = json.load(f)
        data["meshes"].extend(new_meshes)
        f.seek(0)
        json.dump(data, f, indent=2)
        f.truncate()
    
    # 2. Update Queue (Append Safely)
    print(f"Waiting for lock on {QUEUE_FILE}...")
    with open(LOCK_FILE, 'w') as lock_file:
        fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX)
        try:
            if QUEUE_FILE.exists():
                with open(QUEUE_FILE, 'r') as f:
                    queue = json.load(f)
            else:
                queue = []
            
            queue.extend(new_meshes)
            
            with open(QUEUE_FILE, 'w') as f:
                json.dump(queue, f)
            
            print(f"Successfully added {len(new_meshes)} new items to the active queue.")
            print(f"Total queue length: {len(queue)}")
            
        finally:
            fcntl.flock(lock_file.fileno(), fcntl.LOCK_UN)

def main():
    print("Scanning existing meshes...")
    existing = get_existing_ids()
    print(f"Found {len(existing)} existing mesh IDs.")
    
    print("Generating supplemental mesh definitions...")
    new_meshes = generate_extra_meshes(existing)
    print(f"Generated {len(new_meshes)} unique new definitions.")
    
    if new_meshes:
        update_files(new_meshes)
    else:
        print("No new meshes needed.")

if __name__ == "__main__":
    main()
