use serde::{Deserialize, Serialize};

use crate::fishes::fish::Fish;

use super::Difficulty;
use super::battle::{Battle, BattleSetup, Choice, Stakes};
use super::beat::Beat;
use super::bestiary;
use super::fighter::{Fighter, Ghost};
use super::floor;
use super::traits::Haunt;

pub const MAX_PARTY: usize = 4;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Input {
    Choose(Choice),
    Answer(bool),
    Pray(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Foe {
    pub enemy: String,
    pub ghost: Option<Ghost>,
}

impl Foe {
    pub fn of(fighter: &Fighter) -> Option<Foe> {
        fighter.spec().map(|spec| Foe {
            enemy: spec.name.to_string(),
            ghost: fighter.ghost.clone(),
        })
    }

    fn fighter(&self, depth: u32) -> Option<Fighter> {
        let spec = bestiary::find(&self.enemy)?;
        let scale = floor::scale_for(depth);
        Some(match &self.ghost {
            Some(ghost) => Fighter::of_ghost(spec, ghost, depth, scale),
            None => Fighter::of_enemy(spec, depth, scale),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FightRecord {
    pub seed: u64,
    pub stakes: Stakes,
    pub purse: u128,
    pub foes: Vec<Foe>,
    pub haunts: Vec<Ghost>,
    pub inputs: Vec<Input>,
    #[serde(default)]
    pub spent: u128,
    #[serde(default)]
    pub known: Vec<String>,
    #[serde(default)]
    pub every_graeae: bool,
}

impl FightRecord {
    pub fn of(
        seed: u64,
        stakes: Stakes,
        purse: u128,
        foes: &[Fighter],
        haunts: Vec<Ghost>,
    ) -> Self {
        FightRecord {
            seed,
            stakes,
            purse,
            foes: foes.iter().filter_map(Foe::of).collect(),
            haunts,
            inputs: Vec::new(),
            spent: 0,
            known: Vec::new(),
            every_graeae: false,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Expedition {
    pub party: Vec<Fish>,
    pub homes: Vec<String>,
    pub floor: u32,
    pub difficulty: Difficulty,
    pub fight: Option<FightRecord>,
}

impl Expedition {
    pub fn new(party: Vec<Fish>, homes: Vec<String>, difficulty: Difficulty) -> Self {
        Expedition {
            party,
            homes,
            floor: 1,
            difficulty,
            fight: None,
        }
    }

    pub fn setup(&self, record: &FightRecord) -> BattleSetup {
        BattleSetup {
            floor: self.floor,
            difficulty: self.difficulty,
            stakes: record.stakes,
            purse: record.purse,
            items: Vec::new(),
            seed: record.seed,
            ambushed: false,
            haunts: record.haunts.clone(),
            scale: floor::scale_for(self.floor),
            known: record.known.clone(),
            every_graeae: record.every_graeae,
        }
    }

    pub fn battle(&self) -> Option<(Battle, Vec<Beat>)> {
        let record = self.fight.as_ref()?;
        let foes: Vec<Fighter> = record
            .foes
            .iter()
            .filter_map(|foe| foe.fighter(self.floor))
            .collect();
        let mut battle = Battle::new(self.party.clone(), foes, self.setup(record));
        let opening = battle.begin();
        for input in &record.inputs {
            play(&mut battle, input);
        }
        Some((battle, opening))
    }

    pub fn home_of(&self, member: usize) -> Option<&str> {
        self.homes.get(member).map(String::as_str)
    }
}

pub fn play(battle: &mut Battle, input: &Input) -> Vec<Beat> {
    match input {
        Input::Choose(choice) => battle.choose(*choice),
        Input::Answer(sign) => battle.answer(*sign),
        Input::Pray(words) => battle.pray(words),
    }
}

pub fn haunts<'a>(dead: impl Iterator<Item = &'a Fish>) -> Vec<Ghost> {
    dead.map(|fish| {
        let haunt = match (fish.devil_marked, fish.is_unfish()) {
            (true, _) => Haunt::Damned,
            (false, true) => Haunt::Unfish,
            (false, false) => Haunt::Dead,
        };
        Ghost::of(fish, haunt)
    })
    .collect()
}
