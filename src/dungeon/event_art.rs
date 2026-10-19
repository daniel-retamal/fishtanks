use super::bestiary::{BLANK, SpriteRow, row};

pub const BOTTLE: &[SpriteRow] = &[
    row("            _", "e"),
    row("           |~|", "e"),
    row("           | |", "c"),
    row("          /   \\", "c"),
    row("         |  ≡  |", "ccce"),
    row("         |_____|", "c"),
    BLANK,
    row("    ~  ~~   ~   ~~  ~", "B"),
];

pub const PIRATE_WRECK: &[SpriteRow] = &[
    row("               |\\", "d"),
    row("               |=\\", "ddddddddddddddddr"),
    row("               |  \\", "d"),
    row("       ________|___\\____", "n"),
    row("       \\   o   o   o   /", "n"),
    row("        \\_____________/", "n"),
    BLANK,
    row("    .:.:.:.:.:.:.:.:.:.:.:.", "k"),
];

pub const PINCOYA: &[SpriteRow] = &[
    row("            ,~~,", "y"),
    row("           ( °° )", "yeey"),
    row("          --|  |--", "e"),
    row("            )~~(", "B"),
    row("           /    \\", "B"),
    row("          ><    ><", "B"),
    BLANK,
    row("     ~  ~~   ~~   ~~  ~", "B"),
];

pub const CORAL_MAZE: &[SpriteRow] = &[
    row(" Y   Y   Y   Y   Y", "p"),
    row(" |___|   |___|   |", "p"),
    row(" |   |___|   |___|", "p"),
    row(" |___|   |___|   |", "p"),
    row(" |   |   |   |   |", "p"),
    BLANK,
    row(".:.:.:.:.:.:.:.:.:.", "k"),
];

pub const GIANT_PEARL: &[SpriteRow] = &[
    row("   .-‾‾‾‾‾‾‾‾‾-.", "e"),
    row("  /             \\", "e"),
    row(" |      .-.      |", "eeeeeeeewwwe"),
    row(" |     (   )     |", "eeeeeeeweeewe"),
    row(" |      '-'      |", "eeeeeeeewwwe"),
    row("  \\_____________/", "e"),
    row("  /VVVVVVVVVVVVV\\", "e"),
    BLANK,
    row(".:.:.:.:.:.:.:.:.:.", "k"),
];

pub const ANEMONES: &[SpriteRow] = &[
    row("\\|/     \\|/     \\|/", "m"),
    row("(°)     (°)     (°)", "mymmmmmmmymmmmmmmym"),
    row(" |  \\|/  |  \\|/  |", "ppppmpmpppppmpmp"),
    row(" |  (°)  |  (°)  |", "ppppmympppppmymp"),
    row(" |   |   |   |   |", "p"),
    BLANK,
    row(".:.:.:.:.:.:.:.:.:.", "k"),
];

pub const CHOCOLATE_RIVER: &[SpriteRow] = &[
    row(".:.:.:.:.:.:.:.:.:.:.", "k"),
    BLANK,
    row("~~~~~~~~~~~~~~~~~~~~~", "n"),
    row(" ≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈", "n"),
    row("~~~~~~~~~~~~~~~~~~~~~", "n"),
    BLANK,
    row(".:.:.:.:.:.:.:.:.:.:.", "k"),
];

pub const GINGERBREAD_HOUSE: &[SpriteRow] = &[
    row("      /\\", "n"),
    row("     /  \\", "n"),
    row("    /‾‾‾‾\\", "nnnnnwwwwn"),
    row("   /~~~~~~\\", "nnnnppppppn"),
    row("   |[]  []|", "nnnnyynnyyn"),
    row("   |  __  |", "n"),
    row("   |_|  |_|", "n"),
    BLANK,
    row(".:.:.:.:.:.:.:", "k"),
];

pub const SUGAR_ALTAR: &[SpriteRow] = &[
    row("   °   °   °", "w"),
    row("     .-‾-.", "p"),
    row("    ( ¤ ¤ )", "ppppppypyp"),
    row("  ___|___|___", "w"),
    row(" |  ~ ~ ~ ~  |", "wwwwpwpwpwpw"),
    row(" |___________|", "w"),
    BLANK,
    row(".:.:.:.:.:.:.:.", "k"),
];

pub const SEANCE_TABLE: &[SpriteRow] = &[
    row("         i      i      i", "eeeeeeeeeyeeeeeeyeeeeeey"),
    row("         |      |      |", "e"),
    BLANK,
    row(".-------------------------------.", "n"),
    row(
        "|   A B C D E F G H I J K L M   |",
        "nnnnenenenenenenenenenenenenen",
    ),
    row(
        "|   N O P Q R S T U V W X Y Z   |",
        "nnnnenenenenenenenenenenenenen",
    ),
    row(
        "|   YES       <°)><       NO    |",
        "nnnneeennnnnnnwwwwwnnnnnnneen",
    ),
    row("'-------------------------------'", "n"),
];

pub const FRESH_GRAVE: &[SpriteRow] = &[
    row("    _______", "s"),
    row("   /       \\", "s"),
    row("  |   RIP   |", "s"),
    row("  |         |", "s"),
    row("  |   ~~~   |", "s"),
    row(" _|_________|_", "s"),
    row("/  .  .  .  . \\", "n"),
    BLANK,
    row(".:.:.:.:.:.:.:.", "k"),
];

pub const COFFIN: &[SpriteRow] = &[
    row("   _______", "n"),
    row("  /       \\", "n"),
    row(" /    +    \\", "nnnnnnyn"),
    row("|     |     |", "n"),
    row("|           |", "n"),
    row(" \\         /", "n"),
    row("  \\_______/", "n"),
    BLANK,
    row(".:.:.:.:.:.:.", "k"),
];

pub const CROP_CIRCLE: &[SpriteRow] = &[
    row(". ' . ' . ' . ' . ' . '", "G"),
    row("    .-‾‾‾‾‾‾‾‾‾‾‾-.", "y"),
    row("  ,'    .-‾‾‾-.    ',", "y"),
    row(" (     (   °   )     )", "yyyyyyyyyyywy"),
    row("  ',    '-___-'    ,'", "y"),
    row("    '-._________.-'", "y"),
    row(". ' . ' . ' . ' . ' . '", "G"),
];

pub const OASIS: &[SpriteRow] = &[
    row("   __  _  __", "G"),
    row("  /  \\ | /  \\", "GGGGGGGnG"),
    row("       |", "n"),
    row("       |", "n"),
    row("  ~~~~~|~~~~~", "BBBBBBBnB"),
    row(" (≈≈≈≈≈≈≈≈≈≈≈)", "ByBBBBBBBBBBBy"),
    BLANK,
    row(".:.:.:.:.:.:.:.", "y"),
];

pub const CRASHED_SAUCER: &[SpriteRow] = &[
    row("       .-‾-.", "c"),
    row("    __/_____\\__", "sssssscssssscs"),
    row("   (__°__°__°__)", "ssssssGssGssGs"),
    row("      /  |  \\", "s"),
    BLANK,
    row(".:.:.:.:.:.:.:.:.:.", "y"),
];

pub const GLOWING_POOL: &[SpriteRow] = &[
    row("   °   .   °", "G"),
    row("  .-~~~~~~~-.", "G"),
    row(" (  ~  ~  ~  )", "G"),
    row(" (~  ~   ~  ~)", "G"),
    row("  '-.......-'", "G"),
    BLANK,
    row(".:.:.:.:.:.:.:.", "k"),
];

pub const LAB_DOOR: &[SpriteRow] = &[
    row(" _____________", "w"),
    row("|  _________  |", "w"),
    row("| |   /!\\   | |", "wwwwwwyyywwwwww"),
    row("| |_________| |", "w"),
    row("|      o      |", "wwwwwwwRwwwwwww"),
    row("|_____________|", "w"),
];

pub const SPECIMEN_TANK: &[SpriteRow] = &[
    row(".-----------.", "c"),
    row("|           |", "c"),
    row("|  ><(((°>  |", "cccGGGGGGGccc"),
    row("|   °   °   |", "c"),
    row("'-----------'", "c"),
    row("  |       |", "d"),
];

pub const EMAIL: &[SpriteRow] = &[
    row(" ___________", "w"),
    row("|\\         /|", "w"),
    row("| \\       / |", "w"),
    row("|  \\_____/  |", "w"),
    row("|___________|", "w"),
    BLANK,
    row("      @", "y"),
];

pub const CAPTCHA: &[SpriteRow] = &[
    row("+---+---+---+", "w"),
    row("|><>|   |><>|", "wBBBwwwwwBBBw"),
    row("+---+---+---+", "w"),
    row("|   |><>|   |", "wwwwwBBBwwwww"),
    row("+---+---+---+", "w"),
    row("|><>|   |   |", "wBBBwwwwwwwww"),
    row("+---+---+---+", "w"),
];

pub const TERMS: &[SpriteRow] = &[
    row(" ___________", "e"),
    row("/ ========= \\", "eedddddddddee"),
    row("| ========= |", "eedddddddddee"),
    row("| ========= |", "eedddddddddee"),
    row("| ===  [OK] |", "eedddeeGGGGee"),
    row("\\___________/", "e"),
];

pub const CONTRACT: &[SpriteRow] = &[
    row("  _________", "e"),
    row(" / ~~~~~~~ \\", "eeedddddddee"),
    row("|  ~~~~~~~  |", "eeedddddddeee"),
    row("|  ~~~~~    |", "eeedddddeeeee"),
    row("|      ~~@  |", "eeeeeeeddReee"),
    row(" \\_________/", "e"),
];

pub const FERRY: &[SpriteRow] = &[
    row("      ,-.", "d"),
    row("     ( ° )", "dddddddRdd"),
    row("     /| |\\", "d"),
    row(" _____|_|_____", "nnnnnndddnnnnn"),
    row(" \\___________/", "n"),
    row("~^~~^~~^~~^~~^~", "R"),
];

pub const CAULDRON: &[SpriteRow] = &[
    row("  ~  ><>  ~", "GGGGGBBBGGG"),
    row(".-=========-.", "k"),
    row("|           |", "k"),
    row(" \\_________/", "k"),
    row(" /\\ /\\ /\\ /\\", "R"),
];

pub const GATEKEEPER: &[SpriteRow] = &[
    row("   _______", "y"),
    row("  /  (o)  \\", "yyyyyyByyyy"),
    row(" |  |   |  |", "y"),
    row(" |  |   |  |", "y"),
    row(" |  |   |  |", "y"),
    row("_|__|___|__|_", "y"),
];

pub const SCALES: &[SpriteRow] = &[
    row("       o", "y"),
    row("       |", "y"),
    row(".------+------.", "y"),
    row("|      |      |", "y"),
    row("\\~/    |    \\_/", "ywyyyyyyyyyyyyy"),
    row("       |", "y"),
    row("     __|__", "y"),
];

pub const CHOIR: &[SpriteRow] = &[
    row(" o     o     o", "y"),
    row("(^)   (^)   (^)", "w"),
    row("/|\\   /|\\   /|\\", "w"),
    row("/ \\   / \\   / \\", "w"),
    BLANK,
    row("~ ~ ~ ~ ~ ~ ~ ~", "d"),
];

pub const DOOR: &[SpriteRow] = &[
    row("  _______", "n"),
    row(" |  ___  |", "n"),
    row(" | |   | |", "n"),
    row(" | |___| |", "n"),
    row(" |     o |", "nnnnnnnynn"),
    row(" |_______|", "n"),
    row("~~~ ~~~ ~~~", "B"),
];

pub const REFLECTION: &[SpriteRow] = &[
    row("         |", "d"),
    row("><(((°>  |  <°)))><", "BBBBBBBBBdbbbbbbbbb"),
    row("         |", "d"),
    BLANK,
    row("~  ~  ~  ~  ~  ~  ~", "b"),
];

pub const PRAYER: &[SpriteRow] = &[
    row("    .-~~~-.", "v"),
    row("   (   O   )", "vvvvvvvmvvvv"),
    row("    '-~~~-'", "v"),
    BLANK,
    row("~~~~~ ~~~ ~~~~~", "d"),
];
