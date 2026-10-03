use rand::RngExt;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthChar;

use crate::entities::components::BlinkTimer;
pub use crate::entities::glistening::GlisteningMode;
use crate::fishes::fused::FusedComponent;
use crate::fishes::mutations::Mutation;
use crate::fishes::species::{TAIL_EQUAL, TAIL_WAVE_LEFT, TAIL_WAVE_RIGHT, shut_eye};
use crate::sprite::{BodyExtension, Feet};

const WAVE_THRESHOLD: f32 = 0.8;
pub const EXTRA_BODY_FOR_DOUBLE: usize = 2;
pub const MIN_BODY_CHARS: usize = 2;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Serialize, Deserialize)]
pub enum Circadian {
    #[default]
    Neutral,
    NightOwl,
    HelpedByGod,
}

impl Circadian {
    pub fn asleep(self, daylight: bool) -> bool {
        self == Circadian::NightOwl && daylight
    }

    pub fn forced_eye_open(self, daylight: bool) -> Option<bool> {
        match self {
            Circadian::Neutral => None,
            Circadian::NightOwl => self.asleep(daylight).then_some(false),
            Circadian::HelpedByGod => Some(true),
        }
    }
}

pub const LURE_LEAD: usize = 2;
pub const BILL_LEAD: usize = 2;

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Adornments {
    #[serde(default)]
    pub lure: bool,
    #[serde(default)]
    pub bill: bool,
    #[serde(default)]
    pub dorsal_fin: bool,
    #[serde(default)]
    pub ventral_fin: bool,
    #[serde(default)]
    pub lunar: bool,
    #[serde(default)]
    pub puff: bool,
}

impl Adornments {
    pub fn slot(&mut self, mutation: Mutation) -> Option<&mut bool> {
        match mutation {
            Mutation::Lure => Some(&mut self.lure),
            Mutation::Bill => Some(&mut self.bill),
            Mutation::DorsalFin => Some(&mut self.dorsal_fin),
            Mutation::VentralFin => Some(&mut self.ventral_fin),
            Mutation::Lunar => Some(&mut self.lunar),
            Mutation::Puff => Some(&mut self.puff),
            _ => None,
        }
    }

    pub fn has(mut self, mutation: Mutation) -> bool {
        self.slot(mutation).is_some_and(|grown| *grown)
    }

    pub fn grow(&mut self, mutation: Mutation) {
        if let Some(grown) = self.slot(mutation) {
            *grown = true;
        }
    }

    pub fn lead(self) -> usize {
        usize::from(self.lure) * LURE_LEAD + usize::from(self.bill) * BILL_LEAD
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MutantTail {
    Wide,
    Swaying,
    Curly,
    Narrow,
    Bare,
}

impl MutantTail {
    pub fn display_width(self) -> usize {
        match self {
            MutantTail::Wide => 2,
            MutantTail::Swaying => 2,
            MutantTail::Curly => 3,
            MutantTail::Narrow => 1,
            MutantTail::Bare => 0,
        }
    }

    pub fn chars(self, facing_left: bool, phase: f32) -> Vec<char> {
        match self {
            MutantTail::Bare => Vec::new(),
            MutantTail::Narrow => vec![if facing_left { '<' } else { '>' }],
            MutantTail::Wide => vec!['>', '<'],
            MutantTail::Swaying => {
                let base = if facing_left {
                    TAIL_WAVE_LEFT
                } else {
                    TAIL_WAVE_RIGHT
                };
                if phase.sin() > WAVE_THRESHOLD {
                    let bw = UnicodeWidthChar::width(base).unwrap_or(1);
                    let ww = UnicodeWidthChar::width(TAIL_EQUAL).unwrap_or(1);
                    let mut v = vec![TAIL_EQUAL];
                    v.extend(std::iter::repeat_n(' ', bw.saturating_sub(ww)));
                    v
                } else {
                    vec![base]
                }
            }
            MutantTail::Curly => {
                if facing_left {
                    vec!['>', '<', '{']
                } else {
                    vec!['<', '>', '}']
                }
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EyeState {
    blink: BlinkTimer,
    pub color: Option<Color>,
}

impl EyeState {
    pub fn new(rng: &mut impl RngExt) -> Self {
        Self {
            blink: BlinkTimer::fish_eye(rng),
            color: None,
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.blink.tick(dt);
    }

    pub fn glyph(&self, open: char) -> char {
        if self.blink.is_open {
            open
        } else {
            shut_eye(open)
        }
    }

    pub fn is_open(&self) -> bool {
        self.blink.is_open
    }

    pub fn set_open(&mut self, open: bool) {
        self.blink.is_open = open;
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct MutationRecord {
    pub count: u32,
    pub history: Vec<String>,
    pub partners: Vec<String>,
}

impl MutationRecord {
    pub fn child_of(parent_count: u32, parent_name: &str) -> Self {
        Self {
            count: parent_count,
            history: Vec::new(),
            partners: vec![parent_name.to_string()],
        }
    }

    pub fn has(&self, mutation: Mutation) -> bool {
        self.history.iter().any(|token| token == mutation.token())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MutantState {
    pub left_eyes: Vec<EyeState>,
    pub right_eyes: Vec<EyeState>,
    pub eye_color: Option<Color>,
    pub glistening_mode: GlisteningMode,
    pub glistening_color: Option<Color>,
    pub body_variant: u8,
    pub tail_variant: MutantTail,
    pub mouth_inverted: bool,
    pub color_patches: Vec<(usize, Color)>,
    pub is_double: bool,
    pub backwards: bool,
    pub fused: Vec<FusedComponent>,
    pub double_head_eyes: Vec<EyeState>,
    pub hydra_eyes: Vec<EyeState>,
    #[serde(alias = "bubble_color")]
    pub wake_color: Option<Color>,
    pub circadian: Circadian,
    pub heterochromia: bool,
    pub ear_count: usize,
    pub ear_color: Option<Color>,
    pub feet: Option<Feet>,
    pub body_extension: Option<BodyExtension>,
    #[serde(default)]
    pub adornments: Adornments,
    #[serde(default)]
    pub patterned: bool,
}

impl MutantState {
    pub fn new_for_standard(tail_variant: MutantTail, rng: &mut impl RngExt) -> Self {
        Self {
            left_eyes: vec![EyeState::new(rng)],
            right_eyes: vec![EyeState::new(rng)],
            eye_color: None,
            glistening_mode: GlisteningMode::Wave,
            glistening_color: None,
            body_variant: 0,
            tail_variant,
            mouth_inverted: false,
            color_patches: Vec::new(),
            is_double: false,
            backwards: false,
            fused: Vec::new(),
            double_head_eyes: Vec::new(),
            hydra_eyes: Vec::new(),
            wake_color: None,
            circadian: Circadian::Neutral,
            heterochromia: false,
            ear_count: 0,
            ear_color: None,
            feet: None,
            body_extension: None,
            adornments: Adornments::default(),
            patterned: true,
        }
    }

    pub fn new(body_size: usize, seed: u64, rng: &mut impl RngExt) -> Self {
        let (head_count, tail_count) = pick_eye_counts(body_size, rng);
        let tail_variant = match (seed >> 2) % 3 {
            0 => MutantTail::Wide,
            1 => MutantTail::Swaying,
            _ => MutantTail::Curly,
        };
        Self {
            left_eyes: (0..head_count).map(|_| EyeState::new(rng)).collect(),
            right_eyes: (0..tail_count).map(|_| EyeState::new(rng)).collect(),
            eye_color: None,
            glistening_mode: GlisteningMode::Wave,
            glistening_color: None,
            body_variant: (seed % 4) as u8,
            tail_variant,
            mouth_inverted: false,
            color_patches: Vec::new(),
            is_double: false,
            backwards: false,
            fused: Vec::new(),
            double_head_eyes: Vec::new(),
            hydra_eyes: Vec::new(),
            wake_color: None,
            circadian: Circadian::Neutral,
            heterochromia: false,
            ear_count: 0,
            ear_color: None,
            feet: None,
            body_extension: None,
            adornments: Adornments::default(),
            patterned: false,
        }
    }

    pub fn display_width(&self, body_size: usize) -> usize {
        let max_eyes = self.left_eyes.len().max(self.right_eyes.len());
        let base = if self.is_double && self.backwards {
            2 * self.tail_variant.display_width()
                + body_size
                + max_eyes
                + EXTRA_BODY_FOR_DOUBLE
                + self.double_head_eyes.len()
        } else if self.is_double {
            1 + max_eyes + body_size + EXTRA_BODY_FOR_DOUBLE + self.double_head_eyes.len() + 1
        } else {
            1 + max_eyes + body_size + self.tail_variant.display_width()
        };
        base + self.ear_count + self.hydra_eyes.len() + self.adornments.lead()
    }

    pub fn all_eyes_mut(&mut self) -> impl Iterator<Item = &mut EyeState> {
        self.left_eyes
            .iter_mut()
            .chain(self.right_eyes.iter_mut())
            .chain(self.double_head_eyes.iter_mut())
            .chain(self.hydra_eyes.iter_mut())
    }

    pub fn randomize_all_eye_colors(&mut self, rng: &mut impl RngExt) {
        for e in self.all_eyes_mut() {
            e.color = Some(random_rgb(rng));
        }
    }

    pub fn randomize_one_eye_color(&mut self, rng: &mut impl RngExt) {
        let total = self.left_eyes.len()
            + self.right_eyes.len()
            + self.double_head_eyes.len()
            + self.hydra_eyes.len();
        if total == 0 {
            return;
        }
        let mut idx = rng.random_range(0..total);
        for e in self.all_eyes_mut() {
            if idx == 0 {
                e.color = Some(random_rgb(rng));
                return;
            }
            idx -= 1;
        }
    }

    pub fn eye_render_color(&self, eye: &EyeState, default: Color) -> Color {
        eye.color.or(self.eye_color).unwrap_or(default)
    }

    pub fn tick_eyes(&mut self, dt: f32, daylight: bool) {
        let forced = self.circadian.forced_eye_open(daylight);
        for e in self.all_eyes_mut() {
            e.tick(dt);
            if let Some(open) = forced {
                e.set_open(open);
            }
        }
    }
}

pub fn pick_eye_counts(body_size: usize, rng: &mut impl RngExt) -> (usize, usize) {
    let max_eyes = body_size.saturating_sub(MIN_BODY_CHARS).clamp(1, 4);
    let left = rng.random_range(1..=max_eyes);
    let right = rng.random_range(1..=max_eyes);
    (left, right)
}

pub fn random_rgb(rng: &mut impl RngExt) -> Color {
    const HUES: [(u8, u8, u8); 12] = [
        (255, 0, 0),
        (255, 100, 0),
        (255, 220, 0),
        (150, 255, 0),
        (0, 255, 0),
        (0, 255, 140),
        (0, 220, 255),
        (0, 100, 255),
        (0, 0, 255),
        (120, 0, 255),
        (220, 0, 255),
        (255, 0, 160),
    ];
    let (r, g, b) = HUES[rng.random_range(0..HUES.len())];
    Color::Rgb(r, g, b)
}
