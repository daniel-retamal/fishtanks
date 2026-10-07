use serde::{Deserialize, Serialize};

use crate::economy::{Money, grouped};
use crate::fishes::fish::Fish;

pub mod blackjack;
pub mod bubble;
pub mod derby;
pub mod flip;
pub mod net;
pub mod pufferfish;
pub mod seat;
pub mod spins;
pub mod state;

pub const HUNDREDTHS: Money = 100;
const TENTH: Money = 10;
const SCIENTIFIC_MULTIPLE: Money = 1_000_000;
pub const FISH_PREMIUM: (Money, Money) = (5, 4);
pub const POT_SHARE_PER_CENT: Money = 1;
const PER_CENT: Money = 100;
pub const COMP_EVERY: Money = 5_000;

pub fn premium(worth: Money) -> Money {
    let (num, den) = FISH_PREMIUM;
    worth.saturating_mul(num) / den
}

pub fn pot_share(stake: Money) -> Money {
    stake.saturating_mul(POT_SHARE_PER_CENT) / PER_CENT
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct Multiple(Money);

impl Multiple {
    pub const ZERO: Multiple = Multiple(0);
    pub const ONE: Multiple = Multiple(HUNDREDTHS);

    pub const fn whole(n: Money) -> Self {
        Multiple(n * HUNDREDTHS)
    }

    pub const fn tenths(n: Money) -> Self {
        Multiple(n * TENTH)
    }

    pub const fn hundredths(n: Money) -> Self {
        Multiple(n)
    }

    pub fn between(returned: Money, staked: Money) -> Self {
        if staked == 0 {
            return Multiple::ZERO;
        }
        Multiple(returned.saturating_mul(HUNDREDTHS) / staked)
    }

    pub fn of(self, stake: Money) -> Money {
        let whole = (stake / HUNDREDTHS).saturating_mul(self.0);
        let rest = (stake % HUNDREDTHS) * self.0 / HUNDREDTHS;
        whole.saturating_add(rest)
    }

    pub fn raw(self) -> Money {
        self.0
    }

    pub fn as_f64(self) -> f64 {
        self.0 as f64 / HUNDREDTHS as f64
    }

    pub fn label(self) -> String {
        format!("×{}", self.number())
    }

    pub fn number(self) -> String {
        let whole = self.0 / HUNDREDTHS;
        let cents = self.0 % HUNDREDTHS;
        if whole >= SCIENTIFIC_MULTIPLE {
            return format!("{:.1e}", self.as_f64());
        }
        if cents == 0 {
            return grouped(whole);
        }
        if cents.is_multiple_of(TENTH) {
            return format!("{}.{}", grouped(whole), cents / TENTH);
        }
        format!("{}.{:02}", grouped(whole), cents)
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Casino {
    #[serde(default)]
    pub pot: Money,
    #[serde(default)]
    pub best_win: Money,
    #[serde(default)]
    pub comp_progress: Money,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub on_the_table: Vec<String>,
    #[serde(skip)]
    pub tonight: i128,
}

impl Casino {
    pub fn comps_for(&mut self, wagered: Money) -> u32 {
        self.comp_progress = self.comp_progress.saturating_add(wagered);
        let coffees = self.comp_progress / COMP_EVERY;
        self.comp_progress %= COMP_EVERY;
        u32::try_from(coffees).unwrap_or(u32::MAX)
    }

    pub fn won(&mut self, net: Money) {
        self.best_win = self.best_win.max(net);
        self.tonight = self.tonight.saturating_add(signed(net));
    }

    pub fn lost(&mut self, amount: Money) {
        self.tonight = self.tonight.saturating_sub(signed(amount));
    }

    pub fn feed_the_pot(&mut self, stake: Money) {
        self.pot = self.pot.saturating_add(pot_share(stake));
    }

    pub fn empty_the_pot(&mut self) -> Money {
        std::mem::take(&mut self.pot)
    }
}

pub fn signed(money: Money) -> i128 {
    i128::try_from(money).unwrap_or(i128::MAX)
}

#[derive(Clone)]
pub struct Entrant {
    pub name: String,
    pub worth: Money,
    pub portrait: Fish,
    pub stakeable: bool,
}

pub trait Teller {
    fn spendable(&self) -> Money;
    fn entrants(&self) -> Vec<Entrant>;
    fn entrant(&self, name: &str) -> Option<Entrant> {
        self.entrants()
            .into_iter()
            .find(|e| e.name == name && e.stakeable)
    }
    fn room_for_copies(&self, names: &[String]) -> bool;
    fn room_for_a_prize(&self) -> bool;
}

pub trait House: Teller {
    fn wager(&mut self, amount: Money) -> bool;
    fn pay_out(&mut self, amount: Money);
    fn bury(&mut self, name: &str);
    fn copy(&mut self, name: &str) -> Option<String>;
    fn casino(&mut self) -> &mut Casino;
    fn give_food(&mut self, pellets: u32);
    fn land(&mut self, fish: Fish, name: String) -> Option<String>;
    fn sell_as_bait(&mut self, name: &str) -> Option<Money>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_multiple_reads_like_the_game_writes_numbers() {
        assert_eq!(Multiple::whole(1000).label(), "×1,000");
        assert_eq!(Multiple::tenths(25).label(), "×2.5");
        assert_eq!(Multiple::tenths(2).label(), "×0.2");
        assert_eq!(Multiple::hundredths(133).label(), "×1.33");
        assert_eq!(Multiple::whole(1 << 40).label(), "×1.1e12");
    }

    #[test]
    fn a_multiple_pays_exactly_and_never_overflows() {
        assert_eq!(Multiple::tenths(25).of(100), 250);
        assert_eq!(Multiple::tenths(2).of(7), 1);
        assert_eq!(Multiple::whole(2).of(Money::MAX), Money::MAX);
    }

    #[test]
    fn a_fish_plays_for_a_quarter_more_than_its_worth() {
        assert_eq!(premium(400), 500);
    }

    #[test]
    fn a_coffee_is_poured_for_every_five_thousand_wagered() {
        let mut casino = Casino::default();
        assert_eq!(casino.comps_for(4_999), 0);
        assert_eq!(casino.comps_for(10_001), 3);
        assert_eq!(casino.comp_progress, 0);
    }
}
