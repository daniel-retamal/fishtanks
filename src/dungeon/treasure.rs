use std::collections::BTreeSet;
use std::sync::OnceLock;

use rand::RngExt;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use crate::colors::{
    AMBER, CHARCOAL, CREAM, GOLD, LIGHT_GREEN, LIGHT_RED, NAVY_LIGHT, PINK, STEEL, VIOLET, WHITE,
};
use crate::economy::Rarity;
use crate::entities::cow::CowVariant;
use crate::fishes::botfish::BotfishState;
use crate::fishes::chip::Chip;
use crate::fishes::mutations::Mutation;
use crate::fishes::parts::{Bus, Part};
use crate::fishes::toy::ToyPart;

use super::boon::Charm;

pub const SHELL_CASTS: u32 = 10;
pub const INK_CASTS: u32 = 1;
pub const SHARDS_PER_WISH: u32 = 3;
pub const RADIANT_SECS: f32 = 3600.0;
pub const CLOCK_SPOOLS: u32 = 14;
pub const ERRAND_SPOOLS: u32 = 899;
pub const TICK: &str = "tick";
pub const FEED_ERRAND: &str = "/feed";
pub const SELL_ERRAND: &str = "/sell fish {heaviest}";
const COUNT_PIN: &str = "count";
const SHELL_LABEL: &str = "pincoya";
const INK_LABEL: &str = "kraken ink";
const VIAL_PREFIX: &str = "Mutagen Vial";
const COWBELL_PREFIX: &str = "Tuned Cowbell";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum Salvage {
    Clock,
    Counter,
    Feeder,
    Seller,
}

impl Salvage {
    pub const ALL: [Salvage; 4] = [
        Salvage::Clock,
        Salvage::Counter,
        Salvage::Feeder,
        Salvage::Seller,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Salvage::Clock => "Salvaged Clock",
            Salvage::Counter => "Salvaged Counter",
            Salvage::Feeder => "Salvaged Feeder",
            Salvage::Seller => "Salvaged Seller",
        }
    }

    fn about(self) -> &'static str {
        match self {
            Salvage::Clock => {
                "A sealed board out of the Mainframe. Etched onto a botfish, its tick pin beats once a second."
            }
            Salvage::Counter => {
                "A sealed board out of the Mainframe. Etched onto a botfish, its count pin reads how many fish share its tank."
            }
            Salvage::Feeder => {
                "A sealed board out of the Mainframe. Etched onto a botfish, it runs /feed once a minute."
            }
            Salvage::Seller => {
                "A sealed board out of the Mainframe. Etched onto a botfish, it sells the heaviest fish in its tank once a minute."
            }
        }
    }

    pub fn chip(self) -> Chip {
        let (board, outputs) = match self {
            Salvage::Clock => (vec![metronome(CLOCK_SPOOLS)], lines(&[TICK])),
            Salvage::Counter => {
                let width = Part::ShoalCounter
                    .pins()
                    .iter()
                    .find(|pin| pin.name == COUNT_PIN)
                    .map_or(1, |pin| pin.width);
                let mut counter = BotfishState::new();
                counter.install(Part::ShoalCounter);
                counter.wire(Part::ShoalCounter, COUNT_PIN, COUNT_PIN);
                let outputs = Bus::new(COUNT_PIN, width)
                    .map(|bus| bus.lines().into_iter().collect())
                    .unwrap_or_default();
                (vec![counter], outputs)
            }
            Salvage::Feeder => (
                vec![metronome(ERRAND_SPOOLS), errand(FEED_ERRAND)],
                lines(&[TICK]),
            ),
            Salvage::Seller => (
                vec![metronome(ERRAND_SPOOLS), errand(SELL_ERRAND)],
                lines(&[TICK]),
            ),
        };
        Chip::new(self.name().to_string(), board, BTreeSet::new(), outputs).sealed()
    }
}

fn lines(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| name.to_string()).collect()
}

fn metronome(spools: u32) -> BotfishState {
    let mut beat = BotfishState::new();
    beat.listen(TICK);
    beat.drive(TICK);
    beat.install(Part::InverterCoil);
    for _ in 0..spools {
        beat.install(Part::DelaySpool);
    }
    beat
}

fn errand(command: &str) -> BotfishState {
    let mut hand = BotfishState::new();
    hand.install(Part::CommandModule);
    hand.wire(Part::CommandModule, "fire", TICK);
    hand.program(String::new(), vec![command.to_string()]);
    hand
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum Treasure {
    PincoyasShell,
    KrakenInk,
    Vial(Mutation),
    RadiantCore,
    Chip(Salvage),
    MammonsCoin,
    Feather,
    MarkedCard,
    Cowbell(CowVariant),
    WishShard,
    PhoenixRoe,
    Charm(Charm),
    Toy(ToyPart),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Use {
    Casts { casts: u32, label: &'static str },
    OnFish,
    OnTheDead,
    OnTheTank,
    Wish,
    AtTheTable,
    OnTheShelf,
    InTheDeep,
}

pub struct TreasureSpec {
    pub rarity: Rarity,
    pub about: &'static str,
    pub art: &'static [&'static str],
    pub colour: Color,
    pub using: Use,
}

const SHELL_ART: &[&str] = &[r"  .--.", r" /||||\", r"(||||||)", r" '----'"];
const INK_ART: &[&str] = &[r"  ___", r" |___|", r" |   |", r" |~~~|", r" |___|"];
const VIAL_ART: &[&str] = &[r"  _", r" |=|", r" | |", r"/ ~ \", r"\___/"];
const CORE_ART: &[&str] = &[r" .-^-.", r"( (*) )", r" '-v-'"];
const CHIP_ART: &[&str] = &[r" |||||", r"-+---+-", r"-|###|-", r"-+---+-", r" |||||"];
const COIN_ART: &[&str] = &[r" .--.", r"( $$ )", r" '--'"];
const FEATHER_ART: &[&str] = &[r"    /", r"   //", r"  //", r" //", r"'"];
const CARD_ART: &[&str] = &[r" ____", r"|A   |", r"| <> |", r"|___A|"];
const BELL_ART: &[&str] = &[r"  _", r" / \", r"/   \", r"\_o_/"];
const SHARD_ART: &[&str] = &[r"  /\", r" /  \", r" \  /", r"  \/"];
const ROE_ART: &[&str] = &[r" oOo", r"oOoOo", r" oOo"];
const CHARM_ART: &[&str] = &[r"  _o_", r" (   )", r"  \_/"];
const TOY_ART: &[&str] = &[r" [==]", r" |  |", r" '--'"];

const fn spec(
    rarity: Rarity,
    about: &'static str,
    art: &'static [&'static str],
    colour: Color,
    using: Use,
) -> TreasureSpec {
    TreasureSpec {
        rarity,
        about,
        art,
        colour,
        using,
    }
}

const FUSIONS: [Mutation; 3] = [
    Mutation::Telophase,
    Mutation::Engulfment,
    Mutation::Cytokinesis,
];

impl Treasure {
    pub fn spec(self) -> TreasureSpec {
        use Rarity::{Legendary, Rare};
        match self {
            Treasure::PincoyasShell => spec(
                Rare,
                "Hold it to your ear before fishing: the next ten casts bite Rare or better. La Pincoya dances facing the sea.",
                SHELL_ART,
                PINK,
                Use::Casts {
                    casts: SHELL_CASTS,
                    label: SHELL_LABEL,
                },
            ),
            Treasure::KrakenInk => spec(
                Legendary,
                "One cast in any tank where the Legendary school always bites.",
                INK_ART,
                CHARCOAL,
                Use::Casts {
                    casts: INK_CASTS,
                    label: INK_LABEL,
                },
            ),
            Treasure::Vial(Mutation::Telophase) => spec(
                Legendary,
                "A tonic that makes a fish a double, on the fish you choose.",
                VIAL_ART,
                LIGHT_GREEN,
                Use::OnFish,
            ),
            Treasure::Vial(Mutation::Engulfment) => spec(
                Legendary,
                "An enzyme that arms a fish to swallow the next fish it meets, on the fish you choose.",
                VIAL_ART,
                LIGHT_GREEN,
                Use::OnFish,
            ),
            Treasure::Vial(Mutation::Cytokinesis) => spec(
                Legendary,
                "Shears that split a double in two, now.",
                VIAL_ART,
                LIGHT_GREEN,
                Use::OnFish,
            ),
            Treasure::Vial(_) => spec(
                Rare,
                "One named mutation, on the fish you choose. The label says which.",
                VIAL_ART,
                LIGHT_GREEN,
                Use::OnFish,
            ),
            Treasure::RadiantCore => spec(
                Legendary,
                "Set it in a tank and that tank is radioactive for an hour: every fish there mutates on the Radioactivetank's clock.",
                CORE_ART,
                LIGHT_GREEN,
                Use::OnTheTank,
            ),
            Treasure::Chip(salvage) => spec(Rare, salvage.about(), CHIP_ART, STEEL, Use::OnFish),
            Treasure::MammonsCoin => spec(
                Legendary,
                "A soul on your hell wall for ever: devil's luck that never leaves.",
                COIN_ART,
                GOLD,
                Use::OnTheTank,
            ),
            Treasure::Feather => spec(
                Legendary,
                "The fish that wears it counts as holy: it may live in the Heaventank, and the Gate lets it through.",
                FEATHER_ART,
                WHITE,
                Use::OnFish,
            ),
            Treasure::MarkedCard => spec(
                Rare,
                "At the casino, the first round you lose is played again as if it never happened.",
                CARD_ART,
                CREAM,
                Use::AtTheTable,
            ),
            Treasure::Cowbell(_) => spec(
                Rare,
                "Ring it in a tank: the mothership drops a cow of the bell's colour there.",
                BELL_ART,
                AMBER,
                Use::OnTheTank,
            ),
            Treasure::WishShard => spec(
                Rare,
                "Three make a wish, granted on the spot.",
                SHARD_ART,
                VIOLET,
                Use::Wish,
            ),
            Treasure::PhoenixRoe => spec(
                Legendary,
                "Raises any dead fish from the graveyard, marked or not.",
                ROE_ART,
                LIGHT_RED,
                Use::OnTheDead,
            ),
            Treasure::Charm(charm) => spec(
                Legendary,
                charm.description(),
                CHARM_ART,
                GOLD,
                Use::InTheDeep,
            ),
            Treasure::Toy(_) => spec(
                Rare,
                "A part for the toybox, found only in the Deep.",
                TOY_ART,
                NAVY_LIGHT,
                Use::OnTheShelf,
            ),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Treasure::PincoyasShell => "Pincoya's Shell",
            Treasure::KrakenInk => "Kraken Ink",
            Treasure::Vial(Mutation::Telophase) => "Telophase Tonic",
            Treasure::Vial(Mutation::Engulfment) => "Engulfing Enzyme",
            Treasure::Vial(Mutation::Cytokinesis) => "Cytokinesis Shears",
            Treasure::Vial(mutation) => vial_name(mutation),
            Treasure::RadiantCore => "Radiant Core",
            Treasure::Chip(salvage) => salvage.name(),
            Treasure::MammonsCoin => "Mammon's Coin",
            Treasure::Feather => "A Feather of the Choir",
            Treasure::MarkedCard => "Marked Card",
            Treasure::Cowbell(variant) => cowbell_name(variant),
            Treasure::WishShard => "Wish Shard",
            Treasure::PhoenixRoe => "Phoenix Roe",
            Treasure::Charm(charm) => charm.name(),
            Treasure::Toy(part) => part.name(),
        }
    }

    pub fn rarity(self) -> Rarity {
        self.spec().rarity
    }

    pub fn using(self) -> Use {
        self.spec().using
    }

    pub fn charm(self) -> Option<Charm> {
        match self {
            Treasure::Charm(charm) => Some(charm),
            _ => None,
        }
    }

    pub fn mutation(self) -> Option<Mutation> {
        match self {
            Treasure::Vial(mutation) => Some(mutation),
            _ => None,
        }
    }

    pub fn all() -> Vec<Treasure> {
        let mut all = vec![Treasure::PincoyasShell, Treasure::KrakenInk];
        all.extend(vials().into_iter().map(Treasure::Vial));
        all.extend(FUSIONS.map(Treasure::Vial));
        all.push(Treasure::RadiantCore);
        all.extend(Salvage::ALL.map(Treasure::Chip));
        all.extend([
            Treasure::MammonsCoin,
            Treasure::Feather,
            Treasure::MarkedCard,
        ]);
        all.extend(
            CowVariant::ALL
                .iter()
                .map(|&variant| Treasure::Cowbell(variant)),
        );
        all.extend([Treasure::WishShard, Treasure::PhoenixRoe]);
        all.extend(Charm::ALL.map(Treasure::Charm));
        all
    }
}

pub fn vials() -> Vec<Mutation> {
    Mutation::ALL
        .iter()
        .copied()
        .filter(|m| m.auto_selectable() && !FUSIONS.contains(m))
        .collect()
}

fn vial_name(mutation: Mutation) -> &'static str {
    static NAMES: OnceLock<Vec<(Mutation, String)>> = OnceLock::new();
    let names = NAMES.get_or_init(|| {
        Mutation::ALL
            .iter()
            .map(|&m| (m, format!("{VIAL_PREFIX} ({})", m.token())))
            .collect()
    });
    names
        .iter()
        .find(|(m, _)| *m == mutation)
        .map_or(VIAL_PREFIX, |(_, name)| name.as_str())
}

fn cowbell_name(variant: CowVariant) -> &'static str {
    static NAMES: OnceLock<Vec<(CowVariant, String)>> = OnceLock::new();
    let names = NAMES.get_or_init(|| {
        CowVariant::ALL
            .iter()
            .map(|&v| (v, format!("{COWBELL_PREFIX} ({})", v.display_name())))
            .collect()
    });
    names
        .iter()
        .find(|(v, _)| *v == variant)
        .map_or(COWBELL_PREFIX, |(_, name)| name.as_str())
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Find {
    Exactly(Treasure),
    Vial,
    Fusion,
    Chip,
    Cowbell,
    Wish,
}

impl Find {
    pub fn rarity(self) -> Rarity {
        match self {
            Find::Exactly(treasure) => treasure.rarity(),
            Find::Fusion => Rarity::Legendary,
            Find::Vial | Find::Chip | Find::Cowbell | Find::Wish => Rarity::Rare,
        }
    }

    pub fn roll(self, rng: &mut impl RngExt) -> (Treasure, u32) {
        match self {
            Find::Exactly(treasure) => (treasure, 1),
            Find::Vial => {
                let all: Vec<Treasure> = vials().into_iter().map(Treasure::Vial).collect();
                (one_of(&all, rng), 1)
            }
            Find::Fusion => (one_of(&FUSIONS.map(Treasure::Vial), rng), 1),
            Find::Chip => (one_of(&Salvage::ALL.map(Treasure::Chip), rng), 1),
            Find::Cowbell => {
                let all: Vec<Treasure> = CowVariant::ALL
                    .iter()
                    .map(|&variant| Treasure::Cowbell(variant))
                    .collect();
                (one_of(&all, rng), 1)
            }
            Find::Wish => (Treasure::WishShard, SHARDS_PER_WISH),
        }
    }
}

fn one_of(all: &[Treasure], rng: &mut impl RngExt) -> Treasure {
    all[rng.random_range(0..all.len())]
}

pub const DEEP_TOYS: [ToyPart; 4] = [
    ToyPart::DivingHelmet,
    ToyPart::JellyfishCap,
    ToyPart::Periscope,
    ToyPart::Anchor,
];

pub fn art_colour(treasure: Treasure) -> Color {
    match treasure {
        Treasure::Cowbell(variant) => variant.body_color(),
        other => other.spec().colour,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    #[test]
    fn every_treasure_has_its_own_name() {
        let names: BTreeSet<&str> = Treasure::all().iter().map(|t| t.name()).collect();
        assert_eq!(names.len(), Treasure::all().len());
    }

    #[test]
    fn a_wish_is_three_shards() {
        let mut rng = SmallRng::seed_from_u64(1);
        assert_eq!(
            Find::Wish.roll(&mut rng),
            (Treasure::WishShard, SHARDS_PER_WISH)
        );
    }

    #[test]
    fn a_vial_holds_a_mutation_the_dice_could_roll_and_a_fusion_holds_a_fusion() {
        let mut rng = SmallRng::seed_from_u64(2);
        for _ in 0..50 {
            let (vial, _) = Find::Vial.roll(&mut rng);
            let mutation = vial.mutation().expect("a vial");
            assert!(mutation.auto_selectable() && !FUSIONS.contains(&mutation));
            let (fusion, _) = Find::Fusion.roll(&mut rng);
            assert!(FUSIONS.contains(&fusion.mutation().expect("a fusion")));
            assert_eq!(fusion.rarity(), Rarity::Legendary);
        }
    }

    fn worn(salvage: Salvage) -> BotfishState {
        let mut host = BotfishState::new();
        host.etch(salvage.chip());
        host
    }

    fn stage(host: &mut BotfishState, channels: &mut crate::tank::ChannelRegistry) {
        host.step(channels, &crate::tank::WorldView::default());
        channels.commit();
    }

    #[test]
    fn the_salvaged_clock_beats_on_its_tick_pin() {
        let mut host = worn(Salvage::Clock);
        let mut channels = crate::tank::ChannelRegistry::new();
        let period = 2 * (CLOCK_SPOOLS as usize + 1);
        let mut levels = Vec::new();
        for _ in 0..period * 3 {
            stage(&mut host, &mut channels);
            levels.push(channels.level(TICK));
        }
        let rises = levels.windows(2).filter(|w| !w[0] && w[1]).count();
        assert!((2..=3).contains(&rises), "{rises} beats in three periods");
    }

    #[test]
    fn the_salvaged_feeder_runs_feed_once_a_beat() {
        let mut host = worn(Salvage::Feeder);
        let mut channels = crate::tank::ChannelRegistry::new();
        let period = 2 * (ERRAND_SPOOLS as usize + 1);
        let fed = (0..period * 2).any(|_| {
            stage(&mut host, &mut channels);
            host.due_lines(0)
                .iter()
                .any(|line| line.command == FEED_ERRAND)
        });
        assert!(fed, "the feeder fed within two beats");
    }

    #[test]
    fn a_salvaged_board_is_sealed_and_gives_no_parts_back() {
        for salvage in Salvage::ALL {
            let chip = salvage.chip();
            assert_eq!(chip.hardware().count(), 0, "{}", salvage.name());
            assert!(
                chip.pins().next().is_some(),
                "{} shows a pin",
                salvage.name()
            );
        }
    }
}
