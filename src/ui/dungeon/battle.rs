use std::ops::Range;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
};

use crate::colors::{
    AMBER, BLACK, DARK_GRAY, GRAY, LIGHT_BLUE, LIGHT_CYAN, LIGHT_GREEN, LIGHT_RED, RED, WHITE,
    YELLOW,
};
use crate::dungeon::battle::Outcome;
use crate::dungeon::beat::{LogLine, Tone, View};
use crate::dungeon::bestiary::SpriteRow;
use crate::dungeon::expedition::MAX_PARTY;
use crate::dungeon::fighter::{Origin, Side};
use crate::dungeon::info::{Info, Ink};
use crate::dungeon::skill::{EMPTY_BUBBLE, FULL_BUBBLE, cost_label};
use crate::dungeon::state::{Answer, Command, FightState, Menu, Pending};
use crate::economy::grouped;
use crate::entities::cow::{COW_TRANSPARENT, Cow, cow_sprite};
use crate::fishes::fish::{Direction, Fish};
use crate::ui::hint_bar::HintBar;
use crate::ui::hints::{
    HINT_CHOOSE, HINT_ENTER_ANSWER, HINT_ENTER_CONFIRM, HINT_ENTER_GO_ON, HINT_ENTER_SELECT,
    HINT_ENTER_SWIM_HOME, HINT_ESC_BACK, HINT_ESC_LEAVE, HINT_TARGET, HINT_WHICH_FISH,
};
use crate::ui::layout::{Screen, Scrollbar};
use crate::ui::modal::{Modal, ModalSpec, clear};
use crate::ui::table::{draw_box_border, draw_box_title, ellipsize, visual_width, wrap_words};
use crate::ui::{draw_fish_centred, fish_rows, put};

pub(super) const BACKGROUND: Color = Color::Reset;
pub(super) const FRAME: Color = GRAY;
pub(super) const MARGIN_FROM: u16 = 100;
pub(super) const MARGIN: u16 = 1;
pub(super) const BORDER: u16 = 1;
pub(super) const BORDERS: u16 = BORDER * 2;
pub(super) const PAD: u16 = 1;
pub(super) const GAP: u16 = 1;
const PERCENT: u16 = 100;
const LOG_SHARE_PERCENT: u16 = 28;
const LOG_MIN_W: u16 = 18;
const LOG_MAX_W: u16 = 33;
const LEFT_MIN_W: u16 = 36;
const WINDOW_MIN_W: u16 = 19;
const WINDOW_CHROME_W: u16 = 4;
const WINDOW_CHROME_H: u16 = 4;
const LINE_WINDOW_H: u16 = 3;
const ENEMY_MIN_ROWS: u16 = 3;
const ENEMY_CHROME_ROWS: u16 = 2;
const MAIN_MENU_W: u16 = 16;
const SUB_MIN_W: u16 = 24;
const INFO_MIN_W: u16 = 28;
const INFO_MAX_W: u16 = 40;
const INFO_MIN_H: u16 = 7;
const LABEL_W: u16 = 12;
pub(super) const COST_GAP: u16 = 2;
const LINE_GAP: u16 = 3;
const CONTROL_MIN_ROWS: u16 = 3;
pub(super) const TITLE_INSET: u16 = 2;
pub(super) const SEPARATOR: &str = " · ";
const LEAD: &str = "▸";
pub(super) const CURSOR: &str = "> ";
pub(super) const NO_CURSOR: &str = "  ";
const MORE: &str = "▸";
const ROLLING: &str = "↓";
const BELLY_UP: &str = "belly up";
const CONTINUATION: &str = "  ";
pub(super) const RULE: char = '─';
pub(super) const SPLIT_DOWN: char = '┬';
pub(super) const SPLIT_UP: char = '┴';
const TEE_RIGHT: char = '├';
const TEE_LEFT: char = '┤';
pub(super) const UPRIGHT: char = '│';
const HP_GOOD_PERCENT: u32 = 50;
const HP_LOW_PERCENT: u32 = 25;

pub(super) type Span = (String, Style);

pub(super) fn style(fg: Color) -> Style {
    Style::default().fg(fg).bg(BACKGROUND)
}

pub(super) fn inverse(bg: Color) -> Style {
    Style::default().fg(BLACK).bg(bg)
}

#[derive(Clone, Copy)]
pub(super) enum Art<'a> {
    Fish(&'a Fish),
    Cow(&'a Cow),
    Rows(&'a [SpriteRow]),
}

impl Art<'_> {
    fn width(self) -> u16 {
        match self {
            Art::Fish(fish) => fish.display_width as u16,
            Art::Cow(cow) => cow_sprite(cow)
                .iter()
                .map(|row| row.len())
                .max()
                .unwrap_or(0) as u16,
            Art::Rows(rows) => rows.iter().map(|row| row.width() as u16).max().unwrap_or(0),
        }
    }

    fn height(self) -> u16 {
        match self {
            Art::Fish(fish) => fish_rows(fish),
            Art::Cow(cow) => cow_sprite(cow).len() as u16,
            Art::Rows(rows) => rows.len() as u16,
        }
    }
}

struct Scene<'a> {
    state: &'a FightState,
    foes: Vec<usize>,
    party: Vec<usize>,
    actor: Option<usize>,
    targeted: Option<usize>,
}

impl<'a> Scene<'a> {
    fn of(state: &'a FightState) -> Self {
        let shown = state.shown.views.len().min(state.battle.fighters.len());
        let on = |side: Side| -> Vec<usize> {
            (0..shown)
                .filter(|&i| state.battle.fighters[i].side == side && !state.shown.views[i].gone)
                .collect()
        };
        Scene {
            state,
            foes: on(Side::Foes),
            party: on(Side::Party),
            actor: state.actor(),
            targeted: state.targeted(),
        }
    }

    fn view(&self, index: usize) -> &View {
        &self.state.shown.views[index]
    }

    fn name(&self, index: usize) -> &str {
        &self.state.battle.fighters[index].name
    }

    fn art(&self, index: usize) -> Art<'a> {
        let battle = &self.state.battle;
        if let Some(fish) = battle.fighters[index]
            .disguise
            .and_then(|member| battle.party.get(member))
        {
            return Art::Fish(fish);
        }
        match &battle.fighters[index].origin {
            Origin::Fish { member } => match battle.party.get(*member) {
                Some(fish) => Art::Fish(fish),
                None => Art::Rows(&[]),
            },
            Origin::Cow { herd } => match battle.cows.get(*herd) {
                Some(cow) => Art::Cow(cow),
                None => Art::Rows(&[]),
            },
            Origin::Enemy { spec } => Art::Rows(spec.sprite),
        }
    }

    fn compact(&self) -> bool {
        self.party.len() > MAX_PARTY
    }

    fn name_style(&self, index: usize) -> Style {
        if self.targeted == Some(index) {
            return inverse(AMBER);
        }
        if self.state.shown.thinking == Some(index) {
            return inverse(RED);
        }
        if self.actor == Some(index) {
            return inverse(WHITE);
        }
        match self.state.battle.fighters[index].side {
            Side::Foes => style(RED),
            Side::Party if self.view(index).belly_up => style(LIGHT_RED),
            Side::Party => style(WHITE),
        }
    }

    fn label(&self, index: usize) -> String {
        match self.actor == Some(index) {
            true => format!("{LEAD}{}", self.name(index)),
            false => self.name(index).to_string(),
        }
    }
}

pub(super) fn vitals(view: &View) -> Vec<Span> {
    let mut spans = Vec::new();
    match view.belly_up {
        true => spans.push((BELLY_UP.to_string(), style(LIGHT_RED))),
        false => {
            let rolling = if view.rolling > 0 { ROLLING } else { "" };
            let colour = match view.rolling > 0 {
                true => YELLOW,
                false => hp_colour(view.hp, view.max_hp),
            };
            spans.push((
                format!("{}/{}{rolling}", view.hp, view.max_hp),
                style(colour),
            ));
        }
    }
    if view.max_mp > 0 {
        let full = view.mp.min(view.max_mp) as usize;
        let empty = view.max_mp.saturating_sub(view.mp) as usize;
        spans.push((" ".to_string(), style(WHITE)));
        spans.push((FULL_BUBBLE.repeat(full), style(LIGHT_CYAN)));
        spans.push((EMPTY_BUBBLE.repeat(empty), style(DARK_GRAY)));
    }
    spans
}

pub(super) fn hp_colour(hp: u32, max: u32) -> Color {
    let percent = match max {
        0 => 0,
        max => hp * 100 / max,
    };
    match percent {
        p if p > HP_GOOD_PERCENT => LIGHT_GREEN,
        p if p > HP_LOW_PERCENT => YELLOW,
        _ => RED,
    }
}

pub(super) fn spans_width(spans: &[Span]) -> u16 {
    spans
        .iter()
        .map(|(text, _)| visual_width(text) as u16)
        .sum()
}

pub(super) fn put_spans(buf: &mut Buffer, x: u16, y: u16, spans: &[Span], clip: Rect) -> u16 {
    let mut col = x as i32;
    for (text, style) in spans {
        col += i32::from(put(buf, col, y as i32, text, *style, clip));
    }
    (col - x as i32).max(0) as u16
}

pub(super) fn draw_rows(buf: &mut Buffer, rows: &[SpriteRow], x: u16, y: u16, clip: Rect) {
    for (dy, row) in rows.iter().enumerate() {
        let mut col = x as i32;
        for (index, glyph) in row.text.chars().enumerate() {
            let w = unicode_width::UnicodeWidthChar::width(glyph).unwrap_or(1) as i32;
            if glyph != ' ' {
                put(
                    buf,
                    col,
                    y as i32 + dy as i32,
                    &glyph.to_string(),
                    style(row.colour_at(index)),
                    clip,
                );
            }
            col += w;
        }
    }
}

pub(super) fn draw_art(buf: &mut Buffer, art: Art, area: Rect, belly_up: bool) {
    match art {
        Art::Rows(rows) => {
            let x = area.x + area.width.saturating_sub(art.width()) / 2;
            draw_rows(buf, rows, x, area.y, area);
        }
        Art::Fish(fish) => {
            let mut pose = fish.portrait();
            pose.facing = Direction::Right;
            pose.abduction_lock = belly_up;
            draw_fish_centred(buf, &pose, area, BACKGROUND);
        }
        Art::Cow(cow) => draw_cow(buf, cow, area),
    }
    if !belly_up {
        return;
    }
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if buf[(x, y)].symbol() != " " {
                buf[(x, y)].set_fg(DARK_GRAY);
            }
        }
    }
}

fn draw_cow(buf: &mut Buffer, cow: &Cow, area: Rect) {
    let rows = cow_sprite(cow);
    let width = rows.iter().map(Vec::len).max().unwrap_or(0) as u16;
    let x0 = area.x + area.width.saturating_sub(width) / 2;
    let y0 = area.y + area.height.saturating_sub(rows.len() as u16) / 2;
    for (dy, row) in rows.iter().enumerate() {
        for (dx, &(glyph, colour)) in row.iter().enumerate() {
            let (x, y) = (x0 + dx as u16, y0 + dy as u16);
            if glyph == COW_TRANSPARENT || x >= area.right() || y >= area.bottom() {
                continue;
            }
            put(
                buf,
                i32::from(x),
                i32::from(y),
                &glyph.to_string(),
                style(colour),
                area,
            );
        }
    }
}

pub(super) fn frame_width(screen: Screen) -> u16 {
    match screen.whole.width >= MARGIN_FROM {
        true => screen.whole.width - MARGIN * 2,
        false => screen.whole.width,
    }
}

fn hints(state: &FightState, position: Option<(usize, usize)>) -> HintBar {
    if state.is_busy() {
        return HintBar::new(HINT_ESC_LEAVE);
    }
    if let Some(outcome) = state.outcome() {
        let next = match outcome {
            Outcome::Wiped => HINT_ENTER_SWIM_HOME,
            Outcome::Won | Outcome::Fled => HINT_ENTER_GO_ON,
        };
        return HintBar::new(HINT_ESC_LEAVE).action(next);
    }
    if state.offering().is_some() {
        return HintBar::new(HINT_ESC_LEAVE)
            .counted(HINT_CHOOSE, position)
            .action(HINT_ENTER_ANSWER);
    }
    if state.actor().is_none() {
        return HintBar::new(HINT_ESC_LEAVE);
    }
    match state.menu {
        Menu::Main(_) | Menu::Offer(_) => HintBar::new(HINT_ESC_LEAVE)
            .counted(HINT_CHOOSE, position)
            .action(HINT_ENTER_SELECT),
        Menu::Skills(_) | Menu::Items(_) => HintBar::new(HINT_ESC_BACK)
            .counted(HINT_CHOOSE, position)
            .action(HINT_ENTER_SELECT),
        Menu::Target { target, .. } => {
            let aim = match state.side_of(target) {
                Side::Foes => HINT_TARGET,
                Side::Party => HINT_WHICH_FISH,
            };
            HintBar::new(HINT_ESC_BACK)
                .action(aim)
                .action(HINT_ENTER_CONFIRM)
        }
    }
}

#[derive(Clone)]
struct Entry {
    label: String,
    style: Style,
}

fn turn_rows(scene: &Scene, width: u16) -> Vec<Vec<Entry>> {
    let mut rows: Vec<Vec<Entry>> = vec![Vec::new()];
    let mut used = 0u16;
    for (place, &index) in scene.state.shown.order.iter().enumerate() {
        if index >= scene.state.battle.fighters.len() {
            continue;
        }
        let side = scene.state.battle.fighters[index].side;
        let (label, style) = match (place, side) {
            (0, Side::Party) => (format!("{LEAD}{}", scene.name(index)), inverse(WHITE)),
            (0, Side::Foes) => (format!("{LEAD}{}", scene.name(index)), inverse(RED)),
            (_, Side::Party) => (scene.name(index).to_string(), style(WHITE)),
            (_, Side::Foes) => (scene.name(index).to_string(), style(LIGHT_RED)),
        };
        let label = ellipsize(&label, width as usize);
        let w = visual_width(&label) as u16;
        let sep = visual_width(SEPARATOR) as u16;
        let current = rows.last_mut().expect("a row");
        if !current.is_empty() && used + sep + w > width {
            rows.push(Vec::new());
            used = 0;
        }
        let current = rows.last_mut().expect("a row");
        if !current.is_empty() {
            used += sep;
        }
        used += w;
        current.push(Entry { label, style });
    }
    rows
}

fn turn_spans(row: &[Entry]) -> Vec<Span> {
    let mut spans = Vec::new();
    for (place, entry) in row.iter().enumerate() {
        if place > 0 {
            spans.push((SEPARATOR.to_string(), style(DARK_GRAY)));
        }
        spans.push((entry.label.clone(), entry.style));
    }
    spans
}

pub(super) fn tone_colour(tone: Tone) -> Color {
    match tone {
        Tone::Plain | Tone::Hit => GRAY,
        Tone::Hurt => LIGHT_RED,
        Tone::Heal => LIGHT_GREEN,
        Tone::Gold => AMBER,
        Tone::Status => LIGHT_BLUE,
        Tone::Tell => YELLOW,
        Tone::Round | Tone::Quiet => DARK_GRAY,
        Tone::Down => RED,
        Tone::Focus => WHITE,
    }
}

pub(super) fn log_lines(log: &[LogLine], width: u16) -> Vec<Span> {
    let width = width.max(1) as usize;
    let indent = visual_width(CONTINUATION);
    let mut lines = Vec::new();
    for line in log {
        let colour = style(tone_colour(line.tone));
        let first = wrap_words(&line.text, width);
        let head = first.first().cloned().unwrap_or_default();
        lines.push((head.clone(), colour));
        if first.len() <= 1 {
            continue;
        }
        let rest = line.text[line.text.find(&head).map_or(0, |at| at + head.len())..].trim();
        for piece in wrap_words(rest, width.saturating_sub(indent).max(1)) {
            lines.push((format!("{CONTINUATION}{piece}"), colour));
        }
    }
    lines
}

struct MenuRow {
    label: String,
    right: Option<Span>,
    usable: bool,
    selected: bool,
}

fn main_rows(scene: &Scene) -> Vec<MenuRow> {
    let state = scene.state;
    if state.offering().is_some() {
        let selected = match state.menu {
            Menu::Offer(selected) => selected,
            _ => 0,
        };
        return Answer::ALL
            .iter()
            .enumerate()
            .map(|(i, answer)| MenuRow {
                label: answer.name().to_string(),
                right: None,
                usable: true,
                selected: i == selected,
            })
            .collect();
    }
    let Some(actor) = scene.actor else {
        return Vec::new();
    };
    let selected = match state.menu {
        Menu::Main(selected) => Some(selected),
        Menu::Skills(_) | Menu::Items(_) | Menu::Target { .. } | Menu::Offer(_) => None,
    };
    let open = match state.menu {
        Menu::Skills(_)
        | Menu::Target {
            pending: Pending::Skill(_),
            ..
        } => Some(Command::Skills),
        Menu::Items(_)
        | Menu::Target {
            pending: Pending::Item(_),
            ..
        } => Some(Command::Items),
        _ => None,
    };
    Command::ALL
        .iter()
        .enumerate()
        .map(|(i, &command)| {
            let usable = state.usable(actor, command);
            MenuRow {
                label: command.name().to_string(),
                right: command
                    .opens_a_list()
                    .then(|| (MORE.to_string(), style(row_colour(usable)))),
                usable,
                selected: selected == Some(i) || open == Some(command),
            }
        })
        .collect()
}

pub(super) fn row_colour(usable: bool) -> Color {
    match usable {
        true => WHITE,
        false => DARK_GRAY,
    }
}

fn sub_list(scene: &Scene) -> Option<(&'static str, Vec<MenuRow>)> {
    let state = scene.state;
    let actor = scene.actor?;
    let battle = &state.battle;
    let (skills, selected) = match state.menu {
        Menu::Skills(selected) => (true, Some(selected)),
        Menu::Items(selected) => (false, Some(selected)),
        Menu::Target {
            pending: Pending::Skill(_),
            ..
        } => (true, None),
        Menu::Target {
            pending: Pending::Item(_),
            ..
        } => (false, None),
        _ => return None,
    };
    if skills {
        let rows = battle.fighters[actor]
            .skills
            .iter()
            .enumerate()
            .map(|(i, skill)| {
                let usable = state.skill_ready(actor, i);
                let ink = match (usable, skill.bubbles()) {
                    (false, _) => DARK_GRAY,
                    (true, 0) => GRAY,
                    (true, _) => LIGHT_CYAN,
                };
                MenuRow {
                    label: skill.name.to_string(),
                    right: Some((cost_label(skill.price), style(ink))),
                    usable,
                    selected: selected == Some(i),
                }
            })
            .collect();
        return Some((Command::Skills.name(), rows));
    }
    let rows = state
        .packed()
        .iter()
        .enumerate()
        .map(|(i, &pack)| {
            let usable = state.item_ready(actor, pack);
            MenuRow {
                label: battle.items[pack].item.name().to_string(),
                right: Some((
                    format!("×{}", battle.items[pack].count),
                    style(row_colour(usable)),
                )),
                usable,
                selected: selected == Some(i),
            }
        })
        .collect();
    Some((Command::Items.name(), rows))
}

fn list_width(rows: &[MenuRow]) -> u16 {
    rows.iter()
        .map(|row| {
            let right = row
                .right
                .as_ref()
                .map_or(0, |(text, _)| visual_width(text) as u16 + COST_GAP);
            visual_width(CURSOR) as u16 + visual_width(&row.label) as u16 + right
        })
        .max()
        .unwrap_or(0)
        + BORDERS
}

fn draw_list(buf: &mut Buffer, rect: Rect, title: &str, rows: &[MenuRow]) {
    clear(buf, rect, BACKGROUND);
    draw_box_border(buf, rect, "", FRAME, BACKGROUND);
    if !title.is_empty() {
        draw_box_title(buf, rect, &titled(title), WHITE, BACKGROUND);
    }
    let inner = inner(rect);
    for (i, row) in rows.iter().enumerate() {
        let y = inner.y + i as u16;
        if y >= inner.bottom() {
            return;
        }
        let cursor = if row.selected { CURSOR } else { NO_CURSOR };
        let colour = style(row_colour(row.usable));
        put(
            buf,
            inner.x as i32,
            y as i32,
            &format!("{cursor}{}", row.label),
            colour,
            inner,
        );
        if let Some((text, ink)) = &row.right {
            let x = inner.right().saturating_sub(visual_width(text) as u16);
            put(buf, x as i32, y as i32, text, *ink, inner);
        }
    }
}

pub(super) fn inner(rect: Rect) -> Rect {
    Rect::new(
        rect.x + BORDER.min(rect.width),
        rect.y + BORDER.min(rect.height),
        rect.width.saturating_sub(BORDERS),
        rect.height.saturating_sub(BORDERS),
    )
}

fn ink_colour(ink: Ink) -> Color {
    match ink {
        Ink::Plain => WHITE,
        Ink::Quiet => GRAY,
        Ink::Bubbles => LIGHT_CYAN,
        Ink::Gold => AMBER,
        Ink::Warning => YELLOW,
        Ink::Danger => RED,
    }
}

fn info_lines(info: &Info, width: u16) -> Vec<Vec<Span>> {
    let width = width.max(1);
    let mut lines: Vec<Vec<Span>> = wrap_words(&info.about, width as usize)
        .into_iter()
        .map(|line| vec![(line, style(GRAY))])
        .collect();
    if !info.rows.is_empty() {
        lines.push(Vec::new());
    }
    let value_w = width.saturating_sub(LABEL_W).max(1) as usize;
    for row in &info.rows {
        let label = format!("{:<w$}", row.label, w = LABEL_W as usize);
        for (n, piece) in wrap_words(&row.value, value_w).into_iter().enumerate() {
            let head = match n {
                0 => label.clone(),
                _ => " ".repeat(LABEL_W as usize),
            };
            lines.push(vec![
                (head, style(DARK_GRAY)),
                (piece, style(ink_colour(row.ink))),
            ]);
        }
    }
    lines
}

fn folded_info(info: &Info, width: u16) -> Vec<Vec<Span>> {
    let mut lines = info_lines(
        &Info {
            about: String::new(),
            ..info.clone()
        },
        width,
    );
    lines.retain(|spans| spans.iter().any(|(text, _)| !text.trim().is_empty()));
    lines.extend(
        wrap_words(&info.about, width.max(1) as usize)
            .into_iter()
            .map(|line| vec![(line, style(GRAY))]),
    );
    lines
}

fn info_height(info: &Info, box_w: u16) -> u16 {
    let text_w = box_w.saturating_sub(BORDERS + PAD * 2);
    (info_lines(info, text_w).len() as u16 + BORDERS).max(INFO_MIN_H)
}

fn draw_info(buf: &mut Buffer, rect: Rect, info: &Info) {
    clear(buf, rect, BACKGROUND);
    draw_box_border(buf, rect, "", FRAME, BACKGROUND);
    draw_box_title(buf, rect, &titled(&info.title), WHITE, BACKGROUND);
    let inner = inner(rect);
    let text = Rect::new(
        inner.x + PAD.min(inner.width),
        inner.y,
        inner.width.saturating_sub(PAD * 2),
        inner.height,
    );
    for (row, spans) in info_lines(info, text.width).iter().enumerate() {
        let y = text.y + row as u16;
        if y >= text.bottom() {
            return;
        }
        put_spans(buf, text.x, y, spans, text);
    }
}

struct Menus {
    main: Option<(Rect, Vec<MenuRow>)>,
    sub: Option<(Rect, &'static str, Vec<MenuRow>)>,
    info: Option<(Rect, Info)>,
    height: u16,
    beside: bool,
}

impl Menus {
    fn of(scene: &Scene, width: u16) -> Menus {
        let rows = main_rows(scene);
        if rows.is_empty() {
            return Menus {
                main: None,
                sub: None,
                info: None,
                height: 0,
                beside: true,
            };
        }
        let main_w = MAIN_MENU_W.max(list_width(&rows)).min(width);
        let main_h = rows.len() as u16 + BORDERS;
        let main = Rect::new(0, 0, main_w, main_h);
        let mut used = main_w + GAP;
        let sub = sub_list(scene).map(|(title, list)| {
            let w = SUB_MIN_W
                .max(list_width(&list))
                .min(width.saturating_sub(used));
            let rect = Rect::new(used, 0, w, list.len().max(1) as u16 + BORDERS);
            used += w + GAP;
            (rect, title, list)
        });
        let rows_h = sub
            .as_ref()
            .map_or(main_h, |(rect, ..)| main_h.max(rect.height));
        let info = scene.state.info();
        let room = width.saturating_sub(used);
        let beside = room >= INFO_MIN_W;
        let info = info.map(|info| {
            let rect = match beside {
                true => {
                    let w = room.min(INFO_MAX_W);
                    Rect::new(used, 0, w, info_height(&info, w))
                }
                false => {
                    let w = width.min(INFO_MAX_W.max(width));
                    Rect::new(0, rows_h, w, info_height(&info, w))
                }
            };
            (rect, info)
        });
        let height = match (&info, beside) {
            (Some((rect, _)), true) => rows_h.max(rect.height),
            (Some((rect, _)), false) => rows_h + rect.height,
            (None, _) => rows_h,
        };
        Menus {
            main: Some((main, rows)),
            sub,
            info,
            height,
            beside,
        }
    }

    fn draw(self, buf: &mut Buffer, origin: Rect, stretch_to: u16) {
        let at = |rect: Rect| {
            Rect::new(
                origin.x + rect.x,
                origin.y + rect.y,
                rect.width,
                rect.height,
            )
            .intersection(origin)
        };
        if let Some((rect, rows)) = &self.main {
            draw_list(buf, at(*rect), "", rows);
        }
        if let Some((rect, title, rows)) = &self.sub {
            draw_list(buf, at(*rect), title, rows);
        }
        if let Some((rect, info)) = &self.info {
            let mut rect = at(*rect);
            if self.beside {
                rect.height = rect.height.max(stretch_to.saturating_sub(rect.y));
                rect = rect.intersection(origin);
            }
            draw_info(buf, rect, info);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Look {
    Drawn,
    Lines,
}

struct Window {
    index: usize,
    x: u16,
    row: usize,
    width: u16,
    height: u16,
    whole: bool,
}

struct Windows {
    placed: Vec<Window>,
    rows: Vec<u16>,
    raise: bool,
}

impl Windows {
    fn of(scene: &Scene, width: u16) -> Option<Windows> {
        let mut placed = Vec::new();
        let mut rows: Vec<u16> = Vec::new();
        let mut x = 0u16;
        for &index in &scene.party {
            let whole = !scene.compact() || scene.actor == Some(index);
            let (w, h) = match whole {
                true => {
                    let art = scene.art(index);
                    (
                        WINDOW_MIN_W.max(art.width() + WINDOW_CHROME_W),
                        art.height() + WINDOW_CHROME_H,
                    )
                }
                false => {
                    let line = visual_width(&scene.label(index)) as u16
                        + LINE_GAP
                        + spans_width(&vitals(scene.view(index)));
                    (WINDOW_MIN_W.max(line + WINDOW_CHROME_W), LINE_WINDOW_H)
                }
            };
            if w > width {
                return None;
            }
            if rows.is_empty() || x + w > width {
                rows.push(0);
                x = 0;
            }
            let row = rows.len() - 1;
            rows[row] = rows[row].max(h);
            placed.push(Window {
                index,
                x,
                row,
                width: w,
                height: h,
                whole,
            });
            x += w + GAP;
        }
        let raise = placed
            .iter()
            .any(|w| w.row == 0 && scene.actor == Some(w.index));
        Some(Windows {
            placed,
            rows,
            raise,
        })
    }

    fn height(&self) -> u16 {
        self.rows.iter().sum::<u16>() + u16::from(self.raise)
    }

    fn draw(&self, buf: &mut Buffer, scene: &Scene, area: Rect) {
        let top = area.bottom().saturating_sub(self.height());
        for window in &self.placed {
            let active = scene.actor == Some(window.index);
            let lift = u16::from(self.raise && !active);
            let y = top + lift + self.rows[..window.row].iter().sum::<u16>();
            let rect =
                Rect::new(area.x + window.x, y, window.width, window.height).intersection(area);
            draw_window(buf, scene, window, rect, active);
        }
    }
}

fn draw_window(buf: &mut Buffer, scene: &Scene, window: &Window, rect: Rect, active: bool) {
    if rect.width < BORDERS || rect.height < BORDERS {
        return;
    }
    clear(buf, rect, BACKGROUND);
    let frame = if active { WHITE } else { FRAME };
    draw_box_border(buf, rect, "", frame, BACKGROUND);
    let index = window.index;
    let view = scene.view(index);
    let text = Rect::new(
        rect.x + BORDER + PAD,
        rect.y + BORDER,
        rect.width.saturating_sub(BORDERS + PAD * 2),
        rect.height.saturating_sub(BORDERS),
    );
    let label = ellipsize(&scene.label(index), text.width as usize);
    if !window.whole {
        let w = put(
            buf,
            text.x as i32,
            text.y as i32,
            &label,
            scene.name_style(index),
            text,
        );
        let spans = vitals(view);
        let x = (text.x + w + LINE_GAP).min(text.right().saturating_sub(spans_width(&spans)));
        put_spans(buf, x, text.y, &spans, text);
        return;
    }
    put(
        buf,
        text.x as i32,
        text.y as i32,
        &label,
        scene.name_style(index),
        text,
    );
    let art = scene.art(index);
    let art_area = Rect::new(
        rect.x + BORDER,
        text.y + 1,
        rect.width.saturating_sub(BORDERS),
        art.height(),
    )
    .intersection(rect);
    draw_art(buf, art, art_area, view.belly_up);
    let vitals_y = rect.bottom().saturating_sub(BORDER + 1);
    put_spans(buf, text.x, vitals_y, &vitals(view), text);
}

fn draw_line_rows(buf: &mut Buffer, scene: &Scene, list: &[usize], area: Rect) {
    for (row, spans) in line_rows(scene, list, area.width).into_iter().enumerate() {
        let y = area.y + row as u16;
        if y >= area.bottom() {
            return;
        }
        put_spans(buf, area.x, y, &spans, area);
    }
}

fn line_rows(scene: &Scene, list: &[usize], width: u16) -> Vec<Vec<Span>> {
    let marker_w = visual_width(LEAD) as u16;
    let widest_vitals = list
        .iter()
        .map(|&i| spans_width(&vitals(scene.view(i))))
        .max()
        .unwrap_or(0);
    let longest = list
        .iter()
        .map(|&i| visual_width(scene.name(i)) as u16)
        .max()
        .unwrap_or(0);
    let room = width.saturating_sub(PAD + marker_w + LINE_GAP + widest_vitals);
    let name_w = longest.min(room).max(1) as usize;
    list.iter()
        .map(|&i| {
            let marker = match scene.actor == Some(i) {
                true => LEAD.to_string(),
                false => " ".repeat(marker_w as usize),
            };
            let name = ellipsize(scene.name(i), name_w);
            let pad = name_w.saturating_sub(visual_width(&name)) + LINE_GAP as usize;
            let mut spans = vec![
                (" ".repeat(PAD as usize), style(WHITE)),
                (marker, scene.name_style(i)),
                (name, scene.name_style(i)),
                (" ".repeat(pad), style(WHITE)),
            ];
            spans.extend(vitals(scene.view(i)));
            spans
        })
        .collect()
}

struct Enemies {
    look: Look,
    art_rows: u16,
}

impl Enemies {
    fn of(scene: &Scene, width: u16, look: Look) -> Option<Enemies> {
        let art_rows = scene
            .foes
            .iter()
            .map(|&i| scene.art(i).height())
            .max()
            .unwrap_or(0)
            .max(ENEMY_MIN_ROWS);
        if look == Look::Lines {
            return Some(Enemies { look, art_rows });
        }
        let slot = width / scene.foes.len().max(1) as u16;
        let fits = scene.foes.iter().all(|&i| {
            let need = (visual_width(scene.name(i)) as u16)
                .max(scene.art(i).width())
                .max(spans_width(&vitals(scene.view(i))));
            need < slot
        });
        fits.then_some(Enemies { look, art_rows })
    }

    fn height(&self, scene: &Scene) -> u16 {
        match self.look {
            Look::Drawn => self.art_rows + ENEMY_CHROME_ROWS,
            Look::Lines => scene.foes.len() as u16,
        }
    }

    fn draw(&self, buf: &mut Buffer, scene: &Scene, area: Rect) {
        if self.look == Look::Lines {
            draw_line_rows(buf, scene, &scene.foes, area);
            return;
        }
        let slot = area.width / scene.foes.len().max(1) as u16;
        for (n, &index) in scene.foes.iter().enumerate() {
            let centre = area.x + n as u16 * slot + slot / 2;
            let name = scene.name(index);
            let name_x = centre.saturating_sub(visual_width(name) as u16 / 2);
            put(
                buf,
                name_x as i32,
                area.y as i32,
                name,
                scene.name_style(index),
                area,
            );
            let art = scene.art(index);
            let art_x = centre.saturating_sub(art.width() / 2);
            let art_area =
                Rect::new(art_x, area.y + 1, art.width(), self.art_rows).intersection(area);
            draw_art(buf, art, art_area, false);
            let spans = vitals(scene.view(index));
            let x = centre.saturating_sub(spans_width(&spans) / 2);
            put_spans(buf, x, area.y + 1 + self.art_rows, &spans, area);
        }
    }
}

enum Party {
    Windows(Windows),
    Lines,
}

impl Party {
    fn height(&self, scene: &Scene) -> u16 {
        match self {
            Party::Windows(windows) => windows.height(),
            Party::Lines => scene.party.len() as u16,
        }
    }
}

struct Wide {
    log_w: u16,
    left_w: u16,
    turns: Vec<Vec<Entry>>,
    enemies: Enemies,
    party: Party,
    padded: bool,
    hint_h: u16,
    height: u16,
}

impl Wide {
    fn plan(scene: &Scene, inner_w: u16, room: u16, hints: &HintBar) -> Option<Wide> {
        let log_w = (inner_w * LOG_SHARE_PERCENT / PERCENT).clamp(LOG_MIN_W, LOG_MAX_W);
        let left_w = inner_w.checked_sub(log_w + GAP)?;
        if left_w < LEFT_MIN_W {
            return None;
        }
        let content_w = left_w.saturating_sub(PAD * 2);
        let turns = turn_rows(scene, inner_w.saturating_sub(PAD * 2));
        let hint_h = hints.height(left_w);
        let menus_h = Menus::of(scene, content_w).height;
        let looks = [
            (true, Look::Drawn, true),
            (true, Look::Drawn, false),
            (false, Look::Drawn, true),
            (false, Look::Drawn, false),
            (false, Look::Lines, false),
        ];
        for (windows, enemy_look, padded) in looks {
            let Some(enemies) = Enemies::of(scene, content_w, enemy_look) else {
                continue;
            };
            let party = match windows {
                true => match Windows::of(scene, content_w) {
                    Some(windows) => Party::Windows(windows),
                    None => continue,
                },
                false => Party::Lines,
            };
            let pads = if padded { PAD * 3 } else { 0 };
            let height = turns.len() as u16
                + 1
                + pads
                + enemies.height(scene)
                + menus_h
                + party.height(scene)
                + hint_h;
            if height <= room {
                return Some(Wide {
                    log_w,
                    left_w,
                    turns,
                    enemies,
                    party,
                    padded,
                    hint_h,
                    height,
                });
            }
        }
        None
    }

    fn draw(self, buf: &mut Buffer, scene: &Scene, screen: Screen, width: u16, hints: &HintBar) {
        let height = (self.height + BORDERS)
            .max(screen.stage.height)
            .min(screen.whole.height);
        let rect = screen.place(width, height);
        clear(buf, rect, BACKGROUND);
        draw_box_border(buf, rect, "", FRAME, BACKGROUND);
        draw_box_title(buf, rect, &titled(&scene.state.title), WHITE, BACKGROUND);
        let inner = inner(rect);
        for (row, entries) in self.turns.iter().enumerate() {
            let spans = turn_spans(entries);
            let w = spans_width(&spans);
            let x = inner.x + inner.width.saturating_sub(w) / 2;
            put_spans(buf, x, inner.y + row as u16, &spans, inner);
        }
        let rule_y = inner.y + self.turns.len() as u16;
        let divider_x = inner.x + self.left_w;
        rule(buf, rect, rule_y);
        put_char(buf, divider_x, rule_y, SPLIT_DOWN);
        for y in rule_y + 1..inner.bottom() {
            put_char(buf, divider_x, y, UPRIGHT);
        }
        put_char(buf, divider_x, rect.bottom().saturating_sub(1), SPLIT_UP);
        let body_top = rule_y + 1;
        let body_h = inner.bottom().saturating_sub(body_top);
        let left = Rect::new(inner.x, body_top, self.left_w, body_h);
        let content = Rect::new(
            left.x + PAD,
            left.y,
            left.width.saturating_sub(PAD * 2),
            left.height,
        );
        let log = Rect::new(
            divider_x + 1 + PAD,
            body_top,
            self.log_w.saturating_sub(PAD * 2),
            body_h,
        );
        draw_log(buf, &scene.state.shown.log, log);
        let hints_rect = Rect::new(
            left.x,
            left.bottom().saturating_sub(self.hint_h),
            left.width,
            self.hint_h,
        );
        hints.draw(buf, hints_rect, BACKGROUND);
        let mut y = content.y + u16::from(self.padded) * PAD;
        let enemies_h = self.enemies.height(scene);
        self.enemies.draw(
            buf,
            scene,
            Rect::new(content.x, y, content.width, enemies_h),
        );
        y += enemies_h + u16::from(self.padded) * PAD;
        let party_h = self.party.height(scene);
        let party_bottom = hints_rect.y;
        let party_top = party_bottom.saturating_sub(party_h);
        let menus = Menus::of(scene, content.width);
        let menus_area = Rect::new(content.x, y, content.width, party_top.saturating_sub(y));
        let stretch = party_top.saturating_sub(GAP);
        let party_area = Rect::new(content.x, party_top, content.width, party_h);
        match &self.party {
            Party::Windows(windows) => windows.draw(buf, scene, party_area),
            Party::Lines => draw_line_rows(
                buf,
                scene,
                &scene.party,
                Rect::new(left.x, party_top, left.width, party_h),
            ),
        }
        menus.draw(buf, menus_area, stretch);
    }
}

pub(super) fn put_char(buf: &mut Buffer, x: u16, y: u16, glyph: char) {
    if x < buf.area.right() && y < buf.area.bottom() {
        buf[(x, y)].set_char(glyph).set_style(style(FRAME));
    }
}

pub(super) fn rule(buf: &mut Buffer, rect: Rect, y: u16) {
    if rect.width < BORDERS {
        return;
    }
    put_char(buf, rect.x, y, TEE_RIGHT);
    for x in rect.x + 1..rect.right() - 1 {
        put_char(buf, x, y, RULE);
    }
    put_char(buf, rect.right() - 1, y, TEE_LEFT);
}

fn draw_log(buf: &mut Buffer, log: &[LogLine], area: Rect) {
    let lines = log_lines(log, area.width);
    let shown = lines.len().saturating_sub(area.height as usize);
    for (row, (text, style)) in lines[shown..].iter().enumerate() {
        put(
            buf,
            area.x as i32,
            area.y as i32 + row as i32,
            text,
            *style,
            area,
        );
    }
}

enum Line {
    Text(Vec<Span>),
    Rule,
    Titled(String),
}

struct Column {
    fighters: Vec<Line>,
    controls: Vec<Line>,
    target: Option<usize>,
    focus: Range<usize>,
    position: Option<(usize, usize)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fold {
    Open,
    Collapsed,
}

impl Column {
    fn of(scene: &Scene, width: u16, turn_bar: Vec<Line>, fold: Fold) -> Column {
        let pinned = turn_bar.len() == 1;
        let mut fighters = turn_bar;
        if !pinned {
            fighters.push(Line::Rule);
        }
        let foes_at = fighters.len();
        fighters.extend(
            line_rows(scene, &scene.foes, width)
                .into_iter()
                .map(Line::Text),
        );
        fighters.push(Line::Rule);
        let party_at = fighters.len();
        fighters.extend(
            line_rows(scene, &scene.party, width)
                .into_iter()
                .map(Line::Text),
        );
        if pinned {
            fighters.push(Line::Rule);
        }
        let target = scene
            .targeted
            .and_then(|target| match scene.state.side_of(target) {
                Side::Foes => scene
                    .foes
                    .iter()
                    .position(|&i| i == target)
                    .map(|p| foes_at + p),
                Side::Party => scene
                    .party
                    .iter()
                    .position(|&i| i == target)
                    .map(|p| party_at + p),
            });
        let mut controls: Vec<Line> = Vec::new();
        let mut focus = 0..0;
        let mut position = None;
        let rows = main_rows(scene);
        if !rows.is_empty() {
            let list = sub_list(scene);
            let (shown, title) = match &list {
                Some((title, list)) => (list, *title),
                None => (&rows, ""),
            };
            match (title.is_empty(), pinned) {
                (false, _) => controls.push(Line::Titled(title.to_string())),
                (true, false) => controls.push(Line::Rule),
                (true, true) => {}
            }
            let selected = shown.iter().position(|row| row.selected);
            let start = controls.len() + selected.unwrap_or(0);
            let label_w = list_width(shown).saturating_sub(BORDERS);
            let listed: Vec<&MenuRow> = match (fold, selected) {
                (Fold::Collapsed, Some(at)) => {
                    position = Some((at + 1, shown.len()));
                    vec![&shown[at]]
                }
                (Fold::Collapsed, None) => Vec::new(),
                (Fold::Open, _) => shown.iter().collect(),
            };
            let start = match fold {
                Fold::Collapsed => controls.len(),
                Fold::Open => start,
            };
            for row in listed {
                let cursor = if row.selected { CURSOR } else { NO_CURSOR };
                let label = format!("{cursor}{}", row.label);
                let mut spans = vec![
                    (" ".repeat(PAD as usize), style(WHITE)),
                    (label.clone(), style(row_colour(row.usable))),
                ];
                if let Some((text, ink)) = &row.right {
                    let gap = label_w
                        .saturating_sub(visual_width(&label) as u16 + visual_width(text) as u16)
                        .max(COST_GAP);
                    spans.push((" ".repeat(gap as usize), style(WHITE)));
                    spans.push((text.clone(), *ink));
                }
                controls.push(Line::Text(spans));
            }
            if let Some(info) = scene.state.info() {
                let lines = match fold {
                    Fold::Open => {
                        controls.push(Line::Titled(info.title.clone()));
                        info_lines(&info, width.saturating_sub(PAD * 2))
                    }
                    Fold::Collapsed => folded_info(&info, width.saturating_sub(PAD * 2)),
                };
                for spans in lines {
                    let mut line = vec![(" ".repeat(PAD as usize), style(WHITE))];
                    line.extend(spans);
                    controls.push(Line::Text(line));
                }
            }
            focus = start..controls.len();
        }
        if !scene.state.shown.log.is_empty() {
            controls.push(Line::Rule);
            for (text, style) in log_lines(&scene.state.shown.log, width.saturating_sub(PAD * 2)) {
                controls.push(Line::Text(vec![
                    (" ".repeat(PAD as usize), style),
                    (text, style),
                ]));
            }
            if focus.is_empty() {
                let end = controls.len();
                focus = end.saturating_sub(1)..end;
            }
        }
        Column {
            fighters,
            controls,
            target,
            focus,
            position,
        }
    }
}

fn turn_line(scene: &Scene, width: u16) -> Line {
    let spans = turn_spans(&turn_rows(scene, u16::MAX).concat());
    let room = width.saturating_sub(PAD * 2) as usize;
    let mut used = 0usize;
    let mut line = vec![(" ".repeat(PAD as usize), style(WHITE))];
    for (text, style) in spans {
        let w = visual_width(&text);
        if used + w > room {
            line.push((ellipsize(&text, room.saturating_sub(used)), style));
            break;
        }
        used += w;
        line.push((text, style));
    }
    Line::Text(line)
}

fn centred_turns(scene: &Scene, width: u16) -> Vec<Line> {
    turn_rows(scene, width.saturating_sub(PAD * 2))
        .iter()
        .map(|row| {
            let spans = turn_spans(row);
            let indent = width.saturating_sub(spans_width(&spans)) / 2;
            let mut line = vec![(" ".repeat(indent as usize), style(WHITE))];
            line.extend(spans);
            Line::Text(line)
        })
        .collect()
}

fn draw_lines(buf: &mut Buffer, rect: Rect, lines: &[Line], top: u16) {
    let inner = inner(rect);
    for (row, line) in lines.iter().enumerate() {
        let y = top + row as u16;
        if y >= inner.bottom() {
            return;
        }
        match line {
            Line::Rule => rule(buf, rect, y),
            Line::Titled(title) => {
                rule(buf, rect, y);
                put(
                    buf,
                    (rect.x + TITLE_INSET) as i32,
                    y as i32,
                    &titled(title),
                    style(WHITE).add_modifier(Modifier::BOLD),
                    Rect::new(rect.x, y, rect.width.saturating_sub(BORDER), 1),
                );
            }
            Line::Text(spans) => {
                put_spans(buf, inner.x, y, spans, inner);
            }
        }
    }
}

fn draw_column(
    buf: &mut Buffer,
    scene: &Scene,
    screen: Screen,
    width: u16,
    bar: impl Fn(Option<(usize, usize)>) -> HintBar,
) {
    let inner_w = width.saturating_sub(BORDERS);
    let hint_h = bar(None).height(inner_w);
    let room = screen.whole.height.saturating_sub(BORDERS + hint_h);
    let short = Column::of(scene, inner_w, vec![turn_line(scene, inner_w)], Fold::Open);
    let window = room.saturating_sub(short.fighters.len() as u16);
    let pinned = window >= CONTROL_MIN_ROWS;
    let column = match pinned {
        true if short.focus.len() > window as usize => Column::of(
            scene,
            inner_w,
            vec![turn_line(scene, inner_w)],
            Fold::Collapsed,
        ),
        true => short,
        false => {
            let open = Column::of(scene, inner_w, centred_turns(scene, inner_w), Fold::Open);
            match open.focus.len() > room as usize {
                true => Column::of(
                    scene,
                    inner_w,
                    centred_turns(scene, inner_w),
                    Fold::Collapsed,
                ),
                false => open,
            }
        }
    };
    let hints = bar(column.position);
    let hint_h = hints.height(inner_w);
    let total_rows = (column.fighters.len() + column.controls.len()) as u16;
    let height = (total_rows + hint_h + BORDERS)
        .max(screen.stage.height)
        .min(screen.whole.height);
    let rect = screen.place(width, height);
    clear(buf, rect, BACKGROUND);
    draw_box_border(buf, rect, "", FRAME, BACKGROUND);
    draw_box_title(buf, rect, &titled(&scene.state.title), WHITE, BACKGROUND);
    let inner = inner(rect);
    let hint_h = hint_h.min(inner.height.saturating_sub(1));
    let room = inner.height.saturating_sub(hint_h);
    let (fixed, scrolled, focus) = match pinned {
        true => (column.fighters, column.controls, column.focus),
        false => {
            let offset = column.fighters.len();
            let focus = match column.target {
                Some(at) => at..at + 1,
                None => column.focus.start + offset..column.focus.end + offset,
            };
            let mut all = column.fighters;
            all.extend(column.controls);
            (Vec::new(), all, focus)
        }
    };
    draw_lines(buf, rect, &fixed, inner.y);
    let top = inner.y + fixed.len() as u16;
    let window = room.saturating_sub(fixed.len() as u16) as usize;
    let total = scrolled.len();
    let shown = scene.state.scroll.reveal(focus, window, total);
    draw_lines(buf, rect, &scrolled[shown.clone()], top);
    Scrollbar {
        x: rect.right().saturating_sub(BORDER),
        top,
        height: window as u16,
    }
    .draw(buf, shown, total, WHITE);
    hints.draw(
        buf,
        Rect::new(
            inner.x,
            inner.bottom().saturating_sub(hint_h),
            inner.width,
            hint_h,
        ),
        BACKGROUND,
    );
}

pub(super) fn titled(title: &str) -> String {
    format!(" {title} ")
}

fn result(scene: &Scene) -> Option<(&'static str, Vec<Span>)> {
    let outcome = scene.state.outcome()?;
    let battle = &scene.state.battle;
    let upright = scene
        .party
        .iter()
        .filter(|&&i| !scene.view(i).belly_up)
        .count();
    let lines = match outcome {
        Outcome::Won => (
            "Victory",
            vec![
                (
                    format!("The haul: ${}", grouped(u128::from(battle.haul))),
                    style(AMBER),
                ),
                (
                    format!("{upright} of {} fish still upright.", scene.party.len()),
                    style(GRAY),
                ),
            ],
        ),
        Outcome::Wiped => (
            "Lost",
            vec![
                ("Every fish is belly up.".to_string(), style(LIGHT_RED)),
                (wipe_line(battle.setup.difficulty.kills()), style(GRAY)),
            ],
        ),
        Outcome::Fled => (
            "Fled",
            vec![("The party got away.".to_string(), style(GRAY))],
        ),
    };
    Some(lines)
}

fn wipe_line(kills: bool) -> String {
    match kills {
        true => "The ones who died stay dead.".to_string(),
        false => "Everyone swims home alive.".to_string(),
    }
}

fn draw_result(buf: &mut Buffer, scene: &Scene, screen: Screen, hints: &HintBar) {
    let Some((title, lines)) = result(scene) else {
        return;
    };
    let content_w = lines
        .iter()
        .map(|(text, _)| visual_width(text) as u16 + PAD * 2)
        .max()
        .unwrap_or(0);
    let title = titled(title);
    let modal = Modal::open(
        buf,
        screen,
        &ModalSpec {
            title: &title,
            border: WHITE,
            background: BACKGROUND,
            content_w,
            content_h: lines.len() as u16,
            hints,
        },
    );
    for (row, (text, style)) in lines.iter().enumerate() {
        let y = modal.body.y + row as u16;
        if y >= modal.body.bottom() {
            return;
        }
        put(
            buf,
            (modal.body.x + PAD) as i32,
            y as i32,
            text,
            *style,
            modal.body,
        );
    }
}

pub fn draw(buf: &mut Buffer, state: &FightState, screen: Screen) {
    let scene = Scene::of(state);
    let width = frame_width(screen);
    let inner_w = width.saturating_sub(BORDERS);
    let room = screen.whole.height.saturating_sub(BORDERS);
    let bar = hints(state, None);
    let over = state.outcome().is_some();
    let frame_bar = |position| match over {
        true => HintBar::new(HINT_ESC_LEAVE),
        false => hints(state, position),
    };
    match Wide::plan(&scene, inner_w, room, &frame_bar(None)) {
        Some(wide) => wide.draw(buf, &scene, screen, width, &frame_bar(None)),
        None => draw_column(buf, &scene, screen, width, frame_bar),
    }
    if over {
        draw_result(buf, &scene, screen, &bar);
    }
}
