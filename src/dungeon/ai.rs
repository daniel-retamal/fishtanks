use rand::RngExt;

use super::battle::{BASE_TARGET_WEIGHT, Battle, HARDCORE_BELLY_UP_WEIGHT, MAX_FIGHTERS};
use super::effect::{Effect, Reach, Special, StatusKind};
use super::fighter::Side;
use super::skill::Skill;
use super::traits::Trait;

pub const SKILL_CHANCE_PERCENT: u32 = 65;
pub const BIGGEST_FIRST_PERCENT: u32 = 60;
pub const HURT_ALLY_PERCENT: u32 = 80;
pub const MAX_SUMMONED: usize = 4;
pub const LURE_TIMES: u32 = 3;
pub const SHINY_TIMES: u32 = 2;
pub const BOSS_CHALLENGE_TIMES: u32 = 3;
pub const SMALL_ENEMY_HP: u32 = 60;
const PERCENT: u32 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plan {
    Skill { skill: usize, target: Option<usize> },
    Bite(usize),
    Idle,
}

pub fn plan(battle: &mut Battle, actor: usize) -> Plan {
    let fighter = &battle.fighters[actor];
    if let Some(told) = fighter.tell
        && fighter
            .skills
            .get(told)
            .is_some_and(|skill| battle.affordable(actor, skill))
    {
        let reach = fighter.skills[told].reach;
        let target = aim(battle, actor, reach);
        return Plan::Skill {
            skill: told,
            target,
        };
    }
    let usable: Vec<usize> = (0..battle.fighters[actor].skills.len())
        .filter(|&i| {
            let skill = &battle.fighters[actor].skills[i];
            battle.affordable(actor, skill) && sensible(battle, actor, skill)
        })
        .collect();
    let bites = battle.fighters[actor]
        .spec()
        .is_none_or(|spec| spec.bites());
    let wants_a_skill = !bites || battle.rng.random_range(0..PERCENT) < SKILL_CHANCE_PERCENT;
    if !usable.is_empty() && wants_a_skill {
        let biggest = usable
            .iter()
            .copied()
            .max_by_key(|&i| battle.fighters[actor].skills[i].bubbles())
            .unwrap_or(usable[0]);
        let skill = match battle.rng.random_range(0..PERCENT) < BIGGEST_FIRST_PERCENT {
            true => biggest,
            false => usable[battle.rng.random_range(0..usable.len())],
        };
        let reach = battle.fighters[actor].skills[skill].reach;
        let target = aim(battle, actor, reach);
        if !reach.picks_a_foe() || target.is_some() {
            return Plan::Skill { skill, target };
        }
    }
    if !bites {
        return Plan::Idle;
    }
    match pick_target(battle, actor) {
        Some(target) => Plan::Bite(target),
        None => Plan::Idle,
    }
}

fn aim(battle: &mut Battle, actor: usize, reach: Reach) -> Option<usize> {
    match reach {
        Reach::OneFoe | Reach::TwoFoes => pick_target(battle, actor),
        Reach::BellyUpFoe => {
            let side = battle.fighters[actor].side.other();
            let down: Vec<usize> = battle
                .side(side)
                .filter(|&i| battle.fighters[i].belly_up && !battle.fighters[i].gone)
                .collect();
            match down.is_empty() {
                true => None,
                false => Some(down[battle.rng.random_range(0..down.len())]),
            }
        }
        Reach::OneAlly => {
            let side = battle.fighters[actor].side;
            battle
                .standing(side)
                .into_iter()
                .filter(|&i| i != actor)
                .min_by_key(|&i| battle.fighters[i].hp_percent())
        }
        _ => None,
    }
}

fn takes_hold(battle: &Battle, actor: usize, skill: &Skill) -> bool {
    let kinds: Vec<StatusKind> = skill
        .effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Inflict(status) => Some(status.kind),
            _ => None,
        })
        .collect();
    if kinds.len() < skill.effects.len() {
        return true;
    }
    let side = battle.fighters[actor].side;
    let targets: Vec<usize> = match skill.reach {
        Reach::Me => vec![actor],
        Reach::OneFoe
        | Reach::TwoFoes
        | Reach::AllFoes
        | Reach::RandomFoes(_)
        | Reach::MarkedFoes
        | Reach::BellyUpFoe => battle.standing(side.other()),
        Reach::Everyone => (0..battle.fighters.len())
            .filter(|&i| battle.fighters[i].standing())
            .collect(),
        Reach::OneAlly
        | Reach::OtherAllies
        | Reach::AllAllies
        | Reach::BellyUpAlly
        | Reach::FallenAlly
        | Reach::WeakestAlly => battle.standing(side),
    };
    targets.iter().any(|&target| {
        kinds
            .iter()
            .any(|&kind| !battle.fighters[target].has_status(kind))
    })
}

fn sensible(battle: &Battle, actor: usize, skill: &Skill) -> bool {
    if !takes_hold(battle, actor, skill) {
        return false;
    }
    let side = battle.fighters[actor].side;
    let allies: Vec<usize> = battle.standing(side);
    skill.effects.iter().all(|effect| match effect {
        Effect::Heal(_) | Effect::HealPercent(_) => match skill.reach {
            Reach::Me => battle.fighters[actor].hp_percent() < HURT_ALLY_PERCENT,
            _ => allies
                .iter()
                .any(|&i| i != actor && battle.fighters[i].hp_percent() < HURT_ALLY_PERCENT),
        },
        Effect::Shield(_) => match skill.reach {
            Reach::Me => battle.fighters[actor].shield == 0,
            _ => allies.iter().any(|&i| battle.fighters[i].shield == 0),
        },
        Effect::Summon { enemy, .. } => {
            battle.fighters.len() < MAX_FIGHTERS
                && battle
                    .standing(side)
                    .iter()
                    .filter(|&&i| battle.fighters[i].base == *enemy)
                    .count()
                    < MAX_SUMMONED
        }
        Effect::Split => battle.fighters.len() < MAX_FIGHTERS,
        Effect::StealItem => battle.items.iter().any(|pack| pack.count > 0),
        Effect::StealGold { .. } => battle.haul > 0,
        Effect::Special(Special::Reap | Special::Swallow) => battle
            .side(side.other())
            .any(|i| battle.fighters[i].belly_up && !battle.fighters[i].gone),
        Effect::Special(Special::Grace) => battle
            .side(side)
            .any(|i| battle.fighters[i].gone && !battle.fighters[i].is_boss()),
        Effect::Special(Special::Optimise) => {
            battle.fighters[actor].hp < battle.fighters[actor].last_hp
        }
        Effect::Special(Special::Copy) => battle.last_party_skill.is_some(),
        Effect::Special(Special::Fork) => {
            battle.fighters.len() < MAX_FIGHTERS
                && !battle
                    .standing(side)
                    .iter()
                    .any(|&i| i != actor && battle.fighters[i].has(Trait::Background))
        }
        _ => true,
    })
}

pub fn pick_target(battle: &mut Battle, actor: usize) -> Option<usize> {
    let side = battle.fighters[actor].side.other();
    let candidates = battle.within_reach(side);
    let weights: Vec<u32> = candidates
        .iter()
        .map(|&i| weight(battle, actor, i))
        .collect();
    let total: u32 = weights.iter().sum();
    if total == 0 {
        return None;
    }
    let mut roll = battle.rng.random_range(0..total);
    for (&index, &w) in candidates.iter().zip(weights.iter()) {
        if roll < w {
            return Some(index);
        }
        roll -= w;
    }
    None
}

fn weight(battle: &Battle, actor: usize, target: usize) -> u32 {
    let t = &battle.fighters[target];
    let attacker = &battle.fighters[actor];
    if t.has_status(StatusKind::Untouchable)
        || t.has_status(StatusKind::OutOfReach)
        || t.has_status(StatusKind::Away)
        || t.has(Trait::Absent)
    {
        return 0;
    }
    if t.has(Trait::CounterLight) && battle.round <= COUNTER_LIGHT_ROUNDS {
        return 0;
    }
    let mut w = BASE_TARGET_WEIGHT;
    if t.belly_up {
        w = HARDCORE_BELLY_UP_WEIGHT;
    }
    for tr in &t.traits {
        match tr {
            Trait::Conspicuous(percent) => w = w * (PERCENT + percent) / PERCENT,
            Trait::Lure if attacker.max_hp <= SMALL_ENEMY_HP => w *= LURE_TIMES,
            Trait::Shimmer { .. } => w = w * (PERCENT + SHINY_EXTRA) / PERCENT,
            Trait::SharkFin if attacker.is_boss() => w *= BOSS_CHALLENGE_TIMES,
            Trait::Delicious => w *= SHINY_TIMES,
            Trait::Hide if t.hits_taken == 0 => w /= SHINY_TIMES,
            _ => {}
        }
    }
    let undead = attacker.has(Trait::Undying) || attacker.has(Trait::Ethereal);
    if attacker.side == Side::Foes && t.has(Trait::Forbidden) && undead {
        return 0;
    }
    (w * super::passives::target_weight(battle, actor, target) / PERCENT).max(1)
}

pub const COUNTER_LIGHT_ROUNDS: u32 = 2;
pub const SHINY_EXTRA: u32 = 50;
