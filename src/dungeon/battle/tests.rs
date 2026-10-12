use super::*;
use crate::dungeon::bestiary::{EnemySkill, EnemySpec, Rank};
use crate::dungeon::effect::Effect;
use crate::fishes::species::FishSpecies;

const SEEDS: u64 = 40;
const MAX_CHOICES: usize = 2_000;

const fn enemy(name: &'static str, rank: Rank, hp: u32, bite: u32, speed: u32) -> EnemySpec {
    EnemySpec {
        name,
        floor: 1,
        rank,
        hp,
        mp: 3,
        bite,
        speed,
        sprite: &[],
        skills: &[],
        passives: &[],
        traits: &[],
        loot: &[],
        variant_of: None,
        note: "",
    }
}

static BRUTE: EnemySpec = enemy("Brute", Rank::Common, 400, 30, 6);
static SLUG: EnemySpec = enemy("Slug", Rank::Common, 10_000, 1, 1);
static WARDEN: EnemySpec = enemy("Warden", Rank::Boss, 10_000, 1, 6);
static TELLER: EnemySpec = EnemySpec {
    skills: &[EnemySkill {
        name: "Crush",
        cost: 3,
        reach: Reach::OneFoe,
        effects: &[Effect::skill(1)],
        about: "",
    }],
    ..enemy("Teller", Rank::Common, 10_000, 1, 6)
};

fn fish(species: FishSpecies, seed: u64) -> Fish {
    let mut rng = SmallRng::seed_from_u64(seed);
    Fish::new(species, format!("{species:?}"), 0.0, 0.0, &mut rng)
}

fn setup(difficulty: Difficulty, stakes: Stakes, seed: u64) -> BattleSetup {
    BattleSetup {
        floor: 1,
        difficulty,
        stakes,
        purse: 0,
        items: Vec::new(),
        seed,
        ambushed: false,
        haunts: Vec::new(),
        scale: Scale::PLAIN,
    }
}

fn battle(party: &[FishSpecies], foes: &[&'static EnemySpec], setup: BattleSetup) -> Battle {
    let party = party
        .iter()
        .enumerate()
        .map(|(i, &species)| fish(species, setup.seed + i as u64))
        .collect();
    let foes = foes
        .iter()
        .map(|&spec| Fighter::of_enemy(spec, setup.floor, Scale::PLAIN))
        .collect();
    Battle::new(party, foes, setup)
}

fn bite_the_first_foe(battle: &mut Battle) {
    battle.begin();
    for _ in 0..MAX_CHOICES {
        if battle.actor().is_none() {
            return;
        }
        let choice = match battle.standing(Side::Foes).first() {
            Some(&foe) => Choice::Bite(foe),
            None => Choice::Guard,
        };
        battle.choose(choice);
    }
}

fn party_fighter(battle: &Battle) -> usize {
    battle.side(Side::Party).next().expect("a fish")
}

fn foe_fighter(battle: &Battle) -> usize {
    battle.side(Side::Foes).next().expect("an enemy")
}

#[test]
fn normal_never_kills() {
    let party = [
        FishSpecies::Anchoveta,
        FishSpecies::Salmon,
        FishSpecies::Merluza,
    ];
    for seed in 0..SEEDS {
        let mut fight = battle(
            &party,
            &[&BRUTE, &BRUTE, &BRUTE],
            setup(Difficulty::Normal, Stakes::Den, seed),
        );
        bite_the_first_foe(&mut fight);
        assert_eq!(fight.outcome(), Some(Outcome::Wiped), "seed {seed}");
        assert!(fight.dead.is_empty(), "seed {seed} killed {:?}", fight.dead);
        assert!(fight.side(Side::Party).all(|i| !fight.fighters[i].gone));
    }
}

#[test]
fn hardcore_turns_a_hit_on_a_belly_up_fish_into_a_death_blow() {
    let party = [
        FishSpecies::Anchoveta,
        FishSpecies::Salmon,
        FishSpecies::Merluza,
    ];
    let deaths: usize = (0..SEEDS)
        .map(|seed| {
            let mut fight = battle(
                &party,
                &[&BRUTE, &BRUTE, &BRUTE],
                setup(Difficulty::Hardcore, Stakes::Den, seed),
            );
            bite_the_first_foe(&mut fight);
            fight.dead.len()
        })
        .sum();
    assert!(deaths > 0);
}

#[test]
fn a_big_hit_rolls_half_now_and_half_at_the_start_of_the_next_turn() {
    let mut fight = battle(
        &[FishSpecies::Anchoveta],
        &[&SLUG],
        setup(Difficulty::Normal, Stakes::Encounter, 1),
    );
    fight.begin();
    let fish = party_fighter(&fight);
    let slug = foe_fighter(&fight);
    assert_eq!(fight.actor(), Some(fish));
    let max = fight.fighters[fish].max_hp;
    let hit = max / 2;
    fight.harm(slug, fish, hit, DamageKind::Skill);
    let now = hit.div_ceil(HALF);
    assert_eq!(fight.fighters[fish].hp, max - now);
    assert_eq!(fight.fighters[fish].rolling, hit - now);
    fight.choose(Choice::Bite(slug));
    assert_eq!(fight.actor(), Some(fish));
    assert_eq!(fight.fighters[fish].hp, max - hit);
    assert_eq!(fight.fighters[fish].rolling, 0);
}

#[test]
fn a_heal_or_a_guard_stops_the_roll() {
    let mut fight = battle(
        &[FishSpecies::Anchoveta],
        &[&SLUG],
        setup(Difficulty::Normal, Stakes::Encounter, 2),
    );
    fight.begin();
    let fish = party_fighter(&fight);
    let slug = foe_fighter(&fight);
    let hit = fight.fighters[fish].max_hp / 2;
    fight.harm(slug, fish, hit, DamageKind::Skill);
    fight.heal(fish, 1);
    assert_eq!(fight.fighters[fish].rolling, 0);
    fight.harm(slug, fish, hit / 2, DamageKind::Skill);
    assert!(fight.fighters[fish].rolling > 0);
    let before = fight.fighters[fish].hp;
    fight.choose(Choice::Guard);
    assert_eq!(fight.fighters[fish].rolling, 0);
    assert_eq!(fight.fighters[fish].hp, before);
}

#[test]
fn a_small_hit_does_not_roll() {
    let mut fight = battle(
        &[FishSpecies::Salmon],
        &[&SLUG],
        setup(Difficulty::Normal, Stakes::Encounter, 3),
    );
    fight.begin();
    let fish = party_fighter(&fight);
    let slug = foe_fighter(&fight);
    let max = fight.fighters[fish].max_hp;
    fight.harm(slug, fish, 1, DamageKind::Skill);
    assert_eq!(fight.fighters[fish].hp, max - 1);
    assert_eq!(fight.fighters[fish].rolling, 0);
}

#[test]
fn the_turn_bar_starts_with_the_active_fighter_and_favours_the_fast() {
    let mut fight = battle(
        &[FishSpecies::Anchoveta],
        &[&WARDEN],
        setup(Difficulty::Normal, Stakes::Encounter, 4),
    );
    fight.begin();
    let fish = party_fighter(&fight);
    let warden = foe_fighter(&fight);
    let actor = fight.actor().expect("someone chooses");
    let order = fight.turn_order(TURN_BAR_LENGTH);
    assert_eq!(order.len(), TURN_BAR_LENGTH);
    assert_eq!(order[0], actor);
    let long = fight.turn_order(TURN_BAR_LENGTH * 6);
    let fast = long.iter().filter(|&&i| i == fish).count();
    let slow = long.iter().filter(|&&i| i == warden).count();
    let ratio = fight.fighters[fish].speed as usize / fight.fighters[warden].speed as usize;
    assert!(
        fast.abs_diff(slow * ratio) <= ratio,
        "{fast} turns against {slow}"
    );
}

#[test]
fn the_turn_bar_is_what_actually_happens() {
    let mut fight = battle(
        &[FishSpecies::Anchoveta, FishSpecies::Leaffish],
        &[&SLUG],
        setup(Difficulty::Normal, Stakes::Encounter, 5),
    );
    fight.begin();
    let promised = fight.turn_order(4);
    let slug = foe_fighter(&fight);
    let mut acted = vec![fight.actor().expect("a fish chooses")];
    while acted.len() < promised.len() {
        let before = fight.log.len();
        fight.choose(Choice::Guard);
        let slug_acted = fight.log[before..]
            .iter()
            .filter(|line| line.text.starts_with("Slug "))
            .count();
        acted.extend(std::iter::repeat_n(slug, slug_acted));
        acted.extend(fight.actor());
    }
    assert_eq!(&acted[..promised.len()], &promised[..]);
}

#[test]
fn a_move_that_costs_every_bubble_is_told_before_it_lands() {
    let mut fight = battle(
        &[FishSpecies::Salmon],
        &[&TELLER],
        setup(Difficulty::Normal, Stakes::Encounter, 6),
    );
    fight.begin();
    for _ in 0..40 {
        fight.choose(Choice::Guard);
    }
    let lines: Vec<&str> = fight.log.iter().map(|line| line.text.as_str()).collect();
    let told = lines
        .iter()
        .position(|line| line.contains("ready to use Crush"))
        .expect("the tell");
    let used = lines
        .iter()
        .position(|line| *line == "Teller uses Crush.")
        .expect("the move");
    assert!(told < used);
    assert_eq!(fight.log[told].tone, Tone::Tell);
}

#[test]
fn nobody_flees_a_boss() {
    let mut fight = battle(
        &[FishSpecies::Anchoveta],
        &[&WARDEN],
        setup(Difficulty::Normal, Stakes::Boss, 7),
    );
    fight.begin();
    let actor = fight.actor().expect("a fish chooses");
    assert!(!fight.can_choose(actor, Choice::Flee));
    assert!(fight.choose(Choice::Flee).is_empty());
}

#[test]
fn a_walking_fish_always_gets_the_party_away() {
    for seed in 0..SEEDS {
        let mut fight = battle(
            &[FishSpecies::Leaffish],
            &[&SLUG],
            setup(Difficulty::Normal, Stakes::Encounter, seed),
        );
        fight.begin();
        let fish = party_fighter(&fight);
        fight.fighters[fish].traits.push(Trait::Walks);
        fight.choose(Choice::Flee);
        assert_eq!(fight.outcome(), Some(Outcome::Fled), "seed {seed}");
    }
}

#[test]
fn a_fish_with_a_head_on_each_end_never_flees() {
    for seed in 0..SEEDS {
        let mut fight = battle(
            &[FishSpecies::Anchoveta],
            &[&SLUG],
            setup(Difficulty::Normal, Stakes::Encounter, seed),
        );
        fight.begin();
        let fish = party_fighter(&fight);
        fight.fighters[fish]
            .traits
            .extend([Trait::Walks, Trait::Backwards]);
        fight.choose(Choice::Flee);
        assert_eq!(fight.outcome(), None, "seed {seed}");
    }
}

#[test]
fn every_item_packed_for_a_fight_changes_the_fight() {
    use crate::dungeon::items::{Aim, Item};
    for item in Item::for_a_fight() {
        let mut fight = battle(
            &[
                FishSpecies::Salmon,
                FishSpecies::Merluza,
                FishSpecies::Botfish,
            ],
            &[&SLUG],
            setup(Difficulty::Normal, Stakes::Encounter, 8),
        );
        fight.begin();
        let fallen = fight.side(Side::Party).nth(1).expect("a second fish");
        let max = fight.fighters[fallen].max_hp;
        fight.lose_hp(fallen, max, None);
        assert!(fight.fighters[fallen].belly_up);
        let target = match item.aim() {
            Aim::Foe => foe_fighter(&fight),
            Aim::Ally | Aim::Nobody => party_fighter(&fight),
        };
        fight.fighters[target].inflict(Status::new(StatusKind::BiteDown, 2, 1));
        let fighters = fight.fighters.clone();
        let weights: Vec<u32> = fight.party.iter().map(|fish| fish.weight_g).collect();
        crate::dungeon::items::apply(&mut fight, item, target);
        let after: Vec<u32> = fight.party.iter().map(|fish| fish.weight_g).collect();
        assert!(
            fight.fighters != fighters || after != weights,
            "{} does nothing in a fight",
            item.name()
        );
    }
}

#[test]
fn every_enemy_in_the_bestiary_can_be_fought_to_the_end() {
    let party = [
        FishSpecies::Salmon,
        FishSpecies::Lionfish,
        FishSpecies::Holyfish,
        FishSpecies::Botfish,
    ];
    for spec in crate::dungeon::bestiary::all() {
        for difficulty in [Difficulty::Normal, Difficulty::Hardcore] {
            let mut fight = battle(&party, &[spec, spec], setup(difficulty, Stakes::Den, 11));
            bite_the_first_foe(&mut fight);
            for fighter in &fight.fighters {
                assert!(
                    fighter.hp <= fighter.max_hp,
                    "{} in {}",
                    fighter.name,
                    spec.name
                );
                assert!(
                    fighter.mp <= fighter.max_mp,
                    "{} in {}",
                    fighter.name,
                    spec.name
                );
            }
        }
    }
}
