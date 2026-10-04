use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::cheats::Cheats;
use crate::consumable::{ActiveConsumable, ActiveMilkStatus};
use crate::economy::{Money, Purse};
use crate::fishes::fish::Fish;
use crate::fishes::graveyard::Graveyard;
use crate::fishes::species::FishSpecies;
use crate::ledger::Ledger;
use crate::loot::{LootKind, StockItem};
use crate::settings::Settings;
use crate::tank::{Blueprint, DayClock, Tank, TankKind, TankRecord};
use crate::ui::catch_overlay::CatchState;
use crate::ui::input_action::HeldKeys;
use crate::ui::line_editor::{CommandHistory, LineEditor};
use crate::util::sample_exponential;
use crate::vault::CryptSeal;
use crate::void_ritual::{self, VoidRitualState};

use super::{App, GraceBuff, Lessons, Overlay, TERMINAL_HEIGHT_DEFAULT, TERMINAL_WIDTH_DEFAULT};

pub const SAVE_VERSION: u32 = 2;
pub const STARTING_CASH: Money = 50;
pub const STARTING_FOOD: u32 = 60;
pub const FIRST_TANK_NAME: &str = "Fishtank";
pub const FIRST_FISH: [(FishSpecies, &str); 3] = [
    (FishSpecies::Merluza, "Adam"),
    (FishSpecies::Salmon, "Lilith"),
    (FishSpecies::Goldfish, "Eva"),
];

#[derive(Serialize, Deserialize)]
pub struct PendingCatch {
    loot: LootKind,
    fish: Option<Fish>,
}

#[derive(Serialize, Deserialize)]
pub struct SaveFile {
    version: u32,
    settings: Settings,
    tanks: Vec<TankRecord>,
    current_tank: usize,
    purse: Purse,
    debug_mode: bool,
    cheats: Cheats,
    food_supply: u32,
    inventory: BTreeMap<StockItem, u32>,
    active_consumables: Vec<ActiveConsumable>,
    active_statuses: Vec<ActiveMilkStatus>,
    cajetans_grace: GraceBuff,
    history: CommandHistory,
    #[serde(default)]
    graveyard: Vec<Fish>,
    #[serde(default)]
    crypt: Option<CryptSeal>,
    blueprints: Vec<Blueprint>,
    void_ritual: VoidRitualState,
    next_prayer: usize,
    nothing_stacks: u32,
    day_clock: DayClock,
    catch: Option<PendingCatch>,
    #[serde(default)]
    lessons: Lessons,
}

impl SaveFile {
    pub fn new_game(debug_mode: bool) -> Self {
        let mut rng = rand::rng();
        let mut first_tank = Tank::new(FIRST_TANK_NAME.to_string(), TankKind::Base, &[]);
        for (species, name) in FIRST_FISH {
            first_tank.spawn_fish(species, name.to_string(), &mut rng);
        }
        Self {
            version: SAVE_VERSION,
            settings: Settings::default(),
            tanks: vec![TankRecord::capture(&first_tank)],
            current_tank: 0,
            purse: Purse::holding(STARTING_CASH),
            debug_mode,
            cheats: Cheats::default(),
            food_supply: STARTING_FOOD,
            inventory: BTreeMap::new(),
            active_consumables: Vec::new(),
            active_statuses: Vec::new(),
            cajetans_grace: GraceBuff::default(),
            history: CommandHistory::new(),
            graveyard: Vec::new(),
            crypt: None,
            blueprints: Vec::new(),
            void_ritual: VoidRitualState::Idle {
                timer: sample_exponential(&mut rng, void_ritual::VOID_RITUAL_MEAN_SECS),
            },
            next_prayer: 0,
            nothing_stacks: 0,
            day_clock: DayClock::new(),
            catch: None,
            lessons: Lessons::default(),
        }
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn debug_mode(&self) -> bool {
        self.debug_mode
    }

    pub fn crypt(&self) -> Option<CryptSeal> {
        self.crypt
    }

    pub fn entomb(&mut self, crypt: CryptSeal) {
        self.crypt = Some(crypt);
    }

    pub fn lay_to_rest(&mut self, dead: Vec<Fish>) {
        self.graveyard.extend(dead);
    }

    pub fn to_ron(&self) -> Result<String, ron::Error> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
    }

    pub fn from_ron(text: &str) -> Result<Self, ron::error::SpannedError> {
        ron::Options::default()
            .without_recursion_limit()
            .from_str(text)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Dead {
    Inline,
    InTheCrypt,
}

impl App {
    pub fn snapshot(&self) -> SaveFile {
        self.capture(Dead::Inline)
    }

    pub(super) fn capture(&self, dead: Dead) -> SaveFile {
        let App {
            settings,
            tanks,
            current_tank,
            used_tank_names: _,
            purse,
            ledger: _,
            debug_mode,
            cheats,
            food_supply,
            inventory,
            active_consumables,
            active_statuses,
            cajetans_grace,
            editor: _,
            running: _,
            history,
            active_overlay,
            held_keys: _,
            terminal_height: _,
            terminal_width: _,
            graveyard,
            blueprints,
            void_ritual,
            next_prayer,
            nothing_stacks,
            pending_ufo_dest: _,
            day_clock,
            persistence: _,
            zen: _,
            newer_release: _,
            lessons,
        } = self;
        let food_in_the_water: u32 = tanks.iter().map(TankRecord::food_in_the_water).sum();
        let catch = match active_overlay {
            Some(Overlay::Catch(card)) => Some(PendingCatch {
                loot: card.loot.clone(),
                fish: card.fish.clone(),
            }),
            _ => None,
        };
        SaveFile {
            version: SAVE_VERSION,
            settings: settings.clone(),
            tanks: tanks.iter().map(TankRecord::capture).collect(),
            current_tank: *current_tank,
            purse: *purse,
            debug_mode: *debug_mode,
            cheats: *cheats,
            food_supply: food_supply.saturating_add(food_in_the_water),
            inventory: inventory
                .iter()
                .map(|(&stock, &qty)| (stock, qty))
                .collect(),
            active_consumables: active_consumables.clone(),
            active_statuses: active_statuses.clone(),
            cajetans_grace: cajetans_grace.clone(),
            history: history.clone(),
            graveyard: match dead {
                Dead::Inline => graveyard.to_vec(),
                Dead::InTheCrypt => Vec::new(),
            },
            crypt: None,
            blueprints: blueprints.clone(),
            void_ritual: void_ritual.clone(),
            next_prayer: *next_prayer,
            nothing_stacks: *nothing_stacks,
            day_clock: day_clock.clone(),
            catch,
            lessons: *lessons,
        }
    }

    pub fn resume(save: SaveFile, width: u16, height: u16) -> Self {
        let dead_names: Vec<String> = save.graveyard.iter().map(|f| f.name.clone()).collect();
        let tanks: Vec<Tank> = save
            .tanks
            .into_iter()
            .map(|record| record.restore(width, height, &dead_names))
            .collect();
        let used_tank_names: HashSet<String> = tanks.iter().map(|t| t.name.clone()).collect();
        let current_tank = save.current_tank.min(tanks.len().saturating_sub(1));
        let mut app = Self {
            settings: save.settings,
            tanks,
            current_tank,
            used_tank_names,
            purse: save.purse,
            ledger: Ledger::new(),
            debug_mode: save.debug_mode,
            cheats: save.cheats,
            food_supply: save.food_supply,
            inventory: save.inventory.into_iter().collect(),
            active_consumables: save.active_consumables,
            active_statuses: save.active_statuses,
            cajetans_grace: save.cajetans_grace,
            editor: LineEditor::new(),
            running: true,
            history: save.history,
            active_overlay: None,
            held_keys: HeldKeys::default(),
            terminal_height: height,
            terminal_width: width,
            graveyard: Graveyard::of(save.graveyard),
            blueprints: save.blueprints,
            void_ritual: save.void_ritual,
            next_prayer: save.next_prayer,
            nothing_stacks: save.nothing_stacks,
            pending_ufo_dest: HashMap::new(),
            day_clock: save.day_clock,
            persistence: None,
            zen: false,
            newer_release: false,
            lessons: save.lessons,
        };
        app.hang_the_souls();
        if let Some(catch) = save.catch {
            let card = CatchState::holding(catch.loot, catch.fish, &mut rand::rng());
            app.active_overlay = Some(Overlay::Catch(app.counted(card)));
        }
        app
    }

    pub(super) fn new_game(debug_mode: bool) -> Self {
        Self::resume(
            SaveFile::new_game(debug_mode),
            TERMINAL_WIDTH_DEFAULT,
            TERMINAL_HEIGHT_DEFAULT,
        )
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{Event, KeyCode, KeyEvent};

    use super::*;
    use crate::app::Launch;
    use crate::loot::CashValue;

    fn run(app: &mut App, line: &str) {
        app.editor.set(line.to_string());
        app.handle_input(Event::Key(KeyEvent::from(KeyCode::Enter)));
    }

    fn reopened(app: &App) -> App {
        let text = app.snapshot().to_ron().expect("a save");
        let save = SaveFile::from_ron(&text).expect("that reads back");
        App::resume(save, TERMINAL_WIDTH_DEFAULT, TERMINAL_HEIGHT_DEFAULT)
    }

    fn show_the_card(app: &mut App, loot: LootKind) {
        let card = app.counted(CatchState::new(loot, &mut rand::rng()));
        app.set_overlay(Overlay::Catch(card));
    }

    #[test]
    fn a_catch_left_on_its_card_is_still_waiting_when_the_game_comes_back() {
        let mut app = App::launch(Launch::Player);
        show_the_card(&mut app, LootKind::Cash(CashValue::Hundred));

        let back = reopened(&app);

        assert!(matches!(
            back.catch_state().map(|card| &card.loot),
            Some(LootKind::Cash(CashValue::Hundred))
        ));
    }

    #[test]
    fn a_fish_on_its_card_comes_back_the_same_fish() {
        let mut app = App::launch(Launch::Player);
        show_the_card(&mut app, LootKind::Fish(FishSpecies::Koi));
        let caught = app.catch_state().and_then(|card| card.fish.clone());

        let back = reopened(&app);

        let kept = back.catch_state().and_then(|card| card.fish.clone());
        assert_eq!(
            kept.map(|fish| (fish.pattern_seed, fish.color, fish.weight_g)),
            caught.map(|fish| (fish.pattern_seed, fish.color, fish.weight_g))
        );
    }

    #[test]
    fn a_cow_still_in_the_beam_has_landed_when_the_game_comes_back() {
        let mut app = App::launch(Launch::Debug);
        run(&mut app, "/startcowabduction");
        assert!(app.tanks[0].cows.is_empty(), "the cow is still in the UFO");

        let back = reopened(&app);

        assert_eq!(back.tanks[0].cows.len(), 1);
        assert_eq!(back.tanks[0].cow_abduction_count, 1);
        assert!(back.tanks[0].ufos.is_empty());
    }

    #[test]
    fn a_fish_being_abducted_stays_home() {
        let mut app = App::launch(Launch::Debug);
        let fish_before = app.tanks[0].fish.len();
        run(&mut app, "/startfishabduction");

        let back = reopened(&app);

        assert_eq!(back.tanks[0].fish.len(), fish_before);
        assert!(back.tanks[0].fish.iter().all(|fish| !fish.abduction_lock));
    }
}
