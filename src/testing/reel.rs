use std::fmt::{self, Write as _};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ratatui::buffer::{Buffer, Cell};
use ratatui::style::{Color, Modifier};
use unicode_width::UnicodeWidthStr;

const DECADE: usize = 10;
const GUTTER_TAIL: &str = " │";
const DEFAULT_FG: &str = "#cccccc";
const DEFAULT_BG: &str = "#0c0c0c";
const REELS_DIR: &str = "reels";
const TEXT_EXTENSION: &str = "txt";
const HTML_EXTENSION: &str = "html";
const WIDE_CELLS: usize = 2;
const CELL_CLASS: &str = "c";
const WIDE_CELL_CLASS: &str = "w";
const CUBE_BASE: u8 = 16;
const CUBE_SIDE: u8 = 6;
const GRAY_BASE: u8 = 232;
const GRAY_START: u8 = 8;
const GRAY_STEP: u8 = 10;
const CUBE_FIRST_STEP: u8 = 55;
const CUBE_STEP: u8 = 40;
const NAMED_PALETTE: [&str; 16] = [
    "#0c0c0c", "#c50f1f", "#13a10e", "#c19c00", "#0037da", "#881798", "#3a96dd", "#cccccc",
    "#767676", "#e74856", "#16c60c", "#f9f1a5", "#3b78ff", "#b4009e", "#61d6d6", "#f2f2f2",
];

const UP: u8 = 1;
const DOWN: u8 = 2;
const LEFT: u8 = 4;
const RIGHT: u8 = 8;
const LINES: &[(&str, u8)] = &[
    ("─", LEFT | RIGHT),
    ("│", UP | DOWN),
    ("┃", UP | DOWN),
    ("━", LEFT | RIGHT),
    ("┊", UP | DOWN),
    ("┈", LEFT | RIGHT),
    ("┌", DOWN | RIGHT),
    ("┐", DOWN | LEFT),
    ("└", UP | RIGHT),
    ("┘", UP | LEFT),
    ("├", UP | DOWN | RIGHT),
    ("┤", UP | DOWN | LEFT),
    ("┬", DOWN | LEFT | RIGHT),
    ("┴", UP | LEFT | RIGHT),
    ("┼", UP | DOWN | LEFT | RIGHT),
];
const STRAIGHTS: &[&str] = &["─", "│", "┃", "━"];
const OFF_VIEW: &[&str] = &["┊", "┈"];
const COLUMN_RULE: &str = "│";
const TABLE_CROSSING: &str = "┼";
const TABLE_TEE: &str = "┬";
const RULE_LINE: &str = "─";
const INNER_RULE_START: &str = "├";
const BRANCHES: &[&str] = &["├", "└"];
const TOP_LEFT: &str = "┌";
const TOP_RIGHT: &str = "┐";
const BOTTOM_LEFT: &str = "└";
const BOTTOM_RIGHT: &str = "┘";
const LEFT_EDGE: &[&str] = &["│", "├"];
const RIGHT_EDGE: &[&str] = &["│", "┤"];
const BOTTOM_EDGE: &[&str] = &["─", "┴"];
const TITLE_LEAD: &str = "─";
const RULE_RUN: &[&str] = &["─", "┬", "┴", "┼"];
const RULE_START: &[&str] = &["┌", "├"];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Frame {
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
}

impl Frame {
    fn contains(&self, x: usize, y: usize) -> bool {
        (self.left..=self.right).contains(&x) && (self.top..=self.bottom).contains(&y)
    }

    fn has_on_edge(&self, x: usize, y: usize) -> bool {
        self.contains(x, y)
            && (x == self.left || x == self.right || y == self.top || y == self.bottom)
    }
}

fn arms_of(symbol: &str) -> u8 {
    LINES
        .iter()
        .find(|(glyph, _)| *glyph == symbol)
        .map_or(0, |(_, arms)| *arms)
}

fn is_blank(symbol: &str) -> bool {
    symbol.trim().is_empty()
}

fn is_text(symbol: &str) -> bool {
    !is_blank(symbol) && arms_of(symbol) == 0
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Arm {
    Up,
    Down,
    Left,
    Right,
}

impl Arm {
    fn bit(self) -> u8 {
        match self {
            Arm::Up => UP,
            Arm::Down => DOWN,
            Arm::Left => LEFT,
            Arm::Right => RIGHT,
        }
    }

    fn opposite(self) -> Arm {
        match self {
            Arm::Up => Arm::Down,
            Arm::Down => Arm::Up,
            Arm::Left => Arm::Right,
            Arm::Right => Arm::Left,
        }
    }

    fn step(self, x: usize, y: usize) -> Option<(usize, usize)> {
        match self {
            Arm::Up => y.checked_sub(1).map(|up| (x, up)),
            Arm::Down => Some((x, y + 1)),
            Arm::Left => x.checked_sub(1).map(|left| (left, y)),
            Arm::Right => Some((x + 1, y)),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Arm::Up => "up",
            Arm::Down => "down",
            Arm::Left => "left",
            Arm::Right => "right",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Flaw {
    pub x: usize,
    pub y: usize,
    pub glyph: String,
    pub arm: Arm,
    pub neighbour: String,
}

impl fmt::Display for Flaw {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let neighbour = if self.neighbour.is_empty() {
            "the edge of the screen".to_string()
        } else {
            format!("{:?}", self.neighbour)
        };
        write!(
            f,
            "col {} row {}: {:?} reaches {} into {}",
            self.x,
            self.y,
            self.glyph,
            self.arm.name(),
            neighbour
        )
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Glyph {
    pub symbol: String,
    pub fg: Color,
    pub bg: Color,
    pub modifier: Modifier,
}

impl Glyph {
    fn style_key(&self, flawed: bool) -> (Color, Color, Modifier, bool) {
        (self.fg, self.bg, self.modifier, flawed)
    }
}

pub struct TerminalCell<'a> {
    pub cell: &'a Cell,
    pub covered: bool,
}

impl<'a> TerminalCell<'a> {
    pub fn row(buffer: &'a Buffer, y: u16) -> impl Iterator<Item = TerminalCell<'a>> {
        let area = buffer.area;
        (area.left()..area.right()).scan(0usize, move |covering, x| {
            let cell = &buffer[(x, y)];
            let covered = *covering > 0;
            *covering = match covered {
                true => *covering - 1,
                false => cell.symbol().width().saturating_sub(1),
            };
            Some(TerminalCell { cell, covered })
        })
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Still {
    pub label: String,
    rows: Vec<Vec<Glyph>>,
}

impl Still {
    pub fn of(label: impl Into<String>, buffer: &Buffer) -> Self {
        let area = buffer.area;
        let rows = (area.top()..area.bottom())
            .map(|y| {
                TerminalCell::row(buffer, y)
                    .map(|shown| Glyph {
                        symbol: match shown.covered {
                            true => String::new(),
                            false => shown.cell.symbol().to_string(),
                        },
                        fg: shown.cell.fg,
                        bg: shown.cell.bg,
                        modifier: shown.cell.modifier,
                    })
                    .collect()
            })
            .collect();
        Self {
            label: label.into(),
            rows,
        }
    }

    pub fn width(&self) -> usize {
        self.rows.first().map_or(0, Vec::len)
    }

    pub fn height(&self) -> usize {
        self.rows.len()
    }

    pub fn row_text(&self, y: usize) -> String {
        self.rows[y]
            .iter()
            .map(|glyph| glyph.symbol.as_str())
            .collect()
    }

    pub fn text(&self) -> String {
        (0..self.height())
            .map(|y| self.row_text(y))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn glyph(&self, x: usize, y: usize) -> Option<&Glyph> {
        self.rows.get(y)?.get(x)
    }

    fn symbol_at(&self, x: usize, y: usize) -> Option<&str> {
        self.rows.get(y)?.get(x).map(|glyph| glyph.symbol.as_str())
    }

    pub fn flaws(&self) -> Vec<Flaw> {
        let frames = self.frames();
        let mut flaws = Vec::new();
        for y in 0..self.height() {
            for x in 0..self.width() {
                self.audit_cell(x, y, &frames, &mut flaws);
            }
        }
        flaws
    }

    fn frames(&self) -> Vec<Frame> {
        (0..self.height())
            .flat_map(|y| (0..self.width()).map(move |x| (x, y)))
            .filter(|&(x, y)| self.symbol_at(x, y) == Some(TOP_LEFT))
            .filter_map(|(x, y)| self.frame_from(x, y))
            .collect()
    }

    fn frame_from(&self, left: usize, top: usize) -> Option<Frame> {
        let mut bottom = top + 1;
        while self.symbol_at(left, bottom)? != BOTTOM_LEFT {
            if !LEFT_EDGE.contains(&self.symbol_at(left, bottom)?) {
                return None;
            }
            bottom += 1;
        }
        let mut right = left + 1;
        while self.symbol_at(right, bottom)? != BOTTOM_RIGHT {
            if !BOTTOM_EDGE.contains(&self.symbol_at(right, bottom)?) {
                return None;
            }
            right += 1;
        }
        let closed = self.symbol_at(right, top)? == TOP_RIGHT
            && (top + 1..bottom).all(|y| {
                self.symbol_at(right, y)
                    .is_some_and(|symbol| RIGHT_EDGE.contains(&symbol))
            });
        closed.then_some(Frame {
            left,
            top,
            right,
            bottom,
        })
    }

    fn audit_cell(&self, x: usize, y: usize, frames: &[Frame], flaws: &mut Vec<Flaw>) {
        let symbol = self.symbol_at(x, y).unwrap_or_default();
        let arms = arms_of(symbol);
        if arms == 0 {
            return;
        }
        let straight = STRAIGHTS.contains(&symbol);
        for arm in [Arm::Up, Arm::Down, Arm::Left, Arm::Right] {
            let horizontal = matches!(arm, Arm::Left | Arm::Right);
            if arms & arm.bit() == 0 || (horizontal && straight) {
                continue;
            }
            let neighbour = arm
                .step(x, y)
                .and_then(|(nx, ny)| self.symbol_at(nx, ny))
                .unwrap_or_default();
            if arms_of(neighbour) & arm.opposite().bit() != 0 {
                continue;
            }
            if self.is_deliberately_open(x, y, symbol, arm, neighbour, frames) {
                continue;
            }
            flaws.push(Flaw {
                x,
                y,
                glyph: symbol.to_string(),
                arm,
                neighbour: neighbour.to_string(),
            });
        }
    }

    fn is_deliberately_open(
        &self,
        x: usize,
        y: usize,
        symbol: &str,
        arm: Arm,
        neighbour: &str,
        frames: &[Frame],
    ) -> bool {
        let Some((nx, ny)) = arm.step(x, y) else {
            return false;
        };
        if frames
            .iter()
            .any(|frame| frame.has_on_edge(nx, ny) && !frame.contains(x, y))
        {
            return true;
        }
        if arm != Arm::Down && self.is_title_cell(nx, ny) {
            return true;
        }
        if OFF_VIEW.contains(&symbol) {
            return true;
        }
        if BRANCHES.contains(&symbol) {
            return match arm {
                Arm::Up => is_text(neighbour),
                Arm::Right | Arm::Down => is_blank(neighbour),
                Arm::Left => false,
            };
        }
        symbol == COLUMN_RULE && self.hangs_from_a_table_rule(x, y)
    }

    fn is_title_cell(&self, x: usize, y: usize) -> bool {
        let Some(symbol) = self.symbol_at(x, y) else {
            return false;
        };
        if arms_of(symbol) != 0 {
            return false;
        }
        let is_line = |cx: usize| arms_of(self.symbol_at(cx, y).unwrap_or_default()) != 0;
        let Some(start) = (0..x).rev().find(|&cx| is_line(cx)) else {
            return false;
        };
        let Some(end) = (x + 1..self.width()).find(|&cx| is_line(cx)) else {
            return false;
        };
        let lettered =
            (start + 1..end).any(|cx| is_text(self.symbol_at(cx, y).unwrap_or_default()));
        lettered
            && self.symbol_at(start, y) == Some(TITLE_LEAD)
            && arms_of(self.symbol_at(end, y).unwrap_or_default()) & LEFT != 0
            && self.rule_starts_before(start, y)
    }

    fn rule_starts_before(&self, x: usize, y: usize) -> bool {
        (0..=x)
            .rev()
            .map(|cx| self.symbol_at(cx, y).unwrap_or_default())
            .find(|symbol| !RULE_RUN.contains(symbol))
            .is_some_and(|symbol| RULE_START.contains(&symbol))
    }

    fn hangs_from_a_table_rule(&self, x: usize, y: usize) -> bool {
        let run = self.vertical_run(x, y);
        if run.contains(&TABLE_CROSSING) {
            return true;
        }
        let top = (0..=y)
            .rev()
            .take_while(|&row| {
                matches!(self.symbol_at(x, row), Some(COLUMN_RULE) | Some(TABLE_TEE))
            })
            .find(|&row| self.symbol_at(x, row) == Some(TABLE_TEE));
        top.is_some_and(|row| self.is_inner_rule(x, row))
    }

    fn is_inner_rule(&self, x: usize, y: usize) -> bool {
        (0..x)
            .rev()
            .map(|cx| self.symbol_at(cx, y).unwrap_or_default())
            .find(|&symbol| symbol != RULE_LINE && symbol != TABLE_TEE)
            == Some(INNER_RULE_START)
    }

    fn vertical_run(&self, x: usize, y: usize) -> Vec<&str> {
        let joins = |upper: usize| {
            let above = self.symbol_at(x, upper).unwrap_or_default();
            let below = self.symbol_at(x, upper + 1).unwrap_or_default();
            arms_of(above) & DOWN != 0 && arms_of(below) & UP != 0
        };
        let mut top = y;
        while top > 0 && joins(top - 1) {
            top -= 1;
        }
        let mut bottom = y;
        while bottom + 1 < self.height() && joins(bottom) {
            bottom += 1;
        }
        (top..=bottom)
            .filter_map(|row| self.symbol_at(x, row))
            .collect()
    }

    pub fn ruled(&self) -> String {
        let digits = self.height().saturating_sub(1).to_string().len();
        let gutter = " ".repeat(digits + GUTTER_TAIL.chars().count());
        let decades: String = (0..self.width())
            .map(|x| {
                if x % DECADE == 0 {
                    digit((x / DECADE) % DECADE)
                } else {
                    ' '
                }
            })
            .collect();
        let units: String = (0..self.width()).map(|x| digit(x % DECADE)).collect();
        let mut out = String::new();
        let _ = writeln!(out, "{gutter}{decades}");
        let _ = writeln!(out, "{gutter}{units}");
        for y in 0..self.height() {
            let _ = writeln!(out, "{y:>digits$}{GUTTER_TAIL}{}│", self.row_text(y));
        }
        out
    }

    fn html(&self, index: usize) -> String {
        let flaws = self.flaws();
        let mut screen = String::new();
        for y in 0..self.height() {
            let mut run = String::new();
            let mut run_key = None;
            for (x, glyph) in self.rows[y].iter().enumerate() {
                let flawed = flaws.iter().any(|flaw| flaw.x == x && flaw.y == y);
                let key = glyph.style_key(flawed);
                if run_key.is_some_and(|open| open != key) {
                    push_span(&mut screen, run_key, &run);
                    run.clear();
                }
                run_key = Some(key);
                run.push_str(&cell_html(&glyph.symbol));
            }
            push_span(&mut screen, run_key, &run);
            screen.push('\n');
        }
        let findings = if flaws.is_empty() {
            String::new()
        } else {
            let items: String = flaws
                .iter()
                .map(|flaw| format!("<li>{}</li>", escape(&flaw.to_string())))
                .collect();
            format!("<ul class=\"flaws\">{items}</ul>")
        };
        format!(
            "<section id=\"still-{index}\"><h2><span class=\"n\">{index:02}</span> {label} \
             <span class=\"dim\">{w}×{h}</span></h2><pre class=\"screen\">{ruler}{screen}</pre>\
             {findings}</section>\n",
            label = escape(&self.label),
            w = self.width(),
            h = self.height(),
            ruler = escape_ruler(self.width()),
        )
    }
}

fn digit(value: usize) -> char {
    char::from_digit(value as u32, DECADE as u32).unwrap_or(' ')
}

fn escape_ruler(width: usize) -> String {
    let units: String = (0..width).map(|x| digit(x % DECADE)).collect();
    format!("<span class=\"ruler\">{units}\n</span>")
}

fn cell_html(symbol: &str) -> String {
    if symbol.is_ascii() {
        return escape(symbol);
    }
    let class = match symbol.width() {
        WIDE_CELLS.. => WIDE_CELL_CLASS,
        _ => CELL_CLASS,
    };
    format!("<i class=\"{class}\">{}</i>", escape(symbol))
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn push_span(out: &mut String, key: Option<(Color, Color, Modifier, bool)>, run: &str) {
    let Some((fg, bg, modifier, flawed)) = key else {
        return;
    };
    if run.is_empty() {
        return;
    }
    let (mut fg_css, mut bg_css) = (css(fg, DEFAULT_FG), css(bg, DEFAULT_BG));
    if modifier.contains(Modifier::REVERSED) {
        std::mem::swap(&mut fg_css, &mut bg_css);
    }
    if modifier.contains(Modifier::HIDDEN) {
        fg_css.clone_from(&bg_css);
    }
    let mut style = format!("color:{fg_css};background:{bg_css}");
    if modifier.contains(Modifier::BOLD) {
        style.push_str(";font-weight:700");
    }
    if modifier.contains(Modifier::DIM) {
        style.push_str(";opacity:.55");
    }
    if modifier.contains(Modifier::ITALIC) {
        style.push_str(";font-style:italic");
    }
    if modifier.contains(Modifier::UNDERLINED) {
        style.push_str(";text-decoration:underline");
    }
    if modifier.contains(Modifier::CROSSED_OUT) {
        style.push_str(";text-decoration:line-through");
    }
    let class = if flawed { " class=\"flaw\"" } else { "" };
    let _ = write!(out, "<span{class} style=\"{style}\">{run}</span>");
}

fn css(color: Color, fallback: &str) -> String {
    match color {
        Color::Reset => fallback.to_string(),
        Color::Black => NAMED_PALETTE[0].to_string(),
        Color::Red => NAMED_PALETTE[1].to_string(),
        Color::Green => NAMED_PALETTE[2].to_string(),
        Color::Yellow => NAMED_PALETTE[3].to_string(),
        Color::Blue => NAMED_PALETTE[4].to_string(),
        Color::Magenta => NAMED_PALETTE[5].to_string(),
        Color::Cyan => NAMED_PALETTE[6].to_string(),
        Color::Gray => NAMED_PALETTE[7].to_string(),
        Color::DarkGray => NAMED_PALETTE[8].to_string(),
        Color::LightRed => NAMED_PALETTE[9].to_string(),
        Color::LightGreen => NAMED_PALETTE[10].to_string(),
        Color::LightYellow => NAMED_PALETTE[11].to_string(),
        Color::LightBlue => NAMED_PALETTE[12].to_string(),
        Color::LightMagenta => NAMED_PALETTE[13].to_string(),
        Color::LightCyan => NAMED_PALETTE[14].to_string(),
        Color::White => NAMED_PALETTE[15].to_string(),
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Indexed(index) => indexed(index),
    }
}

fn indexed(index: u8) -> String {
    if let Some(named) = NAMED_PALETTE.get(index as usize) {
        return named.to_string();
    }
    if index >= GRAY_BASE {
        let level = GRAY_START + (index - GRAY_BASE) * GRAY_STEP;
        return format!("#{level:02x}{level:02x}{level:02x}");
    }
    let offset = index - CUBE_BASE;
    let channel = |step: u8| {
        if step == 0 {
            0
        } else {
            CUBE_FIRST_STEP + step * CUBE_STEP
        }
    };
    let (r, g, b) = (
        channel(offset / (CUBE_SIDE * CUBE_SIDE)),
        channel((offset / CUBE_SIDE) % CUBE_SIDE),
        channel(offset % CUBE_SIDE),
    );
    format!("#{r:02x}{g:02x}{b:02x}")
}

const PAGE_STYLE: &str = "
:root{color-scheme:dark}
body{margin:0;background:#16161a;color:#d8d8dc;font:14px/1.45 system-ui,sans-serif}
header{padding:20px 28px 12px;border-bottom:1px solid #2a2a30;position:sticky;top:0;background:#16161aee;z-index:2}
header h1{margin:0 0 4px;font-size:18px}
header p{margin:0;color:#9a9aa3}
header label{margin-left:18px;color:#9a9aa3;cursor:pointer}
.layout{display:grid;grid-template-columns:260px 1fr}
nav{padding:16px 12px 40px 20px;position:sticky;top:78px;align-self:start;max-height:calc(100vh - 90px);overflow:auto;font-size:13px}
nav a{display:block;color:#b8b8c0;text-decoration:none;padding:2px 6px;border-radius:4px}
nav a:hover{background:#26262c;color:#fff}
nav a.bad{color:#ff7b72}
main{padding:12px 28px 60px;min-width:0}
section{margin:0 0 30px}
h2{font-size:14px;font-weight:600;margin:0 0 8px;color:#e8e8ec}
.n{color:#6e6e78;font-variant-numeric:tabular-nums;margin-right:4px}
.dim{color:#6e6e78;font-weight:400}
pre.screen{margin:0;background:#0c0c0c;color:#cccccc;padding:10px 12px;border-radius:6px;border:1px solid #2a2a30;overflow-x:auto;font:13px/1.05 'Cascadia Mono',Consolas,'DejaVu Sans Mono',monospace;width:max-content;max-width:100%}
pre.screen i{display:inline-block;font-style:inherit;text-align:center;overflow:hidden;vertical-align:top;height:1.05em}
pre.screen i.c{width:1ch}
pre.screen i.w{width:2ch}
.ruler{display:none;color:#4a4a52!important}
body.rulers .ruler{display:inline}
.flaw{outline:1px solid #ff4d4d;outline-offset:-1px}
ul.flaws{margin:8px 0 0;padding-left:20px;color:#ff7b72;font:12px/1.5 Consolas,monospace}
.ok{color:#7ee787}
@media (max-width:900px){.layout{grid-template-columns:1fr}nav{position:static;max-height:none}}
.bad{color:#ff7b72}
";

#[derive(Clone, Default, PartialEq, Debug)]
pub struct Reel {
    stills: Vec<Still>,
}

impl Reel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, still: Still) {
        self.stills.push(still);
    }

    pub fn stills(&self) -> &[Still] {
        &self.stills
    }

    pub fn flaw_report(&self) -> String {
        let mut report = String::new();
        for (index, still) in self.stills.iter().enumerate() {
            for flaw in still.flaws() {
                let _ = writeln!(report, "still {index:02} {:?}: {flaw}", still.label);
            }
        }
        report
    }

    pub fn text(&self, title: &str) -> String {
        let mut out = format!("{title}\n\n");
        for (index, still) in self.stills.iter().enumerate() {
            let _ = writeln!(
                out,
                "━━ {index:02} · {} ━━ {}×{}",
                still.label,
                still.width(),
                still.height()
            );
            out.push_str(&still.ruled());
            for flaw in still.flaws() {
                let _ = writeln!(out, "   FLAW {flaw}");
            }
            out.push('\n');
        }
        out
    }

    pub fn html(&self, title: &str) -> String {
        let flawed = self
            .stills
            .iter()
            .filter(|still| !still.flaws().is_empty())
            .count();
        let verdict = if flawed == 0 {
            "<span class=\"ok\">no broken borders</span>".to_string()
        } else {
            format!("<span class=\"bad\">{flawed} still(s) with broken borders</span>")
        };
        let nav: String = self
            .stills
            .iter()
            .enumerate()
            .map(|(index, still)| {
                let class = if still.flaws().is_empty() {
                    ""
                } else {
                    " class=\"bad\""
                };
                format!(
                    "<a{class} href=\"#still-{index}\">{index:02} {}</a>",
                    escape(&still.label)
                )
            })
            .collect();
        let sections: String = self
            .stills
            .iter()
            .enumerate()
            .map(|(index, still)| still.html(index))
            .collect();
        format!(
            "<meta charset=\"utf-8\">\n<title>{title}</title>\n<style>{PAGE_STYLE}</style>\n<header><h1>{title}</h1>\
             <p>{count} stills rendered by the real App through src/testing · {verdict}\
             <label><input type=\"checkbox\" onchange=\"document.body.classList.toggle('rulers',this.checked)\"> \
             column ruler</label></p></header>\n<div class=\"layout\"><nav>{nav}</nav><main>{sections}</main></div>\n",
            title = escape(title),
            count = self.stills.len(),
        )
    }

    pub fn save(&self, dir: &Path, name: &str) -> io::Result<PathBuf> {
        let reels = dir.join(REELS_DIR);
        fs::create_dir_all(&reels)?;
        fs::write(
            reels.join(name).with_extension(TEXT_EXTENSION),
            self.text(name),
        )?;
        let html = reels.join(name).with_extension(HTML_EXTENSION);
        fs::write(&html, self.html(name))?;
        Ok(html)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    fn still_of(rows: &[&str]) -> Still {
        let width = rows[0].width() as u16;
        let mut buffer = Buffer::empty(Rect::new(0, 0, width, rows.len() as u16));
        for (y, row) in rows.iter().enumerate() {
            buffer.set_string(0, y as u16, row, ratatui::style::Style::default());
        }
        Still::of("fixture", &buffer)
    }

    #[test]
    fn a_closed_box_with_a_title_and_a_divider_is_whole() {
        let still = still_of(&[
            "┌─ Shop ─┬──┐",
            "│ a      │  │",
            "├────────┤  │",
            "│ b      │  │",
            "└────────┴──┘",
        ]);
        assert_eq!(still.flaws(), Vec::new());
    }

    #[test]
    fn a_right_border_overwritten_by_text_is_a_flaw_on_both_sides_of_the_gap() {
        let still = still_of(&["┌────┐", "│ ok │", "│ tool", "└────┘"]);
        let flaws = still.flaws();
        assert_eq!(flaws.len(), 2, "{flaws:?}");
        assert!(
            flaws
                .iter()
                .any(|flaw| flaw.y == 1 && flaw.arm == Arm::Down && flaw.neighbour == "l")
        );
        assert!(
            flaws
                .iter()
                .any(|flaw| flaw.y == 3 && flaw.arm == Arm::Up && flaw.neighbour == "l")
        );
    }

    #[test]
    fn a_corner_with_nothing_beside_it_is_a_flaw() {
        let still = still_of(&["┌ x ┐", "│   │", "└───┘"]);
        let flaws = still.flaws();
        assert_eq!(flaws.len(), 2, "{flaws:?}");
        assert!(flaws.iter().all(|flaw| flaw.y == 0));
    }

    #[test]
    fn a_table_column_rule_may_float_between_the_title_and_the_padding_row() {
        let still = still_of(&[
            "┌─ Index ─────┐",
            "│ Name │ Kind │",
            "├──────┼──────┤",
            "│ Cid  │ Koi  │",
            "│             │",
            "└─────────────┘",
        ]);
        assert_eq!(still.flaws(), Vec::new());
    }

    #[test]
    fn a_frame_side_that_stops_short_is_still_a_flaw() {
        let still = still_of(&["┌───┐", "│   │", "│    ", "└───┘"]);
        let flaws = still.flaws();
        assert_eq!(flaws.len(), 2, "{flaws:?}");
    }

    #[test]
    fn a_border_under_a_wide_glyphs_right_half_is_covered_as_on_a_terminal() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 3));
        buffer.set_string(0, 2, "彡", ratatui::style::Style::default());
        for (y, row) in ["┌─┐", "│ │", "└─┘"].iter().enumerate() {
            buffer.set_string(1, y as u16, row, ratatui::style::Style::default());
        }
        let still = Still::of("a fish behind a modal", &buffer);
        assert_eq!(still.symbol_at(1, 2), Some(""));
        assert!(
            !still.flaws().is_empty(),
            "ratatui never sends the cell a wide glyph covers, so the corner is not on screen"
        );
    }

    #[test]
    fn a_wide_glyph_covers_whatever_the_backend_kept_in_the_next_cell() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 1));
        buffer.set_string(0, 0, "界ab", ratatui::style::Style::default());
        buffer[(1, 0)].set_symbol("C");
        let still = Still::of("a stale letter from an earlier frame", &buffer);
        assert_eq!(still.row_text(0), "界ab");
    }

    #[test]
    fn a_tree_of_branches_hangs_from_its_label() {
        let still = still_of(&["Assay     ", "├ value   ", "├ over    ", "└ under   "]);
        assert_eq!(still.flaws(), Vec::new());
    }

    #[test]
    fn a_tree_that_repeats_its_last_branch_is_a_flaw() {
        let still = still_of(&["Assay     ", "└ value   ", "└ over    ", "└ under   "]);
        let flaws = still.flaws();
        assert_eq!(
            flaws.len(),
            2,
            "every └ after the first hangs from nothing: {flaws:?}"
        );
        assert!(
            flaws
                .iter()
                .all(|flaw| flaw.arm == Arm::Up && flaw.neighbour == "└")
        );
    }

    #[test]
    fn a_title_may_cover_the_junction_of_the_divider_beneath_it() {
        let still = still_of(&[
            "┌─ Coil to the Fishtank! ─┐",
            "│ art │ text              │",
            "└─────┴───────────────────┘",
        ]);
        assert_eq!(still.flaws(), Vec::new());
    }

    #[test]
    fn a_junction_may_meet_the_space_that_closes_a_title() {
        let still = still_of(&["┌─ Shop ┬────┐", "│ pen   │ a  │", "└───────┴────┘"]);
        assert_eq!(still.flaws(), Vec::new());
    }

    #[test]
    fn a_divider_under_a_bare_rule_still_needs_its_junction() {
        let still = still_of(&["┌───────────┐", "│ art │ txt │", "└─────┴─────┘"]);
        let flaws = still.flaws();
        assert_eq!(flaws.len(), 1, "{flaws:?}");
        assert_eq!(flaws[0].arm, Arm::Up);
    }

    const POPUP_OVER_A_DIVIDER: &[&str] = &[
        "┌──────┬──────────┐",
        "│      │          │",
        "│      ┌─ Buy ─┐  │",
        "│      │ <2>   │  │",
        "│      └───────┘  │",
        "│      │          │",
        "└──────┴──────────┘",
    ];

    #[test]
    fn a_popup_floats_over_the_frame_beneath_it() {
        let still = still_of(POPUP_OVER_A_DIVIDER);
        assert_eq!(still.flaws(), Vec::new());
    }

    #[test]
    fn a_popup_missing_a_corner_is_not_a_popup() {
        let mut rows: Vec<String> = POPUP_OVER_A_DIVIDER
            .iter()
            .map(|row| row.to_string())
            .collect();
        rows[4] = "│      └───────   │".to_string();
        let borrowed: Vec<&str> = rows.iter().map(String::as_str).collect();
        let flaws = still_of(&borrowed).flaws();
        assert!(
            flaws
                .iter()
                .any(|flaw| flaw.y == 1 && flaw.arm == Arm::Down),
            "the divider that runs into an unclosed box is reported: {flaws:?}"
        );
    }

    #[test]
    fn a_wide_glyph_hides_the_cell_it_spills_into() {
        let still = still_of(&["界ab"]);
        assert_eq!(still.row_text(0), "界ab");
        assert_eq!(
            still.width(),
            4,
            "a still keeps one glyph per terminal cell"
        );
    }

    #[test]
    fn a_glyph_outside_ascii_keeps_to_its_own_cells_in_the_html() {
        let html = still_of(&["aｱ界─"]).html(0);
        assert!(html.contains("<i class=\"c\">ｱ</i>"), "{html}");
        assert!(html.contains("<i class=\"w\">界</i>"), "{html}");
        assert!(html.contains("<i class=\"c\">─</i>"), "{html}");
        assert!(!html.contains("<i class=\"c\">a</i>"), "{html}");
    }

    #[test]
    fn the_ruled_view_numbers_every_row_and_fences_trailing_space() {
        let still = still_of(&["ab ", "cd "]);
        let ruled = still.ruled();
        assert!(ruled.contains("0 │ab │"), "{ruled}");
        assert!(ruled.contains("1 │cd │"), "{ruled}");
        assert!(ruled.contains("012"), "{ruled}");
    }

    #[test]
    fn named_and_indexed_colours_share_one_palette() {
        assert_eq!(css(Color::Red, DEFAULT_FG), indexed(1));
        assert_eq!(indexed(GRAY_BASE), "#080808");
        assert_eq!(indexed(CUBE_BASE), "#000000");
        assert_eq!(css(Color::Reset, DEFAULT_BG), DEFAULT_BG);
    }
}
