use ratatui::{buffer::Buffer, layout::Rect};

use crate::casino::Teller;
use crate::casino::blackjack::{Blackjack, Card, Move, Phase, total};
use crate::casino::state::Table;
use crate::colors::{CYAN, DARK_GRAY, LIGHT_CYAN, NAVY_LIGHT, RED, WHITE};
use crate::ui::hint_bar::HintBar;
use crate::ui::hints::{
    HINT_DOUBLE_DOWN, HINT_ENTER_DEAL, HINT_ENTER_STAND, HINT_ESC_LEAVE, HINT_HIT, HINT_SPLIT,
};

use super::{
    Room, TOLLOMIND, bet_hints, bold, centred, draw_fish_at, draw_parts_centred, draw_tollomind,
    facing_right, fish_rows, flash_color, open_frame, put, result_text, stake_parts, status_row,
    style, tollomind_size, total_color,
};

const BASIS: (u16, u16) = (64, 23);
const BIG: (u16, u16) = (7, 5);
const MID: (u16, u16) = (5, 3);
const BIG_FROM: (u16, u16) = (40, 15);
const MID_FROM: (u16, u16) = (26, 9);
const CARD_GAP: u16 = 1;
const SEAT_GAP: u16 = 3;
const SHOWN_AFTER: f32 = 0.12;
const PAYS: &str = "blackjack pays 3 to 2";
const DEALING: &str = "Tollomind is dealing";
const WIDEST_DEALER_TOTAL: &str = " 21+";
const WIDEST_SEAT_TOTAL: &str = " 21";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Size {
    Big,
    Mid,
    Tiny,
}

impl Size {
    fn of(art: Rect) -> Size {
        if art.width >= BIG_FROM.0 && art.height >= BIG_FROM.1 {
            Size::Big
        } else if art.width >= MID_FROM.0 && art.height >= MID_FROM.1 {
            Size::Mid
        } else {
            Size::Tiny
        }
    }

    fn card(self) -> (u16, u16) {
        match self {
            Size::Big => BIG,
            Size::Mid | Size::Tiny => MID,
        }
    }
}

pub fn draw(buf: &mut Buffer, room: &Room, table: &Table, game: Option<&Blackjack>) {
    let hints = hints(table, room.teller, game);
    let chrome = open_frame(
        buf,
        room.screen,
        "Blackjack",
        BASIS,
        flash_color(table.flash),
        &hints,
        super::widest_bet(table, HINT_ENTER_DEAL, &[]),
    );
    status_row(
        buf,
        chrome.status,
        &stake_parts(table, room.teller),
        room.purse,
    );
    let art = chrome.art;
    let seat = seat(table, room.teller);
    match Size::of(art) {
        Size::Tiny => draw_tiny(buf, art, table, game, seat.as_ref()),
        size => draw_felt(buf, art, size, table, game, seat.as_ref()),
    }
}

fn hints(table: &Table, teller: &dyn Teller, game: Option<&Blackjack>) -> HintBar {
    let Some(game) = game.filter(|_| table.round.is_some()) else {
        return bet_hints(table, teller, HINT_ENTER_DEAL, &[]);
    };
    let bar = HintBar::new(HINT_ESC_LEAVE);
    if game.phase != Phase::Player {
        return bar.action(DEALING);
    }
    bar.action(HINT_HIT)
        .action(HINT_ENTER_STAND)
        .action_if(game.can(Move::Double), HINT_DOUBLE_DOWN)
        .action_if(game.can(Move::Split), HINT_SPLIT)
}

fn seat(table: &Table, teller: &dyn Teller) -> Option<crate::fishes::fish::Fish> {
    let name = table
        .fish_in_play()
        .or(table.seat.fish())
        .or(table.result.as_ref().and_then(|r| r.fish()))?;
    teller
        .entrants()
        .into_iter()
        .find(|e| e.name == name)
        .map(|e| facing_right(&e.portrait))
}

fn seat_name(seat: Option<&crate::fishes::fish::Fish>) -> String {
    seat.map_or_else(|| "You".to_string(), |fish| fish.name.clone())
}

fn draw_felt(
    buf: &mut Buffer,
    art: Rect,
    size: Size,
    table: &Table,
    game: Option<&Blackjack>,
    seat: Option<&crate::fishes::fish::Fish>,
) {
    let (_, card_h) = size.card();
    let (tw, th) = tollomind_size();
    let dealer_label = dealer_label(game);
    let left_w = tw.max(widest_dealer_label()) + SEAT_GAP;
    let top = art.y + u16::from(size == Size::Big);
    draw_tollomind(buf, art.x as i32 + 1, top as i32, art);
    let label_y = top + th;
    let dealer_color = match game {
        Some(g)
            if matches!(g.phase, Phase::Finished { .. }) && total(&g.dealer, true).points > 21 =>
        {
            crate::colors::LIGHT_GREEN
        }
        _ => WHITE,
    };
    put(
        buf,
        art.x as i32 + 1,
        label_y as i32,
        &dealer_label,
        bold(dealer_color),
        art,
    );
    if let Some(game) = game {
        draw_hand(
            buf,
            art,
            art.x + left_w,
            top,
            &game.dealer,
            size,
            art.right(),
        );
    }

    let seat_rows = seat.map_or(1, |fish| fish_rows(fish) + 1);
    let band = card_h.max(seat_rows);
    let bottom = art.bottom().saturating_sub(u16::from(size == Size::Big));
    let band_top = bottom.saturating_sub(band);
    let name_y = band_top + band - 1;
    let name = seat_name(seat);
    let label_w = seat_label_width(&name);
    let seat_w = seat
        .map_or(0, |fish| fish.display_width as u16)
        .max(label_w);
    if let Some(fish) = seat {
        let fish_top = name_y.saturating_sub(fish_rows(fish));
        draw_fish_at(buf, fish, centred_over(art, label_w, fish), fish_top, art);
    }
    let cards_x = art.x + seat_w.max(left_w);
    let card_top = band_top + band.saturating_sub(card_h);
    let hands = game.map_or(&[][..], |g| &g.hands[..]);
    let mut name_line = vec![(name.clone(), bold(WHITE))];
    if hands.len() == 1 && !hands[0].cards.is_empty() {
        let t = hands[0].total();
        name_line.push((
            format!(" {}", t.points),
            bold(total_color(t.points, hands[0].done)),
        ));
    }
    let mut x = art.x as i32 + 1;
    for (text, st) in &name_line {
        x += put(buf, x, name_y as i32, text, *st, art) as i32;
    }
    let room_w = art.right().saturating_sub(cards_x);
    let hand_w = if hands.len() > 1 {
        room_w / hands.len() as u16
    } else {
        room_w
    };
    for (i, hand) in hands.iter().enumerate() {
        let hx = cards_x + hand_w * i as u16;
        draw_hand(
            buf,
            art,
            hx,
            card_top,
            &hand.cards,
            size,
            hx + hand_w.saturating_sub(1),
        );
        if hands.len() > 1 {
            let active = game.is_some_and(|g| g.active == i && g.phase == Phase::Player);
            let t = hand.total();
            let mark = if active { "▸ " } else { "  " };
            let outcome = hand
                .outcome
                .map(|o| format!(" {}", o.label()))
                .unwrap_or_default();
            let line = format!("{mark}{}{outcome}", t.points);
            put(
                buf,
                hx as i32,
                card_top as i32 - 1,
                &line,
                bold(total_color(t.points, hand.done)),
                art,
            );
        }
    }

    let middle = (label_y + 1 + band_top) / 2;
    if let Some(parts) = result_text(table) {
        draw_parts_centred(buf, art, middle, &parts);
    } else if game.is_none() || table.round.is_none() {
        centred(buf, art, middle, PAYS, style(DARK_GRAY));
    }
}

fn seat_label_width(name: &str) -> u16 {
    (name.chars().count() + WIDEST_SEAT_TOTAL.len()) as u16
}

fn centred_over(art: Rect, label_w: u16, fish: &crate::fishes::fish::Fish) -> u16 {
    art.x + 1 + label_w.saturating_sub(fish.display_width as u16) / 2
}

fn widest_dealer_label() -> u16 {
    (TOLLOMIND.len() + WIDEST_DEALER_TOTAL.len()) as u16
}

fn dealer_label(game: Option<&Blackjack>) -> String {
    let Some(game) = game.filter(|g| !g.dealer.is_empty()) else {
        return TOLLOMIND.to_string();
    };
    let hidden = game.dealer.iter().any(|c| !c.face_up);
    let points = total(&game.dealer, false).points;
    if hidden {
        return format!("{TOLLOMIND} {points}+");
    }
    format!("{TOLLOMIND} {points}")
}

fn draw_hand(buf: &mut Buffer, art: Rect, x: u16, y: u16, cards: &[Card], size: Size, right: u16) {
    let (card_w, _) = size.card();
    let count = cards.len() as u16;
    if count == 0 {
        return;
    }
    let room = right.saturating_sub(x);
    let stride = if count > 1 {
        ((room.saturating_sub(card_w)) / (count - 1)).clamp(3, card_w + CARD_GAP)
    } else {
        card_w
    };
    for (i, card) in cards.iter().enumerate() {
        draw_card(buf, art, x + stride * i as u16, y, card, size);
    }
}

fn shown(card: &Card) -> bool {
    card.face_up && card.age >= SHOWN_AFTER
}

fn suit_color(card: &Card) -> ratatui::style::Color {
    if card.suit.is_red() { RED } else { WHITE }
}

fn draw_card(buf: &mut Buffer, art: Rect, x: u16, y: u16, card: &Card, size: Size) {
    let (w, h) = size.card();
    let rect = Rect::new(x, y, w, h).intersection(art);
    if rect.width < 2 || rect.height < 2 {
        return;
    }
    crate::ui::modal::clear(buf, rect, super::BACKGROUND);
    let face = shown(card);
    let border = if face { WHITE } else { NAVY_LIGHT };
    crate::ui::table::draw_box_border(
        buf,
        Rect::new(x, y, w, h).intersection(*buf.area()),
        "",
        border,
        super::BACKGROUND,
    );
    let (xi, yi) = (x as i32, y as i32);
    let rank = card.rank.label();
    let suit = card.suit.glyph().to_string();
    match (size, face) {
        (Size::Big, true) => {
            let c = suit_color(card);
            put(buf, xi + 1, yi + 1, &rank, bold(c), art);
            put(buf, xi + 3, yi + 2, &suit, bold(c), art);
            put(
                buf,
                xi + w as i32 - 1 - rank.len() as i32,
                yi + 3,
                &rank,
                bold(c),
                art,
            );
        }
        (Size::Big, false) => {
            put(buf, xi + 1, yi + 1, "≈≈≈≈≈", style(CYAN), art);
            put(buf, xi + 1, yi + 2, "≈><>≈", style(LIGHT_CYAN), art);
            put(buf, xi + 1, yi + 3, "≈≈≈≈≈", style(CYAN), art);
        }
        (_, true) => {
            let c = suit_color(card);
            let rx = if rank.len() == 2 { 1 } else { 2 };
            put(buf, xi + rx, yi + 1, &rank, bold(c), art);
            put(buf, xi + 3, yi + 1, &suit, bold(c), art);
        }
        (_, false) => {
            put(buf, xi + 1, yi + 1, "><>", style(LIGHT_CYAN), art);
        }
    }
}

fn draw_tiny(
    buf: &mut Buffer,
    art: Rect,
    table: &Table,
    game: Option<&Blackjack>,
    seat: Option<&crate::fishes::fish::Fish>,
) {
    let line = |buf: &mut Buffer,
                y: u16,
                label: &str,
                cards: &[Card],
                total_text: Option<(String, ratatui::style::Color)>| {
        let mut x = art.x as i32 + 1;
        x += put(buf, x, y as i32, label, bold(WHITE), art) as i32 + 1;
        for card in cards {
            if shown(card) {
                let text = format!("{}{}", card.rank.label(), card.suit.glyph());
                x += put(buf, x, y as i32, &text, bold(suit_color(card)), art) as i32 + 1;
            } else {
                x += put(buf, x, y as i32, "▒▒", style(NAVY_LIGHT), art) as i32 + 1;
            }
        }
        if let Some((text, color)) = total_text {
            let tx = art.right() as i32 - 1 - text.len() as i32;
            put(buf, tx, y as i32, &text, bold(color), art);
        }
    };
    let dealer = game.map_or(&[][..], |g| &g.dealer[..]);
    let dealer_total = game.filter(|g| !g.dealer.is_empty()).map(|g| {
        let hidden = g.dealer.iter().any(|c| !c.face_up);
        let points = total(&g.dealer, false).points;
        (
            if hidden {
                format!("{points}+")
            } else {
                points.to_string()
            },
            WHITE,
        )
    });
    line(buf, art.y, TOLLOMIND, dealer, dealer_total);
    let name = seat_name(seat);
    let hands = game.map_or(&[][..], |g| &g.hands[..]);
    let rows = hands.len().max(1) as u16;
    let first = art.bottom().saturating_sub(rows).max(art.y + 1);
    if let Some(fish) = seat {
        let fish_h = fish_rows(fish);
        if first >= art.y + 2 + fish_h {
            let name_w = name.chars().count() as u16;
            draw_fish_at(
                buf,
                fish,
                centred_over(art, name_w, fish),
                first - fish_h,
                art,
            );
        }
    }
    if hands.is_empty() {
        line(buf, first, &name, &[], None);
    }
    for (i, hand) in hands.iter().enumerate() {
        let t = hand.total();
        let mark = if hands.len() > 1 && game.is_some_and(|g| g.active == i) {
            "▸"
        } else {
            ""
        };
        line(
            buf,
            first + i as u16,
            &format!("{mark}{name}"),
            &hand.cards,
            (!hand.cards.is_empty())
                .then(|| (t.points.to_string(), total_color(t.points, hand.done))),
        );
    }
    if art.height >= 3 {
        let middle = art.y + (first - art.y) / 2;
        if let Some(parts) = result_text(table) {
            draw_parts_centred(buf, art, middle.max(art.y + 1), &parts);
        } else if table.round.is_none() {
            centred(buf, art, middle.max(art.y + 1), PAYS, style(DARK_GRAY));
        }
    }
}
