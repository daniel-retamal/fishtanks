use crossterm::event::Event;
use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};

use crate::dungeon::Difficulty;
use crate::dungeon::battle::{Battle, Stakes};
use crate::dungeon::expedition::{self, Expedition, FightRecord, MAX_PARTY};
use crate::dungeon::fighter::Side;
use crate::dungeon::floor::{self, FloorSpec};
use crate::dungeon::state::{DungeonState, Step};
use crate::dungeon::traits::Trait;
use crate::fishes::unfish::UnfishKind;
use crate::ledger::Flow;
use crate::ui::input_action::{InputAction, classify};

use super::{App, Overlay};

impl App {
    pub(super) fn open_dungeon(&mut self) -> bool {
        if self.dungeon.expedition.is_none() && !self.set_out() {
            return false;
        }
        let Some(state) = self.dungeon_state() else {
            return false;
        };
        self.set_overlay(Overlay::Dungeon(Box::new(state)));
        true
    }

    fn set_out(&mut self) -> bool {
        let tank = self.current_tank;
        let divers: Vec<usize> = (0..self.tanks[tank].fish.len())
            .filter(|&index| !self.tanks[tank].fish[index].abduction_lock)
            .take(MAX_PARTY)
            .collect();
        if divers.is_empty() {
            return false;
        }
        let mut party: Vec<_> = divers
            .iter()
            .rev()
            .map(|&index| self.tanks[tank].take_fish(index))
            .collect();
        party.reverse();
        let homes = vec![self.tanks[tank].name.clone(); party.len()];
        let mut expedition = Expedition::new(party, homes, Difficulty::Normal);
        let seed: u64 = rand::rng().random();
        let haunts = expedition::haunts(self.graveyard.iter());
        let foes = floor::encounter(
            expedition.floor,
            floor::scale_for(expedition.floor),
            &haunts,
            &mut SmallRng::seed_from_u64(seed),
        );
        let mut record = FightRecord::of(
            seed,
            Stakes::Encounter,
            self.purse.spendable(),
            &foes,
            haunts,
        );
        record.known = self.dungeon.remembered.clone();
        record.every_graeae = !self.graeae_at_home();
        expedition.fight = Some(record);
        self.dungeon.dives += 1;
        self.dungeon.expedition = Some(expedition);
        true
    }

    fn dungeon_state(&self) -> Option<DungeonState> {
        let expedition = self.dungeon.expedition.as_ref()?;
        let fresh = expedition
            .fight
            .as_ref()
            .is_some_and(|record| record.inputs.is_empty());
        let (battle, opening) = expedition.battle()?;
        let title = FloorSpec::title(expedition.floor);
        Some(match fresh {
            true => DungeonState::fresh(battle, opening, title),
            false => DungeonState::resumed(battle, title),
        })
    }

    pub fn dungeon_overlay(&self) -> Option<&DungeonState> {
        match &self.active_overlay {
            Some(Overlay::Dungeon(state)) => Some(state),
            _ => None,
        }
    }

    fn dungeon_overlay_mut(&mut self) -> Option<&mut DungeonState> {
        match &mut self.active_overlay {
            Some(Overlay::Dungeon(state)) => Some(state),
            _ => None,
        }
    }

    pub(super) fn handle_dungeon_input(&mut self, event: Event) {
        let Some(action) = classify(&event) else {
            return;
        };
        if matches!(action, InputAction::Quit) {
            self.running = false;
            return;
        }
        let Some(state) = self.dungeon_overlay_mut() else {
            return;
        };
        match state.key(&action) {
            Step::Stay => {}
            Step::Close => self.close_overlay(),
            Step::Played => self.keep_the_record(),
            Step::Done => self.come_home(),
        }
    }

    pub(super) fn tick_dungeon(&mut self) {
        let dt = 1.0 / self.settings.fps;
        if let Some(state) = self.dungeon_overlay_mut() {
            state.tick(dt);
        }
    }

    fn keep_the_record(&mut self) {
        let Some(state) = self.dungeon_overlay_mut() else {
            return;
        };
        let played = std::mem::take(&mut state.played);
        let spent = state.battle.purse_spent;
        let Some(record) = self
            .dungeon
            .expedition
            .as_mut()
            .and_then(|expedition| expedition.fight.as_mut())
        else {
            return;
        };
        record.inputs.extend(played);
        let due = spent.saturating_sub(record.spent);
        record.spent = spent;
        let due = due.min(self.purse.spendable());
        if due > 0 {
            self.pay(due, Flow::DungeonSpending);
        }
    }

    fn come_home(&mut self) {
        let Some(Overlay::Dungeon(state)) = self.active_overlay.take() else {
            return;
        };
        self.close_overlay();
        let Some(expedition) = self.dungeon.expedition.take() else {
            return;
        };
        self.remember(&state.battle);
        let aftermath = state.battle.finish();
        for (member, fish) in aftermath.party.into_iter().enumerate() {
            if aftermath.dead.contains(&member) {
                self.bury(fish);
                continue;
            }
            let name = fish.name.clone();
            let home = self.home_tank(expedition.home_of(member));
            self.land_or_wait(home, fish, name);
        }
    }

    fn graeae_at_home(&self) -> bool {
        self.tanks
            .iter()
            .flat_map(|tank| tank.fish.iter())
            .any(|fish| fish.unfish_kind() == Some(UnfishKind::Graeae))
    }

    fn remember(&mut self, battle: &Battle) {
        let stacked = battle
            .side(Side::Party)
            .any(|i| battle.fighters[i].has(Trait::CoreStack));
        if !stacked {
            return;
        }
        for foe in battle.side(Side::Foes) {
            let kind = &battle.fighters[foe].base;
            if !self.dungeon.remembered.contains(kind) {
                self.dungeon.remembered.push(kind.clone());
            }
        }
    }

    fn home_tank(&self, name: Option<&str>) -> usize {
        name.and_then(|name| self.tanks.iter().position(|tank| tank.name == name))
            .unwrap_or(self.current_tank)
    }
}
