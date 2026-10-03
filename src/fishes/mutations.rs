use rand::RngExt;
use ratatui::style::Color;

use super::fish::{ENGULF_WINDOW_SECS, Fish, compute_display_width};
use super::fused::FusedComponent;
use super::mutant::{
    Adornments, Circadian, EyeState, MIN_BODY_CHARS, MutantState, MutantTail, MutationRecord,
    random_rgb,
};
use super::species::{BodyTemplate, FishSpecies, SINGLE_EYE, TailKind};
use super::unfish::{
    BALL_HEIGHT, SKULL_HEIGHT, SLIME_GLISTEN_SPEED_FAST, SLIME_GLISTEN_SPEED_SLOW, UnfishKind,
    UnfishMutationStyle, is_multi_row, worm_display_width,
};
use crate::colors::{DARK_GRAY, LIGHT_GREEN, PINK, WHITE};
use crate::sprite::{BodyExtension, ExtensionVariant, Feet, FeetStyle};

const MUTATION_MAX_BODY_SIZE: usize = 12;
pub const MUTATION_PATCH_MAX: usize = 12;
pub const MUTATION_PATCH_COUNT_MIN: usize = 2;
pub const MUTATION_PATCH_COUNT_MAX: usize = 4;
const GLISTEN_SPEED_FAST_MULT: f32 = 3.5;
const GLISTEN_SPEED_SLOW_MULT: f32 = 0.25;
const SWAY_SPEED_CLAMP_MAX: f32 = 2.5;
const SWAY_SPEED_GLISTEN_FLOOR: f32 = 0.08;
const SWAY_SPEED_CLAMP_MIN: f32 = 0.01;
pub const STRAWBERRY_SELL_BONUS_PCT: u32 = 25;
const WORM_MAX_SEGMENTS: usize = 12;
const WORM_MIN_SEGMENTS: usize = 1;
const WORM_MAX_EXTRA_EYES: usize = 4;
const DOUBLE_EYE_COUNT_MAX: usize = 3;
const MAX_EARS: usize = 4;
const HYDRA_EYES_PER_APPLICATION_MIN: usize = 1;
const HYDRA_EYES_PER_APPLICATION_MAX: usize = 2;
const EXTENSION_MAX_LENGTH: usize = 8;
const EXTENSION_VARIANT_COUNT: u8 = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mutation {
    SizeIncrease,
    SizeDecrease,
    EyeIncrease,
    EyeDecrease,
    ColorPatch,
    EyeColor,
    GlistenFast,
    GlistenSlow,
    GlistenMode,
    GlistenColor,
    GlistenEnable,
    GlistenDisable,
    BodyColor,
    BodyVariant,
    TailVariant,
    MouthVariant,
    Telophase,
    BackwardsTelophase,
    Cytokinesis,
    Endocytosis,
    Engulfment,
    Alienation,
    Strawberry,
    BubbleColor,
    NightOwl,
    HelpedByGod,
    Heterochromia,
    Ear,
    EarDecrease,
    EarColor,
    Hydra,
    Feet,
    NoFeet,
    FeetColor,
    BodyExtension,
    DecreaseExtension,
    Lure,
    Bill,
    DorsalFin,
    VentralFin,
    Lunar,
    Puff,
}

impl Mutation {
    pub const ALL: &'static [Mutation] = &[
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
        Mutation::BodyVariant,
        Mutation::TailVariant,
        Mutation::MouthVariant,
        Mutation::Telophase,
        Mutation::BackwardsTelophase,
        Mutation::Cytokinesis,
        Mutation::Endocytosis,
        Mutation::Engulfment,
        Mutation::Alienation,
        Mutation::Strawberry,
        Mutation::BubbleColor,
        Mutation::NightOwl,
        Mutation::HelpedByGod,
        Mutation::Heterochromia,
        Mutation::Ear,
        Mutation::EarDecrease,
        Mutation::EarColor,
        Mutation::Hydra,
        Mutation::Feet,
        Mutation::NoFeet,
        Mutation::FeetColor,
        Mutation::BodyExtension,
        Mutation::DecreaseExtension,
        Mutation::Lure,
        Mutation::Bill,
        Mutation::DorsalFin,
        Mutation::VentralFin,
        Mutation::Lunar,
        Mutation::Puff,
    ];

    pub const ADORNMENTS: &'static [Mutation] = &[
        Mutation::Lure,
        Mutation::Bill,
        Mutation::DorsalFin,
        Mutation::VentralFin,
        Mutation::Lunar,
        Mutation::Puff,
    ];

    pub fn token(self) -> &'static str {
        match self {
            Mutation::SizeIncrease => "sizeincrease",
            Mutation::SizeDecrease => "sizedecrease",
            Mutation::EyeIncrease => "eyeincrease",
            Mutation::EyeDecrease => "eyedecrease",
            Mutation::ColorPatch => "colorpatch",
            Mutation::EyeColor => "eyecolor",
            Mutation::GlistenFast => "glistenfast",
            Mutation::GlistenSlow => "glistenslow",
            Mutation::GlistenMode => "glistenmode",
            Mutation::GlistenColor => "glistencolor",
            Mutation::GlistenEnable => "glistenenable",
            Mutation::GlistenDisable => "glistendisable",
            Mutation::BodyColor => "bodycolor",
            Mutation::BodyVariant => "bodyvariant",
            Mutation::TailVariant => "tailvariant",
            Mutation::MouthVariant => "mouthvariant",
            Mutation::Telophase => "telophase",
            Mutation::BackwardsTelophase => "backwardstelophase",
            Mutation::Cytokinesis => "cytokinesis",
            Mutation::Endocytosis => "endocytosis",
            Mutation::Engulfment => "engulfment",
            Mutation::Alienation => "alienation",
            Mutation::Strawberry => "strawberry",
            Mutation::BubbleColor => "bubblecolor",
            Mutation::NightOwl => "nightowl",
            Mutation::HelpedByGod => "helpedbygod",
            Mutation::Heterochromia => "heterochromia",
            Mutation::Ear => "ear",
            Mutation::EarDecrease => "eardecrease",
            Mutation::EarColor => "earcolor",
            Mutation::Hydra => "hydra",
            Mutation::Feet => "feet",
            Mutation::NoFeet => "nofeet",
            Mutation::FeetColor => "feetcolor",
            Mutation::BodyExtension => "bodyextension",
            Mutation::DecreaseExtension => "decreaseextension",
            Mutation::Lure => "lure",
            Mutation::Bill => "bill",
            Mutation::DorsalFin => "dorsalfin",
            Mutation::VentralFin => "ventralfin",
            Mutation::Lunar => "lunar",
            Mutation::Puff => "puff",
        }
    }

    pub fn parse(s: &str) -> Option<Mutation> {
        let lower = s.to_ascii_lowercase();
        Mutation::ALL.iter().copied().find(|m| m.token() == lower)
    }

    pub fn auto_selectable(self) -> bool {
        !matches!(self, Mutation::Strawberry | Mutation::Alienation)
    }

    pub fn divides(self) -> bool {
        matches!(self, Mutation::Cytokinesis)
    }

    pub fn adorns(self) -> bool {
        Mutation::ADORNMENTS.contains(&self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MutationOutcome {
    Applied,
    SplitRequested,
}

pub fn roll_feet(rng: &mut impl RngExt) -> Feet {
    let style = if rng.random::<bool>() {
        FeetStyle::Quote
    } else {
        FeetStyle::Caret
    };
    Feet { style, color: None }
}

pub fn roll_extension(rng: &mut impl RngExt) -> BodyExtension {
    let variant = match rng.random_range(0..EXTENSION_VARIANT_COUNT) {
        0 => ExtensionVariant::Tentacle,
        1 => ExtensionVariant::Spike,
        _ => ExtensionVariant::Wing,
    };
    BodyExtension {
        variant,
        length: variant.start_length(),
        seed: rng.random::<u64>(),
    }
}

fn grow_extension(slot: &mut Option<BodyExtension>, rng: &mut impl RngExt) {
    match slot.as_mut() {
        Some(ext) => ext.length = (ext.length + 1).min(EXTENSION_MAX_LENGTH),
        None => *slot = Some(roll_extension(rng)),
    }
}

fn shrink_extension(slot: &mut Option<BodyExtension>) {
    if let Some(ext) = slot.as_mut() {
        ext.length = ext.length.saturating_sub(1);
        if ext.length == 0 {
            *slot = None;
        }
    }
}

pub fn tail_kind_to_mutant_tail(kind: TailKind) -> MutantTail {
    match kind {
        TailKind::WideCurly => MutantTail::Curly,
        TailKind::Swaying { .. } => MutantTail::Swaying,
        TailKind::Short | TailKind::Custom { .. } => MutantTail::Narrow,
        TailKind::None => MutantTail::Bare,
        TailKind::Wide => MutantTail::Wide,
    }
}

pub trait Mutatable {
    fn capabilities(&self) -> &'static [Mutation];
    fn is_double(&self) -> bool;
    fn has_glisten(&self) -> bool;
    fn apply_one(&mut self, mutation: Mutation, rng: &mut impl RngExt) -> MutationOutcome;
    fn record_mut(&mut self) -> &mut MutationRecord;

    fn circadian(&self) -> Circadian {
        Circadian::Neutral
    }

    fn ear_count(&self) -> usize {
        0
    }

    fn max_ears(&self) -> usize {
        MAX_EARS
    }

    fn hydra_count(&self) -> usize {
        0
    }

    fn hydra_max(&self) -> usize {
        0
    }

    fn has_feet(&self) -> bool {
        false
    }

    fn has_bodyextension(&self) -> bool {
        false
    }

    fn backwards(&self) -> bool {
        false
    }

    fn adornments(&self) -> Adornments {
        Adornments::default()
    }

    fn has_line_head(&self) -> bool {
        true
    }

    fn supports_now(&self, mutation: Mutation) -> bool {
        if !self.capabilities().contains(&mutation) {
            return false;
        }
        if mutation.adorns() {
            return self.has_line_head() && !self.adornments().has(mutation);
        }
        match mutation {
            Mutation::Telophase => !self.is_double() || self.backwards(),
            Mutation::BackwardsTelophase => !self.is_double() || !self.backwards(),
            Mutation::Cytokinesis => self.is_double(),
            Mutation::Endocytosis => self.is_double(),
            Mutation::Engulfment => !self.is_double(),
            Mutation::GlistenEnable => !self.has_glisten(),
            Mutation::GlistenDisable => self.has_glisten(),
            Mutation::NightOwl => self.circadian() != Circadian::NightOwl,
            Mutation::HelpedByGod => self.circadian() != Circadian::HelpedByGod,
            Mutation::Ear => self.ear_count() < self.max_ears(),
            Mutation::EarDecrease => self.ear_count() > 0,
            Mutation::EarColor => self.ear_count() > 0,
            Mutation::Hydra => self.hydra_count() < self.hydra_max(),
            Mutation::Feet => !self.has_feet() && !self.has_bodyextension(),
            Mutation::NoFeet => self.has_feet(),
            Mutation::FeetColor => self.has_feet(),
            Mutation::BodyExtension => !self.has_feet(),
            Mutation::DecreaseExtension => self.has_bodyextension(),
            _ => true,
        }
    }

    fn available_mutations(&self) -> Vec<Mutation> {
        self.capabilities()
            .iter()
            .copied()
            .filter(|&m| self.supports_now(m))
            .collect()
    }

    fn random_mutation(&self, rng: &mut impl RngExt) -> Option<Mutation> {
        self.random_mutation_with_room(rng, true)
    }

    fn random_mutation_with_room(
        &self,
        rng: &mut impl RngExt,
        room_to_divide: bool,
    ) -> Option<Mutation> {
        let pool: Vec<Mutation> = self
            .available_mutations()
            .into_iter()
            .filter(|m| m.auto_selectable() && (room_to_divide || !m.divides()))
            .collect();
        if pool.is_empty() {
            return None;
        }
        Some(pool[rng.random_range(0..pool.len())])
    }
}

pub trait MutantBacked {
    fn body_size(&self) -> usize;
    fn set_body_size(&mut self, n: usize);
    fn color(&self) -> Color;
    fn set_color(&mut self, c: Color);
    fn sway_speed(&self) -> f32;
    fn set_sway_speed(&mut self, s: f32);
    fn default_sway_speed(&self) -> f32;
    fn mutant(&self) -> &MutantState;
    fn mutant_mut(&mut self) -> &mut MutantState;
    fn doublefish_eye_count(&self, rng: &mut impl RngExt) -> usize;
    fn recompute_display_width(&mut self);
    fn self_component(&self) -> FusedComponent;
    fn arm_engulf(&mut self) {}
    fn add_sell_bonus(&mut self, _pct: u32) {}
    fn gain_segment_mass(&mut self, _grown_from: usize) {}
    fn color_patch_range(&self) -> usize {
        self.mutant().display_width(self.body_size())
    }
    fn hydra_capacity(&self) -> usize {
        self.body_size().saturating_sub(1)
    }
}

fn seed_double<T: MutantBacked>(target: &mut T, rng: &mut impl RngExt, backwards: bool) {
    let count = target.doublefish_eye_count(rng);
    let component = target.self_component();
    let mutant = target.mutant_mut();
    if mutant.fused.is_empty() {
        mutant.fused = vec![component.clone(), component];
    }
    mutant.double_head_eyes = (0..count).map(|_| EyeState::new(rng)).collect();
    mutant.is_double = true;
    mutant.backwards = backwards;
}

pub fn apply_mutant_mutation<T: MutantBacked>(
    target: &mut T,
    mutation: Mutation,
    rng: &mut impl RngExt,
) -> MutationOutcome {
    match mutation {
        Mutation::SizeIncrease | Mutation::SizeDecrease => {
            let delta = if matches!(mutation, Mutation::SizeIncrease) {
                1
            } else {
                -1
            };
            let max_eyes = {
                let mutant = target.mutant();
                mutant.left_eyes.len().max(mutant.right_eyes.len())
            };
            let min_size = max_eyes + MIN_BODY_CHARS;
            let old_size = target.body_size();
            let new_size = (old_size as i32 + delta)
                .clamp(min_size as i32, MUTATION_MAX_BODY_SIZE as i32)
                as usize;
            target.set_body_size(new_size);
            if new_size > old_size {
                target.gain_segment_mass(old_size);
            }
            let hydra_cap = target.hydra_capacity();
            target.mutant_mut().hydra_eyes.truncate(hydra_cap);
        }
        Mutation::ColorPatch => {
            let width = target.color_patch_range().max(1);
            let count = rng.random_range(MUTATION_PATCH_COUNT_MIN..=MUTATION_PATCH_COUNT_MAX);
            let mutant = target.mutant_mut();
            for _ in 0..count {
                let pos = rng.random_range(0..width);
                mutant.color_patches.push((pos, random_rgb(rng)));
            }
            if mutant.color_patches.len() > MUTATION_PATCH_MAX {
                let excess = mutant.color_patches.len() - MUTATION_PATCH_MAX;
                mutant.color_patches.drain(0..excess);
            }
        }
        Mutation::EyeIncrease | Mutation::EyeDecrease => {
            let delta = if matches!(mutation, Mutation::EyeIncrease) {
                1
            } else {
                -1
            };
            let body_size = target.body_size();
            let max_eyes = body_size.saturating_sub(MIN_BODY_CHARS).clamp(1, 4) as i32;
            let mutant = target.mutant_mut();
            let floor = |eyes: usize| (eyes as i32).min(1);
            let new_left = (mutant.left_eyes.len() as i32 + delta)
                .clamp(floor(mutant.left_eyes.len()), max_eyes) as usize;
            while mutant.left_eyes.len() < new_left {
                mutant.left_eyes.push(EyeState::new(rng));
            }
            mutant.left_eyes.truncate(new_left);
            let new_right = (mutant.right_eyes.len() as i32 + delta)
                .clamp(floor(mutant.right_eyes.len()), max_eyes)
                as usize;
            while mutant.right_eyes.len() < new_right {
                mutant.right_eyes.push(EyeState::new(rng));
            }
            mutant.right_eyes.truncate(new_right);
        }
        Mutation::EyeColor => {
            let mutant = target.mutant_mut();
            if mutant.heterochromia {
                mutant.randomize_one_eye_color(rng);
            } else {
                mutant.eye_color = Some(random_rgb(rng));
            }
        }
        Mutation::Heterochromia => {
            let mutant = target.mutant_mut();
            mutant.heterochromia = true;
            mutant.randomize_all_eye_colors(rng);
        }
        Mutation::GlistenFast | Mutation::GlistenSlow => {
            let factor = if matches!(mutation, Mutation::GlistenFast) {
                GLISTEN_SPEED_FAST_MULT
            } else {
                GLISTEN_SPEED_SLOW_MULT
            };
            let s =
                (target.sway_speed() * factor).clamp(SWAY_SPEED_CLAMP_MIN, SWAY_SPEED_CLAMP_MAX);
            target.set_sway_speed(s);
        }
        Mutation::GlistenMode => {
            let mutant = target.mutant_mut();
            mutant.glistening_mode = mutant.glistening_mode.random_other(rng);
        }
        Mutation::GlistenColor => {
            let mutant = target.mutant_mut();
            mutant.glistening_color = if mutant.glistening_color.is_none() || rng.random::<bool>() {
                Some(random_rgb(rng))
            } else {
                None
            };
        }
        Mutation::BodyColor => {
            target.set_color(random_rgb(rng));
            let mutant = target.mutant_mut();
            mutant.color_patches.clear();
            if mutant.heterochromia {
                mutant.randomize_all_eye_colors(rng);
            } else if mutant.eye_color.is_some() {
                mutant.eye_color = Some(random_rgb(rng));
            }
        }
        Mutation::BodyVariant => {
            let mutant = target.mutant_mut();
            mutant.body_variant = (mutant.body_variant + rng.random_range(1..4u8)) % 4;
        }
        Mutation::TailVariant => {
            let mutant = target.mutant_mut();
            mutant.tail_variant = match mutant.tail_variant {
                MutantTail::Wide => MutantTail::Swaying,
                MutantTail::Swaying => MutantTail::Curly,
                MutantTail::Curly | MutantTail::Narrow | MutantTail::Bare => MutantTail::Wide,
            };
        }
        Mutation::MouthVariant => {
            let mutant = target.mutant_mut();
            mutant.mouth_inverted = !mutant.mouth_inverted;
        }
        Mutation::Telophase => seed_double(target, rng, false),
        Mutation::BackwardsTelophase => seed_double(target, rng, true),
        Mutation::Cytokinesis => return MutationOutcome::SplitRequested,
        Mutation::Endocytosis => {
            let mutant = target.mutant_mut();
            mutant.is_double = false;
            mutant.backwards = false;
            mutant.double_head_eyes.clear();
        }
        Mutation::Engulfment => target.arm_engulf(),
        Mutation::GlistenEnable => {
            let s = target
                .default_sway_speed()
                .max(SWAY_SPEED_GLISTEN_FLOOR)
                .clamp(SWAY_SPEED_CLAMP_MIN, SWAY_SPEED_CLAMP_MAX);
            target.set_sway_speed(s);
            target.mutant_mut().glistening_color = Some(WHITE);
        }
        Mutation::GlistenDisable => {
            target.set_sway_speed(target.default_sway_speed());
            target.mutant_mut().glistening_color = None;
        }
        Mutation::Alienation => {
            target.set_color(LIGHT_GREEN);
            let mutant = target.mutant_mut();
            mutant.eye_color = Some(DARK_GRAY);
            mutant.color_patches.clear();
        }
        Mutation::Strawberry => {
            target.set_color(PINK);
            target.mutant_mut().color_patches.clear();
            target.add_sell_bonus(STRAWBERRY_SELL_BONUS_PCT);
        }
        Mutation::BubbleColor => {
            target.mutant_mut().bubble_color = Some(random_rgb(rng));
        }
        Mutation::NightOwl => {
            target.mutant_mut().circadian = Circadian::NightOwl;
        }
        Mutation::HelpedByGod => {
            target.mutant_mut().circadian = Circadian::HelpedByGod;
        }
        Mutation::Ear => {
            target.mutant_mut().ear_count += 1;
        }
        Mutation::EarDecrease => {
            let mutant = target.mutant_mut();
            mutant.ear_count = mutant.ear_count.saturating_sub(1);
        }
        Mutation::EarColor => {
            target.mutant_mut().ear_color = Some(random_rgb(rng));
        }
        Mutation::Hydra => {
            let cap = target.hydra_capacity();
            let remaining = cap.saturating_sub(target.mutant().hydra_eyes.len());
            let requested =
                rng.random_range(HYDRA_EYES_PER_APPLICATION_MIN..=HYDRA_EYES_PER_APPLICATION_MAX);
            let count = requested.min(remaining);
            let mutant = target.mutant_mut();
            for _ in 0..count {
                mutant.hydra_eyes.push(EyeState::new(rng));
            }
        }
        Mutation::Feet => {
            target.mutant_mut().feet = Some(roll_feet(rng));
        }
        Mutation::NoFeet => {
            target.mutant_mut().feet = None;
        }
        Mutation::FeetColor => {
            if let Some(feet) = target.mutant_mut().feet.as_mut() {
                feet.color = Some(random_rgb(rng));
            }
        }
        Mutation::BodyExtension => grow_extension(&mut target.mutant_mut().body_extension, rng),
        Mutation::DecreaseExtension => shrink_extension(&mut target.mutant_mut().body_extension),
        Mutation::Lure
        | Mutation::Bill
        | Mutation::DorsalFin
        | Mutation::VentralFin
        | Mutation::Lunar
        | Mutation::Puff => target.mutant_mut().adornments.grow(mutation),
    }
    target.recompute_display_width();
    MutationOutcome::Applied
}

const MUTANT_FULL_CAPS: &[Mutation] = Mutation::ALL;

const FIXED_MULTICHAR_CAPS: &[Mutation] = &[
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
    Mutation::Alienation,
    Mutation::Strawberry,
    Mutation::BubbleColor,
    Mutation::NightOwl,
    Mutation::HelpedByGod,
    Mutation::Heterochromia,
    Mutation::Feet,
    Mutation::NoFeet,
    Mutation::FeetColor,
    Mutation::BodyExtension,
    Mutation::DecreaseExtension,
    Mutation::Lure,
    Mutation::Bill,
    Mutation::DorsalFin,
    Mutation::VentralFin,
    Mutation::Lunar,
    Mutation::Puff,
];

const FIXED_JELLY_CAPS: &[Mutation] = &[
    Mutation::BodyColor,
    Mutation::Telophase,
    Mutation::BackwardsTelophase,
    Mutation::Cytokinesis,
    Mutation::Endocytosis,
    Mutation::GlistenFast,
    Mutation::GlistenSlow,
    Mutation::GlistenMode,
    Mutation::GlistenColor,
    Mutation::GlistenEnable,
    Mutation::GlistenDisable,
    Mutation::BubbleColor,
    Mutation::NightOwl,
    Mutation::HelpedByGod,
    Mutation::Feet,
    Mutation::NoFeet,
    Mutation::FeetColor,
    Mutation::BodyExtension,
    Mutation::DecreaseExtension,
    Mutation::Lure,
    Mutation::DorsalFin,
    Mutation::VentralFin,
    Mutation::Puff,
];

const SLIME_CAPS: &[Mutation] = &[
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
    Mutation::BubbleColor,
    Mutation::NightOwl,
    Mutation::HelpedByGod,
    Mutation::Heterochromia,
    Mutation::Ear,
    Mutation::EarDecrease,
    Mutation::EarColor,
    Mutation::Hydra,
    Mutation::Feet,
    Mutation::NoFeet,
    Mutation::FeetColor,
    Mutation::BodyExtension,
    Mutation::DecreaseExtension,
    Mutation::Lure,
    Mutation::Bill,
    Mutation::DorsalFin,
    Mutation::VentralFin,
    Mutation::Lunar,
];

const FIGURE_CAPS: &[Mutation] = &[
    Mutation::GlistenFast,
    Mutation::GlistenSlow,
    Mutation::GlistenMode,
    Mutation::GlistenColor,
    Mutation::GlistenEnable,
    Mutation::GlistenDisable,
    Mutation::BodyColor,
    Mutation::Alienation,
    Mutation::Strawberry,
    Mutation::BubbleColor,
    Mutation::NightOwl,
    Mutation::HelpedByGod,
];

const WORM_CAPS: &[Mutation] = &[
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
    Mutation::BubbleColor,
    Mutation::NightOwl,
    Mutation::HelpedByGod,
    Mutation::Heterochromia,
    Mutation::Ear,
    Mutation::EarDecrease,
    Mutation::EarColor,
    Mutation::Hydra,
    Mutation::Feet,
    Mutation::NoFeet,
    Mutation::FeetColor,
    Mutation::BodyExtension,
    Mutation::DecreaseExtension,
];

fn fixed_is_single_char(left: &[&'static str]) -> bool {
    left.first().map(|s| s.chars().count()).unwrap_or(1) <= 1
}

pub(crate) fn ensure_fish_mutant(fish: &mut Fish, rng: &mut impl RngExt) {
    if fish.mutant.is_some() {
        return;
    }
    let config = fish.species.config();
    let body = config.body;
    let tail = match body {
        BodyTemplate::Standard(bc) | BodyTemplate::Alternating(bc, _) => {
            tail_kind_to_mutant_tail(bc.tail)
        }
        BodyTemplate::Fixed { .. } | BodyTemplate::Figure(_) => MutantTail::Wide,
    };
    let mut mutant = MutantState::new_for_standard(tail, rng);
    mutant.eye_color = config.eye_color;
    match body {
        BodyTemplate::Fixed { left, .. } => {
            let n_chars = left.first().map(|s| s.chars().count()).unwrap_or(1);
            if n_chars > 1 {
                let n_base = n_chars.saturating_sub(2).max(1);
                fish.body_size = n_base;
                mutant.left_eyes.clear();
                mutant.right_eyes.clear();
            }
            fish.display_width = compute_display_width(fish.species, 0);
        }
        BodyTemplate::Figure(_) => {
            fish.display_width = compute_display_width(fish.species, 0);
        }
        _ => fish.display_width = mutant.display_width(fish.body_size),
    }
    fish.mutant = Some(Box::new(mutant));
}

pub(crate) fn grow_birthmarks(fish: &mut Fish, rng: &mut impl RngExt) {
    for &mutation in fish.species.config().born_with {
        fish.apply_one(mutation, rng);
    }
}

pub(crate) fn native_eyes(species: FishSpecies, rng: &mut impl RngExt) -> Option<Box<MutantState>> {
    let config = species.config();
    if config.eyes == SINGLE_EYE {
        return None;
    }
    let (BodyTemplate::Standard(chars) | BodyTemplate::Alternating(chars, _)) = config.body else {
        return None;
    };
    let mut mutant = MutantState::new_for_standard(tail_kind_to_mutant_tail(chars.tail), rng);
    mutant.left_eyes = (0..config.eyes).map(|_| EyeState::new(rng)).collect();
    mutant.right_eyes = (0..config.eyes).map(|_| EyeState::new(rng)).collect();
    mutant.eye_color = config.eye_color;
    Some(Box::new(mutant))
}

pub fn apply_unfish_mutation(
    fish: &mut Fish,
    mutation: Mutation,
    rng: &mut impl RngExt,
) -> MutationOutcome {
    if matches!(mutation, Mutation::Cytokinesis) {
        return MutationOutcome::SplitRequested;
    }
    if matches!(mutation, Mutation::Engulfment) {
        fish.engulf_timer = ENGULF_WINDOW_SECS;
        return MutationOutcome::Applied;
    }
    let style = fish
        .unfish_state
        .as_ref()
        .map(|us| us.kind.mutation_style())
        .unwrap();
    let worm_component = fish.fused_self_component();
    let sprite_width = fish.display_width;
    let body_size = fish.body_size;
    {
        let us = fish.unfish_state.as_mut().unwrap();
        match mutation {
            Mutation::SizeIncrease if style == UnfishMutationStyle::Worm => {
                us.worm_segments = (us.worm_segments + 1).min(WORM_MAX_SEGMENTS);
            }
            Mutation::SizeDecrease if style == UnfishMutationStyle::Worm => {
                us.worm_segments = us.worm_segments.saturating_sub(1).max(WORM_MIN_SEGMENTS);
                us.hydra_count = us.hydra_count.min(us.worm_segments.saturating_sub(1));
            }
            Mutation::EyeIncrease => match style {
                UnfishMutationStyle::Worm => {
                    us.worm_extra_eyes = (us.worm_extra_eyes + 1).min(WORM_MAX_EXTRA_EYES);
                }
                UnfishMutationStyle::Slime => us.add_floating_eye(rng),
            },
            Mutation::EyeDecrease => match style {
                UnfishMutationStyle::Worm => {
                    us.worm_extra_eyes = us.worm_extra_eyes.saturating_sub(1);
                }
                UnfishMutationStyle::Slime => us.remove_floating_eye(rng),
            },
            Mutation::Telophase if style == UnfishMutationStyle::Worm => {
                if us.fused.is_empty() {
                    us.fused = vec![worm_component.clone(), worm_component];
                }
                us.worm_is_double = true;
                us.worm_backwards = false;
            }
            Mutation::BackwardsTelophase if style == UnfishMutationStyle::Worm => {
                if us.fused.is_empty() {
                    us.fused = vec![worm_component.clone(), worm_component];
                }
                us.worm_is_double = true;
                us.worm_backwards = true;
            }
            Mutation::Endocytosis if style == UnfishMutationStyle::Worm => {
                us.worm_is_double = false;
                us.worm_backwards = false;
            }
            Mutation::BodyColor => us.slime_body_color = Some(random_rgb(rng)),
            Mutation::EyeColor => us.recolor_one_eye(rng),
            Mutation::Heterochromia => us.make_heterochromatic(rng),
            Mutation::GlistenEnable => us.slime_glisten_enabled = true,
            Mutation::GlistenDisable => us.slime_glisten_enabled = false,
            Mutation::GlistenFast => us.slime_glisten_speed = SLIME_GLISTEN_SPEED_FAST,
            Mutation::GlistenSlow => us.slime_glisten_speed = SLIME_GLISTEN_SPEED_SLOW,
            Mutation::GlistenMode => {
                us.slime_glisten_mode = us.slime_glisten_mode.random_other(rng)
            }
            Mutation::GlistenColor => us.slime_glisten_color = Some(random_rgb(rng)),
            Mutation::BubbleColor => us.bubble_color = Some(random_rgb(rng)),
            Mutation::NightOwl => us.circadian = Circadian::NightOwl,
            Mutation::HelpedByGod => us.circadian = Circadian::HelpedByGod,
            Mutation::Ear => us.ear_count += 1,
            Mutation::EarDecrease => us.ear_count = us.ear_count.saturating_sub(1),
            Mutation::EarColor => us.ear_color = Some(random_rgb(rng)),
            Mutation::Feet => us.feet = Some(roll_feet(rng)),
            Mutation::NoFeet => us.feet = None,
            Mutation::FeetColor => {
                if let Some(feet) = us.feet.as_mut() {
                    feet.color = Some(random_rgb(rng));
                }
            }
            Mutation::BodyExtension => grow_extension(&mut us.body_extension, rng),
            Mutation::DecreaseExtension => shrink_extension(&mut us.body_extension),
            mutation if mutation.adorns() => us.adornments.grow(mutation),
            Mutation::Hydra if style == UnfishMutationStyle::Slime => {
                let cap = body_size.saturating_sub(1);
                let remaining = cap.saturating_sub(us.hydra_count);
                let requested = rng
                    .random_range(HYDRA_EYES_PER_APPLICATION_MIN..=HYDRA_EYES_PER_APPLICATION_MAX);
                us.hydra_count += requested.min(remaining);
            }
            Mutation::Hydra if style == UnfishMutationStyle::Worm => {
                let cap = us.worm_segments.saturating_sub(1);
                let remaining = cap.saturating_sub(us.hydra_count);
                let requested = rng
                    .random_range(HYDRA_EYES_PER_APPLICATION_MIN..=HYDRA_EYES_PER_APPLICATION_MAX);
                us.hydra_count += requested.min(remaining);
            }
            Mutation::ColorPatch => {
                let count = rng.random_range(MUTATION_PATCH_COUNT_MIN..=MUTATION_PATCH_COUNT_MAX);
                for _ in 0..count {
                    let pos = rng.random_range(0..sprite_width.max(1));
                    us.slime_color_patches.push((pos, random_rgb(rng)));
                }
                if us.slime_color_patches.len() > MUTATION_PATCH_MAX {
                    let excess = us.slime_color_patches.len() - MUTATION_PATCH_MAX;
                    us.slime_color_patches.drain(0..excess);
                }
            }
            _ => {}
        }
    }
    let us = fish.unfish_state.as_mut().unwrap();
    if style == UnfishMutationStyle::Worm {
        us.resync_worm_eye_colors(rng);
        fish.display_width = worm_display_width(
            us.worm_segments,
            us.worm_extra_eyes,
            us.worm_is_double,
            us.ear_count,
            us.hydra_count,
        );
    } else if !is_multi_row(us.kind) {
        let extras = us.ear_count + us.hydra_count + us.adornments.lead();
        fish.display_width = compute_display_width(FishSpecies::Unfish, fish.body_size) + extras;
    }
    MutationOutcome::Applied
}

impl Mutatable for Fish {
    fn capabilities(&self) -> &'static [Mutation] {
        if let Some(us) = self.unfish_state.as_ref() {
            return match us.kind.mutation_style() {
                UnfishMutationStyle::Slime => SLIME_CAPS,
                UnfishMutationStyle::Worm => WORM_CAPS,
            };
        }
        if !self.species.config().mutatable {
            return &[];
        }
        match self.species.config().body {
            BodyTemplate::Standard(_) | BodyTemplate::Alternating(_, _) => MUTANT_FULL_CAPS,
            BodyTemplate::Fixed { left, .. } => {
                if fixed_is_single_char(left) {
                    FIXED_JELLY_CAPS
                } else {
                    FIXED_MULTICHAR_CAPS
                }
            }
            BodyTemplate::Figure(_) => FIGURE_CAPS,
        }
    }

    fn adornments(&self) -> Adornments {
        Fish::adornments(self)
    }

    fn has_line_head(&self) -> bool {
        match self.unfish_state.as_ref() {
            Some(us) => !is_multi_row(us.kind) && us.kind != UnfishKind::Worm,
            None => true,
        }
    }

    fn is_double(&self) -> bool {
        if let Some(us) = self.unfish_state.as_ref() {
            return us.worm_is_double;
        }
        self.mutant.as_ref().is_some_and(|m| m.is_double)
    }

    fn backwards(&self) -> bool {
        if let Some(us) = self.unfish_state.as_ref() {
            return us.worm_backwards;
        }
        self.mutant.as_ref().is_some_and(|m| m.backwards)
    }

    fn has_glisten(&self) -> bool {
        if let Some(us) = self.unfish_state.as_ref() {
            return us.slime_glisten_enabled;
        }
        self.mutant
            .as_ref()
            .is_some_and(|m| m.glistening_color.is_some())
    }

    fn apply_one(&mut self, mutation: Mutation, rng: &mut impl RngExt) -> MutationOutcome {
        if self.unfish_state.is_some() {
            return apply_unfish_mutation(self, mutation, rng);
        }
        ensure_fish_mutant(self, rng);
        apply_mutant_mutation(self, mutation, rng)
    }

    fn record_mut(&mut self) -> &mut MutationRecord {
        self.mutations
            .get_or_insert_with(|| Box::new(MutationRecord::default()))
    }

    fn circadian(&self) -> Circadian {
        self.circadian_state()
    }

    fn ear_count(&self) -> usize {
        if let Some(us) = self.unfish_state.as_ref() {
            return us.ear_count;
        }
        self.mutant.as_ref().map_or(0, |m| m.ear_count)
    }

    fn max_ears(&self) -> usize {
        match self.unfish_state.as_ref().map(|us| us.kind) {
            Some(UnfishKind::Ball) => BALL_HEIGHT as usize,
            Some(UnfishKind::Skull) => SKULL_HEIGHT as usize,
            _ => MAX_EARS,
        }
    }

    fn hydra_count(&self) -> usize {
        if let Some(us) = self.unfish_state.as_ref() {
            return us.hydra_count;
        }
        self.mutant.as_ref().map_or(0, |m| m.hydra_eyes.len())
    }

    fn has_feet(&self) -> bool {
        if let Some(us) = self.unfish_state.as_ref() {
            return us.feet.is_some();
        }
        self.mutant.as_ref().is_some_and(|m| m.feet.is_some())
    }

    fn has_bodyextension(&self) -> bool {
        if let Some(us) = self.unfish_state.as_ref() {
            return us.body_extension.is_some();
        }
        self.mutant
            .as_ref()
            .is_some_and(|m| m.body_extension.is_some())
    }

    fn hydra_max(&self) -> usize {
        if let Some(us) = self.unfish_state.as_ref() {
            return match us.kind.mutation_style() {
                UnfishMutationStyle::Slime if !is_multi_row(us.kind) => {
                    self.body_size.saturating_sub(1)
                }
                UnfishMutationStyle::Worm => us.worm_segments.saturating_sub(1),
                _ => 0,
            };
        }
        match self.species.config().body {
            BodyTemplate::Standard(_) | BodyTemplate::Alternating(_, _) => {
                self.body_size.saturating_sub(1)
            }
            BodyTemplate::Fixed { .. } | BodyTemplate::Figure(_) => 0,
        }
    }
}

impl MutantBacked for Fish {
    fn body_size(&self) -> usize {
        self.body_size
    }
    fn set_body_size(&mut self, n: usize) {
        self.body_size = n;
    }
    fn color(&self) -> Color {
        self.color
    }
    fn set_color(&mut self, c: Color) {
        self.color = c;
        if let Some(mutant) = self.mutant.as_mut() {
            mutant.patterned = false;
        }
    }
    fn sway_speed(&self) -> f32 {
        self.sway_speed
    }
    fn set_sway_speed(&mut self, s: f32) {
        self.sway_speed = s;
    }
    fn default_sway_speed(&self) -> f32 {
        self.species.config().sway_speed
    }
    fn mutant(&self) -> &MutantState {
        self.mutant.as_ref().unwrap()
    }
    fn mutant_mut(&mut self) -> &mut MutantState {
        self.mutant.as_mut().unwrap()
    }
    fn doublefish_eye_count(&self, rng: &mut impl RngExt) -> usize {
        if self.has_shifting_body() {
            rng.random_range(1..=DOUBLE_EYE_COUNT_MAX)
        } else {
            match self.species.config().body {
                BodyTemplate::Standard(_) | BodyTemplate::Alternating(_, _) => 1,
                BodyTemplate::Fixed { .. } | BodyTemplate::Figure(_) => 0,
            }
        }
    }
    fn add_sell_bonus(&mut self, pct: u32) {
        self.sell_price_bonus_pct = self.sell_price_bonus_pct.saturating_add(pct);
    }
    fn gain_segment_mass(&mut self, grown_from: usize) {
        if matches!(
            self.species.config().body,
            BodyTemplate::Fixed { .. } | BodyTemplate::Figure(_)
        ) {
            return;
        }
        let Ok(segments) = u32::try_from(grown_from) else {
            return;
        };
        if segments == 0 {
            return;
        }
        self.weight_g = self.weight_g.saturating_add(self.weight_g / segments);
    }
    fn self_component(&self) -> FusedComponent {
        FusedComponent::fish(self.species, self.name.clone(), self.weight_g)
    }
    fn arm_engulf(&mut self) {
        self.engulf_timer = ENGULF_WINDOW_SECS;
    }
    fn recompute_display_width(&mut self) {
        if self.fused_render_halves().is_some() {
            self.display_width = self.fused_render_width();
            return;
        }
        let Some(mutant) = self.mutant.as_ref() else {
            return;
        };
        let lead = mutant.adornments.lead();
        self.display_width = match self.species.config().body {
            BodyTemplate::Fixed { left, .. } => {
                let n_chars = left.first().map(|s| s.chars().count()).unwrap_or(1);
                let width = if n_chars > 1 {
                    let n_eyes = mutant.left_eyes.len().max(mutant.right_eyes.len());
                    if mutant.is_double {
                        2 * (1 + n_eyes + self.body_size)
                    } else {
                        2 + n_eyes + self.body_size
                    }
                } else if mutant.is_double {
                    2
                } else {
                    compute_display_width(self.species, self.body_size)
                };
                width + lead
            }
            BodyTemplate::Figure(_) => compute_display_width(self.species, self.body_size),
            _ => mutant.display_width(self.body_size),
        };
    }
}

pub fn apply_mutation<M: Mutatable>(
    target: &mut M,
    mutation: Mutation,
    rng: &mut impl RngExt,
) -> MutationOutcome {
    let outcome = target.apply_one(mutation, rng);
    let record = target.record_mut();
    record.count += 1;
    record.history.push(mutation.token().to_string());
    outcome
}

pub fn apply_mutation_to_fish(fish: &mut Fish, mutation: Mutation, rng: &mut impl RngExt) {
    apply_mutation(fish, mutation, rng);
}
