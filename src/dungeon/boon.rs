use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Boon {
    SecondWind,
    SharpScales,
    DeepLungs,
}

impl Boon {
    pub const ALL: [Boon; 3] = [Boon::SecondWind, Boon::SharpScales, Boon::DeepLungs];

    pub fn name(self) -> &'static str {
        match self {
            Boon::SecondWind => "Second Wind",
            Boon::SharpScales => "Sharp Scales",
            Boon::DeepLungs => "Deep Lungs",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Boon::SecondWind => "Once a fight, belly up, it rights itself at a quarter of its HP.",
            Boon::SharpScales => "Whoever bites it takes 3.",
            Boon::DeepLungs => "One more bubble.",
        }
    }

    pub fn curse(self) -> Curse {
        match self {
            Boon::SecondWind => Curse::HungryGhost,
            Boon::SharpScales => Curse::Brittle,
            Boon::DeepLungs => Curse::HeavyFins,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Curse {
    HungryGhost,
    Brittle,
    HeavyFins,
    FinePrint,
    Wanting,
    WrongWord,
}

impl Curse {
    pub const ALL: [Curse; 6] = [
        Curse::HungryGhost,
        Curse::Brittle,
        Curse::HeavyFins,
        Curse::FinePrint,
        Curse::Wanting,
        Curse::WrongWord,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Curse::HungryGhost => "Hungry Ghost",
            Curse::Brittle => "Brittle",
            Curse::HeavyFins => "Heavy Fins",
            Curse::FinePrint => "Fine Print",
            Curse::Wanting => "Found Wanting",
            Curse::WrongWord => "The Wrong Word",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Curse::HungryGhost => "One less Bite unless it is full.",
            Curse::Brittle => "Crits against it do triple.",
            Curse::HeavyFins => "One less Speed.",
            Curse::FinePrint => "A tenth less gold from everything it finds.",
            Curse::Wanting => "A tenth less HP.",
            Curse::WrongWord => "Enemies crit it 10% more often.",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Charm {
    CaleuchesLantern,
    KrakensBeak,
    MillalobosCrown,
    SweetTooth,
    LeviathansRib,
    TractorBeam,
    CoreFragment,
    DaemonsFork,
    RootAccess,
    DevilsCoin,
    Halo,
    HeraldsTrumpet,
}

impl Charm {
    pub const ALL: [Charm; 12] = [
        Charm::CaleuchesLantern,
        Charm::KrakensBeak,
        Charm::MillalobosCrown,
        Charm::SweetTooth,
        Charm::LeviathansRib,
        Charm::TractorBeam,
        Charm::CoreFragment,
        Charm::DaemonsFork,
        Charm::RootAccess,
        Charm::DevilsCoin,
        Charm::Halo,
        Charm::HeraldsTrumpet,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Charm::CaleuchesLantern => "The Caleuche's Lantern",
            Charm::KrakensBeak => "The Kraken's Beak",
            Charm::MillalobosCrown => "The Millalobo's Crown",
            Charm::SweetTooth => "The Sweet Tooth",
            Charm::LeviathansRib => "The Leviathan's Rib",
            Charm::TractorBeam => "A Tractor Beam",
            Charm::CoreFragment => "A Core Fragment",
            Charm::DaemonsFork => "The Daemon's Fork",
            Charm::RootAccess => "Root Access",
            Charm::DevilsCoin => "The Devil's Coin",
            Charm::Halo => "A Halo",
            Charm::HeraldsTrumpet => "The Herald's Trumpet",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Charm::CaleuchesLantern => "The party is never ambushed, and it is one Speed faster.",
            Charm::KrakensBeak => "Its bites go through armour.",
            Charm::MillalobosCrown => "Every ally bites one harder.",
            Charm::SweetTooth => "Food heals it double.",
            Charm::LeviathansRib => "Four less from every hit.",
            Charm::TractorBeam => "Never beamed or carried off, and it always gets away.",
            Charm::CoreFragment => "One more Bite for every mutation it carries.",
            Charm::DaemonsFork => "Once a fight, belly up, a fork of it stands up at half HP.",
            Charm::RootAccess => "It reads every enemy's next move.",
            Charm::DevilsCoin => "A quarter more gold from everything it finds.",
            Charm::Halo => "Counts as holy, and holy heals on it are doubled.",
            Charm::HeraldsTrumpet => "Once a fight, every ally gains a bubble.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_boon_comes_with_its_own_curse() {
        let curses: std::collections::BTreeSet<Curse> =
            Boon::ALL.iter().map(|b| b.curse()).collect();
        assert_eq!(curses.len(), Boon::ALL.len());
    }
}
