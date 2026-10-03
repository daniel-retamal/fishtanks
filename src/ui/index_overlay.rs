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
use crate::fishes::unfish::{BALL_HEIGHT, SKULL_HEIGHT, UnfishKind};
use crate::tanks::soul_wall::SoulWall;
use crate::ui::fields::{self, FieldKind, FieldValue};
use crate::ui::grid::{self, CELL_PAD, Grid, HEADER_ROWS, HeaderStyle};
use crate::ui::hint_bar::HintBar;
use crate::ui::hints::{HINT_CLOSE, HINT_ENTER_SHOW, HINT_NAV, HINT_SCROLL};
use crate::ui::layout::{Screen, Scroll, Scrollbar};
use crate::ui::modal::{Frame, Modal};
use crate::ui::{render_fish_sprite, table, tank_view};

pub const NAME_COLUMN_MAX_W: usize = 24;

const TITLE: &str = " FishResource#index ";
const HINT_COLUMNS: &str = "←→ cols";
const NAME_COLUMN: usize = 0;
const FIRST_PAGED_COLUMN: usize = 1;
const RULE_W: u16 = 1;
const MIN_NAME_W: u16 = 4;
const MIN_PAGED_W: u16 = 3;
const MIN_FANTASY_W: usize = 4;
const BACKGROUND: Color = Color::Reset;
const ALIVE: &str = "Alive";
const DEAD: &str = "Dead";

#[derive(Clone, Copy, PartialEq, Eq)]
enum FixedColumn {
    Name,
    Species,
    Display,
    Weight,
    Fishtank,
    Status,
}

impl FixedColumn {
    fn header(self) -> &'static str {
        match self {
            FixedColumn::Name => "Name",
            FixedColumn::Species => "Species",
            FixedColumn::Display => "Display",
            FixedColumn::Weight => "Weight",
            FixedColumn::Fishtank => "Fishtank",
            FixedColumn::Status => "Status",
        }
    }
}

struct FantasyColumn {
    kind: FieldKind,
    cells: Vec<FieldValue>,
    col_width: usize,
}

pub struct FishSnapshot {
    name: String,
    species_name: &'static str,
    art: LineSprite,
    display_width: usize,
    weight_g: u32,
    tank_name: Option<String>,
    display_height: u16,
    is_unfish: bool,
    dead: Option<Color>,
}

impl FishSnapshot {
    fn status(&self) -> &'static str {
        if self.dead.is_some() { DEAD } else { ALIVE }
    }
}

pub struct IndexState {
    pub selected: usize,
    scroll: Scroll,
    col_scroll: Cell<usize>,
    snapshots: Vec<FishSnapshot>,
    fish_clones: Vec<Fish>,
    fixed: Vec<FixedColumn>,
    fixed_widths: Vec<usize>,
    fantasy_cols: Vec<FantasyColumn>,
    animated_fish: Option<Fish>,
}

fn row_height(fish: &Fish, art: &LineSprite) -> u16 {
    match fish.unfish_state.as_ref().map(|us| us.kind) {
        Some(UnfishKind::Ball) => BALL_HEIGHT,
        Some(UnfishKind::Skull) => SKULL_HEIGHT,
        _ => art.rows.len() as u16,
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

impl IndexState {
    pub fn new(
        fish_with_tanks: &[(&str, &Fish)],
        souls: Option<(&str, &SoulWall)>,
        all: bool,
        show_tank_col: bool,
    ) -> Self {
        let mut rng = rand::rng();

        let living = fish_with_tanks
            .iter()
            .filter(|(_, f)| !f.is_invisible())
            .map(|&(tank, fish)| (tank, fish, None));
        let dead = souls
            .into_iter()
            .flat_map(|(tank, wall)| wall.all().map(move |fish| (tank, fish, Some(wall.color()))));
        let filtered: Vec<(&str, &Fish, Option<Color>)> = living.chain(dead).collect();

        let snapshots: Vec<FishSnapshot> = filtered
            .iter()
            .map(|&(tank_name, f, dead)| {
                let art = still_art(f);
                FishSnapshot {
                    name: f.name.clone(),
                    species_name: f.species.display_name(),
                    display_width: f.display_width,
                    weight_g: f.weight_g,
                    tank_name: Some(tank_name.to_string()),
                    display_height: row_height(f, &art),
                    is_unfish: f.unfish_state.is_some(),
                    dead,
                    art,
                }
            })
            .collect();

        let fish_clones: Vec<Fish> = filtered.iter().map(|(_, f, _)| (*f).clone()).collect();
        let fish_names: Vec<String> = filtered.iter().map(|(_, f, _)| f.name.clone()).collect();

        let chosen_kinds: Vec<FieldKind> = if all {
            FieldKind::all().to_vec()
        } else {
            let count = rng.random_range(0..=3usize);
            let mut avail: Vec<FieldKind> = FieldKind::all().to_vec();
            let mut chosen = Vec::new();
            for _ in 0..count.min(avail.len()) {
                let idx = rng.random_range(0..avail.len());
                chosen.push(avail.remove(idx));
            }
            chosen
        };

        let fantasy_cols = chosen_kinds
            .into_iter()
            .map(|kind| {
                let cells: Vec<FieldValue> = fish_clones
                    .iter()
                    .map(|fish| fields::field_value(kind, fish, &fish_names, &mut rng))
                    .collect();
                let max_cell_w = cells
                    .iter()
                    .map(|c| table::visual_width(&c.text))
                    .max()
                    .unwrap_or(0);
                let col_width = max_cell_w
                    .max(table::visual_width(kind.header()))
                    .max(MIN_FANTASY_W);
                FantasyColumn {
                    kind,
                    cells,
                    col_width,
                }
            })
            .collect::<Vec<_>>();

        let mut fixed = vec![FixedColumn::Name];
        if snapshots.iter().any(|s| s.dead.is_some()) {
            fixed.push(FixedColumn::Status);
        }
        fixed.extend([
            FixedColumn::Species,
            FixedColumn::Display,
            FixedColumn::Weight,
        ]);
        if show_tank_col {
            fixed.push(FixedColumn::Fishtank);
        }
        let fixed_widths = fixed
            .iter()
            .map(|&column| {
                let widest = snapshots
                    .iter()
                    .map(|s| fixed_cell_width(column, s))
                    .max()
                    .unwrap_or(0)
                    .max(table::visual_width(column.header()));
                if column == FixedColumn::Name {
                    return widest.min(NAME_COLUMN_MAX_W);
                }
                widest
            })
            .collect();

        let animated_fish = fish_clones.first().cloned().map(display_clone);

        Self {
            selected: 0,
            scroll: Scroll::default(),
            col_scroll: Cell::new(FIRST_PAGED_COLUMN),
            snapshots,
            fish_clones,
            fixed,
            fixed_widths,
            fantasy_cols,
            animated_fish,
        }
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
            .fish_clones
            .get(self.selected)
            .cloned()
            .map(display_clone);
    }

    pub fn scroll_up(&mut self) {
        self.select(self.selected.saturating_sub(1));
    }

    pub fn scroll_down(&mut self) {
        if self.selected + 1 < self.snapshots.len() {
            self.select(self.selected + 1);
        }
    }

    pub fn selected_fish_name(&self) -> Option<&str> {
        self.snapshots
            .get(self.selected)
            .filter(|s| s.dead.is_none())
            .map(|s| s.name.as_str())
    }

    pub fn selected_tank_name(&self) -> &str {
        self.snapshots
            .get(self.selected)
            .and_then(|s| s.tank_name.as_deref())
            .unwrap_or("")
    }

    pub fn scroll_left(&mut self) {
        let start = self.col_scroll.get();
        self.col_scroll
            .set(start.saturating_sub(1).max(FIRST_PAGED_COLUMN));
    }

    pub fn scroll_right(&mut self) {
        let start = self.col_scroll.get();
        let last = self.all_col_widths().len().saturating_sub(1);
        self.col_scroll
            .set((start + 1).min(last.max(FIRST_PAGED_COLUMN)));
    }

    fn all_col_widths(&self) -> Vec<usize> {
        let fantasy = self.fantasy_cols.iter().map(|column| column.col_width);
        self.fixed_widths
            .iter()
            .copied()
            .chain(fantasy)
            .map(|width| width + CELL_PAD as usize * 2)
            .collect()
    }

    fn header(&self, column: usize) -> &str {
        if let Some(fixed) = self.fixed.get(column) {
            return fixed.header();
        }
        self.fantasy_cols[column - self.fixed_widths.len()]
            .kind
            .header()
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
        let start = self.col_scroll.get().clamp(FIRST_PAGED_COLUMN, last_start);
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
        let n = state.snapshots.len();
        let widths = state.all_col_widths();
        let natural_w = (widths.iter().sum::<usize>() + widths.len().saturating_sub(1)) as u16;
        let heights: Vec<usize> = state
            .snapshots
            .iter()
            .map(|snapshot| snapshot.display_height as usize)
            .collect();
        let expected_page = state.page(natural_w.min(self.screen.whole.width.saturating_sub(2)));
        let frame = Frame {
            title: TITLE,
            border: WHITE,
            background: BACKGROUND,
        };
        let content = (
            natural_w,
            heights.iter().sum::<usize>() as u16 + HEADER_ROWS,
        );
        let (modal, _) = Modal::open_fitting(buf, self.screen, &frame, content, |overflowing| {
            let nav = if overflowing { HINT_SCROLL } else { HINT_NAV };
            HintBar::new(HINT_CLOSE)
                .counted(nav, overflowing.then_some((state.selected + 1, n)))
                .action_if(
                    expected_page.more,
                    format!(
                        "{HINT_COLUMNS} ({}/{})",
                        expected_page.shown_up_to, expected_page.total
                    ),
                )
                .action_if(n > 0, HINT_ENTER_SHOW)
        });

        let page = state.page(modal.body.width);
        let grid = Grid {
            x: modal.body.x,
            widths: page.widths.clone(),
        };
        let (header_y, data) = grid::split_header(modal.body);
        if let Some(y) = header_y {
            let headers: Vec<&str> = page.columns.iter().map(|&c| state.header(c)).collect();
            let style = HeaderStyle {
                text: Style::default()
                    .fg(WHITE)
                    .add_modifier(Modifier::BOLD)
                    .bg(BACKGROUND),
                rule: Style::default().fg(WHITE).bg(BACKGROUND),
            };
            grid::draw_header(buf, &grid, modal.rect, y, &headers, &style);
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
        if page.more {
            open_right_edge(buf, modal.rect, header_y);
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

fn line_range(before: usize, shown: &[usize], room: u16) -> Range<usize> {
    let lines: usize = shown.iter().sum();
    before..before + lines.min(room as usize)
}

fn open_right_edge(buf: &mut Buffer, rect: Rect, header_y: Option<u16>) {
    let right = rect.right().saturating_sub(1);
    let style = Style::default().fg(WHITE).bg(BACKGROUND);
    buf[(right, rect.y)].set_char('─').set_style(style);
    let bottom = rect.bottom().saturating_sub(1);
    buf[(right, bottom)].set_char('─').set_style(style);
    for y in rect.y + 1..bottom {
        buf[(right, y)].set_char(' ').set_style(style);
    }
    if let Some(y) = header_y {
        buf[(right, y + 1)].set_char('─').set_style(style);
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
    let snap = &state.snapshots[index];
    let row_bg = if selected { WHITE } else { BACKGROUND };
    let fg = match (selected, snap.dead) {
        (true, _) => BLACK,
        (false, Some(soul)) => soul,
        (false, None) => STEEL,
    };
    let text = Style::default().fg(fg).bg(row_bg);
    let text_y = row_y + row_h / 2;
    let fixed_count = state.fixed_widths.len();

    for (position, &column) in page.columns.iter().enumerate() {
        let block = grid.cell(position, row_y, row_h);
        for y in block.top()..block.bottom() {
            grid::put(buf, Rect::new(block.x, y, block.width, 1), "", text);
        }
        let cell = grid.cell(position, text_y, 1);
        match state.fixed.get(column) {
            Some(FixedColumn::Name) => grid::put(buf, cell, &snap.name, text),
            Some(FixedColumn::Species) => grid::put(buf, cell, snap.species_name, text),
            Some(FixedColumn::Display) => draw_art(buf, state, index, block),
            Some(FixedColumn::Weight) => {
                grid::put(buf, cell, &fields::format_weight(snap.weight_g), text)
            }
            Some(FixedColumn::Fishtank) => {
                grid::put(buf, cell, snap.tank_name.as_deref().unwrap_or(""), text)
            }
            Some(FixedColumn::Status) => grid::put(buf, cell, snap.status(), text),
            None => {
                let Some(col) = state.fantasy_cols.get(column - fixed_count) else {
                    continue;
                };
                let value = &col.cells[index];
                if col.kind == FieldKind::FavoriteColor {
                    let swatch = Style::default().bg(value.swatch.unwrap_or(BLACK));
                    grid::put(buf, cell, "", swatch);
                } else {
                    grid::put(buf, cell, &value.text, text);
                }
            }
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
    let snap = &state.snapshots[index];
    let animated = state
        .animated_fish
        .as_ref()
        .filter(|_| index == state.selected);
    if snap.is_unfish && snap.display_height > 1 {
        if let Some(fish) = animated.or_else(|| state.fish_clones.get(index)) {
            tank_view::render_multi_row_unfish_at(fish, block.x as i32, block.y as i32, block, buf);
        }
        return;
    }
    let live = animated.map(Fish::line_sprite);
    let art = live.as_ref().unwrap_or(&snap.art);
    let body_y = block.y + art.body_row as u16;
    render_fish_sprite(buf, art, block.x, body_y, block, BACKGROUND);
}

fn fixed_cell_width(column: FixedColumn, snapshot: &FishSnapshot) -> usize {
    match column {
        FixedColumn::Name => table::visual_width(&snapshot.name),
        FixedColumn::Species => table::visual_width(snapshot.species_name),
        FixedColumn::Display => snapshot.display_width,
        FixedColumn::Weight => fields::format_weight(snapshot.weight_g).len(),
        FixedColumn::Fishtank => snapshot.tank_name.as_deref().map_or(0, table::visual_width),
        FixedColumn::Status => table::visual_width(snapshot.status()),
    }
}

fn display_clone(fish: Fish) -> Fish {
    let mut fish = fish.portrait();
    fish.facing = Direction::Left;
    fish
}
