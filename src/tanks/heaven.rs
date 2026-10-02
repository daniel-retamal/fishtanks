use rand::RngExt;
use ratatui::style::Color;

use crate::colors::{CYAN, DARK_GRAY, LIGHT_YELLOW};
use crate::sprite::opaque_line;
use crate::tanks::gate::{Gate, GateStyle};
use crate::tanks::soul_wall::SoulWall;

pub const ANGEL_COLOR: Color = LIGHT_YELLOW;
pub const CLOUD_COLOR: Color = CYAN;
pub const SOUL_COLOR: Color = DARK_GRAY;

const GATE_STYLE: GateStyle = GateStyle {
    bars: LIGHT_YELLOW,
    floor: LIGHT_YELLOW,
};

const ANGEL_SPAWN_MIN: f32 = 10.0;
const ANGEL_SPAWN_MAX: f32 = 20.0;
const ANGEL_RISE_MIN: f32 = 2.0;
const ANGEL_RISE_MAX: f32 = 4.5;

pub const ANGEL: [&str; 6] = [
    " _  -=-  _",
    r"( `\(_)/` )",
    r" ( /   \ )",
    r"  (\   /)",
    r"  `/   \`",
    "  '.___.'",
];

pub const CLOUDS_PER_HUNDRED_COLUMNS: f32 = 30.0;
pub const CLOUD_SKY_FRACTION: f32 = 0.6;
const CLOUD_DRIFT_MIN: f32 = 0.2;
const CLOUD_DRIFT_MAX: f32 = 0.9;

const CLOUD_SPRITES: [&[&str]; 6] = [
    &[
        "    ._",
        " .-(`  )",
        ":(      ))",
        "`(    )  ))",
        "  ` __.:'",
    ],
    &[
        "    +_",
        "  (`  ).",
        " (     ).",
        " (       '`.",
        "(      .   )",
        " (..__.:'-'",
    ],
    &["    .--", " .+(   )", " (   .  )", "(   (   ))", " `- __.'"],
    &[
        "     _",
        " .:(`  )`.",
        ":(   .    )",
        "`.  (    ) )",
        "  ` _`  ) )",
        "     (   )",
        "      `-'",
    ],
    &[" .')", "(_  )"],
    &[" ( )", "(_.'"],
];

const BIGGEST_CLOUD: usize = 3;

pub struct Angel {
    pub x: f32,
    pub y: f32,
    rise_speed: f32,
}

impl Angel {
    fn new(width: u16, height: u16, rng: &mut impl RngExt) -> Self {
        let widest = ANGEL
            .iter()
            .map(|row| row.chars().count())
            .max()
            .unwrap_or(0);
        let room = (width as f32 - widest as f32).max(1.0);
        Self {
            x: rng.random_range(0.0..room),
            y: (height as f32 - 1.0).max(0.0),
            rise_speed: rng.random_range(ANGEL_RISE_MIN..ANGEL_RISE_MAX),
        }
    }

    pub fn at(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            rise_speed: ANGEL_RISE_MIN,
        }
    }

    fn tick(&mut self, dt: f32) {
        self.y -= self.rise_speed * dt;
    }

    fn is_gone(&self) -> bool {
        self.y < -(ANGEL.len() as f32)
    }

    pub fn rows(&self) -> Vec<Vec<(char, Color)>> {
        ANGEL
            .iter()
            .map(|row| opaque_line(row, ANGEL_COLOR, |_, _| ANGEL_COLOR))
            .collect()
    }
}

pub struct Cloud {
    pub x: f32,
    altitude: f32,
    sprite: usize,
    drift: f32,
}

impl Cloud {
    fn new(width: u16, rng: &mut impl RngExt) -> Self {
        Self {
            x: rng.random_range(0.0..(width as f32).max(1.0)),
            altitude: rng.random_range(0.0..CLOUD_SKY_FRACTION),
            sprite: rng.random_range(0..CLOUD_SPRITES.len()),
            drift: rng.random_range(CLOUD_DRIFT_MIN..CLOUD_DRIFT_MAX),
        }
    }

    pub fn covering(x: f32, altitude: f32) -> Self {
        Self {
            x,
            altitude,
            sprite: BIGGEST_CLOUD,
            drift: 0.0,
        }
    }

    pub fn top(&self, height: u16) -> i32 {
        (self.altitude * height as f32).floor() as i32
    }

    fn width(&self) -> f32 {
        CLOUD_SPRITES[self.sprite]
            .iter()
            .map(|row| row.chars().count())
            .max()
            .unwrap_or(0) as f32
    }

    fn tick(&mut self, dt: f32, width: u16) {
        self.x += self.drift * dt;
        if self.x > width as f32 {
            self.x = -self.width();
        }
    }

    pub fn rows(&self) -> Vec<Vec<(char, Color)>> {
        CLOUD_SPRITES[self.sprite]
            .iter()
            .map(|row| opaque_line(row, CLOUD_COLOR, |_, _| CLOUD_COLOR))
            .collect()
    }
}

pub fn cloud_count(width: u16) -> usize {
    (width as f32 * CLOUDS_PER_HUNDRED_COLUMNS / 100.0).ceil() as usize
}

pub struct HeavenBackground {
    pub gate: Gate,
    pub clouds: Vec<Cloud>,
    pub angels: Vec<Angel>,
    pub souls: SoulWall,
    angel_timer: f32,
}

impl HeavenBackground {
    pub fn new(width: u16, rng: &mut impl RngExt) -> Self {
        let mut bg = Self {
            gate: Gate::whole(GATE_STYLE, rng),
            clouds: Vec::new(),
            angels: Vec::new(),
            souls: SoulWall::new(SOUL_COLOR),
            angel_timer: rng.random_range(ANGEL_SPAWN_MIN..ANGEL_SPAWN_MAX),
        };
        bg.extend(width, rng);
        bg
    }

    pub fn extend(&mut self, width: u16, rng: &mut impl RngExt) {
        while self.clouds.len() < cloud_count(width) {
            self.clouds.push(Cloud::new(width, rng));
        }
    }

    pub fn tick(&mut self, dt: f32, rng: &mut impl RngExt, width: u16, height: u16) {
        if width == 0 || height == 0 {
            return;
        }
        for cloud in &mut self.clouds {
            cloud.tick(dt, width);
        }
        for angel in &mut self.angels {
            angel.tick(dt);
        }
        self.angels.retain(|angel| !angel.is_gone());
        self.angel_timer -= dt;
        if self.angel_timer <= 0.0 {
            self.angel_timer = rng.random_range(ANGEL_SPAWN_MIN..ANGEL_SPAWN_MAX);
            self.angels.push(Angel::new(width, height, rng));
        }
        self.souls.tick(dt, width, height, rng);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprite::TRANSPARENT;

    const WIDTH: u16 = 80;
    const HEIGHT: u16 = 24;

    #[test]
    fn an_angel_rises_straight_up() {
        let mut rng = rand::rng();
        let mut angel = Angel::new(WIDTH, HEIGHT, &mut rng);
        let x = angel.x;
        let y = angel.y;
        angel.tick(1.0);
        assert_eq!(angel.x, x, "an angel never sways");
        assert!(angel.y < y, "an angel rises");
    }

    #[test]
    fn the_sky_is_as_cloudy_as_its_density_says_and_every_cloud_floats_in_the_upper_sky() {
        let bg = HeavenBackground::new(WIDTH, &mut rand::rng());
        assert_eq!(bg.clouds.len(), cloud_count(WIDTH));
        let highest_bottom = (HEIGHT as f32 * CLOUD_SKY_FRACTION).ceil() as i32;
        assert!(
            bg.clouds
                .iter()
                .all(|cloud| cloud.top(HEIGHT) < highest_bottom)
        );
    }

    #[test]
    fn an_angel_is_solid_from_wing_to_wing() {
        let rows = Angel::at(0.0, 0.0).rows();
        let wings = &rows[2];
        let first = wings.iter().position(|&(ch, _)| ch != TRANSPARENT).unwrap();
        let last = wings
            .iter()
            .rposition(|&(ch, _)| ch != TRANSPARENT)
            .unwrap();
        assert!(wings[first..=last].iter().all(|&(ch, _)| ch != TRANSPARENT));
        assert!(
            rows.iter()
                .flatten()
                .all(|&(_, color)| color == ANGEL_COLOR)
        );
    }
}
