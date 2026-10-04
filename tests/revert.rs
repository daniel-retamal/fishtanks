use fishtank::{
    entities::cow::{Cow, CowVariant},
    fishes::{
        fish::{Direction, Fish},
        mutations::{Mutatable, Mutation, apply_mutation},
        revert::{revert, revert_latest},
        species::FishSpecies,
        unfish::UnfishKind,
    },
    sprite::ExtensionVariant,
};

const BLESSED_BONUS_PCT: u32 = 10;

fn salmon() -> Fish {
    let mut fish = Fish::new(
        FishSpecies::Salmon,
        "Probe".to_string(),
        10.0,
        10.0,
        &mut rand::rng(),
    );
    fish.facing = Direction::Left;
    fish
}

fn mutate<M: Mutatable>(target: &mut M, mutations: &[Mutation]) {
    for &mutation in mutations {
        assert!(target.supports_now(mutation), "{}", mutation.token());
        apply_mutation(target, mutation, &mut rand::rng());
    }
}

fn history(fish: &Fish) -> Vec<String> {
    fish.mutations
        .as_ref()
        .map_or_else(Vec::new, |record| record.history.clone())
}

#[test]
fn a_revert_takes_out_one_mutation_and_keeps_the_rest_exactly() {
    let mut whole = salmon();
    mutate(
        &mut whole,
        &[
            Mutation::BodyColor,
            Mutation::DorsalFin,
            Mutation::Ear,
            Mutation::ColorPatch,
        ],
    );
    let mut reverted = whole.clone();
    assert!(revert_latest(&mut reverted, Mutation::DorsalFin));
    assert_eq!(history(&reverted), ["bodycolor", "ear", "colorpatch"]);
    assert!(!reverted.adornments().dorsal_fin);
    assert_eq!(
        reverted.color, whole.color,
        "the body colour is the same roll"
    );
    let (kept, before) = (
        reverted.mutant.as_ref().unwrap(),
        whole.mutant.as_ref().unwrap(),
    );
    assert_eq!(kept.ear_count, before.ear_count);
    assert_eq!(kept.color_patches, before.color_patches);
    let body = |fish: &Fish| {
        let sprite = fish.line_sprite();
        sprite.rows[sprite.body_row].clone()
    };
    assert_eq!(body(&reverted), body(&whole));
    assert_eq!(reverted.line_sprite().body_row, 0, "the fin row is gone");
}

#[test]
fn reverting_an_extension_brings_back_what_it_took_over() {
    let mut fish = salmon();
    mutate(&mut fish, &[Mutation::Spikes, Mutation::Tentacles]);
    let mut back_to_spikes = fish.clone();
    assert!(revert_latest(&mut back_to_spikes, Mutation::Tentacles));
    assert_eq!(
        back_to_spikes.body_extension().top,
        Some(ExtensionVariant::Spike)
    );
    assert_eq!(
        back_to_spikes.body_extension().bottom,
        Some(ExtensionVariant::Spike)
    );
    let mut only_tentacles = fish.clone();
    assert!(revert_latest(&mut only_tentacles, Mutation::Spikes));
    assert_eq!(only_tentacles.body_extension().top, None);
    assert_eq!(
        only_tentacles.body_extension().bottom,
        Some(ExtensionVariant::Tentacle)
    );
}

#[test]
fn a_revert_takes_back_exactly_the_money_its_mutation_made() {
    let mut fish = salmon();
    fish.sell_price_bonus_pct = BLESSED_BONUS_PCT;
    mutate(&mut fish, &[Mutation::Strawberry]);
    assert!(fish.sell_price_bonus_pct > BLESSED_BONUS_PCT);
    assert!(revert_latest(&mut fish, Mutation::Strawberry));
    assert_eq!(
        fish.sell_price_bonus_pct, BLESSED_BONUS_PCT,
        "a blessing is not a mutation"
    );
}

#[test]
fn a_revert_never_unpicks_a_fusion() {
    let mut fish = salmon();
    mutate(&mut fish, &[Mutation::Telophase, Mutation::Ear]);
    assert!(revert(&mut fish, &mut rand::rng()).is_some());
    assert_eq!(history(&fish), ["telophase"]);
    assert!(fish.is_double_now(), "the double outlives the reverted ear");
    assert!(
        !fish.supports_now(Mutation::Revert),
        "a fusion is not reverted"
    );
    assert!(revert(&mut fish, &mut rand::rng()).is_none());
}

#[test]
fn revert_is_the_only_pull_and_is_never_recorded() {
    let pulls = [
        "sizedecrease",
        "eyedecrease",
        "eardecrease",
        "nofeet",
        "glistendisable",
        "decreaseextension",
    ];
    assert!(pulls.iter().all(|token| Mutation::parse(token).is_none()));
    assert_eq!(
        Mutation::ALL
            .iter()
            .filter(|&&m| m == Mutation::Revert)
            .count(),
        1
    );
    let mut fish = salmon();
    mutate(&mut fish, &[Mutation::Ear]);
    let count = fish.mutation_count();
    mutate(&mut fish, &[Mutation::Revert]);
    assert!(history(&fish).is_empty());
    assert_eq!(
        fish.mutation_count(),
        count + 1,
        "a revert is a mutation event"
    );
}

#[test]
fn a_mutation_with_nothing_left_to_act_on_leaves_too() {
    let mut fish = salmon();
    mutate(
        &mut fish,
        &[Mutation::Ear, Mutation::EarColor, Mutation::Lure],
    );
    assert!(revert_latest(&mut fish, Mutation::Ear));
    assert_eq!(history(&fish), ["lure"], "the ear colour goes with the ear");
}

#[test]
fn an_old_record_keeps_its_mutations_and_reverts_only_new_ones() {
    let mut fish = salmon();
    mutate(&mut fish, &[Mutation::Ear]);
    let record = fish.mutations.as_mut().unwrap();
    record.marks.clear();
    record.origin = None;
    assert!(!fish.supports_now(Mutation::Revert));
    mutate(&mut fish, &[Mutation::Lure]);
    assert!(revert(&mut fish, &mut rand::rng()).is_some());
    assert!(!fish.adornments().lure);
    assert_eq!(
        fish.mutant.as_ref().unwrap().ear_count,
        1,
        "the old ear stays"
    );
    assert!(revert(&mut fish, &mut rand::rng()).is_none());
}

#[test]
fn a_cow_and_an_unfish_revert_like_a_fish() {
    let mut rng = rand::rng();
    let mut cow = Cow::new("Vaquita".into(), CowVariant::Brown, 0.0, 0.0, &mut rng);
    mutate(&mut cow, &[Mutation::Wings]);
    assert!(revert(&mut cow, &mut rng).is_some());
    assert!(cow.extension().is_empty());
    let mut phantom = Fish::new_unfish(UnfishKind::Phantom, "Boo".into(), 0.0, 0.0, &mut rng);
    let width = phantom.display_width;
    mutate(&mut phantom, &[Mutation::Feet, Mutation::Ear]);
    assert!(revert_latest(&mut phantom, Mutation::Ear));
    assert!(revert_latest(&mut phantom, Mutation::Feet));
    assert!(phantom.feet().is_none());
    assert_eq!(phantom.display_width, width);
}

#[test]
fn a_saved_record_still_reverts_after_a_round_trip() {
    let mut fish = salmon();
    mutate(&mut fish, &[Mutation::BodyColor, Mutation::Spikes]);
    let text = ron::to_string(&fish).expect("a fish saves");
    let mut loaded: Fish = ron::from_str(&text).expect("and loads");
    assert!(revert_latest(&mut loaded, Mutation::Spikes));
    assert!(loaded.body_extension().is_empty());
    assert_eq!(loaded.color, fish.color);
}

fn drawn_width(fish: &Fish) -> usize {
    fish.line_sprite()
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|&(c, _)| unicode_width::UnicodeWidthChar::width(c).unwrap_or(1))
                .sum::<usize>()
        })
        .max()
        .unwrap_or(0)
}

#[test]
fn any_history_reverts_down_to_its_fusions_without_breaking_the_body() {
    const ROUNDS: usize = 4;
    const MUTATIONS: usize = 14;
    let mut rng = rand::rng();
    let mut beings: Vec<Fish> = fishtank::fishes::species::ALL_SPECIES
        .iter()
        .map(|&species| Fish::new(species, "Probe".into(), 10.0, 10.0, &mut rng))
        .collect();
    for kind in [
        UnfishKind::Reversed,
        UnfishKind::Phantom,
        UnfishKind::Ball,
        UnfishKind::Worm,
    ] {
        beings.push(Fish::new_unfish(kind, "Probe".into(), 10.0, 10.0, &mut rng));
    }
    for _ in 0..ROUNDS {
        for being in &beings {
            let mut fish = being.clone();
            for _ in 0..MUTATIONS {
                let Some(mutation) = fish.random_mutation_with_room(&mut rng, false) else {
                    break;
                };
                apply_mutation(&mut fish, mutation, &mut rng);
            }
            while revert(&mut fish, &mut rng).is_some() {
                assert!(
                    fish.unfish_body().is_some() || drawn_width(&fish) <= fish.display_width,
                    "{} draws {} in a {}-cell slot after a revert: {:?}",
                    fish.species.display_name(),
                    drawn_width(&fish),
                    fish.display_width,
                    fish.mutations.as_ref().map(|record| record.history.clone())
                );
            }
            let left = fish
                .mutations
                .as_ref()
                .map_or_else(Vec::new, |record| record.history.clone());
            assert!(
                left.iter()
                    .all(|token| Mutation::parse(token).is_some_and(Mutation::fuses)),
                "{} keeps only its fusions: {left:?}",
                fish.species.display_name()
            );
            if left.is_empty() {
                let mut original = being.clone();
                original.facing = fish.facing;
                original.sway = fish.sway.clone();
                assert_eq!(
                    fish.line_sprite().rows,
                    original.line_sprite().rows,
                    "{} looks as it did before its first mutation",
                    fish.species.display_name()
                );
                assert_eq!(fish.display_width, original.display_width);
                assert_eq!(fish.weight_g, original.weight_g);
                assert_eq!(fish.sell_price_bonus_pct, original.sell_price_bonus_pct);
            }
        }
    }
}

#[test]
fn a_cyclops_cow_has_one_eye_and_revert_gives_the_other_back() {
    let mut cow = Cow::new(
        "Polyphemus".to_string(),
        CowVariant::Brown,
        10.0,
        10.0,
        &mut rand::rng(),
    );
    let born = cow.eye_count();
    assert!(born > 1, "a cow is born with two eyes");
    mutate(&mut cow, &[Mutation::Cyclops]);
    assert_eq!(cow.eye_count(), 1);
    assert!(
        !cow.supports_now(Mutation::Cyclops),
        "one eye is as few as it gets"
    );
    assert_eq!(revert(&mut cow, &mut rand::rng()), Some(Mutation::Cyclops));
    assert_eq!(cow.eye_count(), born);
}

#[test]
fn a_fish_with_one_eye_is_never_offered_cyclops() {
    let fish = salmon();
    assert!(!fish.supports_now(Mutation::Cyclops));
}
