use std::path::Path;

use crossterm::event::KeyCode;
use fishtank::{
    colors::{DARK_GRAY, WHITE},
    economy::Money,
    fishes::species::FishSpecies,
    testing::Tui,
};
use ratatui::style::Color;

const SELECTED: &str = "> ";
const FISHES_ROW: &str = "Fishes";
const ROBOTICS_ROW: &str = "Robotics";

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
    let mut tui = Tui::new();
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
