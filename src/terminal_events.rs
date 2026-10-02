use std::io;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event};

pub enum Heard {
    Event(Event),
    Quiet,
    Gone(Option<io::Error>),
}

pub struct TerminalEvents {
    events: Receiver<io::Result<Event>>,
}

impl TerminalEvents {
    pub fn listen() -> Self {
        let (sender, events) = mpsc::channel();
        thread::spawn(move || {
            loop {
                let read = event::read();
                let failed = read.is_err();
                if sender.send(read).is_err() || failed {
                    return;
                }
            }
        });
        Self { events }
    }

    pub fn wait(&self, timeout: Duration) -> Heard {
        match self.events.recv_timeout(timeout) {
            Ok(Ok(event)) => Heard::Event(event),
            Ok(Err(error)) => Heard::Gone(Some(error)),
            Err(RecvTimeoutError::Timeout) => Heard::Quiet,
            Err(RecvTimeoutError::Disconnected) => Heard::Gone(None),
        }
    }

    pub fn waiting(&self) -> Option<Heard> {
        match self.events.try_recv() {
            Ok(Ok(event)) => Some(Heard::Event(event)),
            Ok(Err(error)) => Some(Heard::Gone(Some(error))),
            Err(_) => None,
        }
    }
}

#[cfg(unix)]
pub fn ask_for_key_releases() -> bool {
    use crossterm::event::{KeyboardEnhancementFlags, PushKeyboardEnhancementFlags};
    use crossterm::{execute, terminal};

    if !matches!(terminal::supports_keyboard_enhancement(), Ok(true)) {
        return false;
    }
    let flags = PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES);
    execute!(io::stdout(), flags).is_ok()
}

#[cfg(not(unix))]
pub fn ask_for_key_releases() -> bool {
    false
}

pub fn stop_asking_for_key_releases(asked: bool) {
    if !asked {
        return;
    }
    let _ = crossterm::execute!(io::stdout(), event::PopKeyboardEnhancementFlags);
}
