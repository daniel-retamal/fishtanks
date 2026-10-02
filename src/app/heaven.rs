use crate::fishes::fish::Fish;
use crate::tank::{Afterlife, Tank, WorldSignal};

use super::App;

impl App {
    pub(super) fn bury(&mut self, fish: Fish) {
        let wall = self.afterlife_tank(Afterlife::of(&fish));
        if let Some(tank) = wall {
            self.tanks[tank].receive_soul(fish.clone());
        }
        self.graveyard.push(fish);
    }

    fn afterlife_tank(&self, afterlife: Afterlife) -> Option<usize> {
        self.tanks
            .iter()
            .position(|tank| tank.kind.config().afterlife == Some(afterlife))
    }

    fn souls_bound_for(&self, afterlife: Afterlife) -> impl Iterator<Item = &Fish> {
        self.graveyard
            .iter()
            .filter(move |grave| Afterlife::of(grave) == afterlife)
    }

    pub(super) fn gather_the_dead(&self, tank: &mut Tank) {
        let Some(afterlife) = tank.kind.config().afterlife else {
            return;
        };
        if self.afterlife_tank(afterlife).is_some() {
            return;
        }
        for grave in self.souls_bound_for(afterlife) {
            tank.receive_soul(grave.clone());
        }
    }

    pub(super) fn hang_the_souls(&mut self) {
        for index in 0..self.tanks.len() {
            let Some(afterlife) = self.tanks[index].kind.config().afterlife else {
                continue;
            };
            if self.afterlife_tank(afterlife) != Some(index) {
                continue;
            }
            let souls: Vec<Fish> = self.souls_bound_for(afterlife).cloned().collect();
            for soul in souls {
                self.tanks[index].receive_soul(soul);
            }
        }
    }

    pub(super) fn demolish_tank(&mut self, index: usize) {
        let mut tank = self.tanks.remove(index);
        self.used_tank_names.remove(&tank.name);
        if self.current_tank >= index && self.current_tank > 0 {
            self.current_tank -= 1;
        }
        let waiting = std::mem::take(&mut tank.pending_arrivals);
        self.tanks[self.current_tank]
            .pending_arrivals
            .extend(waiting);
        let Some(afterlife) = tank.kind.config().afterlife else {
            return;
        };
        let Some(heir) = self.afterlife_tank(afterlife) else {
            return;
        };
        for soul in tank.take_souls() {
            self.tanks[heir].receive_soul(soul);
        }
    }

    pub(super) fn exhume(&mut self, grave: usize) -> Fish {
        let fish = self.graveyard.remove(grave);
        for tank in &mut self.tanks {
            tank.release_soul(&fish.name);
            tank.clear_grave_name(&fish.name);
        }
        fish
    }

    pub(super) fn kill_fish(&mut self, name: &str) -> bool {
        let Some((tank, index)) = self.fish_location(name) else {
            return false;
        };
        let fish = self.tanks[tank].take_fish(index);
        self.tanks[tank].signal(WorldSignal::Death);
        self.bury(fish);
        true
    }

    pub(super) fn can_sell_tank(&self, index: usize) -> bool {
        let tank = &self.tanks[index];
        let another_home = self
            .tanks
            .iter()
            .enumerate()
            .any(|(i, other)| i != index && !other.kind.config().holy_only);
        tank.kind.config().sellable && tank.fish.is_empty() && another_home
    }
}
