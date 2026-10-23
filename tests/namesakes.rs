use fishtank::fishes::species::FishSpecies;
use fishtank::tank::{Tank, TankKind};
use fishtank::testing::Tui;

const NAMESAKE: &str = "XVI";

fn two_namesakes() -> (Tui, usize, usize) {
    let mut tui = Tui::new();
    tui.clear_tank();
    let home = tui.app.current_tank;
    tui.run(&format!("/spawn cashfish \"{NAMESAKE}\""));
    let mut reef = Tank::new("Reef".to_string(), TankKind::CoralReef, &[]);
    reef.resize(80, 20, &[]);
    reef.spawn_fish(FishSpecies::Salmon, NAMESAKE.to_string(), &mut rand::rng());
    tui.app.tanks.push(reef);
    let away = tui.app.tanks.len() - 1;
    (tui, home, away)
}

fn species_named(tui: &Tui, tank: usize) -> Option<FishSpecies> {
    tui.app.tanks[tank]
        .fish
        .iter()
        .find(|fish| fish.name == NAMESAKE)
        .map(|fish| fish.species)
}

#[test]
fn a_name_means_the_fish_in_the_tank_you_are_watching() {
    let (mut tui, home, away) = two_namesakes();
    tui.run("/switch \"Reef\"");
    assert_eq!(tui.app.current_tank, away);
    tui.run(&format!("/show \"{NAMESAKE}\""));
    let screen = tui.screen();
    screen.expect_find("Salmon");
    screen.expect_absent("Cashfish");
    assert_eq!(species_named(&tui, home), Some(FishSpecies::Cashfish));
}

#[test]
fn a_move_takes_the_namesake_from_the_tank_you_are_watching() {
    let (mut tui, home, away) = two_namesakes();
    let mut third = Tank::new("Pond".to_string(), TankKind::Base, &[]);
    third.resize(80, 20, &[]);
    tui.app.tanks.push(third);
    let pond = tui.app.tanks.len() - 1;
    tui.run("/switch \"Reef\"");
    tui.run(&format!("/move \"{NAMESAKE}\" \"Pond\""));
    assert_eq!(species_named(&tui, pond), Some(FishSpecies::Salmon));
    assert_eq!(species_named(&tui, away), None);
    assert_eq!(species_named(&tui, home), Some(FishSpecies::Cashfish));
}

#[test]
fn a_name_not_in_the_watched_tank_still_finds_its_fish_elsewhere() {
    let (mut tui, home, _) = two_namesakes();
    tui.run("/spawn salmon \"Lonely\"");
    tui.run("/switch \"Reef\"");
    tui.run("/show \"Lonely\"");
    tui.screen().expect_find("Lonely");
    assert!(tui.app.tanks[home].fish.iter().any(|f| f.name == "Lonely"));
}

fn candytank(tui: &Tui) -> usize {
    tui.app
        .tanks
        .iter()
        .position(|tank| tank.name == "Candytank")
        .expect("a Candytank")
}

fn names_everywhere(tui: &Tui) -> Vec<String> {
    tui.app
        .tanks
        .iter()
        .flat_map(|tank| tank.fish.iter().map(|fish| fish.name.clone()))
        .collect()
}

#[test]
fn a_name_already_swimming_in_another_tank_is_numbered() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/spawn salmon \"Glurp\"");
    tui.run("/give candytank");
    tui.run("/switch \"Candytank\"");
    assert_eq!(tui.app.tanks[tui.app.current_tank].name, "Candytank");
    tui.run("/spawn salmon \"Glurp\"");
    let mut names = names_everywhere(&tui);
    names.retain(|name| name.starts_with("Glurp"));
    names.sort();
    assert_eq!(names, ["Glurp", "Glurp II"]);
}

#[test]
fn a_move_keeps_a_fish_its_name() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/spawn salmon \"Glurp\"");
    tui.run("/give candytank");
    tui.run("/move \"Glurp\" \"Candytank\"");
    let candy = candytank(&tui);
    assert!(tui.app.tanks[candy].fish.iter().any(|f| f.name == "Glurp"));
}

#[test]
fn a_save_with_namesakes_in_two_tanks_loads_with_every_name_its_own() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/spawn salmon \"Glurp\"");
    tui.run("/give candytank");
    let candy = candytank(&tui);
    for name in ["Placeholder", "Another"] {
        tui.app.tanks[candy].spawn_fish(FishSpecies::Salmon, name.into(), &mut rand::rng());
    }
    for fish in &mut tui.app.tanks[candy].fish {
        fish.name = "Glurp".to_string();
    }
    let save = tui.app.snapshot();
    let resumed = Tui::resumed(save, 80, 24);
    let names = names_everywhere(&resumed);
    let unique: std::collections::HashSet<&String> = names.iter().collect();
    assert_eq!(unique.len(), names.len(), "{names:?}");
    assert_eq!(resumed.app.tanks[0].fish[0].name, "Glurp");
}
