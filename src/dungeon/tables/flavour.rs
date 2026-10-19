use crate::fishes::fish::Fish;
use crate::fishes::species::{Flavour, Fortune, Sin};

use crate::dungeon::kit::Kit;
use crate::dungeon::traits::Trait;

pub const SLOTH_BUBBLES: u32 = 2;
pub const DELICIOUS: &str = "Yes.";
pub const FORBIDDEN: &str = "Forbidden";
pub const NOOO: &str = "NOOOOOOOOOO";

pub fn wear_sin(kit: &mut Kit, sin: Sin) {
    match sin {
        Sin::Greed => kit.add(Trait::GreedSin),
        Sin::Lust => kit.add(Trait::LustSin),
        Sin::Gluttony => kit.add(Trait::GluttonySin),
        Sin::Wrath => kit.add(Trait::WrathSin),
        Sin::Envy => kit.add(Trait::EnvySin),
        Sin::Pride => kit.add(Trait::PrideSin),
        Sin::Sloth => {
            kit.start_mp += SLOTH_BUBBLES;
            kit.add(Trait::SlothSin);
        }
    }
}

pub fn wear_fortune(kit: &mut Kit, fortune: Fortune) {
    match fortune {
        Fortune::Golden => kit.add(Trait::Golden),
        Fortune::Doomed => kit.add(Trait::Doomed),
    }
}

pub fn delicious(word: &str) -> Option<Trait> {
    match word {
        DELICIOUS => Some(Trait::Delicious),
        FORBIDDEN => Some(Trait::Forbidden),
        NOOO => Some(Trait::Nooo),
        _ => None,
    }
}

pub fn wear_flavour(kit: &mut Kit, flavour: &Flavour) {
    if let Some(fortune) = flavour.fortune {
        wear_fortune(kit, fortune);
    }
    if let Some(taste) = flavour.delicious.and_then(delicious) {
        kit.add(taste);
    }
    if flavour.has_seen_the_sky {
        kit.add(Trait::SeenTheSky);
    }
    if let Some(colour) = flavour.favorite_color {
        kit.add(Trait::FavouriteColour(colour));
    }
    kit.add(Trait::Voice(flavour.voice));
}

pub fn fit(kit: &mut Kit, fish: &Fish) {
    if let Some(sin) = fish.sin() {
        wear_sin(kit, sin);
    }
    wear_flavour(kit, &fish.species.config().flavour);
    if fish.devil_marked {
        kit.add(Trait::DevilMarked);
    }
    if fish.is_holy() {
        kit.add(Trait::Holy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dungeon::life::Grade;

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
            grade: Grade::Fry,
            hungry: false,
            full: false,
            traits: Vec::new(),
            skills: Vec::new(),
        }
    }

    #[test]
    fn every_sin_and_every_fortune_changes_the_kit() {
        for sin in Sin::ALL {
            let mut kit = blank();
            wear_sin(&mut kit, sin);
            assert_ne!(kit, blank(), "{sin:?}");
        }
        for fortune in [Fortune::Golden, Fortune::Doomed] {
            let mut kit = blank();
            wear_fortune(&mut kit, fortune);
            assert_ne!(kit, blank(), "{fortune:?}");
        }
    }

    #[test]
    fn every_delicious_word_the_game_writes_means_something_down_here() {
        for &species in crate::fishes::species::ALL_SPECIES {
            if let Some(word) = species.config().flavour.delicious {
                assert!(delicious(word).is_some(), "{word}");
            }
        }
    }
}
