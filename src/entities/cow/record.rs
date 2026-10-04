use std::f32::consts::TAU;

use rand::RngExt;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use super::{Cow, CowVariant};
use crate::entities::components::{Position, SwayState};
use crate::fishes::mutant::{MutantState, MutationRecord};
use crate::tank::Sky;

#[derive(Serialize, Deserialize)]
pub struct CowRecord {
    name: String,
    variant: CowVariant,
    color: Color,
    body_length: usize,
    sway_speed: f32,
    mutant: Box<MutantState>,
    mutations: Option<Box<MutationRecord>>,
    display_width: usize,
    engulf_timer: f32,
}

impl From<Cow> for CowRecord {
    fn from(cow: Cow) -> Self {
        let Cow {
            name,
            position: _,
            variant,
            color,
            body_length,
            sway: _,
            sway_speed,
            mutant,
            mutations,
            speech: _,
            display_width,
            engulf_timer,
            sky: _,
        } = cow;
        Self {
            name,
            variant,
            color,
            body_length,
            sway_speed,
            mutant,
            mutations,
            display_width,
            engulf_timer,
        }
    }
}

impl From<CowRecord> for Cow {
    fn from(record: CowRecord) -> Self {
        Self {
            name: record.name,
            position: Position { x: 0.0, y: 0.0 },
            variant: record.variant,
            color: record.color,
            body_length: record.body_length,
            sway: SwayState {
                phase: rand::rng().random::<f32>() * TAU,
            },
            sway_speed: record.sway_speed,
            mutant: record.mutant,
            mutations: record.mutations,
            speech: None,
            display_width: record.display_width,
            engulf_timer: record.engulf_timer,
            sky: Sky::default(),
        }
    }
}
