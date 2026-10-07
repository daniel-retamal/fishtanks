use rand::RngExt;

use crate::economy::Money;

use super::seat::OnTheLine;
use super::{House, Multiple, Teller};

pub const BIG_WIN: Multiple = Multiple::whole(10);
pub const WIN_PER_MILLE: u32 = 475;
pub const BELLY_UP_PER_MILLE: u32 = 50;
const PER_MILLE: u32 = 1000;
pub const FLIGHT_SECS: f32 = 1.25;
pub const LOSS_SHOWN_SECS: f32 = 1.4;
const BIG_LADDER_RUNG: u32 = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub fn other(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Landing {
    Facing(Side),
    BellyUp,
}

impl Landing {
    pub fn roll(call: Side, rng: &mut impl RngExt) -> Landing {
        let draw = rng.random_range(0..PER_MILLE);
        if draw < BELLY_UP_PER_MILLE {
            return Landing::BellyUp;
        }
        if draw < BELLY_UP_PER_MILLE + WIN_PER_MILLE {
            return Landing::Facing(call);
        }
        Landing::Facing(call.other())
    }

    pub fn pays_back() -> f64 {
        2.0 * f64::from(WIN_PER_MILLE) / f64::from(PER_MILLE)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FlipPhase {
    Calling,
    Flying {
        call: Side,
        landing: Landing,
        t: f32,
    },
    Won {
        landing: Landing,
    },
    Lost {
        landing: Landing,
        t: f32,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FlipEnd {
    Collected { cash: Money, rungs: u32 },
    Lost,
}

#[derive(Clone, Debug)]
pub struct Flip {
    pub line: OnTheLine,
    pub base: Money,
    pub base_fish: usize,
    pub rung: u32,
    pub phase: FlipPhase,
}

impl Flip {
    pub fn start(line: OnTheLine, house: &mut dyn House) -> Option<Flip> {
        if !line.can_double(house) {
            return None;
        }
        if line.cash > 0 && !house.wager(line.cash) {
            return None;
        }
        Some(Flip {
            base: line.cash,
            base_fish: line.fish.len(),
            line,
            rung: 0,
            phase: FlipPhase::Calling,
        })
    }

    pub fn multiple(&self) -> Money {
        Money::from(2u8).saturating_pow(self.rung)
    }

    pub fn can_call(&self, house: &dyn Teller) -> bool {
        match self.phase {
            FlipPhase::Calling => true,
            FlipPhase::Won { .. } => {
                self.line.fish.is_empty() || house.room_for_copies(&self.line.fish)
            }
            _ => false,
        }
    }

    pub fn call(&mut self, side: Side, house: &dyn Teller, rng: &mut impl RngExt) {
        if !self.can_call(house) {
            return;
        }
        self.phase = FlipPhase::Flying {
            call: side,
            landing: Landing::roll(side, rng),
            t: 0.0,
        };
    }

    pub fn can_collect(&self) -> bool {
        matches!(self.phase, FlipPhase::Won { .. })
            || (matches!(self.phase, FlipPhase::Calling) && self.rung == 0)
    }

    pub fn collect(&mut self, house: &mut dyn House) -> Option<FlipEnd> {
        if !self.can_collect() {
            return None;
        }
        house.pay_out(self.line.cash);
        if self.line.cash > self.base {
            house.casino().won(self.line.cash - self.base);
        }
        Some(FlipEnd::Collected {
            cash: self.line.cash,
            rungs: self.rung,
        })
    }

    pub fn tick(&mut self, dt: f32, house: &mut dyn House) -> Option<FlipEnd> {
        match &mut self.phase {
            FlipPhase::Flying { call, landing, t } => {
                *t += dt;
                if *t < FLIGHT_SECS {
                    return None;
                }
                let (call, landing) = (*call, *landing);
                if landing == Landing::Facing(call) {
                    self.double_up(house);
                    self.phase = FlipPhase::Won { landing };
                } else {
                    self.lose(house);
                    self.phase = FlipPhase::Lost { landing, t: 0.0 };
                }
                None
            }
            FlipPhase::Lost { t, .. } => {
                *t += dt;
                (*t >= LOSS_SHOWN_SECS).then_some(FlipEnd::Lost)
            }
            _ => None,
        }
    }

    fn double_up(&mut self, house: &mut dyn House) {
        self.rung += 1;
        self.line.cash = self.line.cash.saturating_mul(2);
        let copies: Vec<String> = self
            .line
            .fish
            .clone()
            .iter()
            .filter_map(|name| house.copy(name))
            .collect();
        self.line.fish.extend(copies);
    }

    fn lose(&mut self, house: &mut dyn House) {
        for name in std::mem::take(&mut self.line.fish) {
            house.bury(&name);
        }
        house.casino().lost(self.base);
        self.line.cash = 0;
    }

    pub fn ends_big(end: FlipEnd) -> bool {
        matches!(end, FlipEnd::Collected { rungs, .. } if rungs >= BIG_LADDER_RUNG)
    }

    pub fn at_risk(&self) -> Vec<String> {
        self.line.fish.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flip_pays_back_ninety_five_percent() {
        assert!((Landing::pays_back() - 0.95).abs() < 1e-9);
    }

    #[test]
    fn a_flip_lands_on_the_call_about_as_often_as_it_promises() {
        let mut rng = rand::rng();
        let rolls = 40_000;
        let hits = (0..rolls)
            .filter(|_| Landing::roll(Side::Left, &mut rng) == Landing::Facing(Side::Left))
            .count();
        let share = hits as f64 / rolls as f64;
        assert!((share - 0.475).abs() < 0.02, "{share}");
    }
}
