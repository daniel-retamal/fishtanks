use std::path::Path;

use crossterm::event::{Event, KeyCode};
use fishtank::testing::Tui;
use fishtank::ui::fishing_overlay::Temper;

const CAST_TICKS: usize = 30 * 90;
const CASTS: usize = 6;
const LOOKAHEAD_STEPS: f32 = 6.0;
const DEADBAND: f32 = 0.04;
const REEL_ZONE: f32 = 0.3;
const CENTRE: f32 = 0.5;
const CATCH_CARD_HINTS: [&str; 2] = ["ENTER capture", "ESC/q close"];
const REEL_HINT: &str = "↓ reel";
const TWO_SECONDS: usize = 60;
const WHOLE_HINT_COLS: u16 = 80;
const FILM_SIZES: [(u16, u16); 5] = [(100, 30), (80, 24), (60, 18), (40, 14), (28, 10)];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Emulator {
    Windows,
    Kitty,
    AppleTerminal,
    ConPty,
}

impl Emulator {
    const ALL: [Emulator; 4] = [
        Emulator::Windows,
        Emulator::Kitty,
        Emulator::AppleTerminal,
        Emulator::ConPty,
    ];

    fn releases(self, code: KeyCode) -> bool {
        match self {
            Emulator::Windows => true,
            Emulator::Kitty => !matches!(code, KeyCode::Char(_) | KeyCode::Enter),
            Emulator::AppleTerminal => false,
            Emulator::ConPty => true,
        }
    }

    fn pairs_every_press_with_a_release(self) -> bool {
        self == Emulator::ConPty
    }

    fn repeats_every_held_key(self) -> bool {
        self == Emulator::Windows
    }
}

#[derive(Clone, Copy, Debug)]
struct Cadence {
    first_repeat: u32,
    repeat: u32,
}

const WINDOWS_CADENCE: Cadence = Cadence {
    first_repeat: 15,
    repeat: 1,
};

const MAC_CADENCES: [Cadence; 4] = [
    Cadence {
        first_repeat: 7,
        repeat: 1,
    },
    Cadence {
        first_repeat: 11,
        repeat: 3,
    },
    Cadence {
        first_repeat: 20,
        repeat: 1,
    },
    Cadence {
        first_repeat: 31,
        repeat: 5,
    },
];

struct Hand {
    terminal: Emulator,
    cadence: Cadence,
    held: Vec<(KeyCode, u32)>,
}

impl Hand {
    fn new(terminal: Emulator, cadence: Cadence) -> Self {
        Self {
            terminal,
            cadence,
            held: Vec::new(),
        }
    }

    fn run(&mut self, tui: &mut Tui, line: &str) {
        tui.run(line);
        self.lift(tui, KeyCode::Enter);
    }

    fn lift(&mut self, tui: &mut Tui, code: KeyCode) {
        if self.terminal.releases(code) {
            tui.release(code);
        }
    }

    fn hold(&mut self, tui: &mut Tui, code: KeyCode, down: bool) {
        let at = self.held.iter().position(|(held, _)| *held == code);
        match (at, down) {
            (None, true) => {
                self.strike(tui, code);
                self.held.push((code, 0));
            }
            (Some(i), false) => {
                self.held.remove(i);
                if !self.terminal.pairs_every_press_with_a_release() {
                    self.lift(tui, code);
                }
            }
            _ => {}
        }
    }

    fn strike(&mut self, tui: &mut Tui, code: KeyCode) {
        tui.key(code);
        if self.terminal.pairs_every_press_with_a_release() {
            tui.release(code);
        }
    }

    fn let_go(&mut self, tui: &mut Tui) {
        for (code, _) in std::mem::take(&mut self.held) {
            if !self.terminal.pairs_every_press_with_a_release() {
                self.lift(tui, code);
            }
        }
    }

    fn repeats_on(&self, ticks: u32) -> bool {
        ticks >= self.cadence.first_repeat
            && (ticks - self.cadence.first_repeat).is_multiple_of(self.cadence.repeat)
    }

    fn repeating(&self) -> &[(KeyCode, u32)] {
        if self.terminal.repeats_every_held_key() {
            return &self.held;
        }
        let last = self.held.len().saturating_sub(1);
        &self.held[last..]
    }

    fn tick(&mut self, tui: &mut Tui) {
        for (_, ticks) in &mut self.held {
            *ticks += 1;
        }
        let due: Vec<KeyCode> = self
            .repeating()
            .iter()
            .filter(|(_, ticks)| self.repeats_on(*ticks))
            .map(|(code, _)| *code)
            .collect();
        for code in due {
            self.strike(tui, code);
        }
        tui.tick_n(1);
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Style {
    ReelWhileSteering,
    OneKeyAtATime,
}

fn card_shown(tui: &mut Tui) -> bool {
    let screen = tui.screen();
    CATCH_CARD_HINTS.iter().any(|hint| screen.contains(hint))
}

fn reel_like_a_player(tui: &mut Tui, hand: &mut Hand, style: Style) -> bool {
    hand.run(tui, "/fish --legendary");
    for _ in 0..CAST_TICKS {
        let Some(state) = tui.app.fishing_state() else {
            hand.let_go(tui);
            return card_shown(tui);
        };
        if state.is_catching() {
            let biting = state.is_biting();
            hand.hold(tui, KeyCode::Down, biting);
            hand.tick(tui);
            continue;
        }
        let ahead = state.fish_pos + state.fish_velocity * LOOKAHEAD_STEPS;
        let left = ahead > CENTRE + DEADBAND;
        let right = ahead < CENTRE - DEADBAND;
        let mut reel = (ahead - CENTRE).abs() * 2.0 < REEL_ZONE;
        if style == Style::OneKeyAtATime {
            reel &= !left && !right;
            hand.hold(tui, KeyCode::Left, left);
            hand.hold(tui, KeyCode::Right, right);
            hand.hold(tui, KeyCode::Down, reel);
        } else {
            hand.hold(tui, KeyCode::Down, reel);
            hand.hold(tui, KeyCode::Left, left);
            hand.hold(tui, KeyCode::Right, right);
        }
        hand.tick(tui);
    }
    panic!("a cast outlasted {CAST_TICKS} ticks");
}

fn landed_casts(terminal: Emulator, cadence: Cadence, style: Style) -> usize {
    let mut tui = Tui::new();
    tui.clear_tank();
    let mut hand = Hand::new(terminal, cadence);
    let mut landed = 0;
    for cast in 0..CASTS {
        if !reel_like_a_player(&mut tui, &mut hand, style) {
            continue;
        }
        landed += 1;
        tui.type_text(&format!("Kept {cast}"));
        tui.key(KeyCode::Enter);
        assert!(!card_shown(&mut tui), "the card closed");
    }
    landed
}

fn hook_a_fish(tui: &mut Tui) {
    tui.run("/fish --no-escape");
    for _ in 0..CAST_TICKS {
        if tui.app.fishing_state().is_some_and(|s| s.is_biting()) {
            break;
        }
        tui.tick_n(1);
    }
    tui.key(KeyCode::Down);
    let state = tui.app.fishing_state().expect("the fish is hooked");
    assert!(!state.is_catching(), "↓ on a bite hooks the fish");
    assert!(state.is_reeling, "and reels while ↓ is down");
}

fn reeling(tui: &Tui) -> bool {
    tui.app.fishing_state().expect("no escape").is_reeling
}

#[test]
fn a_player_reeling_while_steering_lands_every_cast_on_windows() {
    assert_eq!(
        landed_casts(Emulator::Windows, WINDOWS_CADENCE, Style::ReelWhileSteering),
        CASTS
    );
}

#[test]
fn a_mac_terminal_that_reports_arrow_releases_plays_exactly_like_windows() {
    for cadence in MAC_CADENCES {
        assert_eq!(
            landed_casts(Emulator::Kitty, cadence, Style::ReelWhileSteering),
            CASTS,
            "{cadence:?}"
        );
    }
}

#[test]
fn a_player_on_terminal_app_lands_every_cast_one_key_at_a_time() {
    for cadence in MAC_CADENCES {
        assert_eq!(
            landed_casts(Emulator::AppleTerminal, cadence, Style::OneKeyAtATime),
            CASTS,
            "{cadence:?}"
        );
    }
}

#[test]
fn a_player_in_a_multiplexer_whose_every_key_up_comes_with_its_key_down_lands_every_cast() {
    for cadence in [WINDOWS_CADENCE].into_iter().chain(MAC_CADENCES) {
        assert_eq!(
            landed_casts(Emulator::ConPty, cadence, Style::OneKeyAtATime),
            CASTS,
            "{cadence:?}"
        );
    }
}

#[test]
fn the_down_that_hooks_the_fish_on_terminal_app_stops_reeling_like_a_tap() {
    let mut tui = Tui::new();
    tui.clear_tank();
    hook_a_fish(&mut tui);
    tui.tick_n(TWO_SECONDS / 4);
    assert!(
        !reeling(&tui),
        "half a second after a tap with no repeat, the reel has stopped"
    );
}

#[test]
fn holding_down_on_terminal_app_reels_through_its_repeats() {
    let mut tui = Tui::new();
    tui.clear_tank();
    hook_a_fish(&mut tui);
    for _ in 0..TWO_SECONDS {
        tui.key(KeyCode::Down);
        tui.tick_n(1);
        assert!(reeling(&tui), "a repeating ↓ is a held ↓");
    }
}

#[test]
fn a_steering_key_with_no_repeat_lets_go_on_its_own() {
    let mut tui = Tui::new();
    tui.clear_tank();
    hook_a_fish(&mut tui);
    tui.key(KeyCode::Right);
    assert!(tui.app.fishing_state().expect("hooked").is_pushing_right);
    tui.tick_n(TWO_SECONDS);
    let state = tui.app.fishing_state().expect("no escape");
    assert!(!state.is_pushing_right, "a key that stops repeating is up");
}

#[test]
fn a_terminal_that_reports_releases_reels_while_down_is_held_and_steered_with() {
    for terminal in [Emulator::Windows, Emulator::Kitty] {
        let mut tui = Tui::new();
        tui.clear_tank();
        if terminal == Emulator::Windows {
            tui.release(KeyCode::Enter);
        } else {
            tui.key(KeyCode::Up).tick_n(1).release(KeyCode::Up);
        }
        hook_a_fish(&mut tui);
        tui.key(KeyCode::Right);
        tui.tick_n(TWO_SECONDS);
        let state = tui.app.fishing_state().expect("no escape");
        assert!(state.is_reeling, "{terminal:?}: held with no repeat");
        assert!(state.is_pushing_right, "{terminal:?}: while steering");
        tui.release(KeyCode::Down);
        assert!(!reeling(&tui), "{terminal:?}: let go at once");
    }
}

#[test]
fn the_reel_says_how_to_reel_at_every_size() {
    for terminal in Emulator::ALL {
        for (cols, rows) in FILM_SIZES {
            let mut tui = Tui::with_size(cols, rows);
            tui.film(
                Path::new(env!("CARGO_TARGET_TMPDIR")),
                &format!("fishing-{terminal:?}-{cols}x{rows}"),
            );
            tui.clear_tank();
            hook_a_fish(&mut tui);
            tui.tick_n(1);
            tui.snap(&format!("{terminal:?}: the reel and its hints"));
            if cols >= WHOLE_HINT_COLS {
                tui.screen().expect_find(REEL_HINT);
            }
        }
    }
}

#[test]
fn a_window_that_loses_focus_lets_go_of_every_key() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.release(KeyCode::Enter);
    hook_a_fish(&mut tui);
    tui.key(KeyCode::Right);
    tui.tick_n(TWO_SECONDS);
    assert!(
        reeling(&tui),
        "held with no repeat on a terminal that sends key-ups"
    );
    tui.app.handle_input(Event::FocusLost);
    tui.tick_n(1);
    let state = tui.app.fishing_state().expect("no escape");
    assert!(!state.is_reeling, "the key-up went to another window");
    assert!(!state.is_pushing_right);
}

#[test]
fn the_reel_shows_where_it_is_safe_to_reel_behind_the_control() {
    for temper in ["--normal", "--legendary"] {
        for (cols, rows) in FILM_SIZES {
            let mut tui = Tui::with_size(cols, rows);
            tui.film(
                Path::new(env!("CARGO_TARGET_TMPDIR")),
                &format!("fishing-zones{temper}-{cols}x{rows}"),
            );
            tui.clear_tank();
            tui.run(&format!("/fish --no-escape {temper}"));
            for _ in 0..CAST_TICKS {
                if tui.app.fishing_state().is_some_and(|s| s.is_biting()) {
                    break;
                }
                tui.tick_n(1);
            }
            tui.key(KeyCode::Down);
            tui.tick_n(TWO_SECONDS);
            tui.snap(&format!("{temper}: the water behind the control"));
        }
    }
}

#[test]
fn the_debug_flags_choose_how_the_hooked_fish_fights() {
    for (flag, temper) in [
        ("--normal", Temper::Normal),
        ("--legendary", Temper::Legendary),
    ] {
        let mut tui = Tui::new();
        tui.clear_tank();
        tui.run(&format!("/fish --no-fight {flag}"));
        let state = tui.app.fishing_state().expect("fishing");
        assert!(!state.is_catching(), "--no-fight hooks at once");
        assert_eq!(state.temper(), temper, "{flag}");
    }
}

#[test]
fn a_player_cannot_choose_how_a_fish_fights() {
    let mut tui = Tui::as_player(80, 24);
    tui.run("/fish --legendary");
    assert!(tui.app.fishing_state().is_none(), "a debug flag is refused");
}
