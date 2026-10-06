use std::path::Path;

use crossterm::event::{Event, KeyCode};
use fishtank::app::LESSON_STREAK;
use fishtank::testing::{Fingers, Tui};
use fishtank::ui::fishing_overlay::{LESSON_PACE, Temper};

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
    fingers: Fingers,
}

impl Hand {
    fn new(terminal: Emulator, cadence: Cadence) -> Self {
        Self::feeling(terminal, cadence, Fingers::withheld())
    }

    fn feeling(terminal: Emulator, cadence: Cadence, fingers: Fingers) -> Self {
        Self {
            terminal,
            cadence,
            held: Vec::new(),
            fingers,
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
                self.fingers.press(code);
                self.strike(tui, code);
                self.held.push((code, 0));
            }
            (Some(i), false) => {
                self.held.remove(i);
                self.fingers.lift(code);
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
            self.fingers.lift(code);
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
    cast_like_a_player(tui, hand, style, "/fish --legendary")
}

fn cast_like_a_player(tui: &mut Tui, hand: &mut Hand, style: Style, line: &str) -> bool {
    hand.run(tui, line);
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
    landed_casts_feeling(terminal, cadence, style, Fingers::withheld())
}

fn landed_casts_feeling(
    terminal: Emulator,
    cadence: Cadence,
    style: Style,
    fingers: Fingers,
) -> usize {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.feel(&fingers);
    let mut hand = Hand::feeling(terminal, cadence, fingers);
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
fn terminal_app_with_input_monitoring_reels_while_steering_like_windows() {
    for cadence in MAC_CADENCES {
        assert_eq!(
            landed_casts_feeling(
                Emulator::AppleTerminal,
                cadence,
                Style::ReelWhileSteering,
                Fingers::granted()
            ),
            CASTS,
            "{cadence:?}"
        );
    }
}

#[test]
fn a_multiplexer_whose_every_key_up_comes_with_its_key_down_reels_while_steering_on_windows() {
    for cadence in [WINDOWS_CADENCE].into_iter().chain(MAC_CADENCES) {
        assert_eq!(
            landed_casts_feeling(
                Emulator::ConPty,
                cadence,
                Style::ReelWhileSteering,
                Fingers::granted()
            ),
            CASTS,
            "{cadence:?}"
        );
    }
}

fn hold_down_then_steer(terminal: Emulator, fingers: Fingers) -> (bool, Tui, Hand) {
    hold_down_then_steer_after(terminal, fingers, &[])
}

fn hold_down_then_steer_after(
    terminal: Emulator,
    fingers: Fingers,
    lines: &[&str],
) -> (bool, Tui, Hand) {
    let mut tui = Tui::new();
    tui.feel(&fingers);
    let mut hand = Hand::feeling(terminal, WINDOWS_CADENCE, fingers);
    for line in lines {
        hand.run(&mut tui, line);
    }
    tui.clear_tank();
    hand.run(&mut tui, "/fish --no-escape");
    for _ in 0..CAST_TICKS {
        if tui.app.fishing_state().is_some_and(|s| s.is_biting()) {
            break;
        }
        hand.tick(&mut tui);
    }
    hand.hold(&mut tui, KeyCode::Down, true);
    for _ in 0..TWO_SECONDS {
        hand.tick(&mut tui);
    }
    hand.hold(&mut tui, KeyCode::Left, true);
    let mut kept_reeling = true;
    for _ in 0..TWO_SECONDS {
        hand.tick(&mut tui);
        let state = tui.app.fishing_state().expect("no escape");
        kept_reeling &= state.is_reeling && state.is_pushing_left;
    }
    (kept_reeling, tui, hand)
}

#[test]
fn holding_down_and_pressing_a_side_keeps_reeling_wherever_the_keyboard_can_be_read() {
    for terminal in [Emulator::ConPty, Emulator::AppleTerminal] {
        let (kept_reeling, mut tui, mut hand) = hold_down_then_steer(terminal, Fingers::granted());
        assert!(kept_reeling, "{terminal:?}: ↓ stays down under ←");
        hand.hold(&mut tui, KeyCode::Down, false);
        hand.tick(&mut tui);
        assert!(
            !reeling(&tui),
            "{terminal:?}: and is up the frame it is let go"
        );
    }
}

#[test]
fn a_new_game_keeps_the_keyboard_it_was_reading() {
    let (kept_reeling, _, _) =
        hold_down_then_steer_after(Emulator::ConPty, Fingers::granted(), &["/reset"]);
    assert!(
        kept_reeling,
        "/reset and /import start a game, not a terminal"
    );
}

#[test]
fn a_terminal_that_hides_key_ups_asks_for_the_keyboard_once() {
    let fingers = Fingers::withheld();
    let (kept_reeling, mut tui, mut hand) =
        hold_down_then_steer(Emulator::AppleTerminal, fingers.clone());
    assert!(!kept_reeling, "an unread keyboard cannot see ↓ under ←");
    assert_eq!(fingers.asks(), 0, "never in the middle of a fight");
    hand.let_go(&mut tui);
    tui.key(KeyCode::Esc);
    assert_eq!(fingers.asks(), 1, "asked as the cast ends");
    hold_down_then_steer_again(&mut tui, &mut hand);
    tui.key(KeyCode::Esc);
    assert_eq!(fingers.asks(), 1, "and only once");
}

fn hold_down_then_steer_again(tui: &mut Tui, hand: &mut Hand) {
    hand.run(tui, "/fish --no-fight");
    hand.hold(tui, KeyCode::Down, true);
    for _ in 0..TWO_SECONDS {
        hand.tick(tui);
    }
    hand.let_go(tui);
}

#[test]
fn a_terminal_that_reports_key_ups_or_a_keyboard_already_read_never_asks() {
    for (terminal, fingers) in [
        (Emulator::Kitty, Fingers::withheld()),
        (Emulator::Windows, Fingers::withheld()),
        (Emulator::AppleTerminal, Fingers::granted()),
    ] {
        let (_, mut tui, mut hand) = hold_down_then_steer(terminal, fingers.clone());
        hand.let_go(&mut tui);
        tui.key(KeyCode::Esc);
        assert_eq!(fingers.asks(), 0, "{terminal:?}");
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

#[test]
fn a_new_player_fights_the_calmest_fish_until_three_are_landed_in_a_row() {
    let mut tui = Tui::as_player(80, 24);
    let mut hand = Hand::new(Emulator::Windows, WINDOWS_CADENCE);
    let mut casts = 0;
    while tui.app.lessons().learning() {
        let streak = tui.app.lessons().streak();
        hand.run(&mut tui, "/fish");
        tui.key(KeyCode::Esc);
        assert_eq!(
            tui.app.lessons().streak(),
            streak,
            "a cast that hooked nothing is no lesson"
        );
        casts += 1;
        assert!(casts <= CASTS, "the lessons never ended");
        if cast_like_a_player(&mut tui, &mut hand, Style::ReelWhileSteering, "/fish") {
            tui.type_text(&format!("Lesson {casts}"));
            tui.key(KeyCode::Enter);
        }
    }
    assert_eq!(tui.app.lessons().streak(), LESSON_STREAK);
}

#[test]
fn a_fish_let_go_in_the_middle_of_a_lesson_starts_the_streak_again() {
    let mut tui = Tui::new();
    tui.clear_tank();
    let mut hand = Hand::new(Emulator::Windows, WINDOWS_CADENCE);
    assert!(cast_like_a_player(
        &mut tui,
        &mut hand,
        Style::ReelWhileSteering,
        "/fish"
    ));
    tui.type_text("First");
    tui.key(KeyCode::Enter);
    assert_eq!(tui.app.lessons().streak(), 1);

    hook_a_fish(&mut tui);
    let state = tui.app.fishing_state().expect("the fish is hooked");
    assert_eq!(
        state.pace(),
        LESSON_PACE,
        "a lesson fights at the calmest pace"
    );
    tui.key(KeyCode::Esc);
    assert_eq!(
        tui.app.lessons().streak(),
        0,
        "giving up a fight is losing it"
    );
}
