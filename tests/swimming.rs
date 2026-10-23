use fishtank::entities::food::Food;
use fishtank::fishes::fish::{Direction, Fish, FishState};
use fishtank::fishes::species::FishSpecies;
use fishtank::settings::Settings;
use fishtank::tank::{Sky, Tank, TankBackground, TankKind};

const W: u16 = 60;
const H: u16 = 14;
const FAR: f32 = 1000.0;
const FRAMES: usize = 90;
const MEAL_PATIENCE: usize = 30 * 60;

fn lab(width: u16, height: u16) -> Tank {
    let mut tank = Tank::new("Lab".into(), TankKind::Base, &[]);
    tank.resize(width, height, &[]);
    if let TankBackground::Plain { plants } = &mut tank.background {
        plants.clear();
    }
    tank
}

fn salmon(name: &str) -> Fish {
    Fish::new(
        FishSpecies::Salmon,
        name.into(),
        20.0,
        5.0,
        &mut rand::rng(),
    )
}

fn frames_in_the_row_it_bounced_off(start_y: f32, dy: f32) -> usize {
    let settings = Settings::default();
    let mut fish = salmon("Bouncer");
    fish.position.y = start_y;
    fish.velocity.dx = 0.0;
    fish.velocity.dy = dy;
    fish.tick(&settings, W, H, 0);
    let row = fish.position.y as i32;
    let mut frames = 1;
    for _ in 0..FRAMES {
        fish.tick(&settings, W, H, 0);
        if fish.position.y as i32 != row {
            break;
        }
        frames += 1;
    }
    frames
}

#[test]
fn a_swimmer_stays_in_the_last_row_as_long_as_in_the_first() {
    let bottom = frames_in_the_row_it_bounced_off(FAR, 1.0);
    let top = frames_in_the_row_it_bounced_off(-FAR, -1.0);
    assert!(bottom > 1, "it left the last row after {bottom} frame");
    assert!(
        bottom.abs_diff(top) <= 1,
        "the last row held it {bottom} frames, the first {top}"
    );
}

#[test]
fn a_fish_wider_than_its_tank_never_flickers() {
    let settings = Settings::default();
    let narrow = 12;
    let mut fish = salmon("Whale");
    fish.display_width = narrow as usize + 8;
    fish.velocity.dx = fish.speed;
    fish.facing = Direction::Right;
    let mut turns = 0;
    let mut facing = fish.facing;
    for _ in 0..FRAMES {
        fish.tick(&settings, narrow, H, 0);
        if fish.facing != facing {
            turns += 1;
            facing = fish.facing;
        }
    }
    assert_eq!(turns, 0, "it turned {turns} times in {FRAMES} frames");
    let head = fish.head_x();
    assert!(
        (0..narrow as i32).contains(&head),
        "its head {head} is on screen"
    );
}

fn eats_within_patience(tank: &mut Tank, name: &str) -> bool {
    let mut food = 1;
    tank.feed(1, &mut food);
    let index = tank
        .fish
        .iter()
        .position(|f| f.name == name)
        .expect("the fish");
    let before = tank.fish[index].weight_g;
    for _ in 0..MEAL_PATIENCE {
        tank.tick(&Settings::default(), 0, Sky::default());
        let index = tank
            .fish
            .iter()
            .position(|f| f.name == name)
            .expect("the fish");
        if tank.fish[index].weight_g > before {
            return true;
        }
    }
    false
}

#[test]
fn a_fish_with_a_lure_reaches_its_food() {
    let mut tank = lab(W, H);
    tank.place_fish(salmon("Angler"), "Angler".into(), &mut rand::rng());
    assert!(tank.apply_named_mutation("Angler", "lure"));
    let lead = tank.fish[0].lead();
    assert!(lead > 1, "lead {lead}");
    let row = (H - 1) as f32;
    let food_x = 40.0;
    let mut pellet = Food::new(food_x);
    pellet.position.y = row;
    pellet.settled = true;
    tank.food.push(pellet);
    let angler = &mut tank.fish[0];
    angler.speed = 0.0;
    angler.facing = Direction::Right;
    angler.position.x = food_x - (angler.display_width as f32 - 1.0);
    angler.position.y = row;
    let before = angler.weight_g;
    for _ in 0..MEAL_PATIENCE {
        tank.tick(&Settings::default(), 0, Sky::default());
        if tank.fish[0].weight_g > before {
            return;
        }
    }
    panic!("the angler never reached the pellet with its mouth");
}

#[test]
fn a_fish_too_long_for_a_small_tank_still_eats() {
    let narrow = 16;
    let mut tank = lab(narrow, 8);
    tank.place_fish(salmon("Long"), "Long".into(), &mut rand::rng());
    tank.fish[0].display_width = narrow as usize + 10;
    assert!(eats_within_patience(&mut tank, "Long"));
    assert!(!matches!(tank.fish[0].state, FishState::SeekingFood { .. }));
}

#[test]
fn big_mutants_in_a_small_tank_never_get_stuck_at_dinner() {
    let mut tank = lab(30, 8);
    for i in 0..6 {
        let name = format!("Big {i}");
        tank.place_fish(salmon(&name), name.clone(), &mut rand::rng());
        for _ in 0..(3 + i) {
            tank.apply_named_mutation(&name, "sizeincrease");
        }
        if i % 2 == 0 {
            tank.apply_named_mutation(&name, "lure");
        }
    }
    let mut food = 30;
    tank.feed(30, &mut food);
    for _ in 0..30 * 180 {
        tank.tick(&Settings::default(), 0, Sky::default());
    }
    let hungry = tank.fish.iter().filter(|fish| fish.seeks_food()).count();
    assert!(
        tank.food.is_empty() || hungry == 0,
        "{} pellets left for {hungry} hungry fish",
        tank.food.len()
    );
}

#[test]
fn a_pellet_on_the_floor_follows_the_glass_when_the_window_shrinks() {
    let mut tank = lab(W, H);
    tank.place_fish(salmon("Diner"), "Diner".into(), &mut rand::rng());
    let mut pellet = Food::new(W as f32 - 2.0);
    pellet.position.y = (H - 1) as f32;
    pellet.settled = true;
    tank.food.push(pellet);
    let narrow = W / 2;
    tank.resize(narrow, H, &[]);
    assert!(tank.food[0].position.x < narrow as f32);
    let before = tank.fish[0].weight_g;
    for _ in 0..MEAL_PATIENCE {
        tank.tick(&Settings::default(), 0, Sky::default());
        if tank.fish[0].weight_g > before {
            return;
        }
    }
    panic!("the diner never reached the pellet");
}
