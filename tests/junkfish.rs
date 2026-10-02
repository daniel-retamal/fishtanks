use crossterm::event::KeyCode;
use fishtank::{
    fishes::species::{FishSpecies, Habitat, SizeCategory},
    loot::{JUNK_PER_JUNKFISH, StockItem},
    tank::Tank,
    testing::Tui,
};

const JUNKFISH_FLOOR: u64 = 100_000;
const LEFT_OVER: u32 = 1;

fn junkfish() -> FishSpecies {
    FishSpecies::from_junk().expect("some fish is made of junk")
}

fn player_with_junk(junk: u32) -> Tui {
    let mut tui = Tui::as_player(100, 30);
    tui.clear_tank();
    tui.app.inventory.insert(StockItem::Junk, junk);
    tui
}

fn junkfish_named<'a>(tui: &'a Tui, name: &str) -> Option<&'a fishtank::fishes::fish::Fish> {
    tui.app
        .tanks
        .iter()
        .flat_map(|tank| tank.fish.iter())
        .find(|fish| fish.name == name && fish.species == junkfish())
}

fn assemble_from_the_inventory(tui: &mut Tui, name: &str) {
    tui.run("/inventory");
    tui.key(KeyCode::Enter);
    tui.screen().expect_find("a Junkfish made of 100 Junk!");
    tui.type_text(name);
    tui.key(KeyCode::Enter);
}

fn fill(tank: &mut Tank) {
    let mut n = 0;
    while !tank.is_full() {
        tank.spawn_fish(
            FishSpecies::Merluza,
            format!("Filler {n}"),
            &mut rand::rng(),
        );
        n += 1;
    }
}

#[test]
fn a_junkfish_is_made_of_junk_and_nothing_else() {
    let config = junkfish().config();
    assert_eq!(config.habitat, Habitat::Junkpile);
    assert!(!config.buyable);
    assert!(!FishSpecies::all_wild().contains(&junkfish()));
    assert!(!FishSpecies::all_buyable().contains(&junkfish()));
    assert!(
        junkfish().is_obtainable(),
        "a cheat or a wish can still give one"
    );
}

#[test]
fn a_hundred_junk_become_a_named_junkfish_and_the_rest_stays_junk() {
    let mut tui = player_with_junk(2 * JUNK_PER_JUNKFISH + LEFT_OVER);
    assemble_from_the_inventory(&mut tui, "Oscar");
    assert!(junkfish_named(&tui, "Oscar").is_some());
    assert_eq!(
        tui.app.stock_of(StockItem::Junk),
        JUNK_PER_JUNKFISH + LEFT_OVER
    );

    assemble_from_the_inventory(&mut tui, "Trashy");
    assert!(junkfish_named(&tui, "Trashy").is_some());
    assert_eq!(tui.app.stock_of(StockItem::Junk), LEFT_OVER);

    tui.run("/inventory");
    tui.key(KeyCode::Enter);
    tui.screen().expect_absent("Junk to the Junkfish!");
    assert_eq!(tui.app.stock_of(StockItem::Junk), LEFT_OVER);
}

#[test]
fn exactly_a_hundred_junk_leave_none_behind() {
    let mut tui = player_with_junk(JUNK_PER_JUNKFISH);
    assemble_from_the_inventory(&mut tui, "Oscar");
    assert!(junkfish_named(&tui, "Oscar").is_some());
    assert_eq!(tui.app.stock_of(StockItem::Junk), 0);
    assert!(!tui.app.inventory.contains_key(&StockItem::Junk));
}

#[test]
fn ninety_nine_junk_are_just_junk() {
    let mut tui = player_with_junk(JUNK_PER_JUNKFISH - 1);
    tui.run("/inventory");
    tui.screen().expect_find("No");
    tui.key(KeyCode::Enter);
    tui.screen().expect_absent("Junk to the Junkfish!");
    tui.key(KeyCode::Esc);
    tui.run("/consume junk");
    tui.screen().expect_absent("Junk to the Junkfish!");
    assert_eq!(tui.app.stock_of(StockItem::Junk), JUNK_PER_JUNKFISH - 1);
}

#[test]
fn the_consume_command_opens_the_same_popup() {
    let mut tui = player_with_junk(JUNK_PER_JUNKFISH);
    tui.run("/consume junk");
    tui.screen().expect_find("Junk to the Junkfish!");
    tui.type_text("Oscar");
    tui.key(KeyCode::Enter);
    assert!(junkfish_named(&tui, "Oscar").is_some());
}

#[test]
fn walking_away_from_the_popup_keeps_every_piece_of_junk() {
    let mut tui = player_with_junk(JUNK_PER_JUNKFISH);
    tui.run("/consume junk");
    tui.type_text("Oscar");
    tui.key(KeyCode::Esc);
    assert!(junkfish_named(&tui, "Oscar").is_none());
    assert_eq!(tui.app.stock_of(StockItem::Junk), JUNK_PER_JUNKFISH);
}

#[test]
fn with_no_room_anywhere_the_junk_stays_junk() {
    let mut tui = player_with_junk(JUNK_PER_JUNKFISH);
    for tank in &mut tui.app.tanks {
        fill(tank);
    }
    tui.run("/consume junk");
    tui.screen().expect_absent("Junk to the Junkfish!");
    assert_eq!(tui.app.stock_of(StockItem::Junk), JUNK_PER_JUNKFISH);
}

#[test]
fn a_junkfish_sells_for_a_jackpot() {
    let mut tui = player_with_junk(JUNK_PER_JUNKFISH);
    assemble_from_the_inventory(&mut tui, "Oscar");
    let worth = junkfish_named(&tui, "Oscar").unwrap().sell_value();
    assert!(worth >= JUNKFISH_FLOOR, "${worth}");
    let before = tui.app.purse.shown();
    tui.run("/sell fish \"Oscar\"");
    assert!(junkfish_named(&tui, "Oscar").is_none());
    assert_eq!(tui.app.purse.shown(), before.map(|cash| cash + worth));
}

#[test]
fn every_size_of_junkfish_is_worth_at_least_the_floor() {
    for size in SizeCategory::ALL {
        assert!(junkfish().appraisal(size, 0) >= JUNKFISH_FLOOR);
    }
}

#[test]
fn a_god_can_still_give_a_junkfish() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/give junkfish \"Oscar\"");
    assert!(junkfish_named(&tui, "Oscar").is_some());
}
