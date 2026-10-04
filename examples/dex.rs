use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fishtank::entities::cow::{Cow, CowVariant};
use fishtank::fishes::fish::{Direction, Fish, compute_display_width};
use fishtank::fishes::mutations::{Mutatable, Mutation, apply_mutation_to_fish};
use fishtank::fishes::species::{
    ALL_SPECIES, FishSpecies, Habit, Habitat, Locomotion, SizeCategory, Skin, SpeciesConfig, Zoomie,
};
use fishtank::fishes::unfish::{UnfishKind, is_multi_row};
use fishtank::settings::{DEFAULT_FPS, Settings};
use fishtank::tank::{Sky, Tank, TankBackground, TankKind};
use fishtank::testing::{Reel, Still};
use fishtank::ui::tank_view::TankView;
use fishtank::ui::{draw_fish_centred, fish_art_height, render_fish_sprite};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthChar;

const USAGE: &str = "usage: cargo run --example dex -- <out-dir>

Films the wiki's dex: every fish species a player can meet, every unfish, every cow and every
mutation, each as a reel of its own (<out>/reels/<slug>.html). A fish whose gift only shows in
motion or in company swims in a small pond; every other one is animated in place, the way /index
draws it. Writes <out>/dex.tsv (a header row, then one row per entry) and a palette swatch reel for
every fish (swatch-<slug>), and the Mutations page's engulfment scene (scene-engulfment).";
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
const POND: (u16, u16) = (36, 8);
const BOX_POND: (u16, u16) = (24, 10);
const COW_POND: (u16, u16) = (34, 8);
const POND_WARMUP_TICKS: usize = 30;
const POND_LOOKOUT_TICKS: usize = 30 * 600;
const POND_LEAD_TICKS: usize = 30;
const POND_TICKS: usize = 180;
const TELEPORT_CELLS: f32 = 3.0;
const SCENE: (u16, u16) = (64, 9);
const SCENE_ROW: f32 = 4.0;
const SCENE_LEFT_X: f32 = 6.0;
const SCENE_RIGHT_X: f32 = 46.0;
const SCENE_AFTER_TICKS: usize = 120;
const SCENE_LIMIT_TICKS: usize = 30 * 60;
const SUBJECT: FishSpecies = FishSpecies::Goldfish;
const PREY: FishSpecies = FishSpecies::Salmon;
const COMPANION: FishSpecies = FishSpecies::Goldfish;
const ENGULFER: FishSpecies = FishSpecies::Salmon;
const ENGULFED: FishSpecies = FishSpecies::Tang;
const SUBJECT_NAME: &str = "Darwin";
const PREY_NAME: &str = "Minnow";
const SIZE_ROLLS: usize = 200;
const SPOT_X: f32 = 12.0;
const SPOT_Y: f32 = 4.0;
const SWATCH: &str = "██";
const TSV: &str = "dex.tsv";
const FIELDS: &str = "kind\tslug\tname\trarity\tslow\tfast\tshort\tlong\tlight\theavy\tcheap\tdear\tprice\thome\tpattern\tmoves\tzoomie\thabit\tskin\tborn\tsprite";
const NOTHING: &str = "-";
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

    fn push(&mut self, buffer: &Buffer) {
        let label = format!("{} {:04}", self.slug, self.reel.stills().len());
        self.reel.push(Still::of(label, buffer));
    }

    fn save(&self, out: &Path) -> std::io::Result<PathBuf> {
        self.reel.save(out, &self.slug)
    }
}

struct Canvas {
    width: u16,
    above: u16,
    below: u16,
}

impl Canvas {
    fn fitting(frames: &[Vec<Fish>]) -> Self {
        let mut canvas = Canvas {
            width: 1,
            above: 0,
            below: 0,
        };
        for frame in frames {
            canvas.width = canvas.width.max(row_width(frame));
            for fish in frame {
                let (above, below) = rows_around_body(fish);
                canvas.above = canvas.above.max(above);
                canvas.below = canvas.below.max(below);
            }
        }
        canvas
    }

    fn area(&self) -> Rect {
        Rect::new(
            0,
            0,
            self.width + 2 * PAD_X,
            self.above + 1 + self.below + 2 * PAD_Y,
        )
    }

    fn body_y(&self) -> u16 {
        PAD_Y + self.above
    }

    fn draw(&self, frame: &[Fish]) -> Buffer {
        let area = self.area();
        let mut buffer = Buffer::empty(area);
        let mut x = area.width.saturating_sub(row_width(frame)) / 2;
        for fish in frame {
            let width = art_width(fish);
            if is_multi(fish) {
                let height = fish_art_height(fish, 1);
                let top = area.height.saturating_sub(height) / 2;
                let slot = Rect::new(x, top, width, height).intersection(area);
                draw_fish_centred(&mut buffer, fish, slot, Color::Reset);
            } else if !fish.is_invisible() {
                let sprite = fish.line_sprite();
                render_fish_sprite(&mut buffer, &sprite, x, self.body_y(), area, Color::Reset);
            }
            x += width + GAP;
        }
        buffer
    }
}

fn rows_around_body(fish: &Fish) -> (u16, u16) {
    if is_multi(fish) {
        let height = fish_art_height(fish, 1);
        return (height / 2, height - height / 2);
    }
    let sprite = fish.line_sprite();
    let above = sprite.body_row as u16;
    let below = sprite.rows.len().saturating_sub(sprite.body_row + 1) as u16;
    (above, below)
}

fn art_width(fish: &Fish) -> u16 {
    if is_multi(fish) {
        return fish.display_width as u16;
    }
    fish.line_sprite()
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

fn is_multi(fish: &Fish) -> bool {
    fish.unfish_state
        .as_ref()
        .is_some_and(|unfish| is_multi_row(unfish.kind))
}

fn row_width(fishes: &[Fish]) -> u16 {
    let arts: u16 = fishes.iter().map(art_width).sum();
    arts + GAP * fishes.len().saturating_sub(1) as u16
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
    let canvas = Canvas::fitting(&frames);
    let mut shot = Shot::new(slug);
    for frame in &frames {
        shot.push(&canvas.draw(frame));
    }
    shot
}

fn slug_of(text: &str) -> String {
    text.to_ascii_lowercase().replace(' ', "-")
}

fn sprite_text(fish: &Fish) -> String {
    let at_rest = posed(fish, Direction::Left);
    let frame = vec![at_rest];
    let buffer = Canvas::fitting(std::slice::from_ref(&frame)).draw(&frame);
    let still = Still::of("sprite", &buffer);
    (0..still.height())
        .map(|y| still.row_text(y).trim_end().to_string())
        .filter(|row| !row.trim().is_empty())
        .collect::<Vec<_>>()
        .join(ROW_BREAK)
}

fn swatch(slug: &str, palette: &[Color]) -> Shot {
    let width = palette.len() as u16 * (SWATCH.chars().count() as u16 + 1);
    let mut buffer = Buffer::empty(Rect::new(0, 0, width.max(1), 1));
    for (index, &color) in palette.iter().enumerate() {
        let x = index as u16 * (SWATCH.chars().count() as u16 + 1);
        buffer.set_string(x, 0, SWATCH, Style::default().fg(color));
    }
    let mut shot = Shot::new(format!("swatch-{slug}"));
    shot.push(&buffer);
    shot
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
            NOTHING.to_string(),
            cheap.to_string(),
            NOTHING.to_string(),
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
        Habitat::Everywhere | Habitat::Junkpile | Habitat::Nowhere => NOTHING.to_string(),
    }
}

fn debug_or_nothing<T: std::fmt::Debug>(value: Option<T>) -> String {
    value.map_or_else(|| NOTHING.to_string(), |value| format!("{value:?}"))
}

fn species_row(species: FishSpecies, fish: &Fish) -> String {
    let config = species.config();
    let (shortest, longest) = length_range(species, &config);
    let [light, heavy, cheap, dear] = weight_and_worth(species, &config);
    let price = if config.buyable {
        species.buy_price().to_string()
    } else {
        NOTHING.to_string()
    };
    let born: Vec<&str> = config.born_with.iter().map(|m| m.token()).collect();
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
        format!("{:?}", config.pattern),
        format!("{:?}", config.locomotion),
        format!("{:?}", config.zoomie),
        debug_or_nothing(config.habit),
        format!("{:?}", config.skin),
        if born.is_empty() {
            NOTHING.to_string()
        } else {
            born.join(",")
        },
        sprite_text(fish),
    ]
    .join("\t")
}

fn unfish_row(kind: UnfishKind, fish: &Fish) -> String {
    let (slow, fast) = kind.speed_range();
    let name = format!("{kind:?}");
    let mut row = vec![
        "unfish".to_string(),
        slug_of(&name),
        name,
        NOTHING.to_string(),
        format!("{slow}"),
        format!("{fast}"),
        fish.display_width.to_string(),
        fish.display_width.to_string(),
    ];
    row.extend((0..12).map(|_| NOTHING.to_string()));
    row.push(sprite_text(fish));
    row.join("\t")
}

fn bare_row(kind: &str, slug: &str, name: &str) -> String {
    let mut row = vec![kind.to_string(), slug.to_string(), name.to_string()];
    row.extend((0..18).map(|_| NOTHING.to_string()));
    row.join("\t")
}

#[derive(Clone, Copy, PartialEq)]
enum Cue {
    Calm,
    Zoomie,
    Teleport,
    Blink,
}

struct Pond {
    tank: Tank,
    sky: Sky,
    cue: Cue,
}

impl Pond {
    fn new(kind: TankKind, size: (u16, u16), night: bool) -> Self {
        let mut tank = Tank::new("Pond".to_string(), kind, &[]);
        tank.resize(size.0, size.1, &[]);
        if let TankBackground::Plain { plants } = &mut tank.background {
            plants.clear();
        }
        let sky = Sky {
            daylight: !night,
            ..Sky::default()
        };
        Self {
            tank,
            sky,
            cue: Cue::Calm,
        }
    }

    fn with_fish(mut self, fish: Fish) -> Self {
        let name = fish.name.clone();
        self.tank.place_fish(fish, name, &mut rand::rng());
        self
    }

    fn with_species(self, species: FishSpecies, name: &str) -> Self {
        let fish = Fish::new(species, name.to_string(), 0.0, 0.0, &mut rand::rng());
        self.with_fish(fish)
    }

    fn with_cow(mut self, variant: CowVariant) -> Self {
        let floor = self.tank.height as f32 - Cow::sprite_height() as f32;
        let cow = Cow::new(
            "Vaquita".to_string(),
            variant,
            self.tank.width as f32 / 4.0,
            floor.max(0.0),
            &mut rand::rng(),
        );
        self.tank.cows.push(cow);
        self
    }

    fn waiting_for(mut self, cue: Cue) -> Self {
        self.cue = cue;
        self
    }

    fn cued(&self, last_x: f32) -> bool {
        let Some(subject) = self.tank.fish.first() else {
            return true;
        };
        match self.cue {
            Cue::Calm => true,
            Cue::Zoomie => subject.is_zooming() || subject.is_puffed(),
            Cue::Teleport => (subject.position.x - last_x).abs() > TELEPORT_CELLS,
            Cue::Blink => subject.is_invisible(),
        }
    }

    fn frame(&self) -> Buffer {
        let area = Rect::new(0, 0, self.tank.width, self.tank.height);
        let mut buffer = Buffer::empty(area);
        TankView::new(&self.tank).render(area, &mut buffer);
        buffer
    }

    fn film(mut self, slug: &str) -> Shot {
        let settings = Settings::default();
        let mut shot = Shot::new(slug);
        for _ in 0..POND_WARMUP_TICKS {
            self.tank.tick(&settings, 0, self.sky);
        }
        let mut lead: Vec<Buffer> = Vec::new();
        let mut filming: Option<usize> = None;
        for tick in 0..POND_LOOKOUT_TICKS {
            let last_x = self.tank.fish.first().map_or(0.0, |fish| fish.position.x);
            self.tank.tick(&settings, 0, self.sky);
            let buffer = self.frame();
            if filming.is_none() && self.cued(last_x) {
                filming = Some(tick);
                for earlier in lead.drain(..) {
                    shot.push(&earlier);
                }
            }
            match filming {
                Some(start) if tick - start >= POND_TICKS => break,
                Some(_) if tick % FILM_EVERY == 0 => shot.push(&buffer),
                None if tick % FILM_EVERY == 0 => {
                    lead.push(buffer);
                    if lead.len() > POND_LEAD_TICKS / FILM_EVERY {
                        lead.remove(0);
                    }
                }
                _ => {}
            }
        }
        if filming.is_none() {
            for earlier in &lead {
                shot.push(earlier);
            }
        }
        shot
    }
}

fn shows_in_motion(config: &SpeciesConfig) -> bool {
    config.locomotion != Locomotion::Swim
        || !matches!(config.zoomie, Zoomie::Burst | Zoomie::None)
        || config.habit.is_some()
        || matches!(config.skin, Skin::Cycle(_) | Skin::Camouflage)
        || config.trail.is_some()
        || config.born_with.contains(&Mutation::Puff)
}

fn shines_at_night(config: &SpeciesConfig) -> bool {
    config.born_with.contains(&Mutation::Lure) || config.habit == Some(Habit::Twinkle)
}

fn company(species: FishSpecies, habit: Option<Habit>) -> Vec<FishSpecies> {
    match habit {
        Some(Habit::Sync) => vec![species, species],
        Some(Habit::Pair | Habit::Duel | Habit::ShellSwap) => vec![species],
        Some(Habit::Chase) => vec![COMPANION, COMPANION],
        Some(Habit::Shadow) => vec![COMPANION],
        _ => Vec::new(),
    }
}

fn pond_shot(species: FishSpecies, slug: &str) -> Shot {
    let config = species.config();
    let size = if config.locomotion == Locomotion::Bounce {
        BOX_POND
    } else {
        POND
    };
    let special_zoomie = !matches!(config.zoomie, Zoomie::Burst | Zoomie::None);
    let cue = if special_zoomie || config.born_with.contains(&Mutation::Puff) {
        Cue::Zoomie
    } else {
        Cue::Calm
    };
    let mut pond = Pond::new(TankKind::Base, size, shines_at_night(&config))
        .with_species(species, config.name)
        .waiting_for(cue);
    for (index, other) in company(species, config.habit).into_iter().enumerate() {
        pond = pond.with_species(other, &format!("{}{index}", other.display_name()));
    }
    pond.film(slug)
}

fn portrait_shot(species: FishSpecies, fish: &Fish, slug: &str) -> Shot {
    let auto = species.config().auto_mutate;
    let ticks = if auto {
        MUTANT_STEP_TICKS * MUTANT_STEPS
    } else {
        PORTRAIT_TICKS
    };
    let mut fish = fish.clone();
    fish.sky.daylight = !shines_at_night(&species.config());
    shoot(slug, vec![(vec![fish], ticks)], |fishes, _, tick| {
        if !auto || tick == 0 || tick % MUTANT_STEP_TICKS != 0 {
            return;
        }
        let mut rng = rand::rng();
        let subject = &mut fishes[0];
        if let Some(mutation) = subject.random_mutation_with_room(&mut rng, false) {
            apply_mutation_to_fish(subject, mutation, &mut rng);
            subject.refresh_width();
        }
    })
}

fn species_shot(species: FishSpecies, rng: &mut impl rand::RngExt) -> (Shot, Fish) {
    let fish = posed(&Fish::new_for_display(species, rng), Direction::Left);
    let slug = format!("fish-{}", slug_of(species.display_name()));
    let config = species.config();
    let shot = if shows_in_motion(&config) {
        pond_shot(species, &slug)
    } else {
        portrait_shot(species, &fish, &slug)
    };
    (shot, fish)
}

fn unfish_shot(kind: UnfishKind, rng: &mut impl rand::RngExt) -> (Shot, Fish) {
    let fish = Fish::new_unfish(kind, String::new(), 0.0, 0.0, rng);
    let slug = format!("unfish-{}", slug_of(&format!("{kind:?}")));
    let cue = match kind {
        UnfishKind::Phantom => Some(Cue::Teleport),
        UnfishKind::Blinker => Some(Cue::Blink),
        _ => None,
    };
    let shot = match cue {
        Some(cue) => Pond::new(TankKind::Base, POND, false)
            .with_fish(Fish::new_unfish(kind, format!("{kind:?}"), 0.0, 0.0, rng))
            .waiting_for(cue)
            .film(&slug),
        None => shoot(
            &slug,
            vec![(vec![posed(&fish, Direction::Left)], PORTRAIT_TICKS)],
            |_, _, _| {},
        ),
    };
    (shot, posed(&fish, Direction::Left))
}

fn cow_shot(variant: CowVariant, kind: TankKind, slug: &str) -> Shot {
    Pond::new(kind, COW_POND, false)
        .with_cow(variant)
        .film(slug)
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
        Mutation::Heterochromia | Mutation::Cyclops => (&[], &["eyeincrease"]),
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

fn wake_shot(mutation: Mutation) -> Shot {
    let mut pond = Pond::new(TankKind::Base, POND, false)
        .with_species(SUBJECT, SUBJECT_NAME)
        .waiting_for(Cue::Zoomie);
    let name = pond.tank.fish[0].name.clone();
    assert!(pond.tank.apply_named_mutation(&name, mutation.token()));
    pond.film(&format!("mutation-{}", mutation.token()))
}

fn shows_only_in_motion(mutation: Mutation) -> bool {
    mutation == Mutation::WakeColor
}

fn steer_towards(fish: &mut Fish, facing: Direction) {
    if fish.facing != facing {
        fish.flip();
    }
    let sign = match facing {
        Direction::Left => -1.0,
        Direction::Right => 1.0,
    };
    fish.velocity.dx = fish.speed * sign;
    fish.velocity.dy = 0.0;
    fish.position.y = SCENE_ROW;
}

fn engulfment_scene() -> Shot {
    let mut pond = Pond::new(TankKind::Base, SCENE, false)
        .with_species(ENGULFER, "Jonah")
        .with_species(ENGULFED, "Dory");
    pond.tank.fish[0].position.x = SCENE_LEFT_X;
    pond.tank.fish[1].position.x = SCENE_RIGHT_X;
    assert!(pond.tank.apply_named_mutation("Jonah", "engulfment"));
    let settings = Settings::default();
    let mut shot = Shot::new("scene-engulfment");
    let mut fused_at = None;
    for tick in 0..SCENE_LIMIT_TICKS {
        if fused_at.is_none() {
            steer_towards(&mut pond.tank.fish[0], Direction::Right);
            if let Some(dory) = pond.tank.fish.get_mut(1) {
                steer_towards(dory, Direction::Left);
            }
        }
        pond.tank.tick(&settings, 0, pond.sky);
        if fused_at.is_none() && pond.tank.fish.len() == 1 {
            fused_at = Some(tick);
        }
        if tick % FILM_EVERY == 0 {
            shot.push(&pond.frame());
        }
        if fused_at.is_some_and(|at| tick - at >= SCENE_AFTER_TICKS) {
            break;
        }
    }
    shot
}

fn main() -> ExitCode {
    let Some(out) = env::args().nth(1).map(PathBuf::from) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let mut rng = rand::rng();
    let mut tsv = format!("{FIELDS}\n");
    let mut shots = Vec::new();
    for &species in ALL_SPECIES {
        if HIDDEN.contains(&species) || !species.is_obtainable() {
            continue;
        }
        let (shot, fish) = species_shot(species, &mut rng);
        let _ = writeln!(tsv, "{}", species_row(species, &fish));
        shots.push(shot);
        shots.push(swatch(
            &slug_of(species.display_name()),
            species.config().palette,
        ));
    }
    for kind in UNFISH {
        let (shot, fish) = unfish_shot(kind, &mut rng);
        let _ = writeln!(tsv, "{}", unfish_row(kind, &fish));
        shots.push(shot);
    }
    for &variant in CowVariant::ALL {
        let name = format!("{variant:?}");
        let slug = format!("cow-{}", slug_of(&name));
        shots.push(cow_shot(variant, TankKind::Base, &slug));
        let milk = format!("{:?}", variant.milk());
        let _ = writeln!(tsv, "{}", bare_row("cow", &slug, &format!("{name}:{milk}")));
    }
    shots.push(cow_shot(CowVariant::Brown, TankKind::Rad, "cow-irradiated"));
    let _ = writeln!(
        tsv,
        "{}",
        bare_row("cow", "cow-irradiated", "Irradiated:Irradiated")
    );
    for &mutation in Mutation::ALL {
        let shot = if shows_only_in_motion(mutation) {
            wake_shot(mutation)
        } else {
            mutation_shot(mutation)
        };
        let _ = writeln!(
            tsv,
            "{}",
            bare_row("mutation", mutation.token(), mutation.token())
        );
        shots.push(shot);
    }
    shots.push(engulfment_scene());
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
