use std::collections::BTreeSet;

use rand::SeedableRng;
use rand::rngs::SmallRng;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::fishes::fish::Fish;
use crate::fishes::mutations::{Mutation, apply_mutation};
use crate::fishes::species::{ALL_SPECIES, BodyChars, BodyTemplate, FishSpecies, TailKind};
use crate::fishes::unfish::SPAWNABLE_UNFISH;
use crate::settings::Settings;
use crate::tank::{Sky, Tank, TankKind};
use crate::ui::index_query::{ASCENDING_MARK, DESCENDING_MARK};
use crate::ui::tank_view::TankView;

use super::skill::{EMPTY_BUBBLE, FULL_BUBBLE};

const PRINTABLE_ASCII: std::ops::RangeInclusive<u32> = 0x20..=0x7E;
const BOX_DRAWING: std::ops::RangeInclusive<u32> = 0x2500..=0x257F;
const BLOCK_ELEMENTS: std::ops::RangeInclusive<u32> = 0x2580..=0x259F;
const TANK_WIDTH: u16 = 160;
const TANK_HEIGHT: u16 = 48;
const SEED: u64 = 1;
const TICKS: usize = 600;
const LOOK_EVERY: usize = 40;
const MEALS: usize = 6;

fn sprite_glyphs(fish: &Fish, glyphs: &mut BTreeSet<char>) {
    for row in fish.line_sprite().rows {
        glyphs.extend(row.into_iter().map(|(glyph, _)| glyph));
    }
}

fn body_glyphs(chars: &BodyChars, glyphs: &mut BTreeSet<char>) {
    glyphs.extend([
        chars.mouth_left,
        chars.mouth_right,
        chars.eye_left,
        chars.eye_right,
        chars.body_left,
        chars.wave_left,
        chars.body_right,
        chars.wave_right,
    ]);
    match chars.tail {
        TailKind::Custom { left, right } => glyphs.extend([left, right]),
        TailKind::Swaying { left, right, wave } => glyphs.extend([left, right, wave]),
        TailKind::Wide | TailKind::Short | TailKind::WideCurly | TailKind::None => {}
    }
}

fn species_glyphs(species: FishSpecies, glyphs: &mut BTreeSet<char>) {
    match species.config().body {
        BodyTemplate::Standard(chars) => body_glyphs(&chars, glyphs),
        BodyTemplate::Alternating(one, two) => {
            body_glyphs(&one, glyphs);
            body_glyphs(&two, glyphs);
        }
        BodyTemplate::Fixed { left, right } => {
            glyphs.extend(left.iter().chain(right).flat_map(|row| row.chars()));
        }
        BodyTemplate::Figure(_) => {}
    }
}

fn look(tank: &Tank, glyphs: &mut BTreeSet<char>) {
    let area = Rect::new(0, 0, TANK_WIDTH, TANK_HEIGHT);
    let mut buf = Buffer::empty(area);
    TankView::new(tank).render(area, &mut buf);
    for cell in buf.content() {
        glyphs.extend(cell.symbol().chars());
    }
}

fn tank_glyphs(kind: TankKind, glyphs: &mut BTreeSet<char>) {
    let mut tank = Tank::new(kind.display_name().to_string(), kind, &[]);
    tank.resize(TANK_WIDTH, TANK_HEIGHT, &[]);
    let mut food = u32::MAX;
    tank.feed(MEALS, &mut food);
    let settings = Settings::default();
    for tick in 0..TICKS {
        tank.tick(&settings, 0, Sky::default());
        if tick % LOOK_EVERY == 0 {
            look(&tank, glyphs);
        }
    }
}

pub fn proven() -> BTreeSet<char> {
    let mut rng = SmallRng::seed_from_u64(SEED);
    let mut glyphs = BTreeSet::new();
    for &species in ALL_SPECIES {
        species_glyphs(species, &mut glyphs);
        sprite_glyphs(
            &Fish::new(species, String::new(), 0.0, 0.0, &mut rng),
            &mut glyphs,
        );
    }
    for &kind in SPAWNABLE_UNFISH {
        sprite_glyphs(
            &Fish::new_unfish(kind, String::new(), 0.0, 0.0, &mut rng),
            &mut glyphs,
        );
    }
    for &mutation in Mutation::ALL {
        let mut fish = Fish::new(FishSpecies::Merluza, String::new(), 0.0, 0.0, &mut rng);
        apply_mutation(&mut fish, mutation, &mut rng);
        sprite_glyphs(&fish, &mut glyphs);
    }
    for &kind in TankKind::all() {
        tank_glyphs(kind, &mut glyphs);
    }
    glyphs.extend(
        PRINTABLE_ASCII
            .chain(BOX_DRAWING)
            .chain(BLOCK_ELEMENTS)
            .filter_map(char::from_u32),
    );
    glyphs.extend(FULL_BUBBLE.chars().chain(EMPTY_BUBBLE.chars()));
    glyphs.extend(ASCENDING_MARK.chars().chain(DESCENDING_MARK.chars()));
    glyphs
}
