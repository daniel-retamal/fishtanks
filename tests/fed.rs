use std::path::Path;

use crossterm::event::KeyCode;
use fishtank::{
    consumable::{ConsumeTarget, apply_milk_to_fish},
    entities::food::{CANDY_GAIN_MULT, FOOD_WEIGHT_GAIN_G},
    fishes::{
        fish::{BURP, Fed, Fish},
        mutations::{Mutation, apply_mutation_to_fish},
        species::{ALL_SPECIES, EYE_ROUND, EYE_ROUND_SHUT, FishSpecies, SizeCategory},
    },
    loot::{ConsumableKind, MilkVariant, StockItem},
    tank::SensedFish,
    testing::Tui,
    ui::consume_picker::ConsumePickerSource,
};

const SIZES: [(u16, u16); 4] = [(100, 30), (60, 18), (40, 14), (28, 10)];
const WEIGHT_STEP_G: usize = 37;
const BURP_WAIT_TICKS: usize = 900;
const FULL: &str = "Full";
const GROWING: &str = "0%";

fn fish_of(species: FishSpecies, size: SizeCategory, weight_g: u32) -> Fish {
    let mut fish = Fish::new(species, "Probe".to_string(), 0.0, 0.0, &mut rand::rng());
    fish.size_category = size;
    fish.weight_g = weight_g;
    fish
}

fn cap_of(species: FishSpecies, size: SizeCategory) -> u32 {
    species.config().weight_cap[size as usize]
}

fn base_of(species: FishSpecies, size: SizeCategory) -> u32 {
    species.config().weight_base[size as usize]
}

fn capped_species() -> impl Iterator<Item = FishSpecies> {
    ALL_SPECIES.iter().copied().filter(|species| {
        let config = species.config();
        config.sellable && !config.auto_mutate && config.weight_cap.iter().all(|&cap| cap > 0)
    })
}

#[test]
fn a_fish_is_full_exactly_when_it_stops_chasing_food() {
    for species in capped_species() {
        for size in SizeCategory::ALL {
            let (base, cap) = (base_of(species, size), cap_of(species, size));
            let hungry = fish_of(species, size, base);
            assert_eq!(hungry.fed(), Fed::Growing(0), "{species:?} {size:?}");
            let full = fish_of(species, size, cap);
            assert_eq!(full.fed(), Fed::Full, "{species:?} {size:?} at its cap");
            for weight_g in (base..=cap).step_by(WEIGHT_STEP_G) {
                let fish = fish_of(species, size, weight_g);
                assert_eq!(
                    fish.fed() == Fed::Full,
                    !fish.earns_from_food(),
                    "{species:?} {size:?} at {weight_g}g: the gauge and the appetite disagree"
                );
                if let Fed::Growing(percent) = fish.fed() {
                    assert!(percent < 100, "only a full fish reads full");
                }
            }
        }
    }
}

#[test]
fn only_a_fish_whose_worth_food_can_move_has_a_gauge() {
    let mutant = fish_of(FishSpecies::Mutantfish, SizeCategory::M, 1_000_000);
    assert_eq!(
        mutant.fed(),
        Fed::Boundless,
        "a mutant's mass is worth for ever"
    );
    assert_eq!(mutant.worth_when_full(), None);
    let cheat = fish_of(FishSpecies::Cheatfish, SizeCategory::S, 100);
    assert_eq!(
        cheat.fed(),
        Fed::Worthless,
        "an unsellable fish earns nothing"
    );
    let unfish = fish_of(FishSpecies::Unfish, SizeCategory::S, 100);
    assert_eq!(unfish.fed(), Fed::Worthless);
}

#[test]
fn a_growing_fish_knows_what_it_is_worth_full() {
    let size = SizeCategory::S;
    let hungry = fish_of(
        FishSpecies::Merluza,
        size,
        base_of(FishSpecies::Merluza, size),
    );
    let full = fish_of(
        FishSpecies::Merluza,
        size,
        cap_of(FishSpecies::Merluza, size),
    );
    assert_eq!(hungry.worth_when_full(), Some(full.sell_value()));
    assert_eq!(
        full.worth_when_full(),
        None,
        "a full fish is worth what it is"
    );
}

#[test]
fn food_stops_at_the_cap_and_everything_else_that_fattens_breaks_it() {
    let mut rng = rand::rng();
    let size = SizeCategory::S;
    let cap = cap_of(FishSpecies::Merluza, size);
    let mut fish = fish_of(FishSpecies::Merluza, size, cap);
    let full_worth = fish.sell_value();

    fish.eat(FOOD_WEIGHT_GAIN_G * CANDY_GAIN_MULT);
    assert_eq!(fish.weight_g, cap, "food never fattens past the cap");
    assert_eq!(fish.sell_value(), full_worth);

    apply_milk_to_fish(MilkVariant::Chocolate, &mut fish, &mut rng);
    assert!(fish.weight_g > cap, "chocolate milk breaks the cap");
    let chocolate_worth = fish.sell_value();
    assert!(chocolate_worth > full_worth, "and its mass is worth money");
    assert_eq!(fish.fed(), Fed::Full, "food still cannot add to it");

    let weight = fish.weight_g;
    apply_mutation_to_fish(&mut fish, Mutation::SizeIncrease, &mut rng);
    assert!(fish.weight_g > weight, "sizeincrease adds a segment's mass");
    assert!(
        fish.sell_value() > chocolate_worth,
        "and that mass pays too"
    );

    let heavier = fish.weight_g;
    fish.eat(FOOD_WEIGHT_GAIN_G);
    assert_eq!(fish.weight_g, heavier, "a pellet never takes mass away");
}

#[test]
fn a_fish_burps_once_when_the_pellet_it_eats_fills_it() {
    let size = SizeCategory::S;
    let cap = cap_of(FishSpecies::Merluza, size);
    let mut fish = fish_of(FishSpecies::Merluza, size, cap - 2 * FOOD_WEIGHT_GAIN_G);

    fish.eat(FOOD_WEIGHT_GAIN_G);
    assert!(
        fish.speech.is_none(),
        "a pellet that does not fill it is just lunch"
    );
    assert!(!fish.is_sated());

    fish.eat(FOOD_WEIGHT_GAIN_G);
    let speech = fish.speech.as_ref().expect("the last pellet fills it");
    assert_eq!(speech.text, BURP);
    assert!(fish.is_sated());
    let eyes: String = fish
        .line_sprite()
        .rows
        .iter()
        .flatten()
        .map(|cell| cell.0)
        .collect();
    assert!(
        eyes.contains(EYE_ROUND_SHUT),
        "it shuts its eyes, content: {eyes}"
    );
    assert!(!eyes.contains(EYE_ROUND));

    fish.speech = None;
    fish.eat(FOOD_WEIGHT_GAIN_G);
    assert!(fish.speech.is_none(), "a full fish burps once");
}

#[test]
fn an_assay_scale_senses_a_full_fish() {
    let size = SizeCategory::S;
    let hungry = fish_of(
        FishSpecies::Merluza,
        size,
        base_of(FishSpecies::Merluza, size),
    );
    let full = fish_of(
        FishSpecies::Merluza,
        size,
        cap_of(FishSpecies::Merluza, size),
    );
    assert!(!SensedFish::of(&hungry).full);
    assert!(SensedFish::of(&full).full);
}

fn fill(tui: &mut Tui, name: &str, weight_g: impl Fn(&Fish) -> u32) {
    let fish = tui.app.tanks[0]
        .fish
        .iter_mut()
        .find(|fish| fish.name == name)
        .expect("the fish lives");
    fish.weight_g = weight_g(fish);
}

fn cap_weight(fish: &Fish) -> u32 {
    cap_of(fish.species, fish.size_category)
}

fn base_weight(fish: &Fish) -> u32 {
    base_of(fish.species, fish.size_category)
}

fn a_full_and_a_hungry_fish(tui: &mut Tui) {
    tui.clear_tank();
    tui.run("/spawn merluza \"Gus\"");
    tui.run("/spawn salmon \"Pip\"");
    fill(tui, "Gus", cap_weight);
    fill(tui, "Pip", base_weight);
}

#[test]
fn a_fish_says_so_in_the_tank_when_food_fills_it() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/spawn merluza \"Gus\"");
    fill(&mut tui, "Gus", |fish| {
        cap_weight(fish) - FOOD_WEIGHT_GAIN_G
    });
    tui.run("/feed");
    let mut waited = 0;
    while !tui.screen().contains(BURP) {
        assert!(waited < BURP_WAIT_TICKS, "Gus never burped");
        tui.tick_n(1);
        waited += 1;
    }
    let gus = &tui.app.tanks[0].fish[0];
    assert_eq!(gus.fed(), Fed::Full);
    assert!(!gus.seeks_food(), "and leaves the rest of the food alone");
}

#[test]
fn the_sell_list_shows_how_fed_every_fish_is_at_every_size() {
    for (cols, rows) in SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("fed-sell-list-{cols}x{rows}"),
        );
        a_full_and_a_hungry_fish(&mut tui);
        tui.run("/shop");
        tui.key(KeyCode::Down).key(KeyCode::Enter);
        tui.snap("the sell list with a full and a hungry fish");
        let screen = tui.screen();
        screen.expect_find(FULL);
        screen.expect_find(GROWING);
        if rows >= SIZES[0].1 {
            screen.expect_find("Fed");
            screen.expect_find("Gus (Merluza)");
        }
        assert!(
            tui.reel().flaw_report().is_empty(),
            "{}",
            tui.reel().flaw_report()
        );
    }
}

#[test]
fn a_sell_list_without_a_fish_has_no_fed_column() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.app.inventory.insert(StockItem::Junk, 3);
    tui.run("/shop");
    tui.key(KeyCode::Down).key(KeyCode::Enter);
    let screen = tui.screen();
    screen.expect_find("Junk (3)");
    screen.expect_absent("Fed");
}

#[test]
fn show_tells_what_a_fish_is_worth_and_how_fed_it_is_at_every_size() {
    for (cols, rows) in SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("fed-show-{cols}x{rows}"),
        );
        a_full_and_a_hungry_fish(&mut tui);
        tui.run("/show \"Pip\"");
        tui.snap("a hungry salmon");
        if rows >= SIZES[0].1 {
            let screen = tui.screen();
            screen.expect_find("Worth");
            screen.expect_find("when full");
            screen.expect_find("Fed");
            screen.expect_find(GROWING);
        }
        tui.key(KeyCode::Esc);
        tui.run("/show \"Gus\"");
        tui.snap("a full merluza");
        if rows >= SIZES[0].1 {
            let screen = tui.screen();
            screen.expect_find(FULL);
            screen.expect_absent("when full");
        }
        assert!(
            tui.reel().flaw_report().is_empty(),
            "{}",
            tui.reel().flaw_report()
        );
    }
}

#[test]
fn the_index_lists_worth_and_fed_at_every_size() {
    for (cols, rows) in SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("fed-index-{cols}x{rows}"),
        );
        a_full_and_a_hungry_fish(&mut tui);
        tui.run("/index");
        tui.snap("the index opens on its first columns");
        if cols >= SIZES[0].0 {
            let screen = tui.screen();
            screen.expect_find("Worth");
            screen.expect_find("Fed");
            screen.expect_find(FULL);
        }
        for _ in 0..4 {
            tui.key(KeyCode::Right);
        }
        tui.snap("the index paged to the right");
        assert!(
            tui.reel().flaw_report().is_empty(),
            "{}",
            tui.reel().flaw_report()
        );
    }
}

fn milk(tui: &mut Tui, variant: MilkVariant) {
    tui.app
        .inventory
        .insert(StockItem::Consumable(ConsumableKind::Milk(variant)), 2);
    assert!(tui.app.open_consume_picker(
        ConsumeTarget::Milk(variant),
        ConsumePickerSource::FromCommand
    ));
}

#[test]
fn chocolate_milk_shows_how_fed_each_fish_is_and_goes_down_a_full_one_too() {
    for (cols, rows) in SIZES {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("fed-chocolate-{cols}x{rows}"),
        );
        a_full_and_a_hungry_fish(&mut tui);
        let worth = tui.app.tanks[0].fish[0].sell_value();
        milk(&mut tui, MilkVariant::Chocolate);
        tui.snap("the chocolate milk picker");
        if cols >= SIZES[0].0 {
            let screen = tui.screen();
            screen.expect_find("Fed");
            screen.expect_find(FULL);
        }
        tui.key(KeyCode::Enter);
        tui.snap("a glass down the full merluza");
        let gus = &tui.app.tanks[0].fish[0];
        assert_eq!(gus.name, "Gus");
        assert!(
            gus.sell_value() > worth,
            "a full fish still fattens on chocolate"
        );
        assert!(
            tui.reel().flaw_report().is_empty(),
            "{}",
            tui.reel().flaw_report()
        );
    }
}

#[test]
fn a_milk_that_does_not_fatten_shows_no_gauge() {
    let mut tui = Tui::new();
    a_full_and_a_hungry_fish(&mut tui);
    milk(&mut tui, MilkVariant::Strawberry);
    let screen = tui.screen();
    screen.expect_find("Gus");
    screen.expect_absent("Fed");
}
