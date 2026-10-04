use rand::RngExt;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

use crate::colors::{DARK_GRAY, GRAY, WHITE};

use crate::entities::components::BlinkTimer;
use crate::entities::glistening::{GlisteningMode, color_for_glisten, derive_glistening_palette};
use crate::fishes::fused::FusedComponent;
use crate::fishes::mutant::{Adornments, Circadian, random_rgb_other};
use crate::fishes::species::Sin;
use crate::sprite::{BodyExtension, EAR_LEFT, EAR_RIGHT, Feet};
use crate::util::{even_indices, sample_exponential};

pub const VOID_SPAWN_MEAN_SECS: f32 = 3600.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum UnfishKind {
    Reversed,
    Doppleganger,
    Phantom,
    Blinker,
    Ball,
    Skull,
    Worm,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UnfishMutationStyle {
    Slime,
    Worm,
}

const WORM_DY_FRACTION: f32 = 0.05;
const DEFAULT_DY_FRACTION: f32 = 0.4;

impl UnfishKind {
    pub fn mutation_style(self) -> UnfishMutationStyle {
        match self {
            UnfishKind::Worm => UnfishMutationStyle::Worm,
            _ => UnfishMutationStyle::Slime,
        }
    }

    pub fn dy_fraction(self) -> f32 {
        match self {
            UnfishKind::Worm => WORM_DY_FRACTION,
            _ => DEFAULT_DY_FRACTION,
        }
    }

    pub fn speed_range(self) -> (f32, f32) {
        match self {
            UnfishKind::Phantom => (0.3, 0.8),
            UnfishKind::Worm => (2.88, 5.04),
            _ => (2.0, 4.0),
        }
    }

    pub fn sin(self) -> Option<Sin> {
        match self {
            UnfishKind::Doppleganger => Some(Sin::Envy),
            _ => None,
        }
    }

    pub fn has_fused_behavior(self) -> bool {
        matches!(
            self,
            UnfishKind::Phantom | UnfishKind::Blinker | UnfishKind::Doppleganger
        )
    }
}

pub const SPAWNABLE_UNFISH: &[UnfishKind] = &[
    UnfishKind::Reversed,
    UnfishKind::Doppleganger,
    UnfishKind::Phantom,
    UnfishKind::Blinker,
    UnfishKind::Ball,
    UnfishKind::Skull,
    UnfishKind::Worm,
];

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlinkerPhase {
    Glistening,
    Invisible,
}

const BLINKER_GLISTENING_MIN: f32 = 4.0;
const BLINKER_GLISTENING_MAX: f32 = 6.4;
const BLINKER_INVISIBLE_MIN: f32 = 4.0;
const BLINKER_INVISIBLE_MAX: f32 = 6.4;

pub const PHANTOM_TELEPORT_MEAN: f32 = 5.0;
pub const PHANTOM_CROSS_TANK_CHANCE: f32 = 0.05;

const WORM_SWAY_INTERVAL: f32 = 0.25;

const EYE_SPEED_MIN: f32 = 0.425;
const EYE_SPEED_MAX: f32 = 2.125;
const EYE_DY_FRACTION: f32 = 0.4;
const EYE_DIR_TIMER_MIN: f32 = 1.5;
const EYE_DIR_TIMER_MAX: f32 = 5.0;
const BALL_CENTER_EYE_CHANCE: f32 = 0.99;
const BALL_MAX_EXTRA_EYES: usize = 4;
const BALL_FILLED_CHANCE: f32 = 0.05;
const SKULL_MAX_EYES: usize = 4;
const SKULL_FILLED_CHANCE: f32 = 0.05;
const SKULL_NO_EYE_CHANCE: f32 = 0.1;

pub const BALL_WIDTH: u16 = 15;
pub const BALL_CENTER_ROW: i32 = 4;
pub const BALL_EYE_ROW: usize = 4;
pub const BALL_EYE_COL: usize = 7;

pub const BALL_BASE: [&str; 9] = [
    "    ,-----.    ",
    "  ,` `    ,`.  ",
    " /  `     ,  \\ ",
    "; `  `      , :",
    "|   `     `   |",
    ":  ,        \\ ;",
    " \\      ` \\  / ",
    "  `. ,   ` ,`  ",
    "    `-----`    ",
];

pub const BALL_INTERIOR: &[(usize, i32, i32)] = &[
    (1, 3, 11),
    (2, 2, 12),
    (3, 2, 12),
    (4, 2, 12),
    (5, 2, 12),
    (6, 2, 12),
    (7, 3, 11),
];

pub const SKULL_WIDTH: u16 = 13;
pub const SKULL_CENTER_ROW: i32 = 3;

pub const SKULL_OPEN: [&str; 6] = [
    "    .---.    ",
    r"   /     \   ",
    r"}\|       |/{",
    "}(|       |){",
    r"}/ \  ^  / \{",
    r"    \___/    ",
];

pub const SKULL_CLOSED: [&str; 6] = [
    "    .---.    ",
    r"   /     \   ",
    " ||       || ",
    " ||       || ",
    r" | \  ^  / | ",
    r"    \___/    ",
];

pub const SKULL_INTERIOR: &[(usize, i32, i32)] = &[(2, 4, 8), (3, 4, 8), (4, 4, 8)];

pub const WORM_DEFAULT_SEGMENTS: usize = 6;
const WORM_SEGMENT_W: usize = 3;
const WORM_EYE_GLYPH: char = '0';
const WORM_TAIL_GLYPH: char = ',';

fn worm_hydra_boundaries(segments: usize, hydra: usize) -> Vec<usize> {
    if segments < 2 || hydra == 0 {
        return Vec::new();
    }
    let avail = segments - 1;
    even_indices(avail, hydra.min(avail))
        .into_iter()
        .map(|slot| 1 + slot)
        .collect()
}

fn worm_body(segments: usize, hydra: usize, pattern: &str) -> String {
    let bounds = worm_hydra_boundaries(segments, hydra);
    let mut body = String::new();
    let mut next = 0;
    for k in 0..segments {
        if next < bounds.len() && bounds[next] == k {
            body.push(WORM_EYE_GLYPH);
            next += 1;
        }
        body.push_str(pattern);
    }
    body
}

fn worm_hydra_cols(segments: usize, hydra: usize, body_start: usize) -> Vec<usize> {
    worm_hydra_boundaries(segments, hydra)
        .into_iter()
        .enumerate()
        .map(|(j, b)| body_start + WORM_SEGMENT_W * b + j)
        .collect()
}

#[derive(Clone, Copy)]
pub struct WormShape {
    pub facing_left: bool,
    pub forward: bool,
    pub segments: usize,
    pub extra_eyes: usize,
    pub is_double: bool,
    pub backwards: bool,
    pub ears: usize,
    pub hydra: usize,
}

pub fn build_worm(shape: &WormShape) -> String {
    let WormShape {
        facing_left,
        forward,
        segments,
        extra_eyes,
        is_double,
        backwards,
        ears,
        hydra,
    } = *shape;
    let head = "(0)".repeat(1 + extra_eyes);
    let left_ears: String = std::iter::repeat_n(EAR_LEFT, ears).collect();
    let right_ears: String = std::iter::repeat_n(EAR_RIGHT, ears).collect();
    if is_double && backwards {
        let body = worm_body(segments, hydra, ",/\\");
        let cap: String = std::iter::repeat_n(WORM_TAIL_GLYPH, head.chars().count()).collect();
        format!("{}{}{},{}", cap, left_ears, body, cap)
    } else if is_double {
        let body = worm_body(segments, hydra, ",/\\");
        format!("{}{}{},{}", head, left_ears, body, head)
    } else if facing_left {
        let pattern = if forward { ",/\\" } else { "\\,/" };
        let body = worm_body(segments, hydra, pattern);
        if forward {
            format!("{}{}{},", head, left_ears, body)
        } else {
            format!("{}{},{}", head, left_ears, body)
        }
    } else {
        let pattern = if forward { ",/\\" } else { "\\,/" };
        let body = worm_body(segments, hydra, pattern);
        format!("{},{}{}", body, right_ears, head)
    }
}

pub fn worm_eye_cols(shape: &WormShape) -> Vec<usize> {
    let WormShape {
        facing_left,
        forward,
        segments,
        extra_eyes,
        is_double,
        backwards,
        ears,
        hydra,
    } = *shape;
    let head_count = 1 + extra_eyes;
    let body_w = WORM_SEGMENT_W * segments + hydra;
    let mut cols = Vec::new();
    if is_double && backwards {
        cols.extend(worm_hydra_cols(
            segments,
            hydra,
            WORM_SEGMENT_W * head_count + ears,
        ));
    } else if is_double {
        for k in 0..head_count {
            cols.push(1 + WORM_SEGMENT_W * k);
        }
        let right_start = WORM_SEGMENT_W * head_count + ears + body_w + 1;
        for k in 0..head_count {
            cols.push(right_start + 1 + WORM_SEGMENT_W * k);
        }
        cols.extend(worm_hydra_cols(
            segments,
            hydra,
            WORM_SEGMENT_W * head_count + ears,
        ));
    } else if facing_left {
        for k in 0..head_count {
            cols.push(1 + WORM_SEGMENT_W * k);
        }
        let body_start = WORM_SEGMENT_W * head_count + ears + if forward { 0 } else { 1 };
        cols.extend(worm_hydra_cols(segments, hydra, body_start));
    } else {
        let head_start = body_w + 1 + ears;
        for k in 0..head_count {
            cols.push(head_start + 1 + WORM_SEGMENT_W * k);
        }
        cols.extend(worm_hydra_cols(segments, hydra, 0));
    }
    cols
}

pub fn worm_display_width(
    segments: usize,
    extra_eyes: usize,
    is_double: bool,
    ears: usize,
    hydra: usize,
) -> usize {
    let head_chars = WORM_SEGMENT_W * (1 + extra_eyes);
    let body_w = WORM_SEGMENT_W * segments + hydra;
    if is_double {
        head_chars + ears + body_w + 1 + head_chars
    } else {
        body_w + 1 + head_chars + ears
    }
}

pub const BLINKER_GLISTEN_SPEED: f32 = 24.0;
pub const BLINKER_GLISTEN_PEAK: f32 = 0.6;
pub const BLINKER_GLISTEN_MID: f32 = 0.1;
pub const BLINKER_BASE_COLOR: Color = DARK_GRAY;
pub const BLINKER_MID_COLOR: Color = GRAY;
pub const BLINKER_PEAK_COLOR: Color = WHITE;

pub const UNFISH_BODY_COLOR: Color = WHITE;
pub const UNFISH_EYE_COLOR: Color = DARK_GRAY;
pub const BALL_HEIGHT: u16 = 9;
pub const SKULL_HEIGHT: u16 = 6;

pub const SLIME_GLISTEN_SPEED_DEFAULT: f32 = 10.0;
pub const SLIME_GLISTEN_SPEED_FAST: f32 = 22.0;
pub const SLIME_GLISTEN_SPEED_SLOW: f32 = 4.0;

pub fn is_multi_row(kind: UnfishKind) -> bool {
    matches!(kind, UnfishKind::Ball | UnfishKind::Skull)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FloatingEye {
    pub row: usize,
    pub row_f: f32,
    pub col: f32,
    pub vel: f32,
    pub vel_y: f32,
    pub blink: BlinkTimer,
    pub color: Option<Color>,
    dir_timer: f32,
}

impl FloatingEye {
    fn new(row: usize, col: f32, rng: &mut impl RngExt) -> Self {
        let speed = rng.random_range(EYE_SPEED_MIN..EYE_SPEED_MAX);
        let angle = rng.random::<f32>() * TAU;
        let vel = angle.cos() * speed;
        let vel_y = angle.sin() * speed * EYE_DY_FRACTION;
        Self {
            row,
            row_f: row as f32,
            col,
            vel,
            vel_y,
            blink: BlinkTimer::entity_eye(rng),
            color: None,
            dir_timer: rng.random_range(EYE_DIR_TIMER_MIN..EYE_DIR_TIMER_MAX),
        }
    }
}

fn try_place_eye(
    eyes: &mut Vec<FloatingEye>,
    interior: &[(usize, i32, i32)],
    exclude: Option<(usize, f32)>,
    rng: &mut impl RngExt,
) {
    const MAX_ATTEMPTS: usize = 30;
    for _ in 0..MAX_ATTEMPTS {
        let idx = rng.random_range(0..interior.len());
        let (row, min_col, max_col) = interior[idx];
        if min_col >= max_col {
            continue;
        }
        let col = rng.random_range(min_col as f32..=max_col as f32);
        let blocked_ex = exclude.is_some_and(|(er, ec)| row == er && (col - ec).abs() < 3.0);
        let blocked = eyes
            .iter()
            .filter(|e| e.row == row)
            .any(|e| (e.col - col).abs() < 3.0);
        if !blocked_ex && !blocked {
            eyes.push(FloatingEye::new(row, col, rng));
            return;
        }
    }
    if let Some((row, col)) = free_spot(eyes, interior, exclude) {
        eyes.push(FloatingEye::new(row, col, rng));
    }
}

type EyeInterior = (&'static [(usize, i32, i32)], Option<(usize, f32)>);

fn free_spot(
    eyes: &[FloatingEye],
    interior: &[(usize, i32, i32)],
    exclude: Option<(usize, f32)>,
) -> Option<(usize, f32)> {
    interior.iter().find_map(|&(row, min_col, max_col)| {
        (min_col..=max_col).map(|col| col as f32).find_map(|col| {
            let blocked_ex = exclude.is_some_and(|(er, ec)| row == er && (col - ec).abs() < 3.0);
            let blocked = eyes
                .iter()
                .filter(|e| e.row == row)
                .any(|e| (e.col - col).abs() < 3.0);
            (!blocked_ex && !blocked).then_some((row, col))
        })
    })
}

fn fill_eyes(
    eyes: &mut Vec<FloatingEye>,
    interior: &[(usize, i32, i32)],
    exclude: Option<(usize, f32)>,
    rng: &mut impl RngExt,
) {
    for &(row, min_col, max_col) in interior {
        let mut col = min_col as f32;
        while col <= max_col as f32 {
            let blocked_ex = exclude.is_some_and(|(er, ec)| row == er && (col - ec).abs() < 3.0);
            let blocked = eyes
                .iter()
                .filter(|e| e.row == row)
                .any(|e| (e.col - col).abs() < 3.0);
            if !blocked_ex && !blocked {
                eyes.push(FloatingEye::new(row, col, rng));
            }
            col += 3.0;
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UnfishState {
    pub kind: UnfishKind,
    pub eye: BlinkTimer,
    pub wings: BlinkTimer,
    pub floating_eyes: Vec<FloatingEye>,
    pub ball_has_center_eye: bool,
    pub blinker_phase: BlinkerPhase,
    pub blinker_timer: f32,
    pub blinker_phase_secs: f32,
    pub phantom_timer: f32,
    pub worm_forward: bool,
    worm_timer: f32,
    pub doppleganger_cloned: bool,
    pub glistening_phase: f32,
    pub slime_body_color: Option<Color>,
    pub slime_glisten_enabled: bool,
    pub slime_glisten_color: Option<Color>,
    pub slime_glisten_mode: GlisteningMode,
    pub slime_glisten_phase: f32,
    pub slime_glisten_speed: f32,
    pub worm_segments: usize,
    pub worm_extra_eyes: usize,
    pub worm_is_double: bool,
    pub worm_backwards: bool,
    pub fused: Vec<FusedComponent>,
    pub worm_eye_colors: Vec<Option<Color>>,
    pub slime_eye_color: Option<Color>,
    pub slime_color_patches: Vec<(usize, Color)>,
    #[serde(alias = "bubble_color")]
    pub wake_color: Option<Color>,
    pub circadian: Circadian,
    pub heterochromia: bool,
    pub ear_count: usize,
    pub ear_color: Option<Color>,
    pub hydra_count: usize,
    pub feet: Option<Feet>,
    pub body_extension: Option<BodyExtension>,
    #[serde(default)]
    pub adornments: Adornments,
}

impl UnfishState {
    pub fn new(kind: UnfishKind, rng: &mut impl RngExt) -> Self {
        let mut floating_eyes: Vec<FloatingEye> = Vec::new();
        let mut ball_has_center_eye = false;

        match kind {
            UnfishKind::Ball => {
                ball_has_center_eye = rng.random::<f32>() < BALL_CENTER_EYE_CHANCE;
                let center = if ball_has_center_eye {
                    Some((BALL_EYE_ROW, BALL_EYE_COL as f32))
                } else {
                    None
                };
                if rng.random::<f32>() < BALL_FILLED_CHANCE {
                    fill_eyes(&mut floating_eyes, BALL_INTERIOR, center, rng);
                } else {
                    let n = rng.random_range(0..=BALL_MAX_EXTRA_EYES);
                    for _ in 0..n {
                        try_place_eye(&mut floating_eyes, BALL_INTERIOR, center, rng);
                    }
                }
            }
            UnfishKind::Skull => {
                if rng.random::<f32>() < SKULL_FILLED_CHANCE {
                    fill_eyes(&mut floating_eyes, SKULL_INTERIOR, None, rng);
                } else if rng.random::<f32>() > SKULL_NO_EYE_CHANCE {
                    let n = rng.random_range(1..=SKULL_MAX_EYES);
                    for _ in 0..n {
                        try_place_eye(&mut floating_eyes, SKULL_INTERIOR, None, rng);
                    }
                }
            }
            _ => {}
        }

        let blinker_phase_secs = rng.random_range(BLINKER_GLISTENING_MIN..BLINKER_GLISTENING_MAX);
        let phantom_timer = if kind == UnfishKind::Phantom {
            sample_exponential(rng, PHANTOM_TELEPORT_MEAN)
        } else {
            0.0
        };
        Self {
            kind,
            eye: BlinkTimer::entity_eye(rng),
            wings: BlinkTimer::wing(rng),
            floating_eyes,
            ball_has_center_eye,
            blinker_phase: BlinkerPhase::Glistening,
            blinker_timer: 0.0,
            blinker_phase_secs,
            phantom_timer,
            worm_forward: true,
            worm_timer: 0.0,
            doppleganger_cloned: false,
            glistening_phase: 0.0,
            slime_body_color: None,
            slime_glisten_enabled: false,
            slime_glisten_color: None,
            slime_glisten_mode: GlisteningMode::Wave,
            slime_glisten_phase: 0.0,
            slime_glisten_speed: SLIME_GLISTEN_SPEED_DEFAULT,
            worm_segments: WORM_DEFAULT_SEGMENTS,
            worm_extra_eyes: 0,
            worm_is_double: false,
            worm_backwards: false,
            fused: Vec::new(),
            worm_eye_colors: Vec::new(),
            slime_eye_color: None,
            slime_color_patches: Vec::new(),
            wake_color: None,
            circadian: Circadian::Neutral,
            heterochromia: false,
            ear_count: 0,
            ear_color: None,
            hydra_count: 0,
            feet: None,
            body_extension: None,
            adornments: Adornments::default(),
        }
    }

    pub fn eye_render_color(&self) -> Color {
        self.slime_eye_color.unwrap_or(UNFISH_EYE_COLOR)
    }

    pub fn line_colors(&self, n: usize, natural: impl Fn(usize) -> Color) -> Vec<Color> {
        if self.slime_glisten_enabled {
            let body = self.slime_body_color.unwrap_or(UNFISH_BODY_COLOR);
            let (base, mid, peak_default) = derive_glistening_palette(body);
            let peak = self.slime_glisten_color.unwrap_or(peak_default);
            return (0..n)
                .map(|i| {
                    color_for_glisten(
                        self.slime_glisten_mode,
                        self.slime_glisten_phase,
                        i,
                        n,
                        base,
                        mid,
                        peak,
                    )
                })
                .collect();
        }
        match self.slime_body_color {
            Some(color) => vec![color; n],
            None => (0..n).map(natural).collect(),
        }
    }

    pub fn wear(&mut self, look: &UnfishState) {
        self.floating_eyes = look.floating_eyes.clone();
        self.ball_has_center_eye = look.ball_has_center_eye;
        self.slime_body_color = look.slime_body_color;
        self.slime_glisten_enabled = look.slime_glisten_enabled;
        self.slime_glisten_color = look.slime_glisten_color;
        self.slime_glisten_mode = look.slime_glisten_mode;
        self.slime_glisten_speed = look.slime_glisten_speed;
        self.worm_segments = look.worm_segments;
        self.worm_extra_eyes = look.worm_extra_eyes;
        self.worm_eye_colors = look.worm_eye_colors.clone();
        self.slime_eye_color = look.slime_eye_color;
        self.slime_color_patches = look.slime_color_patches.clone();
        self.wake_color = look.wake_color;
        self.circadian = look.circadian;
        self.heterochromia = look.heterochromia;
        self.ear_count = look.ear_count;
        self.ear_color = look.ear_color;
        self.hydra_count = look.hydra_count;
        self.feet = look.feet;
        self.body_extension = look.body_extension;
        self.adornments = look.adornments;
    }

    pub fn is_invisible(&self) -> bool {
        self.kind == UnfishKind::Blinker && self.blinker_phase == BlinkerPhase::Invisible
    }

    pub fn tick(&mut self, dt: f32, daylight: bool, rng: &mut impl RngExt) {
        let forced_eye = self.circadian.forced_eye_open(daylight);
        self.eye.tick(dt);
        self.wings.tick(dt);
        if let Some(open) = forced_eye {
            self.eye.is_open = open;
        }

        let interior: &[(usize, i32, i32)] = match self.kind {
            UnfishKind::Ball => BALL_INTERIOR,
            UnfishKind::Skull => SKULL_INTERIOR,
            _ => &[],
        };
        for eye in &mut self.floating_eyes {
            eye.blink.tick(dt);
            if let Some(open) = forced_eye {
                eye.blink.is_open = open;
            }
            eye.dir_timer -= dt;
            if eye.dir_timer <= 0.0 {
                let speed = rng.random_range(EYE_SPEED_MIN..EYE_SPEED_MAX);
                let angle = rng.random::<f32>() * TAU;
                eye.vel = angle.cos() * speed;
                eye.vel_y = angle.sin() * speed * EYE_DY_FRACTION;
                eye.dir_timer = rng.random_range(EYE_DIR_TIMER_MIN..EYE_DIR_TIMER_MAX);
            }
        }
        let min_row = interior.iter().map(|&(r, _, _)| r).min().unwrap_or(0);
        let max_row = interior.iter().map(|&(r, _, _)| r).max().unwrap_or(0);
        let proposed: Vec<(usize, f32, f32, f32, f32)> = self
            .floating_eyes
            .iter()
            .map(|eye| {
                let raw_row_f = eye.row_f + eye.vel_y * dt;
                let (new_row_f, new_vel_y) = if raw_row_f < min_row as f32 {
                    (min_row as f32, eye.vel_y.abs())
                } else if raw_row_f > max_row as f32 {
                    (max_row as f32, -eye.vel_y.abs())
                } else {
                    (raw_row_f, eye.vel_y)
                };
                let new_row = new_row_f.round() as usize;
                let (min_col, max_col) = interior
                    .iter()
                    .find(|&&(r, _, _)| r == new_row)
                    .map(|&(_, mn, mx)| (mn, mx))
                    .unwrap_or_else(|| {
                        let mn = interior.iter().map(|&(_, mn, _)| mn).min().unwrap_or(0);
                        let mx = interior.iter().map(|&(_, _, mx)| mx).max().unwrap_or(0);
                        (mn, mx)
                    });
                let raw_col = eye.col + eye.vel * dt;
                let (new_col, new_vel) = if raw_col < min_col as f32 {
                    (min_col as f32, eye.vel.abs())
                } else if raw_col > max_col as f32 {
                    (max_col as f32, -eye.vel.abs())
                } else {
                    (raw_col, eye.vel)
                };
                (new_row, new_row_f, new_col, new_vel, new_vel_y)
            })
            .collect();
        let current: Vec<(usize, f32)> =
            self.floating_eyes.iter().map(|e| (e.row, e.col)).collect();
        let has_center = self.kind == UnfishKind::Ball && self.ball_has_center_eye;
        for (i, &(p_row, p_row_f, p_col, p_vel, p_vel_y)) in proposed.iter().enumerate() {
            let blocked = current
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .any(|(_, &(cr, cc))| cr == p_row && (cc - p_col).abs() < 3.0)
                || (has_center
                    && p_row == BALL_EYE_ROW
                    && (BALL_EYE_COL as f32 - p_col).abs() < 3.0);
            if !blocked {
                self.floating_eyes[i].row = p_row;
                self.floating_eyes[i].row_f = p_row_f;
                self.floating_eyes[i].col = p_col;
                self.floating_eyes[i].vel = p_vel;
                self.floating_eyes[i].vel_y = p_vel_y;
            }
        }

        if self.kind == UnfishKind::Blinker {
            self.glistening_phase = (self.glistening_phase + dt * BLINKER_GLISTEN_SPEED)
                .rem_euclid(std::f32::consts::TAU);
            self.blinker_timer += dt;
            if self.blinker_timer >= self.blinker_phase_secs {
                self.blinker_timer = 0.0;
                match self.blinker_phase {
                    BlinkerPhase::Glistening => {
                        self.blinker_phase = BlinkerPhase::Invisible;
                        self.blinker_phase_secs =
                            rng.random_range(BLINKER_INVISIBLE_MIN..BLINKER_INVISIBLE_MAX);
                    }
                    BlinkerPhase::Invisible => {
                        self.blinker_phase = BlinkerPhase::Glistening;
                        self.blinker_phase_secs =
                            rng.random_range(BLINKER_GLISTENING_MIN..BLINKER_GLISTENING_MAX);
                    }
                }
            }
        }
        if self.kind == UnfishKind::Worm {
            self.worm_timer += dt;
            if self.worm_timer >= WORM_SWAY_INTERVAL {
                self.worm_timer -= WORM_SWAY_INTERVAL;
                self.worm_forward = !self.worm_forward;
            }
        }
        if self.slime_glisten_enabled {
            self.slime_glisten_phase =
                (self.slime_glisten_phase + dt * self.slime_glisten_speed).rem_euclid(TAU);
        }
    }

    fn eye_interior(&self) -> Option<EyeInterior> {
        let interior: &[(usize, i32, i32)] = match self.kind {
            UnfishKind::Ball => BALL_INTERIOR,
            UnfishKind::Skull => SKULL_INTERIOR,
            _ => return None,
        };
        let center = (self.kind == UnfishKind::Ball && self.ball_has_center_eye)
            .then_some((BALL_EYE_ROW, BALL_EYE_COL as f32));
        Some((interior, center))
    }

    pub fn add_floating_eye(&mut self, rng: &mut impl RngExt) {
        let Some((interior, center)) = self.eye_interior() else {
            return;
        };
        try_place_eye(&mut self.floating_eyes, interior, center, rng);
    }

    pub fn has_room_for_an_eye(&self) -> bool {
        self.eye_interior().is_some_and(|(interior, center)| {
            free_spot(&self.floating_eyes, interior, center).is_some()
        })
    }

    pub fn has_an_eye(&self) -> bool {
        match self.kind {
            UnfishKind::Ball => self.ball_has_center_eye || !self.floating_eyes.is_empty(),
            UnfishKind::Skull => !self.floating_eyes.is_empty(),
            _ => true,
        }
    }

    pub fn worm_eye_count(&self) -> usize {
        let head = 1 + self.worm_extra_eyes;
        let heads = if self.worm_is_double && self.worm_backwards {
            0
        } else if self.worm_is_double {
            head * 2
        } else {
            head
        };
        heads + self.hydra_count
    }

    pub fn resync_worm_eye_colors(&mut self, rng: &mut impl RngExt) {
        if !self.heterochromia {
            return;
        }
        let n = self.worm_eye_count();
        while self.worm_eye_colors.len() < n {
            self.worm_eye_colors
                .push(Some(random_rgb_other(rng, &[self.slime_eye_color])));
        }
        self.worm_eye_colors.truncate(n);
    }

    pub fn worm_eye_render_color(&self, eye_idx: usize) -> Color {
        self.worm_eye_colors
            .get(eye_idx)
            .copied()
            .flatten()
            .or(self.slime_eye_color)
            .unwrap_or(UNFISH_EYE_COLOR)
    }

    pub fn make_heterochromatic(&mut self, rng: &mut impl RngExt) {
        self.heterochromia = true;
        if self.kind == UnfishKind::Worm {
            let n = self.worm_eye_count();
            let shared = self.slime_eye_color;
            self.worm_eye_colors = (0..n)
                .map(|_| Some(random_rgb_other(rng, &[shared])))
                .collect();
            return;
        }
        if self.floating_eyes.is_empty() {
            self.slime_eye_color = Some(random_rgb_other(rng, &[self.slime_eye_color]));
            return;
        }
        for eye in &mut self.floating_eyes {
            eye.color = Some(random_rgb_other(rng, &[eye.color]));
        }
    }

    pub fn recolor_one_eye(&mut self, rng: &mut impl RngExt) {
        if self.kind == UnfishKind::Worm {
            if self.heterochromia {
                self.resync_worm_eye_colors(rng);
                if self.worm_eye_colors.is_empty() {
                    return;
                }
                let idx = rng.random_range(0..self.worm_eye_colors.len());
                self.worm_eye_colors[idx] =
                    Some(random_rgb_other(rng, &[self.worm_eye_colors[idx]]));
            } else {
                self.slime_eye_color = Some(random_rgb_other(rng, &[self.slime_eye_color]));
            }
            return;
        }
        if self.heterochromia && !self.floating_eyes.is_empty() {
            let idx = rng.random_range(0..self.floating_eyes.len());
            self.floating_eyes[idx].color =
                Some(random_rgb_other(rng, &[self.floating_eyes[idx].color]));
            return;
        }
        self.slime_eye_color = Some(random_rgb_other(rng, &[self.slime_eye_color]));
    }
}
