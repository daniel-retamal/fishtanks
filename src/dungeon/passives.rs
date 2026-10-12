use rand::RngExt;
use ratatui::style::Color;

use crate::colors::rgb_of;
use crate::fishes::toy::Line;
use crate::tank::ECHO_FADE;

use super::battle::{BITE, Battle, HALF, MAX_FIGHTERS, ROLL_THRESHOLD_PERCENT, ROUND_TICKS};
use super::beat::Tone;
use super::effect::{DamageKind, Reach, Status, StatusKind};
use super::fighter::Side;
use super::traits::Trait;

pub const SCHOOL_BITE: u32 = 1;
pub const SCHOOL_DODGE: u32 = 5;
pub const SHOAL_COUNTS_AS: u32 = 3;
pub const CROWN_BITE: u32 = 1;
pub const ROYAL_AURA_BITE: u32 = 1;
pub const MECHA_ARMOUR: u32 = 2;
pub const RACER_SPEED: u32 = 2;
pub const DRAGON_FLOOR: u32 = 7;
pub const DRAGON_TIMES: u32 = 2;
pub const COMBINE_COUNT: usize = 4;
pub const TWO_HEADS_PERCENT: u32 = 60;
pub const AFTERBURNER_EVERY: u32 = 3;
pub const PRIDE_PERCENT: u32 = 120;
pub const WIND_UP_PERCENT: u32 = 150;
pub const WIND_UP_TURNS: u32 = 3;
pub const STARE_PERCENT: u32 = 80;
pub const GLARE_PERCENT: u32 = 80;
pub const SHARK_FIN_PERCENT: u32 = 85;
pub const SHIFT_FIRST_PERCENT: u32 = 50;
pub const SHIFT_SECOND_PERCENT: u32 = 125;
pub const LUNAR_NEW_PERCENT: u32 = 50;
pub const LUNAR_FULL_PERCENT: u32 = 200;
pub const LUNAR_FULL_ROUND: u32 = 6;
pub const PALETTE_DODGE: u32 = 15;
pub const PHANTOM_ONE_IN: u32 = 20;
pub const HYDRA_ARGUES_ONE_IN: u32 = 8;
pub const FALSE_EYE_ONE_IN: u32 = 3;
pub const HALF_THERE_ONE_IN: u32 = 2;
pub const REFLECTION_BITES_ONE_IN: u32 = 4;
pub const GEIGER_BITE: u32 = 2;
pub const GOLDEN_TOY_GOLD_DIVISOR: u64 = 8;
pub const LUST_PERCENT: u32 = 125;
pub const FOOD_TIMES: u32 = 2;
pub const SWEET_PERCENT: f64 = 25.0;
pub const SIN_GOLD_PERCENT: u32 = 25;
pub const FINE_PRINT_PERCENT: u32 = 10;
pub const DEVILS_COIN_PERCENT: u32 = 25;
pub const SECOND_WIND_PERCENT: u32 = 25;
pub const FORK_PERCENT: u32 = 50;
pub const WRATH_STEP_PERCENT: u32 = 10;
pub const HUNTER_TIMES: u32 = 2;
pub const BODY_COLOUR_TIMES: u32 = 2;
pub const PATCH_PERCENT: u32 = 25;
pub const FAMILIES: u32 = 6;
pub const GREY_SPREAD: i32 = 24;
const PERCENT: u32 = 100;

pub fn family(colour: Color) -> Option<u32> {
    let (r, g, b) = rgb_of(colour);
    let (r, g, b) = (i32::from(r), i32::from(g), i32::from(b));
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let spread = max - min;
    if spread < GREY_SPREAD {
        return None;
    }
    let hue = if max == r {
        (60 * (g - b) / spread).rem_euclid(360)
    } else if max == g {
        60 * (b - r) / spread + 120
    } else {
        60 * (r - g) / spread + 240
    };
    Some(((hue + 30) / 60) as u32 % FAMILIES)
}

pub fn opposite(one: u32, other: u32) -> bool {
    (one + FAMILIES / 2) % FAMILIES == other
}

fn has(battle: &Battle, index: usize, wanted: Trait) -> bool {
    battle.fighters[index].has(wanted)
}

fn party(battle: &Battle) -> Vec<usize> {
    battle.side(Side::Party).collect()
}

pub fn assemble(battle: &mut Battle) {
    let members = party(battle);
    let school: u32 = members
        .iter()
        .map(|&i| {
            match (
                has(battle, i, Trait::Schooling),
                has(battle, i, Trait::Shoal),
            ) {
                (_, true) => SHOAL_COUNTS_AS,
                (true, false) => 1,
                _ => 0,
            }
        })
        .sum();
    let crowns = members
        .iter()
        .filter(|&&i| has(battle, i, Trait::Crown))
        .count() as u32;
    let royals = members
        .iter()
        .filter(|&&i| has(battle, i, Trait::Signature(Line::Royals)))
        .count() as u32;
    let counters = members
        .iter()
        .filter(|&&i| has(battle, i, Trait::ShoalCounter))
        .count() as u32;
    let size = members.len() as u32;
    let trumpets = members.iter().any(|&i| has(battle, i, Trait::Trumpet));
    let floor = battle.setup.floor;
    let palette = battle.palette.clone();
    for &i in &members {
        let f = &mut battle.fighters[i];
        if f.has(Trait::Schooling) && school > 1 {
            f.bite += SCHOOL_BITE * (school - 1);
            f.dodge += SCHOOL_DODGE * (school - 1);
        }
        f.bite += CROWN_BITE * crowns + ROYAL_AURA_BITE * royals + counters * size;
        if f.has(Trait::Signature(Line::Mecha)) {
            f.armour += MECHA_ARMOUR;
        }
        if f.has(Trait::Signature(Line::Racers)) {
            f.speed += RACER_SPEED;
        }
        if f.has(Trait::Upstream) {
            f.bite += floor.saturating_sub(1);
            if floor >= DRAGON_FLOOR {
                f.max_hp *= DRAGON_TIMES;
                f.hp *= DRAGON_TIMES;
                f.traits.push(Trait::Signature(Line::Kaiju));
            }
        }
        if trumpets {
            f.gain_bubbles(1);
        }
        let colours: Vec<Color> = f
            .traits
            .iter()
            .filter_map(|t| match t {
                Trait::FavouriteColour(colour) => Some(*colour),
                _ => None,
            })
            .collect();
        if colours
            .iter()
            .any(|&colour| family(colour).is_some_and(|hue| palette.contains(&hue)))
        {
            f.gain_bubbles(1);
        }
        if in_palette(f.traits.iter(), &palette) {
            f.dodge += PALETTE_DODGE;
        }
        if f.has(Trait::AlwaysLast) || f.has(Trait::SlothSin) {
            f.ready_at = ROUND_TICKS - 1;
        }
    }
    envy(battle, &members);
    combine(battle, &members);
}

fn in_palette<'a>(traits: impl Iterator<Item = &'a Trait>, palette: &[u32]) -> bool {
    let mut traits = traits.peekable();
    let mut camouflaged = false;
    let mut coloured = false;
    for t in traits.by_ref() {
        match t {
            Trait::Camouflage => camouflaged = true,
            Trait::BodyColor(colour) => {
                coloured = family(*colour).is_some_and(|hue| palette.contains(&hue));
            }
            _ => {}
        }
    }
    camouflaged || coloured
}

fn envy(battle: &mut Battle, members: &[usize]) {
    for &i in members {
        if !has(battle, i, Trait::Envy) {
            continue;
        }
        let model = members
            .iter()
            .copied()
            .filter(|&j| j != i)
            .max_by_key(|&j| battle.fighters[j].bite);
        let Some(model) = model else {
            continue;
        };
        let (bite, speed, skills) = {
            let m = &battle.fighters[model];
            (m.bite, m.speed, m.skills.clone())
        };
        let name = battle.fighters[model].name.clone();
        let f = &mut battle.fighters[i];
        f.bite = f.bite.max(bite);
        f.speed = f.speed.max(speed);
        f.skills.extend(skills);
        let own = f.name.clone();
        battle.note(format!("{own} wears {name}'s look."), Tone::Status);
    }
}

fn combine(battle: &mut Battle, members: &[usize]) {
    for line in Line::ALL {
        let four: Vec<usize> = members
            .iter()
            .copied()
            .filter(|&i| has(battle, i, Trait::Signature(line)))
            .take(COMBINE_COUNT)
            .collect();
        if four.len() < COMBINE_COUNT {
            continue;
        }
        let head = four[0];
        for &other in &four[1..] {
            let (hp, max_hp, skills) = {
                let o = &battle.fighters[other];
                (o.hp, o.max_hp, o.skills.clone())
            };
            let f = &mut battle.fighters[head];
            f.hp += hp;
            f.max_hp += max_hp;
            f.skills.extend(skills);
            battle.fighters[other].inflict(Status::for_turns(
                StatusKind::Away,
                super::specials::FOREVER,
            ));
        }
        let secret = line.secret().name();
        battle.note(
            format!("Four of the {} line combine into {secret}.", line.name()),
            Tone::Gold,
        );
    }
}

pub fn outgoing(battle: &Battle, actor: usize, target: usize) -> u32 {
    let a = &battle.fighters[actor];
    let mut percent = PERCENT;
    if a.has(Trait::TwoHeads) {
        percent = percent * TWO_HEADS_PERCENT / PERCENT;
    }
    if (a.has(Trait::Pride) || a.has(Trait::PrideSin)) && a.hp >= a.max_hp {
        percent = percent * PRIDE_PERCENT / PERCENT;
    }
    if a.has(Trait::WindUp) && a.turns <= WIND_UP_TURNS {
        percent = percent * WIND_UP_PERCENT / PERCENT;
    }
    if a.has(Trait::Lunar) && battle.acting != Some(BITE) {
        percent = match battle.round {
            1 => percent * LUNAR_NEW_PERCENT / PERCENT,
            round if round >= LUNAR_FULL_ROUND => percent * LUNAR_FULL_PERCENT / PERCENT,
            _ => percent,
        };
    }
    if a.has(Trait::Unwatched) {
        percent *= 1 + a.gathered;
    }
    let _ = target;
    percent
}

pub fn incoming(battle: &Battle, actor: usize, target: usize) -> u32 {
    let t = &battle.fighters[target];
    if t.side != Side::Party {
        return PERCENT;
    }
    let a = &battle.fighters[actor];
    let mut percent = PERCENT;
    if battle
        .standing(Side::Party)
        .iter()
        .any(|&i| has(battle, i, Trait::Stare))
    {
        percent = percent * STARE_PERCENT / PERCENT;
    }
    if t.has(Trait::Shifting) {
        percent = match t.hits_taken {
            0 => percent * SHIFT_FIRST_PERCENT / PERCENT,
            1 => percent * SHIFT_SECOND_PERCENT / PERCENT,
            _ => percent,
        };
    }
    let attacker_family = a
        .spec()
        .and_then(|spec| spec.main_colour())
        .and_then(family);
    for tr in &t.traits {
        if let Trait::Glare(eye) = tr
            && family(*eye).is_some()
            && family(*eye) == attacker_family
        {
            percent = percent * GLARE_PERCENT / PERCENT;
        }
    }
    if t.has(Trait::SharkFin) {
        let strongest = battle
            .standing(a.side)
            .into_iter()
            .max_by_key(|&i| battle.fighters[i].effective_bite());
        if strongest == Some(actor) {
            percent = percent * SHARK_FIN_PERCENT / PERCENT;
        }
    }
    percent
}

pub fn ignores_armour(battle: &Battle, actor: usize) -> bool {
    has(battle, actor, Trait::Drill)
}

pub fn misses(battle: &mut Battle, actor: usize, target: usize, kind: DamageKind) -> bool {
    if battle.fighters[target].side != Side::Party {
        return false;
    }
    let bitten = kind == DamageKind::Bite;
    let first = battle.fighters[target].hits_taken == 0;
    let t = &battle.fighters[target];
    let name = t.name.clone();
    if first && (t.has(Trait::Decoy) || t.has(Trait::Reflection)) {
        battle.note(
            format!("The first blow finds {name}'s double instead."),
            Tone::Quiet,
        );
        battle.fighters[target].hits_taken += 1;
        return true;
    }
    if first && t.traits.iter().any(|tr| matches!(tr, Trait::Wake { .. })) {
        battle.note(format!("{name}'s dark wake hides it."), Tone::Quiet);
        battle.fighters[target].hits_taken += 1;
        return true;
    }
    if bitten && t.has(Trait::FalseEye) && battle.rng.random_range(0..FALSE_EYE_ONE_IN) == 0 {
        battle.note(
            format!("The bite goes for {name}'s false eye."),
            Tone::Quiet,
        );
        return true;
    }
    if bitten && t.has(Trait::HalfThere) && battle.rng.random_range(0..HALF_THERE_ONE_IN) == 0 {
        battle.note(format!("{name} is not there to be bitten."), Tone::Quiet);
        return true;
    }
    if bitten && t.has(Trait::Hide) && t.hid_in_round != battle.round {
        battle.fighters[target].hid_in_round = battle.round;
        battle.note(format!("{name} hides from the first bite."), Tone::Quiet);
        return true;
    }
    if t.has_status(StatusKind::Dazzled) {
        battle.fighters[target].clear_status(StatusKind::Dazzled);
        battle.note(format!("The bite slips off {name}."), Tone::Quiet);
        return true;
    }
    let _ = actor;
    false
}

pub fn sweeps_miss(battle: &Battle, reach: Reach, target: usize) -> bool {
    let sweep = matches!(
        reach,
        Reach::AllFoes | Reach::Everyone | Reach::RandomFoes(_)
    );
    let t = &battle.fighters[target];
    sweep
        && t.side == Side::Party
        && (t.has(Trait::Wings) || t.has(Trait::Glide) || t.has(Trait::Hover))
}

pub fn crits(battle: &Battle, actor: usize, target: usize) -> bool {
    let a = &battle.fighters[actor];
    if a.side != Side::Party || battle.acting != Some(BITE) {
        return false;
    }
    let first_bite = a.has(Trait::Camouflage) && !a.once.contains(&Trait::Camouflage);
    let blush = a.has(Trait::Hide) && battle.standing(Side::Party).len() == 1;
    let _ = target;
    first_bite || blush
}

pub fn crit_times(battle: &Battle, target: usize) -> Option<u32> {
    has(battle, target, Trait::Doomed).then_some(3)
}

pub fn wrath_bite(battle: &Battle, actor: usize) -> u32 {
    let a = &battle.fighters[actor];
    if !a.has(Trait::WrathSin) {
        return 0;
    }
    let lost = a.max_hp.saturating_sub(a.hp) * PERCENT / a.max_hp.max(1);
    lost / WRATH_STEP_PERCENT
}

pub fn ledger(battle: &Battle, actor: usize) -> u32 {
    if !has(battle, actor, Trait::LedgerNerve) {
        return 0;
    }
    (battle.purse.max(1) as f64).log10().floor() as u32
}

pub fn after_hit(battle: &mut Battle, actor: usize, target: usize, damage: u32) {
    let attacker = &battle.fighters[actor];
    if attacker.side == Side::Party && attacker.has(Trait::Camouflage) {
        battle.fighters[actor].once.push(Trait::Camouflage);
    }
    if battle.fighters[actor].has(Trait::GoldenToy) && battle.fighters[actor].side == Side::Party {
        let gold = super::floor::gold_share(battle.setup.floor, battle.setup.difficulty)
            / GOLDEN_TOY_GOLD_DIVISOR;
        battle.haul += gold.max(1);
    }
    let t = &battle.fighters[target];
    if t.side != Side::Party {
        return;
    }
    if t.has(Trait::Rusting) {
        battle.junk_found += 1;
    }
    bark(battle, target);
    let big = damage * PERCENT >= battle.fighters[target].max_hp * ROLL_THRESHOLD_PERCENT;
    if big && battle.fighters[target].has(Trait::Scatter) {
        battle.fighters[target].inflict(Status::for_turns(StatusKind::Dazzled, 1));
        let name = battle.fighters[target].name.clone();
        battle.note(format!("{name}'s bones scatter."), Tone::Status);
    }
}

pub fn bark(battle: &mut Battle, target: usize) {
    let t = &battle.fighters[target];
    let voice = t.traits.iter().find_map(|tr| match tr {
        Trait::Voice(voice) => Some(*voice),
        _ => None,
    });
    let alien = t.has(Trait::Alien);
    let loud = t.has(Trait::Loud);
    let name = t.name.clone();
    let words = match (alien, voice) {
        (true, _) => Some(crate::tank::ALIEN_TONGUE.to_string()),
        (false, Some(voice)) => Some(format!("{voice}!")),
        (false, None) => None,
    };
    if let Some(words) = words {
        let words = if loud { words.to_uppercase() } else { words };
        battle.note(format!("{name}: {words}"), Tone::Quiet);
    }
}

pub fn after_bitten(battle: &mut Battle, actor: usize, target: usize) {
    let biter = &battle.fighters[actor];
    if biter.side == Side::Party {
        if let Some(percent) = biter.trait_value(|t| match t {
            Trait::Sting(p) => Some(*p),
            _ => None,
        }) && battle.rng.random_range(0..PERCENT) < percent
        {
            battle.inflict(actor, target, Status::for_turns(StatusKind::Held, 1));
        }
        return;
    }
    let t = &battle.fighters[target];
    if t.has(Trait::Nooo) {
        let wait = battle.wait_of(actor);
        battle.fighters[actor].ready_at += wait;
        let name = battle.fighters[actor].name.clone();
        battle.note(
            format!("{name} spits it out and loses its next turn."),
            Tone::Status,
        );
    }
    let venom = battle.standing(Side::Party).into_iter().find_map(|i| {
        battle.fighters[i].trait_value(|t| match t {
            Trait::Ambush(n) => Some(*n),
            _ => None,
        })
    });
    if let Some(venom) = venom
        && battle.ambush_round != battle.round
    {
        battle.ambush_round = battle.round;
        let name = battle.fighters[actor].name.clone();
        battle.note(
            format!("A stonefish was waiting: {name} takes venom {venom}."),
            Tone::Hit,
        );
        battle.lose_hp(actor, venom, None);
    }
}

pub fn resists(battle: &Battle, target: usize, status: Status) -> bool {
    let t = &battle.fighters[target];
    if t.side != Side::Party {
        return false;
    }
    let see_through = t.trait_value(|tr| match tr {
        Trait::SeeThrough(p) => Some(*p),
        _ => None,
    });
    match status.kind {
        StatusKind::Blind | StatusKind::Dazzled if see_through.is_some() => true,
        StatusKind::Dreaming | StatusKind::Astray => t.has(Trait::Sealed),
        StatusKind::Bleed | StatusKind::Poison | StatusKind::Rot => t.has(Trait::Construct),
        StatusKind::Contract => t.has(Trait::HelpedByGod),
        StatusKind::Charmed => t.has_status(StatusKind::Steady),
        _ => false,
    }
}

pub fn beam_fails(battle: &mut Battle, target: usize) -> bool {
    let t = &battle.fighters[target];
    if t.has_status(StatusKind::Puffed) || t.has(Trait::Signature(Line::Space)) {
        return true;
    }
    let Some(percent) = t.trait_value(|tr| match tr {
        Trait::SeeThrough(p) => Some(*p),
        _ => None,
    }) else {
        return false;
    };
    battle.rng.random_range(0..PERCENT) < percent
}

pub fn envies(battle: &mut Battle, target: usize, status: Status) {
    if status.kind.is_debuff() || battle.fighters[target].side != Side::Party {
        return;
    }
    let envious: Vec<usize> = battle
        .standing(Side::Party)
        .into_iter()
        .filter(|&i| {
            i != target
                && has(battle, i, Trait::EnvySin)
                && !battle.fighters[i].once.contains(&Trait::EnvySin)
        })
        .collect();
    for i in envious {
        battle.fighters[i].inflict(status);
        battle.fighters[i].once.push(Trait::EnvySin);
        let name = battle.fighters[i].name.clone();
        battle.note(
            format!("{name} wants that too, and takes it."),
            Tone::Status,
        );
    }
}

pub fn mutation_immune(battle: &Battle, target: usize) -> bool {
    has(battle, target, Trait::Construct) || has(battle, target, Trait::Sealed)
}

pub fn mutated(battle: &mut Battle) {
    for i in battle.standing(Side::Party) {
        if has(battle, i, Trait::Geiger) {
            battle.fighters[i].bite += GEIGER_BITE;
        }
    }
}

pub fn saved_from_falling(battle: &mut Battle, target: usize) -> bool {
    let name = battle.fighters[target].name.clone();
    let saves = [
        (
            Trait::SecondWind,
            SECOND_WIND_PERCENT,
            "catches its second wind",
        ),
        (Trait::Forked, FORK_PERCENT, "forks, and the fork stands"),
        (Trait::Scatter, FORK_PERCENT, "reassembles"),
    ];
    for (save, percent, text) in saves {
        let f = &battle.fighters[target];
        if f.has(save) && !f.once.contains(&save) {
            let f = &mut battle.fighters[target];
            f.once.push(save);
            f.hp = (f.max_hp * percent / PERCENT).max(1);
            battle.note(format!("{name} {text}."), Tone::Heal);
            return true;
        }
    }
    if has(battle, target, Trait::Segments) && !battle.fighters[target].came_back {
        battle.fighters[target].came_back = true;
        split_the_worm(battle, target);
        return true;
    }
    false
}

fn split_the_worm(battle: &mut Battle, target: usize) {
    let Some(member) = battle.fighters[target].member() else {
        return;
    };
    if battle.fighters.len() >= MAX_FIGHTERS {
        return;
    }
    let mut child = battle.party[member].clone();
    child.name = format!("{} II", child.name);
    child.deep = None;
    let index = battle.party.len();
    let kit = super::kit::Kit::of(&child);
    battle.party.push(child);
    let mut worm = super::fighter::Fighter::of_kit(&kit, index);
    worm.hp = (worm.max_hp / HALF).max(1);
    worm.came_back = true;
    worm.ready_at = battle.clock + battle.wait_of(target);
    let f = &mut battle.fighters[target];
    f.hp = (f.max_hp / HALF).max(1);
    let name = f.name.clone();
    battle.fighters.push(worm);
    battle.note(format!("{name} splits into two worms."), Tone::Status);
}

pub fn startle(battle: &mut Battle) {
    for i in battle.standing(Side::Party) {
        if has(battle, i, Trait::Startle) {
            battle.fighters[i].ready_at = battle.clock;
        }
    }
}

pub fn escapes(battle: &mut Battle, target: usize) -> bool {
    let f = &battle.fighters[target];
    if !f.has(Trait::Escape) || f.once.contains(&Trait::Escape) {
        return false;
    }
    let name = f.name.clone();
    let f = &mut battle.fighters[target];
    f.once.push(Trait::Escape);
    f.belly_up = false;
    f.inflict(Status::for_turns(
        StatusKind::Away,
        super::specials::FOREVER,
    ));
    battle.note(
        format!("{name} squirts ink and escapes home."),
        Tone::Status,
    );
    true
}

pub fn round_start(battle: &mut Battle) {
    let members: Vec<usize> = battle.standing(Side::Party);
    for &i in &members {
        if has(battle, i, Trait::Unstable) {
            battle.mutate(i, 1);
        }
        let stacks = battle.fighters[i].trait_value(|t| match t {
            Trait::Blessed(n) => Some(*n),
            _ => None,
        });
        if let Some(stacks) = stacks
            && battle.round.is_multiple_of(super::battle::BLESSING_ROUNDS)
        {
            for _ in 0..stacks {
                super::specials::perform(
                    battle,
                    i,
                    super::effect::Special::Miracle,
                    &[],
                    &super::moves::miracle(),
                );
            }
        }
        if has(battle, i, Trait::TheLap) {
            lap(battle, i);
        }
    }
}

fn lap(battle: &mut Battle, ouroboros: usize) {
    let turns = battle.fighters[ouroboros].turns;
    if !turns.is_multiple_of(super::battle::LAP_TURNS) {
        return;
    }
    let snapshot = std::mem::take(&mut battle.fighters[ouroboros].lap);
    for (i, hp) in snapshot.iter().copied().enumerate() {
        let Some(f) = battle.fighters.get_mut(i) else {
            continue;
        };
        if f.side == Side::Party && f.standing() && f.hp < hp {
            f.hp = hp.min(f.max_hp);
        }
    }
    if !snapshot.is_empty() {
        battle.note("The lap closes: the party is as it was.", Tone::Heal);
    }
    battle.fighters[ouroboros].lap = battle.fighters.iter().map(|f| f.hp).collect();
}

pub fn acts_again(battle: &mut Battle, actor: usize) -> bool {
    let f = &battle.fighters[actor];
    if f.side != Side::Party || !f.standing() {
        return false;
    }
    if f.has(Trait::TwoHeads) && f.extra == 0 {
        battle.fighters[actor].extra = 1;
        return true;
    }
    if f.has(Trait::Afterburner) && f.turns.is_multiple_of(AFTERBURNER_EVERY) && f.extra == 0 {
        battle.fighters[actor].extra = 1;
        let name = battle.fighters[actor].name.clone();
        battle.note(
            format!("{name}'s afterburner gives it a free turn."),
            Tone::Status,
        );
        return true;
    }
    false
}

pub fn turn_begins(battle: &mut Battle, actor: usize) -> bool {
    let f = &mut battle.fighters[actor];
    f.extra = 0;
    if f.has(Trait::WindUp) && f.turns == WIND_UP_TURNS + 1 {
        let name = f.name.clone();
        battle.note(format!("{name} winds itself up again."), Tone::Quiet);
        return false;
    }
    true
}

pub fn guarded(battle: &mut Battle, actor: usize) {
    if has(battle, actor, Trait::Unwatched) {
        battle.fighters[actor].gathered += 1;
        let name = battle.fighters[actor].name.clone();
        battle.note(format!("{name} gathers itself."), Tone::Quiet);
    }
}

pub fn struck(battle: &mut Battle, actor: usize) {
    if has(battle, actor, Trait::Unwatched) {
        battle.fighters[actor].gathered = 0;
    }
}

pub fn hydra_argues(battle: &mut Battle, actor: usize) -> Option<usize> {
    let heads = battle.fighters[actor]
        .traits
        .iter()
        .any(|t| matches!(t, Trait::Hydra(_)));
    if !heads || battle.fighters[actor].side != Side::Party {
        return None;
    }
    if battle.rng.random_range(0..HYDRA_ARGUES_ONE_IN) != 0 {
        return None;
    }
    let allies: Vec<usize> = battle
        .standing(Side::Party)
        .into_iter()
        .filter(|&i| i != actor)
        .collect();
    let ally = *allies.get(battle.rng.random_range(0..allies.len().max(1)))?;
    let name = battle.fighters[actor].name.clone();
    battle.note(format!("{name}'s heads argue."), Tone::Quiet);
    Some(ally)
}

pub fn bites_a_ghost(battle: &mut Battle, actor: usize) -> bool {
    if !has(battle, actor, Trait::PhantomBites) || battle.rng.random_range(0..PHANTOM_ONE_IN) != 0 {
        return false;
    }
    let name = battle.fighters[actor].name.clone();
    battle.note(
        format!("{name} bites at a ghost that is not there."),
        Tone::Quiet,
    );
    true
}

pub fn reflection_bites(battle: &mut Battle, actor: usize) -> Option<usize> {
    if !has(battle, actor, Trait::Reflection)
        || battle.rng.random_range(0..REFLECTION_BITES_ONE_IN) != 0
    {
        return None;
    }
    battle.pick_target(actor)
}

pub fn echo(battle: &mut Battle, actor: usize) {
    let Some(skill) = battle.last_party_skill.clone() else {
        return;
    };
    let parrots: Vec<usize> = battle
        .standing(Side::Party)
        .into_iter()
        .filter(|&i| i != actor && has(battle, i, Trait::Parrot))
        .collect();
    for parrot in parrots {
        let free = super::skill::Skill {
            price: super::skill::Price::Free,
            ..skill.clone()
        };
        let name = battle.fighters[parrot].name.clone();
        battle.note(format!("{name} echoes it."), Tone::Status);
        battle.strength = ECHO_PERCENT;
        let target = match free.reach.picks_a_foe() {
            true => battle.pick_target(parrot),
            false => None,
        };
        battle.use_skill(parrot, &free, target);
        battle.strength = PERCENT;
    }
}

pub const ECHO_PERCENT: u32 = (ECHO_FADE * 100.0) as u32;

pub fn gold_percent(battle: &Battle, finder: usize) -> u32 {
    let f = &battle.fighters[finder];
    let mut percent = PERCENT as f64;
    for t in &f.traits {
        match t {
            Trait::Sweet(stacks) | Trait::Greedy(stacks) => {
                percent *= 1.0 + SWEET_PERCENT / f64::from(PERCENT) * f64::from(1 + stacks).log2();
            }
            Trait::GreedSin => {
                percent *= f64::from(PERCENT + SIN_GOLD_PERCENT) / f64::from(PERCENT)
            }
            Trait::DevilsCoin => {
                percent *= f64::from(PERCENT + DEVILS_COIN_PERCENT) / f64::from(PERCENT)
            }
            Trait::FinePrint => {
                percent *= f64::from(PERCENT - FINE_PRINT_PERCENT) / f64::from(PERCENT)
            }
            _ => {}
        }
    }
    percent.round() as u32
}

pub fn heal_percent(battle: &Battle, healer: Option<usize>, target: usize, food: bool) -> u32 {
    let mut percent = PERCENT;
    if let Some(healer) = healer
        && has(battle, healer, Trait::LustSin)
    {
        percent = percent * LUST_PERCENT / PERCENT;
    }
    if food && (has(battle, target, Trait::SweetTooth) || has(battle, target, Trait::GluttonySin)) {
        percent *= FOOD_TIMES;
    }
    if food && has(battle, target, Trait::Construct) {
        return 0;
    }
    percent
}

pub fn holy_hurts(battle: &Battle, healer: Option<usize>, target: usize) -> bool {
    let holy = healer.is_some_and(|h| has(battle, h, Trait::Holy));
    holy && has(battle, target, Trait::Undead)
}

pub fn flee_toll(battle: &mut Battle) {
    let caught: Vec<usize> = battle
        .standing(Side::Party)
        .into_iter()
        .filter(|&i| {
            has(battle, i, Trait::CaughtTail)
                || battle.fighters[i]
                    .traits
                    .iter()
                    .any(|t| matches!(t, Trait::Wake { .. }))
        })
        .collect();
    let Some(biter) = battle
        .standing(Side::Foes)
        .into_iter()
        .max_by_key(|&i| battle.fighters[i].effective_bite())
    else {
        return;
    };
    for fish in caught {
        let name = battle.fighters[fish].name.clone();
        battle.note(format!("{name} is caught on the way out."), Tone::Hurt);
        battle.bite(biter, fish);
    }
}

pub fn tells_early(battle: &Battle) -> bool {
    battle
        .standing(Side::Party)
        .iter()
        .any(|&i| has(battle, i, Trait::Ears) || has(battle, i, Trait::Glyph))
}

pub fn reflex(battle: &mut Battle) {
    let quick = battle
        .standing(Side::Party)
        .iter()
        .any(|&i| has(battle, i, Trait::Reflex));
    if !quick {
        return;
    }
    let Some(weakest) = battle
        .standing(Side::Party)
        .into_iter()
        .min_by_key(|&i| battle.fighters[i].hp_percent())
    else {
        return;
    };
    battle.fighters[weakest].guarding = true;
    let name = battle.fighters[weakest].name.clone();
    battle.note(format!("A reflex: {name} braces."), Tone::Status);
}

pub fn target_weight(battle: &Battle, attacker: usize, target: usize) -> u32 {
    let a = &battle.fighters[attacker];
    let t = &battle.fighters[target];
    let mut percent = PERCENT;
    if a.has(Trait::Hunter) && (t.has(Trait::Wings) || t.has(Trait::Glide)) {
        percent *= HUNTER_TIMES;
    }
    let palette = &battle.palette;
    for tr in &t.traits {
        match tr {
            Trait::BodyColor(colour) => {
                match family(*colour).is_some_and(|hue| palette.contains(&hue)) {
                    true => percent /= BODY_COLOUR_TIMES,
                    false => percent *= BODY_COLOUR_TIMES,
                }
            }
            Trait::Glare(eye) => {
                let mine = family(*eye);
                let theirs = a
                    .spec()
                    .and_then(|spec| spec.main_colour())
                    .and_then(family);
                if let (Some(mine), Some(theirs)) = (mine, theirs)
                    && opposite(mine, theirs)
                {
                    percent *= BODY_COLOUR_TIMES;
                }
            }
            Trait::Patches(patches)
                if a.traits
                    .iter()
                    .any(|t| matches!(t, Trait::Feast(_) | Trait::Greed)) =>
            {
                percent += PATCH_PERCENT * patches;
            }
            _ => {}
        }
    }
    percent
}

#[cfg(test)]
mod tests;
