use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};
use unicode_width::UnicodeWidthChar;

use crate::colors::{LIGHT_CYAN, LIGHT_GREEN, PINK, WHITE, YELLOW};
use crate::sprite::{
    Cell, EAR_LEFT, EAR_RIGHT, Feet, PosedExtension, TRANSPARENT, feet_row, opaque_line,
    painted_span,
};

use crate::{
    entities::bubble::Bubble,
    entities::cow::{Cow, cow_sprite},
    entities::food::Food,
    entities::glistening::{color_for_glisten, derive_glistening_palette},
    entities::plant::Seaweed,
    entities::speech::{Side, Tail, build_bubble, build_speech_bubble, is_bubble_text, shift_into},
    entities::ufo::{Ufo, ufo_sprite},
    fishes::botfish::BotfishState,
    fishes::fish::{Fish, line_extension_bands},
    fishes::parts::Display,
    fishes::unfish::{
        BALL_BASE, BALL_CENTER_ROW, BALL_EYE_COL, BALL_EYE_ROW, BALL_WIDTH, SKULL_CENTER_ROW,
        SKULL_CLOSED, SKULL_OPEN, SKULL_WIDTH, UNFISH_BODY_COLOR, UNFISH_EYE_COLOR, UnfishKind,
        is_multi_row,
    },
    tank::speech_ink,
    tank::{Tank, TankBackground},
    tanks::alien::{AlienPyramid, AlienStar, pyramid_canvas_w, pyramid_lines},
    tanks::coral::{
        CORAL_A_LINES, CORAL_A_ROWS, CORAL_COLOR, CoralAlgaeInstance, CoralStructure,
        FLOOR_ALGAE_A, FLOOR_ALGAE_COLOR, FloorAlgae, algae_art, algae_row_shift, mirror_char,
        query_anim_char, vert_wave_char,
    },
    tanks::desert::{
        DesertSky, SUN_H_WAVE_AMPLITUDE, SUN_H_WAVE_SPREAD, SUN_LINES, SUN_ORBIT_RY, sun_cell_color,
    },
    tanks::gate::{
        BARS_PER_BLOCK, BarHoles, BlockMarks, GATE_BAR_PIPE, GATE_BAR_TILE, GATE_BAR_W,
        GATE_BLOCK_MARK_COL, GATE_BLOCK_TILE, GATE_BLOCK_W, GATE_FLOOR_CHAR, Gate,
    },
    tanks::haunted::{BAT_COLOR, BAT_SPRITE, Bat, GRAVE_TRANSPARENT, Ghost},
    tanks::heaven::{Angel, Cloud, HeavenBackground},
    tanks::hell::{
        FACE_COLOR, H_WAVE_AMPLITUDE, H_WAVE_ROW_SPREAD, HellBackground, RANDOM_FACE_COLOR,
    },
    tanks::matrix::{MatrixBackground, trail_color},
    tanks::radioactive::{FLUID_COLOR, FluidChar, RadBarrel},
    tanks::soul_wall::SoulWall,
    tanks::void::{VOID_EYE_CENTER_X, VOID_EYE_VERTICAL_OFFSET, VoidBackground},
    ui::overdraw,
    void_ritual::VOID_TEXT_BELOW_EYE_OFFSET,
};

const NET_COLOR: Color = LIGHT_CYAN;

pub struct TankView<'a> {
    tank: &'a Tank,
    show_names: bool,
    show_nets: bool,
    show_epitaphs: bool,
    ritual_text: Option<[Option<String>; 2]>,
}

impl<'a> TankView<'a> {
    pub fn new(tank: &'a Tank) -> Self {
        Self {
            tank,
            show_names: false,
            show_nets: false,
            show_epitaphs: true,
            ritual_text: None,
        }
    }

    pub fn with_epitaphs(mut self, show_epitaphs: bool) -> Self {
        self.show_epitaphs = show_epitaphs;
        self
    }

    pub fn with_names(mut self, show_names: bool) -> Self {
        self.show_names = show_names;
        self
    }

    pub fn with_nets(mut self, show_nets: bool) -> Self {
        self.show_nets = show_nets;
        self
    }

    pub fn with_ritual(mut self, text: [Option<String>; 2]) -> Self {
        self.ritual_text = Some(text);
        self
    }
}

impl Widget for TankView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let ritual_active = self.ritual_text.is_some() && self.tank.kind.config().hosts_ritual;

        match &self.tank.background {
            TankBackground::Plain { plants } => {
                for plant in plants {
                    if plant.x >= area.width as i32 {
                        break;
                    }
                    render_seaweed(plant, area, buf);
                }
            }
            TankBackground::Coral { plants, corals, .. } => {
                for plant in plants {
                    if plant.x >= area.width as i32 {
                        break;
                    }
                    render_seaweed(plant, area, buf);
                }
                for coral in corals {
                    if coral.base_x >= area.width as i32 {
                        break;
                    }
                    render_coral_structure(coral, area, buf);
                }
                for coral in corals {
                    if coral.base_x >= area.width as i32 {
                        break;
                    }
                    render_coral_algae_all(coral, area, buf);
                }
            }
            TankBackground::Hell { bg, plants } => {
                render_hell_background(bg, area, buf);
                render_soul_wall(&bg.souls, self.show_names, area, buf);
                for plant in plants {
                    if plant.x >= area.width as i32 {
                        break;
                    }
                    render_seaweed(plant, area, buf);
                }
            }
            TankBackground::Void { bg } => {
                if ritual_active {
                    render_void_background_at(bg, 0, area, buf);
                } else {
                    render_void_background(bg, area, buf);
                }
            }
            TankBackground::Alien { bg } => {
                for star in &bg.stars {
                    render_alien_star(star, area, buf);
                }
                for tentacle in &bg.tentacles {
                    if tentacle.x >= area.width as i32 {
                        break;
                    }
                    render_seaweed(tentacle, area, buf);
                }
                for pyramid in &bg.pyramids {
                    if pyramid.base_x >= area.width as i32 {
                        break;
                    }
                    render_alien_pyramid(pyramid, bg.color.pyramid_color(), area, buf);
                }
            }
            TankBackground::Haunted { bg } => {
                render_gate(&bg.gate, area, buf);
                let bottom_y = area.y as i32 + area.height as i32 - 1;
                for grave in &bg.graves {
                    if grave.base_x >= area.width as i32 {
                        break;
                    }
                    let rows = grave.rows(self.show_epitaphs);
                    render_opaque_grid(&rows, grave.base_x, bottom_y, area, buf);
                }
                for pumpkin in &bg.pumpkins {
                    if pumpkin.base_x >= area.width as i32 {
                        continue;
                    }
                    render_opaque_grid(&pumpkin.rows(), pumpkin.base_x, bottom_y, area, buf);
                }
            }
            TankBackground::Candy { bg, pink_plants } => {
                let bottom_y = area.y as i32 + area.height as i32 - 1;
                for plant in pink_plants {
                    if plant.x >= area.width as i32 {
                        break;
                    }
                    render_seaweed(plant, area, buf);
                }
                for plant in &bg.plants {
                    if plant.x >= area.width as i32 {
                        break;
                    }
                    render_opaque_grid(&plant.rows(), plant.x, bottom_y, area, buf);
                }
                let sway = self.tank.candy_man_sway();
                for deco in &bg.decos {
                    if deco.x >= area.width as i32 {
                        break;
                    }
                    render_opaque_grid(&deco.rows(sway), deco.x, bottom_y, area, buf);
                }
            }
            TankBackground::Desert { bg } => {
                if bg.is_night() {
                    render_desert_stars(bg, area, buf);
                } else {
                    render_desert_sun(bg, area, buf);
                }
                let bottom_y = area.y as i32 + area.height as i32 - 1;
                for cactus in &bg.cacti {
                    if cactus.x >= area.width as i32 {
                        break;
                    }
                    render_opaque_grid(&cactus.grid, cactus.x, bottom_y, area, buf);
                }
                if let Some(tw) = &bg.tumbleweed {
                    render_opaque_grid(
                        &tw.grid(),
                        tw.x as i32,
                        bottom_y - tw.hop_height(),
                        area,
                        buf,
                    );
                }
            }
            TankBackground::Rad { bg } => {
                let bottom_y = area.y as i32 + area.height as i32 - 1;
                render_rad_floor(&bg.floor, bottom_y, area, buf);
                for barrel in &bg.barrels {
                    if barrel.x >= area.width as i32 {
                        break;
                    }
                    render_opaque_grid(&barrel.rows(), barrel.x, bottom_y - 1, area, buf);
                    render_rad_spurts(barrel, bottom_y, area, buf);
                }
            }
            TankBackground::Matrix { bg } => {
                render_matrix_background(bg, area, buf);
            }
            TankBackground::Heaven { bg } => {
                render_heaven_background(bg, self.show_names, area, buf);
            }
        }
        for bubble in &self.tank.bubbles {
            render_bubble(bubble, area, buf);
        }
        if let TankBackground::Haunted { bg } = &self.tank.background {
            for bat in &bg.bats {
                render_bat(bat, area, buf);
            }
            for ghost in &bg.ghosts {
                render_ghost(ghost, area, buf);
            }
        }
        if let TankBackground::Heaven { bg } = &self.tank.background {
            for angel in &bg.angels {
                render_angel(angel, area, buf);
            }
        }
        for cow in &self.tank.cows {
            render_cow(cow, area, buf);
        }
        for food in &self.tank.food {
            render_food(food, area, buf);
        }
        for fish in &self.tank.fish {
            render_fish(fish, area, buf);
        }
        for fish in &self.tank.fish {
            render_fish_surfaces(fish, area, buf);
        }
        if let TankBackground::Coral { floor_algae, .. } = &self.tank.background {
            for fa in floor_algae {
                if fa.base_x >= area.width as i32 {
                    break;
                }
                render_floor_algae(fa, area, buf);
            }
        }
        for ufo in &self.tank.ufos {
            render_ufo(ufo, area, buf);
            if let Some(fish) = ufo.carried_fish() {
                render_fish(fish, area, buf);
            }
        }
        if self.show_names {
            for fish in &self.tank.fish {
                render_fish_name(fish, area, buf);
            }
            for cow in &self.tank.cows {
                render_cow_name(cow, area, buf);
            }
        }
        if self.show_nets {
            for fish in &self.tank.fish {
                render_fish_net(fish, self.show_names, area, buf);
            }
        }
        for fish in &self.tank.fish {
            render_fish_speech(fish, area, buf);
        }
        for cow in &self.tank.cows {
            render_cow_speech(cow, area, buf);
        }

        if ritual_active {
            for y in area.y..area.bottom() {
                for x in area.x..area.right() {
                    buf[(x, y)].set_style(Style::default().add_modifier(Modifier::DIM));
                }
            }
            if let TankBackground::Void { bg } = &self.tank.background {
                render_void_background_at(bg, 0, area, buf);
            }
            if let Some(text) = &self.ritual_text {
                render_ritual_text(text, area, buf);
            }
        }
    }
}

fn art_canvas_w(lines: &[&str]) -> i32 {
    lines
        .iter()
        .map(|l| {
            l.chars()
                .map(|c| UnicodeWidthChar::width(c).unwrap_or(1) as i32)
                .sum::<i32>()
        })
        .max()
        .unwrap_or(0)
}

fn display_w(line: &str) -> i32 {
    line.chars()
        .map(|c| UnicodeWidthChar::width(c).unwrap_or(1) as i32)
        .sum()
}

fn render_seaweed(seaweed: &dyn Seaweed, area: Rect, buf: &mut Buffer) {
    let base_x = seaweed.x();
    for row in 0..seaweed.height() {
        let (x_offset, ch) = seaweed.segment_at(row);
        let x = base_x + x_offset;
        let y = area.height as i32 - 1 - row as i32;
        if y < 0 {
            break;
        }
        if x >= 0 && x < area.width as i32 {
            buf[(area.x + x as u16, area.y + y as u16)]
                .set_char(ch)
                .set_fg(seaweed.color_at(row));
        }
    }
}

fn coral_painted_span(chars: &[(char, i32)]) -> Option<(i32, i32)> {
    let mut col = 0i32;
    let mut first = None;
    let mut last = None;
    for &(ch, w) in chars {
        if ch != ' ' && ch != 'X' {
            first.get_or_insert(col);
            last = Some(col);
        }
        col += w;
    }
    first.zip(last)
}

fn render_coral_line(
    line: &str,
    mirrored: bool,
    start_x: i32,
    canvas_w: i32,
    screen_y: i32,
    area: Rect,
    buf: &mut Buffer,
) {
    let style = Style::new().fg(CORAL_COLOR);
    if mirrored {
        let line_w = display_w(line);
        let effective_x = start_x + canvas_w - line_w;
        let pairs: Vec<(char, i32)> = line
            .chars()
            .map(|c| {
                (
                    mirror_char(c),
                    UnicodeWidthChar::width(c).unwrap_or(1) as i32,
                )
            })
            .rev()
            .collect();
        let span = coral_painted_span(&pairs);
        let mut col = 0i32;
        for &(ch, w) in &pairs {
            let sx = effective_x + col;
            if sx >= area.x as i32 && sx < area.right() as i32 {
                if ch != ' ' && ch != 'X' {
                    buf[(sx as u16, screen_y as u16)]
                        .set_char(ch)
                        .set_style(style);
                } else if span.is_some_and(|(lo, hi)| col >= lo && col <= hi) {
                    buf[(sx as u16, screen_y as u16)]
                        .set_char(' ')
                        .set_style(Style::reset());
                }
            }
            col += w;
        }
    } else {
        let chars: Vec<(char, i32)> = line
            .chars()
            .map(|c| (c, UnicodeWidthChar::width(c).unwrap_or(1) as i32))
            .collect();
        let span = coral_painted_span(&chars);
        let mut col = 0i32;
        for &(ch, w) in &chars {
            let sx = start_x + col;
            if sx >= area.x as i32 && sx < area.right() as i32 {
                if ch != ' ' && ch != 'X' {
                    buf[(sx as u16, screen_y as u16)]
                        .set_char(ch)
                        .set_style(style);
                } else if span.is_some_and(|(lo, hi)| col >= lo && col <= hi) {
                    buf[(sx as u16, screen_y as u16)]
                        .set_char(' ')
                        .set_style(Style::reset());
                }
            }
            col += w;
        }
    }
}

fn render_coral_structure(coral: &CoralStructure, area: Rect, buf: &mut Buffer) {
    let canvas_w = art_canvas_w(CORAL_A_LINES);
    for (row_idx, line) in CORAL_A_LINES.iter().enumerate() {
        let screen_y = area.y as i32 + area.height as i32 - CORAL_A_ROWS as i32 + row_idx as i32;
        if screen_y < area.y as i32 || screen_y >= area.bottom() as i32 {
            continue;
        }
        let start_x = area.x as i32 + coral.base_x;
        render_coral_line(line, coral.mirrored, start_x, canvas_w, screen_y, area, buf);
    }
}

#[derive(Clone, Copy)]
struct LinePlacement {
    start_x: i32,
    screen_y: i32,
    canvas_w: i32,
}

fn render_algae_line(
    line: &str,
    li: usize,
    instance: &CoralAlgaeInstance,
    placement: LinePlacement,
    area: Rect,
    buf: &mut Buffer,
) {
    let LinePlacement {
        start_x,
        screen_y,
        canvas_w,
    } = placement;
    let style = Style::new()
        .fg(instance.color)
        .remove_modifier(Modifier::all());
    if instance.mirrored {
        let line_w = display_w(line);
        let effective_x = start_x + canvas_w - line_w;
        let pairs: Vec<(char, i32)> = {
            let mut col = 0usize;
            let mut v: Vec<(char, i32)> = Vec::new();
            for ch in line.chars() {
                let w = UnicodeWidthChar::width(ch).unwrap_or(1);
                let ac = query_anim_char(&instance.anim, li, col, ch);
                v.push((ac, w as i32));
                col += w;
            }
            v.reverse();
            for (c, _) in &mut v {
                *c = mirror_char(*c);
            }
            v
        };
        let mut col = 0i32;
        for (ch, w) in pairs {
            if ch != ' ' {
                let sx = effective_x + col;
                if sx >= area.x as i32 && sx < area.right() as i32 {
                    buf[(sx as u16, screen_y as u16)]
                        .set_char(ch)
                        .set_style(style);
                }
            }
            col += w;
        }
    } else {
        let mut col = 0i32;
        for ch in line.chars() {
            let w = UnicodeWidthChar::width(ch).unwrap_or(1) as i32;
            let ac = query_anim_char(&instance.anim, li, col as usize, ch);
            if ac != ' ' {
                let sx = start_x + col;
                if sx >= area.x as i32 && sx < area.right() as i32 {
                    buf[(sx as u16, screen_y as u16)]
                        .set_char(ac)
                        .set_style(style);
                }
            }
            col += w;
        }
    }
}

fn render_coral_algae_all(coral: &CoralStructure, area: Rect, buf: &mut Buffer) {
    for instance in &coral.algae {
        let anchor_screen_y =
            area.y as i32 + area.height as i32 - CORAL_A_ROWS as i32 + instance.art_row as i32;
        let art = algae_art(instance.algae_idx);
        let n = art.len();
        let bottom_w: i32 = art[n - 1]
            .chars()
            .map(|c| UnicodeWidthChar::width(c).unwrap_or(1) as i32)
            .sum();
        let canvas_w = art_canvas_w(art);
        let anchor_screen_x = if instance.mirrored {
            area.x as i32 + coral.base_x + instance.art_col as i32 - canvas_w + bottom_w / 2
        } else {
            area.x as i32 + coral.base_x + instance.art_col as i32 - bottom_w / 2
        };
        for (li, line) in art.iter().enumerate() {
            let screen_y = anchor_screen_y - (n - 1 - li) as i32;
            if screen_y < area.y as i32 || screen_y >= area.bottom() as i32 {
                continue;
            }
            let shift = algae_row_shift(&instance.anim, li);
            render_algae_line(
                line,
                li,
                instance,
                LinePlacement {
                    start_x: anchor_screen_x + shift,
                    screen_y,
                    canvas_w,
                },
                area,
                buf,
            );
        }
    }
}

fn render_floor_algae(fa: &FloorAlgae, area: Rect, buf: &mut Buffer) {
    let n = FLOOR_ALGAE_A.len();
    let canvas_w = art_canvas_w(FLOOR_ALGAE_A);
    let style = Style::new()
        .fg(FLOOR_ALGAE_COLOR)
        .remove_modifier(Modifier::all());
    for (li, line) in FLOOR_ALGAE_A.iter().enumerate() {
        let screen_y = area.y as i32 + area.height as i32 - n as i32 + li as i32;
        if screen_y < area.y as i32 || screen_y >= area.bottom() as i32 {
            continue;
        }
        let start_x = area.x as i32 + fa.base_x;
        if fa.mirrored {
            let line_w = display_w(line);
            let effective_x = start_x + canvas_w - line_w;
            let pairs: Vec<(char, i32)> = line
                .chars()
                .map(|c| {
                    let ac = vert_wave_char(c, li, fa.phase);
                    (
                        mirror_char(ac),
                        UnicodeWidthChar::width(c).unwrap_or(1) as i32,
                    )
                })
                .rev()
                .collect();
            let mut col = 0i32;
            for (ch, w) in pairs {
                if ch != ' ' {
                    let sx = effective_x + col;
                    if sx >= area.x as i32 && sx < area.right() as i32 {
                        buf[(sx as u16, screen_y as u16)]
                            .set_char(ch)
                            .set_style(style);
                    }
                }
                col += w;
            }
        } else {
            let mut col = 0i32;
            for ch in line.chars() {
                let w = UnicodeWidthChar::width(ch).unwrap_or(1) as i32;
                let ac = vert_wave_char(ch, li, fa.phase);
                if ac != ' ' {
                    let sx = start_x + col;
                    if sx >= area.x as i32 && sx < area.right() as i32 {
                        buf[(sx as u16, screen_y as u16)]
                            .set_char(ac)
                            .set_style(style);
                    }
                }
                col += w;
            }
        }
    }
}

fn render_rad_floor(floor: &[FluidChar], bottom_y: i32, area: Rect, buf: &mut Buffer) {
    if bottom_y < area.y as i32 || bottom_y >= area.bottom() as i32 {
        return;
    }
    let style = Style::new()
        .fg(LIGHT_GREEN)
        .remove_modifier(Modifier::all());
    for (col, cell) in floor.iter().enumerate() {
        let sx = area.x as i32 + col as i32;
        if sx >= area.right() as i32 {
            break;
        }
        buf[(sx as u16, bottom_y as u16)]
            .set_char(cell.ch)
            .set_style(style);
    }
}

fn render_rad_spurts(barrel: &RadBarrel, bottom_y: i32, area: Rect, buf: &mut Buffer) {
    let top_y = bottom_y - barrel.height();
    let style = Style::new()
        .fg(FLUID_COLOR)
        .remove_modifier(Modifier::all());
    for cell in barrel.spurt_cells() {
        let sx = area.x as i32 + barrel.x + cell.col;
        let sy = top_y + cell.row;
        if sx < area.x as i32 || sx >= area.right() as i32 {
            continue;
        }
        if sy < area.y as i32 || sy >= area.bottom() as i32 {
            continue;
        }
        buf[(sx as u16, sy as u16)]
            .set_char(cell.ch)
            .set_style(style);
    }
}

fn render_bubble(bubble: &Bubble, area: Rect, buf: &mut Buffer) {
    let x = area.x + bubble.position.x as u16;
    let y = area.y + bubble.position.y as u16;
    if x < area.right() && y < area.bottom() {
        buf[(x, y)]
            .set_char(bubble.bubble_char)
            .set_style(Style::new().fg(bubble.color).add_modifier(Modifier::DIM));
    }
}

fn render_food(food: &Food, area: Rect, buf: &mut Buffer) {
    let x = area.x + food.position.x as u16;
    let y = area.y + food.position.y as u16;
    if x < area.right() && y < area.bottom() {
        let color = if food.is_candy { PINK } else { YELLOW };
        buf[(x, y)]
            .set_char(food.food_char)
            .set_style(Style::new().fg(color).remove_modifier(Modifier::all()));
    }
}

fn render_fish_name(fish: &Fish, area: Rect, buf: &mut Buffer) {
    render_fish_label(fish, WHITE, area, buf);
}

fn render_fish_label(fish: &Fish, color: Color, area: Rect, buf: &mut Buffer) {
    if fish.is_invisible() {
        return;
    }
    if let Some(ref us) = fish.unfish_state
        && us.kind == UnfishKind::Worm
    {
        render_worm_name_portal(fish, color, area, buf);
        return;
    }

    let name_y = area.y as i32 + fish.position.y as i32 - sprite_rows_above_body(fish) - 1;
    render_label(&fish.name, fish_center(fish), name_y, color, area, buf);
}

fn sprite_rows_above_body(fish: &Fish) -> i32 {
    if let Some(kind) = fish.unfish_body()
        && is_multi_row(kind)
    {
        return unfish_center_row(kind);
    }
    fish.line_sprite().body_row as i32
}

fn fish_center(fish: &Fish) -> i32 {
    fish.position.x as i32 + fish.display_width as i32 / 2
}

fn render_label(text: &str, center_x: i32, y: i32, color: Color, area: Rect, buf: &mut Buffer) {
    let start = center_x - text.chars().count() as i32 / 2;
    render_text(text, start, y, color, area, buf);
}

fn render_text(text: &str, start: i32, y: i32, color: Color, area: Rect, buf: &mut Buffer) {
    if y < area.y as i32 || y >= area.bottom() as i32 {
        return;
    }
    for (i, ch) in text.chars().enumerate() {
        let x = start + i as i32;
        if x < 0 {
            continue;
        }
        let abs_x = area.x + x as u16;
        if abs_x >= area.right() {
            break;
        }
        overdraw(buf, abs_x, y as u16)
            .set_char(ch)
            .set_style(Style::new().fg(color).remove_modifier(Modifier::all()));
    }
}

fn rows_above_body(fish: &Fish, names_shown: bool) -> i32 {
    sprite_rows_above_body(fish) + i32::from(names_shown)
}

fn render_fish_net(fish: &Fish, names_shown: bool, area: Rect, buf: &mut Buffer) {
    if fish.is_invisible() {
        return;
    }
    let Some(net) = fish.script().and_then(BotfishState::net_label) else {
        return;
    };
    let net_y = area.y as i32 + fish.position.y as i32 - rows_above_body(fish, names_shown) - 1;
    render_label(&net, fish_center(fish), net_y, NET_COLOR, area, buf);
}

fn render_fish_speech(fish: &Fish, area: Rect, buf: &mut Buffer) {
    if fish.is_invisible() {
        return;
    }
    let bubbles = fish_bubbles(fish);
    if bubbles.is_empty() {
        return;
    }
    let height = bubbles.iter().map(|(bubble, _)| bubble.len() as i32).sum();
    let tail = fish_tail(fish, height, area);
    let mut edge = tail.bubble_top(0);
    for (bubble, ink) in &bubbles {
        let rows = bubble.len() as i32;
        if tail.side == Side::Above {
            edge -= rows;
        }
        render_fish_bubble(&tail, bubble, *ink, edge, area, buf);
        if tail.side == Side::Below {
            edge += rows;
        }
    }
    for (x, y) in tail.cells() {
        put_cell(buf, area, x, y, tail.glyph(), WHITE);
    }
}

fn fish_tail(fish: &Fish, height: i32, area: Rect) -> Tail {
    let eye = (
        area.x as i32 + fish.eye_x(),
        area.y as i32 + fish.position.y as i32,
    );
    let off_eye = |side| Tail::off_eye(eye, fish.facing_left(), side);
    let (top, bottom) = (area.y as i32, area.bottom() as i32);
    let side = Side::of(
        height,
        off_eye(Side::Above).room(top, bottom),
        off_eye(Side::Below).room(top, bottom),
    );
    off_eye(side)
}

fn bubble_width(bubble: &[String]) -> i32 {
    bubble
        .iter()
        .map(|line| line.chars().count() as i32)
        .max()
        .unwrap_or(0)
}

fn bubble_left(tail: &Tail, width: i32, area: Rect) -> i32 {
    shift_into(
        tail.bubble_left(width) - area.x as i32,
        width,
        area.width as i32,
    )
}

fn fish_bubbles(fish: &Fish) -> Vec<(Vec<String>, Color)> {
    let panels = fish_displays(fish)
        .into_iter()
        .filter_map(|display| match display {
            Display::Bubble(panel) => Some((build_bubble(&panel), WHITE)),
            Display::Body(_) => None,
        });
    let ink = speech_ink(fish.is_alienated(), WHITE);
    let speech = fish
        .speech
        .as_ref()
        .map(|speech| (build_speech_bubble(&speech.text), ink));
    panels.chain(speech).collect()
}

fn fish_displays(fish: &Fish) -> Vec<Display> {
    fish.script()
        .map(BotfishState::displays)
        .unwrap_or_default()
}

fn render_fish_surfaces(fish: &Fish, area: Rect, buf: &mut Buffer) {
    if fish.is_invisible() {
        return;
    }
    let mut y = area.y as i32 + fish.position.y as i32;
    for display in fish_displays(fish) {
        let Display::Body(rows) = display else {
            continue;
        };
        for row in &rows {
            let start = surface_x(fish, row.chars().count() as i32);
            render_text(row, start, y, WHITE, area, buf);
            y += 1;
        }
    }
}

fn surface_x(fish: &Fish, cells: i32) -> i32 {
    let x = fish.position.x as i32;
    let last = fish.display_width.saturating_sub(1);
    let (lo, hi) = fish.body_span().unwrap_or((0, last));
    if fish.facing_left() {
        return x + lo as i32;
    }
    x + hi as i32 + 1 - cells
}

fn render_fish_bubble(
    tail: &Tail,
    bubble: &[String],
    ink: Color,
    top: i32,
    area: Rect,
    buf: &mut Buffer,
) {
    let left = area.x as i32 + bubble_left(tail, bubble_width(bubble), area);
    draw_bubble(bubble, (left, top), WHITE, ink, area, buf);
}

fn draw_bubble(
    bubble: &[String],
    (left, top): (i32, i32),
    frame: Color,
    ink: Color,
    area: Rect,
    buf: &mut Buffer,
) {
    for (row, line) in bubble.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            let color = if is_bubble_text(bubble, row, col) {
                ink
            } else {
                frame
            };
            put_cell(buf, area, left + col as i32, top + row as i32, ch, color);
        }
    }
}

fn render_fish(fish: &Fish, area: Rect, buf: &mut Buffer) {
    if fish.is_invisible() {
        return;
    }
    if let Some(kind) = fish.unfish_body() {
        if is_multi_row(kind) {
            render_multi_row_unfish(fish, area, buf);
            return;
        }
        if kind == UnfishKind::Worm {
            render_worm_portal(fish, area, buf);
            return;
        }
    }

    render_line_sprite(fish, area, buf);
}

fn render_line_sprite(fish: &Fish, area: Rect, buf: &mut Buffer) {
    let sprite = fish.line_sprite();
    let body_screen_y = area.y as i32 + fish.position.y as i32;
    let base_y = body_screen_y - sprite.body_row as i32;
    let x_base = area.x as i32 + fish.position.x as i32;
    for (row_idx, row) in sprite.rows.iter().enumerate() {
        let sy = base_y + row_idx as i32;
        if sy < area.y as i32 || sy >= area.bottom() as i32 {
            continue;
        }
        let mut col = 0i32;
        for &(ch, color) in row {
            let w = UnicodeWidthChar::width(ch).unwrap_or(1) as i32;
            if ch == TRANSPARENT {
                col += w;
                continue;
            }
            let sx = x_base + col;
            if sx >= area.right() as i32 {
                break;
            }
            if sx >= area.x as i32 {
                buf[(sx as u16, sy as u16)]
                    .set_char(ch)
                    .set_style(Style::new().fg(color).remove_modifier(Modifier::all()));
            }
            col += w;
        }
    }
}

fn unfish_center_row(kind: UnfishKind) -> i32 {
    match kind {
        UnfishKind::Skull => SKULL_CENTER_ROW,
        _ => BALL_CENTER_ROW,
    }
}

fn interior_col_range(line: &str) -> Option<(usize, usize)> {
    let first = line
        .chars()
        .enumerate()
        .find(|(_, c)| *c != ' ')
        .map(|(i, _)| i)?;
    let last = line
        .chars()
        .rev()
        .position(|c| c != ' ')
        .map(|rev_pos| line.chars().count() - 1 - rev_pos)?;
    if last <= first + 1 {
        return None;
    }
    Some((first + 1, last - 1))
}

const SKULL_WING_WIDTH: i32 = 2;

fn ear_border_cols(line: &str, width: usize) -> (i32, i32) {
    let chars: Vec<char> = line.chars().collect();
    let left_wing = chars.first() == Some(&'}');
    let right_wing = chars.last() == Some(&'{');
    let first = chars.iter().position(|&c| c != ' ').unwrap_or(0) as i32;
    let last = chars
        .iter()
        .rposition(|&c| c != ' ')
        .map(|p| p as i32)
        .unwrap_or(0);
    let left = if left_wing {
        SKULL_WING_WIDTH
    } else {
        first - 1
    };
    let right = if right_wing {
        width as i32 - 1 - SKULL_WING_WIDTH
    } else {
        last + 1
    };
    (left, right)
}

#[derive(Clone, Copy)]
struct SpriteStyle<'a> {
    eye_overrides: &'a [(usize, char, Color)],
    body_color: Color,
    interior_fg: Option<Color>,
    glisten_colors: &'a [Color],
    color_patches: &'a [(usize, Color)],
}

fn render_sprite_row(
    line: &str,
    base_x: i32,
    screen_y: i32,
    area: Rect,
    buf: &mut Buffer,
    style: &SpriteStyle,
) {
    let SpriteStyle {
        eye_overrides,
        body_color,
        interior_fg,
        glisten_colors,
        color_patches,
    } = *style;
    let sy = screen_y as u16;
    let interior = interior_col_range(line);
    for (char_idx, ch) in line.chars().enumerate() {
        let override_entry = eye_overrides.iter().find(|(col, _, _)| *col == char_idx);
        if ch == ' ' && override_entry.is_none() {
            if let Some((inner_start, inner_end)) = interior
                && char_idx >= inner_start
                && char_idx <= inner_end
            {
                let sx = base_x + char_idx as i32;
                if sx >= area.x as i32 && sx < area.right() as i32 {
                    let style = match interior_fg {
                        Some(c) => Style::default().fg(c).add_modifier(Modifier::DIM),
                        None => Style::default().add_modifier(Modifier::DIM),
                    };
                    buf[(sx as u16, sy)].set_style(style);
                }
            }
            continue;
        }
        let sx = base_x + char_idx as i32;
        if sx < area.x as i32 || sx >= area.right() as i32 {
            continue;
        }
        let (draw_ch, color) = override_entry
            .map(|&(_, ec, c)| (ec, c))
            .unwrap_or_else(|| {
                let c = glisten_colors
                    .get(char_idx)
                    .copied()
                    .or_else(|| {
                        color_patches
                            .iter()
                            .rev()
                            .find(|&&(pos, _)| pos == char_idx)
                            .map(|&(_, c)| c)
                    })
                    .unwrap_or(body_color);
                (ch, c)
            });
        buf[(sx as u16, sy)].set_char(draw_ch).set_fg(color);
    }
}

pub(crate) fn render_multi_row_unfish_at(
    fish: &Fish,
    base_x: i32,
    base_y: i32,
    area: Rect,
    buf: &mut Buffer,
) {
    let unfish_state = match fish.unfish_state.as_ref() {
        Some(s) => s,
        None => return,
    };
    let eye_ch = if unfish_state.eye.is_open { '0' } else { '-' };

    match unfish_state.kind {
        UnfishKind::Ball | UnfishKind::Skull => {
            let body_color = unfish_state.slime_body_color.unwrap_or(UNFISH_BODY_COLOR);
            let interior_fg = unfish_state
                .slime_body_color
                .filter(|&c| c != UNFISH_BODY_COLOR);
            let sprite_w = if unfish_state.kind == UnfishKind::Ball {
                BALL_WIDTH as usize
            } else {
                SKULL_WIDTH as usize
            };
            let glisten_colors: Vec<Color> = if unfish_state.slime_glisten_enabled {
                let (base, mid, peak_default) = derive_glistening_palette(body_color);
                let peak = unfish_state.slime_glisten_color.unwrap_or(peak_default);
                (0..sprite_w)
                    .map(|i| {
                        color_for_glisten(
                            unfish_state.slime_glisten_mode,
                            unfish_state.slime_glisten_phase,
                            i,
                            sprite_w,
                            base,
                            mid,
                            peak,
                        )
                    })
                    .collect()
            } else {
                vec![]
            };
            let lines: &[&str] = if unfish_state.kind == UnfishKind::Ball {
                &BALL_BASE
            } else if unfish_state.wings.is_open {
                &SKULL_OPEN
            } else {
                &SKULL_CLOSED
            };
            for (row_idx, line) in lines.iter().enumerate() {
                let screen_y = base_y + row_idx as i32;
                if screen_y < area.y as i32 || screen_y >= area.bottom() as i32 {
                    continue;
                }
                let mut eye_overrides: Vec<(usize, char, Color)> = Vec::new();
                let eye_color = unfish_state.slime_eye_color.unwrap_or(UNFISH_EYE_COLOR);
                if unfish_state.kind == UnfishKind::Ball
                    && unfish_state.ball_has_center_eye
                    && row_idx == BALL_EYE_ROW
                {
                    eye_overrides.push((BALL_EYE_COL - 1, '(', eye_color));
                    eye_overrides.push((BALL_EYE_COL, eye_ch, eye_color));
                    eye_overrides.push((BALL_EYE_COL + 1, ')', eye_color));
                }
                for eye in &unfish_state.floating_eyes {
                    if eye.row != row_idx {
                        continue;
                    }
                    let col = eye.col.round() as i32;
                    let ec = if eye.blink.is_open { '0' } else { '-' };
                    let ec_color = eye.color.unwrap_or(eye_color);
                    if col >= 1 {
                        eye_overrides.push(((col - 1) as usize, '(', ec_color));
                    }
                    if col >= 0 {
                        eye_overrides.push((col as usize, ec, ec_color));
                    }
                    if col + 1 < line.len() as i32 {
                        eye_overrides.push(((col + 1) as usize, ')', ec_color));
                    }
                }
                render_sprite_row(
                    line,
                    base_x,
                    screen_y,
                    area,
                    buf,
                    &SpriteStyle {
                        eye_overrides: &eye_overrides,
                        body_color,
                        interior_fg,
                        glisten_colors: &glisten_colors,
                        color_patches: &unfish_state.slime_color_patches,
                    },
                );
                if row_idx < unfish_state.ear_count {
                    let ear_color = unfish_state.ear_color.unwrap_or(PINK);
                    let (left_col, right_col) = ear_border_cols(line, sprite_w);
                    for (col, glyph) in [(left_col, EAR_LEFT), (right_col, EAR_RIGHT)] {
                        let sx = base_x + col;
                        if sx >= area.x as i32 && sx < area.right() as i32 {
                            buf[(sx as u16, screen_y as u16)]
                                .set_char(glyph)
                                .set_fg(ear_color);
                        }
                    }
                }
            }
            let coat = Coat {
                body_color,
                glisten_colors: &glisten_colors,
                color_patches: &unfish_state.slime_color_patches,
            };
            if let Some(feet) = unfish_state.feet {
                let lowest = lines.last().copied().unwrap_or("");
                let feet_y = base_y + lines.len() as i32;
                render_feet_row(lowest, &coat, feet, (base_x, feet_y), area, buf);
            }
            render_extension_bands(fish, lines, &coat, (base_x, base_y), area, buf);
        }
        _ => {}
    }
}

struct Coat<'a> {
    body_color: Color,
    glisten_colors: &'a [Color],
    color_patches: &'a [(usize, Color)],
}

impl Coat<'_> {
    fn cells(&self, body_line: &str) -> Vec<Cell> {
        body_line
            .chars()
            .enumerate()
            .map(|(col, ch)| {
                let color = self
                    .glisten_colors
                    .get(col)
                    .copied()
                    .or_else(|| {
                        self.color_patches
                            .iter()
                            .rev()
                            .find(|&&(pos, _)| pos == col)
                            .map(|&(_, c)| c)
                    })
                    .unwrap_or(self.body_color);
                (ch, color)
            })
            .collect()
    }
}

fn draw_appendage_row(row: &[Cell], base_x: i32, screen_y: i32, area: Rect, buf: &mut Buffer) {
    if screen_y < area.y as i32 || screen_y >= area.bottom() as i32 {
        return;
    }
    for (col, &(ch, color)) in row.iter().enumerate() {
        if ch == TRANSPARENT {
            continue;
        }
        let sx = base_x + col as i32;
        if sx >= area.x as i32 && sx < area.right() as i32 {
            buf[(sx as u16, screen_y as u16)].set_char(ch).set_fg(color);
        }
    }
}

fn render_feet_row(
    body_line: &str,
    coat: &Coat,
    feet: Feet,
    (base_x, screen_y): (i32, i32),
    area: Rect,
    buf: &mut Buffer,
) {
    let body_cells = coat.cells(body_line);
    let Some(span) = painted_span(&body_cells) else {
        return;
    };
    draw_appendage_row(
        &feet_row(&body_cells, span, feet),
        base_x,
        screen_y,
        area,
        buf,
    );
}

const MULTI_ROW_MAX_TENTACLES: usize = 2;

fn render_extension_bands(
    fish: &Fish,
    lines: &[&str],
    coat: &Coat,
    (base_x, base_y): (i32, i32),
    area: Rect,
    buf: &mut Buffer,
) {
    let Some(ext) = fish.body_extension() else {
        return;
    };
    let posed = PosedExtension {
        ext,
        facing_left: fish.facing_left(),
        phase: fish.sway.phase,
        max_tentacles: Some(MULTI_ROW_MAX_TENTACLES),
    };
    let top_cells = coat.cells(lines[0]);
    let bottom_line = lines.last().copied().unwrap_or("");
    let bottom_cells = coat.cells(bottom_line);
    let top_span = painted_span(&top_cells);
    let bottom_span = painted_span(&bottom_cells);
    for depth in 0..ext.length {
        if let Some(span) = top_span {
            let top_row = posed.row(&top_cells, span, depth, true);
            draw_appendage_row(&top_row, base_x, base_y - 1 - depth as i32, area, buf);
        }
        if let Some(span) = bottom_span {
            let bottom_row = posed.row(&bottom_cells, span, depth, false);
            draw_appendage_row(
                &bottom_row,
                base_x,
                base_y + lines.len() as i32 + depth as i32,
                area,
                buf,
            );
        }
    }
}

fn render_multi_row_unfish(fish: &Fish, area: Rect, buf: &mut Buffer) {
    let unfish_state = match fish.unfish_state.as_ref() {
        Some(s) => s,
        None => return,
    };
    let base_x = area.x as i32 + fish.position.x as i32;
    let center_row = unfish_center_row(unfish_state.kind);
    let base_y = area.y as i32 + fish.position.y as i32 - center_row;
    render_multi_row_unfish_at(fish, base_x, base_y, area, buf);
}

fn render_worm_portal(fish: &Fish, area: Rect, buf: &mut Buffer) {
    let segs = fish.fused_worm_cells().unwrap_or_else(|| fish.segments());
    let tank_h = area.height as i32;
    let tank_w = area.width as i32;
    let y_wrapped = (fish.position.y as i32).rem_euclid(tank_h);
    let sy = (area.y as i32 + y_wrapped) as u16;
    if sy < area.y || sy >= area.bottom() {
        return;
    }
    for (i, (ch, color)) in segs.iter().enumerate() {
        let tank_x = (fish.position.x as i32 + i as i32).rem_euclid(tank_w);
        let screen_x = area.x as i32 + tank_x;
        buf[(screen_x as u16, sy)]
            .set_char(*ch)
            .set_style(Style::new().fg(*color).remove_modifier(Modifier::all()));
    }
    let origin_x = fish.position.x as i32;
    let body_y = fish.position.y as i32;
    let Some(span) = painted_span(&segs) else {
        return;
    };
    if let Some(ext) = fish.body_extension() {
        let posed = PosedExtension {
            ext,
            facing_left: fish.facing_left(),
            phase: fish.sway.phase,
            max_tentacles: None,
        };
        let (above, below) = line_extension_bands(ext.variant);
        if above {
            for depth in 0..ext.length {
                let row = posed.row(&segs, span, depth, true);
                draw_portal_row(&row, origin_x, body_y - 1 - depth as i32, area, buf);
            }
        }
        if below {
            for depth in 0..ext.length {
                let row = posed.row(&segs, span, depth, false);
                draw_portal_row(&row, origin_x, body_y + 1 + depth as i32, area, buf);
            }
        }
        return;
    }
    if let Some(feet) = fish.feet() {
        draw_portal_row(
            &feet_row(&segs, span, feet),
            origin_x,
            body_y + 1,
            area,
            buf,
        );
    }
}

fn draw_portal_row(row: &[Cell], origin_x: i32, unwrapped_y: i32, area: Rect, buf: &mut Buffer) {
    let tank_h = area.height as i32;
    let tank_w = area.width as i32;
    let y_wrapped = unwrapped_y.rem_euclid(tank_h);
    let sy = (area.y as i32 + y_wrapped) as u16;
    if sy < area.y || sy >= area.bottom() {
        return;
    }
    for (i, &(ch, color)) in row.iter().enumerate() {
        if ch == TRANSPARENT {
            continue;
        }
        let tank_x = (origin_x + i as i32).rem_euclid(tank_w);
        let screen_x = area.x as i32 + tank_x;
        buf[(screen_x as u16, sy)]
            .set_char(ch)
            .set_style(Style::new().fg(color).remove_modifier(Modifier::all()));
    }
}

fn render_worm_name_portal(fish: &Fish, color: Color, area: Rect, buf: &mut Buffer) {
    let tank_h = area.height as i32;
    let tank_w = area.width as i32;
    let name_y_wrapped = (fish.position.y as i32 - 1).rem_euclid(tank_h);
    let name_sy = (area.y as i32 + name_y_wrapped) as u16;
    if name_sy < area.y || name_sy >= area.bottom() {
        return;
    }
    let fish_center = fish.position.x as i32 + fish.display_width as i32 / 2;
    let name_start = fish_center - fish.name.len() as i32 / 2;
    for (i, ch) in fish.name.chars().enumerate() {
        let tank_x = (name_start + i as i32).rem_euclid(tank_w);
        let screen_x = area.x as i32 + tank_x;
        buf[(screen_x as u16, name_sy)]
            .set_char(ch)
            .set_style(Style::new().fg(color).remove_modifier(Modifier::all()));
    }
}

fn render_hell_background(bg: &HellBackground, area: Rect, buf: &mut Buffer) {
    if bg.is_random {
        let w = area.width as usize;
        for y in 0..area.height {
            for x in 0..area.width {
                let idx = y as usize * w + x as usize;
                if idx < bg.random_buffer.len() {
                    let ch = bg.random_buffer[idx];
                    if ch != ' ' {
                        buf[(area.x + x, area.y + y)]
                            .set_char(ch)
                            .set_fg(RANDOM_FACE_COLOR);
                    }
                }
            }
        }
        return;
    }

    let face_grid = &bg.face_grid;
    let face_h = face_grid.len();
    if face_h == 0 {
        return;
    }
    let face_w = face_grid[0].len();
    if face_w == 0 {
        return;
    }

    for y in 0..area.height {
        let world_y = bg.offset_y + y as f32;
        let tile_row_f = (world_y / face_h as f32).floor();
        let face_row = world_y.rem_euclid(face_h as f32) as usize;
        let face_row = face_row.min(face_h - 1);
        let beehive_shift = if (tile_row_f as i64).rem_euclid(2) == 1 {
            face_w / 2
        } else {
            0
        };

        let h_wave_shift: i32 =
            (H_WAVE_AMPLITUDE * (bg.h_phase - y as f32 * H_WAVE_ROW_SPREAD).sin()).round() as i32;

        for x in 0..area.width {
            let world_x = bg.offset_x + x as f32 + h_wave_shift as f32;
            let face_col = (world_x + beehive_shift as f32).rem_euclid(face_w as f32) as usize;
            let face_col = face_col.min(face_w - 1);
            let ch = face_grid[face_row][face_col];
            if ch != ' ' {
                buf[(area.x + x, area.y + y)]
                    .set_char(ch)
                    .set_fg(FACE_COLOR);
            }
        }
    }
}

fn render_void_background(bg: &VoidBackground, area: Rect, buf: &mut Buffer) {
    render_void_background_at(bg, bg.current_frame, area, buf);
}

fn render_matrix_background(bg: &MatrixBackground, area: Rect, buf: &mut Buffer) {
    for col in &bg.columns {
        let ax = area.x as i32 + col.x;
        if ax < area.x as i32 || ax >= area.right() as i32 {
            continue;
        }
        for (dist, &ch) in col.chars.iter().enumerate() {
            let y = col.head_row - dist as i32;
            if y < 0 || y >= area.height as i32 {
                continue;
            }
            buf[(ax as u16, area.y + y as u16)].set_char(ch).set_style(
                Style::new()
                    .fg(trail_color(dist))
                    .remove_modifier(Modifier::all()),
            );
        }
    }
}

fn render_void_background_at(bg: &VoidBackground, frame_idx: usize, area: Rect, buf: &mut Buffer) {
    if bg.frames.is_empty() {
        return;
    }
    let frame = &bg.frames[frame_idx % bg.frames.len()];
    let frame_h = frame.len() as i32;

    let start_x = area.width as i32 / 2 - VOID_EYE_CENTER_X;
    let target_eye_y = area.height as i32 / 2 + VOID_EYE_VERTICAL_OFFSET;
    let start_y = target_eye_y - frame_h / 2;

    let s = Style::new().fg(WHITE).remove_modifier(Modifier::all());

    for (row_i, line) in frame.iter().enumerate() {
        let screen_y = area.y as i32 + start_y + row_i as i32;
        if screen_y < area.y as i32 || screen_y >= area.bottom() as i32 {
            continue;
        }
        for (col_i, ch) in line.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let screen_x = area.x as i32 + start_x + col_i as i32;
            if screen_x < area.x as i32 || screen_x >= area.right() as i32 {
                continue;
            }
            buf[(screen_x as u16, screen_y as u16)]
                .set_char(ch)
                .set_style(s);
        }
    }
}

fn render_alien_pyramid_line(
    line: &str,
    mirrored: bool,
    placement: LinePlacement,
    eye_char: Option<char>,
    color: ratatui::style::Color,
    area: Rect,
    buf: &mut Buffer,
) {
    let LinePlacement {
        start_x,
        screen_y,
        canvas_w,
    } = placement;
    let style = Style::new().fg(color);
    if mirrored {
        let line_w = display_w(line);
        let effective_x = start_x + canvas_w - line_w;
        let pairs: Vec<(char, i32)> = line
            .chars()
            .map(|c| {
                (
                    mirror_char(c),
                    UnicodeWidthChar::width(c).unwrap_or(1) as i32,
                )
            })
            .rev()
            .collect();
        let mut col = 0i32;
        for (ch, w) in pairs {
            let draw_ch = if ch == '0' {
                eye_char.unwrap_or(ch)
            } else {
                ch
            };
            if draw_ch != ' ' {
                let sx = effective_x + col;
                if sx >= area.x as i32 && sx < area.right() as i32 {
                    buf[(sx as u16, screen_y as u16)]
                        .set_char(draw_ch)
                        .set_style(style);
                }
            }
            col += w;
        }
    } else {
        let mut col = 0i32;
        for ch in line.chars() {
            let w = UnicodeWidthChar::width(ch).unwrap_or(1) as i32;
            let draw_ch = if ch == '0' {
                eye_char.unwrap_or(ch)
            } else {
                ch
            };
            if draw_ch != ' ' {
                let sx = start_x + col;
                if sx >= area.x as i32 && sx < area.right() as i32 {
                    buf[(sx as u16, screen_y as u16)]
                        .set_char(draw_ch)
                        .set_style(style);
                }
            }
            col += w;
        }
    }
}

fn render_alien_pyramid(
    pyramid: &AlienPyramid,
    color: ratatui::style::Color,
    area: Rect,
    buf: &mut Buffer,
) {
    let lines = pyramid_lines(pyramid.variant);
    let canvas_w = pyramid_canvas_w(pyramid.variant);
    let n = lines.len();
    for (row_idx, line) in lines.iter().enumerate() {
        let screen_y = area.y as i32 + area.height as i32 - n as i32 + row_idx as i32;
        if screen_y < area.y as i32 || screen_y >= area.bottom() as i32 {
            continue;
        }
        let start_x = area.x as i32 + pyramid.base_x;
        let eye_char = pyramid.eye_char_for_row(row_idx);
        render_alien_pyramid_line(
            line,
            pyramid.mirrored,
            LinePlacement {
                start_x,
                screen_y,
                canvas_w,
            },
            eye_char,
            color,
            area,
            buf,
        );
    }
}

fn render_alien_star(star: &AlienStar, area: Rect, buf: &mut Buffer) {
    let x = area.x + star.x;
    let y = area.y + star.y;
    if x < area.right() && y < area.bottom() {
        buf[(x, y)].set_char(star.twinkle.ch).set_style(
            Style::new()
                .fg(star.twinkle.color())
                .remove_modifier(Modifier::all()),
        );
    }
}

fn render_desert_sun(bg: &DesertSky, area: Rect, buf: &mut Buffer) {
    let w = area.width as f32;
    let h = area.height as f32;
    let sprite_h = SUN_LINES.len() as f32;
    let sprite_w = SUN_LINES
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0) as f32;
    let rx = w / 2.0 + sprite_w / 2.0;
    let s = bg.sky_angle;
    let center_x = w / 2.0 - rx * s.cos();
    let center_y = h - SUN_ORBIT_RY * s.sin();
    let base_x = (area.x as f32 + center_x - sprite_w / 2.0).round() as i32;
    let base_y = (area.y as f32 + center_y - sprite_h / 2.0).round() as i32;
    for (row_idx, line) in SUN_LINES.iter().enumerate() {
        let screen_y = base_y + row_idx as i32;
        if screen_y < area.y as i32 || screen_y >= area.bottom() as i32 {
            continue;
        }
        let h_shift = (SUN_H_WAVE_AMPLITUDE
            * (bg.h_phase - row_idx as f32 * SUN_H_WAVE_SPREAD).sin())
        .round() as i32;
        let rings = &bg.sun_rings[row_idx];
        for (col_idx, ch) in line.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let sx = base_x + col_idx as i32 + h_shift;
            if sx < area.x as i32 || sx >= area.right() as i32 {
                continue;
            }
            let ring = rings.get(col_idx).copied().unwrap_or(0);
            let color = sun_cell_color(ring, &bg.ring_colors);
            buf[(sx as u16, screen_y as u16)]
                .set_char(ch)
                .set_style(Style::new().fg(color).remove_modifier(Modifier::all()));
        }
    }
}

fn render_desert_stars(bg: &DesertSky, area: Rect, buf: &mut Buffer) {
    let w = area.width as f32;
    let h = area.height as f32;
    let rx = w / 2.0;
    let ry = h;
    let n = bg.night_progress();
    for star in &bg.stars {
        let e = star.angle + n;
        let elev = e.sin();
        if elev <= 0.0 {
            continue;
        }
        let sx = (w / 2.0 - rx * star.radius_frac * e.cos()).round() as i32;
        let sy = (h - ry * star.radius_frac * elev).round() as i32;
        let ax = area.x as i32 + sx;
        let ay = area.y as i32 + sy;
        if ax < area.x as i32
            || ax >= area.right() as i32
            || ay < area.y as i32
            || ay >= area.bottom() as i32
        {
            continue;
        }
        buf[(ax as u16, ay as u16)]
            .set_char(star.twinkle.ch)
            .set_style(
                Style::new()
                    .fg(star.twinkle.color())
                    .remove_modifier(Modifier::all()),
            );
    }
}

fn render_gate(gate: &Gate, area: Rect, buf: &mut Buffer) {
    let floor_y = area.y as i32 + area.height as i32 - 1;
    let floor_style = Style::new()
        .fg(gate.style.floor)
        .remove_modifier(Modifier::all());
    for x in area.x..area.right() {
        buf[(x, floor_y as u16)]
            .set_char(GATE_FLOOR_CHAR)
            .set_style(floor_style);
    }
    let style = Style::new()
        .fg(gate.style.bars)
        .remove_modifier(Modifier::all());
    let mut col = 0i32;
    let mut bars_drawn = 0usize;
    let mut bar = 0usize;
    let mut block = 0usize;
    while col < area.width as i32 {
        if bars_drawn < BARS_PER_BLOCK {
            draw_gate_bar_tile(col, floor_y, area, buf, style, gate.bar_holes(bar));
            col += GATE_BAR_W;
            bars_drawn += 1;
            bar += 1;
        } else {
            draw_gate_block_tile(col, floor_y, area, buf, style, gate.block_marks(block));
            col += GATE_BLOCK_W;
            bars_drawn = 0;
            block += 1;
        }
    }
}

fn draw_gate_bar_tile(
    col: i32,
    floor_y: i32,
    area: Rect,
    buf: &mut Buffer,
    style: Style,
    holes: Option<&BarHoles>,
) {
    let n = GATE_BAR_TILE.len() as i32;
    let mut pipe_idx = 0usize;
    for (i, &line) in GATE_BAR_TILE.iter().enumerate() {
        if line == GATE_BAR_PIPE {
            let hidden = holes.is_some_and(|h| h[pipe_idx]);
            pipe_idx += 1;
            if hidden {
                continue;
            }
        }
        let screen_y = floor_y - 1 - (n - 1 - i as i32);
        let see_through = line
            .chars()
            .map(|ch| if ch == ' ' { TRANSPARENT } else { ch });
        draw_gate_line(see_through, col, screen_y, area, buf, style);
    }
}

fn draw_gate_block_tile(
    col: i32,
    floor_y: i32,
    area: Rect,
    buf: &mut Buffer,
    style: Style,
    marks: &BlockMarks,
) {
    let n = GATE_BLOCK_TILE.len() as i32;
    for (i, &line) in GATE_BLOCK_TILE.iter().enumerate() {
        let screen_y = floor_y - 1 - (n - 1 - i as i32);
        let marked: String = line
            .chars()
            .enumerate()
            .map(|(c, ch)| {
                if c == GATE_BLOCK_MARK_COL {
                    marks[i]
                } else {
                    ch
                }
            })
            .collect();
        let solid = opaque_line(&marked, Color::Reset, |_, _| Color::Reset);
        draw_gate_line(
            solid.into_iter().map(|(ch, _)| ch),
            col,
            screen_y,
            area,
            buf,
            style,
        );
    }
}

fn draw_gate_line(
    line: impl Iterator<Item = char>,
    col: i32,
    screen_y: i32,
    area: Rect,
    buf: &mut Buffer,
    style: Style,
) {
    if screen_y < area.y as i32 || screen_y >= area.bottom() as i32 {
        return;
    }
    for (c, ch) in line.enumerate() {
        if ch == TRANSPARENT {
            continue;
        }
        let sx = area.x as i32 + col + c as i32;
        if sx < area.x as i32 || sx >= area.right() as i32 {
            continue;
        }
        buf[(sx as u16, screen_y as u16)]
            .set_char(ch)
            .set_style(style);
    }
}

fn render_heaven_background(bg: &HeavenBackground, show_names: bool, area: Rect, buf: &mut Buffer) {
    render_soul_wall(&bg.souls, show_names, area, buf);
    for cloud in &bg.clouds {
        render_cloud(cloud, area, buf);
    }
    render_gate(&bg.gate, area, buf);
}

fn render_soul_wall(wall: &SoulWall, show_names: bool, area: Rect, buf: &mut Buffer) {
    render_souls(wall, area, buf);
    if show_names {
        for soul in wall.drifting() {
            render_fish_label(soul, wall.color(), area, buf);
        }
    }
}

fn render_souls(wall: &SoulWall, area: Rect, buf: &mut Buffer) {
    let mut canvas = Buffer::empty(area);
    for soul in wall.drifting() {
        render_fish(soul, area, &mut canvas);
    }
    let style = Style::new()
        .fg(wall.color())
        .remove_modifier(Modifier::all());
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let symbol = canvas[(x, y)].symbol();
            if symbol == " " {
                continue;
            }
            buf[(x, y)].set_symbol(symbol).set_style(style);
        }
    }
}

fn render_cloud(cloud: &Cloud, area: Rect, buf: &mut Buffer) {
    let top = area.y as i32 + cloud.top(area.height);
    render_opaque_grid_from(&cloud.rows(), cloud.x.floor() as i32, top, area, buf);
}

fn render_angel(angel: &Angel, area: Rect, buf: &mut Buffer) {
    let top = area.y as i32 + angel.y.floor() as i32;
    render_opaque_grid_from(&angel.rows(), angel.x.floor() as i32, top, area, buf);
}

fn render_opaque_grid_from(
    grid: &[Vec<(char, Color)>],
    base_x: i32,
    top_y: i32,
    area: Rect,
    buf: &mut Buffer,
) {
    let bottom_y = top_y + grid.len() as i32 - 1;
    render_opaque_grid(grid, base_x, bottom_y, area, buf);
}

fn render_opaque_grid(
    grid: &[Vec<(char, Color)>],
    base_x: i32,
    bottom_y: i32,
    area: Rect,
    buf: &mut Buffer,
) {
    let n = grid.len() as i32;
    for (i, row) in grid.iter().enumerate() {
        let screen_y = bottom_y - (n - 1 - i as i32);
        if screen_y < area.y as i32 || screen_y >= area.bottom() as i32 {
            continue;
        }
        for (col, &(ch, color)) in row.iter().enumerate() {
            if ch == GRAVE_TRANSPARENT {
                continue;
            }
            let sx = area.x as i32 + base_x + col as i32;
            if sx < area.x as i32 || sx >= area.right() as i32 {
                continue;
            }
            if ch == ' ' {
                buf[(sx as u16, screen_y as u16)]
                    .set_char(' ')
                    .set_style(Style::reset());
                continue;
            }
            buf[(sx as u16, screen_y as u16)]
                .set_char(ch)
                .set_style(Style::new().fg(color).remove_modifier(Modifier::all()));
        }
    }
}

fn render_ghost(ghost: &Ghost, area: Rect, buf: &mut Buffer) {
    let base_x = area.x as i32 + ghost.x as i32;
    let base_y = area.y as i32 + ghost.y as i32;
    let style = SpriteStyle {
        eye_overrides: &[],
        body_color: ghost.color,
        interior_fg: Some(ghost.color),
        glisten_colors: &[],
        color_patches: &[],
    };
    for (i, line) in ghost.sprite_rows().iter().enumerate() {
        let screen_y = base_y + i as i32;
        if screen_y < area.y as i32 || screen_y >= area.bottom() as i32 {
            continue;
        }
        render_sprite_row(line, base_x, screen_y, area, buf, &style);
    }
}

fn render_bat(bat: &Bat, area: Rect, buf: &mut Buffer) {
    let y = area.y as i32 + bat.y as i32;
    if y < area.y as i32 || y >= area.bottom() as i32 {
        return;
    }
    let style = Style::new().fg(BAT_COLOR).remove_modifier(Modifier::all());
    for (i, &ch) in BAT_SPRITE.iter().enumerate() {
        let sx = area.x as i32 + bat.x as i32 + i as i32;
        if sx < area.x as i32 || sx >= area.right() as i32 {
            continue;
        }
        buf[(sx as u16, y as u16)].set_char(ch).set_style(style);
    }
}

fn render_cow(cow: &Cow, area: Rect, buf: &mut Buffer) {
    let sprite = cow_sprite(cow);
    let base_x = area.x as i32 + cow.position.x as i32;
    let base_y = area.y as i32 + cow.position.y as i32 - cow.sprite_top_offset() as i32;
    for (row_idx, row) in sprite.iter().enumerate() {
        let sy = base_y + row_idx as i32;
        if sy < area.y as i32 || sy >= area.bottom() as i32 {
            continue;
        }
        for (col_idx, &(ch, color)) in row.iter().enumerate() {
            if ch == crate::entities::cow::COW_TRANSPARENT {
                continue;
            }
            let sx = base_x + col_idx as i32;
            if sx < area.x as i32 || sx >= area.right() as i32 {
                continue;
            }
            if ch == ' ' {
                buf[(sx as u16, sy as u16)]
                    .set_char(' ')
                    .set_style(Style::reset());
                continue;
            }
            buf[(sx as u16, sy as u16)]
                .set_char(ch)
                .set_style(Style::new().fg(color).remove_modifier(Modifier::all()));
        }
    }
    render_cow_extension(cow, &sprite, base_x, base_y, area, buf);
}

const COW_FACES_LEFT: bool = true;

fn render_cow_extension(
    cow: &Cow,
    sprite: &[Vec<Cell>],
    base_x: i32,
    base_y: i32,
    area: Rect,
    buf: &mut Buffer,
) {
    let Some(ext) = cow.mutant.body_extension else {
        return;
    };
    let torso_row_idx = cow.sprite_top_offset() as usize + 1;
    let Some(torso_row) = sprite.get(torso_row_idx) else {
        return;
    };
    let mask: Vec<Cell> = torso_row
        .iter()
        .map(|&(ch, color)| {
            if ch == '_' {
                (ch, color)
            } else {
                (TRANSPARENT, color)
            }
        })
        .collect();
    let Some(span) = painted_span(&mask) else {
        return;
    };
    let posed = PosedExtension {
        ext,
        facing_left: COW_FACES_LEFT,
        phase: cow.sway.phase,
        max_tentacles: None,
    };
    for depth in 0..ext.length {
        let row = posed.row(&mask, span, depth, true);
        let screen_y = base_y + torso_row_idx as i32 - 1 - depth as i32;
        draw_appendage_row(&row, base_x, screen_y, area, buf);
    }
}

fn render_cow_name(cow: &Cow, area: Rect, buf: &mut Buffer) {
    let center_x = cow.position.x as i32 + cow.display_width as i32 / 2;
    let name_y = area.y as i32 + cow.position.y as i32 - 1 - cow.sprite_top_offset() as i32;
    render_label(&cow.name, center_x, name_y, WHITE, area, buf);
}

const SPEECH_TAIL_OFFSET_FROM_EYE: i32 = 3;

fn render_cow_speech(cow: &Cow, area: Rect, buf: &mut Buffer) {
    let Some(speech) = &cow.speech else { return };
    if cow.mutant.is_double {
        let head_render = cow.eye_count().clamp(2, 5);
        let head_w = head_render + 2;
        let torso = cow.torso_width();
        let left_eye = 1_i32;
        let right_eye = (head_w + torso + 1 + 1) as i32;
        draw_speech_bubble(cow, &speech.text, left_eye, area, buf);
        draw_speech_bubble(cow, &speech.text, right_eye, area, buf);
        return;
    }
    let left_eye = 1_i32;
    draw_speech_bubble(cow, &speech.text, left_eye, area, buf);
}

fn draw_speech_bubble(cow: &Cow, text: &str, leftmost_eye_col: i32, area: Rect, buf: &mut Buffer) {
    let fg = cow.bubble_color();
    let bubble = build_speech_bubble(text);
    let cow_x = area.x as i32 + cow.position.x as i32;
    let cow_y = area.y as i32 + cow.position.y as i32;
    let sprite_top = cow_y - cow.sprite_top_offset() as i32;
    let sprite_bottom = sprite_top + cow_sprite(cow).len() as i32 - 1;
    let tail_x = cow_x + leftmost_eye_col - SPEECH_TAIL_OFFSET_FROM_EYE;
    let tail_on = |side, y| Tail {
        anchor: (tail_x, y),
        facing_left: COW_FACES_LEFT,
        side,
    };
    let above = tail_on(Side::Above, cow_y + 1);
    let below = tail_on(Side::Below, sprite_bottom - 1);
    let (top, bottom) = (area.y as i32, area.bottom() as i32);
    let tail = match Side::of(
        bubble.len() as i32,
        above.room(top, bottom),
        below.room(top, bottom),
    ) {
        Side::Above => above,
        Side::Below => below,
    };
    let left = area.x as i32 + bubble_left(&tail, bubble_width(&bubble), area);
    let bubble_top = tail.bubble_top(bubble.len() as i32);
    let ink = speech_ink(cow.is_alienated(), fg);
    draw_bubble(&bubble, (left, bubble_top), fg, ink, area, buf);
    for (x, y) in tail.cells() {
        put_cell(buf, area, x, y, tail.glyph(), fg);
    }
}

fn put_cell(buf: &mut Buffer, area: Rect, x: i32, y: i32, ch: char, fg: Color) {
    let inside_x = x >= area.x as i32 && x < area.right() as i32;
    let inside_y = y >= area.y as i32 && y < area.bottom() as i32;
    if !inside_x || !inside_y {
        return;
    }
    overdraw(buf, x as u16, y as u16)
        .set_char(ch)
        .set_style(Style::new().fg(fg).remove_modifier(Modifier::all()));
}

fn render_ufo(ufo: &Ufo, area: Rect, buf: &mut Buffer) {
    let sprite = ufo_sprite(ufo);
    let base_x = area.x as i32 + ufo.x as i32;
    let base_y = area.y as i32 + ufo.y as i32;
    for (row_idx, row) in sprite.iter().enumerate() {
        let sy = base_y + row_idx as i32;
        if sy < area.y as i32 || sy >= area.bottom() as i32 {
            continue;
        }
        for (col_idx, &(ch, color)) in row.iter().enumerate() {
            if ch == '\0' {
                continue;
            }
            let sx = base_x + col_idx as i32;
            if sx < area.x as i32 || sx >= area.right() as i32 {
                continue;
            }
            if ch == ' ' {
                buf[(sx as u16, sy as u16)]
                    .set_char(' ')
                    .set_style(Style::reset());
                continue;
            }
            buf[(sx as u16, sy as u16)]
                .set_char(ch)
                .set_style(Style::new().fg(color).remove_modifier(Modifier::all()));
        }
    }
}

fn render_ritual_text(lines: &[Option<String>; 2], area: Rect, buf: &mut Buffer) {
    let target_eye_y = area.height as i32 / 2 + VOID_EYE_VERTICAL_OFFSET;
    let text_y_base = target_eye_y + VOID_TEXT_BELOW_EYE_OFFSET;
    let s = Style::new().fg(WHITE).remove_modifier(Modifier::all());

    for (i, line_opt) in lines.iter().enumerate() {
        if let Some(text) = line_opt {
            let y = area.y as i32 + text_y_base + i as i32;
            if y < area.y as i32 || y >= area.bottom() as i32 {
                continue;
            }
            let text_w = text.chars().count() as i32;
            let x_start = area.x as i32 + area.width as i32 / 2 - text_w / 2;
            for (col, ch) in text.chars().enumerate() {
                let x = x_start + col as i32;
                if x < area.x as i32 || x >= area.right() as i32 {
                    continue;
                }
                buf[(x as u16, y as u16)].set_char(ch).set_style(s);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::LIGHT_YELLOW;
    use crate::fishes::species::FishSpecies;
    use crate::sprite::opaque_line;
    use crate::tank::TankKind;
    use crate::tanks::heaven::{ANGEL, ANGEL_COLOR, SOUL_COLOR};
    use std::f32::consts::PI;

    fn heaven_with_soul(name: &str) -> Tank {
        let mut tank = Tank::new("Heaventank".to_string(), TankKind::Heaven, &[]);
        tank.resize(80, 30, &[]);
        let fish = Fish::new(
            FishSpecies::Salmon,
            name.to_string(),
            0.0,
            0.0,
            &mut rand::rng(),
        );
        tank.receive_soul(fish);
        tank
    }

    fn render_tank(tank: &Tank, names: bool) -> Buffer {
        let area = Rect::new(0, 0, tank.width, tank.height);
        let mut buf = Buffer::empty(area);
        TankView::new(tank).with_names(names).render(area, &mut buf);
        buf
    }

    fn row_text(buf: &Buffer, y: u16) -> String {
        (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect()
    }

    fn park_soul(tank: &mut Tank, x: f32, y: f32) {
        let TankBackground::Heaven { bg } = &mut tank.background else {
            panic!("a heaven");
        };
        bg.clouds.clear();
        bg.angels.clear();
        let soul = bg.souls.drifting_mut().next().expect("a soul");
        soul.position.x = x;
        soul.position.y = y;
    }

    fn heaven(tank: &mut Tank) -> &mut HeavenBackground {
        let TankBackground::Heaven { bg } = &mut tank.background else {
            panic!("a heaven");
        };
        bg
    }

    fn find_cells(buf: &Buffer, needle: &str) -> Option<(u16, u16)> {
        let width = needle.chars().count() as u16;
        for y in 0..buf.area.height {
            for x in 0..buf.area.width.saturating_sub(width) {
                let found: String = (x..x + width).map(|col| buf[(col, y)].symbol()).collect();
                if found == needle {
                    return Some((x, y));
                }
            }
        }
        None
    }

    fn speaking_tank(alien: bool) -> Tank {
        use crate::fishes::mutations::{Mutation, apply_mutation};
        let mut rng = rand::rng();
        let mut tank = Tank::new("Fishtank".to_string(), TankKind::Base, &[]);
        tank.resize(80, 30, &[]);
        tank.spawn_fish(FishSpecies::Merluza, "Zed".to_string(), &mut rng);
        let fish = &mut tank.fish[0];
        if alien {
            apply_mutation(fish, Mutation::Alienation, &mut rng);
        }
        fish.position.x = 30.0;
        fish.position.y = 15.0;
        fish.say("hello there".to_string());
        tank
    }

    #[test]
    fn an_alien_speaks_in_light_green_inside_a_white_bubble() {
        let buf = render_tank(&speaking_tank(true), false);
        let (x, y) = find_cells(&buf, "hello there").expect("the bubble is drawn");
        assert!((x..x + 11).all(|col| buf[(col, y)].fg == crate::tank::ALIEN_INK));
        assert_eq!(buf[(x - 2, y)].fg, WHITE, "the frame keeps its colour");
    }

    #[test]
    fn an_earthly_fish_speaks_in_white() {
        let buf = render_tank(&speaking_tank(false), false);
        let (x, y) = find_cells(&buf, "hello there").expect("the bubble is drawn");
        assert_eq!(buf[(x, y)].fg, WHITE);
    }

    #[test]
    fn an_alien_cow_speaks_in_light_green() {
        let mut rng = rand::rng();
        let mut tank = Tank::new("Fishtank".to_string(), TankKind::Base, &[]);
        tank.resize(80, 30, &[]);
        tank.spawn_cow(crate::entities::cow::CowVariant::LightGreen, &mut rng);
        tank.cows[0].position.x = 30.0;
        tank.cows[0].say("moo".to_string());
        let buf = render_tank(&tank, false);
        let (x, y) = find_cells(&buf, "moo").expect("the bubble is drawn");
        assert_eq!(buf[(x, y)].fg, crate::tank::ALIEN_INK);
    }

    #[test]
    fn a_fish_in_a_ufo_beam_is_drawn_with_its_eyes_shut() {
        use crate::entities::ufo::{UFO_SPRITE_HEIGHT, Ufo};
        let mut tank = Tank::new("Fishtank".to_string(), TankKind::Base, &[]);
        tank.resize(80, 30, &[]);
        let fish = Fish::new(
            FishSpecies::Merluza,
            "Zed".to_string(),
            0.0,
            0.0,
            &mut rand::rng(),
        );
        let target_y = (tank.height as usize - UFO_SPRITE_HEIGHT) as f32;
        let mut ufo = Ufo::new_drop_fish(20.0, target_y, fish);
        ufo.y = target_y;
        ufo.tick(0.0);
        tank.ufos.push(ufo);
        let buf = render_tank(&tank, false);
        let carried = tank
            .ufos
            .first()
            .and_then(Ufo::carried_fish)
            .expect("in the beam");
        let row = carried.position.y as u16;
        let drawn = row_text(&buf, row);
        assert!(drawn.contains('¯'), "the shut eye is drawn: {drawn}");
        assert!(!drawn.contains('º'), "a carried fish sleeps: {drawn}");
    }

    #[test]
    fn a_dead_fish_and_its_name_are_dark_gray_at_their_own_level() {
        let mut tank = heaven_with_soul("Ann");
        park_soul(&mut tank, 30.0, 5.0);
        let buf = render_tank(&tank, true);
        let name_col = row_text(&buf, 4)
            .find("Ann")
            .expect("the name floats above its soul");
        assert_eq!(buf[(name_col as u16, 4)].fg, SOUL_COLOR);
        let body: Vec<u16> = (0..buf.area.width)
            .filter(|&x| buf[(x, 5)].symbol() != " ")
            .collect();
        assert!(!body.is_empty(), "the soul is drawn");
        assert!(
            body.iter().all(|&x| buf[(x, 5)].fg == SOUL_COLOR),
            "every cell of a dead fish is dark gray"
        );
    }

    #[test]
    fn a_damned_fish_and_its_name_are_dark_red_behind_the_hell_plants() {
        use crate::tanks::hell::SOUL_COLOR as DAMNED_COLOR;
        let mut tank = Tank::new("Helltank".to_string(), TankKind::Hell, &[]);
        tank.resize(80, 30, &[]);
        let mut fish = Fish::new(
            FishSpecies::Salmon,
            "Cain".to_string(),
            0.0,
            0.0,
            &mut rand::rng(),
        );
        fish.devil_marked = true;
        tank.receive_soul(fish);
        let TankBackground::Hell { bg, plants } = &mut tank.background else {
            panic!("a hell");
        };
        plants.clear();
        bg.face_grid.clear();
        let soul = bg.souls.drifting_mut().next().expect("a soul");
        soul.position.x = 30.0;
        soul.position.y = 5.0;

        let buf = render_tank(&tank, true);

        let name_col = row_text(&buf, 4)
            .find("Cain")
            .expect("the name floats above its soul");
        assert_eq!(buf[(name_col as u16, 4)].fg, DAMNED_COLOR);
        let body: Vec<u16> = (0..buf.area.width)
            .filter(|&x| buf[(x, 5)].symbol() != " ")
            .collect();
        assert!(!body.is_empty(), "the soul is drawn");
        assert!(body.iter().all(|&x| buf[(x, 5)].fg == DAMNED_COLOR));
    }

    #[test]
    fn a_cloud_and_the_gate_cover_a_soul() {
        let mut tank = heaven_with_soul("Ann");
        park_soul(&mut tank, 30.0, 5.0);
        let bare = render_tank(&tank, false);
        assert!(
            row_text(&bare, 5).contains('<'),
            "the soul shows on an empty sky"
        );

        let low_cloud_altitude = 3.0 / f32::from(tank.height);
        heaven(&mut tank).clouds = (0..10)
            .map(|x| Cloud::covering(x as f32 * 8.0, low_cloud_altitude))
            .collect();
        let clouded = render_tank(&tank, false);
        assert!(
            !row_text(&clouded, 5).contains('<'),
            "a cloud is drawn over the dead: {}",
            row_text(&clouded, 5)
        );

        park_soul(&mut tank, 30.0, 27.0);
        let gated = render_tank(&tank, false);
        let bar_x = (30..45).find(|&x| gated[(x, 27)].symbol() == "|");
        assert!(
            bar_x.is_some(),
            "the gate's bars stand in front of a low soul"
        );
        assert_eq!(gated[(bar_x.unwrap(), 27)].fg, LIGHT_YELLOW);
    }

    #[test]
    fn a_gate_block_is_solid_and_hides_the_soul_behind_it() {
        let mut tank = heaven_with_soul("Ann");
        let block_x = (BARS_PER_BLOCK as i32 * GATE_BAR_W) as u16;
        let floor_y = tank.height - 1;
        let soul_y = floor_y - GATE_BLOCK_TILE.len() as u16 / 2;
        for x in (0..GATE_BLOCK_W as u16).map(|dx| block_x + dx - 2) {
            park_soul(&mut tank, f32::from(x), f32::from(soul_y));
            let buf = render_tank(&tank, false);
            for (row, line) in GATE_BLOCK_TILE.iter().enumerate() {
                let y = floor_y - GATE_BLOCK_TILE.len() as u16 + row as u16;
                let chars: Vec<char> = line.chars().collect();
                let first = chars.iter().position(|&c| c != ' ').unwrap();
                let last = chars.iter().rposition(|&c| c != ' ').unwrap();
                for col in first..=last {
                    let cell = &buf[(block_x + col as u16, y)];
                    assert_eq!(
                        cell.fg,
                        LIGHT_YELLOW,
                        "row {row} col {col}: {:?}",
                        cell.symbol()
                    );
                }
            }
        }
    }

    #[test]
    fn an_angel_is_solid_bright_yellow_and_hides_the_gate_behind_it() {
        let mut tank = heaven_with_soul("Ann");
        park_soul(&mut tank, 70.0, 2.0);
        let (x, y) = (10u16, 20u16);
        heaven(&mut tank)
            .angels
            .push(Angel::at(f32::from(x), f32::from(y)));
        let buf = render_tank(&tank, false);
        for (row, line) in ANGEL.iter().enumerate() {
            let sy = y + row as u16;
            let cells = opaque_line(line, ANGEL_COLOR, |_, _| ANGEL_COLOR);
            for (col, &(ch, _)) in cells.iter().enumerate() {
                if ch == TRANSPARENT {
                    continue;
                }
                let cell = &buf[(x + col as u16, sy)];
                assert_eq!(cell.symbol(), ch.to_string(), "row {row} col {col}");
                assert!(
                    !cell.modifier.contains(Modifier::DIM),
                    "row {row} col {col}"
                );
                if ch != ' ' {
                    assert_eq!(cell.fg, ANGEL_COLOR, "row {row} col {col}");
                }
            }
        }
    }

    fn render_at_angle(angle: f32, w: u16, h: u16) {
        let mut tank = Tank::new("Smoke".into(), TankKind::Desert, &[]);
        tank.resize(w, h, &[]);
        if let TankBackground::Desert { bg } = &mut tank.background {
            bg.sky_angle = angle;
        }
        let area = Rect::new(0, 0, w, h);
        let mut buf = Buffer::empty(area);
        TankView::new(&tank)
            .with_names(true)
            .with_nets(true)
            .render(area, &mut buf);
    }

    #[test]
    fn desert_renders_day_and_night_without_panic() {
        for &(w, h) in &[(80u16, 24u16), (40, 12), (200, 50), (8, 6)] {
            render_at_angle(PI * 0.5, w, h);
            render_at_angle(PI * 1.5, w, h);
            render_at_angle(0.0, w, h);
            render_at_angle(PI, w, h);
        }
    }

    #[test]
    fn ball_border_ears_trace_both_contours() {
        let mut rng = rand::rng();
        let mut fish = Fish::new_unfish(UnfishKind::Ball, "Orb".into(), 5.0, 5.0, &mut rng);
        let rows = BALL_BASE.len();
        if let Some(us) = fish.unfish_state.as_mut() {
            us.ear_count = rows;
        }
        let area = Rect::new(0, 0, 40, 20);
        let mut buf = Buffer::empty(area);
        render_multi_row_unfish_at(&fish, 10, 2, area, &mut buf);
        let mut left = 0;
        let mut right = 0;
        for y in 0..area.height {
            for x in 0..area.width {
                match buf[(x, y)].symbol() {
                    "Ɛ" => left += 1,
                    "3" => right += 1,
                    _ => {}
                }
            }
        }
        assert_eq!(left, rows, "every ball row gets a left Ɛ ear");
        assert_eq!(right, rows, "every ball row gets a right 3 ear");
    }

    #[test]
    fn ball_spike_extension_flanks_both_edges() {
        use crate::sprite::{BodyExtension, ExtensionVariant};
        let mut rng = rand::rng();
        let mut fish = Fish::new_unfish(UnfishKind::Ball, "Orb".into(), 5.0, 5.0, &mut rng);
        if let Some(us) = fish.unfish_state.as_mut() {
            us.body_extension = Some(BodyExtension {
                variant: ExtensionVariant::Spike,
                length: 2,
                seed: 0,
            });
        }
        let base_x = 10i32;
        let base_y = 4i32;
        let area = Rect::new(0, 0, 40, 24);
        let mut buf = Buffer::empty(area);
        render_multi_row_unfish_at(&fish, base_x, base_y, area, &mut buf);
        let above = |y: i32| {
            (0..area.width)
                .filter(|&x| buf[(x, y as u16)].symbol() == "¦")
                .count()
        };
        let top_band = above(base_y - 1) + above(base_y - 2);
        let bottom_band =
            above(base_y + BALL_BASE.len() as i32) + above(base_y + BALL_BASE.len() as i32 + 1);
        assert!(top_band > 0, "spikes sit above the ball's highest row");
        assert!(bottom_band > 0, "spikes sit below the ball's lowest row");
        let on_body = (base_y..base_y + BALL_BASE.len() as i32)
            .map(above)
            .sum::<usize>();
        assert_eq!(on_body, 0, "spikes never overlap the ball body rows");
    }

    #[test]
    fn ball_tentacle_extension_caps_at_two_per_band() {
        use crate::sprite::{BodyExtension, ExtensionVariant};
        let mut rng = rand::rng();
        let mut fish = Fish::new_unfish(UnfishKind::Ball, "Orb".into(), 5.0, 5.0, &mut rng);
        if let Some(us) = fish.unfish_state.as_mut() {
            us.body_extension = Some(BodyExtension {
                variant: ExtensionVariant::Tentacle,
                length: 1,
                seed: 0,
            });
        }
        let base_x = 10i32;
        let base_y = 4i32;
        let area = Rect::new(0, 0, 40, 24);
        let mut buf = Buffer::empty(area);
        render_multi_row_unfish_at(&fish, base_x, base_y, area, &mut buf);
        let top = (0..area.width)
            .filter(|&x| buf[(x, (base_y - 1) as u16)].symbol() == "|")
            .count();
        assert!(
            (1..=MULTI_ROW_MAX_TENTACLES).contains(&top),
            "a ball band carries at most two tentacles"
        );
    }

    #[test]
    fn ball_feet_sit_on_the_row_below_the_body() {
        use crate::sprite::{Feet, FeetStyle};
        let mut rng = rand::rng();
        let mut fish = Fish::new_unfish(UnfishKind::Ball, "Orb".into(), 5.0, 5.0, &mut rng);
        if let Some(us) = fish.unfish_state.as_mut() {
            us.feet = Some(Feet {
                style: FeetStyle::Quote,
                color: None,
            });
        }
        let base_x = 10i32;
        let base_y = 2i32;
        let area = Rect::new(0, 0, 40, 20);
        let mut buf = Buffer::empty(area);
        render_multi_row_unfish_at(&fish, base_x, base_y, area, &mut buf);
        let feet_y = (base_y + BALL_BASE.len() as i32) as u16;
        let feet = (0..area.width)
            .filter(|&x| buf[(x, feet_y)].symbol() == "\"")
            .count();
        assert!(
            feet > 0,
            "a footed ball stamps feet on the row directly below its lowest line"
        );
        let body_rows = (base_y..base_y + BALL_BASE.len() as i32)
            .flat_map(|y| (0..area.width).map(move |x| (x, y as u16)))
            .filter(|&(x, y)| buf[(x, y)].symbol() == "\"")
            .count();
        assert_eq!(body_rows, 0, "feet never overlap the ball body rows");
    }

    #[test]
    fn worm_feet_sit_one_row_below_the_body() {
        use crate::sprite::{Feet, FeetStyle};
        let mut rng = rand::rng();
        let mut fish = Fish::new_unfish(UnfishKind::Worm, "Wiggly".into(), 6.0, 8.0, &mut rng);
        if let Some(us) = fish.unfish_state.as_mut() {
            us.feet = Some(Feet {
                style: FeetStyle::Caret,
                color: None,
            });
        }
        let area = Rect::new(0, 0, 60, 24);
        let mut buf = Buffer::empty(area);
        render_fish(&fish, area, &mut buf);
        let body_y = area.y + fish.position.y as u16;
        let feet_y = body_y + 1;
        let feet = (0..area.width)
            .filter(|&x| buf[(x, feet_y)].symbol() == "^")
            .count();
        assert!(feet > 0, "a footed worm stamps carets on the row below it");
        let on_body = (0..area.width)
            .filter(|&x| buf[(x, body_y)].symbol() == "^")
            .count();
        assert_eq!(on_body, 0, "the worm body row never carries feet");
    }

    #[test]
    fn cow_spike_extension_sits_above_the_torso_top() {
        use crate::entities::cow::CowVariant;
        use crate::sprite::{BodyExtension, ExtensionVariant};
        let mut rng = rand::rng();
        let mut cow = Cow::new("Bessie".into(), CowVariant::Brown, 5.0, 8.0, &mut rng);
        cow.mutant.body_extension = Some(BodyExtension {
            variant: ExtensionVariant::Spike,
            length: 2,
            seed: 0,
        });
        let area = Rect::new(0, 0, 60, 24);
        let mut buf = Buffer::empty(area);
        render_cow(&cow, area, &mut buf);
        let top_offset = cow.sprite_top_offset() as i32;
        let base_y = area.y as i32 + cow.position.y as i32 - top_offset;
        let torso_top_screen = base_y + top_offset + 1;
        let mut count = 0;
        for y in 0..area.height {
            for x in 0..area.width {
                if buf[(x, y)].symbol() == "¦" {
                    count += 1;
                    assert!(
                        (y as i32) < torso_top_screen,
                        "the cow's spikes sit strictly above the torso top line"
                    );
                }
            }
        }
        assert!(count > 0, "the cow grows spikes above its torso");
    }

    #[test]
    fn worm_tentacle_extension_hangs_below_the_body() {
        use crate::sprite::{BodyExtension, ExtensionVariant};
        let mut rng = rand::rng();
        let mut fish = Fish::new_unfish(UnfishKind::Worm, "Wiggly".into(), 6.0, 8.0, &mut rng);
        if let Some(us) = fish.unfish_state.as_mut() {
            us.body_extension = Some(BodyExtension {
                variant: ExtensionVariant::Tentacle,
                length: 2,
                seed: 0,
            });
        }
        let area = Rect::new(0, 0, 60, 24);
        let mut buf = Buffer::empty(area);
        render_fish(&fish, area, &mut buf);
        let body_y = area.y + fish.position.y as u16;
        let below = (0..area.width)
            .filter(|&x| buf[(x, body_y + 1)].symbol() == "|")
            .count();
        assert!(below > 0, "worm tentacles hang on the rows below the body");
        let on_body = (0..area.width)
            .filter(|&x| buf[(x, body_y)].symbol() == "|")
            .count();
        assert_eq!(on_body, 0, "the worm body row never carries tentacles");
    }

    #[test]
    fn skull_border_ears_sit_inside_the_wings() {
        let mut rng = rand::rng();
        let mut fish = Fish::new_unfish(UnfishKind::Skull, "Skull".into(), 5.0, 5.0, &mut rng);
        if let Some(us) = fish.unfish_state.as_mut() {
            us.ear_count = SKULL_OPEN.len();
            us.wings.is_open = true;
        }
        let base_x = 10i32;
        let base_y = 2i32;
        let area = Rect::new(0, 0, 40, 20);
        let mut buf = Buffer::empty(area);
        render_multi_row_unfish_at(&fish, base_x, base_y, area, &mut buf);
        let wing_row_y = (base_y + 2) as u16;
        assert_eq!(
            buf[((base_x) as u16, wing_row_y)].symbol(),
            "}",
            "the left wing is preserved"
        );
        assert_eq!(
            buf[((base_x + 2) as u16, wing_row_y)].symbol(),
            "Ɛ",
            "the left ear sits at the inner body wall, never outside the wing"
        );
        assert_eq!(
            buf[((base_x + 10) as u16, wing_row_y)].symbol(),
            "3",
            "the right ear sits at the inner body wall"
        );
        assert_eq!(
            buf[((base_x + 12) as u16, wing_row_y)].symbol(),
            "{",
            "the right wing is preserved"
        );
    }

    fn assert_body_row_matches(fish: &Fish, buf: &Buffer, area: Rect) {
        let segs = fish.segments();
        let y = area.y + fish.position.y as u16;
        let mut col = 0u16;
        for (ch, color) in segs {
            let x = area.x + fish.position.x as u16 + col;
            if x >= area.right() {
                break;
            }
            let cell = &buf[(x, y)];
            assert_eq!(cell.symbol(), ch.to_string(), "char at col {col}");
            assert_eq!(cell.fg, color, "color at col {col}");
            col += UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
        }
    }

    #[test]
    fn line_fish_body_row_is_byte_identical_to_segments() {
        use crate::fishes::fish::Direction;
        let mut rng = rand::rng();
        let area = Rect::new(0, 0, 80, 24);
        let line_species = [
            FishSpecies::Merluza,
            FishSpecies::Salmon,
            FishSpecies::Koi,
            FishSpecies::Mutantfish,
        ];
        for species in line_species {
            for facing in [Direction::Left, Direction::Right] {
                let mut fish = Fish::new(species, "T".into(), 6.0, 8.0, &mut rng);
                fish.facing = facing;
                let mut buf = Buffer::empty(area);
                render_fish(&fish, area, &mut buf);
                assert_body_row_matches(&fish, &buf, area);
            }
        }
        for facing in [Direction::Left, Direction::Right] {
            let mut fish = Fish::new_unfish(UnfishKind::Reversed, "U".into(), 6.0, 8.0, &mut rng);
            fish.facing = facing;
            let mut buf = Buffer::empty(area);
            render_fish(&fish, area, &mut buf);
            assert_body_row_matches(&fish, &buf, area);
        }
    }
}
