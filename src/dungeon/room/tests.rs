use rand::SeedableRng;
use rand::rngs::SmallRng;

use super::*;
use crate::dungeon::bestiary;
use crate::dungeon::event::EVENTS;
use crate::dungeon::expedition::{Expedition, Ground, Next, Wound};
use crate::dungeon::items::{Item, Pack};
use crate::dungeon::map::{Room, RoomKind};
use crate::dungeon::sim::Tier;
use crate::dungeon::{Above, Difficulty};

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
                let Some(panel) = expedition.panel(step, &ground.above()) else {
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
    let panel = expedition.panel(0, &Above::default()).expect("a panel");
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
        e.panel(0, &Above::default())
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
            for outcome in [choice.pass, choice.fail].iter().flat_map(|t| t.outcomes()) {
                if let Outcome::Fight { enemy, .. } = outcome {
                    assert!(bestiary::find(enemy).is_some(), "{}: {enemy}", event.name);
                }
                if let Outcome::Waits(boss) = outcome {
                    let bosses = FloorSpec::of(event.floor).bosses();
                    assert!(
                        bosses.iter().any(|spec| spec.name == boss),
                        "{}: {boss} guards no elevator on its floor",
                        event.name
                    );
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
    let panel = expedition.panel(0, &Above::default()).expect("a panel");
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

fn one_fish(species: FishSpecies, seed: u64) -> Expedition {
    let fish = Fish::new(
        species,
        "Solo".to_string(),
        0.0,
        0.0,
        &mut SmallRng::seed_from_u64(seed),
    );
    let pack = vec![
        Pack::of(Item::Food, 1),
        Pack::of(Item::Stock(StockItem::Junk), 1),
    ];
    Expedition::set_out(
        vec![fish],
        vec!["Fishtank".to_string()],
        Difficulty::Normal,
        pack,
        seed,
        &Ground::default(),
    )
}

fn at_event(expedition: &mut Expedition, depth: u32, name: &str, ground: &Ground) {
    expedition.floor = depth;
    let room = Room {
        kind: RoomKind::Event,
        lane: 2,
        seed: expedition.route.seed ^ u64::from(depth),
    };
    let mut visit = Visit::enter(expedition, 1, 0, room, ground);
    visit.event = Some(name.to_string());
    visit.offers.clear();
    if event::find(name).is_some_and(|e| e.calls_the_dead()) {
        visit.offers = ground.haunts.iter().cloned().map(Offer::Dead).collect();
    }
    expedition.visit = Some(visit);
}

fn ghost(
    name: &str,
    sin: Option<crate::fishes::species::Sin>,
    fate: crate::fishes::graveyard::Fate,
) -> Ghost {
    Ghost {
        name: name.to_string(),
        haunt: Haunt::Dead,
        vigor: 40,
        bite: 6,
        speed: 6,
        sin,
        fate: Some(fate),
    }
}

#[test]
fn every_floor_has_three_events_of_its_own() {
    for depth in 1..=floor::LAST_FLOOR {
        assert_eq!(event::of_floor(depth).len(), 3, "floor {depth}");
    }
    let names: std::collections::BTreeSet<&str> = EVENTS.iter().map(|e| e.name).collect();
    assert_eq!(names.len(), EVENTS.len(), "two events share a name");
}

#[test]
fn every_event_ends_on_a_way_out_that_needs_nothing() {
    for event in EVENTS {
        let last = event.choices.last().expect("a choice");
        assert_eq!(last.need, Need::Nothing, "{}", event.name);
        assert_eq!(last.check, None, "{}", event.name);
        assert_eq!(
            last.pass.outcomes(),
            [Outcome::Nothing; 2],
            "{}",
            event.name
        );
    }
}

#[test]
fn the_seance_calls_your_dead_and_the_sold_remember() {
    let ground = Ground {
        haunts: vec![
            ghost("Abel", None, crate::fishes::graveyard::Fate::Sold),
            ghost(
                "Nemo",
                Some(crate::fishes::species::Sin::Greed),
                crate::fishes::graveyard::Fate::Dive,
            ),
        ],
        ..Ground::default()
    };
    let mut expedition = below(7, Difficulty::Normal);
    at_event(&mut expedition, 4, "The Séance Table", &ground);
    let panel = expedition.panel(0, &ground.above()).expect("a panel");
    let labels: Vec<&str> = panel.rows.iter().map(|row| row.label.as_str()).collect();
    assert_eq!(labels, ["Call Abel", "Call Nemo", "Blow out the candles"]);
    assert_eq!(panel.rows[0].note, "sold");
    expedition.act(Act::Call(0), 0, &ground);
    let visit = expedition.visit.as_ref().expect("still in the room");
    assert_eq!(visit.stage, Stage::Sprung);
    assert_eq!(visit.foes[0].enemy, crate::dungeon::specials::GHOST);
    assert_eq!(
        visit.foes[0].ghost.as_ref().map(|g| g.name.as_str()),
        Some("Abel")
    );

    let mut expedition = below(7, Difficulty::Normal);
    at_event(&mut expedition, 4, "The Séance Table", &ground);
    let gold = expedition.haul.gold;
    expedition.act(Act::Call(1), 0, &ground);
    let visit = expedition.visit.as_ref().expect("still in the room");
    assert_eq!(visit.stage, Stage::Spoils);
    assert!(
        expedition.haul.gold > gold,
        "a greedy ghost tells where its gold is"
    );
}

#[test]
fn the_seance_with_nobody_dead_offers_only_the_candles() {
    let mut expedition = below(8, Difficulty::Normal);
    at_event(&mut expedition, 4, "The Séance Table", &Ground::default());
    let panel = expedition.panel(0, &Above::default()).expect("a panel");
    let labels: Vec<&str> = panel.rows.iter().map(|row| row.label.as_str()).collect();
    assert_eq!(labels, ["Blow out the candles"]);
}

#[test]
fn a_snapped_clam_holds_its_fish_at_the_start_of_the_next_fight() {
    let mut expedition = one_fish(FishSpecies::Anchoveta, 9);
    expedition.haul.gold = RICH;
    at_event(&mut expedition, 2, "A Giant Pearl", &Ground::default());
    expedition.act(Act::Choice(0), 0, &Ground::default());
    let held = expedition
        .lingering
        .iter()
        .any(|l| l.fish == "Solo" && l.kind == crate::dungeon::effect::StatusKind::Held);
    assert!(held, "a weak bite cannot pry the pearl loose");
    expedition.visit = None;
    let foes = floor::encounter(2, floor::scale_for(2), &[], &mut SmallRng::seed_from_u64(9));
    expedition.start_fight(
        foes.iter().filter_map(Foe::of).collect(),
        crate::dungeon::battle::Stakes::Encounter,
        false,
        &Ground::default(),
    );
    assert!(
        expedition.lingering.is_empty(),
        "the hold is spent on one fight"
    );
    let (battle, _) = expedition.battle().expect("a fight");
    let fish = battle
        .fighters
        .iter()
        .find(|f| f.member() == Some(0))
        .expect("the fish");
    let still_held = fish.has_status(crate::dungeon::effect::StatusKind::Held);
    let was_held = battle
        .log
        .iter()
        .any(|line| line.text == "Solo is held and cannot act.");
    assert!(still_held || was_held, "it starts the fight held");
}

#[test]
fn a_crop_circle_beams_up_a_fish_that_never_saw_the_sky() {
    let mut expedition = one_fish(FishSpecies::Merluza, 10);
    at_event(&mut expedition, 5, "A Crop Circle", &Ground::default());
    let next = expedition.act(Act::Choice(0), 0, &Ground::default());
    assert_eq!(next, Next::Home, "the only fish is gone, so the dive ends");
    assert!(expedition.party.is_empty());
    assert_eq!(expedition.beamed.len(), 1);
    assert!(!expedition.wiped, "an abduction is not a wipe");
}

#[test]
fn an_offering_too_small_brings_the_jelly_and_spends_what_was_offered() {
    let mut expedition = one_fish(FishSpecies::Merluza, 11);
    at_event(&mut expedition, 3, "The Sugar Altar", &Ground::default());
    expedition.act(Act::Choice(0), 0, &Ground::default());
    let visit = expedition.visit.as_ref().expect("still in the room");
    assert_eq!(visit.stage, Stage::Sprung);
    assert_eq!(visit.foes[0].enemy, "Cotton Candy Jelly");
    let meals = expedition
        .pack
        .iter()
        .find(|p| p.item == Item::Food)
        .map_or(0, |p| p.count);
    assert_eq!(meals, 0, "the one meal went on the altar");
}

#[test]
fn nailing_a_coffin_shut_spends_the_junk() {
    let mut expedition = one_fish(FishSpecies::Merluza, 12);
    at_event(&mut expedition, 4, "A Coffin", &Ground::default());
    let panel = expedition.panel(0, &Above::default()).expect("a panel");
    let nail = panel
        .rows
        .iter()
        .find(|row| row.label == "Nail it shut")
        .expect("Junk is packed");
    assert_eq!(nail.tag.as_deref(), Some("Junk"));
    expedition.act(nail.act, 0, &Ground::default());
    let junk = expedition
        .pack
        .iter()
        .find(|p| p.item == Item::Stock(StockItem::Junk))
        .map_or(0, |p| p.count);
    assert_eq!(junk, 0);
}

#[test]
fn the_grave_bears_the_name_of_a_fish_in_the_party() {
    let mut expedition = below(13, Difficulty::Normal);
    at_event(&mut expedition, 4, "A Fresh Grave", &Ground::default());
    let panel = expedition.panel(0, &Above::default()).expect("a panel");
    let drawn = expedition.drawn(expedition.visit.as_ref().expect("a room"));
    let name = &expedition.party[drawn].name;
    assert!(panel.text[0].text.ends_with(&format!("{name}.")));
}

#[test]
fn a_fish_that_looks_into_its_own_grave_and_fails_comes_out_doomed() {
    for seed in 0..SEEDS {
        let mut expedition = below(seed, Difficulty::Normal);
        at_event(&mut expedition, 4, "A Fresh Grave", &Ground::default());
        let drawn = expedition.drawn(expedition.visit.as_ref().expect("a room"));
        let gold = expedition.haul.gold;
        expedition.act(Act::Choice(0), 0, &Ground::default());
        if expedition.haul.gold > gold {
            continue;
        }
        let kit = Kit::of(&expedition.party[drawn]);
        assert!(kit.traits.contains(&Trait::Doomed));
        return;
    }
    panic!("no seed failed the look");
}

fn panel_labels(expedition: &Expedition) -> Vec<String> {
    expedition
        .panel(0, &Above::default())
        .expect("a panel")
        .rows
        .iter()
        .map(|row| row.label.clone())
        .collect()
}

fn mutations_of(fish: &Fish) -> usize {
    crate::fishes::mutations::Mutatable::record(fish).map_or(0, |record| record.history.len())
}

#[test]
fn dipping_one_fish_in_the_glowing_pool_changes_the_fish_you_chose() {
    let mut expedition = below(14, Difficulty::Normal);
    at_event(&mut expedition, 6, "A Glowing Pool", &Ground::default());
    let panel = expedition.panel(1, &Above::default()).expect("a panel");
    assert!(panel.which_fish, "the pool asks which fish goes in");
    let before: Vec<usize> = expedition.party.iter().map(mutations_of).collect();
    expedition.act(Act::Choice(0), 1, &Ground::default());
    let after: Vec<usize> = expedition.party.iter().map(mutations_of).collect();
    assert_eq!(after[1], before[1] + 1, "the chosen fish changed");
    assert_eq!(after[0], before[0]);
    assert_eq!(after[2], before[2]);
}

#[test]
fn the_line_of_prayer_is_said_and_a_wrong_word_becomes_a_curse() {
    let ground = Ground::default();
    let mut expedition = below(15, Difficulty::Normal);
    at_event(&mut expedition, 10, "A Line of Prayer", &ground);
    let panel = expedition.panel(0, &Above::default()).expect("a panel");
    let pray = &panel.rows[0];
    assert_eq!(pray.act, Act::Pray, "the row opens the words field");
    let rolls = expedition.visit.as_ref().expect("a room").rolls;
    assert_eq!(expedition.act(pray.act, 0, &ground), Next::Stay);
    assert_eq!(
        expedition.visit.as_ref().expect("a room").rolls,
        rolls,
        "opening the field spends no roll"
    );
    let seed = expedition.visit.as_ref().expect("a room").seed;
    let line = event::prayer_line(seed);
    let treasures = expedition.haul.count();
    expedition.pray(&line.to_lowercase(), &ground);
    assert!(expedition.haul.count() > treasures, "the Void gives");

    let mut expedition = below(15, Difficulty::Normal);
    at_event(&mut expedition, 10, "A Line of Prayer", &ground);
    let lead = expedition.lead();
    expedition.pray("not the words", &ground);
    let kit = Kit::of(&expedition.party[lead]);
    let cursed = expedition.party[lead]
        .deep
        .as_ref()
        .is_some_and(|life| life.curses.contains(&Curse::WrongWord));
    assert!(cursed, "{:?}", kit.traits);
}

#[test]
fn an_off_key_voice_brings_the_choir_to_the_elevator() {
    for seed in 0..SEEDS {
        let mut expedition = one_fish(FishSpecies::Merluza, seed);
        at_event(&mut expedition, 9, "Choir Practice", &Ground::default());
        expedition.act(Act::Choice(0), 0, &Ground::default());
        if expedition.guard.is_none() {
            continue;
        }
        for dive in 0..2 {
            expedition.dive = dive;
            assert_eq!(expedition.boss().name, "The Choir");
        }
        expedition.descend(&Ground::default());
        assert_eq!(expedition.guard, None, "the next floor has its own boss");
        return;
    }
    panic!("no seed sang off key");
}

#[test]
fn the_gatekeeper_keeps_the_sinful_outside_the_next_fight() {
    let mut expedition = one_fish(FishSpecies::Cashfish, 16);
    let saint = Fish::new(
        FishSpecies::Merluza,
        "Saint".to_string(),
        0.0,
        0.0,
        &mut SmallRng::seed_from_u64(16),
    );
    expedition.join(saint);
    let sinful: Vec<bool> = expedition.kits().iter().map(event::sinful).collect();
    assert_eq!(sinful, [true, false], "a Cashfish is greedy");
    at_event(&mut expedition, 9, "The Gatekeeper", &Ground::default());
    expedition.act(Act::Choice(0), 0, &Ground::default());
    let outside: Vec<&str> = expedition
        .lingering
        .iter()
        .filter(|l| l.kind == crate::dungeon::effect::StatusKind::Away)
        .map(|l| l.fish.as_str())
        .collect();
    assert_eq!(outside, ["Solo"]);
}

#[test]
fn swimming_the_burning_river_can_mark_every_swimmer() {
    for seed in 0..SEEDS {
        let mut expedition = below(seed, Difficulty::Normal);
        at_event(&mut expedition, 8, "The Ferry", &Ground::default());
        expedition.act(Act::Choice(1), 0, &Ground::default());
        let marked = expedition.party.iter().filter(|f| f.devil_marked).count();
        if marked == 0 {
            continue;
        }
        let markable = expedition
            .party
            .iter()
            .filter(|f| f.species.config().markable)
            .count();
        assert_eq!(marked, markable);
        assert!(expedition.haul.count() > 0, "they still reach the far bank");
        return;
    }
    panic!("no seed marked the swimmers");
}

#[test]
fn phishing_takes_gold_out_of_the_haul_and_never_below_nothing() {
    for seed in 0..SEEDS {
        let mut expedition = below(seed, Difficulty::Normal);
        expedition.haul.gold = 1;
        at_event(&mut expedition, 7, "An Email", &Ground::default());
        expedition.act(Act::Choice(0), 0, &Ground::default());
        if expedition.haul.gold == 0 {
            return;
        }
    }
    panic!("no seed was phished");
}

#[test]
fn the_feather_weighs_a_fish_against_the_weight_it_was_born_with() {
    let ground = Ground::default();
    let mut light = one_fish(FishSpecies::Merluza, 17);
    at_event(&mut light, 9, "The Scales", &ground);
    light.act(Act::Choice(0), 0, &ground);
    assert!(
        light.haul.count() > 0,
        "an unfed fish weighs no more than the feather"
    );

    let mut heavy = one_fish(FishSpecies::Merluza, 17);
    heavy.party[0].weight_g += 1;
    at_event(&mut heavy, 9, "The Scales", &ground);
    heavy.act(Act::Choice(0), 0, &ground);
    let wanting = heavy.party[0]
        .deep
        .as_ref()
        .is_some_and(|life| life.curses.contains(&Curse::Wanting));
    assert!(wanting);
}

#[test]
fn the_captcha_asks_for_eyes_and_the_cheatfish_needs_none() {
    let mut expedition = one_fish(FishSpecies::Blindfish, 18);
    at_event(
        &mut expedition,
        7,
        "Select Every Square With a Fish",
        &Ground::default(),
    );
    assert!(!panel_labels(&expedition).contains(&"Let the Cheatfish do it".to_string()));
    expedition.act(Act::Choice(0), 0, &Ground::default());
    let visit = expedition.visit.as_ref().expect("still in the room");
    assert_eq!(visit.stage, Stage::Sprung, "locked out: the long way round");
}

#[test]
fn the_blind_deep_keeps_the_eyes_events() {
    let deep = floor::LAST_FLOOR + 3;
    let names: Vec<&str> = event::of_floor(deep).iter().map(|e| e.name).collect();
    let eye: Vec<&str> = event::of_floor(floor::LAST_FLOOR)
        .iter()
        .map(|e| e.name)
        .collect();
    assert_eq!(names, eye);
}

fn won_against(expedition: &mut Expedition, enemy: &str) {
    expedition.fight = Some(crate::dungeon::expedition::FightRecord {
        seed: 1,
        stakes: crate::dungeon::battle::Stakes::Boss,
        purse: 0,
        foes: vec![Foe {
            enemy: enemy.to_string(),
            ghost: None,
        }],
        haunts: Vec::new(),
        inputs: Vec::new(),
        spent: 0,
        known: Vec::new(),
        every_graeae: false,
        ambushed: false,
        lingering: Vec::new(),
    });
    let aftermath = crate::dungeon::battle::Aftermath {
        outcome: crate::dungeon::battle::Outcome::Won,
        party: expedition.party.clone(),
        cows: Vec::new(),
        dead: Vec::new(),
        hp: Vec::new(),
        haul: 0,
        junk_found: 0,
        purse_spent: 0,
        items: expedition.pack.clone(),
        beaten: 1,
        spoils: Vec::new(),
    };
    expedition.settle(aftermath);
}

fn found(expedition: &Expedition, treasure: crate::dungeon::treasure::Treasure) -> u32 {
    let stock = haul::treasure_stock(treasure);
    expedition
        .haul
        .treasures
        .iter()
        .filter(|stack| stack.stock == stock)
        .map(|stack| stack.count)
        .sum()
}

#[test]
fn a_boss_pays_its_named_treasures() {
    use crate::dungeon::boon::Charm;
    use crate::dungeon::treasure::Treasure;
    let mut expedition = below(19, Difficulty::Normal);
    expedition.haul = Haul::default();
    won_against(&mut expedition, "The Kraken");
    assert_eq!(found(&expedition, Treasure::KrakenInk), 1);
    assert_eq!(found(&expedition, Treasure::Charm(Charm::KrakensBeak)), 1);

    won_against(&mut expedition, "The Great Eye");
    assert_eq!(
        found(&expedition, Treasure::WishShard),
        crate::dungeon::treasure::SHARDS_PER_WISH,
        "a wish is three shards"
    );
}

#[test]
fn phoenix_roe_is_paid_only_on_hardcore() {
    use crate::dungeon::treasure::Treasure;
    let mut normal = below(20, Difficulty::Normal);
    won_against(&mut normal, "The Lich King");
    assert_eq!(found(&normal, Treasure::PhoenixRoe), 0);
    let mut hardcore = below(20, Difficulty::Hardcore);
    won_against(&mut hardcore, "The Lich King");
    assert_eq!(found(&hardcore, Treasure::PhoenixRoe), 1);
}

#[test]
fn every_boss_drop_is_a_treasure_the_dice_can_roll() {
    let mut rng = SmallRng::seed_from_u64(21);
    for spec in bestiary::all() {
        for loot in spec.loot {
            if let bestiary::Loot::Named(find) | bestiary::Loot::Hardcore(find) = loot {
                let (treasure, count) = find.roll(&mut rng);
                assert!(count > 0, "{}", spec.name);
                assert!(!treasure.name().is_empty());
            }
        }
    }
}
