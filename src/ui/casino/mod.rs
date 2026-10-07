use std::sync::OnceLock;

use rand::{SeedableRng, rngs::SmallRng};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};
use unicode_width::UnicodeWidthChar;

use crate::casino::state::{CasinoState, Flash, View};
use crate::casino::{Casino, Multiple, Teller};
use crate::colors::{
    GOLD, GOLD_BRIGHT, GOLD_PALE, LIGHT_GREEN, LIGHT_RED, LIGHT_YELLOW, WHITE, YELLOW, blended,
};
use crate::economy::{Money, grouped};
use crate::fishes::fish::{Fish, LineSprite};
use crate::fishes::species::FishSpecies;
use crate::sprite::TRANSPARENT;
use crate::ui::command_bar::{metric, scientific};
use crate::ui::hint_bar::HintBar;
use crate::ui::layout::Screen;
use crate::ui::modal::clear;
use crate::ui::{overdraw, table};

mod blackjack;
mod bubble;
mod derby;
mod lobby;
mod net;
mod popups;
mod pufferfish;
mod spins;

pub const BACKGROUND: Color = Color::Reset;
const BORDERS: u16 = 2;
const CHROME_ROWS: u16 = 5;
const SCIENTIFIC_CASH: Money = 1_000_000_000_000_000_000_000;
const GLISTEN_SPEED: f32 = 3.0;
const GLISTEN_STEP: f32 = 0.55;
const FLASH_HZ: f32 = 10.0;
const TOLLOMIND_SEED: u64 = 33;
pub const BOW_TIE: &str = "▸◂";
pub const BOW_TIE_COLOR: Color = GOLD;
pub const TOLLOMIND: &str = "Tollomind";

pub struct CasinoOverlay<'a> {
    state: &'a CasinoState,
    teller: &'a dyn Teller,
    casino: &'a Casino,
    purse: Option<Money>,
    cursor_visible: bool,
    screen: Screen,
}

impl<'a> CasinoOverlay<'a> {
    pub fn new(
        state: &'a CasinoState,
        teller: &'a dyn Teller,
        casino: &'a Casino,
        purse: Option<Money>,
        cursor_visible: bool,
        screen: Screen,
    ) -> Self {
        Self {
            state,
            teller,
            casino,
            purse,
            cursor_visible,
            screen,
        }
    }
}

pub struct Room<'a> {
    pub teller: &'a dyn Teller,
    pub casino: &'a Casino,
    pub purse: Option<Money>,
    pub clock: f32,
    pub screen: Screen,
    pub cursor_visible: bool,
}

impl Widget for CasinoOverlay<'_> {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        let room = Room {
            teller: self.teller,
            casino: self.casino,
            purse: self.purse,
            clock: self.state.clock,
            screen: self.screen,
            cursor_visible: self.cursor_visible,
        };
        match &self.state.view {
            View::Lobby { selected } => lobby::draw(buf, &room, *selected),
            View::Table(table) => match &table.play {
                crate::casino::state::Play::Blackjack(game) => {
                    blackjack::draw(buf, &room, table, game.as_ref())
                }
                crate::casino::state::Play::Spins(spins) => spins::draw(buf, &room, table, spins),
                crate::casino::state::Play::Pufferfish(puffer) => {
                    pufferfish::draw(buf, &room, table, puffer)
                }
                crate::casino::state::Play::BubbleUp { risk, bubbles, lit } => {
                    bubble::draw(buf, &room, table, *risk, bubbles, lit)
                }
                crate::casino::state::Play::Derby(derby) => derby::draw(buf, &room, table, derby),
                crate::casino::state::Play::Net(cast) => net::draw(buf, &room, table, cast),
            },
        }
        if let Some(popup) = &self.state.popup {
            popups::draw(buf, &room, popup, self.state.table());
        }
    }
}

pub fn style(fg: Color) -> Style {
    Style::default().fg(fg).bg(BACKGROUND)
}

pub fn bold(fg: Color) -> Style {
    style(fg).add_modifier(Modifier::BOLD)
}

pub fn cash(amount: Money) -> String {
    if amount >= SCIENTIFIC_CASH {
        return format!("${}", scientific(amount));
    }
    format!("${}", grouped(amount))
}

pub fn cash_within(amount: Money, room: usize) -> String {
    let full = cash(amount);
    if full.chars().count() <= room {
        return full;
    }
    format!("${}", metric(amount))
}

pub fn signed_cash(amount: i128) -> String {
    let sign = if amount < 0 { "-" } else { "+" };
    format!("{sign}{}", cash(amount.unsigned_abs()))
}

pub fn glisten(clock: f32, index: usize) -> Color {
    let wave = ((clock * GLISTEN_SPEED - index as f32 * GLISTEN_STEP).sin() + 1.0) / 2.0;
    blended(GOLD, GOLD_BRIGHT, wave)
}

pub fn flashing(clock: f32) -> bool {
    ((clock * FLASH_HZ) as u32).is_multiple_of(2)
}

pub fn flash_color(flash: Option<(Flash, f32)>) -> Color {
    match flash {
        Some((kind, t)) if flashing(t) => match kind {
            Flash::Win => LIGHT_GREEN,
            Flash::Big => GOLD,
            Flash::Loss => LIGHT_RED,
        },
        _ => WHITE,
    }
}

pub fn put(buf: &mut Buffer, x: i32, y: i32, text: &str, style: Style, clip: Rect) -> u16 {
    let mut col = x;
    for ch in text.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(1) as i32;
        if inside(clip, col, y) && inside(*buf.area(), col, y) {
            overdraw(buf, col as u16, y as u16)
                .set_char(ch)
                .set_style(style);
        }
        col += w;
    }
    (col - x).max(0) as u16
}

pub fn put_opaque(buf: &mut Buffer, x: i32, y: i32, text: &str, style: Style, clip: Rect) {
    for (i, ch) in text.chars().enumerate() {
        if ch != ' ' {
            put(buf, x + i as i32, y, &ch.to_string(), style, clip);
        }
    }
}

pub fn put_colored(
    buf: &mut Buffer,
    x: i32,
    y: i32,
    text: &str,
    clip: Rect,
    color: impl Fn(usize, char) -> Style,
) {
    for (i, ch) in text.chars().enumerate() {
        if ch != ' ' {
            put(buf, x + i as i32, y, &ch.to_string(), color(i, ch), clip);
        }
    }
}

pub fn centred(buf: &mut Buffer, area: Rect, y: u16, text: &str, style: Style) {
    let w = table::visual_width(text) as u16;
    let x = area.x + area.width.saturating_sub(w) / 2;
    let shown = table::ellipsize(text, area.width as usize);
    put(buf, x as i32, y as i32, &shown, style, area);
}

fn inside(area: Rect, x: i32, y: i32) -> bool {
    x >= area.x as i32 && y >= area.y as i32 && x < area.right() as i32 && y < area.bottom() as i32
}

pub fn draw_sprite(buf: &mut Buffer, sprite: &LineSprite, x: i32, top: i32, clip: Rect) {
    for (row, cells) in sprite.rows.iter().enumerate() {
        let mut col = x;
        for &(ch, color) in cells {
            let w = UnicodeWidthChar::width(ch).unwrap_or(1) as i32;
            if ch != TRANSPARENT && ch != ' ' {
                put(
                    buf,
                    col,
                    top + row as i32,
                    &ch.to_string(),
                    style(color),
                    clip,
                );
            }
            col += w;
        }
    }
}

pub fn sprite_width(sprite: &LineSprite) -> u16 {
    sprite.width() as u16
}

pub fn mirrored(sprite: &LineSprite) -> LineSprite {
    LineSprite {
        rows: crate::sprite::mirror_grid(&sprite.rows),
        body_row: sprite.body_row,
    }
}

pub fn tollomind() -> &'static LineSprite {
    static SPRITE: OnceLock<LineSprite> = OnceLock::new();
    SPRITE.get_or_init(|| {
        let mut rng = SmallRng::seed_from_u64(TOLLOMIND_SEED);
        let fish = Fish::new_for_display(FishSpecies::Tollo, &mut rng);
        let mut sprite = fish.line_sprite();
        for row in &mut sprite.rows {
            for cell in row.iter_mut() {
                if cell.0 == crate::fishes::species::EYE_ROUND {
                    cell.1 = YELLOW;
                }
            }
        }
        sprite
    })
}

fn tollomind_facing_left() -> &'static LineSprite {
    static SPRITE: OnceLock<LineSprite> = OnceLock::new();
    SPRITE.get_or_init(|| mirrored(tollomind()))
}

pub fn draw_tollomind(buf: &mut Buffer, x: i32, top: i32, clip: Rect) -> (u16, u16) {
    draw_dealer(buf, tollomind(), x, top, clip)
}

pub fn draw_tollomind_facing_left(buf: &mut Buffer, x: i32, top: i32, clip: Rect) -> (u16, u16) {
    draw_dealer(buf, tollomind_facing_left(), x, top, clip)
}

fn draw_dealer(buf: &mut Buffer, sprite: &LineSprite, x: i32, top: i32, clip: Rect) -> (u16, u16) {
    draw_sprite(buf, sprite, x, top, clip);
    let eye = sprite.rows[sprite.body_row]
        .iter()
        .position(|cell| cell.0 == crate::fishes::species::EYE_ROUND)
        .unwrap_or(0) as i32;
    let tie_y = top + sprite.rows.len() as i32;
    put(buf, x + eye - 1, tie_y, BOW_TIE, bold(BOW_TIE_COLOR), clip);
    (sprite_width(sprite), sprite.rows.len() as u16 + 1)
}

pub fn tollomind_size() -> (u16, u16) {
    let sprite = tollomind();
    (sprite_width(sprite), sprite.rows.len() as u16 + 1)
}

pub struct Chrome {
    pub rect: Rect,
    pub art: Rect,
    pub status: Rect,
}

pub fn open_frame(
    buf: &mut Buffer,
    screen: Screen,
    title: &str,
    basis: (u16, u16),
    border: Color,
    hints: &HintBar,
    widest: u16,
) -> Chrome {
    let width = basis
        .0
        .max(hints.natural_width().max(widest) + BORDERS)
        .min(screen.whole.width);
    let inner_w = width.saturating_sub(BORDERS);
    let hint_rows = hints.height(inner_w);
    let rect = screen.place(width, basis.1.max(CHROME_ROWS + hint_rows + 1));
    clear(buf, rect, BACKGROUND);
    table::draw_box_border(buf, rect, "", border, BACKGROUND);
    table::draw_box_title(buf, rect, &format!(" {title} "), WHITE, BACKGROUND);
    let inner_x = rect.x + 1;
    let hint_rows = hint_rows.min(rect.height.saturating_sub(CHROME_ROWS + 1));
    let art_h = rect.height.saturating_sub(CHROME_ROWS + hint_rows).max(1);
    let art = Rect::new(inner_x, rect.y + 1, inner_w, art_h);
    let sep = art.bottom();
    rule(buf, rect, sep, border);
    let status = Rect::new(inner_x, sep + 1, inner_w, 1);
    rule(buf, rect, sep + 2, border);
    let hint_area = Rect::new(inner_x, sep + 3, inner_w, hint_rows);
    hints.draw(buf, hint_area, BACKGROUND);
    Chrome { rect, art, status }
}

pub fn rule(buf: &mut Buffer, rect: Rect, y: u16, color: Color) {
    if y >= rect.bottom().saturating_sub(1) {
        return;
    }
    table::draw_box_separator(buf, rect.x, y, rect.width, &[], color, BACKGROUND);
}

pub fn status_row(buf: &mut Buffer, area: Rect, left: &[(String, Style)], purse: Option<Money>) {
    let purse_text = |room: usize| -> Option<Vec<(String, Style)>> {
        let amount = match purse {
            Some(amount) => cash_within(amount, room),
            None => table::INFINITY.to_string(),
        };
        Some(vec![
            ("Purse ".to_string(), style(crate::colors::DARK_GRAY)),
            (amount, style(WHITE)),
        ])
    };
    let left_w: usize = left.iter().map(|(s, _)| table::visual_width(s)).sum();
    let inner = area.width as usize;
    let pad: usize = 1;
    let mut x = area.x as i32 + pad as i32;
    for (text, st) in left {
        x += put(buf, x, area.y as i32, text, *st, area) as i32;
    }
    let room = inner.saturating_sub(left_w + pad * 2 + 2);
    let Some(right) = purse_text(room.saturating_sub(6)) else {
        return;
    };
    let right_w: usize = right.iter().map(|(s, _)| table::visual_width(s)).sum();
    if left_w + right_w + pad * 2 + 2 > inner {
        return;
    }
    let mut x = area.right() as i32 - pad as i32 - right_w as i32;
    for (text, st) in &right {
        x += put(buf, x, area.y as i32, text, *st, area) as i32;
    }
}

pub fn total_color(points: u32, done: bool) -> Color {
    if points > 21 {
        return LIGHT_RED;
    }
    if points == 21 {
        return GOLD;
    }
    if points <= crate::casino::blackjack::SAFE_TOTAL {
        return LIGHT_GREEN;
    }
    if done { WHITE } else { LIGHT_YELLOW }
}

pub fn multiple_color(m: Multiple) -> Color {
    if m >= Multiple::whole(100) {
        GOLD
    } else if m > Multiple::ONE {
        LIGHT_GREEN
    } else if m == Multiple::ONE {
        LIGHT_YELLOW
    } else {
        LIGHT_RED
    }
}

pub struct Tier {
    pub name: &'static str,
    pub color: Color,
    pub glistens: bool,
}

pub fn tier(m: Multiple, jackpot: bool) -> Tier {
    let at = |n| m >= Multiple::whole(n);
    let (name, color, glistens) = if jackpot {
        ("Jackpot", GOLD, true)
    } else if at(1000) {
        ("Absolutely Stupid Win", GOLD, true)
    } else if at(250) {
        ("Stupid Win", GOLD, true)
    } else if at(50) {
        ("Huge Win", GOLD_PALE, false)
    } else {
        ("Big Win", LIGHT_GREEN, false)
    };
    Tier {
        name,
        color,
        glistens,
    }
}

const DIGITS: [(char, [&str; 3]); 12] = [
    ('0', [" _ ", "| |", "|_|"]),
    ('1', ["   ", "  |", "  |"]),
    ('2', [" _ ", " _|", "|_ "]),
    ('3', [" _ ", " _|", " _|"]),
    ('4', ["   ", "|_|", "  |"]),
    ('5', [" _ ", "|_ ", " _|"]),
    ('6', [" _ ", "|_ ", "|_|"]),
    ('7', [" _ ", "  |", "  |"]),
    ('8', [" _ ", "|_|", "|_|"]),
    ('9', [" _ ", "|_|", " _|"]),
    ('.', [" ", " ", "."]),
    (',', [" ", " ", ","]),
];
pub const BIG_ROWS: u16 = 3;
const SIGNS: [char; 4] = ['+', '-', '$', '×'];
const SIGN_ROW: i32 = 1;
const SIGN_GAP: u16 = 1;

fn glyph(ch: char) -> Option<&'static [&'static str; 3]> {
    DIGITS.iter().find(|(c, _)| *c == ch).map(|(_, rows)| rows)
}

fn signs_of(text: &str) -> &str {
    let digits = text.find(|c| !SIGNS.contains(&c)).unwrap_or(text.len());
    &text[..digits]
}

pub fn big_width(text: &str) -> u16 {
    let signs = signs_of(text);
    let sign_w = signs.chars().count() as u16;
    let gap = if sign_w > 0 { SIGN_GAP } else { 0 };
    let digits: usize = text[signs.len()..]
        .chars()
        .map(|c| glyph(c).map_or(1, |g| g[0].chars().count()))
        .sum();
    sign_w + gap + digits as u16
}

pub fn big_fits(text: &str) -> bool {
    text[signs_of(text).len()..]
        .chars()
        .all(|c| glyph(c).is_some())
}

pub fn draw_big(buf: &mut Buffer, area: Rect, y: u16, text: &str, color: impl Fn(usize) -> Color) {
    let width = big_width(text);
    let mut x = area.x as i32 + area.width.saturating_sub(width) as i32 / 2;
    let signs = signs_of(text);
    for (i, ch) in signs.chars().enumerate() {
        put_opaque(
            buf,
            x,
            y as i32 + SIGN_ROW,
            &ch.to_string(),
            bold(color(i)),
            area,
        );
        x += 1;
    }
    if !signs.is_empty() {
        x += SIGN_GAP as i32;
    }
    let offset = signs.chars().count();
    for (i, ch) in text[signs.len()..].chars().enumerate() {
        let i = i + offset;
        let Some(rows) = glyph(ch) else {
            x += 1;
            continue;
        };
        for (row, line) in rows.iter().enumerate() {
            put_opaque(buf, x, y as i32 + row as i32, line, bold(color(i)), area);
        }
        x += rows[0].chars().count() as i32;
    }
}

pub fn fish_rows(fish: &Fish) -> u16 {
    match fish
        .unfish_kind()
        .and_then(crate::fishes::unfish::UnfishKind::grid)
    {
        Some(grid) => grid.height,
        None => fish.line_sprite().rows.len() as u16,
    }
}

pub fn draw_fish_at(buf: &mut Buffer, fish: &Fish, x: u16, top: u16, clip: Rect) {
    let area = Rect::new(x, top, fish.display_width as u16, fish_rows(fish)).intersection(clip);
    if area.width == 0 || area.height == 0 {
        return;
    }
    crate::ui::draw_fish_centred(buf, fish, area, BACKGROUND);
}

pub fn facing_right(fish: &Fish) -> Fish {
    let mut portrait = fish.portrait();
    portrait.facing = crate::fishes::fish::Direction::Right;
    portrait
}

pub fn hint_bar(close: &str, actions: &[String]) -> HintBar {
    actions
        .iter()
        .fold(HintBar::new(close), |bar, hint| bar.action(hint.clone()))
}

pub fn stake_parts(
    table: &crate::casino::state::Table,
    teller: &dyn Teller,
) -> Vec<(String, Style)> {
    use crate::colors::DARK_GRAY;
    if let Some(round) = &table.round {
        let amount = cash(round.on_the_line());
        return match round.staked.fish() {
            Some(name) => vec![
                (name.to_string(), bold(WHITE)),
                (" on the line ".to_string(), style(DARK_GRAY)),
                (amount, bold(LIGHT_YELLOW)),
            ],
            None => vec![
                ("On the line ".to_string(), style(DARK_GRAY)),
                (amount, bold(LIGHT_YELLOW)),
            ],
        };
    }
    if let Some(name) = table.seat.fish()
        && let Some(entrant) = teller.entrant(name)
    {
        return vec![
            (name.to_string(), bold(WHITE)),
            (" plays for ".to_string(), style(DARK_GRAY)),
            (cash(crate::casino::premium(entrant.worth)), bold(WHITE)),
        ];
    }
    let purse = teller.spendable();
    if purse == 0 {
        return vec![("Purse empty".to_string(), style(LIGHT_RED))];
    }
    vec![
        ("Stake ".to_string(), style(DARK_GRAY)),
        (cash(table.seat.shown_cash(purse)), bold(WHITE)),
    ]
}

pub fn widest_bet(table: &crate::casino::state::Table, start: &str, middle: &[&str]) -> u16 {
    use crate::ui::hints::{
        HINT_ALL_IN, HINT_BACK, HINT_DOUBLE_OR_NOTHING, HINT_STAKE, HINT_TAB_FISH,
    };
    let mut bar = HintBar::new(HINT_BACK)
        .action(HINT_STAKE)
        .action(HINT_ALL_IN);
    for hint in middle {
        bar = bar.action(*hint);
    }
    bar.action_if(table.game.takes_fish(), HINT_TAB_FISH)
        .action(start)
        .action(HINT_DOUBLE_OR_NOTHING)
        .natural_width()
}

pub fn bet_hints(
    table: &crate::casino::state::Table,
    teller: &dyn Teller,
    start: &str,
    middle: &[&str],
) -> HintBar {
    use crate::ui::hints::{
        HINT_ALL_IN, HINT_BACK, HINT_DOUBLE_OR_NOTHING, HINT_STAKE, HINT_TAB_FISH,
    };
    let mut bar = HintBar::new(HINT_BACK)
        .action(HINT_STAKE)
        .action(HINT_ALL_IN);
    for hint in middle {
        bar = bar.action(*hint);
    }
    bar.action_if(table.game.takes_fish(), HINT_TAB_FISH)
        .action(start)
        .action_if(table.can_double(teller), HINT_DOUBLE_OR_NOTHING)
}

pub fn result_text(table: &crate::casino::state::Table) -> Option<Vec<(String, Style)>> {
    use crate::casino::seat::Verdict;
    let result = table.result.as_ref()?;
    let label = |text: &str| -> Vec<(String, Style)> {
        if text.is_empty() {
            Vec::new()
        } else {
            vec![(format!("{text}  "), style(WHITE))]
        }
    };
    let mut parts = label(&result.label);
    match &result.verdict {
        Verdict::Cash { net, .. } if *net > 0 => {
            parts.push((signed_cash(*net), bold(LIGHT_GREEN)));
        }
        Verdict::Cash { net, .. } if *net == 0 => {
            parts.push(("stake back".to_string(), bold(LIGHT_YELLOW)));
        }
        Verdict::Cash { net, .. } => parts.push((signed_cash(*net), bold(LIGHT_RED))),
        Verdict::FishHome { name, winnings } => {
            parts.push((format!("{name} swims home"), bold(LIGHT_GREEN)));
            if *winnings > 0 {
                parts.push((
                    format!(" {}", signed_cash(crate::casino::signed(*winnings))),
                    bold(LIGHT_GREEN),
                ));
            }
        }
        Verdict::FishEaten { name, returned } => {
            parts.push((format!("{TOLLOMIND} ate {name}"), bold(LIGHT_RED)));
            if *returned > 0 {
                parts.push((
                    format!(" {}", signed_cash(crate::casino::signed(*returned))),
                    bold(LIGHT_YELLOW),
                ));
            }
        }
        Verdict::Netted { fish, .. } => {
            parts = match fish {
                Some(_) => vec![(result.label.clone(), bold(LIGHT_GREEN))],
                None => vec![(result.label.clone(), style(WHITE))],
            };
        }
        Verdict::Doubled {
            cash: amount,
            collected,
            ..
        } => {
            parts = if *collected {
                vec![
                    (format!("{}  ", result.label), style(WHITE)),
                    (cash(*amount), bold(LIGHT_GREEN)),
                ]
            } else {
                vec![(result.label.clone(), bold(LIGHT_RED))]
            };
        }
    }
    Some(parts)
}

pub fn draw_parts_centred(buf: &mut Buffer, area: Rect, y: u16, parts: &[(String, Style)]) {
    let width: usize = parts.iter().map(|(s, _)| table::visual_width(s)).sum();
    let mut x = area.x as i32 + (area.width as i32 - width as i32).max(0) / 2;
    for (text, st) in parts {
        x += put(buf, x, y as i32, text, *st, area) as i32;
    }
}
