use super::effect::{DamageKind, Effect, Power, Reach};

pub const BASE_ACCURACY: u32 = 95;
pub const SURE_ACCURACY: u32 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Price {
    Bubbles(u8),
    Purse { percent: u32 },
    Free,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skill {
    pub name: &'static str,
    pub price: Price,
    pub reach: Reach,
    pub effects: Vec<Effect>,
    pub accuracy: u32,
    pub once_a_fight: bool,
    pub about: String,
}

impl Skill {
    pub fn new(
        name: &'static str,
        cost: u8,
        reach: Reach,
        effects: &[Effect],
        about: &str,
    ) -> Skill {
        let price = match cost {
            0 => Price::Free,
            n => Price::Bubbles(n),
        };
        Skill {
            name,
            price,
            reach,
            effects: effects.to_vec(),
            accuracy: BASE_ACCURACY,
            once_a_fight: false,
            about: about.to_string(),
        }
    }

    pub fn once(mut self) -> Skill {
        self.once_a_fight = true;
        self
    }

    pub fn sure(mut self) -> Skill {
        self.accuracy = SURE_ACCURACY;
        self
    }

    pub fn paid_from_the_purse(mut self, percent: u32) -> Skill {
        self.price = Price::Purse { percent };
        self
    }

    pub fn bubbles(&self) -> u8 {
        match self.price {
            Price::Bubbles(n) => n,
            Price::Purse { .. } | Price::Free => 0,
        }
    }

    pub fn hits(&self) -> u32 {
        self.effects
            .iter()
            .map(|effect| match effect {
                Effect::Hit { hits, .. } => u32::from(*hits),
                _ => 0,
            })
            .sum()
    }

    pub fn deals_damage(&self) -> bool {
        self.effects.iter().any(|effect| {
            matches!(effect, Effect::Hit { .. } | Effect::Drain(_))
                || matches!(effect, Effect::Special(_))
        })
    }

    pub fn bite_kind(&self) -> Option<DamageKind> {
        self.effects.iter().find_map(|effect| match effect {
            Effect::Hit { kind, .. } => Some(*kind),
            _ => None,
        })
    }

    pub fn power(&self) -> Option<Power> {
        self.effects.iter().find_map(|effect| match effect {
            Effect::Hit { power, .. } => Some(*power),
            _ => None,
        })
    }
}

pub fn bubbles(count: u32, of: u32) -> String {
    let full = count.min(of) as usize;
    let empty = of.saturating_sub(count) as usize;
    format!("{}{}", FULL_BUBBLE.repeat(full), EMPTY_BUBBLE.repeat(empty))
}

pub fn cost_label(price: Price) -> String {
    match price {
        Price::Bubbles(n) => FULL_BUBBLE.repeat(usize::from(n)),
        Price::Purse { percent } => format!("{percent}% of your purse"),
        Price::Free => FREE.to_string(),
    }
}

pub const FULL_BUBBLE: &str = "●";
pub const EMPTY_BUBBLE: &str = "○";
pub const FREE: &str = "free";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mp_is_always_drawn_as_bubbles() {
        assert_eq!(bubbles(2, 3), "●●○");
        assert_eq!(bubbles(0, 2), "○○");
        assert_eq!(bubbles(5, 3), "●●●");
        assert_eq!(cost_label(Price::Bubbles(2)), "●●");
        assert_eq!(cost_label(Price::Free), "free");
    }
}
