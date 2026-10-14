use super::bestiary::{BLANK, SpriteRow, row};
use super::map::RoomKind;
use super::room::{Scene, Stage};

pub fn of(kind: RoomKind, stage: &Stage) -> Scene {
    let rows = match (kind, stage) {
        (RoomKind::Chest, Stage::Sprung) => MIMIC,
        (RoomKind::Chest, Stage::Spoils) => OPEN_CHEST,
        (RoomKind::Chest, _) => CHEST,
        (RoomKind::BellJar, Stage::Sprung) => LURKER_JAR,
        (RoomKind::BellJar, Stage::Spoils) => BROKEN_JAR,
        (RoomKind::BellJar, _) => BELL_JAR,
        (RoomKind::Den, _) => DEN,
        (RoomKind::School, _) => SCHOOL,
        (RoomKind::Shrine, _) => SHRINE,
        (RoomKind::Altar, _) => ALTAR,
        (RoomKind::Laboratory, _) => LABORATORY,
        (RoomKind::Vent, _) => VENT,
        (RoomKind::Cleaning, _) => CLEANING,
        (RoomKind::Wreck, _) => WRECK,
        (RoomKind::Hole, _) => HOLE,
        (RoomKind::Elevator, _) => ELEVATOR,
        (RoomKind::Encounter | RoomKind::Boss | RoomKind::Event | RoomKind::Vending, _) => &[],
    };
    Scene::Rows(rows)
}

pub const ALL: &[&[SpriteRow]] = &[
    DEN,
    SCHOOL,
    SHRINE,
    ALTAR,
    LABORATORY,
    VENT,
    CLEANING,
    WRECK,
    ELEVATOR,
    ELEVATOR_LOW,
    ELEVATOR_SMALL,
    CHEST,
    OPEN_CHEST,
    MIMIC,
    LURKER_JAR,
    BELL_JAR,
    BROKEN_JAR,
    HOLE,
];

pub const LIFTS: &[&[SpriteRow]] = &[ELEVATOR, ELEVATOR_LOW, ELEVATOR_SMALL];
pub const LIFT_FROM: (usize, usize) = (1, 11);
pub const LIFT_ARROW: (usize, usize) = (1, 15);
pub const LIFT_TO: (usize, usize) = (1, 19);

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
    row("       \\    |    /", "y"),
    row("            +", "y"),
    row("           /|\\", "y"),
    row("         _/ | \\_", "y"),
    row("       _/       \\_", "y"),
    row("     _/___________\\_", "y"),
    row("      |  .-‾‾‾-.  |", "eeeeeeeeeyyyyyyye"),
    row("      | (  ‿‿‿  ) |", "eeeeeeeeyyyyyyyyye"),
    row("      | ><)))))°> |", "eeeeeeeewwwwwwwwwe"),
    row("      |  _______  |", "e"),
    row(" i    |_|_______|_|    i", "eoeeeeeeeeeeeeeeeeeeeeeo"),
    row(" |   [=============]   |", "e"),
    row("_|_                   _|_", "e"),
];

pub const ALTAR: &[SpriteRow] = &[
    row("     )                       (", "o"),
    row(
        "    (_)     .---------.     (_)",
        "ssssdddssssseeeeeeeeeeesssssd",
    ),
    row(
        "    |#|     | ~~~~~~~ |     |#|",
        "ssssdddssssseeeeeeeeeeesssssd",
    ),
    row(
        "    |#|     | ~~~~~~~ |     |#|",
        "ssssdddssssseeeeeeeeeeesssssd",
    ),
    row(
        "    |#|     | x______ |     |#|",
        "ssssdddssssseeeeeeeeeeesssssd",
    ),
    row(
        " ___|#|_____'---------'_____|#|___",
        "ssssdddssssseeeeeeeeeeesssssddds",
    ),
    row("|                                 |", "s"),
    row(
        "|           .-‾‾‾‾‾‾‾-.           |",
        "ssssssssssssRRRRRRRRRRRs",
    ),
    row(
        "|          / \\       / \\          |",
        "sssssssssssRsRsssssssRsRs",
    ),
    row(
        "|         | ‾‾‾\\   /‾‾‾ |         |",
        "ssssssssssRsRRRRsssRRRRsRs",
    ),
    row(
        "|         |  \\  \\ /  /  |         |",
        "ssssssssssRssRssRsRssRssRs",
    ),
    row(
        "|          \\  \\_/ \\_/  /          |",
        "sssssssssssRssRRRsRRRssRs",
    ),
    row(
        "|___________'-._____.-'___________|",
        "ssssssssssssRRRRRRRRRRRs",
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
    row("<=o=>         ·           ·", "oooooG"),
];

pub const VENT: &[SpriteRow] = &[
    row("              o   °", "c"),
    row("            °   o   °", "c"),
    row("", ""),
    row("               ~~~", "o"),
    row("             /     \\", "n"),
    row("            /   ▒   \\", "nnnnnnnnnnnnnnnnon"),
    row("           /_________\\", "n"),
    row("", ""),
    row("", ""),
    row(
        "     ><)))))°>     <°(((((><",
        "rrrrrrrrrrrrrrrrrrrooooooooo",
    ),
];

pub const CLEANING: &[SpriteRow] = &[
    row("       ^ ^ ^ ^", "e"),
    row("     ><)))))))°>", "r"),
    row("       v v v v", "e"),
    row("", "d"),
    row("", "d"),
    row("    ,,=<     >=,,", "R"),
    row("", "d"),
    row(" .-~~~~~~~~~~~~~~~-.", "d"),
    row("(                   )", "d"),
];

pub const WRECK: &[SpriteRow] = &[
    row("            |\\", "d"),
    row("            | \\", "d"),
    row("    ________|__\\_____", "n"),
    row("     \\  o   o   o  /", "n"),
    row("      \\___________/", "n"),
    row("", "d"),
    row("   <°((x><     <°((x><", "d"),
    row("", "d"),
    row(".:.:.:.:.:.:.:.:.:.:.:.:.", "k"),
];

pub const ELEVATOR: &[SpriteRow] = &[
    row("┌─────────────────────────────┐", "s"),
    row("│          4   ▼   5          │", "s"),
    row("├─────────────────────────────┤", "s"),
    row("│              ║              │", "s"),
    row("│              ║              │", "s"),
    row("│              ║              │", "s"),
    row("│              ║              │", "s"),
    row("│              ║              │   ┌───┐", "s"),
    row("│              ║              │   │ ▲ │", "s"),
    row("│              ║              │   │ ▼ │", "s"),
    row("│              ║              │   └───┘", "s"),
    row("│              ║              │", "s"),
    row("│              ║              │", "s"),
    row("│              ║              │", "s"),
    row("│              ║              │", "s"),
    row("└─────────────────────────────┘", "s"),
];

pub const ELEVATOR_LOW: &[SpriteRow] = &[
    row("┌─────────────────────────────┐", "s"),
    row("│          4   ▼   5          │", "s"),
    row("├─────────────────────────────┤", "s"),
    row("│              ║              │", "s"),
    row("│              ║              │   ┌───┐", "s"),
    row("│              ║              │   │ ▲ │", "s"),
    row("│              ║              │   │ ▼ │", "s"),
    row("│              ║              │   └───┘", "s"),
    row("│              ║              │", "s"),
    row("└─────────────────────────────┘", "s"),
];

pub const ELEVATOR_SMALL: &[SpriteRow] = &[
    row("┌─────────────────────────────┐", "s"),
    row("│          4   ▼   5          │", "s"),
    row("├─────────────────────────────┤", "s"),
    row("│              ║              │   ┌───┐", "s"),
    row("│              ║              │   │ ▲ │", "s"),
    row("│              ║              │   │ ▼ │", "s"),
    row("│              ║              │   └───┘", "s"),
    row("└─────────────────────────────┘", "s"),
];

pub const CHEST: &[SpriteRow] = &[
    row("             °   o", "g"),
    row("          o    °    o", "g"),
    row("", "n"),
    row("    _______________________", "n"),
    row(
        "  /   $  $  $  $  $  $  $   \\",
        "nnnnnngnngnngnngnngnngnngn",
    ),
    row("  |=========================|", "n"),
    row("  |                         |", "n"),
    row("  |           [°]           |", "nnnnnnnnnnnnnngggn"),
    row("  |_________________________|", "n"),
    row(".:.:.:.:.:.:.:.:.:.:.:.:.:.:.:.", "k"),
];

pub const OPEN_CHEST: &[SpriteRow] = &[
    row("    _______________________", "n"),
    row("   |                       |", "n"),
    row("   |_______________________|", "n"),
    row(
        "  / $ $ $ $ $ $ $ $ $ $ $ $ \\",
        "nnnngngngngngngngngngngngngn",
    ),
    row("  |=========================|", "n"),
    row("  |           [°]           |", "nnnnnnnnnnnnnngggn"),
    row("  |_________________________|", "n"),
    row(".:.:.:.:.:.:.:.:.:.:.:.:.:.:.:.", "k"),
];

pub const MIMIC: &[SpriteRow] = &[
    row("    _______________________", "n"),
    row("  /VVVVVVVVVVVVVVVVVVVVVVVVV\\", "w"),
    row("  \\     °             °     /", "wwwwwwwwRwwwwwwwwwwwwwRw"),
    row("   ^^^^^^^^^^^^^^^^^^^^^^^^^", "w"),
    row("  |           [°]           |", "nnnnnnnnnnnnnnRRRn"),
    row("  |_________________________|", "n"),
    row(".:.:.:.:.:.:.:.:.:.:.:.:.:.:.:.", "k"),
];

pub const BELL_JAR: &[SpriteRow] = &[
    row("      _", "s"),
    row("   .-~\"~-.", "c"),
    row(" /         \\", "c"),
    row("|           |", "c"),
    row("|   <°)><   |", "ccccdddddc"),
    row("|           |", "c"),
    row("'-.........-'", "c"),
    row("[===========]", "s"),
    row("", "c"),
    row(" .  ,   ,  .", "k"),
];

pub const LURKER_JAR: &[SpriteRow] = &[
    row("      _", "s"),
    row("   .-~\"~-.", "c"),
    row(" /  .-\"-.  \\", "ccccwwwwwc"),
    row("|  /°   °\\  |", "cccwRwwwRwc"),
    row("|  \\ vvv /  |", "cccwwwwwwwc"),
    row("|   '---'   |", "ccccwwwwwc"),
    row("'-.........-'", "c"),
    row("[===========]", "s"),
    row("", "c"),
    row(" .  ,   ,  .", "k"),
];

pub const BROKEN_JAR: &[SpriteRow] = &[
    row("|\\/\\_   _/\\/|", "c"),
    row("|           |", "c"),
    row("|           |", "c"),
    row("'-.........-'", "c"),
    row("[===========]", "s"),
    row("", "c"),
    row(" .  ,   ,  .", "k"),
];

pub const HOLE: &[SpriteRow] = &[
    row("             |", "s"),
    row("             |", "s"),
    row("             |", "s"),
    row("             |", "s"),
    row("             |", "s"),
    row("             |", "s"),
    row("             |", "s"),
    row("             J", "w"),
    BLANK,
    row(".-~~~~~~~~~~~~~~~~~~~~~~~-.", "k"),
    row("( ░▒▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▒░ )", "kkbbbbbbbbbbbbbbbbbbbbbbbk"),
    row("(  ░▒▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▒░  )", "kkkbbbbbbbbbbbbbbbbbbbbbk"),
    row("'-.......................-'", "k"),
];
