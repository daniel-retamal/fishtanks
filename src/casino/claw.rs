use rand::RngExt;

use crate::economy::{Money, Rarity};
use crate::fishes::fish::{Direction, Fish, LineSprite};
use crate::fishes::species::SizeCategory;
use crate::fishes::toy::{Depth, FittedPart, Line, Material, Shelf, Signature, ToyColor, ToyState};

pub const GLASS_W: i32 = 58;
pub const GLASS_H: i32 = 11;
pub const RAIL: i32 = 0;
pub const PARKED: i32 = 1;
pub const ZONE_TOP: i32 = 4;
pub const GRAVEL: i32 = GLASS_H - 1;
pub const CHUTE_W: i32 = 9;
pub const WALL_X: i32 = GLASS_W - CHUTE_W - 1;
pub const DROP_X: i32 = WALL_X + CHUTE_W / 2 + 1;
pub const CHUTE_TOP: i32 = 5;
pub const PARK_X: i32 = 3;
pub const PRIZES: usize = 8;
pub const MIN_GRIP: f64 = 0.07;
pub const MAX_GRIP: f64 = 0.60;
pub const PAYBACK: f64 = 0.95;
const CLEAN_GRAB: f64 = 1.0;
const OFF_CENTRE_GRAB: f64 = 0.6;
const TIP_GRAB: f64 = 0.25;
const CLEAN_SHARE: f32 = 1.0 / 3.0;
const OFF_CENTRE_SHARE: f32 = 0.75;
const STAGE_SHARES: [f64; 3] = [0.2, 0.3, 0.5];
pub const STEER_SECS: f32 = 15.0;
const STEP_DOWN: f32 = 0.05;
const STEP_SIDE: f32 = 0.035;
const OVER_SECS: f32 = 0.7;
const GRAB_PAUSE: f32 = 0.4;
const RESTOCK_SECS: f32 = 0.9;
const SWAY_REACH: f32 = 1.6;
const SWAY_SPIN: f32 = 9.0;
const SWAY_FADE: f32 = 0.32;
const SWAY_STILL_SECS: f32 = 3.0;
const PART_SHARE: u32 = 5;
const PLACING_TRIES: usize = 24;
const GLASS_SPEED: (f32, f32) = (1.2, 2.4);
const TURN_EVERY_SECS: f32 = 16.0;
const HEAVY_GRIPS: [f64; 2] = [0.5, 0.2];

pub fn price() -> Money {
    Money::from(Rarity::Common.catch_worth() / 2)
}

pub fn best_grip(worth: Money) -> f64 {
    let share = PAYBACK * price() as f64 / worth.max(1) as f64;
    share.clamp(MIN_GRIP, MAX_GRIP)
}

pub fn weight(worth: Money) -> usize {
    let grip = best_grip(worth);
    if grip >= HEAVY_GRIPS[0] {
        1
    } else if grip >= HEAVY_GRIPS[1] {
        2
    } else if grip > MIN_GRIP {
        3
    } else {
        4
    }
}

pub fn part_worth(part: FittedPart) -> Money {
    let multiplier = f64::from(part.part.rarity().value_multiplier());
    (price() as f64 * multiplier).round() as Money
}

#[derive(Clone)]
pub enum Goods {
    Toy(Box<Fish>),
    Part(FittedPart),
}

impl Goods {
    pub fn worth(&self) -> Money {
        match self {
            Goods::Toy(fish) => fish.sell_value(),
            Goods::Part(part) => part_worth(*part),
        }
    }

    pub fn name(&self) -> String {
        match self {
            Goods::Toy(fish) => fish
                .toy
                .as_ref()
                .map_or_else(String::new, |toy| toy.title()),
            Goods::Part(part) => part.name(),
        }
    }

    pub fn toy(&self) -> Option<&ToyState> {
        match self {
            Goods::Toy(fish) => fish.toy.as_deref(),
            Goods::Part(_) => None,
        }
    }

    pub fn needs_room(&self) -> bool {
        matches!(self, Goods::Toy(_))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Motion {
    Resting,
    Arriving,
    Falling,
    Held,
    Chute,
}

#[derive(Clone)]
pub struct Prize {
    pub goods: Goods,
    pub x: f32,
    pub y: i32,
    pub home: i32,
    pub facing: Direction,
    pub speed: f32,
    pub motion: Motion,
    pub fall: f32,
    extent: (i32, i32, i32),
}

pub const CAPSULE: &str = "(O)";

impl Prize {
    pub fn sprite(&self, clock: f32) -> Option<LineSprite> {
        let Goods::Toy(fish) = &self.goods else {
            return None;
        };
        let mut posed = (**fish).clone();
        posed.facing = self.facing;
        posed.habits.toy_clock = clock;
        Some(posed.line_sprite())
    }

    fn measured(mut self) -> Self {
        self.extent = match self.sprite(0.0) {
            Some(sprite) => (
                sprite.width() as i32,
                sprite.body_row as i32,
                (sprite.rows.len() - 1 - sprite.body_row) as i32,
            ),
            None => (CAPSULE.chars().count() as i32, 0, 0),
        };
        self
    }

    pub fn width(&self) -> i32 {
        self.extent.0
    }

    fn spans(&self, other: &Prize) -> bool {
        let (_, above, below) = self.extent;
        let (_, other_above, other_below) = other.extent;
        self.left() < other.left() + other.width()
            && other.left() < self.left() + self.width()
            && self.y - above <= other.y + other_below
            && other.y - other_above <= self.y + below
    }

    pub fn left(&self) -> i32 {
        self.x.round() as i32
    }

    fn covers(&self, col: i32) -> bool {
        col >= self.left() && col < self.left() + self.width()
    }

    fn rows_around(&self) -> (i32, i32) {
        (self.extent.1, self.extent.2)
    }

    fn depth(&self) -> Depth {
        match &self.goods {
            Goods::Toy(fish) => fish.toy.as_ref().map_or(Depth::Floor, |toy| toy.depth()),
            Goods::Part(_) => Depth::Floor,
        }
    }

    fn pace(&self, clock: f32) -> f32 {
        match &self.goods {
            Goods::Toy(fish) => fish.toy.as_ref().map_or(0.0, |toy| toy.pace(clock)),
            Goods::Part(_) => 0.0,
        }
    }

    fn lane(&self, rng: &mut impl RngExt) -> i32 {
        let (above, below) = self.rows_around();
        let top = ZONE_TOP + above;
        let floor = GRAVEL - 1 - below;
        let bottom = (floor - 1).max(top);
        match self.depth() {
            Depth::Floor => floor,
            Depth::Surface => top,
            Depth::Upper(_) | Depth::Anywhere => rng.random_range(top..=bottom),
        }
    }

    fn wander(&mut self, dt: f32, clock: f32) {
        let width = self.width() as f32;
        let ahead = match self.facing {
            Direction::Left => -1.0,
            Direction::Right => 1.0,
        };
        self.x += ahead * self.speed * self.pace(clock) * dt;
        let max_x = (WALL_X as f32 - width).max(1.0);
        if self.x < 1.0 {
            self.x = 1.0;
            self.facing = Direction::Right;
        }
        if self.x > max_x {
            self.x = max_x;
            self.facing = Direction::Left;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Ready,
    Steering,
    Dropping,
    Grabbed,
    Lifting,
    Riding,
    Over,
    Releasing,
    Returning,
}

#[derive(Clone)]
struct Held {
    prize: Prize,
    offset: i32,
    grip: f64,
}

#[derive(Clone)]
pub struct Hand {
    pub rail: i32,
    pub x: i32,
    pub y: i32,
    pub open: bool,
    sway: f32,
    sway_t: f32,
    sway_dir: f32,
    held: Vec<Held>,
    stage: usize,
    half: i32,
}

impl Hand {
    fn parked() -> Self {
        Self {
            rail: PARK_X,
            x: PARK_X,
            y: PARKED,
            open: true,
            sway: 0.0,
            sway_t: SWAY_STILL_SECS,
            sway_dir: 0.0,
            held: Vec::new(),
            stage: 0,
            half: 0,
        }
    }
}

pub enum Won {
    Toy(Box<Fish>),
    Part(FittedPart),
}

#[derive(Clone)]
pub struct Claw {
    pub prizes: Vec<Prize>,
    pub hand: Hand,
    pub phase: Phase,
    pub clock: f32,
    pub steer_left: f32,
    acc: f32,
    restock: Vec<f32>,
    turns: f32,
}

impl Claw {
    pub fn stock(shelf: &Shelf, rng: &mut impl RngExt) -> Self {
        let mut claw = Self {
            prizes: Vec::new(),
            hand: Hand::parked(),
            phase: Phase::Ready,
            clock: 0.0,
            steer_left: 0.0,
            acc: 0.0,
            restock: Vec::new(),
            turns: 0.0,
        };
        for _ in 0..PRIZES {
            let prize = claw.new_prize(shelf, rng);
            claw.prizes.push(prize);
        }
        claw
    }

    fn new_prize(&self, shelf: &Shelf, rng: &mut impl RngExt) -> Prize {
        let goods = draw_goods(shelf, rng);
        let facing = if rng.random::<bool>() {
            Direction::Left
        } else {
            Direction::Right
        };
        let mut prize = Prize {
            goods,
            x: 0.0,
            y: ZONE_TOP,
            home: ZONE_TOP,
            facing,
            speed: rng.random_range(GLASS_SPEED.0..GLASS_SPEED.1),
            motion: Motion::Resting,
            fall: 0.0,
            extent: (0, 0, 0),
        }
        .measured();
        let width = prize.width();
        prize.home = prize.lane(rng);
        prize.y = prize.home;
        let room = (WALL_X - width).max(2) as f32;
        prize.x = rng.random_range(1.0..room);
        for _ in 0..PLACING_TRIES {
            if !self.crowds(&prize, None) {
                break;
            }
            prize.x = rng.random_range(1.0..room);
        }
        prize
    }

    fn crowds(&self, prize: &Prize, skip: Option<usize>) -> bool {
        self.prizes.iter().enumerate().any(|(i, other)| {
            Some(i) != skip && other.motion == Motion::Resting && prize.spans(other)
        })
    }

    pub fn is_busy(&self) -> bool {
        self.phase != Phase::Ready
    }

    pub fn shown_x(&self) -> i32 {
        match self.phase {
            Phase::Ready | Phase::Steering => self.hand.rail + self.hand.sway.round() as i32,
            _ => self.hand.x,
        }
    }

    pub fn paid(&mut self) {
        if self.phase != Phase::Ready {
            return;
        }
        self.phase = Phase::Steering;
        self.steer_left = STEER_SECS;
    }

    pub fn steer(&mut self, dx: i32) {
        if self.phase != Phase::Steering {
            return;
        }
        self.hand.rail = (self.hand.rail + dx).clamp(1, WALL_X - 2);
        self.hand.sway_t = 0.0;
        self.hand.sway_dir = -(dx as f32);
    }

    pub fn drop_hand(&mut self) {
        if self.phase != Phase::Steering {
            return;
        }
        self.hand.x = self.shown_x().clamp(1, WALL_X - 2);
        self.hand.rail = self.hand.x;
        self.hand.sway_t = SWAY_STILL_SECS;
        self.hand.sway = 0.0;
        self.phase = Phase::Dropping;
        self.acc = 0.0;
    }

    pub fn hovered(&self, room: bool) -> Option<&Prize> {
        if !matches!(self.phase, Phase::Ready | Phase::Steering) {
            return None;
        }
        let col = self.shown_x();
        self.prizes
            .iter()
            .filter(|p| p.motion == Motion::Resting && (room || !p.goods.needs_room()))
            .filter(|p| p.covers(col))
            .min_by_key(|p| p.y)
    }

    pub fn hold_fast(&mut self) {
        for held in &mut self.hand.held {
            held.grip = 1.0;
        }
    }

    pub fn held(&self) -> Vec<&Prize> {
        self.hand.held.iter().map(|h| &h.prize).collect()
    }

    pub fn abandon(&mut self) {
        for held in self.hand.held.drain(..) {
            let mut prize = held.prize;
            prize.motion = Motion::Falling;
            prize.x = prize.x.min((WALL_X - prize.width()) as f32).max(1.0);
            self.prizes.push(prize);
        }
        self.hand = Hand::parked();
        self.phase = Phase::Ready;
    }

    pub fn tick(
        &mut self,
        dt: f32,
        shelf: &Shelf,
        room: bool,
        rng: &mut impl RngExt,
    ) -> Option<Won> {
        self.clock += dt;
        self.turns += dt;
        self.tick_sway(dt);
        self.tick_prizes(dt, rng);
        self.tick_restock(dt, shelf, rng);
        self.acc += dt;
        match self.phase {
            Phase::Ready => None,
            Phase::Steering => {
                self.steer_left -= dt;
                if self.steer_left <= 0.0 {
                    self.steer_left = 0.0;
                    self.drop_hand();
                }
                None
            }
            Phase::Dropping => {
                self.descend(room);
                None
            }
            Phase::Grabbed => {
                if self.acc > GRAB_PAUSE {
                    self.phase = Phase::Lifting;
                    self.acc = 0.0;
                }
                None
            }
            Phase::Lifting => {
                self.lift(rng);
                None
            }
            Phase::Riding => {
                self.ride(rng);
                None
            }
            Phase::Over => {
                if self.acc > OVER_SECS {
                    self.acc = 0.0;
                    if self.test(rng) {
                        self.release();
                    }
                }
                None
            }
            Phase::Releasing => self.tick_chute(),
            Phase::Returning => {
                self.go_home();
                None
            }
        }
    }

    fn tick_sway(&mut self, dt: f32) {
        let hand = &mut self.hand;
        hand.sway_t += dt;
        hand.sway = if hand.sway_t < SWAY_STILL_SECS {
            hand.sway_dir
                * SWAY_REACH
                * (SWAY_SPIN * hand.sway_t).cos()
                * (-hand.sway_t / SWAY_FADE).exp()
        } else {
            0.0
        };
    }

    fn tick_prizes(&mut self, dt: f32, rng: &mut impl RngExt) {
        let clock = self.clock;
        let turn_now = self.turns > TURN_EVERY_SECS;
        if turn_now {
            self.turns = 0.0;
        }
        for i in 0..self.prizes.len() {
            if self.prizes[i].motion == Motion::Resting {
                let before = self.prizes[i].clone();
                let crowded_before = self.crowds(&before, Some(i));
                self.prizes[i].wander(dt, clock);
                let moved = self.prizes[i].clone();
                if !crowded_before && self.crowds(&moved, Some(i)) {
                    self.prizes[i].x = before.x;
                    self.prizes[i].facing = before.facing.flip();
                }
                if turn_now && rng.random::<bool>() {
                    self.prizes[i].facing = self.prizes[i].facing.flip();
                }
            }
        }
        for prize in &mut self.prizes {
            match prize.motion {
                Motion::Resting => {}
                Motion::Arriving | Motion::Falling => {
                    prize.fall += dt;
                    while prize.fall > STEP_DOWN {
                        prize.fall -= STEP_DOWN;
                        if prize.y < prize.home {
                            prize.y += 1;
                        } else {
                            prize.y = prize.home;
                            prize.motion = Motion::Resting;
                        }
                    }
                }
                Motion::Held | Motion::Chute => {}
            }
        }
    }

    fn tick_restock(&mut self, dt: f32, shelf: &Shelf, rng: &mut impl RngExt) {
        let mut due = 0;
        for wait in &mut self.restock {
            *wait -= dt;
            if *wait <= 0.0 {
                due += 1;
            }
        }
        self.restock.retain(|wait| *wait > 0.0);
        for _ in 0..due {
            let mut prize = self.new_prize(shelf, rng);
            prize.y = ZONE_TOP + prize.rows_around().0;
            prize.motion = Motion::Arriving;
            self.prizes.push(prize);
        }
    }

    fn descend(&mut self, room: bool) {
        while self.acc > STEP_DOWN && self.phase == Phase::Dropping {
            self.acc -= STEP_DOWN;
            let reach = self.hand.y + 2;
            let x = self.hand.x;
            let mut hits: Vec<usize> = self
                .prizes
                .iter()
                .enumerate()
                .filter(|(_, p)| p.motion == Motion::Resting && (room || !p.goods.needs_room()))
                .filter(|(_, p)| p.y == reach && p.covers(x))
                .map(|(i, _)| i)
                .collect();
            if !hits.is_empty() {
                hits.truncate(2);
                hits.sort_unstable_by(|a, b| b.cmp(a));
                for index in hits {
                    let mut prize = self.prizes.remove(index);
                    let offset = x - prize.left();
                    let aim = aim_of(offset, prize.width());
                    let grip = best_grip(prize.goods.worth()) * aim;
                    prize.motion = Motion::Held;
                    self.hand.held.push(Held {
                        prize,
                        offset,
                        grip,
                    });
                }
                self.hand.open = false;
                self.hand.stage = 0;
                self.phase = Phase::Grabbed;
                self.acc = 0.0;
                return;
            }
            if reach >= GRAVEL {
                self.hand.open = false;
                self.phase = Phase::Returning;
                self.acc = 0.0;
                return;
            }
            self.hand.y += 1;
        }
    }

    fn carry(&mut self) {
        let (x, y) = (self.hand.x, self.hand.y);
        for (i, held) in self.hand.held.iter_mut().enumerate() {
            held.prize.x = (x - held.offset) as f32;
            held.prize.y = y + 2 + i as i32;
        }
    }

    fn lift(&mut self, rng: &mut impl RngExt) {
        while self.acc > STEP_DOWN && self.phase == Phase::Lifting {
            self.acc -= STEP_DOWN;
            if self.hand.y > PARKED {
                self.hand.y -= 1;
                self.carry();
                continue;
            }
            if self.test(rng) {
                self.phase = Phase::Riding;
                self.hand.half = (self.hand.x + DROP_X) / 2;
                self.acc = 0.0;
            }
        }
    }

    fn ride(&mut self, rng: &mut impl RngExt) {
        while self.acc > STEP_SIDE && self.phase == Phase::Riding {
            self.acc -= STEP_SIDE;
            if self.hand.x < DROP_X {
                self.hand.x += 1;
                self.carry();
            }
            if self.hand.x == self.hand.half && self.hand.stage == 1 && !self.test(rng) {
                return;
            }
            if self.hand.x >= DROP_X {
                self.hand.stage = self.hand.stage.max(2);
                self.phase = Phase::Over;
                self.acc = 0.0;
            }
        }
    }

    fn test(&mut self, rng: &mut impl RngExt) -> bool {
        let share = STAGE_SHARES[self.hand.stage.min(STAGE_SHARES.len() - 1)];
        let mut kept = Vec::new();
        for held in self.hand.held.drain(..) {
            if rng.random::<f64>() < held.grip.powf(share) {
                kept.push(held);
                continue;
            }
            let mut prize = held.prize;
            prize.motion = Motion::Falling;
            let max_x = (WALL_X - prize.width()) as f32;
            prize.x = prize.x.min(max_x).max(1.0);
            self.prizes.push(prize);
        }
        self.hand.held = kept;
        self.hand.stage += 1;
        if self.hand.held.is_empty() {
            self.hand.open = true;
            self.phase = Phase::Returning;
            self.acc = 0.0;
            return false;
        }
        true
    }

    fn release(&mut self) {
        self.hand.open = true;
        for held in &mut self.hand.held {
            held.prize.motion = Motion::Chute;
            held.prize.fall = 0.0;
        }
        self.phase = Phase::Releasing;
        self.acc = 0.0;
    }

    fn tick_chute(&mut self) -> Option<Won> {
        while self.acc > STEP_DOWN {
            self.acc -= STEP_DOWN;
            for held in &mut self.hand.held {
                held.prize.y += 1;
            }
        }
        let landed = self
            .hand
            .held
            .iter()
            .position(|held| held.prize.y - held.prize.rows_around().0 > GRAVEL)?;
        let held = self.hand.held.remove(landed);
        self.restock.push(RESTOCK_SECS);
        if self.hand.held.is_empty() {
            self.phase = Phase::Returning;
            self.acc = 0.0;
        }
        Some(match held.prize.goods {
            Goods::Toy(fish) => Won::Toy(fish),
            Goods::Part(part) => Won::Part(part),
        })
    }

    fn go_home(&mut self) {
        while self.acc > STEP_SIDE && self.phase == Phase::Returning {
            self.acc -= STEP_SIDE;
            if self.hand.y > PARKED {
                self.hand.y -= 1;
            } else if self.hand.x > PARK_X {
                self.hand.x -= 1;
            } else {
                self.hand = Hand::parked();
                self.phase = Phase::Ready;
            }
        }
    }
}

fn aim_of(offset: i32, width: i32) -> f64 {
    let centre = (width - 1) as f32 / 2.0;
    let off = (offset as f32 - centre).abs() / (width as f32 / 2.0).max(1.0);
    if off <= CLEAN_SHARE {
        CLEAN_GRAB
    } else if off <= OFF_CENTRE_SHARE {
        OFF_CENTRE_GRAB
    } else {
        TIP_GRAB
    }
}

fn toy(state: ToyState, size: SizeCategory, rng: &mut impl RngExt) -> Goods {
    Goods::Toy(Box::new(Fish::new_toy(state, size, rng)))
}

fn pick<T: Copy>(items: &[T], rng: &mut impl RngExt) -> T {
    items[rng.random_range(0..items.len())]
}

pub fn draw_goods(shelf: &Shelf, rng: &mut impl RngExt) -> Goods {
    match Rarity::roll(rng) {
        Rarity::Common => {
            if rng.random_range(0..PART_SHARE) == 0 {
                return Goods::Part(FittedPart::random(Rarity::Common, rng));
            }
            let color = pick(&ToyColor::of_rarity(Rarity::Common), rng);
            let size = SizeCategory::roll(rng);
            toy(ToyState::plain(color, Material::roll(rng)), size, rng)
        }
        Rarity::Rare => match rng.random_range(0..PART_SHARE) {
            0 => Goods::Part(FittedPart::random(Rarity::Rare, rng)),
            1 | 2 => {
                let color = pick(&ToyColor::of_rarity(Rarity::Rare), rng);
                let size = SizeCategory::roll(rng);
                toy(ToyState::plain(color, Material::roll(rng)), size, rng)
            }
            _ => {
                let signatures: Vec<Signature> =
                    Line::ALL.into_iter().flat_map(Line::signatures).collect();
                let signature = pick(&signatures, rng);
                toy(
                    ToyState::signature(signature, false),
                    ToyState::signature_size(),
                    rng,
                )
            }
        },
        Rarity::Legendary => {
            let mut choices: Vec<ToyState> = Vec::new();
            let signatures: Vec<Signature> =
                Line::ALL.into_iter().flat_map(Line::signatures).collect();
            choices.push(ToyState::signature(pick(&signatures, rng), true));
            choices.extend(
                shelf
                    .secrets_unlocked()
                    .into_iter()
                    .map(|secret| ToyState::signature(secret, false)),
            );
            if shelf.golden_unlocked() {
                choices.push(ToyState::golden());
            }
            let state = choices.swap_remove(rng.random_range(0..choices.len()));
            let size = match state.look {
                crate::fishes::toy::Look::Golden => SizeCategory::roll(rng),
                _ => ToyState::signature_size(),
            };
            toy(state, size, rng)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    #[test]
    fn a_go_costs_half_a_common_catch() {
        assert_eq!(price(), Money::from(Rarity::Common.catch_worth() / 2));
    }

    #[test]
    fn the_grip_never_leaves_its_band() {
        assert_eq!(best_grip(1), MAX_GRIP);
        assert_eq!(best_grip(Money::MAX / 2), MIN_GRIP);
        let mid = best_grip(price() * 4);
        assert!(mid > MIN_GRIP && mid < MAX_GRIP);
    }

    #[test]
    fn a_clean_grab_beats_a_tip() {
        assert_eq!(aim_of(4, 9), CLEAN_GRAB);
        assert_eq!(aim_of(1, 9), OFF_CENTRE_GRAB);
        assert_eq!(aim_of(0, 12), TIP_GRAB);
    }

    #[test]
    fn the_glass_is_stocked_and_every_prize_sits_below_the_claw() {
        let mut rng = SmallRng::seed_from_u64(7);
        for _ in 0..30 {
            let claw = Claw::stock(&Shelf::default(), &mut rng);
            assert_eq!(claw.prizes.len(), PRIZES);
            for prize in &claw.prizes {
                let (above, below) = prize.rows_around();
                assert!(prize.y - above >= ZONE_TOP, "{}", prize.goods.name());
                assert!(prize.y + below < GRAVEL, "{}", prize.goods.name());
                assert!(prize.left() + prize.width() <= WALL_X);
            }
        }
    }

    #[test]
    fn the_gold_and_the_secrets_wait_for_the_shelf() {
        let mut rng = SmallRng::seed_from_u64(3);
        for _ in 0..3000 {
            if let Goods::Toy(fish) = draw_goods(&Shelf::default(), &mut rng) {
                let toy = fish.toy.expect("a toy");
                assert_ne!(toy.look, crate::fishes::toy::Look::Golden);
                if let crate::fishes::toy::Look::Signature { signature, .. } = toy.look {
                    assert!(!signature.is_secret());
                }
            }
        }
    }

    #[test]
    fn a_won_prize_falls_down_the_chute_and_another_drops_in() {
        let mut rng = SmallRng::seed_from_u64(11);
        let shelf = Shelf::default();
        let mut claw = Claw::stock(&shelf, &mut rng);
        let prize = claw.prizes.remove(0);
        claw.hand.held.push(Held {
            prize,
            offset: 0,
            grip: 1.0,
        });
        claw.hand.x = DROP_X;
        claw.carry();
        claw.release();
        let mut won = None;
        for _ in 0..400 {
            if let Some(w) = claw.tick(1.0 / 30.0, &shelf, true, &mut rng) {
                won = Some(w);
            }
        }
        assert!(won.is_some());
        assert_eq!(claw.prizes.len(), PRIZES);
        assert_eq!(claw.phase, Phase::Ready);
    }
}
