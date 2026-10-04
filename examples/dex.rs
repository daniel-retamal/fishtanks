use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fishtank::fishes::fish::{Direction, Fish, LineSprite, compute_display_width};
use fishtank::fishes::mutations::{Mutatable, Mutation, apply_mutation_to_fish};
use fishtank::fishes::species::{ALL_SPECIES, FishSpecies, Habitat, SizeCategory, SpeciesConfig};
use fishtank::fishes::unfish::{UnfishKind, is_multi_row};
use fishtank::settings::{DEFAULT_FPS, Settings};
use fishtank::tank::{Sky, Tank, TankKind};
use fishtank::testing::{Reel, Still};
use fishtank::ui::tank_view::TankView;
use fishtank::ui::{draw_fish_centred, fish_art_height, render_fish_sprite};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthChar;

const USAGE: &str = "usage: cargo run --example dex -- <out-dir>

Films the wiki's dex: every fish species a player can meet, every unfish and every mutation,
each as a reel of its own (<out>/reels/<slug>.html), animated in place the way /index shows a
fish, and writes <out>/dex.tsv with the numbers and the sprite of each entry.";
const PAD_X: u16 = 2;
const PAD_Y: u16 = 1;
const GAP: u16 = 3;
const FILM_EVERY: usize = 2;
const PORTRAIT_TICKS: usize = 90;
const BEFORE_TICKS: usize = 40;
const AFTER_TICKS: usize = 70;
const MUTANT_STEP_TICKS: usize = 30;
const MUTANT_STEPS: usize = 8;
const PUFF_TICKS: usize = 20;
const PUFF_SECS: f32 = 1.0;
const POND_COLS: u16 = 44;
const POND_ROWS: u16 = 8;
const POND_LOOKOUT_TICKS: usize = 30 * 600;
const POND_LEAD_TICKS: usize = 20;
const POND_TICKS: usize = 110;
const SUBJECT: FishSpecies = FishSpecies::Goldfish;
const PREY: FishSpecies = FishSpecies::Salmon;
const SUBJECT_NAME: &str = "Darwin";
const PREY_NAME: &str = "Minnow";
const SIZE_ROLLS: usize = 200;
const SPOT_X: f32 = 12.0;
const SPOT_Y: f32 = 4.0;
const TSV: &str = "dex.tsv";
const NO_LIMIT: &str = "-";
const ROW_BREAK: &str = "\\n";
const HIDDEN: [FishSpecies; 2] = [FishSpecies::Cheatfish, FishSpecies::Junkfish];
const UNFISH: [UnfishKind; 7] = [
    UnfishKind::Reversed,
    UnfishKind::Doppleganger,
    UnfishKind::Phantom,
    UnfishKind::Blinker,
    UnfishKind::Ball,
    UnfishKind::Skull,
    UnfishKind::Worm,
];

struct Shot {
    slug: String,
    reel: Reel,
}

impl Shot {
    fn new(slug: impl Into<String>) -> Self {
        Self {
            slug: slug.into(),
            reel: Reel::new(),
        }
    }

    fn film(&mut self, fishes: &[Fish], canvas: (u16, u16)) {
        let mut buffer = Buffer::empty(Rect::new(0, 0, canvas.0, canvas.1));
        draw_row(&mut buffer, fishes);
        let label = format!("{} {:04}", self.slug, self.reel.stills().len());
        self.reel.push(Still::of(label, &buffer));
    }

    fn save(&self, out: &Path) -> std::io::Result<PathBuf> {
        self.reel.save(out, &self.slug)
    }
}

fn sprite_width(sprite: &LineSprite) -> u16 {
    sprite
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|&(ch, _)| UnicodeWidthChar::width(ch).unwrap_or(1) as u16)
                .sum::<u16>()
        })
        .max()
        .unwrap_or(0)
}

fn art_width(fish: &Fish) -> u16 {
    if is_multi(fish) {
        return fish.display_width as u16;
    }
    sprite_width(&fish.line_sprite())
}

fn art_height(fish: &Fish) -> u16 {
    fish_art_height(fish, 1)
}

fn is_multi(fish: &Fish) -> bool {
    fish.unfish_state
        .as_ref()
        .is_some_and(|unfish| is_multi_row(unfish.kind))
}

fn row_width(fishes: &[Fish]) -> u16 {
    let arts: u16 = fishes.iter().map(art_width).sum();
    arts + GAP * fishes.len().saturating_sub(1) as u16
}

fn row_height(fishes: &[Fish]) -> u16 {
    fishes.iter().map(art_height).max().unwrap_or(1)
}

fn draw_row(buffer: &mut Buffer, fishes: &[Fish]) {
    let area = buffer.area;
    let mut x = area.width.saturating_sub(row_width(fishes)) / 2;
    for fish in fishes {
        let width = art_width(fish);
        let height = art_height(fish);
        let top = area.height.saturating_sub(height) / 2;
        let slot = Rect::new(x, top, width, height).intersection(area);
        if is_multi(fish) {
            draw_fish_centred(buffer, fish, slot, Color::Reset);
        } else {
            let sprite = fish.line_sprite();
            let rows = sprite.rows.len() as u16;
            let body_y = area.height.saturating_sub(rows) / 2 + sprite.body_row as u16;
            render_fish_sprite(buffer, &sprite, x, body_y, area, Color::Reset);
        }
        x += width + GAP;
    }
}

fn canvas_for(frames: &[Vec<Fish>]) -> (u16, u16) {
    let width = frames.iter().map(|f| row_width(f)).max().unwrap_or(1);
    let height = frames.iter().map(|f| row_height(f)).max().unwrap_or(1);
    (width + 2 * PAD_X, height + 2 * PAD_Y)
}

fn posed(fish: &Fish, facing: Direction) -> Fish {
    let mut fish = fish.portrait();
    if fish.facing != facing {
        fish.flip();
    }
    fish
}

fn animate(fishes: &mut [Fish]) {
    let dt = 1.0 / DEFAULT_FPS;
    for fish in fishes {
        fish.tick_animation(dt);
    }
}

fn shoot(
    slug: &str,
    scenes: Vec<(Vec<Fish>, usize)>,
    step: impl Fn(&mut [Fish], usize, usize),
) -> Shot {
    let mut frames: Vec<Vec<Fish>> = Vec::new();
    for (scene, (mut fishes, ticks)) in scenes.into_iter().enumerate() {
        for tick in 0..ticks {
            step(&mut fishes, scene, tick);
            animate(&mut fishes);
            if tick % FILM_EVERY == 0 {
                frames.push(fishes.clone());
            }
        }
    }
    let canvas = canvas_for(&frames);
    let mut shot = Shot::new(slug);
    for frame in &frames {
        shot.film(frame, canvas);
    }
    shot
}

fn slug_of(text: &str) -> String {
    text.to_ascii_lowercase().replace(' ', "-")
}

fn sprite_text(fish: &Fish) -> String {
    let mut buffer = Buffer::empty(Rect::new(0, 0, art_width(fish), art_height(fish)));
    let at_rest = posed(fish, Direction::Left);
    draw_row(&mut buffer, std::slice::from_ref(&at_rest));
    let still = Still::of("sprite", &buffer);
    (0..still.height())
        .map(|y| still.row_text(y).trim_end().to_string())
        .filter(|row| !row.trim().is_empty())
        .collect::<Vec<_>>()
        .join(ROW_BREAK)
}

fn length_range(species: FishSpecies, config: &SpeciesConfig) -> (usize, usize) {
    let widths: Vec<usize> = species
        .born_sizes()
        .iter()
        .map(|&size| compute_display_width(species, config.sizes[size as usize]))
        .collect();
    let shortest = widths.iter().copied().min().unwrap_or(0);
    let longest = widths.iter().copied().max().unwrap_or(0);
    (shortest, longest)
}

fn weight_and_worth(species: FishSpecies, config: &SpeciesConfig) -> [String; 4] {
    let born = species.born_sizes();
    let (smallest, largest) = (born[0], born[born.len() - 1]);
    let light = config.weight_base[smallest as usize];
    let heavy = config.weight_cap[largest as usize];
    let cheap = species.sell_value(light, smallest);
    if heavy == 0 {
        return [
            light.to_string(),
            NO_LIMIT.to_string(),
            cheap.to_string(),
            NO_LIMIT.to_string(),
        ];
    }
    let dear = species.sell_value(heavy, largest);
    [
        light.to_string(),
        heavy.to_string(),
        cheap.to_string(),
        dear.to_string(),
    ]
}

fn habitat(config: &SpeciesConfig) -> String {
    match config.habitat {
        Habitat::Native(kind) => kind.display_name().to_string(),
        Habitat::Everywhere => "everywhere".to_string(),
        Habitat::Junkpile | Habitat::Nowhere => "-".to_string(),
    }
}

fn species_row(species: FishSpecies, fish: &Fish) -> String {
    let config = species.config();
    let (shortest, longest) = length_range(species, &config);
    let [light, heavy, cheap, dear] = weight_and_worth(species, &config);
    let price = if config.buyable {
        species.buy_price().to_string()
    } else {
        "-".to_string()
    };
    [
        "fish".to_string(),
        slug_of(config.name),
        config.name.to_string(),
        format!("{:?}", config.rarity),
        format!("{}", config.speed_range.0),
        format!("{}", config.speed_range.1),
        shortest.to_string(),
        longest.to_string(),
        light,
        heavy,
        cheap,
        dear,
        price,
        habitat(&config),
        sprite_text(fish),
    ]
    .join("\t")
}

fn unfish_row(kind: UnfishKind, fish: &Fish) -> String {
    let (slow, fast) = kind.speed_range();
    let name = format!("{kind:?}");
    [
        "unfish".to_string(),
        slug_of(&name),
        name,
        "-".to_string(),
        format!("{slow}"),
        format!("{fast}"),
        fish.display_width.to_string(),
        fish.display_width.to_string(),
        fish.weight_g.to_string(),
        fish.weight_g.to_string(),
        "0".to_string(),
        "0".to_string(),
        "-".to_string(),
        TankKind::Void.display_name().to_string(),
        sprite_text(fish),
    ]
    .join("\t")
}

fn mutation_row(mutation: Mutation) -> String {
    [
        "mutation".to_string(),
        mutation.token().to_string(),
        mutation.token().to_string(),
    ]
    .join("\t")
}

fn species_shot(species: FishSpecies, rng: &mut impl rand::RngExt) -> (Shot, Fish) {
    let fish = posed(&Fish::new_for_display(species, rng), Direction::Left);
    let slug = format!("fish-{}", slug_of(species.display_name()));
    let auto = species.config().auto_mutate;
    let ticks = if auto {
        MUTANT_STEP_TICKS * MUTANT_STEPS
    } else {
        PORTRAIT_TICKS
    };
    let shot = shoot(
        &slug,
        vec![(vec![fish.clone()], ticks)],
        |fishes, _, tick| {
            if !auto || tick == 0 || tick % MUTANT_STEP_TICKS != 0 {
                return;
            }
            let mut rng = rand::rng();
            let subject = &mut fishes[0];
            if let Some(mutation) = subject.random_mutation_with_room(&mut rng, false) {
                apply_mutation_to_fish(subject, mutation, &mut rng);
                subject.refresh_width();
            }
        },
    );
    (shot, fish)
}

fn unfish_shot(kind: UnfishKind, rng: &mut impl rand::RngExt) -> (Shot, Fish) {
    let fish = Fish::new_unfish(kind, String::new(), 0.0, 0.0, rng);
    let fish = posed(&fish, Direction::Left);
    let slug = format!("unfish-{}", slug_of(&format!("{kind:?}")));
    let shot = shoot(
        &slug,
        vec![(vec![fish.clone()], PORTRAIT_TICKS)],
        |_, _, _| {},
    );
    (shot, fish)
}

struct Stage {
    tank: Tank,
}

impl Stage {
    fn new() -> Self {
        Self {
            tank: Tank::new("Dex".to_string(), TankKind::Base, &[]),
        }
    }

    fn with(mut self, species: FishSpecies, name: &str) -> Self {
        let mut rng = rand::rng();
        let mut fish = Fish::new(species, name.to_string(), SPOT_X, SPOT_Y, &mut rng);
        for _ in 0..SIZE_ROLLS {
            if fish.size_category == SizeCategory::L {
                break;
            }
            fish = Fish::new(species, name.to_string(), SPOT_X, SPOT_Y, &mut rng);
        }
        fish.frozen = true;
        self.tank.place_fish(fish, name.to_string(), &mut rng);
        let placed = self.tank.fish.last_mut().expect("placed");
        placed.position.x = SPOT_X;
        placed.position.y = SPOT_Y;
        self
    }

    fn mutate(&mut self, token: &str) {
        let subject = self.tank.fish[0].name.clone();
        assert!(
            self.tank.apply_named_mutation(&subject, token),
            "{subject} takes {token}"
        );
        self.tank.tick(&Settings::default(), 0, Sky::default());
    }

    fn after(mut self, tokens: &[&str]) -> Self {
        for token in tokens {
            self.mutate(token);
        }
        self
    }

    fn pose(&self, meeting: bool) -> Vec<Fish> {
        let count = self.tank.fish.len();
        self.tank
            .fish
            .iter()
            .enumerate()
            .map(|(index, fish)| {
                let facing = match (count, meeting, index) {
                    (1, _, _) => Direction::Left,
                    (_, true, 0) | (_, false, 1) => Direction::Right,
                    _ => Direction::Left,
                };
                let mut portrait = posed(fish, facing);
                portrait.sky.daylight = true;
                portrait
            })
            .collect()
    }
}

fn prerequisites(mutation: Mutation) -> (&'static [FishSpecies], &'static [&'static str]) {
    match mutation {
        Mutation::GlistenFast
        | Mutation::GlistenSlow
        | Mutation::GlistenMode
        | Mutation::GlistenColor => (&[], &["glistenenable"]),
        Mutation::EarColor => (&[], &["ear"]),
        Mutation::FeetColor => (&[], &["feet"]),
        Mutation::Heterochromia => (&[], &["eyeincrease"]),
        Mutation::Cytokinesis => (&[], &["telophase"]),
        Mutation::Engulfment => (&[PREY], &[]),
        Mutation::Endocytosis => (&[PREY], &["engulfment"]),
        Mutation::Revert => (&[], &["wings"]),
        _ => (&[], &[]),
    }
}

fn subject_for(mutation: Mutation) -> FishSpecies {
    match mutation {
        Mutation::BodyVariant => FishSpecies::Mutantfish,
        _ => SUBJECT,
    }
}

fn mutation_shot(mutation: Mutation) -> Shot {
    let (company, tokens) = prerequisites(mutation);
    let mut stage = Stage::new().with(subject_for(mutation), SUBJECT_NAME);
    for &species in company {
        stage = stage.with(species, PREY_NAME);
    }
    let stage = stage.after(tokens);
    let meeting = mutation == Mutation::Engulfment;
    let before = stage.pose(meeting);
    let mut stage = stage;
    stage.mutate(mutation.token());
    let after = stage.pose(false);
    let slug = format!("mutation-{}", mutation.token());
    let puffs = mutation == Mutation::Puff;
    shoot(
        &slug,
        vec![(before, BEFORE_TICKS), (after, AFTER_TICKS)],
        |fishes, scene, tick| {
            if !puffs || scene == 0 {
                return;
            }
            let puffed = (tick / PUFF_TICKS) % 2 == 1;
            for fish in fishes.iter_mut() {
                fish.habits.puffed = if puffed { PUFF_SECS } else { 0.0 };
            }
        },
    )
}

fn pond_shot(mutation: Mutation) -> Shot {
    let mut tank = Tank::new("Pond".to_string(), TankKind::Base, &[]);
    tank.resize(POND_COLS, POND_ROWS, &[]);
    let mut rng = rand::rng();
    tank.spawn_fish(SUBJECT, SUBJECT_NAME.to_string(), &mut rng);
    assert!(tank.apply_named_mutation(SUBJECT_NAME, mutation.token()));
    let settings = Settings::default();
    let mut lead: Vec<Buffer> = Vec::new();
    let mut shot = Shot::new(format!("mutation-{}", mutation.token()));
    let area = Rect::new(0, 0, POND_COLS, POND_ROWS);
    let mut filming = None;
    for tick in 0..POND_LOOKOUT_TICKS {
        tank.tick(&settings, 0, Sky::default());
        let mut buffer = Buffer::empty(area);
        TankView::new(&tank).render(area, &mut buffer);
        let zooming = tank.fish.first().is_some_and(Fish::is_zooming);
        if filming.is_none() && zooming {
            filming = Some(tick);
            for earlier in lead.drain(..) {
                shot.reel.push(Still::of("lead", &earlier));
            }
        }
        match filming {
            Some(start) if tick - start >= POND_TICKS => break,
            Some(_) if tick % FILM_EVERY == 0 => shot.reel.push(Still::of("pond", &buffer)),
            None if tick % FILM_EVERY == 0 => {
                lead.push(buffer);
                if lead.len() > POND_LEAD_TICKS / FILM_EVERY {
                    lead.remove(0);
                }
            }
            _ => {}
        }
    }
    shot
}

fn shows_only_in_motion(mutation: Mutation) -> bool {
    mutation == Mutation::WakeColor
}

fn main() -> ExitCode {
    let Some(out) = env::args().nth(1).map(PathBuf::from) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let mut rng = rand::rng();
    let mut tsv = String::new();
    let mut shots = Vec::new();
    for &species in ALL_SPECIES {
        if HIDDEN.contains(&species) || !species.is_obtainable() {
            continue;
        }
        let (shot, fish) = species_shot(species, &mut rng);
        let _ = writeln!(tsv, "{}", species_row(species, &fish));
        shots.push(shot);
    }
    for kind in UNFISH {
        let (shot, fish) = unfish_shot(kind, &mut rng);
        let _ = writeln!(tsv, "{}", unfish_row(kind, &fish));
        shots.push(shot);
    }
    for &mutation in Mutation::ALL {
        let shot = if shows_only_in_motion(mutation) {
            pond_shot(mutation)
        } else {
            mutation_shot(mutation)
        };
        let _ = writeln!(tsv, "{}", mutation_row(mutation));
        shots.push(shot);
    }
    for shot in &shots {
        if let Err(error) = shot.save(&out) {
            eprintln!("cannot write {}: {error}", shot.slug);
            return ExitCode::FAILURE;
        }
    }
    if let Err(error) = fs::create_dir_all(&out).and_then(|()| fs::write(out.join(TSV), tsv)) {
        eprintln!("cannot write {TSV}: {error}");
        return ExitCode::FAILURE;
    }
    println!("{} reel(s) under {}", shots.len(), out.display());
    ExitCode::SUCCESS
}
