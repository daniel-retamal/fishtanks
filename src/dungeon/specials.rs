use rand::RngExt;

use crate::fishes::fish::Fish;
use crate::fishes::mutations::{Mutatable, Mutation};
use crate::fishes::species::FishSpecies;

use super::battle::{Battle, HALF, MAX_FIGHTERS};
use super::beat::Tone;
use super::effect::{DamageKind, Power, Special, Status, StatusKind};
use super::fighter::{Fighter, Side};
use super::kit::Kit;
use super::moves;
use super::skill::Skill;
use super::traits::Trait;

pub const BRIBE_PER_FLOOR: u128 = 200;
pub const PAY_TO_WIN_SCALE: f64 = 6.0;
pub const MELTDOWN_MUTATIONS: u8 = 3;
pub const SUGAR_MASS_G: u32 = 100;
pub const DEVOUR_MASS_PER_HP_G: u32 = 10;
pub const LURK_TIMES: u32 = 2;
pub const CORRODE_ARMOUR: u32 = 3;
pub const CHARGE_TIMES: u32 = 2;
pub const SWALLOW_TURNS: u8 = 2;
pub const SWALLOW_DAMAGE: u32 = 10;
pub const PLANK_TURNS: u8 = 2;
pub const FOREVER: u8 = u8::MAX;
pub const FANFARE_BITE: u32 = 4;
pub const FANFARE_TURNS: u8 = 99;
pub const HYMN_HEAL: u32 = 30;
pub const HYMN_DAMAGE: u32 = 14;
pub const MIRACLE_FEED: u32 = 20;
pub const MIRACLE_SMITE: u32 = 18;
pub const CLING_TURNS: u8 = 99;
pub const ECLIPSE_ROUND: u32 = 6;
pub const DOMINATED_TURNS: u8 = 1;
const PERCENT: u32 = 100;

pub fn wearable_mutations(fish: &Fish) -> Vec<Mutation> {
    fish.capabilities()
        .iter()
        .copied()
        .filter(|m| m.auto_selectable())
        .collect()
}

pub fn perform(
    battle: &mut Battle,
    actor: usize,
    special: Special,
    targets: &[usize],
    skill: &Skill,
) {
    let name = battle.fighters[actor].name.clone();
    let first = targets.first().copied();
    match special {
        Special::Bribe => {
            let Some(target) = first.filter(|&t| !battle.fighters[t].is_boss()) else {
                battle.note("The bribe is refused.", Tone::Quiet);
                return;
            };
            let price = BRIBE_PER_FLOOR * u128::from(battle.setup.floor.max(1));
            if battle.purse < price {
                battle.note("There is not enough in your purse.", Tone::Quiet);
                return;
            }
            battle.purse -= price;
            battle.purse_spent += price;
            let target_name = battle.fighters[target].name.clone();
            battle.fighters[target].gone = true;
            battle.note(
                format!("{target_name} takes ${price} and leaves."),
                Tone::Gold,
            );
        }
        Special::PayToWin => {
            let Some(target) = first else {
                return;
            };
            let spent = battle.purse_spent.max(1) as f64;
            let paid = battle.fighters[target].has(Trait::Paid);
            let mut damage = (PAY_TO_WIN_SCALE * spent.log10()).round().max(1.0) as u32;
            if paid {
                damage *= 2;
            }
            let target_name = battle.fighters[target].name.clone();
            battle.note(
                format!("Money leaves your purse. {target_name} takes {damage}."),
                Tone::Gold,
            );
            battle.harm(actor, target, damage, DamageKind::True);
        }
        Special::Radiate => {
            let Some(target) = first.or_else(|| battle.pick_target(actor)) else {
                return;
            };
            battle.mutate(target, 1);
        }
        Special::Meltdown => {
            let everyone: Vec<usize> = (0..battle.fighters.len())
                .filter(|&i| battle.fighters[i].standing())
                .collect();
            for fighter in everyone {
                battle.mutate(fighter, MELTDOWN_MUTATIONS);
            }
        }
        Special::SugarRush => {
            let stacks = stacks_of(battle, actor, FishSpecies::Candyfish).max(1);
            for &target in targets {
                if let Some(member) = battle.fighters[target].member() {
                    let fish = &mut battle.party[member];
                    fish.weight_g = fish.weight_g.saturating_add(SUGAR_MASS_G * stacks);
                    battle.refit(target);
                }
            }
        }
        Special::Candify => {
            let Some(target) = first else {
                return;
            };
            let t = &battle.fighters[target];
            if t.is_boss() || t.hp_percent() >= moves::CANDIFY_BELOW_PERCENT {
                battle.note("It is too strong to turn into candy.", Tone::Quiet);
                return;
            }
            let target_name = t.name.clone();
            battle.note(format!("{target_name} turns into food."), Tone::Gold);
            battle
                .items
                .push(super::items::Pack::of(super::items::Item::Food, 1));
            battle.beat(target);
        }
        Special::Miracle => miracle(battle, actor),
        Special::Script => {
            if let Some(target) = battle.pick_target(actor) {
                battle.bite(actor, target);
            }
        }
        Special::Divide => divide(battle, actor),
        Special::Collapse => {
            let max = battle.fighters[actor].max_hp;
            battle.fighters[actor].hp = max;
            battle.note(
                format!("{name} collapses into itself and is whole again."),
                Tone::Heal,
            );
        }
        Special::Devour => devour(battle, actor, first),
        Special::CallHome => call_home(battle, actor),
        Special::Duel => {
            if let Some(target) = first {
                battle.fighters[actor].inflict(Status::for_turns(StatusKind::BiteUp, FOREVER));
                let target_name = battle.fighters[target].name.clone();
                battle.note(
                    format!("{name} and {target_name} square off."),
                    Tone::Status,
                );
            }
        }
        Special::Chase => {
            if let Some(target) = first {
                let wait = battle.wait_of(target);
                battle.fighters[target].ready_at += wait;
                let target_name = battle.fighters[target].name.clone();
                battle.note(
                    format!("{target_name} flees in a zoomie and loses its next turn."),
                    Tone::Status,
                );
            }
        }
        Special::Mime | Special::Copy | Special::Echo => {
            let copied = match battle.fighters[actor].side {
                Side::Party if special == Special::Echo => battle.last_party_skill.clone(),
                Side::Party => battle.last_enemy_skill.clone(),
                Side::Foes => battle.last_party_skill.clone(),
            };
            match copied {
                Some(copied)
                    if !copied
                        .effects
                        .contains(&super::effect::Effect::Special(special)) =>
                {
                    let free = Skill {
                        price: super::skill::Price::Free,
                        ..copied
                    };
                    battle.use_skill(actor, &free, None);
                }
                _ => battle.note(format!("{name} has nothing to copy."), Tone::Quiet),
            }
        }
        Special::Ricochet => {
            let foes = battle.standing(battle.fighters[actor].side.other());
            let mut struck = 0;
            for &target in &foes {
                let strike = battle.strike(actor, target, Power::BITE, DamageKind::Bite);
                if strike.landed {
                    struck += 1;
                    let target_name = battle.fighters[target].name.clone();
                    battle.note(
                        format!("The bounce hits {target_name} for {}.", strike.damage),
                        Tone::Hit,
                    );
                }
            }
            if struck == foes.len() && struck > 0 {
                battle.note("Corner! Every fish gains a bubble.", Tone::Gold);
                for ally in battle.standing(battle.fighters[actor].side) {
                    battle.fighters[ally].gain_bubbles(1);
                }
            }
        }
        Special::Hop => {}
        Special::Lurk => {
            battle.fighters[actor].inflict(Status::for_turns(StatusKind::OutOfReach, 1));
            let bite = battle.fighters[actor].bite * (LURK_TIMES - 1);
            battle.fighters[actor].inflict(Status::new(StatusKind::BiteUp, 2, bite));
            battle.note(format!("{name} slips into the wall."), Tone::Status);
        }
        Special::Heart => {
            let partner = battle
                .standing(battle.fighters[actor].side)
                .into_iter()
                .find(|&i| {
                    i != actor
                        && battle.fighters[i]
                            .traits
                            .iter()
                            .any(|t| matches!(t, Trait::Parrot))
                });
            if let Some(partner) = partner {
                battle.heal(partner, moves::HEART);
            }
        }
        Special::HandMeDown => {
            if let Some(target) = first {
                let shell = battle.fighters[actor].armour;
                battle.fighters[actor].armour = 0;
                battle.fighters[target].armour += shell;
                let target_name = battle.fighters[target].name.clone();
                battle.note(
                    format!("{name} gives its shell to {target_name}."),
                    Tone::Status,
                );
            }
        }
        Special::Kiss => {
            if let Some(target) =
                first.filter(|&t| battle.fighters[t].side != battle.fighters[actor].side)
            {
                battle.inflict(actor, target, Status::for_turns(StatusKind::Charmed, 1));
            }
        }
        Special::Corrode => {
            if let Some(target) = first {
                let f = &mut battle.fighters[target];
                f.armour = f.armour.saturating_sub(CORRODE_ARMOUR);
                let target_name = f.name.clone();
                battle.note(format!("{target_name}'s armour rusts away."), Tone::Status);
            }
        }
        Special::Wear => {
            if let Some(target) = first {
                let (bite, speed) = (battle.fighters[target].bite, battle.fighters[target].speed);
                let f = &mut battle.fighters[actor];
                f.bite = f.bite.max(bite);
                f.speed = f.speed.max(speed);
                battle.note(format!("{name} wears another's look."), Tone::Status);
            }
        }
        Special::CrossOver => {
            if let Some(target) = first {
                battle.fighters[target].inflict(Status::for_turns(StatusKind::Away, FOREVER));
                let target_name = battle.fighters[target].name.clone();
                battle.note(
                    format!("{target_name} crosses over, home through the glass."),
                    Tone::Status,
                );
            }
        }
        Special::Roll => {
            let eyes = battle.fighters[actor]
                .member()
                .map_or(1, |m| battle.party[m].eyes_a_side().max(1) * HALF as usize);
            let foes = battle.standing(battle.fighters[actor].side.other());
            for target in foes.into_iter().cycle().take(eyes) {
                if !battle.fighters[target].standing() {
                    continue;
                }
                let strike = battle.strike(actor, target, Power::BITE, DamageKind::Bite);
                let target_name = battle.fighters[target].name.clone();
                if strike.landed {
                    battle.note(
                        format!("{name} rolls through {target_name} for {}.", strike.damage),
                        Tone::Hit,
                    );
                }
            }
        }
        Special::Dominate => {
            if let Some(target) = first.filter(|&t| !battle.fighters[t].is_boss()) {
                battle.inflict(
                    actor,
                    target,
                    Status::for_turns(StatusKind::Charmed, DOMINATED_TURNS),
                );
            }
        }
        Special::Erase | Special::Amnesia => {
            if let Some(target) = first.or_else(|| battle.pick_target(actor)) {
                let f = &mut battle.fighters[target];
                f.tell = None;
                f.statuses.retain(|s| s.kind.is_debuff());
                f.shield = 0;
                let target_name = f.name.clone();
                battle.note(format!("{target_name} forgets."), Tone::Status);
            }
        }
        Special::Cling => {
            if let Some(target) = first {
                battle.inflict(
                    actor,
                    target,
                    Status::new(StatusKind::Bleed, CLING_TURNS, moves::CLING_DRAIN),
                );
            }
        }
        Special::Static => {}
        Special::Touch | Special::Invert => {
            if let Some(target) = first {
                let f = &mut battle.fighters[target];
                for status in f.statuses.iter_mut() {
                    if let Some(kind) = status.kind.mirror() {
                        status.kind = kind;
                    }
                }
                let target_name = f.name.clone();
                battle.note(format!("{target_name} turns inside out."), Tone::Status);
            }
        }
        Special::Turn => {
            battle.note(format!("{name} turns to its other side."), Tone::Status);
            let half = battle.fighters[actor].max_hp / HALF;
            battle.fighters[actor].heal(half);
        }
        Special::Shuffle => {
            let foes = battle.standing(battle.fighters[actor].side.other());
            let mut bites: Vec<u32> = foes.iter().map(|&i| battle.fighters[i].bite).collect();
            for index in (1..bites.len()).rev() {
                let swap = battle.rng.random_range(0..=index);
                bites.swap(index, swap);
            }
            for (&foe, bite) in foes.iter().zip(bites) {
                battle.fighters[foe].bite = bite;
            }
            battle.note("The enemies' strengths are dealt out again.", Tone::Status);
        }
        Special::Shed => {
            battle.fighters[actor].shield += battle.fighters[actor].max_hp / HALF;
            battle.note(format!("{name} sheds a grey skin."), Tone::Status);
        }
        Special::Reverse => battle.note("The next round runs backwards.", Tone::Status),
        Special::Eclipse => {
            if battle.round < ECLIPSE_ROUND {
                battle.note("The moon is not there yet.", Tone::Quiet);
                return;
            }
            for foe in battle.standing(battle.fighters[actor].side.other()) {
                let wait = battle.wait_of(foe);
                battle.fighters[foe].ready_at += wait;
            }
            battle.note(
                "The moon covers the sun. Every enemy loses its next turn.",
                Tone::Status,
            );
        }
        Special::Charge => {
            let bite = battle.fighters[actor].bite * (CHARGE_TIMES - 1);
            battle.fighters[actor].inflict(Status::new(StatusKind::BiteUp, 2, bite));
            battle.note(format!("{name} charges."), Tone::Status);
        }
        Special::BlastOff => {
            let wait = battle.wait_of(actor);
            battle.fighters[actor].ready_at += wait;
        }
        Special::Kick => {
            if let Some(target) = first {
                let wait = battle.wait_of(target) / u64::from(HALF);
                battle.fighters[target].ready_at += wait;
                let target_name = battle.fighters[target].name.clone();
                battle.note(
                    format!("{name} kicks {target_name} back in the line."),
                    Tone::Status,
                );
            }
        }
        Special::Alarm => {
            for &target in targets {
                if battle.fighters[target].max_hp <= super::ai::SMALL_ENEMY_HP {
                    battle.inflict(actor, target, Status::for_turns(StatusKind::Held, 1));
                }
            }
        }
        Special::ThrowJunk => {
            if let Some(target) = first {
                battle.harm(actor, target, moves::THROWN_JUNK, DamageKind::Skill);
                let target_name = battle.fighters[target].name.clone();
                battle.note(
                    format!("Junk hits {target_name} for {}.", moves::THROWN_JUNK),
                    Tone::Hit,
                );
            }
        }
        Special::WalkThePlank => {
            if let Some(target) = first {
                battle.fighters[target].inflict(Status::for_turns(StatusKind::Away, PLANK_TURNS));
                let target_name = battle.fighters[target].name.clone();
                battle.note(
                    format!("{target_name} walks the plank and leaves the fight for a while."),
                    Tone::Status,
                );
            }
        }
        Special::Abduct => {
            if let Some(target) = first {
                let sky = battle.fighters[target].has(Trait::SeenTheSky)
                    || battle.fighters[target].has(Trait::Glide)
                    || super::passives::beam_fails(battle, target);
                let target_name = battle.fighters[target].name.clone();
                if sky {
                    battle.note(
                        format!("The beam slides off {target_name}. It has seen the sky."),
                        Tone::Quiet,
                    );
                    return;
                }
                battle.fighters[target].inflict(Status::for_turns(StatusKind::Away, FOREVER));
                battle.note(
                    format!("{target_name} is lifted into the light."),
                    Tone::Down,
                );
            }
        }
        Special::Swallow => {
            if let Some(target) = first {
                battle.fighters[target].inflict(Status::for_turns(StatusKind::Away, SWALLOW_TURNS));
                battle.fighters[target].inflict(Status::new(
                    StatusKind::Burn,
                    SWALLOW_TURNS,
                    SWALLOW_DAMAGE,
                ));
                let target_name = battle.fighters[target].name.clone();
                battle.note(format!("{name} swallows {target_name}."), Tone::Down);
            }
        }
        Special::Reap => {
            let down = battle
                .side(Side::Party)
                .find(|&i| battle.fighters[i].belly_up && !battle.fighters[i].gone);
            if let Some(target) = down {
                let target_name = battle.fighters[target].name.clone();
                battle.note(format!("{name} carries off {target_name}."), Tone::Down);
                if battle.setup.difficulty.kills() {
                    battle.lose_hp(target, 1, Some(actor));
                } else {
                    battle.fighters[target].inflict(Status::for_turns(StatusKind::Away, FOREVER));
                }
            }
        }
        Special::Raise => battle.summon(actor, GHOST, 1),
        Special::Command => {
            for ally in battle.standing(battle.fighters[actor].side) {
                if ally != actor {
                    battle.fighters[ally].ready_at = battle.clock;
                }
            }
            battle.note(
                format!("{name} commands: every other enemy acts again."),
                Tone::Tell,
            );
        }
        Special::Grace => {
            let beaten = battle
                .side(battle.fighters[actor].side)
                .find(|&i| battle.fighters[i].gone && !battle.fighters[i].is_boss());
            if let Some(index) = beaten {
                let f = &mut battle.fighters[index];
                f.gone = false;
                f.hp = f.max_hp / HALF;
                let target_name = f.name.clone();
                f.ready_at = battle.clock;
                battle.note(
                    format!("{target_name} is brought back at half."),
                    Tone::Heal,
                );
            }
        }
        Special::Banish => {
            if let Some(target) = first {
                let unholy = battle.fighters[target].has(Trait::DevilMarked)
                    || battle.fighters[target]
                        .member()
                        .is_some_and(|m| battle.party[m].devil_marked);
                if unholy {
                    battle.fighters[target].inflict(Status::for_turns(StatusKind::Away, FOREVER));
                    let target_name = battle.fighters[target].name.clone();
                    battle.note(
                        format!("{target_name} is banished from the fight."),
                        Tone::Down,
                    );
                } else {
                    battle.note("Nothing unholy stands there.", Tone::Quiet);
                }
            }
        }
        Special::Hymn => match battle.rng.random_range(0..HYMN_VERSES) {
            0 => {
                for ally in battle.standing(battle.fighters[actor].side) {
                    battle.heal(ally, HYMN_HEAL);
                }
            }
            1 => {
                for target in battle.standing(battle.fighters[actor].side.other()) {
                    battle.inflict(actor, target, Status::for_turns(StatusKind::Blind, 1));
                }
            }
            _ => {
                let damage = battle.fighters[actor].scale.bite(HYMN_DAMAGE);
                for target in battle.within_reach(battle.fighters[actor].side.other()) {
                    battle.harm(actor, target, damage, DamageKind::Holy);
                }
                battle.note(
                    format!("The hymn hits every fish for {damage}."),
                    Tone::Hurt,
                );
            }
        },
        Special::Crescendo => {
            let voices = battle.standing(battle.fighters[actor].side).len() as u32;
            let damage = battle.fighters[actor]
                .scale
                .bite(CRESCENDO_PER_VOICE * voices);
            for target in battle.within_reach(battle.fighters[actor].side.other()) {
                battle.harm(actor, target, damage, DamageKind::Holy);
            }
            battle.note(
                format!("The crescendo hits every fish for {damage}."),
                Tone::Hurt,
            );
        }
        Special::Fanfare => {
            for ally in battle.standing(battle.fighters[actor].side) {
                battle.fighters[ally].inflict(Status::new(
                    StatusKind::BiteUp,
                    FANFARE_TURNS,
                    FANFARE_BITE,
                ));
            }
            battle.note("Every enemy bites harder.", Tone::Tell);
        }
        Special::Contract => {
            if let Some(target) = first {
                battle.inflict(
                    actor,
                    target,
                    Status::for_turns(StatusKind::Contract, FOREVER),
                );
            }
        }
        Special::Encrypt => {
            if let Some(item) = battle.take_a_stack(actor) {
                battle.note(format!("{name} locks your {item} away."), Tone::Status);
            }
        }
        Special::Borrow => {
            if let Some(target) = first.or_else(|| battle.pick_target(actor)) {
                let borrowed = battle.fighters[target].effective_bite().max(1);
                let strike = battle.strike(actor, target, Power::Flat(borrowed), DamageKind::Bite);
                let target_name = battle.fighters[target].name.clone();
                if strike.landed {
                    battle.note(
                        format!(
                            "{name} bites {target_name} with its own teeth for {}.",
                            strike.damage
                        ),
                        Tone::Hurt,
                    );
                } else {
                    battle.note(
                        format!("{name} snaps at {target_name} and misses."),
                        Tone::Quiet,
                    );
                }
            }
        }
        Special::Assimilate => {
            let down = battle
                .side(battle.fighters[actor].side.other())
                .find(|&i| battle.fighters[i].belly_up && !battle.fighters[i].gone);
            if let Some(target) = down {
                let target_name = battle.fighters[target].name.clone();
                let gained = battle.fighters[target].max_hp;
                battle.note(
                    format!("{name} takes {target_name} in as one of its heads."),
                    Tone::Down,
                );
                if battle.setup.difficulty.kills() {
                    battle.lose_hp(target, 1, Some(actor));
                } else {
                    battle.fighters[target].inflict(Status::for_turns(StatusKind::Away, FOREVER));
                }
                let f = &mut battle.fighters[actor];
                f.max_hp += gained;
                f.hp += gained;
            }
        }
        Special::Optimise => {
            let lost = battle.fighters[actor]
                .last_hp
                .saturating_sub(battle.fighters[actor].hp);
            battle.fighters[actor].heal(lost);
            battle.note(format!("{name} optimises: +{lost}."), Tone::Heal);
        }
        Special::Fork => {
            if battle.fighters.len() < MAX_FIGHTERS {
                let hp = battle.fighters[actor].hp;
                battle.fighters[actor].hp = hp.div_ceil(HALF);
                let mut fork = battle.fighters[actor].clone();
                fork.hp = hp / HALF;
                fork.max_hp = battle.fighters[actor].max_hp / HALF;
                fork.ready_at = battle.clock + battle.wait_of(actor);
                battle.fighters.push(fork);
                battle.note(format!("{name} forks into two processes."), Tone::Status);
            }
        }
        Special::Moo => {
            battle.fighters[actor].inflict(Status::for_turns(StatusKind::Untouchable, 0));
            for other in battle.standing(battle.fighters[actor].side) {
                if other != actor {
                    battle.fighters[other].inflict(Status::for_turns(StatusKind::Untouchable, 1));
                }
            }
            battle.note(
                format!("{name} moos. Every bite comes to it."),
                Tone::Status,
            );
        }
        Special::Milk => {
            if let Some(target) = first {
                battle.heal(target, MILK_HEAL);
            }
        }
    }
    let _ = skill;
}

fn stacks_of(battle: &Battle, actor: usize, species: FishSpecies) -> u32 {
    battle.fighters[actor]
        .member()
        .map_or(1, |m| battle.party[m].ability_stacks(species))
}

fn miracle(battle: &mut Battle, actor: usize) {
    let side = battle.fighters[actor].side;
    let down: Vec<usize> = battle
        .side(side)
        .filter(|&i| battle.fighters[i].belly_up && !battle.fighters[i].gone)
        .collect();
    let verse = battle.rng.random_range(0..MIRACLE_VERSES);
    match (verse, down.first()) {
        (0, Some(&fallen)) => {
            battle.note("A Blessing: Revive.", Tone::Gold);
            battle.revive(fallen, PERCENT);
        }
        (0 | 1, _) => {
            battle.note("A Blessing: Feed Burst.", Tone::Gold);
            for ally in battle.standing(side) {
                battle.heal(ally, MIRACLE_FEED);
            }
        }
        (2, _) => {
            battle.note("A Blessing: Turn Holy.", Tone::Gold);
            if let Some(target) = battle.pick_target(actor)
                && !battle.fighters[target].is_boss()
            {
                battle.inflict(
                    actor,
                    target,
                    Status::for_turns(StatusKind::Charmed, FOREVER),
                );
            }
        }
        (3, _) => {
            battle.note("A Blessing: Grace.", Tone::Gold);
            for ally in battle.standing(side) {
                battle.fighters[ally].shield += MIRACLE_FEED;
            }
        }
        _ => {
            battle.note("A Blessing: Smite.", Tone::Gold);
            for target in battle.standing(side.other()) {
                battle.harm(actor, target, MIRACLE_SMITE, DamageKind::Holy);
            }
        }
    }
}

fn divide(battle: &mut Battle, actor: usize) {
    let Some(member) = battle.fighters[actor].member() else {
        return;
    };
    if battle.fighters.len() >= MAX_FIGHTERS {
        return;
    }
    let mut child = battle.party[member].clone();
    child.name = format!("{} II", child.name);
    child.weight_g = (child.weight_g / HALF).max(1);
    child.deep = None;
    battle.party[member].weight_g = (battle.party[member].weight_g / HALF).max(1);
    battle.refit(actor);
    let index = battle.party.len();
    let kit = Kit::of(&child);
    let name = child.name.clone();
    battle.party.push(child);
    let mut fighter = Fighter::of_kit(&kit, index);
    fighter.hp = fighter.max_hp / HALF;
    fighter.ready_at = battle.clock + battle.wait_of(actor);
    battle.fighters.push(fighter);
    battle.note(
        format!("The double splits. {name} joins the party."),
        Tone::Gold,
    );
}

fn devour(battle: &mut Battle, actor: usize, target: Option<usize>) {
    let Some(target) = target else {
        return;
    };
    let t = &battle.fighters[target];
    if t.is_boss() || t.hp_percent() >= moves::DEVOUR_BELOW_PERCENT {
        battle.note("It is too big to swallow yet.", Tone::Quiet);
        return;
    }
    let mass = t.max_hp * DEVOUR_MASS_PER_HP_G;
    let target_name = t.name.clone();
    let name = battle.fighters[actor].name.clone();
    battle.note(format!("{name} devours {target_name}."), Tone::Gold);
    battle.beat(target);
    if let Some(member) = battle.fighters[actor].member() {
        battle.party[member].weight_g = battle.party[member].weight_g.saturating_add(mass);
        battle.refit(actor);
    }
    battle.fighters[actor].inflict(Status::new(
        StatusKind::Slowed,
        FOREVER,
        super::tables::mutations::ENGULFED_SPEED,
    ));
}

fn call_home(battle: &mut Battle, actor: usize) {
    if battle.fighters.len() >= MAX_FIGHTERS {
        return;
    }
    let wild = FishSpecies::all_wild();
    let species = wild[battle.rng.random_range(0..wild.len())];
    let mut companion = Fish::new(species, String::new(), 0.0, 0.0, &mut battle.rng);
    companion.name = format!("{} from Above", species.display_name());
    crate::fishes::mutations::apply_mutation(&mut companion, Mutation::Alienation, &mut battle.rng);
    let index = battle.party.len();
    let kit = Kit::of(&companion);
    let name = companion.name.clone();
    battle.party.push(companion);
    let mut fighter = Fighter::of_kit(&kit, index);
    fighter.ready_at = battle.clock + battle.wait_of(actor);
    battle.fighters.push(fighter);
    battle.note(
        format!("A UFO drops {name}. It joins the party."),
        Tone::Gold,
    );
}

pub const GHOST: &str = "Ghost of your dead";
pub const HYMN_VERSES: u32 = 3;
pub const MIRACLE_VERSES: u32 = 5;
pub const CRESCENDO_PER_VOICE: u32 = 4;
pub const MILK_HEAL: u32 = 15;
