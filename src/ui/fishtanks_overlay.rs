use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

use crate::colors::{BLACK, STEEL, WHITE};
use crate::tank::{Tank, TankKind};
use crate::ui::{
    grid::{self, Grid, HEADER_ROWS, HeaderStyle},
    hint_bar::HintBar,
    hints::{HINT_CLOSE, HINT_ENTER_SWITCH, HINT_NAV, HINT_SCROLL},
    layout::{Screen, Scroll, Scrollbar},
    modal::{Frame, Modal},
    table::visual_width,
};

const TITLE: &str = " Fishtank#index ";
const HEADERS: [&str; 3] = ["Name", "Type", "Fishes"];
const MIN_COLUMN_W: u16 = 3;
const BACKGROUND: Color = Color::Reset;

struct FishtanksEntry {
    name: String,
    kind: TankKind,
    fish_count: usize,
    capacity: usize,
}

impl FishtanksEntry {
    fn cells(&self) -> [String; 3] {
        [
            self.name.clone(),
            self.kind.display_name().to_string(),
            format!("{}/{}", self.fish_count, self.capacity),
        ]
    }
}

pub struct FishtanksState {
    entries: Vec<FishtanksEntry>,
    pub selected: usize,
    scroll: Scroll,
    pub current_tank: usize,
}

impl FishtanksState {
    pub fn new(tanks: &[Tank], current_tank: usize) -> Self {
        let entries = tanks
            .iter()
            .map(|t| FishtanksEntry {
                name: t.name.clone(),
                kind: t.kind,
                fish_count: t.fish.len(),
                capacity: t.capacity(),
            })
            .collect();
        Self {
            entries,
            selected: current_tank,
            scroll: Scroll::default(),
            current_tank,
        }
    }

    pub fn scroll_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }
}

pub struct FishtanksOverlay<'a> {
    state: &'a FishtanksState,
    screen: Screen,
}

impl<'a> FishtanksOverlay<'a> {
    pub fn new(state: &'a FishtanksState, screen: Screen) -> Self {
        Self { state, screen }
    }
}

impl Widget for FishtanksOverlay<'_> {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        let state = self.state;
        let n = state.entries.len();
        let rows: Vec<[String; 3]> = state.entries.iter().map(FishtanksEntry::cells).collect();
        let columns: Vec<_> = (0..HEADERS.len())
            .map(|column| {
                grid::text_column(
                    HEADERS[column],
                    rows.iter().map(|cells| visual_width(&cells[column])),
                    MIN_COLUMN_W,
                )
            })
            .collect();
        let frame = Frame {
            title: TITLE,
            border: WHITE,
            background: BACKGROUND,
        };
        let content = (Grid::natural_width(&columns), n as u16 + HEADER_ROWS);
        let (modal, _) = Modal::open_fitting(buf, self.screen, &frame, content, |overflowing| {
            let nav = if overflowing { HINT_SCROLL } else { HINT_NAV };
            HintBar::new(HINT_CLOSE)
                .counted(nav, overflowing.then_some((state.selected + 1, n)))
                .action_if(state.selected != state.current_tank, HINT_ENTER_SWITCH)
        });

        let mut grid = Grid::fit(modal.body.x, modal.body.width, &columns);
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
        } else {
            grid.bare = true;
        }

        let shown = state
            .scroll
            .follow(&vec![1; n], state.selected, data.height as usize);
        for (row, index) in shown.clone().enumerate() {
            let selected = index == state.selected;
            let row_bg = if selected { WHITE } else { BACKGROUND };
            let fg = if selected { BLACK } else { STEEL };
            let y = data.y + row as u16;
            for (column, text) in rows[index].iter().enumerate() {
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
