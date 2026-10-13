use super::bestiary::{SpriteRow, row};
use super::map::RoomKind;
use super::room::{Scene, Stage};

pub fn of(kind: RoomKind, stage: &Stage) -> Scene {
    let rows = match (kind, stage) {
        (RoomKind::Chest, Stage::Sprung) => MIMIC,
        (RoomKind::Chest, _) => CHEST,
        (RoomKind::BellJar, Stage::Sprung) => LURKER_JAR,
        (RoomKind::BellJar, _) => BELL_JAR,
        (RoomKind::Den, _) => DEN,
        (RoomKind::School, _) => SCHOOL,
        (RoomKind::Shrine, _) => SHRINE,
        (RoomKind::Altar, _) => ALTAR,
        (RoomKind::Laboratory, _) => LABORATORY,
        (RoomKind::Vent, _) => VENT,
        (RoomKind::Cleaning, _) => CLEANING,
        (RoomKind::Wreck, _) => WRECK,
        (RoomKind::Elevator, _) => ELEVATOR,
        (RoomKind::Encounter | RoomKind::Boss | RoomKind::Event | RoomKind::Vending, _) => &[],
    };
    Scene::Rows(rows)
}

pub const ALL: &[&[SpriteRow]] = &[
    DEN, SCHOOL, SHRINE, ALTAR, LABORATORY, VENT, CLEANING, WRECK, ELEVATOR, CHEST, MIMIC,
    LURKER_JAR, BELL_JAR,
];

pub const LIFT_FROM: (usize, usize) = (1, 13);
pub const LIFT_ARROW: (usize, usize) = (1, 18);
pub const LIFT_TO: (usize, usize) = (1, 23);

pub const DEN: &[SpriteRow] = &[
    row("                      :               `:", "k"),
    row("                           : :     . :", "k"),
    row("      :                   .", "k"),
    row("         .                        .        `", "k"),
    row("  :              :", "k"),
    row("           °            °  .", "kkkkkkkkkkkykkkkkkkkkkkkyk"),
    row(":                                     `", "k"),
    row(" `", "k"),
    row("          :              .      .    .", "k"),
    row("      :                 `      :", "k"),
    row("  `     .", "k"),
    row("       `", "k"),
    row(
        "     <°}}}<    <°}}}}<     <°}}<      `",
        "ddddddddddddddddddddddddddddddddddddddk",
    ),
    row("                   `  `          :", "k"),
    row(" ‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾", "k"),
];

pub const SCHOOL: &[SpriteRow] = &[
    row("<>)))*>", "o"),
    row("", "d"),
    row("              <>)═)°>", "B"),
    row("", "d"),
    row("", "d"),
    row("  <>««°>", "t"),
    row("", "d"),
    row("                         |\\", "n"),
    row("                  <>)))))))º>", "n"),
    row("", "d"),
    row("    <>)))*>", "o"),
];

pub const SHRINE: &[SpriteRow] = &[
    row("      \\    |    /", "y"),
    row("            +", "y"),
    row("           /|\\", "y"),
    row("         _/ | \\_", "y"),
    row("       _/       \\_", "y"),
    row("     _/___________\\_", "y"),
    row("      |  .-‾‾‾-.  |", "eeeeeeeeyyyyyyye"),
    row("      | (  ‿‿‿  ) |", "eeeeeeeeeyyyyye"),
    row("      | ><)))))°> |", "eeeeeeeewwwwwwwwwe"),
    row("      |  _______  |", "e"),
    row(" i    |_|_______|_|    i", "eoeeeeeeeeeeeeeeeeeeeoe"),
    row(" |   [=============]   |", "e"),
    row("_|_                   _|_", "e"),
];

pub const ALTAR: &[SpriteRow] = &[
    row("     )                       (", "o"),
    row("    (_)     .---------.     (_)", "dddddddddddeeeeeeeeeeed"),
    row("    |#|     | ~~~~~~~ |     |#|", "ddddddddddddeeeeeeeeed"),
    row("    |#|     | ~~~~~~~ |     |#|", "ddddddddddddeeeeeeeeed"),
    row("    |#|     | x______ |     |#|", "ddddddddddddeeeeeeeeed"),
    row(
        " ___|#|_____'---------'_____|#|___",
        "ssssssssssseeeeeeeeeees",
    ),
    row("|                                  |", "s"),
    row(
        "|           .-‾‾‾‾‾‾-.             |",
        "sssssssssRRRRRRRRRRRRRRRRs",
    ),
    row(
        "|          / \\      / \\            |",
        "sssssssssRRRRRRRRRRRRRRRRs",
    ),
    row(
        "|         | ‾‾‾\\  /‾‾‾ |           |",
        "sssssssssRRRRRRRRRRRRRRRRs",
    ),
    row(
        "|         |  \\  \\/  /  |           |",
        "sssssssssRRRRRRRRRRRRRRRRs",
    ),
    row(
        "|          \\  \\_/\\_/  /            |",
        "sssssssssRRRRRRRRRRRRRRRRs",
    ),
    row(
        "|___________'-.____.-'_____________|",
        "sssssssssRRRRRRRRRRRRRRRRs",
    ),
];

pub const LABORATORY: &[SpriteRow] = &[
    row(".---.       .---.       .---.", "s"),
    row("|   |       |   |       |   |", "s"),
    row("| ° |       |   |       |   |", "ssGs"),
    row("|   |       | ≡ |       |   |", "ssssssssssssssGs"),
    row(
        "|   |       |   |       | ¤ |",
        "ssssssssssssssssssssssssssGs",
    ),
    row(
        "|~~~|       |~~~|       |~~~|",
        "sGGGsssssssssGGGsssssssssGGGs",
    ),
    row(
        "|~~~|       |~~~|       |~~~|",
        "sGGGsssssssssGGGsssssssssGGGs",
    ),
    row("'---'       '---'       '---'", "s"),
    row("", "G"),
    row("", "G"),
    row(" <=o=>   ·   ·    ·", "G"),
];

pub const VENT: &[SpriteRow] = &[
    row("              o   °", "c"),
    row("            °   o   °", "c"),
    row("", ""),
    row("               ~~~~", "o"),
    row("             /      \\", "n"),
    row("            /   ▒▒   \\", "nnnnnnnnnnnnnnnnoon"),
    row("           /__________\\", "n"),
    row("", ""),
    row("", ""),
    row("}><)))))))°>      <°((((((><", "rrrrrrrrrrrrs"),
];

pub const CLEANING: &[SpriteRow] = &[
    row("      ^ ^ ^ ^", "e"),
    row("   ><))))))))°>", "r"),
    row("      v v v v", "e"),
    row("", "d"),
    row("", "d"),
    row("     ,,=<    >=,,", "R"),
    row("", "d"),
    row(" .-~~~~~~~~~~~~~~~~-.", "d"),
    row("(                    )", "d"),
];

pub const WRECK: &[SpriteRow] = &[
    row("             |\\", "d"),
    row("             | \\", "d"),
    row("          ___|__\\___", "n"),
    row("          \\_o___o__/", "n"),
    row("", "d"),
    row("", "d"),
    row("  <°(((x><   <°((x><", "d"),
    row("", "d"),
    row(".:.:.:.:.:.:.:.:.:.:.:.:.", "k"),
];

pub const ELEVATOR: &[SpriteRow] = &[
    row("    ┌─────────────────────────────┐", "s"),
    row("    │        4    ▼    5          │", "s"),
    row("    ├──────────────┬┬─────────────┤", "s"),
    row("    │              ││             │", "s"),
    row("    │              ││             │", "s"),
    row("    │              ││             │  ┌───┐", "s"),
    row("    │              ││             │  │ ▲ │", "s"),
    row("    │              ││             │  │ ▼ │", "s"),
    row("    │              ││             │  └───┘", "s"),
    row("    │              ││             │", "s"),
    row("    │              ││             │", "s"),
    row("    │              ││             │", "s"),
    row("    │              ││             │", "s"),
    row("    └──────────────┴┴─────────────┘", "s"),
    row("‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾", "d"),
];

pub const CHEST: &[SpriteRow] = &[
    row("    °    o", "g"),
    row("  o    °    o", "g"),
    row("", "n"),
    row("    _______________________", "n"),
    row(
        "  / $  $  $  $  $  $  $  $  \\",
        "ggngggggggggggggggggggggggggn",
    ),
    row("  |=========================|", "n"),
    row("  |                         |", "n"),
    row("  |           [°]           |", "nnnnnnnnnnnnnngggn"),
    row("  |_________________________|", "n"),
    row(".:.:.:.:.:.:.:.:.:.:.:.:.:.:.:.", "k"),
];

pub const MIMIC: &[SpriteRow] = &[
    row("    _______________________", "n"),
    row("  /VVVVVVVVVVVVVVVVVVVVVVVVV\\", "w"),
    row("  \\    °             °      /", "R"),
    row("   ^^^^^^^^^^^^^^^^^^^^^^^^^", "w"),
    row("  |           [°]           |", "nnnnnnnnnnnnnnRRRn"),
    row("  |_________________________|", "n"),
    row(".:.:.:.:.:.:.:.:.:.:.:.:.:.:.:.", "k"),
];

pub const LURKER_JAR: &[SpriteRow] = &[
    row("          _", "s"),
    row("      .-~\"~-.", "c"),
    row("    /  ,  ,  \\", "c"),
    row("   |  /  \\    |", "c"),
    row("   | .-\"-.  \\ |", "c"),
    row("   |/°   °\\   |", "cccccRcccRc"),
    row("   |\\ vvv /   |", "ccccccwwwc"),
    row("   '-.....-'", "c"),
    row("  [=============]", "s"),
    row("", "c"),
    row(".  ,   .   , .", "c"),
];

pub const BELL_JAR: &[SpriteRow] = &[
    row("       _", "s"),
    row("   .-~\"~-.", "c"),
    row(" /         \\", "c"),
    row("|           |", "c"),
    row("|   <°)><   |", "ccccdddddc"),
    row("|           |", "c"),
    row("'-.........-'", "c"),
    row("[=============]", "s"),
    row("", "c"),
    row("  .  ,   .   , .", "c"),
];
