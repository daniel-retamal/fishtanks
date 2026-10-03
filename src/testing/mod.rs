use std::path::{Path, PathBuf};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};

use crate::app::{App, Launch, SaveFile};
use crate::economy::Money;
use crate::ledger::Flow;

mod reel;
mod screenplay;
mod stage;

pub use reel::{Arm, Flaw, Glyph, Reel, Still, TerminalCell};
pub use screenplay::{Cue, CueError, Screenplay};
pub use stage::{PLAY_EXTENSION, TerminalSize, play_name, plays_in, review, stage};

pub const DEFAULT_COLS: u16 = 100;
pub const DEFAULT_ROWS: u16 = 30;
pub const LAB_STAKE: Money = 40_000;

const SELECTION_MARKER: char = '>';
const COVERED_CELL: &str = " ";
const SELECT_MAX_STEPS: usize = 256;

pub struct Tui {
    pub app: App,
    terminal: Terminal<TestBackend>,
    reel: Reel,
    film: Option<(PathBuf, String)>,
}

impl Default for Tui {
    fn default() -> Self {
        Self::new()
    }
}

impl Tui {
    pub fn new() -> Self {
        Self::with_size(DEFAULT_COLS, DEFAULT_ROWS)
    }

    pub fn with_size(cols: u16, rows: u16) -> Self {
        Self::launched(Launch::Debug, cols, rows)
    }

    pub fn as_player(cols: u16, rows: u16) -> Self {
        Self::launched(Launch::Player, cols, rows)
    }

    pub fn launched(launch: Launch, cols: u16, rows: u16) -> Self {
        Self::playing(App::launch(launch), cols, rows)
    }

    pub fn resumed(save: SaveFile, cols: u16, rows: u16) -> Self {
        Self::playing(App::resume(save, cols, rows), cols, rows)
    }

    pub fn around(app: App, cols: u16, rows: u16) -> Self {
        Self::playing(app, cols, rows)
    }

    fn playing(app: App, cols: u16, rows: u16) -> Self {
        let terminal = Terminal::new(TestBackend::new(cols, rows)).expect("test terminal");
        let mut tui = Self {
            app,
            terminal,
            reel: Reel::new(),
            film: None,
        };
        tui.draw();
        tui
    }

    pub fn leave_debug_mode(&mut self) -> &mut Self {
        self.app.debug_mode = false;
        self.draw();
        self
    }

    pub fn stake(&mut self) -> &mut Self {
        self.app.earn(LAB_STAKE, Flow::Godsend);
        self.draw();
        self
    }

    pub fn clear_tank(&mut self) -> &mut Self {
        let tank = &mut self.app.tanks[self.app.current_tank];
        tank.fish.clear();
        tank.cows.clear();
        tank.food.clear();
        tank.used_names.clear();
        tank.used_cow_names.clear();
        self.draw();
        self
    }

    pub fn key(&mut self, code: KeyCode) -> &mut Self {
        self.app
            .handle_input(Event::Key(KeyEvent::new(code, KeyModifiers::empty())));
        self.draw();
        self
    }

    pub fn release(&mut self, code: KeyCode) -> &mut Self {
        let mut key = KeyEvent::new(code, KeyModifiers::empty());
        key.kind = KeyEventKind::Release;
        self.app.handle_input(Event::Key(key));
        self.draw();
        self
    }

    pub fn type_text(&mut self, text: &str) -> &mut Self {
        for ch in text.chars() {
            self.key(KeyCode::Char(ch));
        }
        self
    }

    pub fn run(&mut self, line: &str) -> &mut Self {
        self.app.editor.set(line.to_string());
        self.key(KeyCode::Enter)
    }

    pub fn tick_n(&mut self, ticks: usize) -> &mut Self {
        for _ in 0..ticks {
            self.app.tick();
        }
        self.draw();
        self
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> &mut Self {
        self.terminal.backend_mut().resize(cols, rows);
        self.draw();
        self
    }

    pub fn film(&mut self, dir: &Path, name: &str) -> &mut Self {
        self.film = Some((dir.to_path_buf(), name.to_string()));
        self
    }

    pub fn snap(&mut self, label: &str) -> &mut Self {
        self.draw();
        self.reel
            .push(Still::of(label, self.terminal.backend().buffer()));
        if let Some((dir, name)) = &self.film {
            self.reel.save(dir, name).expect("the reel is writable");
        }
        self
    }

    pub fn try_select(&mut self, label: &str) -> Result<(), String> {
        let marked = format!("{SELECTION_MARKER} {label}");
        let mut steps = 0;
        while !self.screen().contains(&marked) {
            if steps == SELECT_MAX_STEPS {
                return Err(format!(
                    "could not select {label:?} by walking down the list"
                ));
            }
            self.key(KeyCode::Down);
            steps += 1;
        }
        Ok(())
    }

    pub fn select(&mut self, label: &str) -> &mut Self {
        if let Err(message) = self.try_select(label) {
            panic!("{message}");
        }
        self
    }

    pub fn reel(&self) -> &Reel {
        &self.reel
    }

    pub fn screen(&mut self) -> Screen {
        self.draw();
        let buffer = self.terminal.backend().buffer();
        let rows = (buffer.area.top()..buffer.area.bottom())
            .map(|y| {
                TerminalCell::row(buffer, y)
                    .map(|shown| match shown.covered {
                        true => COVERED_CELL,
                        false => shown.cell.symbol(),
                    })
                    .collect::<String>()
            })
            .collect();
        Screen { rows }
    }

    fn draw(&mut self) {
        let app = &mut self.app;
        self.terminal.draw(|frame| app.draw(frame)).expect("draw");
    }
}

pub struct Screen {
    rows: Vec<String>,
}

impl Screen {
    pub fn rows(&self) -> &[String] {
        &self.rows
    }

    pub fn text(&self) -> String {
        self.rows.join("\n")
    }

    pub fn contains(&self, needle: &str) -> bool {
        self.rows.iter().any(|row| row.contains(needle))
    }

    pub fn find(&self, needle: &str) -> Option<(usize, usize)> {
        self.rows.iter().enumerate().find_map(|(y, row)| {
            row.find(needle)
                .map(|byte_offset| (row[..byte_offset].chars().count(), y))
        })
    }

    pub fn expect_find(&self, needle: &str) -> (usize, usize) {
        self.find(needle)
            .unwrap_or_else(|| panic!("{needle:?} is not on screen:\n{}", self.text()))
    }

    pub fn expect_absent(&self, needle: &str) {
        assert!(
            !self.contains(needle),
            "{needle:?} should not be on screen:\n{}",
            self.text()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_found_column_counts_terminal_cells_not_bytes() {
        let screen = Screen {
            rows: vec!["◦°○[-[[[[[º-".to_string()],
        };
        assert_eq!(
            screen.find("[-[[[[["),
            Some((3, 0)),
            "multi-byte background characters never shift a reported column"
        );
    }
}
