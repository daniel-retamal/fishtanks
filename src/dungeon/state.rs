use std::collections::VecDeque;

use crate::ui::input_action::InputAction;
use crate::ui::layout::Scroll;

use super::battle::{Battle, Choice, Outcome, Phase, TURN_BAR_LENGTH};
use super::beat::{Beat, LogLine, Pause, View};
use super::effect::Reach;
use super::expedition::{Input, play};
use super::fighter::Side;
use super::info::Info;
use super::items::Aim;
use super::traits::Trait;

pub const THINK_SECS: f32 = 0.7;
pub const BEAT_SECS: f32 = 0.55;
pub const ROUND_SECS: f32 = 0.55;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Bite,
    Skills,
    Guard,
    Items,
    Flee,
}

impl Command {
    pub const ALL: [Command; 5] = [
        Command::Bite,
        Command::Skills,
        Command::Guard,
        Command::Items,
        Command::Flee,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Command::Bite => "Bite",
            Command::Skills => "Skills",
            Command::Guard => "Guard",
            Command::Items => "Items",
            Command::Flee => "Flee",
        }
    }

    pub fn opens_a_list(self) -> bool {
        matches!(self, Command::Skills | Command::Items)
    }

    fn index(self) -> usize {
        Command::ALL
            .iter()
            .position(|&command| command == self)
            .unwrap_or(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pending {
    Bite,
    Skill(usize),
    Item(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Sign,
    Refuse,
}

impl Answer {
    pub const ALL: [Answer; 2] = [Answer::Sign, Answer::Refuse];

    pub fn name(self) -> &'static str {
        match self {
            Answer::Sign => "Sign",
            Answer::Refuse => "Refuse",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Menu {
    Main(usize),
    Skills(usize),
    Items(usize),
    Target { pending: Pending, target: usize },
    Offer(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FightStep {
    Stay,
    Close,
    Played,
    Done,
}

#[derive(Clone, Debug, Default)]
pub struct Shown {
    pub views: Vec<View>,
    pub log: Vec<LogLine>,
    pub order: Vec<usize>,
    pub thinking: Option<usize>,
}

pub struct FightState {
    pub battle: Battle,
    pub title: String,
    pub menu: Menu,
    pub shown: Shown,
    pub played: Vec<Input>,
    pub scroll: Scroll,
    queue: VecDeque<Beat>,
    wait: f32,
}

impl FightState {
    pub fn fresh(battle: Battle, beats: Vec<Beat>, title: String) -> Self {
        let (views, order) = match beats.first() {
            Some(beat) => (beat.views.clone(), beat.order.clone()),
            None => (battle.views(), battle.turn_order(TURN_BAR_LENGTH)),
        };
        let mut state = FightState {
            shown: Shown {
                views,
                order,
                ..Shown::default()
            },
            battle,
            title,
            menu: Menu::Main(0),
            played: Vec::new(),
            scroll: Scroll::default(),
            queue: VecDeque::new(),
            wait: 0.0,
        };
        state.queue.extend(beats);
        state
    }

    pub fn resumed(battle: Battle, title: String) -> Self {
        let shown = Shown {
            views: battle.views(),
            log: battle.log.clone(),
            order: battle.turn_order(TURN_BAR_LENGTH),
            thinking: None,
        };
        let menu = match battle.offer() {
            Some(_) => Menu::Offer(0),
            None => Menu::Main(0),
        };
        FightState {
            battle,
            title,
            menu,
            shown,
            played: Vec::new(),
            scroll: Scroll::default(),
            queue: VecDeque::new(),
            wait: 0.0,
        }
    }

    pub fn is_busy(&self) -> bool {
        !self.queue.is_empty() || self.wait > 0.0
    }

    pub fn praying(&self) -> bool {
        let battle = &self.battle;
        let shielded = battle
            .side(Side::Foes)
            .any(|i| battle.fighters[i].has(Trait::PrayerShield) && !battle.fighters[i].gone);
        shielded && battle.prayer.line().is_some() && battle.outcome().is_none()
    }

    pub fn prayer(&self) -> Option<&'static str> {
        if self.actor().is_none() || self.battle.offer().is_some() || !self.praying() {
            return None;
        }
        match self.battle.prayer.shields() {
            true => self.battle.prayer.line(),
            false => None,
        }
    }

    pub fn pray(&mut self, words: String) -> FightStep {
        if self.prayer().is_none() {
            return FightStep::Stay;
        }
        self.play(Input::Pray(words))
    }

    pub fn actor(&self) -> Option<usize> {
        if self.is_busy() {
            return None;
        }
        self.battle.actor()
    }

    pub fn outcome(&self) -> Option<Outcome> {
        if self.is_busy() {
            return None;
        }
        self.battle.outcome()
    }

    pub fn offering(&self) -> Option<usize> {
        if self.is_busy() {
            return None;
        }
        self.battle.offer()
    }

    pub fn tick(&mut self, dt: f32) {
        self.wait -= dt;
        while self.wait <= 0.0 {
            let Some(beat) = self.queue.pop_front() else {
                self.wait = 0.0;
                self.settle();
                return;
            };
            self.wait += pause_secs(beat.pause);
            self.show(beat);
        }
    }

    fn show(&mut self, beat: Beat) {
        self.shown.thinking = match (beat.pause, &beat.line) {
            (Pause::Think, _) => beat.actor,
            (_, Some(_)) => None,
            _ => self.shown.thinking,
        };
        if let Some(line) = beat.line {
            self.shown.log.push(line);
        }
        self.shown.views = beat.views;
        self.shown.order = beat.order;
    }

    fn settle(&mut self) {
        self.shown.views = self.battle.views();
        self.shown.order = self.battle.turn_order(TURN_BAR_LENGTH);
        self.shown.thinking = None;
    }

    fn play(&mut self, input: Input) -> FightStep {
        let beats = play(&mut self.battle, &input);
        self.played.push(input);
        self.queue.extend(beats);
        self.menu = match self.battle.phase {
            Phase::Offer(_) => Menu::Offer(0),
            Phase::Running | Phase::Choosing(_) | Phase::Over(_) => Menu::Main(0),
        };
        self.tick(0.0);
        FightStep::Played
    }

    pub fn key(&mut self, action: &InputAction) -> FightStep {
        if matches!(action, InputAction::Cancel) && self.leaves_on_escape() {
            return FightStep::Close;
        }
        if self.is_busy() {
            return FightStep::Stay;
        }
        if self.battle.outcome().is_some() {
            return match action {
                InputAction::Confirm => FightStep::Done,
                _ => FightStep::Stay,
            };
        }
        if self.battle.offer().is_some() {
            return self.offer_key(action);
        }
        let Some(actor) = self.battle.actor() else {
            return FightStep::Stay;
        };
        match self.menu {
            Menu::Main(selected) => self.main_key(action, actor, selected),
            Menu::Skills(selected) => self.skills_key(action, actor, selected),
            Menu::Items(selected) => self.items_key(action, selected),
            Menu::Target { pending, target } => self.target_key(action, actor, pending, target),
            Menu::Offer(_) => {
                self.menu = Menu::Main(0);
                FightStep::Stay
            }
        }
    }

    fn leaves_on_escape(&self) -> bool {
        self.is_busy()
            || self.battle.outcome().is_some()
            || matches!(self.menu, Menu::Main(_) | Menu::Offer(_))
    }

    fn offer_key(&mut self, action: &InputAction) -> FightStep {
        let selected = match self.menu {
            Menu::Offer(selected) => selected,
            _ => 0,
        };
        match action {
            InputAction::Up | InputAction::Down => {
                self.menu = Menu::Offer(cycle(selected, Answer::ALL.len(), action));
                FightStep::Stay
            }
            InputAction::Confirm => self.play(Input::Answer(Answer::ALL[selected] == Answer::Sign)),
            _ => FightStep::Stay,
        }
    }

    fn main_key(&mut self, action: &InputAction, actor: usize, selected: usize) -> FightStep {
        match action {
            InputAction::Up | InputAction::Down => {
                self.menu = Menu::Main(cycle(selected, Command::ALL.len(), action));
                FightStep::Stay
            }
            InputAction::Confirm => self.command(actor, Command::ALL[selected]),
            _ => FightStep::Stay,
        }
    }

    fn command(&mut self, actor: usize, command: Command) -> FightStep {
        if !self.usable(actor, command) {
            return FightStep::Stay;
        }
        match command {
            Command::Bite => {
                if let Some(target) = self.targets(actor, Pending::Bite).first().copied() {
                    self.menu = Menu::Target {
                        pending: Pending::Bite,
                        target,
                    };
                }
                FightStep::Stay
            }
            Command::Skills => {
                self.menu = Menu::Skills(0);
                FightStep::Stay
            }
            Command::Items => {
                self.menu = Menu::Items(0);
                FightStep::Stay
            }
            Command::Guard => self.play(Input::Choose(Choice::Guard)),
            Command::Flee => self.play(Input::Choose(Choice::Flee)),
        }
    }

    fn skills_key(&mut self, action: &InputAction, actor: usize, selected: usize) -> FightStep {
        let count = self.battle.fighters[actor].skills.len();
        match action {
            InputAction::Up | InputAction::Down => {
                self.menu = Menu::Skills(cycle(selected, count, action));
                FightStep::Stay
            }
            InputAction::Cancel => {
                self.menu = Menu::Main(Command::Skills.index());
                FightStep::Stay
            }
            InputAction::Confirm => self.pick(actor, Pending::Skill(selected)),
            _ => FightStep::Stay,
        }
    }

    fn items_key(&mut self, action: &InputAction, selected: usize) -> FightStep {
        let packs = self.packed();
        match action {
            InputAction::Up | InputAction::Down => {
                self.menu = Menu::Items(cycle(selected, packs.len(), action));
                FightStep::Stay
            }
            InputAction::Cancel => {
                self.menu = Menu::Main(Command::Items.index());
                FightStep::Stay
            }
            InputAction::Confirm => match packs.get(selected) {
                Some(&pack) => match self.battle.actor() {
                    Some(actor) => self.pick(actor, Pending::Item(pack)),
                    None => FightStep::Stay,
                },
                None => FightStep::Stay,
            },
            _ => FightStep::Stay,
        }
    }

    fn pick(&mut self, actor: usize, pending: Pending) -> FightStep {
        if !self.ready(actor, pending) {
            return FightStep::Stay;
        }
        let targets = self.targets(actor, pending);
        match (self.needs_a_target(actor, pending), targets.first()) {
            (true, Some(&target)) => {
                self.menu = Menu::Target { pending, target };
                FightStep::Stay
            }
            (true, None) => FightStep::Stay,
            (false, _) => {
                let choice = self.choice(actor, pending, None);
                self.play(Input::Choose(choice))
            }
        }
    }

    fn target_key(
        &mut self,
        action: &InputAction,
        actor: usize,
        pending: Pending,
        target: usize,
    ) -> FightStep {
        let targets = self.targets(actor, pending);
        match action {
            InputAction::Left | InputAction::Right | InputAction::Up | InputAction::Down => {
                let at = targets.iter().position(|&t| t == target).unwrap_or(0);
                if let Some(&next) = targets.get(cycle(at, targets.len(), action)) {
                    self.menu = Menu::Target {
                        pending,
                        target: next,
                    };
                }
                FightStep::Stay
            }
            InputAction::Cancel => {
                self.menu = match pending {
                    Pending::Bite => Menu::Main(Command::Bite.index()),
                    Pending::Skill(skill) => Menu::Skills(skill),
                    Pending::Item(pack) => {
                        Menu::Items(self.packed().iter().position(|&p| p == pack).unwrap_or(0))
                    }
                };
                FightStep::Stay
            }
            InputAction::Confirm if targets.contains(&target) => {
                let choice = self.choice(actor, pending, Some(target));
                self.play(Input::Choose(choice))
            }
            _ => FightStep::Stay,
        }
    }

    fn choice(&self, actor: usize, pending: Pending, target: Option<usize>) -> Choice {
        match pending {
            Pending::Bite => Choice::Bite(target.unwrap_or(actor)),
            Pending::Skill(skill) => Choice::Skill { skill, target },
            Pending::Item(pack) => Choice::Item {
                pack,
                target: target.unwrap_or(actor),
            },
        }
    }

    pub fn packed(&self) -> Vec<usize> {
        (0..self.battle.items.len())
            .filter(|&pack| self.battle.items[pack].count > 0)
            .collect()
    }

    fn reach(&self, actor: usize, pending: Pending) -> Reach {
        match pending {
            Pending::Bite => Reach::OneFoe,
            Pending::Skill(skill) => self.battle.fighters[actor].skills[skill].reach,
            Pending::Item(pack) => match self.battle.items[pack].item.aim() {
                Aim::Ally => Reach::OneAlly,
                Aim::Foe => Reach::OneFoe,
                Aim::Nobody => Reach::Me,
            },
        }
    }

    fn needs_a_target(&self, actor: usize, pending: Pending) -> bool {
        let reach = self.reach(actor, pending);
        reach.picks_a_foe() || reach.picks_an_ally()
    }

    pub fn targets(&self, actor: usize, pending: Pending) -> Vec<usize> {
        let battle = &self.battle;
        let reach = self.reach(actor, pending);
        let side = battle.fighters[actor].side;
        if reach.picks_a_foe() {
            return battle
                .side(side.other())
                .filter(|&i| match reach {
                    Reach::BellyUpFoe => battle.fighters[i].belly_up && !battle.fighters[i].gone,
                    _ => battle.targetable(actor, i),
                })
                .collect();
        }
        battle
            .side(side)
            .filter(|&i| {
                let f = &battle.fighters[i];
                match reach {
                    Reach::BellyUpAlly => f.belly_up && !f.gone,
                    Reach::FallenAlly => f.gone || f.belly_up,
                    _ => !f.gone,
                }
            })
            .collect()
    }

    fn ready(&self, actor: usize, pending: Pending) -> bool {
        let target = match self.needs_a_target(actor, pending) {
            true => match self.targets(actor, pending).first() {
                Some(&target) => Some(target),
                None => return false,
            },
            false => None,
        };
        self.battle
            .can_choose(actor, self.choice(actor, pending, target))
    }

    pub fn usable(&self, actor: usize, command: Command) -> bool {
        match command {
            Command::Bite => self.ready(actor, Pending::Bite),
            Command::Skills => !self.battle.fighters[actor].skills.is_empty(),
            Command::Guard => self.battle.can_choose(actor, Choice::Guard),
            Command::Items => !self.packed().is_empty(),
            Command::Flee => self.battle.can_flee(),
        }
    }

    pub fn skill_ready(&self, actor: usize, skill: usize) -> bool {
        self.ready(actor, Pending::Skill(skill))
    }

    pub fn item_ready(&self, actor: usize, pack: usize) -> bool {
        self.ready(actor, Pending::Item(pack))
    }

    pub fn info(&self) -> Option<Info> {
        if self.is_busy() {
            return None;
        }
        if let Some(devil) = self.battle.offer() {
            return Some(Info::offer(&self.battle, devil));
        }
        let actor = self.battle.actor()?;
        let battle = &self.battle;
        Some(match self.menu {
            Menu::Main(selected) => match Command::ALL[selected] {
                Command::Bite => Info::bite(battle, actor, None),
                Command::Skills => Info::skills(battle, actor),
                Command::Guard => Info::guard(battle, actor),
                Command::Items => Info::items(battle),
                Command::Flee => Info::flee(battle),
            },
            Menu::Skills(selected) => {
                let skill = battle.fighters[actor].skills.get(selected)?;
                Info::skill(battle, actor, skill, None)
            }
            Menu::Items(selected) => {
                let pack = *self.packed().get(selected)?;
                Info::item(battle, &battle.items[pack], None)
            }
            Menu::Target { pending, target } => match pending {
                Pending::Bite => Info::bite(battle, actor, Some(target)),
                Pending::Skill(skill) => Info::skill(
                    battle,
                    actor,
                    &battle.fighters[actor].skills[skill],
                    Some(target),
                ),
                Pending::Item(pack) => Info::item(battle, &battle.items[pack], Some(target)),
            },
            Menu::Offer(_) => return None,
        })
    }

    pub fn targeted(&self) -> Option<usize> {
        match self.menu {
            Menu::Target { target, .. } if !self.is_busy() => Some(target),
            _ => None,
        }
    }

    pub fn side_of(&self, index: usize) -> Side {
        self.battle.fighters[index].side
    }
}

fn cycle(selected: usize, count: usize, action: &InputAction) -> usize {
    if count == 0 {
        return 0;
    }
    match action {
        InputAction::Up | InputAction::Left => (selected + count - 1) % count,
        _ => (selected + 1) % count,
    }
}

fn pause_secs(pause: Pause) -> f32 {
    match pause {
        Pause::Think => THINK_SECS,
        Pause::Result => BEAT_SECS,
        Pause::Round => ROUND_SECS,
        Pause::None => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    use super::*;
    use crate::dungeon::Difficulty;
    use crate::dungeon::battle::{BattleSetup, Stakes};
    use crate::dungeon::fighter::Scale;
    use crate::dungeon::floor;
    use crate::fishes::fish::Fish;
    use crate::fishes::species::FishSpecies;

    const SEED: u64 = 21;
    const SLICES: f32 = 4.0;
    const MAX_TICKS: usize = 10_000;

    fn opened() -> FightState {
        let mut rng = SmallRng::seed_from_u64(SEED);
        let party = vec![
            Fish::new(FishSpecies::Merluza, "Adam".to_string(), 0.0, 0.0, &mut rng),
            Fish::new(FishSpecies::Goldfish, "Eva".to_string(), 0.0, 0.0, &mut rng),
        ];
        let foes = floor::encounter(1, floor::scale_for(1), &[], &mut rng);
        let setup = BattleSetup {
            floor: 1,
            difficulty: Difficulty::Normal,
            stakes: Stakes::Encounter,
            purse: 0,
            items: Vec::new(),
            seed: SEED,
            ambushed: false,
            haunts: Vec::new(),
            scale: Scale::PLAIN,
            known: Vec::new(),
            every_graeae: false,
        };
        let mut battle = Battle::new(party, foes, setup);
        let beats = battle.begin();
        FightState::fresh(battle, beats, "The Shallows".to_string())
    }

    fn play_out(state: &mut FightState) -> (usize, bool) {
        let dt = BEAT_SECS / SLICES;
        let mut most = 0;
        let mut thought = false;
        for _ in 0..MAX_TICKS {
            if !state.is_busy() {
                break;
            }
            let before = state.shown.log.len();
            state.tick(dt);
            most = most.max(state.shown.log.len() - before);
            thought |= state.shown.thinking.is_some();
        }
        (most, thought)
    }

    #[test]
    fn each_line_arrives_a_beat_after_the_last() {
        let mut state = opened();
        let (most, _) = play_out(&mut state);
        assert!(most <= 1, "{most} lines arrived in one slice of a beat");
        assert!(
            state.actor().is_some(),
            "a fish chooses once the beats are shown"
        );
    }

    #[test]
    fn an_enemy_lights_up_while_it_chooses() {
        let mut state = opened();
        play_out(&mut state);
        let mut thought = false;
        for _ in 0..MAX_TICKS {
            if state.battle.outcome().is_some() {
                break;
            }
            state.key(&InputAction::Confirm);
            state.key(&InputAction::Confirm);
            let (_, lit) = play_out(&mut state);
            thought |= lit;
            if thought {
                break;
            }
        }
        assert!(thought);
    }

    #[test]
    fn nothing_can_be_chosen_while_the_beats_play() {
        let mut state = opened();
        assert!(state.is_busy());
        assert_eq!(state.key(&InputAction::Confirm), FightStep::Stay);
        assert!(state.played.is_empty());
    }

    #[test]
    fn escape_on_the_first_menu_leaves_and_inside_a_list_goes_back() {
        let mut state = opened();
        play_out(&mut state);
        state.menu = Menu::Skills(0);
        assert_eq!(state.key(&InputAction::Cancel), FightStep::Stay);
        assert_eq!(state.menu, Menu::Main(Command::Skills.index()));
        assert_eq!(state.key(&InputAction::Cancel), FightStep::Close);
    }
}
