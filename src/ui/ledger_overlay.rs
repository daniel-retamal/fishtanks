use std::cell::Cell;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

use crate::colors::{STEEL, WHITE};
use crate::economy::{Money, grouped};
use crate::ledger::{Direction, Flow, LEDGER_MINUTES, Ledger, Tally};
use crate::ui::{
    command_bar::metric,
    grid::{self, Grid, HEADER_ROWS, HeaderStyle},
    hint_bar::HintBar,
    hints::{HINT_CLOSE, HINT_SCROLL},
    layout::{Screen, Scrollbar},
    modal::{Frame, Modal},
    table::{self, NOTHING, visual_width},
};

const TITLE: &str = " Ledger#show ";
const HEADERS: [&str; 3] = ["Line", "Last hour", "Since launch"];
const REVENUE: &str = "Revenue";
const EXPENSES: &str = "Expenses";
const NET: &str = "Net";
const PER_MINUTE: &str = "Per minute";
const MIN_LABEL_W: u16 = 5;
const MIN_MONEY_W: u16 = 6;
const RULE_LINE: &str = "──";
const BACKGROUND: Color = Color::Reset;
const RULE_CLOSES_UP: char = '┴';

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Amount {
    Money(i128),
    Nothing,
}

impl Amount {
    fn full(self) -> String {
        match self {
            Amount::Nothing => NOTHING.to_string(),
            Amount::Money(value) => format!("{}${}", sign(value), grouped(value.unsigned_abs())),
        }
    }

    fn compact(self) -> String {
        match self {
            Amount::Nothing => NOTHING.to_string(),
            Amount::Money(value) => {
                let magnitude = value.unsigned_abs();
                format!("{}${}", sign(value), metric(magnitude))
            }
        }
    }

    fn fitted(self, room: usize) -> String {
        let full = self.full();
        if visual_width(&full) <= room {
            return full;
        }
        self.compact()
    }
}

fn sign(value: i128) -> &'static str {
    if value < 0 { "-" } else { "" }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Row {
    Section(Option<&'static str>),
    Line {
        label: String,
        hour: Amount,
        launch: Amount,
        total: bool,
    },
}

pub struct LedgerState {
    rows: Vec<Row>,
    top: usize,
    room: Cell<usize>,
}

fn shown_flows(tally: &Tally, direction: Direction, godsend: bool) -> Vec<Flow> {
    tally
        .flows(direction)
        .filter(|flow| godsend || !flow.is_godsend())
        .collect()
}

fn net(tally: &Tally, flows_in: &[Flow], flows_out: &[Flow]) -> i128 {
    let side = |direction: Direction, flows: &[Flow]| -> i128 {
        flows
            .iter()
            .map(|&flow| signed(tally.line(direction, flow)))
            .sum()
    };
    side(Direction::In, flows_in) - side(Direction::Out, flows_out)
}

fn signed(money: Money) -> i128 {
    i128::try_from(money).unwrap_or(i128::MAX)
}

fn per_minute(net: i128, minutes: f32) -> Amount {
    if minutes <= 0.0 {
        return Amount::Nothing;
    }
    Amount::Money((net as f64 / f64::from(minutes)).round() as i128)
}

impl LedgerState {
    pub fn new(ledger: &Ledger, godsend: bool) -> Self {
        let hour = ledger.last_hour();
        let launch = ledger.since_launch();
        let flows_in = shown_flows(launch, Direction::In, godsend);
        let flows_out = shown_flows(launch, Direction::Out, godsend);
        let mut rows = Vec::new();
        for (title, direction, flows) in [
            (REVENUE, Direction::In, &flows_in),
            (EXPENSES, Direction::Out, &flows_out),
        ] {
            rows.push(Row::Section(Some(title)));
            if flows.is_empty() {
                rows.push(Row::Line {
                    label: NOTHING.to_string(),
                    hour: Amount::Nothing,
                    launch: Amount::Nothing,
                    total: false,
                });
            }
            for &flow in flows.iter() {
                rows.push(Row::Line {
                    label: flow.label().to_string(),
                    hour: Amount::Money(signed(hour.line(direction, flow))),
                    launch: Amount::Money(signed(launch.line(direction, flow))),
                    total: false,
                });
            }
        }
        let hour_net = net(&hour, &flows_in, &flows_out);
        let launch_net = net(launch, &flows_in, &flows_out);
        let minutes = ledger.minutes_open();
        rows.push(Row::Section(None));
        rows.push(Row::Line {
            label: NET.to_string(),
            hour: Amount::Money(hour_net),
            launch: Amount::Money(launch_net),
            total: true,
        });
        rows.push(Row::Line {
            label: PER_MINUTE.to_string(),
            hour: per_minute(hour_net, minutes.min(LEDGER_MINUTES as f32)),
            launch: per_minute(launch_net, minutes),
            total: true,
        });
        Self {
            rows,
            top: 0,
            room: Cell::new(0),
        }
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn scroll_up(&mut self) {
        self.top = self.top.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        if self.top + self.room.get() < self.rows.len() {
            self.top += 1;
        }
    }

    fn top(&self, room: usize) -> usize {
        self.top.min(self.rows.len().saturating_sub(room))
    }
}

pub struct LedgerOverlay<'a> {
    state: &'a LedgerState,
    screen: Screen,
}

impl<'a> LedgerOverlay<'a> {
    pub fn new(state: &'a LedgerState, screen: Screen) -> Self {
        Self { state, screen }
    }
}

fn money_width(rows: &[Row], pick: impl Fn(&Row) -> Option<Amount>) -> usize {
    rows.iter()
        .filter_map(pick)
        .map(|amount| visual_width(&amount.full()))
        .max()
        .unwrap_or(0)
}

impl Widget for LedgerOverlay<'_> {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        let state = self.state;
        let rows = &state.rows;
        let total = rows.len();
        let labels = rows.iter().filter_map(|row| match row {
            Row::Line { label, .. } => Some(visual_width(label)),
            Row::Section(_) => None,
        });
        let hours = money_width(rows, |row| match row {
            Row::Line { hour, .. } => Some(*hour),
            Row::Section(_) => None,
        });
        let launches = money_width(rows, |row| match row {
            Row::Line { launch, .. } => Some(*launch),
            Row::Section(_) => None,
        });
        let columns = [
            grid::text_column(HEADERS[0], labels, MIN_LABEL_W),
            grid::text_column(HEADERS[1], std::iter::once(hours), MIN_MONEY_W),
            grid::text_column(HEADERS[2], std::iter::once(launches), MIN_MONEY_W),
        ];
        let frame = Frame {
            title: TITLE,
            border: WHITE,
            background: BACKGROUND,
        };
        let content = (Grid::natural_width(&columns), total as u16 + HEADER_ROWS);
        let (modal, _) = Modal::open_fitting(buf, self.screen, &frame, content, |overflowing| {
            let top = state.top(state.room.get());
            let bar = HintBar::new(HINT_CLOSE);
            if !overflowing {
                return bar;
            }
            bar.counted(HINT_SCROLL, Some((top + 1, total)))
        });
        let grid = Grid::fit(modal.body.x, modal.body.width, &columns);
        let (header_y, data) = grid::split_header(modal.body);
        let bold = Style::default()
            .fg(WHITE)
            .add_modifier(Modifier::BOLD)
            .bg(BACKGROUND);
        let rule = Style::default().fg(WHITE).bg(BACKGROUND);
        if let Some(y) = header_y {
            let style = HeaderStyle { text: bold, rule };
            grid::draw_header(buf, &grid, modal.rect, y, &HEADERS, &style);
        }
        let room = data.height as usize;
        state.room.set(room);
        let top = state.top(room);
        let shown = top..(top + room).min(total);
        for (offset, index) in shown.clone().enumerate() {
            let y = data.y + offset as u16;
            match &rows[index] {
                Row::Section(label) => {
                    table::draw_box_separator(
                        buf,
                        modal.rect.x,
                        y,
                        modal.rect.width,
                        &grid.rule_xs(),
                        WHITE,
                        BACKGROUND,
                    );
                    if offset + 1 == shown.len() {
                        for x in grid.rule_xs() {
                            buf[(x, y)].set_char(RULE_CLOSES_UP).set_style(rule);
                        }
                    }
                    if let Some(label) = label {
                        let text = format!("{RULE_LINE} {label} ");
                        let label_x = modal.rect.x + 1;
                        let first_junction = grid
                            .rule_xs()
                            .first()
                            .copied()
                            .unwrap_or(modal.rect.right().saturating_sub(1));
                        let room = first_junction.saturating_sub(label_x) as usize;
                        let text = table::ellipsize(&text, room);
                        buf.set_stringn(label_x, y, text, room, rule);
                    }
                }
                Row::Line {
                    label,
                    hour,
                    launch,
                    total,
                } => {
                    let style = if *total {
                        bold
                    } else {
                        Style::default().fg(STEEL).bg(BACKGROUND)
                    };
                    grid::put(buf, grid.cell(0, y, 1), label, style);
                    for (column, amount) in [(1, hour), (2, launch)] {
                        let text = amount.fitted(grid.text_width(column));
                        grid::put_money(buf, grid.cell(column, y, 1), &text, style);
                    }
                    grid.draw_rules(buf, y, 1, rule);
                }
            }
        }
        Scrollbar {
            x: modal.scrollbar_x(),
            top: data.y,
            height: data.height,
        }
        .draw(buf, shown, total, WHITE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_is_grouped_by_thousands_and_signed() {
        assert_eq!(Amount::Money(1_234_567).full(), "$1,234,567");
        assert_eq!(Amount::Money(-980).full(), "-$980");
        assert_eq!(Amount::Money(0).full(), "$0");
        assert_eq!(Amount::Nothing.full(), NOTHING);
    }

    #[test]
    fn money_too_wide_for_its_column_turns_compact_instead_of_vanishing() {
        let fortune = Amount::Money(1_234_567_890_123);
        assert_eq!(fortune.fitted(40), "$1,234,567,890,123");
        assert_eq!(fortune.fitted(6), "$1.2T");
    }

    #[test]
    fn an_empty_statement_says_so_on_both_sides_and_nets_nothing() {
        let state = LedgerState::new(&Ledger::new(), false);
        let nothing = Row::Line {
            label: NOTHING.to_string(),
            hour: Amount::Nothing,
            launch: Amount::Nothing,
            total: false,
        };
        assert_eq!(
            state.rows().iter().filter(|row| **row == nothing).count(),
            2
        );
        assert!(state.rows().contains(&Row::Line {
            label: NET.to_string(),
            hour: Amount::Money(0),
            launch: Amount::Money(0),
            total: true,
        }));
    }

    #[test]
    fn a_godsend_is_on_the_statement_only_in_debug_mode() {
        let mut ledger = Ledger::new();
        ledger.record(Direction::In, Flow::Godsend, 40_000);
        ledger.record(Direction::In, Flow::FishSales, 12);
        let player = LedgerState::new(&ledger, false);
        let labelled = |state: &LedgerState, label: &str| {
            state
                .rows()
                .iter()
                .any(|row| matches!(row, Row::Line { label: l, .. } if l == label))
        };
        assert!(!labelled(&player, Flow::Godsend.label()));
        assert!(labelled(&player, Flow::FishSales.label()));
        assert!(labelled(
            &LedgerState::new(&ledger, true),
            Flow::Godsend.label()
        ));
    }
}
