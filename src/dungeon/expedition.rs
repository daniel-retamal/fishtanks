use std::collections::HashSet;

use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::economy::Rarity;
use crate::entities::cow::Cow;
use crate::fishes::fish::Fish;
use crate::names::unique_name_in;

use super::battle::{Aftermath, Battle, BattleSetup, Choice, Outcome, Stakes};
use super::beat::Beat;
use super::bestiary::{self, Loot};
use super::effect::{Status, StatusKind};
use super::event::{self, Omen};
use super::fighter::{Fighter, Ghost, Scale};
use super::floor::{self, FloorSpec};
use super::haul::{self, Haul};
use super::items::Pack;
use super::kit::Kit;
use super::life::DeepLife;
use super::map::{LAYERS, Map, RoomKind, Shape};
use super::room::{Stage, Visit};
use super::traits::{Haunt, Trait};
use super::{Above, Difficulty, Wreck};

pub const MAX_PARTY: usize = 4;
pub const SEED_STRIDE: u64 = 0x9E37_79B9_7F4A_7C15;

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

    pub fn fighter(&self, depth: u32, scale: Scale) -> Option<Fighter> {
        let spec = bestiary::find(&self.enemy)?;
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
    #[serde(default)]
    pub ambushed: bool,
    #[serde(default)]
    pub lingering: Vec<Lingering>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lingering {
    pub fish: String,
    pub kind: StatusKind,
    pub turns: u8,
    pub amount: u32,
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
            ambushed: false,
            lingering: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ground {
    pub haunts: Vec<Ghost>,
    pub purse: u128,
    pub known: Vec<String>,
    pub every_graeae: bool,
    pub dives: u32,
    pub today: String,
    pub wrecks: Vec<Wreck>,
    #[serde(default)]
    pub fabricators: u32,
    #[serde(default)]
    pub blueprints: usize,
}

impl Ground {
    pub fn above(&self) -> Above<'_> {
        Above {
            wrecks: &self.wrecks,
            fabricators: self.fabricators,
            blueprints: self.blueprints,
        }
    }

    pub fn wreck(&self, depth: u32) -> Option<&Wreck> {
        self.wrecks.iter().find(|wreck| wreck.floor == depth)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wound {
    pub hp: Option<u32>,
    pub belly_up: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Route {
    pub seed: u64,
    pub wreck: bool,
    pub path: Vec<u8>,
    pub done: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Fight,
    Room,
    Choosing,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Next {
    Stay,
    Map,
    Fight,
    Raise(String),
    Home,
    Cast,
    Bench(Bench),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bench {
    Program(usize),
    Dress(usize),
    Print,
}

pub struct Settled {
    pub dead: Vec<Fish>,
    pub next: Next,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Expedition {
    pub party: Vec<Fish>,
    pub homes: Vec<String>,
    pub floor: u32,
    pub difficulty: Difficulty,
    pub fight: Option<FightRecord>,
    #[serde(default)]
    pub wounds: Vec<Wound>,
    #[serde(default)]
    pub pack: Vec<Pack>,
    #[serde(default)]
    pub haul: Haul,
    #[serde(default)]
    pub route: Route,
    #[serde(default)]
    pub visit: Option<Visit>,
    #[serde(default)]
    pub fallen: Vec<String>,
    #[serde(default)]
    pub joined: u32,
    #[serde(default)]
    pub omens: Vec<Omen>,
    #[serde(default)]
    pub lucky_spent: bool,
    #[serde(default)]
    pub wiped: bool,
    #[serde(default)]
    pub wreck_found: Option<Haul>,
    #[serde(default)]
    pub dive: u32,
    #[serde(default)]
    pub lingering: Vec<Lingering>,
    #[serde(default)]
    pub beamed: Vec<Fish>,
    #[serde(default)]
    pub cows: Vec<Cow>,
    #[serde(default)]
    pub guard: Option<String>,
    #[serde(skip)]
    pub tuning: Option<Scale>,
}

impl Expedition {
    pub fn new(party: Vec<Fish>, homes: Vec<String>, difficulty: Difficulty) -> Self {
        let wounds = vec![Wound::default(); party.len()];
        Expedition {
            party,
            homes,
            floor: 1,
            difficulty,
            fight: None,
            wounds,
            pack: Vec::new(),
            haul: Haul::default(),
            route: Route::default(),
            visit: None,
            fallen: Vec::new(),
            joined: 0,
            omens: Vec::new(),
            lucky_spent: false,
            wiped: false,
            wreck_found: None,
            dive: 0,
            lingering: Vec::new(),
            beamed: Vec::new(),
            cows: Vec::new(),
            guard: None,
            tuning: None,
        }
    }

    pub fn set_out(
        party: Vec<Fish>,
        homes: Vec<String>,
        difficulty: Difficulty,
        pack: Vec<Pack>,
        seed: u64,
        ground: &Ground,
    ) -> Self {
        let mut expedition = Expedition::new(party, homes, difficulty);
        expedition.pack = pack;
        expedition.dive = ground.dives;
        expedition.route = Route {
            seed,
            wreck: ground.wreck(1).is_some(),
            path: Vec::new(),
            done: false,
        };
        expedition
    }

    pub fn map(&self) -> Map {
        Map::generate(
            self.route.seed,
            Shape {
                wreck: self.route.wreck,
                events: !event::of_floor(self.floor).is_empty(),
            },
        )
    }

    pub fn place(&self) -> Place {
        if self.fight.is_some() {
            return Place::Fight;
        }
        if self.visit.is_some() {
            return Place::Room;
        }
        Place::Choosing
    }

    pub fn ahead(&self) -> Vec<u8> {
        if self.visit.is_some() || self.fight.is_some() {
            return Vec::new();
        }
        self.map().next(&self.route.path)
    }

    pub fn kits(&self) -> Vec<Kit> {
        self.party.iter().map(Kit::of).collect()
    }

    pub fn wound(&self, member: usize) -> Wound {
        self.wounds.get(member).copied().unwrap_or_default()
    }

    pub fn hp(&self, member: usize, kit: &Kit) -> u32 {
        let wound = self.wound(member);
        match wound.belly_up {
            true => 0,
            false => wound.hp.unwrap_or(kit.vigor).min(kit.vigor),
        }
    }

    pub fn standing(&self) -> Vec<usize> {
        (0..self.party.len())
            .filter(|&member| !self.wound(member).belly_up)
            .collect()
    }

    pub fn lead(&self) -> usize {
        self.standing().first().copied().unwrap_or(0)
    }

    pub fn carries(&self, wanted: Trait) -> Option<usize> {
        self.kits()
            .iter()
            .position(|kit| kit.traits.contains(&wanted))
    }

    pub fn scale(&self) -> Scale {
        self.tuning.unwrap_or_else(|| floor::scale_for(self.floor))
    }

    pub fn share(&self) -> u64 {
        floor::gold_share(self.floor, self.difficulty)
    }

    pub fn price(&self, shares: u64) -> u64 {
        self.share() * shares
    }

    pub fn rng(&self, salt: u64) -> SmallRng {
        SmallRng::seed_from_u64(self.route.seed ^ salt.wrapping_mul(SEED_STRIDE))
    }

    pub fn go(&mut self, index: u8, ground: &Ground) -> bool {
        if !self.ahead().contains(&index) {
            return false;
        }
        let map = self.map();
        let row = self.route.path.len();
        let Some(room) = map.room(row, usize::from(index)) else {
            return false;
        };
        self.route.path.push(index);
        self.route.done = false;
        self.angle(room.seed);
        self.visit = Some(Visit::enter(self, row, index, room, ground));
        true
    }

    pub fn leave(&mut self) -> Next {
        let Some(visit) = self.visit.take() else {
            return Next::Map;
        };
        self.route.done = true;
        if visit.kind == RoomKind::Boss {
            self.reach_the_elevator();
        }
        Next::Map
    }

    fn reach_the_elevator(&mut self) {
        let depth = self.floor;
        for member in self.standing() {
            let life = self.party[member]
                .deep
                .get_or_insert_with(|| Box::new(DeepLife::default()));
            life.floors_survived += 1;
            life.deepest = life.deepest.max(depth);
        }
        self.visit = Some(Visit::elevator(self.route.path.len(), self.route.seed));
    }

    pub fn at_the_elevator(&self) -> bool {
        self.visit
            .as_ref()
            .is_some_and(|visit| visit.kind == RoomKind::Elevator)
    }

    pub fn descend(&mut self, ground: &Ground) {
        self.floor += 1;
        let mut rng = self.rng(u64::from(self.floor));
        self.route = Route {
            seed: rng.random(),
            wreck: ground.wreck(self.floor).is_some(),
            path: Vec::new(),
            done: false,
        };
        self.visit = None;
        self.omens.clear();
        self.lucky_spent = false;
        self.guard = None;
    }

    pub fn boss(&self) -> &'static bestiary::EnemySpec {
        let spec = FloorSpec::of(self.floor);
        self.guard
            .as_deref()
            .and_then(|name| spec.bosses().into_iter().find(|boss| boss.name == name))
            .unwrap_or_else(|| spec.boss(self.dive))
    }

    pub fn lair(&self) -> Vec<Fighter> {
        vec![Fighter::of_enemy(self.boss(), self.floor, self.scale())]
    }

    pub fn start_fight(
        &mut self,
        foes: Vec<Foe>,
        stakes: Stakes,
        ambushed: bool,
        ground: &Ground,
    ) -> Next {
        let salt = self
            .visit
            .as_ref()
            .map_or(self.route.seed, |visit| visit.seed ^ u64::from(visit.rolls));
        let seed = self.rng(salt).random();
        let silent = self.carries(Trait::SilentSteps).is_some();
        self.fight = Some(FightRecord {
            seed,
            stakes,
            purse: ground.purse,
            foes,
            haunts: ground.haunts.clone(),
            inputs: Vec::new(),
            spent: 0,
            known: ground.known.clone(),
            every_graeae: ground.every_graeae,
            ambushed: ambushed && !silent,
            lingering: std::mem::take(&mut self.lingering),
        });
        Next::Fight
    }

    pub fn setup(&self, record: &FightRecord) -> BattleSetup {
        BattleSetup {
            floor: self.floor,
            difficulty: self.difficulty,
            stakes: record.stakes,
            purse: record.purse,
            items: self.pack.clone(),
            seed: record.seed,
            ambushed: record.ambushed,
            haunts: record.haunts.clone(),
            scale: self.scale(),
            known: record.known.clone(),
            every_graeae: record.every_graeae,
        }
    }

    pub fn battle(&self) -> Option<(Battle, Vec<Beat>)> {
        let record = self.fight.as_ref()?;
        let foes: Vec<Fighter> = record
            .foes
            .iter()
            .filter_map(|foe| foe.fighter(self.floor, self.scale()))
            .collect();
        let mut battle = Battle::new(self.party.clone(), foes, self.setup(record));
        battle.herd(self.cows.clone());
        for member in 0..self.party.len() {
            let wound = self.wound(member);
            if wound.hp.is_some() || wound.belly_up {
                battle.carry(member, wound.hp.unwrap_or(0), wound.belly_up);
            }
        }
        for lingering in &record.lingering {
            let Some(member) = self
                .party
                .iter()
                .position(|fish| fish.name == lingering.fish)
            else {
                continue;
            };
            let status = Status::new(lingering.kind, lingering.turns, lingering.amount);
            battle.linger(member, status);
        }
        let opening = battle.begin();
        for input in &record.inputs {
            play(&mut battle, input);
        }
        Some((battle, opening))
    }

    pub fn home_of(&self, member: usize) -> Option<&str> {
        self.homes.get(member).map(String::as_str)
    }

    pub fn names(&self) -> HashSet<String> {
        self.party.iter().map(|fish| fish.name.clone()).collect()
    }

    pub fn join(&mut self, mut fish: Fish) -> String {
        let name = unique_name_in(&self.names(), &fish.name);
        fish.name = name.clone();
        self.party.push(fish);
        self.wounds.push(Wound::default());
        let home = self.homes.first().cloned().unwrap_or_default();
        self.homes.push(home);
        self.joined += 1;
        name
    }

    pub fn rejoin(&mut self, fish: Fish, home: String) {
        self.fallen.retain(|name| *name != fish.name);
        self.party.push(fish);
        self.wounds.push(Wound::default());
        self.homes.push(home);
    }

    pub fn settle(&mut self, aftermath: Aftermath) -> Settled {
        let Some(record) = self.fight.take() else {
            return Settled {
                dead: Vec::new(),
                next: Next::Map,
            };
        };
        let outcome = aftermath.outcome;
        self.cows = aftermath.cows;
        let mut found = Haul {
            gold: aftermath.haul,
            treasures: Vec::new(),
        };
        found.add(crate::loot::StockItem::Junk, aftermath.junk_found);
        self.pack = aftermath
            .items
            .into_iter()
            .filter(|p| p.count > 0)
            .collect();
        let joined = aftermath.party.len().saturating_sub(self.party.len());
        self.joined += joined as u32;
        while self.homes.len() < aftermath.party.len() {
            let home = self.homes.first().cloned().unwrap_or_default();
            self.homes.push(home);
        }
        let mut wounds: Vec<Wound> = (0..aftermath.party.len())
            .map(|member| {
                aftermath.hp.iter().find(|(m, _, _)| *m == member).map_or(
                    Wound::default(),
                    |&(_, hp, belly_up)| Wound {
                        hp: Some(hp),
                        belly_up,
                    },
                )
            })
            .collect();
        let mut dead = Vec::new();
        let mut party = Vec::new();
        let mut homes = Vec::new();
        let mut kept = Vec::new();
        for (member, mut fish) in aftermath.party.into_iter().enumerate() {
            if aftermath.dead.contains(&member) {
                self.fallen.push(fish.name.clone());
                dead.push(fish);
                continue;
            }
            if outcome == Outcome::Won {
                let life = fish
                    .deep
                    .get_or_insert_with(|| Box::new(DeepLife::default()));
                life.beaten += aftermath.beaten;
            }
            party.push(fish);
            homes.push(self.homes.get(member).cloned().unwrap_or_default());
            kept.push(std::mem::take(&mut wounds[member]));
        }
        self.party = party;
        self.homes = homes;
        self.wounds = kept;
        let everyone_down = self.wounds.iter().all(|wound| wound.belly_up);
        if outcome == Outcome::Wiped || self.party.is_empty() || everyone_down {
            self.haul.absorb(found);
            self.wiped = true;
            return Settled {
                dead,
                next: Next::Home,
            };
        }
        if outcome != Outcome::Won {
            self.haul.absorb(found);
            let next = self.after_a_flight();
            return Settled { dead, next };
        }
        let mut rng = self.rng(record.seed);
        for foe in &record.foes {
            let Some(spec) = bestiary::find(&foe.enemy) else {
                continue;
            };
            for loot in spec.loot {
                match loot {
                    Loot::Rare => found.add(haul::treasure(self.floor, Rarity::Rare, &mut rng), 1),
                    Loot::Gold => found.gold += self.share() * spec.rank.shares(),
                    Loot::RareEvery(_) => {}
                    Loot::Named(find) => haul::find(&mut found, *find, &mut rng),
                    Loot::Hardcore(find) if self.difficulty.kills() => {
                        haul::find(&mut found, *find, &mut rng)
                    }
                    Loot::Hardcore(_) => {}
                    Loot::Fish {
                        species,
                        one_in,
                        hardcore,
                    } => {
                        let earned = !hardcore || self.difficulty.kills();
                        if earned && rng.random_range(0..(*one_in).max(1)) == 0 {
                            let fish = Fish::new(
                                *species,
                                species.display_name().to_string(),
                                0.0,
                                0.0,
                                &mut rng,
                            );
                            self.join(fish);
                        }
                    }
                }
            }
        }
        for &(name, taken) in &aftermath.spoils {
            let Some(spec) = bestiary::find(name) else {
                continue;
            };
            for loot in spec.loot {
                if let Loot::RareEvery(every) = loot {
                    for _ in 0..taken / (*every).max(1) {
                        found.add(haul::treasure(self.floor, Rarity::Rare, &mut rng), 1);
                    }
                }
            }
        }
        let next = self.after_a_win(found, &mut rng);
        Settled { dead, next }
    }

    fn after_a_flight(&mut self) -> Next {
        let Some(visit) = self.visit.as_mut() else {
            return Next::Map;
        };
        visit.stage = Stage::Spoils;
        visit.said = vec![super::beat::LogLine::new(
            "The party got away.",
            super::beat::Tone::Quiet,
        )];
        Next::Stay
    }

    fn after_a_win(&mut self, mut found: Haul, rng: &mut SmallRng) -> Next {
        let Some(visit) = self.visit.take() else {
            self.haul.absorb(found);
            return Next::Map;
        };
        let mut visit = visit;
        let bonus = visit.prize(self, rng);
        found.absorb(bonus);
        visit.stage = Stage::Spoils;
        visit.said = super::room::spoils_lines(&found);
        self.haul.absorb(found);
        self.visit = Some(visit);
        Next::Stay
    }

    pub fn chest_tier(&self) -> Rarity {
        let mut tier = Rarity::Common;
        if self.difficulty.kills() {
            tier = haul::richer(tier);
        }
        for omen in &self.omens {
            tier = match omen {
                Omen::ChestsRicher => haul::richer(tier),
                Omen::ChestsPoorer => haul::poorer(tier),
            };
        }
        tier
    }

    fn angle(&mut self, salt: u64) {
        if self.carries(Trait::Angler).is_none() {
            return;
        }
        let bait = self.pack.iter().position(|pack| {
            pack.item == super::items::Item::Stock(crate::loot::StockItem::BAIT) && pack.count > 0
        });
        let Some(bait) = bait else {
            return;
        };
        self.pack[bait].count -= 1;
        let mut rng = self.rng(salt ^ ANGLER_SALT);
        let pool =
            crate::loot::LootPool::default_pool().with_native(FloorSpec::of(self.floor).tank);
        match pool.roll(&mut rng) {
            crate::loot::LootKind::Fish(species) => {
                let fish = Fish::new(
                    species,
                    species.display_name().to_string(),
                    0.0,
                    0.0,
                    &mut rng,
                );
                self.join(fish);
            }
            crate::loot::LootKind::Item(item) => {
                if let Some(stock) = crate::loot::StockItem::from_item(&item) {
                    self.haul.add(stock, 1);
                }
            }
            crate::loot::LootKind::Cash(_) | crate::loot::LootKind::Food(_) => {
                self.haul.gold += self.share();
            }
        }
    }

    pub fn wreck(&self, ground: &Ground) -> Wreck {
        Wreck {
            floor: self.floor,
            day: ground.today.clone(),
            haul: self.haul.clone(),
        }
    }

    pub fn last_room(&self) -> Option<RoomKind> {
        let row = self.route.path.len().checked_sub(1)?;
        let index = *self.route.path.last()?;
        self.map()
            .room(row, usize::from(index))
            .map(|room| room.kind)
    }

    pub fn floor_title(&self) -> String {
        FloorSpec::title(self.floor)
    }

    pub fn rows_left(&self) -> usize {
        (LAYERS + 1).saturating_sub(self.route.path.len())
    }
}

pub const ANGLER_SALT: u64 = 0xA9_61E5;

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
