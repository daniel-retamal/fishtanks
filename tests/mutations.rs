use fishtank::colors::{LIGHT_GREEN, PINK};
use fishtank::entities::cow::{CowVariant, cow_hydra_capacity, cow_sprite};
use fishtank::fishes::fish::Fish;
use fishtank::fishes::species::FishSpecies;
use fishtank::fishes::unfish::{BALL_HEIGHT, SKULL_HEIGHT, UnfishKind};
use fishtank::sprite::{BodyExtension, ExtensionVariant, mirror_char};
use fishtank::tank::{Tank, TankKind};

const STRAWBERRY_SELL_BONUS_PCT: u32 = 25;

fn make_tank() -> Tank {
    Tank::new("Test".to_string(), TankKind::Base, &[])
}

fn rng() -> impl rand::RngExt {
    rand::rng()
}

#[test]
fn holyfish_cannot_mutate() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Holyfish, "Saint".to_string(), &mut rng);
    assert!(!tank.apply_named_mutation("Saint", "telophase"));
    assert!(!tank.apply_named_mutation("Saint", "eyeincrease"));
    assert!(!tank.apply_named_mutation("Saint", "engulfment"));
}

#[test]
fn holyfish_is_never_devil_marked_in_the_helltank() {
    let mut tank = Tank::new("Hell".to_string(), TankKind::Hell, &[]);
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Holyfish, "Saint".to_string(), &mut rng);
    tank.spawn_fish(FishSpecies::Merluza, "Sinner".to_string(), &mut rng);
    let saint = tank.fish.iter().find(|f| f.name == "Saint").unwrap();
    let sinner = tank.fish.iter().find(|f| f.name == "Sinner").unwrap();
    assert!(
        !saint.devil_marked,
        "the holy fish resists the devil's mark"
    );
    assert!(
        sinner.devil_marked,
        "ordinary fish are still marked in hell"
    );
}

#[test]
fn anchoveta_eye_plus_sets_display_width_and_left_eye() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Anchoveta, "Tiny".to_string(), &mut rng);
    tank.apply_named_mutation("Tiny", "eyeincrease");
    let fish = &tank.fish[0];
    assert_eq!(fish.display_width, 4);
    let m = fish
        .mutant
        .as_ref()
        .expect("mutant should exist after eye+");
    assert_eq!(m.left_eyes.len(), 1);
}

#[test]
fn anchoveta_size_plus_sets_display_width_and_body_size() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Anchoveta, "Tiny".to_string(), &mut rng);
    tank.apply_named_mutation("Tiny", "sizeincrease");
    let fish = &tank.fish[0];
    assert_eq!(fish.display_width, 4);
    assert_eq!(fish.body_size, 2);
}

#[test]
fn anchoveta_doublefish_sets_is_double_and_display_width() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Anchoveta, "Tiny".to_string(), &mut rng);
    tank.apply_named_mutation("Tiny", "telophase");
    let fish = &tank.fish[0];
    assert_eq!(fish.display_width, 4);
    let m = fish
        .mutant
        .as_ref()
        .expect("mutant should exist after doublefish");
    assert!(m.is_double);
    assert!(
        m.double_head_eyes.is_empty(),
        "anchoveta has no eyes, so the second head must also have none"
    );
}

#[test]
fn standard_fish_doublefish_has_one_double_head_eye() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Bubbles".to_string(), &mut rng);
    tank.apply_named_mutation("Bubbles", "telophase");
    let m = tank.fish[0]
        .mutant
        .as_ref()
        .expect("mutant should exist after doublefish");
    assert!(m.is_double);
    assert_eq!(
        m.double_head_eyes.len(),
        1,
        "standard fish start with 1 eye, so the second head must also have exactly 1"
    );
}

#[test]
fn anchoveta_bodycolor_changes_fish_color() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Anchoveta, "Tiny".to_string(), &mut rng);
    let color_before = tank.fish[0].color;
    tank.apply_named_mutation("Tiny", "bodycolor");
    let color_after = tank.fish[0].color;
    assert_ne!(
        color_before, color_after,
        "bodycolor mutation should change fish color"
    );
}

#[test]
fn anchoveta_bodycolor_sets_glistening_color_on_mutant() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Anchoveta, "Tiny".to_string(), &mut rng);
    tank.apply_named_mutation("Tiny", "bodycolor");
    let fish = &tank.fish[0];
    assert!(
        fish.mutant.is_some(),
        "mutant should exist after bodycolor mutation"
    );
}

#[test]
fn ball_unfish_eyecolor_mutation_sets_slime_eye_color() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Ball, "Orb".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Orb".to_string(), &mut rng);
    tank.apply_named_mutation("Orb", "eyecolor");
    let us = tank.fish[0]
        .unfish_state
        .as_ref()
        .expect("unfish_state must exist");
    assert!(us.slime_eye_color.is_some());
}

#[test]
fn ball_unfish_bodycolor_mutation_sets_slime_body_color() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Ball, "Orb".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Orb".to_string(), &mut rng);
    tank.apply_named_mutation("Orb", "bodycolor");
    let us = tank.fish[0]
        .unfish_state
        .as_ref()
        .expect("unfish_state must exist");
    assert!(us.slime_body_color.is_some());
}

#[test]
fn ball_unfish_eyecolor_increments_mutation_count() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Ball, "Orb".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Orb".to_string(), &mut rng);
    tank.apply_named_mutation("Orb", "eyecolor");
    let record = tank.fish[0]
        .mutations
        .as_ref()
        .expect("mutation record must exist");
    assert_eq!(record.count, 1);
}

#[test]
fn ball_unfish_bodycolor_adds_to_mutation_history() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Ball, "Orb".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Orb".to_string(), &mut rng);
    tank.apply_named_mutation("Orb", "bodycolor");
    let record = tank.fish[0]
        .mutations
        .as_ref()
        .expect("mutation record must exist");
    assert_eq!(record.history.len(), 1);
    assert_eq!(record.history[0], "bodycolor");
}

#[test]
fn worm_mitosis_both_fish_have_partner_name() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Worm, "Wiggly".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Wiggly".to_string(), &mut rng);
    tank.apply_named_mutation("Wiggly", "telophase");
    tank.apply_named_mutation("Wiggly", "cytokinesis");
    assert_eq!(tank.fish.len(), 2, "mitosis should produce a second fish");
    let orig = &tank.fish[0];
    let child = &tank.fish[1];
    let orig_record = orig
        .mutations
        .as_ref()
        .expect("orig must have a mutation record");
    let child_record = child
        .mutations
        .as_ref()
        .expect("child must have a mutation record");
    assert!(
        orig_record.partners.contains(&child.name),
        "original fish must list child as mitosis partner"
    );
    assert!(
        child_record.partners.contains(&orig.name),
        "child fish must list original as mitosis partner"
    );
}

#[test]
fn worm_mitosis_without_doublefish_does_not_split() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Worm, "Wiggly".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Wiggly".to_string(), &mut rng);
    tank.apply_named_mutation("Wiggly", "cytokinesis");
    assert_eq!(
        tank.fish.len(),
        1,
        "mitosis on single worm should not spawn a second fish"
    );
}

#[test]
fn ball_unfish_colorpatch_adds_slime_color_patches() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Ball, "Orb".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Orb".to_string(), &mut rng);
    tank.apply_named_mutation("Orb", "colorpatch");
    let us = tank.fish[0]
        .unfish_state
        .as_ref()
        .expect("unfish_state must exist");
    assert!(
        !us.slime_color_patches.is_empty(),
        "colorpatch mutation must add at least one patch to ball unfish"
    );
}

#[test]
fn skull_unfish_colorpatch_adds_slime_color_patches() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(
        UnfishKind::Skull,
        "Skelly".to_string(),
        20.0,
        10.0,
        &mut rng,
    );
    tank.place_fish(fish, "Skelly".to_string(), &mut rng);
    tank.apply_named_mutation("Skelly", "colorpatch");
    let us = tank.fish[0]
        .unfish_state
        .as_ref()
        .expect("unfish_state must exist");
    assert!(
        !us.slime_color_patches.is_empty(),
        "colorpatch mutation must add at least one patch to skull unfish"
    );
}

#[test]
fn ball_unfish_colorpatch_increments_mutation_count() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Ball, "Orb".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Orb".to_string(), &mut rng);
    tank.apply_named_mutation("Orb", "colorpatch");
    let record = tank.fish[0]
        .mutations
        .as_ref()
        .expect("mutation record must exist");
    assert_eq!(record.count, 1);
}

#[test]
fn ball_unfish_colorpatch_adds_to_mutation_history() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Ball, "Orb".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Orb".to_string(), &mut rng);
    tank.apply_named_mutation("Orb", "colorpatch");
    let record = tank.fish[0]
        .mutations
        .as_ref()
        .expect("mutation record must exist");
    assert_eq!(record.history.len(), 1);
    assert_eq!(record.history[0], "colorpatch");
}

#[test]
fn ball_unfish_colorpatch_patches_within_ball_width() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Ball, "Orb".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Orb".to_string(), &mut rng);
    tank.apply_named_mutation("Orb", "colorpatch");
    let us = tank.fish[0]
        .unfish_state
        .as_ref()
        .expect("unfish_state must exist");
    for &(pos, _) in &us.slime_color_patches {
        assert!(
            pos < fishtank::fishes::unfish::BALL_WIDTH as usize,
            "colorpatch pos {pos} must be within BALL_WIDTH"
        );
    }
}

#[test]
fn skull_unfish_colorpatch_patches_within_skull_width() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(
        UnfishKind::Skull,
        "Skelly".to_string(),
        20.0,
        10.0,
        &mut rng,
    );
    tank.place_fish(fish, "Skelly".to_string(), &mut rng);
    tank.apply_named_mutation("Skelly", "colorpatch");
    let us = tank.fish[0]
        .unfish_state
        .as_ref()
        .expect("unfish_state must exist");
    for &(pos, _) in &us.slime_color_patches {
        assert!(
            pos < fishtank::fishes::unfish::SKULL_WIDTH as usize,
            "colorpatch pos {pos} must be within SKULL_WIDTH"
        );
    }
}

#[test]
fn worm_unfish_colorpatch_adds_slime_color_patches() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Worm, "Wiggly".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Wiggly".to_string(), &mut rng);
    tank.apply_named_mutation("Wiggly", "colorpatch");
    let us = tank.fish[0]
        .unfish_state
        .as_ref()
        .expect("unfish_state must exist");
    assert!(
        !us.slime_color_patches.is_empty(),
        "colorpatch mutation must add at least one patch to worm unfish"
    );
}

#[test]
fn worm_unfish_colorpatch_patches_within_display_width() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Worm, "Wiggly".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Wiggly".to_string(), &mut rng);
    tank.apply_named_mutation("Wiggly", "colorpatch");
    let dw = tank.fish[0].display_width;
    let us = tank.fish[0]
        .unfish_state
        .as_ref()
        .expect("unfish_state must exist");
    for &(pos, _) in &us.slime_color_patches {
        assert!(
            pos < dw,
            "colorpatch pos {pos} must be within display_width {dw}"
        );
    }
}

#[test]
fn reversed_unfish_colorpatch_adds_slime_color_patches() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(
        UnfishKind::Reversed,
        "Flip".to_string(),
        20.0,
        10.0,
        &mut rng,
    );
    tank.place_fish(fish, "Flip".to_string(), &mut rng);
    tank.apply_named_mutation("Flip", "colorpatch");
    let us = tank.fish[0]
        .unfish_state
        .as_ref()
        .expect("unfish_state must exist");
    assert!(
        !us.slime_color_patches.is_empty(),
        "colorpatch mutation must add at least one patch to reversed unfish"
    );
}

#[test]
fn reversed_unfish_colorpatch_patches_within_display_width() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(
        UnfishKind::Reversed,
        "Flip".to_string(),
        20.0,
        10.0,
        &mut rng,
    );
    tank.place_fish(fish, "Flip".to_string(), &mut rng);
    tank.apply_named_mutation("Flip", "colorpatch");
    let dw = tank.fish[0].display_width;
    let us = tank.fish[0]
        .unfish_state
        .as_ref()
        .expect("unfish_state must exist");
    for &(pos, _) in &us.slime_color_patches {
        assert!(
            pos < dw,
            "colorpatch pos {pos} must be within display_width {dw}"
        );
    }
}

#[test]
fn blinker_unfish_colorpatch_adds_slime_color_patches() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(
        UnfishKind::Blinker,
        "Blink".to_string(),
        20.0,
        10.0,
        &mut rng,
    );
    tank.place_fish(fish, "Blink".to_string(), &mut rng);
    tank.apply_named_mutation("Blink", "colorpatch");
    let us = tank.fish[0]
        .unfish_state
        .as_ref()
        .expect("unfish_state must exist");
    assert!(
        !us.slime_color_patches.is_empty(),
        "colorpatch mutation must add at least one patch to blinker unfish"
    );
}

#[test]
fn blinker_unfish_colorpatch_patches_within_display_width() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(
        UnfishKind::Blinker,
        "Blink".to_string(),
        20.0,
        10.0,
        &mut rng,
    );
    tank.place_fish(fish, "Blink".to_string(), &mut rng);
    tank.apply_named_mutation("Blink", "colorpatch");
    let dw = tank.fish[0].display_width;
    let us = tank.fish[0]
        .unfish_state
        .as_ref()
        .expect("unfish_state must exist");
    for &(pos, _) in &us.slime_color_patches {
        assert!(
            pos < dw,
            "colorpatch pos {pos} must be within display_width {dw}"
        );
    }
}

#[test]
fn doppleganger_unfish_colorpatch_adds_slime_color_patches() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(
        UnfishKind::Doppleganger,
        "Doppel".to_string(),
        20.0,
        10.0,
        &mut rng,
    );
    tank.place_fish(fish, "Doppel".to_string(), &mut rng);
    tank.apply_named_mutation("Doppel", "colorpatch");
    let us = tank.fish[0]
        .unfish_state
        .as_ref()
        .expect("unfish_state must exist");
    assert!(
        !us.slime_color_patches.is_empty(),
        "colorpatch mutation must add at least one patch to doppleganger unfish"
    );
}

#[test]
fn doppleganger_unfish_colorpatch_patches_within_display_width() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(
        UnfishKind::Doppleganger,
        "Doppel".to_string(),
        20.0,
        10.0,
        &mut rng,
    );
    tank.place_fish(fish, "Doppel".to_string(), &mut rng);
    tank.apply_named_mutation("Doppel", "colorpatch");
    let dw = tank.fish[0].display_width;
    let us = tank.fish[0]
        .unfish_state
        .as_ref()
        .expect("unfish_state must exist");
    for &(pos, _) in &us.slime_color_patches {
        assert!(
            pos < dw,
            "colorpatch pos {pos} must be within display_width {dw}"
        );
    }
}

#[test]
fn phantom_unfish_colorpatch_adds_slime_color_patches() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(
        UnfishKind::Phantom,
        "Ghost".to_string(),
        20.0,
        10.0,
        &mut rng,
    );
    tank.place_fish(fish, "Ghost".to_string(), &mut rng);
    tank.apply_named_mutation("Ghost", "colorpatch");
    let us = tank.fish[0]
        .unfish_state
        .as_ref()
        .expect("unfish_state must exist");
    assert!(
        !us.slime_color_patches.is_empty(),
        "colorpatch mutation must add at least one patch to phantom unfish"
    );
}

#[test]
fn phantom_unfish_colorpatch_patches_within_display_width() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(
        UnfishKind::Phantom,
        "Ghost".to_string(),
        20.0,
        10.0,
        &mut rng,
    );
    tank.place_fish(fish, "Ghost".to_string(), &mut rng);
    tank.apply_named_mutation("Ghost", "colorpatch");
    let dw = tank.fish[0].display_width;
    let us = tank.fish[0]
        .unfish_state
        .as_ref()
        .expect("unfish_state must exist");
    for &(pos, _) in &us.slime_color_patches {
        assert!(
            pos < dw,
            "colorpatch pos {pos} must be within display_width {dw}"
        );
    }
}

#[test]
fn strawberry_command_pinks_fish_and_adds_sell_bonus() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Berry".to_string(), &mut rng);
    tank.apply_named_mutation("Berry", "strawberry");
    let fish = &tank.fish[0];
    assert_eq!(
        fish.color, PINK,
        "strawberry command recolors the fish pink"
    );
    assert_eq!(
        fish.sell_price_bonus_pct, STRAWBERRY_SELL_BONUS_PCT,
        "strawberry command grants the same sell bonus as drinking the milk"
    );
}

#[test]
fn alienation_command_recolors_fish_light_green() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Probe".to_string(), &mut rng);
    tank.apply_named_mutation("Probe", "alienation");
    assert_eq!(
        tank.fish[0].color, LIGHT_GREEN,
        "alienation command recolors the fish light green"
    );
}

#[test]
fn wakecolor_sets_the_mutant_wake_colour_on_a_standard_fish() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Fizz".to_string(), &mut rng);
    tank.apply_named_mutation("Fizz", "wakecolor");
    let m = tank.fish[0]
        .mutant
        .as_ref()
        .expect("mutant should exist after wakecolor");
    assert!(
        m.wake_color.is_some(),
        "wakecolor must set the mutant wake_color"
    );
}

#[test]
fn an_unfish_never_zooms_so_it_has_no_wake_to_colour() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Ball, "Orb".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Orb".to_string(), &mut rng);
    assert!(!tank.apply_named_mutation("Orb", "wakecolor"));
}

#[test]
fn nightowl_sets_circadian_and_cannot_be_stacked() {
    use fishtank::fishes::mutant::Circadian;
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Owl".to_string(), &mut rng);
    assert!(tank.apply_named_mutation("Owl", "nightowl"));
    assert_eq!(tank.fish[0].circadian_state(), Circadian::NightOwl);
    assert!(
        !tank.apply_named_mutation("Owl", "nightowl"),
        "nightowl cannot be stacked once already a night owl"
    );
}

#[test]
fn helpedbygod_cancels_nightowl() {
    use fishtank::fishes::mutant::Circadian;
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Saint".to_string(), &mut rng);
    tank.apply_named_mutation("Saint", "nightowl");
    assert!(tank.apply_named_mutation("Saint", "helpedbygod"));
    assert_eq!(
        tank.fish[0].circadian_state(),
        Circadian::HelpedByGod,
        "helpedbygod replaces nightowl since they cancel each other"
    );
}

#[test]
fn heterochromia_flags_mutant_and_colors_each_eye() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Iris".to_string(), &mut rng);
    tank.apply_named_mutation("Iris", "heterochromia");
    let m = tank.fish[0]
        .mutant
        .as_ref()
        .expect("mutant should exist after heterochromia");
    assert!(m.heterochromia, "heterochromia must set the flag");
    assert!(
        m.left_eyes.iter().all(|e| e.color.is_some()),
        "every eye must get its own color"
    );
}

#[test]
fn eyecolor_after_heterochromia_keeps_global_eye_color_unset() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Iris".to_string(), &mut rng);
    tank.apply_named_mutation("Iris", "heterochromia");
    tank.apply_named_mutation("Iris", "eyecolor");
    let m = tank.fish[0].mutant.as_ref().unwrap();
    assert!(
        m.eye_color.is_none(),
        "once heterochromatic, eyecolor recolors a single eye, not the global eye color"
    );
}

#[test]
fn heterochromia_sets_flag_on_ball_unfish() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Ball, "Orb".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Orb".to_string(), &mut rng);
    tank.apply_named_mutation("Orb", "heterochromia");
    let us = tank.fish[0]
        .unfish_state
        .as_ref()
        .expect("unfish_state must exist");
    assert!(us.heterochromia, "heterochromia must set the unfish flag");
}

#[test]
fn worm_accepts_heterochromia_and_colors_each_eye() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Worm, "Wiggly".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Wiggly".to_string(), &mut rng);
    assert!(
        tank.apply_named_mutation("Wiggly", "heterochromia"),
        "worm now supports heterochromia"
    );
    let us = tank.fish[0]
        .unfish_state
        .as_ref()
        .expect("unfish_state must exist");
    assert!(us.heterochromia, "heterochromia must set the unfish flag");
    assert_eq!(
        us.worm_eye_colors.len(),
        us.worm_eye_count(),
        "every worm eye gets an independent color"
    );
    assert!(
        us.worm_eye_colors.iter().all(|c| c.is_some()),
        "every worm eye color must be assigned"
    );
}

#[test]
fn worm_eyeincrease_after_heterochromia_colors_new_eye() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Worm, "Wiggly".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Wiggly".to_string(), &mut rng);
    tank.apply_named_mutation("Wiggly", "heterochromia");
    tank.apply_named_mutation("Wiggly", "eyeincrease");
    let us = tank.fish[0].unfish_state.as_ref().unwrap();
    assert_eq!(
        us.worm_eye_colors.len(),
        us.worm_eye_count(),
        "growing the eye count resyncs the per-eye color list"
    );
    assert!(
        us.worm_eye_colors.iter().all(|c| c.is_some()),
        "the newly grown eye is colored too"
    );
}

fn segment_display_width(segs: &[(char, ratatui::style::Color)]) -> usize {
    use unicode_width::UnicodeWidthChar;
    segs.iter()
        .map(|(c, _)| UnicodeWidthChar::width(*c).unwrap_or(1))
        .sum()
}

#[test]
fn ear_adds_left_glyph_behind_eye_and_widens_standard_fish() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Vincent".to_string(), &mut rng);
    let before = tank.fish[0].display_width;
    assert!(tank.apply_named_mutation("Vincent", "ear"));
    let fish = &tank.fish[0];
    assert_eq!(
        fish.mutant.as_ref().unwrap().ear_count,
        1,
        "ear stacks the count"
    );
    assert_eq!(
        fish.display_width,
        before + 1,
        "an ear widens the fish by exactly one cell"
    );
    let segs = fish.static_left_segments();
    let ear_pos = segs
        .iter()
        .position(|&(c, _)| c == 'Ɛ')
        .expect("a left-facing fish wears the Ɛ ear");
    assert!(
        ear_pos > 0,
        "the ear sits behind the eye, never at the mouth"
    );
    assert!(
        !segs.iter().any(|&(c, _)| c == '3'),
        "a left-facing fish never wears the right-facing 3 ear"
    );
    assert_eq!(
        segment_display_width(&segs),
        fish.display_width,
        "display_width must match the real rendered width once ears are added"
    );
}

#[test]
fn ear_color_pink_by_default_then_earcolor_recolors() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Vincent".to_string(), &mut rng);
    tank.apply_named_mutation("Vincent", "ear");
    let segs = tank.fish[0].static_left_segments();
    let ear_color = segs.iter().find(|&&(c, _)| c == 'Ɛ').map(|&(_, col)| col);
    assert_eq!(
        ear_color,
        Some(PINK),
        "ears start pink until earcolor is applied"
    );
    assert!(tank.apply_named_mutation("Vincent", "earcolor"));
    assert!(
        tank.fish[0].mutant.as_ref().unwrap().ear_color.is_some(),
        "earcolor assigns an explicit ear color"
    );
}

#[test]
fn revert_and_earcolor_are_locked_until_an_ear_exists() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Vincent".to_string(), &mut rng);
    assert!(
        !tank.apply_named_mutation("Vincent", "revert"),
        "revert is locked while the fish has no mutation"
    );
    assert!(
        !tank.apply_named_mutation("Vincent", "earcolor"),
        "earcolor is locked while the fish has no ears"
    );
    tank.apply_named_mutation("Vincent", "ear");
    let widened = tank.fish[0].display_width;
    assert!(tank.apply_named_mutation("Vincent", "revert"));
    let fish = &tank.fish[0];
    assert_eq!(fish.mutant.as_ref().map_or(0, |m| m.ear_count), 0);
    assert_eq!(
        fish.display_width,
        widened - 1,
        "reverting the ear shrinks the fish back"
    );
    assert!(
        !tank.apply_named_mutation("Vincent", "revert"),
        "a revert is never itself reverted"
    );
}

#[test]
fn ball_mutation_does_not_corrupt_its_fixed_width() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Ball, "Orb".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Orb".to_string(), &mut rng);
    let before = tank.fish[0].display_width;
    tank.apply_named_mutation("Orb", "eyecolor");
    assert_eq!(
        tank.fish[0].display_width, before,
        "a multi-row ball keeps its fixed display_width across mutations"
    );
}

#[test]
fn ball_ear_fills_every_border_row_then_locks() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Ball, "Orb".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Orb".to_string(), &mut rng);
    let width_before = tank.fish[0].display_width;
    let mut applied = 0;
    while tank.apply_named_mutation("Orb", "ear") {
        applied += 1;
        assert!(
            applied <= BALL_HEIGHT as usize + 1,
            "ear must stop at the cap"
        );
    }
    let fish = &tank.fish[0];
    assert_eq!(
        applied, BALL_HEIGHT as usize,
        "a ball fills exactly one ear per border row then locks"
    );
    assert_eq!(
        fish.unfish_state.as_ref().unwrap().ear_count,
        BALL_HEIGHT as usize
    );
    assert_eq!(
        fish.display_width, width_before,
        "border ears overhang the ball, they do not change its fixed width"
    );
}

#[test]
fn skull_ear_cap_matches_its_row_count() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Skull, "Skull".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Skull".to_string(), &mut rng);
    let mut applied = 0;
    while tank.apply_named_mutation("Skull", "ear") {
        applied += 1;
        assert!(
            applied <= SKULL_HEIGHT as usize + 1,
            "ear must stop at the cap"
        );
    }
    assert_eq!(applied, SKULL_HEIGHT as usize);
    assert!(
        tank.apply_named_mutation("Skull", "revert"),
        "revert unlocks once the skull has ears"
    );
    assert!(
        tank.apply_named_mutation("Skull", "ear"),
        "freeing a border row re-opens the ear slot"
    );
}

#[test]
fn worm_ear_widens_and_appears_in_sprite() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Worm, "Wiggly".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Wiggly".to_string(), &mut rng);
    let before = tank.fish[0].display_width;
    assert!(tank.apply_named_mutation("Wiggly", "ear"));
    let fish = &tank.fish[0];
    assert_eq!(fish.unfish_state.as_ref().unwrap().ear_count, 1);
    assert_eq!(fish.display_width, before + 1);
    let segs = fish.static_left_segments();
    assert!(
        segs.iter().any(|&(c, _)| c == 'Ɛ'),
        "the worm grows a visible ear"
    );
    assert_eq!(
        segment_display_width(&segs),
        fish.display_width,
        "worm display_width must match its rendered width with ears"
    );
}

fn eye_glyph(c: char) -> bool {
    matches!(c, 'º' | '¯')
}

#[test]
fn hydra_adds_in_body_eyes_and_widens_the_standard_fish() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Hydro".to_string(), &mut rng);
    for _ in 0..6 {
        tank.apply_named_mutation("Hydro", "sizeincrease");
    }
    let before_w = tank.fish[0].display_width;
    assert!(tank.apply_named_mutation("Hydro", "hydra"));
    let fish = &tank.fish[0];
    let added = fish.mutant.as_ref().unwrap().hydra_eyes.len();
    assert!(
        (1..=2).contains(&added),
        "hydra spawns one or two extra eyes per application"
    );
    assert_eq!(
        fish.display_width,
        before_w + added,
        "every in-body hydra eye widens the fish by one cell"
    );
    let segs = fish.static_left_segments();
    assert_eq!(
        segment_display_width(&segs),
        fish.display_width,
        "display_width must match the rendered width once hydra eyes are added"
    );
}

#[test]
fn hydra_eyes_stay_at_least_one_cell_from_every_other_eye() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Hydro".to_string(), &mut rng);
    for _ in 0..6 {
        tank.apply_named_mutation("Hydro", "sizeincrease");
    }
    for _ in 0..3 {
        tank.apply_named_mutation("Hydro", "hydra");
    }
    let segs = tank.fish[0].static_left_segments();
    let eye_cols: Vec<usize> = segs
        .iter()
        .enumerate()
        .filter(|&(_, &(c, _))| eye_glyph(c))
        .map(|(i, _)| i)
        .collect();
    assert!(
        eye_cols.len() >= 2,
        "the head eye plus hydra eyes are present"
    );
    assert!(
        max_adjacent_run(&eye_cols) <= 2,
        "hydra eyes cluster into heads of at most two adjacent eyes, separated from the rest"
    );
}

fn max_adjacent_run(cols: &[usize]) -> usize {
    let mut best = if cols.is_empty() { 0 } else { 1 };
    let mut run = best;
    for pair in cols.windows(2) {
        if pair[1] - pair[0] == 1 {
            run += 1;
            best = best.max(run);
        } else {
            run = 1;
        }
    }
    best
}

#[test]
fn hydra_fills_to_body_capacity_then_locks() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Hydro".to_string(), &mut rng);
    for _ in 0..6 {
        tank.apply_named_mutation("Hydro", "sizeincrease");
    }
    let mut applied = 0;
    while tank.apply_named_mutation("Hydro", "hydra") {
        applied += 1;
        assert!(applied <= 50, "hydra must stop at the body capacity");
    }
    let fish = &tank.fish[0];
    assert_eq!(
        fish.mutant.as_ref().unwrap().hydra_eyes.len(),
        fish.body_size - 1,
        "a line fish hosts one fewer hydra eye than its body cells"
    );
    assert_eq!(
        segment_display_width(&fish.static_left_segments()),
        fish.display_width,
        "the rendered width matches display_width even when hydra is maxed"
    );
}

#[test]
fn reverting_growth_sheds_excess_hydra_eyes() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Hydro".to_string(), &mut rng);
    for _ in 0..6 {
        tank.apply_named_mutation("Hydro", "sizeincrease");
    }
    while tank.apply_named_mutation("Hydro", "hydra") {}
    for _ in 0..4 {
        tank.apply_named_mutation("Hydro", "revert");
    }
    let fish = &tank.fish[0];
    assert!(
        fish.mutant.as_ref().unwrap().hydra_eyes.len() <= fish.body_size.saturating_sub(1),
        "shrinking the body never leaves more hydra eyes than it can host"
    );
    assert_eq!(
        segment_display_width(&fish.static_left_segments()),
        fish.display_width,
        "width stays consistent after the body shrinks beneath the hydra count"
    );
}

#[test]
fn hydra_widens_a_slime_unfish_and_keeps_width_consistent() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(
        UnfishKind::Reversed,
        "Goo".to_string(),
        20.0,
        10.0,
        &mut rng,
    );
    tank.place_fish(fish, "Goo".to_string(), &mut rng);
    let before = tank.fish[0].display_width;
    assert!(tank.apply_named_mutation("Goo", "hydra"));
    let fish = &tank.fish[0];
    let added = fish.unfish_state.as_ref().unwrap().hydra_count;
    assert!(
        (1..=2).contains(&added),
        "a slime unfish grows one or two hydra eyes per application"
    );
    assert_eq!(fish.display_width, before + added);
    assert_eq!(
        segment_display_width(&fish.static_left_segments()),
        fish.display_width,
        "slime display_width must match its rendered width with hydra eyes"
    );
}

#[test]
fn hydra_eyes_on_a_slime_unfish_keep_their_spacing() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(
        UnfishKind::Doppleganger,
        "Goo".to_string(),
        20.0,
        10.0,
        &mut rng,
    );
    tank.place_fish(fish, "Goo".to_string(), &mut rng);
    while tank.apply_named_mutation("Goo", "hydra") {}
    let segs = tank.fish[0].static_left_segments();
    let eye_cols: Vec<usize> = segs
        .iter()
        .enumerate()
        .filter(|&(_, &(c, _))| c == 'º')
        .map(|(i, _)| i)
        .collect();
    assert!(
        eye_cols.len() >= 2,
        "the head eye plus hydra eyes are present"
    );
    assert!(
        max_adjacent_run(&eye_cols) <= 2,
        "slime hydra eyes cluster into heads of at most two adjacent eyes"
    );
}

#[test]
fn hydra_adds_body_eyes_to_the_worm_and_keeps_width() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Worm, "Wiggly".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Wiggly".to_string(), &mut rng);
    let before = tank.fish[0].display_width;
    let head_eyes = tank.fish[0]
        .static_left_segments()
        .iter()
        .filter(|&&(c, _)| c == '0')
        .count();
    assert!(tank.apply_named_mutation("Wiggly", "hydra"));
    let fish = &tank.fish[0];
    let added = fish.unfish_state.as_ref().unwrap().hydra_count;
    assert!(
        (1..=2).contains(&added),
        "the worm grows one or two hydra eyes per application"
    );
    assert_eq!(fish.display_width, before + added);
    let segs = fish.static_left_segments();
    assert_eq!(
        segment_display_width(&segs),
        fish.display_width,
        "worm display_width must match its rendered width with hydra eyes"
    );
    assert_eq!(
        segs.iter().filter(|&&(c, _)| c == '0').count(),
        head_eyes + added,
        "each hydra eye shows an extra worm eye in the body"
    );
}

#[test]
fn hydra_fills_the_worm_to_segment_capacity_then_locks() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Worm, "Wiggly".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Wiggly".to_string(), &mut rng);
    let segments = tank.fish[0].unfish_state.as_ref().unwrap().worm_segments;
    let mut applied = 0;
    while tank.apply_named_mutation("Wiggly", "hydra") {
        applied += 1;
        assert!(applied <= 50, "hydra must stop at the segment capacity");
    }
    let fish = &tank.fish[0];
    assert_eq!(
        fish.unfish_state.as_ref().unwrap().hydra_count,
        segments - 1,
        "the worm hosts one fewer hydra eye than its segments"
    );
    assert_eq!(
        segment_display_width(&fish.static_left_segments()),
        fish.display_width
    );
}

#[test]
fn hydra_is_unavailable_on_ball_and_skull() {
    let mut tank = make_tank();
    let mut rng = rng();
    let ball = Fish::new_unfish(UnfishKind::Ball, "Orb".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(ball, "Orb".to_string(), &mut rng);
    let skull = Fish::new_unfish(UnfishKind::Skull, "Bone".to_string(), 40.0, 10.0, &mut rng);
    tank.place_fish(skull, "Bone".to_string(), &mut rng);
    assert!(
        !tank.apply_named_mutation("Orb", "hydra"),
        "a ball has no head, so hydra never applies"
    );
    assert!(
        !tank.apply_named_mutation("Bone", "hydra"),
        "a skull has no head, so hydra never applies"
    );
}

#[test]
fn cow_hydra_spawns_a_head_inside_the_torso_without_widening() {
    let mut tank = make_tank();
    let mut rng = rng();
    let name = tank.spawn_cow(CowVariant::Brown, &mut rng);
    let before_w = tank.cows[0].display_width;
    let carets_before = {
        let cow = &tank.cows[0];
        let top = cow.sprite_top_offset() as usize;
        cow_sprite(cow)[top]
            .iter()
            .filter(|&&(c, _)| c == '^')
            .count()
    };
    assert!(tank.apply_named_mutation(&name, "hydra"));
    let cow = &tank.cows[0];
    assert_eq!(cow.mutant.hydra_eyes.len(), 1, "one hydra head spawns");
    assert_eq!(
        cow.display_width, before_w,
        "the hydra head sits inside the torso, so the cow keeps its width"
    );
    let top = cow.sprite_top_offset() as usize;
    let carets_after = cow_sprite(cow)[top]
        .iter()
        .filter(|&&(c, _)| c == '^')
        .count();
    assert_eq!(
        carets_after,
        carets_before + 2,
        "the hydra head adds its own ^__^ crown above the torso"
    );
}

#[test]
fn cow_grows_spikes_on_its_back_and_gates_its_followups() {
    let mut tank = make_tank();
    let mut rng = rng();
    let name = tank.spawn_cow(CowVariant::Brown, &mut rng);
    assert!(
        !tank.apply_named_mutation(&name, "revert"),
        "revert is locked until the cow has a mutation"
    );
    let before = tank.cows[0].display_width;
    assert!(
        !tank.apply_named_mutation(&name, "tentacles"),
        "a cow stands on its legs: the bottom band is taken"
    );
    assert!(tank.apply_named_mutation(&name, "spikes"));
    let ext = tank.cows[0].mutant.body_extension.unwrap();
    assert_eq!(ext.top, Some(ExtensionVariant::Spike));
    assert_eq!(ext.bottom, None);
    assert_eq!(
        tank.cows[0].display_width, before,
        "extensions rise above the torso and never widen the cow"
    );
}

#[test]
fn cow_hydra_fills_to_torso_capacity_then_locks() {
    let mut tank = make_tank();
    let mut rng = rng();
    let name = tank.spawn_cow(CowVariant::Brown, &mut rng);
    for _ in 0..8 {
        tank.apply_named_mutation(&name, "sizeincrease");
    }
    let cap = cow_hydra_capacity(&tank.cows[0]);
    assert!(cap >= 2, "a maxed torso hosts at least two hydra heads");
    let mut applied = 0;
    while tank.apply_named_mutation(&name, "hydra") {
        applied += 1;
        assert!(applied <= 20, "hydra must stop at the torso capacity");
    }
    assert_eq!(tank.cows[0].mutant.hydra_eyes.len(), cap);
}

#[test]
fn cow_hydra_is_invalid_while_doubled() {
    let mut tank = make_tank();
    let mut rng = rng();
    let name = tank.spawn_cow(CowVariant::Brown, &mut rng);
    tank.apply_named_mutation(&name, "telophase");
    assert!(
        tank.cows[0].mutant.is_double,
        "telophase doubles the cow first"
    );
    assert!(
        !tank.apply_named_mutation(&name, "hydra"),
        "a doubled cow has no clear torso for a hydra head"
    );
}

#[test]
fn cow_reverting_growth_sheds_excess_hydra_heads() {
    let mut tank = make_tank();
    let mut rng = rng();
    let name = tank.spawn_cow(CowVariant::Brown, &mut rng);
    for _ in 0..8 {
        tank.apply_named_mutation(&name, "sizeincrease");
    }
    while tank.apply_named_mutation(&name, "hydra") {}
    for _ in 0..8 {
        tank.apply_named_mutation(&name, "revert");
    }
    let cow = &tank.cows[0];
    assert!(
        cow.mutant.hydra_eyes.len() <= cow_hydra_capacity(cow),
        "reverting growth sheds hydra heads the torso can no longer host"
    );
    let _ = cow_sprite(cow);
}

#[test]
fn every_mutation_token_parses_back_to_itself() {
    use fishtank::fishes::mutations::Mutation;
    for &m in Mutation::ALL {
        assert_eq!(
            Mutation::parse(m.token()),
            Some(m),
            "token {} must round-trip through parse",
            m.token()
        );
    }
}

#[test]
fn telophase_then_cytokinesis_splits_a_standard_fish() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Split".to_string(), &mut rng);
    tank.apply_named_mutation("Split", "telophase");
    assert!(
        tank.fish[0].mutant.as_ref().unwrap().is_double,
        "telophase doubles the fish"
    );
    tank.apply_named_mutation("Split", "cytokinesis");
    assert_eq!(
        tank.fish.len(),
        2,
        "cytokinesis on a doubled fish splits it in two"
    );
    assert!(
        !tank.fish[0].mutant.as_ref().unwrap().is_double,
        "the parent is no longer doubled once it has split"
    );
}

#[test]
fn cytokinesis_without_telophase_does_not_split() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Solo".to_string(), &mut rng);
    let applied = tank.apply_named_mutation("Solo", "cytokinesis");
    assert!(
        !applied,
        "cytokinesis is rejected when the fish is not doubled"
    );
    assert_eq!(tank.fish.len(), 1);
}

fn is_foot(c: char) -> bool {
    c == '"' || c == '^'
}

#[test]
fn feet_add_a_row_below_the_body_without_changing_width() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Steps".to_string(), &mut rng);
    let before_width = tank.fish[0].display_width;
    assert!(tank.apply_named_mutation("Steps", "feet"));
    let fish = &tank.fish[0];
    assert!(
        fish.mutant.as_ref().unwrap().feet.is_some(),
        "feet are recorded on the mutant state"
    );
    assert_eq!(
        fish.display_width, before_width,
        "feet sit below the body and never change the rendered width"
    );
    let sprite = fish.line_sprite();
    assert_eq!(sprite.body_row, 0, "the body stays on the top row");
    assert_eq!(sprite.rows.len(), 2, "a feet row is added below the body");
    assert_eq!(
        sprite.rows[0],
        fish.segments(),
        "the body row is exactly the entity's segments"
    );
    assert!(
        sprite.rows[1].iter().any(|&(c, _)| is_foot(c)),
        "the feet row carries the foot glyphs"
    );
}

#[test]
fn feet_copy_the_color_of_the_cell_directly_above() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Mirror".to_string(), &mut rng);
    tank.apply_named_mutation("Mirror", "feet");
    let sprite = tank.fish[0].line_sprite();
    let body = &sprite.rows[0];
    for (col, &(c, color)) in sprite.rows[1].iter().enumerate() {
        if is_foot(c) {
            assert_eq!(
                color, body[col].1,
                "a foot copies the color of the body cell directly above it"
            );
        }
    }
}

#[test]
fn feetcolor_is_locked_until_feet_exist_and_goes_with_them() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Bare".to_string(), &mut rng);
    assert!(
        !tank.apply_named_mutation("Bare", "feetcolor"),
        "feetcolor is locked while the fish has no feet"
    );
    tank.apply_named_mutation("Bare", "feet");
    assert!(
        !tank.apply_named_mutation("Bare", "feet"),
        "feet cannot be applied twice"
    );
    assert!(tank.apply_named_mutation("Bare", "feetcolor"));
    assert!(
        tank.fish[0]
            .mutant
            .as_ref()
            .unwrap()
            .feet
            .unwrap()
            .color
            .is_some(),
        "feetcolor assigns an explicit foot color"
    );
    while tank.fish[0].feet().is_some() {
        assert!(tank.apply_named_mutation("Bare", "revert"));
    }
    assert!(
        tank.fish[0]
            .mutations
            .as_ref()
            .unwrap()
            .history
            .iter()
            .all(|token| token != "feetcolor"),
        "a foot colour leaves with the feet it coloured"
    );
    assert_eq!(
        tank.fish[0].line_sprite().rows.len(),
        1,
        "with no feet the sprite is a single body row again"
    );
}

#[test]
fn slime_unfish_grows_a_feet_row_below_its_body() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(
        UnfishKind::Reversed,
        "Slick".to_string(),
        20.0,
        10.0,
        &mut rng,
    );
    tank.place_fish(fish, "Slick".to_string(), &mut rng);
    let before = tank.fish[0].display_width;
    assert!(tank.apply_named_mutation("Slick", "feet"));
    let fish = &tank.fish[0];
    assert!(fish.unfish_state.as_ref().unwrap().feet.is_some());
    assert_eq!(
        fish.display_width, before,
        "feet never change a slime unfish's width"
    );
    let sprite = fish.line_sprite();
    assert_eq!(sprite.rows.len(), 2);
    assert!(sprite.rows[1].iter().any(|&(c, _)| is_foot(c)));
}

#[test]
fn ball_feet_are_recorded_and_keep_the_fixed_width() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Ball, "Orb".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Orb".to_string(), &mut rng);
    let before = tank.fish[0].display_width;
    assert!(tank.apply_named_mutation("Orb", "feet"));
    assert!(tank.fish[0].unfish_state.as_ref().unwrap().feet.is_some());
    assert_eq!(
        tank.fish[0].display_width, before,
        "feet hang below the ball and keep its fixed width"
    );
    assert!(
        !tank.apply_named_mutation("Orb", "feet"),
        "a ball cannot grow a second set of feet"
    );
    assert!(tank.apply_named_mutation("Orb", "revert"));
    assert!(tank.fish[0].unfish_state.as_ref().unwrap().feet.is_none());
}

#[test]
fn worm_feet_are_recorded_and_keep_its_width() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Worm, "Wiggly".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Wiggly".to_string(), &mut rng);
    let before = tank.fish[0].display_width;
    assert!(tank.apply_named_mutation("Wiggly", "feet"));
    assert!(tank.fish[0].unfish_state.as_ref().unwrap().feet.is_some());
    assert_eq!(
        tank.fish[0].display_width, before,
        "feet hang below the worm and never change its width"
    );
    assert!(tank.apply_named_mutation("Wiggly", "feetcolor"));
    assert!(
        tank.fish[0]
            .unfish_state
            .as_ref()
            .unwrap()
            .feet
            .unwrap()
            .color
            .is_some()
    );
}

fn force_extension(fish: &mut Fish, variant: ExtensionVariant) {
    let mut ext = BodyExtension::default();
    for &band in variant.bands() {
        ext.set(band, Some(variant));
    }
    fish.mutant.as_mut().unwrap().body_extension = Some(ext);
}

#[test]
fn spikes_grow_appendage_rows_without_changing_width() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Octo".to_string(), &mut rng);
    let before_width = tank.fish[0].display_width;
    assert!(tank.apply_named_mutation("Octo", "spikes"));
    let fish = &tank.fish[0];
    let ext = fish
        .mutant
        .as_ref()
        .unwrap()
        .body_extension
        .expect("extension recorded on the mutant state");
    assert_eq!(ext.top, Some(ExtensionVariant::Spike));
    assert_eq!(ext.bottom, Some(ExtensionVariant::Spike));
    assert_eq!(
        fish.display_width, before_width,
        "extensions live above/below the body and never change the rendered width"
    );
    let sprite = fish.line_sprite();
    assert_eq!(
        sprite.rows[sprite.body_row],
        fish.segments(),
        "the body row is exactly the entity's segments"
    );
    assert!(sprite.rows.len() > 1, "at least one appendage row is grown");
}

#[test]
fn tentacle_hangs_below_only_spike_sits_on_both_sides() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Mutantfish, "Reach".to_string(), &mut rng);

    force_extension(&mut tank.fish[0], ExtensionVariant::Tentacle);
    let sprite = tank.fish[0].line_sprite();
    assert_eq!(sprite.body_row, 0, "tentacles only hang below the body");
    assert_eq!(sprite.rows.len(), 3, "body plus two tentacle rows");
    assert!(
        sprite.rows[1].iter().any(|&(c, _)| c == '|'),
        "tentacle glyphs sit below the body"
    );

    force_extension(&mut tank.fish[0], ExtensionVariant::Spike);
    let sprite = tank.fish[0].line_sprite();
    assert_eq!(sprite.body_row, 1, "spikes flank the body top and bottom");
    assert_eq!(sprite.rows.len(), 3, "one spike row above, one below");
    assert!(sprite.rows[0].iter().any(|&(c, _)| c == '¦'));
    assert!(sprite.rows[2].iter().any(|&(c, _)| c == '¦'));
}

#[test]
fn wing_glyph_mirrors_top_versus_bottom() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Mutantfish, "Soar".to_string(), &mut rng);
    force_extension(&mut tank.fish[0], ExtensionVariant::Wing);
    let sprite = tank.fish[0].line_sprite();
    let top: Vec<char> = sprite.rows[0]
        .iter()
        .map(|&(c, _)| c)
        .filter(|&c| c == '/' || c == '\\')
        .collect();
    let bottom: Vec<char> = sprite.rows[2]
        .iter()
        .map(|&(c, _)| c)
        .filter(|&c| c == '/' || c == '\\')
        .collect();
    assert!(!top.is_empty() && !bottom.is_empty(), "wings on both bands");
    assert_ne!(
        top[0], bottom[0],
        "the top wing glyph is the mirror of the bottom wing glyph"
    );
}

#[test]
fn extension_followups_and_feet_exclusion_are_gated() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Gate".to_string(), &mut rng);
    assert!(tank.apply_named_mutation("Gate", "spikes"));
    assert!(
        !tank.apply_named_mutation("Gate", "feet"),
        "feet cannot grow where a spike holds the bottom band"
    );
}

#[test]
fn reverting_the_only_extension_clears_both_bands() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Salmon, "Trim".to_string(), &mut rng);
    assert!(tank.apply_named_mutation("Trim", "spikes"));
    assert!(tank.apply_named_mutation("Trim", "revert"));
    assert!(
        tank.fish[0].body_extension().is_empty(),
        "the spikes are gone from both bands"
    );
    assert_eq!(tank.fish[0].line_sprite().rows.len(), 1);
}

#[test]
fn feet_keep_the_bottom_band_and_spikes_take_the_top() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Booted".to_string(), &mut rng);
    assert!(tank.apply_named_mutation("Booted", "feet"));
    assert!(
        !tank.apply_named_mutation("Booted", "tentacles"),
        "tentacles need the bottom band the feet stand on"
    );
    assert!(tank.apply_named_mutation("Booted", "spikes"));
    let mutant = tank.fish[0].mutant.as_ref().unwrap();
    let ext = mutant.body_extension.unwrap();
    assert_eq!(ext.top, Some(ExtensionVariant::Spike));
    assert_eq!(ext.bottom, None, "the feet keep their band");
    assert!(mutant.feet.is_some());
}

#[test]
fn slime_unfish_grows_extension_rows_without_changing_width() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(
        UnfishKind::Reversed,
        "Slimy".to_string(),
        20.0,
        10.0,
        &mut rng,
    );
    tank.place_fish(fish, "Slimy".to_string(), &mut rng);
    let before = tank.fish[0].display_width;
    assert!(tank.apply_named_mutation("Slimy", "spikes"));
    assert!(
        tank.fish[0]
            .unfish_state
            .as_ref()
            .unwrap()
            .body_extension
            .is_some()
    );
    assert_eq!(
        tank.fish[0].display_width, before,
        "extensions never change a slime unfish's width"
    );
    assert!(
        tank.fish[0].line_sprite().rows.len() > 1,
        "the slime sprite grows appendage rows"
    );
}

#[test]
fn ball_extension_is_recorded_and_excludes_feet() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Ball, "Orb".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Orb".to_string(), &mut rng);
    let before = tank.fish[0].display_width;
    assert!(tank.apply_named_mutation("Orb", "spikes"));
    assert!(
        tank.fish[0]
            .unfish_state
            .as_ref()
            .unwrap()
            .body_extension
            .is_some()
    );
    assert_eq!(
        tank.fish[0].display_width, before,
        "a ball's extensions live above/below and keep its fixed width"
    );
    assert!(
        !tank.apply_named_mutation("Orb", "feet"),
        "feet cannot grow on a ball that already has a body extension"
    );
}

#[test]
fn worm_extension_is_recorded_and_keeps_its_width() {
    let mut tank = make_tank();
    let mut rng = rng();
    let fish = Fish::new_unfish(UnfishKind::Worm, "Squirm".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(fish, "Squirm".to_string(), &mut rng);
    let before = tank.fish[0].display_width;
    assert!(tank.apply_named_mutation("Squirm", "spikes"));
    assert!(
        tank.fish[0]
            .unfish_state
            .as_ref()
            .unwrap()
            .body_extension
            .is_some()
    );
    assert_eq!(
        tank.fish[0].display_width, before,
        "a worm's extensions never change its width"
    );
}

#[test]
fn jellyfish_rejects_a_mutation_outside_its_capability_set() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Jellyfish, "Jelly".to_string(), &mut rng);
    let before = tank.fish[0].body_size;
    let applied = tank.apply_named_mutation("Jelly", "sizeincrease");
    assert!(
        !applied,
        "sizeincrease is not in a jellyfish's capability set, so it is rejected"
    );
    assert_eq!(tank.fish[0].body_size, before);
}

#[test]
fn backwardstelophase_sets_double_and_backwards_flag() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Janus".to_string(), &mut rng);
    assert!(tank.apply_named_mutation("Janus", "backwardstelophase"));
    let m = tank.fish[0].mutant.as_ref().unwrap();
    assert!(m.is_double, "backwardstelophase makes the fish a double");
    assert!(m.backwards, "and flags it as the tail-joined form");
}

#[test]
fn backwardstelophase_display_width_matches_rendered_width() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Janus".to_string(), &mut rng);
    tank.apply_named_mutation("Janus", "backwardstelophase");
    let fish = &tank.fish[0];
    let segs = fish.static_left_segments();
    assert_eq!(
        segment_display_width(&segs),
        fish.display_width,
        "backwardstelophase display_width must equal the real rendered width"
    );
}

#[test]
fn backwardstelophase_renders_tails_not_head_eyes() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Solo".to_string(), &mut rng);
    let single = tank.fish[0].static_left_segments();
    let eye_glyph = single[1].0;
    tank.spawn_fish(FishSpecies::Merluza, "Janus".to_string(), &mut rng);
    tank.apply_named_mutation("Janus", "backwardstelophase");
    let segs = tank.fish[1].static_left_segments();
    let tail_w = tank.fish[1]
        .mutant
        .as_ref()
        .unwrap()
        .tail_variant
        .display_width();
    assert!(
        !segs.iter().any(|&(c, _)| c == eye_glyph),
        "a plain backwardstelophase fish has no head eyes"
    );
    let head: Vec<char> = segs.iter().take(tail_w).map(|&(c, _)| c).collect();
    let rear: Vec<char> = segs.iter().rev().take(tail_w).map(|&(c, _)| c).collect();
    let rear: Vec<char> = rear.into_iter().rev().collect();
    let rear_mirrored: Vec<char> = rear.iter().rev().map(|&c| mirror_char(c)).collect();
    assert_eq!(
        head, rear_mirrored,
        "the two tails are mirror images of each other, not identical copies"
    );
}

#[test]
fn telophase_and_backwardstelophase_interconvert() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Janus".to_string(), &mut rng);
    assert!(tank.apply_named_mutation("Janus", "telophase"));
    assert!(!tank.fish[0].mutant.as_ref().unwrap().backwards);
    assert!(
        !tank.apply_named_mutation("Janus", "telophase"),
        "re-telophasing a forward double is a no-op"
    );
    assert!(
        tank.apply_named_mutation("Janus", "backwardstelophase"),
        "a forward double can flip to backward"
    );
    {
        let m = tank.fish[0].mutant.as_ref().unwrap();
        assert!(m.is_double && m.backwards);
    }
    assert!(
        !tank.apply_named_mutation("Janus", "backwardstelophase"),
        "re-backwardstelophasing a backward double is a no-op"
    );
    assert!(
        tank.apply_named_mutation("Janus", "telophase"),
        "a backward double can flip back to forward"
    );
    let m = tank.fish[0].mutant.as_ref().unwrap();
    assert!(m.is_double && !m.backwards);
}

#[test]
fn anchoveta_backwardstelophase_holds_width_invariant() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Anchoveta, "Ana".to_string(), &mut rng);
    assert!(tank.apply_named_mutation("Ana", "backwardstelophase"));
    let fish = &tank.fish[0];
    let m = fish.mutant.as_ref().unwrap();
    assert!(
        m.is_double && m.backwards,
        "fixed multichar fish go backward too"
    );
    let segs = fish.static_left_segments();
    assert_eq!(
        segment_display_width(&segs),
        fish.display_width,
        "a backward fixed fish renders exactly its display_width"
    );
}

#[test]
fn jellyfish_backwardstelophase_is_still_two_cells() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Jellyfish, "Jelly".to_string(), &mut rng);
    assert!(tank.apply_named_mutation("Jelly", "backwardstelophase"));
    let fish = &tank.fish[0];
    let m = fish.mutant.as_ref().unwrap();
    assert!(m.is_double && m.backwards);
    assert_eq!(
        fish.static_left_segments().len(),
        2,
        "a doubled jelly is two cells whether forward or backward"
    );
}

#[test]
fn cytokinesis_splits_a_backward_fixed_fish() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Anchoveta, "Ana".to_string(), &mut rng);
    tank.apply_named_mutation("Ana", "backwardstelophase");
    assert!(tank.apply_named_mutation("Ana", "cytokinesis"));
    assert_eq!(tank.fish.len(), 2);
    for f in &tank.fish {
        let m = f.mutant.as_ref().unwrap();
        assert!(
            !m.is_double && !m.backwards,
            "split clears double + backward"
        );
    }
}

#[test]
fn cytokinesis_splits_a_backward_telophase_into_two_singles() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Merluza, "Janus".to_string(), &mut rng);
    tank.apply_named_mutation("Janus", "backwardstelophase");
    assert!(tank.apply_named_mutation("Janus", "cytokinesis"));
    assert_eq!(
        tank.fish.len(),
        2,
        "cytokinesis splits the backward double in two"
    );
    for f in &tank.fish {
        let m = f.mutant.as_ref().unwrap();
        assert!(!m.is_double, "each half is a single fish");
        assert!(!m.backwards, "splitting clears the backward flag");
    }
}

#[test]
fn worm_backwardstelophase_holds_width_and_hides_heads() {
    let mut tank = make_tank();
    let mut rng = rng();
    let worm = Fish::new_unfish(UnfishKind::Worm, "Squirm".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(worm, "Squirm".to_string(), &mut rng);
    assert!(tank.apply_named_mutation("Squirm", "backwardstelophase"));
    let fish = &tank.fish[0];
    let us = fish.unfish_state.as_ref().unwrap();
    assert!(us.worm_is_double && us.worm_backwards);
    let segs = fish.static_left_segments();
    assert_eq!(
        segment_display_width(&segs),
        fish.display_width,
        "a backward worm renders exactly its display_width"
    );
    assert!(
        !segs.iter().any(|&(c, _)| c == '('),
        "a backward worm shows comma tails, not (0) heads"
    );
}

#[test]
fn worm_telophase_and_backwardstelophase_interconvert() {
    let mut tank = make_tank();
    let mut rng = rng();
    let worm = Fish::new_unfish(UnfishKind::Worm, "Squirm".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(worm, "Squirm".to_string(), &mut rng);
    assert!(tank.apply_named_mutation("Squirm", "telophase"));
    assert!(!tank.fish[0].unfish_state.as_ref().unwrap().worm_backwards);
    assert!(tank.apply_named_mutation("Squirm", "backwardstelophase"));
    assert!(tank.fish[0].unfish_state.as_ref().unwrap().worm_backwards);
    assert!(
        !tank.apply_named_mutation("Squirm", "backwardstelophase"),
        "re-backwardstelophasing a backward worm is a no-op"
    );
    assert!(tank.apply_named_mutation("Squirm", "telophase"));
    assert!(!tank.fish[0].unfish_state.as_ref().unwrap().worm_backwards);
}

#[test]
fn cytokinesis_splits_a_backward_worm() {
    let mut tank = make_tank();
    let mut rng = rng();
    let worm = Fish::new_unfish(UnfishKind::Worm, "Squirm".to_string(), 20.0, 10.0, &mut rng);
    tank.place_fish(worm, "Squirm".to_string(), &mut rng);
    tank.apply_named_mutation("Squirm", "backwardstelophase");
    assert!(tank.apply_named_mutation("Squirm", "cytokinesis"));
    assert_eq!(tank.fish.len(), 2);
    for f in &tank.fish {
        let us = f.unfish_state.as_ref().unwrap();
        assert!(!us.worm_is_double && !us.worm_backwards);
    }
}

#[test]
fn cow_backwardstelophase_routes_and_doubles_milk() {
    let mut tank = make_tank();
    let mut rng = rng();
    let name = tank.spawn_cow(CowVariant::Brown, &mut rng);
    assert!(tank.apply_named_mutation(&name, "backwardstelophase"));
    let cow = &tank.cows[0];
    assert!(cow.mutant.is_double && cow.mutant.backwards);
    assert_eq!(
        cow.milk_yield(),
        2,
        "a backwardstelophase cow yields double milk"
    );
    assert!(
        tank.apply_named_mutation(&name, "telophase"),
        "a backward cow can flip forward"
    );
    assert!(!tank.cows[0].mutant.backwards);
    assert_eq!(
        tank.cows[0].milk_yield(),
        2,
        "a forward telophase cow also yields per-component milk (C4)"
    );
}

#[test]
fn telophase_seeds_two_same_species_components() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Cashfish, "Midas".to_string(), &mut rng);
    tank.apply_named_mutation("Midas", "telophase");
    let fish = &tank.fish[0];
    assert_eq!(fish.fused_components().len(), 2);
    assert_eq!(
        fish.ability_stacks(FishSpecies::Cashfish),
        2,
        "a plain telophase stacks its own species' passive twice"
    );
}

#[test]
fn single_fish_has_one_ability_stack() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Cashfish, "Midas".to_string(), &mut rng);
    let fish = &tank.fish[0];
    assert!(fish.fused_components().is_empty());
    assert_eq!(fish.ability_stacks(FishSpecies::Cashfish), 1);
}

#[test]
fn endocytosis_collapses_to_single_but_keeps_stacks() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Cashfish, "Midas".to_string(), &mut rng);
    tank.apply_named_mutation("Midas", "telophase");
    assert!(tank.fish[0].mutant.as_ref().unwrap().is_double);
    assert!(tank.apply_named_mutation("Midas", "endocytosis"));
    let fish = &tank.fish[0];
    assert!(
        !fish.mutant.as_ref().unwrap().is_double,
        "endocytosis collapses the double back to one body"
    );
    assert_eq!(tank.fish.len(), 1, "no second body spawns");
    assert_eq!(
        fish.ability_stacks(FishSpecies::Cashfish),
        2,
        "the absorbed half's ability stays stacked"
    );
}

#[test]
fn endocytosis_unavailable_on_single_entity() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Cashfish, "Midas".to_string(), &mut rng);
    assert!(
        !tank.apply_named_mutation("Midas", "endocytosis"),
        "a single entity cannot endocytose"
    );
}

#[test]
fn endocytosis_keeps_cow_milk_stacked() {
    let mut tank = make_tank();
    let mut rng = rng();
    let name = tank.spawn_cow(CowVariant::Brown, &mut rng);
    tank.apply_named_mutation(&name, "telophase");
    tank.apply_named_mutation(&name, "endocytosis");
    let cow = &tank.cows[0];
    assert!(!cow.mutant.is_double);
    assert_eq!(
        cow.milk_yield(),
        2,
        "an endocytosed cow keeps its doubled milk"
    );
}

#[test]
fn telophase_mutantfish_doubles_auto_mutate_stacks() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Mutantfish, "Goo".to_string(), &mut rng);
    assert_eq!(tank.fish[0].auto_mutate_stacks(), 1);
    tank.apply_named_mutation("Goo", "telophase");
    assert_eq!(
        tank.fish[0].auto_mutate_stacks(),
        2,
        "a telophase mutantfish auto-mutates at double frequency"
    );
}

#[test]
fn telophase_candyfish_doubles_touch_stacks() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Candyfish, "Sweet".to_string(), &mut rng);
    tank.apply_named_mutation("Sweet", "telophase");
    assert_eq!(tank.fish[0].ability_stacks(FishSpecies::Candyfish), 2);
}

#[test]
fn cytokinesis_clears_fused_components() {
    let mut tank = make_tank();
    let mut rng = rng();
    tank.spawn_fish(FishSpecies::Cashfish, "Midas".to_string(), &mut rng);
    tank.apply_named_mutation("Midas", "telophase");
    tank.apply_named_mutation("Midas", "cytokinesis");
    assert_eq!(tank.fish.len(), 2);
    for f in &tank.fish {
        assert!(
            f.fused_components().is_empty(),
            "a split-off half is a fresh single with no stacked abilities"
        );
        assert_eq!(f.ability_components().len(), 1, "each half is one entity");
    }
}
