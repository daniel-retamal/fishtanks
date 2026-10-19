use std::path::Path;

use crossterm::event::KeyCode;
use fishtank::dungeon::Difficulty;
use fishtank::dungeon::Wreck;
use fishtank::dungeon::battle::Stakes;
use fishtank::dungeon::boon::{Boon, Charm, Curse};
use fishtank::dungeon::expedition::{
    Expedition, FightRecord, Ground, HAUNT_SAMPLE, MAX_PARTY, Route, Wound,
};
use fishtank::dungeon::fighter::Ghost;
use fishtank::dungeon::floor;
use fishtank::dungeon::haul::Haul;
use fishtank::dungeon::items::{Item, Pack};
use fishtank::dungeon::life::DeepLife;
use fishtank::dungeon::map::{Map, RoomKind, Shape};
use fishtank::dungeon::room::{Offer, Stage, Visit};
use fishtank::dungeon::screen::View;
use fishtank::dungeon::sim::Tier;
use fishtank::dungeon::traits::Haunt;
use fishtank::fishes::fish::Fish;
use fishtank::fishes::graveyard::Fate;
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

const CROWD: usize = 12;
const PAGE_TURNS: usize = 40;

fn crowded_muster(cols: u16, rows: u16) -> Tui {
    let mut tui = Tui::with_size(cols, rows);
    for n in 1..=CROWD {
        tui.run(&format!("/spawn salmon \"Crowd {n}\""));
    }
    tui.run("/dungeon");
    tui
}

fn every_page(tui: &mut Tui) -> String {
    let mut seen = tui.screen().text();
    for _ in 0..PAGE_TURNS {
        tui.key(KeyCode::Right);
        seen.push_str(&tui.screen().text());
    }
    seen
}

#[test]
fn a_crowded_muster_keeps_every_line_of_the_sheet_on_some_page() {
    for (cols, rows) in SIZES {
        let mut tui = crowded_muster(cols, rows);
        tui.film(reels(), &format!("deep-crowded-muster-{cols}x{rows}"));
        tui.key(KeyCode::Enter);
        tui.key(KeyCode::Down).key(KeyCode::Enter);
        tui.key(KeyCode::Down).key(KeyCode::Enter);
        for _ in 0..CROWD {
            tui.key(KeyCode::Down);
        }
        tui.key(KeyCode::Enter);
        tui.snap("four slots filled, the last fish highlighted");
        let slots = slot_row(&mut tui);
        assert!(
            !slots.contains("┌────"),
            "{cols}x{rows}: an empty slot is left: {slots}"
        );
        let read = every_page(&mut tui);
        tui.snap("its last page");
        tui.write_reel();
        for needle in ["Vigor", "Dodge", "Skills", "Fishtank"] {
            assert!(read.contains(needle), "{cols}x{rows} never showed {needle}");
        }
        no_flaws(&tui);
    }
}

fn slot_row(tui: &mut Tui) -> String {
    let screen = tui.screen();
    let (_, title) = screen.expect_find("The Deep · Party");
    screen.rows()[title + 1].clone()
}

#[test]
fn the_slots_keep_the_order_the_fish_were_picked_in() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    tui.run("/dungeon");
    tui.key(KeyCode::Down)
        .key(KeyCode::Down)
        .key(KeyCode::Enter);
    tui.key(KeyCode::Up).key(KeyCode::Up).key(KeyCode::Enter);
    let slots = slot_row(&mut tui);
    let first = slots.find("┌─ Eva ").expect("Eva in a slot");
    let second = slots.find("┌─ Adam ").expect("Adam in a slot");
    assert!(first < second);
    tui.key(KeyCode::Tab)
        .key(KeyCode::Enter)
        .key(KeyCode::Enter);
    let expedition = tui.app.dungeon.expedition.as_ref().expect("a dive");
    let names: Vec<&str> = expedition
        .party
        .iter()
        .map(|fish| fish.name.as_str())
        .collect();
    assert_eq!(names, ["Eva", "Adam"]);
}

#[test]
fn dropping_a_fish_frees_its_slot_for_the_next() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    tui.run("/dungeon");
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Down).key(KeyCode::Enter);
    tui.key(KeyCode::Up).key(KeyCode::Enter);
    let slots = slot_row(&mut tui);
    assert!(!slots.contains("┌─ Adam "), "{slots}");
    assert!(slots.contains("┌─ Lilith "), "{slots}");
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

const ROOMS: [RoomKind; 14] = [
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
    RoomKind::Hole,
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

const FLOOR_EVENTS: [(u32, &str); 10] = [
    (1, "shallows"),
    (2, "reef"),
    (3, "candy"),
    (4, "graveyard"),
    (5, "dunes"),
    (6, "hot-zone"),
    (7, "mainframe"),
    (8, "inferno"),
    (9, "gate"),
    (10, "eye"),
];

fn the_dead() -> Vec<Ghost> {
    let mut rng = SmallRng::seed_from_u64(CROWD_SEED);
    let mut dead = Vec::new();
    for (species, name, fate) in [
        (FishSpecies::Merluza, "Abel", Fate::Sold),
        (FishSpecies::Cashfish, "Nemo", Fate::Dive),
        (FishSpecies::Salmon, "Pip", Fate::Killed),
    ] {
        let mut fish = Fish::new(species, name.to_string(), 0.0, 0.0, &mut rng);
        fish.fate = Some(fate);
        dead.push(Ghost::of(&fish, Haunt::Dead));
    }
    dead
}

fn at_event(tui: &mut Tui, depth: u32, name: &str, party: Vec<Fish>) {
    let mut expedition = diving(party, CROWD_SEED, &Ground::default());
    expedition.floor = depth;
    expedition
        .pack
        .push(Pack::of(Item::Stock(StockItem::Junk), 1));
    at(tui, expedition, RoomKind::Event);
    let visit = tui
        .app
        .dungeon
        .expedition
        .as_mut()
        .and_then(|e| e.visit.as_mut())
        .expect("a room");
    visit.event = Some(name.to_string());
    visit.offers = the_dead().into_iter().map(Offer::Dead).collect();
}

#[test]
fn the_floor_events_on_film() {
    for (depth, slug) in FLOOR_EVENTS {
        for (cols, rows) in ROOM_SIZES {
            let mut tui = Tui::as_player(cols, rows);
            tui.film(reels(), &format!("deep-events-{slug}-{cols}x{rows}"));
            for event in fishtank::dungeon::event::of_floor(depth) {
                at_event(&mut tui, depth, event.name, strong_party());
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
}

fn slow_and_weak() -> Vec<Fish> {
    let mut party = Tier::Starter.party(&mut SmallRng::seed_from_u64(CROWD_SEED));
    party.truncate(1);
    party
}

#[test]
fn events_gone_wrong_on_film() {
    for (cols, rows) in ROOM_SIZES {
        let mut tui = Tui::as_player(cols, rows);
        tui.film(reels(), &format!("deep-events-wrong-{cols}x{rows}"));
        for (depth, name) in [
            (2, "The Coral Maze"),
            (2, "A Giant Pearl"),
            (3, "The Sugar Altar"),
            (4, "The Séance Table"),
            (5, "A Crashed Saucer"),
            (6, "A Locked Lab Door"),
            (7, "Select Every Square With a Fish"),
            (8, "A Cauldron"),
            (9, "The Scales"),
            (10, "A Reflection"),
        ] {
            at_event(&mut tui, depth, name, slow_and_weak());
            if let Some(expedition) = tui.app.dungeon.expedition.as_mut() {
                expedition.pack.retain(|pack| pack.item != Item::Food);
                expedition.pack.push(Pack::of(Item::Food, 1));
            }
            tui.run("/dungeon");
            tui.key(KeyCode::Enter);
            tui.snap(&format!("{name}: gone wrong"));
            tui.key(KeyCode::Esc);
        }
        tui.write_reel();
        no_flaws(&tui);
    }
}

#[test]
fn a_crop_circle_takes_a_fish_home_to_an_alien_base() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    let fry = Fish::new(
        FishSpecies::Merluza,
        "Pip".to_string(),
        0.0,
        0.0,
        &mut SmallRng::seed_from_u64(CROWD_SEED),
    );
    at_event(&mut tui, 5, "A Crop Circle", vec![fry]);
    tui.film(reels(), "deep-crop-circle");
    tui.run("/dungeon");
    tui.snap("a crop circle");
    tui.key(KeyCode::Enter);
    tui.write_reel();
    assert_eq!(
        view(&tui),
        "closed",
        "the only fish is gone, so the dive ends"
    );
    assert!(tui.app.dungeon.expedition.is_none());
    let base = tui
        .app
        .tanks
        .iter()
        .find(|tank| tank.fish.iter().any(|fish| fish.name == "Pip"))
        .expect("Pip is home");
    assert!(base.kind.is_ufo_base(), "{:?}", base.kind);
    let pip = base
        .fish
        .iter()
        .find(|fish| fish.name == "Pip")
        .expect("Pip");
    assert!(pip.is_alienated());
}

#[test]
fn the_seance_calls_a_sold_fish_back_as_a_ghost_to_fight() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    at_event(&mut tui, 4, "The Séance Table", strong_party());
    tui.run("/dungeon");
    tui.screen().expect_find("Call Abel");
    tui.screen().expect_find("sold");
    tui.key(KeyCode::Enter);
    tui.screen().expect_find("Abel remembers, and it is angry.");
    tui.key(KeyCode::Enter);
    assert_eq!(view(&tui), "fight");
    let fighting_abel = tui
        .app
        .dungeon_overlay()
        .and_then(|state| state.fight())
        .is_some_and(|fight| {
            fight
                .battle
                .fighters
                .iter()
                .any(|f| f.ghost.as_ref().is_some_and(|g| g.name == "Abel"))
        });
    assert!(fighting_abel);
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
fn the_party_pick_holds_big_and_uneven_fish_whole() {
    for (cols, rows) in SIZES {
        let mut tui = Tui::as_player(cols, rows);
        tui.clear_tank();
        let mut rng = SmallRng::seed_from_u64(CROWD_SEED);
        let lion = Fish::new(FishSpecies::Lionfish, "Leo".to_string(), 0.0, 0.0, &mut rng);
        for fish in big_fish().into_iter().chain([lion]) {
            let name = fish.name.clone();
            tui.app.tanks[0].place_fish(fish, name, &mut rng);
        }
        tui.film(reels(), &format!("deep-big-muster-{cols}x{rows}"));
        tui.run("/dungeon");
        for _ in 0..MAX_PARTY {
            tui.key(KeyCode::Enter).key(KeyCode::Down);
        }
        tui.snap("a Ball, a Skull, a double and a moray in the slots, a lionfish highlighted");
        tui.key(KeyCode::Up)
            .key(KeyCode::Up)
            .key(KeyCode::Up)
            .key(KeyCode::Up);
        tui.snap("the Ball highlighted");
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

#[test]
fn a_cast_in_the_fishing_hole_lands_below_and_comes_back_to_the_hole() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    at(
        &mut tui,
        diving(strong_party(), CROWD_SEED, &Ground::default()),
        RoomKind::Hole,
    );
    let home = fish_at_home(&tui);
    tui.film(reels(), "deep-hole");
    tui.run("/dungeon");
    tui.snap("the fishing hole");
    tui.screen().expect_find("Casts left: 3");
    tui.key(KeyCode::Enter);
    assert!(tui.app.fishing_state().is_some(), "the real minigame");
    tui.snap("a cast");
    if let Some(state) = tui.app.fishing_state_mut() {
        state.captured = true;
    }
    tui.tick_n(1);
    tui.snap("what bit");
    let a_fish = tui.screen().contains("ENTER capture");
    if a_fish {
        tui.type_text("Hondo");
    }
    tui.key(KeyCode::Enter);
    assert_eq!(view(&tui), "room");
    tui.screen().expect_find("Casts left: 2");
    tui.snap("back at the hole");
    let expedition = tui.app.dungeon.expedition.as_ref().expect("still below");
    match a_fish {
        true => {
            let hondo = expedition
                .party
                .iter()
                .find(|fish| fish.name == "Hondo")
                .expect("Hondo joins the party");
            assert!(FishSpecies::of_the_deep().contains(&hondo.species));
        }
        false => assert!(!expedition.haul.treasures.is_empty()),
    }
    assert_eq!(
        fish_at_home(&tui),
        home,
        "nothing caught below lands at home"
    );
    tui.key(KeyCode::Enter);
    assert!(tui.app.fishing_state().is_some());
    tui.key(KeyCode::Esc);
    assert_eq!(
        view(&tui),
        "room",
        "a fish that gets away still leaves the hole"
    );
    tui.screen().expect_find("Casts left: 1");
    tui.write_reel();
}

#[test]
fn a_cast_left_unfinished_is_not_spent_when_the_game_is_saved() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    at(
        &mut tui,
        diving(strong_party(), CROWD_SEED, &Ground::default()),
        RoomKind::Hole,
    );
    tui.run("/dungeon");
    tui.key(KeyCode::Enter);
    let text = tui.app.snapshot().to_ron().expect("a save");
    let save = fishtank::app::SaveFile::from_ron(&text).expect("it reads back");
    let mut back = Tui::resumed(save, SIZE.0, SIZE.1);
    back.run("/dungeon");
    back.screen().expect_find("Casts left: 3");
}

fn workshop_party() -> Vec<Fish> {
    let mut rng = SmallRng::seed_from_u64(CROWD_SEED);
    let mut bot = Fish::new(FishSpecies::Botfish, "Neo".to_string(), 0.0, 0.0, &mut rng);
    if let Some(script) = bot.script_mut() {
        script.listen("dawn");
        script.drive("go");
    }
    let toy = Fish::new(FishSpecies::Toyfish, "Robo".to_string(), 0.0, 0.0, &mut rng);
    let merluza = Fish::new(
        FishSpecies::Merluza,
        "Merlín".to_string(),
        0.0,
        0.0,
        &mut rng,
    );
    vec![bot, toy, merluza]
}

const CHOCOLATE: StockItem = StockItem::Consumable(fishtank::loot::ConsumableKind::Milk(
    fishtank::loot::MilkVariant::Chocolate,
));

fn at_the_vent(tui: &mut Tui) {
    let mut expedition = diving(workshop_party(), CROWD_SEED, &Ground::default());
    expedition.pack.push(Pack::of(Item::Stock(CHOCOLATE), 2));
    expedition.haul.gold = 50_000;
    at(tui, expedition, RoomKind::Vent);
}

fn party_fish<'a>(tui: &'a Tui, name: &str) -> &'a Fish {
    tui.app
        .dungeon
        .expedition
        .as_ref()
        .and_then(|e| e.party.iter().find(|f| f.name == name))
        .expect("in the party")
}

#[test]
fn the_vent_workshop_on_film() {
    for (cols, rows) in ROOM_SIZES {
        let mut tui = Tui::as_player(cols, rows);
        tui.film(reels(), &format!("deep-vent-{cols}x{rows}"));
        at_the_vent(&mut tui);
        tui.run("/dungeon");
        tui.snap("the vent, working on a botfish");
        tui.key(KeyCode::Right);
        tui.snap("working on a toy");
        tui.key(KeyCode::Right);
        tui.select("Items");
        tui.key(KeyCode::Enter);
        tui.snap("items out of a fight");
        tui.select("Chocolate Milk");
        tui.key(KeyCode::Enter);
        tui.snap("chocolate milk taken");
        tui.key(KeyCode::Esc);
        tui.write_reel();
        no_flaws(&tui);
    }
}

#[test]
fn a_lasting_item_at_the_vent_changes_the_fish_for_good() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    at_the_vent(&mut tui);
    tui.run("/dungeon");
    tui.key(KeyCode::Right).key(KeyCode::Right);
    let before = party_fish(&tui, "Merlín").mutation_count();
    tui.select("Items");
    tui.key(KeyCode::Enter);
    tui.select("Chocolate Milk");
    tui.key(KeyCode::Enter);
    tui.screen().expect_find("Merlín takes the Chocolate Milk.");
    assert_eq!(party_fish(&tui, "Merlín").mutation_count(), before + 1);
}

#[test]
fn programming_a_botfish_below_saves_onto_the_fish_in_the_party() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    at_the_vent(&mut tui);
    tui.film(reels(), "deep-vent-program");
    tui.run("/dungeon");
    tui.select("Program a botfish");
    tui.key(KeyCode::Enter);
    assert_eq!(view(&tui), "closed", "the wiring panel is open");
    tui.snap("the wiring panel, below");
    tui.type_text(", moon");
    tui.key(KeyCode::Enter);
    assert_eq!(view(&tui), "room", "back at the vent");
    tui.snap("back at the vent");
    tui.write_reel();
    let listens: Vec<String> = party_fish(&tui, "Neo")
        .script()
        .expect("a botfish")
        .listens()
        .map(str::to_string)
        .collect();
    assert!(listens.contains(&"moon".to_string()), "{listens:?}");
}

#[test]
fn dressing_a_toy_below_opens_the_toybox_on_it_and_comes_back() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    at_the_vent(&mut tui);
    tui.film(reels(), "deep-vent-dress");
    tui.run("/dungeon");
    tui.key(KeyCode::Right);
    tui.select("Dress a toy");
    tui.key(KeyCode::Enter);
    assert_eq!(view(&tui), "closed", "the toybox is open");
    tui.snap("the toybox, below");
    tui.screen().expect_find("Toybox#edit");
    tui.key(KeyCode::Esc).key(KeyCode::Esc);
    assert_eq!(view(&tui), "room");
    tui.write_reel();
}

#[test]
fn printing_at_the_vent_adds_printed_botfish_to_the_party() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    let bot = workshop_party().remove(0);
    let blueprint =
        fishtank::tank::Blueprint::capture("Relay".to_string(), &[bot]).expect("a design");
    tui.app.blueprints.push(blueprint);
    tui.app.inventory.insert(
        StockItem::Consumable(fishtank::loot::ConsumableKind::Fabricator),
        1,
    );
    tui.app.inventory.insert(
        StockItem::Consumable(fishtank::loot::ConsumableKind::BlankWafer),
        4,
    );
    at_the_vent(&mut tui);
    let party = tui
        .app
        .dungeon
        .expedition
        .as_ref()
        .expect("below")
        .party
        .len();
    tui.film(reels(), "deep-vent-print");
    tui.run("/dungeon");
    tui.snap("print is open");
    tui.select("Print");
    tui.key(KeyCode::Enter);
    tui.snap("the foundry, below");
    tui.key(KeyCode::Char('p'));
    assert_eq!(view(&tui), "room", "back at the vent");
    tui.snap("printed");
    tui.write_reel();
    let expedition = tui.app.dungeon.expedition.as_ref().expect("below");
    assert_eq!(expedition.party.len(), party + 1);
    assert_eq!(fish_at_home(&tui), 3, "nothing printed lands at home");
}

const NATIVES: [(&str, &str); 6] = [
    ("gota", "Gota"),
    ("hacha", "Hacha"),
    ("barril", "Barril"),
    ("ogro", "Ogro"),
    ("quimera", "Quimera"),
    ("celacanto", "Celacanto"),
];
const NATIVE_GAP: f32 = 16.0;
const NATIVE_LEFT: f32 = 4.0;

fn line_up(tui: &mut Tui) {
    let height = tui.app.tanks[0].height as f32;
    for (index, (_, name)) in NATIVES.iter().enumerate() {
        if let Some(fish) = tui.app.tanks[0].fish.iter_mut().find(|f| f.name == *name) {
            fish.position.x = NATIVE_LEFT + NATIVE_GAP * index as f32;
            fish.position.y = height / 2.0;
            fish.facing = fishtank::fishes::fish::Direction::Right;
        }
    }
    if let Some(above) = tui.app.tanks[0]
        .fish
        .iter_mut()
        .find(|f| f.name == "Sombra")
    {
        above.position.x = NATIVE_LEFT + NATIVE_GAP * 2.0;
        above.position.y = height / 4.0;
    }
}

#[test]
fn the_fish_of_the_deep_on_film() {
    let mut tui = Tui::new();
    tui.resize(110, 20);
    tui.clear_tank();
    tui.film(reels(), "deep-natives");
    for (species, name) in NATIVES {
        tui.run(&format!("/spawn {species} \"{name}\""));
    }
    tui.run("/names");
    line_up(&mut tui);
    tui.tick_n(1);
    line_up(&mut tui);
    tui.snap("the six fish of the Deep, alone with each other");
    tui.run("/spawn betta \"Sombra\"");
    line_up(&mut tui);
    tui.tick_n(1);
    line_up(&mut tui);
    tui.snap("a fish swims above Barril: its eyes look up; Quimera takes the new colours");
    tui.write_reel();
}

const QUIMERA_WAIT: usize = 8;

fn acting(tui: &Tui) -> Option<String> {
    let fight = tui.app.dungeon_overlay()?.fight()?;
    let actor = fight.battle.actor()?;
    Some(fight.battle.fighters[actor].name.clone())
}

fn deep_party() -> Vec<Fish> {
    let mut rng = SmallRng::seed_from_u64(CROWD_SEED);
    [
        FishSpecies::Quimera,
        FishSpecies::Kissfish,
        FishSpecies::Lionfish,
    ]
    .into_iter()
    .map(|species| {
        Fish::new(
            species,
            species.display_name().to_string(),
            0.0,
            0.0,
            &mut rng,
        )
    })
    .collect()
}

#[test]
fn the_deep_fish_and_a_cow_in_a_fight_on_film() {
    for (cols, rows) in SIZES {
        let mut tui = crowd(deep_party());
        if let Some(expedition) = tui.app.dungeon.expedition.as_mut() {
            expedition.cows.push(fishtank::entities::cow::Cow::new(
                "Vaquita from Above".to_string(),
                fishtank::entities::cow::CowVariant::Blue,
                0.0,
                0.0,
                &mut SmallRng::seed_from_u64(CROWD_SEED),
            ));
        }
        tui.resize(cols, rows);
        tui.film(reels(), &format!("deep-natives-fight-{cols}x{rows}"));
        tui.run("/dungeon");
        settle(&mut tui);
        tui.snap("Quimera, a Kissfish, a Lionfish and a cow from above");
        for _ in 0..QUIMERA_WAIT {
            if acting(&tui) == Some("Quimera".to_string()) {
                break;
            }
            tui.select("Guard");
            tui.key(KeyCode::Enter);
            settle(&mut tui);
        }
        assert_eq!(acting(&tui), Some("Quimera".to_string()));
        tui.key(KeyCode::Down);
        tui.key(KeyCode::Enter);
        tui.snap("Quimera's Skills: Kiss and Venom, borrowed");
        tui.write_reel();
        no_flaws(&tui);
    }
}

const EYE: u32 = 10;

fn before_the_eye() -> Tui {
    let mut tui = crowd(strong_party());
    let expedition = tui.app.dungeon.expedition.as_mut().expect("a dive");
    expedition.floor = EYE;
    let foes = floor::lair(EYE, floor::scale_for(EYE), 0);
    expedition.fight = Some(FightRecord::of(
        CROWD_SEED,
        Stakes::Boss,
        0,
        &foes,
        Vec::new(),
    ));
    tui
}

fn prayer_of(tui: &Tui) -> Option<&'static str> {
    tui.app
        .dungeon_overlay()
        .and_then(|state| state.fight())
        .and_then(|fight| fight.prayer())
}

fn words_open(tui: &Tui) -> bool {
    tui.app
        .dungeon_overlay()
        .is_some_and(|state| state.words.is_some())
}

fn menu_of(tui: &Tui) -> Option<fishtank::dungeon::state::Menu> {
    tui.app
        .dungeon_overlay()
        .and_then(|state| state.fight())
        .map(|fight| fight.menu)
}

#[test]
fn the_great_eye_hears_the_line_said_in_its_words_field() {
    for (cols, rows) in SIZES {
        let mut tui = before_the_eye();
        tui.resize(cols, rows);
        tui.film(reels(), &format!("deep-prayer-{cols}x{rows}"));
        tui.run("/dungeon");
        settle(&mut tui);
        let line = prayer_of(&tui).expect("the Eye asks for a line");
        tui.screen().expect_find("> Pray");
        tui.snap("the Eye shields itself: Pray heads the menu");
        tui.key(KeyCode::Enter);
        assert!(words_open(&tui), "Pray opens the words field");
        tui.snap("the words field, the line in quotes and as grey ghost text");
        tui.type_text(&line.to_lowercase());
        tui.snap("the line typed in the field");
        assert!(
            tui.app.editor.text.is_empty(),
            "the command bar is never used"
        );
        tui.key(KeyCode::Enter);
        assert!(
            !words_open(&tui),
            "ENTER says the words and closes the field"
        );
        let said = tui
            .app
            .dungeon_overlay()
            .and_then(|state| state.fight())
            .map(|fight| fight.battle.prayer.said);
        assert_eq!(said, Some(1));
        settle(&mut tui);
        tui.screen().expect_absent("> Pray");
        tui.snap("the Eye blinks, and Pray leaves the menu while it listens");
        tui.write_reel();
        no_flaws(&tui);
    }
}

#[test]
fn a_wrong_line_falls_into_the_dark_and_esc_closes_the_field() {
    let mut tui = before_the_eye();
    tui.run("/dungeon");
    settle(&mut tui);
    tui.key(KeyCode::Enter);
    tui.type_text("wrong words");
    tui.key(KeyCode::Enter);
    assert!(
        log_of(&tui)
            .iter()
            .any(|line| line == "The words fall into the dark.")
    );
    settle(&mut tui);
    assert!(
        prayer_of(&tui).is_some(),
        "the Eye still waits for its line"
    );
    tui.key(KeyCode::Enter);
    tui.type_text("x");
    tui.key(KeyCode::Esc);
    assert!(!words_open(&tui), "ESC closes the field");
    assert_eq!(view(&tui), "fight");
    tui.key(KeyCode::Down);
    assert_eq!(menu_of(&tui), Some(fishtank::dungeon::state::Menu::Main(1)));
}

#[test]
fn the_line_of_prayer_is_said_in_the_room_too() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    at_event(&mut tui, EYE, "A Line of Prayer", strong_party());
    tui.film(reels(), "deep-prayer-event");
    tui.run("/dungeon");
    tui.snap("a line of prayer");
    let seed = tui
        .app
        .dungeon
        .expedition
        .as_ref()
        .and_then(|e| e.visit.as_ref())
        .map(|visit| visit.seed)
        .expect("a room");
    let line = fishtank::dungeon::event::prayer_line(seed);
    tui.screen().expect_find(&format!("\"{line}\""));
    tui.key(KeyCode::Enter);
    assert!(words_open(&tui), "Say it opens the words field");
    tui.screen().expect_find("Words to the Void");
    tui.type_text(line);
    tui.snap("typed");
    tui.key(KeyCode::Enter);
    tui.snap("said");
    tui.write_reel();
    tui.screen().expect_find("The Void listens");
    no_flaws(&tui);
}

#[test]
fn the_blind_deep_opens_under_the_eye_and_never_ends() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    at_the_elevator(&mut tui, Haul::default());
    tui.app.dungeon.expedition.as_mut().expect("a dive").floor = EYE;
    tui.film(reels(), "deep-blind");
    tui.run("/dungeon");
    tui.snap("the last elevator");
    tui.screen().expect_find("The Blind Deep 1");
    let eye = floor::scale_for(EYE).hp_percent;
    for depth in EYE + 1..=EYE + 3 {
        tui.key(KeyCode::Down).key(KeyCode::Enter);
        assert_eq!(view(&tui), "map");
        tui.snap(&format!("floor {depth}"));
        let expedition = tui.app.dungeon.expedition.as_ref().expect("a dive");
        assert_eq!(expedition.floor, depth);
        assert_eq!(expedition.boss().name, "The Great Eye");
        assert!(expedition.scale().hp_percent > eye);
        assert_eq!(
            floor::haul(depth, Difficulty::Normal),
            floor::haul(1, Difficulty::Normal) * u64::from(depth * depth)
        );
        tui.screen()
            .expect_find(&format!("The Blind Deep {}", depth - EYE));
        tui.key(KeyCode::Esc);
        at_the_elevator(&mut tui, Haul::default());
        tui.app.dungeon.expedition.as_mut().expect("a dive").floor = depth;
        tui.run("/dungeon");
    }
    tui.write_reel();
    no_flaws(&tui);
}

const INVENTORY_PAGE: usize = 6;

#[test]
fn treasures_at_home_on_film() {
    use fishtank::dungeon::boon::Charm;
    use fishtank::dungeon::treasure::{Salvage, Treasure};
    use fishtank::loot::ConsumableKind;
    let treasures = [
        Treasure::PincoyasShell,
        Treasure::KrakenInk,
        Treasure::Vial(Mutation::EyeIncrease),
        Treasure::Vial(Mutation::Telophase),
        Treasure::RadiantCore,
        Treasure::Chip(Salvage::Feeder),
        Treasure::MammonsCoin,
        Treasure::Feather,
        Treasure::MarkedCard,
        Treasure::Cowbell(fishtank::entities::cow::CowVariant::Blue),
        Treasure::WishShard,
        Treasure::PhoenixRoe,
        Treasure::Charm(Charm::KrakensBeak),
    ];
    for (cols, rows) in SIZES {
        let mut tui = Tui::as_player(cols, rows);
        tui.film(reels(), &format!("deep-treasures-{cols}x{rows}"));
        for treasure in treasures {
            tui.app
                .inventory
                .insert(StockItem::Consumable(ConsumableKind::Treasure(treasure)), 1);
        }
        tui.run("/inventory");
        for (index, treasure) in treasures.iter().enumerate() {
            if index % INVENTORY_PAGE == 0 || index + 1 == treasures.len() {
                tui.snap(treasure.name());
            }
            tui.key(KeyCode::Down);
        }
        tui.key(KeyCode::Esc);
        let charm = Treasure::Charm(Charm::KrakensBeak);
        tui.app.try_consume_kind(
            ConsumableKind::Treasure(charm),
            fishtank::ui::consume_picker::ConsumePickerSource::FromInventory,
        );
        tui.snap("a charm asks which fish wears it");
        tui.write_reel();
        no_flaws(&tui);
    }
}

#[test]
fn a_boss_pays_its_named_treasures_on_film() {
    for (cols, rows) in ROOM_SIZES {
        let mut tui = Tui::as_player(cols, rows);
        tui.film(reels(), &format!("deep-treasure-spoils-{cols}x{rows}"));
        let mut expedition = diving(strong_party(), CROWD_SEED, &Ground::default());
        expedition.floor = 2;
        at(&mut tui, expedition, RoomKind::Boss);
        let mut found = Haul::default();
        let mut rng = SmallRng::seed_from_u64(CROWD_SEED);
        for loot in fishtank::dungeon::bestiary::find("The Kraken")
            .expect("the Kraken")
            .loot
        {
            if let fishtank::dungeon::bestiary::Loot::Named(find) = loot {
                fishtank::dungeon::haul::find(&mut found, *find, &mut rng);
            }
        }
        let visit = tui
            .app
            .dungeon
            .expedition
            .as_mut()
            .and_then(|e| e.visit.as_mut())
            .expect("the boss room");
        visit.stage = Stage::Spoils;
        visit.said = fishtank::dungeon::room::spoils_lines(&found);
        tui.run("/dungeon");
        tui.snap("the Kraken's spoils");
        tui.write_reel();
        no_flaws(&tui);
    }
}

fn hollows(tui: &mut Tui) {
    let mut rng = SmallRng::seed_from_u64(CROWD_SEED);
    for (name, worn) in [("Empty", None), ("Full", Some("Abel"))] {
        let mut hollow = Fish::new_unfish(UnfishKind::Hollow, name.to_string(), 0.0, 0.0, &mut rng);
        if let (Some(worn), Some(fishtank::fishes::quirk::Quirk::Hollow(state))) =
            (worn, hollow.quirk_mut())
        {
            state.worn = Some(fishtank::fishes::quirk::Worn {
                name: worn.to_string(),
                species: FishSpecies::Koi,
                color: fishtank::colors::ORANGE,
            });
        }
        let named = hollow.name.clone();
        tui.app.tanks[0].place_fish(hollow, named, &mut rng);
    }
}

fn line_up_the_hollows(tui: &mut Tui) {
    let height = tui.app.tanks[0].height as f32;
    for (index, name) in ["Empty", "Full"].into_iter().enumerate() {
        if let Some(fish) = tui.app.tanks[0].fish.iter_mut().find(|f| f.name == name) {
            fish.position.x = NATIVE_LEFT + NATIVE_GAP * 2.0 * index as f32;
            fish.position.y = height / 2.0;
            fish.facing = fishtank::fishes::fish::Direction::Right;
        }
    }
}

#[test]
fn the_hollow_on_film() {
    let mut tui = Tui::new();
    tui.resize(110, 20);
    tui.clear_tank();
    tui.film(reels(), "deep-hollow");
    hollows(&mut tui);
    tui.run("/names");
    line_up_the_hollows(&mut tui);
    tui.tick_n(1);
    line_up_the_hollows(&mut tui);
    tui.snap("an empty Hollow, and one wearing a dead Koi");
    tui.write_reel();

    for (cols, rows) in SIZES {
        let mut party = deep_party();
        party.push(Fish::new_unfish(
            UnfishKind::Hollow,
            "Hollow".to_string(),
            0.0,
            0.0,
            &mut SmallRng::seed_from_u64(CROWD_SEED),
        ));
        let mut tui = crowd(party);
        if let Some(record) = tui
            .app
            .dungeon
            .expedition
            .as_mut()
            .and_then(|e| e.fight.as_mut())
        {
            record.haunts = the_dead();
        }
        tui.resize(cols, rows);
        tui.film(reels(), &format!("deep-hollow-fight-{cols}x{rows}"));
        tui.run("/dungeon");
        settle(&mut tui);
        tui.snap("the Hollow fills with one of your dead");
        tui.write_reel();
        no_flaws(&tui);
        assert!(
            log_of(&tui)
                .iter()
                .any(|line| line.starts_with("Hollow fills with"))
        );
    }
}

#[test]
fn the_deep_toy_parts_on_film() {
    use fishtank::fishes::toy::{FittedPart, Material, ToyColor, ToyPart, ToyState};
    let mut tui = Tui::new();
    tui.resize(110, 20);
    tui.clear_tank();
    tui.film(reels(), "deep-toys");
    let mut rng = SmallRng::seed_from_u64(CROWD_SEED);
    let wearing = [
        (ToyPart::DivingHelmet, "Helmet"),
        (ToyPart::JellyfishCap, "Cap"),
        (ToyPart::Periscope, "Periscope"),
        (ToyPart::Anchor, "Anchor"),
    ];
    for (index, (part, name)) in wearing.into_iter().enumerate() {
        let mut state = ToyState::plain(ToyColor::Coral, Material::Plastic);
        state
            .fittings
            .set(part.slot(), Some(FittedPart::of(part, &mut rng)));
        let mut toy = Fish::new_toy(state, fishtank::fishes::species::SizeCategory::L, &mut rng);
        toy.name = name.to_string();
        let named = toy.name.clone();
        tui.app.tanks[0].place_fish(toy, named, &mut rng);
        let height = tui.app.tanks[0].height as f32;
        if let Some(fish) = tui.app.tanks[0].fish.iter_mut().find(|f| f.name == name) {
            fish.position.x = NATIVE_LEFT + NATIVE_GAP * index as f32;
            fish.position.y = height / 2.0;
        }
    }
    tui.run("/names");
    tui.snap("a Diving Helmet, a Jellyfish Cap, a Periscope and an Anchor");
    tui.write_reel();
}

const MANY_DEAD: usize = 2_000;
const DIVE_SAVE_MOST_BYTES: usize = 64 * 1024;

fn saved_bytes(tui: &Tui) -> usize {
    tui.app.snapshot().to_ron().expect("a save").len()
}

#[test]
fn a_fight_keeps_a_handful_of_ghosts_however_many_fish_have_died() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    let mut rng = SmallRng::seed_from_u64(CROWD_SEED);
    for n in 0..MANY_DEAD {
        let sold = Fish::new(
            FishSpecies::Merluza,
            format!("Sold {n}"),
            0.0,
            0.0,
            &mut rng,
        );
        tui.app.graveyard.push(sold);
    }
    let haunts = tui.app.deep_ground().haunts.len();
    assert!(
        haunts <= HAUNT_SAMPLE,
        "{haunts} ghosts drawn from one kind of dead"
    );
    at(
        &mut tui,
        diving(strong_party(), CROWD_SEED, &Ground::default()),
        RoomKind::Encounter,
    );
    tui.run("/dungeon");
    tui.select("Fight");
    tui.key(KeyCode::Enter);
    assert_eq!(view(&tui), "fight");
    let record = tui
        .app
        .dungeon
        .expedition
        .as_ref()
        .and_then(|e| e.fight.as_ref())
        .expect("a fight");
    assert!(record.haunts.len() <= HAUNT_SAMPLE);
    let diving_save = saved_bytes(&tui);
    let expedition = tui.app.dungeon.expedition.take();
    let home_save = saved_bytes(&tui);
    tui.app.dungeon.expedition = expedition;
    let dive = diving_save - home_save;
    assert!(
        dive < DIVE_SAVE_MOST_BYTES,
        "a dive mid-fight wrote {dive} bytes into the save"
    );
}

#[test]
fn a_game_that_never_dived_writes_nothing_about_the_deep() {
    let tui = Tui::as_player(SIZE.0, SIZE.1);
    let save = tui.app.snapshot().to_ron().expect("a save");
    assert!(
        !save.contains("dungeon:"),
        "an untouched Deep is not written"
    );
    assert!(
        !save.contains("deep:"),
        "a fish that never dived carries nothing"
    );
}

const FIELD_SIZES: [(u16, u16); 4] = [(100, 30), (60, 18), (40, 14), (28, 10)];

fn veteran(tui: &mut Tui) {
    let adam = tui
        .app
        .tanks
        .iter_mut()
        .flat_map(|tank| tank.fish.iter_mut())
        .find(|fish| fish.name == "Adam")
        .expect("Adam");
    adam.deep = Some(Box::new(DeepLife {
        floors_survived: 11,
        dives: 7,
        deepest: 6,
        beaten: 212,
        boons: vec![Boon::SecondWind],
        curses: vec![Curse::HungryGhost],
        charm: Some(Charm::CaleuchesLantern),
        ..DeepLife::default()
    }));
}

#[test]
fn the_deep_fields_on_show_and_index_on_film() {
    for (cols, rows) in FIELD_SIZES {
        let mut tui = Tui::as_player(cols, rows);
        tui.film(reels(), &format!("deep-fields-{cols}x{rows}"));
        veteran(&mut tui);
        tui.run("/show \"Adam\" all");
        tui.snap("Adam's card: the Deep's fields come last");
        for (field, label) in [
            ("Elder (11", "scrolled to the Grade"),
            ("Floor 6", "the Dives"),
            ("Lantern", "the Charm"),
            ("Ghost", "the Boons and the Curses, last"),
        ] {
            let mut found = false;
            for _ in 0..FIGHT_TURNS {
                if tui.screen().contains(field) {
                    found = true;
                    break;
                }
                tui.key(KeyCode::Down);
            }
            assert!(found, "the card scrolls to {field} at {cols}x{rows}");
            tui.snap(label);
        }
        tui.key(KeyCode::Esc);
        tui.run("/index");
        tui.snap("the index grows a Grade column once a fish has dived");
        tui.key(KeyCode::Esc);
        tui.run("/index sort:-grade");
        tui.snap("sorted by grade, the deepest first, the column in view");
        tui.write_reel();
        no_flaws(&tui);
    }
}

#[test]
fn a_tank_that_never_dived_shows_no_grade() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    tui.run("/index");
    tui.screen().expect_absent("Grade");
    tui.key(KeyCode::Esc);
    tui.run("/index grade:elder");
    tui.screen().expect_find("Grade");
}
