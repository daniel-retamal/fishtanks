use ratatui::{buffer::Buffer, layout::Rect};

use crate::casino::COMP_EVERY;
use crate::casino::state::Game;
use crate::colors::{AMBER, DARK_GRAY, GOLD, LIGHT_GREEN, LIGHT_RED, WHITE};
use crate::ui::hint_bar::HintBar;
use crate::ui::hints::{HINT_CLOSE, HINT_ENTER_SIT, HINT_NAV};
use crate::ui::panels::{PanelSpec, Panels, Reach};
use crate::ui::table::visual_width;

use super::{
    BACKGROUND, Room, bold, cash_within, draw_tollomind, glisten, put, signed_cash, style,
    tollomind_size,
};

const TITLE: &str = " Casino ";
const SIDE_PAD: u16 = 2;
const BODY_W: u16 = 46;
const BODY_MIN_W: u16 = 16;
const NAME_GAP: usize = 2;
const STATS: u16 = 4;
const TIGHT_STATS: u16 = 2;

#[derive(Clone)]
struct Stat {
    label: &'static str,
    short: &'static str,
    brief: String,
    value: String,
    color: ratatui::style::Color,
    pot: bool,
}

pub fn draw(buf: &mut Buffer, room: &Room, selected: usize) {
    let (art_w, art_h) = tollomind_size();
    let rows = Game::ALL.len() as u16 + 1 + STATS;
    let hints_for = |overflowing: bool| {
        HintBar::new(HINT_CLOSE)
            .counted(
                HINT_NAV,
                overflowing.then_some((selected + 1, Game::ALL.len())),
            )
            .action(HINT_ENTER_SIT)
    };
    let calm = hints_for(false);
    let rows_for = |_: u16| rows;
    let spec = |hints| PanelSpec {
        title: TITLE,
        title_style: bold(WHITE),
        border: style(WHITE),
        background: BACKGROUND,
        side: (art_w + SIDE_PAD * 2, art_h),
        body_w: BODY_W,
        body_min_w: BODY_MIN_W,
        body_rows: &rows_for,
        hints,
        reach: Reach::Tab,
    };
    let overflowing = Panels::measure(room.screen, &spec(&calm)).body.height < rows;
    let bar = hints_for(overflowing);
    let panels = Panels::open(buf, room.screen, &spec(&bar));
    let side = panels.side;
    let x = side.x + side.width.saturating_sub(art_w) / 2;
    draw_tollomind(buf, x as i32, side.y as i32, side);
    draw_body(buf, room, panels.body, selected);
}

fn draw_body(buf: &mut Buffer, room: &Room, body: Rect, selected: usize) {
    let stats = stats(room);
    let full = body.height >= Game::ALL.len() as u16 + 1 + STATS;
    let stat_rows = if full {
        STATS
    } else {
        TIGHT_STATS.min(body.height.saturating_sub(1))
    };
    let list_rows = body
        .height
        .saturating_sub(stat_rows + u16::from(full))
        .max(1) as usize;
    let start = selected
        .saturating_sub(list_rows.saturating_sub(1))
        .min(Game::ALL.len() - list_rows.min(Game::ALL.len()));
    let name_w = Game::ALL.iter().map(|g| g.name().len()).max().unwrap_or(0) + NAME_GAP;
    let tag_w = Game::ALL
        .iter()
        .map(|g| g.tagline().len())
        .max()
        .unwrap_or(0);
    let tags = body.width as usize > 2 + name_w + tag_w;
    for (row, game) in Game::ALL.iter().enumerate().skip(start).take(list_rows) {
        let y = body.y as i32 + (row - start) as i32;
        let open = game.is_open(room.teller);
        let chosen = row == selected;
        put(
            buf,
            body.x as i32 + 1,
            y,
            if chosen { "> " } else { "  " },
            bold(WHITE),
            body,
        );
        let name_style = if !open {
            style(DARK_GRAY)
        } else if chosen {
            bold(WHITE)
        } else {
            style(WHITE)
        };
        put(buf, body.x as i32 + 3, y, game.name(), name_style, body);
        if tags {
            put(
                buf,
                body.x as i32 + 3 + name_w as i32,
                y,
                game.tagline(),
                style(DARK_GRAY),
                body,
            );
        }
    }
    let first = body.bottom() as i32 - stat_rows as i32;
    if full {
        for (i, stat) in stats.iter().enumerate() {
            stat_line(buf, room, body, first + i as i32, stat.label, stat);
        }
        return;
    }
    if stat_rows == 0 {
        return;
    }
    pair(buf, room, body, &stats[0], &stats[1], first);
    if stat_rows > 1 {
        pair(buf, room, body, &stats[2], &stats[3], first + 1);
    }
}

fn pair(buf: &mut Buffer, room: &Room, body: Rect, a: &Stat, b: &Stat, y: i32) {
    let left = Stat {
        value: a.brief.clone(),
        ..a.clone()
    };
    let right = Stat {
        value: b.brief.clone(),
        ..b.clone()
    };
    put(
        buf,
        body.x as i32 + 1,
        y,
        left.short,
        style(DARK_GRAY),
        body,
    );
    paint(
        buf,
        room,
        body,
        body.x as i32 + 2 + left.short.len() as i32,
        y,
        &left,
    );
    let text = format!("{} {}", right.short, right.value);
    let rx = body.right() as i32 - 1 - visual_width(&text) as i32;
    put(buf, rx, y, right.short, style(DARK_GRAY), body);
    paint(
        buf,
        room,
        body,
        rx + right.short.len() as i32 + 1,
        y,
        &right,
    );
}

fn paint(buf: &mut Buffer, room: &Room, body: Rect, x: i32, y: i32, stat: &Stat) {
    if stat.pot {
        for (i, ch) in stat.value.chars().enumerate() {
            put(
                buf,
                x + i as i32,
                y,
                &ch.to_string(),
                bold(glisten(room.clock, i)),
                body,
            );
        }
        return;
    }
    put(buf, x, y, &stat.value, style(stat.color), body);
}

fn stat_line(buf: &mut Buffer, room: &Room, body: Rect, y: i32, label: &str, stat: &Stat) {
    put(buf, body.x as i32 + 1, y, label, style(DARK_GRAY), body);
    let x = body.right() as i32 - 1 - visual_width(&stat.value) as i32;
    paint(buf, room, body, x, y, stat);
}

fn stats(room: &Room) -> [Stat; 4] {
    let casino = room.casino;
    let tonight_color = match casino.tonight.signum() {
        1 => LIGHT_GREEN,
        -1 => LIGHT_RED,
        _ => WHITE,
    };
    let to_next = COMP_EVERY - casino.comp_progress;
    [
        Stat {
            label: "Tonight",
            short: "Tonight",
            value: signed_cash(casino.tonight),
            brief: brief_signed(casino.tonight),
            color: tonight_color,
            pot: false,
        },
        Stat {
            label: "Best win",
            short: "Best",
            value: cash_within(casino.best_win, 12),
            brief: compact(casino.best_win),
            color: if casino.best_win > 0 { GOLD } else { WHITE },
            pot: false,
        },
        Stat {
            label: "Next coffee",
            short: "Coffee",
            value: cash_within(to_next, 12),
            brief: compact(to_next),
            color: AMBER,
            pot: false,
        },
        Stat {
            label: "The Pot",
            short: "Pot",
            value: cash_within(casino.pot, 14),
            brief: compact(casino.pot),
            color: GOLD,
            pot: true,
        },
    ]
}

fn compact(amount: crate::economy::Money) -> String {
    format!("${}", crate::ui::command_bar::metric(amount))
}

fn brief_signed(amount: i128) -> String {
    let sign = if amount < 0 { "-" } else { "+" };
    format!("{sign}{}", compact(amount.unsigned_abs()))
}
