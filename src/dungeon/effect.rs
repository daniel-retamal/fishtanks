#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DamageKind {
    Bite,
    Skill,
    Pierce,
    Holy,
    True,
}

impl DamageKind {
    pub fn dodgeable(self) -> bool {
        self == DamageKind::Bite
    }

    pub fn armoured(self) -> bool {
        self != DamageKind::True
    }

    pub fn shielded(self) -> bool {
        !matches!(self, DamageKind::True | DamageKind::Pierce)
    }

    pub fn guarded(self) -> bool {
        self != DamageKind::Pierce
    }

    pub fn halved_by_ether(self) -> bool {
        self == DamageKind::Bite
    }

    pub fn doubled_on_the_unholy(self) -> bool {
        self == DamageKind::Holy
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Power {
    Bite { percent: u32 },
    Flat(u32),
    PerVoice(u32),
}

impl Power {
    pub const BITE: Power = Power::Bite { percent: 100 };

    pub const fn bite_times(times: u32) -> Power {
        Power::Bite {
            percent: times * 100,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reach {
    Me,
    OneFoe,
    TwoFoes,
    AllFoes,
    RandomFoes(u8),
    OneAlly,
    OtherAllies,
    AllAllies,
    Everyone,
    BellyUpAlly,
    FallenAlly,
    BellyUpFoe,
    WeakestAlly,
    MarkedFoes,
}

impl Reach {
    pub fn picks_a_foe(self) -> bool {
        matches!(self, Reach::OneFoe | Reach::TwoFoes | Reach::BellyUpFoe)
    }

    pub fn picks_an_ally(self) -> bool {
        matches!(
            self,
            Reach::OneAlly | Reach::BellyUpAlly | Reach::FallenAlly
        )
    }

    pub fn label(self) -> &'static str {
        match self {
            Reach::Me => "itself",
            Reach::OneFoe => "one enemy",
            Reach::TwoFoes => "two enemies",
            Reach::AllFoes => "every enemy",
            Reach::RandomFoes(_) => "random enemies",
            Reach::OneAlly => "one ally",
            Reach::OtherAllies => "every other ally",
            Reach::AllAllies => "every ally",
            Reach::Everyone => "every fighter",
            Reach::BellyUpAlly => "a belly-up ally",
            Reach::FallenAlly => "a fish that fell",
            Reach::BellyUpFoe => "a belly-up enemy",
            Reach::WeakestAlly => "the weakest ally",
            Reach::MarkedFoes => "every marked enemy",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StatusKind {
    Held,
    Poison,
    Burn,
    Bleed,
    Rot,
    Blind,
    Charmed,
    Slowed,
    BiteUp,
    BiteDown,
    AccuracyDown,
    Invisible,
    OutOfReach,
    Away,
    Contract,
    SkillsLocked,
    Dreaming,
    Candied,
    Astray,
    Inked,
    Dazzled,
    Doomed,
    Untouchable,
    Puffed,
    Overfull,
    Hungry,
    Hasted,
    Steady,
    Sure,
    Primed,
    Infected,
}

impl StatusKind {
    pub fn ticks(self) -> bool {
        matches!(
            self,
            StatusKind::Poison
                | StatusKind::Burn
                | StatusKind::Bleed
                | StatusKind::Rot
                | StatusKind::Infected
        )
    }

    pub fn is_debuff(self) -> bool {
        !matches!(
            self,
            StatusKind::BiteUp
                | StatusKind::Invisible
                | StatusKind::OutOfReach
                | StatusKind::Untouchable
                | StatusKind::Puffed
                | StatusKind::Hasted
                | StatusKind::Steady
                | StatusKind::Sure
                | StatusKind::Primed
        )
    }

    pub fn stops_acting(self) -> bool {
        matches!(self, StatusKind::Held | StatusKind::Away)
    }

    pub fn name(self) -> &'static str {
        match self {
            StatusKind::Held => "held",
            StatusKind::Poison => "poisoned",
            StatusKind::Burn => "burning",
            StatusKind::Bleed => "bleeding",
            StatusKind::Rot => "rotting",
            StatusKind::Blind => "blinded",
            StatusKind::Charmed => "charmed",
            StatusKind::Slowed => "slowed",
            StatusKind::BiteUp => "fierce",
            StatusKind::BiteDown => "weakened",
            StatusKind::AccuracyDown => "unsteady",
            StatusKind::Invisible => "invisible",
            StatusKind::OutOfReach => "out of reach",
            StatusKind::Away => "out of the fight",
            StatusKind::Contract => "under contract",
            StatusKind::SkillsLocked => "locked out of its skills",
            StatusKind::Dreaming => "dreaming",
            StatusKind::Candied => "candied",
            StatusKind::Astray => "led astray",
            StatusKind::Inked => "inked",
            StatusKind::Dazzled => "dazzled",
            StatusKind::Doomed => "doomed",
            StatusKind::Untouchable => "untouchable",
            StatusKind::Puffed => "puffed up",
            StatusKind::Overfull => "overfull",
            StatusKind::Hungry => "hungry",
            StatusKind::Hasted => "quickened",
            StatusKind::Steady => "steady",
            StatusKind::Sure => "locked on",
            StatusKind::Primed => "about to strike",
            StatusKind::Infected => "infected",
        }
    }

    pub fn mirror(self) -> Option<StatusKind> {
        match self {
            StatusKind::BiteUp => Some(StatusKind::BiteDown),
            StatusKind::BiteDown => Some(StatusKind::BiteUp),
            StatusKind::Invisible => Some(StatusKind::Blind),
            StatusKind::Blind => Some(StatusKind::Invisible),
            StatusKind::Slowed => Some(StatusKind::Untouchable),
            StatusKind::Untouchable => Some(StatusKind::Slowed),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Status {
    pub kind: StatusKind,
    pub turns: u8,
    pub amount: u32,
}

impl Status {
    pub const fn new(kind: StatusKind, turns: u8, amount: u32) -> Status {
        Status {
            kind,
            turns,
            amount,
        }
    }

    pub const fn for_turns(kind: StatusKind, turns: u8) -> Status {
        Status::new(kind, turns, 0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    Hit {
        power: Power,
        kind: DamageKind,
        hits: u8,
    },
    Heal(u32),
    HealPercent(u32),
    Shield(u32),
    Bubbles(i8),
    Inflict(Status),
    Cleanse,
    Revive {
        percent: u32,
    },
    Summon {
        enemy: &'static str,
        count: u8,
    },
    Split,
    Drain(u32),
    Recoil(u32),
    Mutate(u8),
    StealItem,
    StealGold {
        percent: u32,
    },
    Haul(u32),
    Special(Special),
}

impl Effect {
    pub const fn bite() -> Effect {
        Effect::Hit {
            power: Power::BITE,
            kind: DamageKind::Bite,
            hits: 1,
        }
    }

    pub const fn bites(hits: u8) -> Effect {
        Effect::Hit {
            power: Power::BITE,
            kind: DamageKind::Bite,
            hits,
        }
    }

    pub const fn skill(amount: u32) -> Effect {
        Effect::Hit {
            power: Power::Flat(amount),
            kind: DamageKind::Skill,
            hits: 1,
        }
    }

    pub const fn pierce(amount: u32) -> Effect {
        Effect::Hit {
            power: Power::Flat(amount),
            kind: DamageKind::Pierce,
            hits: 1,
        }
    }

    pub const fn holy(amount: u32) -> Effect {
        Effect::Hit {
            power: Power::Flat(amount),
            kind: DamageKind::Holy,
            hits: 1,
        }
    }

    pub const fn inflict(kind: StatusKind, turns: u8, amount: u32) -> Effect {
        Effect::Inflict(Status::new(kind, turns, amount))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Special {
    Bribe,
    PayToWin,
    Radiate,
    Meltdown,
    SugarRush,
    Candify,
    Miracle,
    Script,
    Divide,
    Collapse,
    Devour,
    CallHome,
    Duel,
    Chase,
    Mime,
    Echo,
    Ricochet,
    Hop,
    Lurk,
    Heart,
    HandMeDown,
    Kiss,
    Corrode,
    Wear,
    CrossOver,
    Roll,
    Dominate,
    Erase,
    Cling,
    Static,
    Touch,
    Amnesia,
    Turn,
    Shuffle,
    Shed,
    Reverse,
    Eclipse,
    Invert,
    Charge,
    BlastOff,
    Kick,
    Alarm,
    ThrowJunk,
    Copy,
    WalkThePlank,
    Abduct,
    Swallow,
    Reap,
    Raise,
    Command,
    Grace,
    Banish,
    Hymn,
    Crescendo,
    Fanfare,
    Contract,
    Encrypt,
    Optimise,
    Fork,
    Moo,
    Milk,
    Borrow,
    Assimilate,
}

impl Special {
    pub fn is_random(self) -> bool {
        matches!(
            self,
            Special::Miracle
                | Special::Radiate
                | Special::Meltdown
                | Special::CallHome
                | Special::Static
                | Special::Shuffle
        )
    }
}
