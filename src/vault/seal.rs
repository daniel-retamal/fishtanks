use serde::{Deserialize, Serialize};

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const CARRIAGE_RETURN: u8 = b'\r';
const LINE_BREAK: char = '\n';
const STAMP: &str = "// fishtank seal ";
const HEX_DIGITS: usize = 16;
const HEX_RADIX: u32 = 16;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Seal(u64);

impl Default for Seal {
    fn default() -> Self {
        Self(FNV_OFFSET_BASIS)
    }
}

impl Seal {
    pub fn of(bytes: &[u8]) -> Self {
        let mut seal = Self::default();
        seal.feed(bytes);
        seal
    }

    pub fn value(self) -> u64 {
        self.0
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        for &byte in bytes.iter().filter(|&&byte| byte != CARRIAGE_RETURN) {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(FNV_PRIME);
        }
    }

    pub fn stamp(body: &str) -> String {
        format!(
            "{STAMP}{:0width$x}{LINE_BREAK}{body}",
            Self::of(body.as_bytes()).0,
            width = HEX_DIGITS
        )
    }

    pub fn is_intact(text: &str) -> bool {
        let Some(stamped) = text.strip_prefix(STAMP) else {
            return false;
        };
        let Some((hex, body)) = stamped.split_once(LINE_BREAK) else {
            return false;
        };
        u64::from_str_radix(hex.trim_end(), HEX_RADIX)
            .is_ok_and(|seal| Self(seal) == Self::of(body.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = "(\n    cash: 50,\n)";

    #[test]
    fn a_stamped_text_is_intact() {
        assert!(Seal::is_intact(&Seal::stamp(BODY)));
    }

    #[test]
    fn one_changed_byte_breaks_the_seal() {
        let forged = Seal::stamp(BODY).replace("50", "59");
        assert!(!Seal::is_intact(&forged));
    }

    #[test]
    fn a_text_without_a_stamp_is_not_intact() {
        assert!(!Seal::is_intact(BODY));
    }

    #[test]
    fn line_endings_do_not_break_the_seal() {
        let windows = Seal::stamp(BODY).replace('\n', "\r\n");
        assert!(Seal::is_intact(&windows));
    }

    #[test]
    fn the_seal_is_the_same_on_every_machine_and_every_rust() {
        assert_eq!(Seal::of(b"fishtank"), Seal(0xc5a0_4c4a_1a95_ffc7));
    }
}
