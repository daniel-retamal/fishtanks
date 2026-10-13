use std::ops::Range;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

use crate::colors::{BLACK, DARK_GRAY, STEEL, WHITE};
use crate::fishes::botfish::{BotfishState, level_color};
use crate::fishes::chip::Chip;
use crate::fishes::fish::Fish;
use crate::fishes::parts::{Part, Pin, PinDirection, PinOwner};
use crate::tank::Netlist;
use crate::ui::hint_bar::HintBar;
use crate::ui::hints::{
    HINT_CLOSE, HINT_ENTER_WIRE, HINT_SELECT, HINT_TAB_SCHEMATIC, HINT_TAB_SIGNAL,
};
use crate::ui::layout::{FlexItem, PanBar, Screen, Scroll, Scrollbar, shrink};
use crate::ui::modal::{Frame, Modal};
use crate::ui::table::{self, NOTHING};

const SIGNAL_HIGH: char = '●';
const SIGNAL_LOW: char = '○';
const CHANNEL_JOIN: &str = ", ";
const PIN_BRANCH: char = '├';
const LAST_PIN_BRANCH: char = '└';
const INPUT_ARROW: char = '◂';
const OUTPUT_ARROW: char = '▸';
const OWNER_JOIN: char = '.';
const LEVEL_INDENT: u16 = 2;
const LEVEL_W: u16 = 1;
const COL_GAP: u16 = 2;
const EDGE_PAD: u16 = 1;
const ROW_PREFIX_W: u16 = LEVEL_INDENT + LEVEL_W + COL_GAP;
const GAPS_W: u16 = COL_GAP * (COLUMN_COUNT as u16 - 1);
const HEADER_ROWS: u16 = 1;
const MIN_DATA_ROWS: u16 = 1;
const NOTICE_GAP_ROWS: u16 = 1;
const BACKGROUND: Color = Color::Reset;

const TITLE_SIGNAL: &str = " Circuit#signal ";
const TITLE_SCHEMATIC: &str = " Circuit#schematic ";
const FISH_HEADER: &str = "Fish";
const PARTS_HEADER: &str = "Parts";
const LISTENS_HEADER: &str = "Listens";
const DRIVES_HEADER: &str = "Drives";
const FISH_COLUMN: usize = 0;
const PARTS_COLUMN: usize = 1;
const LISTENS_COLUMN: usize = 2;
const COLUMN_COUNT: usize = 4;
const HEADERS: [&str; COLUMN_COUNT] = [FISH_HEADER, PARTS_HEADER, LISTENS_HEADER, DRIVES_HEADER];
const GIVE_ORDER: [u8; COLUMN_COUNT] = [3, 0, 1, 2];

const SCHEMATIC_MAX_FISH: usize = 24;
const SCHEMATIC_MAX_COLUMNS: usize = 8;
const NODE_OPEN: char = '[';
const NODE_CLOSE: char = ']';
const WIRE_ARROW: char = '▶';
const WIRE_COLOR: Color = DARK_GRAY;
pub(crate) const OFF_VIEW_VERTICAL: char = '┊';
pub(crate) const OFF_VIEW_HORIZONTAL: char = '┈';
const GUTTER_PAD: usize = 3;
const GRAPH_MARGIN: usize = 1;

pub(crate) const NORTH: u8 = 1;
pub(crate) const SOUTH: u8 = 2;
pub(crate) const WEST: u8 = 4;
pub(crate) const EAST: u8 = 8;
const ARROW: u8 = 16;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CircuitView {
    Signal,
    Schematic,
}

pub struct CircuitFish {
    pub name: String,
    listens: Vec<String>,
    drives: Option<String>,
    produces: Vec<String>,
    consumes: Vec<String>,
    parts: Vec<String>,
    pins: Vec<PinRow>,
    output_level: bool,
}

struct PinRow {
    label: String,
    channel: Option<String>,
    direction: PinDirection,
}

impl PinRow {
    fn arrow(&self) -> char {
        match self.direction {
            PinDirection::In => INPUT_ARROW,
            PinDirection::Out => OUTPUT_ARROW,
        }
    }
}

impl CircuitFish {
    pub fn of(fish: &Fish) -> Option<Self> {
        fish.script().map(|bot| Self::from_design(&fish.name, bot))
    }

    pub fn from_design(name: &str, bot: &BotfishState) -> Self {
        let declared: Vec<Pin> = bot
            .declared_pins()
            .into_iter()
            .filter(|pin| !pin.is_output())
            .collect();
        let pins = declared
            .iter()
            .map(|pin| {
                let channel = bot.pins().channel_of(pin).map(str::to_string);
                let shared = declared
                    .iter()
                    .filter(|other| other.name == pin.name)
                    .count()
                    > 1;
                let label = if shared {
                    format!(
                        "{}{OWNER_JOIN}{}",
                        owner_name(bot, pin.owner, name),
                        pin.name
                    )
                } else {
                    pin.name.to_string()
                };
                PinRow {
                    label,
                    channel,
                    direction: pin.direction,
                }
            })
            .collect();
        let parts = bot
            .parts()
            .iter()
            .map(|(part, count)| stacked_name(part, count))
            .chain(bot.chip().map(Chip::display_name))
            .collect();
        Self {
            name: name.to_string(),
            listens: bot.listens().map(str::to_string).collect(),
            drives: bot.drives().map(str::to_string),
            produces: bot.driven_lines(),
            consumes: bot.fed_lines(),
            parts,
            pins,
            output_level: bot.output_level(),
        }
    }

    fn height(&self) -> usize {
        1 + self.pins.len()
    }

    fn parts_text(&self) -> String {
        if self.parts.is_empty() {
            return NOTHING.to_string();
        }
        self.parts.join(CHANNEL_JOIN)
    }

    fn listens_text(&self) -> String {
        if self.listens.is_empty() {
            return NOTHING.to_string();
        }
        self.listens.join(CHANNEL_JOIN)
    }

    fn drives_text(&self) -> String {
        self.drives.clone().unwrap_or_else(|| NOTHING.to_string())
    }

    fn pin_text(&self, index: usize) -> String {
        let row = &self.pins[index];
        let branch = if index + 1 == self.pins.len() {
            LAST_PIN_BRANCH
        } else {
            PIN_BRANCH
        };
        format!(
            "{} {} {} {}",
            branch,
            row.label,
            row.arrow(),
            row.channel.as_deref().unwrap_or(NOTHING)
        )
    }

    fn feeds(&self, listener: &CircuitFish) -> impl Iterator<Item = &str> {
        self.produces
            .iter()
            .filter(|line| listener.consumes.contains(line))
            .map(String::as_str)
    }

    fn node_label(&self) -> String {
        format!("{}{}{}", NODE_OPEN, self.name, NODE_CLOSE)
    }
}

pub fn owner_name(bot: &BotfishState, owner: PinOwner, fish_name: &str) -> String {
    match owner {
        PinOwner::Part(part) => part.display_name().to_string(),
        PinOwner::Body => bot
            .chip()
            .map_or_else(|| fish_name.to_string(), Chip::display_name),
    }
}

pub fn stacked_name(part: Part, count: u32) -> String {
    stacked_label(part.display_name(), count)
}

pub fn stacked_label(name: &str, count: u32) -> String {
    if count > 1 {
        return format!("{name} ×{count}");
    }
    name.to_string()
}

fn depths(fish: &[CircuitFish]) -> Vec<usize> {
    let produces: Vec<&Vec<String>> = fish.iter().map(|f| &f.produces).collect();
    let consumes: Vec<&Vec<String>> = fish.iter().map(|f| &f.consumes).collect();
    let edges = Netlist::of(&produces, &consumes).settle().feed_forward;
    let mut depth = vec![0usize; fish.len()];
    for _ in 0..fish.len() {
        let mut changed = false;
        for &(driver, listener) in &edges {
            if depth[listener] > depth[driver] {
                continue;
            }
            depth[listener] = depth[driver] + 1;
            changed = true;
        }
        if !changed {
            break;
        }
    }
    depth
}

fn column_count(fish: &[CircuitFish]) -> usize {
    depths(fish)
        .into_iter()
        .max()
        .map_or(0, |deepest| deepest + 1)
}

pub fn fits_schematic(fish: &[CircuitFish]) -> bool {
    fish.len() <= SCHEMATIC_MAX_FISH && column_count(fish) <= SCHEMATIC_MAX_COLUMNS
}

pub fn too_big_to_draw(fish: &[CircuitFish]) -> String {
    format!(
        "{} fish {} deep is too big to draw; the schematic holds {} fish and {} columns",
        fish.len(),
        column_count(fish),
        SCHEMATIC_MAX_FISH,
        SCHEMATIC_MAX_COLUMNS
    )
}

pub fn sorted_by_depth(fish: Vec<CircuitFish>) -> Vec<CircuitFish> {
    let mut paired: Vec<(usize, CircuitFish)> = depths(&fish).into_iter().zip(fish).collect();
    paired.sort_by(|(left_depth, left), (right_depth, right)| {
        left_depth
            .cmp(right_depth)
            .then_with(|| left.name.cmp(&right.name))
    });
    paired.into_iter().map(|(_, fish)| fish).collect()
}

pub struct CircuitState {
    pub tank_idx: usize,
    pub fish: Vec<CircuitFish>,
    pub selected: usize,
    view: CircuitView,
    refused: bool,
    rows: Scroll,
    pan: Scroll,
}

impl CircuitState {
    pub fn new(tank_idx: usize, fish: Vec<CircuitFish>) -> Option<Self> {
        if fish.is_empty() {
            return None;
        }
        Some(Self {
            tank_idx,
            fish: sorted_by_depth(fish),
            selected: 0,
            view: CircuitView::Signal,
            refused: false,
            rows: Scroll::default(),
            pan: Scroll::default(),
        })
    }

    pub fn refresh_levels(&mut self, fish: &[Fish]) {
        for row in &mut self.fish {
            row.output_level = fish
                .iter()
                .find(|f| f.name == row.name)
                .and_then(|f| f.script())
                .is_some_and(|bot| bot.output_level());
        }
    }

    pub fn select(&mut self, name: &str) {
        if let Some(pos) = self.fish.iter().position(|f| f.name == name) {
            self.selected = pos;
        }
    }

    pub fn selected_fish_name(&self) -> &str {
        self.fish[self.selected].name.as_str()
    }

    pub fn toggle_view(&mut self) {
        self.rows.reset();
        self.pan.reset();
        if self.view == CircuitView::Schematic {
            self.view = CircuitView::Signal;
            return;
        }
        self.refused = !self.fits_schematic();
        if !self.refused {
            self.view = CircuitView::Schematic;
        }
    }

    pub fn restore_view(&mut self, previous: &CircuitState) {
        if previous.view != CircuitView::Schematic || !self.fits_schematic() {
            return;
        }
        self.view = CircuitView::Schematic;
    }

    fn fits_schematic(&self) -> bool {
        fits_schematic(&self.fish)
    }

    fn notice(&self) -> Option<String> {
        if !self.refused || self.view == CircuitView::Schematic {
            return None;
        }
        Some(too_big_to_draw(&self.fish))
    }

    fn tab_hint(showing_schematic: bool) -> &'static str {
        if showing_schematic {
            return HINT_TAB_SIGNAL;
        }
        HINT_TAB_SCHEMATIC
    }

    fn title(&self) -> &'static str {
        match self.view {
            CircuitView::Signal => TITLE_SIGNAL,
            CircuitView::Schematic => TITLE_SCHEMATIC,
        }
    }

    pub fn scroll_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        if self.selected + 1 < self.fish.len() {
            self.selected += 1;
        }
    }

    fn line_heights(&self) -> Vec<usize> {
        self.fish.iter().map(CircuitFish::height).collect()
    }

    fn hints(&self, overflowing: bool) -> HintBar {
        let showing_schematic = self.view == CircuitView::Schematic;
        HintBar::new(HINT_CLOSE)
            .counted(
                HINT_SELECT,
                overflowing.then_some((self.selected + 1, self.fish.len())),
            )
            .action(HINT_ENTER_WIRE)
            .action(Self::tab_hint(showing_schematic))
    }
}

impl CircuitFish {
    fn cell(&self, column: usize) -> String {
        match column {
            FISH_COLUMN => self.name.clone(),
            PARTS_COLUMN => self.parts_text(),
            LISTENS_COLUMN => self.listens_text(),
            _ => self.drives_text(),
        }
    }
}

struct Columns {
    x: u16,
    widths: Vec<u16>,
}

impl Columns {
    fn items(state: &CircuitState) -> Vec<FlexItem> {
        (0..COLUMN_COUNT)
            .map(|column| {
                let header_w = table::visual_width(HEADERS[column]) as u16;
                let widest = state
                    .fish
                    .iter()
                    .map(|fish| table::visual_width(&fish.cell(column)) as u16)
                    .max()
                    .unwrap_or(0)
                    .max(header_w);
                FlexItem::new(widest, header_w).gives_in_turn(GIVE_ORDER[column])
            })
            .collect()
    }

    fn natural_width(items: &[FlexItem]) -> u16 {
        ROW_PREFIX_W + items.iter().map(|item| item.basis).sum::<u16>() + GAPS_W + EDGE_PAD
    }

    fn fit(state: &CircuitState, x: u16, inner_w: u16) -> Self {
        let room = inner_w.saturating_sub(ROW_PREFIX_W + GAPS_W + EDGE_PAD);
        Self {
            x: x + ROW_PREFIX_W,
            widths: shrink(&Self::items(state), room),
        }
    }

    fn cell_x(&self, column: usize) -> u16 {
        self.x + self.widths[..column].iter().sum::<u16>() + COL_GAP * column as u16
    }
}

pub struct CircuitOverlay<'a> {
    state: &'a CircuitState,
    screen: Screen,
}

impl<'a> CircuitOverlay<'a> {
    pub fn new(state: &'a CircuitState, screen: Screen) -> Self {
        Self { state, screen }
    }

    fn frame(title: &str) -> Frame<'_> {
        Frame {
            title,
            border: WHITE,
            background: BACKGROUND,
        }
    }

    fn render_signal(&self, buf: &mut Buffer) {
        let state = self.state;
        let title = state.title();
        let natural_w = Columns::natural_width(&Columns::items(state));
        let inner_w = Modal::inner_width(self.screen, title, natural_w, &state.hints(true));
        let notice_lines = state
            .notice()
            .map(|text| table::wrap_words(&text, inner_w.saturating_sub(EDGE_PAD * 2) as usize))
            .unwrap_or_default();
        let heights = state.line_heights();
        let total: usize = heights.iter().sum();
        let notice_block = if notice_lines.is_empty() {
            0
        } else {
            NOTICE_GAP_ROWS + notice_lines.len() as u16
        };
        let content_h = HEADER_ROWS + total as u16 + notice_block;
        let (modal, _) = Modal::open_fitting(
            buf,
            self.screen,
            &Self::frame(title),
            (inner_w, content_h),
            |overflowing| state.hints(overflowing),
        );
        let body = modal.body;
        if body.height == 0 {
            return;
        }
        let cols = Columns::fit(state, body.x, body.width);
        draw_header(buf, &cols, body);
        let below_header = body.height.saturating_sub(HEADER_ROWS);
        let notice_rows = notice_block.min(below_header.saturating_sub(MIN_DATA_ROWS));
        let data = Rect::new(
            body.x,
            body.y + HEADER_ROWS.min(body.height),
            body.width,
            below_header - notice_rows,
        );
        let notice_style = Style::default().fg(STEEL).bg(BACKGROUND);
        let shown_notice = notice_rows.saturating_sub(NOTICE_GAP_ROWS) as usize;
        for (row, line) in notice_lines.iter().take(shown_notice).enumerate() {
            let y = data.bottom() + NOTICE_GAP_ROWS + row as u16;
            buf.set_stringn(
                body.x + EDGE_PAD,
                y,
                line,
                body.width.saturating_sub(EDGE_PAD * 2) as usize,
                notice_style,
            );
        }
        let selected_start: usize = heights[..state.selected].iter().sum();
        let lines = state.rows.reveal(
            selected_start..selected_start + heights[state.selected],
            data.height as usize,
            total,
        );
        let mut line = 0;
        let mut y = data.y;
        for (index, fish) in state.fish.iter().enumerate() {
            for row in 0..fish.height() {
                if !lines.contains(&line) {
                    line += 1;
                    continue;
                }
                let row_line = RowLine {
                    cols: &cols,
                    body,
                    y,
                    selected: index == state.selected,
                };
                if row == 0 {
                    row_line.draw_fish(buf, fish);
                } else {
                    row_line.draw_pin(buf, &fish.pin_text(row - 1));
                }
                line += 1;
                y += 1;
            }
        }
        Scrollbar {
            x: modal.scrollbar_x(),
            top: data.y,
            height: data.height,
        }
        .draw(buf, lines, total, WHITE);
    }

    fn render_schematic(&self, buf: &mut Buffer) {
        let state = self.state;
        let title = state.title();
        let schematic = Schematic::of(&state.fish);
        let content = (
            (schematic.width + GRAPH_MARGIN * 2) as u16,
            (GRAPH_MARGIN + schematic.height) as u16,
        );
        let (modal, _) = Modal::open_fitting(
            buf,
            self.screen,
            &Self::frame(title),
            content,
            |overflowing| state.hints(overflowing),
        );
        let body = modal.body;
        if body.height == 0 || body.width == 0 {
            return;
        }
        let top_margin = (GRAPH_MARGIN as u16).min(body.height - 1);
        let side_margin = if body.width as usize > schematic.width {
            (body.width as usize - schematic.width) / 2
        } else {
            0
        };
        let view = Rect::new(
            body.x + side_margin as u16,
            body.y + top_margin,
            body.width.min(schematic.width as u16),
            body.height - top_margin,
        );
        let mut heights = vec![1; state.fish.len()];
        if let Some(last) = heights.last_mut() {
            *last += schematic.height - state.fish.len();
        }
        let shown = state
            .rows
            .follow(&heights, state.selected, view.height as usize);
        let rows = shown.start..(shown.start + view.height as usize).min(schematic.height);
        let node = &schematic.nodes[state.selected];
        let columns = state.pan.reveal(
            node.gutter_x..node.x + table::visual_width(&node.label),
            view.width as usize,
            schematic.width,
        );
        schematic.draw(
            buf,
            view,
            (rows.clone(), columns.clone()),
            Some(state.selected),
        );
        Scrollbar {
            x: modal.scrollbar_x(),
            top: view.y,
            height: view.height,
        }
        .draw(buf, rows, schematic.height, WHITE);
        PanBar {
            y: modal.rect.bottom() - 1,
            left: view.x,
            width: view.width,
        }
        .draw(buf, columns, schematic.width, WHITE);
    }
}

impl Widget for CircuitOverlay<'_> {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        match self.state.view {
            CircuitView::Schematic => self.render_schematic(buf),
            CircuitView::Signal => self.render_signal(buf),
        }
    }
}

fn draw_header(buf: &mut Buffer, cols: &Columns, body: Rect) {
    let bold = Style::default()
        .fg(WHITE)
        .add_modifier(Modifier::BOLD)
        .bg(BACKGROUND);
    let level_x = body.x + LEVEL_INDENT;
    if level_x < body.right() {
        buf[(level_x, body.y)].set_char(SIGNAL_HIGH).set_style(bold);
    }
    let cells = HEADERS.map(str::to_string);
    draw_cells(buf, cols, &cells, body.right(), body.y, bold);
}

fn draw_cells(
    buf: &mut Buffer,
    cols: &Columns,
    cells: &[String; COLUMN_COUNT],
    right_edge: u16,
    y: u16,
    style: Style,
) {
    for (column, text) in cells.iter().enumerate() {
        let x = cols.cell_x(column);
        if x >= right_edge {
            return;
        }
        let room = cols.widths[column].min(right_edge - x) as usize;
        buf.set_stringn(x, y, table::ellipsize(text, room), room, style);
    }
}

struct RowLine<'a> {
    cols: &'a Columns,
    body: Rect,
    y: u16,
    selected: bool,
}

impl RowLine<'_> {
    fn background(&self) -> Color {
        if self.selected { WHITE } else { BACKGROUND }
    }

    fn fill(&self, buf: &mut Buffer) {
        for x in self.body.x..self.body.right() {
            buf[(x, self.y)].set_char(' ').set_bg(self.background());
        }
    }

    fn draw_fish(&self, buf: &mut Buffer, fish: &CircuitFish) {
        let row_bg = self.background();
        let fg = if self.selected { BLACK } else { STEEL };
        self.fill(buf);
        let level_x = self.body.x + LEVEL_INDENT;
        if level_x < self.body.right() {
            let signal = if fish.output_level {
                SIGNAL_HIGH
            } else {
                SIGNAL_LOW
            };
            buf[(level_x, self.y)]
                .set_char(signal)
                .set_fg(level_color(fish.output_level))
                .set_bg(row_bg);
        }
        let cells = std::array::from_fn(|column| fish.cell(column));
        draw_cells(
            buf,
            self.cols,
            &cells,
            self.body.right(),
            self.y,
            Style::default().fg(fg).bg(row_bg),
        );
    }

    fn draw_pin(&self, buf: &mut Buffer, text: &str) {
        let fg = if self.selected { BLACK } else { DARK_GRAY };
        self.fill(buf);
        let x = self.cols.cell_x(FISH_COLUMN);
        let right_edge = self.body.right().saturating_sub(EDGE_PAD);
        if x >= right_edge {
            return;
        }
        let room = (right_edge - x) as usize;
        buf.set_stringn(
            x,
            self.y,
            table::ellipsize(text, room),
            room,
            Style::default().fg(fg).bg(self.background()),
        );
    }
}

pub(crate) fn wire_char(mask: u8) -> Option<char> {
    if mask & ARROW != 0 {
        return Some(WIRE_ARROW);
    }
    let north = mask & NORTH != 0;
    let south = mask & SOUTH != 0;
    let west = mask & WEST != 0;
    let east = mask & EAST != 0;
    Some(match (north, south, west, east) {
        (false, false, false, false) => return None,
        (true, true, true, true) => '┼',
        (true, true, true, false) => '┤',
        (true, true, false, true) => '├',
        (true, false, true, true) => '┴',
        (false, true, true, true) => '┬',
        (true, false, true, false) => '┘',
        (true, false, false, true) => '└',
        (false, true, true, false) => '┐',
        (false, true, false, true) => '┌',
        (true, true, false, false) | (true, false, false, false) | (false, true, false, false) => {
            '│'
        }
        (false, false, true, true) | (false, false, true, false) | (false, false, false, true) => {
            '─'
        }
    })
}

struct Wires {
    width: usize,
    cells: Vec<u8>,
}

impl Wires {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            cells: vec![0; width * height],
        }
    }

    fn add(&mut self, x: usize, y: usize, bits: u8) {
        let Some(cell) = self.cells.get_mut(y * self.width + x) else {
            return;
        };
        *cell |= bits;
    }

    fn horizontal(&mut self, row: usize, from_x: usize, to_x: usize) {
        let (start, end) = (from_x.min(to_x), from_x.max(to_x));
        for x in start..=end {
            let mut bits = 0;
            if x > start {
                bits |= WEST;
            }
            if x < end {
                bits |= EAST;
            }
            self.add(x, row, bits);
        }
    }

    fn vertical(&mut self, column: usize, from_y: usize, to_y: usize) {
        let (start, end) = (from_y.min(to_y), from_y.max(to_y));
        for y in start..=end {
            let mut bits = 0;
            if y > start {
                bits |= NORTH;
            }
            if y < end {
                bits |= SOUTH;
            }
            self.add(column, y, bits);
        }
    }

    fn arrow(&mut self, x: usize, y: usize) {
        self.add(x, y, ARROW);
    }

    #[cfg(test)]
    fn char_at(&self, x: usize, y: usize) -> Option<char> {
        wire_char(self.mask(x, y))
    }

    fn mask(&self, x: usize, y: usize) -> u8 {
        if x >= self.width {
            return 0;
        }
        self.cells.get(y * self.width + x).copied().unwrap_or(0)
    }

    fn char_in_view(&self, x: usize, y: usize, window: &Window) -> Option<char> {
        let mask = self.mask(x, y);
        let vertical_only = mask & (WEST | EAST | ARROW) == 0;
        let horizontal_only = mask & (NORTH | SOUTH | ARROW) == 0;
        let leaves_top = y == window.rows.start && window.rows.start > 0 && mask & NORTH != 0;
        let leaves_bottom =
            y + 1 == window.rows.end && window.rows.end < window.height && mask & SOUTH != 0;
        let leaves_left = x == window.columns.start && window.columns.start > 0 && mask & WEST != 0;
        let leaves_right =
            x + 1 == window.columns.end && window.columns.end < window.width && mask & EAST != 0;
        if (leaves_top || leaves_bottom) && vertical_only {
            return Some(OFF_VIEW_VERTICAL);
        }
        if (leaves_left || leaves_right) && horizontal_only {
            return Some(OFF_VIEW_HORIZONTAL);
        }
        let off_view = [
            (leaves_top, NORTH),
            (leaves_bottom, SOUTH),
            (leaves_left, WEST),
            (leaves_right, EAST),
        ]
        .into_iter()
        .filter(|(leaves, _)| *leaves)
        .fold(0, |bits, (_, arm)| bits | arm);
        wire_char(mask & !off_view)
    }
}

struct Window {
    rows: Range<usize>,
    columns: Range<usize>,
    width: usize,
    height: usize,
}

struct Edge {
    from: usize,
    to: usize,
    channel: String,
    backward: bool,
}

fn edges_of(fish: &[CircuitFish], depth: &[usize]) -> Vec<Edge> {
    let mut edges = Vec::new();
    for (from, driver) in fish.iter().enumerate() {
        for (to, listener) in fish.iter().enumerate() {
            for channel in driver.feeds(listener) {
                edges.push(Edge {
                    from,
                    to,
                    channel: channel.to_string(),
                    backward: depth[to] <= depth[from],
                });
            }
        }
    }
    edges
}

struct Gutters {
    lanes: Vec<Vec<String>>,
    gutter_x: Vec<usize>,
    column_x: Vec<usize>,
    width: usize,
}

impl Gutters {
    fn new(columns: usize) -> Self {
        Self {
            lanes: vec![Vec::new(); columns + 1],
            gutter_x: Vec::new(),
            column_x: Vec::new(),
            width: 0,
        }
    }

    fn claim(&mut self, gutter: usize, channel: &str) {
        let lanes = &mut self.lanes[gutter];
        if lanes.iter().any(|held| held == channel) {
            return;
        }
        lanes.push(channel.to_string());
    }

    fn place(&mut self, column_w: &[usize]) {
        let mut gutter_x = Vec::with_capacity(self.lanes.len());
        let mut column_x = Vec::with_capacity(column_w.len());
        let mut x = 0;
        for (gutter, lanes) in self.lanes.iter().enumerate() {
            gutter_x.push(x);
            x += lanes.len() + GUTTER_PAD;
            if let Some(width) = column_w.get(gutter) {
                column_x.push(x);
                x += width;
            }
        }
        self.gutter_x = gutter_x;
        self.column_x = column_x;
        self.width = x;
    }

    fn lane_x(&self, gutter: usize, channel: &str) -> usize {
        let lane = self.lanes[gutter]
            .iter()
            .position(|held| held == channel)
            .unwrap_or(0);
        self.gutter_x[gutter] + 1 + lane
    }

    fn node_x(&self, column: usize) -> usize {
        self.column_x[column]
    }
}

struct SchematicNode {
    label: String,
    gutter_x: usize,
    x: usize,
    y: usize,
    level: bool,
}

pub struct Schematic {
    nodes: Vec<SchematicNode>,
    wires: Wires,
    width: usize,
    height: usize,
}

impl Schematic {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn of(fish: &[CircuitFish]) -> Self {
        let depth = depths(fish);
        let labels: Vec<String> = fish.iter().map(CircuitFish::node_label).collect();
        let columns = depth.iter().max().map_or(0, |deepest| deepest + 1);
        let mut column_w = vec![0usize; columns];
        for (index, label) in labels.iter().enumerate() {
            column_w[depth[index]] = column_w[depth[index]].max(table::visual_width(label));
        }

        let edges = edges_of(fish, &depth);
        let mut gutters = Gutters::new(columns);
        for edge in &edges {
            gutters.claim(depth[edge.to], &edge.channel);
            if edge.backward {
                gutters.claim(depth[edge.from] + 1, &edge.channel);
            }
        }
        gutters.place(&column_w);

        let returns = edges.iter().filter(|edge| edge.backward).count();
        let height = fish.len() + returns;
        let width = gutters.width;
        let mut wires = Wires::new(width, height);
        let mut return_row = fish.len();
        for edge in &edges {
            let source_end =
                gutters.node_x(depth[edge.from]) + table::visual_width(&labels[edge.from]);
            let target_x = gutters.node_x(depth[edge.to]);
            let entry_lane = gutters.lane_x(depth[edge.to], &edge.channel);
            if edge.backward {
                let exit_lane = gutters.lane_x(depth[edge.from] + 1, &edge.channel);
                wires.horizontal(edge.from, source_end, exit_lane);
                wires.vertical(exit_lane, edge.from, return_row);
                wires.horizontal(return_row, exit_lane, entry_lane);
                wires.vertical(entry_lane, return_row, edge.to);
                return_row += 1;
            } else {
                wires.horizontal(edge.from, source_end, entry_lane);
                wires.vertical(entry_lane, edge.from, edge.to);
            }
            wires.horizontal(edge.to, entry_lane, target_x - 1);
            wires.arrow(target_x - 1, edge.to);
        }

        let nodes = labels
            .into_iter()
            .enumerate()
            .map(|(index, label)| SchematicNode {
                label,
                gutter_x: gutters.gutter_x[depth[index]],
                x: gutters.node_x(depth[index]),
                y: index,
                level: fish[index].output_level,
            })
            .collect();
        Self {
            nodes,
            wires,
            width,
            height,
        }
    }

    pub fn draw(
        &self,
        buf: &mut Buffer,
        view: Rect,
        (rows, columns): (Range<usize>, Range<usize>),
        selected: Option<usize>,
    ) {
        let window = Window {
            rows: rows.clone(),
            columns: columns.clone(),
            width: self.width,
            height: self.height,
        };
        for (y, row) in (view.y..view.bottom()).zip(rows) {
            for (screen_x, x) in (view.x..view.right()).zip(columns.clone()) {
                let Some(symbol) = self.wires.char_in_view(x, row, &window) else {
                    continue;
                };
                buf[(screen_x, y)]
                    .set_char(symbol)
                    .set_fg(WIRE_COLOR)
                    .set_bg(BACKGROUND);
            }
            for (index, node) in self
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, node)| node.y == row)
            {
                let style = if selected == Some(index) {
                    Style::default().fg(BLACK).bg(WHITE)
                } else {
                    Style::default().fg(level_color(node.level)).bg(BACKGROUND)
                };
                for (offset, ch) in node.label.chars().enumerate() {
                    let x = node.x + offset;
                    if !columns.contains(&x) {
                        continue;
                    }
                    buf[(view.x + (x - columns.start) as u16, y)]
                        .set_char(ch)
                        .set_style(style);
                }
            }
        }
    }

    #[cfg(test)]
    fn node(&self, label: &str) -> &SchematicNode {
        self.nodes
            .iter()
            .find(|node| node.label == label)
            .unwrap_or_else(|| panic!("{label} is on the schematic"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wired(name: &str, listens: &[&str], drives: Option<&str>) -> CircuitFish {
        CircuitFish {
            name: name.to_string(),
            listens: listens.iter().map(|c| c.to_string()).collect(),
            drives: drives.map(str::to_string),
            produces: drives.iter().map(|c| c.to_string()).collect(),
            consumes: listens.iter().map(|c| c.to_string()).collect(),
            parts: Vec::new(),
            pins: Vec::new(),
            output_level: false,
        }
    }

    fn order(state: &CircuitState) -> Vec<&str> {
        state.fish.iter().map(|f| f.name.as_str()).collect()
    }

    fn state_of(fish: Vec<CircuitFish>) -> CircuitState {
        CircuitState::new(0, fish).expect("the board has fish on it")
    }

    fn chain_of(length: usize) -> Vec<CircuitFish> {
        (0..length)
            .map(|step| {
                let name = format!("F{step}");
                let listens = format!("c{step}");
                let drives = format!("c{}", step + 1);
                CircuitFish {
                    name,
                    listens: vec![listens.clone()],
                    drives: Some(drives.clone()),
                    produces: vec![drives],
                    consumes: vec![listens],
                    parts: Vec::new(),
                    pins: Vec::new(),
                    output_level: false,
                }
            })
            .collect()
    }

    #[test]
    fn an_empty_board_has_no_circuit_to_read() {
        assert!(CircuitState::new(0, Vec::new()).is_none());
    }

    #[test]
    fn rows_run_downstream_so_the_signal_flows_down_the_table() {
        let state = state_of(vec![
            wired("C", &["b"], Some("c")),
            wired("A", &["x"], Some("a")),
            wired("B", &["a"], Some("b")),
        ]);
        assert_eq!(order(&state), vec!["A", "B", "C"]);
    }

    #[test]
    fn fish_at_the_same_depth_are_listed_by_name() {
        let state = state_of(vec![
            wired("Zed", &["x"], Some("z")),
            wired("Ann", &["x"], Some("a")),
        ]);
        assert_eq!(order(&state), vec!["Ann", "Zed"]);
    }

    #[test]
    fn a_feedback_loop_is_ordered_without_hanging() {
        let state = state_of(vec![
            wired("Q", &["r", "qn"], Some("q")),
            wired("Qn", &["s", "q"], Some("qn")),
        ]);
        assert_eq!(state.fish.len(), 2, "a cross-coupled latch still lists");
    }

    #[test]
    fn a_loop_never_pushes_its_own_fish_deeper_every_pass() {
        let latch = vec![
            wired("Ss", &[], Some("s")),
            wired("Sr", &[], Some("r")),
            wired("Q", &["r", "qn"], Some("q")),
            wired("Qn", &["s", "q"], Some("qn")),
        ];
        assert!(
            column_count(&latch) <= latch.len(),
            "a cross-coupled pair relaxes each other upward forever unless the edge that \
             closes the loop is dropped, and then a 4-fish latch reports 15 columns"
        );
        assert_eq!(
            column_count(&latch),
            3,
            "source, then Q, then its complement"
        );
    }

    #[test]
    fn a_ring_of_three_stays_three_columns_wide() {
        let ring = vec![
            wired("R0", &["c2"], Some("c0")),
            wired("R1", &["c0"], Some("c1")),
            wired("R2", &["c1"], Some("c2")),
        ];
        assert_eq!(column_count(&ring), 3, "the loop is cut exactly once");
    }

    #[test]
    fn a_fish_that_hears_itself_is_still_a_source() {
        let state = state_of(vec![wired("Ring", &["clk"], Some("clk"))]);
        assert_eq!(order(&state), vec!["Ring"]);
    }

    #[test]
    fn an_unwired_column_reads_as_nothing() {
        let fish = wired("Deaf", &[], None);
        assert_eq!(fish.listens_text(), NOTHING);
        assert_eq!(fish.drives_text(), NOTHING);
        assert_eq!(fish.parts_text(), NOTHING);
    }

    #[test]
    fn a_stack_of_one_part_is_written_without_a_count() {
        assert_eq!(stacked_name(Part::DelaySpool, 1), "Delay Spool");
        assert_eq!(stacked_name(Part::DelaySpool, 3), "Delay Spool ×3");
    }

    fn pin_row(label: &str, channel: Option<&str>, direction: PinDirection) -> PinRow {
        PinRow {
            label: label.to_string(),
            channel: channel.map(str::to_string),
            direction,
        }
    }

    #[test]
    fn a_part_pin_hangs_under_its_fish_with_the_channel_it_reads() {
        let mut fish = wired("Reaper", &[], None);
        fish.pins = vec![pin_row("fire", Some("harvest"), PinDirection::In)];
        assert_eq!(fish.height(), 2, "the pin takes a line of its own");
        assert_eq!(fish.pin_text(0), "└ fire ◂ harvest");
    }

    #[test]
    fn an_unwired_part_pin_still_shows_itself() {
        let mut fish = wired("Reaper", &[], None);
        fish.pins = vec![pin_row("fire", None, PinDirection::In)];
        assert_eq!(fish.pin_text(0), "└ fire ◂ -");
    }

    #[test]
    fn several_pins_branch_like_a_tree_and_only_the_last_one_closes_it() {
        let mut fish = wired("Assay", &[], None);
        fish.pins = vec![
            pin_row("weight", None, PinDirection::Out),
            pin_row("value", None, PinDirection::Out),
            pin_row("over", Some("ripe"), PinDirection::Out),
        ];
        let branches: Vec<String> = (0..fish.pins.len()).map(|pin| fish.pin_text(pin)).collect();
        assert_eq!(
            branches,
            vec!["├ weight ▸ -", "├ value ▸ -", "└ over ▸ ripe"]
        );
    }

    #[test]
    fn a_pin_rows_arrow_is_the_way_its_signal_flows() {
        let mut fish = wired("Ram", &[], None);
        fish.pins = vec![
            pin_row("data_in", Some("d"), PinDirection::In),
            pin_row("data_out", Some("q"), PinDirection::Out),
        ];
        assert_eq!(
            fish.pin_text(0),
            "├ data_in ◂ d",
            "the channel flows into an input"
        );
        assert_eq!(
            fish.pin_text(1),
            "└ data_out ▸ q",
            "an output flows onto its channel"
        );
    }

    #[test]
    fn a_sense_pin_and_a_fire_pin_are_wires_the_schematic_draws() {
        let mut sense = wired("Assay", &[], None);
        sense.produces = vec!["ripe".to_string()];
        let gate = wired("Nr", &["ripe"], Some("nripe"));
        let mut module = wired("Reaper", &[], None);
        module.consumes = vec!["nripe".to_string()];

        let state = state_of(vec![module, gate, sense]);

        assert_eq!(
            order(&state),
            vec!["Assay", "Nr", "Reaper"],
            "a sense feeds its gate, and the gate pulls the module's trigger"
        );
        let schematic = Schematic::of(&state.fish);
        assert_eq!(
            edges_of(&state.fish, &depths(&state.fish)).len(),
            2,
            "both pin wires are edges"
        );
        assert!(schematic.width > 0);
    }

    #[test]
    fn selection_walks_down_and_back_up() {
        let mut state = state_of(vec![
            wired("A", &["x"], Some("a")),
            wired("B", &["a"], Some("b")),
            wired("C", &["b"], Some("c")),
        ]);
        state.scroll_down();
        state.scroll_down();
        assert_eq!(state.selected, 2);
        state.scroll_up();
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn selection_stops_at_both_ends() {
        let mut state = state_of(vec![wired("A", &["x"], Some("a"))]);
        state.scroll_up();
        state.scroll_down();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn a_named_fish_can_be_selected_when_the_view_reopens() {
        let mut state = state_of(vec![
            wired("A", &["x"], Some("a")),
            wired("B", &["a"], Some("b")),
        ]);
        state.select("B");
        assert_eq!(state.selected_fish_name(), "B");
        state.select("Nobody");
        assert_eq!(
            state.selected_fish_name(),
            "B",
            "an unknown name moves nothing"
        );
    }

    const CRAMPED_COLS: u16 = 30;
    const CRAMPED_ROWS: u16 = 9;

    fn rendered(state: &CircuitState, cols: u16, rows: u16) -> String {
        let area = Rect::new(0, 0, cols, rows);
        let mut buffer = Buffer::empty(area);
        CircuitOverlay::new(state, Screen::only(area)).render(area, &mut buffer);
        (0..rows)
            .map(|y| {
                (0..cols)
                    .map(|x| buffer[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn refusal_text(state: &CircuitState) -> String {
        state.notice().unwrap_or_default()
    }

    #[test]
    fn tab_swaps_the_two_views_of_a_small_board() {
        let mut state = state_of(chain_of(3));
        state.toggle_view();
        assert!(state.view == CircuitView::Schematic);
        state.toggle_view();
        assert!(state.view == CircuitView::Signal);
        assert_eq!(CircuitState::tab_hint(true), HINT_TAB_SIGNAL);
        assert_eq!(CircuitState::tab_hint(false), HINT_TAB_SCHEMATIC);
    }

    #[test]
    fn a_board_of_too_many_fish_says_so_and_stays_in_the_signal_view() {
        let mut state = state_of(
            (0..SCHEMATIC_MAX_FISH + 1)
                .map(|index| wired(&format!("F{index}"), &["x"], Some("y")))
                .collect(),
        );
        state.toggle_view();
        assert!(state.view == CircuitView::Signal);
        assert!(
            refusal_text(&state).contains("too big to draw"),
            "{}",
            refusal_text(&state)
        );
    }

    #[test]
    fn a_board_too_deep_to_lay_out_is_refused_too() {
        let mut state = state_of(chain_of(SCHEMATIC_MAX_COLUMNS + 1));
        state.toggle_view();
        assert!(state.view == CircuitView::Signal);
        assert!(
            refusal_text(&state).contains("9 deep"),
            "{}",
            refusal_text(&state)
        );
    }

    #[test]
    fn a_board_exactly_at_the_limit_still_draws() {
        let mut state = state_of(chain_of(SCHEMATIC_MAX_COLUMNS));
        state.toggle_view();
        assert!(state.view == CircuitView::Schematic);
    }

    #[test]
    fn a_small_screen_still_draws_the_schematic_and_every_hint() {
        let mut state = state_of(chain_of(3));
        state.toggle_view();
        let text = rendered(&state, CRAMPED_COLS, CRAMPED_ROWS);
        assert!(
            text.contains("[F0"),
            "the board is drawn, not refused:\n{text}"
        );
        for hint in [HINT_ENTER_WIRE, HINT_TAB_SIGNAL, HINT_CLOSE] {
            assert!(
                text.contains(hint),
                "{hint} wrapped onto a row of its own:\n{text}"
            );
        }
    }

    #[test]
    fn a_wide_schematic_pans_to_the_selected_fish() {
        let mut state = state_of(chain_of(SCHEMATIC_MAX_COLUMNS));
        state.toggle_view();
        for _ in 1..SCHEMATIC_MAX_COLUMNS {
            state.scroll_down();
        }
        let text = rendered(&state, CRAMPED_COLS, CRAMPED_ROWS + 6);
        assert!(
            text.contains("[F7]"),
            "the selected fish is panned into view:\n{text}"
        );
        assert!(
            text.contains('━'),
            "and the bottom border shows where the view sits:\n{text}"
        );
    }

    #[test]
    fn narrow_columns_give_up_width_parts_first() {
        let mut fish = wired("Clk", &["clk"], Some("clk"));
        fish.parts = vec![
            stacked_name(Part::InverterCoil, 1),
            stacked_name(Part::DelaySpool, 3),
        ];
        let state = state_of(vec![fish]);
        let natural = Columns::fit(&state, 0, u16::MAX / 2);
        let room = Columns::natural_width(&Columns::items(&state)) - 10;
        let fitted = Columns::fit(&state, 0, room);
        assert_eq!(
            ROW_PREFIX_W + fitted.widths.iter().sum::<u16>() + GAPS_W + EDGE_PAD,
            room,
            "the table shrinks to exactly the room"
        );
        assert_eq!(
            fitted.widths[PARTS_COLUMN],
            natural.widths[PARTS_COLUMN] - 10,
            "the long parts list pays for it"
        );
        assert_eq!(fitted.widths[FISH_COLUMN], natural.widths[FISH_COLUMN]);
    }

    #[test]
    fn the_schematic_lays_every_fish_out_in_the_column_of_its_depth() {
        let schematic = Schematic::of(&sorted_by_depth(chain_of(3)));
        let (first, second, third) = (
            schematic.node("[F0]"),
            schematic.node("[F1]"),
            schematic.node("[F2]"),
        );
        assert!(
            first.x < second.x && second.x < third.x,
            "a driver is drawn to the left of what it drives"
        );
        assert_eq!((first.y, second.y, third.y), (0, 1, 2), "one fish per row");
        assert_eq!(
            schematic.height, 3,
            "and no returning line to make room for"
        );
    }

    #[test]
    fn a_wire_reaches_its_target_with_an_arrowhead() {
        let schematic = Schematic::of(&sorted_by_depth(chain_of(2)));
        let target = schematic.node("[F1]");
        assert_eq!(
            schematic.wires.char_at(target.x - 1, target.y),
            Some(WIRE_ARROW),
            "the wire lands on the left of the box it drives"
        );
    }

    #[test]
    fn a_feedback_edge_returns_on_a_line_below_the_graph() {
        let ring = Schematic::of(&sorted_by_depth(vec![wired("Ring", &["clk"], Some("clk"))]));
        assert_eq!(
            ring.height, 2,
            "one fish row plus the row its own signal returns along"
        );
        let node = ring.node("[Ring]");
        assert!(
            ring.wires
                .char_at(node.x + node.label.len(), node.y)
                .is_some(),
            "the loop leaves the fish"
        );
        assert!(
            (0..ring.width).any(|x| ring.wires.char_at(x, 1).is_some()),
            "and travels back along the return row"
        );
    }

    #[test]
    fn two_channels_entering_one_column_never_share_a_lane() {
        const COLUMN_W: usize = 3;
        let mut gutters = Gutters::new(2);
        gutters.claim(1, "a");
        gutters.claim(1, "b");
        gutters.claim(1, "a");
        gutters.place(&[COLUMN_W, COLUMN_W]);
        assert_ne!(
            gutters.lane_x(1, "a"),
            gutters.lane_x(1, "b"),
            "two nets in one gutter are two vertical lanes"
        );
        assert!(
            gutters.lane_x(1, "a") < gutters.node_x(1),
            "and both run to the left of the column they enter"
        );
    }

    #[test]
    fn a_crossing_of_two_wires_is_drawn_as_a_junction() {
        assert_eq!(wire_char(NORTH | SOUTH | EAST | WEST), Some('┼'));
        assert_eq!(wire_char(SOUTH | EAST), Some('┌'));
        assert_eq!(wire_char(NORTH | WEST), Some('┘'));
        assert_eq!(wire_char(EAST | WEST), Some('─'));
        assert_eq!(wire_char(NORTH | SOUTH), Some('│'));
        assert_eq!(wire_char(0), None, "an untouched cell stays empty");
        assert_eq!(wire_char(ARROW | WEST), Some(WIRE_ARROW), "the head wins");
    }

    #[test]
    fn a_tall_schematic_scrolls_to_keep_the_selected_fish_in_view() {
        let mut state = state_of(chain_of(SCHEMATIC_MAX_COLUMNS));
        state.toggle_view();
        for _ in 0..SCHEMATIC_MAX_COLUMNS {
            state.scroll_down();
        }
        let text = rendered(&state, CRAMPED_COLS, CRAMPED_ROWS);
        assert!(
            text.contains("[F7"),
            "the last fish scrolled into view:\n{text}"
        );
        assert!(
            !text.contains("[F0]"),
            "and the first one scrolled out:\n{text}"
        );
    }

    #[test]
    fn a_wiring_edit_that_leaves_the_board_drawable_comes_back_to_the_schematic() {
        let mut previous = state_of(chain_of(3));
        previous.toggle_view();
        let mut reopened = state_of(chain_of(3));
        reopened.restore_view(&previous);
        assert!(reopened.view == CircuitView::Schematic);

        let mut outgrown = state_of(chain_of(SCHEMATIC_MAX_COLUMNS + 1));
        outgrown.restore_view(&previous);
        assert!(
            outgrown.view == CircuitView::Signal,
            "a board that outgrew the limit comes back as a table"
        );
    }
}
