use rand::RngExt;

use crate::void_ritual::{FINAL_PRAYER_PHRASE, PRAYERS, phrase_matches};

use super::battle::{Battle, HALF, MAX_FIGHTERS};
use super::beat::Tone;
use super::bestiary::{self, EnemySpec, Rank};
use super::effect::{DamageKind, Status, StatusKind};
use super::fighter::{Fighter, Side};
use super::floor::LAST_FLOOR;
use super::traits::{BodyParts, Haunt, Trait};

pub const TENTACLE: &str = "Kraken Tentacle";
pub const TENTACLES: u32 = 4;
pub const HEAD_REACHED_AT: u32 = 2;
pub const HEAD: &str = "Abomination Head";
pub const HEADS: u32 = 5;
pub const HEAD_REGROW_TURNS: u32 = 2;
pub const VOICE: &str = "Choir Voice";
pub const VOICES: u32 = 12;
pub const VOICES_TO_SING: u32 = 6;
pub const RIBS: u32 = 6;
pub const PHYLACTERY: &str = "The Phylactery";
pub const COURTIER: &str = "Reef Shark";
pub const COURTIERS: u8 = 2;
pub const HARD_CANDY: u32 = 6;
pub const CHOCOLATE_HEAL: u32 = 20;
pub const LAYERS: u32 = 3;
pub const PINATA_CANDY_DIVISOR: u64 = 4;
pub const PINATA_SPRINKLES_ONE_IN: u32 = 4;
pub const SPRINKLES: &str = "Sprinkle Swarm";
pub const GREED_PERCENT: u64 = 2;
pub const GREED_BITE: u32 = 1;
pub const HALF_STRENGTH: u32 = 50;
pub const DREAM_PERCENT: u32 = 50;
pub const HATCH_TURNS: u32 = 3;
pub const HATCHLING: &str = "Unborn";
pub const HATCHLINGS: u8 = 2;
pub const BUFF_BITE: u32 = 3;
pub const BUFF_SHIELD: u32 = 8;
pub const BUFF_SPEED: u32 = 2;
pub const BUFF_TURNS: u8 = 2;
pub const BUFFS: u32 = 3;
pub const DREAM_TURNS: u8 = 2;
pub const DEVIL_THIRDS: u8 = 3;
const PERCENT: u32 = 100;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prayer {
    pub lines: Vec<&'static str>,
    pub said: usize,
    pub open: bool,
}

impl Prayer {
    pub fn of_seed(seed: u64) -> Prayer {
        let verses = PRAYERS[(seed as usize) % PRAYERS.len()];
        let lines = verses
            .iter()
            .copied()
            .chain(std::iter::once(FINAL_PRAYER_PHRASE))
            .collect();
        Prayer {
            lines,
            said: 0,
            open: false,
        }
    }

    pub fn shields(&self) -> bool {
        !self.open && self.said < self.lines.len()
    }

    pub fn line(&self) -> Option<&'static str> {
        self.lines.get(self.said).copied()
    }
}

fn spec(name: &str) -> Option<&'static EnemySpec> {
    bestiary::find(name)
}

pub fn parts_of(battle: &Battle, body: usize) -> Vec<usize> {
    (0..battle.fighters.len())
        .filter(|&i| battle.fighters[i].body == Some(body) && !battle.fighters[i].gone)
        .collect()
}

fn grow(battle: &mut Battle, body: usize, name: &str, count: u32) {
    let Some(part) = spec(name) else {
        return;
    };
    for _ in 0..count {
        if battle.fighters.len() >= MAX_FIGHTERS {
            return;
        }
        let mut fighter = Fighter::of_enemy(part, battle.setup.floor, battle.setup.scale);
        fighter.side = battle.fighters[body].side;
        fighter.body = Some(body);
        fighter.shares = 0;
        fighter.ready_at = battle.fighters[body].ready_at + battle.wait_of(body) / 2;
        battle.fighters.push(fighter);
    }
}

pub fn assemble(battle: &mut Battle) {
    for body in 0..battle.fighters.len() {
        for a_trait in battle.fighters[body].traits.clone() {
            match a_trait {
                Trait::Parts(BodyParts::Tentacles) => grow(battle, body, TENTACLE, TENTACLES),
                Trait::Parts(BodyParts::Heads) => grow(battle, body, HEAD, HEADS),
                Trait::Parts(BodyParts::Voices) => grow(battle, body, VOICE, VOICES),
                Trait::Parts(BodyParts::Ribs) => battle.fighters[body].parts = RIBS,
                Trait::Phylactery => grow(battle, body, PHYLACTERY, 1),
                Trait::Court => battle.summon(body, COURTIER, COURTIERS),
                Trait::Layers => battle.fighters[body].armour += HARD_CANDY,
                Trait::PrayerShield => {
                    let name = battle.fighters[body].name.clone();
                    battle.note(
                        format!("{name} cannot be touched. Type its line in the command bar."),
                        Tone::Tell,
                    );
                }
                _ => {}
            }
        }
    }
}

pub fn shielded(battle: &Battle, target: usize) -> bool {
    let t = &battle.fighters[target];
    t.traits.iter().any(|a_trait| match a_trait {
        Trait::Parts(BodyParts::Tentacles) => {
            parts_of(battle, target).len() as u32 > HEAD_REACHED_AT
        }
        Trait::Parts(BodyParts::Voices) => true,
        Trait::PrayerShield => battle.prayer.shields(),
        _ => false,
    })
}

pub fn pray(battle: &mut Battle, words: &str) -> bool {
    let Some(line) = battle.prayer.line() else {
        return false;
    };
    if !phrase_matches(words, line) {
        battle.note("The words fall into the dark.", Tone::Quiet);
        return false;
    }
    battle.prayer.said += 1;
    battle.prayer.open = true;
    let text = match battle.prayer.line() {
        Some(_) => "The Eye blinks. For a moment, it can be hurt.",
        None => "The Eye opens all the way. Nothing shields it now.",
    };
    battle.note(text, Tone::Tell);
    true
}

pub fn turn_ended(battle: &mut Battle, actor: usize) {
    if battle.fighters[actor].has(Trait::PrayerShield) && battle.prayer.line().is_some() {
        battle.prayer.open = false;
    }
}

pub fn turn_start(battle: &mut Battle, actor: usize) {
    let name = battle.fighters[actor].name.clone();
    for a_trait in battle.fighters[actor].traits.clone() {
        if !battle.fighters[actor].standing() {
            return;
        }
        match a_trait {
            Trait::Glitch => glitch(battle, actor),
            Trait::RandomBuffs => random_buff(battle, actor),
            Trait::Merge => merge(battle, actor),
            Trait::Hatch if battle.fighters[actor].turns >= HATCH_TURNS => {
                battle.note(format!("{name} hatches."), Tone::Status);
                vanish(battle, actor);
                battle.summon(actor, HATCHLING, HATCHLINGS);
            }
            Trait::Dreamer => dream(battle, actor),
            Trait::Spreads => spread_dreams(battle, actor),
            Trait::Greed => greed(battle, actor),
            Trait::LastTrump(count) => last_trump(battle, actor, count),
            Trait::Parts(BodyParts::Heads) => regrow_head(battle, actor),
            Trait::Layers if battle.fighters[actor].phase == 1 => {
                let healed = battle.fighters[actor].heal(CHOCOLATE_HEAL);
                battle.note(
                    format!("The chocolate melts back over {name}: +{healed}."),
                    Tone::Heal,
                );
            }
            _ => {}
        }
    }
}

fn glitch(battle: &mut Battle, actor: usize) {
    let f = &battle.fighters[actor];
    let mut numbers = [f.hp, f.bite, f.speed];
    for index in (1..numbers.len()).rev() {
        let swap = battle.rng.random_range(0..=index);
        numbers.swap(index, swap);
    }
    let f = &mut battle.fighters[actor];
    f.hp = numbers[0].clamp(1, f.max_hp);
    f.bite = numbers[1];
    f.speed = numbers[2].max(1);
    let name = f.name.clone();
    battle.note(
        format!("{name} corrupts: its numbers shuffle."),
        Tone::Status,
    );
}

fn random_buff(battle: &mut Battle, actor: usize) {
    let name = battle.fighters[actor].name.clone();
    match battle.rng.random_range(0..BUFFS) {
        0 => {
            battle.fighters[actor].inflict(Status::new(StatusKind::BiteUp, BUFF_TURNS, BUFF_BITE));
            battle.note(format!("{name} grows another jaw."), Tone::Status);
        }
        1 => {
            battle.fighters[actor].shield += BUFF_SHIELD;
            battle.note(format!("{name} grows a crust."), Tone::Status);
        }
        _ => {
            battle.fighters[actor].inflict(Status::new(StatusKind::Hasted, BUFF_TURNS, BUFF_SPEED));
            battle.note(format!("{name} grows another fin."), Tone::Status);
        }
    }
}

fn merge(battle: &mut Battle, actor: usize) {
    let side = battle.fighters[actor].side;
    let other = battle
        .standing(side)
        .into_iter()
        .find(|&i| i != actor && battle.fighters[i].has(Trait::Merge));
    let Some(other) = other else {
        return;
    };
    let (hp, max_hp, shares) = {
        let o = &battle.fighters[other];
        (o.hp, o.max_hp, o.shares)
    };
    vanish(battle, other);
    let f = &mut battle.fighters[actor];
    f.max_hp += max_hp;
    f.hp += hp;
    f.shares += shares;
    let name = f.name.clone();
    battle.note(format!("Two {name} touch and become one."), Tone::Status);
}

fn earlier_commons(floor: u32) -> Vec<&'static EnemySpec> {
    bestiary::all()
        .filter(|spec| {
            spec.floor >= 1
                && spec.floor < floor.min(LAST_FLOOR)
                && spec.rank == Rank::Common
                && !spec.traits.contains(&Trait::Appendage)
                && !spec.traits.iter().any(|t| matches!(t, Trait::Yours(_)))
        })
        .collect()
}

fn dream(battle: &mut Battle, actor: usize) {
    let pool = earlier_commons(battle.setup.floor);
    if pool.is_empty() || battle.fighters.len() >= MAX_FIGHTERS {
        return;
    }
    let dreamt = pool[battle.rng.random_range(0..pool.len())];
    let mut fighter = Fighter::of_enemy(dreamt, battle.setup.floor, battle.setup.scale);
    fighter.side = battle.fighters[actor].side;
    fighter.max_hp = (fighter.max_hp * DREAM_PERCENT / PERCENT).max(1);
    fighter.hp = fighter.max_hp;
    fighter.summoned_by = Some(actor);
    fighter.shares = 0;
    fighter.ready_at = battle.clock + battle.wait_of(actor) / 2;
    let name = battle.fighters[actor].name.clone();
    battle.note(
        format!("{name} dreams a {} into the room.", dreamt.name),
        Tone::Status,
    );
    battle.fighters.push(fighter);
}

pub fn wake(battle: &mut Battle, dreamer: usize) {
    if !battle.fighters[dreamer].has(Trait::Dreamer) {
        return;
    }
    let dreams: Vec<usize> = (0..battle.fighters.len())
        .filter(|&i| battle.fighters[i].summoned_by == Some(dreamer) && !battle.fighters[i].gone)
        .collect();
    if dreams.is_empty() {
        return;
    }
    for dream in dreams {
        vanish(battle, dream);
    }
    let name = battle.fighters[dreamer].name.clone();
    battle.note(
        format!("{name} wakes, and its dreams are gone."),
        Tone::Status,
    );
}

fn spread_dreams(battle: &mut Battle, actor: usize) {
    let side = battle.fighters[actor].side.other();
    let order = battle.turn_order(battle.fighters.len() * 2);
    let dreaming: Vec<usize> = battle
        .standing(side)
        .into_iter()
        .filter(|&i| battle.fighters[i].has_status(StatusKind::Dreaming))
        .collect();
    for sleeper in dreaming {
        let start = order.iter().position(|&i| i == sleeper).unwrap_or(0);
        let next = order
            .iter()
            .cycle()
            .skip(start + 1)
            .take(order.len())
            .copied()
            .find(|&i| {
                i != sleeper
                    && battle.fighters[i].side == side
                    && battle.fighters[i].standing()
                    && !battle.fighters[i].has_status(StatusKind::Dreaming)
            });
        if let Some(next) = next {
            battle.fighters[next].inflict(Status::for_turns(StatusKind::Dreaming, DREAM_TURNS));
            let name = battle.fighters[next].name.clone();
            battle.note(format!("The dream jumps to {name}."), Tone::Status);
        }
    }
}

fn greed(battle: &mut Battle, actor: usize) {
    let eaten = battle.haul * GREED_PERCENT / u64::from(PERCENT);
    if eaten == 0 {
        return;
    }
    battle.haul -= eaten;
    let f = &mut battle.fighters[actor];
    f.eaten_gold += eaten;
    f.bite += GREED_BITE;
    let name = f.name.clone();
    battle.note(
        format!("{name} eats ${eaten} of your haul and grows."),
        Tone::Gold,
    );
}

fn last_trump(battle: &mut Battle, actor: usize, count: u8) {
    let name = battle.fighters[actor].name.clone();
    battle.fighters[actor].phase += 1;
    let left = count.saturating_sub(battle.fighters[actor].phase);
    if left > 0 {
        battle.note(format!("{name} counts: {left}."), Tone::Tell);
        return;
    }
    battle.fighters[actor].phase = 0;
    battle.note(format!("{name} sounds the Last Trump."), Tone::Tell);
    let marked: Vec<usize> = battle
        .standing(battle.fighters[actor].side.other())
        .into_iter()
        .filter(|&i| battle.is_marked(i))
        .collect();
    for fish in marked {
        let f = &mut battle.fighters[fish];
        f.hp = f.hp.min(1);
        let fish_name = f.name.clone();
        battle.note(format!("{fish_name} is left at 1 HP."), Tone::Hurt);
    }
}

fn regrow_head(battle: &mut Battle, actor: usize) {
    let turns = battle.fighters[actor].turns;
    if !turns.is_multiple_of(HEAD_REGROW_TURNS) || parts_of(battle, actor).len() as u32 >= HEADS {
        return;
    }
    grow(battle, actor, HEAD, 1);
    let name = battle.fighters[actor].name.clone();
    battle.note(format!("{name} grows a new head."), Tone::Status);
}

pub fn vanish(battle: &mut Battle, index: usize) {
    let f = &mut battle.fighters[index];
    f.gone = true;
    f.hp = 0;
    f.shares = 0;
}

pub fn absorbs_the_hit(battle: &mut Battle, target: usize) -> bool {
    let t = &battle.fighters[target];
    if t.parts == 0 || !t.has(Trait::Parts(BodyParts::Ribs)) {
        return false;
    }
    battle.fighters[target].parts -= 1;
    let left = battle.fighters[target].parts;
    let text = match left {
        0 => "The last rib breaks. The spine is open.".to_string(),
        n => format!("A rib breaks. {n} left."),
    };
    battle.note(text, Tone::Hit);
    true
}

pub fn heals_from(battle: &Battle, target: usize, kind: DamageKind) -> bool {
    kind == DamageKind::Holy && battle.fighters[target].has(Trait::Unholy)
}

pub fn learned_percent(battle: &mut Battle, target: usize) -> u32 {
    if !battle.fighters[target].has(Trait::Learns) {
        return PERCENT;
    }
    let Some(acting) = battle.acting else {
        return PERCENT;
    };
    let known = battle.fighters[target].learned == Some(acting);
    battle.fighters[target].learned = Some(acting);
    match known {
        true => PERCENT / HALF,
        false => PERCENT,
    }
}

pub fn hit_landed(battle: &mut Battle, target: usize, damage: u32) {
    battle.fighters[target].damage_taken += damage;
    wake(battle, target);
    if battle.fighters[target].has(Trait::Pinata) && battle.fighters[target].in_the_fight() {
        let candy = super::floor::gold_share(battle.setup.floor, battle.setup.difficulty)
            / PINATA_CANDY_DIVISOR;
        battle.haul += candy;
        battle.note(format!("Candy spills into the haul: ${candy}."), Tone::Gold);
        if battle.rng.random_range(0..PINATA_SPRINKLES_ONE_IN) == 0 {
            battle.summon(target, SPRINKLES, 1);
        }
    }
}

pub fn wounded(battle: &mut Battle, target: usize) {
    layers(battle, target);
    if battle.fighters[target].has(Trait::DevilsBargain) {
        let f = &battle.fighters[target];
        let third = (f.max_hp * u32::from(DEVIL_THIRDS - 1 - f.phase.min(DEVIL_THIRDS - 1)))
            / u32::from(DEVIL_THIRDS);
        if f.phase < DEVIL_THIRDS - 1 && f.hp <= third {
            battle.fighters[target].phase += 1;
            battle.offer = Some(target);
        }
    }
}

fn layers(battle: &mut Battle, target: usize) {
    if !battle.fighters[target].has(Trait::Layers) {
        return;
    }
    let f = &battle.fighters[target];
    let name = f.name.clone();
    let layer_now = LAYERS - 1 - (f.hp * LAYERS / f.max_hp.max(1)).min(LAYERS - 1);
    if u32::from(f.phase) >= layer_now {
        return;
    }
    if f.phase == 0 {
        battle.fighters[target].armour = battle.fighters[target].armour.saturating_sub(HARD_CANDY);
        battle.note(
            format!("{name}'s hard candy cracks. Chocolate underneath."),
            Tone::Status,
        );
    }
    if layer_now >= LAYERS - 1 {
        battle.fighters[target].phase = 2;
        battle.note(
            format!("The chocolate is gone. {name}'s gummy core splits."),
            Tone::Status,
        );
        battle.split(target);
        return;
    }
    battle.fighters[target].phase = layer_now as u8;
}

pub fn answer(battle: &mut Battle, sign: bool) {
    let Some(devil) = battle.offer.take() else {
        return;
    };
    let name = battle.fighters[devil].name.clone();
    if !sign {
        battle.note(
            format!("You do not sign. {name} laughs anyway."),
            Tone::Quiet,
        );
        return;
    }
    let f = &mut battle.fighters[devil];
    f.hp = f.hp.saturating_mul(HALF);
    f.max_hp = f.max_hp.max(f.hp);
    battle.haul = battle.haul.saturating_mul(u64::from(HALF));
    battle.note(
        format!("You sign. {name} doubles, and so does your haul."),
        Tone::Gold,
    );
}

pub enum Fate {
    Falls,
    Stands,
}

pub fn before_falling(battle: &mut Battle, target: usize) -> Fate {
    let name = battle.fighters[target].name.clone();
    let phylactery = battle.fighters[target].has(Trait::Phylactery)
        && parts_of(battle, target)
            .iter()
            .any(|&part| battle.fighters[part].base == PHYLACTERY);
    if phylactery {
        let f = &mut battle.fighters[target];
        f.hp = f.max_hp / HALF;
        battle.note(
            format!("{name} rises again. Its soul is still in the jar."),
            Tone::Status,
        );
        return Fate::Stands;
    }
    if battle.fighters[target].has(Trait::Background) {
        let side = battle.fighters[target].side;
        let twin = battle
            .standing(side)
            .into_iter()
            .any(|i| i != target && battle.fighters[i].has(Trait::Background));
        if twin {
            let f = &mut battle.fighters[target];
            f.belly_up = true;
            f.hp = 0;
            battle.note(
                format!("{name} drops, but its other process keeps it alive."),
                Tone::Status,
            );
            return Fate::Stands;
        }
    }
    if battle.fighters[target].has(Trait::Cycle) && !battle.fighters[target].came_back {
        cycle(battle, target);
        return Fate::Stands;
    }
    Fate::Falls
}

fn cycle(battle: &mut Battle, eye: usize) {
    let name = battle.fighters[eye].name.clone();
    battle.note(
        format!("{name} closes, and everything unlifed is born again."),
        Tone::Tell,
    );
    let unlifed: Vec<usize> = (0..battle.fighters.len())
        .filter(|&i| {
            let f = &battle.fighters[i];
            i != eye
                && f.gone
                && f.side == battle.fighters[eye].side
                && f.spec().is_some_and(|spec| spec.floor == LAST_FLOOR)
        })
        .collect();
    let clock = battle.clock;
    for index in unlifed {
        let f = &mut battle.fighters[index];
        f.gone = false;
        f.belly_up = false;
        f.hp = (f.max_hp / HALF).max(1);
        f.ready_at = clock;
    }
    let f = &mut battle.fighters[eye];
    f.came_back = true;
    f.hp = f.max_hp / HALF;
    battle.prayer.open = false;
}

pub fn after_falling(battle: &mut Battle, target: usize, by: Option<usize>) {
    let name = battle.fighters[target].name.clone();
    for part in parts_of(battle, target) {
        vanish(battle, part);
    }
    if battle.fighters[target].has(Trait::Background) {
        let downed: Vec<usize> = (0..battle.fighters.len())
            .filter(|&i| {
                let f = &battle.fighters[i];
                f.belly_up && !f.gone && f.has(Trait::Background) && f.side == Side::Foes
            })
            .collect();
        for twin in downed {
            battle.beat_down(twin);
        }
    }
    if let Some(body) = battle.fighters[target].body {
        let voices = battle.fighters[body].has(Trait::Parts(BodyParts::Voices));
        if voices
            && !battle.fighters[body].gone
            && (parts_of(battle, body).len() as u32) < VOICES_TO_SING
        {
            let choir = battle.fighters[body].name.clone();
            battle.note(
                format!("Too few voices are left. {choir} falls silent."),
                Tone::Gold,
            );
            battle.beat(body);
        }
    }
    if battle.fighters[target].has(Trait::Hollow) {
        battle.note(format!("Something climbs out of {name}."), Tone::Status);
        battle.raise(target, Haunt::Dead);
    }
    if battle.fighters[target].has(Trait::Rebirth) && !battle.fighters[target].came_back {
        reborn(battle, target);
    }
    if battle.fighters[target].has(Trait::MutatesKiller)
        && let Some(killer) = by.filter(|&k| battle.fighters[k].side == Side::Party)
    {
        battle.note(format!("{name} bursts over its killer."), Tone::Status);
        battle.mutate(killer, 1);
    }
}

fn reborn(battle: &mut Battle, target: usize) {
    let pool: Vec<&'static EnemySpec> = bestiary::of_floor(LAST_FLOOR)
        .filter(|spec| {
            spec.rank == Rank::Common
                && !spec.traits.contains(&Trait::Rebirth)
                && !spec.traits.iter().any(|t| matches!(t, Trait::Yours(_)))
        })
        .collect();
    if pool.is_empty() || battle.fighters.len() >= MAX_FIGHTERS {
        return;
    }
    let born = pool[battle.rng.random_range(0..pool.len())];
    let mut fighter = Fighter::of_enemy(born, battle.setup.floor, battle.setup.scale);
    fighter.side = battle.fighters[target].side;
    fighter.hp = (fighter.max_hp / HALF).max(1);
    fighter.came_back = true;
    fighter.ready_at = battle.clock + battle.wait_of(target);
    let name = battle.fighters[target].name.clone();
    battle.note(
        format!("{name} is born again as a {}.", born.name),
        Tone::Status,
    );
    battle.fighters.push(fighter);
}

pub fn half_remembered(battle: &mut Battle, actor: usize) -> bool {
    let skills: Vec<_> = bestiary::all()
        .filter(|spec| spec.rank == Rank::Boss && spec.floor < battle.setup.floor.min(LAST_FLOOR))
        .flat_map(|spec| spec.skills.iter())
        .collect();
    if skills.is_empty() {
        return false;
    }
    let remembered = skills[battle.rng.random_range(0..skills.len())].skill();
    let free = super::skill::Skill {
        price: super::skill::Price::Free,
        ..remembered
    };
    let name = battle.fighters[actor].name.clone();
    battle.note(
        format!("{name} half-remembers {}.", free.name),
        Tone::Status,
    );
    battle.strength = HALF_STRENGTH;
    let target = match free.reach.picks_a_foe() {
        true => battle.pick_target(actor),
        false => None,
    };
    battle.use_skill(actor, &free, target);
    battle.strength = PERCENT;
    true
}

#[cfg(test)]
mod tests;
