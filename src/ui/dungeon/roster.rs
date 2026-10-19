use std::ops::Range;

use ratatui::{buffer::Buffer, layout::Rect};

use crate::colors::{DARK_GRAY, GRAY, WHITE};
use crate::dungeon::beat::{LogLine, Tone};
use crate::dungeon::room::{Panel, Scene};
use crate::fishes::fish::Fish;
use crate::ui::hint_bar::HintBar;
use crate::ui::hints::{HINT_CHOOSE, HINT_ESC_BACK, HINT_ESC_LEAVE, HINT_PAGE};
use crate::ui::layout::{Screen, Scroll, Scrollbar};
use crate::ui::table::{ellipsize, visual_width};

use super::battle::{
    BACKGROUND, BORDER, BORDERS, PAD, SPLIT_DOWN, SPLIT_UP, Span, TITLE_INSET, UPRIGHT, inner,
    put_char, put_spans, rule, style, tone_colour,
};
use super::canvas::Canvas;
use super::room::{
    ART_GAP, ART_W, DIVIDER, NOTE_GAP, NOTE_PAD, RULE_ROWS, TEXT_MIN_W, TEXT_W, boxed, cards,
    draw_lines, frame, place, portrait, row_lines, tab_hint, text_lines, titled_box, verb_hint,
};

pub const LIST_MIN_W: u16 = 20;
pub const LIST_MAX_W: u16 = 36;
pub const LIST_SHARE: u16 = 3;
pub const SHEET_MIN_W: u16 = TEXT_MIN_W;
pub const LIST_FLOOR: u16 = 3;
pub const SLOT_GAP: usize = 1;
pub const SLOT_PAD: usize = 1;
pub const SLOT_MIN_INNER: usize = 10;
pub const SHEET_FLOOR: u16 = 3;
pub const SHEET_TOP: usize = 1;
pub const SHEET_TOP_FROM: usize = 6;
pub const PARTY: &str = "Party";
pub const NOBODY_YET: &str = "nobody yet";
const FRAME_W: u16 = BORDERS + ART_W + DIVIDER + TEXT_W;
const CURSOR_W: usize = 2;

type Line = Vec<Span>;

pub struct Roster<'a> {
    pub panel: &'a Panel,
    pub selected: usize,
    pub slots: Option<Vec<Option<&'a Fish>>>,
    pub list: &'a Scroll,
    pub page: &'a Scroll,
}

fn hints(panel: &Panel, list: Option<(usize, usize)>, page: Option<(usize, usize)>) -> HintBar {
    let close = match panel.back {
        true => HINT_ESC_BACK,
        false => HINT_ESC_LEAVE,
    };
    let mut bar = HintBar::new(close);
    if panel.rows.len() > 1 {
        bar = bar.counted(HINT_CHOOSE, list);
    }
    if page.is_some() {
        bar = bar.counted(HINT_PAGE, page);
    }
    if let Some(verb) = verb_hint(panel.verb) {
        bar = bar.action(verb);
    }
    match panel.tab {
        Some(tab) => bar.action(tab_hint(tab)),
        None => bar,
    }
}

fn tallest_hints(panel: &Panel, width: u16) -> u16 {
    let most = Some((panel.rows.len(), panel.rows.len()));
    hints(panel, most, most).height(width)
}

fn slot_box(fish: Option<&Fish>, (w, inner_h): (usize, usize), drawn: bool) -> Canvas {
    let h = inner_h + 2;
    let mut canvas = Canvas::default();
    let Some(fish) = fish else {
        boxed(&mut canvas, 0, 0, w, h, style(DARK_GRAY));
        return canvas;
    };
    titled_box(&mut canvas, (w, h), &fish.name, style(WHITE));
    let art = Canvas::fish(fish).trimmed();
    let inside = w.saturating_sub(2);
    let room = inside.saturating_sub(SLOT_PAD * 2);
    if drawn && art.width() as usize <= room && art.height() as usize <= inner_h {
        let x = 1 + inside.saturating_sub(art.width() as usize) / 2;
        let y = 1 + (inner_h - art.height() as usize) / 2;
        canvas.blit(x, y, &art);
        return canvas;
    }
    if inner_h > 0 {
        let species = ellipsize(fish.species.display_name(), room.max(1));
        let x = 1 + inside.saturating_sub(visual_width(&species)) / 2;
        canvas.text(x, 1 + (inner_h - 1) / 2, &species, style(GRAY));
    }
    canvas
}

fn natural_slot(fish: Option<&Fish>) -> (usize, usize) {
    let Some(fish) = fish else {
        return (SLOT_MIN_INNER + 2, 1);
    };
    let art = Canvas::fish(fish).trimmed();
    let title = visual_width(&fish.name) + TITLE_INSET as usize * 2 + 2;
    let w = (art.width() as usize + SLOT_PAD * 2 + 2)
        .max(title)
        .max(SLOT_MIN_INNER + 2);
    (w, art.height().max(1) as usize)
}

fn flex_slots(slots: &[Option<&Fish>], width: usize) -> Option<Canvas> {
    let natural: Vec<(usize, usize)> = slots.iter().map(|fish| natural_slot(*fish)).collect();
    if natural.iter().any(|&(w, _)| w > width) {
        return None;
    }
    let mut lines: Vec<Vec<usize>> = vec![Vec::new()];
    let mut used = 0;
    for (index, &(w, _)) in natural.iter().enumerate() {
        let line = lines.last_mut().expect("a line");
        let gap = if line.is_empty() { 0 } else { SLOT_GAP };
        if !line.is_empty() && used + gap + w > width {
            lines.push(vec![index]);
            used = w;
            continue;
        }
        line.push(index);
        used += gap + w;
    }
    let mut canvas = Canvas::default();
    let mut y = 0;
    for line in &lines {
        let natural_w: usize = line.iter().map(|&i| natural[i].0).sum::<usize>()
            + line.len().saturating_sub(1) * SLOT_GAP;
        let spare = width.saturating_sub(natural_w);
        let inner_h = line.iter().map(|&i| natural[i].1).max().unwrap_or(1);
        let mut x = 0;
        for (rank, &index) in line.iter().enumerate() {
            let grow = spare / line.len() + usize::from(rank < spare % line.len());
            let w = natural[index].0 + grow;
            canvas.blit(x, y, &slot_box(slots[index], (w, inner_h), true));
            x += w + SLOT_GAP;
        }
        y += inner_h + 2;
    }
    Some(canvas)
}

fn slot_grid(slots: &[Option<&Fish>], width: usize, columns: usize, drawn: bool) -> Option<Canvas> {
    let columns = columns.max(1);
    let slot_w = width.saturating_sub((columns - 1) * SLOT_GAP) / columns;
    if slot_w < SLOT_MIN_INNER + 2 {
        return None;
    }
    let room = slot_w - 2 - SLOT_PAD * 2;
    let arts: Vec<Canvas> = slots
        .iter()
        .flatten()
        .map(|fish| Canvas::fish(fish).trimmed())
        .collect();
    let inner_h = match drawn {
        true => arts
            .iter()
            .filter(|art| art.width() as usize <= room)
            .map(|art| art.height() as usize)
            .max()
            .unwrap_or(1)
            .max(1),
        false => 0,
    };
    let h = inner_h + 2;
    let mut canvas = Canvas::default();
    for (index, fish) in slots.iter().enumerate() {
        let (line, column) = (index / columns, index % columns);
        let slot = slot_box(*fish, (slot_w, inner_h), drawn);
        canvas.blit(column * (slot_w + SLOT_GAP), line * h, &slot);
    }
    Some(canvas)
}

fn party_line(slots: &[Option<&Fish>], width: usize) -> Canvas {
    let names: Vec<&str> = slots
        .iter()
        .flatten()
        .map(|fish| fish.name.as_str())
        .collect();
    let (who, tone) = match names.is_empty() {
        true => (NOBODY_YET.to_string(), Tone::Quiet),
        false => (names.join(", "), Tone::Focus),
    };
    let said = format!("{PARTY} {} of {}: {who}", names.len(), slots.len());
    let text = ellipsize(&said, width.max(1));
    let mut canvas = Canvas::default();
    canvas.text(0, 0, &text, style(tone_colour(tone)));
    canvas
}

fn band(slots: &[Option<&Fish>], width: usize, room: u16) -> Canvas {
    let shapes = [slots.len(), slots.len().div_ceil(2)];
    let partial = shapes
        .iter()
        .filter_map(|&columns| slot_grid(slots, width, columns, true));
    let titles = shapes
        .iter()
        .filter_map(|&columns| slot_grid(slots, width, columns, false));
    flex_slots(slots, width)
        .into_iter()
        .chain(partial.chain(titles))
        .find(|canvas| canvas.height() <= room)
        .unwrap_or_else(|| party_line(slots, width))
}

struct Sheet {
    art: Option<Canvas>,
    lines: Vec<Vec<Span>>,
    pages: Vec<Range<usize>>,
}

impl Sheet {
    fn natural_height(&self) -> usize {
        height_of(self.art.as_ref(), self.lines.len())
    }
}

fn height_of(art: Option<&Canvas>, lines: usize) -> usize {
    let Some(art) = art else {
        return lines;
    };
    let gap = match lines {
        0 => 0,
        _ => ART_GAP,
    };
    art.height() as usize + gap + lines
}

fn first_room(art: usize, height: usize) -> usize {
    match art {
        0 => height,
        _ => height.saturating_sub(art + ART_GAP),
    }
}

fn paginate(art: usize, groups: &[usize], height: usize) -> Vec<Range<usize>> {
    let page = height.max(1);
    let mut pages = Vec::new();
    let mut room = first_room(art, height);
    if room == 0 {
        pages.push(0..0);
        room = page;
    }
    let (mut start, mut at) = (0, 0);
    for &size in groups {
        let opens_the_sheet = pages.is_empty() && at == start;
        if opens_the_sheet && size > room && size <= page {
            pages.push(0..0);
            room = page;
        }
        if at > start && at - start + size > room {
            pages.push(start..at);
            start = at;
            room = page;
        }
        at += size;
        while at - start > room {
            pages.push(start..start + room);
            start += room;
            room = page;
        }
    }
    if at > start || pages.is_empty() {
        pages.push(start..at);
    }
    pages
}

#[derive(Default)]
struct Text {
    lines: Vec<Line>,
    groups: Vec<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fit {
    Whole,
    Paged,
}

struct Layout {
    art: Option<Canvas>,
    text: Text,
    fit: Fit,
}

fn layouts(panel: &Panel, selected: usize, width: usize) -> Vec<Layout> {
    let about = panel
        .rows
        .get(selected)
        .map(|row| row.about.clone())
        .unwrap_or_default();
    let Scene::Card(card) = &panel.art else {
        return vec![Layout {
            art: None,
            text: text(&about, width),
            fit: Fit::Paged,
        }];
    };
    let mut told = card.caption.clone();
    told.push(LogLine::new(String::new(), Tone::Plain));
    told.extend(about.iter().cloned());
    let (alone, _) = portrait(&card.portrait);
    vec![
        Layout {
            art: cards(card).into_iter().next(),
            text: text(&about, width),
            fit: Fit::Whole,
        },
        Layout {
            art: Some(alone.clone()),
            text: text(&told, width),
            fit: Fit::Whole,
        },
        Layout {
            art: Some(alone),
            text: text(&told, width),
            fit: Fit::Paged,
        },
        Layout {
            art: None,
            text: text(&told, width),
            fit: Fit::Paged,
        },
    ]
}

fn text(said: &[LogLine], width: usize) -> Text {
    let mut text = Text::default();
    for line in said {
        let before = text.lines.len();
        text_lines(&mut text.lines, std::slice::from_ref(line), width);
        text.groups.push(text.lines.len() - before);
    }
    text
}

fn sheet(panel: &Panel, selected: usize, width: usize, height: usize) -> Sheet {
    let chosen = layouts(panel, selected, width).into_iter().find(|layout| {
        let art = layout.art.as_ref();
        let (art_w, art_h) = art.map_or((0, 0), |art| (art.width(), art.height()));
        let fits = art_w as usize <= width && art_h as usize <= height;
        match layout.fit {
            Fit::Whole => fits && height_of(art, layout.text.lines.len()) <= height,
            Fit::Paged => fits,
        }
    });
    let (art, text) = chosen.map_or((None, Text::default()), |layout| (layout.art, layout.text));
    let art_h = art.as_ref().map_or(0, |art| art.height() as usize);
    let pages = paginate(art_h, &text.groups, height);
    Sheet {
        art,
        lines: text.lines,
        pages,
    }
}

fn row_width(panel: &Panel) -> u16 {
    panel
        .rows
        .iter()
        .map(|row| {
            let note = match row.note.is_empty() {
                true => 0,
                false => NOTE_GAP + visual_width(&row.note),
            };
            PAD as usize + CURSOR_W + visual_width(&row.label) + note + NOTE_PAD
        })
        .max()
        .unwrap_or(0) as u16
}

fn list_lines(panel: &Panel, selected: usize, width: usize) -> Vec<Vec<Line>> {
    panel
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| row_lines(row, index == selected, width))
        .collect()
}

struct List {
    rows: Vec<Vec<Line>>,
}

impl List {
    fn heights(&self) -> Vec<usize> {
        self.rows.iter().map(Vec::len).collect()
    }

    fn height(&self) -> usize {
        self.heights().iter().sum()
    }
}

fn draw_list(buf: &mut Buffer, list: &List, roster: &Roster, area: Rect, bar_x: u16) -> bool {
    let heights = list.heights();
    let shown = roster
        .list
        .follow(&heights, roster.selected, area.height as usize);
    let lines: Vec<Line> = list.rows[shown.clone()].iter().flatten().cloned().collect();
    draw_lines(buf, &lines, area);
    let before: usize = heights[..shown.start].iter().sum();
    let within: usize = heights[shown.clone()].iter().sum();
    let total = list.height();
    Scrollbar {
        x: bar_x,
        top: area.y,
        height: area.height,
    }
    .draw(buf, before..before + within, total, WHITE);
    total > area.height as usize
}

fn draw_sheet(buf: &mut Buffer, sheet: &Sheet, page: &Scroll, area: Rect, bar_x: u16) -> usize {
    let last = sheet.pages.len().saturating_sub(1);
    let at = page.settle(last);
    let mut y = area.y;
    if at == 0
        && let Some(art) = &sheet.art
    {
        let x = area.x + area.width.saturating_sub(art.width()) / 2;
        art.draw(buf, x as i32, y as i32, area);
        y += art.height() + ART_GAP as u16;
    }
    let range = sheet.pages.get(at).cloned().unwrap_or(0..0);
    for (row, spans) in sheet.lines[range].iter().enumerate() {
        let line_y = y + row as u16;
        if line_y >= area.bottom() {
            break;
        }
        put_spans(buf, area.x, line_y, spans, area);
    }
    Scrollbar {
        x: bar_x,
        top: area.y,
        height: area.height,
    }
    .draw(buf, at..at + 1, sheet.pages.len(), WHITE);
    at
}

fn sheet_room(height: usize) -> (usize, usize) {
    match height >= SHEET_TOP_FROM {
        true => (SHEET_TOP, height - SHEET_TOP),
        false => (0, height),
    }
}

struct Rules {
    band: bool,
    split: bool,
}

impl Rules {
    fn of(band: Option<u16>, side: bool, room: u16) -> Rules {
        let mut rules = Rules {
            band: band.is_some(),
            split: !side,
        };
        let floor = match side {
            true => 1,
            false => 2,
        };
        let band = band.unwrap_or(0);
        let fits = |rules: &Rules| band + rules.under_band() + rules.between() + floor <= room;
        if !side && !fits(&rules) {
            rules.band = false;
        }
        if !fits(&rules) {
            rules.split = false;
        }
        rules
    }

    fn under_band(&self) -> u16 {
        u16::from(self.band) * RULE_ROWS
    }

    fn between(&self) -> u16 {
        u16::from(self.split) * RULE_ROWS
    }
}

fn stacked_list_height(body: u16, list: u16, sheet: u16, between: u16) -> u16 {
    let shared = body.saturating_sub(between);
    if list + sheet <= shared {
        return list;
    }
    let half = shared / 2;
    let most = shared.saturating_sub(1).max(1);
    let list_floor = LIST_FLOOR.min(list).min(most);
    let list_h = match sheet <= half {
        true => shared.saturating_sub(sheet).max(list_floor),
        false => list.min(half.max(list_floor)),
    };
    list_h.min(most)
}

pub fn draw(buf: &mut Buffer, roster: &Roster, screen: Screen) {
    let panel = roster.panel;
    let width = screen.whole.width.min(FRAME_W);
    let inner_w = width.saturating_sub(BORDERS);
    let hint_h = tallest_hints(panel, inner_w);
    let room_h = screen
        .whole
        .height
        .saturating_sub(BORDERS + RULE_ROWS + hint_h);
    let side = inner_w >= LIST_MIN_W + DIVIDER + SHEET_MIN_W;
    let below = match side {
        true => LIST_FLOOR.max(SHEET_FLOOR),
        false => LIST_FLOOR + RULE_ROWS + SHEET_FLOOR,
    };
    let band_room = room_h.saturating_sub(RULE_ROWS + below);
    let band = roster
        .slots
        .as_ref()
        .map(|slots| band(slots, inner_w.saturating_sub(PAD * 2) as usize, band_room));
    let list_w = match side {
        true => row_width(panel)
            .max(inner_w / LIST_SHARE)
            .clamp(LIST_MIN_W, LIST_MAX_W)
            .min(inner_w.saturating_sub(DIVIDER + SHEET_MIN_W)),
        false => inner_w,
    };
    let sheet_w = match side {
        true => inner_w.saturating_sub(list_w + DIVIDER),
        false => inner_w,
    };
    let list = List {
        rows: list_lines(panel, roster.selected, list_w as usize),
    };
    let natural = sheet(panel, roster.selected, sheet_w as usize, usize::MAX / 2);
    let sheet_need = (natural.natural_height() + SHEET_TOP) as u16;
    let list_need = list.height() as u16;
    let rules = Rules::of(band.as_ref().map(Canvas::height), side, room_h);
    let band_h = band.as_ref().map_or(0, Canvas::height) + rules.under_band();
    let content = match side {
        true => list_need.max(sheet_need),
        false => list_need + rules.between() + sheet_need,
    };
    let rect = place(
        screen,
        width,
        band_h + content + RULE_ROWS + hint_h + BORDERS,
    );
    let inside = inner(rect);
    let hint_h = hint_h.min(inside.height.saturating_sub(1));
    let body_top = inside.y + band_h;
    let hint_rule = inside.y + inside.height.saturating_sub(hint_h + RULE_ROWS);
    let body_h = hint_rule.saturating_sub(body_top);
    frame(buf, rect, &panel.title);
    if let Some(band) = &band {
        let area = Rect::new(inside.x, inside.y, inside.width, band.height());
        band.draw_centred(buf, area);
        if rules.band {
            rule(buf, rect, inside.y + band.height());
        }
    }
    rule(buf, rect, hint_rule);
    let right_bar = rect.right().saturating_sub(BORDER);
    let (list_area, sheet_area, list_bar) = match side {
        true => {
            let divider = inside.x + list_w;
            let top = match band.is_some() {
                true => body_top.saturating_sub(rules.under_band()),
                false => rect.y,
            };
            put_char(buf, divider, top, SPLIT_DOWN);
            for y in body_top..hint_rule {
                put_char(buf, divider, y, UPRIGHT);
            }
            put_char(buf, divider, hint_rule, SPLIT_UP);
            (
                Rect::new(inside.x, body_top, list_w, body_h),
                Rect::new(divider + 1, body_top, sheet_w, body_h),
                divider,
            )
        }
        false => {
            let list_h = stacked_list_height(body_h, list_need, sheet_need, rules.between());
            let sheet_y = body_top + list_h + rules.between();
            if rules.split {
                rule(buf, rect, body_top + list_h);
            }
            (
                Rect::new(inside.x, body_top, inner_w, list_h),
                Rect::new(
                    inside.x,
                    sheet_y,
                    inner_w,
                    hint_rule.saturating_sub(sheet_y),
                ),
                right_bar,
            )
        }
    };
    let overflows = draw_list(buf, &list, roster, list_area, list_bar);
    let (top, room) = sheet_room(sheet_area.height as usize);
    let paged = sheet(panel, roster.selected, sheet_w as usize, room);
    let pages_area = Rect::new(
        sheet_area.x,
        sheet_area.y + top as u16,
        sheet_area.width,
        room as u16,
    );
    let at = draw_sheet(buf, &paged, roster.page, pages_area, right_bar);
    let pages = paged.pages.len();
    let bar = hints(
        panel,
        overflows.then_some((roster.selected + 1, panel.rows.len())),
        (pages > 1).then_some((at + 1, pages)),
    );
    bar.draw(
        buf,
        Rect::new(inside.x, hint_rule + RULE_ROWS, inside.width, hint_h),
        BACKGROUND,
    );
}
