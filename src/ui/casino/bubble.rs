use std::collections::HashSet;

use ratatui::{buffer::Buffer, layout::Rect};

use crate::casino::Multiple;
use crate::casino::bubble::{Bubble, Risk};
use crate::casino::state::Table;
use crate::colors::{DARK_GRAY, GOLD, LIGHT_CYAN, PINK, WHITE};
use crate::ui::hints::{HINT_ENTER_BLOW, HINT_PAYTABLE, HINT_RISK};

use super::{
    Room, bet_hints, bold, flash_color, flashing, multiple_color, open_frame, put, result_text,
    stake_parts, status_row, style,
};

const BASIS: (u16, u16) = (74, 24);
const WIDEST_SPACING: u16 = 4;
const NARROWEST_SPACING: u16 = 2;
const LEGEND_FROM_ROWS: u16 = 6;
const LAUNCH: &str = "○";
const PEG: &str = "·";
const SHELL: &str = "‿";
const LIT_SHELL: &str = "▀";

pub fn draw(
    buf: &mut Buffer,
    room: &Room,
    table: &Table,
    risk: Risk,
    bubbles: &[Bubble],
    lit: &[(Risk, usize, f32)],
) {
    let hints = bet_hints(
        table,
        room.teller,
        HINT_ENTER_BLOW,
        &[HINT_RISK, HINT_PAYTABLE],
    );
    let chrome = open_frame(
        buf,
        room.screen,
        "Bubble Up",
        BASIS,
        flash_color(table.flash),
        &hints,
        super::widest_bet(table, HINT_ENTER_BLOW, &[HINT_RISK, HINT_PAYTABLE]),
    );
    let mut parts = stake_parts(table, room.teller);
    parts.push(("   Risk ".to_string(), style(DARK_GRAY)));
    parts.push((
        risk.name().to_string(),
        bold(if risk == Risk::Stupid { GOLD } else { WHITE }),
    ));
    status_row(buf, chrome.status, &parts, room.purse);
    draw_board(buf, chrome.art, table, risk, bubbles, lit);
}

fn draw_board(
    buf: &mut Buffer,
    art: Rect,
    table: &Table,
    risk: Risk,
    bubbles: &[Bubble],
    lit: &[(Risk, usize, f32)],
) {
    let rows = risk.rows();
    let shells = risk.shells();
    let cups = shells.len() as u16;
    let mut spacing = (art.width.saturating_sub(2) / cups).clamp(NARROWEST_SPACING, WIDEST_SPACING);
    spacing -= spacing % 2;
    let label_w = (spacing - 1) as usize;
    let width = spacing * cups;
    let first_x = art.x as i32 + (art.width as i32 - width as i32) / 2 + spacing as i32 / 2;
    let centre = first_x + (rows as i32 * spacing as i32) / 2;
    let fits = shells.iter().all(|m| short(*m).len() <= label_w);
    let legend_rows = if !fits && art.height >= LEGEND_FROM_ROWS {
        draw_legend(buf, art, &shells)
    } else {
        0
    };
    let top = art.y + legend_rows;
    let board = art.bottom().saturating_sub(top + 2).max(1);
    let row_y = |step: usize| -> i32 {
        top as i32 + 2 + ((rows - step.min(rows)) as i32 * (board as i32 - 1)) / rows as i32
    };
    for (k, m) in shells.iter().enumerate() {
        let x = first_x + k as i32 * spacing as i32;
        let flashing_now = lit.iter().any(|(r, s, _)| *r == risk && *s == k);
        let color = if flashing_now && flashing(table.clock) {
            WHITE
        } else {
            multiple_color(*m)
        };
        if fits && label_w >= 1 {
            let label = short(*m);
            put(
                buf,
                x - (label.len() as i32 - 1) / 2,
                top as i32,
                &label,
                bold(color),
                art,
            );
        } else {
            put(buf, x, top as i32, LIT_SHELL, bold(color), art);
        }
        if art.height > 2 {
            let cup = if flashing_now { LIT_SHELL } else { SHELL };
            put(buf, x, top as i32 + 1, cup, bold(color), art);
        }
    }
    let mut drawn = HashSet::new();
    for step in (1..=rows).rev() {
        let y = row_y(step);
        if y <= top as i32 + 1 || y >= art.bottom() as i32 - 1 || !drawn.insert(y) {
            continue;
        }
        for j in 0..=step {
            let x = centre + (2 * j as i32 - step as i32) * spacing as i32 / 2;
            put(buf, x, y, PEG, style(PINK), art);
        }
    }
    let launch_y = art.bottom() as i32 - 1;
    if art.height >= 3 {
        put(buf, centre, launch_y, LAUNCH, style(LIGHT_CYAN), art);
    }
    for bubble in bubbles {
        let offset = bubble.offset() as i32;
        let x = centre + offset * spacing as i32 / 2;
        let y = if bubble.step == 0 {
            launch_y
        } else {
            row_y(bubble.step.min(rows))
        };
        let progress = bubble.step as f32 / rows as f32;
        let glyph = if progress < 0.25 {
            "."
        } else if progress < 0.6 {
            "o"
        } else {
            "O"
        };
        let color = if bubble.round.staked.fish().is_some() {
            GOLD
        } else {
            LIGHT_CYAN
        };
        put(buf, x, y, glyph, bold(color), art);
    }
    if art.height >= 4
        && let Some(parts) = result_text(table)
    {
        let width: usize = parts.iter().map(|(s, _)| s.chars().count()).sum();
        let mut x = art.right() as i32 - 1 - width as i32;
        for (text, st) in &parts {
            x += put(buf, x, launch_y, text, *st, art) as i32;
        }
    }
}

fn short(m: Multiple) -> String {
    let raw = m.raw();
    if raw >= 100_000 {
        return format!("{}k", raw / 100_000);
    }
    let label = m.number();
    label.trim_start_matches('0').to_string()
}

fn draw_legend(buf: &mut Buffer, art: Rect, shells: &[Multiple]) -> u16 {
    let half = &shells[..shells.len().div_ceil(2)];
    let mut x = art.x as i32 + 1;
    let mut y = art.y as i32;
    let mut rows = 1;
    for m in half {
        let label = m.label();
        if x + label.chars().count() as i32 >= art.right() as i32 {
            x = art.x as i32 + 1;
            y += 1;
            rows += 1;
        }
        x += put(buf, x, y, &label, style(multiple_color(*m)), art) as i32 + 1;
    }
    rows
}
