use std::path::Path;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use fishtank::{
    colors::{DARK_GRAY, WHITE},
    economy::Money,
    fishes::species::FishSpecies,
    testing::{DEFAULT_COLS, Tui},
};
use ratatui::style::Color;

const SELECTED: &str = "> ";
const FISHES_ROW: &str = "Fishes";
const ROBOTICS_ROW: &str = "Robotics";
const SHELF_CHROME_ROWS: usize = 20;

fn rows_for_every_fish() -> u16 {
    (FishSpecies::all_buyable().len() + SHELF_CHROME_ROWS) as u16
}

fn ink(tui: &mut Tui, needle: &str) -> Color {
    let (x, y) = tui.screen().expect_find(needle);
    let reel = tui.reel();
    let still = reel.stills().last().expect("a still was taken");
    still.glyph(x, y).expect("on screen").fg
}

fn row_of(tui: &mut Tui, needle: &str) -> usize {
    tui.screen().expect_find(needle).1
}

fn cheapest() -> FishSpecies {
    FishSpecies::all_buyable()
        .iter()
        .copied()
        .min_by_key(|species| (species.buy_price(), species.display_name()))
        .expect("the shop sells fish")
}

fn shop_with(cash: Money) -> Tui {
    let mut tui = Tui::with_size(DEFAULT_COLS, rows_for_every_fish());
    tui.film(Path::new(env!("CARGO_TARGET_TMPDIR")), "shop-availability");
    tui.clear_tank();
    tui.run("/spawn merluza \"Ann\"");
    tui.app.purse = fishtank::economy::Purse::holding(cash);
    tui.run("/shop");
    tui
}

#[test]
fn white_is_for_sale_grey_is_not_and_the_cursor_is_the_arrow() {
    let mut tui = shop_with(Money::from(cheapest().buy_price()));
    tui.snap("the counter");
    assert_eq!(ink(&mut tui, "Buy"), WHITE);
    assert_eq!(
        ink(&mut tui, "Sell"),
        WHITE,
        "an unselected row that works is white"
    );
    tui.screen().expect_find(&format!("{SELECTED}Buy"));

    tui.key(KeyCode::Enter);
    tui.snap("the categories, no Matrixtank");
    assert_eq!(ink(&mut tui, FISHES_ROW), WHITE);
    assert_eq!(
        ink(&mut tui, ROBOTICS_ROW),
        DARK_GRAY,
        "the locked bench is grey"
    );
    tui.screen().expect_find(&format!("{SELECTED}{FISHES_ROW}"));
}

#[test]
fn what_is_not_for_sale_sinks_below_everything_that_is() {
    let mut tui = shop_with(Money::from(cheapest().buy_price()));
    tui.key(KeyCode::Enter);
    tui.snap("the categories");
    let robotics = row_of(&mut tui, ROBOTICS_ROW);
    for category in [FISHES_ROW, "Food"] {
        assert!(
            row_of(&mut tui, category) < robotics,
            "{category} is for sale and sits above the locked bench"
        );
    }

    tui.select(FISHES_ROW);
    tui.key(KeyCode::Enter);
    tui.snap("the fish shelf on the cheapest fish's budget");
    let affordable = cheapest().display_name();
    assert_eq!(ink(&mut tui, affordable), WHITE);
    tui.screen().expect_find(&format!("{SELECTED}{affordable}"));
    let dearer = FishSpecies::all_buyable()
        .iter()
        .copied()
        .filter(|species| species.buy_price() > cheapest().buy_price())
        .min_by_key(|species| (species.buy_price(), species.display_name()))
        .expect("the shop sells something dearer");
    assert_eq!(ink(&mut tui, dearer.display_name()), DARK_GRAY);
    assert!(
        row_of(&mut tui, dearer.display_name()) > row_of(&mut tui, affordable),
        "what the purse cannot pay for sits below what it can"
    );
    tui.key(KeyCode::Down);
    tui.snap("↓ on the last fish for sale");
    assert!(
        tui.screen()
            .find(&format!("{SELECTED}{}", dearer.display_name()))
            .is_none(),
        "the cursor never lands on a fish the purse cannot pay for"
    );
}

#[test]
fn a_priced_shelf_reads_cheapest_first_and_ties_by_name() {
    let mut tui = shop_with(Money::MAX / 2);
    tui.key(KeyCode::Enter);
    tui.select(FISHES_ROW);
    tui.key(KeyCode::Enter);
    tui.snap("the fish shelf, rich");
    let mut shelf: Vec<(u32, &str)> = FishSpecies::all_buyable()
        .iter()
        .map(|species| (species.buy_price(), species.display_name()))
        .collect();
    shelf.sort();
    let screen = tui.screen();
    let rows: Vec<usize> = shelf
        .iter()
        .filter_map(|(_, name)| screen.find(&format!(" {name} ")).map(|(_, y)| y))
        .collect();
    assert!(rows.len() > 1, "the shelf shows several fish");
    assert!(rows.is_sorted(), "cheapest first, ties by name");
}

fn cursor_row(tui: &mut Tui) -> Option<String> {
    tui.screen()
        .text()
        .lines()
        .find_map(|line| {
            ["│> ", "┤> "]
                .iter()
                .find_map(|mark| line.split_once(mark).map(|(_, row)| row.to_string()))
        })
        .map(|row| {
            row.split("  ")
                .next()
                .unwrap_or_default()
                .trim()
                .to_string()
        })
}

fn by_price() -> Vec<FishSpecies> {
    let mut shelf: Vec<FishSpecies> = FishSpecies::all_buyable().to_vec();
    shelf.sort_by_key(|species| (species.buy_price(), species.display_name()));
    shelf
}

#[test]
fn a_fish_you_can_no_longer_afford_leaves_the_cursor_beside_it() {
    let shelf = by_price();
    let dearer = shelf
        .iter()
        .position(|species| species.buy_price() > shelf[0].buy_price())
        .expect("the shelf has two prices");
    let bought = shelf[dearer];
    let beside = shelf[dearer - 1];
    let mut tui = shop_with(Money::from(bought.buy_price() + beside.buy_price()));
    tui.key(KeyCode::Enter);
    tui.select(FISHES_ROW);
    tui.key(KeyCode::Enter);
    tui.select(bought.display_name());
    tui.key(KeyCode::Enter);
    tui.type_text("Pip");
    tui.key(KeyCode::Enter);
    tui.snap("the dearer fish went grey");
    assert_eq!(
        cursor_row(&mut tui).as_deref(),
        Some(beside.display_name()),
        "the cursor waits on the row beside it, never back at the top"
    );
}

#[test]
fn selling_keeps_the_cursor_where_it_was() {
    let mut tui = Tui::with_size(DEFAULT_COLS, rows_for_every_fish());
    tui.film(
        Path::new(env!("CARGO_TARGET_TMPDIR")),
        "shop-keeps-its-place",
    );
    tui.clear_tank();
    for name in ["Ann", "Bea", "Cid"] {
        tui.run(&format!("/spawn merluza \"{name}\""));
    }
    tui.run("/add coffee 3");
    tui.run("/add bait 3");
    tui.run("/shop");
    tui.select("Sell");
    tui.key(KeyCode::Enter);
    tui.select("Bait (3)");
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Enter);
    tui.snap("one bait sold");
    assert_eq!(
        cursor_row(&mut tui).as_deref(),
        Some("Bait (2)"),
        "the cursor stays on what is left of it"
    );
    tui.select("Bea (Merluza)");
    let below = {
        tui.key(KeyCode::Down);
        let below = cursor_row(&mut tui).expect("a row below Bea");
        tui.key(KeyCode::Up);
        below
    };
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Enter);
    tui.snap("Bea sold");
    assert_eq!(
        cursor_row(&mut tui),
        Some(below),
        "the row that took its place is selected"
    );
}

#[test]
fn past_thirty_the_food_counter_moves_by_thirty() {
    let mut tui = shop_with(Money::from(10_000u32));
    tui.key(KeyCode::Enter);
    tui.select("Food");
    tui.key(KeyCode::Enter);
    for _ in 1..30 {
        tui.key(KeyCode::Right);
    }
    tui.screen().expect_find("< 30 >");
    tui.key(KeyCode::Right);
    tui.screen().expect_find("< 60 >");
    tui.key(KeyCode::Right);
    tui.screen().expect_find("< 90 >");
    tui.key(KeyCode::Left);
    tui.screen().expect_find("< 60 >");
    tui.key(KeyCode::Left);
    tui.key(KeyCode::Left);
    tui.screen().expect_find("< 29 >");
}

#[test]
fn the_food_counter_stops_at_what_the_purse_holds() {
    let mut tui = shop_with(Money::from(45u32));
    tui.key(KeyCode::Enter);
    tui.select("Food");
    tui.key(KeyCode::Enter);
    for _ in 1..30 {
        tui.key(KeyCode::Right);
    }
    tui.key(KeyCode::Right);
    tui.screen().expect_find("< 45 >");
    tui.key(KeyCode::Left);
    tui.screen().expect_find("< 30 >");
}

fn hold_down(tui: &mut Tui, first_delay: usize, repeats: usize, kind: KeyEventKind) -> usize {
    let mut dark = 0;
    let mut watch = |tui: &mut Tui| {
        tui.tick_n(1);
        if cursor_row(tui).is_none() {
            dark += 1;
        }
    };
    tui.key(KeyCode::Down);
    for _ in 0..first_delay {
        watch(tui);
    }
    for _ in 0..repeats {
        let mut repeat = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
        repeat.kind = kind;
        tui.app.handle_input(Event::Key(repeat));
        watch(tui);
    }
    tui.release(KeyCode::Down).tick_n(1);
    dark
}

const FIRST_REPEAT_TICKS: usize = 15;
const REPEAT_TICKS: usize = 45;

#[test]
fn a_held_arrow_never_blinks_the_cursor_away() {
    let mut tui = shop_with(Money::MAX / 2);
    tui.key(KeyCode::Enter).tick_n(1).release(KeyCode::Enter);
    tui.select(FISHES_ROW);
    tui.release(KeyCode::Down).tick_n(1);
    tui.key(KeyCode::Enter).tick_n(1).release(KeyCode::Enter);
    assert_eq!(
        hold_down(
            &mut tui,
            FIRST_REPEAT_TICKS,
            REPEAT_TICKS,
            KeyEventKind::Press
        ),
        0,
        "Windows repeats a held key as presses"
    );
    for _ in 0..REPEAT_TICKS {
        tui.key(KeyCode::Up);
    }
    tui.release(KeyCode::Up).tick_n(1);
    assert_eq!(
        hold_down(
            &mut tui,
            FIRST_REPEAT_TICKS,
            REPEAT_TICKS,
            KeyEventKind::Repeat
        ),
        0,
        "a kitty terminal repeats it as repeats"
    );
}
