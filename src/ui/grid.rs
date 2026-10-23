use ratatui::{buffer::Buffer, layout::Rect, style::Style};

use crate::ui::layout::{FlexItem, shrink};
use crate::ui::table::{ellipsize, visual_width};

const RULE: char = '│';
const RULE_W: u16 = 1;
pub const CELL_PAD: u16 = 1;

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Grid {
    pub x: u16,
    pub widths: Vec<u16>,
    pub bare: bool,
}

impl Grid {
    pub fn fit(x: u16, inner_w: u16, columns: &[FlexItem]) -> Self {
        let rules = RULE_W * columns.len().saturating_sub(1) as u16;
        let room = inner_w.saturating_sub(rules);
        let mut widths = shrink(columns, room);
        let spare = room.saturating_sub(widths.iter().sum());
        if let Some(last) = widths.last_mut() {
            *last += spare;
        }
        Self {
            x,
            widths,
            bare: false,
        }
    }

    pub fn natural_width(columns: &[FlexItem]) -> u16 {
        columns.iter().map(|column| column.basis).sum::<u16>()
            + RULE_W * columns.len().saturating_sub(1) as u16
    }

    pub fn cell(&self, column: usize, y: u16, height: u16) -> Rect {
        let x = self.x + self.widths[..column].iter().sum::<u16>() + RULE_W * column as u16;
        Rect::new(x, y, self.widths[column], height)
    }

    pub fn text_width(&self, column: usize) -> usize {
        inside(self.cell(column, 0, 1)).width as usize
    }

    pub fn rule_xs(&self) -> Vec<u16> {
        if self.bare {
            return Vec::new();
        }
        (1..self.widths.len())
            .map(|column| self.cell(column, 0, 0).x - RULE_W)
            .collect()
    }

    pub fn draw_rules(&self, buf: &mut Buffer, y: u16, height: u16, style: Style) {
        for x in self.rule_xs() {
            for row in y..y + height {
                buf[(x, row)].set_char(RULE).set_style(style);
            }
        }
    }
}

pub const HEADER_ROWS: u16 = 2;

pub fn split_header(body: Rect) -> (Option<u16>, Rect) {
    if body.height <= HEADER_ROWS {
        return (None, body);
    }
    (
        Some(body.y),
        Rect::new(
            body.x,
            body.y + HEADER_ROWS,
            body.width,
            body.height - HEADER_ROWS,
        ),
    )
}

pub struct HeaderStyle {
    pub text: Style,
    pub rule: Style,
}

pub fn draw_header(
    buf: &mut Buffer,
    grid: &Grid,
    frame: Rect,
    y: u16,
    headers: &[&str],
    style: &HeaderStyle,
) {
    for (column, header) in headers.iter().enumerate().take(grid.widths.len()) {
        put(buf, grid.cell(column, y, 1), header, style.text);
    }
    grid.draw_rules(buf, y, 1, style.rule);
    let separator_y = y + 1;
    let fg = style.rule.fg.unwrap_or_default();
    let bg = style.rule.bg.unwrap_or_default();
    crate::ui::table::draw_box_separator(
        buf,
        frame.x,
        separator_y,
        frame.width,
        &grid.rule_xs(),
        fg,
        bg,
    );
}

pub fn inside(cell: Rect) -> Rect {
    let pad = CELL_PAD.min(cell.width.saturating_sub(1) / 2);
    Rect::new(cell.x + pad, cell.y, cell.width - pad * 2, cell.height)
}

pub fn put(buf: &mut Buffer, cell: Rect, text: &str, style: Style) {
    if cell.width == 0 || cell.height == 0 {
        return;
    }
    let width = cell.width as usize;
    buf.set_stringn(cell.x, cell.y, " ".repeat(width), width, style);
    let text_area = inside(cell);
    let room = text_area.width as usize;
    buf.set_stringn(text_area.x, cell.y, ellipsize(text, room), room, style);
}

pub fn put_money(buf: &mut Buffer, cell: Rect, text: &str, style: Style) {
    if cell.width == 0 || cell.height == 0 {
        return;
    }
    let width = cell.width as usize;
    buf.set_stringn(cell.x, cell.y, " ".repeat(width), width, style);
    let text_area = inside(cell);
    let room = text_area.width as usize;
    let shown = visual_width(text).min(room);
    let x = text_area.right().saturating_sub(shown as u16);
    buf.set_stringn(x, cell.y, ellipsize(text, room), room, style);
}

pub fn text_column(header: &str, cells: impl Iterator<Item = usize>, min: u16) -> FlexItem {
    let widest = cells.max().unwrap_or(0).max(visual_width(header)) as u16;
    FlexItem::new(widest + CELL_PAD * 2, min)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_that_fit_sit_side_by_side_with_one_rule_between() {
        let grid = Grid::fit(1, 20, &[FlexItem::fixed(4), FlexItem::fixed(5)]);
        assert_eq!(grid.cell(0, 0, 1).x, 1);
        assert_eq!(grid.cell(1, 0, 1).x, 6);
        assert_eq!(grid.rule_xs(), vec![5]);
        assert_eq!(grid.widths, vec![4, 15], "the last column takes the slack");
    }

    #[test]
    fn a_cell_is_left_aligned_one_space_inside_its_rules() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 8, 1));
        put(&mut buf, Rect::new(0, 0, 8, 1), "Neo", Style::default());
        let row: String = (0..8).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert_eq!(row, " Neo    ");
    }

    #[test]
    fn money_sits_flush_right_one_space_inside_its_rules() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 8, 1));
        put_money(&mut buf, Rect::new(0, 0, 8, 1), "$120", Style::default());
        let row: String = (0..8).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert_eq!(row, "   $120 ");
    }

    #[test]
    fn a_text_column_is_as_wide_as_its_text_and_both_pads() {
        let column = text_column("Name", [3, 7].into_iter(), 3);
        assert_eq!(column.basis, 7 + CELL_PAD * 2);
    }

    #[test]
    fn a_narrow_grid_shrinks_its_columns_instead_of_hiding_one() {
        let columns = [
            FlexItem::new(14, 4),
            FlexItem::new(8, 4),
            FlexItem::new(30, 6),
        ];
        let grid = Grid::fit(0, 24, &columns);
        assert_eq!(grid.widths.len(), 3);
        assert_eq!(grid.widths.iter().sum::<u16>() + 2, 24);
    }
}
