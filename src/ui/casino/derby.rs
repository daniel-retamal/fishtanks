use ratatui::{buffer::Buffer, layout::Rect};

use crate::casino::derby::{Derby, RacePhase};
use crate::casino::state::Table;
use crate::colors::{BLACK, CYAN, DARK_GRAY, LIGHT_CYAN, LIGHT_YELLOW, WHITE, blended};
use crate::ui::hint_bar::HintBar;
use crate::ui::hints::{
    HINT_ALL_IN, HINT_BACK, HINT_DOUBLE_OR_NOTHING, HINT_ENTER_RACE, HINT_ESC_LEAVE,
    HINT_PICK_A_FISH, HINT_STAKE, HINT_TAB_FISH,
};

use super::{
    Room, bold, draw_fish_at, draw_parts_centred, facing_right, fish_rows, flash_color, flashing,
    open_frame, put, result_text, stake_parts, status_row, style,
};

const BASIS: (u16, u16) = (74, 19);
const NAMES_FROM: u16 = 50;
const COMMENTARY_FROM: u16 = 3;
const NAME_W: u16 = 12;
const NUMBER_W: u16 = 3;
const ODDS_W: u16 = 7;
const TRACK_MIN: u16 = 6;
const RIPPLE_HZ: f32 = 10.0;
const IDLE_RIPPLE_HZ: f32 = 2.0;
const RIPPLE_EVERY: i32 = 4;
const TRAIL: &str = "≈≈";
const SPLASH: &str = "°";

pub fn draw(buf: &mut Buffer, room: &Room, table: &Table, derby: &Derby) {
    let running = matches!(derby.phase, RacePhase::Running { .. });
    let hints = if running {
        HintBar::new(HINT_ESC_LEAVE).action("they're off")
    } else {
        HintBar::new(HINT_BACK)
            .action(HINT_PICK_A_FISH)
            .action(HINT_STAKE)
            .action(HINT_ALL_IN)
            .action(HINT_TAB_FISH)
            .action(HINT_ENTER_RACE)
            .action_if(table.can_double(room.teller), HINT_DOUBLE_OR_NOTHING)
    };
    let chrome = open_frame(
        buf,
        room.screen,
        "Derby",
        BASIS,
        flash_color(table.flash),
        &hints,
        HintBar::new(HINT_BACK)
            .action(HINT_PICK_A_FISH)
            .action(HINT_STAKE)
            .action(HINT_ALL_IN)
            .action(HINT_TAB_FISH)
            .action(HINT_ENTER_RACE)
            .action(HINT_DOUBLE_OR_NOTHING)
            .natural_width(),
    );
    let mut parts = stake_parts(table, room.teller);
    let picked = derby.picked();
    if chrome.status.width >= NAMES_FROM {
        parts.push(("  on ".to_string(), style(DARK_GRAY)));
        parts.push((picked.name().to_string(), bold(WHITE)));
    }
    parts.push((format!(" {}", picked.odds().label()), bold(WHITE)));
    status_row(buf, chrome.status, &parts, room.purse);
    draw_track(buf, chrome.art, table, derby);
}

fn draw_track(buf: &mut Buffer, art: Rect, table: &Table, derby: &Derby) {
    let label_w = if art.width >= NAMES_FROM {
        NAME_W
    } else {
        NUMBER_W
    };
    let track_x = art.x + label_w + 1;
    let track_w = art
        .width
        .saturating_sub(label_w + ODDS_W + 2)
        .max(TRACK_MIN);
    let finish_x = track_x + track_w;
    let heights: Vec<u16> = derby
        .lanes
        .iter()
        .map(|l| fish_rows(&l.portrait).max(1))
        .collect();
    let room_rows = if art.height >= COMMENTARY_FROM {
        art.height - 1
    } else {
        art.height
    };
    let natural: u16 = heights.iter().sum::<u16>() + heights.len() as u16 - 1;
    let gap = u16::from(natural <= room_rows);
    let total: u16 = heights.iter().sum::<u16>() + gap * (heights.len() as u16 - 1);
    let first = first_shown(&heights, gap, room_rows, derby.pick);
    let shown: u16 = heights[first..].iter().sum::<u16>()
        + gap * (heights.len() - first).saturating_sub(1) as u16;
    let mut y = art.y + room_rows.saturating_sub(shown.min(total)) / 2;
    let clock = derby.clock();
    let winner = derby.winner();
    let leader = derby
        .lanes
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.pos.partial_cmp(&b.1.pos).unwrap())
        .map(|(i, _)| i)
        .filter(|_| clock > 0.0);
    let ripple = if clock > 0.0 {
        RIPPLE_HZ
    } else {
        IDLE_RIPPLE_HZ
    };
    for (i, lane) in derby.lanes.iter().enumerate().skip(first) {
        let h = heights[i];
        if y + h > art.y + room_rows {
            break;
        }
        let body = y + lane.portrait.line_sprite().body_row.min(h as usize - 1) as u16;
        let chosen = i == derby.pick;
        let label = if label_w == NAME_W {
            crate::ui::table::ellipsize(lane.name(), NAME_W as usize - 2)
        } else {
            (i + 1).to_string()
        };
        put(
            buf,
            art.x as i32,
            body as i32,
            if chosen { ">" } else { " " },
            bold(WHITE),
            art,
        );
        let name_color = if winner == Some(i) && flashing(table.clock) {
            LIGHT_YELLOW
        } else if chosen {
            WHITE
        } else {
            DARK_GRAY
        };
        put(
            buf,
            art.x as i32 + 1,
            body as i32,
            &label,
            if chosen {
                bold(name_color)
            } else {
                style(name_color)
            },
            art,
        );
        for x in 0..track_w as i32 {
            let phase = (x + (table.clock * ripple) as i32).rem_euclid(RIPPLE_EVERY);
            let (glyph, color) = if phase == 0 {
                ("~", blended(CYAN, BLACK, 0.5))
            } else {
                ("·", blended(DARK_GRAY, BLACK, 0.4))
            };
            put(
                buf,
                track_x as i32 + x,
                body as i32,
                glyph,
                style(color),
                art,
            );
        }
        let flag = if (body + i as u16).is_multiple_of(2) {
            "▚"
        } else {
            "▞"
        };
        for row in y..y + h {
            put(buf, finish_x as i32, row as i32, flag, style(WHITE), art);
        }
        let fish = facing_right(&lane.portrait);
        let width = fish.display_width as u16;
        let reach = track_w.saturating_sub(width);
        let x = track_x + (lane.pos * reach as f32).round() as u16;
        if lane.is_zooming(clock) {
            put(
                buf,
                x as i32 - TRAIL.len() as i32,
                body as i32,
                TRAIL,
                bold(LIGHT_CYAN),
                art,
            );
        } else if clock > 0.0 && x > track_x {
            put(
                buf,
                x as i32 - 1,
                body as i32,
                SPLASH,
                style(LIGHT_CYAN),
                art,
            );
        }
        draw_fish_at(buf, &fish, x, y, art);
        let odds = lane.odds().label();
        let strong = chosen || leader == Some(i);
        let odds_color = if strong { WHITE } else { DARK_GRAY };
        let ox = art.right() as i32 - 1 - odds.chars().count() as i32;
        put(
            buf,
            ox,
            body as i32,
            &odds,
            if strong {
                bold(odds_color)
            } else {
                style(odds_color)
            },
            art,
        );
        y += h + gap;
    }
    let line_y = art.bottom() - 1;
    if let Some(parts) = result_text(table).filter(|_| winner.is_some()) {
        draw_parts_centred(buf, art, line_y, &parts);
    } else if let Some((call, _)) = &derby.call {
        super::centred(buf, art, line_y, call, bold(LIGHT_YELLOW));
    } else if let Some(parts) = result_text(table) {
        draw_parts_centred(buf, art, line_y, &parts);
    }
}

fn first_shown(heights: &[u16], gap: u16, room: u16, pick: usize) -> usize {
    let span = |from: usize, to: usize| -> u16 {
        heights[from..=to].iter().sum::<u16>() + gap * (to - from) as u16
    };
    if span(0, heights.len() - 1) <= room {
        return 0;
    }
    let mut first = 0;
    while first < pick && span(first, pick) > room {
        first += 1;
    }
    first
}
