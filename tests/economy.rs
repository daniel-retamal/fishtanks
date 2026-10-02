use std::path::{Path, PathBuf};

use crossterm::event::KeyCode;
use fishtank::{
    app::{App, Launch},
    consumable::{CASTS_PER_BUFF, COFFEE_DURATION},
    economy::{Money, Purse, Rarity},
    entities::food::FOOD_BUY_PRICE,
    fishes::{
        fish::Fish,
        fused::FusedComponent,
        mutations::{Mutation, apply_mutation_to_fish},
        parts::{Part, RIG_WAIT_MAX_SECS, RIG_WAIT_MIN_SECS},
        species::{ALL_SPECIES, FishSpecies, SizeCategory, pellets_to_cap},
    },
    ledger::{Direction, Flow},
    loot::{CashValue, ConsumableKind, LootKind, StockItem},
    settings::Settings,
    settings::{DEFAULT_FPS, DEFAULT_STAGES_PER_TICK},
    tank::{Tank, TankEvent, TankKind},
    testing::{Reel, Still, Tui},
    ui::{
        catch_overlay::{CatchOverlay, CatchState},
        layout::Screen,
    },
};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

const SECS_PER_MINUTE: f32 = 60.0;
const CASHFISH_SHOAL: usize = 20;
const CASHFISH_MINUTES: f32 = 10.0;
const FRAME_RATES: [f32; 3] = [1.0, 30.0, 120.0];
const INCOME_TOLERANCE: f64 = 0.2;
const MONEY_DOORS: [&str; 2] = ["src/economy.rs", "src/app/money.rs"];
const PURSE_MUTATIONS: [&str; 3] = ["purse.earn(", "purse.spend(", "purse.lose("];

fn lab() -> App {
    let mut app = App::launch(Launch::Debug);
    let tank = &mut app.tanks[0];
    tank.fish.clear();
    tank.used_names.clear();
    app
}

fn cashfish_income(fps: f32) -> Money {
    let mut app = lab();
    for n in 0..CASHFISH_SHOAL {
        app.tanks[0].spawn_fish(FishSpecies::Cashfish, format!("Coin{n}"), &mut rand::rng());
    }
    app.settings.fps = fps;
    let frames = (CASHFISH_MINUTES * SECS_PER_MINUTE * fps) as usize;
    for _ in 0..frames {
        app.tick();
    }
    app.ledger
        .since_launch()
        .line(Direction::In, Flow::Cashfish)
}

#[test]
fn a_cashfish_earns_the_same_per_game_minute_at_any_frame_rate() {
    let incomes: Vec<Money> = FRAME_RATES.into_iter().map(cashfish_income).collect();
    let reference = incomes[1] as f64;
    assert!(reference > 0.0, "a shoal of Cashfish pays in ten minutes");
    for (fps, income) in FRAME_RATES.into_iter().zip(&incomes) {
        let drift = (*income as f64 - reference).abs() / reference;
        assert!(
            drift < INCOME_TOLERANCE,
            "at {fps} fps a Cashfish shoal earned ${income}, at 30 fps ${reference}: {incomes:?}"
        );
    }
}

#[test]
fn a_botfish_circuit_steps_as_many_stages_per_frame_at_any_frame_rate() {
    for fps in FRAME_RATES {
        let mut tui = Tui::new();
        tui.clear_tank();
        tui.run("/spawn botfish \"Coil\"");
        tui.app.settings.fps = fps;
        assert_eq!(
            tui.app.tick_fabric(),
            DEFAULT_STAGES_PER_TICK,
            "at {fps} fps"
        );
    }
}

#[test]
fn a_purse_past_the_old_ceiling_keeps_counting() {
    let mut app = lab();
    app.purse = Purse::holding(Money::from(u32::MAX));
    app.earn(1u32, Flow::Godsend);
    assert_eq!(app.purse.balance(), Money::from(u32::MAX) + 1);
}

fn rust_sources(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a source directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            rust_sources(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
}

#[test]
fn earn_and_pay_are_the_only_doors_to_the_purse() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut sources = Vec::new();
    rust_sources(&root.join("src"), &mut sources);
    for path in sources {
        let relative = path
            .strip_prefix(root)
            .expect("under the crate")
            .to_string_lossy()
            .replace('\\', "/");
        if MONEY_DOORS.contains(&relative.as_str()) {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("a readable source");
        for mutation in PURSE_MUTATIONS {
            assert!(
                !text.contains(mutation),
                "{relative} moves money with `{mutation}` instead of App::earn/App::pay"
            );
        }
    }
}

#[test]
fn every_coin_that_moves_lands_on_a_ledger_line() {
    let mut tui = Tui::new();
    tui.stake();
    let before = tui.app.purse.balance();
    let ledger_before = net(&tui.app);
    tui.run("/buy food 10");
    tui.run("/buy coffee 2");
    tui.run("/sell fish \"Adam\"");
    tui.run("/sell coffee 1");
    let moved = tui.app.purse.balance() as i128 - before as i128;
    assert_eq!(net(&tui.app) - ledger_before, moved);
    let statement = tui.app.ledger.since_launch();
    assert_eq!(statement.line(Direction::Out, Flow::Food), 10);
    assert!(statement.line(Direction::In, Flow::FishSales) > 0);
    assert!(statement.line(Direction::In, Flow::StockSales) > 0);
    assert!(statement.line(Direction::Out, Flow::Coffee) > 0);
}

fn net(app: &App) -> i128 {
    let statement = app.ledger.since_launch();
    statement.total(Direction::In) as i128 - statement.total(Direction::Out) as i128
}

const REEL_TICKS: usize = 240;
const CATCH_CARD_HINTS: [&str; 2] = ["ENTER capture", "ESC/q close"];
const KEEPER: &str = "Kept";

fn land_a_cast(tui: &mut Tui) {
    tui.run("/fish --no-fight");
    let card = |tui: &mut Tui| {
        let screen = tui.screen();
        CATCH_CARD_HINTS.iter().any(|hint| screen.contains(hint))
    };
    for _ in 0..REEL_TICKS {
        tui.key(KeyCode::Down);
        tui.tick_n(1);
        if tui.app.fishing_state().is_none() {
            break;
        }
    }
    assert!(
        card(tui),
        "the reel landed a catch: {}",
        tui.screen().text()
    );
    tui.release(KeyCode::Down);
    tui.type_text(KEEPER);
    tui.key(KeyCode::Enter);
    assert!(!card(tui), "the card closed");
}

fn bar(tui: &mut Tui) -> String {
    tui.screen().text()
}

#[test]
fn every_bait_is_its_own_stack_and_lasts_its_own_casts() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/add bait 3");
    tui.run("/consume bait");
    tui.run("/consume bait");
    assert!(bar(&mut tui).contains("baiting II: 5 casts"));

    land_a_cast(&mut tui);
    tui.run("/consume bait");
    assert!(bar(&mut tui).contains("baiting III: 4 casts"));

    for _ in 1..CASTS_PER_BUFF {
        land_a_cast(&mut tui);
    }
    assert!(
        bar(&mut tui).contains("baiting I: 1 cast"),
        "the first two baits went together after {CASTS_PER_BUFF} casts"
    );
    land_a_cast(&mut tui);
    assert!(
        !bar(&mut tui).contains("baiting"),
        "three baits bought six casts"
    );
}

#[test]
fn every_cup_of_coffee_lasts_its_own_full_minute() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/add coffee 2");
    tui.run("/consume coffee");
    let half = (COFFEE_DURATION * DEFAULT_FPS / 2.0) as usize;
    tui.tick_n(half);
    tui.run("/consume coffee");
    assert!(
        bar(&mut tui).contains("caffeinated II: 00:"),
        "{}",
        bar(&mut tui)
    );
    tui.tick_n(half + 1);
    assert!(
        bar(&mut tui).contains("caffeinated I: 00:"),
        "the first cup ran out, the second has half a minute left: {}",
        bar(&mut tui)
    );
    tui.tick_n(half + 1);
    assert!(!bar(&mut tui).contains("caffeinated"));
}

#[test]
fn a_glass_of_fishing_milk_lasts_its_casts_and_not_a_second_longer() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/add blueberry milk 1");
    tui.run("/consume blueberry milk");
    tui.tick_n((COFFEE_DURATION * DEFAULT_FPS) as usize * 3);
    assert!(
        bar(&mut tui).contains("visual-calculus I: 5 casts"),
        "a paused rod spends nothing"
    );
    for _ in 0..CASTS_PER_BUFF {
        land_a_cast(&mut tui);
    }
    assert!(!bar(&mut tui).contains("visual-calculus"));
}

const RIG_TICKS: usize = 600;

fn bot<'a>(app: &'a mut App, name: &str) -> &'a mut fishtank::fishes::botfish::BotfishState {
    app.tanks[0]
        .fish
        .iter_mut()
        .find(|f| f.name == name)
        .and_then(|f| f.script_mut())
        .expect("a botfish")
}

#[test]
fn a_rig_spends_the_bait_it_is_helped_by_and_never_the_milk_it_is_not() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/fps 1");
    tui.run("/add bait 1");
    tui.run("/consume bait");
    tui.run("/add blueberry milk 1");
    tui.run("/consume blueberry milk");
    tui.app.inventory.insert(StockItem::BAIT, 50);
    tui.app.tanks[0].spawn_fish(FishSpecies::Botfish, "Rod".to_string(), &mut rand::rng());
    let rod = bot(&mut tui.app, "Rod");
    rod.listen("go");
    rod.drive("go");
    assert!(rod.install(Part::InverterCoil));
    assert!(rod.install(Part::AnglerRig));
    rod.wire(Part::AnglerRig, "cast", "go");
    for _ in 0..RIG_TICKS {
        tui.tick_n(1);
        if tui.app.active_consumables.is_empty() {
            break;
        }
    }
    let screen = bar(&mut tui);
    assert!(!screen.contains("baiting"), "{screen}");
    assert!(screen.contains("visual-calculus I: 5 casts"), "{screen}");
}

const FILM_SIZES: [(u16, u16); 4] = [(100, 30), (60, 18), (40, 14), (28, 10)];

#[test]
fn every_buff_on_the_status_bar_at_every_size() {
    for (cols, rows) in FILM_SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("economy-buffs-{cols}x{rows}"),
        );
        tui.clear_tank();
        tui.run("/add bait 3");
        tui.run("/add coffee 2");
        tui.run("/add matcha milk 1");
        tui.run("/consume bait");
        tui.run("/consume bait");
        tui.run("/consume coffee");
        tui.run("/consume matcha milk");
        tui.snap("two baits, a cup and a glass of matcha");
        let screen = tui.screen().text();
        assert!(screen.contains("5"), "{screen}");
        assert!(
            tui.reel().flaw_report().is_empty(),
            "{}",
            tui.reel().flaw_report()
        );
    }
}

fn fish_of(species: FishSpecies, size: SizeCategory, weight_g: u32) -> Fish {
    let mut fish = Fish::new(species, "Probe".to_string(), 0.0, 0.0, &mut rand::rng());
    fish.size_category = size;
    fish.weight_g = weight_g;
    fish
}

fn fed_species() -> impl Iterator<Item = FishSpecies> {
    ALL_SPECIES.iter().copied().filter(|species| {
        let config = species.config();
        config.sellable && !config.auto_mutate && config.weight_cap.iter().all(|&cap| cap > 0)
    })
}

#[test]
fn feeding_a_fish_to_its_cap_always_pays_small_fish_in_percent_big_fish_in_dollars() {
    let pellets = pellets_to_cap();
    for species in fed_species() {
        let config = species.config();
        let mut last: Option<(f64, Money)> = None;
        for size in SizeCategory::ALL {
            let i = size as usize;
            let mut fish = fish_of(species, size, config.weight_base[i]);
            let base = fish.sell_value();
            fish.weight_g = config.weight_cap[i];
            let cap = fish.sell_value();
            let food = Money::from(pellets[i] * FOOD_BUY_PRICE);
            assert!(
                cap - base > food,
                "{} {size:?} loses money fed",
                config.name
            );
            let profit = cap - base - food;
            let roi = profit as f64 / food as f64;
            if let Some((last_roi, last_profit)) = last {
                assert!(
                    roi < last_roi,
                    "{} {size:?}: the percentage falls",
                    config.name
                );
                assert!(
                    profit > last_profit,
                    "{} {size:?}: the dollars rise",
                    config.name
                );
            }
            last = Some((roi, profit));
        }
    }
}

#[test]
fn a_bought_fish_fattened_never_nets_its_price() {
    let pellets = pellets_to_cap();
    for species in FishSpecies::all_buyable().iter().copied() {
        let config = species.config();
        let size = SizeCategory::M;
        let i = size as usize;
        let fattened = fish_of(species, size, config.weight_cap[i]).sell_value();
        let food = Money::from(pellets[i] * FOOD_BUY_PRICE);
        assert!(
            fattened - food < Money::from(species.buy_price()),
            "a bought {} fattened nets {} against its ${} price",
            config.name,
            fattened - food,
            species.buy_price()
        );
    }
}

#[test]
fn a_fish_that_cannot_earn_from_food_does_not_chase_it() {
    let config = FishSpecies::Merluza.config();
    let hungry = fish_of(FishSpecies::Merluza, SizeCategory::S, config.weight_base[0]);
    assert!(hungry.seeks_food());
    let full = fish_of(FishSpecies::Merluza, SizeCategory::S, config.weight_cap[0]);
    assert!(!full.seeks_food(), "a fish at its cap cannot earn more");
    let cheat = fish_of(FishSpecies::Cheatfish, SizeCategory::S, 100);
    assert!(!cheat.seeks_food(), "an unsellable fish earns nothing");
    let mutant = fish_of(FishSpecies::Mutantfish, SizeCategory::M, 1_000_000);
    assert!(mutant.seeks_food(), "a mutant's weight is worth for ever");
    let candy = fish_of(FishSpecies::Candyfish, SizeCategory::S, 100);
    assert!(candy.seeks_food(), "a candyfish chases food to candy it");

    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/spawn cheatfish \"Pride\"");
    tui.run("/spawn merluza \"Glutton\"");
    let weight = |tui: &Tui, name: &str| {
        tui.app.tanks[0]
            .fish
            .iter()
            .find(|f| f.name == name)
            .map(|f| f.weight_g)
            .expect("the fish lives")
    };
    let (pride, glutton) = (weight(&tui, "Pride"), weight(&tui, "Glutton"));
    tui.run("/feed");
    tui.tick_n(DEFAULT_FPS as usize * 30);
    assert_eq!(
        weight(&tui, "Pride"),
        pride,
        "the cheatfish left the food alone"
    );
    assert!(weight(&tui, "Glutton") > glutton, "the merluza ate it all");
}

fn worth_inputs(fish: &Fish) -> (FishSpecies, SizeCategory, u32, u32) {
    (
        fish.species,
        fish.size_category,
        fish.weight_g,
        fish.sell_price_bonus_pct,
    )
}

#[test]
fn a_mutation_is_worth_money_only_through_what_a_fish_is_worth_by() {
    let mut rng = rand::rng();
    for species in [
        FishSpecies::Merluza,
        FishSpecies::Mutantfish,
        FishSpecies::Koi,
    ] {
        for &mutation in Mutation::ALL {
            for _ in 0..8 {
                let mut fish = fish_of(species, SizeCategory::M, 3_000);
                apply_mutation_to_fish(&mut fish, Mutation::BodyColor, &mut rng);
                let (before, worth) = (worth_inputs(&fish), fish.sell_value());
                apply_mutation_to_fish(&mut fish, mutation, &mut rng);
                if worth_inputs(&fish) == before {
                    assert_eq!(
                        fish.sell_value(),
                        worth,
                        "{mutation:?} changed a {species:?}'s worth without changing its weight"
                    );
                }
            }
        }
    }
}

#[test]
fn sizeincrease_grows_a_fish_by_one_segment_of_its_mass() {
    let mut rng = rand::rng();
    let mut fish = fish_of(FishSpecies::Mutantfish, SizeCategory::M, 6_000);
    apply_mutation_to_fish(&mut fish, Mutation::BodyColor, &mut rng);
    let body = fish.body_size as u32;
    let worth = fish.sell_value();
    apply_mutation_to_fish(&mut fish, Mutation::SizeIncrease, &mut rng);
    assert_eq!(fish.body_size as u32, body + 1);
    assert_eq!(fish.weight_g, 6_000 + 6_000 / body);
    assert!(
        fish.sell_value() > worth,
        "a Mutantfish's mass is its worth"
    );

    let before = fish.weight_g;
    apply_mutation_to_fish(&mut fish, Mutation::SizeDecrease, &mut rng);
    assert_eq!(fish.weight_g, before, "growth never runs backwards");
}

#[test]
fn a_body_at_its_largest_or_a_fixed_body_gains_no_mass() {
    let mut rng = rand::rng();
    let mut grown = fish_of(FishSpecies::Merluza, SizeCategory::S, 1_000);
    for _ in 0..40 {
        apply_mutation_to_fish(&mut grown, Mutation::SizeIncrease, &mut rng);
    }
    let capped = grown.weight_g;
    apply_mutation_to_fish(&mut grown, Mutation::SizeIncrease, &mut rng);
    assert_eq!(
        grown.weight_g, capped,
        "the body stopped growing, so did the mass"
    );

    let mut fixed = fish_of(FishSpecies::Anchoveta, SizeCategory::S, 1_000);
    apply_mutation_to_fish(&mut fixed, Mutation::SizeIncrease, &mut rng);
    assert_eq!(
        fixed.weight_g, 1_000,
        "a fixed sprite has no segment to add"
    );
}

#[test]
fn a_fed_catch_is_worth_what_its_rarity_says() {
    assert_eq!(
        [Rarity::Common, Rarity::Rare, Rarity::Legendary].map(Rarity::catch_worth),
        [101, 282, 1_837]
    );
}

#[test]
fn no_found_item_resells_above_a_fed_catch_of_its_rarity() {
    for kind in ConsumableKind::all() {
        assert!(
            kind.sell_price() <= kind.rarity().catch_worth(),
            "{} resells for ${}, above the ${} a fed {:?} catch is worth",
            kind.display_name(),
            kind.sell_price(),
            kind.rarity().catch_worth(),
            kind.rarity()
        );
    }
}

#[test]
fn a_grown_tank_never_sells_above_its_seed_and_a_bought_one_sells_at_half() {
    for seed in ConsumableKind::seeds() {
        let tank = seed.summons_tank().expect("a seed grows a tank");
        assert!(
            tank.sell_price() <= seed.sell_price(),
            "{} sells above its seed",
            tank.display_name()
        );
    }
    for tank in TankKind::all_buyable() {
        assert_eq!(
            tank.sell_price(),
            tank.buy_price() / 2,
            "{}",
            tank.display_name()
        );
    }
    assert!(TankKind::Alien.sell_price() <= Rarity::Rare.catch_worth());
}

#[test]
fn no_legendary_is_on_the_shops_fish_page() {
    for species in FishSpecies::all_buyable() {
        assert_ne!(
            species.config().rarity,
            Rarity::Legendary,
            "{} is a Legendary: caught on its banner, never bought",
            species.display_name()
        );
    }
    for species in [
        FishSpecies::Cashfish,
        FishSpecies::Mutantfish,
        FishSpecies::Botfish,
    ] {
        assert!(
            species.is_obtainable(),
            "{} is still caught",
            species.display_name()
        );
    }
}

#[test]
fn a_bot_cannot_buy_a_legendary_either() {
    let mut tui = Tui::new();
    tui.stake();
    let fish = tui.app.tanks[0].fish.len();
    tui.run("/buy botfish");
    assert_eq!(tui.app.tanks[0].fish.len(), fish);
    assert!(!tui.screen().contains("Botfish for sale"));
}

const RIG_WATCH_SECS: usize = 400;
const CAST_RISE_SECS: f32 = 2.0;

#[test]
fn a_rig_always_lands_a_fish_and_never_quickly() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/fps 1");
    tui.app.inventory.insert(StockItem::BAIT, 50);
    tui.app.tanks[0].spawn_fish(FishSpecies::Botfish, "Rod".to_string(), &mut rand::rng());
    let rod = bot(&mut tui.app, "Rod");
    rod.listen("go");
    rod.drive("go");
    assert!(rod.install(Part::InverterCoil));
    assert!(rod.install(Part::AnglerRig));
    rod.wire(Part::AnglerRig, "cast", "go");
    let stock_but_bait = |tui: &Tui| -> u32 {
        tui.app
            .inventory
            .iter()
            .filter(|(item, _)| **item != StockItem::BAIT)
            .map(|(_, qty)| qty)
            .sum()
    };
    let (cash, food, stock) = (
        tui.app.purse.balance(),
        tui.app.food_supply,
        stock_but_bait(&tui),
    );
    let mut shoal = tui.app.tanks[0].fish.len();
    let mut last = 0.0_f32;
    let mut landings = 0;
    for second in 1..=RIG_WATCH_SECS {
        tui.tick_n(1);
        let now = tui.app.tanks[0].fish.len();
        if now == shoal {
            continue;
        }
        assert_eq!(now, shoal + 1, "one cast, one fish");
        let waited = second as f32 - last;
        assert!(
            (RIG_WAIT_MIN_SECS..=RIG_WAIT_MAX_SECS + CAST_RISE_SECS).contains(&waited),
            "a landing after {waited} game seconds"
        );
        let caught = tui.app.tanks[0].fish.last().expect("the catch");
        assert!(
            caught.name.starts_with(caught.species.display_name()),
            "a rig's catch is a fish, named the way a bot names one: {}",
            caught.name
        );
        (shoal, last) = (now, second as f32);
        landings += 1;
    }
    assert!(landings >= 5, "{landings} landings in {RIG_WATCH_SECS} s");
    assert_eq!(
        (
            tui.app.purse.balance(),
            tui.app.food_supply,
            stock_but_bait(&tui)
        ),
        (cash, food, stock),
        "a rig brings up nothing but fish"
    );
}

#[test]
fn a_rig_with_no_room_anywhere_keeps_its_line_down_and_spends_nothing() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/fps 1");
    tui.app.inventory.insert(StockItem::BAIT, 50);
    tui.app.tanks[0].spawn_fish(FishSpecies::Botfish, "Rod".to_string(), &mut rand::rng());
    let rod = bot(&mut tui.app, "Rod");
    rod.listen("go");
    rod.drive("go");
    assert!(rod.install(Part::InverterCoil));
    assert!(rod.install(Part::AnglerRig));
    rod.wire(Part::AnglerRig, "cast", "go");
    let capacity = tui.app.tanks[0].capacity();
    for n in tui.app.tanks[0].fish.len()..capacity {
        tui.app.tanks[0].spawn_fish(FishSpecies::Merluza, format!("Filler{n}"), &mut rand::rng());
    }
    tui.tick_n(RIG_WATCH_SECS / 2);
    assert_eq!(
        tui.app.inventory.get(&StockItem::BAIT),
        Some(&50),
        "no bait went down"
    );
    assert_eq!(tui.app.tanks[0].fish.len(), capacity);
}

fn stacked(species: FishSpecies, stacks: usize, name: &str, weight_g: u32) -> Fish {
    let mut rng = rand::rng();
    let mut fish = Fish::new(species, name.to_string(), 20.0, 8.0, &mut rng);
    apply_mutation_to_fish(&mut fish, Mutation::BodyColor, &mut rng);
    fish.weight_g = weight_g;
    fish.set_fused(
        (0..stacks)
            .map(|_| FusedComponent::fish(species, name.to_string(), weight_g))
            .collect(),
    );
    fish
}

fn pen(fish: Vec<Fish>) -> Tank {
    let mut tank = Tank::new("Pen".to_string(), TankKind::Base, &[]);
    for mut one in fish {
        let (x, y) = (one.position.x, one.position.y);
        one.frozen = true;
        let name = one.name.clone();
        tank.place_fish(one, name, &mut rand::rng());
        let placed = tank.fish.last_mut().expect("placed");
        placed.position.x = x;
        placed.position.y = y;
    }
    tank
}

#[test]
fn a_four_stack_fish_engulfing_a_three_stack_one_carries_seven_and_every_gram() {
    let mut tank = pen(vec![
        stacked(FishSpecies::Cashfish, 4, "Big", 4_000),
        stacked(FishSpecies::Cashfish, 3, "Small", 3_000),
    ]);
    assert!(tank.apply_named_mutation("Big", "engulfment"));
    let settings = Settings::default();
    for _ in 0..3 {
        tank.tick(&settings, 0);
    }
    assert_eq!(tank.fish.len(), 1, "one swallowed the other");
    let fused = &tank.fish[0];
    assert_eq!(fused.ability_stacks(FishSpecies::Cashfish), 7);
    assert_eq!(fused.weight_g, 7_000, "fusing throws no food away");
}

#[test]
fn engulfing_again_and_again_never_caps_the_stacks() {
    let mut tank = pen(vec![stacked(FishSpecies::Cashfish, 2, "Glutton", 1_000)]);
    let settings = Settings::default();
    for meal in 0..3 {
        let name = format!("Meal{meal}");
        let spot = tank.fish[0].position.clone();
        let mut prey = stacked(FishSpecies::Cashfish, 2, &name, 1_000);
        prey.frozen = true;
        tank.place_fish(prey, name.clone(), &mut rand::rng());
        tank.fish.last_mut().expect("placed").position = spot;
        let host = tank.fish[0].name.clone();
        assert!(tank.apply_named_mutation(&host, "engulfment"));
        tank.tick(&settings, 0);
        assert_eq!(tank.fish.len(), 1, "meal {meal} was swallowed");
        tank.apply_named_mutation(&tank.fish[0].name.clone(), "endocytosis");
    }
    assert_eq!(tank.fish[0].ability_stacks(FishSpecies::Cashfish), 8);
}

#[test]
fn a_split_child_is_its_parents_species() {
    let mut tank = pen(vec![stacked(FishSpecies::Cashfish, 0, "Mother", 2_000)]);
    assert!(tank.apply_named_mutation("Mother", "telophase"));
    assert!(tank.apply_named_mutation("Mother", "cytokinesis"));
    assert_eq!(tank.fish.len(), 2, "the double split");
    assert!(
        tank.fish
            .iter()
            .all(|fish| fish.species == FishSpecies::Cashfish),
        "a Cashfish breeds Cashfish"
    );
}

#[test]
fn a_holyfish_blesses_once_for_every_holyfish_it_carries() {
    let mut tank = pen(vec![stacked(FishSpecies::Holyfish, 3, "Saint", 1_000)]);
    tank.fish[0].blessing_timer = 0.0;
    let events = tank.tick(&Settings::default(), 0);
    let blessings = events
        .iter()
        .filter(|event| matches!(event, TankEvent::Blessing))
        .count();
    assert_eq!(blessings, 3);
}

#[test]
fn a_million_dollar_catch_card_draws_whole_at_every_size() {
    let mut reel = Reel::new();
    for (cols, rows) in FILM_SIZES {
        for jackpot in [
            CashValue::TenThousand,
            CashValue::HundredThousand,
            CashValue::Million,
        ] {
            let state = CatchState::new(LootKind::Cash(jackpot), &mut rand::rng());
            let area = Rect::new(0, 0, cols, rows);
            let mut buffer = Buffer::empty(area);
            CatchOverlay::new(&state, Screen::only(area)).render(area, &mut buffer);
            let still = Still::of(format!("${} · {cols}×{rows}", jackpot.amount()), &buffer);
            assert!(
                still.text().contains(&jackpot.amount().to_string()),
                "the card names its sum:
{}",
                still.text()
            );
            reel.push(still);
        }
    }
    reel.save(
        Path::new(env!("CARGO_TARGET_TMPDIR")),
        "economy-jackpot-cards",
    )
    .expect("the reel is writable");
    let report = reel.flaw_report();
    assert!(report.is_empty(), "{report}");
}

#[test]
fn strawberry_has_no_ceiling() {
    let mut rng = rand::rng();
    let mut fish = fish_of(FishSpecies::Merluza, SizeCategory::M, 3_000);
    let worth = fish.sell_value();
    for _ in 0..20 {
        apply_mutation_to_fish(&mut fish, Mutation::Strawberry, &mut rng);
    }
    assert_eq!(fish.sell_price_bonus_pct, 500, "twenty glasses, 500%");
    assert_eq!(fish.sell_value(), worth * 6);
}

const MUTANT_SHOAL: usize = 40;
const MUTANT_WATCH_SECS: usize = 20 * 60;

#[test]
fn the_first_auto_mutation_is_timed_from_the_mutant_count() {
    let mut tank = Tank::new("Lab".to_string(), TankKind::Base, &[]);
    tank.expand(MUTANT_SHOAL as u32);
    for n in 0..MUTANT_SHOAL {
        tank.spawn_fish(FishSpecies::Mutantfish, format!("M{n}"), &mut rand::rng());
    }
    let history = |tank: &Tank| -> (usize, u32) {
        (
            tank.fish.len(),
            tank.fish.iter().map(Fish::mutation_count).sum(),
        )
    };
    let before = history(&tank);
    let settings = Settings {
        fps: 1.0,
        ..Settings::default()
    };
    let mutated = (0..MUTANT_WATCH_SECS).position(|_| {
        tank.tick(&settings, 0);
        history(&tank) != before
    });
    assert!(
        mutated.is_some(),
        "{MUTANT_SHOAL} mutants mutate long before the lone mutant's half hour"
    );
}

#[test]
fn the_ledger_reads_whole_at_every_size() {
    for (cols, rows) in FILM_SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("economy-ledger-{cols}x{rows}"),
        );
        tui.run("/ledger");
        tui.snap("an empty statement");
        tui.key(KeyCode::Esc);
        tui.stake();
        tui.run("/buy food 100");
        tui.run("/buy bait 3");
        tui.run("/buy coffee 2");
        tui.tick_n(DEFAULT_FPS as usize * 90);
        tui.run("/sell fish \"Adam\"");
        tui.run("/sell coffee 1");
        tui.leave_debug_mode();
        tui.run("/ledger");
        tui.snap("a statement after shopping and a sale, as a player");
        let screen = tui.screen();
        assert!(screen.contains("Ledger#show"));
        assert!(
            !screen.contains("Godsend"),
            "a player never sees the lab's money"
        );
        for _ in 0..12 {
            tui.key(KeyCode::Down);
        }
        tui.snap("scrolled to the bottom");
        assert!(tui.screen().contains("Net"), "{}", tui.screen().text());
        tui.key(KeyCode::Esc);
        assert!(!tui.screen().contains("Ledger#show"));
        assert!(
            tui.reel().flaw_report().is_empty(),
            "{}",
            tui.reel().flaw_report()
        );
    }
}

#[test]
fn feed_is_offered_bare_on_the_command_bar() {
    let mut tui = Tui::as_player(100, 30);
    let prompt = |tui: &mut Tui| -> String {
        tui.screen()
            .rows()
            .iter()
            .find(|row| row.starts_with('>'))
            .cloned()
            .unwrap_or_default()
    };
    tui.type_text("/fee");
    let line = prompt(&mut tui);
    assert_eq!(
        line.trim_end(),
        "> /feed",
        "the ghost completes the bare command"
    );
    tui.type_text("d ");
    let line = prompt(&mut tui);
    assert_eq!(
        line.trim_end(),
        "> /feed",
        "nothing is suggested after /feed"
    );
}
