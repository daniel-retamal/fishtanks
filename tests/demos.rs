use std::path::Path;

use crossterm::event::KeyCode;
use fishtank::{
    economy::Money,
    entities::food::{FOOD_BUY_PRICE, FOOD_WEIGHT_GAIN_G},
    fishes::parts::{RIG_WAIT_MAX_SECS, RIG_WAIT_MIN_SECS},
    fishes::{
        parts::{Part, PartTier},
        species::FishSpecies,
    },
    ledger::{Direction, Flow},
    loot::{ConsumableKind, StockItem},
    tank::FEED_PORTION,
    testing::Tui,
    ui::fishing_overlay::Temper,
};

const REEL_DIR: &str = env!("CARGO_TARGET_TMPDIR");
const NO_SHOT: Option<&str> = None;

fn filmed(name: &str) -> Tui {
    let mut tui = Tui::new();
    tui.film(Path::new(REEL_DIR), name);
    tui.clear_tank();
    tui.stake();
    tui
}

fn assert_no_broken_borders(tui: &Tui) {
    let report = tui.reel().flaw_report();
    assert!(
        report.is_empty(),
        "a still drew a broken border — open {REEL_DIR}/reels to see it:\n{report}"
    );
}

fn spawn(tui: &mut Tui, name: &str) {
    tui.run(&format!("/spawn botfish \"{name}\""));
}

fn install(tui: &mut Tui, part: &str, downs: usize) {
    install_on_rows(tui, part, &[downs], NO_SHOT);
}

fn install_on_rows(tui: &mut Tui, part: &str, rows: &[usize], shot: Option<&str>) {
    tui.run(&format!("/consume {part}"));
    let mut cursor = 0;
    for &row in rows {
        for _ in cursor..row {
            tui.key(KeyCode::Down);
        }
        cursor = row;
        tui.key(KeyCode::Enter);
    }
    if let Some(label) = shot {
        tui.snap(label);
    }
    tui.key(KeyCode::Esc);
}

fn wire(tui: &mut Tui, name: &str, listens: &str, drives: &str, shot: Option<&str>) {
    tui.run(&format!("/program \"{name}\""));
    tui.type_text(listens);
    tui.key(KeyCode::Down);
    tui.type_text(drives);
    if let Some(label) = shot {
        tui.snap(label);
    }
    tui.key(KeyCode::Enter);
}

fn switch_off(tui: &mut Tui, name: &str, shot: Option<&str>) {
    tui.run(&format!("/program \"{name}\""));
    for _ in 0.."one".len() {
        tui.key(KeyCode::Backspace);
    }
    if let Some(label) = shot {
        tui.snap(label);
    }
    tui.key(KeyCode::Enter);
}

fn switch_on(tui: &mut Tui, name: &str) {
    tui.run(&format!("/program \"{name}\""));
    tui.type_text("one");
    tui.key(KeyCode::Enter);
}

fn level(tui: &Tui, channel: &str) -> bool {
    tui.app.tanks[tui.app.current_tank].channels.level(channel)
}

const SETTLE: usize = 6;
const COILS_STOCKED: usize = 12;

#[test]
fn the_phase_two_demo_written_in_the_plan_still_runs_keystroke_for_keystroke() {
    let mut tui = filmed("phase-2-demo");
    tui.run("/fps 1");
    tui.run("/clock 1");
    tui.run(&format!("/add inverter coil {COILS_STOCKED}"));
    tui.run("/add delay spool 1");
    tui.snap("0 · The bench: an empty tank, twelve coils and a spool in stock");

    spawn(&mut tui, "osc");
    install_on_rows(
        &mut tui,
        "inverter coil",
        &[0],
        Some("1 · /consume inverter coil — the picker after ENTER on Osc"),
    );
    wire(
        &mut tui,
        "Osc",
        "ring",
        "ring",
        Some("1 · /program \"Osc\" — listens ring, drives ring, before ENTER"),
    );
    tui.tick_n(1);
    let first = level(&tui, "ring");
    tui.tick_n(1);
    assert_ne!(level(&tui, "ring"), first, "the oscillator toggles");
    tui.tick_n(1);
    assert_eq!(level(&tui, "ring"), first, "and toggles back");
    tui.snap("1 · The ring oscillator swimming");

    spawn(&mut tui, "one");
    spawn(&mut tui, "sa");
    install(&mut tui, "inverter coil", 1);
    wire(
        &mut tui,
        "One",
        "",
        "one",
        Some("2 · /program \"One\" — listens left empty, drives one"),
    );
    wire(&mut tui, "Sa", "one", "a", NO_SHOT);
    tui.tick_n(SETTLE);
    assert!(level(&tui, "one"), "the rail is high");
    assert!(level(&tui, "a"), "and the switch passes it");
    switch_off(
        &mut tui,
        "Sa",
        Some("2 · THE FLIP, off — Sa's listens field backspaced empty"),
    );
    tui.tick_n(SETTLE);
    assert!(!level(&tui, "a"), "the switch drops its channel");
    switch_on(&mut tui, "Sa");
    tui.tick_n(SETTLE);
    assert!(level(&tui, "a"), "and raises it again");

    for name in ["sb", "na", "nb", "and"] {
        spawn(&mut tui, name);
    }
    install_on_rows(
        &mut tui,
        "inverter coil",
        &[4, 5, 6],
        Some("3 · /consume inverter coil — three installs in one picker"),
    );
    wire(&mut tui, "Sb", "one", "b", NO_SHOT);
    wire(&mut tui, "Na", "a", "na", NO_SHOT);
    wire(&mut tui, "Nb", "b", "nb", NO_SHOT);
    wire(
        &mut tui,
        "And",
        "na, nb",
        "and",
        Some("3 · /program \"And\" — two channels, one comma"),
    );
    tui.tick_n(SETTLE);
    assert!(level(&tui, "and"), "1 AND 1");
    tui.run("/circuit");
    tui.snap("3 · /circuit — the AND gate with both switches on");
    tui.key(KeyCode::Esc);
    switch_off(&mut tui, "Sb", NO_SHOT);
    tui.tick_n(SETTLE);
    assert!(!level(&tui, "and"), "1 AND 0");
    switch_off(&mut tui, "Sa", NO_SHOT);
    tui.tick_n(SETTLE);
    assert!(!level(&tui, "and"), "0 AND 0");
    switch_on(&mut tui, "Sb");
    tui.tick_n(SETTLE);
    assert!(!level(&tui, "and"), "0 AND 1");

    spawn(&mut tui, "px");
    spawn(&mut tui, "qx");
    install_on_rows(&mut tui, "inverter coil", &[7, 8], NO_SHOT);
    wire(&mut tui, "Px", "na, b", "xor", NO_SHOT);
    wire(&mut tui, "Qx", "a, nb", "xor", NO_SHOT);
    tui.tick_n(SETTLE);
    assert!(level(&tui, "xor"), "0 XOR 1");
    switch_on(&mut tui, "Sa");
    tui.tick_n(SETTLE);
    assert!(!level(&tui, "xor"), "1 XOR 1");
    assert!(
        level(&tui, "and"),
        "and the AND still answers on its own channel"
    );

    for name in ["ss", "sr", "ql", "qn"] {
        spawn(&mut tui, name);
    }
    install_on_rows(&mut tui, "inverter coil", &[11, 12], NO_SHOT);
    wire(&mut tui, "Ss", "", "s", NO_SHOT);
    wire(&mut tui, "Sr", "", "r", NO_SHOT);
    wire(&mut tui, "Ql", "r, qn", "ql", NO_SHOT);
    wire(&mut tui, "Qn", "s, ql", "qn", NO_SHOT);
    switch_on(&mut tui, "Ss");
    tui.tick_n(SETTLE);
    assert!(level(&tui, "ql"), "setting raises ql");
    switch_off(&mut tui, "Ss", NO_SHOT);
    tui.tick_n(SETTLE);
    assert!(level(&tui, "ql"), "and it holds after the input drops");
    switch_on(&mut tui, "Sr");
    tui.tick_n(SETTLE);
    assert!(!level(&tui, "ql"), "resetting clears it");
    switch_off(&mut tui, "Sr", NO_SHOT);
    tui.tick_n(SETTLE);
    assert!(!level(&tui, "ql"), "and it holds cleared");

    for name in ["sx", "dd", "nx", "pu"] {
        spawn(&mut tui, name);
    }
    install(&mut tui, "delay spool", 14);
    install_on_rows(&mut tui, "inverter coil", &[15, 16], NO_SHOT);
    wire(&mut tui, "Sx", "", "x", NO_SHOT);
    wire(&mut tui, "Dd", "x", "xd", NO_SHOT);
    wire(&mut tui, "Nx", "x", "nx", NO_SHOT);
    wire(&mut tui, "Pu", "nx, xd", "pulse", NO_SHOT);
    tui.tick_n(SETTLE);
    assert!(!level(&tui, "pulse"), "nothing has happened yet");

    switch_on(&mut tui, "Sx");
    let mut fired = 0;
    for _ in 0..8 {
        tui.tick_n(1);
        if level(&tui, "pulse") {
            fired += 1;
        }
    }
    assert_eq!(fired, 1, "the edge fires for exactly one stage");
    tui.snap("5 · The finished 17-fish board swimming");

    tui.run("/circuit");
    let screen = tui.screen();
    assert!(
        screen.contains("Circuit#signal"),
        "the table opens on the finished board"
    );
    assert!(
        screen.contains("Osc"),
        "and every build is still on it:\n{}",
        screen.text()
    );
    tui.snap("Finish · /circuit — the table");
    tui.key(KeyCode::Tab);
    assert!(
        !tui.screen().contains("too big to draw"),
        "a 17-fish board holding two feedback loops is still small enough to draw"
    );
    assert!(
        tui.screen().contains("[Ql]"),
        "and the latch is on it:\n{}",
        tui.screen().text()
    );
    tui.snap("Finish · /circuit then TAB — the schematic");
    tui.key(KeyCode::Esc);
    tui.run("/nets");
    tui.snap("Finish · /nets — each fish's driven channel over it");

    assert_no_broken_borders(&tui);
}

const CLEAR_STROKES: usize = 48;

fn set_fields(tui: &mut Tui, name: &str, entries: &[(usize, &str)], shot: Option<&str>) {
    tui.run(&format!("/program \"{name}\""));
    let mut cursor = 0;
    for &(field, text) in entries {
        for _ in cursor..field {
            tui.key(KeyCode::Down);
        }
        cursor = field;
        for _ in 0..CLEAR_STROKES {
            tui.key(KeyCode::Backspace);
        }
        tui.type_text(text);
    }
    if let Some(label) = shot {
        tui.snap(label);
    }
    tui.key(KeyCode::Enter);
}

const MUTANT_SEED: usize = 4;
const FOOD_STOCKED: i64 = 4000;
const RIPE_THRESHOLD: &str = "550";
const RESTOCK_CEILING: &str = "10";
const BREEDER_FLOOR: &str = "3";
const REAPER_SETTLE: usize = 12;
const SCRIPT_TICKS: usize = 40;

fn spread(shoal: &[(String, Money)]) -> Money {
    let values = shoal.iter().map(|(_, value)| *value);
    values.clone().max().unwrap_or(0) - values.min().unwrap_or(0)
}

fn mutants(tui: &Tui) -> Vec<(String, Money)> {
    tui.app.tanks[tui.app.current_tank]
        .fish
        .iter()
        .filter(|f| f.species == FishSpecies::Mutantfish)
        .map(|f| (f.name.clone(), f.sell_value()))
        .collect()
}

fn stock_the_reapers_bench(tui: &mut Tui) {
    tui.run("/add inverter coil 6");
    tui.run("/add delay spool 3");
    tui.run("/add shoal counter 2");
    tui.run("/add assay scale 1");
    tui.run("/add command module 3");
    tui.run(&format!("/add food {FOOD_STOCKED}"));
    tui.run("/fps 1");
    tui.run("/clock 1");
}

fn build_the_reaper(tui: &mut Tui) {
    for name in [
        "clk", "assay", "count", "floor", "nr", "ns", "nc", "harvest", "reaper", "nt", "restock",
        "breeder", "feeder",
    ] {
        spawn(tui, name);
    }
    install(tui, "delay spool", 0);
    install(tui, "delay spool", 0);
    install(tui, "delay spool", 0);
    install(tui, "assay scale", 1);
    install_on_rows(tui, "shoal counter", &[2, 3], NO_SHOT);
    install_on_rows(
        tui,
        "inverter coil",
        &[0, 4, 5, 6, 7, 9, 10],
        Some("2 · /consume inverter coil — seven coils, one picker"),
    );
    tui.run("/consume command module");
    tui.key(KeyCode::Down).key(KeyCode::Down).key(KeyCode::Down);
    tui.key(KeyCode::Down).key(KeyCode::Down).key(KeyCode::Down);
    tui.key(KeyCode::Down).key(KeyCode::Down);
    tui.key(KeyCode::Enter);
    for _ in 8..11 {
        tui.key(KeyCode::Down);
    }
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Down);
    tui.snap("2 · /consume command module — Reaper and Breeder done, cursor on Feeder");
    tui.key(KeyCode::Enter);
    assert!(
        !tui.screen().contains(COMMAND_MODULE_PICKER),
        "installing the last module in stock closes the picker by itself"
    );

    set_fields(tui, "Clk", &[(0, "clk"), (1, "clk")], NO_SHOT);
    set_fields(
        tui,
        "Assay",
        &[(2, "richest mutantfish"), (4, RIPE_THRESHOLD), (7, "ripe")],
        Some("2 · /program \"Assay\" — target, threshold and the over pin, before ENTER"),
    );
    set_fields(
        tui,
        "Count",
        &[(2, "mutantfish"), (3, RESTOCK_CEILING), (8, "thin")],
        Some("2 · /program \"Count\" — under wired to thin"),
    );
    set_fields(
        tui,
        "Floor",
        &[(2, "mutantfish"), (3, BREEDER_FLOOR), (7, "spare")],
        NO_SHOT,
    );
    set_fields(tui, "Nr", &[(0, "ripe"), (1, "nripe")], NO_SHOT);
    set_fields(tui, "Ns", &[(0, "spare"), (1, "nspare")], NO_SHOT);
    set_fields(tui, "Nc", &[(0, "clk"), (1, "nclk")], NO_SHOT);
    set_fields(
        tui,
        "Harvest",
        &[(0, "nripe, nspare, nclk"), (1, "harvest")],
        Some("2 · /program \"Harvest\" — the three-input NOR that is an AND"),
    );
    set_fields(tui, "Nt", &[(0, "thin"), (1, "nthin")], NO_SHOT);
    set_fields(
        tui,
        "Restock",
        &[(0, "nthin, nclk"), (1, "restock")],
        NO_SHOT,
    );
    set_fields(
        tui,
        "Reaper",
        &[(2, "harvest"), (4, "/sell fish {richest mutantfish}")],
        Some("2 · /program \"Reaper\" — fire wired, a selector in script line 1"),
    );
    set_fields(
        tui,
        "Breeder",
        &[(2, "restock"), (4, "/clone {cheapest mutantfish}")],
        NO_SHOT,
    );
    set_fields(tui, "Feeder", &[(2, "clk"), (4, "/feed")], NO_SHOT);
}

const FISHTANK_ROW: &str = "Fishtank";
const ROBOTICS_ROW: &str = "Robotics";
const COMPUTER_ROW: &str = "Computer";
const MATRIX_TANK_NAME: &str = "Zion";
const COMMAND_MODULE_PICKER: &str = "Command Module to the botfishes";
const COILS_BOUGHT_AT_THE_BENCH: u32 = 2;
const BENCH_TIERS: [&str; 5] = ["Fabric", "Senses", "Hands", "Peripherals", "Materials"];
const FABRIC_TIER: &str = "Fabric";
const CLOSE_THE_BENCH: usize = 4;
const REAPER_RUN_TICKS: usize = SCRIPT_TICKS * 30;
const POPULATION_CEILING: usize = 10;
const SNAPPER_SEED: usize = 3;
const FAT_SNAPPER: &str = "700";
const SNAPPER_CEILING: &str = "8";
const SNAPPER_CEILING_COUNT: usize = 8;
const SNAPPER_FLOOR: &str = "1";
const MUTANT_SPREAD_AFTER: Money = 120;

fn grow_the_matrixtank(tui: &mut Tui) {
    tui.run("/shop");
    tui.snap("0 · /shop — the front counter");
    tui.key(KeyCode::Enter);
    tui.select(FISHTANK_ROW);
    tui.snap("0 · Buy — the categories, cursor on Fishtank");
    tui.key(KeyCode::Enter);
    tui.screen().expect_absent(COMPUTER_ROW);
    tui.snap("0 · Fishtank — the catalogue sells tanks, never the items that grow them");
    for _ in 0..3 {
        tui.key(KeyCode::Esc);
    }
    tui.run("/give computer");
    tui.run("/consume computer");
    tui.snap("0 · The Computer names the Matrixtank it grows");
    tui.type_text(MATRIX_TANK_NAME);
    tui.key(KeyCode::Enter);
    tui.run("/switch \"Fishtank\"");
}

fn walk_the_whole_bench(tui: &mut Tui) {
    let screen = tui.screen();
    for tier in BENCH_TIERS {
        screen.expect_find(tier);
    }
    screen.expect_absent(Part::InverterCoil.display_name());
    tui.snap("0 · Robotics · the five tiers, by name");
    tui.select(FABRIC_TIER);
    tui.key(KeyCode::Enter);
    let mut fabric: Vec<ConsumableKind> = ConsumableKind::bench_stock()
        .into_iter()
        .filter(|kind| kind.bench_tier() == PartTier::Fabric)
        .collect();
    fabric.sort_by_key(|kind| (kind.buy_price(), kind.display_name()));
    let (first, last) = (fabric[0], fabric[fabric.len() - 1]);
    for _ in 0..fabric.len() {
        tui.key(KeyCode::Down);
    }
    tui.snap("0 · Fabric — the cursor on its last row");
    tui.screen()
        .expect_find(&format!("> {}", last.display_name()));
    for _ in 0..fabric.len() {
        tui.key(KeyCode::Up);
    }
    tui.screen()
        .expect_find(&format!("> {}", first.display_name()));
}

fn buy_coils_at_the_robotics_bench(tui: &mut Tui) {
    grow_the_matrixtank(tui);
    tui.run("/shop");
    tui.key(KeyCode::Enter);
    tui.select(ROBOTICS_ROW);
    tui.snap("0 · Buy — the Matrixtank unlocked Robotics");
    tui.key(KeyCode::Enter);
    walk_the_whole_bench(tui);
    tui.select(Part::InverterCoil.display_name());
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Right);
    tui.snap("0 · The counter on Inverter Coil, → pressed once");
    tui.key(KeyCode::Enter);
    tui.snap("0 · Back on the Fabric page after buying");
    for _ in 0..CLOSE_THE_BENCH {
        tui.key(KeyCode::Esc);
    }
}

fn coils_in_stock(tui: &Tui) -> u32 {
    tui.app
        .inventory
        .get(&StockItem::Consumable(ConsumableKind::Part(
            Part::InverterCoil,
        )))
        .copied()
        .unwrap_or(0)
}

fn sold(tui: &Tui, species: FishSpecies) -> usize {
    tui.app
        .graveyard
        .iter()
        .filter(|f| f.species == species)
        .count()
}

fn shoal_of(tui: &Tui, species: FishSpecies) -> Vec<(String, u32)> {
    tui.app.tanks[tui.app.current_tank]
        .fish
        .iter()
        .filter(|f| f.species == species)
        .map(|f| (f.name.clone(), f.weight_g))
        .collect()
}

fn convert_the_reaper_into_the_confectioner(tui: &mut Tui) {
    set_fields(
        tui,
        "Assay",
        &[(2, "heaviest snapper"), (3, "weight"), (4, FAT_SNAPPER)],
        Some("3 · /program \"Assay\" — retargeted onto the heaviest snapper by weight"),
    );
    set_fields(
        tui,
        "Count",
        &[(2, "snapper"), (3, SNAPPER_CEILING)],
        NO_SHOT,
    );
    set_fields(tui, "Floor", &[(2, "snapper"), (3, SNAPPER_FLOOR)], NO_SHOT);
    set_fields(
        tui,
        "Reaper",
        &[(4, "/sell fish {heaviest snapper}")],
        NO_SHOT,
    );
    set_fields(
        tui,
        "Breeder",
        &[(4, "/buy snapper")],
        Some("3 · /program \"Breeder\" — clone becomes buy"),
    );
}

#[test]
fn the_phase_three_demo_written_in_the_plan_still_runs_keystroke_for_keystroke() {
    let mut tui = filmed("phase-3-demo");

    buy_coils_at_the_robotics_bench(&mut tui);
    assert_eq!(
        coils_in_stock(&tui),
        COILS_BOUGHT_AT_THE_BENCH,
        "ENTER, →, ENTER at the bench buys exactly the two coils the demo promises"
    );

    stock_the_reapers_bench(&mut tui);
    for index in 0..MUTANT_SEED {
        tui.run(&format!("/spawn mutantfish \"seed{index}\""));
    }
    build_the_reaper(&mut tui);

    tui.tick_n(REAPER_SETTLE);
    assert!(level(&tui, "spare"), "four fish clear the breeder floor");
    assert!(level(&tui, "thin"), "and four is under the restock ceiling");
    assert!(
        !level(&tui, "ripe"),
        "but nothing is fat enough to harvest yet"
    );
    tui.snap("2 · Twelve stages later — the Reaper's board swimming");

    let purse = tui.app.purse.balance();
    let mut spreads: Vec<Money> = (0..REAPER_RUN_TICKS / SCRIPT_TICKS)
        .map(|_| {
            tui.tick_n(SCRIPT_TICKS);
            spread(&mutants(&tui))
        })
        .collect();
    tui.snap("2 · Twenty minutes later — cash up, shoal under its ceiling");

    assert!(
        tui.app.purse.balance() > purse,
        "the Reaper sold unattended: {purse} -> {}",
        tui.app.purse.balance()
    );
    assert!(
        sold(&tui, FishSpecies::Mutantfish) > 0,
        "and what it sold went to the graveyard, as any sale does"
    );
    let shoal = mutants(&tui);
    assert!(
        !shoal.is_empty() && shoal.len() <= POPULATION_CEILING,
        "the farm never eats its seed corn and never overruns the tank: {shoal:?}"
    );
    spreads.sort_unstable();
    let typical = spreads[spreads.len() / 2];
    assert!(
        typical < MUTANT_SPREAD_AFTER,
        "selling the richest and cloning the cheapest keeps the shoal flat between mutations: \
         a median spread of ${typical} over the run"
    );

    tui.run("/index");
    tui.snap("2 · /index — the clone dynasty");
    tui.key(KeyCode::Esc);

    tui.run("/circuit");
    let screen = tui.screen();
    assert!(screen.contains("Circuit#signal"));
    assert!(
        screen.contains("Command Module"),
        "the actuators are on the board:\n{}",
        screen.text()
    );
    tui.snap("2 · /circuit — the Reaper's table");
    tui.key(KeyCode::Tab);
    assert!(
        !tui.screen().contains("too big to draw"),
        "thirteen fish still draw as a schematic"
    );
    tui.snap("2 · /circuit then TAB — the Reaper's schematic");
    tui.key(KeyCode::Esc);

    for index in 0..SNAPPER_SEED {
        tui.run(&format!("/spawn snapper \"pen{index}\""));
    }
    convert_the_reaper_into_the_confectioner(&mut tui);
    let bought_before = ledger_line(&tui, Direction::Out, Flow::Fish);
    tui.tick_n(REAPER_RUN_TICKS);
    tui.snap("3 · The Confectioner, twenty minutes in");

    let pen = shoal_of(&tui, FishSpecies::Snapper);
    assert!(
        ledger_line(&tui, Direction::Out, Flow::Fish) > bought_before,
        "the Breeder now buys snappers instead of cloning mutants: {pen:?}"
    );
    assert!(
        sold(&tui, FishSpecies::Snapper) > 0,
        "a snapper that crossed the line went to the till: {pen:?}"
    );
    assert!(
        pen.len() <= SNAPPER_CEILING_COUNT,
        "and the pen never overruns its own ceiling: {pen:?}"
    );

    tui.run("/nets");
    tui.snap("Finish · /nets — the farm labelled with its channels");

    assert_no_broken_borders(&tui);
}

const FULL_ADDER: &str = "Full Adder";
const ADDER_TANK: &str = "Adder";
const ADDER_BOARD: [(&str, &str, &str, &str); 9] = [
    ("n1", "N1", "a, b", "n1"),
    ("n2", "N2", "a, n1", "n2"),
    ("n3", "N3", "b, n1", "n3"),
    ("n4", "N4", "n2, n3", "n4"),
    ("n5", "N5", "n4, cin", "n5"),
    ("n6", "N6", "n4, n5", "n6"),
    ("n7", "N7", "cin, n5", "n7"),
    ("sum", "Sum", "n6, n7", "sum"),
    ("cout", "Cout", "n1, n5", "cout"),
];
const ADDER_BITS: usize = 4;
const COILS_FOR_THE_ADDERS: u32 = 46;
const WAFERS_FOR_FOUR_ETCHES: u32 = 72;
const FIRST_HOST_ROW: usize = 9;
const RAIL_ROW: usize = 13;
const CHIP_A_FIELD: usize = 2;
const CHIP_CIN_FIELD: usize = 4;
const CHIP_COUT_FIELD: usize = 5;
const CHIP_SUM_FIELD: usize = 6;
const ADDER_SETTLE: usize = 8;
const CLOSE_THE_SHOP: usize = 3;
const FIRST_PROBE_STAGE: usize = 3;
const RIPPLE_WATCHED_STAGES: usize = 7;

fn probe_lit(tui: &Tui, name: &str) -> bool {
    tui.app.tanks[tui.app.current_tank]
        .fish
        .iter()
        .find(|fish| fish.name == name)
        .and_then(|fish| fish.script())
        .is_some_and(|bot| bot.output_level())
}

fn carry_probe(index: usize) -> String {
    format!("C{index}")
}

fn sum_probe(index: usize) -> String {
    format!("S{index}")
}

type FirstStages = Vec<Option<usize>>;

fn watch_the_carry(
    tui: &mut Tui,
    carrying: bool,
    shot: Option<&str>,
) -> (FirstStages, FirstStages) {
    let mut carries = vec![None; ADDER_BITS];
    let mut sums = vec![None; ADDER_BITS];
    for stage in 1..=RIPPLE_WATCHED_STAGES {
        tui.tick_n(1);
        if let Some(label) = shot {
            tui.snap(&format!("{label} {stage} after the flip"));
        }
        for index in 0..ADDER_BITS {
            if carries[index].is_none() && probe_lit(tui, &carry_probe(index + 1)) == carrying {
                carries[index] = Some(stage);
            }
            if sums[index].is_none() && probe_lit(tui, &sum_probe(index)) != carrying {
                sums[index] = Some(stage);
            }
        }
    }
    (carries, sums)
}

fn design_the_full_adder(tui: &mut Tui) {
    for (spawned, _, _, _) in ADDER_BOARD {
        spawn(tui, spawned);
    }
    let rows: Vec<usize> = (0..ADDER_BOARD.len()).collect();
    install_on_rows(
        tui,
        "inverter coil",
        &rows,
        Some("1 · /consume inverter coil — nine installs in one picker"),
    );
    for (_, stored, listens, drives) in ADDER_BOARD {
        let shot = (stored == "Cout").then_some("1 · /program \"Cout\" — the last of the nine");
        wire(tui, stored, listens, drives, shot);
    }
    tui.tick_n(ADDER_SETTLE);
    assert!(!level(tui, "sum") && !level(tui, "cout"), "0 + 0 + 0 is 0");
    tui.run("/circuit");
    tui.key(KeyCode::Tab);
    assert!(tui.screen().contains("[Cout]"), "{}", tui.screen().text());
    tui.snap("1 · /circuit then TAB — nine coils are a full adder");
    tui.key(KeyCode::Esc);
}

fn capture_the_full_adder(tui: &mut Tui) {
    tui.run("/consume blank circuit blueprint");
    tui.type_text("full adder");
    tui.snap("2 · The naming popup, before ENTER");
    tui.key(KeyCode::Enter);
    assert!(
        tui.app
            .blueprints
            .iter()
            .any(|kept| kept.name == FULL_ADDER),
        "the design is kept as Full Adder"
    );
    tui.run("/foundry");
    assert!(
        tui.screen().contains("a, b, cin → cout, sum"),
        "{}",
        tui.screen().text()
    );
    tui.snap("2 · /foundry — the pins, the bill, the schematic");
    tui.key(KeyCode::Esc);
    tui.screen().expect_absent("─ Foundry ");
}

fn open_a_second_tank(tui: &mut Tui) {
    tui.run("/buy fishtank");
    tui.type_text("adder");
    tui.snap("3 · /buy fishtank — the naming popup");
    tui.key(KeyCode::Enter);
    for _ in 0..CLOSE_THE_SHOP {
        tui.key(KeyCode::Esc);
    }
    assert_eq!(tui.app.tanks[tui.app.current_tank].name, ADDER_TANK);
    tui.screen().expect_absent("Price/unit");
}

fn etch_four_adders(tui: &mut Tui) {
    for index in 0..ADDER_BITS {
        spawn(tui, &format!("fa{index}"));
    }
    tui.run("/foundry");
    tui.key(KeyCode::Char('e'));
    for _ in 0..FIRST_HOST_ROW {
        tui.key(KeyCode::Down);
    }
    tui.snap("4 · E in /foundry — the picker, cursor walked down to Fa0");
    tui.key(KeyCode::Enter);
    tui.screen().expect_absent("─ Foundry ");
    for index in 1..ADDER_BITS {
        tui.run(&format!("/etch \"{FULL_ADDER}\" \"Fa{index}\""));
    }
    let chips = tui.app.tanks[tui.app.current_tank]
        .fish
        .iter()
        .filter_map(|fish| fish.script())
        .filter(|bot| bot.chip().is_some())
        .count();
    assert_eq!(chips, ADDER_BITS, "four fish, thirty-six gates");
}

fn chain_the_adders(tui: &mut Tui) {
    for index in 0..ADDER_BITS {
        let cin = format!("c{index}");
        let cout = format!("c{}", index + 1);
        let sum = format!("s{index}");
        let shot =
            (index == 0).then_some("5 · /program \"Fa0\" — a ▸ one, cin ▸ c0, cout ▸ c1, sum ▸ s0");
        set_fields(
            tui,
            &format!("Fa{index}"),
            &[
                (CHIP_A_FIELD, "one"),
                (CHIP_CIN_FIELD, &cin),
                (CHIP_COUT_FIELD, &cout),
                (CHIP_SUM_FIELD, &sum),
            ],
            shot,
        );
    }
    spawn(tui, "one");
    spawn(tui, "sc");
    install(tui, "inverter coil", RAIL_ROW);
    wire(tui, "One", "", "one", NO_SHOT);
    wire(tui, "Sc", "", "c0", NO_SHOT);
    for index in 1..=ADDER_BITS {
        spawn(tui, &format!("c{index}"));
    }
    for index in 0..ADDER_BITS {
        spawn(tui, &format!("s{index}"));
    }
    for index in 1..=ADDER_BITS {
        wire(tui, &carry_probe(index), &format!("c{index}"), "", NO_SHOT);
    }
    for index in 0..ADDER_BITS {
        wire(tui, &sum_probe(index), &format!("s{index}"), "", NO_SHOT);
    }
}

#[test]
fn the_phase_four_demo_written_in_the_plan_still_runs_keystroke_for_keystroke() {
    let mut tui = filmed("phase-4-demo");
    tui.run("/clock 1");
    tui.run(&format!("/add inverter coil {COILS_FOR_THE_ADDERS}"));
    tui.run(&format!("/add blank wafer {WAFERS_FOR_FOUR_ETCHES}"));
    tui.run(&format!("/add fabricator {ADDER_BITS}"));
    tui.run("/add blank circuit blueprint 1");

    design_the_full_adder(&mut tui);
    capture_the_full_adder(&mut tui);
    open_a_second_tank(&mut tui);
    etch_four_adders(&mut tui);
    chain_the_adders(&mut tui);

    tui.run("/fps 1");
    tui.tick_n(ADDER_SETTLE);
    for index in 0..ADDER_BITS {
        assert!(
            probe_lit(&tui, &sum_probe(index)),
            "15 + 0 lights every sum bit"
        );
        assert!(
            !probe_lit(&tui, &carry_probe(index + 1)),
            "and carries nothing"
        );
    }
    tui.run("/circuit");
    tui.snap("6 · /circuit — fourteen fish, 15 + 0 on the probes");
    tui.key(KeyCode::Tab);
    assert!(
        !tui.screen().contains("too big to draw"),
        "fourteen fish draw as a schematic"
    );
    tui.snap("6 · TAB — the chain as a staircase, before the flip");

    tui.key(KeyCode::Down);
    tui.key(KeyCode::Enter);
    tui.snap("7 · ↓ to [Sc], ENTER — its panel over the schematic");
    tui.type_text("one");
    tui.key(KeyCode::Enter);
    assert!(
        tui.screen().contains("[C4]"),
        "ENTER saves and hands back the schematic:\n{}",
        tui.screen().text()
    );

    let expected: Vec<Option<usize>> = (0..ADDER_BITS)
        .map(|index| Some(FIRST_PROBE_STAGE + index))
        .collect();
    let (carried, cleared) = watch_the_carry(&mut tui, true, Some("7 · Stage"));
    assert_eq!(carried, expected, "the carry walks one adder per stage");
    assert_eq!(cleared, expected, "and each sum bit goes dark as it passes");

    tui.key(KeyCode::Enter);
    for _ in 0.."one".len() {
        tui.key(KeyCode::Backspace);
    }
    tui.key(KeyCode::Enter);
    let (dropped, relit) = watch_the_carry(&mut tui, false, NO_SHOT);
    tui.snap("8 · The switch off again — 15 + 0 + 0 = 15, walked back");
    assert_eq!(
        dropped, expected,
        "the carry falls away one adder per stage"
    );
    assert_eq!(relit, expected, "and each sum bit relights as it goes");

    tui.key(KeyCode::Tab);
    tui.snap("Finish · TAB back to the table — the probes read 15 + 0 + 0 = 15");
    tui.key(KeyCode::Esc);
    tui.run("/nets");
    tui.snap("Finish · /nets — each chip under its channels");
    tui.run("/nets");
    tui.run("/fps 30");
    tui.run("/switch Fishtank");
    assert_eq!(tui.app.current_tank, 0);

    assert_no_broken_borders(&tui);
}

const PHASE_FIVE_BENCH: [(&str, u32); 9] = [
    ("inverter coil", 4),
    ("cochlea", 1),
    ("glyph panel", 1),
    ("cathode array", 1),
    ("reflex arc", 1),
    ("command module", 5),
    ("relay mast", 1),
    ("angler rig", 1),
    ("bait", 2),
];
const TYPED_LINE: &str = "6 + 4";
const ECHO_TICKS: usize = 20;
const LCD_FIELDS: [(usize, &str); 7] = [
    (0, "clk"),
    (1, "wr"),
    (4, "kbd"),
    (5, "wr"),
    (7, "8"),
    (12, "kbd"),
    (13, "wr"),
];
const EAR_FIELDS: [(usize, &str); 5] = [
    (0, "clk, fin"),
    (1, "clk"),
    (2, "kbd"),
    (3, "clk"),
    (5, "fin"),
];
const PAD_MOVERS: [(&str, &str, KeyCode, i32, i32); 4] = [
    ("North", "n", KeyCode::Up, 0, -1),
    ("South", "s", KeyCode::Down, 0, 1),
    ("West", "w", KeyCode::Left, -3, 0),
    ("East", "e", KeyCode::Right, 3, 0),
];
const PAD_FIELDS: [(usize, &str); 7] = [
    (2, "glup"),
    (11, "n"),
    (12, "s"),
    (13, "w"),
    (14, "e"),
    (15, "glup"),
    (20, "/say \"glup glup\""),
];
const PAD_ROW: usize = 2;
const MOVER_FIRE_FIELD: usize = 2;
const MOVER_LINE_FIELD: usize = 4;
const HAND_TICKS: usize = 70;
const MAST_TANK: &str = "Annex";
const TX_ROW: usize = 7;
const TX_FIELDS: [(usize, &str); 4] = [(1, "x"), (2, "Annex"), (3, "y"), (4, "x")];
const RELAY_SETTLE: usize = 4;
const ROD_ROW: usize = 8;
const ROD_FIELDS: [(usize, &str); 3] = [(1, "go"), (3, "go"), (4, "caught")];
const BELL_FIELDS: [(usize, &str); 2] = [(0, "caught, held"), (1, "held")];
const BITE_WAIT_TICKS: usize = 3000;
const RAD_TANK: &str = "Pripyat";
const RAD_WAIT_TICKS: usize = 600;

fn fish_state<'a>(tui: &'a Tui, name: &str) -> &'a fishtank::fishes::botfish::BotfishState {
    tui.app
        .tanks
        .iter()
        .flat_map(|tank| &tank.fish)
        .find(|fish| fish.name == name)
        .and_then(|fish| fish.script())
        .expect("a programmable fish of that name")
}

fn fish_position(tui: &Tui, name: &str) -> (f32, f32) {
    let fish = tui.app.tanks[tui.app.current_tank]
        .fish
        .iter()
        .find(|fish| fish.name == name)
        .expect("the fish swims here");
    (fish.position.x, fish.position.y)
}

fn park(tui: &mut Tui, name: &str, sideways: i32) {
    tui.run(&format!("/freeze \"{name}\""));
    tui.run(&format!("/nudge \"{name}\" {} 20", sideways.signum() * 200));
    tui.run(&format!("/nudge \"{name}\" {} 0", -sideways));
}

fn echo_the_typed_line(tui: &mut Tui) {
    spawn(tui, "ear");
    spawn(tui, "lcd");
    install_on_rows(tui, "inverter coil", &[0], NO_SHOT);
    install_on_rows(tui, "cochlea", &[0], NO_SHOT);
    install_on_rows(tui, "glyph panel", &[1], NO_SHOT);
    install_on_rows(
        tui,
        "cathode array",
        &[1],
        Some("1 · /consume cathode array — ↓ to Lcd"),
    );
    park(tui, "Lcd", 15);
    set_fields(
        tui,
        "Lcd",
        &LCD_FIELDS,
        Some("1 · /program \"Lcd\" — the last field, write_byte ◂ wr"),
    );
    tui.run(TYPED_LINE);
    set_fields(
        tui,
        "Ear",
        &EAR_FIELDS,
        Some("1 · /program \"Ear\" — wired last, on a line already waiting"),
    );
    tui.tick_n(ECHO_TICKS);
    tui.screen().expect_find(&format!("/ {TYPED_LINE}"));
    tui.snap("1 · The line reads back off Lcd's panel, its last byte as dots on its body");
}

fn drive_the_pad(tui: &mut Tui) {
    spawn(tui, "pad");
    for (mover, _, _, _, _) in PAD_MOVERS {
        spawn(tui, mover);
    }
    install_on_rows(tui, "reflex arc", &[PAD_ROW], NO_SHOT);
    let hands: Vec<usize> = (PAD_ROW..=PAD_ROW + PAD_MOVERS.len()).collect();
    install_on_rows(
        tui,
        "command module",
        &hands,
        Some("2 · /consume command module — Pad and the four movers"),
    );
    park(tui, "Pad", -15);
    set_fields(
        tui,
        "Pad",
        &PAD_FIELDS,
        Some("2 · /program \"Pad\" — its script, glup glup"),
    );
    for (mover, net, _, dx, dy) in PAD_MOVERS {
        let line = format!("/nudge \"Pad\" {dx} {dy}");
        let shot = (mover == "North").then_some("2 · /program \"North\" — fire ◂ n, one nudge up");
        set_fields(
            tui,
            mover,
            &[(MOVER_FIRE_FIELD, net), (MOVER_LINE_FIELD, &line)],
            shot,
        );
    }
    tui.run("/console \"Pad\"");
    tui.snap("2 · /console \"Pad\" — the key bar replaces the editor");
    for (mover, _, key, dx, dy) in PAD_MOVERS {
        let before = fish_position(tui, "Pad");
        tui.key(key);
        tui.tick_n(1);
        tui.release(key);
        tui.tick_n(HAND_TICKS);
        let after = fish_position(tui, "Pad");
        assert_eq!(
            (after.0 - before.0, after.1 - before.1),
            (dx as f32, dy as f32),
            "{mover} nudged Pad once"
        );
    }
    tui.snap("2 · After ↑ ↓ ← →, Pad is back where it started");
    tui.key(KeyCode::Char(' '));
    tui.release(KeyCode::Char(' '));
    tui.tick_n(HAND_TICKS / 2);
    tui.screen().expect_find("glup glup");
    tui.snap("2 · SPACE — Pad says glup glup");
    tui.key(KeyCode::Esc);
    tui.screen().expect_absent("console Pad");
}

fn bridge_to_the_annex(tui: &mut Tui) {
    tui.run("/buy fishtank");
    tui.type_text("annex");
    tui.key(KeyCode::Enter);
    for _ in 0..CLOSE_THE_SHOP {
        tui.key(KeyCode::Esc);
    }
    assert_eq!(tui.app.tanks[tui.app.current_tank].name, MAST_TANK);
    spawn(tui, "rx");
    set_fields(tui, "Rx", &[(0, "y")], NO_SHOT);
    tui.run("/switch Fishtank");
    spawn(tui, "tx");
    install_on_rows(tui, "inverter coil", &[TX_ROW], NO_SHOT);
    install_on_rows(
        tui,
        "relay mast",
        &[TX_ROW],
        Some("3 · /consume relay mast — ↓ to Tx"),
    );
    set_fields(
        tui,
        "Tx",
        &TX_FIELDS,
        Some("3 · /program \"Tx\" — aimed at y in the Annex"),
    );
    tui.run("/switch Annex");
    tui.tick_n(RELAY_SETTLE);
    tui.run("/circuit");
    tui.screen().expect_find("●  Rx");
    tui.snap("3 · /circuit in the Annex — Rx lit by a rail in the Fishtank");
    tui.key(KeyCode::Esc);
}

fn fish_with_the_rig(tui: &mut Tui) {
    tui.run("/switch Fishtank");
    spawn(tui, "rod");
    spawn(tui, "bell");
    install_on_rows(tui, "inverter coil", &[ROD_ROW], NO_SHOT);
    install_on_rows(
        tui,
        "angler rig",
        &[ROD_ROW],
        Some("4 · /consume angler rig — ↓ to Rod"),
    );
    set_fields(tui, "Bell", &BELL_FIELDS, NO_SHOT);
    let bait = |tui: &Tui| {
        tui.app
            .inventory
            .get(&StockItem::BAIT)
            .copied()
            .unwrap_or(0)
    };
    let bait_before = bait(tui);
    set_fields(
        tui,
        "Rod",
        &ROD_FIELDS,
        Some("4 · /program \"Rod\" — a rail that casts once"),
    );
    let mut lowest = bait(tui);
    let shoal = tui.app.tanks[tui.app.current_tank].fish.len();
    let landed = (0..BITE_WAIT_TICKS).position(|_| {
        tui.tick_n(1);
        lowest = lowest.min(bait(tui));
        probe_lit(tui, "Bell")
    });
    let landed = landed.expect("a rig always lands a fish");
    let waited = (landed + 1) as f32 / tui.app.settings.fps;
    assert!(
        (RIG_WAIT_MIN_SECS..=RIG_WAIT_MAX_SECS + 1.0).contains(&waited),
        "the line stayed down {waited} game seconds"
    );
    assert_eq!(lowest + 1, bait_before, "one cast, one bait");
    assert_eq!(
        tui.app.tanks[tui.app.current_tank].fish.len(),
        shoal + 1,
        "and what came up was a fish"
    );
    tui.run("/circuit");
    tui.screen().expect_find("●  Bell");
    tui.snap("4 · /circuit — Bell latched the one-stage caught");
    tui.key(KeyCode::Esc);
}

fn irradiate_the_rig(tui: &mut Tui) {
    tui.run("/add demon core 1");
    tui.run("/consume demon core");
    tui.type_text("pripyat");
    tui.key(KeyCode::Enter);
    assert_eq!(tui.app.tanks[tui.app.current_tank].name, RAD_TANK);
    tui.run(&format!("/move \"Rod\" \"{RAD_TANK}\""));
    let fitted = |tui: &Tui| {
        let rod = fish_state(tui, "Rod");
        let wires: Vec<String> = rod
            .pins()
            .iter()
            .map(|(_, pin, channel)| format!("{pin}{channel}"))
            .collect();
        (wires, rod.parts().len())
    };
    let before = fitted(tui);
    tui.run("/circuit");
    tui.snap("5 · /circuit in Pripyat — Rod as it arrived");
    tui.key(KeyCode::Esc);
    tui.run("/fps 1");
    let struck = (0..RAD_WAIT_TICKS).any(|_| {
        tui.tick_n(1);
        fitted(tui) != before
    });
    assert!(
        struck,
        "a Radtank strikes a wired fish about every thirty seconds"
    );
    tui.run("/circuit");
    tui.snap("5 · /circuit in Pripyat — Rod's wiring or parts after the strike");
    tui.key(KeyCode::Esc);
}

#[test]
fn the_phase_five_demo_written_in_the_plan_still_runs_keystroke_for_keystroke() {
    let mut tui = filmed("phase-5-demo");
    for (part, count) in PHASE_FIVE_BENCH {
        tui.run(&format!("/add {part} {count}"));
    }

    echo_the_typed_line(&mut tui);
    drive_the_pad(&mut tui);
    bridge_to_the_annex(&mut tui);
    fish_with_the_rig(&mut tui);
    irradiate_the_rig(&mut tui);

    tui.run("/nets");
    tui.snap("Finish · /nets in Pripyat");
    tui.run("/nets");
    tui.run("/fps 30");
    tui.run("/switch Fishtank");
    assert_eq!(tui.app.current_tank, 0);
    assert_no_broken_borders(&tui);
}

const DEMO_SECONDS_AT_30: usize = 30;
const FEED_ROUNDS_SLACK: usize = 2;
const FEED_WAITS_PER_PORTION: usize = 3;
const FEED_WAIT_SECS: usize = 12;
const COINS: [&str; 5] = ["Coin1", "Coin2", "Coin3", "Coin4", "Coin5"];
const COIN_PAYOUT_SECS: usize = 36;
const LEFTOVER_WAIT_SECS: usize = 30;
const COIN_RATE_DRIFT: f64 = 0.45;
const REEL_TICKS: usize = 240;
const RIG_WATCH_TICKS: usize = 70;
const CASH_TOPUP: &str = "/add cash 5000";

fn tank_fish<'a>(tui: &'a Tui, name: &str) -> &'a fishtank::fishes::fish::Fish {
    tui.app.tanks[tui.app.current_tank]
        .fish
        .iter()
        .find(|fish| fish.name == name)
        .unwrap_or_else(|| panic!("{name} is in the tank"))
}

fn pellets_to_cap(fish: &fishtank::fishes::fish::Fish) -> usize {
    let cap = fish.species.config().weight_cap[fish.size_category as usize];
    cap.saturating_sub(fish.weight_g)
        .div_ceil(FOOD_WEIGHT_GAIN_G) as usize
}

fn seconds_at(tui: &mut Tui, fps: usize, seconds: usize) {
    tui.tick_n(fps * seconds);
}

fn ledger_line(tui: &Tui, direction: Direction, flow: Flow) -> Money {
    tui.app.ledger.since_launch().line(direction, flow)
}

fn hurry_the_coins(tui: &mut Tui, fps: usize) -> Money {
    let before = ledger_line(tui, Direction::In, Flow::Cashfish);
    tui.run("/cheat");
    tui.type_text("hardcore to the mega");
    tui.key(KeyCode::Enter);
    seconds_at(tui, fps, COIN_PAYOUT_SECS);
    ledger_line(tui, Direction::In, Flow::Cashfish) - before
}

fn land_one_cast(tui: &mut Tui) {
    tui.run("/fish --no-fight");
    let card = |tui: &mut Tui| {
        let screen = tui.screen();
        screen.contains("ENTER capture") || screen.contains("ESC/q close")
    };
    for _ in 0..REEL_TICKS {
        tui.key(KeyCode::Down);
        tui.tick_n(1);
        if tui.app.fishing_state().is_none() {
            break;
        }
    }
    assert!(card(tui), "holding the reel with --no-fight lands the cast");
    tui.release(KeyCode::Down);
    tui.type_text("Bo");
    tui.key(KeyCode::Enter);
    assert!(!card(tui), "the card closed");
}

#[test]
fn the_economy_demo_runs_keystroke_for_keystroke() {
    let mut tui = Tui::new();
    tui.film(Path::new(REEL_DIR), "economy-demo");
    tui.run("/reset");
    tui.run(CASH_TOPUP);
    tui.run("/sell fish \"Adam\"");
    tui.run("/sell fish \"Lilith\"");
    tui.run("/sell fish \"Eva\"");
    tui.snap("0 · a fresh debug game: the starters sold, $5,000 to play with");

    tui.run("/give merluza \"Kip\"");
    let caught = tank_fish(&tui, "Kip").sell_value();
    tui.run("/shop");
    tui.key(KeyCode::Down);
    tui.key(KeyCode::Enter);
    tui.screen().expect_find("Kip (Merluza)");
    tui.snap("1 · /shop, Sell: Kip's price before a single pellet");
    tui.key(KeyCode::Esc);
    tui.key(KeyCode::Esc);
    let pellets_to_cap = pellets_to_cap(tank_fish(&tui, "Kip"));
    let feed_rounds_max = pellets_to_cap.div_ceil(FEED_PORTION) + FEED_ROUNDS_SLACK;
    tui.run(&format!("/buy food {pellets_to_cap}"));
    let bag = tui.app.food_supply;
    tui.run("/feed");
    assert_eq!(
        tui.app.food_supply,
        bag - FEED_PORTION as u32,
        "/feed alone drops one portion"
    );
    tui.snap("1 · /feed: thirty pellets shower down");
    let mut rounds = 1;
    let mut waits = 1;
    seconds_at(&mut tui, DEMO_SECONDS_AT_30, FEED_WAIT_SECS);
    while tank_fish(&tui, "Kip").seeks_food() && waits < feed_rounds_max * FEED_WAITS_PER_PORTION {
        if tui.app.tanks[tui.app.current_tank].food.is_empty() {
            tui.run("/feed");
            rounds += 1;
        }
        seconds_at(&mut tui, DEMO_SECONDS_AT_30, FEED_WAIT_SECS);
        waits += 1;
    }
    let kip = tank_fish(&tui, "Kip");
    assert!(
        !kip.seeks_food(),
        "Kip reached its cap in {rounds} portions and {waits} waits"
    );
    assert!(rounds <= feed_rounds_max, "Kip ate {rounds} portions");
    let fattened = kip.sell_value();
    let config = kip.species.config();
    let size = kip.size_category as usize;
    let pellets = Money::from((config.weight_cap[size] - config.weight_base[size]) / 50);
    assert!(
        fattened - caught > pellets,
        "Kip's worth rose ${} on {pellets} pellets",
        fattened - caught
    );
    tui.run("/shop");
    tui.key(KeyCode::Down);
    tui.key(KeyCode::Enter);
    tui.snap("1 · /shop, Sell: Kip at its cap is worth more than the food it ate");
    tui.key(KeyCode::Esc);
    tui.key(KeyCode::Esc);
    tui.run("/sell fish \"Kip\"");

    for coin in COINS {
        tui.run(&format!("/give cashfish \"{coin}\""));
    }
    seconds_at(&mut tui, DEMO_SECONDS_AT_30, LEFTOVER_WAIT_SECS);
    assert!(
        tui.app.tanks[tui.app.current_tank].food.is_empty(),
        "the coins ate every leftover pellet"
    );
    tui.run("/fps 30");
    let at_30 = hurry_the_coins(&mut tui, 30);
    tui.snap("2 · /fps 30: five coins zoom, $ bubbles rise and pay");
    tui.run("/fps 120");
    let at_120 = hurry_the_coins(&mut tui, 120);
    tui.snap("2 · /fps 120: the same zoomies pay about the same");
    tui.run("/fps 30");
    assert!(at_30 > 0 && at_120 > 0, "{at_30} and {at_120}");
    let drift = (at_120 as f64 - at_30 as f64).abs() / at_30 as f64;
    assert!(
        drift < COIN_RATE_DRIFT,
        "${at_30} at 30 fps, ${at_120} at 120 fps"
    );

    tui.run("/add bait 5");
    tui.run("/consume bait");
    tui.run("/consume bait");
    tui.screen().expect_find("baiting II: 5 casts");
    tui.snap("3 · two Baits: two stacks of five casts");
    land_one_cast(&mut tui);
    tui.screen().expect_find("baiting II: 4 casts");
    tui.snap("3 · one cast landed: both stacks spent one");
    land_one_cast(&mut tui);
    tui.screen().expect_find("baiting II: 3 casts");

    tui.run("/give botfish \"Rod\"");
    tui.run("/add inverter coil 1");
    tui.run("/add angler rig 1");
    tui.run("/consume inverter coil");
    tui.key(KeyCode::Enter);
    tui.run("/consume angler rig");
    tui.key(KeyCode::Enter);
    tui.run("/program \"Rod\"");
    tui.type_text("go");
    tui.key(KeyCode::Down);
    tui.type_text("go");
    tui.key(KeyCode::Down);
    tui.key(KeyCode::Down);
    tui.type_text("go");
    tui.snap("4 · Rod listens to go, drives go, and casts on go");
    tui.key(KeyCode::Enter);
    tui.run("/fps 1");
    let shoal = tui.app.tanks[tui.app.current_tank].fish.len();
    let landed = (0..RIG_WATCH_TICKS).position(|_| {
        tui.tick_n(1);
        tui.app.tanks[tui.app.current_tank].fish.len() > shoal
    });
    let landed = landed.expect("the rig landed a fish") + 1;
    assert!(
        (RIG_WAIT_MIN_SECS as usize..=RIG_WAIT_MAX_SECS as usize + 2).contains(&landed),
        "the rig's fish came up after {landed} s"
    );
    tui.run("/names");
    tui.snap("4 · a fish named Un-something joined after half a minute or more");
    tui.run("/names");
    tui.run("/fps 30");

    tui.run("/ledger");
    for line in ["Fish sales", "Cashfish", "Food", "Godsend"] {
        tui.screen().expect_find(line);
    }
    tui.snap("5 · /ledger: every line the demo moved");
    for _ in 0..16 {
        tui.key(KeyCode::Down);
    }
    tui.screen().expect_find("Per minute");
    tui.snap("5 · the net and the net per minute");
    tui.key(KeyCode::Esc);
    tui.screen().expect_absent("Ledger#show");
    assert!(
        ledger_line(&tui, Direction::Out, Flow::Food)
            >= Money::from(FOOD_BUY_PRICE) * pellets_to_cap as Money
    );
    assert_no_broken_borders(&tui);
}

const BITE_TICKS: usize = 30 * 90;
const FIGHT_TICKS: usize = 60;
const HEAVEN_NAME: &str = "Heaventank";

fn strike(tui: &mut Tui, flag: &str) {
    tui.run(&format!("/fish --no-escape {flag}"));
    for _ in 0..BITE_TICKS {
        if tui.app.fishing_state().is_some_and(|s| s.is_biting()) {
            break;
        }
        tui.tick_n(1);
    }
    tui.key(KeyCode::Down);
    tui.tick_n(FIGHT_TICKS);
}

#[test]
fn the_normal_legendary_and_pearl_demo_runs_keystroke_for_keystroke() {
    let mut tui = filmed("demo-fish-tempers-and-the-pearl");

    strike(&mut tui, "--normal");
    let state = tui.app.fishing_state().expect("the fish is on the line");
    assert_eq!(state.temper(), Temper::Normal);
    tui.snap("a Normal fish two seconds in: green water in the middle, red at the sides");
    tui.key(KeyCode::Esc);

    strike(&mut tui, "--legendary");
    let state = tui.app.fishing_state().expect("the fish is on the line");
    assert_eq!(state.temper(), Temper::Legendary);
    tui.snap("a Legendary fish two seconds in");
    tui.key(KeyCode::Esc);

    tui.run("/spawn salmon \"Ann\"");
    tui.run("/sell fish \"Ann\"");
    tui.run("/fishtanks");
    tui.screen().expect_absent(HEAVEN_NAME);
    tui.snap("a sale opens no heaven");
    tui.key(KeyCode::Esc);

    tui.run("/give golden pearl");
    tui.run("/consume golden pearl");
    tui.screen().expect_find("Name your");
    tui.type_text(HEAVEN_NAME);
    tui.key(KeyCode::Enter);
    tui.run("/names");
    tui.tick_n(FIGHT_TICKS);
    let wall = &tui.app.tanks[tui.app.current_tank];
    assert_eq!(wall.name, HEAVEN_NAME, "a new tank switches you into it");
    assert!(wall.souls().iter().any(|soul| soul.name == "Ann"));
    tui.snap("the pearl grew Heaven and Ann was waiting for it");
    tui.run("/give golden pearl");
    assert_eq!(tui.app.inventory.get(&StockItem::GOLDEN_PEARL), None);
}
