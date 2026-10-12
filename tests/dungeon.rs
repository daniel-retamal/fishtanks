use std::path::Path;

use crossterm::event::KeyCode;
use fishtank::dungeon::Difficulty;
use fishtank::dungeon::battle::Stakes;
use fishtank::dungeon::expedition::{Expedition, FightRecord};
use fishtank::dungeon::floor;
use fishtank::fishes::fish::Fish;
use fishtank::fishes::mutations::{Mutation, apply_mutation_to_fish};
use fishtank::fishes::species::FishSpecies;
use fishtank::fishes::unfish::UnfishKind;
use fishtank::testing::Tui;
use rand::SeedableRng;
use rand::rngs::SmallRng;

const SIZES: [(u16, u16); 6] = [(120, 38), (100, 30), (80, 24), (60, 18), (40, 14), (28, 10)];
const SETTLE_TICKS: usize = 600;

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

#[test]
fn the_battle_on_film() {
    for (cols, rows) in SIZES {
        let mut tui = Tui::as_player(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("deep-battle-{cols}x{rows}"),
        );
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
        assert!(
            tui.reel().flaw_report().is_empty(),
            "{}",
            tui.reel().flaw_report()
        );
    }
}

const FIGHT_TURNS: usize = 400;
const SIZE: (u16, u16) = (120, 38);

fn diving() -> Tui {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    tui.run("/dungeon");
    settle(&mut tui);
    tui
}

fn bite_once(tui: &mut Tui) {
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Enter);
    settle(tui);
}

fn log_of(tui: &Tui) -> Vec<String> {
    tui.app
        .dungeon_overlay()
        .map(|state| {
            state
                .battle
                .log
                .iter()
                .map(|line| line.text.clone())
                .collect()
        })
        .unwrap_or_default()
}

fn fish_at_home(tui: &Tui) -> usize {
    tui.app.tanks.iter().map(|tank| tank.fish.len()).sum()
}

#[test]
fn the_party_leaves_its_tank_for_the_deep() {
    let tui = diving();
    assert_eq!(fish_at_home(&tui), 0);
    let expedition = tui.app.dungeon.expedition.as_ref().expect("a dive");
    assert_eq!(expedition.party.len(), 3);
}

#[test]
fn leaving_the_deep_keeps_the_dive_and_dungeon_resumes_it() {
    let mut tui = diving();
    bite_once(&mut tui);
    let before = log_of(&tui);
    tui.key(KeyCode::Esc);
    assert!(tui.app.dungeon_overlay().is_none());
    assert_eq!(fish_at_home(&tui), 0, "the party waits below");
    tui.run("/dungeon");
    assert_eq!(log_of(&tui), before);
    tui.screen().expect_find("The Shallows");
}

#[test]
fn a_dive_survives_a_save() {
    let mut tui = diving();
    bite_once(&mut tui);
    bite_once(&mut tui);
    let before = log_of(&tui);
    tui.key(KeyCode::Esc);
    let text = tui.app.snapshot().to_ron().expect("a save");
    let save = fishtank::app::SaveFile::from_ron(&text).expect("it reads back");
    let mut back = Tui::resumed(save, SIZE.0, SIZE.1);
    back.run("/dungeon");
    assert_eq!(log_of(&back), before);
}

#[test]
fn a_fight_to_the_end_brings_the_party_home() {
    let mut tui = diving();
    tui.film(Path::new(env!("CARGO_TARGET_TMPDIR")), "deep-whole-fight");
    for _ in 0..FIGHT_TURNS {
        if tui.app.dungeon_overlay().is_none() {
            break;
        }
        tui.snap("a turn");
        bite_once(&mut tui);
    }
    tui.write_reel();
    assert!(tui.app.dungeon_overlay().is_none(), "the fight ended");
    assert!(tui.app.dungeon.expedition.is_none());
    assert_eq!(fish_at_home(&tui), 3, "Normal never kills");
}

#[test]
fn the_deep_is_offered_to_the_player_and_never_to_a_bot() {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    tui.type_text("/dung");
    tui.screen().expect_find("/dungeon");
}

const MORAY_SEGMENTS: usize = 8;
const CROWD_SEED: u64 = 17;

fn crowd(party: Vec<Fish>) -> Tui {
    let mut tui = Tui::as_player(SIZE.0, SIZE.1);
    let names = vec!["Fishtank".to_string(); party.len()];
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
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("{name}-{cols}x{rows}"),
        );
        tui.run("/dungeon");
        settle(&mut tui);
        tui.snap("a fish chooses");
        tui.key(KeyCode::Down);
        tui.key(KeyCode::Enter);
        tui.snap("its skills");
        tui.write_reel();
        assert!(
            tui.reel().flaw_report().is_empty(),
            "{}",
            tui.reel().flaw_report()
        );
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
