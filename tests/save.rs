use std::fs;
use std::path::{Path, PathBuf};

use crossterm::event::KeyCode;
use fishtank::app::{
    App, FIRST_FISH, FIRST_TANK_NAME, Launch, SAVE_VERSION, STARTING_CASH, STARTING_FOOD, SaveFile,
};
use fishtank::economy::Money;
use fishtank::entities::plant::SWAY_AMOUNT;
use fishtank::fishes::fish::Fish;
use fishtank::ledger::Flow;
use fishtank::loot::{ConsumableKind, MilkVariant, StockItem};
use fishtank::tank::{FEED_PORTION, TankKind};
use fishtank::testing::{DEFAULT_COLS, DEFAULT_ROWS, Tui};
use fishtank::vault::{self, Loaded, Vault, VaultError};

const SCRATCH: &str = env!("CARGO_TARGET_TMPDIR");
const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/saves/fishtank-v1.ron");
const HELL_NAME: &str = "Pit";
const MATRIX_NAME: &str = "Zion";
const LATCH: &str = "Latch";
const HOST: &str = "Host";
const EARNED: Money = 1_234;
const WIDE_COLS: u16 = 160;
const FED: u32 = FEED_PORTION as u32;
const SWAY_REACH: usize = SWAY_AMOUNT as usize;
const SEEN_COLS: usize = DEFAULT_COLS as usize - SWAY_REACH;

fn scratch(test: &str) -> PathBuf {
    let dir = Path::new(SCRATCH).join("save").join(test);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

fn quoted(path: &Path) -> String {
    format!("\"{}\"", path.display())
}

fn reopened(app: &App) -> App {
    let text = app.snapshot().to_ron().expect("a game writes itself down");
    let save = SaveFile::from_ron(&text).expect("and reads itself back");
    App::resume(save, DEFAULT_COLS, DEFAULT_ROWS)
}

fn fish<'a>(app: &'a App, name: &str) -> &'a Fish {
    app.tanks
        .iter()
        .flat_map(|tank| tank.fish.iter())
        .find(|fish| fish.name == name)
        .unwrap_or_else(|| panic!("{name} swims somewhere"))
}

fn tank_index(app: &App, name: &str) -> usize {
    app.tanks
        .iter()
        .position(|tank| tank.name == name)
        .unwrap_or_else(|| panic!("{name} is a tank"))
}

fn held(app: &App, kind: ConsumableKind) -> u32 {
    app.inventory
        .get(&StockItem::Consumable(kind))
        .copied()
        .unwrap_or(0)
}

fn wire_a_latch(tui: &mut Tui) {
    tui.run("/add inverter coil 2");
    tui.run("/spawn botfish \"q\"");
    tui.run("/spawn botfish \"qn\"");
    tui.run("/consume inverter coil");
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Down);
    tui.key(KeyCode::Enter);
    tui.run("/program \"Q\"");
    tui.type_text("r, qn");
    tui.key(KeyCode::Down);
    tui.type_text("q");
    tui.key(KeyCode::Enter);
    tui.run("/program \"Qn\"");
    tui.type_text("s, q");
    tui.key(KeyCode::Down);
    tui.type_text("qn");
    tui.key(KeyCode::Enter);
}

fn grow(tui: &mut Tui, item: &str, name: &str) {
    tui.run(&format!("/give {item}"));
    tui.run(&format!("/consume {item}"));
    tui.type_text(name);
    tui.key(KeyCode::Enter);
}

fn rich_game() -> Tui {
    let mut tui = Tui::new();
    tui.stake();
    wire_a_latch(&mut tui);
    grow(&mut tui, "computer", MATRIX_NAME);
    tui.run(&format!("/switch \"{FIRST_TANK_NAME}\""));
    tui.run("/give blank circuit blueprint");
    tui.run("/consume blank circuit blueprint");
    tui.type_text(LATCH);
    tui.key(KeyCode::Enter);
    tui.run("/add fabricator 1");
    tui.run(&format!("/spawn botfish \"{HOST}\""));
    tui.run(&format!("/etch \"{LATCH}\" \"{HOST}\""));
    tui.run("/freeze \"Q\"");
    tui.run("/nudge \"Q\" 3 1");
    tui.run("/flip \"Q\"");
    tui.app.tanks[0].channels.set_level("s", true);
    tui.tick_n(3);
    tui.app.tanks[0].channels.set_level("s", false);
    tui.tick_n(3);
    tui.run("/spawn mutantfish \"Mo\"");
    tui.run("/mutate \"Mo\" eyeincrease");
    tui.run("/mutate \"Mo\" telophase");
    tui.run("/kill \"Adam\"");
    grow(&mut tui, "golden pearl", TankKind::Heaven.display_name());
    grow(&mut tui, "necronomicon", HELL_NAME);
    tui.run("/spawn salmon \"Sinner\"");
    tui.run("/kill \"Sinner\"");
    tui.run("/add strawberry milk 2");
    tui.run("/add coffee 1");
    tui.run("/consume coffee");
    tui
}

#[test]
fn a_new_game_is_adam_lilith_and_eva_with_fifty_in_cash_and_sixty_food() {
    let app = App::new();

    assert_eq!(app.tanks.len(), 1);
    assert_eq!(app.tanks[0].name, FIRST_TANK_NAME);
    assert_eq!(app.tanks[0].kind, TankKind::Base);
    let school: Vec<_> = app.tanks[0]
        .fish
        .iter()
        .map(|fish| (fish.species, fish.name.as_str()))
        .collect();
    assert_eq!(school, FIRST_FISH.to_vec());
    assert_eq!(app.purse.balance(), STARTING_CASH);
    assert_eq!(app.food_supply, STARTING_FOOD);
    assert!(app.inventory.is_empty(), "no items");
    assert!(app.graveyard.is_empty() && app.blueprints.is_empty());
    assert!(!app.debug_mode);
}

#[test]
fn a_reopened_game_writes_itself_down_exactly_as_it_was() {
    let tui = rich_game();
    let first = tui.app.snapshot().to_ron().expect("a save");

    let second = reopened(&tui.app).snapshot().to_ron().expect("a save");

    assert_eq!(first, second);
}

#[test]
fn everything_the_player_owns_comes_back() {
    let tui = rich_game();
    let before = &tui.app;
    let after = reopened(before);

    assert_eq!(after.purse, before.purse);
    assert_eq!(after.food_supply, before.food_supply);
    assert!(after.inventory == before.inventory);
    assert_eq!(
        held(&after, ConsumableKind::Milk(MilkVariant::Strawberry)),
        2
    );
    assert_eq!(
        after.active_consumables.len(),
        before.active_consumables.len()
    );
    let names = |app: &App| -> Vec<(String, TankKind)> {
        app.tanks.iter().map(|t| (t.name.clone(), t.kind)).collect()
    };
    assert_eq!(names(&after), names(before));
    assert_eq!(
        after.tanks[after.current_tank].name, HELL_NAME,
        "you wake up where you fell asleep"
    );
    let dead =
        |app: &App| -> Vec<String> { app.graveyard.iter().map(|f| f.name.clone()).collect() };
    assert_eq!(dead(&after), dead(before));
    let heaven = tank_index(&after, TankKind::Heaven.display_name());
    assert!(
        after.tanks[heaven]
            .souls()
            .iter()
            .any(|soul| soul.name == "Adam")
    );
    let hell = tank_index(&after, HELL_NAME);
    assert!(
        after.tanks[hell]
            .souls()
            .iter()
            .any(|soul| soul.name == "Sinner")
    );
    assert!(
        after.tanks[hell]
            .souls()
            .iter()
            .all(|soul| soul.devil_marked)
    );
    assert_eq!(after.blueprints.len(), 1);
    assert_eq!(after.blueprints[0].name, LATCH);
}

#[test]
fn a_fish_keeps_its_looks_and_its_history() {
    let tui = rich_game();
    let after = reopened(&tui.app);
    for name in ["Lilith", "Eva", "Mo"] {
        let (was, is) = (fish(&tui.app, name), fish(&after, name));
        assert_eq!(is.species, was.species, "{name}");
        assert_eq!(is.color, was.color, "{name}");
        assert_eq!(is.pattern_seed, was.pattern_seed, "{name}");
        assert_eq!(is.weight_g, was.weight_g, "{name}");
        assert_eq!(is.display_width, was.display_width, "{name}");
        let mut still = is.clone();
        still.sway = was.sway.clone();
        still.facing = was.facing;
        assert_eq!(
            still.line_sprite().rows,
            was.line_sprite().rows,
            "{name} looks the same"
        );
    }
    let history = |app: &App| {
        fish(app, "Mo")
            .mutations
            .as_ref()
            .map(|record| record.history.clone())
    };
    assert_eq!(history(&after), history(&tui.app));
}

#[test]
fn a_botfish_keeps_its_place_its_wiring_and_its_memory() {
    let tui = rich_game();
    let after = reopened(&tui.app);

    let (was, is) = (fish(&tui.app, "Q"), fish(&after, "Q"));
    assert_eq!(
        (is.position.x, is.position.y),
        (was.position.x, was.position.y)
    );
    assert!(is.facing == was.facing && is.frozen);
    let wiring = |fish: &Fish| {
        let bot = fish.script().expect("a botfish");
        (
            bot.listens().map(str::to_string).collect::<Vec<_>>(),
            bot.drives().map(str::to_string),
        )
    };
    assert_eq!(wiring(is), wiring(was));
    assert!(
        after.tanks[0].channels.level("q"),
        "the latch still holds what it was set to"
    );
    assert!(
        fish(&after, HOST)
            .script()
            .and_then(|bot| bot.chip())
            .is_some(),
        "the etched chip survives"
    );
}

#[test]
fn the_status_bar_reads_the_same_after_a_restart() {
    let mut tui = rich_game();
    let bar = |tui: &mut Tui| tui.screen().rows().last().cloned().unwrap_or_default();
    let before = bar(&mut tui);

    let mut after = Tui::resumed(
        SaveFile::from_ron(&tui.app.snapshot().to_ron().unwrap()).unwrap(),
        DEFAULT_COLS,
        DEFAULT_ROWS,
    );

    assert_eq!(bar(&mut after), before);
}

#[test]
fn food_left_in_the_water_goes_back_in_the_bag() {
    let mut tui = Tui::new();
    tui.run("/feed");
    assert_eq!(tui.app.food_supply, STARTING_FOOD - FED);

    let after = reopened(&tui.app);

    assert_eq!(after.food_supply, STARTING_FOOD);
    assert!(after.tanks[0].food.is_empty());
}

#[test]
fn a_tank_is_drawn_from_its_seed_the_same_every_time_at_any_width() {
    let mut tui = Tui::new();
    tui.run("/give candytank");
    tui.run("/switch \"Uncandytank\"");
    tui.clear_tank();
    let save = tui.app.snapshot().to_ron().unwrap();
    let scenery = |cols: u16| {
        let mut again = Tui::resumed(SaveFile::from_ron(&save).unwrap(), cols, DEFAULT_ROWS);
        let rows = again.screen().rows().to_vec();
        rows.iter()
            .take(DEFAULT_ROWS as usize / 2)
            .map(|row| row.chars().take(SEEN_COLS).collect::<String>())
            .collect::<Vec<_>>()
    };

    assert_eq!(scenery(DEFAULT_COLS), scenery(DEFAULT_COLS));
    assert_eq!(
        scenery(DEFAULT_COLS),
        scenery(WIDE_COLS),
        "a wider window only adds scenery"
    );
}

#[test]
fn a_fresh_vault_starts_the_new_game() {
    let dir = scratch("fresh");
    let vault = Vault::open_in(dir, Launch::Player).expect("a vault");
    assert!(matches!(vault.load(), Ok(Loaded::Fresh)));

    let app = App::open(Launch::Player, vault, DEFAULT_COLS, DEFAULT_ROWS).expect("a game");

    assert_eq!(app.purse.balance(), STARTING_CASH);
}

#[test]
fn the_next_launch_resumes_the_game_the_last_one_left() {
    let dir = scratch("resume");
    {
        let vault = Vault::open_in(dir.clone(), Launch::Player).expect("a vault");
        let mut app = App::open(Launch::Player, vault, DEFAULT_COLS, DEFAULT_ROWS).expect("a game");
        app.earn(EARNED, Flow::Godsend);
        app.persist();
    }

    let vault = Vault::open_in(dir, Launch::Player).expect("the lock was let go");
    let app = App::open(Launch::Player, vault, DEFAULT_COLS, DEFAULT_ROWS).expect("a game");

    assert_eq!(app.purse.balance(), STARTING_CASH + EARNED);
}

#[test]
fn a_second_window_cannot_swim_in_the_same_water_but_the_debug_bench_can() {
    let dir = scratch("lock");
    let first = Vault::open_in(dir.clone(), Launch::Player).expect("a vault");

    assert!(matches!(
        Vault::open_in(dir.clone(), Launch::Player),
        Err(VaultError::AlreadyRunning)
    ));
    let debug = Vault::open_in(dir, Launch::Debug).expect("its own slot");
    assert_ne!(debug.path(), first.path());
}

#[test]
fn an_unreadable_save_is_set_aside_and_never_overwritten() {
    let dir = scratch("garbled");
    let vault = Vault::open_in(dir, Launch::Player).expect("a vault");
    fs::write(vault.path(), "not a fishtank").unwrap();

    let Ok(Loaded::Unreadable { set_aside }) = vault.load() else {
        panic!("garbage is not a game");
    };

    assert_eq!(fs::read_to_string(set_aside).unwrap(), "not a fishtank");
    assert!(!vault.path().exists());
}

#[test]
fn a_save_from_a_newer_game_is_refused() {
    let dir = scratch("future");
    let path = dir.join("future.ron");
    let text = App::new().snapshot().to_ron().unwrap();
    let future = text.replacen(&format!("version: {SAVE_VERSION},"), "version: 999,", 1);
    assert_ne!(future, text, "the version is the first field");
    fs::write(&path, future).unwrap();

    assert!(vault::read_save(&path).is_err());
}

#[test]
fn an_exported_game_comes_back_on_import() {
    let dir = scratch("export");
    let file = dir.join("aquarium.ron");
    let mut tui = Tui::as_player(DEFAULT_COLS, DEFAULT_ROWS);
    tui.run(&format!("/export {}", quoted(&file)));
    assert!(file.exists(), "the export is a save file");
    tui.app.earn(EARNED, Flow::Godsend);

    tui.run(&format!("/import {}", quoted(&file)));

    assert_eq!(tui.app.purse.balance(), STARTING_CASH);
}

#[test]
fn importing_nothing_changes_nothing() {
    let dir = scratch("import-nothing");
    let mut tui = Tui::as_player(DEFAULT_COLS, DEFAULT_ROWS);
    tui.app.earn(EARNED, Flow::Godsend);

    tui.run(&format!("/import {}", quoted(&dir.join("missing.ron"))));

    assert_eq!(tui.app.purse.balance(), STARTING_CASH + EARNED);
}

#[test]
fn reset_starts_over_from_adam_lilith_and_eva() {
    let mut tui = rich_game();

    tui.run("/reset");

    let fresh = App::new();
    assert_eq!(tui.app.tanks.len(), 1);
    let names =
        |app: &App| -> Vec<String> { app.tanks[0].fish.iter().map(|f| f.name.clone()).collect() };
    assert_eq!(names(&tui.app), names(&fresh));
    assert_eq!(tui.app.purse.balance(), STARTING_CASH);
    assert_eq!(tui.app.food_supply, STARTING_FOOD);
    assert!(tui.app.inventory.is_empty() && tui.app.blueprints.is_empty());
    assert!(tui.app.graveyard.is_empty());
    assert!(tui.app.debug_mode, "the bench stays a bench");
}

#[test]
fn a_player_cannot_reset() {
    let mut tui = Tui::as_player(DEFAULT_COLS, DEFAULT_ROWS);
    tui.app.earn(EARNED, Flow::Godsend);

    tui.run("/reset");

    assert_eq!(tui.app.purse.balance(), STARTING_CASH + EARNED);
}

#[test]
fn the_first_save_file_ever_written_still_loads() {
    let save = vault::read_save(Path::new(FIXTURE))
        .unwrap_or_else(|error| panic!("tests/saves/fishtank-v1.ron no longer loads: {error}"))
        .save;
    let app = App::resume(save, DEFAULT_COLS, DEFAULT_ROWS);

    assert_eq!(app.tanks[app.current_tank].name, HELL_NAME);
    assert!(app.tanks.iter().any(|tank| tank.kind == TankKind::Heaven));
    assert!(app.tanks.iter().any(|tank| tank.name == MATRIX_NAME));
    assert_eq!(app.blueprints[0].name, LATCH);
    assert!(
        fish(&app, HOST)
            .script()
            .and_then(|bot| bot.chip())
            .is_some()
    );
    assert!(fish(&app, "Q").frozen);
    assert_eq!(held(&app, ConsumableKind::Milk(MilkVariant::Strawberry)), 2);
    assert!(app.graveyard.iter().any(|dead| dead.name == "Adam"));
}
