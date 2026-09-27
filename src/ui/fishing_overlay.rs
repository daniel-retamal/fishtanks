use std::ops::Range;

use crossterm::event::KeyCode;
use rand::RngExt;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};
use unicode_width::UnicodeWidthChar;

use crate::colors::{CYAN, DARK_GRAY, GREEN, LIGHT_GREEN, LIGHT_RED, LIGHT_YELLOW, RED, WHITE};
use crate::consumable::{
    PHYSICAL_INSTRUMENT_ALPHA, REACTION_SPEED_ALPHA, VISUAL_CALCULUS_ALPHA, VOLITION_ALPHA,
};
use crate::economy::Rarity;
use crate::loot::LootKind;
use crate::settings::DEFAULT_FPS;
use crate::ui::{hints::HINT_CLOSE, input_action::HeldKeys, table};
use crate::util::{Metronome, hyperbolic_scale};

const DAMPING: f32 = 0.95;
const PLAYER_FORCE: f32 = 0.028;
const REEL_RATE: f32 = 0.009;
const COFFEE_REEL_RATE_PER_STACK: f32 = 0.002;
const WALL_BOUNCE: f32 = -0.5;
const EDGE_DRAIN_RATE: f32 = 0.004;
const EDGE_DRAIN_SLOPE: f32 = 1.0;
const BAD_REEL_PENALTY: f32 = 7.3;
pub const DANGER_THRESHOLD: f32 = 0.10;
pub const COMPLETION_START: f32 = 0.2;
const REEL_PENALTY_THRESHOLD: f32 = 0.45;
const BASE_GRACE_SECS: f32 = 1.0;
const MAX_GRACE_SECS: f32 = 11.0;
const OPENING_GRACE_SECS: f32 = 2.0;
const STRETCHED_BAND: f32 = COMPLETION_START;
const BOTTOM_STRETCH: f32 = 2.0;
const LOSS_FLOOR: f32 = STRETCHED_BAND * (1.0 - BOTTOM_STRETCH);
const CONTROL_COLOR: Color = WHITE;
const SAFE_WATER: Color = GREEN;
const DANGER_WATER: Color = RED;
const COMPLETION_WARN: f32 = 0.5;
const OVERLAY_FILL: f32 = 0.75;
const COMP_WIDTH: u16 = 3;
const INDICATOR_FRACTION: f32 = 0.15;
const ART_CONTENT_WIDTH: u16 = 16;
const ART_LINES: u16 = 6;
const CENTER_FRAME_IDX: usize = 2;
const REEL_HANDLE_FREQ: u32 = 6;
const BACKGROUND: Color = Color::Reset;

const FLASH_PERIOD: u32 = 6;
const REEL_STEP_SECS: f32 = 1.0 / DEFAULT_FPS;

const BITE_MIN_WAIT_SECS: f32 = 2.0;
const BITE_MEAN_WAIT_SECS: f32 = 5.0;
const BITE_MEAN_WAIT_BUFFED_SECS: f32 = 3.0;
const BITE_DURATION_BASE_SECS: f32 = 0.5;
const BITE_DURATION_BUFFED_SECS: f32 = 10.0;
const REACTION_SPEED_REF_STACKS: u32 = 10;
const BITE_DIP_COUNT: u32 = 2;

const HOOK_IDLE_PERIOD_SECS: f32 = 1.6;

const WAVE_GLYPH: &str = "~~~";
const WAVE_WIDTH: u16 = 3;
const WAVE_SPEED_CPS: f32 = 12.0;
const WAVE_SPAWN_MEAN_SECS: f32 = 0.4;
const WAVE_SPAWN_MIN_SECS: f32 = 0.08;
const WAVE_COLOR: Color = CYAN;
const INITIAL_WAVE_COUNT: u32 = 6;

const REEL_FOOTER_LEFT: &str = " ←→ control the fish  ↓ reel";
const REEL_FOOTER_SHORT: &str = " ←→ steer  ↓ reel";
const FOOTER_GAP: usize = 4;
const CATCH_FOOTER_LEFT: &str = " ↓ catch once fish bites";

const ART_FRAMES: [[&str; 6]; 5] = [
    [
        "       ﾄ､       ",
        "      ╱  ⟍      ",
        "    ⟋      ⟍    ",
        "(⊂ ' ⊃)     ╲   ",
        "             @  ",
        "             ⎹  ",
    ],
    [
        "       ﾄ､       ",
        "      |  ⟍      ",
        "     |     ⟍    ",
        "  (⊂ ' ⊃)   ╲   ",
        "             @  ",
        "             ⎹  ",
    ],
    [
        "        ﾄ､      ",
        "       j  ⟍     ",
        "      l    ⟍    ",
        "   (⊂ ' ⊃)  ╲   ",
        "             @  ",
        "             ⎹  ",
    ],
    [
        "         ﾄ､     ",
        "        j  ⟍    ",
        "       l    ⟍   ",
        "    (⊂ ' ⊃)  ╲  ",
        "              @ ",
        "              ⎹ ",
    ],
    [
        "           ﾄ､   ",
        "          j  ⟍  ",
        "         l    ⟍ ",
        "       (⊂ ' ⊃) ╲",
        "               @",
        "              ╱ ",
    ],
];

const CATCH_BITE_FRAME: [&str; 6] = [
    "                ",
    "        ﾄ⟍      ",
    "       j   ⟍    ",
    "   (⊂ l ⊃)  ╲   ",
    "             @  ",
    "             ⎹  ",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Temper {
    #[default]
    Normal,
    Legendary,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Sides {
    Alternate,
    Random,
}

struct Rest {
    chance: f64,
    spots: Range<f32>,
    steps: Range<f32>,
}

struct Temperament {
    pull: f32,
    top_speed: f32,
    rest: Option<Rest>,
    sides: Sides,
    dart_reach: Range<f32>,
    dart_steps: Range<f32>,
}

#[derive(Clone, Copy, Default, Debug)]
struct Swim {
    resting: bool,
    left: bool,
}

impl Temper {
    pub fn legendary_chance(rarity: Rarity) -> f64 {
        Rarity::Legendary.catch_weight() as f64 / rarity.catch_weight() as f64
    }

    pub fn roll(rarity: Rarity, rng: &mut impl RngExt) -> Self {
        if rng.random_bool(Self::legendary_chance(rarity).min(1.0)) {
            return Temper::Legendary;
        }
        Temper::Normal
    }

    fn temperament(self) -> Temperament {
        match self {
            Temper::Normal => Temperament {
                pull: 0.018,
                top_speed: 0.025,
                rest: None,
                sides: Sides::Alternate,
                dart_reach: 0.05..0.20,
                dart_steps: 45.0..90.0,
            },
            Temper::Legendary => Temperament {
                pull: 0.022,
                top_speed: 0.05,
                rest: Some(Rest {
                    chance: 0.3,
                    spots: 0.30..0.70,
                    steps: 20.0..50.0,
                }),
                sides: Sides::Random,
                dart_reach: 0.05..0.20,
                dart_steps: 20.0..50.0,
            },
        }
    }

    fn next_swim(self, swim: Swim, rng: &mut impl RngExt) -> (Swim, f32, f32) {
        let temperament = self.temperament();
        if let Some(rest) = temperament
            .rest
            .filter(|rest| !swim.resting && rng.random_bool(rest.chance))
        {
            let resting = Swim {
                resting: true,
                ..swim
            };
            return (
                resting,
                rng.random_range(rest.spots),
                rng.random_range(rest.steps),
            );
        }
        let left = match temperament.sides {
            Sides::Alternate => !swim.left,
            Sides::Random => rng.random(),
        };
        let reach = rng.random_range(temperament.dart_reach);
        let target = if left { reach } else { 1.0 - reach };
        (
            Swim {
                resting: false,
                left,
            },
            target,
            rng.random_range(temperament.dart_steps),
        )
    }
}

struct OceanWave {
    x: f32,
    row: u16,
}

struct CatchPhase {
    wait_remaining: f32,
    biting: bool,
    bite_elapsed: f32,
    bite_duration: f32,
}

impl CatchPhase {
    fn new(milk: MilkBuffs, rng: &mut impl RngExt) -> Self {
        Self {
            wait_remaining: milk.bite_wait_secs(rng),
            biting: false,
            bite_elapsed: 0.0,
            bite_duration: milk.bite_duration_secs(),
        }
    }
}

enum FishPhase {
    Catch(CatchPhase),
    Reel,
}

pub struct FishingState {
    phase: FishPhase,
    safe_zone: f32,
    waves: Vec<OceanWave>,
    waves_seeded: bool,
    wave_spawn_timer: f32,
    idle_timer: f32,
    reel_clock: Metronome,
    pub fish_pos: f32,
    pub completion: f32,
    pub fish_velocity: f32,
    pub target_pos: f32,
    pub target_timer: f32,
    pub danger_timer: f32,
    pub game_over: bool,
    pub captured: bool,
    pub is_reeling: bool,
    pub is_pushing_left: bool,
    pub is_pushing_right: bool,
    pub no_escape: bool,
    pub no_fight: bool,
    pub reel_punish_timer: u32,
    pub reel_anim_tick: u32,
    pub forced_temper: Option<Temper>,
    temper: Temper,
    swim: Swim,
    hooked: Option<LootKind>,
    reel_steps: u32,
}

impl Default for FishingState {
    fn default() -> Self {
        Self::new(MilkBuffs::default(), &mut rand::rng())
    }
}

impl FishingState {
    pub fn new(milk: MilkBuffs, rng: &mut impl RngExt) -> Self {
        Self {
            phase: FishPhase::Catch(CatchPhase::new(milk, rng)),
            safe_zone: milk.safe_zone(),
            waves: Vec::new(),
            waves_seeded: false,
            wave_spawn_timer: sample_exp(WAVE_SPAWN_MEAN_SECS, WAVE_SPAWN_MIN_SECS, rng),
            idle_timer: 0.0,
            reel_clock: Metronome::default(),
            fish_pos: 0.5,
            completion: COMPLETION_START,
            fish_velocity: 0.0,
            target_pos: 0.5,
            target_timer: 0.0,
            danger_timer: 0.0,
            game_over: false,
            captured: false,
            is_reeling: false,
            is_pushing_left: false,
            is_pushing_right: false,
            no_escape: false,
            no_fight: false,
            reel_punish_timer: 0,
            reel_anim_tick: 0,
            forced_temper: None,
            temper: Temper::default(),
            swim: Swim {
                resting: false,
                left: rng.random(),
            },
            hooked: None,
            reel_steps: 0,
        }
    }

    pub fn is_catching(&self) -> bool {
        matches!(self.phase, FishPhase::Catch(_))
    }

    pub fn is_biting(&self) -> bool {
        matches!(&self.phase, FishPhase::Catch(c) if c.biting)
    }

    fn start_reeling(&mut self) {
        self.phase = FishPhase::Reel;
    }

    pub fn hook(&mut self, catch: LootKind) {
        self.temper = self
            .forced_temper
            .unwrap_or_else(|| Temper::roll(catch.rarity(), &mut rand::rng()));
        self.hooked = Some(catch);
        self.start_reeling();
    }

    pub fn take_catch(&mut self) -> Option<LootKind> {
        self.hooked.take()
    }

    pub fn temper(&self) -> Temper {
        self.temper
    }

    pub fn shown_completion(&self) -> f32 {
        if self.completion >= STRETCHED_BAND {
            return self.completion;
        }
        STRETCHED_BAND - (STRETCHED_BAND - self.completion) / BOTTOM_STRETCH
    }

    fn opening_penalty(&self) -> f32 {
        let opened = (self.reel_steps as f32 * REEL_STEP_SECS / OPENING_GRACE_SECS).min(1.0);
        1.0 + (BAD_REEL_PENALTY - 1.0) * opened
    }

    pub fn hold(&mut self, keys: &HeldKeys) {
        self.is_pushing_left = keys.is_down(KeyCode::Left);
        self.is_pushing_right = keys.is_down(KeyCode::Right);
        self.is_reeling = !self.is_catching() && keys.is_certainly_down(KeyCode::Down);
    }

    pub fn tick(
        &mut self,
        fps: f32,
        coffee_stacks: u32,
        milk: MilkBuffs,
        geom: Option<FishingGeometry>,
    ) {
        if self.game_over || self.captured {
            return;
        }
        self.safe_zone = milk.safe_zone();
        let dt = 1.0 / fps;
        self.idle_timer = (self.idle_timer + dt).rem_euclid(HOOK_IDLE_PERIOD_SECS);
        self.tick_waves(dt, geom.as_ref());

        if !matches!(self.phase, FishPhase::Reel) {
            self.tick_catch(dt);
            return;
        }
        for _ in 0..self.reel_clock.beats(dt, REEL_STEP_SECS) {
            if self.game_over || self.captured {
                return;
            }
            self.tick_reel(coffee_stacks, milk);
        }
    }

    fn tick_catch(&mut self, dt: f32) {
        let FishPhase::Catch(c) = &mut self.phase else {
            return;
        };
        if c.biting {
            c.bite_elapsed += dt;
            if c.bite_elapsed >= c.bite_duration && !self.no_escape {
                self.game_over = true;
            }
        } else {
            c.wait_remaining -= dt;
            if c.wait_remaining <= 0.0 {
                c.biting = true;
                c.bite_elapsed = 0.0;
            }
        }
    }

    fn tick_waves(&mut self, dt: f32, geom: Option<&FishingGeometry>) {
        let Some(geom) = geom else {
            self.waves.clear();
            return;
        };

        if !self.waves_seeded {
            self.waves_seeded = true;
            let mut rng = rand::rng();
            for _ in 0..INITIAL_WAVE_COUNT {
                if let Some(row) = random_free_row(geom, &mut rng) {
                    self.waves.push(OceanWave {
                        x: rng.random_range(0.0..geom.art_w as f32),
                        row,
                    });
                }
            }
        }

        for w in &mut self.waves {
            w.x += WAVE_SPEED_CPS * dt;
        }
        self.waves
            .retain(|w| w.x < geom.art_w as f32 && w.row < geom.art_h && !geom.is_rod_row(w.row));

        self.wave_spawn_timer -= dt;
        if self.wave_spawn_timer <= 0.0 {
            let mut rng = rand::rng();
            self.wave_spawn_timer = sample_exp(WAVE_SPAWN_MEAN_SECS, WAVE_SPAWN_MIN_SECS, &mut rng);
            if let Some(row) = random_free_row(geom, &mut rng) {
                self.waves.push(OceanWave {
                    x: -(WAVE_WIDTH as f32),
                    row,
                });
            }
        }
    }

    fn tick_reel(&mut self, coffee_stacks: u32, milk: MilkBuffs) {
        let safe_zone = milk.safe_zone();
        let grace_secs = milk.grace_secs();
        let temperament = self.temper.temperament();

        if !self.no_fight {
            self.target_timer -= 1.0;
            if self.target_timer <= 0.0 {
                (self.swim, self.target_pos, self.target_timer) =
                    self.temper.next_swim(self.swim, &mut rand::rng());
            }
            let pull = (self.target_pos - self.fish_pos).signum() * temperament.pull;
            self.fish_velocity += pull * milk.fish_force_mult();
        }

        if self.is_pushing_left {
            self.fish_velocity -= PLAYER_FORCE;
        }
        if self.is_pushing_right {
            self.fish_velocity += PLAYER_FORCE;
        }

        self.fish_velocity *= DAMPING;
        self.fish_velocity = self
            .fish_velocity
            .clamp(-temperament.top_speed, temperament.top_speed);
        self.fish_pos = (self.fish_pos + self.fish_velocity).clamp(0.0, 1.0);

        if self.fish_pos <= 0.0 || self.fish_pos >= 1.0 {
            self.fish_velocity *= WALL_BOUNCE;
        }

        let abs_offset = (self.fish_pos - 0.5).abs() * 2.0;
        let drain = (EDGE_DRAIN_SLOPE * abs_offset) * EDGE_DRAIN_RATE;

        let reel_punish = self.is_reeling && abs_offset > safe_zone;

        let reel_rate = REEL_RATE + COFFEE_REEL_RATE_PER_STACK * coffee_stacks as f32;
        if self.is_reeling {
            if reel_punish {
                self.completion -= drain * self.opening_penalty();
            } else {
                self.completion += reel_rate;
            }
        } else {
            self.completion -= drain;
        }

        self.reel_punish_timer = if reel_punish {
            self.reel_punish_timer.saturating_add(1)
        } else {
            0
        };

        self.reel_anim_tick = if self.is_reeling {
            self.reel_anim_tick.wrapping_add(1)
        } else {
            0
        };

        self.completion = self.completion.clamp(LOSS_FLOOR, 1.0);
        self.reel_steps = self.reel_steps.saturating_add(1);

        if self.shown_completion() <= DANGER_THRESHOLD {
            self.danger_timer += 1.0;
            if self.danger_timer >= DEFAULT_FPS * grace_secs && !self.no_escape {
                self.game_over = true;
            }
        } else {
            self.danger_timer = 0.0;
        }

        if self.completion >= 1.0 {
            self.captured = true;
        }
    }
}

fn sample_exp(mean: f32, min: f32, rng: &mut impl RngExt) -> f32 {
    let u = rng.random::<f32>();
    (-mean * (1.0 - u).ln()).max(min)
}

fn random_free_row(geom: &FishingGeometry, rng: &mut impl RngExt) -> Option<u16> {
    let free: Vec<u16> = (0..geom.art_h).filter(|&r| !geom.is_rod_row(r)).collect();
    if free.is_empty() {
        return None;
    }
    Some(free[rng.random_range(0..free.len())])
}

#[derive(Clone, Copy, Default)]
pub struct MilkBuffs {
    pub visual_calculus: u32,
    pub volition: u32,
    pub physical_instrument: u32,
    pub reaction_speed: u32,
}

fn hyperbolic_ramp(stacks: u32, alpha: f32) -> f32 {
    1.0 - hyperbolic_scale(1.0, stacks, alpha)
}

impl MilkBuffs {
    fn safe_zone(&self) -> f32 {
        if self.visual_calculus == 0 {
            return REEL_PENALTY_THRESHOLD;
        }
        let ramp = hyperbolic_ramp(self.visual_calculus, VISUAL_CALCULUS_ALPHA);
        REEL_PENALTY_THRESHOLD + (1.0 - REEL_PENALTY_THRESHOLD) * ramp
    }

    fn grace_secs(&self) -> f32 {
        if self.volition == 0 {
            return BASE_GRACE_SECS;
        }
        let ramp = hyperbolic_ramp(self.volition, VOLITION_ALPHA);
        BASE_GRACE_SECS + (MAX_GRACE_SECS - BASE_GRACE_SECS) * ramp
    }

    fn fish_force_mult(&self) -> f32 {
        if self.physical_instrument == 0 {
            return 1.0;
        }
        1.0 - hyperbolic_ramp(self.physical_instrument, PHYSICAL_INSTRUMENT_ALPHA)
    }

    fn reaction_ramp(&self) -> f32 {
        if self.reaction_speed == 0 {
            return 0.0;
        }
        hyperbolic_ramp(self.reaction_speed, REACTION_SPEED_ALPHA)
            / hyperbolic_ramp(REACTION_SPEED_REF_STACKS, REACTION_SPEED_ALPHA)
    }

    fn bite_duration_secs(&self) -> f32 {
        BITE_DURATION_BASE_SECS
            + (BITE_DURATION_BUFFED_SECS - BITE_DURATION_BASE_SECS) * self.reaction_ramp()
    }

    fn bite_mean_wait_secs(&self) -> f32 {
        BITE_MEAN_WAIT_SECS
            + (BITE_MEAN_WAIT_BUFFED_SECS - BITE_MEAN_WAIT_SECS) * self.reaction_ramp()
    }

    pub fn bite_wait_secs(&self, rng: &mut impl RngExt) -> f32 {
        sample_exp(self.bite_mean_wait_secs(), BITE_MIN_WAIT_SECS, rng)
    }
}

pub struct FishingGeometry {
    ox: u16,
    oy: u16,
    overlay_w: u16,
    overlay_h: u16,
    inner_x: u16,
    inner_w: u16,
    art_x: u16,
    art_y: u16,
    art_w: u16,
    art_h: u16,
    art_vert_pad: u16,
    sep_x: u16,
    comp_x: u16,
}

impl FishingGeometry {
    pub fn from_area(area: Rect) -> Option<Self> {
        if area.width < 6 || area.height < 6 {
            return None;
        }
        let max_w = ((area.width as f32 * OVERLAY_FILL) as u16)
            .max(6)
            .min(area.width);
        let max_h = ((area.height as f32 * OVERLAY_FILL) as u16)
            .max(6)
            .min(area.height);
        let side = (max_w / 2).min(max_h);
        let overlay_w = (side * 2).max(6);
        let overlay_h = side.max(6);
        let layout = table::OverlayLayout::centered(area, overlay_w, overlay_h)?;

        let inner_w = overlay_w.saturating_sub(2);
        let art_w = inner_w.saturating_sub(1 + COMP_WIDTH);
        let art_h = overlay_h.saturating_sub(6);
        let art_vert_pad = art_h.saturating_sub(ART_LINES) / 2;
        let art_x_offset = art_w.saturating_sub(ART_CONTENT_WIDTH) / 2;
        let inner_x = layout.ox + 1;
        let sep_x = inner_x + art_w;

        Some(Self {
            ox: layout.ox,
            oy: layout.oy,
            overlay_w,
            overlay_h,
            inner_x,
            inner_w,
            art_x: inner_x + art_x_offset,
            art_y: layout.oy + 1,
            art_w,
            art_h,
            art_vert_pad,
            sep_x,
            comp_x: sep_x + 1,
        })
    }

    pub fn catch_layout(area: Rect) -> Option<Self> {
        let dims = Self::from_area(area)?;
        let art_w = dims.art_w;
        let art_h = dims.art_h;
        let box_w = art_w + 2;
        let box_h = art_h + 4;
        let layout = table::OverlayLayout::centered(area, box_w, box_h)?;

        let art_x_offset = art_w.saturating_sub(ART_CONTENT_WIDTH) / 2;
        let inner_x = layout.ox + 1;
        let sep_x = inner_x + art_w;

        Some(Self {
            ox: layout.ox,
            oy: layout.oy,
            overlay_w: box_w,
            overlay_h: box_h,
            inner_x,
            inner_w: art_w,
            art_x: inner_x + art_x_offset,
            art_y: layout.oy + 1,
            art_w,
            art_h,
            art_vert_pad: dims.art_vert_pad,
            sep_x,
            comp_x: sep_x + 1,
        })
    }

    fn is_rod_row(&self, row: u16) -> bool {
        row >= self.art_vert_pad && row < self.art_vert_pad + ART_LINES
    }
}

pub struct FishingOverlay<'a> {
    state: &'a FishingState,
}

impl<'a> FishingOverlay<'a> {
    pub fn new(state: &'a FishingState) -> Self {
        Self { state }
    }
}

impl Widget for FishingOverlay<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let state = self.state;
        let reeling = matches!(state.phase, FishPhase::Reel);
        let geom = if reeling {
            FishingGeometry::from_area(area)
        } else {
            FishingGeometry::catch_layout(area)
        };
        let Some(geom) = geom else {
            return;
        };

        let layout = table::OverlayLayout {
            ox: geom.ox,
            oy: geom.oy,
            w: geom.overlay_w,
            h: geom.overlay_h,
        };
        layout.clear_bg(buf, BACKGROUND);

        let bcolor = state_border_color(state);
        draw_border(
            buf,
            geom.ox,
            geom.oy,
            geom.overlay_w,
            geom.overlay_h,
            bcolor,
        );

        let alt_handle =
            reeling && state.is_reeling && (state.reel_anim_tick / REEL_HANDLE_FREQ) % 2 == 1;
        draw_art_block(
            buf,
            &geom,
            current_frame(state),
            alt_handle,
            hook_spread(state),
        );
        draw_waves(buf, &geom, &state.waves);

        match &state.phase {
            FishPhase::Reel => draw_reel_panels(buf, &geom, state, bcolor),
            FishPhase::Catch(_) => {
                let sep_y = geom.art_y + geom.art_h;
                draw_inner_separator(buf, geom.ox, sep_y, geom.overlay_w, bcolor);
                draw_footer(
                    buf,
                    geom.inner_x,
                    sep_y + 1,
                    geom.inner_w,
                    CATCH_FOOTER_LEFT,
                );
            }
        }
    }
}

fn current_frame(state: &FishingState) -> &'static [&'static str; 6] {
    match &state.phase {
        FishPhase::Reel => &ART_FRAMES[art_frame_idx(state.fish_pos)],
        FishPhase::Catch(c) => catch_frame(c),
    }
}

fn catch_frame(c: &CatchPhase) -> &'static [&'static str; 6] {
    if c.biting {
        let seg = (c.bite_elapsed / c.bite_duration * (BITE_DIP_COUNT * 2) as f32) as u32;
        if seg.is_multiple_of(2) {
            return &CATCH_BITE_FRAME;
        }
    }
    &ART_FRAMES[CENTER_FRAME_IDX]
}

fn hook_spread(state: &FishingState) -> bool {
    if let FishPhase::Catch(c) = &state.phase
        && c.biting
    {
        return false;
    }
    state.idle_timer >= HOOK_IDLE_PERIOD_SECS / 2.0
}

fn art_frame_idx(fish_pos: f32) -> usize {
    if fish_pos < 0.2 {
        0
    } else if fish_pos < 0.4 {
        1
    } else if fish_pos < 0.6 {
        2
    } else if fish_pos < 0.8 {
        3
    } else {
        4
    }
}

fn flash_on(tick: u32) -> bool {
    tick % FLASH_PERIOD < FLASH_PERIOD / 2
}

fn state_border_color(state: &FishingState) -> Color {
    if matches!(state.phase, FishPhase::Catch(_)) {
        return WHITE;
    }
    if state.danger_timer > 0.0 {
        if flash_on(state.danger_timer as u32) {
            LIGHT_RED
        } else {
            WHITE
        }
    } else if state.reel_punish_timer > 0 {
        if flash_on(state.reel_punish_timer) {
            LIGHT_RED
        } else {
            WHITE
        }
    } else {
        WHITE
    }
}

fn draw_border(buf: &mut Buffer, ox: u16, oy: u16, w: u16, h: u16, border_color: Color) {
    if w < 2 || h < 2 {
        return;
    }
    let border_style = Style::default().fg(border_color).bg(BACKGROUND);
    let title_style = Style::default()
        .fg(WHITE)
        .add_modifier(Modifier::BOLD)
        .bg(BACKGROUND);
    let right = ox + w - 1;
    let bottom = oy + h - 1;

    buf[(ox, oy)].set_char('┌').set_style(border_style);
    buf[(right, oy)].set_char('┐').set_style(border_style);
    buf[(ox, bottom)].set_char('└').set_style(border_style);
    buf[(right, bottom)].set_char('┘').set_style(border_style);

    for dx in 1..w - 1 {
        buf[(ox + dx, oy)].set_char('─').set_style(border_style);
        buf[(ox + dx, bottom)].set_char('─').set_style(border_style);
    }
    for dy in 1..h - 1 {
        buf[(ox, oy + dy)].set_char('│').set_style(border_style);
        buf[(right, oy + dy)].set_char('│').set_style(border_style);
    }

    let title = " Fishing ";
    if (title.len() as u16 + 4) < w {
        buf.set_string(ox + 2, oy, title, title_style);
    }
}

fn draw_inner_separator(buf: &mut Buffer, ox: u16, y: u16, w: u16, color: Color) {
    if w < 2 {
        return;
    }
    let style = Style::default().fg(color).bg(BACKGROUND);
    buf[(ox, y)].set_char('├').set_style(style);
    buf[(ox + w - 1, y)].set_char('┤').set_style(style);
    for dx in 1..w - 1 {
        buf[(ox + dx, y)].set_char('─').set_style(style);
    }
}

fn draw_art_block(
    buf: &mut Buffer,
    geom: &FishingGeometry,
    frame: &[&str; 6],
    alt_handle: bool,
    spread: bool,
) {
    let art_style = Style::default().fg(WHITE).bg(BACKGROUND);
    for row in 0..geom.art_h {
        let y = geom.art_y + row;
        for dx in 0..geom.art_w {
            buf[(geom.inner_x + dx, y)]
                .set_char(' ')
                .set_style(art_style);
        }

        let art_line = row
            .checked_sub(geom.art_vert_pad)
            .and_then(|r| (r < ART_LINES).then(|| frame[r as usize]));
        let Some(line) = art_line else {
            continue;
        };

        let content_w = ART_CONTENT_WIDTH.min(geom.art_w.saturating_sub(geom.art_x - geom.inner_x));
        let swapped;
        let line = if alt_handle && line.contains('@') {
            swapped = line.replace('@', "Ə");
            swapped.as_str()
        } else {
            line
        };
        draw_art_content(buf, geom.art_x, y, line, content_w, art_style);

        if spread && line.contains('(') {
            apply_hook_spread(buf, geom, y, line, art_style);
        }
    }
}

fn apply_hook_spread(buf: &mut Buffer, geom: &FishingGeometry, y: u16, line: &str, style: Style) {
    let mut left = None;
    let mut right = None;
    let mut col = 0u16;
    for ch in line.chars() {
        if ch == '(' && left.is_none() {
            left = Some(col);
        }
        if ch == ')' {
            right = Some(col);
        }
        col += UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
    }
    let (Some(left), Some(right)) = (left, right) else {
        return;
    };

    if geom.art_x + left > geom.inner_x {
        buf[(geom.art_x + left, y)].set_char(' ').set_style(style);
        buf[(geom.art_x + left - 1, y)]
            .set_char('(')
            .set_style(style);
    }
    if geom.art_x + right + 1 < geom.inner_x + geom.art_w {
        buf[(geom.art_x + right, y)].set_char(' ').set_style(style);
        buf[(geom.art_x + right + 1, y)]
            .set_char(')')
            .set_style(style);
    }
}

fn draw_waves(buf: &mut Buffer, geom: &FishingGeometry, waves: &[OceanWave]) {
    let style = Style::default().fg(WAVE_COLOR).bg(BACKGROUND);
    for wave in waves {
        let y = geom.art_y + wave.row;
        let base = wave.x.floor() as i32;
        for (i, ch) in WAVE_GLYPH.chars().enumerate() {
            let col = base + i as i32;
            if col < 0 || col >= geom.art_w as i32 {
                continue;
            }
            buf[(geom.inner_x + col as u16, y)]
                .set_char(ch)
                .set_style(style);
        }
    }
}

fn draw_reel_panels(buf: &mut Buffer, geom: &FishingGeometry, state: &FishingState, bcolor: Color) {
    for row in 0..geom.art_h {
        let y = geom.art_y + row;
        buf[(geom.sep_x, y)]
            .set_char('│')
            .set_fg(DARK_GRAY)
            .set_bg(BACKGROUND);
        draw_comp_row(buf, geom.comp_x, y, row, geom.art_h, state);
    }

    let sep_y = geom.art_y + geom.art_h;
    draw_inner_separator(buf, geom.ox, sep_y, geom.overlay_w, bcolor);

    let ctrl_y = sep_y + 1;
    draw_control_bar(buf, geom.inner_x, ctrl_y, geom.inner_w, state);

    let sep2_y = ctrl_y + 1;
    draw_inner_separator(buf, geom.ox, sep2_y, geom.overlay_w, bcolor);

    draw_footer(
        buf,
        geom.inner_x,
        sep2_y + 1,
        geom.inner_w,
        reel_footer(geom.inner_w),
    );
}

fn draw_art_content(buf: &mut Buffer, x: u16, y: u16, line: &str, max_w: u16, style: Style) {
    let mut col = 0u16;
    for ch in line.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
        if col + cw > max_w {
            break;
        }
        buf[(x + col, y)].set_char(ch).set_style(style);
        if cw == 2 && col + 1 < max_w {
            buf[(x + col + 1, y)].set_char(' ').set_style(style);
        }
        col += cw;
    }
}

fn draw_comp_row(buf: &mut Buffer, x: u16, y: u16, row: u16, art_h: u16, state: &FishingState) {
    let in_danger = state.danger_timer > 0.0;
    let danger_flash = flash_on(state.danger_timer as u32);

    let shown = state.shown_completion();
    let empty_rows = (art_h as f32 * (1.0 - shown)) as u16;
    let filled = row >= empty_rows;

    let (content, fg) = if state.captured {
        ("███", LIGHT_YELLOW)
    } else if filled {
        let c = if shown <= DANGER_THRESHOLD {
            if in_danger && danger_flash {
                LIGHT_RED
            } else {
                RED
            }
        } else if shown < COMPLETION_WARN {
            LIGHT_YELLOW
        } else {
            LIGHT_GREEN
        };
        ("███", c)
    } else {
        ("   ", BACKGROUND)
    };

    buf.set_string(x, y, content, Style::default().fg(fg).bg(BACKGROUND));
}

fn draw_control_bar(buf: &mut Buffer, x: u16, y: u16, inner_w: u16, state: &FishingState) {
    if inner_w < 4 {
        return;
    }
    let indicator_w = ((inner_w as f32 * INDICATOR_FRACTION) as u16).max(1) | 1;
    let movable = inner_w.saturating_sub(2 + indicator_w);
    let indicator_col = (state.fish_pos * movable as f32) as u16;
    let control_color = if state.captured {
        LIGHT_YELLOW
    } else {
        CONTROL_COLOR
    };

    let bracket_style = Style::default().fg(DARK_GRAY).bg(BACKGROUND);
    let indicator_style = Style::default()
        .fg(control_color)
        .add_modifier(Modifier::BOLD)
        .bg(BACKGROUND);

    buf[(x, y)].set_char('[').set_style(bracket_style);
    buf[(x + inner_w - 1, y)]
        .set_char(']')
        .set_style(bracket_style);

    let ind_start = 1 + indicator_col;
    let ind_end = ind_start + indicator_w;
    for dx in 1..inner_w - 1 {
        if dx >= ind_start && dx < ind_end {
            buf[(x + dx, y)].set_char('█').set_style(indicator_style);
            continue;
        }
        let water = water_color(dx, indicator_w, movable, state.safe_zone);
        buf[(x + dx, y)]
            .set_char('─')
            .set_style(Style::default().fg(water).bg(BACKGROUND));
    }
}

fn water_color(dx: u16, indicator_w: u16, movable: u16, safe_zone: f32) -> Color {
    let col = dx as f32 - 1.0 - (indicator_w / 2) as f32;
    if movable == 0 || col < 0.0 || col > movable as f32 {
        return DANGER_WATER;
    }
    let pos = ((col + 0.5) / movable as f32).min(1.0);
    if (pos - 0.5).abs() * 2.0 > safe_zone {
        return DANGER_WATER;
    }
    SAFE_WATER
}

fn draw_footer(buf: &mut Buffer, x: u16, y: u16, inner_w: u16, left: &str) {
    let style = Style::default().fg(DARK_GRAY).bg(BACKGROUND);
    let right = HINT_CLOSE;
    let total = inner_w as usize;
    buf.set_string(x, y, truncate_to_width(left, total), style);
    let left_w = visual_width(left);
    let right_w = visual_width(right);
    if left_w + FOOTER_GAP + right_w <= total {
        buf.set_string(x + total as u16 - right_w as u16 - 1, y, right, style);
    }
}

fn reel_footer(inner_w: u16) -> &'static str {
    let whole = visual_width(REEL_FOOTER_LEFT) + FOOTER_GAP + visual_width(HINT_CLOSE);
    if whole <= inner_w as usize {
        return REEL_FOOTER_LEFT;
    }
    REEL_FOOTER_SHORT
}

fn visual_width(s: &str) -> usize {
    s.chars()
        .map(|c| UnicodeWidthChar::width(c).unwrap_or(1))
        .sum()
}

fn truncate_to_width(s: &str, max_w: usize) -> String {
    let mut out = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(1);
        if w + cw > max_w {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out
}

#[cfg(test)]
mod milk_buff_spec_tests {
    use super::*;

    fn approx(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn visual_calculus_hits_80_percent_safe_at_10_stacks() {
        let buffs = MilkBuffs {
            visual_calculus: 10,
            ..Default::default()
        };
        assert!(approx(buffs.safe_zone(), 0.80, 0.005));
    }

    #[test]
    fn volition_hits_10_seconds_grace_at_10_stacks() {
        let buffs = MilkBuffs {
            volition: 10,
            ..Default::default()
        };
        assert!(approx(buffs.grace_secs(), 10.0, 0.05));
    }

    #[test]
    fn physical_instrument_hits_10_percent_fish_force_at_10_stacks() {
        let buffs = MilkBuffs {
            physical_instrument: 10,
            ..Default::default()
        };
        assert!(approx(buffs.fish_force_mult(), 0.10, 0.005));
    }

    #[test]
    fn reaction_speed_hits_10_second_bite_at_10_stacks() {
        let buffs = MilkBuffs {
            reaction_speed: 10,
            ..Default::default()
        };
        assert!(approx(buffs.bite_duration_secs(), 10.0, 0.005));
    }

    #[test]
    fn reaction_speed_hits_3_second_mean_wait_at_10_stacks() {
        let buffs = MilkBuffs {
            reaction_speed: 10,
            ..Default::default()
        };
        assert!(approx(buffs.bite_mean_wait_secs(), 3.0, 0.005));
    }

    #[test]
    fn zero_stacks_returns_base_values() {
        let buffs = MilkBuffs::default();
        assert_eq!(buffs.safe_zone(), REEL_PENALTY_THRESHOLD);
        assert_eq!(buffs.grace_secs(), BASE_GRACE_SECS);
        assert_eq!(buffs.fish_force_mult(), 1.0);
        assert_eq!(buffs.bite_duration_secs(), BITE_DURATION_BASE_SECS);
        assert_eq!(buffs.bite_mean_wait_secs(), BITE_MEAN_WAIT_SECS);
    }

    #[test]
    fn reaction_speed_does_not_affect_reel_buffs() {
        let buffs = MilkBuffs {
            reaction_speed: 99,
            ..Default::default()
        };
        assert_eq!(buffs.safe_zone(), REEL_PENALTY_THRESHOLD);
        assert_eq!(buffs.grace_secs(), BASE_GRACE_SECS);
        assert_eq!(buffs.fish_force_mult(), 1.0);
    }
}

#[cfg(test)]
mod reel_clock_tests {
    use super::*;

    const ONE_SECOND_OF_STEPS: f32 = DEFAULT_FPS;

    #[test]
    fn a_second_of_reeling_fills_the_bar_as_far_at_any_frame_rate() {
        let expected = COMPLETION_START + ONE_SECOND_OF_STEPS * REEL_RATE;
        for fps in [1.0, 30.0, 60.0, 120.0] {
            let mut state = FishingState {
                no_fight: true,
                is_reeling: true,
                ..FishingState::default()
            };
            state.start_reeling();
            for _ in 0..fps as usize {
                state.tick(fps, 0, MilkBuffs::default(), None);
            }
            assert!(
                (state.completion - expected).abs() < 1e-4,
                "at {fps} fps a second of reeling reached {}",
                state.completion
            );
        }
    }
}

#[cfg(test)]
mod cast_cycle_tests {
    use super::*;
    use crate::consumable::{BAIT_DURATION, CASTS_PER_BUFF};

    const CARD_READ_SECS: f32 = 4.0;

    #[test]
    fn a_buff_lasts_the_casts_a_human_makes_in_a_minute_of_bait() {
        let bite_wait = BITE_MIN_WAIT_SECS
            + BITE_MEAN_WAIT_SECS * (-BITE_MIN_WAIT_SECS / BITE_MEAN_WAIT_SECS).exp();
        let reel = (1.0 - COMPLETION_START) / REEL_RATE / DEFAULT_FPS;
        let cycle = bite_wait + reel + CARD_READ_SECS;
        assert_eq!(CASTS_PER_BUFF, (BAIT_DURATION / cycle).round() as u32);
    }
}

#[cfg(test)]
mod temper_tests {
    use std::collections::VecDeque;

    use super::*;

    const FIGHTS: usize = 200;
    const HOLD_ONLY_FIGHTS: usize = 2000;
    const REACTION_STEPS: usize = 8;
    const LOOKAHEAD_STEPS: f32 = 3.0;
    const STEER_DEADBAND: f32 = 0.06;
    const CAREFUL_REEL_ZONE: f32 = 0.32;
    const FIGHT_STEPS: usize = 30 * 120;
    const ROLLS: usize = 20_000;
    const CHANCE_SLACK: f64 = 0.02;
    const SWIMS: usize = 1000;

    #[derive(Clone, Copy, PartialEq)]
    enum Angler {
        HoldsDown,
        ReelsOnGreen,
        Steers,
    }

    fn lands(temper: Temper, angler: Angler) -> bool {
        let mut state = FishingState {
            forced_temper: Some(temper),
            ..FishingState::default()
        };
        state.hook(LootKind::Food(1));
        let mut seen: VecDeque<(f32, f32)> = VecDeque::new();
        for _ in 0..FIGHT_STEPS {
            seen.push_back((state.fish_pos, state.fish_velocity));
            if seen.len() > REACTION_STEPS {
                seen.pop_front();
            }
            let (pos, velocity) = seen[0];
            let ahead = pos + velocity * (REACTION_STEPS as f32 + LOOKAHEAD_STEPS);
            let steers = angler == Angler::Steers;
            state.is_pushing_left = steers && ahead > 0.5 + STEER_DEADBAND;
            state.is_pushing_right = steers && ahead < 0.5 - STEER_DEADBAND;
            state.is_reeling =
                angler == Angler::HoldsDown || (ahead - 0.5).abs() * 2.0 < CAREFUL_REEL_ZONE;
            state.tick(DEFAULT_FPS, 0, MilkBuffs::default(), None);
            if state.captured || state.game_over {
                return state.captured;
            }
        }
        false
    }

    fn landed(temper: Temper, angler: Angler, fights: usize) -> usize {
        (0..fights).filter(|_| lands(temper, angler)).count()
    }

    #[test]
    fn holding_down_and_nothing_else_never_lands_a_fish() {
        for temper in [Temper::Normal, Temper::Legendary] {
            assert_eq!(
                landed(temper, Angler::HoldsDown, HOLD_ONLY_FIGHTS),
                0,
                "{temper:?}"
            );
            assert_eq!(
                landed(temper, Angler::ReelsOnGreen, FIGHTS),
                0,
                "{temper:?}"
            );
        }
    }

    #[test]
    fn an_angler_who_steers_against_the_pull_lands_a_normal_fish() {
        assert!(landed(Temper::Normal, Angler::Steers, FIGHTS) * 10 >= FIGHTS * 9);
    }

    #[test]
    fn a_normal_fish_pulls_to_alternate_sides_and_never_rests() {
        let mut rng = rand::rng();
        let mut swim = Swim::default();
        for _ in 0..SWIMS {
            let (next, target, _) = Temper::Normal.next_swim(swim, &mut rng);
            assert!(!next.resting);
            assert_ne!(next.left, swim.left, "it pulls the other way each time");
            assert_eq!(target < 0.5, next.left);
            swim = next;
        }
    }

    #[test]
    fn a_legendary_fish_never_rests_twice_in_a_row() {
        let mut rng = rand::rng();
        let mut swim = Swim::default();
        let mut rested = 0;
        for _ in 0..SWIMS {
            let (next, _, _) = Temper::Legendary.next_swim(swim, &mut rng);
            assert!(!(swim.resting && next.resting));
            rested += usize::from(next.resting);
            swim = next;
        }
        assert!(rested > 0, "it still rests");
    }

    #[test]
    fn a_normal_fish_swims_slower_than_a_legendary_one() {
        let top_speed = |temper: Temper| {
            let mut state = FishingState {
                forced_temper: Some(temper),
                no_escape: true,
                ..FishingState::default()
            };
            state.hook(LootKind::Food(1));
            (0..FIGHT_STEPS)
                .map(|_| {
                    state.tick(DEFAULT_FPS, 0, MilkBuffs::default(), None);
                    state.fish_velocity.abs()
                })
                .fold(0.0, f32::max)
        };
        let normal = Temper::Normal.temperament().top_speed;
        assert!(top_speed(Temper::Normal) <= normal + f32::EPSILON);
        assert!(top_speed(Temper::Legendary) > normal);
    }

    #[test]
    fn a_legendary_catch_always_fights_like_a_legendary() {
        let mut rng = rand::rng();
        assert!((0..ROLLS).all(|_| Temper::roll(Rarity::Legendary, &mut rng) == Temper::Legendary));
    }

    #[test]
    fn a_common_catch_rarely_fights_like_a_legendary_and_a_rare_one_more_often() {
        let mut rng = rand::rng();
        for rarity in [Rarity::Common, Rarity::Rare] {
            let legendary = (0..ROLLS)
                .filter(|_| Temper::roll(rarity, &mut rng) == Temper::Legendary)
                .count() as f64
                / ROLLS as f64;
            let expected = Temper::legendary_chance(rarity);
            assert!((legendary - expected).abs() < CHANCE_SLACK, "{legendary}");
        }
        assert!(Temper::legendary_chance(Rarity::Common) < Temper::legendary_chance(Rarity::Rare));
        assert!(Temper::legendary_chance(Rarity::Rare) < 1.0);
    }

    #[test]
    fn the_bottom_of_the_bar_holds_twice_its_height() {
        let mut state = FishingState {
            completion: LOSS_FLOOR,
            ..FishingState::default()
        };
        assert_eq!(state.shown_completion(), 0.0);
        state.completion = STRETCHED_BAND;
        assert_eq!(state.shown_completion(), STRETCHED_BAND);
        state.completion = 0.0;
        assert!((state.shown_completion() - STRETCHED_BAND / BOTTOM_STRETCH).abs() < 1e-6);
        state.completion = 0.6;
        assert_eq!(
            state.shown_completion(),
            0.6,
            "the top of the bar is unchanged"
        );
    }

    #[test]
    fn a_fish_at_the_wall_takes_twice_as_long_to_reach_danger() {
        let mut state = FishingState {
            no_fight: true,
            ..FishingState::default()
        };
        state.hook(LootKind::Food(1));
        state.fish_pos = 0.0;
        let mut steps = 0;
        while state.danger_timer == 0.0 {
            state.tick(DEFAULT_FPS, 0, MilkBuffs::default(), None);
            steps += 1;
        }
        let unstretched = (COMPLETION_START - DANGER_THRESHOLD) / EDGE_DRAIN_RATE;
        let stretched = (COMPLETION_START - DANGER_THRESHOLD) * BOTTOM_STRETCH / EDGE_DRAIN_RATE;
        assert!(steps as f32 > unstretched * 1.9, "{steps}");
        assert!((steps as f32 - stretched).abs() <= 2.0, "{steps}");
    }

    #[test]
    fn a_bad_reel_costs_only_the_drain_at_the_strike_and_its_full_penalty_after_the_opening() {
        let mut state = FishingState::default();
        state.hook(LootKind::Food(1));
        assert_eq!(state.opening_penalty(), 1.0);
        let opening_steps = (OPENING_GRACE_SECS / REEL_STEP_SECS).round() as u32;
        state.reel_steps = opening_steps / 2;
        assert!((state.opening_penalty() - (1.0 + BAD_REEL_PENALTY) / 2.0).abs() < 1e-3);
        state.reel_steps = opening_steps;
        assert_eq!(state.opening_penalty(), BAD_REEL_PENALTY);
    }
}

#[cfg(test)]
mod water_tests {
    use super::*;

    const BAR_W: u16 = 60;

    fn water(state: &FishingState) -> Vec<Color> {
        let area = Rect::new(0, 0, BAR_W, 1);
        let mut buf = Buffer::empty(area);
        draw_control_bar(&mut buf, 0, 0, BAR_W, state);
        (1..BAR_W - 1)
            .filter(|&x| buf[(x, 0)].symbol() == "─")
            .map(|x| buf[(x, 0)].fg)
            .collect()
    }

    fn safe_cells(state: &FishingState) -> usize {
        water(state).iter().filter(|&&c| c == SAFE_WATER).count()
    }

    #[test]
    fn the_water_is_green_in_the_middle_red_at_the_sides_and_the_control_keeps_its_colour() {
        let mut state = FishingState::default();
        for pos in [0.0, 0.5, 1.0] {
            state.fish_pos = pos;
            let cells = water(&state);
            assert!(cells.iter().all(|&c| c == SAFE_WATER || c == DANGER_WATER));
            let area = Rect::new(0, 0, BAR_W, 1);
            let mut buf = Buffer::empty(area);
            draw_control_bar(&mut buf, 0, 0, BAR_W, &state);
            let control: Vec<Color> = (0..BAR_W)
                .filter(|&x| buf[(x, 0)].symbol() == "█")
                .map(|x| buf[(x, 0)].fg)
                .collect();
            assert!(!control.is_empty() && control.iter().all(|&c| c == CONTROL_COLOR));
        }
        state.fish_pos = 0.0;
        let cells = water(&state);
        assert_eq!(*cells.last().unwrap(), DANGER_WATER);
        state.fish_pos = 1.0;
        let cells = water(&state);
        assert_eq!(cells[0], DANGER_WATER);
        assert!(cells.contains(&SAFE_WATER));
    }

    #[test]
    fn a_wider_safe_zone_is_more_green_water() {
        let plain = FishingState::default();
        let calculated = FishingState::new(
            MilkBuffs {
                visual_calculus: 10,
                ..MilkBuffs::default()
            },
            &mut rand::rng(),
        );
        assert!(safe_cells(&calculated) > safe_cells(&plain));
    }
}
