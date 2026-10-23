use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use fishtank::{
    app::{App, Launch},
    economy::{self, Money, Purse},
    fishes::{
        botfish::BotfishState,
        chip::Chip,
        fish::{Direction, FishState},
        parts::{Display, Part, PinOwner},
        species::FishSpecies,
    },
    ledger::Flow,
    loot::{CIRCUIT_BLUEPRINT_SELL_PRICE, ConsumableKind, StockItem},
    settings::{DEFAULT_STAGES_PER_TICK, STAGES_PER_TICK_MAX},
    tank::{Blueprint, Fabrication, HOURS_PER_DAY, Tank, TankBackground, TankKind, WorldSignal},
    testing::LAB_STAKE,
    void_ritual::GIVE_RESOURCE_AMOUNT,
};

const TICKS_WATCHED: usize = 60;

fn enter() -> Event {
    Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()))
}

fn run(app: &mut App, line: &str) {
    app.editor.set(line.to_string());
    app.handle_input(enter());
}

fn app_with_fish(name: &str) -> App {
    app_with_species(FishSpecies::Botfish, name)
}

fn app_with_species(species: FishSpecies, name: &str) -> App {
    let mut app = App::launch(Launch::Debug);
    app.tanks[0].spawn_fish(species, name.to_string(), &mut rand::rng());
    app
}

fn fish_named<'a>(app: &'a App, name: &str) -> &'a fishtank::fishes::fish::Fish {
    app.tanks[0]
        .fish
        .iter()
        .find(|f| f.name == name)
        .expect("the fish exists")
}

fn frozen(app: &App, name: &str) -> bool {
    fish_named(app, name).frozen
}

#[test]
fn freeze_pins_a_fish_and_unfreeze_releases_it() {
    let mut app = app_with_fish("Ann");
    assert!(!frozen(&app, "Ann"));

    run(&mut app, "/freeze \"Ann\"");
    assert!(frozen(&app, "Ann"), "the fish is pinned");

    run(&mut app, "/unfreeze \"Ann\"");
    assert!(!frozen(&app, "Ann"), "the fish swims again");
}

#[test]
fn freeze_takes_an_unquoted_name() {
    let mut app = app_with_fish("Ann");
    run(&mut app, "/freeze Ann");
    assert!(frozen(&app, "Ann"));
}

#[test]
fn a_frozen_fish_holds_its_position_across_ticks() {
    let mut app = app_with_fish("Ann");
    run(&mut app, "/freeze \"Ann\"");
    let pinned = fish_named(&app, "Ann");
    let (x, y) = (pinned.position.x, pinned.position.y);

    for _ in 0..TICKS_WATCHED {
        app.tick();
    }

    let pinned = fish_named(&app, "Ann");
    assert_eq!(pinned.position.x, x, "a frozen fish never drifts sideways");
    assert_eq!(
        pinned.position.y, y,
        "a frozen fish never drifts vertically"
    );
}

#[test]
fn freezing_an_unknown_fish_is_a_silent_no_op() {
    let mut app = app_with_fish("Ann");
    run(&mut app, "/freeze \"Nobody\"");
    assert!(!frozen(&app, "Ann"), "no other fish is touched");
}

#[test]
fn an_ordinary_fish_cannot_be_frozen() {
    let mut app = app_with_species(FishSpecies::Merluza, "Mer");

    run(&mut app, "/freeze \"Mer\"");

    assert!(
        !frozen(&app, "Mer"),
        "freezing is for fish you mean to program, not the wildlife"
    );
}

fn position(app: &App, name: &str) -> (f32, f32) {
    let fish = fish_named(app, name);
    (fish.position.x, fish.position.y)
}

fn placed_at(app: &mut App, name: &str, x: f32, y: f32) {
    let fish = app.tanks[0]
        .fish
        .iter_mut()
        .find(|f| f.name == name)
        .expect("the fish exists");
    fish.position.x = x;
    fish.position.y = y;
}

const PLACED_X: f32 = 20.0;
const PLACED_Y: f32 = 6.0;
const PLACEMENT_TOLERANCE: f32 = 0.001;

fn app_with_arrangeable_fish(name: &str) -> App {
    let mut app = app_with_species(FishSpecies::Botfish, name);
    run(&mut app, &format!("/freeze \"{name}\""));
    placed_at(&mut app, name, PLACED_X, PLACED_Y);
    app
}

#[test]
fn nudge_shifts_a_fish_by_exactly_the_given_offsets() {
    let mut app = app_with_arrangeable_fish("Ann");

    run(&mut app, "/nudge \"Ann\" 3 2");

    assert_eq!(position(&app, "Ann"), (PLACED_X + 3.0, PLACED_Y + 2.0));
}

#[test]
fn nudge_accepts_negative_offsets() {
    let mut app = app_with_arrangeable_fish("Ann");

    run(&mut app, "/nudge \"Ann\" -4 -1");

    assert_eq!(position(&app, "Ann"), (PLACED_X - 4.0, PLACED_Y - 1.0));
}

#[test]
fn nudge_clamps_to_the_tank_instead_of_escaping_it() {
    let mut app = app_with_arrangeable_fish("Ann");

    run(&mut app, "/nudge \"Ann\" -9999 -9999");

    let (x, y) = position(&app, "Ann");
    assert!(x >= 0.0, "a nudged fish never leaves the tank to the left");
    assert!(y >= 0.0, "a nudged fish never leaves the tank upward");
}

#[test]
fn nudge_without_both_offsets_is_a_no_op() {
    let mut app = app_with_arrangeable_fish("Ann");

    run(&mut app, "/nudge \"Ann\" 3");

    assert_eq!(position(&app, "Ann"), (PLACED_X, PLACED_Y));
}

#[test]
fn flip_reverses_the_way_a_fish_faces() {
    let mut app = app_with_arrangeable_fish("Ann");
    let facing_left = fish_named(&app, "Ann").facing_left();

    run(&mut app, "/flip \"Ann\"");

    assert_ne!(
        fish_named(&app, "Ann").facing_left(),
        facing_left,
        "the fish turns around"
    );

    run(&mut app, "/flip \"Ann\"");

    assert_eq!(
        fish_named(&app, "Ann").facing_left(),
        facing_left,
        "flipping twice returns it to where it started"
    );
}

#[test]
fn a_frozen_fish_keeps_the_position_it_was_nudged_to() {
    let mut app = app_with_arrangeable_fish("Ann");
    run(&mut app, "/nudge \"Ann\" 5 1");
    let placed = position(&app, "Ann");

    for _ in 0..TICKS_WATCHED {
        app.tick();
    }

    assert_eq!(position(&app, "Ann"), placed, "the board stays as arranged");
}

const FOOD_DROPPED: usize = 8;
const TICKS_FEEDING: usize = 30;
const FEEDING_TANK_W: u16 = 60;
const FEEDING_TANK_H: u16 = 20;

fn bot_state<'a>(app: &'a mut App, name: &str) -> &'a mut BotfishState {
    app.tanks[0]
        .fish
        .iter_mut()
        .find(|f| f.name == name)
        .expect("the fish exists")
        .script_mut()
        .expect("the fish is programmable")
}

fn wired(app: &App, name: &str) -> bool {
    fish_named(app, name).is_wired()
}

fn ever_seeks_food(app: &mut App, name: &str) -> bool {
    app.tanks[0].width = FEEDING_TANK_W;
    app.tanks[0].height = FEEDING_TANK_H;
    app.food_supply = FOOD_DROPPED as u32;
    run(app, "/feed");
    assert!(!app.tanks[0].food.is_empty(), "the food really fell");
    for _ in 0..TICKS_FEEDING {
        app.tick();
        if !matches!(fish_named(app, name).state, FishState::Idle) {
            return true;
        }
    }
    false
}

#[test]
fn a_bare_botfish_is_unwired_and_still_chases_food() {
    let mut app = app_with_fish("Neo");
    assert!(!wired(&app, "Neo"), "nothing is connected to it yet");
    assert!(
        ever_seeks_food(&mut app, "Neo"),
        "an unwired botfish is just a fish"
    );
}

#[test]
fn listening_to_a_channel_alone_wires_a_botfish() {
    let mut app = app_with_fish("Neo");
    bot_state(&mut app, "Neo").listen("clk");
    assert!(wired(&app, "Neo"));
    assert!(
        !ever_seeks_food(&mut app, "Neo"),
        "a fish on a wire has a job"
    );
}

#[test]
fn driving_a_channel_alone_wires_a_botfish() {
    let mut app = app_with_fish("Neo");
    bot_state(&mut app, "Neo").drive("q");
    assert!(wired(&app, "Neo"));
    assert!(!ever_seeks_food(&mut app, "Neo"));
}

#[test]
fn installing_a_part_alone_wires_a_botfish() {
    let mut app = app_with_fish("Neo");
    bot_state(&mut app, "Neo").install(Part::InverterCoil);
    assert!(wired(&app, "Neo"));
    assert!(!ever_seeks_food(&mut app, "Neo"));
}

const CHAIN: [&str; 3] = ["A", "B", "C"];
const SMALL_RING: usize = 3;
const RING_LAPS: usize = 4;
const RING_FISH: usize = 60;
const RESPONSIVE_LIMIT: Duration = Duration::from_millis(500);

fn app_with_chain() -> App {
    let mut app = App::launch(Launch::Debug);
    let mut rng = rand::rng();
    for name in CHAIN {
        app.tanks[0].spawn_fish(FishSpecies::Botfish, name.to_string(), &mut rng);
    }
    for (i, name) in CHAIN.iter().enumerate() {
        let listens = if i == 0 {
            "x".to_string()
        } else {
            CHAIN[i - 1].to_ascii_lowercase()
        };
        let bot = bot_state(&mut app, name);
        bot.listen(&listens);
        bot.drive(&name.to_ascii_lowercase());
    }
    app.tanks[0].channels.set_level("x", true);
    app
}

fn level(app: &App, channel: &str) -> bool {
    app.tanks[0].channels.level(channel)
}

fn app_with_ring(size: usize) -> App {
    let mut app = App::launch(Launch::Debug);
    app.tanks[0].fish.clear();
    app.tanks[0].expand(size as u32);
    let mut rng = rand::rng();
    for i in 0..size {
        app.tanks[0].spawn_fish(FishSpecies::Botfish, format!("Ring{i}"), &mut rng);
    }
    for i in 0..size {
        let bot = bot_state(&mut app, &format!("Ring{i}"));
        bot.listen(&format!("c{i}"));
        bot.drive(&format!("c{}", (i + 1) % size));
    }
    app.tanks[0].channels.set_level("c0", true);
    app
}

#[test]
fn a_signal_walks_exactly_one_fish_per_stage() {
    let mut app = app_with_chain();

    app.tick();
    assert!(level(&app, "a"), "the first fish saw its input");
    assert!(
        !level(&app, "b"),
        "the fabric never settles ahead of itself"
    );
    assert!(!level(&app, "c"));

    app.tick();
    assert!(level(&app, "b"));
    assert!(!level(&app, "c"));

    app.tick();
    assert!(level(&app, "c"), "three fish cost three stages");
}

#[test]
fn the_default_clock_is_one_stage_per_tick() {
    let app = App::launch(Launch::Debug);
    assert_eq!(app.settings.stages_per_tick, DEFAULT_STAGES_PER_TICK);
    assert_eq!(
        DEFAULT_STAGES_PER_TICK, 1,
        "the readable mode is the default"
    );
}

#[test]
fn the_clock_runs_the_fabric_ahead_of_the_frame_rate() {
    let mut app = app_with_chain();
    run(&mut app, "/clock 3");

    app.tick();

    assert!(
        level(&app, "c"),
        "three stages in one tick carried the signal the whole way"
    );
}

#[test]
fn coffee_multiplies_the_clock() {
    let mut app = App::launch(Launch::Debug);
    assert_eq!(app.tick_fabric(), 1, "one stage at the default clock");

    run(&mut app, "/add coffee 1");
    run(&mut app, "/consume coffee");

    assert_eq!(
        app.tick_fabric(),
        2,
        "one stack of coffee doubles the clock"
    );
}

#[test]
fn the_clock_is_set_by_command_and_refuses_nonsense() {
    let mut app = App::launch(Launch::Debug);

    run(&mut app, "/clock 12");
    assert_eq!(app.settings.stages_per_tick, 12);

    run(&mut app, "/clock 0");
    assert_eq!(
        app.settings.stages_per_tick, 12,
        "a stopped clock is refused"
    );

    run(
        &mut app,
        &format!("/clock {}", STAGES_PER_TICK_MAX as u64 + 1),
    );
    assert_eq!(app.settings.stages_per_tick, 12, "out of range is refused");

    run(&mut app, "/clock fast");
    assert_eq!(app.settings.stages_per_tick, 12, "garbage is refused");
}

#[test]
fn a_ring_of_fish_never_settles() {
    let mut app = app_with_ring(SMALL_RING);
    let token_at = |app: &App| (0..SMALL_RING).find(|i| level(app, &format!("c{i}")));

    assert_eq!(token_at(&app), Some(0));
    for stage in 1..=(SMALL_RING * RING_LAPS) {
        app.tick_fabric();
        assert_eq!(
            token_at(&app),
            Some(stage % SMALL_RING),
            "the pulse keeps circling; oscillation costs one stage, never a settle loop"
        );
    }
}

#[test]
fn the_time_budget_cuts_an_impossible_clock_short() {
    let mut app = app_with_ring(RING_FISH);
    run(&mut app, &format!("/clock {STAGES_PER_TICK_MAX}"));

    let started = Instant::now();
    let advanced = app.tick_fabric();
    let elapsed = started.elapsed();

    assert!(
        advanced < STAGES_PER_TICK_MAX,
        "the budget stopped the fabric at {advanced} stages, short of what was asked"
    );
    assert!(advanced > 0, "the fabric still advanced");
    assert!(
        elapsed < RESPONSIVE_LIMIT,
        "a tick took {elapsed:?}, so no circuit can freeze the aquarium"
    );
}

#[test]
fn the_sim_stays_responsive_at_the_maximum_clock() {
    let mut app = app_with_ring(RING_FISH);
    run(&mut app, &format!("/clock {STAGES_PER_TICK_MAX}"));

    let started = Instant::now();
    app.tick();
    let elapsed = started.elapsed();

    assert!(
        elapsed < RESPONSIVE_LIMIT,
        "a full tick with a tank of oscillators took {elapsed:?}"
    );
}

const SETTLE_STAGES: usize = 4;
const NOON: u32 = 12;
const COIL: &[Part] = &[Part::InverterCoil];
const BARE: &[Part] = &[];
const TRUTH_TABLE: [(bool, bool); 4] = [(false, false), (false, true), (true, false), (true, true)];

fn board(names: &[&str]) -> App {
    let mut app = App::launch(Launch::Debug);
    app.tanks[0].fish.clear();
    let mut rng = rand::rng();
    for name in names {
        assert!(
            app.tanks[0].spawn_fish(FishSpecies::Botfish, name.to_string(), &mut rng),
            "the board has room for {name}"
        );
    }
    app
}

fn gate(app: &mut App, name: &str, listens: &[&str], drives: &str, parts: &[Part]) {
    let bot = bot_state(app, name);
    for channel in listens {
        bot.listen(channel);
    }
    bot.drive(drives);
    for &part in parts {
        assert!(bot.install(part), "{name} takes a {}", part.display_name());
    }
}

fn set(app: &mut App, channel: &str, level: bool) {
    app.tanks[0].channels.set_level(channel, level);
}

fn step(app: &mut App) {
    let cash = economy::reading(app.purse.balance());
    let mut world = app.tanks[0].observe(cash, NOON);
    app.tanks[0].advance_stage(&mut world);
}

fn advance(app: &mut App, stages: usize) {
    for _ in 0..stages {
        step(app);
    }
}

fn trace(app: &mut App, channel: &str, stages: usize) -> Vec<bool> {
    (0..stages)
        .map(|_| {
            step(app);
            level(app, channel)
        })
        .collect()
}

#[test]
fn a_coil_turns_one_fish_into_a_not_gate() {
    let mut app = board(&["N"]);
    gate(&mut app, "N", &["x"], "nx", COIL);

    advance(&mut app, SETTLE_STAGES);
    assert!(
        level(&app, "nx"),
        "a low input is denied into a high output"
    );

    set(&mut app, "x", true);
    advance(&mut app, SETTLE_STAGES);
    assert!(!level(&app, "nx"), "and a high input into a low one");
}

fn and_board() -> App {
    let mut app = board(&["Na", "Nb", "And"]);
    gate(&mut app, "Na", &["a"], "na", COIL);
    gate(&mut app, "Nb", &["b"], "nb", COIL);
    gate(&mut app, "And", &["na", "nb"], "q", COIL);
    app
}

#[test]
fn three_coiled_fish_are_an_and_gate() {
    for (a, b) in TRUTH_TABLE {
        let mut app = and_board();
        set(&mut app, "a", a);
        set(&mut app, "b", b);

        advance(&mut app, SETTLE_STAGES);

        assert_eq!(level(&app, "q"), a && b, "AND({a}, {b})");
    }
}

fn xor_board() -> App {
    let mut app = board(&["Na", "Nb", "P", "Q"]);
    gate(&mut app, "Na", &["a"], "na", COIL);
    gate(&mut app, "Nb", &["b"], "nb", COIL);
    gate(&mut app, "P", &["na", "b"], "xor", COIL);
    gate(&mut app, "Q", &["a", "nb"], "xor", COIL);
    app
}

#[test]
fn four_coiled_fish_are_an_xor_gate() {
    for (a, b) in TRUTH_TABLE {
        let mut app = xor_board();
        set(&mut app, "a", a);
        set(&mut app, "b", b);

        advance(&mut app, SETTLE_STAGES);

        assert_eq!(
            level(&app, "xor"),
            a != b,
            "XOR({a}, {b}) — the two halves meet on one channel, and fan-in is wired-OR"
        );
    }
}

const OSCILLATOR_STAGES: usize = 24;

fn assert_period(levels: &[bool], period: usize) {
    for (i, &now) in levels.iter().enumerate() {
        if i + period < levels.len() {
            assert_eq!(now, levels[i + period], "stage {i} repeats {period} later");
        }
        if i + period / 2 < levels.len() {
            assert_ne!(
                now,
                levels[i + period / 2],
                "stage {i} is the opposite of half a period later"
            );
        }
    }
}

#[test]
fn a_coiled_fish_that_hears_itself_oscillates_every_stage() {
    let mut app = board(&["Osc"]);
    gate(&mut app, "Osc", &["q"], "q", COIL);
    set(&mut app, "q", true);

    let levels = trace(&mut app, "q", OSCILLATOR_STAGES);

    assert_period(&levels, 2);
}

#[test]
fn a_longer_ring_of_coils_oscillates_more_slowly() {
    let ring = ["R0", "R1", "R2"];
    let mut app = board(&ring);
    for (i, name) in ring.iter().enumerate() {
        let listens = format!("c{}", (i + ring.len() - 1) % ring.len());
        gate(&mut app, name, &[&listens], &format!("c{i}"), COIL);
    }
    set(&mut app, "c0", true);

    let levels = trace(&mut app, "c0", OSCILLATOR_STAGES);

    assert_period(&levels, 2 * ring.len());
}

#[test]
fn spools_stretch_the_period_of_a_ring_one_stage_each() {
    for spools in 1..=2u32 {
        let mut app = board(&["Osc"]);
        let mut parts = vec![Part::InverterCoil];
        parts.extend(std::iter::repeat_n(Part::DelaySpool, spools as usize));
        gate(&mut app, "Osc", &["q"], "q", &parts);
        set(&mut app, "q", true);

        let levels = trace(&mut app, "q", OSCILLATOR_STAGES);

        assert_period(&levels, 2 * (spools as usize + 1));
    }
}

fn latch_board() -> App {
    let mut app = board(&["Q", "Qn"]);
    gate(&mut app, "Q", &["r", "qn"], "q", COIL);
    gate(&mut app, "Qn", &["s", "q"], "qn", COIL);
    app
}

#[test]
fn two_cross_coupled_fish_remember_a_pulse() {
    let mut app = latch_board();

    set(&mut app, "s", true);
    advance(&mut app, SETTLE_STAGES);
    assert!(level(&app, "q"), "setting raises q");

    set(&mut app, "s", false);
    advance(&mut app, SETTLE_STAGES);
    assert!(level(&app, "q"), "and q holds after the set input drops");
    assert!(!level(&app, "qn"), "its complement stays down");

    set(&mut app, "r", true);
    advance(&mut app, SETTLE_STAGES);
    assert!(!level(&app, "q"), "resetting clears it");

    set(&mut app, "r", false);
    advance(&mut app, SETTLE_STAGES);
    assert!(!level(&app, "q"), "and it holds cleared");
}

#[test]
fn driving_a_latch_both_ways_at_once_is_the_illegal_state() {
    let mut app = latch_board();

    set(&mut app, "s", true);
    set(&mut app, "r", true);
    advance(&mut app, SETTLE_STAGES);

    assert!(!level(&app, "q"), "both outputs are pulled low");
    assert!(
        !level(&app, "qn"),
        "q and its complement agree, which is the documented illegal state"
    );
}

const SPOOL: &[Part] = &[Part::DelaySpool];
const WARM_UP_STAGES: usize = 4;
const EDGE_WATCHED_STAGES: usize = 8;

fn edge_detector(delay: &[Part]) -> App {
    let mut app = board(&["D", "Nx", "P"]);
    gate(&mut app, "D", &["x"], "xd", delay);
    gate(&mut app, "Nx", &["x"], "nx", COIL);
    gate(&mut app, "P", &["nx", "xd"], "pulse", COIL);
    advance(&mut app, WARM_UP_STAGES);
    app
}

#[test]
fn an_edge_detector_fires_for_exactly_one_stage() {
    let mut app = edge_detector(SPOOL);
    assert!(!level(&app, "pulse"), "nothing has happened yet");

    set(&mut app, "x", true);
    let levels = trace(&mut app, "pulse", EDGE_WATCHED_STAGES);

    assert_eq!(
        levels.iter().filter(|&&high| high).count(),
        1,
        "a rising edge fires once, not for as long as the input stays up"
    );
}

#[test]
fn the_same_edge_detector_without_its_spool_never_fires() {
    let mut app = edge_detector(BARE);

    set(&mut app, "x", true);
    let levels = trace(&mut app, "pulse", EDGE_WATCHED_STAGES);

    assert!(
        !levels.iter().any(|&high| high),
        "without the delay the two paths arrive together, so the pulse never exists"
    );
}

const DARK: (bool, bool) = (false, false);
const LIT: (bool, bool) = (true, true);

fn probe(app: &App, name: &str) -> (bool, bool) {
    let bot = fish_named(app, name)
        .script()
        .expect("the fish is programmable");
    (bot.input_level(), bot.output_level())
}

#[test]
fn a_signal_lights_each_fish_in_turn() {
    let mut app = app_with_chain();
    for name in CHAIN {
        assert_eq!(probe(&app, name), DARK, "{name} starts dark");
    }

    advance(&mut app, 1);
    assert_eq!(
        probe(&app, "A"),
        LIT,
        "the first fish sees it and passes it on"
    );
    assert_eq!(probe(&app, "B"), DARK, "the light has not reached B yet");

    advance(&mut app, 1);
    assert_eq!(probe(&app, "B"), LIT);
    assert_eq!(probe(&app, "C"), DARK);

    advance(&mut app, 1);
    assert_eq!(
        probe(&app, "C"),
        LIT,
        "the signal walked the board fish by fish, in the open"
    );
}

#[test]
fn a_coiled_fish_lights_its_antenna_while_its_eyes_stay_dark() {
    let mut app = board(&["N"]);
    gate(&mut app, "N", &["x"], "nx", COIL);

    advance(&mut app, 1);
    assert_eq!(probe(&app, "N"), (false, true), "nothing in, a denial out");

    set(&mut app, "x", true);
    advance(&mut app, 1);
    assert_eq!(probe(&app, "N"), (true, false), "and the other way round");
}

#[test]
fn a_spooled_fish_lights_its_eyes_a_stage_before_its_antenna() {
    let mut app = board(&["D"]);
    gate(&mut app, "D", &["x"], "xd", SPOOL);
    set(&mut app, "x", true);

    advance(&mut app, 1);
    assert_eq!(
        probe(&app, "D"),
        (true, false),
        "the signal arrived but the spool still holds it"
    );

    advance(&mut app, 1);
    assert_eq!(probe(&app, "D"), LIT, "a stage later it leaves");
}

#[test]
fn a_fused_host_carries_the_wiring_of_the_botfish_inside_it() {
    let mut app = app_with_fish("Neo");
    let bot = bot_state(&mut app, "Neo");
    bot.listen("x");
    bot.drive("nx");
    bot.install(Part::DelaySpool);
    bot.install(Part::DelaySpool);

    let carried = fish_named(&app, "Neo")
        .capture_program()
        .expect("the program travels through fusion");

    assert!(carried.hears("x"));
    assert_eq!(carried.drives(), Some("nx"));
    assert_eq!(carried.parts().count(Part::DelaySpool), 2);
    assert!(carried.is_wired());
}

const SHOAL_BUS: &str = "shoal";
const SHOAL_BUS_WIDTH: u8 = 8;
const STOCK_THRESHOLD: &str = "2";

fn sense(app: &mut App, name: &str, part: Part, config: &[(&str, &str)], pins: &[(&str, &str)]) {
    let bot = bot_state(app, name);
    assert!(bot.install(part), "{name} takes a {}", part.display_name());
    for &(field, value) in config {
        let spec = part
            .config()
            .iter()
            .find(|spec| spec.name == field)
            .unwrap_or_else(|| panic!("{part:?} has no {field} field"));
        bot.configure(part, spec, value);
    }
    for &(pin, channel) in pins {
        assert!(bot.wire(part, pin, channel), "{name} wires {pin}");
    }
}

fn bus_value(app: &App, base: &str, width: u8) -> u32 {
    (0..width)
        .filter(|bit| app.tanks[0].channels.level(&format!("{base}{bit}")))
        .map(|bit| 1u32 << bit)
        .sum()
}

fn stock_tank(app: &mut App, species: FishSpecies, count: usize) {
    let mut rng = rand::rng();
    for index in 0..count {
        assert!(app.tanks[0].spawn_fish(species, format!("Fry{index}"), &mut rng));
    }
}

#[test]
fn a_shoal_counter_puts_the_tank_population_on_a_bus() {
    let mut app = board(&["Eye"]);
    sense(
        &mut app,
        "Eye",
        Part::ShoalCounter,
        &[],
        &[("count", SHOAL_BUS)],
    );
    stock_tank(&mut app, FishSpecies::Merluza, 5);

    advance(&mut app, SETTLE_STAGES);

    assert_eq!(
        bus_value(&app, SHOAL_BUS, SHOAL_BUS_WIDTH),
        6,
        "five merluza and the counter fish itself"
    );
}

#[test]
fn a_targeted_counter_ignores_everything_it_was_not_asked_about() {
    let mut app = board(&["Eye"]);
    sense(
        &mut app,
        "Eye",
        Part::ShoalCounter,
        &[("target", "betta")],
        &[("count", SHOAL_BUS)],
    );
    stock_tank(&mut app, FishSpecies::Merluza, 5);

    advance(&mut app, SETTLE_STAGES);

    assert_eq!(bus_value(&app, SHOAL_BUS, SHOAL_BUS_WIDTH), 0);
}

#[test]
fn a_flag_pin_drives_a_plain_channel_with_no_bit_suffix() {
    let mut app = board(&["Eye"]);
    sense(
        &mut app,
        "Eye",
        Part::ShoalCounter,
        &[("target", "merluza"), ("threshold", STOCK_THRESHOLD)],
        &[("over", "plenty"), ("under", "thin")],
    );

    advance(&mut app, SETTLE_STAGES);
    assert!(level(&app, "thin"), "an empty pen is under the floor");
    assert!(!level(&app, "plenty"));
    assert!(
        !app.tanks[0].channels.contains("thin0"),
        "a one-bit pin keeps its bare channel name"
    );

    stock_tank(&mut app, FishSpecies::Merluza, 5);
    advance(&mut app, SETTLE_STAGES);

    assert!(level(&app, "plenty"));
    assert!(!level(&app, "thin"));
}

#[test]
fn a_sensed_flag_feeds_the_gates_downstream_of_it() {
    let mut app = board(&["Eye", "Not"]);
    sense(
        &mut app,
        "Eye",
        Part::ShoalCounter,
        &[("threshold", STOCK_THRESHOLD)],
        &[("over", "plenty")],
    );
    gate(&mut app, "Not", &["plenty"], "quiet", COIL);
    stock_tank(&mut app, FishSpecies::Merluza, 5);

    advance(&mut app, SETTLE_STAGES);

    assert!(level(&app, "plenty"));
    assert!(!level(&app, "quiet"), "the coil denies a crowded tank");
}

#[test]
fn a_ledger_nerve_reads_the_purse_the_player_is_holding() {
    let mut app = board(&["Bank"]);
    app.purse = Purse::holding(1234);
    sense(
        &mut app,
        "Bank",
        Part::LedgerNerve,
        &[],
        &[("cash", "purse"), ("over", "rich")],
    );

    advance(&mut app, SETTLE_STAGES);

    assert_eq!(bus_value(&app, "purse", 16), 1234);
    assert!(level(&app, "rich"));
}

#[test]
fn a_startle_nerve_fires_for_exactly_one_stage_when_a_fish_is_born() {
    let mut app = board(&["Nerve"]);
    sense(
        &mut app,
        "Nerve",
        Part::StartleNerve,
        &[],
        &[("birth", "hatched")],
    );
    advance(&mut app, SETTLE_STAGES);
    assert!(!level(&app, "hatched"));

    stock_tank(&mut app, FishSpecies::Merluza, 1);

    let mut world = {
        let cash = economy::reading(app.purse.balance());
        app.tanks[0].observe(cash, NOON)
    };
    app.tanks[0].advance_stage(&mut world);
    assert!(level(&app, "hatched"), "the birth reaches the wire");

    app.tanks[0].advance_stage(&mut world);
    assert!(
        !level(&app, "hatched"),
        "an interrupt is a pulse, not a level"
    );
}

#[test]
fn an_unwired_sense_pin_drives_nothing_at_all() {
    let mut app = board(&["Eye"]);
    sense(
        &mut app,
        "Eye",
        Part::ShoalCounter,
        &[],
        &[("count", SHOAL_BUS)],
    );

    advance(&mut app, SETTLE_STAGES);

    assert!(
        !app.tanks[0].channels.contains("full"),
        "a pin nobody wired never registers a channel"
    );
}

#[test]
fn an_assay_scale_weighs_the_richest_fish_it_can_find() {
    let mut app = board(&["Scale"]);
    sense(
        &mut app,
        "Scale",
        Part::AssayScale,
        &[("target", "richest merluza")],
        &[("value", "worth")],
    );
    stock_tank(&mut app, FishSpecies::Merluza, 3);

    advance(&mut app, SETTLE_STAGES);

    let richest = app.tanks[0]
        .fish
        .iter()
        .filter(|fish| fish.species == FishSpecies::Merluza)
        .map(|fish| fish.sell_value())
        .max()
        .expect("the pen has merluza in it");
    assert_eq!(Money::from(bus_value(&app, "worth", 16)), richest);
}

#[test]
fn a_sense_only_reports_on_the_tank_it_swims_in() {
    let mut app = board(&["Eye"]);
    sense(
        &mut app,
        "Eye",
        Part::ShoalCounter,
        &[],
        &[("count", SHOAL_BUS)],
    );
    app.tanks
        .push(Tank::new("Next".to_string(), TankKind::Base, &[]));
    let mut rng = rand::rng();
    for index in 0..9 {
        app.tanks[1].spawn_fish(FishSpecies::Merluza, format!("Far{index}"), &mut rng);
    }

    advance(&mut app, SETTLE_STAGES);

    assert_eq!(
        bus_value(&app, SHOAL_BUS, SHOAL_BUS_WIDTH),
        1,
        "the counter sees its own tank only"
    );
}

const SENSING_FISH: usize = 40;
const SENSE_CONFIG: &[(&str, &str)] = &[("target", "merluza"), ("threshold", "7")];

#[test]
fn a_configured_sense_survives_being_captured_for_fusion() {
    let mut app = board(&["Eye"]);
    sense(
        &mut app,
        "Eye",
        Part::ShoalCounter,
        SENSE_CONFIG,
        &[("count", SHOAL_BUS)],
    );

    let carried = bot_state(&mut app, "Eye").clone();

    let spec = Part::ShoalCounter
        .config()
        .iter()
        .find(|spec| spec.name == "threshold")
        .expect("the counter has a threshold");
    assert_eq!(carried.config(Part::ShoalCounter).number(spec), 7);
    assert_eq!(
        carried.pins().channel(Part::ShoalCounter, "count"),
        Some(SHOAL_BUS)
    );
}

#[test]
fn a_tank_of_senses_stays_inside_its_stage_budget() {
    let mut app = board(&[]);
    let mut rng = rand::rng();
    for index in 0..SENSING_FISH {
        let name = format!("Eye{index}");
        assert!(app.tanks[0].spawn_fish(FishSpecies::Botfish, name.clone(), &mut rng));
        sense(
            &mut app,
            &name,
            Part::ShoalCounter,
            SENSE_CONFIG,
            &[("count", &format!("bus{index}"))],
        );
    }
    run(&mut app, &format!("/clock {STAGES_PER_TICK_MAX}"));

    let started = Instant::now();
    let advanced = app.tick_fabric();
    let elapsed = started.elapsed();

    assert!(advanced > 0, "the fabric still advanced");
    assert!(
        elapsed < RESPONSIVE_LIMIT,
        "a full tick with a tank of senses took {elapsed:?}"
    );
}

const BONUS_FISH: &str = "Berry";
const PLAIN_FISH: &str = "Plain";

fn worth(app: &App, name: &str) -> Money {
    app.tanks[0]
        .fish
        .iter()
        .find(|f| f.name == name)
        .expect("the fish exists")
        .sell_value()
}

fn twin_fish(app: &mut App) {
    let mut rng = rand::rng();
    for name in [PLAIN_FISH, BONUS_FISH] {
        assert!(app.tanks[0].spawn_fish(FishSpecies::Merluza, name.to_string(), &mut rng));
    }
    let twin = app.tanks[0]
        .fish
        .iter()
        .find(|f| f.name == PLAIN_FISH)
        .map(|f| (f.weight_g, f.size_category))
        .expect("the fish exists");
    for name in [PLAIN_FISH, BONUS_FISH] {
        let fish = app.tanks[0]
            .fish
            .iter_mut()
            .find(|f| f.name == name)
            .expect("the fish exists");
        fish.weight_g = twin.0;
        fish.size_category = twin.1;
    }
    assert!(
        worth(app, PLAIN_FISH) > 0,
        "a merluza is worth something to begin with"
    );
    assert_eq!(
        worth(app, PLAIN_FISH),
        worth(app, BONUS_FISH),
        "the twins start identical, so any later gap is the bonus"
    );
}

#[test]
fn a_strawberry_fish_is_actually_worth_more_than_its_plain_twin() {
    let mut app = board(&[]);
    twin_fish(&mut app);

    assert!(
        app.tanks[0].apply_named_mutation(BONUS_FISH, "strawberry"),
        "the strawberry mutation applies"
    );

    let plain = worth(&app, PLAIN_FISH);
    let berry = worth(&app, BONUS_FISH);
    assert!(
        berry > plain,
        "a +25% sell bonus that pays nothing is a lie: {berry} vs {plain}"
    );
}

#[test]
fn the_sell_bonus_is_the_percentage_it_promises() {
    let mut app = board(&[]);
    twin_fish(&mut app);
    let plain = worth(&app, PLAIN_FISH);

    let fish = app.tanks[0]
        .fish
        .iter_mut()
        .find(|f| f.name == BONUS_FISH)
        .expect("the fish exists");
    fish.sell_price_bonus_pct = 100;

    assert_eq!(
        worth(&app, BONUS_FISH),
        plain * 2,
        "a hundred percent bonus doubles the price"
    );
}

#[test]
fn selling_a_bonused_fish_pays_the_price_the_shop_quoted() {
    let mut app = board(&[]);
    twin_fish(&mut app);
    let fish = app.tanks[0]
        .fish
        .iter_mut()
        .find(|f| f.name == BONUS_FISH)
        .expect("the fish exists");
    fish.sell_price_bonus_pct = 50;
    let quoted = worth(&app, BONUS_FISH);
    let purse = app.purse.balance();

    run(&mut app, &format!("/sell fish \"{BONUS_FISH}\""));

    assert_eq!(
        app.purse.balance() - purse,
        quoted,
        "the till and the price tag must agree"
    );
}

#[test]
fn a_fish_with_no_bonus_is_paid_exactly_its_base_price() {
    let mut app = board(&[]);
    twin_fish(&mut app);
    let base = app.tanks[0]
        .fish
        .iter()
        .find(|f| f.name == PLAIN_FISH)
        .map(|f| f.species.sell_value(f.weight_g, f.size_category))
        .expect("the fish exists");

    assert_eq!(
        worth(&app, PLAIN_FISH),
        Money::from(base),
        "no bonus, no change"
    );
}

const PULSE_STAGES: usize = 2;

fn nerve(app: &mut App, name: &str, pin: &str, channel: &str) {
    sense(app, name, Part::StartleNerve, &[], &[(pin, channel)]);
}

fn first_stage(app: &mut App) -> bool {
    let cash = economy::reading(app.purse.balance());
    let mut world = app.tanks[0].observe(cash, NOON);
    app.tanks[0].advance_stage(&mut world);
    true
}

#[test]
fn a_fish_split_off_by_cytokinesis_pulses_the_birth_line() {
    let mut app = board(&["Nerve"]);
    nerve(&mut app, "Nerve", "birth", "hatched");
    let mut rng = rand::rng();
    assert!(app.tanks[0].spawn_fish(FishSpecies::Merluza, "Ma".to_string(), &mut rng));
    advance(&mut app, SETTLE_STAGES);
    assert!(!level(&app, "hatched"), "the board is quiet to begin with");

    assert!(
        app.tanks[0].apply_named_mutation("Ma", "telophase"),
        "a fish must be double before it can split"
    );
    advance(&mut app, PULSE_STAGES);
    assert!(!level(&app, "hatched"), "telophase makes no new fish");

    let before = app.tanks[0].fish.len();
    assert!(app.tanks[0].apply_named_mutation("Ma", "cytokinesis"));
    assert_eq!(
        app.tanks[0].fish.len(),
        before + 1,
        "cytokinesis makes a new fish"
    );

    first_stage(&mut app);
    assert!(
        level(&app, "hatched"),
        "a fish born by splitting is still a fish being born"
    );
}

#[test]
fn a_birth_pulse_lasts_one_stage_however_the_fish_arrived() {
    let mut app = board(&["Nerve"]);
    nerve(&mut app, "Nerve", "birth", "hatched");
    advance(&mut app, SETTLE_STAGES);

    let mut rng = rand::rng();
    assert!(app.tanks[0].spawn_fish(FishSpecies::Merluza, "Fry".to_string(), &mut rng));

    advance(&mut app, PULSE_STAGES);

    assert!(
        !level(&app, "hatched"),
        "the pulse is spent by the second stage"
    );
}

#[test]
fn a_fish_admitted_to_a_hell_tank_is_still_marked_by_the_one_door() {
    let mut tank = Tank::new("Inferno".to_string(), TankKind::Hell, &[]);
    let mut rng = rand::rng();

    assert!(tank.spawn_fish(FishSpecies::Merluza, "Damned".to_string(), &mut rng));

    assert!(
        tank.fish
            .iter()
            .find(|f| f.name == "Damned")
            .expect("the fish exists")
            .devil_marked,
        "admit() marks the fish, so no spawn path can forget to"
    );
}

#[test]
fn every_admitted_fish_reserves_its_name() {
    let mut app = board(&[]);
    let mut rng = rand::rng();
    app.tanks[0].spawn_fish(FishSpecies::Merluza, "Twin".to_string(), &mut rng);
    app.tanks[0].spawn_fish(FishSpecies::Merluza, "Twin".to_string(), &mut rng);

    let names: Vec<&str> = app.tanks[0].fish.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names.len(), 2);
    assert_ne!(names[0], names[1], "admit() reserves the name it was given");
}

const DESERT_TANK: usize = 1;
const ABDUCTION_TICKS: usize = 30 * 60;
const MIDNIGHT_SKY: f32 = std::f32::consts::PI * 1.5;

fn desert_at_night(app: &mut App) {
    let mut tank = Tank::new("Dunes".to_string(), TankKind::Desert, &[]);
    let mut rng = rand::rng();
    tank.spawn_fish(FishSpecies::Merluza, "Taken".to_string(), &mut rng);
    tank.spawn_fish(FishSpecies::Botfish, "Nerve".to_string(), &mut rng);
    app.tanks.push(tank);

    let TankBackground::Desert { bg } = &mut app.tanks[DESERT_TANK].background else {
        panic!("a desert tank has a desert sky");
    };
    bg.sky_angle = MIDNIGHT_SKY;
    assert!(
        app.tanks[DESERT_TANK].background.is_night(),
        "abduction only happens after dark"
    );
}

fn wire_nerve_in_desert(app: &mut App, pin: &str, channel: &str) {
    let bot = app.tanks[DESERT_TANK]
        .fish
        .iter_mut()
        .find(|f| f.name == "Nerve")
        .expect("the nerve fish exists")
        .script_mut()
        .expect("the nerve fish is programmable");
    assert!(bot.install(Part::StartleNerve));
    assert!(bot.wire(Part::StartleNerve, pin, channel));
}

#[test]
fn an_abduction_pulses_the_line_of_the_tank_it_happens_in() {
    let mut app = board(&[]);
    desert_at_night(&mut app);
    wire_nerve_in_desert(&mut app, "abduction", "taken");
    app.current_tank = DESERT_TANK;

    app.tanks[DESERT_TANK].ufo_timer = 0.0;
    let mut pulsed = false;
    for _ in 0..ABDUCTION_TICKS {
        app.tick();
        pulsed |= app.tanks[DESERT_TANK].channels.level("taken");
        if pulsed {
            break;
        }
    }

    assert!(
        !app.tanks[DESERT_TANK]
            .fish
            .iter()
            .any(|f| f.name == "Taken"),
        "the fish was actually abducted"
    );
    assert!(pulsed, "a farm hears the abduction it watched");
}

#[test]
fn a_tank_nobody_senses_does_not_pay_to_look_at_itself() {
    let mut app = board(&["Coil"]);
    gate(&mut app, "Coil", &["x"], "nx", COIL);
    stock_tank(&mut app, FishSpecies::Merluza, 3);

    assert!(
        !app.tanks[0].reads_world(),
        "a coil computes, it does not look"
    );
    let cash = economy::reading(app.purse.balance());
    assert!(
        app.tanks[0].observe(cash, NOON).shoal().is_empty(),
        "an unobserved view never counts the fish"
    );
}

#[test]
fn installing_one_sense_makes_the_whole_tank_worth_observing() {
    let mut app = board(&["Coil", "Eye"]);
    gate(&mut app, "Coil", &["x"], "nx", COIL);
    stock_tank(&mut app, FishSpecies::Merluza, 3);
    assert!(!app.tanks[0].reads_world());

    sense(
        &mut app,
        "Eye",
        Part::ShoalCounter,
        &[],
        &[("count", SHOAL_BUS)],
    );

    assert!(app.tanks[0].reads_world());
    let cash = economy::reading(app.purse.balance());
    assert_eq!(
        app.tanks[0].observe(cash, NOON).shoal().len(),
        5,
        "the view sees every fish, sensing or not"
    );
}

#[test]
fn an_unobserved_tank_still_drains_its_pulses_instead_of_hoarding_them() {
    let mut app = board(&["Coil"]);
    gate(&mut app, "Coil", &["x"], "nx", COIL);
    stock_tank(&mut app, FishSpecies::Merluza, 1);

    let cash = economy::reading(app.purse.balance());
    assert!(
        app.tanks[0].observe(cash, NOON).pulsed(WorldSignal::Birth),
        "the pulse is drained even though nothing listens"
    );
    assert!(
        !app.tanks[0].observe(cash, NOON).pulsed(WorldSignal::Birth),
        "a drained pulse does not come back"
    );
}

#[test]
fn a_sense_reads_the_same_numbers_whether_or_not_a_coil_shares_its_tank() {
    let mut alone = board(&["Eye"]);
    sense(
        &mut alone,
        "Eye",
        Part::ShoalCounter,
        &[],
        &[("count", SHOAL_BUS)],
    );
    stock_tank(&mut alone, FishSpecies::Merluza, 4);
    advance(&mut alone, SETTLE_STAGES);

    assert_eq!(
        bus_value(&alone, SHOAL_BUS, SHOAL_BUS_WIDTH),
        5,
        "the gate must not change a single reading"
    );
}

const CASH_BUS_WIDTH: u8 = 16;
const CASH_BUS_CEILING: u32 = 65_535;

#[test]
fn a_purse_bigger_than_the_bus_pegs_it_instead_of_wrapping_to_nothing() {
    let mut app = board(&["Bank"]);
    app.purse = Purse::holding(Money::from(CASH_BUS_CEILING) * 4);
    sense(
        &mut app,
        "Bank",
        Part::LedgerNerve,
        &[],
        &[("cash", "purse")],
    );

    advance(&mut app, SETTLE_STAGES);

    assert_eq!(
        bus_value(&app, "purse", CASH_BUS_WIDTH),
        CASH_BUS_CEILING,
        "a rich player reads full, never broke"
    );
}

#[test]
fn a_landed_catch_is_both_a_catch_and_a_birth() {
    let mut app = board(&["Nerve"]);
    sense(
        &mut app,
        "Nerve",
        Part::StartleNerve,
        &[],
        &[("catch", "landed"), ("birth", "hatched")],
    );
    advance(&mut app, SETTLE_STAGES);
    assert!(!level(&app, "landed") && !level(&app, "hatched"));

    app.tanks[0].signal(WorldSignal::Catch);
    app.tanks[0].signal(WorldSignal::Birth);

    let cash = economy::reading(app.purse.balance());
    let mut world = app.tanks[0].observe(cash, NOON);
    app.tanks[0].advance_stage(&mut world);

    assert!(level(&app, "landed"));
    assert!(
        level(&app, "hatched"),
        "a caught fish enters the tank, so both lines are honest"
    );
}

#[test]
fn the_clock_bus_never_reports_an_hour_that_does_not_exist() {
    let mut app = board(&["Clock"]);
    sense(
        &mut app,
        "Clock",
        Part::TidalClock,
        &[],
        &[("time", "hour")],
    );

    for hour in 0..HOURS_PER_DAY {
        let cash = economy::reading(app.purse.balance());
        let mut world = app.tanks[0].observe(cash, hour);
        app.tanks[0].advance_stage(&mut world);
        assert_eq!(
            bus_value(&app, "hour", 8),
            hour,
            "the hour rides the bus intact"
        );
    }
}

const FIRE_PIN: &str = "fire";
const FIRE_CHANNEL: &str = "go";
const GATE_INPUT: &str = "x";
const SCRIPT_TICKS: usize = 40;
const NO_TRIGGER: &str = "";

fn tick_n(app: &mut App, ticks: usize) {
    for _ in 0..ticks {
        app.tick();
    }
}

fn commanded(app: &mut App, name: &str, script: &[&str]) {
    let bot = bot_state(app, name);
    assert!(bot.install(Part::CommandModule), "{name} takes a module");
    assert!(
        bot.wire(Part::CommandModule, FIRE_PIN, FIRE_CHANNEL),
        "{name} wires its fire pin"
    );
    bot.program(
        NO_TRIGGER.to_string(),
        script.iter().map(|line| line.to_string()).collect(),
    );
}

fn broke_board(names: &[&str]) -> App {
    let mut app = board(names);
    app.purse = Purse::holding(0);
    app
}

fn speech_of(app: &App, name: &str) -> Option<String> {
    fish_named(app, name)
        .speech
        .as_ref()
        .map(|bubble| bubble.text.clone())
}

#[test]
fn a_rising_fire_wire_runs_the_command_modules_program() {
    let mut app = broke_board(&["Bot"]);
    commanded(&mut app, "Bot", &["/give cash"]);

    tick_n(&mut app, SCRIPT_TICKS);
    assert_eq!(app.purse.balance(), 0, "nothing has driven the wire yet");

    set(&mut app, FIRE_CHANNEL, true);
    tick_n(&mut app, SCRIPT_TICKS);

    assert_eq!(
        app.purse.balance(),
        Money::from(GIVE_RESOURCE_AMOUNT),
        "the rise ran the program"
    );
}

#[test]
fn a_wire_held_high_runs_the_program_exactly_once() {
    let mut app = broke_board(&["Bot"]);
    commanded(&mut app, "Bot", &["/give cash"]);

    set(&mut app, FIRE_CHANNEL, true);
    tick_n(&mut app, SCRIPT_TICKS * 4);

    assert_eq!(
        app.purse.balance(),
        Money::from(GIVE_RESOURCE_AMOUNT),
        "a level is not a stream of triggers"
    );
}

#[test]
fn a_falling_wire_runs_nothing_and_the_next_rise_runs_it_again() {
    let mut app = broke_board(&["Bot"]);
    commanded(&mut app, "Bot", &["/give cash"]);

    set(&mut app, FIRE_CHANNEL, true);
    tick_n(&mut app, SCRIPT_TICKS);
    set(&mut app, FIRE_CHANNEL, false);
    tick_n(&mut app, SCRIPT_TICKS);
    assert_eq!(
        app.purse.balance(),
        Money::from(GIVE_RESOURCE_AMOUNT),
        "a fall is not a trigger"
    );

    set(&mut app, FIRE_CHANNEL, true);
    tick_n(&mut app, SCRIPT_TICKS);

    assert_eq!(
        app.purse.balance(),
        Money::from(GIVE_RESOURCE_AMOUNT) * 2,
        "a fresh rise is a fresh run"
    );
}

#[test]
fn a_gate_can_pull_the_trigger() {
    let mut app = broke_board(&["Or", "Bot"]);
    gate(&mut app, "Or", &[GATE_INPUT], FIRE_CHANNEL, BARE);
    commanded(&mut app, "Bot", &["/give cash"]);

    set(&mut app, GATE_INPUT, true);
    tick_n(&mut app, SCRIPT_TICKS);

    assert_eq!(
        app.purse.balance(),
        Money::from(GIVE_RESOURCE_AMOUNT),
        "the fabric itself pulled the trigger"
    );
}

#[test]
fn a_coil_fires_the_program_when_its_input_goes_low() {
    let mut app = broke_board(&["Not", "Bot"]);
    gate(&mut app, "Not", &[GATE_INPUT], FIRE_CHANNEL, COIL);
    commanded(&mut app, "Bot", &["/give cash"]);

    set(&mut app, GATE_INPUT, true);
    tick_n(&mut app, SCRIPT_TICKS);
    assert_eq!(app.purse.balance(), 0, "the coil is holding the wire down");

    set(&mut app, GATE_INPUT, false);
    tick_n(&mut app, SCRIPT_TICKS);

    assert_eq!(app.purse.balance(), Money::from(GIVE_RESOURCE_AMOUNT));
}

#[test]
fn a_failing_line_aborts_the_rest_of_a_fired_program() {
    let mut app = broke_board(&["Bot"]);
    commanded(&mut app, "Bot", &["/buy necronomicon 4500", "/give cash"]);

    set(&mut app, FIRE_CHANNEL, true);
    tick_n(&mut app, SCRIPT_TICKS * 3);

    assert_eq!(
        app.purse.balance(),
        0,
        "the give must never run after the bad buy"
    );
}

#[test]
fn the_whitelist_still_blocks_a_line_a_bot_may_not_run() {
    let mut app = broke_board(&["Bot"]);
    commanded(&mut app, "Bot", &["/circuit", "/give cash"]);

    set(&mut app, FIRE_CHANNEL, true);
    tick_n(&mut app, SCRIPT_TICKS * 3);

    assert_eq!(
        app.purse.balance(),
        0,
        "a rejected line aborts the whole script"
    );
}

#[test]
fn a_fired_say_puts_the_words_over_the_fish_that_ran_it() {
    let mut app = broke_board(&["Bot"]);
    commanded(&mut app, "Bot", &["/say \"glup glup\""]);

    set(&mut app, FIRE_CHANNEL, true);
    tick_n(&mut app, SCRIPT_TICKS);

    assert_eq!(speech_of(&app, "Bot"), Some("glup glup".to_string()));
}

#[test]
fn a_circuit_can_wake_another_circuit_by_saying_its_trigger() {
    let mut app = broke_board(&["Bot", "Ear"]);
    commanded(&mut app, "Bot", &["/say \"wake up\""]);
    bot_state(&mut app, "Ear").program("wake up".to_string(), vec!["/give cash".to_string()]);

    set(&mut app, FIRE_CHANNEL, true);
    tick_n(&mut app, SCRIPT_TICKS * 2);

    assert_eq!(
        app.purse.balance(),
        Money::from(GIVE_RESOURCE_AMOUNT),
        "the announcement reached the listener"
    );
}

#[test]
fn a_command_module_never_drives_a_wire_of_its_own() {
    let mut app = board(&["Bot"]);
    commanded(&mut app, "Bot", &["/feed 1"]);

    set(&mut app, FIRE_CHANNEL, true);
    advance(&mut app, SETTLE_STAGES);

    assert!(
        !app.tanks[0].channels.contains(FIRE_PIN),
        "an In pin is read, never driven"
    );
    assert_eq!(
        app.tanks[0].channels.len(),
        1,
        "only the wire the test made"
    );
}

const RICH_FISH: &str = "Croesus";
const POOR_FISH: &str = "Pauper";
const RICH_WEIGHT_G: u32 = 39_000;
const POOR_WEIGHT_G: u32 = 1_500;
const SELECTOR_SCRIPT_TICKS: usize = SCRIPT_TICKS * 3;

fn graded_pair(app: &mut App) {
    let mut rng = rand::rng();
    for name in [RICH_FISH, POOR_FISH] {
        assert!(app.tanks[0].spawn_fish(FishSpecies::Merluza, name.to_string(), &mut rng));
    }
    let size = fish_named(app, RICH_FISH).size_category;
    for (name, weight) in [(RICH_FISH, RICH_WEIGHT_G), (POOR_FISH, POOR_WEIGHT_G)] {
        let fish = app.tanks[0]
            .fish
            .iter_mut()
            .find(|f| f.name == name)
            .expect("the fish exists");
        fish.size_category = size;
        fish.weight_g = weight;
    }
    assert!(
        worth(app, RICH_FISH) > worth(app, POOR_FISH),
        "the pair has to be graded before a superlative means anything"
    );
}

fn swims_in(app: &App, tank_idx: usize, name: &str) -> bool {
    app.tanks[tank_idx].fish.iter().any(|f| f.name == name)
}

#[test]
fn a_scripted_sell_resolves_its_selector_to_the_richest_fish() {
    let mut app = broke_board(&["Bot"]);
    graded_pair(&mut app);
    let price = worth(&app, RICH_FISH);
    commanded(
        &mut app,
        "Bot",
        &["/sell fish {richest merluza}", "/give cash"],
    );

    set(&mut app, FIRE_CHANNEL, true);
    tick_n(&mut app, SELECTOR_SCRIPT_TICKS);

    assert!(
        !swims_in(&app, 0, RICH_FISH),
        "the richest went to the till"
    );
    assert!(swims_in(&app, 0, POOR_FISH), "the cheap one is seed corn");
    assert_eq!(
        app.purse.balance(),
        price + Money::from(GIVE_RESOURCE_AMOUNT),
        "the selector resolved before parse, so the line paid out and the script carried on"
    );
}

#[test]
fn a_scripted_clone_resolves_its_selector_to_the_cheapest_fish() {
    let mut app = broke_board(&["Bot"]);
    graded_pair(&mut app);
    commanded(&mut app, "Bot", &["/clone {cheapest merluza}"]);

    set(&mut app, FIRE_CHANNEL, true);
    tick_n(&mut app, SELECTOR_SCRIPT_TICKS);

    assert!(
        swims_in(&app, 0, &format!("{POOR_FISH}'s Clone")),
        "the breeder copies the fish it can most afford to lose"
    );
    assert!(
        !swims_in(&app, 0, &format!("{RICH_FISH}'s Clone")),
        "the ripe fish is for selling, not for breeding"
    );
}

#[test]
fn a_selector_that_finds_nothing_aborts_the_rest_of_the_script() {
    let mut app = broke_board(&["Bot"]);
    commanded(
        &mut app,
        "Bot",
        &["/sell fish {richest merluza}", "/give cash"],
    );

    set(&mut app, FIRE_CHANNEL, true);
    tick_n(&mut app, SELECTOR_SCRIPT_TICKS);

    assert_eq!(
        app.purse.balance(),
        0,
        "an empty tank is a truthful false, and a false line never runs the next one"
    );
}

#[test]
fn a_selector_picks_out_of_the_tank_its_bot_swims_in() {
    let mut app = broke_board(&["Bot"]);
    let mut rng = rand::rng();
    assert!(app.tanks[0].spawn_fish(FishSpecies::Merluza, POOR_FISH.to_string(), &mut rng));
    app.tanks
        .push(Tank::new("Next".to_string(), TankKind::Base, &[]));
    assert!(app.tanks[1].spawn_fish(FishSpecies::Merluza, RICH_FISH.to_string(), &mut rng));
    commanded(&mut app, "Bot", &["/sell fish {richest merluza}"]);

    set(&mut app, FIRE_CHANNEL, true);
    tick_n(&mut app, SELECTOR_SCRIPT_TICKS);

    assert!(!swims_in(&app, 0, POOR_FISH), "the bot sold its own tank");
    assert!(
        swims_in(&app, 1, RICH_FISH),
        "a selector never reaches across the glass"
    );
}

#[test]
fn a_mistyped_selector_never_silently_picks_a_fish() {
    let mut app = broke_board(&["Bot"]);
    graded_pair(&mut app);
    commanded(
        &mut app,
        "Bot",
        &[&format!("/sell fish {{heaviest {RICH_FISH}}}")],
    );

    set(&mut app, FIRE_CHANNEL, true);
    tick_n(&mut app, SELECTOR_SCRIPT_TICKS);

    assert!(swims_in(&app, 0, RICH_FISH), "a typo sells nothing");
    assert!(swims_in(&app, 0, POOR_FISH));
    assert_eq!(app.purse.balance(), 0);
}

fn lines(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| name.to_string()).collect()
}

fn captured(app: &mut App, name: &str) -> Blueprint {
    assert!(
        app.capture_blueprint(name),
        "the board has a circuit to keep"
    );
    app.blueprints
        .last()
        .cloned()
        .expect("a blueprint was kept")
}

#[test]
fn a_captured_and_gate_pins_out_exactly_its_two_inputs_and_its_output() {
    let mut app = and_board();
    advance(&mut app, SETTLE_STAGES);

    let pins = captured(&mut app, "and").pins();

    assert_eq!(pins.inputs, lines(&["a", "b"]));
    assert_eq!(pins.outputs, lines(&["q"]));
    assert_eq!(pins.internals, lines(&["na", "nb"]));
}

#[test]
fn a_captured_ring_oscillator_is_all_inside() {
    let mut app = board(&["Osc"]);
    gate(&mut app, "Osc", &["q"], "q", COIL);
    advance(&mut app, SETTLE_STAGES);

    let pins = captured(&mut app, "clock").pins();

    assert!(pins.inputs.is_empty(), "the ring feeds itself");
    assert!(pins.outputs.is_empty(), "and hears everything it drives");
    assert_eq!(pins.internals, lines(&["q"]));
}

#[test]
fn a_captured_latch_hides_its_own_outputs_because_it_hears_them() {
    let mut app = latch_board();

    let pins = captured(&mut app, "latch").pins();

    assert_eq!(pins.inputs, lines(&["r", "s"]));
    assert!(
        pins.outputs.is_empty(),
        "§9.2: a channel driven and heard inside the group is internal, so q is not a pin"
    );
    assert_eq!(pins.internals, lines(&["q", "qn"]));
}

#[test]
fn a_captured_latch_exports_q_once_the_player_marks_it() {
    let mut app = latch_board();
    captured(&mut app, "latch");

    app.blueprints[0].export("q");

    let pins = app.blueprints[0].pins();
    assert_eq!(pins.inputs, lines(&["r", "s"]));
    assert_eq!(
        pins.outputs,
        lines(&["q"]),
        "§19: a player-chosen export is an output pin"
    );
    assert_eq!(
        pins.internals,
        lines(&["qn"]),
        "and only the rest stays inside"
    );
}

#[test]
fn a_captured_edge_detector_pins_out_its_input_and_its_pulse() {
    let mut app = edge_detector(SPOOL);

    let blueprint = captured(&mut app, "edge");

    let pins = blueprint.pins();
    assert_eq!(pins.inputs, lines(&["x"]));
    assert_eq!(pins.outputs, lines(&["pulse"]));
    assert_eq!(pins.internals, lines(&["nx", "xd"]));
    let spooled = blueprint
        .fish
        .iter()
        .find(|fish| fish.name == "D")
        .expect("the delay fish was kept");
    assert_eq!(spooled.design.parts().count(Part::DelaySpool), 1);
}

#[test]
fn a_captured_farm_reads_its_sense_pins_and_its_fire_pin_by_direction() {
    let mut app = board(&["Assay", "Nr", "Harvest", "Reaper"]);
    sense(
        &mut app,
        "Assay",
        Part::AssayScale,
        &[],
        &[("over", "ripe")],
    );
    gate(&mut app, "Nr", &["ripe"], "nripe", COIL);
    gate(&mut app, "Harvest", &["nripe", "nclk"], "harvest", COIL);
    commanded(&mut app, "Reaper", &["/sell fish {richest mutantfish}"]);
    assert!(bot_state(&mut app, "Reaper").wire(Part::CommandModule, FIRE_PIN, "harvest"));

    let pins = captured(&mut app, "reaper").pins();

    assert_eq!(pins.inputs, lines(&["nclk"]), "the clock was left outside");
    assert!(
        pins.internals.contains("ripe"),
        "the sense pin feeds a gate"
    );
    assert!(
        pins.internals.contains("harvest"),
        "the gate fires the module"
    );
    assert!(pins.outputs.is_empty());
}

#[test]
fn a_capture_keeps_only_the_wired_fish_and_where_they_sat() {
    let mut app = board(&["Left", "Right", "Idle"]);
    app.tanks[0].spawn_fish(FishSpecies::Merluza, "Ann".to_string(), &mut rand::rng());
    gate(&mut app, "Left", &["x"], "nx", COIL);
    gate(&mut app, "Right", &["nx"], "y", BARE);
    placed_at(&mut app, "Left", PLACED_X, PLACED_Y);
    placed_at(&mut app, "Right", PLACED_X + 12.0, PLACED_Y + 3.0);
    run(&mut app, "/freeze \"Right\"");
    run(&mut app, "/flip \"Right\"");

    let blueprint = captured(&mut app, "pair");

    let names: Vec<&str> = blueprint.fish.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, vec!["Left", "Right"], "no unwired fish, no wildlife");
    let right = &blueprint.fish[1];
    assert_eq!((right.offset.x, right.offset.y), (12.0, 3.0));
    assert!(right.frozen);
    assert_eq!(
        matches!(right.facing, Direction::Left),
        fish_named(&app, "Right").facing_left(),
        "the arrangement is kept, facing and all"
    );
    assert_eq!(
        (blueprint.fish[0].offset.x, blueprint.fish[0].offset.y),
        (0.0, 0.0)
    );
}

#[test]
fn a_blueprint_is_a_design_and_does_not_follow_the_board_after_capture() {
    let mut app = and_board();
    let blueprint = captured(&mut app, "and");

    gate(&mut app, "And", &["extra"], "q", BARE);
    advance(&mut app, SETTLE_STAGES);

    assert_eq!(blueprint.pins().inputs, lines(&["a", "b"]));
    assert_eq!(
        app.blueprints[0].pins().inputs,
        lines(&["a", "b"]),
        "rewiring the tank never reaches back into a kept blueprint"
    );
}

#[test]
fn a_tank_with_no_circuit_captures_nothing() {
    let mut app = board(&["Idle"]);

    assert!(!app.capture_blueprint("nothing"));
    assert!(app.blueprints.is_empty());
}

const LATCH_NAME: &str = "Latch";
const FIRST_LATCH: &str = "latch1";
const SECOND_LATCH: &str = "latch2";

fn held(app: &App, kind: ConsumableKind) -> u32 {
    app.inventory
        .get(&StockItem::Consumable(kind))
        .copied()
        .unwrap_or(0)
}

fn stock(app: &mut App, kind: ConsumableKind, qty: u32) {
    app.inventory.insert(StockItem::Consumable(kind), qty);
}

fn connect(app: &mut App) {
    run(app, "/give matrixtank");
    assert!(app.is_connected(), "the bench opens on a Matrixtank");
}

fn latch_blueprint_on_an_empty_board() -> App {
    let mut app = latch_board();
    captured(&mut app, LATCH_NAME);
    app.tanks[0].fish.clear();
    connect(&mut app);
    app
}

fn wire(latch: &str, channel: &str) -> String {
    format!("{latch}.{channel}")
}

#[test]
fn selling_by_kind_takes_only_the_blueprint_the_tank_or_the_fish_that_was_named() {
    let mut app = latch_blueprint_on_an_empty_board();
    app.tanks
        .push(Tank::new(LATCH_NAME.to_string(), TankKind::Base, &[]));
    assert!(app.tanks[0].spawn_fish(
        FishSpecies::Merluza,
        LATCH_NAME.to_string(),
        &mut rand::rng()
    ));
    let fish_here = |app: &App| app.tanks[0].fish.iter().any(|f| f.name == LATCH_NAME);
    let purse = app.purse.balance();
    let tanks_before = app.tanks.len();

    run(&mut app, &format!("/sell \"{LATCH_NAME}\""));
    assert_eq!(
        app.purse.balance(),
        purse,
        "a bare name no longer guesses what to sell"
    );

    run(&mut app, "/sell blueprint \"latch\"");
    assert!(app.blueprints.is_empty());
    assert_eq!(
        app.purse.balance(),
        purse + Money::from(CIRCUIT_BLUEPRINT_SELL_PRICE)
    );
    assert!(
        fish_here(&app) && app.tanks.len() == tanks_before,
        "only the blueprint went"
    );

    run(&mut app, &format!("/sell tank {LATCH_NAME}"));
    assert_eq!(app.tanks.len(), tanks_before - 1);
    assert!(
        fish_here(&app),
        "the fish that shared the tank's name still swims"
    );

    run(&mut app, &format!("/sell fish \"{LATCH_NAME}\""));
    assert!(!fish_here(&app));
}

#[test]
fn a_blueprint_that_is_not_owned_sells_for_nothing() {
    let mut app = latch_blueprint_on_an_empty_board();
    let purse = app.purse.balance();

    run(&mut app, "/sell blueprint \"Clock\"");

    assert_eq!(app.purse.balance(), purse);
    assert_eq!(app.blueprints.len(), 1);
}

#[test]
fn two_printed_latches_remember_different_things_because_their_insides_are_namespaced() {
    let mut app = latch_blueprint_on_an_empty_board();
    app.earn(LAB_STAKE, Flow::Godsend);
    stock(&mut app, ConsumableKind::Fabricator, 2);

    assert!(app.print_blueprint(LATCH_NAME));
    assert!(
        app.print_blueprint("latch"),
        "the name is matched in any case"
    );
    set(&mut app, &wire(FIRST_LATCH, "q"), true);
    set(&mut app, &wire(SECOND_LATCH, "qn"), true);
    advance(&mut app, SETTLE_STAGES);

    assert!(level(&app, &wire(FIRST_LATCH, "q")));
    assert!(!level(&app, &wire(FIRST_LATCH, "qn")));
    assert!(!level(&app, &wire(SECOND_LATCH, "q")));
    assert!(
        level(&app, &wire(SECOND_LATCH, "qn")),
        "a wired-OR between the copies would have dragged both to the same state"
    );
    assert_eq!(app.tanks[0].fish.len(), 4);
    assert_eq!(held(&app, ConsumableKind::Fabricator), 0);
}

#[test]
fn printed_latches_share_their_inputs_and_an_exported_output() {
    let mut app = latch_blueprint_on_an_empty_board();
    app.earn(LAB_STAKE, Flow::Godsend);
    app.blueprints[0].export("q");
    stock(&mut app, ConsumableKind::Fabricator, 2);
    assert!(app.print_blueprint(LATCH_NAME));
    assert!(app.print_blueprint(LATCH_NAME));

    set(&mut app, "s", true);
    advance(&mut app, SETTLE_STAGES);

    assert!(
        level(&app, "q"),
        "both copies heard s and drive the one exported q"
    );
    for latch in [FIRST_LATCH, SECOND_LATCH] {
        assert!(!level(&app, &wire(latch, "qn")));
    }
    let drivers = app.tanks[0]
        .fish
        .iter()
        .filter_map(|fish| fish.script())
        .filter(|bot| bot.drives() == Some("q"))
        .count();
    assert_eq!(drivers, 2);
}

#[test]
fn a_print_names_its_fish_after_the_instance_and_keeps_the_arrangement() {
    let mut app = board(&["Left", "Right"]);
    app.earn(LAB_STAKE, Flow::Godsend);
    gate(&mut app, "Left", &["x"], "nx", COIL);
    gate(&mut app, "Right", &["nx"], "y", BARE);
    placed_at(&mut app, "Left", PLACED_X, PLACED_Y);
    placed_at(&mut app, "Right", PLACED_X + 12.0, PLACED_Y + 3.0);
    run(&mut app, "/freeze \"Right\"");
    run(&mut app, "/flip \"Right\"");
    let right_faces_left = fish_named(&app, "Right").facing_left();
    captured(&mut app, "Pair");
    connect(&mut app);
    stock(&mut app, ConsumableKind::Fabricator, 1);

    assert!(app.print_blueprint("Pair"));

    let left = fish_named(&app, "Pair1 Left");
    let right = fish_named(&app, "Pair1 Right");
    let offset = (
        right.position.x - left.position.x,
        right.position.y - left.position.y,
    );
    assert!(
        (offset.0 - 12.0).abs() < PLACEMENT_TOLERANCE
            && (offset.1 - 3.0).abs() < PLACEMENT_TOLERANCE,
        "the board keeps its shape wherever it lands: {offset:?}"
    );
    assert!(right.frozen && !left.frozen);
    assert_eq!(right.facing_left(), right_faces_left);
    assert!(right.is_programmable());
}

#[test]
fn a_print_spends_held_stock_first_and_pays_only_for_what_is_missing() {
    let mut app = latch_blueprint_on_an_empty_board();
    stock(&mut app, ConsumableKind::Fabricator, 1);
    stock(&mut app, ConsumableKind::BlankWafer, 1);
    stock(&mut app, ConsumableKind::Part(Part::InverterCoil), 3);
    let cash = app.purse.balance();

    assert!(app.print_blueprint(LATCH_NAME));

    assert_eq!(
        app.purse.balance(),
        cash - Money::from(ConsumableKind::BlankWafer.buy_price())
    );
    assert_eq!(held(&app, ConsumableKind::BlankWafer), 0);
    assert_eq!(held(&app, ConsumableKind::Part(Part::InverterCoil)), 1);
    assert_eq!(held(&app, ConsumableKind::Fabricator), 0);
}

#[test]
fn a_print_into_a_full_tank_fails_cleanly_and_the_blueprint_survives() {
    let mut app = latch_blueprint_on_an_empty_board();
    let mut rng = rand::rng();
    while app.tanks[0].spawn_fish(FishSpecies::Merluza, "Filler".to_string(), &mut rng) {}
    stock(&mut app, ConsumableKind::Fabricator, 1);
    let cash = app.purse.balance();
    let fish = app.tanks[0].fish.len();

    assert!(!app.print_blueprint(LATCH_NAME));

    assert_eq!(app.tanks[0].fish.len(), fish);
    assert_eq!(app.purse.balance(), cash);
    assert_eq!(held(&app, ConsumableKind::Fabricator), 1);
    assert_eq!(app.blueprints.len(), 1, "the design is never spent");
    let quote = app.print_quote(&app.blueprints[0]);
    assert_eq!(
        quote.err().map(|refusal| refusal.reason()).as_deref(),
        Some("tank full: needs room for 2 fish")
    );
}

#[test]
fn printing_needs_a_fabricator_and_a_blueprint_by_that_name() {
    let mut app = latch_blueprint_on_an_empty_board();
    app.earn(LAB_STAKE, Flow::Godsend);

    assert!(!app.print_blueprint(LATCH_NAME), "no Fabricator");
    stock(&mut app, ConsumableKind::Fabricator, 1);
    assert!(!app.print_blueprint("Nowhere"));
    assert!(app.tanks[0].fish.is_empty());
    assert!(app.print_blueprint(LATCH_NAME));
    assert_eq!(app.blueprints.len(), 1, "the blueprint survives the print");
}

#[test]
fn a_printed_botfish_is_worth_nothing_at_the_till() {
    let mut app = latch_board();
    app.earn(LAB_STAKE, Flow::Godsend);
    captured(&mut app, LATCH_NAME);
    connect(&mut app);
    stock(&mut app, ConsumableKind::Fabricator, 1);
    assert!(app.print_blueprint(LATCH_NAME));

    assert!(
        fish_named(&app, "Q").sell_value() > 0,
        "a fished botfish still sells"
    );
    assert_eq!(fish_named(&app, "Latch1 Q").sell_value(), 0);
}

const HOST: &str = "Chip";
const SECOND_HOST: &str = "Chip2";
const WIDE_HOST: &str = "Big";
const LATCH_FISH: u32 = 2;
const LATCH_COILS: u32 = 2;
const HOST_SPOOLS: u32 = 2;
const THREE_INPUTS: u8 = 8;

fn stock_the_bill(app: &mut App, blueprint: &str) {
    let bill = app
        .blueprints
        .iter()
        .find(|kept| kept.name == blueprint)
        .expect("the blueprint was kept")
        .bill_of_materials(Fabrication::Etch);
    for (kind, qty) in bill {
        let held = held(app, kind);
        stock(app, kind, held + qty);
    }
    let fabricators = held(app, ConsumableKind::Fabricator);
    stock(app, ConsumableKind::Fabricator, fabricators + 1);
}

fn etch_into(app: &mut App, blueprint: &str, host: &str) {
    assert!(
        app.tanks[0].spawn_fish(FishSpecies::Botfish, host.to_string(), &mut rand::rng()),
        "the tank has room for {host}"
    );
    stock_the_bill(app, blueprint);
    assert!(
        app.etch_blueprint(blueprint, host),
        "{blueprint} burns into {host}"
    );
}

fn etched(mut app: App, blueprint: &str) -> App {
    captured(&mut app, blueprint);
    app.tanks[0].fish.clear();
    etch_into(&mut app, blueprint, HOST);
    app
}

#[test]
fn an_etched_xor_reproduces_the_truth_table_of_the_board_it_was_drawn_from() {
    for (a, b) in TRUTH_TABLE {
        let mut app = etched(xor_board(), "Xor");
        set(&mut app, "a", a);
        set(&mut app, "b", b);

        advance(&mut app, SETTLE_STAGES);

        assert_eq!(
            level(&app, "xor"),
            a != b,
            "the chip computes XOR({a}, {b})"
        );
    }
}

#[test]
fn an_etched_board_answers_in_one_stage_where_the_board_took_several() {
    let steps = [(true, false), (true, true), (false, true), (false, false)];
    let mut board = xor_board();
    let mut chip = etched(xor_board(), "Xor");
    let mut board_lagged = false;
    for (a, b) in steps {
        for app in [&mut board, &mut chip] {
            set(app, "a", a);
            set(app, "b", b);
        }

        let original = trace(&mut board, "xor", SETTLE_STAGES);
        let burned = trace(&mut chip, "xor", SETTLE_STAGES);

        board_lagged |= original[0] != (a != b);
        assert_eq!(
            original.last(),
            Some(&(a != b)),
            "the board gets there in the end"
        );
        assert_eq!(
            burned,
            vec![a != b; SETTLE_STAGES],
            "the chip is one fish, so it answers XOR({a}, {b}) the stage after its inputs change"
        );
    }
    assert!(
        board_lagged,
        "the board walked its answer through its depth"
    );
}

#[test]
fn a_chain_etched_in_any_order_settles_in_one_stage() {
    let mut app = board(&["C", "B", "A"]);
    gate(&mut app, "C", &["b"], "c", COIL);
    gate(&mut app, "B", &["a"], "b", COIL);
    gate(&mut app, "A", &["x"], "a", COIL);
    let mut app = etched(app, "Chain");

    for x in [true, false, true] {
        set(&mut app, "x", x);
        step(&mut app);
        assert_eq!(
            level(&app, "c"),
            !x,
            "three inversions of {x}, evaluated driver before listener"
        );
    }
}

fn etched_exporting(mut app: App, blueprint: &str, exports: &str) -> App {
    captured(&mut app, blueprint);
    app.blueprints
        .last_mut()
        .expect("the blueprint was kept")
        .export(exports);
    app.tanks[0].fish.clear();
    etch_into(&mut app, blueprint, HOST);
    app
}

const HELD_STAGES: usize = 20;
const LOOP_CUT_STAGES: usize = 2;

#[test]
fn an_etched_latch_holds_its_bit_stage_after_stage() {
    let mut app = etched_exporting(latch_board(), LATCH_NAME, "q");

    set(&mut app, "r", true);
    step(&mut app);
    assert!(!level(&app, "q"), "reset answers in one stage");
    set(&mut app, "r", false);
    assert!(
        trace(&mut app, "q", HELD_STAGES).iter().all(|&q| !q),
        "and the cleared bit holds"
    );

    set(&mut app, "s", true);
    step(&mut app);
    set(&mut app, "s", false);
    let after_the_pulse = trace(&mut app, "q", HELD_STAGES);

    assert!(
        after_the_pulse[LOOP_CUT_STAGES - 1],
        "a one-stage set pulse lands once it has crossed the edge that closes the loop"
    );
    assert!(
        after_the_pulse[LOOP_CUT_STAGES - 1..].iter().all(|&q| q),
        "and the chip keeps the bit with nothing holding s"
    );
}

#[test]
fn an_etched_latch_driven_both_ways_is_still_the_illegal_state() {
    let mut app = etched_exporting(latch_board(), LATCH_NAME, "q");

    set(&mut app, "s", true);
    set(&mut app, "r", true);
    advance(&mut app, SETTLE_STAGES);

    assert!(!level(&app, "q"));
}

#[test]
fn an_etched_ring_of_three_coils_toggles_every_stage() {
    let ring = ["R0", "R1", "R2"];
    let mut app = board(&ring);
    for (i, name) in ring.iter().enumerate() {
        let listens = format!("c{}", (i + ring.len() - 1) % ring.len());
        gate(&mut app, name, &[&listens], &format!("c{i}"), COIL);
    }
    let mut app = etched_exporting(app, "Ring", "c0");

    let levels = trace(&mut app, "c0", OSCILLATOR_STAGES);

    assert_period(&levels, 2);
}

#[test]
fn spools_still_count_stages_inside_a_chip() {
    for spools in 1..=2u32 {
        let mut app = board(&["Osc"]);
        let mut parts = vec![Part::InverterCoil];
        parts.extend(std::iter::repeat_n(Part::DelaySpool, spools as usize));
        gate(&mut app, "Osc", &["q"], "q", &parts);
        let mut app = etched_exporting(app, "Slow", "q");

        let levels = trace(&mut app, "q", OSCILLATOR_STAGES);

        assert_period(&levels, 2 * (spools as usize + 1));
    }
}

#[test]
fn an_etched_edge_detector_fires_for_exactly_one_stage_and_never_without_its_spool() {
    for (delay, pulses) in [(SPOOL, 1), (BARE, 0)] {
        let mut app = etched(edge_detector(delay), "Edge");
        advance(&mut app, WARM_UP_STAGES);

        set(&mut app, "x", true);
        let levels = trace(&mut app, "pulse", EDGE_WATCHED_STAGES);

        assert_eq!(levels.iter().filter(|&&high| high).count(), pulses);
    }
}

const FULL_ADDER: &str = "Full Adder";
const FULL_ADDER_FISH: [&str; 9] = ["N1", "N2", "N3", "N4", "N5", "N6", "N7", "Sum", "Cout"];
const FULL_ADDER_DEPTH: usize = 6;
const THREE_BIT_INPUTS: u32 = 8;
const ADDER_BITS: u8 = 4;
const ADDER_OPERANDS: u32 = 1 << ADDER_BITS;
const CARRY_BITS: u32 = 2;
const RIPPLE_SETTLE_STAGES: usize = ADDER_BITS as usize + 1;

fn full_adder_board() -> App {
    let mut app = board(&FULL_ADDER_FISH);
    gate(&mut app, "N1", &["a", "b"], "n1", COIL);
    gate(&mut app, "N2", &["a", "n1"], "n2", COIL);
    gate(&mut app, "N3", &["b", "n1"], "n3", COIL);
    gate(&mut app, "N4", &["n2", "n3"], "n4", COIL);
    gate(&mut app, "N5", &["n4", "cin"], "n5", COIL);
    gate(&mut app, "N6", &["n4", "n5"], "n6", COIL);
    gate(&mut app, "N7", &["cin", "n5"], "n7", COIL);
    gate(&mut app, "Sum", &["n6", "n7"], "sum", COIL);
    gate(&mut app, "Cout", &["n1", "n5"], "cout", COIL);
    app
}

fn bit(value: u32, index: u8) -> bool {
    value >> index & 1 == 1
}

fn put_bus(app: &mut App, base: &str, width: u8, value: u32) {
    for index in 0..width {
        set(app, &format!("{base}{index}"), bit(value, index));
    }
}

#[test]
fn nine_coiled_fish_add_every_combination_of_three_bits() {
    let mut app = full_adder_board();

    for inputs in 0..THREE_BIT_INPUTS {
        let (a, b, cin) = (bit(inputs, 0), bit(inputs, 1), bit(inputs, 2));
        set(&mut app, "a", a);
        set(&mut app, "b", b);
        set(&mut app, "cin", cin);
        advance(&mut app, FULL_ADDER_DEPTH + 1);

        let total = a as u8 + b as u8 + cin as u8;
        assert_eq!(
            level(&app, "sum"),
            total & 1 == 1,
            "sum of {a} + {b} + {cin}"
        );
        assert_eq!(level(&app, "cout"), total > 1, "carry of {a} + {b} + {cin}");
    }
}

#[test]
fn a_captured_full_adder_pins_out_a_b_and_carry_in_to_sum_and_carry_out() {
    let mut app = full_adder_board();

    let pins = captured(&mut app, FULL_ADDER).pins();

    assert_eq!(pins.inputs, lines(&["a", "b", "cin"]));
    assert_eq!(pins.outputs, lines(&["cout", "sum"]));
}

fn adder_host(index: u8) -> String {
    format!("Adder{index}")
}

fn four_bit_adder() -> App {
    let mut app = full_adder_board();
    captured(&mut app, FULL_ADDER);
    app.tanks[0].fish.clear();
    for index in 0..ADDER_BITS {
        let host = adder_host(index);
        etch_into(&mut app, FULL_ADDER, &host);
        let chip = bot_state(&mut app, &host);
        let wiring = [
            ("a", format!("a{index}")),
            ("b", format!("b{index}")),
            ("cin", format!("c{index}")),
            ("sum", format!("s{index}")),
            ("cout", format!("c{}", index + 1)),
        ];
        for (pin, channel) in wiring {
            assert!(
                chip.wire(PinOwner::Body, pin, &channel),
                "{host} rewires {pin} to {channel}"
            );
        }
    }
    app
}

fn adder_total(app: &App) -> u32 {
    bus_value(app, "s", ADDER_BITS) + ((level(app, &format!("c{ADDER_BITS}")) as u32) << ADDER_BITS)
}

#[test]
fn four_etched_full_adders_chained_carry_to_carry_add_every_pair_of_nibbles() {
    let mut app = four_bit_adder();
    assert_eq!(
        app.tanks[0].fish.len(),
        ADDER_BITS as usize,
        "36 gates live in four fish"
    );

    for a in 0..ADDER_OPERANDS {
        for b in 0..ADDER_OPERANDS {
            for carry_in in 0..CARRY_BITS {
                put_bus(&mut app, "a", ADDER_BITS, a);
                put_bus(&mut app, "b", ADDER_BITS, b);
                set(&mut app, "c0", carry_in == 1);
                advance(&mut app, RIPPLE_SETTLE_STAGES);

                assert_eq!(
                    adder_total(&app),
                    a + b + carry_in,
                    "{a} + {b} + {carry_in}"
                );
            }
        }
    }
}

#[test]
fn a_carry_ripples_through_four_etched_adders_one_fish_per_stage() {
    let mut app = four_bit_adder();
    put_bus(&mut app, "a", ADDER_BITS, ADDER_OPERANDS - 1);
    put_bus(&mut app, "b", ADDER_BITS, 0);
    advance(&mut app, RIPPLE_SETTLE_STAGES);
    assert_eq!(adder_total(&app), ADDER_OPERANDS - 1);

    set(&mut app, "c0", true);
    let carries: Vec<Vec<bool>> = (0..RIPPLE_SETTLE_STAGES)
        .map(|_| {
            step(&mut app);
            (1..=ADDER_BITS)
                .map(|index| level(&app, &format!("c{index}")))
                .collect()
        })
        .collect();

    for index in 0..ADDER_BITS as usize {
        let rose = carries.iter().position(|stage| stage[index]);
        assert_eq!(
            rose,
            Some(index),
            "carry {} rises on stage {}",
            index + 1,
            index + 1
        );
    }
    assert_eq!(
        adder_total(&app),
        ADDER_OPERANDS,
        "15 + 0 + 1 carries all the way out"
    );
}

const REGISTER_BITS: u8 = 8;
const REGISTER_SETTLE_STAGES: usize = 8;
const REGISTER_HELD_TICKS: usize = 60;
const FIRST_WORD: u32 = 0b1010_0110;
const SECOND_WORD: u32 = 0b0101_1001;

fn register_board() -> App {
    let names: Vec<String> = std::iter::once("Nwe".to_string())
        .chain(
            (0..REGISTER_BITS)
                .flat_map(|index| ["R", "S", "Q", "Qn"].map(|role| format!("{role}{index}"))),
        )
        .collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut app = board(&refs);
    gate(&mut app, "Nwe", &["we"], "nwe", COIL);
    for index in 0..REGISTER_BITS {
        let [d, r, s, q, qn] = ["d", "r", "s", "q", "qn"].map(|line| format!("{line}{index}"));
        gate(&mut app, &format!("R{index}"), &[&d, "nwe"], &r, COIL);
        gate(&mut app, &format!("S{index}"), &[&r, "nwe"], &s, COIL);
        gate(&mut app, &format!("Q{index}"), &[&r, &qn], &q, COIL);
        gate(&mut app, &format!("Qn{index}"), &[&s, &q], &qn, COIL);
    }
    app
}

fn write_word(app: &mut App, word: u32) {
    put_bus(app, "d", REGISTER_BITS, word);
    set(app, "we", true);
    advance(app, REGISTER_SETTLE_STAGES);
    set(app, "we", false);
    advance(app, REGISTER_SETTLE_STAGES);
}

fn assert_register_holds_what_was_written(mut app: App) {
    write_word(&mut app, FIRST_WORD);
    put_bus(&mut app, "d", REGISTER_BITS, SECOND_WORD);
    tick_n(&mut app, REGISTER_HELD_TICKS);
    assert_eq!(
        bus_value(&app, "q", REGISTER_BITS),
        FIRST_WORD,
        "with write enable low the register ignores its inputs, tick after tick"
    );

    set(&mut app, "we", true);
    advance(&mut app, REGISTER_SETTLE_STAGES);
    assert_eq!(
        bus_value(&app, "q", REGISTER_BITS),
        SECOND_WORD,
        "while write enable is high it copies its inputs"
    );
    set(&mut app, "we", false);
    advance(&mut app, REGISTER_SETTLE_STAGES);
    put_bus(&mut app, "d", REGISTER_BITS, FIRST_WORD);
    tick_n(&mut app, REGISTER_HELD_TICKS);
    assert_eq!(
        bus_value(&app, "q", REGISTER_BITS),
        SECOND_WORD,
        "inputs changed only after write enable has closed, which is the board's hold time"
    );
}

#[test]
fn changing_a_boards_inputs_as_write_enable_drops_races_its_latches() {
    let mut app = register_board();
    write_word(&mut app, FIRST_WORD);
    set(&mut app, "we", true);
    put_bus(&mut app, "d", REGISTER_BITS, SECOND_WORD);
    advance(&mut app, REGISTER_SETTLE_STAGES);

    set(&mut app, "we", false);
    put_bus(&mut app, "d", REGISTER_BITS, FIRST_WORD);
    advance(&mut app, REGISTER_SETTLE_STAGES);

    assert_ne!(
        bus_value(&app, "q", REGISTER_BITS),
        SECOND_WORD,
        "the new inputs reach the gates a stage before the closing enable does"
    );
}

#[test]
fn eight_latches_hold_a_written_word_across_ticks() {
    assert_register_holds_what_was_written(register_board());
}

#[test]
fn an_etched_register_holds_a_written_word_across_ticks() {
    let exports: Vec<String> = (0..REGISTER_BITS)
        .map(|index| format!("q{index}"))
        .collect();
    let app = etched_exporting(register_board(), "Register", &exports.join(", "));
    assert_eq!(app.tanks[0].fish.len(), 1);

    assert_register_holds_what_was_written(app);
}

const THOUSAND_GATES: usize = 1_000;
const TIMED_STAGES: u32 = 10;
const TIMED_BATCHES: usize = 5;
const FRAME_COST_LIMIT: Duration = Duration::from_millis(1);

fn thousand_gate_chip() -> Chip {
    let wire = |index: usize| format!("n{index}");
    let board = (0..THOUSAND_GATES)
        .map(|index| {
            let mut gate = BotfishState::new();
            gate.listen(&wire(index));
            gate.drive(&wire(index + 1));
            gate.install(Part::InverterCoil);
            gate
        })
        .collect();
    let inputs = BTreeSet::from([wire(0)]);
    let outputs = BTreeSet::from([wire(THOUSAND_GATES)]);
    Chip::new("Thousand".to_string(), board, inputs, outputs)
}

#[test]
fn a_chip_of_a_thousand_gates_settles_in_one_stage_without_a_frame_cost() {
    let mut app = board(&[HOST]);
    bot_state(&mut app, HOST).etch(thousand_gate_chip());
    let last = format!("n{THOUSAND_GATES}");

    for input in [true, false, true] {
        set(&mut app, "n0", input);
        step(&mut app);
        assert_eq!(
            level(&app, &last),
            input,
            "a thousand inversions of {input} in a single stage"
        );
    }

    let per_stage = (0..TIMED_BATCHES)
        .map(|_| {
            let started = Instant::now();
            advance(&mut app, TIMED_STAGES as usize);
            started.elapsed() / TIMED_STAGES
        })
        .min()
        .expect("at least one batch was timed");

    assert!(
        per_stage < FRAME_COST_LIMIT,
        "a thousand-gate stage cost {per_stage:?}"
    );
}
#[test]
fn a_chip_keeps_its_insides_out_of_the_tank() {
    let mut app = etched(and_board(), "And");
    set(&mut app, "a", true);
    set(&mut app, "b", true);

    advance(&mut app, SETTLE_STAGES);

    assert!(level(&app, "q"));
    let wires: Vec<&str> = app.tanks[0].channels.names().collect();
    assert_eq!(
        wires,
        vec!["a", "b", "q"],
        "na and nb live inside the chip and never touch the board"
    );
}

#[test]
fn an_etched_latch_remembers_a_pulse_through_its_exported_output() {
    let mut app = latch_board();
    captured(&mut app, LATCH_NAME);
    app.blueprints[0].export("q");
    app.tanks[0].fish.clear();
    etch_into(&mut app, LATCH_NAME, HOST);

    set(&mut app, "s", true);
    advance(&mut app, SETTLE_STAGES);
    assert!(level(&app, "q"), "setting raises q");

    set(&mut app, "s", false);
    advance(&mut app, SETTLE_STAGES);
    assert!(level(&app, "q"), "and the chip holds it after s drops");

    set(&mut app, "r", true);
    advance(&mut app, SETTLE_STAGES);
    assert!(!level(&app, "q"), "resetting clears it");
    assert!(
        !app.tanks[0].channels.contains("qn"),
        "the complement never left the chip"
    );
}

#[test]
fn an_etched_ring_oscillator_blinks_the_antenna_of_the_fish_it_lives_in() {
    let mut ring = board(&["Osc"]);
    gate(&mut ring, "Osc", &["q"], "q", COIL);
    let mut app = etched(ring, "Clock");

    let antenna: Vec<bool> = (0..OSCILLATOR_STAGES)
        .map(|_| {
            step(&mut app);
            probe(&app, HOST).1
        })
        .collect();

    assert_period(&antenna, 2);
    assert!(
        app.tanks[0].channels.is_empty(),
        "a chip with no pins shows its work only on its antenna"
    );
}

#[test]
fn an_etched_fish_can_be_captured_and_etched_again() {
    let mut app = etched(and_board(), "And");
    etch_into(&mut app, "And", SECOND_HOST);
    let second = bot_state(&mut app, SECOND_HOST);
    assert!(second.wire(PinOwner::Body, "a", "q"));
    assert!(second.wire(PinOwner::Body, "b", "c"));
    assert!(second.wire(PinOwner::Body, "q", "q2"));

    let pins = captured(&mut app, "And3").pins();
    assert_eq!(pins.inputs, lines(&["a", "b", "c"]));
    assert_eq!(pins.outputs, lines(&["q2"]));
    app.tanks[0].fish.clear();
    app.tanks[0].channels = Default::default();
    etch_into(&mut app, "And3", WIDE_HOST);

    for bits in 0..THREE_INPUTS {
        let (a, b, c) = (bits & 1 == 1, bits & 2 == 2, bits & 4 == 4);
        set(&mut app, "a", a);
        set(&mut app, "b", b);
        set(&mut app, "c", c);

        step(&mut app);

        assert_eq!(
            level(&app, "q2"),
            a && b && c,
            "a chip of two chips is a three-input AND({a}, {b}, {c}) that still costs one stage"
        );
    }
}

#[test]
fn etching_needs_a_fabricator_and_the_blueprint_survives() {
    let mut app = latch_board();
    app.earn(LAB_STAKE, Flow::Godsend);
    captured(&mut app, LATCH_NAME);
    app.tanks[0].fish.clear();
    connect(&mut app);
    assert!(app.tanks[0].spawn_fish(FishSpecies::Botfish, HOST.to_string(), &mut rand::rng()));

    assert!(!app.etch_blueprint(LATCH_NAME, HOST), "no Fabricator");
    assert!(
        !fish_named(&app, HOST).is_wired(),
        "a refused etch touches nothing"
    );
    assert_eq!(
        app.etch_quote(&app.blueprints[0])
            .err()
            .map(|refusal| refusal.reason())
            .as_deref(),
        Some("needs a Fabricator")
    );

    stock(&mut app, ConsumableKind::Fabricator, 1);
    assert!(!app.etch_blueprint("Nowhere", HOST));
    assert!(!app.etch_blueprint(LATCH_NAME, "Nobody"));
    assert!(app.etch_blueprint(LATCH_NAME, HOST));

    assert_eq!(held(&app, ConsumableKind::Fabricator), 0);
    assert_eq!(app.blueprints.len(), 1, "the blueprint survives the etch");
}

#[test]
fn an_etch_bills_two_wafers_a_fish_and_hands_back_the_parts_it_replaced() {
    let mut app = latch_board();
    captured(&mut app, LATCH_NAME);
    app.tanks[0].fish.clear();
    assert!(app.tanks[0].spawn_fish(FishSpecies::Botfish, HOST.to_string(), &mut rand::rng()));
    let host = bot_state(&mut app, HOST);
    host.listen("old");
    for _ in 0..HOST_SPOOLS {
        assert!(host.install(Part::DelaySpool));
    }
    stock(&mut app, ConsumableKind::Fabricator, 1);
    stock(&mut app, ConsumableKind::BlankWafer, LATCH_FISH * 2);
    stock(
        &mut app,
        ConsumableKind::Part(Part::InverterCoil),
        LATCH_COILS,
    );
    let cash = app.purse.balance();

    assert!(app.etch_blueprint(LATCH_NAME, HOST));

    assert_eq!(
        app.purse.balance(),
        cash,
        "everything the etch needed was held"
    );
    assert_eq!(held(&app, ConsumableKind::BlankWafer), 0);
    assert_eq!(held(&app, ConsumableKind::Part(Part::InverterCoil)), 0);
    assert_eq!(
        held(&app, ConsumableKind::Part(Part::DelaySpool)),
        HOST_SPOOLS,
        "the spools the chip replaced go back in the inventory"
    );
    let bot = fish_named(&app, HOST).script().expect("still a botfish");
    assert!(!bot.hears("old"), "the old wiring is burned away");
    assert!(bot.parts().is_empty());
    assert_eq!(bot.chip().map(|chip| chip.name()), Some(LATCH_NAME));
    assert!(
        fish_named(&app, HOST).sell_value() > 0,
        "etching makes no new fish, so the fish keeps its price"
    );
}

#[test]
fn a_script_inside_a_chip_runs_as_the_fish_it_was_burned_into() {
    let mut farm = broke_board(&["Or", "Bot"]);
    gate(&mut farm, "Or", &[GATE_INPUT], FIRE_CHANNEL, BARE);
    commanded(&mut farm, "Bot", &["/freeze \"Bot\"", "/say \"glup glup\""]);
    let mut app = etched(farm, "Greeter");

    set(&mut app, GATE_INPUT, true);
    tick_n(&mut app, SCRIPT_TICKS * 3);

    assert!(
        frozen(&app, HOST),
        "a line naming a board fish now names the fish the board became"
    );
    assert_eq!(speech_of(&app, HOST), Some("glup glup".to_string()));
}

#[test]
fn a_run_trigger_inside_a_chip_answers_to_the_host_name() {
    let mut ear = broke_board(&["Ear"]);
    bot_state(&mut ear, "Ear").program("/run Ear".to_string(), vec!["/give cash".to_string()]);
    let mut app = etched(ear, "Ears");

    run(&mut app, &format!("/run {HOST}"));
    tick_n(&mut app, SCRIPT_TICKS);

    assert_eq!(app.purse.balance(), Money::from(GIVE_RESOURCE_AMOUNT));
}

#[test]
fn the_etch_command_burns_a_blueprint_by_name() {
    let mut app = xor_board();
    captured(&mut app, "Xor");
    app.tanks[0].fish.clear();
    assert!(app.tanks[0].spawn_fish(FishSpecies::Botfish, HOST.to_string(), &mut rand::rng()));
    stock_the_bill(&mut app, "Xor");

    run(&mut app, &format!("/etch xor {HOST}"));

    assert!(
        fish_named(&app, HOST)
            .script()
            .and_then(BotfishState::chip)
            .is_some()
    );
}

const KEYBOARD: &str = "kbd";
const KEYBOARD_WIDTH: u8 = 8;
const STROBE: &str = "clk";
const COCHLEA_WIRING: &[(&str, &str)] = &[
    ("char", KEYBOARD),
    ("strobe", STROBE),
    ("ready", "rdy"),
    ("done", "fin"),
];
const TYPED_SUM: &str = "6 + 4";
const TYPED_SUM_ASCII: [u32; 5] = [0x36, 0x20, 0x2B, 0x20, 0x34];

fn cochlea_board() -> App {
    let mut app = board(&["Ear"]);
    sense(&mut app, "Ear", Part::Cochlea, &[], COCHLEA_WIRING);
    app
}

fn strobed(app: &mut App) -> u32 {
    set(app, STROBE, false);
    step(app);
    set(app, STROBE, true);
    step(app);
    bus_value(app, KEYBOARD, KEYBOARD_WIDTH)
}

#[test]
fn a_cochlea_clocks_a_typed_line_onto_its_bus_one_strobe_per_character() {
    let mut app = cochlea_board();
    advance(&mut app, SETTLE_STAGES);
    assert!(!level(&app, "rdy"), "nothing has been typed");

    run(&mut app, TYPED_SUM);
    step(&mut app);
    assert!(level(&app, "rdy"), "the line is latched and waiting");
    assert_eq!(bus_value(&app, KEYBOARD, KEYBOARD_WIDTH), 0);

    let mut clocked = Vec::new();
    for _ in TYPED_SUM_ASCII {
        assert!(!level(&app, "fin"), "done waits for the last character");
        clocked.push(strobed(&mut app));
    }

    assert_eq!(
        clocked,
        TYPED_SUM_ASCII.to_vec(),
        "the raw ASCII of every key, the spaces included: nothing is parsed"
    );
    assert!(!level(&app, "rdy"));
    assert!(level(&app, "fin"));
}

#[test]
fn a_strobe_held_high_moves_the_line_by_one_character() {
    let mut app = cochlea_board();
    run(&mut app, "ab");

    set(&mut app, STROBE, true);
    advance(&mut app, SETTLE_STAGES);

    assert_eq!(bus_value(&app, KEYBOARD, KEYBOARD_WIDTH), u32::from(b'a'));
    assert!(level(&app, "rdy"), "the b is still waiting for its strobe");
}

#[test]
fn a_command_is_not_a_line_but_the_words_of_a_say_are() {
    let mut app = cochlea_board();

    run(&mut app, "/feed 1");
    step(&mut app);
    assert!(!level(&app, "rdy"), "a command is obeyed, never heard");

    run(&mut app, &format!("/say \"{TYPED_SUM}\""));
    step(&mut app);
    assert!(level(&app, "rdy"));
    let clocked: Vec<u32> = TYPED_SUM_ASCII.iter().map(|_| strobed(&mut app)).collect();
    assert_eq!(
        clocked,
        TYPED_SUM_ASCII.to_vec(),
        "the spoken words, unquoted"
    );
}

#[test]
fn a_cochlea_hears_a_say_only_in_its_own_tank_and_a_typed_line_everywhere() {
    let mut app = cochlea_board();
    app.tanks
        .push(Tank::new("Next".to_string(), TankKind::Base, &[]));
    app.current_tank = 1;

    run(&mut app, "/say \"far away\"");
    step(&mut app);
    assert!(
        !level(&app, "rdy"),
        "a say is heard in the room it was said in"
    );

    run(&mut app, "hello");
    step(&mut app);
    assert!(level(&app, "rdy"), "a bare typed line reaches every tank");
}

#[test]
fn a_second_line_replaces_the_one_being_read() {
    let mut app = cochlea_board();
    run(&mut app, "abc");
    strobed(&mut app);

    run(&mut app, "z");
    step(&mut app);
    assert_eq!(bus_value(&app, KEYBOARD, KEYBOARD_WIDTH), 0);

    assert_eq!(strobed(&mut app), u32::from(b'z'));
    assert!(level(&app, "fin"));
}

fn self_clocked_reader() -> App {
    let mut app = board(&["Ear"]);
    gate(&mut app, "Ear", &[STROBE, "fin"], STROBE, COIL);
    sense(&mut app, "Ear", Part::Cochlea, &[], COCHLEA_WIRING);
    app
}

const READER_STAGES: usize = 40;

#[test]
fn a_cochlea_that_clocks_itself_reads_its_line_to_the_end_and_stops() {
    let mut app = self_clocked_reader();
    assert!(
        trace(&mut app, STROBE, SETTLE_STAGES)
            .windows(2)
            .any(|pair| pair[0] != pair[1]),
        "with nothing to read, the coil rings on"
    );

    run(&mut app, "hi");
    let mut seen = Vec::new();
    for _ in 0..READER_STAGES {
        step(&mut app);
        let byte = bus_value(&app, KEYBOARD, KEYBOARD_WIDTH);
        if seen.last() != Some(&byte) {
            seen.push(byte);
        }
    }

    assert_eq!(
        seen,
        vec![0, u32::from(b'h'), u32::from(b'i')],
        "nothing until the first strobe, then one character per tick of its own clock"
    );
    assert!(level(&app, "fin"));
    assert!(
        trace(&mut app, STROBE, SETTLE_STAGES)
            .iter()
            .all(|&high| !high),
        "done holds the coil's input high, so the clock stops for good"
    );
}

const WRITE: &str = "wr";
const PANEL_WIRING: &[(&str, &str)] = &[("char", KEYBOARD), ("write", WRITE)];
const EAR_WIRING: &[(&str, &str)] = &[("char", KEYBOARD), ("strobe", STROBE), ("done", "fin")];

fn echo_board() -> App {
    let mut app = board(&["Ear", "Lcd"]);
    sense(&mut app, "Ear", Part::Cochlea, &[], EAR_WIRING);
    gate(&mut app, "Lcd", &[STROBE], WRITE, BARE);
    sense(&mut app, "Lcd", Part::GlyphPanel, &[], PANEL_WIRING);
    app
}

fn start_the_clock(app: &mut App) {
    gate(app, "Ear", &[STROBE, "fin"], STROBE, COIL);
}

fn panel_rows(app: &App, name: &str) -> Vec<String> {
    fish_named(app, name)
        .script()
        .map(BotfishState::displays)
        .unwrap_or_default()
        .first()
        .map(|display| display.rows().to_vec())
        .unwrap_or_default()
        .iter()
        .map(|row| row.trim_end().to_string())
        .collect()
}

#[test]
fn a_typed_line_is_echoed_onto_a_glyph_panel_by_a_two_fish_circuit() {
    let mut app = echo_board();
    advance(&mut app, SETTLE_STAGES);
    assert!(
        panel_rows(&app, "Lcd").is_empty(),
        "nothing written, no panel"
    );

    run(&mut app, "hi");
    start_the_clock(&mut app);
    advance(&mut app, READER_STAGES);

    assert_eq!(
        panel_rows(&app, "Lcd"),
        vec!["hi".to_string(), String::new()],
        "every character, and only the characters: the bare fish delays the write by the stage the byte needs"
    );
    assert!(level(&app, "fin"));
    assert!(
        trace(&mut app, STROBE, SETTLE_STAGES)
            .iter()
            .all(|&high| !high),
        "done stopped the clock"
    );
    assert_eq!(
        panel_rows(&app, "Lcd")[0],
        "hi",
        "with every wire low the panel still shows its line"
    );
}

#[test]
fn a_second_line_restarts_the_echo_and_lands_after_the_first() {
    let mut app = echo_board();
    run(&mut app, "hi");
    start_the_clock(&mut app);
    advance(&mut app, READER_STAGES);

    run(&mut app, "yo");
    advance(&mut app, READER_STAGES);

    assert_eq!(panel_rows(&app, "Lcd")[0], "hiyo", "the cursor carried on");
}

#[test]
fn clocking_before_anything_was_typed_writes_the_empty_bus_as_undrawable_cells() {
    let mut app = echo_board();
    start_the_clock(&mut app);
    advance(&mut app, SETTLE_STAGES);
    let rows = panel_rows(&app, "Lcd");
    assert!(
        rows[0].chars().all(|cell| cell == '\u{FFFD}'),
        "a free-running clock writes NULs: specify the input, then run ({rows:?})"
    );
}

fn letter_on_the_bus(app: &mut App, letter: u8) {
    for bit in 0..KEYBOARD_WIDTH {
        set(app, &format!("{KEYBOARD}{bit}"), letter >> bit & 1 == 1);
    }
}

#[test]
fn a_glyph_panel_never_speaks_so_no_ear_ever_hears_it() {
    let mut app = board(&["Lcd", "Ear"]);
    sense(&mut app, "Lcd", Part::GlyphPanel, &[], PANEL_WIRING);
    sense(&mut app, "Ear", Part::Cochlea, &[], &[("ready", "rdy")]);
    letter_on_the_bus(&mut app, b'A');

    for _ in 0..3 {
        set(&mut app, WRITE, false);
        step(&mut app);
        set(&mut app, WRITE, true);
        step(&mut app);
    }

    assert_eq!(panel_rows(&app, "Lcd")[0], "AAA");
    assert!(!level(&app, "rdy"), "the Cochlea beside it heard nothing");
    assert!(
        app.tanks[0].fish.iter().all(|fish| fish.speech.is_none()),
        "a panel is a display, not an utterance"
    );
}

#[test]
fn clear_blanks_the_panel_on_the_board() {
    let mut app = board(&["Lcd"]);
    sense(
        &mut app,
        "Lcd",
        Part::GlyphPanel,
        &[],
        &[("char", KEYBOARD), ("write", WRITE), ("clear", "cls")],
    );
    letter_on_the_bus(&mut app, b'Z');
    set(&mut app, WRITE, true);
    step(&mut app);
    assert_eq!(panel_rows(&app, "Lcd")[0], "Z");

    set(&mut app, "cls", true);
    step(&mut app);

    assert!(
        panel_rows(&app, "Lcd").is_empty(),
        "a blank panel has no bubble"
    );
}

#[test]
fn a_panel_written_on_the_strobes_own_edge_reads_the_byte_before() {
    let mut app = board(&["Ear", "Lcd"]);
    sense(&mut app, "Ear", Part::Cochlea, &[], EAR_WIRING);
    sense(
        &mut app,
        "Lcd",
        Part::GlyphPanel,
        &[],
        &[("char", KEYBOARD), ("write", STROBE)],
    );

    run(&mut app, "hi");
    start_the_clock(&mut app);
    advance(&mut app, READER_STAGES);

    assert_eq!(
        panel_rows(&app, "Lcd")[0],
        "\u{FFFD}h",
        "the byte lands on the bus the stage its strobe rises, so a write on that edge is one behind"
    );
}

const WORD_BUS: &str = "q";
const WORD_WIDTH: u8 = 8;
const RAIL: &str = "d6";
const AT_SIGN: u32 = b'@' as u32;
const STACK_READ: &[(&str, &str)] = &[("addr", "d"), ("data_in", "d"), ("data_out", WORD_BUS)];

fn memory_board(write: Option<&str>) -> App {
    let mut app = board(&["Ram", "Lcd"]);
    gate(&mut app, "Ram", &[], RAIL, COIL);
    sense(&mut app, "Ram", Part::CoreStack, &[], STACK_READ);
    if let Some(strobe) = write {
        assert!(bot_state(&mut app, "Ram").wire(Part::CoreStack, "write", strobe));
    }
    gate(&mut app, "Lcd", &[RAIL], WRITE, BARE);
    sense(
        &mut app,
        "Lcd",
        Part::GlyphPanel,
        &[],
        &[("char", WORD_BUS), ("write", WRITE)],
    );
    app
}

#[test]
fn a_rail_writes_an_at_sign_at_address_sixty_four_and_the_stack_reads_it_back() {
    let mut app = memory_board(Some(RAIL));
    advance(&mut app, SETTLE_STAGES);
    assert_eq!(
        bus_value(&app, WORD_BUS, WORD_WIDTH),
        AT_SIGN,
        "the rail is bit 6 of d, so it is both the address and the word, and its rise is the write"
    );
    assert_eq!(
        panel_rows(&app, "Lcd")[0],
        "@",
        "and a panel reads data_out"
    );
}

#[test]
fn a_stack_whose_write_is_never_wired_holds_nothing_so_the_panel_shows_an_empty_word() {
    let mut app = memory_board(None);
    advance(&mut app, SETTLE_STAGES);
    assert_eq!(bus_value(&app, WORD_BUS, WORD_WIDTH), 0);
    assert_eq!(
        panel_rows(&app, "Lcd")[0],
        "\u{FFFD}",
        "word 64 was never written"
    );
}

fn address_board(address_follows_the_strobe: bool) -> App {
    let mut app = board(&["Ram", "Adr"]);
    sense(
        &mut app,
        "Ram",
        Part::CoreStack,
        &[],
        &[("addr", "a"), ("data_in", "d"), ("write", STROBE)],
    );
    if address_follows_the_strobe {
        gate(&mut app, "Adr", &[STROBE], "a0", BARE);
    }
    for bit in 0..WORD_WIDTH {
        set(&mut app, &format!("d{bit}"), bit == 0);
    }
    app
}

fn word_at(app: &mut App, addr: u32) -> u32 {
    bot_state(app, "Ram").wire(Part::CoreStack, "data_out", WORD_BUS);
    bot_state(app, "Adr").unwire(PinOwner::Body, "out");
    for bit in 0..WORD_WIDTH {
        set(app, &format!("a{bit}"), addr >> bit & 1 == 1);
    }
    step(app);
    bus_value(app, WORD_BUS, WORD_WIDTH)
}

#[test]
fn an_address_computed_from_the_write_strobe_arrives_a_stage_late_so_the_old_address_is_written() {
    let mut app = address_board(true);
    set(&mut app, STROBE, true);
    advance(&mut app, SETTLE_STAGES);
    assert!(level(&app, "a0"), "the address fish did raise address 1");

    assert_eq!(
        word_at(&mut app, 0),
        1,
        "the strobe rose while the address still read 0: hold the address a stage before you write"
    );
    assert_eq!(word_at(&mut app, 1), 0);
}

#[test]
fn an_address_held_a_stage_before_the_strobe_is_the_address_written() {
    let mut app = address_board(false);
    set(&mut app, "a0", true);
    step(&mut app);
    set(&mut app, STROBE, true);
    advance(&mut app, SETTLE_STAGES);

    assert_eq!(word_at(&mut app, 1), 1);
    assert_eq!(word_at(&mut app, 0), 0);
}

const MEMORY_FISH: usize = 40;
const STACK_PINS: [&str; 4] = ["addr", "data_in", "data_out", "write"];

fn tank_of_stacks() -> App {
    let mut app = board(&[]);
    let mut rng = rand::rng();
    for index in 0..MEMORY_FISH {
        let name = format!("Ram{index}");
        assert!(app.tanks[0].spawn_fish(FishSpecies::Botfish, name.clone(), &mut rng));
        let bot = bot_state(&mut app, &name);
        assert!(bot.install(Part::CoreStack));
        for pin in STACK_PINS {
            assert!(bot.wire(Part::CoreStack, pin, &format!("{pin} {index}")));
        }
    }
    app
}

#[test]
fn a_tank_of_core_stacks_stays_inside_its_stage_budget() {
    let mut app = tank_of_stacks();
    run(&mut app, &format!("/clock {STAGES_PER_TICK_MAX}"));

    let started = Instant::now();
    let advanced = app.tick_fabric();
    let elapsed = started.elapsed();

    assert!(advanced > 0, "the fabric still advanced");
    assert!(
        elapsed < RESPONSIVE_LIMIT,
        "a full tick with a tank of memories took {elapsed:?}"
    );
}

const SCREEN_RAIL: &str = "d0";
const SCREEN_WIDTH: &str = "8";
const DOT_AND_BYTE: &str = "⠊⠀⠀⠀";

fn surfaces(app: &App, name: &str) -> Vec<String> {
    fish_named(app, name)
        .script()
        .map(BotfishState::displays)
        .unwrap_or_default()
        .iter()
        .filter(|display| matches!(display, Display::Body(_)))
        .flat_map(|display| display.rows().to_vec())
        .collect()
}

fn screen_board(pins: &[(&str, &str)]) -> App {
    let mut app = board(&["Crt"]);
    gate(&mut app, "Crt", &[], SCREEN_RAIL, COIL);
    sense(
        &mut app,
        "Crt",
        Part::CathodeArray,
        &[("width", SCREEN_WIDTH)],
        pins,
    );
    app
}

const SCREEN_WIRING: &[(&str, &str)] = &[
    ("addr", "d"),
    ("bit", SCREEN_RAIL),
    ("write", SCREEN_RAIL),
    ("byte", "d"),
    ("write_byte", SCREEN_RAIL),
];

#[test]
fn a_rail_draws_a_dot_and_a_byte_on_its_own_body_with_one_rise() {
    let mut app = screen_board(SCREEN_WIRING);
    assert!(surfaces(&app, "Crt").is_empty(), "a cold screen is dark");
    advance(&mut app, SETTLE_STAGES);
    assert_eq!(
        surfaces(&app, "Crt"),
        vec![DOT_AND_BYTE.to_string()],
        "addr 1 is dot 1 for the bit and byte 1 (dot 8, the second row) for the byte"
    );
}

#[test]
fn a_screen_missing_either_strobe_draws_only_the_other_write() {
    let without = |pin: &str| -> Vec<(&str, &str)> {
        SCREEN_WIRING
            .iter()
            .copied()
            .filter(|&(name, _)| name != pin)
            .collect()
    };
    let mut app = screen_board(&without("write_byte"));
    advance(&mut app, SETTLE_STAGES);
    assert_eq!(surfaces(&app, "Crt"), vec!["⠈⠀⠀⠀".to_string()]);

    let mut app = screen_board(&without("write"));
    advance(&mut app, SETTLE_STAGES);
    assert_eq!(surfaces(&app, "Crt"), vec!["⠂⠀⠀⠀".to_string()]);
}

const RASTER: [&str; 4] = ["Row0", "Row1", "Row2", "Row3"];

#[test]
fn a_gang_of_screens_sharing_addr_and_write_draws_one_dot_each_per_strobe() {
    let mut app = board(&RASTER);
    for (index, name) in RASTER.iter().enumerate() {
        sense(
            &mut app,
            name,
            Part::CathodeArray,
            &[("width", SCREEN_WIDTH)],
            &[
                ("addr", "x"),
                ("bit", &format!("column{index}")),
                ("write", STROBE),
            ],
        );
        set(&mut app, &format!("column{index}"), index % 2 == 0);
    }
    set(&mut app, "x0", true);
    step(&mut app);
    set(&mut app, STROBE, true);
    step(&mut app);

    let drawn: Vec<Vec<String>> = RASTER.iter().map(|name| surfaces(&app, name)).collect();
    assert_eq!(
        drawn,
        vec![
            vec!["⠈⠀⠀⠀".to_string()],
            vec![],
            vec!["⠈⠀⠀⠀".to_string()],
            vec![],
        ],
        "four screens, one strobe, four bits: ganging is the first parallel write"
    );
}

const FRAME: [u32; 4] = [0b0000_1111, 0b1111_0000, 0b0011_1100, 0b1100_0011];

#[test]
fn a_frame_held_in_a_core_stack_is_blitted_to_a_screen_a_byte_per_strobe() {
    let mut app = board(&["Ram", "Crt"]);
    sense(
        &mut app,
        "Ram",
        Part::CoreStack,
        &[],
        &[
            ("addr", "a"),
            ("data_in", "d"),
            ("data_out", WORD_BUS),
            ("write", WRITE),
        ],
    );
    sense(
        &mut app,
        "Crt",
        Part::CathodeArray,
        &[("width", SCREEN_WIDTH)],
        &[("addr", "a"), ("byte", WORD_BUS), ("write_byte", STROBE)],
    );
    let hold = |app: &mut App, bus: &str, value: u32| {
        for bit in 0..WORD_WIDTH {
            set(app, &format!("{bus}{bit}"), value >> bit & 1 == 1);
        }
    };
    for (address, &byte) in FRAME.iter().enumerate() {
        hold(&mut app, "a", address as u32);
        hold(&mut app, "d", byte);
        set(&mut app, WRITE, true);
        step(&mut app);
        set(&mut app, WRITE, false);
        step(&mut app);
    }
    assert!(
        surfaces(&app, "Crt").is_empty(),
        "the frame is in memory, not on screen"
    );

    let mut strobes = 0;
    for address in 0..FRAME.len() {
        hold(&mut app, "a", address as u32);
        step(&mut app);
        set(&mut app, STROBE, true);
        step(&mut app);
        set(&mut app, STROBE, false);
        strobes += 1;
    }

    assert_eq!(strobes, FRAME.len(), "one strobe per byte of the frame");
    assert_eq!(
        surfaces(&app, "Crt"),
        vec!["\u{28C9}\u{282D}\u{2836}\u{28D2}".to_string()],
        "an 8×4 frame is four bytes, copied from the stack the way it was stored"
    );
}
const FAR_TANK: &str = "Zion";
const THIRD_TANK: &str = "Eden";

fn relay_board() -> App {
    let mut app = board(&["Tx"]);
    for name in [FAR_TANK, THIRD_TANK] {
        app.tanks
            .push(Tank::new(name.to_string(), TankKind::Matrix, &[]));
    }
    app.tanks[1].spawn_fish(FishSpecies::Botfish, "Rx".to_string(), &mut rand::rng());
    gate(&mut app, "Tx", &[], "x", COIL);
    let bot = bot_state(&mut app, "Tx");
    assert!(bot.install(Part::RelayMast));
    let fields = Part::RelayMast.config();
    bot.configure(Part::RelayMast, &fields[0], FAR_TANK);
    bot.configure(Part::RelayMast, &fields[1], "y");
    bot.wire(Part::RelayMast, "in", "x");
    bot.wire(Part::RelayMast, "out", "echo");
    app.tanks[1].fish[0]
        .script_mut()
        .expect("a botfish is programmable")
        .listen("y");
    app.tanks[1].fish[0]
        .script_mut()
        .expect("a botfish is programmable")
        .drive("heard");
    app
}

fn far_level(app: &App, tank: usize, channel: &str) -> bool {
    app.tanks[tank].channels.level(channel)
}

#[test]
fn a_relay_mast_carries_a_rail_into_the_next_tank_one_fish_per_stage() {
    let mut app = relay_board();
    let mut seen = Vec::new();
    for _ in 0..4 {
        assert_eq!(app.tick_fabric(), 1);
        seen.push((
            level(&app, "x"),
            far_level(&app, 1, "y"),
            far_level(&app, 1, "heard"),
            level(&app, "echo"),
        ));
    }
    assert_eq!(
        seen,
        vec![
            (true, false, false, false),
            (true, true, false, false),
            (true, true, true, true),
            (true, true, true, true),
        ],
        "the rail, then the far wire, then the far fish and the read-back, a stage each"
    );
    assert!(
        !far_level(&app, 2, "y") && app.tanks[2].channels.is_empty(),
        "the Mast bridges exactly one channel and only to its configured tank"
    );
    assert!(!level(&app, "y"), "the home tank's y is another wire");
}

#[test]
fn a_relay_mast_whose_far_tank_is_sold_bridges_nothing() {
    let mut app = relay_board();
    for _ in 0..3 {
        app.tick_fabric();
    }
    assert!(level(&app, "echo"));
    run(&mut app, "/move \"Rx\" \"Fishtank\"");
    run(&mut app, &format!("/sell tank \"{FAR_TANK}\""));
    assert_eq!(app.tanks.len(), 2, "the far tank is gone");

    for _ in 0..2 {
        app.tick_fabric();
    }

    assert!(
        !level(&app, "echo"),
        "a tower with nobody to agree with hears nothing"
    );
    assert!(
        app.tanks[1].channels.is_empty(),
        "and no other tank inherits its wire"
    );
}

const FISHING_TICKS: usize = 300;

fn rig_board(cast_listens: &[&str], cast_drives: &str, bait: u32) -> App {
    let mut app = board(&["Rod"]);
    run(&mut app, "/fps 1");
    app.inventory.remove(&StockItem::BAIT);
    if bait > 0 {
        app.inventory.insert(StockItem::BAIT, bait);
    }
    gate(&mut app, "Rod", cast_listens, cast_drives, COIL);
    let bot = bot_state(&mut app, "Rod");
    assert!(bot.install(Part::AnglerRig));
    bot.wire(Part::AnglerRig, "cast", cast_drives);
    bot.wire(Part::AnglerRig, "caught", "caught");
    bot.wire(Part::AnglerRig, "bait_low", "low");
    app
}

fn bait(app: &App) -> u32 {
    app.inventory.get(&StockItem::BAIT).copied().unwrap_or(0)
}

fn haul(app: &App) -> (Money, u32, u32, usize) {
    let stock: u32 = app.inventory.values().sum();
    (
        app.purse.balance(),
        app.food_supply,
        stock,
        app.tanks[0].fish.len(),
    )
}

#[test]
fn one_cast_spends_one_bait_brings_one_fish_up_and_caught_pulses_once() {
    let mut app = rig_board(&[], "go", 1);
    let mut pulses = 0;
    let mut line_down = None;
    for _ in 0..FISHING_TICKS {
        app.tick();
        if line_down.is_none() && bait(&app) == 0 {
            line_down = Some(haul(&app));
        }
        pulses += usize::from(level(&app, "caught"));
    }
    let line_down = line_down.expect("the one cast took the one bait");
    assert_eq!(pulses, 1, "caught is one stage, once");
    assert_ne!(haul(&app), line_down, "a fish always comes up");
    assert_eq!(
        level(&app, "low"),
        bait(&app) == 0,
        "bait_low reads the stock the catch may have refilled"
    );
}

#[test]
fn a_cast_with_no_bait_does_nothing_at_all() {
    let mut app = rig_board(&[], "go", 0);
    let before = haul(&app);
    for _ in 0..FISHING_TICKS {
        app.tick();
        assert!(
            !level(&app, "caught"),
            "nothing on the hook, nothing comes up"
        );
    }
    assert_eq!(haul(&app), before);
    assert!(level(&app, "low"));
}

#[test]
fn a_fish_the_rig_lands_joins_its_tank_named_after_its_species_and_startles_it_like_a_human_catch()
{
    let mut app = rig_board(&["go"], "go", 50);
    let nerve = "Nerve";
    app.tanks[0].spawn_fish(FishSpecies::Botfish, nerve.to_string(), &mut rand::rng());
    let bot = bot_state(&mut app, nerve);
    assert!(bot.install(Part::StartleNerve));
    bot.wire(Part::StartleNerve, "catch", "fishy");
    let mut startled = false;
    let mut landed = None;
    for _ in 0..FISHING_TICKS * 10 {
        app.tick();
        startled |= level(&app, "fishy");
        landed = app.tanks[0]
            .fish
            .iter()
            .find(|f| f.name.starts_with(f.species.display_name()))
            .map(|f| (f.name.clone(), f.species));
        if landed.is_some() && startled {
            break;
        }
    }
    let (name, species) = landed.expect("a rig that keeps casting lands a fish");
    assert_eq!(
        name,
        species.display_name(),
        "named the way every bot names a fish"
    );
    assert!(
        startled,
        "the tank's catch pulse fires for a rig as for a human"
    );
}

const RAD_WATCH_TICKS: usize = 600;

#[test]
fn a_part_the_radtank_knocks_loose_comes_back_to_the_inventory() {
    let mut app = App::launch(Launch::Debug);
    run(&mut app, "/fps 1");
    app.tanks
        .push(Tank::new("Pripyat".to_string(), TankKind::Rad, &[]));
    app.tanks[1].spawn_fish(FishSpecies::Botfish, "Glow".to_string(), &mut rand::rng());
    assert!(
        app.tanks[1].fish[0]
            .script_mut()
            .expect("a botfish")
            .install(Part::InverterCoil)
    );
    let coil = StockItem::Consumable(ConsumableKind::Part(Part::InverterCoil));
    assert_eq!(app.inventory.get(&coil), None);

    let returned = (0..RAD_WATCH_TICKS).any(|_| {
        app.tick();
        app.inventory.get(&coil) == Some(&1)
    });

    assert!(returned, "the coil fell off and the player has it back");
    assert!(
        !app.tanks[1].fish[0]
            .script()
            .expect("a botfish")
            .parts()
            .has(Part::InverterCoil)
    );
}
