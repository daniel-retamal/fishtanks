use rand::RngExt;

use crate::economy::Rarity;
use crate::tank::TankKind;

use super::Difficulty;
use super::bestiary::{self, EnemySpec, Rank};
use super::fighter::{Fighter, Ghost, Scale};
use super::traits::Trait;

pub const LAST_FLOOR: u32 = 10;
pub const HAUL_CATCHES: u64 = 10;
pub const HARDCORE_PAY: u64 = 2;
pub const BLIND_DEEP_PERCENT: u32 = 125;
pub const CHEST_SHARES: u64 = 1;
pub const BASE_WEIGHT: u32 = 2;
pub const VARIANT_WEIGHT: u32 = 1;
pub const PALETTE_SHARE_DIVISOR: usize = 5;
const PERCENT: u32 = 100;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloorSpec {
    pub depth: u32,
    pub name: &'static str,
    pub tank: TankKind,
    pub tuning: Scale,
    pub crowd: (u8, u8),
    pub light: Light,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Light {
    Bright,
    Dark,
}

pub const CROWD: (u8, u8) = (2, 4);

const fn floor(
    depth: u32,
    name: &'static str,
    tank: TankKind,
    percent: u32,
    light: Light,
) -> FloorSpec {
    FloorSpec {
        depth,
        name,
        tank,
        tuning: Scale {
            hp_percent: percent,
            bite_percent: percent,
        },
        crowd: CROWD,
        light,
    }
}

pub const FLOORS: [FloorSpec; LAST_FLOOR as usize] = [
    floor(1, "The Shallows", TankKind::Base, 70, Light::Bright),
    floor(2, "The Reef", TankKind::CoralReef, 45, Light::Bright),
    floor(3, "The Candy Abyss", TankKind::Candy, 42, Light::Bright),
    floor(4, "The Graveyard", TankKind::Haunted, 65, Light::Dark),
    floor(5, "The Dunes", TankKind::Desert, 50, Light::Bright),
    floor(6, "The Hot Zone", TankKind::Rad, 82, Light::Bright),
    floor(7, "The Mainframe", TankKind::Matrix, 62, Light::Dark),
    floor(8, "The Inferno", TankKind::Hell, 70, Light::Bright),
    floor(9, "The Gate", TankKind::Heaven, 115, Light::Bright),
    floor(10, "The Eye", TankKind::Void, 85, Light::Dark),
];

pub const BLIND_DEEP: &str = "The Blind Deep";

impl FloorSpec {
    pub fn of(depth: u32) -> FloorSpec {
        let index = depth.clamp(1, LAST_FLOOR) as usize - 1;
        FLOORS[index]
    }

    pub fn title(depth: u32) -> String {
        match depth > LAST_FLOOR {
            true => format!("{BLIND_DEEP} {}", depth - LAST_FLOOR),
            false => FloorSpec::of(depth).name.to_string(),
        }
    }

    fn table(self) -> u32 {
        self.depth
    }

    pub fn commons(self) -> Vec<&'static EnemySpec> {
        bestiary::of_floor(self.table())
            .filter(|spec| spec.rank == Rank::Common && !spec.traits.contains(&Trait::Appendage))
            .collect()
    }

    pub fn elites(self) -> Vec<&'static EnemySpec> {
        bestiary::of_floor(self.table())
            .filter(|spec| spec.rank == Rank::Elite)
            .collect()
    }

    pub fn bosses(self) -> Vec<&'static EnemySpec> {
        bestiary::of_floor(self.table())
            .filter(|spec| spec.rank == Rank::Boss && spec.variant_of.is_none())
            .collect()
    }

    pub fn boss(self, dives: u32) -> &'static EnemySpec {
        let bosses = self.bosses();
        bosses[dives as usize % bosses.len()]
    }
}

pub fn palette(depth: u32) -> Vec<u32> {
    let mut counts = [0usize; super::passives::FAMILIES as usize];
    let mut total = 0;
    for spec in bestiary::of_floor(FloorSpec::of(depth).depth) {
        for row in spec.sprite {
            for (index, glyph) in row.text.chars().enumerate() {
                if glyph == ' ' {
                    continue;
                }
                total += 1;
                if let Some(hue) = super::passives::family(row.colour_at(index)) {
                    counts[hue as usize] += 1;
                }
            }
        }
    }
    (0..super::passives::FAMILIES)
        .filter(|&hue| counts[hue as usize] * PALETTE_SHARE_DIVISOR >= total)
        .collect()
}

pub fn haul(floor: u32, difficulty: Difficulty) -> u64 {
    let depth = u64::from(floor.max(1));
    let pay = match difficulty {
        Difficulty::Normal => 1,
        Difficulty::Hardcore => HARDCORE_PAY,
    };
    HAUL_CATCHES * u64::from(Rarity::Common.catch_worth()) * depth * depth * pay
}

pub fn shares_per_floor() -> u64 {
    super::map::expected_shares()
}

pub fn gold_share(floor: u32, difficulty: Difficulty) -> u64 {
    (haul(floor, difficulty) / shares_per_floor()).max(1)
}

pub fn scale_for(floor: u32) -> Scale {
    let tuning = FloorSpec::of(floor).tuning;
    let below = floor.saturating_sub(LAST_FLOOR);
    let deep = (0..below).fold(PERCENT, |percent, _| {
        percent.saturating_mul(BLIND_DEEP_PERCENT) / PERCENT
    });
    Scale {
        hp_percent: tuning.hp_percent * deep / PERCENT,
        bite_percent: tuning.bite_percent * deep / PERCENT,
    }
}

fn weight(spec: &EnemySpec) -> u32 {
    match spec.variant_of {
        Some(_) => VARIANT_WEIGHT,
        None => BASE_WEIGHT,
    }
}

fn haunted(spec: &EnemySpec) -> Option<super::traits::Haunt> {
    spec.traits.iter().find_map(|t| match t {
        Trait::Yours(haunt) => Some(*haunt),
        _ => None,
    })
}

fn draw(pool: &[&'static EnemySpec], rng: &mut impl RngExt) -> &'static EnemySpec {
    let total: u32 = pool.iter().map(|spec| weight(spec)).sum();
    let mut roll = rng.random_range(0..total);
    for spec in pool {
        if roll < weight(spec) {
            return spec;
        }
        roll -= weight(spec);
    }
    pool[0]
}

fn fighter_for(
    spec: &'static EnemySpec,
    depth: u32,
    scale: Scale,
    haunts: &[Ghost],
    rng: &mut impl RngExt,
) -> Option<Fighter> {
    let Some(haunt) = haunted(spec) else {
        return Some(Fighter::of_enemy(spec, depth, scale));
    };
    let ghosts: Vec<&Ghost> = haunts.iter().filter(|ghost| ghost.haunt == haunt).collect();
    if ghosts.is_empty() {
        return None;
    }
    let ghost = ghosts[rng.random_range(0..ghosts.len())];
    Some(Fighter::of_ghost(spec, ghost, depth, scale))
}

pub fn encounter(
    depth: u32,
    scale: Scale,
    haunts: &[Ghost],
    rng: &mut impl RngExt,
) -> Vec<Fighter> {
    let spec = FloorSpec::of(depth);
    let pool = spec.commons();
    let (least, most) = spec.crowd;
    let size = rng.random_range(least..=most);
    let mut foes = Vec::with_capacity(usize::from(size));
    while foes.len() < usize::from(size) {
        if let Some(foe) = fighter_for(draw(&pool, rng), depth, scale, haunts, rng) {
            foes.push(foe);
        }
    }
    foes
}

pub fn den(depth: u32, scale: Scale, rng: &mut impl RngExt) -> Vec<Fighter> {
    let elites = FloorSpec::of(depth).elites();
    vec![Fighter::of_enemy(draw(&elites, rng), depth, scale)]
}

pub fn lair(depth: u32, scale: Scale, dives: u32) -> Vec<Fighter> {
    vec![Fighter::of_enemy(
        FloorSpec::of(depth).boss(dives),
        depth,
        scale,
    )]
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    #[test]
    fn a_floor_pays_by_the_square_of_its_depth() {
        let first = haul(1, Difficulty::Normal);
        assert_eq!(
            first,
            HAUL_CATCHES * u64::from(Rarity::Common.catch_worth())
        );
        assert_eq!(haul(3, Difficulty::Normal), first * 9);
        assert_eq!(haul(10, Difficulty::Normal), first * 100);
        assert_eq!(
            haul(4, Difficulty::Hardcore),
            haul(4, Difficulty::Normal) * HARDCORE_PAY
        );
    }

    #[test]
    fn a_cleared_floor_pays_its_haul_in_shares() {
        for depth in 1..=LAST_FLOOR {
            let paid = gold_share(depth, Difficulty::Normal) * shares_per_floor();
            let haul = haul(depth, Difficulty::Normal);
            assert!(haul - paid < shares_per_floor(), "floor {depth}");
        }
    }

    #[test]
    fn the_blind_deep_grows_a_quarter_a_floor() {
        let eye = FloorSpec::of(LAST_FLOOR).tuning;
        assert_eq!(scale_for(LAST_FLOOR), eye);
        assert_eq!(
            scale_for(LAST_FLOOR + 1).hp_percent,
            eye.hp_percent * BLIND_DEEP_PERCENT / PERCENT
        );
        let two_floors = BLIND_DEEP_PERCENT * BLIND_DEEP_PERCENT / PERCENT;
        assert_eq!(
            scale_for(LAST_FLOOR + 2).bite_percent,
            eye.bite_percent * two_floors / PERCENT
        );
        assert_eq!(FloorSpec::title(LAST_FLOOR + 3), "The Blind Deep 3");
    }

    #[test]
    fn every_floor_draws_from_its_own_bestiary() {
        let mut rng = SmallRng::seed_from_u64(4);
        for depth in 1..=LAST_FLOOR + 2 {
            let spec = FloorSpec::of(depth);
            assert!(!spec.commons().is_empty());
            assert_eq!(spec.elites().len(), 2, "{}", spec.name);
            assert!(!spec.bosses().is_empty());
            let foes = encounter(depth, scale_for(depth), &[], &mut rng);
            let (least, most) = spec.crowd;
            assert!((usize::from(least)..=usize::from(most)).contains(&foes.len()));
            for foe in &foes {
                let enemy = foe.spec().expect("an enemy");
                assert_eq!(enemy.floor, spec.depth);
                assert!(!enemy.traits.contains(&Trait::Appendage));
            }
        }
    }

    #[test]
    fn the_two_bosses_take_turns_guarding_the_elevator() {
        for depth in 1..LAST_FLOOR {
            let spec = FloorSpec::of(depth);
            assert_ne!(spec.boss(0).name, spec.boss(1).name, "{}", spec.name);
            assert_eq!(spec.boss(0).name, spec.boss(2).name, "{}", spec.name);
        }
        let eye = FloorSpec::of(LAST_FLOOR);
        assert_eq!(eye.boss(0).name, eye.boss(1).name);
    }

    #[test]
    fn your_own_dead_haunt_a_floor_only_when_you_have_some() {
        let mut rng = SmallRng::seed_from_u64(5);
        for _ in 0..50 {
            for foe in encounter(4, scale_for(4), &[], &mut rng) {
                assert!(!foe.has(Trait::Yours(super::super::traits::Haunt::Dead)));
            }
        }
    }
}
