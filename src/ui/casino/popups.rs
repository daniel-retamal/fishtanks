use std::sync::OnceLock;

use rand::{SeedableRng, rngs::SmallRng};
use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

use crate::casino::bubble::Risk;
use crate::casino::flip::{Flip, FlipPhase, Landing, Side, Swim};
use crate::casino::spins::{self, PEARLS_TO_DIVE, Symbol};
use crate::casino::state::{Banner, Picker, Play, Popup, PrizeCard, Purpose, Table};
use crate::casino::{Multiple, premium};
use crate::colors::{DARK_GRAY, GOLD, LIGHT_GREEN, LIGHT_RED, LIGHT_YELLOW, WHITE};
use crate::fishes::fish::{Fish, LineSprite};
use crate::fishes::species::FishSpecies;
use crate::loot::LootKind;
use crate::ui::catch_overlay::{CatchOverlay, CatchState};
use crate::ui::hint_bar::HintBar;
use crate::ui::hints::{
    HINT_CALL_IT, HINT_CANCEL, HINT_CLOSE, HINT_DOUBLE_OR_NOTHING, HINT_ENTER_BAIT,
    HINT_ENTER_COLLECT, HINT_ENTER_LEAVE, HINT_ENTER_STAKE, HINT_ESC_STAY, HINT_FLIP_AGAIN,
    HINT_NAV, HINT_SCROLL,
};
use crate::ui::modal::{Frame, Modal};
use crate::ui::text_input::TextInput;

use super::{
    BACKGROUND, BIG_ROWS, Room, TOLLOMIND, big_fits, big_width, bold, cash, cash_within, centred,
    draw_big, draw_sprite, draw_tollomind, draw_tollomind_facing_left, glisten, mirrored,
    multiple_color, put, sprite_width, style, tier, tollomind, tollomind_size,
};

const PICKER_W: u16 = 54;
const MONEY_W: usize = 10;
const NOTE_FROM_ROWS: u16 = 18;
const TWO_COLUMNS_FROM: u16 = 40;
const FLIP_STAGE: (u16, u16) = (40, 10);
const LADDER_W: u16 = 22;
const LADDER_FROM_W: u16 = 56;
const FLIP_FRAME_SECS: f32 = 0.09;
const BANNER_BODY: (u16, u16) = (48, 7);
const COINS: usize = 9;
const COIN_GLYPHS: [&str; 2] = ["o", "°"];
const BIG_MARGIN: u16 = 2;
const COIN_SPEED: f32 = 0.7;
const CONFIRM_W: u16 = 40;
const GOLDFISH_SEED: u64 = 7;
const LAST_BUBBLE: &str = "°";
const STAKE_NOTE: &str = "it comes home unless it loses; then Tollomind eats it";

pub fn draw(buf: &mut Buffer, room: &Room, popup: &Popup, table: Option<&Table>) {
    match popup {
        Popup::Picker(picker) => draw_picker(buf, room, picker),
        Popup::Flip(flip) => draw_flip(buf, room, flip),
        Popup::Banner(banner) => draw_banner(buf, room, banner),
        Popup::Confirm => draw_confirm(buf, room, table),
        Popup::Paytable(scroll) => draw_paytable(buf, room, table, *scroll),
        Popup::Prize(card) => draw_prize(buf, room, card),
    }
}

fn picker_title(purpose: Purpose) -> &'static str {
    match purpose {
        Purpose::Stake => " Fishes to the Table! ",
        Purpose::Bait => " Bait to the Net! ",
    }
}

fn draw_picker(buf: &mut Buffer, room: &Room, picker: &Picker) {
    let action = match picker.purpose {
        Purpose::Stake => HINT_ENTER_STAKE,
        Purpose::Bait => HINT_ENTER_BAIT,
    };
    let rows = picker.rows.len();
    let hints = |overflowing: bool| {
        let bar = HintBar::new(HINT_CANCEL)
            .counted(HINT_NAV, overflowing.then_some((picker.selected + 1, rows)))
            .action(action);
        if picker.purpose == Purpose::Stake && room.screen.whole.height >= NOTE_FROM_ROWS {
            bar.aside(STAKE_NOTE)
        } else {
            bar
        }
    };
    let frame = Frame {
        title: picker_title(picker.purpose),
        border: WHITE,
        background: BACKGROUND,
    };
    let (modal, _) =
        Modal::open_fitting(buf, room.screen, &frame, (PICKER_W, rows as u16 + 1), hints);
    let body = modal.body;
    let stake = picker.purpose == Purpose::Stake;
    let both = stake && body.width >= TWO_COLUMNS_FROM;
    let money_x = body.right() as i32 - 2;
    let second_x = money_x - MONEY_W as i32 - 1;
    let header = u16::from(body.height >= 2);
    if header > 0 {
        put(
            buf,
            body.x as i32 + 2,
            body.y as i32,
            "Fish",
            bold(WHITE),
            body,
        );
        let head = if stake { "Plays For" } else { "Worth" };
        put(
            buf,
            money_x - head.len() as i32 + 1,
            body.y as i32,
            head,
            bold(WHITE),
            body,
        );
        if both {
            put(buf, second_x - 4, body.y as i32, "Worth", bold(WHITE), body);
        }
    }
    let room_rows = body.height.saturating_sub(header).max(1) as usize;
    let start = picker.selected.saturating_sub(room_rows.saturating_sub(1));
    for (row, entrant) in picker.rows.iter().enumerate().skip(start).take(room_rows) {
        let y = body.y as i32 + header as i32 + (row - start) as i32;
        let usable = picker.is_usable(row);
        let chosen = row == picker.selected;
        let color = if usable { WHITE } else { DARK_GRAY };
        put(
            buf,
            body.x as i32,
            y,
            if chosen { "> " } else { "  " },
            bold(WHITE),
            body,
        );
        let species = entrant.portrait.species.display_name();
        let label = if entrant.name == species {
            entrant.name.clone()
        } else {
            format!("{} ({species})", entrant.name)
        };
        let column_end = if both {
            second_x - MONEY_W as i32
        } else {
            money_x - MONEY_W as i32
        };
        let room_w = (column_end - body.x as i32 - 3).max(1) as usize;
        let label = crate::ui::table::ellipsize(&label, room_w);
        put(
            buf,
            body.x as i32 + 2,
            y,
            &label,
            if chosen { bold(color) } else { style(color) },
            body,
        );
        let worth = if entrant.stakeable {
            cash_within(entrant.worth, MONEY_W)
        } else {
            "unsellable".to_string()
        };
        if stake {
            let plays = if entrant.stakeable {
                cash_within(premium(entrant.worth), MONEY_W)
            } else {
                crate::ui::table::NOTHING.to_string()
            };
            put(
                buf,
                money_x - plays.chars().count() as i32 + 1,
                y,
                &plays,
                style(color),
                body,
            );
            if both {
                put(
                    buf,
                    second_x - worth.chars().count() as i32 + 1,
                    y,
                    &worth,
                    style(color),
                    body,
                );
            }
        } else {
            put(
                buf,
                money_x - worth.chars().count() as i32 + 1,
                y,
                &worth,
                style(color),
                body,
            );
        }
    }
}

fn goldfish() -> &'static LineSprite {
    static SPRITE: OnceLock<LineSprite> = OnceLock::new();
    SPRITE.get_or_init(|| {
        let mut rng = SmallRng::seed_from_u64(GOLDFISH_SEED);
        let mut fish = Fish::new_for_display(FishSpecies::Goldfish, &mut rng);
        fish.facing = crate::fishes::fish::Direction::Right;
        fish.line_sprite()
    })
}

fn draw_flip(buf: &mut Buffer, room: &Room, flip: &Flip) {
    let won = matches!(flip.phase, FlipPhase::Won { .. });
    let calling = matches!(flip.phase, FlipPhase::Calling);
    let can_call = flip.can_call(room.teller);
    let hints = HintBar::new("")
        .action_if(
            calling || (won && can_call),
            if calling {
                HINT_CALL_IT
            } else {
                HINT_FLIP_AGAIN
            },
        )
        .action_if(flip.can_collect(), HINT_ENTER_COLLECT);
    let border = match flip.phase {
        FlipPhase::Lost { .. } => LIGHT_RED,
        FlipPhase::Won { .. } => LIGHT_GREEN,
        _ => WHITE,
    };
    let frame = Frame {
        title: " Double or Nothing ",
        border,
        background: BACKGROUND,
    };
    let wide = room.screen.whole.width >= LADDER_FROM_W;
    let content_w = FLIP_STAGE.0 + if wide { LADDER_W } else { 0 };
    let (modal, _) =
        Modal::open_fitting(buf, room.screen, &frame, (content_w, FLIP_STAGE.1), |_| {
            hints.clone()
        });
    let body = modal.body;
    let ladder_w = if wide && body.width > LADDER_W + 10 {
        LADDER_W
    } else {
        0
    };
    let stage = Rect::new(body.x, body.y, body.width - ladder_w, body.height);
    rule_the_flip(buf, &modal, (ladder_w > 0).then_some(stage.right()), border);
    if ladder_w > 0 {
        draw_ladder(
            buf,
            Rect::new(stage.right(), body.y, ladder_w, body.height),
            flip,
        );
    } else {
        let line = format!(
            "On the line {}",
            on_the_line(flip.line.cash, flip.line.fish.len())
        );
        put(
            buf,
            stage.x as i32 + 1,
            stage.bottom() as i32 - 1,
            &line,
            bold(LIGHT_YELLOW),
            stage,
        );
    }
    let message = flip_message(flip);
    centred(buf, stage, stage.y, &message.0, bold(message.1));
    let rows = goldfish().rows.len() as u16;
    let width = sprite_width(goldfish());
    let floor = stage.bottom().saturating_sub(2 + u16::from(ladder_w == 0));
    let rest = floor.saturating_sub(rows - 1);
    let (lift, frame_index, landing) = match flip.phase {
        FlipPhase::Flying { t, .. } => {
            let arc = (t / crate::casino::flip::FLIGHT_SECS * std::f32::consts::PI).sin();
            let height = rest.saturating_sub(stage.y + 2) as f32;
            (
                (arc * height).round() as u16,
                (t / FLIP_FRAME_SECS) as usize % 4,
                None,
            )
        }
        FlipPhase::Won { landing } | FlipPhase::Lost { landing, .. } => (0, 0, Some(landing)),
        FlipPhase::Calling => (0, 0, None),
    };
    let top = rest.saturating_sub(lift);
    let x = stage.x + stage.width.saturating_sub(width) / 2;
    let face_left = matches!(landing, Some(Landing::Facing(Side::Left))) || frame_index == 2;
    let edge_on = frame_index % 2 == 1;
    let body_y = top + goldfish().body_row as u16;
    let swim = flip.tollomind();
    let eaten = swim.is_some_and(|swim| swim.has_passed(stage, (x + width / 2) as i32));
    if eaten {
        put(
            buf,
            (x + width / 2) as i32,
            body_y as i32 - 1,
            LAST_BUBBLE,
            style(DARK_GRAY),
            stage,
        );
    } else if edge_on {
        for row in 0..rows {
            put(
                buf,
                (x + width / 2) as i32,
                (top + row) as i32,
                "|",
                bold(GOLD),
                stage,
            );
        }
    } else if matches!(landing, Some(Landing::BellyUp)) {
        let grey = LineSprite {
            rows: goldfish()
                .rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|&(c, _)| (if c == 'º' { 'x' } else { c }, DARK_GRAY))
                        .collect()
                })
                .collect(),
            body_row: goldfish().body_row,
        };
        draw_sprite(buf, &grey, x as i32, top as i32, stage);
    } else {
        let sprite = if face_left {
            mirrored(goldfish())
        } else {
            goldfish().clone()
        };
        draw_sprite(buf, &sprite, x as i32, top as i32, stage);
    }
    let labels_y = stage.bottom().saturating_sub(1 + u16::from(ladder_w == 0));
    let call = match flip.phase {
        FlipPhase::Flying { call, .. } => Some(call),
        _ => None,
    };
    let left_style = if call == Some(Side::Left) {
        bold(WHITE)
    } else {
        style(DARK_GRAY)
    };
    let right_style = if call == Some(Side::Right) {
        bold(WHITE)
    } else {
        style(DARK_GRAY)
    };
    put(
        buf,
        stage.x as i32 + 1,
        labels_y as i32,
        "← left",
        left_style,
        stage,
    );
    put(
        buf,
        stage.right() as i32 - 9,
        labels_y as i32,
        "right →",
        right_style,
        stage,
    );
    if let Some(swim) = swim {
        draw_swim(buf, stage, swim, body_y);
    }
}

trait Swimming {
    fn left_edge(&self, stage: Rect) -> i32;
    fn has_passed(&self, stage: Rect, x: i32) -> bool;
}

impl Swimming for Swim {
    fn left_edge(&self, stage: Rect) -> i32 {
        let width = tollomind_size().0 as i32;
        let travelled = (self.swum * (stage.width as i32 + width) as f32).round() as i32;
        match self.from {
            Side::Left => stage.x as i32 - width + travelled,
            Side::Right => stage.right() as i32 - travelled,
        }
    }

    fn has_passed(&self, stage: Rect, x: i32) -> bool {
        let left = self.left_edge(stage);
        match self.from {
            Side::Left => left + tollomind_size().0 as i32 > x,
            Side::Right => left <= x,
        }
    }
}

fn draw_swim(buf: &mut Buffer, stage: Rect, swim: Swim, body_y: u16) {
    let x = swim.left_edge(stage);
    let top = body_y as i32 - tollomind().body_row as i32;
    match swim.from {
        Side::Left => draw_tollomind(buf, x, top, stage),
        Side::Right => draw_tollomind_facing_left(buf, x, top, stage),
    };
}

fn rule_the_flip(buf: &mut Buffer, modal: &Modal, column: Option<u16>, border: Color) {
    let rule_y = modal.hints.y.saturating_sub(1);
    if modal.hints.height == 0 || modal.body.bottom() > rule_y {
        return;
    }
    let rect = modal.rect;
    crate::ui::table::draw_box_separator(buf, rect.x, rule_y, rect.width, &[], border, BACKGROUND);
    let Some(x) = column else {
        return;
    };
    let line = ratatui::style::Style::default().fg(border).bg(BACKGROUND);
    if buf[(x, rect.y)].symbol() == "─" {
        buf[(x, rect.y)].set_char('┬').set_style(line);
    }
    for y in modal.body.y..rule_y {
        buf[(x, y)].set_char('│').set_style(line);
    }
    buf[(x, rule_y)].set_char('┴').set_style(line);
}

fn on_the_line(cash_amount: crate::economy::Money, fish: usize) -> String {
    match (cash_amount, fish) {
        (0, n) => format!("{n} fish"),
        (c, 0) => cash(c),
        (c, n) => format!("{} + {n} fish", cash_within(c, 10)),
    }
}

fn flip_message(flip: &Flip) -> (String, ratatui::style::Color) {
    match flip.phase {
        FlipPhase::Calling if flip.rung == 0 => ("Call the flip".to_string(), WHITE),
        FlipPhase::Calling => ("Call it again".to_string(), WHITE),
        FlipPhase::Flying { .. } => ("...".to_string(), LIGHT_YELLOW),
        FlipPhase::Won { .. } => (
            format!(
                "{}  {}",
                Multiple::whole(flip.multiple()).label(),
                on_the_line(flip.line.cash, flip.line.fish.len())
            ),
            LIGHT_GREEN,
        ),
        FlipPhase::Lost { landing, .. } => (
            if landing == Landing::BellyUp {
                "Belly up"
            } else {
                "Wrong side"
            }
            .to_string(),
            LIGHT_RED,
        ),
    }
}

fn draw_ladder(buf: &mut Buffer, area: Rect, flip: &Flip) {
    let rows = area.height as u32;
    let lost = matches!(flip.phase, FlipPhase::Lost { .. });
    let next = flip.rung + 1;
    let lowest = next.saturating_sub(rows / 2).max(1);
    for row in 0..rows {
        let rung = lowest + (rows - 1 - row);
        let y = area.y as i32 + row as i32;
        let multiple = Multiple::whole(crate::economy::Money::from(2u8).saturating_pow(rung));
        let done = rung <= flip.rung && !lost;
        let color = if done {
            LIGHT_GREEN
        } else if rung == next && !lost {
            LIGHT_YELLOW
        } else {
            DARK_GRAY
        };
        let mark = if rung == next && !lost { "▸" } else { " " };
        let label = format!("{mark}{}", multiple.label());
        put(
            buf,
            area.x as i32 + 2,
            y,
            &label,
            if rung == next {
                bold(color)
            } else {
                style(color)
            },
            area,
        );
        let amount = flip
            .base
            .saturating_mul(crate::economy::Money::from(2u8).saturating_pow(rung));
        let fish = flip.base_fish << rung.min(60);
        let text = if flip.base == 0 {
            format!("{fish} fish")
        } else {
            cash_within(amount, 8)
        };
        put(
            buf,
            area.right() as i32 - 1 - text.chars().count() as i32,
            y,
            &text,
            style(color),
            area,
        );
    }
}

fn draw_banner(buf: &mut Buffer, room: &Room, banner: &Banner) {
    let tier = tier(banner.multiple, banner.jackpot);
    let border = if tier.glistens {
        glisten(room.clock, 0)
    } else {
        tier.color
    };
    let can_double = banner
        .line
        .as_ref()
        .is_some_and(|line| line.can_double(room.teller));
    let hints = HintBar::new("")
        .action(HINT_ENTER_COLLECT)
        .action_if(can_double, HINT_DOUBLE_OR_NOTHING);
    let title = format!(" {} ", tier.name);
    let frame = Frame {
        title: &title,
        border,
        background: BACKGROUND,
    };
    let (modal, _) = Modal::open_fitting(buf, room.screen, &frame, BANNER_BODY, |_| hints.clone());
    let body = modal.body;
    for k in 0..COINS {
        let column = (k * 37 + 11) % body.width.max(1) as usize;
        let rise = (room.clock * COIN_SPEED + k as f32 * 0.37).fract();
        let y = body.bottom() as i32 - 1 - (rise * body.height as f32) as i32;
        let coin = COIN_GLYPHS[k % COIN_GLYPHS.len()];
        put(
            buf,
            body.x as i32 + column as i32,
            y,
            coin,
            style(GOLD),
            body,
        );
    }
    let shown = format!("+{}", cash(banner.shown_amount()));
    let big =
        big_fits(&shown) && big_width(&shown) + 2 <= body.width && body.height >= BIG_ROWS + 2;
    let color = |i: usize| {
        if tier.glistens {
            glisten(room.clock, i)
        } else {
            tier.color
        }
    };
    if big {
        let clear = Rect::new(
            body.x
                + body
                    .width
                    .saturating_sub(big_width(&shown) + BIG_MARGIN * 2)
                    / 2,
            body.y,
            big_width(&shown) + BIG_MARGIN * 2,
            BIG_ROWS,
        )
        .intersection(body);
        crate::ui::modal::clear(buf, clear, BACKGROUND);
        draw_big(buf, body, body.y, &shown, color);
    } else {
        let text = format!(
            "+{}",
            cash_within(banner.shown_amount(), body.width as usize - 1)
        );
        centred(buf, body, body.y, &text, bold(color(0)));
    }
    let sub = if banner.label.contains('×') {
        banner.label.clone()
    } else {
        format!("{}  {}", banner.multiple.label(), banner.label)
    };
    let y = body.y + if big { BIG_ROWS + 1 } else { 2 };
    if y < body.bottom() {
        centred(buf, body, y, &sub, style(WHITE));
    }
}

fn draw_confirm(buf: &mut Buffer, room: &Room, table: Option<&Table>) {
    let Some(table) = table else {
        return;
    };
    let hints = HintBar::new(HINT_ESC_STAY).action(HINT_ENTER_LEAVE);
    let title = format!(" {} ", table.game.name());
    let frame = Frame {
        title: &title,
        border: WHITE,
        background: BACKGROUND,
    };
    let (modal, _) =
        Modal::open_fitting(buf, room.screen, &frame, (CONFIRM_W, 2), |_| hints.clone());
    let body = modal.body;
    put(
        buf,
        body.x as i32 + 1,
        body.y as i32,
        "Leave the table?",
        bold(WHITE),
        body,
    );
    let at_risk = table.at_risk();
    let line = match at_risk.first() {
        Some(name) => format!("{name} stays with {TOLLOMIND}."),
        None => match &table.round {
            Some(round) => format!("Your {} stays on it.", cash(round.on_the_line())),
            None => "What is in the air stays on the table.".to_string(),
        },
    };
    put(
        buf,
        body.x as i32 + 1,
        body.y as i32 + 1,
        &line,
        style(LIGHT_YELLOW),
        body,
    );
}

struct Line {
    left: Vec<(String, ratatui::style::Style)>,
    right: Option<(String, ratatui::style::Style)>,
}

impl Line {
    fn text(text: impl Into<String>, st: ratatui::style::Style) -> Line {
        Line {
            left: vec![(text.into(), st)],
            right: None,
        }
    }

    fn priced(
        left: Vec<(String, ratatui::style::Style)>,
        right: String,
        st: ratatui::style::Style,
    ) -> Line {
        Line {
            left,
            right: Some((right, st)),
        }
    }
}

fn draw_lines(
    buf: &mut Buffer,
    room: &Room,
    title: &str,
    width: u16,
    lines: &[Line],
    scroll: usize,
) {
    let frame = Frame {
        title,
        border: WHITE,
        background: BACKGROUND,
    };
    let total = lines.len();
    let (modal, _) = Modal::open_fitting(
        buf,
        room.screen,
        &frame,
        (width, total as u16),
        |overflowing| {
            HintBar::new(HINT_CLOSE)
                .counted(HINT_SCROLL, overflowing.then_some((scroll + 1, total)))
        },
    );
    let body = modal.body;
    let room_rows = body.height as usize;
    let first = scroll.min(total.saturating_sub(room_rows));
    for (row, line) in lines.iter().skip(first).take(room_rows).enumerate() {
        let y = body.y as i32 + row as i32;
        let mut x = body.x as i32 + 1;
        for (text, st) in &line.left {
            x += put(buf, x, y, text, *st, body) as i32;
        }
        if let Some((text, st)) = &line.right {
            put(
                buf,
                body.right() as i32 - 1 - text.chars().count() as i32,
                y,
                text,
                *st,
                body,
            );
        }
    }
}

fn draw_paytable(buf: &mut Buffer, room: &Room, table: Option<&Table>, scroll: usize) {
    match table.map(|t| &t.play) {
        Some(Play::BubbleUp { risk, .. }) => draw_shells(buf, room, *risk, scroll),
        _ => draw_reels(buf, room, scroll),
    }
}

fn draw_reels(buf: &mut Buffer, room: &Room, scroll: usize) {
    let mut symbols: Vec<Symbol> = Symbol::ALL
        .into_iter()
        .filter(|s| s.three().is_some())
        .collect();
    symbols.sort_by_key(|s| std::cmp::Reverse(s.three()));
    let (_, pearl, _) = super::spins::glyphs(Symbol::Pearl);
    let mut lines = vec![
        Line::text(
            format!("{PEARLS_TO_DIVE} {pearl} anywhere: Pearl Dive"),
            bold(GOLD),
        ),
        Line::text(
            format!(
                "Pays back {:.1}%, the Pot included",
                (spins::pays_back() + crate::casino::POT_SHARE_PER_CENT as f64 / 100.0) * 100.0
            ),
            style(DARK_GRAY),
        ),
    ];
    for symbol in &symbols {
        let (_, short, color) = super::spins::glyphs(*symbol);
        lines.push(Line::priced(
            vec![
                (format!("{short} {short} {short}  "), bold(color)),
                (symbol.name().to_string(), style(DARK_GRAY)),
            ],
            symbol.three().map(Multiple::label).unwrap_or_default(),
            style(WHITE),
        ));
    }
    for (label, pays) in [
        ("any 3 fish", Multiple::whole(3)),
        ("2 Anchoveta", Multiple::whole(2)),
        ("1 Anchoveta", Multiple::ONE),
    ] {
        lines.push(Line::priced(
            vec![(label.to_string(), style(WHITE))],
            pays.label(),
            style(WHITE),
        ));
    }
    draw_lines(buf, room, " Spins#paytable ", 44, &lines, scroll);
}

fn draw_shells(buf: &mut Buffer, room: &Room, risk: Risk, scroll: usize) {
    let shells = risk.shells();
    let half = shells.len().div_ceil(2);
    let mut lines = vec![
        Line::text(
            format!(
                "{}: pays back {:.1}%",
                risk.name(),
                risk.pays_back() * 100.0
            ),
            style(DARK_GRAY),
        ),
        Line::priced(
            vec![("Shell".to_string(), bold(WHITE))],
            "Chance".to_string(),
            bold(WHITE),
        ),
    ];
    for (k, m) in shells.iter().take(half).enumerate() {
        let both = if k == shells.len() - 1 - k { 1.0 } else { 2.0 };
        let chance = risk.chance(k) * both;
        let odds = if chance >= 0.01 {
            format!("{:.0}%", chance * 100.0)
        } else {
            format!(
                "1 in {}",
                crate::economy::grouped((1.0 / chance).round() as u128)
            )
        };
        lines.push(Line::priced(
            vec![(m.label(), bold(multiple_color(*m)))],
            odds,
            style(WHITE),
        ));
    }
    draw_lines(buf, room, " BubbleUp#paytable ", 34, &lines, scroll);
}

fn draw_prize(buf: &mut Buffer, room: &Room, card: &PrizeCard) {
    let mut state = CatchState::holding(
        LootKind::Fish(card.fish.species),
        Some(card.fish.clone()),
        &mut SmallRng::seed_from_u64(GOLDFISH_SEED),
    );
    state.name_input = TextInput::with_value(card.name.as_str().to_string());
    state.cursor_visible = room.cursor_visible;
    CatchOverlay::new(&state, room.screen).render(room.screen.whole, buf);
}
