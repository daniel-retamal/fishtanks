use crate::cheats::{CHEAT_RESOURCE_AMOUNT, Cheat, DEBUG_MODE_LABEL, Switch};
use crate::commands::Clearance;
use crate::consumable::{ActiveMilkStatus, MilkStatus};
use crate::fishes::fish::Fish;
use crate::fishes::species::FishSpecies;
use crate::ledger::Flow;
use crate::loot::StockItem;
use crate::tank::Tank;
use crate::ui::text_input::TextInput;
use crate::void_ritual::GiveTarget;

use super::{App, Overlay};

const SPOON_BOY: &str = "Neo";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Launch {
    #[default]
    Player,
    Debug,
}

impl Launch {
    pub const DEBUG_FLAG: &'static str = "--debug";

    pub fn from_args(mut args: impl Iterator<Item = String>) -> Self {
        if args.any(|arg| arg == Self::DEBUG_FLAG) {
            return Launch::Debug;
        }
        Launch::Player
    }
}

impl App {
    pub fn clearance(&self) -> Clearance {
        if self.debug_mode {
            return Clearance::Debug;
        }
        if self.cheats.godmode {
            return Clearance::God;
        }
        Clearance::Player
    }

    pub(super) fn cleared_for(&self, needed: Clearance) -> bool {
        needed <= self.clearance()
    }

    pub(super) fn toggle_debug_mode(&mut self) -> bool {
        self.debug_mode = !self.debug_mode;
        true
    }

    pub(super) fn modes(&self) -> Vec<&'static str> {
        let debug = self.debug_mode.then_some(DEBUG_MODE_LABEL);
        debug
            .into_iter()
            .chain(self.unsaved_label())
            .chain(self.update_label())
            .chain(
                Switch::ALL
                    .iter()
                    .filter(|&&switch| self.is_switched_on(switch))
                    .filter_map(|switch| switch.status_label()),
            )
            .collect()
    }

    pub(super) fn open_cheat_popup(&mut self) -> bool {
        self.set_overlay(Overlay::Cheat(TextInput::new()));
        true
    }

    pub fn is_switched_on(&self, switch: Switch) -> bool {
        match switch {
            Switch::Godmode => self.cheats.godmode,
            Switch::Bottomless => self.purse.is_bottomless(),
            Switch::Boundless => self.cheats.boundless,
            Switch::NoEscape => self.cheats.no_escape,
        }
    }

    fn throw_switch(&mut self, switch: Switch, on: bool) {
        match switch {
            Switch::Godmode => self.cheats.godmode = on,
            Switch::Bottomless => self.purse.set_bottomless(on),
            Switch::Boundless => {
                self.cheats.boundless = on;
                for tank in &mut self.tanks {
                    tank.boundless = on;
                }
            }
            Switch::NoEscape => self.cheats.no_escape = on,
        }
    }

    pub fn enter_cheat(&mut self, cheat: Cheat) -> bool {
        let granted = match cheat.switch() {
            Some(switch) => {
                let on = !self.is_switched_on(switch);
                self.throw_switch(switch, on);
                on
            }
            None => self.grant(cheat),
        };
        if granted {
            self.mint_cheatfish(cheat.fish_name());
        }
        granted || cheat.switch().is_some()
    }

    pub(super) fn mint_cheatfish(&mut self, name: String) {
        if self.debug_mode {
            return;
        }
        let fish = Fish::new(
            FishSpecies::Cheatfish,
            String::new(),
            0.0,
            0.0,
            &mut rand::rng(),
        );
        self.land_or_wait(self.current_tank, fish, name);
    }

    fn grant(&mut self, cheat: Cheat) -> bool {
        let here = self.current_tank;
        match cheat {
            Cheat::ShowMeTheMoney => {
                self.earn(CHEAT_RESOURCE_AMOUNT, Flow::Godsend);
                true
            }
            Cheat::BreatheDeep => {
                self.food_supply = self.food_supply.saturating_add(CHEAT_RESOURCE_AMOUNT);
                true
            }
            Cheat::ThereIsNoCowLevel => {
                self.execute_give(GiveTarget::Cow(None), here, Flow::Godsend)
            }
            Cheat::StayingAlive => self.revive_latest(),
            Cheat::ThereIsNoSpoon => self.gift_fish(here, FishSpecies::Botfish, SPOON_BOY),
            Cheat::ModifyThePhaseVariance => {
                self.execute_give(GiveTarget::Item(StockItem::COMPUTER), here, Flow::Godsend)
            }
            Cheat::HighwayToHell => self.execute_give(
                GiveTarget::Item(StockItem::NECRONOMICON),
                here,
                Flow::Godsend,
            ),
            Cheat::Electrochemistry => {
                self.execute_give(GiveTarget::Item(StockItem::COFFEE), here, Flow::Godsend)
            }
            Cheat::Nothing => {
                self.execute_give(GiveTarget::Item(StockItem::VOID_SEED), here, Flow::Godsend)
            }
            Cheat::TheTruthIsOutThere => self.plan_abduction(here, &mut rand::rng()),
            Cheat::RadioFreeFishtank => self.mutate_everyone(here),
            Cheat::SomethingForNothing => {
                for &status in MilkStatus::ALL {
                    self.gain_status(status);
                }
                true
            }
            Cheat::HardcoreToTheMega => {
                let dancers = self.tanks[here]
                    .fish
                    .iter_mut()
                    .map(Fish::hurry_zoomie)
                    .filter(|&hurried| hurried)
                    .count();
                dancers > 0
            }
            Cheat::FoodForThought
            | Cheat::BendTheLight
            | Cheat::FishToTheFishtank
            | Cheat::PowerOverwhelming => false,
        }
    }

    fn revive_latest(&mut self) -> bool {
        let Some(name) = self.graveyard.last().map(|fish| fish.name.clone()) else {
            return false;
        };
        self.revive_fish(&name)
    }

    fn mutate_everyone(&mut self, tank_idx: usize) -> bool {
        let tank = &mut self.tanks[tank_idx];
        let names: Vec<String> = tank
            .fish
            .iter()
            .map(|fish| fish.name.clone())
            .chain(tank.cows.iter().map(|cow| cow.name.clone()))
            .collect();
        let mutated = names
            .iter()
            .map(|name| tank.apply_named_mutation(name, ""))
            .filter(|&mutated| mutated)
            .count();
        mutated > 0
    }

    pub(super) fn gain_status(&mut self, kind: MilkStatus) {
        self.active_statuses.push(ActiveMilkStatus::fresh(kind));
    }

    pub(super) fn found_tank(&mut self, mut tank: Tank) -> usize {
        tank.boundless = self.cheats.boundless;
        self.enrol(&mut tank);
        self.gather_the_dead(&mut tank);
        self.tanks.push(tank);
        self.tanks.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{Event, KeyCode, KeyEvent};

    use super::*;
    use crate::loot::ConsumableKind;
    use crate::tank::TankKind;

    fn fish_can_get_away(overwhelming: bool) -> bool {
        let mut app = App::new();
        if overwhelming {
            app.enter_cheat(Cheat::PowerOverwhelming);
        }
        app.editor.set("/fish".to_string());
        app.handle_input(Event::Key(KeyEvent::from(KeyCode::Enter)));
        let Some(state) = app.fishing_state() else {
            panic!("/fish opens the fishing overlay");
        };
        !state.no_escape
    }

    #[test]
    fn the_void_seed_is_fished_only_while_nothing_claims_the_voidtank() {
        let mut app = App::new();
        assert!(!app.withheld_loot().contains(&ConsumableKind::VoidSeed));
        app.enter_cheat(Cheat::Nothing);
        assert!(app.withheld_loot().contains(&ConsumableKind::VoidSeed));
        app.inventory.clear();
        assert!(app.execute_give(
            GiveTarget::Tank(TankKind::Void),
            app.current_tank,
            Flow::Godsend
        ));
        assert!(app.withheld_loot().contains(&ConsumableKind::VoidSeed));
        assert!(!app.withheld_loot().contains(&ConsumableKind::Computer));
    }

    #[test]
    fn power_overwhelming_means_no_fish_gets_away() {
        assert!(fish_can_get_away(false));
        assert!(!fish_can_get_away(true));
    }
}
