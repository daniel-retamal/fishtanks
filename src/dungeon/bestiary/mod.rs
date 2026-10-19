use ratatui::style::Color;

use crate::colors;
use crate::fishes::species::FishSpecies;

use super::effect::{Effect, Reach};
use super::skill::Skill;
use super::traits::Trait;
use super::treasure::Find;

pub mod anywhere;
pub mod candy_abyss;
pub mod dunes;
pub mod eye;
pub mod gate;
pub mod graveyard;
pub mod hot_zone;
pub mod inferno;
pub mod mainframe;
pub mod reef;
pub mod shallows;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rank {
    Common,
    Elite,
    Boss,
}

impl Rank {
    pub fn shares(self) -> u64 {
        match self {
            Rank::Common => 1,
            Rank::Elite => 4,
            Rank::Boss => 10,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Rank::Common => "Common",
            Rank::Elite => "Elite",
            Rank::Boss => "Boss",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpriteRow {
    pub text: &'static str,
    pub paint: &'static str,
}

pub const BLANK: SpriteRow = row("", "");

pub const fn row(text: &'static str, paint: &'static str) -> SpriteRow {
    SpriteRow { text, paint }
}

impl SpriteRow {
    pub fn colour_at(&self, index: usize) -> Color {
        let keys: Vec<char> = self.paint.chars().collect();
        let key = match keys.len() {
            0 => 's',
            1 => keys[0],
            n => keys[index.min(n - 1)],
        };
        ink(key)
    }

    pub fn width(&self) -> usize {
        self.text.chars().count()
    }
}

pub fn ink(key: char) -> Color {
    match key {
        'r' => colors::RED,
        'R' => colors::LIGHT_RED,
        'q' => colors::RED_DARK,
        'o' => colors::ORANGE,
        'y' => colors::YELLOW,
        'g' => colors::AMBER,
        'G' => colors::LIGHT_GREEN,
        'f' => colors::FOREST,
        'c' => colors::LIGHT_CYAN,
        'b' => colors::NAVY,
        'B' => colors::LIGHT_BLUE,
        'm' => colors::MAGENTA,
        'p' => colors::PINK,
        'w' => colors::WHITE,
        'd' => colors::DARK_GRAY,
        'k' => colors::CHARCOAL,
        't' => colors::TEAL,
        'v' => colors::VIOLET,
        'e' => colors::CREAM,
        'n' => colors::TAN,
        _ => colors::GRAY,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemySkill {
    pub name: &'static str,
    pub cost: u8,
    pub reach: Reach,
    pub effects: &'static [Effect],
    pub about: &'static str,
}

impl EnemySkill {
    pub fn skill(&self) -> Skill {
        Skill::new(self.name, self.cost, self.reach, self.effects, self.about)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Passive {
    pub name: &'static str,
    pub about: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loot {
    Rare,
    Gold,
    RareEvery(u32),
    Named(Find),
    Hardcore(Find),
    Fish {
        species: FishSpecies,
        one_in: u32,
        hardcore: bool,
    },
}

pub const NOW_AND_THEN: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemySpec {
    pub name: &'static str,
    pub floor: u32,
    pub rank: Rank,
    pub hp: u32,
    pub mp: u32,
    pub bite: u32,
    pub speed: u32,
    pub sprite: &'static [SpriteRow],
    pub skills: &'static [EnemySkill],
    pub passives: &'static [Passive],
    pub traits: &'static [Trait],
    pub loot: &'static [Loot],
    pub variant_of: Option<&'static str>,
    pub note: &'static str,
}

impl EnemySpec {
    pub fn width(&self) -> usize {
        self.sprite.iter().map(SpriteRow::width).max().unwrap_or(0)
    }

    pub fn main_colour(&self) -> Option<Color> {
        let mut counts: std::collections::BTreeMap<char, usize> = std::collections::BTreeMap::new();
        for row in self.sprite {
            let keys: Vec<char> = row.paint.chars().collect();
            for (index, glyph) in row.text.chars().enumerate() {
                if glyph == ' ' || keys.is_empty() {
                    continue;
                }
                *counts.entry(keys[index.min(keys.len() - 1)]).or_default() += 1;
            }
        }
        counts
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(key, _)| ink(key))
    }

    pub fn bites(&self) -> bool {
        !self.traits.contains(&Trait::NeverBites)
    }
}

pub fn all() -> impl Iterator<Item = &'static EnemySpec> {
    FLOORS.iter().flat_map(|floor| floor.iter())
}

pub fn find(name: &str) -> Option<&'static EnemySpec> {
    all().find(|spec| spec.name.eq_ignore_ascii_case(name))
}

pub fn of_floor(floor: u32) -> impl Iterator<Item = &'static EnemySpec> {
    all().filter(move |spec| spec.floor == floor)
}

const FLOORS: &[&[EnemySpec]] = &[
    anywhere::ROWS,
    shallows::ROWS,
    reef::ROWS,
    candy_abyss::ROWS,
    graveyard::ROWS,
    dunes::ROWS,
    hot_zone::ROWS,
    mainframe::ROWS,
    inferno::ROWS,
    gate::ROWS,
    eye::ROWS,
];

#[cfg(test)]
mod tests;
