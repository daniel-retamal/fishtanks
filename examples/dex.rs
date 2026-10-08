use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fishtank::entities::cow::{Cow, CowVariant};
use fishtank::entities::plant::Plant;
use fishtank::fishes::fish::{Direction, Fish, FishState, compute_display_width};
use fishtank::fishes::mutations::{Mutatable, Mutation, apply_mutation, apply_mutation_to_fish};
use fishtank::fishes::quirk::{
    ANAGRAM_SHUFFLE_MEAN_SECS, FAULT_SLIP_MEAN_SECS, FAULT_SLIP_SECS, GRAEAE_PASS_REST_SECS,
    MOLT_MEAN_SECS, Mood, Quirk, REFLECTION_DISAGREE_MEAN_SECS,
};
use fishtank::fishes::species::{
    ALL_SPECIES, FishSpecies, Habit, Habitat, Locomotion, SizeCategory, SpeciesConfig, Zoomie,
};
use fishtank::fishes::toy::{FittedPart, Line, Material, Paint, ToyColor, ToyPart, ToyState};
use fishtank::fishes::unfish::{SPAWNABLE_UNFISH, UnfishKind, is_multi_row};
use fishtank::settings::{DEFAULT_FPS, Settings};
use fishtank::sprite::TRANSPARENT;
use fishtank::tank::{
    PLANT_HEIGHT_MAX, PLANT_HEIGHT_MIN, SHED_SINK_PER_SEC, Sky, Tank, TankBackground, TankKind,
};
use fishtank::testing::{Reel, Still};
use fishtank::ui::tank_view::{TankView, render_cow};
use fishtank::ui::{draw_fish_centred, fish_art_height, render_fish_sprite};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthChar;

const USAGE: &str = "usage: cargo run --example dex -- <out-dir>

Films the wiki's dex: every fish species a player can meet, every unfish, every cow and every
mutation, each as a reel of its own (<out>/reels/<slug>.html). Every fish is animated in place,
the way /index draws it, showing its gift (a puff, wings, a flash); only a bouncer, and the
Phantom's teleport, move about a bubble-free box. A cow stands on the bottom edge of its frame;
the irradiated one stands in a Radioactivetank, growing a hydra head and mutating. Writes
<out>/dex.tsv (a header row, then one row per entry) and the Mutations page's engulfment scene
(scene-engulfment).";
const PAD_X: u16 = 2;
const PAD_Y: u16 = 1;
const GAP: u16 = 3;
const FILM_EVERY: usize = 2;
const PORTRAIT_TICKS: usize = 120;
const GIFT_TICKS: usize = 30;
const BLINK_TICKS: usize = 30 * 10;
const FLASH_SECS: f32 = 1.0;
const GLIDE_SECS: f32 = 1.0;
const BEFORE_TICKS: usize = 40;
const AFTER_TICKS: usize = 70;
const MUTANT_STEP_TICKS: usize = 30;
const MUTANT_STEPS: usize = 8;
const PUFF_TICKS: usize = 20;
const PUFF_SECS: f32 = 1.0;
const POND: (u16, u16) = (36, 8);
const BOX_POND: (u16, u16) = (24, 10);
const COW_SIDE_PAD: u16 = 3;
const COW_TOP_PAD: u16 = 2;
const COW_MUTATIONS: usize = 16;
const SHOWCASE: [Mutation; 3] = [Mutation::Hydra, Mutation::ColorPatch, Mutation::EyeColor];
const STAGE: (u16, u16) = (96, 40);
const COW_SPOT: (f32, f32) = (24.0, 20.0);
const COW_NAME: &str = "Vaquita";
const MUTATION_TRIES: usize = 50;
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
const ENGULFER: FishSpecies = FishSpecies::Salmon;
const ENGULFED: FishSpecies = FishSpecies::Tang;
const SUBJECT_NAME: &str = "Darwin";
const PREY_NAME: &str = "Minnow";
const SIZE_ROLLS: usize = 200;
const SPOT_X: f32 = 12.0;
const SPOT_Y: f32 = 4.0;
const TSV: &str = "dex.tsv";
const FIELDS: &str = "kind\tslug\tname\trarity\tslow\tfast\tshort\tlong\tlight\theavy\tcheap\tdear\tprice\thome\tpattern\tmoves\tzoomie\thabit\tskin\tborn\tpalette\tsprite";
const NOTHING: &str = "-";
const ROW_BREAK: &str = "\\n";
const ON_THE_FLOOR: Margin = Margin {
    top: 2 * PAD_Y,
    bottom: 0,
};
const TRIMMED: [(FishSpecies, Margin); 5] = [
    (
        FishSpecies::Pejesapo,
        Margin {
            top: PAD_Y,
            bottom: 0,
        },
    ),
    (
        FishSpecies::Lanternfish,
        Margin {
            top: 0,
            bottom: PAD_Y,
        },
    ),
    (FishSpecies::Stonefish, ON_THE_FLOOR),
    (FishSpecies::Crabfish, ON_THE_FLOOR),
    (FishSpecies::Snailfish, ON_THE_FLOOR),
];
const HIDDEN: [FishSpecies; 3] = [
    FishSpecies::Cheatfish,
    FishSpecies::Junkfish,
    FishSpecies::Toyfish,
];
const RING_TICKS: usize = 240;
const SIGNAL_EVERY: usize = 3;
const FISH_PADS: Pads = Pads {
    side: PAD_X,
    top: PAD_Y,
    bottom: PAD_Y,
};
const COW_PADS: Pads = Pads {
    side: COW_SIDE_PAD,
    top: COW_TOP_PAD,
    bottom: 0,
};
const BOWL_SPOT: (f32, f32) = (40.0, 16.0);
const STACK_STEP: f32 = (1 + PAD_Y) as f32;
const ALGAE_EVERY: usize = 2;
const GRAEAE_SIGHT: [bool; 3] = [true, false, false];
const SPELL: usize = 60;
const TURN_TICKS: usize = 4 * SPELL;
const SHAPE_TICKS: usize = 5 * SPELL;
const SCATTER_SECS: f32 = 3.0;
const SCATTER_TICKS: usize = 5 * SPELL / 2;
const FAULT_TICKS: usize = SPELL + (FAULT_SLIP_SECS * DEFAULT_FPS) as usize;
const MOLT_DROP: f32 = 4.0;
const MOLT_EVERY: usize = ((MOLT_DROP + 1.0) / SHED_SINK_PER_SEC * DEFAULT_FPS) as usize;

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

#[derive(Clone, Copy)]
struct Margin {
    top: u16,
    bottom: u16,
}

impl Margin {
    const EVEN: Margin = Margin {
        top: PAD_Y,
        bottom: PAD_Y,
    };

    fn of(species: FishSpecies) -> Self {
        TRIMMED
            .iter()
            .find(|(trimmed, _)| *trimmed == species)
            .map_or(Self::EVEN, |&(_, margin)| margin)
    }
}

struct Canvas {
    width: u16,
    above: u16,
    below: u16,
    tall: Option<u16>,
    margin: Margin,
}

impl Canvas {
    fn fitting(frames: &[Vec<Fish>], margin: Margin) -> Self {
        let mut canvas = Canvas {
            width: 1,
            above: 0,
            below: 0,
            tall: None,
            margin,
        };
        for frame in frames {
            canvas.width = canvas.width.max(row_width(frame));
            for fish in frame {
                if is_multi(fish) {
                    let height = fish_art_height(fish, 1);
                    canvas.tall = Some(canvas.tall.map_or(height, |tall| tall.max(height)));
                    continue;
                }
                let (above, below) = rows_around_body(fish);
                canvas.above = canvas.above.max(above);
                canvas.below = canvas.below.max(below);
            }
        }
        canvas
    }

    fn area(&self) -> Rect {
        let art = self.tall.unwrap_or(self.above + 1 + self.below);
        let height = self.margin.top + art + self.margin.bottom;
        Rect::new(0, 0, self.width + 2 * PAD_X, height)
    }

    fn body_y(&self) -> u16 {
        self.margin.top + self.above
    }

    fn draw(&self, frame: &[Fish]) -> Buffer {
        let area = self.area();
        let mut buffer = Buffer::empty(area);
        let mut x = area.width.saturating_sub(row_width(frame)) / 2;
        for fish in frame {
            let width = art_width(fish);
            if is_multi(fish) {
                let height = fish_art_height(fish, 1);
                let top = self.margin.top;
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
    let sprite = fish.line_sprite();
    let painted: Vec<usize> = sprite
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.iter().any(|&(ch, _)| ch != TRANSPARENT && ch != ' '))
        .map(|(index, _)| index)
        .collect();
    let first = painted.first().copied().unwrap_or(sprite.body_row);
    let last = painted.last().copied().unwrap_or(sprite.body_row);
    let above = sprite.body_row.saturating_sub(first) as u16;
    let below = last.saturating_sub(sprite.body_row) as u16;
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
    margin: Margin,
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
    let canvas = Canvas::fitting(&frames, margin);
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
    let buffer = Canvas::fitting(std::slice::from_ref(&frame), Margin::EVEN).draw(&frame);
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
        Habitat::Claw => "Claw".to_string(),
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
        config
            .palette
            .iter()
            .map(|color| format!("{color:?}"))
            .collect::<Vec<_>>()
            .join(";"),
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
    row.extend((0..13).map(|_| NOTHING.to_string()));
    row.push(sprite_text(fish));
    row.join("\t")
}

const TOY_PART_BODY: ToyColor = ToyColor::White;
const TOY_PART_PAINT: Paint = Paint::Sky;
const TOY_MATERIAL_COLOR: ToyColor = ToyColor::Sky;

fn toy_shot(slug: &str, toy: ToyState, size: SizeCategory) -> Shot {
    let mut rng = rand::rng();
    let fish = posed(&Fish::new_toy(toy, size, &mut rng), Direction::Left);
    shoot(
        slug,
        Margin::EVEN,
        vec![(vec![fish], PORTRAIT_TICKS)],
        |_, _, _| {},
    )
}

fn toy_shots(tsv: &mut String, shots: &mut Vec<Shot>) {
    for line in Line::ALL {
        for signature in line.signatures() {
            let slug = format!("toy-{}", slug_of(signature.name()));
            let parts: Vec<&str> = signature
                .fittings(false)
                .worn()
                .iter()
                .map(|fitted| fitted.part.name())
                .collect();
            let name = format!(
                "{}:{}:{}:{}",
                signature.name(),
                line.name(),
                signature.material().name(),
                parts.join(",")
            );
            let _ = writeln!(tsv, "{}", bare_row("signature", &slug, &name));
            shots.push(toy_shot(
                &slug,
                ToyState::signature(signature, false),
                ToyState::signature_size(),
            ));
        }
    }
    for color in ToyColor::ALL {
        let slug = format!("toy-color-{}", slug_of(color.name()));
        let rarity = format!("{:?}", color.rarity());
        let name = format!("{}:{rarity}", color.name());
        let _ = writeln!(tsv, "{}", bare_row("toycolor", &slug, &name));
        shots.push(toy_shot(
            &slug,
            ToyState::plain(color, Material::Plastic),
            SizeCategory::M,
        ));
    }
    for material in Material::ALL {
        let slug = format!("toy-material-{}", slug_of(material.name()));
        let name = format!("{}:{:?}", material.name(), material.rarity());
        let _ = writeln!(tsv, "{}", bare_row("material", &slug, &name));
        shots.push(toy_shot(
            &slug,
            ToyState::plain(TOY_MATERIAL_COLOR, material),
            SizeCategory::L,
        ));
    }
    for part in ToyPart::ALL {
        let slug = format!("toy-part-{}", slug_of(part.name()));
        let name = format!("{}:{}:{:?}", part.name(), part.slot().name(), part.rarity());
        let _ = writeln!(tsv, "{}", bare_row("toypart", &slug, &name));
        let mut toy = ToyState::plain(TOY_PART_BODY, Material::Plastic);
        toy.fittings.set(
            part.slot(),
            Some(FittedPart {
                part,
                paint: TOY_PART_PAINT,
            }),
        );
        shots.push(toy_shot(&slug, toy, SizeCategory::L));
    }
}

fn bare_row(kind: &str, slug: &str, name: &str) -> String {
    let mut row = vec![kind.to_string(), slug.to_string(), name.to_string()];
    row.extend((0..19).map(|_| NOTHING.to_string()));
    row.join("\t")
}

#[derive(Clone, Copy, PartialEq)]
enum Cue {
    Calm,
    Zoomie,
    Teleport,
}

#[derive(Clone, Copy, PartialEq)]
enum Water {
    Bubbly,
    Sprinkled,
    Clear,
}

struct Pond {
    tank: Tank,
    sky: Sky,
    cue: Cue,
    water: Water,
}

impl Pond {
    fn new(kind: TankKind, size: (u16, u16), night: bool) -> Self {
        let mut tank = Tank::new("Pond".to_string(), kind, &[]);
        tank.resize(size.0, size.1, &[]);
        match &mut tank.background {
            TankBackground::Plain { plants } => plants.clear(),
            TankBackground::Rad { bg } => bg.barrels.clear(),
            _ => {}
        }
        let sky = Sky {
            daylight: !night,
            ..Sky::default()
        };
        Self {
            tank,
            sky,
            cue: Cue::Calm,
            water: Water::Bubbly,
        }
    }

    fn without_bubbles(mut self) -> Self {
        self.water = Water::Sprinkled;
        self
    }

    fn clear(mut self) -> Self {
        self.water = Water::Clear;
        self
    }

    fn step(&mut self, settings: &Settings) {
        self.tank.tick(settings, 0, self.sky);
        match self.water {
            Water::Bubbly => {}
            Water::Sprinkled => self.tank.bubbles.retain(|bubble| !bubble.poppable),
            Water::Clear => self.tank.bubbles.clear(),
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
            self.step(&settings);
        }
        let mut lead: Vec<Buffer> = Vec::new();
        let mut filming: Option<usize> = None;
        for tick in 0..POND_LOOKOUT_TICKS {
            let last_x = self.tank.fish.first().map_or(0.0, |fish| fish.position.x);
            self.step(&settings);
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

fn shines_at_night(config: &SpeciesConfig) -> bool {
    config.born_with.contains(&Mutation::Lure) || config.habit == Some(Habit::Twinkle)
}

fn box_shot(species: FishSpecies, slug: &str) -> Shot {
    let config = species.config();
    Pond::new(TankKind::Base, BOX_POND, shines_at_night(&config))
        .with_species(species, config.name)
        .without_bubbles()
        .film(slug)
}

fn show_gift(fish: &mut Fish, config: &SpeciesConfig, showing: bool) {
    if config.born_with.contains(&Mutation::Puff) {
        fish.habits.puffed = if showing { PUFF_SECS } else { 0.0 };
    }
    if config.habit == Some(Habit::Sync) {
        fish.habits.lit = if showing { FLASH_SECS } else { 0.0 };
    }
    if config.zoomie == Zoomie::Glide {
        fish.state = if showing {
            FishState::Zoomie {
                time_remaining: GLIDE_SECS,
                total_duration: GLIDE_SECS,
                will_turn: false,
                has_turned: false,
            }
        } else {
            FishState::Idle
        };
    }
}

fn portrait_shot(species: FishSpecies, fish: &Fish, slug: &str) -> Shot {
    let config = species.config();
    let auto = config.auto_mutate;
    let ticks = if auto {
        MUTANT_STEP_TICKS * MUTANT_STEPS
    } else {
        PORTRAIT_TICKS
    };
    let mut fish = fish.clone();
    fish.sky.daylight = !shines_at_night(&config);
    shoot(
        slug,
        Margin::of(species),
        vec![(vec![fish], ticks)],
        |fishes, _, tick| {
            show_gift(&mut fishes[0], &config, (tick / GIFT_TICKS) % 2 == 1);
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
    )
}

fn species_shot(species: FishSpecies, rng: &mut impl rand::RngExt) -> (Shot, Fish) {
    let fish = posed(&Fish::new_for_display(species, rng), Direction::Left);
    let slug = format!("fish-{}", slug_of(species.display_name()));
    let shot = if species.config().locomotion == Locomotion::Bounce {
        box_shot(species, &slug)
    } else {
        portrait_shot(species, &fish, &slug)
    };
    (shot, fish)
}

fn unfish_shot(kind: UnfishKind, rng: &mut impl rand::RngExt) -> (Shot, Fish) {
    let fish = Fish::new_unfish(kind, String::new(), 0.0, 0.0, rng);
    let slug = format!("unfish-{}", slug_of(&format!("{kind:?}")));
    let shot = match kind {
        UnfishKind::Phantom => Pond::new(TankKind::Base, BOX_POND, false)
            .with_fish(Fish::new_unfish(kind, format!("{kind:?}"), 0.0, 0.0, rng))
            .without_bubbles()
            .waiting_for(Cue::Teleport)
            .film(&slug),
        UnfishKind::Blinker => shoot(
            &slug,
            Margin::EVEN,
            vec![(vec![just_before_a_blink(&fish)], BLINK_TICKS)],
            |_, _, _| {},
        ),
        UnfishKind::Signal => shoot(
            &slug,
            Margin::EVEN,
            vec![(vec![posed(&fish, Direction::Left)], PORTRAIT_TICKS)],
            |fishes, _, tick| {
                if tick % SIGNAL_EVERY == 0 {
                    fishes[0].broadcast();
                }
            },
        ),
        UnfishKind::Ouroboros => shoot(
            &slug,
            Margin::EVEN,
            vec![(vec![posed(&fish, Direction::Left)], RING_TICKS)],
            |_, _, _| {},
        ),
        UnfishKind::Absence => absence_shot(&slug, rng),
        UnfishKind::Bones => bones_shot(&slug, rng),
        UnfishKind::Forgetting => {
            Bowl::of(kind, 1, rng).film(&slug, 0, SHAPE_TICKS, |tank, tick| {
                if tick % (2 * SPELL) == SPELL
                    && let Some(Quirk::Forgetting(forgetting)) = tank.fish[0].quirk_mut()
                {
                    forgetting.shape_clock = 0.0;
                }
            })
        }
        UnfishKind::Verso => Bowl::of(kind, 1, rng).film(&slug, 0, TURN_TICKS, |tank, tick| {
            if tick % (2 * SPELL) == SPELL {
                tank.fish[0].flip();
            }
        }),
        UnfishKind::Fault => Bowl::of(kind, 1, rng).film(&slug, 0, FAULT_TICKS, |tank, tick| {
            if let Some(Quirk::Fault(fault)) = tank.fish[0].quirk_mut() {
                fault.clock = FAULT_SLIP_MEAN_SECS;
                if tick == SPELL / 2 {
                    fault.slip = FAULT_SLIP_SECS;
                }
            }
        }),
        UnfishKind::Anagram => Bowl::of(kind, 1, rng).film(&slug, 0, SHAPE_TICKS, |tank, tick| {
            if let Some(Quirk::Anagram(anagram)) = tank.fish[0].quirk_mut() {
                anagram.clock = if tick % SPELL == 0 {
                    0.0
                } else {
                    ANAGRAM_SHUFFLE_MEAN_SECS
                };
            }
        }),
        UnfishKind::Reflection => {
            Bowl::of(kind, 1, rng).film(&slug, 0, TURN_TICKS, |tank, tick| {
                if let Some(Quirk::Reflection(reflection)) = tank.fish[0].quirk_mut() {
                    reflection.clock = REFLECTION_DISAGREE_MEAN_SECS;
                    if tick % (2 * SPELL) == SPELL {
                        reflection.mood = Mood::Strays;
                        reflection.mood_secs = SPELL as f32 / DEFAULT_FPS;
                    }
                }
            })
        }
        UnfishKind::Molt => molt_shot(&slug, rng),
        UnfishKind::Graeae => {
            Bowl::of(kind, GRAEAE_SIGHT.len(), rng).film(&slug, 0, PORTRAIT_TICKS, |tank, _| {
                for (sister, sighted) in tank.fish.iter_mut().zip(GRAEAE_SIGHT) {
                    if let Some(Quirk::Graeae(graeae)) = sister.quirk_mut() {
                        graeae.sighted = sighted;
                        graeae.rest = GRAEAE_PASS_REST_SECS;
                    }
                }
            })
        }
        UnfishKind::Leech | UnfishKind::Negative | UnfishKind::Still => {
            Bowl::of(kind, 1, rng).film(&slug, 0, PORTRAIT_TICKS, |_, _| {})
        }
        _ => shoot(
            &slug,
            Margin::EVEN,
            vec![(vec![posed(&fish, Direction::Left)], PORTRAIT_TICKS)],
            |_, _, _| {},
        ),
    };
    (shot, posed(&fish, Direction::Left))
}

struct Bowl {
    pond: Pond,
    rows: Vec<f32>,
}

impl Bowl {
    fn new(size: (u16, u16), spot: (f32, f32), cast: Vec<Fish>) -> Self {
        let mut pond = Pond::new(TankKind::Base, size, false).clear();
        let mut rows = Vec::new();
        for (k, mut fish) in cast.into_iter().enumerate() {
            if fish.facing != Direction::Left {
                fish.flip();
            }
            fish.position.x = spot.0;
            fish.position.y = spot.1 + k as f32 * STACK_STEP;
            rows.push(fish.position.y);
            pond.tank.place_fish_dropped(fish);
        }
        Self { pond, rows }
    }

    fn of(kind: UnfishKind, count: usize, rng: &mut impl rand::RngExt) -> Self {
        let cast = (0..count)
            .map(|_| Fish::new_unfish(kind, format!("{kind:?}"), 0.0, 0.0, rng))
            .collect();
        Self::new(STAGE, BOWL_SPOT, cast)
    }

    fn with_algae(mut self, rng: &mut impl rand::RngExt) -> Self {
        let width = self.pond.tank.width;
        if let TankBackground::Plain { plants } = &mut self.pond.tank.background {
            *plants = (0..width)
                .step_by(ALGAE_EVERY)
                .map(|x| {
                    let height = rng.random_range(PLANT_HEIGHT_MIN..=PLANT_HEIGHT_MAX);
                    Plant::new(i32::from(x), height, rng)
                })
                .collect();
        }
        self
    }

    fn hold(&mut self) {
        for (fish, &row) in self.pond.tank.fish.iter_mut().zip(&self.rows) {
            fish.speed = 0.0;
            fish.velocity.dx = 0.0;
            fish.velocity.dy = 0.0;
            fish.position.y = row;
        }
    }

    fn stills(
        mut self,
        warmup: usize,
        ticks: usize,
        mut hook: impl FnMut(&mut Tank, usize),
    ) -> Vec<Buffer> {
        let settings = Settings::default();
        let mut stills = Vec::new();
        for tick in 0..warmup + ticks {
            self.hold();
            self.pond.step(&settings);
            self.hold();
            hook(&mut self.pond.tank, tick);
            if tick >= warmup && tick % FILM_EVERY == 0 {
                stills.push(self.pond.frame());
            }
        }
        stills
    }

    fn film(
        self,
        slug: &str,
        warmup: usize,
        ticks: usize,
        hook: impl FnMut(&mut Tank, usize),
    ) -> Shot {
        let stills = self.stills(warmup, ticks, hook);
        let paddock = Paddock::around(&stills, FISH_PADS);
        let mut shot = Shot::new(slug);
        for still in &stills {
            shot.push(&paddock.crop(still));
        }
        shot
    }
}

fn absence_shot(slug: &str, rng: &mut impl rand::RngExt) -> Shot {
    let grid = UnfishKind::Absence.grid().expect("the Absence has a grid");
    let size = (grid.width + 2 * PAD_X, grid.height + 2 * PAD_Y);
    let spot = (f32::from(PAD_X), f32::from(PAD_Y) + grid.center as f32);
    let hole = Fish::new_unfish(UnfishKind::Absence, "Absence".into(), 0.0, 0.0, rng);
    let stills =
        Bowl::new(size, spot, vec![hole])
            .with_algae(rng)
            .stills(0, PORTRAIT_TICKS, |_, _| {});
    let mut shot = Shot::new(slug);
    for still in &stills {
        shot.push(still);
    }
    shot
}

fn bones_shot(slug: &str, rng: &mut impl rand::RngExt) -> Shot {
    let bowl = Bowl::of(UnfishKind::Bones, 1, rng);
    let whole = bowl.pond.tank.fish[0].clone();
    bowl.film(slug, 0, SCATTER_TICKS, |tank, tick| {
        tank.fish.truncate(1);
        let bones = &mut tank.fish[0];
        bones.position.x = whole.position.x;
        if bones.body_size != whole.body_size {
            bones.body_size = whole.body_size;
            bones.refresh_width();
        }
        if tick == SPELL / 2 {
            bones.state = FishState::Zoomie {
                time_remaining: SCATTER_SECS,
                total_duration: SCATTER_SECS,
                will_turn: false,
                has_turned: false,
            };
        }
    })
}

fn molt_shot(slug: &str, rng: &mut impl rand::RngExt) -> Shot {
    let bowl = Bowl::of(UnfishKind::Molt, 1, rng);
    let lowest = bowl.rows[0] + MOLT_DROP + 1.0;
    bowl.film(slug, MOLT_EVERY, MOLT_EVERY, |tank, tick| {
        tank.sheddings.retain(|shed| shed.y < lowest);
        if let Some(Quirk::Molt(molt)) = tank.fish[0].quirk_mut() {
            molt.clock = if tick % MOLT_EVERY == 0 {
                0.0
            } else {
                MOLT_MEAN_SECS
            };
        }
    })
}

fn just_before_a_blink(fish: &Fish) -> Fish {
    let mut history = vec![posed(fish, Direction::Left)];
    for _ in 0..POND_LOOKOUT_TICKS {
        let mut next = history[history.len() - 1].clone();
        animate(std::slice::from_mut(&mut next));
        let blinked = next.is_invisible();
        history.push(next);
        if blinked {
            let lead = history.len().saturating_sub(POND_LEAD_TICKS + 1);
            return history.swap_remove(lead);
        }
        if history.len() > POND_LEAD_TICKS + 1 {
            history.remove(0);
        }
    }
    history.swap_remove(0)
}

#[derive(Clone, Copy)]
struct Pads {
    side: u16,
    top: u16,
    bottom: u16,
}

struct Paddock {
    bounds: Rect,
    pads: Pads,
}

impl Paddock {
    fn around(stills: &[Buffer], pads: Pads) -> Self {
        let bounds = stills
            .iter()
            .filter_map(painted_bounds)
            .reduce(Rect::union)
            .expect("the stage paints something");
        let stage = Rect::new(0, 0, STAGE.0, STAGE.1);
        let room = Rect::new(
            bounds.x.saturating_sub(pads.side),
            bounds.y.saturating_sub(pads.top),
            bounds.width + 2 * pads.side,
            bounds.height + pads.top + pads.bottom,
        );
        assert_eq!(stage.intersection(room), room, "the frame fits its stage");
        Self { bounds, pads }
    }

    fn area(&self) -> Rect {
        Rect::new(
            0,
            0,
            self.bounds.width + 2 * self.pads.side,
            self.bounds.height + self.pads.top + self.pads.bottom,
        )
    }

    fn left(&self) -> u16 {
        self.bounds.x - self.pads.side
    }

    fn top(&self) -> u16 {
        self.bounds.y - self.pads.top
    }

    fn crop(&self, stage: &Buffer) -> Buffer {
        let area = self.area();
        let mut buffer = Buffer::empty(area);
        for y in 0..area.height {
            for x in 0..area.width {
                buffer[(x, y)] = stage[(self.left() + x, self.top() + y)].clone();
            }
        }
        buffer
    }

    fn moved(&self, cow: &Cow) -> Cow {
        let mut cow = cow.clone();
        cow.position.x -= f32::from(self.left());
        cow.position.y -= f32::from(self.top());
        cow
    }
}

fn on_stage(cow: &Cow) -> Buffer {
    let area = Rect::new(0, 0, STAGE.0, STAGE.1);
    let mut buffer = Buffer::empty(area);
    render_cow(cow, area, &mut buffer);
    buffer
}

fn stage_all(cows: &[Cow]) -> Vec<Buffer> {
    cows.iter().map(on_stage).collect()
}

fn painted_bounds(buffer: &Buffer) -> Option<Rect> {
    let area = buffer.area;
    let painted: Vec<(u16, u16)> = (0..area.height)
        .flat_map(|y| (0..area.width).map(move |x| (x, y)))
        .filter(|&(x, y)| buffer[(x, y)].symbol() != " ")
        .collect();
    let left = painted.iter().map(|&(x, _)| x).min()?;
    let right = painted.iter().map(|&(x, _)| x).max()?;
    let top = painted.iter().map(|&(_, y)| y).min()?;
    let bottom = painted.iter().map(|&(_, y)| y).max()?;
    Some(Rect::new(left, top, right - left + 1, bottom - top + 1))
}

fn staged_cow(variant: CowVariant) -> Cow {
    Cow::new(
        COW_NAME.to_string(),
        variant,
        COW_SPOT.0,
        COW_SPOT.1,
        &mut rand::rng(),
    )
}

fn cow_frames(mut cow: Cow, ticks: usize, step: impl Fn(&mut Cow, usize)) -> Vec<Cow> {
    let dt = 1.0 / DEFAULT_FPS;
    let mut frames = Vec::new();
    for tick in 0..ticks {
        step(&mut cow, tick);
        cow.tick(dt, Sky::default());
        if tick % FILM_EVERY == 0 {
            frames.push(cow.clone());
        }
    }
    frames
}

fn cow_shot(variant: CowVariant, slug: &str) -> Shot {
    let frames = cow_frames(staged_cow(variant), PORTRAIT_TICKS, |_, _| {});
    let paddock = Paddock::around(&stage_all(&frames), COW_PADS);
    let mut shot = Shot::new(slug);
    for cow in &frames {
        shot.push(&paddock.crop(&on_stage(cow)));
    }
    shot
}

fn shows_hydra(cow: &Cow) -> bool {
    cow.hydra_count().min(cow.hydra_max()) > 0
}

fn mutate_in_sight(cow: &mut Cow, step: usize) {
    let mut rng = rand::rng();
    let before = on_stage(cow);
    for _ in 0..MUTATION_TRIES {
        let mutation = match SHOWCASE.get(step) {
            Some(&showcased) => showcased,
            None => match cow.random_mutation_with_room(&mut rng, false) {
                Some(drawn) => drawn,
                None => return,
            },
        };
        let mut trial = cow.clone();
        apply_mutation(&mut trial, mutation, &mut rng);
        trial.refresh_width();
        if shows_hydra(&trial) && on_stage(&trial) != before {
            *cow = trial;
            return;
        }
    }
}

fn irradiated_cow_shot(slug: &str) -> Shot {
    let frames = cow_frames(
        staged_cow(CowVariant::Brown),
        MUTANT_STEP_TICKS * (COW_MUTATIONS + 1),
        |cow, tick| {
            if tick > 0 && tick % MUTANT_STEP_TICKS == 0 {
                mutate_in_sight(cow, tick / MUTANT_STEP_TICKS - 1);
            }
        },
    );
    assert!(
        frames.last().is_some_and(shows_hydra),
        "the irradiated cow keeps its hydra head"
    );
    let paddock = Paddock::around(&stage_all(&frames), COW_PADS);
    let area = paddock.area();
    let mut pond = Pond::new(TankKind::Rad, (area.width, area.height), false);
    let settings = Settings::default();
    for _ in 0..POND_WARMUP_TICKS {
        pond.step(&settings);
    }
    let mut shot = Shot::new(slug);
    for cow in &frames {
        for _ in 0..FILM_EVERY {
            pond.step(&settings);
        }
        pond.tank.cows = vec![paddock.moved(cow)];
        shot.push(&pond.frame());
        pond.tank.cows.clear();
    }
    shot
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
        Margin::EVEN,
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
    }
    for &kind in SPAWNABLE_UNFISH {
        let (shot, fish) = unfish_shot(kind, &mut rng);
        let _ = writeln!(tsv, "{}", unfish_row(kind, &fish));
        shots.push(shot);
    }
    for &variant in CowVariant::ALL {
        let name = format!("{variant:?}");
        let slug = format!("cow-{}", slug_of(&name));
        shots.push(cow_shot(variant, &slug));
        let milk = format!("{:?}", variant.milk());
        let _ = writeln!(tsv, "{}", bare_row("cow", &slug, &format!("{name}:{milk}")));
    }
    shots.push(irradiated_cow_shot("cow-irradiated"));
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
    toy_shots(&mut tsv, &mut shots);
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
