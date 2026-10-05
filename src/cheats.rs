use crate::names;
use crate::void_ritual::phrase_matches;
use serde::{Deserialize, Serialize};

pub const CHEAT_RESOURCE_AMOUNT: u32 = 10_000;
pub const DEBUG_MODE_LABEL: &str = "debug mode";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Switch {
    Godmode,
    Bottomless,
    Boundless,
    NoEscape,
}

impl Switch {
    pub const ALL: &'static [Switch] = &[
        Switch::Godmode,
        Switch::Bottomless,
        Switch::Boundless,
        Switch::NoEscape,
    ];

    pub fn status_label(self) -> Option<&'static str> {
        match self {
            Switch::Godmode => Some("godmode"),
            Switch::NoEscape => Some("power overwhelming"),
            Switch::Bottomless | Switch::Boundless => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub enum Cheat {
    ShowMeTheMoney,
    BreatheDeep,
    FoodForThought,
    BendTheLight,
    FishToTheFishtank,
    PowerOverwhelming,
    ThereIsNoCowLevel,
    StayingAlive,
    ThereIsNoSpoon,
    ModifyThePhaseVariance,
    HighwayToHell,
    TheTruthIsOutThere,
    RadioFreeFishtank,
    SomethingForNothing,
    Electrochemistry,
    HardcoreToTheMega,
    Nothing,
}

impl Cheat {
    pub const ALL: &'static [Cheat] = &[
        Cheat::ShowMeTheMoney,
        Cheat::BreatheDeep,
        Cheat::FoodForThought,
        Cheat::BendTheLight,
        Cheat::FishToTheFishtank,
        Cheat::PowerOverwhelming,
        Cheat::ThereIsNoCowLevel,
        Cheat::StayingAlive,
        Cheat::ThereIsNoSpoon,
        Cheat::ModifyThePhaseVariance,
        Cheat::HighwayToHell,
        Cheat::TheTruthIsOutThere,
        Cheat::RadioFreeFishtank,
        Cheat::SomethingForNothing,
        Cheat::Electrochemistry,
        Cheat::HardcoreToTheMega,
        Cheat::Nothing,
    ];

    pub fn codes(self) -> &'static [&'static str] {
        match self {
            Cheat::ShowMeTheMoney => &["show me the money"],
            Cheat::BreatheDeep => &["breathe deep"],
            Cheat::FoodForThought => &["food for thought"],
            Cheat::BendTheLight => &["bend the light"],
            Cheat::FishToTheFishtank => &["fish to the fishtank"],
            Cheat::PowerOverwhelming => &["power overwhelming"],
            Cheat::ThereIsNoCowLevel => &["there is no cow level"],
            Cheat::StayingAlive => &["staying alive"],
            Cheat::ThereIsNoSpoon => &["there is no spoon"],
            Cheat::ModifyThePhaseVariance => &["modify the phase variance"],
            Cheat::HighwayToHell => &["highway to hell"],
            Cheat::TheTruthIsOutThere => &["the truth is out there"],
            Cheat::RadioFreeFishtank => &["radio free fishtank"],
            Cheat::SomethingForNothing => &["something for nothing"],
            Cheat::Electrochemistry => &["electrochemistry"],
            Cheat::HardcoreToTheMega => &["hardcore to the mega"],
            Cheat::Nothing => &["nothing", "nada"],
        }
    }

    pub fn code(self) -> &'static str {
        self.codes()[0]
    }

    pub fn switch(self) -> Option<Switch> {
        match self {
            Cheat::FishToTheFishtank => Some(Switch::Godmode),
            Cheat::BendTheLight => Some(Switch::Bottomless),
            Cheat::FoodForThought => Some(Switch::Boundless),
            Cheat::PowerOverwhelming => Some(Switch::NoEscape),
            _ => None,
        }
    }

    pub fn parse(text: &str) -> Option<Cheat> {
        Cheat::ALL
            .iter()
            .copied()
            .find(|cheat| cheat.codes().iter().any(|code| phrase_matches(text, code)))
    }

    pub fn fish_name(self) -> String {
        names::title_case(self.code())
    }

    pub fn of_switch(switch: Switch) -> Cheat {
        Cheat::ALL
            .iter()
            .copied()
            .find(|cheat| cheat.switch() == Some(switch))
            .expect("every switch is thrown by a cheat")
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Cheats {
    pub godmode: bool,
    pub boundless: bool,
    pub no_escape: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn a_code_is_read_whatever_its_case_and_spacing() {
        assert_eq!(
            Cheat::parse("  Show   ME the MONEY "),
            Some(Cheat::ShowMeTheMoney)
        );
        assert_eq!(Cheat::parse("show me the mone"), None);
        assert_eq!(Cheat::parse("   "), None);
    }

    #[test]
    fn nothing_or_nada_is_the_void_seed() {
        for typed in ["NOTHING", "nada"] {
            assert_eq!(Cheat::parse(typed), Some(Cheat::Nothing), "{typed:?}");
        }
    }

    #[test]
    fn no_two_cheats_share_a_code() {
        let codes: Vec<&str> = Cheat::ALL
            .iter()
            .flat_map(|cheat| cheat.codes().iter().copied())
            .collect();
        let distinct: HashSet<&str> = codes.iter().copied().collect();
        assert_eq!(distinct.len(), codes.len());
    }

    #[test]
    fn every_code_parses_back_to_its_own_cheat() {
        for &cheat in Cheat::ALL {
            for code in cheat.codes() {
                assert_eq!(Cheat::parse(code), Some(cheat));
            }
        }
    }

    #[test]
    fn every_switch_is_thrown_by_exactly_one_cheat() {
        for &switch in Switch::ALL {
            let throwers = Cheat::ALL
                .iter()
                .filter(|cheat| cheat.switch() == Some(switch))
                .count();
            assert_eq!(throwers, 1, "{switch:?}");
        }
    }

    #[test]
    fn a_cheatfish_is_named_after_the_code_that_made_it() {
        assert_eq!(Cheat::ShowMeTheMoney.fish_name(), "Show Me The Money");
    }
}
