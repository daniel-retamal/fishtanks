use serde::{Deserialize, Serialize};
pub trait Purchasable {
    fn buy_price(&self) -> u32;
    fn display_name(&self) -> &str;
}

pub trait Sellable {
    fn sell_price(&self) -> u32;
    fn display_name(&self) -> &str;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rarity {
    Common,
    Rare,
    Legendary,
}

const THOUSANDS: usize = 3;

pub fn grouped(value: u128) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(THOUSANDS) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

const PART_PLATFORM_RARITY: Rarity = Rarity::Legendary;
const GIFT_UNIT_RARITY: Rarity = Rarity::Legendary;
const VALUE_REFERENCE_RARITY: Rarity = Rarity::Common;

impl Rarity {
    pub fn catch_weight(self) -> u32 {
        match self {
            Rarity::Common => 62,
            Rarity::Rare => 22,
            Rarity::Legendary => 4,
        }
    }

    pub fn fish_buy_price(self) -> u32 {
        match self {
            Rarity::Common => 126,
            Rarity::Rare => 341,
            Rarity::Legendary => 7700,
        }
    }

    pub fn part_price(self) -> u32 {
        PART_PLATFORM_RARITY.fish_buy_price() * PART_PLATFORM_RARITY.catch_weight()
            / self.catch_weight()
    }

    pub fn gift_quantity(self) -> u32 {
        self.catch_weight() / GIFT_UNIT_RARITY.catch_weight()
    }

    pub fn catch_worth(self) -> u32 {
        crate::fishes::species::fed_catch_worth(self)
    }

    pub fn value_multiplier(self) -> f32 {
        VALUE_REFERENCE_RARITY.catch_weight() as f32 / self.catch_weight() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loot::ConsumableKind;
    use crate::tank::TankKind;

    const RAREST_FIRST: [Rarity; 3] = [Rarity::Legendary, Rarity::Rare, Rarity::Common];

    #[test]
    fn the_most_exotic_part_costs_the_most_exotic_fish() {
        assert_eq!(
            PART_PLATFORM_RARITY.part_price(),
            PART_PLATFORM_RARITY.fish_buy_price(),
            "a part is priced against the fish it is installed on"
        );
    }

    #[test]
    fn a_part_gets_cheaper_exactly_as_fast_as_it_gets_commoner() {
        for pair in RAREST_FIRST.windows(2) {
            let (rarer, commoner) = (pair[0], pair[1]);
            assert!(
                rarer.part_price() > commoner.part_price(),
                "{commoner:?} hardware must not cost more than {rarer:?} hardware"
            );
            let rarer_worth = rarer.part_price() * rarer.catch_weight();
            let commoner_worth = commoner.part_price() * commoner.catch_weight();
            assert!(
                rarer_worth - commoner_worth < commoner.catch_weight(),
                "{commoner:?} hardware is priced off the drop weight to the nearest dollar"
            );
        }
    }

    #[test]
    fn hardware_is_never_cheaper_than_a_fish_of_its_own_tier() {
        for rarity in RAREST_FIRST {
            assert!(
                rarity.part_price() >= rarity.fish_buy_price(),
                "{rarity:?} hardware undercuts the {rarity:?} fish it rides"
            );
        }
    }

    #[test]
    fn a_gift_of_the_rarest_thing_is_exactly_one_of_it() {
        assert_eq!(GIFT_UNIT_RARITY.gift_quantity(), 1);
    }

    #[test]
    fn a_gift_is_as_many_as_the_sea_gives_while_it_gives_one_legendary() {
        let unit = GIFT_UNIT_RARITY.catch_weight();
        for rarity in RAREST_FIRST {
            let gift = rarity.gift_quantity();
            assert!(
                gift * unit <= rarity.catch_weight() && rarity.catch_weight() < (gift + 1) * unit,
                "a {rarity:?} gift is the {rarity:?} catches in one Legendary's time, rounded down"
            );
        }
        for pair in RAREST_FIRST.windows(2) {
            let (rarer, commoner) = (pair[0], pair[1]);
            assert!(
                commoner.gift_quantity() > rarer.gift_quantity(),
                "a {commoner:?} gift must be bigger than a {rarer:?} one"
            );
        }
    }

    #[test]
    fn an_item_that_only_grows_a_tank_is_never_sold_and_neither_is_that_tank() {
        for seed in ConsumableKind::seeds() {
            let kind = seed.summons_tank().expect("a seed grows a tank");
            assert!(
                !kind.config().buyable,
                "{} is grown from an item, so the catalogue must not sell the tank either",
                kind.display_name()
            );
            assert!(
                !ConsumableKind::bench_stock().contains(&seed),
                "{} only grows a tank, so no shelf may sell it",
                seed.display_name()
            );
        }
    }

    #[test]
    fn the_catalogue_sells_the_ordinary_tanks_and_nothing_you_are_meant_to_find() {
        let catalogue = TankKind::all_buyable();
        assert!(
            catalogue.contains(&TankKind::Base),
            "the plain tank is sold"
        );
        assert!(
            !catalogue.contains(&TankKind::Alien),
            "an Alien Base is taken from an abduction, never bought"
        );
        for kind in catalogue {
            assert!(
                kind.buy_price() > 0,
                "{} is sold, so it has a price",
                kind.display_name()
            );
        }
    }
}

pub type Money = u64;

pub fn reading(money: Money) -> u32 {
    u32::try_from(money).unwrap_or(u32::MAX)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Purse {
    balance: Money,
    bottomless: bool,
}

impl Purse {
    pub const fn holding(balance: Money) -> Self {
        Self {
            balance,
            bottomless: false,
        }
    }

    pub fn balance(self) -> Money {
        self.balance
    }

    pub fn is_bottomless(self) -> bool {
        self.bottomless
    }

    pub fn set_bottomless(&mut self, bottomless: bool) {
        self.bottomless = bottomless;
    }

    pub fn spendable(self) -> Money {
        if self.bottomless {
            return Money::MAX;
        }
        self.balance
    }

    pub fn can_afford(self, cost: impl Into<Money>) -> bool {
        cost.into() <= self.spendable()
    }

    pub(crate) fn spend(&mut self, cost: Money) -> bool {
        if !self.can_afford(cost) {
            return false;
        }
        if !self.bottomless {
            self.balance -= cost;
        }
        true
    }

    pub(crate) fn earn(&mut self, amount: Money) {
        self.balance = self.balance.saturating_add(amount);
    }

    pub fn shown(self) -> Option<Money> {
        (!self.bottomless).then_some(self.balance)
    }
}

#[cfg(test)]
mod purse_tests {
    use super::{Money, Purse, reading};

    #[test]
    fn a_bottomless_purse_pays_for_anything_and_keeps_its_balance() {
        let mut purse = Purse::holding(10);
        purse.set_bottomless(true);
        assert!(purse.spend(Money::MAX));
        assert_eq!(purse.balance(), 10);
        assert_eq!(purse.shown(), None);
        purse.set_bottomless(false);
        assert_eq!(purse.shown(), Some(10));
    }

    #[test]
    fn an_ordinary_purse_refuses_what_it_cannot_pay() {
        let mut purse = Purse::holding(10);
        assert!(!purse.spend(11));
        assert!(purse.spend(10));
        assert_eq!(purse.balance(), 0);
    }

    #[test]
    fn a_purse_keeps_counting_past_the_old_ceiling() {
        let mut purse = Purse::holding(Money::from(u32::MAX));
        purse.earn(1);
        assert_eq!(purse.balance(), Money::from(u32::MAX) + 1);
    }

    #[test]
    fn earnings_never_overflow() {
        let mut purse = Purse::holding(Money::MAX);
        purse.earn(1);
        assert_eq!(purse.balance(), Money::MAX);
    }

    #[test]
    fn a_fortune_too_big_for_a_reading_reads_as_the_largest_one() {
        assert_eq!(reading(700), 700);
        assert_eq!(reading(Money::from(u32::MAX) * 3), u32::MAX);
    }
}
