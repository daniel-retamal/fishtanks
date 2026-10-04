use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use rand::RngExt;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
};

use crate::{
    abduction::Abductable,
    cheats::Cheats,
    commands,
    consumable::{ActiveConsumable, ActiveMilkStatus, Buff, Caster, ConsumeTarget},
    economy::Purse,
    fishes::{fish::Fish, graveyard::Graveyard, species::FishSpecies},
    ledger::{Flow, Ledger},
    loot::{ConsumableKind, CowCounts, JUNK_PER_JUNKFISH, LootKind, LootPool, StockItem},
    names,
    settings::Settings,
    tank::{Blueprint, DayClock, Tank, TankEvent, TankKind, UfoRole, WorldSignal},
    ui::{
        catch_overlay::{CatchOverlay, CatchState},
        cheat_popup::CheatPopup,
        circuit_overlay::{CircuitOverlay, CircuitState},
        command_bar::{self, CommandBar},
        console::ConsoleState,
        consume_picker::{ConsumePickerOverlay, ConsumePickerState},
        fishing_overlay::{FishingGeometry, FishingOverlay, FishingState},
        fishtanks_overlay::{FishtanksOverlay, FishtanksState},
        foundry_overlay::{FoundryOverlay, FoundryState, Quotes},
        index_overlay::{IndexOverlay, IndexState},
        input_action::HeldKeys,
        inventory_overlay::{InventoryOverlay, InventoryState},
        layout::Screen,
        ledger_overlay::{LedgerOverlay, LedgerState},
        line_editor::{CommandHistory, LineEditor},
        notice::{NoticePopup, NoticeState},
        shop_overlay::{
            JunkfishPopup, JunkfishPopupWidget, NamingPopupWidget, ShopOverlay, ShopState,
        },
        show_overlay::{ShowOverlay, ShowState},
        tank_view::TankView,
        text_input::TextInput,
        wiring_panel::{WiringPanel, WiringPanelState},
    },
    util::sample_exponential,
    void_ritual::{self, VoidRitualState},
};

mod cheats;
mod console;
mod heaven;
mod input;
mod lessons;
mod money;
mod news;
mod persistence;
mod room;
mod snapshot;
mod zen;

pub use cheats::Launch;
pub use lessons::{LESSON_STREAK, Lessons};
pub use snapshot::{
    FIRST_FISH, FIRST_TANK_NAME, SAVE_VERSION, STARTING_CASH, STARTING_FOOD, SaveFile,
};

use persistence::Persistence;

const TERMINAL_HEIGHT_DEFAULT: u16 = 24;
const TERMINAL_WIDTH_DEFAULT: u16 = 80;
const ONE_OF_A_KIND: u32 = 1;

enum Overlay {
    Index(IndexState),
    Show {
        state: ShowState,
        backed_index: Option<Box<IndexState>>,
    },
    Inventory(InventoryState),
    Fishing(FishingState),
    Catch(CatchState),
    Shop(ShopState),
    Fishtanks(FishtanksState),
    Circuit(CircuitState),
    Foundry(FoundryState),
    Wiring {
        state: WiringPanelState,
        backed_circuit: Option<Box<CircuitState>>,
    },
    ConsumePicker(ConsumePickerState),
    Console(ConsoleState),
    Naming {
        input: TextInput,
        kind: ConsumableKind,
    },
    Junkfish(JunkfishPopup),
    Cheat(TextInput),
    Notice(NoticeState),
    Ledger(LedgerState),
}

pub const CAJETANS_GRACE_SECS: f32 = 3.0 * 60.0 + 33.0;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct GraceBuff {
    pub stacks: u32,
    pub time_remaining: f32,
}

pub struct App {
    pub settings: Settings,
    pub tanks: Vec<Tank>,
    pub current_tank: usize,
    used_tank_names: HashSet<String>,
    pub purse: Purse,
    pub ledger: Ledger,
    pub debug_mode: bool,
    pub cheats: Cheats,
    pub food_supply: u32,
    pub inventory: HashMap<StockItem, u32>,
    pub active_consumables: Vec<ActiveConsumable>,
    pub active_statuses: Vec<ActiveMilkStatus>,
    pub cajetans_grace: GraceBuff,
    pub editor: LineEditor,
    pub running: bool,
    history: CommandHistory,
    active_overlay: Option<Overlay>,
    held_keys: HeldKeys,
    terminal_height: u16,
    terminal_width: u16,
    pub graveyard: Graveyard,
    pub blueprints: Vec<Blueprint>,
    pub void_ritual: VoidRitualState,
    pub next_prayer: usize,
    pub nothing_stacks: u32,
    pending_ufo_dest: HashMap<String, usize>,
    day_clock: DayClock,
    persistence: Option<Persistence>,
    zen: bool,
    newer_release: bool,
    lessons: Lessons,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self::launch(Launch::Player)
    }

    pub fn launch(launch: Launch) -> Self {
        Self::new_game(launch == Launch::Debug)
    }

    fn index_state(&self) -> Option<&IndexState> {
        match &self.active_overlay {
            Some(Overlay::Index(s)) => Some(s),
            _ => None,
        }
    }

    fn index_state_mut(&mut self) -> Option<&mut IndexState> {
        match &mut self.active_overlay {
            Some(Overlay::Index(s)) => Some(s),
            _ => None,
        }
    }

    fn show_state_mut(&mut self) -> Option<&mut ShowState> {
        match &mut self.active_overlay {
            Some(Overlay::Show { state, .. }) => Some(state),
            _ => None,
        }
    }

    fn inventory_state(&self) -> Option<&InventoryState> {
        match &self.active_overlay {
            Some(Overlay::Inventory(s)) => Some(s),
            _ => None,
        }
    }

    fn inventory_listing(&self) -> Option<InventoryState> {
        InventoryState::new(&self.inventory, &self.blueprints, &mut rand::rng())
    }

    fn inventory_state_mut(&mut self) -> Option<&mut InventoryState> {
        match &mut self.active_overlay {
            Some(Overlay::Inventory(s)) => Some(s),
            _ => None,
        }
    }

    pub fn fishing_state(&self) -> Option<&FishingState> {
        match &self.active_overlay {
            Some(Overlay::Fishing(s)) => Some(s),
            _ => None,
        }
    }

    fn fishing_state_mut(&mut self) -> Option<&mut FishingState> {
        match &mut self.active_overlay {
            Some(Overlay::Fishing(s)) => Some(s),
            _ => None,
        }
    }

    fn catch_state(&self) -> Option<&CatchState> {
        match &self.active_overlay {
            Some(Overlay::Catch(s)) => Some(s),
            _ => None,
        }
    }

    fn catch_state_mut(&mut self) -> Option<&mut CatchState> {
        match &mut self.active_overlay {
            Some(Overlay::Catch(s)) => Some(s),
            _ => None,
        }
    }

    fn consume_picker_state(&self) -> Option<&ConsumePickerState> {
        match &self.active_overlay {
            Some(Overlay::ConsumePicker(s)) => Some(s),
            _ => None,
        }
    }

    fn consume_picker_state_mut(&mut self) -> Option<&mut ConsumePickerState> {
        match &mut self.active_overlay {
            Some(Overlay::ConsumePicker(s)) => Some(s),
            _ => None,
        }
    }

    fn naming_input(&self) -> Option<&TextInput> {
        match &self.active_overlay {
            Some(Overlay::Naming { input, .. }) => Some(input),
            _ => None,
        }
    }

    fn naming_input_mut(&mut self) -> Option<&mut TextInput> {
        match &mut self.active_overlay {
            Some(Overlay::Naming { input, .. }) => Some(input),
            _ => None,
        }
    }

    fn open_naming_popup(&mut self, kind: ConsumableKind) {
        self.set_overlay(Overlay::Naming {
            input: TextInput::new(),
            kind,
        });
    }

    pub fn index_overlay_open(&self) -> bool {
        matches!(self.active_overlay, Some(Overlay::Index(_)))
    }

    pub fn fishtanks_overlay_open(&self) -> bool {
        matches!(self.active_overlay, Some(Overlay::Fishtanks(_)))
    }

    pub fn fishtanks_state_mut(&mut self) -> Option<&mut FishtanksState> {
        match &mut self.active_overlay {
            Some(Overlay::Fishtanks(s)) => Some(s),
            _ => None,
        }
    }

    fn take_catch_state(&mut self) -> Option<CatchState> {
        match self.active_overlay.take() {
            Some(Overlay::Catch(s)) => Some(s),
            other => {
                self.active_overlay = other;
                None
            }
        }
    }

    fn take_shop_state(&mut self) -> Option<ShopState> {
        match self.active_overlay.take() {
            Some(Overlay::Shop(s)) => Some(s),
            other => {
                self.active_overlay = other;
                None
            }
        }
    }

    fn wiring_state_mut(&mut self) -> Option<&mut WiringPanelState> {
        match &mut self.active_overlay {
            Some(Overlay::Wiring { state, .. }) => Some(state),
            _ => None,
        }
    }

    fn take_wiring_state(&mut self) -> Option<(WiringPanelState, Option<Box<CircuitState>>)> {
        match self.active_overlay.take() {
            Some(Overlay::Wiring {
                state,
                backed_circuit,
            }) => Some((state, backed_circuit)),
            other => {
                self.active_overlay = other;
                None
            }
        }
    }

    fn circuit_state(&self) -> Option<&CircuitState> {
        match &self.active_overlay {
            Some(Overlay::Circuit(s)) => Some(s),
            _ => None,
        }
    }

    fn circuit_state_mut(&mut self) -> Option<&mut CircuitState> {
        match &mut self.active_overlay {
            Some(Overlay::Circuit(s)) => Some(s),
            _ => None,
        }
    }

    fn take_circuit_state(&mut self) -> Option<CircuitState> {
        match self.active_overlay.take() {
            Some(Overlay::Circuit(s)) => Some(s),
            other => {
                self.active_overlay = other;
                None
            }
        }
    }

    fn set_overlay(&mut self, overlay: Overlay) {
        self.leave_the_water();
        self.leave_console();
        self.held_keys.let_go();
        self.active_overlay = Some(overlay);
    }

    fn close_overlay(&mut self) {
        self.leave_the_water();
        self.leave_console();
        self.held_keys.let_go();
        self.active_overlay = None;
    }

    fn tank(&self) -> &Tank {
        &self.tanks[self.current_tank]
    }

    fn tank_mut(&mut self) -> &mut Tank {
        &mut self.tanks[self.current_tank]
    }

    fn graveyard_names(&self) -> Vec<String> {
        self.graveyard.iter().map(|f| f.name.clone()).collect()
    }

    fn coffee_stacks(&self) -> u32 {
        self.active_consumables
            .iter()
            .filter(|c| matches!(c.kind, ConsumableKind::Coffee))
            .map(|c| c.stacks)
            .sum()
    }

    fn bait_stacks(&self) -> u32 {
        self.active_consumables
            .iter()
            .filter(|c| matches!(c.kind, ConsumableKind::Bait))
            .map(|c| c.stacks)
            .sum()
    }

    fn milk_buffs(&self) -> crate::ui::fishing_overlay::MilkBuffs {
        use crate::consumable::MilkStatus;
        let stacks_of = |kind: MilkStatus| -> u32 {
            self.active_statuses
                .iter()
                .filter(|s| s.kind == kind)
                .map(|s| s.stacks)
                .sum()
        };
        crate::ui::fishing_overlay::MilkBuffs {
            visual_calculus: stacks_of(MilkStatus::VisualCalculus),
            volition: stacks_of(MilkStatus::Volition),
            physical_instrument: stacks_of(MilkStatus::PhysicalInstrument),
            reaction_speed: stacks_of(MilkStatus::ReactionSpeed),
        }
    }

    pub fn entity_mutations(&self) -> Vec<(String, Vec<crate::fishes::mutations::Mutation>)> {
        use crate::fishes::mutations::Mutatable;
        let tank = self.tank();
        let mut out = Vec::with_capacity(tank.fish.len() + tank.cows.len());
        for fish in &tank.fish {
            out.push((fish.name.clone(), fish.available_mutations()));
        }
        for cow in &tank.cows {
            out.push((cow.name.clone(), cow.available_mutations()));
        }
        out
    }

    fn cow_counts_in(tank: &Tank) -> CowCounts {
        let mut counts = CowCounts::default();
        let irradiates_milk = tank.kind.config().irradiates_milk;
        for cow in &tank.cows {
            if irradiates_milk {
                counts.add(crate::loot::MilkVariant::Irradiated, cow.milk_yield());
                continue;
            }
            for variant in cow.milk_components() {
                counts.add(variant.milk(), 1);
            }
        }
        counts
    }

    fn devils_luck(&self) -> u32 {
        Self::devils_luck_in(self.tank())
    }

    fn devils_luck_in(tank: &Tank) -> u32 {
        if !tank.kind.config().devils_luck {
            return 0;
        }
        (tank.fish.len() + tank.soul_count()) as u32
    }

    fn grace_stacks(&self) -> u32 {
        self.cajetans_grace.stacks
    }

    fn owned_consumable_names(&self) -> Vec<String> {
        ConsumableKind::all()
            .into_iter()
            .filter(|kind| kind.can_be_consumed())
            .filter(|kind| {
                self.inventory
                    .get(&StockItem::Consumable(*kind))
                    .is_some_and(|&qty| qty > 0)
            })
            .map(ConsumableKind::lowercase_name)
            .chain(
                StockItem::Junk
                    .consumable_at(self.stock_of(StockItem::Junk))
                    .then(|| StockItem::Junk.display_name().to_ascii_lowercase()),
            )
            .collect()
    }

    pub fn stock_of(&self, stock: StockItem) -> u32 {
        self.inventory.get(&stock).copied().unwrap_or(0)
    }

    pub fn try_consume_stock(
        &mut self,
        stock: StockItem,
        source: crate::ui::consume_picker::ConsumePickerSource,
    ) -> bool {
        match stock {
            StockItem::Consumable(kind) => self.try_consume_kind(kind, source),
            StockItem::Junk => self.open_junkfish_popup(),
        }
    }

    fn open_junkfish_popup(&mut self) -> bool {
        let Some(species) = FishSpecies::from_junk() else {
            return false;
        };
        if !StockItem::Junk.consumable_at(self.stock_of(StockItem::Junk)) {
            return false;
        }
        let fish = Fish::new(species, String::new(), 0.0, 0.0, &mut rand::rng());
        if self.landing_tank(self.current_tank, &fish).is_none() {
            return false;
        }
        self.set_overlay(Overlay::Junkfish(JunkfishPopup {
            fish,
            name_input: TextInput::new(),
        }));
        true
    }

    fn assemble_junkfish(&mut self, fish: Fish, name: String) -> bool {
        let junk = self.stock_of(StockItem::Junk);
        if !StockItem::Junk.consumable_at(junk) {
            return false;
        }
        if self.land_fish(self.current_tank, fish, name).is_err() {
            return false;
        }
        self.inventory
            .insert(StockItem::Junk, junk - JUNK_PER_JUNKFISH);
        self.inventory.retain(|_, qty| *qty > 0);
        true
    }

    pub fn shop_access(&self) -> crate::ui::shop_overlay::ShopAccess {
        crate::ui::shop_overlay::ShopAccess {
            cash: self.purse.spendable(),
            connected: self.is_connected(),
            room: FishSpecies::all_buyable()
                .iter()
                .any(|&species| self.has_room_for_a_new(species)),
            sellable: self.build_sell_menu_state().is_some(),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.tanks.iter().any(|t| t.kind.config().connects)
    }

    pub(super) fn claims(&self, kind: TankKind) -> bool {
        if !kind.config().unique {
            return false;
        }
        let owned = self.tanks.iter().any(|t| t.kind == kind);
        let held = ConsumableKind::seeds()
            .into_iter()
            .filter(|seed| seed.summons_tank() == Some(kind))
            .any(|seed| self.held(StockItem::Consumable(seed)) > 0);
        owned || held
    }

    fn held(&self, stock: StockItem) -> u32 {
        self.inventory.get(&stock).copied().unwrap_or(0)
    }

    pub(super) fn room_for(&self, stock: StockItem) -> u32 {
        let StockItem::Consumable(kind) = stock else {
            return u32::MAX;
        };
        match kind.summons_tank() {
            Some(tank) if tank.config().unique => {
                if self.claims(tank) {
                    0
                } else {
                    ONE_OF_A_KIND
                }
            }
            _ => u32::MAX,
        }
    }

    pub(super) fn stock_up(&mut self, stock: StockItem, qty: u32) -> bool {
        let qty = qty.min(self.room_for(stock));
        if qty == 0 {
            return false;
        }
        *self.inventory.entry(stock).or_insert(0) += qty;
        true
    }

    fn withheld_loot(&self) -> Vec<ConsumableKind> {
        ConsumableKind::seeds()
            .into_iter()
            .filter(|&seed| self.room_for(StockItem::Consumable(seed)) == 0)
            .collect()
    }

    fn consume_item(&mut self, kind: ConsumableKind) {
        self.active_consumables
            .extend(ActiveConsumable::fresh(kind));
    }

    fn spend_a_cast(&mut self, caster: Caster) {
        for buff in &mut self.active_consumables {
            buff.spend_cast(caster);
        }
        self.active_consumables.retain(|buff| !buff.is_spent());
        for status in &mut self.active_statuses {
            status.spend_cast(caster);
        }
        self.active_statuses.retain(|status| !status.is_spent());
    }

    pub fn try_consume_kind(
        &mut self,
        kind: ConsumableKind,
        source: crate::ui::consume_picker::ConsumePickerSource,
    ) -> bool {
        let stock = StockItem::Consumable(kind);
        if self.inventory.get(&stock).copied().unwrap_or(0) == 0 {
            return false;
        }
        if kind.summons_tank().is_some() {
            self.open_naming_popup(kind);
            return true;
        }
        match kind {
            ConsumableKind::Necronomicon
            | ConsumableKind::DemonCore
            | ConsumableKind::Computer
            | ConsumableKind::VoidSeed
            | ConsumableKind::GoldenPearl => unreachable!("a tank's item opens the naming popup"),
            ConsumableKind::BlankBlueprint => {
                if !self.tank().fish.iter().any(|f| f.is_wired()) {
                    return false;
                }
                self.open_naming_popup(kind);
            }
            ConsumableKind::Milk(variant) => {
                let Some(status) = variant.status() else {
                    return self.open_consume_picker(ConsumeTarget::Milk(variant), source);
                };
                self.drink_milk(stock, status);
            }
            ConsumableKind::Part(part) => {
                return self.open_consume_picker(ConsumeTarget::Part(part), source);
            }
            ConsumableKind::BlankWafer => return false,
            ConsumableKind::Fabricator => return self.open_foundry(),
            ConsumableKind::Coffee | ConsumableKind::Bait => {
                self.consume_item(kind);
                let entry = self.inventory.entry(stock).or_insert(0);
                *entry = entry.saturating_sub(1);
                self.inventory.retain(|_, v| *v > 0);
                let now_empty = if let Some(Overlay::Inventory(state)) = &mut self.active_overlay {
                    state.update_from(&self.inventory, &self.blueprints, &mut rand::rng());
                    state.items.is_empty()
                } else {
                    false
                };
                if now_empty {
                    self.close_overlay();
                }
                let bh = self.bar_height();
                let tw = self.terminal_width;
                let th = self.terminal_height.saturating_sub(bh);
                let dead_names = self.graveyard_names();
                self.tanks[self.current_tank].resize(tw, th, &dead_names);
            }
        }
        true
    }

    fn tick_fishing(&mut self) {
        let fps = self.settings.fps;
        let coffee = self.coffee_stacks();
        let milk = self.milk_buffs();
        let geom = FishingGeometry::from_area(self.tank_area());
        if let Some(Overlay::Fishing(s)) = &mut self.active_overlay {
            s.hold(&self.held_keys);
            s.tick(fps, coffee, milk, geom);
        }
        let (game_over, captured) = match self.fishing_state() {
            Some(s) => (s.game_over, s.captured),
            None => return,
        };
        if game_over {
            self.close_overlay();
            return;
        }
        if !captured {
            return;
        }
        let mut rng = rand::rng();
        let hooked = self.fishing_state_mut().and_then(FishingState::take_catch);
        let loot = hooked.unwrap_or_else(|| self.roll_catch(self.current_tank, &mut rng));
        self.spend_a_cast(Caster::Angler);
        let card = self.counted(CatchState::new(loot, &mut rng));
        self.set_overlay(Overlay::Catch(card));
    }

    fn counted(&self, mut card: CatchState) -> CatchState {
        card.item_qty = match &card.loot {
            LootKind::Item(item) => {
                StockItem::from_item(item)
                    .and_then(|stock| self.inventory.get(&stock).copied())
                    .unwrap_or(0)
                    + 1
            }
            _ => 0,
        };
        card
    }

    fn catch_pool(&self, tank_idx: usize) -> LootPool {
        let tank = &self.tanks[tank_idx];
        LootPool::default_pool()
            .with_native(tank.kind)
            .with_bait(self.bait_stacks())
            .without(&self.withheld_loot())
            .with_devils_luck(Self::devils_luck_in(tank))
            .with_grace(self.grace_stacks())
            .keeping_species(|species| self.has_room_for_a_new(species))
    }

    pub(super) fn roll_catch(&self, tank_idx: usize, rng: &mut impl RngExt) -> LootKind {
        self.catch_pool(tank_idx)
            .with_cows(&Self::cow_counts_in(&self.tanks[tank_idx]))
            .roll(rng)
    }

    pub(super) fn roll_rig_catch(
        &self,
        tank_idx: usize,
        rng: &mut impl RngExt,
    ) -> Option<FishSpecies> {
        self.catch_pool(tank_idx).roll_species(rng)
    }

    pub(super) fn has_room_for_a_catch(&self, tank_idx: usize) -> bool {
        self.catch_pool(tank_idx).offers_fish()
    }

    pub fn tick(&mut self) {
        let lifted = self.held_keys.tick(1.0 / self.settings.fps);
        if matches!(self.active_overlay, Some(Overlay::Fishing(_))) {
            self.tick_fishing();
            return;
        }
        self.lift_console_keys(lifted);
        match &mut self.active_overlay {
            Some(Overlay::Catch(catch)) => {
                catch.tick(self.settings.fps);
                return;
            }
            Some(Overlay::Naming { .. })
            | Some(Overlay::Junkfish(_))
            | Some(Overlay::Cheat(_))
            | Some(Overlay::Notice(_))
            | Some(Overlay::Ledger(_))
            | Some(Overlay::Inventory(_))
            | Some(Overlay::Fishtanks(_))
            | Some(Overlay::Foundry(_))
            | Some(Overlay::Wiring { .. }) => return,
            Some(Overlay::Show { state, .. }) => {
                state.tick_animation(1.0 / self.settings.fps);
                return;
            }
            Some(Overlay::Index(idx)) => {
                idx.tick_animation(1.0 / self.settings.fps);
                return;
            }
            Some(Overlay::Shop(shop)) => {
                shop.tick(self.settings.fps);
                return;
            }
            Some(Overlay::Fishing(_)) => return,
            Some(Overlay::Circuit(_))
            | Some(Overlay::ConsumePicker(_))
            | Some(Overlay::Console(_))
            | None => {}
        }

        let dt = 1.0 / self.settings.fps;
        self.ledger.tick(dt);

        if self.day_clock.tick(dt) {
            for tank in &mut self.tanks {
                tank.signal(WorldSignal::Dawn);
            }
        }

        self.tick_void_ritual(dt);
        if self.void_ritual.is_blocking() {
            self.tick_blink();
            return;
        }

        for buff in &mut self.active_consumables {
            buff.tick(dt);
        }
        self.active_consumables.retain(|buff| !buff.is_spent());

        if self.cajetans_grace.stacks > 0 {
            self.cajetans_grace.time_remaining -= dt;
            if self.cajetans_grace.time_remaining <= 0.0 {
                self.cajetans_grace = GraceBuff::default();
            }
        }

        let coffee = self.coffee_stacks();
        let sky = self.day_clock.sky(self.zen);
        for i in 0..self.tanks.len() {
            let events = self.tanks[i].tick(&self.settings, coffee, sky);
            let star_cash = std::mem::take(&mut self.tanks[i].pending_star_cash);
            self.earn(star_cash, Flow::Cashfish);
            for part in std::mem::take(&mut self.tanks[i].pending_loose_parts) {
                *self
                    .inventory
                    .entry(StockItem::Consumable(ConsumableKind::Part(part)))
                    .or_insert(0) += 1;
            }
            for lost in std::mem::take(&mut self.tanks[i].pending_graveyard) {
                self.bury(lost);
            }
            for event in events {
                match event {
                    TankEvent::Blessing => {
                        self.perform_blessing(i);
                    }
                    TankEvent::Wander { fish_name } => {
                        self.handle_wander(i, &fish_name);
                    }
                    TankEvent::Echo {
                        speaker,
                        text,
                        strength,
                    } => {
                        self.spread_speech(&text, Some(i), strength, Some(&speaker));
                    }
                    TankEvent::UfoTimerFired => {
                        self.handle_ufo_timer_fired(i);
                    }
                    TankEvent::UfoLockFish { fish_name } => {
                        if let Some(fish) =
                            self.tanks[i].fish.iter_mut().find(|f| f.name == fish_name)
                        {
                            fish.abduction_lock = true;
                        }
                    }
                    TankEvent::UfoTakeFish { fish_name } => {
                        self.handle_ufo_take_fish(i, &fish_name);
                    }
                    TankEvent::UfoReleaseFish(fish) => {
                        self.tanks[i].place_fish_dropped(*fish);
                    }
                    TankEvent::UfoReleaseCow(cow) => {
                        self.tanks[i].land_cow_delivery(*cow);
                    }
                    TankEvent::UfoFinished => {}
                    TankEvent::CallHome => self.answer_call_home(i),
                }
            }
        }
        self.tick_fabric();
        self.settle_console();
        self.tick_botfish();
        self.tick_casts();
        self.settle_arrivals();
        self.refresh_circuit();
        self.tick_blink();
    }

    fn refresh_circuit(&mut self) {
        let Some(Overlay::Circuit(state)) = &mut self.active_overlay else {
            return;
        };
        let Some(tank) = self.tanks.get(state.tank_idx) else {
            return;
        };
        state.refresh_levels(&tank.fish);
    }

    fn bar_height(&self) -> u16 {
        if self.zen {
            return 0;
        }
        command_bar::height(
            self.settings.show_stats,
            self.terminal_width,
            &self.stats_bar(&self.modes()),
            self.console_bar().as_ref(),
        )
    }

    fn stats_bar<'a>(&'a self, modes: &'a [&'static str]) -> command_bar::StatsBar<'a> {
        command_bar::StatsBar {
            active_consumables: &self.active_consumables,
            active_statuses: &self.active_statuses,
            cash: self.purse.shown(),
            food_supply: self.food_supply,
            fish_count: self.tank().fish.len(),
            fish_capacity: self.tank().shown_capacity(),
            modes,
            tank_name: &self.tank().name,
            devils_luck: self.devils_luck(),
            cajetans_grace: self.grace_stacks(),
        }
    }

    fn tank_height(&self) -> u16 {
        self.terminal_height.saturating_sub(self.bar_height())
    }

    fn tank_area(&self) -> Rect {
        Rect::new(0, 0, self.terminal_width, self.tank_height())
    }

    fn ghost(&self) -> String {
        if self.void_ritual.is_blocking() || !self.editor.text.starts_with('/') {
            return String::new();
        }
        let fish_names: Vec<&str> = self
            .tank()
            .fish
            .iter()
            .map(|f| f.name.as_str())
            .chain(self.tank().cows.iter().map(|c| c.name.as_str()))
            .collect();
        let owned_consumable_strings = self.owned_consumable_names();
        let consumable_names: Vec<&str> = owned_consumable_strings
            .iter()
            .map(String::as_str)
            .collect();
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
        let living_fish_names = self.living_fish_names();
        let sellable_fish_names = self.sellable_fish_names();
        let sellable_tank_names = self.sellable_tank_names();
        let sellable_stackable_strings = self.build_sellable_stackable_names();
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
        commands::autocomplete(
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
                console_names: &self.console_names(),
                blueprint_names: &self.blueprint_names(),
                arrangeable_names: &arrangeable_names,
                sellable_fish_names: &sellable_fish_names,
                sellable_tank_names: &sellable_tank_names,
                sellable_stackable_names: &sellable_stackable_strings,
                living_fish_names: &living_fish_names,
                clearance: self.clearance(),
            },
        )
        .map(|c| c.ghost)
        .unwrap_or_default()
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let full_area = frame.area();
        self.terminal_height = full_area.height;
        self.terminal_width = full_area.width;

        let [tank_area, command_area] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(self.bar_height())])
                .areas(full_area);

        let tank = &self.tanks[self.current_tank];
        if (tank.width, tank.height) != (tank_area.width, tank_area.height) {
            let dead_names = self.graveyard_names();
            self.tanks[self.current_tank].resize(tank_area.width, tank_area.height, &dead_names);
        }

        {
            let ritual_blocking = self.void_ritual.is_blocking()
                && self.tanks[self.current_tank].kind.config().hosts_ritual;
            let mut tv = TankView::new(self.tank())
                .with_names(self.settings.show_names && !self.zen)
                .with_nets(self.settings.show_nets && !self.zen)
                .with_epitaphs(!self.zen);
            if ritual_blocking {
                let text = void_ritual::wish_display_text(&self.void_ritual, self.next_prayer);
                tv = tv.with_ritual(text);
            }
            frame.render_widget(tv, tank_area);
        }

        if self.zen {
            return;
        }
        let ghost = self.ghost();
        let modes = self.modes();
        frame.render_widget(
            CommandBar {
                input: &self.editor.text,
                cursor_pos: self.editor.cursor,
                cursor_visible: self.editor.visible,
                ghost: &ghost,
                show_stats: self.settings.show_stats,
                stats: self.stats_bar(&modes),
                console: self.console_bar(),
            },
            command_area,
        );

        let screen = Screen::new(full_area, tank_area);
        match &self.active_overlay {
            Some(Overlay::Index(state)) => {
                frame.render_widget(IndexOverlay::new(state, screen), full_area)
            }
            Some(Overlay::Show { state, .. }) => {
                frame.render_widget(ShowOverlay::new(state, screen), full_area)
            }
            Some(Overlay::Inventory(state)) => {
                frame.render_widget(InventoryOverlay::new(state, screen), full_area)
            }
            Some(Overlay::Fishtanks(state)) => {
                frame.render_widget(FishtanksOverlay::new(state, screen), full_area)
            }
            Some(Overlay::ConsumePicker(state)) => {
                frame.render_widget(ConsumePickerOverlay { state, screen }, full_area)
            }
            Some(Overlay::Fishing(state)) => {
                frame.render_widget(FishingOverlay::new(state), tank_area)
            }
            Some(Overlay::Catch(state)) => {
                frame.render_widget(CatchOverlay::new(state, screen), full_area)
            }
            Some(Overlay::Shop(state)) => frame.render_widget(
                ShopOverlay::new(state, self.shop_access(), screen),
                full_area,
            ),
            Some(Overlay::Circuit(state)) => {
                frame.render_widget(CircuitOverlay::new(state, screen), full_area)
            }
            Some(Overlay::Wiring { state, .. }) => frame.render_widget(
                WiringPanel::new(state, self.editor.visible, screen),
                full_area,
            ),
            Some(Overlay::Foundry(state)) => {
                let quotes = self.blueprints.get(state.selected).map(|blueprint| Quotes {
                    print: self.print_quote(blueprint),
                    etch: self.etch_quote(blueprint),
                });
                frame.render_widget(
                    FoundryOverlay::new(
                        state,
                        &self.blueprints,
                        quotes,
                        self.editor.visible,
                        screen,
                    ),
                    full_area,
                )
            }
            Some(Overlay::Naming { input, kind }) => frame.render_widget(
                NamingPopupWidget {
                    input,
                    kind: *kind,
                    cursor_visible: self.editor.visible,
                    screen,
                },
                full_area,
            ),
            Some(Overlay::Junkfish(popup)) => frame.render_widget(
                JunkfishPopupWidget {
                    popup,
                    cursor_visible: self.editor.visible,
                    screen,
                },
                full_area,
            ),
            Some(Overlay::Cheat(input)) => frame.render_widget(
                CheatPopup {
                    input,
                    cursor_visible: self.editor.visible,
                    screen,
                },
                full_area,
            ),
            Some(Overlay::Notice(state)) => {
                frame.render_widget(NoticePopup::new(state, screen), full_area)
            }
            Some(Overlay::Ledger(state)) => {
                frame.render_widget(LedgerOverlay::new(state, screen), full_area)
            }
            Some(Overlay::Console(_)) | None => {}
        }
    }

    fn has_any_overlay(&self) -> bool {
        self.active_overlay.is_some()
    }

    fn tick_void_ritual(&mut self, dt: f32) {
        enum Tr {
            None,
            Abort,
            ResetMaybeStart(f32, bool),
        }

        let hosts_ritual = self.tanks[self.current_tank].kind.config().hosts_ritual;
        let has_overlay = self.has_any_overlay();

        let tr = match &mut self.void_ritual {
            VoidRitualState::Wish { .. } => Tr::None,
            VoidRitualState::Prayer { timeout, .. } | VoidRitualState::FinalPhrase { timeout } => {
                *timeout -= dt;
                if *timeout <= 0.0 { Tr::Abort } else { Tr::None }
            }
            VoidRitualState::Idle { timer } => {
                *timer -= dt;
                if *timer > 0.0 {
                    Tr::None
                } else {
                    let mean = void_ritual::ritual_mean_secs(self.nothing_stacks);
                    let new_t = sample_exponential(&mut rand::rng(), mean);
                    Tr::ResetMaybeStart(new_t, hosts_ritual && !has_overlay)
                }
            }
        };

        match tr {
            Tr::None => {}
            Tr::Abort => self.abort_void_ritual(),
            Tr::ResetMaybeStart(new_t, start) => {
                if let VoidRitualState::Idle { timer } = &mut self.void_ritual {
                    *timer = new_t;
                }
                if start {
                    self.start_void_ritual();
                }
            }
        }
    }

    fn start_void_ritual(&mut self) {
        self.editor.clear();
        self.void_ritual = VoidRitualState::Prayer {
            prayer_idx: self.next_prayer,
            phrase_idx: 0,
            timeout: void_ritual::PRAYER_TIMEOUT_SECS,
        };
    }

    fn abort_void_ritual(&mut self) {
        let mean = void_ritual::ritual_mean_secs(self.nothing_stacks);
        let timer = sample_exponential(&mut rand::rng(), mean);
        self.void_ritual = VoidRitualState::Idle { timer };
        self.editor.clear();
    }

    fn handle_ufo_timer_fired(&mut self, source_idx: usize) {
        let mut rng = rand::rng();
        match self.tanks[source_idx].kind.config().ufo_role {
            Some(UfoRole::DeliversCows) => {
                let variant = crate::entities::cow::random_cow_color(&mut rng);
                self.plan_cow_delivery(source_idx, variant, &mut rng);
            }
            Some(UfoRole::AbductsAtNight) if self.tanks[source_idx].background.is_night() => {
                self.plan_abduction(source_idx, &mut rng);
            }
            _ => {}
        }
    }

    pub(super) fn answer_call_home(&mut self, tank_idx: usize) {
        use crate::loot::Companion;
        self.trigger_botfish(crate::tank::ALIEN_TONGUE, Some(tank_idx));
        if !self.tanks[tank_idx].answers_call_home() {
            return;
        }
        let mut rng = rand::rng();
        match Companion::roll(self.tanks[tank_idx].kind, &mut rng) {
            Companion::Fish(species) => {
                let name = crate::tank::alien_name(&mut rng);
                let mut fish = Fish::new(species, name, 0.0, 0.0, &mut rng);
                alienate(&mut fish, &mut rng);
                self.drop_fish(tank_idx, fish, &mut rng);
            }
            Companion::Cow(variant) => self.plan_cow_delivery(tank_idx, variant, &mut rng),
        }
    }

    fn drop_fish(&mut self, dest_idx: usize, fish: Fish, rng: &mut impl RngExt) {
        use crate::entities::ufo::{UFO_SPRITE_HEIGHT, UFO_SPRITE_WIDTH, Ufo};
        const DROP_X_MIN: i32 = 0;
        const DROP_X_MAX_FLOOR: i32 = DROP_X_MIN + 1;
        if dest_idx != self.current_tank {
            let name = fish.name.clone();
            self.tanks[dest_idx].place_fish(fish, name, rng);
            return;
        }
        let dest = &self.tanks[dest_idx];
        let max_x = (dest.width as i32 - UFO_SPRITE_WIDTH as i32).max(DROP_X_MAX_FLOOR);
        let x = rng.random_range(DROP_X_MIN..max_x) as f32;
        let target_y = (dest.height as f32 - UFO_SPRITE_HEIGHT as f32).max(0.0);
        self.tanks[dest_idx]
            .ufos
            .push(Ufo::new_drop_fish(x, target_y, fish));
    }

    fn alienate_on_arrival(&self, dest_idx: usize, fish: &mut Fish, rng: &mut impl RngExt) {
        if self.tanks[dest_idx].kind.is_ufo_base() {
            alienate(fish, rng);
        }
    }

    fn plan_cow_delivery(
        &mut self,
        tank_idx: usize,
        variant: crate::entities::cow::CowVariant,
        rng: &mut impl RngExt,
    ) {
        use crate::entities::cow::Cow;
        use crate::entities::ufo::Ufo;
        const UFO_BAY_LEFT_EYE_COL: i32 = 7;
        const COW_HEAD_EYE_OFFSET: i32 = 1;
        let tank = &mut self.tanks[tank_idx];
        let name = tank.unique_cow_name("Vaquita");
        let cow_floor = (tank.height as f32) - (Cow::sprite_height() as f32);
        let probe = Cow::new(name.clone(), variant, 0.0, cow_floor.max(0.0), rng);
        let cow_w = probe.display_width as i32;
        let max_x = (tank.width as i32 - cow_w).max(0);
        let x = pick_cow_drop_x(&tank.cows, cow_w, max_x, rng);
        let cow = Cow {
            position: crate::entities::components::Position {
                x: x as f32,
                y: cow_floor.max(0.0),
            },
            ..probe
        };
        let is_active = tank_idx == self.current_tank;
        if is_active {
            let target_y =
                (tank.height as f32 - crate::entities::ufo::UFO_SPRITE_HEIGHT as f32).max(0.0);
            let ufo_x = x + COW_HEAD_EYE_OFFSET - UFO_BAY_LEFT_EYE_COL;
            tank.ufos
                .push(Ufo::new_drop_cow(ufo_x as f32, target_y, cow));
        } else {
            tank.land_cow_delivery(cow);
        }
    }

    fn take_for_abduction(&mut self, source_idx: usize, pick: usize) -> Fish {
        let fish = self.tanks[source_idx].take_fish(pick);
        self.tanks[source_idx].signal(WorldSignal::Abduction);
        fish
    }

    fn plan_abduction(&mut self, source_idx: usize, rng: &mut impl RngExt) -> bool {
        use crate::entities::ufo::Ufo;
        if self.tanks[source_idx].kind.is_ufo_base() {
            return false;
        }
        let source = &self.tanks[source_idx];
        let abductable: Vec<usize> = source
            .fish
            .iter()
            .enumerate()
            .filter(|(_, f)| f.can_be_abducted() && !source.is_being_abducted(&f.name))
            .map(|(i, _)| i)
            .collect();
        if abductable.is_empty() {
            return false;
        }
        let pick = abductable[rng.random_range(0..abductable.len())];
        let fish_name = self.tanks[source_idx].fish[pick].name.clone();

        let dest_idx = self.abduction_base(source_idx, rng);

        let source_active = source_idx == self.current_tank;

        if source_idx == dest_idx {
            return false;
        }

        if source_active {
            use crate::entities::ufo::{UFO_CENTER_COL, UFO_PAYLOAD_CONE_ROW, UFO_SHIP_ROWS};
            let source = &self.tanks[source_idx];
            let fish_ref = &source.fish[pick];
            let fx =
                fish_ref.position.x + fish_ref.display_width as f32 / 2.0 - UFO_CENTER_COL as f32;
            let target_y =
                (fish_ref.position.y - (UFO_SHIP_ROWS + UFO_PAYLOAD_CONE_ROW) as f32).max(0.0);
            self.tanks[source_idx]
                .ufos
                .push(Ufo::new_abduct(fx, target_y, fish_name.clone()));
            self.pending_ufo_dest.insert(fish_name, dest_idx);
        } else {
            let mut fish = self.take_for_abduction(source_idx, pick);
            self.alienate_on_arrival(dest_idx, &mut fish, rng);
            self.drop_fish(dest_idx, fish, rng);
        }
        true
    }

    fn handle_ufo_take_fish(&mut self, tank_idx: usize, fish_name: &str) {
        let pos = match self.tanks[tank_idx]
            .fish
            .iter()
            .position(|f| f.name == fish_name)
        {
            Some(p) => p,
            None => return,
        };
        let planned = self.pending_ufo_dest.remove(fish_name);
        let mut fish = self.take_for_abduction(tank_idx, pos);
        fish.abduction_lock = false;
        let name = fish.name.clone();
        let mut rng = rand::rng();
        let dest = match planned.filter(|&base| self.is_open_base(base)) {
            Some(base) => base,
            None => self.abduction_base(tank_idx, &mut rng),
        };
        self.alienate_on_arrival(dest, &mut fish, &mut rng);
        self.tanks[dest].place_fish(fish, name, &mut rng);
    }

    fn is_open_base(&self, index: usize) -> bool {
        self.tanks
            .get(index)
            .is_some_and(|tank| tank.kind.is_ufo_base() && !tank.is_full())
    }

    fn abduction_base(&mut self, source_idx: usize, rng: &mut impl RngExt) -> usize {
        match self.choose_abduction_dest(source_idx) {
            Some(base) => base,
            None => self.create_alien_base_tank(rng),
        }
    }

    fn choose_abduction_dest(&self, source_idx: usize) -> Option<usize> {
        const COW_DELIVERIES_PER_NEW_BASE: u32 = 10;
        for (i, t) in self.tanks.iter().enumerate().rev() {
            if i == source_idx {
                continue;
            }
            if self.is_open_base(i) && t.cow_abduction_count < COW_DELIVERIES_PER_NEW_BASE {
                return Some(i);
            }
        }
        None
    }

    fn create_alien_base_tank(&mut self, rng: &mut impl RngExt) -> usize {
        let n = rng.random_range(1000..=9999);
        let base_name = format!("Alien Base #{}", n);
        let name = names::unique_name_in(&self.used_tank_names, &base_name);
        self.used_tank_names.insert(name.clone());
        let mut tank = Tank::new(name, TankKind::Alien, &[]);
        tank.resize(self.terminal_width, self.tank_height(), &[]);
        self.found_tank(tank)
    }

    fn handle_wander(&mut self, source_idx: usize, fish_name: &str) {
        let Some(pos) = self.tanks[source_idx]
            .fish
            .iter()
            .position(|f| f.name == fish_name)
        else {
            return;
        };
        let fish = &self.tanks[source_idx].fish[pos];
        let candidates: Vec<usize> = (0..self.tanks.len())
            .filter(|&i| {
                i != source_idx && !self.tanks[i].is_full() && self.tanks[i].welcomes(fish)
            })
            .collect();
        if candidates.is_empty() {
            return;
        }
        let mut rng = rand::rng();
        let target_idx = candidates[rng.random_range(0..candidates.len())];
        let fish = self.tanks[source_idx].take_fish(pos);
        let name = fish.name.clone();
        self.tanks[target_idx].place_fish(fish, name, &mut rng);
    }

    fn tick_blink(&mut self) {
        self.editor
            .tick_blink(self.settings.fps, self.settings.cursor_blink);
    }

    fn reset_blink(&mut self) {
        self.editor.reset_blink();
        match &mut self.active_overlay {
            Some(Overlay::Shop(s)) => s.reset_blink(),
            Some(Overlay::Catch(s)) => s.reset_blink(),
            _ => {}
        }
    }
}

fn alienate(fish: &mut Fish, rng: &mut impl RngExt) {
    use crate::fishes::mutations::{Mutation, apply_mutation_to_fish};
    apply_mutation_to_fish(fish, Mutation::Alienation, rng);
}

fn pick_cow_drop_x(
    cows: &[crate::entities::cow::Cow],
    cow_w: i32,
    max_x: i32,
    rng: &mut impl RngExt,
) -> i32 {
    if max_x <= 0 {
        return 0;
    }
    const ATTEMPTS: u32 = 32;
    for _ in 0..ATTEMPTS {
        let candidate = rng.random_range(0..=max_x);
        let overlap = cows.iter().any(|c| {
            let c_left = c.position.x as i32;
            let c_right = c_left + c.display_width as i32;
            let cand_right = candidate + cow_w;
            candidate < c_right && cand_right > c_left
        });
        if !overlap {
            return candidate;
        }
    }
    rng.random_range(0..=max_x)
}

#[cfg(test)]
mod catch_tests {
    use super::*;
    use crate::loot::ItemKind;

    const ROLLS: usize = 4_000;

    #[test]
    fn a_full_house_withholds_the_fish_and_nothing_else() {
        let mut app = App::launch(Launch::Debug);
        app.tanks[0] = Tank::new("Zion".to_string(), TankKind::Matrix, &[]);
        let mut rng = rand::rng();
        let capacity = app.tanks[0].capacity();
        for n in app.tanks[0].fish.len()..capacity {
            app.tanks[0].spawn_fish(FishSpecies::Merluza, format!("Full{n}"), &mut rng);
        }
        assert!(!app.has_room_for_a_catch(0));
        let mut robotics = false;
        for _ in 0..ROLLS {
            match app.roll_catch(0, &mut rng) {
                LootKind::Fish(species) => panic!("a full house caught a {species:?}"),
                LootKind::Item(ItemKind::Consumable(ConsumableKind::Part(_))) => robotics = true,
                _ => {}
            }
        }
        assert!(robotics, "a full Matrixtank still gives up its parts");
    }

    #[test]
    fn a_catch_is_only_a_fish_some_tank_would_take() {
        let mut app = App::launch(Launch::Debug);
        let mut rng = rand::rng();
        let capacity = app.tanks[0].capacity();
        for n in app.tanks[0].fish.len()..capacity {
            app.tanks[0].spawn_fish(FishSpecies::Merluza, format!("Full{n}"), &mut rng);
        }
        let heaven = app.found_tank(Tank::new("Heaven".to_string(), TankKind::Heaven, &[]));
        for _ in 0..ROLLS {
            if let LootKind::Fish(species) = app.roll_catch(heaven, &mut rng) {
                assert!(
                    app.has_room_for_a_new(species),
                    "only heaven has room, and it caught a {species:?}"
                );
            }
        }
        assert!(
            app.has_room_for_a_catch(heaven),
            "a Holyfish may still bite"
        );
        assert!(!app.has_room_for_a_catch(0), "but nothing heaven refuses");
    }
}
