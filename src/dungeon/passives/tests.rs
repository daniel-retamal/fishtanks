use rand::SeedableRng;
use rand::rngs::SmallRng;

use super::*;
use crate::dungeon::Difficulty;
use crate::dungeon::battle::{BattleSetup, Choice, Stakes};
use crate::dungeon::bestiary::{EnemySpec, Rank};
use crate::dungeon::fighter::{Fighter, Scale};
use crate::fishes::fish::Fish;
use crate::fishes::species::FishSpecies;

const TRAITS: &str = include_str!("../traits.rs");
const ENGINE: [&str; 9] = [
    include_str!("../battle.rs"),
    include_str!("../rules.rs"),
    include_str!("../passives.rs"),
    include_str!("../specials.rs"),
    include_str!("../ai.rs"),
    include_str!("../items.rs"),
    include_str!("../fighter.rs"),
    include_str!("../floor.rs"),
    include_str!("../bestiary/mod.rs"),
];
const FIGHT_ARM: &str = "=> Reader::Fight,";
const READER: &str = "pub fn reader(self) -> Reader {";
const TRAIT_PREFIX: &str = "Trait::";

static DUMMY: EnemySpec = EnemySpec {
    name: "Dummy",
    floor: 1,
    rank: Rank::Common,
    hp: 10_000,
    mp: 0,
    bite: 1,
    speed: 1,
    sprite: &[],
    skills: &[],
    passives: &[],
    traits: &[],
    loot: &[],
    variant_of: None,
    note: "",
};

fn fight_traits() -> Vec<&'static str> {
    let start = TRAITS.find(READER).expect("the reader table");
    let end = start + TRAITS[start..].find(FIGHT_ARM).expect("the fight arm");
    TRAITS[start..end]
        .split(TRAIT_PREFIX)
        .skip(1)
        .map(|rest| {
            rest.split(|c: char| !c.is_alphanumeric())
                .next()
                .unwrap_or("")
        })
        .filter(|name| !name.is_empty())
        .collect()
}

#[test]
fn every_fight_trait_is_read_by_the_battle() {
    let names = fight_traits();
    assert!(names.len() > 100, "the parser found {} names", names.len());
    let unread: Vec<&str> = names
        .into_iter()
        .filter(|name| {
            let needle = format!("{TRAIT_PREFIX}{name}");
            !ENGINE.iter().any(|source| {
                source
                    .match_indices(&needle)
                    .any(|(at, _)| !source[at + needle.len()..].starts_with(char::is_alphanumeric))
            })
        })
        .collect();
    assert!(
        unread.is_empty(),
        "granted but never read in a fight: {unread:?}"
    );
}

fn party(species: &[FishSpecies]) -> Vec<Fish> {
    let mut rng = SmallRng::seed_from_u64(31);
    species
        .iter()
        .map(|&s| Fish::new(s, format!("{s:?}"), 0.0, 0.0, &mut rng))
        .collect()
}

fn fight(species: &[FishSpecies]) -> Battle {
    let setup = BattleSetup {
        floor: 1,
        difficulty: Difficulty::Normal,
        stakes: Stakes::Encounter,
        purse: 0,
        items: Vec::new(),
        seed: 31,
        ambushed: false,
        haunts: Vec::new(),
        scale: Scale::PLAIN,
    };
    let foes = vec![Fighter::of_enemy(&DUMMY, 1, Scale::PLAIN)];
    let mut battle = Battle::new(party(species), foes, setup);
    battle.begin();
    battle
}

#[test]
fn a_school_of_commons_bites_harder_together() {
    let alone = fight(&[FishSpecies::Merluza]);
    let school = fight(&[
        FishSpecies::Merluza,
        FishSpecies::Salmon,
        FishSpecies::Jurel,
    ]);
    let fish = |battle: &Battle| battle.side(Side::Party).next().expect("a fish");
    let lone = &alone.fighters[fish(&alone)];
    let schooled = &school.fighters[fish(&school)];
    assert_eq!(schooled.bite, lone.bite + 2 * SCHOOL_BITE);
    assert_eq!(schooled.dodge, lone.dodge + 2 * SCHOOL_DODGE);
}

#[test]
fn a_shoalfish_counts_as_three_commons() {
    let school = fight(&[FishSpecies::Merluza, FishSpecies::Shoalfish]);
    let merluza = school.side(Side::Party).next().expect("a fish");
    let alone = fight(&[FishSpecies::Merluza]);
    let lone = alone.side(Side::Party).next().expect("a fish");
    assert_eq!(
        school.fighters[merluza].bite,
        alone.fighters[lone].bite + SHOAL_COUNTS_AS * SCHOOL_BITE
    );
}

#[test]
fn two_heads_act_twice_a_turn() {
    let mut battle = fight(&[FishSpecies::Merluza]);
    let fish = battle.side(Side::Party).next().expect("a fish");
    battle.fighters[fish].traits.push(Trait::TwoHeads);
    let dummy = battle.side(Side::Foes).next().expect("the dummy");
    let turns = battle.fighters[fish].turns;
    battle.choose(Choice::Bite(dummy));
    assert_eq!(battle.actor(), Some(fish));
    assert_eq!(battle.fighters[fish].turns, turns);
    battle.choose(Choice::Bite(dummy));
    assert!(battle.fighters[fish].turns > turns || battle.actor() != Some(fish));
}

#[test]
fn a_second_wind_rights_a_fish_once_a_fight() {
    let mut battle = fight(&[FishSpecies::Merluza]);
    let fish = battle.side(Side::Party).next().expect("a fish");
    battle.fighters[fish].traits.push(Trait::SecondWind);
    let max = battle.fighters[fish].max_hp;
    battle.lose_hp(fish, max, None);
    assert!(!battle.fighters[fish].belly_up);
    assert_eq!(battle.fighters[fish].hp, max * SECOND_WIND_PERCENT / 100);
    let hp = battle.fighters[fish].hp;
    battle.lose_hp(fish, hp, None);
    assert!(battle.fighters[fish].belly_up);
}

#[test]
fn a_staring_face_softens_every_enemy() {
    let mut battle = fight(&[FishSpecies::Merluza, FishSpecies::Salmon]);
    let fish = battle.side(Side::Party).next().expect("a fish");
    let other = battle.side(Side::Party).nth(1).expect("another fish");
    let dummy = battle.side(Side::Foes).next().expect("the dummy");
    assert_eq!(incoming(&battle, dummy, fish), 100);
    battle.fighters[other].traits.push(Trait::Stare);
    assert_eq!(incoming(&battle, dummy, fish), STARE_PERCENT);
}

#[test]
fn hue_families_split_the_wheel() {
    use crate::colors;
    assert_eq!(family(colors::GRAY), None);
    assert_ne!(family(colors::RED), family(colors::LIGHT_GREEN));
    let red = family(colors::RED).expect("a hue");
    let cyan = family(colors::LIGHT_CYAN).expect("a hue");
    assert!(opposite(red, cyan));
}
