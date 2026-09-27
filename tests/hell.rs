use std::path::Path;

use crossterm::event::KeyCode;
use fishtank::{
    tank::{Tank, TankKind},
    testing::Tui,
};

const HOME: &str = "Fishtank";
const PIT: &str = "Pit";
const ABYSS: &str = "Abyss";
const REEL_DIR: &str = env!("CARGO_TARGET_TMPDIR");

fn hell<'a>(tui: &'a Tui, name: &str) -> &'a Tank {
    tui.app
        .tanks
        .iter()
        .find(|t| t.kind == TankKind::Hell && t.name == name)
        .expect("the helltank")
}

fn damned(tui: &Tui, name: &str) -> Vec<String> {
    hell(tui, name)
        .souls()
        .iter()
        .map(|f| f.name.clone())
        .collect()
}

fn heaven_souls(tui: &Tui) -> Vec<String> {
    tui.app
        .tanks
        .iter()
        .find(|t| t.kind == TankKind::Heaven)
        .map(|t| t.souls().iter().map(|f| f.name.clone()).collect())
        .unwrap_or_default()
}

fn living(tui: &Tui, name: &str) -> Option<String> {
    tui.app
        .tanks
        .iter()
        .find(|t| t.fish.iter().any(|f| f.name == name))
        .map(|t| t.name.clone())
}

fn found_hell(tui: &mut Tui, name: &str) {
    tui.run("/add necronomicon 1");
    tui.run("/consume necronomicon");
    tui.type_text(name);
    tui.key(KeyCode::Enter);
    tui.run(&format!("/switch \"{HOME}\""));
}

fn found_heaven(tui: &mut Tui) {
    tui.run("/add golden pearl 1");
    tui.run("/consume golden pearl");
    tui.type_text("Heaventank");
    tui.key(KeyCode::Enter);
    tui.run(&format!("/switch \"{HOME}\""));
}

fn home_with(names: &[&str]) -> Tui {
    let mut tui = Tui::with_size(100, 30);
    tui.clear_tank();
    for name in names {
        tui.run(&format!("/spawn salmon \"{name}\""));
    }
    tui
}

fn sin_in_hell(tui: &mut Tui, name: &str) {
    tui.run(&format!("/move \"{name}\" \"{PIT}\""));
    tui.run(&format!("/kill \"{name}\""));
}

#[test]
fn a_fish_the_devil_marked_hangs_on_the_helltank_wall_and_never_in_heaven() {
    let mut tui = home_with(&["Ann"]);
    found_hell(&mut tui, PIT);

    sin_in_hell(&mut tui, "Ann");

    assert_eq!(damned(&tui, PIT), ["Ann"]);
    assert!(heaven_souls(&tui).is_empty());
    assert!(
        hell(&tui, PIT).fish.is_empty(),
        "a soul is not a living fish"
    );
}

#[test]
fn an_unmarked_death_never_reaches_hell() {
    let mut tui = home_with(&["Bob"]);
    found_hell(&mut tui, PIT);
    found_heaven(&mut tui);
    tui.run("/kill \"Bob\"");
    assert!(damned(&tui, PIT).is_empty());
    assert_eq!(heaven_souls(&tui), ["Bob"]);
}

#[test]
fn a_revived_sinner_leaves_the_hell_wall() {
    let mut tui = home_with(&["Ann"]);
    found_hell(&mut tui, PIT);
    sin_in_hell(&mut tui, "Ann");

    tui.run("/revive \"Ann\"");

    assert!(damned(&tui, PIT).is_empty());
    assert!(living(&tui, "Ann").is_some());
}

#[test]
fn the_damned_on_the_wall_count_as_devils_luck() {
    let mut tui = home_with(&["Ann", "Bob"]);
    found_hell(&mut tui, PIT);
    tui.run(&format!("/move \"Bob\" \"{PIT}\""));
    tui.run(&format!("/switch \"{PIT}\""));
    tui.screen().expect_find("devil's luck I ");

    sin_in_hell(&mut tui, "Ann");

    assert!(hell(&tui, PIT).fish.len() == 1 && damned(&tui, PIT) == ["Ann"]);
    tui.screen().expect_find("devil's luck II");
}

#[test]
fn each_afterlife_founded_after_its_dead_died_gathers_them() {
    let mut tui = home_with(&["Ann", "Bob"]);
    for fish in tui.app.tanks[0].fish.iter_mut().filter(|f| f.name == "Ann") {
        fish.devil_marked = true;
    }
    tui.run("/kill \"Ann\"");
    tui.run("/kill \"Bob\"");

    found_hell(&mut tui, PIT);
    assert_eq!(damned(&tui, PIT), ["Ann"]);
    assert!(heaven_souls(&tui).is_empty(), "no heaven opens by itself");

    found_heaven(&mut tui);
    assert_eq!(heaven_souls(&tui), ["Bob"]);
}

#[test]
fn a_sinner_hangs_on_one_wall_however_many_hells_there_are() {
    let mut tui = home_with(&["Ann"]);
    found_hell(&mut tui, PIT);
    sin_in_hell(&mut tui, "Ann");

    found_hell(&mut tui, ABYSS);

    assert_eq!(damned(&tui, PIT), ["Ann"]);
    assert!(damned(&tui, ABYSS).is_empty());
}

#[test]
fn selling_a_helltank_hands_its_damned_to_the_next_one() {
    let mut tui = home_with(&["Ann"]);
    found_hell(&mut tui, PIT);
    sin_in_hell(&mut tui, "Ann");
    found_hell(&mut tui, ABYSS);

    tui.run(&format!("/sell tank \"{PIT}\""));

    assert!(tui.app.tanks.iter().all(|t| t.name != PIT));
    assert_eq!(damned(&tui, ABYSS), ["Ann"]);
}

#[test]
fn the_damned_of_a_sold_last_hell_wait_in_the_graveyard_for_the_next() {
    let mut tui = home_with(&["Ann"]);
    found_hell(&mut tui, PIT);
    sin_in_hell(&mut tui, "Ann");

    tui.run(&format!("/sell tank \"{PIT}\""));
    found_hell(&mut tui, ABYSS);

    assert_eq!(damned(&tui, ABYSS), ["Ann"]);
}

#[test]
fn the_helltank_index_lists_its_damned_as_dead() {
    let mut tui = home_with(&["Ann"]);
    found_hell(&mut tui, PIT);
    sin_in_hell(&mut tui, "Ann");

    tui.run(&format!("/index \"{PIT}\""));

    let screen = tui.screen();
    screen.expect_find("Status");
    screen.expect_find("Ann");
    screen.expect_find("Dead");
}

#[test]
fn hell_draws_its_damned_at_every_size_without_breaking_a_border() {
    for (cols, rows) in [(100, 30), (60, 18), (40, 14), (28, 10)] {
        let mut tui = home_with(&["Ann", "Cain"]);
        found_hell(&mut tui, PIT);
        sin_in_hell(&mut tui, "Ann");
        sin_in_hell(&mut tui, "Cain");
        tui.run(&format!("/switch \"{PIT}\""));
        tui.resize(cols, rows);
        tui.film(Path::new(REEL_DIR), &format!("hell-wall-{cols}x{rows}"));
        tui.tick_n(30);
        tui.snap("the damned drift behind the hell plants");
        tui.run("/names");
        tui.snap("their names in dark red");
        tui.run(&format!("/index \"{PIT}\""));
        tui.snap("the index lists them as dead");
        assert!(
            tui.reel().flaw_report().is_empty(),
            "{cols}x{rows}:\n{}",
            tui.reel().flaw_report()
        );
    }
}
