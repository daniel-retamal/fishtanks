use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

use ratatui::style::Color;

use super::art::{CABALLITO, CARACOL, COFRE, ERMITANO, MARTILLO, NEON, PULPO};
use super::figure::Figure;
use super::mutations::Mutation;
use crate::economy::{Money, Purchasable, Rarity, Sellable};
use crate::entities::food::{FOOD_BUY_PRICE, FOOD_WEIGHT_GAIN_G};
use crate::loot::{CashValue, JUNK_OUTLINE, StockItem};
use crate::tank::TankKind;

pub const SINGLE_EYE: usize = 1;
pub const CHEATFISH_EYES: usize = 3;
pub const EYE_ROUND: char = 'º';
pub const EYE_CIRCLE: char = 'ʘ';
pub const EYE_DEAD: char = 'Ↄ';
pub const EYE_ROUND_SHUT: char = '¯';
pub const EYE_CIRCLE_SHUT: char = '-';

pub fn shut_eye(glyph: char) -> char {
    match glyph {
        EYE_ROUND => EYE_ROUND_SHUT,
        EYE_CIRCLE => EYE_CIRCLE_SHUT,
        other => other,
    }
}
pub const TAIL_WAVE_LEFT: char = '彡';
pub const TAIL_WAVE_RIGHT: char = 'ミ';
pub const TAIL_EQUAL: char = '≡';
use crate::colors::{
    AMBER, AMBER_DARK, AMBER_LIGHT, BLUE, BROWN, BROWN_DARK, CREAM, CYAN, DARK_GRAY, FOREST, GOLD,
    GRAY, GREEN_BRIGHT, GREEN_DARK, GREEN_LIGHT, KHAKI, LIGHT_BLUE, LIGHT_CYAN, LIGHT_MAGENTA,
    LIGHT_RED, LIGHT_YELLOW, MAGENTA, NAVY, NAVY_DARK, NAVY_LIGHT, OLIVE, OLIVE_LIGHT, ORANGE,
    ORANGE_DARK, ORANGE_LIGHT, PINK, PURPLE, PURPLE_LIGHT, RED, RED_DARK, SILVER, STEEL, TAN, TEAL,
    TERRACOTTA, VIOLET, WHITE, YELLOW,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SizeCategory {
    S = 0,
    M = 1,
    L = 2,
    XL = 3,
}

impl SizeCategory {
    pub const ALL: [SizeCategory; 4] = [
        SizeCategory::S,
        SizeCategory::M,
        SizeCategory::L,
        SizeCategory::XL,
    ];
    pub const ODDS_TOTAL: u32 = 100;

    pub fn odds(self) -> u32 {
        match self {
            SizeCategory::S => 55,
            SizeCategory::M => 30,
            SizeCategory::L => 12,
            SizeCategory::XL => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FishSpecies {
    Merluza,
    Betta,
    Salmon,
    Chromis,
    Tang,
    Koi,
    Carpin,
    Turbofish,
    Deadfish,
    Anchoveta,
    Jellyfish,
    Cashfish,
    Goldfish,
    Snapper,
    Mutantfish,
    Nishiki,
    Aka,
    Kuro,
    Candyfish,
    Holyfish,
    Botfish,
    Cheatfish,
    Unfish,
    Junkfish,
    Caracol,
    Babosa,
    Estrella,
    Ermitano,
    Lenguado,
    Piedra,
    Pejesapo,
    Cofre,
    Caballito,
    Morena,
    Volador,
    Espada,
    Tollo,
    Mariposa,
    Linterna,
    Luciernaga,
    Bagre,
    Luna,
    Loro,
    Mimo,
    Timido,
    Ciego,
    Globo,
    Pulpo,
    Draco,
    Neon,
    Martillo,
}

impl FishSpecies {
    pub fn display_name(self) -> &'static str {
        self.config().name
    }

    pub fn all_buyable() -> &'static [FishSpecies] {
        &BUYABLE_SPECIES
    }

    pub fn all_wild() -> &'static [FishSpecies] {
        &WILD_SPECIES
    }

    pub fn native_to(kind: TankKind) -> Vec<FishSpecies> {
        ALL_SPECIES
            .iter()
            .copied()
            .filter(|species| species.config().habitat == Habitat::Native(kind))
            .collect()
    }

    pub fn buy_price(self) -> u32 {
        self.config().rarity.fish_buy_price()
    }

    pub fn from_junk() -> Option<FishSpecies> {
        ALL_SPECIES
            .iter()
            .copied()
            .find(|species| species.config().habitat == Habitat::Junkpile)
    }

    pub fn appraisal(self, size: SizeCategory, seed: u64) -> Money {
        self.config()
            .appraisal
            .map_or(0, |appraisal| appraisal.worth(size, seed))
    }

    pub fn is_obtainable(self) -> bool {
        let config = self.config();
        config.buyable || config.habitat != Habitat::Nowhere
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Habitat {
    Everywhere,
    Native(TankKind),
    Junkpile,
    Nowhere,
}

impl Habitat {
    pub fn is_fished(self) -> bool {
        matches!(self, Habitat::Everywhere | Habitat::Native(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyFill {
    Species,
    Junk,
    Stones,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EyeAt {
    Head,
    Middle,
}

const APPRAISAL_LUCK_BITS: u32 = 53;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Appraisal {
    pub floor: Money,
    pub ceiling: Money,
}

impl Appraisal {
    pub fn worth(self, size: SizeCategory, seed: u64) -> Money {
        let luck = (seed >> (u64::BITS - APPRAISAL_LUCK_BITS)) as f64
            / (1u64 << APPRAISAL_LUCK_BITS) as f64;
        let reach = (size as usize as f64 + luck) / SizeCategory::ALL.len() as f64;
        let ratio = self.ceiling as f64 / self.floor as f64;
        (self.floor as f64 * ratio.powf(reach)).round() as Money
    }
}

const JUNKFISH_APPRAISAL: Appraisal = Appraisal {
    floor: CashValue::HundredThousand.amount() as Money,
    ceiling: CashValue::Million.amount() as Money,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tint {
    Plain,
    Body,
    Fixed(Color),
}

impl Tint {
    pub fn resolve(self, plain: Color, body: Color) -> Color {
        match self {
            Tint::Plain => plain,
            Tint::Body => body,
            Tint::Fixed(color) => color,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zoomie {
    None,
    Burst,
    Vertical,
    Still,
    Glide,
    Lunge,
    Hop,
    Ink,
}

impl Zoomie {
    pub fn zooms(self) -> bool {
        self != Zoomie::None
    }

    pub fn moves(self) -> bool {
        !matches!(self, Zoomie::None | Zoomie::Still)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locomotion {
    Swim,
    Through,
    Bounce,
    Floor,
    Sideways,
    Glass,
}

impl Locomotion {
    pub fn swims(self) -> bool {
        matches!(self, Locomotion::Swim | Locomotion::Through)
    }

    pub fn walks(self) -> bool {
        matches!(self, Locomotion::Floor | Locomotion::Sideways)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Habit {
    Sync,
    Pair,
    Duel,
    ShellSwap,
    Chase,
    Shadow,
    Echo,
    Shy,
    Blind,
    Backwards,
    Ambush,
    Escape,
    Gape,
    Twinkle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cycle {
    OnBounce,
    OnClock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skin {
    Palette,
    Camouflage,
    Cycle(Cycle),
    SeeThrough,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fortune {
    Doomed,
    Golden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sin {
    Lust,
    Gluttony,
    Greed,
    Sloth,
    Wrath,
    Envy,
    Pride,
}

impl Sin {
    pub const ALL: [Sin; 7] = [
        Sin::Lust,
        Sin::Gluttony,
        Sin::Greed,
        Sin::Sloth,
        Sin::Wrath,
        Sin::Envy,
        Sin::Pride,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Sin::Lust => "Lust",
            Sin::Gluttony => "Gluttony",
            Sin::Greed => "Greed",
            Sin::Sloth => "Sloth",
            Sin::Wrath => "Wrath",
            Sin::Envy => "Envy",
            Sin::Pride => "Pride",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Flavour {
    pub card_color: Tint,
    pub favorite_color: Option<Color>,
    pub fortune: Option<Fortune>,
    pub favorite_quote: Option<&'static str>,
    pub sin: Option<Sin>,
    pub delicious: Option<&'static str>,
    pub has_seen_the_sky: bool,
    pub voice: &'static str,
}

const GLUB: &str = "glub";
const MIAU: &str = "miau";

pub const ORDINARY_FLAVOUR: Flavour = Flavour {
    card_color: Tint::Plain,
    favorite_color: None,
    fortune: None,
    favorite_quote: None,
    sin: None,
    delicious: None,
    has_seen_the_sky: false,
    voice: GLUB,
};

const MUTANT_FLAVOUR: Flavour = Flavour {
    card_color: Tint::Body,
    favorite_color: None,
    fortune: Some(Fortune::Doomed),
    favorite_quote: Some("OOGHHHHHHH"),
    sin: Some(Sin::Wrath),
    delicious: Some("NOOOOOOOOOO"),
    has_seen_the_sky: false,
    voice: GLUB,
};

const CASH_FLAVOUR: Flavour = Flavour {
    card_color: Tint::Fixed(LIGHT_RED),
    favorite_color: Some(GOLD),
    fortune: Some(Fortune::Golden),
    favorite_quote: Some("Gonna be, gonna be golden"),
    sin: Some(Sin::Greed),
    delicious: Some("Yes."),
    has_seen_the_sky: true,
    voice: GLUB,
};

const HOLY_FLAVOUR: Flavour = Flavour {
    card_color: Tint::Fixed(LIGHT_YELLOW),
    favorite_color: Some(LIGHT_YELLOW),
    fortune: Some(Fortune::Golden),
    favorite_quote: Some("Blessed be the deep, glub"),
    sin: Some(Sin::Lust),
    delicious: Some("Forbidden"),
    has_seen_the_sky: true,
    voice: GLUB,
};

const SKY_FLAVOUR: Flavour = Flavour {
    has_seen_the_sky: true,
    ..ORDINARY_FLAVOUR
};

const CAT_FLAVOUR: Flavour = Flavour {
    voice: MIAU,
    ..ORDINARY_FLAVOUR
};

const CANDY_FLAVOUR: Flavour = Flavour {
    card_color: Tint::Fixed(PINK),
    sin: Some(Sin::Gluttony),
    ..ORDINARY_FLAVOUR
};

const CHEAT_FLAVOUR: Flavour = Flavour {
    sin: Some(Sin::Pride),
    ..ORDINARY_FLAVOUR
};

const BOT_FLAVOUR: Flavour = Flavour {
    sin: Some(Sin::Sloth),
    ..ORDINARY_FLAVOUR
};

#[derive(Debug, Clone, Copy)]
pub struct SpeciesConfig {
    pub name: &'static str,
    pub body: BodyTemplate,
    pub body_source: BodySource,
    pub body_fill: BodyFill,
    pub eye_at: EyeAt,
    pub palette: &'static [Color],
    pub pattern: PatternKind,
    pub sway_speed: f32,
    pub speed_range: (f32, f32),
    pub rarity: Rarity,
    pub buyable: bool,
    pub habitat: Habitat,
    pub abductable: bool,
    pub mutatable: bool,
    pub markable: bool,
    pub sellable: bool,
    pub eyes: usize,
    pub auto_glisten: bool,
    pub auto_mutate: bool,
    pub programmable: bool,
    pub zoomie: Zoomie,
    pub locomotion: Locomotion,
    pub habit: Option<Habit>,
    pub skin: Skin,
    pub trail: Option<Tint>,
    pub keepsake: Option<StockItem>,
    pub born_with: &'static [Mutation],
    pub eye_color: Option<Color>,
    pub zoomie_bubble_color: Tint,
    pub flavour: Flavour,
    pub appraisal: Option<Appraisal>,
    pub sizes: [usize; 4],
    pub weight_base: [u32; 4],
    pub weight_cap: [u32; 4],
    pub sell_base: [u32; 4],
    pub sell_cap: [u32; 4],
}

pub const MUTANT_SELL_BASE: u32 = 500;
pub const MUTANT_SELL_WEIGHT_DIVISOR: u32 = 10;

const COMMON_SIZES: [usize; 4] = [3, 5, 7, 9];
const RARE_SIZES: [usize; 4] = [4, 6, 8, 10];
const LEGENDARY_SIZES: [usize; 4] = [5, 7, 9, 11];

const STD_WEIGHT_BASE: [u32; 4] = [100, 250, 500, 1_000];
const STD_WEIGHT_CAP: [u32; 4] = [2_500, 7_500, 20_000, 40_000];

const COMMON_SELL_BASE: [u32; 4] = [12, 27, 65, 175];
const RARE_SELL_BASE: [u32; 4] = [30, 75, 175, 500];
const LEGENDARY_SELL_BASE: [u32; 4] = [200, 800, 2_000, 3_500];
const SMALLEST: usize = SizeCategory::S as usize;

pub fn pellets_to_cap() -> [u32; 4] {
    std::array::from_fn(|size| (STD_WEIGHT_CAP[size] - STD_WEIGHT_BASE[size]) / FOOD_WEIGHT_GAIN_G)
}

pub fn fattening_profit(rarity: Rarity, size: SizeCategory) -> u32 {
    let pellets = pellets_to_cap();
    let reach = (pellets[SMALLEST] * pellets[size as usize]) as f32;
    (rarity.value_multiplier() * reach.sqrt()).round() as u32
}

pub fn fed_catch_worth(rarity: Rarity) -> u32 {
    let pellets = pellets_to_cap();
    let caps = rarity_arrays(rarity).2;
    let expected: u32 = SizeCategory::ALL
        .into_iter()
        .map(|size| {
            let i = size as usize;
            size.odds() * (caps[i] - pellets[i] * FOOD_BUY_PRICE)
        })
        .sum();
    (expected + SizeCategory::ODDS_TOTAL / 2) / SizeCategory::ODDS_TOTAL
}

fn fattened_caps(rarity: Rarity, sell_base: [u32; 4]) -> [u32; 4] {
    let pellets = pellets_to_cap();
    std::array::from_fn(|size| {
        sell_base[size]
            + pellets[size] * FOOD_BUY_PRICE
            + fattening_profit(rarity, SizeCategory::ALL[size])
    })
}

pub const ALL_SPECIES: &[FishSpecies] = &[
    FishSpecies::Merluza,
    FishSpecies::Betta,
    FishSpecies::Salmon,
    FishSpecies::Chromis,
    FishSpecies::Tang,
    FishSpecies::Koi,
    FishSpecies::Carpin,
    FishSpecies::Turbofish,
    FishSpecies::Deadfish,
    FishSpecies::Anchoveta,
    FishSpecies::Jellyfish,
    FishSpecies::Cashfish,
    FishSpecies::Goldfish,
    FishSpecies::Snapper,
    FishSpecies::Mutantfish,
    FishSpecies::Nishiki,
    FishSpecies::Aka,
    FishSpecies::Kuro,
    FishSpecies::Caracol,
    FishSpecies::Babosa,
    FishSpecies::Estrella,
    FishSpecies::Ermitano,
    FishSpecies::Lenguado,
    FishSpecies::Piedra,
    FishSpecies::Pejesapo,
    FishSpecies::Cofre,
    FishSpecies::Caballito,
    FishSpecies::Morena,
    FishSpecies::Volador,
    FishSpecies::Espada,
    FishSpecies::Tollo,
    FishSpecies::Mariposa,
    FishSpecies::Linterna,
    FishSpecies::Luciernaga,
    FishSpecies::Bagre,
    FishSpecies::Luna,
    FishSpecies::Loro,
    FishSpecies::Mimo,
    FishSpecies::Timido,
    FishSpecies::Ciego,
    FishSpecies::Globo,
    FishSpecies::Pulpo,
    FishSpecies::Draco,
    FishSpecies::Neon,
    FishSpecies::Martillo,
    FishSpecies::Candyfish,
    FishSpecies::Holyfish,
    FishSpecies::Botfish,
    FishSpecies::Cheatfish,
    FishSpecies::Junkfish,
];

static BUYABLE_SPECIES: LazyLock<Vec<FishSpecies>> = LazyLock::new(|| {
    ALL_SPECIES
        .iter()
        .copied()
        .filter(|species| species.config().buyable)
        .collect()
});

static WILD_SPECIES: LazyLock<Vec<FishSpecies>> = LazyLock::new(|| {
    ALL_SPECIES
        .iter()
        .copied()
        .filter(|species| species.config().habitat == Habitat::Everywhere)
        .collect()
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodySource {
    Species,
    MutantState,
    UnfishState,
}

#[derive(Debug, Clone, Copy)]
pub enum BodyTemplate {
    Standard(BodyChars),
    Alternating(BodyChars, BodyChars),
    Fixed {
        left: &'static [&'static str],
        right: &'static [&'static str],
    },
    Figure(&'static Figure),
}

#[derive(Debug, Clone, Copy)]
pub struct BodyPair {
    pub left_body: char,
    pub left_wave: char,
    pub right_body: char,
    pub right_wave: char,
}

pub const BODY_ROUND: BodyPair = BodyPair {
    left_body: '(',
    left_wave: '{',
    right_body: ')',
    right_wave: '}',
};
pub const BODY_CURLY: BodyPair = BodyPair {
    left_body: '}',
    left_wave: ')',
    right_body: '{',
    right_wave: '(',
};

#[derive(Debug, Clone, Copy)]
pub struct BodyChars {
    pub mouth_left: char,
    pub mouth_right: char,
    pub eye_left: char,
    pub eye_right: char,
    pub body_left: char,
    pub wave_left: char,
    pub body_right: char,
    pub wave_right: char,
    pub tail: TailKind,
}

#[derive(Debug, Clone, Copy)]
pub enum TailKind {
    Wide,
    Short,
    Custom { left: char, right: char },
    Swaying { left: char, right: char, wave: char },
    WideCurly,
    None,
}

#[derive(Debug, Clone, Copy)]
pub enum PatternKind {
    Solid,
    Striped,
    Patchy,
    PatchyAll,
    Glistening,
}

const fn standard_with(eye: char, tail: TailKind, body: BodyPair) -> BodyChars {
    BodyChars {
        mouth_left: '<',
        mouth_right: '>',
        eye_left: eye,
        eye_right: eye,
        body_left: body.left_body,
        wave_left: body.left_wave,
        body_right: body.right_body,
        wave_right: body.right_wave,
        tail,
    }
}

const fn standard(eye: char, tail: TailKind) -> BodyChars {
    standard_with(eye, tail, BODY_ROUND)
}

static MERLUZA_PALETTE: [Color; 2] = [GRAY, WHITE];

static BETTA_PALETTE: [Color; 3] = [BLUE, LIGHT_BLUE, CYAN];
static TANG_PALETTE: [Color; 3] = [NAVY_DARK, NAVY, NAVY_LIGHT];
static JELLYFISH_PALETTE: [Color; 3] = [BLUE, CYAN, LIGHT_BLUE];

static CHROMIS_PALETTE: [Color; 3] = [CYAN, LIGHT_CYAN, WHITE];
static ANCHOVETA_PALETTE: [Color; 2] = [CYAN, LIGHT_CYAN];

static SALMON_PALETTE: [Color; 3] = [ORANGE_DARK, ORANGE, ORANGE_LIGHT];
static GOLDFISH_PALETTE: [Color; 3] = [ORANGE, AMBER, AMBER_LIGHT];
static SNAPPER_PALETTE: [Color; 3] = [RED_DARK, RED, LIGHT_RED];
static TURBOFISH_PALETTE: [Color; 2] = [YELLOW, ORANGE];

static CASHFISH_PALETTE: [Color; 1] = [LIGHT_RED];
static HOLYFISH_PALETTE: [Color; 1] = [GRAY];
static BOTFISH_PALETTE: [Color; 1] = [DARK_GRAY];
static CHEATFISH_PALETTE: [Color; 1] = [WHITE];
static JUNKFISH_PALETTE: [Color; 1] = [JUNK_OUTLINE];

static AKA_PALETTE: [Color; 1] = [RED];
static KURO_PALETTE: [Color; 1] = [DARK_GRAY];
static NISHIKI_PALETTE: [Color; 3] = [WHITE, LIGHT_RED, DARK_GRAY];

static KOI_PALETTE: [Color; 5] = [WHITE, WHITE, WHITE, LIGHT_RED, DARK_GRAY];

static CARPIN_PALETTE: [Color; 3] = [YELLOW, LIGHT_YELLOW, AMBER_DARK];

static DEADFISH_PALETTE: [Color; 2] = [WHITE, DARK_GRAY];
static UNFISH_PALETTE: [Color; 1] = [WHITE];
static CANDYFISH_PALETTE: [Color; 3] = [PINK, MAGENTA, LIGHT_MAGENTA];

static CARACOL_PALETTE: [Color; 3] = [OLIVE_LIGHT, BROWN, TAN];
static BABOSA_PALETTE: [Color; 3] = [PURPLE_LIGHT, VIOLET, PINK];
static ESTRELLA_PALETTE: [Color; 4] = [TERRACOTTA, AMBER, PINK, VIOLET];
static ERMITANO_PALETTE: [Color; 3] = [TERRACOTTA, ORANGE_LIGHT, JUNK_OUTLINE];
static LENGUADO_PALETTE: [Color; 1] = [TAN];
static PIEDRA_PALETTE: [Color; 4] = [GRAY, BROWN_DARK, DARK_GRAY, OLIVE];
static PEJESAPO_PALETTE: [Color; 2] = [OLIVE, KHAKI];
static COFRE_PALETTE: [Color; 8] = [
    RED,
    ORANGE,
    YELLOW,
    GREEN_BRIGHT,
    CYAN,
    NAVY_LIGHT,
    VIOLET,
    PINK,
];
static CABALLITO_PALETTE: [Color; 3] = [AMBER, ORANGE_LIGHT, AMBER_DARK];
static MORENA_PALETTE: [Color; 3] = [OLIVE, OLIVE_LIGHT, KHAKI];
static VOLADOR_PALETTE: [Color; 3] = [NAVY_LIGHT, SILVER, LIGHT_BLUE];
static ESPADA_PALETTE: [Color; 3] = [NAVY, SILVER, NAVY_LIGHT];
static TOLLO_PALETTE: [Color; 3] = [GRAY, SILVER, STEEL];
static MARIPOSA_PALETTE: [Color; 3] = [YELLOW, YELLOW, WHITE];
static LINTERNA_PALETTE: [Color; 2] = [DARK_GRAY, BROWN_DARK];
static LUCIERNAGA_PALETTE: [Color; 2] = [DARK_GRAY, GREEN_DARK];
static BAGRE_PALETTE: [Color; 3] = [BROWN_DARK, BROWN, TAN];
static LUNA_PALETTE: [Color; 1] = [SILVER];
static LORO_PALETTE: [Color; 4] = [TEAL, GREEN_BRIGHT, PINK, NAVY_LIGHT];
static MIMO_PALETTE: [Color; 2] = [WHITE, DARK_GRAY];
static TIMIDO_PALETTE: [Color; 2] = [CREAM, PINK];
static CIEGO_PALETTE: [Color; 1] = [CREAM];
static GLOBO_PALETTE: [Color; 3] = [KHAKI, TAN, CREAM];
static PULPO_PALETTE: [Color; 4] = [TERRACOTTA, VIOLET, OLIVE_LIGHT, PINK];
static DRACO_PALETTE: [Color; 3] = [LIGHT_CYAN, SILVER, WHITE];
static NEON_PALETTE: [Color; 2] = [CYAN, LIGHT_RED];
static MARTILLO_PALETTE: [Color; 3] = [STEEL, SILVER, GRAY];

static ESTRELLA_LR: [&str; 1] = ["✶"];
const NO_EYES: usize = 0;
const TWO_EYES: usize = 2;
const MORENA_SIZES: [usize; 4] = [8, 12, 16, 20];

pub const DEADFISH_BC_SEMI: BodyChars = BodyChars {
    mouth_left: '<',
    mouth_right: '>',
    eye_left: EYE_DEAD,
    eye_right: 'C',
    body_left: ';',
    wave_left: ':',
    body_right: ';',
    wave_right: ':',
    tail: TailKind::Wide,
};

pub const DEADFISH_BC_PLUS: BodyChars = BodyChars {
    mouth_left: '<',
    mouth_right: '>',
    eye_left: EYE_DEAD,
    eye_right: 'C',
    body_left: '+',
    wave_left: '-',
    body_right: '+',
    wave_right: '-',
    tail: TailKind::Wide,
};

pub static MUTANT_GREEN_PALETTE: [Color; 3] = [FOREST, GREEN_LIGHT, GREEN_BRIGHT];
pub static MUTANT_PURPLE_PALETTE: [Color; 3] = [PURPLE_LIGHT, PURPLE, VIOLET];
pub static MUTANT_WHITE_PALETTE: [Color; 3] = [GRAY, SILVER, WHITE];

static ANCHOVETA_L: [&str; 1] = ["<><"];
static ANCHOVETA_R: [&str; 1] = ["><>"];
static JELLYFISH_LR: [&str; 1] = ["ଳ"];
static BOTFISH_L: [&str; 1] = ["-º]]]]]-]"];
static BOTFISH_R: [&str; 1] = ["[-[[[[[º-"];

fn rarity_arrays(rarity: Rarity) -> ([usize; 4], [u32; 4], [u32; 4]) {
    let (sizes, sell_base) = match rarity {
        Rarity::Common => (COMMON_SIZES, COMMON_SELL_BASE),
        Rarity::Rare => (RARE_SIZES, RARE_SELL_BASE),
        Rarity::Legendary => (LEGENDARY_SIZES, LEGENDARY_SELL_BASE),
    };
    (sizes, sell_base, fattened_caps(rarity, sell_base))
}

fn standard_config(
    name: &'static str,
    body: BodyChars,
    palette: &'static [Color],
    pattern: PatternKind,
    sway_speed: f32,
    speed_range: (f32, f32),
    rarity: Rarity,
) -> SpeciesConfig {
    let (sizes, sell_base, sell_cap) = rarity_arrays(rarity);
    SpeciesConfig {
        name,
        body: BodyTemplate::Standard(body),
        body_source: BodySource::Species,
        body_fill: BodyFill::Species,
        eye_at: EyeAt::Head,
        palette,
        pattern,
        sway_speed,
        speed_range,
        rarity,
        buyable: true,
        habitat: Habitat::Everywhere,
        abductable: true,
        mutatable: true,
        markable: true,
        sellable: true,
        eyes: SINGLE_EYE,
        auto_glisten: false,
        auto_mutate: false,
        programmable: false,
        zoomie: Zoomie::Burst,
        locomotion: Locomotion::Swim,
        habit: None,
        skin: Skin::Palette,
        trail: None,
        keepsake: None,
        born_with: &[],
        eye_color: None,
        zoomie_bubble_color: Tint::Plain,
        flavour: ORDINARY_FLAVOUR,
        appraisal: None,
        sizes,
        weight_base: STD_WEIGHT_BASE,
        weight_cap: STD_WEIGHT_CAP,
        sell_base,
        sell_cap,
    }
}

fn fixed_config(
    name: &'static str,
    left: &'static [&'static str],
    right: &'static [&'static str],
    palette: &'static [Color],
    pattern: PatternKind,
    speed_range: (f32, f32),
    rarity: Rarity,
) -> SpeciesConfig {
    let (sizes, sell_base, sell_cap) = rarity_arrays(rarity);
    SpeciesConfig {
        name,
        body: BodyTemplate::Fixed { left, right },
        body_source: BodySource::Species,
        body_fill: BodyFill::Species,
        eye_at: EyeAt::Head,
        palette,
        pattern,
        sway_speed: 0.0,
        speed_range,
        rarity,
        buyable: true,
        habitat: Habitat::Everywhere,
        abductable: true,
        mutatable: true,
        markable: true,
        sellable: true,
        eyes: SINGLE_EYE,
        auto_glisten: false,
        auto_mutate: false,
        programmable: false,
        zoomie: Zoomie::Burst,
        locomotion: Locomotion::Swim,
        habit: None,
        skin: Skin::Palette,
        trail: None,
        keepsake: None,
        born_with: &[],
        eye_color: None,
        zoomie_bubble_color: Tint::Plain,
        flavour: ORDINARY_FLAVOUR,
        appraisal: None,
        sizes,
        weight_base: STD_WEIGHT_BASE,
        weight_cap: STD_WEIGHT_CAP,
        sell_base,
        sell_cap,
    }
}

fn figure_config(
    name: &'static str,
    figure: &'static Figure,
    palette: &'static [Color],
    speed_range: (f32, f32),
    rarity: Rarity,
) -> SpeciesConfig {
    let mut config = fixed_config(
        name,
        &[],
        &[],
        palette,
        PatternKind::Solid,
        speed_range,
        rarity,
    );
    config.body = BodyTemplate::Figure(figure);
    config.sway_speed = FIGURE_SWAY_SPEED;
    config
}

const FIGURE_SWAY_SPEED: f32 = 0.08;

const fn chars(
    mouth: (char, char),
    eye: char,
    body: (char, char),
    wave: (char, char),
    tail: TailKind,
) -> BodyChars {
    BodyChars {
        mouth_left: mouth.0,
        mouth_right: mouth.1,
        eye_left: eye,
        eye_right: eye,
        body_left: body.0,
        wave_left: wave.0,
        body_right: body.1,
        wave_right: wave.1,
        tail,
    }
}

const MOUTH: (char, char) = ('<', '>');
const ROUND: (char, char) = ('(', ')');
const CURLED: (char, char) = ('{', '}');

impl FishSpecies {
    pub fn config(self) -> SpeciesConfig {
        use FishSpecies::*;
        use PatternKind::*;
        use Rarity::*;
        match self {
            Merluza => standard_config(
                "Merluza",
                standard(EYE_ROUND, TailKind::Wide),
                &MERLUZA_PALETTE,
                Solid,
                0.10,
                (2.5, 4.0),
                Common,
            ),
            Betta => standard_config(
                "Betta",
                standard('\'', TailKind::Wide),
                &BETTA_PALETTE,
                Striped,
                0.09,
                (2.0, 3.5),
                Common,
            ),
            Salmon => standard_config(
                "Salmon",
                standard('*', TailKind::Wide),
                &SALMON_PALETTE,
                Striped,
                0.10,
                (4.0, 6.0),
                Common,
            ),
            Chromis => standard_config(
                "Chromis",
                standard(EYE_ROUND, TailKind::Short),
                &CHROMIS_PALETTE,
                Striped,
                0.13,
                (3.0, 5.0),
                Common,
            ),
            Tang => standard_config(
                "Tang",
                standard('\'', TailKind::Wide),
                &TANG_PALETTE,
                Striped,
                0.09,
                (2.5, 4.0),
                Common,
            ),
            Goldfish => standard_config(
                "Goldfish",
                standard(EYE_ROUND, TailKind::WideCurly),
                &GOLDFISH_PALETTE,
                Striped,
                0.08,
                (1.5, 3.0),
                Common,
            ),
            Snapper => standard_config(
                "Snapper",
                standard(EYE_ROUND, TailKind::Wide),
                &SNAPPER_PALETTE,
                Striped,
                0.10,
                (2.5, 4.0),
                Common,
            ),
            Nishiki => standard_config(
                "Nishiki",
                standard(EYE_ROUND, TailKind::WideCurly),
                &NISHIKI_PALETTE,
                PatchyAll,
                0.08,
                (1.5, 3.0),
                Common,
            ),
            Aka => standard_config(
                "Aka",
                standard(EYE_ROUND, TailKind::WideCurly),
                &AKA_PALETTE,
                Solid,
                0.09,
                (2.0, 3.5),
                Common,
            ),
            Kuro => standard_config(
                "Kuro",
                standard(EYE_ROUND, TailKind::WideCurly),
                &KURO_PALETTE,
                Solid,
                0.07,
                (1.5, 3.0),
                Common,
            ),
            Deadfish => {
                let (sizes, sell_base, sell_cap) = rarity_arrays(Rare);
                SpeciesConfig {
                    name: "Deadfish",
                    body: BodyTemplate::Alternating(DEADFISH_BC_SEMI, DEADFISH_BC_PLUS),
                    body_source: BodySource::Species,
                    body_fill: BodyFill::Species,
                    eye_at: EyeAt::Head,
                    palette: &DEADFISH_PALETTE,
                    pattern: Solid,
                    sway_speed: 0.04,
                    speed_range: (1.0, 2.5),
                    rarity: Rare,
                    buyable: true,
                    habitat: Habitat::Everywhere,
                    abductable: true,
                    mutatable: true,
                    markable: true,
                    sellable: true,
                    eyes: SINGLE_EYE,
                    auto_glisten: false,
                    auto_mutate: false,
                    programmable: false,
                    zoomie: Zoomie::Burst,
                    locomotion: Locomotion::Swim,
                    habit: None,
                    skin: Skin::Palette,
                    trail: None,
                    keepsake: None,
                    born_with: &[],
                    eye_color: None,
                    zoomie_bubble_color: Tint::Plain,
                    flavour: ORDINARY_FLAVOUR,
                    appraisal: None,
                    sizes,
                    weight_base: STD_WEIGHT_BASE,
                    weight_cap: STD_WEIGHT_CAP,
                    sell_base,
                    sell_cap,
                }
            }
            Anchoveta => fixed_config(
                "Anchoveta",
                &ANCHOVETA_L,
                &ANCHOVETA_R,
                &ANCHOVETA_PALETTE,
                Solid,
                (5.0, 7.0),
                Common,
            ),
            Jellyfish => {
                let mut config = fixed_config(
                    "Jellyfish",
                    &JELLYFISH_LR,
                    &JELLYFISH_LR,
                    &JELLYFISH_PALETTE,
                    Solid,
                    (1.5, 3.0),
                    Rare,
                );
                config.zoomie = Zoomie::Vertical;
                config
            }
            Turbofish => {
                let mut config = standard_config(
                    "Turbofish",
                    BodyChars {
                        mouth_left: '<',
                        mouth_right: '>',
                        eye_left: '>',
                        eye_right: '<',
                        body_left: ':',
                        wave_left: '~',
                        body_right: ':',
                        wave_right: '~',
                        tail: TailKind::None,
                    },
                    &TURBOFISH_PALETTE,
                    Striped,
                    0.18,
                    (6.0, 9.0),
                    Rare,
                );
                config.sizes = [2, 2, 2, 2];
                config
            }
            Koi => standard_config(
                "Koi",
                BodyChars {
                    mouth_left: '>',
                    mouth_right: '<',
                    eye_left: EYE_ROUND,
                    eye_right: EYE_ROUND,
                    body_left: BODY_CURLY.left_body,
                    wave_left: BODY_CURLY.left_wave,
                    body_right: BODY_CURLY.right_body,
                    wave_right: BODY_CURLY.right_wave,
                    tail: TailKind::Swaying {
                        left: TAIL_WAVE_LEFT,
                        right: TAIL_WAVE_RIGHT,
                        wave: TAIL_EQUAL,
                    },
                },
                &KOI_PALETTE,
                Patchy,
                0.07,
                (1.5, 3.0),
                Rare,
            ),
            Carpin => standard_config(
                "Carpin",
                standard_with(
                    EYE_CIRCLE,
                    TailKind::Custom {
                        left: '(',
                        right: ')',
                    },
                    BODY_CURLY,
                ),
                &CARPIN_PALETTE,
                Patchy,
                0.09,
                (2.0, 3.5),
                Common,
            ),
            Cashfish => {
                let mut config = standard_config(
                    "Cashfish",
                    standard(EYE_ROUND, TailKind::WideCurly),
                    &CASHFISH_PALETTE,
                    Solid,
                    0.20,
                    (2.0, 3.5),
                    Legendary,
                );
                config.buyable = false;
                config.habitat = Habitat::Native(TankKind::Hell);
                config.eye_color = Some(LIGHT_YELLOW);
                config.flavour = CASH_FLAVOUR;
                config
            }
            Holyfish => {
                let mut config = standard_config(
                    "Holyfish",
                    standard(EYE_ROUND, TailKind::Wide),
                    &HOLYFISH_PALETTE,
                    Solid,
                    0.12,
                    (1.5, 3.0),
                    Legendary,
                );
                config.buyable = false;
                config.habitat = Habitat::Native(TankKind::Heaven);
                config.mutatable = false;
                config.markable = false;
                config.eye_color = Some(LIGHT_YELLOW);
                config.flavour = HOLY_FLAVOUR;
                config
            }
            Botfish => {
                let mut config = fixed_config(
                    "Botfish",
                    &BOTFISH_L,
                    &BOTFISH_R,
                    &BOTFISH_PALETTE,
                    Solid,
                    (1.0, 2.0),
                    Legendary,
                );
                config.buyable = false;
                config.habitat = Habitat::Native(TankKind::Matrix);
                config.abductable = false;
                config.markable = false;
                config.programmable = true;
                config.flavour = BOT_FLAVOUR;
                config
            }
            Cheatfish => {
                let mut config = standard_config(
                    "Cheatfish",
                    standard(EYE_ROUND, TailKind::Wide),
                    &CHEATFISH_PALETTE,
                    Solid,
                    0.10,
                    (2.5, 4.0),
                    Common,
                );
                config.buyable = false;
                config.habitat = Habitat::Nowhere;
                config.sellable = false;
                config.eyes = CHEATFISH_EYES;
                config.eye_color = Some(RED);
                config.flavour = CHEAT_FLAVOUR;
                config
            }
            Junkfish => {
                let mut config = standard_config(
                    "Junkfish",
                    standard(EYE_ROUND, TailKind::Wide),
                    &JUNKFISH_PALETTE,
                    Solid,
                    0.10,
                    (2.0, 3.5),
                    Legendary,
                );
                config.buyable = false;
                config.habitat = Habitat::Junkpile;
                config.body_fill = BodyFill::Junk;
                config.appraisal = Some(JUNKFISH_APPRAISAL);
                config
            }
            Mutantfish => SpeciesConfig {
                name: "Mutantfish",
                body: BodyTemplate::Standard(standard(EYE_CIRCLE, TailKind::Wide)),
                body_source: BodySource::MutantState,
                body_fill: BodyFill::Species,
                eye_at: EyeAt::Head,
                palette: &MUTANT_GREEN_PALETTE,
                pattern: Solid,
                sway_speed: 0.11,
                speed_range: (2.0, 5.0),
                rarity: Legendary,
                buyable: false,
                habitat: Habitat::Native(TankKind::Rad),
                abductable: true,
                mutatable: true,
                markable: true,
                sellable: true,
                eyes: SINGLE_EYE,
                auto_glisten: true,
                auto_mutate: true,
                programmable: false,
                zoomie: Zoomie::Burst,
                locomotion: Locomotion::Swim,
                habit: None,
                skin: Skin::Palette,
                trail: None,
                keepsake: None,
                born_with: &[],
                eye_color: None,
                zoomie_bubble_color: Tint::Body,
                flavour: MUTANT_FLAVOUR,
                appraisal: None,
                sizes: LEGENDARY_SIZES,
                weight_base: [0, 250, 0, 0],
                weight_cap: [0; 4],
                sell_base: [0; 4],
                sell_cap: [0; 4],
            },
            Candyfish => {
                let mut config = standard_config(
                    "Candyfish",
                    standard(EYE_ROUND, TailKind::Wide),
                    &CANDYFISH_PALETTE,
                    Solid,
                    0.10,
                    (1.5, 3.0),
                    Legendary,
                );
                config.buyable = false;
                config.habitat = Habitat::Native(TankKind::Candy);
                config.zoomie_bubble_color = Tint::Fixed(PINK);
                config.flavour = CANDY_FLAVOUR;
                config
            }
            Caracol => {
                let mut config =
                    figure_config("Caracol", &CARACOL, &CARACOL_PALETTE, (0.3, 0.6), Rare);
                config.locomotion = Locomotion::Glass;
                config.zoomie = Zoomie::Still;
                config.trail = Some(Tint::Fixed(GRAY));
                config
            }
            Babosa => {
                let mut config = standard_config(
                    "Babosa",
                    chars(
                        ('"', '"'),
                        EYE_ROUND,
                        ('~', '~'),
                        ('≈', '≈'),
                        TailKind::Custom {
                            left: '*',
                            right: '*',
                        },
                    ),
                    &BABOSA_PALETTE,
                    Glistening,
                    0.12,
                    (0.4, 0.8),
                    Rare,
                );
                config.locomotion = Locomotion::Glass;
                config.zoomie = Zoomie::None;
                config.trail = Some(Tint::Body);
                config
            }
            Estrella => {
                let mut config = fixed_config(
                    "Estrella",
                    &ESTRELLA_LR,
                    &ESTRELLA_LR,
                    &ESTRELLA_PALETTE,
                    Solid,
                    (0.05, 0.15),
                    Rare,
                );
                config.locomotion = Locomotion::Glass;
                config.zoomie = Zoomie::None;
                config.habit = Some(Habit::Twinkle);
                config.flavour = SKY_FLAVOUR;
                config
            }
            Ermitano => {
                let mut config =
                    figure_config("Ermitano", &ERMITANO, &ERMITANO_PALETTE, (0.8, 1.6), Rare);
                config.locomotion = Locomotion::Sideways;
                config.zoomie = Zoomie::None;
                config.habit = Some(Habit::ShellSwap);
                config.body_fill = BodyFill::Junk;
                config.keepsake = Some(StockItem::Junk);
                config.eye_color = Some(WHITE);
                config
            }
            Lenguado => {
                let mut config = standard_config(
                    "Lenguado",
                    chars(MOUTH, EYE_ROUND, ('=', '='), ('-', '-'), TailKind::Short),
                    &LENGUADO_PALETTE,
                    Solid,
                    0.06,
                    (1.0, 2.0),
                    Rare,
                );
                config.eyes = TWO_EYES;
                config.eye_color = Some(WHITE);
                config.locomotion = Locomotion::Floor;
                config.skin = Skin::Camouflage;
                config
            }
            Piedra => {
                let mut config = standard_config(
                    "Piedra",
                    chars(
                        ('.', '.'),
                        EYE_ROUND,
                        ('O', 'O'),
                        ('o', 'o'),
                        TailKind::Custom {
                            left: '.',
                            right: '.',
                        },
                    ),
                    &PIEDRA_PALETTE,
                    Patchy,
                    0.03,
                    (0.1, 0.2),
                    Rare,
                );
                config.eye_color = Some(AMBER);
                config.locomotion = Locomotion::Floor;
                config.zoomie = Zoomie::None;
                config.habit = Some(Habit::Ambush);
                config.body_fill = BodyFill::Stones;
                config.eye_at = EyeAt::Middle;
                config
            }
            Pejesapo => {
                let mut config = standard_config(
                    "Pejesapo",
                    chars(
                        MOUTH,
                        EYE_CIRCLE,
                        ('o', 'o'),
                        ('O', 'O'),
                        TailKind::Custom {
                            left: ')',
                            right: '(',
                        },
                    ),
                    &PEJESAPO_PALETTE,
                    Patchy,
                    0.08,
                    (0.6, 1.2),
                    Rare,
                );
                config.eye_color = Some(AMBER);
                config.locomotion = Locomotion::Floor;
                config.zoomie = Zoomie::Hop;
                config.born_with = &[Mutation::Feet];
                config
            }
            Cofre => {
                let mut config = figure_config("Cofre", &COFRE, &COFRE_PALETTE, (2.0, 3.0), Rare);
                config.locomotion = Locomotion::Bounce;
                config.zoomie = Zoomie::None;
                config.skin = Skin::Cycle(Cycle::OnBounce);
                config
            }
            Caballito => {
                let mut config = figure_config(
                    "Caballito",
                    &CABALLITO,
                    &CABALLITO_PALETTE,
                    (0.5, 1.2),
                    Rare,
                );
                config.zoomie = Zoomie::Vertical;
                config.habit = Some(Habit::Pair);
                config
            }
            Morena => {
                let mut config = standard_config(
                    "Morena",
                    chars(
                        MOUTH,
                        EYE_CIRCLE,
                        ('≈', '≈'),
                        ('~', '~'),
                        TailKind::Custom {
                            left: '-',
                            right: '-',
                        },
                    ),
                    &MORENA_PALETTE,
                    Patchy,
                    0.09,
                    (1.5, 3.0),
                    Rare,
                );
                config.sizes = MORENA_SIZES;
                config.locomotion = Locomotion::Through;
                config.habit = Some(Habit::Gape);
                config
            }
            Volador => {
                let mut config = standard_config(
                    "Volador",
                    chars(MOUTH, EYE_ROUND, ROUND, CURLED, TailKind::Wide),
                    &VOLADOR_PALETTE,
                    Striped,
                    0.12,
                    (4.0, 6.5),
                    Rare,
                );
                config.zoomie = Zoomie::Glide;
                config.flavour = SKY_FLAVOUR;
                config
            }
            Espada => {
                let mut config = standard_config(
                    "Espada",
                    chars(MOUTH, EYE_ROUND, ROUND, CURLED, TailKind::Wide),
                    &ESPADA_PALETTE,
                    Striped,
                    0.14,
                    (5.0, 8.0),
                    Rare,
                );
                config.zoomie = Zoomie::Lunge;
                config.habit = Some(Habit::Duel);
                config.born_with = &[Mutation::Bill];
                config
            }
            Tollo => {
                let mut config = standard_config(
                    "Tollo",
                    chars(MOUTH, EYE_ROUND, ROUND, CURLED, TailKind::Wide),
                    &TOLLO_PALETTE,
                    Striped,
                    0.11,
                    (3.0, 5.0),
                    Rare,
                );
                config.habit = Some(Habit::Chase);
                config.born_with = &[Mutation::DorsalFin];
                config
            }
            Mariposa => {
                let mut config = standard_config(
                    "Mariposa",
                    chars(
                        MOUTH,
                        EYE_ROUND,
                        ROUND,
                        CURLED,
                        TailKind::Custom {
                            left: EYE_ROUND,
                            right: EYE_ROUND,
                        },
                    ),
                    &MARIPOSA_PALETTE,
                    Striped,
                    0.13,
                    (2.0, 3.5),
                    Rare,
                );
                config.eye_color = Some(DARK_GRAY);
                config.habit = Some(Habit::Backwards);
                config
            }
            Linterna => {
                let mut config = standard_config(
                    "Linterna",
                    chars(MOUTH, EYE_CIRCLE, ROUND, CURLED, TailKind::Wide),
                    &LINTERNA_PALETTE,
                    Patchy,
                    0.08,
                    (1.0, 2.0),
                    Rare,
                );
                config.eye_color = Some(WHITE);
                config.born_with = &[Mutation::Lure];
                config
            }
            Luciernaga => {
                let mut config = standard_config(
                    "Luciernaga",
                    chars(MOUTH, '·', ROUND, CURLED, TailKind::Short),
                    &LUCIERNAGA_PALETTE,
                    Striped,
                    0.10,
                    (1.5, 3.0),
                    Rare,
                );
                config.habit = Some(Habit::Sync);
                config
            }
            Bagre => {
                let mut config = standard_config(
                    "Bagre",
                    chars(('=', '='), EYE_ROUND, ROUND, CURLED, TailKind::Wide),
                    &BAGRE_PALETTE,
                    Striped,
                    0.08,
                    (1.5, 3.0),
                    Rare,
                );
                config.born_with = &[Mutation::NightOwl];
                config.flavour = CAT_FLAVOUR;
                config
            }
            Luna => {
                let mut config = standard_config(
                    "Luna",
                    chars(
                        MOUTH,
                        EYE_ROUND,
                        ('o', 'o'),
                        ('O', 'O'),
                        TailKind::Custom {
                            left: ')',
                            right: '(',
                        },
                    ),
                    &LUNA_PALETTE,
                    Solid,
                    0.05,
                    (0.5, 1.2),
                    Rare,
                );
                config.born_with = &[Mutation::DorsalFin, Mutation::VentralFin, Mutation::Lunar];
                config
            }
            Loro => {
                let mut config = standard_config(
                    "Loro",
                    chars(
                        MOUTH,
                        EYE_ROUND,
                        ('}', '{'),
                        (')', '('),
                        TailKind::WideCurly,
                    ),
                    &LORO_PALETTE,
                    Patchy,
                    0.10,
                    (2.0, 3.5),
                    Rare,
                );
                config.habit = Some(Habit::Echo);
                config
            }
            Mimo => {
                let mut config = standard_config(
                    "Mimo",
                    chars(MOUTH, EYE_ROUND, ROUND, CURLED, TailKind::Wide),
                    &MIMO_PALETTE,
                    Striped,
                    0.10,
                    (2.5, 4.0),
                    Rare,
                );
                config.habit = Some(Habit::Shadow);
                config
            }
            Timido => {
                let mut config = standard_config(
                    "Timido",
                    chars(MOUTH, EYE_ROUND, ROUND, CURLED, TailKind::Short),
                    &TIMIDO_PALETTE,
                    Striped,
                    0.10,
                    (2.0, 3.5),
                    Rare,
                );
                config.habit = Some(Habit::Shy);
                config
            }
            Ciego => {
                let mut config = standard_config(
                    "Ciego",
                    chars(MOUTH, EYE_ROUND, ROUND, CURLED, TailKind::Wide),
                    &CIEGO_PALETTE,
                    Solid,
                    0.10,
                    (2.0, 3.5),
                    Rare,
                );
                config.eyes = NO_EYES;
                config.habit = Some(Habit::Blind);
                config
            }
            Globo => {
                let mut config = standard_config(
                    "Globo",
                    chars(MOUTH, EYE_ROUND, ('o', 'o'), ('O', 'O'), TailKind::Short),
                    &GLOBO_PALETTE,
                    Patchy,
                    0.09,
                    (1.5, 3.0),
                    Rare,
                );
                config.born_with = &[Mutation::Puff];
                config
            }
            Pulpo => {
                let mut config = figure_config("Pulpo", &PULPO, &PULPO_PALETTE, (1.0, 2.5), Rare);
                config.eye_color = Some(WHITE);
                config.zoomie = Zoomie::Ink;
                config.skin = Skin::Cycle(Cycle::OnClock);
                config.habit = Some(Habit::Escape);
                config
            }
            Draco => {
                let mut config = standard_config(
                    "Draco",
                    chars(MOUTH, EYE_ROUND, ROUND, CURLED, TailKind::Wide),
                    &DRACO_PALETTE,
                    Glistening,
                    0.10,
                    (2.0, 3.5),
                    Rare,
                );
                config.skin = Skin::SeeThrough;
                config
            }
            Neon => figure_config("Neon", &NEON, &NEON_PALETTE, (3.0, 5.0), Rare),
            Martillo => {
                let mut config =
                    figure_config("Martillo", &MARTILLO, &MARTILLO_PALETTE, (2.5, 4.0), Rare);
                config.eye_color = Some(WHITE);
                config
            }
            Unfish => SpeciesConfig {
                name: "Unfish",
                body: BodyTemplate::Standard(standard(EYE_ROUND, TailKind::Wide)),
                body_source: BodySource::UnfishState,
                body_fill: BodyFill::Species,
                eye_at: EyeAt::Head,
                palette: &UNFISH_PALETTE,
                pattern: Solid,
                sway_speed: 0.10,
                speed_range: (2.0, 4.0),
                rarity: Common,
                buyable: false,
                habitat: Habitat::Nowhere,
                abductable: true,
                mutatable: true,
                markable: true,
                sellable: true,
                eyes: SINGLE_EYE,
                auto_glisten: false,
                auto_mutate: false,
                programmable: false,
                zoomie: Zoomie::None,
                locomotion: Locomotion::Swim,
                habit: None,
                skin: Skin::Palette,
                trail: None,
                keepsake: None,
                born_with: &[],
                eye_color: None,
                zoomie_bubble_color: Tint::Plain,
                flavour: ORDINARY_FLAVOUR,
                appraisal: None,
                sizes: COMMON_SIZES,
                weight_base: [1; 4],
                weight_cap: [0; 4],
                sell_base: [0; 4],
                sell_cap: [0; 4],
            },
        }
    }

    pub fn sell_value(self, weight_g: u32, size_cat: SizeCategory) -> u32 {
        let config = self.config();
        if config.auto_mutate {
            return MUTANT_SELL_BASE.saturating_add(weight_g / MUTANT_SELL_WEIGHT_DIVISOR);
        }
        let i = size_cat as usize;
        let (weight_base, weight_cap, sell_base, sell_cap) = (
            config.weight_base[i],
            config.weight_cap[i],
            config.sell_base[i],
            config.sell_cap[i],
        );
        if weight_cap == 0 || weight_g <= weight_base {
            return sell_base;
        }
        let frac = (weight_g - weight_base).min(weight_cap - weight_base) as f32
            / (weight_cap - weight_base) as f32;
        sell_base + (frac * (sell_cap - sell_base) as f32) as u32
    }

    pub fn mutant_color_for_seed(seed: u64) -> Color {
        let family: &[Color] = match (seed >> 6) % 3 {
            0 => &MUTANT_GREEN_PALETTE,
            1 => &MUTANT_PURPLE_PALETTE,
            _ => &MUTANT_WHITE_PALETTE,
        };
        family[(seed >> 8) as usize % family.len()]
    }

    pub fn parse(s: &str) -> Option<Self> {
        let lower = s.to_ascii_lowercase();
        ALL_SPECIES
            .iter()
            .find(|&&sp| sp.config().name.to_ascii_lowercase() == lower)
            .copied()
    }
}

impl Purchasable for FishSpecies {
    fn buy_price(&self) -> u32 {
        self.config().rarity.fish_buy_price()
    }
    fn display_name(&self) -> &str {
        self.config().name
    }
}

impl Sellable for FishSpecies {
    fn sell_price(&self) -> u32 {
        self.config().sell_base[0]
    }
    fn display_name(&self) -> &str {
        self.config().name
    }
}

#[cfg(test)]
mod appraisal_tests {
    use super::*;

    const SEEDS: [u64; 5] = [0, 1, u64::MAX / 3, u64::MAX / 2, u64::MAX];

    #[test]
    fn a_junkfish_is_worth_a_jackpot_and_a_bigger_one_a_bigger_jackpot() {
        let species = FishSpecies::from_junk().expect("some fish is made of junk");
        let mut previous_ceiling = 0;
        for size in SizeCategory::ALL {
            let worths: Vec<Money> = SEEDS
                .iter()
                .map(|&seed| species.appraisal(size, seed))
                .collect();
            let lowest = *worths.iter().min().unwrap();
            let highest = *worths.iter().max().unwrap();
            assert!(lowest >= JUNKFISH_APPRAISAL.floor, "{size:?}");
            assert!(highest <= JUNKFISH_APPRAISAL.ceiling, "{size:?}");
            assert!(
                lowest >= previous_ceiling,
                "{size:?} is never worth less than a smaller one"
            );
            previous_ceiling = highest;
        }
        assert_eq!(
            species.appraisal(SizeCategory::S, 0),
            CashValue::HundredThousand.amount() as Money
        );
    }

    #[test]
    fn only_the_junkfish_carries_an_appraisal() {
        for &species in ALL_SPECIES {
            assert_eq!(
                species.config().appraisal.is_some(),
                species.config().habitat == Habitat::Junkpile,
                "{}",
                species.display_name()
            );
        }
    }
}

#[cfg(test)]
mod fattening_tests {
    use super::*;

    const PLANNED_CAPS: [(Rarity, [u32; 4]); 3] = [
        (Rarity::Common, [108, 255, 592, 1_148]),
        (Rarity::Rare, [213, 455, 951, 1_825]),
        (Rarity::Legendary, [992, 2_238, 4_511, 7_279]),
    ];

    #[test]
    fn every_cap_is_the_one_the_feeding_law_derives() {
        assert_eq!(pellets_to_cap(), [48, 145, 390, 780]);
        for (rarity, caps) in PLANNED_CAPS {
            assert_eq!(rarity_arrays(rarity).2, caps, "{rarity:?}");
        }
    }
}
