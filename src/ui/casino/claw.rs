use ratatui::{buffer::Buffer, layout::Rect, style::Style};

use crate::casino::claw::{
    CAPSULE, CHUTE_TOP, Claw, GLASS_H, GLASS_W, GRAVEL, Goods, Motion, Phase, Prize, RAIL, WALL_X,
    weight,
};
use crate::casino::state::Table;
use crate::colors::{DARK_GRAY, GOLD, LIGHT_YELLOW, WHITE};
use crate::fishes::fish::LineSprite;
use crate::fishes::toy::Shelf;
use crate::ui::hint_bar::HintBar;
use crate::ui::hints::{HINT_BACK, HINT_DROP, HINT_ENTER_PAY, HINT_PAYTABLE, HINT_STEER};

use super::{
    Room, bold, cash, draw_parts_centred, draw_sprite, flash_color, open_frame, put, result_text,
    status_row, style,
};

const BASIS: (u16, u16) = (GLASS_W as u16 + 2, GLASS_H as u16 + 7);
const RAIL_GLYPH: &str = "═";
const GRAVEL_GLYPH: &str = "░";
const WALL_GLYPH: &str = "|";
const VENT: &str = "PRIZE";
const CABLE: &str = "|";
const HEAD: &str = "_|_";
const OPEN: &str = "/ \\";
const SHUT: &str = "\\ /";
const LEAN_RIGHT: &str = "\\";
const LEAN_LEFT: &str = "/";
const NEW: &str = "new!";
const PIP: &str = "●";
const NO_PIP: &str = "○";
pub const PIPS: usize = 4;

pub fn pips(worth: crate::economy::Money) -> String {
    let full = weight(worth);
    format!("{}{}", PIP.repeat(full), NO_PIP.repeat(PIPS - full))
}

fn hints(claw: &Claw) -> HintBar {
    match claw.phase {
        Phase::Steering => HintBar::new(HINT_BACK).action(HINT_STEER).action(HINT_DROP),
        Phase::Ready => HintBar::new(HINT_BACK)
            .action(HINT_ENTER_PAY)
            .action(HINT_PAYTABLE),
        _ => HintBar::new(HINT_BACK),
    }
}

fn widest() -> u16 {
    HintBar::new(HINT_BACK)
        .action(HINT_ENTER_PAY)
        .action(HINT_PAYTABLE)
        .natural_width()
        .max(
            HintBar::new(HINT_BACK)
                .action(HINT_STEER)
                .action(HINT_DROP)
                .natural_width(),
        )
}

pub fn draw(buf: &mut Buffer, room: &Room, table: &Table, claw: &Claw) {
    let bar = hints(claw);
    let chrome = open_frame(
        buf,
        room.screen,
        "Claw",
        BASIS,
        flash_color(table.flash),
        &bar,
        widest(),
    );
    let room_for_toys = room.teller.room_for_a_prize();
    let shelf = room.teller.shelf();
    status_row(
        buf,
        chrome.status,
        &status(claw, room_for_toys, &shelf),
        room.purse,
    );
    let art = chrome.art;
    let glass = Rect::new(art.x, art.y, art.width, art.height.saturating_sub(1).max(1));
    let pan_x = pan(claw.shown_x(), glass.width as i32, GLASS_W);
    let focus_y = match claw.phase {
        Phase::Ready | Phase::Steering => (claw.hand.y + GRAVEL) / 2,
        _ => claw.hand.y,
    };
    let pan_y = pan(focus_y, glass.height as i32, GLASS_H);
    let left = glass.x as i32 + (glass.width as i32 - GLASS_W).max(0) / 2 - pan_x;
    let top = glass.y as i32 - pan_y;
    let at = |x: i32, y: i32| (left + x, top + y);
    for x in 0..GLASS_W {
        let (sx, sy) = at(x, RAIL);
        put(buf, sx, sy, RAIL_GLYPH, style(DARK_GRAY), glass);
        let (gx, gy) = at(x, GRAVEL);
        put(buf, gx, gy, GRAVEL_GLYPH, style(DARK_GRAY), glass);
    }
    for y in CHUTE_TOP..=GRAVEL {
        let (wx, wy) = at(WALL_X, y);
        put(buf, wx, wy, WALL_GLYPH, style(DARK_GRAY), glass);
    }
    let vent_x = WALL_X + 1 + (GLASS_W - WALL_X - 1 - VENT.len() as i32) / 2;
    let (vx, vy) = at(vent_x, GRAVEL);
    put(buf, vx, vy, VENT, bold(DARK_GRAY), glass);
    for prize in &claw.prizes {
        let grey = !room_for_toys && prize.goods.needs_room();
        draw_prize(buf, prize, claw.clock, at(0, 0), grey, glass);
    }
    let chute = Rect::new(
        glass.x,
        glass.y,
        glass.width,
        ((top + GRAVEL) - glass.y as i32).clamp(0, glass.height as i32) as u16,
    );
    for prize in claw.held() {
        let clip = if prize.motion == Motion::Chute {
            chute
        } else {
            glass
        };
        draw_prize(buf, prize, claw.clock, at(0, 0), false, clip);
    }
    draw_hand(buf, claw, at(0, 0), glass);
    if !claw.is_busy()
        && art.height >= 2
        && let Some(parts) = result_text(table)
    {
        draw_parts_centred(buf, art, art.bottom() - 1, &parts);
    }
}

fn pan(focus: i32, room: i32, total: i32) -> i32 {
    if room >= total {
        return 0;
    }
    (focus - room / 2).clamp(0, total - room)
}

fn draw_prize(
    buf: &mut Buffer,
    prize: &Prize,
    clock: f32,
    origin: (i32, i32),
    grey: bool,
    clip: Rect,
) {
    let x = origin.0 + prize.left();
    let y = origin.1 + prize.y;
    match &prize.goods {
        Goods::Toy(_) => {
            let Some(sprite) = prize.sprite(clock) else {
                return;
            };
            let sprite = if grey { greyed(sprite) } else { sprite };
            draw_sprite(buf, &sprite, x, y - sprite.body_row as i32, clip);
        }
        Goods::Part(part) => {
            let color = if grey { DARK_GRAY } else { part.paint.color() };
            put(buf, x, y, CAPSULE, bold(color), clip);
        }
    }
}

fn greyed(sprite: LineSprite) -> LineSprite {
    LineSprite {
        rows: sprite
            .rows
            .into_iter()
            .map(|row| row.into_iter().map(|(ch, _)| (ch, DARK_GRAY)).collect())
            .collect(),
        body_row: sprite.body_row,
    }
}

fn draw_hand(buf: &mut Buffer, claw: &Claw, origin: (i32, i32), clip: Rect) {
    let hand = &claw.hand;
    let x = claw.shown_x();
    let lean = x - hand.rail;
    for y in RAIL + 1..hand.y {
        let low = lean != 0 && y * 2 > hand.y;
        let (cx, glyph) = match (low, lean.signum()) {
            (true, 1) => (hand.rail + 1, LEAN_RIGHT),
            (true, -1) => (hand.rail - 1, LEAN_LEFT),
            _ => (hand.rail, CABLE),
        };
        put(buf, origin.0 + cx, origin.1 + y, glyph, style(WHITE), clip);
    }
    put(
        buf,
        origin.0 + x - 1,
        origin.1 + hand.y,
        HEAD,
        style(WHITE),
        clip,
    );
    let prongs = if hand.open { OPEN } else { SHUT };
    put(
        buf,
        origin.0 + x - 1,
        origin.1 + hand.y + 1,
        prongs,
        style(WHITE),
        clip,
    );
}

fn status(claw: &Claw, room_for_toys: bool, shelf: &Shelf) -> Vec<(String, Style)> {
    let mut parts = match claw.phase {
        Phase::Steering => vec![(
            format!("{}s", claw.steer_left.ceil() as u32),
            bold(LIGHT_YELLOW),
        )],
        _ => vec![
            (cash(crate::casino::claw::price()), bold(WHITE)),
            (" a go".to_string(), style(DARK_GRAY)),
        ],
    };
    let Some(prize) = claw.hovered(room_for_toys) else {
        return parts;
    };
    let color = match &prize.goods {
        Goods::Toy(fish) => fish
            .toy
            .as_ref()
            .map_or(WHITE, |toy| toy.paints().0.color()),
        Goods::Part(part) => part.paint.color(),
    };
    parts.push(("   ".to_string(), style(WHITE)));
    parts.push((prize.goods.name(), bold(color)));
    parts.push((format!(" {}", pips(prize.goods.worth())), style(WHITE)));
    if prize.goods.toy().is_some_and(|toy| !shelf.holds(toy)) {
        parts.push((format!(" {NEW}"), bold(GOLD)));
    }
    parts
}
