use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::fishes::fish::Fish;
use crate::vault::Burial;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Fate {
    Sold,
    Killed,
    Absorbed,
    Dive,
}

impl Fate {
    pub fn epitaph(self) -> &'static str {
        match self {
            Fate::Sold => "sold",
            Fate::Killed => "killed",
            Fate::Absorbed => "absorbed",
            Fate::Dive => "died on a dive",
        }
    }
}

#[derive(Default)]
pub struct Graveyard {
    dead: Vec<Fish>,
    entombed: usize,
}

impl Graveyard {
    pub fn of(dead: Vec<Fish>) -> Self {
        Self { dead, entombed: 0 }
    }

    pub fn entombed_up_to(mut self, graves: usize) -> Self {
        self.entombed = graves.min(self.dead.len());
        self
    }

    pub fn push(&mut self, fish: Fish) {
        self.dead.push(fish);
    }

    pub fn remove(&mut self, grave: usize) -> Fish {
        self.entombed = self.entombed.min(grave);
        self.dead.remove(grave)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.dead.iter().map(|fish| fish.name.as_str())
    }

    pub fn burial(&mut self) -> Burial {
        let burial = Burial {
            keep: self.entombed,
            fresh: self.dead[self.entombed..].to_vec(),
        };
        self.entombed = self.dead.len();
        burial
    }
}

impl Deref for Graveyard {
    type Target = [Fish];

    fn deref(&self) -> &[Fish] {
        &self.dead
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fishes::species::FishSpecies;

    fn dead(name: &str) -> Fish {
        Fish::new(
            FishSpecies::Goldfish,
            name.to_string(),
            0.0,
            0.0,
            &mut rand::rng(),
        )
    }

    fn names(fish: &[Fish]) -> Vec<&str> {
        fish.iter().map(|fish| fish.name.as_str()).collect()
    }

    #[test]
    fn a_burial_hands_over_only_the_dead_the_crypt_has_not_seen() {
        let mut graveyard = Graveyard::default();
        graveyard.push(dead("Ann"));
        graveyard.push(dead("Bob"));
        let first = graveyard.burial();
        graveyard.push(dead("Cid"));

        let second = graveyard.burial();

        assert_eq!((first.keep, names(&first.fresh)), (0, vec!["Ann", "Bob"]));
        assert_eq!((second.keep, names(&second.fresh)), (2, vec!["Cid"]));
        assert!(
            graveyard.burial().fresh.is_empty(),
            "nothing new, nothing written"
        );
    }

    #[test]
    fn exhuming_a_grave_rewrites_the_crypt_from_that_grave_on() {
        let mut graveyard = Graveyard::default();
        for name in ["Ann", "Bob", "Cid"] {
            graveyard.push(dead(name));
        }
        let _ = graveyard.burial();

        let bob = graveyard.remove(1);
        let burial = graveyard.burial();

        assert_eq!(bob.name, "Bob");
        assert_eq!((burial.keep, names(&burial.fresh)), (1, vec!["Cid"]));
    }
}
