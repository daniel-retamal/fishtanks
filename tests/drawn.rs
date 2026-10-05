use fishtank::{
    entities::cow::{Cow, CowVariant},
    fishes::{
        fish::{Direction, Fish, PUFF_SECS},
        mutations::{Mutatable, Mutation, apply_mutation},
        species::{ALL_SPECIES, FishSpecies},
        unfish::SPAWNABLE_UNFISH,
    },
    tank::{FULL_MOON, Sky, Tank, TankBackground, TankKind},
    ui::tank_view::TankView,
};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

const TANK_W: u16 = 48;
const TANK_H: u16 = 20;
const SPOT: (f32, f32) = (14.0, 10.0);
const PATCH_SAMPLES: usize = 2000;
const PHASES: [f32; 2] = [0.0, 1.6];
const SKIES: [Sky; 2] = [
    Sky {
        daylight: true,
        moon: FULL_MOON,
        calm: false,
    },
    Sky {
        daylight: false,
        moon: FULL_MOON / 2,
        calm: false,
    },
];
const UNSEEN_BY_DESIGN: [Mutation; 11] = [
    Mutation::NightOwl,
    Mutation::HelpedByGod,
    Mutation::Alienation,
    Mutation::Strawberry,
    Mutation::Engulfment,
    Mutation::Cytokinesis,
    Mutation::Endocytosis,
    Mutation::Revert,
    Mutation::WakeColor,
    Mutation::GlistenFast,
    Mutation::GlistenSlow,
];
const GROWN: [Mutation; 4] = [
    Mutation::Ear,
    Mutation::Feet,
    Mutation::GlistenEnable,
    Mutation::Telophase,
];

#[derive(Clone)]
enum Being {
    Fish(Fish),
    Cow(Cow),
}

impl Being {
    fn name(&self) -> String {
        match self {
            Being::Fish(fish) => match fish.unfish_body() {
                Some(kind) => format!("Unfish {kind:?}"),
                None => fish.species.display_name().to_string(),
            },
            Being::Cow(cow) => format!("Cow {:?}", cow.variant),
        }
    }

    fn available(&self) -> Vec<Mutation> {
        match self {
            Being::Fish(fish) => fish.available_mutations(),
            Being::Cow(cow) => cow.available_mutations(),
        }
    }

    fn mutate(&mut self, mutation: Mutation) {
        let mut rng = rand::rng();
        match self {
            Being::Fish(fish) => {
                apply_mutation(fish, mutation, &mut rng);
            }
            Being::Cow(cow) => {
                apply_mutation(cow, mutation, &mut rng);
            }
        }
    }

    fn puffs(&self) -> bool {
        matches!(self, Being::Fish(fish) if fish.adornments().puff)
    }

    fn frames(&self) -> Vec<Vec<String>> {
        let mut frames = Vec::new();
        for facing in [Direction::Left, Direction::Right] {
            for phase in PHASES {
                for sky in SKIES {
                    frames.push(self.frame(facing, phase, sky, false));
                    if self.puffs() {
                        frames.push(self.frame(facing, phase, sky, true));
                    }
                }
            }
        }
        frames
    }

    fn frame(&self, facing: Direction, phase: f32, sky: Sky, puffed: bool) -> Vec<String> {
        let mut tank = Tank::new("Lab".into(), TankKind::Base, &[]);
        tank.resize(TANK_W, TANK_H, &[]);
        if let TankBackground::Plain { plants } = &mut tank.background {
            plants.clear();
        }
        match self.clone() {
            Being::Fish(mut fish) => {
                fish.position.x = SPOT.0;
                fish.position.y = SPOT.1;
                fish.facing = facing;
                fish.sway.phase = phase;
                fish.sky = sky;
                if let Some(mutant) = fish.mutant.as_mut() {
                    for eye in mutant.all_eyes_mut() {
                        eye.set_open(true);
                    }
                }
                if puffed {
                    fish.habits.puffed = PUFF_SECS;
                }
                tank.fish.push(fish);
            }
            Being::Cow(mut cow) => {
                cow.position.x = SPOT.0;
                cow.position.y = SPOT.1;
                cow.sway.phase = phase;
                cow.sky = sky;
                tank.cows.push(cow);
            }
        }
        let area = Rect::new(0, 0, TANK_W, TANK_H);
        let mut buf = Buffer::empty(area);
        TankView::new(&tank)
            .with_epitaphs(false)
            .render(area, &mut buf);
        (0..TANK_H)
            .map(|y| {
                (0..TANK_W)
                    .map(|x| {
                        let cell = &buf[(x, y)];
                        format!("{}{:?}{:?}", cell.symbol(), cell.fg, cell.bg)
                    })
                    .collect()
            })
            .collect()
    }
}

fn beings() -> Vec<Being> {
    let mut rng = rand::rng();
    let mut beings: Vec<Being> = ALL_SPECIES
        .iter()
        .map(|&species| Being::Fish(Fish::new(species, "Probe".into(), SPOT.0, SPOT.1, &mut rng)))
        .collect();
    for &kind in SPAWNABLE_UNFISH {
        beings.push(Being::Fish(Fish::new_unfish(
            kind,
            "Probe".into(),
            SPOT.0,
            SPOT.1,
            &mut rng,
        )));
    }
    for &variant in CowVariant::ALL {
        beings.push(Being::Cow(Cow::new(
            "Vaquita".into(),
            variant,
            SPOT.0,
            SPOT.1,
            &mut rng,
        )));
    }
    beings
}

fn unseen(being: &Being) -> Vec<String> {
    let before = being.frames();
    being
        .available()
        .into_iter()
        .filter(|mutation| !UNSEEN_BY_DESIGN.contains(mutation))
        .filter(|&mutation| {
            let mut mutated = being.clone();
            mutated.mutate(mutation);
            mutated.frames() == before
        })
        .map(|mutation| format!("{} {}", being.name(), mutation.token()))
        .collect()
}

#[test]
fn every_mutation_a_being_is_offered_changes_how_it_is_drawn() {
    let mut invisible = Vec::new();
    for being in beings() {
        invisible.extend(unseen(&being));
        let mut grown = being.clone();
        for mutation in GROWN {
            if grown.available().contains(&mutation) {
                grown.mutate(mutation);
            }
        }
        invisible.extend(
            unseen(&grown)
                .into_iter()
                .map(|line| format!("grown {line}")),
        );
    }
    assert!(
        invisible.is_empty(),
        "offered but never drawn:\n{}",
        invisible.join("\n")
    );
}

#[test]
fn a_botfish_is_exactly_as_wide_as_it_draws() {
    let mut rng = rand::rng();
    let mut bot = Fish::new(
        fishtank::fishes::species::FishSpecies::Botfish,
        "Neo".into(),
        SPOT.0,
        SPOT.1,
        &mut rng,
    );
    let widest = |fish: &Fish| {
        fish.line_sprite()
            .rows
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(0)
    };
    for mutation in [Mutation::Lure, Mutation::Telophase] {
        apply_mutation(&mut bot, mutation, &mut rng);
        for facing in [Direction::Left, Direction::Right] {
            bot.facing = facing;
            assert_eq!(
                widest(&bot),
                bot.display_width,
                "after {} facing {facing:?}",
                mutation.token()
            );
        }
    }
    assert!(
        !bot.supports_now(Mutation::SizeIncrease),
        "its body is a fixed board"
    );
}

#[test]
fn a_colour_patch_always_lands_on_a_cell_the_fish_draws() {
    for _ in 0..PATCH_SAMPLES {
        let born_with_an_ear = Being::Fish(Fish::new(
            FishSpecies::Rabbitfish,
            "Probe".into(),
            SPOT.0,
            SPOT.1,
            &mut rand::rng(),
        ));
        let before = born_with_an_ear.frames();
        let mut patched = born_with_an_ear.clone();
        patched.mutate(Mutation::ColorPatch);
        assert!(
            patched.frames() != before,
            "a patch fell off the drawn body"
        );
    }
}
