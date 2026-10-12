use crate::fishes::fish::Fish;
use crate::fishes::parts::Part;

use crate::dungeon::kit::Kit;
use crate::dungeon::moves;
use crate::dungeon::traits::Trait;

pub fn wear(kit: &mut Kit, part: Part) {
    match part {
        Part::InverterCoil => kit.skills.push(moves::invert()),
        Part::DelaySpool => kit.skills.push(moves::charge()),
        Part::ShoalCounter => kit.add(Trait::ShoalCounter),
        Part::TidalClock => kit.add(Trait::TidalClock),
        Part::AssayScale => kit.add(Trait::Assay),
        Part::LedgerNerve => kit.add(Trait::LedgerNerve),
        Part::GeigerCoil => kit.add(Trait::Geiger),
        Part::StartleNerve => kit.add(Trait::Startle),
        Part::CommandModule => {
            kit.add(Trait::Commanded);
            kit.skills.push(moves::script());
        }
        Part::Cochlea => kit.add(Trait::Cochlea),
        Part::GlyphPanel => kit.add(Trait::Glyph),
        Part::ReflexArc => kit.add(Trait::Reflex),
        Part::CoreStack => kit.add(Trait::CoreStack),
        Part::CathodeArray => kit.add(Trait::Decoy),
        Part::RelayMast => kit.add(Trait::Uplink),
        Part::AnglerRig => kit.add(Trait::Angler),
    }
}

pub fn fit(kit: &mut Kit, fish: &Fish) {
    let Some(board) = fish.script() else {
        return;
    };
    for (part, _) in board.hardware() {
        wear(kit, part);
    }
    if board.chip().is_some() {
        kit.add(Trait::Sealed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fishes::species::FishSpecies;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    #[test]
    fn every_botfish_part_changes_the_kit() {
        let mut rng = SmallRng::seed_from_u64(9);
        let fish = Fish::new(FishSpecies::Botfish, "Probe".into(), 0.0, 0.0, &mut rng);
        for &part in Part::ALL {
            let mut kit = Kit::of(&fish);
            let before = kit.clone();
            wear(&mut kit, part);
            assert_ne!(before, kit, "{part:?} does nothing in the Deep");
        }
    }

    #[test]
    fn an_installed_part_reaches_the_kit() {
        let mut rng = SmallRng::seed_from_u64(10);
        let mut fish = Fish::new(FishSpecies::Botfish, "Probe".into(), 0.0, 0.0, &mut rng);
        assert!(!Kit::of(&fish).has(Trait::TidalClock));
        fish.script_mut()
            .expect("a botfish is programmable")
            .install(Part::TidalClock);
        assert!(Kit::of(&fish).has(Trait::TidalClock));
    }
}
