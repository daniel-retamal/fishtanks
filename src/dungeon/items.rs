use rand::RngExt;

use crate::entities::food::FOOD_WEIGHT_GAIN_G;
use crate::fishes::mutations::{Mutation, apply_mutation};
use crate::loot::{ConsumableKind, MilkVariant, StockItem};
use crate::restore::Restorable;

use super::battle::Battle;
use super::beat::Tone;
use super::effect::{DamageKind, Status, StatusKind};
use super::fighter::Side;
use super::traits::Trait;

pub const PACK_STACKS: usize = 8;
pub const MEAL_PELLETS: u32 = 6;
pub const HEAL_PER_PELLET: u32 = 2;
pub const COFFEE_TURNS: u8 = 3;
pub const MILK_ARMOUR: u32 = 2;
pub const SHARP_PERCENT: u32 = 15;
pub const MATCHA_SPEED: u32 = 2;
pub const JUNK_THROW: u32 = 3;
pub const JUNK_ARMOUR_MAX: u32 = 5;
pub const JUNKFISH_HEAL: u32 = 5;
pub const IRRADIATED_MIN: u8 = 10;
pub const IRRADIATED_MAX: u8 = 15;
pub const DEMON_CORE_MUTATIONS: u8 = 3;
pub const STEADY_TURNS: u8 = 99;
pub const GOLDEN_PEARL_PERCENT: u32 = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    Food,
    Stock(StockItem),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Aim {
    Ally,
    Foe,
    Nobody,
}

impl Item {
    pub fn for_a_fight() -> Vec<Item> {
        [Item::Food, Item::Stock(StockItem::Junk)]
            .into_iter()
            .chain(
                ConsumableKind::all()
                    .into_iter()
                    .map(|kind| Item::Stock(StockItem::Consumable(kind))),
            )
            .filter(|item| item.usable_in_a_fight())
            .collect()
    }

    pub fn name(self) -> &'static str {
        match self {
            Item::Food => "Food",
            Item::Stock(stock) => stock.display_name(),
        }
    }

    pub fn aim(self) -> Aim {
        match self {
            Item::Food => Aim::Ally,
            Item::Stock(StockItem::Junk) => Aim::Foe,
            Item::Stock(StockItem::Consumable(kind)) => match kind {
                ConsumableKind::Coffee | ConsumableKind::Milk(_) => Aim::Ally,
                ConsumableKind::VoidSeed => Aim::Foe,
                _ => Aim::Nobody,
            },
        }
    }

    pub fn usable_in_a_fight(self) -> bool {
        match self {
            Item::Food | Item::Stock(StockItem::Junk) => true,
            Item::Stock(StockItem::Consumable(kind)) => matches!(
                kind,
                ConsumableKind::Coffee
                    | ConsumableKind::Milk(_)
                    | ConsumableKind::Necronomicon
                    | ConsumableKind::DemonCore
                    | ConsumableKind::Computer
                    | ConsumableKind::VoidSeed
                    | ConsumableKind::GoldenPearl
            ),
        }
    }

    pub fn about(self) -> &'static str {
        match self {
            Item::Food => "One fish eats. Food also rights a belly-up fish.",
            Item::Stock(StockItem::Junk) => {
                "Thrown at an enemy, worn by a Crabfish, or eaten by a Junkfish."
            }
            Item::Stock(StockItem::Consumable(kind)) => match kind {
                ConsumableKind::Coffee => "A second action every other turn, for three turns.",
                ConsumableKind::Milk(MilkVariant::Plain) => {
                    "Less from every hit for the rest of the fight."
                }
                ConsumableKind::Milk(MilkVariant::Blueberry) => "Sharper: more accuracy and crit.",
                ConsumableKind::Milk(MilkVariant::Honey) => {
                    "Calm: fear, charm and sleep slide off."
                }
                ConsumableKind::Milk(MilkVariant::Matcha) => "Quicker.",
                ConsumableKind::Milk(MilkVariant::Chocolate) => "A segment of mass, for good.",
                ConsumableKind::Milk(MilkVariant::Strawberry) => "Sweet, for good.",
                ConsumableKind::Milk(MilkVariant::Alien) => "Alienated, for good.",
                ConsumableKind::Milk(MilkVariant::Vanilla) => {
                    "Wipes every debuff, mark and mutation."
                }
                ConsumableKind::Milk(MilkVariant::Irradiated) => {
                    "Ten to fifteen mutations, for good."
                }
                ConsumableKind::Necronomicon => "Every enemy is damned.",
                ConsumableKind::DemonCore => "Three mutations on every fighter.",
                ConsumableKind::Computer => "Every botfish acts twice as often for three turns.",
                ConsumableKind::VoidSeed => "One enemy is erased.",
                ConsumableKind::GoldenPearl => "Every fish that fell comes back.",
                _ => "Not for a fight.",
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Pack {
    pub item: Item,
    pub count: u32,
    pub held: u32,
}

impl Pack {
    pub fn of(item: Item, count: u32) -> Pack {
        Pack {
            item,
            count,
            held: 0,
        }
    }
}

pub fn apply(battle: &mut Battle, item: Item, target: usize) {
    match item {
        Item::Food => feed(battle, target),
        Item::Stock(StockItem::Junk) => junk(battle, target),
        Item::Stock(StockItem::Consumable(kind)) => consume(battle, kind, target),
    }
}

fn feed(battle: &mut Battle, target: usize) {
    let candy = battle
        .standing(Side::Party)
        .iter()
        .any(|&i| battle.fighters[i].has(Trait::Candy));
    let mut heal = MEAL_PELLETS * HEAL_PER_PELLET;
    if candy {
        heal *= CANDY_TIMES;
    }
    if battle.fighters[target].has(Trait::GluttonySin) {
        heal *= 2;
    }
    battle.fighters[target].clear_status(StatusKind::Rot);
    if let Some(member) = battle.fighters[target].member() {
        battle.party[member].eat(FOOD_WEIGHT_GAIN_G * MEAL_PELLETS);
        battle.refit(target);
    }
    battle.heal_with(target, heal, true);
}

fn junk(battle: &mut Battle, target: usize) {
    let name = battle.fighters[target].name.clone();
    match battle.fighters[target].side {
        Side::Foes => {
            let thrower = battle.current.unwrap_or(target);
            battle.note(format!("Junk hits {name} for {JUNK_THROW}."), Tone::Hit);
            battle.harm(thrower, target, JUNK_THROW, DamageKind::Skill);
        }
        Side::Party if battle.fighters[target].has(Trait::JunkArmour) => {
            let f = &mut battle.fighters[target];
            f.armour = (f.armour + 1).min(JUNK_ARMOUR_MAX);
            battle.note(format!("{name} wears the Junk."), Tone::Status);
        }
        Side::Party if battle.fighters[target].has(Trait::Junkfish) => {
            battle.heal(target, JUNKFISH_HEAL)
        }
        Side::Party => battle.note("Nothing comes of it.".to_string(), Tone::Quiet),
    }
}

fn consume(battle: &mut Battle, kind: ConsumableKind, target: usize) {
    match kind {
        ConsumableKind::Coffee => {
            let speed = battle.fighters[target].speed;
            battle.fighters[target].inflict(Status::new(StatusKind::Hasted, COFFEE_TURNS, speed));
            battle.fighters[target]
                .traits
                .retain(|t| *t != Trait::NightOwl);
            let name = battle.fighters[target].name.clone();
            battle.note(format!("{name} is wide awake."), Tone::Status);
        }
        ConsumableKind::Milk(variant) => milk(battle, variant, target),
        ConsumableKind::Necronomicon => {
            for foe in battle.standing(Side::Foes) {
                battle.fighters[foe].traits.push(Trait::DevilMarked);
            }
            battle.note("Every enemy is damned.".to_string(), Tone::Status);
        }
        ConsumableKind::DemonCore => {
            let everyone: Vec<usize> = (0..battle.fighters.len())
                .filter(|&i| battle.fighters[i].standing())
                .collect();
            for fighter in everyone {
                battle.mutate(fighter, DEMON_CORE_MUTATIONS);
            }
        }
        ConsumableKind::Computer => {
            for fish in battle.standing(Side::Party) {
                let programmable = battle.fighters[fish]
                    .member()
                    .is_some_and(|m| battle.party[m].is_programmable());
                if programmable {
                    let speed = battle.fighters[fish].speed;
                    battle.fighters[fish].inflict(Status::new(
                        StatusKind::Hasted,
                        COFFEE_TURNS,
                        speed,
                    ));
                }
            }
            battle.note(
                "Every botfish runs twice as fast.".to_string(),
                Tone::Status,
            );
        }
        ConsumableKind::VoidSeed => {
            let name = battle.fighters[target].name.clone();
            battle.note(format!("{name} is erased."), Tone::Down);
            battle.beat(target);
        }
        ConsumableKind::GoldenPearl => {
            let down: Vec<usize> = battle
                .side(Side::Party)
                .filter(|&i| battle.fighters[i].belly_up && !battle.fighters[i].gone)
                .collect();
            for fish in down {
                battle.revive(fish, GOLDEN_PEARL_PERCENT);
            }
        }
        _ => {}
    }
}

fn milk(battle: &mut Battle, variant: MilkVariant, target: usize) {
    let name = battle.fighters[target].name.clone();
    match variant {
        MilkVariant::Plain => {
            battle.fighters[target].armour += MILK_ARMOUR;
            battle.note(format!("{name} hardens."), Tone::Status);
        }
        MilkVariant::Blueberry => {
            battle.fighters[target].crit += SHARP_PERCENT;
            battle.note(format!("{name} sharpens."), Tone::Status);
        }
        MilkVariant::Honey => {
            battle.fighters[target].inflict(Status::for_turns(StatusKind::Steady, STEADY_TURNS));
            battle.fighters[target].clear_status(StatusKind::Charmed);
            battle.note(format!("{name} calms."), Tone::Status);
        }
        MilkVariant::Matcha => {
            battle.fighters[target].speed += MATCHA_SPEED;
            battle.note(format!("{name} quickens."), Tone::Status);
        }
        MilkVariant::Chocolate => wear(battle, target, Mutation::SizeIncrease),
        MilkVariant::Strawberry => wear(battle, target, Mutation::Strawberry),
        MilkVariant::Alien => wear(battle, target, Mutation::Alienation),
        MilkVariant::Vanilla => {
            if let Some(member) = battle.fighters[target].member() {
                battle.party[member].restore();
                battle.refit(target);
            }
            battle.fighters[target]
                .statuses
                .retain(|s| !s.kind.is_debuff());
            battle.note(format!("{name} is itself again."), Tone::Status);
        }
        MilkVariant::Irradiated => {
            let count = battle.rng.random_range(IRRADIATED_MIN..=IRRADIATED_MAX);
            battle.mutate(target, count);
        }
    }
}

fn wear(battle: &mut Battle, target: usize, mutation: Mutation) {
    let Some(member) = battle.fighters[target].member() else {
        return;
    };
    apply_mutation(&mut battle.party[member], mutation, &mut battle.rng);
    battle.refit(target);
    let name = battle.fighters[target].name.clone();
    battle.note(format!("{name} changes for good."), Tone::Status);
}

pub const CANDY_TIMES: u32 = 10;
