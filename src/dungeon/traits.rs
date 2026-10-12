use ratatui::style::Color;

use crate::fishes::toy::Line;

use super::effect::Power;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Trait {
    Swarm {
        bites: u8,
        power: Power,
    },
    ExtraBites(u8),
    NeverBites,
    Fuse {
        turns: u8,
        damage: u32,
    },
    Ethereal,
    Thorns(u32),
    Armour(u32),
    HoldsBiter,
    Frenzy,
    SplitsAtHalf,
    Fluff(u32),
    Undying,
    Enrage,
    Feast(u32),
    Shade(u32),
    Mirage,
    MutatesKiller,
    RandomBuffs,
    Soft,
    DeathPoison {
        amount: u32,
        turns: u8,
    },
    Inverted,
    PoisonImmune,
    NoBody,
    Exposure(u32),
    Glitch,
    Learns,
    Upgrade(u32),
    Spill(u32),
    Grudge(u32),
    Unholy,
    AllSeeing,
    Hollow,
    Rebirth,
    Dreamer,
    Grows(u32),
    Merge,
    Rooted,
    Waiting,
    Neither,
    Hatch,
    FirstStrike,
    Lookout,
    Clownfish(u32),
    Burns(u32),
    Crumble,
    Court,
    Parts(BodyParts),
    Phylactery,
    Layers,
    Pinata,
    Background,
    DevilsBargain,
    Greed,
    Paid,
    LastTrump(u8),
    PrayerShield,
    Cycle,
    HalfRemembered,
    Conspicuous(u32),
    SoftEyes(u32),
    Cyclops,
    SeesGhosts,
    PhantomBites,
    Glare(Color),
    BodyColor(Color),
    Patches(u32),
    Wake {
        dark: bool,
    },
    Negative,
    Shimmer {
        percent: u32,
        every_other: bool,
        all_enemies: bool,
        colour: Option<Color>,
    },
    Shifting,
    CaughtTail,
    Loud,
    Ears,
    Gossip,
    Hydra(u8),
    Walks,
    SilentSteps,
    Spikes,
    Wings,
    Tentacles,
    NightOwl,
    HelpedByGod,
    Lure,
    Bill,
    SharkFin,
    Keel,
    Lunar,
    Alien,
    Sweet(u32),
    TwoHeads,
    Backwards,
    Undead,
    Sting(u32),
    Afterburner,
    Upstream,
    Slime,
    Glitter,
    Regrow(u32),
    JunkArmour,
    Camouflage,
    Ambush(u32),
    Glide,
    FalseEye,
    Lantern,
    NineLives,
    Parrot,
    Hide,
    Contact(u32),
    Escape,
    SeeThrough(u32),
    Shoal,
    Electrosense,
    Lucky,
    Cartographer,
    Rusting,
    Greedy(u32),
    Unstable,
    Wrath,
    Candy,
    Blessed(u32),
    Sloth,
    Junkfish,
    Pride,
    Schooling,
    Squish,
    CounterLight,
    LooksUp,
    Fangs,
    Chimera,
    Lazarus(u32),
    AlwaysLast,
    Envy,
    HalfThere,
    Telepathy,
    Segments,
    Absent,
    Scatter,
    TheLap,
    Stare,
    Graeae,
    Unwatched,
    Vessel,
    Light,
    Lustre(u32),
    Plated(u32),
    Sparkle(u32),
    Phase(u32),
    Crown,
    Overhears,
    Hover,
    Treads,
    WindUp,
    GoldenToy,
    ShoalCounter,
    TidalClock,
    Assay,
    LedgerNerve,
    Geiger,
    Startle,
    Commanded,
    Angler,
    Cochlea,
    Reflex,
    Glyph,
    CoreStack,
    Decoy,
    Uplink,
    Sealed,
    GreedSin,
    LustSin,
    GluttonySin,
    WrathSin,
    EnvySin,
    PrideSin,
    SlothSin,
    Golden,
    Doomed,
    Delicious,
    Forbidden,
    Nooo,
    SeenTheSky,
    FavouriteColour(Color),
    Voice(&'static str),
    DevilMarked,
    Holy,
    Cow,
    Construct,
    Signature(Line),
    Drill,
    Lag,
    Reflection,
    SecondWind,
    HungryGhost,
    FinePrint,
    SweetTooth,
    Forked,
    DevilsCoin,
    Trumpet,
    Scaled {
        hp: u32,
        hp_per_floor: u32,
        bite: u32,
        bite_per_floor: u32,
    },
    Wrapped(u32),
    Yours(Haunt),
    Spreads,
    Appendage,
    Hunter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reader {
    Fight,
    Kit,
    Map,
    DeepFish,
    Cows,
}

impl Trait {
    pub fn reader(self) -> Reader {
        match self {
            Trait::Swarm { .. }
            | Trait::ExtraBites(_)
            | Trait::NeverBites
            | Trait::Fuse { .. }
            | Trait::Ethereal
            | Trait::Thorns(_)
            | Trait::Armour(_)
            | Trait::HoldsBiter
            | Trait::Frenzy
            | Trait::SplitsAtHalf
            | Trait::Fluff(_)
            | Trait::Undying
            | Trait::Enrage
            | Trait::Feast(_)
            | Trait::Shade(_)
            | Trait::MutatesKiller
            | Trait::RandomBuffs
            | Trait::Soft
            | Trait::DeathPoison { .. }
            | Trait::Inverted
            | Trait::PoisonImmune
            | Trait::NoBody
            | Trait::Exposure(_)
            | Trait::Glitch
            | Trait::Learns
            | Trait::Upgrade(_)
            | Trait::Spill(_)
            | Trait::Grudge(_)
            | Trait::Unholy
            | Trait::AllSeeing
            | Trait::Hollow
            | Trait::Rebirth
            | Trait::Dreamer
            | Trait::Grows(_)
            | Trait::Merge
            | Trait::Rooted
            | Trait::Waiting
            | Trait::Neither
            | Trait::Hatch
            | Trait::FirstStrike
            | Trait::Lookout
            | Trait::Clownfish(_)
            | Trait::Burns(_)
            | Trait::Crumble
            | Trait::Court
            | Trait::Parts(_)
            | Trait::Phylactery
            | Trait::Layers
            | Trait::Pinata
            | Trait::Background
            | Trait::DevilsBargain
            | Trait::Greed
            | Trait::Paid
            | Trait::LastTrump(_)
            | Trait::PrayerShield
            | Trait::Cycle
            | Trait::HalfRemembered
            | Trait::Conspicuous(_)
            | Trait::SoftEyes(_)
            | Trait::Cyclops
            | Trait::SeesGhosts
            | Trait::PhantomBites
            | Trait::Glare(_)
            | Trait::BodyColor(_)
            | Trait::Patches(_)
            | Trait::Wake { .. }
            | Trait::Negative
            | Trait::Shimmer { .. }
            | Trait::Shifting
            | Trait::CaughtTail
            | Trait::Loud
            | Trait::Ears
            | Trait::Hydra(_)
            | Trait::Walks
            | Trait::Spikes
            | Trait::Wings
            | Trait::Tentacles
            | Trait::HelpedByGod
            | Trait::Lure
            | Trait::Bill
            | Trait::SharkFin
            | Trait::Keel
            | Trait::Lunar
            | Trait::Alien
            | Trait::Sweet(_)
            | Trait::TwoHeads
            | Trait::Backwards
            | Trait::Undead
            | Trait::Sting(_)
            | Trait::Afterburner
            | Trait::Upstream
            | Trait::Slime
            | Trait::Glitter
            | Trait::Regrow(_)
            | Trait::JunkArmour
            | Trait::Camouflage
            | Trait::Ambush(_)
            | Trait::Glide
            | Trait::FalseEye
            | Trait::NineLives
            | Trait::Parrot
            | Trait::Hide
            | Trait::Contact(_)
            | Trait::Escape
            | Trait::SeeThrough(_)
            | Trait::Shoal
            | Trait::Rusting
            | Trait::Greedy(_)
            | Trait::Unstable
            | Trait::Candy
            | Trait::Blessed(_)
            | Trait::Junkfish
            | Trait::Pride
            | Trait::Schooling
            | Trait::CounterLight
            | Trait::AlwaysLast
            | Trait::Envy
            | Trait::HalfThere
            | Trait::Segments
            | Trait::Absent
            | Trait::Scatter
            | Trait::TheLap
            | Trait::Stare
            | Trait::Unwatched
            | Trait::Lustre(_)
            | Trait::Sparkle(_)
            | Trait::Phase(_)
            | Trait::Crown
            | Trait::Hover
            | Trait::WindUp
            | Trait::GoldenToy
            | Trait::ShoalCounter
            | Trait::TidalClock
            | Trait::LedgerNerve
            | Trait::Geiger
            | Trait::Startle
            | Trait::Reflex
            | Trait::Glyph
            | Trait::Decoy
            | Trait::Sealed
            | Trait::GreedSin
            | Trait::LustSin
            | Trait::GluttonySin
            | Trait::WrathSin
            | Trait::EnvySin
            | Trait::PrideSin
            | Trait::SlothSin
            | Trait::Golden
            | Trait::Doomed
            | Trait::Delicious
            | Trait::Forbidden
            | Trait::Nooo
            | Trait::SeenTheSky
            | Trait::FavouriteColour(_)
            | Trait::Voice(_)
            | Trait::DevilMarked
            | Trait::Holy
            | Trait::Construct
            | Trait::Signature(_)
            | Trait::Drill
            | Trait::Lag
            | Trait::Reflection
            | Trait::SecondWind
            | Trait::FinePrint
            | Trait::SweetTooth
            | Trait::Forked
            | Trait::DevilsCoin
            | Trait::Trumpet
            | Trait::Scaled { .. }
            | Trait::Wrapped(_)
            | Trait::Yours(_)
            | Trait::Spreads
            | Trait::Appendage
            | Trait::Hunter
            | Trait::Mirage
            | Trait::Gossip
            | Trait::Electrosense
            | Trait::Telepathy
            | Trait::Graeae
            | Trait::Overhears
            | Trait::Assay
            | Trait::Cochlea
            | Trait::CoreStack => Reader::Fight,
            Trait::Wrath
            | Trait::Sloth
            | Trait::Light
            | Trait::Plated(_)
            | Trait::Treads
            | Trait::Commanded
            | Trait::HungryGhost => Reader::Kit,
            Trait::SilentSteps
            | Trait::NightOwl
            | Trait::Lantern
            | Trait::Lucky
            | Trait::Cartographer
            | Trait::Angler
            | Trait::Uplink => Reader::Map,
            Trait::Squish
            | Trait::LooksUp
            | Trait::Fangs
            | Trait::Chimera
            | Trait::Lazarus(_)
            | Trait::Vessel => Reader::DeepFish,
            Trait::Cow => Reader::Cows,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Haunt {
    Dead,
    Damned,
    Unfish,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BodyParts {
    Tentacles,
    Ribs,
    Heads,
    Voices,
}
