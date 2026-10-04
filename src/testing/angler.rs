use crossterm::event::KeyCode;

use super::Tui;
use crate::ui::fishing_overlay::FishingState;

const CAST_LIMIT_TICKS: usize = 30 * 120;
const LOOKAHEAD_STEPS: f32 = 6.0;
const DEADBAND: f32 = 0.04;
const CENTRE: f32 = 0.5;
const BITE_REACTION_TICKS: usize = 6;
const GLANCE_TICKS: usize = 5;
const SETTLE_TICKS: usize = GLANCE_TICKS;
const LEFT: usize = 1;
const RIGHT: usize = 2;
const ROD_KEYS: [KeyCode; 3] = [KeyCode::Down, KeyCode::Left, KeyCode::Right];
const KEY_NAMES: [&str; 3] = ["down", "left", "right"];
const KEYS_TAG: &str = "keys=";
const BITE_TAG: &str = "bite=";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Angling {
    Steer,
    Hold,
    Watch,
}

impl Angling {
    pub fn parse(word: &str) -> Option<Self> {
        match word.to_ascii_lowercase().as_str() {
            "steer" => Some(Angling::Steer),
            "hold" => Some(Angling::Hold),
            "watch" => Some(Angling::Watch),
            _ => None,
        }
    }
}

fn steering(state: &FishingState, held: [bool; 3]) -> (bool, bool) {
    let ahead = state.fish_pos + state.fish_velocity * LOOKAHEAD_STEPS;
    let left = ahead > CENTRE + DEADBAND;
    let right = ahead < CENTRE - DEADBAND;
    let switching = (held[LEFT] && right) || (held[RIGHT] && left);
    if switching {
        return (false, false);
    }
    (left, right)
}

pub struct Angler {
    style: Angling,
    every: usize,
    label: String,
    held: [bool; 3],
    bite_seen: usize,
    hooked_at: Option<usize>,
}

impl Angler {
    pub fn new(style: Angling, every: usize, label: &str) -> Self {
        Self {
            style,
            every,
            label: label.to_string(),
            held: [false; 3],
            bite_seen: 0,
            hooked_at: None,
        }
    }

    pub fn cast(mut self, tui: &mut Tui) -> Result<(), String> {
        if tui.app.fishing_state().is_none() {
            return Err("angle needs the fishing window open (run /fish first)".to_string());
        }
        for tick in 0..CAST_LIMIT_TICKS {
            if tui.app.fishing_state().is_none() {
                self.let_go(tui);
                tui.write_reel();
                return Ok(());
            }
            if tui
                .app
                .fishing_state()
                .is_some_and(|state| state.is_biting())
            {
                self.bite_seen += 1;
            }
            if self.hooked_at.is_none()
                && tui
                    .app
                    .fishing_state()
                    .is_some_and(|state| !state.is_catching())
            {
                self.hooked_at = Some(tick);
            }
            let wants = self.wants(tui, tick);
            for (slot, &down) in wants.iter().enumerate() {
                self.hold(tui, slot, down);
            }
            tui.tick_n(1);
            if self.every > 0 && tick % self.every == 0 {
                let biting = tui
                    .app
                    .fishing_state()
                    .is_some_and(|state| state.is_biting());
                tui.record_frame(&format!(
                    "{} {tick:04} {} {BITE_TAG}{}",
                    self.label,
                    self.keys(),
                    u8::from(biting)
                ));
            }
        }
        Err(format!("the cast outlasted {CAST_LIMIT_TICKS} ticks"))
    }

    fn wants(&self, tui: &Tui, tick: usize) -> [bool; 3] {
        let Some(state) = tui.app.fishing_state() else {
            return [false; 3];
        };
        if self.style == Angling::Watch {
            return [false; 3];
        }
        if state.is_catching() {
            return [self.bite_seen >= BITE_REACTION_TICKS, false, false];
        }
        if self.style == Angling::Hold {
            return [true, false, false];
        }
        let settling = self
            .hooked_at
            .is_some_and(|hooked| tick < hooked + SETTLE_TICKS);
        if settling {
            return [state.in_the_green(), false, false];
        }
        if !tick.is_multiple_of(GLANCE_TICKS) {
            return self.held;
        }
        let reel = state.in_the_green();
        let (left, right) = steering(state, self.held);
        [reel, left, right]
    }

    fn hold(&mut self, tui: &mut Tui, slot: usize, down: bool) {
        let was = self.held[slot];
        self.held[slot] = down;
        match (was, down) {
            (_, true) => tui.key(ROD_KEYS[slot]),
            (true, false) => tui.release(ROD_KEYS[slot]),
            (false, false) => tui,
        };
    }

    fn keys(&self) -> String {
        let held: Vec<&str> = KEY_NAMES
            .iter()
            .zip(self.held)
            .filter(|(_, down)| *down)
            .map(|(name, _)| *name)
            .collect();
        format!("{KEYS_TAG}{}", held.join(","))
    }

    fn let_go(&mut self, tui: &mut Tui) {
        for slot in 0..ROD_KEYS.len() {
            self.hold(tui, slot, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CARD_HINTS: [&str; 2] = ["ENTER capture", "ESC/q close"];
    const CASTS: usize = 5;

    fn a_card_is_up(tui: &mut Tui) -> bool {
        let screen = tui.screen();
        CARD_HINTS.iter().any(|hint| screen.contains(hint))
    }

    fn cast(style: Angling) -> bool {
        let mut tui = Tui::new();
        tui.clear_tank();
        tui.run("/fish");
        Angler::new(style, 0, "cast")
            .cast(&mut tui)
            .expect("the cast ends");
        a_card_is_up(&mut tui)
    }

    #[test]
    fn an_angler_who_steers_lands_every_fish() {
        assert!((0..CASTS).all(|_| cast(Angling::Steer)));
    }

    #[test]
    fn an_angler_who_only_watches_never_lands_a_fish() {
        assert!((0..CASTS).all(|_| !cast(Angling::Watch)));
    }

    #[test]
    fn an_angler_who_only_holds_down_loses_every_fish() {
        assert!((0..CASTS).all(|_| !cast(Angling::Hold)));
    }
}
