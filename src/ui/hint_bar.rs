use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};

use crate::colors::DARK_GRAY;
use crate::fishes::botfish::level_color;
use crate::ui::table::{visual_width, wrap_words};

const EDGE_PAD: usize = 1;
const ACTION_GAP: usize = 3;
const CLOSE_GAP: usize = 2;

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct HintLine {
    pub actions: Vec<String>,
    pub levels: Vec<Option<bool>>,
    pub close: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct Hint {
    text: String,
    level: Option<bool>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct HintBar {
    actions: Vec<Hint>,
    close: String,
    aside: Option<String>,
}

impl HintBar {
    pub fn new(close: &str) -> Self {
        Self {
            actions: Vec::new(),
            close: close.to_string(),
            aside: None,
        }
    }

    pub fn aside(mut self, note: impl Into<String>) -> Self {
        self.aside = Some(note.into());
        self
    }

    pub fn action(self, hint: impl Into<String>) -> Self {
        self.push(hint.into(), None)
    }

    pub fn level(self, hint: impl Into<String>, high: bool) -> Self {
        self.push(hint.into(), Some(high))
    }

    fn push(mut self, text: String, level: Option<bool>) -> Self {
        if !text.is_empty() {
            self.actions.push(Hint { text, level });
        }
        self
    }

    pub fn action_if(self, shown: bool, hint: impl Into<String>) -> Self {
        if !shown {
            return self;
        }
        self.action(hint)
    }

    pub fn counted(self, hint: &str, position: Option<(usize, usize)>) -> Self {
        match position {
            Some((shown, total)) => self.action(format!("{hint} ({shown}/{total})")),
            None => self.action(hint),
        }
    }

    pub fn natural_width(&self) -> u16 {
        let actions: usize = self
            .actions
            .iter()
            .map(|hint| visual_width(&hint.text))
            .sum::<usize>()
            + ACTION_GAP * self.actions.len().saturating_sub(1);
        let gap = if self.actions.is_empty() {
            0
        } else {
            CLOSE_GAP
        };
        let hints = EDGE_PAD * 2 + actions + gap + visual_width(&self.close);
        let aside = self
            .aside
            .as_deref()
            .map_or(0, |note| EDGE_PAD * 2 + visual_width(note));
        hints.max(aside) as u16
    }

    fn aside_lines(&self, room: usize) -> Vec<HintLine> {
        self.aside
            .as_deref()
            .map(|note| wrap_words(note, room))
            .unwrap_or_default()
            .into_iter()
            .map(|piece| HintLine {
                close: Some(piece),
                ..HintLine::default()
            })
            .collect()
    }

    pub fn lines(&self, width: u16) -> Vec<HintLine> {
        let room = (width as usize).saturating_sub(EDGE_PAD * 2).max(1);
        let mut lines: Vec<HintLine> = self.aside_lines(room);
        let mut current = HintLine::default();
        let mut used = 0;
        let pieces = self.actions.iter().flat_map(|hint| {
            wrap_words(&hint.text, room)
                .into_iter()
                .map(|piece| (piece, hint.level))
        });
        for (piece, level) in pieces {
            let piece_w = visual_width(&piece);
            if !current.actions.is_empty() && used + ACTION_GAP + piece_w > room {
                lines.push(std::mem::take(&mut current));
                used = 0;
            }
            used = if current.actions.is_empty() {
                piece_w
            } else {
                used + ACTION_GAP + piece_w
            };
            current.actions.push(piece);
            current.levels.push(level);
        }
        let mut close = wrap_words(&self.close, room);
        let last_close = close.pop().unwrap_or_default();
        for piece in close {
            lines.push(std::mem::take(&mut current));
            current.actions.push(piece);
            current.levels.push(None);
            used = room;
        }
        let shares_line =
            current.actions.is_empty() || used + CLOSE_GAP + visual_width(&last_close) <= room;
        if !shares_line {
            lines.push(std::mem::take(&mut current));
        }
        current.close = Some(last_close);
        lines.push(current);
        lines.retain(|line| !line.actions.is_empty() || line.close.is_some());
        lines
    }

    pub fn height(&self, width: u16) -> u16 {
        self.lines(width).len() as u16
    }

    pub fn draw(&self, buf: &mut Buffer, area: Rect, bg: Color) {
        let style = Style::default().fg(DARK_GRAY).bg(bg);
        let left_x = area.x + EDGE_PAD as u16;
        let right_edge = area.right().saturating_sub(EDGE_PAD as u16);
        let lines = self.lines(area.width);
        let asides = self
            .aside_lines((area.width as usize).saturating_sub(EDGE_PAD * 2).max(1))
            .len();
        let unseen = asides.min(lines.len().saturating_sub(area.height as usize));
        for (row, line) in lines.iter().skip(unseen).enumerate() {
            let y = area.y + row as u16;
            if y >= area.bottom() {
                return;
            }
            let joined = line.actions.join(&" ".repeat(ACTION_GAP));
            buf.set_stringn(
                left_x,
                y,
                &joined,
                right_edge.saturating_sub(left_x) as usize,
                style,
            );
            let mut x = left_x;
            for (piece, level) in line.actions.iter().zip(&line.levels) {
                if let Some(high) = level {
                    buf.set_stringn(
                        x,
                        y,
                        piece,
                        right_edge.saturating_sub(x) as usize,
                        style.fg(level_color(*high)),
                    );
                }
                x = x.saturating_add((visual_width(piece) + ACTION_GAP) as u16);
            }
            if let Some(close) = &line.close {
                let close_x = right_edge
                    .saturating_sub(visual_width(close) as u16)
                    .max(left_x);
                buf.set_stringn(
                    close_x,
                    y,
                    close,
                    right_edge.saturating_sub(close_x) as usize,
                    style,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLOSE: &str = "ESC/q close";

    fn circuit() -> HintBar {
        HintBar::new(CLOSE)
            .counted("↑↓ select", Some((3, 13)))
            .action("ENTER wire")
            .action("TAB schematic")
    }

    fn texts(bar: &HintBar, width: u16) -> Vec<String> {
        bar.lines(width)
            .iter()
            .map(|line| {
                let mut text = line.actions.join(" | ");
                if let Some(close) = &line.close {
                    text.push_str(" >> ");
                    text.push_str(close);
                }
                text
            })
            .collect()
    }

    #[test]
    fn a_wide_bar_is_one_line_with_close_on_the_right() {
        let bar = circuit();
        assert_eq!(bar.height(bar.natural_width()), 1);
        assert_eq!(
            texts(&bar, 80),
            vec!["↑↓ select (3/13) | ENTER wire | TAB schematic >> ESC/q close"]
        );
    }

    #[test]
    fn a_narrow_bar_wraps_every_hint_instead_of_dropping_one() {
        let lines = texts(&circuit(), 34);
        assert_eq!(
            lines,
            vec![
                "↑↓ select (3/13) | ENTER wire",
                "TAB schematic >> ESC/q close"
            ]
        );
    }

    #[test]
    fn a_very_narrow_bar_stacks_one_hint_per_line_and_still_closes() {
        let lines = circuit().lines(18);
        assert_eq!(lines.len(), 4);
        assert_eq!(
            lines.last().and_then(|line| line.close.clone()),
            Some(CLOSE.to_string())
        );
    }

    #[test]
    fn a_hint_wider_than_the_bar_wraps_by_word_instead_of_losing_its_end() {
        let bar = HintBar::new("ESC").action("↑↓ scroll (10/18)");
        let lines = texts(&bar, 14);
        assert_eq!(lines, vec!["↑↓ scroll", "(10/18) >> ESC"]);
    }

    #[test]
    fn a_level_hint_wraps_like_any_other_and_keeps_its_level_on_every_piece() {
        let bar = HintBar::new("ESC leave")
            .action("console Pad")
            .level("SPACE fire", true)
            .level("↑ up", false);
        let lines = bar.lines(16);
        let levels: Vec<Option<bool>> = lines.iter().flat_map(|l| l.levels.clone()).collect();
        let pieces: Vec<String> = lines.iter().flat_map(|l| l.actions.clone()).collect();
        assert_eq!(pieces.len(), levels.len(), "every piece carries its level");
        assert_eq!(levels, vec![None, Some(true), Some(false)], "{pieces:?}");
    }

    #[test]
    fn a_high_hint_is_drawn_in_the_level_colour_and_a_low_one_in_hint_grey() {
        let bar = HintBar::new("ESC leave").level("A", true).level("B", false);
        let area = Rect::new(0, 0, 30, 1);
        let mut buf = Buffer::empty(area);
        bar.draw(&mut buf, area, Color::Reset);
        let fg = |needle: &str| {
            (0..area.width)
                .find(|&x| buf[(x, 0)].symbol() == needle)
                .map(|x| buf[(x, 0)].fg)
                .expect("drawn")
        };
        assert_eq!(fg("A"), level_color(true));
        assert_eq!(fg("B"), level_color(false));
        assert_eq!(level_color(false), DARK_GRAY, "low is the hint grey itself");
    }

    #[test]
    fn natural_width_is_one_padded_line() {
        let bar = HintBar::new(CLOSE).action("ENTER save");
        assert_eq!(
            bar.natural_width() as usize,
            EDGE_PAD * 2 + "ENTER save".len() + CLOSE_GAP + CLOSE.len()
        );
    }
}
