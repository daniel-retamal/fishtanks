use std::ops::Range;

use ratatui::{buffer::Buffer, layout::Rect, style::Style};

use crate::colors::{AMBER, CHARCOAL, DARK_GRAY, LIGHT_CYAN, RED, VIOLET, WHITE, YELLOW};
use crate::dungeon::beat::{LogLine, View};
use crate::dungeon::items::Item;
use crate::dungeon::room::{Figure, Panel, Row, Scene, Tab, VEND_COLUMNS, Verb, slot};
use crate::dungeon::room_art::{ELEVATOR, LIFT_ARROW, LIFT_FROM, LIFT_TO};
use crate::economy::grouped;
use crate::ui::catch_overlay::{food_sprite_rows, stock_sprite};
use crate::ui::hint_bar::HintBar;
use crate::ui::hints::{
    HINT_CHOOSE, HINT_ENTER_BUY, HINT_ENTER_FIGHT, HINT_ENTER_HIRE, HINT_ENTER_ONWARD,
    HINT_ENTER_PAY, HINT_ENTER_PICK, HINT_ENTER_PRAY, HINT_ENTER_RIDE, HINT_ENTER_SELECT,
    HINT_ENTER_SIGN, HINT_ESC_BACK, HINT_ESC_LEAVE, HINT_HOW_MANY, HINT_TAB_PACK, HINT_TAB_READ,
    HINT_WHICH_FISH,
};
use crate::ui::layout::{Screen, Scroll, Scrollbar};
use crate::ui::modal::clear;
use crate::ui::table::{draw_box_border, draw_box_title, ellipsize, visual_width, wrap_words};

use super::battle::{
    BACKGROUND, BORDER, BORDERS, CURSOR, FRAME, NO_CURSOR, PAD, SPLIT_DOWN, SPLIT_UP, Span,
    UPRIGHT, inner, put_char, put_spans, row_colour, rule, style, titled, tone_colour, vitals,
};
use super::canvas::{Canvas, Cuts, seal};

pub const ART_W: u16 = 51;
pub const TEXT_W: u16 = 56;
pub const TEXT_MIN_W: u16 = 26;
pub const DESIGN_H: u16 = 20;
pub const TEXT_INDENT: usize = 2;
pub const NOTE_PAD: usize = 1;
pub const FIGURE_GAP: usize = 4;
pub const FIGURE_TOP: usize = 3;
pub const LABEL_INSET: usize = 2;
pub const SLOT_GAP: usize = 3;
pub const MACHINE_PAD: usize = 1;
pub const KEYPAD: [&str; 3] = ["[A][B]", "[1][2]", "[3][#]"];
pub const COIN_SLOT: &str = "[ $ ]";
pub const COIN_TRAY: &str = " ▒▒▒";
pub const VEND_TITLE: &str = "D E E P   V E N D";
pub const PUSH: &str = "P U S H";
pub const COIL: char = '@';
pub const PENGUIN: [&str; 4] = ["'", "(O>", "//\\", "\\V_"];
pub const TAG_OPEN: &str = "[";
pub const TAG_CLOSE: &str = "]";
const DIVIDER: u16 = 1;
const RULE_ROWS: u16 = 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cursor {
    pub selected: usize,
    pub fish: usize,
    pub reading: bool,
}

fn verb_hint(verb: Verb) -> &'static str {
    match verb {
        Verb::Select => HINT_ENTER_SELECT,
        Verb::Fight => HINT_ENTER_FIGHT,
        Verb::Hire => HINT_ENTER_HIRE,
        Verb::Pray => HINT_ENTER_PRAY,
        Verb::Sign => HINT_ENTER_SIGN,
        Verb::Pay => HINT_ENTER_PAY,
        Verb::Buy => HINT_ENTER_BUY,
        Verb::Ride => HINT_ENTER_RIDE,
        Verb::Pick => HINT_ENTER_PICK,
        Verb::Pack => HINT_ENTER_ONWARD,
    }
}

fn tab_hint(tab: Tab) -> &'static str {
    match tab {
        Tab::ReadBoth => HINT_TAB_READ,
        Tab::Pack => HINT_TAB_PACK,
    }
}

pub fn hints(panel: &Panel, position: Option<(usize, usize)>) -> HintBar {
    let close = match panel.back {
        true => HINT_ESC_BACK,
        false => HINT_ESC_LEAVE,
    };
    let mut bar = HintBar::new(close);
    if panel.rows.len() > 1 {
        bar = bar.counted(HINT_CHOOSE, position);
    }
    bar = bar
        .action_if(panel.which_fish, HINT_WHICH_FISH)
        .action_if(panel.counts, HINT_HOW_MANY)
        .action(verb_hint(panel.verb));
    match panel.tab {
        Some(tab) => bar.action(tab_hint(tab)),
        None => bar,
    }
}

fn figure_view(figure: &Figure) -> View {
    View {
        hp: figure.hp,
        max_hp: figure.max_hp,
        mp: figure.mp,
        max_mp: figure.max_mp,
        rolling: 0,
        belly_up: false,
        gone: false,
        shield: 0,
        statuses: Vec::new(),
    }
}

fn label(canvas: &mut Canvas, x: usize, y: usize, figure: &Figure) -> usize {
    let name = canvas.text(x, y, &figure.name, style(RED));
    let mut col = x;
    for (text, ink) in vitals(&figure_view(figure)) {
        col += canvas.text(col, y + 1, &text, ink);
    }
    name.max(col - x)
}

fn figure_width(figure: &Figure) -> usize {
    let sprite = figure
        .sprite
        .iter()
        .map(|row| row.width())
        .max()
        .unwrap_or(0);
    let vitals: usize = vitals(&figure_view(figure))
        .iter()
        .map(|(text, _)| visual_width(text))
        .sum();
    sprite.max(visual_width(&figure.name)).max(vitals)
}

fn figures(figures: &[Figure], room: usize) -> Canvas {
    let mut canvas = Canvas::default();
    let mut lines: Vec<Vec<&Figure>> = vec![Vec::new()];
    let mut used = 0;
    for figure in figures {
        let width = figure_width(figure);
        let line = lines.last_mut().expect("a line");
        if !line.is_empty() && used + FIGURE_GAP + width > room {
            lines.push(Vec::new());
            used = 0;
        }
        let line = lines.last_mut().expect("a line");
        if !line.is_empty() {
            used += FIGURE_GAP;
        }
        used += width;
        line.push(figure);
    }
    let mut y = FIGURE_TOP;
    for line in lines {
        let tallest = line.iter().map(|f| f.sprite.len()).max().unwrap_or(0);
        let mut x = 0;
        for figure in line {
            canvas.sprite(x, y + tallest - figure.sprite.len(), figure.sprite);
            label(&mut canvas, x, y + tallest + 1, figure);
            x += figure_width(figure) + FIGURE_GAP;
        }
        y += tallest + FIGURE_TOP;
    }
    canvas
}

fn item_rows(item: Item) -> Vec<Vec<(char, ratatui::style::Color)>> {
    match item {
        Item::Food => food_sprite_rows(),
        Item::Stock(stock) => stock_sprite(stock),
    }
}

fn vending(stock: &[(Item, u64)], room: usize) -> Canvas {
    let sprites: Vec<Vec<Vec<(char, ratatui::style::Color)>>> =
        stock.iter().map(|(item, _)| item_rows(*item)).collect();
    let prices: Vec<String> = stock
        .iter()
        .enumerate()
        .map(|(index, (_, price))| format!("{}  ${}", slot(index), grouped(u128::from(*price))))
        .collect();
    let slot_w = sprites
        .iter()
        .map(|rows| rows.iter().map(Vec::len).max().unwrap_or(0))
        .chain(prices.iter().map(|price| visual_width(price)))
        .max()
        .unwrap_or(0);
    let slot_h = sprites.iter().map(Vec::len).max().unwrap_or(0);
    let keypad_w = KEYPAD[0].len().max(COIN_SLOT.len()) + MACHINE_PAD * 2;
    let chrome = (MACHINE_PAD + 1) * 2 + MACHINE_PAD * 2 + 2;
    let fits = |columns: usize, keypad: usize| {
        chrome + columns * slot_w + columns.saturating_sub(1) * SLOT_GAP + keypad <= room
    };
    let columns = (1..=VEND_COLUMNS.min(stock.len().max(1)))
        .rev()
        .find(|&columns| fits(columns, 0))
        .unwrap_or(1);
    let keypad_beside = fits(columns, keypad_w);
    let shelves = stock.len().div_ceil(columns).max(1);
    let window_w = MACHINE_PAD * 2 + columns * slot_w + (columns - 1) * SLOT_GAP + 2;
    let shelf_h = slot_h + 2;
    let window_h = shelves * shelf_h + 2 + shelves.saturating_sub(1);
    let keypad_h = KEYPAD.len() + 3;
    let below = match keypad_beside {
        true => 0,
        false => keypad_h,
    };
    let machine_w = window_w + MACHINE_PAD * 2 + 2 + if keypad_beside { keypad_w } else { 0 };
    let machine_h = window_h + 5 + below;
    let mut canvas = Canvas::default();
    let frame = style(crate::colors::GRAY);
    let dim = style(DARK_GRAY);
    boxed(&mut canvas, 0, 0, machine_w, machine_h, frame);
    let title_x = machine_w.saturating_sub(VEND_TITLE.len()) / 2;
    canvas.text(title_x, 1, VEND_TITLE, style(LIGHT_CYAN));
    let wx = MACHINE_PAD + 1;
    let wy = 2;
    boxed(&mut canvas, wx, wy, window_w, window_h, dim);
    for (index, rows) in sprites.iter().enumerate() {
        let shelf = index / columns;
        let column = index % columns;
        let x = wx + 1 + MACHINE_PAD + column * (slot_w + SLOT_GAP);
        let top = wy + 1 + shelf * (shelf_h + 1);
        let width = rows.iter().map(Vec::len).max().unwrap_or(0);
        canvas.cells(x + (slot_w - width) / 2, top + slot_h - rows.len(), rows);
        let coil: String = std::iter::repeat_n(COIL, slot_w).collect();
        canvas.text(x, top + slot_h, &coil, style(CHARCOAL));
        canvas.text(x, top + slot_h + 1, &prices[index], style(YELLOW));
        if shelf + 1 < shelves && column == 0 {
            let y = top + shelf_h;
            canvas.set(wx, y, '├', dim);
            for dx in 1..window_w - 1 {
                canvas.set(wx + dx, y, '─', dim);
            }
            canvas.set(wx + window_w - 1, y, '┤', dim);
        }
    }
    let (kx, ky) = match keypad_beside {
        true => (wx + window_w + MACHINE_PAD, wy + 2),
        false => (wx + MACHINE_PAD, wy + window_h + 1),
    };
    for (row, keys) in KEYPAD.iter().enumerate() {
        canvas.text(kx, ky + row, keys, frame);
    }
    canvas.text(kx, ky + KEYPAD.len() + 1, COIN_SLOT, style(AMBER));
    canvas.text(kx, ky + KEYPAD.len() + 2, COIN_TRAY, dim);
    let push = format!("[ {PUSH} ]");
    let push_x = wx + window_w.saturating_sub(visual_width(&push)) / 2;
    canvas.text(push_x, machine_h - 2, &push, dim);
    let penguin_w = PENGUIN
        .iter()
        .map(|line| visual_width(line))
        .max()
        .unwrap_or(0);
    let (px, py) = match machine_w + 1 + penguin_w <= room {
        true => (machine_w + 1, machine_h / 2),
        false => (machine_w.saturating_sub(penguin_w) / 2, machine_h),
    };
    for (row, line) in PENGUIN.iter().enumerate() {
        canvas.text(px, py + row, line, style(WHITE));
    }
    canvas
}

fn boxed(canvas: &mut Canvas, x: usize, y: usize, w: usize, h: usize, ink: Style) {
    if w < 2 || h < 2 {
        return;
    }
    for dx in 1..w - 1 {
        canvas.set(x + dx, y, '─', ink);
        canvas.set(x + dx, y + h - 1, '─', ink);
    }
    for dy in 1..h - 1 {
        canvas.set(x, y + dy, '│', ink);
        canvas.set(x + w - 1, y + dy, '│', ink);
    }
    canvas.set(x, y, '┌', ink);
    canvas.set(x + w - 1, y, '┐', ink);
    canvas.set(x, y + h - 1, '└', ink);
    canvas.set(x + w - 1, y + h - 1, '┘', ink);
}

fn lift(from: u32, to: u32) -> Canvas {
    let mut canvas = Canvas::default();
    canvas.sprite(0, 0, ELEVATOR);
    let from = from.to_string();
    let (row, col) = LIFT_FROM;
    let start = (col + 1).saturating_sub(from.len());
    for x in start..=col {
        canvas.set(x, row, ' ', style(BACKGROUND));
    }
    canvas.text(start, row, &from, style(YELLOW));
    let (row, col) = LIFT_ARROW;
    canvas.set(col, row, '▼', style(YELLOW));
    let (row, col) = LIFT_TO;
    canvas.set(col, row, ' ', style(BACKGROUND));
    canvas.text(col, row, &to.to_string(), style(DARK_GRAY));
    let button = ELEVATOR
        .iter()
        .enumerate()
        .skip(LIFT_ARROW.0 + 1)
        .find_map(|(y, row)| row.text.chars().position(|g| g == '▼').map(|x| (x, y)));
    if let Some((x, y)) = button {
        canvas.set(x, y, '▼', style(YELLOW));
    }
    canvas
}

pub fn scene(panel: &Panel, room: usize) -> Canvas {
    let mut canvas = match &panel.art {
        Scene::Rows(rows) => {
            let mut canvas = Canvas::default();
            canvas.sprite(0, 0, rows);
            canvas
        }
        Scene::Fish(fish) => {
            let mut canvas = Canvas::default();
            canvas.blit(0, FIGURE_TOP, &Canvas::fish(fish));
            canvas
        }
        Scene::Item(item) => {
            let mut canvas = Canvas::default();
            canvas.cells(0, FIGURE_TOP, &item_rows(*item));
            canvas
        }
        Scene::Vending(stock) => vending(stock, room),
        Scene::Lift { from, to } => lift(*from, *to),
    };
    if panel.figures.is_empty() {
        return canvas;
    }
    match panel.draw_figures {
        true => {
            let drawn = figures(&panel.figures, room);
            let y = canvas.height() as usize;
            canvas.blit(0, y, &drawn);
        }
        false => {
            let mut y = canvas.height() as usize + 1;
            for figure in &panel.figures {
                label(&mut canvas, LABEL_INSET, y, figure);
                y += LABEL_ROWS;
            }
        }
    }
    canvas
}

pub const LABEL_ROWS: usize = 2;

fn row_lines(row: &Row, selected: bool, width: usize) -> Vec<Vec<Span>> {
    let mark = if selected { CURSOR } else { NO_CURSOR };
    let ink = row_colour(row.usable);
    let tag = row
        .tag
        .as_ref()
        .map(|tag| format!(" {TAG_OPEN}{tag}{TAG_CLOSE}"))
        .unwrap_or_default();
    let lead = PAD as usize + visual_width(mark);
    let label_room = width
        .saturating_sub(lead + NOTE_PAD + visual_width(&tag))
        .max(1);
    let label = ellipsize(&row.label, label_room);
    let mut first = vec![
        (" ".repeat(PAD as usize), style(WHITE)),
        (mark.to_string(), style(WHITE)),
        (label.clone(), style(ink)),
    ];
    if !tag.is_empty() {
        first.push((tag.clone(), style(VIOLET)));
    }
    if row.note.is_empty() {
        return vec![first];
    }
    let note_ink = match row.usable {
        true => tone_colour(row.note_tone),
        false => DARK_GRAY,
    };
    let note_w = visual_width(&row.note);
    let used = lead + visual_width(&label) + visual_width(&tag);
    if used + NOTE_GAP + note_w + NOTE_PAD <= width {
        let gap = width - used - note_w - NOTE_PAD;
        first.push((" ".repeat(gap), style(WHITE)));
        first.push((row.note.clone(), style(note_ink)));
        return vec![first];
    }
    let mut lines = vec![first];
    for piece in wrap_words(&row.note, width.saturating_sub(lead + NOTE_PAD).max(1)) {
        let gap = width.saturating_sub(visual_width(&piece) + NOTE_PAD);
        lines.push(vec![
            (" ".repeat(gap), style(WHITE)),
            (piece, style(note_ink)),
        ]);
    }
    lines
}

pub const NOTE_GAP: usize = 2;

pub struct Body {
    pub lines: Vec<Vec<Span>>,
    pub focus: Range<usize>,
}

fn text_lines(lines: &mut Vec<Vec<Span>>, said: &[LogLine], width: usize) {
    for line in said {
        if line.text.is_empty() {
            lines.push(Vec::new());
            continue;
        }
        for piece in wrap_words(
            &line.text,
            width.saturating_sub(TEXT_INDENT + NOTE_PAD).max(1),
        ) {
            lines.push(vec![
                (" ".repeat(TEXT_INDENT), style(WHITE)),
                (piece, style(tone_colour(line.tone))),
            ]);
        }
    }
}

pub fn body(panel: &Panel, cursor: Cursor, width: usize) -> Body {
    let mut lines: Vec<Vec<Span>> = vec![Vec::new()];
    text_lines(&mut lines, &panel.text, width);
    lines.push(Vec::new());
    let mut focus = lines.len()..lines.len();
    for (index, row) in panel.rows.iter().enumerate() {
        let selected = index == cursor.selected;
        let start = lines.len();
        lines.extend(row_lines(row, selected, width));
        if selected {
            focus = start..lines.len();
        }
    }
    let about = panel
        .rows
        .get(cursor.selected)
        .map(|row| row.about.as_slice())
        .unwrap_or_default();
    let shows = match panel.tab {
        Some(Tab::ReadBoth) => cursor.reading,
        Some(Tab::Pack) | None => true,
    };
    if shows && !about.is_empty() {
        lines.push(Vec::new());
        text_lines(&mut lines, about, width);
        focus.end = lines.len();
    }
    Body { lines, focus }
}

fn draw_lines(buf: &mut Buffer, lines: &[Vec<Span>], area: Rect) {
    for (row, spans) in lines.iter().enumerate() {
        let y = area.y + row as u16;
        if y >= area.bottom() {
            return;
        }
        put_spans(buf, area.x, y, spans, area);
    }
}

fn frame(buf: &mut Buffer, rect: Rect, title: &str) {
    clear(buf, rect, BACKGROUND);
    draw_box_border(buf, rect, "", FRAME, BACKGROUND);
    draw_box_title(buf, rect, &titled(title), WHITE, BACKGROUND);
}

pub struct Sheet<'a> {
    pub panel: &'a Panel,
    pub cursor: Cursor,
    pub scroll: &'a Scroll,
}

pub fn draw(buf: &mut Buffer, sheet: &Sheet, screen: Screen) {
    let art = scene(sheet.panel, ART_W as usize);
    let left_w = ART_W.max(art.width() + PAD * 2);
    let natural = BORDERS + left_w + DIVIDER + TEXT_W;
    let width = natural.min(screen.whole.width);
    let text_w = width.saturating_sub(BORDERS + DIVIDER + left_w);
    match text_w >= TEXT_MIN_W {
        true => draw_wide(buf, sheet, screen, &art, (left_w, width)),
        false => {
            let room = screen.whole.width.saturating_sub(BORDERS + PAD * 2) as usize;
            draw_stacked(buf, sheet, screen, &scene(sheet.panel, room))
        }
    }
}

fn place(screen: Screen, width: u16, content_h: u16) -> Rect {
    let floor = DESIGN_H.min(screen.whole.height);
    let height = content_h.max(floor).min(screen.whole.height);
    screen.place(width, height)
}

fn position(body: &Body, window: usize, rows: usize, selected: usize) -> Option<(usize, usize)> {
    (body.lines.len() > window).then_some((selected + 1, rows))
}

fn draw_wide(
    buf: &mut Buffer,
    sheet: &Sheet,
    screen: Screen,
    art: &Canvas,
    (left_w, width): (u16, u16),
) {
    let (panel, cursor, scroll) = (sheet.panel, sheet.cursor, sheet.scroll);
    let text_w = width.saturating_sub(BORDERS + DIVIDER + left_w);
    let body = body(panel, cursor, text_w as usize);
    let inner_w = width.saturating_sub(BORDERS);
    let calm = hints(panel, None);
    let hint_h = calm.height(inner_w);
    let content = (art.height()).max(body.lines.len() as u16);
    let rect = place(screen, width, content + RULE_ROWS + hint_h + BORDERS);
    let inside = inner(rect);
    let hint_h = hint_h.min(inside.height.saturating_sub(1));
    let body_h = inside.height.saturating_sub(hint_h + RULE_ROWS);
    let shown_position = position(&body, body_h as usize, panel.rows.len(), cursor.selected);
    let bar = hints(panel, shown_position);
    let hint_h = bar.height(inner_w).min(inside.height.saturating_sub(1));
    let body_h = inside.height.saturating_sub(hint_h + RULE_ROWS);
    frame(buf, rect, &panel.title);
    let divider = inside.x + left_w;
    let rule_y = inside.y + body_h;
    put_char(buf, divider, rect.y, SPLIT_DOWN);
    for y in inside.y..rule_y {
        put_char(buf, divider, y, UPRIGHT);
    }
    rule(buf, rect, rule_y);
    put_char(buf, divider, rule_y, SPLIT_UP);
    let left = Rect::new(inside.x, inside.y, left_w, body_h);
    let x = left.x as i32 + (left.width.saturating_sub(art.width()) / 2) as i32;
    art.draw(buf, x, left.y as i32, left);
    seal(
        buf,
        left,
        Cuts {
            bottom: art.height() > body_h,
            ..Cuts::default()
        },
    );
    let right = Rect::new(divider + 1, inside.y, text_w, body_h);
    let total = body.lines.len();
    let shown = scroll.reveal(body.focus.clone(), body_h as usize, total);
    draw_lines(buf, &body.lines[shown.clone()], right);
    Scrollbar {
        x: rect.right().saturating_sub(BORDER),
        top: right.y,
        height: body_h,
    }
    .draw(buf, shown, total, WHITE);
    bar.draw(
        buf,
        Rect::new(inside.x, rule_y + RULE_ROWS, inside.width, hint_h),
        BACKGROUND,
    );
}

fn draw_stacked(buf: &mut Buffer, sheet: &Sheet, screen: Screen, art: &Canvas) {
    let (panel, cursor, scroll) = (sheet.panel, sheet.cursor, sheet.scroll);
    let width = screen.whole.width;
    let inner_w = width.saturating_sub(BORDERS);
    let body = body(panel, cursor, inner_w as usize);
    let indent = (inner_w.saturating_sub(art.width()) / 2) as usize;
    let mut lines: Vec<Vec<Span>> = (0..art.height() as usize)
        .map(|row| {
            let mut spans = vec![(" ".repeat(indent), style(WHITE))];
            spans.extend(art.row_spans(row));
            spans
        })
        .collect();
    let offset = lines.len();
    lines.extend(body.lines);
    let focus = body.focus.start + offset..body.focus.end + offset;
    let calm = hints(panel, None);
    let hint_h = calm.height(inner_w);
    let rect = place(
        screen,
        width,
        lines.len() as u16 + RULE_ROWS + hint_h + BORDERS,
    );
    let inside = inner(rect);
    let window = inside.height.saturating_sub(hint_h + RULE_ROWS) as usize;
    let overflowing = lines.len() > window;
    let bar = hints(
        panel,
        overflowing.then_some((cursor.selected + 1, panel.rows.len())),
    );
    let hint_h = bar.height(inner_w).min(inside.height.saturating_sub(1));
    let body_h = inside.height.saturating_sub(hint_h + RULE_ROWS);
    frame(buf, rect, &panel.title);
    let rule_y = inside.y + body_h;
    rule(buf, rect, rule_y);
    let total = lines.len();
    let shown = scroll.reveal(focus, body_h as usize, total);
    let window = Rect::new(inside.x, inside.y, inside.width, body_h);
    draw_lines(buf, &lines[shown.clone()], window);
    let wide_art = art.width() > inside.width;
    seal(
        buf,
        window,
        Cuts {
            top: shown.start > 0,
            bottom: shown.end < total,
            left: wide_art,
            right: wide_art,
        },
    );
    Scrollbar {
        x: rect.right().saturating_sub(BORDER),
        top: inside.y,
        height: body_h,
    }
    .draw(buf, shown, total, WHITE);
    bar.draw(
        buf,
        Rect::new(inside.x, rule_y + RULE_ROWS, inside.width, hint_h),
        BACKGROUND,
    );
}
