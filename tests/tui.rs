use std::path::Path;

use crossterm::event::KeyCode;
use fishtank::{
    consumable::ConsumeTarget,
    economy::Money,
    entities::cow::CowVariant,
    fishes::botfish::{BotfishState, level_color},
    fishes::fish::Direction,
    fishes::parts::{Part, PartTier},
    loot::{ConsumableKind, StockItem},
    tank::{Link, TankKind},
    testing::Tui,
};
use unicode_width::UnicodeWidthChar;

const BOT_RIGHT: &str = "[-[[[[[º-";
const BOT_LEFT: &str = "-º]]]]]-]";
const SCRIPT_TICKS: usize = 40;
const NUDGE_ARG_HINT: &str = "<name> <dx> <dy>";
const NAME_ARG_HINT: &str = "<name>";

fn bot_at(tui: &mut Tui) -> (usize, usize) {
    let screen = tui.screen();
    screen
        .find(BOT_RIGHT)
        .or_else(|| screen.find(BOT_LEFT))
        .unwrap_or_else(|| panic!("no botfish on screen:\n{}", screen.text()))
}

fn facing_right(tui: &mut Tui) -> bool {
    tui.screen().contains(BOT_RIGHT)
}

fn spawn_bot(tui: &mut Tui) {
    tui.clear_tank();
    tui.run("/spawn botfish \"Neo\"");
}

fn spawn_frozen_bot(tui: &mut Tui) {
    spawn_bot(tui);
    tui.run("/freeze \"Neo\"");
}

fn frozen(tui: &Tui) -> bool {
    tui.app.tanks[0]
        .fish
        .iter()
        .find(|f| f.name == "Neo")
        .expect("Neo exists")
        .frozen
}

const DEFAULT_TRIGGER: &str = "/run Neo";
const FIELDS_ABOVE_THE_SCRIPT: usize = 3;

fn program(tui: &mut Tui, lines: &[&str]) {
    tui.run("/program \"Neo\"");
    for _ in 0..FIELDS_ABOVE_THE_SCRIPT {
        tui.key(KeyCode::Down);
    }
    for (slot, line) in lines.iter().enumerate() {
        if slot > 0 {
            tui.key(KeyCode::Down);
        }
        tui.type_text(line);
    }
    tui.key(KeyCode::Enter);
    let saved = script_of(tui);
    assert_eq!(saved.1, lines, "the overlay saved the script lines");
    assert_eq!(
        saved.0, DEFAULT_TRIGGER,
        "the overlay keeps its default trigger"
    );
}

fn script_of(tui: &Tui) -> (String, Vec<String>) {
    let bot = tui.app.tanks[0]
        .fish
        .iter()
        .find(|f| f.name == "Neo")
        .expect("Neo exists")
        .script()
        .expect("Neo carries a script");
    (bot.trigger.clone(), bot.script.clone())
}

#[test]
fn the_botfish_is_drawn_on_screen() {
    let mut tui = Tui::new();
    spawn_frozen_bot(&mut tui);
    bot_at(&mut tui);
}

#[test]
fn a_typed_nudge_moves_the_botfish_on_screen() {
    let mut tui = Tui::new();
    spawn_frozen_bot(&mut tui);
    let (x, y) = bot_at(&mut tui);

    tui.run("/nudge \"Neo\" 5 2");

    assert_eq!(
        bot_at(&mut tui),
        (x + 5, y + 2),
        "the sprite moved on screen"
    );
}

#[test]
fn a_typed_flip_mirrors_the_botfish_on_screen() {
    let mut tui = Tui::new();
    spawn_frozen_bot(&mut tui);
    let before = facing_right(&mut tui);

    tui.run("/flip \"Neo\"");

    assert_ne!(facing_right(&mut tui), before, "the sprite mirrored");
}

#[test]
fn a_programmed_script_can_nudge_and_flip() {
    let mut tui = Tui::new();
    spawn_frozen_bot(&mut tui);
    program(&mut tui, &["/nudge \"Neo\" 5 2", "/flip \"Neo\""]);
    let (x, y) = bot_at(&mut tui);
    let facing_before = facing_right(&mut tui);

    tui.run(DEFAULT_TRIGGER);
    tui.tick_n(SCRIPT_TICKS * 3);

    assert_eq!(
        bot_at(&mut tui),
        (x + 5, y + 2),
        "the script's own /nudge must move the fish, exactly like typing it"
    );
    assert_ne!(
        facing_right(&mut tui),
        facing_before,
        "the script's own /flip must mirror the fish"
    );
}

#[test]
fn a_programmed_script_can_freeze_its_own_fish() {
    let mut tui = Tui::new();
    spawn_bot(&mut tui);
    program(&mut tui, &["/freeze \"Neo\""]);
    assert!(!frozen(&tui), "Neo starts loose");

    tui.run(DEFAULT_TRIGGER);
    tui.tick_n(SCRIPT_TICKS * 2);

    assert!(frozen(&tui), "the script pinned its own fish");
}

#[test]
fn a_programmed_script_can_set_the_clock() {
    let mut tui = Tui::new();
    spawn_frozen_bot(&mut tui);
    program(&mut tui, &["/clock 8"]);

    tui.run(DEFAULT_TRIGGER);
    tui.tick_n(SCRIPT_TICKS * 2);

    assert_eq!(
        tui.app.settings.stages_per_tick, 8,
        "a circuit may overclock itself, so /clock must be on the bot whitelist"
    );
}

#[test]
fn a_refused_clock_line_aborts_the_rest_of_the_script() {
    let mut tui = Tui::new();
    spawn_frozen_bot(&mut tui);
    program(&mut tui, &["/clock 0", "/nudge \"Neo\" 5 2"]);
    let (x, y) = bot_at(&mut tui);

    tui.run(DEFAULT_TRIGGER);
    tui.tick_n(SCRIPT_TICKS * 3);

    assert_eq!(
        tui.app.settings.stages_per_tick, 1,
        "a stopped clock is refused"
    );
    assert_eq!(
        bot_at(&mut tui),
        (x, y),
        "and a false line aborts the script"
    );
}

#[test]
fn a_script_line_the_bot_may_not_run_aborts_the_rest() {
    let mut tui = Tui::new();
    spawn_frozen_bot(&mut tui);
    program(&mut tui, &["/exit", "/nudge \"Neo\" 5 2"]);
    let (x, y) = bot_at(&mut tui);

    tui.run(DEFAULT_TRIGGER);
    tui.tick_n(SCRIPT_TICKS * 3);

    assert_eq!(
        bot_at(&mut tui),
        (x, y),
        "a forbidden line still aborts the script"
    );
    assert!(tui.app.running, "and /exit never escapes the whitelist");
}

#[test]
fn only_a_frozen_fish_can_be_arranged() {
    let mut tui = Tui::new();
    spawn_bot(&mut tui);
    let (x, y) = bot_at(&mut tui);

    tui.run("/nudge \"Neo\" 5 2");

    assert_eq!(
        bot_at(&mut tui),
        (x, y),
        "a loose botfish refuses to be shoved"
    );

    tui.run("/freeze \"Neo\"");
    let (x, y) = bot_at(&mut tui);
    tui.run("/nudge \"Neo\" 5 2");

    assert_eq!(
        bot_at(&mut tui),
        (x + 5, y + 2),
        "freezing it makes it arrangeable"
    );
}

#[test]
fn only_a_programmable_fish_can_be_arranged() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/spawn merluza \"Ann\"");
    let ann = |t: &Tui| {
        let f = t.app.tanks[0]
            .fish
            .iter()
            .find(|f| f.name == "Ann")
            .unwrap();
        (f.position.x, f.position.y, f.facing_left(), f.frozen)
    };
    let before = ann(&tui);

    tui.run("/freeze \"Ann\"");
    tui.run("/nudge \"Ann\" 5 2");
    tui.run("/flip \"Ann\"");

    assert_eq!(
        ann(&tui),
        before,
        "an ordinary fish cannot be frozen and is not circuit board furniture"
    );
}

#[test]
fn autocomplete_offers_only_arrangeable_fish() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/spawn merluza \"Ann\"");
    tui.run("/spawn botfish \"Neo\"");
    tui.run("/freeze \"Ann\"");

    tui.type_text("/nudge ");
    tui.screen().expect_absent(NUDGE_ARG_HINT);
    tui.type_text("A");
    tui.key(KeyCode::Tab);
    assert_eq!(
        tui.app.editor.text, "/nudge A",
        "wildlife cannot even be frozen, so it is never offered"
    );

    tui.run("/freeze \"Neo\"");
    tui.type_text("/nudge ");
    assert!(
        tui.screen().contains(NUDGE_ARG_HINT),
        "the hint returns once something is arrangeable"
    );
    tui.type_text("N");
    tui.key(KeyCode::Tab);
    assert!(
        tui.app.editor.text.contains("Neo"),
        "the frozen botfish is offered, got {:?}",
        tui.app.editor.text
    );
}

const INSTALL_HEADER: &str = "Inverter Coil to the botfishes";
const STOCKED_PARTS: u32 = 4;
const INSTALL_HINT: &str = "ENTER install";

fn held(tui: &Tui, kind: ConsumableKind) -> u32 {
    tui.app
        .inventory
        .get(&StockItem::Consumable(kind))
        .copied()
        .unwrap_or(0)
}

fn gift(kind: ConsumableKind) -> u32 {
    StockItem::Consumable(kind).gift_quantity()
}

fn stock(tui: &Tui, part: Part) -> u32 {
    tui.app
        .inventory
        .get(&ConsumeTarget::Part(part).stock())
        .copied()
        .unwrap_or(0)
}

fn installed(tui: &Tui, name: &str, part: Part) -> u32 {
    tui.app.tanks[0]
        .fish
        .iter()
        .find(|f| f.name == name)
        .expect("the fish exists")
        .script()
        .expect("the fish is programmable")
        .parts()
        .count(part)
}

fn bot_with_parts(tui: &mut Tui, part_name: &str) {
    spawn_bot(tui);
    tui.run(&format!("/add {part_name} {STOCKED_PARTS}"));
}

#[test]
fn a_given_part_goes_into_the_inventory_like_any_other_item() {
    let mut tui = Tui::new();
    spawn_bot(&mut tui);

    tui.run("/give inverter coil");

    assert_eq!(
        stock(&tui, Part::InverterCoil),
        gift(ConsumableKind::Part(Part::InverterCoil)),
        "a gift of a part is its rarity's gift, like every other item"
    );
    tui.run("/inventory");
    tui.screen().expect_find("Inverter Coil");
}

#[test]
fn consuming_a_part_installs_it_on_the_fish_you_pick() {
    let mut tui = Tui::new();
    bot_with_parts(&mut tui, "inverter coil");

    tui.run("/consume inverter coil");
    let screen = tui.screen();
    screen.expect_find(INSTALL_HEADER);
    screen.expect_find(INSTALL_HINT);

    tui.key(KeyCode::Enter);

    assert_eq!(installed(&tui, "Neo", Part::InverterCoil), 1);
    assert_eq!(
        stock(&tui, Part::InverterCoil),
        STOCKED_PARTS - 1,
        "exactly one part left the inventory"
    );
}

#[test]
fn every_picker_cell_keeps_a_space_between_its_text_and_the_column_rule() {
    let mut tui = Tui::new();
    bot_with_parts(&mut tui, "inverter coil");

    tui.run("/consume inverter coil");

    let row = row_with(&mut tui, "Botfish");
    assert!(
        !row.contains("│Neo") && !row.contains("│Botfish"),
        "no cell text is flush against a rule: {row:?}"
    );
    assert!(
        row.contains(" Neo ") && row.contains(" Botfish "),
        "every value is padded and left-aligned in its column: {row:?}"
    );
    let header = row_with(&mut tui, "Species");
    assert!(
        header.contains(" Species "),
        "and so is every header: {header:?}"
    );
}

#[test]
fn the_install_picker_offers_only_programmable_fish() {
    let mut tui = Tui::new();
    bot_with_parts(&mut tui, "inverter coil");
    tui.run("/spawn merluza \"Ann\"");

    tui.run("/consume inverter coil");

    let screen = tui.screen();
    screen.expect_find("Neo");
    screen.expect_absent("Ann");
}

#[test]
fn the_milk_picker_still_offers_every_fish() {
    let mut tui = Tui::new();
    spawn_bot(&mut tui);
    tui.run("/spawn merluza \"Ann\"");
    tui.run("/give chocolate milk");

    tui.run("/consume chocolate milk");

    let screen = tui.screen();
    screen.expect_find("Chocolate Milk to the fishes");
    screen.expect_find("Neo");
    screen.expect_find("Ann");
}

#[test]
fn a_refused_second_coil_costs_the_player_nothing() {
    let mut tui = Tui::new();
    bot_with_parts(&mut tui, "inverter coil");
    tui.run("/consume inverter coil");

    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Enter);

    assert_eq!(
        installed(&tui, "Neo", Part::InverterCoil),
        1,
        "a coil never double-inverts, so the second one is refused"
    );
    assert_eq!(
        stock(&tui, Part::InverterCoil),
        STOCKED_PARTS - 1,
        "and a refused install keeps the part in the inventory"
    );
}

#[test]
fn spools_stack_on_one_fish() {
    let mut tui = Tui::new();
    bot_with_parts(&mut tui, "delay spool");
    tui.run("/consume delay spool");

    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Enter);

    assert_eq!(installed(&tui, "Neo", Part::DelaySpool), 2);
    assert_eq!(stock(&tui, Part::DelaySpool), STOCKED_PARTS - 2);
}

#[test]
fn a_programmed_script_can_give_itself_parts() {
    let mut tui = Tui::new();
    spawn_frozen_bot(&mut tui);
    program(&mut tui, &["/give inverter coil"]);

    tui.run(DEFAULT_TRIGGER);
    tui.tick_n(SCRIPT_TICKS * 2);

    assert_eq!(
        stock(&tui, Part::InverterCoil),
        gift(ConsumableKind::Part(Part::InverterCoil)),
        "a part is an item, so /give reaches it from a script too"
    );
}

const NET: &str = "harvest";
const ANTENNA_ROWS: usize = 2;

fn drive(tui: &mut Tui, name: &str, channel: &str) {
    tui.app.tanks[0]
        .fish
        .iter_mut()
        .find(|f| f.name == name)
        .expect("the fish exists")
        .script_mut()
        .expect("the fish is programmable")
        .drive(channel);
}

fn park_mid_tank(tui: &mut Tui, name: &str) {
    let tank = &mut tui.app.tanks[0];
    let (mid_x, mid_y) = (tank.width as f32 / 2.0, tank.height as f32 / 2.0);
    let fish = tank
        .fish
        .iter_mut()
        .find(|f| f.name == name)
        .expect("the fish exists");
    fish.position.x = mid_x;
    fish.position.y = mid_y;
}

#[test]
fn nets_float_the_driven_channel_above_the_fish() {
    let mut tui = Tui::new();
    spawn_frozen_bot(&mut tui);
    drive(&mut tui, "Neo", NET);
    park_mid_tank(&mut tui, "Neo");
    tui.screen().expect_absent(NET);

    tui.run("/nets");

    let (bot_x, bot_y) = bot_at(&mut tui);
    let (net_x, net_y) = tui.screen().expect_find(NET);
    assert_eq!(
        net_y,
        bot_y - ANTENNA_ROWS - 1,
        "the channel name floats clear of the antenna"
    );
    assert!(
        net_x + NET.len() > bot_x && net_x < bot_x + BOT_RIGHT.chars().count(),
        "and sits over its own fish"
    );

    tui.run("/nets");
    tui.screen().expect_absent(NET);
}

#[test]
fn only_a_fish_that_drives_something_gets_a_net_label() {
    let mut tui = Tui::new();
    spawn_frozen_bot(&mut tui);
    tui.run("/spawn botfish \"Trin\"");
    tui.run("/freeze \"Trin\"");
    drive(&mut tui, "Trin", NET);
    park_mid_tank(&mut tui, "Trin");

    tui.run("/nets");

    assert_eq!(
        tui.screen().text().matches(NET).count(),
        1,
        "the unwired botfish is labelled with nothing"
    );
}

const ANTENNA_STEM: char = '‖';

#[test]
fn a_name_sits_above_the_antenna_and_the_net_above_the_name() {
    let mut tui = Tui::new();
    spawn_frozen_bot(&mut tui);
    drive(&mut tui, "Neo", NET);
    park_mid_tank(&mut tui, "Neo");

    tui.run("/names");
    tui.run("/nets");

    let (_, bot_y) = bot_at(&mut tui);
    let screen = tui.screen();
    let (_, name_y) = screen.expect_find("Neo");
    let (_, net_y) = screen.expect_find(NET);
    assert_eq!(
        name_y,
        bot_y - ANTENNA_ROWS - 1,
        "the name clears the antenna instead of writing over it:\n{}",
        screen.text()
    );
    assert!(
        screen.rows()[bot_y - 1].contains(ANTENNA_STEM),
        "the antenna stem is still drawn:\n{}",
        screen.text()
    );
    assert_eq!(
        net_y,
        name_y - 1,
        "and the net floats directly above the name"
    );
}

#[test]
fn a_programmed_script_can_toggle_nets() {
    let mut tui = Tui::new();
    spawn_frozen_bot(&mut tui);
    program(&mut tui, &["/nets"]);

    tui.run(DEFAULT_TRIGGER);
    tui.tick_n(SCRIPT_TICKS * 2);

    assert!(
        tui.app.settings.show_nets,
        "a circuit may light up its own board, so /nets must be on the bot whitelist"
    );
}

const CIRCUIT_TITLE: &str = "─ Circuit#";
const SIGNAL_HIGH: char = '●';
const SIGNAL_LOW: char = '○';
const LISTENS_LABEL: &str = "listens";
const LISTENS_HEADER: &str = "Listens";
const DRIVES_LABEL: &str = "drives";
const PARTS_LABEL: &str = "parts";
const CHAIN: [(&str, &str, &str); 3] = [
    ("Charlie", "sum", "done"),
    ("Bravo", "carry", "sum"),
    ("Alfa", "clk", "carry"),
];
const CHAIN_SOURCE_CHANNEL: &str = "clk";

fn listen(tui: &mut Tui, name: &str, channel: &str) {
    tui.app.tanks[0]
        .fish
        .iter_mut()
        .find(|f| f.name == name)
        .expect("the fish exists")
        .script_mut()
        .expect("the fish is programmable")
        .listen(channel);
}

fn spawn_chain(tui: &mut Tui) {
    tui.clear_tank();
    for (name, listens, drives) in CHAIN {
        tui.run(&format!("/spawn botfish \"{name}\""));
        listen(tui, name, listens);
        drive(tui, name, drives);
    }
}

fn row_with(tui: &mut Tui, needle: &str) -> String {
    let screen = tui.screen();
    screen
        .rows()
        .iter()
        .find(|row| row.contains(needle))
        .unwrap_or_else(|| panic!("no row holds {needle:?}:\n{}", screen.text()))
        .clone()
}

fn row_y(tui: &mut Tui, needle: &str) -> usize {
    tui.screen().expect_find(needle).1
}

#[test]
fn the_circuit_lists_wired_fish_in_the_order_the_signal_reaches_them() {
    let mut tui = Tui::new();
    spawn_chain(&mut tui);

    tui.run("/circuit");

    let (alfa, bravo, charlie) = (
        row_y(&mut tui, "Alfa"),
        row_y(&mut tui, "Bravo"),
        row_y(&mut tui, "Charlie"),
    );
    assert!(
        alfa < bravo && bravo < charlie,
        "the table flows downstream even though they were spawned upstream-last"
    );
}

#[test]
fn a_circuit_row_carries_the_parts_channels_and_driven_wire_of_its_fish() {
    let mut tui = Tui::new();
    spawn_chain(&mut tui);
    tui.run("/give inverter coil");
    tui.run("/consume inverter coil");
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Esc);

    tui.run("/circuit");

    let row = row_with(&mut tui, "Alfa");
    assert!(row.contains("clk"), "the channel it hears: {row:?}");
    assert!(row.contains("carry"), "the channel it drives: {row:?}");
    assert!(
        row_with(&mut tui, "Charlie").contains("Inverter Coil"),
        "and the fish the picker installed on carries its hardware"
    );
}

#[test]
fn the_circuit_reads_the_live_output_level_of_every_fish() {
    let mut tui = Tui::new();
    spawn_chain(&mut tui);

    tui.run("/circuit");
    assert!(
        row_with(&mut tui, "Alfa").contains(SIGNAL_LOW),
        "a cold board is all low"
    );

    tui.app.tanks[0]
        .channels
        .set_level(CHAIN_SOURCE_CHANNEL, true);
    tui.tick_n(1);

    assert!(
        row_with(&mut tui, "Alfa").contains(SIGNAL_HIGH),
        "one stage later the first fish is driving its wire"
    );
    assert!(
        row_with(&mut tui, "Bravo").contains(SIGNAL_LOW),
        "and the signal has not reached the next one yet"
    );
}

#[test]
fn an_unwired_botfish_has_no_circuit_to_read() {
    let mut tui = Tui::new();
    spawn_bot(&mut tui);

    tui.run("/circuit");

    tui.screen().expect_absent(CIRCUIT_TITLE);
}

#[test]
fn the_circuit_hints_keep_a_space_of_padding_on_both_sides() {
    let mut tui = Tui::new();
    spawn_chain(&mut tui);

    tui.run("/circuit");

    let hints = row_with(&mut tui, "ENTER wire");
    assert!(
        hints.contains("│ ↑↓ select"),
        "actions sit one space in from the left border: {hints:?}"
    );
    assert!(
        hints.contains("ESC/q close │"),
        "and the close hint one space in from the right: {hints:?}"
    );
}

const SCHEMATIC_ARROW: &str = "▶";
const SCHEMATIC_MAX_COLUMNS: usize = 8;
const HINT_TAB_SCHEMATIC: &str = "TAB schematic";
const HINT_TAB_SIGNAL: &str = "TAB signal";

fn spawn_deep_chain(tui: &mut Tui, length: usize) {
    tui.clear_tank();
    for step in 0..length {
        let name = format!("Fish{step}");
        tui.run(&format!("/spawn botfish \"{name}\""));
        listen(tui, &name, &format!("c{step}"));
        drive(tui, &name, &format!("c{}", step + 1));
    }
}

#[test]
fn tab_draws_the_circuit_as_a_layered_schematic() {
    let mut tui = Tui::new();
    spawn_chain(&mut tui);
    tui.run("/circuit");

    tui.key(KeyCode::Tab);

    let screen = tui.screen();
    let (alfa_x, alfa_y) = screen.expect_find("[Alfa]");
    let (bravo_x, bravo_y) = screen.expect_find("[Bravo]");
    let (charlie_x, _) = screen.expect_find("[Charlie]");
    assert!(
        alfa_x < bravo_x && bravo_x < charlie_x,
        "each fish sits in the column of its depth:\n{}",
        screen.text()
    );
    assert!(alfa_y < bravo_y, "and on a row of its own");
    assert!(
        screen.contains(SCHEMATIC_ARROW),
        "the wires arrive with an arrowhead:\n{}",
        screen.text()
    );
    screen.expect_absent(LISTENS_HEADER);
}

#[test]
fn tab_twice_comes_back_to_the_signal_view() {
    let mut tui = Tui::new();
    spawn_chain(&mut tui);
    tui.run("/circuit");

    tui.key(KeyCode::Tab);
    tui.key(KeyCode::Tab);

    let screen = tui.screen();
    screen.expect_find(LISTENS_HEADER);
    screen.expect_absent("[Alfa]");
}

#[test]
fn the_schematic_hint_offers_the_way_back_to_the_signal_view() {
    let mut tui = Tui::new();
    spawn_chain(&mut tui);
    tui.run("/circuit");
    assert!(
        row_with(&mut tui, "ENTER wire").contains(HINT_TAB_SCHEMATIC),
        "the signal view advertises the schematic"
    );

    tui.key(KeyCode::Tab);

    let hints = row_with(&mut tui, "ENTER wire");
    assert!(
        hints.contains("│ ↑↓ select"),
        "actions sit one space in from the left border: {hints:?}"
    );
    assert!(
        hints.contains(HINT_TAB_SIGNAL),
        "and the same key reads as the way back: {hints:?}"
    );
    assert!(
        hints.contains("ESC/q close │"),
        "and the close hint one space in from the right: {hints:?}"
    );
}

#[test]
fn a_circuit_too_deep_to_draw_says_so_and_stays_in_the_signal_view() {
    let mut tui = Tui::new();
    spawn_deep_chain(&mut tui, SCHEMATIC_MAX_COLUMNS + 1);
    tui.run("/circuit");

    tui.key(KeyCode::Tab);

    let screen = tui.screen();
    screen.expect_find(LISTENS_HEADER);
    screen.expect_absent("[Fish0]");
    assert!(
        screen.contains("too big to draw"),
        "the refusal is on screen, not silent:\n{}",
        screen.text()
    );
}

#[test]
fn a_circuit_one_fish_shallower_draws_after_all() {
    let mut tui = Tui::new();
    spawn_deep_chain(&mut tui, SCHEMATIC_MAX_COLUMNS);
    tui.run("/circuit");

    tui.key(KeyCode::Tab);

    let screen = tui.screen();
    screen.expect_find("[Fish0]");
    screen.expect_absent("too big to draw");
}

fn selected_row(tui: &mut Tui) -> String {
    tui.key(KeyCode::Enter);
    let screen = tui.screen();
    let name = CHAIN
        .iter()
        .map(|(name, _, _)| *name)
        .find(|name| screen.contains(&format!("─ {name} ")))
        .unwrap_or_else(|| panic!("no wiring panel opened:\n{}", screen.text()))
        .to_string();
    tui.key(KeyCode::Esc);
    name
}

#[test]
fn the_schematic_is_centred_in_its_box() {
    let mut tui = Tui::new();
    spawn_chain(&mut tui);
    tui.run("/circuit");

    tui.key(KeyCode::Tab);

    let screen = tui.screen();
    let (border_x, _) = screen.expect_find("┌─ Circuit#");
    let (graph_x, _) = screen.expect_find("[Alfa]");
    assert!(
        graph_x > border_x + 4,
        "the board sits centred in the box, not flush against the left border \
         (border at {border_x}, graph at {graph_x})"
    );
}

#[test]
fn left_and_right_walk_the_circuit_like_up_and_down() {
    let mut tui = Tui::new();
    spawn_chain(&mut tui);
    tui.run("/circuit");
    assert_eq!(selected_row(&mut tui), "Alfa");

    tui.key(KeyCode::Right);
    assert_eq!(selected_row(&mut tui), "Bravo", "right walks downstream");

    tui.key(KeyCode::Right);
    assert_eq!(selected_row(&mut tui), "Charlie");

    tui.key(KeyCode::Left);
    assert_eq!(selected_row(&mut tui), "Bravo", "and left walks back up");
}

#[test]
fn the_schematic_survives_a_trip_through_the_wiring_panel() {
    let mut tui = Tui::new();
    spawn_chain(&mut tui);
    tui.run("/circuit");
    tui.key(KeyCode::Tab);

    tui.key(KeyCode::Enter);
    tui.screen().expect_find(LISTENS_LABEL);
    tui.key(KeyCode::Esc);

    tui.screen().expect_find("[Alfa]");
}

#[test]
fn the_wiring_panel_hints_keep_a_space_of_padding_on_both_sides() {
    let mut tui = Tui::new();
    spawn_bot(&mut tui);

    tui.run("/program \"Neo\"");

    let hints = row_with(&mut tui, "ENTER save");
    assert!(
        hints.contains("│ ↑↓ field"),
        "actions sit one space in from the left border: {hints:?}"
    );
    assert!(
        hints.contains("ESC cancel │"),
        "and the close hint one space in from the right: {hints:?}"
    );
}

#[test]
fn enter_on_a_circuit_row_opens_that_fishs_wiring_panel_and_esc_comes_back() {
    let mut tui = Tui::new();
    spawn_chain(&mut tui);
    tui.run("/circuit");
    tui.key(KeyCode::Down);

    tui.key(KeyCode::Enter);

    let screen = tui.screen();
    screen.expect_absent(CIRCUIT_TITLE);
    screen.expect_find(LISTENS_LABEL);
    assert!(
        row_with(&mut tui, LISTENS_LABEL).contains("carry"),
        "the second row's fish is the one that opened"
    );

    tui.key(KeyCode::Esc);

    tui.screen().expect_find(CIRCUIT_TITLE);
}

#[test]
fn the_wiring_panel_puts_a_fish_onto_the_wires() {
    let mut tui = Tui::new();
    spawn_bot(&mut tui);

    tui.run("/program \"Neo\"");
    tui.type_text("x");
    tui.key(KeyCode::Down);
    tui.type_text("q");
    tui.key(KeyCode::Enter);

    let bot = tui.app.tanks[0]
        .fish
        .iter()
        .find(|f| f.name == "Neo")
        .expect("Neo exists")
        .script()
        .expect("Neo is programmable");
    assert!(bot.hears("x"), "the panel is how a player wires a fish");
    assert_eq!(bot.drives(), Some("q"));

    tui.app.tanks[0].channels.set_level("x", true);
    tui.tick_n(1);

    assert!(
        tui.app.tanks[0].channels.level("q"),
        "and the wiring it saved is the wiring the fabric runs"
    );
}

#[test]
fn the_wiring_panel_shows_the_parts_bolted_onto_the_fish() {
    let mut tui = Tui::new();
    bot_with_parts(&mut tui, "delay spool");
    tui.run("/consume delay spool");
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Esc);

    tui.run("/program \"Neo\"");

    let screen = tui.screen();
    screen.expect_find(PARTS_LABEL);
    screen.expect_find("Delay Spool ×2");
    screen.expect_find(DRIVES_LABEL);
}

#[test]
fn esc_discards_an_edit_and_enter_keeps_it() {
    let mut tui = Tui::new();
    spawn_bot(&mut tui);
    let heard = |tui: &Tui| {
        tui.app.tanks[0]
            .fish
            .iter()
            .find(|f| f.name == "Neo")
            .expect("Neo exists")
            .script()
            .expect("Neo is programmable")
            .hears("clk")
    };

    tui.run("/program \"Neo\"");
    tui.type_text("clk");
    tui.key(KeyCode::Esc);
    assert!(!heard(&tui), "ESC throws the edit away");

    tui.run("/program \"Neo\"");
    tui.type_text("clk");
    tui.key(KeyCode::Enter);
    assert!(heard(&tui), "ENTER saves it");
}

#[test]
fn freeze_autocomplete_only_offers_programmable_fish() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/spawn merluza \"Ann\"");

    tui.type_text("/freeze ");
    tui.screen().expect_absent(NAME_ARG_HINT);
    tui.type_text("A");
    tui.key(KeyCode::Tab);
    assert_eq!(
        tui.app.editor.text, "/freeze A",
        "wildlife is never offered to /freeze"
    );

    tui.run("/spawn botfish \"Neo\"");
    tui.type_text("/freeze ");
    assert!(
        tui.screen().contains(NAME_ARG_HINT),
        "the hint appears once there is something programmable to pin"
    );
    tui.type_text("N");
    tui.key(KeyCode::Tab);
    assert!(
        tui.app.editor.text.contains("Neo"),
        "the botfish is offered, got {:?}",
        tui.app.editor.text
    );
}

const CONFIG_RULE_TEXT: &str = "── config";
const SHOAL_COUNT_PIN_ROW: &str = "count[8]";
const SENSE_FIELDS_ABOVE_THE_COUNT_PIN: usize = 4;

fn bot_with_part(tui: &mut Tui, part: Part) {
    spawn_bot(tui);
    tui.run(&format!("/add {} 1", part.display_name().to_lowercase()));
    tui.run(&format!("/consume {}", part.display_name().to_lowercase()));
    assert_eq!(stock(tui, part), 1, "the part reached the inventory");
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Esc);
    assert_eq!(installed(tui, "Neo", part), 1);
}

#[test]
fn a_sense_part_is_bought_and_installed_through_the_doors_that_already_existed() {
    let mut tui = Tui::new();
    spawn_bot(&mut tui);

    tui.run("/give shoal counter");

    assert_eq!(
        stock(&tui, Part::ShoalCounter),
        gift(ConsumableKind::Part(Part::ShoalCounter))
    );
    tui.run("/inventory");
    tui.screen().expect_find("Shoal Counter");
}

#[test]
fn installing_a_sense_opens_its_config_and_pin_rows_in_the_wiring_panel() {
    let mut tui = Tui::new();
    bot_with_part(&mut tui, Part::ShoalCounter);

    tui.run("/program \"Neo\"");
    let screen = tui.screen();

    screen.expect_find(CONFIG_RULE_TEXT);
    screen.expect_find("target");
    screen.expect_find("threshold");
    screen.expect_find(SHOAL_COUNT_PIN_ROW);
    screen.expect_find("Shoal Counter");
}

#[test]
fn a_config_typed_into_the_panel_survives_the_save() {
    let mut tui = Tui::new();
    bot_with_part(&mut tui, Part::AssayScale);

    tui.run("/program \"Neo\"");
    tui.key(KeyCode::Down);
    tui.key(KeyCode::Down);
    for _ in 0.."heaviest".len() {
        tui.key(KeyCode::Backspace);
    }
    tui.type_text("richest");
    tui.key(KeyCode::Enter);

    let spec = Part::AssayScale
        .config()
        .iter()
        .find(|spec| spec.name == "target")
        .expect("the scale has a target");
    let saved = tui.app.tanks[0]
        .fish
        .iter()
        .find(|f| f.name == "Neo")
        .expect("Neo exists")
        .script()
        .expect("Neo is programmable")
        .config(Part::AssayScale);
    assert_eq!(saved.text(spec), "richest");
}

#[test]
fn a_wired_sense_bus_shows_up_under_its_fish_in_the_circuit() {
    let mut tui = Tui::new();
    bot_with_part(&mut tui, Part::ShoalCounter);

    tui.run("/program \"Neo\"");
    for _ in 0..SENSE_FIELDS_ABOVE_THE_COUNT_PIN {
        tui.key(KeyCode::Down);
    }
    tui.type_text("shoal");
    tui.key(KeyCode::Enter);

    tui.run("/circuit");
    let screen = tui.screen();
    screen.expect_find("count");
    screen.expect_find("shoal");
}

#[test]
fn a_sense_counts_the_tank_onto_its_bus_while_the_app_runs() {
    let mut tui = Tui::new();
    bot_with_part(&mut tui, Part::ShoalCounter);
    tui.run("/spawn merluza \"Fry\"");

    let bot = tui.app.tanks[0]
        .fish
        .iter_mut()
        .find(|f| f.name == "Neo")
        .expect("Neo exists")
        .script_mut()
        .expect("Neo is programmable");
    bot.wire(Part::ShoalCounter, "count", "shoal");

    tui.tick_n(4);

    assert!(
        tui.app.tanks[0].channels.level("shoal1"),
        "two fish is binary 10, so bit 1 is the high one"
    );
    assert!(!tui.app.tanks[0].channels.level("shoal0"));
}

const FIRE_PIN_ROW: &str = "fire[1]";
const FIELDS_ABOVE_THE_FIRE_PIN: usize = 2;
const FIRE_CHANNEL: &str = "go";
const SPOKEN: &str = "glup glup";
const BUBBLE: &str = "< glup glup >";

#[test]
fn installing_a_command_module_opens_a_fire_pin_field_in_the_wiring_panel() {
    let mut tui = Tui::new();
    bot_with_part(&mut tui, Part::CommandModule);

    tui.run("/program \"Neo\"");
    let screen = tui.screen();

    screen.expect_find(FIRE_PIN_ROW);
    screen.expect_find("Command Module");
}

#[test]
fn a_fire_pin_wired_in_the_panel_shows_up_in_the_circuit() {
    let mut tui = Tui::new();
    bot_with_part(&mut tui, Part::CommandModule);

    tui.run("/program \"Neo\"");
    for _ in 0..FIELDS_ABOVE_THE_FIRE_PIN {
        tui.key(KeyCode::Down);
    }
    tui.type_text(FIRE_CHANNEL);
    tui.key(KeyCode::Enter);

    tui.run("/circuit");
    let screen = tui.screen();
    screen.expect_find("fire");
    screen.expect_find(FIRE_CHANNEL);
}

#[test]
fn a_scripted_say_puts_a_bubble_over_the_fish_on_screen() {
    let mut tui = Tui::new();
    spawn_frozen_bot(&mut tui);
    park_mid_tank(&mut tui, "Neo");
    program(&mut tui, &[&format!("/say \"{SPOKEN}\"")]);
    tui.screen().expect_absent(BUBBLE);

    tui.run(DEFAULT_TRIGGER);
    tui.tick_n(SCRIPT_TICKS);

    tui.screen().expect_find(BUBBLE);
}

#[test]
fn a_fired_command_module_runs_the_script_the_panel_saved() {
    let mut tui = Tui::new();
    bot_with_part(&mut tui, Part::CommandModule);
    park_mid_tank(&mut tui, "Neo");

    tui.run("/program \"Neo\"");
    for _ in 0..FIELDS_ABOVE_THE_FIRE_PIN {
        tui.key(KeyCode::Down);
    }
    tui.type_text(FIRE_CHANNEL);
    tui.key(KeyCode::Down);
    tui.key(KeyCode::Down);
    tui.type_text(&format!("/say \"{SPOKEN}\""));
    tui.key(KeyCode::Enter);

    tui.app.tanks[0].channels.set_level(FIRE_CHANNEL, true);
    tui.tick_n(SCRIPT_TICKS);

    tui.screen().expect_find(BUBBLE);
}

const COCHLEA_PINS: [(&str, &str); 4] = [
    ("char", "kbd"),
    ("strobe", "clk"),
    ("ready", "rdy"),
    ("done", "fin"),
];

#[test]
fn a_cochlea_is_wired_through_the_panel_at_every_size() {
    for (cols, rows) in POPUP_SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("wiring-cochlea-{cols}x{rows}"),
        );
        bot_with_part(&mut tui, Part::Cochlea);
        tui.run("/program \"Neo\"");
        tui.snap("the panel opens on a fish with a Cochlea");
        for _ in 0..FIELDS_ABOVE_THE_FIRE_PIN {
            tui.key(KeyCode::Down);
        }
        for (index, (pin, channel)) in COCHLEA_PINS.iter().enumerate() {
            if index > 0 {
                tui.key(KeyCode::Down);
            }
            tui.type_text(channel);
            tui.snap(&format!("{pin} wired to {channel}"));
            tui.screen().expect_find(pin);
        }
        tui.key(KeyCode::Enter);
        tui.run("/circuit");
        tui.snap("the circuit lists the Cochlea's wires under Neo");
        tui.screen().expect_find("char");

        let bot = tui.app.tanks[0]
            .fish
            .iter()
            .find(|f| f.name == "Neo")
            .and_then(|f| f.script())
            .expect("Neo is programmable");
        for (pin, channel) in COCHLEA_PINS {
            assert_eq!(
                bot.pins().channel(Part::Cochlea, pin),
                Some(channel),
                "{cols}×{rows}"
            );
        }
        let report = tui.reel().flaw_report();
        assert!(report.is_empty(), "{cols}×{rows}: {report}");
    }
}

const SHOP_SIZES: [(u16, u16); 4] = [(100, 30), (60, 18), (40, 14), (28, 10)];
const ROBOTICS_ROW: &str = "Robotics";
const FISHTANK_ROW: &str = "Fishtank";
const PART_LIST_HEADER: &str = "Price/unit";
const MATRIX_TANK_NAME: &str = "Zion";
const HOME_TANK: &str = "Fishtank";

fn open_buy_categories(tui: &mut Tui) {
    tui.run("/shop");
    tui.key(KeyCode::Enter);
}

fn open_the_bench(tui: &mut Tui) {
    open_buy_categories(tui);
    tui.select(ROBOTICS_ROW);
    tui.key(KeyCode::Enter);
}

fn open_the_bench_tier(tui: &mut Tui, tier: PartTier) {
    open_the_bench(tui);
    tui.select(tier.display_name());
    tui.key(KeyCode::Enter);
}

fn grow_a_matrixtank(tui: &mut Tui) {
    tui.run("/give computer");
    tui.run("/consume computer");
    tui.type_text(MATRIX_TANK_NAME);
    tui.key(KeyCode::Enter);
    tui.run(&format!("/switch \"{HOME_TANK}\""));
}

#[test]
fn the_robotics_bench_is_locked_until_a_matrixtank_is_grown() {
    let mut tui = Tui::new();
    tui.stake();
    open_buy_categories(&mut tui);
    tui.screen().expect_find(ROBOTICS_ROW);

    tui.select(FISHTANK_ROW);
    tui.key(KeyCode::Down);
    tui.key(KeyCode::Enter);

    let screen = tui.screen();
    screen.expect_absent(Part::InverterCoil.display_name());
    screen.expect_find(TankKind::Base.display_name());
}

#[test]
fn a_matrixtank_opens_the_robotics_bench_onto_its_tiers() {
    let mut tui = Tui::new();
    tui.stake();
    grow_a_matrixtank(&mut tui);
    assert!(
        tui.app.is_connected(),
        "the terminal is connected by the tank, never by the item"
    );
    open_the_bench_tier(&mut tui, PartTier::Fabric);

    let screen = tui.screen();
    screen.expect_find(PART_LIST_HEADER);
    for &part in Part::ALL.iter().take(2) {
        screen.expect_find(part.display_name());
    }
}

#[test]
fn a_computer_in_the_inventory_opens_nothing_on_its_own() {
    let mut tui = Tui::new();
    tui.run("/give computer");

    assert!(
        !tui.app.is_connected(),
        "the seed is stock; the tank it grows is the connection"
    );
}

#[test]
fn the_fishtank_category_sells_no_tank_you_are_meant_to_find() {
    let mut tui = Tui::new();
    tui.stake();
    open_buy_categories(&mut tui);
    tui.select(FISHTANK_ROW);
    tui.key(KeyCode::Enter);

    let screen = tui.screen();
    screen.expect_find(TankKind::Base.display_name());
    screen.expect_absent(ConsumableKind::Computer.display_name());
    screen.expect_absent(TankKind::Matrix.display_name());
    screen.expect_absent(TankKind::Alien.display_name());
}

#[test]
fn a_tank_grown_from_an_item_cannot_be_bought_at_all() {
    let mut tui = Tui::new();
    let before = tui.app.purse.balance();
    tui.run(&format!("/buy {}", TankKind::Matrix.display_name()));

    assert_eq!(tui.app.tanks.len(), 1, "no tank appeared");
    assert_eq!(
        held(&tui, ConsumableKind::Computer),
        0,
        "and no item either"
    );
    assert_eq!(tui.app.purse.balance(), before, "and nothing was charged");
}

#[test]
fn buying_a_tank_with_no_seed_still_names_it_on_the_spot() {
    let mut tui = Tui::new();
    tui.stake();
    open_buy_categories(&mut tui);
    tui.select(FISHTANK_ROW);
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Enter);
    tui.type_text("Annex");
    tui.key(KeyCode::Enter);

    assert_eq!(tui.app.tanks.len(), 2);
    assert_eq!(tui.app.tanks[1].name, "Annex");
}

#[test]
fn an_item_that_grows_a_tank_is_sold_nowhere() {
    let mut tui = Tui::new();
    tui.stake();
    let before = tui.app.purse.balance();
    tui.run(&format!(
        "/buy {}",
        ConsumableKind::Necronomicon.display_name()
    ));

    assert_eq!(
        held(&tui, ConsumableKind::Necronomicon),
        0,
        "a funded player still buys none"
    );
    assert_eq!(tui.app.purse.balance(), before);
}

#[test]
fn the_tank_catalogue_draws_whole_at_every_size() {
    for (cols, rows) in SHOP_SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.stake();
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("shop-tanks-{cols}x{rows}"),
        );
        open_buy_categories(&mut tui);
        tui.snap("the six buy categories, Robotics dimmed");
        tui.select(FISHTANK_ROW);
        tui.key(KeyCode::Enter);
        tui.snap("the tank catalogue: only the tanks the shop sells");
        tui.key(KeyCode::Enter);
        tui.snap("a tank row opens the naming popup over the shop");
        tui.key(KeyCode::Esc);
        tui.select(TankKind::Candy.display_name());
        tui.snap("the cursor on the Candytank");
        tui.key(KeyCode::Enter);
        tui.snap("its naming popup, the shop still behind it");
        tui.key(KeyCode::Esc);

        let report = tui.reel().flaw_report();
        assert!(report.is_empty(), "{cols}×{rows}: {report}");
    }
}

#[test]
fn buying_a_part_at_the_bench_stocks_it_and_charges_for_it() {
    let mut tui = Tui::new();
    tui.stake();
    grow_a_matrixtank(&mut tui);
    let before = tui.app.purse.balance();
    open_the_bench_tier(&mut tui, PartTier::Fabric);

    let coil = Part::InverterCoil;
    tui.select(coil.display_name());
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Right);
    tui.key(KeyCode::Enter);

    assert_eq!(
        stock(&tui, coil),
        2,
        "the popup opened at one and RIGHT bought a second"
    );
    assert_eq!(
        tui.app.purse.balance(),
        before - Money::from(coil.price()) * 2
    );
}

const WAFER_NAME: &str = "Blank Wafer";
const PICKER_HEADER_TAIL: &str = "to the botfishes";

fn wafers(tui: &Tui) -> u32 {
    tui.app
        .inventory
        .get(&StockItem::Consumable(ConsumableKind::BlankWafer))
        .copied()
        .unwrap_or(0)
}

#[test]
fn a_blank_wafer_is_foundry_stock_and_never_goes_into_a_fish() {
    let mut tui = Tui::new();
    tui.run(&format!("/give {}", WAFER_NAME.to_ascii_lowercase()));
    assert_eq!(wafers(&tui), gift(ConsumableKind::BlankWafer));

    tui.run(&format!("/consume {}", WAFER_NAME.to_ascii_lowercase()));

    assert_eq!(
        wafers(&tui),
        gift(ConsumableKind::BlankWafer),
        "nothing was spent"
    );
    tui.screen().expect_absent(PICKER_HEADER_TAIL);
}

const DYNASTY_NAME: &str = "Seed1's Clone II's Clone's Clone III's Clone";
const NARROW_COLS: u16 = 60;
const NARROW_ROWS: u16 = 24;

#[test]
fn a_dynasty_name_is_clipped_so_the_index_keeps_its_columns() {
    let mut tui = Tui::with_size(NARROW_COLS, NARROW_ROWS);
    tui.clear_tank();
    tui.run(&format!("/spawn merluza \"{DYNASTY_NAME}\""));

    tui.run("/index");

    let screen = tui.screen();
    screen.expect_find("Seed1's Clone II's Clon…");
    screen.expect_find("Species");
    screen.expect_find("Display");
}

#[test]
fn a_blank_wafer_shows_up_in_the_inventory_by_name() {
    let mut tui = Tui::new();
    tui.run(&format!("/give {}", WAFER_NAME.to_ascii_lowercase()));
    tui.run("/inventory");

    tui.screen().expect_find(WAFER_NAME);
}

const CONSUME_BLANK: &str = "/consume blank circuit blueprint";
const NAMING_HEADER: &str = "Name your Circuit Blueprint";
const NAMING_HINT: &str = "ENTER capture";
const NAMING_CANCEL: &str = "ESC cancel";
const CLOCK_NAME: &str = "ring clock";
const CLOCK_TITLE: &str = "Ring Clock";
const POPUP_SIZES: [(u16, u16); 4] = [(100, 30), (60, 18), (40, 14), (28, 10)];

fn blanks(tui: &Tui) -> u32 {
    tui.app
        .inventory
        .get(&StockItem::Consumable(ConsumableKind::BlankBlueprint))
        .copied()
        .unwrap_or(0)
}

fn wired_clock(tui: &mut Tui) {
    tui.clear_tank();
    tui.run("/spawn botfish \"osc\"");
    tui.run("/add inverter coil 1");
    tui.run("/consume inverter coil");
    tui.key(KeyCode::Enter);
    tui.run("/program \"Osc\"");
    tui.type_text("ring");
    tui.key(KeyCode::Down);
    tui.type_text("ring");
    tui.key(KeyCode::Enter);
    tui.run("/give blank circuit blueprint");
}

fn capture(tui: &mut Tui, name: &str) {
    tui.run(CONSUME_BLANK);
    tui.type_text(name);
    tui.key(KeyCode::Enter);
}

#[test]
fn consuming_a_blank_blueprint_captures_the_tank_under_the_name_you_type() {
    let mut tui = Tui::new();
    wired_clock(&mut tui);

    tui.run(CONSUME_BLANK);
    let screen = tui.screen();
    screen.expect_find(NAMING_HEADER);
    screen.expect_find(NAMING_HINT);
    screen.expect_find(NAMING_CANCEL);
    tui.type_text(CLOCK_NAME);
    tui.key(KeyCode::Enter);

    tui.screen().expect_absent(NAMING_HEADER);
    assert_eq!(tui.app.blueprints.len(), 1);
    let blueprint = &tui.app.blueprints[0];
    assert_eq!(blueprint.name, CLOCK_TITLE, "the name is title-cased");
    assert_eq!(blueprint.fish.len(), 1);
    assert!(
        blueprint.pins().internals.contains("ring"),
        "a clock that hears itself is all inside"
    );
    assert_eq!(
        blanks(&tui),
        gift(ConsumableKind::BlankBlueprint) - 1,
        "exactly one blank was spent"
    );
}

#[test]
fn a_blank_blueprint_in_a_tank_with_no_circuit_opens_nothing_and_costs_nothing() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/spawn botfish \"idle\"");
    tui.run("/give blank circuit blueprint");

    tui.run(CONSUME_BLANK);

    tui.screen().expect_absent(NAMING_HEADER);
    assert!(tui.app.blueprints.is_empty());
    assert_eq!(blanks(&tui), gift(ConsumableKind::BlankBlueprint));
}

#[test]
fn escaping_the_naming_popup_keeps_the_blank_and_captures_nothing() {
    let mut tui = Tui::new();
    wired_clock(&mut tui);
    tui.run(CONSUME_BLANK);
    tui.type_text(CLOCK_NAME);

    tui.key(KeyCode::Esc);

    let screen = tui.screen();
    screen.expect_absent(NAMING_HEADER);
    screen.expect_find(ConsumableKind::BlankBlueprint.display_name());
    assert!(tui.app.blueprints.is_empty());
    assert_eq!(blanks(&tui), gift(ConsumableKind::BlankBlueprint));
}

#[test]
fn enter_on_a_blank_name_captures_nothing_and_leaves_the_popup_open() {
    let mut tui = Tui::new();
    wired_clock(&mut tui);
    tui.run(CONSUME_BLANK);

    tui.key(KeyCode::Enter);

    tui.screen().expect_find(NAMING_HEADER);
    assert!(tui.app.blueprints.is_empty());
    assert_eq!(blanks(&tui), gift(ConsumableKind::BlankBlueprint));
}

#[test]
fn two_blueprints_given_one_name_are_told_apart() {
    let mut tui = Tui::new();
    wired_clock(&mut tui);

    capture(&mut tui, CLOCK_NAME);
    capture(&mut tui, CLOCK_NAME);

    let names: Vec<&str> = tui.app.blueprints.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, vec![CLOCK_TITLE, "Ring Clock II"]);
}

#[test]
fn a_summoning_item_still_names_a_tank_through_the_same_popup() {
    let mut tui = Tui::new();
    tui.run("/give necronomicon");

    tui.run("/consume necronomicon");
    tui.screen().expect_find("Name your Helltank");
    tui.type_text("pit");
    tui.key(KeyCode::Enter);

    assert_eq!(tui.app.tanks.len(), 2);
    assert_eq!(tui.app.tanks[1].name, "Pit");
    assert!(tui.app.blueprints.is_empty());
}

const SELECTED_PREFIX: &str = "> ";

fn top_of_the_page(tier: PartTier) -> ConsumableKind {
    ConsumableKind::bench_stock()
        .into_iter()
        .filter(|kind| kind.bench_tier() == tier)
        .min_by_key(|kind| (kind.buy_price(), kind.display_name()))
        .expect("every tier sells something")
}

fn selected_label(kind: ConsumableKind) -> String {
    let first_word = kind.display_name().split(' ').next().unwrap_or_default();
    format!("{SELECTED_PREFIX}{first_word}")
}

#[test]
fn robotics_opens_a_menu_of_tiers_named_and_nothing_more() {
    let mut tui = Tui::new();
    grow_a_matrixtank(&mut tui);
    open_the_bench(&mut tui);

    let screen = tui.screen();
    for tier in PartTier::ALL {
        screen.expect_find(&format!("{} ", tier.display_name()));
    }
    screen.expect_absent(" — ");
    screen.expect_absent(Part::InverterCoil.display_name());
}

const UNSELECTED_PREFIX: &str = "  ";

fn tier_row(tier: PartTier) -> String {
    format!("{UNSELECTED_PREFIX}{} ", tier.display_name())
}

#[test]
fn the_bench_walks_through_every_tier_at_every_size() {
    for (cols, rows) in SHOP_SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.stake();
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("shop-bench-{cols}x{rows}"),
        );
        grow_a_matrixtank(&mut tui);
        open_the_bench(&mut tui);
        tui.snap("Robotics · the five tiers, by name");
        for &tier in PartTier::ALL {
            let kind = top_of_the_page(tier);
            tui.select(tier.display_name());
            tui.key(KeyCode::Enter);
            tui.snap(&format!("{} · a page of its own", tier.display_name()));
            let screen = tui.screen();
            let cursor = selected_label(kind);
            screen
                .find(&cursor)
                .unwrap_or_else(|| panic!("{cols}×{rows}: {cursor:?} is not on the {tier:?} page"));
            for other in PartTier::ALL.iter().filter(|&&other| other != tier) {
                screen.expect_absent(&tier_row(*other));
            }
            tui.key(KeyCode::Esc);
        }

        let report = tui.reel().flaw_report();
        assert!(report.is_empty(), "{cols}×{rows}: {report}");
    }
}

#[test]
fn the_bench_sells_blank_blueprints_on_the_materials_page() {
    let mut tui = Tui::new();
    tui.stake();
    grow_a_matrixtank(&mut tui);
    let before = tui.app.purse.balance();
    open_the_bench_tier(&mut tui, PartTier::Materials);
    tui.select(ConsumableKind::BlankBlueprint.display_name());

    tui.key(KeyCode::Enter);
    tui.screen().expect_find("Buy Blank Circuit Blueprint");
    tui.key(KeyCode::Enter);

    assert_eq!(blanks(&tui), 1);
    assert_eq!(
        tui.app.purse.balance(),
        before - Money::from(ConsumableKind::BlankBlueprint.buy_price())
    );
}

#[test]
fn the_naming_popup_draws_whole_at_every_size() {
    for (cols, rows) in POPUP_SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("blueprint-naming-{cols}x{rows}"),
        );
        wired_clock(&mut tui);
        tui.run(CONSUME_BLANK);
        tui.snap("the naming popup opens on an empty name");
        tui.type_text("a rather long name for one clock");
        tui.snap("a long name scrolls inside its field");

        let text = tui.screen().text();
        for hint in [NAMING_HINT, NAMING_CANCEL] {
            assert!(
                text.contains(hint),
                "the popup lost {hint:?} at {cols}×{rows}:\n{text}"
            );
        }
        let report = tui.reel().flaw_report();
        assert!(report.is_empty(), "{cols}×{rows}: {report}");
    }
}

const FOUNDRY_TITLE: &str = "─ Foundry ";
const LATCH_ROW: &str = "Latch (Circuit Blueprint)";
const LATCH_PINS: &str = "r, s → -";
const EXPORTED_LATCH_PINS: &str = "r, s → q";
const DISABLED_PRINT: &str = "needs a Fabricator";
const EXPORTS_HINT: &str = "X exports";
const SAVE_HINT: &str = "ENTER save";

fn wired_latch(tui: &mut Tui) {
    tui.clear_tank();
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
    grow_a_matrixtank(tui);
    tui.run("/give blank circuit blueprint");
    capture(tui, "latch");
}

#[test]
fn the_foundry_opens_nothing_without_a_blueprint() {
    let mut tui = Tui::new();
    tui.clear_tank();

    tui.run("/foundry");

    tui.screen().expect_absent(FOUNDRY_TITLE);
}

#[test]
fn the_foundry_lists_a_captured_blueprint_with_its_pins_parts_and_schematic() {
    let mut tui = Tui::new();
    wired_latch(&mut tui);

    tui.run("/foundry");

    let screen = tui.screen();
    screen.expect_find(FOUNDRY_TITLE);
    screen.expect_find("Latch");
    screen.expect_find("2 fish");
    screen.expect_find(LATCH_PINS);
    screen.expect_find("Inverter Coil ×2");
    screen.expect_find("[Q]");
    screen.expect_find("[Qn]");
    assert_eq!(
        screen.text().matches(DISABLED_PRINT).count(),
        2,
        "print and etch both say why they cannot run"
    );
}

#[test]
fn exporting_q_gives_a_latch_the_output_it_hears() {
    let mut tui = Tui::new();
    wired_latch(&mut tui);
    tui.run("/foundry");

    tui.type_text("x");
    tui.screen().expect_find(SAVE_HINT);
    tui.type_text("q");
    tui.key(KeyCode::Enter);

    tui.screen().expect_find(EXPORTED_LATCH_PINS);
    let pins = tui.app.blueprints[0].pins();
    assert!(pins.outputs.contains("q"));
    assert!(
        !pins.internals.contains("q"),
        "an export is no longer internal"
    );
    tui.key(KeyCode::Esc);
    tui.screen().expect_absent(FOUNDRY_TITLE);
}

#[test]
fn escaping_an_export_edit_keeps_the_old_exports_and_the_foundry_open() {
    let mut tui = Tui::new();
    wired_latch(&mut tui);
    tui.run("/foundry");

    tui.type_text("x");
    tui.type_text("q");
    tui.key(KeyCode::Esc);

    tui.screen().expect_find(FOUNDRY_TITLE);
    tui.screen().expect_find(EXPORTS_HINT);
    assert!(tui.app.blueprints[0].exports.is_empty());
}

#[test]
fn a_captured_blueprint_is_kept_in_the_inventory_by_name() {
    let mut tui = Tui::new();
    wired_latch(&mut tui);

    tui.run("/inventory");

    tui.screen().expect_find(LATCH_ROW);
}

#[test]
fn a_blueprint_sells_at_the_shop_for_a_blanks_resale_price() {
    let mut tui = Tui::new();
    wired_latch(&mut tui);
    let before = tui.app.purse.balance();
    tui.run("/shop");
    tui.key(KeyCode::Down);
    tui.key(KeyCode::Enter);
    tui.screen().expect_find(LATCH_ROW);
    tui.select(LATCH_ROW);

    tui.key(KeyCode::Enter);
    let price = ConsumableKind::BlankBlueprint.sell_price();
    tui.screen()
        .expect_find(&format!("Sell Latch for ${price}?"));
    tui.key(KeyCode::Enter);

    assert!(
        tui.app.blueprints.is_empty(),
        "the design left with the sale"
    );
    assert_eq!(tui.app.purse.balance(), before + Money::from(price));
}

#[test]
fn a_programmed_script_can_sell_its_blank_wafers() {
    let mut tui = Tui::new();
    spawn_frozen_bot(&mut tui);
    tui.run("/give blank wafer");
    program(&mut tui, &["/sell blank wafer 2"]);

    tui.run(DEFAULT_TRIGGER);
    tui.tick_n(SCRIPT_TICKS * 2);

    assert_eq!(wafers(&tui), gift(ConsumableKind::BlankWafer) - 2);
}

#[test]
fn the_foundry_draws_whole_at_every_size() {
    for (cols, rows) in POPUP_SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("foundry-{cols}x{rows}"),
        );
        wired_latch(&mut tui);
        tui.run("/foundry");
        tui.snap("the foundry opens on the latch, list focused");
        let listing = tui.screen().text();
        for hint in [EXPORTS_HINT, "TAB preview", "ESC/q close"] {
            assert!(
                listing.contains(hint),
                "the list lost {hint:?} at {cols}×{rows}:\n{listing}"
            );
        }
        tui.key(KeyCode::Tab);
        tui.snap("TAB gives the preview the arrows");
        for _ in 0..rows {
            tui.key(KeyCode::Down);
        }
        tui.snap("the preview scrolled to its end");
        for _ in 0..cols {
            tui.key(KeyCode::Right);
        }
        tui.snap("the schematic panned right");
        tui.key(KeyCode::Tab);
        tui.type_text("x");
        tui.type_text("q");
        tui.snap("typing an export");
        let editing = tui.screen().text();
        for hint in [SAVE_HINT, NAMING_CANCEL] {
            assert!(
                editing.contains(hint),
                "the export edit lost {hint:?} at {cols}×{rows}:\n{editing}"
            );
        }
        tui.key(KeyCode::Enter);
        tui.snap("q is exported");

        assert!(tui.app.blueprints[0].pins().outputs.contains("q"));
        let report = tui.reel().flaw_report();
        assert!(report.is_empty(), "{cols}×{rows}: {report}");
    }
}

const PRINT_HINT: &str = "P print";
const PRINTED_Q: &str = "Latch1 Q";
const LATCH_FISH: u32 = 2;

fn ready_print() -> String {
    let each = ConsumableKind::BlankWafer.buy_price() + Part::InverterCoil.price();
    format!(
        "Blank Wafer ×{LATCH_FISH}, Inverter Coil ×{LATCH_FISH} (buys ${})",
        LATCH_FISH * each
    )
}

fn fabricators(tui: &Tui) -> u32 {
    tui.app
        .inventory
        .get(&StockItem::Consumable(ConsumableKind::Fabricator))
        .copied()
        .unwrap_or(0)
}

fn swims(tui: &Tui, name: &str) -> bool {
    tui.app.tanks[tui.app.current_tank]
        .fish
        .iter()
        .any(|fish| fish.name == name)
}

#[test]
fn p_prints_the_selected_blueprint_and_closes_the_foundry() {
    let mut tui = Tui::new();
    tui.stake();
    wired_latch(&mut tui);
    tui.run("/give fabricator");
    let before = fabricators(&tui);
    tui.run("/foundry");
    tui.screen().expect_find(&ready_print());
    tui.screen().expect_find(PRINT_HINT);

    tui.type_text("p");

    tui.screen().expect_absent(FOUNDRY_TITLE);
    assert!(swims(&tui, PRINTED_Q));
    assert!(swims(&tui, "Latch1 Qn"));
    assert_eq!(fabricators(&tui), before - 1);
    assert_eq!(tui.app.blueprints.len(), 1, "the blueprint survives");
}

#[test]
fn p_without_a_fabricator_prints_nothing_and_keeps_the_reason_on_screen() {
    let mut tui = Tui::new();
    wired_latch(&mut tui);
    tui.run("/foundry");

    tui.type_text("p");

    let screen = tui.screen();
    screen.expect_find(FOUNDRY_TITLE);
    screen.expect_find(DISABLED_PRINT);
    assert!(!swims(&tui, PRINTED_Q));
}

#[test]
fn consuming_a_fabricator_opens_the_foundry_and_spends_nothing_until_a_print() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/give fabricator");
    let before = fabricators(&tui);

    tui.run("/consume fabricator");
    tui.screen().expect_absent(FOUNDRY_TITLE);

    wired_latch(&mut tui);
    tui.run("/consume fabricator");
    tui.screen().expect_find(FOUNDRY_TITLE);
    tui.key(KeyCode::Esc);
    assert_eq!(fabricators(&tui), before, "opening the Foundry is free");
}

#[test]
fn a_programmed_script_can_print_a_blueprint() {
    let mut tui = Tui::new();
    tui.stake();
    wired_latch(&mut tui);
    spawn_frozen_bot(&mut tui);
    tui.run("/give fabricator");
    program(&mut tui, &["/print \"Latch\""]);

    tui.run(DEFAULT_TRIGGER);
    tui.tick_n(SCRIPT_TICKS * 2);

    assert!(swims(&tui, PRINTED_Q), "the bot built a board");
}

#[test]
fn a_programmed_script_can_neither_reset_nor_export_nor_import_the_game() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("bot-save");
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let kept = dir.join("kept.ron");
    let smuggled = dir.join("smuggled.ron");
    let _ = std::fs::remove_file(&smuggled);
    for line in [
        "/reset".to_string(),
        format!("/export \"{}\"", smuggled.display()),
        format!("/import \"{}\"", kept.display()),
    ] {
        let mut tui = Tui::new();
        spawn_frozen_bot(&mut tui);
        tui.run(&format!("/export \"{}\"", kept.display()));
        tui.stake();
        let purse = tui.app.purse.balance();
        program(&mut tui, &[line.as_str()]);

        tui.run(DEFAULT_TRIGGER);
        tui.tick_n(SCRIPT_TICKS * 2);

        assert_eq!(tui.app.purse.balance(), purse, "{line}");
        assert!(swims(&tui, "Neo"), "{line}");
    }
    assert!(!smuggled.exists(), "a bot never writes a file");
}

#[test]
fn print_autocompletes_the_blueprints_you_own() {
    let mut tui = Tui::new();
    wired_latch(&mut tui);

    tui.type_text("/print ");
    tui.screen().expect_find("<blueprint>");
    tui.type_text("La");
    tui.key(KeyCode::Tab);

    assert!(
        tui.app.editor.text.contains("Latch"),
        "got {:?}",
        tui.app.editor.text
    );
}

#[test]
fn printing_from_the_foundry_draws_whole_at_every_size() {
    for (cols, rows) in POPUP_SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.stake();
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("foundry-print-{cols}x{rows}"),
        );
        wired_latch(&mut tui);
        tui.run("/foundry");
        tui.snap("no Fabricator: print says why");
        tui.key(KeyCode::Esc);
        tui.run("/give fabricator");
        tui.run("/consume fabricator");
        tui.snap("a Fabricator in stock: print lists what it spends and buys");
        let ready = tui.screen().text();
        for hint in [PRINT_HINT, "ESC/q close"] {
            assert!(
                ready.contains(hint),
                "the print hints lost {hint:?} at {cols}×{rows}:\n{ready}"
            );
        }
        tui.key(KeyCode::Tab);
        tui.snap("the preview holds the arrows and still offers P");
        tui.type_text("p");
        tui.snap("the board was printed and the Foundry closed");
        assert!(swims(&tui, PRINTED_Q), "{cols}×{rows}");
        tui.run("/circuit");
        tui.snap("the printed latch in /circuit");
        tui.key(KeyCode::Esc);

        let report = tui.reel().flaw_report();
        assert!(report.is_empty(), "{cols}×{rows}: {report}");
    }
}

#[test]
fn an_unconnected_foundry_says_it_cannot_buy_what_is_missing() {
    for (cols, rows) in POPUP_SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("foundry-unconnected-{cols}x{rows}"),
        );
        wired_latch(&mut tui);
        tui.run("/give fabricator");
        tui.run(&format!("/sell tank \"{MATRIX_TANK_NAME}\""));
        assert!(!tui.app.is_connected(), "{cols}x{rows}");

        tui.run("/foundry");
        tui.snap("no Computer: the bill cannot be quick-bought");
        let refused = tui.screen().text();
        assert!(
            refused.contains(NEEDS_CONNECTION) || (cols, rows) == SCROLLED_POPUP_SIZE,
            "the Foundry hid the connection refusal at {cols}x{rows}:\n{refused}"
        );

        tui.type_text("p");
        tui.snap("P is refused and the Foundry stays open");
        assert!(!swims(&tui, PRINTED_Q), "{cols}x{rows}");
        tui.screen().expect_find(FOUNDRY_TITLE);
        tui.key(KeyCode::Esc);

        let report = tui.reel().flaw_report();
        assert!(report.is_empty(), "{cols}x{rows}: {report}");
    }
}

const NEEDS_CONNECTION: &str = "Matrixtank";
const SCROLLED_POPUP_SIZE: (u16, u16) = (28, 10);
const ETCH_HINT: &str = "E etch";
const ETCH_PICKER: &str = "Latch to the botfishes";
const ETCH_CONFIRM: &str = "ENTER etch";
const READY_ETCH: &str = "Blank Wafer ×4, Inverter Coil ×2";
const LATCH_CHIP: &str = "Latch chip";

fn chip_of(tui: &Tui, name: &str) -> Option<String> {
    tui.app.tanks[tui.app.current_tank]
        .fish
        .iter()
        .find(|fish| fish.name == name)
        .and_then(|fish| fish.script())
        .and_then(|bot| bot.chip())
        .map(|chip| chip.name().to_string())
}

fn latch_with_a_host(tui: &mut Tui) {
    wired_latch(tui);
    tui.run("/foundry");
    tui.type_text("x");
    tui.type_text("q");
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Esc);
    spawn_bot(tui);
    tui.run("/give fabricator");
}

#[test]
fn e_opens_the_fish_picker_and_enter_burns_the_blueprint_into_the_fish() {
    let mut tui = Tui::new();
    tui.stake();
    latch_with_a_host(&mut tui);
    let before = fabricators(&tui);
    tui.run("/foundry");
    tui.screen().expect_find(READY_ETCH);
    tui.screen().expect_find(ETCH_HINT);

    tui.type_text("e");

    let picker = tui.screen();
    picker.expect_find(ETCH_PICKER);
    picker.expect_find(ETCH_CONFIRM);
    picker.expect_find("Neo");
    tui.key(KeyCode::Enter);

    let screen = tui.screen();
    screen.expect_absent(ETCH_PICKER);
    screen.expect_absent(FOUNDRY_TITLE);
    assert_eq!(chip_of(&tui, "Neo").as_deref(), Some("Latch"));
    assert_eq!(fabricators(&tui), before - 1);
    assert_eq!(tui.app.blueprints.len(), 1, "the blueprint survives");
}

#[test]
fn escaping_the_etch_picker_returns_to_the_foundry_on_the_same_blueprint() {
    let mut tui = Tui::new();
    tui.stake();
    latch_with_a_host(&mut tui);
    tui.run("/foundry");
    tui.type_text("e");

    tui.key(KeyCode::Esc);

    let screen = tui.screen();
    screen.expect_find(FOUNDRY_TITLE);
    screen.expect_find(READY_ETCH);
    assert_eq!(chip_of(&tui, "Neo"), None, "nothing was burned");
}

#[test]
fn e_without_a_fabricator_opens_no_picker() {
    let mut tui = Tui::new();
    wired_latch(&mut tui);
    tui.run("/foundry");

    tui.type_text("e");

    let screen = tui.screen();
    screen.expect_find(FOUNDRY_TITLE);
    screen.expect_absent(ETCH_PICKER);
}

#[test]
fn an_etched_fish_shows_its_chip_and_its_pins_in_the_circuit_and_the_panel() {
    let mut tui = Tui::new();
    tui.stake();
    latch_with_a_host(&mut tui);
    tui.run("/etch \"Latch\" \"Neo\"");

    tui.run("/circuit");
    let circuit = tui.screen();
    circuit.expect_find(LATCH_CHIP);
    circuit.expect_find("r ◂ r");
    circuit.expect_find("q ▸ q");
    tui.key(KeyCode::Esc);

    tui.run("/program \"Neo\"");
    let panel = tui.screen();
    panel.expect_find(LATCH_CHIP);
    panel.expect_find("s[1]");
    panel.expect_find("q[1]");
}

#[test]
fn a_programmed_script_can_etch_a_blueprint() {
    let mut tui = Tui::new();
    tui.stake();
    wired_latch(&mut tui);
    spawn_frozen_bot(&mut tui);
    tui.run("/spawn botfish \"Tim\"");
    tui.run("/give fabricator");
    program(&mut tui, &["/etch \"Latch\" \"Tim\""]);

    tui.run(DEFAULT_TRIGGER);
    tui.tick_n(SCRIPT_TICKS * 2);

    assert_eq!(chip_of(&tui, "Tim").as_deref(), Some("Latch"));
}

#[test]
fn etch_autocompletes_a_blueprint_and_then_a_botfish() {
    let mut tui = Tui::new();
    wired_latch(&mut tui);
    spawn_bot(&mut tui);

    tui.type_text("/etch ");
    tui.screen().expect_find("<blueprint> <fish>");
    tui.type_text("\"La");
    tui.key(KeyCode::Tab);
    assert_eq!(tui.app.editor.text, "/etch \"Latch\"");
    tui.type_text(" ");
    tui.screen().expect_find("<fish>");
    tui.type_text("N");
    tui.key(KeyCode::Tab);

    assert_eq!(tui.app.editor.text, "/etch \"Latch\" Neo");
}

#[test]
fn etching_from_the_foundry_draws_whole_at_every_size() {
    for (cols, rows) in POPUP_SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.stake();
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("foundry-etch-{cols}x{rows}"),
        );
        latch_with_a_host(&mut tui);
        tui.run("/foundry");
        tui.snap("a Fabricator in stock: etch lists what it spends and buys");
        let ready = tui.screen().text();
        for hint in [ETCH_HINT, PRINT_HINT, "ESC/q close"] {
            assert!(
                ready.contains(hint),
                "the foundry hints lost {hint:?} at {cols}×{rows}:\n{ready}"
            );
        }
        tui.type_text("e");
        tui.snap("E opens the fish picker");
        let picker = tui.screen().text();
        assert!(picker.contains(ETCH_CONFIRM), "{cols}×{rows}:\n{picker}");
        tui.key(KeyCode::Esc);
        tui.snap("ESC goes back to the Foundry");
        tui.key(KeyCode::Tab);
        tui.type_text("e");
        tui.snap("E from the preview focus opens the picker too");
        tui.key(KeyCode::Enter);
        tui.snap("the latch was burned into Neo and the Foundry closed");
        assert_eq!(
            chip_of(&tui, "Neo").as_deref(),
            Some("Latch"),
            "{cols}×{rows}"
        );
        tui.run("/circuit");
        tui.snap("the chip in /circuit, pins hanging under Neo");
        tui.key(KeyCode::Tab);
        tui.snap("the chip in the schematic");
        tui.key(KeyCode::Esc);
        tui.run("/program \"Neo\"");
        tui.snap("the chip's pins in the wiring panel");
        tui.key(KeyCode::Esc);

        let report = tui.reel().flaw_report();
        assert!(report.is_empty(), "{cols}×{rows}: {report}");
    }
}

const PANEL_FIELDS_ABOVE_ITS_CHAR_PIN: usize = 4;
const PANEL_TEXT: &[u8] = b"Hi";
const PANEL_TOP_ROW: &str = "/ Hi";
const PANEL_BUS: &str = "kbd";
const PANEL_WRITE: &str = "wr";
const BUS_WIDTH: u8 = 8;
const TALL_ENOUGH_FOR_THE_BUBBLE: u16 = 14;
const TALL_ENOUGH_FOR_BOTH_BUBBLES: u16 = 30;
const ENGULF_SECS: f32 = 5.0;
const HOST_WEIGHT_G: u32 = 5000;
const HOST: &str = "Ann";
const FUSED_HOST: &str = "Ann / Neo";

const ROWS_UNDER_A_LOW_FISH: f32 = 2.0;

fn park_low(tui: &mut Tui, name: &str) {
    let tank = &mut tui.app.tanks[0];
    let (mid_x, low_y) = (
        tank.width as f32 / 2.0,
        tank.height as f32 - ROWS_UNDER_A_LOW_FISH,
    );
    let fish = tank
        .fish
        .iter_mut()
        .find(|f| f.name == name)
        .expect("the fish exists");
    fish.position.x = mid_x;
    fish.position.y = low_y;
}

fn light_the_panel(tui: &mut Tui, text: &[u8]) {
    for &byte in text {
        let channels = &mut tui.app.tanks[0].channels;
        for bit in 0..BUS_WIDTH {
            channels.set_level(&format!("{PANEL_BUS}{bit}"), byte >> bit & 1 == 1);
        }
        channels.set_level(PANEL_WRITE, true);
        tui.tick_n(1);
        tui.app.tanks[0].channels.set_level(PANEL_WRITE, false);
        tui.tick_n(1);
    }
}

fn first_panel_row(tui: &Tui, name: &str) -> Option<String> {
    let displays = tui.app.tanks[0]
        .fish
        .iter()
        .find(|f| f.name == name)?
        .script()?
        .displays();
    Some(displays.first()?.rows()[0].trim_end().to_string())
}

fn wire_a_panel_through_the_program_overlay(tui: &mut Tui) {
    tui.run("/program \"Neo\"");
    tui.snap("the wiring panel on a fish with a Glyph Panel");
    for _ in 0..PANEL_FIELDS_ABOVE_ITS_CHAR_PIN {
        tui.key(KeyCode::Down);
    }
    tui.type_text(PANEL_BUS);
    tui.snap("char wired to kbd");
    tui.screen().expect_find("char[8]");
    tui.key(KeyCode::Down);
    tui.type_text(PANEL_WRITE);
    tui.snap("write wired to wr");
    tui.screen().expect_find("write[1]");
    tui.key(KeyCode::Enter);
}

#[test]
fn a_glyph_panel_is_wired_and_floats_its_bubble_at_every_size() {
    for (cols, rows) in POPUP_SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("glyph-panel-{cols}x{rows}"),
        );
        bot_with_part(&mut tui, Part::GlyphPanel);
        tui.run("/freeze \"Neo\"");
        park_low(&mut tui, "Neo");
        wire_a_panel_through_the_program_overlay(&mut tui);
        tui.screen().expect_absent(PANEL_TOP_ROW);

        light_the_panel(&mut tui, PANEL_TEXT);

        tui.snap("the panel's bubble floats over Neo");
        assert_eq!(first_panel_row(&tui, "Neo").as_deref(), Some("Hi"));
        if rows >= TALL_ENOUGH_FOR_THE_BUBBLE {
            tui.screen().expect_find(PANEL_TOP_ROW);
        }
        tui.tick_n(SCRIPT_TICKS);
        tui.snap("the bubble never expires");
        if rows >= TALL_ENOUGH_FOR_THE_BUBBLE {
            tui.screen().expect_find(PANEL_TOP_ROW);
        }

        tui.app.tanks[0]
            .fish
            .iter_mut()
            .find(|f| f.name == "Neo")
            .expect("Neo exists")
            .say("glup".to_string());
        tui.snap("a speech rides above the panel");
        let screen = tui.screen();
        let speech = screen.find("< glup >");
        let panel = screen.find(PANEL_TOP_ROW);
        if rows >= TALL_ENOUGH_FOR_BOTH_BUBBLES {
            assert!(
                speech.is_some() && panel.is_some(),
                "{cols}×{rows}: both bubbles fit"
            );
        }
        if let (Some((_, speech_row)), Some((_, panel_row))) = (speech, panel) {
            assert!(
                speech_row < panel_row,
                "{cols}×{rows}: the transient bubble sits on top"
            );
        }

        tui.run("/circuit");
        tui.snap("the circuit lists the panel's wires under Neo");
        tui.screen().expect_find("write");
        tui.key(KeyCode::Esc);

        let report = tui.reel().flaw_report();
        assert!(report.is_empty(), "{cols}×{rows}: {report}");
    }
}

#[test]
fn a_fish_that_engulfs_a_lit_panel_carries_its_bubble_at_every_size() {
    for (cols, rows) in POPUP_SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("glyph-panel-fused-{cols}x{rows}"),
        );
        bot_with_part(&mut tui, Part::GlyphPanel);
        tui.run("/freeze \"Neo\"");
        park_low(&mut tui, "Neo");
        wire_a_panel_through_the_program_overlay(&mut tui);
        light_the_panel(&mut tui, PANEL_TEXT);
        tui.run(&format!("/spawn merluza \"{HOST}\""));
        {
            let tank = &mut tui.app.tanks[0];
            let (x, y) = {
                let neo = tank.fish.iter().find(|f| f.name == "Neo").expect("Neo");
                (neo.position.x, neo.position.y)
            };
            let host = tank.fish.iter_mut().find(|f| f.name == HOST).expect("Ann");
            host.position.x = x;
            host.position.y = y;
            host.weight_g = HOST_WEIGHT_G;
            host.engulf_timer = ENGULF_SECS;
        }

        tui.tick_n(1);
        park_low(&mut tui, FUSED_HOST);
        tui.snap("the host that swallowed the panel still shows it");

        assert_eq!(
            first_panel_row(&tui, FUSED_HOST).as_deref(),
            Some("Hi"),
            "{cols}×{rows}: the bubble survives a fusion"
        );
        if rows >= TALL_ENOUGH_FOR_THE_BUBBLE {
            tui.screen().expect_find(PANEL_TOP_ROW);
        }
        let report = tui.reel().flaw_report();
        assert!(report.is_empty(), "{cols}×{rows}: {report}");
    }
}

const FIRE_NET: &str = "go";
const SPACE: KeyCode = KeyCode::Char(' ');
const KEY_HINTS: [&str; 5] = ["↑ up", "↓ down", "← left", "→ right", "SPACE fire"];
const CONSOLE_BAR: &str = "console Neo";
const LEAVE_HINT: &str = "ESC leave";

fn bot_mut<'a>(tui: &'a mut Tui, name: &str) -> &'a mut BotfishState {
    tui.app.tanks[0]
        .fish
        .iter_mut()
        .find(|f| f.name == name)
        .expect("the fish exists")
        .script_mut()
        .expect("the fish is programmable")
}

fn keypad(tui: &mut Tui) {
    bot_with_part(tui, Part::ReflexArc);
    bot_mut(tui, "Neo").wire(Part::ReflexArc, "fire", FIRE_NET);
}

fn fire_net(tui: &Tui) -> bool {
    tui.app.tanks[0].channels.level(FIRE_NET)
}

#[test]
fn console_mode_holds_a_pin_high_only_while_its_key_is_held() {
    let mut tui = Tui::new();
    keypad(&mut tui);
    tui.run("/console \"Neo\"");
    assert!(tui.app.console_open());
    tui.tick_n(1);
    assert!(!fire_net(&tui), "nothing is held yet");

    tui.key(SPACE);
    for _ in 0..3 {
        tui.tick_n(1);
        assert!(fire_net(&tui), "SPACE is held, so fire is high");
    }
    tui.release(SPACE);
    tui.tick_n(1);
    assert!(!fire_net(&tui), "let go, so low");
}

#[test]
fn a_key_tapped_between_two_ticks_is_high_for_exactly_the_next_tick() {
    let mut tui = Tui::new();
    keypad(&mut tui);
    tui.run("/console \"Neo\"");
    hold_through_a_repeat(&mut tui, SPACE);

    tui.key(SPACE);
    tui.release(SPACE);
    tui.tick_n(1);
    assert!(fire_net(&tui), "a tap is never lost");
    tui.tick_n(1);
    assert!(
        !fire_net(&tui),
        "and it lets go once the fabric has seen it"
    );
}

const FIRST_REPEAT_TICKS: usize = 15;
const UNPROVEN_TAP_TICKS: usize = 30;

fn hold_through_a_repeat(tui: &mut Tui, code: KeyCode) {
    tui.key(code);
    tui.tick_n(FIRST_REPEAT_TICKS);
    tui.key(code);
    tui.tick_n(1);
    tui.release(code);
    tui.tick_n(1);
}

#[test]
fn a_tap_whose_key_up_comes_in_the_same_frame_waits_for_its_repeats_until_key_ups_are_proven() {
    let mut tui = Tui::new();
    keypad(&mut tui);
    tui.run("/console \"Neo\"");

    tui.key(SPACE);
    tui.release(SPACE);
    tui.tick_n(1);
    assert!(fire_net(&tui), "a tap is never lost");
    tui.tick_n(1);
    assert!(
        fire_net(&tui),
        "a multiplexer's ConPTY sends that key-up with every key-down, so it proves nothing yet"
    );
    tui.tick_n(UNPROVEN_TAP_TICKS);
    assert!(!fire_net(&tui), "the key lets go when no repeat follows");
}

#[test]
fn console_mode_routes_keys_away_from_the_editor_and_esc_restores_it() {
    let mut tui = Tui::new();
    keypad(&mut tui);
    tui.run("/console \"Neo\"");
    tui.release(KeyCode::Enter);
    assert!(
        tui.app.console_open(),
        "the Enter that opened it changes nothing"
    );
    let screen = tui.screen();
    screen.expect_find(CONSOLE_BAR);
    screen.expect_find(LEAVE_HINT);

    tui.type_text("abc");
    tui.key(KeyCode::Backspace);
    assert_eq!(tui.app.editor.text, "", "no key reached the editor");
    tui.screen().expect_find(CONSOLE_BAR);

    tui.key(KeyCode::Esc);
    assert!(!tui.app.console_open());
    tui.screen().expect_absent(CONSOLE_BAR);
    tui.type_text("x");
    assert_eq!(tui.app.editor.text, "x", "the editor is back");
}

#[test]
fn leaving_the_console_lets_go_of_every_held_key() {
    let mut tui = Tui::new();
    keypad(&mut tui);
    tui.run("/console \"Neo\"");
    tui.key(SPACE);
    tui.tick_n(1);
    assert!(fire_net(&tui));

    tui.key(KeyCode::Esc);
    tui.tick_n(1);

    assert!(
        !fire_net(&tui),
        "a pin cannot stay held once the keyboard is gone"
    );
    tui.release(SPACE);
    assert_eq!(tui.app.editor.text, "", "a late release types nothing");
}

#[test]
fn outside_the_console_a_release_is_never_a_keystroke() {
    let mut tui = Tui::new();
    tui.release(KeyCode::Char('x'));
    tui.release(KeyCode::Enter);
    assert_eq!(tui.app.editor.text, "");
    tui.key(KeyCode::Char('x'));
    assert_eq!(tui.app.editor.text, "x", "a press still types");
}

#[test]
fn the_console_opens_only_on_a_fish_that_binds_a_key() {
    let mut tui = Tui::new();
    bot_with_part(&mut tui, Part::CommandModule);
    tui.run("/console \"Neo\"");
    assert!(
        !tui.app.console_open(),
        "a Command Module hears no keyboard"
    );
    tui.run("/console \"Nobody\"");
    assert!(!tui.app.console_open());
    tui.screen().expect_absent(CONSOLE_BAR);
}

#[test]
fn a_script_cannot_take_the_keyboard() {
    let mut tui = Tui::new();
    keypad(&mut tui);
    bot_mut(&mut tui, "Neo").program(
        DEFAULT_TRIGGER.to_string(),
        vec!["/console \"Neo\"".to_string()],
    );
    tui.run(DEFAULT_TRIGGER);
    tui.tick_n(SCRIPT_TICKS);
    assert!(
        !tui.app.console_open(),
        "/console is the player's, never a bot verb"
    );
}

#[test]
fn the_console_closes_when_its_fish_is_gone() {
    let mut tui = Tui::new();
    keypad(&mut tui);
    tui.run("/console \"Neo\"");
    tui.app.tanks[0].fish.clear();
    tui.tick_n(1);
    assert!(!tui.app.console_open());
    tui.type_text("x");
    assert_eq!(tui.app.editor.text, "x");
}

#[test]
fn console_autocompletes_the_fish_that_bind_a_key() {
    let mut tui = Tui::new();
    keypad(&mut tui);
    tui.type_text("/console N");
    tui.key(KeyCode::Tab);
    assert_eq!(tui.app.editor.text, "/console Neo");
}

#[test]
fn a_held_key_lights_its_hint_in_the_level_colour() {
    let mut tui = Tui::new();
    keypad(&mut tui);
    tui.run("/console \"Neo\"");
    tui.key(SPACE);
    tui.tick_n(1);
    tui.snap("SPACE held");
    let screen = tui.screen();
    let still = &tui.reel().stills()[0];
    let fg = |needle: &str| {
        let (x, y) = screen.expect_find(needle);
        still.glyph(x, y).expect("on screen").fg
    };
    assert_eq!(fg("SPACE fire"), level_color(true), "held is high");
    assert_eq!(fg("↑ up"), level_color(false), "unheld is low");
}

const MOVERS: [(&str, &str, &str, i32, i32); 4] = [
    ("North", "up", "n", 0, -1),
    ("South", "down", "s", 0, 1),
    ("West", "left", "w", -3, 0),
    ("East", "right", "e", 3, 0),
];
const GLUP_NET: &str = "glup";

fn position(tui: &Tui, name: &str) -> (f32, f32) {
    let fish = tui.app.tanks[0]
        .fish
        .iter()
        .find(|f| f.name == name)
        .expect("the fish exists");
    (fish.position.x, fish.position.y)
}

#[test]
fn the_section_8_2_demo_drives_a_botfish_with_the_arrow_keys() {
    let mut tui = Tui::new();
    tui.film(Path::new(env!("CARGO_TARGET_TMPDIR")), "console-arrow-keys");
    bot_with_part(&mut tui, Part::ReflexArc);
    tui.run("/freeze \"Neo\"");
    park_mid_tank(&mut tui, "Neo");
    {
        let pad = bot_mut(&mut tui, "Neo");
        pad.install(Part::CommandModule);
        pad.wire(Part::ReflexArc, "fire", GLUP_NET);
        pad.wire(Part::CommandModule, "fire", GLUP_NET);
        pad.program(
            DEFAULT_TRIGGER.to_string(),
            vec!["/say \"glup glup\"".to_string()],
        );
    }
    for (mover, pin, net, dx, dy) in MOVERS {
        tui.run(&format!("/spawn botfish \"{mover}\""));
        bot_mut(&mut tui, "Neo").wire(Part::ReflexArc, pin, net);
        let hand = bot_mut(&mut tui, mover);
        hand.install(Part::CommandModule);
        hand.wire(Part::CommandModule, "fire", net);
        hand.program(String::new(), vec![format!("/nudge \"Neo\" {dx} {dy}")]);
    }
    tui.run("/console \"Neo\"");
    tui.snap("the console hands the keyboard to Neo");

    let arrows = [KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right];
    for (code, (_, _, _, dx, dy)) in arrows.into_iter().zip(MOVERS) {
        let before = position(&tui, "Neo");
        tui.key(code);
        tui.tick_n(1);
        tui.release(code);
        tui.tick_n(SCRIPT_TICKS);
        let after = position(&tui, "Neo");
        assert_eq!(
            (after.0 - before.0, after.1 - before.1),
            (dx as f32, dy as f32),
            "{code:?} moved Neo by exactly one nudge"
        );
        tui.snap(&format!("{code:?} nudged Neo"));
    }

    tui.tick_n(SCRIPT_TICKS);
    let before = position(&tui, "Neo");
    tui.key(KeyCode::Right);
    tui.tick_n(SCRIPT_TICKS * 3);
    assert_eq!(
        position(&tui, "Neo").0 - before.0,
        3.0,
        "a held key is one rising edge: the module fires once"
    );
    tui.release(KeyCode::Right);

    tui.key(SPACE);
    tui.release(SPACE);
    tui.tick_n(SCRIPT_TICKS);
    tui.snap("SPACE made Neo say glup glup");
    tui.screen().expect_find("glup glup");
    tui.key(KeyCode::Esc);
    let report = tui.reel().flaw_report();
    assert!(report.is_empty(), "{report}");
}

#[test]
fn the_console_bar_draws_whole_and_drops_no_key_at_every_size() {
    for (cols, rows) in POPUP_SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("console-{cols}x{rows}"),
        );
        keypad(&mut tui);
        tui.run("/freeze \"Neo\"");
        park_low(&mut tui, "Neo");
        tui.snap("before: the editor");
        tui.run("/console \"Neo\"");
        tui.snap("the console bar replaces the editor");
        let bar = tui.screen().text();
        for hint in KEY_HINTS.iter().chain([&LEAVE_HINT]) {
            let first_word = hint.split(' ').next().expect("a hint has a word");
            assert!(
                bar.contains(first_word),
                "{cols}×{rows} dropped {hint:?}:\n{bar}"
            );
        }
        tui.key(SPACE);
        tui.key(KeyCode::Left);
        tui.tick_n(1);
        tui.snap("SPACE and ← held light in the level colour");
        tui.release(SPACE);
        tui.release(KeyCode::Left);
        tui.tick_n(2);
        tui.snap("let go");
        tui.key(KeyCode::Esc);
        tui.snap("ESC gives the editor back");
        tui.screen().expect_absent(CONSOLE_BAR);

        let report = tui.reel().flaw_report();
        assert!(report.is_empty(), "{cols}×{rows}: {report}");
    }
}

const CORE_STACK_PINS: [(&str, &str); 4] = [
    ("addr", "a"),
    ("data_in", "d"),
    ("data_out", "q"),
    ("write", "we"),
];

#[test]
fn a_core_stack_is_wired_through_the_panel_at_every_size() {
    for (cols, rows) in POPUP_SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("wiring-core-stack-{cols}x{rows}"),
        );
        bot_with_part(&mut tui, Part::CoreStack);
        tui.run("/program \"Neo\"");
        tui.snap("the panel opens on a fish with a Core Stack");
        tui.screen().expect_absent(CONFIG_RULE_TEXT);
        for _ in 0..FIELDS_ABOVE_THE_FIRE_PIN {
            tui.key(KeyCode::Down);
        }
        for (index, (pin, channel)) in CORE_STACK_PINS.iter().enumerate() {
            if index > 0 {
                tui.key(KeyCode::Down);
            }
            tui.type_text(channel);
            tui.snap(&format!("{pin} wired to {channel}"));
            tui.screen().expect_find(pin);
        }
        tui.key(KeyCode::Enter);
        tui.run("/circuit");
        tui.snap("the circuit lists the Core Stack's wires under Neo");
        tui.screen().expect_find("addr");

        let bot = bot_mut(&mut tui, "Neo");
        for (pin, channel) in CORE_STACK_PINS {
            assert_eq!(
                bot.pins().channel(Part::CoreStack, pin),
                Some(channel),
                "{cols}×{rows}"
            );
        }
        let report = tui.reel().flaw_report();
        assert!(report.is_empty(), "{cols}×{rows}: {report}");
    }
}

const SCREEN_FIELDS_ABOVE_ITS_ADDR_PIN: usize = 4;
const SCREEN_PINS: [(&str, &str); 5] = [
    ("addr", "a"),
    ("bit", "px"),
    ("write", "plot"),
    ("byte", "b"),
    ("write_byte", "blit"),
];
const SCREEN_ROW_BYTES: u32 = 8;
const SCREEN_BYTES: u32 = 32;
const BOX_LEFT: &str = "⣏⣉⣉";
const BOX_RIGHT: &str = "⣉⣉⣹";

fn place(tui: &mut Tui, name: &str, x: f32, y: f32, facing: Direction) {
    let fish = tui.app.tanks[0]
        .fish
        .iter_mut()
        .find(|f| f.name == name)
        .expect("the fish exists");
    fish.position.x = x;
    fish.position.y = y;
    fish.facing = facing;
}

fn hold_bus(tui: &mut Tui, bus: &str, value: u32) {
    let channels = &mut tui.app.tanks[0].channels;
    for bit in 0..BUS_WIDTH {
        channels.set_level(&format!("{bus}{bit}"), value >> bit & 1 == 1);
    }
}

fn blit_bytes(tui: &mut Tui, bytes: impl Iterator<Item = (u32, u32)>) {
    for (address, byte) in bytes {
        hold_bus(tui, "a", address);
        hold_bus(tui, "b", byte);
        tui.tick_n(1);
        tui.app.tanks[0].channels.set_level("blit", true);
        tui.tick_n(1);
        tui.app.tanks[0].channels.set_level("blit", false);
    }
    tui.tick_n(1);
}

fn outline(address: u32) -> u32 {
    let row = address / SCREEN_ROW_BYTES;
    let column = address % SCREEN_ROW_BYTES;
    if row == 0 || row == 3 {
        return 0xFF;
    }
    match column {
        0 => 0b0000_0001,
        7 => 0b1000_0000,
        _ => 0,
    }
}

fn draw_a_box(tui: &mut Tui) {
    blit_bytes(
        tui,
        (0..SCREEN_BYTES).map(|address| (address, outline(address))),
    );
}

fn wire_a_screen_through_the_program_overlay(tui: &mut Tui) {
    tui.run("/program \"Neo\"");
    tui.snap("the wiring panel on a fish with a Cathode Array");
    tui.screen().expect_find(CONFIG_RULE_TEXT);
    for _ in 0..SCREEN_FIELDS_ABOVE_ITS_ADDR_PIN {
        tui.key(KeyCode::Down);
    }
    for (index, (pin, channel)) in SCREEN_PINS.iter().enumerate() {
        if index > 0 {
            tui.key(KeyCode::Down);
        }
        tui.type_text(channel);
        tui.snap(&format!("{pin} wired to {channel}"));
        tui.screen().expect_find(pin);
    }
    tui.key(KeyCode::Enter);
}

#[test]
fn a_cathode_array_is_wired_and_draws_on_its_body_at_every_size() {
    for (cols, rows) in POPUP_SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("cathode-array-{cols}x{rows}"),
        );
        bot_with_part(&mut tui, Part::CathodeArray);
        tui.run("/freeze \"Neo\"");
        place(&mut tui, "Neo", 0.0, 3.0, Direction::Left);
        tui.snap("before: a dark screen is the plain botfish");
        tui.screen().expect_find(BOT_LEFT);
        wire_a_screen_through_the_program_overlay(&mut tui);

        draw_a_box(&mut tui);
        tui.snap("a box drawn a byte per strobe on Neo's body");
        tui.screen().expect_find(BOX_LEFT);
        tui.screen().expect_find("-º⣏");
        let bot = bot_mut(&mut tui, "Neo");
        for (pin, channel) in SCREEN_PINS {
            assert_eq!(
                bot.pins().channel(Part::CathodeArray, pin),
                Some(channel),
                "{cols}×{rows}"
            );
        }

        tui.run("/flip \"Neo\"");
        let right_edge = f32::from(tui.app.tanks[0].width) - BOT_RIGHT.chars().count() as f32;
        place(&mut tui, "Neo", right_edge, 3.0, Direction::Right);
        tui.tick_n(1);
        tui.snap("facing right the screen trails behind the eye, picture unmirrored");
        tui.screen().expect_find(&format!("{BOX_RIGHT}º-"));

        let glyphs = Part::CathodeArray
            .config()
            .iter()
            .find(|spec| spec.name == "glyphs")
            .expect("the cathode has a glyph style");
        bot_mut(&mut tui, "Neo").configure(Part::CathodeArray, glyphs, "quadrants");
        tui.tick_n(1);
        tui.snap("quadrants: the same box as solid blocks, two rows tall");
        tui.screen().expect_find("▜º-");

        tui.run("/circuit");
        tui.snap("the circuit lists the screen's wires under Neo");
        tui.screen().expect_find("addr");
        tui.key(KeyCode::Esc);

        let report = tui.reel().flaw_report();
        assert!(report.is_empty(), "{cols}×{rows}: {report}");
    }
}

const RASTER_ROWS: [&str; 3] = ["Top", "Mid", "Low"];
const RASTER_WIDTH: &str = "16";
const CHECKER_ROW: &str = "-º⢕⢕⢕⢕";

#[test]
fn ganged_screens_stacked_row_on_row_make_one_raster_at_every_size() {
    for (cols, rows) in POPUP_SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("cathode-raster-{cols}x{rows}"),
        );
        tui.clear_tank();
        let width = Part::CathodeArray
            .config()
            .iter()
            .find(|spec| spec.name == "width")
            .expect("the cathode has a width");
        for (index, name) in RASTER_ROWS.iter().enumerate() {
            tui.run(&format!("/spawn botfish \"{name}\""));
            tui.run(&format!("/freeze \"{name}\""));
            let top = f32::from(tui.app.tanks[0].height) - RASTER_ROWS.len() as f32;
            place(&mut tui, name, 2.0, top + index as f32, Direction::Left);
            let bot = bot_mut(&mut tui, name);
            assert!(bot.install(Part::CathodeArray));
            bot.configure(Part::CathodeArray, width, RASTER_WIDTH);
            bot.wire(Part::CathodeArray, "addr", "a");
            bot.wire(Part::CathodeArray, "byte", "b");
            bot.wire(Part::CathodeArray, "write_byte", "blit");
        }
        tui.tick_n(1);
        tui.snap("three dark screens stacked, eyes down the left edge");

        blit_bytes(
            &mut tui,
            (0..8).map(|address| (address, if address / 2 % 2 == 0 { 0x55 } else { 0xAA })),
        );
        tui.snap("one strobe per byte writes all three rows at once: a checkerboard");
        let screen = tui.screen();
        for name in RASTER_ROWS {
            let drawn = tui.app.tanks[0]
                .fish
                .iter()
                .find(|f| f.name == name)
                .and_then(|f| f.script())
                .map(|bot| bot.displays())
                .unwrap_or_default();
            assert_eq!(drawn.len(), 1, "{cols}×{rows}: {name} lit");
        }
        let raster: Vec<usize> = screen
            .rows()
            .iter()
            .enumerate()
            .filter(|(_, row)| row.contains(CHECKER_ROW))
            .map(|(index, _)| index)
            .collect();
        assert_eq!(raster.len(), RASTER_ROWS.len(), "{cols}×{rows}: {raster:?}");
        assert_eq!(
            raster
                .last()
                .zip(raster.first())
                .map(|(low, top)| low - top),
            Some(2),
            "{cols}×{rows}: three rows, one text row each"
        );

        let report = tui.reel().flaw_report();
        assert!(report.is_empty(), "{cols}×{rows}: {report}");
    }
}

const MAST_FIELDS_ABOVE_ITS_TANK: usize = 2;
const MAST_FIELDS: [(&str, &str); 4] = [
    ("tank", "Zion"),
    ("channel", "y"),
    ("in", "x"),
    ("out", "echo"),
];

#[test]
fn a_relay_mast_is_aimed_and_wired_through_the_panel_at_every_size() {
    for (cols, rows) in POPUP_SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("wiring-relay-mast-{cols}x{rows}"),
        );
        bot_with_part(&mut tui, Part::RelayMast);
        tui.run("/program \"Neo\"");
        tui.snap("the panel opens on a fish with a Relay Mast");
        for _ in 0..MAST_FIELDS_ABOVE_ITS_TANK {
            tui.key(KeyCode::Down);
        }
        for (index, (field, value)) in MAST_FIELDS.iter().enumerate() {
            if index > 0 {
                tui.key(KeyCode::Down);
            }
            tui.type_text(value);
            tui.snap(&format!("{field} set to {value}"));
            tui.screen().expect_find(field);
        }
        tui.key(KeyCode::Enter);
        tui.run("/circuit");
        tui.snap("the circuit lists the Mast's two wires under Neo");
        tui.screen().expect_find("echo");

        let bot = bot_mut(&mut tui, "Neo");
        assert_eq!(bot.links(), vec![Link::new("Zion", "y").expect("a link")]);
        assert_eq!(bot.pins().channel(Part::RelayMast, "in"), Some("x"));
        assert_eq!(bot.pins().channel(Part::RelayMast, "out"), Some("echo"));
        let report = tui.reel().flaw_report();
        assert!(report.is_empty(), "{cols}×{rows}: {report}");
    }
}

const RIG_FIELDS_ABOVE_ITS_CAST: usize = 3;
const RIG_PINS: [(&str, &str); 3] = [("cast", "go"), ("caught", "fish"), ("bait_low", "low")];

#[test]
fn an_angler_rig_is_wired_through_the_panel_at_every_size() {
    for (cols, rows) in POPUP_SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("wiring-angler-rig-{cols}x{rows}"),
        );
        bot_with_part(&mut tui, Part::AnglerRig);
        tui.run("/program \"Neo\"");
        tui.snap("the panel opens on a fish with an Angler Rig");
        for _ in 0..RIG_FIELDS_ABOVE_ITS_CAST {
            tui.key(KeyCode::Down);
        }
        for (index, (pin, channel)) in RIG_PINS.iter().enumerate() {
            if index > 0 {
                tui.key(KeyCode::Down);
            }
            tui.type_text(channel);
            tui.snap(&format!("{pin} wired to {channel}"));
            tui.screen().expect_find(pin);
        }
        tui.key(KeyCode::Enter);
        tui.run("/circuit");
        tui.snap("the circuit lists the rig's three wires under Neo");
        tui.screen().expect_find("cast");

        let bot = bot_mut(&mut tui, "Neo");
        for (pin, channel) in RIG_PINS {
            assert_eq!(
                bot.pins().channel(Part::AnglerRig, pin),
                Some(channel),
                "{cols}×{rows}"
            );
        }
        let report = tui.reel().flaw_report();
        assert!(report.is_empty(), "{cols}×{rows}: {report}");
    }
}

const ROW_OF_A_FISH_AT_THE_TOP: f32 = 1.0;
const PANEL_INNER_WIDTH: usize = 16;
const COW_AGAINST_THE_WALL: f32 = 0.0;

fn park_at(tui: &mut Tui, name: &str, x: f32, y: f32) {
    let fish = tui.app.tanks[0]
        .fish
        .iter_mut()
        .find(|f| f.name == name)
        .expect("the fish exists");
    fish.position.x = x;
    fish.position.y = y;
}

fn whole_panel_row(text: &str) -> String {
    format!("/ {text:<PANEL_INNER_WIDTH$} \\")
}

#[test]
fn a_panel_with_no_room_above_its_fish_opens_below_it() {
    for (cols, rows) in POPUP_SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("glyph-panel-flipped-{cols}x{rows}"),
        );
        bot_with_part(&mut tui, Part::GlyphPanel);
        tui.leave_debug_mode();
        tui.run("/freeze \"Neo\"");
        let mid = tui.app.tanks[0].width as f32 / 2.0;
        park_at(&mut tui, "Neo", mid, ROW_OF_A_FISH_AT_THE_TOP);
        wire_a_panel_through_the_program_overlay(&mut tui);
        light_the_panel(&mut tui, PANEL_TEXT);
        tui.snap("a fish at the top of the tank opens its panel below itself");
        let (_, fish_row) = bot_at(&mut tui);
        let screen = tui.screen();
        let Some((_, panel_row)) = screen.find(PANEL_TOP_ROW) else {
            panic!("{cols}×{rows}: the panel vanished:\n{}", screen.text());
        };
        assert!(
            panel_row > fish_row,
            "{cols}×{rows}: a panel with no room above opens below its fish"
        );
        let report = tui.reel().flaw_report();
        assert!(report.is_empty(), "{cols}×{rows}: {report}");
    }
}

#[test]
fn a_bubble_at_the_edge_of_the_tank_slides_inside_it_whole() {
    for (cols, rows) in POPUP_SIZES {
        if rows < TALL_ENOUGH_FOR_THE_BUBBLE {
            continue;
        }
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("glyph-panel-at-the-edge-{cols}x{rows}"),
        );
        bot_with_part(&mut tui, Part::GlyphPanel);
        tui.run("/freeze \"Neo\"");
        park_low(&mut tui, "Neo");
        let right_edge = (tui.app.tanks[0].width - BOT_RIGHT.chars().count() as u16) as f32;
        let low = tui.app.tanks[0].height as f32 - ROWS_UNDER_A_LOW_FISH;
        park_at(&mut tui, "Neo", right_edge, low);
        wire_a_panel_through_the_program_overlay(&mut tui);
        light_the_panel(&mut tui, PANEL_TEXT);
        tui.snap("a fish against the right wall keeps its whole panel on screen");
        tui.screen().expect_find(&whole_panel_row("Hi"));
        park_at(&mut tui, "Neo", 0.0, low);
        tui.snap("and against the left wall too");
        tui.screen().expect_find(&whole_panel_row("Hi"));
        let report = tui.reel().flaw_report();
        assert!(report.is_empty(), "{cols}×{rows}: {report}");
    }
}

#[test]
fn a_cow_against_the_left_wall_keeps_its_whole_bubble() {
    let mut tui = Tui::with_size(60, 18);
    tui.film(
        Path::new(env!("CARGO_TARGET_TMPDIR")),
        "cow-speaks-at-the-wall",
    );
    tui.clear_tank();
    tui.app.tanks[0].spawn_cow(CowVariant::WhiteBlack, &mut rand::rng());
    let cow = &mut tui.app.tanks[0].cows[0];
    cow.position.x = COW_AGAINST_THE_WALL;
    cow.say("moo".to_string());
    tui.snap("a cow against the left wall says moo, bubble slid inside the tank");
    tui.screen().expect_find("< moo >");
    let report = tui.reel().flaw_report();
    assert!(report.is_empty(), "{report}");
}

const TAIL_WAVE_TICKS: usize = 200;
const KOI_SPRITE_SPAN: usize = 24;
const OVERLAY_TOP_LEFT: &str = "┌─";

fn swing_a_wide_tail(tui: &mut Tui, name: &str) {
    for _ in 0..TAIL_WAVE_TICKS {
        let fish = tui.app.tanks[0]
            .fish
            .iter()
            .find(|f| f.name == name)
            .expect("the fish exists");
        if fish
            .segments()
            .iter()
            .any(|&(ch, _)| UnicodeWidthChar::width(ch).unwrap_or(1) > 1)
        {
            return;
        }
        tui.tick_n(1);
    }
    panic!("{name} never swung its wide tail");
}

#[test]
fn a_wide_tail_under_an_overlays_edge_never_breaks_its_border() {
    let mut tui = Tui::new();
    tui.film(
        Path::new(env!("CARGO_TARGET_TMPDIR")),
        "wide-tail-under-an-overlay",
    );
    tui.clear_tank();
    tui.run("/spawn koi \"Tailor\"");
    swing_a_wide_tail(&mut tui, "Tailor");
    tui.run("/fishtanks");
    let (left, top) = tui.screen().expect_find(OVERLAY_TOP_LEFT);
    for x in left.saturating_sub(KOI_SPRITE_SPAN)..=left {
        park_at(&mut tui, "Tailor", x as f32, (top + 1) as f32);
        tui.snap(&format!("the koi's sprite starting at column {x}"));
    }
    let report = tui.reel().flaw_report();
    assert!(report.is_empty(), "{report}");
}

const EYE: char = 'º';
const BUBBLE_TOP_EDGE: char = '_';
const BUBBLE_BOTTOM_EDGE: char = '-';

fn face(tui: &mut Tui, facing: Direction) {
    tui.app.tanks[0].fish[0].facing = facing;
}

fn char_at(tui: &mut Tui, x: i32, y: i32) -> char {
    tui.screen().rows()[y as usize]
        .chars()
        .nth(x as usize)
        .expect("the cell is on screen")
}

fn eye_of_the_fish(tui: &mut Tui) -> (i32, i32) {
    let fish = &tui.app.tanks[0].fish[0];
    (fish.eye_x(), fish.position.y as i32)
}

fn expect_tail(tui: &mut Tui, forward: i32, up: i32, glyph: char, edge: char) {
    let (eye_x, eye_y) = eye_of_the_fish(tui);
    assert_eq!(
        char_at(tui, eye_x, eye_y),
        EYE,
        "the eye is where the fish says it is"
    );
    for step in 1..=2 {
        assert_eq!(
            char_at(tui, eye_x + forward * step, eye_y + up * step),
            glyph,
            "diagonal {step} leans off the eye"
        );
    }
    assert_eq!(
        char_at(tui, eye_x + forward * 2, eye_y + up * 3),
        edge,
        "the bubble sits right on its tail"
    );
}

#[test]
fn a_fish_bubble_hangs_from_a_tail_that_leans_towards_its_head() {
    let mut tui = Tui::new();
    tui.film(Path::new(env!("CARGO_TARGET_TMPDIR")), "fish-bubble-tail");
    spawn_frozen_bot(&mut tui);
    park_at(&mut tui, "Neo", 30.0, 12.0);
    program(&mut tui, &[&format!("/say \"{SPOKEN}\"")]);
    tui.run(DEFAULT_TRIGGER);
    tui.tick_n(SCRIPT_TICKS);

    face(&mut tui, Direction::Right);
    tui.snap("a fish facing right speaks through a / off its eye");
    expect_tail(&mut tui, 1, -1, '/', BUBBLE_BOTTOM_EDGE);

    face(&mut tui, Direction::Left);
    tui.snap("the same fish facing left speaks through a \\ off its eye");
    expect_tail(&mut tui, -1, -1, '\\', BUBBLE_BOTTOM_EDGE);

    park_at(&mut tui, "Neo", 30.0, 0.0);
    face(&mut tui, Direction::Right);
    tui.snap("no room above, so the bubble and its tail flip below");
    expect_tail(&mut tui, 1, 1, '\\', BUBBLE_TOP_EDGE);

    let report = tui.reel().flaw_report();
    assert!(report.is_empty(), "{report}");
}
