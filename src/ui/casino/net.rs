use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use ratatui::{buffer::Buffer, layout::Rect, style::Color};

use crate::casino::net::{Cast, Net, Prize};
use crate::casino::state::Table;
use crate::colors::{BLACK, DARK_GRAY, GOLD, TAN, VIOLET, WHITE, blended};
use crate::economy::Rarity;
use crate::fishes::fish::{Direction, Fish};
use crate::fishes::species::FishSpecies;
use crate::ui::hint_bar::HintBar;
use crate::ui::hints::{
    HINT_BACK, HINT_DOUBLE_OR_NOTHING, HINT_ENTER_CAST, HINT_ESC_LEAVE, HINT_NET, HINT_TAB_BAIT,
};

use super::{
    Room, bold, cash, draw_parts_centred, flash_color, glisten, open_frame, put, result_text,
    status_row, style,
};

const BASIS: (u16, u16) = (70, 16);
const WIDE_CARD: u16 = 9;
const NARROW_CARD: u16 = 6;
const WIDE_FROM: u16 = 50;
const FULL_FROM_ROWS: u16 = 6;
const FOOD_GLYPH: &str = "•••";
const COMPACT_LABEL: usize = 4;
const CASTING: &str = "the net is out";
const ALL_NETS_FROM: u16 = 68;

type Glyph = Vec<(char, Color)>;

fn species_glyph(species: FishSpecies) -> Glyph {
    static CACHE: OnceLock<Mutex<HashMap<FishSpecies, Glyph>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache.lock().expect("the glyph cache");
    cache
        .entry(species)
        .or_insert_with(|| {
            let mut fish = Fish::new_for_display(species, &mut rand::rng());
            fish.facing = Direction::Right;
            let sprite = fish.line_sprite();
            sprite.rows[sprite.body_row].clone()
        })
        .clone()
}

pub fn rarity_color(rarity: Option<Rarity>, clock: f32, i: usize) -> Color {
    match rarity {
        Some(Rarity::Common) => WHITE,
        Some(Rarity::Rare) => VIOLET,
        Some(Rarity::Legendary) => glisten(clock, i),
        None => TAN,
    }
}

fn label_of(prize: Prize) -> &'static str {
    match prize.rarity() {
        Some(Rarity::Common) => "Common",
        Some(Rarity::Rare) => "Rare",
        Some(Rarity::Legendary) => "Legend",
        None => "Food",
    }
}

pub fn draw(buf: &mut Buffer, room: &Room, table: &Table, cast: &Cast) {
    let hints = if cast.is_casting() {
        HintBar::new(HINT_ESC_LEAVE).action(CASTING)
    } else {
        HintBar::new(HINT_BACK)
            .action(HINT_NET)
            .action(HINT_TAB_BAIT)
            .action(HINT_ENTER_CAST)
            .action_if(table.can_double(room.teller), HINT_DOUBLE_OR_NOTHING)
    };
    let chrome = open_frame(
        buf,
        room.screen,
        "Mystery Net",
        BASIS,
        flash_color(table.flash),
        &hints,
        HintBar::new(HINT_BACK)
            .action(HINT_NET)
            .action(HINT_TAB_BAIT)
            .action(HINT_ENTER_CAST)
            .action(HINT_DOUBLE_OR_NOTHING)
            .natural_width(),
    );
    status_row(
        buf,
        chrome.status,
        &status(table, cast, chrome.status.width),
        room.purse,
    );
    let art = chrome.art;
    if art.height < FULL_FROM_ROWS {
        return draw_compact(buf, art, table, cast, room.clock);
    }
    let card_w = if art.width >= WIDE_FROM {
        WIDE_CARD
    } else {
        NARROW_CARD
    };
    let strip_top = art.y + art.height.saturating_sub(6) / 2 + 1;
    let centre = art.x as i32 + art.width as i32 / 2;
    let base = cast.offset;
    let frac = base - base.floor();
    let visible = (art.width / card_w) as i32 / 2 + 2;
    for i in -visible..=visible {
        let index = base.floor() as i32 + i;
        let Some(prize) = usize::try_from(index).ok().and_then(|k| cast.strip.get(k)) else {
            continue;
        };
        let x = centre + ((i as f32 - frac) * card_w as f32 - card_w as f32 / 2.0).round() as i32;
        let hit = (i as f32 - frac).abs() < 0.5;
        let dim = cast.is_casting() && !hit;
        draw_card(
            buf,
            art,
            x,
            strip_top as i32,
            card_w,
            *prize,
            dim,
            room.clock,
        );
    }
    put(buf, centre, strip_top as i32 - 1, "▼", bold(GOLD), art);
    put(buf, centre, strip_top as i32 + 4, "▲", bold(GOLD), art);
    let line = strip_top + 5;
    if line < art.bottom()
        && let Some(parts) = result_text(table).filter(|_| !cast.is_casting())
    {
        draw_parts_centred(buf, art, line, &parts);
    }
}

fn status(table: &Table, cast: &Cast, width: u16) -> Vec<(String, ratatui::style::Style)> {
    let mut parts = Vec::new();
    if width < ALL_NETS_FROM {
        let index = Net::ALL.iter().position(|n| *n == cast.net).unwrap_or(0);
        parts.push((
            format!("{} {}", cast.net.name(), cash(cast.net.price())),
            bold(WHITE),
        ));
        parts.push((
            format!(" ({}/{})", index + 1, Net::ALL.len()),
            style(DARK_GRAY),
        ));
        if let Some(bait) = &table.bait {
            parts.push((format!("  bait {bait}"), style(DARK_GRAY)));
        }
        return parts;
    }
    for (i, net) in Net::ALL.iter().enumerate() {
        let chosen = *net == cast.net;
        parts.push((
            format!(
                "{}{}",
                if i == 0 { "" } else { "  " },
                if chosen { "> " } else { "" }
            ),
            bold(WHITE),
        ));
        let text = format!("{} {}", net.name(), cash(net.price()));
        parts.push((
            text,
            if chosen {
                bold(WHITE)
            } else {
                style(DARK_GRAY)
            },
        ));
    }
    if let Some(bait) = &table.bait {
        parts.push(("  bait ".to_string(), style(DARK_GRAY)));
        parts.push((bait.clone(), bold(WHITE)));
    }
    parts
}

#[allow(clippy::too_many_arguments)]
fn draw_card(
    buf: &mut Buffer,
    art: Rect,
    x: i32,
    y: i32,
    w: u16,
    prize: Prize,
    dim: bool,
    clock: f32,
) {
    if x < art.x as i32 || x + w as i32 > art.right() as i32 {
        return;
    }
    let rarity = prize.rarity();
    let edge = if dim {
        DARK_GRAY
    } else {
        rarity_color(rarity, clock, 0)
    };
    let inner = w as i32 - 2;
    for dx in 0..w as i32 {
        let (top, bottom) = match dx {
            0 => ("┌", "└"),
            d if d == w as i32 - 1 => ("┐", "┘"),
            _ => ("─", "─"),
        };
        put(buf, x + dx, y, top, style(edge), art);
        put(buf, x + dx, y + 3, bottom, style(edge), art);
    }
    for row in 1..=2 {
        put(buf, x, y + row, "│", style(edge), art);
        put(buf, x + w as i32 - 1, y + row, "│", style(edge), art);
    }
    let glyph: Glyph = match prize {
        Prize::Fish(species) => species_glyph(species),
        Prize::Food => FOOD_GLYPH.chars().map(|c| (c, TAN)).collect(),
    };
    let shown: Glyph = glyph
        .iter()
        .copied()
        .skip(glyph.len().saturating_sub(inner as usize))
        .collect();
    let gx = x + 1 + (inner - shown.len() as i32) / 2;
    for (k, (ch, color)) in shown.iter().enumerate() {
        if *ch == ' ' || *ch == crate::sprite::TRANSPARENT {
            continue;
        }
        let fg = if dim {
            blended(*color, BLACK, 0.5)
        } else {
            *color
        };
        put(buf, gx + k as i32, y + 1, &ch.to_string(), style(fg), art);
    }
    let label: String = label_of(prize).chars().take(inner as usize).collect();
    let lx = x + 1 + (inner - label.len() as i32) / 2;
    let label_color = if dim {
        DARK_GRAY
    } else {
        rarity_color(rarity, clock, 2)
    };
    put(buf, lx, y + 2, &label, bold(label_color), art);
}

fn draw_compact(buf: &mut Buffer, art: Rect, table: &Table, cast: &Cast, clock: f32) {
    let cell = COMPACT_LABEL as i32 + 2;
    let centre = art.x as i32 + art.width as i32 / 2;
    let base = cast.offset;
    let frac = base - base.floor();
    let label_y = if art.height >= 3 { art.y + 1 } else { art.y };
    for i in -6..=6 {
        let index = base.floor() as i32 + i;
        let Some(prize) = usize::try_from(index).ok().and_then(|k| cast.strip.get(k)) else {
            continue;
        };
        let x = centre + ((i as f32 - frac) * cell as f32 - cell as f32 / 2.0).round() as i32;
        let hit = (i as f32 - frac).abs() < 0.5;
        let label: String = label_of(*prize).chars().take(COMPACT_LABEL).collect();
        let color = if hit {
            rarity_color(prize.rarity(), clock, 0)
        } else {
            DARK_GRAY
        };
        if hit {
            put(buf, x, label_y as i32, "[", bold(GOLD), art);
            put(buf, x + cell - 1, label_y as i32, "]", bold(GOLD), art);
        }
        put(buf, x + 1, label_y as i32, &label, bold(color), art);
    }
    if art.height >= 3 {
        put(buf, centre, art.y as i32, "▼", bold(GOLD), art);
    }
    let y = art.bottom() - 1;
    if y > label_y
        && let Some(parts) = result_text(table).filter(|_| !cast.is_casting())
    {
        draw_parts_centred(buf, art, y, &parts);
    }
}
