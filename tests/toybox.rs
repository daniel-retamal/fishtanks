use std::path::Path;

use crossterm::event::KeyCode;
use fishtank::fishes::fish::Fish;
use fishtank::fishes::species::SizeCategory;
use fishtank::fishes::toy::{
    FittedPart, Line, Material, Paint, Shelf, Signature, ToyColor, ToyPart, ToyState,
};
use fishtank::testing::Tui;

const SIZES: [(u16, u16); 4] = [(100, 30), (60, 18), (40, 14), (28, 10)];

fn toy_named(tui: &mut Tui, name: &str, toy: ToyState, size: SizeCategory) {
    let fish = Fish::new_toy(toy, size, &mut rand::rng());
    tui.app.tanks[0].place_fish(fish, name.to_string(), &mut rand::rng());
}

fn part(part: ToyPart, paint: Paint) -> FittedPart {
    FittedPart { part, paint }
}

fn stocked() -> Tui {
    let mut tui = Tui::new();
    tui.clear_tank();
    toy_named(
        &mut tui,
        "Pip",
        ToyState::plain(ToyColor::Galaxy, Material::Metallic),
        SizeCategory::L,
    );
    toy_named(
        &mut tui,
        "Bolt",
        ToyState::plain(ToyColor::Coral, Material::Plastic),
        SizeCategory::S,
    );
    toy_named(
        &mut tui,
        "Mecha",
        ToyState::signature(Signature::Mecha, false),
        ToyState::signature_size(),
    );
    for fitted in [
        part(ToyPart::Rotor, Paint::Mint),
        part(ToyPart::Wheels, Paint::Charcoal),
        part(ToyPart::Antenna, Paint::Lemon),
        part(ToyPart::Rocket, Paint::Cherry),
    ] {
        tui.app.casino.toybox.add(fitted);
    }
    tui
}

fn pip(tui: &Tui) -> &Fish {
    tui.app
        .tanks
        .iter()
        .flat_map(|t| t.fish.iter())
        .find(|f| f.name == "Pip")
        .expect("Pip")
}

#[test]
fn the_toybox_dresses_a_toy_and_takes_the_part_from_the_box() {
    let mut tui = stocked();
    tui.run("/toybox");
    tui.screen().expect_find("Toybox");
    tui.screen().expect_find("Pip");
    tui.select("Pip");
    tui.key(KeyCode::Enter);
    tui.screen().expect_find("Toybox#edit");
    tui.key(KeyCode::Down);
    tui.key(KeyCode::Right);
    tui.screen().expect_find("Lemon Antenna");
    tui.key(KeyCode::Enter);
    let toy = pip(&tui).toy.as_ref().expect("a toy");
    assert_eq!(toy.fittings.top, Some(part(ToyPart::Antenna, Paint::Lemon)));
    assert_eq!(
        tui.app
            .casino
            .toybox
            .count(part(ToyPart::Antenna, Paint::Lemon)),
        0
    );
}

#[test]
fn escape_puts_every_part_back_where_it_was() {
    let mut tui = stocked();
    tui.run("/toybox");
    tui.select("Pip");
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Down);
    tui.key(KeyCode::Right);
    tui.key(KeyCode::Esc);
    tui.screen().expect_absent("Toybox#edit");
    assert_eq!(pip(&tui).toy.as_ref().unwrap().fittings.top, None);
    assert_eq!(tui.app.casino.toybox.total(), 4);
}

#[test]
fn a_part_taken_off_goes_back_into_the_box() {
    let mut tui = stocked();
    tui.run("/toybox");
    tui.select("Pip");
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Down);
    tui.key(KeyCode::Down);
    tui.key(KeyCode::Right);
    tui.key(KeyCode::Enter);
    let wheels = part(ToyPart::Wheels, Paint::Charcoal);
    assert_eq!(pip(&tui).toy.as_ref().unwrap().fittings.under, Some(wheels));
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Down);
    tui.key(KeyCode::Down);
    tui.key(KeyCode::Left);
    tui.key(KeyCode::Enter);
    assert_eq!(pip(&tui).toy.as_ref().unwrap().fittings.under, None);
    assert_eq!(tui.app.casino.toybox.count(wheels), 1);
}

#[test]
fn a_signature_is_sealed() {
    let mut tui = stocked();
    tui.run("/toybox");
    tui.select("Mecha");
    tui.screen().expect_absent("ENTER dress up");
    tui.key(KeyCode::Enter);
    tui.screen().expect_absent("Toybox#edit");
}

#[test]
fn the_shelf_hides_what_was_never_won() {
    let mut tui = stocked();
    let mut shelf = Shelf::default();
    shelf.shelve(&ToyState::plain(ToyColor::Coral, Material::Plastic));
    for signature in Line::Kaiju.signatures() {
        shelf.shelve(&ToyState::signature(signature, false));
    }
    tui.app.casino.toybox.shelf = shelf;
    tui.run("/toybox");
    tui.key(KeyCode::Tab);
    tui.screen().expect_find("Toybox#shelf");
    tui.screen().expect_find("Colors 1/21");
    tui.screen().expect_find("Kaiju 4/4");
    tui.screen().expect_find("???");
    tui.screen().expect_absent("Golden");
    tui.screen().expect_absent("Mechakaiju");
}

#[test]
fn the_toybox_is_filmed_whole_at_every_size() {
    for (cols, rows) in SIZES {
        let mut tui = stocked();
        tui.resize(cols, rows);
        tui.film(
            Path::new(env!("CARGO_TARGET_TMPDIR")),
            &format!("toybox-{cols}x{rows}"),
        );
        tui.run("/toybox");
        tui.tick_n(10);
        tui.snap("the toys");
        tui.select("Pip");
        tui.key(KeyCode::Enter);
        tui.key(KeyCode::Right);
        tui.tick_n(10);
        tui.snap("a part on the head");
        tui.key(KeyCode::Down);
        tui.key(KeyCode::Right);
        tui.tick_n(10);
        tui.snap("a rotor on top");
        tui.key(KeyCode::Down);
        tui.key(KeyCode::Right);
        tui.tick_n(10);
        tui.snap("wheels under");
        tui.key(KeyCode::Down);
        tui.key(KeyCode::Right);
        tui.tick_n(10);
        tui.snap("a rocket on the tail");
        tui.key(KeyCode::Enter);
        tui.key(KeyCode::Tab);
        tui.tick_n(4);
        tui.snap("the shelf");
        tui.write_reel();
        let flaws = tui.reel().flaw_report();
        assert!(flaws.is_empty(), "{cols}x{rows}: {flaws}");
    }
}

#[test]
fn a_dressed_toy_its_box_and_its_shelf_survive_a_save() {
    use fishtank::app::{App, SaveFile};
    let mut tui = stocked();
    tui.run("/toybox");
    tui.select("Pip");
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Down);
    tui.key(KeyCode::Right);
    tui.key(KeyCode::Enter);
    tui.app
        .casino
        .toybox
        .shelf
        .shelve(&ToyState::signature(Signature::Kaiju, true));
    let text = tui.app.snapshot().to_ron().expect("a save");
    let back = App::resume(SaveFile::from_ron(&text).expect("it loads"), 100, 30);
    let pip = back
        .tanks
        .iter()
        .flat_map(|t| t.fish.iter())
        .find(|f| f.name == "Pip")
        .expect("Pip comes back");
    let toy = pip.toy.as_ref().expect("still a toy");
    assert_eq!(toy.fittings.top, Some(part(ToyPart::Antenna, Paint::Lemon)));
    assert_eq!(pip.display_width, toy.width(pip.size_category));
    assert_eq!(back.casino.toybox, tui.app.casino.toybox);
    assert!(back.casino.toybox.shelf.shinies.contains(&Signature::Kaiju));
}

fn dressed(tui: &mut Tui, name: &str, fittings: &[FittedPart]) {
    let mut toy = ToyState::plain(ToyColor::Lime, Material::Plastic);
    for fitted in fittings {
        toy.fittings.set(fitted.part.slot(), Some(*fitted));
    }
    toy_named(tui, name, toy, SizeCategory::M);
}

fn find<'a>(tui: &'a Tui, name: &str) -> &'a Fish {
    tui.app
        .tanks
        .iter()
        .flat_map(|t| t.fish.iter())
        .find(|f| f.name == name)
        .expect("the toy")
}

#[test]
fn wheels_keep_a_toy_on_the_gravel_and_a_rotor_at_the_surface() {
    let mut tui = Tui::new();
    tui.clear_tank();
    dressed(
        &mut tui,
        "Roller",
        &[part(ToyPart::Wheels, Paint::Charcoal)],
    );
    dressed(&mut tui, "Hover", &[part(ToyPart::Rotor, Paint::Mint)]);
    dressed(&mut tui, "Plain", &[]);
    tui.tick_n(400);
    let height = tui.app.tanks[0].height;
    let roller = find(&tui, "Roller");
    assert!((roller.position.y - roller.floor_y(height)).abs() < 3.5);
    let hover = find(&tui, "Hover");
    let above = hover.line_sprite().body_row as f32;
    assert!(hover.position.y <= above + 0.5, "{}", hover.position.y);
    let plain = find(&tui, "Plain");
    assert!(plain.position.y <= f32::from(height) / 3.0 + 1.0);
}

#[test]
fn dressing_a_slot_lists_its_parts_on_hand_six_at_a_time() {
    let mut tui = stocked();
    let unders = [
        part(ToyPart::Feet, Paint::Coral),
        part(ToyPart::SmallBoots, Paint::Cocoa),
        part(ToyPart::Boots, Paint::Cocoa),
        part(ToyPart::Boots, Paint::Cocoa),
        part(ToyPart::Treads, Paint::Lime),
        part(ToyPart::VentralFin, Paint::Sky),
        part(ToyPart::Wheels, Paint::Cherry),
    ];
    for fitted in unders {
        tui.app.casino.toybox.add(fitted);
    }
    tui.run("/toybox");
    tui.select("Pip");
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Down);
    tui.key(KeyCode::Down);
    let screen = tui.screen();
    screen.expect_find("Under parts");
    screen.expect_find("• -");
    screen.expect_find("Cocoa Small Boots");
    screen.expect_absent("Lime Treads");
    for _ in 0..4 {
        tui.key(KeyCode::Right);
    }
    let screen = tui.screen();
    let row = screen
        .text()
        .lines()
        .find(|line| line.contains("• Cocoa Boots"))
        .expect("the chosen row")
        .to_string();
    assert!(row.contains("×2"), "{row}");
    for _ in 0..3 {
        tui.key(KeyCode::Right);
    }
    let screen = tui.screen();
    screen.expect_find("• Lime Treads");
    screen.expect_find("Coral Feet");
    screen.expect_absent("Sky Ventral Fin");
}
