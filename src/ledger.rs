use std::collections::{BTreeMap, VecDeque};

use crate::economy::Money;

pub const LEDGER_MINUTES: usize = 60;
pub const SECS_PER_MINUTE: f32 = 60.0;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Flow {
    FishSales,
    StockSales,
    TankSales,
    CashCatches,
    Cashfish,
    Blessings,
    Wishes,
    Fish,
    Tanks,
    Food,
    Coffee,
    Bait,
    Robotics,
    Fabrication,
    CasinoWinnings,
    CasinoStakes,
    Godsend,
}

impl Flow {
    pub const ALL: [Flow; 17] = [
        Flow::FishSales,
        Flow::StockSales,
        Flow::TankSales,
        Flow::CashCatches,
        Flow::Cashfish,
        Flow::Blessings,
        Flow::Wishes,
        Flow::Fish,
        Flow::Tanks,
        Flow::Food,
        Flow::Coffee,
        Flow::Bait,
        Flow::Robotics,
        Flow::Fabrication,
        Flow::CasinoWinnings,
        Flow::CasinoStakes,
        Flow::Godsend,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Flow::FishSales => "Fish sales",
            Flow::StockSales => "Stock sales",
            Flow::TankSales => "Tank sales",
            Flow::CashCatches => "Cash catches",
            Flow::Cashfish => "Cashfish",
            Flow::Blessings => "Blessings",
            Flow::Wishes => "Wishes",
            Flow::Fish => "Fish",
            Flow::Tanks => "Tanks",
            Flow::Food => "Food",
            Flow::Coffee => "Coffee",
            Flow::Bait => "Bait",
            Flow::Robotics => "Robotics",
            Flow::Fabrication => "Fabrication",
            Flow::CasinoWinnings => "Casino winnings",
            Flow::CasinoStakes => "Casino stakes",
            Flow::Godsend => "Godsend",
        }
    }

    pub fn is_godsend(self) -> bool {
        self == Flow::Godsend
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Direction {
    In,
    Out,
}

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Tally {
    lines: BTreeMap<(Direction, Flow), Money>,
}

impl Tally {
    fn add(&mut self, direction: Direction, flow: Flow, amount: Money) {
        let line = self.lines.entry((direction, flow)).or_insert(0);
        *line = line.saturating_add(amount);
    }

    fn absorb(&mut self, other: &Tally) {
        for (&(direction, flow), &amount) in &other.lines {
            self.add(direction, flow, amount);
        }
    }

    pub fn line(&self, direction: Direction, flow: Flow) -> Money {
        self.lines.get(&(direction, flow)).copied().unwrap_or(0)
    }

    pub fn total(&self, direction: Direction) -> Money {
        self.lines
            .iter()
            .filter(|((side, _), _)| *side == direction)
            .map(|(_, &amount)| amount)
            .fold(0, Money::saturating_add)
    }

    pub fn flows(&self, direction: Direction) -> impl Iterator<Item = Flow> + '_ {
        self.lines
            .keys()
            .filter(move |(side, _)| *side == direction)
            .map(|&(_, flow)| flow)
    }
}

#[derive(Clone, Debug)]
pub struct Ledger {
    minutes: VecDeque<Tally>,
    into_minute: f32,
    since_launch: Tally,
    secs_open: f32,
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}

impl Ledger {
    pub fn new() -> Self {
        Self {
            minutes: VecDeque::from([Tally::default()]),
            into_minute: 0.0,
            since_launch: Tally::default(),
            secs_open: 0.0,
        }
    }

    pub fn record(&mut self, direction: Direction, flow: Flow, amount: Money) {
        if amount == 0 {
            return;
        }
        if let Some(current) = self.minutes.front_mut() {
            current.add(direction, flow, amount);
        }
        self.since_launch.add(direction, flow, amount);
    }

    pub fn tick(&mut self, dt: f32) {
        self.secs_open += dt;
        self.into_minute += dt;
        while self.into_minute >= SECS_PER_MINUTE {
            self.into_minute -= SECS_PER_MINUTE;
            self.minutes.push_front(Tally::default());
            self.minutes.truncate(LEDGER_MINUTES);
        }
    }

    pub fn last_hour(&self) -> Tally {
        let mut hour = Tally::default();
        for minute in &self.minutes {
            hour.absorb(minute);
        }
        hour
    }

    pub fn since_launch(&self) -> &Tally {
        &self.since_launch
    }

    pub fn minutes_open(&self) -> f32 {
        self.secs_open / SECS_PER_MINUTE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_is_kept_on_the_side_its_money_moved() {
        let mut ledger = Ledger::new();
        ledger.record(Direction::In, Flow::FishSales, 120);
        ledger.record(Direction::Out, Flow::Food, 30);
        ledger.record(Direction::In, Flow::FishSales, 5);
        let hour = ledger.last_hour();
        assert_eq!(hour.line(Direction::In, Flow::FishSales), 125);
        assert_eq!(hour.line(Direction::Out, Flow::Food), 30);
        assert_eq!(hour.total(Direction::In), 125);
        assert_eq!(hour.total(Direction::Out), 30);
        assert_eq!(
            hour.flows(Direction::In).collect::<Vec<_>>(),
            [Flow::FishSales]
        );
    }

    #[test]
    fn nothing_moved_is_no_line() {
        let mut ledger = Ledger::new();
        ledger.record(Direction::In, Flow::Cashfish, 0);
        assert_eq!(ledger.since_launch().flows(Direction::In).count(), 0);
    }

    #[test]
    fn the_last_hour_forgets_what_the_launch_remembers() {
        let mut ledger = Ledger::new();
        ledger.record(Direction::In, Flow::CashCatches, 100);
        ledger.tick(SECS_PER_MINUTE * LEDGER_MINUTES as f32 - 1.0);
        assert_eq!(
            ledger.last_hour().line(Direction::In, Flow::CashCatches),
            100
        );
        ledger.tick(1.0);
        assert_eq!(ledger.last_hour().line(Direction::In, Flow::CashCatches), 0);
        assert_eq!(
            ledger.since_launch().line(Direction::In, Flow::CashCatches),
            100
        );
    }

    #[test]
    fn every_flow_has_its_own_label() {
        let labels: std::collections::BTreeSet<&str> =
            Flow::ALL.iter().map(|f| f.label()).collect();
        assert_eq!(labels.len(), Flow::ALL.len());
    }
}
