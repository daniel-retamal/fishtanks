use std::ops::Range;

use ratatui::{buffer::Buffer, layout::Rect, style::Style};

use crate::colors::{AMBER, CHARCOAL, DARK_GRAY, GRAY, LIGHT_CYAN, RED, VIOLET, WHITE, YELLOW};
use crate::dungeon::beat::{LogLine, View};
use crate::dungeon::bestiary::SpriteRow;
use crate::dungeon::items::Item;
use crate::dungeon::room::{
    Card, Figure, Panel, Portrait, Row, Scene, Tab, VEND_COLUMNS, Verb, slot,
};
use crate::dungeon::room_art::{LIFT_ARROW, LIFT_FROM, LIFT_TO, LIFTS};
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
use crate::ui::shop_overlay::PENGUIN_LINES;
use crate::ui::table::{draw_box_border, draw_box_title, ellipsize, visual_width, wrap_words};

use super::battle::{
    BACKGROUND, BORDER, BORDERS, CURSOR, FRAME, NO_CURSOR, PAD, SPLIT_DOWN, SPLIT_UP, Span,
    TITLE_INSET, UPRIGHT, inner, put_char, put_spans, row_colour, rule, style, titled, tone_colour,
    vitals,
};
use super::canvas::{Canvas, Picture};

pub const ART_W: u16 = 51;
pub const TEXT_W: u16 = 56;
pub const TEXT_MIN_W: u16 = 26;
pub const DESIGN_H: u16 = 20;
pub const TEXT_INDENT: usize = 2;
pub const NOTE_PAD: usize = 1;
pub const NOTE_GAP: usize = 2;
pub const FIGURE_GAP: usize = 4;
pub const FIGURE_LINE_GAP: usize = 2;
pub const ART_GAP: usize = 1;
pub const STACK_TEXT_FLOOR: usize = 8;
pub const LABEL_ROWS: usize = 2;
pub const CARD_PAD_X: usize = 3;
pub const CARD_PAD_Y: usize = 1;
pub const CARD_MIN_INNER: usize = 21;
pub const CARD_TITLE_ROOM: usize = 6;
pub const SEABED: [char; 2] = ['.', ':'];
pub const KEYPAD: [&str; 3] = ["[A][B]", "[1][2]", "[3][#]"];
pub const COIN_SLOT: &str = "[ $ ]";
pub const COIN_RETURN: &str = "▒▒▒";
pub const VEND_TITLE: &str = "≋  D E E P   V E N D  ≋";
pub const PUSH: &str = "P U S H";
pub const SOLD_OUT: &str = "SOLD OUT";
pub const NONE: &str = "-";
pub const COIL: char = '@';
pub const DISPLAY_INNER: usize = 4;
pub const SIDE_W: usize = 10;
pub const SLOT_GAP: usize = 3;
pub const TIGHT_SLOT_GAP: usize = 1;
pub const MACHINE_PAD: usize = 1;
pub const FLAP_INSET: usize = 4;
pub const PENGUIN_GAP: usize = 1;
pub const LABEL_GAP: usize = 2;
pub const TAG_OPEN: &str = "[";
pub const TAG_CLOSE: &str = "]";
const DIVIDER: u16 = 1;
const RULE_ROWS: u16 = 1;
const SHELF_TOP: usize = 1;
const SHELF_FOOT: usize = 2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cursor {
    pub selected: usize,
    pub fish: usize,
    pub reading: bool,
}

fn verb_hint(verb: Verb) -> Option<&'static str> {
    Some(match verb {
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
        Verb::Browse => return None,
    })
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
        .action_if(panel.counts, HINT_HOW_MANY);
    if let Some(verb) = verb_hint(panel.verb) {
        bar = bar.action(verb);
    }
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

fn label_spans(figure: &Figure) -> (Vec<Span>, Vec<Span>) {
    (
        vec![(figure.name.clone(), style(RED))],
        vitals(&figure_view(figure)),
    )
}

fn spans_width(spans: &[Span]) -> usize {
    spans.iter().map(|(text, _)| visual_width(text)).sum()
}

fn sprite(rows: &[SpriteRow]) -> Canvas {
    let mut canvas = Canvas::default();
    canvas.sprite(0, 0, rows);
    canvas.trimmed()
}

fn figure_canvas(figure: &Figure) -> Canvas {
    let art = sprite(figure.sprite);
    let (name, vitals) = label_spans(figure);
    let width = (art.width() as usize)
        .max(spans_width(&name))
        .max(spans_width(&vitals));
    let mut canvas = Canvas::default();
    canvas.blit((width - art.width() as usize) / 2, 0, &art);
    let y = art.height() as usize + 1;
    canvas.centred_row(y, width, &name);
    canvas.centred_row(y + 1, width, &vitals);
    canvas
}

fn lineup(figures: &[Figure], room: usize) -> Canvas {
    let drawn: Vec<Canvas> = figures.iter().map(figure_canvas).collect();
    let mut lines: Vec<Vec<&Canvas>> = vec![Vec::new()];
    let mut used = 0;
    for canvas in &drawn {
        let width = canvas.width() as usize;
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
        line.push(canvas);
    }
    let widths: Vec<usize> = lines
        .iter()
        .map(|line| {
            line.iter().map(|c| c.width() as usize).sum::<usize>()
                + line.len().saturating_sub(1) * FIGURE_GAP
        })
        .collect();
    let widest = widths.iter().copied().max().unwrap_or(0);
    let mut canvas = Canvas::default();
    let mut y = 0;
    for (line, width) in lines.iter().zip(widths) {
        let tallest = line.iter().map(|c| c.height() as usize).max().unwrap_or(0);
        let mut x = (widest - width) / 2;
        for figure in line {
            canvas.blit(x, y + tallest - figure.height() as usize, figure);
            x += figure.width() as usize + FIGURE_GAP;
        }
        y += tallest + FIGURE_LINE_GAP;
    }
    canvas
}

fn labels(figures: &[Figure]) -> Canvas {
    let lines: Vec<(Vec<Span>, Vec<Span>)> = figures.iter().map(label_spans).collect();
    let width = lines
        .iter()
        .map(|(name, vitals)| spans_width(name).max(spans_width(vitals)))
        .max()
        .unwrap_or(0);
    let mut canvas = Canvas::default();
    for (index, (name, vitals)) in lines.iter().enumerate() {
        canvas.centred_row(index * LABEL_ROWS, width, name);
        canvas.centred_row(index * LABEL_ROWS + 1, width, vitals);
    }
    canvas
}

fn below(top: &Canvas, under: &Canvas) -> Canvas {
    if top.height() == 0 {
        return under.clone();
    }
    let width = top.width().max(under.width()) as usize;
    let mut canvas = Canvas::default();
    canvas.blit((width - top.width() as usize) / 2, 0, top);
    canvas.blit(
        (width - under.width() as usize) / 2,
        top.height() as usize + ART_GAP,
        under,
    );
    canvas
}

fn item_canvas(item: Item) -> Canvas {
    let rows = match item {
        Item::Food => food_sprite_rows(),
        Item::Stock(stock) => stock_sprite(stock),
    };
    let mut canvas = Canvas::default();
    canvas.cells(0, 0, &rows);
    canvas.trimmed()
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

fn titled_box(canvas: &mut Canvas, (w, h): (usize, usize), title: &str, ink: Style) {
    boxed(canvas, 0, 0, w, h, ink);
    let room = w.saturating_sub(TITLE_INSET as usize * 2 + 2);
    let title = format!(" {} ", ellipsize(title, room));
    for (dx, glyph) in title.chars().enumerate() {
        canvas.set(TITLE_INSET as usize + dx, 0, glyph, style(WHITE));
    }
}

fn seabed(canvas: &mut Canvas, x: usize, y: usize, width: usize) {
    for dx in 0..width {
        canvas.set(x + dx, y, SEABED[dx % SEABED.len()], style(CHARCOAL));
    }
}

fn cards(card: &Card) -> Vec<Canvas> {
    let (portrait, grounded) = match &card.portrait {
        Portrait::Fish(fish) => (Canvas::fish(fish).trimmed(), true),
        Portrait::Item(item) => (item_canvas(*item), false),
    };
    let pw = portrait.width() as usize;
    let mut inner = (pw + CARD_PAD_X * 2)
        .max(CARD_MIN_INNER)
        .max(visual_width(&card.title) + CARD_TITLE_ROOM);
    if (inner - pw) % 2 == 1 {
        inner += 1;
    }
    let ground_rows = usize::from(grounded);
    let inner_h = portrait.height() as usize + CARD_PAD_Y * 2 + ground_rows;
    let (w, h) = (inner + 2, inner_h + 2);
    let frame = match card.lit {
        true => style(WHITE),
        false => style(GRAY),
    };
    let mut canvas = Canvas::default();
    titled_box(&mut canvas, (w, h), &card.title, frame);
    canvas.blit(1 + (inner - pw) / 2, 1 + CARD_PAD_Y, &portrait);
    if grounded {
        seabed(&mut canvas, 1, h - 2, inner);
    }
    for (index, line) in card.caption.iter().enumerate() {
        let spans = vec![(line.text.clone(), style(tone_colour(line.tone)))];
        canvas.centred_row(h + index, w, &spans);
    }
    let name = vec![(card.title.clone(), style(WHITE))];
    let compact_w = pw.max(visual_width(&card.title));
    let mut compact = Canvas::default();
    compact.blit(centred_x(compact_w, pw), 0, &portrait);
    compact.centred_row(portrait.height() as usize + ART_GAP, compact_w, &name);
    vec![canvas, compact]
}

struct Machine {
    shelves: usize,
    side: bool,
    flap_box: bool,
    gap: usize,
}

const MACHINES: [Machine; 4] = [
    Machine {
        shelves: 2,
        side: true,
        flap_box: true,
        gap: SLOT_GAP,
    },
    Machine {
        shelves: 1,
        side: true,
        flap_box: true,
        gap: SLOT_GAP,
    },
    Machine {
        shelves: 1,
        side: true,
        flap_box: false,
        gap: SLOT_GAP,
    },
    Machine {
        shelves: 1,
        side: false,
        flap_box: false,
        gap: TIGHT_SLOT_GAP,
    },
];

struct Shelf<'a> {
    goods: Vec<Option<(&'a Canvas, String, String)>>,
}

fn centred_x(room: usize, width: usize) -> usize {
    room.saturating_sub(width) / 2
}

fn penguin() -> Canvas {
    let mut canvas = Canvas::default();
    for (y, line) in PENGUIN_LINES.iter().enumerate() {
        canvas.text(0, y, line, style(WHITE));
    }
    canvas.trimmed()
}

fn vending_machine(
    shelves: &[Shelf],
    (slot_w, slot_h): (usize, usize),
    shown: &str,
    machine: &Machine,
) -> Canvas {
    let shelf_h = SHELF_TOP + slot_h + SHELF_FOOT;
    let window_w = 2 + MACHINE_PAD * 2 + VEND_COLUMNS * slot_w + (VEND_COLUMNS - 1) * machine.gap;
    let window_h = 2 + machine.shelves * shelf_h + machine.shelves.saturating_sub(1);
    let side_w = match machine.side {
        true => SIDE_W + MACHINE_PAD,
        false => 0,
    };
    let flap_h = match machine.flap_box {
        true => 3,
        false => 1,
    };
    let cabinet_w = 2 + MACHINE_PAD * 2 + window_w + side_w;
    let cabinet_h = 2 + 1 + window_h + flap_h;
    let frame = style(GRAY);
    let dim = style(DARK_GRAY);
    let mut canvas = Canvas::default();
    boxed(&mut canvas, 0, 0, cabinet_w, cabinet_h, frame);
    canvas.text(
        centred_x(cabinet_w, visual_width(VEND_TITLE)),
        1,
        VEND_TITLE,
        style(LIGHT_CYAN),
    );
    let (wx, wy) = (1 + MACHINE_PAD, 2);
    boxed(&mut canvas, wx, wy, window_w, window_h, dim);
    for (row, shelf) in shelves.iter().take(machine.shelves).enumerate() {
        let top = wy + 1 + row * (shelf_h + 1);
        if row > 0 {
            canvas.set(wx, top - 1, '├', dim);
            for dx in 1..window_w - 1 {
                canvas.set(wx + dx, top - 1, '─', dim);
            }
            canvas.set(wx + window_w - 1, top - 1, '┤', dim);
        }
        for (column, goods) in shelf.goods.iter().enumerate() {
            let x = wx + 1 + MACHINE_PAD + column * (slot_w + machine.gap);
            let coil_y = top + SHELF_TOP + slot_h;
            let coil: String = std::iter::repeat_n(COIL, slot_w).collect();
            canvas.text(x, coil_y, &coil, style(CHARCOAL));
            match goods {
                Some((art, code, price)) => {
                    let ax = x + centred_x(slot_w, art.width() as usize);
                    canvas.blit(ax, coil_y - art.height() as usize, art);
                    let label_w = visual_width(code) + LABEL_GAP + visual_width(price);
                    let lx = x + centred_x(slot_w, label_w);
                    let lx = lx + canvas.text(lx, coil_y + 1, code, style(WHITE)) + LABEL_GAP;
                    canvas.text(lx, coil_y + 1, price, style(AMBER));
                }
                None => {
                    let sx = x + centred_x(slot_w, visual_width(SOLD_OUT));
                    canvas.text(sx, top + SHELF_TOP + slot_h / 2, SOLD_OUT, dim);
                    let code = slot(row * VEND_COLUMNS + column);
                    let label_w = visual_width(&code) + LABEL_GAP + visual_width(NONE);
                    let lx = x + centred_x(slot_w, label_w);
                    let lx = lx + canvas.text(lx, coil_y + 1, &code, dim) + LABEL_GAP;
                    canvas.text(lx, coil_y + 1, NONE, dim);
                }
            }
        }
    }
    if machine.side {
        let sx = wx + window_w + MACHINE_PAD;
        let mut y = wy + 1;
        let display_w = DISPLAY_INNER + 2;
        let dx = sx + centred_x(SIDE_W, display_w);
        boxed(&mut canvas, dx, y, display_w, 3, frame);
        canvas.text(
            dx + 1 + centred_x(DISPLAY_INNER, visual_width(shown)),
            y + 1,
            shown,
            style(YELLOW),
        );
        y += 4;
        for keys in KEYPAD {
            canvas.text(sx + centred_x(SIDE_W, visual_width(keys)), y, keys, frame);
            y += 1;
        }
        y += 1;
        canvas.text(
            sx + centred_x(SIDE_W, visual_width(COIN_SLOT)),
            y,
            COIN_SLOT,
            style(AMBER),
        );
        canvas.text(
            sx + centred_x(SIDE_W, visual_width(COIN_RETURN)),
            y + 1,
            COIN_RETURN,
            dim,
        );
    }
    let flap_y = wy + window_h;
    match machine.flap_box {
        true => {
            let flap_w = window_w - FLAP_INSET * 2;
            boxed(&mut canvas, wx + FLAP_INSET, flap_y, flap_w, flap_h, dim);
            canvas.text(
                wx + FLAP_INSET + centred_x(flap_w, visual_width(PUSH)),
                flap_y + 1,
                PUSH,
                dim,
            );
        }
        false => {
            let push = format!("[ {PUSH} ]");
            canvas.text(
                wx + centred_x(window_w, visual_width(&push)),
                flap_y,
                &push,
                dim,
            );
        }
    }
    let bird = penguin();
    canvas.blit(
        cabinet_w + PENGUIN_GAP,
        cabinet_h - bird.height() as usize,
        &bird,
    );
    canvas
}

fn vending(stock: &[(Item, u64)], selected: usize) -> Vec<Canvas> {
    let arts: Vec<Canvas> = stock.iter().map(|(item, _)| item_canvas(*item)).collect();
    let codes: Vec<String> = (0..stock.len()).map(slot).collect();
    let prices: Vec<String> = stock
        .iter()
        .map(|(_, price)| format!("${}", grouped(u128::from(*price))))
        .collect();
    let slot_w = arts
        .iter()
        .map(|art| art.width() as usize)
        .chain(
            codes
                .iter()
                .zip(&prices)
                .map(|(code, price)| visual_width(code) + LABEL_GAP + visual_width(price)),
        )
        .chain([visual_width(SOLD_OUT)])
        .max()
        .unwrap_or(0);
    let slot_h = arts
        .iter()
        .map(|art| art.height() as usize)
        .max()
        .unwrap_or(0);
    let goods: Vec<Option<(&Canvas, String, String)>> = arts
        .iter()
        .zip(codes)
        .zip(prices)
        .map(|((art, code), price)| Some((art, code, price)))
        .collect();
    let stocked = goods.len().div_ceil(VEND_COLUMNS).max(1);
    let mut shelves: Vec<Shelf> = (0..stocked)
        .map(|row| Shelf {
            goods: (0..VEND_COLUMNS)
                .map(|column| goods.get(row * VEND_COLUMNS + column).cloned().flatten())
                .collect(),
        })
        .collect();
    shelves.push(Shelf {
        goods: vec![None; VEND_COLUMNS],
    });
    let shown = match selected < stock.len() {
        true => slot(selected),
        false => "--".to_string(),
    };
    MACHINES
        .iter()
        .filter(|machine| machine.shelves >= stocked)
        .map(|machine| vending_machine(&shelves, (slot_w, slot_h), &shown, machine))
        .collect()
}

fn lift(from: u32, to: u32) -> Vec<Canvas> {
    LIFTS
        .iter()
        .map(|rows| {
            let mut canvas = Canvas::default();
            canvas.sprite(0, 0, rows);
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
            let button = rows
                .iter()
                .enumerate()
                .skip(LIFT_ARROW.0 + 1)
                .find_map(|(y, row)| row.text.chars().position(|g| g == '▼').map(|x| (x, y)));
            if let Some((x, y)) = button {
                canvas.set(x, y, '▼', style(YELLOW));
            }
            canvas
        })
        .collect()
}

pub fn scene(panel: &Panel, cursor: Cursor, room: usize) -> Picture {
    let bases: Vec<Canvas> = match &panel.art {
        Scene::Rows(rows) => vec![sprite(rows)],
        Scene::Card(shown) => cards(shown),
        Scene::Vending(stock) => vending(stock, cursor.selected),
        Scene::Lift { from, to } => lift(*from, *to),
    };
    if panel.figures.is_empty() {
        return Picture::of(bases);
    }
    let figures = match panel.draw_figures {
        true => lineup(&panel.figures, room),
        false => labels(&panel.figures),
    };
    Picture::of(bases.iter().map(|base| below(base, &figures)).collect())
}

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
    if !panel.text.is_empty() {
        text_lines(&mut lines, &panel.text, width);
        lines.push(Vec::new());
    }
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

fn tallest_hints(panel: &Panel, width: u16) -> u16 {
    hints(panel, Some((panel.rows.len(), panel.rows.len()))).height(width)
}

pub fn draw(buf: &mut Buffer, sheet: &Sheet, screen: Screen) {
    let width = screen.whole.width;
    let inner_w = width.saturating_sub(BORDERS);
    let room_h = screen
        .whole
        .height
        .saturating_sub(BORDERS + RULE_ROWS + tallest_hints(sheet.panel, inner_w));
    let wide_room = inner_w.saturating_sub(DIVIDER + TEXT_MIN_W + PAD * 2);
    let natural_room = ART_W.saturating_sub(PAD * 2) as usize;
    let wide_picture = scene(sheet.panel, sheet.cursor, natural_room);
    if let Some(art) = wide_picture.fit(wide_room, room_h) {
        let left_w = ART_W.max(art.width() + PAD * 2);
        let frame_w = (BORDERS + left_w + DIVIDER + TEXT_W).min(width);
        let text_w = frame_w.saturating_sub(BORDERS + DIVIDER + left_w);
        if text_w >= TEXT_MIN_W {
            return draw_wide(buf, sheet, screen, art, (left_w, frame_w));
        }
    }
    let art_room = inner_w.saturating_sub(PAD * 2);
    let body_lines = body(sheet.panel, sheet.cursor, inner_w as usize)
        .lines
        .len();
    let text_floor = body_lines.min(STACK_TEXT_FLOOR);
    let stacked = scene(sheet.panel, sheet.cursor, art_room as usize);
    let art = stacked.fit(art_room, room_h.saturating_sub(text_floor as u16));
    draw_stacked(buf, sheet, screen, art);
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
    let content = art.height().max(body.lines.len() as u16);
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
    if art.height() <= left.height && art.width() <= left.width {
        art.draw_centred(buf, left);
    }
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

fn draw_stacked(buf: &mut Buffer, sheet: &Sheet, screen: Screen, art: Option<&Canvas>) {
    let (panel, cursor, scroll) = (sheet.panel, sheet.cursor, sheet.scroll);
    let width = screen.whole.width;
    let inner_w = width.saturating_sub(BORDERS);
    let body = body(panel, cursor, inner_w as usize);
    let band = art.map_or(0, Canvas::height);
    let calm = hints(panel, None);
    let hint_h = calm.height(inner_w);
    let rect = place(
        screen,
        width,
        band + body.lines.len() as u16 + RULE_ROWS + hint_h + BORDERS,
    );
    let inside = inner(rect);
    let window = inside.height.saturating_sub(hint_h + RULE_ROWS + band) as usize;
    let overflowing = body.lines.len() > window;
    let bar = hints(
        panel,
        overflowing.then_some((cursor.selected + 1, panel.rows.len())),
    );
    let hint_h = bar.height(inner_w).min(inside.height.saturating_sub(1));
    let text_h = inside.height.saturating_sub(hint_h + RULE_ROWS);
    let band = band.min(text_h);
    frame(buf, rect, &panel.title);
    let rule_y = inside.y + text_h;
    rule(buf, rect, rule_y);
    if let Some(art) = art {
        let area = Rect::new(inside.x, inside.y, inside.width, art.height());
        if art.height() <= text_h && art.width() <= area.width {
            art.draw_centred(buf, area);
        }
    }
    let body_h = text_h.saturating_sub(band);
    let total = body.lines.len();
    let shown = scroll.reveal(body.focus.clone(), body_h as usize, total);
    let window = Rect::new(inside.x, inside.y + band, inside.width, body_h);
    draw_lines(buf, &body.lines[shown.clone()], window);
    Scrollbar {
        x: rect.right().saturating_sub(BORDER),
        top: window.y,
        height: body_h,
    }
    .draw(buf, shown, total, WHITE);
    bar.draw(
        buf,
        Rect::new(inside.x, rule_y + RULE_ROWS, inside.width, hint_h),
        BACKGROUND,
    );
}
