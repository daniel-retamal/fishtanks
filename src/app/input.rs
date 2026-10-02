use std::collections::HashSet;

use crossterm::event::{Event, KeyCode, KeyEventKind};
use rand::RngExt;

use crate::{
    cheats::{Cheat, Switch},
    commands::{self, Clearance},
    consumable::{Caster, ConsumeTarget},
    economy::{self, Money, Sellable},
    entities::food,
    fishes::botfish::{BotfishState, DueCast, DueLine, Tackle},
    fishes::fish::Fish,
    fishes::parts::{RIG_WAIT_MAX_SECS, RIG_WAIT_MIN_SECS},
    fishes::species::FishSpecies,
    ledger::Flow,
    loot::{ConsumableKind, LootKind, MilkVariant, NameTarget, StockItem},
    names,
    settings::{FPS_MAX, FPS_MIN, STAGES_PER_TICK_MAX, STAGES_PER_TICK_MIN},
    tank::{
        Blueprint, FEED_PORTION, Fabrication, FabricationQuote, FabricationRefusal, Selector,
        StageBudget, Tank, TankKind, Workshop, WorldSignal, WorldView,
    },
    ui::{
        circuit_overlay::{CircuitFish, CircuitState},
        fields,
        fishing_overlay::FishingState,
        fishtanks_overlay::FishtanksState,
        foundry_overlay::FoundryState,
        index_overlay::IndexState,
        input_action::{InputAction, classify, hold},
        ledger_overlay::LedgerState,
        shop_overlay::{
            BuyCategory, BuyList, BuyListState, BuyTankPopup, Counter, FishNamePopup, Purchase,
            QtyPopup, QtyTarget, SellConfirm, SellEntry, SellMenuState, ShopPage, ShopState,
            buy_cat_first_available, buy_cat_next, tier_shelf,
        },
        show_overlay::{ShowSource, ShowState},
        wiring_panel::WiringPanelState,
    },
    void_ritual::{self, GiveTarget, VoidRitualState, WishAction},
};

use super::{App, Overlay};

const BLESSING_CASH: u32 = 333;
const BLESSING_FEED_AMOUNT: u32 = 33;
const BLESSING_FEED_TIMES: u32 = 10;

#[derive(Clone, Copy)]
enum Blessing {
    Restore,
    Clone,
    Cash,
    FeedBurst,
    SellBonus,
    TurnHoly,
    Revive,
    Grace,
    Gift,
    Expand,
}

impl Blessing {
    const ALL: &'static [Blessing] = &[
        Blessing::Restore,
        Blessing::Clone,
        Blessing::Cash,
        Blessing::FeedBurst,
        Blessing::SellBonus,
        Blessing::TurnHoly,
        Blessing::Revive,
        Blessing::Grace,
        Blessing::Gift,
        Blessing::Expand,
    ];
}

impl App {
    pub fn handle_input(&mut self, event: Event) {
        self.note_input(&event);
        self.held_keys.hear(&event);
        if matches!(event, Event::FocusLost) {
            let lifted = self.held_keys.let_go();
            self.lift_console_keys(lifted);
            self.hold_the_rod();
        }
        if let Event::Resize(w, h) = event {
            self.terminal_height = h;
            self.terminal_width = w;
            let bh = self.bar_height();
            let dead_names = self.graveyard_names();
            self.tanks[self.current_tank].resize(w, h.saturating_sub(bh), &dead_names);
            return;
        }
        if self.in_zen() {
            self.wake_from_zen(&event);
            return;
        }
        let is_key_press = matches!(&event, Event::Key(k) if k.kind == KeyEventKind::Press);
        if self.void_ritual.is_blocking() {
            self.handle_void_ritual_input(event);
        } else if matches!(self.active_overlay, Some(Overlay::Catch(_))) {
            self.handle_catch_input(event);
        } else if matches!(self.active_overlay, Some(Overlay::Naming { .. })) {
            self.handle_naming_input(event);
        } else if matches!(self.active_overlay, Some(Overlay::Junkfish(_))) {
            self.handle_junkfish_input(event);
        } else if matches!(self.active_overlay, Some(Overlay::Cheat(_))) {
            self.handle_cheat_input(event);
        } else if matches!(self.active_overlay, Some(Overlay::Notice(_))) {
            self.handle_notice_input(event);
        } else if matches!(self.active_overlay, Some(Overlay::Ledger(_))) {
            self.handle_ledger_input(event);
        } else if matches!(self.active_overlay, Some(Overlay::Fishing(_))) {
            self.handle_fishing_input(event);
        } else if matches!(self.active_overlay, Some(Overlay::Show { .. })) {
            self.handle_show_input(event);
        } else if matches!(self.active_overlay, Some(Overlay::Index(_))) {
            self.handle_index_input(event);
        } else if matches!(self.active_overlay, Some(Overlay::Inventory(_))) {
            self.handle_inventory_input(event);
        } else if matches!(self.active_overlay, Some(Overlay::Fishtanks(_))) {
            self.handle_fishtanks_input(event);
        } else if matches!(self.active_overlay, Some(Overlay::ConsumePicker(_))) {
            self.handle_consume_picker_input(event);
        } else if matches!(self.active_overlay, Some(Overlay::Shop(_))) {
            self.handle_shop_input(event);
        } else if matches!(self.active_overlay, Some(Overlay::Circuit(_))) {
            self.handle_circuit_input(event);
        } else if matches!(self.active_overlay, Some(Overlay::Wiring { .. })) {
            self.handle_wiring_input(event);
        } else if matches!(self.active_overlay, Some(Overlay::Foundry(_))) {
            self.handle_foundry_input(event);
        } else if matches!(self.active_overlay, Some(Overlay::Console(_))) {
            self.handle_console_input(event);
        } else {
            self.handle_command_input(event);
        }
        self.settle_arrivals();
        if is_key_press {
            self.reset_blink();
        }
    }

    fn handle_command_input(&mut self, event: Event) {
        let Some(action) = classify(&event) else {
            return;
        };
        match action {
            InputAction::Quit => {
                self.running = false;
            }
            InputAction::Confirm => {
                let fish_names_for_parse: Vec<&str> = self
                    .tanks
                    .iter()
                    .flat_map(|t| t.fish.iter().map(|f| f.name.as_str()))
                    .chain(
                        self.tanks
                            .iter()
                            .flat_map(|t| t.cows.iter().map(|c| c.name.as_str())),
                    )
                    .collect();
                let tank_names_for_parse: Vec<&str> =
                    self.tanks.iter().map(|t| t.name.as_str()).collect();
                let cmd_action = commands::parse(
                    &self.editor.text,
                    &fish_names_for_parse,
                    &tank_names_for_parse,
                );
                let text = self.editor.text.clone();
                if !text.trim().is_empty() {
                    self.history.record(text.clone());
                }
                let is_speech = matches!(cmd_action, commands::Action::Unknown);
                self.apply(cmd_action);
                if is_speech {
                    self.trigger_botfish(text.trim(), None);
                }
                self.editor.clear();
                self.history.reset();
            }
            InputAction::Up => {
                if let Some(text) = self.history.older(&self.editor.text) {
                    self.editor.set(text);
                }
            }
            InputAction::Down => {
                if let Some(text) = self.history.newer() {
                    self.editor.set(text);
                }
            }
            InputAction::Left => self.editor.left(),
            InputAction::Right => self.editor.right(),
            InputAction::Home => self.editor.home(),
            InputAction::End => self.editor.end(),
            InputAction::Backspace => self.editor.backspace(),
            InputAction::Delete => self.editor.delete(),
            InputAction::Tab => {
                let fish_names: Vec<&str> = self
                    .tank()
                    .fish
                    .iter()
                    .map(|f| f.name.as_str())
                    .chain(self.tank().cows.iter().map(|c| c.name.as_str()))
                    .collect();
                let consumable_name_strings = self.owned_consumable_names();
                let consumable_names: Vec<&str> =
                    consumable_name_strings.iter().map(String::as_str).collect();
                let tank_names: Vec<&str> = self.tanks.iter().map(|t| t.name.as_str()).collect();
                let fish_in_tanks: Vec<(&str, &str)> = self
                    .tanks
                    .iter()
                    .flat_map(|t| {
                        t.fish
                            .iter()
                            .map(move |f| (f.name.as_str(), t.name.as_str()))
                            .chain(
                                t.cows
                                    .iter()
                                    .map(move |c| (c.name.as_str(), t.name.as_str())),
                            )
                    })
                    .collect();
                let entity_mutations = self.entity_mutations();
                let entity_mutations_slice: Vec<(&str, Vec<crate::fishes::mutations::Mutation>)> =
                    entity_mutations
                        .iter()
                        .map(|(n, m)| (n.as_str(), m.clone()))
                        .collect();
                let graveyard_name_strings = self.graveyard_names();
                let graveyard_names: Vec<&str> =
                    graveyard_name_strings.iter().map(String::as_str).collect();
                let programmable_names: Vec<&str> = self
                    .tanks
                    .iter()
                    .flat_map(|t| t.fish.iter())
                    .filter(|f| f.is_programmable())
                    .map(|f| f.name.as_str())
                    .collect();
                let arrangeable_names: Vec<&str> = self
                    .tanks
                    .iter()
                    .flat_map(|t| t.fish.iter())
                    .filter(|f| f.is_arrangeable())
                    .map(|f| f.name.as_str())
                    .collect();
                let living_fish_names = self.living_fish_names();
                let sellable_fish_names = self.sellable_fish_names();
                let sellable_tank_names = self.sellable_tank_names();
                let sellable_stackable_name_strings = self.build_sellable_stackable_names();
                let blueprint_names = self.blueprint_names();
                let console_names = self.console_names();
                if let Some(new_input) = commands::tab_complete(
                    &self.editor.text,
                    &commands::CompletionCtx {
                        fish_names: &fish_names,
                        consumable_names: &consumable_names,
                        tank_names: &tank_names,
                        current_tank: self.tank().name.as_str(),
                        fish_in_tanks: &fish_in_tanks,
                        has_cow_in_current: self.tank().has_cow(),
                        entity_mutations: &entity_mutations_slice,
                        graveyard_names: &graveyard_names,
                        programmable_names: &programmable_names,
                        console_names: &console_names,
                        blueprint_names: &blueprint_names,
                        arrangeable_names: &arrangeable_names,
                        sellable_fish_names: &sellable_fish_names,
                        sellable_tank_names: &sellable_tank_names,
                        sellable_stackable_names: &sellable_stackable_name_strings,
                        living_fish_names: &living_fish_names,
                        clearance: self.clearance(),
                    },
                ) {
                    self.editor.set(new_input);
                }
            }
            InputAction::Char(c) => self.editor.insert(c),
            _ => {}
        }
    }

    fn handle_notice_input(&mut self, event: Event) {
        let Some(action) = classify(&event) else {
            return;
        };
        let Some(Overlay::Notice(state)) = &mut self.active_overlay else {
            return;
        };
        match action {
            InputAction::Quit => self.running = false,
            InputAction::Up => state.scroll_up(),
            InputAction::Down => state.scroll_down(),
            InputAction::Cancel | InputAction::Confirm | InputAction::Char('q') => {
                self.close_overlay()
            }
            _ => {}
        }
    }

    fn handle_ledger_input(&mut self, event: Event) {
        let Some(action) = classify(&event) else {
            return;
        };
        let Some(Overlay::Ledger(state)) = &mut self.active_overlay else {
            return;
        };
        match action {
            InputAction::Quit => self.running = false,
            InputAction::Up => state.scroll_up(),
            InputAction::Down => state.scroll_down(),
            InputAction::Cancel | InputAction::Char('q') => self.close_overlay(),
            _ => {}
        }
    }

    fn open_ledger(&mut self) -> bool {
        let state = LedgerState::new(&self.ledger, self.debug_mode);
        self.set_overlay(Overlay::Ledger(state));
        true
    }

    fn handle_show_input(&mut self, event: Event) {
        let Some(action) = classify(&event) else {
            return;
        };
        match action {
            InputAction::Quit => {
                self.running = false;
            }
            InputAction::Cancel | InputAction::Char('q') => match self.active_overlay.take() {
                Some(Overlay::Show {
                    state,
                    backed_index,
                }) => {
                    if matches!(state.source, ShowSource::FromIndex) {
                        self.active_overlay = backed_index.map(|b| Overlay::Index(*b));
                    }
                }
                other => self.active_overlay = other,
            },
            InputAction::Up => {
                if let Some(s) = self.show_state_mut() {
                    s.scroll_up();
                }
            }
            InputAction::Down => {
                if let Some(s) = self.show_state_mut() {
                    s.scroll_down();
                }
            }
            _ => {}
        }
    }

    fn handle_index_input(&mut self, event: Event) {
        let Some(action) = classify(&event) else {
            return;
        };
        match action {
            InputAction::Quit => {
                self.running = false;
            }
            InputAction::Cancel | InputAction::Char('q') => {
                self.close_overlay();
            }
            InputAction::Up => {
                if let Some(s) = self.index_state_mut() {
                    s.scroll_up();
                }
            }
            InputAction::Down => {
                if let Some(s) = self.index_state_mut() {
                    s.scroll_down();
                }
            }
            InputAction::Left => {
                if let Some(s) = self.index_state_mut() {
                    s.scroll_left();
                }
            }
            InputAction::Right => {
                if let Some(s) = self.index_state_mut() {
                    s.scroll_right();
                }
            }
            InputAction::Confirm => {
                let fish_name = self
                    .index_state()
                    .and_then(|s| s.selected_fish_name())
                    .map(|s| s.to_string());
                let tank_name = self
                    .index_state()
                    .map(|s| s.selected_tank_name().to_string())
                    .unwrap_or_default();
                if let Some(fish_name) = fish_name {
                    let visible = self
                        .tanks
                        .iter()
                        .flat_map(|t| t.fish.iter())
                        .find(|f| f.name == fish_name)
                        .is_some_and(|f| !f.is_invisible());
                    if !visible {
                        return;
                    }
                    let tank_kind = self
                        .tanks
                        .iter()
                        .find(|t| t.name == tank_name)
                        .map(|t| t.kind)
                        .unwrap_or(TankKind::Base);
                    let all_names: Vec<String> = self
                        .tanks
                        .iter()
                        .flat_map(|t| t.fish.iter().map(|f| f.name.clone()))
                        .collect();
                    let mut rng = rand::rng();
                    'populate: for tank in &mut self.tanks {
                        for fish in &mut tank.fish {
                            if fish.name == fish_name {
                                fields::populate_field_cache(fish, &all_names, &mut rng);
                                break 'populate;
                            }
                        }
                    }
                    let fish = self
                        .tanks
                        .iter()
                        .flat_map(|t| t.fish.iter())
                        .find(|f| f.name == fish_name)
                        .cloned();
                    if let Some(fish) = fish {
                        let backed_index = match self.active_overlay.take() {
                            Some(Overlay::Index(idx)) => Some(Box::new(idx)),
                            other => {
                                self.active_overlay = other;
                                None
                            }
                        };
                        self.active_overlay = Some(Overlay::Show {
                            state: ShowState::new(
                                &fish,
                                &tank_name,
                                tank_kind,
                                &all_names,
                                ShowSource::FromIndex,
                                false,
                                &mut rng,
                            ),
                            backed_index,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_fishing_input(&mut self, event: Event) {
        let Some(held) = hold(&event) else { return };
        if !held.down {
            self.held_keys.release(held.code);
            self.hold_the_rod();
            return;
        }
        if held.quits {
            self.running = false;
            return;
        }
        if matches!(held.code, KeyCode::Esc | KeyCode::Char('q')) {
            self.close_overlay();
            return;
        }
        self.held_keys.press(held.code);
        let strike = self.fishing_state().and_then(|s| {
            let striking =
                !s.game_over && !s.captured && s.is_catching() && held.code == KeyCode::Down;
            striking.then(|| s.is_biting())
        });
        match strike {
            Some(true) => self.hook_the_catch(),
            Some(false) => {
                self.close_overlay();
                return;
            }
            None => {}
        }
        self.hold_the_rod();
    }

    fn hook_the_catch(&mut self) {
        let catch = self.roll_catch(self.current_tank, &mut rand::rng());
        if let Some(s) = self.fishing_state_mut() {
            s.hook(catch);
        }
    }

    fn hold_the_rod(&mut self) {
        if let Some(Overlay::Fishing(s)) = &mut self.active_overlay {
            s.hold(&self.held_keys);
        }
    }

    fn handle_catch_input(&mut self, event: Event) {
        let Some(action) = classify(&event) else {
            return;
        };
        if matches!(action, InputAction::Quit) {
            self.running = false;
            return;
        }

        let is_fish = self.catch_state().is_some_and(|s| s.is_fish());

        if !is_fish {
            match action {
                InputAction::Confirm | InputAction::Cancel | InputAction::Char('q') => {
                    if let Some(state) = self.take_catch_state() {
                        self.apply_non_fish_loot(state.loot);
                    }
                }
                _ => {}
            }
            return;
        }

        match action {
            InputAction::Confirm => {
                if self.catch_state().is_some_and(|s| !s.name_input.is_empty())
                    && let Some(state) = self.take_catch_state()
                {
                    let name = names::title_case(state.name_input.as_str());
                    self.keep_catch(self.current_tank, state.fish.unwrap(), name);
                }
            }
            _ => {
                if let Some(s) = self.catch_state_mut() {
                    s.name_input.handle_action(&action);
                }
            }
        }
    }

    fn keep_catch(&mut self, tank_idx: usize, fish: Fish, name: String) {
        let landed = self.land_or_wait(tank_idx, fish, name).unwrap_or(tank_idx);
        self.tanks[landed].signal(WorldSignal::Catch);
    }

    fn land_catch(&mut self, tank_idx: usize, rng: &mut impl RngExt) {
        let Some(species) = self.roll_rig_catch(tank_idx, rng) else {
            return;
        };
        self.spend_a_cast(Caster::Rig);
        let fish = Fish::new(species, String::new(), 0.0, 0.0, rng);
        self.keep_catch(tank_idx, fish, species.display_name().to_string());
    }

    fn apply_non_fish_loot(&mut self, loot: LootKind) {
        match loot {
            LootKind::Cash(cv) => {
                self.earn(cv.amount(), Flow::CashCatches);
            }
            LootKind::Food(amount) => {
                self.food_supply += amount;
            }
            LootKind::Item(item) => {
                if let Some(stock) = StockItem::from_item(&item) {
                    self.stock_up(stock, 1);
                }
            }
            LootKind::Fish(_) => {}
        }
    }

    fn handle_inventory_input(&mut self, event: Event) {
        let Some(action) = classify(&event) else {
            return;
        };
        match action {
            InputAction::Quit => {
                self.running = false;
            }
            InputAction::Cancel | InputAction::Char('q') => {
                self.close_overlay();
            }
            InputAction::Up => {
                if let Some(s) = self.inventory_state_mut() {
                    s.scroll_up();
                }
            }
            InputAction::Down => {
                if let Some(s) = self.inventory_state_mut() {
                    s.scroll_down();
                }
            }
            InputAction::Confirm => {
                let selected_name = self
                    .inventory_state()
                    .and_then(|s| s.items.get(s.selected))
                    .map(|item| item.name.clone());
                if let Some(stock) =
                    selected_name.and_then(|name| StockItem::from_display_name(&name))
                {
                    self.try_consume_stock(
                        stock,
                        crate::ui::consume_picker::ConsumePickerSource::FromInventory,
                    );
                }
            }
            _ => {}
        }
    }

    fn reopen_inventory_on(&mut self, item_name: &str) {
        if let Some(mut state) = self.inventory_listing() {
            if let Some(pos) = state.items.iter().position(|i| i.name == item_name) {
                state.selected = pos;
            }
            self.set_overlay(Overlay::Inventory(state));
        }
    }

    fn reopen_inventory_on_first(&mut self) {
        if let Some(state) = self.inventory_listing() {
            self.set_overlay(Overlay::Inventory(state));
        }
    }

    fn build_picker_entries(
        &self,
        target: ConsumeTarget,
    ) -> Vec<crate::ui::consume_picker::ConsumePickerEntry> {
        let mut out = Vec::new();
        for (ti, tank) in self.tanks.iter().enumerate() {
            for (fi, fish) in tank.fish.iter().enumerate() {
                if !target.accepts(fish) {
                    continue;
                }
                out.push(crate::ui::consume_picker::ConsumePickerEntry {
                    fish_name: fish.name.clone(),
                    species_display: fish.species.display_name().to_string(),
                    tank_name: tank.name.clone(),
                    tank_idx: ti,
                    fish_idx: fi,
                });
            }
        }
        out
    }

    pub fn open_consume_picker(
        &mut self,
        target: ConsumeTarget,
        source: crate::ui::consume_picker::ConsumePickerSource,
    ) -> bool {
        let entries = self.build_picker_entries(target);
        if entries.is_empty() {
            return false;
        }
        let remaining = self.inventory.get(&target.stock()).copied().unwrap_or(0);
        let label = self.picker_label(target);
        self.active_overlay = crate::ui::consume_picker::ConsumePickerState::new(
            target, label, remaining, entries, source,
        )
        .map(Overlay::ConsumePicker);
        self.active_overlay.is_some()
    }

    fn picker_label(&self, target: ConsumeTarget) -> String {
        match target {
            ConsumeTarget::Etch(index) => self
                .blueprints
                .get(index)
                .map(|blueprint| blueprint.name.clone())
                .unwrap_or_default(),
            other => other.display_name().to_string(),
        }
    }

    pub fn drink_milk(&mut self, stock: StockItem, pick: crate::consumable::MilkStatus) {
        let entry = self.inventory.entry(stock).or_insert(0);
        *entry = entry.saturating_sub(1);
        self.inventory.retain(|_, v| *v > 0);
        let mut rng = rand::rng();
        self.gain_status(pick);
        let now_empty = if let Some(Overlay::Inventory(s)) = &mut self.active_overlay {
            s.update_from(&self.inventory, &self.blueprints, &mut rng);
            s.items.is_empty()
        } else {
            false
        };
        if now_empty {
            self.close_overlay();
        }
    }

    fn handle_consume_picker_input(&mut self, event: Event) {
        let Some(action) = classify(&event) else {
            return;
        };
        match action {
            InputAction::Quit => {
                self.running = false;
            }
            InputAction::Cancel | InputAction::Char('q') => {
                let restore = self
                    .consume_picker_state()
                    .map(|s| (s.source, s.target, s.item_name().to_string()));
                self.close_overlay();
                match restore {
                    Some((
                        crate::ui::consume_picker::ConsumePickerSource::FromInventory,
                        _,
                        item_name,
                    )) => self.reopen_inventory_on(&item_name),
                    Some((
                        crate::ui::consume_picker::ConsumePickerSource::FromFoundry,
                        ConsumeTarget::Etch(index),
                        _,
                    )) => self.reopen_foundry_on(index),
                    _ => {}
                }
            }
            InputAction::Up => {
                if let Some(s) = self.consume_picker_state_mut() {
                    s.scroll_up();
                }
            }
            InputAction::Down => {
                if let Some(s) = self.consume_picker_state_mut() {
                    s.scroll_down();
                }
            }
            InputAction::Confirm => {
                let (target, sel_entry) = {
                    let Some(s) = self.consume_picker_state() else {
                        return;
                    };
                    let Some(e) = s.entries.get(s.selected) else {
                        return;
                    };
                    (s.target, (e.tank_idx, e.fish_idx, e.fish_name.clone()))
                };
                let stock = target.stock();
                let (ti, _, fn_name) = sel_entry;
                if let ConsumeTarget::Etch(index) = target {
                    self.etch_from_picker(index, &fn_name);
                    return;
                }
                let mut rng = rand::rng();
                let Some(pos) = self.tanks[ti].fish.iter().position(|f| f.name == fn_name) else {
                    return;
                };
                if !target.apply_to(&mut self.tanks[ti].fish[pos], &mut rng) {
                    return;
                }
                let entry = self.inventory.entry(stock).or_insert(0);
                *entry = entry.saturating_sub(1);
                self.inventory.retain(|_, v| *v > 0);
                let remaining = self.inventory.get(&stock).copied().unwrap_or(0);
                if remaining == 0 {
                    let source = self.consume_picker_state().map(|s| s.source);
                    self.close_overlay();
                    if source == Some(crate::ui::consume_picker::ConsumePickerSource::FromInventory)
                    {
                        self.reopen_inventory_on_first();
                    }
                } else {
                    let new_entries = self.build_picker_entries(target);
                    let now_empty = if let Some(s) = self.consume_picker_state_mut() {
                        s.refresh_after_consume(remaining, || {
                            new_entries
                                .iter()
                                .map(|e| crate::ui::consume_picker::ConsumePickerEntry {
                                    fish_name: e.fish_name.clone(),
                                    species_display: e.species_display.clone(),
                                    tank_name: e.tank_name.clone(),
                                    tank_idx: e.tank_idx,
                                    fish_idx: e.fish_idx,
                                })
                                .collect()
                        });
                        s.entries.is_empty()
                    } else {
                        false
                    };
                    if now_empty {
                        self.close_overlay();
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_naming_input(&mut self, event: Event) {
        let Some(action) = classify(&event) else {
            return;
        };
        let kind = match &self.active_overlay {
            Some(Overlay::Naming { kind, .. }) => *kind,
            _ => return,
        };
        match action {
            InputAction::Quit => {
                self.running = false;
            }
            InputAction::Cancel => {
                self.close_overlay();
                self.reopen_inventory_on(kind.display_name());
            }
            InputAction::Confirm => {
                let Some(target) = kind.name_target() else {
                    return;
                };
                let Some(requested) = self
                    .naming_input()
                    .filter(|input| !input.is_empty())
                    .map(|input| names::title_case(input.as_str()))
                else {
                    return;
                };
                self.close_overlay();
                if !self.christen(target, &requested) {
                    return;
                }
                let entry = self
                    .inventory
                    .entry(StockItem::Consumable(kind))
                    .or_insert(0);
                *entry = entry.saturating_sub(1);
                self.inventory.retain(|_, v| *v > 0);
            }
            _ => {
                if let Some(input) = self.naming_input_mut() {
                    input.handle_action(&action);
                }
            }
        }
    }

    fn handle_junkfish_input(&mut self, event: Event) {
        let Some(action) = classify(&event) else {
            return;
        };
        let Some(Overlay::Junkfish(popup)) = &mut self.active_overlay else {
            return;
        };
        match action {
            InputAction::Quit => self.running = false,
            InputAction::Cancel => {
                self.close_overlay();
                self.reopen_inventory_on(StockItem::Junk.display_name());
            }
            InputAction::Confirm => {
                if popup.name_input.is_empty() {
                    return;
                }
                let name = names::title_case(popup.name_input.as_str());
                let Some(Overlay::Junkfish(popup)) = self.active_overlay.take() else {
                    return;
                };
                self.close_overlay();
                self.assemble_junkfish(popup.fish, name);
            }
            _ => popup.name_input.handle_action(&action),
        }
    }

    fn handle_cheat_input(&mut self, event: Event) {
        let Some(action) = classify(&event) else {
            return;
        };
        let Some(Overlay::Cheat(input)) = &mut self.active_overlay else {
            return;
        };
        match action {
            InputAction::Quit => self.running = false,
            InputAction::Cancel => self.close_overlay(),
            InputAction::Confirm => {
                let Some(cheat) = Cheat::parse(input.as_str()) else {
                    return;
                };
                self.close_overlay();
                self.enter_cheat(cheat);
            }
            _ => input.handle_action(&action),
        }
    }

    fn christen(&mut self, target: NameTarget, requested: &str) -> bool {
        match target {
            NameTarget::Tank(tank_kind) => {
                let actual_name = names::unique_name_in(&self.used_tank_names, requested);
                self.used_tank_names.insert(actual_name.clone());
                self.found_tank(Tank::new(actual_name, tank_kind, &[]));
                self.current_tank = self.tanks.len() - 1;
                true
            }
            NameTarget::Blueprint => self.capture_blueprint(requested),
        }
    }

    pub fn capture_blueprint(&mut self, requested: &str) -> bool {
        let taken: HashSet<String> = self.blueprints.iter().map(|b| b.name.clone()).collect();
        let name = names::unique_name_in(&taken, requested);
        let Some(blueprint) = Blueprint::capture(name, &self.tank().fish) else {
            return false;
        };
        self.blueprints.push(blueprint);
        true
    }

    pub(super) fn blueprint_names(&self) -> Vec<&str> {
        self.blueprints
            .iter()
            .map(|blueprint| blueprint.name.as_str())
            .collect()
    }

    fn workshop(&self) -> Workshop<'_> {
        Workshop {
            stock: &self.inventory,
            cash: self.purse.spendable(),
            room: self.tank().room(),
            hosts: self.etch_hosts().count(),
            connected: self.is_connected(),
        }
    }

    pub fn print_quote(
        &self,
        blueprint: &Blueprint,
    ) -> Result<FabricationQuote, FabricationRefusal> {
        blueprint.quote(Fabrication::Print, &self.workshop())
    }

    pub fn etch_quote(
        &self,
        blueprint: &Blueprint,
    ) -> Result<FabricationQuote, FabricationRefusal> {
        blueprint.quote(Fabrication::Etch, &self.workshop())
    }

    fn blueprint_index(&self, name: &str) -> Option<usize> {
        self.blueprints
            .iter()
            .position(|blueprint| blueprint.name.eq_ignore_ascii_case(name))
    }

    fn etch_hosts(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.tanks.iter().enumerate().flat_map(|(ti, tank)| {
            tank.fish
                .iter()
                .enumerate()
                .filter(|(_, fish)| ConsumeTarget::Etch(0).accepts(fish))
                .map(move |(fi, _)| (ti, fi))
        })
    }

    fn spend_on(&mut self, quote: &FabricationQuote) {
        self.take_stock(StockItem::Consumable(ConsumableKind::Fabricator), 1);
        for material in &quote.materials {
            self.take_stock(StockItem::Consumable(material.kind), material.from_stock());
        }
        self.pay(quote.cost, Flow::Fabrication);
    }

    pub fn print_blueprint(&mut self, name: &str) -> bool {
        let Some(index) = self.blueprint_index(name) else {
            return false;
        };
        let Ok(quote) = self.print_quote(&self.blueprints[index]) else {
            return false;
        };
        let current = self.current_tank;
        if !self.tanks[current].print(&self.blueprints[index], &mut rand::rng()) {
            return false;
        }
        self.spend_on(&quote);
        true
    }

    pub fn etch_blueprint(&mut self, name: &str, fish_name: &str) -> bool {
        let Some(index) = self.blueprint_index(name) else {
            return false;
        };
        let Some((ti, fi)) = self
            .etch_hosts()
            .find(|&(ti, fi)| self.tanks[ti].fish[fi].name.eq_ignore_ascii_case(fish_name))
        else {
            return false;
        };
        let Ok(quote) = self.etch_quote(&self.blueprints[index]) else {
            return false;
        };
        let host = &mut self.tanks[ti].fish[fi];
        let chip = self.blueprints[index].chip(&host.name);
        let Some(bot) = host.script_mut() else {
            return false;
        };
        let salvaged = bot.etch(chip);
        self.spend_on(&quote);
        for (part, count) in salvaged {
            *self
                .inventory
                .entry(StockItem::Consumable(ConsumableKind::Part(part)))
                .or_insert(0) += count;
        }
        true
    }

    fn etch_from_picker(&mut self, index: usize, fish_name: &str) {
        let Some(name) = self.blueprints.get(index).map(|b| b.name.clone()) else {
            return;
        };
        if self.etch_blueprint(&name, fish_name) {
            self.close_overlay();
        }
    }

    fn handle_wiring_input(&mut self, event: Event) {
        let Some(action) = classify(&event) else {
            return;
        };
        match action {
            InputAction::Quit => self.running = false,
            InputAction::Cancel => self.leave_wiring_panel(false),
            InputAction::Confirm => self.leave_wiring_panel(true),
            InputAction::Up => {
                if let Some(s) = self.wiring_state_mut() {
                    s.select_prev();
                }
            }
            InputAction::Down => {
                if let Some(s) = self.wiring_state_mut() {
                    s.select_next();
                }
            }
            _ => {
                if let Some(s) = self.wiring_state_mut() {
                    s.selected_input_mut().handle_action(&action);
                }
            }
        }
    }

    fn leave_wiring_panel(&mut self, save: bool) {
        let Some((state, backed_circuit)) = self.take_wiring_state() else {
            return;
        };
        if save && let Some(bot) = self.botfish_mut(state.tank_idx, &state.fish_name) {
            state.apply_to(bot);
        }
        let Some(backed_circuit) = backed_circuit else {
            return;
        };
        if !self.open_circuit_overlay(Some(&state.fish_name)) {
            return;
        }
        if let Some(circuit) = self.circuit_state_mut() {
            circuit.restore_view(&backed_circuit);
        }
    }

    fn handle_circuit_input(&mut self, event: Event) {
        let Some(action) = classify(&event) else {
            return;
        };
        match action {
            InputAction::Quit => self.running = false,
            InputAction::Cancel | InputAction::Char('q') => self.close_overlay(),
            InputAction::Up | InputAction::Left => {
                if let Some(s) = self.circuit_state_mut() {
                    s.scroll_up();
                }
            }
            InputAction::Down | InputAction::Right => {
                if let Some(s) = self.circuit_state_mut() {
                    s.scroll_down();
                }
            }
            InputAction::Tab => {
                if let Some(s) = self.circuit_state_mut() {
                    s.toggle_view();
                }
            }
            InputAction::Confirm => {
                let Some(name) = self
                    .circuit_state()
                    .map(|s| s.selected_fish_name().to_string())
                else {
                    return;
                };
                let Some(circuit) = self.take_circuit_state() else {
                    return;
                };
                if !self.open_wiring_panel(&name, Some(Box::new(circuit))) {
                    self.open_circuit_overlay(Some(&name));
                }
            }
            _ => {}
        }
    }

    fn handle_fishtanks_input(&mut self, event: Event) {
        let Some(action) = classify(&event) else {
            return;
        };
        match action {
            InputAction::Quit => {
                self.running = false;
            }
            InputAction::Cancel | InputAction::Char('q') => {
                self.close_overlay();
            }
            InputAction::Up => {
                if let Some(s) = self.fishtanks_state_mut() {
                    s.scroll_up();
                }
            }
            InputAction::Down => {
                if let Some(s) = self.fishtanks_state_mut() {
                    s.scroll_down();
                }
            }
            InputAction::Confirm => {
                let switch_to = match &self.active_overlay {
                    Some(Overlay::Fishtanks(s)) if s.selected != s.current_tank => Some(s.selected),
                    _ => None,
                };
                if let Some(idx) = switch_to {
                    self.current_tank = idx;
                }
                self.close_overlay();
            }
            _ => {}
        }
    }

    fn handle_shop_input(&mut self, event: Event) {
        let Some(action) = classify(&event) else {
            return;
        };
        if matches!(action, InputAction::Quit) {
            self.running = false;
            return;
        }

        let mut shop = match self.take_shop_state() {
            Some(s) => s,
            None => return,
        };

        let cash = self.purse.spendable();
        let access = self.shop_access();

        match shop.page {
            ShopPage::Main { ref mut selected } => {
                let counters = Counter::shelf(access);
                *selected = counters.settle(*selected);
                match action {
                    InputAction::Cancel | InputAction::Char('q') => {
                        return;
                    }
                    InputAction::Up => {
                        *selected = counters.step(*selected, false);
                    }
                    InputAction::Down => {
                        *selected = counters.step(*selected, true);
                    }
                    InputAction::Confirm if counters.is_available(*selected) => {
                        match Counter::ALL[*selected] {
                            Counter::Buy => {
                                shop.page = ShopPage::BuyCategory {
                                    selected: buy_cat_first_available(access),
                                    buy_popup: None,
                                };
                            }
                            Counter::Sell => {
                                if let Some(sm) = self.build_sell_menu_state() {
                                    shop.page = ShopPage::Sell(sm);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            ShopPage::BuyCategory {
                ref mut selected,
                ref mut buy_popup,
            } => {
                if let Some(popup) = buy_popup {
                    match action {
                        InputAction::Cancel | InputAction::Char('q') => {
                            *buy_popup = None;
                        }
                        InputAction::Left if popup.qty > 1 => {
                            popup.qty -= 1;
                        }
                        InputAction::Right if popup.qty < popup.max_qty => {
                            popup.qty += 1;
                        }
                        InputAction::Confirm => {
                            let bought = buy_popup.take().expect("the popup is open");
                            self.take_qty_purchase(&bought);
                        }
                        _ => {}
                    }
                } else {
                    match action {
                        InputAction::Cancel | InputAction::Char('q') => {
                            shop.page = ShopPage::Main {
                                selected: Counter::Buy.index(),
                            };
                        }
                        InputAction::Up => {
                            *selected = buy_cat_next(*selected, false, access);
                        }
                        InputAction::Down => {
                            *selected = buy_cat_next(*selected, true, access);
                        }
                        InputAction::Confirm
                            if BuyCategory::ALL[*selected].is_available(access) =>
                        {
                            match BuyCategory::ALL[*selected].purchase() {
                                Purchase::Browse(list @ BuyList::Fishes) => {
                                    shop.page = ShopPage::BuyFishList(Box::new(BuyListState::new(
                                        list, access,
                                    )));
                                }
                                Purchase::Browse(list @ BuyList::Tanks) => {
                                    shop.page =
                                        ShopPage::BuyTankList(BuyListState::new(list, access));
                                }
                                Purchase::Browse(list @ BuyList::Bench(_)) => {
                                    shop.page =
                                        ShopPage::BuyBenchList(BuyListState::new(list, access));
                                }
                                Purchase::Tiers => {
                                    shop.page = ShopPage::BuyBenchTiers {
                                        selected: tier_shelf(access).first(),
                                    };
                                }
                                Purchase::Counted { target, unit_price } => {
                                    *buy_popup = QtyPopup::new(target, unit_price, cash);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            ShopPage::BuyBenchTiers { ref mut selected } => {
                let tiers = crate::ui::shop_overlay::bench_tiers();
                let shelf = tier_shelf(access);
                *selected = shelf.settle(*selected);
                match action {
                    InputAction::Cancel | InputAction::Char('q') => {
                        shop.page = ShopPage::BuyCategory {
                            selected: BuyCategory::Robotics.index(),
                            buy_popup: None,
                        };
                    }
                    InputAction::Up => {
                        *selected = shelf.step(*selected, false);
                    }
                    InputAction::Down => {
                        *selected = shelf.step(*selected, true);
                    }
                    InputAction::Confirm if shelf.is_available(*selected) => {
                        if let Some(&tier) = tiers.get(*selected) {
                            shop.page = ShopPage::BuyBenchList(BuyListState::new(
                                BuyList::Bench(tier),
                                access,
                            ));
                        }
                    }
                    _ => {}
                }
                self.set_overlay(Overlay::Shop(shop));
                return;
            }

            ShopPage::BuyFishList(ref mut fl) => {
                let should_commit = fl.popup.as_ref().is_some_and(|p| {
                    matches!(action, InputAction::Confirm) && !p.name_input.is_empty()
                });

                if should_commit {
                    if let Some(FishNamePopup {
                        fish,
                        name_input,
                        catalog_idx,
                        ..
                    }) = fl.popup.take()
                    {
                        let price = FishSpecies::all_buyable()[catalog_idx].buy_price();
                        let name = names::title_case(name_input.as_str());
                        self.buy_fish(self.current_tank, fish, name, price);
                    }
                    self.set_overlay(Overlay::Shop(shop));
                    return;
                }

                if let Some(ref mut popup) = fl.popup {
                    match action {
                        InputAction::Cancel => {
                            fl.popup = None;
                        }
                        _ => {
                            popup.name_input.handle_action(&action);
                        }
                    }
                    self.set_overlay(Overlay::Shop(shop));
                    return;
                }

                fl.settle(access);
                match action {
                    InputAction::Cancel | InputAction::Char('q') => {
                        shop.page = ShopPage::BuyCategory {
                            selected: BuyCategory::Fishes.index(),
                            buy_popup: None,
                        };
                    }
                    InputAction::Up => {
                        fl.scroll_up(access);
                    }
                    InputAction::Down => {
                        fl.scroll_down(access);
                    }
                    InputAction::Confirm => {
                        fl.popup = fl.purchase(access);
                    }
                    _ => {}
                }
            }

            ShopPage::BuyTankList(ref mut tl) => {
                let should_commit = tl.popup.as_ref().is_some_and(|popup| {
                    matches!(action, InputAction::Confirm) && !popup.name_input.is_empty()
                });

                if should_commit {
                    if let Some(BuyTankPopup { kind, name_input }) = tl.popup.take() {
                        self.buy_tank_named(kind, &names::title_case(name_input.as_str()));
                    }
                    self.set_overlay(Overlay::Shop(shop));
                    return;
                }

                if let Some(ref mut popup) = tl.popup {
                    match action {
                        InputAction::Cancel => {
                            tl.popup = None;
                        }
                        _ => {
                            popup.name_input.handle_action(&action);
                        }
                    }
                    self.set_overlay(Overlay::Shop(shop));
                    return;
                }

                tl.settle(access);
                match action {
                    InputAction::Cancel | InputAction::Char('q') => {
                        shop.page = ShopPage::BuyCategory {
                            selected: BuyCategory::Fishtank.index(),
                            buy_popup: None,
                        };
                    }
                    InputAction::Up => {
                        tl.scroll_up(access);
                    }
                    InputAction::Down => {
                        tl.scroll_down(access);
                    }
                    InputAction::Confirm => {
                        tl.popup = tl.purchase(access);
                    }
                    _ => {}
                }
            }

            ShopPage::BuyBenchList(ref mut pl) => {
                if let Some(ref mut popup) = pl.popup {
                    match action {
                        InputAction::Cancel | InputAction::Char('q') => {
                            pl.popup = None;
                        }
                        InputAction::Left if popup.qty > 1 => {
                            popup.qty -= 1;
                        }
                        InputAction::Right if popup.qty < popup.max_qty => {
                            popup.qty += 1;
                        }
                        InputAction::Confirm => {
                            let bought = pl.popup.take().expect("the popup is open");
                            self.take_qty_purchase(&bought);
                        }
                        _ => {}
                    }
                    self.set_overlay(Overlay::Shop(shop));
                    return;
                }

                pl.settle(access);
                match action {
                    InputAction::Cancel | InputAction::Char('q') => {
                        shop.page = ShopPage::BuyBenchTiers {
                            selected: crate::ui::shop_overlay::bench_tiers()
                                .iter()
                                .position(|&tier| BuyList::Bench(tier) == pl.list())
                                .unwrap_or(0),
                        };
                    }
                    InputAction::Up => {
                        pl.scroll_up(access);
                    }
                    InputAction::Down => {
                        pl.scroll_down(access);
                    }
                    InputAction::Confirm => {
                        pl.popup = pl.purchase(access);
                    }
                    _ => {}
                }
            }

            ShopPage::Sell(ref mut sm) => {
                if sm.confirm.is_some() {
                    let max_qty = sm.items[sm.confirm.as_ref().unwrap().item_idx].max_qty();
                    match action {
                        InputAction::Cancel | InputAction::Char('q') => {
                            sm.confirm = None;
                        }
                        InputAction::Left => {
                            sm.confirm.as_mut().unwrap().qty_down();
                        }
                        InputAction::Right => {
                            sm.confirm.as_mut().unwrap().qty_up(max_qty);
                        }
                        InputAction::Confirm => {
                            let confirm = sm.confirm.take().unwrap();
                            let item = &sm.items[confirm.item_idx];
                            let earned = Money::from(confirm.sell_qty) * item.unit_price();
                            let flow = item.flow();
                            match item {
                                SellEntry::Fish { name, .. } => {
                                    let fish_name = name.clone();
                                    let mut sold = None;
                                    for tank in &mut self.tanks {
                                        if let Some(pos) =
                                            tank.fish.iter().position(|f| f.name == fish_name)
                                        {
                                            sold = Some(tank.take_fish(pos));
                                            tank.signal(WorldSignal::Sale);
                                            break;
                                        }
                                    }
                                    if let Some(fish) = sold {
                                        self.bury(fish);
                                    }
                                }
                                SellEntry::Junk { .. } => {
                                    let sell_qty = confirm.sell_qty;
                                    let qty = self.inventory.entry(StockItem::Junk).or_insert(0);
                                    *qty = qty.saturating_sub(sell_qty);
                                }
                                SellEntry::Coffee { .. } => {
                                    let sell_qty = confirm.sell_qty;
                                    let qty = self.inventory.entry(StockItem::COFFEE).or_insert(0);
                                    *qty = qty.saturating_sub(sell_qty);
                                }
                                SellEntry::Bait { .. } => {
                                    let sell_qty = confirm.sell_qty;
                                    let qty = self.inventory.entry(StockItem::BAIT).or_insert(0);
                                    *qty = qty.saturating_sub(sell_qty);
                                }
                                SellEntry::Milk { variant, .. } => {
                                    let sell_qty = confirm.sell_qty;
                                    let stock =
                                        StockItem::Consumable(ConsumableKind::Milk(*variant));
                                    let qty = self.inventory.entry(stock).or_insert(0);
                                    *qty = qty.saturating_sub(sell_qty);
                                }
                                SellEntry::Seed { kind, .. } | SellEntry::Robotics { kind, .. } => {
                                    let sell_qty = confirm.sell_qty;
                                    let qty = self
                                        .inventory
                                        .entry(StockItem::Consumable(*kind))
                                        .or_insert(0);
                                    *qty = qty.saturating_sub(sell_qty);
                                }
                                SellEntry::Blueprint { name } => {
                                    self.take_blueprint(name);
                                }
                                SellEntry::Tank { name, .. } => {
                                    let tank_name = name.clone();
                                    if let Some(pos) =
                                        self.tanks.iter().position(|t| t.name == tank_name)
                                    {
                                        self.demolish_tank(pos);
                                    }
                                }
                            }
                            self.inventory.retain(|_, v| *v > 0);
                            self.earn(earned, flow);

                            match self.build_sell_menu_state() {
                                Some(new_sm) => shop.page = ShopPage::Sell(new_sm),
                                None => {
                                    shop.page = ShopPage::Main {
                                        selected: Counter::Sell.index(),
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                } else {
                    match action {
                        InputAction::Cancel | InputAction::Char('q') => {
                            shop.page = ShopPage::Main {
                                selected: Counter::Sell.index(),
                            };
                        }
                        InputAction::Up => {
                            sm.scroll_up();
                        }
                        InputAction::Down => {
                            sm.scroll_down();
                        }
                        InputAction::Confirm => {
                            sm.confirm = Some(SellConfirm {
                                item_idx: sm.selected,
                                sell_qty: 1,
                            });
                        }
                        _ => {}
                    }
                }
            }
        }

        self.set_overlay(Overlay::Shop(shop));
    }

    pub(super) fn build_sell_menu_state(&self) -> Option<SellMenuState> {
        let fish: Vec<(String, FishSpecies, Money)> = self
            .tanks
            .iter()
            .flat_map(|t| {
                t.fish
                    .iter()
                    .filter(|f| f.is_sellable())
                    .map(|f| (f.name.clone(), f.species, f.sell_value()))
            })
            .collect();
        let sellable_tanks = self.sellable_tanks_with_price();
        let blueprint_names: Vec<String> = self.blueprints.iter().map(|b| b.name.clone()).collect();
        SellMenuState::new(&fish, &self.inventory, &sellable_tanks, &blueprint_names)
    }

    fn buy_fish(&mut self, from: usize, fish: Fish, name: String, price: u32) -> bool {
        if self.landing_tank(from, &fish).is_none() || !self.pay(price, Flow::Fish) {
            return false;
        }
        self.land_fish(from, fish, name).is_ok()
    }

    pub(super) fn sellable_tanks_with_price(&self) -> Vec<(String, u32)> {
        (0..self.tanks.len())
            .filter(|&index| self.can_sell_tank(index))
            .map(|index| &self.tanks[index])
            .map(|t| (t.name.clone(), t.kind.sell_price()))
            .collect()
    }

    pub(super) fn living_fish_names(&self) -> Vec<String> {
        self.tanks
            .iter()
            .flat_map(|t| t.fish.iter().map(|f| f.name.clone()))
            .collect()
    }

    pub(super) fn sellable_fish_names(&self) -> Vec<String> {
        self.tanks
            .iter()
            .flat_map(|t| t.fish.iter())
            .filter(|f| f.is_sellable())
            .map(|f| f.name.clone())
            .collect()
    }

    pub(super) fn sellable_tank_names(&self) -> Vec<String> {
        self.sellable_tanks_with_price()
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    }

    pub(super) fn build_sellable_stackable_names(&self) -> Vec<String> {
        let mut v = Vec::new();
        if self.inventory.get(&StockItem::Junk).copied().unwrap_or(0) > 0 {
            v.push(StockItem::Junk.display_name().to_string());
        }
        if self.inventory.get(&StockItem::COFFEE).copied().unwrap_or(0) > 0 {
            v.push(StockItem::COFFEE.display_name().to_string());
        }
        if self.inventory.get(&StockItem::BAIT).copied().unwrap_or(0) > 0 {
            v.push(StockItem::BAIT.display_name().to_string());
        }
        for &variant in MilkVariant::ALL {
            let stock = StockItem::Consumable(ConsumableKind::Milk(variant));
            if self.inventory.get(&stock).copied().unwrap_or(0) > 0 {
                v.push(variant.display_name().to_string());
            }
        }
        for kind in ConsumableKind::seeds()
            .into_iter()
            .chain(ConsumableKind::robotics_stock())
        {
            if self
                .inventory
                .get(&StockItem::Consumable(kind))
                .is_some_and(|&qty| qty > 0)
            {
                v.push(kind.display_name().to_string());
            }
        }
        v
    }

    fn handle_void_ritual_input(&mut self, event: Event) {
        if matches!(self.void_ritual, VoidRitualState::Wish { .. }) {
            if matches!(self.active_overlay, Some(Overlay::Index(_))) {
                self.handle_index_input(event);
                return;
            }
            if matches!(self.active_overlay, Some(Overlay::Show { .. })) {
                self.handle_show_input(event);
                return;
            }
            if matches!(self.active_overlay, Some(Overlay::Fishtanks(_))) {
                let before = self.current_tank;
                self.handle_fishtanks_input(event);
                if self.current_tank != before {
                    self.abort_void_ritual();
                }
                return;
            }
        }
        let Some(action) = classify(&event) else {
            return;
        };
        match action {
            InputAction::Char(c) => self.editor.append(c),
            InputAction::Backspace => self.editor.backspace_end(),
            InputAction::Confirm => self.submit_ritual_input(),
            _ => {}
        }
    }

    pub fn submit_ritual_input(&mut self) {
        let input = self.editor.text.clone();
        self.editor.clear();

        match self.void_ritual {
            VoidRitualState::Prayer {
                prayer_idx,
                phrase_idx,
                ..
            } => {
                let expected = void_ritual::PRAYERS[prayer_idx][phrase_idx];
                if void_ritual::phrase_matches(&input, expected) {
                    let phrases_len = void_ritual::PRAYERS[prayer_idx].len();
                    if phrase_idx + 1 < phrases_len {
                        self.void_ritual = VoidRitualState::Prayer {
                            prayer_idx,
                            phrase_idx: phrase_idx + 1,
                            timeout: void_ritual::PRAYER_TIMEOUT_SECS,
                        };
                    } else {
                        self.void_ritual = VoidRitualState::FinalPhrase {
                            timeout: void_ritual::PRAYER_TIMEOUT_SECS,
                        };
                    }
                } else {
                    self.abort_void_ritual();
                }
            }
            VoidRitualState::FinalPhrase { .. } => {
                if void_ritual::phrase_matches(&input, void_ritual::FINAL_PRAYER_PHRASE) {
                    self.next_prayer = (self.next_prayer + 1) % void_ritual::PRAYERS.len();
                    self.void_ritual = VoidRitualState::Wish {
                        retries_left: void_ritual::MAX_WISH_RETRIES,
                    };
                } else {
                    self.abort_void_ritual();
                }
            }
            VoidRitualState::Wish { retries_left } => {
                if input.starts_with('/') {
                    let fish_names_ref: Vec<&str> = self
                        .tanks
                        .iter()
                        .flat_map(|t| t.fish.iter().map(|f| f.name.as_str()))
                        .collect();
                    let tank_names_ref: Vec<&str> =
                        self.tanks.iter().map(|t| t.name.as_str()).collect();
                    let action = commands::parse(&input, &fish_names_ref, &tank_names_ref);
                    match action {
                        commands::Action::Index { .. }
                        | commands::Action::Show { .. }
                        | commands::Action::Fishtanks
                        | commands::Action::ToggleNames
                        | commands::Action::ToggleNets
                        | commands::Action::ToggleStats => {
                            self.apply(action);
                            return;
                        }
                        _ => {}
                    }
                }
                let fish_names: Vec<String> = self
                    .tanks
                    .iter()
                    .flat_map(|t| t.fish.iter().map(|f| f.name.clone()))
                    .collect();
                let tank_names: Vec<String> = self.tanks.iter().map(|t| t.name.clone()).collect();
                let graveyard_names: Vec<String> =
                    self.graveyard.iter().map(|f| f.name.clone()).collect();
                let cow_names: Vec<String> = self
                    .tanks
                    .iter()
                    .flat_map(|t| t.cows.iter().map(|c| c.name.clone()))
                    .collect();
                let ctx = void_ritual::WishCtx {
                    fish_names: &fish_names,
                    tank_names: &tank_names,
                    graveyard_names: &graveyard_names,
                    cow_names: &cow_names,
                };
                match void_ritual::parse_wish(input.trim(), &ctx) {
                    None => {
                        if retries_left <= 1 {
                            self.abort_void_ritual();
                        } else {
                            self.void_ritual = VoidRitualState::Wish {
                                retries_left: retries_left - 1,
                            };
                        }
                    }
                    Some(action) => {
                        self.execute_wish(action);
                        self.abort_void_ritual();
                    }
                }
            }
            VoidRitualState::Idle { .. } => {}
        }
    }

    fn execute_wish(&mut self, action: WishAction) {
        match action {
            WishAction::Give(target) => {
                self.execute_give(target, self.current_tank, Flow::Wishes);
            }
            WishAction::Mutate {
                fish_name,
                mutation,
            } => {
                if let Some(ti) = self.tanks.iter().position(|t| {
                    t.fish.iter().any(|f| f.name == fish_name)
                        || t.cows.iter().any(|c| c.name == fish_name)
                }) {
                    self.tanks[ti].apply_named_mutation(&fish_name, &mutation);
                }
            }
            WishAction::Revive { fish_name } => {
                self.revive_fish(&fish_name);
            }
            WishAction::Kill { fish_name } => {
                self.kill_fish(&fish_name);
            }
            WishAction::Clone { fish_name } => {
                self.clone_entity(&fish_name);
            }
            WishAction::Bless => self.perform_blessing(self.current_tank),
            WishAction::Expand { tank_name } => {
                self.expand_tank(&tank_name);
            }
            WishAction::Anything => {
                let mut rng = rand::rng();
                for _ in 0..2 {
                    let boon = match rng.random_range(0..4u32) {
                        0 => GiveTarget::Cash,
                        1 => GiveTarget::Food,
                        2 => GiveTarget::Item(StockItem::COFFEE),
                        _ => GiveTarget::Item(StockItem::BAIT),
                    };
                    self.execute_give(boon, self.current_tank, Flow::Wishes);
                }
            }
            WishAction::Nothing => {
                self.nothing_stacks += 1;
            }
            WishAction::Restore { name } => {
                self.restore_entity(&name);
            }
        }
    }

    pub(super) fn revive_fish(&mut self, fish_name: &str) -> bool {
        let Some(pos) = self
            .graveyard
            .iter()
            .position(|f| f.name.eq_ignore_ascii_case(fish_name))
        else {
            return false;
        };
        self.raise_from_grave(pos, self.current_tank)
    }

    fn raise_from_grave(&mut self, grave: usize, from: usize) -> bool {
        if self.landing_tank(from, &self.graveyard[grave]).is_none() {
            return false;
        }
        let fish = self.exhume(grave);
        let name = fish.name.clone();
        self.land_fish(from, fish, name).is_ok()
    }

    fn clone_entity(&mut self, name: &str) -> bool {
        let original_fish = self
            .tanks
            .iter()
            .flat_map(|t| t.fish.iter())
            .find(|f| f.name.eq_ignore_ascii_case(name))
            .cloned();
        if let Some(orig) = original_fish {
            let clone_name = format!("{}'s Clone", orig.name);
            return self.land_fish(self.current_tank, orig, clone_name).is_ok();
        }
        let original_cow = self
            .tanks
            .iter()
            .flat_map(|t| t.cows.iter())
            .find(|c| c.name.eq_ignore_ascii_case(name))
            .cloned();
        if let Some(mut orig) = original_cow {
            orig.name = format!("{}'s Clone", orig.name);
            let ct = self.current_tank;
            let mut rng = rand::rng();
            self.tanks[ct].place_cow(orig, &mut rng);
            return true;
        }
        false
    }

    pub(super) fn perform_blessing(&mut self, tank_idx: usize) {
        let mut rng = rand::rng();
        let blessing = Blessing::ALL[rng.random_range(0..Blessing::ALL.len())];
        self.apply_blessing(tank_idx, blessing, &mut rng);
    }

    fn apply_blessing(&mut self, tank_idx: usize, blessing: Blessing, rng: &mut impl RngExt) {
        match blessing {
            Blessing::Restore => self.bless_restore(tank_idx, rng),
            Blessing::Clone => self.bless_clone(tank_idx, rng),
            Blessing::Cash => self.earn(BLESSING_CASH, Flow::Blessings),
            Blessing::FeedBurst => self.bless_feed(tank_idx),
            Blessing::SellBonus => self.bless_sell_bonus(tank_idx, rng),
            Blessing::TurnHoly => self.bless_turn_holy(tank_idx, rng),
            Blessing::Revive => self.bless_revive(tank_idx, rng),
            Blessing::Grace => {
                self.cajetans_grace.stacks += 1;
                self.cajetans_grace.time_remaining = super::CAJETANS_GRACE_SECS;
            }
            Blessing::Gift => {
                let target = void_ritual::random_give_target(rng);
                self.execute_give(target, tank_idx, Flow::Blessings);
            }
            Blessing::Expand => self.tanks[tank_idx].expand(void_ritual::EXPAND_AMOUNT),
        }
    }

    fn bless_restore(&mut self, tank_idx: usize, rng: &mut impl RngExt) {
        use crate::restore::Restorable;
        let tank = &mut self.tanks[tank_idx];
        if tank.fish.is_empty() {
            return;
        }
        let idx = rng.random_range(0..tank.fish.len());
        tank.fish[idx].restore();
    }

    fn bless_clone(&mut self, tank_idx: usize, rng: &mut impl RngExt) {
        let fish_count = self.tanks[tank_idx].fish.len();
        let total = fish_count + self.tanks[tank_idx].cows.len();
        if total == 0 {
            return;
        }
        let pick = rng.random_range(0..total);
        if pick < fish_count {
            let orig = self.tanks[tank_idx].fish[pick].clone();
            let clone_name = format!("{}'s Clone", orig.name);
            let _ = self.land_fish(tank_idx, orig, clone_name);
        } else {
            let mut orig = self.tanks[tank_idx].cows[pick - fish_count].clone();
            orig.name = format!("{}'s Clone", orig.name);
            self.tanks[tank_idx].place_cow(orig, rng);
        }
    }

    fn bless_feed(&mut self, tank_idx: usize) {
        let mut free = BLESSING_FEED_AMOUNT * BLESSING_FEED_TIMES;
        for _ in 0..BLESSING_FEED_TIMES {
            self.tanks[tank_idx].feed(BLESSING_FEED_AMOUNT as usize, &mut free);
        }
    }

    fn bless_sell_bonus(&mut self, tank_idx: usize, rng: &mut impl RngExt) {
        let tank = &mut self.tanks[tank_idx];
        if tank.fish.is_empty() {
            return;
        }
        let idx = rng.random_range(0..tank.fish.len());
        let fish = &mut tank.fish[idx];
        fish.sell_price_bonus_pct = fish
            .sell_price_bonus_pct
            .saturating_add(crate::fishes::mutations::STRAWBERRY_SELL_BONUS_PCT);
    }

    fn bless_turn_holy(&mut self, tank_idx: usize, rng: &mut impl RngExt) {
        let candidates: Vec<usize> = self.tanks[tank_idx]
            .fish
            .iter()
            .enumerate()
            .filter(|(_, f)| {
                f.unfish_state.is_none()
                    && f.ability_stacks(FishSpecies::Holyfish) == 0
                    && f.is_sellable()
            })
            .map(|(i, _)| i)
            .collect();
        if candidates.is_empty() {
            return;
        }
        let idx = candidates[rng.random_range(0..candidates.len())];
        let fish = &self.tanks[tank_idx].fish[idx];
        let name = fish.name.clone();
        let (x, y) = (fish.position.x, fish.position.y);
        self.tanks[tank_idx].fish[idx] = Fish::new(FishSpecies::Holyfish, name, x, y, rng);
    }

    fn bless_revive(&mut self, tank_idx: usize, rng: &mut impl RngExt) {
        let candidates: Vec<usize> = self
            .graveyard
            .iter()
            .enumerate()
            .filter(|(_, f)| !f.devil_marked && self.landing_tank(tank_idx, f).is_some())
            .map(|(i, _)| i)
            .collect();
        if candidates.is_empty() {
            return;
        }
        let pos = candidates[rng.random_range(0..candidates.len())];
        self.raise_from_grave(pos, tank_idx);
    }

    fn expand_tank(&mut self, tank_name: &str) -> bool {
        if let Some(tank) = self
            .tanks
            .iter_mut()
            .find(|t| t.name.eq_ignore_ascii_case(tank_name))
        {
            tank.expand(void_ritual::EXPAND_AMOUNT);
            return true;
        }
        false
    }

    fn restore_entity(&mut self, name: &str) -> bool {
        use crate::restore::Restorable;
        for tank in &mut self.tanks {
            if let Some(fish) = tank
                .fish
                .iter_mut()
                .find(|f| f.name.eq_ignore_ascii_case(name))
            {
                fish.restore();
                return true;
            }
            if let Some(cow) = tank
                .cows
                .iter_mut()
                .find(|c| c.name.eq_ignore_ascii_case(name))
            {
                cow.restore();
                return true;
            }
        }
        false
    }

    pub(super) fn gift_fish(&mut self, tank_idx: usize, species: FishSpecies, name: &str) -> bool {
        let fish = Fish::new(species, String::new(), 0.0, 0.0, &mut rand::rng());
        self.land_fish(tank_idx, fish, name.to_string()).is_ok()
    }

    pub(super) fn execute_give(&mut self, target: GiveTarget, tank_idx: usize, flow: Flow) -> bool {
        match target {
            GiveTarget::Cash => {
                self.earn(void_ritual::GIVE_RESOURCE_AMOUNT, flow);
                true
            }
            GiveTarget::Food => {
                self.food_supply += void_ritual::GIVE_RESOURCE_AMOUNT;
                true
            }
            GiveTarget::Item(stock) => self.stock_up(stock, stock.gift_quantity()),
            GiveTarget::Fish(species) => self.gift_fish(tank_idx, species, species.display_name()),
            GiveTarget::NamedFish { species, name } => {
                self.gift_fish(tank_idx, species, &names::title_case(&name))
            }
            GiveTarget::Tank(kind) => {
                if !kind.config().sellable || self.claims(kind) {
                    return false;
                }
                let actual_name = names::unique_name_in(&self.used_tank_names, kind.display_name());
                self.used_tank_names.insert(actual_name.clone());
                let dead_names = self.graveyard_names();
                self.found_tank(Tank::new(actual_name, kind, &dead_names));
                true
            }
            GiveTarget::Cow(variant_opt) => {
                let mut rng = rand::rng();
                let variant =
                    variant_opt.unwrap_or_else(|| crate::entities::cow::random_cow_color(&mut rng));
                self.tanks[tank_idx].spawn_cow(variant, &mut rng);
                true
            }
        }
    }

    fn move_entity(&mut self, fish: &str, tank: &str) -> bool {
        let Some(target_idx) = self
            .tanks
            .iter()
            .position(|t| t.name.eq_ignore_ascii_case(tank))
        else {
            return false;
        };
        if let Some(source_idx) = (0..self.tanks.len())
            .filter(|&i| i != target_idx)
            .find(|&i| {
                self.tanks[i]
                    .fish
                    .iter()
                    .any(|f| f.name.eq_ignore_ascii_case(fish))
            })
        {
            if self.tanks[target_idx].is_full() {
                return false;
            }
            if self.tanks[target_idx]
                .fish
                .iter()
                .any(|f| f.name.eq_ignore_ascii_case(fish))
            {
                return false;
            }
            let fish_pos = self.tanks[source_idx]
                .fish
                .iter()
                .position(|f| f.name.eq_ignore_ascii_case(fish))
                .unwrap();
            if !self.tanks[target_idx].welcomes(&self.tanks[source_idx].fish[fish_pos]) {
                return false;
            }
            let fish_obj = self.tanks[source_idx].take_fish(fish_pos);
            let fish_name = fish_obj.name.clone();
            let mut rng = rand::rng();
            self.tanks[target_idx].place_fish(fish_obj, fish_name, &mut rng);
            return true;
        }
        let Some(source_idx) = (0..self.tanks.len())
            .filter(|&i| i != target_idx)
            .find(|&i| {
                self.tanks[i]
                    .cows
                    .iter()
                    .any(|c| c.name.eq_ignore_ascii_case(fish))
            })
        else {
            return false;
        };
        if !self.tanks[target_idx].welcomes_cows()
            || self.tanks[target_idx]
                .cows
                .iter()
                .any(|c| c.name.eq_ignore_ascii_case(fish))
        {
            return false;
        }
        let cow_pos = self.tanks[source_idx]
            .cows
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(fish))
            .unwrap();
        let cow_obj = self.tanks[source_idx].cows.remove(cow_pos);
        let cow_name = cow_obj.name.clone();
        self.tanks[source_idx].used_cow_names.remove(&cow_name);
        let mut rng = rand::rng();
        self.tanks[target_idx].place_cow(cow_obj, &mut rng);
        true
    }

    fn cowsay(&mut self, text: String) -> bool {
        let mut rng = rand::rng();
        let tank = &mut self.tanks[self.current_tank];
        for cow in &mut tank.cows {
            cow.speech = None;
        }
        let candidates: Vec<usize> = tank
            .cows
            .iter()
            .enumerate()
            .filter(|(_, c)| c.can_speak())
            .map(|(i, _)| i)
            .collect();
        if candidates.is_empty() {
            return false;
        }
        let idx = candidates[rng.random_range(0..candidates.len())];
        let cow = &mut tank.cows[idx];
        let speech = if cow.is_alienated() {
            crate::tank::ALIEN_TONGUE.to_string()
        } else {
            text
        };
        cow.say(speech.clone());
        let tank_idx = self.current_tank;
        self.trigger_botfish(&speech, Some(tank_idx));
        true
    }

    fn say(&mut self, speaker: Option<String>, text: String) -> bool {
        if text.trim().is_empty() {
            return false;
        }
        let tank_idx = self.current_tank;
        if let Some(name) = speaker {
            let Some(fish) = self.tanks[tank_idx]
                .fish
                .iter_mut()
                .find(|fish| fish.name == name)
            else {
                return false;
            };
            fish.say(text.clone());
        }
        self.trigger_botfish(&text, Some(tank_idx));
        true
    }

    pub(super) fn fish_location(&self, name: &str) -> Option<(usize, usize)> {
        self.tanks.iter().enumerate().find_map(|(ti, tank)| {
            tank.fish
                .iter()
                .position(|f| f.name.eq_ignore_ascii_case(name))
                .map(|fi| (ti, fi))
        })
    }

    fn programmable_location(&self, name: &str) -> Option<(usize, usize)> {
        let (ti, fi) = self.fish_location(name)?;
        self.tanks[ti].fish[fi]
            .is_programmable()
            .then_some((ti, fi))
    }

    fn set_frozen(&mut self, name: &str, frozen: bool) -> bool {
        let Some((ti, fi)) = self.programmable_location(name) else {
            return false;
        };
        let fish = &mut self.tanks[ti].fish[fi];
        if fish.frozen == frozen {
            return false;
        }
        fish.frozen = frozen;
        if frozen {
            fish.cancel_seek();
        }
        true
    }

    fn arrangeable_location(&self, name: &str) -> Option<(usize, usize)> {
        let (ti, fi) = self.fish_location(name)?;
        self.tanks[ti].fish[fi].is_arrangeable().then_some((ti, fi))
    }

    fn nudge_fish(&mut self, name: &str, dx: i32, dy: i32) -> bool {
        let Some((ti, fi)) = self.arrangeable_location(name) else {
            return false;
        };
        if dx == 0 && dy == 0 {
            return false;
        }
        let (width, height) = (self.tanks[ti].width, self.tanks[ti].height);
        self.tanks[ti].fish[fi].nudge(dx, dy, width, height);
        true
    }

    fn flip_fish(&mut self, name: &str) -> bool {
        let Some((ti, fi)) = self.arrangeable_location(name) else {
            return false;
        };
        self.tanks[ti].fish[fi].flip();
        true
    }

    pub(super) fn programmable_fish(&self, name: &str) -> Option<(usize, &Fish)> {
        self.tanks.iter().enumerate().find_map(|(ti, tank)| {
            tank.fish
                .iter()
                .find(|f| f.is_programmable() && f.name.eq_ignore_ascii_case(name))
                .map(|fish| (ti, fish))
        })
    }

    fn open_wiring_panel(&mut self, name: &str, backed_circuit: Option<Box<CircuitState>>) -> bool {
        let panel = self.programmable_fish(name).and_then(|(ti, fish)| {
            Some(WiringPanelState::new(ti, fish.name.clone(), fish.script()?))
        });
        let Some(state) = panel else {
            return false;
        };
        self.set_overlay(Overlay::Wiring {
            state,
            backed_circuit,
        });
        true
    }

    pub(super) fn open_foundry(&mut self) -> bool {
        let Some(state) = FoundryState::new(&self.blueprints) else {
            return false;
        };
        self.set_overlay(Overlay::Foundry(state));
        true
    }

    fn reopen_foundry_on(&mut self, index: usize) {
        let Some(mut state) = FoundryState::new(&self.blueprints) else {
            return;
        };
        state.selected = index.min(self.blueprints.len() - 1);
        self.set_overlay(Overlay::Foundry(state));
    }

    fn handle_foundry_input(&mut self, event: Event) {
        let Some(action) = classify(&event) else {
            return;
        };
        let Some(Overlay::Foundry(state)) = &mut self.active_overlay else {
            return;
        };
        if let InputAction::Quit = action {
            self.running = false;
            return;
        }
        if state.is_editing() {
            match action {
                InputAction::Cancel => state.cancel_edit(),
                InputAction::Confirm => {
                    let exports = state.take_edit().unwrap_or_default();
                    if let Some(blueprint) = self.blueprints.get_mut(state.selected) {
                        blueprint.export(&exports);
                    }
                }
                other => {
                    if let Some(input) = state.editor_mut() {
                        input.handle_action(&other);
                    }
                }
            }
            return;
        }
        match action {
            InputAction::Cancel | InputAction::Char('q') => self.close_overlay(),
            InputAction::Up => state.scroll_up(),
            InputAction::Down => state.scroll_down(),
            InputAction::Left => state.pan(false),
            InputAction::Right => state.pan(true),
            InputAction::Tab => state.toggle_focus(),
            InputAction::Char('x') | InputAction::Char('X') => {
                if let Some(blueprint) = self.blueprints.get(state.selected) {
                    state.start_editing(blueprint);
                }
            }
            InputAction::Char('p') | InputAction::Char('P') => {
                let selected = self
                    .blueprints
                    .get(state.selected)
                    .map(|blueprint| blueprint.name.clone());
                if let Some(name) = selected
                    && self.print_blueprint(&name)
                {
                    self.close_overlay();
                }
            }
            InputAction::Char('e') | InputAction::Char('E') => {
                let selected = state.selected;
                let ready = self
                    .blueprints
                    .get(selected)
                    .is_some_and(|blueprint| self.etch_quote(blueprint).is_ok());
                if ready {
                    self.open_consume_picker(
                        ConsumeTarget::Etch(selected),
                        crate::ui::consume_picker::ConsumePickerSource::FromFoundry,
                    );
                }
            }
            _ => {}
        }
    }

    fn open_circuit_overlay(&mut self, select: Option<&str>) -> bool {
        let wired: Vec<CircuitFish> = self
            .tank()
            .fish
            .iter()
            .filter(|f| f.is_wired())
            .filter_map(CircuitFish::of)
            .collect();
        let Some(mut state) = CircuitState::new(self.current_tank, wired) else {
            return false;
        };
        if let Some(name) = select {
            state.select(name);
        }
        self.set_overlay(Overlay::Circuit(state));
        true
    }

    pub(super) fn trigger_botfish(&mut self, speech: &str, only_tank: Option<usize>) {
        if speech.is_empty() {
            return;
        }
        for (ti, tank) in self.tanks.iter_mut().enumerate() {
            if only_tank.is_some_and(|t| t != ti) {
                continue;
            }
            for fish in &mut tank.fish {
                if let Some(bot) = fish.script_mut() {
                    bot.hear(speech);
                }
            }
        }
    }

    fn set_clock(&mut self, stages: u32) -> bool {
        if !(STAGES_PER_TICK_MIN..=STAGES_PER_TICK_MAX).contains(&stages) {
            return false;
        }
        self.settings.stages_per_tick = stages;
        true
    }

    fn stages_requested(&self) -> u32 {
        self.settings
            .stages_per_tick
            .saturating_mul(1 + self.coffee_stacks())
    }

    pub fn tick_fabric(&mut self) -> u32 {
        let requested = self.stages_requested();
        let budget = StageBudget::new();
        let cash = economy::reading(self.purse.spendable());
        let hour = self.day_clock.hour();
        let bait = self.inventory.get(&StockItem::BAIT).copied().unwrap_or(0);
        let mut worlds: Vec<WorldView> = self
            .tanks
            .iter_mut()
            .map(|tank| tank.observe(cash, hour).with_bait(bait))
            .collect();
        let mut advanced = 0;
        while advanced < requested {
            Tank::advance_together(&mut self.tanks, &mut worlds);
            advanced += 1;
            if budget.spent() {
                break;
            }
        }
        advanced
    }

    pub(super) fn tick_botfish(&mut self) {
        let coffee = self.coffee_stacks();
        let mut due: Vec<(usize, String, DueLine)> = Vec::new();
        for (ti, tank) in self.tanks.iter_mut().enumerate() {
            for fish in &mut tank.fish {
                let Some(bot) = fish.script_mut() else {
                    continue;
                };
                for line in bot.due_lines(coffee) {
                    due.push((ti, fish.name.clone(), line));
                }
            }
        }
        for (tank_idx, fish_name, line) in due {
            let success = self.run_bot_command(tank_idx, &fish_name, &line.command);
            if let Some(bot) = self.botfish_mut(tank_idx, &fish_name) {
                bot.report_line(&line.path, success);
            }
        }
    }

    pub(super) fn tick_casts(&mut self) {
        let dt = 1.0 / self.settings.fps;
        let mut due: Vec<(usize, String, DueCast)> = Vec::new();
        for (ti, tank) in self.tanks.iter_mut().enumerate() {
            for fish in &mut tank.fish {
                let Some(bot) = fish.script_mut() else {
                    continue;
                };
                for cast in bot.due_casts(dt) {
                    due.push((ti, fish.name.clone(), cast));
                }
            }
        }
        let mut rng = rand::rng();
        for (tank_idx, fish_name, cast) in due {
            if !self.has_room_for_a_catch(tank_idx) {
                continue;
            }
            match cast.tackle {
                Tackle::Bait => {
                    let baited = self.inventory.contains_key(&StockItem::BAIT);
                    if baited {
                        self.take_stock(StockItem::BAIT, 1);
                    }
                    let wait = rng.random_range(RIG_WAIT_MIN_SECS..=RIG_WAIT_MAX_SECS);
                    let Some(bot) = self.botfish_mut(tank_idx, &fish_name) else {
                        continue;
                    };
                    if baited {
                        bot.lower_line(&cast.path, wait);
                    } else {
                        bot.reel_in(&cast.path, false);
                    }
                }
                Tackle::Landed => {
                    self.land_catch(tank_idx, &mut rng);
                    if let Some(bot) = self.botfish_mut(tank_idx, &fish_name) {
                        bot.reel_in(&cast.path, true);
                    }
                }
            }
        }
    }

    fn botfish_mut(&mut self, tank_idx: usize, fish_name: &str) -> Option<&mut BotfishState> {
        self.tanks
            .get_mut(tank_idx)?
            .fish
            .iter_mut()
            .find(|f| f.name == fish_name)
            .and_then(|f| f.script_mut())
    }

    fn run_bot_command(&mut self, tank_idx: usize, speaker: &str, command: &str) -> bool {
        if tank_idx >= self.tanks.len() {
            return false;
        }
        let Some(command) = Selector::expand(command, &self.tanks[tank_idx].shoal()) else {
            return false;
        };
        let fish_names: Vec<&str> = self
            .tanks
            .iter()
            .flat_map(|t| {
                t.fish
                    .iter()
                    .map(|f| f.name.as_str())
                    .chain(t.cows.iter().map(|c| c.name.as_str()))
            })
            .collect();
        let tank_names: Vec<&str> = self.tanks.iter().map(|t| t.name.as_str()).collect();
        let action = commands::parse(&command, &fish_names, &tank_names);
        if !bot_action_allowed(&action) {
            return false;
        }
        if let commands::Action::Buy(target) = action {
            return self.bot_buy(tank_idx, target);
        }
        let previous_tank = self.current_tank;
        self.current_tank = tank_idx;
        let success = match action {
            commands::Action::Say(text) => self.say(Some(speaker.to_string()), text),
            other => self.apply(other),
        };
        self.current_tank = previous_tank.min(self.tanks.len().saturating_sub(1));
        success
    }

    fn buy_tank_named(&mut self, kind: TankKind, base_name: &str) -> bool {
        if !self.pay(kind.buy_price(), Flow::Tanks) {
            return false;
        }
        let actual_name = names::unique_name_in(&self.used_tank_names, base_name);
        self.used_tank_names.insert(actual_name.clone());
        let dead_names = self.graveyard_names();
        self.current_tank = self.found_tank(Tank::new(actual_name, kind, &dead_names));
        true
    }

    fn take_qty_purchase(&mut self, popup: &QtyPopup) {
        if !self.pay(popup.cost(), popup.target.flow()) {
            return;
        }
        match popup.target {
            QtyTarget::Stock(item) => *self.inventory.entry(item).or_insert(0) += popup.qty,
            QtyTarget::Food => self.food_supply += popup.qty,
        }
    }

    fn bot_buy(&mut self, tank_idx: usize, target: commands::BuyTarget) -> bool {
        match target {
            commands::BuyTarget::Fish(species) => {
                if !species.config().buyable {
                    return false;
                }
                let fish = Fish::new(species, String::new(), 0.0, 0.0, &mut rand::rng());
                let name = species.display_name().to_string();
                self.buy_fish(tank_idx, fish, name, species.buy_price())
            }
            commands::BuyTarget::Tank(kind) => {
                kind.config().buyable && self.buy_tank_named(kind, kind.display_name())
            }
            other => self.execute_buy(other),
        }
    }

    fn apply(&mut self, action: commands::Action) -> bool {
        let needed = action.clearance();
        if !self.cleared_for(needed) {
            return false;
        }
        let performed = self.perform(action);
        if performed && needed == Clearance::God {
            self.mint_cheatfish(Cheat::of_switch(Switch::Godmode).fish_name());
        }
        performed
    }

    fn perform(&mut self, action: commands::Action) -> bool {
        match action {
            commands::Action::Feed => {
                let ct = self.current_tank;
                self.tanks[ct].feed(FEED_PORTION, &mut self.food_supply)
            }
            commands::Action::SetFps(fps) => {
                self.settings.fps = fps.clamp(FPS_MIN, FPS_MAX);
                true
            }
            commands::Action::SetClock(stages) => self.set_clock(stages),
            commands::Action::ToggleNames => {
                self.settings.show_names = !self.settings.show_names;
                true
            }
            commands::Action::ToggleNets => {
                self.settings.show_nets = !self.settings.show_nets;
                true
            }
            commands::Action::Zen => self.enter_zen(),
            commands::Action::ToggleStats => {
                self.settings.show_stats = !self.settings.show_stats;
                let bh = self.bar_height();
                let dead_names = self.graveyard_names();
                self.tanks[self.current_tank].resize(
                    self.terminal_width,
                    self.terminal_height.saturating_sub(bh),
                    &dead_names,
                );
                true
            }
            commands::Action::ModResource { name, delta } => match name.to_lowercase().as_str() {
                "cash" => {
                    match u32::try_from(delta) {
                        Ok(gain) => self.earn(gain, Flow::Godsend),
                        Err(_) => {
                            let owed = Money::from(delta.unsigned_abs()).min(self.purse.balance());
                            self.pay(owed, Flow::Godsend);
                        }
                    }
                    true
                }
                "food" => {
                    self.food_supply = (self.food_supply as i64 + delta as i64).max(0) as u32;
                    true
                }
                _ => {
                    let Some(stock) = StockItem::from_display_name(&name) else {
                        return false;
                    };
                    let current = self.inventory.get(&stock).copied().unwrap_or(0);
                    let room = self.room_for(stock);
                    if delta > 0 && room == 0 {
                        return false;
                    }
                    let new_val = ((current as i64 + delta as i64).max(0) as u32)
                        .min(current.saturating_add(room));
                    if new_val == 0 {
                        self.inventory.remove(&stock);
                    } else {
                        self.inventory.insert(stock, new_val);
                    }
                    true
                }
            },
            commands::Action::Spawn(species, name) => {
                let mut rng = rand::rng();
                self.tank_mut()
                    .spawn_fish(species, names::title_case(&name), &mut rng)
            }
            commands::Action::Mutate(fish_name, mutation_name) => {
                let ct = self.current_tank;
                self.tanks[ct].apply_named_mutation(&fish_name, &mutation_name)
            }
            commands::Action::Give(target) => {
                self.execute_give(target, self.current_tank, Flow::Godsend)
            }
            commands::Action::Revive(name) => self.revive_fish(&name),
            commands::Action::Kill(name) => self.kill_fish(&name),
            commands::Action::Clone(name) => self.clone_entity(&name),
            commands::Action::Bless => {
                self.perform_blessing(self.current_tank);
                true
            }
            commands::Action::Expand(name) => self.expand_tank(&name),
            commands::Action::Restore(name) => self.restore_entity(&name),
            commands::Action::Program(name) => self.open_wiring_panel(&name, None),
            commands::Action::Console(name) => self.open_console(&name),
            commands::Action::Circuit => self.open_circuit_overlay(None),
            commands::Action::Foundry => self.open_foundry(),
            commands::Action::Print(name) => self.print_blueprint(&name),
            commands::Action::Etch { blueprint, fish } => self.etch_blueprint(&blueprint, &fish),
            commands::Action::SetFrozen { name, frozen } => self.set_frozen(&name, frozen),
            commands::Action::Nudge { name, dx, dy } => self.nudge_fish(&name, dx, dy),
            commands::Action::Flip(name) => self.flip_fish(&name),
            commands::Action::Index { all, tank_filter } => {
                let all_names: Vec<String> = self
                    .tanks
                    .iter()
                    .flat_map(|t| t.fish.iter().map(|f| f.name.clone()))
                    .collect();
                let mut rng = rand::rng();
                if let Some(filter) = tank_filter {
                    let tank_idx = self
                        .tanks
                        .iter()
                        .position(|t| t.name.eq_ignore_ascii_case(&filter));
                    if let Some(idx) = tank_idx {
                        for fish in &mut self.tanks[idx].fish {
                            fields::populate_field_cache(fish, &all_names, &mut rng);
                        }
                        let tank = &self.tanks[idx];
                        let fish_with_tanks: Vec<(&str, &Fish)> =
                            tank.fish.iter().map(|f| (tank.name.as_str(), f)).collect();
                        let souls = tank.soul_wall().map(|wall| (tank.name.as_str(), wall));
                        self.set_overlay(Overlay::Index(IndexState::new(
                            &fish_with_tanks,
                            souls,
                            all,
                            false,
                        )));
                    }
                } else {
                    for tank in &mut self.tanks {
                        for fish in &mut tank.fish {
                            fields::populate_field_cache(fish, &all_names, &mut rng);
                        }
                    }
                    let fish_with_tanks: Vec<(&str, &Fish)> = self
                        .tanks
                        .iter()
                        .flat_map(|t| t.fish.iter().map(|f| (t.name.as_str(), f)))
                        .collect();
                    self.set_overlay(Overlay::Index(IndexState::new(
                        &fish_with_tanks,
                        None,
                        all,
                        true,
                    )));
                }
                true
            }
            commands::Action::Inventory => {
                if let Some(state) = self.inventory_listing() {
                    self.set_overlay(Overlay::Inventory(state));
                }
                true
            }
            commands::Action::Shop => {
                self.set_overlay(Overlay::Shop(ShopState::open(self.shop_access())));
                true
            }
            commands::Action::Consume { name } => {
                let normalized: String = name.to_ascii_lowercase().split_whitespace().collect();
                let Some(stock) = ConsumableKind::all()
                    .into_iter()
                    .map(StockItem::Consumable)
                    .chain([StockItem::Junk])
                    .find(|stock| {
                        let dn: String = stock
                            .display_name()
                            .to_ascii_lowercase()
                            .split_whitespace()
                            .collect();
                        dn == normalized
                    })
                else {
                    return false;
                };
                self.try_consume_stock(
                    stock,
                    crate::ui::consume_picker::ConsumePickerSource::FromCommand,
                )
            }
            commands::Action::Fish {
                no_escape,
                no_fight,
                temper,
            } => {
                let milk = self.milk_buffs();
                let mut state = FishingState::new(milk, &mut rand::rng());
                state.no_escape = no_escape || self.cheats.no_escape;
                state.no_fight = no_fight;
                state.forced_temper = temper;
                self.set_overlay(Overlay::Fishing(state));
                if no_fight {
                    self.hook_the_catch();
                }
                true
            }
            commands::Action::Switch(tank_name) => {
                if self.tanks.len() <= 1 {
                    return false;
                }
                let Some(idx) = self
                    .tanks
                    .iter()
                    .position(|t| t.name.eq_ignore_ascii_case(&tank_name))
                else {
                    return false;
                };
                self.current_tank = idx;
                true
            }
            commands::Action::Move { fish, tank } => self.move_entity(&fish, &tank),
            commands::Action::Ledger => self.open_ledger(),
            commands::Action::Fishtanks => {
                self.set_overlay(Overlay::Fishtanks(FishtanksState::new(
                    &self.tanks,
                    self.current_tank,
                )));
                true
            }
            commands::Action::Show {
                name: fish_name,
                all,
            } => {
                let all_names: Vec<String> = self
                    .tanks
                    .iter()
                    .flat_map(|t| t.fish.iter().map(|f| f.name.clone()))
                    .collect();
                let mut rng = rand::rng();
                'populate: for tank in &mut self.tanks {
                    for fish in &mut tank.fish {
                        if fish.name.eq_ignore_ascii_case(&fish_name) {
                            fields::populate_field_cache(fish, &all_names, &mut rng);
                            break 'populate;
                        }
                    }
                }
                let found = self
                    .tanks
                    .iter()
                    .flat_map(|t| t.fish.iter().map(move |f| (f, t.name.as_str())))
                    .find(|(f, _)| f.name.eq_ignore_ascii_case(&fish_name))
                    .map(|(f, tn)| (f.clone(), tn.to_string()));
                let Some((fish, tank_name)) = found else {
                    return false;
                };
                if fish.is_invisible() {
                    return false;
                }
                let tank_kind = self
                    .tanks
                    .iter()
                    .find(|t| t.name == tank_name)
                    .map(|t| t.kind)
                    .unwrap_or(TankKind::Base);
                self.set_overlay(Overlay::Show {
                    state: ShowState::new(
                        &fish,
                        &tank_name,
                        tank_kind,
                        &all_names,
                        ShowSource::FromCommand,
                        all,
                        &mut rng,
                    ),
                    backed_index: None,
                });
                true
            }
            commands::Action::Exit => {
                self.running = false;
                true
            }
            commands::Action::Reset => self.reset_game(),
            commands::Action::Export(path) => self.export_game(&path),
            commands::Action::Import(path) => self.import_game(&path),
            commands::Action::Cowsay(text) => self.cowsay(text),
            commands::Action::Say(text) => self.say(None, text),
            commands::Action::VoidSpawn => {
                if !self.tanks[self.current_tank].kind.config().spawns_unfish {
                    return false;
                }
                let mut rng = rand::rng();
                self.tanks[self.current_tank].spawn_unfish(&mut rng);
                true
            }
            commands::Action::StartVoidWish { skip } => {
                if !self.tanks[self.current_tank].kind.config().hosts_ritual {
                    return false;
                }
                if skip {
                    self.editor.clear();
                    self.void_ritual = VoidRitualState::Wish {
                        retries_left: void_ritual::MAX_WISH_RETRIES,
                    };
                } else {
                    self.start_void_ritual();
                }
                true
            }
            commands::Action::StartFishAbduction => {
                let ct = self.current_tank;
                self.plan_abduction(ct, &mut rand::rng())
            }
            commands::Action::StartCowAbduction => {
                let mut rng = rand::rng();
                let ct = self.current_tank;
                let variant = crate::entities::cow::random_cow_color(&mut rng);
                self.plan_cow_delivery(ct, variant, &mut rng);
                true
            }
            commands::Action::StartCallHome => {
                let ct = self.current_tank;
                if !self.tanks[ct].call_home_now(&mut rand::rng()) {
                    return false;
                }
                self.answer_call_home(ct);
                true
            }
            commands::Action::Buy(target) => self.execute_buy(target),
            commands::Action::Sell(target) => self.execute_sell(target),
            commands::Action::Cheat => self.open_cheat_popup(),
            commands::Action::ToggleDebugMode => self.toggle_debug_mode(),
            commands::Action::Unknown => false,
        }
    }

    fn execute_buy(&mut self, target: commands::BuyTarget) -> bool {
        let access = self.shop_access();
        match target {
            commands::BuyTarget::Fish(species) => {
                let Some(catalog_idx) = FishSpecies::all_buyable()
                    .iter()
                    .position(|&buyable| buyable == species)
                else {
                    return false;
                };
                let mut fl: BuyListState<FishNamePopup> =
                    BuyListState::new(BuyList::Fishes, access);
                fl.selected = catalog_idx;
                let Some(popup) = fl.purchase(access) else {
                    return false;
                };
                fl.popup = Some(popup);
                let mut shop = ShopState::new();
                shop.page = ShopPage::BuyFishList(Box::new(fl));
                self.set_overlay(Overlay::Shop(shop));
                true
            }
            commands::BuyTarget::Tank(kind) => {
                let Some(catalog_idx) = TankKind::all_buyable().iter().position(|&s| s == kind)
                else {
                    return false;
                };
                let mut tl: BuyListState<BuyTankPopup> = BuyListState::new(BuyList::Tanks, access);
                tl.selected = catalog_idx;
                let Some(popup) = tl.purchase(access) else {
                    return false;
                };
                tl.popup = Some(popup);
                let mut shop = ShopState::new();
                shop.page = ShopPage::BuyTankList(tl);
                self.set_overlay(Overlay::Shop(shop));
                true
            }
            commands::BuyTarget::Food { qty } => {
                let cost = Money::from(qty) * Money::from(food::FOOD_BUY_PRICE);
                if !self.pay(cost, Flow::Food) {
                    return false;
                }
                self.food_supply += qty;
                true
            }
            commands::BuyTarget::Coffee { qty } => {
                let cost = Money::from(qty) * Money::from(ConsumableKind::Coffee.buy_price());
                if !self.pay(cost, Flow::Coffee) {
                    return false;
                }
                *self.inventory.entry(StockItem::COFFEE).or_insert(0) += qty;
                true
            }
            commands::BuyTarget::Bait { qty } => {
                let cost = Money::from(qty) * Money::from(ConsumableKind::Bait.buy_price());
                if !self.pay(cost, Flow::Bait) {
                    return false;
                }
                *self.inventory.entry(StockItem::BAIT).or_insert(0) += qty;
                true
            }
        }
    }

    fn execute_sell(&mut self, target: commands::SellTarget) -> bool {
        match target {
            commands::SellTarget::Fish(name) => self.sell_fish_by_name(&name),
            commands::SellTarget::Tank(name) => self.sell_tank_by_name(&name),
            commands::SellTarget::Blueprint(name) => self.sell_blueprint_by_name(&name),
            commands::SellTarget::Junk { qty } => self.sell_stock_checked(
                StockItem::Junk,
                qty,
                crate::ui::shop_overlay::JUNK_SELL_PRICE,
            ),
            commands::SellTarget::Coffee { qty } => {
                self.sell_stock_checked(StockItem::COFFEE, qty, ConsumableKind::Coffee.sell_price())
            }
            commands::SellTarget::Bait { qty } => {
                self.sell_stock_checked(StockItem::BAIT, qty, ConsumableKind::Bait.sell_price())
            }
            commands::SellTarget::Milk { variant, qty } => self.sell_stock_checked(
                StockItem::Consumable(ConsumableKind::Milk(variant)),
                qty,
                ConsumableKind::Milk(variant).sell_price(),
            ),
            commands::SellTarget::Seed { kind, qty }
            | commands::SellTarget::Robotics { kind, qty } => {
                self.sell_stock_checked(StockItem::Consumable(kind), qty, kind.sell_price())
            }
        }
    }

    fn sell_stock_checked(&mut self, stock: StockItem, qty: u32, unit_price: u32) -> bool {
        let owned = self.inventory.get(&stock).copied().unwrap_or(0);
        let sell_qty = qty.min(owned);
        if sell_qty == 0 {
            return false;
        }
        self.sell_stock(stock, sell_qty, unit_price);
        true
    }

    fn sell_fish_by_name(&mut self, name: &str) -> bool {
        let mut sold = None;
        for tank in &mut self.tanks {
            if let Some(pos) = tank
                .fish
                .iter()
                .position(|f| f.name.eq_ignore_ascii_case(name) && f.is_sellable())
            {
                let price = tank.fish[pos].sell_value();
                sold = Some((tank.take_fish(pos), price));
                tank.signal(WorldSignal::Sale);
                break;
            }
        }
        let Some((fish, price)) = sold else {
            return false;
        };
        self.bury(fish);
        self.earn(price, Flow::FishSales);
        true
    }

    fn sell_tank_by_name(&mut self, name: &str) -> bool {
        let Some(pos) = self
            .tanks
            .iter()
            .position(|t| t.name.eq_ignore_ascii_case(name))
        else {
            return false;
        };
        if !self.can_sell_tank(pos) {
            return false;
        }
        let price = self.tanks[pos].kind.sell_price();
        self.demolish_tank(pos);
        self.earn(price, Flow::TankSales);
        true
    }

    fn take_blueprint(&mut self, name: &str) -> Option<Blueprint> {
        let index = self.blueprint_index(name)?;
        Some(self.blueprints.remove(index))
    }

    fn sell_blueprint_by_name(&mut self, name: &str) -> bool {
        let Some(blueprint) = self.take_blueprint(name) else {
            return false;
        };
        self.earn(blueprint.sell_price(), Flow::StockSales);
        true
    }

    fn sell_stock(&mut self, stock: StockItem, qty: u32, unit_price: u32) {
        self.take_stock(stock, qty);
        self.earn(Money::from(qty) * Money::from(unit_price), Flow::StockSales);
    }

    fn take_stock(&mut self, stock: StockItem, qty: u32) {
        let entry = self.inventory.entry(stock).or_insert(0);
        *entry = entry.saturating_sub(qty);
        self.inventory.retain(|_, v| *v > 0);
    }
}

fn bot_action_allowed(action: &commands::Action) -> bool {
    use commands::Action::*;
    matches!(
        action,
        Feed | SetFps(_)
            | SetClock(_)
            | ToggleNames
            | ToggleNets
            | ToggleStats
            | ModResource { .. }
            | Spawn(..)
            | Mutate(..)
            | Give(_)
            | Revive(_)
            | Kill(_)
            | Clone(_)
            | Bless
            | Expand(_)
            | Restore(_)
            | Move { .. }
            | Cowsay(_)
            | Say(_)
            | Buy(_)
            | Sell(_)
            | SetFrozen { .. }
            | Nudge { .. }
            | Flip(_)
            | Print(_)
            | Etch { .. }
    )
}
