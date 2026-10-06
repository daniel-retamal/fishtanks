pub mod catch_overlay;
pub mod cheat_popup;
pub mod circuit_overlay;
pub mod command_bar;
pub mod console;
pub mod consume_picker;
pub mod fields;
pub mod fishing_overlay;
pub mod fishtanks_overlay;
pub mod foundry_overlay;
pub mod grid;
pub mod hint_bar;
pub mod hints;
pub mod index_overlay;
pub mod index_query;
pub mod input_action;
pub mod inventory_overlay;
pub mod layout;
pub mod ledger_overlay;
pub mod line_editor;
pub mod modal;
pub mod notice;
pub mod panels;
pub mod shop_overlay;
pub mod show_overlay;
pub mod table;
pub mod tank_view;
pub mod text_input;
pub mod wiring_panel;

use ratatui::{
    buffer::{Buffer, Cell},
    layout::Rect,
    style::Color,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::fishes::fish::{Fish, LineSprite};
use crate::fishes::unfish::{UnfishKind, is_multi_row};
use crate::sprite::TRANSPARENT;

const UNCOVERED: &str = " ";
const FISH_ART_PAD_ROWS: u16 = 2;

pub fn overdraw(buf: &mut Buffer, x: u16, y: u16) -> &mut Cell {
    if x > buf.area.left() && buf[(x - 1, y)].symbol().width() > 1 {
        buf[(x - 1, y)].set_symbol(UNCOVERED);
    }
    &mut buf[(x, y)]
}

pub fn render_fish_segs(
    buf: &mut Buffer,
    segs: &[(char, Color)],
    x: u16,
    y: u16,
    max_width: u16,
    bg: Color,
) {
    let mut col = 0u16;
    for (ch, color) in segs {
        let cw = UnicodeWidthChar::width(*ch).unwrap_or(1) as u16;
        if col + cw > max_width {
            break;
        }
        buf[(x + col, y)].set_char(*ch).set_fg(*color).set_bg(bg);
        col += cw;
    }
}

pub fn render_fish_sprite(
    buf: &mut Buffer,
    sprite: &LineSprite,
    x: u16,
    body_y: u16,
    clip: Rect,
    bg: Color,
) {
    let max_width = clip.right().saturating_sub(x);
    for (row_idx, row) in sprite.rows.iter().enumerate() {
        let row_y = body_y as i32 + row_idx as i32 - sprite.body_row as i32;
        if row_y < clip.y as i32 || row_y >= clip.bottom() as i32 {
            continue;
        }
        let row_y = row_y as u16;
        let mut col = 0u16;
        for &(ch, color) in row {
            let cw = UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
            if col + cw > max_width {
                break;
            }
            if ch != TRANSPARENT {
                buf[(x + col, row_y)].set_char(ch).set_fg(color).set_bg(bg);
            }
            col += cw;
        }
    }
}

pub fn fish_art_height(fish: &Fish, floor: u16) -> u16 {
    match fish.unfish_kind().and_then(UnfishKind::grid) {
        Some(grid) => grid.height,
        None => floor.max(fish.line_sprite().rows.len() as u16 + FISH_ART_PAD_ROWS),
    }
}

pub fn draw_fish_centred(buf: &mut Buffer, fish: &Fish, area: Rect, bg: Color) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let art_w = fish.display_width as u16;
    let art_x = area.x + area.width.saturating_sub(art_w) / 2;
    if let Some(ref us) = fish.unfish_state
        && is_multi_row(us.kind)
    {
        tank_view::render_multi_row_unfish_at(fish, art_x as i32, area.y as i32, area, buf);
        return;
    }
    let sprite = fish.line_sprite();
    let rows = sprite.rows.len() as u16;
    let top = area.y + area.height.saturating_sub(rows) / 2;
    let body_y = (top + sprite.body_row as u16).min(area.bottom() - 1);
    render_fish_sprite(buf, &sprite, art_x, body_y, area, bg);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fishes::species::FishSpecies;
    use crate::tank::{Tank, TankKind};
    use ratatui::style::Style;

    #[test]
    fn a_cell_drawn_over_a_wide_glyphs_right_half_blanks_the_glyph() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 2));
        buf.set_string(0, 0, "彡", Style::default());
        buf.set_string(0, 1, "彡", Style::default());
        overdraw(&mut buf, 1, 0).set_char('│');
        overdraw(&mut buf, 2, 1).set_char('│');
        assert_eq!(
            buf[(0, 0)].symbol(),
            UNCOVERED,
            "a terminal would otherwise paint the glyph's right half over what was drawn"
        );
        assert_eq!(
            buf[(0, 1)].symbol(),
            "彡",
            "a cell beside the glyph's whole width leaves it alone"
        );
    }

    #[test]
    fn footed_fish_draws_feet_one_row_below_the_body() {
        let mut rng = rand::rng();
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        tank.spawn_fish(FishSpecies::Merluza, "Steps".to_string(), &mut rng);
        tank.apply_named_mutation("Steps", "feet");
        assert!(tank.fish[0].feet().is_some());
        let area = Rect::new(0, 0, 40, 6);
        let mut buf = Buffer::empty(area);
        render_fish_sprite(
            &mut buf,
            &tank.fish[0].line_sprite(),
            2,
            2,
            area,
            Color::Reset,
        );
        let is_foot = |s: &str| s == "\"" || s == "^";
        let body_feet = (0..area.width)
            .filter(|&x| is_foot(buf[(x, 2)].symbol()))
            .count();
        let below_feet = (0..area.width)
            .filter(|&x| is_foot(buf[(x, 3)].symbol()))
            .count();
        assert_eq!(body_feet, 0, "the body row never carries feet glyphs");
        assert!(
            below_feet > 0,
            "feet land on the row directly below the body"
        );
    }
}
