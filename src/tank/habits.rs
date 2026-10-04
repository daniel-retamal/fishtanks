use rand::RngExt;
use ratatui::style::Color;

use super::{Tank, TankEvent};
use crate::colors::{DARK_GRAY, GOLD_BRIGHT, GRAY, PINK, WHITE};
use crate::entities::bubble::{Bubble, BubblePhase};
use crate::entities::speech::SpeechBubble;
use crate::fishes::fish::{EATING_DURATION, Fish, FishState};
use crate::fishes::habits::{Chase, Echo, Footstep};
use crate::fishes::species::{Habit, Locomotion, Zoomie};
use crate::sprite::TRANSPARENT;
use crate::util::{events_in, sample_exponential};

pub const TRAIL_SECS: f32 = 10.0;
const TRAIL_FADE_SECS: f32 = 3.0;
pub const TRAIL_GLYPH: char = '·';
const TRAIL_DEFAULT: Color = GRAY;
const TRAIL_FADED: Color = DARK_GRAY;
pub const INK_SECS: f32 = 5.0;
const INK_GLYPHS: [char; 3] = ['%', '&', '@'];
pub const INK_COLOR: Color = DARK_GRAY;
const INK_REACH_X: i32 = 2;
const INK_REACH_Y: i32 = 1;
const CONFETTI_GLYPHS: [char; 4] = ['*', '°', '\'', '.'];
const CONFETTI_PIECES: usize = 12;
const SPARK: char = '*';
const SPARK_COLOR: Color = GOLD_BRIGHT;
const CLASH: char = 'x';
const HEART: char = 'ღ';
const HEART_COLOR: Color = PINK;
const SNORES: [char; 2] = ['z', 'Z'];
const SNORES_PER_SEC: f32 = 0.5;
const SNORE_COLOR: Color = WHITE;
const PAIR_REACH: i32 = 2;
const PAIR_REST_SECS: f32 = 3.0;
const PAIR_COOLDOWN_SECS: f32 = 60.0;
const DUEL_REACH: i32 = 2;
const DUEL_COOLDOWN_SECS: f32 = 10.0;
const SHELL_SWAP_COOLDOWN_SECS: f32 = 30.0;
const BUMP_COOLDOWN_SECS: f32 = 1.0;
const FLASH_PERIOD_SECS: f32 = 4.0;
const FLASH_PERIOD_SPREAD: f32 = 0.2;
pub const FLASH_SECS: f32 = 0.4;
const SYNC_NUDGE: f32 = 0.25;
const SYNC_PULL: f32 = 0.4;
const SEED_FRACTION_STEPS: u64 = 1000;
const CHASE_MEAN_SECS: f32 = 90.0;
const CHASE_SECS: f32 = 6.0;
const CHASE_SPEED_MULT: f32 = 2.5;
const SCARE_DISTANCE: f32 = 4.0;
const SHADOW_DELAY_SECS: f32 = 2.0;
const ECHO_DELAY_MIN_SECS: f32 = 2.0;
const ECHO_DELAY_MAX_SECS: f32 = 4.0;
pub const FRESH_SPEECH: f32 = 1.0;
pub const ECHO_FADE: f32 = 0.6;
const AMBUSH_REACH: i32 = 3;
const ALERT_SECS: f32 = 2.0;
const ESCAPE_MEAN_SECS: f32 = 20.0 * 60.0;
const LURE_PULL_RADIUS: f32 = 12.0;
pub const BLIND_SMELL_RADIUS: f32 = 6.0;

#[derive(Clone, Copy, Debug)]
pub struct TrailMark {
    pub x: i32,
    pub y: i32,
    pub ttl: f32,
    pub color: Color,
}

impl TrailMark {
    pub fn color_now(&self) -> Color {
        if self.ttl < TRAIL_FADE_SECS {
            TRAIL_FADED
        } else {
            self.color
        }
    }
}

#[derive(Clone, Debug)]
pub struct InkBlot {
    pub cells: Vec<(i32, i32, char)>,
    pub ttl: f32,
    pub color: Color,
}

#[derive(Clone, Copy)]
struct Footprint {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl Footprint {
    fn of(fish: &Fish) -> Self {
        let sprite = fish.line_sprite();
        let left = fish.position.x as i32;
        let top = fish.position.y as i32 - sprite.body_row as i32;
        let width = sprite.rows.iter().map(Vec::len).max().unwrap_or(0) as i32;
        Self {
            left,
            top,
            right: left + width - 1,
            bottom: top + sprite.rows.len() as i32 - 1,
        }
    }

    fn holds(self, x: i32, y: i32) -> bool {
        (self.left..=self.right).contains(&x) && (self.top..=self.bottom).contains(&y)
    }

    fn gap(self, other: Footprint) -> i32 {
        (other.left - self.right).max(self.left - other.right) - 1
    }

    fn center_x(self) -> f32 {
        (self.left + self.right) as f32 / 2.0
    }
}

fn painted_cells(fish: &Fish) -> Vec<(i32, i32)> {
    let sprite = fish.line_sprite();
    let x0 = fish.position.x as i32;
    let y0 = fish.position.y as i32 - sprite.body_row as i32;
    sprite
        .rows
        .iter()
        .enumerate()
        .flat_map(|(r, row)| {
            row.iter()
                .enumerate()
                .filter(|(_, cell)| cell.0 != TRANSPARENT && cell.0 != ' ')
                .map(move |(c, _)| (x0 + c as i32, y0 + r as i32))
        })
        .collect()
}

fn seed_fraction(seed: u64) -> f32 {
    (seed % SEED_FRACTION_STEPS) as f32 / SEED_FRACTION_STEPS as f32
}

fn flash_period(fish: &Fish) -> f32 {
    FLASH_PERIOD_SECS * (1.0 + FLASH_PERIOD_SPREAD * (seed_fraction(fish.pattern_seed) - 0.5))
}

enum Meeting {
    Pair(usize, usize),
    Duel(usize, usize),
    Swap(usize, usize),
    Bump(usize),
}

impl Tank {
    fn sprinkle(&mut self, x: f32, y: f32, glyph: char, color: Color) {
        let mut rng = rand::rng();
        let mut bubble = Bubble::new(
            x,
            y,
            BubblePhase::rising(&mut rng),
            color,
            Some(glyph),
            &mut rng,
        );
        bubble.poppable = false;
        self.bubbles.push(bubble);
    }

    pub(super) fn tick_habits(&mut self, dt: f32) -> Vec<TankEvent> {
        let mut rng = rand::rng();
        self.lay_trails(dt);
        self.spill_ink(dt, &mut rng);
        self.throw_confetti(&mut rng);
        self.snore(dt, &mut rng);
        self.flash_together(dt);
        self.meet();
        self.chase(dt, &mut rng);
        self.shadow(dt, &mut rng);
        self.pull_to_lures();
        self.walk_to_food();
        self.graze();
        self.ambush();
        self.lunge_through_bubbles(dt);
        let mut events = self.echo_due(dt);
        events.extend(self.escapes(dt, &mut rng));
        events
    }

    fn lay_trails(&mut self, dt: f32) {
        for mark in &mut self.trails {
            mark.ttl -= dt;
        }
        self.trails.retain(|mark| mark.ttl > 0.0);
        for fish in &mut self.fish {
            let Some(tint) = fish.species.config().trail else {
                continue;
            };
            let color = fish
                .wake_color()
                .unwrap_or_else(|| tint.resolve(TRAIL_DEFAULT, fish.color));
            let cells = painted_cells(fish);
            let previous = std::mem::replace(&mut fish.habits.occupied, cells.clone());
            for (x, y) in previous {
                if !cells.contains(&(x, y)) {
                    self.trails.push(TrailMark {
                        x,
                        y,
                        ttl: TRAIL_SECS,
                        color,
                    });
                }
            }
        }
    }

    fn spill_ink(&mut self, dt: f32, rng: &mut impl RngExt) {
        for blot in &mut self.inks {
            blot.ttl -= dt;
        }
        self.inks.retain(|blot| blot.ttl > 0.0);
        for fish in &mut self.fish {
            if !std::mem::take(&mut fish.habits.inked) {
                continue;
            }
            let tail_x = if fish.facing_left() {
                fish.position.x as i32 + fish.display_width as i32
            } else {
                fish.position.x as i32 - 1
            };
            let y = fish.position.y as i32;
            let mut cells = Vec::new();
            for dy in -INK_REACH_Y..=INK_REACH_Y {
                for dx in -INK_REACH_X..=INK_REACH_X {
                    let glyph = INK_GLYPHS[rng.random_range(0..INK_GLYPHS.len())];
                    cells.push((tail_x + dx, y + dy, glyph));
                }
            }
            self.inks.push(InkBlot {
                cells,
                ttl: INK_SECS,
                color: fish.wake_color().unwrap_or(INK_COLOR),
            });
        }
    }

    fn throw_confetti(&mut self, rng: &mut impl RngExt) {
        let corners: Vec<(f32, f32, &'static [Color])> = self
            .fish
            .iter_mut()
            .filter_map(|fish| {
                std::mem::take(&mut fish.habits.cornered).then(|| {
                    (
                        fish.position.x + fish.display_width as f32 / 2.0,
                        fish.position.y,
                        fish.species.config().palette,
                    )
                })
            })
            .collect();
        for (x, y, palette) in corners {
            for piece in 0..CONFETTI_PIECES {
                let glyph = CONFETTI_GLYPHS[piece % CONFETTI_GLYPHS.len()];
                let color = palette[piece % palette.len()];
                let spread =
                    rng.random_range(-(CONFETTI_PIECES as f32)..CONFETTI_PIECES as f32) / 2.0;
                self.sprinkle(x + spread, y, glyph, color);
            }
        }
    }

    fn snore(&mut self, dt: f32, rng: &mut impl RngExt) {
        let mut snores: Vec<(f32, f32)> = Vec::new();
        for fish in &self.fish {
            if !fish.is_asleep() {
                continue;
            }
            let top = fish.position.y - fish.line_sprite().body_row as f32 - 1.0;
            for _ in 0..events_in(rng, SNORES_PER_SEC, dt) {
                snores.push((fish.head_x() as f32, top));
            }
        }
        for cow in &self.cows {
            if !cow.is_asleep() {
                continue;
            }
            let head = cow.position.x + cow.lead() as f32 + 1.0;
            for _ in 0..events_in(rng, SNORES_PER_SEC, dt) {
                snores.push((head, cow.position.y - 1.0));
            }
        }
        for (x, y) in snores {
            let glyph = SNORES[rng.random_range(0..SNORES.len())];
            self.sprinkle(x, y.max(0.0), glyph, SNORE_COLOR);
        }
    }

    fn flash_together(&mut self, dt: f32) {
        let sync: Vec<usize> = (0..self.fish.len())
            .filter(|&i| self.fish[i].habit() == Some(Habit::Sync))
            .collect();
        let mut fired = Vec::new();
        for &i in &sync {
            let fish = &mut self.fish[i];
            let period = flash_period(fish);
            let phase = fish
                .habits
                .flash
                .get_or_insert_with(|| seed_fraction(fish.pattern_seed));
            *phase += dt / period;
            if *phase >= 1.0 {
                *phase = 0.0;
                fish.habits.lit = FLASH_SECS;
                fired.push(i);
            }
        }
        let mut seen = 0;
        while seen < fired.len() {
            let flashes = fired.len() - seen;
            seen = fired.len();
            for &i in &sync {
                if fired.contains(&i) {
                    continue;
                }
                let fish = &mut self.fish[i];
                let Some(phase) = fish.habits.flash.as_mut() else {
                    continue;
                };
                for _ in 0..flashes {
                    *phase += SYNC_NUDGE + SYNC_PULL * *phase;
                }
                if *phase >= 1.0 {
                    *phase = 0.0;
                    fish.habits.lit = FLASH_SECS;
                    fired.push(i);
                }
            }
        }
    }

    fn meet(&mut self) {
        let prints: Vec<Footprint> = self.fish.iter().map(Footprint::of).collect();
        let mut meetings = Vec::new();
        for i in 0..self.fish.len() {
            let a = &self.fish[i];
            if a.habit() == Some(Habit::Blind) && a.habits.rest_clock <= 0.0 {
                let head = (a.head_x(), a.position.y as i32);
                let bumped =
                    (0..self.fish.len()).any(|j| j != i && prints[j].holds(head.0, head.1));
                if bumped || a.habits.bumped {
                    meetings.push(Meeting::Bump(i));
                }
            }
            for j in i + 1..self.fish.len() {
                let b = &self.fish[j];
                let same = a.habit().is_some() && a.habit() == b.habit();
                let rested = a.habits.rest_clock <= 0.0 && b.habits.rest_clock <= 0.0;
                if !same || !rested {
                    continue;
                }
                let gap = prints[i].gap(prints[j]);
                let rows_apart = (a.position.y - b.position.y).abs();
                match a.habit() {
                    Some(Habit::Pair) if gap <= PAIR_REACH && rows_apart <= 1.0 => {
                        meetings.push(Meeting::Pair(i, j));
                    }
                    Some(Habit::Duel) if gap <= DUEL_REACH && rows_apart < 1.0 => {
                        let (left, right) = if prints[i].left < prints[j].left {
                            (a, b)
                        } else {
                            (b, a)
                        };
                        if !left.facing_left() && right.facing_left() {
                            meetings.push(Meeting::Duel(i, j));
                        }
                    }
                    Some(Habit::ShellSwap) if gap <= 0 => meetings.push(Meeting::Swap(i, j)),
                    _ => {}
                }
            }
        }
        for meeting in meetings {
            match meeting {
                Meeting::Pair(i, j) => {
                    let (left, right) = if prints[i].left < prints[j].left {
                        (i, j)
                    } else {
                        (j, i)
                    };
                    self.fish[left].facing = crate::fishes::fish::Direction::Right;
                    self.fish[right].facing = crate::fishes::fish::Direction::Left;
                    for k in [i, j] {
                        self.fish[k].habits.resting = PAIR_REST_SECS;
                        self.fish[k].habits.rest_clock = PAIR_COOLDOWN_SECS;
                    }
                    let x = (prints[i].center_x() + prints[j].center_x()) / 2.0;
                    let y = prints[i].top.min(prints[j].top) as f32 - 1.0;
                    self.sprinkle(x, y.max(0.0), HEART, HEART_COLOR);
                }
                Meeting::Duel(i, j) => {
                    for k in [i, j] {
                        let fish = &mut self.fish[k];
                        fish.velocity.dx = -fish.velocity.dx;
                        fish.facing = if fish.facing_left() {
                            crate::fishes::fish::Direction::Right
                        } else {
                            crate::fishes::fish::Direction::Left
                        };
                        fish.habits.rest_clock = DUEL_COOLDOWN_SECS;
                    }
                    let x = (prints[i].center_x() + prints[j].center_x()) / 2.0;
                    self.sprinkle(x, self.fish[i].position.y, CLASH, SPARK_COLOR);
                }
                Meeting::Swap(i, j) => {
                    let seed = self.fish[i].pattern_seed;
                    self.fish[i].pattern_seed = self.fish[j].pattern_seed;
                    self.fish[j].pattern_seed = seed;
                    for k in [i, j] {
                        self.fish[k].habits.rest_clock = SHELL_SWAP_COOLDOWN_SECS;
                    }
                }
                Meeting::Bump(i) => {
                    let fish = &mut self.fish[i];
                    let head = (fish.head_x() as f32, fish.position.y);
                    if !std::mem::take(&mut fish.habits.bumped) {
                        fish.velocity.dx = -fish.velocity.dx;
                        fish.velocity.dy = -fish.velocity.dy;
                        fish.facing = if fish.facing_left() {
                            crate::fishes::fish::Direction::Right
                        } else {
                            crate::fishes::fish::Direction::Left
                        };
                    }
                    fish.habits.rest_clock = BUMP_COOLDOWN_SECS;
                    self.sprinkle(head.0, head.1, SPARK, SPARK_COLOR);
                }
            }
        }
    }

    fn chase(&mut self, dt: f32, rng: &mut impl RngExt) {
        for i in 0..self.fish.len() {
            if self.fish[i].habit() != Some(Habit::Chase) {
                continue;
            }
            let hunter = &self.fish[i];
            if hunter.is_pinned() || hunter.is_wired() || hunter.is_asleep() {
                continue;
            }
            let Some(chase) = self.fish[i].habits.chase.clone() else {
                let clock = self.fish[i]
                    .habits
                    .chase_clock
                    .get_or_insert_with(|| sample_exponential(rng, CHASE_MEAN_SECS));
                *clock -= dt;
                if *clock > 0.0 {
                    continue;
                }
                self.fish[i].habits.chase_clock = None;
                if let Some(target) = self.nearest_prey(i) {
                    self.fish[i].habits.chase = Some(Chase {
                        target,
                        secs: CHASE_SECS,
                    });
                }
                continue;
            };
            let target = self.fish.iter().position(|f| f.name == chase.target);
            let secs = chase.secs - dt;
            let Some(t) = target.filter(|_| secs > 0.0) else {
                self.fish[i].habits.chase = None;
                continue;
            };
            let (hx, hy) = (self.fish[i].position.x, self.fish[i].position.y);
            let (tx, ty) = (self.fish[t].position.x, self.fish[t].position.y);
            let (dx, dy) = (tx - hx, ty - hy);
            let distance = (dx * dx + dy * dy).sqrt();
            if distance < SCARE_DISTANCE {
                let from = hx + self.fish[i].display_width as f32 / 2.0;
                self.fish[t].flee_from(from);
                self.fish[i].habits.chase = None;
                continue;
            }
            let hunter = &mut self.fish[i];
            let pace = hunter.speed * CHASE_SPEED_MULT / distance.max(f32::EPSILON);
            hunter.velocity.dx = dx * pace;
            hunter.velocity.dy = dy * pace;
            hunter.facing = if dx < 0.0 {
                crate::fishes::fish::Direction::Left
            } else {
                crate::fishes::fish::Direction::Right
            };
            hunter.habits.chase = Some(Chase {
                target: chase.target,
                secs,
            });
        }
    }

    fn nearest_prey(&self, hunter: usize) -> Option<String> {
        let (hx, hy) = (self.fish[hunter].position.x, self.fish[hunter].position.y);
        self.fish
            .iter()
            .enumerate()
            .filter(|&(j, fish)| {
                j != hunter
                    && fish.locomotion().swims()
                    && fish.habit() != Some(Habit::Chase)
                    && !fish.is_pinned()
                    && !fish.is_invisible()
            })
            .min_by(|(_, a), (_, b)| {
                let da = (a.position.x - hx).powi(2) + (a.position.y - hy).powi(2);
                let db = (b.position.x - hx).powi(2) + (b.position.y - hy).powi(2);
                da.partial_cmp(&db).unwrap()
            })
            .map(|(_, fish)| fish.name.clone())
    }

    fn shadow(&mut self, dt: f32, rng: &mut impl RngExt) {
        for i in 0..self.fish.len() {
            if self.fish[i].habit() != Some(Habit::Shadow) || self.fish[i].is_pinned() {
                continue;
            }
            let muse = self.fish[i]
                .habits
                .muse
                .as_ref()
                .and_then(|name| self.fish.iter().position(|f| &f.name == name))
                .filter(|&m| m != i);
            let Some(m) = muse.or_else(|| self.choose_muse(i, rng)) else {
                continue;
            };
            let step = {
                let muse = &self.fish[m];
                Footstep {
                    at: self.fish[i].habits.clock,
                    x: muse.position.x,
                    y: muse.position.y,
                    facing: muse.facing,
                    eating: muse.is_eating(),
                    zooming: muse.is_zooming(),
                }
            };
            let muse_speaks = self.fish[m]
                .speech
                .as_ref()
                .map(|speech| speech.text.chars().count());
            let muse_name = self.fish[m].name.clone();
            let mime = &mut self.fish[i];
            mime.habits.muse = Some(muse_name);
            mime.habits.clock += dt;
            mime.habits.footsteps.push_back(step);
            let due = mime.habits.clock - SHADOW_DELAY_SECS;
            while mime
                .habits
                .footsteps
                .get(1)
                .is_some_and(|next| next.at <= due)
            {
                mime.habits.footsteps.pop_front();
            }
            match muse_speaks {
                Some(len) if !mime.habits.mimed => {
                    mime.speech = Some(SpeechBubble::new(" ".repeat(len)));
                    mime.habits.mimed = true;
                }
                None => mime.habits.mimed = false,
                _ => {}
            }
            let Some(&step) = mime.habits.footsteps.front() else {
                continue;
            };
            if step.at > due {
                continue;
            }
            mime.position.x = step.x;
            mime.position.y = step.y;
            mime.facing = step.facing;
            if step.eating && !mime.is_eating() {
                mime.state = FishState::Eating {
                    time_remaining: EATING_DURATION,
                };
            }
            if step.zooming && !mime.is_zooming() && !step.eating {
                mime.hurry_zoomie();
            }
        }
    }

    fn choose_muse(&self, mime: usize, rng: &mut impl RngExt) -> Option<usize> {
        let candidates: Vec<usize> = (0..self.fish.len())
            .filter(|&j| {
                j != mime
                    && self.fish[j].habit() != Some(Habit::Shadow)
                    && !self.fish[j].is_invisible()
            })
            .collect();
        if candidates.is_empty() {
            return None;
        }
        Some(candidates[rng.random_range(0..candidates.len())])
    }

    fn pull_to_lures(&mut self) {
        if self.sky.daylight {
            return;
        }
        let lures: Vec<(usize, f32, f32, usize)> = self
            .fish
            .iter()
            .enumerate()
            .filter(|(_, fish)| fish.adornments().lure && !fish.is_asleep())
            .map(|(i, fish)| {
                let x = if fish.facing_left() {
                    fish.position.x
                } else {
                    fish.position.x + fish.display_width as f32 - 1.0
                };
                (i, x, fish.position.y, fish.display_width)
            })
            .collect();
        for (lure_idx, lx, ly, size) in lures {
            for (i, fish) in self.fish.iter_mut().enumerate() {
                let drawn = i != lure_idx
                    && fish.display_width < size
                    && fish.locomotion().swims()
                    && matches!(fish.state, FishState::Idle)
                    && !fish.is_pinned()
                    && !fish.adornments().lure;
                if !drawn {
                    continue;
                }
                let (dx, dy) = (lx - fish.position.x, ly - fish.position.y);
                let distance = (dx * dx + dy * dy).sqrt();
                if !(f32::EPSILON..=LURE_PULL_RADIUS).contains(&distance) {
                    continue;
                }
                let pace = fish.speed / distance;
                fish.velocity.dx = dx * pace;
                fish.velocity.dy = dy * pace;
                fish.facing = if dx < 0.0 {
                    crate::fishes::fish::Direction::Left
                } else {
                    crate::fishes::fish::Direction::Right
                };
            }
        }
    }

    fn walk_to_food(&mut self) {
        let settled: Vec<f32> = self
            .food
            .iter()
            .filter(|food| food.settled && !food.eaten)
            .map(|food| food.position.x)
            .collect();
        if settled.is_empty() {
            return;
        }
        for fish in &mut self.fish {
            let walks = fish.locomotion().walks() && fish.habit() != Some(Habit::Ambush);
            if !walks || !matches!(fish.state, FishState::Idle) || !fish.seeks_food() {
                continue;
            }
            let center = fish.position.x + fish.display_width as f32 / 2.0;
            let Some(&target) = settled.iter().min_by(|a, b| {
                (*a - center)
                    .abs()
                    .partial_cmp(&(*b - center).abs())
                    .unwrap()
            }) else {
                continue;
            };
            let toward = if target < center { -1.0 } else { 1.0 };
            fish.velocity.dx = toward * fish.speed;
            if fish.locomotion() == Locomotion::Floor {
                fish.facing = if toward < 0.0 {
                    crate::fishes::fish::Direction::Left
                } else {
                    crate::fishes::fish::Direction::Right
                };
            }
        }
    }

    fn graze(&mut self) {
        for i in 0..self.fish.len() {
            let fish = &self.fish[i];
            let grazes = !fish.locomotion().swims() && fish.habit() != Some(Habit::Ambush);
            if !grazes || fish.is_eating() || !fish.seeks_food() {
                continue;
            }
            let cells = painted_cells(fish);
            let bite = self.food.iter().position(|food| {
                !food.eaten && cells.contains(&(food.position.x as i32, food.position.y as i32))
            });
            if let Some(food_idx) = bite {
                self.bite(i, food_idx);
            }
        }
    }

    fn ambush(&mut self) {
        for i in 0..self.fish.len() {
            let fish = &self.fish[i];
            if fish.habit() != Some(Habit::Ambush) || fish.is_eating() || !fish.seeks_food() {
                continue;
            }
            let print = Footprint::of(fish);
            let bite = self.food.iter().position(|food| {
                let (x, y) = (food.position.x as i32, food.position.y as i32);
                !food.eaten
                    && food.settled
                    && x >= print.left - AMBUSH_REACH
                    && x <= print.right + AMBUSH_REACH
                    && y >= print.top - 1
                    && y <= print.bottom + 1
            });
            if let Some(food_idx) = bite {
                self.bite(i, food_idx);
                self.fish[i].habits.alert = ALERT_SECS;
            }
        }
    }

    fn lunge_through_bubbles(&mut self, dt: f32) {
        let lunges: Vec<(i32, i32, i32)> = self
            .fish
            .iter()
            .filter(|fish| fish.zoomie() == Zoomie::Lunge && fish.is_zooming())
            .map(|fish| {
                let print = Footprint::of(fish);
                let swept = (fish.velocity.dx * dt).abs().ceil() as i32;
                let (left, right) = if fish.velocity.dx < 0.0 {
                    (print.left, print.right + swept)
                } else {
                    (print.left - swept, print.right)
                };
                (fish.position.y as i32, left, right)
            })
            .collect();
        for (row, left, right) in lunges {
            for bubble in &mut self.bubbles {
                let (x, y) = (bubble.position.x as i32, bubble.position.y as i32);
                if bubble.poppable && y == row && (left..=right).contains(&x) {
                    bubble.dead = true;
                }
            }
        }
    }

    fn echo_due(&mut self, dt: f32) -> Vec<TankEvent> {
        let mut events = Vec::new();
        for fish in &mut self.fish {
            let Some(echo) = fish.habits.echo.as_mut() else {
                continue;
            };
            echo.delay -= dt;
            if echo.delay > 0.0 {
                continue;
            }
            let Some(echo) = fish.habits.echo.take() else {
                continue;
            };
            fish.say(echo.text.clone());
            events.push(TankEvent::Echo {
                speaker: fish.name.clone(),
                text: echo.text,
                strength: echo.strength,
            });
        }
        events
    }

    fn escapes(&mut self, dt: f32, rng: &mut impl RngExt) -> Vec<TankEvent> {
        let mut events = Vec::new();
        for fish in &mut self.fish {
            if fish.habit() != Some(Habit::Escape) || fish.is_pinned() {
                continue;
            }
            let clock = fish
                .habits
                .escape_clock
                .get_or_insert_with(|| sample_exponential(rng, ESCAPE_MEAN_SECS));
            *clock -= dt;
            if *clock > 0.0 {
                continue;
            }
            fish.habits.escape_clock = None;
            events.push(TankEvent::Wander {
                fish_name: fish.name.clone(),
            });
        }
        events
    }

    pub fn hear(&mut self, speech: &str, strength: f32, speaker: Option<&str>) {
        let mut rng = rand::rng();
        for fish in &mut self.fish {
            if fish.habit() != Some(Habit::Echo) || fish.habits.echo.is_some() {
                continue;
            }
            if speaker.is_some_and(|name| name == fish.name) {
                continue;
            }
            if rng.random::<f32>() >= strength {
                continue;
            }
            fish.habits.echo = Some(Echo {
                text: speech.to_string(),
                strength: strength * ECHO_FADE,
                delay: rng.random_range(ECHO_DELAY_MIN_SECS..ECHO_DELAY_MAX_SECS),
            });
        }
    }

    pub fn startle(&mut self) {
        let width = self.width;
        for fish in &mut self.fish {
            fish.startle(width);
        }
    }
}
