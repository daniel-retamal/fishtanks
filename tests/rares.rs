use std::collections::HashSet;

use fishtank::{
    colors::{AMBER_DARK, GOLD, GOLD_BRIGHT},
    economy::Rarity,
    entities::{
        bubble::{Bubble, BubblePhase},
        cow::{Cow, CowVariant, cow_sprite},
        food::Food,
    },
    fishes::{
        fish::{Direction, Fish, FishState, MOON_DARK, MOON_LIT},
        habits::{Chase, Side},
        mutant::Circadian,
        mutations::{Mutatable, Mutation, apply_mutation, apply_mutation_to_fish},
        species::{ALL_SPECIES, BodyTemplate, FishSpecies, Habitat, Locomotion},
        unfish::UnfishKind,
    },
    loot::{LootPool, StockItem, school_weight},
    restore::Restorable,
    settings::Settings,
    sprite::TRANSPARENT,
    tank::{FULL_MOON, LUNAR_MONTH_DAYS, Sky, Tank, TankEvent, TankKind},
    testing::Tui,
    ui::{
        fields::{FieldKind, field_value},
        tank_view::TankView,
    },
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier},
    widgets::Widget,
};
use unicode_width::UnicodeWidthChar;

const NEW_RARES: [FishSpecies; 27] = [
    FishSpecies::Caracol,
    FishSpecies::Babosa,
    FishSpecies::Estrella,
    FishSpecies::Ermitano,
    FishSpecies::Lenguado,
    FishSpecies::Piedra,
    FishSpecies::Pejesapo,
    FishSpecies::Cofre,
    FishSpecies::Caballito,
    FishSpecies::Morena,
    FishSpecies::Volador,
    FishSpecies::Espada,
    FishSpecies::Tollo,
    FishSpecies::Mariposa,
    FishSpecies::Linterna,
    FishSpecies::Luciernaga,
    FishSpecies::Bagre,
    FishSpecies::Luna,
    FishSpecies::Loro,
    FishSpecies::Mimo,
    FishSpecies::Timido,
    FishSpecies::Ciego,
    FishSpecies::Globo,
    FishSpecies::Pulpo,
    FishSpecies::Draco,
    FishSpecies::Neon,
    FishSpecies::Martillo,
];

const BUBBLE_ROW_SLACK: f32 = 0.9;
const TANK_W: u16 = 60;
const TANK_H: u16 = 16;
const FPS: f32 = 10.0;
const DAY: Sky = Sky {
    daylight: true,
    moon: FULL_MOON,
    calm: false,
};
const NIGHT: Sky = Sky {
    daylight: false,
    moon: FULL_MOON,
    calm: false,
};

fn settings() -> Settings {
    Settings {
        fps: FPS,
        ..Settings::default()
    }
}

fn tank_of(kind: TankKind) -> Tank {
    let mut tank = Tank::new("T".to_string(), kind, &[]);
    tank.resize(TANK_W, TANK_H, &[]);
    tank
}

fn add(tank: &mut Tank, species: FishSpecies, name: &str) -> usize {
    let mut rng = rand::rng();
    assert!(tank.spawn_fish(species, name.to_string(), &mut rng));
    tank.fish
        .iter()
        .position(|f| f.name == name)
        .expect("the fish is in the tank")
}

fn tick(tank: &mut Tank, sky: Sky, ticks: usize) -> Vec<TankEvent> {
    let settings = settings();
    let mut events = Vec::new();
    for _ in 0..ticks {
        events.extend(tank.tick(&settings, 0, sky));
    }
    events
}

fn rows_of(fish: &Fish) -> Vec<String> {
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

fn render(tank: &Tank) -> Buffer {
    let area = Rect::new(0, 0, tank.width, tank.height);
    let mut buf = Buffer::empty(area);
    TankView::new(tank).render(area, &mut buf);
    buf
}

fn footprint(fish: &Fish) -> (i32, i32, i32, i32) {
    let sprite = fish.line_sprite();
    let left = fish.position.x as i32;
    let top = fish.position.y as i32 - sprite.body_row as i32;
    let width = sprite.rows.iter().map(Vec::len).max().unwrap_or(0) as i32;
    (
        left,
        top,
        left + width - 1,
        top + sprite.rows.len() as i32 - 1,
    )
}

#[test]
fn a_rare_bites_an_eighth_of_the_time_however_many_rares_there_are() {
    let common = school_weight(Rarity::Common);
    let rare = school_weight(Rarity::Rare);
    assert_eq!(
        rare * 8,
        common + rare,
        "a rare is an eighth of the wild fish"
    );
    let pool = LootPool::default_pool();
    let mut rng = rand::rng();
    let draws = 40_000;
    let rares = (0..draws)
        .filter_map(|_| pool.roll_species(&mut rng))
        .filter(|species| species.config().rarity == Rarity::Rare)
        .count();
    let share = rares as f64 / draws as f64;
    assert!(
        (share - 0.125).abs() < 0.01,
        "rares are {share:.3} of wild fish"
    );
}

#[test]
fn every_new_rare_is_sold_in_the_shop_and_caught_everywhere() {
    for species in NEW_RARES {
        let config = species.config();
        assert_eq!(config.rarity, Rarity::Rare, "{}", config.name);
        assert!(config.buyable, "{} is sold in the shop", config.name);
        assert_eq!(config.habitat, Habitat::Everywhere, "{}", config.name);
        assert!(
            FishSpecies::parse(config.name) == Some(species),
            "{} is one word the commands can parse",
            config.name
        );
    }
    let wild: HashSet<FishSpecies> = FishSpecies::all_wild().iter().copied().collect();
    assert!(NEW_RARES.iter().all(|species| wild.contains(species)));
}

#[test]
fn every_species_fills_its_display_width_both_ways() {
    let mut rng = rand::rng();
    for &species in ALL_SPECIES {
        let mut fish = Fish::new(species, "Probe".to_string(), 10.0, 10.0, &mut rng);
        for facing in [Direction::Left, Direction::Right] {
            for phase in [0.0, std::f32::consts::FRAC_PI_2] {
                fish.facing = facing;
                fish.sway.phase = phase;
                let widest = fish
                    .line_sprite()
                    .rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|&(c, _)| UnicodeWidthChar::width(c).unwrap_or(1))
                            .sum::<usize>()
                    })
                    .max()
                    .unwrap_or(0);
                let figure = matches!(species.config().body, BodyTemplate::Figure(_));
                assert!(
                    figure || widest == fish.display_width,
                    "{} {facing:?} draws {widest} cells in a {}-cell slot",
                    species.display_name(),
                    fish.display_width
                );
                assert!(
                    widest <= fish.display_width,
                    "{} {facing:?} draws {widest} cells in a {}-cell slot",
                    species.display_name(),
                    fish.display_width
                );
            }
        }
    }
}

#[test]
fn a_species_born_with_a_mutation_keeps_it_through_every_door() {
    let mut rng = rand::rng();
    for &species in ALL_SPECIES {
        let born = species.config().born_with;
        if born.is_empty() {
            continue;
        }
        let spawned = Fish::new(species, "Probe".to_string(), 0.0, 0.0, &mut rng);
        let bought = Fish::new_for_display(species, &mut rng);
        let mut restored = spawned.clone();
        restored.restore();
        for fish in [&spawned, &bought, &restored] {
            for &mutation in born {
                let has = fish.adornments().has(mutation)
                    || (mutation == Mutation::NightOwl
                        && fish.circadian_state() == Circadian::NightOwl)
                    || (mutation == Mutation::Feet && fish.feet().is_some());
                assert!(
                    has,
                    "{} lost its {}",
                    species.display_name(),
                    mutation.token()
                );
            }
            assert_eq!(fish.mutation_count(), 0, "a birthmark is not a mutation");
        }
    }
}

#[test]
fn the_lure_grows_on_every_being_with_a_head_at_the_end_of_its_line() {
    let mut rng = rand::rng();
    let heads = [
        Fish::new(FishSpecies::Salmon, "A".into(), 0.0, 0.0, &mut rng),
        Fish::new(FishSpecies::Anchoveta, "B".into(), 0.0, 0.0, &mut rng),
        Fish::new(FishSpecies::Jellyfish, "C".into(), 0.0, 0.0, &mut rng),
        Fish::new_unfish(UnfishKind::Phantom, "D".into(), 0.0, 0.0, &mut rng),
        Fish::new_unfish(UnfishKind::Reversed, "E".into(), 0.0, 0.0, &mut rng),
    ];
    for mut fish in heads {
        assert!(fish.supports_now(Mutation::Lure), "{}", fish.name);
        let width = fish.display_width;
        apply_mutation_to_fish(&mut fish, Mutation::Lure, &mut rng);
        assert_eq!(
            fish.display_width,
            width + 2,
            "{} makes room for its lure",
            fish.name
        );
        assert!(
            !fish.supports_now(Mutation::Lure),
            "{} grows one lure",
            fish.name
        );
        let rows = rows_of(&fish);
        let body = &rows[fish.line_sprite().body_row];
        assert!(body.contains('º'), "{}: {body}", fish.name);
        assert!(
            rows.iter().any(|row| row.contains(',')),
            "{} hangs its lure from a stalk: {rows:?}",
            fish.name
        );
    }
    for kind in [UnfishKind::Ball, UnfishKind::Skull, UnfishKind::Worm] {
        let fish = Fish::new_unfish(kind, "U".into(), 0.0, 0.0, &mut rng);
        assert!(
            !fish.supports_now(Mutation::Lure),
            "{kind:?} has no head to hang it from"
        );
    }
    let mut cow = Cow::new(
        "Vaquita".into(),
        CowVariant::random(&mut rng),
        0.0,
        0.0,
        &mut rng,
    );
    assert!(cow.supports_now(Mutation::Lure));
    let plain_rows = cow_sprite(&cow).len();
    apply_mutation(&mut cow, Mutation::Lure, &mut rng);
    let rows = cow_sprite(&cow);
    assert_eq!(rows.len(), plain_rows);
    let face = cow.sprite_top_offset() as usize + 1;
    let head_row: String = rows[face].iter().map(|&(c, _)| c).collect();
    assert!(
        head_row.starts_with('º'),
        "the cow's lure hangs before its face: {head_row}"
    );
}

#[test]
fn a_lure_glows_gold_at_night_and_dims_by_day() {
    let mut rng = rand::rng();
    let mut angler = Fish::new(FishSpecies::Linterna, "Lamp".into(), 0.0, 0.0, &mut rng);
    angler.facing = Direction::Left;
    let lure_color = |fish: &Fish| {
        let sprite = fish.line_sprite();
        sprite.rows[sprite.body_row][0]
    };
    angler.sky = DAY;
    assert_eq!(lure_color(&angler), ('º', AMBER_DARK));
    angler.sky = NIGHT;
    let (glyph, color) = lure_color(&angler);
    assert_eq!(glyph, 'º');
    assert!(color == GOLD || color == GOLD_BRIGHT, "{color:?}");
}

#[test]
fn a_night_owl_sleeps_by_day_and_wakes_at_night() {
    let mut rng = rand::rng();
    let mut cat = Fish::new(FishSpecies::Bagre, "Michi".into(), 0.0, 0.0, &mut rng);
    cat.sky = DAY;
    assert!(cat.is_asleep());
    assert!(rows_of(&cat).concat().contains('¯') || rows_of(&cat).concat().contains('-'));
    cat.sky = NIGHT;
    assert!(!cat.is_asleep());
    let mut salmon = Fish::new(FishSpecies::Salmon, "Sal".into(), 0.0, 0.0, &mut rng);
    apply_mutation_to_fish(&mut salmon, Mutation::NightOwl, &mut rng);
    salmon.sky = NIGHT;
    assert!(
        !salmon.is_asleep(),
        "the night-owl mutation is awake at night"
    );
    salmon.sky = DAY;
    assert!(salmon.is_asleep());
}

#[test]
fn a_sleeper_snores_and_leaves_the_food_to_the_fish_that_are_awake() {
    let mut tank = tank_of(TankKind::Base);
    let cat = add(&mut tank, FishSpecies::Bagre, "Michi");
    tank.food.push(Food::new(10.0));
    let mut snored = false;
    for _ in 0..400 {
        tick(&mut tank, DAY, 1);
        assert!(
            !matches!(tank.fish[cat].state, FishState::SeekingFood { .. }),
            "a sleeping catfish does not hunt"
        );
        snored |= tank
            .bubbles
            .iter()
            .any(|b| b.bubble_char == 'z' || b.bubble_char == 'Z');
    }
    assert!(snored, "a sleeper breathes z bubbles");
}

#[test]
fn a_snail_crawls_round_the_glass_and_never_lets_go() {
    let mut tank = tank_of(TankKind::Base);
    let snail = add(&mut tank, FishSpecies::Caracol, "Shelly");
    tank.fish[snail].speed = 6.0;
    let mut sides = HashSet::new();
    for _ in 0..3000 {
        tick(&mut tank, DAY, 1);
        let fish = &tank.fish[snail];
        let (left, top, right, bottom) = footprint(fish);
        let touching =
            left == 0 || top == 0 || right == TANK_W as i32 - 1 || bottom == TANK_H as i32 - 1;
        assert!(
            touching,
            "the snail let go of the glass at {left},{top}..{right},{bottom}"
        );
        if let Some(crawl) = fish.habits.crawl {
            sides.insert(format!("{:?}", crawl.side));
        }
    }
    assert_eq!(sides.len(), 4, "it went all the way round: {sides:?}");
    assert!(!tank.trails.is_empty(), "it leaves a slime trail");
}

#[test]
fn a_trail_never_covers_anything() {
    let mut tank = tank_of(TankKind::Base);
    let fish = add(&mut tank, FishSpecies::Merluza, "Mer");
    tank.fish[fish].frozen = true;
    let (x, y) = (tank.fish[fish].head_x(), tank.fish[fish].position.y as i32);
    tank.trails.push(fishtank::tank::TrailMark {
        x,
        y,
        ttl: 5.0,
        color: Color::Gray,
    });
    tank.trails.push(fishtank::tank::TrailMark {
        x: 1,
        y: 1,
        ttl: 5.0,
        color: Color::Gray,
    });
    let buf = render(&tank);
    assert_ne!(
        buf[(x as u16, y as u16)].symbol(),
        fishtank::tank::TRAIL_GLYPH.to_string(),
        "a fish draws over the trail"
    );
    let empty_cell = buf[(1, 1)].symbol().to_string();
    assert!(
        empty_cell == fishtank::tank::TRAIL_GLYPH.to_string() || empty_cell != " ",
        "a trail shows only on empty water"
    );
}

#[test]
fn walkers_keep_to_the_floor_and_the_crab_never_turns_round() {
    let mut tank = tank_of(TankKind::Base);
    let walkers: Vec<usize> = [
        FishSpecies::Lenguado,
        FishSpecies::Piedra,
        FishSpecies::Ermitano,
    ]
    .iter()
    .enumerate()
    .map(|(i, &species)| add(&mut tank, species, &format!("W{i}")))
    .collect();
    let crab = walkers[2];
    let facing = tank.fish[crab].facing;
    for _ in 0..600 {
        tick(&mut tank, DAY, 1);
        for &i in &walkers {
            let (_, _, _, bottom) = footprint(&tank.fish[i]);
            assert_eq!(
                bottom,
                TANK_H as i32 - 1,
                "{} keeps to the floor",
                tank.fish[i].name
            );
        }
        assert_eq!(
            tank.fish[crab].facing, facing,
            "a hermit crab walks sideways"
        );
    }
}

#[test]
fn a_frogfish_hops_off_the_floor_and_lands_again() {
    let mut tank = tank_of(TankKind::Base);
    let frog = add(&mut tank, FishSpecies::Pejesapo, "Sapo");
    tick(&mut tank, DAY, 1);
    let floor = tank.fish[frog].position.y;
    assert!(tank.fish[frog].hurry_zoomie());
    let mut highest = floor;
    for _ in 0..40 {
        tick(&mut tank, DAY, 1);
        highest = highest.min(tank.fish[frog].position.y);
    }
    assert!(highest < floor - 1.0, "it hopped from {floor} to {highest}");
    assert_eq!(tank.fish[frog].position.y, floor, "and landed");
}

#[test]
fn the_box_changes_colour_at_every_wall_and_celebrates_a_corner() {
    let mut tank = tank_of(TankKind::Base);
    let boxfish = add(&mut tank, FishSpecies::Cofre, "DVD");
    let hue = tank.fish[boxfish].habits.hue;
    tank.fish[boxfish].position.x = 0.2;
    tank.fish[boxfish].position.y = 0.2;
    tank.fish[boxfish].velocity.dx = -3.0;
    tank.fish[boxfish].velocity.dy = -3.0;
    let before = tank.bubbles.len();
    tick(&mut tank, DAY, 1);
    assert_ne!(
        tank.fish[boxfish].habits.hue, hue,
        "a bounce is a new colour"
    );
    assert!(
        tank.bubbles.len() >= before + 10,
        "a perfect corner throws confetti"
    );
}

#[test]
fn the_moray_slips_out_through_the_wall_and_peeks_back() {
    let mut tank = tank_of(TankKind::Base);
    let eel = add(&mut tank, FishSpecies::Morena, "Mora");
    tank.fish[eel].position.x = 1.0;
    tank.fish[eel].velocity.dx = -tank.fish[eel].speed.max(1.0) * 4.0;
    tank.fish[eel].velocity.dy = 0.0;
    tank.fish[eel].facing = Direction::Left;
    let mut out = false;
    for _ in 0..200 {
        tick(&mut tank, DAY, 1);
        out |= tank.fish[eel].is_out_of_sight(TANK_W);
        if out {
            break;
        }
    }
    assert!(out, "the moray left through the wall");
    tank.fish[eel].habits.lurk = Some(fishtank::fishes::habits::Lurk::Out(0.0));
    tick(&mut tank, DAY, 1);
    let fish = &tank.fish[eel];
    assert!(!fish.is_out_of_sight(TANK_W), "it peeks back in");
    let (left, _, right, _) = footprint(fish);
    assert!(left < 0 || right >= TANK_W as i32, "only its head shows");
}

#[test]
fn the_flying_fish_glides_up_to_the_top_with_its_wings_out() {
    let mut tank = tank_of(TankKind::Base);
    let flyer = add(&mut tank, FishSpecies::Volador, "Icaro");
    tank.fish[flyer].position.y = TANK_H as f32 - 3.0;
    assert!(tank.fish[flyer].hurry_zoomie());
    let mut spread = false;
    let mut top = f32::MAX;
    for _ in 0..12 {
        tick(&mut tank, DAY, 1);
        let fish = &tank.fish[flyer];
        spread |= fish.is_gliding() && rows_of(fish).len() == 3;
        top = top.min(fish.position.y);
    }
    assert!(spread, "its wings open while it glides");
    assert!(top <= 1.0, "it climbs to the surface, reached {top}");
}

#[test]
fn a_lunge_pops_plain_bubbles_but_never_a_cash_bubble() {
    let mut tank = tank_of(TankKind::Base);
    let sword = add(&mut tank, FishSpecies::Espada, "Zorro");
    let mut rng = rand::rng();
    tank.fish[sword].facing = Direction::Right;
    tank.fish[sword].position.x = 5.0;
    tank.fish[sword].position.y = 8.0;
    assert!(tank.fish[sword].hurry_zoomie());
    tick(&mut tank, DAY, 1);
    let y = tank.fish[sword].position.y.floor() + BUBBLE_ROW_SLACK;
    let x = tank.fish[sword].position.x + 4.0;
    let plain = Bubble::new(
        x,
        y,
        BubblePhase::rising(&mut rng),
        Color::Cyan,
        None,
        &mut rng,
    );
    let mut cash = Bubble::new(
        x + 1.0,
        y,
        BubblePhase::rising(&mut rng),
        Color::Yellow,
        Some('$'),
        &mut rng,
    );
    cash.cash_value = Some(100);
    cash.poppable = false;
    tank.bubbles = vec![plain, cash];
    tick(&mut tank, DAY, 1);
    assert!(
        tank.bubbles
            .iter()
            .all(|b| b.bubble_char == '$' || b.dead || !b.poppable),
        "the plain bubble popped"
    );
    assert!(
        tank.bubbles.iter().any(|b| b.bubble_char == '$' && !b.dead),
        "the cash bubble is not the swordfish's to pop"
    );
}

#[test]
fn a_chased_cashfish_flees_in_a_zoomie_and_pays_for_it() {
    let mut tank = tank_of(TankKind::Hell);
    let shark = add(&mut tank, FishSpecies::Tollo, "Tollo");
    let prey = add(&mut tank, FishSpecies::Cashfish, "Midas");
    tank.fish[prey].position.x = 30.0;
    tank.fish[prey].position.y = 8.0;
    tank.fish[shark].position.x = 20.0;
    tank.fish[shark].position.y = 8.0;
    tank.fish[shark].habits.chase = Some(Chase {
        target: "Midas".into(),
        secs: 10.0,
    });
    let mut fled = false;
    let mut paid = false;
    for _ in 0..80 {
        tick(&mut tank, DAY, 1);
        fled |= tank.fish[prey].is_zooming();
        paid |= tank.bubbles.iter().any(|b| b.cash_value.is_some());
    }
    assert!(fled, "the cashfish fled at zoomie speed");
    assert!(paid, "and its fleeing zoomie blew money bubbles");
}

#[test]
fn a_butterflyfish_swims_backwards_now_and_then() {
    let mut tank = tank_of(TankKind::Base);
    let fly = add(&mut tank, FishSpecies::Mariposa, "Mari");
    tank.fish[fly].position.x = 30.0;
    tank.fish[fly].velocity.dx = 2.0;
    tank.fish[fly].facing = Direction::Right;
    tank.fish[fly].habits.backwards_clock = Some(0.0);
    tick(&mut tank, DAY, 2);
    let fish = &tank.fish[fly];
    assert!(fish.habits.backwards > 0.0);
    assert!(
        (fish.velocity.dx < 0.0) != fish.facing_left(),
        "it moves against the way it faces"
    );
}

#[test]
fn fireflies_fall_into_step() {
    let mut tank = tank_of(TankKind::Base);
    let flies: Vec<usize> = (0..3)
        .map(|i| add(&mut tank, FishSpecies::Luciernaga, &format!("Luz{i}")))
        .collect();
    for (k, &i) in flies.iter().enumerate() {
        tank.fish[i].habits.flash = Some(k as f32 * 0.3);
    }
    tick(&mut tank, NIGHT, 3000);
    let mut together = 0;
    for _ in 0..200 {
        tick(&mut tank, NIGHT, 1);
        let lit = flies
            .iter()
            .filter(|&&i| tank.fish[i].habits.lit > 0.0)
            .count();
        if lit == flies.len() {
            together += 1;
        }
    }
    assert!(together > 0, "after a while every firefly flashes at once");
}

#[test]
fn a_parrot_echoes_and_parrots_echo_each_other_until_they_stop() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/spawn loro \"Polly\"").run("/spawn loro \"Pepe\"");
    tui.type_text("hello").key(crossterm::event::KeyCode::Enter);
    let mut echoes = 0;
    let mut quiet_ticks = 0;
    for _ in 0..6000 {
        tui.tick_n(1);
        let tank = &tui.app.tanks[tui.app.current_tank];
        let speaking = tank
            .fish
            .iter()
            .filter(|f| f.speech.as_ref().is_some_and(|s| s.text == "hello"))
            .count();
        let pending = tank.fish.iter().any(|f| f.habits.echo.is_some());
        if speaking > 0 {
            echoes += 1;
        }
        if !pending {
            quiet_ticks += 1;
        } else {
            quiet_ticks = 0;
        }
    }
    assert!(echoes > 0, "a parrot repeats what it heard");
    assert!(quiet_ticks > 0, "and the parrots fall quiet in the end");
}

#[test]
fn a_mime_shadows_its_muse_and_mimes_its_speech() {
    let mut tank = tank_of(TankKind::Base);
    let mime = add(&mut tank, FishSpecies::Mimo, "Marcel");
    let muse = add(&mut tank, FishSpecies::Salmon, "Sal");
    let mut trail = Vec::new();
    for _ in 0..60 {
        tick(&mut tank, DAY, 1);
        trail.push((tank.fish[muse].position.x, tank.fish[muse].position.y));
    }
    let delay = (2.0 * FPS) as usize;
    let then = trail[trail.len() - 1 - delay];
    let now = (tank.fish[mime].position.x, tank.fish[mime].position.y);
    assert!(
        (now.0 - then.0).abs() < 0.5 && (now.1 - then.1).abs() < 0.5,
        "the mime is where its muse was two seconds ago: {now:?} vs {then:?}"
    );
    tank.fish[muse].say("hi".to_string());
    tick(&mut tank, DAY, 1);
    let bubble = tank.fish[mime]
        .speech
        .as_ref()
        .expect("the mime opens a bubble");
    assert_eq!(bubble.text.trim(), "", "and says nothing in it");
}

#[test]
fn a_shy_fish_hides_when_a_key_is_pressed_and_comes_out_in_zen() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/spawn timido \"Tim\"");
    tui.tick_n(2);
    tui.key(crossterm::event::KeyCode::Char('a'));
    let tank = &tui.app.tanks[tui.app.current_tank];
    let tim = tank.fish.iter().find(|f| f.name == "Tim").unwrap();
    assert!(tim.habits.hiding > 0.0, "a key press sends it to the wall");
    assert!(rows_of(tim).concat().contains('¯'), "with its eyes shut");
    tui.key(crossterm::event::KeyCode::Backspace);
    tui.run("/zen");
    tui.tick_n(2);
    let tank = &tui.app.tanks[tui.app.current_tank];
    let tim = tank.fish.iter().find(|f| f.name == "Tim").unwrap();
    assert!(tim.sky.calm, "zen is calm water");
    assert!(
        !rows_of(tim).concat().contains('¯'),
        "it opens its eyes in zen"
    );
}

#[test]
fn a_blind_fish_has_no_eyes_and_turns_when_it_bumps_into_a_fish() {
    let mut rng = rand::rng();
    let blind = Fish::new(FishSpecies::Ciego, "Ray".into(), 0.0, 0.0, &mut rng);
    let drawn = rows_of(&blind).concat();
    assert!(!drawn.contains('º') && !drawn.contains('ʘ'), "{drawn}");
    let mut tank = tank_of(TankKind::Base);
    let ray = add(&mut tank, FishSpecies::Ciego, "Ray");
    let wall = add(&mut tank, FishSpecies::Merluza, "Wall");
    tank.fish[wall].frozen = true;
    tank.fish[wall].position.x = 20.0;
    tank.fish[wall].position.y = 8.0;
    tank.fish[ray].position.x = 12.0;
    tank.fish[ray].position.y = 8.0;
    tank.fish[ray].facing = Direction::Right;
    tank.fish[ray].velocity.dx = 3.0;
    tank.fish[ray].velocity.dy = 0.0;
    let mut turned = false;
    for _ in 0..40 {
        tick(&mut tank, DAY, 1);
        turned |= tank.fish[ray].velocity.dx < 0.0;
    }
    assert!(turned, "it turns only once it touches the other fish");
}

#[test]
fn a_pufferfish_puffs_instead_of_zooming_and_when_a_beam_takes_it() {
    let mut tank = tank_of(TankKind::Base);
    let puffer = add(&mut tank, FishSpecies::Globo, "Globo");
    assert!(tank.fish[puffer].hurry_zoomie());
    tick(&mut tank, DAY, 2);
    assert!(tank.fish[puffer].is_puffed());
    assert!(rows_of(&tank.fish[puffer]).concat().contains(':'));
    assert!(!tank.fish[puffer].is_zooming(), "a puff is not a zoomie");
    tank.fish[puffer].habits.puffed = 0.0;
    tank.fish[puffer].abduction_lock = true;
    tick(&mut tank, DAY, 1);
    assert!(tank.fish[puffer].is_puffed(), "a beam puffs it up");
}

#[test]
fn an_octopus_changes_colour_inks_and_wanders_off() {
    let mut tank = tank_of(TankKind::Base);
    let octo = add(&mut tank, FishSpecies::Pulpo, "Inky");
    let hue = tank.fish[octo].habits.hue;
    tick(&mut tank, DAY, (4.0 * FPS) as usize);
    assert_ne!(
        tank.fish[octo].habits.hue, hue,
        "its colour shifts with time"
    );
    assert!(tank.fish[octo].hurry_zoomie());
    tick(&mut tank, DAY, 2);
    assert!(!tank.inks.is_empty(), "it jets away in a cloud of ink");
    tank.fish[octo].habits.escape_clock = Some(0.0);
    let events = tick(&mut tank, DAY, 1);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, TankEvent::Wander { fish_name } if fish_name == "Inky")),
        "an octopus escapes"
    );
}

#[test]
fn an_octopus_escapes_to_another_tank_with_room() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/spawn pulpo \"Inky\"");
    let base = tui.app.current_tank;
    tui.app.tanks.push(tank_of(TankKind::CoralReef));
    let reef = tui.app.tanks.len() - 1;
    let inky = tui.app.tanks[base]
        .fish
        .iter_mut()
        .find(|f| f.name == "Inky")
        .unwrap();
    inky.habits.escape_clock = Some(0.0);
    tui.tick_n(2);
    assert!(tui.app.tanks[reef].fish.iter().any(|f| f.name == "Inky"));
    assert!(!tui.app.tanks[base].fish.iter().any(|f| f.name == "Inky"));
}

#[test]
fn an_icefish_lets_the_tank_show_through() {
    let mut tank = tank_of(TankKind::Base);
    let ice = add(&mut tank, FishSpecies::Draco, "Hielo");
    tank.fish[ice].frozen = true;
    tank.fish[ice].facing = Direction::Left;
    tank.fish[ice].position.x = 20.0;
    tank.fish[ice].position.y = 8.0;
    let mut rng = rand::rng();
    let behind = Bubble::new(
        23.0,
        8.0,
        BubblePhase::rising(&mut rng),
        Color::Cyan,
        Some('@'),
        &mut rng,
    );
    tank.bubbles = vec![behind];
    let buf = render(&tank);
    assert_eq!(
        buf[(23, 8)].symbol(),
        "@",
        "the bubble shows through the icefish"
    );
    let width = tank.fish[ice].display_width as u16;
    assert!(
        (20..20 + width).any(|x| buf[(x, 8)].modifier.contains(Modifier::DIM)),
        "over open water its body is only a faint outline"
    );
}

#[test]
fn a_flounder_wears_the_colour_of_its_water_and_of_whatever_lies_under_it() {
    for kind in [TankKind::Base, TankKind::Hell, TankKind::Candy] {
        let mut tank = tank_of(kind);
        let empty = render(&tank);
        let flat = add(&mut tank, FishSpecies::Lenguado, "Plano");
        let fish = &mut tank.fish[flat];
        fish.frozen = true;
        fish.position.x = 20.0;
        fish.position.y = 8.0;
        fish.facing = Direction::Left;
        let water = tank.water_color();
        let width = tank.fish[flat].display_width as u16;
        let drawn = render(&tank);
        let mut body_cells = 0;
        for x in 20..20 + width {
            let symbol = drawn[(x, 8)].symbol();
            if symbol == "º" || symbol == "ʘ" || symbol == "¯" || symbol == " " {
                continue;
            }
            body_cells += 1;
            let under = &empty[(x, 8)];
            let expected = if under.symbol() == " " || under.fg == Color::Reset {
                water
            } else {
                under.fg
            };
            assert_eq!(
                drawn[(x, 8)].fg,
                expected,
                "{} column {x}: it takes the colour of what lies under it",
                kind.display_name()
            );
        }
        assert!(body_cells > 0);
    }
}

#[test]
fn selling_a_hermit_crab_leaves_its_shell_in_the_bag() {
    let mut tui = Tui::new();
    tui.clear_tank();
    tui.run("/spawn ermitano \"Pagu\"");
    let junk = tui.app.stock_of(StockItem::Junk);
    tui.run("/sell fish \"Pagu\"");
    assert_eq!(tui.app.stock_of(StockItem::Junk), junk + 1);
}

#[test]
fn two_hermit_crabs_that_meet_swap_shells() {
    let mut tank = tank_of(TankKind::Base);
    let a = add(&mut tank, FishSpecies::Ermitano, "A");
    let b = add(&mut tank, FishSpecies::Ermitano, "B");
    let (seed_a, seed_b) = (tank.fish[a].pattern_seed, tank.fish[b].pattern_seed);
    tank.fish[a].position.x = 20.0;
    tank.fish[b].position.x = 24.0;
    for i in [a, b] {
        tank.fish[i].velocity.dx = 0.0;
    }
    tick(&mut tank, DAY, 1);
    assert_eq!(tank.fish[a].pattern_seed, seed_b);
    assert_eq!(tank.fish[b].pattern_seed, seed_a);
}

#[test]
fn seahorses_that_meet_send_up_a_heart() {
    let mut tank = tank_of(TankKind::Base);
    let a = add(&mut tank, FishSpecies::Caballito, "Romeo");
    let b = add(&mut tank, FishSpecies::Caballito, "Julieta");
    for (i, x) in [(a, 20.0), (b, 24.0)] {
        tank.fish[i].position.x = x;
        tank.fish[i].position.y = 6.0;
        tank.fish[i].velocity.dx = 0.0;
        tank.fish[i].velocity.dy = 0.0;
    }
    tick(&mut tank, DAY, 1);
    assert!(tank.bubbles.iter().any(|b| b.bubble_char == 'ღ'));
    assert!(
        !tank.fish[a].facing_left() && tank.fish[b].facing_left(),
        "they face each other"
    );
}

#[test]
fn a_stonefish_snaps_up_food_that_settles_by_it() {
    let mut tank = tank_of(TankKind::Base);
    let stone = add(&mut tank, FishSpecies::Piedra, "Roca");
    tick(&mut tank, DAY, 1);
    let weight = tank.fish[stone].weight_g;
    let x = tank.fish[stone].position.x + tank.fish[stone].display_width as f32 + 1.0;
    let mut food = Food::new(x);
    food.position.y = TANK_H as f32 - 1.0;
    food.settled = true;
    tank.food.push(food);
    tick(&mut tank, DAY, 1);
    assert!(tank.fish[stone].weight_g > weight, "it ate without moving");
    assert!(tank.fish[stone].habits.alert > 0.0, "and its eye opened");
}

#[test]
fn a_catfish_says_miau() {
    let mut rng = rand::rng();
    let cat = Fish::new(FishSpecies::Bagre, "Michi".into(), 0.0, 0.0, &mut rng);
    let quote = field_value(FieldKind::FavoriteQuote, &cat, &[], &mut rng).text;
    assert!(quote.split(' ').all(|word| word == "miau"), "{quote}");
}

#[test]
fn a_star_twinkles_at_night() {
    let mut rng = rand::rng();
    let mut star = Fish::new(FishSpecies::Estrella, "Patricio".into(), 0.0, 0.0, &mut rng);
    star.sky = DAY;
    let day = star.line_sprite().rows[0][0].1;
    star.sky = NIGHT;
    let night = star.line_sprite().rows[0][0].1;
    assert_ne!(day, night);
}

#[test]
fn the_sunfish_wears_the_moon() {
    let mut rng = rand::rng();
    let mut mola = Fish::new(FishSpecies::Luna, "Mola".into(), 0.0, 0.0, &mut rng);
    let body_colors = |fish: &Fish| -> Vec<Color> {
        let sprite = fish.line_sprite();
        let (lo, hi) = fish.body_span().unwrap();
        sprite.rows[sprite.body_row][lo..=hi]
            .iter()
            .map(|&(_, c)| c)
            .collect()
    };
    mola.sky = Sky {
        moon: FULL_MOON,
        ..DAY
    };
    assert!(body_colors(&mola).iter().all(|&c| c == MOON_LIT));
    mola.sky = Sky { moon: 0, ..DAY };
    assert!(body_colors(&mola).iter().all(|&c| c == MOON_DARK));
    mola.sky = Sky {
        moon: LUNAR_MONTH_DAYS / 4,
        ..DAY
    };
    let half = body_colors(&mola);
    assert!(half.contains(&MOON_LIT) && half.contains(&MOON_DARK));
}

#[test]
fn every_rare_comes_back_from_a_save_with_its_birthmarks() {
    let mut tui = Tui::new();
    tui.clear_tank();
    let tank = tui.app.current_tank;
    let mut rng = rand::rng();
    for species in NEW_RARES {
        let fish = Fish::new(
            species,
            species.display_name().to_string(),
            5.0,
            5.0,
            &mut rng,
        );
        tui.app.tanks[tank].fish.push(fish);
    }
    let save = tui.app.snapshot();
    let back = fishtank::app::App::resume(save, 100, 30);
    for species in NEW_RARES {
        let name = species.display_name();
        let before = tui.app.tanks[tank]
            .fish
            .iter()
            .find(|f| f.name == name)
            .unwrap();
        let after = back.tanks[tank]
            .fish
            .iter()
            .find(|f| f.name == name)
            .unwrap();
        assert_eq!(after.adornments(), before.adornments(), "{name}");
        assert_eq!(after.circadian_state(), before.circadian_state(), "{name}");
        assert_eq!(after.display_width, before.display_width, "{name}");
    }
}

#[test]
fn a_figure_cannot_take_a_mutation_it_cannot_draw() {
    let mut rng = rand::rng();
    for &species in ALL_SPECIES {
        if !matches!(species.config().body, BodyTemplate::Figure(_)) {
            continue;
        }
        let fish = Fish::new(species, "F".into(), 0.0, 0.0, &mut rng);
        for mutation in [
            Mutation::Lure,
            Mutation::Telophase,
            Mutation::Feet,
            Mutation::Ear,
        ] {
            assert!(
                !fish.supports_now(mutation),
                "{}: {}",
                species.display_name(),
                mutation.token()
            );
        }
        assert!(fish.supports_now(Mutation::BodyColor));
    }
}

#[test]
fn glass_and_floor_dwellers_never_swim_after_food() {
    let mut tank = tank_of(TankKind::Base);
    let snail = add(&mut tank, FishSpecies::Caracol, "Shelly");
    let crab = add(&mut tank, FishSpecies::Ermitano, "Pagu");
    tank.food.push(Food::new(30.0));
    for _ in 0..50 {
        tick(&mut tank, DAY, 1);
        for i in [snail, crab] {
            assert!(!matches!(tank.fish[i].state, FishState::SeekingFood { .. }));
            assert!(tank.fish[i].locomotion() != Locomotion::Swim);
        }
    }
}

#[test]
fn a_crawler_on_the_wall_is_drawn_upright() {
    let mut rng = rand::rng();
    let mut snail = Fish::new(FishSpecies::Caracol, "S".into(), 0.0, 0.0, &mut rng);
    snail.habits.crawl = Some(fishtank::fishes::habits::Crawl {
        side: Side::Right,
        clockwise: false,
        along: 0.0,
    });
    snail.facing = Direction::Left;
    let rows = rows_of(&snail);
    assert!(
        rows.len() > 1,
        "a snail climbing a wall stands on end: {rows:?}"
    );
    assert!(rows.iter().all(|row| row.trim().chars().count() == 1));
    assert_eq!(rows[0].trim(), "º", "its eye leads the way up");
}

const REEL_DIR: &str = env!("CARGO_TARGET_TMPDIR");
const CARD_SIZES: [(u16, u16); 4] = [(100, 26), (60, 18), (40, 14), (28, 10)];

#[test]
fn every_new_rare_draws_whole_on_its_catch_card_at_every_size() {
    use fishtank::loot::LootKind;
    use fishtank::testing::{Reel, Still};
    use fishtank::ui::{
        catch_overlay::{CatchOverlay, CatchState},
        layout::Screen,
    };
    let mut reel = Reel::new();
    for species in NEW_RARES {
        for (cols, rows) in CARD_SIZES {
            let state = CatchState::new(LootKind::Fish(species), &mut rand::rng());
            let area = Rect::new(0, 0, cols, rows);
            let mut buffer = Buffer::empty(area);
            CatchOverlay::new(&state, Screen::only(area)).render(area, &mut buffer);
            let label = format!("{} · {cols}×{rows}", species.display_name());
            let still = Still::of(&label, &buffer);
            assert!(
                still.text().contains(species.display_name()),
                "{label} names its catch:\n{}",
                still.text()
            );
            reel.push(still);
        }
    }
    reel.save(std::path::Path::new(REEL_DIR), "rare-catch-cards")
        .expect("the reel is writable");
    let report = reel.flaw_report();
    assert!(report.is_empty(), "{report}");
}

#[test]
fn a_blind_fish_sparks_when_it_meets_the_glass() {
    let mut tank = tank_of(TankKind::Base);
    let ray = add(&mut tank, FishSpecies::Ciego, "Ray");
    let fish = &mut tank.fish[ray];
    fish.position.x = TANK_W as f32 - fish.display_width as f32 - 0.5;
    fish.position.y = 8.0;
    fish.facing = Direction::Right;
    fish.velocity.dx = 4.0;
    fish.velocity.dy = 0.0;
    tick(&mut tank, DAY, 3);
    assert!(tank.fish[ray].velocity.dx < 0.0, "it turned at the glass");
    assert!(
        tank.bubbles.iter().any(|b| b.bubble_char == '*'),
        "and sparked where it hit"
    );
}
