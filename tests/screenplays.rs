use std::path::Path;

use fishtank::{
    app::Launch,
    fishes::{parts::Part, species::FishSpecies},
    loot::{CashValue, ConsumableKind, ItemKind, LootKind, MilkVariant},
    testing::{
        DEFAULT_COLS, DEFAULT_ROWS, Reel, Still, TerminalSize, play_name, plays_in, review, stage,
    },
    ui::{
        catch_overlay::{CatchOverlay, CatchState},
        layout::Screen,
    },
};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

const REEL_DIR: &str = env!("CARGO_TARGET_TMPDIR");
const SCREENPLAY_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/screenplays");
const CARD_COLS: u16 = 100;
const CARD_ROWS: u16 = 26;
type Catch = (&'static str, fn() -> LootKind);

const SMALL_CARD_SIZES: [(u16, u16); 3] = [(60, 18), (40, 14), (28, 10)];

#[test]
fn every_screenplay_performs_to_the_end_and_draws_no_broken_border() {
    let plays = plays_in(Path::new(SCREENPLAY_DIR));
    assert!(
        !plays.is_empty(),
        "tests/screenplays holds at least one play"
    );
    let size = TerminalSize::new(DEFAULT_COLS, DEFAULT_ROWS);
    let failures: Vec<String> = plays
        .iter()
        .filter_map(|path| {
            let name = play_name(path);
            review(
                stage(path, size, Launch::Debug, Path::new(REEL_DIR), &name),
                &name,
            )
        })
        .collect();
    assert!(
        failures.is_empty(),
        "open {REEL_DIR}/reels to see the screens:\n{}",
        failures.join("\n")
    );
}

fn card_at(label: &str, loot: LootKind, (cols, rows): (u16, u16)) -> Still {
    let state = CatchState::new(loot, &mut rand::rng());
    let area = Rect::new(0, 0, cols, rows);
    let mut buffer = Buffer::empty(area);
    CatchOverlay::new(&state, Screen::only(area)).render(area, &mut buffer);
    Still::of(label, &buffer)
}

fn card(label: &str, kind: ConsumableKind) -> Still {
    card_at(
        label,
        LootKind::Item(ItemKind::Consumable(kind)),
        (CARD_COLS, CARD_ROWS),
    )
}

#[test]
fn every_robotics_catch_card_draws_whole() {
    let mut reel = Reel::new();
    for &part in Part::ALL {
        reel.push(card(
            &format!("Catch · {}", part.display_name()),
            ConsumableKind::Part(part),
        ));
    }
    reel.push(card("Catch · Blank Wafer", ConsumableKind::BlankWafer));
    reel.push(card(
        "Catch · Blank Circuit Blueprint",
        ConsumableKind::BlankBlueprint,
    ));
    reel.push(card("Catch · Fabricator", ConsumableKind::Fabricator));
    reel.push(card(
        "Control · Coffee, an older card",
        ConsumableKind::Coffee,
    ));
    reel.push(card(
        "Control · Computer, the tallest older card",
        ConsumableKind::Computer,
    ));
    reel.save(Path::new(REEL_DIR), "robotics-catch-cards")
        .expect("the reel is writable");
    let report = reel.flaw_report();
    assert!(report.is_empty(), "{report}");
}

#[test]
fn every_seed_catch_card_draws_whole() {
    let mut reel = Reel::new();
    for seed in ConsumableKind::seeds() {
        for size in [(CARD_COLS, CARD_ROWS)].into_iter().chain(SMALL_CARD_SIZES) {
            reel.push(card_at(
                &format!("{} · {}×{}", seed.display_name(), size.0, size.1),
                LootKind::Item(ItemKind::Consumable(seed)),
                size,
            ));
        }
    }
    reel.save(Path::new(REEL_DIR), "seed-catch-cards")
        .expect("the reel is writable");
    let report = reel.flaw_report();
    assert!(report.is_empty(), "{report}");
}

#[test]
fn a_catch_card_on_a_small_screen_wraps_its_art_above_the_text_and_keeps_its_hint() {
    let catches: [Catch; 5] = [
        ("fish", || LootKind::Fish(FishSpecies::Salmon)),
        ("cash", || LootKind::Cash(CashValue::Hundred)),
        ("computer", || {
            LootKind::Item(ItemKind::Consumable(ConsumableKind::Computer))
        }),
        ("fabricator", || {
            LootKind::Item(ItemKind::Consumable(ConsumableKind::Fabricator))
        }),
        ("blank blueprint", || {
            LootKind::Item(ItemKind::Consumable(ConsumableKind::BlankBlueprint))
        }),
    ];
    let mut reel = Reel::new();
    for size in SMALL_CARD_SIZES {
        for (name, loot) in catches {
            let still = card_at(&format!("{name} · {}×{}", size.0, size.1), loot(), size);
            let hint = if name == "fish" {
                "ENTER capture"
            } else {
                "ESC/q close"
            };
            assert!(
                still.text().contains(hint),
                "the {name} card lost its hint at {size:?}:\n{}",
                still.text()
            );
            reel.push(still);
        }
    }
    reel.save(Path::new(REEL_DIR), "small-catch-cards")
        .expect("the reel is writable");
    let report = reel.flaw_report();
    assert!(report.is_empty(), "{report}");
}

#[test]
fn every_milk_catch_card_draws_whole() {
    let mut reel = Reel::new();
    for &milk in MilkVariant::ALL {
        reel.push(card(
            &format!("Catch · {}", milk.display_name()),
            ConsumableKind::Milk(milk),
        ));
        for size in SMALL_CARD_SIZES {
            reel.push(card_at(
                &format!("{} · {}×{}", milk.display_name(), size.0, size.1),
                LootKind::Item(ItemKind::Consumable(ConsumableKind::Milk(milk))),
                size,
            ));
        }
    }
    reel.save(Path::new(REEL_DIR), "milk-catch-cards")
        .expect("the reel is writable");
    let report = reel.flaw_report();
    assert!(report.is_empty(), "{report}");
}

const ANTENNA_STEM: &str = "‖";
const BOTFISH_EYE: &str = "º";
const SMALLEST_JOURNEY_ROWS: u16 = 14;

#[test]
fn a_botfish_on_a_catch_card_keeps_its_antenna_and_never_loses_its_body() {
    let mut reel = Reel::new();
    for size in [(CARD_COLS, CARD_ROWS)].into_iter().chain(SMALL_CARD_SIZES) {
        let still = card_at(
            &format!("botfish · {}×{}", size.0, size.1),
            LootKind::Fish(FishSpecies::Botfish),
            size,
        );
        let text = still.text();
        assert!(
            text.contains(BOTFISH_EYE),
            "the botfish lost its body at {size:?}:
{text}"
        );
        assert!(
            size.1 < SMALLEST_JOURNEY_ROWS || text.contains(ANTENNA_STEM),
            "the botfish lost its antenna at {size:?}:
{text}"
        );
        reel.push(still);
    }
    reel.save(Path::new(REEL_DIR), "botfish-catch-card")
        .expect("the reel is writable");
    let report = reel.flaw_report();
    assert!(report.is_empty(), "{report}");
}
