use crate::ui::input_action::InputAction;
use crate::ui::layout::Scroll;
use crate::ui::text_input::TextInput;

use super::Difficulty;
use super::event;
use super::expedition::{Expedition, Ground, Place};
use super::muster::{Leg, Muster, MusterStep};
use super::room::{Act, Panel, Stage};
use super::state::{FightState, FightStep};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Popup {
    Items,
    Party,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Chart {
    pub selected: usize,
    pub popup: Option<Popup>,
    pub at: usize,
}

impl Chart {
    pub fn sheet(&self, expedition: &Expedition) -> Option<Panel> {
        Some(match self.popup? {
            Popup::Party => expedition.party_sheet(self.at),
            Popup::Items => expedition.item_sheet(self.at),
        })
    }

    fn open(&mut self, popup: Popup) {
        self.popup = Some(popup);
        self.at = 0;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lobby {
    pub selected: usize,
    pub fish: usize,
    pub reading: bool,
}

pub struct Words {
    pub line: &'static str,
    pub listener: &'static str,
    pub input: TextInput,
}

impl Words {
    fn of(line: &'static str, listener: &'static str) -> Words {
        Words {
            line,
            listener,
            input: TextInput::new(),
        }
    }

    pub fn ghost(&self) -> &'static str {
        let typed = self.input.as_str().to_uppercase();
        match self.line.to_uppercase().starts_with(&typed) {
            true => &self.line[typed.len().min(self.line.len())..],
            false => "",
        }
    }
}

pub const THE_EYE: &str = "the Eye";
pub const THE_VOID: &str = "the Void";

pub enum View {
    Muster(Muster),
    Map(Chart),
    Room(Lobby),
    Fight(Box<FightState>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Stay,
    Close,
    Played,
    FightOver,
    Go(u8),
    Act(Act, usize),
    SetOut(Difficulty),
    Pray(String),
}

pub struct DungeonState {
    pub view: View,
    pub scroll: Scroll,
    pub sheet_scroll: Scroll,
    pub page: Scroll,
    pub words: Option<Words>,
}

impl DungeonState {
    pub fn muster(muster: Muster) -> DungeonState {
        DungeonState {
            view: View::Muster(muster),
            scroll: Scroll::default(),
            sheet_scroll: Scroll::default(),
            page: Scroll::default(),
            words: None,
        }
    }

    pub fn of(expedition: &Expedition, fight: Option<FightState>) -> DungeonState {
        let view = match (expedition.place(), fight) {
            (Place::Fight, Some(fight)) => View::Fight(Box::new(fight)),
            (Place::Room, _) | (Place::Fight, None) => View::Room(Lobby::default()),
            (Place::Choosing, _) => View::Map(Chart::default()),
        };
        DungeonState {
            view,
            scroll: Scroll::default(),
            sheet_scroll: Scroll::default(),
            page: Scroll::default(),
            words: None,
        }
    }

    pub fn fight(&self) -> Option<&FightState> {
        match &self.view {
            View::Fight(fight) => Some(fight),
            _ => None,
        }
    }

    pub fn fight_mut(&mut self) -> Option<&mut FightState> {
        match &mut self.view {
            View::Fight(fight) => Some(fight),
            _ => None,
        }
    }

    pub fn is_busy(&self) -> bool {
        self.fight().is_some_and(FightState::is_busy)
    }

    pub fn wanted_words(&self, expedition: Option<&Expedition>) -> Option<&'static str> {
        match &self.view {
            View::Fight(fight) => fight.prayer(),
            View::Room(_) => {
                let visit = expedition?.visit.as_ref()?;
                let event = visit.event()?;
                event.asks_a_prayer()?;
                (visit.stage == Stage::Open).then(|| event::prayer_line(visit.seed))
            }
            View::Muster(_) | View::Map(_) => None,
        }
    }

    fn pray(&mut self, words: String) -> Step {
        match &mut self.view {
            View::Fight(fight) => match fight.pray(words) {
                FightStep::Played => Step::Played,
                FightStep::Stay | FightStep::Close | FightStep::Done | FightStep::Words(_) => {
                    Step::Stay
                }
            },
            View::Room(_) => Step::Pray(words),
            View::Muster(_) | View::Map(_) => Step::Stay,
        }
    }

    pub fn tick(&mut self, dt: f32) {
        if let Some(fight) = self.fight_mut() {
            fight.tick(dt);
        }
    }

    pub fn key(
        &mut self,
        action: &InputAction,
        expedition: Option<&Expedition>,
        ground: &Ground,
    ) -> Step {
        if self.words.is_some() {
            return self.words_key(action, expedition);
        }
        if let Some(turn) = self.page_turn(action, expedition) {
            self.page.nudge(turn);
            return Step::Stay;
        }
        self.page.reset();
        let step = self.view_key(action, expedition, ground);
        match step {
            Step::Act(Act::Pray, _) => {
                self.words = self
                    .wanted_words(expedition)
                    .map(|line| Words::of(line, THE_VOID));
                Step::Stay
            }
            step => step,
        }
    }

    fn page_turn(&self, action: &InputAction, expedition: Option<&Expedition>) -> Option<isize> {
        let turn = match action {
            InputAction::Left => -1,
            InputAction::Right => 1,
            _ => return None,
        };
        let paged = match &self.view {
            View::Muster(muster) => muster.leg == Leg::Party,
            View::Map(chart) => chart.popup == Some(Popup::Party) && expedition.is_some(),
            View::Room(_) | View::Fight(_) => false,
        };
        paged.then_some(turn)
    }

    fn words_key(&mut self, action: &InputAction, expedition: Option<&Expedition>) -> Step {
        if self.wanted_words(expedition).is_none() {
            self.words = None;
            return Step::Stay;
        }
        let Some(words) = self.words.as_mut() else {
            return Step::Stay;
        };
        match action {
            InputAction::Cancel => self.words = None,
            InputAction::Confirm if !words.input.as_str().trim().is_empty() => {
                let said = words.input.as_str().trim().to_string();
                self.words = None;
                return self.pray(said);
            }
            _ => words.input.handle_action(action),
        }
        Step::Stay
    }

    fn view_key(
        &mut self,
        action: &InputAction,
        expedition: Option<&Expedition>,
        ground: &Ground,
    ) -> Step {
        match &mut self.view {
            View::Muster(muster) => match muster.key(action) {
                MusterStep::Stay => Step::Stay,
                MusterStep::Close => Step::Close,
                MusterStep::SetOut(difficulty) => Step::SetOut(difficulty),
            },
            View::Fight(fight) => match fight.key(action) {
                FightStep::Stay => Step::Stay,
                FightStep::Close => Step::Close,
                FightStep::Played => Step::Played,
                FightStep::Done => Step::FightOver,
                FightStep::Words(line) => {
                    self.words = Some(Words::of(line, THE_EYE));
                    Step::Stay
                }
            },
            View::Map(chart) => {
                let Some(expedition) = expedition else {
                    return Step::Close;
                };
                map_key(chart, action, expedition)
            }
            View::Room(lobby) => {
                let Some(expedition) = expedition else {
                    return Step::Close;
                };
                let Some(panel) = expedition.panel(lobby.fish, &ground.above()) else {
                    return Step::Stay;
                };
                room_key(lobby, action, &panel, expedition.party.len())
            }
        }
    }
}

fn cycle(selected: usize, count: usize, back: bool) -> usize {
    if count == 0 {
        return 0;
    }
    match back {
        true => (selected + count - 1) % count,
        false => (selected + 1) % count,
    }
}

fn map_key(chart: &mut Chart, action: &InputAction, expedition: &Expedition) -> Step {
    if let Some(panel) = chart.sheet(expedition) {
        let rows = panel.rows.len();
        match action {
            InputAction::Up | InputAction::Left => chart.at = cycle(chart.at, rows, true),
            InputAction::Down | InputAction::Right => chart.at = cycle(chart.at, rows, false),
            InputAction::Char('i') if chart.popup != Some(Popup::Items) => chart.open(Popup::Items),
            InputAction::Char('p') if chart.popup != Some(Popup::Party) => chart.open(Popup::Party),
            InputAction::Char('i' | 'p') | InputAction::Cancel => chart.popup = None,
            _ => {}
        }
        return Step::Stay;
    }
    let ahead = expedition.ahead();
    match action {
        InputAction::Left | InputAction::Up => {
            chart.selected = cycle(chart.selected, ahead.len(), true);
            Step::Stay
        }
        InputAction::Right | InputAction::Down => {
            chart.selected = cycle(chart.selected, ahead.len(), false);
            Step::Stay
        }
        InputAction::Char('i') => {
            chart.open(Popup::Items);
            Step::Stay
        }
        InputAction::Char('p') => {
            chart.open(Popup::Party);
            Step::Stay
        }
        InputAction::Confirm => {
            match ahead.get(chart.selected.min(ahead.len().saturating_sub(1))) {
                Some(&index) => Step::Go(index),
                None => Step::Stay,
            }
        }
        InputAction::Cancel => Step::Close,
        _ => Step::Stay,
    }
}

fn room_key(lobby: &mut Lobby, action: &InputAction, panel: &Panel, party: usize) -> Step {
    let rows = panel.rows.len();
    lobby.selected = lobby.selected.min(rows.saturating_sub(1));
    match action {
        InputAction::Up => {
            lobby.selected = cycle(lobby.selected, rows, true);
            Step::Stay
        }
        InputAction::Down => {
            lobby.selected = cycle(lobby.selected, rows, false);
            Step::Stay
        }
        InputAction::Left | InputAction::Right if panel.which_fish => {
            lobby.fish = cycle(lobby.fish, party, matches!(action, InputAction::Left));
            Step::Stay
        }
        InputAction::Tab if panel.tab.is_some() => {
            lobby.reading = !lobby.reading;
            Step::Stay
        }
        InputAction::Confirm => match panel.rows.get(lobby.selected) {
            Some(row) if row.usable => Step::Act(row.act, lobby.fish),
            _ => Step::Stay,
        },
        InputAction::Cancel => Step::Close,
        _ => Step::Stay,
    }
}
