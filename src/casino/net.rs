use std::sync::OnceLock;

use rand::RngExt;

use crate::economy::{Money, Rarity};
use crate::entities::food::FOOD_BUY_PRICE;
use crate::fishes::species::{ALL_SPECIES, FishSpecies};
use crate::tank::FEED_PORTION;

pub const STRIP_LEN: usize = 44;
pub const LANDS_AT: usize = 36;
pub const CAST_SECS: f32 = 4.2;
const LANDING_JITTER: f32 = 0.4;
const EASE_POWER: i32 = 4;
const RESTING_OFFSET: f32 = 6.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Net {
    Small,
    Big,
    Golden,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Prize {
    Fish(FishSpecies),
    Food,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Slot {
    Fish(Rarity),
    Food,
}

impl Net {
    pub const ALL: [Net; 3] = [Net::Small, Net::Big, Net::Golden];

    pub fn name(self) -> &'static str {
        match self {
            Net::Small => "Small Net",
            Net::Big => "Big Net",
            Net::Golden => "Golden Net",
        }
    }

    pub fn price(self) -> Money {
        match self {
            Net::Small => Money::from(Rarity::Common.fish_buy_price()),
            Net::Big => Money::from(Rarity::Rare.fish_buy_price()),
            Net::Golden => {
                Money::from(Rarity::Rare.fish_buy_price())
                    * Money::from(Rarity::Rare.catch_weight())
                    / Money::from(Rarity::Legendary.catch_weight())
            }
        }
    }

    fn slots(self) -> &'static [(Slot, u32)] {
        match self {
            Net::Small => &[
                (Slot::Fish(Rarity::Common), 480),
                (Slot::Fish(Rarity::Rare), 55),
                (Slot::Fish(Rarity::Legendary), 2),
                (Slot::Food, 463),
            ],
            Net::Big => &[
                (Slot::Fish(Rarity::Rare), 450),
                (Slot::Fish(Rarity::Common), 300),
                (Slot::Fish(Rarity::Legendary), 12),
                (Slot::Food, 238),
            ],
            Net::Golden => &[
                (Slot::Fish(Rarity::Legendary), 190),
                (Slot::Fish(Rarity::Rare), 450),
                (Slot::Fish(Rarity::Common), 160),
                (Slot::Food, 200),
            ],
        }
    }

    pub fn cheaper(self) -> Net {
        match self {
            Net::Golden => Net::Big,
            Net::Big | Net::Small => Net::Small,
        }
    }

    pub fn dearer(self) -> Net {
        match self {
            Net::Small => Net::Big,
            Net::Big | Net::Golden => Net::Golden,
        }
    }

    pub fn roll(self, rng: &mut impl RngExt) -> Prize {
        let total: u32 = self.slots().iter().map(|(_, w)| w).sum();
        let mut draw = rng.random_range(0..total);
        for (slot, weight) in self.slots() {
            if draw < *weight {
                return match slot {
                    Slot::Food => Prize::Food,
                    Slot::Fish(rarity) => {
                        let school = school(*rarity);
                        Prize::Fish(school[rng.random_range(0..school.len())])
                    }
                };
            }
            draw -= weight;
        }
        Prize::Food
    }

    pub fn chance_of(self, rarity: Option<Rarity>) -> f64 {
        let total: u32 = self.slots().iter().map(|(_, w)| w).sum();
        let hits: u32 = self
            .slots()
            .iter()
            .filter(|(slot, _)| match (slot, rarity) {
                (Slot::Fish(r), Some(want)) => *r == want,
                (Slot::Food, None) => true,
                _ => false,
            })
            .map(|(_, w)| w)
            .sum();
        f64::from(hits) / f64::from(total)
    }

    pub fn pays_back(self) -> f64 {
        let total: u32 = self.slots().iter().map(|(_, w)| w).sum();
        let value: f64 = self
            .slots()
            .iter()
            .map(|(slot, w)| {
                let worth = match slot {
                    Slot::Fish(rarity) => rarity.fish_buy_price() as f64,
                    Slot::Food => food_worth() as f64,
                };
                worth * f64::from(*w)
            })
            .sum();
        value / f64::from(total) / self.price() as f64
    }
}

pub fn food_worth() -> Money {
    Money::from(FEED_PORTION as u32) * Money::from(FOOD_BUY_PRICE)
}

pub fn food_portion() -> u32 {
    FEED_PORTION as u32
}

pub fn school(rarity: Rarity) -> &'static [FishSpecies] {
    static SCHOOLS: OnceLock<[Vec<FishSpecies>; 3]> = OnceLock::new();
    let schools = SCHOOLS.get_or_init(|| {
        let of = |rarity: Rarity| -> Vec<FishSpecies> {
            ALL_SPECIES
                .iter()
                .copied()
                .filter(|s| s.config().habitat.is_fished() && s.config().rarity == rarity)
                .collect()
        };
        [of(Rarity::Common), of(Rarity::Rare), of(Rarity::Legendary)]
    });
    match rarity {
        Rarity::Common => &schools[0],
        Rarity::Rare => &schools[1],
        Rarity::Legendary => &schools[2],
    }
}

impl Prize {
    pub fn rarity(self) -> Option<Rarity> {
        match self {
            Prize::Fish(species) => Some(species.config().rarity),
            Prize::Food => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CastPhase {
    Waiting,
    Casting { clock: f32 },
    Landed,
}

#[derive(Clone, Debug)]
pub struct Cast {
    pub net: Net,
    pub strip: Vec<Prize>,
    pub phase: CastPhase,
    pub offset: f32,
    target: f32,
}

impl Cast {
    pub fn new(net: Net, rng: &mut impl RngExt) -> Self {
        Self {
            net,
            strip: (0..STRIP_LEN).map(|_| net.roll(rng)).collect(),
            phase: CastPhase::Waiting,
            offset: RESTING_OFFSET,
            target: RESTING_OFFSET,
        }
    }

    pub fn cast(&mut self, rng: &mut impl RngExt) {
        *self = Self::new(self.net, rng);
        self.target = LANDS_AT as f32 + rng.random_range(-LANDING_JITTER..LANDING_JITTER);
        self.phase = CastPhase::Casting { clock: 0.0 };
    }

    pub fn prize(&self) -> Prize {
        self.strip[LANDS_AT]
    }

    pub fn tick(&mut self, dt: f32) -> Option<Prize> {
        let CastPhase::Casting { clock } = &mut self.phase else {
            return None;
        };
        *clock += dt;
        let e = (*clock / CAST_SECS).min(1.0);
        self.offset =
            RESTING_OFFSET + (self.target - RESTING_OFFSET) * (1.0 - (1.0 - e).powi(EASE_POWER));
        if e < 1.0 {
            return None;
        }
        self.phase = CastPhase::Landed;
        Some(self.prize())
    }

    pub fn is_casting(&self) -> bool {
        matches!(self.phase, CastPhase::Casting { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_net_pays_back_less_than_its_price_at_the_shops_prices() {
        for net in Net::ALL {
            let rtp = net.pays_back();
            assert!((0.8..0.92).contains(&rtp), "{net:?}: {rtp}");
        }
    }

    #[test]
    fn every_rarity_has_fish_to_net() {
        for rarity in [Rarity::Common, Rarity::Rare, Rarity::Legendary] {
            assert!(!school(rarity).is_empty(), "{rarity:?}");
        }
    }

    #[test]
    fn a_net_never_brings_up_a_fish_nobody_can_catch() {
        for rarity in [Rarity::Common, Rarity::Rare, Rarity::Legendary] {
            for species in school(rarity) {
                assert!(species.config().habitat.is_fished(), "{species:?}");
            }
        }
    }

    #[test]
    fn the_strip_stops_on_the_prize() {
        let mut rng = rand::rng();
        let mut cast = Cast::new(Net::Small, &mut rng);
        cast.cast(&mut rng);
        let mut landed = None;
        for _ in 0..1000 {
            if let Some(prize) = cast.tick(1.0 / 30.0) {
                landed = Some(prize);
                break;
            }
        }
        assert_eq!(landed, Some(cast.prize()));
        assert!((cast.offset - LANDS_AT as f32).abs() <= LANDING_JITTER);
    }
}
