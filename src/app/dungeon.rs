use crossterm::event::Event;
use rand::RngExt;

use crate::dungeon::battle::{Battle, Outcome};
use crate::dungeon::beat::{LogLine, Tone};
use crate::dungeon::expedition::{self, Bench, Expedition, Ground, Next};
use crate::dungeon::fighter::Side;
use crate::dungeon::items::{Item, MEAL_PELLETS, Pack};
use crate::dungeon::life::DeepLife;
use crate::dungeon::map::RoomKind;
use crate::dungeon::muster::{Candidate, Muster, Supply};
use crate::dungeon::room::Act;
use crate::dungeon::screen::{Chart, DungeonState, Lobby, Step, View};
use crate::dungeon::state::FightState;
use crate::dungeon::traits::Trait;
use crate::dungeon::{Above, Difficulty};
use crate::fishes::fish::Fish;
use crate::fishes::graveyard::Fate;
use crate::fishes::unfish::UnfishKind;
use crate::ledger::Flow;
use crate::loot::{ConsumableKind, LootKind, StockItem};
use crate::tank::WorldSignal;
use crate::tank::{FabricationQuote, Tank, TankKind};
use crate::ui::fishing_overlay::FishingState;
use crate::ui::input_action::{InputAction, classify};
use crate::ui::toybox::ToyboxState;
use crate::ui::wiring_panel::WiringPanelState;
use crate::util::today;

use super::{App, Below, Overlay};

impl App {
    pub(super) fn open_dungeon(&mut self) -> bool {
        let state = match self.dungeon.expedition.is_some() {
            true => self.dungeon_state(),
            false => match self.muster() {
                Some(muster) => DungeonState::muster(muster),
                None => return false,
            },
        };
        self.set_overlay(Overlay::Dungeon(Box::new(state)));
        true
    }

    fn muster(&self) -> Option<Muster> {
        let candidates: Vec<Candidate> = self
            .tanks
            .iter()
            .flat_map(|tank| {
                tank.fish
                    .iter()
                    .filter(|fish| !fish.abduction_lock)
                    .map(|fish| Candidate {
                        tank: tank.name.clone(),
                        fish: fish.clone(),
                        picked: false,
                    })
            })
            .collect();
        if candidates.is_empty() {
            return None;
        }
        let supplies = Item::for_the_pack()
            .into_iter()
            .filter_map(|item| {
                let held = match item {
                    Item::Food => self.food_supply / MEAL_PELLETS,
                    Item::Stock(stock) => self.inventory.get(&stock).copied().unwrap_or(0),
                };
                (held > 0).then_some(Supply {
                    item,
                    held,
                    packed: 0,
                })
            })
            .collect();
        Some(Muster::new(candidates, supplies))
    }

    pub fn deep_ground(&self) -> Ground {
        Ground {
            haunts: expedition::haunts(self.graveyard.iter()),
            purse: self.purse.spendable(),
            known: self.dungeon.remembered.clone(),
            every_graeae: !self.graeae_at_home(),
            dives: self.dungeon.dives,
            today: today(),
            wrecks: self.dungeon.wrecks.clone(),
            fabricators: self.stock_of(StockItem::Consumable(ConsumableKind::Fabricator)),
            blueprints: self.blueprints.len(),
        }
    }

    fn dungeon_state(&self) -> DungeonState {
        let Some(expedition) = self.dungeon.expedition.as_ref() else {
            return DungeonState::muster(Muster::new(Vec::new(), Vec::new()));
        };
        DungeonState::of(expedition, fight_of(expedition))
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
        let ground = self.deep_ground();
        let Some(Overlay::Dungeon(mut state)) = self.active_overlay.take() else {
            return;
        };
        let step = state.key(&action, self.dungeon.expedition.as_ref(), &ground);
        self.active_overlay = Some(Overlay::Dungeon(state));
        match step {
            Step::Stay => {}
            Step::Close => self.close_overlay(),
            Step::Played => self.keep_the_record(),
            Step::FightOver => self.settle_fight(),
            Step::Go(index) => {
                let moved = self
                    .dungeon
                    .expedition
                    .as_mut()
                    .is_some_and(|expedition| expedition.go(index, &ground));
                if moved {
                    self.redraw_the_deep(Next::Stay);
                }
            }
            Step::Act(act, fish) => self.act_below(act, fish, &ground),
            Step::SetOut(difficulty) => self.set_out(difficulty, &ground),
        }
    }

    fn act_below(&mut self, act: Act, fish: usize, ground: &Ground) {
        let Some(expedition) = self.dungeon.expedition.as_mut() else {
            return;
        };
        let before = Pulse::of(expedition);
        let next = expedition.act(act, fish, ground);
        let after = Pulse::of(expedition);
        self.uplink(before, after);
        self.beam_home();
        self.follow(next);
    }

    pub(super) fn open_room_below(&mut self, name: &str) -> bool {
        let ground = self.deep_ground();
        let staged = self
            .dungeon
            .expedition
            .as_mut()
            .is_some_and(|expedition| expedition.stage_room(name, &ground));
        staged && self.open_dungeon()
    }

    fn open_bench(&mut self, bench: Bench) {
        let opened = match bench {
            Bench::Program(member) => self.program_below(member),
            Bench::Dress(member) => self.dress_below(member),
            Bench::Print => self.open_foundry(),
        };
        if opened {
            self.below = Some(Below::Workshop);
        }
    }

    fn program_below(&mut self, member: usize) -> bool {
        let Some(fish) = self
            .dungeon
            .expedition
            .as_ref()
            .and_then(|expedition| expedition.party.get(member))
        else {
            return false;
        };
        let Some(script) = fish.script() else {
            return false;
        };
        let state = WiringPanelState::new(self.current_tank, fish.name.clone(), script);
        self.set_overlay(Overlay::Wiring {
            state,
            backed_circuit: None,
        });
        true
    }

    fn dress_below(&mut self, member: usize) -> bool {
        let Some(name) = self
            .dungeon
            .expedition
            .as_ref()
            .and_then(|expedition| expedition.party.get(member))
            .map(|fish| fish.name.clone())
        else {
            return false;
        };
        let party = self.dungeon.expedition.iter().flat_map(|e| e.party.iter());
        let mut state = ToyboxState::new(super::toybox::toys_in(party));
        let Some(at) = state.toys.iter().position(|entry| entry.name == name) else {
            return false;
        };
        state.selected = at;
        state.begin_edit();
        self.set_overlay(Overlay::Toybox(Box::new(state)));
        true
    }

    pub(super) fn print_below(&mut self, index: usize, quote: &FabricationQuote) -> bool {
        let mut bench = Tank::new(String::new(), TankKind::Base, &[]);
        bench.resize(self.terminal_width, self.terminal_height, &[]);
        bench.boundless = true;
        if !bench.print(&self.blueprints[index], &mut rand::rng()) {
            return false;
        }
        let printed: Vec<Fish> = (0..bench.fish.len()).map(|_| bench.take_fish(0)).collect();
        let Some(expedition) = self.dungeon.expedition.as_mut() else {
            return false;
        };
        let names: Vec<String> = printed
            .into_iter()
            .map(|fish| expedition.join(fish))
            .collect();
        expedition.landed(LogLine::new(
            format!("{} joins the party.", names.join(", ")),
            Tone::Heal,
        ));
        self.spend_on(quote);
        true
    }

    pub(super) fn above(&self) -> Above<'_> {
        Above {
            wrecks: &self.dungeon.wrecks,
            fabricators: self.stock_of(StockItem::Consumable(ConsumableKind::Fabricator)),
            blueprints: self.blueprints.len(),
        }
    }

    fn cast_below(&mut self) {
        let milk = self.milk_buffs();
        let mut state = FishingState::new(milk, &mut rand::rng());
        state.no_escape = self.cheats.no_escape;
        state.lesson = self.lessons.learning();
        self.set_overlay(Overlay::Fishing(state));
        self.below = Some(Below::Hole);
    }

    pub(super) fn keep_below(&mut self, loot: LootKind, fish: Option<Fish>, name: String) {
        let Some(expedition) = self.dungeon.expedition.as_mut() else {
            return;
        };
        let said = match (loot, fish) {
            (_, Some(mut fish)) => {
                if !name.is_empty() {
                    fish.name = name;
                }
                if fish.name.is_empty() {
                    fish.name = fish.species.display_name().to_string();
                }
                let name = expedition.join(fish);
                LogLine::new(
                    format!("{name} comes up on the line and joins the party."),
                    Tone::Heal,
                )
            }
            (LootKind::Item(item), None) => match StockItem::from_item(&item) {
                Some(stock) => {
                    expedition.haul.add(stock, 1);
                    LogLine::new(format!("Caught: {}.", stock.display_name()), Tone::Gold)
                }
                None => LogLine::new("It got away.", Tone::Quiet),
            },
            (LootKind::Cash(cash), None) => {
                let gold = u64::from(cash.amount());
                expedition.haul.gold += gold;
                LogLine::new(format!("Caught: ${gold}."), Tone::Gold)
            }
            (LootKind::Food(_) | LootKind::Fish(_), None) => {
                LogLine::new("It got away.", Tone::Quiet)
            }
        };
        expedition.landed(said);
        self.uplink_signal(WorldSignal::Catch);
    }

    pub(super) fn surface(&mut self, below: Below) {
        let Some(expedition) = self.dungeon.expedition.as_mut() else {
            return;
        };
        if below == Below::Hole {
            expedition.cast_spent();
        }
        self.open_dungeon();
    }

    fn beam_home(&mut self) {
        let Some(expedition) = self.dungeon.expedition.as_mut() else {
            return;
        };
        let beamed = std::mem::take(&mut expedition.beamed);
        let mut rng = rand::rng();
        for mut fish in beamed {
            let base = self.abduction_base(self.current_tank, &mut rng);
            super::alienate(&mut fish, &mut rng);
            let name = fish.name.clone();
            self.land_or_wait(base, fish, name);
        }
    }

    fn follow(&mut self, next: Next) {
        match next {
            Next::Raise(name) => {
                self.raise_below(&name);
                self.redraw_the_deep(Next::Stay);
            }
            Next::Home => self.come_home(),
            Next::Cast => self.cast_below(),
            Next::Bench(bench) => self.open_bench(bench),
            next => self.redraw_the_deep(next),
        }
    }

    fn redraw_the_deep(&mut self, next: Next) {
        let Some(expedition) = self.dungeon.expedition.as_ref() else {
            self.close_overlay();
            return;
        };
        let place = expedition.place();
        let in_a_room = matches!(
            self.dungeon_overlay().map(|state| &state.view),
            Some(View::Room(_))
        );
        if in_a_room && place == expedition::Place::Room && next == Next::Stay {
            return;
        }
        let fresh = DungeonState::of(expedition, fight_of(expedition));
        let Some(state) = self.dungeon_overlay_mut() else {
            return;
        };
        state.view = fresh.view;
        state.scroll.reset();
        if let View::Room(lobby) = &mut state.view {
            *lobby = Lobby::default();
        }
        if let View::Map(chart) = &mut state.view {
            *chart = Chart::default();
        }
    }

    fn set_out(&mut self, difficulty: Difficulty, ground: &Ground) {
        let Some(View::Muster(muster)) = self.dungeon_overlay().map(|state| &state.view) else {
            return;
        };
        let picked: Vec<(String, String)> = muster
            .picked()
            .iter()
            .map(|&i| {
                let candidate = &muster.candidates[i];
                (candidate.tank.clone(), candidate.fish.name.clone())
            })
            .collect();
        let packed = muster.packed();
        let mut party = Vec::new();
        let mut homes = Vec::new();
        for (tank_name, fish_name) in picked {
            let Some(tank) = self.tanks.iter().position(|tank| tank.name == tank_name) else {
                continue;
            };
            let Some(index) = self.tanks[tank]
                .fish
                .iter()
                .position(|f| f.name == fish_name)
            else {
                continue;
            };
            party.push(self.tanks[tank].take_fish(index));
            homes.push(tank_name);
        }
        if party.is_empty() {
            return;
        }
        let mut pack = Vec::new();
        for (item, count) in packed {
            let count = match item {
                Item::Food => {
                    let meals = count.min(self.food_supply / MEAL_PELLETS);
                    self.food_supply -= meals * MEAL_PELLETS;
                    meals
                }
                Item::Stock(stock) => {
                    let held = self.inventory.get(&stock).copied().unwrap_or(0);
                    let taken = count.min(held);
                    self.take_from_inventory(stock, taken);
                    taken
                }
            };
            if count > 0 {
                pack.push(Pack::of(item, count));
            }
        }
        let seed: u64 = rand::rng().random();
        let expedition = Expedition::set_out(party, homes, difficulty, pack, seed, ground);
        self.dungeon.dives += 1;
        self.dungeon.expedition = Some(expedition);
        self.redraw_the_deep(Next::Map);
    }

    fn take_from_inventory(&mut self, stock: crate::loot::StockItem, count: u32) {
        let Some(held) = self.inventory.get_mut(&stock) else {
            return;
        };
        *held = held.saturating_sub(count);
        if *held == 0 {
            self.inventory.remove(&stock);
        }
    }

    pub(super) fn tick_dungeon(&mut self) {
        let dt = 1.0 / self.settings.fps;
        if let Some(state) = self.dungeon_overlay_mut() {
            state.tick(dt);
        }
    }

    fn keep_the_record(&mut self) {
        let Some(state) = self.dungeon_overlay_mut().and_then(DungeonState::fight_mut) else {
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

    fn settle_fight(&mut self) {
        let Some(Overlay::Dungeon(state)) = self.active_overlay.as_mut() else {
            return;
        };
        let View::Fight(fight) = std::mem::replace(&mut state.view, View::Map(Chart::default()))
        else {
            return;
        };
        self.remember(&fight.battle);
        let aftermath = fight.battle.finish();
        let won = aftermath.outcome == Outcome::Won;
        let Some(expedition) = self.dungeon.expedition.as_mut() else {
            return;
        };
        let floor = expedition.floor;
        let wreck = expedition.visit.as_ref().map(|visit| visit.kind) == Some(RoomKind::Wreck);
        let before = Pulse::of(expedition);
        let settled = expedition.settle(aftermath);
        let after = Pulse::of(expedition);
        if wreck && won {
            self.dungeon.wrecks.retain(|w| w.floor != floor);
        }
        let died = !settled.dead.is_empty();
        for fish in settled.dead {
            self.bury(fish, Fate::Dive);
        }
        self.uplink(before, after);
        if died {
            self.uplink_signal(WorldSignal::Death);
        }
        self.follow(settled.next);
    }

    fn raise_below(&mut self, name: &str) {
        let Some(grave) = self.graveyard.iter().rposition(|fish| fish.name == name) else {
            if let Some(expedition) = self.dungeon.expedition.as_mut() {
                expedition.fallen.retain(|fallen| fallen != name);
            }
            return;
        };
        let fish = self.exhume(grave);
        let Some(expedition) = self.dungeon.expedition.as_mut() else {
            return;
        };
        let home = expedition.homes.first().cloned().unwrap_or_default();
        expedition.rejoin(fish, home);
    }

    fn come_home(&mut self) {
        self.close_overlay();
        let Some(mut expedition) = self.dungeon.expedition.take() else {
            return;
        };
        let depth = expedition.floor;
        self.dungeon.deepest = self.dungeon.deepest.max(depth);
        if expedition.wiped {
            self.leave_a_wreck(&expedition);
        } else {
            self.bank(&expedition);
        }
        for pack in std::mem::take(&mut expedition.pack) {
            match pack.item {
                Item::Food => self.food_supply += pack.count * MEAL_PELLETS,
                Item::Stock(stock) => {
                    self.stock_up(stock, pack.count);
                }
            }
        }
        let pasture = self.home_tank(expedition.homes.first().map(String::as_str));
        for cow in std::mem::take(&mut expedition.cows) {
            self.land_cow(pasture, cow);
        }
        let homes = expedition.homes.clone();
        for (member, mut fish) in expedition.party.into_iter().enumerate() {
            let life = fish
                .deep
                .get_or_insert_with(|| Box::new(DeepLife::default()));
            life.dives += 1;
            life.deepest = life.deepest.max(depth);
            let name = fish.name.clone();
            let home = self.home_tank(homes.get(member).map(String::as_str));
            self.land_or_wait(home, fish, name);
        }
    }

    fn bank(&mut self, expedition: &Expedition) {
        if expedition.haul.gold > 0 {
            self.earn(expedition.haul.gold, Flow::Treasure);
        }
        for treasure in &expedition.haul.treasures {
            self.stock_up(treasure.stock, treasure.count);
        }
    }

    fn leave_a_wreck(&mut self, expedition: &Expedition) {
        if expedition.haul.is_empty() {
            return;
        }
        let mut wreck = expedition.wreck(&self.deep_ground());
        if let Some(at) = self
            .dungeon
            .wrecks
            .iter()
            .position(|w| w.floor == wreck.floor)
        {
            let older = self.dungeon.wrecks.remove(at);
            wreck.haul.absorb(older.haul);
        }
        self.dungeon.wrecks.push(wreck);
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

    fn uplink(&mut self, before: Pulse, after: Pulse) {
        if after.treasures > before.treasures {
            self.uplink_signal(WorldSignal::Catch);
        }
        if after.joined > before.joined {
            self.uplink_signal(WorldSignal::Birth);
        }
    }

    fn uplink_signal(&mut self, signal: WorldSignal) {
        let Some(expedition) = self.dungeon.expedition.as_ref() else {
            return;
        };
        let homes: Vec<usize> = expedition
            .kits()
            .iter()
            .enumerate()
            .filter(|(_, kit)| kit.traits.contains(&Trait::Uplink))
            .map(|(member, _)| self.home_tank(expedition.home_of(member)))
            .collect();
        for home in homes {
            self.tanks[home].signal(signal);
        }
    }
}

#[derive(Clone, Copy)]
struct Pulse {
    treasures: u32,
    joined: u32,
}

impl Pulse {
    fn of(expedition: &Expedition) -> Pulse {
        Pulse {
            treasures: expedition.haul.count(),
            joined: expedition.joined,
        }
    }
}

fn fight_of(expedition: &Expedition) -> Option<FightState> {
    let fresh = expedition
        .fight
        .as_ref()
        .is_some_and(|record| record.inputs.is_empty());
    let (battle, opening) = expedition.battle()?;
    let title = expedition.floor_title();
    Some(match fresh {
        true => FightState::fresh(battle, opening, title),
        false => FightState::resumed(battle, title),
    })
}
