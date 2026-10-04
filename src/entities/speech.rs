pub const SPEECH_BUBBLE_TTL: f32 = 6.0;

#[derive(Clone)]
pub struct SpeechBubble {
    pub text: String,
    pub ttl: f32,
}

impl SpeechBubble {
    pub fn new(text: String) -> Self {
        Self {
            text,
            ttl: SPEECH_BUBBLE_TTL,
        }
    }

    pub fn fade(bubble: &mut Option<SpeechBubble>, dt: f32) {
        let Some(held) = bubble else { return };
        held.ttl -= dt;
        if held.ttl <= 0.0 {
            *bubble = None;
        }
    }
}

const LONE_LINE_EDGES: (char, char) = ('<', '>');
const FIRST_LINE_EDGES: (char, char) = ('/', '\\');
const MIDDLE_LINE_EDGES: (char, char) = ('|', '|');
const LAST_LINE_EDGES: (char, char) = ('\\', '/');

const BUBBLE_TEXT_INDENT: usize = 2;

pub fn is_bubble_text(bubble: &[String], row: usize, col: usize) -> bool {
    let width = bubble.first().map_or(0, |top| top.chars().count());
    let body = 1..bubble.len().saturating_sub(1);
    body.contains(&row) && col >= BUBBLE_TEXT_INDENT && col + BUBBLE_TEXT_INDENT < width
}

pub fn build_speech_bubble(text: &str) -> Vec<String> {
    build_bubble(&[text])
}

pub fn build_bubble<S: AsRef<str>>(lines: &[S]) -> Vec<String> {
    let inner = lines
        .iter()
        .map(|line| line.as_ref().chars().count())
        .max()
        .unwrap_or(0);
    let top: String = std::iter::repeat_n('_', inner + 2).collect();
    let bot: String = std::iter::repeat_n('-', inner + 2).collect();
    let last = lines.len().saturating_sub(1);
    let framed = lines.iter().enumerate().map(|(index, line)| {
        let (open, close) = match index {
            _ if last == 0 => LONE_LINE_EDGES,
            0 => FIRST_LINE_EDGES,
            _ if index == last => LAST_LINE_EDGES,
            _ => MIDDLE_LINE_EDGES,
        };
        format!("{open} {:<inner$} {close}", line.as_ref())
    });
    std::iter::once(format!(" {top} "))
        .chain(framed)
        .chain(std::iter::once(format!(" {bot} ")))
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Above,
    Below,
}

impl Side {
    pub fn of(height: i32, room_above: i32, room_below: i32) -> Self {
        if height <= room_above {
            return Side::Above;
        }
        if height <= room_below {
            return Side::Below;
        }
        if room_below > room_above {
            return Side::Below;
        }
        Side::Above
    }
}

pub fn shift_into(left: i32, width: i32, room: i32) -> i32 {
    left.min(room - width).max(0)
}

pub const TAIL_LENGTH: usize = 2;
const TAIL_OVERHANG: i32 = 2;
const TAIL_FALLING: char = '\\';
const TAIL_RISING: char = '/';

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Tail {
    pub anchor: (i32, i32),
    pub facing_left: bool,
    pub side: Side,
}

impl Tail {
    pub fn off_eye(eye: (i32, i32), facing_left: bool, side: Side) -> Self {
        let beside = Tail {
            anchor: eye,
            facing_left,
            side,
        };
        let (dx, dy) = beside.step();
        Tail {
            anchor: (eye.0 + dx, eye.1 + dy),
            ..beside
        }
    }

    fn step(&self) -> (i32, i32) {
        let dx = if self.facing_left { -1 } else { 1 };
        let dy = match self.side {
            Side::Above => -1,
            Side::Below => 1,
        };
        (dx, dy)
    }

    pub fn glyph(&self) -> char {
        let (dx, dy) = self.step();
        if dx == dy { TAIL_FALLING } else { TAIL_RISING }
    }

    pub fn cells(&self) -> [(i32, i32); TAIL_LENGTH] {
        let (dx, dy) = self.step();
        let (x, y) = self.anchor;
        std::array::from_fn(|index| (x + dx * index as i32, y + dy * index as i32))
    }

    fn end(&self) -> (i32, i32) {
        self.cells()[TAIL_LENGTH - 1]
    }

    pub fn bubble_left(&self, width: i32) -> i32 {
        let (x, _) = self.end();
        if self.facing_left {
            return x + TAIL_OVERHANG + 1 - width;
        }
        x - TAIL_OVERHANG
    }

    pub fn bubble_top(&self, height: i32) -> i32 {
        let (_, y) = self.end();
        match self.side {
            Side::Above => y - height,
            Side::Below => y + 1,
        }
    }

    pub fn room(&self, top: i32, bottom: i32) -> i32 {
        let (_, y) = self.end();
        match self.side {
            Side::Above => y - top,
            Side::Below => bottom - y - 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bubble_that_fits_above_stays_above() {
        assert_eq!(Side::of(3, 3, 10), Side::Above);
    }

    #[test]
    fn a_bubble_with_no_room_above_flips_below() {
        assert_eq!(Side::of(3, 2, 3), Side::Below);
    }

    #[test]
    fn a_bubble_that_fits_nowhere_takes_the_roomier_side() {
        assert_eq!(Side::of(9, 2, 5), Side::Below);
        assert_eq!(Side::of(9, 5, 2), Side::Above);
        assert_eq!(Side::of(9, 4, 4), Side::Above, "a tie keeps the usual side");
    }

    #[test]
    fn a_bubble_slides_back_inside_either_edge() {
        assert_eq!(shift_into(-3, 6, 20), 0);
        assert_eq!(shift_into(17, 6, 20), 14);
        assert_eq!(shift_into(5, 6, 20), 5, "a bubble that fits is not moved");
    }

    #[test]
    fn a_bubble_wider_than_the_room_starts_at_its_left_edge() {
        assert_eq!(shift_into(4, 30, 20), 0);
    }

    fn tail(facing_left: bool, side: Side) -> Tail {
        Tail {
            anchor: (10, 10),
            facing_left,
            side,
        }
    }

    #[test]
    fn a_tail_has_the_two_diagonals_of_cowsay() {
        assert_eq!(TAIL_LENGTH, 2);
        assert_eq!(tail(true, Side::Above).cells(), [(10, 10), (9, 9)]);
        assert_eq!(tail(false, Side::Above).cells(), [(10, 10), (11, 9)]);
        assert_eq!(tail(true, Side::Below).cells(), [(10, 10), (9, 11)]);
        assert_eq!(tail(false, Side::Below).cells(), [(10, 10), (11, 11)]);
    }

    #[test]
    fn a_tail_leans_toward_the_bubble_in_front_of_the_speaker() {
        assert_eq!(tail(true, Side::Above).glyph(), '\\');
        assert_eq!(tail(false, Side::Above).glyph(), '/');
        assert_eq!(tail(true, Side::Below).glyph(), '/');
        assert_eq!(tail(false, Side::Below).glyph(), '\\');
    }

    #[test]
    fn a_bubble_sits_on_its_tail_and_reaches_forward() {
        let left = tail(true, Side::Above);
        assert_eq!(left.bubble_top(3), 6);
        assert_eq!(left.bubble_left(7) + 7 - 1, 9 + TAIL_OVERHANG);
        let right = tail(false, Side::Above);
        assert_eq!(right.bubble_left(7), 11 - TAIL_OVERHANG);
        assert_eq!(tail(false, Side::Below).bubble_top(3), 12);
    }

    #[test]
    fn a_tail_off_an_eye_ends_on_the_eye_diagonal_in_front() {
        let eye = (10, 10);
        assert_eq!(Tail::off_eye(eye, true, Side::Above).anchor, (9, 9));
        assert_eq!(Tail::off_eye(eye, false, Side::Above).anchor, (11, 9));
        assert_eq!(Tail::off_eye(eye, false, Side::Below).anchor, (11, 11));
    }

    #[test]
    fn a_tail_measures_the_room_past_its_end() {
        assert_eq!(tail(true, Side::Above).room(0, 20), 9);
        assert_eq!(tail(true, Side::Below).room(0, 20), 8);
    }

    #[test]
    fn a_bubble_is_three_rows_of_the_same_width() {
        let bubble = build_speech_bubble("glup glup");
        assert_eq!(bubble.len(), 3);
        let widths: Vec<usize> = bubble.iter().map(|line| line.chars().count()).collect();
        assert_eq!(widths, vec![13, 13, 13]);
    }

    #[test]
    fn a_bubble_frames_the_text_it_was_given() {
        assert_eq!(build_speech_bubble("hi")[1], "< hi >");
    }

    #[test]
    fn a_bubble_of_several_lines_is_framed_the_way_cowsay_frames_one() {
        assert_eq!(
            build_bubble(&["ab", "c", "de"]),
            vec![
                " ____ ".to_string(),
                "/ ab \\".to_string(),
                "| c  |".to_string(),
                "\\ de /".to_string(),
                " ---- ".to_string(),
            ]
        );
    }

    #[test]
    fn a_bubble_keeps_the_spaces_of_the_lines_it_frames() {
        assert_eq!(build_bubble(&["  x ", "    "])[1], "/   x  \\");
    }

    #[test]
    fn a_bubble_fades_after_its_time_is_up() {
        let mut bubble = Some(SpeechBubble::new("glup".to_string()));
        SpeechBubble::fade(&mut bubble, SPEECH_BUBBLE_TTL / 2.0);
        assert!(bubble.is_some(), "half its life is still on screen");
        SpeechBubble::fade(&mut bubble, SPEECH_BUBBLE_TTL);
        assert!(bubble.is_none());
    }

    #[test]
    fn fading_nothing_is_harmless() {
        let mut bubble = None;
        SpeechBubble::fade(&mut bubble, 1.0);
        assert!(bubble.is_none());
    }
}
