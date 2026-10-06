use crossterm::event::KeyCode;

const REMOTE_SESSION: [&str; 2] = ["SSH_CONNECTION", "SSH_TTY"];

pub trait Keyboard: Send {
    fn is_down(&self, code: KeyCode) -> Option<bool>;

    fn ask(&self) {}
}

pub struct Numb;

impl Keyboard for Numb {
    fn is_down(&self, _code: KeyCode) -> Option<bool> {
        None
    }
}

pub fn this_machines() -> Box<dyn Keyboard> {
    if REMOTE_SESSION
        .iter()
        .any(|name| std::env::var_os(name).is_some())
    {
        return Box::new(Numb);
    }
    platform::keyboard()
}

#[cfg(any(windows, target_os = "macos"))]
fn key_number(keys: &[(KeyCode, u16)], code: KeyCode) -> Option<u16> {
    keys.iter()
        .find(|(named, _)| *named == code)
        .map(|&(_, number)| number)
}

#[cfg(windows)]
mod platform {
    use crossterm::event::KeyCode;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_BACK, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_INSERT,
        VK_LEFT, VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SPACE, VK_TAB, VK_UP,
    };

    use super::{Keyboard, key_number};

    const VIRTUAL_KEYS: [(KeyCode, u16); 15] = [
        (KeyCode::Up, VK_UP),
        (KeyCode::Down, VK_DOWN),
        (KeyCode::Left, VK_LEFT),
        (KeyCode::Right, VK_RIGHT),
        (KeyCode::Enter, VK_RETURN),
        (KeyCode::Esc, VK_ESCAPE),
        (KeyCode::Tab, VK_TAB),
        (KeyCode::Backspace, VK_BACK),
        (KeyCode::Char(' '), VK_SPACE),
        (KeyCode::Home, VK_HOME),
        (KeyCode::End, VK_END),
        (KeyCode::PageUp, VK_PRIOR),
        (KeyCode::PageDown, VK_NEXT),
        (KeyCode::Insert, VK_INSERT),
        (KeyCode::Delete, VK_DELETE),
    ];

    struct AsyncKeyState;

    impl Keyboard for AsyncKeyState {
        fn is_down(&self, code: KeyCode) -> Option<bool> {
            let key = key_number(&VIRTUAL_KEYS, code)?;
            let state = unsafe { GetAsyncKeyState(i32::from(key)) };
            Some(state < 0)
        }
    }

    pub fn keyboard() -> Box<dyn Keyboard> {
        Box::new(AsyncKeyState)
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use crossterm::event::KeyCode;

    use super::{Keyboard, key_number};

    const HID_SYSTEM_STATE: i32 = 1;

    const MAC_KEYS: [(KeyCode, u16); 14] = [
        (KeyCode::Up, 0x7E),
        (KeyCode::Down, 0x7D),
        (KeyCode::Left, 0x7B),
        (KeyCode::Right, 0x7C),
        (KeyCode::Enter, 0x24),
        (KeyCode::Esc, 0x35),
        (KeyCode::Tab, 0x30),
        (KeyCode::Backspace, 0x33),
        (KeyCode::Char(' '), 0x31),
        (KeyCode::Home, 0x73),
        (KeyCode::End, 0x77),
        (KeyCode::PageUp, 0x74),
        (KeyCode::PageDown, 0x79),
        (KeyCode::Delete, 0x75),
    ];

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventSourceKeyState(state: i32, key: u16) -> bool;
        fn CGPreflightListenEventAccess() -> bool;
        fn CGRequestListenEventAccess() -> bool;
    }

    struct EventSource;

    impl Keyboard for EventSource {
        fn is_down(&self, code: KeyCode) -> Option<bool> {
            let key = key_number(&MAC_KEYS, code)?;
            Some(unsafe { CGEventSourceKeyState(HID_SYSTEM_STATE, key) })
        }

        fn ask(&self) {
            unsafe {
                if !CGPreflightListenEventAccess() {
                    CGRequestListenEventAccess();
                }
            }
        }
    }

    pub fn keyboard() -> Box<dyn Keyboard> {
        Box::new(EventSource)
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod platform {
    use super::{Keyboard, Numb};

    pub fn keyboard() -> Box<dyn Keyboard> {
        Box::new(Numb)
    }
}
