use crate::economy::Money;

use super::{House, Multiple, Teller, premium};

const LADDER: [Money; 3] = [1, 2, 5];
const DECADE: Money = 10;
pub const FIRST_STAKE: Money = 10;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Choice {
    Cash,
    Fish(String),
}

#[derive(Clone, Debug)]
pub struct Seat {
    pub cash: Money,
    pub choice: Choice,
}

impl Default for Seat {
    fn default() -> Self {
        Self {
            cash: FIRST_STAKE,
            choice: Choice::Cash,
        }
    }
}

impl Seat {
    pub fn shown_cash(&self, purse: Money) -> Money {
        self.cash.min(purse).max(1)
    }

    pub fn raise(&mut self, purse: Money) {
        self.choice = Choice::Cash;
        let now = self.shown_cash(purse);
        self.cash = rung_above(now).min(purse.max(1));
    }

    pub fn lower(&mut self, purse: Money) {
        self.choice = Choice::Cash;
        self.cash = rung_below(self.shown_cash(purse));
    }

    pub fn all_in(&mut self, purse: Money) {
        self.choice = Choice::Cash;
        self.cash = purse.max(1);
    }

    pub fn pick_fish(&mut self, name: String) {
        self.choice = Choice::Fish(name);
    }

    pub fn fish(&self) -> Option<&str> {
        match &self.choice {
            Choice::Fish(name) => Some(name),
            Choice::Cash => None,
        }
    }

    pub fn can_stake(&self, house: &dyn Teller) -> bool {
        match &self.choice {
            Choice::Fish(name) => house.entrant(name).is_some(),
            Choice::Cash => house.spendable() >= 1,
        }
    }

    pub fn take(&mut self, house: &mut dyn House) -> Option<Round> {
        if let Choice::Fish(name) = self.choice.clone() {
            self.choice = Choice::Cash;
            let entrant = house.entrant(&name)?;
            return Some(Round {
                staked: Staked::Fish {
                    name,
                    worth: entrant.worth,
                    value: premium(entrant.worth),
                },
                extra: 0,
            });
        }
        let amount = self.shown_cash(house.spendable());
        if !house.wager(amount) {
            return None;
        }
        Some(Round {
            staked: Staked::Cash(amount),
            extra: 0,
        })
    }
}

pub fn rung_above(stake: Money) -> Money {
    let mut decade = 1;
    loop {
        for step in LADDER {
            let rung = step.saturating_mul(decade);
            if rung > stake {
                return rung;
            }
        }
        match decade.checked_mul(DECADE) {
            Some(next) => decade = next,
            None => return stake,
        }
    }
}

pub fn rung_below(stake: Money) -> Money {
    let mut best = 1;
    let mut decade: Money = 1;
    while decade <= stake {
        for step in LADDER {
            let rung = step.saturating_mul(decade);
            if rung < stake {
                best = best.max(rung);
            }
        }
        match decade.checked_mul(DECADE) {
            Some(next) => decade = next,
            None => break,
        }
    }
    best
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Staked {
    Cash(Money),
    Fish {
        name: String,
        worth: Money,
        value: Money,
    },
}

impl Staked {
    pub fn value(&self) -> Money {
        match self {
            Staked::Cash(amount) => *amount,
            Staked::Fish { value, .. } => *value,
        }
    }

    pub fn fish(&self) -> Option<&str> {
        match self {
            Staked::Fish { name, .. } => Some(name),
            Staked::Cash(_) => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Round {
    pub staked: Staked,
    pub extra: Money,
}

impl Round {
    pub fn on_the_line(&self) -> Money {
        self.staked.value().saturating_add(self.extra)
    }

    pub fn add(&mut self, house: &mut dyn House, amount: Money) -> bool {
        if !house.wager(amount) {
            return false;
        }
        self.extra = self.extra.saturating_add(amount);
        true
    }

    pub fn settle(self, returned: Money, label: String, house: &mut dyn House) -> RoundResult {
        let staked = self.on_the_line();
        let multiple = Multiple::between(returned, staked);
        let verdict = match self.staked {
            Staked::Cash(_) => {
                house.pay_out(returned);
                if returned > staked {
                    house.casino().won(returned - staked);
                } else {
                    house.casino().lost(staked - returned);
                }
                Verdict::Cash {
                    returned,
                    net: super::signed(returned) - super::signed(staked),
                }
            }
            Staked::Fish { name, worth, value } => {
                if returned >= staked {
                    let winnings = returned - value;
                    house.pay_out(winnings);
                    house.casino().won(returned - staked);
                    Verdict::FishHome { name, winnings }
                } else {
                    house.bury(&name);
                    house.pay_out(returned);
                    house
                        .casino()
                        .lost((worth + self.extra).saturating_sub(returned));
                    Verdict::FishEaten { name, returned }
                }
            }
        };
        RoundResult {
            label,
            multiple,
            verdict,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Verdict {
    Cash {
        returned: Money,
        net: i128,
    },
    FishHome {
        name: String,
        winnings: Money,
    },
    FishEaten {
        name: String,
        returned: Money,
    },
    Clawed {
        prize: Option<String>,
    },
    Doubled {
        cash: Money,
        fish: usize,
        collected: bool,
    },
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RoundResult {
    pub label: String,
    pub multiple: Multiple,
    pub verdict: Verdict,
}

impl RoundResult {
    pub fn on_the_line(&self) -> Option<OnTheLine> {
        match &self.verdict {
            Verdict::Cash { returned, net } if *net > 0 => Some(OnTheLine {
                cash: *returned,
                fish: Vec::new(),
            }),
            Verdict::FishHome { name, winnings } => Some(OnTheLine {
                cash: *winnings,
                fish: vec![name.clone()],
            }),
            _ => None,
        }
    }

    pub fn fish(&self) -> Option<&str> {
        match &self.verdict {
            Verdict::FishHome { name, .. } | Verdict::FishEaten { name, .. } => Some(name),
            _ => None,
        }
    }

    pub fn won(&self) -> bool {
        match &self.verdict {
            Verdict::Cash { net, .. } => *net > 0,
            Verdict::FishHome { .. } => true,
            Verdict::Clawed { prize } => prize.is_some(),
            Verdict::Doubled { collected, .. } => *collected,
            Verdict::FishEaten { .. } => false,
        }
    }

    pub fn is_big(&self) -> bool {
        self.won() && self.multiple >= super::flip::BIG_WIN
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct OnTheLine {
    pub cash: Money,
    pub fish: Vec<String>,
}

impl OnTheLine {
    pub fn is_empty(&self) -> bool {
        self.cash == 0 && self.fish.is_empty()
    }

    pub fn can_double(&self, house: &dyn Teller) -> bool {
        !self.is_empty() && (self.fish.is_empty() || house.room_for_copies(&self.fish))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stake_climbs_one_two_five() {
        let mut stake = 1;
        let mut seen = Vec::new();
        for _ in 0..7 {
            seen.push(stake);
            stake = rung_above(stake);
        }
        assert_eq!(seen, [1, 2, 5, 10, 20, 50, 100]);
        assert_eq!(rung_below(100), 50);
        assert_eq!(rung_below(73), 50);
        assert_eq!(rung_below(1), 1);
    }

    #[test]
    fn the_ladder_has_no_top_but_never_overflows() {
        assert!(rung_above(Money::MAX / 2) > Money::MAX / 2);
        assert_eq!(rung_above(Money::MAX), Money::MAX);
    }
}
