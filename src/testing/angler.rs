use crossterm::event::KeyCode;

use super::Tui;

const CAST_LIMIT_TICKS: usize = 30 * 120;
const LOOKAHEAD_STEPS: f32 = 6.0;
const DEADBAND: f32 = 0.04;
const REEL_ZONE: f32 = 0.3;
const CENTRE: f32 = 0.5;
const BITE_REACTION_TICKS: usize = 6;
const ROD_KEYS: [KeyCode; 3] = [KeyCode::Down, KeyCode::Left, KeyCode::Right];
const KEY_NAMES: [&str; 3] = ["down", "left", "right"];
const KEYS_TAG: &str = "keys=";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Angling {
    Steer,
    Hold,
}

impl Angling {
    pub fn parse(word: &str) -> Option<Self> {
        match word.to_ascii_lowercase().as_str() {
            "steer" => Some(Angling::Steer),
            "hold" => Some(Angling::Hold),
            _ => None,
        }
    }

    fn wants(self, tui: &Tui, bite_seen: usize) -> [bool; 3] {
        let Some(state) = tui.app.fishing_state() else {
            return [false; 3];
        };
        if state.is_catching() {
            return [bite_seen >= BITE_REACTION_TICKS, false, false];
        }
        if self == Angling::Hold {
            return [true, false, false];
        }
        let ahead = state.fish_pos + state.fish_velocity * LOOKAHEAD_STEPS;
        let reel = (ahead - CENTRE).abs() * 2.0 < REEL_ZONE;
        [reel, ahead > CENTRE + DEADBAND, ahead < CENTRE - DEADBAND]
    }
}

pub struct Angler {
    style: Angling,
    every: usize,
    label: String,
    held: [bool; 3],
    bite_seen: usize,
}

impl Angler {
    pub fn new(style: Angling, every: usize, label: &str) -> Self {
        Self {
            style,
            every,
            label: label.to_string(),
            held: [false; 3],
            bite_seen: 0,
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
            let wants = self.style.wants(tui, self.bite_seen);
            for (slot, &down) in wants.iter().enumerate() {
                self.hold(tui, slot, down);
            }
            tui.tick_n(1);
            if self.every > 0 && tick % self.every == 0 {
                tui.record_frame(&format!("{} {tick:04} {}", self.label, self.keys()));
            }
        }
        Err(format!("the cast outlasted {CAST_LIMIT_TICKS} ticks"))
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
    fn an_angler_who_only_holds_down_loses_every_fish() {
        assert!((0..CASTS).all(|_| !cast(Angling::Hold)));
    }
}
