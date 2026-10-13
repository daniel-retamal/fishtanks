use crate::ui::input_action::InputAction;
use crate::ui::layout::Scroll;

use super::Difficulty;
use super::expedition::{Expedition, Ground, Place};
use super::muster::{Muster, MusterStep};
use super::room::{Act, Panel};
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
}

pub struct DungeonState {
    pub view: View,
    pub scroll: Scroll,
    pub sheet_scroll: Scroll,
}

impl DungeonState {
    pub fn muster(muster: Muster) -> DungeonState {
        DungeonState {
            view: View::Muster(muster),
            scroll: Scroll::default(),
            sheet_scroll: Scroll::default(),
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
                let Some(panel) = expedition.panel(lobby.fish, &ground.wrecks) else {
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
