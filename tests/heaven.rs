use crossterm::event::KeyCode;
use fishtank::{
    economy::Rarity,
    fishes::fish::Fish,
    fishes::species::FishSpecies,
    loot::StockItem,
    tank::{Tank, TankKind},
    testing::Tui,
};

const HEAVEN: &str = "Heaventank";
const HOME: &str = "Fishtank";
const DEAD: &str = "Dead";
const ALIVE: &str = "Alive";

fn heaven(tui: &Tui) -> Option<&Tank> {
    tui.app.tanks.iter().find(|t| t.kind == TankKind::Heaven)
}

fn heavens(tui: &Tui) -> usize {
    tui.app
        .tanks
        .iter()
        .filter(|t| t.kind == TankKind::Heaven)
        .count()
}

fn pearls(tui: &Tui) -> u32 {
    tui.app
        .inventory
        .get(&StockItem::GOLDEN_PEARL)
        .copied()
        .unwrap_or(0)
}

fn soul_names(tui: &Tui) -> Vec<String> {
    heaven(tui)
        .map(|t| t.souls().iter().map(|f| f.name.clone()).collect())
        .unwrap_or_default()
}

fn living(tui: &Tui, name: &str) -> Option<String> {
    tui.app
        .tanks
        .iter()
        .find(|t| t.fish.iter().any(|f| f.name == name))
        .map(|t| t.name.clone())
}

fn open_heaven(tui: &mut Tui) {
    tui.run("/give golden pearl");
    tui.run("/consume golden pearl");
    tui.type_text(HEAVEN);
    tui.key(KeyCode::Enter);
    tui.run(&format!("/switch \"{HOME}\""));
}

fn home_with(names: &[&str]) -> Tui {
    let mut tui = Tui::new();
    tui.clear_tank();
    open_heaven(&mut tui);
    for name in names {
        tui.run(&format!("/spawn salmon \"{name}\""));
    }
    tui
}

#[test]
fn a_death_opens_no_heaven_and_the_pearl_that_opens_one_hangs_every_waiting_soul() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/spawn salmon \"Ann\"");
    tui.run("/sell fish \"Ann\"");
    assert!(
        heaven(&tui).is_none(),
        "heaven is opened by a pearl, never by a death"
    );
    assert!(tui.app.graveyard.iter().any(|f| f.name == "Ann"));

    open_heaven(&mut tui);
    assert_eq!(soul_names(&tui), ["Ann"], "the dead were waiting for it");
}

#[test]
fn selling_a_fish_hangs_it_on_the_heaven_wall() {
    let mut tui = home_with(&["Ann"]);

    tui.run("/sell fish \"Ann\"");

    assert_eq!(soul_names(&tui), ["Ann"]);
    assert_eq!(
        tui.app.tanks[tui.app.current_tank].name, HOME,
        "you stay where you are"
    );
    assert!(
        heaven(&tui).unwrap().fish.is_empty(),
        "a soul is not a living fish"
    );
    assert!(living(&tui, "Ann").is_none());
}

#[test]
fn a_kill_wish_and_a_kill_command_are_the_same_death() {
    let mut tui = home_with(&["Ann"]);
    tui.run("/kill \"Ann\"");
    assert_eq!(soul_names(&tui), ["Ann"]);
    assert!(tui.app.graveyard.iter().any(|f| f.name == "Ann"));
}

#[test]
fn a_fish_swallowed_for_good_goes_to_heaven_too() {
    let mut tui = home_with(&[]);
    let lost = Fish::new(
        FishSpecies::Salmon,
        "Lost".to_string(),
        0.0,
        0.0,
        &mut rand::rng(),
    );
    tui.app.tanks[0].pending_graveyard.push(lost);
    tui.tick_n(1);
    assert_eq!(soul_names(&tui), ["Lost"]);
}

#[test]
fn there_is_only_ever_one_heaventank() {
    let mut tui = home_with(&["Ann", "Bob"]);
    tui.run("/kill \"Ann\"");
    tui.run("/kill \"Bob\"");
    assert_eq!(heavens(&tui), 1);
    assert_eq!(soul_names(&tui), ["Ann", "Bob"]);
}

#[test]
fn a_fish_the_devil_marked_never_hangs_in_heaven_and_can_still_be_revived() {
    let mut tui = home_with(&["Ann"]);
    tui.app.tanks[0].fish[0].devil_marked = true;
    tui.run("/kill \"Ann\"");
    assert!(soul_names(&tui).is_empty(), "a damned soul is not let in");
    assert!(tui.app.graveyard.iter().any(|f| f.name == "Ann"));

    tui.run("/revive \"Ann\"");
    assert_eq!(living(&tui, "Ann").as_deref(), Some(HOME));
}

#[test]
fn a_damned_soul_never_hangs_on_a_wall_that_already_exists() {
    let mut tui = home_with(&["Ann", "Bob"]);
    tui.run("/kill \"Ann\"");
    tui.app.tanks[0].fish[0].devil_marked = true;
    tui.run("/kill \"Bob\"");
    assert_eq!(soul_names(&tui), ["Ann"]);
}

#[test]
fn a_revived_fish_leaves_the_wall() {
    let mut tui = home_with(&["Ann"]);
    tui.run("/kill \"Ann\"");
    tui.run("/revive \"Ann\"");
    assert!(soul_names(&tui).is_empty());
    assert_eq!(living(&tui, "Ann").as_deref(), Some(HOME));
}

#[test]
fn a_fish_revived_while_you_stand_in_heaven_lands_in_the_next_tank() {
    let mut tui = home_with(&["Ann"]);
    tui.run("/kill \"Ann\"");
    tui.run(&format!("/switch \"{HEAVEN}\""));
    tui.run("/revive \"Ann\"");
    assert_eq!(living(&tui, "Ann").as_deref(), Some(HOME));
    assert!(heaven(&tui).unwrap().fish.is_empty());
}

#[test]
fn only_a_holy_fish_is_born_in_heaven() {
    let mut tui = home_with(&["Ann"]);
    tui.run("/kill \"Ann\"");
    tui.run(&format!("/switch \"{HEAVEN}\""));

    tui.run("/spawn salmon \"Sal\"");
    assert!(living(&tui, "Sal").is_none(), "no fish spawns in heaven");

    tui.run("/spawn holyfish \"Gabriel\"");
    assert_eq!(living(&tui, "Gabriel").as_deref(), Some(HEAVEN));
}

#[test]
fn no_fish_moves_into_heaven_but_a_holy_one_and_a_holy_one_may_leave() {
    let mut tui = home_with(&["Ann", "Sal"]);
    tui.run("/kill \"Ann\"");
    tui.run(&format!("/move \"Sal\" \"{HEAVEN}\""));
    assert_eq!(living(&tui, "Sal").as_deref(), Some(HOME));

    tui.run("/spawn holyfish \"Gabriel\"");
    tui.run(&format!("/move \"Gabriel\" \"{HEAVEN}\""));
    assert_eq!(living(&tui, "Gabriel").as_deref(), Some(HEAVEN));
    tui.run(&format!("/move \"Gabriel\" \"{HOME}\""));
    assert_eq!(living(&tui, "Gabriel").as_deref(), Some(HOME));
}

#[test]
fn a_fish_the_devil_marked_is_never_let_into_heaven_even_if_it_is_holy() {
    let mut tui = home_with(&["Ann"]);
    tui.run("/kill \"Ann\"");
    tui.run("/spawn holyfish \"Gabriel\"");
    let gabriel = tui.app.tanks[0]
        .fish
        .iter_mut()
        .find(|f| f.name == "Gabriel")
        .unwrap();
    gabriel.devil_marked = true;
    tui.run(&format!("/move \"Gabriel\" \"{HEAVEN}\""));
    assert_eq!(living(&tui, "Gabriel").as_deref(), Some(HOME));
}

#[test]
fn whatever_arrives_in_heaven_that_is_not_holy_lands_in_the_next_tank() {
    let mut tui = home_with(&["Ann"]);
    tui.run("/kill \"Ann\"");
    tui.run(&format!("/switch \"{HEAVEN}\""));
    let before = tui.app.tanks[0].fish.len();

    tui.run("/give salmon");
    tui.run("/give cow");

    let heaven = heaven(&tui).unwrap();
    assert!(heaven.fish.is_empty() && heaven.cows.is_empty());
    assert_eq!(tui.app.tanks[0].fish.len(), before + 1);
    assert_eq!(tui.app.tanks[0].cows.len(), 1);
}

#[test]
fn heaven_is_never_bought_and_one_pearl_is_all_a_player_can_hold() {
    let mut tui = home_with(&["Ann"]);
    tui.run("/kill \"Ann\"");
    let tanks = tui.app.tanks.len();
    let cash = tui.app.purse.balance();

    tui.run("/give heaventank");
    tui.run("/buy heaventank");
    tui.run("/give golden pearl");

    assert_eq!(tui.app.tanks.len(), tanks);
    assert_eq!(heavens(&tui), 1);
    assert_eq!(tui.app.purse.balance(), cash);
    assert_eq!(pearls(&tui), 0, "no pearl beside a Heaventank");
}

#[test]
fn an_empty_heaven_sells_like_a_found_legendary_tank_and_frees_the_pearl() {
    let mut tui = home_with(&["Ann"]);
    tui.run("/kill \"Ann\"");
    let cash = tui.app.purse.balance();

    tui.run(&format!("/sell tank \"{HEAVEN}\""));

    assert_eq!(heavens(&tui), 0);
    assert_eq!(
        tui.app.purse.balance() - cash,
        fishtank::economy::Money::from(Rarity::Legendary.catch_worth())
    );
    assert!(
        tui.app.graveyard.iter().any(|f| f.name == "Ann"),
        "the dead stay dead"
    );
    tui.run("/give golden pearl");
    assert_eq!(pearls(&tui), 1);
    open_heaven(&mut tui);
    assert_eq!(soul_names(&tui), ["Ann"], "a new heaven hangs the old dead");
}

#[test]
fn the_last_tank_a_mortal_fish_can_live_in_is_never_sold_to_heaven() {
    let mut tui = home_with(&["Ann"]);
    tui.run("/kill \"Ann\"");
    tui.run(&format!("/sell tank \"{HOME}\""));
    assert!(tui.app.tanks.iter().any(|t| t.name == HOME));
}

#[test]
fn the_dead_cannot_be_moved_mutated_sold_or_killed_again() {
    let mut tui = home_with(&["Ann"]);
    tui.run("/kill \"Ann\"");
    let cash = tui.app.purse.balance();
    tui.run(&format!("/move \"Ann\" \"{HOME}\""));
    tui.run("/mutate \"Ann\" eyeincrease");
    tui.run("/sell fish \"Ann\"");
    tui.run("/kill \"Ann\"");
    assert_eq!(soul_names(&tui), ["Ann"]);
    assert_eq!(tui.app.graveyard.len(), 1);
    assert_eq!(tui.app.purse.balance(), cash);
    let soul = heaven(&tui).unwrap().souls()[0].clone();
    assert_eq!(soul.mutation_count(), 0);
}

#[test]
fn the_heaventank_index_lists_the_dead_and_every_other_index_stays_among_the_living() {
    let mut tui = Tui::with_size(100, 30);
    tui.clear_tank();
    open_heaven(&mut tui);
    tui.run("/spawn salmon \"Ann\"");
    tui.run("/spawn salmon \"Bob\"");
    tui.run("/kill \"Ann\"");

    tui.run(&format!("/index \"{HEAVEN}\""));
    let screen = tui.screen();
    screen.expect_find("Status");
    screen.expect_find("Ann");
    screen.expect_find(DEAD);
    screen.expect_absent("Bob");
    tui.key(crossterm::event::KeyCode::Esc);

    tui.run("/index");
    let screen = tui.screen();
    screen.expect_find("Bob");
    screen.expect_absent("Ann");
    screen.expect_absent(DEAD);
    screen.expect_absent(ALIVE);
}

#[test]
fn a_holy_fish_that_lives_in_heaven_shows_as_alive_beside_the_dead() {
    let mut tui = Tui::with_size(100, 30);
    tui.clear_tank();
    open_heaven(&mut tui);
    tui.run("/spawn salmon \"Ann\"");
    tui.run("/kill \"Ann\"");
    tui.run(&format!("/switch \"{HEAVEN}\""));
    tui.run("/spawn holyfish \"Gabriel\"");
    tui.run(&format!("/index \"{HEAVEN}\""));
    let screen = tui.screen();
    screen.expect_find(ALIVE);
    screen.expect_find(DEAD);
}

#[test]
fn a_dead_row_in_the_index_opens_nothing() {
    let mut tui = Tui::with_size(100, 30);
    tui.clear_tank();
    open_heaven(&mut tui);
    tui.run("/spawn salmon \"Ann\"");
    tui.run("/kill \"Ann\"");
    tui.run(&format!("/index \"{HEAVEN}\""));
    tui.key(crossterm::event::KeyCode::Enter);
    tui.screen().expect_find("FishResource#index");
}

#[test]
fn heaven_draws_at_every_size_without_breaking_a_border() {
    let mut tui = home_with(&["Ann"]);
    tui.run("/kill \"Ann\"");
    tui.run(&format!("/switch \"{HEAVEN}\""));
    for (cols, rows) in [(100, 30), (60, 18), (40, 14), (28, 10)] {
        tui.resize(cols, rows);
        tui.tick_n(30);
        tui.run("/names");
        tui.screen();
        tui.run("/names");
    }
}
