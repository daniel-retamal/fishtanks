use std::cell::Cell;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

use crate::colors::{DARK_GRAY, GOLD, VIOLET, WHITE};
use crate::fishes::fish::{Direction, Fish};
use crate::fishes::toy::{
    FittedPart, Fittings, Line, Material, STITCH, Shelf, Signature, Slot, ToyColor, Toybox,
    size_name,
};
use crate::ui::{
    draw_fish_centred,
    hint_bar::HintBar,
    hints::{
        HINT_CLOSE, HINT_ENTER_SAVE, HINT_NAV, HINT_SCROLL, HINT_TAB_SHELF, HINT_TAB_TOYS,
        HINT_TOY_DISCARD, HINT_TOY_DRESS, HINT_TOY_PART, HINT_TOY_SLOT,
    },
    layout::{Screen, Scroll},
    panels::{PanelSpec, Panels, Reach},
    table,
};

const BACKGROUND: Color = Color::Reset;
const TITLE: &str = " Toybox ";
const EDIT_TITLE: &str = " Toybox#edit ";
const SHELF_TITLE: &str = " Toybox#shelf ";
const SIDE: (u16, u16) = (19, 6);
const BODY_W: u16 = 44;
const BODY_MIN_W: u16 = 24;
const PAD: u16 = 1;
const NAME_W: usize = 12;
const SLOT_W: usize = 7;
const MARK: &str = "> ";
const LEFT: &str = "◂ ";
const RIGHT: &str = " ▸";
const SWATCH: &str = "{xx}";
const UNSEEN: &str = "{··}";
const SECRET: &str = "???";
const SHINY: &str = "✦";
const DOT: char = '·';
const GAP: &str = "  ";
const CURRENT: &str = "• ";
const PARTS_SHOWN: usize = 6;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Toys,
    Edit,
    Shelf,
}

pub struct ToyEntry {
    pub name: String,
    pub fish: Fish,
}

pub struct ToyboxState {
    pub mode: Mode,
    pub toys: Vec<ToyEntry>,
    pub selected: usize,
    pub slot: usize,
    pub draft: Fittings,
    pub clock: f32,
    scroll: Scroll,
    shelf_scroll: Scroll,
}

impl ToyboxState {
    pub fn new(toys: Vec<ToyEntry>) -> Self {
        Self {
            mode: Mode::Toys,
            toys,
            selected: 0,
            slot: 0,
            draft: Fittings::default(),
            clock: 0.0,
            scroll: Scroll::default(),
            shelf_scroll: Scroll::default(),
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.clock += dt;
    }

    pub fn current(&self) -> Option<&ToyEntry> {
        self.toys.get(self.selected)
    }

    pub fn sealed(&self) -> bool {
        self.current()
            .and_then(|entry| entry.fish.toy.as_ref())
            .is_some_and(|toy| toy.is_sealed())
    }

    pub fn step(&mut self, down: bool) {
        match self.mode {
            Mode::Toys => {
                let n = self.toys.len();
                if n == 0 {
                    return;
                }
                self.selected = if down {
                    (self.selected + 1).min(n - 1)
                } else {
                    self.selected.saturating_sub(1)
                };
            }
            Mode::Edit => {
                let n = Slot::ALL.len();
                self.slot = if down {
                    (self.slot + 1) % n
                } else {
                    (self.slot + n - 1) % n
                };
            }
            Mode::Shelf => self.shelf_scroll.nudge(if down { 1 } else { -1 }),
        }
    }

    pub fn begin_edit(&mut self) -> bool {
        if self.sealed() {
            return false;
        }
        let Some(toy) = self.current().and_then(|e| e.fish.toy.as_ref()) else {
            return false;
        };
        self.draft = toy.fittings;
        self.slot = 0;
        self.mode = Mode::Edit;
        true
    }

    pub fn worn(&self) -> Fittings {
        self.current()
            .and_then(|e| e.fish.toy.as_ref())
            .map_or_else(Fittings::default, |toy| toy.fittings)
    }

    pub fn options(&self, toybox: &Toybox) -> Vec<Option<FittedPart>> {
        let slot = Slot::ALL[self.slot];
        let mut out = vec![None];
        let mut add = |part: FittedPart| {
            if !out.contains(&Some(part)) {
                out.push(Some(part));
            }
        };
        if let Some(part) = self.worn().get(slot) {
            add(part);
        }
        for part in toybox.for_slot(slot) {
            add(part);
        }
        out.sort_by_key(|option| option.map(|p| (p.part, p.paint)));
        out
    }

    pub fn choice(&self, options: &[Option<FittedPart>]) -> usize {
        let slot = Slot::ALL[self.slot];
        options
            .iter()
            .position(|option| *option == self.draft.get(slot))
            .unwrap_or(0)
    }

    pub fn turn_part(&mut self, toybox: &Toybox, ahead: bool) {
        let options = self.options(toybox);
        let slot = Slot::ALL[self.slot];
        let at = self.choice(&options);
        let n = options.len();
        let next = if ahead {
            (at + 1) % n
        } else {
            (at + n - 1) % n
        };
        self.draft.set(slot, options[next]);
    }

    pub fn shown(&self) -> Option<Fish> {
        let entry = self.current()?;
        let mut fish = entry.fish.portrait();
        fish.facing = Direction::Right;
        fish.habits.toy_clock = self.clock;
        if self.mode == Mode::Edit
            && let Some(toy) = fish.toy.as_mut()
        {
            toy.fittings = self.draft;
            fish.fit_the_toy();
        }
        Some(fish)
    }
}

pub struct ToyboxOverlay<'a> {
    state: &'a ToyboxState,
    toybox: &'a Toybox,
    screen: Screen,
}

impl<'a> ToyboxOverlay<'a> {
    pub fn new(state: &'a ToyboxState, toybox: &'a Toybox, screen: Screen) -> Self {
        Self {
            state,
            toybox,
            screen,
        }
    }
}

type Row = Vec<(String, Style)>;

fn plain(color: Color) -> Style {
    Style::default().fg(color).bg(BACKGROUND)
}

fn bold(color: Color) -> Style {
    plain(color).add_modifier(Modifier::BOLD)
}

fn toy_rows(state: &ToyboxState) -> Vec<Row> {
    state
        .toys
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let chosen = i == state.selected;
            let mark = if chosen { MARK } else { "  " };
            let Some(toy) = entry.fish.toy.as_ref() else {
                return Vec::new();
            };
            let name = table::ellipsize(&entry.name, NAME_W);
            let mut row = vec![
                (mark.to_string(), bold(WHITE)),
                (
                    format!("{name:<NAME_W$}"),
                    if chosen { bold(WHITE) } else { plain(WHITE) },
                ),
                (GAP.to_string(), plain(WHITE)),
                (toy.title(), plain(toy.paints().0.color())),
            ];
            if !toy.is_sealed() {
                row.push((
                    format!("{GAP}{}", size_name(entry.fish.size_category)),
                    plain(DARK_GRAY),
                ));
            }
            row
        })
        .collect()
}

fn slot_rows(state: &ToyboxState) -> Vec<Row> {
    let sealed = state.sealed();
    let fittings = state
        .shown()
        .and_then(|fish| fish.toy.map(|toy| toy.fittings()))
        .unwrap_or_default();
    Slot::ALL
        .iter()
        .enumerate()
        .map(|(i, slot)| {
            let chosen = i == state.slot && !sealed;
            let part = fittings.get(*slot);
            let label = part.map_or_else(|| table::NOTHING.to_string(), FittedPart::name);
            let color = match part {
                Some(_) if sealed => DARK_GRAY,
                Some(fitted) => fitted.paint.color(),
                None => DARK_GRAY,
            };
            let arrows = if sealed { ("  ", "  ") } else { (LEFT, RIGHT) };
            vec![
                (if chosen { MARK } else { "  " }.to_string(), bold(WHITE)),
                (
                    format!("{:<SLOT_W$}", slot.name()),
                    if chosen { bold(WHITE) } else { plain(WHITE) },
                ),
                (arrows.0.to_string(), plain(WHITE)),
                (label, if chosen { bold(color) } else { plain(color) }),
                (arrows.1.to_string(), plain(WHITE)),
            ]
        })
        .collect()
}

struct EditRows {
    rows: Vec<Row>,
    choice_row: usize,
}

fn edit_rows(state: &ToyboxState, toybox: &Toybox, width: usize, listed: usize) -> EditRows {
    let mut rows = slot_rows(state);
    if listed == 0 {
        return EditRows {
            rows,
            choice_row: state.slot,
        };
    }
    let slot = Slot::ALL[state.slot];
    let options = state.options(toybox);
    let at = state.choice(&options);
    let shown = options.len().min(listed);
    let first = at.saturating_sub(shown / 2).min(options.len() - shown);
    rows.push(Vec::new());
    rows.push(vec![(format!("{} parts", slot.name()), bold(WHITE))]);
    let choice_row = rows.len() + at - first;
    let worn = state.worn().get(slot);
    for (i, option) in options.iter().enumerate().skip(first).take(shown) {
        let chosen = i == at;
        let mark = if chosen { CURRENT } else { "  " };
        let Some(part) = option else {
            rows.push(vec![
                (mark.to_string(), bold(WHITE)),
                (table::NOTHING.to_string(), plain(DARK_GRAY)),
            ]);
            continue;
        };
        let label = part.name();
        let color = part.paint.color();
        let count = toybox.count(*part) + u32::from(worn == Some(*part));
        let tally = format!("×{count}");
        let used = table::visual_width(mark) + table::visual_width(&label);
        let pad = width
            .saturating_sub(used + table::visual_width(&tally))
            .max(1);
        rows.push(vec![
            (mark.to_string(), bold(WHITE)),
            (label, if chosen { bold(color) } else { plain(color) }),
            (" ".repeat(pad), plain(WHITE)),
            (tally, plain(DARK_GRAY)),
        ]);
    }
    EditRows { rows, choice_row }
}

fn wrap_tokens(tokens: Vec<Row>, width: usize) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    let mut row: Row = Vec::new();
    let mut used = 0;
    for token in tokens {
        let w: usize = token.iter().map(|(s, _)| table::visual_width(s)).sum();
        if used > 0 && used + GAP.len() + w > width {
            rows.push(std::mem::take(&mut row));
            used = 0;
        }
        if used > 0 {
            row.push((GAP.to_string(), plain(WHITE)));
            used += GAP.len();
        }
        used += w;
        row.extend(token);
    }
    if !row.is_empty() {
        rows.push(row);
    }
    rows
}

fn swatch(color: ToyColor, shelf: &Shelf) -> Row {
    if !shelf.colors.contains(&color) {
        return vec![(UNSEEN.to_string(), plain(DARK_GRAY))];
    }
    let (a, b) = color.paints();
    SWATCH
        .chars()
        .enumerate()
        .map(|(i, ch)| {
            let paint = if ch == STITCH && i % 2 == 0 { b } else { a };
            (ch.to_string(), plain(paint.color()))
        })
        .collect()
}

fn signature_token(signature: Signature, shelf: &Shelf, secret: bool) -> Row {
    let won = shelf.signatures.contains(&signature);
    if secret && !won {
        let lit = shelf.secrets_unlocked().contains(&signature);
        return vec![(
            SECRET.to_string(),
            plain(if lit { GOLD } else { DARK_GRAY }),
        )];
    }
    if !won {
        let dots: String = signature.name().chars().map(|_| DOT).collect();
        return vec![(dots, plain(DARK_GRAY))];
    }
    let paint = crate::fishes::toy::ToyState::signature(signature, false)
        .paints()
        .0
        .color();
    let mut token = vec![(signature.name().to_string(), bold(paint))];
    if !secret {
        let shiny = shelf.shinies.contains(&signature);
        token.push((
            SHINY.to_string(),
            plain(if shiny { VIOLET } else { DARK_GRAY }),
        ));
    }
    token
}

fn shelf_rows(shelf: &Shelf, width: usize) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    rows.push(vec![(
        format!("Colors {}/{}", shelf.colors.len(), ToyColor::ALL.len()),
        bold(WHITE),
    )]);
    let mut swatches: Vec<Row> = ToyColor::ALL.iter().map(|c| swatch(*c, shelf)).collect();
    swatches.push(if shelf.golden {
        vec![(SWATCH.to_string(), bold(GOLD))]
    } else {
        vec![(SECRET.to_string(), plain(DARK_GRAY))]
    });
    rows.extend(wrap_tokens(swatches, width));
    rows.push(Vec::new());
    rows.push(vec![("Materials".to_string(), bold(WHITE))]);
    let materials: Vec<Row> = Material::ALL
        .iter()
        .map(|m| {
            let seen = shelf.materials.contains(m);
            vec![(
                m.name().to_string(),
                plain(if seen { WHITE } else { DARK_GRAY }),
            )]
        })
        .collect();
    rows.extend(wrap_tokens(materials, width));
    for line in Line::ALL {
        rows.push(Vec::new());
        let won = line
            .signatures()
            .iter()
            .filter(|s| shelf.signatures.contains(s))
            .count();
        rows.push(vec![(
            format!("{} {}/{}", line.name(), won, line.signatures().len()),
            bold(WHITE),
        )]);
        let mut tokens: Vec<Row> = line
            .signatures()
            .into_iter()
            .map(|s| signature_token(s, shelf, false))
            .collect();
        tokens.push(signature_token(line.secret(), shelf, true));
        rows.extend(wrap_tokens(tokens, width));
    }
    rows
}

fn hints(state: &ToyboxState, toybox: &Toybox, overflowing: bool) -> HintBar {
    match state.mode {
        Mode::Toys => HintBar::new(HINT_CLOSE)
            .counted(
                HINT_NAV,
                overflowing.then_some((state.selected + 1, state.toys.len())),
            )
            .action_if(!state.sealed() && !state.toys.is_empty(), HINT_TOY_DRESS)
            .action(HINT_TAB_SHELF),
        Mode::Edit if state.sealed() => HintBar::new(HINT_CLOSE),
        Mode::Edit => {
            let options = state.options(toybox);
            let at = state.choice(&options);
            HintBar::new(HINT_TOY_DISCARD)
                .action(HINT_TOY_SLOT)
                .counted(
                    HINT_TOY_PART,
                    (options.len() > 1).then_some((at + 1, options.len())),
                )
                .action(HINT_ENTER_SAVE)
        }
        Mode::Shelf => HintBar::new(HINT_CLOSE)
            .action_if(overflowing, HINT_SCROLL)
            .action(HINT_TAB_TOYS),
    }
}

fn widest_hints() -> u16 {
    [
        HintBar::new(HINT_CLOSE)
            .counted(HINT_NAV, Some((99, 99)))
            .action(HINT_TOY_DRESS)
            .action(HINT_TAB_SHELF),
        HintBar::new(HINT_TOY_DISCARD)
            .action(HINT_TOY_SLOT)
            .counted(HINT_TOY_PART, Some((99, 99)))
            .action(HINT_ENTER_SAVE),
        HintBar::new(HINT_CLOSE)
            .action(HINT_SCROLL)
            .action(HINT_TAB_TOYS),
    ]
    .iter()
    .map(HintBar::natural_width)
    .max()
    .unwrap_or(0)
}

fn draw_row(buf: &mut Buffer, x: u16, y: u16, width: u16, row: &Row) {
    let mut col = x;
    let right = x + width;
    for (text, style) in row {
        if col >= right {
            break;
        }
        let room = (right - col) as usize;
        let shown = table::ellipsize(text, room);
        buf.set_stringn(col, y, &shown, room, *style);
        col += table::visual_width(&shown) as u16;
    }
}

impl Widget for ToyboxOverlay<'_> {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        let state = self.state;
        let text_w = |body_w: u16| body_w.saturating_sub(PAD * 2).max(1);
        let rows_for = |body_w: u16, listed: usize| -> Vec<Row> {
            match state.mode {
                Mode::Toys => toy_rows(state),
                Mode::Edit => edit_rows(state, self.toybox, text_w(body_w) as usize, listed).rows,
                Mode::Shelf => shelf_rows(&self.toybox.shelf, text_w(body_w) as usize),
            }
        };
        let title = match state.mode {
            Mode::Toys => TITLE,
            Mode::Edit => EDIT_TITLE,
            Mode::Shelf => SHELF_TITLE,
        };
        let spec_for = |bar, count| PanelSpec {
            title,
            title_style: bold(WHITE),
            border: plain(WHITE),
            background: BACKGROUND,
            side: SIDE,
            body_w: BODY_W.max(widest_hints()),
            body_min_w: BODY_MIN_W,
            body_rows: count,
            hints: bar,
            reach: Reach::Full,
        };
        let calm = hints(state, self.toybox, false);
        let toy_rows_tall = state
            .shown()
            .map_or(0, |fish| fish.line_sprite().rows.len() as u16)
            .min(SIDE.1);
        let listed = Cell::new(PARTS_SHOWN);
        let count = |body_w: u16| rows_for(body_w, listed.get()).len().max(1) as u16;
        let measured = loop {
            let measured = Panels::measure(self.screen, &spec_for(&calm, &count));
            if !measured.stacked || measured.side.height >= toy_rows_tall || listed.get() == 0 {
                break measured;
            }
            listed.set(listed.get() - 1);
        };
        let listed = listed.get();
        let total = count(measured.body.width) as usize;
        let overflowing = total > measured.body.height as usize;
        let bar = hints(state, self.toybox, overflowing);
        let panels = Panels::open(buf, self.screen, &spec_for(&bar, &count));
        if let Some(fish) = state.shown() {
            draw_fish_centred(buf, &fish, panels.side, BACKGROUND);
        }
        if state.toys.is_empty() && state.mode == Mode::Toys {
            draw_row(
                buf,
                panels.body.x + PAD,
                panels.body.y,
                text_w(panels.body.width),
                &vec![("Toyfish come from the Claw.".to_string(), plain(DARK_GRAY))],
            );
            return;
        }
        let rows = rows_for(panels.body.width, listed);
        let room = panels.body.height as usize;
        let heights = vec![1; rows.len()];
        let shown = match state.mode {
            Mode::Toys => state.scroll.follow(&heights, state.selected, room),
            Mode::Edit => {
                let choice = edit_rows(
                    state,
                    self.toybox,
                    text_w(panels.body.width) as usize,
                    listed,
                )
                .choice_row;
                state
                    .scroll
                    .reveal(state.slot..choice + 1, room, rows.len())
            }
            Mode::Shelf => {
                let last = rows.len().saturating_sub(room);
                let first = state.shelf_scroll.settle(last);
                first..(first + room).min(rows.len())
            }
        };
        for (line, index) in shown.clone().take(room).enumerate() {
            if let Some(row) = rows.get(index) {
                draw_row(
                    buf,
                    panels.body.x + PAD,
                    panels.body.y + line as u16,
                    text_w(panels.body.width),
                    row,
                );
            }
        }
    }
}
