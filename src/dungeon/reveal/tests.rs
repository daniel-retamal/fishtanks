use rand::SeedableRng;
use rand::rngs::SmallRng;

use super::*;
use crate::dungeon::Difficulty;
use crate::dungeon::battle::{BattleSetup, Stakes};
use crate::dungeon::bestiary::{EnemySpec, Rank};
use crate::dungeon::fighter::{Fighter, Scale};
use crate::fishes::fish::Fish;
use crate::fishes::species::FishSpecies;

const READ: &str = " reads ";

static PROWLER: EnemySpec = EnemySpec {
    name: "Prowler",
    floor: 1,
    rank: Rank::Common,
    hp: 10_000,
    mp: 0,
    bite: 1,
    speed: 6,
    sprite: &[],
    skills: &[],
    passives: &[],
    traits: &[],
    loot: &[],
    variant_of: None,
    note: "",
};

static MIRAGE: EnemySpec = EnemySpec {
    name: "Shimmer",
    traits: &[Trait::Mirage],
    ..PROWLER
};

fn battle(foes: &[&'static EnemySpec], known: Vec<String>) -> Battle {
    let mut rng = SmallRng::seed_from_u64(9);
    let party = vec![Fish::new(
        FishSpecies::Salmon,
        "Lilith".to_string(),
        0.0,
        0.0,
        &mut rng,
    )];
    let setup = BattleSetup {
        floor: 1,
        difficulty: Difficulty::Normal,
        stakes: Stakes::Encounter,
        purse: 0,
        items: Vec::new(),
        seed: 9,
        ambushed: false,
        haunts: Vec::new(),
        scale: Scale::PLAIN,
        known,
        every_graeae: false,
    };
    let foes = foes
        .iter()
        .map(|&spec| Fighter::of_enemy(spec, 1, Scale::PLAIN))
        .collect();
    Battle::new(party, foes, setup)
}

fn reads(battle: &Battle) -> usize {
    battle
        .log
        .iter()
        .filter(|line| line.text.contains(READ))
        .count()
}

fn granted(mut battle: Battle, gift: Trait) -> Battle {
    battle.fighters[0].traits.push(gift);
    battle.begin();
    battle
}

#[test]
fn every_enemy_plans_its_next_move_before_it_makes_it() {
    let mut battle = battle(&[&PROWLER, &PROWLER], Vec::new());
    battle.begin();
    for foe in battle.standing(Side::Foes) {
        assert!(battle.fighters[foe].foresight.is_some());
    }
}

#[test]
fn nobody_reads_a_move_without_a_gift_for_it() {
    let mut battle = battle(&[&PROWLER, &PROWLER], Vec::new());
    battle.begin();
    assert_eq!(reads(&battle), 0);
}

#[test]
fn a_telepath_reads_every_enemy() {
    for gift in [Trait::Telepathy, Trait::Electrosense, Trait::Cochlea] {
        let battle = granted(battle(&[&PROWLER, &PROWLER], Vec::new()), gift);
        assert!(reads(&battle) >= 2, "{gift:?}");
    }
}

#[test]
fn an_ear_hears_only_the_next_enemy() {
    for gift in [Trait::Gossip, Trait::Overhears] {
        let battle = granted(battle(&[&PROWLER, &PROWLER, &PROWLER], Vec::new()), gift);
        assert_eq!(reads(&battle), 1, "{gift:?}");
    }
}

#[test]
fn a_core_stack_reads_only_the_kinds_it_remembers() {
    let forgotten = granted(battle(&[&PROWLER], Vec::new()), Trait::CoreStack);
    assert_eq!(reads(&forgotten), 0);
    let known = granted(
        battle(&[&PROWLER], vec![PROWLER.name.to_string()]),
        Trait::CoreStack,
    );
    assert_eq!(reads(&known), 1);
}

#[test]
fn a_move_that_was_read_is_the_move_that_is_made() {
    let mut battle = granted(battle(&[&PROWLER], Vec::new()), Trait::Telepathy);
    let foe = battle.standing(Side::Foes)[0];
    let read = battle.fighters[foe].foresight.expect("a plan").plan;
    assert_eq!(next_move(&mut battle, foe), read);
}

#[test]
fn a_mirage_wears_one_of_your_fish() {
    let mut battle = battle(&[&MIRAGE], Vec::new());
    battle.begin();
    let mirage = battle.side(Side::Foes).next().expect("the mirage");
    assert_eq!(battle.fighters[mirage].disguise, Some(0));
}
