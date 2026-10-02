use crossterm::event::{Event, KeyCode};

use crate::fishes::botfish::BotfishState;
use crate::fishes::parts::KeyBinding;
use crate::ui::console::ConsoleState;
use crate::ui::hint_bar::HintBar;
use crate::ui::input_action::{hold, key_name};

use super::{App, Overlay};

impl App {
    pub(super) fn open_console(&mut self, name: &str) -> bool {
        let Some((_, fish)) = self.programmable_fish(name) else {
            return false;
        };
        if fish.script().is_none_or(|bot| bot.bindings().is_empty()) {
            return false;
        }
        let state = ConsoleState::new(fish.name.clone());
        self.set_overlay(Overlay::Console(state));
        true
    }

    pub fn console_open(&self) -> bool {
        matches!(self.active_overlay, Some(Overlay::Console(_)))
    }

    pub(super) fn console_names(&self) -> Vec<&str> {
        self.tanks
            .iter()
            .flat_map(|tank| tank.fish.iter())
            .filter(|fish| fish.script().is_some_and(|bot| !bot.bindings().is_empty()))
            .map(|fish| fish.name.as_str())
            .collect()
    }

    fn console_bot(&self, name: &str) -> Option<&BotfishState> {
        self.tanks
            .iter()
            .flat_map(|tank| tank.fish.iter())
            .find(|fish| fish.name == name)
            .and_then(|fish| fish.script())
    }

    fn console_bot_mut(&mut self, name: &str) -> Option<&mut BotfishState> {
        self.tanks
            .iter_mut()
            .flat_map(|tank| tank.fish.iter_mut())
            .find(|fish| fish.name == name)
            .and_then(|fish| fish.script_mut())
    }

    fn console_bindings(&self, name: &str) -> Vec<KeyBinding> {
        self.console_bot(name)
            .map(BotfishState::bindings)
            .unwrap_or_default()
    }

    pub(super) fn console_bar(&self) -> Option<HintBar> {
        let Some(Overlay::Console(state)) = &self.active_overlay else {
            return None;
        };
        Some(state.hints(&self.console_bindings(&state.fish)))
    }

    pub(super) fn handle_console_input(&mut self, event: Event) {
        let Some(held) = hold(&event) else {
            return;
        };
        if held.quits {
            self.running = false;
            return;
        }
        if held.down && held.code == KeyCode::Esc {
            self.close_overlay();
            return;
        }
        if held.down {
            let lifted = self.held_keys.press(held.code);
            self.lift_console_keys(lifted);
            self.console_key(held.code, true);
            return;
        }
        if self.held_keys.release(held.code) {
            self.console_key(held.code, false);
        }
    }

    pub(super) fn lift_console_keys(&mut self, lifted: Vec<KeyCode>) {
        for code in lifted {
            self.console_key(code, false);
        }
    }

    fn console_key(&mut self, code: KeyCode, down: bool) {
        let Some(key) = key_name(code) else {
            return;
        };
        let Some(Overlay::Console(state)) = &mut self.active_overlay else {
            return;
        };
        let changed = if down {
            state.press(&key)
        } else {
            state.release(&key)
        };
        if !changed {
            return;
        }
        let fish = state.fish.clone();
        if let Some(bot) = self.console_bot_mut(&fish) {
            bot.key(&key, down);
        }
    }

    pub(super) fn settle_console(&mut self) {
        let Some(Overlay::Console(state)) = &mut self.active_overlay else {
            return;
        };
        let lifted = state.settle();
        let fish = state.fish.clone();
        if self.console_bindings(&fish).is_empty() {
            self.close_overlay();
            return;
        }
        if let Some(bot) = self.console_bot_mut(&fish) {
            for key in lifted {
                bot.key(&key, false);
            }
        }
    }

    pub(super) fn leave_console(&mut self) {
        if !self.console_open() {
            return;
        }
        let Some(Overlay::Console(state)) = self.active_overlay.take() else {
            return;
        };
        let fish = state.fish.clone();
        let held = state.leave();
        if let Some(bot) = self.console_bot_mut(&fish) {
            for key in held {
                bot.key(&key, false);
            }
        }
    }
}
