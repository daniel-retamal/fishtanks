use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard};

use crossterm::event::KeyCode;

use crate::keyboard::Keyboard;

#[derive(Default)]
struct Touch {
    down: HashSet<KeyCode>,
    granted: bool,
    asks: u32,
}

#[derive(Clone, Default)]
pub struct Fingers {
    touch: Arc<Mutex<Touch>>,
}

impl Fingers {
    pub fn granted() -> Self {
        let fingers = Self::default();
        fingers.touch().granted = true;
        fingers
    }

    pub fn withheld() -> Self {
        Self::default()
    }

    fn touch(&self) -> MutexGuard<'_, Touch> {
        self.touch.lock().expect("one test holds the keyboard")
    }

    pub fn press(&self, code: KeyCode) {
        self.touch().down.insert(code);
    }

    pub fn lift(&self, code: KeyCode) {
        self.touch().down.remove(&code);
    }

    pub fn asks(&self) -> u32 {
        self.touch().asks
    }

    pub fn keyboard(&self) -> Box<dyn Keyboard> {
        Box::new(self.clone())
    }
}

impl Keyboard for Fingers {
    fn is_down(&self, code: KeyCode) -> Option<bool> {
        if matches!(code, KeyCode::Char(ch) if ch != ' ') {
            return None;
        }
        let touch = self.touch();
        Some(touch.granted && touch.down.contains(&code))
    }

    fn ask(&self) {
        self.touch().asks += 1;
    }
}
