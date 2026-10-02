use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

pub const KEY_NAMES: &[(&str, KeyCode)] = &[
    ("enter", KeyCode::Enter),
    ("esc", KeyCode::Esc),
    ("tab", KeyCode::Tab),
    ("backtab", KeyCode::BackTab),
    ("backspace", KeyCode::Backspace),
    ("delete", KeyCode::Delete),
    ("insert", KeyCode::Insert),
    ("up", KeyCode::Up),
    ("down", KeyCode::Down),
    ("left", KeyCode::Left),
    ("right", KeyCode::Right),
    ("home", KeyCode::Home),
    ("end", KeyCode::End),
    ("pageup", KeyCode::PageUp),
    ("pagedown", KeyCode::PageDown),
    ("space", KeyCode::Char(' ')),
];

pub enum InputAction {
    Quit,
    Cancel,
    Confirm,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
    Char(char),
    Backspace,
    Delete,
    Tab,
}

pub fn classify(event: &Event) -> Option<InputAction> {
    let Event::Key(key) = event else { return None };
    if key.kind == KeyEventKind::Release {
        return None;
    }
    Some(match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => InputAction::Quit,
        KeyCode::Esc => InputAction::Cancel,
        KeyCode::Enter => InputAction::Confirm,
        KeyCode::Up => InputAction::Up,
        KeyCode::Down => InputAction::Down,
        KeyCode::Left => InputAction::Left,
        KeyCode::Right => InputAction::Right,
        KeyCode::PageUp => InputAction::PageUp,
        KeyCode::PageDown => InputAction::PageDown,
        KeyCode::Home => InputAction::Home,
        KeyCode::End => InputAction::End,
        KeyCode::Char(c) => InputAction::Char(c),
        KeyCode::Backspace => InputAction::Backspace,
        KeyCode::Delete => InputAction::Delete,
        KeyCode::Tab => InputAction::Tab,
        _ => return None,
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Hold {
    pub code: KeyCode,
    pub down: bool,
    pub quits: bool,
}

pub fn hold(event: &Event) -> Option<Hold> {
    let Event::Key(key) = event else { return None };
    let down = key.kind != KeyEventKind::Release;
    let quits =
        down && key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL);
    Some(Hold {
        code: key.code,
        down,
        quits,
    })
}

const UNLEARNED_FIRST_REPEAT_SECS: f32 = 0.5;
const UNLEARNED_REPEAT_SECS: f32 = 0.1;
const MAX_FIRST_REPEAT_SECS: f32 = 2.0;
const MAX_REPEAT_SECS: f32 = 0.5;
const REPEAT_SLACK: f32 = 1.25;
const STAMP_TICKS: f32 = 2.0;

fn is_text_key(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::Char(_) | KeyCode::Enter | KeyCode::Tab | KeyCode::Backspace
    )
}

#[derive(Clone, Copy, Default, Debug)]
struct Evidence {
    text_releases: bool,
    other_releases: bool,
    repeats_while_down: bool,
}

impl Evidence {
    fn hear_release_of(&mut self, code: KeyCode) {
        if is_text_key(code) {
            self.text_releases = true;
        } else {
            self.other_releases = true;
        }
    }

    fn releases(self, code: KeyCode) -> bool {
        self.text_releases || (self.other_releases && !is_text_key(code))
    }

    fn trusts_every_release_of(self, code: KeyCode) -> bool {
        self.releases(code) && self.repeats_while_down
    }
}

#[derive(Clone, Copy)]
struct HeldKey {
    code: KeyCode,
    seen: f32,
    repeated: bool,
    down: bool,
    fresh: bool,
}

pub struct HeldKeys {
    evidence: Evidence,
    clock: f32,
    tick_secs: f32,
    first_repeat: f32,
    repeat: f32,
    keys: Vec<HeldKey>,
    silenced: Vec<HeldKey>,
    last_down: Option<KeyCode>,
}

impl Default for HeldKeys {
    fn default() -> Self {
        Self {
            evidence: Evidence::default(),
            clock: 0.0,
            tick_secs: 0.0,
            first_repeat: UNLEARNED_FIRST_REPEAT_SECS,
            repeat: UNLEARNED_REPEAT_SECS,
            keys: Vec::new(),
            silenced: Vec::new(),
            last_down: None,
        }
    }
}

impl HeldKeys {
    pub fn hear(&mut self, event: &Event) {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Release
        {
            self.evidence.hear_release_of(key.code);
        }
    }

    fn held_by_release(&self, key: &HeldKey) -> bool {
        key.down && self.evidence.releases(key.code)
    }

    pub fn press(&mut self, code: KeyCode) -> Vec<KeyCode> {
        let lifted = self.lift_the_silent_but(code);
        self.last_down = Some(code);
        let mut key = HeldKey {
            code,
            seen: self.clock,
            repeated: false,
            down: true,
            fresh: true,
        };
        if let Some(before) = self.take(code) {
            if !before.fresh {
                self.learn(&before);
            }
            if before.down {
                self.evidence.repeats_while_down = true;
            }
            key.repeated = true;
        }
        self.keys.push(key);
        lifted
    }

    fn take(&mut self, code: KeyCode) -> Option<HeldKey> {
        if let Some(at) = self.keys.iter().position(|key| key.code == code) {
            return Some(self.keys.remove(at));
        }
        let at = self.silenced.iter().position(|key| key.code == code)?;
        Some(self.silenced.remove(at))
    }

    fn learn(&mut self, before: &HeldKey) {
        let gap = self.clock - before.seen;
        if before.repeated {
            self.repeat = gap.min(MAX_REPEAT_SECS);
        } else {
            self.first_repeat = gap.min(MAX_FIRST_REPEAT_SECS);
        }
    }

    fn lift_the_silent_but(&mut self, code: KeyCode) -> Vec<KeyCode> {
        let (kept, lifted): (Vec<HeldKey>, Vec<HeldKey>) = std::mem::take(&mut self.keys)
            .into_iter()
            .partition(|key| key.code == code || self.held_by_release(key));
        self.keys = kept;
        lifted.into_iter().map(|key| key.code).collect()
    }

    pub fn release(&mut self, code: KeyCode) -> bool {
        let trusted = self.evidence.trusts_every_release_of(code);
        let Some(at) = self.keys.iter().position(|key| key.code == code) else {
            return false;
        };
        let key = &mut self.keys[at];
        key.down = false;
        if key.fresh && !trusted {
            return false;
        }
        self.keys.remove(at);
        true
    }

    pub fn is_down(&self, code: KeyCode) -> bool {
        self.keys.iter().any(|key| key.code == code)
    }

    pub fn is_certainly_down(&self, code: KeyCode) -> bool {
        self.keys.iter().any(|key| {
            key.code == code
                && (self.held_by_release(key) || self.clock - key.seen <= self.repeat_window())
        })
    }

    pub fn tick(&mut self, dt: f32) -> Vec<KeyCode> {
        self.clock += dt;
        self.tick_secs = dt;
        let clock = self.clock;
        self.silenced
            .retain(|key| clock - key.seen <= MAX_FIRST_REPEAT_SECS);
        let (kept, lifted): (Vec<HeldKey>, Vec<HeldKey>) = std::mem::take(&mut self.keys)
            .into_iter()
            .partition(|key| self.keeps(key));
        self.keys = kept;
        for key in &mut self.keys {
            key.fresh = false;
        }
        self.silenced.extend(lifted.iter().copied());
        lifted.into_iter().map(|key| key.code).collect()
    }

    fn keeps(&self, key: &HeldKey) -> bool {
        if self.clock - key.seen <= self.window(key) {
            return true;
        }
        if !self.held_by_release(key) {
            return false;
        }
        let repeats_when_held = self.evidence.trusts_every_release_of(key.code);
        !(repeats_when_held && self.last_down == Some(key.code))
    }

    pub fn let_go(&mut self) -> Vec<KeyCode> {
        self.silenced.clear();
        std::mem::take(&mut self.keys)
            .into_iter()
            .map(|key| key.code)
            .collect()
    }

    fn window(&self, key: &HeldKey) -> f32 {
        if key.repeated {
            return self.repeat_window();
        }
        self.first_repeat * REPEAT_SLACK + STAMP_TICKS * self.tick_secs
    }

    fn repeat_window(&self) -> f32 {
        self.repeat * REPEAT_SLACK + STAMP_TICKS * self.tick_secs
    }
}

pub fn key_code(name: &str) -> Option<KeyCode> {
    let lower = name.trim().to_lowercase();
    if let Some(&(_, code)) = KEY_NAMES.iter().find(|(named, _)| *named == lower) {
        return Some(code);
    }
    let mut chars = lower.chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) => Some(KeyCode::Char(ch)),
        _ => None,
    }
}

pub fn key_name(code: KeyCode) -> Option<String> {
    if let Some(&(name, _)) = KEY_NAMES.iter().find(|(_, named)| *named == code) {
        return Some(name.to_string());
    }
    let KeyCode::Char(ch) = code else {
        return None;
    };
    Some(ch.to_lowercase().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;

    fn event(code: KeyCode, kind: KeyEventKind) -> Event {
        let mut key = KeyEvent::new(code, KeyModifiers::empty());
        key.kind = kind;
        Event::Key(key)
    }

    #[test]
    fn every_named_key_spells_itself_both_ways() {
        for &(name, code) in KEY_NAMES {
            assert_eq!(key_code(name), Some(code), "{name}");
            assert_eq!(key_name(code).as_deref(), Some(name), "{name}");
        }
    }

    #[test]
    fn any_other_character_is_itself_lowercased_so_shift_never_strands_a_key() {
        assert_eq!(key_name(KeyCode::Char('W')).as_deref(), Some("w"));
        assert_eq!(key_name(KeyCode::Char('w')).as_deref(), Some("w"));
        assert_eq!(key_code(" W "), Some(KeyCode::Char('w')));
        assert_eq!(
            key_code("spacebar"),
            None,
            "a word that is no key names none"
        );
    }

    #[test]
    fn a_press_and_a_repeat_hold_a_key_down_and_a_release_lets_it_go() {
        for (kind, down) in [
            (KeyEventKind::Press, true),
            (KeyEventKind::Repeat, true),
            (KeyEventKind::Release, false),
        ] {
            let held = hold(&event(KeyCode::Up, kind)).expect("a key event");
            assert_eq!(held.down, down, "{kind:?}");
            assert!(!held.quits);
        }
        assert_eq!(hold(&Event::FocusLost), None);
    }

    #[test]
    fn only_a_pressed_control_c_quits() {
        let mut key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(hold(&Event::Key(key)).expect("a key").quits);
        key.kind = KeyEventKind::Release;
        assert!(!hold(&Event::Key(key)).expect("a key").quits);
    }

    const TICK: f32 = 1.0 / 30.0;
    const LEARNED_EPSILON: f32 = 1e-4;

    fn ticks(keys: &mut HeldKeys, n: usize) -> Vec<KeyCode> {
        (0..n).flat_map(|_| keys.tick(TICK)).collect()
    }

    fn secs(n: usize) -> f32 {
        n as f32 * TICK
    }

    #[test]
    fn a_terminal_that_sends_releases_holds_a_key_until_its_release() {
        let mut keys = HeldKeys::default();
        keys.hear(&event(KeyCode::Down, KeyEventKind::Release));
        keys.press(KeyCode::Down);
        assert!(keys.press(KeyCode::Left).is_empty(), "two keys are held");
        assert!(ticks(&mut keys, 300).is_empty(), "no repeat is needed");
        assert!(keys.is_certainly_down(KeyCode::Down));
        assert!(keys.is_down(KeyCode::Left));
        assert!(keys.release(KeyCode::Down));
        assert!(!keys.is_down(KeyCode::Down));
    }

    #[test]
    fn an_arrow_release_proves_the_arrows_release_and_not_the_text_keys() {
        let mut keys = HeldKeys::default();
        keys.hear(&event(KeyCode::Up, KeyEventKind::Release));
        keys.press(KeyCode::Down);
        keys.press(KeyCode::Char(' '));
        assert_eq!(
            keys.press(KeyCode::Left),
            vec![KeyCode::Char(' ')],
            "only a key that cannot say it is up is lifted by another press"
        );
        assert_eq!(ticks(&mut keys, 300), Vec::<KeyCode>::new());
        assert!(keys.is_certainly_down(KeyCode::Down));
        assert!(keys.is_certainly_down(KeyCode::Left));
    }

    #[test]
    fn a_tap_on_a_terminal_that_never_sends_a_release_lets_go_on_its_own() {
        let mut keys = HeldKeys::default();
        keys.hear(&event(KeyCode::Down, KeyEventKind::Press));
        keys.press(KeyCode::Down);
        let window = UNLEARNED_FIRST_REPEAT_SECS * REPEAT_SLACK + STAMP_TICKS * TICK;
        let held = (window / TICK) as usize;
        assert!(
            ticks(&mut keys, held).is_empty(),
            "held until a repeat is due"
        );
        assert!(keys.is_down(KeyCode::Down));
        assert_eq!(ticks(&mut keys, 2), vec![KeyCode::Down]);
        assert!(!keys.is_down(KeyCode::Down));
    }

    #[test]
    fn a_press_with_no_repeat_is_certainly_down_only_for_one_repeat() {
        let mut keys = HeldKeys::default();
        keys.press(KeyCode::Down);
        ticks(&mut keys, 1);
        assert!(keys.is_certainly_down(KeyCode::Down), "just pressed");
        let certain = UNLEARNED_REPEAT_SECS * REPEAT_SLACK + STAMP_TICKS * TICK;
        ticks(&mut keys, (certain / TICK) as usize + 1);
        assert!(keys.is_down(KeyCode::Down), "it may still be held");
        assert!(
            !keys.is_certainly_down(KeyCode::Down),
            "but nothing has proved it"
        );
        keys.press(KeyCode::Down);
        assert!(keys.is_certainly_down(KeyCode::Down), "a repeat proves it");
    }

    #[test]
    fn a_key_held_through_its_repeats_stays_down_and_lets_go_soon_after_the_last() {
        const FIRST_REPEAT: usize = 11;
        const REPEAT: usize = 3;
        let mut keys = HeldKeys::default();
        keys.press(KeyCode::Left);
        for tick in 1..=120 {
            keys.tick(TICK);
            let since_first = tick as isize - FIRST_REPEAT as isize;
            if since_first >= 0 && (since_first as usize).is_multiple_of(REPEAT) {
                assert!(
                    keys.press(KeyCode::Left).is_empty(),
                    "a repeat lifts nothing"
                );
            }
            assert!(keys.is_down(KeyCode::Left), "tick {tick}: still held");
            if since_first >= 0 {
                assert!(keys.is_certainly_down(KeyCode::Left), "tick {tick}");
            }
        }
        assert!((keys.first_repeat - secs(FIRST_REPEAT)).abs() < LEARNED_EPSILON);
        assert!((keys.repeat - secs(REPEAT)).abs() < LEARNED_EPSILON);
        let lag = (secs(REPEAT) * REPEAT_SLACK + STAMP_TICKS * TICK) / TICK;
        let lifted = ticks(&mut keys, lag.ceil() as usize + 1);
        assert_eq!(lifted, vec![KeyCode::Left], "let go within {lag} ticks");
    }

    #[test]
    fn a_terminal_that_never_sends_a_release_holds_only_the_last_key_pressed() {
        let mut keys = HeldKeys::default();
        keys.press(KeyCode::Down);
        assert_eq!(keys.press(KeyCode::Right), vec![KeyCode::Down]);
        assert!(!keys.is_down(KeyCode::Down), "its repeats stopped for good");
        assert!(keys.is_down(KeyCode::Right));
    }

    #[test]
    fn letting_go_empties_the_hand() {
        let mut keys = HeldKeys::default();
        keys.hear(&event(KeyCode::Left, KeyEventKind::Release));
        keys.press(KeyCode::Left);
        keys.press(KeyCode::Down);
        assert_eq!(keys.let_go(), vec![KeyCode::Left, KeyCode::Down]);
        assert!(!keys.is_down(KeyCode::Left));
        assert!(!keys.release(KeyCode::Down), "nothing is left to release");
    }

    const FIRST_REPEAT_TICKS: usize = 15;
    const REPEAT_TICKS: usize = 1;

    fn hold_through_repeats(keys: &mut HeldKeys, code: KeyCode, held: usize, pairs: bool) {
        for tick in 1..=held {
            ticks(keys, 1);
            let due = tick >= FIRST_REPEAT_TICKS
                && (tick - FIRST_REPEAT_TICKS).is_multiple_of(REPEAT_TICKS);
            if !due {
                continue;
            }
            keys.press(code);
            if pairs {
                keys.hear(&event(code, KeyEventKind::Release));
                keys.release(code);
            }
        }
    }

    #[test]
    fn a_release_in_the_frame_of_its_press_proves_nothing_so_a_key_held_through_paired_repeats_stays_down()
     {
        let mut keys = HeldKeys::default();
        keys.press(KeyCode::Down);
        keys.hear(&event(KeyCode::Down, KeyEventKind::Release));
        assert!(
            !keys.release(KeyCode::Down),
            "a key-up in the frame of its key-down may be the terminal's"
        );
        assert!(keys.is_down(KeyCode::Down));
        for tick in 1..=120 {
            ticks(&mut keys, 1);
            if tick >= FIRST_REPEAT_TICKS {
                keys.press(KeyCode::Down);
                keys.hear(&event(KeyCode::Down, KeyEventKind::Release));
                assert!(!keys.release(KeyCode::Down));
                assert!(keys.is_certainly_down(KeyCode::Down), "tick {tick}");
            }
            assert!(keys.is_down(KeyCode::Down), "tick {tick}: still held");
        }
        let lag = (secs(REPEAT_TICKS) * REPEAT_SLACK + STAMP_TICKS * TICK) / TICK;
        assert_eq!(
            ticks(&mut keys, lag.ceil() as usize + 1),
            vec![KeyCode::Down],
            "and it lets go soon after its last repeat"
        );
    }

    #[test]
    fn a_release_after_a_frame_of_hold_lets_go_at_once() {
        let mut keys = HeldKeys::default();
        keys.press(KeyCode::Left);
        ticks(&mut keys, 1);
        keys.hear(&event(KeyCode::Left, KeyEventKind::Release));
        assert!(keys.release(KeyCode::Left));
        assert!(!keys.is_down(KeyCode::Left));
    }

    #[test]
    fn a_key_that_repeats_while_down_proves_every_release_of_its_kind() {
        let mut keys = HeldKeys::default();
        keys.hear(&event(KeyCode::Down, KeyEventKind::Release));
        keys.press(KeyCode::Down);
        hold_through_repeats(&mut keys, KeyCode::Down, FIRST_REPEAT_TICKS, false);
        keys.release(KeyCode::Down);
        keys.press(KeyCode::Left);
        assert!(
            keys.release(KeyCode::Left),
            "a quick tap on a terminal that repeats without key-ups is up at once"
        );
        keys.press(KeyCode::Char('w'));
        assert!(
            !keys.release(KeyCode::Char('w')),
            "a text key has proved nothing yet"
        );
    }

    #[test]
    fn a_lost_release_cannot_hold_the_last_key_down_for_ever() {
        let mut keys = HeldKeys::default();
        keys.hear(&event(KeyCode::Left, KeyEventKind::Release));
        keys.press(KeyCode::Down);
        keys.press(KeyCode::Left);
        hold_through_repeats(&mut keys, KeyCode::Left, 60, false);
        assert!(keys.is_down(KeyCode::Left));
        let lag = (secs(REPEAT_TICKS) * REPEAT_SLACK + STAMP_TICKS * TICK) / TICK;
        assert_eq!(
            ticks(&mut keys, lag.ceil() as usize + 1),
            vec![KeyCode::Left],
            "the last key pressed repeats while it is down, so its silence is its release"
        );
        assert!(
            keys.is_certainly_down(KeyCode::Down),
            "a key held under it never repeats, so only its key-up lets it go"
        );
    }

    #[test]
    fn repeats_that_arrive_in_one_frame_teach_no_timing() {
        let mut keys = HeldKeys::default();
        keys.press(KeyCode::Right);
        ticks(&mut keys, FIRST_REPEAT_TICKS);
        keys.press(KeyCode::Right);
        let learned = keys.first_repeat;
        keys.press(KeyCode::Right);
        keys.press(KeyCode::Right);
        assert!((keys.repeat - UNLEARNED_REPEAT_SECS).abs() < LEARNED_EPSILON);
        assert!((keys.first_repeat - learned).abs() < LEARNED_EPSILON);
    }

    #[test]
    fn a_first_repeat_slower_than_its_window_is_learned_when_it_comes() {
        let mut keys = HeldKeys::default();
        keys.press(KeyCode::Right);
        ticks(&mut keys, 3);
        keys.press(KeyCode::Right);
        let slow_first_repeat = 30;
        keys.let_go();
        keys.press(KeyCode::Right);
        let lifted = ticks(&mut keys, slow_first_repeat);
        assert_eq!(lifted, vec![KeyCode::Right], "a guess that was too short");
        keys.press(KeyCode::Right);
        assert!((keys.first_repeat - secs(slow_first_repeat)).abs() < LEARNED_EPSILON);
        keys.let_go();
        keys.press(KeyCode::Right);
        assert!(ticks(&mut keys, slow_first_repeat).is_empty());
    }

    #[test]
    fn classify_takes_a_press_and_a_repeat_and_never_a_release() {
        assert!(classify(&event(KeyCode::Char('x'), KeyEventKind::Release)).is_none());
        assert!(classify(&event(KeyCode::Char('x'), KeyEventKind::Repeat)).is_some());
        assert!(classify(&event(KeyCode::Char('x'), KeyEventKind::Press)).is_some());
    }
}
