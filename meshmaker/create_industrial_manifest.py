#!/usr/bin/env python3
import json
import random

def create_manifest():
    meshes = []
    
    # --- Categories and Combinations ---
    
    # 1. Fasteners (approx 150)
    fastener_types = ["hex_bolt", "socket_head_screw", "countersunk_screw", "hex_nut", "wing_nut", "washer_flat", "washer_split", "rivet"]
    materials = ["stainless_steel", "black_oxide", "brass", "zinc_plated"]
    sizes = ["M3_small", "M6_medium", "M12_large", "long", "short"]
    
    for ft in fastener_types:
        for mat in materials:
            for sz in sizes:
                # Randomize slightly to avoid exact duplicates if needed, but combinatorial is fine here
                meshes.append({
                    "id": f"fastener_{ft}_{mat}_{sz}",
                    "category": "industrial_fasteners",
                    "prompt": f"{mat.replace('_', ' ')} {ft.replace('_', ' ')}, {sz.replace('_', ' ')}, industrial standard, isolated, photorealistic, white background"
                })

    # 2. Gears & Transmission (approx 100)
    gear_types = ["spur_gear", "helical_gear", "bevel_gear", "worm_gear", "rack_gear", "timing_pulley"]
    gear_materials = ["steel", "brass", "white_nylon"]
    gear_specs = ["10_teeth", "24_teeth", "50_teeth", "hollow_bore", "shaft_mounted"]
    
    for gt in gear_types:
        for gm in gear_materials:
            for gs in gear_specs:
                 if random.random() < 0.8: # trim slightly
                    meshes.append({
                        "id": f"gear_{gt}_{gm}_{gs}",
                        "category": "mechanical_transmission",
                        "prompt": f"{gm.replace('_', ' ')} {gt.replace('_', ' ')}, {gs.replace('_', ' ')}, engineering part, macro photography, white background"
                    })

    # 3. Bearings (approx 50)
    bearing_types = ["ball_bearing", "roller_bearing", "thrust_bearing", "pillow_block_bearing", "flange_bearing"]
    bearing_styles = ["sealed_rubber", "shielded_metal", "open_cage"]
    
    for bt in bearing_types:
        for bs in bearing_styles:
             meshes.append({
                "id": f"bearing_{bt}_{bs}",
                "category": "mechanical_bearings",
                "prompt": f"Industrial {bt.replace('_', ' ')}, {bs.replace('_', ' ')}, shiny metal, grease, detailed, white background"
            })

    # 4. Electronics - Passive (approx 100)
    passives = ["resistor_axial", "capacitor_electrolytic", "capacitor_ceramic_disc", "inductor_toroid", "transformer_small"]
    passive_specs = ["blue", "brown", "green_coating", "large_power", "smd_style"]
    
    for p in passives:
        for ps in passive_specs:
             for i in range(3): # a few variations
                meshes.append({
                    "id": f"elec_passive_{p}_{ps}_v{i}",
                    "category": "electronics_passive",
                    "prompt": f"Electronic component {p.replace('_', ' ')}, {ps.replace('_', ' ')}, variation {i+1}, macro shot, white background"
                })

    # 5. Electronics - Connectors (approx 100)
    connectors = ["usb_type_a_male", "usb_type_c_female", "rj45_jack", "hdmi_port", "db9_connector", "terminal_block_2pin", "molex_4pin", "xt60_connector", "banana_plug"]
    conn_views = ["front_view", "angled_view", "pcb_mount"]
    
    for c in connectors:
        for cv in conn_views:
             for i in range(3):
                meshes.append({
                    "id": f"conn_{c}_{cv}_{i}",
                    "category": "electronics_connectors",
                    "prompt": f"Electrical connector {c.replace('_', ' ')}, {cv.replace('_', ' ')}, metallic contacts, plastic housing, white background"
                })

    # 6. Motors & Actuators (approx 60)
    motors = ["stepper_motor_nema17", "dc_motor_small", "servo_motor_standard", "brushless_drone_motor", "linear_actuator"]
    motor_views = ["shaft_up", "side_profile", "connector_view"]
    
    for m in motors:
        for mv in motor_views:
             for i in range(4):
                meshes.append({
                    "id": f"motor_{m}_{mv}_{i}",
                    "category": "motors_actuators",
                    "prompt": f"{m.replace('_', ' ')}, {mv.replace('_', ' ')}, industrial motor, metal casing, wires visible, white background"
                })

    # 7. Industrial Sensors (approx 50)
    sensors = ["inductive_proximity_sensor", "limit_switch_roller", "photoelectric_sensor", "pressure_transducer", "thermocouple_probe"]
    
    for s in sensors:
         for i in range(10):
            meshes.append({
                "id": f"sensor_{s}_v{i}",
                "category": "industrial_sensors",
                "prompt": f"Industrial automation sensor, {s.replace('_', ' ')}, variation {i}, threaded body, cable tail, white background"
            })

    # 8. Structural & Enclosures (approx 100)
    structs = ["aluminum_extrusion_2020", "steel_angle_bracket", "corner_brace", "din_rail_segment", "cable_carrier_chain"]
    boxes = ["abs_project_box", "aluminum_diecast_enclosure", "electrical_junction_box"]
    
    for s in structs:
        for i in range(10):
             meshes.append({
                "id": f"struct_{s}_len{i}",
                "category": "structural_components",
                "prompt": f"{s.replace('_', ' ')}, length variant {i}, industrial construction part, silver metal, white background"
            })
            
    for b in boxes:
        for i in range(5):
             meshes.append({
                "id": f"enclosure_{b}_v{i}",
                "category": "enclosures",
                "prompt": f"{b.replace('_', ' ')}, industrial housing, variation {i}, screws, detailed texture, white background"
            })

    # 9. Tools (approx 50)
    tools = ["digital_calipers", "micrometer", "allen_key_set", "wire_strippers", "multimeter", "soldering_iron_tip"]
    
    for t in tools:
        for i in range(8):
            meshes.append({
                "id": f"tool_{t}_v{i}",
                "category": "industrial_tools",
                "prompt": f"Industrial tool {t.replace('_', ' ')}, used condition, workshop context but isolated, white background"
            })

    # Shuffle to distribute load across categories if processed sequentially (though queue handles this)
    random.shuffle(meshes)
    
    # Trim or pad if necessary (aiming for ~800)
    # Current logic generates roughly: 
    # Fasteners: 8 * 4 * 5 = 160
    # Gears: 6 * 3 * 5 = 90
    # Bearings: 5 * 3 = 15
    # Passives: 5 * 5 * 3 = 75
    # Connectors: 9 * 3 * 3 = 81
    # Motors: 5 * 3 * 4 = 60
    # Sensors: 5 * 10 = 50
    # Struct: 5 * 10 = 50
    # Enclosures: 3 * 5 = 15
    # Tools: 6 * 8 = 48
    # Total: ~644. Let's add more ICs/Chips and Switches to hit 800.
    
    # 10. ICs & Chips (approx 100)
    packages = ["DIP8_ic", "DIP16_ic", "QFP44_ic", "BGA_chip", "TO220_transistor"]
    for p in packages:
        for i in range(20):
             meshes.append({
                "id": f"ic_{p}_v{i}",
                "category": "electronics_ics",
                "prompt": f"Electronic component {p.replace('_', ' ')}, macro view, black epoxy, metal pins, silicon chip, white background"
            })

    # 11. Switches & LEDs (approx 60)
    switches = ["toggle_switch", "push_button_industrial", "rocker_switch", "led_5mm_red", "led_5mm_green"]
    for s in switches:
        for i in range(12):
             meshes.append({
                "id": f"switch_{s}_v{i}",
                "category": "electromechanical",
                "prompt": f"{s.replace('_', ' ')}, electrical component, detailed macro, metal and plastic, white background"
            })

    # Final shuffle
    random.shuffle(meshes)
    
    manifest = {
        "version": "1.0",
        "description": "Industrial Manufacturing and Electronics Dataset",
        "meshes": meshes
    }
    
    print(f"Generated {len(meshes)} mesh definitions.")
    
    with open("meshmaker/mesh_manifest_industrial.json", "w") as f:
        json.dump(manifest, f, indent=2)

if __name__ == "__main__":
    create_manifest()
