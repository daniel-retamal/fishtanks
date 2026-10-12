use std::collections::BTreeMap;

use crate::fishes::fish::Fish;
use crate::fishes::mutant::MutantTail;
use crate::fishes::mutations::{Mutatable, Mutation};

use crate::dungeon::kit::{Kit, carried_mutations};
use crate::dungeon::moves;
use crate::dungeon::traits::Trait;

pub const CONSPICUOUS_PERCENT: u32 = 20;
pub const SOFT_EYE_PERCENT: u32 = 3;
pub const CYCLOPS_CRIT: u32 = 5;
pub const PATCH_DODGE: u32 = 5;
pub const SHIMMER_PERCENT: u32 = 10;
pub const STROBE_PERCENT: u32 = 10;
pub const SLOW_SHIMMER_PERCENT: u32 = 25;
pub const WIDE_TAIL_SPEED: u32 = 2;
pub const WIDE_TAIL_DODGE: u32 = 5;
pub const SWAYING_TAIL_DODGE: u32 = 10;
pub const SWAYING_TAIL_SPEED: u32 = 1;
pub const MOUTH_BITE: u32 = 2;
pub const FEET_SPEED: u32 = 1;
pub const KEEL_ARMOUR: u32 = 1;
pub const ENGULFED_SPEED: u32 = 2;

pub struct KissAndCurse {
    pub gift: &'static str,
    pub quirk: &'static str,
}

pub fn row(mutation: Mutation) -> KissAndCurse {
    let (gift, quirk) = match mutation {
        Mutation::SizeIncrease => (
            "A segment of mass: more Vigor, for good.",
            "Bigger target: enemies pick it a little more often.",
        ),
        Mutation::EyeIncrease => (
            "More crit for every eye.",
            "An eye is a soft place: crits against it are likelier for every eye.",
        ),
        Mutation::Cyclops => (
            "Never misses, never dazzled; its crits hit triple.",
            "Its crit chance falls to the minimum.",
        ),
        Mutation::ColorPatch => (
            "More dodge for every patch.",
            "Every patch is a different flavour: gluttons nibble it.",
        ),
        Mutation::EyeColor => (
            "Glare: enemies of its eye's hue hit it softer.",
            "Enemies of the opposite hue notice it first.",
        ),
        Mutation::GlistenFast => (
            "Dazzles every turn.",
            "A strobe: its allies' first action each fight is less accurate.",
        ),
        Mutation::GlistenSlow => (
            "Every other turn it dazzles harder.",
            "On the off turns it does nothing.",
        ),
        Mutation::GlistenMode => (
            "Its shimmer dazzles every enemy, not one.",
            "At half strength.",
        ),
        Mutation::GlistenColor => (
            "A shimmer in the floor's palette dazzles twice as well.",
            "In any other palette, half as well.",
        ),
        Mutation::GlistenEnable => (
            "Dazzle: some attacks on it miss.",
            "Shiny: magpie-like enemies go for it.",
        ),
        Mutation::BodyColor => (
            "A body in the floor's palette dodges more and is picked last.",
            "Off the palette it stands out and is picked first.",
        ),
        Mutation::BodyVariant => (
            "The first hit each fight finds the wrong body and does half.",
            "The body it shifted into is ticklish: the second hit does more.",
        ),
        Mutation::TailVariant => (
            "Wide is faster, Swaying dodges, Curly holds another bubble.",
            "Wide is clumsy, Swaying is slow, Curly gets caught when fleeing.",
        ),
        Mutation::MouthVariant => (
            "Bites harder.",
            "Talks with its mouth full: its barks are louder.",
        ),
        Mutation::Telophase => (
            "Two heads: two actions a turn, each weaker.",
            "Two heads, two targets for crits.",
        ),
        Mutation::BackwardsTelophase => (
            "A head on each end: never surprised.",
            "Cannot flee: it goes both ways at once.",
        ),
        Mutation::Cytokinesis => (
            "Divide: the double splits mid-fight and the child joins.",
            "Both halves start at half vigor.",
        ),
        Mutation::Endocytosis => (
            "Collapse: a double eats its lighter half and heals fully.",
            "The lost identity goes to the graveyard, as always.",
        ),
        Mutation::Engulfment => (
            "Devour: swallow a weakened enemy; its mass joins the fish for good.",
            "A full fish is a slow fish for the rest of the fight.",
        ),
        Mutation::Alienation => (
            "Call Home: a UFO drops a companion who joins.",
            "Says GLORP VLERP when hit, which the Mothership hears.",
        ),
        Mutation::Strawberry => (
            "Sweet: more gold from everything it finds.",
            "The Candy Abyss's things want a bite.",
        ),
        Mutation::WakeColor => (
            "Its wake lingers: the next attack on it is inked or dazzled.",
            "The wake gives away a flee: fleeing with it costs a bite.",
        ),
        Mutation::NightOwl => (
            "Fiercer on dark floors.",
            "Asleep on bright ones unless it has had coffee.",
        ),
        Mutation::HelpedByGod => (
            "Eyes always open, another bubble, coffee works twice.",
            "The Inferno's contracts do not hold on it, good or bad.",
        ),
        Mutation::Heterochromia => (
            "Full damage to the ethereal and the invisible.",
            "Sees ghosts everywhere: now and then it bites one that is not there.",
        ),
        Mutation::Ear => (
            "Overhears: enemy tells come a turn earlier.",
            "Sirens and screeches hit it twice as hard.",
        ),
        Mutation::EarColor => (
            "Overhears the enemy's next move even without a tell.",
            "Gossip: it repeats what it hears aloud.",
        ),
        Mutation::Hydra => (
            "Bite hits once a head.",
            "The heads argue: now and then it bites an ally.",
        ),
        Mutation::Feet => (
            "Walks: fleeing always works, floor hazards miss it.",
            "Walking is slow in water.",
        ),
        Mutation::FeetColor => (
            "Silent steps: the party is never ambushed.",
            "None. Some things are just nice.",
        ),
        Mutation::Spikes => (
            "Thorns: whoever bites it is cut.",
            "Prickly: heals on it are weaker.",
        ),
        Mutation::Wings => (
            "Floor and sweep attacks miss it.",
            "Flying things hunt flyers.",
        ),
        Mutation::Tentacles => (
            "One more hit on Bite; a hit may hold its target.",
            "Holds onto allies too.",
        ),
        Mutation::Lure => (
            "Draws the small enemies away from the others.",
            "So it is the one they bite.",
        ),
        Mutation::Bill => (
            "A bite also hits a second enemy.",
            "The second hit is half.",
        ),
        Mutation::DorsalFin => (
            "Shark fin: the strongest enemy hits it softer.",
            "Bosses take it as a challenge and pick it.",
        ),
        Mutation::VentralFin => ("Keel: never stunned or held, a little armour.", "None."),
        Mutation::Lunar => (
            "Its skills wax through a fight.",
            "Short fights catch it new.",
        ),
        Mutation::Puff => (
            "Puff Up: half damage and triple thorns until its next turn.",
            "A puffed fish cannot bite or eat.",
        ),
        Mutation::Negative => (
            "Debuffs aimed at it turn into the matching buffs.",
            "Buffs turn into debuffs too. Heals still heal.",
        ),
        Mutation::Revert => ("Never worn: the Laboratory runs it on demand.", "-"),
    };
    KissAndCurse { gift, quirk }
}

pub fn fit(kit: &mut Kit, fish: &Fish) {
    let mut counts: BTreeMap<&'static str, (Mutation, u32)> = BTreeMap::new();
    for mutation in carried_mutations(fish) {
        counts.entry(mutation.token()).or_insert((mutation, 0)).1 += 1;
    }
    let mut shimmer: Option<Trait> = None;
    for (mutation, count) in counts.into_values() {
        wear(kit, fish, mutation, count, &mut shimmer);
    }
    if let Some(shimmer) = shimmer {
        kit.add(shimmer);
    }
}

fn shimmer_of(shimmer: &mut Option<Trait>) -> (&mut u32, &mut bool, &mut bool) {
    if shimmer.is_none() {
        *shimmer = Some(Trait::Shimmer {
            percent: SHIMMER_PERCENT,
            every_other: false,
            all_enemies: false,
            colour: None,
        });
    }
    match shimmer {
        Some(Trait::Shimmer {
            percent,
            every_other,
            all_enemies,
            ..
        }) => (percent, every_other, all_enemies),
        _ => unreachable!(),
    }
}

fn wear(kit: &mut Kit, fish: &Fish, mutation: Mutation, count: u32, shimmer: &mut Option<Trait>) {
    let mutant = fish.mutant.as_deref();
    match mutation {
        Mutation::SizeIncrease => kit.add(Trait::Conspicuous(CONSPICUOUS_PERCENT * count)),
        Mutation::EyeIncrease => kit.add(Trait::SoftEyes(
            SOFT_EYE_PERCENT * fish.eyes_a_side() as u32,
        )),
        Mutation::Cyclops => {
            kit.crit = CYCLOPS_CRIT;
            kit.add(Trait::Cyclops);
        }
        Mutation::ColorPatch => {
            let patches = mutant
                .map_or(count, |m| m.color_patches.len() as u32)
                .max(1);
            kit.dodge += PATCH_DODGE * patches;
            kit.add(Trait::Patches(patches));
        }
        Mutation::EyeColor => {
            if let Some(colour) = mutant.and_then(|m| m.eye_color) {
                kit.add(Trait::Glare(colour));
            }
        }
        Mutation::GlistenEnable => {
            shimmer_of(shimmer);
        }
        Mutation::GlistenFast => {
            let (percent, _, _) = shimmer_of(shimmer);
            *percent += STROBE_PERCENT;
        }
        Mutation::GlistenSlow => {
            let (percent, every_other, _) = shimmer_of(shimmer);
            *percent = SLOW_SHIMMER_PERCENT;
            *every_other = true;
        }
        Mutation::GlistenMode => {
            let (percent, _, all_enemies) = shimmer_of(shimmer);
            *percent /= 2;
            *all_enemies = true;
        }
        Mutation::GlistenColor => {
            shimmer_of(shimmer);
            if let (Some(Trait::Shimmer { colour, .. }), Some(m)) = (shimmer.as_mut(), mutant) {
                *colour = m.glistening_color;
            }
        }
        Mutation::BodyColor => kit.add(Trait::BodyColor(fish.color)),
        Mutation::BodyVariant => kit.add(Trait::Shifting),
        Mutation::TailVariant => wear_the_tail(kit, mutant.map(|m| m.tail_variant)),
        Mutation::MouthVariant => {
            kit.bite += MOUTH_BITE * count;
            kit.add(Trait::Loud);
        }
        Mutation::Telophase => kit.add(Trait::TwoHeads),
        Mutation::BackwardsTelophase => kit.add(Trait::Backwards),
        Mutation::Cytokinesis => kit.skills.push(moves::divide()),
        Mutation::Endocytosis => kit.skills.push(moves::collapse()),
        Mutation::Engulfment => kit.skills.push(moves::devour()),
        Mutation::Alienation => {
            kit.skills.push(moves::call_home());
            kit.add(Trait::Alien);
        }
        Mutation::Strawberry => kit.add(Trait::Sweet(count)),
        Mutation::WakeColor => {
            let dark = mutant
                .and_then(|m| m.wake_color)
                .is_some_and(crate::dungeon::tables::is_dark);
            kit.add(Trait::Wake { dark });
        }
        Mutation::NightOwl => kit.add(Trait::NightOwl),
        Mutation::HelpedByGod => {
            kit.max_mp += 1;
            kit.add(Trait::HelpedByGod);
        }
        Mutation::Heterochromia => {
            kit.add(Trait::SeesGhosts);
            kit.add(Trait::PhantomBites);
        }
        Mutation::Ear => kit.add(Trait::Ears),
        Mutation::EarColor => kit.add(Trait::Gossip),
        Mutation::Hydra => {
            let heads = u8::try_from(fish.hydra_count().max(1) + 1).unwrap_or(u8::MAX);
            kit.add(Trait::Hydra(heads));
        }
        Mutation::Feet => {
            kit.speed = kit.speed.saturating_sub(FEET_SPEED).max(1);
            kit.add(Trait::Walks);
        }
        Mutation::FeetColor => kit.add(Trait::SilentSteps),
        Mutation::Spikes => kit.add(Trait::Spikes),
        Mutation::Wings => kit.add(Trait::Wings),
        Mutation::Tentacles => kit.add(Trait::Tentacles),
        Mutation::Lure => kit.add(Trait::Lure),
        Mutation::Bill => kit.add(Trait::Bill),
        Mutation::DorsalFin => kit.add(Trait::SharkFin),
        Mutation::VentralFin => {
            kit.armour += KEEL_ARMOUR;
            kit.add(Trait::Keel);
        }
        Mutation::Lunar => kit.add(Trait::Lunar),
        Mutation::Puff => kit.skills.push(moves::puff_up()),
        Mutation::Negative => kit.add(Trait::Negative),
        Mutation::Revert => {}
    }
}

fn wear_the_tail(kit: &mut Kit, tail: Option<MutantTail>) {
    match tail {
        Some(MutantTail::Wide) => {
            kit.speed += WIDE_TAIL_SPEED;
            kit.dodge = kit.dodge.saturating_sub(WIDE_TAIL_DODGE);
        }
        Some(MutantTail::Swaying) => {
            kit.dodge += SWAYING_TAIL_DODGE;
            kit.speed = kit.speed.saturating_sub(SWAYING_TAIL_SPEED).max(1);
        }
        Some(MutantTail::Curly) => {
            kit.max_mp += 1;
            kit.add(Trait::CaughtTail);
        }
        Some(MutantTail::Narrow | MutantTail::Bare) | None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fishes::mutations::{MutationOutcome, apply_mutation};
    use crate::fishes::species::FishSpecies;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    #[test]
    fn every_mutation_has_a_gift_and_a_quirk() {
        for &mutation in Mutation::ALL {
            let row = row(mutation);
            assert!(!row.gift.is_empty(), "{mutation:?} has no gift");
            assert!(!row.quirk.is_empty(), "{mutation:?} has no quirk");
        }
    }

    #[test]
    fn every_mutation_a_fish_can_wear_changes_its_kit() {
        for &mutation in Mutation::ALL {
            if mutation == Mutation::Revert {
                continue;
            }
            let mut rng = SmallRng::seed_from_u64(11);
            let mut fish = Fish::new(FishSpecies::Merluza, "Probe".into(), 0.0, 0.0, &mut rng);
            let before = Kit::of(&fish);
            if apply_mutation(&mut fish, mutation, &mut rng) != MutationOutcome::Applied
                || !fish
                    .mutations
                    .as_ref()
                    .is_some_and(|record| record.has(mutation))
            {
                let record = fish.mutations.get_or_insert_with(Default::default);
                record.history.push(mutation.token().to_string());
            }
            let after = Kit::of(&fish);
            assert_ne!(before, after, "{mutation:?} changes nothing in the Deep");
        }
    }
}
