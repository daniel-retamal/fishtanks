use crossterm::event::Event;

use crate::casino::state::{CasinoState, Game, Leave};
use crate::casino::{Casino, Entrant, House, Teller};
use crate::economy::Money;
use crate::fishes::fish::Fish;
use crate::fishes::species::FishSpecies;
use crate::fishes::toy::{FittedPart, Shelf, ToyState};
use crate::ledger::Flow;
use crate::loot::{ConsumableKind, StockItem};
use crate::names;
use crate::ui::input_action::{InputAction, classify};

use super::{App, Overlay};

const PRIZE_PROBE: FishSpecies = FishSpecies::Toyfish;

pub(super) struct Lens<'a> {
    app: &'a App,
    busy: &'a [String],
}

impl Teller for Lens<'_> {
    fn spendable(&self) -> Money {
        self.app.purse.spendable()
    }

    fn entrants(&self) -> Vec<Entrant> {
        self.app
            .tanks
            .iter()
            .flat_map(|tank| tank.fish.iter())
            .map(|fish| Entrant {
                name: fish.name.clone(),
                worth: fish.sell_value(),
                portrait: fish.portrait(),
                stakeable: fish.is_sellable()
                    && fish.sell_value() > 0
                    && !fish.abduction_lock
                    && !self.busy.contains(&fish.name),
            })
            .collect()
    }

    fn room_for_copies(&self, names: &[String]) -> bool {
        let app = self.app;
        let mut room: Vec<usize> = app.tanks.iter().map(|tank| tank.room()).collect();
        for name in names {
            let Some((home, index)) = app.fish_location(name) else {
                return false;
            };
            let fish = &app.tanks[home].fish[index];
            let count = app.tanks.len();
            let seat = (0..count)
                .map(|step| (home + step) % count)
                .find(|&i| room[i] > 0 && app.tanks[i].welcomes(fish));
            match seat {
                Some(i) => room[i] -= 1,
                None => return false,
            }
        }
        true
    }

    fn room_for_a_prize(&self) -> bool {
        self.app.has_room_for_a_new(PRIZE_PROBE)
    }

    fn shelf(&self) -> Shelf {
        self.app.casino.toybox.shelf.clone()
    }
}

struct Cashier<'a> {
    app: &'a mut App,
    busy: Vec<String>,
}

impl Cashier<'_> {
    fn lens(&self) -> Lens<'_> {
        Lens {
            app: self.app,
            busy: &self.busy,
        }
    }
}

impl Teller for Cashier<'_> {
    fn spendable(&self) -> Money {
        self.lens().spendable()
    }

    fn entrants(&self) -> Vec<Entrant> {
        self.lens().entrants()
    }

    fn room_for_copies(&self, names: &[String]) -> bool {
        self.lens().room_for_copies(names)
    }

    fn room_for_a_prize(&self) -> bool {
        self.lens().room_for_a_prize()
    }

    fn shelf(&self) -> Shelf {
        self.lens().shelf()
    }
}

impl House for Cashier<'_> {
    fn wager(&mut self, amount: Money) -> bool {
        if !self.app.pay(amount, Flow::CasinoStakes) {
            return false;
        }
        let coffees = self.app.casino.comps_for(amount);
        if coffees > 0 {
            self.app
                .stock_up(StockItem::Consumable(ConsumableKind::Coffee), coffees);
        }
        true
    }

    fn pay_out(&mut self, amount: Money) {
        self.app.earn(amount, Flow::CasinoWinnings);
    }

    fn bury(&mut self, name: &str) {
        self.app.kill_fish(name);
    }

    fn copy(&mut self, name: &str) -> Option<String> {
        let (tank, index) = self.app.fish_location(name)?;
        let original = self.app.tanks[tank].fish[index].clone();
        let taken = self.app.all_fish_names();
        let copy_name = names::unique_name_in(&taken, &original.name);
        let to = self.app.land_fish(tank, original, copy_name).ok()?;
        self.app.tanks[to].fish.last().map(|fish| fish.name.clone())
    }

    fn casino(&mut self) -> &mut Casino {
        &mut self.app.casino
    }

    fn land(&mut self, fish: Fish, name: String) -> Option<String> {
        let to = self.app.land_fish(self.app.current_tank, fish, name).ok()?;
        self.app.tanks[to].fish.last().map(|fish| fish.name.clone())
    }

    fn shelve(&mut self, toy: &ToyState) {
        self.app.casino.toybox.shelf.shelve(toy);
    }

    fn stock_part(&mut self, part: FittedPart) {
        self.app.casino.toybox.add(part);
    }
}

impl App {
    fn all_fish_names(&self) -> std::collections::HashSet<String> {
        self.tanks
            .iter()
            .flat_map(|tank| tank.fish.iter().map(|fish| fish.name.clone()))
            .collect()
    }

    pub(super) fn open_casino(&mut self, game: Option<Game>) -> bool {
        let mut rng = rand::rng();
        let state = CasinoState::open(
            game,
            &Lens {
                app: self,
                busy: &[],
            },
            &mut rng,
        );
        self.set_overlay(Overlay::Casino(Box::new(state)));
        true
    }

    pub fn casino_state(&self) -> Option<&CasinoState> {
        match &self.active_overlay {
            Some(Overlay::Casino(state)) => Some(state),
            _ => None,
        }
    }

    pub fn casino_state_mut(&mut self) -> Option<&mut CasinoState> {
        match &mut self.active_overlay {
            Some(Overlay::Casino(state)) => Some(state),
            _ => None,
        }
    }

    fn take_casino(&mut self) -> Option<Box<CasinoState>> {
        match self.active_overlay.take() {
            Some(Overlay::Casino(state)) => Some(state),
            other => {
                self.active_overlay = other;
                None
            }
        }
    }

    pub(super) fn handle_casino_input(&mut self, event: Event) {
        let Some(action) = classify(&event) else {
            return;
        };
        if matches!(action, InputAction::Quit) {
            self.running = false;
            return;
        }
        let Some(mut state) = self.take_casino() else {
            return;
        };
        let busy = state.at_risk();
        let leave = {
            let mut cashier = Cashier { app: self, busy };
            state.key(&action, &mut cashier, &mut rand::rng())
        };
        match leave {
            Leave::Close => self.close_overlay(),
            Leave::Stay => self.active_overlay = Some(Overlay::Casino(state)),
        }
    }

    pub(super) fn tick_casino(&mut self) {
        let dt = 1.0 / self.settings.fps;
        let Some(mut state) = self.take_casino() else {
            return;
        };
        let busy = state.at_risk();
        {
            let mut cashier = Cashier { app: self, busy };
            state.tick(dt, &mut cashier, &mut rand::rng());
        }
        self.active_overlay = Some(Overlay::Casino(state));
    }

    pub(super) fn fish_on_the_table(&self) -> Vec<String> {
        self.casino_state()
            .map(CasinoState::at_risk)
            .unwrap_or_default()
    }

    pub(super) fn casino_lens<'a>(&'a self, busy: &'a [String]) -> Lens<'a> {
        Lens { app: self, busy }
    }

    pub(super) fn walk_away_from_the_table(&mut self, names: &[String]) {
        for name in names {
            self.kill_fish(name);
        }
    }
}
