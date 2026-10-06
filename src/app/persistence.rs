use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyEventKind};

use crate::fishes::species::FishSpecies;
use crate::ui::notice::NoticeState;
use crate::vault::{self, Crypt, Letter, Loaded, Opened, Scribe, Vault, VaultError};

use super::snapshot::Dead;
use super::{App, Launch, Overlay, SaveFile};

pub const HEARTBEAT: Duration = Duration::from_secs(30);
pub const SETTLE_AFTER_A_KEY: Duration = Duration::from_secs(1);
const UNSAVED_LABEL: &str = "unsaved";
const BEFORE_RESET: &str = "before-reset";
const BEFORE_IMPORT: &str = "before-import";

#[derive(Clone, Copy, Debug)]
pub struct Cadence {
    last_saved: Instant,
    last_key: Option<Instant>,
}

impl Cadence {
    pub fn starting(now: Instant) -> Self {
        Self {
            last_saved: now,
            last_key: Some(now),
        }
    }

    pub fn pressed(&mut self, now: Instant) {
        self.last_key = Some(now);
    }

    pub fn due(&self, now: Instant) -> bool {
        now.duration_since(self.last_saved) >= HEARTBEAT
            || self
                .last_key
                .is_some_and(|key| now.duration_since(key) >= SETTLE_AFTER_A_KEY)
    }

    pub fn saved(&mut self, now: Instant) {
        self.last_saved = now;
        self.last_key = None;
    }
}

pub struct Persistence {
    scribe: Scribe,
    vault: Vault,
    cadence: Cadence,
}

struct Awakening {
    app: App,
    crypt: Crypt,
    forged: bool,
    set_aside: Option<PathBuf>,
}

impl App {
    pub fn open(launch: Launch, vault: Vault, width: u16, height: u16) -> Result<Self, VaultError> {
        let debug = launch == Launch::Debug;
        let Awakening {
            mut app,
            crypt,
            forged,
            set_aside,
        } = match vault.load()? {
            Loaded::Resumed(opened) => Self::awaken(*opened, width, height),
            Loaded::Fresh => Self::begin(debug, &vault, None),
            Loaded::Unreadable { set_aside } => Self::begin(debug, &vault, Some(set_aside)),
        };
        app.debug_mode |= debug;
        app.persistence = Some(Persistence {
            scribe: vault.scribe(crypt),
            vault,
            cadence: Cadence::starting(Instant::now()),
        });
        if forged {
            app.mint_forged_cheatfish();
        }
        if let Some(path) = set_aside {
            app.set_overlay(Overlay::Notice(NoticeState::save_set_aside(&path)));
        }
        Ok(app)
    }

    fn awaken(opened: Opened, width: u16, height: u16) -> Awakening {
        let Opened {
            save,
            intact,
            crypt,
        } = opened;
        let mut app = Self::resume(save, width, height);
        app.graveyard = std::mem::take(&mut app.graveyard).entombed_up_to(crypt.holds());
        Awakening {
            app,
            crypt,
            forged: !intact,
            set_aside: None,
        }
    }

    fn begin(debug: bool, vault: &Vault, set_aside: Option<PathBuf>) -> Awakening {
        Awakening {
            app: Self::new_game(debug),
            crypt: Crypt::empty(&vault.path()),
            forged: false,
            set_aside,
        }
    }

    pub(super) fn note_input(&mut self, event: &Event) {
        let Event::Key(key) = event else {
            return;
        };
        if key.kind != KeyEventKind::Press {
            return;
        }
        if let Some(persistence) = &mut self.persistence {
            persistence.cadence.pressed(Instant::now());
        }
    }

    pub fn persist_if_due(&mut self) {
        let now = Instant::now();
        if self
            .persistence
            .as_ref()
            .is_some_and(|persistence| persistence.cadence.due(now))
        {
            self.persist();
        }
    }

    pub fn persist(&mut self) {
        if self.persistence.is_none() {
            return;
        }
        let save = self.capture(Dead::InTheCrypt);
        let burial = self.graveyard.burial();
        let Some(persistence) = &mut self.persistence else {
            return;
        };
        persistence.scribe.post(Letter { save, burial });
        persistence.cadence.saved(Instant::now());
    }

    pub fn persist_and_wait(&mut self) -> Result<(), String> {
        self.persist();
        let Some(persistence) = &self.persistence else {
            return Ok(());
        };
        persistence.scribe.flush();
        if !persistence.scribe.is_failing() {
            return Ok(());
        }
        Err(persistence.scribe.last_error().unwrap_or_default())
    }

    fn mint_forged_cheatfish(&mut self) {
        self.mint_cheatfish(FishSpecies::Cheatfish.display_name().to_string());
    }

    pub(super) fn unsaved_label(&self) -> Option<&'static str> {
        self.persistence
            .as_ref()
            .is_some_and(|persistence| persistence.scribe.is_failing())
            .then_some(UNSAVED_LABEL)
    }

    pub(super) fn reset_game(&mut self) -> bool {
        self.set_aside(BEFORE_RESET);
        self.replace_game(SaveFile::new_game(self.debug_mode));
        self.persist();
        true
    }

    pub(super) fn export_game(&self, path: &str) -> bool {
        vault::write_save(Path::new(path), &self.snapshot()).is_ok()
    }

    pub(super) fn import_game(&mut self, path: &str) -> bool {
        let Ok(opened) = vault::read_save(Path::new(path)) else {
            return false;
        };
        self.set_aside(BEFORE_IMPORT);
        self.replace_game(opened.save);
        if !opened.intact {
            self.mint_forged_cheatfish();
        }
        self.persist();
        true
    }

    fn set_aside(&self, label: &str) {
        if let Some(persistence) = &self.persistence {
            let _ = persistence.vault.set_aside(label, &self.snapshot());
        }
    }

    fn replace_game(&mut self, save: SaveFile) {
        let debug_mode = self.debug_mode;
        let persistence = self.persistence.take();
        let held_keys = std::mem::take(&mut self.held_keys);
        *self = Self::resume(save, self.terminal_width, self.terminal_height);
        self.debug_mode |= debug_mode;
        self.persistence = persistence;
        self.held_keys = held_keys;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A_BEAT: Duration = Duration::from_millis(100);

    #[test]
    fn a_fresh_game_writes_itself_down_once_it_settles() {
        let start = Instant::now();
        let cadence = Cadence::starting(start);

        assert!(!cadence.due(start));
        assert!(cadence.due(start + SETTLE_AFTER_A_KEY));
    }

    #[test]
    fn typing_never_saves_until_the_keys_rest() {
        let start = Instant::now();
        let mut cadence = Cadence::starting(start);
        cadence.saved(start);
        let mut now = start;
        while now < start + HEARTBEAT - A_BEAT {
            now += A_BEAT;
            cadence.pressed(now);
            assert!(!cadence.due(now), "a key {:?} in", now - start);
        }

        assert!(cadence.due(now + SETTLE_AFTER_A_KEY));
    }

    #[test]
    fn a_game_left_alone_saves_on_the_heartbeat() {
        let start = Instant::now();
        let mut cadence = Cadence::starting(start);
        cadence.saved(start);

        assert!(!cadence.due(start + HEARTBEAT - A_BEAT));
        assert!(cadence.due(start + HEARTBEAT));
    }

    #[test]
    fn a_key_held_down_for_ever_still_saves_on_the_heartbeat() {
        let start = Instant::now();
        let mut cadence = Cadence::starting(start);
        cadence.saved(start);
        cadence.pressed(start + HEARTBEAT);

        assert!(cadence.due(start + HEARTBEAT));
    }
}
