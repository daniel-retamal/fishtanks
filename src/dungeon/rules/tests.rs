use rand::SeedableRng;
use rand::rngs::SmallRng;

use super::*;
use crate::dungeon::Difficulty;
use crate::dungeon::battle::{BattleSetup, Choice, Stakes};
use crate::dungeon::fighter::{Ghost, Scale};
use crate::fishes::fish::Fish;
use crate::fishes::species::FishSpecies;

const SEED: u64 = 21;
const HAUL: u64 = 10_000;

fn party() -> Vec<Fish> {
    let mut rng = SmallRng::seed_from_u64(SEED);
    [FishSpecies::Salmon, FishSpecies::Merluza]
        .into_iter()
        .map(|species| Fish::new(species, format!("{species:?}"), 0.0, 0.0, &mut rng))
        .collect()
}

fn setup(floor: u32, haunts: Vec<Ghost>) -> BattleSetup {
    BattleSetup {
        floor,
        difficulty: Difficulty::Normal,
        stakes: Stakes::Boss,
        purse: 0,
        items: Vec::new(),
        seed: SEED,
        ambushed: false,
        haunts,
        scale: Scale::PLAIN,
        known: Vec::new(),
        every_graeae: false,
    }
}

fn against(name: &str) -> (Battle, usize) {
    let spec = bestiary::find(name).expect("in the bestiary");
    let foes = vec![Fighter::of_enemy(spec, spec.floor, Scale::PLAIN)];
    let mut battle = Battle::new(party(), foes, setup(spec.floor, Vec::new()));
    battle.begin();
    let boss = battle
        .side(Side::Foes)
        .find(|&i| battle.fighters[i].name == name)
        .expect("the boss stands");
    (battle, boss)
}

fn fish(battle: &Battle) -> usize {
    battle.side(Side::Party).next().expect("a fish")
}

fn arms(battle: &Battle, body: usize) -> usize {
    parts_of(battle, body).len()
}

#[test]
fn the_kraken_head_opens_once_two_arms_are_cut() {
    let (mut battle, kraken) = against("The Kraken");
    let fish = fish(&battle);
    assert_eq!(arms(&battle, kraken), TENTACLES as usize);
    assert!(!battle.targetable(fish, kraken));
    let tentacles = parts_of(&battle, kraken);
    battle.beat(tentacles[0]);
    assert!(!battle.targetable(fish, kraken));
    battle.beat(tentacles[1]);
    assert!(battle.targetable(fish, kraken));
    battle.beat(kraken);
    assert_eq!(arms(&battle, kraken), 0, "the arms go with the head");
}

#[test]
fn six_ribs_turn_away_six_hits() {
    let (mut battle, bones) = against("Leviathan Bones");
    let fish = fish(&battle);
    let full = battle.fighters[bones].hp;
    for _ in 0..RIBS {
        assert_eq!(battle.harm(fish, bones, 50, DamageKind::Skill), 0);
    }
    assert_eq!(battle.fighters[bones].hp, full);
    assert!(battle.harm(fish, bones, 50, DamageKind::Skill) > 0);
}

#[test]
fn the_choir_falls_silent_under_six_voices() {
    let (mut battle, choir) = against("The Choir");
    let fish = fish(&battle);
    assert!(!battle.targetable(fish, choir));
    let voices = parts_of(&battle, choir);
    assert_eq!(voices.len(), VOICES as usize);
    let silenced = VOICES - VOICES_TO_SING + 1;
    for &voice in voices.iter().take(silenced as usize - 1) {
        battle.beat(voice);
    }
    assert!(!battle.fighters[choir].gone);
    battle.beat(voices[silenced as usize - 1]);
    assert!(battle.fighters[choir].gone);
}

#[test]
fn the_lich_rises_while_its_phylactery_stands() {
    let (mut battle, lich) = against("The Lich King");
    battle.beat(lich);
    assert!(!battle.fighters[lich].gone);
    assert_eq!(
        battle.fighters[lich].hp,
        battle.fighters[lich].max_hp / HALF
    );
    let jar = parts_of(&battle, lich)[0];
    battle.beat(jar);
    battle.beat(lich);
    assert!(battle.fighters[lich].gone);
}

#[test]
fn the_daemon_falls_only_when_both_processes_fall() {
    let (mut battle, daemon) = against("The Daemon");
    let fork = super::super::skill::Skill::new(
        "Fork",
        0,
        crate::dungeon::effect::Reach::Me,
        &[crate::dungeon::effect::Effect::Special(
            crate::dungeon::effect::Special::Fork,
        )],
        "",
    );
    battle.use_skill(daemon, &fork, None);
    let twin = battle
        .side(Side::Foes)
        .find(|&i| i != daemon && battle.fighters[i].has(Trait::Background))
        .expect("a second process");
    battle.beat(daemon);
    assert!(!battle.fighters[daemon].gone);
    assert!(battle.fighters[daemon].belly_up);
    battle.beat(twin);
    assert!(battle.fighters[twin].gone);
    assert!(battle.fighters[daemon].gone);
}

#[test]
fn the_devil_offers_a_contract_at_each_third() {
    let (mut battle, devil) = against("The Devil");
    let fish = fish(&battle);
    battle.haul = HAUL;
    let third = battle.fighters[devil].max_hp / u32::from(DEVIL_THIRDS) + 1;
    battle.harm(fish, devil, third, DamageKind::True);
    assert_eq!(battle.offer, Some(devil));
    let hp = battle.fighters[devil].hp;
    answer(&mut battle, true);
    assert_eq!(battle.fighters[devil].hp, hp * HALF);
    assert_eq!(battle.haul, HAUL * u64::from(HALF));
    assert_eq!(battle.offer, None);
}

#[test]
fn an_offer_waits_for_the_player() {
    let (mut battle, devil) = against("The Devil");
    let fish = fish(&battle);
    while battle.actor().is_none() && battle.outcome().is_none() {
        battle.choose(Choice::Guard);
    }
    let third = battle.fighters[devil].max_hp / u32::from(DEVIL_THIRDS) + 1;
    battle.fighters[devil].hp = battle.fighters[devil].max_hp;
    battle.harm(fish, devil, third, DamageKind::True);
    battle.choose(Choice::Guard);
    assert_eq!(battle.offer(), Some(devil));
    assert!(battle.choose(Choice::Guard).is_empty());
    battle.answer(false);
    assert_eq!(battle.offer(), None);
}

#[test]
fn mammon_eats_the_haul_and_gives_it_back_when_beaten() {
    let (mut battle, mammon) = against("Mammon");
    battle.haul = HAUL;
    let bite = battle.fighters[mammon].bite;
    turn_start(&mut battle, mammon);
    let eaten = HAUL * GREED_PERCENT / 100;
    assert_eq!(battle.haul, HAUL - eaten);
    assert_eq!(battle.fighters[mammon].bite, bite + GREED_BITE);
    battle.beat(mammon);
    assert!(battle.haul >= HAUL);
}

#[test]
fn the_last_trump_leaves_the_marked_at_one() {
    let (mut battle, herald) = against("The Herald");
    let fish = fish(&battle);
    let member = battle.fighters[fish].member().expect("a fish");
    battle.party[member].devil_marked = true;
    for _ in 0..5 {
        turn_start(&mut battle, herald);
    }
    assert_eq!(battle.fighters[fish].hp, 1);
}

#[test]
fn the_great_eye_is_untouchable_until_the_prayer_is_typed() {
    let (mut battle, eye) = against("The Great Eye");
    let fish = fish(&battle);
    assert!(!battle.targetable(fish, eye));
    assert!(!pray(&mut battle, "wrong words"));
    let line = battle.prayer.line().expect("a line");
    assert!(pray(&mut battle, &line.to_lowercase()));
    assert!(battle.targetable(fish, eye));
    turn_ended(&mut battle, eye);
    assert!(!battle.targetable(fish, eye));
}

#[test]
fn the_last_word_opens_the_eye_for_good() {
    let (mut battle, eye) = against("The Great Eye");
    let fish = fish(&battle);
    while let Some(line) = battle.prayer.line() {
        assert!(pray(&mut battle, line));
        turn_ended(&mut battle, eye);
    }
    assert_eq!(battle.prayer.said, battle.prayer.lines.len());
    assert!(battle.targetable(fish, eye));
}

#[test]
fn the_eye_closes_once_and_the_unlifed_are_born_again() {
    let (mut battle, eye) = against("The Great Eye");
    battle.summon(eye, "Unborn", 1);
    let unborn = battle
        .side(Side::Foes)
        .find(|&i| battle.fighters[i].name == "Unborn")
        .expect("summoned");
    battle.fighters[unborn]
        .traits
        .retain(|t| *t != Trait::Rebirth);
    battle.beat(unborn);
    assert!(battle.fighters[unborn].gone);
    battle.beat(eye);
    assert!(!battle.fighters[eye].gone);
    assert!(!battle.fighters[unborn].gone);
    battle.beat(eye);
    assert!(battle.fighters[eye].gone);
}

#[test]
fn a_ghost_rises_only_from_your_own_dead() {
    let spec = bestiary::find("The Lich King").expect("in the bestiary");
    let mut battle = Battle::new(
        party(),
        vec![Fighter::of_enemy(spec, spec.floor, Scale::PLAIN)],
        setup(spec.floor, Vec::new()),
    );
    battle.begin();
    let lich = battle.side(Side::Foes).next().expect("the lich");
    let before = battle.fighters.len();
    battle.raise(lich, Haunt::Dead);
    assert_eq!(battle.fighters.len(), before);
    let mut rng = SmallRng::seed_from_u64(SEED);
    let dead = Fish::new(FishSpecies::Koi, "Abel".into(), 0.0, 0.0, &mut rng);
    battle.setup.haunts.push(Ghost::of(&dead, Haunt::Dead));
    battle.raise(lich, Haunt::Dead);
    let ghost = battle.fighters.last().expect("a ghost");
    assert_eq!(ghost.name, "Ghost of your dead Abel");
    assert_eq!(ghost.max_hp, crate::dungeon::kit::Kit::of(&dead).vigor);
}

#[test]
fn the_glow_cannot_be_bitten() {
    let (mut battle, glow) = against("The Glow");
    let fish = fish(&battle);
    let full = battle.fighters[glow].hp;
    for _ in 0..20 {
        battle.bite(fish, glow);
    }
    assert_eq!(battle.fighters[glow].hp, full);
}

#[test]
fn the_court_swims_beside_the_millalobo() {
    let (battle, _) = against("The Millalobo");
    let sharks = battle
        .side(Side::Foes)
        .filter(|&i| battle.fighters[i].base == COURTIER)
        .count();
    assert_eq!(sharks, usize::from(COURTIERS));
}

#[test]
fn the_candy_leviathan_sheds_its_layers() {
    let (mut battle, leviathan) = against("The Candy Leviathan");
    let fish = fish(&battle);
    let hard = battle.fighters[leviathan].armour;
    assert_eq!(hard, HARD_CANDY);
    let max = battle.fighters[leviathan].max_hp;
    battle.harm(fish, leviathan, max / 2, DamageKind::True);
    assert_eq!(battle.fighters[leviathan].armour, 0);
    let foes = battle.side(Side::Foes).count();
    battle.harm(fish, leviathan, max / 3, DamageKind::True);
    assert_eq!(
        battle.side(Side::Foes).count(),
        foes + 1,
        "the gummy core splits"
    );
}
