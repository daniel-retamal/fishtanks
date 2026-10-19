use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};

use crate::app::FIRST_FISH;
use crate::fishes::fish::Fish;
use crate::fishes::fused::FusedComponent;
use crate::fishes::mutations::{Mutation, apply_mutation};
use crate::fishes::species::{FishSpecies, SizeCategory};
use crate::loot::{ConsumableKind, MilkVariant, StockItem};

use super::Difficulty;
use super::battle::{Battle, Choice};
use super::effect::{Effect, Power, Reach};
use super::expedition::{Expedition, Ground, Next, Place};
use super::fighter::{Scale, Side};
use super::floor::{self, LAST_FLOOR};
use super::items::{Item, Pack};
use super::life::{DeepLife, Grade};
use super::map::RoomKind;
use super::room::Act;
use super::skill::Skill;
use super::traits::Trait;

pub const FLOOR_MINUTES: f64 = 12.0;
pub const MINUTES_AN_HOUR: f64 = 60.0;
pub const HEAL_BELOW_PERCENT: u32 = 40;
pub const GUARD_BELOW_PERCENT: u32 = 50;
pub const FLEE_BELOW_PERCENT: u32 = 25;
pub const REST_PERCENT: u32 = 50;
pub const MEALS_PACKED: u32 = 8;
pub const JUNK_PACKED: u32 = 10;
pub const MAX_CHOICES: usize = 600;
pub const MID_MUTATIONS: usize = 2;
pub const DEN_NERVE_PERCENT: u32 = 100;
pub const BRED_STACKS: usize = 2;
pub const STRONG_SIZE: SizeCategory = SizeCategory::M;
pub const BRED_SIZE: SizeCategory = SizeCategory::L;
const PERCENT: u32 = 100;
const SEED_STRIDE: u64 = 7_919;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tier {
    Starter,
    Mid,
    Strong,
    Bred,
}

impl Tier {
    pub const ALL: [Tier; 4] = [Tier::Starter, Tier::Mid, Tier::Strong, Tier::Bred];

    pub fn name(self) -> &'static str {
        match self {
            Tier::Starter => "Starter",
            Tier::Mid => "Mid",
            Tier::Strong => "Strong",
            Tier::Bred => "Bred",
        }
    }

    pub fn targets(self) -> [Option<f64>; LAST_FLOOR as usize] {
        let t = |f1, f3, f5, f7, f9, f10| {
            [
                Some(f1),
                None,
                Some(f3),
                None,
                Some(f5),
                None,
                Some(f7),
                None,
                f9,
                f10,
            ]
        };
        match self {
            Tier::Starter => t(0.92, 0.45, 0.12, 0.02, None, None),
            Tier::Mid => t(0.99, 0.90, 0.72, 0.44, Some(0.18), Some(0.10)),
            Tier::Strong => t(1.0, 0.98, 0.93, 0.86, Some(0.78), Some(0.70)),
            Tier::Bred => t(1.0, 0.995, 0.985, 0.97, Some(0.95), Some(0.92)),
        }
    }

    pub fn all_ten(self) -> f64 {
        match self {
            Tier::Starter => 0.0,
            Tier::Mid => 0.001,
            Tier::Strong => 0.30,
            Tier::Bred => 0.77,
        }
    }

    pub fn hourly(self) -> f64 {
        match self {
            Tier::Starter => 8_000.0,
            Tier::Mid => 31_000.0,
            Tier::Strong => 79_000.0,
            Tier::Bred => 156_000.0,
        }
    }

    pub fn party(self, rng: &mut SmallRng) -> Vec<Fish> {
        match self {
            Tier::Starter => FIRST_FISH
                .iter()
                .map(|&(species, name)| Fish::new(species, name.to_string(), 0.0, 0.0, rng))
                .collect(),
            Tier::Mid => [
                FishSpecies::Swordfish,
                FishSpecies::Lionfish,
                FishSpecies::Seahorsefish,
                FishSpecies::Pufferfish,
            ]
            .into_iter()
            .map(|species| {
                let mut fish = grown(species, SizeCategory::M, rng);
                for _ in 0..MID_MUTATIONS {
                    mutate_at_random(&mut fish, rng);
                }
                fish
            })
            .collect(),
            Tier::Strong => {
                let mut party = vec![grown(FishSpecies::Holyfish, STRONG_SIZE, rng)];
                for (species, vial) in [
                    (FishSpecies::Swordfish, Mutation::MouthVariant),
                    (FishSpecies::Lionfish, Mutation::Spikes),
                ] {
                    let mut fish = grown(species, STRONG_SIZE, rng);
                    apply_mutation(&mut fish, vial, rng);
                    party.push(fish);
                }
                for fish in &mut party {
                    graded(fish, Grade::Adult);
                }
                party
            }
            Tier::Bred => {
                let mut party = vec![
                    stacked(FishSpecies::Cashfish, rng),
                    stacked(FishSpecies::Holyfish, rng),
                    stacked(FishSpecies::Swordfish, rng),
                ];
                for fish in &mut party {
                    graded(fish, Grade::Elder);
                }
                party
            }
        }
    }
}

fn grown(species: FishSpecies, size: SizeCategory, rng: &mut SmallRng) -> Fish {
    let mut fish = Fish::new(species, species.display_name().to_string(), 0.0, 0.0, rng);
    fish.size_category = size;
    fish.weight_g = species.config().weight_cap[size as usize].max(fish.weight_g);
    fish
}

fn mutate_at_random(fish: &mut Fish, rng: &mut SmallRng) {
    let choices = super::specials::wearable_mutations(fish);
    if choices.is_empty() {
        return;
    }
    let pick = choices[rng.random_range(0..choices.len())];
    apply_mutation(fish, pick, rng);
}

fn graded(fish: &mut Fish, grade: Grade) {
    fish.deep = Some(Box::new(DeepLife {
        floors_survived: grade.floors_needed(),
        ..DeepLife::default()
    }));
}

fn stacked(species: FishSpecies, rng: &mut SmallRng) -> Fish {
    let mut fish = grown(species, BRED_SIZE, rng);
    apply_mutation(&mut fish, Mutation::Telophase, rng);
    let weight = fish.weight_g;
    let name = fish.name.clone();
    fish.set_fused(
        (0..BRED_STACKS)
            .map(|_| FusedComponent::fish(species, name.clone(), weight))
            .collect(),
    );
    fish.weight_g = weight.saturating_mul(BRED_STACKS as u32);
    fish
}

pub fn pack() -> Vec<Pack> {
    vec![
        Pack::of(Item::Food, MEALS_PACKED),
        Pack::of(
            Item::Stock(StockItem::Consumable(ConsumableKind::Milk(
                MilkVariant::Plain,
            ))),
            1,
        ),
        Pack::of(Item::Stock(StockItem::Junk), JUNK_PACKED),
    ]
}

pub const REST_BELOW_PERCENT: u32 = 60;
pub const STARTER_RUNS: u64 = 1_000;
const MAX_ROOMS: usize = 200;

struct Diver {
    expedition: Expedition,
    ground: Ground,
    rng: SmallRng,
}

fn hp_percent(expedition: &Expedition) -> u32 {
    let kits = expedition.kits();
    let (hp, max) = (0..expedition.party.len()).fold((0, 0), |(hp, max), member| {
        (
            hp + expedition.hp(member, &kits[member]),
            max + kits[member].vigor,
        )
    });
    hp * PERCENT / max.max(1)
}

fn hurt(expedition: &Expedition) -> bool {
    hp_percent(expedition) < REST_BELOW_PERCENT
        || (0..expedition.party.len()).any(|member| expedition.wound(member).belly_up)
}

impl Diver {
    fn new(
        party: Vec<Fish>,
        depth: u32,
        difficulty: Difficulty,
        scale: Option<Scale>,
        seed: u64,
    ) -> Diver {
        let ground = Ground::default();
        let homes = vec![String::new(); party.len()];
        let mut expedition = Expedition::set_out(party, homes, difficulty, pack(), seed, &ground);
        expedition.floor = depth;
        expedition.dive = seed as u32;
        expedition.tuning = scale;
        Diver {
            expedition,
            ground,
            rng: SmallRng::seed_from_u64(seed ^ SEED_STRIDE),
        }
    }

    fn floor(&mut self) -> Result<(), String> {
        for _ in 0..MAX_ROOMS {
            if self.expedition.at_the_elevator() {
                return Ok(());
            }
            match self.expedition.place() {
                Place::Fight => self.fight()?,
                Place::Room => self.room()?,
                Place::Choosing => {
                    let index = self.route();
                    if !self.expedition.go(index, &self.ground) {
                        return Err("a dead end".to_string());
                    }
                }
            }
        }
        Err("lost on the floor".to_string())
    }

    fn route(&mut self) -> u8 {
        let map = self.expedition.map();
        let row = self.expedition.route.path.len();
        let ahead = self.expedition.ahead();
        let hurt = hurt(&self.expedition);
        let scored: Vec<(u8, u32)> = ahead
            .iter()
            .map(|&index| {
                let kind = map
                    .room(row, usize::from(index))
                    .map_or(RoomKind::Encounter, |room| room.kind);
                let score = self.appeal(kind, hurt) * PERCENT + self.rng.random_range(0..PERCENT);
                (index, score)
            })
            .collect();
        scored
            .iter()
            .max_by_key(|(_, score)| *score)
            .map_or(0, |&(index, _)| index)
    }

    fn appeal(&self, kind: RoomKind, hurt: bool) -> u32 {
        let rich = self.expedition.haul.gold >= self.expedition.price(super::room::CLEAN_SHARES);
        match (kind, hurt) {
            (RoomKind::Vent | RoomKind::Shrine, true) => 6,
            (RoomKind::Cleaning, true) if rich => 6,
            (RoomKind::Den, true) => 0,
            (RoomKind::Encounter | RoomKind::Wreck, true) => 1,
            (RoomKind::Encounter | RoomKind::Chest | RoomKind::Wreck, false) => 4,
            (RoomKind::Den, false) => 3,
            (RoomKind::Laboratory | RoomKind::School, _) => 3,
            (RoomKind::Event | RoomKind::BellJar, _) => 2,
            (RoomKind::Vent | RoomKind::Shrine | RoomKind::Vending, _) => 2,
            (RoomKind::Cleaning | RoomKind::Chest, _) => 1,
            (RoomKind::Altar | RoomKind::Hole | RoomKind::Boss | RoomKind::Elevator, _) => 0,
        }
    }

    fn act(&mut self, act: Act, fish: usize) -> Result<(), String> {
        match self.expedition.act(act, fish, &self.ground) {
            Next::Raise(name) => {
                self.expedition.fallen.retain(|fallen| *fallen != name);
                Ok(())
            }
            Next::Home => Err("rode up".to_string()),
            Next::Cast => {
                self.expedition.cast_spent();
                Ok(())
            }
            Next::Bench(_) => Ok(()),
            Next::Stay | Next::Map | Next::Fight => Ok(()),
        }
    }

    fn room(&mut self) -> Result<(), String> {
        let Some(panel) = self.expedition.panel(0, &self.ground.above()) else {
            return Ok(());
        };
        let usable: Vec<Act> = panel
            .rows
            .iter()
            .filter(|row| row.usable)
            .map(|row| row.act)
            .collect();
        let has = |act: Act| usable.contains(&act);
        let kind = self
            .expedition
            .visit
            .as_ref()
            .map_or(RoomKind::Encounter, |visit| visit.kind);
        if has(Act::MoveOn) {
            return self.act(Act::MoveOn, 0);
        }
        if has(Act::Fight) && kind != RoomKind::Den {
            return self.act(Act::Fight, 0);
        }
        if let Some(line) = self.asked_to_pray() {
            self.expedition.pray(line, &self.ground);
            return Ok(());
        }
        let choice = match kind {
            RoomKind::Den => match self.dares() {
                true => Act::Fight,
                false => Act::Leave,
            },
            RoomKind::Chest if has(Act::LookFirst) => Act::LookFirst,
            RoomKind::Chest if has(Act::OpenWithBait) => Act::OpenWithBait,
            RoomKind::Chest => Act::Open,
            RoomKind::BellJar => Act::Break,
            RoomKind::Event => usable.first().copied().unwrap_or(Act::MoveOn),
            RoomKind::Shrine => usable.first().copied().unwrap_or(Act::Leave),
            RoomKind::Laboratory if has(Act::Dose) => Act::Dose,
            RoomKind::Vent if has(Act::Rest) => Act::Rest,
            RoomKind::Vent => return self.feed(),
            RoomKind::Cleaning if has(Act::Clean) => Act::Clean,
            RoomKind::Cleaning => usable
                .iter()
                .copied()
                .find(|act| matches!(act, Act::BringBack(_)))
                .unwrap_or(Act::Leave),
            RoomKind::School => usable
                .iter()
                .copied()
                .find(|act| matches!(act, Act::Hire(_)))
                .unwrap_or(Act::Leave),
            RoomKind::Vending if has(Act::Buy(0)) && self.meals() < MEALS_PACKED => Act::Buy(0),
            _ => Act::Leave,
        };
        self.act(choice, 0)
    }

    fn asked_to_pray(&self) -> Option<&'static str> {
        let visit = self.expedition.visit.as_ref()?;
        visit.event()?.asks_a_prayer()?;
        (visit.stage == super::room::Stage::Open).then(|| super::event::prayer_line(visit.seed))
    }

    fn meals(&self) -> u32 {
        self.expedition
            .pack
            .iter()
            .find(|pack| pack.item == Item::Food)
            .map_or(0, |pack| pack.count)
    }

    fn feed(&mut self) -> Result<(), String> {
        let kits = self.expedition.kits();
        let hungry = (0..self.expedition.party.len()).find(|&member| {
            self.expedition.hp(member, &kits[member]) * PERCENT < kits[member].vigor * REST_PERCENT
        });
        match (hungry, self.meals() > 0) {
            (Some(member), true) => self.act(Act::Feed, member),
            _ => self.act(Act::Leave, 0),
        }
    }

    fn dares(&self) -> bool {
        let Some(visit) = self.expedition.visit.as_ref() else {
            return false;
        };
        let kits = self.expedition.kits();
        let party: u32 = (0..self.expedition.party.len())
            .map(|member| self.expedition.hp(member, &kits[member]))
            .sum();
        let threat: u32 = visit
            .foes
            .iter()
            .filter_map(|foe| foe.fighter(self.expedition.floor, self.expedition.scale()))
            .map(|foe| foe.max_hp)
            .sum();
        party * PERCENT >= threat * DEN_NERVE_PERCENT
    }

    fn fight(&mut self) -> Result<(), String> {
        let Some((mut battle, _)) = self.expedition.battle() else {
            return Err("a fight that cannot start".to_string());
        };
        let cause = battle
            .side(Side::Foes)
            .map(|i| battle.fighters[i].name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        play(&mut battle);
        let settled = self.expedition.settle(battle.finish());
        match settled.next {
            Next::Home => Err(cause),
            _ => Ok(()),
        }
    }
}

fn play(battle: &mut Battle) {
    for _ in 0..MAX_CHOICES {
        if battle.offer().is_some() {
            battle.answer(false);
            continue;
        }
        let Some(actor) = battle.actor() else {
            break;
        };
        if let Some(line) = battle.prayer.line().filter(|_| shielded_by_prayer(battle)) {
            battle.pray(line);
        }
        let choice = decide(battle, actor);
        if battle.choose(choice).is_empty() && battle.actor() == Some(actor) {
            battle.choose(Choice::Guard);
        }
    }
}

pub fn fate(tier: Tier, depth: u32, difficulty: Difficulty, seed: u64) -> Result<(), String> {
    fate_at(tier, depth, difficulty, floor::scale_for(depth), seed)
}

pub fn fate_at(
    tier: Tier,
    depth: u32,
    difficulty: Difficulty,
    scale: Scale,
    seed: u64,
) -> Result<(), String> {
    let mut rng = SmallRng::seed_from_u64(seed);
    let party = tier.party(&mut rng);
    let mut diver = Diver::new(party, depth, difficulty, Some(scale), seed);
    diver.floor()
}

pub fn clears(tier: Tier, depth: u32, difficulty: Difficulty, seed: u64) -> bool {
    fate(tier, depth, difficulty, seed).is_ok()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dived {
    pub floors: u32,
    pub party: usize,
    pub joined: u32,
}

pub fn dive(tier: Tier, difficulty: Difficulty, seed: u64) -> u32 {
    dived(tier, difficulty, seed).floors
}

pub fn dived(tier: Tier, difficulty: Difficulty, seed: u64) -> Dived {
    let mut rng = SmallRng::seed_from_u64(seed);
    let party = tier.party(&mut rng);
    let mut diver = Diver::new(party, 1, difficulty, None, seed);
    let mut floors = 0;
    while floors < LAST_FLOOR {
        if diver.floor().is_err() {
            break;
        }
        floors = diver.expedition.floor;
        if floors < LAST_FLOOR {
            diver.expedition.descend(&diver.ground);
        }
    }
    Dived {
        floors,
        party: diver.expedition.party.len(),
        joined: diver.expedition.joined,
    }
}

fn seed_of(run: u64, depth: u32) -> u64 {
    run * SEED_STRIDE + u64::from(depth)
}

pub fn autopsy(
    tier: Tier,
    depth: u32,
    difficulty: Difficulty,
    scale: Scale,
    runs: u64,
) -> Vec<(String, u32)> {
    let mut causes: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    for run in 0..runs {
        if let Err(cause) = fate_at(tier, depth, difficulty, scale, seed_of(run, depth)) {
            *causes.entry(cause).or_default() += 1;
        }
    }
    let mut causes: Vec<(String, u32)> = causes.into_iter().collect();
    causes.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    causes
}

fn shielded_by_prayer(battle: &Battle) -> bool {
    battle.prayer.shields()
        && battle
            .side(Side::Foes)
            .any(|i| battle.fighters[i].has(Trait::PrayerShield) && !battle.fighters[i].gone)
}

pub fn survival(tier: Tier, depth: u32, difficulty: Difficulty, runs: u64) -> f64 {
    survival_at(tier, depth, difficulty, floor::scale_for(depth), runs)
}

pub fn survival_at(tier: Tier, depth: u32, difficulty: Difficulty, scale: Scale, runs: u64) -> f64 {
    let cleared = (0..runs)
        .filter(|&run| fate_at(tier, depth, difficulty, scale, seed_of(run, depth)).is_ok())
        .count();
    cleared as f64 / runs as f64
}

pub struct Report {
    pub tier: Tier,
    pub floors: Vec<f64>,
}

impl Report {
    pub fn of(tier: Tier, difficulty: Difficulty, runs: u64) -> Report {
        Report {
            tier,
            floors: (1..=LAST_FLOOR)
                .map(|depth| survival(tier, depth, difficulty, runs))
                .collect(),
        }
    }

    pub fn all_ten(&self) -> f64 {
        self.floors.iter().product()
    }

    pub fn best_hourly(&self, difficulty: Difficulty) -> (u32, f64) {
        let mut reach = 1.0;
        let mut minutes = 0.0;
        let mut best = (0, 0.0);
        for (index, &clear) in self.floors.iter().enumerate() {
            let depth = index as u32 + 1;
            minutes += reach * FLOOR_MINUTES;
            reach *= clear;
            let hauls: u64 = (1..=depth).map(|n| floor::haul(n, difficulty)).sum();
            let hourly = reach * hauls as f64 / minutes * MINUTES_AN_HOUR;
            if hourly > best.1 {
                best = (depth, hourly);
            }
        }
        best
    }
}

fn decide(battle: &Battle, actor: usize) -> Choice {
    let foes: Vec<usize> = battle
        .side(Side::Foes)
        .filter(|&i| battle.targetable(actor, i))
        .collect();
    let me = &battle.fighters[actor];
    if let Some(choice) = heal(battle, actor) {
        return choice;
    }
    let told = battle
        .side(Side::Foes)
        .any(|i| battle.fighters[i].tell.is_some() && battle.fighters[i].standing());
    if told && me.hp_percent() < GUARD_BELOW_PERCENT && battle.can_choose(actor, Choice::Guard) {
        return Choice::Guard;
    }
    if party_percent(battle) < FLEE_BELOW_PERCENT && battle.can_choose(actor, Choice::Flee) {
        return Choice::Flee;
    }
    let Some(target) = target_of(battle, actor, &foes) else {
        return Choice::Guard;
    };
    let bodiless = battle.fighters[target].has(Trait::NoBody);
    if let Some(choice) = strike(battle, actor, target, foes.len(), bodiless) {
        return choice;
    }
    if !bodiless {
        return Choice::Bite(target);
    }
    if let Some(choice) = throw_junk(battle, actor, target) {
        return choice;
    }
    foes.iter()
        .copied()
        .find(|&i| !battle.fighters[i].has(Trait::NoBody))
        .map_or(Choice::Guard, Choice::Bite)
}

fn target_of(battle: &Battle, actor: usize, foes: &[usize]) -> Option<usize> {
    let keeps_a_body_alive = |&i: &usize| {
        battle.fighters[i]
            .body
            .is_some_and(|body| battle.fighters[body].has(Trait::Phylactery))
    };
    let bite = battle.fighters[actor].effective_bite();
    foes.iter()
        .copied()
        .find(keeps_a_body_alive)
        .or_else(|| {
            foes.iter()
                .copied()
                .find(|&i| battle.fighters[i].hp <= bite)
        })
        .or_else(|| {
            foes.iter()
                .copied()
                .max_by_key(|&i| battle.fighters[i].effective_bite())
        })
}

fn throw_junk(battle: &Battle, actor: usize, target: usize) -> Option<Choice> {
    let junk = battle
        .items
        .iter()
        .position(|pack| pack.item == Item::Stock(StockItem::Junk) && pack.count > 0)?;
    let choice = Choice::Item { pack: junk, target };
    battle.can_choose(actor, choice).then_some(choice)
}

fn party_percent(battle: &Battle) -> u32 {
    let (hp, max) = battle
        .side(Side::Party)
        .map(|i| &battle.fighters[i])
        .filter(|f| !f.gone)
        .fold((0, 0), |(hp, max), f| (hp + f.hp, max + f.max_hp));
    hp * PERCENT / max.max(1)
}

fn heal(battle: &Battle, actor: usize) -> Option<Choice> {
    let hurt = battle
        .side(Side::Party)
        .filter(|&i| {
            let f = &battle.fighters[i];
            !f.gone && (f.belly_up || f.hp_percent() < HEAL_BELOW_PERCENT)
        })
        .min_by_key(|&i| battle.fighters[i].hp_percent())?;
    let me = &battle.fighters[actor];
    for (index, skill) in me.skills.iter().enumerate() {
        let heals = skill
            .effects
            .iter()
            .any(|e| matches!(e, Effect::Heal(_) | Effect::HealPercent(_)));
        let reach_fits = matches!(
            skill.reach,
            Reach::OneAlly | Reach::AllAllies | Reach::WeakestAlly
        ) || (skill.reach == Reach::Me && hurt == actor);
        let choice = Choice::Skill {
            skill: index,
            target: Some(hurt),
        };
        if heals && reach_fits && battle.can_choose(actor, choice) {
            return Some(choice);
        }
    }
    let food = battle
        .items
        .iter()
        .position(|pack| pack.item == Item::Food && pack.count > 0)?;
    let choice = Choice::Item {
        pack: food,
        target: hurt,
    };
    battle.can_choose(actor, choice).then_some(choice)
}

fn expected(skill: &Skill, bite: u32, foes: usize) -> u32 {
    let reach = match skill.reach {
        Reach::AllFoes => foes as u32,
        Reach::TwoFoes => foes.min(2) as u32,
        Reach::OneFoe | Reach::RandomFoes(_) => 1,
        _ => 0,
    };
    let damage: u32 = skill
        .effects
        .iter()
        .map(|effect| match effect {
            Effect::Hit { power, hits, .. } => {
                let each = match power {
                    Power::Bite { percent } => bite * percent / PERCENT,
                    Power::Flat(n) => *n,
                    Power::PerVoice(n) => *n,
                };
                each * u32::from(*hits)
            }
            Effect::Inflict(status) if status.kind.ticks() => {
                status.amount * u32::from(status.turns)
            }
            _ => 0,
        })
        .sum();
    damage * reach
}

fn strike(
    battle: &Battle,
    actor: usize,
    target: usize,
    foes: usize,
    any_damage: bool,
) -> Option<Choice> {
    let me = &battle.fighters[actor];
    let bite = me.effective_bite();
    let (index, value) = me
        .skills
        .iter()
        .enumerate()
        .map(|(index, skill)| (index, expected(skill, bite, foes)))
        .filter(|&(index, _)| {
            battle.can_choose(
                actor,
                Choice::Skill {
                    skill: index,
                    target: Some(target),
                },
            )
        })
        .max_by_key(|&(_, value)| value)?;
    let worth_it = value > bite || (any_damage && value > 0);
    worth_it.then_some(Choice::Skill {
        skill: index,
        target: Some(target),
    })
}

#[cfg(test)]
mod tests;
