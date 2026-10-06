use rand::{RngExt, SeedableRng, rngs::SmallRng};

use ratatui::style::Color;

use crate::colors::{
    INDIGO, LIGHT_MAGENTA, MAGENTA, PINK, PURPLE, PURPLE_DARK, PURPLE_LIGHT, VIOLET,
};
use crate::entities::components::extend_spaced;
use crate::entities::plant::{Plant, Seaweed};
use crate::fishes::fish::Fish;
use crate::tanks::alien::{AlienBackground, extend_alien_pyramids, extend_alien_tentacles};
use crate::tanks::candy::{CandyBackground, extend_candy_decos, extend_candy_plants};
use crate::tanks::coral::{CoralStructure, FloorAlgae, extend_coral_reef};
use crate::tanks::desert::{DesertSky, extend_desert_cacti};
use crate::tanks::haunted::{HauntedBackground, extend_haunted};
use crate::tanks::heaven::HeavenBackground;
use crate::tanks::hell::{HellBackground, HellPlant, extend_hell_plants};
use crate::tanks::matrix::{MatrixBackground, extend_matrix};
use crate::tanks::radioactive::{RadBackground, extend_rad};
use crate::tanks::soul_wall::SoulWall;
use crate::tanks::void::VoidBackground;

use super::{INITIAL_HEIGHT, INITIAL_WIDTH, TankKind};

const PLANT_SPACING_MIN: i32 = 3;
const PLANT_SPACING_MAX: i32 = 6;
pub const PLANT_HEIGHT_MIN: usize = 8;
pub const PLANT_HEIGHT_MAX: usize = 27;
const PLANT_SPAWN_LOOKAHEAD: i32 = 30;
const CORAL_SPAWN_LOOKAHEAD: i32 = 130;
const CANDY_PINK_PLANT_COLORS: &[Color] = &[
    PINK,
    MAGENTA,
    LIGHT_MAGENTA,
    PURPLE_DARK,
    PURPLE,
    PURPLE_LIGHT,
    VIOLET,
    INDIGO,
];

#[derive(Clone, Copy)]
enum Lane {
    Frame,
    Growth,
    Structures,
    Accents,
}

const LANES: usize = 4;

pub struct Scenery {
    lanes: [SmallRng; LANES],
}

impl Scenery {
    pub fn new(seed: u64) -> Self {
        Self {
            lanes: std::array::from_fn(|lane| {
                SmallRng::seed_from_u64(seed.wrapping_add(lane as u64))
            }),
        }
    }

    fn lane(&mut self, lane: Lane) -> &mut SmallRng {
        &mut self.lanes[lane as usize]
    }
}

pub enum TankBackground {
    Plain {
        plants: Vec<Plant>,
    },
    Coral {
        plants: Vec<Plant>,
        corals: Vec<CoralStructure>,
        floor_algae: Vec<FloorAlgae>,
    },
    Hell {
        bg: HellBackground,
        plants: Vec<HellPlant>,
    },
    Void {
        bg: VoidBackground,
    },
    Alien {
        bg: AlienBackground,
    },
    Haunted {
        bg: HauntedBackground,
    },
    Candy {
        bg: CandyBackground,
        pink_plants: Vec<Plant>,
    },
    Desert {
        bg: DesertSky,
    },
    Rad {
        bg: RadBackground,
    },
    Matrix {
        bg: MatrixBackground,
    },
    Heaven {
        bg: HeavenBackground,
    },
}

impl TankBackground {
    pub fn new(kind: TankKind, dead_names: &[String], scenery: &mut Scenery) -> Self {
        let mut background = match kind {
            TankKind::Base => TankBackground::Plain { plants: Vec::new() },
            TankKind::CoralReef => TankBackground::Coral {
                plants: Vec::new(),
                corals: Vec::new(),
                floor_algae: Vec::new(),
            },
            TankKind::Hell => TankBackground::Hell {
                bg: HellBackground::new(scenery.lane(Lane::Frame)),
                plants: Vec::new(),
            },
            TankKind::Void => TankBackground::Void {
                bg: VoidBackground::new(),
            },
            TankKind::Alien => {
                let mut bg = AlienBackground::new(scenery.lane(Lane::Frame));
                bg.init_stars(scenery.lane(Lane::Frame), INITIAL_WIDTH, INITIAL_HEIGHT);
                TankBackground::Alien { bg }
            }
            TankKind::Haunted => TankBackground::Haunted {
                bg: HauntedBackground::new(scenery.lane(Lane::Frame)),
            },
            TankKind::Candy => TankBackground::Candy {
                bg: CandyBackground::new(),
                pink_plants: Vec::new(),
            },
            TankKind::Desert => {
                let mut bg = DesertSky::new(scenery.lane(Lane::Frame));
                bg.init_stars(scenery.lane(Lane::Frame), INITIAL_WIDTH, INITIAL_HEIGHT);
                TankBackground::Desert { bg }
            }
            TankKind::Rad => TankBackground::Rad {
                bg: RadBackground::new(),
            },
            TankKind::Matrix => TankBackground::Matrix {
                bg: MatrixBackground::new(0, scenery.lane(Lane::Frame)),
            },
            TankKind::Heaven => TankBackground::Heaven {
                bg: HeavenBackground::new(INITIAL_WIDTH, scenery.lane(Lane::Frame)),
            },
        };
        background.extend(INITIAL_WIDTH, dead_names, scenery);
        background
    }

    pub fn tick(&mut self, dt: f32, rng: &mut impl RngExt, width: u16, height: u16) {
        match self {
            TankBackground::Plain { plants } => {
                for plant in plants {
                    plant.tick(dt);
                }
            }
            TankBackground::Coral {
                plants,
                corals,
                floor_algae,
            } => {
                for plant in plants {
                    plant.tick(dt);
                }
                for coral in corals {
                    coral.tick(dt, rng);
                }
                for fa in floor_algae {
                    fa.tick(dt);
                }
            }
            TankBackground::Hell { bg, plants } => {
                bg.tick(dt, rng, width, height);
                for plant in plants {
                    plant.tick(dt);
                }
            }
            TankBackground::Void { bg } => bg.tick(),
            TankBackground::Alien { bg } => bg.tick(dt, rng, width, height),
            TankBackground::Haunted { bg } => bg.tick(dt, rng, width, height),
            TankBackground::Candy { pink_plants, .. } => {
                for plant in pink_plants {
                    plant.tick(dt);
                }
            }
            TankBackground::Desert { bg } => bg.tick(dt, rng, width, height),
            TankBackground::Rad { bg } => bg.tick(dt, rng),
            TankBackground::Matrix { bg } => bg.tick(dt, height, rng),
            TankBackground::Heaven { bg } => bg.tick(dt, rng, width, height),
        }
    }

    pub fn extend(&mut self, width: u16, dead_names: &[String], scenery: &mut Scenery) {
        let growth_to = width as i32 + PLANT_SPAWN_LOOKAHEAD;
        let structures_to = width as i32 + CORAL_SPAWN_LOOKAHEAD;
        match self {
            TankBackground::Plain { plants } => {
                extend_plants(plants, growth_to, scenery.lane(Lane::Growth));
            }
            TankBackground::Coral {
                plants,
                corals,
                floor_algae,
            } => {
                extend_plants(plants, growth_to, scenery.lane(Lane::Growth));
                extend_coral_reef(
                    corals,
                    floor_algae,
                    structures_to,
                    scenery.lane(Lane::Structures),
                );
            }
            TankBackground::Hell { plants, .. } => {
                extend_hell_plants(plants, growth_to, scenery.lane(Lane::Growth));
            }
            TankBackground::Void { .. } => {}
            TankBackground::Alien { bg } => {
                let color = bg.color;
                extend_alien_tentacles(
                    &mut bg.tentacles,
                    growth_to,
                    color,
                    scenery.lane(Lane::Growth),
                );
                extend_alien_pyramids(
                    &mut bg.pyramids,
                    structures_to,
                    scenery.lane(Lane::Structures),
                );
            }
            TankBackground::Haunted { bg } => {
                extend_haunted(
                    &mut bg.graves,
                    &mut bg.pumpkins,
                    structures_to,
                    dead_names,
                    scenery.lane(Lane::Structures),
                );
            }
            TankBackground::Candy { bg, pink_plants } => {
                extend_candy_plants(&mut bg.plants, structures_to, scenery.lane(Lane::Growth));
                extend_candy_decos(&mut bg.decos, structures_to, scenery.lane(Lane::Structures));
                extend_candy_pink_plants(pink_plants, structures_to, scenery.lane(Lane::Accents));
            }
            TankBackground::Desert { bg } => {
                extend_desert_cacti(&mut bg.cacti, structures_to, scenery.lane(Lane::Growth));
            }
            TankBackground::Rad { bg } => {
                extend_rad(
                    &mut bg.barrels,
                    &mut bg.floor,
                    structures_to,
                    scenery.lane(Lane::Structures),
                );
            }
            TankBackground::Matrix { bg } => {
                extend_matrix(&mut bg.columns, width as i32, scenery.lane(Lane::Growth));
            }
            TankBackground::Heaven { bg } => bg.extend(width, scenery.lane(Lane::Structures)),
        }
    }

    pub fn init_stars(&mut self, rng: &mut impl RngExt, width: u16, height: u16) {
        match self {
            TankBackground::Alien { bg } => bg.init_stars(rng, width, height),
            TankBackground::Desert { bg } => bg.init_stars(rng, width, height),
            _ => {}
        }
    }

    pub fn clear_grave_name(&mut self, name: &str) {
        if let TankBackground::Haunted { bg } = self {
            bg.clear_grave_name(name);
        }
    }

    pub fn soul_wall(&self) -> Option<&SoulWall> {
        match self {
            TankBackground::Heaven { bg } => Some(&bg.souls),
            TankBackground::Hell { bg, .. } => Some(&bg.souls),
            _ => None,
        }
    }

    fn soul_wall_mut(&mut self) -> Option<&mut SoulWall> {
        match self {
            TankBackground::Heaven { bg } => Some(&mut bg.souls),
            TankBackground::Hell { bg, .. } => Some(&mut bg.souls),
            _ => None,
        }
    }

    pub fn receive_soul(
        &mut self,
        fish: Fish,
        width: u16,
        height: u16,
        rng: &mut impl RngExt,
    ) -> bool {
        let Some(wall) = self.soul_wall_mut() else {
            return false;
        };
        wall.receive(fish, width, height, rng);
        true
    }

    pub fn release_soul(
        &mut self,
        name: &str,
        width: u16,
        height: u16,
        rng: &mut impl RngExt,
    ) -> bool {
        self.soul_wall_mut()
            .is_some_and(|wall| wall.release(name, width, height, rng))
    }

    pub fn take_souls(&mut self) -> Vec<Fish> {
        self.soul_wall_mut()
            .map(SoulWall::take_all)
            .unwrap_or_default()
    }

    pub fn bubble_color_override(&self) -> Option<Color> {
        match self {
            TankBackground::Alien { bg } => Some(bg.color.bubble_color()),
            TankBackground::Desert { bg } => Some(bg.bubble_color()),
            _ => None,
        }
    }

    pub fn is_night(&self) -> bool {
        match self {
            TankBackground::Desert { bg } => bg.is_night(),
            _ => false,
        }
    }
}

fn extend_plants(plants: &mut Vec<Plant>, to_width: i32, rng: &mut impl RngExt) {
    extend_spaced(
        plants,
        to_width,
        0,
        PLANT_SPACING_MIN..=PLANT_SPACING_MAX,
        rng,
        |p| p.x,
        |x, rng| {
            let height = rng.random_range(PLANT_HEIGHT_MIN..=PLANT_HEIGHT_MAX);
            Plant::new(x, height, rng)
        },
    );
}

fn extend_candy_pink_plants(plants: &mut Vec<Plant>, to_width: i32, rng: &mut impl RngExt) {
    extend_spaced(
        plants,
        to_width,
        0,
        PLANT_SPACING_MIN..=PLANT_SPACING_MAX,
        rng,
        |p| p.x,
        |x, rng| {
            let height = rng.random_range(PLANT_HEIGHT_MIN..=PLANT_HEIGHT_MAX);
            let mut p = Plant::new(x, height, rng);
            p.color = CANDY_PINK_PLANT_COLORS[rng.random_range(0..CANDY_PINK_PLANT_COLORS.len())];
            p
        },
    );
}
