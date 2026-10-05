use std::f32::consts::PI;

use rand::RngExt;

use super::adorn::posed_size;
use super::{
    DY_FRACTION, Direction, Fish, FishState, ZOOMIE_COOLDOWN_MAX, ZOOMIE_COOLDOWN_MIN,
    ZOOMIE_DURATION_MAX, ZOOMIE_DURATION_MIN, ZOOMIE_SPEED_MULTIPLIER,
};
use crate::fishes::habits::{Crawl, Lurk, Side};
use crate::fishes::quirk::BONES_ZOOMIE_STRETCH;
use crate::fishes::species::{Habit, Locomotion, Zoomie};
use crate::util::sample_exponential;

pub const PUFF_SECS: f32 = 4.0;
const GLIDE_CLIMB_SPEED: f32 = 20.0;
const GLIDE_SPEED_FRACTION: f32 = 0.6;
const HOP_SECS: f32 = 1.2;
const HOP_HEIGHT: f32 = 3.0;
const HOP_SPEED_MULT: f32 = 3.0;
const BOUNCE_DY_RATIO: f32 = 0.5;
const CORNER_SLACK: f32 = 0.75;
const LURK_MIN_SECS: f32 = 20.0;
const LURK_MAX_SECS: f32 = 60.0;
const PEEK_SECS: f32 = 6.0;
const PEEK_CELLS: f32 = 3.0;
const BACKWARDS_MEAN_SECS: f32 = 40.0;
const BACKWARDS_SECS: f32 = 2.5;
const SHY_DART_MULT: f32 = 4.0;
pub const SHY_HIDING_SECS: f32 = 8.0;

impl Fish {
    pub(super) fn locomote(&mut self, dt: f32, speed_mult: f32, width: u16, height: u16) {
        match self.locomotion() {
            Locomotion::Swim => self.swim(dt, speed_mult, width, height),
            Locomotion::Through => self.swim_through(dt, speed_mult, width, height),
            Locomotion::Bounce => self.bounce_about(dt, speed_mult, width, height),
            Locomotion::Floor | Locomotion::Sideways => self.walk(dt, speed_mult, width, height),
            Locomotion::Glass => self.crawl(dt, speed_mult, width, height),
        }
    }

    fn swim(&mut self, dt: f32, speed_mult: f32, width: u16, height: u16) {
        if self.habits.hiding > 0.0 {
            self.hide(dt, speed_mult, width, height);
            return;
        }
        let sink_dy = self.circadian_sink_dy();
        self.position.x += self.velocity.dx * dt * speed_mult;
        self.position.y += (self.velocity.dy + sink_dy) * dt * speed_mult;
        self.bounce_walls(width, height);
        self.keep_backwards(dt);
    }

    fn keep_backwards(&mut self, dt: f32) {
        if self.habit() != Some(Habit::Backwards) {
            return;
        }
        if self.habits.backwards > 0.0 {
            self.habits.backwards = (self.habits.backwards - dt).max(0.0);
            self.facing = if self.velocity.dx < 0.0 {
                Direction::Right
            } else {
                Direction::Left
            };
            if self.habits.backwards == 0.0 {
                self.facing = self.facing.flip();
            }
            return;
        }
        if !matches!(self.state, FishState::Idle) {
            return;
        }
        let mut rng = rand::rng();
        let clock = self
            .habits
            .backwards_clock
            .get_or_insert_with(|| sample_exponential(&mut rng, BACKWARDS_MEAN_SECS));
        *clock -= dt;
        if *clock > 0.0 {
            return;
        }
        self.habits.backwards_clock = None;
        self.habits.backwards = BACKWARDS_SECS;
        self.velocity.dx = -self.velocity.dx;
    }

    pub fn startle(&mut self, width: u16) {
        if self.habit() != Some(Habit::Shy) || self.is_pinned() {
            return;
        }
        let toward_left = self.position.x + self.display_width as f32 / 2.0 < width as f32 / 2.0;
        self.facing = if toward_left {
            Direction::Left
        } else {
            Direction::Right
        };
        self.habits.hiding = SHY_HIDING_SECS;
    }

    fn hide(&mut self, dt: f32, speed_mult: f32, width: u16, height: u16) {
        let (min_x, max_x, _, _) = self.position_bounds(width, height);
        let dart = self.speed * SHY_DART_MULT * speed_mult * dt;
        self.position.x = if self.facing_left() {
            (self.position.x - dart).max(min_x)
        } else {
            (self.position.x + dart).min(max_x.max(min_x))
        };
    }

    fn swim_through(&mut self, dt: f32, speed_mult: f32, width: u16, height: u16) {
        let mut rng = rand::rng();
        match self.habits.lurk {
            Some(Lurk::Out(secs)) => {
                let left = secs - dt;
                if left > 0.0 {
                    self.habits.lurk = Some(Lurk::Out(left));
                    return;
                }
                self.peek_in(width, height, &mut rng);
                return;
            }
            Some(Lurk::Peek(secs)) => {
                let left = secs - dt;
                if left > 0.0 {
                    self.habits.lurk = Some(Lurk::Peek(left));
                    return;
                }
                self.habits.lurk = None;
                self.velocity.dx = if self.facing_left() {
                    -self.speed
                } else {
                    self.speed
                };
                self.velocity.dy = 0.0;
            }
            None => {}
        }
        self.position.x += self.velocity.dx * dt * speed_mult;
        self.position.y += (self.velocity.dy + self.circadian_sink_dy()) * dt * speed_mult;
        let (_, _, min_y, max_y) = self.position_bounds(width, height);
        if self.position.y < min_y {
            self.position.y = min_y;
            self.velocity.dy = self.velocity.dy.abs();
        } else if self.position.y > max_y {
            self.position.y = max_y.max(min_y);
            self.velocity.dy = -self.velocity.dy.abs();
        }
        let gone_left = self.position.x + self.display_width as f32 <= 0.0;
        let gone_right = self.position.x >= width as f32;
        if gone_left || gone_right {
            self.habits.lurk = Some(Lurk::Out(rng.random_range(LURK_MIN_SECS..LURK_MAX_SECS)));
        }
    }

    fn peek_in(&mut self, width: u16, height: u16, rng: &mut impl RngExt) {
        let from_left = rng.random::<bool>();
        let (_, _, min_y, max_y) = self.position_bounds(width, height);
        self.position.y = if max_y > min_y {
            rng.random_range(min_y..=max_y)
        } else {
            min_y
        };
        let body = self.display_width as f32;
        if from_left {
            self.facing = Direction::Right;
            self.position.x = PEEK_CELLS - body;
        } else {
            self.facing = Direction::Left;
            self.position.x = width as f32 - PEEK_CELLS;
        }
        self.velocity.dx = 0.0;
        self.velocity.dy = 0.0;
        self.habits.lurk = Some(Lurk::Peek(PEEK_SECS));
    }

    pub fn is_out_of_sight(&self, width: u16) -> bool {
        matches!(self.habits.lurk, Some(Lurk::Out(_)))
            || self.position.x + self.display_width as f32 <= 0.0
            || self.position.x >= width as f32
    }

    fn bounce_about(&mut self, dt: f32, speed_mult: f32, width: u16, height: u16) {
        self.habits.cornered = false;
        if self.velocity.dx == 0.0 || self.velocity.dy == 0.0 {
            let mut rng = rand::rng();
            let across = if rng.random::<bool>() { 1.0 } else { -1.0 };
            let down = if rng.random::<bool>() { 1.0 } else { -1.0 };
            self.velocity.dx = across * self.speed;
            self.velocity.dy = down * self.speed * BOUNCE_DY_RATIO;
        }
        self.position.x += self.velocity.dx * dt * speed_mult;
        self.position.y += self.velocity.dy * dt * speed_mult;
        let (min_x, max_x, min_y, max_y) = self.position_bounds(width, height);
        let max_x = max_x.max(min_x);
        let max_y = max_y.max(min_y);
        let hit_x = self.position.x <= min_x || self.position.x >= max_x;
        let hit_y = self.position.y <= min_y || self.position.y >= max_y;
        if hit_x {
            self.velocity.dx = if self.position.x <= min_x {
                self.velocity.dx.abs()
            } else {
                -self.velocity.dx.abs()
            };
            self.position.x = self.position.x.clamp(min_x, max_x);
        }
        if hit_y {
            self.velocity.dy = if self.position.y <= min_y {
                self.velocity.dy.abs()
            } else {
                -self.velocity.dy.abs()
            };
            self.position.y = self.position.y.clamp(min_y, max_y);
        }
        if hit_x || hit_y {
            self.habits.hue = self.habits.hue.wrapping_add(1);
            let near_x =
                self.position.x - min_x <= CORNER_SLACK || max_x - self.position.x <= CORNER_SLACK;
            let near_y =
                self.position.y - min_y <= CORNER_SLACK || max_y - self.position.y <= CORNER_SLACK;
            self.habits.cornered = near_x && near_y;
        }
        self.facing = if self.velocity.dx < 0.0 {
            Direction::Left
        } else {
            Direction::Right
        };
    }

    pub fn floor_y(&self, height: u16) -> f32 {
        let sprite = self.line_sprite();
        let below = sprite.rows.len() - 1 - sprite.body_row;
        (height as f32 - 1.0 - below as f32).max(0.0)
    }

    fn walk(&mut self, dt: f32, speed_mult: f32, width: u16, height: u16) {
        if self.habit() == Some(Habit::Ambush) {
            self.position.y = self.floor_y(height);
            return;
        }
        self.velocity.dy = 0.0;
        self.position.x += self.velocity.dx * dt * speed_mult;
        let (min_x, max_x, _, _) = self.position_bounds(width, height);
        let max_x = max_x.max(min_x);
        if self.position.x < min_x || self.position.x > max_x {
            self.position.x = self.position.x.clamp(min_x, max_x);
            self.velocity.dx = -self.velocity.dx;
            if self.locomotion() == Locomotion::Floor {
                self.facing = self.facing.flip();
            }
        }
        let rise = match self.habits.hop {
            Some(t) => {
                let t = t + dt;
                if t >= HOP_SECS {
                    self.habits.hop = None;
                    0.0
                } else {
                    self.habits.hop = Some(t);
                    HOP_HEIGHT * (PI * t / HOP_SECS).sin()
                }
            }
            None => 0.0,
        };
        self.position.y = self.floor_y(height) - rise;
    }

    fn crawl(&mut self, dt: f32, speed_mult: f32, width: u16, height: u16) {
        let mut crawl = match self.habits.crawl {
            Some(crawl) => crawl,
            None => self.settle_on_glass(width, height),
        };
        if self.is_zooming() {
            self.habits.crawl = Some(crawl);
            self.place_on_glass(crawl, width, height);
            return;
        }
        let step = self.speed * speed_mult * dt;
        let forward = match crawl.side {
            Side::Floor => !crawl.clockwise,
            Side::Ceiling => crawl.clockwise,
            Side::Left => !crawl.clockwise,
            Side::Right => crawl.clockwise,
        };
        crawl.along += if forward { step } else { -step };
        let limit = self.glass_limit(crawl, width, height);
        if crawl.along < 0.0 || crawl.along > limit {
            crawl = self.turn_corner(crawl, width, height);
        }
        self.habits.crawl = Some(crawl);
        self.place_on_glass(crawl, width, height);
    }

    fn glass_extent(&self, crawl: Crawl) -> (f32, f32) {
        let mut probe = self.clone();
        probe.habits.crawl = Some(crawl);
        probe.facing = if crawl.heading_left() || !matches!(crawl.side, Side::Floor | Side::Ceiling)
        {
            Direction::Left
        } else {
            Direction::Right
        };
        let (w, h) = posed_size(&probe.line_sprite());
        (w as f32, h as f32)
    }

    fn glass_limit(&self, crawl: Crawl, width: u16, height: u16) -> f32 {
        let (w, h) = self.glass_extent(crawl);
        match crawl.side {
            Side::Floor | Side::Ceiling => (width as f32 - w).max(0.0),
            Side::Left | Side::Right => (height as f32 - h).max(0.0),
        }
    }

    fn turn_corner(&self, crawl: Crawl, width: u16, height: u16) -> Crawl {
        let clockwise = crawl.clockwise;
        let next = |side: Side| Crawl {
            side,
            clockwise,
            along: 0.0,
        };
        let at_end = |me: &Fish, side: Side| {
            let mut next = next(side);
            next.along = me.glass_limit(next, width, height);
            next
        };
        let going_low = crawl.along < 0.0;
        match (crawl.side, going_low) {
            (Side::Floor, true) => at_end(self, Side::Left),
            (Side::Floor, false) => at_end(self, Side::Right),
            (Side::Left, true) => next(Side::Ceiling),
            (Side::Left, false) => next(Side::Floor),
            (Side::Ceiling, true) => next(Side::Left),
            (Side::Ceiling, false) => next(Side::Right),
            (Side::Right, true) => at_end(self, Side::Ceiling),
            (Side::Right, false) => at_end(self, Side::Floor),
        }
    }

    fn settle_on_glass(&self, width: u16, height: u16) -> Crawl {
        let x = self.position.x;
        let y = self.position.y;
        let distances = [
            (height as f32 - 1.0 - y, Side::Floor),
            (y, Side::Ceiling),
            (x, Side::Left),
            (width as f32 - x, Side::Right),
        ];
        let side = distances
            .iter()
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .map_or(Side::Floor, |&(_, side)| side);
        let along = match side {
            Side::Floor | Side::Ceiling => x,
            Side::Left | Side::Right => y,
        };
        let crawl = Crawl {
            side,
            clockwise: self.pattern_seed.is_multiple_of(2),
            along,
        };
        let limit = self.glass_limit(crawl, width, height);
        Crawl {
            along: along.clamp(0.0, limit),
            ..crawl
        }
    }

    fn place_on_glass(&mut self, crawl: Crawl, width: u16, height: u16) {
        self.facing = match crawl.side {
            Side::Floor | Side::Ceiling if !crawl.heading_left() => Direction::Right,
            _ => Direction::Left,
        };
        let sprite = self.line_sprite();
        let (w, h) = posed_size(&sprite);
        let body_row = sprite.body_row as f32;
        let (x, y) = match crawl.side {
            Side::Floor => (crawl.along, height as f32 - h as f32 + body_row),
            Side::Ceiling => (crawl.along, body_row),
            Side::Left => (0.0, crawl.along + body_row),
            Side::Right => (width as f32 - w as f32, crawl.along + body_row),
        };
        self.position.x = x.max(0.0);
        self.position.y = y.max(0.0);
    }

    pub fn turn_on_glass(&mut self) {
        if let Some(crawl) = self.habits.crawl.as_mut() {
            crawl.clockwise = !crawl.clockwise;
        }
    }

    pub(super) fn launch_zoomie(&mut self) {
        let mut rng = rand::rng();
        self.zoomie_timer = rng.random_range(ZOOMIE_COOLDOWN_MIN..ZOOMIE_COOLDOWN_MAX);
        if self.adornments().puff {
            self.habits.puffed = PUFF_SECS;
            return;
        }
        let duration = rng.random_range(ZOOMIE_DURATION_MIN..ZOOMIE_DURATION_MAX);
        let zoomie_speed = self.speed * ZOOMIE_SPEED_MULTIPLIER;
        let ahead = match self.facing {
            Direction::Left => -1.0,
            Direction::Right => 1.0,
        };
        let style = self.zoomie();
        let will_turn = match style {
            Zoomie::Vertical => {
                self.velocity.dx = 0.0;
                self.velocity.dy = -zoomie_speed;
                false
            }
            Zoomie::Still | Zoomie::None => {
                self.velocity.dx = 0.0;
                self.velocity.dy = 0.0;
                false
            }
            Zoomie::Glide => {
                self.velocity.dx = ahead * zoomie_speed * GLIDE_SPEED_FRACTION;
                self.velocity.dy = -GLIDE_CLIMB_SPEED;
                false
            }
            Zoomie::Hop => {
                self.velocity.dx = ahead * self.speed * HOP_SPEED_MULT;
                self.habits.hop = Some(0.0);
                false
            }
            Zoomie::Lunge => {
                self.velocity.dx = ahead * zoomie_speed;
                self.velocity.dy = 0.0;
                false
            }
            Zoomie::Burst | Zoomie::Ink => {
                self.velocity.dx = ahead * zoomie_speed;
                self.velocity.dy = 0.0;
                self.habits.inked = style == Zoomie::Ink;
                rng.random::<bool>()
            }
            Zoomie::Scatter => {
                self.velocity.dx = ahead * self.speed;
                self.velocity.dy = 0.0;
                false
            }
        };
        let duration = match style {
            Zoomie::Hop => HOP_SECS,
            Zoomie::Scatter => duration * BONES_ZOOMIE_STRETCH,
            _ => duration,
        };
        self.state = FishState::Zoomie {
            time_remaining: duration,
            total_duration: duration,
            will_turn,
            has_turned: false,
        };
    }

    pub fn flee_from(&mut self, x: f32) {
        if self.is_pinned() || self.is_wired() || self.is_asleep() || !self.zoomie().zooms() {
            return;
        }
        let center = self.position.x + self.display_width as f32 / 2.0;
        self.facing = if center < x {
            Direction::Left
        } else {
            Direction::Right
        };
        self.launch_zoomie();
    }

    pub fn wander_direction(&mut self, rng: &mut impl RngExt) {
        match self.locomotion() {
            Locomotion::Bounce => {}
            Locomotion::Glass => {
                if rng.random::<bool>() {
                    self.turn_on_glass();
                }
            }
            Locomotion::Floor | Locomotion::Sideways => {
                let sign = if rng.random::<bool>() { 1.0 } else { -1.0 };
                self.velocity.dx = sign * self.speed;
                self.velocity.dy = 0.0;
                if self.locomotion() == Locomotion::Floor {
                    self.facing = if sign < 0.0 {
                        Direction::Left
                    } else {
                        Direction::Right
                    };
                }
            }
            Locomotion::Swim | Locomotion::Through => {
                let angle = rng.random::<f32>() * std::f32::consts::TAU;
                let dy_frac = self
                    .unfish_state
                    .as_ref()
                    .map_or(DY_FRACTION, |us| us.kind.dy_fraction());
                self.velocity.dx = angle.cos() * self.speed;
                self.velocity.dy = angle.sin() * self.speed * dy_frac;
                if self.velocity.dx != 0.0 {
                    self.facing = if self.velocity.dx < 0.0 {
                        Direction::Left
                    } else {
                        Direction::Right
                    };
                }
            }
        }
    }
}
