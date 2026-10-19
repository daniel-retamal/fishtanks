use std::mem::discriminant;

use rand::RngExt;
use serde::{Deserialize, Serialize};

use crate::economy::Rarity;
use crate::fishes::graveyard::Fate;
use crate::fishes::mutations::Mutation;
use crate::fishes::species::{FishSpecies, Sin, SizeCategory};
use crate::loot::StockItem;
use crate::void_ritual::{PRAYERS, phrase_matches};

use super::beat::Tone;
use super::bestiary::SpriteRow;
use super::boon::Curse;
use super::effect::StatusKind;
use super::event_art::{
    ANEMONES, BOTTLE, CAPTCHA, CAULDRON, CHOCOLATE_RIVER, CHOIR, COFFIN, CONTRACT, CORAL_MAZE,
    CRASHED_SAUCER, CROP_CIRCLE, DOOR, EMAIL, FERRY, FRESH_GRAVE, GATEKEEPER, GIANT_PEARL,
    GINGERBREAD_HOUSE, GLOWING_POOL, LAB_DOOR, OASIS, PINCOYA, PIRATE_WRECK, PRAYER, REFLECTION,
    SCALES, SEANCE_TABLE, SPECIMEN_TANK, SUGAR_ALTAR, TERMS,
};
use super::fighter::Ghost;
use super::floor::FloorSpec;
use super::items::Item;
use super::kit::Kit;
use super::traits::Trait;
use super::treasure::{Find, Treasure};

pub const LUCK_PERCENT: i32 = 50;
pub const LUCKY_PERCENT: i32 = 15;
pub const FORTUNE_PERCENT: i32 = 10;
pub const LUCK_FLOOR_PERCENT: i32 = 10;
pub const LUCK_CEILING_PERCENT: i32 = 90;
pub const PERCENT: i32 = 100;
pub const FISH: &str = "{fish}";
pub const SEANCE_CALLS: usize = 3;
pub const LINGER_TURNS: u8 = 2;
pub const OVEREAT_SLOW: u32 = 3;
pub const ALTAR_MEALS: u32 = 2;
pub const ANSWER_MEALS: u32 = 2;
pub const OUTSIDE_TURNS: u8 = 3;
pub const ECHOES: u8 = 2;

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
    Speed,
    Bite,
    Has(Trait),
    Lacks(Trait),
    Offering,
    Grudge,
    Sinless,
    EverySinless,
    Feather,
    Sighted,
    Voice,
    Prayer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Subject {
    Lead,
    Smallest,
    Fastest,
    Strongest,
    Drawn,
    Carrier(Trait),
    Kind(FishSpecies),
    Dead,
    Chosen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Need {
    Nothing,
    Trait(Trait),
    Species(FishSpecies),
    Item(Item),
    Dead,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    Gold(u64),
    Treasure(Rarity),
    Fight { enemy: &'static str, count: u8 },
    Encounter,
    Hurt(u32),
    Omen(Omen),
    Keep(StockItem),
    Meals(u32),
    Rest,
    Linger { kind: StatusKind, amount: u32 },
    Curse(Curse),
    Abducted,
    GiveUp(Mutation, Rarity),
    Haunt,
    Answer,
    Mutate,
    MutateAll,
    Lose(u64),
    Mark,
    MarkAll,
    Join,
    Free,
    Outside,
    Waits(&'static str),
    Find(Find),
}

impl Outcome {
    pub fn tone(self) -> Tone {
        match self {
            Outcome::Hurt(_)
            | Outcome::Linger { .. }
            | Outcome::Curse(_)
            | Outcome::Abducted
            | Outcome::Fight { .. }
            | Outcome::Encounter
            | Outcome::Haunt
            | Outcome::Lose(_)
            | Outcome::Mark
            | Outcome::MarkAll
            | Outcome::Outside
            | Outcome::Waits(_) => Tone::Hurt,
            Outcome::Rest | Outcome::Join | Outcome::Free => Tone::Heal,
            Outcome::Mutate
            | Outcome::MutateAll
            | Outcome::Nothing
            | Outcome::Gold(_)
            | Outcome::Treasure(_)
            | Outcome::Omen(_)
            | Outcome::Keep(_)
            | Outcome::Meals(_)
            | Outcome::GiveUp(..)
            | Outcome::Find(_)
            | Outcome::Answer => Tone::Plain,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Turn {
    pub text: &'static str,
    pub outcome: Outcome,
    pub also: Outcome,
}

impl Turn {
    const fn and(self, also: Outcome) -> Turn {
        Turn { also, ..self }
    }

    pub fn outcomes(self) -> [Outcome; 2] {
        [self.outcome, self.also]
    }

    pub fn tone(self) -> Tone {
        let tones = self.outcomes().map(Outcome::tone);
        match tones {
            _ if tones.contains(&Tone::Hurt) => Tone::Hurt,
            _ if tones.contains(&Tone::Heal) => Tone::Heal,
            _ => Tone::Plain,
        }
    }
}

const fn turn(text: &'static str, outcome: Outcome) -> Turn {
    Turn {
        text,
        outcome,
        also: Outcome::Nothing,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChoiceSpec {
    pub label: &'static str,
    pub need: Need,
    pub check: Option<Check>,
    pub subject: Subject,
    pub cost: u32,
    pub pass: Turn,
    pub fail: Turn,
}

impl ChoiceSpec {
    const fn on(self, subject: Subject) -> ChoiceSpec {
        ChoiceSpec { subject, ..self }
    }
}

const fn subject_of(need: Need, check: Option<Check>) -> Subject {
    match (check, need) {
        (Some(Check::Size), _) => Subject::Smallest,
        (Some(Check::Speed), _) => Subject::Fastest,
        (Some(Check::Bite), _) => Subject::Strongest,
        (Some(Check::Lacks(wanted)), _) => Subject::Carrier(wanted),
        (Some(Check::Grudge), _) | (_, Need::Dead) => Subject::Dead,
        (Some(Check::Feather), _) => Subject::Chosen,
        (_, Need::Trait(wanted)) => Subject::Carrier(wanted),
        (_, Need::Species(species)) => Subject::Kind(species),
        _ => Subject::Lead,
    }
}

const fn sure(label: &'static str, need: Need, then: Turn) -> ChoiceSpec {
    ChoiceSpec {
        label,
        need,
        check: None,
        subject: subject_of(need, None),
        cost: 1,
        pass: then,
        fail: then,
    }
}

const fn checked(label: &'static str, check: Check, pass: Turn, fail: Turn) -> ChoiceSpec {
    ChoiceSpec {
        label,
        need: Need::Nothing,
        check: Some(check),
        subject: subject_of(Need::Nothing, Some(check)),
        cost: 1,
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

impl EventSpec {
    pub fn calls_the_dead(&self) -> bool {
        self.choices.iter().any(|choice| choice.need == Need::Dead)
    }

    pub fn picks_a_fish(&self) -> bool {
        self.choices
            .iter()
            .any(|choice| choice.subject == Subject::Chosen)
    }

    pub fn asks_a_prayer(&self) -> Option<usize> {
        self.choices
            .iter()
            .position(|choice| choice.check == Some(Check::Prayer))
    }
}

const LEAVE: Turn = turn("The party swims on.", Outcome::Nothing);

pub const EVENTS: &[EventSpec] = &[
    EventSpec {
        name: "A Note in a Bottle",
        floor: 1,
        art: BOTTLE,
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
        art: PIRATE_WRECK,
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
        art: PINCOYA,
        text: "La Pincoya dances on the shore. Facing the sea, she giveth; facing the land, she taketh away.",
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
    EventSpec {
        name: "The Coral Maze",
        floor: 2,
        art: CORAL_MAZE,
        text: "The reef closes into a maze of coral. Something glitters in the middle of it.",
        choices: &[
            checked(
                "Swim through",
                Check::Speed,
                turn(
                    "{fish} leads the way through before the walls can close.",
                    Outcome::Gold(2),
                ),
                turn(
                    "The party is lost, and something finds it on the way out.",
                    Outcome::Encounter,
                ),
            ),
            sure(
                "Follow the Eelfish",
                Need::Species(FishSpecies::Eelfish),
                turn(
                    "{fish} goes through the walls, and the maze gives up its middle.",
                    Outcome::Treasure(Rarity::Rare),
                ),
            ),
            sure("Go around", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "A Giant Pearl",
        floor: 2,
        art: GIANT_PEARL,
        text: "A clam as big as a boat lies open. The pearl inside is the size of a fish.",
        choices: &[
            checked(
                "Reach in",
                Check::Bite,
                turn(
                    "{fish} pries the pearl loose before the clam notices.",
                    Outcome::Treasure(Rarity::Rare),
                ),
                turn(
                    "The clam snaps shut on {fish}. It will be held at the start of the next fight.",
                    Outcome::Linger {
                        kind: StatusKind::Held,
                        amount: 0,
                    },
                ),
            ),
            sure(
                "Tickle the clam",
                Need::Species(FishSpecies::Kissfish),
                turn(
                    "{fish} tickles the clam until it laughs the pearl out.",
                    Outcome::Treasure(Rarity::Rare),
                ),
            ),
            sure("Leave", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "The Anemone Garden",
        floor: 2,
        art: ANEMONES,
        text: "A garden of anemones sways in warm water. It looks like a good place to rest.",
        choices: &[
            checked(
                "Rest among them",
                Check::Lacks(Trait::Delicious),
                turn(
                    "The party rests among the anemones.",
                    Outcome::Rest,
                ),
                turn(
                    "An anemone tries to eat {fish}, who tasted too good to leave alone.",
                    Outcome::Hurt(30),
                ),
            ),
            sure("Pass by", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "A Chocolate River",
        floor: 3,
        art: CHOCOLATE_RIVER,
        text: "A river of warm chocolate runs across the floor.",
        choices: &[
            checked(
                "Drink",
                Check::Lacks(Trait::GluttonySin),
                turn("The party drinks, and feels better for it.", Outcome::Rest),
                turn(
                    "{fish} drinks the river half dry. It will be slow in the next fight.",
                    Outcome::Linger {
                        kind: StatusKind::Slowed,
                        amount: OVEREAT_SLOW,
                    },
                ),
            ),
            checked(
                "Swim across",
                Check::Speed,
                turn(
                    "{fish} crosses first and finds chocolate coins on the far bank.",
                    Outcome::Gold(2),
                ),
                turn(
                    "The current is thick, and it carries {fish} into the rocks.",
                    Outcome::Hurt(20),
                ),
            ),
            sure("Walk the bank", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "A Gingerbread House",
        floor: 3,
        art: GINGERBREAD_HOUSE,
        text: "A gingerbread house stands on the sugar sand. There is a light in the window.",
        choices: &[
            checked(
                "Knock",
                Check::Luck,
                turn(
                    "Nobody is home, and the door was never locked.",
                    Outcome::Treasure(Rarity::Rare),
                ),
                turn(
                    "A Gingerbread Shark is home.",
                    Outcome::Fight {
                        enemy: "Gingerbread Shark",
                        count: 1,
                    },
                ),
            ),
            checked(
                "Eat the walls",
                Check::Luck,
                turn("The walls are good. The party eats its fill.", Outcome::Rest),
                turn(
                    "A Gingerbread Shark is home, and it was eating them first.",
                    Outcome::Fight {
                        enemy: "Gingerbread Shark",
                        count: 1,
                    },
                ),
            ),
            sure("Leave", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "The Sugar Altar",
        floor: 3,
        art: SUGAR_ALTAR,
        text: "An altar of spun sugar. Something sweet is wanted on it.",
        choices: &[
            ChoiceSpec {
                label: "Offer two meals",
                need: Need::Item(Item::Food),
                check: Some(Check::Offering),
                subject: Subject::Lead,
                cost: ALTAR_MEALS,
                pass: turn(
                    "The altar takes the food and gives back something better.",
                    Outcome::Treasure(Rarity::Rare),
                ),
                fail: turn(
                    "It was not enough. A Cotton Candy Jelly takes the offering and stays.",
                    Outcome::Fight {
                        enemy: "Cotton Candy Jelly",
                        count: 1,
                    },
                ),
            },
            sure(
                "Offer a fish's sweetness",
                Need::Trait(Trait::Sweet(0)),
                turn(
                    "{fish} gives up its sweetness. The altar gives back something rare.",
                    Outcome::GiveUp(Mutation::Strawberry, Rarity::Legendary),
                ),
            ),
            sure("Leave", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "The Séance Table",
        floor: 4,
        art: SEANCE_TABLE,
        text: "Three candles, a board of letters, a planchette shaped like a fin. The names of your dead are carved round the edge.",
        choices: &[
            ChoiceSpec {
                label: "Call",
                need: Need::Dead,
                check: Some(Check::Grudge),
                subject: Subject::Dead,
                cost: 0,
                pass: turn("The planchette moves.", Outcome::Answer),
                fail: turn(
                    "The candles go out. {fish} remembers, and it is angry.",
                    Outcome::Haunt,
                ),
            },
            sure("Blow out the candles", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "A Fresh Grave",
        floor: 4,
        art: FRESH_GRAVE,
        text: "A fresh grave. The name on the stone is {fish}.",
        choices: &[
            checked(
                "Look in",
                Check::Fortune,
                turn(
                    "{fish} looks in. Someone buried their gold here by mistake.",
                    Outcome::Gold(2),
                ),
                turn(
                    "{fish} looks in, and something looks back. It comes out Doomed.",
                    Outcome::Curse(Curse::Doomed),
                ),
            )
            .on(Subject::Drawn),
            sure("Fill it in", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "A Coffin",
        floor: 4,
        art: COFFIN,
        text: "A coffin rests on two stones. The lid is loose.",
        choices: &[
            checked(
                "Open it",
                Check::Luck,
                turn(
                    "Grave goods, and nobody left to want them.",
                    Outcome::Treasure(Rarity::Rare),
                ),
                turn(
                    "A Vampire Squid wakes.",
                    Outcome::Fight {
                        enemy: "Vampire Squid",
                        count: 1,
                    },
                ),
            ),
            sure(
                "Nail it shut",
                Need::Item(Item::Stock(StockItem::Junk)),
                turn(
                    "Nailed shut with Junk. On the lid, a coin for the ferryman.",
                    Outcome::Gold(1),
                ),
            ),
            sure("Leave", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "A Crop Circle",
        floor: 5,
        art: CROP_CIRCLE,
        text: "Rings pressed into the sand, perfect and fresh. The air hums above the middle.",
        choices: &[
            checked(
                "Stand in the middle",
                Check::Has(Trait::SeenTheSky),
                turn(
                    "A light finds {fish}, who has seen the sky before, and lets go. Something is left in the sand.",
                    Outcome::Treasure(Rarity::Rare),
                ),
                turn(
                    "A light takes {fish}. It will wake at home, in an alien base.",
                    Outcome::Abducted,
                ),
            )
            .on(Subject::Drawn),
            sure("Walk around it", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "An Oasis",
        floor: 5,
        art: OASIS,
        text: "Water and a palm, in the middle of the sand.",
        choices: &[
            checked(
                "Drink",
                Check::Luck,
                turn("The water is real, and cold.", Outcome::Rest),
                turn(
                    "It was a Mirage.",
                    Outcome::Fight {
                        enemy: "Mirage",
                        count: 2,
                    },
                ),
            ),
            sure(
                "Send the Blindfish first",
                Need::Species(FishSpecies::Blindfish),
                turn(
                    "{fish} cannot be fooled by what it cannot see. The water is real.",
                    Outcome::Rest,
                ),
            ),
            sure("Leave", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "A Crashed Saucer",
        floor: 5,
        art: CRASHED_SAUCER,
        text: "A saucer lies half buried, still warm. The pilot is slumped over the controls.",
        choices: &[
            checked(
                "Loot it",
                Check::Speed,
                turn(
                    "{fish} is in and out before the pilot wakes.",
                    Outcome::Treasure(Rarity::Rare),
                ),
                turn(
                    "The pilot is armed. Two Greys climb out.",
                    Outcome::Fight {
                        enemy: "Grey",
                        count: 2,
                    },
                ),
            ),
            sure(
                "Help the pilot",
                Need::Nothing,
                turn(
                    "The pilot wakes long enough to press a coin into {fish}'s fin.",
                    Outcome::Gold(1),
                ),
            ),
            sure("Leave", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "A Glowing Pool",
        floor: 6,
        art: GLOWING_POOL,
        text: "A pool of green water, warm to the fin. Whatever swims in it comes out changed.",
        choices: &[
            sure(
                "Dip one fish",
                Need::Nothing,
                turn("{fish} goes under and comes up changed.", Outcome::Mutate),
            )
            .on(Subject::Chosen),
            sure(
                "Dip everyone",
                Need::Nothing,
                turn(
                    "The whole party goes under and comes up changed.",
                    Outcome::MutateAll,
                ),
            ),
            sure("Leave", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "A Locked Lab Door",
        floor: 6,
        art: LAB_DOOR,
        text: "A steel door with a warning painted on it. Behind it, something hums.",
        choices: &[
            checked(
                "Force it",
                Check::Bite,
                turn(
                    "{fish} bends the door open. The lab inside is empty, its shelves are not.",
                    Outcome::Find(Find::Vial),
                ),
                turn(
                    "The alarm goes off, and Patient Zero answers it.",
                    Outcome::Fight {
                        enemy: "Patient Zero",
                        count: 1,
                    },
                ),
            ),
            sure(
                "Let a botfish open it",
                Need::Species(FishSpecies::Botfish),
                turn(
                    "{fish} talks to the lock until it opens. Nobody hears a thing, and the samples are still in the fridge.",
                    Outcome::Find(Find::Fusion),
                ),
            ),
            sure("Leave", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "A Specimen Tank",
        floor: 6,
        art: SPECIMEN_TANK,
        text: "A cracked tank holds a fish of {fish}'s kind, grown wrong.",
        choices: &[
            checked(
                "Free it",
                Check::Luck,
                turn(
                    "The glass gives. It swims out and joins the party, changed for good.",
                    Outcome::Free,
                ),
                turn(
                    "It is not friendly any more.",
                    Outcome::Fight {
                        enemy: "The Specimen",
                        count: 1,
                    },
                ),
            )
            .on(Subject::Drawn),
            sure("Leave it", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "An Email",
        floor: 7,
        art: EMAIL,
        text: "An email floats by with an attachment. The subject line is your name.",
        choices: &[
            checked(
                "Open it",
                Check::Luck,
                turn(
                    "It was an invoice, paid to you by mistake.",
                    Outcome::Gold(2),
                ),
                turn(
                    "Phishing. Gold leaves the haul before anyone can stop it.",
                    Outcome::Lose(2),
                ),
            ),
            sure("Delete it", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "Select Every Square With a Fish",
        floor: 7,
        art: CAPTCHA,
        text: "A grid of squares blocks the way. To pass, select every square with a fish.",
        choices: &[
            checked(
                "Answer",
                Check::Sighted,
                turn(
                    "{fish} picks every square. Access granted.",
                    Outcome::Treasure(Rarity::Rare),
                ),
                turn(
                    "{fish} cannot see the squares. Locked out, the party takes the long way round.",
                    Outcome::Encounter,
                ),
            ),
            sure(
                "Let the Cheatfish do it",
                Need::Species(FishSpecies::Cheatfish),
                turn(
                    "{fish} does not even look. Access granted.",
                    Outcome::Treasure(Rarity::Rare),
                ),
            ),
            sure("Go around", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "The Terms and Conditions",
        floor: 7,
        art: TERMS,
        text: "Four thousand pages of terms and conditions, and a button that says Accept.",
        choices: &[
            sure(
                "Accept",
                Need::Nothing,
                turn(
                    "{fish} accepts. Something is delivered, and on page 4,000 {fish} signed something else.",
                    Outcome::Treasure(Rarity::Rare),
                )
                .and(Outcome::Curse(Curse::FinePrint)),
            ),
            sure(
                "Scroll to the end",
                Need::Nothing,
                turn(
                    "{fish} reads every page and declines the clause on page 4,000. It will be tired in the next fight.",
                    Outcome::Treasure(Rarity::Rare),
                )
                .and(Outcome::Linger {
                    kind: StatusKind::Held,
                    amount: 0,
                }),
            ),
            sure("Close it", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "A Contract in Your Name",
        floor: 8,
        art: CONTRACT,
        text: "A contract floats in the heat, already signed in your name.",
        choices: &[
            checked(
                "Read the fine print",
                Check::Sinless,
                turn(
                    "{fish} finds the loophole. The contract pays out.",
                    Outcome::Gold(2),
                ),
                turn(
                    "The fine print reads {fish} back to itself. It is marked.",
                    Outcome::Mark,
                ),
            ),
            sure(
                "Burn it",
                Need::Species(FishSpecies::Holyfish),
                turn(
                    "{fish} burns the contract. Something below was cheated, and it left its stake behind.",
                    Outcome::Treasure(Rarity::Rare),
                ),
            ),
            sure("Leave it", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "The Ferry",
        floor: 8,
        art: FERRY,
        text: "A ferry waits at the edge of a burning river. The ferryman holds out a hand.",
        choices: &[
            sure(
                "Pay the fare",
                Need::Nothing,
                turn(
                    "The ferryman takes the fare. On the far bank, something was left for whoever paid.",
                    Outcome::Lose(1),
                )
                .and(Outcome::Treasure(Rarity::Rare)),
            ),
            checked(
                "Swim across",
                Check::Luck,
                turn(
                    "The river lets the party through. On the far bank, something waits.",
                    Outcome::Treasure(Rarity::Rare),
                ),
                turn(
                    "The swimmers arrive, and every one of them is marked.",
                    Outcome::MarkAll,
                )
                .and(Outcome::Treasure(Rarity::Rare)),
            ),
            sure("Stay on this side", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "A Cauldron",
        floor: 8,
        art: CAULDRON,
        text: "A cauldron simmers over the coals. A fish inside is still swimming.",
        choices: &[
            checked(
                "Pull it out",
                Check::Bite,
                turn(
                    "{fish} pulls it out of the broth. It joins the party.",
                    Outcome::Join,
                ),
                turn(
                    "A Lava Eel was keeping it warm.",
                    Outcome::Fight {
                        enemy: "Lava Eel",
                        count: 1,
                    },
                ),
            ),
            sure("Leave it", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "The Gatekeeper",
        floor: 9,
        art: GATEKEEPER,
        text: "The Gatekeeper stands at a gate of light. It wants to read every fish before it lets them through.",
        choices: &[
            checked(
                "Let it read each fish",
                Check::EverySinless,
                turn(
                    "It reads every fish and finds nothing. The party goes through blessed.",
                    Outcome::Rest,
                ),
                turn(
                    "It names every sin it finds. Those fish will wait outside the next fight.",
                    Outcome::Outside,
                ),
            ),
            sure(
                "Show a halo",
                Need::Trait(Trait::Holy),
                turn(
                    "{fish} shows its halo. The Gatekeeper bows, and the party goes through blessed.",
                    Outcome::Rest,
                ),
            ),
            sure(
                "Argue",
                Need::Trait(Trait::PrideSin),
                turn(
                    "{fish} argues until the Gatekeeper gives up. Everyone goes through.",
                    Outcome::Nothing,
                ),
            ),
            sure("Turn back", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "The Scales",
        floor: 9,
        art: SCALES,
        text: "A pair of scales, a feather on one pan. The other pan is empty and waiting.",
        choices: &[
            checked(
                "Weigh a fish against the feather",
                Check::Feather,
                turn(
                    "{fish} weighs no more than the feather, and the feather is given to it.",
                    Outcome::Find(Find::Exactly(Treasure::Feather)),
                ),
                turn(
                    "{fish} is heavier than the feather. It is found wanting.",
                    Outcome::Curse(Curse::Wanting),
                ),
            ),
            sure("Refuse", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "Choir Practice",
        floor: 9,
        art: CHOIR,
        text: "A choir is practising in the clouds. There is room for one more voice.",
        choices: &[
            checked(
                "Sing along",
                Check::Voice,
                turn(
                    "{fish} sings in tune. The hymn rests the whole party.",
                    Outcome::Rest,
                ),
                turn(
                    "{fish} sings off key. The Choir notices you, and it will be waiting at the elevator.",
                    Outcome::Waits("The Choir"),
                ),
            ),
            sure("Listen", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "A Door in the Water",
        floor: 10,
        art: DOOR,
        text: "A door stands in the water, with nothing on either side of it.",
        choices: &[
            checked(
                "Open it",
                Check::Luck,
                turn(
                    "On the other side, something left for whoever opened it.",
                    Outcome::Treasure(Rarity::Legendary),
                ),
                turn(
                    "A Door steps through.",
                    Outcome::Fight {
                        enemy: "Door",
                        count: 1,
                    },
                ),
            ),
            sure(
                "Knock",
                Need::Nothing,
                turn(
                    "Something knocks back, twice, and slides a coin under the door.",
                    Outcome::Gold(1),
                ),
            ),
            sure("Leave", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "A Reflection",
        floor: 10,
        art: REFLECTION,
        text: "A reflection of the party hangs in the water. It is almost you.",
        choices: &[
            checked(
                "Talk to it",
                Check::Has(Trait::Stare),
                turn(
                    "{fish} stares it down. The reflection looks away first and gives something up.",
                    Outcome::Treasure(Rarity::Legendary),
                ),
                turn(
                    "It talks back. Echoes of your fish climb out.",
                    Outcome::Fight {
                        enemy: "Echo",
                        count: ECHOES,
                    },
                ),
            )
            .on(Subject::Carrier(Trait::Stare)),
            sure(
                "Break it",
                Need::Nothing,
                turn(
                    "It breaks into shards, and one of them is still almost you.",
                    Outcome::Find(Find::Exactly(Treasure::WishShard)),
                ),
            ),
            sure("Leave", Need::Nothing, LEAVE),
        ],
    },
    EventSpec {
        name: "A Line of Prayer",
        floor: 10,
        art: PRAYER,
        text: "A line of prayer is written on the dark. The Void waits for someone to say it.",
        choices: &[
            ChoiceSpec {
                label: "Say it",
                need: Need::Nothing,
                check: Some(Check::Prayer),
                subject: Subject::Lead,
                cost: 1,
                pass: turn(
                    "{fish} says it. The Void listens, and gives.",
                    Outcome::Find(Find::Exactly(Treasure::WishShard)),
                ),
                fail: turn(
                    "{fish} says the wrong word, and the word stays with it.",
                    Outcome::Curse(Curse::WrongWord),
                ),
            },
            sure("Stay silent", Need::Nothing, LEAVE),
        ],
    },
];

pub fn answer(sin: Option<Sin>) -> Turn {
    match sin {
        Some(Sin::Greed) => turn("{fish} spells out where it hid its gold.", Outcome::Gold(2)),
        Some(Sin::Lust) => turn(
            "{fish} spells out a blessing, and the party feels it.",
            Outcome::Rest,
        ),
        Some(Sin::Gluttony) => turn(
            "{fish} spells F-O-O-D. There are two meals under the table.",
            Outcome::Meals(ANSWER_MEALS),
        ),
        Some(Sin::Sloth) => turn(
            "The planchette barely moves. {fish} is resting.",
            Outcome::Nothing,
        ),
        Some(Sin::Pride) => turn(
            "{fish} spells out its own name, again and again.",
            Outcome::Nothing,
        ),
        Some(Sin::Wrath | Sin::Envy) => turn("{fish} says nothing.", Outcome::Nothing),
        None => turn(
            "{fish} spells Y-E-S, and the board gives up what it was holding.",
            Outcome::Treasure(Rarity::Rare),
        ),
    }
}

pub fn holds_a_grudge(ghost: &Ghost) -> bool {
    ghost.fate == Some(Fate::Sold) || matches!(ghost.sin, Some(Sin::Wrath | Sin::Envy))
}

pub fn of_floor(depth: u32) -> Vec<&'static EventSpec> {
    let table = FloorSpec::of(depth).depth;
    EVENTS.iter().filter(|event| event.floor == table).collect()
}

pub fn prayer_line(seed: u64) -> &'static str {
    let verses = PRAYERS[(seed as usize) % PRAYERS.len()];
    verses[(seed as usize / PRAYERS.len()) % verses.len()]
}

pub fn heard(words: &str, seed: u64) -> bool {
    phrase_matches(words, prayer_line(seed))
}

pub fn sinful(kit: &Kit) -> bool {
    kit.traits.iter().any(|worn| SINS.contains(worn))
}

const SINS: [Trait; 7] = [
    Trait::GreedSin,
    Trait::LustSin,
    Trait::GluttonySin,
    Trait::WrathSin,
    Trait::EnvySin,
    Trait::PrideSin,
    Trait::SlothSin,
];

pub fn find(name: &str) -> Option<&'static EventSpec> {
    EVENTS.iter().find(|event| event.name == name)
}

pub fn alike(wanted: Trait, worn: Trait) -> bool {
    discriminant(&wanted) == discriminant(&worn)
}

pub fn wears(kit: &Kit, wanted: Trait) -> bool {
    kit.traits.iter().any(|&worn| alike(wanted, worn))
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

pub fn pace(depth: u32) -> u32 {
    FloorSpec::of(depth)
        .commons()
        .iter()
        .map(|spec| spec.speed)
        .max()
        .unwrap_or(0)
}

pub fn grip(depth: u32) -> u32 {
    FloorSpec::of(depth)
        .commons()
        .iter()
        .map(|spec| spec.bite)
        .max()
        .unwrap_or(0)
}

pub struct Trial<'a> {
    pub kits: &'a [Kit],
    pub standing: &'a [usize],
    pub subject: usize,
    pub size: SizeCategory,
    pub depth: u32,
    pub offered: bool,
    pub light: bool,
    pub said: bool,
}

pub fn passes(check: Check, trial: &Trial, rng: &mut impl RngExt) -> bool {
    let kit = &trial.kits[trial.subject];
    match check {
        Check::Luck => lucky(trial.kits, rng),
        Check::Fortune if kit.traits.contains(&Trait::Golden) => true,
        Check::Fortune if kit.traits.contains(&Trait::Doomed) => false,
        Check::Fortune => lucky(trial.kits, rng),
        Check::Size => match trial.size {
            SizeCategory::S => true,
            SizeCategory::M => lucky(trial.kits, rng),
            SizeCategory::L | SizeCategory::XL => false,
        },
        Check::Speed => kit.speed >= pace(trial.depth),
        Check::Bite => kit.bite >= grip(trial.depth),
        Check::Has(wanted) => wears(kit, wanted),
        Check::Lacks(wanted) => !trial
            .standing
            .iter()
            .any(|&member| wears(&trial.kits[member], wanted)),
        Check::Offering => trial.offered,
        Check::Grudge => true,
        Check::Sinless => !sinful(kit),
        Check::EverySinless => !trial
            .standing
            .iter()
            .any(|&member| sinful(&trial.kits[member])),
        Check::Feather => trial.light,
        Check::Sighted => !wears(kit, Trait::Contact(0)),
        Check::Voice => {
            let sings = trial.standing.iter().any(|&member| {
                let kit = &trial.kits[member];
                wears(kit, Trait::Parrot) || wears(kit, Trait::Loud)
            });
            sings || lucky(trial.kits, rng)
        }
        Check::Prayer => trial.said,
    }
}
