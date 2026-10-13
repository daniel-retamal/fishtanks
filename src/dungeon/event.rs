use rand::RngExt;
use serde::{Deserialize, Serialize};

use crate::economy::Rarity;
use crate::fishes::species::SizeCategory;
use crate::loot::StockItem;

use super::bestiary::{SpriteRow, row};
use super::items::Item;
use super::kit::Kit;
use super::traits::Trait;

pub const LUCK_PERCENT: i32 = 50;
pub const LUCKY_PERCENT: i32 = 15;
pub const FORTUNE_PERCENT: i32 = 10;
pub const LUCK_FLOOR_PERCENT: i32 = 10;
pub const LUCK_CEILING_PERCENT: i32 = 90;
pub const PERCENT: i32 = 100;
pub const FISH: &str = "{fish}";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Omen {
    ChestsRicher,
    ChestsPoorer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    Luck,
    Fortune,
    Size,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Need {
    Nothing,
    Trait(Trait),
    Item(Item),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    Gold(u64),
    Treasure(Rarity),
    Fight { enemy: &'static str, count: u8 },
    Hurt(u32),
    Omen(Omen),
    Keep(StockItem),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Turn {
    pub text: &'static str,
    pub outcome: Outcome,
}

const fn turn(text: &'static str, outcome: Outcome) -> Turn {
    Turn { text, outcome }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChoiceSpec {
    pub label: &'static str,
    pub need: Need,
    pub check: Option<Check>,
    pub pass: Turn,
    pub fail: Turn,
}

const fn sure(label: &'static str, need: Need, then: Turn) -> ChoiceSpec {
    ChoiceSpec {
        label,
        need,
        check: None,
        pass: then,
        fail: then,
    }
}

const fn checked(label: &'static str, check: Check, pass: Turn, fail: Turn) -> ChoiceSpec {
    ChoiceSpec {
        label,
        need: Need::Nothing,
        check: Some(check),
        pass,
        fail,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventSpec {
    pub name: &'static str,
    pub floor: u32,
    pub art: &'static [SpriteRow],
    pub text: &'static str,
    pub choices: &'static [ChoiceSpec],
}

const LEAVE: Turn = turn("The party swims on.", Outcome::Nothing);

pub const EVENTS: &[EventSpec] = &[
    EventSpec {
        name: "A Note in a Bottle",
        floor: 1,
        art: &[
            row("            _", "e"),
            row("           |~|", "e"),
            row("           | |", "c"),
            row("          /   \\", "c"),
            row("         |  ≡  |", "ccce"),
            row("         |_____|", "c"),
            row("", " "),
            row("    ~  ~~   ~   ~~  ~", "B"),
        ],
        text: "A bottle bobs against the rocks, a rolled note inside.",
        choices: &[
            checked(
                "Read it aloud",
                Check::Fortune,
                turn(
                    "{fish} reads out the way to a pirate's stash.",
                    Outcome::Gold(2),
                ),
                turn(
                    "It was a call to arms. Two Pirate Merluzas board.",
                    Outcome::Fight {
                        enemy: "Pirate Merluza",
                        count: 2,
                    },
                ),
            ),
            sure(
                "Keep it corked",
                Need::Nothing,
                turn(
                    "The bottle goes into the haul.",
                    Outcome::Keep(StockItem::Junk),
                ),
            ),
            sure("Throw it back", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "A Pirate Wreck",
        floor: 1,
        art: &[
            row("              |\\", "d"),
            row("              |=\\", "dr"),
            row("              |  \\", "d"),
            row("         _____|___\\____", "n"),
            row("         \\  o   o   o /", "n"),
            row("          \\__________/", "n"),
            row("", " "),
            row("       .:.:.:.:.:.:.:.:.:.", "k"),
        ],
        text: "A pirate wreck lists in the sand. Its hold is open and its flag is furled.",
        choices: &[
            checked(
                "Search the hold",
                Check::Size,
                turn(
                    "{fish} slips through a porthole and comes back with something.",
                    Outcome::Treasure(Rarity::Rare),
                ),
                turn("{fish} gets stuck in a porthole.", Outcome::Hurt(30)),
            ),
            checked(
                "Raise the flag",
                Check::Luck,
                turn("Gold rattles down from the crow's nest.", Outcome::Gold(1)),
                turn(
                    "The flag calls the crew back. Two Ghost Deckhands rise.",
                    Outcome::Fight {
                        enemy: "Ghost Deckhand",
                        count: 2,
                    },
                ),
            ),
            sure("Leave", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "La Pincoya Dances",
        floor: 1,
        art: &[
            row("            ,~~,", "y"),
            row("           ( °° )", "yeey"),
            row("          --|  |--", "e"),
            row("            )~~(", "B"),
            row("           /    \\", "B"),
            row("          ><    ><", "B"),
            row("", " "),
            row("     ~  ~~   ~   ~~  ~", "B"),
        ],
        text: "La Pincoya dances on the shore. Facing the sea, she brings plenty; facing the land, she takes it.",
        choices: &[
            checked(
                "Watch",
                Check::Luck,
                turn(
                    "She dances facing the sea. The floor's chests will hold more.",
                    Outcome::Omen(Omen::ChestsRicher),
                ),
                turn(
                    "She dances facing the land. The floor's chests will hold less.",
                    Outcome::Omen(Omen::ChestsPoorer),
                ),
            ),
            sure(
                "Sing to her",
                Need::Trait(Trait::Parrot),
                turn(
                    "She turns to the song and dances facing the sea.",
                    Outcome::Omen(Omen::ChestsRicher),
                ),
            ),
            sure("Leave", Need::Nothing, LEAVE),
        ],
    },
];

pub fn of_floor(depth: u32) -> Vec<&'static EventSpec> {
    EVENTS.iter().filter(|event| event.floor == depth).collect()
}

pub fn find(name: &str) -> Option<&'static EventSpec> {
    EVENTS.iter().find(|event| event.name == name)
}

pub fn luck(kits: &[Kit]) -> i32 {
    let lucky = kits.iter().any(|kit| kit.traits.contains(&Trait::Lucky));
    let golden = kits
        .iter()
        .filter(|kit| kit.traits.contains(&Trait::Golden))
        .count() as i32;
    let doomed = kits
        .iter()
        .filter(|kit| kit.traits.contains(&Trait::Doomed))
        .count() as i32;
    let chance =
        LUCK_PERCENT + i32::from(lucky) * LUCKY_PERCENT + (golden - doomed) * FORTUNE_PERCENT;
    chance.clamp(LUCK_FLOOR_PERCENT, LUCK_CEILING_PERCENT)
}

pub fn lucky(kits: &[Kit], rng: &mut impl RngExt) -> bool {
    rng.random_range(0..PERCENT) < luck(kits)
}

pub fn passes(
    check: Check,
    kits: &[Kit],
    subject: usize,
    size: SizeCategory,
    rng: &mut impl RngExt,
) -> bool {
    let kit = &kits[subject];
    match check {
        Check::Luck => lucky(kits, rng),
        Check::Fortune if kit.traits.contains(&Trait::Golden) => true,
        Check::Fortune if kit.traits.contains(&Trait::Doomed) => false,
        Check::Fortune => lucky(kits, rng),
        Check::Size => match size {
            SizeCategory::S => true,
            SizeCategory::M => lucky(kits, rng),
            SizeCategory::L | SizeCategory::XL => false,
        },
    }
}
