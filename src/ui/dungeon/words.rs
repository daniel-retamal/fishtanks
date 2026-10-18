use ratatui::buffer::Buffer;
use ratatui::style::Style;

use crate::colors::{DARK_GRAY, WHITE};
use crate::dungeon::screen::Words;
use crate::ui::hint_bar::HintBar;
use crate::ui::hints::{HINT_ENTER_SAY, HINT_ESC_CLOSE};
use crate::ui::layout::Screen;
use crate::ui::modal::{Modal, ModalSpec};
use crate::ui::table::{scrolled_to_fit, visual_width, wrap_words};
use crate::ui::text_input::draw_text_cursor;

use super::battle::{BACKGROUND, PAD};

const BORDER: ratatui::style::Color = WHITE;
const QUOTE: char = '"';
const PROMPT_W: u16 = 2;
const CURSOR_W: u16 = 1;
const FIELD_ROWS: u16 = 1;
const GAP_ROWS: u16 = 1;

fn title(words: &Words) -> String {
    format!(" Words to {} ", words.listener)
}

fn hints() -> HintBar {
    HintBar::new(HINT_ESC_CLOSE).action(HINT_ENTER_SAY)
}

fn quoted(words: &Words) -> String {
    format!("{QUOTE}{}{QUOTE}", words.line)
}

pub(super) fn draw(buf: &mut Buffer, words: &Words, cursor_visible: bool, screen: Screen) {
    let hints = hints();
    let title = title(words);
    let quoted = quoted(words);
    let field_w = visual_width(words.line) as u16 + PROMPT_W + CURSOR_W;
    let natural_w = (visual_width(&quoted) as u16).max(field_w) + PAD * 2;
    let inner_w = Modal::inner_width(screen, &title, natural_w, &hints);
    let text_w = inner_w.saturating_sub(PAD * 2).max(1);
    let lines = wrap_words(&quoted, usize::from(text_w));
    let content_h = lines.len() as u16 + GAP_ROWS + FIELD_ROWS;
    let modal = Modal::open(
        buf,
        screen,
        &ModalSpec {
            title: &title,
            border: BORDER,
            background: BACKGROUND,
            content_w: natural_w,
            content_h,
            hints: &hints,
        },
    );
    let body = modal.body;
    if body.height == 0 {
        return;
    }
    let x = body.x + PAD;
    let field_y = body.bottom() - FIELD_ROWS;
    draw_text_cursor(
        buf,
        &words.input,
        cursor_visible,
        x,
        field_y,
        text_w,
        BACKGROUND,
    );
    draw_ghost(buf, words, x, field_y, text_w);
    let room = body.height - FIELD_ROWS;
    let shown = (lines.len() as u16).min(room);
    let gap = u16::from(room > shown).min(GAP_ROWS);
    let top = field_y - gap - shown;
    let white = Style::default().fg(WHITE).bg(BACKGROUND);
    for (row, line) in lines
        .iter()
        .skip(lines.len() - usize::from(shown))
        .enumerate()
    {
        buf.set_string(x, top + row as u16, line, white);
    }
}

fn draw_ghost(buf: &mut Buffer, words: &Words, x: u16, y: u16, w: u16) {
    let input = &words.input;
    if input.cursor != input.as_str().len() {
        return;
    }
    let ghost = words.ghost();
    let room = usize::from(w.saturating_sub(PROMPT_W + CURSOR_W));
    let typed_w = visual_width(scrolled_to_fit(input.as_str(), room)) as u16;
    let cursor_x = x + PROMPT_W + typed_w;
    let mut chars = ghost.chars();
    let Some(first) = chars.next() else {
        return;
    };
    let right = x + w;
    if cursor_x < right {
        buf[(cursor_x, y)].set_char(first);
        if buf[(cursor_x, y)].bg != WHITE {
            buf[(cursor_x, y)].set_fg(DARK_GRAY);
        }
    }
    let rest: String = chars.collect();
    let after = cursor_x + CURSOR_W;
    if after < right && !rest.is_empty() {
        let clip = usize::from(right - after);
        let shown: String = rest.chars().take(clip).collect();
        buf.set_string(
            after,
            y,
            shown,
            Style::default().fg(DARK_GRAY).bg(BACKGROUND),
        );
    }
}
