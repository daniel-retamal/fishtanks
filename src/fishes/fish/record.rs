use std::f32::consts::TAU;

use rand::RngExt;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use super::{
    DIRECTION_TIMER_MAX, DIRECTION_TIMER_MIN, DY_FRACTION, Direction, Fish, FishState,
    ZOOMIE_INITIAL_TIMER_MAX, ZOOMIE_INITIAL_TIMER_MIN, random_heading,
};
use crate::entities::components::{Position, SwayState};
use crate::fishes::botfish::BotfishState;
use crate::fishes::mutant::{MutantState, MutationRecord};
use crate::fishes::species::{FishSpecies, SizeCategory};
use crate::fishes::toy::ToyState;
use crate::fishes::unfish::UnfishState;
use crate::tank::Sky;

#[derive(Serialize, Deserialize)]
pub struct Placement {
    position: Position,
    facing: Direction,
}

#[derive(Serialize, Deserialize)]
pub struct FishRecord {
    name: String,
    species: FishSpecies,
    body_size: usize,
    color: Color,
    speed: f32,
    pattern_seed: u64,
    display_width: usize,
    sway_speed: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mutant: Option<Box<MutantState>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mutations: Option<Box<MutationRecord>>,
    weight_g: u32,
    size_category: SizeCategory,
    #[serde(default, skip_serializing_if = "is_default")]
    devil_marked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    unfish_state: Option<Box<UnfishState>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    botfish_state: Option<Box<BotfishState>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    toy: Option<Box<ToyState>>,
    #[serde(default, skip_serializing_if = "is_default")]
    sell_price_bonus_pct: u32,
    #[serde(default, skip_serializing_if = "is_default")]
    frozen: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    engulf_timer: f32,
    blessing_timer: f32,
    #[serde(default, skip_serializing_if = "is_default")]
    pending_rad_mutations: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    placement: Option<Placement>,
    #[serde(default, skip_serializing_if = "is_default")]
    leeched: bool,
}

fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

impl From<Fish> for FishRecord {
    fn from(fish: Fish) -> Self {
        let placement = fish.remembers_its_place().then(|| Placement {
            position: fish.position.clone(),
            facing: fish.facing,
        });
        let Fish {
            name,
            position: _,
            velocity: _,
            sway: _,
            state: _,
            facing: _,
            body_size,
            color,
            speed,
            seek_boost: _,
            species,
            pattern_seed,
            display_width,
            sway_speed,
            mutant,
            mutations,
            weight_g,
            size_category,
            devil_marked,
            unfish_state,
            botfish_state,
            toy,
            sell_price_bonus_pct,
            abduction_lock: _,
            frozen,
            engulf_timer,
            blessing_timer,
            blessing_glow: _,
            pending_rad_mutations,
            speech: _,
            field_cache: _,
            sky: _,
            habits: _,
            leeched,
            direction_timer: _,
            zoomie_timer: _,
            zoomed_secs: _,
        } = fish;
        Self {
            name,
            species,
            body_size,
            color,
            speed,
            pattern_seed,
            display_width,
            sway_speed,
            mutant,
            mutations,
            weight_g,
            size_category,
            devil_marked,
            unfish_state,
            botfish_state,
            toy,
            sell_price_bonus_pct,
            frozen,
            engulf_timer,
            blessing_timer,
            pending_rad_mutations,
            placement,
            leeched,
        }
    }
}

impl From<FishRecord> for Fish {
    fn from(record: FishRecord) -> Self {
        let mut rng = rand::rng();
        let dy_fraction = record
            .unfish_state
            .as_ref()
            .map_or(DY_FRACTION, |unfish| unfish.kind.dy_fraction());
        let (mut velocity, mut facing) = random_heading(record.speed, dy_fraction, &mut rng);
        let mut position = Position { x: 0.0, y: 0.0 };
        if let Some(placement) = record.placement {
            position = placement.position;
            if placement.facing != facing {
                velocity.dx = -velocity.dx;
            }
            facing = placement.facing;
        }
        let mut fish = Self {
            name: record.name,
            position,
            velocity,
            sway: SwayState {
                phase: rng.random::<f32>() * TAU,
            },
            state: FishState::Idle,
            facing,
            body_size: record.body_size,
            color: record.color,
            speed: record.speed,
            seek_boost: 0.0,
            species: record.species,
            pattern_seed: record.pattern_seed,
            display_width: record.display_width,
            sway_speed: record.sway_speed,
            mutant: record.mutant,
            mutations: record.mutations,
            weight_g: record.weight_g,
            size_category: record.size_category,
            devil_marked: record.devil_marked,
            unfish_state: record.unfish_state,
            botfish_state: record.botfish_state,
            toy: record.toy,
            sell_price_bonus_pct: record.sell_price_bonus_pct,
            abduction_lock: false,
            frozen: record.frozen,
            engulf_timer: record.engulf_timer,
            blessing_timer: record.blessing_timer,
            blessing_glow: 0.0,
            pending_rad_mutations: record.pending_rad_mutations,
            speech: None,
            field_cache: Vec::new(),
            sky: Sky::default(),
            habits: Box::default(),
            leeched: record.leeched,
            direction_timer: rng.random_range(DIRECTION_TIMER_MIN..DIRECTION_TIMER_MAX),
            zoomie_timer: rng.random_range(ZOOMIE_INITIAL_TIMER_MIN..ZOOMIE_INITIAL_TIMER_MAX),
            zoomed_secs: 0.0,
        };
        fish.shed_reserved_growths();
        fish
    }
}

impl Fish {
    pub fn remembers_its_place(&self) -> bool {
        self.is_programmable()
    }
}
