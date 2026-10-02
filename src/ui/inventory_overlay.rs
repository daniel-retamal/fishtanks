use rand::RngExt;
use std::collections::HashMap;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

use crate::colors::{BLACK, STEEL, WHITE};
use crate::loot::{CIRCUIT_BLUEPRINT_NAME, StockItem};
use crate::tank::Blueprint;
use crate::ui::{
    grid::{self, Grid, HEADER_ROWS, HeaderStyle},
    hint_bar::HintBar,
    hints::{HINT_CLOSE, HINT_ENTER_CONSUME, HINT_NAV, HINT_SCROLL},
    layout::{FlexItem, Screen, Scroll, Scrollbar},
    modal::{Frame, Modal},
    table::{visual_width, wrap_words},
};

const TITLE: &str = " Inventory#index ";
const HEADERS: [&str; 4] = ["Item", "Quantity", "Consumable?", "Description"];
const DESCRIPTION_COLUMN: usize = 3;
const MIN_COLUMN_W: u16 = 3;
const MIN_DESCRIPTION_W: u16 = 16;
const BACKGROUND: Color = Color::Reset;
const BLUEPRINT_QTY: u32 = 1;

pub struct InventoryItem {
    pub name: String,
    pub qty: u32,
    pub is_consumable: bool,
    pub desc: String,
}

impl InventoryItem {
    fn short_cells(&self) -> [String; 3] {
        let consumable = if self.is_consumable { "Yes" } else { "No" };
        [
            self.name.clone(),
            self.qty.to_string(),
            consumable.to_string(),
        ]
    }
}

pub struct InventoryState {
    pub selected: usize,
    scroll: Scroll,
    pub items: Vec<InventoryItem>,
}

fn item_desc(stock: StockItem, rng: &mut impl RngExt) -> String {
    match stock {
        StockItem::Consumable(kind) => kind.description().to_string(),
        StockItem::Junk => {
            if rng.random_range(0..10u32) == 0 {
                "Junk... having 100 would be nice".to_string()
            } else {
                "Junk...".to_string()
            }
        }
    }
}

pub fn blueprint_row_name(blueprint: &Blueprint) -> String {
    format!("{} ({})", blueprint.name, CIRCUIT_BLUEPRINT_NAME)
}

fn listing(
    inventory: &HashMap<StockItem, u32>,
    blueprints: &[Blueprint],
    old_descs: &HashMap<String, String>,
    rng: &mut impl RngExt,
) -> Vec<InventoryItem> {
    let mut items: Vec<InventoryItem> = inventory
        .iter()
        .filter(|(_, qty)| **qty > 0)
        .map(|(stock, qty)| {
            let name = stock.display_name().to_string();
            let desc = old_descs
                .get(&name)
                .cloned()
                .unwrap_or_else(|| item_desc(*stock, rng));
            InventoryItem {
                name,
                qty: *qty,
                is_consumable: stock.consumable_at(*qty),
                desc,
            }
        })
        .collect();
    items.extend(blueprints.iter().map(|blueprint| InventoryItem {
        name: blueprint_row_name(blueprint),
        qty: BLUEPRINT_QTY,
        is_consumable: false,
        desc: blueprint.description(),
    }));
    items.sort_by_key(|item| item.name.clone());
    items
}

impl InventoryState {
    pub fn new(
        inventory: &HashMap<StockItem, u32>,
        blueprints: &[Blueprint],
        rng: &mut impl RngExt,
    ) -> Option<Self> {
        let items = listing(inventory, blueprints, &HashMap::new(), rng);
        if items.is_empty() {
            return None;
        }
        Some(Self {
            selected: 0,
            scroll: Scroll::default(),
            items,
        })
    }

    pub fn update_from(
        &mut self,
        inventory: &HashMap<StockItem, u32>,
        blueprints: &[Blueprint],
        rng: &mut impl RngExt,
    ) {
        let old_descs: HashMap<String, String> = self
            .items
            .iter()
            .map(|item| (item.name.clone(), item.desc.clone()))
            .collect();
        self.items = listing(inventory, blueprints, &old_descs, rng);
        self.selected = self.selected.min(self.items.len().saturating_sub(1));
    }

    pub fn scroll_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        if self.selected + 1 < self.items.len() {
            self.selected += 1;
        }
    }

    fn columns(&self) -> Vec<FlexItem> {
        let mut columns: Vec<FlexItem> = (0..DESCRIPTION_COLUMN)
            .map(|column| {
                let widths = || {
                    self.items
                        .iter()
                        .map(move |item| visual_width(&item.short_cells()[column]))
                };
                let content = widths().max().unwrap_or(0) as u16;
                grid::text_column(HEADERS[column], widths(), content.max(MIN_COLUMN_W))
            })
            .collect();
        columns.push(
            grid::text_column(
                HEADERS[DESCRIPTION_COLUMN],
                self.items.iter().map(|item| visual_width(&item.desc)),
                MIN_DESCRIPTION_W,
            )
            .gives_first(),
        );
        columns
    }
}

pub struct InventoryOverlay<'a> {
    state: &'a InventoryState,
    screen: Screen,
}

impl<'a> InventoryOverlay<'a> {
    pub fn new(state: &'a InventoryState, screen: Screen) -> Self {
        Self { state, screen }
    }
}

impl Widget for InventoryOverlay<'_> {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        let state = self.state;
        let n = state.items.len();
        let columns = state.columns();
        let hints = |overflowing: bool| {
            let nav = if overflowing { HINT_SCROLL } else { HINT_NAV };
            let consumable = state
                .items
                .get(state.selected)
                .is_some_and(|item| item.is_consumable);
            HintBar::new(HINT_CLOSE)
                .counted(nav, overflowing.then_some((state.selected + 1, n)))
                .action_if(consumable, HINT_ENTER_CONSUME)
        };
        let natural_w = Grid::natural_width(&columns);
        let expected_inner_w = natural_w.min(self.screen.whole.width.saturating_sub(2));
        let expected_grid = Grid::fit(0, expected_inner_w, &columns);
        let heights = row_heights(state, &expected_grid);
        let frame = Frame {
            title: TITLE,
            border: WHITE,
            background: BACKGROUND,
        };
        let content_h = heights.iter().sum::<usize>() as u16 + HEADER_ROWS;
        let (modal, _) =
            Modal::open_fitting(buf, self.screen, &frame, (natural_w, content_h), hints);

        let grid = Grid::fit(modal.body.x, modal.body.width, &columns);
        let heights = row_heights(state, &grid);
        let (header_y, data) = grid::split_header(modal.body);
        if let Some(y) = header_y {
            let style = HeaderStyle {
                text: Style::default()
                    .fg(WHITE)
                    .add_modifier(Modifier::BOLD)
                    .bg(BACKGROUND),
                rule: Style::default().fg(WHITE).bg(BACKGROUND),
            };
            grid::draw_header(buf, &grid, modal.rect, y, &HEADERS, &style);
        }

        let shown = state
            .scroll
            .follow(&heights, state.selected, data.height as usize);
        let mut y = data.y;
        for index in shown.clone() {
            let room = data.bottom().saturating_sub(y);
            let height = (heights[index] as u16).min(room);
            if height == 0 {
                break;
            }
            draw_item(
                buf,
                &grid,
                &state.items[index],
                y,
                height,
                index == state.selected,
            );
            y += height;
        }
        let lines_before: usize = heights[..shown.start].iter().sum();
        let lines_shown: usize = heights[shown.clone()].iter().sum();
        Scrollbar {
            x: modal.scrollbar_x(),
            top: data.y,
            height: data.height,
        }
        .draw(
            buf,
            lines_before..lines_before + lines_shown.min(data.height as usize),
            heights.iter().sum(),
            WHITE,
        );
    }
}

fn row_heights(state: &InventoryState, grid: &Grid) -> Vec<usize> {
    let description_w = grid.text_width(DESCRIPTION_COLUMN);
    state
        .items
        .iter()
        .map(|item| wrap_words(&item.desc, description_w).len().max(1))
        .collect()
}

fn draw_item(
    buf: &mut Buffer,
    grid: &Grid,
    item: &InventoryItem,
    y: u16,
    height: u16,
    selected: bool,
) {
    let row_bg = if selected { WHITE } else { BACKGROUND };
    let fg = if selected { BLACK } else { STEEL };
    let style = Style::default().fg(fg).bg(row_bg);
    for column in 0..HEADERS.len() {
        for row in 0..height {
            grid::put(buf, grid.cell(column, y + row, 1), "", style);
        }
    }
    let centre_y = y + height.saturating_sub(1) / 2;
    for (column, text) in item.short_cells().iter().enumerate() {
        grid::put(buf, grid.cell(column, centre_y, 1), text, style);
    }
    let description_w = grid.text_width(DESCRIPTION_COLUMN);
    for (row, line) in wrap_words(&item.desc, description_w)
        .iter()
        .take(height as usize)
        .enumerate()
    {
        grid::put(
            buf,
            grid.cell(DESCRIPTION_COLUMN, y + row as u16, 1),
            line,
            style,
        );
    }
    grid.draw_rules(buf, y, height, Style::default().fg(WHITE).bg(row_bg));
}
