use std::f32::consts::TAU;

use rand::RngExt;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

use crate::colors::{
    BROWN_DARK, CREAM, DARK_GRAY, GREEN, KHAKI, LIGHT_GREEN, LIGHT_RED, RED, WHITE,
};
use crate::fishes::fish::{Direction, Fish};
use crate::loot::{
    ConsumableKind, ItemKind, JunkSprite, LootKind, bait_sprite_rows, blank_blueprint_sprite_rows,
    blank_wafer_sprite_rows, coffee_sprite_rows, computer_sprite_rows, demoncore_sprite_rows,
    fabricator_sprite_rows, golden_pearl_sprite_rows, milk_sprite_rows, part_sprite_rows,
    void_seed_sprite_rows,
};
use crate::ui::{
    hint_bar::HintBar,
    hints::{HINT_CLOSE, HINT_ENTER_CAPTURE},
    layout::Screen,
    panels::{PanelSpec, Panels, Reach},
    render_fish_sprite, table,
    text_input::{TextInput, draw_text_cursor},
};
use unicode_width::UnicodeWidthChar;

const BACKGROUND: Color = Color::Reset;
const RIGHT_PANEL_WIDTH: u16 = 34;
const OVERLAY_HEIGHT: u16 = 7;
const CARD_FRAME_ROWS: u16 = 2;
const MIN_BODY_WIDTH: u16 = 16;
const TEXT_PAD: u16 = 1;
const ART_INSET: u16 = 1;
const HOOK_LINE_MIN_ROWS: u16 = 1;
const HOOK_LINE: char = '⎹';
const HOOK: char = 'J';
const CASH_SPRITE: &str = "[ $ ]";
const CASH_HOOK_COL: u16 = 5;
const FOOD_HOOK_COL: u16 = 11;
const FOOD_HOOK_ROW: u16 = 2;
const NAME_LABEL: &str = "Name it";
const CONGRATULATIONS: &str = "Congratulations!";
const CASH_ASIDE: &str = "Chasing cash, making money.";
const ADDED_TO_INVENTORY: &str = "Added to inventory";
const MILK_OVERLAY_HEIGHT: u16 = 9;

const DEMON_CORE_OVERLAY_HEIGHT: u16 = 9;
const DEMON_CORE_GLISTEN_SPEED: f32 = 3.0;

const COMPUTER_OVERLAY_HEIGHT: u16 = 13;

const NECRO_OVERLAY_HEIGHT: u16 = 8;
const NECRO_HOOK_COL: u16 = 12;
const NECRO_HOOK_ROW: u16 = 0;
const NECRO_LEFT_PANEL_WIDTH: u16 = NECRO_HOOK_COL + 4;
const NECRO_BOOK_COLOR: Color = RED;
const NECRO_EYE_OPEN_CHAR: char = 'ʘ';
const NECRO_EYE_CLOSED_CHAR: char = 'u';
const NECRO_EYE_OPEN_TIME: f32 = 0.7;
const NECRO_EYE_CLOSED_TIME: f32 = 0.2;
const NECRO_EYE_COUNT: usize = 9;

const BREAD: Color = CREAM;
const CHEESE: Color = KHAKI;
const BURGER: Color = BROWN_DARK;

pub struct CatchState {
    pub loot: LootKind,
    pub fish: Option<Fish>,
    pub name_input: TextInput,
    pub cursor_visible: bool,
    pub item_qty: u32,
    pub anim_phase: bool,
    pub necro_eye_open: Vec<bool>,
    necro_eye_timers: Vec<f32>,
    blink_timer: f32,
    anim_tick: f32,
    glisten_phase: f32,
}

impl CatchState {
    pub fn new(loot: LootKind, rng: &mut impl RngExt) -> Self {
        let fish = match &loot {
            LootKind::Fish(species) => Some(Fish::new(*species, String::new(), 0.0, 0.0, rng)),
            _ => None,
        };
        Self::holding(loot, fish, rng)
    }

    pub fn holding(loot: LootKind, fish: Option<Fish>, rng: &mut impl RngExt) -> Self {
        let fish = fish.map(|mut f| {
            f.facing = Direction::Right;
            f.velocity.dx = f.velocity.dx.abs();
            f
        });
        let necro_eye_open: Vec<bool> =
            (0..NECRO_EYE_COUNT).map(|_| rng.random::<bool>()).collect();
        let necro_eye_timers: Vec<f32> = necro_eye_open
            .iter()
            .map(|&open| {
                if open {
                    rng.random_range(0.0..NECRO_EYE_OPEN_TIME)
                } else {
                    rng.random_range(0.0..NECRO_EYE_CLOSED_TIME)
                }
            })
            .collect();
        Self {
            loot,
            fish,
            name_input: TextInput::new(),
            cursor_visible: true,
            item_qty: 0,
            anim_phase: false,
            necro_eye_open,
            necro_eye_timers,
            blink_timer: 0.0,
            anim_tick: 0.0,
            glisten_phase: 0.0,
        }
    }

    pub fn is_fish(&self) -> bool {
        self.fish.is_some()
    }

    pub fn tick(&mut self, fps: f32) {
        let dt = 1.0 / fps;
        if let Some(ref mut fish) = self.fish {
            fish.tick_animation(dt);
        }
        if self.is_fish() {
            self.blink_timer += 1.0;
            let half_period = (fps * 0.5).max(1.0);
            if self.blink_timer >= half_period {
                self.blink_timer = 0.0;
                self.cursor_visible = !self.cursor_visible;
            }
        }
        if matches!(
            self.loot,
            LootKind::Item(ItemKind::Consumable(ConsumableKind::Coffee))
        ) {
            self.anim_tick += 1.0;
            let half = (fps * 0.3).max(1.0);
            if self.anim_tick >= half {
                self.anim_tick = 0.0;
                self.anim_phase = !self.anim_phase;
            }
        }
        if glistens(&self.loot) {
            self.glisten_phase =
                (self.glisten_phase + dt * DEMON_CORE_GLISTEN_SPEED).rem_euclid(TAU);
        }
        if is_necronomicon(&self.loot) {
            for i in 0..self.necro_eye_timers.len() {
                self.necro_eye_timers[i] -= dt;
                if self.necro_eye_timers[i] <= 0.0 {
                    self.necro_eye_open[i] = !self.necro_eye_open[i];
                    self.necro_eye_timers[i] = if self.necro_eye_open[i] {
                        NECRO_EYE_OPEN_TIME
                    } else {
                        NECRO_EYE_CLOSED_TIME
                    };
                }
            }
        }
    }

    pub fn reset_blink(&mut self) {
        self.cursor_visible = true;
        self.blink_timer = 0.0;
    }
}

pub struct CatchOverlay<'a> {
    state: &'a CatchState,
    screen: Screen,
}

impl<'a> CatchOverlay<'a> {
    pub fn new(state: &'a CatchState, screen: Screen) -> Self {
        Self { state, screen }
    }
}

impl Widget for CatchOverlay<'_> {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        let state = self.state;
        let art = CardArt::of(state);
        let lines = card_lines(state);
        let hints = card_hints(state);
        let border = loot_border_color(state);
        let title = overlay_title(&state.loot);
        let rows_for = |width: u16| CardLine::rows(&lines, width);
        let spec = PanelSpec {
            title: &title,
            title_style: Style::default()
                .fg(border)
                .add_modifier(Modifier::BOLD)
                .bg(BACKGROUND),
            border: Style::default().fg(border).bg(BACKGROUND),
            background: BACKGROUND,
            side: (art.width, art.height),
            body_w: RIGHT_PANEL_WIDTH,
            body_min_w: MIN_BODY_WIDTH.max(CardLine::longest_word(&lines) + TEXT_PAD * 2),
            body_rows: &rows_for,
            hints: &hints,
            reach: Reach::Full,
        };
        let panels = Panels::open(buf, self.screen, &spec);
        art.draw(buf, panels.side);
        CardLine::draw_all(buf, panels.body, &lines, state);
    }
}

enum Sprite<'a> {
    Fish(&'a Fish),
    Cells(Vec<Vec<(char, Color)>>),
}

struct CardArt<'a> {
    sprite: Sprite<'a>,
    hook: (u16, u16),
    width: u16,
    height: u16,
}

impl<'a> CardArt<'a> {
    fn of(state: &'a CatchState) -> Self {
        let (sprite, hook) = match &state.loot {
            LootKind::Fish(_) => match state.fish.as_ref() {
                Some(fish) => (
                    Sprite::Fish(fish),
                    (
                        fish.display_width as u16,
                        fish.line_sprite().body_row as u16,
                    ),
                ),
                None => (Sprite::Cells(Vec::new()), (0, 0)),
            },
            LootKind::Cash(cv) => (
                Sprite::Cells(vec![
                    CASH_SPRITE.chars().map(|ch| (ch, cv.color())).collect(),
                ]),
                (CASH_HOOK_COL, 0),
            ),
            LootKind::Food(_) => (
                Sprite::Cells(food_sprite_rows()),
                (FOOD_HOOK_COL, FOOD_HOOK_ROW),
            ),
            LootKind::Item(ItemKind::Junk(sprite)) => (
                Sprite::Cells(sprite.rows.clone()),
                (JunkSprite::hook_col(), JunkSprite::hook_row()),
            ),
            LootKind::Item(ItemKind::Consumable(ConsumableKind::Necronomicon)) => (
                Sprite::Cells(necro_sprite_rows(&state.necro_eye_open)),
                (NECRO_HOOK_COL, NECRO_HOOK_ROW),
            ),
            LootKind::Item(ItemKind::Consumable(kind)) => (
                Sprite::Cells(consumable_rows(*kind, state).unwrap_or_default()),
                (kind.hook_col(), kind.hook_row()),
            ),
        };
        let mut art = Self {
            sprite,
            hook,
            width: left_panel_inner_w(&state.loot, state.fish.as_ref()),
            height: 0,
        };
        art.height =
            base_card_height(&state.loot).max(art.rows() + CARD_FRAME_ROWS) - CARD_FRAME_ROWS;
        art
    }

    fn rows(&self) -> u16 {
        match &self.sprite {
            Sprite::Fish(fish) => fish.line_sprite().rows.len() as u16,
            Sprite::Cells(rows) => rows.len() as u16,
        }
    }

    fn draw(&self, buf: &mut Buffer, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let (hook_col, hook_row) = self.hook;
        let mut top = area.height.saturating_sub(self.rows()) / 2;
        if top + hook_row == 0 && area.height > self.rows() {
            top = HOOK_LINE_MIN_ROWS;
        }
        let x = area.x + area.width.saturating_sub(self.width) / 2 + ART_INSET;
        let hook_y = (area.y + top + hook_row).min(area.bottom() - 1);
        let y = hook_y as i32 - hook_row as i32;
        let hook_x = x + hook_col;
        let line = Style::default().fg(DARK_GRAY).bg(BACKGROUND);
        let inside = |cx: u16, cy: u16| cx < area.right() && cy < area.bottom();
        for line_y in area.y..hook_y {
            if inside(hook_x, line_y) {
                buf[(hook_x, line_y)].set_char(HOOK_LINE).set_style(line);
            }
        }
        match &self.sprite {
            Sprite::Fish(fish) => {
                let sprite = fish.line_sprite();
                let body_y = y + sprite.body_row as i32;
                if let Ok(body_y) = u16::try_from(body_y) {
                    render_fish_sprite(buf, &sprite, x, body_y, area, BACKGROUND);
                }
            }
            Sprite::Cells(rows) => draw_cells(buf, rows, x, y, area),
        }
        if inside(hook_x, hook_y) {
            buf[(hook_x, hook_y)].set_char(HOOK).set_style(line);
        }
    }
}

fn draw_cells(buf: &mut Buffer, rows: &[Vec<(char, Color)>], x: u16, y: i32, area: Rect) {
    for (row_idx, row) in rows.iter().enumerate() {
        let Ok(row_y) = u16::try_from(y + row_idx as i32) else {
            continue;
        };
        if row_y < area.y {
            continue;
        }
        if row_y >= area.bottom() {
            return;
        }
        let mut col = x;
        for &(ch, color) in row {
            let ch_w = UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
            if col + ch_w > area.right() {
                break;
            }
            buf[(col, row_y)]
                .set_char(ch)
                .set_fg(color)
                .set_bg(BACKGROUND);
            col += ch_w;
        }
    }
}

enum CardLine {
    Headline(String),
    Text(String, Color),
    Gap,
    NameInput,
}

impl CardLine {
    fn wrapped(text: &str, width: u16) -> Vec<String> {
        table::wrap_words(text, width.saturating_sub(TEXT_PAD * 2) as usize)
    }

    fn height(&self, width: u16) -> u16 {
        match self {
            CardLine::Headline(text) | CardLine::Text(text, _) => {
                Self::wrapped(text, width).len() as u16
            }
            CardLine::Gap | CardLine::NameInput => 1,
        }
    }

    fn longest_word(lines: &[CardLine]) -> u16 {
        lines
            .iter()
            .filter_map(|line| match line {
                CardLine::Headline(text) | CardLine::Text(text, _) => Some(text),
                CardLine::Gap | CardLine::NameInput => None,
            })
            .flat_map(|text| text.split_whitespace())
            .map(|word| table::visual_width(word) as u16)
            .max()
            .unwrap_or(0)
    }

    fn rows(lines: &[CardLine], width: u16) -> u16 {
        lines.iter().map(|line| line.height(width)).sum()
    }

    fn draw_all(buf: &mut Buffer, body: Rect, lines: &[CardLine], state: &CatchState) {
        if body.height == 0 {
            return;
        }
        let x = body.x + TEXT_PAD;
        let room = body.width.saturating_sub(TEXT_PAD * 2);
        let has_input = lines.iter().any(|line| matches!(line, CardLine::NameInput));
        let squeezed = Self::rows(lines, body.width) > body.height;
        let input_y = body.bottom() - 1;
        let text_bottom = if squeezed && has_input {
            input_y
        } else {
            body.bottom()
        };
        let mut y = body.y;
        for line in lines {
            let fits = |at: u16| at < text_bottom;
            match line {
                CardLine::Gap => y += 1,
                CardLine::NameInput => {
                    let at = if squeezed { input_y } else { y };
                    draw_text_cursor(
                        buf,
                        &state.name_input,
                        state.cursor_visible,
                        x,
                        at,
                        room,
                        BACKGROUND,
                    );
                    y += 1;
                }
                CardLine::Headline(text) | CardLine::Text(text, _) => {
                    let style = line.style();
                    for piece in Self::wrapped(text, body.width) {
                        if !fits(y) {
                            break;
                        }
                        buf.set_stringn(x, y, &piece, room as usize, style);
                        y += 1;
                    }
                }
            }
        }
    }

    fn style(&self) -> Style {
        let base = Style::default().bg(BACKGROUND);
        match self {
            CardLine::Headline(_) => base.fg(WHITE).add_modifier(Modifier::BOLD),
            CardLine::Text(_, color) => base.fg(*color),
            CardLine::Gap | CardLine::NameInput => base.fg(WHITE),
        }
    }
}

fn card_lines(state: &CatchState) -> Vec<CardLine> {
    match &state.loot {
        LootKind::Fish(species) => vec![
            CardLine::Headline(format!("{} captured!", species.display_name())),
            CardLine::Gap,
            CardLine::Text(NAME_LABEL.to_string(), WHITE),
            CardLine::NameInput,
        ],
        LootKind::Cash(cv) => vec![
            CardLine::Headline(CONGRATULATIONS.to_string()),
            CardLine::Text(format!("${} found!", cv.amount()), cv.color()),
        ],
        LootKind::Food(amount) => vec![
            CardLine::Headline(CONGRATULATIONS.to_string()),
            CardLine::Text(format!("+{amount} food!"), WHITE),
        ],
        LootKind::Item(item) => vec![
            CardLine::Text(format!("{}!", item.display_name()), WHITE),
            CardLine::Text(ADDED_TO_INVENTORY.to_string(), WHITE),
        ],
    }
}

fn card_hints(state: &CatchState) -> HintBar {
    let hints = HintBar::new(HINT_CLOSE);
    match &state.loot {
        LootKind::Fish(_) => HintBar::new(HINT_ENTER_CAPTURE),
        LootKind::Cash(_) => hints.aside(CASH_ASIDE),
        LootKind::Food(_) => hints,
        LootKind::Item(item) => hints.aside(format!(
            "You now have {} {}(s)",
            state.item_qty,
            item.display_name()
        )),
    }
}

fn base_card_height(loot: &LootKind) -> u16 {
    if is_necronomicon(loot) {
        NECRO_OVERLAY_HEIGHT
    } else if is_demoncore(loot) {
        DEMON_CORE_OVERLAY_HEIGHT
    } else if is_computer(loot) {
        COMPUTER_OVERLAY_HEIGHT
    } else if is_milk(loot) {
        MILK_OVERLAY_HEIGHT
    } else {
        OVERLAY_HEIGHT
    }
}

fn is_necronomicon(loot: &LootKind) -> bool {
    matches!(
        loot,
        LootKind::Item(ItemKind::Consumable(ConsumableKind::Necronomicon))
    )
}

fn is_milk(loot: &LootKind) -> bool {
    matches!(
        loot,
        LootKind::Item(ItemKind::Consumable(ConsumableKind::Milk(_)))
    )
}

fn left_panel_inner_w(loot: &LootKind, fish: Option<&Fish>) -> u16 {
    match loot {
        LootKind::Fish(_) => fish.map_or(10, |f| f.display_width as u16) + 3,
        LootKind::Cash(_) => 5 + 3,
        LootKind::Food(_) => 11 + 3,
        LootKind::Item(item) => match item {
            ItemKind::Junk(_) => JunkSprite::hook_col() + 3,
            ItemKind::Consumable(ConsumableKind::Necronomicon) => NECRO_LEFT_PANEL_WIDTH,
            ItemKind::Consumable(kind) => kind.panel_inner_w(),
        },
    }
}

fn overlay_title(loot: &LootKind) -> String {
    let catch = match loot {
        LootKind::Fish(_) => "Fish",
        LootKind::Cash(_) => "Cash",
        LootKind::Food(_) => "Food",
        LootKind::Item(ItemKind::Consumable(ConsumableKind::Milk(_))) => "Milk",
        LootKind::Item(item) => item.display_name(),
    };
    format!(" {catch} to the Fishtank! ")
}

fn glistens(loot: &LootKind) -> bool {
    matches!(
        loot,
        LootKind::Item(ItemKind::Consumable(
            ConsumableKind::DemonCore | ConsumableKind::GoldenPearl
        ))
    )
}

fn is_demoncore(loot: &LootKind) -> bool {
    matches!(
        loot,
        LootKind::Item(ItemKind::Consumable(ConsumableKind::DemonCore))
    )
}

fn is_computer(loot: &LootKind) -> bool {
    matches!(
        loot,
        LootKind::Item(ItemKind::Consumable(ConsumableKind::Computer))
    )
}

fn loot_border_color(state: &CatchState) -> Color {
    match &state.loot {
        LootKind::Fish(species) => species
            .config()
            .flavour
            .card_color
            .resolve(WHITE, state.fish.as_ref().map_or(WHITE, |f| f.color)),
        l if is_necronomicon(l) => LIGHT_RED,
        l if is_demoncore(l) => LIGHT_GREEN,
        l if is_computer(l) => GREEN,
        _ => WHITE,
    }
}

fn necro_sprite_rows(eye_open: &[bool]) -> Vec<Vec<(char, Color)>> {
    let c = NECRO_BOOK_COLOR;
    let ec = |i: usize| -> char {
        if eye_open.get(i).copied().unwrap_or(true) {
            NECRO_EYE_OPEN_CHAR
        } else {
            NECRO_EYE_CLOSED_CHAR
        }
    };
    vec![
        vec![
            (' ', c),
            (' ', c),
            (' ', c),
            (' ', c),
            (' ', c),
            ('_', c),
            ('_', c),
            ('_', c),
            ('_', c),
            ('_', c),
            ('_', c),
            ('_', c),
            ('_', c),
        ],
        vec![
            (' ', c),
            (' ', c),
            (' ', c),
            (' ', c),
            ('/', c),
            (' ', c),
            (ec(0), c),
            (' ', c),
            (' ', c),
            (ec(1), c),
            (ec(2), c),
            (' ', c),
            ('/', c),
            (',', c),
        ],
        vec![
            (' ', c),
            (' ', c),
            (' ', c),
            ('/', c),
            (' ', c),
            (ec(3), c),
            (' ', c),
            (ec(4), c),
            (ec(5), c),
            (' ', c),
            (' ', c),
            ('/', c),
            ('/', c),
        ],
        vec![
            (' ', c),
            (' ', c),
            ('/', c),
            (' ', c),
            (ec(6), c),
            (ec(7), c),
            (' ', c),
            (' ', c),
            (ec(8), c),
            (' ', c),
            ('/', c),
            ('/', c),
        ],
        vec![
            (' ', c),
            ('/', c),
            ('_', c),
            ('_', c),
            ('_', c),
            ('_', c),
            ('_', c),
            ('_', c),
            ('_', c),
            ('/', c),
            ('/', c),
        ],
        vec![
            ('(', c),
            ('_', c),
            ('_', c),
            ('_', c),
            ('_', c),
            ('_', c),
            ('_', c),
            ('_', c),
            ('(', c),
            ('/', c),
        ],
    ]
}

fn food_sprite_rows() -> Vec<Vec<(char, Color)>> {
    vec![
        vec![
            (' ', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
        ],
        vec![
            ('/', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('\\', BREAD),
        ],
        vec![
            ('{', CHEESE),
            ('_', CHEESE),
            ('/', CHEESE),
            ('M', BURGER),
            ('M', BURGER),
            ('M', BURGER),
            ('M', BURGER),
            ('M', BURGER),
            ('\\', CHEESE),
            ('_', CHEESE),
            ('}', CHEESE),
        ],
        vec![
            (':', BURGER),
            ('M', BURGER),
            ('M', BURGER),
            ('M', BURGER),
            ('M', BURGER),
            ('\\', CHEESE),
            ('_', CHEESE),
            ('/', CHEESE),
            ('M', BURGER),
            ('M', BURGER),
            (':', BURGER),
        ],
        vec![
            ('\\', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('_', BREAD),
            ('/', BREAD),
        ],
    ]
}

fn consumable_rows(kind: ConsumableKind, state: &CatchState) -> Option<Vec<Vec<(char, Color)>>> {
    Some(match kind {
        ConsumableKind::Coffee => coffee_sprite_rows(state.anim_phase),
        ConsumableKind::Bait => bait_sprite_rows(),
        ConsumableKind::Milk(v) => milk_sprite_rows(v),
        ConsumableKind::DemonCore => demoncore_sprite_rows(state.glisten_phase),
        ConsumableKind::Computer => computer_sprite_rows(),
        ConsumableKind::BlankWafer => blank_wafer_sprite_rows(),
        ConsumableKind::Part(part) => part_sprite_rows(part),
        ConsumableKind::Fabricator => fabricator_sprite_rows(),
        ConsumableKind::BlankBlueprint => blank_blueprint_sprite_rows(),
        ConsumableKind::VoidSeed => void_seed_sprite_rows(),
        ConsumableKind::GoldenPearl => golden_pearl_sprite_rows(state.glisten_phase),
        ConsumableKind::Necronomicon => return None,
    })
}
