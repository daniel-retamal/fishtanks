use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

use crate::colors::{BLACK, STEEL, WHITE};
use crate::consumable::ConsumeTarget;
use crate::ui::fields::FED_HEADER;
use crate::ui::{
    grid::{self, Grid, HEADER_ROWS, HeaderStyle},
    hint_bar::HintBar,
    hints::{HINT_CLOSE, HINT_NAV, HINT_SCROLL},
    layout::{FlexItem, Screen, Scroll, Scrollbar},
    modal::{Frame, Modal},
    table::visual_width,
};

pub struct ConsumePickerEntry {
    pub fish_name: String,
    pub species_display: String,
    pub tank_name: String,
    pub fed: Option<String>,
    pub tank_idx: usize,
    pub fish_idx: usize,
}

impl ConsumePickerEntry {
    fn cells(&self) -> Vec<&str> {
        let mut cells = vec![
            self.fish_name.as_str(),
            &self.species_display,
            &self.tank_name,
        ];
        cells.extend(self.fed.as_deref());
        cells
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConsumePickerSource {
    FromInventory,
    FromCommand,
    FromFoundry,
}

pub struct ConsumePickerState {
    pub target: ConsumeTarget,
    label: String,
    pub remaining: u32,
    pub entries: Vec<ConsumePickerEntry>,
    pub selected: usize,
    scroll: Scroll,
    pub source: ConsumePickerSource,
}

impl ConsumePickerState {
    pub fn new(
        target: ConsumeTarget,
        label: String,
        remaining: u32,
        entries: Vec<ConsumePickerEntry>,
        source: ConsumePickerSource,
    ) -> Option<Self> {
        if entries.is_empty() {
            return None;
        }
        Some(Self {
            target,
            label,
            remaining,
            entries,
            selected: 0,
            scroll: Scroll::default(),
            source,
        })
    }

    pub fn item_name(&self) -> &str {
        &self.label
    }

    fn headers(&self) -> Vec<&'static str> {
        let mut headers = HEADERS.to_vec();
        if self.target.fattens() {
            headers.push(FED_HEADER);
        }
        headers
    }

    pub fn scroll_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }

    pub fn refresh_after_consume(
        &mut self,
        remaining: u32,
        entries_fn: impl Fn() -> Vec<ConsumePickerEntry>,
    ) {
        self.remaining = remaining;
        let prev_name = self.entries.get(self.selected).map(|e| e.fish_name.clone());
        self.entries = entries_fn();
        if let Some(name) = prev_name
            && let Some(pos) = self.entries.iter().position(|e| e.fish_name == name)
        {
            self.selected = pos;
            return;
        }
        if self.selected >= self.entries.len() {
            self.selected = self.entries.len().saturating_sub(1);
        }
    }
}

const MIN_COLUMN_W: u16 = 3;
const HEADERS: [&str; 3] = ["Name", "Species", "Fishtank"];
const BACKGROUND: Color = Color::Reset;

pub struct ConsumePickerOverlay<'a> {
    pub state: &'a ConsumePickerState,
    pub screen: Screen,
}

impl Widget for ConsumePickerOverlay<'_> {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        let state = self.state;
        let n = state.entries.len();
        let headers = state.headers();
        let columns: Vec<FlexItem> = (0..headers.len())
            .map(|column| {
                grid::text_column(
                    headers[column],
                    state
                        .entries
                        .iter()
                        .map(|entry| visual_width(entry.cells()[column])),
                    MIN_COLUMN_W,
                )
            })
            .collect();
        let title = state.target.header(state.item_name(), state.remaining);
        let frame = Frame {
            title: &title,
            border: WHITE,
            background: BACKGROUND,
        };
        let content = (Grid::natural_width(&columns), n as u16 + HEADER_ROWS);
        let (modal, _) = Modal::open_fitting(buf, self.screen, &frame, content, |overflowing| {
            let nav = if overflowing { HINT_SCROLL } else { HINT_NAV };
            HintBar::new(HINT_CLOSE)
                .counted(nav, overflowing.then_some((state.selected + 1, n)))
                .action(state.target.confirm_hint())
        });

        let grid = Grid::fit(modal.body.x, modal.body.width, &columns);
        let (header_y, data) = grid::split_header(modal.body);
        if let Some(y) = header_y {
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
            .follow(&vec![1; n], state.selected, data.height as usize);
        for (row, index) in shown.clone().enumerate() {
            let selected = index == state.selected;
            let row_bg = if selected { WHITE } else { BACKGROUND };
            let fg = if selected { BLACK } else { STEEL };
            let y = data.y + row as u16;
            for (column, text) in state.entries[index].cells().iter().enumerate() {
                grid::put(
                    buf,
                    grid.cell(column, y, 1),
                    text,
                    Style::default().fg(fg).bg(row_bg),
                );
            }
            grid.draw_rules(buf, y, 1, Style::default().fg(WHITE).bg(row_bg));
        }
        Scrollbar {
            x: modal.scrollbar_x(),
            top: data.y,
            height: data.height,
        }
        .draw(buf, shown, n, WHITE);
    }
}
