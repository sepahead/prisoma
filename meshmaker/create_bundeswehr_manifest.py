#!/usr/bin/env python3
import json
import random

def create_manifest():
    meshes = []
    
    # 1. Armored Vehicles
    vehicles = [
        ("Leopard_2A7", "German main battle tank Leopard 2A7, desert camo"),
        ("Puma_IFV", "Bundeswehr Puma infantry fighting vehicle, forest camo"),
        ("Boxer_GTK", "GTK Boxer armored transport vehicle, modular mission module"),
        ("Fennek_Recon", "Fennek reconnaissance vehicle, light armored car"),
        ("Dingo_2", "ATF Dingo 2 protected patrol vehicle"),
        ("PzH_2000", "Panzerhaubitze 2000 self-propelled howitzer")
    ]
    for vid, vdesc in vehicles:
        meshes.append({
            "id": f"vehicle_{vid}",
            "category": "bundeswehr_vehicles",
            "prompt": f"{vdesc}, photorealistic, isolated, white background, military vehicle"
        })

    # 2. Air Defense Systems
    air_def = [
        ("Gepard_1A2", "Flakpanzer Gepard anti-aircraft gun tank, dual autocannons"),
        ("IRIS_T_SLM_Launcher", "IRIS-T SLM launcher vehicle, german air defense system"),
        ("Patriot_Launcher_Truck", "M901 Launching Station on MAN truck chassis, Luftwaffe markings"),
        ("MANTIS_Turret", "MANTIS air defence system turret, stationary C-RAM"),
        ("Ozelot_Wiesel_2", "LeFlaSys Ozelot on Wiesel 2 chassis, stinger launcher"),
        ("Radar_TRML_4D", "TRML-4D radar system truck, active electronically scanned array")
    ]
    for adid, addesc in air_def:
        meshes.append({
            "id": f"airdef_{adid}",
            "category": "bundeswehr_airdefense",
            "prompt": f"{addesc}, military engineering, detailed, isolated, white background"
        })

    # 3. Aircraft
    aircraft = [
        ("Eurofighter_Typhoon", "Eurofighter Typhoon, Luftwaffe grey livery, air superiority fighter"),
        ("Panavia_Tornado_IDS", "Panavia Tornado IDS, German Air Force, recon configuration"),
        ("Tiger_UHT_Helicopter", "Eurocopter Tiger UHT attack helicopter, anti-tank missiles"),
        ("NH90_TTH", "NH90 tactical transport helicopter, olive drab army aviation"),
        ("CH53_G_Stallion", "Sikorsky CH-53G heavy transport helicopter, Heer markings"),
        ("A400M_Atlas", "Airbus A400M Atlas military transport aircraft")
    ]
    for aid, adesc in aircraft:
         meshes.append({
            "id": f"aircraft_{aid}",
            "category": "bundeswehr_aircraft",
            "prompt": f"{adesc}, aircraft in flight configuration or landed, isolated, white background"
        })

    # 4. Infantry Gear & Weapons
    gear = [
        ("G36_Rifle", "Heckler & Koch G36 assault rifle, standard issue, optic handle"),
        ("Gefechtshelm_M92", "Bundeswehr combat helmet M92, flecktarn cover"),
        ("Flecktarn_Backpack", "Military backpack in 5-color Flecktarn camouflage"),
        ("Panzerfaust_3", "Panzerfaust 3 anti-tank weapon, loaded"),
        ("MG5_MachineGun", "Heckler & Koch MG5 general purpose machine gun"),
        ("SEM_70_Radio", "SEM 70 manpack military radio, vintage bundeswehr"),
        ("Combat_Boots_Gen2", "Bundeswehr combat boots model 2000, black leather"),
        ("Plate_Carrier_System", "Modern IdZ infantry vest, flecktarn camo")
    ]
    for gid, gdesc in gear:
        meshes.append({
            "id": f"gear_{gid}",
            "category": "bundeswehr_gear",
            "prompt": f"{gdesc}, military equipment, studio lighting, white background"
        })

    # 5. Logistics & Support
    logistics = [
        ("Unimog_U1300L", "Unimog U1300L military truck, canvas top, olive green"),
        ("MAN_KAT1_8x8", "MAN KAT1 8x8 heavy tactical truck"),
        ("Field_Kitchen_TFK250", "Taktische Feldkueche TFK 250, mobile field kitchen trailer"),
        ("Faltstrasse_Mat", "Military temporary road mat roll, engineering equipment"),
        ("Jerrycan_20L", "Classic 20L steel jerrycan, olive drab, bundeswehr markings")
    ]
    for lid, ldesc in logistics:
        meshes.append({
            "id": f"logistics_{lid}",
            "category": "bundeswehr_logistics",
            "prompt": f"{ldesc}, logistics equipment, used condition, white background"
        })
        
    # Naval
    naval = [
        ("Frigate_Baden_Wuerttemberg", "F125 class frigate Baden-Wuerttemberg, german navy"),
        ("Submarine_Type_212A", "Type 212A submarine, deutsche marine, fuel cell propulsion"),
        ("Corvette_Braunschweig", "K130 Braunschweig class corvette")
    ]
    for nid, ndesc in naval:
        meshes.append({
            "id": f"naval_{nid}",
            "category": "bundeswehr_naval",
            "prompt": f"{ndesc}, warship, waterline model, white background"
        })

    manifest = {
        "version": "1.0",
        "description": "Bundeswehr and German Military Equipment",
        "meshes": meshes
    }
    
    print(f"Generated {len(meshes)} mesh definitions.")
    
    with open("meshmaker/mesh_manifest_bundeswehr.json", "w") as f:
        json.dump(manifest, f, indent=2)

if __name__ == "__main__":
    create_manifest()
