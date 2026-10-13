use std::ops::Range;

use ratatui::{buffer::Buffer, layout::Rect, style::Modifier};

use crate::colors::{AMBER, DARK_GRAY, GRAY, LIGHT_RED, RED, WHITE};
use crate::dungeon::beat::View;
use crate::dungeon::expedition::Expedition;
use crate::dungeon::floor::FloorSpec;
use crate::dungeon::items::PACK_STACKS;
use crate::dungeon::kit::Kit;
use crate::dungeon::map::{ENTRY_GLYPH, ENTRY_LANE, LANES, RoomKind};
use crate::dungeon::room::hidden;
use crate::dungeon::screen::{Chart, Popup};
use crate::dungeon::traits::Trait;
use crate::ui::circuit_overlay::{EAST, NORTH, SOUTH, WEST, wire_char};
use crate::ui::hint_bar::HintBar;
use crate::ui::hints::{
    HINT_ENTER_GO, HINT_ESC_BACK, HINT_ESC_LEAVE, HINT_I_ITEMS, HINT_P_PARTY, HINT_ROUTE,
};
use crate::ui::layout::{Screen, Scroll, Scrollbar};
use crate::ui::modal::{Modal, ModalSpec};
use crate::ui::put;
use crate::ui::table::{ellipsize, visual_width};

use super::battle::{
    BACKGROUND, BORDER, BORDERS, FRAME, PAD, SEPARATOR, SPLIT_DOWN, SPLIT_UP, Span, UPRIGHT,
    frame_width, inner, inverse, put_char, put_spans, rule, style, titled, vitals,
};
use super::canvas::{Canvas, Cuts, seal};
use super::room::DESIGN_H;

pub const PITCH: usize = 11;
pub const MIN_PITCH: usize = 3;
pub const ROW_PITCH: usize = 3;
pub const LABEL_GAP: usize = 3;
pub const SIDE_W: u16 = 34;
pub const SIDE_MIN_W: u16 = 26;
pub const NAME_W: usize = 12;
pub const LEGEND_W: usize = 16;
pub const FROM_HOME: &str = "from your tanks";
pub const ELEVATOR: &str = "Elevator";
const DIVIDER: u16 = 1;
const RULE_ROWS: u16 = 1;

struct Wires {
    width: usize,
    cells: Vec<u8>,
}

impl Wires {
    fn new(width: usize, height: usize) -> Wires {
        Wires {
            width,
            cells: vec![0; width * height],
        }
    }

    fn add(&mut self, x: usize, y: usize, bits: u8) {
        if let Some(cell) = self.cells.get_mut(y * self.width + x) {
            *cell |= bits;
        }
    }

    fn mask(&self, x: usize, y: usize) -> u8 {
        self.cells.get(y * self.width + x).copied().unwrap_or(0)
    }

    fn edge(&mut self, (x1, y1): (usize, usize), x2: usize) {
        let junction = y1 + 1;
        let (low, high) = (x1.min(x2), x1.max(x2));
        for x in low..=high {
            let mut bits = 0;
            if x > low {
                bits |= WEST;
            }
            if x < high {
                bits |= EAST;
            }
            self.add(x, junction, bits);
        }
        self.add(x1, junction, NORTH);
        self.add(x2, junction, SOUTH);
        self.add(x2, junction + 1, NORTH | SOUTH);
    }
}

pub struct Layout {
    pub pitch: usize,
    pub lane_x: Vec<usize>,
}

impl Layout {
    pub fn of(room: usize) -> Layout {
        let pitch = (room.saturating_sub(1) / (usize::from(LANES) - 1)).clamp(MIN_PITCH, PITCH);
        Layout {
            pitch,
            lane_x: (0..usize::from(LANES)).map(|lane| lane * pitch).collect(),
        }
    }

    fn x(&self, lane: u8) -> usize {
        self.lane_x[usize::from(lane)]
    }

    fn row_y(row: Option<usize>) -> usize {
        match row {
            None => 0,
            Some(row) => (row + 1) * ROW_PITCH,
        }
    }
}

pub struct Chartwork {
    pub canvas: Canvas,
    pub focus: Range<usize>,
}

pub fn chart(expedition: &Expedition, chart: &Chart, room: usize) -> Chartwork {
    let map = expedition.map();
    let layout = Layout::of(room);
    let path = &expedition.route.path;
    let ahead = expedition.ahead();
    let pick = ahead
        .get(chart.selected.min(ahead.len().saturating_sub(1)))
        .copied();
    let boss_y = Layout::row_y(Some(map.boss_row()));
    let height = boss_y + ROW_PITCH;
    let width = layout.x(LANES - 1) + 1;
    let mut all = Wires::new(width, height);
    let mut past = Wires::new(width, height);
    let mut lit = Wires::new(width, height);
    let here = path
        .len()
        .checked_sub(1)
        .map(|row| (row, usize::from(path[row])));
    let visited =
        |row: usize, index: usize| path.get(row).is_some_and(|&at| usize::from(at) == index);
    let from_entry: Vec<(Option<usize>, u8)> = vec![(None, ENTRY_LANE)];
    let parents_of = |row: usize| -> Vec<(Option<usize>, u8, usize)> {
        match row {
            0 => from_entry.iter().map(|&(r, lane)| (r, lane, 0)).collect(),
            row => map.rows[row - 1]
                .iter()
                .enumerate()
                .map(|(index, room)| (Some(row - 1), room.lane, index))
                .collect(),
        }
    };
    for row in 0..map.rows.len() {
        for (child_index, child) in map.rows[row].iter().enumerate() {
            for (parent_row, lane, parent_index) in parents_of(row) {
                let from = parent_row.map(|r| (r, lane));
                if !map.linked(from, child.lane) {
                    continue;
                }
                let start = (layout.x(lane), Layout::row_y(parent_row));
                let end = layout.x(child.lane);
                all.edge(start, end);
                let parent_visited = match parent_row {
                    None => true,
                    Some(r) => visited(r, parent_index),
                };
                if parent_visited && visited(row, child_index) {
                    past.edge(start, end);
                }
                let parent_here = match (parent_row, here) {
                    (None, None) => true,
                    (Some(r), Some((hr, hi))) => r == hr && parent_index == hi,
                    _ => false,
                };
                if parent_here && path.len() == row && ahead.contains(&(child_index as u8)) {
                    lit.edge(start, end);
                }
            }
        }
    }
    let mut canvas = Canvas::default();
    for y in 0..height {
        for x in 0..width {
            let Some(glyph) = wire_char(all.mask(x, y)) else {
                continue;
            };
            let ink = match (lit.mask(x, y) != 0, past.mask(x, y) != 0) {
                (true, _) => style(WHITE),
                (false, true) => style(GRAY),
                (false, false) => style(DARK_GRAY),
            };
            canvas.set(x, y, glyph, ink);
        }
    }
    let entry_x = layout.x(ENTRY_LANE);
    canvas.set(entry_x, 0, ENTRY_GLYPH, style(GRAY));
    let from = match expedition.floor {
        1 => FROM_HOME.to_string(),
        n => format!("from Floor {}", n - 1),
    };
    canvas.text(entry_x + LABEL_GAP, 0, &from, style(DARK_GRAY));
    let kits = expedition.kits();
    let mapped = kits
        .iter()
        .any(|kit| kit.traits.contains(&Trait::Cartographer));
    for (row, rooms) in map.rows.iter().enumerate() {
        let y = Layout::row_y(Some(row));
        for (index, room) in rooms.iter().enumerate() {
            let x = layout.x(room.lane);
            let next = path.len() == row && ahead.contains(&(index as u8));
            let ink = if Some((row, index)) == here {
                inverse(WHITE)
            } else if next && pick == Some(index as u8) {
                inverse(AMBER)
            } else if next {
                style(WHITE).add_modifier(Modifier::BOLD)
            } else if visited(row, index) {
                style(GRAY)
            } else if room.kind == RoomKind::Boss {
                style(RED)
            } else if mapped && hidden(*room, &kits) {
                style(LIGHT_RED)
            } else {
                style(DARK_GRAY)
            };
            canvas.set(x, y, room.kind.glyph(), ink);
            if room.kind == RoomKind::Boss {
                let name = expedition
                    .visit
                    .as_ref()
                    .filter(|visit| visit.kind == RoomKind::Boss)
                    .and_then(|visit| visit.foes.first().map(|foe| foe.enemy.clone()))
                    .unwrap_or_else(|| boss_name(expedition));
                canvas.text(x + LABEL_GAP, y, &name, style(RED));
            }
        }
    }
    let boss_x = layout.x(ENTRY_LANE);
    canvas.set(boss_x, boss_y + 1, '│', style(DARK_GRAY));
    canvas.set(boss_x, boss_y + 2, RoomKind::Elevator.glyph(), style(WHITE));
    canvas.text(boss_x + LABEL_GAP, boss_y + 2, ELEVATOR, style(WHITE));
    let row = path.len();
    let top = Layout::row_y(row.checked_sub(1));
    let bottom = (Layout::row_y(Some(row.min(map.boss_row()))) + 1).min(height);
    Chartwork {
        canvas,
        focus: top..bottom,
    }
}

fn boss_name(expedition: &Expedition) -> String {
    FloorSpec::of(expedition.floor)
        .boss(expedition.dive)
        .name
        .to_string()
}

pub fn side_lines(expedition: &Expedition, width: usize) -> Vec<Vec<Span>> {
    let mut lines: Vec<Vec<Span>> = vec![Vec::new()];
    let spec = FloorSpec::of(expedition.floor);
    let head = format!(
        "Floor {}{SEPARATOR}{}{SEPARATOR}{}",
        expedition.floor,
        spec.tank.display_name(),
        expedition.difficulty.name()
    );
    lines.push(vec![(ellipsize(&head, width), style(DARK_GRAY))]);
    lines.push(Vec::new());
    let legend: Vec<RoomKind> = RoomKind::ALL.to_vec();
    let two = width >= LEGEND_W * 2;
    for pair in legend.chunks(if two { 2 } else { 1 }) {
        let mut spans = Vec::new();
        for (column, kind) in pair.iter().enumerate() {
            if column > 0 {
                let used: usize = spans.iter().map(|(t, _): &Span| visual_width(t)).sum();
                spans.push((" ".repeat(LEGEND_W.saturating_sub(used)), style(WHITE)));
            }
            spans.push((format!("{} ", kind.glyph()), style(WHITE)));
            spans.push((kind.legend().to_string(), style(GRAY)));
        }
        lines.push(spans);
    }
    lines.push(Vec::new());
    lines.push(vec![("Party".to_string(), style(DARK_GRAY))]);
    for (member, fish) in expedition.party.iter().enumerate() {
        let kit = Kit::of(fish);
        let wound = expedition.wound(member);
        let view = View {
            hp: expedition.hp(member, &kit),
            max_hp: kit.vigor,
            mp: kit.start_mp.min(kit.max_mp),
            max_mp: kit.max_mp,
            rolling: 0,
            belly_up: wound.belly_up,
            gone: false,
            shield: 0,
            statuses: Vec::new(),
        };
        let ink = match wound.belly_up {
            true => style(LIGHT_RED),
            false => style(WHITE),
        };
        let name = ellipsize(&fish.name, NAME_W - 1);
        let mut spans = vec![(format!("{name:<NAME_W$}"), ink)];
        spans.extend(vitals(&view));
        lines.push(spans);
    }
    lines.push(Vec::new());
    lines.push(vec![
        ("Haul   ".to_string(), style(DARK_GRAY)),
        (expedition.haul.line(), style(AMBER)),
    ]);
    let stacks = expedition.pack.iter().filter(|p| p.count > 0).count();
    lines.push(vec![
        ("Items  ".to_string(), style(DARK_GRAY)),
        (format!("{stacks} of {PACK_STACKS}"), style(GRAY)),
    ]);
    lines
}

fn hints(chart: &Chart) -> HintBar {
    if chart.popup.is_some() {
        return HintBar::new(HINT_ESC_BACK);
    }
    HintBar::new(HINT_ESC_LEAVE)
        .action(HINT_ROUTE)
        .action(HINT_ENTER_GO)
        .action(HINT_I_ITEMS)
        .action(HINT_P_PARTY)
}

fn draw_lines(buf: &mut Buffer, lines: &[Vec<Span>], area: Rect) {
    for (row, spans) in lines.iter().enumerate() {
        let y = area.y + row as u16;
        if y >= area.bottom() {
            return;
        }
        put_spans(buf, area.x, y, spans, area);
    }
}

fn frame(buf: &mut Buffer, rect: Rect, title: &str) {
    crate::ui::modal::clear(buf, rect, BACKGROUND);
    crate::ui::table::draw_box_border(buf, rect, "", FRAME, BACKGROUND);
    crate::ui::table::draw_box_title(buf, rect, &titled(title), WHITE, BACKGROUND);
}

pub fn draw(
    buf: &mut Buffer,
    expedition: &Expedition,
    chart_state: &Chart,
    scroll: &Scroll,
    screen: Screen,
) {
    let width = frame_width(screen);
    let inner_w = width.saturating_sub(BORDERS);
    let bar = hints(chart_state);
    let hint_h = bar.height(inner_w);
    let wide_map = inner_w.saturating_sub(SIDE_W + DIVIDER + PAD * 2) as usize;
    let wide = Layout::of(wide_map).pitch >= MIN_PITCH
        && inner_w >= SIDE_MIN_W + DIVIDER + MIN_PITCH as u16 * 5;
    let title = expedition.floor_title();
    if wide {
        let side_w = SIDE_W.min(inner_w / 2);
        let map_room = inner_w.saturating_sub(side_w + DIVIDER + PAD * 2) as usize;
        let work = chart(expedition, chart_state, map_room);
        let side = side_lines(expedition, side_w.saturating_sub(PAD * 2) as usize);
        let content = (work.canvas.height() + PAD).max(side.len() as u16);
        let height = (content + RULE_ROWS + hint_h + BORDERS)
            .max(DESIGN_H.min(screen.whole.height))
            .min(screen.whole.height);
        let rect = screen.place(width, height);
        frame(buf, rect, &title);
        let inside = inner(rect);
        let hint_h = hint_h.min(inside.height.saturating_sub(1));
        let body_h = inside.height.saturating_sub(hint_h + RULE_ROWS);
        let left_w = inside.width.saturating_sub(side_w + DIVIDER);
        let divider = inside.x + left_w;
        let rule_y = inside.y + body_h;
        put_char(buf, divider, rect.y, SPLIT_DOWN);
        for y in inside.y..rule_y {
            put_char(buf, divider, y, UPRIGHT);
        }
        rule(buf, rect, rule_y);
        put_char(buf, divider, rule_y, SPLIT_UP);
        let map_area = Rect::new(inside.x, inside.y + PAD, left_w, body_h.saturating_sub(PAD));
        let total = work.canvas.height() as usize;
        let shown = scroll.reveal(work.focus.clone(), map_area.height as usize, total);
        let x = map_area.x as i32 + (map_area.width.saturating_sub(work.canvas.width()) / 2) as i32;
        work.canvas
            .draw(buf, x, map_area.y as i32 - shown.start as i32, map_area);
        seal(
            buf,
            map_area,
            Cuts {
                top: shown.start > 0,
                bottom: shown.end < total,
                left: work.canvas.width() > map_area.width,
                right: work.canvas.width() > map_area.width,
            },
        );
        Scrollbar {
            x: divider,
            top: map_area.y,
            height: map_area.height,
        }
        .draw(buf, shown, total, WHITE);
        draw_lines(
            buf,
            &side,
            Rect::new(
                divider + 1 + PAD,
                inside.y,
                side_w.saturating_sub(PAD * 2),
                body_h,
            ),
        );
        bar.draw(
            buf,
            Rect::new(inside.x, rule_y + RULE_ROWS, inside.width, hint_h),
            BACKGROUND,
        );
    } else {
        draw_column(buf, expedition, chart_state, scroll, screen, &bar);
    }
    if let Some(popup) = chart_state.popup {
        draw_popup(buf, expedition, popup, screen);
    }
}

fn draw_column(
    buf: &mut Buffer,
    expedition: &Expedition,
    chart_state: &Chart,
    scroll: &Scroll,
    screen: Screen,
    bar: &HintBar,
) {
    let width = frame_width(screen);
    let inner_w = width.saturating_sub(BORDERS);
    let work = chart(
        expedition,
        chart_state,
        inner_w.saturating_sub(PAD * 2) as usize,
    );
    let indent = (inner_w.saturating_sub(work.canvas.width()) / 2) as usize;
    let mut lines: Vec<Vec<Span>> = (0..work.canvas.height() as usize)
        .map(|row| {
            let mut spans = vec![(" ".repeat(indent), style(WHITE))];
            spans.extend(work.canvas.row_spans(row));
            spans
        })
        .collect();
    let focus = work.focus.clone();
    for mut line in side_lines(expedition, inner_w.saturating_sub(PAD * 2) as usize) {
        line.insert(0, (" ".repeat(PAD as usize), style(WHITE)));
        lines.push(line);
    }
    let hint_h = bar.height(inner_w);
    let height = (lines.len() as u16 + RULE_ROWS + hint_h + BORDERS).min(screen.whole.height);
    let rect = screen.place(width, height);
    frame(buf, rect, &expedition.floor_title());
    let inside = inner(rect);
    let hint_h = hint_h.min(inside.height.saturating_sub(1));
    let body_h = inside.height.saturating_sub(hint_h + RULE_ROWS);
    let rule_y = inside.y + body_h;
    rule(buf, rect, rule_y);
    let total = lines.len();
    let shown = scroll.reveal(focus, body_h as usize, total);
    let window = Rect::new(inside.x, inside.y, inside.width, body_h);
    draw_lines(buf, &lines[shown.clone()], window);
    seal(
        buf,
        window,
        Cuts {
            top: shown.start > 0,
            bottom: shown.end < total,
            left: work.canvas.width() > inside.width,
            right: work.canvas.width() > inside.width,
        },
    );
    Scrollbar {
        x: rect.right().saturating_sub(BORDER),
        top: inside.y,
        height: body_h,
    }
    .draw(buf, shown, total, WHITE);
    bar.draw(
        buf,
        Rect::new(inside.x, rule_y + RULE_ROWS, inside.width, hint_h),
        BACKGROUND,
    );
}

fn popup_lines(expedition: &Expedition, popup: Popup) -> (String, Vec<Span>) {
    match popup {
        Popup::Items => {
            let lines: Vec<Span> = match expedition.pack.iter().any(|p| p.count > 0) {
                false => vec![("Nothing is packed.".to_string(), style(GRAY))],
                true => expedition
                    .pack
                    .iter()
                    .filter(|p| p.count > 0)
                    .map(|p| (format!("{} ×{}", p.item.name(), p.count), style(WHITE)))
                    .collect(),
            };
            ("Items".to_string(), lines)
        }
        Popup::Party => {
            let lines = expedition
                .party
                .iter()
                .map(|fish| {
                    let kit = Kit::of(fish);
                    (
                        format!(
                            "{}{SEPARATOR}{}{SEPARATOR}{}{SEPARATOR}Bite {}{SEPARATOR}Speed {}",
                            fish.name,
                            fish.species.display_name(),
                            kit.grade.name(),
                            kit.bite,
                            kit.speed
                        ),
                        style(WHITE),
                    )
                })
                .collect();
            ("Party".to_string(), lines)
        }
    }
}

fn draw_popup(buf: &mut Buffer, expedition: &Expedition, popup: Popup, screen: Screen) {
    let (title, lines) = popup_lines(expedition, popup);
    let title = titled(&title);
    let bar = HintBar::new(HINT_ESC_BACK);
    let content_w = lines
        .iter()
        .map(|(text, _)| visual_width(text) as u16 + PAD * 2)
        .max()
        .unwrap_or(0);
    let modal = Modal::open(
        buf,
        screen,
        &ModalSpec {
            title: &title,
            border: WHITE,
            background: BACKGROUND,
            content_w,
            content_h: lines.len() as u16,
            hints: &bar,
        },
    );
    for (row, (text, ink)) in lines.iter().enumerate() {
        let y = modal.body.y + row as u16;
        if y >= modal.body.bottom() {
            return;
        }
        let shown = ellipsize(text, modal.body.width.saturating_sub(PAD * 2) as usize);
        put(
            buf,
            (modal.body.x + PAD) as i32,
            y as i32,
            &shown,
            *ink,
            modal.body,
        );
    }
}
