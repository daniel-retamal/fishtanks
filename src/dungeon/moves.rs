use super::effect::{DamageKind, Effect, Power, Reach, Special, Status, StatusKind};
use super::skill::Skill;

pub const PUFF_THORNS: u32 = 3;
pub const DEVOUR_BELOW_PERCENT: u32 = 25;
pub const CANDIFY_BELOW_PERCENT: u32 = 30;
pub const SMITE: u32 = 9;
pub const HEART: u32 = 12;
pub const KISS: u32 = 10;
pub const VENOM: u32 = 4;
pub const VENOM_TURNS: u8 = 3;
pub const SAW_BLEED: u32 = 3;
pub const SAW_TURNS: u8 = 3;
pub const CLING_DRAIN: u32 = 4;
pub const CORRODE: u32 = 3;
pub const THROWN_JUNK: u32 = 3;
pub const RAIN_PER_STACK: u8 = 3;
pub const RAIN_DROP: u32 = 4;
pub const RAIN_GOLD: u32 = 50;
pub const PAY_TO_WIN_PERCENT: u32 = 10;

const ONE_TURN: u8 = 1;

pub fn puff_up() -> Skill {
    Skill::new(
        "Puff Up",
        1,
        Reach::Me,
        &[Effect::inflict(StatusKind::Puffed, ONE_TURN, PUFF_THORNS)],
        "It puffs up: half damage and triple thorns until its next turn, and it cannot bite or eat.",
    )
}

pub fn divide() -> Skill {
    Skill::new(
        "Divide",
        3,
        Reach::Me,
        &[Effect::Special(Special::Divide)],
        "The double splits in two; the child joins the party as a fish of its own.",
    )
}

pub fn collapse() -> Skill {
    Skill::new(
        "Collapse",
        0,
        Reach::Me,
        &[Effect::Special(Special::Collapse)],
        "The double eats its lighter half and heals fully, keeping every stack.",
    )
    .once()
}

pub fn devour() -> Skill {
    Skill::new(
        "Devour",
        2,
        Reach::OneFoe,
        &[Effect::Special(Special::Devour)],
        "Swallows an enemy that is not a boss and is under a quarter of its HP. Its mass stays.",
    )
}

pub fn call_home() -> Skill {
    Skill::new(
        "Call Home",
        0,
        Reach::Me,
        &[Effect::Special(Special::CallHome)],
        "A UFO drops a companion who joins the party and comes home alienated.",
    )
    .once()
}

pub fn make_it_rain(stacks: u32) -> Skill {
    let drops = u8::try_from(stacks.saturating_mul(u32::from(RAIN_PER_STACK))).unwrap_or(u8::MAX);
    Skill::new(
        "Make it Rain",
        2,
        Reach::RandomFoes(drops),
        &[
            Effect::Hit {
                power: Power::Flat(RAIN_DROP),
                kind: DamageKind::Skill,
                hits: drops,
            },
            Effect::Haul(RAIN_GOLD),
        ],
        "Bursts $ bubbles onto random enemies. Every bubble that pops adds gold to the haul.",
    )
}

pub fn pay_to_win() -> Skill {
    Skill::new(
        "Pay to Win",
        0,
        Reach::OneFoe,
        &[Effect::Special(Special::PayToWin)],
        "Spends a tenth of your purse and turns it into damage that nothing blocks.",
    )
    .sure()
    .paid_from_the_purse(PAY_TO_WIN_PERCENT)
}

pub fn bribe() -> Skill {
    Skill::new(
        "Bribe",
        1,
        Reach::OneFoe,
        &[Effect::Special(Special::Bribe)],
        "An enemy that is not a boss takes the money and leaves.",
    )
    .sure()
    .once()
}

pub fn radiate() -> Skill {
    Skill::new(
        "Radiate",
        2,
        Reach::Everyone,
        &[Effect::Special(Special::Radiate)],
        "One fighter, friend or foe, takes a mutation for good.",
    )
}

pub fn meltdown() -> Skill {
    Skill::new(
        "Meltdown",
        0,
        Reach::Everyone,
        &[Effect::Special(Special::Meltdown)],
        "Every fighter on both sides takes three mutations.",
    )
    .once()
}

pub fn sugar_rush() -> Skill {
    Skill::new(
        "Sugar Rush",
        1,
        Reach::AllAllies,
        &[Effect::Bubbles(1), Effect::Special(Special::SugarRush)],
        "Every ally gains a bubble and candy mass.",
    )
}

pub fn candify() -> Skill {
    Skill::new(
        "Candify",
        3,
        Reach::OneFoe,
        &[Effect::Special(Special::Candify)],
        "An enemy that is not a boss and is under 30% of its HP turns into food.",
    )
}

pub fn miracle() -> Skill {
    Skill::new(
        "Miracle",
        3,
        Reach::Everyone,
        &[Effect::Special(Special::Miracle)],
        "A Blessing comes. Which one is not up to it.",
    )
}

pub fn smite() -> Skill {
    Skill::new(
        "Smite",
        1,
        Reach::OneFoe,
        &[Effect::holy(SMITE)],
        "Strikes with holy light, doubled on the ethereal and the damned.",
    )
    .sure()
}

pub fn script() -> Skill {
    Skill::new(
        "Script",
        0,
        Reach::Me,
        &[Effect::Special(Special::Script)],
        "Its Command Module plays its turn.",
    )
}

pub fn throw_junk() -> Skill {
    Skill::new(
        "Throw Junk",
        0,
        Reach::OneFoe,
        &[Effect::Special(Special::ThrowJunk)],
        "Throws a Junk from Items.",
    )
}

pub fn play_dead() -> Skill {
    Skill::new(
        "Play Dead",
        1,
        Reach::Me,
        &[Effect::inflict(StatusKind::Untouchable, ONE_TURN, 0)],
        "Nobody picks it until its next turn.",
    )
}

pub fn shell() -> Skill {
    Skill::new(
        "Shell",
        1,
        Reach::Me,
        &[Effect::inflict(StatusKind::Untouchable, ONE_TURN, 0)],
        "It hides in its shell: nothing hurts it until its next turn.",
    )
}

pub fn hand_me_down() -> Skill {
    Skill::new(
        "Hand-me-down",
        0,
        Reach::OneAlly,
        &[Effect::Special(Special::HandMeDown)],
        "Gives its shell to an ally.",
    )
}

pub fn hop() -> Skill {
    Skill::new(
        "Hop",
        1,
        Reach::OneFoe,
        &[Effect::bites(2)],
        "Leaps and bites twice.",
    )
}

pub fn ricochet() -> Skill {
    Skill::new(
        "Ricochet",
        2,
        Reach::AllFoes,
        &[Effect::Special(Special::Ricochet)],
        "Bounces from enemy to enemy. Hit every one and the party gains a bubble.",
    )
}

pub fn heart() -> Skill {
    Skill::new(
        "Heart",
        1,
        Reach::OneAlly,
        &[Effect::Heal(HEART), Effect::Special(Special::Heart)],
        "Heals an ally; with a partner, both of the pair.",
    )
}

pub fn lurk() -> Skill {
    Skill::new(
        "Lurk",
        1,
        Reach::Me,
        &[Effect::Special(Special::Lurk)],
        "Into the wall for a turn, then a bite at double that nobody saw coming.",
    )
}

pub fn lunge() -> Skill {
    Skill::new(
        "Lunge",
        1,
        Reach::TwoFoes,
        &[Effect::bite()],
        "Through one enemy and into another.",
    )
}

pub fn duel() -> Skill {
    Skill::new(
        "Duel",
        2,
        Reach::OneFoe,
        &[Effect::Special(Special::Duel)],
        "It and one enemy may target nobody else until one of them is belly up.",
    )
}

pub fn chase() -> Skill {
    Skill::new(
        "Chase",
        1,
        Reach::OneFoe,
        &[Effect::bite(), Effect::Special(Special::Chase)],
        "Bites, and the bitten enemy flees in a zoomie and loses its next turn.",
    )
}

pub fn flash() -> Skill {
    Skill::new(
        "Flash",
        1,
        Reach::AllFoes,
        &[Effect::Inflict(Status::for_turns(
            StatusKind::Blind,
            ONE_TURN,
        ))],
        "Blinds every enemy for a turn.",
    )
}

pub fn eclipse() -> Skill {
    Skill::new(
        "Eclipse",
        0,
        Reach::AllFoes,
        &[Effect::Special(Special::Eclipse)],
        "From the sixth round on, every enemy loses its next turn.",
    )
    .once()
}

pub fn mime() -> Skill {
    Skill::new(
        "Mime",
        1,
        Reach::OneFoe,
        &[Effect::Special(Special::Mime)],
        "Repeats the last thing an enemy did, against them.",
    )
}

pub fn ink() -> Skill {
    Skill::new(
        "Ink",
        1,
        Reach::AllFoes,
        &[Effect::Inflict(Status::for_turns(
            StatusKind::Blind,
            ONE_TURN,
        ))],
        "Every enemy is blinded for a turn.",
    )
}

pub fn hammer() -> Skill {
    Skill::new(
        "Hammer",
        1,
        Reach::OneFoe,
        &[
            Effect::bite(),
            Effect::Inflict(Status::for_turns(StatusKind::Held, ONE_TURN)),
        ],
        "Bites and stuns.",
    )
}

pub fn venom() -> Skill {
    Skill::new(
        "Venom",
        1,
        Reach::OneFoe,
        &[
            Effect::bite(),
            Effect::inflict(StatusKind::Poison, VENOM_TURNS, VENOM),
        ],
        "Bites with its spines; the poison keeps working for three turns.",
    )
}

pub fn kiss() -> Skill {
    Skill::new(
        "Kiss",
        1,
        Reach::OneAlly,
        &[Effect::Heal(KISS), Effect::Special(Special::Kiss)],
        "Heals an ally, or charms an enemy for a turn.",
    )
}

pub fn corrode() -> Skill {
    Skill::new(
        "Corrode",
        1,
        Reach::OneFoe,
        &[Effect::Special(Special::Corrode)],
        "Takes armour off an enemy for the rest of the fight.",
    )
}

pub fn saw() -> Skill {
    Skill::new(
        "Saw",
        1,
        Reach::TwoFoes,
        &[Effect::inflict(StatusKind::Bleed, SAW_TURNS, SAW_BLEED)],
        "Two enemies bleed for three turns.",
    )
}

pub fn wear() -> Skill {
    Skill::new(
        "Wear",
        2,
        Reach::OneFoe,
        &[Effect::Special(Special::Wear)],
        "Puts on an enemy's look and kit.",
    )
}

pub fn cross_over() -> Skill {
    Skill::new(
        "Cross Over",
        0,
        Reach::OneAlly,
        &[Effect::Special(Special::CrossOver)],
        "Carries one ally home through the glass, out of the dive.",
    )
    .once()
}

pub fn blink() -> Skill {
    Skill::new(
        "Blink",
        1,
        Reach::Me,
        &[Effect::inflict(StatusKind::Invisible, ONE_TURN, 0)],
        "Invisible until its next turn: bites pass through it.",
    )
}

pub fn roll() -> Skill {
    Skill::new(
        "Roll",
        1,
        Reach::AllFoes,
        &[Effect::Special(Special::Roll)],
        "Rolls through every enemy, one hit for each of its eyes.",
    )
}

pub fn mind_spike() -> Skill {
    Skill::new(
        "Mind Spike",
        1,
        Reach::OneFoe,
        &[Effect::Hit {
            power: Power::BITE,
            kind: DamageKind::True,
            hits: 1,
        }],
        "Goes through armour and through the ethereal.",
    )
}

pub fn dominate() -> Skill {
    Skill::new(
        "Dominate",
        3,
        Reach::OneFoe,
        &[Effect::Special(Special::Dominate)],
        "An enemy that is not a boss takes its next turn for you.",
    )
}

pub fn erase() -> Skill {
    Skill::new(
        "Erase",
        2,
        Reach::Everyone,
        &[Effect::Special(Special::Erase)],
        "One buff, tell or curse is gone.",
    )
}

pub fn cling() -> Skill {
    Skill::new(
        "Cling",
        1,
        Reach::OneFoe,
        &[Effect::Special(Special::Cling)],
        "Drains an enemy every turn into the party.",
    )
}

pub fn static_burst() -> Skill {
    Skill::new(
        "Static",
        1,
        Reach::AllFoes,
        &[Effect::Inflict(Status::for_turns(
            StatusKind::Dreaming,
            ONE_TURN,
        ))],
        "The enemies act at random for a turn.",
    )
}

pub fn touch() -> Skill {
    Skill::new(
        "Touch",
        1,
        Reach::Everyone,
        &[Effect::Special(Special::Touch)],
        "An enemy's buffs become debuffs; an ally's debuffs become buffs.",
    )
}

pub fn amnesia() -> Skill {
    Skill::new(
        "Amnesia",
        1,
        Reach::OneFoe,
        &[Effect::Special(Special::Amnesia)],
        "An enemy forgets its buffs and its tell.",
    )
}

pub fn turn() -> Skill {
    Skill::new(
        "Turn",
        0,
        Reach::Me,
        &[Effect::Special(Special::Turn)],
        "Swaps its two sides.",
    )
}

pub fn shuffle() -> Skill {
    Skill::new(
        "Shuffle",
        1,
        Reach::AllFoes,
        &[Effect::Special(Special::Shuffle)],
        "The enemies' stats are dealt out again among them.",
    )
}

pub fn shed() -> Skill {
    Skill::new(
        "Shed",
        1,
        Reach::Me,
        &[Effect::Cleanse, Effect::Special(Special::Shed)],
        "A grey skin takes the next attack, and every debuff goes with it.",
    )
}

pub fn reverse() -> Skill {
    Skill::new(
        "Reverse",
        0,
        Reach::Everyone,
        &[Effect::Special(Special::Reverse)],
        "The next round runs backwards.",
    )
    .once()
}

pub fn invert() -> Skill {
    Skill::new(
        "Invert",
        1,
        Reach::Everyone,
        &[Effect::Special(Special::Invert)],
        "A fighter's buffs and debuffs trade places.",
    )
}

pub fn charge() -> Skill {
    Skill::new(
        "Charge",
        1,
        Reach::Me,
        &[Effect::Special(Special::Charge)],
        "Its next bite does double.",
    )
}

pub fn blast_off() -> Skill {
    Skill::new(
        "Blast-off",
        0,
        Reach::OneFoe,
        &[
            Effect::Hit {
                power: Power::bite_times(4),
                kind: DamageKind::Bite,
                hits: 1,
            },
            Effect::Special(Special::BlastOff),
        ],
        "A strike at four times its Bite, then it skips a turn.",
    )
    .once()
}

pub fn kick() -> Skill {
    Skill::new(
        "Kick",
        0,
        Reach::OneFoe,
        &[Effect::Special(Special::Kick)],
        "The target's next turn comes later.",
    )
}

pub fn stomp() -> Skill {
    Skill::new(
        "Stomp",
        1,
        Reach::OneFoe,
        &[
            Effect::bite(),
            Effect::Inflict(Status::for_turns(StatusKind::Held, ONE_TURN)),
        ],
        "Stomps and stuns.",
    )
}

pub fn alarm() -> Skill {
    Skill::new(
        "Alarm",
        1,
        Reach::AllFoes,
        &[Effect::Special(Special::Alarm)],
        "Every enemy bites it for a turn; the small ones are stunned.",
    )
}

pub fn moo() -> Skill {
    Skill::new(
        "Moo",
        0,
        Reach::Me,
        &[Effect::Special(Special::Moo)],
        "Every bite comes to the cow for a turn.",
    )
}

pub fn milk() -> Skill {
    Skill::new(
        "Milk",
        1,
        Reach::OneAlly,
        &[Effect::Special(Special::Milk)],
        "Gives an ally its milk.",
    )
}
