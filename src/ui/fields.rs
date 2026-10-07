use rand::RngExt;
use ratatui::style::Color;

use crate::colors::{
    BLUE, CYAN, GREEN, LIGHT_BLUE, LIGHT_CYAN, LIGHT_GREEN, LIGHT_MAGENTA, LIGHT_RED, LIGHT_YELLOW,
    MAGENTA, ORANGE, RED, TEAL, YELLOW,
};
use crate::economy::{Money, grouped};
use crate::fishes::fish::{Fed, Fish};
use crate::fishes::species::{Fortune, Sin};
use crate::ui::table::{INFINITY, NOTHING};

pub const FED_HEADER: &str = "Fed";
pub const WORTH_HEADER: &str = "Worth";
const FULL: &str = "Full";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Iq,
    ZodiacSign,
    ChineseZodiac,
    Tanganana,
    HappinessLevel,
    Claustrophobic,
    AttrText,
    AttrEnumBar,
    AttrBoolean,
    PickACard,
    FavoriteColor,
    FavoriteLetter,
    FavoriteNumber,
    TarotPrediction,
    FavoriteQuote,
    FavoriteHour,
    Lonely,
    Gmi,
    ElOLaPr,
    TheOrThePr,
    Crush,
    MarriedTo,
    Hates,
    Status,
    Sin,
    HasSeenTheSky,
    Temperature,
    Delicious,
    Region,
    Dni,
    FavoriteSeason,
    FavoritePassage,
}

pub struct FieldValue {
    pub text: String,
    pub swatch: Option<Color>,
}

impl FieldKind {
    pub fn all() -> &'static [FieldKind] {
        use FieldKind::*;
        &[
            Iq,
            ZodiacSign,
            ChineseZodiac,
            Tanganana,
            HappinessLevel,
            Claustrophobic,
            AttrText,
            AttrEnumBar,
            AttrBoolean,
            PickACard,
            FavoriteColor,
            FavoriteLetter,
            FavoriteNumber,
            TarotPrediction,
            FavoriteQuote,
            FavoriteHour,
            Lonely,
            Gmi,
            ElOLaPr,
            TheOrThePr,
            Crush,
            MarriedTo,
            Hates,
            Status,
            Sin,
            HasSeenTheSky,
            Temperature,
            Delicious,
            Region,
            Dni,
            FavoriteSeason,
            FavoritePassage,
        ]
    }

    pub fn header(self) -> &'static str {
        match self {
            FieldKind::Iq => "IQ",
            FieldKind::ZodiacSign => "Zodiac Sign",
            FieldKind::ChineseZodiac => "Chinese Zodiac Sign",
            FieldKind::Tanganana => "Tangananica or Tanganana?",
            FieldKind::HappinessLevel => "Happiness Level",
            FieldKind::Claustrophobic => "Claustrophobic?",
            FieldKind::AttrText => "Attr text",
            FieldKind::AttrEnumBar => "Attr enum Bar",
            FieldKind::AttrBoolean => "Attr boolean",
            FieldKind::PickACard => "Pick a card",
            FieldKind::FavoriteColor => "Favorite Color",
            FieldKind::FavoriteLetter => "Favorite Letter",
            FieldKind::FavoriteNumber => "Favorite Number",
            FieldKind::TarotPrediction => "Tarot Prediction",
            FieldKind::FavoriteQuote => "Favorite Quote",
            FieldKind::FavoriteHour => "Favorite Hour",
            FieldKind::Lonely => "Lonely?",
            FieldKind::Gmi => "gmi?",
            FieldKind::ElOLaPr => "El o La PR?",
            FieldKind::TheOrThePr => "The or The PR?",
            FieldKind::Crush => "Crush",
            FieldKind::MarriedTo => "Married to",
            FieldKind::Hates => "Hates",
            FieldKind::Status => "Status",
            FieldKind::Sin => "Sin",
            FieldKind::HasSeenTheSky => "Has seen the sky?",
            FieldKind::Temperature => "Temperature (ºC)",
            FieldKind::Delicious => "Delicious?",
            FieldKind::Region => "Region",
            FieldKind::Dni => "DNI",
            FieldKind::FavoriteSeason => "Favorite Season",
            FieldKind::FavoritePassage => "Favorite Passage?",
        }
    }
}

pub fn format_weight(g: u32) -> String {
    if g >= 1000 {
        format!("{:.1}kg", g as f32 / 1000.0)
    } else {
        format!("{}g", g)
    }
}

pub fn format_money(amount: Money) -> String {
    format!("${}", grouped(amount))
}

pub fn format_fed(fed: Fed) -> String {
    match fed {
        Fed::Growing(percent) => format!("{percent}%"),
        Fed::Full => FULL.to_string(),
        Fed::Boundless => INFINITY.to_string(),
        Fed::Worthless => NOTHING.to_string(),
    }
}

pub fn format_worth(fish: &Fish) -> String {
    let worth = format_money(fish.sell_value());
    match fish.worth_when_full() {
        Some(full) => format!("{worth} ({} when full)", format_money(full)),
        None => worth,
    }
}

pub fn generate_rut(rng: &mut impl RngExt) -> String {
    let body: u32 = rng.random_range(1_000_000..25_000_001);
    let s = body.to_string();
    let digits: Vec<u32> = s.chars().rev().map(|c| c as u32 - '0' as u32).collect();
    let multipliers = [2u32, 3, 4, 5, 6, 7];
    let sum: u32 = digits
        .iter()
        .enumerate()
        .map(|(i, &d)| d * multipliers[i % multipliers.len()])
        .sum();
    let rem = 11 - (sum % 11);
    let v = match rem {
        11 => '0',
        10 => 'K',
        n => char::from_digit(n, 10).unwrap_or('0'),
    };
    match s.len() {
        8 => format!("{}.{}.{}-{}", &s[..2], &s[2..5], &s[5..], v),
        _ => format!("{}.{}.{}-{}", &s[..1], &s[1..4], &s[4..], v),
    }
}

pub fn populate_field_cache(fish: &mut Fish, all_names: &[String], rng: &mut impl RngExt) {
    if fish.is_unfish() {
        return;
    }
    let all_kinds = FieldKind::all();
    let n = all_kinds.len();
    if fish.field_cache.len() < n {
        fish.field_cache.resize(n, None);
    }
    for (idx, &kind) in all_kinds.iter().enumerate() {
        if fish.field_cache[idx].is_none() {
            let fv = gen_field_value(kind, fish, all_names, rng);
            fish.field_cache[idx] = Some((fv.text, fv.swatch));
        }
    }
}

pub fn cached_field_value(fish: &Fish, kind: FieldKind) -> Option<FieldValue> {
    let idx = FieldKind::all().iter().position(|&k| k == kind)?;
    let (text, swatch) = fish.field_cache.get(idx)?.as_ref()?;
    Some(FieldValue {
        text: text.clone(),
        swatch: *swatch,
    })
}

pub fn field_value(
    kind: FieldKind,
    fish: &Fish,
    all_names: &[String],
    rng: &mut impl RngExt,
) -> FieldValue {
    if fish.is_unfish() {
        return unfish_field_value(kind, fish);
    }
    cached_field_value(fish, kind).unwrap_or_else(|| gen_field_value(kind, fish, all_names, rng))
}

fn unfish_field_value(kind: FieldKind, fish: &Fish) -> FieldValue {
    match (kind, fish.sin()) {
        (FieldKind::Sin, Some(sin)) => plain(sin.name()),
        _ => plain(""),
    }
}

fn plain(s: impl Into<String>) -> FieldValue {
    FieldValue {
        text: s.into(),
        swatch: None,
    }
}

pub fn gen_field_value(
    kind: FieldKind,
    fish: &Fish,
    all_names: &[String],
    rng: &mut impl RngExt,
) -> FieldValue {
    match kind {
        FieldKind::Iq => {
            let v: i32 = match rng.random_range(0..100u32) {
                0 => -30,
                1 => 3000,
                _ => rng.random_range(55..=145i32),
            };
            plain(v.to_string())
        }

        FieldKind::ZodiacSign => {
            const S: &[&str] = &[
                "Aries",
                "Taurus",
                "Gemini",
                "Cancer",
                "Leo",
                "Virgo",
                "Libra",
                "Scorpio",
                "Sagittarius",
                "Capricorn",
                "Aquarius",
                "Pisces",
            ];
            plain(S[rng.random_range(0..S.len())])
        }

        FieldKind::ChineseZodiac => {
            const S: &[&str] = &[
                "鼠", "牛", "虎", "兔", "龍", "蛇", "馬", "羊", "猴", "雞", "狗", "豬",
            ];
            plain(S[rng.random_range(0..S.len())])
        }

        FieldKind::Tanganana => plain(if rng.random::<bool>() {
            "Tangananica"
        } else {
            "Tanganana"
        }),

        FieldKind::HappinessLevel => plain(format!("{}%", rng.random_range(0..=100u32))),

        FieldKind::Claustrophobic => plain(if rng.random_range(0..10u32) == 0 {
            "Yes"
        } else {
            "No"
        }),

        FieldKind::AttrText => plain("corge"),

        FieldKind::AttrEnumBar => plain(""),

        FieldKind::AttrBoolean => plain(if rng.random::<bool>() { "Sí" } else { "No" }),

        FieldKind::PickACard => {
            const RANKS: &[&str] = &[
                "Ace", "2", "3", "4", "5", "6", "7", "8", "9", "10", "Jack", "Queen", "King",
            ];
            const SUITS: &[&str] = &["Spades", "Hearts", "Diamonds", "Clubs"];
            let roll = rng.random_range(0..54u32);
            if roll >= 52 {
                plain("Joker")
            } else {
                let rank = RANKS[(roll % 13) as usize];
                let suit = SUITS[(roll / 13) as usize];
                plain(format!("{} of {}", rank, suit))
            }
        }

        FieldKind::FavoriteColor => {
            const COLORS: &[Color] = &[
                RED,
                GREEN,
                BLUE,
                YELLOW,
                MAGENTA,
                CYAN,
                LIGHT_RED,
                LIGHT_GREEN,
                LIGHT_BLUE,
                LIGHT_YELLOW,
                LIGHT_MAGENTA,
                LIGHT_CYAN,
                ORANGE,
                TEAL,
            ];
            let c = fish
                .species
                .config()
                .flavour
                .favorite_color
                .unwrap_or_else(|| COLORS[rng.random_range(0..COLORS.len())]);
            FieldValue {
                text: "      ".to_string(),
                swatch: Some(c),
            }
        }

        FieldKind::FavoriteLetter => {
            plain(char::from(b'A' + rng.random_range(0..26u8)).to_string())
        }

        FieldKind::FavoriteNumber => plain(rng.random::<i64>().to_string()),

        FieldKind::TarotPrediction => {
            const MUTANT_SPREADS: &[&str] = &[
                "The Tower + Death + Ten of Swords",
                "Three of Swords + The Devil + Nine of Swords",
                "Five of Pentacles + Ten of Wands + The Moon",
                "The Tower Reversed + Eight of Swords + The Hanged Man",
                "Death Reversed + Four of Pentacles + Judgement Reversed",
                "The Devil Reversed + Seven of Swords + Wheel of Fortune Reversed",
                "Ten of Swords Reversed + The Moon Reversed + Five of Cups",
                "Five of Cups + Hermit Reversed + Lovers Reversed",
                "Justice Reversed + The Tower + King of Pentacles Reversed",
                "Sun Reversed + Star Reversed + Nine of Wands",
            ];
            const GOLDEN_SPREADS: &[&str] = &[
                "The Sun + Ten of Cups + Ace of Pentacles",
                "The Star + Lovers + The World",
                "Wheel of Fortune + Six of Wands + The Emperor",
                "Ace of Cups + The Empress + Four of Wands",
                "The Magician + The Chariot + The Sun",
                "Death + The Star + Ace of Wands",
                "The Devil Reversed + Judgement + The Fool",
                "Nine of Pentacles + King of Pentacles + The World",
                "Two of Cups + Ten of Cups + Star Reversed",
                "Strength + The Hierophant + Sun Reversed",
            ];
            const CARDS: &[&str] = &[
                "The Fool",
                "The Magician",
                "The High Priestess",
                "The Empress",
                "The Emperor",
                "The Hierophant",
                "The Lovers",
                "The Chariot",
                "Strength",
                "The Hermit",
                "Wheel of Fortune",
                "Justice",
                "The Hanged Man",
                "Death",
                "Temperance",
                "The Devil",
                "The Tower",
                "The Star",
                "The Moon",
                "The Sun",
                "Judgement",
                "The World",
            ];
            match fish.species.config().flavour.fortune {
                Some(Fortune::Doomed) => {
                    plain(MUTANT_SPREADS[rng.random_range(0..MUTANT_SPREADS.len())])
                }
                Some(Fortune::Golden) => {
                    plain(GOLDEN_SPREADS[rng.random_range(0..GOLDEN_SPREADS.len())])
                }
                None => {
                    let mut deck: Vec<&str> = CARDS.to_vec();
                    let i1 = rng.random_range(0..deck.len());
                    let c1 = deck.remove(i1);
                    let i2 = rng.random_range(0..deck.len());
                    let c2 = deck.remove(i2);
                    let i3 = rng.random_range(0..deck.len());
                    let c3 = deck.remove(i3);
                    let r1 = if rng.random::<bool>() { " (R)" } else { "" };
                    let r2 = if rng.random::<bool>() { " (R)" } else { "" };
                    let r3 = if rng.random::<bool>() { " (R)" } else { "" };
                    plain(format!("{}{}, {}{}, {}{}", c1, r1, c2, r2, c3, r3))
                }
            }
        }

        FieldKind::FavoriteQuote => match fish.species.config().flavour.favorite_quote {
            Some(quote) => plain(quote),
            None => {
                let count = rng.random_range(2..=8u32);
                let voice = fish.species.config().flavour.voice;
                plain((0..count).map(|_| voice).collect::<Vec<_>>().join(" "))
            }
        },

        FieldKind::FavoriteHour => plain(format!(
            "{:02}:{:02}",
            rng.random_range(0..24u32),
            rng.random_range(0..60u32)
        )),

        FieldKind::Lonely => plain(if rng.random_range(0..10u32) == 0 {
            "Yes"
        } else {
            "No"
        }),

        FieldKind::Gmi => plain(if rng.random::<bool>() { "gmi" } else { "ngmi" }),

        FieldKind::ElOLaPr => plain("La PR"),

        FieldKind::TheOrThePr => plain("The PR"),

        FieldKind::Crush => {
            if all_names.is_empty() {
                plain(super::table::NOTHING)
            } else {
                plain(all_names[rng.random_range(0..all_names.len())].clone())
            }
        }

        FieldKind::MarriedTo => {
            if all_names.is_empty() {
                plain(super::table::NOTHING)
            } else {
                plain(all_names[rng.random_range(0..all_names.len())].clone())
            }
        }

        FieldKind::Hates => {
            if all_names.is_empty() {
                plain("No one")
            } else {
                let target = &all_names[rng.random_range(0..all_names.len())];
                if *target == fish.name {
                    plain("No one")
                } else {
                    plain(target.clone())
                }
            }
        }

        FieldKind::Status => {
            const S: &[&str] = &[
                "Swimming",
                "Pondering",
                "Breathing",
                "Prompting",
                "Prooompting",
                "Fishing",
                "Dreaming",
                "Feeling",
                "Happy",
                "Sad",
                "Nauseous",
                "Kicking Rocks",
                "Giving the Time",
                "Taking out the turn",
                "Falling",
                "Floating",
            ];
            plain(S[rng.random_range(0..S.len())])
        }

        FieldKind::Sin => plain(
            fish.sin()
                .unwrap_or_else(|| Sin::ALL[rng.random_range(0..Sin::ALL.len())])
                .name(),
        ),

        FieldKind::HasSeenTheSky => plain(if fish.species.config().flavour.has_seen_the_sky {
            "Yes"
        } else {
            "No"
        }),

        FieldKind::Temperature => {
            let t = 25.0f32 + rng.random_range(-14.0f32..14.0);
            plain(format!("{:.1}°C", t))
        }

        FieldKind::Delicious => match fish.species.config().flavour.delicious {
            Some(verdict) => plain(verdict),
            None => plain(match rng.random_range(0..3u32) {
                0 => "Yes",
                1 => "No",
                _ => "Maybe",
            }),
        },

        FieldKind::Region => {
            const R: &[&str] = &[
                "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI", "XII", "RM",
                "XIV", "XV",
            ];
            plain(R[rng.random_range(0..R.len())])
        }

        FieldKind::Dni => plain(generate_rut(rng)),

        FieldKind::FavoriteSeason => {
            const S: &[&str] = &["Winter", "Autumn", "Spring", "Summer"];
            plain(S[rng.random_range(0..S.len())])
        }

        FieldKind::FavoritePassage => {
            const PASSAGES: &[&str] = &[
                "\"Let the water teem with living creatures.\" (Genesis 1:20)",
                "\"Follow me, and I will make you fishers of men.\" (Matthew 4:19)",
                "\"Cast the net on the right side of the boat.\" (John 21:6)",
                "\"Bring some of the fish you have just caught.\" (John 21:10)",
                "\"They caught so many fish that their nets began to break.\" (Luke 5:6)",
                "\"He blessed the five loaves and the two fish.\" (Luke 9:16)",
                "\"The Lord is my shepherd; I shall not want.\" (Psalm 23:1)",
                "\"For the love of money is a root of all kinds of evil.\" (1 Timothy 6:10)",
                "\"You cannot serve both God and money.\" (Matthew 6:24)",
                "\"Do not store up treasures on earth, where moth and rust destroy.\" (Matthew 6:19)",
                "\"A generous person will prosper.\" (Proverbs 11:25)",
                "\"Wealth gained hastily will dwindle.\" (Proverbs 13:11)",
                "\"Trust in the Lord with all your heart.\" (Proverbs 3:5)",
                "\"I can do all things through him who strengthens me.\" (Philippians 4:13)",
                "\"Give thanks to the Lord, for he is good.\" (Psalm 107:1)",
            ];
            plain(PASSAGES[rng.random_range(0..PASSAGES.len())])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fishes::species::{ALL_SPECIES, FishSpecies};
    use crate::fishes::unfish::{SPAWNABLE_UNFISH, UnfishKind};

    fn confessed_sin(fish: &Fish) -> String {
        field_value(FieldKind::Sin, fish, &[], &mut rand::rng()).text
    }

    fn fish_of(species: FishSpecies) -> Fish {
        Fish::new(species, "Ann".to_string(), 0.0, 0.0, &mut rand::rng())
    }

    fn unfish_of(kind: UnfishKind) -> Fish {
        Fish::new_unfish(kind, "Ann".to_string(), 0.0, 0.0, &mut rand::rng())
    }

    #[test]
    fn each_sinful_fish_confesses_its_own_sin() {
        let table = [
            (FishSpecies::Cashfish, Sin::Greed),
            (FishSpecies::Holyfish, Sin::Lust),
            (FishSpecies::Candyfish, Sin::Gluttony),
            (FishSpecies::Mutantfish, Sin::Wrath),
            (FishSpecies::Cheatfish, Sin::Pride),
            (FishSpecies::Botfish, Sin::Sloth),
        ];
        for (species, sin) in table {
            assert_eq!(confessed_sin(&fish_of(species)), sin.name(), "{species:?}");
        }
        assert_eq!(
            confessed_sin(&unfish_of(UnfishKind::Doppleganger)),
            Sin::Envy.name()
        );
    }

    #[test]
    fn every_sin_belongs_to_exactly_one_fish() {
        let species_sins = ALL_SPECIES
            .iter()
            .filter_map(|species| species.config().flavour.sin);
        let unfish_sins = SPAWNABLE_UNFISH.iter().filter_map(|kind| kind.sin());
        let owned: Vec<Sin> = species_sins.chain(unfish_sins).collect();
        for sin in Sin::ALL {
            let owners = owned.iter().filter(|&&owned| owned == sin).count();
            assert_eq!(owners, 1, "{} has {owners} fish", sin.name());
        }
    }

    #[test]
    fn an_unfish_without_a_sin_keeps_every_field_blank() {
        let reversed = unfish_of(UnfishKind::Reversed);
        for &kind in FieldKind::all() {
            let field = field_value(kind, &reversed, &[], &mut rand::rng());
            assert!(field.text.is_empty() && field.swatch.is_none());
        }
    }
}
