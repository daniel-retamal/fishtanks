use std::collections::VecDeque;

use rand::RngExt;
use ratatui::style::Color;

use crate::fishes::fish::{Direction, Fish};
use crate::fishes::quirk::FORGET_TURN_MEAN_SECS;
use crate::util::exponential_event;

pub const SOUL_WALL_LIMIT: usize = 75;

const SOUL_DRIFT_MIN: f32 = 0.6;
const SOUL_DRIFT_MAX: f32 = 1.8;
const SOUL_TOP_ROW: f32 = 1.0;
const SOUL_FLOOR_MARGIN: f32 = 2.0;

pub struct Soul {
    pub fish: Fish,
    drift: f32,
}

impl Soul {
    fn new(mut fish: Fish, x: f32, height: u16, rng: &mut impl RngExt) -> Self {
        let drift = rng.random_range(SOUL_DRIFT_MIN..SOUL_DRIFT_MAX);
        fish.position.x = x;
        fish.position.y = soul_row(height, rng);
        Self { fish, drift }
    }

    fn arriving(fish: Fish, width: u16, height: u16, rng: &mut impl RngExt) -> Self {
        let room = (width as f32 - fish.display_width as f32).max(1.0);
        let x = rng.random_range(0.0..room);
        let mut soul = Self::new(fish, x, height, rng);
        soul.fish.facing = random_direction(rng);
        soul
    }

    fn returning(fish: Fish, width: u16, height: u16, rng: &mut impl RngExt) -> Self {
        let facing = random_direction(rng);
        let x = match facing {
            Direction::Right => -(fish.display_width as f32),
            Direction::Left => width as f32,
        };
        let mut soul = Self::new(fish, x, height, rng);
        soul.fish.facing = facing;
        soul
    }

    fn tick(&mut self, dt: f32, width: u16, height: u16, rng: &mut impl RngExt) {
        if self.fish.forgets_it_died() {
            self.roam(dt, width, height, rng);
            return;
        }
        let step = self.drift * dt;
        match self.fish.facing {
            Direction::Left => self.fish.position.x -= step,
            Direction::Right => self.fish.position.x += step,
        }
        self.fish.position.y = self.fish.position.y.min(lowest_soul_row(height));
    }

    fn roam(&mut self, dt: f32, width: u16, height: u16, rng: &mut impl RngExt) {
        let room = (width as f32 - self.fish.display_width as f32).max(0.0);
        let turns = exponential_event(rng, FORGET_TURN_MEAN_SECS, dt);
        let at_wall = match self.fish.facing {
            Direction::Left => self.fish.position.x <= 0.0,
            Direction::Right => self.fish.position.x >= room,
        };
        if turns || at_wall {
            self.fish.facing = match self.fish.facing {
                Direction::Left => Direction::Right,
                Direction::Right => Direction::Left,
            };
        }
        let step = self.fish.speed * dt;
        let x = match self.fish.facing {
            Direction::Left => self.fish.position.x - step,
            Direction::Right => self.fish.position.x + step,
        };
        self.fish.position.x = x.clamp(0.0, room);
        self.fish.position.y = self.fish.position.y.min(lowest_soul_row(height));
    }

    fn is_gone(&self, width: u16) -> bool {
        if self.fish.forgets_it_died() {
            return false;
        }
        let x = self.fish.position.x;
        match self.fish.facing {
            Direction::Left => x + self.fish.display_width as f32 <= 0.0,
            Direction::Right => x >= width as f32,
        }
    }
}

fn random_direction(rng: &mut impl RngExt) -> Direction {
    if rng.random::<bool>() {
        Direction::Left
    } else {
        Direction::Right
    }
}

fn lowest_soul_row(height: u16) -> f32 {
    (height as f32 - SOUL_FLOOR_MARGIN).max(SOUL_TOP_ROW)
}

fn soul_row(height: u16, rng: &mut impl RngExt) -> f32 {
    let lowest = lowest_soul_row(height);
    if lowest <= SOUL_TOP_ROW {
        return SOUL_TOP_ROW;
    }
    rng.random_range(SOUL_TOP_ROW..=lowest).floor()
}

pub struct SoulWall {
    color: Color,
    drifting: Vec<Soul>,
    waiting: VecDeque<Fish>,
}

impl SoulWall {
    pub fn new(color: Color) -> Self {
        Self {
            color,
            drifting: Vec::new(),
            waiting: VecDeque::new(),
        }
    }

    pub fn color(&self) -> Color {
        self.color
    }

    pub fn receive(&mut self, fish: Fish, width: u16, height: u16, rng: &mut impl RngExt) {
        if self.drifting.len() >= SOUL_WALL_LIMIT {
            self.waiting.push_back(fish);
            return;
        }
        self.drifting.push(Soul::arriving(fish, width, height, rng));
    }

    pub fn release(&mut self, name: &str, width: u16, height: u16, rng: &mut impl RngExt) -> bool {
        if let Some(pos) = self.drifting.iter().position(|soul| soul.fish.name == name) {
            self.drifting.remove(pos);
            self.call_in(width, height, rng);
            return true;
        }
        let Some(pos) = self.waiting.iter().position(|fish| fish.name == name) else {
            return false;
        };
        self.waiting.remove(pos);
        true
    }

    pub fn take_all(&mut self) -> Vec<Fish> {
        let drifting = std::mem::take(&mut self.drifting)
            .into_iter()
            .map(|soul| soul.fish);
        drifting.chain(std::mem::take(&mut self.waiting)).collect()
    }

    pub fn tick(&mut self, dt: f32, width: u16, height: u16, rng: &mut impl RngExt) {
        if width == 0 || height == 0 {
            return;
        }
        for soul in &mut self.drifting {
            soul.tick(dt, width, height, rng);
        }
        let (gone, staying): (Vec<Soul>, Vec<Soul>) = std::mem::take(&mut self.drifting)
            .into_iter()
            .partition(|soul| soul.is_gone(width));
        self.drifting = staying;
        self.waiting.extend(gone.into_iter().map(|soul| soul.fish));
        self.call_in(width, height, rng);
    }

    fn call_in(&mut self, width: u16, height: u16, rng: &mut impl RngExt) {
        while self.drifting.len() < SOUL_WALL_LIMIT {
            let Some(fish) = self.waiting.pop_front() else {
                return;
            };
            self.drifting
                .push(Soul::returning(fish, width, height, rng));
        }
    }

    pub fn drifting(&self) -> impl Iterator<Item = &Fish> {
        self.drifting.iter().map(|soul| &soul.fish)
    }

    pub fn drifting_mut(&mut self) -> impl Iterator<Item = &mut Fish> {
        self.drifting.iter_mut().map(|soul| &mut soul.fish)
    }

    pub fn all(&self) -> impl Iterator<Item = &Fish> {
        self.drifting().chain(self.waiting.iter())
    }

    pub fn len(&self) -> usize {
        self.drifting.len() + self.waiting.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::DARK_GRAY;
    use crate::fishes::species::FishSpecies;

    const WIDTH: u16 = 80;
    const HEIGHT: u16 = 24;

    fn dead(name: &str) -> Fish {
        Fish::new(
            FishSpecies::Salmon,
            name.to_string(),
            0.0,
            0.0,
            &mut rand::rng(),
        )
    }

    fn wall() -> SoulWall {
        SoulWall::new(DARK_GRAY)
    }

    #[test]
    fn the_wall_shows_at_most_its_limit_and_the_rest_wait_their_turn() {
        let mut rng = rand::rng();
        let mut wall = wall();
        let total = SOUL_WALL_LIMIT + 5;
        for n in 0..total {
            wall.receive(dead(&format!("Soul{n}")), WIDTH, HEIGHT, &mut rng);
        }
        assert_eq!(wall.drifting().count(), SOUL_WALL_LIMIT);
        assert_eq!(wall.len(), total);
    }

    #[test]
    fn a_soul_that_drifts_off_the_wall_lets_a_waiting_one_in() {
        let mut rng = rand::rng();
        let mut wall = wall();
        for n in 0..=SOUL_WALL_LIMIT {
            wall.receive(dead(&format!("Soul{n}")), WIDTH, HEIGHT, &mut rng);
        }
        let waiting = format!("Soul{SOUL_WALL_LIMIT}");
        assert!(wall.drifting().all(|fish| fish.name != waiting));
        wall.drifting[0].fish.position.x = WIDTH as f32 * 2.0;
        wall.drifting[0].fish.facing = Direction::Right;
        wall.tick(0.0, WIDTH, HEIGHT, &mut rng);
        assert!(wall.drifting().any(|fish| fish.name == waiting));
        assert_eq!(wall.drifting().count(), SOUL_WALL_LIMIT);
        assert_eq!(wall.len(), SOUL_WALL_LIMIT + 1);
    }

    #[test]
    fn a_lone_soul_that_drifts_off_comes_back() {
        let mut rng = rand::rng();
        let mut wall = wall();
        wall.receive(dead("Ann"), WIDTH, HEIGHT, &mut rng);
        wall.drifting[0].fish.position.x = -100.0;
        wall.drifting[0].fish.facing = Direction::Left;
        wall.tick(0.0, WIDTH, HEIGHT, &mut rng);
        assert_eq!(wall.drifting().count(), 1);
        let fish = wall.drifting().next().unwrap();
        assert!(!wall.drifting[0].is_gone(WIDTH), "{}", fish.position.x);
    }

    #[test]
    fn a_released_soul_leaves_the_wall_wherever_it_was() {
        let mut rng = rand::rng();
        let mut wall = wall();
        for n in 0..=SOUL_WALL_LIMIT {
            wall.receive(dead(&format!("Soul{n}")), WIDTH, HEIGHT, &mut rng);
        }
        assert!(wall.release("Soul0", WIDTH, HEIGHT, &mut rng));
        let called_in = format!("Soul{SOUL_WALL_LIMIT}");
        assert!(wall.drifting().any(|fish| fish.name == called_in));
        assert!(wall.release(&called_in, WIDTH, HEIGHT, &mut rng));
        assert_eq!(wall.len(), SOUL_WALL_LIMIT - 1);
        assert!(!wall.all().any(|fish| fish.name == "Soul0"));
        assert!(!wall.release("Nobody", WIDTH, HEIGHT, &mut rng));
    }

    #[test]
    fn taking_every_soul_empties_the_wall_drifting_and_waiting_alike() {
        let mut rng = rand::rng();
        let mut wall = wall();
        let total = SOUL_WALL_LIMIT + 2;
        for n in 0..total {
            wall.receive(dead(&format!("Soul{n}")), WIDTH, HEIGHT, &mut rng);
        }
        assert_eq!(wall.take_all().len(), total);
        assert!(wall.is_empty());
    }
}
