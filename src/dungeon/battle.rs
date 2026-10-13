use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::fishes::fish::Fish;
use crate::fishes::toy::Line;

use super::Difficulty;
use super::beat::{Beat, LogLine, Pause, Tone, View};
use super::effect::{DamageKind, Effect, Power, Reach, Status, StatusKind};
use super::fighter::{Fighter, Ghost, Scale, Side};
use super::items::Pack;
use super::kit::Kit;
use super::passives;
use super::reveal;
use super::rules::{self, Fate, Prayer};
use super::skill::{Price, Skill};
use super::traits::{Haunt, Trait};

pub const TURN_LENGTH: u64 = 720_720;
pub const ROUND_SPEED: u64 = 6;
pub const ROUND_TICKS: u64 = TURN_LENGTH / ROUND_SPEED;
pub const TURN_BAR_LENGTH: usize = 8;
pub const ROLL_THRESHOLD_PERCENT: u32 = 25;
pub const SPREAD_LOW_PERCENT: u32 = 85;
pub const SPREAD_HIGH_PERCENT: u32 = 115;
pub const CRIT_TIMES: u32 = 2;
pub const HARSH_CRIT_TIMES: u32 = 3;
pub const BLIND_MISS_PERCENT: u32 = 50;
pub const FLEE_BASE_PERCENT: i32 = 50;
pub const FLEE_PER_SPEED: i32 = 5;
pub const FLEE_MIN_PERCENT: i32 = 20;
pub const FLEE_MAX_PERCENT: i32 = 95;
pub const FRENZY_PERCENT: u32 = 150;
pub const CONTRACT_PERCENT: u32 = 130;
pub const ENRAGE_TIMES: u32 = 2;
pub const HALF: u32 = 2;
pub const ENEMY_BIG_MOVE_CHANCE: u32 = 60;
pub const HARDCORE_BELLY_UP_WEIGHT: u32 = 30;
pub const BASE_TARGET_WEIGHT: u32 = 100;
pub const MAX_FIGHTERS: usize = 40;
const PERCENT: u32 = 100;
const MAX_STEPS: usize = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Outcome {
    Won,
    Wiped,
    Fled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    Running,
    Choosing(usize),
    Offer(usize),
    Over(Outcome),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stakes {
    Encounter,
    Den,
    Boss,
}

impl Stakes {
    pub fn can_flee(self) -> bool {
        self == Stakes::Encounter
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Choice {
    Bite(usize),
    Skill { skill: usize, target: Option<usize> },
    Guard,
    Item { pack: usize, target: usize },
    Flee,
}

#[derive(Clone, Debug)]
pub struct BattleSetup {
    pub floor: u32,
    pub difficulty: Difficulty,
    pub stakes: Stakes,
    pub purse: u128,
    pub items: Vec<Pack>,
    pub seed: u64,
    pub ambushed: bool,
    pub haunts: Vec<Ghost>,
    pub scale: Scale,
    pub known: Vec<String>,
    pub every_graeae: bool,
}

#[derive(Clone)]
pub struct Battle {
    pub fighters: Vec<Fighter>,
    pub party: Vec<Fish>,
    pub dead: Vec<usize>,
    pub clock: u64,
    pub round: u32,
    pub phase: Phase,
    pub current: Option<usize>,
    pub log: Vec<LogLine>,
    pub haul: u64,
    pub purse: u128,
    pub purse_spent: u128,
    pub items: Vec<Pack>,
    pub setup: BattleSetup,
    pub rng: SmallRng,
    pub last_party_skill: Option<Skill>,
    pub last_enemy_skill: Option<Skill>,
    pub prayer: Prayer,
    pub offer: Option<usize>,
    pub strength: u32,
    pub acting: Option<&'static str>,
    pub palette: Vec<u32>,
    pub junk_found: u32,
    pub ambush_round: u32,
    pub lagging: Vec<(usize, Choice)>,
    pub turn: Option<usize>,
    pub sure: bool,
    named: usize,
    beats: Vec<Beat>,
}

pub struct Strike {
    pub landed: bool,
    pub damage: u32,
    pub crit: bool,
    pub at: Mark,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mark {
    log: usize,
    beats: usize,
}

impl Battle {
    pub fn new(party: Vec<Fish>, foes: Vec<Fighter>, setup: BattleSetup) -> Battle {
        let mut fighters: Vec<Fighter> = party
            .iter()
            .enumerate()
            .map(|(member, fish)| Fighter::of_kit(&Kit::of(fish), member))
            .collect();
        fighters.extend(foes);
        Battle {
            fighters,
            party,
            dead: Vec::new(),
            clock: 0,
            round: 1,
            phase: Phase::Running,
            current: None,
            log: Vec::new(),
            haul: 0,
            purse: setup.purse,
            purse_spent: 0,
            items: setup.items.clone(),
            rng: SmallRng::seed_from_u64(setup.seed),
            prayer: Prayer::of_seed(setup.seed),
            palette: super::floor::palette(setup.floor),
            setup,
            last_party_skill: None,
            last_enemy_skill: None,
            offer: None,
            strength: PERCENT,
            acting: None,
            junk_found: 0,
            ambush_round: 0,
            lagging: Vec::new(),
            turn: None,
            sure: false,
            named: 0,
            beats: Vec::new(),
        }
    }

    pub fn begin(&mut self) -> Vec<Beat> {
        self.distinguish();
        let count = self.fighters.len() as u64;
        for index in 0..self.fighters.len() {
            let fighter = &self.fighters[index];
            let wait = TURN_LENGTH / u64::from(fighter.speed.max(1));
            let stagger = wait * (index as u64) / (count * 10).max(1);
            let first = fighter.has(Trait::FirstStrike)
                || (fighter.side == Side::Foes && self.setup.ambushed)
                || (fighter.side == Side::Party && fighter.has(Trait::TidalClock));
            self.fighters[index].ready_at = if first { 0 } else { wait / 2 + stagger };
        }
        let lookout = self
            .fighters
            .iter()
            .any(|f| f.side == Side::Foes && f.has(Trait::Lookout));
        if lookout {
            for fighter in self.fighters.iter_mut().filter(|f| f.side == Side::Foes) {
                fighter.ready_at /= 4;
            }
        }
        self.say(format!("Round {}", self.round), Tone::Round, Pause::Round);
        rules::assemble(self);
        passives::assemble(self);
        reveal::assemble(self);
        reveal::foresee(self);
        self.advance();
        std::mem::take(&mut self.beats)
    }

    pub fn answer(&mut self, sign: bool) -> Vec<Beat> {
        let Phase::Offer(_) = self.phase else {
            return Vec::new();
        };
        rules::answer(self, sign);
        self.phase = Phase::Running;
        self.advance();
        std::mem::take(&mut self.beats)
    }

    pub fn pray(&mut self, words: &str) -> Vec<Beat> {
        rules::pray(self, words);
        std::mem::take(&mut self.beats)
    }

    pub fn offer(&self) -> Option<usize> {
        match self.phase {
            Phase::Offer(devil) => Some(devil),
            Phase::Running | Phase::Choosing(_) | Phase::Over(_) => None,
        }
    }

    pub fn choose(&mut self, choice: Choice) -> Vec<Beat> {
        let Phase::Choosing(actor) = self.phase else {
            return Vec::new();
        };
        if !self.can_choose(actor, choice) {
            return Vec::new();
        }
        self.act(actor, choice);
        if self.fighters[actor].has(Trait::Lag) {
            self.lagging.push((actor, choice));
        }
        if !self.settle_outcome_quietly()
            && self.outcome().is_none()
            && passives::acts_again(self, actor)
        {
            self.phase = Phase::Choosing(actor);
            return std::mem::take(&mut self.beats);
        }
        self.end_turn(actor);
        self.advance();
        std::mem::take(&mut self.beats)
    }

    pub fn actor(&self) -> Option<usize> {
        match self.phase {
            Phase::Choosing(actor) => Some(actor),
            Phase::Running | Phase::Offer(_) | Phase::Over(_) => None,
        }
    }

    pub fn outcome(&self) -> Option<Outcome> {
        match self.phase {
            Phase::Over(outcome) => Some(outcome),
            Phase::Running | Phase::Choosing(_) | Phase::Offer(_) => None,
        }
    }

    pub fn carry(&mut self, member: usize, hp: u32, belly_up: bool) {
        let Some(fighter) = self
            .fighters
            .iter_mut()
            .find(|f| f.member() == Some(member))
        else {
            return;
        };
        fighter.hp = hp.min(fighter.max_hp);
        fighter.belly_up = belly_up || fighter.hp == 0;
        fighter.last_hp = fighter.hp;
    }

    pub fn views(&self) -> Vec<View> {
        self.fighters.iter().map(View::of).collect()
    }

    pub fn side(&self, side: Side) -> impl Iterator<Item = usize> + '_ {
        (0..self.fighters.len()).filter(move |&i| self.fighters[i].side == side)
    }

    pub fn standing(&self, side: Side) -> Vec<usize> {
        self.side(side)
            .filter(|&i| self.fighters[i].standing())
            .collect()
    }

    pub fn turn_order(&self, length: usize) -> Vec<usize> {
        let mut clocks: Vec<(usize, u64)> = self
            .fighters
            .iter()
            .enumerate()
            .filter(|(_, f)| f.standing())
            .map(|(i, f)| (i, f.ready_at))
            .collect();
        let mut order = Vec::with_capacity(length);
        if let Some(actor) = self
            .turn
            .filter(|&actor| self.fighters[actor].in_the_fight())
        {
            order.push(actor);
        }
        if clocks.is_empty() {
            return order;
        }
        while order.len() < length {
            let (slot, &(index, at)) = clocks
                .iter()
                .enumerate()
                .min_by_key(|(_, (i, at))| (*at, *i))
                .expect("someone is standing");
            order.push(index);
            clocks[slot].1 = at + self.wait_of(index);
        }
        order
    }

    pub fn can_flee(&self) -> bool {
        self.setup.stakes.can_flee()
    }

    pub fn can_choose(&self, actor: usize, choice: Choice) -> bool {
        let fighter = &self.fighters[actor];
        match choice {
            Choice::Bite(target) => {
                !fighter.has_status(StatusKind::Puffed) && self.targetable(actor, target)
            }
            Choice::Skill { skill, target } => {
                let Some(skill) = fighter.skills.get(skill) else {
                    return false;
                };
                if fighter.has_status(StatusKind::SkillsLocked) {
                    return false;
                }
                self.affordable(actor, skill)
                    && (!skill.reach.picks_a_foe()
                        || target.is_some_and(|t| self.targetable(actor, t)))
                    && (!skill.reach.picks_an_ally() || target.is_some())
            }
            Choice::Guard => !fighter.has(Trait::PrideSin) && !fighter.has(Trait::Pride),
            Choice::Item { pack, target } => {
                self.items.get(pack).is_some_and(|p| p.count > 0) && target < self.fighters.len()
            }
            Choice::Flee => self.can_flee(),
        }
    }

    pub fn affordable(&self, actor: usize, skill: &Skill) -> bool {
        let fighter = &self.fighters[actor];
        if skill.once_a_fight && fighter.spent.contains(&skill.name) {
            return false;
        }
        match skill.price {
            Price::Bubbles(n) => fighter.mp >= u32::from(n),
            Price::Purse { percent } => self.purse * u128::from(percent) / 100 > 0,
            Price::Free => true,
        }
    }

    pub fn targetable(&self, actor: usize, target: usize) -> bool {
        let Some(t) = self.fighters.get(target) else {
            return false;
        };
        let attacker = &self.fighters[actor];
        if t.side == attacker.side || t.gone || t.belly_up {
            return false;
        }
        !t.has_status(StatusKind::OutOfReach)
            && !t.has_status(StatusKind::Away)
            && !rules::shielded(self, target)
    }

    pub fn wait_of(&self, index: usize) -> u64 {
        let fighter = &self.fighters[index];
        let slowed = fighter.status_amount(StatusKind::Slowed);
        let candied = fighter.status_amount(StatusKind::Candied);
        let hasted = fighter.status_amount(StatusKind::Hasted);
        let speed = (fighter.speed + hasted)
            .saturating_sub(slowed)
            .saturating_sub(candied)
            .max(1);
        TURN_LENGTH / u64::from(speed)
    }

    fn distinguish(&mut self) {
        self.named = self.fighters.len();
        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for fighter in &self.fighters {
            *counts.entry(fighter.base.clone()).or_default() += 1;
        }
        let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for fighter in &mut self.fighters {
            if counts[&fighter.base] < 2 {
                fighter.name = fighter.base.clone();
                continue;
            }
            let place = seen.entry(fighter.base.clone()).or_default();
            fighter.name = format!("{} {}", fighter.base, letter(*place));
            *place += 1;
        }
    }

    fn say(&mut self, text: impl Into<String>, tone: Tone, pause: Pause) {
        if self.fighters.len() != self.named {
            self.distinguish();
        }
        let line = LogLine::new(text, tone);
        self.log.push(line.clone());
        self.beats.push(Beat {
            line: Some(line),
            actor: self.current,
            pause,
            views: self.views(),
            order: self.turn_order(TURN_BAR_LENGTH),
        });
    }

    pub fn note(&mut self, text: impl Into<String>, tone: Tone) {
        self.say(text, tone, Pause::Result);
    }

    pub fn mark(&self) -> Mark {
        Mark {
            log: self.log.len(),
            beats: self.beats.len(),
        }
    }

    pub fn note_at(&mut self, at: Mark, text: impl Into<String>, tone: Tone) -> Mark {
        if at.log >= self.log.len() || at.beats >= self.beats.len() {
            self.note(text, tone);
            return self.mark();
        }
        let line = LogLine::new(text, tone);
        self.log.insert(at.log, line.clone());
        let after = &self.beats[at.beats];
        let beat = Beat {
            line: Some(line),
            actor: self.current,
            pause: Pause::Result,
            views: after.views.clone(),
            order: after.order.clone(),
        };
        self.beats.insert(at.beats, beat);
        Mark {
            log: at.log + 1,
            beats: at.beats + 1,
        }
    }

    fn think(&mut self, actor: usize) {
        self.beats.push(Beat {
            line: None,
            actor: Some(actor),
            pause: Pause::Think,
            views: self.views(),
            order: self.turn_order(TURN_BAR_LENGTH),
        });
    }

    fn advance(&mut self) {
        for _ in 0..MAX_STEPS {
            if self.settle_outcome() {
                return;
            }
            if let Some(devil) = self.offer {
                let name = self.fighters[devil].name.clone();
                self.note(
                    format!(
                        "{name} offers a contract: sign, and its HP and your haul both double."
                    ),
                    Tone::Tell,
                );
                self.phase = Phase::Offer(devil);
                return;
            }
            let Some(actor) = self.next_actor() else {
                self.phase = Phase::Over(Outcome::Wiped);
                return;
            };
            let at = self.fighters[actor].ready_at;
            let round = 1 + u32::try_from(at / ROUND_TICKS).unwrap_or(u32::MAX);
            while self.round < round {
                self.round += 1;
                self.say(format!("Round {}", self.round), Tone::Round, Pause::Round);
                self.round_start();
            }
            self.clock = at;
            self.fighters[actor].ready_at = match self.fighters[actor].has(Trait::AlwaysLast) {
                true => (at / ROUND_TICKS + LAST_IN_ROUND) * ROUND_TICKS - 1,
                false => at + self.wait_of(actor),
            };
            self.current = Some(actor);
            self.turn = Some(actor);
            if !self.start_turn(actor) {
                self.end_turn(actor);
                continue;
            }
            match self.fighters[actor].side {
                Side::Party if self.acts_on_its_own(actor) => {
                    let choice = self.auto_choice(actor);
                    self.act(actor, choice);
                    self.end_turn(actor);
                }
                Side::Party => {
                    self.catch_up(actor);
                    if self.settle_outcome() {
                        return;
                    }
                    self.phase = Phase::Choosing(actor);
                    return;
                }
                Side::Foes => {
                    self.enemy_turn(actor);
                    self.end_turn(actor);
                }
            }
        }
        self.phase = Phase::Over(Outcome::Wiped);
    }

    fn catch_up(&mut self, actor: usize) {
        let Some(at) = self.lagging.iter().position(|&(who, _)| who == actor) else {
            return;
        };
        let (_, choice) = self.lagging.remove(at);
        let ready = match choice {
            Choice::Guard | Choice::Flee => true,
            _ => self.can_choose(actor, choice),
        };
        if !ready {
            return;
        }
        let name = self.fighters[actor].name.clone();
        self.note(
            format!("{name}'s back half catches up and does it again."),
            Tone::Status,
        );
        self.act(actor, choice);
    }

    fn acts_on_its_own(&self, actor: usize) -> bool {
        let fighter = &self.fighters[actor];
        fighter.has_status(StatusKind::Dreaming) || fighter.has_status(StatusKind::Charmed)
    }

    fn auto_choice(&mut self, actor: usize) -> Choice {
        let charmed = self.fighters[actor].has_status(StatusKind::Charmed);
        let pool: Vec<usize> = if charmed {
            self.standing(Side::Party)
                .into_iter()
                .filter(|&i| i != actor)
                .collect()
        } else {
            (0..self.fighters.len())
                .filter(|&i| i != actor && self.fighters[i].standing())
                .collect()
        };
        match pool.is_empty() {
            true => Choice::Guard,
            false => Choice::Bite(pool[self.rng.random_range(0..pool.len())]),
        }
    }

    fn next_actor(&self) -> Option<usize> {
        self.fighters
            .iter()
            .enumerate()
            .filter(|(_, f)| f.in_the_fight() && !f.belly_up)
            .min_by_key(|(i, f)| (f.ready_at, *i))
            .map(|(i, _)| i)
    }

    fn settle_outcome(&mut self) -> bool {
        if let Phase::Over(Outcome::Fled) = self.phase {
            return true;
        }
        let foes_left = self.side(Side::Foes).any(|i| !self.fighters[i].gone);
        if !foes_left {
            self.phase = Phase::Over(Outcome::Won);
            self.note("The room is quiet.", Tone::Gold);
            return true;
        }
        let party_left = self.side(Side::Party).any(|i| self.fighters[i].standing())
            || self.side(Side::Party).any(|i| {
                let f = &self.fighters[i];
                !f.gone && !f.belly_up && f.has_status(StatusKind::Away)
            });
        if !party_left {
            self.phase = Phase::Over(Outcome::Wiped);
            self.note("Every fish is belly up.", Tone::Down);
            self.keep_the_fallen();
            return true;
        }
        false
    }

    fn round_start(&mut self) {
        for index in 0..self.fighters.len() {
            if !self.fighters[index].standing() {
                continue;
            }
            for t in self.fighters[index].traits.clone() {
                match t {
                    Trait::Upgrade(amount) => {
                        self.fighters[index].bite += amount;
                        let name = self.fighters[index].name.clone();
                        self.note(
                            format!("{name} upgrades itself: +{amount} Bite."),
                            Tone::Status,
                        );
                    }
                    Trait::Grows(amount) => {
                        let f = &mut self.fighters[index];
                        f.max_hp += amount;
                        f.hp += amount;
                    }
                    _ => {}
                }
            }
        }
        passives::round_start(self);
    }

    fn start_turn(&mut self, actor: usize) -> bool {
        let name = self.fighters[actor].name.clone();
        if !passives::turn_begins(self, actor) {
            self.fighters[actor].turns += 1;
            return false;
        }
        {
            let f = &mut self.fighters[actor];
            f.turns += 1;
            f.guarding = false;
            for kind in [
                StatusKind::Invisible,
                StatusKind::Untouchable,
                StatusKind::Puffed,
                StatusKind::OutOfReach,
            ] {
                f.clear_status(kind);
            }
        }
        if let Some(primed) = self.fighters[actor].status(StatusKind::Primed).copied() {
            self.fighters[actor].clear_status(StatusKind::Primed);
            self.release(actor, primed.amount);
        }
        if self.fighters[actor].rolling > 0 {
            let rest = std::mem::take(&mut self.fighters[actor].rolling);
            self.note(
                format!("{name}'s HP stops rolling: {rest} more."),
                Tone::Hurt,
            );
            self.lose_hp(actor, rest, None);
            if !self.fighters[actor].standing() {
                return false;
            }
        }
        for status in self.fighters[actor].statuses.clone() {
            if !status.kind.ticks() || status.amount == 0 {
                continue;
            }
            if status.kind == StatusKind::Poison && self.immune_to_poison(actor) {
                continue;
            }
            let verb = match status.kind {
                StatusKind::Poison => "Poison eats at",
                StatusKind::Burn => "Fire burns",
                StatusKind::Bleed => "Wounds bleed on",
                StatusKind::Infected => "The infection eats at",
                _ => "Rot spreads on",
            };
            self.note(format!("{verb} {name}: {}.", status.amount), Tone::Hurt);
            self.lose_hp(actor, status.amount, None);
            if status.kind == StatusKind::Infected {
                self.spread_infection(actor, status);
            }
            if !self.fighters[actor].standing() {
                return false;
            }
        }
        self.passives_at_turn_start(actor);
        if !self.fighters[actor].standing() {
            return false;
        }
        let held = self.fighters[actor]
            .statuses
            .iter()
            .any(|s| s.kind.stops_acting());
        self.tick_statuses(actor);
        if self.fighters[actor].side == Side::Party {
            self.fighters[actor].gain_bubbles(1);
        }
        if held {
            let away = self.fighters[actor].has_status(StatusKind::Away);
            if !away {
                self.note(format!("{name} is held and cannot act."), Tone::Status);
            }
            return false;
        }
        true
    }

    fn release(&mut self, actor: usize, amount: u32) {
        let name = self.fighters[actor].name.clone();
        self.note(
            format!("{name} strikes from below: {amount} to every fish."),
            Tone::Hurt,
        );
        let side = self.fighters[actor].side.other();
        for target in self.within_reach(side) {
            self.harm(actor, target, amount, DamageKind::Skill);
        }
    }

    fn spread_infection(&mut self, actor: usize, status: Status) {
        let side = self.fighters[actor].side;
        let clean: Vec<usize> = self
            .standing(side)
            .into_iter()
            .filter(|&i| i != actor && !self.fighters[i].has_status(StatusKind::Infected))
            .collect();
        if clean.is_empty() {
            return;
        }
        let next = clean[self.rng.random_range(0..clean.len())];
        self.fighters[next].inflict(status);
        let name = self.fighters[next].name.clone();
        self.note(format!("The infection spreads to {name}."), Tone::Status);
    }

    fn tick_statuses(&mut self, actor: usize) {
        let f = &mut self.fighters[actor];
        for status in f.statuses.iter_mut() {
            status.turns = status.turns.saturating_sub(1);
        }
        let finished: Vec<StatusKind> = f
            .statuses
            .iter()
            .filter(|s| s.turns == 0 && s.kind != StatusKind::Rot && s.kind != StatusKind::Contract)
            .map(|s| s.kind)
            .collect();
        f.statuses
            .retain(|s| s.turns > 0 || s.kind == StatusKind::Rot || s.kind == StatusKind::Contract);
        if finished.contains(&StatusKind::Away) {
            let name = f.name.clone();
            self.note(format!("{name} is back in the fight."), Tone::Status);
        }
    }

    fn passives_at_turn_start(&mut self, actor: usize) {
        let name = self.fighters[actor].name.clone();
        for t in self.fighters[actor].traits.clone() {
            match t {
                Trait::Regrow(percent) => {
                    let amount = (self.fighters[actor].max_hp * percent / PERCENT).max(1);
                    let healed = self.fighters[actor].heal(amount);
                    if healed > 0 {
                        self.note(format!("{name} regrows {healed}."), Tone::Heal);
                    }
                }
                Trait::Lustre(amount) => {
                    self.fighters[actor].heal(amount);
                }
                Trait::Exposure(amount) => {
                    let amount = self.fighters[actor].scale.bite(amount);
                    self.note(
                        format!("{name} glows. Every fish loses {amount}."),
                        Tone::Hurt,
                    );
                    for target in self.standing(Side::Party) {
                        self.lose_hp(target, amount, None);
                    }
                }
                Trait::Clownfish(amount) => {
                    let host = self
                        .standing(Side::Foes)
                        .into_iter()
                        .find(|&i| i != actor && self.fighters[i].hp < self.fighters[i].max_hp);
                    if let Some(host) = host {
                        let healed = self.fighters[host].heal(amount);
                        let host_name = self.fighters[host].name.clone();
                        self.note(format!("{name} tends {host_name}: +{healed}."), Tone::Heal);
                    }
                }
                Trait::Fuse { turns, damage } => {
                    let damage = self.fighters[actor].scale.bite(damage);
                    let left = u32::from(turns).saturating_sub(self.fighters[actor].turns);
                    if left == 0 {
                        self.note(
                            format!("{name} bursts for {damage} on every fish."),
                            Tone::Hurt,
                        );
                        for target in self.standing(Side::Party) {
                            self.harm(actor, target, damage, DamageKind::Skill);
                        }
                        self.beat(actor);
                    } else {
                        self.note(format!("{name}'s fuse burns down: {left}."), Tone::Tell);
                    }
                }
                _ => {}
            }
        }
        rules::turn_start(self, actor);
    }

    fn end_turn(&mut self, actor: usize) {
        if actor >= self.fighters.len() || self.fighters[actor].gone {
            return;
        }
        rules::turn_ended(self, actor);
        self.turn = None;
        if self.fighters[actor].side == Side::Foes && self.fighters[actor].standing() {
            let before = self.fighters[actor].mp;
            self.fighters[actor].gain_bubbles(1);
            self.announce_tell(actor, before);
            reveal::foresee(self);
        }
        self.fighters[actor].last_hp = self.fighters[actor].hp;
        if self.phase == Phase::Choosing(actor) {
            self.phase = Phase::Running;
        }
    }

    fn announce_tell(&mut self, actor: usize, before: u32) {
        let early = u32::from(passives::tells_early(self));
        let f = &self.fighters[actor];
        if f.mp == before || f.mp + early < f.max_mp || f.tell.is_some() {
            return;
        }
        let Some(big) = big_move(f) else {
            return;
        };
        let skill_name = f.skills[big].name;
        let name = f.name.clone();
        self.fighters[actor].tell = Some(big);
        let text = match self.fighters[actor].mp >= self.fighters[actor].max_mp {
            true => format!("{name}'s bubbles are full. It is ready to use {skill_name}."),
            false => format!("{name}'s bubbles are nearly full. Soon it will use {skill_name}."),
        };
        self.note(text, Tone::Tell);
        passives::reflex(self);
    }

    fn act(&mut self, actor: usize, choice: Choice) {
        let name = self.fighters[actor].name.clone();
        let choice = self.lead_astray(actor, choice);
        match choice {
            Choice::Bite(target) => self.bite(actor, target),
            Choice::Guard => {
                let f = &mut self.fighters[actor];
                f.guarding = true;
                f.rolling = 0;
                f.gain_bubbles(1);
                self.note(format!("{name} guards and gains a bubble."), Tone::Plain);
                passives::guarded(self, actor);
            }
            Choice::Skill { skill, target } => {
                let skill = self.fighters[actor].skills[skill].clone();
                self.use_skill(actor, &skill, target);
                passives::struck(self, actor);
                if self.fighters[actor].side == Side::Party {
                    passives::echo(self, actor);
                }
            }
            Choice::Item { pack, target } => self.use_item(actor, pack, target),
            Choice::Flee => self.flee(actor),
        }
        if let Choice::Bite(_) = choice {
            passives::struck(self, actor);
        }
        if let Some(target) = passives::reflection_bites(self, actor) {
            let name = self.fighters[actor].name.clone();
            self.note(
                format!("{name}'s reflection bites on its own."),
                Tone::Status,
            );
            self.bite(actor, target);
        }
    }

    fn lead_astray(&mut self, actor: usize, choice: Choice) -> Choice {
        if !self.fighters[actor].has_status(StatusKind::Astray) {
            return choice;
        }
        self.fighters[actor].clear_status(StatusKind::Astray);
        let anyone: Vec<usize> = (0..self.fighters.len())
            .filter(|&i| i != actor && self.fighters[i].standing())
            .collect();
        if anyone.is_empty() {
            return choice;
        }
        let stray = anyone[self.rng.random_range(0..anyone.len())];
        let name = self.fighters[actor].name.clone();
        self.note(format!("{name} is led astray."), Tone::Status);
        match choice {
            Choice::Bite(_) => Choice::Bite(stray),
            Choice::Skill { skill, .. } => Choice::Skill {
                skill,
                target: Some(stray),
            },
            Choice::Item { pack, .. } => Choice::Item {
                pack,
                target: stray,
            },
            Choice::Guard | Choice::Flee => choice,
        }
    }

    pub fn bite(&mut self, actor: usize, target: usize) {
        self.acting = Some(BITE);
        if passives::bites_a_ghost(self, actor) {
            return;
        }
        let target = passives::hydra_argues(self, actor).unwrap_or(target);
        if self.fighters[actor].has(Trait::Signature(Line::Kaiju)) {
            let side = self.fighters[actor].side.other();
            for foe in self.standing(side) {
                let strike = self.strike(actor, foe, Power::BITE, DamageKind::Bite);
                self.report_strike(actor, foe, &strike, "stomps");
                if strike.landed {
                    self.after_bitten(actor, foe, strike.damage);
                }
            }
            return;
        }
        let (hits, power) = bite_hits(&self.fighters[actor]);
        let name = self.fighters[actor].name.clone();
        let swarm = self.fighters[actor]
            .traits
            .iter()
            .any(|t| matches!(t, Trait::Swarm { .. }));
        if swarm {
            self.note(format!("{name} swarms."), Tone::Plain);
        }
        for hit in 0..hits {
            let aim = if swarm {
                match self.pick_target(actor) {
                    Some(t) => t,
                    None => return,
                }
            } else {
                target
            };
            if !self.fighters[aim].in_the_fight()
                || (self.fighters[aim].belly_up && !self.setup.difficulty.kills())
            {
                continue;
            }
            let strike = self.strike(actor, aim, power, DamageKind::Bite);
            self.report_strike(
                actor,
                aim,
                &strike,
                if hit == 0 { "bites" } else { "bites again" },
            );
            if strike.landed {
                self.after_bitten(actor, aim, strike.damage);
            }
        }
        if self.fighters[actor].has(Trait::Bill) {
            let second = self
                .standing(self.fighters[actor].side.other())
                .into_iter()
                .find(|&i| i != target);
            if let Some(second) = second {
                let strike =
                    self.strike(actor, second, Power::Bite { percent: 50 }, DamageKind::Bite);
                self.report_strike(actor, second, &strike, "catches");
            }
        }
    }

    pub fn strike(
        &mut self,
        actor: usize,
        target: usize,
        power: Power,
        kind: DamageKind,
    ) -> Strike {
        let miss = Strike {
            landed: false,
            damage: 0,
            crit: false,
            at: self.mark(),
        };
        if self.passes_through(actor, target, kind) {
            return miss;
        }
        if passives::misses(self, actor, target, kind) {
            return miss;
        }
        if !self.hits(actor, target, kind) {
            return miss;
        }
        let base = self.base_damage(actor, power);
        let crit = passives::crits(self, actor, target)
            || self.rng.random_range(0..PERCENT) < self.crit_chance(actor, target);
        let times = match crit {
            true => passives::crit_times(self, target).unwrap_or(self.crit_times(actor, target)),
            false => 1,
        };
        let raw = base * times;
        let percent = self.outgoing_percent(actor, target, kind)
            * self.incoming_percent(actor, target, kind)
            / PERCENT;
        let mut damage = raw * percent / PERCENT;
        let at = self.mark();
        damage = self.harm(actor, target, damage.max(1), kind);
        Strike {
            landed: true,
            damage,
            crit,
            at,
        }
    }

    fn passes_through(&self, actor: usize, target: usize, kind: DamageKind) -> bool {
        let t = &self.fighters[target];
        let sees = self.fighters[actor].has(Trait::SeesGhosts);
        if t.has_status(StatusKind::Untouchable) {
            return true;
        }
        if self.acting == Some(BITE) && t.has(Trait::NoBody) {
            return true;
        }
        kind == DamageKind::Bite && t.has_status(StatusKind::Invisible) && !sees
    }

    fn hits(&mut self, actor: usize, target: usize, kind: DamageKind) -> bool {
        let attacker = &self.fighters[actor];
        let t = &self.fighters[target];
        if attacker.has(Trait::Cyclops) || attacker.has(Trait::AllSeeing) {
            return true;
        }
        if attacker.has_status(StatusKind::Sure) {
            self.fighters[actor].clear_status(StatusKind::Sure);
            return true;
        }
        if self.sure {
            return true;
        }
        let chance = self.accuracy(actor);
        if self.rng.random_range(0..PERCENT) >= chance {
            return false;
        }
        if !kind.dodgeable() || t.has(Trait::Rooted) {
            return true;
        }
        let dodge = t.dodge + self.shimmer_dodge(target);
        self.rng.random_range(0..PERCENT) >= dodge
    }

    pub fn accuracy(&self, actor: usize) -> u32 {
        let attacker = &self.fighters[actor];
        if attacker.has(Trait::Cyclops) || attacker.has(Trait::AllSeeing) {
            return super::skill::SURE_ACCURACY;
        }
        let chance = super::skill::BASE_ACCURACY
            .saturating_sub(attacker.status_amount(StatusKind::AccuracyDown));
        match attacker.has_status(StatusKind::Blind) {
            true => chance * BLIND_MISS_PERCENT / PERCENT,
            false => chance,
        }
    }

    fn shimmer_dodge(&self, target: usize) -> u32 {
        let t = &self.fighters[target];
        t.traits
            .iter()
            .map(|tr| match tr {
                Trait::Shimmer {
                    percent,
                    every_other,
                    ..
                } => match every_other {
                    true if t.turns % 2 == 1 => 0,
                    _ => *percent,
                },
                Trait::Sparkle(percent) | Trait::Phase(percent) => *percent,
                _ => 0,
            })
            .sum()
    }

    fn base_damage(&mut self, actor: usize, power: Power) -> u32 {
        let (low, high) = self.spread(actor, power);
        let roll = self.rng.random_range(low..=high.max(low));
        (roll + PERCENT / 2) / PERCENT
    }

    pub fn damage_range(&self, actor: usize, power: Power) -> (u32, u32) {
        let (low, high) = self.spread(actor, power);
        (
            (low + PERCENT / 2) / PERCENT,
            (high + PERCENT / 2) / PERCENT,
        )
    }

    fn spread(&self, actor: usize, power: Power) -> (u32, u32) {
        let extra = passives::wrath_bite(self, actor) + passives::ledger(self, actor);
        let attacker = &self.fighters[actor];
        let fixed = |damage: u32| (damage * PERCENT, damage * PERCENT);
        match power {
            Power::Bite { percent } => {
                let bite = enraged_bite(attacker) + extra;
                (
                    bite * percent * SPREAD_LOW_PERCENT / PERCENT,
                    bite * percent * SPREAD_HIGH_PERCENT / PERCENT,
                )
            }
            Power::Flat(n) => fixed(attacker.scale.bite((n * self.strength / PERCENT).max(1))),
            Power::PerVoice(n) => {
                let side = attacker.side;
                let voices = self
                    .side(side)
                    .filter(|&i| self.fighters[i].standing())
                    .count();
                fixed(attacker.scale.bite(n * voices as u32))
            }
        }
    }

    pub fn crit_odds(&self, actor: usize, target: Option<usize>) -> (u32, u32) {
        match target {
            Some(target) => (
                self.crit_chance(actor, target),
                self.crit_times(actor, target),
            ),
            None => (
                self.fighters[actor].crit,
                match self.fighters[actor].has(Trait::Cyclops) {
                    true => HARSH_CRIT_TIMES,
                    false => CRIT_TIMES,
                },
            ),
        }
    }

    fn crit_chance(&self, actor: usize, target: usize) -> u32 {
        let attacker = &self.fighters[actor];
        let t = &self.fighters[target];
        let soft: u32 = t
            .traits
            .iter()
            .map(|tr| match tr {
                Trait::SoftEyes(p) => *p,
                _ => 0,
            })
            .sum();
        attacker.crit + soft
    }

    fn crit_times(&self, actor: usize, target: usize) -> u32 {
        let harsh =
            self.fighters[actor].has(Trait::Cyclops) || self.fighters[target].has(Trait::Soft);
        match harsh {
            true => HARSH_CRIT_TIMES,
            false => CRIT_TIMES,
        }
    }

    fn outgoing_percent(&self, actor: usize, target: usize, kind: DamageKind) -> u32 {
        let attacker = &self.fighters[actor];
        let t = &self.fighters[target];
        let mut percent = PERCENT;
        if attacker.has(Trait::Frenzy) && t.hp * HALF < t.max_hp {
            percent = percent * FRENZY_PERCENT / PERCENT;
        }
        if self.is_marked(target) {
            for tr in &attacker.traits {
                if let Trait::Grudge(extra) = tr {
                    percent = percent * (PERCENT + extra) / PERCENT;
                }
            }
        }
        if attacker.has(Trait::DevilMarked) {
            percent = percent * DEVIL_PERCENT / PERCENT;
        }
        if kind == DamageKind::Holy && (t.has(Trait::Ethereal) || self.is_marked(target)) {
            percent *= HALF;
        }
        percent * passives::outgoing(self, actor, target) / PERCENT
    }

    fn incoming_percent(&self, actor: usize, target: usize, kind: DamageKind) -> u32 {
        let t = &self.fighters[target];
        let mut percent = PERCENT;
        if kind.halved_by_ether()
            && t.has(Trait::Ethereal)
            && !self.fighters[actor].has(Trait::SeesGhosts)
        {
            percent /= HALF;
        }
        if t.has_status(StatusKind::Contract) {
            percent = percent * CONTRACT_PERCENT / PERCENT;
        }
        if t.side == Side::Foes {
            let shaded = self
                .standing(Side::Foes)
                .into_iter()
                .filter(|&i| i != target)
                .any(|i| {
                    self.fighters[i]
                        .traits
                        .iter()
                        .any(|tr| matches!(tr, Trait::Shade(_)))
                });
            if shaded {
                percent = percent * (PERCENT - SHADE_PERCENT) / PERCENT;
            }
        }
        percent * passives::incoming(self, actor, target) / PERCENT
    }

    pub fn is_marked(&self, index: usize) -> bool {
        let f = &self.fighters[index];
        f.has(Trait::DevilMarked)
            || f.member()
                .and_then(|m| self.party.get(m))
                .is_some_and(|fish| fish.devil_marked)
    }

    pub fn harm(&mut self, actor: usize, target: usize, amount: u32, kind: DamageKind) -> u32 {
        if rules::heals_from(self, target, kind) {
            let healed = self.fighters[target].heal(amount);
            let name = self.fighters[target].name.clone();
            self.note(format!("Holy light feeds {name}: +{healed}."), Tone::Heal);
            return 0;
        }
        if rules::absorbs_the_hit(self, target) {
            return 0;
        }
        let mut damage = amount * rules::learned_percent(self, target) / PERCENT;
        if kind.armoured() && !passives::ignores_armour(self, actor) {
            damage = damage.saturating_sub(self.fighters[target].armour).max(1);
        }
        if kind.shielded() && self.fighters[target].shield > 0 {
            let absorbed = self.fighters[target].shield.min(damage);
            self.fighters[target].shield -= absorbed;
            damage -= absorbed;
            if damage == 0 {
                return 0;
            }
        }
        let braced =
            self.fighters[target].guarding || self.fighters[target].has_status(StatusKind::Puffed);
        if braced && kind.guarded() {
            damage = damage.div_ceil(HALF);
        }
        if let Some(fluff) = self.fighters[target].trait_value(|t| match t {
            Trait::Fluff(n) => Some(*n),
            _ => None,
        }) && damage <= fluff
            && kind != DamageKind::True
        {
            let healed = self.fighters[target].heal(damage);
            let name = self.fighters[target].name.clone();
            self.note(
                format!("The hit only fluffs {name} up: +{healed}."),
                Tone::Heal,
            );
            return 0;
        }
        let threshold = self.fighters[target].max_hp * ROLL_THRESHOLD_PERCENT / PERCENT;
        let rolls = self.fighters[target].side == Side::Party
            && !self.fighters[target].guarding
            && damage >= threshold
            && damage > 1
            && !self.fighters[target].belly_up;
        if rolls {
            let now = damage.div_ceil(HALF);
            self.fighters[target].rolling += damage - now;
            self.lose_hp(target, now, Some(actor));
        } else {
            self.lose_hp(target, damage, Some(actor));
        }
        rules::hit_landed(self, target, damage);
        passives::after_hit(self, actor, target, damage);
        damage
    }

    pub fn lose_hp(&mut self, target: usize, amount: u32, by: Option<usize>) {
        let was_down = self.fighters[target].belly_up;
        let f = &mut self.fighters[target];
        f.hits_taken += 1;
        if was_down {
            if f.side == Side::Party && self.setup.difficulty.kills() && by.is_some() {
                self.death_blow(target);
            }
            return;
        }
        if amount >= f.hp && f.has(Trait::Neither) && !f.came_back {
            f.came_back = true;
            f.hp = f.max_hp / HALF;
            let name = f.name.clone();
            self.note(
                format!("{name} is not dead. It never was. It is back at half."),
                Tone::Status,
            );
            return;
        }
        f.hp = f.hp.saturating_sub(amount);
        if f.hp > 0 {
            if f.side == Side::Foes {
                self.check_split(target);
                rules::wounded(self, target);
            }
            return;
        }
        self.fall(target, by);
    }

    fn fall(&mut self, target: usize, by: Option<usize>) {
        let name = self.fighters[target].name.clone();
        match self.fighters[target].side {
            Side::Party => {
                if passives::saved_from_falling(self, target) {
                    return;
                }
                let f = &mut self.fighters[target];
                f.belly_up = true;
                f.rolling = 0;
                f.statuses.clear();
                self.note(format!("{name} is belly up."), Tone::Down);
                self.on_belly_up(target);
                passives::startle(self);
            }
            Side::Foes => {
                if self.fighters[target].has(Trait::Undying) && !self.fighters[target].came_back {
                    let f = &mut self.fighters[target];
                    f.came_back = true;
                    f.hp = f.max_hp / HALF;
                    self.note(
                        format!("{name} rattles back together at half."),
                        Tone::Status,
                    );
                    return;
                }
                if let Fate::Stands = rules::before_falling(self, target) {
                    return;
                }
                self.fighters[target].gone = true;
                self.fighters[target].hp = 0;
                self.give_back(target);
                let finder = by.filter(|&k| self.fighters[k].side == Side::Party);
                let percent = finder.map_or(PERCENT, |k| passives::gold_percent(self, k));
                let gold = self.enemy_gold(target) * u64::from(percent) / u64::from(PERCENT);
                self.haul += gold;
                self.note(
                    format!("{name} is beaten. ${gold} goes into the haul."),
                    Tone::Gold,
                );
                self.on_beaten(target, by);
                rules::after_falling(self, target, by);
            }
        }
    }

    fn give_back(&mut self, target: usize) {
        let gold = std::mem::take(&mut self.fighters[target].eaten_gold);
        let stolen = std::mem::take(&mut self.fighters[target].stolen);
        if gold > 0 {
            self.haul += gold;
            self.note(format!("${gold} comes back to the haul."), Tone::Gold);
        }
        for pack in stolen {
            if let Some(pack) = self.items.get_mut(pack) {
                pack.held = pack.held.saturating_sub(1);
                pack.count += 1;
                let item = pack.item.name();
                self.note(format!("The {item} comes back."), Tone::Status);
            }
        }
    }

    fn keep_the_fallen(&mut self) {
        if !self.setup.difficulty.kills() {
            return;
        }
        let down: Vec<usize> = self
            .side(Side::Party)
            .filter(|&i| self.fighters[i].belly_up && !self.fighters[i].gone)
            .collect();
        for fish in down {
            self.death_blow(fish);
        }
    }

    fn death_blow(&mut self, target: usize) {
        let name = self.fighters[target].name.clone();
        if passives::escapes(self, target) {
            return;
        }
        if self.fighters[target].has(Trait::Golden)
            && !self.fighters[target].has(Trait::Doomed)
            && self.rng.random_range(0..GOLDEN_SAVES_OF) != 0
        {
            self.note(format!("Luck holds {name} together."), Tone::Status);
            return;
        }
        if let Some(member) = self.fighters[target].member()
            && self.fighters[target].has(Trait::NineLives)
            && let Some(life) = self
                .party
                .get_mut(member)
                .map(|fish| fish.deep.get_or_insert_with(Default::default))
            && life.lives_spent < NINE_LIVES
        {
            life.lives_spent += 1;
            self.note(
                format!("{name} spends one of its nine lives."),
                Tone::Status,
            );
            return;
        }
        self.fighters[target].gone = true;
        if let Some(member) = self.fighters[target].member()
            && !self.dead.contains(&member)
        {
            self.dead.push(member);
        }
        self.note(format!("{name} dies."), Tone::Down);
    }

    fn on_belly_up(&mut self, target: usize) {
        for index in self.standing(Side::Foes) {
            if let Some(amount) = self.fighters[index].trait_value(|t| match t {
                Trait::Feast(n) => Some(*n),
                _ => None,
            }) {
                let healed = self.fighters[index].heal(amount);
                let name = self.fighters[index].name.clone();
                self.note(format!("{name} feasts: +{healed}."), Tone::Heal);
            }
        }
        let _ = target;
    }

    fn on_beaten(&mut self, target: usize, by: Option<usize>) {
        for t in self.fighters[target].traits.clone() {
            match t {
                Trait::DeathPoison { amount, turns } => {
                    let name = self.fighters[target].name.clone();
                    self.note(
                        format!("{name} bursts. Every fish is poisoned."),
                        Tone::Hurt,
                    );
                    for fish in self.standing(Side::Party) {
                        self.fighters[fish].inflict(Status::new(StatusKind::Poison, turns, amount));
                    }
                }
                Trait::Crumble => self.summon(target, CRUMBS, CRUMB_COUNT),
                _ => {}
            }
        }
        let _ = by;
    }

    fn check_split(&mut self, target: usize) {
        let f = &self.fighters[target];
        if !f.has(Trait::SplitsAtHalf) || f.came_back || f.hp * HALF >= f.max_hp {
            return;
        }
        if self.fighters.len() >= MAX_FIGHTERS {
            return;
        }
        let mut twin = self.fighters[target].clone();
        self.fighters[target].came_back = true;
        twin.came_back = true;
        twin.ready_at = self.clock + self.wait_of(target);
        let name = twin.name.clone();
        self.fighters.push(twin);
        self.note(format!("{name} splits in two."), Tone::Status);
    }

    fn enemy_gold(&self, target: usize) -> u64 {
        super::floor::gold_share(self.setup.floor, self.setup.difficulty)
            * self.fighters[target].shares
    }

    fn report_strike(&mut self, actor: usize, target: usize, strike: &Strike, verb: &str) {
        let name = self.fighters[actor].name.clone();
        let target_name = self.fighters[target].name.clone();
        if !strike.landed {
            self.note_at(
                strike.at,
                format!("{name} {verb} at {target_name} and misses."),
                Tone::Quiet,
            );
            return;
        }
        let crit = if strike.crit { ", a crit" } else { "" };
        let tone = match self.fighters[target].side {
            Side::Party => Tone::Hurt,
            Side::Foes => Tone::Hit,
        };
        let next = self.note_at(
            strike.at,
            format!("{name} {verb} {target_name} for {}{crit}.", strike.damage),
            tone,
        );
        let rolling = self.fighters[target].rolling;
        if rolling > 0 && self.fighters[target].standing() {
            self.note_at(
                next,
                format!(
                    "{target_name} is rolling: {rolling} more will land at the start of its turn."
                ),
                Tone::Tell,
            );
        }
    }

    fn after_bitten(&mut self, actor: usize, target: usize, damage: u32) {
        let target_name = self.fighters[target].name.clone();
        let actor_name = self.fighters[actor].name.clone();
        for t in self.fighters[target].traits.clone() {
            match t {
                Trait::Thorns(amount) | Trait::Contact(amount) => {
                    self.note(
                        format!("{target_name}'s spines cut {actor_name} for {amount}."),
                        Tone::Hit,
                    );
                    self.harm(target, actor, amount, DamageKind::True);
                }
                Trait::Spikes => {
                    let amount = SPIKES_BASE + damage * SPIKES_PERCENT / PERCENT;
                    self.note(
                        format!("{target_name}'s spikes cut {actor_name} for {amount}."),
                        Tone::Hit,
                    );
                    self.harm(target, actor, amount, DamageKind::True);
                }
                Trait::HoldsBiter => {
                    self.fighters[actor].inflict(Status::for_turns(StatusKind::Held, 1));
                    self.note(
                        format!("{target_name}'s tendrils hold {actor_name}."),
                        Tone::Status,
                    );
                }
                Trait::Spill(gold) => {
                    self.haul += u64::from(gold);
                    self.note(
                        format!("Gold spills from {target_name}: ${gold}."),
                        Tone::Gold,
                    );
                }
                Trait::Slime => self.fighters[actor].inflict(Status::new(StatusKind::Slowed, 2, 1)),
                Trait::Glitter => self.fighters[actor].inflict(Status::new(
                    StatusKind::AccuracyDown,
                    2,
                    GLITTER_ACCURACY,
                )),
                Trait::Burns(amount) => {
                    self.fighters[actor].inflict(Status::new(StatusKind::Burn, 2, amount))
                }
                _ => {}
            }
        }
        if self.fighters[target].has(Trait::Waiting) {
            self.fighters[target]
                .traits
                .retain(|t| *t != Trait::Waiting);
        }
        passives::after_bitten(self, actor, target);
        if self.fighters[actor].has(Trait::Tentacles)
            && self.rng.random_range(0..PERCENT) < TENTACLE_HOLD_PERCENT
        {
            self.fighters[target].inflict(Status::for_turns(StatusKind::Held, 1));
            self.note(
                format!("{actor_name}'s tentacles hold {target_name}."),
                Tone::Status,
            );
        }
    }

    fn immune_to_poison(&self, index: usize) -> bool {
        let f = &self.fighters[index];
        f.has(Trait::PoisonImmune) || f.has(Trait::Undead)
    }

    pub fn use_skill(&mut self, actor: usize, skill: &Skill, target: Option<usize>) {
        let name = self.fighters[actor].name.clone();
        self.acting = Some(skill.name);
        match skill.price {
            Price::Bubbles(n) => self.fighters[actor].lose_bubbles(u32::from(n)),
            Price::Purse { percent } => {
                let spent = self.purse * u128::from(percent) / 100;
                self.purse -= spent;
                self.purse_spent += spent;
            }
            Price::Free => {}
        }
        if skill.once_a_fight {
            self.fighters[actor].spent.push(skill.name);
        }
        match self.fighters[actor].side {
            Side::Party => self.last_party_skill = Some(skill.clone()),
            Side::Foes => self.last_enemy_skill = Some(skill.clone()),
        }
        self.note(format!("{name} uses {}.", skill.name), Tone::Plain);
        let targets = self.reach(actor, skill.reach, target);
        self.sure = skill.accuracy >= super::skill::SURE_ACCURACY;
        for effect in &skill.effects {
            self.apply(actor, *effect, &targets, skill);
            if self.settle_outcome_quietly() {
                break;
            }
        }
        self.sure = false;
    }

    pub fn within_reach(&self, side: Side) -> Vec<usize> {
        let cruel = side == Side::Party && self.setup.difficulty.kills();
        self.side(side)
            .filter(|&i| {
                let f = &self.fighters[i];
                let there = match cruel && f.belly_up {
                    true => !f.gone,
                    false => f.standing(),
                };
                there && !rules::shielded(self, i)
            })
            .collect()
    }

    fn settle_outcome_quietly(&self) -> bool {
        !self.side(Side::Foes).any(|i| !self.fighters[i].gone)
    }

    pub fn reach(&mut self, actor: usize, reach: Reach, target: Option<usize>) -> Vec<usize> {
        let side = self.fighters[actor].side;
        let foes = self.within_reach(side.other());
        let allies = self.standing(side);
        match reach {
            Reach::Me => vec![actor],
            Reach::OneFoe | Reach::BellyUpFoe => target
                .or_else(|| self.pick_target(actor))
                .into_iter()
                .collect(),
            Reach::TwoFoes => {
                let first = target.or_else(|| self.pick_target(actor));
                let mut picks: Vec<usize> = first.into_iter().collect();
                let rest: Vec<usize> = foes.iter().copied().filter(|i| Some(*i) != first).collect();
                if !rest.is_empty() {
                    picks.push(rest[self.rng.random_range(0..rest.len())]);
                }
                picks
            }
            Reach::AllFoes | Reach::RandomFoes(_) => foes,
            Reach::MarkedFoes => foes.into_iter().filter(|&i| self.is_marked(i)).collect(),
            Reach::OneAlly | Reach::BellyUpAlly | Reach::FallenAlly => target.into_iter().collect(),
            Reach::OtherAllies => allies.into_iter().filter(|&i| i != actor).collect(),
            Reach::AllAllies => allies,
            Reach::Everyone => target.map(|t| vec![t]).unwrap_or_else(|| {
                (0..self.fighters.len())
                    .filter(|&i| self.fighters[i].standing())
                    .collect()
            }),
            Reach::WeakestAlly => allies
                .into_iter()
                .min_by_key(|&i| self.fighters[i].hp_percent())
                .into_iter()
                .collect(),
        }
    }

    pub fn apply(&mut self, actor: usize, effect: Effect, targets: &[usize], skill: &Skill) {
        let name = self.fighters[actor].name.clone();
        match effect {
            Effect::Hit { power, kind, hits } => {
                let random = matches!(skill.reach, Reach::RandomFoes(_));
                let aims: Vec<usize> = match random {
                    true => Vec::new(),
                    false => targets.to_vec(),
                };
                if random {
                    for _ in 0..hits {
                        let Some(target) = self.pick_target(actor) else {
                            break;
                        };
                        let strike = self.strike(actor, target, power, kind);
                        self.report_strike(actor, target, &strike, "hits");
                    }
                }
                for target in aims {
                    if passives::sweeps_miss(self, skill.reach, target) {
                        let target_name = self.fighters[target].name.clone();
                        self.note(format!("It sweeps under {target_name}."), Tone::Quiet);
                        continue;
                    }
                    for _ in 0..hits {
                        if !self.fighters[target].in_the_fight() {
                            break;
                        }
                        let strike = self.strike(actor, target, power, kind);
                        self.report_strike(actor, target, &strike, "hits");
                        if strike.landed && kind == DamageKind::Bite {
                            self.after_bitten(actor, target, strike.damage);
                        }
                    }
                }
            }
            Effect::Heal(amount) => {
                let amount = self.fighters[actor].scale.hp(amount);
                for &target in targets {
                    self.heal(target, amount);
                }
            }
            Effect::HealPercent(percent) => {
                for &target in targets {
                    let amount = self.fighters[target].max_hp * percent / PERCENT;
                    self.heal(target, amount);
                }
            }
            Effect::Shield(amount) => {
                let amount = self.fighters[actor].scale.hp(amount);
                for &target in targets {
                    self.fighters[target].shield += amount;
                    let target_name = self.fighters[target].name.clone();
                    self.note(
                        format!("{target_name} is shielded for {amount}."),
                        Tone::Status,
                    );
                }
            }
            Effect::Bubbles(delta) => {
                for &target in targets {
                    let target_name = self.fighters[target].name.clone();
                    match delta {
                        d if d >= 0 => self.fighters[target].gain_bubbles(d.unsigned_abs().into()),
                        d => self.fighters[target].lose_bubbles(d.unsigned_abs().into()),
                    }
                    let verb = if delta >= 0 { "gains" } else { "loses" };
                    self.note(format!("{target_name} {verb} a bubble."), Tone::Status);
                }
            }
            Effect::Inflict(status) => {
                for &target in targets {
                    self.inflict(actor, target, status);
                }
            }
            Effect::Cleanse => {
                for &target in targets {
                    self.fighters[target]
                        .statuses
                        .retain(|s| !s.kind.is_debuff());
                }
            }
            Effect::Revive { percent } => {
                for &target in targets {
                    self.revive(target, percent);
                }
            }
            Effect::Summon { enemy, count } => self.summon(actor, enemy, count),
            Effect::Split => self.split(actor),
            Effect::Drain(amount) => {
                let amount = self.fighters[actor].scale.bite(amount);
                for &target in targets {
                    let at = self.mark();
                    let taken = self.harm(actor, target, amount, DamageKind::Skill);
                    let target_name = self.fighters[target].name.clone();
                    self.note_at(
                        at,
                        format!("{name} drains {taken} from {target_name}."),
                        Tone::Hurt,
                    );
                    self.fighters[actor].heal(taken);
                }
            }
            Effect::Recoil(amount) => {
                self.note(format!("{name} takes {amount} itself."), Tone::Hit);
                self.lose_hp(actor, amount, None);
            }
            Effect::Mutate(count) => {
                for &target in targets {
                    self.mutate(target, count);
                }
            }
            Effect::StealItem => {
                if let Some(item) = self.take_a_stack(actor) {
                    self.note(format!("{name} steals a {item}."), Tone::Status);
                }
            }
            Effect::StealGold { percent } => {
                let taken = self.haul * u64::from(percent) / u64::from(PERCENT);
                self.haul -= taken;
                self.fighters[actor].eaten_gold += taken;
                self.note(format!("{name} takes ${taken} from the haul."), Tone::Gold);
            }
            Effect::Haul(gold) => {
                let gold = u64::from(gold) * u64::from(skill.hits().max(1));
                self.haul += gold;
                self.note(format!("${gold} goes into the haul."), Tone::Gold);
            }
            Effect::Special(special) => {
                super::specials::perform(self, actor, special, targets, skill)
            }
        }
    }

    pub fn take_a_stack(&mut self, actor: usize) -> Option<&'static str> {
        let index = self.items.iter().rposition(|pack| pack.count > 0)?;
        let pack = &mut self.items[index];
        pack.count -= 1;
        pack.held += 1;
        let item = pack.item.name();
        self.fighters[actor].stolen.push(index);
        Some(item)
    }

    pub fn inflict(&mut self, actor: usize, target: usize, status: Status) {
        let status = match status.kind.ticks() || status.kind == StatusKind::Primed {
            true => Status {
                amount: self.fighters[actor].scale.bite(status.amount),
                ..status
            },
            false => status,
        };
        let t = &self.fighters[target];
        if status.kind == StatusKind::Held && (t.has(Trait::Keel) || t.is_boss()) {
            return;
        }
        if status.kind == StatusKind::Poison && self.immune_to_poison(target) {
            return;
        }
        let negative = t.has(Trait::Negative);
        let inverted = t.has(Trait::Inverted) && !status.kind.is_debuff();
        let status = match (negative, status.kind.mirror()) {
            (true, Some(kind)) => Status { kind, ..status },
            _ => status,
        };
        if inverted {
            let mirrored = status.kind.mirror().unwrap_or(StatusKind::BiteDown);
            self.fighters[actor].inflict(Status {
                kind: mirrored,
                ..status
            });
            return;
        }
        let name = self.fighters[target].name.clone();
        if passives::resists(self, target, status) {
            self.note(format!("{name} shrugs it off."), Tone::Quiet);
            return;
        }
        if status.kind == StatusKind::Burn {
            self.unwrap(target);
        }
        if status.kind == StatusKind::Candied {
            self.candy_coat(target, status);
            return;
        }
        self.fighters[target].inflict(status);
        self.note(format!("{name} is {}.", status.kind.name()), Tone::Status);
        passives::envies(self, target, status);
    }

    fn unwrap(&mut self, target: usize) {
        let Some(wraps) = self.fighters[target].trait_value(|t| match t {
            Trait::Wrapped(n) => Some(*n),
            _ => None,
        }) else {
            return;
        };
        let f = &mut self.fighters[target];
        f.traits.retain(|t| !matches!(t, Trait::Wrapped(_)));
        f.armour = f.armour.saturating_sub(wraps);
        let name = f.name.clone();
        self.note(format!("{name}'s wrappings burn away."), Tone::Status);
    }

    fn candy_coat(&mut self, target: usize, status: Status) {
        let name = self.fighters[target].name.clone();
        let layers = self.fighters[target].status_amount(StatusKind::Candied) + status.amount;
        self.fighters[target].clear_status(StatusKind::Candied);
        if layers >= CANDIED_STUCK {
            self.fighters[target].inflict(Status::for_turns(StatusKind::Held, 1));
            self.note(format!("{name} is candied stuck."), Tone::Status);
            return;
        }
        self.fighters[target].inflict(Status {
            amount: layers,
            ..status
        });
        self.note(format!("{name} is candied: {layers}."), Tone::Status);
    }

    pub fn heal(&mut self, target: usize, amount: u32) {
        self.heal_with(target, amount, false);
    }

    pub fn heal_with(&mut self, target: usize, amount: u32, food: bool) {
        let name = self.fighters[target].name.clone();
        let healer = self.current;
        if passives::holy_hurts(self, healer, target) {
            self.note(format!("Holy light burns {name}."), Tone::Hurt);
            self.lose_hp(target, amount, healer);
            return;
        }
        let amount = amount * passives::heal_percent(self, healer, target, food) / PERCENT;
        let prickly = self.fighters[target].has(Trait::Spikes);
        let amount = match prickly {
            true => amount * (PERCENT - PRICKLY_PERCENT) / PERCENT,
            false => amount,
        };
        if self.fighters[target].belly_up {
            self.right(target, amount);
            return;
        }
        let healed = self.fighters[target].heal(amount);
        self.note(format!("{name} heals {healed}."), Tone::Heal);
    }

    pub fn right(&mut self, target: usize, amount: u32) {
        let f = &mut self.fighters[target];
        if f.gone || !f.belly_up {
            return;
        }
        f.belly_up = false;
        f.hp = amount.max(1).min(f.max_hp);
        let name = f.name.clone();
        let wait = self.wait_of(target);
        self.fighters[target].ready_at = self.clock + wait;
        self.note(format!("{name} rights itself."), Tone::Heal);
    }

    pub fn revive(&mut self, target: usize, percent: u32) {
        let amount = self.fighters[target].max_hp * percent / PERCENT;
        self.right(target, amount);
    }

    pub fn split(&mut self, actor: usize) {
        let hp = self.fighters[actor].hp;
        if hp <= 1 || self.fighters.len() >= MAX_FIGHTERS {
            return;
        }
        self.fighters[actor].hp = hp.div_ceil(HALF);
        let mut twin = self.fighters[actor].clone();
        twin.hp = hp / HALF;
        twin.ready_at = self.clock + self.wait_of(actor);
        let name = twin.name.clone();
        self.fighters.push(twin);
        self.note(format!("{name} splits in two."), Tone::Status);
    }

    pub fn raise(&mut self, near: usize, haunt: Haunt) {
        let ghosts: Vec<&Ghost> = self
            .setup
            .haunts
            .iter()
            .filter(|ghost| ghost.haunt == haunt)
            .collect();
        let spec = super::bestiary::all().find(|spec| spec.traits.contains(&Trait::Yours(haunt)));
        let (Some(spec), false) = (spec, ghosts.is_empty()) else {
            self.note("Nothing answers.", Tone::Quiet);
            return;
        };
        if self.fighters.len() >= MAX_FIGHTERS {
            return;
        }
        let ghost = ghosts[self.rng.random_range(0..ghosts.len())].clone();
        let mut fighter = Fighter::of_ghost(spec, &ghost, self.setup.floor, self.setup.scale);
        fighter.side = self.fighters[near].side;
        fighter.ready_at = self.clock + self.wait_of(near);
        let name = fighter.name.clone();
        self.fighters.push(fighter);
        self.note(format!("{name} rises."), Tone::Status);
    }

    pub fn beat_down(&mut self, index: usize) {
        self.fighters[index].belly_up = false;
        self.beat(index);
    }

    pub fn summon(&mut self, actor: usize, enemy: &str, count: u8) {
        let Some(spec) = super::bestiary::find(enemy) else {
            return;
        };
        if let Some(haunt) = spec.traits.iter().find_map(|t| match t {
            Trait::Yours(haunt) => Some(*haunt),
            _ => None,
        }) {
            for _ in 0..count {
                self.raise(actor, haunt);
            }
            return;
        }
        for _ in 0..count {
            if self.fighters.len() >= MAX_FIGHTERS {
                return;
            }
            let mut fighter = Fighter::of_enemy(spec, self.setup.floor, self.setup.scale);
            fighter.side = self.fighters[actor].side;
            fighter.ready_at = self.clock + self.wait_of(actor);
            self.fighters.push(fighter);
        }
        let name = spec.name;
        let what = match count {
            1 => format!("A {name} joins the fight."),
            n => format!("{n} {name}s join the fight."),
        };
        self.note(what, Tone::Status);
    }

    pub fn mutate(&mut self, target: usize, count: u8) {
        let name = self.fighters[target].name.clone();
        if passives::mutation_immune(self, target) {
            self.note(format!("Nothing in {name} can change."), Tone::Quiet);
            return;
        }
        let Some(member) = self.fighters[target].member() else {
            for _ in 0..count {
                let buff = self.rng.random_range(0..ENEMY_MUTATION_BITE);
                self.fighters[target].bite += buff + 1;
            }
            self.note(format!("{name} mutates."), Tone::Status);
            return;
        };
        let mut mutated = 0;
        for _ in 0..count {
            let fish = &mut self.party[member];
            let choices = super::specials::wearable_mutations(fish);
            if choices.is_empty() {
                break;
            }
            let pick = choices[self.rng.random_range(0..choices.len())];
            crate::fishes::mutations::apply_mutation(fish, pick, &mut self.rng);
            mutated += 1;
        }
        if mutated > 0 {
            self.refit(target);
            self.note(format!("{name} mutates."), Tone::Status);
            passives::mutated(self);
        }
    }

    pub fn refit(&mut self, target: usize) {
        let Some(member) = self.fighters[target].member() else {
            return;
        };
        let kit = Kit::of(&self.party[member]);
        let f = &mut self.fighters[target];
        let lost = f.max_hp.saturating_sub(f.hp);
        f.max_hp = kit.vigor;
        f.hp = kit.vigor.saturating_sub(lost).max(u32::from(!f.belly_up));
        f.max_mp = kit.max_mp;
        f.mp = f.mp.min(f.max_mp);
        f.bite = kit.bite;
        f.speed = kit.speed;
        f.dodge = kit.dodge;
        f.crit = kit.crit;
        f.armour = kit.armour;
        f.traits = kit.traits;
        f.skills = kit.skills;
    }

    pub fn use_item(&mut self, actor: usize, pack: usize, target: usize) {
        let Some(item) = self.items.get(pack).map(|p| p.item) else {
            return;
        };
        self.items[pack].count -= 1;
        let name = self.fighters[actor].name.clone();
        self.note(format!("{name} uses {}.", item.name()), Tone::Plain);
        reveal::gossip(self, item.name());
        super::items::apply(self, item, target);
    }

    fn flee(&mut self, actor: usize) {
        let name = self.fighters[actor].name.clone();
        if !self.can_flee() {
            self.note("There is no way out until the fight ends.", Tone::Quiet);
            return;
        }
        let always = self.standing(Side::Party).iter().any(|&i| {
            let f = &self.fighters[i];
            f.has(Trait::Walks) || f.has(Trait::Hover)
        });
        let party = self.fastest(Side::Party) as i32;
        let foes = self.fastest(Side::Foes) as i32;
        let chance = (FLEE_BASE_PERCENT + FLEE_PER_SPEED * (party - foes))
            .clamp(FLEE_MIN_PERCENT, FLEE_MAX_PERCENT) as u32;
        let cannot = self
            .standing(Side::Party)
            .iter()
            .any(|&i| self.fighters[i].has(Trait::Backwards));
        if !cannot && (always || self.rng.random_range(0..PERCENT) < chance) {
            passives::flee_toll(self);
            self.note(format!("{name} leads the party away."), Tone::Plain);
            self.phase = Phase::Over(Outcome::Fled);
            return;
        }
        self.note("The party tries to flee and is cut off.", Tone::Quiet);
    }

    fn fastest(&self, side: Side) -> u32 {
        self.standing(side)
            .into_iter()
            .map(|i| self.fighters[i].speed)
            .max()
            .unwrap_or(0)
    }

    fn enemy_turn(&mut self, actor: usize) {
        self.think(actor);
        if self.fighters[actor].has(Trait::HalfRemembered) && rules::half_remembered(self, actor) {
            return;
        }
        if self.fighters[actor].has(Trait::Waiting) {
            let name = self.fighters[actor].name.clone();
            self.note(format!("{name} waits."), Tone::Quiet);
            return;
        }
        let plan = reveal::next_move(self, actor);
        let plan = match (self.fighters[actor].has_status(StatusKind::Astray), plan) {
            (true, super::ai::Plan::Bite(_) | super::ai::Plan::Skill { .. }) => {
                match self.lead_astray(actor, Choice::Bite(actor)) {
                    Choice::Bite(stray) if stray != actor => super::ai::Plan::Bite(stray),
                    _ => plan,
                }
            }
            _ => plan,
        };
        match plan {
            super::ai::Plan::Skill { skill, target } => {
                let skill = self.fighters[actor].skills[skill].clone();
                self.fighters[actor].tell = None;
                self.use_skill(actor, &skill, target);
            }
            super::ai::Plan::Bite(target) => self.bite(actor, target),
            super::ai::Plan::Idle => {
                let name = self.fighters[actor].name.clone();
                self.note(format!("{name} watches."), Tone::Quiet);
            }
        }
    }

    pub fn pick_target(&mut self, actor: usize) -> Option<usize> {
        super::ai::pick_target(self, actor)
    }

    pub fn beat(&mut self, index: usize) {
        if self.fighters[index].gone {
            return;
        }
        self.fighters[index].hp = 0;
        self.fall(index, None);
    }

    pub fn finish(self) -> Aftermath {
        Aftermath {
            outcome: self.outcome().unwrap_or(Outcome::Wiped),
            party: self.party,
            dead: self.dead,
            hp: self
                .fighters
                .iter()
                .filter_map(|f| f.member().map(|m| (m, f.hp, f.belly_up)))
                .collect(),
            haul: self.haul,
            junk_found: self.junk_found,
            purse_spent: self.purse_spent,
            items: self.items,
            beaten: self
                .fighters
                .iter()
                .filter(|f| f.side == Side::Foes && f.gone)
                .count() as u32,
            spoils: self
                .fighters
                .iter()
                .filter(|f| f.side == Side::Foes)
                .filter_map(|f| f.spec().map(|spec| (spec.name, f.damage_taken)))
                .collect(),
        }
    }
}

pub struct Aftermath {
    pub outcome: Outcome,
    pub party: Vec<Fish>,
    pub dead: Vec<usize>,
    pub hp: Vec<(usize, u32, bool)>,
    pub haul: u64,
    pub junk_found: u32,
    pub purse_spent: u128,
    pub items: Vec<Pack>,
    pub beaten: u32,
    pub spoils: Vec<(&'static str, u32)>,
}

pub fn big_move(fighter: &Fighter) -> Option<usize> {
    fighter
        .skills
        .iter()
        .enumerate()
        .filter(|(_, skill)| {
            skill.bubbles() >= BIG_MOVE_BUBBLES && u32::from(skill.bubbles()) >= fighter.max_mp
        })
        .max_by_key(|(_, skill)| skill.bubbles())
        .map(|(i, _)| i)
}

pub fn bite_hits(fighter: &Fighter) -> (u8, Power) {
    for t in &fighter.traits {
        match t {
            Trait::Swarm { bites, power } => return (*bites, *power),
            Trait::ExtraBites(n) => return (*n, Power::BITE),
            Trait::Hydra(heads) => return (*heads, Power::BITE),
            Trait::Shoal => {
                return (
                    SHOAL_BITES,
                    Power::Bite {
                        percent: SHOAL_PERCENT,
                    },
                );
            }
            _ => {}
        }
    }
    let tentacle = u8::from(fighter.has(Trait::Tentacles));
    (1 + tentacle, Power::BITE)
}

fn letter(place: usize) -> String {
    match LETTERS.chars().nth(place) {
        Some(letter) => letter.to_string(),
        None => (place + 1).to_string(),
    }
}

fn enraged_bite(fighter: &Fighter) -> u32 {
    let bite = fighter.effective_bite();
    match fighter.has(Trait::Enrage) && fighter.hp * HALF < fighter.max_hp {
        true => bite * ENRAGE_TIMES,
        false => bite,
    }
}

pub const BIG_MOVE_BUBBLES: u8 = 2;
pub const BLESSING_ROUNDS: u32 = 3;
pub const LAP_TURNS: u32 = 4;
pub const LAST_IN_ROUND: u64 = 2;
pub const BITE: &str = "Bite";
pub const CANDIED_STUCK: u32 = 5;
pub const SHOAL_BITES: u8 = 3;
pub const SHOAL_PERCENT: u32 = 40;
pub const DEVIL_PERCENT: u32 = 120;
pub const SHADE_PERCENT: u32 = 20;
pub const GOLDEN_SAVES_OF: u32 = 4;
pub const NINE_LIVES: u32 = 9;
pub const SPIKES_BASE: u32 = 3;
pub const SPIKES_PERCENT: u32 = 10;
pub const PRICKLY_PERCENT: u32 = 20;
pub const GLITTER_ACCURACY: u32 = 20;
pub const TENTACLE_HOLD_PERCENT: u32 = 30;
pub const ENEMY_MUTATION_BITE: u32 = 3;
pub const CRUMBS: &str = "Gingerbread Fry";
pub const CRUMB_COUNT: u8 = 3;
pub const LETTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

#[cfg(test)]
mod tests;
