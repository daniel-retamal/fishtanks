use crate::fishes::fish::Fish;

use crate::dungeon::boon::{Boon, Charm, Curse};
use crate::dungeon::kit::{Kit, MIN_SPEED};
use crate::dungeon::life::DeepLife;
use crate::dungeon::traits::Trait;

pub const SHARP_SCALES: u32 = 3;
pub const WANTING_PERCENT: u32 = 10;
pub const WRONG_WORD_CRIT: u32 = 10;
pub const LANTERN_SPEED: u32 = 1;
pub const RIB_ARMOUR: u32 = 4;
const PERCENT: u32 = 100;

pub fn wear_boon(kit: &mut Kit, boon: Boon) {
    match boon {
        Boon::SecondWind => kit.add(Trait::SecondWind),
        Boon::SharpScales => kit.add(Trait::Thorns(SHARP_SCALES)),
        Boon::DeepLungs => kit.max_mp += 1,
    }
}

pub fn wear_curse(kit: &mut Kit, curse: Curse) {
    match curse {
        Curse::HungryGhost => {
            if !kit.full {
                kit.bite = kit.bite.saturating_sub(1);
            }
            kit.add(Trait::HungryGhost);
        }
        Curse::Brittle => kit.add(Trait::Soft),
        Curse::HeavyFins => kit.speed = kit.speed.saturating_sub(1).max(MIN_SPEED),
        Curse::FinePrint => kit.add(Trait::FinePrint),
        Curse::Wanting => kit.vigor = (kit.vigor * (PERCENT - WANTING_PERCENT) / PERCENT).max(1),
        Curse::WrongWord => kit.add(Trait::SoftEyes(WRONG_WORD_CRIT)),
        Curse::Doomed => kit.add(Trait::Doomed),
    }
}

pub fn wear_charm(kit: &mut Kit, fish: &Fish, charm: Charm) {
    match charm {
        Charm::CaleuchesLantern => {
            kit.speed += LANTERN_SPEED;
            kit.add(Trait::SilentSteps);
        }
        Charm::KrakensBeak => kit.add(Trait::Drill),
        Charm::MillalobosCrown => kit.add(Trait::Crown),
        Charm::SweetTooth => kit.add(Trait::SweetTooth),
        Charm::LeviathansRib => kit.armour += RIB_ARMOUR,
        Charm::TractorBeam => {
            kit.add(Trait::Walks);
            kit.add(Trait::SeenTheSky);
        }
        Charm::CoreFragment => kit.bite += fish.mutation_count().max(1),
        Charm::DaemonsFork => kit.add(Trait::Forked),
        Charm::RootAccess => kit.add(Trait::Telepathy),
        Charm::DevilsCoin => kit.add(Trait::DevilsCoin),
        Charm::Halo => kit.add(Trait::Holy),
        Charm::HeraldsTrumpet => kit.add(Trait::Trumpet),
    }
}

pub fn fit(kit: &mut Kit, fish: &Fish, life: &DeepLife) {
    for &boon in &life.boons {
        wear_boon(kit, boon);
    }
    for &curse in &life.curses {
        wear_curse(kit, curse);
    }
    if let Some(charm) = life.charm {
        wear_charm(kit, fish, charm);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fishes::species::FishSpecies;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    fn probe() -> (Fish, Kit) {
        let mut rng = SmallRng::seed_from_u64(12);
        let fish = Fish::new(FishSpecies::Merluza, "Probe".into(), 0.0, 0.0, &mut rng);
        let kit = Kit::of(&fish);
        (fish, kit)
    }

    #[test]
    fn every_boon_curse_and_charm_changes_the_kit() {
        let (fish, kit) = probe();
        for boon in Boon::ALL {
            let mut worn = kit.clone();
            wear_boon(&mut worn, boon);
            assert_ne!(worn, kit, "{boon:?}");
        }
        for curse in Curse::ALL {
            let mut worn = kit.clone();
            wear_curse(&mut worn, curse);
            assert_ne!(worn, kit, "{curse:?}");
        }
        for charm in Charm::ALL {
            let mut worn = kit.clone();
            wear_charm(&mut worn, &fish, charm);
            assert_ne!(worn, kit, "{charm:?}");
        }
    }
}
