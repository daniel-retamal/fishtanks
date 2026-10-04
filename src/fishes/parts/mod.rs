pub mod buffer;
pub mod config;
pub mod pins;
pub mod set;
pub mod surface;

pub use buffer::{Buffer, Grid};
pub use config::{ConfigSpec, PartConfig};
pub use pins::{
    Bus, OUTPUT_PIN, PIN_WIDTH_MAX, PIN_WIDTH_MIN, Pin, PinDirection, PinMap, PinOwner,
    PinReadings, PinSpec, Sensitivity,
};
pub use set::PartSet;
pub use surface::{DOT_ROWS, Glyphs, Surface};

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::economy::Rarity;
use crate::tank::{Link, Selector, WorldSignal, WorldView};

const NARROW_BUS_WIDTH: u8 = 8;
pub const RIG_WAIT_MIN_SECS: f32 = 30.0;
pub const RIG_WAIT_MAX_SECS: f32 = 60.0;
const WIDE_BUS_WIDTH: u8 = 16;

const INVERTER_COIL_DESCRIPTION: &str = "The wound against itself. Everything it is told, it denies. Not. Will invert the fish's output";

const DELAY_SPOOL_DESCRIPTION: &str =
    "Time, spooled and sold by the metre. Do not hurry me. Do not hurry me. Adds a delay.";

const SHOAL_COUNTER_DESCRIPTION: &str = "A false eye. It counts ... them. Twelve thousand mornings of watching a room and reporting whether it is full. Counts the tank.";

const TIDAL_CLOCK_DESCRIPTION: &str =
    "It tells you whose turn it is. The night shift is always someone's. Reads the hour.";

const ASSAY_SCALE_DESCRIPTION: &str =
    "Meat is a number pretending to be a shape. It weighs your fish. Reads worth.";

const LEDGER_NERVE_DESCRIPTION: &str = "A strand of an economic paradigm, still twitching. Ownership rendered as a sensation. Reads money.";

const GEIGER_COIL_DESCRIPTION: &str =
    "Measures how quickly the tank is becoming something else. Reads mutation pressure.";

const COMMAND_MODULE_DESCRIPTION: &str = "Obedience boxed. It does not want anything. Give it eight instructions and never think of it again. Runs your program.";

const STARTLE_NERVE_DESCRIPTION: &str = "A reflex with no animal attached. It flinches for you, faster than you can be told, at deaths you did not witness. The interrupt line.";

const COCHLEA_DESCRIPTION: &str = "A lonely ear. It does not understand you and it never will; it only forwards. Every word you have ever typed went somewhere. Buffered input.";

const REFLEX_ARC_DESCRIPTION: &str = "It answers before the message arrives. Live keys.";

const GLYPH_PANEL_DESCRIPTION: &str = "Somewhere there is a room full of this things, keeping afloat a company, a family, a nation, or a thought. Character display.";

const CORE_STACK_DESCRIPTION: &str = "Woven by hand in a room that no longer exists. Every bit is a ring you can hold. It remembers what you were, which is more than what you can do. Addressed memory.";

const CATHODE_ARRAY_DESCRIPTION: &str = "The body agrees to become a surface. Each cell can change on command, forever, for you. The fish becomes a screen.";

const RELAY_MAST_DESCRIPTION: &str = "A tower that agrees with another tower. What is decided in one tank is believed in the next, and no one asks who made the first decision. Bridges tanks.";

const ANGLER_RIG_DESCRIPTION: &str = "It fishes without hunger, the only honest way to fish. The line goes down. Something always comes up. Neither party consents. Automates the fishing.";

const INVERTER_COIL_SPRITE_LINES: &[&str] = &[
    "  -------",
    r" ///---\\\",
    "|| || || ||",
    r" \\\---///",
    "  -------",
];

const DELAY_SPOOL_SPRITE_LINES: &[&str] = &[
    "  ,-----.",
    r" // ,-. \\",
    "(( (   ) ))",
    r" \\ `-' //",
    "  `-----'",
];

const SHOAL_COUNTER_SPRITE_LINES: &[&str] = &[
    r#"  ,-"""-."#,
    r" / .---. \",
    "| ( (o) ) |",
    r" \ '---' /",
    "  `-...-'",
];

const TIDAL_CLOCK_SPRITE_LINES: &[&str] = &[
    r#"  ,-"""-."#,
    r" /   |2  \",
    "| 9  |  3 |",
    r" \   6\  /",
    "  `-...-'",
];

const ASSAY_SCALE_SPRITE_LINES: &[&str] = &[
    r"   __/^\__",
    "  |   |   |",
    " ---  +  ---",
    "      |",
    "    __|__",
];

const LEDGER_NERVE_SPRITE_LINES: &[&str] = &[
    "   $ $  $",
    r"    \| / $",
    "   $ |/ /",
    r"    \| /",
    "     |/",
    "     |",
];

const GEIGER_COIL_SPRITE_LINES: &[&str] = &[
    ",------------,",
    "|,----------,|",
    "||  ,-----,.||",
    "|| '     .' ||",
    "||      /   ||",
    "||    .'    ||",
    "|'----------'|",
    "'------------'",
];

const STARTLE_NERVE_SPRITE_LINES: &[&str] = &[
    r"   \  |  /",
    r"    \_|_/",
    "   --(o)--",
    r"    _/|\_",
    r"   /  |  \",
];

const COMMAND_MODULE_SPRITE_LINES: &[&str] = &[
    "  .________.",
    "  | ______ |",
    "  ||>_    ||",
    "  ||______||",
    "  |________|",
];

const COCHLEA_SPRITE_LINES: &[&str] = &[
    ",------------,",
    "|    _,..._  |",
    "|   .'    '`.|",
    "|,..|. <']| ||",
    r"||.  \.../ ,||",
    "| `._   ,_.' |",
    "|   '''''    |",
    "'------------'",
];

const REFLEX_ARC_SPRITE_LINES: &[&str] = &[
    ",-------------,",
    "|   _-----_   |",
    "| -',.   ,.`. |",
    r"|(   `\.`/   )|",
    "| ..  -' '`' /|",
    "| `._-----_,' |",
    "|        '    |",
    "'-------------'",
];

const GLYPH_PANEL_SPRITE_LINES: &[&str] = &[
    "   ________",
    "  |__====__|",
    "  |[ABCDEF]|",
    "  | ______ |",
    r"  |/______\|",
];

const CORE_STACK_SPRITE_LINES: &[&str] = &[
    ",-----------,",
    "|,---------,|",
    "||#-#-#-#-#||",
    "|||o|o|o|o|||",
    "||#-#-#-#-#||",
    "|||o|o|o|o|||",
    "||#-#-#-#-#||",
    "|'---------'|",
    "'-----------'",
];

const CATHODE_ARRAY_SPRITE_LINES: &[&str] = &[
    "  +-+-+-+-+",
    "  |#| |#| |",
    "  +-+-+-+-+",
    "  | |#| |#|",
    "  +-+-+-+-+",
];

const RELAY_MAST_SPRITE_LINES: &[&str] = &[
    "    .", "    |", r"   /|\", "  |-.-|", "  '-:-'", "   [|]", "   [|]", "   [|]",
];

const ANGLER_RIG_SPRITE_LINES: &[&str] = &[
    "      .",
    "     /|",
    "    / |",
    "   /__|__",
    r" \--------/",
    "  `      `",
];

const TARGET_CONFIG: ConfigSpec = ConfigSpec {
    name: "target",
    default: "",
};

const ASSAY_TARGET_CONFIG: ConfigSpec = ConfigSpec {
    name: "target",
    default: "heaviest",
};

const MEASURE_CONFIG: ConfigSpec = ConfigSpec {
    name: "measure",
    default: WORTH_MEASURE,
};

const SHOAL_THRESHOLD_CONFIG: ConfigSpec = ConfigSpec {
    name: "threshold",
    default: "10",
};

const ASSAY_THRESHOLD_CONFIG: ConfigSpec = ConfigSpec {
    name: "threshold",
    default: "1000",
};

const LEDGER_THRESHOLD_CONFIG: ConfigSpec = ConfigSpec {
    name: "threshold",
    default: "1000",
};

const GEIGER_THRESHOLD_CONFIG: ConfigSpec = ConfigSpec {
    name: "threshold",
    default: "1",
};

const PANEL_WIDTH_CONFIG: ConfigSpec = ConfigSpec {
    name: "width",
    default: "16",
};

const PANEL_HEIGHT_CONFIG: ConfigSpec = ConfigSpec {
    name: "height",
    default: "2",
};

const SURFACE_WIDTH_CONFIG: ConfigSpec = ConfigSpec {
    name: "width",
    default: "64",
};

const GLYPHS_CONFIG: ConfigSpec = ConfigSpec {
    name: "glyphs",
    default: "braille",
};

const FAR_TANK_CONFIG: ConfigSpec = ConfigSpec {
    name: "tank",
    default: "",
};

const FAR_CHANNEL_CONFIG: ConfigSpec = ConfigSpec {
    name: "channel",
    default: "",
};

const BAIT_THRESHOLD_CONFIG: ConfigSpec = ConfigSpec {
    name: "threshold",
    default: "1",
};

const DISPLAY_RAM_CELLS: usize = 80;
const DISPLAY_SIDE_MIN: usize = 1;
const PRINTABLE: std::ops::RangeInclusive<u8> = b' '..=b'~';
const UNDRAWABLE: char = '\u{FFFD}';

const WORTH_MEASURE: &str = "value";
const WEIGHT_MEASURE: &str = "weight";

const OVER_PIN: PinSpec = PinSpec::flag("over");
const UNDER_PIN: PinSpec = PinSpec::flag("under");

const COUNT_PIN: PinSpec = PinSpec::out("count", NARROW_BUS_WIDTH);
const FULL_PIN: PinSpec = PinSpec::flag("full");
const EMPTY_PIN: PinSpec = PinSpec::flag("empty");

const TIME_PIN: PinSpec = PinSpec::out("time", NARROW_BUS_WIDTH);
const NIGHT_PIN: PinSpec = PinSpec::flag("night");
const DAWN_PIN: PinSpec = PinSpec::flag("dawn");

const WEIGHT_PIN: PinSpec = PinSpec::out("weight", WIDE_BUS_WIDTH);
const VALUE_PIN: PinSpec = PinSpec::out("value", WIDE_BUS_WIDTH);

const CASH_PIN: PinSpec = PinSpec::out("cash", WIDE_BUS_WIDTH);
const TANK_VALUE_PIN: PinSpec = PinSpec::out("tank_value", WIDE_BUS_WIDTH);
const BROKE_PIN: PinSpec = PinSpec::flag("broke");

const RADS_PIN: PinSpec = PinSpec::out("rads", NARROW_BUS_WIDTH);
const HOT_PIN: PinSpec = PinSpec::flag("hot");
const MUTATED_PIN: PinSpec = PinSpec::flag("mutated");

const DEATH_PIN: PinSpec = PinSpec::flag("death");
const BIRTH_PIN: PinSpec = PinSpec::flag("birth");
const SALE_PIN: PinSpec = PinSpec::flag("sale");
const CATCH_PIN: PinSpec = PinSpec::flag("catch");
const ABDUCTION_PIN: PinSpec = PinSpec::flag("abduction");

const FIRE_PIN: PinSpec = PinSpec::strobe("fire");

const EAR_CHAR_PIN: PinSpec = PinSpec::out("char", NARROW_BUS_WIDTH);
const STROBE_PIN: PinSpec = PinSpec::strobe("strobe");
const READY_PIN: PinSpec = PinSpec::flag("ready");
const DONE_PIN: PinSpec = PinSpec::flag("done");

const PANEL_CHAR_PIN: PinSpec = PinSpec::input("char", NARROW_BUS_WIDTH);
const WRITE_PIN: PinSpec = PinSpec::strobe("write");
const CLEAR_PIN: PinSpec = PinSpec::strobe("clear");

const ADDR_PIN: PinSpec = PinSpec::input("addr", NARROW_BUS_WIDTH);
const DATA_IN_PIN: PinSpec = PinSpec::input("data_in", NARROW_BUS_WIDTH);
const DATA_OUT_PIN: PinSpec = PinSpec::out("data_out", NARROW_BUS_WIDTH);

const BIT_PIN: PinSpec = PinSpec::input("bit", PIN_WIDTH_MIN);
const BYTE_PIN: PinSpec = PinSpec::input("byte", NARROW_BUS_WIDTH);
const WRITE_BYTE_PIN: PinSpec = PinSpec::strobe("write_byte");

const MAST_IN_PIN: PinSpec = PinSpec::input("in", PIN_WIDTH_MIN);
const MAST_OUT_PIN: PinSpec = PinSpec::flag("out");

const CAST_PIN: PinSpec = PinSpec::strobe("cast");
const CAUGHT_PIN: PinSpec = PinSpec::flag("caught");
const BAIT_LOW_PIN: PinSpec = PinSpec::flag("bait_low");

const REFLEX_ARC_KEYS: [KeyLine; 8] = [
    KeyLine::new("up", "up key", "up"),
    KeyLine::new("down", "down key", "down"),
    KeyLine::new("left", "left key", "left"),
    KeyLine::new("right", "right key", "right"),
    KeyLine::new("fire", "fire key", "space"),
    KeyLine::new("alt", "alt key", "z"),
    KeyLine::new("start", "start key", "enter"),
    KeyLine::new("select", "select key", "tab"),
];
static REFLEX_ARC_PINS: [PinSpec; REFLEX_ARC_KEYS.len()] = KeyLine::pins(REFLEX_ARC_KEYS);
static REFLEX_ARC_CONFIG: [ConfigSpec; REFLEX_ARC_KEYS.len()] = KeyLine::config(REFLEX_ARC_KEYS);

const SHOAL_COUNTER_PINS: &[PinSpec] = &[COUNT_PIN, FULL_PIN, EMPTY_PIN, OVER_PIN, UNDER_PIN];
const TIDAL_CLOCK_PINS: &[PinSpec] = &[TIME_PIN, NIGHT_PIN, DAWN_PIN];
const ASSAY_SCALE_PINS: &[PinSpec] = &[WEIGHT_PIN, VALUE_PIN, OVER_PIN, UNDER_PIN, FULL_PIN];
const LEDGER_NERVE_PINS: &[PinSpec] = &[CASH_PIN, TANK_VALUE_PIN, OVER_PIN, BROKE_PIN];
const GEIGER_COIL_PINS: &[PinSpec] = &[RADS_PIN, HOT_PIN, MUTATED_PIN];
const STARTLE_NERVE_PINS: &[PinSpec] = &[DEATH_PIN, BIRTH_PIN, SALE_PIN, CATCH_PIN, ABDUCTION_PIN];
const COMMAND_MODULE_PINS: &[PinSpec] = &[FIRE_PIN];
const COCHLEA_PINS: &[PinSpec] = &[EAR_CHAR_PIN, STROBE_PIN, READY_PIN, DONE_PIN];
const GLYPH_PANEL_PINS: &[PinSpec] = &[PANEL_CHAR_PIN, WRITE_PIN, CLEAR_PIN];
const CORE_STACK_PINS: &[PinSpec] = &[ADDR_PIN, DATA_IN_PIN, DATA_OUT_PIN, WRITE_PIN];
const CATHODE_ARRAY_PINS: &[PinSpec] = &[ADDR_PIN, BIT_PIN, WRITE_PIN, BYTE_PIN, WRITE_BYTE_PIN];
const RELAY_MAST_PINS: &[PinSpec] = &[MAST_IN_PIN, MAST_OUT_PIN];
const ANGLER_RIG_PINS: &[PinSpec] = &[CAST_PIN, CAUGHT_PIN, BAIT_LOW_PIN];

const SHOAL_COUNTER_CONFIG: &[ConfigSpec] = &[TARGET_CONFIG, SHOAL_THRESHOLD_CONFIG];
const ASSAY_SCALE_CONFIG: &[ConfigSpec] =
    &[ASSAY_TARGET_CONFIG, MEASURE_CONFIG, ASSAY_THRESHOLD_CONFIG];
const LEDGER_NERVE_CONFIG: &[ConfigSpec] = &[LEDGER_THRESHOLD_CONFIG];
const GEIGER_COIL_CONFIG: &[ConfigSpec] = &[GEIGER_THRESHOLD_CONFIG];
const GLYPH_PANEL_CONFIG: &[ConfigSpec] = &[PANEL_WIDTH_CONFIG, PANEL_HEIGHT_CONFIG];
const CATHODE_ARRAY_CONFIG: &[ConfigSpec] = &[SURFACE_WIDTH_CONFIG, GLYPHS_CONFIG];
const RELAY_MAST_CONFIG: &[ConfigSpec] = &[FAR_TANK_CONFIG, FAR_CHANNEL_CONFIG];
const ANGLER_RIG_CONFIG: &[ConfigSpec] = &[BAIT_THRESHOLD_CONFIG];
const NO_CONFIG: &[ConfigSpec] = &[];
const NO_PINS: &[PinSpec] = &[];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PartTier {
    Fabric,
    Senses,
    Hands,
    Peripherals,
    Materials,
}

impl PartTier {
    pub const ALL: &'static [PartTier] = &[
        PartTier::Fabric,
        PartTier::Senses,
        PartTier::Hands,
        PartTier::Peripherals,
        PartTier::Materials,
    ];

    pub fn display_name(self) -> &'static str {
        match self {
            PartTier::Fabric => "Fabric",
            PartTier::Senses => "Senses",
            PartTier::Hands => "Hands",
            PartTier::Peripherals => "Peripherals",
            PartTier::Materials => "Materials",
        }
    }

    pub fn index(self) -> usize {
        PartTier::ALL
            .iter()
            .position(|&tier| tier == self)
            .unwrap_or(0)
    }

    pub fn rarity(self) -> Option<Rarity> {
        match self {
            PartTier::Fabric => Some(Rarity::Common),
            PartTier::Senses | PartTier::Hands => Some(Rarity::Rare),
            PartTier::Peripherals => Some(Rarity::Legendary),
            PartTier::Materials => None,
        }
    }
}

#[derive(Clone, Copy)]
struct KeyLine {
    pin: PinSpec,
    key: ConfigSpec,
}

impl KeyLine {
    const fn new(pin: &'static str, field: &'static str, key: &'static str) -> Self {
        Self {
            pin: PinSpec::flag(pin),
            key: ConfigSpec {
                name: field,
                default: key,
            },
        }
    }

    const fn pins<const N: usize>(lines: [KeyLine; N]) -> [PinSpec; N] {
        let mut pins = [PinSpec::flag(""); N];
        let mut index = 0;
        while index < N {
            pins[index] = lines[index].pin;
            index += 1;
        }
        pins
    }

    const fn config<const N: usize>(lines: [KeyLine; N]) -> [ConfigSpec; N] {
        let mut config = [ConfigSpec {
            name: "",
            default: "",
        }; N];
        let mut index = 0;
        while index < N {
            config[index] = lines[index].key;
            index += 1;
        }
        config
    }

    fn binds(&self, config: &PartConfig, key: &str) -> bool {
        config.text(&self.key).to_lowercase() == key
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KeyBinding {
    pub key: String,
    pub pin: &'static str,
}

pub struct PartSpec {
    pub token: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub tier: PartTier,
    pub stacks: bool,
    pub sprite_lines: &'static [&'static str],
    pub pins: &'static [PinSpec],
    pub config: &'static [ConfigSpec],
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum Part {
    InverterCoil,
    DelaySpool,
    ShoalCounter,
    TidalClock,
    AssayScale,
    LedgerNerve,
    GeigerCoil,
    StartleNerve,
    CommandModule,
    Cochlea,
    GlyphPanel,
    ReflexArc,
    CoreStack,
    CathodeArray,
    RelayMast,
    AnglerRig,
}

pub enum PartEffect {
    RunScript,
    Cast,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Display {
    Bubble(Vec<String>),
    Body(Vec<String>),
}

impl Display {
    pub fn rows(&self) -> &[String] {
        match self {
            Display::Bubble(rows) | Display::Body(rows) => rows,
        }
    }
}

impl Part {
    pub const ALL: &'static [Part] = &[
        Part::InverterCoil,
        Part::DelaySpool,
        Part::ShoalCounter,
        Part::TidalClock,
        Part::AssayScale,
        Part::LedgerNerve,
        Part::GeigerCoil,
        Part::StartleNerve,
        Part::CommandModule,
        Part::Cochlea,
        Part::GlyphPanel,
        Part::ReflexArc,
        Part::CoreStack,
        Part::CathodeArray,
        Part::RelayMast,
        Part::AnglerRig,
    ];

    pub fn spec(self) -> PartSpec {
        match self {
            Part::InverterCoil => PartSpec {
                token: "invertercoil",
                display_name: "Inverter Coil",
                description: INVERTER_COIL_DESCRIPTION,
                tier: PartTier::Fabric,
                stacks: false,
                sprite_lines: INVERTER_COIL_SPRITE_LINES,
                pins: NO_PINS,
                config: NO_CONFIG,
            },
            Part::DelaySpool => PartSpec {
                token: "delayspool",
                display_name: "Delay Spool",
                description: DELAY_SPOOL_DESCRIPTION,
                tier: PartTier::Fabric,
                stacks: true,
                sprite_lines: DELAY_SPOOL_SPRITE_LINES,
                pins: NO_PINS,
                config: NO_CONFIG,
            },
            Part::ShoalCounter => PartSpec {
                token: "shoalcounter",
                display_name: "Shoal Counter",
                description: SHOAL_COUNTER_DESCRIPTION,
                tier: PartTier::Senses,
                stacks: false,
                sprite_lines: SHOAL_COUNTER_SPRITE_LINES,
                pins: SHOAL_COUNTER_PINS,
                config: SHOAL_COUNTER_CONFIG,
            },
            Part::TidalClock => PartSpec {
                token: "tidalclock",
                display_name: "Tidal Clock",
                description: TIDAL_CLOCK_DESCRIPTION,
                tier: PartTier::Senses,
                stacks: false,
                sprite_lines: TIDAL_CLOCK_SPRITE_LINES,
                pins: TIDAL_CLOCK_PINS,
                config: NO_CONFIG,
            },
            Part::AssayScale => PartSpec {
                token: "assayscale",
                display_name: "Assay Scale",
                description: ASSAY_SCALE_DESCRIPTION,
                tier: PartTier::Senses,
                stacks: false,
                sprite_lines: ASSAY_SCALE_SPRITE_LINES,
                pins: ASSAY_SCALE_PINS,
                config: ASSAY_SCALE_CONFIG,
            },
            Part::LedgerNerve => PartSpec {
                token: "ledgernerve",
                display_name: "Ledger Nerve",
                description: LEDGER_NERVE_DESCRIPTION,
                tier: PartTier::Senses,
                stacks: false,
                sprite_lines: LEDGER_NERVE_SPRITE_LINES,
                pins: LEDGER_NERVE_PINS,
                config: LEDGER_NERVE_CONFIG,
            },
            Part::GeigerCoil => PartSpec {
                token: "geigercoil",
                display_name: "Geiger Coil",
                description: GEIGER_COIL_DESCRIPTION,
                tier: PartTier::Senses,
                stacks: false,
                sprite_lines: GEIGER_COIL_SPRITE_LINES,
                pins: GEIGER_COIL_PINS,
                config: GEIGER_COIL_CONFIG,
            },
            Part::StartleNerve => PartSpec {
                token: "startlenerve",
                display_name: "Startle Nerve",
                description: STARTLE_NERVE_DESCRIPTION,
                tier: PartTier::Senses,
                stacks: false,
                sprite_lines: STARTLE_NERVE_SPRITE_LINES,
                pins: STARTLE_NERVE_PINS,
                config: NO_CONFIG,
            },
            Part::CommandModule => PartSpec {
                token: "commandmodule",
                display_name: "Command Module",
                description: COMMAND_MODULE_DESCRIPTION,
                tier: PartTier::Hands,
                stacks: false,
                sprite_lines: COMMAND_MODULE_SPRITE_LINES,
                pins: COMMAND_MODULE_PINS,
                config: NO_CONFIG,
            },
            Part::Cochlea => PartSpec {
                token: "cochlea",
                display_name: "Cochlea",
                description: COCHLEA_DESCRIPTION,
                tier: PartTier::Peripherals,
                stacks: false,
                sprite_lines: COCHLEA_SPRITE_LINES,
                pins: COCHLEA_PINS,
                config: NO_CONFIG,
            },
            Part::ReflexArc => PartSpec {
                token: "reflexarc",
                display_name: "Reflex Arc",
                description: REFLEX_ARC_DESCRIPTION,
                tier: PartTier::Peripherals,
                stacks: false,
                sprite_lines: REFLEX_ARC_SPRITE_LINES,
                pins: &REFLEX_ARC_PINS,
                config: &REFLEX_ARC_CONFIG,
            },
            Part::GlyphPanel => PartSpec {
                token: "glyphpanel",
                display_name: "Glyph Panel",
                description: GLYPH_PANEL_DESCRIPTION,
                tier: PartTier::Peripherals,
                stacks: false,
                sprite_lines: GLYPH_PANEL_SPRITE_LINES,
                pins: GLYPH_PANEL_PINS,
                config: GLYPH_PANEL_CONFIG,
            },
            Part::CoreStack => PartSpec {
                token: "corestack",
                display_name: "Core Stack",
                description: CORE_STACK_DESCRIPTION,
                tier: PartTier::Peripherals,
                stacks: false,
                sprite_lines: CORE_STACK_SPRITE_LINES,
                pins: CORE_STACK_PINS,
                config: NO_CONFIG,
            },
            Part::CathodeArray => PartSpec {
                token: "cathodearray",
                display_name: "Cathode Array",
                description: CATHODE_ARRAY_DESCRIPTION,
                tier: PartTier::Peripherals,
                stacks: false,
                sprite_lines: CATHODE_ARRAY_SPRITE_LINES,
                pins: CATHODE_ARRAY_PINS,
                config: CATHODE_ARRAY_CONFIG,
            },
            Part::RelayMast => PartSpec {
                token: "relaymast",
                display_name: "Relay Mast",
                description: RELAY_MAST_DESCRIPTION,
                tier: PartTier::Peripherals,
                stacks: false,
                sprite_lines: RELAY_MAST_SPRITE_LINES,
                pins: RELAY_MAST_PINS,
                config: RELAY_MAST_CONFIG,
            },
            Part::AnglerRig => PartSpec {
                token: "anglerrig",
                display_name: "Angler Rig",
                description: ANGLER_RIG_DESCRIPTION,
                tier: PartTier::Hands,
                stacks: false,
                sprite_lines: ANGLER_RIG_SPRITE_LINES,
                pins: ANGLER_RIG_PINS,
                config: ANGLER_RIG_CONFIG,
            },
        }
    }

    pub fn token(self) -> &'static str {
        self.spec().token
    }

    pub fn parse(s: &str) -> Option<Part> {
        let lower = s.to_ascii_lowercase();
        Part::ALL.iter().copied().find(|p| p.token() == lower)
    }

    pub fn display_name(self) -> &'static str {
        self.spec().display_name
    }

    pub fn description(self) -> &'static str {
        self.spec().description
    }

    pub fn tier(self) -> PartTier {
        self.spec().tier
    }

    pub fn rarity(self) -> Rarity {
        self.tier()
            .rarity()
            .expect("every part sits in a tier that prices it")
    }

    pub fn price(self) -> u32 {
        self.rarity().part_price()
    }

    pub fn stacks(self) -> bool {
        self.spec().stacks
    }

    pub fn sprite_lines(self) -> &'static [&'static str] {
        self.spec().sprite_lines
    }

    pub fn pins(self) -> &'static [PinSpec] {
        self.spec().pins
    }

    pub fn strobes(self) -> impl Iterator<Item = &'static PinSpec> {
        self.pins().iter().filter(|pin| pin.is_strobe())
    }

    pub fn config(self) -> &'static [ConfigSpec] {
        self.spec().config
    }

    pub fn modify_output(self, level: bool, count: u32, memory: &mut VecDeque<bool>) -> bool {
        match self {
            Part::InverterCoil => !level,
            Part::DelaySpool => {
                memory.push_back(level);
                let mut delayed = false;
                while memory.len() > count as usize {
                    delayed = memory.pop_front().unwrap_or(level);
                }
                delayed
            }
            _ => level,
        }
    }

    pub fn report(
        self,
        config: &PartConfig,
        inputs: &dyn Fn(&PinSpec) -> u32,
        buffer: &Buffer,
        world: &WorldView,
        readings: &mut PinReadings,
    ) {
        match self {
            Part::ShoalCounter => read_shoal(config, world, readings),
            Part::TidalClock => read_tide(world, readings),
            Part::AssayScale => read_assay(config, world, readings),
            Part::LedgerNerve => read_ledger(config, world, readings),
            Part::GeigerCoil => read_geiger(config, world, readings),
            Part::StartleNerve => read_startle(world, readings),
            Part::Cochlea => read_cochlea(buffer, readings),
            Part::ReflexArc => read_keys(&REFLEX_ARC_KEYS, buffer, readings),
            Part::CoreStack => read_core(inputs, buffer, readings),
            Part::RelayMast => readings.flag(
                &MAST_OUT_PIN,
                self.link(config).is_some_and(|link| world.relayed(&link)),
            ),
            Part::AnglerRig => read_rig(config, buffer, world, readings),
            Part::InverterCoil
            | Part::DelaySpool
            | Part::CommandModule
            | Part::GlyphPanel
            | Part::CathodeArray => {}
        }
    }

    pub fn on_rising_edge(
        self,
        pin: &str,
        config: &PartConfig,
        inputs: &dyn Fn(&PinSpec) -> u32,
        buffer: &mut Buffer,
    ) -> Option<PartEffect> {
        match self {
            Part::CommandModule if pin == FIRE_PIN.name => Some(PartEffect::RunScript),
            Part::AnglerRig if pin == CAST_PIN.name => Some(PartEffect::Cast),
            Part::Cochlea if pin == STROBE_PIN.name => {
                buffer.advance();
                None
            }
            Part::GlyphPanel if pin == WRITE_PIN.name => {
                buffer.put(inputs(&PANEL_CHAR_PIN) as u8, panel_grid(config));
                None
            }
            Part::GlyphPanel if pin == CLEAR_PIN.name => {
                buffer.blank(panel_grid(config));
                None
            }
            Part::CoreStack if pin == WRITE_PIN.name => {
                buffer.store(inputs(&ADDR_PIN) as usize, inputs(&DATA_IN_PIN) as u8);
                None
            }
            Part::CathodeArray if pin == WRITE_PIN.name => {
                let dot = inputs(&ADDR_PIN) as usize;
                cathode_surface(config).plot(dot, inputs(&BIT_PIN) != 0, buffer);
                None
            }
            Part::CathodeArray if pin == WRITE_BYTE_PIN.name => {
                let address = inputs(&ADDR_PIN) as usize;
                cathode_surface(config).blit(address, inputs(&BYTE_PIN) as u8, buffer);
                None
            }
            _ => None,
        }
    }

    pub fn display(self, config: &PartConfig, buffer: &Buffer) -> Option<Display> {
        match self {
            Part::GlyphPanel => panel_rows(config, buffer).map(Display::Bubble),
            Part::CathodeArray => cathode_surface(config)
                .draw(Glyphs::parse(config.text(&GLYPHS_CONFIG)), buffer)
                .map(Display::Body),
            _ => None,
        }
    }

    pub fn link(self, config: &PartConfig) -> Option<Link> {
        match self {
            Part::RelayMast => Link::new(
                config.text(&FAR_TANK_CONFIG),
                config.text(&FAR_CHANNEL_CONFIG),
            ),
            _ => None,
        }
    }

    pub fn transmit(self, inputs: &dyn Fn(&PinSpec) -> Option<u32>) -> Option<bool> {
        match self {
            Part::RelayMast => inputs(&MAST_IN_PIN).map(|level| level != 0),
            _ => None,
        }
    }

    pub fn hear(self, speech: &str, buffer: &mut Buffer) {
        if self == Part::Cochlea {
            buffer.latch(speech);
        }
    }

    fn key_lines(self) -> &'static [KeyLine] {
        match self {
            Part::ReflexArc => &REFLEX_ARC_KEYS,
            _ => &[],
        }
    }

    pub fn key(self, key: &str, down: bool, config: &PartConfig, buffer: &mut Buffer) {
        for (line, key_line) in self.key_lines().iter().enumerate() {
            if key_line.binds(config, key) {
                buffer.set_line(line, down);
            }
        }
    }

    pub fn bindings(self, config: &PartConfig) -> Vec<KeyBinding> {
        self.key_lines()
            .iter()
            .map(|line| KeyBinding {
                key: config.text(&line.key).to_lowercase(),
                pin: line.pin.name,
            })
            .collect()
    }
}

fn panel_grid(config: &PartConfig) -> Grid {
    let width =
        (config.number(&PANEL_WIDTH_CONFIG) as usize).clamp(DISPLAY_SIDE_MIN, DISPLAY_RAM_CELLS);
    let height = (config.number(&PANEL_HEIGHT_CONFIG) as usize)
        .clamp(DISPLAY_SIDE_MIN, DISPLAY_RAM_CELLS / width);
    Grid { width, height }
}

fn panel_rows(config: &PartConfig, buffer: &Buffer) -> Option<Vec<String>> {
    let rows = buffer.rows(panel_grid(config))?;
    Some(
        rows.iter()
            .map(|row| row.iter().map(|&cell| glyph(cell)).collect())
            .collect(),
    )
}

fn surface_dots() -> usize {
    1 << ADDR_PIN.width
}

fn cathode_surface(config: &PartConfig) -> Surface {
    let width = (config.number(&SURFACE_WIDTH_CONFIG) as usize)
        .clamp(DISPLAY_SIDE_MIN, surface_dots() / DOT_ROWS);
    Surface { width }
}

fn glyph(cell: u8) -> char {
    if PRINTABLE.contains(&cell) {
        return char::from(cell);
    }
    UNDRAWABLE
}

fn configured_selector(config: &PartConfig, spec: &ConfigSpec) -> Selector {
    Selector::parse(config.text(spec)).unwrap_or_default()
}

fn read_shoal(config: &PartConfig, world: &WorldView, readings: &mut PinReadings) {
    let counted = configured_selector(config, &TARGET_CONFIG).count(world.shoal());
    let threshold = config.number(&SHOAL_THRESHOLD_CONFIG);
    readings.set(&COUNT_PIN, counted);
    readings.flag(&FULL_PIN, world.is_full());
    readings.flag(&EMPTY_PIN, counted == 0);
    readings.flag(&OVER_PIN, counted > threshold);
    readings.flag(&UNDER_PIN, counted < threshold);
}

fn read_tide(world: &WorldView, readings: &mut PinReadings) {
    readings.set(&TIME_PIN, world.hour());
    readings.flag(&NIGHT_PIN, world.is_night());
    readings.flag(&DAWN_PIN, world.pulsed(WorldSignal::Dawn));
}

fn read_assay(config: &PartConfig, world: &WorldView, readings: &mut PinReadings) {
    let Some(fish) = configured_selector(config, &ASSAY_TARGET_CONFIG).resolve(world.shoal())
    else {
        readings.set(&WEIGHT_PIN, 0);
        readings.set(&VALUE_PIN, 0);
        readings.flag(&OVER_PIN, false);
        readings.flag(&UNDER_PIN, false);
        readings.flag(&FULL_PIN, false);
        return;
    };
    let measured = if config.text(&MEASURE_CONFIG) == WEIGHT_MEASURE {
        fish.weight_g
    } else {
        fish.value
    };
    let threshold = config.number(&ASSAY_THRESHOLD_CONFIG);
    readings.set(&WEIGHT_PIN, fish.weight_g);
    readings.set(&VALUE_PIN, fish.value);
    readings.flag(&OVER_PIN, measured > threshold);
    readings.flag(&UNDER_PIN, measured < threshold);
    readings.flag(&FULL_PIN, fish.full);
}

fn read_ledger(config: &PartConfig, world: &WorldView, readings: &mut PinReadings) {
    readings.set(&CASH_PIN, world.cash());
    readings.set(&TANK_VALUE_PIN, world.tank_value());
    readings.flag(
        &OVER_PIN,
        world.cash() > config.number(&LEDGER_THRESHOLD_CONFIG),
    );
    readings.flag(&BROKE_PIN, world.cash() == 0);
}

fn read_geiger(config: &PartConfig, world: &WorldView, readings: &mut PinReadings) {
    readings.set(&RADS_PIN, world.rads());
    readings.flag(
        &HOT_PIN,
        world.rads() > config.number(&GEIGER_THRESHOLD_CONFIG),
    );
    readings.flag(&MUTATED_PIN, world.pulsed(WorldSignal::Mutation));
}

fn read_cochlea(buffer: &Buffer, readings: &mut PinReadings) {
    readings.set(&EAR_CHAR_PIN, u32::from(buffer.shown()));
    readings.flag(&READY_PIN, buffer.is_ready());
    readings.flag(&DONE_PIN, buffer.is_done());
}

fn read_core(inputs: &dyn Fn(&PinSpec) -> u32, buffer: &Buffer, readings: &mut PinReadings) {
    let word = buffer.load(inputs(&ADDR_PIN) as usize);
    readings.set(&DATA_OUT_PIN, u32::from(word));
}

fn read_rig(config: &PartConfig, buffer: &Buffer, world: &WorldView, readings: &mut PinReadings) {
    readings.flag(&CAUGHT_PIN, buffer.pulsed());
    readings.flag(
        &BAIT_LOW_PIN,
        world.bait() < config.number(&BAIT_THRESHOLD_CONFIG),
    );
}

fn read_keys(lines: &[KeyLine], buffer: &Buffer, readings: &mut PinReadings) {
    for (line, key_line) in lines.iter().enumerate() {
        readings.flag(&key_line.pin, buffer.line(line));
    }
}

fn read_startle(world: &WorldView, readings: &mut PinReadings) {
    readings.flag(&DEATH_PIN, world.pulsed(WorldSignal::Death));
    readings.flag(&BIRTH_PIN, world.pulsed(WorldSignal::Birth));
    readings.flag(&SALE_PIN, world.pulsed(WorldSignal::Sale));
    readings.flag(&CATCH_PIN, world.pulsed(WorldSignal::Catch));
    readings.flag(&ABDUCTION_PIN, world.pulsed(WorldSignal::Abduction));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fishes::species::FishSpecies;
    use crate::tank::SensedFish;
    use std::collections::BTreeSet;

    #[test]
    fn every_part_is_named_described_and_drawn() {
        for &part in Part::ALL {
            assert!(!part.token().is_empty(), "{part:?} has no token");
            assert!(
                !part.display_name().is_empty(),
                "{part:?} has no display name"
            );
            assert!(
                !part.description().is_empty(),
                "{part:?} has no description"
            );
            assert!(!part.sprite_lines().is_empty(), "{part:?} has no sprite");
            assert!(part.price() > 0, "{part:?} is free");
        }
    }

    #[test]
    fn a_token_round_trips_through_parse() {
        for &part in Part::ALL {
            assert_eq!(Part::parse(part.token()), Some(part));
            assert_eq!(Part::parse(&part.token().to_ascii_uppercase()), Some(part));
        }
    }

    #[test]
    fn an_unknown_token_is_not_a_part() {
        assert_eq!(Part::parse("flipflop"), None);
        assert_eq!(Part::parse(""), None);
    }

    #[test]
    fn every_part_has_its_own_token_and_name() {
        let tokens: BTreeSet<&str> = Part::ALL.iter().map(|p| p.token()).collect();
        let names: BTreeSet<&str> = Part::ALL.iter().map(|p| p.display_name()).collect();
        assert_eq!(tokens.len(), Part::ALL.len(), "one token per part");
        assert_eq!(names.len(), Part::ALL.len(), "one name per part");
    }

    #[test]
    fn every_declared_pin_is_uniquely_named_and_within_range() {
        for &part in Part::ALL {
            let mut seen = BTreeSet::new();
            for pin in part.pins() {
                assert!(
                    seen.insert(pin.name),
                    "{part:?} declares {} twice",
                    pin.name
                );
                assert!(
                    (PIN_WIDTH_MIN..=PIN_WIDTH_MAX).contains(&pin.width),
                    "{part:?} pin {} is {} bits wide",
                    pin.name,
                    pin.width
                );
            }
        }
    }

    #[test]
    fn every_config_field_is_uniquely_named_within_its_part() {
        for &part in Part::ALL {
            let mut seen = BTreeSet::new();
            for field in part.config() {
                assert!(
                    seen.insert(field.name),
                    "{part:?} configures {} twice",
                    field.name
                );
            }
        }
    }

    #[test]
    fn a_part_reports_on_every_out_pin_it_declares_and_on_nothing_else() {
        let world = board(&[("Ann", FishSpecies::Merluza, 400, 90)]);
        for &part in Part::ALL {
            let readings = read(part, &PartConfig::new(), &world);
            for pin in part.pins() {
                let reported = readings.get(pin.name).is_some();
                assert_eq!(
                    reported,
                    pin.direction == PinDirection::Out,
                    "{part:?} pin {} is an input the part drives, or an output it left floating",
                    pin.name
                );
            }
            assert_eq!(
                readings.is_empty(),
                !part
                    .pins()
                    .iter()
                    .any(|pin| pin.direction == PinDirection::Out),
                "{part:?} reports on a pin it never declared"
            );
        }
    }

    #[test]
    fn the_command_module_only_listens_and_it_listens_on_one_wire() {
        let pins = Part::CommandModule.pins();
        assert_eq!(pins.len(), 1, "one way in, and that is all");
        assert_eq!(pins[0].name, "fire");
        assert_eq!(pins[0].direction, PinDirection::In);
        assert_eq!(pins[0].width, PIN_WIDTH_MIN);
    }

    fn edge(part: Part, pin: &str, buffer: &mut Buffer) -> Option<PartEffect> {
        part.on_rising_edge(pin, &PartConfig::new(), &|_| 0, buffer)
    }

    const HANDS: [(Part, &str); 2] = [(Part::CommandModule, "fire"), (Part::AnglerRig, "cast")];

    #[test]
    fn only_the_hands_hand_the_fish_an_effect_and_each_only_on_its_own_pin() {
        let mut buffer = Buffer::new();
        assert!(matches!(
            edge(Part::CommandModule, "fire", &mut buffer),
            Some(PartEffect::RunScript)
        ));
        assert!(matches!(
            edge(Part::AnglerRig, "cast", &mut buffer),
            Some(PartEffect::Cast)
        ));
        for &part in Part::ALL {
            let own = HANDS
                .iter()
                .find(|(hand, _)| *hand == part)
                .map(|(_, pin)| *pin);
            assert!(
                edge(part, OUTPUT_PIN.name, &mut buffer).is_none(),
                "{part:?} owns its own pins, not every pin on the fish"
            );
            for (_, pin) in HANDS.iter().filter(|(_, pin)| Some(*pin) != own) {
                assert!(
                    edge(part, pin, &mut buffer).is_none(),
                    "{part:?} answers a pin it does not own"
                );
            }
            for pin in part
                .pins()
                .iter()
                .filter(|pin| pin.direction == PinDirection::In && Some(pin.name) != own)
            {
                assert!(
                    edge(part, pin.name, &mut buffer).is_none(),
                    "{part:?} asks the fish to act on its {} edge; only a hand does",
                    pin.name
                );
            }
        }
    }

    #[test]
    fn a_strobe_is_a_one_bit_input_declared_one_and_every_other_input_is_a_value() {
        for &part in Part::ALL {
            for pin in part.strobes() {
                assert_eq!(pin.direction, PinDirection::In, "{part:?} {}", pin.name);
                assert_eq!(pin.width, PIN_WIDTH_MIN, "{part:?} {}", pin.name);
            }
            for pin in part.pins().iter().filter(|pin| pin.width > PIN_WIDTH_MIN) {
                assert!(!pin.is_strobe(), "{part:?} clocks its {} bus", pin.name);
            }
        }
        let strobes: Vec<&str> = Part::GlyphPanel.strobes().map(|pin| pin.name).collect();
        assert_eq!(
            strobes,
            vec!["write", "clear"],
            "the char bus is read, never clocked"
        );
        let strobes: Vec<&str> = Part::CathodeArray.strobes().map(|pin| pin.name).collect();
        assert_eq!(
            strobes,
            vec!["write", "write_byte"],
            "the bit is a data line, read as a level at the rise of write"
        );
    }

    #[test]
    fn the_fabric_is_common_silicon_and_a_sense_is_not() {
        assert_eq!(Part::InverterCoil.rarity(), Rarity::Common);
        assert_eq!(Part::DelaySpool.rarity(), Rarity::Common);
        assert_eq!(Part::ShoalCounter.rarity(), Rarity::Rare);
        assert!(Part::ShoalCounter.price() > Part::InverterCoil.price());
    }

    #[test]
    fn a_parts_tier_is_the_one_knob_and_it_prices_the_part() {
        for &part in Part::ALL {
            let tier = part.tier();
            assert_ne!(
                tier,
                PartTier::Materials,
                "{part:?} is hardware, never bench material"
            );
            assert_eq!(
                tier.rarity(),
                Some(part.rarity()),
                "{part:?} is priced apart from the tier it is sold in"
            );
        }
        assert_eq!(
            PartTier::Materials.rarity(),
            None,
            "materials are priced by the item, not by a tier"
        );
    }

    #[test]
    fn every_tier_names_itself_exactly_once_and_knows_its_place() {
        let mut names: Vec<&str> = PartTier::ALL
            .iter()
            .map(|tier| tier.display_name())
            .collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), PartTier::ALL.len(), "two tiers share a name");
        for (index, &tier) in PartTier::ALL.iter().enumerate() {
            assert_eq!(tier.index(), index, "index() disagrees with ALL");
        }
    }

    #[test]
    fn the_tiers_run_from_fabric_to_exotic_before_the_materials_they_are_bought_with() {
        assert_eq!(PartTier::ALL.last(), Some(&PartTier::Materials));
        let rarities: Vec<Option<Rarity>> =
            PartTier::ALL.iter().map(|tier| tier.rarity()).collect();
        assert_eq!(
            rarities,
            vec![
                Some(Rarity::Common),
                Some(Rarity::Rare),
                Some(Rarity::Rare),
                Some(Rarity::Legendary),
                None,
            ]
        );
    }

    #[test]
    fn a_parts_price_is_its_rarity_and_nothing_else() {
        for &part in Part::ALL {
            assert_eq!(
                part.price(),
                part.rarity().part_price(),
                "{part:?} carries a price of its own"
            );
        }
        assert_eq!(
            Part::CommandModule.price(),
            Part::TidalClock.price(),
            "parts of one rarity cost the same, so none can be mispriced against another"
        );
    }

    #[test]
    fn only_the_spool_stacks() {
        assert!(Part::DelaySpool.stacks());
        for &part in Part::ALL.iter().filter(|&&p| p != Part::DelaySpool) {
            assert!(!part.stacks(), "{part:?} should be one to a fish");
        }
    }

    const LONE_COIL: u32 = 1;
    const LONE_SPOOL: u32 = 1;
    const SPOOL_STACK: u32 = 3;
    const STAGES_WATCHED: usize = 8;

    fn coil(level: bool) -> bool {
        Part::InverterCoil.modify_output(level, LONE_COIL, &mut VecDeque::new())
    }

    fn spool_run(spools: u32, inputs: &[bool]) -> Vec<bool> {
        let mut memory = VecDeque::new();
        inputs
            .iter()
            .map(|&level| Part::DelaySpool.modify_output(level, spools, &mut memory))
            .collect()
    }

    #[test]
    fn the_coil_denies_everything_it_is_told() {
        assert!(!coil(true));
        assert!(coil(false));
    }

    #[test]
    fn a_sense_leaves_the_fishs_own_output_alone() {
        for &part in Part::ALL.iter().filter(|p| !p.pins().is_empty()) {
            assert!(part.modify_output(true, 1, &mut VecDeque::new()));
            assert!(!part.modify_output(false, 1, &mut VecDeque::new()));
        }
    }

    #[test]
    fn a_spool_hands_back_what_it_was_given_a_stage_ago() {
        let inputs = [true, false, true, false];
        assert_eq!(
            spool_run(LONE_SPOOL, &inputs),
            vec![false, true, false, true]
        );
    }

    #[test]
    fn a_stack_of_spools_delays_by_one_stage_each() {
        let mut inputs = vec![true];
        inputs.extend(std::iter::repeat_n(false, STAGES_WATCHED - 1));
        let out = spool_run(SPOOL_STACK, &inputs);
        assert_eq!(
            out.iter().position(|&level| level),
            Some(SPOOL_STACK as usize),
            "three spools push the pulse three stages downstream"
        );
    }

    #[test]
    fn a_spool_starts_low_until_its_line_has_filled() {
        let out = spool_run(SPOOL_STACK, &[true, true, true]);
        assert_eq!(out, vec![false, false, false]);
    }

    #[test]
    fn a_sprite_is_a_rectangle_of_drawable_rows() {
        for &part in Part::ALL {
            for line in part.sprite_lines() {
                assert!(!line.is_empty(), "{part:?} has a blank sprite row");
            }
        }
    }

    const TANK_CAPACITY: usize = 50;
    const PURSE: u32 = 2000;

    fn board(fish: &[(&str, FishSpecies, u32, u32)]) -> WorldView {
        WorldView::new(
            fish.iter()
                .map(|&(name, species, weight_g, value)| SensedFish {
                    name: name.to_string(),
                    species,
                    weight_g,
                    value,
                    full: false,
                })
                .collect(),
            TANK_CAPACITY,
            PURSE,
            0,
            0,
            BTreeSet::new(),
        )
    }

    fn configured(part: Part, fields: &[(&str, &str)]) -> PartConfig {
        let mut config = PartConfig::new();
        for &(name, value) in fields {
            let spec = part
                .config()
                .iter()
                .find(|spec| spec.name == name)
                .unwrap_or_else(|| panic!("{part:?} has no {name} field"));
            config.set(spec, value);
        }
        config
    }

    fn read(part: Part, config: &PartConfig, world: &WorldView) -> PinReadings {
        let mut readings = PinReadings::new();
        part.report(config, &|_| 0, &Buffer::new(), world, &mut readings);
        readings
    }

    fn high(readings: &PinReadings, pin: &str) -> bool {
        readings.get(pin).expect("the pin is driven") == 1
    }

    fn shoal() -> WorldView {
        board(&[
            ("Ann", FishSpecies::Merluza, 400, 90),
            ("Bob", FishSpecies::Betta, 900, 20),
            ("Cid", FishSpecies::Merluza, 100, 500),
        ])
    }

    #[test]
    fn the_shoal_counter_counts_the_whole_tank_by_default() {
        let readings = read(Part::ShoalCounter, &PartConfig::new(), &shoal());
        assert_eq!(readings.get("count"), Some(3));
        assert!(!high(&readings, "empty"));
        assert!(!high(&readings, "full"));
    }

    #[test]
    fn the_shoal_counter_counts_only_what_its_target_admits() {
        let config = configured(Part::ShoalCounter, &[("target", "merluza")]);
        assert_eq!(
            read(Part::ShoalCounter, &config, &shoal()).get("count"),
            Some(2)
        );
    }

    #[test]
    fn over_and_under_are_the_two_sides_of_one_threshold() {
        let below = configured(Part::ShoalCounter, &[("threshold", "5")]);
        let readings = read(Part::ShoalCounter, &below, &shoal());
        assert!(high(&readings, "under"));
        assert!(!high(&readings, "over"));

        let above = configured(Part::ShoalCounter, &[("threshold", "2")]);
        let readings = read(Part::ShoalCounter, &above, &shoal());
        assert!(high(&readings, "over"));
        assert!(!high(&readings, "under"));
    }

    #[test]
    fn a_count_that_sits_exactly_on_the_threshold_is_neither_side() {
        let config = configured(Part::ShoalCounter, &[("threshold", "3")]);
        let readings = read(Part::ShoalCounter, &config, &shoal());
        assert!(!high(&readings, "over"));
        assert!(!high(&readings, "under"));
    }

    #[test]
    fn an_empty_tank_says_so_and_a_packed_one_says_full() {
        let readings = read(Part::ShoalCounter, &PartConfig::new(), &board(&[]));
        assert!(high(&readings, "empty"));

        let packed = vec![("Ann", FishSpecies::Merluza, 1, 1); TANK_CAPACITY];
        assert!(high(
            &read(Part::ShoalCounter, &PartConfig::new(), &board(&packed)),
            "full"
        ));
    }

    fn at_hour(hour: u32, signals: BTreeSet<WorldSignal>) -> WorldView {
        WorldView::new(Vec::new(), TANK_CAPACITY, 0, 0, hour, signals)
    }

    #[test]
    fn the_tidal_clock_reports_the_hour_on_its_bus() {
        let world = at_hour(13, BTreeSet::new());
        assert_eq!(
            read(Part::TidalClock, &PartConfig::new(), &world).get("time"),
            Some(13)
        );
    }

    #[test]
    fn the_night_flag_follows_the_hour_and_dawn_only_pulses() {
        let world = at_hour(0, BTreeSet::new());
        let midnight = read(Part::TidalClock, &PartConfig::new(), &world);
        assert!(high(&midnight, "night"));
        assert!(!high(&midnight, "dawn"), "no turn, no pulse");

        let world = at_hour(12, BTreeSet::new());
        assert!(!high(
            &read(Part::TidalClock, &PartConfig::new(), &world),
            "night"
        ));

        let world = at_hour(6, BTreeSet::from([WorldSignal::Dawn]));
        let sunup = read(Part::TidalClock, &PartConfig::new(), &world);
        assert!(high(&sunup, "dawn"));
        assert!(!high(&sunup, "night"));
    }

    #[test]
    fn the_assay_scale_weighs_the_fish_its_target_picks() {
        let readings = read(Part::AssayScale, &PartConfig::new(), &shoal());
        assert_eq!(readings.get("weight"), Some(900), "heaviest by default");
        assert_eq!(readings.get("value"), Some(20));
    }

    #[test]
    fn the_assay_scale_ranks_by_worth_when_told_to() {
        let config = configured(Part::AssayScale, &[("target", "richest")]);
        let readings = read(Part::AssayScale, &config, &shoal());
        assert_eq!(readings.get("value"), Some(500));
        assert_eq!(readings.get("weight"), Some(100));
    }

    #[test]
    fn over_and_under_compare_whichever_measure_is_configured() {
        let by_value = configured(
            Part::AssayScale,
            &[("target", "richest"), ("threshold", "400")],
        );
        assert!(high(&read(Part::AssayScale, &by_value, &shoal()), "over"));

        let by_weight = configured(
            Part::AssayScale,
            &[
                ("target", "richest"),
                ("measure", "weight"),
                ("threshold", "400"),
            ],
        );
        let readings = read(Part::AssayScale, &by_weight, &shoal());
        assert!(
            high(&readings, "under"),
            "the richest fish weighs 100g, which is under 400"
        );
        assert!(!high(&readings, "over"));
    }

    #[test]
    fn the_assay_scale_says_full_when_its_fish_will_eat_no_more() {
        let mut world = shoal();
        assert!(!high(
            &read(Part::AssayScale, &PartConfig::new(), &world),
            "full"
        ));
        world = WorldView::new(
            world
                .shoal()
                .iter()
                .cloned()
                .map(|fish| SensedFish { full: true, ..fish })
                .collect(),
            TANK_CAPACITY,
            PURSE,
            0,
            0,
            BTreeSet::new(),
        );
        assert!(high(
            &read(Part::AssayScale, &PartConfig::new(), &world),
            "full"
        ));
    }

    #[test]
    fn a_scale_with_nothing_on_it_reads_flat_and_fires_neither_flag() {
        let readings = read(Part::AssayScale, &PartConfig::new(), &board(&[]));
        assert_eq!(readings.get("weight"), Some(0));
        assert_eq!(readings.get("value"), Some(0));
        assert!(
            !high(&readings, "over") && !high(&readings, "under"),
            "an empty tank must not trip a farm"
        );
    }

    #[test]
    fn the_ledger_nerve_reads_the_purse_and_what_swims_in_the_tank() {
        let readings = read(Part::LedgerNerve, &PartConfig::new(), &shoal());
        assert_eq!(readings.get("cash"), Some(PURSE));
        assert_eq!(readings.get("tank_value"), Some(610));
        assert!(high(&readings, "over"));
        assert!(!high(&readings, "broke"));
    }

    #[test]
    fn broke_is_an_empty_purse_not_a_low_one() {
        let skint = WorldView::new(Vec::new(), TANK_CAPACITY, 0, 0, 0, BTreeSet::new());
        let readings = read(Part::LedgerNerve, &PartConfig::new(), &skint);
        assert!(high(&readings, "broke"));
        assert!(!high(&readings, "over"));

        let thin = WorldView::new(Vec::new(), TANK_CAPACITY, 1, 0, 0, BTreeSet::new());
        assert!(!high(
            &read(Part::LedgerNerve, &PartConfig::new(), &thin),
            "broke"
        ));
    }

    fn irradiated(rads: u32, signals: BTreeSet<WorldSignal>) -> WorldView {
        WorldView::new(Vec::new(), TANK_CAPACITY, 0, rads, 0, signals)
    }

    #[test]
    fn the_geiger_coil_reads_the_pressure_and_trips_over_its_threshold() {
        let world = irradiated(0, BTreeSet::new());
        let quiet = read(Part::GeigerCoil, &PartConfig::new(), &world);
        assert_eq!(quiet.get("rads"), Some(0));
        assert!(!high(&quiet, "hot"));

        let world = irradiated(4, BTreeSet::new());
        let loud = read(Part::GeigerCoil, &PartConfig::new(), &world);
        assert_eq!(loud.get("rads"), Some(4));
        assert!(high(&loud, "hot"));
    }

    #[test]
    fn the_geiger_coil_pulses_when_something_changes_shape() {
        let world = irradiated(0, BTreeSet::from([WorldSignal::Mutation]));
        let readings = read(Part::GeigerCoil, &PartConfig::new(), &world);
        assert!(high(&readings, "mutated"));
        assert!(!high(&readings, "hot"), "a mutation is not pressure");
    }

    #[test]
    fn the_startle_nerve_sits_quiet_until_the_world_does_something() {
        let readings = read(Part::StartleNerve, &PartConfig::new(), &board(&[]));
        for pin in Part::StartleNerve.pins() {
            assert!(!high(&readings, pin.name), "{} fired at nothing", pin.name);
        }
    }

    #[test]
    fn every_startle_line_answers_only_its_own_event() {
        let events = [
            (WorldSignal::Death, "death"),
            (WorldSignal::Birth, "birth"),
            (WorldSignal::Sale, "sale"),
            (WorldSignal::Catch, "catch"),
            (WorldSignal::Abduction, "abduction"),
        ];
        for (signal, pin) in events {
            let world =
                WorldView::new(Vec::new(), TANK_CAPACITY, 0, 0, 0, BTreeSet::from([signal]));
            let readings = read(Part::StartleNerve, &PartConfig::new(), &world);
            for &(_, other) in &events {
                assert_eq!(
                    high(&readings, other),
                    other == pin,
                    "{signal:?} should raise {pin} and nothing else"
                );
            }
        }
    }

    fn ear(buffer: &Buffer) -> PinReadings {
        let mut readings = PinReadings::new();
        Part::Cochlea.report(
            &PartConfig::new(),
            &|_| 0,
            buffer,
            &WorldView::default(),
            &mut readings,
        );
        readings
    }

    fn strobe(buffer: &mut Buffer) {
        let effect = edge(Part::Cochlea, STROBE_PIN.name, buffer);
        assert!(effect.is_none(), "the ear asks the fish for nothing");
    }

    #[test]
    fn the_cochlea_is_an_exotic_ear_with_a_byte_bus_a_strobe_and_two_flags() {
        assert_eq!(Part::Cochlea.tier(), PartTier::Peripherals);
        assert_eq!(Part::Cochlea.rarity(), Rarity::Legendary);
        let pins: Vec<(&str, PinDirection, u8)> = Part::Cochlea
            .pins()
            .iter()
            .map(|pin| (pin.name, pin.direction, pin.width))
            .collect();
        assert_eq!(
            pins,
            vec![
                ("char", PinDirection::Out, NARROW_BUS_WIDTH),
                ("strobe", PinDirection::In, PIN_WIDTH_MIN),
                ("ready", PinDirection::Out, PIN_WIDTH_MIN),
                ("done", PinDirection::Out, PIN_WIDTH_MIN),
            ]
        );
    }

    #[test]
    fn a_cochlea_that_has_heard_nothing_holds_every_line_low() {
        let readings = ear(&Buffer::new());
        assert_eq!(readings.get("char"), Some(0));
        assert!(!high(&readings, "ready"));
        assert!(!high(&readings, "done"));
    }

    #[test]
    fn a_heard_line_raises_ready_before_a_single_character_leaves() {
        let mut buffer = Buffer::new();
        Part::Cochlea.hear("6 + 4", &mut buffer);
        let readings = ear(&buffer);
        assert!(high(&readings, "ready"));
        assert!(!high(&readings, "done"));
        assert_eq!(readings.get("char"), Some(0));
    }

    #[test]
    fn the_cochlea_clocks_out_the_line_it_heard_one_byte_per_strobe_and_parses_nothing() {
        let mut buffer = Buffer::new();
        Part::Cochlea.hear("6 + 4", &mut buffer);
        let mut clocked = Vec::new();
        for _ in 0.."6 + 4".len() {
            assert!(high(&ear(&buffer), "ready"), "a character is still waiting");
            strobe(&mut buffer);
            clocked.push(ear(&buffer).get("char").expect("the bus is driven"));
        }
        assert_eq!(clocked, vec![0x36, 0x20, 0x2B, 0x20, 0x34]);
        let end = ear(&buffer);
        assert!(!high(&end, "ready"));
        assert!(high(&end, "done"), "the whole line has left");
    }

    #[test]
    fn only_the_cochlea_hears_a_line_and_only_the_strobe_moves_it() {
        for &part in Part::ALL.iter().filter(|&&p| p != Part::Cochlea) {
            let mut buffer = Buffer::new();
            part.hear("hello", &mut buffer);
            assert_eq!(buffer, Buffer::new(), "{part:?} latched a line");
        }
        let mut buffer = Buffer::new();
        Part::Cochlea.hear("hi", &mut buffer);
        edge(Part::Cochlea, OUTPUT_PIN.name, &mut buffer);
        edge(Part::Cochlea, FIRE_PIN.name, &mut buffer);
        assert_eq!(ear(&buffer).get("char"), Some(0), "no strobe, no character");
    }

    fn panel_config(width: &str, height: &str) -> PartConfig {
        configured(Part::GlyphPanel, &[("width", width), ("height", height)])
    }

    fn write(config: &PartConfig, buffer: &mut Buffer, byte: u8) {
        let on_the_bus = |spec: &PinSpec| {
            assert_eq!(
                spec.name, "char",
                "a write reads the char bus and nothing else"
            );
            u32::from(byte)
        };
        let effect = Part::GlyphPanel.on_rising_edge(WRITE_PIN.name, config, &on_the_bus, buffer);
        assert!(effect.is_none(), "the panel asks the fish for nothing");
    }

    fn typed_on(config: &PartConfig, text: &[u8]) -> Buffer {
        let mut buffer = Buffer::new();
        for &byte in text {
            write(config, &mut buffer, byte);
        }
        buffer
    }

    fn shown(config: &PartConfig, buffer: &Buffer) -> Option<Vec<String>> {
        match Part::GlyphPanel.display(config, buffer)? {
            Display::Bubble(rows) => Some(rows),
            Display::Body(_) => panic!("a panel is a bubble above the fish"),
        }
    }

    #[test]
    fn the_glyph_panel_is_an_exotic_display_that_reads_a_byte_bus_and_two_strobes() {
        assert_eq!(Part::GlyphPanel.tier(), PartTier::Peripherals);
        assert_eq!(Part::GlyphPanel.rarity(), Rarity::Legendary);
        let pins: Vec<(&str, PinDirection, u8)> = Part::GlyphPanel
            .pins()
            .iter()
            .map(|pin| (pin.name, pin.direction, pin.width))
            .collect();
        assert_eq!(
            pins,
            vec![
                ("char", PinDirection::In, NARROW_BUS_WIDTH),
                ("write", PinDirection::In, PIN_WIDTH_MIN),
                ("clear", PinDirection::In, PIN_WIDTH_MIN),
            ]
        );
        assert!(
            Part::GlyphPanel
                .pins()
                .iter()
                .any(|pin| Part::Cochlea.pins().iter().any(|ear| ear.name == pin.name)),
            "the panel keeps §7's `char`, the name the Cochlea also uses"
        );
    }

    #[test]
    fn a_panel_nobody_wrote_to_shows_nothing() {
        assert_eq!(shown(&PartConfig::new(), &Buffer::new()), None);
    }

    #[test]
    fn a_panel_is_sixteen_by_two_until_it_is_told_otherwise() {
        let config = PartConfig::new();
        let rows = shown(&config, &typed_on(&config, b"hi")).expect("hi was written");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], format!("{:<16}", "hi"));
        assert_eq!(rows[1], " ".repeat(16));
    }

    #[test]
    fn the_cursor_advances_wraps_and_scrolls_like_a_terminal() {
        let config = panel_config("3", "2");
        assert_eq!(
            shown(&config, &typed_on(&config, b"abcd")),
            Some(vec!["abc".to_string(), "d  ".to_string()])
        );
        assert_eq!(
            shown(&config, &typed_on(&config, b"abcdefg")),
            Some(vec!["def".to_string(), "g  ".to_string()])
        );
    }

    #[test]
    fn clear_blanks_the_panel_and_the_next_write_starts_at_home() {
        let config = panel_config("3", "2");
        let mut buffer = typed_on(&config, b"abcd");
        let effect = Part::GlyphPanel.on_rising_edge(CLEAR_PIN.name, &config, &|_| 0, &mut buffer);
        assert!(effect.is_none());
        assert_eq!(shown(&config, &buffer), None, "a blank panel has no bubble");
        write(&config, &mut buffer, b'z');
        assert_eq!(shown(&config, &buffer).expect("z")[0], "z  ");
    }

    #[test]
    fn a_byte_with_no_printable_ascii_glyph_shows_the_replacement_character() {
        let config = panel_config("4", "1");
        let rows = shown(&config, &typed_on(&config, &[b'A', 0x07, 0x7F, 0xC3])).expect("written");
        assert_eq!(rows, vec!["A\u{FFFD}\u{FFFD}\u{FFFD}".to_string()]);
        let cell = panel_config("1", "1");
        for byte in b'!'..=b'~' {
            assert_eq!(
                shown(&cell, &typed_on(&cell, &[byte])),
                Some(vec![char::from(byte).to_string()]),
                "a printable byte is its own ASCII character"
            );
        }
        assert_eq!(
            shown(&cell, &typed_on(&cell, b" ")),
            None,
            "a space is a blank cell"
        );
    }

    #[test]
    fn a_panel_never_holds_more_cells_than_an_hd44780_has_display_ram() {
        let huge = panel_config("500", "500");
        let rows = shown(&huge, &typed_on(&huge, b"x")).expect("x");
        assert_eq!(rows.len(), 1, "an 80-wide panel has room for one line");
        assert_eq!(rows[0].chars().count(), DISPLAY_RAM_CELLS);
        let tall = panel_config("20", "9");
        let rows = shown(&tall, &typed_on(&tall, b"x")).expect("x");
        assert_eq!(
            (rows.len(), rows[0].len()),
            (4, 20),
            "20×4 is the tallest 20-wide panel"
        );
        let nothing = panel_config("0", "0");
        let rows = shown(&nothing, &typed_on(&nothing, b"x")).expect("x");
        assert_eq!(rows, vec!["x".to_string()], "zero clamps to a single cell");
    }

    #[test]
    fn only_the_two_displays_show_anything() {
        let config = PartConfig::new();
        let buffer = typed_on(&config, b"hi");
        let displays = [Part::GlyphPanel, Part::CathodeArray];
        for &part in Part::ALL.iter().filter(|p| !displays.contains(p)) {
            assert_eq!(
                part.display(&config, &buffer),
                None,
                "{part:?} drew a panel"
            );
        }
        assert!(matches!(
            Part::GlyphPanel.display(&config, &buffer),
            Some(Display::Bubble(_))
        ));
        assert!(matches!(
            Part::CathodeArray.display(&config, &buffer),
            Some(Display::Body(_))
        ));
    }

    fn arc(buffer: &Buffer) -> PinReadings {
        let mut readings = PinReadings::new();
        Part::ReflexArc.report(
            &PartConfig::new(),
            &|_| 0,
            buffer,
            &WorldView::default(),
            &mut readings,
        );
        readings
    }

    fn held_pins(readings: &PinReadings) -> Vec<&'static str> {
        Part::ReflexArc
            .pins()
            .iter()
            .filter(|pin| readings.get(pin.name) == Some(1))
            .map(|pin| pin.name)
            .collect()
    }

    #[test]
    fn the_reflex_arc_is_an_exotic_eight_line_pad_one_byte_wide() {
        assert_eq!(Part::ReflexArc.tier(), PartTier::Peripherals);
        assert_eq!(Part::ReflexArc.rarity(), Rarity::Legendary);
        let pins: Vec<(&str, PinDirection, u8)> = Part::ReflexArc
            .pins()
            .iter()
            .map(|pin| (pin.name, pin.direction, pin.width))
            .collect();
        assert_eq!(
            pins,
            [
                "up", "down", "left", "right", "fire", "alt", "start", "select"
            ]
            .iter()
            .map(|&name| (name, PinDirection::Out, PIN_WIDTH_MIN))
            .collect::<Vec<_>>()
        );
        let fields: Vec<(&str, &str)> = Part::ReflexArc
            .config()
            .iter()
            .map(|spec| (spec.name, spec.default))
            .collect();
        assert_eq!(
            fields,
            vec![
                ("up key", "up"),
                ("down key", "down"),
                ("left key", "left"),
                ("right key", "right"),
                ("fire key", "space"),
                ("alt key", "z"),
                ("start key", "enter"),
                ("select key", "tab"),
            ],
            "every line is mapped to the key a player expects"
        );
        assert_eq!(
            Part::ReflexArc.pins().len(),
            u8::BITS as usize,
            "a pad is one byte: wired pad0..pad7 it is a bus"
        );
    }

    #[test]
    fn an_arc_nobody_touched_holds_every_line_low() {
        let readings = arc(&Buffer::new());
        assert!(held_pins(&readings).is_empty());
        for pin in Part::ReflexArc.pins() {
            assert_eq!(readings.get(pin.name), Some(0), "{} floats", pin.name);
        }
    }

    #[test]
    fn a_pin_is_high_only_while_its_key_is_held() {
        let config = PartConfig::new();
        let mut buffer = Buffer::new();
        Part::ReflexArc.key("up", true, &config, &mut buffer);
        assert_eq!(held_pins(&arc(&buffer)), vec!["up"]);
        Part::ReflexArc.key("space", true, &config, &mut buffer);
        assert_eq!(held_pins(&arc(&buffer)), vec!["up", "fire"]);
        Part::ReflexArc.key("up", false, &config, &mut buffer);
        assert_eq!(held_pins(&arc(&buffer)), vec!["fire"]);
        Part::ReflexArc.key("space", false, &config, &mut buffer);
        assert!(held_pins(&arc(&buffer)).is_empty());
    }

    #[test]
    fn a_key_nobody_mapped_never_reaches_the_circuit() {
        let mut buffer = Buffer::new();
        Part::ReflexArc.key("x", true, &PartConfig::new(), &mut buffer);
        assert!(held_pins(&arc(&buffer)).is_empty(), "the map is the filter");
    }

    #[test]
    fn the_key_map_is_the_players_and_letters_ignore_their_case() {
        let config = configured(Part::ReflexArc, &[("up key", "W"), ("fire key", "w")]);
        let mut buffer = Buffer::new();
        Part::ReflexArc.key("up", true, &config, &mut buffer);
        assert!(
            held_pins(&arc(&buffer)).is_empty(),
            "up is no longer mapped"
        );
        Part::ReflexArc.key("w", true, &config, &mut buffer);
        assert_eq!(
            held_pins(&arc(&buffer)),
            vec!["up", "fire"],
            "one key may hold two lines"
        );
        let bound: Vec<(String, &str)> = Part::ReflexArc
            .bindings(&config)
            .into_iter()
            .map(|binding| (binding.key, binding.pin))
            .collect();
        assert_eq!(bound[0], ("w".to_string(), "up"));
        assert_eq!(bound[4], ("w".to_string(), "fire"));
    }

    #[test]
    fn only_the_reflex_arc_binds_a_key_and_only_it_answers_one() {
        for &part in Part::ALL.iter().filter(|&&p| p != Part::ReflexArc) {
            assert!(part.bindings(&PartConfig::new()).is_empty(), "{part:?}");
            let mut buffer = Buffer::new();
            part.key("up", true, &PartConfig::new(), &mut buffer);
            assert_eq!(buffer, Buffer::new(), "{part:?} felt a key");
        }
        assert_eq!(
            Part::ReflexArc.bindings(&PartConfig::new()).len(),
            Part::ReflexArc.pins().len(),
            "one binding per line"
        );
    }

    #[test]
    fn the_arc_and_the_command_module_each_own_a_fire_pin() {
        let fire = |part: Part| part.pins().iter().any(|pin| pin.name == "fire");
        assert!(fire(Part::ReflexArc) && fire(Part::CommandModule));
    }

    #[test]
    fn a_panel_reports_nothing_because_it_drives_no_wire() {
        let config = PartConfig::new();
        let mut readings = PinReadings::new();
        Part::GlyphPanel.report(
            &config,
            &|_| 0,
            &typed_on(&config, b"hi"),
            &WorldView::default(),
            &mut readings,
        );
        assert!(readings.is_empty());
    }

    const WORDS: usize = 256;

    struct Wiring {
        addr: u32,
        data_in: u32,
    }

    impl Wiring {
        fn value(&self, spec: &PinSpec) -> u32 {
            match spec.name {
                "addr" => self.addr,
                "data_in" => self.data_in,
                other => panic!("the stack reads addr and data_in as values, never {other}"),
            }
        }
    }

    fn store(buffer: &mut Buffer, addr: u32, data_in: u32) {
        let bus = Wiring { addr, data_in };
        let effect = Part::CoreStack.on_rising_edge(
            WRITE_PIN.name,
            &PartConfig::new(),
            &|spec| bus.value(spec),
            buffer,
        );
        assert!(effect.is_none(), "memory asks the fish for nothing");
    }

    fn data_out(buffer: &Buffer, addr: u32) -> u32 {
        let bus = Wiring { addr, data_in: 0 };
        let mut readings = PinReadings::new();
        Part::CoreStack.report(
            &PartConfig::new(),
            &|spec| bus.value(spec),
            buffer,
            &WorldView::default(),
            &mut readings,
        );
        readings.get("data_out").expect("data_out is always driven")
    }

    #[test]
    fn the_core_stack_is_exotic_memory_with_sevens_four_pins() {
        assert_eq!(Part::CoreStack.tier(), PartTier::Peripherals);
        assert_eq!(Part::CoreStack.rarity(), Rarity::Legendary);
        let pins: Vec<(&str, PinDirection, u8)> = Part::CoreStack
            .pins()
            .iter()
            .map(|pin| (pin.name, pin.direction, pin.width))
            .collect();
        assert_eq!(
            pins,
            vec![
                ("addr", PinDirection::In, NARROW_BUS_WIDTH),
                ("data_in", PinDirection::In, NARROW_BUS_WIDTH),
                ("data_out", PinDirection::Out, NARROW_BUS_WIDTH),
                ("write", PinDirection::In, PIN_WIDTH_MIN),
            ]
        );
        let strobes: Vec<&str> = Part::CoreStack.strobes().map(|pin| pin.name).collect();
        assert_eq!(strobes, vec!["write"], "addr and data_in are values");
        assert!(Part::CoreStack.config().is_empty(), "256 words, always");
    }

    #[test]
    fn write_then_read_at_an_address_returns_the_value() {
        let mut buffer = Buffer::new();
        store(&mut buffer, 64, u32::from(b'@'));
        assert_eq!(data_out(&buffer, 64), u32::from(b'@'));
    }

    #[test]
    fn an_unwritten_address_reads_zero() {
        let mut buffer = Buffer::new();
        assert_eq!(
            data_out(&buffer, 0),
            0,
            "a stack nobody wrote to is all zeros"
        );
        store(&mut buffer, 64, 7);
        assert_eq!(data_out(&buffer, 63), 0);
        assert_eq!(data_out(&buffer, 65), 0);
        assert_eq!(data_out(&buffer, 255), 0);
    }

    #[test]
    fn reading_writes_nothing_and_only_the_write_strobe_stores() {
        let mut buffer = Buffer::new();
        let bus = Wiring {
            addr: 9,
            data_in: 200,
        };
        for pin in Part::CoreStack
            .pins()
            .iter()
            .filter(|pin| pin.name != "write")
        {
            let effect = Part::CoreStack.on_rising_edge(
                pin.name,
                &PartConfig::new(),
                &|spec| bus.value(spec),
                &mut buffer,
            );
            assert!(effect.is_none());
        }
        assert_eq!(data_out(&buffer, 9), 0, "no strobe, no word");
        assert_eq!(buffer, Buffer::new(), "a read leaves the memory as it was");
    }

    #[test]
    fn an_eight_bit_address_reaches_every_one_of_the_256_words() {
        let mut buffer = Buffer::new();
        for addr in 0..WORDS as u32 {
            store(&mut buffer, addr, 255 - addr);
        }
        for addr in 0..WORDS as u32 {
            assert_eq!(data_out(&buffer, addr), 255 - addr, "word {addr}");
        }
        assert_eq!(
            1usize << ADDR_PIN.width,
            WORDS,
            "the address bus is exactly as wide as the stack is deep"
        );
    }

    #[test]
    fn a_later_write_to_one_address_replaces_only_that_word() {
        let mut buffer = Buffer::new();
        store(&mut buffer, 1, 10);
        store(&mut buffer, 2, 20);
        store(&mut buffer, 1, 11);
        assert_eq!((data_out(&buffer, 1), data_out(&buffer, 2)), (11, 20));
    }

    struct Screen {
        addr: u32,
        bit: bool,
        byte: u32,
    }

    impl Screen {
        fn value(&self, spec: &PinSpec) -> u32 {
            match spec.name {
                "addr" => self.addr,
                "bit" => u32::from(self.bit),
                "byte" => self.byte,
                other => panic!("the cathode reads addr, bit and byte as values, never {other}"),
            }
        }
    }

    fn strobe_cathode(config: &PartConfig, buffer: &mut Buffer, pin: &str, screen: &Screen) {
        let effect =
            Part::CathodeArray.on_rising_edge(pin, config, &|spec| screen.value(spec), buffer);
        assert!(effect.is_none(), "a screen asks the fish for nothing");
    }

    fn plot(config: &PartConfig, buffer: &mut Buffer, addr: u32, bit: bool) {
        let screen = Screen { addr, bit, byte: 0 };
        strobe_cathode(config, buffer, WRITE_PIN.name, &screen);
    }

    fn blit(config: &PartConfig, buffer: &mut Buffer, addr: u32, byte: u32) {
        let screen = Screen {
            addr,
            bit: false,
            byte,
        };
        strobe_cathode(config, buffer, WRITE_BYTE_PIN.name, &screen);
    }

    fn surface_rows(config: &PartConfig, buffer: &Buffer) -> Option<Vec<String>> {
        match Part::CathodeArray.display(config, buffer)? {
            Display::Body(rows) => Some(rows),
            Display::Bubble(_) => panic!("the cathode draws on the body, never in a bubble"),
        }
    }

    fn narrow(width: &str) -> PartConfig {
        configured(Part::CathodeArray, &[("width", width)])
    }

    #[test]
    fn the_cathode_array_is_an_exotic_screen_with_sevens_three_pins_and_a_parallel_write() {
        assert_eq!(Part::CathodeArray.tier(), PartTier::Peripherals);
        assert_eq!(Part::CathodeArray.rarity(), Rarity::Legendary);
        let pins: Vec<(&str, PinDirection, u8)> = Part::CathodeArray
            .pins()
            .iter()
            .map(|pin| (pin.name, pin.direction, pin.width))
            .collect();
        assert_eq!(
            pins,
            vec![
                ("addr", PinDirection::In, NARROW_BUS_WIDTH),
                ("bit", PinDirection::In, PIN_WIDTH_MIN),
                ("write", PinDirection::In, PIN_WIDTH_MIN),
                ("byte", PinDirection::In, NARROW_BUS_WIDTH),
                ("write_byte", PinDirection::In, PIN_WIDTH_MIN),
            ],
            "§7's addr, bit and write, then the parallel write"
        );
        let fields: Vec<(&str, &str)> = Part::CathodeArray
            .config()
            .iter()
            .map(|spec| (spec.name, spec.default))
            .collect();
        assert_eq!(fields, vec![("width", "64"), ("glyphs", "braille")]);
    }

    #[test]
    fn the_address_bus_reaches_exactly_the_whole_default_surface() {
        let surface = cathode_surface(&PartConfig::new());
        assert_eq!(
            surface.dots(),
            1 << ADDR_PIN.width,
            "256 dots for an 8-bit addr"
        );
        assert_eq!(
            SURFACE_WIDTH_CONFIG.default.parse::<usize>().ok(),
            Some(surface_dots() / DOT_ROWS),
            "the default width is what the address reaches"
        );
        assert_eq!(surface.bytes(), 32, "a 64×4 surface is 32 bytes");
        assert_eq!(
            cathode_surface(&narrow("500")).width,
            64,
            "no wider than the bus reaches"
        );
        assert_eq!(cathode_surface(&narrow("0")).width, 1);
    }

    #[test]
    fn a_cathode_nobody_wrote_to_shows_nothing() {
        assert_eq!(surface_rows(&PartConfig::new(), &Buffer::new()), None);
    }

    #[test]
    fn a_write_stores_the_bit_at_the_address_and_nothing_else_changes() {
        let config = narrow("8");
        let mut buffer = Buffer::new();
        plot(&config, &mut buffer, 1, true);
        assert_eq!(
            surface_rows(&config, &buffer),
            Some(vec!["⠈⠀⠀⠀".to_string()]),
            "dot 1 is the second dot of the top row"
        );
        let lit: Vec<usize> = (0..32).filter(|&dot| buffer.load_bit(dot)).collect();
        assert_eq!(lit, vec![1]);
        plot(&config, &mut buffer, 1, false);
        assert_eq!(
            surface_rows(&config, &buffer),
            None,
            "a low bit clears the dot"
        );
    }

    #[test]
    fn a_write_without_the_strobe_does_nothing() {
        let config = narrow("8");
        let mut buffer = Buffer::new();
        let screen = Screen {
            addr: 3,
            bit: true,
            byte: 0xFF,
        };
        for pin in ["addr", "bit", "byte"] {
            strobe_cathode(&config, &mut buffer, pin, &screen);
        }
        assert_eq!(buffer, Buffer::new(), "only a strobe writes");
    }

    #[test]
    fn the_parallel_write_stores_eight_dots_of_a_row_in_one_strobe() {
        let config = narrow("8");
        let mut buffer = Buffer::new();
        blit(&config, &mut buffer, 2, 0xFF);
        assert_eq!(
            surface_rows(&config, &buffer),
            Some(vec!["⠤⠤⠤⠤".to_string()]),
            "byte 2 of an 8-wide surface is its whole third row"
        );
        blit(&config, &mut buffer, 2, 0b0000_0001);
        assert_eq!(
            surface_rows(&config, &buffer),
            Some(vec!["⠄⠀⠀⠀".to_string()]),
            "a byte replaces all eight dots, lowest bit leftmost"
        );
    }

    #[test]
    fn a_dot_and_a_byte_address_count_different_things() {
        let config = narrow("8");
        let mut buffer = Buffer::new();
        plot(&config, &mut buffer, 1, true);
        blit(&config, &mut buffer, 1, 0b0000_0001);
        assert_eq!(
            surface_rows(&config, &buffer),
            Some(vec!["⠊⠀⠀⠀".to_string()]),
            "addr 1 is dot 1 for write and byte 1 (dot 8) for write_byte"
        );
    }

    #[test]
    fn a_write_past_the_end_of_the_surface_lands_nowhere() {
        let config = narrow("8");
        let mut buffer = Buffer::new();
        plot(&config, &mut buffer, 32, true);
        blit(&config, &mut buffer, 4, 0xFF);
        assert_eq!(surface_rows(&config, &buffer), None);
    }

    #[test]
    fn the_cells_are_one_run_of_memory_and_the_width_only_says_where_rows_break() {
        let mut buffer = Buffer::new();
        plot(&narrow("8"), &mut buffer, 8, true);
        assert_eq!(
            surface_rows(&narrow("4"), &buffer),
            Some(vec!["⠄⠀".to_string()]),
            "dot 8 is the first dot of the third row at width 4"
        );
    }

    #[test]
    fn quadrants_draw_the_same_surface_two_rows_tall() {
        let config = configured(
            Part::CathodeArray,
            &[("width", "4"), ("glyphs", "quadrants")],
        );
        let mut buffer = Buffer::new();
        blit(&config, &mut buffer, 0, 0xFF);
        assert_eq!(
            surface_rows(&config, &buffer),
            Some(vec!["██".to_string(), "  ".to_string()]),
            "byte 0 of a 4-wide surface is its top two rows"
        );
    }

    fn mast(tank: &str, channel: &str) -> PartConfig {
        configured(Part::RelayMast, &[("tank", tank), ("channel", channel)])
    }

    #[test]
    fn the_relay_mast_is_an_exotic_tower_with_one_wire_in_and_one_out() {
        assert_eq!(Part::RelayMast.tier(), PartTier::Peripherals);
        assert_eq!(Part::RelayMast.rarity(), Rarity::Legendary);
        let pins: Vec<(&str, PinDirection, u8, bool)> = Part::RelayMast
            .pins()
            .iter()
            .map(|pin| (pin.name, pin.direction, pin.width, pin.is_strobe()))
            .collect();
        assert_eq!(
            pins,
            vec![
                ("in", PinDirection::In, PIN_WIDTH_MIN, false),
                ("out", PinDirection::Out, PIN_WIDTH_MIN, false),
            ],
            "in is a level to forward, never an edge"
        );
        let fields: Vec<(&str, &str)> = Part::RelayMast
            .config()
            .iter()
            .map(|field| (field.name, field.default))
            .collect();
        assert_eq!(fields, vec![("tank", ""), ("channel", "")]);
    }

    #[test]
    fn a_mast_is_tuned_to_exactly_the_tank_and_channel_it_was_given() {
        assert_eq!(Part::RelayMast.link(&PartConfig::new()), None);
        assert_eq!(Part::RelayMast.link(&mast("Zion", "")), None);
        assert_eq!(Part::RelayMast.link(&mast("", "y")), None);
        assert_eq!(
            Part::RelayMast.link(&mast("Zion", " Y ")),
            Link::new("Zion", "y")
        );
    }

    #[test]
    fn a_mast_forwards_its_in_wire_and_nothing_when_in_is_unwired() {
        assert_eq!(Part::RelayMast.transmit(&|_| None), None);
        assert_eq!(Part::RelayMast.transmit(&|_| Some(1)), Some(true));
        assert_eq!(Part::RelayMast.transmit(&|_| Some(0)), Some(false));
    }

    #[test]
    fn a_masts_out_reads_back_the_far_channel_it_is_tuned_to() {
        use crate::tank::{Tank, TankKind};
        let link = Link::new("Zion", "y").expect("a link");
        let mut world = WorldView::default().tuned_to([link]);
        world.tune_in(&[]);
        assert!(!high(
            &read(Part::RelayMast, &mast("Zion", "y"), &world),
            "out"
        ));

        let mut tanks = vec![Tank::new("Zion".to_string(), TankKind::Matrix, &[])];
        tanks[0].channels.set_level("y", true);
        world.tune_in(&tanks);
        assert!(high(
            &read(Part::RelayMast, &mast("zion", "Y"), &world),
            "out"
        ));
        assert!(
            !high(&read(Part::RelayMast, &mast("Zion", "z"), &world), "out"),
            "a channel the tank never tuned to reads low"
        );
        assert!(
            !high(&read(Part::RelayMast, &PartConfig::new(), &world), "out"),
            "a mast aimed at nothing hears nothing"
        );
    }

    #[test]
    fn only_the_relay_mast_reaches_another_tank() {
        let config = mast("Zion", "y");
        for &part in Part::ALL.iter().filter(|&&p| p != Part::RelayMast) {
            assert_eq!(part.link(&config), None, "{part:?} is tuned to a far tank");
            assert_eq!(
                part.transmit(&|_| Some(1)),
                None,
                "{part:?} puts a level on a far wire"
            );
        }
    }

    fn rig(config: &PartConfig, buffer: &Buffer, bait: u32) -> PinReadings {
        let mut readings = PinReadings::new();
        let world = WorldView::default().with_bait(bait);
        Part::AnglerRig.report(config, &|_| 0, buffer, &world, &mut readings);
        readings
    }

    #[test]
    fn the_angler_rig_is_a_hand_with_a_cast_strobe_and_two_flags() {
        assert_eq!(
            Part::AnglerRig.tier(),
            PartTier::Hands,
            "a part that hands the fish an effect is a Hand"
        );
        assert_eq!(Part::AnglerRig.rarity(), Rarity::Rare);
        let pins: Vec<(&str, PinDirection, u8, bool)> = Part::AnglerRig
            .pins()
            .iter()
            .map(|pin| (pin.name, pin.direction, pin.width, pin.is_strobe()))
            .collect();
        assert_eq!(
            pins,
            vec![
                ("cast", PinDirection::In, PIN_WIDTH_MIN, true),
                ("caught", PinDirection::Out, PIN_WIDTH_MIN, false),
                ("bait_low", PinDirection::Out, PIN_WIDTH_MIN, false),
            ]
        );
        let fields: Vec<(&str, &str)> = Part::AnglerRig
            .config()
            .iter()
            .map(|field| (field.name, field.default))
            .collect();
        assert_eq!(fields, vec![("threshold", "1")]);
    }

    #[test]
    fn caught_is_high_only_while_the_rigs_pulse_is_raised() {
        let mut buffer = Buffer::new();
        assert!(!high(&rig(&PartConfig::new(), &buffer, 5), "caught"));
        buffer.raise();
        assert!(high(&rig(&PartConfig::new(), &buffer, 5), "caught"));
        buffer.settle();
        assert!(!high(&rig(&PartConfig::new(), &buffer, 5), "caught"));
    }

    #[test]
    fn bait_low_by_default_means_the_next_cast_has_nothing_on_the_hook() {
        let buffer = Buffer::new();
        assert!(high(&rig(&PartConfig::new(), &buffer, 0), "bait_low"));
        assert!(!high(&rig(&PartConfig::new(), &buffer, 1), "bait_low"));

        let config = configured(Part::AnglerRig, &[("threshold", "10")]);
        assert!(high(&rig(&config, &buffer, 9), "bait_low"));
        assert!(!high(&rig(&config, &buffer, 10), "bait_low"));
    }

    #[test]
    fn a_cast_asks_the_fish_to_fish_and_touches_nothing_of_its_own() {
        let mut buffer = Buffer::new();
        assert!(matches!(
            edge(Part::AnglerRig, "cast", &mut buffer),
            Some(PartEffect::Cast)
        ));
        assert_eq!(
            buffer,
            Buffer::new(),
            "the line is the fish's, not the rig's"
        );
    }
}
