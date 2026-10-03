use std::collections::HashMap;

use fishtank::{
    economy::Rarity,
    fishes::{
        fish::{Direction, Fish},
        species::{ALL_SPECIES, FishSpecies, Habitat},
    },
    loot::LootKind,
    testing::{Reel, Still},
    ui::{
        catch_overlay::{CatchOverlay, CatchState},
        layout::Screen,
    },
};
use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

const NEW_COMMONS: [FishSpecies; 28] = [
    FishSpecies::Jurel,
    FishSpecies::Congrio,
    FishSpecies::Pejerrey,
    FishSpecies::Puye,
    FishSpecies::Pintacha,
    FishSpecies::Borrachilla,
    FishSpecies::Cojinoba,
    FishSpecies::Trucha,
    FishSpecies::Corvina,
    FishSpecies::Perca,
    FishSpecies::Bacalao,
    FishSpecies::Guppy,
    FishSpecies::Gramma,
    FishSpecies::Rasbora,
    FishSpecies::Gourami,
    FishSpecies::Zebrafish,
    FishSpecies::Bumblebeefish,
    FishSpecies::Mandarinfish,
    FishSpecies::Damselfish,
    FishSpecies::Platy,
    FishSpecies::Molly,
    FishSpecies::Tilapia,
    FishSpecies::Sunsetfish,
    FishSpecies::Leaffish,
    FishSpecies::Galaxyfish,
    FishSpecies::Opalfish,
    FishSpecies::Tartanfish,
    FishSpecies::Bitfish,
];

const REEL_DIR: &str = env!("CARGO_TARGET_TMPDIR");
const CARD_SIZES: [(u16, u16); 4] = [(100, 26), (60, 18), (40, 14), (28, 10)];
const SAMPLES: usize = 40;

fn colors_from_the_head(species: FishSpecies) -> Vec<Color> {
    let mut fish = Fish::new(species, "Probe".into(), 10.0, 10.0, &mut rand::rng());
    fish.facing = Direction::Left;
    fish.sway.phase = 0.0;
    let sprite = fish.line_sprite();
    sprite.rows[sprite.body_row]
        .iter()
        .map(|&(_, color)| color)
        .collect()
}

fn palette(species: FishSpecies) -> &'static [Color] {
    species.config().palette
}

#[test]
fn every_new_common_is_sold_in_the_shop_and_caught_everywhere() {
    for species in NEW_COMMONS {
        let config = species.config();
        assert_eq!(config.rarity, Rarity::Common, "{}", config.name);
        assert!(config.buyable, "{} is sold in the shop", config.name);
        assert_eq!(config.habitat, Habitat::Everywhere, "{}", config.name);
        assert_eq!(
            FishSpecies::parse(config.name),
            Some(species),
            "{} is one word the commands can parse",
            config.name
        );
    }
}

#[test]
fn no_two_species_share_a_body_a_palette_and_a_pattern() {
    let mut seen: HashMap<String, &str> = HashMap::new();
    for &species in ALL_SPECIES {
        let config = species.config();
        let look = format!(
            "{:?} {:?} {:?} {:?}",
            config.body, config.palette, config.pattern, config.born_with
        );
        if let Some(twin) = seen.insert(look, config.name) {
            panic!("{} looks exactly like {twin}", config.name);
        }
    }
}

#[test]
fn a_banded_fish_wears_its_colours_two_cells_at_a_time() {
    let colors = colors_from_the_head(FishSpecies::Pintacha);
    let bands = palette(FishSpecies::Pintacha);
    for (i, &color) in colors.iter().enumerate() {
        assert_eq!(color, bands[(i / 2) % bands.len()], "cell {i}");
    }
}

#[test]
fn a_fish_in_halves_is_one_colour_in_front_and_another_behind() {
    let colors = colors_from_the_head(FishSpecies::Gramma);
    let halves = palette(FishSpecies::Gramma);
    let front = colors.len().div_ceil(2);
    assert!(colors[..front].iter().all(|&c| c == halves[0]));
    assert!(colors[front..].iter().all(|&c| c == halves[1]));
}

#[test]
fn a_gradient_runs_from_the_first_colour_at_the_mouth_to_the_last_at_the_tail() {
    let colors = colors_from_the_head(FishSpecies::Sunsetfish);
    let stops = palette(FishSpecies::Sunsetfish);
    assert_eq!(colors.first(), stops.first());
    assert_eq!(colors.last(), stops.last());
    let order: Vec<usize> = colors
        .iter()
        .map(|c| stops.iter().position(|s| s == c).unwrap())
        .collect();
    assert!(order.windows(2).all(|w| w[0] <= w[1]), "{order:?}");
}

#[test]
fn a_speckled_fish_is_its_first_colour_flecked_with_the_rest() {
    for _ in 0..SAMPLES {
        let colors = colors_from_the_head(FishSpecies::Trucha);
        let flecks = palette(FishSpecies::Trucha);
        assert!(colors.iter().all(|c| flecks.contains(c)));
        let base = colors.iter().filter(|&&c| c == flecks[0]).count();
        assert!(base > 0, "{colors:?}");
    }
}

#[test]
fn a_zoned_fish_has_its_own_mouth_body_and_tail_colours() {
    let colors = colors_from_the_head(FishSpecies::Damselfish);
    let zones = palette(FishSpecies::Damselfish);
    let tail = 2;
    assert_eq!(colors[0], zones[0]);
    assert!(
        colors[1..colors.len() - tail]
            .iter()
            .all(|&c| c == zones[1])
    );
    assert!(colors[colors.len() - tail..].iter().all(|&c| c == zones[2]));
}

#[test]
fn a_guppy_tail_is_one_of_its_bright_colours() {
    let zones = palette(FishSpecies::Guppy);
    for _ in 0..SAMPLES {
        let colors = colors_from_the_head(FishSpecies::Guppy);
        let tail = *colors.last().unwrap();
        assert!(zones[2..].contains(&tail), "{tail:?}");
        assert!(colors[..colors.len() - 3].iter().all(|&c| c == zones[1]));
    }
}

#[test]
fn every_new_common_draws_whole_on_its_catch_card_at_every_size() {
    let mut reel = Reel::new();
    for species in NEW_COMMONS {
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
    reel.save(std::path::Path::new(REEL_DIR), "common-catch-cards")
        .expect("the reel is writable");
    let report = reel.flaw_report();
    assert!(report.is_empty(), "{report}");
}
