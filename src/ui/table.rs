use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
};

use unicode_width::UnicodeWidthChar;

use crate::ui::modal;

pub const NOTHING: &str = "-";
pub const INFINITY: &str = "∞";
const TITLE_INSET: u16 = 2;
const TITLE_MARGIN: u16 = 4;

pub struct OverlayLayout {
    pub ox: u16,
    pub oy: u16,
    pub w: u16,
    pub h: u16,
}

impl OverlayLayout {
    pub fn centered(area: Rect, w: u16, h: u16) -> Option<Self> {
        if area.width < w || area.height < h {
            return None;
        }
        Some(Self {
            ox: area.x + (area.width - w) / 2,
            oy: area.y + (area.height - h) / 2,
            w,
            h,
        })
    }

    fn rect(&self) -> Rect {
        Rect::new(self.ox, self.oy, self.w, self.h)
    }

    pub fn clear_bg(&self, buf: &mut Buffer, bg: Color) {
        modal::clear(buf, self.rect(), bg);
    }

    pub fn draw_border(&self, buf: &mut Buffer, title: &str, fg: Color, bg: Color) {
        draw_box_border(buf, self.rect(), title, fg, bg);
    }

    pub fn draw_title(&self, buf: &mut Buffer, title: &str, fg: Color, bg: Color) {
        draw_box_title(buf, self.rect(), title, fg, bg);
    }

    pub fn inner_x(&self) -> u16 {
        self.ox + 1
    }

    pub fn inner_y(&self) -> u16 {
        self.oy + 1
    }

    pub fn inner_w(&self) -> u16 {
        self.w.saturating_sub(2)
    }
}

pub fn visual_width(s: &str) -> usize {
    s.chars()
        .map(|c| UnicodeWidthChar::width(c).unwrap_or(1))
        .sum()
}

pub fn pad_right(s: &str, width: usize) -> String {
    let vw = visual_width(s);
    if vw >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - vw))
    }
}

const ELLIPSIS: char = '…';

pub fn ellipsize(s: &str, max_width: usize) -> String {
    if visual_width(s) <= max_width {
        return s.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    let mut clipped = truncate_str(s, max_width - 1);
    clipped.push(ELLIPSIS);
    clipped
}

pub fn truncate_str(s: &str, max_width: usize) -> String {
    let mut out = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(1);
        if w + cw > max_width {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out
}

pub fn draw_box_border(buf: &mut Buffer, area: Rect, title: &str, fg: Color, bg: Color) {
    let (ox, oy, w, h) = (area.x, area.y, area.width, area.height);
    if w < 2 || h < 2 {
        return;
    }
    let s = Style::default().fg(fg).bg(bg);
    let right = ox + w - 1;
    let bottom = oy + h - 1;

    buf[(ox, oy)].set_char('┌').set_style(s);
    buf[(right, oy)].set_char('┐').set_style(s);
    buf[(ox, bottom)].set_char('└').set_style(s);
    buf[(right, bottom)].set_char('┘').set_style(s);

    for dx in 1..w - 1 {
        buf[(ox + dx, oy)].set_char('─').set_style(s);
        buf[(ox + dx, bottom)].set_char('─').set_style(s);
    }
    for dy in 1..h - 1 {
        buf[(ox, oy + dy)].set_char('│').set_style(s);
        buf[(right, oy + dy)].set_char('│').set_style(s);
    }

    draw_box_title(buf, area, title, fg, bg);
}

pub fn draw_box_title(buf: &mut Buffer, area: Rect, title: &str, fg: Color, bg: Color) {
    let style = Style::default().fg(fg).add_modifier(Modifier::BOLD).bg(bg);
    draw_title_with(buf, area, title, style);
}

pub fn draw_title_with(buf: &mut Buffer, area: Rect, title: &str, style: Style) {
    let room = area.width.saturating_sub(TITLE_MARGIN) as usize;
    if title.is_empty() || room == 0 {
        return;
    }
    buf.set_string(area.x + TITLE_INSET, area.y, ellipsize(title, room), style);
}

pub fn wrap_words(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_w = 0usize;
    for word in text.split_whitespace() {
        for piece in break_word(word, width) {
            let piece_w = visual_width(&piece);
            if current_w > 0 && current_w + 1 + piece_w > width {
                lines.push(std::mem::take(&mut current));
                current_w = 0;
            }
            if current_w > 0 {
                current.push(' ');
                current_w += 1;
            }
            current.push_str(&piece);
            current_w += piece_w;
        }
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }
    lines
}

fn break_word(word: &str, width: usize) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut piece = String::new();
    let mut piece_w = 0;
    for ch in word.chars() {
        let ch_w = UnicodeWidthChar::width(ch).unwrap_or(1);
        if piece_w + ch_w > width && !piece.is_empty() {
            pieces.push(std::mem::take(&mut piece));
            piece_w = 0;
        }
        piece.push(ch);
        piece_w += ch_w;
    }
    if !piece.is_empty() {
        pieces.push(piece);
    }
    pieces
}

pub fn scrolled_to_fit(s: &str, room: usize) -> &str {
    let mut excess = visual_width(s).saturating_sub(room);
    let mut start = 0;
    for (index, ch) in s.char_indices() {
        if excess == 0 {
            start = index;
            break;
        }
        excess = excess.saturating_sub(UnicodeWidthChar::width(ch).unwrap_or(1));
        start = index + ch.len_utf8();
    }
    &s[start..]
}

pub fn draw_box_separator(
    buf: &mut Buffer,
    ox: u16,
    sep_y: u16,
    w: u16,
    col_xs: &[u16],
    fg: Color,
    bg: Color,
) {
    let style = Style::default().fg(fg).bg(bg);

    buf[(ox, sep_y)].set_char('├').set_style(style);
    buf[(ox + w - 1, sep_y)].set_char('┤').set_style(style);
    for dx in 1..w - 1 {
        buf[(ox + dx, sep_y)].set_char('─').set_style(style);
    }
    for &cx in col_xs {
        if cx > ox && cx < ox + w - 1 {
            buf[(cx, sep_y)].set_char('┼').set_style(style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ellipsis_marks_exactly_where_text_was_cut() {
        assert_eq!(ellipsize("Inverter Coil", 8), "Inverte…");
        assert_eq!(ellipsize("clk", 8), "clk", "text that fits is untouched");
        assert_eq!(ellipsize("clk", 0), "");
    }

    #[test]
    fn a_scrolled_field_keeps_the_end_of_its_text_in_view() {
        assert_eq!(scrolled_to_fit("/clone {cheapest}", 8), "heapest}");
        assert_eq!(scrolled_to_fit("short", 8), "short");
    }
}
