use crossterm::event::KeyCode;
use rand::{SeedableRng, rngs::SmallRng};

use fishtank::casino::blackjack::{Blackjack, Phase};
use fishtank::casino::bubble::Risk;
use fishtank::casino::derby::{CHANCES_PER_MILLE, PAYS_BACK_PER_MILLE, odds};
use fishtank::casino::flip::{FLIGHT_SECS, FlipPhase, LOSS_SHOWN_SECS, Landing, SWIM_SECS, Side};
use fishtank::casino::net::Net;
use fishtank::casino::pufferfish::{PuffPhase, reach_chance};
use fishtank::casino::seat::Verdict;
use fishtank::casino::spins;
use fishtank::casino::state::{CasinoState, Game, Play, Popup, REVEAL_SECS, View};
use fishtank::casino::{COMP_EVERY, FISH_PREMIUM, Multiple, POT_SHARE_PER_CENT, premium};
use fishtank::economy::Money;
use fishtank::ledger::{Direction, Flow};
use fishtank::loot::{ConsumableKind, StockItem};
use fishtank::testing::Tui;

const POT: f64 = POT_SHARE_PER_CENT as f64 / 100.0;
const HANDS: usize = 200_000;

#[test]
fn every_table_pays_back_less_than_it_takes_in_cash() {
    let spins = spins::pays_back() + POT;
    assert!(spins < 0.93, "spins {spins}");
    for risk in Risk::ALL {
        assert!(risk.pays_back() < 0.99, "{risk:?}");
    }
    for target in [1.01, 1.5, 2.0, 10.0, 1000.0, 1e9] {
        assert!(
            reach_chance(target) * target < 1.0,
            "pufferfish at {target}"
        );
    }
    for net in Net::ALL {
        assert!(net.pays_back() < 0.92, "{net:?}");
    }
    assert!(Landing::pays_back() < 1.0);
    for chance in CHANCES_PER_MILLE {
        let rtp = odds(chance, PAYS_BACK_PER_MILLE).as_f64() * f64::from(chance) / 1000.0;
        assert!(rtp < 0.93, "derby at {chance}: {rtp}");
    }
}

#[test]
fn a_fish_plays_for_a_quarter_more_than_its_worth_at_every_table_that_takes_one() {
    let (num, den) = FISH_PREMIUM;
    let worth: Money = 10_000;
    assert_eq!(premium(worth), worth * num / den);
    let tables: Vec<Game> = Game::ALL.into_iter().filter(|g| g.takes_fish()).collect();
    assert_eq!(
        tables,
        [
            Game::Blackjack,
            Game::Spins,
            Game::Pufferfish,
            Game::BubbleUp,
            Game::Derby
        ],
        "a net is bought, and bait is a sale"
    );
}

#[test]
fn a_fish_hand_at_blackjack_doubles_and_splits_like_any_other() {
    let mut rng = SmallRng::seed_from_u64(11);
    let value = premium(400);
    let doubled = (0..HANDS / 100).any(|_| {
        let mut game = Blackjack::deal(value);
        game.finish_now(&mut rng);
        while game.phase == Phase::Player {
            let play = game.basic_move();
            game.play(play, &mut rng);
            game.finish_now(&mut rng);
        }
        game.total_bet() > value
    });
    assert!(doubled);
}

fn ticks(tui: &Tui, secs: f32) -> usize {
    (secs * tui.app.settings.fps).ceil() as usize
}

fn open(tui: &mut Tui, line: &str) {
    tui.stake();
    tui.run(line);
}

fn casino(tui: &mut Tui) -> &mut CasinoState {
    tui.app.casino_state_mut().expect("the casino is open")
}

fn graves(tui: &Tui) -> Vec<String> {
    tui.app.graveyard.iter().map(|f| f.name.clone()).collect()
}

fn living(tui: &Tui, name: &str) -> usize {
    tui.app
        .tanks
        .iter()
        .flat_map(|t| t.fish.iter())
        .filter(|f| f.name == name)
        .count()
}

fn stake_adam(tui: &mut Tui) {
    tui.key(KeyCode::Tab);
    tui.select("Adam (Merluza)");
    tui.key(KeyCode::Enter);
}

#[test]
fn the_lobby_lists_every_table_under_tollomind() {
    let mut tui = Tui::new();
    open(&mut tui, "/casino");
    let screen = tui.screen();
    screen.expect_find("Casino");
    for game in Game::ALL {
        screen.expect_find(game.name());
    }
    screen.expect_find("▸◂");
    screen.expect_find("The Pot");
}

#[test]
fn a_table_can_be_named_on_the_command_line() {
    let mut tui = Tui::new();
    open(&mut tui, "/casino bubbleup");
    tui.screen().expect_find("Bubble Up");
    assert!(matches!(casino(&mut tui).view, View::Table(_)));
}

#[test]
fn a_fish_that_loses_is_eaten_and_rests_in_the_graveyard() {
    let mut tui = Tui::new();
    open(&mut tui, "/casino pufferfish");
    stake_adam(&mut tui);
    tui.screen().expect_find("Adam plays for");
    tui.key(KeyCode::Enter);
    if let View::Table(table) = &mut casino(&mut tui).view
        && let Play::Pufferfish(puffer) = &mut table.play
    {
        puffer.phase = PuffPhase::Puffing {
            clock: 0.0,
            pops_at: Multiple::ONE,
        };
    }
    tui.tick_n(3);
    assert!(graves(&tui).contains(&"Adam".to_string()));
    assert_eq!(living(&tui, "Adam"), 0);
    tui.screen().expect_find("Tollomind ate Adam");
}

#[test]
fn a_fish_that_wins_swims_home_with_its_winnings() {
    let mut tui = Tui::new();
    open(&mut tui, "/casino pufferfish");
    stake_adam(&mut tui);
    let purse = tui.app.purse.balance();
    tui.key(KeyCode::Enter);
    if let View::Table(table) = &mut casino(&mut tui).view
        && let Play::Pufferfish(puffer) = &mut table.play
    {
        puffer.phase = PuffPhase::Puffing {
            clock: 14.0,
            pops_at: Multiple::whole(50),
        };
    }
    tui.key(KeyCode::Enter);
    assert_eq!(living(&tui, "Adam"), 1);
    assert!(tui.app.purse.balance() > purse, "the winnings are paid");
    tui.screen().expect_find("Adam swims home");
}

#[test]
fn a_tie_brings_a_fish_home() {
    let mut tui = Tui::new();
    open(&mut tui, "/casino pufferfish");
    stake_adam(&mut tui);
    tui.key(KeyCode::Enter);
    if let View::Table(table) = &mut casino(&mut tui).view
        && let Play::Pufferfish(puffer) = &mut table.play
    {
        puffer.phase = PuffPhase::Puffing {
            clock: 0.0,
            pops_at: Multiple::whole(50),
        };
    }
    tui.key(KeyCode::Enter);
    assert_eq!(living(&tui, "Adam"), 1);
    assert!(!graves(&tui).contains(&"Adam".to_string()));
    tui.screen().expect_find("Adam swims home");
}

#[test]
fn walking_away_from_a_fish_on_the_table_buries_it_when_the_game_comes_back() {
    let mut tui = Tui::new();
    open(&mut tui, "/casino blackjack");
    stake_adam(&mut tui);
    tui.key(KeyCode::Enter);
    tui.tick_n(5);
    let save = tui.app.snapshot();
    let back = Tui::resumed(save, 100, 30);
    assert!(graves(&back).contains(&"Adam".to_string()));
    assert_eq!(living(&back, "Adam"), 0);
}

#[test]
fn quitting_on_a_win_card_keeps_the_fish_that_won_it() {
    let mut tui = Tui::new();
    open(&mut tui, "/casino pufferfish");
    stake_adam(&mut tui);
    tui.key(KeyCode::Enter);
    if let View::Table(table) = &mut casino(&mut tui).view
        && let Play::Pufferfish(puffer) = &mut table.play
    {
        puffer.phase = PuffPhase::Puffing {
            clock: 28.0,
            pops_at: Multiple::whole(50),
        };
    }
    tui.key(KeyCode::Enter);
    tui.tick_n(ticks(&tui, REVEAL_SECS) + 1);
    assert!(matches!(casino(&mut tui).popup, Some(Popup::Banner(_))));
    let back = Tui::resumed(tui.app.snapshot(), 100, 30);
    assert_eq!(living(&back, "Adam"), 1);
    assert!(!graves(&back).contains(&"Adam".to_string()));
}

#[test]
fn the_pot_and_the_best_win_are_saved() {
    let mut tui = Tui::new();
    tui.app.casino.pot = 12_345;
    tui.app.casino.best_win = 678;
    let back = Tui::resumed(tui.app.snapshot(), 100, 30);
    assert_eq!(back.app.casino.pot, 12_345);
    assert_eq!(back.app.casino.best_win, 678);
}

#[test]
fn stakes_and_winnings_have_their_own_ledger_lines() {
    let mut tui = Tui::new();
    open(&mut tui, "/casino spins");
    tui.key(KeyCode::Enter);
    tui.tick_n(200);
    let ledger = tui.app.ledger.since_launch();
    assert!(ledger.line(Direction::Out, Flow::CasinoStakes) > 0);
    tui.key(KeyCode::Esc);
    tui.key(KeyCode::Esc);
    tui.run("/ledger");
    tui.screen().expect_find("Casino stakes");
}

#[test]
fn tollomind_pours_a_coffee_for_every_five_thousand_wagered() {
    let mut tui = Tui::new();
    open(&mut tui, "/casino pufferfish");
    for _ in 0..3 {
        tui.key(KeyCode::Right);
    }
    let coffee = StockItem::Consumable(ConsumableKind::Coffee);
    let before = tui.app.stock_of(coffee);
    tui.key(KeyCode::Char('a'));
    tui.key(KeyCode::Enter);
    let wagered = tui
        .app
        .ledger
        .since_launch()
        .line(Direction::Out, Flow::CasinoStakes);
    let poured = tui.app.stock_of(coffee) - before;
    assert_eq!(Money::from(poured), wagered / COMP_EVERY);
}

#[test]
fn double_or_nothing_copies_every_fish_on_the_line() {
    let mut tui = Tui::new();
    open(&mut tui, "/casino pufferfish");
    stake_adam(&mut tui);
    tui.key(KeyCode::Enter);
    if let View::Table(table) = &mut casino(&mut tui).view
        && let Play::Pufferfish(puffer) = &mut table.play
    {
        puffer.phase = PuffPhase::Puffing {
            clock: 14.0,
            pops_at: Multiple::whole(50),
        };
    }
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Char('d'));
    let state = casino(&mut tui);
    let Some(Popup::Flip(flip)) = &mut state.popup else {
        panic!("D opens the flip");
    };
    flip.phase = FlipPhase::Flying {
        call: Side::Left,
        landing: Landing::Facing(Side::Left),
        t: 0.0,
    };
    tui.tick_n(60);
    assert_eq!(living(&tui, "Adam") + living(&tui, "Adam II"), 2);
    tui.screen().expect_find("2 fish");
}

#[test]
fn a_lost_double_or_nothing_closes_by_itself_and_takes_the_fish() {
    let mut tui = Tui::new();
    open(&mut tui, "/casino pufferfish");
    stake_adam(&mut tui);
    tui.key(KeyCode::Enter);
    if let View::Table(table) = &mut casino(&mut tui).view
        && let Play::Pufferfish(puffer) = &mut table.play
    {
        puffer.phase = PuffPhase::Puffing {
            clock: 14.0,
            pops_at: Multiple::whole(50),
        };
    }
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Char('d'));
    if let Some(Popup::Flip(flip)) = &mut casino(&mut tui).popup {
        flip.phase = FlipPhase::Flying {
            call: Side::Left,
            landing: Landing::BellyUp,
            t: 0.0,
        };
    }
    let swimming = ticks(&tui, FLIGHT_SECS + LOSS_SHOWN_SECS + SWIM_SECS / 2.0);
    tui.tick_n(swimming);
    assert!(
        matches!(casino(&mut tui).popup, Some(Popup::Flip(_))),
        "Tollomind is still crossing the stage"
    );
    let Some(Popup::Flip(flip)) = &casino(&mut tui).popup else {
        unreachable!();
    };
    let swim = flip.tollomind().expect("Tollomind swims in");
    assert_eq!(
        swim.from,
        Side::Right,
        "he comes from the side nobody called"
    );
    tui.tick_n(ticks(&tui, SWIM_SECS / 2.0) + 1);
    assert!(
        casino(&mut tui).popup.is_none(),
        "the flip closes once Tollomind has left the stage"
    );
    assert!(graves(&tui).contains(&"Adam".to_string()));
    tui.screen().expect_absent("keeps it");
}

#[test]
fn a_big_win_card_waits_for_the_table_to_show_what_won() {
    let mut tui = Tui::new();
    open(&mut tui, "/casino pufferfish");
    tui.key(KeyCode::Enter);
    if let View::Table(table) = &mut casino(&mut tui).view
        && let Play::Pufferfish(puffer) = &mut table.play
    {
        puffer.phase = PuffPhase::Puffing {
            clock: 28.0,
            pops_at: Multiple::whole(50),
        };
    }
    tui.key(KeyCode::Enter);
    assert!(
        casino(&mut tui).popup.is_none(),
        "the card is still face down"
    );
    tui.key(KeyCode::Enter);
    tui.tick_n(ticks(&tui, REVEAL_SECS / 2.0));
    assert!(casino(&mut tui).popup.is_none(), "keys wait for the card");
    tui.tick_n(ticks(&tui, REVEAL_SECS / 2.0) + 1);
    assert!(matches!(casino(&mut tui).popup, Some(Popup::Banner(_))));
}

#[test]
fn an_ordinary_bubble_never_offers_double_or_nothing() {
    let mut tui = Tui::new();
    open(&mut tui, "/casino bubbleup");
    for _ in 0..30 {
        tui.key(KeyCode::Enter);
        tui.tick_n(10);
    }
    tui.tick_n(60);
    let state = casino(&mut tui);
    if let Some(Popup::Banner(_)) = state.popup {
        return;
    }
    if let View::Table(table) = &state.view {
        assert!(table.double.is_none());
    }
    tui.screen().expect_absent("D double or nothing");
}

#[test]
fn a_big_bubble_win_keeps_its_double_or_nothing_while_other_bubbles_land() {
    let mut tui = Tui::new();
    open(&mut tui, "/casino bubbleup");
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Enter);
    if let View::Table(table) = &mut casino(&mut tui).view
        && let Play::BubbleUp { bubbles, .. } = &mut table.play
    {
        bubbles[0].path = vec![false; bubbles[0].path.len()];
    }
    tui.tick_n(90);
    let state = casino(&mut tui);
    assert!(
        matches!(state.popup, Some(Popup::Banner(_))),
        "the edge shell opens a win card"
    );
    tui.screen().expect_find("D double or nothing");
}

#[test]
fn a_net_lands_its_fish_in_a_tank_or_food_in_the_bag() {
    let mut tui = Tui::new();
    open(&mut tui, "/casino mysterynet");
    let fish_before: usize = tui.app.tanks.iter().map(|t| t.fish.len()).sum();
    let food_before = tui.app.food_supply;
    tui.key(KeyCode::Enter);
    tui.tick_n(200);
    let prize = matches!(casino(&mut tui).popup, Some(Popup::Prize(_)));
    if prize {
        tui.type_text("Nemo");
        tui.key(KeyCode::Enter);
        let fish_after: usize = tui.app.tanks.iter().map(|t| t.fish.len()).sum();
        assert_eq!(fish_after, fish_before + 1);
        assert_eq!(living(&tui, "Nemo"), 1);
    } else {
        assert!(tui.app.food_supply > food_before);
    }
    if let View::Table(table) = &casino(&mut tui).view {
        assert!(matches!(
            table.result.as_ref().map(|r| &r.verdict),
            Some(Verdict::Netted { .. })
        ));
    }
}

fn leave(tui: &mut Tui) {
    for _ in 0..8 {
        let Some(state) = tui.app.casino_state_mut() else {
            return;
        };
        if matches!(state.popup, Some(Popup::Confirm)) {
            tui.key(KeyCode::Enter);
            continue;
        }
        state.popup = None;
        tui.key(KeyCode::Esc);
    }
}

fn sit(tui: &mut Tui, line: &str) {
    leave(tui);
    tui.run(line);
}

fn with_popup(tui: &mut Tui, popup: Popup, label: &str) {
    casino(tui).popup = Some(popup);
    tui.tick_n(40);
    tui.snap(label);
    casino(tui).popup = None;
}

#[test]
fn every_table_and_popup_is_filmed_whole_at_every_size() {
    use fishtank::casino::flip::Flip;
    use fishtank::casino::seat::OnTheLine;
    use fishtank::casino::spins::{Dive, REELS, SpinPhase, Symbol};
    use fishtank::casino::state::{Banner, PrizeCard};
    use fishtank::fishes::fish::Fish;
    use fishtank::fishes::species::FishSpecies;
    use fishtank::ui::text_input::TextInput;
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"));
    for (cols, rows) in [(100, 30), (60, 18), (40, 14), (28, 10)] {
        let mut tui = Tui::with_size(cols, rows);
        tui.film(dir, &format!("casino-{cols}x{rows}"));
        tui.run("/spawn seahorsefish \"Pegaso\"");
        open(&mut tui, "/casino");
        tui.snap("lobby");
        for game in Game::ALL {
            sit(&mut tui, &format!("/casino {}", game.token()));
            tui.snap(game.name());
            tui.key(KeyCode::Enter);
            tui.tick_n(50);
            tui.snap(&format!("{} in play", game.name()));
            tui.tick_n(400);
            if casino(&mut tui).popup.is_some() {
                tui.snap(&format!("{} popup", game.name()));
                casino(&mut tui).popup = None;
            }
            tui.snap(&format!("{} settled", game.name()));
        }
        sit(&mut tui, "/casino blackjack");
        tui.key(KeyCode::Tab);
        tui.snap("picker");
        if tui.try_select("Pegaso").is_err() {
            panic!(
                "{cols}x{rows}
{}",
                tui.screen().text()
            );
        }
        tui.key(KeyCode::Enter);
        tui.key(KeyCode::Enter);
        tui.tick_n(60);
        tui.snap("a seahorse at the table");
        sit(&mut tui, "/casino spins");
        if let View::Table(table) = &mut casino(&mut tui).view
            && let Play::Spins(spins) = &mut table.play
        {
            let window = [[Symbol::Pearl, Symbol::Coffee, Symbol::Bubbles]; REELS];
            let dive = Dive::start(&window, &mut rand::rng());
            spins.phase = SpinPhase::Diving(Box::new(dive));
        }
        tui.tick_n(5);
        tui.snap("three pearls surface");
        tui.tick_n(55);
        tui.snap("the pearls open");
        tui.tick_n(100);
        tui.snap("pearl dive");
        let line = OnTheLine {
            cash: 4_000,
            fish: Vec::new(),
        };
        with_popup(
            &mut tui,
            Popup::Flip(Flip {
                line: line.clone(),
                base: 500,
                base_fish: 0,
                rung: 3,
                phase: FlipPhase::Won {
                    landing: Landing::Facing(Side::Left),
                },
            }),
            "double or nothing",
        );
        let half_way = LOSS_SHOWN_SECS + SWIM_SECS / 2.0 - 40.0 / tui.app.settings.fps;
        for (swum, label) in [
            (-0.3, "tollomind comes for the goldfish"),
            (0.0, "the goldfish is eaten"),
        ] {
            with_popup(
                &mut tui,
                Popup::Flip(Flip {
                    line: line.clone(),
                    base: 500,
                    base_fish: 0,
                    rung: 0,
                    phase: FlipPhase::Lost {
                        landing: Landing::Facing(Side::Right),
                        call: Side::Left,
                        t: half_way + swum * SWIM_SECS,
                    },
                }),
                label,
            );
        }
        with_popup(
            &mut tui,
            Popup::Banner(Banner {
                line: Some(line),
                amount: 125_000,
                multiple: Multiple::whole(250),
                label: "Pearl Dive ×250".to_string(),
                jackpot: false,
                t: 0.0,
            }),
            "stupid win",
        );
        with_popup(&mut tui, Popup::Paytable(0), "paytable");
        let fish = Fish::new(
            FishSpecies::Salmon,
            String::new(),
            0.0,
            0.0,
            &mut rand::rng(),
        );
        with_popup(
            &mut tui,
            Popup::Prize(PrizeCard {
                fish,
                name: TextInput::new(),
            }),
            "a netted fish",
        );
        tui.write_reel();
        let flaws = tui.reel().flaw_report();
        assert!(flaws.is_empty(), "{cols}x{rows}: {flaws}");
    }
}
