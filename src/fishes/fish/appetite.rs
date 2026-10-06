use super::{Fish, PERCENT_WHOLE};
use crate::economy::Money;
use crate::entities::food::FOOD_WEIGHT_GAIN_G;
use crate::entities::speech::SPEECH_BUBBLE_TTL;
use crate::fishes::species::FishSpecies;

pub const BURP: &str = "burp!";
const MOST_A_GROWING_FISH_IS_FED: Money = PERCENT_WHOLE - 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fed {
    Growing(u8),
    Full,
    Boundless,
    Worthless,
}

impl Fed {
    pub fn percent(self) -> Option<f64> {
        match self {
            Fed::Growing(percent) => Some(f64::from(percent)),
            Fed::Full => Some(PERCENT_WHOLE as f64),
            Fed::Boundless => Some(f64::INFINITY),
            Fed::Worthless => None,
        }
    }
}

impl Fish {
    pub fn fed(&self) -> Fed {
        let Some(cap) = self.food_cap_g() else {
            return match self.earns_from_food() {
                true => Fed::Boundless,
                false => Fed::Worthless,
            };
        };
        if self.sells_for_nothing() {
            return Fed::Worthless;
        }
        if !self.earns_from_food() {
            return Fed::Full;
        }
        let base = self.species.config().weight_base[self.size_category as usize];
        let eaten = Money::from(self.weight_g.saturating_sub(base));
        let room = Money::from(cap.saturating_sub(base)).max(1);
        let percent = (eaten * PERCENT_WHOLE / room).min(MOST_A_GROWING_FISH_IS_FED);
        Fed::Growing(percent as u8)
    }

    pub fn worth_when_full(&self) -> Option<Money> {
        match self.fed() {
            Fed::Growing(_) => self.food_cap_g().map(|cap| self.worth_at(cap)),
            Fed::Full | Fed::Boundless | Fed::Worthless => None,
        }
    }

    pub fn food_cap_g(&self) -> Option<u32> {
        if self.unfish_state.is_some() {
            return None;
        }
        let cap = self.species.config().weight_cap[self.size_category as usize];
        (cap > 0).then_some(cap)
    }

    fn fed_weight(&self, gain: u32) -> u32 {
        let fed = self.weight_g.saturating_add(gain);
        match self.food_cap_g() {
            Some(cap) => fed.min(cap).max(self.weight_g),
            None => fed,
        }
    }

    pub fn earns_from_food(&self) -> bool {
        self.worth_at(self.fed_weight(FOOD_WEIGHT_GAIN_G)) > self.sell_value()
    }

    pub fn seeks_food(&self) -> bool {
        self.ability_stacks(FishSpecies::Candyfish) > 0 || self.earns_from_food()
    }

    pub fn eat(&mut self, gain: u32) {
        let hungry = self.earns_from_food();
        self.weight_g = self.fed_weight(gain);
        if hungry && !self.earns_from_food() {
            self.burp();
        }
    }

    fn burp(&mut self) {
        self.say(BURP.to_string());
        self.habits.sated = SPEECH_BUBBLE_TTL;
    }

    pub fn is_sated(&self) -> bool {
        self.habits.sated > 0.0
    }
}
