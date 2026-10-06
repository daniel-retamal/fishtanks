use std::path::Path;

use crossterm::event::KeyCode;
use fishtank::testing::Tui;

const SIZES: [(u16, u16); 5] = [(100, 30), (80, 24), (60, 18), (40, 14), (28, 10)];

fn shoal(cols: u16, rows: u16) -> Tui {
    let mut tui = Tui::with_size(cols, rows);
    tui.clear_tank();
    for (species, name, grams) in [
        ("salmon", "Ann", 400),
        ("koi", "Bob", 900),
        ("salmon", "Cid", 150),
        ("betta", "Dee", 600),
    ] {
        tui.run(&format!("/spawn {species} \"{name}\""));
        let fish = tui.app.tanks[tui.app.current_tank]
            .fish
            .iter_mut()
            .find(|fish| fish.name == name)
            .expect("spawned");
        fish.weight_g = grams;
    }
    tui
}

fn rows_in_order(tui: &mut Tui, names: &[&str]) -> Vec<String> {
    let text = tui.screen().text();
    let lines: Vec<Vec<char>> = text.lines().map(|line| line.chars().collect()).collect();
    let starts_at = |line: &[char], at: usize, needle: &str| {
        let needle: Vec<char> = needle.chars().collect();
        line.get(at..at + needle.len()) == Some(&needle[..])
    };
    let Some(column) = lines
        .iter()
        .find_map(|line| (0..line.len()).find(|&at| starts_at(line, at, "│ Name ")))
    else {
        return Vec::new();
    };
    lines
        .iter()
        .filter_map(|line| {
            names
                .iter()
                .find(|name| starts_at(line, column, &format!("│ {name} ")))
                .map(|name| name.to_string())
        })
        .collect()
}

const NAMES: [&str; 4] = ["Ann", "Bob", "Cid", "Dee"];

#[test]
fn s_sorts_by_the_focused_column_up_then_down_then_not_at_all() {
    let mut tui = shoal(100, 30);
    tui.run("/index");
    tui.key(KeyCode::Right).key(KeyCode::Right);
    tui.type_text("s");
    tui.screen().expect_find("Weight ▲");
    tui.screen().expect_find("/index sort:weight");
    assert_eq!(
        rows_in_order(&mut tui, &NAMES),
        ["Cid", "Ann", "Dee", "Bob"]
    );
    tui.type_text("s");
    tui.screen().expect_find("Weight ▼");
    assert_eq!(
        rows_in_order(&mut tui, &NAMES),
        ["Bob", "Dee", "Ann", "Cid"]
    );
    tui.type_text("s");
    tui.screen().expect_absent("▼");
    tui.screen().expect_absent("/index");
    assert_eq!(rows_in_order(&mut tui, &NAMES), NAMES);
}

#[test]
fn a_filter_narrows_the_rows_as_it_is_typed_and_esc_takes_it_back() {
    let mut tui = shoal(100, 30);
    tui.run("/index");
    tui.key(KeyCode::Right);
    tui.type_text("f");
    tui.screen().expect_find("ENTER keep");
    tui.type_text("sal");
    tui.screen().expect_find("/index species:sal");
    assert_eq!(rows_in_order(&mut tui, &NAMES), ["Ann", "Cid"]);
    tui.key(KeyCode::Esc);
    tui.screen().expect_find("FishResource#index");
    assert_eq!(rows_in_order(&mut tui, &NAMES), NAMES);
    tui.type_text("fkoi");
    tui.key(KeyCode::Enter);
    assert_eq!(rows_in_order(&mut tui, &NAMES), ["Bob"]);
    tui.type_text("c");
    assert_eq!(rows_in_order(&mut tui, &NAMES), NAMES);
}

#[test]
fn q_while_typing_a_filter_is_a_letter() {
    let mut tui = shoal(100, 30);
    tui.run("/index");
    tui.type_text("fq");
    tui.screen().expect_find("FishResource#index");
    tui.screen().expect_find("/index name:q");
}

#[test]
fn the_command_bar_asks_the_table_the_same_question() {
    let mut tui = shoal(100, 30);
    tui.run("/index species:salmon sort:-weight");
    tui.screen()
        .expect_find("/index species:salmon sort:-weight");
    tui.screen().expect_find("Weight ▼");
    assert_eq!(rows_in_order(&mut tui, &NAMES), ["Ann", "Cid"]);
    tui.key(KeyCode::Esc);
    tui.run("/index weight:>500");
    assert_eq!(rows_in_order(&mut tui, &NAMES), ["Bob", "Dee"]);
    tui.key(KeyCode::Esc);
    tui.run("/index tank:fish worth:<0");
    assert!(rows_in_order(&mut tui, &NAMES).is_empty());
    tui.screen().expect_find("C clear");
}

#[test]
fn a_tank_name_is_a_filter_the_table_can_clear() {
    let mut tui = shoal(100, 30);
    let tank = tui.app.tanks[tui.app.current_tank].name.clone();
    tui.run(&format!("/index \"{tank}\""));
    tui.screen()
        .expect_find(&format!("/index fishtank:\"{tank}\""));
    assert_eq!(rows_in_order(&mut tui, &NAMES), NAMES);
    tui.type_text("c");
    tui.screen().expect_absent("/index");
    assert_eq!(rows_in_order(&mut tui, &NAMES), NAMES);
}

#[test]
fn a_sort_keeps_the_selected_fish_selected() {
    let mut tui = shoal(100, 30);
    tui.run("/index");
    tui.key(KeyCode::Down);
    tui.key(KeyCode::Right).key(KeyCode::Right);
    tui.type_text("s");
    tui.key(KeyCode::Enter);
    tui.screen().expect_find("FishResource#show");
    tui.screen().expect_find("Bob");
}

#[test]
fn the_index_with_a_query_draws_whole_at_every_size() {
    for (cols, rows) in SIZES {
        let mut tui = shoal(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("index-query-{cols}x{rows}"),
        );
        tui.run("/index species:a sort:-weight");
        tui.snap("filtered and sorted");
        tui.key(KeyCode::Right);
        tui.type_text("f");
        tui.type_text("x");
        tui.snap("typing a filter");
        tui.key(KeyCode::Enter);
        tui.snap("nothing left");
        tui.type_text("c");
        tui.snap("cleared");
        let report = tui.reel().flaw_report();
        assert!(report.is_empty(), "{cols}x{rows}:\n{report}");
    }
}
