use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};

use crate::app::FIRST_FISH;
use crate::fishes::fish::Fish;
use crate::fishes::fused::FusedComponent;
use crate::fishes::mutations::{Mutation, apply_mutation};
use crate::fishes::species::{FishSpecies, SizeCategory};
use crate::loot::{ConsumableKind, MilkVariant, StockItem};

use super::Difficulty;
use super::battle::{Battle, BattleSetup, Choice, Outcome, Stakes};
use super::effect::{Effect, Power, Reach};
use super::fighter::{Fighter, Scale, Side};
use super::floor::{self, LAST_FLOOR};
use super::items::{Item, Pack};
use super::life::{DeepLife, Grade};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Room {
    Encounter,
    Den,
    Rest,
    Lair,
}

const PLAN: [Room; 9] = [
    Room::Encounter,
    Room::Encounter,
    Room::Rest,
    Room::Encounter,
    Room::Den,
    Room::Encounter,
    Room::Rest,
    Room::Encounter,
    Room::Lair,
];

#[derive(Clone, Copy, Debug)]
struct Wound {
    hp: Option<u32>,
    belly_up: bool,
}

struct Dive {
    party: Vec<Fish>,
    wounds: Vec<Wound>,
    items: Vec<Pack>,
    difficulty: Difficulty,
    scale: Scale,
    rng: SmallRng,
}

impl Dive {
    fn fight(&mut self, depth: u32, foes: Vec<Fighter>, stakes: Stakes) -> Result<(), String> {
        let cause = foes
            .iter()
            .map(|foe| foe.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let setup = BattleSetup {
            floor: depth,
            difficulty: self.difficulty,
            stakes,
            purse: 0,
            items: self.items.clone(),
            seed: self.rng.random(),
            ambushed: false,
            haunts: Vec::new(),
            scale: self.scale,
        };
        let mut battle = Battle::new(self.party.clone(), foes, setup);
        for (member, wound) in self.wounds.iter().enumerate() {
            if let Some(hp) = wound.hp {
                battle.carry(member, hp, wound.belly_up);
            }
        }
        battle.begin();
        for _ in 0..MAX_CHOICES {
            if battle.offer().is_some() {
                battle.answer(false);
                continue;
            }
            let Some(actor) = battle.actor() else {
                break;
            };
            if let Some(line) = battle.prayer.line().filter(|_| shielded_by_prayer(&battle)) {
                battle.pray(line);
            }
            let choice = decide(&battle, actor);
            if battle.choose(choice).is_empty() && battle.actor() == Some(actor) {
                battle.choose(Choice::Guard);
            }
        }
        let outcome = battle.outcome();
        let aftermath = battle.finish();
        self.items = aftermath.items;
        let alive: Vec<bool> = (0..aftermath.party.len())
            .map(|member| !aftermath.dead.contains(&member))
            .collect();
        self.wounds = (0..aftermath.party.len())
            .map(|member| {
                let (hp, belly_up) = aftermath
                    .hp
                    .iter()
                    .find(|(m, _, _)| *m == member)
                    .map_or((None, false), |(_, hp, down)| (Some(*hp), *down));
                Wound { hp, belly_up }
            })
            .collect();
        let (party, wounds) = aftermath
            .party
            .into_iter()
            .zip(std::mem::take(&mut self.wounds))
            .zip(alive)
            .filter_map(|(pair, alive)| alive.then_some(pair))
            .unzip();
        self.party = party;
        self.wounds = wounds;
        match outcome {
            Some(Outcome::Won | Outcome::Fled) => Ok(()),
            _ => Err(cause),
        }
    }

    fn dares(&self, foes: &[Fighter]) -> bool {
        let party: u32 = self
            .wounds
            .iter()
            .zip(&self.party)
            .map(|(wound, fish)| match wound.belly_up {
                true => 0,
                false => wound.hp.unwrap_or_else(|| super::kit::Kit::of(fish).vigor),
            })
            .sum();
        let threat: u32 = foes.iter().map(|foe| foe.max_hp).sum();
        party * PERCENT >= threat * DEN_NERVE_PERCENT
    }

    fn rest(&mut self) {
        for (member, wound) in self.wounds.iter_mut().enumerate() {
            let full = super::kit::Kit::of(&self.party[member]).vigor;
            let hp = wound.hp.unwrap_or(full);
            wound.hp = Some((hp + full * REST_PERCENT / PERCENT).min(full));
            wound.belly_up = false;
        }
    }

    fn clear(&mut self, depth: u32, dives: u32) -> Result<(), String> {
        for room in PLAN {
            let survived = match room {
                Room::Encounter => {
                    let foes = floor::encounter(depth, self.scale, &[], &mut self.rng);
                    self.fight(depth, foes, Stakes::Encounter)
                }
                Room::Den => {
                    let foes = floor::den(depth, self.scale, &mut self.rng);
                    match self.dares(&foes) {
                        true => self.fight(depth, foes, Stakes::Den),
                        false => Ok(()),
                    }
                }
                Room::Lair => {
                    let foes = floor::lair(depth, self.scale, dives);
                    self.fight(depth, foes, Stakes::Boss)
                }
                Room::Rest => {
                    self.rest();
                    Ok(())
                }
            };
            survived.map_err(|cause| format!("{room:?}: {cause}"))?;
            if self.party.is_empty() {
                return Err(format!("{room:?}: every fish died"));
            }
        }
        Ok(())
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
    let wounds = vec![
        Wound {
            hp: None,
            belly_up: false,
        };
        party.len()
    ];
    let mut dive = Dive {
        party,
        wounds,
        items: pack(),
        difficulty,
        scale,
        rng,
    };
    dive.clear(depth, seed as u32)
}

pub fn clears(tier: Tier, depth: u32, difficulty: Difficulty, seed: u64) -> bool {
    fate(tier, depth, difficulty, seed).is_ok()
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
