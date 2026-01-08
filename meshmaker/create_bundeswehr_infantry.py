#!/usr/bin/env python3
import json

def create_manifest():
    meshes = []
    
    # Categories: Heer, Luftwaffe, Marine, Sanitätsdienst, SKB, CIR, KSK
    
    # 1. Heer (Army) - The bulk of infantry
    heer_units = [
        ("Panzergrenadier_Dismounted", "German Panzergrenadier soldier, IdZ vest, G36, flecktarn, running pose"),
        ("Jaeger_Light_Infantry", "Bundeswehr Jäger soldier, forest camo, patrolling, Boonie hat"),
        ("Gebirgsjaeger_Alpine", "German mountain trooper (Gebirgsjäger), Edelweiss patch, heavy backpack, snow camo elements"),
        ("Fallschirmjaeger_Paratrooper", "German paratrooper (Fallschirmjäger), jump helmet, knee pads, G36K"),
        ("Panzer_Crewman", "Leopard 2 tank crewman, olive tank suit, beret, headset"),
        ("KSK_Operator_Combat", "KSK special forces operator, multicam/flecktarn mix, high-cut helmet, G95 rifle, tactical"),
        ("Recon_Scout", "Heeresaufklärungstruppe scout, Fennek crew dismounted, light gear, binoculars")
    ]
    
    for uid, desc in heer_units:
        meshes.append({
            "id": f"heer_{uid}",
            "category": "bundeswehr_infantry_heer",
            "prompt": f"{desc}, photorealistic, full body character, isolated, white background, military uniform detail"
        })

    # 2. Luftwaffe (Air Force)
    luft_units = [
        ("Objektschutz_Soldier", "Luftwaffe Objektschutzregiment soldier, infantry gear, guarding posture, air force beret"),
        ("Eurofighter_Pilot", "German fighter pilot, full flight suit, g-suit, helmet under arm"),
        ("Ground_Crew_Technician", "Luftwaffe ground crew, safety vest, ear protection, guiding marshalling wands")
    ]
    
    for uid, desc in luft_units:
        meshes.append({
            "id": f"luftwaffe_{uid}",
            "category": "bundeswehr_infantry_luftwaffe",
            "prompt": f"{desc}, german air force personnel, detailed uniform, isolated, white background"
        })

    # 3. Marine (Navy)
    marine_units = [
        ("Seebataillon_Boarding", "German Navy Seebataillon soldier, boarding team, dark blue/camo mix, MP7, tactical vest"),
        ("Marine_Officer_Dress", "Deutsche Marine officer, service dress blue uniform, peaked cap, standing attention"),
        ("Deck_Crew_Sailor", "German navy sailor, shipboard working uniform (Bordparka), life vest"),
        ("Combat_Swimmer_KSM", "Kommando Spezialkräfte Marine diver, wetsuit, rebreather, underwater gear")
    ]
    
    for uid, desc in marine_units:
        meshes.append({
            "id": f"marine_{uid}",
            "category": "bundeswehr_infantry_marine",
            "prompt": f"{desc}, german navy personnel, photorealistic character, isolated, white background"
        })

    # 4. Sanitätsdienst (Medical)
    med_units = [
        ("Combat_Medic_Field", "Bundeswehr combat medic (Sanitäter), red cross patch, medic backpack, treating kneeling posture"),
        ("Medical_Officer_Doctor", "German military doctor, Flecktarn field uniform, stethoscope, standing")
    ]
    
    for uid, desc in med_units:
        meshes.append({
            "id": f"sanitaet_{uid}",
            "category": "bundeswehr_infantry_medical",
            "prompt": f"{desc}, german medical corps, military medicine, isolated, white background"
        })

    # 5. Streitkräftebasis (Support/Logistics)
    skb_units = [
        ("Feldjaeger_MP", "German Military Police (Feldjäger), orange brassard 'MP', white belt, traffic control baton"),
        ("Logistics_Soldier_Crate", "Bundeswehr logistics soldier carrying supply crate, field cap")
    ]
    
    for uid, desc in skb_units:
        meshes.append({
            "id": f"skb_{uid}",
            "category": "bundeswehr_infantry_support",
            "prompt": f"{desc}, german support troops, detailed gear, isolated, white background"
        })

    # 6. Cyber & Information Space (CIR)
    cir_units = [
        ("Cyber_Ops_Soldier", "Bundeswehr CIR soldier, field uniform, working on rugged military laptop, tactical environment"),
        ("EloKa_Specialist", "Electronic warfare specialist, radio equipment, antenna, headphones")
    ]
    
    for uid, desc in cir_units:
        meshes.append({
            "id": f"cir_{uid}",
            "category": "bundeswehr_infantry_cyber",
            "prompt": f"{desc}, german cyber command personnel, modern equipment, isolated, white background"
        })
        
    # 7. Historical / Tradition (Wachbataillon)
    wach_units = [
        ("Wachbataillon_Guard", "Wachbataillon soldier, ceremonial uniform, Karabiner 98k, presenting arms, drill protocol")
    ]
    
    for uid, desc in wach_units:
        meshes.append({
            "id": f"wach_{uid}",
            "category": "bundeswehr_infantry_ceremonial",
            "prompt": f"{desc}, german ceremonial guard, pristine uniform, isolated, white background"
        })

    manifest = {
        "version": "1.0",
        "description": "Bundeswehr Infantry - All Branches",
        "meshes": meshes
    }
    
    print(f"Generated {len(meshes)} infantry definitions.")
    
    with open("meshmaker/mesh_manifest_bundeswehr_infantry.json", "w") as f:
        json.dump(manifest, f, indent=2)

if __name__ == "__main__":
    create_manifest()
