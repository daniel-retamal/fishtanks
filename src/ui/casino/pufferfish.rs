use ratatui::{buffer::Buffer, layout::Rect, style::Style};

use crate::casino::Multiple;
use crate::casino::pufferfish::{PuffPhase, Pufferfish, STUPID_FROM};
use crate::casino::state::Table;
use crate::colors::{CREAM, DARK_GRAY, KHAKI, LIGHT_GREEN, LIGHT_RED, LIGHT_YELLOW, TAN, WHITE};
use crate::ui::hint_bar::HintBar;
use crate::ui::hints::{
    HINT_AUTO_CASH_OUT, HINT_ENTER_CASH_OUT, HINT_ENTER_PUFF, HINT_ESC_CASH_OUT,
};

use super::{
    BIG_ROWS, Room, bet_hints, big_fits, big_width, bold, cash, centred, draw_big,
    draw_parts_centred, flash_color, glisten, open_frame, put, result_text, stake_parts,
    status_row, style,
};

const BASIS: (u16, u16) = (58, 22);
const BIG_FROM_ROWS: u16 = 14;
const RADIUS_PER_DOUBLING: f64 = 1.5;
const WIDTH_PER_ROW: f64 = 2.3;
const SPIKES: usize = 12;
const SHAKE_HZ: f32 = 14.0;
const SHARDS: usize = 22;
const SHARD_SPEED_X: f32 = 18.0;
const SHARD_SPEED_Y: f32 = 7.0;
const SHARD_LIFE: f32 = 1.0;
const POP: &str = "pop!";
const RESULT_FROM_ROWS: u16 = 4;
const RESTING: &str = ">ooooooº>";
const SPENT: &str = ">oooooox>";

pub fn draw(buf: &mut Buffer, room: &Room, table: &Table, puffer: &Pufferfish) {
    let hints = if puffer.is_puffing() {
        HintBar::new(HINT_ESC_CASH_OUT).action(HINT_ENTER_CASH_OUT)
    } else {
        bet_hints(table, room.teller, HINT_ENTER_PUFF, &[HINT_AUTO_CASH_OUT])
    };
    let border = match puffer.phase {
        PuffPhase::Puffing { .. } if puffer.now() >= STUPID_FROM => glisten(table.clock, 0),
        _ => flash_color(table.flash),
    };
    let chrome = open_frame(
        buf,
        room.screen,
        "Pufferfish",
        BASIS,
        border,
        &hints,
        super::widest_bet(table, HINT_ENTER_PUFF, &[HINT_AUTO_CASH_OUT]),
    );
    status_row(
        buf,
        chrome.status,
        &status(table, puffer, room.teller),
        room.purse,
    );
    let art = chrome.art;
    let now = puffer.now();
    let color = match puffer.phase {
        PuffPhase::Waiting => WHITE,
        PuffPhase::Puffing { .. } => LIGHT_YELLOW,
        PuffPhase::Popped { .. } => LIGHT_RED,
        PuffPhase::Cashed { .. } => LIGHT_GREEN,
    };
    let label = now.label();
    let mut floor = art.y + 1;
    if art.height >= BIG_FROM_ROWS && big_fits(&label) && big_width(&label) < art.width {
        let stupid = now >= STUPID_FROM;
        draw_big(buf, art, art.y + 1, &label, |i| {
            if stupid {
                glisten(table.clock, i)
            } else {
                color
            }
        });
        floor = art.y + 1 + BIG_ROWS + 1;
    } else {
        centred(buf, art, art.y, &label, bold(color));
    }
    let history = art.height >= 7;
    let bottom = art.bottom().saturating_sub(u16::from(history) * 2);
    let stage =
        Rect::new(art.x, floor, art.width, bottom.saturating_sub(floor).max(1)).intersection(art);
    let centre = (stage.x + stage.width / 2) as i32;
    let middle = (stage.y + stage.height / 2) as i32;
    match puffer.phase {
        PuffPhase::Popped { t, .. } => {
            draw_pop(buf, stage, centre, middle, t);
        }
        _ => {
            let max = (stage.height.saturating_sub(1) / 2)
                .saturating_sub(u16::from(stage.height >= 9)) as i32;
            let max = max
                .min(((stage.width as f64 - 8.0) / (WIDTH_PER_ROW * 2.0)).floor() as i32)
                .max(0);
            let want = if puffer.is_puffing() {
                (now.as_f64().log2() * RADIUS_PER_DOUBLING).floor() as i32
            } else {
                0
            };
            let shake = if puffer.is_puffing() && want > max {
                if ((table.clock * SHAKE_HZ) as u32).is_multiple_of(2) {
                    1
                } else {
                    -1
                }
            } else {
                0
            };
            draw_puffer(
                buf,
                stage,
                centre + shake,
                middle,
                want.min(max),
                table.clock,
                now >= STUPID_FROM,
            );
        }
    }
    if art.height >= RESULT_FROM_ROWS
        && let Some(parts) = result_text(table).filter(|_| !puffer.is_puffing())
    {
        let y = art.bottom() - 1 - u16::from(history);
        draw_parts_centred(buf, art, y, &parts);
    }
    if history {
        draw_history(buf, art, puffer);
    }
}

fn status(
    table: &Table,
    puffer: &Pufferfish,
    teller: &dyn crate::casino::Teller,
) -> Vec<(String, Style)> {
    if puffer.is_puffing()
        && let Some(round) = &table.round
    {
        let value = puffer.now().of(round.on_the_line());
        let who = round
            .staked
            .fish()
            .map_or("On the line ".to_string(), |n| format!("{n} "));
        return vec![(who, style(DARK_GRAY)), (cash(value), bold(LIGHT_YELLOW))];
    }
    let mut parts = stake_parts(table, teller);
    let auto = puffer.target().map_or("off".to_string(), |t| t.label());
    parts.push(("   Auto ".to_string(), style(DARK_GRAY)));
    parts.push((
        auto,
        bold(if puffer.target().is_some() {
            WHITE
        } else {
            DARK_GRAY
        }),
    ));
    parts
}

fn draw_puffer(buf: &mut Buffer, area: Rect, cx: i32, cy: i32, r: i32, clock: f32, stupid: bool) {
    let palette = [KHAKI, TAN, CREAM];
    if r <= 0 {
        let x = cx - RESTING.len() as i32 / 2;
        for (i, ch) in RESTING.chars().enumerate() {
            let fg = if ch == 'º' { WHITE } else { palette[i % 3] };
            put(buf, x + i as i32, cy, &ch.to_string(), bold(fg), area);
        }
        return;
    }
    let ry = r as f64 + 0.5;
    let rx = (r as f64 * WIDTH_PER_ROW).round();
    let inside = |x: i32, y: i32| {
        let (fx, fy) = (x as f64, y as f64);
        fx * fx / (rx * rx) + fy * fy / (ry * ry) <= 1.0
    };
    let edge_color = |i: i32| {
        if stupid {
            glisten(clock, i.unsigned_abs() as usize)
        } else {
            KHAKI
        }
    };
    for y in -r..=r {
        for x in -(rx as i32) - 1..=rx as i32 + 1 {
            if !inside(x, y) {
                continue;
            }
            let up = !inside(x, y - 1);
            let down = !inside(x, y + 1);
            let left = !inside(x - 1, y);
            let right = !inside(x + 1, y);
            let glyph = if (left || right) && y == 0 {
                Some(if x < 0 { '(' } else { ')' })
            } else if (up || down) && !left && !right {
                Some('-')
            } else if up && (left || right) {
                Some('.')
            } else if down && (left || right) {
                Some('\'')
            } else if left || right {
                Some(if x < 0 { '(' } else { ')' })
            } else {
                None
            };
            match glyph {
                Some(ch) => {
                    put(
                        buf,
                        cx + x,
                        cy + y,
                        &ch.to_string(),
                        bold(edge_color(x + y)),
                        area,
                    );
                }
                None if (x + 2 * y).rem_euclid(3) == 0 => {
                    let fg = palette[(x * 7 + y * 3).rem_euclid(3) as usize];
                    put(buf, cx + x, cy + y, "o", style(fg), area);
                }
                None => {}
            }
        }
    }
    let eye_x = cx + (rx * 0.5).round() as i32;
    let eye_y = cy - (r as f64 * 0.35).round() as i32;
    put(buf, eye_x, eye_y, "º", bold(WHITE), area);
    put(buf, cx + rx as i32 + 1, cy, ">", bold(KHAKI), area);
    put(buf, cx - rx as i32 - 1, cy, ">", bold(KHAKI), area);
    if r < 2 {
        return;
    }
    for k in 0..SPIKES {
        let angle = k as f64 / SPIKES as f64 * std::f64::consts::TAU + 0.26;
        let (s, c) = angle.sin_cos();
        let sx = cx + (c * (rx + 1.6)).round() as i32;
        let sy = cy + (s * (ry + 0.5)).round() as i32;
        let ch = if s.abs() > 0.85 {
            '|'
        } else if c.abs() > 0.85 {
            '-'
        } else if (c > 0.0) == (s < 0.0) {
            '/'
        } else {
            '\\'
        };
        let fg = if stupid { glisten(clock, k) } else { TAN };
        put(buf, sx, sy, &ch.to_string(), style(fg), area);
    }
}

fn draw_pop(buf: &mut Buffer, area: Rect, cx: i32, cy: i32, t: f32) {
    if t < SHARD_LIFE {
        for k in 0..SHARDS {
            let angle = k as f32 / SHARDS as f32 * std::f32::consts::TAU;
            let speed = 0.7 + (k % 5) as f32 * 0.15;
            let x = cx + (angle.cos() * SHARD_SPEED_X * speed * t).round() as i32;
            let y = cy + (angle.sin() * SHARD_SPEED_Y * speed * t).round() as i32;
            let ch = ['*', '\'', ',', '.', 'o'][k % 5];
            let fg = if t < SHARD_LIFE / 2.0 {
                LIGHT_RED
            } else {
                KHAKI
            };
            put(buf, x, y, &ch.to_string(), bold(fg), area);
        }
    } else {
        let x = cx - SPENT.len() as i32 / 2;
        put(buf, x, cy, SPENT, style(DARK_GRAY), area);
    }
    let pop_y = (cy + 2).min(area.bottom() as i32 - 1) as u16;
    centred(buf, area, pop_y, POP, bold(LIGHT_RED));
}

fn draw_history(buf: &mut Buffer, art: Rect, puffer: &Pufferfish) {
    let y = art.bottom() as i32 - 1;
    let mut x = art.x as i32 + 1;
    x += put(buf, x, y, "Last:", style(DARK_GRAY), art) as i32 + 1;
    for pop in &puffer.history {
        let label = pop.label();
        if x + label.len() as i32 >= art.right() as i32 {
            break;
        }
        let fg = if *pop < Multiple::tenths(15) {
            LIGHT_RED
        } else if *pop < Multiple::whole(3) {
            LIGHT_YELLOW
        } else if *pop >= STUPID_FROM {
            crate::colors::GOLD
        } else {
            LIGHT_GREEN
        };
        x += put(buf, x, y, &label, style(fg), art) as i32 + 1;
    }
}
