use rand::RngExt;

use super::ai::{self, Plan};
use super::battle::Battle;
use super::beat::Tone;
use super::fighter::Side;
use super::traits::Trait;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Foresight {
    pub plan: Plan,
    pub told: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reach {
    Every,
    Next,
    Known,
    Bosses,
}

pub fn assemble(battle: &mut Battle) {
    let party: Vec<usize> = battle
        .side(Side::Party)
        .filter(|&i| battle.fighters[i].member().is_some())
        .collect();
    if party.is_empty() {
        return;
    }
    let mirages: Vec<usize> = battle
        .side(Side::Foes)
        .filter(|&i| battle.fighters[i].has(Trait::Mirage))
        .collect();
    for mirage in mirages {
        let worn = party[battle.rng.random_range(0..party.len())];
        battle.fighters[mirage].disguise = battle.fighters[worn].member();
    }
}

pub fn foresee(battle: &mut Battle) {
    let unplanned: Vec<usize> = battle
        .standing(Side::Foes)
        .into_iter()
        .filter(|&i| battle.fighters[i].foresight.is_none())
        .collect();
    for foe in unplanned {
        let plan = ai::plan(battle, foe);
        battle.fighters[foe].foresight = Some(Foresight { plan, told: false });
    }
    tell(battle);
}

pub fn next_move(battle: &mut Battle, actor: usize) -> Plan {
    match battle.fighters[actor].foresight.take() {
        Some(seen) if holds(battle, actor, seen.plan) => seen.plan,
        _ => ai::plan(battle, actor),
    }
}

fn holds(battle: &Battle, actor: usize, plan: Plan) -> bool {
    let side = battle.fighters[actor].side;
    match plan {
        Plan::Bite(target) => battle.within_reach(side.other()).contains(&target),
        Plan::Skill { skill, target } => {
            let Some(skill) = battle.fighters[actor].skills.get(skill) else {
                return false;
            };
            let aimed = match target {
                Some(t) if skill.reach.picks_a_foe() => {
                    battle.within_reach(side.other()).contains(&t)
                }
                Some(t) => battle.fighters[t].in_the_fight(),
                None => true,
            };
            battle.affordable(actor, skill) && aimed
        }
        Plan::Idle => true,
    }
}

fn tell(battle: &mut Battle) {
    let next_foe = battle
        .turn_order(super::battle::TURN_BAR_LENGTH)
        .into_iter()
        .find(|&i| battle.fighters[i].side == Side::Foes);
    let foes: Vec<usize> = battle.standing(Side::Foes);
    for foe in foes {
        let Some(seen) = battle.fighters[foe].foresight else {
            continue;
        };
        if seen.told {
            continue;
        }
        let Some(reader) = reader_of(battle, foe, next_foe == Some(foe)) else {
            continue;
        };
        let reader_name = battle.fighters[reader].name.clone();
        let foe_name = battle.fighters[foe].name.clone();
        let deed = deed(battle, foe, seen.plan);
        battle.note(
            format!("{reader_name} reads {foe_name}: it will {deed} next."),
            Tone::Tell,
        );
        if let Some(seen) = battle.fighters[foe].foresight.as_mut() {
            seen.told = true;
        }
    }
}

fn reader_of(battle: &Battle, foe: usize, next: bool) -> Option<usize> {
    let party = battle.standing(Side::Party);
    let graeae = party
        .iter()
        .filter(|&&i| battle.fighters[i].has(Trait::Graeae))
        .count();
    party.into_iter().find(|&reader| {
        reaches(battle, reader, graeae)
            .iter()
            .any(|reach| match reach {
                Reach::Every => true,
                Reach::Next => next,
                Reach::Known => battle.setup.known.contains(&battle.fighters[foe].base),
                Reach::Bosses => battle.fighters[foe].is_boss(),
            })
    })
}

fn reaches(battle: &Battle, reader: usize, graeae: usize) -> Vec<Reach> {
    let fighter = &battle.fighters[reader];
    let mut reaches = Vec::new();
    if fighter.has(Trait::Telepathy)
        || fighter.has(Trait::Electrosense)
        || fighter.has(Trait::Cochlea)
    {
        reaches.push(Reach::Every);
    }
    if fighter.has(Trait::Gossip) || fighter.has(Trait::Overhears) {
        reaches.push(Reach::Next);
    }
    if fighter.has(Trait::CoreStack) {
        reaches.push(Reach::Known);
    }
    if fighter.has(Trait::Graeae) && graeae > 0 && battle.setup.every_graeae {
        reaches.push(Reach::Bosses);
    }
    reaches
}

fn deed(battle: &Battle, foe: usize, plan: Plan) -> String {
    let name = |i: usize| battle.fighters[i].name.clone();
    match plan {
        Plan::Bite(target) => format!("bite {}", name(target)),
        Plan::Skill { skill, target } => {
            let skill = battle.fighters[foe].skills[skill].name;
            match target {
                Some(t) => format!("use {skill} on {}", name(t)),
                None => format!("use {skill}"),
            }
        }
        Plan::Idle => "do nothing".to_string(),
    }
}

pub fn gossip(battle: &mut Battle, item: &str) {
    let Some(gossip) = battle
        .standing(Side::Party)
        .into_iter()
        .find(|&i| battle.fighters[i].has(Trait::Gossip))
    else {
        return;
    };
    let name = battle.fighters[gossip].name.clone();
    battle.note(
        format!("{name} tells everyone about the {item}."),
        Tone::Quiet,
    );
}

pub fn assayed(battle: &Battle) -> bool {
    battle
        .standing(Side::Party)
        .iter()
        .any(|&i| battle.fighters[i].has(Trait::Assay))
}

#[cfg(test)]
mod tests;
