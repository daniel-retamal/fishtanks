use fishtank::app::App;
use fishtank::entities::food::FOOD_WEIGHT_GAIN_G;
use fishtank::fishes::fish::{Direction, Fish, FishState};
use fishtank::fishes::mutations::Mutatable;
use fishtank::fishes::quirk::{Cling, Quirk, SIGNAL_GAP_BITS};
use fishtank::fishes::species::FishSpecies;
use fishtank::fishes::unfish::{SPAWNABLE_UNFISH, UnfishKind};
use fishtank::settings::Settings;
use fishtank::tank::{Sky, Tank, TankBackground, TankKind};
use fishtank::ui::tank_view::TankView;
use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

const W: u16 = 60;
const H: u16 = 14;
const PATIENCE: usize = 4000;
const PELLET: u32 = FOOD_WEIGHT_GAIN_G;

fn lab() -> Tank {
    let mut tank = Tank::new("Lab".into(), TankKind::Base, &[]);
    tank.resize(W, H, &[]);
    if let TankBackground::Plain { plants } = &mut tank.background {
        plants.clear();
    }
    tank
}

fn add(tank: &mut Tank, fish: Fish, x: f32, y: f32) -> usize {
    let name = fish.name.clone();
    tank.place_fish(fish, name.clone(), &mut rand::rng());
    let i = index_of(tank, &name);
    tank.fish[i].position.x = x;
    tank.fish[i].position.y = y;
    i
}

fn index_of(tank: &Tank, name: &str) -> usize {
    tank.fish
        .iter()
        .position(|fish| fish.name == name)
        .expect("the fish is in the tank")
}

fn unfish(kind: UnfishKind, name: &str) -> Fish {
    Fish::new_unfish(kind, name.into(), 0.0, 0.0, &mut rand::rng())
}

fn fish(species: FishSpecies, name: &str) -> Fish {
    Fish::new(species, name.into(), 0.0, 0.0, &mut rand::rng())
}

fn render(tank: &Tank) -> Buffer {
    let area = Rect::new(0, 0, tank.width, tank.height);
    let mut buf = Buffer::empty(area);
    TankView::new(tank).render(area, &mut buf);
    buf
}

fn rows(buf: &Buffer) -> Vec<String> {
    let area = buf.area;
    (0..area.height)
        .map(|y| (0..area.width).map(|x| buf[(x, y)].symbol()).collect())
        .collect()
}

fn tick(tank: &mut Tank) {
    tank.tick(&Settings::default(), 0, Sky::default());
}

fn fed(species: FishSpecies, name: &str, extra_g: u32) -> Fish {
    let mut fish = fish(species, name);
    let base = species.config().weight_base[fish.size_category as usize];
    fish.weight_g = base + extra_g;
    fish
}

fn leech_state(fish: &Fish) -> (Option<String>, Option<Cling>, u32) {
    match fish.quirk() {
        Some(Quirk::Leech(leech)) => (leech.host.clone(), leech.cling, leech.drunk_g),
        _ => panic!("not a leech"),
    }
}

fn hurry_leeches(tank: &mut Tank) {
    for fish in &mut tank.fish {
        if let Some(Quirk::Leech(leech)) = fish.quirk_mut() {
            leech.bite_clock = 0.0;
        }
    }
}

#[test]
fn every_unfish_can_be_sold_and_only_nothing_takes_no_seat() {
    for &kind in SPAWNABLE_UNFISH {
        let fish = unfish(kind, "Probe");
        assert!(fish.is_sellable(), "{kind:?} can be sold");
        assert_eq!(fish.sell_value(), 0, "{kind:?} is worth nothing");
        assert_eq!(fish.takes_a_seat(), kind != UnfishKind::Absence, "{kind:?}");
    }
}

#[test]
fn an_absence_comes_even_to_a_full_tank_and_takes_no_room() {
    let mut tank = lab();
    while !tank.is_full() {
        let name = format!("Filler {}", tank.fish.len());
        add(&mut tank, fish(FishSpecies::Salmon, &name), 5.0, 5.0);
    }
    assert!(tank.spawn_unfish_of(UnfishKind::Absence, &mut rand::rng()));
    assert!(tank.is_full());
    assert!(!tank.spawn_unfish_of(UnfishKind::Fault, &mut rand::rng()));
}

#[test]
fn an_absence_erases_whatever_lies_under_its_shape() {
    let mut tank = lab();
    for (i, x) in (0..20).map(|i| (i, 10.0 + i as f32)) {
        add(
            &mut tank,
            fish(FishSpecies::Salmon, &format!("S{i}")),
            x,
            6.0,
        );
    }
    let before = rows(&render(&tank));
    let hole = add(&mut tank, unfish(UnfishKind::Absence, "Nothing"), 14.0, 6.0);
    tank.fish[hole].facing = Direction::Left;
    let after = rows(&render(&tank));
    let line_before: Vec<char> = before[6].chars().collect();
    let line_after: Vec<char> = after[6].chars().collect();
    assert_ne!(line_before[16], ' ');
    assert_eq!(line_after[16], ' ', "{}", after[6]);
    assert!(after.iter().all(|row| !row.contains("Nothing")));
}

#[test]
fn a_leech_drinks_only_what_food_put_on_a_fish() {
    let mut tank = lab();
    let host = add(
        &mut tank,
        fed(FishSpecies::Salmon, "Host", PELLET),
        20.0,
        5.0,
    );
    let base = tank.fish[host].weight_g - PELLET;
    add(&mut tank, unfish(UnfishKind::Leech, "Leech"), 20.0, 6.0);
    for _ in 0..PATIENCE {
        hurry_leeches(&mut tank);
        tick(&mut tank);
    }
    let host = index_of(&tank, "Host");
    assert_eq!(tank.fish[host].weight_g, base);
    let leech = index_of(&tank, "Leech");
    assert!(!tank.fish[leech].leeched);
    assert_eq!(leech_state(&tank.fish[leech]).2, PELLET);
}

#[test]
fn a_leech_that_drank_a_whole_fish_becomes_its_pale_copy() {
    let mut tank = lab();
    add(
        &mut tank,
        fed(FishSpecies::Cashfish, "Midas", 5000),
        20.0,
        5.0,
    );
    add(&mut tank, unfish(UnfishKind::Leech, "XII"), 20.0, 6.0);
    for _ in 0..PATIENCE {
        hurry_leeches(&mut tank);
        tick(&mut tank);
        if tank.fish[index_of(&tank, "XII")].leeched {
            break;
        }
    }
    let copy = &tank.fish[index_of(&tank, "XII")];
    assert!(copy.leeched, "the leech became a copy");
    assert_eq!(copy.species, FishSpecies::Cashfish);
    assert_eq!(copy.ability_stacks(FishSpecies::Cashfish), 1);
    assert_eq!(copy.kind_name(), "Unfish");
    assert_eq!(copy.sell_value(), 0);
    assert!(copy.is_unfish());
    let host = &tank.fish[index_of(&tank, "Midas")];
    let colours = |fish: &Fish| -> Vec<Color> {
        fish.line_sprite()
            .rows
            .iter()
            .flatten()
            .map(|&(_, color)| color)
            .collect()
    };
    assert_ne!(colours(copy), colours(host), "the copy is flesh-coloured");
}

#[test]
fn a_fish_carries_two_leeches_at_most_one_above_one_below() {
    let mut tank = lab();
    add(
        &mut tank,
        fed(FishSpecies::Salmon, "Host", 100_000),
        20.0,
        6.0,
    );
    for name in ["I", "II", "III"] {
        add(&mut tank, unfish(UnfishKind::Leech, name), 21.0, 6.0);
    }
    for _ in 0..50 {
        for name in ["I", "II", "III"] {
            let leech = index_of(&tank, name);
            if leech_state(&tank.fish[leech]).0.is_none() {
                tank.fish[leech].position.x = 21.0;
                tank.fish[leech].position.y = tank.fish[index_of(&tank, "Host")].position.y;
            }
        }
        tick(&mut tank);
    }
    let clings: Vec<Cling> = ["I", "II", "III"]
        .iter()
        .filter_map(|name| leech_state(&tank.fish[index_of(&tank, name)]).1)
        .collect();
    assert_eq!(clings.len(), 2);
    assert!(clings.contains(&Cling::Above) && clings.contains(&Cling::Below));
}

fn bones_total(tank: &Tank) -> (usize, usize) {
    let bones: Vec<&Fish> = tank
        .fish
        .iter()
        .filter(|fish| fish.unfish_kind() == Some(UnfishKind::Bones))
        .collect();
    (bones.len(), bones.iter().map(|fish| fish.body_size).sum())
}

fn scatter(tank: &mut Tank, remaining: f32) {
    for fish in &mut tank.fish {
        if fish.unfish_kind() == Some(UnfishKind::Bones) {
            fish.state = FishState::Zoomie {
                time_remaining: remaining,
                total_duration: 3.0,
                will_turn: false,
                has_turned: false,
            };
        }
    }
}

#[test]
fn bones_come_back_as_any_number_of_skeletons_and_keep_every_rib() {
    let (mut split, mut merged) = (false, false);
    for _ in 0..200 {
        let mut tank = lab();
        for (k, ribs) in [3usize, 4, 5].into_iter().enumerate() {
            let i = add(
                &mut tank,
                unfish(UnfishKind::Bones, &format!("B{k}")),
                10.0 + 12.0 * k as f32,
                6.0,
            );
            tank.fish[i].body_size = ribs;
        }
        scatter(&mut tank, 2.0);
        tick(&mut tank);
        scatter(&mut tank, 0.001);
        tick(&mut tank);
        let (count, ribs) = bones_total(&tank);
        assert_eq!(ribs, 12, "the ribs are conserved");
        assert!((1..=12).contains(&count));
        split |= count > 3;
        merged |= count < 3;
    }
    assert!(split, "three skeletons sometimes come back as more");
    assert!(merged, "three skeletons sometimes come back as fewer");
}

#[test]
fn a_scattering_skeleton_draws_its_bones_apart() {
    let mut tank = lab();
    let i = add(&mut tank, unfish(UnfishKind::Bones, "Bones"), 25.0, 6.0);
    let whole: usize = rows(&render(&tank))
        .iter()
        .filter(|row| !row.trim().is_empty())
        .count();
    tank.fish[i].state = FishState::Zoomie {
        time_remaining: 1.5,
        total_duration: 3.0,
        will_turn: false,
        has_turned: false,
    };
    let apart: usize = rows(&render(&tank))
        .iter()
        .filter(|row| !row.trim().is_empty())
        .count();
    assert_eq!(whole, 1);
    assert!(apart > 1, "the bones leave their row");
}

#[test]
fn a_signal_spells_its_cheat_code_on_a_channel_named_after_it() {
    let mut tank = lab();
    let i = add(&mut tank, unfish(UnfishKind::Signal, "XIV"), 20.0, 6.0);
    let (code, period) = match tank.fish[i].quirk() {
        Some(Quirk::Signal(signal)) => (signal.cheat.code(), signal.period() as usize),
        _ => panic!("a signal"),
    };
    let mut bits = Vec::new();
    for _ in 0..period * 2 {
        let mut world = tank.observe(0, 0);
        tank.advance_stage(&mut world);
        bits.push(tank.channels.level("xiv"));
    }
    let gap = SIGNAL_GAP_BITS as usize;
    let start = (gap..bits.len())
        .find(|&k| bits[k] && bits[k - gap..k].iter().all(|bit| !bit))
        .expect("a pause, then a start bit");
    let mut spelled = String::new();
    let mut k = start;
    while k + 10 <= bits.len() && bits[k] {
        let byte = (0..8).fold(0u8, |acc, b| acc | (u8::from(bits[k + 1 + b]) << b));
        assert!(!bits[k + 9], "a stop bit ends every letter");
        spelled.push(byte as char);
        k += 10;
    }
    assert_eq!(spelled, code);
}

#[test]
fn whatever_swims_into_the_ring_comes_out_of_the_far_wall() {
    let mut tank = lab();
    add(&mut tank, unfish(UnfishKind::Ouroboros, "Ring"), 20.0, 6.0);
    let s = add(&mut tank, fish(FishSpecies::Salmon, "Traveller"), 26.0, 6.0);
    tank.fish[s].facing = Direction::Left;
    tank.fish[s].velocity.dx = -1.0;
    tank.fish[s].velocity.dy = 0.0;
    tick(&mut tank);
    let traveller = &tank.fish[index_of(&tank, "Traveller")];
    assert!(traveller.position.x > 40.0, "{}", traveller.position.x);
}

#[test]
fn the_ring_turns_its_head_round_the_rim() {
    let mut tank = lab();
    add(&mut tank, unfish(UnfishKind::Ouroboros, "Ring"), 20.0, 6.0);
    let head = |tank: &Tank| -> (usize, usize) {
        let rows = rows(&render(tank));
        rows.iter()
            .enumerate()
            .find_map(|(y, row)| row.find("(º)").map(|x| (x, y)))
            .expect("the head is drawn")
    };
    let first = head(&tank);
    for _ in 0..20 {
        tick(&mut tank);
        let i = index_of(&tank, "Ring");
        tank.fish[i].position.x = 20.0;
        tank.fish[i].position.y = 6.0;
    }
    assert_ne!(head(&tank), first);
}

#[test]
fn the_negative_develops_a_fish_and_a_second_touch_restores_it() {
    let mut tank = lab();
    let s = add(&mut tank, fish(FishSpecies::Salmon, "Sitter"), 20.0, 6.0);
    tank.fish[s].frozen = true;
    let before = tank.fish[s].line_sprite().rows;
    let n = add(&mut tank, unfish(UnfishKind::Negative, "Neg"), 21.0, 6.0);
    tank.fish[n].frozen = true;
    tick(&mut tank);
    let sitter = &tank.fish[index_of(&tank, "Sitter")];
    assert!(sitter.is_negative());
    assert_ne!(sitter.line_sprite().rows, before);
    let neg = index_of(&tank, "Neg");
    if let Some(Quirk::Negative(negative)) = tank.fish[neg].quirk_mut() {
        negative.rests.clear();
    }
    tick(&mut tank);
    assert!(!tank.fish[index_of(&tank, "Sitter")].is_negative());
}

#[test]
fn the_negative_never_touches_an_unfish() {
    let mut tank = lab();
    let o = add(&mut tank, unfish(UnfishKind::Fault, "Other"), 20.0, 6.0);
    tank.fish[o].frozen = true;
    let n = add(&mut tank, unfish(UnfishKind::Negative, "Neg"), 21.0, 6.0);
    tank.fish[n].frozen = true;
    tick(&mut tank);
    assert_eq!(tank.fish[index_of(&tank, "Other")].mutation_count(), 0);
}

#[test]
fn the_forgetting_wears_a_question_for_an_eye_and_forgets_its_mutations() {
    let mut tank = lab();
    let f = add(&mut tank, unfish(UnfishKind::Forgetting, "Who"), 20.0, 6.0);
    let cells: String = tank.fish[f].segments().iter().map(|&(c, _)| c).collect();
    assert!(cells.contains('?'), "{cells}");
    assert!(tank.apply_named_mutation("Who", "bodycolor"));
    assert!(tank.fish[f].record().is_some_and(|r| !r.history.is_empty()));
    let f = index_of(&tank, "Who");
    if let Some(Quirk::Forgetting(forgetting)) = tank.fish[f].quirk_mut() {
        forgetting.forget_clock = 0.0;
    }
    tick(&mut tank);
    let who = &tank.fish[index_of(&tank, "Who")];
    assert!(who.record().is_none_or(|r| r.history.is_empty()));
}

#[test]
fn each_side_of_a_verso_keeps_its_own_body_and_mutations() {
    let mut tank = lab();
    let v = add(&mut tank, unfish(UnfishKind::Verso, "Two"), 20.0, 6.0);
    tank.fish[v].frozen = true;
    tank.fish[v].facing = Direction::Left;
    if let Some(Quirk::Verso(verso)) = tank.fish[v].quirk_mut() {
        verso.facing_left = true;
        verso.side.size = 2;
        verso.other.size = 8;
    }
    tank.fish[v].body_size = 2;
    assert!(tank.apply_named_mutation("Two", "ear"));
    let left_width = tank.fish[v].display_width;
    tank.fish[v].facing = Direction::Right;
    tank.fish[v].frozen = false;
    tank.fish[v].velocity.dx = 0.0;
    tick(&mut tank);
    let v = index_of(&tank, "Two");
    assert_eq!(tank.fish[v].body_size, 8);
    assert_ne!(tank.fish[v].display_width, left_width);
    assert_eq!(
        tank.fish[v].ear_count(),
        0,
        "the ear stayed on the left side"
    );
}

#[test]
fn a_fault_slips_its_back_half_a_row_down() {
    let mut tank = lab();
    let f = add(&mut tank, unfish(UnfishKind::Fault, "Crack"), 20.0, 6.0);
    let whole = rows(&render(&tank));
    assert!(whole[7].trim().is_empty());
    if let Some(Quirk::Fault(fault)) = tank.fish[f].quirk_mut() {
        fault.slip = 1.0;
    }
    let slipped = rows(&render(&tank));
    assert!(!slipped[7].trim().is_empty(), "{}", slipped[7]);
}

#[test]
fn an_anagram_is_made_of_its_neighbours_parts() {
    let mut tank = lab();
    let s = add(&mut tank, fish(FishSpecies::Salmon, "Model"), 20.0, 6.0);
    tank.fish[s].frozen = true;
    let a = add(&mut tank, unfish(UnfishKind::Anagram, "Mix"), 30.0, 6.0);
    tank.fish[a].frozen = true;
    tick(&mut tank);
    let sorted = |fish: &Fish| -> Vec<char> {
        let mut glyphs: Vec<char> = fish
            .static_left_segments()
            .iter()
            .map(|&(c, _)| c)
            .filter(|&c| c != ' ' && c != '\0')
            .collect();
        glyphs.sort_unstable();
        glyphs
    };
    let model = &tank.fish[index_of(&tank, "Model")];
    let mix = &tank.fish[index_of(&tank, "Mix")];
    assert_eq!(sorted(mix), sorted(model));
}

#[test]
fn a_reflection_hangs_under_its_fish() {
    let mut tank = lab();
    add(&mut tank, unfish(UnfishKind::Reflection, "Twin"), 20.0, 6.0);
    let rows = rows(&render(&tank));
    assert_eq!(rows[6].trim(), rows[7].trim());
}

#[test]
fn a_molt_sheds_its_old_self_when_it_mutates() {
    let mut tank = lab();
    add(&mut tank, unfish(UnfishKind::Molt, "Skin"), 20.0, 3.0);
    assert!(tank.apply_named_mutation("Skin", "ear"));
    assert_eq!(tank.sheddings.len(), 1);
    let y = tank.sheddings[0].y;
    for _ in 0..30 {
        tick(&mut tank);
    }
    assert!(tank.sheddings.first().is_some_and(|shed| shed.y > y));
}

#[test]
fn the_face_looks_at_you_whichever_way_it_swims() {
    let mut tank = lab();
    let f = add(&mut tank, unfish(UnfishKind::Face, "Face"), 20.0, 6.0);
    for facing in [Direction::Left, Direction::Right] {
        tank.fish[f].facing = facing;
        if let Some(us) = tank.fish[f].unfish_state.as_mut() {
            us.eye.is_open = true;
        }
        let row = rows(&render(&tank))[6].clone();
        assert!(row.contains("(°‿°)"), "{row}");
        let area = Rect::new(0, 0, W, H);
        let mut buf = Buffer::empty(area);
        TankView::new(&tank)
            .with_typing(true)
            .render(area, &mut buf);
        assert!(rows(&buf)[6].contains("(._.)"));
    }
}

fn sighted(fish: &Fish) -> bool {
    matches!(fish.quirk(), Some(Quirk::Graeae(graeae)) if graeae.sighted)
}

#[test]
fn the_graeae_share_one_eye_between_every_tank() {
    let mut app = App::new();
    let mut second = lab();
    let mut rng = rand::rng();
    for name in ["Deino", "Enyo"] {
        app.tanks[0].place_fish(unfish(UnfishKind::Graeae, name), name.into(), &mut rng);
    }
    second.place_fish(
        unfish(UnfishKind::Graeae, "Pemphredo"),
        "Pemphredo".into(),
        &mut rng,
    );
    app.tanks.push(second);
    for _ in 0..3 {
        app.tick();
    }
    let eyes = app
        .tanks
        .iter()
        .flat_map(|tank| tank.fish.iter())
        .filter(|fish| sighted(fish))
        .count();
    assert_eq!(eyes, 1);
}

#[test]
fn a_blind_sister_who_touches_the_one_with_the_eye_takes_it() {
    let mut tank = lab();
    let a = add(&mut tank, unfish(UnfishKind::Graeae, "Deino"), 20.0, 6.0);
    let b = add(&mut tank, unfish(UnfishKind::Graeae, "Enyo"), 21.0, 6.0);
    for i in [a, b] {
        tank.fish[i].frozen = true;
    }
    if let Some(Quirk::Graeae(graeae)) = tank.fish[a].quirk_mut() {
        graeae.sighted = true;
    }
    tick(&mut tank);
    assert!(!sighted(&tank.fish[index_of(&tank, "Deino")]));
    assert!(sighted(&tank.fish[index_of(&tank, "Enyo")]));
    let blind: String = tank.fish[index_of(&tank, "Deino")]
        .segments()
        .iter()
        .map(|&(c, _)| c)
        .collect();
    assert!(blind.contains('-'), "{blind}");
}

#[test]
fn the_still_moves_only_while_its_tank_is_not_watched() {
    let mut tank = lab();
    add(&mut tank, fish(FishSpecies::Salmon, "Prey"), 45.0, 3.0);
    add(&mut tank, unfish(UnfishKind::Still, "Still"), 5.0, 10.0);
    tank.watched = true;
    for _ in 0..200 {
        tick(&mut tank);
    }
    let still = &tank.fish[index_of(&tank, "Still")];
    assert_eq!((still.position.x, still.position.y), (5.0, 10.0));
    tank.watched = false;
    for _ in 0..600 {
        tick(&mut tank);
    }
    let still = &tank.fish[index_of(&tank, "Still")];
    let prey = &tank.fish[index_of(&tank, "Prey")];
    assert!((still.position.y - prey.position.y).abs() < 2.0);
}

#[test]
fn every_unfish_survives_a_save() {
    for &kind in SPAWNABLE_UNFISH {
        let fish = unfish(kind, "Saved");
        let text = ron::to_string(&fish).expect("saves");
        let back: Fish = ron::from_str(&text).expect("loads");
        assert_eq!(back.unfish_kind(), Some(kind));
        assert_eq!(back.display_width, fish.display_width, "{kind:?}");
    }
    let mut copy = fish(FishSpecies::Salmon, "Copy");
    copy.leeched = true;
    let back: Fish = ron::from_str(&ron::to_string(&copy).expect("saves")).expect("loads");
    assert!(back.leeched);
}
