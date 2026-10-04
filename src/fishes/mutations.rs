use rand::{RngExt, SeedableRng, rngs::SmallRng};
use ratatui::style::Color;

use super::fish::{ENGULF_WINDOW_SECS, Fish, compute_display_width};
use super::fused::FusedComponent;
use super::mutant::{
    Adornments, Circadian, EyeState, MIN_BODY_CHARS, Mark, MutantState, MutantTail, MutationRecord,
    random_rgb, random_rgb_other,
};
use super::revert::{Look, can_revert, revert, settle_old_record};
use super::species::{BodyTemplate, FishSpecies, SINGLE_EYE, TailKind};
use super::unfish::{
    BALL_HEIGHT, SKULL_HEIGHT, SLIME_GLISTEN_SPEED_FAST, SLIME_GLISTEN_SPEED_SLOW, UnfishKind,
    UnfishMutationStyle, is_multi_row, worm_display_width,
};
use crate::colors::{DARK_GRAY, LIGHT_GREEN, PINK, WHITE};
use crate::sprite::{Band, BodyExtension, ExtensionVariant, Feet, FeetStyle};

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
const LEGACY_WAKE_TOKEN: &str = "bubblecolor";
const WORM_MAX_SEGMENTS: usize = 12;
const WORM_MAX_EXTRA_EYES: usize = 4;
const DOUBLE_EYE_COUNT_MAX: usize = 3;
const MOUTH_CELLS: usize = 1;
const MAX_EARS: usize = 4;
const HYDRA_EYES_PER_APPLICATION_MIN: usize = 1;
const HYDRA_EYES_PER_APPLICATION_MAX: usize = 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mutation {
    SizeIncrease,
    EyeIncrease,
    ColorPatch,
    EyeColor,
    GlistenFast,
    GlistenSlow,
    GlistenMode,
    GlistenColor,
    GlistenEnable,
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
    WakeColor,
    NightOwl,
    HelpedByGod,
    Heterochromia,
    Ear,
    EarColor,
    Hydra,
    Feet,
    FeetColor,
    Spikes,
    Wings,
    Tentacles,
    Lure,
    Bill,
    DorsalFin,
    VentralFin,
    Lunar,
    Puff,
    Revert,
}

impl Mutation {
    pub const ALL: &'static [Mutation] = &[
        Mutation::SizeIncrease,
        Mutation::EyeIncrease,
        Mutation::ColorPatch,
        Mutation::EyeColor,
        Mutation::GlistenFast,
        Mutation::GlistenSlow,
        Mutation::GlistenMode,
        Mutation::GlistenColor,
        Mutation::GlistenEnable,
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
        Mutation::WakeColor,
        Mutation::NightOwl,
        Mutation::HelpedByGod,
        Mutation::Heterochromia,
        Mutation::Ear,
        Mutation::EarColor,
        Mutation::Hydra,
        Mutation::Feet,
        Mutation::FeetColor,
        Mutation::Spikes,
        Mutation::Wings,
        Mutation::Tentacles,
        Mutation::Lure,
        Mutation::Bill,
        Mutation::DorsalFin,
        Mutation::VentralFin,
        Mutation::Lunar,
        Mutation::Puff,
        Mutation::Revert,
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
            Mutation::EyeIncrease => "eyeincrease",
            Mutation::ColorPatch => "colorpatch",
            Mutation::EyeColor => "eyecolor",
            Mutation::GlistenFast => "glistenfast",
            Mutation::GlistenSlow => "glistenslow",
            Mutation::GlistenMode => "glistenmode",
            Mutation::GlistenColor => "glistencolor",
            Mutation::GlistenEnable => "glistenenable",
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
            Mutation::WakeColor => "wakecolor",
            Mutation::NightOwl => "nightowl",
            Mutation::HelpedByGod => "helpedbygod",
            Mutation::Heterochromia => "heterochromia",
            Mutation::Ear => "ear",
            Mutation::EarColor => "earcolor",
            Mutation::Hydra => "hydra",
            Mutation::Feet => "feet",
            Mutation::FeetColor => "feetcolor",
            Mutation::Spikes => "spikes",
            Mutation::Wings => "wings",
            Mutation::Tentacles => "tentacles",
            Mutation::Lure => "lure",
            Mutation::Bill => "bill",
            Mutation::DorsalFin => "dorsalfin",
            Mutation::VentralFin => "ventralfin",
            Mutation::Lunar => "lunar",
            Mutation::Puff => "puff",
            Mutation::Revert => "revert",
        }
    }

    pub fn parse(s: &str) -> Option<Mutation> {
        let lower = s.to_ascii_lowercase();
        if lower == LEGACY_WAKE_TOKEN {
            return Some(Mutation::WakeColor);
        }
        Mutation::ALL.iter().copied().find(|m| m.token() == lower)
    }

    pub fn auto_selectable(self) -> bool {
        !matches!(self, Mutation::Strawberry | Mutation::Alienation)
    }

    pub fn divides(self) -> bool {
        matches!(self, Mutation::Cytokinesis)
    }

    pub fn fuses(self) -> bool {
        matches!(
            self,
            Mutation::Telophase
                | Mutation::BackwardsTelophase
                | Mutation::Cytokinesis
                | Mutation::Endocytosis
                | Mutation::Engulfment
        )
    }

    pub fn can_be_reverted(self) -> bool {
        !self.fuses() && self != Mutation::Revert
    }

    pub fn adorns(self) -> bool {
        Mutation::ADORNMENTS.contains(&self)
    }

    pub fn extension(self) -> Option<ExtensionVariant> {
        match self {
            Mutation::Spikes => Some(ExtensionVariant::Spike),
            Mutation::Wings => Some(ExtensionVariant::Wing),
            Mutation::Tentacles => Some(ExtensionVariant::Tentacle),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Holder {
    Nothing,
    Fin,
    Feet,
    Extension(ExtensionVariant),
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

fn extend(slot: &mut Option<BodyExtension>, variant: ExtensionVariant, bands: &[Band]) {
    let extension = slot.get_or_insert_default();
    for &band in bands {
        extension.set(band, Some(variant));
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
    fn record(&self) -> Option<&MutationRecord>;
    fn record_mut(&mut self) -> &mut MutationRecord;
    fn look(&self) -> Look;
    fn wear(&mut self, look: &Look);
    fn refresh_width(&mut self);

    fn worth(&self) -> (u32, u32) {
        (0, 0)
    }

    fn set_worth(&mut self, _worth: (u32, u32)) {}

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

    fn extension(&self) -> BodyExtension {
        BodyExtension::default()
    }

    fn reserves(&self, _band: Band) -> bool {
        false
    }

    fn holds(&self, band: Band) -> Holder {
        let adornments = self.adornments();
        let fin = match band {
            Band::Top => adornments.dorsal_fin,
            Band::Bottom => adornments.ventral_fin,
        };
        if fin {
            return Holder::Fin;
        }
        if band == Band::Bottom && self.has_feet() {
            return Holder::Feet;
        }
        match self.extension().on(band) {
            Some(variant) => Holder::Extension(variant),
            None => Holder::Nothing,
        }
    }

    fn band_free(&self, band: Band) -> bool {
        !self.reserves(band) && self.holds(band) == Holder::Nothing
    }

    fn can_extend(&self, band: Band, variant: ExtensionVariant) -> bool {
        if self.reserves(band) {
            return false;
        }
        match self.holds(band) {
            Holder::Nothing => true,
            Holder::Extension(held) => held != variant,
            Holder::Fin | Holder::Feet => false,
        }
    }

    fn bands_to_extend(&self, variant: ExtensionVariant) -> Vec<Band> {
        variant
            .bands()
            .iter()
            .copied()
            .filter(|&band| self.can_extend(band, variant))
            .collect()
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

    fn leaves_a_wake(&self) -> bool {
        true
    }

    fn can_grow_body(&self) -> bool {
        true
    }

    fn can_grow_eyes(&self) -> bool {
        true
    }

    fn shows_eye_colour(&self) -> bool {
        true
    }

    fn shifts_body(&self) -> bool {
        false
    }

    fn turns_mouth(&self) -> bool {
        true
    }

    fn head_differs_from_tail(&self) -> bool {
        true
    }

    fn supports_now(&self, mutation: Mutation) -> bool {
        if !self.capabilities().contains(&mutation) {
            return false;
        }
        if let Some(variant) = mutation.extension() {
            return !self.bands_to_extend(variant).is_empty();
        }
        let adornments = self.adornments();
        if mutation.adorns() && (!self.has_line_head() || adornments.has(mutation)) {
            return false;
        }
        match mutation {
            Mutation::Lure | Mutation::Bill => !adornments.lure && !adornments.bill,
            Mutation::DorsalFin => self.band_free(Band::Top),
            Mutation::VentralFin => self.band_free(Band::Bottom),
            Mutation::Telophase => {
                !self.is_double() || (self.backwards() && self.head_differs_from_tail())
            }
            Mutation::BackwardsTelophase => {
                !self.is_double() || (!self.backwards() && self.head_differs_from_tail())
            }
            Mutation::SizeIncrease => self.can_grow_body(),
            Mutation::EyeIncrease => self.can_grow_eyes(),
            Mutation::EyeColor | Mutation::Heterochromia => self.shows_eye_colour(),
            Mutation::GlistenMode => self.has_glisten(),
            Mutation::ColorPatch => !self.has_glisten() && !adornments.lunar,
            Mutation::BodyVariant => self.shifts_body(),
            Mutation::MouthVariant => self.turns_mouth(),
            Mutation::TailVariant => !self.is_double() || self.backwards(),
            Mutation::Cytokinesis => self.is_double(),
            Mutation::Endocytosis => self.is_double(),
            Mutation::Engulfment => !self.is_double(),
            Mutation::GlistenEnable => !self.has_glisten(),
            Mutation::NightOwl => self.circadian() != Circadian::NightOwl,
            Mutation::HelpedByGod => self.circadian() != Circadian::HelpedByGod,
            Mutation::Ear => self.ear_count() < self.max_ears(),
            Mutation::EarColor => self.ear_count() > 0,
            Mutation::Hydra => self.hydra_count() < self.hydra_max(),
            Mutation::Feet => self.band_free(Band::Bottom),
            Mutation::FeetColor => self.has_feet(),
            Mutation::WakeColor => self.leaves_a_wake(),
            Mutation::Revert => can_revert(self),
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
    fn color_patch_range(&self) -> std::ops::Range<usize> {
        let mutant = self.mutant();
        body_cells(mutant, mutant.display_width(self.body_size()))
    }
    fn hydra_capacity(&self) -> usize {
        self.body_size().saturating_sub(1)
    }
}

fn body_cells(mutant: &MutantState, width: usize) -> std::ops::Range<usize> {
    let eyes = mutant.left_eyes.len().max(mutant.right_eyes.len());
    let inserted = mutant.adornments.lead() + mutant.ear_count + mutant.hydra_eyes.len();
    let end = width.saturating_sub(inserted).max(1);
    let start = (MOUTH_CELLS + eyes).min(end - 1);
    start..end
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

pub fn apply_mutant_mutation<T: MutantBacked + Mutatable>(
    target: &mut T,
    mutation: Mutation,
    rng: &mut impl RngExt,
) -> MutationOutcome {
    if let Some(variant) = mutation.extension() {
        let bands = target.bands_to_extend(variant);
        extend(&mut target.mutant_mut().body_extension, variant, &bands);
        target.recompute_display_width();
        return MutationOutcome::Applied;
    }
    match mutation {
        Mutation::SizeIncrease => {
            let old_size = target.body_size();
            let new_size = (old_size + 1).min(MUTATION_MAX_BODY_SIZE).max(old_size);
            target.set_body_size(new_size);
            if new_size > old_size {
                target.gain_segment_mass(old_size);
            }
            let hydra_cap = target.hydra_capacity();
            target.mutant_mut().hydra_eyes.truncate(hydra_cap);
        }
        Mutation::ColorPatch => {
            let cells = target.color_patch_range();
            let skin = target.color();
            let count = rng.random_range(MUTATION_PATCH_COUNT_MIN..=MUTATION_PATCH_COUNT_MAX);
            let mutant = target.mutant_mut();
            for _ in 0..count {
                let pos = rng.random_range(cells.clone());
                mutant
                    .color_patches
                    .push((pos, random_rgb_other(rng, &[Some(skin)])));
            }
            if mutant.color_patches.len() > MUTATION_PATCH_MAX {
                let excess = mutant.color_patches.len() - MUTATION_PATCH_MAX;
                mutant.color_patches.drain(0..excess);
            }
        }
        Mutation::EyeIncrease => {
            let body_size = target.body_size();
            let max_eyes = body_size.saturating_sub(MIN_BODY_CHARS).clamp(1, 4);
            let mutant = target.mutant_mut();
            for eyes in [&mut mutant.left_eyes, &mut mutant.right_eyes] {
                let grown = (eyes.len() + 1).min(max_eyes).max(eyes.len());
                while eyes.len() < grown {
                    eyes.push(EyeState::new(rng));
                }
            }
        }
        Mutation::EyeColor => {
            let mutant = target.mutant_mut();
            if mutant.heterochromia {
                mutant.randomize_one_eye_color(rng);
            } else {
                mutant.eye_color = Some(random_rgb_other(rng, &[mutant.eye_color]));
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
            mutant.glistening_color = Some(random_rgb_other(rng, &[mutant.glistening_color]));
        }
        Mutation::BodyColor => {
            let worn = target.color();
            target.set_color(random_rgb_other(rng, &[Some(worn)]));
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
        Mutation::WakeColor => {
            let mutant = target.mutant_mut();
            mutant.wake_color = Some(random_rgb_other(rng, &[mutant.wake_color]));
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
        Mutation::EarColor => {
            let mutant = target.mutant_mut();
            mutant.ear_color = Some(random_rgb_other(rng, &[mutant.ear_color]));
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
        Mutation::FeetColor => {
            if let Some(feet) = target.mutant_mut().feet.as_mut() {
                feet.color = Some(random_rgb_other(rng, &[feet.color]));
            }
        }
        Mutation::Spikes | Mutation::Wings | Mutation::Tentacles | Mutation::Revert => {}
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
    Mutation::EyeIncrease,
    Mutation::ColorPatch,
    Mutation::EyeColor,
    Mutation::GlistenFast,
    Mutation::GlistenSlow,
    Mutation::GlistenMode,
    Mutation::GlistenColor,
    Mutation::GlistenEnable,
    Mutation::BodyColor,
    Mutation::Telophase,
    Mutation::BackwardsTelophase,
    Mutation::Cytokinesis,
    Mutation::Endocytosis,
    Mutation::Alienation,
    Mutation::Strawberry,
    Mutation::WakeColor,
    Mutation::NightOwl,
    Mutation::HelpedByGod,
    Mutation::Heterochromia,
    Mutation::Feet,
    Mutation::FeetColor,
    Mutation::Spikes,
    Mutation::Wings,
    Mutation::Tentacles,
    Mutation::Lure,
    Mutation::Bill,
    Mutation::DorsalFin,
    Mutation::VentralFin,
    Mutation::Lunar,
    Mutation::Puff,
    Mutation::Revert,
];

const BOTFISH_CAPS: &[Mutation] = &[
    Mutation::Telophase,
    Mutation::Cytokinesis,
    Mutation::Endocytosis,
    Mutation::Engulfment,
    Mutation::Alienation,
    Mutation::Strawberry,
    Mutation::BodyColor,
    Mutation::WakeColor,
    Mutation::NightOwl,
    Mutation::HelpedByGod,
    Mutation::Feet,
    Mutation::FeetColor,
    Mutation::Spikes,
    Mutation::Wings,
    Mutation::Tentacles,
    Mutation::Lure,
    Mutation::Bill,
    Mutation::VentralFin,
    Mutation::Lunar,
    Mutation::Puff,
    Mutation::Revert,
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
    Mutation::WakeColor,
    Mutation::NightOwl,
    Mutation::HelpedByGod,
    Mutation::Feet,
    Mutation::FeetColor,
    Mutation::Spikes,
    Mutation::Wings,
    Mutation::Tentacles,
    Mutation::Lure,
    Mutation::DorsalFin,
    Mutation::VentralFin,
    Mutation::Puff,
    Mutation::Revert,
];

const SLIME_CAPS: &[Mutation] = &[
    Mutation::EyeIncrease,
    Mutation::ColorPatch,
    Mutation::EyeColor,
    Mutation::GlistenFast,
    Mutation::GlistenSlow,
    Mutation::GlistenMode,
    Mutation::GlistenColor,
    Mutation::GlistenEnable,
    Mutation::BodyColor,
    Mutation::NightOwl,
    Mutation::HelpedByGod,
    Mutation::Heterochromia,
    Mutation::Ear,
    Mutation::EarColor,
    Mutation::Hydra,
    Mutation::Feet,
    Mutation::FeetColor,
    Mutation::Spikes,
    Mutation::Wings,
    Mutation::Tentacles,
    Mutation::Lure,
    Mutation::Bill,
    Mutation::DorsalFin,
    Mutation::VentralFin,
    Mutation::Lunar,
    Mutation::Revert,
];

const FIGURE_CAPS: &[Mutation] = &[
    Mutation::GlistenFast,
    Mutation::GlistenSlow,
    Mutation::GlistenMode,
    Mutation::GlistenColor,
    Mutation::GlistenEnable,
    Mutation::BodyColor,
    Mutation::Alienation,
    Mutation::Strawberry,
    Mutation::WakeColor,
    Mutation::NightOwl,
    Mutation::HelpedByGod,
    Mutation::DorsalFin,
    Mutation::VentralFin,
    Mutation::Feet,
    Mutation::FeetColor,
    Mutation::Spikes,
    Mutation::Wings,
    Mutation::Tentacles,
    Mutation::Revert,
];

const WORM_CAPS: &[Mutation] = &[
    Mutation::SizeIncrease,
    Mutation::EyeIncrease,
    Mutation::ColorPatch,
    Mutation::EyeColor,
    Mutation::GlistenFast,
    Mutation::GlistenSlow,
    Mutation::GlistenMode,
    Mutation::GlistenColor,
    Mutation::GlistenEnable,
    Mutation::BodyColor,
    Mutation::Telophase,
    Mutation::BackwardsTelophase,
    Mutation::Cytokinesis,
    Mutation::Endocytosis,
    Mutation::Engulfment,
    Mutation::NightOwl,
    Mutation::HelpedByGod,
    Mutation::Heterochromia,
    Mutation::Ear,
    Mutation::EarColor,
    Mutation::Hydra,
    Mutation::Feet,
    Mutation::FeetColor,
    Mutation::Spikes,
    Mutation::Wings,
    Mutation::Tentacles,
    Mutation::Revert,
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

impl Fish {
    fn eye_counts(&self) -> (usize, usize) {
        if let Some(mutant) = self.mutant.as_ref() {
            return (mutant.left_eyes.len(), mutant.right_eyes.len());
        }
        match self.species.config().body {
            BodyTemplate::Standard(_) | BodyTemplate::Alternating(_, _) => (1, 1),
            BodyTemplate::Fixed { .. } | BodyTemplate::Figure(_) => (0, 0),
        }
    }

    fn natural_body_size(&self) -> usize {
        match self.species.config().body {
            BodyTemplate::Fixed { left, .. } if self.mutant.is_none() => left
                .first()
                .map_or(1, |row| row.chars().count())
                .saturating_sub(2),
            _ => self.body_size,
        }
    }

    fn can_grow_body(&self) -> bool {
        if let Some(us) = self.unfish_state.as_ref() {
            return us.worm_segments < WORM_MAX_SEGMENTS;
        }
        self.natural_body_size() < MUTATION_MAX_BODY_SIZE
    }

    fn can_grow_eyes(&self) -> bool {
        if let Some(us) = self.unfish_state.as_ref() {
            return match us.kind.mutation_style() {
                UnfishMutationStyle::Worm => us.worm_extra_eyes < WORM_MAX_EXTRA_EYES,
                UnfishMutationStyle::Slime => us.has_room_for_an_eye(),
            };
        }
        let (left, right) = self.eye_counts();
        let most = self
            .natural_body_size()
            .saturating_sub(MIN_BODY_CHARS)
            .clamp(1, 4);
        left < most || right < most
    }

    fn shows_eye_colour(&self) -> bool {
        if let Some(us) = self.unfish_state.as_ref() {
            return us.has_an_eye();
        }
        let (left, right) = self.eye_counts();
        left + right > 0
    }

    fn turns_mouth(&self) -> bool {
        let turns = |mouth: char| invert_mouth_glyph(mouth) != mouth;
        if self.has_shifting_body() {
            return true;
        }
        match self.species.config().body {
            BodyTemplate::Standard(chars) | BodyTemplate::Alternating(chars, _) => {
                turns(chars.mouth_left)
            }
            BodyTemplate::Fixed { left, .. } => left
                .first()
                .and_then(|row| row.chars().next())
                .is_some_and(turns),
            BodyTemplate::Figure(_) => false,
        }
    }

    fn head_differs_from_tail(&self) -> bool {
        let BodyTemplate::Fixed { left, .. } = self.species.config().body else {
            return true;
        };
        let row: Vec<char> = left
            .first()
            .map_or_else(Vec::new, |row| row.chars().collect());
        let (eyes, _) = self.eye_counts();
        row.len() > 1 && (row.first() != row.last() || eyes > 0)
    }
}

fn invert_mouth_glyph(glyph: char) -> char {
    match glyph {
        '<' => '>',
        '>' => '<',
        other => other,
    }
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
    let extension = mutation
        .extension()
        .map(|variant| (variant, fish.bands_to_extend(variant)));
    {
        let us = fish.unfish_state.as_mut().unwrap();
        match mutation {
            Mutation::SizeIncrease if style == UnfishMutationStyle::Worm => {
                us.worm_segments = (us.worm_segments + 1).min(WORM_MAX_SEGMENTS);
            }
            Mutation::EyeIncrease => match style {
                UnfishMutationStyle::Worm => {
                    us.worm_extra_eyes = (us.worm_extra_eyes + 1).min(WORM_MAX_EXTRA_EYES);
                }
                UnfishMutationStyle::Slime => us.add_floating_eye(rng),
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
            Mutation::BodyColor => {
                us.slime_body_color = Some(random_rgb_other(rng, &[us.slime_body_color]))
            }
            Mutation::EyeColor => us.recolor_one_eye(rng),
            Mutation::Heterochromia => us.make_heterochromatic(rng),
            Mutation::GlistenEnable => us.slime_glisten_enabled = true,
            Mutation::GlistenFast => us.slime_glisten_speed = SLIME_GLISTEN_SPEED_FAST,
            Mutation::GlistenSlow => us.slime_glisten_speed = SLIME_GLISTEN_SPEED_SLOW,
            Mutation::GlistenMode => {
                us.slime_glisten_mode = us.slime_glisten_mode.random_other(rng)
            }
            Mutation::GlistenColor => {
                us.slime_glisten_color = Some(random_rgb_other(rng, &[us.slime_glisten_color]));
                us.slime_glisten_enabled = true;
            }
            Mutation::WakeColor => us.wake_color = Some(random_rgb_other(rng, &[us.wake_color])),
            Mutation::NightOwl => us.circadian = Circadian::NightOwl,
            Mutation::HelpedByGod => us.circadian = Circadian::HelpedByGod,
            Mutation::Ear => us.ear_count += 1,
            Mutation::EarColor => us.ear_color = Some(random_rgb_other(rng, &[us.ear_color])),
            Mutation::Feet => us.feet = Some(roll_feet(rng)),
            Mutation::FeetColor => {
                if let Some(feet) = us.feet.as_mut() {
                    feet.color = Some(random_rgb_other(rng, &[feet.color]));
                }
            }
            Mutation::Spikes | Mutation::Wings | Mutation::Tentacles => {
                if let Some((variant, bands)) = &extension {
                    extend(&mut us.body_extension, *variant, bands);
                }
            }
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
    if style == UnfishMutationStyle::Worm
        && let Some(us) = fish.unfish_state.as_mut()
    {
        us.resync_worm_eye_colors(rng);
    }
    refresh_unfish_width(fish);
    MutationOutcome::Applied
}

fn refresh_unfish_width(fish: &mut Fish) {
    let Some(us) = fish.unfish_state.as_ref() else {
        return;
    };
    if us.kind.mutation_style() == UnfishMutationStyle::Worm {
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
        if self.botfish_state.is_some() {
            return BOTFISH_CAPS;
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
        self.species.config().auto_glisten
            || self
                .mutant
                .as_ref()
                .is_some_and(|m| m.glistening_color.is_some())
    }

    fn can_grow_body(&self) -> bool {
        Fish::can_grow_body(self)
    }

    fn can_grow_eyes(&self) -> bool {
        Fish::can_grow_eyes(self)
    }

    fn shows_eye_colour(&self) -> bool {
        Fish::shows_eye_colour(self)
    }

    fn shifts_body(&self) -> bool {
        self.has_shifting_body()
    }

    fn turns_mouth(&self) -> bool {
        Fish::turns_mouth(self)
    }

    fn head_differs_from_tail(&self) -> bool {
        Fish::head_differs_from_tail(self)
    }

    fn apply_one(&mut self, mutation: Mutation, rng: &mut impl RngExt) -> MutationOutcome {
        if self.unfish_state.is_some() {
            return apply_unfish_mutation(self, mutation, rng);
        }
        ensure_fish_mutant(self, rng);
        apply_mutant_mutation(self, mutation, rng)
    }

    fn record(&self) -> Option<&MutationRecord> {
        self.mutations.as_deref()
    }

    fn record_mut(&mut self) -> &mut MutationRecord {
        self.mutations
            .get_or_insert_with(|| Box::new(MutationRecord::default()))
    }

    fn look(&self) -> Look {
        Fish::look(self)
    }

    fn wear(&mut self, look: &Look) {
        Fish::wear(self, look);
    }

    fn refresh_width(&mut self) {
        if self.unfish_state.is_some() {
            refresh_unfish_width(self);
            return;
        }
        self.recompute_display_width();
    }

    fn worth(&self) -> (u32, u32) {
        (self.weight_g, self.sell_price_bonus_pct)
    }

    fn set_worth(&mut self, (weight_g, bonus_pct): (u32, u32)) {
        self.weight_g = weight_g;
        self.sell_price_bonus_pct = bonus_pct;
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

    fn extension(&self) -> BodyExtension {
        self.body_extension()
    }

    fn reserves(&self, band: Band) -> bool {
        Fish::reserves(self, band)
    }

    fn leaves_a_wake(&self) -> bool {
        Fish::leaves_a_wake(self)
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
    fn color_patch_range(&self) -> std::ops::Range<usize> {
        body_cells(self.mutant(), self.display_width)
    }
    fn arm_engulf(&mut self) {
        self.engulf_timer = ENGULF_WINDOW_SECS;
    }
    fn recompute_display_width(&mut self) {
        if self.fused_render_halves().is_some() {
            self.display_width = self.fused_render_width();
            return;
        }
        if let Some(width) = self.botfish_width() {
            self.display_width = width;
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
    if mutation == Mutation::Revert {
        revert(target, rng);
        return MutationOutcome::Applied;
    }
    settle_old_record(target);
    if target.record().is_none_or(|record| record.origin.is_none()) {
        let look = target.look();
        target.record_mut().settle(look);
    }
    let seed: u64 = rng.random();
    let (mass_before, bonus_before) = target.worth();
    let outcome = target.apply_one(mutation, &mut SmallRng::seed_from_u64(seed));
    let (mass_after, bonus_after) = target.worth();
    target.record_mut().note(
        mutation,
        Mark {
            seed,
            mass_g: mass_after.saturating_sub(mass_before),
            bonus_pct: bonus_after.saturating_sub(bonus_before),
        },
    );
    outcome
}

pub fn apply_mutation_to_fish(fish: &mut Fish, mutation: Mutation, rng: &mut impl RngExt) {
    apply_mutation(fish, mutation, rng);
}
