use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

use rand::{RngExt, SeedableRng, rngs::SmallRng};
use ratatui::style::Color;
use unicode_width::UnicodeWidthChar;

use super::botfish::{ANTENNA_LENGTH, ANTENNA_STALK, ANTENNA_TIP, BODY_COLOR, BotfishState};
use super::fused::FusedComponent;
use super::habits::Habits;
use super::mutant::{
    Adornments, Circadian, EXTRA_BODY_FOR_DOUBLE, MutantState, MutantTail, MutationRecord,
};
use super::mutations::{grow_birthmarks, native_eyes, tail_kind_to_mutant_tail};
use super::species::{
    BodyChars, BodyFill, BodySource, BodyTemplate, Cycle, EYE_CIRCLE, EYE_ROUND, EyeAt,
    FishSpecies, Habit, Locomotion, PatternKind, Sin, SizeCategory, Skin, TailKind, Zoomie,
};
use super::unfish::{
    BALL_WIDTH, BLINKER_BASE_COLOR, BLINKER_GLISTEN_MID, BLINKER_GLISTEN_PEAK, BLINKER_MID_COLOR,
    BLINKER_PEAK_COLOR, SKULL_WIDTH, UNFISH_BODY_COLOR, UnfishKind, UnfishState,
    WORM_DEFAULT_SEGMENTS, WormShape, build_worm, is_multi_row, worm_display_width, worm_eye_cols,
};
use crate::colors::{PINK, WHITE};
use crate::consumable::{COFFEE_SPEED_MULT, COFFEE_SWAY_MULT, COFFEE_ZOOMIE_DT_MULT};
use crate::economy::Money;
use crate::entities::components::{Position, SwayState, Velocity, tick_sway};
use crate::entities::glistening::{GlisteningMode, color_for_glisten, derive_glistening_palette};
use crate::entities::speech::SpeechBubble;
use crate::loot::{StockItem, junk_cell};
use crate::settings::Settings;
use crate::sprite::{
    Band, BodyExtension, EAR_LEFT, EAR_RIGHT, Feet, PosedExtension, ear_glyph, feet_row,
    mirror_char, painted_span,
};
use crate::tank::Sky;
use crate::util::even_indices;

mod adorn;
mod appetite;
mod locomotion;
mod record;

pub use adorn::{MOON_DARK, MOON_LIT};
pub use appetite::{BURP, Fed};
pub use locomotion::{PUFF_SECS, SHY_HIDING_SECS};
use record::FishRecord;

const CHAR_SPREAD: f32 = 1.0;
const PERCENT_WHOLE: Money = 100;
pub const EATING_DURATION: f32 = 0.15;
const MOUTH_WIDTH: i32 = 1;
const BAND_WIDTH: usize = 2;
const PATTERN_HALVES: usize = 2;
const SPECKLE_ODDS: u32 = 3;
const ZONE_HEAD: usize = 0;
const ZONE_BODY: usize = 1;
const ZONE_TAIL: usize = 2;
const ZOOMIE_COOLDOWN_MIN: f32 = 120.0;
const ZOOMIE_COOLDOWN_MAX: f32 = 180.0;
const ZOOMIE_SPEED_MULTIPLIER: f32 = 11.0;
const ZOOMIE_SWAY_MULTIPLIER: f32 = 4.0;
const ZOOMIE_DURATION_MIN: f32 = 0.8;
const ZOOMIE_DURATION_MAX: f32 = 1.2;
const ZOOMIE_TURN_THRESHOLD: f32 = 0.35;
const DY_FRACTION: f32 = 0.4;
const WAVE_THRESHOLD: f32 = 0.8;
const VELOCITY_NORM_MIN: f32 = 0.01;
const DIRECTION_TIMER_MIN: u32 = 180;
const DIRECTION_TIMER_MAX: u32 = 480;
const DIRECTION_TIMER_POST_EVENT_MIN: u32 = 60;
const DIRECTION_TIMER_POST_EVENT_MAX: u32 = 180;
const ZOOMIE_INITIAL_TIMER_MIN: f32 = 30.0;
const ZOOMIE_INITIAL_TIMER_MAX: f32 = 90.0;
const DIRECTION_TIMER_DISPLAY_DEFAULT: u32 = 300;
const ZOOMIE_TIMER_DISPLAY_DEFAULT: f32 = 60.0;
const LINE_TAIL_W: usize = 2;
const HYDRA_EYES_PER_HEAD: usize = 2;
const HELPEDBYGOD_COFFEE_MULT: f32 = 2.0;
const HELPEDBYGOD_ZOOMIE_MULT: f32 = 3.0;
const NIGHTOWL_SPEED_MULT: f32 = 0.4;
const NIGHTOWL_SINK_DY: f32 = 1.5;
const HUE_SHIFT_SECS: f32 = 3.0;
const FEET_ROW_COUNT: usize = 1;
const STONES: [char; 3] = ['o', 'O', '0'];
pub const ENGULF_WINDOW_SECS: f32 = 10.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Direction {
    Left,
    Right,
}

impl Direction {
    fn flip(self) -> Self {
        match self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

pub struct LineSprite {
    pub rows: Vec<Vec<(char, Color)>>,
    pub body_row: usize,
}

impl LineSprite {
    fn shut_eyes(&mut self) {
        for cell in self.rows.iter_mut().flatten() {
            cell.0 = crate::fishes::species::shut_eye(cell.0);
        }
    }
}

type LineCells = (Vec<(char, Color)>, Option<(usize, usize)>);

#[derive(Clone, Copy)]
pub enum FishState {
    Idle,
    SeekingFood {
        food_idx: usize,
        approach_right: bool,
    },
    Eating {
        time_remaining: f32,
    },
    Zoomie {
        time_remaining: f32,
        total_duration: f32,
        will_turn: bool,
        has_turned: bool,
    },
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(into = "FishRecord", from = "FishRecord")]
pub struct Fish {
    pub name: String,
    pub position: Position,
    pub velocity: Velocity,
    pub sway: SwayState,
    pub state: FishState,
    pub facing: Direction,
    pub body_size: usize,
    pub color: Color,
    pub speed: f32,
    pub seek_boost: f32,
    pub species: FishSpecies,
    pub pattern_seed: u64,
    pub display_width: usize,
    pub sway_speed: f32,
    pub mutant: Option<Box<MutantState>>,
    pub mutations: Option<Box<MutationRecord>>,
    pub weight_g: u32,
    pub size_category: SizeCategory,
    pub devil_marked: bool,
    pub unfish_state: Option<Box<UnfishState>>,
    pub botfish_state: Option<Box<BotfishState>>,
    pub sell_price_bonus_pct: u32,
    pub abduction_lock: bool,
    pub frozen: bool,
    pub engulf_timer: f32,
    pub blessing_timer: f32,
    pub blessing_glow: f32,
    pub pending_rad_mutations: u32,
    pub speech: Option<SpeechBubble>,
    pub field_cache: Vec<Option<(String, Option<Color>)>>,
    pub sky: Sky,
    pub habits: Box<Habits>,
    direction_timer: u32,
    zoomie_timer: f32,
    zoomed_secs: f32,
}

pub const BLESSING_INTERVAL_SECS: f32 = 33.0 * 60.0;
pub const BLESSING_GLOW_SECS: f32 = 3.0;
const BLESSING_GLOW_SPEED: f32 = 14.0;

fn roll_size_category(rng: &mut impl RngExt) -> SizeCategory {
    let mut v = rng.random_range(0..SizeCategory::ODDS_TOTAL);
    for size in SizeCategory::ALL {
        if v < size.odds() {
            return size;
        }
        v -= size.odds();
    }
    SizeCategory::XL
}

struct BodyFields {
    body_size: usize,
    weight_g: u32,
    speed: f32,
    pattern_seed: u64,
    color: Color,
    mutant: Option<Box<MutantState>>,
    display_width: usize,
    sway_speed: f32,
}

fn random_heading(speed: f32, dy_fraction: f32, rng: &mut impl RngExt) -> (Velocity, Direction) {
    let angle = rng.random::<f32>() * TAU;
    let dx = angle.cos() * speed;
    let dy = angle.sin() * speed * dy_fraction;
    let facing = if dx < 0.0 {
        Direction::Left
    } else {
        Direction::Right
    };
    (Velocity { dx, dy }, facing)
}

fn programmable_state(species: FishSpecies) -> Option<Box<BotfishState>> {
    species
        .config()
        .programmable
        .then(|| Box::new(BotfishState::new()))
}

fn init_body_fields(
    species: FishSpecies,
    size_cat: SizeCategory,
    rng: &mut impl RngExt,
) -> BodyFields {
    let config = species.config();
    let body_size = match config.body {
        BodyTemplate::Fixed { .. } | BodyTemplate::Figure(_) => 0,
        _ => config.sizes[size_cat as usize],
    };
    let weight_g = config.weight_base[size_cat as usize];
    let speed = rng.random_range(config.speed_range.0..config.speed_range.1);
    let pattern_seed: u64 = rng.random();
    let color = if config.auto_mutate {
        FishSpecies::mutant_color_for_seed(pattern_seed)
    } else {
        config.palette[rng.random_range(0..config.palette.len())]
    };
    let mutant = if config.auto_mutate {
        Some(Box::new(MutantState::new(body_size, pattern_seed, rng)))
    } else {
        native_eyes(species, rng)
    };
    let display_width = mutant
        .as_ref()
        .map(|mutant| mutant.display_width(body_size))
        .unwrap_or_else(|| compute_display_width(species, body_size));
    BodyFields {
        body_size,
        weight_g,
        speed,
        pattern_seed,
        color,
        mutant,
        display_width,
        sway_speed: config.sway_speed,
    }
}

impl Fish {
    pub fn new(species: FishSpecies, name: String, x: f32, y: f32, rng: &mut impl RngExt) -> Self {
        let size_cat = if species.config().auto_mutate {
            SizeCategory::M
        } else {
            roll_size_category(rng)
        };
        let fields = init_body_fields(species, size_cat, rng);
        let botfish_state = programmable_state(species);
        let (velocity, facing) = random_heading(fields.speed, DY_FRACTION, rng);
        let mut fish = Self {
            name,
            position: Position { x, y },
            velocity,
            sway: SwayState {
                phase: rng.random::<f32>() * TAU,
            },
            state: FishState::Idle,
            facing,
            body_size: fields.body_size,
            color: fields.color,
            speed: fields.speed,
            seek_boost: 0.0,
            direction_timer: rng.random_range(DIRECTION_TIMER_MIN..DIRECTION_TIMER_MAX),
            zoomie_timer: rng.random_range(ZOOMIE_INITIAL_TIMER_MIN..ZOOMIE_INITIAL_TIMER_MAX),
            zoomed_secs: 0.0,
            species,
            pattern_seed: fields.pattern_seed,
            display_width: fields.display_width,
            sway_speed: fields.sway_speed,
            mutant: fields.mutant,
            mutations: None,
            weight_g: fields.weight_g,
            size_category: size_cat,
            devil_marked: false,
            unfish_state: None,
            botfish_state,
            sell_price_bonus_pct: 0,
            abduction_lock: false,
            frozen: false,
            engulf_timer: 0.0,
            blessing_timer: BLESSING_INTERVAL_SECS,
            blessing_glow: 0.0,
            pending_rad_mutations: 0,
            speech: None,
            field_cache: Vec::new(),
            sky: Sky::default(),
            habits: Box::default(),
        };
        grow_birthmarks(&mut fish, rng);
        fish
    }

    pub fn new_for_display(species: FishSpecies, rng: &mut impl RngExt) -> Self {
        let fields = init_body_fields(species, SizeCategory::M, rng);
        let mut fish = Self {
            name: String::new(),
            position: Position { x: 0.0, y: 0.0 },
            velocity: Velocity {
                dx: fields.speed,
                dy: 0.0,
            },
            sway: SwayState {
                phase: rng.random::<f32>() * TAU,
            },
            state: FishState::Idle,
            facing: Direction::Right,
            body_size: fields.body_size,
            color: fields.color,
            speed: fields.speed,
            seek_boost: 0.0,
            direction_timer: DIRECTION_TIMER_DISPLAY_DEFAULT,
            zoomie_timer: ZOOMIE_TIMER_DISPLAY_DEFAULT,
            zoomed_secs: 0.0,
            species,
            pattern_seed: fields.pattern_seed,
            display_width: fields.display_width,
            sway_speed: fields.sway_speed,
            mutant: fields.mutant,
            mutations: None,
            weight_g: fields.weight_g,
            size_category: SizeCategory::M,
            devil_marked: false,
            unfish_state: None,
            botfish_state: programmable_state(species),
            sell_price_bonus_pct: 0,
            abduction_lock: false,
            frozen: false,
            engulf_timer: 0.0,
            blessing_timer: BLESSING_INTERVAL_SECS,
            blessing_glow: 0.0,
            pending_rad_mutations: 0,
            speech: None,
            field_cache: Vec::new(),
            sky: Sky::default(),
            habits: Box::default(),
        };
        grow_birthmarks(&mut fish, rng);
        fish
    }

    pub fn new_unfish(
        kind: UnfishKind,
        name: String,
        x: f32,
        y: f32,
        rng: &mut impl RngExt,
    ) -> Self {
        let (speed_lo, speed_hi) = kind.speed_range();
        let speed = rng.random_range(speed_lo..speed_hi);
        let display_width = match kind {
            UnfishKind::Skull => SKULL_WIDTH as usize,
            UnfishKind::Ball => BALL_WIDTH as usize,
            UnfishKind::Worm => worm_display_width(WORM_DEFAULT_SEGMENTS, 0, false, 0, 0),
            _ => compute_display_width(FishSpecies::Unfish, 5),
        };
        let (velocity, facing) = random_heading(speed, kind.dy_fraction(), rng);
        Self {
            name,
            position: Position { x, y },
            velocity,
            sway: SwayState {
                phase: rng.random::<f32>() * TAU,
            },
            state: FishState::Idle,
            facing,
            body_size: 5,
            color: WHITE,
            speed,
            seek_boost: 0.0,
            direction_timer: rng.random_range(DIRECTION_TIMER_MIN..DIRECTION_TIMER_MAX),
            zoomie_timer: rng.random_range(ZOOMIE_INITIAL_TIMER_MIN..ZOOMIE_INITIAL_TIMER_MAX),
            zoomed_secs: 0.0,
            species: FishSpecies::Unfish,
            pattern_seed: rng.random(),
            display_width,
            sway_speed: FishSpecies::Unfish.config().sway_speed,
            mutant: None,
            mutations: None,
            weight_g: 1,
            size_category: SizeCategory::M,
            devil_marked: false,
            unfish_state: Some(Box::new(UnfishState::new(kind, rng))),
            botfish_state: None,
            sell_price_bonus_pct: 0,
            abduction_lock: false,
            frozen: false,
            engulf_timer: 0.0,
            blessing_timer: BLESSING_INTERVAL_SECS,
            blessing_glow: 0.0,
            pending_rad_mutations: 0,
            speech: None,
            field_cache: Vec::new(),
            sky: Sky::default(),
            habits: Box::default(),
        }
    }

    pub fn portrait(&self) -> Fish {
        let mut fish = self.clone();
        fish.habits = Box::default();
        fish
    }

    pub fn adornments(&self) -> Adornments {
        if let Some(us) = self.unfish_state.as_ref() {
            return us.adornments;
        }
        self.mutant
            .as_ref()
            .map_or_else(Adornments::default, |mutant| mutant.adornments)
    }

    pub fn lead(&self) -> usize {
        self.adornments().lead()
    }

    pub fn is_asleep(&self) -> bool {
        self.circadian_state().asleep(self.sky.daylight)
    }

    pub fn zoomie(&self) -> Zoomie {
        self.species.config().zoomie
    }

    pub fn locomotion(&self) -> Locomotion {
        self.species.config().locomotion
    }

    pub fn habit(&self) -> Option<Habit> {
        self.species.config().habit
    }

    pub fn is_zooming(&self) -> bool {
        matches!(self.state, FishState::Zoomie { .. })
    }

    pub fn is_puffed(&self) -> bool {
        self.habits.puffed > 0.0
    }

    pub fn has_shifting_body(&self) -> bool {
        self.species.config().body_source == BodySource::MutantState
    }

    pub fn unfish_body(&self) -> Option<UnfishKind> {
        if self.species.config().body_source != BodySource::UnfishState {
            return None;
        }
        self.unfish_state.as_deref().map(|us| us.kind)
    }

    pub fn is_invisible(&self) -> bool {
        if let Some(us) = self.unfish_state.as_deref() {
            return us.is_invisible();
        }
        self.fused_personas().any(|us| us.is_invisible())
    }

    fn fused_personas(&self) -> impl Iterator<Item = &UnfishState> {
        self.mutant
            .as_ref()
            .into_iter()
            .flat_map(|m| m.fused.iter())
            .filter_map(|c| c.persona.as_deref())
    }

    pub fn personas_mut(&mut self) -> Vec<&mut UnfishState> {
        if let Some(us) = self.unfish_state.as_deref_mut() {
            return vec![us];
        }
        match self.mutant.as_mut() {
            Some(m) => m
                .fused
                .iter_mut()
                .filter_map(|c| c.persona.as_deref_mut())
                .collect(),
            None => Vec::new(),
        }
    }

    pub fn capture_persona(&self) -> Option<Box<UnfishState>> {
        let us = self.unfish_state.as_deref()?;
        us.kind.has_fused_behavior().then(|| Box::new(us.clone()))
    }

    pub fn capture_program(&self) -> Option<Box<BotfishState>> {
        self.script().map(|bot| Box::new(bot.clone()))
    }

    pub fn fused_program(&self, donor: &Fish) -> Option<Box<BotfishState>> {
        let Some(own) = self.script().filter(|own| own.is_wired()) else {
            return donor.capture_program().or_else(|| self.capture_program());
        };
        let Some(theirs) = donor.script() else {
            return self.capture_program();
        };
        Some(Box::new(own.fuse(theirs)))
    }

    pub fn is_programmable(&self) -> bool {
        self.ability_stacks(FishSpecies::Botfish) > 0
    }

    pub fn is_wired(&self) -> bool {
        self.script().is_some_and(|bot| bot.is_wired())
    }

    pub fn is_pinned(&self) -> bool {
        self.abduction_lock || self.frozen
    }

    pub fn is_arrangeable(&self) -> bool {
        self.is_programmable() && self.frozen
    }

    pub fn say(&mut self, text: String) -> bool {
        if text.trim().is_empty() {
            return false;
        }
        self.speech = Some(SpeechBubble::new(text));
        true
    }

    pub fn script(&self) -> Option<&BotfishState> {
        if let Some(bot) = self.botfish_state.as_deref() {
            return Some(bot);
        }
        self.fused_components()
            .iter()
            .find_map(|c| c.program.as_deref())
    }

    pub fn script_mut(&mut self) -> Option<&mut BotfishState> {
        if self.botfish_state.is_some() {
            return self.botfish_state.as_deref_mut();
        }
        self.fused_components_mut()
            .iter_mut()
            .find_map(|c| c.program.as_deref_mut())
    }

    fn tick_passengers(&mut self, dt: f32, rng: &mut impl RngExt) {
        if self.unfish_state.is_some() {
            return;
        }
        let daylight = self.sky.daylight;
        let Some(mutant) = self.mutant.as_mut() else {
            return;
        };
        for component in &mut mutant.fused {
            if let Some(us) = component.persona.as_deref_mut() {
                us.tick(dt, daylight, rng);
            }
        }
    }

    pub fn wake_color(&self) -> Option<Color> {
        if let Some(us) = self.unfish_state.as_ref() {
            return us.wake_color;
        }
        self.mutant.as_ref().and_then(|m| m.wake_color)
    }

    pub fn leaves_a_wake(&self) -> bool {
        self.zoomie().moves() || self.species.config().trail.is_some()
    }

    pub fn fused_components(&self) -> &[FusedComponent] {
        if let Some(us) = self.unfish_state.as_ref() {
            return &us.fused;
        }
        match self.mutant.as_ref() {
            Some(m) => &m.fused,
            None => &[],
        }
    }

    pub fn fused_components_mut(&mut self) -> &mut [FusedComponent] {
        if let Some(us) = self.unfish_state.as_mut() {
            return &mut us.fused;
        }
        match self.mutant.as_mut() {
            Some(m) => &mut m.fused,
            None => &mut [],
        }
    }

    pub fn set_fused(&mut self, components: Vec<FusedComponent>) {
        if let Some(us) = self.unfish_state.as_mut() {
            us.fused = components;
        } else if let Some(m) = self.mutant.as_mut() {
            m.fused = components;
        }
    }

    pub fn ability_components(&self) -> Vec<FishSpecies> {
        let fused = self.fused_components();
        if fused.is_empty() {
            return vec![self.species];
        }
        fused
            .iter()
            .flat_map(FusedComponent::ability_species)
            .collect()
    }

    pub fn fused_self_component(&self) -> FusedComponent {
        match self.unfish_state.as_ref() {
            Some(us) => FusedComponent::unfish(us.kind, self.name.clone(), self.weight_g),
            None => FusedComponent::fish(self.species, self.name.clone(), self.weight_g),
        }
    }

    pub fn ability_stacks(&self, species: FishSpecies) -> u32 {
        self.ability_components()
            .iter()
            .filter(|&&s| s == species)
            .count() as u32
    }

    pub fn sin(&self) -> Option<Sin> {
        match self.unfish_state.as_deref() {
            Some(us) => us.kind.sin(),
            None => self.species.config().flavour.sin,
        }
    }

    pub fn is_holy(&self) -> bool {
        self.ability_stacks(FishSpecies::Holyfish) > 0
    }

    pub fn is_alienated(&self) -> bool {
        self.mutations
            .as_ref()
            .is_some_and(|record| record.has(crate::fishes::mutations::Mutation::Alienation))
    }

    pub fn auto_mutate_stacks(&self) -> u32 {
        self.ability_components()
            .iter()
            .filter(|s| s.config().auto_mutate)
            .count() as u32
    }

    pub fn mutation_count(&self) -> u32 {
        self.mutations.as_ref().map_or(0, |record| record.count)
    }

    pub fn keepsakes(&self) -> Vec<StockItem> {
        self.ability_components()
            .iter()
            .filter_map(|species| species.config().keepsake)
            .collect()
    }

    pub fn is_sellable(&self) -> bool {
        self.ability_components()
            .iter()
            .all(|species| species.config().sellable)
    }

    pub fn sell_value(&self) -> Money {
        self.worth_at(self.weight_g)
    }

    fn sells_for_nothing(&self) -> bool {
        !self.is_sellable() || self.script().is_some_and(BotfishState::is_printed)
    }

    fn worth_at(&self, weight_g: u32) -> Money {
        if self.sells_for_nothing() {
            return 0;
        }
        let base = Money::from(self.species.sell_value(weight_g, self.size_category))
            + self
                .species
                .appraisal(self.size_category, self.pattern_seed);
        base + base * Money::from(self.sell_price_bonus_pct) / PERCENT_WHOLE
    }

    pub fn circadian_state(&self) -> Circadian {
        if let Some(us) = self.unfish_state.as_ref() {
            return us.circadian;
        }
        self.mutant
            .as_ref()
            .map_or(Circadian::Neutral, |m| m.circadian)
    }

    fn coffee_response(&self, coffee_stacks: u32) -> f32 {
        if self.is_asleep() {
            return 0.0;
        }
        match self.circadian_state() {
            Circadian::HelpedByGod => coffee_stacks as f32 * HELPEDBYGOD_COFFEE_MULT,
            Circadian::NightOwl | Circadian::Neutral => coffee_stacks as f32,
        }
    }

    fn circadian_zoomie_mult(&self) -> f32 {
        if self.is_asleep() {
            return 0.0;
        }
        match self.circadian_state() {
            Circadian::HelpedByGod => HELPEDBYGOD_ZOOMIE_MULT,
            Circadian::NightOwl | Circadian::Neutral => 1.0,
        }
    }

    fn circadian_speed_mult(&self) -> f32 {
        if self.is_asleep() {
            NIGHTOWL_SPEED_MULT
        } else {
            1.0
        }
    }

    fn circadian_sink_dy(&self) -> f32 {
        if self.is_asleep() {
            NIGHTOWL_SINK_DY
        } else {
            0.0
        }
    }

    pub fn segments(&self) -> Vec<(char, Color)> {
        self.line_cells().0
    }

    fn line_cells(&self) -> LineCells {
        if let Some(bot) = self.botfish_state.as_ref() {
            return (self.botfish_body_cells(bot.eye_color()), None);
        }
        if self.unfish_state.is_some() {
            return self.segments_unfish();
        }
        if let BodyTemplate::Figure(figure) = self.species.config().body {
            let sprite = self.figure_sprite(figure);
            let body = sprite
                .rows
                .into_iter()
                .nth(sprite.body_row)
                .unwrap_or_default();
            return (body, None);
        }
        let (mut cells, span) = if self.mutant.is_some() {
            self.segments_mutant()
        } else {
            (self.segments_plain(), None)
        };
        self.fill_body(&mut cells, self.facing_left());
        (cells, span)
    }

    fn fill_body(&self, cells: &mut [(char, Color)], facing_left: bool) {
        let config = self.species.config();
        if config.body_fill == BodyFill::Species {
            return;
        }
        let (BodyTemplate::Standard(chars) | BodyTemplate::Alternating(chars, _)) = config.body
        else {
            return;
        };
        let body_glyphs = [
            chars.body_left,
            chars.wave_left,
            chars.body_right,
            chars.wave_right,
        ];
        let keeps_its_colours = self.blessing_glow > 0.0 || self.paints_body();
        let mut rng = SmallRng::seed_from_u64(self.pattern_seed);
        let from_the_head: Vec<usize> = if facing_left {
            (0..cells.len()).collect()
        } else {
            (0..cells.len()).rev().collect()
        };
        for index in from_the_head {
            let cell = &mut cells[index];
            if !body_glyphs.contains(&cell.0) {
                continue;
            }
            let (glyph, color) = match config.body_fill {
                BodyFill::Stones => (STONES[rng.random_range(0..STONES.len())], None),
                _ => {
                    let (glyph, color) = junk_cell(&mut rng);
                    (glyph, Some(color))
                }
            };
            cell.0 = if facing_left {
                glyph
            } else {
                mirror_char(glyph)
            };
            if let Some(color) = color.filter(|_| !keeps_its_colours) {
                cell.1 = color;
            }
        }
    }

    fn segments_plain(&self) -> Vec<(char, Color)> {
        let config = self.species.config();
        let chars = self.build_chars(&config.body);
        let colors = if matches!(config.pattern, PatternKind::Glistening) {
            self.build_glistening_colors(
                chars.len(),
                config.palette[0],
                config.palette[1],
                config.palette[2],
            )
        } else {
            self.build_colors(chars.len(), config.palette, config.pattern)
        };
        let mut segs: Vec<(char, Color)> = chars.into_iter().zip(colors).collect();
        if !matches!(config.body, BodyTemplate::Fixed { .. })
            && matches!(self.facing, Direction::Right)
        {
            segs.reverse();
        }
        if let Some(eye_color) = config.eye_color
            && !matches!(config.body, BodyTemplate::Fixed { .. })
            && segs.len() >= 2
        {
            let from_head = MOUTH_WIDTH as usize + self.cells_before_eye(self.body_size);
            let eye_idx = if matches!(self.facing, Direction::Right) {
                segs.len() - 1 - from_head
            } else {
                from_head
            };
            segs[eye_idx].1 = eye_color;
        }
        if self.blessing_glow > 0.0 {
            apply_blessing_glow(&mut segs, self.color, self.blessing_glow);
        }
        segs
    }

    pub fn feet(&self) -> Option<Feet> {
        if let Some(us) = self.unfish_state.as_ref() {
            return us.feet;
        }
        self.mutant.as_ref().and_then(|m| m.feet)
    }

    pub fn body_extension(&self) -> BodyExtension {
        let slot = match self.unfish_state.as_ref() {
            Some(us) => us.body_extension,
            None => self.mutant.as_ref().and_then(|m| m.body_extension),
        };
        slot.unwrap_or_default()
    }

    pub fn line_sprite(&self) -> LineSprite {
        self.crawl_pose(self.sprite_with_eyes(self.keeps_eyes_shut()))
    }

    fn open_eyed_line_sprite(&self) -> LineSprite {
        self.sprite_with_eyes(false)
    }

    fn sprite_with_eyes(&self, shut: bool) -> LineSprite {
        if let Some((left, right)) = self.fused_render_halves() {
            let mut sprite = self.fused_line_sprite(&left, &right);
            if shut {
                sprite.shut_eyes();
            }
            return sprite;
        }
        let (mut sprite, span) = self.bare_line_sprite();
        if shut {
            sprite.shut_eyes();
        }
        let sprite = self.adorn(sprite, span);
        self.dress(sprite)
    }

    fn bare_line_sprite(&self) -> (LineSprite, Option<(usize, usize)>) {
        let (mut sprite, span) = self.body_sprite();
        if let Some(span) = span {
            self.grow_bands(&mut sprite, span);
        }
        (sprite, span)
    }

    fn body_sprite(&self) -> (LineSprite, Option<(usize, usize)>) {
        if let Some(bot) = self.botfish_state.as_ref() {
            let sprite = self.botfish_line_sprite(bot.eye_color(), bot.tip_color());
            let span = self.botfish_body_span();
            return (sprite, span);
        }
        if let BodyTemplate::Figure(figure) = self.species.config().body {
            let sprite = self.figure_sprite(figure);
            let span = sprite
                .rows
                .get(sprite.body_row)
                .and_then(|row| painted_span(row));
            return (sprite, span);
        }
        let (body, struct_span) = self.line_cells();
        let span = struct_span.or_else(|| painted_span(&body));
        (
            LineSprite {
                rows: vec![body],
                body_row: 0,
            },
            span,
        )
    }

    fn grow_bands(&self, sprite: &mut LineSprite, span: (usize, usize)) {
        let body = sprite.rows[sprite.body_row].clone();
        let extension = self.body_extension();
        let posed = |variant| PosedExtension {
            variant,
            facing_left: self.facing_left(),
            phase: self.sway.phase,
            max_tentacles: None,
        };
        let below = match (extension.bottom, self.feet()) {
            (Some(variant), _) => posed(variant).rows(&body, span, Band::Bottom),
            (None, Some(feet)) => vec![feet_row(&body, span, feet)],
            (None, None) => Vec::new(),
        };
        let at = sprite.body_row + 1;
        sprite.rows.splice(at..at, below);
        if let Some(variant) = extension.top {
            let above = posed(variant).rows(&body, span, Band::Top);
            let at = sprite.body_row;
            sprite.body_row += above.len();
            sprite.rows.splice(at..at, above);
        }
    }

    pub fn reserves(&self, band: Band) -> bool {
        if self.zoomie() == Zoomie::Glide {
            return true;
        }
        if self.botfish_state.is_some() {
            return band == Band::Top;
        }
        match self.species.config().body {
            BodyTemplate::Figure(figure) if self.unfish_state.is_none() => {
                !figure.leaves_free(band)
            }
            _ => false,
        }
    }

    fn botfish_body_cells(&self, eye_color: Color) -> Vec<(char, Color)> {
        let single = self.botfish_single_cells(eye_color);
        if !self.is_double_now() {
            return single;
        }
        let Some((lo, hi)) = botfish_span_in(&single, self.facing_left()) else {
            return single;
        };
        let mirrored = |cells: &[(char, Color)]| -> Vec<(char, Color)> {
            cells
                .iter()
                .rev()
                .map(|&(glyph, color)| (mirror_char(glyph), color))
                .collect()
        };
        if self.facing_left() {
            let head = &single[..=hi];
            let mut cells = head.to_vec();
            cells.extend(mirrored(head));
            return cells;
        }
        let head = &single[lo..];
        let mut cells = mirrored(head);
        cells.extend_from_slice(head);
        cells
    }

    fn botfish_single_cells(&self, eye_color: Color) -> Vec<(char, Color)> {
        let BodyTemplate::Fixed { left, right } = self.species.config().body else {
            return Vec::new();
        };
        let variants = if self.facing_left() { left } else { right };
        let line = variants.first().copied().unwrap_or("");
        let body = self.botfish_color();
        line.chars()
            .map(|c| {
                let color = if c == EYE_ROUND { eye_color } else { body };
                (c, color)
            })
            .collect()
    }

    fn botfish_color(&self) -> Color {
        match self.mutant.as_ref() {
            Some(mutant) if !mutant.patterned => self.color,
            _ => BODY_COLOR,
        }
    }

    pub fn skin(&self) -> Skin {
        let skin = self.species.config().skin;
        let borrowed = matches!(skin, Skin::Camouflage | Skin::SeeThrough);
        if borrowed && self.wears_paint() {
            return Skin::Palette;
        }
        skin
    }

    fn wears_paint(&self) -> bool {
        self.paints_body()
            || self.mutant.as_ref().is_some_and(|mutant| {
                mutant.ear_color.is_some() || mutant.feet.is_some_and(|feet| feet.color.is_some())
            })
    }

    fn paints_body(&self) -> bool {
        self.mutant.as_ref().is_some_and(|mutant| {
            !mutant.patterned
                || mutant.glistening_color.is_some()
                || !mutant.color_patches.is_empty()
                || mutant.adornments.lunar
        })
    }

    pub fn body_span(&self) -> Option<(usize, usize)> {
        if self.botfish_state.is_some() {
            return self.botfish_body_span();
        }
        let (cells, span) = self.line_cells();
        span.or_else(|| painted_span(&cells))
    }

    fn botfish_body_span(&self) -> Option<(usize, usize)> {
        let single = self.botfish_single_cells(BODY_COLOR);
        let (lo, hi) = botfish_span_in(&single, self.facing_left())?;
        if self.is_double_now() && !self.facing_left() {
            let shift = single.len() - lo;
            return Some((lo + shift, hi + shift));
        }
        Some((lo, hi))
    }

    fn botfish_line_sprite(&self, eye_color: Color, tip_color: Color) -> LineSprite {
        let body = self.botfish_body_cells(eye_color);
        let width = body.len();
        let center = width / 2;
        let mut rows: Vec<Vec<(char, Color)>> = Vec::with_capacity(ANTENNA_LENGTH + 1);
        for depth in (0..ANTENNA_LENGTH).rev() {
            let mut row = vec![(crate::sprite::TRANSPARENT, Color::Reset); width];
            if center < width {
                row[center] = if depth == ANTENNA_LENGTH - 1 {
                    (ANTENNA_TIP, tip_color)
                } else {
                    (ANTENNA_STALK, self.botfish_color())
                };
            }
            rows.push(row);
        }
        let body_row = rows.len();
        rows.push(body);
        LineSprite { rows, body_row }
    }

    pub fn botfish_width(&self) -> Option<usize> {
        self.botfish_state
            .as_ref()
            .map(|_| self.botfish_body_cells(BODY_COLOR).len() + self.lead())
    }

    pub fn is_double_now(&self) -> bool {
        if let Some(us) = self.unfish_state.as_ref() {
            return us.worm_is_double;
        }
        self.mutant.as_ref().is_some_and(|m| m.is_double)
    }

    pub fn fused_render_halves(&self) -> Option<(Fish, Fish)> {
        if !self.is_double_now() {
            return None;
        }
        let comps = self.fused_components();
        if comps.len() != 2 {
            return None;
        }
        let left = comps[0].fish_snapshot()?;
        let right = comps[1].fish_snapshot()?;
        Some((left.clone(), right.clone()))
    }

    pub fn fused_render_width(&self) -> usize {
        if let Some(cells) = self.fused_worm_cells() {
            return cells.len();
        }
        match self.fused_render_halves() {
            Some((left, right)) => self
                .fused_line_sprite(&left, &right)
                .rows
                .first()
                .map_or(self.display_width, |r| r.len()),
            None => self.display_width,
        }
    }

    fn is_worm(&self) -> bool {
        self.unfish_state
            .as_ref()
            .is_some_and(|us| us.kind == UnfishKind::Worm)
    }

    pub fn fused_worm_cells(&self) -> Option<Vec<(char, Color)>> {
        if !self.is_worm() {
            return None;
        }
        let (left, right) = self.fused_render_halves()?;
        let mut cells = worm_half_cells(&left);
        let mut mirrored = worm_half_cells(&right);
        mirrored.reverse();
        for cell in &mut mirrored {
            cell.0 = crate::sprite::mirror_char(cell.0);
        }
        cells.extend(mirrored);
        Some(cells)
    }

    fn half_sprite(&self, snapshot: &Fish, facing: Direction) -> (LineSprite, usize, usize) {
        let mut half = snapshot.clone();
        half.facing = facing;
        half.sway.phase = self.sway.phase;
        half.sky = self.sky;
        let (cells, span) = half.line_cells();
        let lead = half.lead();
        let last = cells.len().saturating_sub(1);
        let (lo, hi) = match facing {
            Direction::Left => (0, span.map_or(last, |(_, hi)| hi) + lead),
            Direction::Right => (span.map_or(0, |(lo, _)| lo), last + lead),
        };
        (half.line_sprite(), lo, hi)
    }

    fn fused_line_sprite(&self, left: &Fish, right: &Fish) -> LineSprite {
        let (left_sprite, l_lo, l_hi) = self.half_sprite(left, Direction::Left);
        let (right_sprite, r_lo, r_hi) = self.half_sprite(right, Direction::Right);
        let left_w = l_hi + 1 - l_lo;
        let right_w = r_hi + 1 - r_lo;
        let top = left_sprite.body_row.max(right_sprite.body_row);
        let l_bottom = left_sprite.rows.len() - 1 - left_sprite.body_row;
        let r_bottom = right_sprite.rows.len() - 1 - right_sprite.body_row;
        let bottom = l_bottom.max(r_bottom);
        let mut rows = Vec::with_capacity(top + 1 + bottom);
        for i in 0..(top + 1 + bottom) {
            let rel = i as isize - top as isize;
            let mut row = half_row(&left_sprite, rel, l_lo, l_hi, left_w);
            row.extend(half_row(&right_sprite, rel, r_lo, r_hi, right_w));
            rows.push(row);
        }
        LineSprite {
            rows,
            body_row: top,
        }
    }

    fn build_chars(&self, body: &BodyTemplate) -> Vec<char> {
        match *body {
            BodyTemplate::Standard(body_chars) => self.build_standard_chars(body_chars),
            BodyTemplate::Alternating(even_body_chars, odd_body_chars) => {
                let body_chars = if self.pattern_seed.is_multiple_of(2) {
                    even_body_chars
                } else {
                    odd_body_chars
                };
                self.build_standard_chars(body_chars)
            }
            BodyTemplate::Fixed { left, right } => {
                let variants = match self.facing {
                    Direction::Left => left,
                    Direction::Right => right,
                };
                let idx = self.pattern_seed as usize % variants.len();
                variants[idx].chars().collect()
            }
            BodyTemplate::Figure(_) => self.line_cells().0.into_iter().map(|(c, _)| c).collect(),
        }
    }

    fn build_standard_chars(&self, body_chars: BodyChars) -> Vec<char> {
        self.build_standard_chars_for(body_chars, self.facing)
    }

    fn build_standard_chars_for(&self, body_chars: BodyChars, facing: Direction) -> Vec<char> {
        let (body_char, wave_char, raw_mouth, eye) = match facing {
            Direction::Left => (
                body_chars.body_left,
                body_chars.wave_left,
                body_chars.mouth_left,
                body_chars.eye_left,
            ),
            Direction::Right => (
                body_chars.body_right,
                body_chars.wave_right,
                body_chars.mouth_right,
                body_chars.eye_right,
            ),
        };
        let mouth = if matches!(self.state, FishState::Eating { .. }) {
            invert_mouth(raw_mouth)
        } else {
            self.gaped(raw_mouth)
        };
        let substituting = self.sway.phase.sin() > WAVE_THRESHOLD;
        let body: Vec<char> = (0..self.body_size)
            .map(|_| if substituting { wave_char } else { body_char })
            .collect();
        let tail = tail_chars(body_chars, facing, self.sway.phase);
        let before = self.cells_before_eye(body.len());
        let mut chars = vec![mouth];
        chars.extend(&body[..before]);
        chars.push(eye);
        chars.extend(&body[before..]);
        chars.extend(tail);
        chars
    }

    fn cells_before_eye(&self, body_len: usize) -> usize {
        match self.species.config().eye_at {
            EyeAt::Head => 0,
            EyeAt::Middle => body_len / 2,
        }
    }

    fn centre_eyes(
        &self,
        segs: &mut [(char, Color)],
        eye_start: usize,
        eye_count: usize,
        body_len: usize,
    ) -> usize {
        let before = self.cells_before_eye(body_len);
        segs[eye_start..eye_start + eye_count + before].rotate_left(eye_count);
        before
    }

    fn segments_unfish(&self) -> LineCells {
        let unfish_state = self.unfish_state.as_ref().unwrap();
        if is_multi_row(unfish_state.kind) {
            return (vec![], None);
        }
        let config = FishSpecies::Unfish.config();
        let body_chars = match config.body {
            BodyTemplate::Standard(b) => b,
            _ => unreachable!(),
        };
        match unfish_state.kind {
            UnfishKind::Worm => {
                let facing_left = matches!(self.facing, Direction::Left);
                let shape = WormShape {
                    facing_left,
                    forward: unfish_state.worm_forward,
                    segments: unfish_state.worm_segments,
                    extra_eyes: unfish_state.worm_extra_eyes,
                    is_double: unfish_state.worm_is_double,
                    backwards: unfish_state.worm_backwards,
                    ears: unfish_state.ear_count,
                    hydra: unfish_state.hydra_count,
                };
                let sprite = build_worm(&shape);
                let eye_cols = worm_eye_cols(&shape);
                let ear_color = unfish_state.ear_color.unwrap_or(PINK);
                let n = sprite.chars().count();
                let mut glisten_colors = unfish_state.line_colors(n, |_| UNFISH_BODY_COLOR);
                if !unfish_state.slime_glisten_enabled {
                    for &(pos, color) in &unfish_state.slime_color_patches {
                        if pos < n && !eye_cols.contains(&pos) {
                            glisten_colors[pos] = color;
                        }
                    }
                }
                let cells: Vec<(char, Color)> = sprite
                    .chars()
                    .enumerate()
                    .map(|(i, c)| {
                        if let Some(eye_idx) = eye_cols.iter().position(|&col| col == i) {
                            let ch = if unfish_state.eye.is_open { '0' } else { '-' };
                            (ch, unfish_state.worm_eye_render_color(eye_idx))
                        } else if c == EAR_LEFT || c == EAR_RIGHT {
                            (c, ear_color)
                        } else {
                            (c, glisten_colors[i])
                        }
                    })
                    .collect();
                (cells, None)
            }
            UnfishKind::Reversed => {
                let opposite = match self.facing {
                    Direction::Left => Direction::Right,
                    Direction::Right => Direction::Left,
                };
                let chars = self.build_standard_chars_for(body_chars, opposite);
                let colors = unfish_state.line_colors(chars.len(), |_| UNFISH_BODY_COLOR);
                let mut segs: Vec<(char, Color)> = chars.into_iter().zip(colors).collect();
                if matches!(self.facing, Direction::Left) {
                    segs.reverse();
                }
                let n = segs.len();
                for &(pos, color) in &unfish_state.slime_color_patches {
                    if pos < n {
                        segs[pos].1 = color;
                    }
                }
                let mut span = None;
                if n >= 2 {
                    let eye_idx = if matches!(self.facing, Direction::Left) {
                        n - 2
                    } else {
                        1
                    };
                    segs[eye_idx].1 = unfish_state.eye_render_color();
                    span = insert_line_appendages(
                        &mut segs,
                        eye_idx,
                        unfish_state.ear_count,
                        unfish_state.ear_color.unwrap_or(PINK),
                        unfish_state.hydra_count,
                        unfish_state.eye_render_color(),
                    );
                }
                (segs, span)
            }
            UnfishKind::Blinker => {
                let chars = self.build_standard_chars(body_chars);
                let n = chars.len();
                let colors = unfish_state.line_colors(n, |i| {
                    let s = (unfish_state.glistening_phase - i as f32 * CHAR_SPREAD).sin();
                    if s > BLINKER_GLISTEN_PEAK {
                        BLINKER_PEAK_COLOR
                    } else if s > BLINKER_GLISTEN_MID {
                        BLINKER_MID_COLOR
                    } else {
                        BLINKER_BASE_COLOR
                    }
                });
                let mut segs: Vec<(char, Color)> = chars.into_iter().zip(colors).collect();
                if matches!(self.facing, Direction::Right) {
                    segs.reverse();
                }
                for &(pos, color) in &unfish_state.slime_color_patches {
                    if pos < n {
                        segs[pos].1 = color;
                    }
                }
                let mut span = None;
                if n >= 2 {
                    let eye_idx = if matches!(self.facing, Direction::Left) {
                        1
                    } else {
                        n - 2
                    };
                    if let Some(eye) = unfish_state.slime_eye_color {
                        segs[eye_idx].1 = eye;
                    }
                    span = insert_line_appendages(
                        &mut segs,
                        eye_idx,
                        unfish_state.ear_count,
                        unfish_state.ear_color.unwrap_or(PINK),
                        unfish_state.hydra_count,
                        unfish_state.eye_render_color(),
                    );
                }
                (segs, span)
            }
            _ => {
                let chars = self.build_standard_chars(body_chars);
                let colors = unfish_state.line_colors(chars.len(), |_| UNFISH_BODY_COLOR);
                let mut segs: Vec<(char, Color)> = chars.into_iter().zip(colors).collect();
                if matches!(self.facing, Direction::Right) {
                    segs.reverse();
                }
                let n = segs.len();
                for &(pos, color) in &unfish_state.slime_color_patches {
                    if pos < n {
                        segs[pos].1 = color;
                    }
                }
                let mut span = None;
                if n >= 2 {
                    let eye_idx = if matches!(self.facing, Direction::Right) {
                        n - 2
                    } else {
                        1
                    };
                    segs[eye_idx].1 = unfish_state.eye_render_color();
                    span = insert_line_appendages(
                        &mut segs,
                        eye_idx,
                        unfish_state.ear_count,
                        unfish_state.ear_color.unwrap_or(PINK),
                        unfish_state.hydra_count,
                        unfish_state.eye_render_color(),
                    );
                }
                (segs, span)
            }
        }
    }

    fn build_colors(
        &self,
        len: usize,
        palette: &'static [Color],
        pattern: PatternKind,
    ) -> Vec<Color> {
        match pattern {
            PatternKind::Solid => vec![self.color; len],
            PatternKind::Striped => (0..len).map(|i| palette[i % palette.len()]).collect(),
            PatternKind::Patchy => {
                let mut rng = SmallRng::seed_from_u64(self.pattern_seed);
                (0..len)
                    .map(|_| palette[rng.random_range(0..palette.len())])
                    .collect()
            }
            PatternKind::PatchyAll => {
                let mut rng = SmallRng::seed_from_u64(self.pattern_seed);
                let mut colors: Vec<Color> = (0..len)
                    .map(|_| palette[rng.random_range(0..palette.len())])
                    .collect();
                for (slot, &forced) in palette.iter().enumerate().take(len) {
                    if !colors.contains(&forced) {
                        colors[slot] = forced;
                    }
                }
                colors
            }
            PatternKind::Banded => (0..len)
                .map(|i| palette[(i / BAND_WIDTH) % palette.len()])
                .collect(),
            PatternKind::Halves => {
                let front = len.div_ceil(PATTERN_HALVES);
                (0..len)
                    .map(|i| palette[usize::from(i >= front) % palette.len()])
                    .collect()
            }
            PatternKind::Gradient => (0..len)
                .map(|i| palette[(i * palette.len() / len).min(palette.len() - 1)])
                .collect(),
            PatternKind::Speckled => {
                let mut rng = SmallRng::seed_from_u64(self.pattern_seed);
                (0..len)
                    .map(|_| {
                        if rng.random_range(0..SPECKLE_ODDS) == 0 {
                            palette[rng.random_range(1..palette.len())]
                        } else {
                            palette[0]
                        }
                    })
                    .collect()
            }
            PatternKind::Zones => {
                let mut rng = SmallRng::seed_from_u64(self.pattern_seed);
                let tail = palette[rng.random_range(ZONE_TAIL..palette.len())];
                let tail_from = len.saturating_sub(self.species_tail_width());
                (0..len)
                    .map(|i| match i {
                        i if i < MOUTH_WIDTH as usize => palette[ZONE_HEAD],
                        i if i >= tail_from => tail,
                        _ => palette[ZONE_BODY],
                    })
                    .collect()
            }
            PatternKind::Glistening => unreachable!(),
        }
    }

    fn species_tail_width(&self) -> usize {
        match self.species.config().body {
            BodyTemplate::Standard(chars) | BodyTemplate::Alternating(chars, _) => {
                tail_width(chars.tail)
            }
            BodyTemplate::Fixed { .. } | BodyTemplate::Figure(_) => 0,
        }
    }

    fn build_glistening_colors(
        &self,
        len: usize,
        base: Color,
        mid: Color,
        peak: Color,
    ) -> Vec<Color> {
        (0..len)
            .map(|i| {
                color_for_glisten(
                    GlisteningMode::Wave,
                    self.sway.phase,
                    i,
                    len,
                    base,
                    mid,
                    peak,
                )
            })
            .collect()
    }

    fn segments_mutant(&self) -> LineCells {
        let mutant = self.mutant.as_ref().unwrap();

        if let BodyTemplate::Fixed { left, right } = self.species.config().body {
            let facing_left_fixed = matches!(self.facing, Direction::Left);
            let idx = self.pattern_seed as usize % left.len();
            let chars_left: Vec<char> = left[idx].chars().collect();

            if chars_left.len() <= 1 {
                let jelly_char = chars_left.first().copied().unwrap_or(' ');
                let n = if mutant.is_double { 2 } else { 1 };
                if mutant.glistening_color.is_some() {
                    let (base, mid, peak_default) = derive_glistening_palette(self.color);
                    let peak = mutant.glistening_color.unwrap_or(peak_default);
                    let cells: Vec<(char, Color)> = (0..n)
                        .map(|i| {
                            let c = color_for_glisten(
                                mutant.glistening_mode,
                                self.sway.phase,
                                i,
                                n,
                                base,
                                mid,
                                peak,
                            );
                            (jelly_char, c)
                        })
                        .collect();
                    return (cells, None);
                } else {
                    return ((0..n).map(|_| (jelly_char, self.color)).collect(), None);
                }
            }

            let _ = right;
            let mouth_ch = if facing_left_fixed {
                chars_left[0]
            } else {
                invert_mouth(chars_left[0])
            };
            let body_ch = if facing_left_fixed {
                chars_left[1]
            } else {
                invert_mouth(chars_left[1])
            };
            let tail_ch = if facing_left_fixed {
                *chars_left.last().unwrap()
            } else {
                invert_mouth(*chars_left.last().unwrap())
            };

            let (left_eye, right_eye) = self.eye_glyphs();
            let eye_chars: Vec<char> = if facing_left_fixed {
                mutant.left_eyes.iter().map(|e| e.glyph(left_eye)).collect()
            } else {
                mutant
                    .right_eyes
                    .iter()
                    .map(|e| e.glyph(right_eye))
                    .collect()
            };

            let mut out_chars: Vec<char> = Vec::new();
            let (eye_start, eye_count, double_eye_start, double_eye_count) =
                if mutant.is_double && mutant.backwards {
                    let mirror_body = invert_mouth(body_ch);
                    let mirror_tail = invert_mouth(tail_ch);
                    out_chars.push(tail_ch);
                    for _ in 0..(eye_chars.len() + self.body_size) {
                        out_chars.push(body_ch);
                    }
                    for _ in 0..(self.body_size + mutant.double_head_eyes.len()) {
                        out_chars.push(mirror_body);
                    }
                    out_chars.push(mirror_tail);
                    (0usize, 0usize, 0usize, 0usize)
                } else {
                    out_chars.push(mouth_ch);
                    let eye_start = out_chars.len();
                    out_chars.extend(eye_chars.iter().copied());
                    let eye_count = eye_chars.len();
                    for _ in 0..self.body_size {
                        out_chars.push(body_ch);
                    }
                    let (double_eye_start, double_eye_count) = if mutant.is_double {
                        let mirror_body = invert_mouth(body_ch);
                        let mirror_mouth = invert_mouth(mouth_ch);
                        for _ in 0..self.body_size {
                            out_chars.push(mirror_body);
                        }
                        let d_start = out_chars.len();
                        let double_eyes: Vec<char> = mutant
                            .double_head_eyes
                            .iter()
                            .map(|e| {
                                e.glyph(if facing_left_fixed {
                                    right_eye
                                } else {
                                    left_eye
                                })
                            })
                            .collect();
                        let d_count = double_eyes.len();
                        out_chars.extend(double_eyes);
                        out_chars.push(mirror_mouth);
                        (d_start, d_count)
                    } else {
                        out_chars.push(tail_ch);
                        (0usize, 0usize)
                    };
                    (eye_start, eye_count, double_eye_start, double_eye_count)
                };

            let n = out_chars.len();
            let use_glisten = mutant.glistening_color.is_some();
            let mut colors: Vec<Color> = if use_glisten {
                let (base, mid, peak_default) = derive_glistening_palette(self.color);
                let peak = mutant.glistening_color.unwrap_or(peak_default);
                (0..n)
                    .map(|i| {
                        color_for_glisten(
                            mutant.glistening_mode,
                            self.sway.phase,
                            i,
                            n,
                            base,
                            mid,
                            peak,
                        )
                    })
                    .collect()
            } else {
                vec![self.color; n]
            };
            let eye_a_eyes = if facing_left_fixed {
                &mutant.left_eyes
            } else {
                &mutant.right_eyes
            };
            let eye_a_colors: Vec<Option<Color>> = eye_a_eyes
                .iter()
                .take(eye_count)
                .map(|e| e.color.or(mutant.eye_color))
                .collect();
            let eye_b_colors: Vec<Option<Color>> = mutant
                .double_head_eyes
                .iter()
                .take(double_eye_count)
                .map(|e| e.color.or(mutant.eye_color))
                .collect();
            apply_patches_and_eyes(
                &mut colors,
                &mutant.color_patches,
                eye_start,
                &eye_a_colors,
                double_eye_start,
                &eye_b_colors,
            );
            let mut segs: Vec<(char, Color)> = out_chars.into_iter().zip(colors).collect();
            if !facing_left_fixed {
                segs.reverse();
            }
            return (segs, None);
        }

        let facing_left = matches!(self.facing, Direction::Left);
        let eating = matches!(self.state, FishState::Eating { .. });

        let (mouth, body_ch, wave_ch, non_double_tail): (char, char, char, Vec<char>) =
            if self.species.config().auto_mutate {
                let (raw_mouth, bc, wc) =
                    body_chars_for_variant(mutant.body_variant, facing_left, mutant.mouth_inverted);
                let mo = if eating {
                    invert_mouth(raw_mouth)
                } else {
                    raw_mouth
                };
                (
                    mo,
                    bc,
                    wc,
                    mutant.tail_variant.chars(facing_left, self.sway.phase),
                )
            } else {
                let config = self.species.config();
                let body_chars = match config.body {
                    BodyTemplate::Standard(b) | BodyTemplate::Alternating(b, _) => b,
                    BodyTemplate::Fixed { .. } | BodyTemplate::Figure(_) => unreachable!(),
                };
                let (body_char, wave_char, raw_mouth) = if facing_left {
                    (
                        body_chars.body_left,
                        body_chars.wave_left,
                        body_chars.mouth_left,
                    )
                } else {
                    (
                        body_chars.body_right,
                        body_chars.wave_right,
                        body_chars.mouth_right,
                    )
                };
                let pre_eat = if mutant.mouth_inverted {
                    invert_mouth(raw_mouth)
                } else {
                    raw_mouth
                };
                let mo = if eating {
                    invert_mouth(pre_eat)
                } else {
                    self.gaped(pre_eat)
                };
                (
                    mo,
                    body_char,
                    wave_char,
                    grown_tail(
                        body_chars,
                        mutant.tail_variant,
                        self.facing,
                        self.sway.phase,
                    ),
                )
            };

        let sway_high = self.sway.phase.sin() > WAVE_THRESHOLD;
        let max_eyes = mutant.left_eyes.len().max(mutant.right_eyes.len());
        let (left_eye, right_eye) = self.eye_glyphs();

        let mut chars: Vec<char> = Vec::new();
        let (eye_start, eye_count, extra_body, double_eye_start, double_eye_count) =
            if mutant.is_double && mutant.backwards {
                let body_ch_now = if sway_high { wave_ch } else { body_ch };
                let extra_body = max_eyes + EXTRA_BODY_FOR_DOUBLE + mutant.double_head_eyes.len();
                chars.extend(mirror_tail(&non_double_tail));
                let body_start = chars.len();
                for _ in 0..(self.body_size + extra_body) {
                    chars.push(body_ch_now);
                }
                chars.extend(non_double_tail.iter().copied());
                (body_start, 0usize, extra_body, 0usize, 0usize)
            } else {
                chars.push(mouth);
                let eye_start = chars.len();
                let eye_count = if facing_left {
                    for e in &mutant.left_eyes {
                        chars.push(e.glyph(left_eye));
                    }
                    mutant.left_eyes.len()
                } else {
                    for e in &mutant.right_eyes {
                        chars.push(e.glyph(right_eye));
                    }
                    mutant.right_eyes.len()
                };
                let extra_body = max_eyes - eye_count;
                for _ in 0..(self.body_size + extra_body) {
                    chars.push(if sway_high { wave_ch } else { body_ch });
                }
                let (double_eye_start, double_eye_count) = if mutant.is_double {
                    for _ in 0..EXTRA_BODY_FOR_DOUBLE {
                        chars.push(if sway_high { wave_ch } else { body_ch });
                    }
                    let start = chars.len();
                    for e in &mutant.double_head_eyes {
                        chars.push(e.glyph(if facing_left { right_eye } else { left_eye }));
                    }
                    let count = mutant.double_head_eyes.len();
                    chars.push(invert_mouth(mouth));
                    (start, count)
                } else {
                    chars.extend(non_double_tail);
                    (0usize, 0usize)
                };
                (
                    eye_start,
                    eye_count,
                    extra_body,
                    double_eye_start,
                    double_eye_count,
                )
            };

        let n = chars.len();
        let use_glisten = mutant.glistening_color.is_some() || self.species.config().auto_glisten;
        let mut colors: Vec<Color> = if use_glisten {
            let (base, mid, peak_default) = derive_glistening_palette(self.color);
            let peak = mutant.glistening_color.unwrap_or(peak_default);
            (0..n)
                .map(|i| {
                    color_for_glisten(
                        mutant.glistening_mode,
                        self.sway.phase,
                        i,
                        n,
                        base,
                        mid,
                        peak,
                    )
                })
                .collect()
        } else {
            self.mutant_colors(mutant, n)
        };
        let eye_a_eyes = if facing_left {
            &mutant.left_eyes
        } else {
            &mutant.right_eyes
        };
        let eye_a_colors: Vec<Option<Color>> = eye_a_eyes
            .iter()
            .take(eye_count)
            .map(|e| e.color.or(mutant.eye_color))
            .collect();
        let eye_b_colors: Vec<Option<Color>> = mutant
            .double_head_eyes
            .iter()
            .take(double_eye_count)
            .map(|e| e.color.or(mutant.eye_color))
            .collect();
        apply_patches_and_eyes(
            &mut colors,
            &mutant.color_patches,
            eye_start,
            &eye_a_colors,
            double_eye_start,
            &eye_b_colors,
        );
        let mut segs: Vec<(char, Color)> = chars.into_iter().zip(colors).collect();
        let body_len = self.body_size + extra_body;
        let before = if mutant.is_double && mutant.backwards {
            0
        } else {
            self.centre_eyes(&mut segs, eye_start, eye_count, body_len)
        };
        insert_line_hydra(
            &mut segs,
            eye_start + before + eye_count,
            body_len - before,
            &mutant_hydra_cells(mutant),
        );
        insert_ears(
            &mut segs,
            eye_start + before + eye_count,
            mutant.ear_count,
            ear_glyph(facing_left),
            mutant.ear_color.unwrap_or(PINK),
        );
        if !facing_left {
            segs.reverse();
        }
        let body_core_len = if mutant.is_double && !mutant.backwards {
            self.body_size + extra_body + EXTRA_BODY_FOR_DOUBLE
        } else {
            self.body_size + extra_body
        };
        let hydra_used = mutant
            .hydra_eyes
            .len()
            .min((self.body_size + extra_body).saturating_sub(1));
        let around_eyes = if before > 0 {
            eye_count + mutant.ear_count
        } else {
            0
        };
        let span = self.mutant_body_span(
            eye_start + eye_count + mutant.ear_count - around_eyes,
            body_core_len + hydra_used + around_eyes,
            segs.len(),
            facing_left,
        );
        (segs, span)
    }

    fn mutant_body_span(
        &self,
        body_lo: usize,
        body_len: usize,
        n: usize,
        facing_left: bool,
    ) -> Option<(usize, usize)> {
        if body_len == 0 || body_lo + body_len > n {
            return None;
        }
        let body_hi = body_lo + body_len - 1;
        Some(if facing_left {
            (body_lo, body_hi)
        } else {
            (n - 1 - body_hi, n - 1 - body_lo)
        })
    }

    pub fn facing_left(&self) -> bool {
        matches!(self.facing, Direction::Left)
    }

    pub fn head_x(&self) -> i32 {
        let lead = self.lead() as i32;
        match self.facing {
            Direction::Left => self.position.x as i32 + lead,
            Direction::Right => self.position.x as i32 + self.display_width as i32 - 1 - lead,
        }
    }

    fn eye_glyphs(&self) -> (char, char) {
        let config = self.species.config();
        let (left, right) = match config.body {
            BodyTemplate::Standard(chars) | BodyTemplate::Alternating(chars, _) => {
                (chars.eye_left, chars.eye_right)
            }
            BodyTemplate::Fixed { .. } | BodyTemplate::Figure(_) => (EYE_ROUND, EYE_ROUND),
        };
        if config.auto_mutate {
            return (left, EYE_CIRCLE);
        }
        (left, right)
    }

    fn mutant_colors(&self, mutant: &MutantState, n: usize) -> Vec<Color> {
        if !mutant.patterned {
            return vec![self.color; n];
        }
        let config = self.species.config();
        match config.pattern {
            PatternKind::Glistening => self.build_glistening_colors(
                n,
                config.palette[0],
                config.palette[1],
                config.palette[2],
            ),
            pattern => self.build_colors(n, config.palette, pattern),
        }
    }

    pub fn eye_x(&self) -> i32 {
        let from_head = MOUTH_WIDTH + self.cells_before_eye(self.body_size) as i32;
        match self.facing {
            Direction::Left => self.head_x() + from_head,
            Direction::Right => self.head_x() - from_head,
        }
    }

    pub fn tick(
        &mut self,
        settings: &Settings,
        tank_width: u16,
        tank_height: u16,
        coffee_stacks: u32,
    ) {
        let dt = 1.0 / settings.fps;
        let coffee = self.coffee_response(coffee_stacks);
        self.zoomed_secs = 0.0;

        if self.engulf_timer > 0.0 {
            self.engulf_timer = (self.engulf_timer - dt).max(0.0);
        }
        if self.blessing_glow > 0.0 {
            self.blessing_glow = (self.blessing_glow - dt).max(0.0);
        }
        SpeechBubble::fade(&mut self.speech, dt);
        self.tick_habit_clocks(dt);

        if self.is_pinned() {
            if self.abduction_lock && self.adornments().puff && !self.is_puffed() {
                self.habits.puffed = PUFF_SECS;
            }
            tick_sway(&mut self.sway, self.sway_speed);
            if let Some(ref mut mutant) = self.mutant {
                mutant.tick_eyes(dt, self.sky.daylight);
            }
            return;
        }

        match self.state {
            FishState::Idle => {
                if self.zoomie().zooms() && !self.is_wired() && !self.is_puffed() {
                    let zoomie_dt =
                        dt * (1.0 + COFFEE_ZOOMIE_DT_MULT * coffee) * self.circadian_zoomie_mult();
                    self.zoomie_timer -= zoomie_dt;
                    if self.zoomie_timer <= 0.0 {
                        self.launch_zoomie();
                    } else {
                        self.direction_timer = self.direction_timer.saturating_sub(1);
                        if self.direction_timer == 0 {
                            self.randomize_direction();
                        }
                    }
                } else {
                    self.direction_timer = self.direction_timer.saturating_sub(1);
                    if self.direction_timer == 0 {
                        self.randomize_direction();
                    }
                }
            }
            FishState::Zoomie {
                time_remaining,
                total_duration,
                will_turn,
                has_turned,
            } => {
                if self.zoomie().moves() {
                    self.zoomed_secs = time_remaining.min(dt);
                }
                let new_time = time_remaining - dt;
                let should_turn =
                    will_turn && !has_turned && new_time < total_duration * ZOOMIE_TURN_THRESHOLD;

                if should_turn {
                    self.velocity.dx = -self.velocity.dx;
                    self.facing = self.facing.flip();
                }

                if new_time <= 0.0 {
                    self.end_zoomie();
                } else {
                    self.state = FishState::Zoomie {
                        time_remaining: new_time,
                        total_duration,
                        will_turn,
                        has_turned: has_turned || should_turn,
                    };
                }
            }
            FishState::SeekingFood { .. } => {}
            FishState::Eating { time_remaining } => {
                let new_time = time_remaining - dt;
                if new_time <= 0.0 {
                    self.cancel_seek();
                } else {
                    self.state = FishState::Eating {
                        time_remaining: new_time,
                    };
                }
            }
        }

        let holding_still = matches!(self.state, FishState::Eating { .. })
            || self.is_puffed()
            || self.habits.resting > 0.0;
        if !holding_still {
            let speed_mult = (1.0 + COFFEE_SPEED_MULT * coffee) * self.circadian_speed_mult();
            self.locomote(dt, speed_mult, tank_width, tank_height);
        }

        let sway_mult = 1.0 + COFFEE_SWAY_MULT * coffee;
        let effective_sway_speed = if matches!(self.state, FishState::Zoomie { .. }) {
            self.sway_speed * ZOOMIE_SWAY_MULTIPLIER * sway_mult
        } else {
            self.sway_speed * sway_mult
        };
        tick_sway(&mut self.sway, effective_sway_speed);

        let daylight = self.sky.daylight;
        if let Some(ref mut mutant) = self.mutant {
            mutant.tick_eyes(dt, daylight);
        }
        if let Some(ref mut us) = self.unfish_state {
            let mut rng = rand::rng();
            us.tick(dt, daylight, &mut rng);
        }
        if let Some(ref mut bot) = self.botfish_state {
            bot.tick_blink(dt);
        }
        self.tick_passengers(dt, &mut rand::rng());
    }

    fn tick_habit_clocks(&mut self, dt: f32) {
        let habits = &mut self.habits;
        for clock in [
            &mut habits.puffed,
            &mut habits.lit,
            &mut habits.alert,
            &mut habits.sated,
            &mut habits.hiding,
            &mut habits.resting,
            &mut habits.rest_clock,
        ] {
            *clock = (*clock - dt).max(0.0);
        }
        if self.species.config().skin == Skin::Cycle(Cycle::OnClock) {
            self.habits.hue_clock += dt;
            if self.habits.hue_clock >= HUE_SHIFT_SECS {
                self.habits.hue_clock = 0.0;
                self.habits.hue = self.habits.hue.wrapping_add(1);
            }
        }
    }

    pub fn cancel_seek(&mut self) {
        let norm = (self.velocity.dx * self.velocity.dx + self.velocity.dy * self.velocity.dy)
            .sqrt()
            .max(VELOCITY_NORM_MIN);
        self.velocity.dx = (self.velocity.dx / norm) * self.speed;
        self.velocity.dy = (self.velocity.dy / norm) * self.speed * DY_FRACTION;
        self.seek_boost = 0.0;
        let mut rng = rand::rng();
        self.direction_timer =
            rng.random_range(DIRECTION_TIMER_POST_EVENT_MIN..DIRECTION_TIMER_POST_EVENT_MAX);
        self.state = FishState::Idle;
    }

    pub fn randomize_direction(&mut self) {
        let mut rng = rand::rng();
        if self.habits.hiding > 0.0 || self.habits.backwards > 0.0 {
            self.direction_timer = rng.random_range(DIRECTION_TIMER_MIN..DIRECTION_TIMER_MAX);
            return;
        }
        self.wander_direction(&mut rng);
        self.direction_timer = rng.random_range(DIRECTION_TIMER_MIN..DIRECTION_TIMER_MAX);
    }

    pub fn zoomed_secs(&self) -> f32 {
        self.zoomed_secs
    }

    pub fn hurry_zoomie(&mut self) -> bool {
        if !self.zoomie().zooms() || self.is_wired() {
            return false;
        }
        self.zoomie_timer = 0.0;
        true
    }

    fn end_zoomie(&mut self) {
        let mut rng = rand::rng();
        let sign = if self.velocity.dx >= 0.0 {
            1.0_f32
        } else {
            -1.0_f32
        };
        self.velocity.dx = sign * self.speed;
        self.velocity.dy = 0.0;
        self.habits.hop = None;
        self.direction_timer =
            rng.random_range(DIRECTION_TIMER_POST_EVENT_MIN..DIRECTION_TIMER_POST_EVENT_MAX);
        self.state = FishState::Idle;
    }

    pub fn tick_animation(&mut self, dt: f32) {
        tick_sway(&mut self.sway, self.sway_speed);
        let daylight = self.sky.daylight;
        if let Some(ref mut mutant) = self.mutant {
            mutant.tick_eyes(dt, daylight);
        }
        if let Some(ref mut us) = self.unfish_state {
            let mut rng = rand::rng();
            us.tick(dt, daylight, &mut rng);
        }
    }

    pub fn static_left_segments(&self) -> Vec<(char, Color)> {
        let mut cells = self.lead_cells_left();
        if self.unfish_state.is_some() {
            cells.extend(self.static_left_segments_unfish());
            return cells;
        }
        let mut body = self.static_left_body();
        self.fill_body(&mut body, true);
        cells.extend(body);
        cells
    }

    fn static_left_body(&self) -> Vec<(char, Color)> {
        let config = self.species.config();
        if let BodyTemplate::Figure(_) = config.body {
            let mut still = self.clone();
            still.facing = Direction::Left;
            still.sway.phase = 0.0;
            return still.line_cells().0;
        }
        if self.mutant.is_some() {
            return self.static_left_segments_mutant();
        }
        match config.body {
            BodyTemplate::Figure(_) => unreachable!(),
            BodyTemplate::Fixed { left, .. } => {
                let idx = self.pattern_seed as usize % left.len();
                let chars: Vec<char> = left[idx].chars().collect();
                let len = chars.len();
                let colors = self.build_colors(len, config.palette, config.pattern);
                chars.into_iter().zip(colors).collect()
            }
            BodyTemplate::Standard(bc) | BodyTemplate::Alternating(bc, _) => {
                let body_chars = match config.body {
                    BodyTemplate::Alternating(even_body_chars, odd_body_chars) => {
                        if self.pattern_seed.is_multiple_of(2) {
                            even_body_chars
                        } else {
                            odd_body_chars
                        }
                    }
                    _ => bc,
                };
                let mut chars = vec![body_chars.mouth_left, body_chars.eye_left];
                chars.extend(std::iter::repeat_n(body_chars.body_left, self.body_size));
                chars.extend(tail_chars(body_chars, Direction::Left, 0.0));
                let len = chars.len();
                let colors = if matches!(config.pattern, PatternKind::Glistening) {
                    self.build_glistening_colors(
                        len,
                        config.palette[0],
                        config.palette[1],
                        config.palette[2],
                    )
                } else {
                    self.build_colors(len, config.palette, config.pattern)
                };
                chars.into_iter().zip(colors).collect()
            }
        }
    }

    fn static_left_segments_unfish(&self) -> Vec<(char, Color)> {
        let unfish_state = self.unfish_state.as_ref().unwrap();
        if is_multi_row(unfish_state.kind) {
            return vec![];
        }
        let config = FishSpecies::Unfish.config();
        let body_chars = match config.body {
            BodyTemplate::Standard(b) => b,
            _ => unreachable!(),
        };
        match unfish_state.kind {
            UnfishKind::Worm => {
                let shape = WormShape {
                    facing_left: true,
                    forward: true,
                    segments: unfish_state.worm_segments,
                    extra_eyes: unfish_state.worm_extra_eyes,
                    is_double: unfish_state.worm_is_double,
                    backwards: unfish_state.worm_backwards,
                    ears: unfish_state.ear_count,
                    hydra: unfish_state.hydra_count,
                };
                let sprite = build_worm(&shape);
                let eye_cols = worm_eye_cols(&shape);
                let ear_color = unfish_state.ear_color.unwrap_or(PINK);
                let body_color = unfish_state.slime_body_color.unwrap_or(UNFISH_BODY_COLOR);
                let mut segs: Vec<(char, Color)> = sprite
                    .chars()
                    .enumerate()
                    .map(|(i, c)| {
                        if let Some(eye_idx) = eye_cols.iter().position(|&col| col == i) {
                            ('0', unfish_state.worm_eye_render_color(eye_idx))
                        } else if c == EAR_LEFT || c == EAR_RIGHT {
                            (c, ear_color)
                        } else {
                            (c, body_color)
                        }
                    })
                    .collect();
                for &(pos, color) in &unfish_state.slime_color_patches {
                    let is_ear =
                        matches!(segs.get(pos), Some(&(EAR_LEFT, _)) | Some(&(EAR_RIGHT, _)));
                    if pos < segs.len() && !eye_cols.contains(&pos) && !is_ear {
                        segs[pos].1 = color;
                    }
                }
                segs
            }
            UnfishKind::Reversed => {
                let chars = self.build_standard_chars_for(body_chars, Direction::Right);
                let colors = unfish_state.line_colors(chars.len(), |_| UNFISH_BODY_COLOR);
                let mut segs: Vec<(char, Color)> = chars.into_iter().zip(colors).collect();
                segs.reverse();
                let n = segs.len();
                for &(pos, color) in &unfish_state.slime_color_patches {
                    if pos < n {
                        segs[pos].1 = color;
                    }
                }
                if n >= 2 {
                    segs[n - 2].1 = unfish_state.eye_render_color();
                    insert_line_appendages(
                        &mut segs,
                        n - 2,
                        unfish_state.ear_count,
                        unfish_state.ear_color.unwrap_or(PINK),
                        unfish_state.hydra_count,
                        unfish_state.eye_render_color(),
                    );
                }
                segs
            }
            UnfishKind::Blinker => {
                let chars = self.build_standard_chars_for(body_chars, Direction::Left);
                let colors = unfish_state.line_colors(chars.len(), |_| BLINKER_BASE_COLOR);
                let mut segs: Vec<(char, Color)> = chars.into_iter().zip(colors).collect();
                let n = segs.len();
                for &(pos, color) in &unfish_state.slime_color_patches {
                    if pos < n {
                        segs[pos].1 = color;
                    }
                }
                if n >= 2 {
                    segs[1].1 = unfish_state.eye_render_color();
                    insert_line_appendages(
                        &mut segs,
                        1,
                        unfish_state.ear_count,
                        unfish_state.ear_color.unwrap_or(PINK),
                        unfish_state.hydra_count,
                        unfish_state.eye_render_color(),
                    );
                }
                segs
            }
            _ => {
                let chars = self.build_standard_chars_for(body_chars, Direction::Left);
                let colors = unfish_state.line_colors(chars.len(), |_| UNFISH_BODY_COLOR);
                let mut segs: Vec<(char, Color)> = chars.into_iter().zip(colors).collect();
                let n = segs.len();
                for &(pos, color) in &unfish_state.slime_color_patches {
                    if pos < n {
                        segs[pos].1 = color;
                    }
                }
                if n >= 2 {
                    segs[1].1 = unfish_state.eye_render_color();
                    insert_line_appendages(
                        &mut segs,
                        1,
                        unfish_state.ear_count,
                        unfish_state.ear_color.unwrap_or(PINK),
                        unfish_state.hydra_count,
                        unfish_state.eye_render_color(),
                    );
                }
                segs
            }
        }
    }

    fn static_left_segments_mutant(&self) -> Vec<(char, Color)> {
        let mutant = self.mutant.as_ref().unwrap();

        if let BodyTemplate::Fixed { left, .. } = self.species.config().body {
            let idx = self.pattern_seed as usize % left.len();
            let chars_left: Vec<char> = left[idx].chars().collect();

            if chars_left.len() <= 1 {
                let jelly_char = chars_left.first().copied().unwrap_or(' ');
                let n = if mutant.is_double { 2 } else { 1 };
                if mutant.glistening_color.is_some() {
                    let (base, mid, peak_default) = derive_glistening_palette(self.color);
                    let peak = mutant.glistening_color.unwrap_or(peak_default);
                    return (0..n)
                        .map(|i| {
                            let c = color_for_glisten(
                                mutant.glistening_mode,
                                0.0,
                                i,
                                n,
                                base,
                                mid,
                                peak,
                            );
                            (jelly_char, c)
                        })
                        .collect();
                } else {
                    return (0..n).map(|_| (jelly_char, self.color)).collect();
                }
            }

            let mouth_ch = chars_left[0];
            let body_ch = chars_left[1];
            let tail_ch = *chars_left.last().unwrap();

            let mut out_chars: Vec<char> = Vec::new();
            let (eye_start, eye_count, double_eye_start, double_eye_count) =
                if mutant.is_double && mutant.backwards {
                    let mirror_body = invert_mouth(body_ch);
                    let mirror_tail = invert_mouth(tail_ch);
                    out_chars.push(tail_ch);
                    for _ in 0..(mutant.left_eyes.len() + self.body_size) {
                        out_chars.push(body_ch);
                    }
                    for _ in 0..(self.body_size + mutant.double_head_eyes.len()) {
                        out_chars.push(mirror_body);
                    }
                    out_chars.push(mirror_tail);
                    (0usize, 0usize, 0usize, 0usize)
                } else {
                    out_chars.push(mouth_ch);
                    let eye_start = out_chars.len();
                    for e in &mutant.left_eyes {
                        out_chars.push(e.glyph(self.eye_glyphs().0));
                    }
                    let eye_count = mutant.left_eyes.len();
                    for _ in 0..self.body_size {
                        out_chars.push(body_ch);
                    }
                    let (double_eye_start, double_eye_count) = if mutant.is_double {
                        let mirror_body = invert_mouth(body_ch);
                        let mirror_mouth = invert_mouth(mouth_ch);
                        for _ in 0..self.body_size {
                            out_chars.push(mirror_body);
                        }
                        let d_start = out_chars.len();
                        for e in &mutant.double_head_eyes {
                            out_chars.push(e.glyph(self.eye_glyphs().1));
                        }
                        let d_count = mutant.double_head_eyes.len();
                        out_chars.push(mirror_mouth);
                        (d_start, d_count)
                    } else {
                        out_chars.push(tail_ch);
                        (0usize, 0usize)
                    };
                    (eye_start, eye_count, double_eye_start, double_eye_count)
                };

            let n = out_chars.len();
            let use_glisten = mutant.glistening_color.is_some();
            let mut colors: Vec<Color> = if use_glisten {
                let (base, mid, peak_default) = derive_glistening_palette(self.color);
                let peak = mutant.glistening_color.unwrap_or(peak_default);
                (0..n)
                    .map(|i| color_for_glisten(mutant.glistening_mode, 0.0, i, n, base, mid, peak))
                    .collect()
            } else {
                vec![self.color; n]
            };
            let eye_a_colors: Vec<Option<Color>> = mutant
                .left_eyes
                .iter()
                .take(eye_count)
                .map(|e| e.color.or(mutant.eye_color))
                .collect();
            let eye_b_colors: Vec<Option<Color>> = mutant
                .double_head_eyes
                .iter()
                .take(double_eye_count)
                .map(|e| e.color.or(mutant.eye_color))
                .collect();
            apply_patches_and_eyes(
                &mut colors,
                &mutant.color_patches,
                eye_start,
                &eye_a_colors,
                double_eye_start,
                &eye_b_colors,
            );
            return out_chars.into_iter().zip(colors).collect();
        }

        let (raw_mouth, body_ch, non_double_tail): (char, char, Vec<char>) =
            if self.has_shifting_body() {
                let (rm, bc, _) =
                    body_chars_for_variant(mutant.body_variant, true, mutant.mouth_inverted);
                (rm, bc, mutant.tail_variant.chars(true, 0.0))
            } else {
                let config = self.species.config();
                let body_chars = match config.body {
                    BodyTemplate::Standard(b) | BodyTemplate::Alternating(b, _) => b,
                    BodyTemplate::Fixed { .. } | BodyTemplate::Figure(_) => unreachable!(),
                };
                let rm = if mutant.mouth_inverted {
                    invert_mouth(body_chars.mouth_left)
                } else {
                    body_chars.mouth_left
                };
                (
                    rm,
                    body_chars.body_left,
                    grown_tail(body_chars, mutant.tail_variant, Direction::Left, 0.0),
                )
            };
        let max_eyes = mutant.left_eyes.len().max(mutant.right_eyes.len());
        let mut chars: Vec<char> = Vec::new();
        let (eye_start, eye_count, extra_body, double_eye_start, double_eye_count) =
            if mutant.is_double && mutant.backwards {
                let extra_body = max_eyes + EXTRA_BODY_FOR_DOUBLE + mutant.double_head_eyes.len();
                chars.extend(mirror_tail(&non_double_tail));
                let body_start = chars.len();
                for _ in 0..(self.body_size + extra_body) {
                    chars.push(body_ch);
                }
                chars.extend(non_double_tail.iter().copied());
                (body_start, 0usize, extra_body, 0usize, 0usize)
            } else {
                chars.push(raw_mouth);
                let eye_start = chars.len();
                let (left_eye, right_eye) = self.eye_glyphs();
                for e in &mutant.left_eyes {
                    chars.push(e.glyph(left_eye));
                }
                let eye_count = mutant.left_eyes.len();
                let extra_body = max_eyes - eye_count;
                for _ in 0..(self.body_size + extra_body) {
                    chars.push(body_ch);
                }
                let (double_eye_start, double_eye_count) = if mutant.is_double {
                    for _ in 0..EXTRA_BODY_FOR_DOUBLE {
                        chars.push(body_ch);
                    }
                    let start = chars.len();
                    for e in &mutant.double_head_eyes {
                        chars.push(e.glyph(right_eye));
                    }
                    let count = mutant.double_head_eyes.len();
                    chars.push(invert_mouth(raw_mouth));
                    (start, count)
                } else {
                    chars.extend(non_double_tail);
                    (0, 0)
                };
                (
                    eye_start,
                    eye_count,
                    extra_body,
                    double_eye_start,
                    double_eye_count,
                )
            };
        let n = chars.len();
        let use_glisten = mutant.glistening_color.is_some() || self.species.config().auto_glisten;
        let mut colors: Vec<Color> = if use_glisten {
            let (base, mid, peak_default) = derive_glistening_palette(self.color);
            let peak = mutant.glistening_color.unwrap_or(peak_default);
            (0..n)
                .map(|i| color_for_glisten(mutant.glistening_mode, 0.0, i, n, base, mid, peak))
                .collect()
        } else {
            self.mutant_colors(mutant, n)
        };
        let eye_a_colors: Vec<Option<Color>> = mutant
            .left_eyes
            .iter()
            .take(eye_count)
            .map(|e| e.color.or(mutant.eye_color))
            .collect();
        let eye_b_colors: Vec<Option<Color>> = mutant
            .double_head_eyes
            .iter()
            .take(double_eye_count)
            .map(|e| e.color.or(mutant.eye_color))
            .collect();
        apply_patches_and_eyes(
            &mut colors,
            &mutant.color_patches,
            eye_start,
            &eye_a_colors,
            double_eye_start,
            &eye_b_colors,
        );
        let mut segs: Vec<(char, Color)> = chars.into_iter().zip(colors).collect();
        let body_len = self.body_size + extra_body;
        let before = if mutant.is_double && mutant.backwards {
            0
        } else {
            self.centre_eyes(&mut segs, eye_start, eye_count, body_len)
        };
        insert_line_hydra(
            &mut segs,
            eye_start + before + eye_count,
            body_len - before,
            &mutant_hydra_cells(mutant),
        );
        insert_ears(
            &mut segs,
            eye_start + before + eye_count,
            mutant.ear_count,
            ear_glyph(true),
            mutant.ear_color.unwrap_or(PINK),
        );
        segs
    }

    fn appendage_extent(&self) -> (f32, f32) {
        let extension = self.body_extension();
        let feet = if self.feet().is_some() {
            FEET_ROW_COUNT
        } else {
            0
        };
        let below = extension.rows(Band::Bottom).max(feet);
        (extension.rows(Band::Top) as f32, below as f32)
    }

    fn sprite_y_margins(&self) -> (f32, f32) {
        let (base_top, base_bottom) = match self.unfish_state.as_ref().map(|us| us.kind) {
            Some(UnfishKind::Skull) => (3.0, 2.0),
            Some(UnfishKind::Ball) => (4.0, 4.0),
            _ => {
                let sprite = self.open_eyed_line_sprite();
                let below = sprite.rows.len().saturating_sub(1 + sprite.body_row);
                return (sprite.body_row as f32, below as f32);
            }
        };
        let (extra_top, extra_bottom) = self.appendage_extent();
        (base_top + extra_top, base_bottom + extra_bottom)
    }

    fn bounce_walls(&mut self, tank_width: u16, tank_height: u16) {
        if let Some(ref us) = self.unfish_state
            && us.kind == UnfishKind::Worm
        {
            self.position.x = self.position.x.rem_euclid(tank_width as f32);
            self.position.y = self.position.y.rem_euclid(tank_height as f32);
            return;
        }
        let (min_x, max_x, min_y, max_y) = self.position_bounds(tank_width, tank_height);

        let hit_wall = self.position.x < min_x || self.position.x > max_x;
        if self.position.x < min_x {
            self.position.x = min_x;
            self.velocity.dx = self.velocity.dx.abs();
            if matches!(self.state, FishState::Idle | FishState::Zoomie { .. }) {
                self.facing = Direction::Right;
            }
        } else if self.position.x > max_x {
            self.position.x = max_x;
            self.velocity.dx = -self.velocity.dx.abs();
            if matches!(self.state, FishState::Idle | FishState::Zoomie { .. }) {
                self.facing = Direction::Left;
            }
        }
        if hit_wall && self.habit() == Some(Habit::Blind) {
            self.habits.bumped = true;
        }

        if self.position.y < min_y {
            self.position.y = min_y;
            self.velocity.dy = self.velocity.dy.abs();
        } else if self.position.y > max_y {
            self.position.y = max_y;
            self.velocity.dy = -self.velocity.dy.abs();
        }
    }

    fn position_bounds(&self, tank_width: u16, tank_height: u16) -> (f32, f32, f32, f32) {
        let (top_margin, bottom_margin) = self.sprite_y_margins();
        (
            0.0,
            tank_width as f32 - self.display_width as f32,
            top_margin,
            (tank_height as f32 - 1.0) - bottom_margin,
        )
    }

    pub fn nudge(&mut self, dx: i32, dy: i32, tank_width: u16, tank_height: u16) {
        let (min_x, max_x, min_y, max_y) = self.position_bounds(tank_width, tank_height);
        self.position.x = (self.position.x + dx as f32).clamp(min_x, max_x.max(min_x));
        self.position.y = (self.position.y + dy as f32).clamp(min_y, max_y.max(min_y));
    }

    pub fn flip(&mut self) {
        self.facing = match self.facing {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        };
        self.velocity.dx = -self.velocity.dx;
    }
}

fn botfish_span_in(cells: &[(char, Color)], facing_left: bool) -> Option<(usize, usize)> {
    let eye = cells.iter().position(|&(c, _)| c == EYE_ROUND)?;
    let same_as = |index: usize, glyph: char| cells.get(index).is_some_and(|&(c, _)| c == glyph);
    if facing_left {
        let glyph = cells.get(eye + 1)?.0;
        let hi = (eye + 1..cells.len())
            .take_while(|&index| same_as(index, glyph))
            .last()?;
        return Some((eye + 1, hi));
    }
    let hi = eye.checked_sub(1)?;
    let glyph = cells[hi].0;
    let lo = (0..=hi)
        .rev()
        .take_while(|&index| same_as(index, glyph))
        .last()?;
    Some((lo, hi))
}

fn invert_mouth(ch: char) -> char {
    match ch {
        '<' => '>',
        '>' => '<',
        _ => ch,
    }
}

fn body_chars_for_variant(
    variant: u8,
    facing_left: bool,
    mouth_inverted: bool,
) -> (char, char, char) {
    let mouth = if facing_left ^ mouth_inverted {
        '<'
    } else {
        '>'
    };
    let (body, wave) = match (variant, facing_left) {
        (0, true) => ('(', '{'),
        (0, false) => (')', '}'),
        (1, true) => ('}', ')'),
        (1, false) => ('{', '('),
        (3, _) => ('+', '-'),
        _ => (';', ':'),
    };
    (mouth, body, wave)
}

fn apply_blessing_glow(segs: &mut [(char, Color)], base: Color, glow_remaining: f32) {
    let phase = (BLESSING_GLOW_SECS - glow_remaining) * BLESSING_GLOW_SPEED;
    let n = segs.len();
    for (i, seg) in segs.iter_mut().enumerate() {
        seg.1 = color_for_glisten(GlisteningMode::FullGlow, phase, i, n, base, base, WHITE);
    }
}

fn apply_patches_and_eyes(
    colors: &mut [Color],
    patches: &[(usize, Color)],
    eye_a_start: usize,
    eye_a_colors: &[Option<Color>],
    eye_b_start: usize,
    eye_b_colors: &[Option<Color>],
) {
    for &(pos, c) in patches {
        if pos < colors.len() {
            colors[pos] = c;
        }
    }
    for (ranges_start, eye_colors) in [(eye_a_start, eye_a_colors), (eye_b_start, eye_b_colors)] {
        for (k, color) in eye_colors.iter().enumerate() {
            let Some(color) = color else { continue };
            let i = ranges_start + k;
            if i < colors.len() {
                colors[i] = *color;
            }
        }
    }
}

fn worm_half_cells(snapshot: &Fish) -> Vec<(char, Color)> {
    let mut half = snapshot.clone();
    half.facing = Direction::Left;
    if let Some(us) = half.unfish_state.as_mut() {
        us.worm_forward = true;
        us.worm_is_double = false;
        us.worm_backwards = false;
    }
    let mut cells = half.segments();
    cells.pop();
    cells
}

fn insert_ears(segs: &mut Vec<(char, Color)>, at: usize, count: usize, glyph: char, color: Color) {
    if count == 0 {
        return;
    }
    let at = at.min(segs.len());
    segs.splice(at..at, std::iter::repeat_n((glyph, color), count));
}

fn insert_line_appendages(
    segs: &mut Vec<(char, Color)>,
    eye_idx: usize,
    ears: usize,
    ear_color: Color,
    hydra: usize,
    hydra_color: Color,
) -> Option<(usize, usize)> {
    if segs.is_empty() {
        return None;
    }
    let n = segs.len();
    let head_left = eye_idx * 2 < n;
    let mut inserts: Vec<(usize, (char, Color))> = Vec::new();
    let ear_at = if head_left { eye_idx + 1 } else { eye_idx };
    for _ in 0..ears {
        inserts.push((ear_at, (ear_glyph(head_left), ear_color)));
    }
    let (body_start, body_end) = if head_left {
        (eye_idx + 1, n.saturating_sub(LINE_TAIL_W))
    } else {
        (LINE_TAIL_W, eye_idx)
    };
    let avail = body_end.saturating_sub(body_start).saturating_sub(1);
    let hydra_used = hydra.min(avail);
    for (pos, _eye_start, size) in hydra_cluster_slots(body_start, avail, hydra_used) {
        for _ in 0..size {
            inserts.push((pos, (EYE_ROUND, hydra_color)));
        }
    }
    inserts.sort_by_key(|insert| std::cmp::Reverse(insert.0));
    for (pos, cell) in inserts {
        let pos = pos.min(segs.len());
        segs.insert(pos, cell);
    }
    if body_end <= body_start {
        return None;
    }
    let span = if head_left {
        (body_start + ears, body_end - 1 + ears + hydra_used)
    } else {
        (body_start, body_end - 1 + hydra_used)
    };
    Some(span)
}

fn hydra_cluster_slots(
    body_start: usize,
    avail: usize,
    count: usize,
) -> Vec<(usize, usize, usize)> {
    if count == 0 {
        return Vec::new();
    }
    let clusters = count.div_ceil(HYDRA_EYES_PER_HEAD);
    even_indices(avail, clusters)
        .into_iter()
        .enumerate()
        .map(|(ci, slot)| {
            let eye_start = ci * HYDRA_EYES_PER_HEAD;
            let size = (count - eye_start).min(HYDRA_EYES_PER_HEAD);
            (body_start + 1 + slot, eye_start, size)
        })
        .collect()
}

fn insert_line_hydra(
    segs: &mut Vec<(char, Color)>,
    body_start: usize,
    body_len: usize,
    eyes: &[(char, Option<Color>)],
) {
    let avail = body_len.saturating_sub(1);
    let count = eyes.len().min(avail);
    for (pos, eye_start, size) in hydra_cluster_slots(body_start, avail, count)
        .into_iter()
        .rev()
    {
        for k in (eye_start..eye_start + size).rev() {
            let (ch, color) = eyes[k];
            let color = color.unwrap_or_else(|| segs[pos.saturating_sub(1)].1);
            segs.insert(pos, (ch, color));
        }
    }
}

fn mutant_hydra_cells(mutant: &MutantState) -> Vec<(char, Option<Color>)> {
    mutant
        .hydra_eyes
        .iter()
        .map(|e| (e.glyph(EYE_ROUND), e.color.or(mutant.eye_color)))
        .collect()
}

fn mirror_tail(tail: &[char]) -> Vec<char> {
    tail.iter().rev().map(|&c| mirror_char(c)).collect()
}

fn half_row(
    sprite: &LineSprite,
    rel: isize,
    lo: usize,
    hi: usize,
    width: usize,
) -> Vec<(char, Color)> {
    let src = sprite.body_row as isize + rel;
    if src >= 0 && (src as usize) < sprite.rows.len() {
        sprite.rows[src as usize][lo..=hi].to_vec()
    } else {
        vec![(crate::sprite::TRANSPARENT, Color::Reset); width]
    }
}

fn grown_tail(
    body_chars: BodyChars,
    grown: MutantTail,
    facing: Direction,
    phase: f32,
) -> Vec<char> {
    if tail_kind_to_mutant_tail(body_chars.tail) == grown {
        return tail_chars(body_chars, facing, phase);
    }
    grown.chars(matches!(facing, Direction::Left), phase)
}

fn tail_chars(body_chars: BodyChars, facing: Direction, phase: f32) -> Vec<char> {
    match body_chars.tail {
        TailKind::Wide => vec!['>', '<'],
        TailKind::WideCurly => match facing {
            Direction::Left => vec!['>', '<', '{'],
            Direction::Right => vec!['<', '>', '}'],
        },
        TailKind::Short => match facing {
            Direction::Left => vec!['<'],
            Direction::Right => vec!['>'],
        },
        TailKind::Custom { left, right } => match facing {
            Direction::Left => vec![left],
            Direction::Right => vec![right],
        },
        TailKind::Swaying { left, right, wave } => {
            let base = match facing {
                Direction::Left => left,
                Direction::Right => right,
            };
            if phase.sin() > WAVE_THRESHOLD {
                let base_w = UnicodeWidthChar::width(base).unwrap_or(1);
                let wave_w = UnicodeWidthChar::width(wave).unwrap_or(1);
                let mut chars = vec![wave];
                chars.extend(std::iter::repeat_n(' ', base_w.saturating_sub(wave_w)));
                chars
            } else {
                vec![base]
            }
        }
        TailKind::None => vec![],
    }
}

fn tail_width(tail: TailKind) -> usize {
    match tail {
        TailKind::Wide => 2,
        TailKind::WideCurly => 3,
        TailKind::Short => 1,
        TailKind::None => 0,
        TailKind::Custom { left, right } | TailKind::Swaying { left, right, .. } => {
            let lw = UnicodeWidthChar::width(left).unwrap_or(1);
            let rw = UnicodeWidthChar::width(right).unwrap_or(1);
            lw.max(rw)
        }
    }
}

pub fn compute_display_width(species: FishSpecies, body_size: usize) -> usize {
    let config = species.config();
    match config.body {
        BodyTemplate::Standard(body_chars) | BodyTemplate::Alternating(body_chars, _) => {
            2 + body_size + tail_width(body_chars.tail)
        }
        BodyTemplate::Fixed { left, .. } => {
            let variant = left.first().copied().unwrap_or("");
            unicode_width::UnicodeWidthStr::width(variant)
        }
        BodyTemplate::Figure(figure) => figure.width(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_constructor_that_makes_a_real_fish_gives_a_programmable_one_its_circuit() {
        let mut rng = rand::rng();
        for &species in crate::fishes::species::ALL_SPECIES {
            let programmable = species.config().programmable;
            let spawned = Fish::new(species, "Probe".to_string(), 0.0, 0.0, &mut rng);
            let bought = Fish::new_for_display(species, &mut rng);
            assert_eq!(
                spawned.botfish_state.is_some(),
                programmable,
                "{} spawned without the circuit its config declares",
                species.display_name()
            );
            assert_eq!(
                bought.botfish_state.is_some(),
                programmable,
                "{} bought from the shop lost the circuit a spawned one keeps",
                species.display_name()
            );
            assert_eq!(
                spawned.adornments(),
                bought.adornments(),
                "{} is born with the same birthmarks through both doors",
                species.display_name()
            );
            assert_eq!(
                spawned.circadian_state(),
                bought.circadian_state(),
                "{}",
                species.display_name()
            );
            assert_eq!(
                spawned.feet().is_some(),
                bought.feet().is_some(),
                "{}",
                species.display_name()
            );
        }
    }

    #[test]
    fn a_botfish_body_is_the_run_behind_its_eye_whichever_way_it_faces() {
        let mut rng = rand::rng();
        let mut bot = Fish::new(FishSpecies::Botfish, "Crt".to_string(), 0.0, 0.0, &mut rng);
        for facing in [Direction::Left, Direction::Right] {
            bot.facing = facing;
            let side = if bot.facing_left() { "left" } else { "right" };
            let cells = bot.segments();
            let (lo, hi) = bot.body_span().expect("a botfish has a body");
            let body: String = cells[lo..=hi].iter().map(|&(c, _)| c).collect();
            assert_eq!(body.chars().count(), 5, "{side}: {body}");
            assert!(
                body.chars()
                    .all(|c| c == body.chars().next().unwrap_or(' ')),
                "{side}: the body is one run of one glyph, got {body}"
            );
            let eye = cells
                .iter()
                .position(|&(c, _)| c == EYE_ROUND)
                .expect("an eye");
            let beside = if bot.facing_left() { lo - 1 } else { hi + 1 };
            assert_eq!(eye, beside, "{side}: the body starts right behind the eye");
        }
    }
}
