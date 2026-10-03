use fishtank::{
    entities::cow::{Cow, CowVariant},
    fishes::{
        fish::{Direction, Fish, PUFF_SECS},
        mutations::{Holder, Mutatable, Mutation, apply_mutation, apply_mutation_to_fish},
        species::FishSpecies,
        unfish::UnfishKind,
    },
    sprite::{Band, ExtensionVariant, TRANSPARENT},
};

fn fish(species: FishSpecies) -> Fish {
    let mut fish = Fish::new(species, "Probe".to_string(), 10.0, 10.0, &mut rand::rng());
    fish.facing = Direction::Left;
    fish
}

fn grow(fish: &mut Fish, mutation: Mutation) {
    assert!(
        fish.supports_now(mutation),
        "{} is offered",
        mutation.token()
    );
    apply_mutation_to_fish(fish, mutation, &mut rand::rng());
}

fn rows(fish: &Fish) -> Vec<String> {
    fish.line_sprite()
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|&(c, _)| if c == TRANSPARENT { ' ' } else { c })
                .collect()
        })
        .collect()
}

fn rows_above(fish: &Fish) -> usize {
    fish.line_sprite().body_row
}

fn rows_below(fish: &Fish) -> usize {
    let sprite = fish.line_sprite();
    sprite.rows.len() - 1 - sprite.body_row
}

#[test]
fn feet_and_a_ventral_fin_never_share_the_bottom_band() {
    let mut footed = fish(FishSpecies::Salmon);
    grow(&mut footed, Mutation::Feet);
    assert!(!footed.supports_now(Mutation::VentralFin));
    let mut finned = fish(FishSpecies::Salmon);
    grow(&mut finned, Mutation::VentralFin);
    assert!(!finned.supports_now(Mutation::Feet));
}

#[test]
fn a_lure_and_a_bill_never_share_the_head() {
    let mut lured = fish(FishSpecies::Salmon);
    grow(&mut lured, Mutation::Lure);
    assert!(!lured.supports_now(Mutation::Bill));
    let mut billed = fish(FishSpecies::Salmon);
    grow(&mut billed, Mutation::Bill);
    assert!(!billed.supports_now(Mutation::Lure));
}

#[test]
fn growths_on_different_bands_live_together() {
    let mut salmon = fish(FishSpecies::Salmon);
    grow(&mut salmon, Mutation::DorsalFin);
    grow(&mut salmon, Mutation::Tentacles);
    grow(&mut salmon, Mutation::Lure);
    assert_eq!(salmon.holds(Band::Top), Holder::Fin);
    assert_eq!(
        salmon.holds(Band::Bottom),
        Holder::Extension(ExtensionVariant::Tentacle)
    );
    let mut walker = fish(FishSpecies::Salmon);
    grow(&mut walker, Mutation::Feet);
    grow(&mut walker, Mutation::Lure);
}

#[test]
fn spikes_and_wings_grow_one_row_and_tentacles_two() {
    for (mutation, above, below) in [
        (Mutation::Spikes, 1, 1),
        (Mutation::Wings, 1, 1),
        (Mutation::Tentacles, 0, 2),
    ] {
        let mut salmon = fish(FishSpecies::Salmon);
        grow(&mut salmon, mutation);
        assert_eq!(rows_above(&salmon), above, "{}", mutation.token());
        assert_eq!(rows_below(&salmon), below, "{}", mutation.token());
        assert!(
            !salmon.supports_now(mutation),
            "{} grows once",
            mutation.token()
        );
    }
}

#[test]
fn a_new_extension_takes_over_the_bands_it_grows_on() {
    let mut salmon = fish(FishSpecies::Salmon);
    grow(&mut salmon, Mutation::Spikes);
    grow(&mut salmon, Mutation::Tentacles);
    let extension = salmon.body_extension();
    assert_eq!(
        extension.top,
        Some(ExtensionVariant::Spike),
        "the top keeps its spikes"
    );
    assert_eq!(extension.bottom, Some(ExtensionVariant::Tentacle));
    grow(&mut salmon, Mutation::Wings);
    let extension = salmon.body_extension();
    assert_eq!(extension.top, Some(ExtensionVariant::Wing));
    assert_eq!(extension.bottom, Some(ExtensionVariant::Wing));
}

#[test]
fn an_extension_grows_around_a_fin_never_over_it() {
    let mut salmon = fish(FishSpecies::Salmon);
    grow(&mut salmon, Mutation::DorsalFin);
    grow(&mut salmon, Mutation::Wings);
    assert_eq!(salmon.holds(Band::Top), Holder::Fin);
    assert_eq!(
        salmon.holds(Band::Bottom),
        Holder::Extension(ExtensionVariant::Wing)
    );
    let mut both = fish(FishSpecies::Salmon);
    grow(&mut both, Mutation::DorsalFin);
    grow(&mut both, Mutation::VentralFin);
    for mutation in [Mutation::Spikes, Mutation::Wings, Mutation::Tentacles] {
        assert!(
            !both.supports_now(mutation),
            "{} has no band left",
            mutation.token()
        );
    }
}

#[test]
fn nothing_grows_on_a_botfish_antenna() {
    let mut bot = fish(FishSpecies::Botfish);
    assert!(bot.reserves(Band::Top));
    assert!(!bot.supports_now(Mutation::DorsalFin));
    let antenna = rows(&bot)[..rows_above(&bot)].to_vec();
    grow(&mut bot, Mutation::Spikes);
    assert_eq!(bot.body_extension().top, None);
    assert_eq!(bot.body_extension().bottom, Some(ExtensionVariant::Spike));
    grow(&mut bot, Mutation::Puff);
    bot.habits.puffed = PUFF_SECS;
    assert_eq!(
        rows(&bot)[..rows_above(&bot)].to_vec(),
        antenna,
        "the rows above a botfish are its antenna and nothing else"
    );
}

#[test]
fn a_gliding_fish_keeps_both_bands_for_its_wings() {
    let volador = fish(FishSpecies::Volador);
    for mutation in [
        Mutation::DorsalFin,
        Mutation::VentralFin,
        Mutation::Feet,
        Mutation::Spikes,
        Mutation::Wings,
        Mutation::Tentacles,
    ] {
        assert!(!volador.supports_now(mutation), "{}", mutation.token());
    }
}

#[test]
fn a_puff_spikes_only_the_free_bands() {
    let mut globo = fish(FishSpecies::Globo);
    grow(&mut globo, Mutation::Feet);
    globo.habits.puffed = PUFF_SECS;
    let below = rows(&globo)[rows_above(&globo) + 1..].to_vec();
    assert_eq!(below.len(), 1, "only the feet hang below: {below:?}");
    assert!(
        rows_above(&globo) >= 1,
        "the top band is free, so it spikes"
    );
    let mut finned = fish(FishSpecies::Globo);
    grow(&mut finned, Mutation::DorsalFin);
    grow(&mut finned, Mutation::VentralFin);
    finned.habits.puffed = PUFF_SECS;
    assert_eq!(
        rows_above(&finned),
        1,
        "only the dorsal fin: {:?}",
        rows(&finned)
    );
    assert_eq!(
        rows_below(&finned),
        1,
        "only the ventral fin: {:?}",
        rows(&finned)
    );
}

#[test]
fn an_octopus_is_born_with_tentacles_that_spikes_can_replace() {
    let mut pulpo = fish(FishSpecies::Pulpo);
    assert_eq!(
        pulpo.body_extension().bottom,
        Some(ExtensionVariant::Tentacle),
        "its tentacles are a birthmark"
    );
    assert_eq!(pulpo.mutation_count(), 0);
    assert!(!pulpo.supports_now(Mutation::Tentacles));
    assert!(pulpo.reserves(Band::Top), "its mantle fills the top band");
    grow(&mut pulpo, Mutation::Spikes);
    assert_eq!(pulpo.body_extension().bottom, Some(ExtensionVariant::Spike));
    assert_eq!(pulpo.body_extension().top, None);
}

#[test]
fn a_school_or_a_hammerhead_has_no_free_band() {
    for species in [FishSpecies::Neon, FishSpecies::Martillo] {
        let figure = fish(species);
        assert!(figure.reserves(Band::Top), "{}", species.display_name());
        assert!(figure.reserves(Band::Bottom), "{}", species.display_name());
    }
}

#[test]
fn a_one_glyph_body_draws_every_growth_it_takes() {
    for species in [FishSpecies::Jellyfish, FishSpecies::Estrella] {
        for (mutation, above, below) in [
            (Mutation::DorsalFin, 1, 0),
            (Mutation::VentralFin, 0, 1),
            (Mutation::Feet, 0, 1),
            (Mutation::Spikes, 1, 1),
            (Mutation::Tentacles, 0, 2),
        ] {
            let mut body = fish(species);
            grow(&mut body, mutation);
            assert_eq!(
                (rows_above(&body), rows_below(&body)),
                (above, below),
                "{} {}: {:?}",
                species.display_name(),
                mutation.token(),
                rows(&body)
            );
        }
        let mut puffer = fish(species);
        grow(&mut puffer, Mutation::Puff);
        puffer.habits.puffed = PUFF_SECS;
        assert_eq!(
            (rows_above(&puffer), rows_below(&puffer)),
            (1, 1),
            "{} puffs: {:?}",
            species.display_name(),
            rows(&puffer)
        );
    }
}

#[test]
fn a_cow_grows_extensions_on_its_back_only() {
    let mut rng = rand::rng();
    let mut cow = Cow::new("Vaquita".into(), CowVariant::Brown, 0.0, 0.0, &mut rng);
    assert!(cow.reserves(Band::Bottom));
    assert!(!cow.supports_now(Mutation::Tentacles));
    assert!(cow.supports_now(Mutation::Wings));
    apply_mutation(&mut cow, Mutation::Wings, &mut rng);
    assert_eq!(cow.extension().top, Some(ExtensionVariant::Wing));
    assert_eq!(cow.extension().bottom, None);
}

#[test]
fn a_line_unfish_follows_the_same_bands() {
    let mut phantom = Fish::new_unfish(
        UnfishKind::Phantom,
        "Boo".into(),
        0.0,
        0.0,
        &mut rand::rng(),
    );
    grow(&mut phantom, Mutation::Feet);
    assert!(!phantom.supports_now(Mutation::VentralFin));
    grow(&mut phantom, Mutation::Spikes);
    assert_eq!(phantom.body_extension().top, Some(ExtensionVariant::Spike));
    assert_eq!(phantom.body_extension().bottom, None);
}
