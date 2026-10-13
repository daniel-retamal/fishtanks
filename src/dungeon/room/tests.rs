use rand::SeedableRng;
use rand::rngs::SmallRng;

use super::*;
use crate::dungeon::Difficulty;
use crate::dungeon::bestiary;
use crate::dungeon::event::EVENTS;
use crate::dungeon::expedition::{Expedition, Ground, Next, Wound};
use crate::dungeon::items::{Item, Pack};
use crate::dungeon::map::{Room, RoomKind};
use crate::dungeon::sim::Tier;

const SEEDS: u64 = 60;
const RICH: u64 = 1_000_000;
const ACTS: usize = 12;
const SAMPLE: u64 = 2_000;

fn party(seed: u64) -> Vec<Fish> {
    Tier::Starter.party(&mut SmallRng::seed_from_u64(seed))
}

fn below(seed: u64, difficulty: Difficulty) -> Expedition {
    let party = party(seed);
    let homes = vec!["Fishtank".to_string(); party.len()];
    let pack = vec![
        Pack::of(Item::Food, 4),
        Pack::of(Item::Stock(StockItem::BAIT), 1),
    ];
    let mut expedition =
        Expedition::set_out(party, homes, difficulty, pack, seed, &Ground::default());
    expedition.haul.gold = RICH;
    expedition
}

fn enter(expedition: &mut Expedition, kind: RoomKind, seed: u64) {
    let room = Room {
        kind,
        lane: 2,
        seed,
    };
    expedition.visit = Some(Visit::enter(expedition, 1, 0, room, &Ground::default()));
}

fn hp(expedition: &Expedition) -> Vec<u32> {
    let kits = expedition.kits();
    (0..expedition.party.len())
        .map(|member| expedition.hp(member, &kits[member]))
        .collect()
}

const SERVICES: [RoomKind; 7] = [
    RoomKind::School,
    RoomKind::Shrine,
    RoomKind::Altar,
    RoomKind::Laboratory,
    RoomKind::Vent,
    RoomKind::Cleaning,
    RoomKind::Vending,
];

#[test]
fn a_service_room_never_turns_bad() {
    for kind in SERVICES {
        assert!(kind.is_service());
        for seed in 0..SEEDS {
            let mut expedition = below(seed, Difficulty::Hardcore);
            expedition.wounds[0] = Wound {
                hp: Some(1),
                belly_up: false,
            };
            enter(&mut expedition, kind, seed);
            let ground = Ground::default();
            for step in 0..ACTS {
                let Some(panel) = expedition.panel(step, &ground.wrecks) else {
                    break;
                };
                let Some(row) = panel
                    .rows
                    .iter()
                    .find(|row| row.usable && !matches!(row.act, Act::Leave | Act::MoveOn))
                else {
                    break;
                };
                let before = hp(&expedition);
                let party = expedition.party.len();
                let next = expedition.act(row.act, step, &ground);
                assert!(
                    !matches!(next, Next::Fight | Next::Home),
                    "{kind:?} turned on the party"
                );
                assert!(expedition.fight.is_none(), "{kind:?} started a fight");
                assert!(expedition.party.len() >= party, "{kind:?} took a fish");
                for (old, new) in before.iter().zip(hp(&expedition)) {
                    assert!(new >= *old, "{kind:?} hurt a fish");
                }
            }
        }
    }
}

#[test]
fn luck_makes_a_mimic_rarer_and_doom_more_common() {
    let room = |seed| Room {
        kind: RoomKind::Chest,
        lane: 0,
        seed,
    };
    let count = |kits: &[Kit]| (0..SAMPLE).filter(|&seed| hidden(room(seed), kits)).count();
    let plain = Kit::of(&party(1)[0]);
    let mut lucky = plain.clone();
    lucky.traits.push(Trait::Lucky);
    let mut doomed = plain.clone();
    doomed.traits.push(Trait::Doomed);
    doomed.traits.push(Trait::Doomed);
    let (plain, lucky, doomed) = (count(&[plain]), count(&[lucky]), count(&[doomed]));
    assert!(lucky < plain && plain < doomed, "{lucky} {plain} {doomed}");
    assert!(plain > 0);
}

#[test]
fn a_chest_is_richer_on_hardcore_with_bait_and_when_la_pincoya_faces_the_sea() {
    let normal = below(2, Difficulty::Normal);
    assert_eq!(normal.chest_tier(), Rarity::Common);
    let hardcore = below(2, Difficulty::Hardcore);
    assert_eq!(hardcore.chest_tier(), Rarity::Rare);
    let mut blessed = below(2, Difficulty::Normal);
    blessed.omens.push(event::Omen::ChestsRicher);
    assert_eq!(blessed.chest_tier(), Rarity::Rare);
    let mut cursed = below(2, Difficulty::Hardcore);
    cursed.omens.push(event::Omen::ChestsPoorer);
    assert_eq!(cursed.chest_tier(), Rarity::Common);
}

#[test]
fn opening_a_chest_with_bait_spends_the_bait() {
    for seed in 0..SEEDS {
        let mut expedition = below(seed, Difficulty::Normal);
        enter(&mut expedition, RoomKind::Chest, seed);
        let next = expedition.act(Act::OpenWithBait, 0, &Ground::default());
        assert_eq!(next, Next::Stay);
        let bait = expedition
            .pack
            .iter()
            .find(|p| p.item == Item::Stock(StockItem::BAIT))
            .map_or(0, |p| p.count);
        let stage = &expedition.visit.as_ref().expect("in the room").stage;
        match stage {
            Stage::Sprung => assert_eq!(bait, 1, "a mimic eats nothing"),
            Stage::Spoils => assert_eq!(bait, 0),
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn the_shrine_offers_revive_first_when_a_fish_fell() {
    for seed in 0..SEEDS {
        let mut expedition = below(seed, Difficulty::Normal);
        expedition.wounds[1] = Wound {
            hp: None,
            belly_up: true,
        };
        enter(&mut expedition, RoomKind::Shrine, seed);
        let visit = expedition.visit.as_ref().expect("a shrine");
        assert_eq!(
            visit.offers.first(),
            Some(&Offer::Blessing(Blessing::Revive))
        );
        assert_eq!(visit.offers.len(), BLESSINGS_OFFERED);
        expedition.act(Act::Bless(0), 0, &Ground::default());
        assert!(
            !expedition.wound(1).belly_up,
            "Revive rights the fallen fish"
        );
    }
}

#[test]
fn a_fish_that_died_on_hardcore_is_raised_through_the_app() {
    let mut expedition = below(3, Difficulty::Hardcore);
    expedition.fallen.push("Abel".to_string());
    enter(&mut expedition, RoomKind::Cleaning, 3);
    let panel = expedition.panel(0, &[]).expect("a panel");
    let bring = panel
        .rows
        .iter()
        .find(|row| row.label == "Bring back Abel")
        .expect("a way back for Abel");
    let next = expedition.act(bring.act, 0, &Ground::default());
    assert_eq!(next, Next::Raise("Abel".to_string()));
}

#[test]
fn an_event_offers_a_tagged_choice_only_to_a_party_that_can_make_it() {
    let mut expedition = below(4, Difficulty::Normal);
    enter(&mut expedition, RoomKind::Event, 4);
    expedition.visit.as_mut().expect("an event").event = Some("La Pincoya Dances".to_string());
    let labels = |e: &Expedition| -> Vec<String> {
        e.panel(0, &[])
            .expect("a panel")
            .rows
            .iter()
            .map(|row| row.label.clone())
            .collect()
    };
    assert!(!labels(&expedition).contains(&"Sing to her".to_string()));
    let parrot = Fish::new(
        FishSpecies::Parrotfish,
        "Polly".to_string(),
        0.0,
        0.0,
        &mut SmallRng::seed_from_u64(4),
    );
    expedition.join(parrot);
    assert!(labels(&expedition).contains(&"Sing to her".to_string()));
}

#[test]
fn every_event_fight_is_a_real_enemy() {
    for event in EVENTS {
        for choice in event.choices {
            for turn in [choice.pass, choice.fail] {
                if let Outcome::Fight { enemy, .. } = turn.outcome {
                    assert!(bestiary::find(enemy).is_some(), "{}: {enemy}", event.name);
                }
            }
        }
    }
    assert!(bestiary::find(MIMIC).is_some());
    assert!(bestiary::find(LURKER).is_some());
}

#[test]
fn a_hired_fish_joins_the_party_and_shares_its_home() {
    let mut expedition = below(5, Difficulty::Normal);
    enter(&mut expedition, RoomKind::School, 5);
    let before = expedition.party.len();
    let next = expedition.act(Act::Hire(0), 0, &Ground::default());
    assert_eq!(next, Next::Stay);
    assert_eq!(expedition.party.len(), before + 1);
    assert_eq!(expedition.homes.len(), expedition.party.len());
    assert_eq!(expedition.homes.last(), expedition.homes.first());
    assert_eq!(expedition.joined, 1);
    assert!(expedition.haul.gold < RICH);
}

#[test]
fn a_room_costs_the_haul_and_never_the_purse() {
    let mut expedition = below(6, Difficulty::Normal);
    expedition.haul.gold = 0;
    enter(&mut expedition, RoomKind::Cleaning, 6);
    expedition.wounds[0] = Wound {
        hp: Some(1),
        belly_up: false,
    };
    let panel = expedition.panel(0, &[]).expect("a panel");
    let clean = panel
        .rows
        .iter()
        .find(|row| row.act == Act::Clean)
        .expect("a clean");
    assert!(!clean.usable, "an empty haul buys nothing");
    assert_eq!(
        expedition.act(Act::Clean, 0, &Ground::default()),
        Next::Stay
    );
    assert_eq!(expedition.wound(0).hp, Some(1));
}

#[test]
fn a_dark_floor_can_ambush_an_encounter_but_a_lantern_lights_it() {
    let ambushed = |lantern: bool| {
        (0..SAMPLE).any(|seed| {
            let mut expedition = below(seed, Difficulty::Normal);
            expedition.floor = 4;
            if lantern {
                let lantern = Fish::new(
                    FishSpecies::Lanternfish,
                    "Lumen".to_string(),
                    0.0,
                    0.0,
                    &mut SmallRng::seed_from_u64(seed),
                );
                expedition.join(lantern);
            }
            enter(&mut expedition, RoomKind::Encounter, seed);
            expedition.visit.as_ref().is_some_and(|visit| visit.ambush)
        })
    };
    assert!(ambushed(false));
    assert!(!ambushed(true));
}

#[test]
fn rooms_and_the_map_draw_only_glyphs_the_game_already_draws() {
    let proven = crate::dungeon::glyphs::proven();
    let events = EVENTS.iter().map(|event| event.art);
    let mut strangers = std::collections::BTreeSet::new();
    for art in super::room_art::ALL.iter().copied().chain(events) {
        for row in art {
            strangers.extend(row.text.chars().filter(|glyph| !proven.contains(glyph)));
        }
    }
    let map = RoomKind::ALL
        .iter()
        .map(|kind| kind.glyph())
        .chain([crate::dungeon::map::ENTRY_GLYPH]);
    strangers.extend(map.filter(|glyph| !proven.contains(glyph)));
    assert!(
        strangers.is_empty(),
        "glyphs no tank or fish draws: {strangers:?}"
    );
}
