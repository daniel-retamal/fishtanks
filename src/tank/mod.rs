use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};

use rand::RngExt;

use ratatui::style::Color;

use crate::colors::{CYAN, GREEN, LIGHT_GREEN, LIGHT_YELLOW, PINK, RED, WHITE};

use crate::economy::{Money, Purchasable, Rarity, Sellable};
use crate::entities::bubble::{Bubble, BubbleSpawner};
use crate::entities::components::Position;
use crate::entities::cow::{Cow, CowVariant};
use crate::entities::food::Food;
use crate::entities::ufo::Ufo;
use crate::fishes::fish::{Fish, FishState};
use crate::fishes::parts::Part;
use crate::fishes::species::FishSpecies;
use crate::fishes::unfish::VOID_SPAWN_MEAN_SECS;
use crate::names;
use crate::settings::Settings;
use crate::tanks::candy::man_sway_offset;
use crate::tanks::soul_wall::SoulWall;
use crate::util::{Metronome, sample_exponential};

mod background;
mod blueprint;
mod channels;
mod fabric;
mod mothership;
mod mutations;
mod netlist;
mod record;
mod relay;
mod simulation;
mod world;

pub use background::{Scenery, TankBackground};
pub use blueprint::{
    Blueprint, BlueprintFish, BlueprintPins, Fabrication, FabricationQuote, FabricationRefusal,
    Material, Workshop,
};
pub use channels::{ChannelRegistry, Wires};
pub use fabric::StageBudget;
pub use mothership::{
    ALIEN_INK, ALIEN_NAME_WORDS_MAX, ALIEN_NAME_WORDS_MIN, ALIEN_SOUNDS, ALIEN_TONGUE, alien_name,
    speech_ink,
};
pub use netlist::{Netlist, Settling};
pub use record::TankRecord;
pub use relay::{Link, Transmission};
pub use world::{
    DAWN_HOUR, DAY_LENGTH_SECS, DUSK_HOUR, DayClock, HOUR_SECS, HOURS_PER_DAY, RAD_TANK_RADS,
    Selector, SensedFish, Superlative, WorldSignal, WorldView,
};

pub enum TankEvent {
    PhantomCrossTank { fish_name: String },
    Blessing,
    UfoTimerFired,
    UfoLockFish { fish_name: String },
    UfoTakeFish { fish_name: String },
    UfoReleaseFish(Box<Fish>),
    UfoReleaseCow(Box<Cow>),
    UfoFinished,
    CallHome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TankKind {
    Base,
    CoralReef,
    Hell,
    Void,
    Alien,
    Haunted,
    Candy,
    Desert,
    Rad,
    Matrix,
    Heaven,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Afterlife {
    Blessed,
    Damned,
}

impl Afterlife {
    pub fn of(fish: &Fish) -> Self {
        if fish.devil_marked {
            Afterlife::Damned
        } else {
            Afterlife::Blessed
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UfoRole {
    DeliversCows,
    AbductsAtNight,
}

pub struct TankConfig {
    pub display_name: &'static str,
    pub buy_price: u32,
    pub capacity: usize,
    pub bubble_color: Color,
    pub rarity: Rarity,
    pub bubble_rate_mult: f32,
    pub auto_mutate_all: bool,
    pub robotics_loot: bool,
    pub buyable: bool,
    pub unique: bool,
    pub sellable: bool,
    pub holy_only: bool,
    pub marks_for_devil: bool,
    pub devils_luck: bool,
    pub afterlife: Option<Afterlife>,
    pub irradiates_milk: bool,
    pub connects: bool,
    pub ufo_frequency_mult: f32,
    pub ufo_role: Option<UfoRole>,
    pub spawns_unfish: bool,
    pub hosts_ritual: bool,
}

impl TankKind {
    pub fn config(self) -> TankConfig {
        match self {
            TankKind::Base => TankConfig {
                display_name: "Fishtank",
                buy_price: 3000,
                capacity: 50,
                bubble_color: CYAN,
                rarity: Rarity::Common,
                bubble_rate_mult: 1.0,
                auto_mutate_all: false,
                buyable: true,
                robotics_loot: false,
                unique: false,
                sellable: true,
                holy_only: false,
                marks_for_devil: false,
                devils_luck: false,
                afterlife: None,
                irradiates_milk: false,
                connects: false,
                ufo_frequency_mult: 1.0,
                ufo_role: None,
                spawns_unfish: false,
                hosts_ritual: false,
            },
            TankKind::CoralReef => TankConfig {
                display_name: "Coralreeftank",
                buy_price: 5000,
                capacity: 50,
                bubble_color: CYAN,
                rarity: Rarity::Rare,
                bubble_rate_mult: 1.0,
                auto_mutate_all: false,
                buyable: true,
                robotics_loot: false,
                unique: false,
                sellable: true,
                holy_only: false,
                marks_for_devil: false,
                devils_luck: false,
                afterlife: None,
                irradiates_milk: false,
                connects: false,
                ufo_frequency_mult: 1.0,
                ufo_role: None,
                spawns_unfish: false,
                hosts_ritual: false,
            },
            TankKind::Hell => TankConfig {
                display_name: "Helltank",
                buy_price: 8000,
                capacity: 100,
                bubble_color: RED,
                rarity: Rarity::Legendary,
                bubble_rate_mult: 1.0,
                auto_mutate_all: false,
                buyable: false,
                robotics_loot: false,
                unique: false,
                sellable: true,
                holy_only: false,
                marks_for_devil: true,
                devils_luck: true,
                afterlife: Some(Afterlife::Damned),
                irradiates_milk: false,
                connects: false,
                ufo_frequency_mult: 1.0,
                ufo_role: None,
                spawns_unfish: false,
                hosts_ritual: false,
            },
            TankKind::Void => TankConfig {
                display_name: "Voidtank",
                buy_price: 8000,
                capacity: 100,
                bubble_color: CYAN,
                rarity: Rarity::Legendary,
                bubble_rate_mult: VOID_BUBBLE_RATE_MULT,
                auto_mutate_all: false,
                buyable: false,
                robotics_loot: false,
                unique: true,
                sellable: true,
                holy_only: false,
                marks_for_devil: false,
                devils_luck: false,
                afterlife: None,
                irradiates_milk: false,
                connects: false,
                ufo_frequency_mult: 1.0,
                ufo_role: None,
                spawns_unfish: true,
                hosts_ritual: true,
            },
            TankKind::Alien => TankConfig {
                display_name: "Alientank",
                buy_price: 5000,
                capacity: 100,
                bubble_color: CYAN,
                rarity: Rarity::Rare,
                bubble_rate_mult: 1.0,
                auto_mutate_all: false,
                buyable: false,
                robotics_loot: false,
                unique: false,
                sellable: true,
                holy_only: false,
                marks_for_devil: false,
                devils_luck: false,
                afterlife: None,
                irradiates_milk: false,
                connects: false,
                ufo_frequency_mult: 1.0,
                ufo_role: Some(UfoRole::DeliversCows),
                spawns_unfish: false,
                hosts_ritual: false,
            },
            TankKind::Haunted => TankConfig {
                display_name: "Hauntedtank",
                buy_price: 5000,
                capacity: 50,
                bubble_color: WHITE,
                rarity: Rarity::Rare,
                bubble_rate_mult: 1.0,
                auto_mutate_all: false,
                buyable: true,
                robotics_loot: false,
                unique: false,
                sellable: true,
                holy_only: false,
                marks_for_devil: false,
                devils_luck: false,
                afterlife: None,
                irradiates_milk: false,
                connects: false,
                ufo_frequency_mult: 1.0,
                ufo_role: None,
                spawns_unfish: false,
                hosts_ritual: false,
            },
            TankKind::Candy => TankConfig {
                display_name: "Candytank",
                buy_price: 5000,
                capacity: 50,
                bubble_color: PINK,
                rarity: Rarity::Rare,
                bubble_rate_mult: 1.0,
                auto_mutate_all: false,
                buyable: true,
                robotics_loot: false,
                unique: false,
                sellable: true,
                holy_only: false,
                marks_for_devil: false,
                devils_luck: false,
                afterlife: None,
                irradiates_milk: false,
                connects: false,
                ufo_frequency_mult: 1.0,
                ufo_role: None,
                spawns_unfish: false,
                hosts_ritual: false,
            },
            TankKind::Desert => TankConfig {
                display_name: "Desertank",
                buy_price: 5000,
                capacity: 50,
                bubble_color: CYAN,
                rarity: Rarity::Rare,
                bubble_rate_mult: 1.0,
                auto_mutate_all: false,
                buyable: true,
                robotics_loot: false,
                unique: false,
                sellable: true,
                holy_only: false,
                marks_for_devil: false,
                devils_luck: false,
                afterlife: None,
                irradiates_milk: false,
                connects: false,
                ufo_frequency_mult: UFO_DESERT_FREQUENCY_MULT,
                ufo_role: Some(UfoRole::AbductsAtNight),
                spawns_unfish: false,
                hosts_ritual: false,
            },
            TankKind::Rad => TankConfig {
                display_name: "Radioactivetank",
                buy_price: 8000,
                capacity: 100,
                bubble_color: LIGHT_GREEN,
                rarity: Rarity::Legendary,
                bubble_rate_mult: RAD_BUBBLE_RATE_MULT,
                auto_mutate_all: true,
                buyable: false,
                robotics_loot: false,
                unique: false,
                sellable: true,
                holy_only: false,
                marks_for_devil: false,
                devils_luck: false,
                afterlife: None,
                irradiates_milk: true,
                connects: false,
                ufo_frequency_mult: 1.0,
                ufo_role: None,
                spawns_unfish: false,
                hosts_ritual: false,
            },
            TankKind::Matrix => TankConfig {
                display_name: "Matrixtank",
                buy_price: 8000,
                capacity: 100,
                bubble_color: GREEN,
                rarity: Rarity::Legendary,
                bubble_rate_mult: 1.0,
                auto_mutate_all: false,
                buyable: false,
                robotics_loot: true,
                unique: false,
                sellable: true,
                holy_only: false,
                marks_for_devil: false,
                devils_luck: false,
                afterlife: None,
                irradiates_milk: false,
                connects: true,
                ufo_frequency_mult: 1.0,
                ufo_role: None,
                spawns_unfish: false,
                hosts_ritual: false,
            },
            TankKind::Heaven => TankConfig {
                display_name: "Heaventank",
                buy_price: 8000,
                capacity: 100,
                bubble_color: LIGHT_YELLOW,
                rarity: Rarity::Legendary,
                bubble_rate_mult: 1.0,
                auto_mutate_all: false,
                buyable: false,
                robotics_loot: false,
                unique: true,
                sellable: true,
                holy_only: true,
                marks_for_devil: false,
                devils_luck: false,
                afterlife: Some(Afterlife::Blessed),
                irradiates_milk: false,
                connects: false,
                ufo_frequency_mult: 1.0,
                ufo_role: None,
                spawns_unfish: false,
                hosts_ritual: false,
            },
        }
    }

    pub fn all_buyable() -> Vec<TankKind> {
        Self::all()
            .iter()
            .copied()
            .filter(|kind| kind.config().buyable)
            .collect()
    }
    pub fn display_name(self) -> &'static str {
        self.config().display_name
    }
    pub fn is_ufo_base(self) -> bool {
        self.config().ufo_role == Some(UfoRole::DeliversCows)
    }
    pub fn buy_price(self) -> u32 {
        self.config().buy_price
    }
    pub fn sell_price(self) -> u32 {
        let config = self.config();
        if !config.sellable {
            return 0;
        }
        if config.buyable {
            return config.buy_price * TANK_RESALE_PERCENT / PERCENT_WHOLE;
        }
        config.rarity.catch_worth()
    }
}

impl Purchasable for TankKind {
    fn buy_price(&self) -> u32 {
        TankKind::buy_price(*self)
    }
    fn display_name(&self) -> &str {
        TankKind::display_name(*self)
    }
}

impl Sellable for TankKind {
    fn sell_price(&self) -> u32 {
        TankKind::sell_price(*self)
    }
    fn display_name(&self) -> &str {
        TankKind::display_name(*self)
    }
}

impl TankKind {
    pub fn parse(s: &str) -> Option<Self> {
        let normalized: String = s.to_ascii_lowercase().split_whitespace().collect();
        Self::all()
            .iter()
            .find(|&&k| k.config().display_name.to_ascii_lowercase() == normalized.as_str())
            .copied()
    }

    pub fn all() -> &'static [TankKind] {
        &[
            TankKind::Base,
            TankKind::CoralReef,
            TankKind::Hell,
            TankKind::Void,
            TankKind::Alien,
            TankKind::Haunted,
            TankKind::Candy,
            TankKind::Desert,
            TankKind::Rad,
            TankKind::Matrix,
            TankKind::Heaven,
        ]
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Arrival {
    Fish(Box<Fish>),
    Cow(Box<Cow>),
}

const TANK_RESALE_PERCENT: u32 = 50;
const PERCENT_WHOLE: u32 = 100;
const SEEK_BOOST_GROWTH: f32 = 0.8;
const FOOD_SPAWN_SPREAD: f32 = 15.0;
pub const FEED_PORTION: usize = 2 * FOOD_SPAWN_SPREAD as usize;
const INITIAL_WIDTH: u16 = 80;
const INITIAL_HEIGHT: u16 = 24;
const SEEK_DX_DEADZONE: f32 = 0.5;
const SEEK_NORM_MIN: f32 = 0.01;
const SEEK_DY_MULTIPLIER: f32 = 1.2;
const SEEK_BOOST_INITIAL_MAX: f32 = 0.2;
const ZOOMIE_BUBBLES_PER_SEC: f32 = 13.5;
const MUTATION_INTERVAL_BASE: f32 = 30.0 * 60.0;
const MUTATION_TIMER_UNARMED: f32 = f32::INFINITY;
const MUTATION_ALPHA: f32 = 1.0 / 3.0;
const MUTATION_MEAN_FLOOR_SECS: f32 = 3.0;
const RAD_BUBBLE_RATE_MULT: f32 = 2.5;
const VOID_BUBBLE_RATE_MULT: f32 = 0.0;
const RAD_MUTATION_MEAN_SECS: f32 = 30.0;
const RAD_AUTO_MUTANT_MEAN_SECS: f32 = 5.0;
const RAD_WEIGHT_INTERVAL_SECS: f32 = 5.0;
const RAD_WEIGHT_GAIN_G: u32 = 1;
const RAD_MILK_MUTATIONS_PER_SEC: f32 = 3.0;
const MIN_SPLIT_BODY_SIZE: usize = 2;
pub const UFO_MEAN_SECS: f32 = 60.0 * 60.0;
const UFO_DESERT_FREQUENCY_MULT: f32 = 2.0;

fn ufo_mean_secs(kind: TankKind) -> f32 {
    UFO_MEAN_SECS / kind.config().ufo_frequency_mult
}
const FISH_SPAWN_X_MIN: f32 = 5.0;
const FISH_SPAWN_X_MAX_OFFSET: f32 = 15.0;
const FISH_SPAWN_X_SAFE_MIN: f32 = 6.0;
const FISH_SPAWN_Y_MIN: f32 = 2.0;
const FISH_SPAWN_Y_MAX_OFFSET: f32 = 5.0;
const FISH_SPAWN_Y_SAFE_MIN: f32 = 3.0;

pub struct Tank {
    pub name: String,
    pub kind: TankKind,
    seed: u64,
    scenery: Scenery,
    pub fish: Vec<Fish>,
    pub cows: Vec<Cow>,
    pub food: Vec<Food>,
    pub background: TankBackground,
    pub channels: ChannelRegistry,
    pub bubbles: Vec<Bubble>,
    pub width: u16,
    pub height: u16,
    pub used_names: HashSet<String>,
    pub used_cow_names: HashSet<String>,
    pub pending_star_cash: Money,
    pub pending_graveyard: Vec<Fish>,
    pub pending_loose_parts: Vec<Part>,
    pub pending_arrivals: Vec<Arrival>,
    pending_signals: BTreeSet<WorldSignal>,
    pub extra_capacity: u32,
    pub boundless: bool,
    pub cow_abduction_count: u32,
    bubble_spawner: BubbleSpawner,
    mutation_timer: f32,
    rad_weight_timer: f32,
    void_spawn_timer: f32,
    pub ufo_timer: f32,
    pub ufos: Vec<Ufo>,
    pub(super) candy_tick: u32,
    candy_scan: Metronome,
    milk_clock: Metronome,
}

impl Tank {
    pub fn new(name: String, kind: TankKind, dead_names: &[String]) -> Self {
        Self::seeded(name, kind, dead_names, rand::rng().random())
    }

    fn seeded(name: String, kind: TankKind, dead_names: &[String], seed: u64) -> Self {
        let mut rng = rand::rng();
        let mut scenery = Scenery::new(seed);
        let background = TankBackground::new(kind, dead_names, &mut scenery);
        Self {
            name,
            kind,
            seed,
            scenery,
            fish: Vec::new(),
            cows: Vec::new(),
            food: Vec::new(),
            background,
            channels: ChannelRegistry::new(),
            bubbles: Vec::new(),
            width: INITIAL_WIDTH,
            height: INITIAL_HEIGHT,
            used_names: HashSet::new(),
            used_cow_names: HashSet::new(),
            pending_star_cash: 0,
            pending_graveyard: Vec::new(),
            pending_loose_parts: Vec::new(),
            pending_arrivals: Vec::new(),
            pending_signals: BTreeSet::new(),
            extra_capacity: 0,
            boundless: false,
            cow_abduction_count: 0,
            bubble_spawner: BubbleSpawner::new(&mut rng),
            mutation_timer: MUTATION_TIMER_UNARMED,
            rad_weight_timer: RAD_WEIGHT_INTERVAL_SECS,
            void_spawn_timer: sample_exponential(&mut rng, VOID_SPAWN_MEAN_SECS),
            ufo_timer: sample_exponential(&mut rng, ufo_mean_secs(kind)),
            ufos: Vec::new(),
            candy_tick: 0,
            candy_scan: Metronome::default(),
            milk_clock: Metronome::default(),
        }
    }

    pub fn capacity(&self) -> usize {
        self.shown_capacity().unwrap_or(usize::MAX)
    }

    pub fn shown_capacity(&self) -> Option<usize> {
        (!self.boundless).then(|| self.kind.config().capacity + self.extra_capacity as usize)
    }

    pub fn expand(&mut self, amount: u32) {
        self.extra_capacity += amount;
    }

    pub fn room(&self) -> usize {
        self.capacity()
            .saturating_sub(self.fish.len() + self.incoming_fish())
    }

    pub fn is_full(&self) -> bool {
        self.room() == 0
    }

    pub fn has_room_for(&self, fish: &Fish) -> bool {
        self.welcomes(fish) && !self.is_full()
    }

    pub fn resize(&mut self, width: u16, height: u16, dead_names: &[String]) {
        let old_width = self.width;
        self.width = width;
        self.height = height;
        let max_y = height as f32 - 1.0;
        for fish in &mut self.fish {
            if fish.position.y > max_y {
                fish.position.y = max_y;
                if fish.velocity.dy > 0.0 {
                    fish.velocity.dy = -fish.velocity.dy;
                }
            }
        }
        for food in &mut self.food {
            if food.position.y > max_y {
                food.position.y = max_y;
            }
        }
        let cow_floor = (height as f32) - (crate::entities::cow::Cow::sprite_height() as f32);
        for cow in &mut self.cows {
            cow.position.y = cow_floor.max(0.0);
            let max_x = (width as i32 - cow.display_width as i32).max(0) as f32;
            if cow.position.x > max_x {
                cow.position.x = max_x;
            }
        }
        let mut rng = rand::rng();
        if width > old_width {
            self.background
                .extend(self.width, dead_names, &mut self.scenery);
        }
        self.background.init_stars(&mut rng, width, height);
    }

    pub fn clear_grave_name(&mut self, name: &str) {
        self.background.clear_grave_name(name);
    }

    pub fn candy_man_sway(&self) -> i32 {
        man_sway_offset(self.candy_tick)
    }

    pub(super) fn mark_for_devil(&self, fish: &mut Fish) {
        if self.kind.config().marks_for_devil && fish.species.config().markable {
            fish.devil_marked = true;
        }
    }

    pub fn welcomes(&self, fish: &Fish) -> bool {
        if !self.kind.config().holy_only {
            return true;
        }
        fish.is_holy() && !fish.devil_marked
    }

    pub fn welcomes_cows(&self) -> bool {
        !self.kind.config().holy_only
    }

    pub(super) fn admit(&mut self, mut fish: Fish, name: String) {
        if !self.welcomes(&fish) {
            fish.name = name;
            self.pending_arrivals.push(Arrival::Fish(Box::new(fish)));
            return;
        }
        self.mark_for_devil(&mut fish);
        self.used_names.insert(name);
        self.fish.push(fish);
        self.signal(WorldSignal::Birth);
    }

    pub fn spawn_fish(
        &mut self,
        species: FishSpecies,
        name: String,
        rng: &mut impl RngExt,
    ) -> bool {
        if self.is_full() {
            return false;
        }
        let actual_name = self.unique_name(&name);
        let Position { x, y } = self.spawn_point(rng);
        let fish = Fish::new(species, actual_name.clone(), x, y, rng);
        if !self.welcomes(&fish) {
            return false;
        }
        self.admit(fish, actual_name);
        true
    }

    fn spawn_point(&self, rng: &mut impl RngExt) -> Position {
        let x_max = (self.width as f32 - FISH_SPAWN_X_MAX_OFFSET).max(FISH_SPAWN_X_SAFE_MIN);
        let y_max = (self.height as f32 - FISH_SPAWN_Y_MAX_OFFSET).max(FISH_SPAWN_Y_SAFE_MIN);
        Position {
            x: rng.random_range(FISH_SPAWN_X_MIN..x_max),
            y: rng.random_range(FISH_SPAWN_Y_MIN..y_max),
        }
    }

    pub fn take_fish(&mut self, index: usize) -> Fish {
        let fish = self.fish.remove(index);
        self.used_names.remove(&fish.name);
        fish
    }

    fn admit_cow(&mut self, cow: Cow) {
        if !self.welcomes_cows() {
            self.pending_arrivals.push(Arrival::Cow(Box::new(cow)));
            return;
        }
        self.used_cow_names.insert(cow.name.clone());
        self.cows.push(cow);
    }

    pub fn receive_soul(&mut self, fish: Fish) -> bool {
        let (width, height) = (self.width, self.height);
        self.background
            .receive_soul(fish, width, height, &mut rand::rng())
    }

    pub fn release_soul(&mut self, name: &str) -> bool {
        let (width, height) = (self.width, self.height);
        self.background
            .release_soul(name, width, height, &mut rand::rng())
    }

    pub fn soul_wall(&self) -> Option<&SoulWall> {
        self.background.soul_wall()
    }

    pub fn souls(&self) -> Vec<&Fish> {
        self.soul_wall()
            .map(|wall| wall.all().collect())
            .unwrap_or_default()
    }

    pub fn soul_count(&self) -> usize {
        self.soul_wall().map_or(0, SoulWall::len)
    }

    pub fn take_souls(&mut self) -> Vec<Fish> {
        self.background.take_souls()
    }

    pub fn place_fish(&mut self, mut fish: Fish, name: String, rng: &mut impl RngExt) {
        let actual_name = self.unique_name(&name);
        fish.position = self.spawn_point(rng);
        fish.name = actual_name.clone();
        self.admit(fish, actual_name);
    }

    pub fn place_fish_dropped(&mut self, mut fish: Fish) {
        let actual_name = self.unique_name(&fish.name);
        let max_x = (self.width as f32 - fish.display_width as f32).max(0.0);
        let max_y = (self.height as f32 - 1.0).max(0.0);
        fish.position.x = fish.position.x.clamp(0.0, max_x);
        fish.position.y = fish.position.y.clamp(0.0, max_y);
        fish.name = actual_name.clone();
        self.admit(fish, actual_name);
    }

    pub fn feed(&mut self, count: usize, food_supply: &mut u32) -> bool {
        if self.width == 0 {
            return false;
        }
        let actual = count.min(*food_supply as usize);
        if actual == 0 {
            return false;
        }
        *food_supply -= actual as u32;
        let mut rng = rand::rng();
        let spread = FOOD_SPAWN_SPREAD;
        let min_center = spread;
        let max_center = (self.width as f32 - 1.0 - spread).max(spread + 1.0);
        let center_x = rng.random_range(min_center..max_center);
        for _ in 0..actual {
            let offset = rng.random_range(-spread..spread);
            let x = (center_x + offset).clamp(0.0, self.width as f32 - 1.0);
            self.food.push(Food::new(x));
        }
        true
    }

    pub fn tick(&mut self, settings: &Settings, coffee: u32) -> Vec<TankEvent> {
        let dt = 1.0 / settings.fps;
        let mut rng = rand::rng();

        self.background.tick(dt, &mut rng, self.width, self.height);
        for food in &mut self.food {
            food.tick(settings, self.width, self.height);
        }
        let star: u32 = self
            .bubbles
            .iter_mut()
            .map(|b| b.tick(settings, self.width))
            .sum();
        self.pending_star_cash += Money::from(star);
        self.bubbles.retain(|b| !b.dead);

        self.candy_tick = self.candy_tick.wrapping_add(1);
        self.steer_seeking_fish();
        self.tick_fish(settings, coffee);
        self.spawn_bubbles(dt);
        self.tick_candyfish_effects(dt);
        self.check_eating_collisions();

        let prev_len = self.food.len();
        self.food.retain(|f| !f.eaten);
        if self.food.len() < prev_len {
            for fish in &mut self.fish {
                if matches!(fish.state, FishState::SeekingFood { .. }) {
                    fish.cancel_seek();
                }
            }
        }

        self.assign_food_to_idle_fish();
        self.tick_mutations(dt);
        self.tick_irradiated_milk_mutations(dt);
        self.tick_dopplegangers();
        self.tick_cows(dt);
        self.tick_engulfment();
        if self.kind.config().spawns_unfish {
            self.tick_void_spawn(dt, &mut rng);
        }
        let mut events = self.tick_phantoms(dt, &mut rng);
        events.extend(self.tick_blessings(dt));
        self.tick_ufo_timer(dt, &mut rng, &mut events);
        self.tick_calls_home(dt, &mut rng, &mut events);
        self.tick_ufo_animation(dt, &mut events);
        events
    }

    fn tick_ufo_timer(&mut self, dt: f32, rng: &mut impl RngExt, events: &mut Vec<TankEvent>) {
        self.ufo_timer -= dt;
        if self.ufo_timer <= 0.0 {
            self.ufo_timer = sample_exponential(rng, ufo_mean_secs(self.kind));
            events.push(TankEvent::UfoTimerFired);
        }
    }

    fn tick_ufo_animation(&mut self, dt: f32, events: &mut Vec<TankEvent>) {
        use crate::entities::ufo::UfoTickResult;
        for ufo in &mut self.ufos {
            Self::follow_abductee(ufo, &self.fish);
            events.push(match ufo.tick(dt) {
                UfoTickResult::None => continue,
                UfoTickResult::LockFish(name) => TankEvent::UfoLockFish { fish_name: name },
                UfoTickResult::TakeFish(name) => TankEvent::UfoTakeFish { fish_name: name },
                UfoTickResult::ReleaseFish(f) => TankEvent::UfoReleaseFish(Box::new(f)),
                UfoTickResult::ReleaseCow(c) => TankEvent::UfoReleaseCow(Box::new(c)),
                UfoTickResult::Finished => TankEvent::UfoFinished,
            });
        }
        self.ufos.retain(|ufo| !ufo.is_done());
    }

    fn follow_abductee(ufo: &mut Ufo, fish: &[Fish]) {
        use crate::entities::ufo::{
            UFO_CENTER_COL, UFO_PAYLOAD_CONE_ROW, UFO_SHIP_ROWS, UfoPayload, UfoPhase,
        };
        if let UfoPayload::AbductingFish { fish_name } = &ufo.payload
            && let Some(fish) = fish.iter().find(|f| &f.name == fish_name)
        {
            let target_x =
                fish.position.x + fish.display_width as f32 / 2.0 - UFO_CENTER_COL as f32;
            let target_y = fish.position.y - (UFO_SHIP_ROWS + UFO_PAYLOAD_CONE_ROW) as f32;
            match ufo.phase {
                UfoPhase::Descending => {
                    ufo.x = target_x;
                    ufo.target_y = target_y;
                }
                UfoPhase::GrowingCone { .. } => {
                    ufo.x = target_x;
                    ufo.y = target_y;
                }
                _ => {}
            }
        }
    }

    pub fn is_being_abducted(&self, name: &str) -> bool {
        self.ufos.iter().any(|ufo| ufo.abductee() == Some(name))
    }

    pub fn incoming_fish(&self) -> usize {
        self.ufos
            .iter()
            .filter(|ufo| ufo.carried_fish().is_some())
            .count()
    }

    fn unique_name(&self, requested: &str) -> String {
        names::unique_name_in(&self.used_names, requested)
    }

    pub fn unique_cow_name(&self, requested: &str) -> String {
        names::unique_name_in(&self.used_cow_names, requested)
    }

    pub fn spawn_cow(&mut self, variant: CowVariant, rng: &mut impl RngExt) -> String {
        let name = self.unique_cow_name("Vaquita");
        let cow = Cow::new(name.clone(), variant, 0.0, 0.0, rng);
        let cow_floor = (self.height as f32) - (Cow::sprite_height() as f32);
        let x = self.cow_spot(&cow, rng);
        self.admit_cow(Cow {
            position: Position {
                x,
                y: cow_floor.max(0.0),
            },
            ..cow
        });
        name
    }

    pub fn place_cow_dropped(&mut self, mut cow: Cow) {
        cow.name = self.unique_cow_name(&cow.name);
        let cow_floor = (self.height as f32) - (Cow::sprite_height() as f32);
        cow.position.y = cow_floor.max(0.0);
        let max_x = (self.width as i32 - cow.display_width as i32).max(0) as f32;
        if cow.position.x > max_x {
            cow.position.x = max_x;
        }
        if cow.position.x < 0.0 {
            cow.position.x = 0.0;
        }
        self.admit_cow(cow);
    }

    pub fn land_cow_delivery(&mut self, cow: Cow) {
        self.place_cow_dropped(cow);
        self.cow_abduction_count = self.cow_abduction_count.saturating_add(1);
    }

    pub fn place_cow(&mut self, mut cow: Cow, rng: &mut impl RngExt) {
        cow.name = self.unique_cow_name(&cow.name);
        let cow_floor = (self.height as f32) - (Cow::sprite_height() as f32);
        cow.position.x = self.cow_spot(&cow, rng);
        cow.position.y = cow_floor.max(0.0);
        self.admit_cow(cow);
    }

    fn cow_spot(&self, cow: &Cow, rng: &mut impl RngExt) -> f32 {
        let max_x = (self.width as i32 - cow.display_width as i32).max(0);
        if max_x > 0 {
            rng.random_range(0..max_x) as f32
        } else {
            0.0
        }
    }

    pub fn cow_count_by_variant(&self, variant: CowVariant) -> u32 {
        self.cows.iter().filter(|c| c.variant == variant).count() as u32
    }

    pub fn has_cow(&self) -> bool {
        !self.cows.is_empty()
    }

    fn tick_cows(&mut self, dt: f32) {
        for cow in &mut self.cows {
            cow.tick(dt);
        }
    }
}
