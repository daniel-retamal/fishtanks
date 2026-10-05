use rand::{SeedableRng, rngs::SmallRng};
use ratatui::style::Color;

use super::{Fish, FishState, LineSprite, WAVE_THRESHOLD};
use crate::colors::{
    AMBER_DARK, CREAM, DARK_GRAY, GOLD, GOLD_BRIGHT, GOLD_PALE, LIGHT_MAGENTA, LIGHT_YELLOW, PINK,
    WHITE,
};
use crate::entities::glistening::{GlisteningMode, color_for_glisten, derive_glistening_palette};
use crate::fishes::figure::{Figure, Paint};
use crate::fishes::habits::{Crawl, Side};
use crate::fishes::mutant::BILL_LEAD;
use crate::fishes::mutations::Mutatable;
use crate::fishes::species::{Habit, Skin, Zoomie};
use crate::fishes::unfish::UnfishKind;
use crate::loot::junk_cell;
use crate::sprite::{Band, Cell, TRANSPARENT, mirror_char};

const LURE: char = 'º';
const LURE_STALK: char = ',';
const LURE_ROOT: char = '.';
const BILL: char = '-';
const DORSAL_LEAD: char = '/';
const VENTRAL_LEAD: char = '\\';
const FIN_BACK: char = '|';
const SPIKE_FRONT: char = '\\';
const SPIKE_MIDDLE: char = '|';
const SPIKE_BACK: char = '/';
const PUFFED_BODY: char = ':';
pub const MOON_LIT: Color = CREAM;
pub const MOON_DARK: Color = DARK_GRAY;
const LURE_BY_DAY: Color = AMBER_DARK;
const FLASH_PALETTE: [Color; 3] = [GOLD_BRIGHT, LIGHT_YELLOW, GOLD_PALE];
const BLUSH_PEAK: Color = LIGHT_MAGENTA;
const TWINKLE_PEAK: Color = WHITE;
const WING_UP: char = '/';
const WING_DOWN: char = '\\';
const WING_ROOT: usize = 1;
const WING_SPAN: usize = 2;
const SOLO_DORSAL: char = '^';
const SOLO_VENTRAL: char = 'v';

fn blank_row(width: usize) -> Vec<Cell> {
    vec![(TRANSPARENT, Color::Reset); width]
}

fn is_painted(cell: Cell) -> bool {
    cell.0 != TRANSPARENT && cell.0 != ' '
}

fn widen(sprite: &mut LineSprite, cells: usize, at_front: bool) {
    for row in &mut sprite.rows {
        let pad = std::iter::repeat_n((TRANSPARENT, Color::Reset), cells);
        if at_front {
            row.splice(0..0, pad);
        } else {
            row.extend(pad);
        }
    }
}

fn width_of(sprite: &LineSprite) -> usize {
    sprite.rows.iter().map(Vec::len).max().unwrap_or(0)
}

fn even_out(sprite: &mut LineSprite) {
    let width = width_of(sprite);
    for row in &mut sprite.rows {
        row.resize(width, (TRANSPARENT, Color::Reset));
    }
}

fn place(sprite: &mut LineSprite, cells: &[(usize, Cell)], above: bool) {
    even_out(sprite);
    let width = width_of(sprite);
    let fits = |row: &Vec<Cell>| {
        cells
            .iter()
            .all(|&(col, _)| col < width && !is_painted(row[col]))
    };
    let mut row_index = sprite.body_row as isize;
    let step: isize = if above { -1 } else { 1 };
    loop {
        row_index += step;
        if row_index < 0 || row_index as usize >= sprite.rows.len() {
            break;
        }
        if fits(&sprite.rows[row_index as usize]) {
            let row = &mut sprite.rows[row_index as usize];
            for &(col, cell) in cells {
                row[col] = cell;
            }
            return;
        }
    }
    let mut row = blank_row(width);
    for &(col, cell) in cells {
        if col < width {
            row[col] = cell;
        }
    }
    if above {
        sprite.rows.insert(0, row);
        sprite.body_row += 1;
    } else {
        sprite.rows.push(row);
    }
}

fn mirror_glyph(glyph: char, head_left: bool) -> char {
    if head_left { glyph } else { mirror_char(glyph) }
}

fn flip_vertical(glyph: char) -> char {
    match glyph {
        '.' => '\'',
        '\'' => '.',
        '_' => '¯',
        '¯' => '_',
        '^' => 'v',
        'v' => '^',
        '/' => '\\',
        '\\' => '/',
        other => other,
    }
}

fn quarter_turn(glyph: char) -> char {
    match glyph {
        '_' | '-' | '=' | '¯' => '|',
        '|' | '¦' => '-',
        '~' => ')',
        '≈' => '(',
        '(' | '{' | '<' => '^',
        ')' | '}' | '>' => 'v',
        '.' | ',' => '\'',
        '"' => ':',
        other => other,
    }
}

impl Fish {
    pub(super) fn head_on_left(&self) -> bool {
        let reversed = self
            .unfish_state
            .as_ref()
            .is_some_and(|us| us.kind == UnfishKind::Reversed);
        self.facing_left() != reversed
    }

    pub(super) fn figure_sprite(&self, figure: &Figure) -> LineSprite {
        let wave = self.sway.phase.sin() > WAVE_THRESHOLD;
        let facing_left = self.facing_left();
        let art = figure.pose(facing_left, wave, self.is_zooming());
        let config = self.species.config();
        let shift = match config.skin {
            Skin::Cycle(_) => self.habits.hue,
            _ => 0,
        };
        let recolored = self.mutant.as_ref().is_some_and(|m| !m.patterned);
        let eye = self
            .mutant
            .as_ref()
            .and_then(|m| m.eye_color)
            .or(config.eye_color)
            .unwrap_or(config.palette[0]);
        let mut cells = art.cells();
        let mut junk_slots: Vec<(usize, usize)> = cells
            .iter()
            .enumerate()
            .flat_map(|(r, row)| {
                row.iter()
                    .enumerate()
                    .filter(|(_, cell)| matches!(cell, Some((_, Paint::Junk))))
                    .map(move |(c, _)| (r, c))
            })
            .collect();
        if !facing_left {
            junk_slots.reverse();
        }
        let mut junk = SmallRng::seed_from_u64(self.pattern_seed);
        let mut shells: Vec<((usize, usize), Cell)> = Vec::new();
        for slot in junk_slots {
            let (glyph, color) = junk_cell(&mut junk);
            shells.push((slot, (mirror_glyph(glyph, facing_left), color)));
        }
        let mut rows: Vec<Vec<Cell>> = cells
            .drain(..)
            .map(|row| {
                row.into_iter()
                    .map(|cell| match cell {
                        None => (TRANSPARENT, Color::Reset),
                        Some((glyph, Paint::Eye)) => (glyph, eye),
                        Some((glyph, _)) if recolored => (glyph, self.color),
                        Some((glyph, paint)) => (glyph, paint.color(config.palette, shift, eye)),
                    })
                    .collect()
            })
            .collect();
        for ((r, c), cell) in shells {
            rows[r][c] = cell;
        }
        if let Some(mutant) = self.mutant.as_ref()
            && let Some(peak) = mutant.glistening_color
        {
            let (base, mid, _) = derive_glistening_palette(self.color);
            for row in &mut rows {
                let n = row.len();
                for (i, cell) in row.iter_mut().enumerate() {
                    if is_painted(*cell) {
                        cell.1 = color_for_glisten(
                            mutant.glistening_mode,
                            self.sway.phase,
                            i,
                            n,
                            base,
                            mid,
                            peak,
                        );
                    }
                }
            }
        }
        LineSprite {
            rows,
            body_row: art.body_row,
        }
    }

    pub(super) fn adorn(&self, mut sprite: LineSprite, span: Option<(usize, usize)>) -> LineSprite {
        let adornments = self.adornments();
        let head_left = self.head_on_left();
        let mut span = span;
        if self.is_puffed() {
            self.puff_up(&mut sprite, span);
        }
        if adornments.lunar
            && let Some((lo, hi)) = span
        {
            let width = hi + 1 - lo;
            let body = &mut sprite.rows[sprite.body_row];
            for (offset, cell) in body[lo..=hi].iter_mut().enumerate() {
                if !is_painted(*cell) {
                    continue;
                }
                let screen = if head_left {
                    offset
                } else {
                    width - 1 - offset
                };
                cell.1 = if self.sky.lights(screen, width) {
                    MOON_LIT
                } else {
                    MOON_DARK
                };
            }
        }
        if self.is_gliding()
            && let Some(span) = span
        {
            self.spread_wings(&mut sprite, span);
        }
        let lead = adornments.lead();
        if lead > 0 {
            widen(&mut sprite, lead, head_left);
            if head_left {
                span = span.map(|(lo, hi)| (lo + lead, hi + lead));
            }
            let width = width_of(&sprite);
            let body_row = sprite.body_row;
            let mouth = if head_left { lead } else { width - 1 - lead };
            let skin = sprite.rows[body_row]
                .get(mouth)
                .map_or(self.color, |cell| cell.1);
            let ahead_of_mouth = |steps: usize| {
                if head_left {
                    mouth - steps
                } else {
                    mouth + steps
                }
            };
            let mut reach = 0;
            if adornments.bill {
                for _ in 0..BILL_LEAD {
                    reach += 1;
                    sprite.rows[body_row][ahead_of_mouth(reach)] = (BILL, skin);
                }
            }
            if adornments.lure {
                let gap = ahead_of_mouth(reach + 1);
                let lure = ahead_of_mouth(reach + 2);
                sprite.rows[body_row][lure] = (LURE, self.lure_color());
                let root = ahead_of_mouth(reach);
                let over_the_body = span.is_some_and(|(lo, hi)| (lo..=hi).contains(&root));
                let stalk: &[(usize, Cell)] = if over_the_body {
                    &[(gap, (LURE_STALK, skin))]
                } else {
                    &[(gap, (LURE_STALK, skin)), (root, (LURE_ROOT, skin))]
                };
                place(&mut sprite, stalk, true);
            }
        }
        if let Some((lo, hi)) = span {
            let mid = (lo + hi) / 2;
            let color_at = |sprite: &LineSprite, col: usize| {
                sprite.rows[sprite.body_row]
                    .get(col)
                    .map_or(self.color, |cell| cell.1)
            };
            if adornments.dorsal_fin {
                let cells = fin_cells(lo, hi, mid, head_left, DORSAL_LEAD, |c| {
                    color_at(&sprite, c)
                });
                place(&mut sprite, &cells, true);
            }
            if adornments.ventral_fin {
                let cells = fin_cells(lo, hi, mid, head_left, VENTRAL_LEAD, |c| {
                    color_at(&sprite, c)
                });
                place(&mut sprite, &cells, false);
            }
        }
        even_out(&mut sprite);
        sprite
    }

    fn lure_color(&self) -> Color {
        if self.sky.daylight {
            return LURE_BY_DAY;
        }
        if self.sway.phase.sin() > 0.0 {
            GOLD_BRIGHT
        } else {
            GOLD
        }
    }

    fn puff_up(&self, sprite: &mut LineSprite, span: Option<(usize, usize)>) {
        let Some((lo, hi)) = span else {
            return;
        };
        let mid = (lo + hi) / 2;
        let body_row = sprite.body_row;
        let colors: Vec<Color> = sprite.rows[body_row].iter().map(|cell| cell.1).collect();
        for cell in &mut sprite.rows[body_row][lo..=hi] {
            if is_painted(*cell) {
                cell.0 = PUFFED_BODY;
            }
        }
        let spikes = |marks: [(usize, char); 3]| -> Vec<(usize, Cell)> {
            if lo == hi {
                return vec![(lo, (SPIKE_MIDDLE, colors[lo]))];
            }
            marks
                .into_iter()
                .map(|(col, glyph)| (col, (glyph, colors[col])))
                .collect()
        };
        if self.band_free(Band::Top) {
            let above = [(lo, SPIKE_FRONT), (mid, SPIKE_MIDDLE), (hi, SPIKE_BACK)];
            place(sprite, &spikes(above), true);
        }
        if self.band_free(Band::Bottom) {
            let below = [(lo, SPIKE_BACK), (mid, SPIKE_MIDDLE), (hi, SPIKE_FRONT)];
            place(sprite, &spikes(below), false);
        }
    }

    pub fn is_gliding(&self) -> bool {
        self.zoomie() == Zoomie::Glide && self.is_zooming()
    }

    fn spread_wings(&self, sprite: &mut LineSprite, span: (usize, usize)) {
        let (lo, hi) = span;
        let head_left = self.head_on_left();
        let roots: Vec<usize> = (0..WING_SPAN)
            .map(|step| {
                if head_left {
                    (lo + WING_ROOT + step).min(hi)
                } else {
                    hi.saturating_sub(WING_ROOT + step).max(lo)
                }
            })
            .collect();
        let colors: Vec<Color> = roots
            .iter()
            .map(|&col| sprite.rows[sprite.body_row][col].1)
            .collect();
        for top in [true, false] {
            let glyph = mirror_glyph(if top { WING_UP } else { WING_DOWN }, head_left);
            let cells: Vec<(usize, Cell)> = roots
                .iter()
                .zip(colors.iter())
                .map(|(&col, &color)| (col, (glyph, color)))
                .collect();
            place(sprite, &cells, top);
        }
    }

    pub(super) fn dress(&self, mut sprite: LineSprite) -> LineSprite {
        let habit = self.habit();
        if habit == Some(Habit::Sync) && self.habits.lit > 0.0 {
            recolor(&mut sprite, |i, _| FLASH_PALETTE[i % FLASH_PALETTE.len()]);
        }
        if habit == Some(Habit::Twinkle) && self.sky.is_night() {
            glisten(&mut sprite, self.sway.phase, GOLD_PALE, TWINKLE_PEAK);
        }
        if habit == Some(Habit::Shy) && self.sky.calm {
            glisten(&mut sprite, self.sway.phase, PINK, BLUSH_PEAK);
        }
        self.tint(&mut sprite);
        sprite
    }

    pub(super) fn keeps_eyes_shut(&self) -> bool {
        if self.abduction_lock || self.is_asleep() || self.is_sated() {
            return true;
        }
        match self.habit() {
            Some(Habit::Shy) => !self.sky.calm,
            Some(Habit::Ambush) => self.habits.alert <= 0.0,
            _ => false,
        }
    }

    pub(super) fn gaped(&self, mouth: char) -> char {
        if self.habit() == Some(Habit::Gape) && self.sway.phase.sin() > WAVE_THRESHOLD {
            return '=';
        }
        mouth
    }

    pub fn crawl_pose(&self, sprite: LineSprite) -> LineSprite {
        let Some(crawl) = self.habits.crawl else {
            return sprite;
        };
        pose(sprite, crawl)
    }

    pub fn lead_cells_left(&self) -> Vec<Cell> {
        let adornments = self.adornments();
        let mut cells = Vec::new();
        if adornments.lure {
            cells.push((LURE, self.lure_color()));
            cells.push((TRANSPARENT, Color::Reset));
        }
        if adornments.bill {
            for _ in 0..BILL_LEAD {
                cells.push((BILL, self.color));
            }
        }
        cells
    }

    pub fn is_eating(&self) -> bool {
        matches!(self.state, FishState::Eating { .. })
    }
}

fn fin_cells(
    lo: usize,
    hi: usize,
    mid: usize,
    head_left: bool,
    lead_glyph: char,
    color_at: impl Fn(usize) -> Color,
) -> Vec<(usize, Cell)> {
    if lo == hi {
        let solo = if lead_glyph == DORSAL_LEAD {
            SOLO_DORSAL
        } else {
            SOLO_VENTRAL
        };
        return vec![(lo, (solo, color_at(lo)))];
    }
    if head_left {
        let back = (mid + 1).min(hi);
        return vec![
            (mid, (lead_glyph, color_at(mid))),
            (back, (FIN_BACK, color_at(back))),
        ];
    }
    let front = lo + hi - mid;
    let back = front.saturating_sub(1).max(lo);
    vec![
        (back, (FIN_BACK, color_at(back))),
        (front, (mirror_char(lead_glyph), color_at(front))),
    ]
}

fn recolor(sprite: &mut LineSprite, color: impl Fn(usize, Color) -> Color) {
    for row in &mut sprite.rows {
        for (i, cell) in row.iter_mut().enumerate() {
            if is_painted(*cell) {
                cell.1 = color(i, cell.1);
            }
        }
    }
}

fn glisten(sprite: &mut LineSprite, phase: f32, base: Color, peak: Color) {
    let (low, mid, _) = derive_glistening_palette(base);
    let width = width_of(sprite);
    recolor(sprite, |i, _| {
        color_for_glisten(GlisteningMode::Wave, phase, i, width, low, mid, peak)
    });
}

fn transposed(sprite: &LineSprite) -> LineSprite {
    let width = width_of(sprite);
    let height = sprite.rows.len();
    let rows = (0..width)
        .map(|col| {
            (0..height)
                .map(|row| {
                    let cell = sprite.rows[row]
                        .get(col)
                        .copied()
                        .unwrap_or((TRANSPARENT, Color::Reset));
                    (quarter_turn(cell.0), cell.1)
                })
                .collect()
        })
        .collect();
    let body_col = sprite.rows[sprite.body_row]
        .iter()
        .position(|&cell| is_painted(cell))
        .unwrap_or(0);
    LineSprite {
        rows,
        body_row: body_col,
    }
}

fn upside_down(mut sprite: LineSprite) -> LineSprite {
    sprite.rows.reverse();
    for row in &mut sprite.rows {
        for cell in row.iter_mut() {
            cell.0 = flip_vertical(cell.0);
        }
    }
    sprite.body_row = sprite.rows.len() - 1 - sprite.body_row;
    sprite
}

fn mirrored(mut sprite: LineSprite) -> LineSprite {
    for row in &mut sprite.rows {
        row.reverse();
        for cell in row.iter_mut() {
            cell.0 = mirror_char(cell.0);
        }
    }
    sprite
}

fn pose(sprite: LineSprite, crawl: Crawl) -> LineSprite {
    match crawl.side {
        Side::Floor => sprite,
        Side::Ceiling => upside_down(sprite),
        Side::Right | Side::Left => {
            let mut column = transposed(&sprite);
            if !crawl.climbing() {
                column = upside_down(column);
            }
            if crawl.side == Side::Left {
                column = mirrored(column);
            }
            column
        }
    }
}

pub fn posed_size(sprite: &LineSprite) -> (usize, usize) {
    (width_of(sprite), sprite.rows.len())
}
