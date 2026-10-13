use std::path::Path;

use crossterm::event::KeyCode;
use fishtank::dungeon::Difficulty;
use fishtank::dungeon::Wreck;
use fishtank::dungeon::battle::Stakes;
use fishtank::dungeon::expedition::{Expedition, FightRecord, Ground, Route, Wound};
use fishtank::dungeon::floor;
use fishtank::dungeon::haul::Haul;
use fishtank::dungeon::items::{Item, Pack};
use fishtank::dungeon::map::{Map, RoomKind, Shape};
use fishtank::dungeon::room::{Stage, Visit};
use fishtank::dungeon::screen::View;
use fishtank::dungeon::sim::Tier;
use fishtank::fishes::fish::Fish;
use fishtank::fishes::mutations::{Mutation, apply_mutation_to_fish};
use fishtank::fishes::species::FishSpecies;
use fishtank::fishes::unfish::UnfishKind;
use fishtank::loot::StockItem;
use fishtank::testing::Tui;
use rand::SeedableRng;
use rand::rngs::SmallRng;

const SIZES: [(u16, u16); 6] = [(120, 38), (100, 30), (80, 24), (60, 18), (40, 14), (28, 10)];
const ROOM_SIZES: [(u16, u16); 7] = [
    (120, 38),
    (110, 20),
    (100, 30),
    (80, 24),
    (60, 18),
    (40, 14),
    (28, 10),
];
const SETTLE_TICKS: usize = 600;
const FIGHT_TURNS: usize = 400;
const SIZE: (u16, u16) = (120, 38);
const MORAY_SEGMENTS: usize = 8;
const CROWD_SEED: u64 = 17;
const MAP_SEEDS: u64 = 500;
const HOME: &str = "Fishtank";

fn reels() -> &'static Path {
    Path::new(env!("CARGO_TARGET_TMPDIR"))
}

fn settle(tui: &mut Tui) {
    for _ in 0..SETTLE_TICKS {
        if tui
            .app
            .dungeon_overlay()
            .is_some_and(|state| !state.is_busy())
        {
            return;
        }
        tui.tick_n(1);
    }
}

fn no_flaws(tui: &Tui) {
    assert!(
        tui.reel().flaw_report().is_empty(),
        "{}",
        tui.reel().flaw_report()
    );
}

fn view(tui: &Tui) -> &'static str {
    match tui.app.dungeon_overlay().map(|state| &state.view) {
        Some(View::Muster(_)) => "muster",
        Some(View::Map(_)) => "map",
        Some(View::Room(_)) => "room",
        Some(View::Fight(_)) => "fight",
        None => "closed",
    }
}

fn set_out(tui: &mut Tui) {
    tui.run("/dungeon");
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Down).key(KeyCode::Enter);
    tui.key(KeyCode::Down).key(KeyCode::Enter);
    tui.key(KeyCode::Tab);
    tui.key(KeyCode::Right);
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Enter);
}

fn fish_at_home(tui: &Tui) -> usize {
    tui.app.tanks.iter().map(|tank| tank.fish.len()).sum()
}

fn log_of(tui: &Tui) -> Vec<String> {
    tui.app
        .dungeon_overlay()
        .and_then(|state| state.fight())
        .map(|fight| {
            fight
                .battle
                .log
                .iter()
                .map(|line| line.text.clone())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn the_muster_on_film() {
    for (cols, rows) in SIZES {
        let mut tui = Tui::as_player(cols, rows);
        tui.film(reels(), &format!("deep-muster-{cols}x{rows}"));
        tui.run("/dungeon");
        tui.snap("the party pick");
        tui.key(KeyCode::Enter);
        tui.key(KeyCode::Down).key(KeyCode::Enter);
        tui.snap("two fish picked");
        tui.key(KeyCode::Down).key(KeyCode::Enter);
        tui.key(KeyCode::Tab);
        tui.snap("packing");
        tui.key(KeyCode::Right).key(KeyCode::Right);
        tui.snap("two meals packed");
        tui.key(KeyCode::Enter);
        tui.snap("the first elevator");
        tui.key(KeyCode::Enter);
        tui.snap("the first floor");
        tui.write_reel();
        no_flaws(&tui);
    }
}

#[test]
fn setting_out_takes_the_picked_fish_and_the_packed_meals() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    let food = tui.app.food_supply;
    set_out(&mut tui);
    assert_eq!(view(&tui), "map");
    assert_eq!(fish_at_home(&tui), 0);
    let expedition = tui.app.dungeon.expedition.as_ref().expect("a dive");
    assert_eq!(expedition.party.len(), 3);
    assert_eq!(expedition.difficulty, Difficulty::Normal);
    assert_eq!(expedition.pack, vec![Pack::of(Item::Food, 1)]);
    assert!(tui.app.food_supply < food);
}

#[test]
fn the_muster_takes_no_more_than_four_and_escape_leaves_everyone_home() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    tui.run("/dungeon");
    tui.key(KeyCode::Esc);
    assert_eq!(view(&tui), "closed");
    assert!(tui.app.dungeon.expedition.is_none());
    assert_eq!(fish_at_home(&tui), 3);
}

#[test]
fn the_hardcore_lift_chooses_the_whole_dive() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    tui.run("/dungeon");
    tui.key(KeyCode::Enter)
        .key(KeyCode::Tab)
        .key(KeyCode::Enter);
    tui.key(KeyCode::Down).key(KeyCode::Enter);
    let expedition = tui.app.dungeon.expedition.as_ref().expect("a dive");
    assert_eq!(expedition.difficulty, Difficulty::Hardcore);
    assert_eq!(expedition.party.len(), 1);
}

#[test]
fn the_map_on_film() {
    for (cols, rows) in SIZES {
        let mut tui = Tui::as_player(cols, rows);
        tui.film(reels(), &format!("deep-map-{cols}x{rows}"));
        set_out(&mut tui);
        tui.snap("the floor map");
        tui.key(KeyCode::Right);
        tui.snap("the next passage");
        tui.key(KeyCode::Char('p'));
        tui.snap("the party");
        tui.key(KeyCode::Char('i'));
        tui.snap("the items");
        tui.key(KeyCode::Esc);
        tui.write_reel();
        no_flaws(&tui);
    }
}

fn strong_party() -> Vec<Fish> {
    Tier::Bred.party(&mut SmallRng::seed_from_u64(CROWD_SEED))
}

fn diving(party: Vec<Fish>, seed: u64, ground: &Ground) -> Expedition {
    let homes = vec![HOME.to_string(); party.len()];
    let pack = vec![
        Pack::of(Item::Food, 3),
        Pack::of(Item::Stock(StockItem::BAIT), 2),
    ];
    Expedition::set_out(party, homes, Difficulty::Normal, pack, seed, ground)
}

fn at(tui: &mut Tui, mut expedition: Expedition, kind: RoomKind) {
    let ground = Ground::default();
    let room = fishtank::dungeon::map::Room {
        kind,
        lane: 2,
        seed: CROWD_SEED,
    };
    expedition.visit = Some(Visit::enter(&expedition, 1, 0, room, &ground));
    tui.app.dungeon.expedition = Some(expedition);
}

const ROOMS: [RoomKind; 13] = [
    RoomKind::Encounter,
    RoomKind::Den,
    RoomKind::Chest,
    RoomKind::BellJar,
    RoomKind::Event,
    RoomKind::School,
    RoomKind::Shrine,
    RoomKind::Altar,
    RoomKind::Laboratory,
    RoomKind::Vent,
    RoomKind::Cleaning,
    RoomKind::Vending,
    RoomKind::Elevator,
];

#[test]
fn every_room_on_film() {
    for (cols, rows) in ROOM_SIZES {
        let mut tui = Tui::as_player(cols, rows);
        tui.film(reels(), &format!("deep-rooms-{cols}x{rows}"));
        for kind in ROOMS {
            let mut party = strong_party();
            party.truncate(2);
            let mut expedition = diving(party, CROWD_SEED, &Ground::default());
            expedition.haul.gold = 5_000;
            expedition.wounds[1] = Wound {
                hp: None,
                belly_up: true,
            };
            at(&mut tui, expedition, kind);
            tui.run("/dungeon");
            tui.snap(kind.title());
            tui.key(KeyCode::Esc);
        }
        tui.write_reel();
        no_flaws(&tui);
    }
}

#[test]
fn the_rooms_gone_wrong_on_film() {
    for (cols, rows) in ROOM_SIZES {
        let mut tui = Tui::as_player(cols, rows);
        tui.film(reels(), &format!("deep-sprung-{cols}x{rows}"));
        for (kind, enemy) in [(RoomKind::Chest, "Mimic"), (RoomKind::BellJar, "Lurker")] {
            at(
                &mut tui,
                diving(strong_party(), CROWD_SEED, &Ground::default()),
                kind,
            );
            let visit = tui
                .app
                .dungeon
                .expedition
                .as_mut()
                .and_then(|e| e.visit.as_mut())
                .expect("a room");
            visit.stage = Stage::Sprung;
            visit.foes = vec![fishtank::dungeon::expedition::Foe {
                enemy: enemy.to_string(),
                ghost: None,
            }];
            visit.said = vec![fishtank::dungeon::beat::LogLine::new(
                format!("The {enemy} moves first."),
                fishtank::dungeon::beat::Tone::Hurt,
            )];
            tui.run("/dungeon");
            tui.snap(&format!("{} gone wrong", kind.title()));
            tui.key(KeyCode::Esc);
        }
        for (kind, label) in [
            (RoomKind::Chest, "Sunken Chest opened"),
            (RoomKind::BellJar, "Bell Jar broken"),
        ] {
            at(
                &mut tui,
                diving(strong_party(), CROWD_SEED, &Ground::default()),
                kind,
            );
            let visit = tui
                .app
                .dungeon
                .expedition
                .as_mut()
                .and_then(|e| e.visit.as_mut())
                .expect("a room");
            visit.stage = Stage::Spoils;
            visit.said = vec![fishtank::dungeon::beat::LogLine::new(
                "Found: $40.",
                fishtank::dungeon::beat::Tone::Gold,
            )];
            tui.run("/dungeon");
            tui.snap(label);
            tui.key(KeyCode::Esc);
        }
        tui.write_reel();
        no_flaws(&tui);
    }
}

#[test]
fn the_shallows_events_on_film() {
    for (cols, rows) in ROOM_SIZES {
        let mut tui = Tui::as_player(cols, rows);
        tui.film(reels(), &format!("deep-events-{cols}x{rows}"));
        for event in fishtank::dungeon::event::of_floor(1) {
            let expedition = diving(strong_party(), CROWD_SEED, &Ground::default());
            at(&mut tui, expedition, RoomKind::Event);
            let visit = tui
                .app
                .dungeon
                .expedition
                .as_mut()
                .and_then(|e| e.visit.as_mut())
                .expect("a room");
            visit.event = Some(event.name.to_string());
            tui.run("/dungeon");
            tui.snap(event.name);
            tui.key(KeyCode::Enter);
            tui.snap(&format!("{}: what happened", event.name));
            tui.key(KeyCode::Esc);
        }
        tui.write_reel();
        no_flaws(&tui);
    }
}

fn seed_with(kind: RoomKind) -> (u64, u8) {
    for seed in 0..MAP_SEEDS {
        let map = Map::generate(
            seed,
            Shape {
                wreck: false,
                events: true,
            },
        );
        if let Some(index) = map.rows[0].iter().position(|room| room.kind == kind) {
            return (seed, index as u8);
        }
    }
    panic!("no map opens on a {kind:?}");
}

fn walk_to(tui: &mut Tui, index: u8) {
    let ahead = tui
        .app
        .dungeon
        .expedition
        .as_ref()
        .map(|e| e.ahead())
        .unwrap_or_default();
    let steps = ahead.iter().position(|&at| at == index).expect("reachable");
    for _ in 0..steps {
        tui.key(KeyCode::Right);
    }
    tui.key(KeyCode::Enter);
}

fn dive_from(seed: u64) -> Tui {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    let expedition = diving(strong_party(), seed, &Ground::default());
    tui.app.dungeon.expedition = Some(expedition);
    tui.run("/dungeon");
    tui
}

#[test]
fn a_room_is_entered_from_the_map_and_the_map_waits_after_it() {
    let (seed, index) = seed_with(RoomKind::Chest);
    let mut tui = dive_from(seed);
    assert_eq!(view(&tui), "map");
    walk_to(&mut tui, index);
    assert_eq!(view(&tui), "room");
    tui.screen().expect_find("Sunken Chest");
    tui.select("Leave it");
    tui.key(KeyCode::Enter);
    assert_eq!(view(&tui), "map");
    let route = &tui.app.dungeon.expedition.as_ref().expect("a dive").route;
    assert_eq!(route.path, vec![index]);
    assert!(route.done);
}

#[test]
fn a_fight_from_the_map_plays_out_and_returns_to_it() {
    let (seed, index) = seed_with(RoomKind::Encounter);
    let mut tui = dive_from(seed);
    tui.film(reels(), "deep-encounter");
    walk_to(&mut tui, index);
    tui.snap("an encounter");
    tui.key(KeyCode::Enter);
    assert_eq!(view(&tui), "fight");
    for _ in 0..FIGHT_TURNS {
        if view(&tui) != "fight" {
            break;
        }
        settle(&mut tui);
        tui.snap("a turn");
        tui.key(KeyCode::Enter).key(KeyCode::Enter);
    }
    tui.snap("after the fight");
    tui.write_reel();
    assert!(matches!(view(&tui), "room" | "map"), "{}", view(&tui));
    let expedition = tui.app.dungeon.expedition.as_ref().expect("still below");
    assert!(expedition.fight.is_none());
    assert!(expedition.haul.gold > 0, "a won encounter pays");
}

#[test]
fn leaving_the_deep_keeps_the_dive_and_dungeon_resumes_it() {
    let (seed, index) = seed_with(RoomKind::Encounter);
    let mut tui = dive_from(seed);
    let home = fish_at_home(&tui);
    walk_to(&mut tui, index);
    tui.key(KeyCode::Enter);
    settle(&mut tui);
    tui.key(KeyCode::Enter).key(KeyCode::Enter);
    settle(&mut tui);
    let before = log_of(&tui);
    tui.key(KeyCode::Esc);
    assert_eq!(view(&tui), "closed");
    assert_eq!(fish_at_home(&tui), home, "the party waits below");
    tui.run("/dungeon");
    assert_eq!(log_of(&tui), before);
}

#[test]
fn a_dive_survives_a_save() {
    let (seed, index) = seed_with(RoomKind::Encounter);
    let mut tui = dive_from(seed);
    walk_to(&mut tui, index);
    tui.key(KeyCode::Enter);
    settle(&mut tui);
    tui.key(KeyCode::Enter).key(KeyCode::Enter);
    settle(&mut tui);
    let before = log_of(&tui);
    tui.key(KeyCode::Esc);
    let text = tui.app.snapshot().to_ron().expect("a save");
    let save = fishtank::app::SaveFile::from_ron(&text).expect("it reads back");
    let mut back = Tui::resumed(save, SIZE.0, SIZE.1);
    back.run("/dungeon");
    assert_eq!(log_of(&back), before);
    let route = back
        .app
        .dungeon
        .expedition
        .as_ref()
        .expect("a dive")
        .route
        .clone();
    assert_eq!(route.path, vec![index]);
}

#[test]
fn a_room_survives_a_save() {
    let (seed, index) = seed_with(RoomKind::Chest);
    let mut tui = dive_from(seed);
    walk_to(&mut tui, index);
    tui.key(KeyCode::Esc);
    let text = tui.app.snapshot().to_ron().expect("a save");
    let save = fishtank::app::SaveFile::from_ron(&text).expect("it reads back");
    let mut back = Tui::resumed(save, SIZE.0, SIZE.1);
    back.run("/dungeon");
    assert_eq!(view(&back), "room");
    back.screen().expect_find("Sunken Chest");
}

fn at_the_elevator(tui: &mut Tui, haul: Haul) {
    let mut expedition = diving(strong_party(), CROWD_SEED, &Ground::default());
    expedition.haul = haul;
    expedition.route.path = vec![0; fishtank::dungeon::map::LAYERS + 1];
    expedition.route.done = true;
    expedition.visit = Some(Visit::elevator(expedition.route.path.len(), CROWD_SEED));
    tui.app.dungeon.expedition = Some(expedition);
}

#[test]
fn up_at_the_elevator_banks_everything_and_swims_home() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    let purse = tui.app.purse.spendable();
    let mut haul = Haul {
        gold: 4_321,
        treasures: Vec::new(),
    };
    haul.add(StockItem::COFFEE, 2);
    at_the_elevator(&mut tui, haul);
    let home = fish_at_home(&tui);
    tui.film(reels(), "deep-elevator");
    tui.run("/dungeon");
    tui.snap("the elevator");
    tui.screen().expect_find("bank it all");
    tui.key(KeyCode::Enter);
    tui.write_reel();
    assert_eq!(view(&tui), "closed");
    assert!(tui.app.dungeon.expedition.is_none());
    assert_eq!(tui.app.purse.spendable(), purse + 4_321);
    assert_eq!(tui.app.inventory.get(&StockItem::COFFEE).copied(), Some(2));
    assert_eq!(fish_at_home(&tui), home + 3);
    let bait = tui.app.inventory.get(&StockItem::BAIT).copied();
    assert_eq!(bait, Some(2), "what was packed and not used comes home");
}

#[test]
fn down_at_the_elevator_opens_the_next_floor() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    at_the_elevator(&mut tui, Haul::default());
    tui.run("/dungeon");
    tui.key(KeyCode::Down).key(KeyCode::Enter);
    assert_eq!(view(&tui), "map");
    let expedition = tui.app.dungeon.expedition.as_ref().expect("a dive");
    assert_eq!(expedition.floor, 2);
    assert!(expedition.route.path.is_empty());
    tui.screen().expect_find("The Reef");
}

fn one_fry(difficulty: Difficulty) -> Tui {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    let fry = Fish::new(
        FishSpecies::Goldfish,
        "Pip".to_string(),
        0.0,
        0.0,
        &mut SmallRng::seed_from_u64(CROWD_SEED),
    );
    let mut expedition = Expedition::set_out(
        vec![fry],
        vec![HOME.to_string()],
        difficulty,
        Vec::new(),
        CROWD_SEED,
        &Ground::default(),
    );
    expedition.haul.gold = 777;
    let foes = floor::lair(1, floor::scale_for(1), 0);
    expedition.fight = Some(FightRecord::of(
        CROWD_SEED,
        Stakes::Boss,
        0,
        &foes,
        Vec::new(),
    ));
    tui.app.dungeon.expedition = Some(expedition);
    tui
}

fn fight_to_the_end(tui: &mut Tui) {
    tui.run("/dungeon");
    for _ in 0..FIGHT_TURNS {
        if view(tui) != "fight" {
            return;
        }

        settle(tui);
        tui.key(KeyCode::Enter).key(KeyCode::Enter);
    }
}

#[test]
fn a_wipe_leaves_the_haul_below_as_a_wreck() {
    let mut tui = one_fry(Difficulty::Normal);
    let home = fish_at_home(&tui);
    fight_to_the_end(&mut tui);
    assert!(tui.app.dungeon.expedition.is_none(), "the dive is lost");
    assert_eq!(fish_at_home(&tui), home + 1, "Normal never kills");
    let wreck = tui.app.dungeon.wrecks.first().expect("a wreck");
    assert_eq!(wreck.floor, 1);
    assert!(wreck.haul.gold >= 777);
}

#[test]
fn hardcore_buries_a_fish_that_dies() {
    let mut tui = one_fry(Difficulty::Hardcore);
    let home = fish_at_home(&tui);
    fight_to_the_end(&mut tui);

    assert!(tui.app.dungeon.expedition.is_none());
    assert_eq!(fish_at_home(&tui), home);
    assert!(tui.app.graveyard.iter().any(|fish| fish.name == "Pip"));
}

#[test]
fn a_wreck_is_won_back() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    tui.app.dungeon.wrecks = vec![Wreck {
        floor: 1,
        day: "3 Oct".to_string(),
        haul: Haul {
            gold: 9_999,
            treasures: Vec::new(),
        },
    }];
    let ground = Ground {
        wrecks: tui.app.dungeon.wrecks.clone(),
        ..Ground::default()
    };
    let mut expedition = diving(strong_party(), CROWD_SEED, &ground);
    let room = fishtank::dungeon::map::Room {
        kind: RoomKind::Wreck,
        lane: 2,
        seed: CROWD_SEED,
    };
    expedition.route = Route {
        seed: CROWD_SEED,
        wreck: true,
        path: vec![0, 0],
        done: false,
    };
    expedition.visit = Some(Visit::enter(&expedition, 1, 0, room, &ground));
    tui.app.dungeon.expedition = Some(expedition);
    tui.film(reels(), "deep-wreck");
    tui.run("/dungeon");
    tui.snap("the wreck");
    tui.screen().expect_find("Your party fell here on 3 Oct.");
    tui.key(KeyCode::Enter);
    fight_to_the_end(&mut tui);
    tui.snap("won back");
    tui.write_reel();
    let expedition = tui.app.dungeon.expedition.as_ref().expect("still below");
    assert!(expedition.haul.gold >= 9_999);
    assert!(tui.app.dungeon.wrecks.is_empty());
}

#[test]
fn the_deep_is_offered_to_the_player_and_never_to_a_bot() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    tui.type_text("/dung");
    tui.screen().expect_find("/dungeon");
}

fn crowd(party: Vec<Fish>) -> Tui {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    let names = vec![HOME.to_string(); party.len()];
    let mut expedition = Expedition::new(party, names, Difficulty::Normal);
    let mut rng = SmallRng::seed_from_u64(CROWD_SEED);
    let foes = floor::encounter(1, floor::scale_for(1), &[], &mut rng);
    expedition.fight = Some(FightRecord::of(
        CROWD_SEED,
        Stakes::Encounter,
        0,
        &foes,
        Vec::new(),
    ));
    tui.app.dungeon.expedition = Some(expedition);
    tui
}

#[test]
fn the_battle_on_film() {
    for (cols, rows) in SIZES {
        let mut tui = crowd(Tier::Starter.party(&mut SmallRng::seed_from_u64(CROWD_SEED)));
        tui.resize(cols, rows);
        tui.film(reels(), &format!("deep-battle-{cols}x{rows}"));
        tui.run("/dungeon");
        tui.snap("the fight opens");
        settle(&mut tui);
        tui.snap("a fish chooses");
        tui.key(KeyCode::Down);
        tui.snap("skills highlighted");
        tui.key(KeyCode::Up);
        tui.key(KeyCode::Enter);
        tui.snap("bite: pick a target");
        tui.key(KeyCode::Enter);
        tui.tick_n(20);
        tui.snap("the bite lands");
        settle(&mut tui);
        tui.snap("the next choice");
        tui.write_reel();
        no_flaws(&tui);
    }
}

fn big_fish() -> Vec<Fish> {
    let mut rng = SmallRng::seed_from_u64(CROWD_SEED);
    let ball = Fish::new_unfish(UnfishKind::Ball, "Orbe".to_string(), 0.0, 0.0, &mut rng);
    let skull = Fish::new_unfish(UnfishKind::Skull, "Yorick".to_string(), 0.0, 0.0, &mut rng);
    let mut midas = Fish::new(
        FishSpecies::Cashfish,
        "Midas".to_string(),
        0.0,
        0.0,
        &mut rng,
    );
    apply_mutation_to_fish(&mut midas, Mutation::Telophase, &mut rng);
    let mut morena = Fish::new(
        FishSpecies::Eelfish,
        "Morena".to_string(),
        0.0,
        0.0,
        &mut rng,
    );
    for _ in 0..MORAY_SEGMENTS {
        apply_mutation_to_fish(&mut morena, Mutation::SizeIncrease, &mut rng);
    }
    vec![ball, skull, midas, morena]
}

fn many_fish() -> Vec<Fish> {
    let mut rng = SmallRng::seed_from_u64(CROWD_SEED);
    [
        FishSpecies::Merluza,
        FishSpecies::Salmon,
        FishSpecies::Goldfish,
        FishSpecies::Koi,
        FishSpecies::Lionfish,
        FishSpecies::Starfish,
        FishSpecies::Holyfish,
    ]
    .into_iter()
    .map(|species| Fish::new(species, format!("{species:?}"), 0.0, 0.0, &mut rng))
    .collect()
}

fn film_crowd(name: &str, party: impl Fn() -> Vec<Fish>) {
    for (cols, rows) in SIZES {
        let mut tui = crowd(party());
        tui.resize(cols, rows);
        tui.film(reels(), &format!("{name}-{cols}x{rows}"));
        tui.run("/dungeon");
        settle(&mut tui);
        tui.snap("a fish chooses");
        tui.key(KeyCode::Down);
        tui.key(KeyCode::Enter);
        tui.snap("its skills");
        tui.write_reel();
        no_flaws(&tui);
    }
}

#[test]
fn big_fish_on_film() {
    film_crowd("deep-big", big_fish);
}

#[test]
fn seven_fish_on_film() {
    film_crowd("deep-seven", many_fish);
}

#[test]
fn a_fight_to_the_end_comes_back_to_the_dive() {
    let mut tui = crowd(Tier::Starter.party(&mut SmallRng::seed_from_u64(CROWD_SEED)));
    tui.film(reels(), "deep-whole-fight");
    tui.run("/dungeon");
    for _ in 0..FIGHT_TURNS {
        if view(&tui) != "fight" {
            break;
        }
        settle(&mut tui);
        tui.snap("a turn");
        tui.key(KeyCode::Enter).key(KeyCode::Enter);
    }
    tui.write_reel();
    assert_ne!(view(&tui), "fight", "the fight ended");
}
