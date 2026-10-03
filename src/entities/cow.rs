use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

use rand::RngExt;
use ratatui::style::Color;

use crate::colors::{
    AMBER, AMBER_DARK, BROWN, DARK_GRAY, GOLD_BRIGHT, LIGHT_GREEN, LIGHT_YELLOW, NAVY, OLIVE, PINK,
    WHITE,
};
use crate::entities::components::{Position, SwayState, tick_sway};
use crate::entities::speech::SpeechBubble;
use crate::fishes::fused::FusedComponent;
use crate::fishes::mutant::{Circadian, EyeState, MutantState, MutantTail, MutationRecord};
use crate::fishes::mutations::{
    MutantBacked, Mutatable, Mutation, MutationOutcome, apply_mutant_mutation,
};
use crate::loot::MilkVariant;
use crate::sprite::{Band, BodyExtension};
use crate::tank::Sky;

mod record;

use record::CowRecord;

pub const COW_SPRITE_ROWS: u16 = 5;
pub const COW_BASE_TORSO: usize = 7;
const COW_HEAD_CAP: usize = 5;
const COW_HEAD_TOP_W: usize = 4;
pub const COW_DEFAULT_SWAY_SPEED: f32 = 0.05;
pub const COW_TRANSPARENT: char = crate::sprite::TRANSPARENT;
pub const COW_ANTENNA_ROWS: u16 = 1;
const ANTENNA_BALL: char = 'o';
const ANTENNA_LEFT_STEM: char = '\\';
const ANTENNA_RIGHT_STEM: char = '/';
const ANTENNA_STEM_FILL: char = '_';
const COW_UDDER: char = 'w';
const COW_BELLY: char = '-';
const COW_TAIL_W: usize = 4;
const COW_LEFT_TAIL: [char; COW_TAIL_W] = ['/', '\\', '/', '('];
const COW_RIGHT_TAIL: [char; COW_TAIL_W] = [')', '\\', '/', '\\'];

pub fn cow_default_sway_speed() -> f32 {
    COW_DEFAULT_SWAY_SPEED
}
const COW_MIN_TORSO: usize = 3;
const COW_MAX_TORSO: usize = 12;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum CowVariant {
    Brown,
    WhiteBlack,
    Pink,
    LightYellow,
    LightGreen,
    Blue,
    Amber,
    Olive,
}

impl CowVariant {
    pub const ALL: &'static [CowVariant] = &[
        CowVariant::Brown,
        CowVariant::WhiteBlack,
        CowVariant::Pink,
        CowVariant::LightYellow,
        CowVariant::LightGreen,
        CowVariant::Blue,
        CowVariant::Amber,
        CowVariant::Olive,
    ];

    pub fn body_color(self) -> Color {
        match self {
            CowVariant::Brown => BROWN,
            CowVariant::WhiteBlack => WHITE,
            CowVariant::Pink => PINK,
            CowVariant::LightYellow => LIGHT_YELLOW,
            CowVariant::LightGreen => LIGHT_GREEN,
            CowVariant::Blue => NAVY,
            CowVariant::Amber => AMBER,
            CowVariant::Olive => OLIVE,
        }
    }

    pub fn milk(self) -> MilkVariant {
        match self {
            CowVariant::Brown => MilkVariant::Chocolate,
            CowVariant::WhiteBlack => MilkVariant::Plain,
            CowVariant::Pink => MilkVariant::Strawberry,
            CowVariant::LightYellow => MilkVariant::Vanilla,
            CowVariant::LightGreen => MilkVariant::Alien,
            CowVariant::Blue => MilkVariant::Blueberry,
            CowVariant::Amber => MilkVariant::Honey,
            CowVariant::Olive => MilkVariant::Matcha,
        }
    }

    pub fn patches_color(self) -> Option<Color> {
        match self {
            CowVariant::WhiteBlack => Some(DARK_GRAY),
            _ => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            CowVariant::Brown => "brown",
            CowVariant::WhiteBlack => "white",
            CowVariant::Pink => "pink",
            CowVariant::LightYellow => "yellow",
            CowVariant::LightGreen => "alien",
            CowVariant::Blue => "blue",
            CowVariant::Amber => "honey",
            CowVariant::Olive => "matcha",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "brown" => Some(CowVariant::Brown),
            "white" | "whiteblack" | "white-black" => Some(CowVariant::WhiteBlack),
            "pink" => Some(CowVariant::Pink),
            "yellow" | "lightyellow" => Some(CowVariant::LightYellow),
            "alien" | "lightgreen" | "green" => Some(CowVariant::LightGreen),
            "blue" | "blueberry" | "navy" => Some(CowVariant::Blue),
            "honey" | "amber" => Some(CowVariant::Amber),
            "matcha" | "olive" => Some(CowVariant::Olive),
            _ => None,
        }
    }

    pub fn random(rng: &mut impl RngExt) -> Self {
        Self::ALL[rng.random_range(0..Self::ALL.len())]
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(into = "CowRecord", from = "CowRecord")]
pub struct Cow {
    pub name: String,
    pub position: Position,
    pub variant: CowVariant,
    pub color: Color,
    pub body_length: usize,
    pub sway: SwayState,
    pub sway_speed: f32,
    pub mutant: Box<MutantState>,
    pub mutations: Option<Box<MutationRecord>>,
    pub speech: Option<SpeechBubble>,
    pub display_width: usize,
    pub engulf_timer: f32,
    pub sky: Sky,
}

impl Cow {
    pub fn new(name: String, variant: CowVariant, x: f32, y: f32, rng: &mut impl RngExt) -> Self {
        let mut mutant = MutantState::new_for_standard(MutantTail::Wide, rng);
        mutant.left_eyes.clear();
        mutant.right_eyes.clear();
        for _ in 0..2 {
            mutant.left_eyes.push(EyeState::new(rng));
        }
        mutant.eye_color = Some(DARK_GRAY);
        let mut cow = Self {
            name,
            position: Position { x, y },
            variant,
            color: variant.body_color(),
            body_length: 0,
            sway: SwayState {
                phase: rng.random::<f32>() * TAU,
            },
            sway_speed: COW_DEFAULT_SWAY_SPEED,
            mutant: Box::new(mutant),
            mutations: None,
            speech: None,
            display_width: 0,
            engulf_timer: 0.0,
            sky: Sky::default(),
        };
        cow.display_width = cow_display_width(&cow);
        cow
    }

    pub fn tick(&mut self, dt: f32, sky: Sky) {
        self.sky = sky;
        if self.engulf_timer > 0.0 {
            self.engulf_timer = (self.engulf_timer - dt).max(0.0);
        }
        tick_sway(&mut self.sway, self.sway_speed);
        self.mutant.tick_eyes(dt, sky.daylight);
        SpeechBubble::fade(&mut self.speech, dt);
    }

    pub fn is_asleep(&self) -> bool {
        self.mutant.circadian.asleep(self.sky.daylight)
    }

    pub fn lead(&self) -> usize {
        if self.mutant.is_double && self.mutant.backwards {
            return 0;
        }
        self.mutant.adornments.lead()
    }

    pub fn say(&mut self, text: String) {
        if text.trim().is_empty() {
            return;
        }
        self.speech = Some(SpeechBubble::new(text));
    }

    pub fn can_speak(&self) -> bool {
        !self.mutant.backwards
    }

    pub fn torso_width(&self) -> usize {
        COW_BASE_TORSO + self.body_length
    }

    pub fn eye_count(&self) -> usize {
        self.mutant.left_eyes.len()
    }

    pub fn milk_yield(&self) -> u32 {
        (self.milk_components().len() as u32).max(1)
    }

    pub fn milk_components(&self) -> Vec<CowVariant> {
        if self.mutant.fused.is_empty() {
            return vec![self.variant];
        }
        self.mutant
            .fused
            .iter()
            .flat_map(|c| c.milk_variants())
            .collect()
    }

    pub fn is_alienated(&self) -> bool {
        self.variant == CowVariant::LightGreen
            || self
                .mutations
                .as_ref()
                .is_some_and(|m| m.has(Mutation::Alienation))
    }

    pub fn sprite_top_offset(&self) -> u16 {
        if self.is_alienated() {
            COW_ANTENNA_ROWS
        } else {
            0
        }
    }
}

impl Cow {
    pub const fn sprite_height() -> u16 {
        COW_SPRITE_ROWS
    }
}

const COW_CAPS: &[Mutation] = &[
    Mutation::SizeIncrease,
    Mutation::SizeDecrease,
    Mutation::EyeIncrease,
    Mutation::EyeDecrease,
    Mutation::ColorPatch,
    Mutation::EyeColor,
    Mutation::GlistenFast,
    Mutation::GlistenSlow,
    Mutation::GlistenMode,
    Mutation::GlistenColor,
    Mutation::GlistenEnable,
    Mutation::GlistenDisable,
    Mutation::BodyColor,
    Mutation::Telophase,
    Mutation::BackwardsTelophase,
    Mutation::Cytokinesis,
    Mutation::Endocytosis,
    Mutation::Engulfment,
    Mutation::Alienation,
    Mutation::Strawberry,
    Mutation::NightOwl,
    Mutation::HelpedByGod,
    Mutation::Heterochromia,
    Mutation::Hydra,
    Mutation::Spikes,
    Mutation::Wings,
    Mutation::DecreaseExtension,
    Mutation::Lure,
];

const COW_HYDRA_HEAD_W: usize = 4;
const COW_HYDRA_HEAD_GAP: usize = 1;

pub fn cow_hydra_capacity(cow: &Cow) -> usize {
    if cow.mutant.is_double {
        return 0;
    }
    let torso = cow.torso_width();
    (torso + COW_HYDRA_HEAD_GAP) / (COW_HYDRA_HEAD_W + COW_HYDRA_HEAD_GAP)
}

fn cow_hydra_offsets(torso: usize, count: usize) -> Vec<usize> {
    let used = count * COW_HYDRA_HEAD_W + count.saturating_sub(1) * COW_HYDRA_HEAD_GAP;
    let margin = torso.saturating_sub(used) / 2;
    (0..count)
        .map(|i| margin + i * (COW_HYDRA_HEAD_W + COW_HYDRA_HEAD_GAP))
        .collect()
}

fn stamp_cow_row(row: &mut Vec<(char, Color)>, col: usize, cells: &[(char, Color)], body: Color) {
    while row.len() < col {
        row.push((COW_TRANSPARENT, body));
    }
    for (k, &cell) in cells.iter().enumerate() {
        let idx = col + k;
        if idx < row.len() {
            row[idx] = cell;
        } else {
            row.push(cell);
        }
    }
}

fn overlay_hydra_heads(rows: &mut [Vec<(char, Color)>], cow: &Cow) {
    let count = cow.mutant.hydra_eyes.len().min(cow_hydra_capacity(cow));
    if count == 0 {
        return;
    }
    let body = cow.color;
    let eye_default = cow.mutant.eye_color.unwrap_or(DARK_GRAY);
    let torso = cow.torso_width();
    let torso_start = cow_head_render_count(cow) + 2 + 1;
    let top = cow.sprite_top_offset() as usize;
    for (i, off) in cow_hydra_offsets(torso, count).into_iter().enumerate() {
        let col = torso_start + off;
        let eye = &cow.mutant.hydra_eyes[i];
        let ec = cow.mutant.eye_render_color(eye, eye_default);
        let eye_ch = if eye.is_open() { 'o' } else { '-' };
        let top_row = [('^', body), ('_', body), ('_', body), ('^', body)];
        let mid_row = [('(', body), (eye_ch, ec), (eye_ch, ec), (')', body)];
        let bot_row = [('(', body), ('_', body), ('_', body), (')', body)];
        stamp_cow_row(&mut rows[top], col, &top_row, body);
        stamp_cow_row(&mut rows[top + 1], col, &mid_row, body);
        stamp_cow_row(&mut rows[top + 2], col, &bot_row, body);
    }
}

impl Mutatable for Cow {
    fn capabilities(&self) -> &'static [Mutation] {
        COW_CAPS
    }
    fn is_double(&self) -> bool {
        self.mutant.is_double
    }
    fn has_glisten(&self) -> bool {
        self.mutant.glistening_color.is_some()
    }
    fn apply_one(&mut self, mutation: Mutation, rng: &mut impl RngExt) -> MutationOutcome {
        apply_mutant_mutation(self, mutation, rng)
    }
    fn record_mut(&mut self) -> &mut MutationRecord {
        self.mutations
            .get_or_insert_with(|| Box::new(MutationRecord::default()))
    }
    fn circadian(&self) -> Circadian {
        self.mutant.circadian
    }
    fn backwards(&self) -> bool {
        self.mutant.backwards
    }
    fn hydra_count(&self) -> usize {
        self.mutant.hydra_eyes.len()
    }
    fn hydra_max(&self) -> usize {
        cow_hydra_capacity(self)
    }
    fn extension(&self) -> BodyExtension {
        self.mutant.body_extension.unwrap_or_default()
    }
    fn reserves(&self, band: Band) -> bool {
        band == Band::Bottom
    }
}

impl MutantBacked for Cow {
    fn body_size(&self) -> usize {
        self.body_length + COW_BASE_TORSO
    }
    fn set_body_size(&mut self, n: usize) {
        let clamped = n.clamp(COW_MIN_TORSO, COW_MAX_TORSO);
        self.body_length = clamped.saturating_sub(COW_BASE_TORSO);
    }
    fn color(&self) -> Color {
        self.color
    }
    fn set_color(&mut self, c: Color) {
        self.color = c;
    }
    fn sway_speed(&self) -> f32 {
        self.sway_speed
    }
    fn set_sway_speed(&mut self, s: f32) {
        self.sway_speed = s;
    }
    fn default_sway_speed(&self) -> f32 {
        COW_DEFAULT_SWAY_SPEED
    }
    fn mutant(&self) -> &MutantState {
        &self.mutant
    }
    fn mutant_mut(&mut self) -> &mut MutantState {
        &mut self.mutant
    }
    fn doublefish_eye_count(&self, _rng: &mut impl RngExt) -> usize {
        self.eye_count().clamp(2, COW_HEAD_CAP)
    }
    fn recompute_display_width(&mut self) {
        self.display_width = cow_display_width(self);
    }
    fn self_component(&self) -> FusedComponent {
        FusedComponent::cow(self.variant, self.name.clone())
    }
    fn arm_engulf(&mut self) {
        self.engulf_timer = crate::fishes::fish::ENGULF_WINDOW_SECS;
    }
    fn color_patch_range(&self) -> usize {
        cow_paintable_cell_count(self)
    }
    fn hydra_capacity(&self) -> usize {
        cow_hydra_capacity(self)
    }
}

pub fn cow_paintable_cell_count(cow: &Cow) -> usize {
    cow_sprite(cow)
        .iter()
        .flatten()
        .filter(|(c, _)| *c != ' ' && *c != COW_TRANSPARENT)
        .count()
}

pub fn cow_display_width(cow: &Cow) -> usize {
    let torso = cow.torso_width();
    let head_eye_render = cow_head_render_count(cow);
    let head_w = (head_eye_render.max(2)) + 2;
    let body = if cow.mutant.is_double && cow.mutant.backwards {
        2 * COW_TAIL_W + torso
    } else if cow.mutant.is_double {
        let right_head_w = (cow.mutant.double_head_eyes.len().max(1)) + 2;
        head_w + torso + 1 + right_head_w
    } else {
        let tail = 4;
        let row3_w = head_w + torso + tail;
        let row2_w = head_w + 1 + torso;
        row3_w.max(row2_w)
    };
    body + cow.lead()
}

const COW_LURE: char = 'º';
const COW_LURE_STALK: char = ',';
const COW_LURE_GLOW_DAY: Color = AMBER_DARK;
const COW_LURE_GLOW_NIGHT: Color = GOLD_BRIGHT;

pub fn cow_sprite(cow: &Cow) -> Vec<Vec<(char, Color)>> {
    let mut rows = cow_body_sprite(cow);
    let lead = cow.lead();
    if lead == 0 {
        return rows;
    }
    for row in &mut rows {
        row.splice(
            0..0,
            std::iter::repeat_n((COW_TRANSPARENT, cow.color), lead),
        );
    }
    let head_row = cow.sprite_top_offset() as usize + 1;
    let glow = if cow.sky.daylight {
        COW_LURE_GLOW_DAY
    } else {
        COW_LURE_GLOW_NIGHT
    };
    if let Some(row) = rows.get_mut(head_row) {
        row[0] = (COW_LURE, glow);
    }
    if let Some(row) = rows.get_mut(head_row - 1) {
        row[1] = (COW_LURE_STALK, cow.color);
    }
    rows
}

fn cow_head_render_count(cow: &Cow) -> usize {
    let eyes = cow.eye_count();
    eyes.clamp(2, COW_HEAD_CAP)
}

fn cow_body_sprite(cow: &Cow) -> Vec<Vec<(char, Color)>> {
    let body = cow.color;
    let eye_color = cow.mutant.eye_color.unwrap_or(DARK_GRAY);
    let patch_color = cow.variant.patches_color();
    let phase = cow.sway.phase;
    let glisten_color = cow.mutant.glistening_color;
    let glisten_mode = cow.mutant.glistening_mode;

    let total_eyes = cow.eye_count();
    let head_render = cow_head_render_count(cow);
    let head_eye_render = head_render.min(total_eyes);
    let head_w = head_render + 2;
    let torso = cow.torso_width();
    let body_overflow = total_eyes.saturating_sub(head_eye_render);
    let alienated = cow.is_alienated();

    if cow.mutant.is_double {
        if cow.mutant.backwards {
            let mut rows = backward_cow_sprite(cow, torso, alienated);
            apply_cow_skin_decor(&mut rows, cow);
            return rows;
        }
        if let Some((left, right)) = cow_fused_render_halves(cow) {
            return per_half_double_cow(cow, left, right, head_render, torso, alienated);
        }
        let right_eyes = cow.mutant.double_head_eyes.len().max(1);
        return skinned_double_cow(
            cow,
            &cow.mutant.left_eyes,
            &cow.mutant.double_head_eyes,
            head_render,
            torso,
            right_eyes,
            alienated,
        );
    }

    let row1 = build_row1(head_w, body, alienated);

    let mut row2: Vec<(char, Color)> = Vec::new();
    row2.push(('(', body));
    for i in 0..head_render {
        let eye = cow.mutant.left_eyes.get(i);
        let open = eye.is_none_or(|e| e.is_open());
        let ch = if i < head_eye_render && open {
            'o'
        } else if i < head_eye_render {
            '-'
        } else {
            ' '
        };
        let cell_color = eye.map_or(eye_color, |e| cow.mutant.eye_render_color(e, eye_color));
        row2.push((ch, cell_color));
    }
    row2.push((')', body));
    row2.push(('\\', body));
    for _ in 0..torso {
        row2.push(('_', body));
    }

    let mut row3: Vec<(char, Color)> = Vec::new();
    for _ in 0..(head_w.saturating_sub(4)) {
        row3.push((COW_TRANSPARENT, body));
    }
    row3.push(('(', body));
    row3.push(('_', body));
    row3.push(('_', body));
    row3.push((')', body));
    row3.push(('\\', body));
    for i in 0..torso {
        let eye_index = head_eye_render + i;
        let is_eye = i < body_overflow;
        let eye = cow.mutant.left_eyes.get(eye_index);
        let open = eye.is_none_or(|e| e.is_open());
        let ch = if is_eye && open {
            'o'
        } else if is_eye {
            '-'
        } else {
            ' '
        };
        let color = if is_eye {
            eye.map_or(eye_color, |e| cow.mutant.eye_render_color(e, eye_color))
        } else {
            body
        };
        row3.push((ch, color));
    }
    row3.push((')', body));
    row3.push(('\\', body));
    row3.push(('/', body));
    row3.push(('\\', body));

    let leg_indent = head_w.saturating_sub(1);
    let mut row4: Vec<(char, Color)> = Vec::new();
    for _ in 0..leg_indent {
        row4.push((COW_TRANSPARENT, body));
    }
    row4.push(('|', body));
    row4.push(('|', body));
    for _ in 0..(torso.saturating_sub(3)) {
        row4.push((COW_BELLY, body));
    }
    row4.push((COW_BELLY, body));
    row4.push((COW_UDDER, body));
    row4.push((' ', body));
    row4.push(('|', body));

    let mut row5: Vec<(char, Color)> = Vec::new();
    for _ in 0..leg_indent {
        row5.push((COW_TRANSPARENT, body));
    }
    row5.push(('|', body));
    row5.push(('|', body));
    for _ in 1..torso {
        row5.push((COW_TRANSPARENT, body));
    }
    row5.push(('|', body));
    row5.push(('|', body));

    let mut rows = vec![row1, row2, row3, row4, row5];
    if alienated {
        rows.insert(0, build_antenna_row(head_w, body));
    }

    if let Some(pc) = patch_color {
        apply_random_patches(
            &mut rows,
            pc,
            cow.position.x as u64 ^ (cow.name.len() as u64) << 8,
        );
    }

    if let Some(peak) = glisten_color {
        crate::sprite::apply_glisten(&mut rows, phase, glisten_mode, cow.color, peak);
    }

    for &(pos, color) in &cow.mutant.color_patches {
        apply_color_patch(&mut rows, pos, color);
    }

    overlay_hydra_heads(&mut rows, cow);

    rows
}

fn build_row1(head_w: usize, body: Color, alienated: bool) -> Vec<(char, Color)> {
    let mut row = Vec::new();
    let pad = head_w.saturating_sub(COW_HEAD_TOP_W) / 2;
    for _ in 0..pad {
        row.push((COW_TRANSPARENT, body));
    }
    if alienated {
        row.push((ANTENNA_LEFT_STEM, body));
        row.push((ANTENNA_STEM_FILL, body));
        row.push((ANTENNA_STEM_FILL, body));
        row.push((ANTENNA_RIGHT_STEM, body));
    } else {
        row.push(('^', body));
        row.push(('_', body));
        row.push(('_', body));
        row.push(('^', body));
    }
    row
}

fn build_antenna_row(head_w: usize, body: Color) -> Vec<(char, Color)> {
    let mut row = Vec::new();
    let pad = head_w.saturating_sub(COW_HEAD_TOP_W) / 2;
    for _ in 0..pad {
        row.push((COW_TRANSPARENT, body));
    }
    row.push((ANTENNA_BALL, body));
    row.push((COW_TRANSPARENT, body));
    row.push((COW_TRANSPARENT, body));
    row.push((ANTENNA_BALL, body));
    row
}

fn apply_cow_skin_decor(rows: &mut [Vec<(char, Color)>], skin: &Cow) {
    if let Some(pc) = skin.variant.patches_color() {
        apply_random_patches(
            rows,
            pc,
            skin.position.x as u64 ^ (skin.name.len() as u64) << 8,
        );
    }
    if let Some(peak) = skin.mutant.glistening_color {
        crate::sprite::apply_glisten(
            rows,
            skin.sway.phase,
            skin.mutant.glistening_mode,
            skin.color,
            peak,
        );
    }
    for &(pos, color) in &skin.mutant.color_patches {
        apply_color_patch(rows, pos, color);
    }
}

fn cow_fused_render_halves(cow: &Cow) -> Option<(&Cow, &Cow)> {
    if !cow.mutant.is_double || cow.mutant.fused.len() != 2 {
        return None;
    }
    let left = cow.mutant.fused[0].cow_snapshot()?;
    let right = cow.mutant.fused[1].cow_snapshot()?;
    Some((left, right))
}

fn skinned_double_cow(
    skin: &Cow,
    left_head_eyes: &[EyeState],
    right_head_eyes: &[EyeState],
    head_render: usize,
    torso: usize,
    right_eyes: usize,
    alienated: bool,
) -> Vec<Vec<(char, Color)>> {
    let mut rows = double_cow_sprite(
        skin,
        left_head_eyes,
        right_head_eyes,
        head_render,
        torso,
        right_eyes,
        alienated,
    );
    apply_cow_skin_decor(&mut rows, skin);
    rows
}

fn per_half_double_cow(
    host: &Cow,
    left: &Cow,
    right: &Cow,
    head_render: usize,
    torso: usize,
    alienated: bool,
) -> Vec<Vec<(char, Color)>> {
    let right_eyes = host.mutant.double_head_eyes.len().max(1);
    let left_rows = skinned_double_cow(
        left,
        &left.mutant.left_eyes,
        &[],
        head_render,
        torso,
        right_eyes,
        alienated,
    );
    let right_rows = skinned_double_cow(
        right,
        &[],
        &right.mutant.left_eyes,
        head_render,
        torso,
        right_eyes,
        alienated,
    );
    let seam = head_render + 3 + torso / 2;
    splice_cow_halves(left_rows, right_rows, seam)
}

fn splice_cow_halves(
    left: Vec<Vec<(char, Color)>>,
    right: Vec<Vec<(char, Color)>>,
    seam: usize,
) -> Vec<Vec<(char, Color)>> {
    left.into_iter()
        .zip(right)
        .map(|(l, r)| {
            let cut = seam.min(l.len());
            let mut row = l[..cut].to_vec();
            if seam < r.len() {
                row.extend_from_slice(&r[seam..]);
            }
            row
        })
        .collect()
}

fn double_cow_sprite(
    skin: &Cow,
    left_head_eyes: &[EyeState],
    right_head_eyes: &[EyeState],
    head_render: usize,
    torso: usize,
    right_eyes: usize,
    alienated: bool,
) -> Vec<Vec<(char, Color)>> {
    let body = skin.color;
    let eye_color = skin.mutant.eye_color.unwrap_or(DARK_GRAY);
    let head_w = head_render + 2;
    let right_head_w = right_eyes + 2;

    let left_pad = head_w.saturating_sub(COW_HEAD_TOP_W) / 2;
    let right_head_col = head_w + torso + 2;
    let right_pad = right_head_w.saturating_sub(COW_HEAD_TOP_W) / 2;
    let right_top_col = right_head_col + right_pad;

    let (left_a, left_b, left_c, left_d) = if alienated {
        (
            ANTENNA_LEFT_STEM,
            ANTENNA_STEM_FILL,
            ANTENNA_STEM_FILL,
            ANTENNA_RIGHT_STEM,
        )
    } else {
        ('^', '_', '_', '^')
    };

    let mut row1 = Vec::new();
    for _ in 0..left_pad {
        row1.push((COW_TRANSPARENT, body));
    }
    row1.push((left_a, body));
    row1.push((left_b, body));
    row1.push((left_c, body));
    row1.push((left_d, body));
    while row1.len() < right_top_col {
        row1.push((COW_TRANSPARENT, body));
    }
    row1.push((left_a, body));
    row1.push((left_b, body));
    row1.push((left_c, body));
    row1.push((left_d, body));

    let mut row2 = Vec::new();
    row2.push(('(', body));
    for i in 0..head_render {
        let eye = left_head_eyes.get(i);
        let open = eye.is_none_or(|e| e.is_open());
        let cell_color = eye.map_or(eye_color, |e| skin.mutant.eye_render_color(e, eye_color));
        row2.push((if open { 'o' } else { '-' }, cell_color));
    }
    row2.push((')', body));
    row2.push(('\\', body));
    for _ in 0..torso {
        row2.push(('_', body));
    }
    row2.push(('/', body));
    row2.push(('(', body));
    for i in 0..right_eyes {
        let eye = right_head_eyes.get(i);
        let open = eye.is_none_or(|e| e.is_open());
        let cell_color = eye.map_or(eye_color, |e| skin.mutant.eye_render_color(e, eye_color));
        row2.push((if open { 'o' } else { '-' }, cell_color));
    }
    row2.push((')', body));

    let mut row3 = vec![
        ('(', body),
        ('_', body),
        ('_', body),
        (')', body),
        ('\\', body),
    ];
    for _ in 0..torso {
        row3.push((' ', body));
    }
    row3.push(('/', body));
    row3.push(('(', body));
    row3.push(('_', body));
    row3.push(('_', body));
    row3.push((')', body));

    let leg_indent = head_w.saturating_sub(1);
    let mut row4 = Vec::new();
    for _ in 0..leg_indent {
        row4.push((COW_TRANSPARENT, body));
    }
    row4.push(('|', body));
    row4.push(('|', body));
    row4.push((COW_BELLY, body));
    for _ in 0..(torso.saturating_sub(3)) {
        row4.push((COW_BELLY, body));
    }
    row4.push((COW_BELLY, body));
    row4.push((COW_BELLY, body));
    row4.push(('|', body));
    row4.push(('|', body));

    let mut row5 = Vec::new();
    for _ in 0..leg_indent {
        row5.push((COW_TRANSPARENT, body));
    }
    row5.push(('|', body));
    row5.push(('|', body));
    for _ in 0..torso {
        row5.push((COW_TRANSPARENT, body));
    }
    row5.push(('|', body));
    row5.push(('|', body));

    let mut rows = vec![row1, row2, row3, row4, row5];
    if alienated {
        let mut antenna = Vec::new();
        for _ in 0..left_pad {
            antenna.push((COW_TRANSPARENT, body));
        }
        antenna.push((ANTENNA_BALL, body));
        antenna.push((COW_TRANSPARENT, body));
        antenna.push((COW_TRANSPARENT, body));
        antenna.push((ANTENNA_BALL, body));
        while antenna.len() < right_top_col {
            antenna.push((COW_TRANSPARENT, body));
        }
        antenna.push((ANTENNA_BALL, body));
        antenna.push((COW_TRANSPARENT, body));
        antenna.push((COW_TRANSPARENT, body));
        antenna.push((ANTENNA_BALL, body));
        rows.insert(0, antenna);
    }
    rows
}

fn backward_cow_sprite(cow: &Cow, torso: usize, alienated: bool) -> Vec<Vec<(char, Color)>> {
    let body = cow.color;
    let total_w = 2 * COW_TAIL_W + torso;
    let belly_indent = COW_TAIL_W - 1;
    let belly_dashes = torso.saturating_sub(4);
    let leg_gap = torso.saturating_sub(2);

    let row1: Vec<(char, Color)> = vec![(COW_TRANSPARENT, body); total_w];

    let mut row2 = Vec::new();
    for _ in 0..COW_TAIL_W {
        row2.push((COW_TRANSPARENT, body));
    }
    for _ in 0..torso {
        row2.push(('_', body));
    }

    let mut row3 = Vec::new();
    for &c in COW_LEFT_TAIL.iter() {
        row3.push((c, body));
    }
    for _ in 0..torso {
        row3.push((' ', body));
    }
    for &c in COW_RIGHT_TAIL.iter() {
        row3.push((c, body));
    }

    let mut row4 = Vec::new();
    for _ in 0..belly_indent {
        row4.push((COW_TRANSPARENT, body));
    }
    row4.push(('|', body));
    row4.push((' ', body));
    row4.push((COW_UDDER, body));
    for _ in 0..belly_dashes {
        row4.push((COW_BELLY, body));
    }
    row4.push((COW_UDDER, body));
    row4.push((' ', body));
    row4.push(('|', body));

    let mut row5 = Vec::new();
    for _ in 0..belly_indent {
        row5.push((COW_TRANSPARENT, body));
    }
    row5.push(('|', body));
    row5.push(('|', body));
    for _ in 0..leg_gap {
        row5.push((COW_TRANSPARENT, body));
    }
    row5.push(('|', body));
    row5.push(('|', body));

    let mut rows = vec![row1, row2, row3, row4, row5];
    if alienated {
        let mut antenna = vec![(COW_TRANSPARENT, body); total_w];
        antenna[0] = (ANTENNA_BALL, body);
        antenna[total_w - 1] = (ANTENNA_BALL, body);
        rows.insert(0, antenna);
    }
    rows
}

fn apply_color_patch(rows: &mut [Vec<(char, Color)>], linear_pos: usize, color: Color) {
    let mut idx = 0usize;
    for row in rows.iter_mut() {
        for cell in row.iter_mut() {
            if cell.0 != ' ' && cell.0 != COW_TRANSPARENT {
                if idx == linear_pos {
                    cell.1 = color;
                    return;
                }
                idx += 1;
            }
        }
    }
}

fn apply_random_patches(rows: &mut [Vec<(char, Color)>], patch: Color, seed: u64) {
    use rand::SeedableRng;
    use rand::rngs::SmallRng;
    let mut rng = SmallRng::seed_from_u64(seed);
    let n_underscores: usize = rows.iter().flatten().filter(|(c, _)| *c == '_').count();
    let n_patch = (n_underscores / 3).max(2);
    for _ in 0..n_patch {
        let mut target = rng.random_range(0..n_underscores.max(1));
        for row in rows.iter_mut() {
            for cell in row.iter_mut() {
                if cell.0 == '_' {
                    if target == 0 {
                        cell.1 = patch;
                        break;
                    }
                    target -= 1;
                }
            }
        }
    }
}

pub fn random_cow_color(rng: &mut impl RngExt) -> CowVariant {
    CowVariant::random(rng)
}

#[cfg(test)]
mod tests {
    use super::*;

    const UDDER_ROW: usize = 3;

    fn test_cow(double: bool) -> Cow {
        let mut rng = rand::rng();
        let mut cow = Cow::new("Bessie".to_string(), CowVariant::Brown, 0.0, 0.0, &mut rng);
        cow.mutant.is_double = double;
        if double {
            let component = cow.self_component();
            cow.mutant.fused = vec![component.clone(), component];
        }
        cow
    }

    fn row_chars(rows: &[Vec<(char, Color)>], idx: usize) -> Vec<char> {
        rows[idx].iter().map(|(c, _)| *c).collect()
    }

    #[test]
    fn telophase_cow_has_dash_udders_not_w() {
        let row = row_chars(&cow_sprite(&test_cow(true)), UDDER_ROW);
        assert!(
            !row.contains(&COW_UDDER),
            "plain telophase cow must show '-' udders, got {row:?}"
        );
    }

    #[test]
    fn single_cow_keeps_one_w_udder() {
        let row = row_chars(&cow_sprite(&test_cow(false)), UDDER_ROW);
        let udders = row.iter().filter(|&&c| c == COW_UDDER).count();
        assert_eq!(udders, 1, "single cow keeps its one udder, got {row:?}");
    }

    fn backward_cow() -> Cow {
        let mut cow = test_cow(true);
        cow.mutant.backwards = true;
        cow.display_width = cow_display_width(&cow);
        cow
    }

    #[test]
    fn backwardstelophase_cow_has_two_w_udders() {
        let rows = cow_sprite(&backward_cow());
        let udders = rows
            .iter()
            .flatten()
            .filter(|&&(c, _)| c == COW_UDDER)
            .count();
        assert_eq!(udders, 2, "a backwardstelophase cow shows two w udders");
    }

    #[test]
    fn backwardstelophase_cow_shows_tails_not_faces() {
        let rows = cow_sprite(&backward_cow());
        let chars: Vec<char> = rows.iter().flatten().map(|&(c, _)| c).collect();
        assert!(
            chars.contains(&'/') && chars.contains(&'\\'),
            "a backward cow shows tail glyphs"
        );
        assert!(
            !chars.contains(&'o'),
            "a backward cow has no (oo) face eyes"
        );
        assert!(
            !chars.contains(&'^'),
            "a backward cow has no head-top horns"
        );
    }

    #[test]
    fn backward_cow_display_width_matches_widest_row() {
        let cow = backward_cow();
        let widest = cow_sprite(&cow).iter().map(|r| r.len()).max().unwrap();
        assert_eq!(
            cow.display_width, widest,
            "width tracks the rendered sprite"
        );
    }

    const TELOPHASE_COW_MILK_YIELD: u32 = 2;

    #[test]
    fn telophase_cow_yields_double_milk() {
        assert_eq!(test_cow(false).milk_yield(), 1, "single cow yields one");
        assert_eq!(
            test_cow(true).milk_yield(),
            TELOPHASE_COW_MILK_YIELD,
            "a forward telophase cow yields per-component milk"
        );
        assert_eq!(
            backward_cow().milk_yield(),
            TELOPHASE_COW_MILK_YIELD,
            "a backwardstelophase cow yields per-component milk"
        );
    }
}
