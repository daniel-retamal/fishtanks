use std::f32::consts::TAU;

use rand::RngExt;
use ratatui::style::Color;

use super::components::{Position, SwayState, tick_sway};
use crate::settings::Settings;

const RISE_SPEED_MIN: f32 = 2.0;
const RISE_SPEED_MAX: f32 = 5.0;
const SURFACE_RISE_SPEED_MIN: f32 = 0.5;
const SURFACE_RISE_SPEED_MAX: f32 = 1.5;
const SWAY_SPEED: f32 = 0.02;
const SWAY_AMOUNT: f32 = 0.4;
const HOVER_TIME_MIN: f32 = 0.5;
const HOVER_TIME_MAX: f32 = 1.5;
const SURFACE_LIFETIME_MIN: f32 = 3.0;
const SURFACE_LIFETIME_MAX: f32 = 8.0;
const BOTTOM_SPAWN_RATE_MIN: f32 = 0.2;
const BOTTOM_SPAWN_RATE_MAX: f32 = 1.0;
const SURFACE_SPAWN_RATE_MIN: f32 = 0.67;
const SURFACE_SPAWN_RATE_MAX: f32 = 2.0;

const CHARS: [char; 3] = ['°', '◦', '○'];

#[derive(Copy, Clone)]
pub enum BubblePhase {
    Rising { hover_time: f32 },
    Hovering { remaining: f32 },
    Surface { remaining: f32 },
}

impl BubblePhase {
    pub fn rising(rng: &mut impl RngExt) -> Self {
        Self::Rising {
            hover_time: rng.random_range(HOVER_TIME_MIN..HOVER_TIME_MAX),
        }
    }

    pub fn surface(rng: &mut impl RngExt) -> Self {
        Self::Surface {
            remaining: rng.random_range(SURFACE_LIFETIME_MIN..SURFACE_LIFETIME_MAX),
        }
    }
}

pub struct Bubble {
    pub position: Position,
    pub bubble_char: char,
    pub color: Color,
    pub dead: bool,
    pub cash_value: Option<u32>,
    pub poppable: bool,
    sway: SwayState,
    base_x: f32,
    rise_speed: f32,
    phase: BubblePhase,
}

impl Bubble {
    pub fn new(
        x: f32,
        y: f32,
        phase: BubblePhase,
        color: Color,
        ch: Option<char>,
        rng: &mut impl RngExt,
    ) -> Self {
        let rise_speed = match phase {
            BubblePhase::Rising { .. } | BubblePhase::Hovering { .. } => {
                rng.random_range(RISE_SPEED_MIN..RISE_SPEED_MAX)
            }
            BubblePhase::Surface { .. } => {
                rng.random_range(SURFACE_RISE_SPEED_MIN..SURFACE_RISE_SPEED_MAX)
            }
        };
        Self {
            position: Position { x, y },
            sway: SwayState {
                phase: rng.random::<f32>() * TAU,
            },
            base_x: x,
            rise_speed,
            bubble_char: ch.unwrap_or_else(|| CHARS[rng.random_range(0..CHARS.len())]),
            color,
            dead: false,
            cash_value: None,
            poppable: true,
            phase,
        }
    }

    pub fn tick(&mut self, settings: &Settings, tank_width: u16) -> u32 {
        if self.dead {
            return 0;
        }
        let dt = 1.0 / settings.fps;

        tick_sway(&mut self.sway, SWAY_SPEED);
        self.position.x = (self.base_x + SWAY_AMOUNT * self.sway.phase.sin())
            .clamp(0.0, (tank_width as f32 - 1.0).max(0.0));

        match self.phase {
            BubblePhase::Rising { hover_time } => {
                self.position.y -= self.rise_speed * dt;
                if self.position.y <= 0.0 {
                    self.position.y = 0.0;
                    self.phase = BubblePhase::Hovering {
                        remaining: hover_time,
                    };
                    return self.cash_value.unwrap_or(0);
                }
            }
            BubblePhase::Hovering { remaining } => {
                let new_rem = remaining - dt;
                if new_rem <= 0.0 {
                    self.dead = true;
                } else {
                    self.phase = BubblePhase::Hovering { remaining: new_rem };
                }
            }
            BubblePhase::Surface { remaining } => {
                self.position.y -= self.rise_speed * dt;
                let new_rem = remaining - dt;
                if new_rem <= 0.0 || self.position.y <= 0.0 {
                    self.dead = true;
                } else {
                    self.phase = BubblePhase::Surface { remaining: new_rem };
                }
            }
        }
        0
    }
}

pub struct BubbleSpawner {
    bottom_timer: f32,
    surface_timer: f32,
}

impl BubbleSpawner {
    pub fn new(rng: &mut impl RngExt) -> Self {
        Self {
            bottom_timer: rng.random_range(BOTTOM_SPAWN_RATE_MIN..BOTTOM_SPAWN_RATE_MAX),
            surface_timer: rng.random_range(SURFACE_SPAWN_RATE_MIN..SURFACE_SPAWN_RATE_MAX),
        }
    }

    pub fn tick(
        &mut self,
        dt: f32,
        width: u16,
        height: u16,
        color: Color,
        rng: &mut impl RngExt,
    ) -> Vec<Bubble> {
        let mut bubbles = Vec::new();

        self.bottom_timer -= dt;
        if self.bottom_timer <= 0.0 {
            let x = rng.random_range(0.0..width as f32);
            let y = (height as f32 - 1.0).max(0.0);
            bubbles.push(Bubble::new(
                x,
                y,
                BubblePhase::rising(rng),
                color,
                None,
                rng,
            ));
            self.bottom_timer = rng.random_range(BOTTOM_SPAWN_RATE_MIN..BOTTOM_SPAWN_RATE_MAX);
        }

        self.surface_timer -= dt;
        if self.surface_timer <= 0.0 {
            let x = rng.random_range(0.0..width as f32);
            let max_y = (height as f32 * 0.25).max(1.0);
            let y = rng.random_range(0.0..max_y);
            bubbles.push(Bubble::new(
                x,
                y,
                BubblePhase::surface(rng),
                color,
                None,
                rng,
            ));
            self.surface_timer = rng.random_range(SURFACE_SPAWN_RATE_MIN..SURFACE_SPAWN_RATE_MAX);
        }

        bubbles
    }
}
