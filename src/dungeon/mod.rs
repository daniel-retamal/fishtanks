use serde::{Deserialize, Serialize};

pub mod ai;
pub mod battle;
pub mod beat;
pub mod bestiary;
pub mod boon;
pub mod effect;
pub mod fighter;
pub mod floor;
#[cfg(test)]
pub mod glyphs;
pub mod items;
pub mod kit;
pub mod life;
pub mod moves;
pub mod passives;
pub mod rules;
pub mod sim;
pub mod skill;
pub mod specials;
pub mod tables;
pub mod traits;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Difficulty {
    #[default]
    Normal,
    Hardcore,
}

impl Difficulty {
    pub fn name(self) -> &'static str {
        match self {
            Difficulty::Normal => "Normal",
            Difficulty::Hardcore => "Hardcore",
        }
    }

    pub fn kills(self) -> bool {
        self == Difficulty::Hardcore
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Dungeon {
    #[serde(default)]
    pub deepest: u32,
    #[serde(default)]
    pub dives: u32,
}
