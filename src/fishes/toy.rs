use rand::RngExt;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use crate::colors::{
    BUBBLEGUM, CHARCOAL, CHERRY, COCOA, CORAL, GOLD, LEMON, LILAC, LIME, MINT, NAVY, NAVY_LIGHT,
    SKY, SNOW, TANGERINE, VIOLET, WHITE, blended,
};
use crate::economy::{Money, Rarity};
use crate::fishes::species::{Locomotion, SizeCategory, Zoomie, sell_base};
use crate::sprite::{Cell, Feet, FeetStyle, TRANSPARENT, feet_row, mirror_char};

pub const STITCH: char = 'x';
const EYE: char = '°';
const MOUTH: char = '<';
const TAIL: char = '<';
const OPEN: char = '{';
const CLOSE: char = '}';
const STITCHES: [usize; 4] = [2, 3, 4, 6];
const HEAD_LEAD: usize = 2;
const TAIL_REACH: usize = 2;
const ABOVE: usize = 2;
const BELOW: usize = 1;
const LURE: char = 'º';
const LURE_STALK: char = ',';
const LURE_ROOT: char = '.';
const BILL: char = '-';
const DRILL_TIP: char = '<';
const DRILL_TURNS: [char; 2] = ['=', '≡'];
const FIN_LEAD: char = '/';
const VENTRAL_LEAD: char = '\\';
const FIN_BACK: char = '|';
const SPIKE: char = '¦';
const WING: char = '/';
const ANTENNA_TIP: char = crate::fishes::botfish::ANTENNA_TIP;
const ANTENNA_STALK: char = crate::fishes::botfish::ANTENNA_STALK;
const HORN: char = '\\';
const ROTOR_HUB: char = '+';
const ROTOR_BLADE: char = '-';
const CROWN: char = 'w';
const SIREN: char = '*';
const WHEEL: char = 'o';
const TREAD_OPEN: char = '(';
const TREAD_CLOSE: char = ')';
const TREAD_LINKS: [char; 2] = ['o', '·'];
const SHAFT_PLAIN: char = '-';
const NOZZLE: char = '=';
const FLAMES: [char; 2] = ['≈', '~'];
const PROPELLER_TURNS: [char; 2] = ['+', '×'];
const KEY_TURNS: [char; 2] = ['O', '-'];
const SPIN_HZ: f32 = 8.0;
const FLAME_HZ: f32 = 9.0;
const KEY_HZ: f32 = 1.5;
const SIREN_HZ: f32 = 4.0;
const SIREN_LIGHTS: [Color; 2] = [CHERRY, NAVY_LIGHT];
const FLAME_LIGHTS: [Color; 2] = [TANGERINE, LEMON];
const PEARL_BASE: f32 = 0.15;
const PEARL_SWELL: f32 = 0.25;
const PEARL_HZ: f32 = 2.2;
const PEARL_STEP: f32 = 0.5;
const SHINE_SECS: f32 = 2.2;
const SHINE_MARGIN: f32 = 3.0;
const SHINE_CORE: f32 = 0.8;
const SHINE_EDGE: f32 = 1.8;
const SHINE_HALF: f32 = 0.5;
const GLITTER_HZ: f32 = 6.0;
const GLITTER_SHARE: f32 = 0.18;
const HOLO_SPIN: f32 = 110.0;
const HOLO_STEP: f32 = 32.0;
const HOLO_SATURATION: f32 = 0.85;
const HOLO_LIGHTNESS: f32 = 0.7;
const DEGREES: f32 = 360.0;
const WINDUP_RUN_SECS: f32 = 2.0;
const WINDUP_REST_SECS: f32 = 1.0;
const WINDUP_FULL: f32 = 1.6;
const PACE_WHEELS: f32 = 1.6;
const PACE_TREADS: f32 = 0.5;
const PACE_FEET: f32 = 0.8;
const PACE_PROPELLER: f32 = 1.2;
const PACE_ROTOR: f32 = 0.6;
const PACE_DRIFT: f32 = 0.45;
const UPPER_THIRD: f32 = 1.0 / 3.0;
const UPPER_HALF: f32 = 0.5;

pub fn size_name(size: SizeCategory) -> &'static str {
    match size {
        SizeCategory::S => "small",
        SizeCategory::M => "medium",
        SizeCategory::L => "large",
        SizeCategory::XL => "giant",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Paint {
    Coral,
    Tangerine,
    Lemon,
    Lime,
    Mint,
    Sky,
    Lilac,
    Bubblegum,
    Cocoa,
    White,
    Charcoal,
    Cherry,
    Navy,
    Violet,
    Gold,
}

impl Paint {
    pub const SOLIDS: [Paint; 12] = [
        Paint::Coral,
        Paint::Tangerine,
        Paint::Lemon,
        Paint::Lime,
        Paint::Mint,
        Paint::Sky,
        Paint::Lilac,
        Paint::Bubblegum,
        Paint::Cocoa,
        Paint::White,
        Paint::Charcoal,
        Paint::Cherry,
    ];

    pub fn color(self) -> Color {
        match self {
            Paint::Coral => CORAL,
            Paint::Tangerine => TANGERINE,
            Paint::Lemon => LEMON,
            Paint::Lime => LIME,
            Paint::Mint => MINT,
            Paint::Sky => SKY,
            Paint::Lilac => LILAC,
            Paint::Bubblegum => BUBBLEGUM,
            Paint::Cocoa => COCOA,
            Paint::White => SNOW,
            Paint::Charcoal => CHARCOAL,
            Paint::Cherry => CHERRY,
            Paint::Navy => NAVY,
            Paint::Violet => VIOLET,
            Paint::Gold => GOLD,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Paint::Coral => "Coral",
            Paint::Tangerine => "Tangerine",
            Paint::Lemon => "Lemon",
            Paint::Lime => "Lime",
            Paint::Mint => "Mint",
            Paint::Sky => "Sky",
            Paint::Lilac => "Lilac",
            Paint::Bubblegum => "Bubblegum",
            Paint::Cocoa => "Cocoa",
            Paint::White => "White",
            Paint::Charcoal => "Charcoal",
            Paint::Cherry => "Cherry",
            Paint::Navy => "Navy",
            Paint::Violet => "Violet",
            Paint::Gold => "Gold",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ToyColor {
    Coral,
    Tangerine,
    Lemon,
    Lime,
    Mint,
    Sky,
    Lilac,
    Bubblegum,
    Cocoa,
    White,
    Charcoal,
    Cherry,
    Clownfish,
    Galaxy,
    Watermelon,
    Bumblebee,
    CandyCane,
    Panda,
    Sunset,
    Toxic,
    Ocean,
}

impl ToyColor {
    pub const ALL: [ToyColor; 21] = [
        ToyColor::Coral,
        ToyColor::Tangerine,
        ToyColor::Lemon,
        ToyColor::Lime,
        ToyColor::Mint,
        ToyColor::Sky,
        ToyColor::Lilac,
        ToyColor::Bubblegum,
        ToyColor::Cocoa,
        ToyColor::White,
        ToyColor::Charcoal,
        ToyColor::Cherry,
        ToyColor::Clownfish,
        ToyColor::Galaxy,
        ToyColor::Watermelon,
        ToyColor::Bumblebee,
        ToyColor::CandyCane,
        ToyColor::Panda,
        ToyColor::Sunset,
        ToyColor::Toxic,
        ToyColor::Ocean,
    ];

    pub fn paints(self) -> (Paint, Paint) {
        match self {
            ToyColor::Coral => (Paint::Coral, Paint::Coral),
            ToyColor::Tangerine => (Paint::Tangerine, Paint::Tangerine),
            ToyColor::Lemon => (Paint::Lemon, Paint::Lemon),
            ToyColor::Lime => (Paint::Lime, Paint::Lime),
            ToyColor::Mint => (Paint::Mint, Paint::Mint),
            ToyColor::Sky => (Paint::Sky, Paint::Sky),
            ToyColor::Lilac => (Paint::Lilac, Paint::Lilac),
            ToyColor::Bubblegum => (Paint::Bubblegum, Paint::Bubblegum),
            ToyColor::Cocoa => (Paint::Cocoa, Paint::Cocoa),
            ToyColor::White => (Paint::White, Paint::White),
            ToyColor::Charcoal => (Paint::Charcoal, Paint::Charcoal),
            ToyColor::Cherry => (Paint::Cherry, Paint::Cherry),
            ToyColor::Clownfish => (Paint::Tangerine, Paint::White),
            ToyColor::Galaxy => (Paint::Violet, Paint::Navy),
            ToyColor::Watermelon => (Paint::Bubblegum, Paint::Lime),
            ToyColor::Bumblebee => (Paint::Lemon, Paint::Charcoal),
            ToyColor::CandyCane => (Paint::Cherry, Paint::White),
            ToyColor::Panda => (Paint::White, Paint::Charcoal),
            ToyColor::Sunset => (Paint::Tangerine, Paint::Bubblegum),
            ToyColor::Toxic => (Paint::Lime, Paint::Lilac),
            ToyColor::Ocean => (Paint::Sky, Paint::Navy),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ToyColor::Clownfish => "Clownfish",
            ToyColor::Galaxy => "Galaxy",
            ToyColor::Watermelon => "Watermelon",
            ToyColor::Bumblebee => "Bumblebee",
            ToyColor::CandyCane => "Candy Cane",
            ToyColor::Panda => "Panda",
            ToyColor::Sunset => "Sunset",
            ToyColor::Toxic => "Toxic",
            ToyColor::Ocean => "Ocean",
            solid => solid.paints().0.name(),
        }
    }

    pub fn rarity(self) -> Rarity {
        let (a, b) = self.paints();
        if a == b { Rarity::Common } else { Rarity::Rare }
    }

    pub fn of_rarity(rarity: Rarity) -> Vec<ToyColor> {
        ToyColor::ALL
            .into_iter()
            .filter(|color| color.rarity() == rarity)
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Material {
    Plastic,
    Pearl,
    Metallic,
    Glitter,
    Holographic,
}

impl Material {
    pub const ALL: [Material; 5] = [
        Material::Plastic,
        Material::Pearl,
        Material::Metallic,
        Material::Glitter,
        Material::Holographic,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Material::Plastic => "Plastic",
            Material::Pearl => "Pearl",
            Material::Metallic => "Metallic",
            Material::Glitter => "Glitter",
            Material::Holographic => "Holographic",
        }
    }

    pub fn rarity(self) -> Rarity {
        match self {
            Material::Plastic => Rarity::Common,
            Material::Pearl | Material::Metallic | Material::Glitter => Rarity::Rare,
            Material::Holographic => Rarity::Legendary,
        }
    }

    pub fn roll(rng: &mut impl RngExt) -> Material {
        let rarity = Rarity::roll(rng);
        let tier: Vec<Material> = Material::ALL
            .into_iter()
            .filter(|material| material.rarity() == rarity)
            .collect();
        tier[rng.random_range(0..tier.len())]
    }

    fn shade(self, base: Color, at: Shading) -> Color {
        match self {
            Material::Plastic => base,
            Material::Pearl => {
                let swell = (at.clock * PEARL_HZ + at.x as f32 * PEARL_STEP).sin() * 0.5 + 0.5;
                blended(base, WHITE, PEARL_BASE + PEARL_SWELL * swell)
            }
            Material::Metallic => {
                let span = at.width as f32 + SHINE_MARGIN * 2.0;
                let phase = (at.clock + at.seed as f32 % SHINE_SECS).rem_euclid(SHINE_SECS);
                let sweep = phase / SHINE_SECS * span - SHINE_MARGIN;
                let distance = (at.x as f32 - sweep).abs();
                if distance < SHINE_CORE {
                    WHITE
                } else if distance < SHINE_EDGE {
                    blended(base, WHITE, SHINE_HALF)
                } else {
                    base
                }
            }
            Material::Glitter => {
                let beat = (at.clock * GLITTER_HZ) as u64;
                if sparkle(at.x as u64, at.row as u64, beat, at.seed) < GLITTER_SHARE {
                    WHITE
                } else {
                    base
                }
            }
            Material::Holographic => {
                let hue = (at.clock * HOLO_SPIN + at.x as f32 * HOLO_STEP).rem_euclid(DEGREES);
                hsl(hue, HOLO_SATURATION, HOLO_LIGHTNESS)
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Shading {
    x: usize,
    row: usize,
    width: usize,
    clock: f32,
    seed: u64,
}

fn turned(ch: char) -> char {
    match ch {
        '<' => '>',
        '>' => '<',
        other => mirror_char(other),
    }
}

fn sparkle(x: u64, row: u64, beat: u64, seed: u64) -> f32 {
    let mut h = x
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(row.wrapping_mul(0xC2B2_AE3D_27D4_EB4F))
        .wrapping_add(beat.wrapping_mul(0x1656_67B1_9E37_79F9))
        .wrapping_add(seed);
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    (h % 1000) as f32 / 1000.0
}

fn hsl(hue: f32, saturation: f32, lightness: f32) -> Color {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let sector = hue / 60.0;
    let second = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match sector as u32 {
        0 => (chroma, second, 0.0),
        1 => (second, chroma, 0.0),
        2 => (0.0, chroma, second),
        3 => (0.0, second, chroma),
        4 => (second, 0.0, chroma),
        _ => (chroma, 0.0, second),
    };
    let lift = lightness - chroma / 2.0;
    let channel = |v: f32| ((v + lift) * 255.0).round().clamp(0.0, 255.0) as u8;
    Color::Rgb(channel(r), channel(g), channel(b))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Slot {
    Head,
    Top,
    Under,
    Tail,
}

impl Slot {
    pub const ALL: [Slot; 4] = [Slot::Head, Slot::Top, Slot::Under, Slot::Tail];

    pub fn name(self) -> &'static str {
        match self {
            Slot::Head => "Head",
            Slot::Top => "Top",
            Slot::Under => "Under",
            Slot::Tail => "Tail",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ToyPart {
    Lure,
    Bill,
    Drill,
    DorsalFin,
    Spikes,
    Wings,
    Antenna,
    Horn,
    Rotor,
    Crown,
    Siren,
    VentralFin,
    Feet,
    Wheels,
    Treads,
    Rocket,
    Propeller,
    WindUpKey,
}

pub struct PartSpec {
    pub name: &'static str,
    pub slot: Slot,
    pub rarity: Rarity,
}

impl ToyPart {
    pub const ALL: [ToyPart; 18] = [
        ToyPart::Lure,
        ToyPart::Bill,
        ToyPart::Drill,
        ToyPart::DorsalFin,
        ToyPart::Spikes,
        ToyPart::Wings,
        ToyPart::Antenna,
        ToyPart::Horn,
        ToyPart::Rotor,
        ToyPart::Crown,
        ToyPart::Siren,
        ToyPart::VentralFin,
        ToyPart::Feet,
        ToyPart::Wheels,
        ToyPart::Treads,
        ToyPart::Rocket,
        ToyPart::Propeller,
        ToyPart::WindUpKey,
    ];

    pub fn spec(self) -> PartSpec {
        use Rarity::{Common, Rare};
        use Slot::{Head, Tail, Top, Under};
        let (name, slot, rarity) = match self {
            ToyPart::Lure => ("Lure", Head, Common),
            ToyPart::Bill => ("Bill", Head, Common),
            ToyPart::Drill => ("Drill", Head, Rare),
            ToyPart::DorsalFin => ("Dorsal Fin", Top, Common),
            ToyPart::Spikes => ("Spikes", Top, Common),
            ToyPart::Wings => ("Wings", Top, Common),
            ToyPart::Antenna => ("Antenna", Top, Common),
            ToyPart::Horn => ("Horn", Top, Common),
            ToyPart::Rotor => ("Rotor", Top, Rare),
            ToyPart::Crown => ("Crown", Top, Rare),
            ToyPart::Siren => ("Siren", Top, Rare),
            ToyPart::VentralFin => ("Ventral Fin", Under, Common),
            ToyPart::Feet => ("Feet", Under, Common),
            ToyPart::Wheels => ("Wheels", Under, Rare),
            ToyPart::Treads => ("Treads", Under, Rare),
            ToyPart::Rocket => ("Rocket", Tail, Rare),
            ToyPart::Propeller => ("Propeller", Tail, Rare),
            ToyPart::WindUpKey => ("Wind-up Key", Tail, Rare),
        };
        PartSpec { name, slot, rarity }
    }

    pub fn name(self) -> &'static str {
        self.spec().name
    }

    pub fn slot(self) -> Slot {
        self.spec().slot
    }

    pub fn rarity(self) -> Rarity {
        self.spec().rarity
    }

    pub fn of_rarity(rarity: Rarity) -> Vec<ToyPart> {
        ToyPart::ALL
            .into_iter()
            .filter(|part| part.rarity() == rarity)
            .collect()
    }

    fn walks(self) -> bool {
        matches!(self, ToyPart::Wheels | ToyPart::Treads | ToyPart::Feet)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FittedPart {
    pub part: ToyPart,
    pub paint: Paint,
}

impl FittedPart {
    pub fn name(self) -> String {
        format!("{} {}", self.paint.name(), self.part.name())
    }

    pub fn random(rarity: Rarity, rng: &mut impl RngExt) -> FittedPart {
        let parts = ToyPart::of_rarity(rarity);
        FittedPart {
            part: parts[rng.random_range(0..parts.len())],
            paint: Paint::SOLIDS[rng.random_range(0..Paint::SOLIDS.len())],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fittings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<FittedPart>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<FittedPart>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub under: Option<FittedPart>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tail: Option<FittedPart>,
}

impl Fittings {
    pub fn get(&self, slot: Slot) -> Option<FittedPart> {
        match slot {
            Slot::Head => self.head,
            Slot::Top => self.top,
            Slot::Under => self.under,
            Slot::Tail => self.tail,
        }
    }

    pub fn set(&mut self, slot: Slot, part: Option<FittedPart>) {
        match slot {
            Slot::Head => self.head = part,
            Slot::Top => self.top = part,
            Slot::Under => self.under = part,
            Slot::Tail => self.tail = part,
        }
    }

    pub fn worn(&self) -> Vec<FittedPart> {
        Slot::ALL
            .into_iter()
            .filter_map(|slot| self.get(slot))
            .collect()
    }

    fn part(&self, slot: Slot) -> Option<ToyPart> {
        self.get(slot).map(|fitted| fitted.part)
    }

    fn has(&self, part: ToyPart) -> bool {
        self.part(part.slot()) == Some(part)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Line {
    Mecha,
    Kaiju,
    Racers,
    DeepSea,
    Space,
    Royals,
}

impl Line {
    pub const ALL: [Line; 6] = [
        Line::Mecha,
        Line::Kaiju,
        Line::Racers,
        Line::DeepSea,
        Line::Space,
        Line::Royals,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Line::Mecha => "Mecha",
            Line::Kaiju => "Kaiju",
            Line::Racers => "Racers",
            Line::DeepSea => "Deep Sea",
            Line::Space => "Space",
            Line::Royals => "Royals",
        }
    }

    pub fn signatures(self) -> Vec<Signature> {
        Signature::ALL
            .into_iter()
            .filter(|s| s.line() == self && !s.is_secret())
            .collect()
    }

    pub fn secret(self) -> Signature {
        Signature::ALL
            .into_iter()
            .find(|s| s.line() == self && s.is_secret())
            .expect("every line keeps a secret")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    A,
    B,
    Accent,
    Fixed(Paint),
}

struct SignatureSpec {
    name: &'static str,
    line: Line,
    paints: (Paint, Paint, Paint),
    shiny: (Paint, Paint, Paint),
    material: Material,
    parts: &'static [(ToyPart, Role)],
    secret: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Signature {
    Mecha,
    Vanguard,
    Sentinel,
    Titan,
    Prime,
    Kaiju,
    Moth,
    Trihead,
    Shellback,
    Mechakaiju,
    HotRod,
    MonsterTruck,
    Patrol,
    JetCar,
    Turbo,
    Submarine,
    Angler,
    Narwhal,
    Diver,
    Nautilus,
    Starship,
    Saucer,
    Astronaut,
    Comet,
    Supernova,
    King,
    Queen,
    Knight,
    Jester,
    Emperor,
}

impl Signature {
    pub const ALL: [Signature; 30] = [
        Signature::Mecha,
        Signature::Vanguard,
        Signature::Sentinel,
        Signature::Titan,
        Signature::Prime,
        Signature::Kaiju,
        Signature::Moth,
        Signature::Trihead,
        Signature::Shellback,
        Signature::Mechakaiju,
        Signature::HotRod,
        Signature::MonsterTruck,
        Signature::Patrol,
        Signature::JetCar,
        Signature::Turbo,
        Signature::Submarine,
        Signature::Angler,
        Signature::Narwhal,
        Signature::Diver,
        Signature::Nautilus,
        Signature::Starship,
        Signature::Saucer,
        Signature::Astronaut,
        Signature::Comet,
        Signature::Supernova,
        Signature::King,
        Signature::Queen,
        Signature::Knight,
        Signature::Jester,
        Signature::Emperor,
    ];

    fn spec(self) -> SignatureSpec {
        use Material::{Glitter, Holographic, Metallic, Pearl, Plastic};
        use Paint::*;
        use Role::{A, Accent, B, Fixed};
        use ToyPart::*;
        let spec = |name: &'static str,
                    line: Line,
                    paints: (Paint, Paint, Paint),
                    shiny: (Paint, Paint, Paint),
                    material: Material,
                    parts: &'static [(ToyPart, Role)],
                    secret: bool| SignatureSpec {
            name,
            line,
            paints,
            shiny,
            material,
            parts,
            secret,
        };
        match self {
            Signature::Mecha => spec(
                "Mecha",
                Line::Mecha,
                (Lilac, Lime, Lime),
                (Tangerine, White, White),
                Metallic,
                &[(Horn, B), (Treads, A)],
                false,
            ),
            Signature::Vanguard => spec(
                "Vanguard",
                Line::Mecha,
                (Cherry, Tangerine, Tangerine),
                (Sky, White, White),
                Metallic,
                &[(Antenna, Accent), (Wheels, Fixed(Charcoal))],
                false,
            ),
            Signature::Sentinel => spec(
                "Sentinel",
                Line::Mecha,
                (White, Sky, Sky),
                (Lemon, Charcoal, Charcoal),
                Pearl,
                &[(Rotor, Accent), (Feet, A)],
                false,
            ),
            Signature::Titan => spec(
                "Titan",
                Line::Mecha,
                (Charcoal, Lemon, Lemon),
                (White, Cherry, Cherry),
                Metallic,
                &[(Drill, Accent), (Treads, A)],
                false,
            ),
            Signature::Prime => spec(
                "Prime",
                Line::Mecha,
                (White, Lilac, Lilac),
                (White, Lilac, Lilac),
                Holographic,
                &[(Crown, Accent), (Treads, A), (Rocket, A)],
                true,
            ),
            Signature::Kaiju => spec(
                "Kaiju",
                Line::Kaiju,
                (Charcoal, Charcoal, Sky),
                (Cherry, Charcoal, Tangerine),
                Plastic,
                &[(Spikes, Accent), (Feet, A)],
                false,
            ),
            Signature::Moth => spec(
                "Moth",
                Line::Kaiju,
                (Tangerine, Lemon, Lemon),
                (Lilac, Sky, Sky),
                Pearl,
                &[(Wings, Accent)],
                false,
            ),
            Signature::Trihead => spec(
                "Trihead",
                Line::Kaiju,
                (Lemon, Tangerine, Lemon),
                (Charcoal, Lime, Lime),
                Metallic,
                &[(Crown, Accent), (Feet, B)],
                false,
            ),
            Signature::Shellback => spec(
                "Shellback",
                Line::Kaiju,
                (Lime, Cocoa, Cocoa),
                (Mint, Charcoal, Charcoal),
                Plastic,
                &[(Spikes, Accent), (Rocket, Accent)],
                false,
            ),
            Signature::Mechakaiju => spec(
                "Mechakaiju",
                Line::Kaiju,
                (White, Charcoal, Sky),
                (White, Charcoal, Sky),
                Metallic,
                &[(Drill, A), (Spikes, Accent), (Treads, B)],
                true,
            ),
            Signature::HotRod => spec(
                "Hot Rod",
                Line::Racers,
                (Cherry, Tangerine, Tangerine),
                (Lilac, Sky, Sky),
                Metallic,
                &[(DorsalFin, Accent), (Wheels, Fixed(Charcoal)), (Rocket, A)],
                false,
            ),
            Signature::MonsterTruck => spec(
                "Monster Truck",
                Line::Racers,
                (Lime, Charcoal, Lime),
                (Bubblegum, Charcoal, Bubblegum),
                Plastic,
                &[(Antenna, Accent), (Wheels, B)],
                false,
            ),
            Signature::Patrol => spec(
                "Patrol",
                Line::Racers,
                (White, Charcoal, Charcoal),
                (Lemon, Charcoal, Charcoal),
                Plastic,
                &[(Siren, Accent), (Wheels, B)],
                false,
            ),
            Signature::JetCar => spec(
                "Jet Car",
                Line::Racers,
                (White, Sky, Sky),
                (Charcoal, Lemon, Lemon),
                Metallic,
                &[(Bill, A), (Wheels, Fixed(Charcoal)), (Rocket, Accent)],
                false,
            ),
            Signature::Turbo => spec(
                "Turbo",
                Line::Racers,
                (Lemon, Cherry, Cherry),
                (Lemon, Cherry, Cherry),
                Holographic,
                &[(Wings, A), (Wheels, Accent), (Rocket, A)],
                true,
            ),
            Signature::Submarine => spec(
                "Submarine",
                Line::DeepSea,
                (Lemon, Lemon, Charcoal),
                (Sky, Sky, White),
                Metallic,
                &[(Antenna, Accent), (Propeller, Accent)],
                false,
            ),
            Signature::Angler => spec(
                "Angler",
                Line::DeepSea,
                (Charcoal, Lilac, Lemon),
                (Bubblegum, White, Mint),
                Pearl,
                &[(Lure, Accent), (VentralFin, B)],
                false,
            ),
            Signature::Narwhal => spec(
                "Narwhal",
                Line::DeepSea,
                (White, Sky, White),
                (Charcoal, Lilac, Lilac),
                Plastic,
                &[(Bill, Accent), (VentralFin, B)],
                false,
            ),
            Signature::Diver => spec(
                "Diver",
                Line::DeepSea,
                (Tangerine, Cocoa, Cocoa),
                (Sky, White, White),
                Metallic,
                &[(Antenna, A), (Feet, Accent)],
                false,
            ),
            Signature::Nautilus => spec(
                "Nautilus",
                Line::DeepSea,
                (Mint, Navy, Mint),
                (Mint, Navy, Mint),
                Holographic,
                &[(Drill, Accent), (Spikes, B), (Propeller, Accent)],
                true,
            ),
            Signature::Starship => spec(
                "Starship",
                Line::Space,
                (White, Cherry, Cherry),
                (White, Navy, Navy),
                Plastic,
                &[(DorsalFin, Accent), (Rocket, Accent)],
                false,
            ),
            Signature::Saucer => spec(
                "Saucer",
                Line::Space,
                (Mint, Lilac, Lilac),
                (Lime, Charcoal, Charcoal),
                Glitter,
                &[(Rotor, Accent)],
                false,
            ),
            Signature::Astronaut => spec(
                "Astronaut",
                Line::Space,
                (White, Tangerine, Tangerine),
                (White, Lilac, Lilac),
                Pearl,
                &[(Antenna, Accent), (Feet, A)],
                false,
            ),
            Signature::Comet => spec(
                "Comet",
                Line::Space,
                (Sky, White, White),
                (Lilac, Bubblegum, Bubblegum),
                Glitter,
                &[(Wings, Accent), (Rocket, A)],
                false,
            ),
            Signature::Supernova => spec(
                "Supernova",
                Line::Space,
                (Lemon, Tangerine, Tangerine),
                (Lemon, Tangerine, Tangerine),
                Holographic,
                &[(Crown, A), (Rocket, Accent)],
                true,
            ),
            Signature::King => spec(
                "King",
                Line::Royals,
                (Cherry, Lemon, Lemon),
                (Lilac, Lemon, Lemon),
                Metallic,
                &[(Crown, Accent)],
                false,
            ),
            Signature::Queen => spec(
                "Queen",
                Line::Royals,
                (Bubblegum, White, White),
                (Mint, White, White),
                Pearl,
                &[(Crown, Accent)],
                false,
            ),
            Signature::Knight => spec(
                "Knight",
                Line::Royals,
                (White, Charcoal, Charcoal),
                (Lemon, Charcoal, Charcoal),
                Metallic,
                &[(Bill, Accent), (Feet, B)],
                false,
            ),
            Signature::Jester => spec(
                "Jester",
                Line::Royals,
                (Lime, Bubblegum, Bubblegum),
                (Sky, Lemon, Lemon),
                Plastic,
                &[(Spikes, Accent), (Feet, A)],
                false,
            ),
            Signature::Emperor => spec(
                "Emperor",
                Line::Royals,
                (Violet, Lemon, Lemon),
                (Violet, Lemon, Lemon),
                Holographic,
                &[(Crown, Accent), (Feet, A)],
                true,
            ),
        }
    }

    pub fn name(self) -> &'static str {
        self.spec().name
    }

    pub fn line(self) -> Line {
        self.spec().line
    }

    pub fn is_secret(self) -> bool {
        self.spec().secret
    }

    pub fn material(self) -> Material {
        self.spec().material
    }

    fn palette(self, shiny: bool) -> (Paint, Paint, Paint) {
        let spec = self.spec();
        if shiny { spec.shiny } else { spec.paints }
    }

    pub fn fittings(self, shiny: bool) -> Fittings {
        let (a, b, accent) = self.palette(shiny);
        let mut fittings = Fittings::default();
        for &(part, role) in self.spec().parts {
            let paint = match role {
                Role::A => a,
                Role::B => b,
                Role::Accent => accent,
                Role::Fixed(paint) => paint,
            };
            fittings.set(part.slot(), Some(FittedPart { part, paint }));
        }
        fittings
    }

    pub fn of_line(line: Line) -> Vec<Signature> {
        line.signatures()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Look {
    Plain { color: ToyColor, material: Material },
    Signature { signature: Signature, shiny: bool },
    Golden,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Depth {
    Floor,
    Surface,
    Upper(f32),
    Anywhere,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToyState {
    pub look: Look,
    #[serde(default)]
    pub fittings: Fittings,
}

pub struct ToySprite {
    pub rows: Vec<Vec<Cell>>,
    pub body_row: usize,
    pub span: (usize, usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ink {
    Body,
    Eye,
    Lit,
}

type Mark = Option<(char, Color, Ink)>;

impl ToyState {
    pub fn plain(color: ToyColor, material: Material) -> Self {
        Self {
            look: Look::Plain { color, material },
            fittings: Fittings::default(),
        }
    }

    pub fn signature(signature: Signature, shiny: bool) -> Self {
        Self {
            look: Look::Signature { signature, shiny },
            fittings: Fittings::default(),
        }
    }

    pub fn golden() -> Self {
        Self {
            look: Look::Golden,
            fittings: Fittings::default(),
        }
    }

    pub fn random(rng: &mut impl RngExt) -> Self {
        let rarity = match Rarity::roll(rng) {
            Rarity::Common => Rarity::Common,
            Rarity::Rare | Rarity::Legendary => Rarity::Rare,
        };
        let colors = ToyColor::of_rarity(rarity);
        Self::plain(
            colors[rng.random_range(0..colors.len())],
            Material::roll(rng),
        )
    }

    pub fn signature_size() -> SizeCategory {
        SizeCategory::L
    }

    pub fn is_sealed(&self) -> bool {
        !matches!(self.look, Look::Plain { .. })
    }

    pub fn fittings(&self) -> Fittings {
        match self.look {
            Look::Signature { signature, shiny } => signature.fittings(shiny),
            Look::Plain { .. } | Look::Golden => self.fittings,
        }
    }

    pub fn paints(&self) -> (Paint, Paint) {
        match self.look {
            Look::Plain { color, .. } => color.paints(),
            Look::Signature { signature, shiny } => {
                let (a, b, _) = signature.palette(shiny);
                (a, b)
            }
            Look::Golden => (Paint::Gold, Paint::Gold),
        }
    }

    pub fn material(&self) -> Material {
        match self.look {
            Look::Plain { material, .. } => material,
            Look::Signature { signature, .. } => signature.material(),
            Look::Golden => Material::Metallic,
        }
    }

    pub fn rarity(&self) -> Rarity {
        match self.look {
            Look::Plain { color, .. } => color.rarity(),
            Look::Signature { signature, shiny } if shiny || signature.is_secret() => {
                Rarity::Legendary
            }
            Look::Signature { .. } => Rarity::Rare,
            Look::Golden => Rarity::Legendary,
        }
    }

    pub fn title(&self) -> String {
        match self.look {
            Look::Plain { color, .. } => format!("{} Toyfish", color.name()),
            Look::Signature {
                signature,
                shiny: true,
            } => format!("Shiny {}", signature.name()),
            Look::Signature { signature, .. } => signature.name().to_string(),
            Look::Golden => "Golden Toyfish".to_string(),
        }
    }

    pub fn worth(&self, size: SizeCategory, floor: Money) -> Money {
        let base = Money::from(sell_base(self.rarity())[size as usize]);
        let (floored, multiplier) = match self.look {
            Look::Golden => (base, Rarity::Legendary.value_multiplier()),
            _ => (base.max(floor), self.material().rarity().value_multiplier()),
        };
        (floored as f64 * f64::from(multiplier)).round() as Money
    }

    pub fn locomotion(&self) -> Locomotion {
        match self.fittings().part(Slot::Under) {
            Some(part) if part.walks() => Locomotion::Floor,
            _ => Locomotion::Swim,
        }
    }

    pub fn depth(&self) -> Depth {
        let fittings = self.fittings();
        if self.locomotion() == Locomotion::Floor {
            return Depth::Floor;
        }
        if fittings.has(ToyPart::Rotor) {
            return Depth::Surface;
        }
        if fittings.has(ToyPart::Propeller) {
            return Depth::Anywhere;
        }
        if fittings.has(ToyPart::Wings) {
            return Depth::Upper(UPPER_HALF);
        }
        Depth::Upper(UPPER_THIRD)
    }

    pub fn zoomie(&self) -> Zoomie {
        let fittings = self.fittings();
        if self.locomotion() == Locomotion::Floor && fittings.has(ToyPart::Rotor) {
            return Zoomie::Hop;
        }
        if fittings.has(ToyPart::Rocket) {
            return Zoomie::Lunge;
        }
        if fittings.has(ToyPart::Propeller) {
            return Zoomie::Burst;
        }
        Zoomie::None
    }

    pub fn pace(&self, clock: f32) -> f32 {
        let fittings = self.fittings();
        let base = match fittings.part(Slot::Under) {
            Some(ToyPart::Wheels) => PACE_WHEELS,
            Some(ToyPart::Treads) => PACE_TREADS,
            Some(ToyPart::Feet) => PACE_FEET,
            _ if fittings.has(ToyPart::Propeller) => PACE_PROPELLER,
            _ if fittings.has(ToyPart::Rotor) => PACE_ROTOR,
            _ => PACE_DRIFT,
        };
        if !fittings.has(ToyPart::WindUpKey) {
            return base;
        }
        let turn = clock.rem_euclid(WINDUP_RUN_SECS + WINDUP_REST_SECS);
        if turn >= WINDUP_RUN_SECS {
            return 0.0;
        }
        base * WINDUP_FULL * (1.0 - turn / WINDUP_RUN_SECS)
    }

    pub fn width(&self, size: SizeCategory) -> usize {
        let fittings = self.fittings();
        let lead = if fittings.head.is_some() {
            HEAD_LEAD
        } else {
            0
        };
        let reach = if fittings.tail.is_some() {
            TAIL_REACH
        } else {
            0
        };
        lead + STITCHES[size as usize] + 5 + reach
    }

    pub fn sprite(
        &self,
        facing_left: bool,
        size: SizeCategory,
        clock: f32,
        seed: u64,
    ) -> ToySprite {
        let fittings = self.fittings();
        let width = self.width(size);
        let stitches = STITCHES[size as usize];
        let lead = if fittings.head.is_some() {
            HEAD_LEAD
        } else {
            0
        };
        let mouth = lead;
        let eye = mouth + 1;
        let open = mouth + 2;
        let lo = mouth + 3;
        let hi = lo + stitches - 1;
        let mid = (lo + hi) / 2;
        let close = hi + 1;
        let tail = hi + 2;
        let rows = ABOVE + 1 + BELOW;
        let body = ABOVE;
        let mut grid: Vec<Vec<Mark>> = vec![vec![None; width]; rows];
        let (a, b) = self.paints();
        let mut put = |row: usize, col: usize, ch: char, color: Color, ink: Ink| {
            if let Some(cell) = grid.get_mut(row).and_then(|r| r.get_mut(col)) {
                *cell = Some((ch, color, ink));
            }
        };
        put(body, mouth, MOUTH, a.color(), Ink::Body);
        put(body, eye, EYE, WHITE, Ink::Eye);
        put(body, open, OPEN, a.color(), Ink::Body);
        for (i, col) in (lo..=hi).enumerate() {
            let paint = if i % 2 == 0 { a } else { b };
            put(body, col, STITCH, paint.color(), Ink::Body);
        }
        put(body, close, CLOSE, a.color(), Ink::Body);
        put(body, tail, TAIL, b.color(), Ink::Body);
        let turn = |hz: f32, frames: usize| ((clock * hz) as usize) % frames;
        if let Some(fitted) = fittings.head {
            let paint = fitted.paint.color();
            match fitted.part {
                ToyPart::Lure => {
                    put(body, mouth - 2, LURE, paint, Ink::Body);
                    put(body - 1, mouth - 1, LURE_STALK, paint, Ink::Body);
                    put(body - 1, mouth, LURE_ROOT, paint, Ink::Body);
                }
                ToyPart::Bill => {
                    put(body, mouth - 2, BILL, paint, Ink::Body);
                    put(body, mouth - 1, BILL, paint, Ink::Body);
                }
                _ => {
                    put(body, mouth - 2, DRILL_TIP, paint, Ink::Body);
                    put(
                        body,
                        mouth - 1,
                        DRILL_TURNS[turn(SPIN_HZ, 2)],
                        paint,
                        Ink::Body,
                    );
                }
            }
        }
        if let Some(fitted) = fittings.top {
            let paint = fitted.paint.color();
            let over = body - 1;
            match fitted.part {
                ToyPart::DorsalFin => {
                    put(over, mid, FIN_LEAD, paint, Ink::Body);
                    put(over, mid + 1, FIN_BACK, paint, Ink::Body);
                }
                ToyPart::Spikes => {
                    for col in lo..=hi {
                        put(over, col, SPIKE, paint, Ink::Body);
                    }
                }
                ToyPart::Wings => {
                    for col in lo..=hi {
                        put(over, col, WING, paint, Ink::Body);
                    }
                }
                ToyPart::Antenna => {
                    put(over, mid, ANTENNA_STALK, paint, Ink::Body);
                    put(over - 1, mid, ANTENNA_TIP, paint, Ink::Body);
                }
                ToyPart::Horn => put(over, eye, HORN, paint, Ink::Body),
                ToyPart::Rotor => {
                    put(over, mid, ROTOR_HUB, paint, Ink::Body);
                    if turn(SPIN_HZ, 2) == 0 {
                        put(over, mid - 1, ROTOR_BLADE, paint, Ink::Body);
                        put(over, mid + 1, ROTOR_BLADE, paint, Ink::Body);
                    }
                }
                ToyPart::Crown => {
                    put(over, eye, CROWN, paint, Ink::Body);
                    put(over, open, CROWN, paint, Ink::Body);
                }
                _ => put(over, lo, SIREN, SIREN_LIGHTS[turn(SIREN_HZ, 2)], Ink::Lit),
            }
        }
        if let Some(fitted) = fittings.under {
            let paint = fitted.paint.color();
            let below = body + 1;
            match fitted.part {
                ToyPart::VentralFin => {
                    put(below, mid, VENTRAL_LEAD, paint, Ink::Body);
                    put(below, mid + 1, FIN_BACK, paint, Ink::Body);
                }
                ToyPart::Feet => {
                    let body_cells: Vec<Cell> = (0..width).map(|_| (STITCH, paint)).collect();
                    let feet = Feet {
                        style: FeetStyle::Quote,
                        color: Some(paint),
                    };
                    for (col, (ch, color)) in feet_row(&body_cells, (lo, hi), feet)
                        .into_iter()
                        .enumerate()
                    {
                        if ch != TRANSPARENT {
                            put(below, col, ch, color, Ink::Body);
                        }
                    }
                }
                ToyPart::Wheels => {
                    put(below, lo, WHEEL, paint, Ink::Body);
                    put(below, hi, WHEEL, paint, Ink::Body);
                }
                _ => {
                    put(below, lo - 1, TREAD_OPEN, paint, Ink::Body);
                    for (i, col) in (lo..=hi).enumerate() {
                        put(below, col, TREAD_LINKS[i % 2], paint, Ink::Body);
                    }
                    put(below, close, TREAD_CLOSE, paint, Ink::Body);
                }
            }
        }
        if let Some(fitted) = fittings.tail {
            let paint = fitted.paint.color();
            match fitted.part {
                ToyPart::Rocket => {
                    put(body, tail + 1, NOZZLE, paint, Ink::Body);
                    let flame = turn(FLAME_HZ, 2);
                    put(body, tail + 2, FLAMES[flame], FLAME_LIGHTS[flame], Ink::Lit);
                }
                ToyPart::Propeller => {
                    put(body, tail + 1, SHAFT_PLAIN, paint, Ink::Body);
                    put(
                        body,
                        tail + 2,
                        PROPELLER_TURNS[turn(SPIN_HZ, 2)],
                        paint,
                        Ink::Body,
                    );
                }
                _ => {
                    put(body, tail + 1, SHAFT_PLAIN, paint, Ink::Body);
                    put(body, tail + 2, KEY_TURNS[turn(KEY_HZ, 2)], paint, Ink::Body);
                }
            }
        }
        let material = self.material();
        let first = grid
            .iter()
            .position(|row| row.iter().any(Option::is_some))
            .unwrap_or(body);
        let last = grid
            .iter()
            .rposition(|row| row.iter().any(Option::is_some))
            .unwrap_or(body);
        let mut rows_out: Vec<Vec<Cell>> = Vec::with_capacity(last + 1 - first);
        for (r, row) in grid.iter().enumerate().take(last + 1).skip(first) {
            let cells: Vec<Cell> = row
                .iter()
                .enumerate()
                .map(|(x, mark)| match mark {
                    None => (TRANSPARENT, Color::Reset),
                    Some((ch, color, Ink::Body)) => {
                        let screen_x = if facing_left { x } else { width - 1 - x };
                        let at = Shading {
                            x: screen_x,
                            row: r,
                            width,
                            clock,
                            seed,
                        };
                        (*ch, material.shade(*color, at))
                    }
                    Some((ch, color, _)) => (*ch, *color),
                })
                .collect();
            rows_out.push(if facing_left {
                cells
            } else {
                cells
                    .into_iter()
                    .rev()
                    .map(|(ch, color)| (turned(ch), color))
                    .collect()
            });
        }
        let span = if facing_left {
            (lo, hi)
        } else {
            (width - 1 - hi, width - 1 - lo)
        };
        ToySprite {
            rows: rows_out,
            body_row: body - first,
            span,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PartStack {
    pub fitted: FittedPart,
    pub count: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shelf {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub colors: Vec<ToyColor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materials: Vec<Material>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signatures: Vec<Signature>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shinies: Vec<Signature>,
    #[serde(default)]
    pub golden: bool,
}

impl Shelf {
    pub fn shelve(&mut self, toy: &ToyState) {
        let add = |list: &mut Vec<_>, item| {
            if !list.contains(&item) {
                list.push(item);
                list.sort();
            }
        };
        match toy.look {
            Look::Plain { color, material } => {
                add(&mut self.colors, color);
                if !self.materials.contains(&material) {
                    self.materials.push(material);
                    self.materials.sort();
                }
            }
            Look::Signature { signature, shiny } => {
                let list = if shiny {
                    &mut self.shinies
                } else {
                    &mut self.signatures
                };
                if !list.contains(&signature) {
                    list.push(signature);
                    list.sort();
                }
            }
            Look::Golden => self.golden = true,
        }
    }

    pub fn holds(&self, toy: &ToyState) -> bool {
        match toy.look {
            Look::Plain { color, .. } => self.colors.contains(&color),
            Look::Signature {
                signature,
                shiny: true,
            } => self.shinies.contains(&signature),
            Look::Signature { signature, .. } => self.signatures.contains(&signature),
            Look::Golden => self.golden,
        }
    }

    pub fn every_color(&self) -> bool {
        ToyColor::ALL
            .iter()
            .all(|color| self.colors.contains(color))
    }

    pub fn golden_unlocked(&self) -> bool {
        self.every_color() && !self.golden
    }

    pub fn line_complete(&self, line: Line) -> bool {
        line.signatures()
            .iter()
            .all(|signature| self.signatures.contains(signature))
    }

    pub fn secrets_unlocked(&self) -> Vec<Signature> {
        Line::ALL
            .into_iter()
            .filter(|&line| self.line_complete(line))
            .map(Line::secret)
            .filter(|secret| !self.signatures.contains(secret))
            .collect()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Toybox {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<PartStack>,
    #[serde(default)]
    pub shelf: Shelf,
}

impl Toybox {
    pub fn add(&mut self, fitted: FittedPart) {
        match self.parts.iter_mut().find(|stack| stack.fitted == fitted) {
            Some(stack) => stack.count += 1,
            None => {
                self.parts.push(PartStack { fitted, count: 1 });
                self.parts.sort();
            }
        }
    }

    pub fn take(&mut self, fitted: FittedPart) -> bool {
        let Some(index) = self.parts.iter().position(|stack| stack.fitted == fitted) else {
            return false;
        };
        self.parts[index].count -= 1;
        if self.parts[index].count == 0 {
            self.parts.remove(index);
        }
        true
    }

    pub fn count(&self, fitted: FittedPart) -> u32 {
        self.parts
            .iter()
            .find(|stack| stack.fitted == fitted)
            .map_or(0, |stack| stack.count)
    }

    pub fn for_slot(&self, slot: Slot) -> Vec<FittedPart> {
        self.parts
            .iter()
            .filter(|stack| stack.fitted.part.slot() == slot)
            .map(|stack| stack.fitted)
            .collect()
    }

    pub fn total(&self) -> u32 {
        self.parts.iter().map(|stack| stack.count).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLOOR: Money = 50;

    fn body_text(toy: &ToyState, facing_left: bool, size: SizeCategory) -> Vec<String> {
        toy.sprite(facing_left, size, 0.0, 0)
            .rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|(ch, _)| if *ch == TRANSPARENT { ' ' } else { *ch })
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    #[test]
    fn a_plain_toy_is_stitches_and_a_button_eye() {
        let toy = ToyState::plain(ToyColor::Coral, Material::Plastic);
        assert_eq!(body_text(&toy, true, SizeCategory::L), vec!["<°{xxxx}<"]);
        assert_eq!(body_text(&toy, false, SizeCategory::S), vec![">{xx}°>"]);
    }

    #[test]
    fn the_size_adds_stitches_and_nothing_else() {
        let toy = ToyState::plain(ToyColor::Lime, Material::Plastic);
        let widths: Vec<usize> = SizeCategory::ALL.iter().map(|&s| toy.width(s)).collect();
        assert_eq!(widths, vec![7, 8, 9, 11]);
    }

    #[test]
    fn a_horn_points_where_the_toy_looks() {
        let mut toy = ToyState::plain(ToyColor::Sky, Material::Plastic);
        toy.fittings.top = Some(FittedPart {
            part: ToyPart::Horn,
            paint: Paint::White,
        });
        assert_eq!(
            body_text(&toy, true, SizeCategory::M),
            vec![" \\", "<°{xxx}<"]
        );
        assert_eq!(
            body_text(&toy, false, SizeCategory::M),
            vec!["      /", ">{xxx}°>"]
        );
    }

    #[test]
    fn an_antenna_stands_in_the_middle_of_the_body() {
        let mut toy = ToyState::plain(ToyColor::Sky, Material::Plastic);
        toy.fittings.top = Some(FittedPart {
            part: ToyPart::Antenna,
            paint: Paint::White,
        });
        assert_eq!(
            body_text(&toy, true, SizeCategory::M),
            vec!["    o", "    ‖", "<°{xxx}<"]
        );
    }

    #[test]
    fn wings_flip_with_the_toy_and_never_flap() {
        let mut toy = ToyState::plain(ToyColor::Sky, Material::Plastic);
        toy.fittings.top = Some(FittedPart {
            part: ToyPart::Wings,
            paint: Paint::White,
        });
        let left: Vec<Vec<String>> = (0..10)
            .map(|t| {
                toy.sprite(true, SizeCategory::M, t as f32 * 0.37, 0)
                    .rows
                    .iter()
                    .map(|r| r.iter().map(|c| c.0).collect())
                    .collect()
            })
            .collect();
        assert!(left.windows(2).all(|w| w[0] == w[1]));
        assert_eq!(body_text(&toy, true, SizeCategory::M)[0], "   ///");
        assert_eq!(body_text(&toy, false, SizeCategory::M)[0], "  \\\\\\");
    }

    #[test]
    fn a_wind_up_key_turns_between_a_bow_and_its_edge() {
        let mut toy = ToyState::plain(ToyColor::Sky, Material::Plastic);
        toy.fittings.tail = Some(FittedPart {
            part: ToyPart::WindUpKey,
            paint: Paint::Lemon,
        });
        let frames: Vec<char> = [0.0, 0.7]
            .iter()
            .map(|&t| {
                let row = &toy.sprite(true, SizeCategory::S, t, 0).rows[0];
                row[row.len() - 1].0
            })
            .collect();
        assert_eq!(frames, vec!['O', '-']);
    }

    #[test]
    fn wheels_and_feet_put_a_toy_on_the_floor_and_a_rotor_floats_it() {
        let mut toy = ToyState::plain(ToyColor::Sky, Material::Plastic);
        assert_eq!(toy.depth(), Depth::Upper(UPPER_THIRD));
        toy.fittings.top = Some(FittedPart {
            part: ToyPart::Rotor,
            paint: Paint::White,
        });
        assert_eq!(toy.depth(), Depth::Surface);
        toy.fittings.under = Some(FittedPart {
            part: ToyPart::Wheels,
            paint: Paint::Charcoal,
        });
        assert_eq!(toy.depth(), Depth::Floor);
        assert_eq!(toy.zoomie(), Zoomie::Hop);
    }

    #[test]
    fn the_material_shines_the_parts_too() {
        let mut toy = ToyState::plain(ToyColor::White, Material::Holographic);
        toy.fittings.top = Some(FittedPart {
            part: ToyPart::Spikes,
            paint: Paint::White,
        });
        let sprite = toy.sprite(true, SizeCategory::L, 1.3, 0);
        let spikes: Vec<Color> = sprite.rows[0]
            .iter()
            .filter(|c| c.0 == SPIKE)
            .map(|c| c.1)
            .collect();
        assert!(spikes.iter().all(|&c| c != SNOW), "{spikes:?}");
    }

    #[test]
    fn nothing_from_the_claw_is_worth_less_than_a_go() {
        for color in ToyColor::ALL {
            for material in Material::ALL {
                for size in SizeCategory::ALL {
                    let toy = ToyState::plain(color, material);
                    assert!(toy.worth(size, FLOOR) >= FLOOR);
                }
            }
        }
        for signature in Signature::ALL {
            for shiny in [false, true] {
                let toy = ToyState::signature(signature, shiny);
                assert!(toy.worth(ToyState::signature_size(), FLOOR) >= FLOOR);
            }
        }
    }

    #[test]
    fn the_golden_toyfish_is_worth_the_most_of_all() {
        let golden = ToyState::golden().worth(SizeCategory::XL, FLOOR);
        for signature in Signature::ALL {
            let toy = ToyState::signature(signature, true);
            assert!(toy.worth(ToyState::signature_size(), FLOOR) < golden);
        }
        for color in ToyColor::ALL {
            let toy = ToyState::plain(color, Material::Holographic);
            assert!(toy.worth(SizeCategory::XL, FLOOR) < golden);
        }
    }

    #[test]
    fn every_line_has_four_signatures_and_one_secret() {
        for line in Line::ALL {
            assert_eq!(line.signatures().len(), 4, "{}", line.name());
            assert!(line.secret().is_secret());
        }
    }

    #[test]
    fn a_shiny_repaints_its_parts_with_its_body() {
        for signature in Signature::ALL {
            let plain = signature.fittings(false);
            let shiny = signature.fittings(true);
            for slot in Slot::ALL {
                assert_eq!(
                    plain.get(slot).map(|p| p.part),
                    shiny.get(slot).map(|p| p.part)
                );
            }
            if signature.is_secret() {
                continue;
            }
            assert_ne!(
                signature.palette(false),
                signature.palette(true),
                "{} has a shiny that looks the same",
                signature.name()
            );
        }
    }

    #[test]
    fn a_signature_wears_one_part_per_slot() {
        for signature in Signature::ALL {
            let mut seen = Vec::new();
            for &(part, _) in signature.spec().parts {
                assert!(!seen.contains(&part.slot()), "{}", signature.name());
                seen.push(part.slot());
            }
        }
    }

    #[test]
    fn the_shelf_unlocks_a_secret_once_a_line_is_whole() {
        let mut shelf = Shelf::default();
        for signature in Line::Kaiju.signatures() {
            assert!(shelf.secrets_unlocked().is_empty());
            shelf.shelve(&ToyState::signature(signature, false));
        }
        assert_eq!(shelf.secrets_unlocked(), vec![Signature::Mechakaiju]);
        shelf.shelve(&ToyState::signature(Signature::Mechakaiju, false));
        assert!(shelf.secrets_unlocked().is_empty());
    }

    #[test]
    fn the_golden_waits_for_every_color() {
        let mut shelf = Shelf::default();
        for color in ToyColor::ALL {
            assert!(!shelf.golden_unlocked());
            shelf.shelve(&ToyState::plain(color, Material::Plastic));
        }
        assert!(shelf.golden_unlocked());
        shelf.shelve(&ToyState::golden());
        assert!(!shelf.golden_unlocked());
    }

    #[test]
    fn a_toybox_counts_its_parts() {
        let mut toybox = Toybox::default();
        let wings = FittedPart {
            part: ToyPart::Wings,
            paint: Paint::Mint,
        };
        toybox.add(wings);
        toybox.add(wings);
        assert_eq!(toybox.count(wings), 2);
        assert!(toybox.take(wings));
        assert!(toybox.take(wings));
        assert!(!toybox.take(wings));
        assert!(toybox.parts.is_empty());
    }
}
