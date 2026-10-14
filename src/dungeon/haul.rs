use rand::RngExt;
use serde::{Deserialize, Serialize};

use crate::economy::{Rarity, grouped};
use crate::loot::{ConsumableKind, LootPool, StockItem};

use super::floor::FloorSpec;
use super::treasure::{Find, Treasure};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stack {
    pub stock: StockItem,
    pub count: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Haul {
    #[serde(default)]
    pub gold: u64,
    #[serde(default)]
    pub treasures: Vec<Stack>,
}

impl Haul {
    pub fn add(&mut self, stock: StockItem, count: u32) {
        if count == 0 {
            return;
        }
        match self.treasures.iter_mut().find(|t| t.stock == stock) {
            Some(treasure) => treasure.count += count,
            None => self.treasures.push(Stack { stock, count }),
        }
    }

    pub fn absorb(&mut self, other: Haul) {
        self.gold += other.gold;
        for treasure in other.treasures {
            self.add(treasure.stock, treasure.count);
        }
    }

    pub fn count(&self) -> u32 {
        self.treasures.iter().map(|t| t.count).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.gold == 0 && self.treasures.is_empty()
    }

    pub fn spend(&mut self, price: u64) -> bool {
        if self.gold < price {
            return false;
        }
        self.gold -= price;
        true
    }

    pub fn line(&self) -> String {
        format!(
            "${} · {}",
            grouped(u128::from(self.gold)),
            counted(self.count(), TREASURE)
        )
    }
}

pub const TREASURE: (&str, &str) = ("treasure", "treasures");
pub const FISH: (&str, &str) = ("fish", "fish");
pub const MEAL: (&str, &str) = ("meal", "meals");

pub fn counted(count: u32, (one, many): (&str, &str)) -> String {
    match count {
        1 => format!("1 {one}"),
        n => format!("{n} {many}"),
    }
}

pub fn richer(rarity: Rarity) -> Rarity {
    match rarity {
        Rarity::Common => Rarity::Rare,
        Rarity::Rare | Rarity::Legendary => Rarity::Legendary,
    }
}

pub fn poorer(rarity: Rarity) -> Rarity {
    match rarity {
        Rarity::Legendary => Rarity::Rare,
        Rarity::Rare | Rarity::Common => Rarity::Common,
    }
}

pub fn finds(depth: u32) -> Vec<ConsumableKind> {
    LootPool::default_pool()
        .with_native(FloorSpec::of(depth).tank)
        .consumables()
}

pub fn treasure(depth: u32, tier: Rarity, rng: &mut impl RngExt) -> StockItem {
    let found = finds(depth);
    let deep = FloorSpec::of(depth).finds;
    let mut tier = tier;
    loop {
        let shelf: Vec<Shelf> = found
            .iter()
            .copied()
            .filter(|kind| kind.rarity() == tier)
            .map(Shelf::Stock)
            .chain(
                deep.iter()
                    .copied()
                    .filter(|find| find.rarity() == tier)
                    .map(Shelf::Deep),
            )
            .collect();
        if !shelf.is_empty() {
            return match shelf[rng.random_range(0..shelf.len())] {
                Shelf::Stock(kind) => StockItem::Consumable(kind),
                Shelf::Deep(find) => treasure_stock(find.roll(rng).0),
            };
        }
        if tier == Rarity::Common {
            return StockItem::Junk;
        }
        tier = poorer(tier);
    }
}

#[derive(Clone, Copy)]
enum Shelf {
    Stock(ConsumableKind),
    Deep(Find),
}

pub fn treasure_stock(treasure: Treasure) -> StockItem {
    StockItem::Consumable(ConsumableKind::Treasure(treasure))
}

pub fn find(haul: &mut Haul, find: Find, rng: &mut impl RngExt) {
    let (treasure, count) = find.roll(rng);
    haul.add(treasure_stock(treasure), count);
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    use super::*;
    use crate::dungeon::floor::LAST_FLOOR;

    #[test]
    fn a_treasure_is_something_the_floor_could_have_given_you() {
        let mut rng = SmallRng::seed_from_u64(3);
        for depth in 1..=LAST_FLOOR {
            for tier in Rarity::ALL {
                let found = treasure(depth, tier, &mut rng);
                let StockItem::Consumable(kind) = found else {
                    continue;
                };
                let deep = kind.treasure().is_some_and(|treasure| {
                    FloorSpec::of(depth)
                        .finds
                        .iter()
                        .any(|find| find.rarity() == treasure.rarity())
                });
                assert!(finds(depth).contains(&kind) || deep, "floor {depth}");
                assert!(kind.rarity() as u8 <= tier as u8);
            }
        }
    }

    #[test]
    fn a_haul_keeps_one_stack_a_kind() {
        let mut haul = Haul::default();
        haul.add(StockItem::COFFEE, 2);
        haul.add(StockItem::COFFEE, 1);
        haul.add(StockItem::Junk, 1);
        assert_eq!(haul.treasures.len(), 2);
        assert_eq!(haul.count(), 4);
        assert!(!haul.spend(1));
        haul.gold = 10;
        assert!(haul.spend(4));
        assert_eq!(haul.gold, 6);
    }
}
