use crossterm::event::Event;

use crate::fishes::toy::Slot;
use crate::ui::input_action::{InputAction, classify};
use crate::ui::toybox::{Mode, ToyEntry, ToyboxState};

use super::{App, Overlay};

impl App {
    pub(super) fn open_toybox(&mut self) -> bool {
        let state = ToyboxState::new(self.toy_entries());
        self.set_overlay(Overlay::Toybox(Box::new(state)));
        true
    }

    fn toy_entries(&self) -> Vec<ToyEntry> {
        let mut toys: Vec<ToyEntry> = self
            .tanks
            .iter()
            .flat_map(|tank| tank.fish.iter())
            .filter(|fish| fish.toy.is_some())
            .map(|fish| ToyEntry {
                name: fish.name.clone(),
                fish: fish.portrait(),
            })
            .collect();
        toys.sort_by(|a, b| a.name.cmp(&b.name));
        toys
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
        let Some((tank, index)) = self.fish_location(&name) else {
            return;
        };
        let Some(worn) = self.tanks[tank].fish[index]
            .toy
            .as_ref()
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
        let fish = &mut self.tanks[tank].fish[index];
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
