use ratatui::{buffer::Buffer, layout::Rect, style::Color};

use crate::casino::spins::{
    CELLS, Dive, DiveStage, LEVER_SECS, PEARLS_TO_DIVE, REELS, RESPINS, ROWS, SpinPhase, Spins,
    Symbol, at, cell,
};
use crate::casino::state::Table;
use crate::colors::{
    AMBER, CYAN, DARK_GRAY, GOLD_BRIGHT, GOLD_PALE, KHAKI, LIGHT_RED, LIGHT_YELLOW, SILVER, STEEL,
    WHITE, blended,
};
use crate::ui::hint_bar::HintBar;
use crate::ui::hints::{HINT_ESC_LEAVE, HINT_PAYTABLE, HINT_SPIN};

use super::{
    BACKGROUND, Room, bet_hints, bold, cash_within, centred, draw_parts_centred, flash_color,
    flashing, glisten, open_frame, put, result_text, stake_parts, status_row, style,
};

const BASIS: (u16, u16) = (60, 21);
const LONG_CELL: u16 = 7;
const SHORT_CELL: u16 = 3;
const LONG_FROM: u16 = 40;
const SPINNING: &str = "the reels are spinning";
const DIVING: &str = "Pearl Dive";
const IDLE_LINE: &str = "3 Golden Pearls start a Pearl Dive";
const LAMP_ON: &str = "●";
const LAMP_OFF: &str = "○";
const OPENING: &str = "every pearl holds a prize";
const RESETS: &str = "a new pearl resets them";

pub fn glyphs(symbol: Symbol) -> (&'static str, &'static str, Color) {
    match symbol {
        Symbol::Bubbles => ("  °o°  ", "°o°", CYAN),
        Symbol::Anchoveta => ("  ><>  ", "><>", SILVER),
        Symbol::Merluza => ("<>)))º>", ")º>", STEEL),
        Symbol::Pufferfish => (" >ooº> ", "oº>", KHAKI),
        Symbol::Coffee => (" c[_]  ", "c[]", AMBER),
        Symbol::Cash => (" [ $ ] ", "[$]", GOLD_PALE),
        Symbol::Pearl => (" ( ● ) ", "(●)", GOLD_BRIGHT),
    }
}

pub fn draw(buf: &mut Buffer, room: &Room, table: &Table, spins: &Spins) {
    let hints = match spins.phase {
        SpinPhase::Idle => bet_hints(table, room.teller, HINT_SPIN, &[HINT_PAYTABLE]),
        SpinPhase::Spinning => HintBar::new(HINT_ESC_LEAVE).action(SPINNING),
        SpinPhase::Diving(_) => HintBar::new(HINT_ESC_LEAVE).action(DIVING),
    };
    let border = match spins.phase {
        SpinPhase::Diving(_) => glisten(table.clock, 0),
        _ if spins.in_suspense() && flashing(table.clock) => LIGHT_YELLOW,
        _ => flash_color(table.flash),
    };
    let chrome = open_frame(
        buf,
        room.screen,
        "Spins",
        BASIS,
        border,
        &hints,
        super::widest_bet(table, HINT_SPIN, &[HINT_PAYTABLE]),
    );
    status_row(
        buf,
        chrome.status,
        &stake_parts(table, room.teller),
        room.purse,
    );
    let art = chrome.art;
    if art.height < ROWS as u16 {
        return draw_inline(buf, art, table, spins);
    }
    if art.height < ROWS as u16 + 2 {
        return draw_bare(buf, art, table, spins);
    }
    let cell_w = if art.width >= LONG_FROM {
        LONG_CELL
    } else {
        SHORT_CELL
    };
    let box_w = cell_w * REELS as u16 + REELS as u16 + 1;
    let box_h = ROWS as u16 + 2;
    let pot_rows = u16::from(art.height >= box_h + 3);
    let top = art.y + pot_rows + art.height.saturating_sub(box_h + pot_rows + 1) / 2;
    let left = art.x + art.width.saturating_sub(box_w) / 2;
    if pot_rows > 0 {
        draw_pot_line(buf, room, art, art.y, table.clock);
    }
    let frame = Rect::new(left, top, box_w, box_h).intersection(art);
    let frame_color = if spins.in_suspense() && flashing(table.clock) {
        LIGHT_YELLOW
    } else if spins.dive().is_some() {
        glisten(table.clock, 3)
    } else {
        WHITE
    };
    crate::ui::table::draw_box_border(buf, frame, "", frame_color, BACKGROUND);
    for i in 1..REELS as u16 {
        let x = left + i * (cell_w + 1);
        put(buf, x as i32, top as i32, "┬", style(frame_color), art);
        for row in 1..=ROWS as u16 {
            put(
                buf,
                x as i32,
                (top + row) as i32,
                "│",
                style(frame_color),
                art,
            );
        }
        put(
            buf,
            x as i32,
            (top + box_h - 1) as i32,
            "┴",
            style(frame_color),
            art,
        );
    }
    match spins.dive() {
        Some(dive) => draw_dive(buf, art, left, top, cell_w, dive, table.clock),
        None => draw_reels(buf, art, left, top, cell_w, spins, table),
    }
    let mid = (top + 1 + ROWS as u16 / 2) as i32;
    let winning = table.result.as_ref().is_some_and(|r| r.won()) && spins.is_idle();
    let marks = if winning {
        glisten(table.clock, 1)
    } else {
        DARK_GRAY
    };
    put(buf, left as i32 - 2, mid, "▶", bold(marks), art);
    put(buf, (left + box_w + 1) as i32, mid, "◀", bold(marks), art);
    draw_lever(buf, art, left + box_w + 3, top, box_h, spins.lever);
    let below = top + box_h + u16::from(art.bottom() > top + box_h + 1);
    if below >= art.bottom() {
        return;
    }
    if let Some(dive) = spins.dive() {
        centred(
            buf,
            art,
            below,
            &dive_line(dive),
            bold(glisten(table.clock, 2)),
        );
    } else if let Some(parts) = result_text(table).filter(|_| spins.is_idle()) {
        draw_parts_centred(buf, art, below, &parts);
    } else if spins.is_idle() {
        centred(buf, art, below, IDLE_LINE, style(DARK_GRAY));
    }
}

fn dive_line(dive: &Dive) -> String {
    match dive.stage() {
        DiveStage::Surfacing => format!("{PEARLS_TO_DIVE} Golden Pearls!"),
        DiveStage::Opening => OPENING.to_string(),
        DiveStage::Rules => format!("{RESPINS} respins: {RESETS}   {}", lamps(dive)),
        DiveStage::Respins => format!("Respins {}   {}", lamps(dive), dive.total().label()),
    }
}

fn lamps(dive: &Dive) -> String {
    (0..RESPINS)
        .map(|i| if i < dive.respins { LAMP_ON } else { LAMP_OFF })
        .collect::<Vec<_>>()
        .join(" ")
}

fn draw_pot_line(buf: &mut Buffer, room: &Room, art: Rect, y: u16, clock: f32) {
    let amount = cash_within(room.casino.pot, art.width as usize / 2);
    let label = "The Pot ";
    let width = (label.len() + amount.chars().count()) as u16;
    let x = art.x + art.width.saturating_sub(width) / 2;
    put(buf, x as i32, y as i32, label, style(DARK_GRAY), art);
    for (i, ch) in amount.chars().enumerate() {
        put(
            buf,
            x as i32 + label.len() as i32 + i as i32,
            y as i32,
            &ch.to_string(),
            bold(glisten(clock, i)),
            art,
        );
    }
}

fn draw_reels(
    buf: &mut Buffer,
    art: Rect,
    left: u16,
    top: u16,
    cell_w: u16,
    spins: &Spins,
    table: &Table,
) {
    let winning = table.result.as_ref().is_some_and(|r| r.won()) && spins.is_idle();
    for (i, reel) in spins.reels.iter().enumerate() {
        let x = left + 1 + i as u16 * (cell_w + 1);
        let stop = reel.stop();
        for row in 0..ROWS {
            let offset = row as isize - 1;
            let symbol = at(stop + offset);
            let (long, short, color) = glyphs(symbol);
            let text = if cell_w == LONG_CELL { long } else { short };
            let blurred = reel.is_blurred();
            let y = (top + 1 + row as u16) as i32;
            for (k, ch) in text.chars().enumerate() {
                let fg = if blurred {
                    DARK_GRAY
                } else if symbol == Symbol::Pearl {
                    glisten(table.clock, k + i * 3)
                } else if offset != 0 {
                    blended(color, crate::colors::BLACK, 0.45)
                } else if winning && flashing(table.clock * 0.6) {
                    GOLD_BRIGHT
                } else {
                    color
                };
                if ch != ' ' {
                    let st = if offset == 0 && !blurred {
                        bold(fg)
                    } else {
                        style(fg)
                    };
                    put(buf, x as i32 + k as i32, y, &ch.to_string(), st, art);
                }
            }
        }
    }
}

fn draw_dive(
    buf: &mut Buffer,
    art: Rect,
    left: u16,
    top: u16,
    cell_w: u16,
    dive: &Dive,
    clock: f32,
) {
    let stage = dive.stage();
    for index in 0..CELLS {
        let reel = index % REELS;
        let row = index / REELS;
        let x = (left + 1 + reel as u16 * (cell_w + 1)) as i32;
        let y = (top + 1 + row as u16) as i32;
        let landed = dive.window[reel][row];
        if stage == DiveStage::Surfacing || (dive.cells[index].is_some() && !dive.is_open(index)) {
            let (long, short, color) = glyphs(landed);
            let text = if cell_w == LONG_CELL { long } else { short };
            let pearl = landed == Symbol::Pearl;
            for (k, ch) in text.chars().enumerate() {
                if ch == ' ' {
                    continue;
                }
                let st = if !pearl {
                    style(blended(color, crate::colors::BLACK, 0.7))
                } else if flashing(clock) {
                    bold(WHITE)
                } else {
                    bold(glisten(clock, k + index))
                };
                put(buf, x + k as i32, y, &ch.to_string(), st, art);
            }
            continue;
        }
        if dive.cells[index].is_none() && stage != DiveStage::Respins {
            continue;
        }
        match dive.cells[cell(reel, row)] {
            Some(value) => {
                let label = value.label();
                let text = if cell_w == LONG_CELL {
                    format!("{:^7}", label)
                } else {
                    format!("{:^3}", label.trim_start_matches('×'))
                };
                let fresh = dive.fresh[index] && flashing(clock);
                for (k, ch) in text.chars().enumerate() {
                    if ch != ' ' {
                        let fg = if fresh {
                            WHITE
                        } else {
                            glisten(clock, k + index)
                        };
                        put(buf, x + k as i32, y, &ch.to_string(), bold(fg), art);
                    }
                }
            }
            None => {
                let mut stop = (clock * 20.0) as isize + index as isize * 7;
                while at(stop) == Symbol::Pearl {
                    stop += 1;
                }
                let (long, short, _) = glyphs(at(stop));
                let text = if cell_w == LONG_CELL { long } else { short };
                let fg = if dive.over { LIGHT_RED } else { DARK_GRAY };
                put(buf, x, y, text, style(fg), art);
            }
        }
    }
}

fn draw_lever(buf: &mut Buffer, art: Rect, x: u16, top: u16, height: u16, lever: f32) {
    if x >= art.right() {
        return;
    }
    let pulled = if lever > 0.0 {
        ((1.0 - lever / LEVER_SECS) * std::f32::consts::PI).sin()
    } else {
        0.0
    };
    for y in top..top + height {
        put(buf, x as i32, y as i32, "|", style(DARK_GRAY), art);
    }
    put(
        buf,
        x as i32 - 1,
        (top + height - 1) as i32,
        "_",
        style(DARK_GRAY),
        art,
    );
    let knob = top as i32 - 1 + (pulled * (height.saturating_sub(1)) as f32).round() as i32;
    put(
        buf,
        x as i32,
        knob.max(art.y as i32),
        "●",
        bold(LIGHT_RED),
        art,
    );
}

fn draw_bare(buf: &mut Buffer, art: Rect, table: &Table, spins: &Spins) {
    let box_w = SHORT_CELL * REELS as u16 + REELS as u16 + 1;
    let left = art.x + art.width.saturating_sub(box_w) / 2;
    let top = art.y + art.height.saturating_sub(ROWS as u16) / 2 - 1;
    match spins.dive() {
        Some(dive) => draw_dive(buf, art, left, top, SHORT_CELL, dive, table.clock),
        None => draw_reels(buf, art, left, top, SHORT_CELL, spins, table),
    }
    let mid = (top + 1 + ROWS as u16 / 2) as i32;
    let winning = table.result.as_ref().is_some_and(|r| r.won()) && spins.is_idle();
    let marks = if winning {
        glisten(table.clock, 1)
    } else {
        DARK_GRAY
    };
    put(buf, left as i32 - 1, mid, "▶", bold(marks), art);
    put(buf, (left + box_w) as i32, mid, "◀", bold(marks), art);
}

fn draw_inline(buf: &mut Buffer, art: Rect, table: &Table, spins: &Spins) {
    let x0 = art.x as i32 + (art.width as i32 - 15).max(0) / 2;
    let winning = table.result.as_ref().is_some_and(|r| r.won()) && spins.is_idle();
    let marks = if winning {
        glisten(table.clock, 0)
    } else {
        DARK_GRAY
    };
    put(buf, x0, art.y as i32, "▶", bold(marks), art);
    match spins.dive() {
        Some(dive) => {
            let line = dive_line(dive);
            put(
                buf,
                x0 + 2,
                art.y as i32,
                &line,
                bold(glisten(table.clock, 0)),
                art,
            );
        }
        None => {
            for (i, reel) in spins.reels.iter().enumerate() {
                let (_, short, color) = glyphs(at(reel.stop()));
                let fg = if reel.is_blurred() { DARK_GRAY } else { color };
                put(
                    buf,
                    x0 + 2 + i as i32 * 4,
                    art.y as i32,
                    short,
                    bold(fg),
                    art,
                );
            }
            put(buf, x0 + 14, art.y as i32, "◀", bold(marks), art);
        }
    }
    if art.height < 2 {
        return;
    }
    if let Some(parts) = result_text(table).filter(|_| spins.is_idle()) {
        draw_parts_centred(buf, art, art.y + 1, &parts);
    }
}
