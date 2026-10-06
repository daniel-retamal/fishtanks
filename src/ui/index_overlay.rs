use std::cell::Cell;
use std::ops::Range;

use rand::RngExt;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

use crate::colors::{BLACK, STEEL, WHITE};
use crate::fishes::fish::{Direction, Fish, LineSprite};
use crate::fishes::unfish::UnfishKind;
use crate::tanks::soul_wall::SoulWall;
use crate::ui::fields::{self, FieldKind, FieldValue};
use crate::ui::grid::{self, CELL_PAD, Grid, HEADER_ROWS, HeaderStyle};
use crate::ui::hint_bar::HintBar;
use crate::ui::hints::{HINT_CLOSE, HINT_ENTER_SHOW, HINT_NAV, HINT_SCROLL};
use crate::ui::index_query::{self, ALIVE, Column, DEAD, IndexQuery};
use crate::ui::input_action::InputAction;
use crate::ui::layout::{Screen, Scroll, Scrollbar};
use crate::ui::modal::{Frame, Modal};
use crate::ui::text_input::TextInput;
use crate::ui::{render_fish_sprite, table, tank_view};

pub const NAME_COLUMN_MAX_W: usize = 24;

const TITLE: &str = " FishResource#index ";
const HINT_COLUMNS: &str = "←→ column";
const HINT_SORT: &str = "S sort";
const HINT_FILTER: &str = "F filter";
const HINT_CLEAR: &str = "C clear";
const HINT_KEEP: &str = "ENTER keep";
const HINT_UNDO: &str = "ESC undo";
const QUERY_PROMPT: &str = "/index ";
const QUERY_ROWS: u16 = 2;
const QUERY_PAD: u16 = 1;
const NAME_COLUMN: usize = 0;
const FIRST_PAGED_COLUMN: usize = 1;
const RULE_W: u16 = 1;
const MIN_NAME_W: u16 = 4;
const MIN_PAGED_W: u16 = 3;
const MIN_FANTASY_W: usize = 4;
const MAX_RANDOM_FIELDS: usize = 3;
const MIN_TABLE_ROWS: usize = 1;
const BACKGROUND: Color = Color::Reset;

pub struct FishSnapshot {
    name: String,
    species_name: &'static str,
    art: LineSprite,
    display_width: usize,
    weight_g: u32,
    worth: String,
    fed: String,
    fed_level: Option<f64>,
    tank_name: String,
    display_height: u16,
    is_unfish: bool,
    dead: Option<Color>,
}

impl FishSnapshot {
    fn status(&self) -> &'static str {
        if self.dead.is_some() { DEAD } else { ALIVE }
    }
}

struct Row {
    snapshot: FishSnapshot,
    fish: Fish,
    fields: Vec<FieldValue>,
}

struct Editing {
    column: Column,
    input: TextInput,
    before: Option<String>,
}

pub struct IndexState {
    pub selected: usize,
    scroll: Scroll,
    col_scroll: Cell<usize>,
    focus: usize,
    rows: Vec<Row>,
    view: Vec<usize>,
    columns: Vec<Column>,
    fantasy: Vec<FieldKind>,
    data_widths: Vec<(Column, usize)>,
    query: IndexQuery,
    editing: Option<Editing>,
    animated_fish: Option<Fish>,
}

fn row_height(fish: &Fish, art: &LineSprite) -> u16 {
    match fish.unfish_kind().and_then(UnfishKind::grid) {
        Some(grid) => grid.height,
        None => art.rows.len() as u16,
    }
}

fn still_art(fish: &Fish) -> LineSprite {
    let mut art = display_clone(fish.clone()).line_sprite();
    let body_row = art.body_row;
    if let Some(body) = art.rows.get_mut(body_row) {
        *body = fish.static_left_segments();
    }
    art
}

fn snapshot(tank_name: &str, fish: &Fish, dead: Option<Color>) -> FishSnapshot {
    let art = still_art(fish);
    let alive = dead.is_none();
    FishSnapshot {
        name: fish.name.clone(),
        species_name: fish.kind_name(),
        display_width: fish.display_width,
        weight_g: fish.weight_g,
        worth: if alive {
            fields::format_money(fish.sell_value())
        } else {
            table::NOTHING.to_string()
        },
        fed: if alive {
            fields::format_fed(fish.fed())
        } else {
            table::NOTHING.to_string()
        },
        fed_level: fish.fed().percent().filter(|_| alive),
        tank_name: tank_name.to_string(),
        display_height: row_height(fish, &art),
        is_unfish: fish.unfish_state.is_some(),
        dead,
        art,
    }
}

fn fantasy_kinds(query: &IndexQuery, rng: &mut impl RngExt) -> Vec<FieldKind> {
    let mut chosen: Vec<FieldKind> = if query.all {
        FieldKind::all().to_vec()
    } else {
        let mut avail = FieldKind::all().to_vec();
        let count = rng.random_range(0..=MAX_RANDOM_FIELDS);
        (0..count)
            .map(|_| avail.remove(rng.random_range(0..avail.len())))
            .collect()
    };
    for column in query.columns() {
        if let Column::Field(kind) = column
            && !chosen.contains(&kind)
        {
            chosen.push(kind);
        }
    }
    chosen
}

impl IndexState {
    pub fn new(living: &[(&str, &Fish)], walls: &[(&str, &SoulWall)], query: IndexQuery) -> Self {
        let mut rng = rand::rng();
        let alive = living
            .iter()
            .filter(|(_, fish)| !fish.is_invisible())
            .map(|&(tank, fish)| (tank, fish, None));
        let dead = walls
            .iter()
            .flat_map(|&(tank, wall)| wall.all().map(move |fish| (tank, fish, Some(wall.color()))));
        let everyone: Vec<(&str, &Fish, Option<Color>)> = alive.chain(dead).collect();
        let fantasy = fantasy_kinds(&query, &mut rng);
        let names: Vec<String> = everyone.iter().map(|(_, f, _)| f.name.clone()).collect();
        let rows: Vec<Row> = everyone
            .iter()
            .map(|&(tank, fish, dead)| Row {
                snapshot: snapshot(tank, fish, dead),
                fish: fish.clone(),
                fields: fantasy
                    .iter()
                    .map(|&kind| fields::field_value(kind, fish, &names, &mut rng))
                    .collect(),
            })
            .collect();

        let mut state = Self {
            selected: 0,
            scroll: Scroll::default(),
            col_scroll: Cell::new(FIRST_PAGED_COLUMN),
            focus: NAME_COLUMN,
            rows,
            view: Vec::new(),
            columns: vec![Column::Name],
            fantasy,
            data_widths: Vec::new(),
            query,
            editing: None,
            animated_fish: None,
        };
        state.data_widths = state
            .candidates()
            .map(|column| (column, state.data_width(column)))
            .collect();
        state.refresh(None);
        if let Some(sort) = state.query.sort
            && let Some(at) = state.columns.iter().position(|&c| c == sort.column)
        {
            state.focus = at;
        }
        state
    }

    fn candidates(&self) -> impl Iterator<Item = Column> + '_ {
        Column::FIXED
            .into_iter()
            .chain(self.fantasy.iter().map(|&kind| Column::Field(kind)))
    }

    fn passes(&self, row: &Row) -> bool {
        let shown = row.snapshot.dead.is_none() || self.query.shows_the_dead();
        shown
            && self
                .query
                .filters
                .iter()
                .all(|filter| filter.matches(&self.cell(row, filter.column)))
    }

    fn arrange_columns(&mut self) {
        let focused = self.focused();
        let tells_the_dead = self.query.mentions(Column::Status)
            || self
                .view
                .iter()
                .any(|&row| self.rows[row].snapshot.dead.is_some());
        self.columns = self
            .candidates()
            .filter(|&column| column != Column::Status || tells_the_dead)
            .collect();
        self.focus = self
            .columns
            .iter()
            .position(|&column| column == focused)
            .unwrap_or(self.focus.min(self.columns.len() - 1));
    }

    fn data_width(&self, column: Column) -> usize {
        let widest = self
            .rows
            .iter()
            .map(|row| self.cell_width(row, column))
            .max()
            .unwrap_or(0);
        match column {
            Column::Name => widest.min(NAME_COLUMN_MAX_W),
            Column::Field(_) => widest.max(MIN_FANTASY_W),
            _ => widest,
        }
    }

    fn cell_width(&self, row: &Row, column: Column) -> usize {
        match column {
            Column::Display => row.snapshot.display_width,
            _ => table::visual_width(&self.cell(row, column).text),
        }
    }

    fn field<'r>(&self, row: &'r Row, kind: FieldKind) -> Option<&'r FieldValue> {
        let at = self.fantasy.iter().position(|&k| k == kind)?;
        row.fields.get(at)
    }

    fn cell(&self, row: &Row, column: Column) -> index_query::Cell {
        let snap = &row.snapshot;
        match column {
            Column::Name => index_query::Cell::counted(&snap.name, None),
            Column::Status => index_query::Cell::counted(snap.status(), None),
            Column::Species => index_query::Cell::counted(snap.species_name, None),
            Column::Display => index_query::Cell::counted("", None),
            Column::Weight => index_query::Cell::counted(
                fields::format_weight(snap.weight_g),
                Some(f64::from(snap.weight_g)),
            ),
            Column::Worth => index_query::Cell::text(&snap.worth),
            Column::Fed => index_query::Cell::counted(&snap.fed, snap.fed_level),
            Column::Fishtank => index_query::Cell::counted(&snap.tank_name, None),
            Column::Field(kind) => {
                index_query::Cell::text(self.field(row, kind).map_or("", |v| v.text.as_str()))
            }
        }
    }

    fn refresh(&mut self, keep: Option<usize>) {
        let mut view: Vec<usize> = (0..self.rows.len())
            .filter(|&index| self.passes(&self.rows[index]))
            .collect();
        if let Some(sort) = self.query.sort {
            view.sort_by(|&a, &b| {
                let a = self.cell(&self.rows[a], sort.column);
                let b = self.cell(&self.rows[b], sort.column);
                a.ranked(&b, sort.descending)
            });
        }
        self.view = view;
        self.arrange_columns();
        let position = keep.and_then(|row| self.view.iter().position(|&shown| shown == row));
        let selected = position.unwrap_or(self.selected.min(self.view.len().saturating_sub(1)));
        self.selected = usize::MAX;
        self.select(selected);
    }

    fn selected_row(&self) -> Option<usize> {
        self.view.get(self.selected).copied()
    }

    pub fn tick_animation(&mut self, dt: f32) {
        if let Some(ref mut fish) = self.animated_fish {
            fish.tick_animation(dt);
        }
    }

    fn select(&mut self, index: usize) {
        if index == self.selected {
            return;
        }
        self.selected = index;
        self.animated_fish = self
            .selected_row()
            .map(|row| display_clone(self.rows[row].fish.clone()));
    }

    pub fn scroll_up(&mut self) {
        self.select(self.selected.saturating_sub(1));
    }

    pub fn scroll_down(&mut self) {
        if self.selected + 1 < self.view.len() {
            self.select(self.selected + 1);
        }
    }

    pub fn shown(&self) -> usize {
        self.view.len()
    }

    pub fn selected_fish_name(&self) -> Option<&str> {
        let row = &self.rows[self.selected_row()?];
        row.snapshot
            .dead
            .is_none()
            .then_some(row.snapshot.name.as_str())
    }

    pub fn selected_tank_name(&self) -> &str {
        self.selected_row()
            .map_or("", |row| self.rows[row].snapshot.tank_name.as_str())
    }

    fn focused(&self) -> Column {
        self.columns[self.focus]
    }

    fn step_focus(&mut self, right: bool) {
        let queryable = |at: &usize| self.columns[*at].is_queryable();
        let next = if right {
            (self.focus + 1..self.columns.len()).find(queryable)
        } else {
            (0..self.focus).rev().find(queryable)
        };
        if let Some(next) = next {
            self.focus = next;
        }
    }

    pub fn scroll_left(&mut self) {
        self.step_focus(false);
    }

    pub fn scroll_right(&mut self) {
        self.step_focus(true);
    }

    pub fn sort_focused(&mut self) {
        let column = self.focused();
        if !column.is_queryable() {
            return;
        }
        self.query.cycle_sort(column);
        self.refresh(self.selected_row());
    }

    pub fn clear(&mut self) {
        self.query.clear();
        self.refresh(self.selected_row());
    }

    pub fn is_editing(&self) -> bool {
        self.editing.is_some()
    }

    pub fn edit_focused(&mut self) {
        let column = self.focused();
        if !column.is_queryable() {
            return;
        }
        let before = self.query.filter_text(column).map(str::to_string);
        self.editing = Some(Editing {
            column,
            input: TextInput::with_value(before.clone().unwrap_or_default()),
            before,
        });
    }

    pub fn edit(&mut self, action: &InputAction) {
        let Some(mut editing) = self.editing.take() else {
            return;
        };
        match action {
            InputAction::Confirm => {}
            InputAction::Cancel => {
                let restored = editing.before.take().unwrap_or_default();
                self.query.set_filter(editing.column, restored);
            }
            _ => {
                editing.input.handle_action(action);
                self.query
                    .set_filter(editing.column, editing.input.as_str().to_string());
                self.editing = Some(editing);
            }
        }
        self.refresh(self.selected_row());
    }

    fn header_text(&self, column: Column) -> String {
        match self.query.sort {
            Some(sort) if sort.column == column => format!("{} {}", column.header(), sort.mark()),
            _ => column.header().to_string(),
        }
    }

    fn all_col_widths(&self) -> Vec<usize> {
        self.columns
            .iter()
            .map(|&column| {
                let data = self
                    .data_widths
                    .iter()
                    .find(|(known, _)| *known == column)
                    .map_or(0, |&(_, width)| width);
                let header = table::visual_width(&self.header_text(column));
                data.max(header) + CELL_PAD as usize * 2
            })
            .collect()
    }

    fn query_line(&self) -> Option<(String, usize)> {
        if !self.query.is_narrowed() && self.editing.is_none() {
            return None;
        }
        let mut words = self.query.words();
        let Some(editing) = &self.editing else {
            let line = format!("{QUERY_PROMPT}{}", words.join(" "));
            let end = line.len();
            return Some((line, end));
        };
        let head = format!("{}:", editing.column.key());
        let at = match words.iter().position(|word| word.starts_with(&head)) {
            Some(at) => at,
            None => {
                let at = words.len() - usize::from(self.query.sort.is_some());
                words.insert(at, head.clone());
                at
            }
        };
        let before: usize = words[..at].iter().map(|word| word.len() + 1).sum();
        let cursor = QUERY_PROMPT.len() + before + head.len() + editing.input.cursor;
        Some((format!("{QUERY_PROMPT}{}", words.join(" ")), cursor))
    }

    fn hints(&self, overflowing: bool, page_more: Option<(usize, usize)>) -> HintBar {
        if self.editing.is_some() {
            return HintBar::new(HINT_UNDO).action(HINT_KEEP);
        }
        let nav = if overflowing { HINT_SCROLL } else { HINT_NAV };
        let column = match page_more {
            Some((shown, total)) => format!("{HINT_COLUMNS} ({shown}/{total})"),
            None => HINT_COLUMNS.to_string(),
        };
        let queryable = self.focused().is_queryable();
        HintBar::new(HINT_CLOSE)
            .counted(
                nav,
                overflowing.then_some((self.selected + 1, self.view.len())),
            )
            .action(column)
            .action_if(queryable, HINT_SORT)
            .action_if(queryable, HINT_FILTER)
            .action_if(self.query.is_narrowed(), HINT_CLEAR)
            .action_if(!self.view.is_empty(), HINT_ENTER_SHOW)
    }

    fn page(&self, inner_w: u16) -> Page {
        let widths = self.all_col_widths();
        let total = widths.len();
        let name_w = (widths[NAME_COLUMN] as u16)
            .min(inner_w.saturating_sub(RULE_W + MIN_PAGED_W))
            .max(MIN_NAME_W.min(inner_w));
        let room = inner_w.saturating_sub(name_w + RULE_W);
        let fits_from = |start: usize| {
            let mut used = 0u16;
            let mut end = start;
            while end < total {
                let needed = widths[end] as u16 + if end > start { RULE_W } else { 0 };
                if used + needed > room {
                    break;
                }
                used += needed;
                end += 1;
            }
            end
        };
        let last_start = (FIRST_PAGED_COLUMN..total)
            .find(|&start| fits_from(start) == total)
            .unwrap_or(total.saturating_sub(1))
            .max(FIRST_PAGED_COLUMN);
        let mut start = self.col_scroll.get().clamp(FIRST_PAGED_COLUMN, last_start);
        if self.focus >= FIRST_PAGED_COLUMN {
            start = start.min(self.focus);
            while start < self.focus && fits_from(start) <= self.focus {
                start += 1;
            }
        }
        self.col_scroll.set(start);
        let end = fits_from(start).max((start + 1).min(total));
        let mut page_widths: Vec<u16> = vec![name_w];
        page_widths.extend((start..end).map(|column| widths[column] as u16));
        let used: u16 = page_widths.iter().sum::<u16>() + RULE_W * (page_widths.len() as u16 - 1);
        let last = page_widths.len() - 1;
        if used < inner_w {
            page_widths[last] += inner_w - used;
        } else if used > inner_w {
            page_widths[last] = page_widths[last].saturating_sub(used - inner_w);
        }
        Page {
            columns: std::iter::once(NAME_COLUMN).chain(start..end).collect(),
            widths: page_widths,
            more: start > FIRST_PAGED_COLUMN || end < total,
            shown_up_to: end,
            total,
        }
    }
}

struct Page {
    columns: Vec<usize>,
    widths: Vec<u16>,
    more: bool,
    shown_up_to: usize,
    total: usize,
}

pub struct IndexOverlay<'a> {
    state: &'a IndexState,
    screen: Screen,
}

impl<'a> IndexOverlay<'a> {
    pub fn new(state: &'a IndexState, screen: Screen) -> Self {
        Self { state, screen }
    }
}

impl Widget for IndexOverlay<'_> {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        let state = self.state;
        let widths = state.all_col_widths();
        let natural_w = (widths.iter().sum::<usize>() + widths.len().saturating_sub(1)) as u16;
        let heights: Vec<usize> = state
            .view
            .iter()
            .map(|&row| state.rows[row].snapshot.display_height as usize)
            .collect();
        let query = state.query_line();
        let query_rows = if query.is_some() { QUERY_ROWS } else { 0 };
        let expected_page = state.page(natural_w.min(self.screen.whole.width.saturating_sub(2)));
        let page_more = expected_page
            .more
            .then_some((expected_page.shown_up_to, expected_page.total));
        let frame = Frame {
            title: TITLE,
            border: WHITE,
            background: BACKGROUND,
        };
        let rows = heights.iter().sum::<usize>().max(MIN_TABLE_ROWS) as u16;
        let content = (natural_w, rows + HEADER_ROWS + query_rows);
        let (modal, _) = Modal::open_fitting(buf, self.screen, &frame, content, |overflowing| {
            state.hints(overflowing, page_more)
        });

        let mut body = modal.body;
        let page = state.page(body.width);
        let grid = Grid {
            x: body.x,
            widths: page.widths.clone(),
        };
        let mut query_rule = None;
        if let Some((line, cursor)) = &query
            && body.height > 0
        {
            draw_query(buf, body, line, *cursor, state.is_editing());
            let rows = QUERY_ROWS.min(body.height);
            if rows == QUERY_ROWS {
                let y = body.y + 1;
                let hangs = if body.height > QUERY_ROWS {
                    grid.rule_xs()
                } else {
                    Vec::new()
                };
                draw_query_rule(buf, modal.rect, y, &hangs);
                query_rule = Some(y);
            }
            body.y += rows;
            body.height -= rows;
        }

        let (header_y, data) = grid::split_header(body);
        if let Some(y) = header_y {
            let headers: Vec<String> = page
                .columns
                .iter()
                .map(|&c| state.header_text(state.columns[c]))
                .collect();
            let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
            let style = HeaderStyle {
                text: Style::default()
                    .fg(WHITE)
                    .add_modifier(Modifier::BOLD)
                    .bg(BACKGROUND),
                rule: Style::default().fg(WHITE).bg(BACKGROUND),
            };
            let frame_rect = Rect::new(modal.rect.x, y, modal.rect.width, HEADER_ROWS);
            grid::draw_header(buf, &grid, frame_rect, y, &header_refs, &style);
            if let Some(position) = page.columns.iter().position(|&c| c == state.focus) {
                let focused = Style::default()
                    .fg(BLACK)
                    .add_modifier(Modifier::BOLD)
                    .bg(WHITE);
                grid::put(buf, grid.cell(position, y, 1), &headers[position], focused);
            }
        }

        if heights.is_empty() && data.height > 0 {
            let style = Style::default().fg(STEEL).bg(BACKGROUND);
            grid::put(buf, grid.cell(0, data.y, 1), table::NOTHING, style);
            grid.draw_rules(buf, data.y, 1, Style::default().fg(WHITE).bg(BACKGROUND));
        }
        let shown = state
            .scroll
            .follow(&heights, state.selected, data.height as usize);
        let mut y = data.y;
        for index in shown.clone() {
            let height = (heights[index] as u16).min(data.bottom().saturating_sub(y));
            if height == 0 {
                break;
            }
            draw_data_row(buf, state, &page, &grid, index, y, height);
            y += height;
        }
        if shown.is_empty() && !heights.is_empty() && data.height > 0 {
            draw_data_row(
                buf,
                state,
                &page,
                &grid,
                state.selected,
                data.y,
                data.height,
            );
        }
        if page.more {
            let rules: Vec<u16> = header_y
                .map(|y| y + 1)
                .into_iter()
                .chain(query_rule)
                .collect();
            open_right_edge(buf, modal.rect, &rules);
            return;
        }
        let lines_before: usize = heights[..shown.start].iter().sum();
        Scrollbar {
            x: modal.scrollbar_x(),
            top: data.y,
            height: data.height,
        }
        .draw(
            buf,
            line_range(lines_before, &heights[shown], data.height),
            heights.iter().sum(),
            WHITE,
        );
    }
}

fn draw_query(buf: &mut Buffer, body: Rect, line: &str, cursor: usize, editing: bool) {
    let x = body.x + QUERY_PAD;
    let room = body.width.saturating_sub(QUERY_PAD * 2) as usize;
    let text = Style::default().fg(WHITE).bg(BACKGROUND);
    let head = table::scrolled_to_fit(&line[..cursor], room.saturating_sub(1));
    let skipped = cursor - head.len();
    let shown = &line[skipped..];
    buf.set_stringn(x, body.y, shown, room, text);
    if !editing {
        return;
    }
    let cursor_x = x + table::visual_width(head) as u16;
    if cursor_x >= x + room as u16 {
        return;
    }
    let under = line[cursor..].chars().next().unwrap_or(' ');
    buf[(cursor_x, body.y)]
        .set_char(under)
        .set_style(Style::default().fg(BLACK).bg(WHITE));
}

fn line_range(before: usize, shown: &[usize], room: u16) -> Range<usize> {
    let lines: usize = shown.iter().sum();
    before..before + lines.min(room as usize)
}

fn draw_query_rule(buf: &mut Buffer, rect: Rect, y: u16, rule_xs: &[u16]) {
    table::draw_box_separator(buf, rect.x, y, rect.width, &[], WHITE, BACKGROUND);
    let style = Style::default().fg(WHITE).bg(BACKGROUND);
    for &x in rule_xs {
        if x > rect.x && x + 1 < rect.right() {
            buf[(x, y)].set_char('┬').set_style(style);
        }
    }
}

fn open_right_edge(buf: &mut Buffer, rect: Rect, rules: &[u16]) {
    let right = rect.right().saturating_sub(1);
    let style = Style::default().fg(WHITE).bg(BACKGROUND);
    buf[(right, rect.y)].set_char('─').set_style(style);
    let bottom = rect.bottom().saturating_sub(1);
    buf[(right, bottom)].set_char('─').set_style(style);
    for y in rect.y + 1..bottom {
        buf[(right, y)].set_char(' ').set_style(style);
    }
    for &y in rules {
        buf[(right, y)].set_char('─').set_style(style);
    }
}

fn draw_data_row(
    buf: &mut Buffer,
    state: &IndexState,
    page: &Page,
    grid: &Grid,
    index: usize,
    row_y: u16,
    row_h: u16,
) {
    let selected = index == state.selected;
    let row = &state.rows[state.view[index]];
    let snap = &row.snapshot;
    let row_bg = if selected { WHITE } else { BACKGROUND };
    let fg = match (selected, snap.dead) {
        (true, _) => BLACK,
        (false, Some(soul)) => soul,
        (false, None) => STEEL,
    };
    let text = Style::default().fg(fg).bg(row_bg);
    let text_y = row_y + row_h / 2;

    for (position, &column) in page.columns.iter().enumerate() {
        let block = grid.cell(position, row_y, row_h);
        for y in block.top()..block.bottom() {
            grid::put(buf, Rect::new(block.x, y, block.width, 1), "", text);
        }
        let cell = grid.cell(position, text_y, 1);
        match state.columns[column] {
            Column::Display => draw_art(buf, state, index, block),
            Column::Worth => grid::put_money(buf, cell, &snap.worth, text),
            Column::Field(FieldKind::FavoriteColor) => {
                let swatch = state
                    .field(row, FieldKind::FavoriteColor)
                    .and_then(|value| value.swatch);
                grid::put(buf, cell, "", Style::default().bg(swatch.unwrap_or(BLACK)));
            }
            other => grid::put(buf, cell, &state.cell(row, other).text, text),
        }
    }
    grid.draw_rules(buf, row_y, row_h, Style::default().fg(WHITE).bg(row_bg));
}

fn draw_art(buf: &mut Buffer, state: &IndexState, index: usize, block: Rect) {
    for y in block.top()..block.bottom() {
        for x in block.left()..block.right() {
            buf[(x, y)].reset();
        }
    }
    let block = grid::inside(block);
    let row = &state.rows[state.view[index]];
    let snap = &row.snapshot;
    let animated = state
        .animated_fish
        .as_ref()
        .filter(|_| index == state.selected);
    if snap.is_unfish && snap.display_height > 1 {
        let fish = animated.unwrap_or(&row.fish);
        tank_view::render_multi_row_unfish_at(fish, block.x as i32, block.y as i32, block, buf);
        return;
    }
    let live = animated.map(Fish::line_sprite);
    let art = live.as_ref().unwrap_or(&snap.art);
    let body_y = block.y + art.body_row as u16;
    render_fish_sprite(buf, art, block.x, body_y, block, BACKGROUND);
}

fn display_clone(fish: Fish) -> Fish {
    let mut fish = fish.portrait();
    fish.facing = Direction::Left;
    fish
}
