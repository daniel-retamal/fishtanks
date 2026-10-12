use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::colors;
use crate::dungeon::effect::Effect;
use crate::dungeon::floor::LAST_FLOOR;
use crate::dungeon::glyphs;
use crate::dungeon::specials::GHOST;

const GLOW: &str = "The Glow";
const INSIDE_OUT: &str = "Inside-out Fish";
const GLOW_EYE: char = '●';
const MIN_COMMONS: usize = 6;

#[test]
fn deep_sprites_draw_only_glyphs_the_game_already_draws() {
    let proven = glyphs::proven();
    let mut strangers: BTreeMap<char, Vec<&str>> = BTreeMap::new();
    for spec in all() {
        for row in spec.sprite {
            for glyph in row.text.chars() {
                if !proven.contains(&glyph) {
                    strangers.entry(glyph).or_default().push(spec.name);
                }
            }
        }
    }
    assert!(
        strangers.is_empty(),
        "glyphs no tank or fish draws: {strangers:?}"
    );
}

#[test]
fn every_name_is_its_own() {
    let mut seen = BTreeSet::new();
    for spec in all() {
        assert!(seen.insert(spec.name), "{} twice", spec.name);
    }
}

#[test]
fn a_variant_names_a_real_enemy_of_its_floor() {
    for spec in all() {
        if let Some(base) = spec.variant_of {
            let base = find(base).unwrap_or_else(|| panic!("{} varies nothing", spec.name));
            assert_eq!(base.floor, spec.floor, "{}", spec.name);
        }
    }
}

#[test]
fn every_summon_calls_an_enemy_the_bestiary_holds() {
    let mut called = vec![crate::dungeon::battle::CRUMBS, GHOST];
    for spec in all() {
        for skill in spec.skills {
            for effect in skill.effects {
                if let Effect::Summon { enemy, .. } = effect {
                    called.push(enemy);
                }
            }
        }
    }
    for name in called {
        assert!(find(name).is_some(), "{name} is not in the bestiary");
    }
}

#[test]
fn every_floor_has_its_bosses_and_its_elites() {
    for floor in 1..=LAST_FLOOR {
        let count = |rank: Rank| {
            of_floor(floor)
                .filter(|spec| spec.rank == rank && spec.variant_of.is_none())
                .count()
        };
        let bosses = if floor == LAST_FLOOR { 1 } else { 2 };
        assert_eq!(count(Rank::Boss), bosses, "floor {floor}");
        assert_eq!(count(Rank::Elite), 2, "floor {floor}");
        assert!(count(Rank::Common) >= MIN_COMMONS, "floor {floor}");
    }
}

#[test]
fn a_skill_costs_no_more_bubbles_than_its_owner_holds() {
    for spec in all() {
        for skill in spec.skills {
            assert!(
                u32::from(skill.cost) <= spec.mp,
                "{} cannot afford {}",
                spec.name,
                skill.name
            );
        }
    }
}

#[test]
fn the_inside_out_fish_is_red() {
    let spec = find(INSIDE_OUT).expect("the Hot Zone holds it");
    for row in spec.sprite.iter().filter(|row| !row.text.trim().is_empty()) {
        for (index, glyph) in row.text.chars().enumerate() {
            if glyph != ' ' {
                assert_eq!(row.colour_at(index), colors::RED, "{:?}", row.text);
            }
        }
    }
}

#[test]
fn the_glow_looks_you_in_the_eye() {
    let spec = find(GLOW).expect("the Hot Zone holds it");
    let width = spec.width();
    assert_eq!(width % 2, 1, "an even sprite has no centre column");
    let centre = width / 2;
    let eyes: Vec<(usize, usize)> = spec
        .sprite
        .iter()
        .enumerate()
        .flat_map(|(y, row)| {
            row.text
                .chars()
                .enumerate()
                .filter(|&(_, glyph)| glyph == GLOW_EYE)
                .map(move |(x, _)| (x, y))
        })
        .collect();
    assert_eq!(eyes.len(), 1, "one eye");
    let (eye_x, eye_y) = eyes[0];
    assert_eq!(eye_x, centre, "the eye sits in the centre column");
    for (y, row) in spec.sprite.iter().enumerate() {
        if y == eye_y || row.text.trim().is_empty() {
            continue;
        }
        let glyphs: Vec<char> = row.text.chars().collect();
        let first = glyphs.iter().position(|g| *g != ' ').expect("a drawn row");
        let last = glyphs.iter().rposition(|g| *g != ' ').expect("a drawn row");
        assert_eq!(
            first + last,
            centre * 2,
            "row {y} {:?} is centred",
            row.text
        );
    }
}
