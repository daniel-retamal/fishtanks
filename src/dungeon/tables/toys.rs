use crate::fishes::fish::Fish;
use crate::fishes::toy::{Look, Material, ToyPart, ToyState};

use crate::dungeon::kit::{Kit, MIN_SPEED};
use crate::dungeon::moves;
use crate::dungeon::traits::Trait;

pub const WHEELS_SPEED: u32 = 3;
pub const TREADS_ARMOUR: u32 = 4;
pub const PROPELLER_SPEED: u32 = 2;
pub const PROPELLER_DODGE: u32 = 10;
pub const LIGHT_DODGE: u32 = 5;
pub const PLATED: u32 = 3;
pub const LUSTRE: u32 = 3;
pub const SPARKLE_PERCENT: u32 = 20;
pub const PHASE_PERCENT: u32 = 30;

pub fn material_vigor(material: Material) -> u32 {
    match material {
        Material::Plastic => 40,
        Material::Pearl => 50,
        Material::Metallic => 70,
        Material::Glitter => 50,
        Material::Holographic => 60,
    }
}

pub fn wear_material(kit: &mut Kit, material: Material) {
    match material {
        Material::Plastic => {
            kit.dodge += LIGHT_DODGE;
            kit.add(Trait::Light);
        }
        Material::Pearl => kit.add(Trait::Lustre(LUSTRE)),
        Material::Metallic => {
            kit.armour += PLATED;
            kit.add(Trait::Plated(PLATED));
        }
        Material::Glitter => kit.add(Trait::Sparkle(SPARKLE_PERCENT)),
        Material::Holographic => kit.add(Trait::Phase(PHASE_PERCENT)),
    }
}

pub fn wear_part(kit: &mut Kit, part: ToyPart) {
    match part {
        ToyPart::Lure => kit.add(Trait::Lure),
        ToyPart::Bill => kit.add(Trait::Bill),
        ToyPart::Drill => kit.add(Trait::Drill),
        ToyPart::DorsalFin => kit.add(Trait::SharkFin),
        ToyPart::Spikes => kit.add(Trait::Spikes),
        ToyPart::Wings => kit.add(Trait::Wings),
        ToyPart::Antenna => kit.add(Trait::Overhears),
        ToyPart::Horn => kit.skills.push(moves::charge()),
        ToyPart::Rotor => kit.add(Trait::Hover),
        ToyPart::Crown => kit.add(Trait::Crown),
        ToyPart::Siren => kit.skills.push(moves::alarm()),
        ToyPart::VentralFin => kit.add(Trait::Keel),
        ToyPart::Feet => kit.add(Trait::Walks),
        ToyPart::SmallBoots => kit.skills.push(moves::kick()),
        ToyPart::Boots => kit.skills.push(moves::stomp()),
        ToyPart::Wheels => kit.speed += WHEELS_SPEED,
        ToyPart::Treads => {
            kit.armour += TREADS_ARMOUR;
            kit.speed = MIN_SPEED;
            kit.add(Trait::Treads);
        }
        ToyPart::Rocket => kit.skills.push(moves::blast_off()),
        ToyPart::Propeller => {
            kit.speed += PROPELLER_SPEED;
            kit.dodge += PROPELLER_DODGE;
        }
        ToyPart::WindUpKey => kit.add(Trait::WindUp),
    }
}

pub fn wear_toy(kit: &mut Kit, toy: &ToyState) {
    kit.add(Trait::Construct);
    wear_material(kit, toy.material());
    for fitted in toy.fittings().worn() {
        wear_part(kit, fitted.part);
    }
    match toy.look {
        Look::Signature { signature, .. } => kit.add(Trait::Signature(signature.line())),
        Look::Golden => kit.add(Trait::GoldenToy),
        Look::Plain { .. } => {}
    }
}

pub fn fit(kit: &mut Kit, fish: &Fish) {
    if let Some(toy) = fish.toy.as_deref() {
        wear_toy(kit, toy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fishes::toy::{Line, Signature};

    fn blank() -> Kit {
        Kit {
            name: String::new(),
            vigor: 50,
            bite: 6,
            speed: 6,
            dodge: 6,
            crit: 5,
            max_mp: 3,
            start_mp: 1,
            armour: 0,
            grade: crate::dungeon::life::Grade::Fry,
            hungry: false,
            full: false,
            traits: Vec::new(),
            skills: Vec::new(),
        }
    }

    #[test]
    fn every_material_and_every_toy_part_changes_the_kit() {
        for material in Material::ALL {
            let mut kit = blank();
            wear_material(&mut kit, material);
            assert_ne!(kit, blank(), "{material:?}");
        }
        for part in ToyPart::ALL {
            let mut kit = blank();
            wear_part(&mut kit, part);
            assert_ne!(kit, blank(), "{part:?}");
        }
    }

    #[test]
    fn a_signature_carries_its_line() {
        let mut kit = blank();
        wear_toy(&mut kit, &ToyState::signature(Signature::Kaiju, false));
        assert!(kit.has(Trait::Signature(Line::Kaiju)));
        assert!(kit.has(Trait::Construct));
    }
}
