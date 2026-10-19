use crossterm::event::Event;

use crate::fishes::fish::Fish;
use crate::fishes::toy::Slot;
use crate::ui::input_action::{InputAction, classify};
use crate::ui::toybox::{Mode, ToyEntry, ToyboxState};

use super::{App, Overlay};

pub(super) fn toys_in<'a>(fish: impl Iterator<Item = &'a Fish>) -> Vec<ToyEntry> {
    let mut toys: Vec<ToyEntry> = fish
        .filter(|fish| fish.toy.is_some())
        .map(|fish| ToyEntry {
            name: fish.name.clone(),
            fish: fish.portrait(),
        })
        .collect();
    toys.sort_by(|a, b| a.name.cmp(&b.name));
    toys
}

impl App {
    pub(super) fn open_toybox(&mut self) -> bool {
        let state = ToyboxState::new(toys_in(self.tanks.iter().flat_map(|tank| tank.fish.iter())));
        self.set_overlay(Overlay::Toybox(Box::new(state)));
        true
    }

    pub(super) fn handle_toybox_input(&mut self, event: Event) {
        let Some(action) = classify(&event) else {
            return;
        };
        if matches!(action, InputAction::Quit) {
            self.running = false;
            return;
        }
        let Some(Overlay::Toybox(state)) = self.active_overlay.as_mut() else {
            return;
        };
        let leave = matches!(action, InputAction::Cancel | InputAction::Char('q'));
        match state.mode {
            Mode::Toys => match action {
                InputAction::Up => state.step(false),
                InputAction::Down => state.step(true),
                InputAction::Confirm => {
                    state.begin_edit();
                }
                InputAction::Tab => state.mode = Mode::Shelf,
                _ if leave => self.close_overlay(),
                _ => {}
            },
            Mode::Shelf => match action {
                InputAction::Up => state.step(false),
                InputAction::Down => state.step(true),
                InputAction::Tab => state.mode = Mode::Toys,
                _ if leave => self.close_overlay(),
                _ => {}
            },
            Mode::Edit => match action {
                InputAction::Up => state.step(false),
                InputAction::Down => state.step(true),
                InputAction::Left => state.turn_part(&self.casino.toybox, false),
                InputAction::Right => state.turn_part(&self.casino.toybox, true),
                InputAction::Confirm => self.save_toy(),
                _ if leave => state.mode = Mode::Toys,
                _ => {}
            },
        }
    }

    fn toy_named(&mut self, name: &str) -> Option<&mut Fish> {
        if self.below.is_some() {
            return self
                .dungeon
                .expedition
                .as_mut()?
                .party
                .iter_mut()
                .find(|fish| fish.name == name);
        }
        let (tank, index) = self.fish_location(name)?;
        Some(&mut self.tanks[tank].fish[index])
    }

    fn save_toy(&mut self) {
        let Some(Overlay::Toybox(state)) = self.active_overlay.as_ref() else {
            return;
        };
        if state.sealed() {
            return;
        }
        let Some(name) = state.current().map(|entry| entry.name.clone()) else {
            return;
        };
        let draft = state.draft;
        let Some(worn) = self
            .toy_named(&name)
            .and_then(|fish| fish.toy.as_ref())
            .map(|toy| toy.fittings)
        else {
            return;
        };
        let swaps: Vec<Slot> = Slot::ALL
            .into_iter()
            .filter(|&slot| worn.get(slot) != draft.get(slot))
            .collect();
        let affordable = swaps.iter().all(|&slot| {
            draft
                .get(slot)
                .is_none_or(|part| self.casino.toybox.count(part) > 0)
        });
        if !affordable {
            return;
        }
        for slot in swaps {
            if let Some(part) = draft.get(slot) {
                self.casino.toybox.take(part);
            }
            if let Some(part) = worn.get(slot) {
                self.casino.toybox.add(part);
            }
        }
        let Some(fish) = self.toy_named(&name) else {
            return;
        };
        if let Some(toy) = fish.toy.as_mut() {
            toy.fittings = draft;
        }
        fish.fit_the_toy();
        let portrait = fish.portrait();
        if let Some(Overlay::Toybox(state)) = self.active_overlay.as_mut() {
            if let Some(entry) = state.toys.get_mut(state.selected) {
                entry.fish = portrait;
            }
            state.mode = Mode::Toys;
        }
    }
}
