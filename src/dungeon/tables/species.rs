use std::collections::BTreeMap;

use crate::fishes::fish::Fish;
use crate::fishes::species::FishSpecies;

use crate::dungeon::kit::Kit;
use crate::dungeon::moves;
use crate::dungeon::traits::Trait;

pub const STING_PERCENT: u32 = 30;
pub const AFTERBURNER_DODGE: u32 = 25;
pub const REGROW_PERCENT: u32 = 8;
pub const AMBUSH_VENOM: u32 = 12;
pub const CONTACT_SPARK: u32 = 4;
pub const SEE_THROUGH_PERCENT: u32 = 60;
pub const ELECTROSENSE_CRIT: u32 = 10;
pub const PRIDE_CRIT: u32 = 15;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeepRole {
    Schooling,
    Undead,
    Sting,
    Afterburner,
    Upstream,
    Shell,
    Glitter,
    Regrow,
    HermitShell,
    Camouflage,
    Ambush,
    Hop,
    Ricochet,
    Heart,
    Lurk,
    Glide,
    Duelist,
    Chase,
    FalseEye,
    Lantern,
    Flash,
    NineLives,
    Eclipse,
    Echo,
    Mime,
    Hide,
    Contact,
    Puffer,
    Escape,
    SeeThrough,
    Shoal,
    Electrosense,
    Lucky,
    Cartographer,
    Venom,
    Kiss,
    Corrode,
    Saw,
    Greed,
    Wrath,
    Gluttony,
    Lust,
    Sloth,
    Junk,
    Pride,
    Toy,
    Unfish,
}

impl DeepRole {
    pub fn is_school(self) -> bool {
        self == DeepRole::Schooling
    }
}

pub fn fit(kit: &mut Kit, fish: &Fish) {
    let mut roles: BTreeMap<DeepRole, u32> = BTreeMap::new();
    for species in fish.ability_components() {
        *roles.entry(species.config().deep).or_default() += 1;
    }
    for (role, stacks) in roles {
        wear(kit, fish, role, stacks);
    }
}

pub fn role_of(species: FishSpecies) -> DeepRole {
    species.config().deep
}

pub fn wear(kit: &mut Kit, fish: &Fish, role: DeepRole, stacks: u32) {
    match role {
        DeepRole::Schooling => kit.add(Trait::Schooling),
        DeepRole::Undead => {
            kit.add(Trait::Undead);
            kit.skills.push(moves::play_dead());
        }
        DeepRole::Sting => kit.add(Trait::Sting(STING_PERCENT)),
        DeepRole::Afterburner => {
            kit.dodge = kit.dodge.max(AFTERBURNER_DODGE);
            kit.add(Trait::Afterburner);
        }
        DeepRole::Upstream => kit.add(Trait::Upstream),
        DeepRole::Shell => {
            kit.skills.push(moves::shell());
            kit.add(Trait::Slime);
        }
        DeepRole::Glitter => kit.add(Trait::Glitter),
        DeepRole::Regrow => kit.add(Trait::Regrow(REGROW_PERCENT)),
        DeepRole::HermitShell => {
            kit.add(Trait::JunkArmour);
            kit.skills.push(moves::hand_me_down());
        }
        DeepRole::Camouflage => kit.add(Trait::Camouflage),
        DeepRole::Ambush => kit.add(Trait::Ambush(AMBUSH_VENOM)),
        DeepRole::Hop => kit.skills.push(moves::hop()),
        DeepRole::Ricochet => kit.skills.push(moves::ricochet()),
        DeepRole::Heart => kit.skills.push(moves::heart()),
        DeepRole::Lurk => kit.skills.push(moves::lurk()),
        DeepRole::Glide => kit.add(Trait::Glide),
        DeepRole::Duelist => {
            kit.skills.push(moves::lunge());
            kit.skills.push(moves::duel());
        }
        DeepRole::Chase => kit.skills.push(moves::chase()),
        DeepRole::FalseEye => kit.add(Trait::FalseEye),
        DeepRole::Lantern => kit.add(Trait::Lantern),
        DeepRole::Flash => kit.skills.push(moves::flash()),
        DeepRole::NineLives => kit.add(Trait::NineLives),
        DeepRole::Eclipse => {
            kit.add(Trait::Lunar);
            kit.skills.push(moves::eclipse());
        }
        DeepRole::Echo => kit.add(Trait::Parrot),
        DeepRole::Mime => kit.skills.push(moves::mime()),
        DeepRole::Hide => kit.add(Trait::Hide),
        DeepRole::Contact => kit.add(Trait::Contact(CONTACT_SPARK)),
        DeepRole::Puffer => kit.skills.push(moves::puff_up()),
        DeepRole::Escape => {
            kit.add(Trait::Escape);
            kit.skills.push(moves::ink());
        }
        DeepRole::SeeThrough => kit.add(Trait::SeeThrough(SEE_THROUGH_PERCENT)),
        DeepRole::Shoal => kit.add(Trait::Shoal),
        DeepRole::Electrosense => {
            kit.crit = kit.crit.max(ELECTROSENSE_CRIT);
            kit.add(Trait::Electrosense);
            kit.skills.push(moves::hammer());
        }
        DeepRole::Lucky => kit.add(Trait::Lucky),
        DeepRole::Cartographer => kit.add(Trait::Cartographer),
        DeepRole::Venom => kit.skills.push(moves::venom()),
        DeepRole::Kiss => kit.skills.push(moves::kiss()),
        DeepRole::Corrode => {
            kit.add(Trait::Rusting);
            kit.skills.push(moves::corrode());
        }
        DeepRole::Saw => kit.skills.push(moves::saw()),
        DeepRole::Greed => {
            kit.add(Trait::Greedy(stacks));
            kit.skills.push(moves::make_it_rain(stacks));
            kit.skills.push(moves::pay_to_win());
            kit.skills.push(moves::bribe());
        }
        DeepRole::Wrath => {
            kit.add(Trait::Unstable);
            kit.add(Trait::Wrath);
            kit.bite += fish.mutation_count();
            kit.skills.push(moves::radiate());
            kit.skills.push(moves::meltdown());
        }
        DeepRole::Gluttony => {
            kit.add(Trait::Candy);
            kit.skills.push(moves::sugar_rush());
            kit.skills.push(moves::candify());
        }
        DeepRole::Lust => {
            kit.add(Trait::Blessed(stacks));
            kit.skills.push(moves::miracle());
            kit.skills.push(moves::smite());
        }
        DeepRole::Sloth => kit.add(Trait::Sloth),
        DeepRole::Junk => {
            kit.add(Trait::Junkfish);
            kit.skills.push(moves::throw_junk());
        }
        DeepRole::Pride => {
            kit.crit = kit.crit.max(PRIDE_CRIT);
            kit.add(Trait::Pride);
            kit.add(Trait::Telepathy);
        }
        DeepRole::Toy => super::toys::fit(kit, fish),
        DeepRole::Unfish => super::unfish::fit(kit, fish),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economy::Rarity;
    use crate::fishes::species::ALL_SPECIES;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    fn blank(fish: &Fish) -> Kit {
        let mut kit = Kit::of(fish);
        kit.traits.clear();
        kit.skills.clear();
        kit
    }

    #[test]
    fn a_common_is_a_common_and_nothing_else_is() {
        for &species in ALL_SPECIES {
            let config = species.config();
            let ordinary = config.rarity == Rarity::Common
                && config.flavour.sin.is_none()
                && config.habitat != crate::fishes::species::Habitat::Claw
                && config.habitat != crate::fishes::species::Habitat::Nowhere;
            assert_eq!(
                config.deep.is_school(),
                ordinary,
                "{} is {:?}",
                config.name,
                config.deep
            );
        }
    }

    #[test]
    fn every_species_row_changes_the_kit() {
        for &species in ALL_SPECIES {
            let mut rng = SmallRng::seed_from_u64(3);
            let fish = Fish::new(species, "Probe".into(), 0.0, 0.0, &mut rng);
            let before = blank(&fish);
            let mut after = before.clone();
            wear(&mut after, &fish, role_of(species), 1);
            assert_ne!(
                before,
                after,
                "{} does nothing in the Deep",
                species.display_name()
            );
        }
    }

    #[test]
    fn no_two_rares_share_a_role() {
        let mut seen = std::collections::BTreeSet::new();
        for &species in ALL_SPECIES {
            let config = species.config();
            if config.rarity == Rarity::Common {
                continue;
            }
            assert!(
                seen.insert(config.deep),
                "{} repeats {:?}",
                config.name,
                config.deep
            );
        }
    }
}
