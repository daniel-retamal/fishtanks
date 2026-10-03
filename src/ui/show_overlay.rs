use rand::RngExt;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

use crate::colors::WHITE;
use crate::fishes::fish::{Direction, Fish};
use crate::tank::TankKind;
use crate::ui::{
    draw_fish_centred,
    fields::{self, FieldKind},
    fish_art_height,
    hint_bar::HintBar,
    hints::{HINT_CLOSE, HINT_NAV, HINT_RETURN},
    layout::{Screen, Scroll, Scrollbar},
    panels::{PanelSpec, Panels, Reach},
    table::{self, wrap_words},
};

const BACKGROUND: Color = Color::Reset;
const TITLE: &str = " FishResource#show ";
const FISH_PAD: u16 = 1;
const FISH_INNER_HEIGHT: u16 = 3;
const MIN_BODY_WIDTH: u16 = 38;
const BODY_MIN_WIDTH_BESIDE: u16 = 18;
const TEXT_PAD: u16 = 1;
const SWATCH_W: u16 = 6;
const FIELD_GAP_ROWS: usize = 1;
const FIELD_COUNT_MIN: usize = 3;
const FIELD_COUNT_MAX: usize = 6;

pub enum ShowSource {
    FromIndex,
    FromCommand,
}

struct ShowField {
    label: &'static str,
    value: String,
    swatch: Option<Color>,
}

impl ShowField {
    fn lines(&self, text_w: u16) -> Vec<String> {
        if self.swatch.is_some() {
            return vec![String::new()];
        }
        wrap_words(&self.value, text_w as usize)
    }

    fn height(&self, text_w: u16) -> usize {
        1 + self.lines(text_w).len()
    }
}

pub struct ShowState {
    pub fish: Fish,
    fields: Vec<ShowField>,
    scroll: Scroll,
    pub source: ShowSource,
}

impl ShowState {
    pub fn new(
        fish: &Fish,
        tank_name: &str,
        tank_kind: TankKind,
        all_names: &[String],
        source: ShowSource,
        all: bool,
        rng: &mut impl RngExt,
    ) -> Self {
        let mut display_fish = fish.portrait();
        display_fish.facing = Direction::Right;

        let mut show_fields: Vec<ShowField> = vec![
            ShowField {
                label: "Name",
                value: fish.name.clone(),
                swatch: None,
            },
            ShowField {
                label: "Species",
                value: fish.species.display_name().to_string(),
                swatch: None,
            },
            ShowField {
                label: "Weight",
                value: fields::format_weight(fish.weight_g),
                swatch: None,
            },
            ShowField {
                label: "Fishtank",
                value: tank_name.to_string(),
                swatch: None,
            },
            ShowField {
                label: "Tank Type",
                value: tank_kind.display_name().to_string(),
                swatch: None,
            },
        ];

        if fish.devil_marked {
            show_fields.push(ShowField {
                label: "Marked by the Devil",
                value: "Yes".to_string(),
                swatch: None,
            });
        }

        let kinds: Vec<FieldKind> = if all {
            FieldKind::all().to_vec()
        } else {
            let count = rng.random_range(FIELD_COUNT_MIN..=FIELD_COUNT_MAX);
            let mut avail: Vec<FieldKind> = FieldKind::all().to_vec();
            (0..count.min(avail.len()))
                .map(|_| {
                    let idx = rng.random_range(0..avail.len());
                    avail.remove(idx)
                })
                .collect()
        };
        for kind in kinds {
            let field = fields::field_value(kind, fish, all_names, rng);
            show_fields.push(ShowField {
                label: kind.header(),
                value: field.text,
                swatch: field.swatch,
            });
        }

        if let Some(ref mr) = fish.mutations {
            if !mr.partners.is_empty() {
                show_fields.push(ShowField {
                    label: "Cytokinesis Partners",
                    value: mr.partners.join(", "),
                    swatch: None,
                });
            }
            show_fields.push(ShowField {
                label: "Mutation Count",
                value: mr.count.to_string(),
                swatch: None,
            });
            let history = if mr.history.is_empty() {
                super::table::NOTHING.to_string()
            } else {
                mr.history.join(", ")
            };
            show_fields.push(ShowField {
                label: "Mutation History",
                value: history,
                swatch: None,
            });
        }

        Self {
            fish: display_fish,
            fields: show_fields,
            scroll: Scroll::default(),
            source,
        }
    }

    pub fn tick_animation(&mut self, dt: f32) {
        self.fish.tick_animation(dt);
    }

    pub fn scroll_up(&mut self) {
        self.scroll.nudge(-1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll.nudge(1);
    }

    fn heights(&self, text_w: u16) -> Vec<usize> {
        let last = self.fields.len().saturating_sub(1);
        self.fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                field.height(text_w) + if index < last { FIELD_GAP_ROWS } else { 0 }
            })
            .collect()
    }

    fn content_w(&self) -> u16 {
        let widest = self
            .fields
            .iter()
            .flat_map(|f| [table::visual_width(f.label), table::visual_width(&f.value)])
            .max()
            .unwrap_or(0) as u16;
        MIN_BODY_WIDTH.max(widest + TEXT_PAD * 2)
    }
}

fn side_size(fish: &Fish) -> (u16, u16) {
    (
        fish.display_width as u16 + FISH_PAD * 2,
        fish_art_height(fish, FISH_INNER_HEIGHT),
    )
}

fn text_width(body_w: u16) -> u16 {
    body_w.saturating_sub(TEXT_PAD * 2).max(1)
}

pub struct ShowOverlay<'a> {
    state: &'a ShowState,
    screen: Screen,
}

impl<'a> ShowOverlay<'a> {
    pub fn new(state: &'a ShowState, screen: Screen) -> Self {
        Self { state, screen }
    }
}

impl Widget for ShowOverlay<'_> {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        let state = self.state;
        let border_color = state
            .fish
            .species
            .config()
            .flavour
            .card_color
            .resolve(WHITE, state.fish.color);
        let close = match state.source {
            ShowSource::FromIndex => HINT_RETURN,
            ShowSource::FromCommand => HINT_CLOSE,
        };
        let rows_for = |body_w: u16| state.heights(text_width(body_w)).iter().sum::<usize>() as u16;
        let spec_for = |hints| PanelSpec {
            title: TITLE,
            title_style: Style::default()
                .fg(WHITE)
                .add_modifier(Modifier::BOLD)
                .bg(BACKGROUND),
            border: Style::default().fg(border_color).bg(BACKGROUND),
            background: BACKGROUND,
            side: side_size(&state.fish),
            body_w: state.content_w(),
            body_min_w: BODY_MIN_WIDTH_BESIDE,
            body_rows: &rows_for,
            hints,
            reach: Reach::Tab,
        };
        let calm = HintBar::new(close);
        let measured = Panels::measure(self.screen, &spec_for(&calm));
        let heights = state.heights(text_width(measured.body.width));
        let total_rows: usize = heights.iter().sum();
        let overflowing = total_rows > measured.body.height as usize;
        let hints = HintBar::new(close).action_if(overflowing, HINT_NAV);
        let panels = Panels::open(buf, self.screen, &spec_for(&hints));

        draw_fish_centred(buf, &state.fish, panels.side, BACKGROUND);

        let text_w = text_width(panels.body.width);
        let heights = state.heights(text_w);
        let room = panels.body.height as usize;
        let last_start = (0..heights.len())
            .find(|&start| heights[start..].iter().sum::<usize>() <= room)
            .unwrap_or(heights.len().saturating_sub(1));
        let first = state.scroll.settle(last_start);
        let x = panels.body.x + TEXT_PAD;
        let bold = Style::default()
            .fg(WHITE)
            .add_modifier(Modifier::BOLD)
            .bg(BACKGROUND);
        let plain = Style::default().fg(WHITE).bg(BACKGROUND);
        let mut y = panels.body.y;
        let bottom = panels.body.bottom();
        let mut shown = 0;
        for field in state.fields.iter().skip(first) {
            if y >= bottom {
                break;
            }
            buf.set_stringn(
                x,
                y,
                table::ellipsize(field.label, text_w as usize),
                text_w as usize,
                bold,
            );
            for (row, line) in field.lines(text_w).iter().enumerate() {
                let line_y = y + 1 + row as u16;
                if line_y >= bottom {
                    break;
                }
                if let Some(swatch) = field.swatch {
                    for dx in 0..SWATCH_W.min(text_w) {
                        buf[(x + dx, line_y)].set_char(' ').set_bg(swatch);
                    }
                    continue;
                }
                buf.set_stringn(x, line_y, line, text_w as usize, plain);
            }
            y += (field.height(text_w) + FIELD_GAP_ROWS) as u16;
            shown += 1;
        }
        let rows_before: usize = heights[..first].iter().sum();
        Scrollbar {
            x: panels.right_x(),
            top: panels.body.y,
            height: panels.body.height,
        }
        .draw(
            buf,
            rows_before..(rows_before + room).min(total_rows),
            heights.iter().sum(),
            border_color,
        );
        if shown == 0 {
            state.scroll.reset();
        }
    }
}
