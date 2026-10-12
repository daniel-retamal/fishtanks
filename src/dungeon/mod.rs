use serde::{Deserialize, Serialize};

pub mod ai;
pub mod battle;
pub mod beat;
pub mod bestiary;
pub mod boon;
pub mod effect;
pub mod expedition;
pub mod fighter;
pub mod floor;
#[cfg(test)]
pub mod glyphs;
pub mod info;
pub mod items;
pub mod kit;
pub mod life;
pub mod moves;
pub mod passives;
pub mod reveal;
pub mod rules;
pub mod sim;
pub mod skill;
pub mod specials;
pub mod state;
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

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Dungeon {
    #[serde(default)]
    pub deepest: u32,
    #[serde(default)]
    pub dives: u32,
    #[serde(default)]
    pub expedition: Option<expedition::Expedition>,
    #[serde(default)]
    pub remembered: Vec<String>,
}
