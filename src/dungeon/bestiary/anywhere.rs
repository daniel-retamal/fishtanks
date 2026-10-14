use super::{BLANK, EnemySkill, EnemySpec, Loot, Passive, Rank, row};
use crate::dungeon::effect::{DamageKind, Effect, Power, Reach, Special, StatusKind};
use crate::dungeon::traits::Trait;

pub const ROWS: &[EnemySpec] = &[
    EnemySpec {
        name: "Mimic",
        floor: 0,
        rank: Rank::Common,
        hp: 0,
        mp: 2,
        bite: 0,
        speed: 6,
        sprite: &[
            row("  _________", "n"),
            row(" /VVVVVVVVV\\", "w"),
            row("|===[°]=====|", "n"),
        ],
        skills: &[EnemySkill {
            name: "Snap",
            cost: 2,
            reach: Reach::OneFoe,
            effects: &[Effect::Hit {
                power: Power::bite_times(2),
                kind: DamageKind::Bite,
                hits: 1,
            }],
            about: "Bite ×2 to one fish",
        }],
        passives: &[
            Passive {
                name: "Scaled",
                about: "HP 40 + 15 a floor, Bite 8 + 2 a floor",
            },
            Passive {
                name: "First Bite",
                about: "its first bite lands before the fight begins",
            },
        ],
        traits: &[
            Trait::Scaled {
                hp: 40,
                hp_per_floor: 15,
                bite: 8,
                bite_per_floor: 2,
            },
            Trait::FirstStrike,
        ],
        loot: &[],
        variant_of: None,
        note: "In any Sunken Chest, sometimes.",
    },
    EnemySpec {
        name: "Lurker",
        floor: 0,
        rank: Rank::Common,
        hp: 0,
        mp: 2,
        bite: 0,
        speed: 9,
        sprite: &[
            row("  .-\"-.", "c"),
            row(" /°   °\\", "d"),
            row(" \\ vvv /", "d"),
        ],
        skills: &[EnemySkill {
            name: "Cling",
            cost: 2,
            reach: Reach::OneFoe,
            effects: &[Effect::inflict(StatusKind::Held, 2, 0)],
            about: "holds a fish two turns",
        }],
        passives: &[
            Passive {
                name: "Scaled",
                about: "HP 30 + 12 a floor, Bite 7 + 2 a floor",
            },
            Passive {
                name: "Ambush",
                about: "always acts first in the fight it starts",
            },
        ],
        traits: &[
            Trait::Scaled {
                hp: 30,
                hp_per_floor: 12,
                bite: 7,
                bite_per_floor: 2,
            },
            Trait::FirstStrike,
        ],
        loot: &[],
        variant_of: None,
        note: "In any Bell Jar, sometimes.",
    },
    EnemySpec {
        name: "Door",
        floor: 0,
        rank: Rank::Common,
        hp: 0,
        mp: 2,
        bite: 0,
        speed: 5,
        sprite: &[
            row("  _______", "n"),
            row(" |  ___  |", "n"),
            row(" | |   | |", "n"),
            row(" | |___| |", "n"),
            row(" |     o |", "nnnnnnnynn"),
            row(" |_______|", "n"),
        ],
        skills: &[EnemySkill {
            name: "Slam",
            cost: 2,
            reach: Reach::OneFoe,
            effects: &[Effect::Hit {
                power: Power::bite_times(2),
                kind: DamageKind::Bite,
                hits: 1,
            }],
            about: "Bite ×2 to one fish",
        }],
        passives: &[Passive {
            name: "Scaled",
            about: "HP 40 + 15 a floor, Bite 8 + 2 a floor",
        }],
        traits: &[Trait::Scaled {
            hp: 40,
            hp_per_floor: 15,
            bite: 8,
            bite_per_floor: 2,
        }],
        loot: &[Loot::Rare],
        variant_of: None,
        note: "Behind a door in the water, sometimes.",
    },
    EnemySpec {
        name: "Echo",
        floor: 0,
        rank: Rank::Common,
        hp: 0,
        mp: 1,
        bite: 0,
        speed: 7,
        sprite: &[BLANK, row("~ ~ ~ ~", "b"), BLANK],
        skills: &[EnemySkill {
            name: "Borrow",
            cost: 1,
            reach: Reach::OneFoe,
            effects: &[Effect::Special(Special::Borrow)],
            about: "bites with that fish's Bite",
        }],
        passives: &[
            Passive {
                name: "Scaled",
                about: "HP 20 + 8 a floor",
            },
            Passive {
                name: "Shimmer",
                about: "it looks like one of your fish",
            },
        ],
        traits: &[
            Trait::Scaled {
                hp: 20,
                hp_per_floor: 8,
                bite: 0,
                bite_per_floor: 0,
            },
            Trait::Mirage,
            Trait::NeverBites,
        ],
        loot: &[],
        variant_of: None,
        note: "Out of a reflection that is almost you.",
    },
];
