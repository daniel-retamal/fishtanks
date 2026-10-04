use std::collections::VecDeque;

use super::fish::Direction;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Floor,
    Ceiling,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug)]
pub struct Crawl {
    pub side: Side,
    pub clockwise: bool,
    pub along: f32,
}

impl Crawl {
    pub fn heading_left(self) -> bool {
        match self.side {
            Side::Floor => self.clockwise,
            Side::Ceiling => !self.clockwise,
            Side::Left | Side::Right => false,
        }
    }

    pub fn climbing(self) -> bool {
        match self.side {
            Side::Left => self.clockwise,
            Side::Right => !self.clockwise,
            Side::Floor | Side::Ceiling => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lurk {
    Out(f32),
    Peek(f32),
}

#[derive(Clone, Debug)]
pub struct Chase {
    pub target: String,
    pub secs: f32,
}

#[derive(Clone, Debug)]
pub struct Echo {
    pub text: String,
    pub strength: f32,
    pub delay: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Footstep {
    pub at: f32,
    pub x: f32,
    pub y: f32,
    pub facing: Direction,
    pub eating: bool,
    pub zooming: bool,
}

#[derive(Clone, Default)]
pub struct Habits {
    pub crawl: Option<Crawl>,
    pub hop: Option<f32>,
    pub puffed: f32,
    pub hue: usize,
    pub hue_clock: f32,
    pub cornered: bool,
    pub inked: bool,
    pub flash: Option<f32>,
    pub lit: f32,
    pub lurk: Option<Lurk>,
    pub backwards: f32,
    pub backwards_clock: Option<f32>,
    pub chase: Option<Chase>,
    pub chase_clock: Option<f32>,
    pub echo: Option<Echo>,
    pub muse: Option<String>,
    pub footsteps: VecDeque<Footstep>,
    pub clock: f32,
    pub hiding: f32,
    pub resting: f32,
    pub rest_clock: f32,
    pub escape_clock: Option<f32>,
    pub alert: f32,
    pub sated: f32,
    pub bumped: bool,
    pub mimed: bool,
    pub occupied: Vec<(i32, i32)>,
}
