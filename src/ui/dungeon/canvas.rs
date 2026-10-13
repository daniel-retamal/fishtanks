use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};
use unicode_width::UnicodeWidthChar;

use crate::dungeon::bestiary::SpriteRow;
use crate::fishes::fish::{Direction, Fish};
use crate::ui::circuit_overlay::{
    EAST, NORTH, OFF_VIEW_HORIZONTAL, OFF_VIEW_VERTICAL, SOUTH, WEST, wire_char,
};
use crate::ui::{draw_fish_centred, fish_rows, put};

use super::battle::{BACKGROUND, style};

pub const BLANK: char = ' ';

#[derive(Clone, Default)]
pub struct Canvas {
    pub rows: Vec<Vec<(char, Style)>>,
}

impl Canvas {
    pub fn width(&self) -> u16 {
        self.rows.iter().map(|row| row.len()).max().unwrap_or(0) as u16
    }

    pub fn height(&self) -> u16 {
        self.rows.len() as u16
    }

    fn reach(&mut self, x: usize, y: usize) {
        while self.rows.len() <= y {
            self.rows.push(Vec::new());
        }
        let row = &mut self.rows[y];
        while row.len() <= x {
            row.push((BLANK, style(BACKGROUND)));
        }
    }

    pub fn set(&mut self, x: usize, y: usize, glyph: char, ink: Style) {
        self.reach(x, y);
        self.rows[y][x] = (glyph, ink);
    }

    pub fn text(&mut self, x: usize, y: usize, text: &str, ink: Style) -> usize {
        let mut col = x;
        for glyph in text.chars() {
            if glyph != BLANK {
                self.set(col, y, glyph, ink);
            } else {
                self.reach(col, y);
            }
            col += UnicodeWidthChar::width(glyph).unwrap_or(1);
        }
        col - x
    }

    pub fn sprite(&mut self, x: usize, y: usize, rows: &[SpriteRow]) {
        for (dy, row) in rows.iter().enumerate() {
            for (index, glyph) in row.text.chars().enumerate() {
                if glyph == BLANK {
                    continue;
                }
                self.set(x + index, y + dy, glyph, style(row.colour_at(index)));
            }
        }
        if rows.is_empty() {
            return;
        }
        self.reach(x, y + rows.len() - 1);
    }

    pub fn cells(&mut self, x: usize, y: usize, rows: &[Vec<(char, Color)>]) {
        for (dy, row) in rows.iter().enumerate() {
            for (dx, &(glyph, colour)) in row.iter().enumerate() {
                if glyph == BLANK || glyph == crate::sprite::TRANSPARENT {
                    continue;
                }
                self.set(x + dx, y + dy, glyph, style(colour));
            }
        }
    }

    pub fn blit(&mut self, x: usize, y: usize, other: &Canvas) {
        for (dy, row) in other.rows.iter().enumerate() {
            for (dx, &(glyph, ink)) in row.iter().enumerate() {
                if glyph == BLANK {
                    self.reach(x + dx, y + dy);
                    continue;
                }
                self.set(x + dx, y + dy, glyph, ink);
            }
        }
    }

    pub fn fish(fish: &Fish) -> Canvas {
        let mut pose = fish.portrait();
        pose.facing = Direction::Right;
        let width = (pose.display_width as u16).max(1);
        let height = fish_rows(&pose).max(1);
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        draw_fish_centred(&mut buf, &pose, area, BACKGROUND);
        let mut canvas = Canvas::default();
        for y in 0..height {
            for x in 0..width {
                let cell = &buf[(x, y)];
                let glyph = cell.symbol().chars().next().unwrap_or(BLANK);
                match glyph {
                    BLANK => canvas.reach(x as usize, y as usize),
                    _ => canvas.set(
                        x as usize,
                        y as usize,
                        glyph,
                        Style::default().fg(cell.fg).bg(BACKGROUND),
                    ),
                }
            }
        }
        canvas
    }

    pub fn draw(&self, buf: &mut Buffer, x: i32, y: i32, clip: Rect) {
        for (dy, row) in self.rows.iter().enumerate() {
            for (dx, &(glyph, ink)) in row.iter().enumerate() {
                if glyph == BLANK {
                    continue;
                }
                put(
                    buf,
                    x + dx as i32,
                    y + dy as i32,
                    &glyph.to_string(),
                    ink,
                    clip,
                );
            }
        }
    }

    pub fn row_spans(&self, row: usize) -> Vec<(String, Style)> {
        let Some(cells) = self.rows.get(row) else {
            return Vec::new();
        };
        cells
            .iter()
            .map(|&(glyph, ink)| (glyph.to_string(), ink))
            .collect()
    }

    fn drawn(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.rows.iter().enumerate().flat_map(|(y, row)| {
            row.iter()
                .enumerate()
                .filter(|(_, (glyph, _))| *glyph != BLANK)
                .map(move |(x, _)| (x, y))
        })
    }

    pub fn trimmed(&self) -> Canvas {
        let none: Option<(usize, usize, usize, usize)> = None;
        let Some((left, top, right, bottom)) = self.drawn().fold(none, |bounds, (x, y)| {
            Some(match bounds {
                None => (x, y, x, y),
                Some((l, t, r, b)) => (l.min(x), t.min(y), r.max(x), b.max(y)),
            })
        }) else {
            return Canvas::default();
        };
        let mut canvas = Canvas::default();
        for y in top..=bottom {
            for x in left..=right {
                let cell = self.rows[y].get(x).copied();
                match cell {
                    Some((glyph, ink)) if glyph != BLANK => {
                        canvas.set(x - left, y - top, glyph, ink)
                    }
                    _ => canvas.reach(x - left, y - top),
                }
            }
        }
        canvas
    }

    pub fn centred_row(&mut self, y: usize, width: usize, spans: &[(String, Style)]) {
        let used: usize = spans
            .iter()
            .map(|(text, _)| crate::ui::table::visual_width(text))
            .sum();
        let mut x = width.saturating_sub(used) / 2;
        for (text, ink) in spans {
            x += self.text(x, y, text, *ink);
        }
    }

    pub fn draw_centred(&self, buf: &mut Buffer, area: Rect) {
        let x = area.x + area.width.saturating_sub(self.width()) / 2;
        let y = area.y + area.height.saturating_sub(self.height()) / 2;
        self.draw(buf, x as i32, y as i32, area);
    }
}

#[derive(Clone, Default)]
pub struct Picture {
    variants: Vec<Canvas>,
}

impl Picture {
    pub fn of(variants: Vec<Canvas>) -> Picture {
        Picture {
            variants: variants
                .iter()
                .map(Canvas::trimmed)
                .filter(|canvas| canvas.height() > 0)
                .collect(),
        }
    }

    pub fn fit(&self, width: u16, height: u16) -> Option<&Canvas> {
        self.variants
            .iter()
            .find(|canvas| canvas.width() <= width && canvas.height() <= height)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cuts {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

fn arms(glyph: char) -> u8 {
    match glyph {
        '─' => WEST | EAST,
        '│' => NORTH | SOUTH,
        '┌' => SOUTH | EAST,
        '┐' => SOUTH | WEST,
        '└' => NORTH | EAST,
        '┘' => NORTH | WEST,
        '├' => NORTH | SOUTH | EAST,
        '┤' => NORTH | SOUTH | WEST,
        '┬' => SOUTH | WEST | EAST,
        '┴' => NORTH | WEST | EAST,
        '┼' => NORTH | SOUTH | WEST | EAST,
        _ => 0,
    }
}

fn sealed(glyph: char, leaving: u8) -> Option<char> {
    let mask = arms(glyph);
    if mask & leaving == 0 {
        return None;
    }
    let left = mask & !leaving;
    let vertical = NORTH | SOUTH;
    let horizontal = WEST | EAST;
    if mask & horizontal == 0 && leaving & vertical != 0 {
        return Some(OFF_VIEW_VERTICAL);
    }
    if mask & vertical == 0 && leaving & horizontal != 0 {
        return Some(OFF_VIEW_HORIZONTAL);
    }
    wire_char(left).or(Some(BLANK))
}

fn seal_cell(buf: &mut Buffer, x: u16, y: u16, leaving: u8) {
    let Some(glyph) = buf[(x, y)].symbol().chars().next() else {
        return;
    };
    if let Some(open) = sealed(glyph, leaving) {
        buf[(x, y)].set_char(open);
    }
}

pub fn seal(buf: &mut Buffer, area: Rect, cuts: Cuts) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    for x in area.left()..area.right() {
        if cuts.top {
            seal_cell(buf, x, area.top(), NORTH);
        }
        if cuts.bottom {
            seal_cell(buf, x, area.bottom() - 1, SOUTH);
        }
    }
    for y in area.top()..area.bottom() {
        if cuts.left {
            seal_cell(buf, area.left(), y, WEST);
        }
        if cuts.right {
            seal_cell(buf, area.right() - 1, y, EAST);
        }
    }
}
