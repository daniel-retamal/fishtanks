use fishtank::app::App;
use fishtank::economy::Rarity;
use fishtank::fishes::fish::Fish;
use fishtank::fishes::species::{FishSpecies, Habitat};
use fishtank::loot::LootPool;
use fishtank::tank::TankKind;
use rand::SeedableRng;
use rand::rngs::SmallRng;

const SEED: u64 = 23;
const DUE: f32 = 0.0;
const TANK_W: u16 = 120;
const TANK_H: u16 = 30;
const ABOVE: f32 = 4.0;
const BELOW: f32 = 20.0;
const LOOK_UP: char = '°';
const LOOK_AHEAD: char = 'o';

fn fish(species: FishSpecies, name: &str) -> Fish {
    Fish::new(
        species,
        name.to_string(),
        0.0,
        0.0,
        &mut SmallRng::seed_from_u64(SEED),
    )
}

fn place(app: &mut App, fish: Fish) {
    let (x, y) = (fish.position.x, fish.position.y);
    let name = fish.name.clone();
    app.tanks[0].place_fish(fish, name.clone(), &mut SmallRng::seed_from_u64(SEED));
    let placed = app.tanks[0]
        .fish
        .iter_mut()
        .find(|f| f.name == name)
        .expect("placed");
    placed.position.x = x;
    placed.position.y = y;
}

#[test]
fn six_fish_live_only_in_the_deep() {
    let deep = FishSpecies::of_the_deep();
    assert_eq!(deep.len(), 6);
    for species in deep {
        let config = species.config();
        assert_eq!(config.habitat, Habitat::Deep, "{}", config.name);
        assert!(!config.buyable, "{} is never sold", config.name);
        assert!(
            !config.habitat.is_fished(),
            "{} is never caught",
            config.name
        );
        assert!(!FishSpecies::all_wild().contains(&species));
        assert_eq!(FishSpecies::parse(config.name), Some(species));
    }
    let legendary: Vec<&str> = FishSpecies::of_the_deep()
        .into_iter()
        .filter(|s| s.config().rarity == Rarity::Legendary)
        .map(|s| s.config().name)
        .collect();
    assert_eq!(legendary, ["Quimera", "Celacanto"]);
}

#[test]
fn a_bell_jar_or_the_fishing_hole_can_hold_a_deep_rare_but_never_a_deep_legendary() {
    let jar = LootPool::default_pool()
        .with_native(TankKind::CoralReef)
        .with_deep()
        .species();
    let hole = LootPool::of_the_hole(TankKind::CoralReef).species();
    for species in FishSpecies::of_the_deep() {
        let rare = species.config().rarity != Rarity::Legendary;
        assert_eq!(jar.contains(&species), rare, "{species:?}");
        assert_eq!(hole.contains(&species), rare, "{species:?}");
    }
    assert!(
        hole.iter().all(|s| s.config().habitat == Habitat::Deep),
        "the hole is Deep catches only"
    );
}

#[test]
fn a_celacanto_brings_back_a_kind_you_no_longer_have() {
    let mut app = App::new();
    let living = app.tanks[0].fish[0].species;
    app.graveyard.push(fish(living, "Gemelo"));
    let mut celacanto = fish(FishSpecies::Celacanto, "Lázaro");
    celacanto.habits.lazarus_clock = Some(DUE);
    place(&mut app, celacanto);
    app.tick();
    let back = |app: &App, name: &str| {
        app.tanks
            .iter()
            .flat_map(|tank| tank.fish.iter())
            .any(|f| f.name == name)
    };
    assert!(!back(&app, "Gemelo"), "its kind still lives");
    app.graveyard.push(fish(FishSpecies::Sawfish, "Dentón"));
    if let Some(celacanto) = app.tanks[0].fish.iter_mut().find(|f| f.name == "Lázaro") {
        celacanto.habits.lazarus_clock = Some(DUE);
    }
    app.tick();
    assert!(
        back(&app, "Dentón"),
        "no Sawfish lived, so the Sawfish comes back"
    );
    assert!(!app.graveyard.iter().any(|f| f.name == "Dentón"));
}

fn barril_eyes(app: &App) -> Vec<char> {
    let barril = app.tanks[0]
        .fish
        .iter()
        .find(|f| f.species == FishSpecies::Barril)
        .expect("a Barril");
    let sprite = barril.line_sprite();
    sprite.rows[sprite.body_row]
        .iter()
        .map(|cell| cell.0)
        .filter(|&glyph| glyph == LOOK_UP || glyph == LOOK_AHEAD)
        .collect()
}

#[test]
fn a_barril_looks_up_at_whatever_swims_above_it() {
    let mut app = App::new();
    app.tanks[0].fish.clear();
    app.tanks[0].resize(TANK_W, TANK_H, &[]);
    let mut barril = fish(FishSpecies::Barril, "Domo");
    barril.position.x = 50.0;
    barril.position.y = BELOW;
    barril.frozen = true;
    place(&mut app, barril);
    app.tick();
    assert_eq!(barril_eyes(&app), [LOOK_AHEAD, LOOK_AHEAD]);
    let mut above = fish(FishSpecies::Merluza, "Sombra");
    above.position.x = 50.0;
    above.position.y = ABOVE;
    above.frozen = true;
    place(&mut app, above);
    app.tick();
    assert_eq!(barril_eyes(&app), [LOOK_UP, LOOK_UP]);
}

#[test]
fn a_quimera_wears_the_colours_of_the_fish_around_it() {
    let mut app = App::new();
    app.tanks[0].fish.clear();
    app.tanks[0].resize(TANK_W, TANK_H, &[]);
    let quimera = fish(FishSpecies::Quimera, "Retazos");
    let alone: Vec<_> = quimera.line_sprite().rows.concat();
    place(&mut app, quimera);
    let mut neighbour = fish(FishSpecies::Betta, "Vecina");
    neighbour.color = fishtank::colors::ORANGE;
    place(&mut app, neighbour);
    app.tick();
    let quimera = app.tanks[0]
        .fish
        .iter()
        .find(|f| f.species == FishSpecies::Quimera)
        .expect("Quimera");
    let stitched: Vec<_> = quimera.line_sprite().rows.concat();
    assert_ne!(alone, stitched);
    assert!(
        stitched
            .iter()
            .any(|cell| cell.1 == fishtank::colors::ORANGE),
        "a patch of its neighbour's orange"
    );
}
