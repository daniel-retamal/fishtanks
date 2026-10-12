use crate::economy::Rarity;
use crate::fishes::fish::{Fed, Fish};
use crate::fishes::mutations::Mutation;
use crate::fishes::species::SizeCategory;

use super::life::{DeepLife, Grade};
use super::skill::Skill;
use super::tables;
use super::traits::Trait;

pub const VIGOR_PER_DOUBLING: f64 = 10.0;
pub const VIGOR_REFERENCE_G: f64 = 25.0;
pub const MIN_VIGOR: u32 = 10;
pub const UNFISH_VIGOR: u32 = 45;
pub const GRADE_VIGOR_PERCENT: u32 = 10;
pub const BASE_BITE: u32 = 4;
pub const BITE_PER_SIZE_STEP: u32 = 2;
pub const GRADE_BITE: u32 = 1;
pub const CRIT_PER_EYE: u32 = 5;
pub const BASE_MP: u32 = 3;
pub const START_BUBBLES: u32 = 1;
pub const HUNGRY_BELOW_PERCENT: f64 = 25.0;
pub const HUNGRY_BITE: u32 = 1;
pub const MIN_SPEED: u32 = 1;
const PERCENT: u32 = 100;
const SPEED_SCALE: f32 = 2.0;
const SIZE_DODGE: [u32; 4] = [20, 12, 6, 0];
const MP_GRADE: Grade = Grade::Adult;

#[derive(Clone, Debug, PartialEq)]
pub struct Kit {
    pub name: String,
    pub vigor: u32,
    pub bite: u32,
    pub speed: u32,
    pub dodge: u32,
    pub crit: u32,
    pub max_mp: u32,
    pub start_mp: u32,
    pub armour: u32,
    pub grade: Grade,
    pub hungry: bool,
    pub full: bool,
    pub traits: Vec<Trait>,
    pub skills: Vec<Skill>,
}

pub fn rarity_bite(rarity: Rarity) -> u32 {
    match rarity {
        Rarity::Common => 0,
        Rarity::Rare => 2,
        Rarity::Legendary => 5,
    }
}

pub fn size_step(size: SizeCategory) -> u32 {
    size as u32 + 1
}

pub fn vigor_of_weight(weight_g: u32) -> u32 {
    let doublings = (f64::from(weight_g.max(1)) / VIGOR_REFERENCE_G).log2();
    ((VIGOR_PER_DOUBLING * doublings).round().max(0.0) as u32).max(MIN_VIGOR)
}

pub fn carried_mutations(fish: &Fish) -> Vec<Mutation> {
    let born = fish.species.config().born_with.iter().copied();
    let worn = fish
        .mutations
        .as_ref()
        .map(|record| record.history.clone())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|token| Mutation::parse(&token));
    born.chain(worn).collect()
}

impl Kit {
    pub fn of(fish: &Fish) -> Kit {
        let life = fish.deep.as_deref().cloned().unwrap_or_default();
        let mut kit = Kit::base(fish, &life);
        tables::fit(&mut kit, fish, &life);
        kit
    }

    fn base(fish: &Fish, life: &DeepLife) -> Kit {
        let config = fish.species.config();
        let grade = life.grade();
        let raw_vigor = match (fish.toy.as_deref(), fish.unfish_state.is_some()) {
            (Some(toy), _) => tables::toys::material_vigor(toy.material()),
            (None, true) => UNFISH_VIGOR,
            (None, false) => vigor_of_weight(fish.weight_g),
        };
        let vigor = raw_vigor * (PERCENT + GRADE_VIGOR_PERCENT * grade.step()) / PERCENT;
        let fed = fish.fed();
        let hungry = fed
            .percent()
            .is_some_and(|percent| percent < HUNGRY_BELOW_PERCENT);
        let full = matches!(fed, Fed::Full | Fed::Boundless);
        let bite = (BASE_BITE
            + BITE_PER_SIZE_STEP * size_step(fish.size_category)
            + rarity_bite(config.rarity)
            + GRADE_BITE * grade.step())
        .saturating_sub(if hungry { HUNGRY_BITE } else { 0 });
        let (lo, hi) = config.speed_range;
        let speed = ((((lo + hi) / 2.0) * SPEED_SCALE).round() as u32).max(MIN_SPEED);
        let dodge = SIZE_DODGE[fish.size_category as usize];
        let crit = CRIT_PER_EYE * fish.eyes_a_side() as u32;
        let max_mp = BASE_MP + u32::from(grade >= MP_GRADE);
        let start_mp = START_BUBBLES + u32::from(full);
        Kit {
            name: fish.name.clone(),
            vigor,
            bite,
            speed,
            dodge,
            crit,
            max_mp,
            start_mp,
            armour: 0,
            grade,
            hungry,
            full,
            traits: Vec::new(),
            skills: Vec::new(),
        }
    }

    pub fn has(&self, wanted: Trait) -> bool {
        self.traits.contains(&wanted)
    }

    pub fn add(&mut self, a_trait: Trait) {
        self.traits.push(a_trait);
    }

    pub fn settle(&mut self) {
        let mut traits: Vec<Trait> = Vec::with_capacity(self.traits.len());
        for a_trait in self.traits.drain(..) {
            if !traits.contains(&a_trait) {
                traits.push(a_trait);
            }
        }
        self.traits = traits;
        let mut skills: Vec<Skill> = Vec::with_capacity(self.skills.len());
        for skill in self.skills.drain(..) {
            if !skills.iter().any(|known| known.name == skill.name) {
                skills.push(skill);
            }
        }
        self.skills = skills;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fishes::species::FishSpecies;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    fn fish(species: FishSpecies) -> Fish {
        let mut rng = SmallRng::seed_from_u64(7);
        Fish::new(species, "Probe".to_string(), 0.0, 0.0, &mut rng)
    }

    #[test]
    fn vigor_grows_with_the_log_of_weight() {
        assert_eq!(vigor_of_weight(100), 20);
        assert_eq!(vigor_of_weight(2_500), 66);
        assert_eq!(vigor_of_weight(40_000), 106);
        assert_eq!(vigor_of_weight(1_000_000), 153);
        assert_eq!(vigor_of_weight(1), MIN_VIGOR);
    }

    #[test]
    fn speed_is_twice_the_middle_of_the_swim_speed() {
        assert_eq!(Kit::of(&fish(FishSpecies::Anchoveta)).speed, 12);
        assert_eq!(Kit::of(&fish(FishSpecies::Salmon)).speed, 10);
        assert_eq!(Kit::of(&fish(FishSpecies::Holyfish)).speed, 5);
        assert_eq!(Kit::of(&fish(FishSpecies::Leaffish)).speed, 2);
    }

    #[test]
    fn crit_is_five_percent_an_eye_a_side() {
        assert_eq!(Kit::of(&fish(FishSpecies::Merluza)).crit, 5);
        assert_eq!(Kit::of(&fish(FishSpecies::Lenguado)).crit, 10);
        assert_eq!(Kit::of(&fish(FishSpecies::Cheatfish)).crit, 15);
    }

    #[test]
    fn bite_reads_size_rarity_and_grade() {
        let mut salmon = fish(FishSpecies::Salmon);
        salmon.size_category = SizeCategory::L;
        salmon.weight_g = salmon.species.config().weight_cap[SizeCategory::L as usize];
        assert_eq!(Kit::of(&salmon).bite, BASE_BITE + BITE_PER_SIZE_STEP * 3);
        salmon.deep = Some(Box::new(DeepLife {
            floors_survived: Grade::Adult.floors_needed(),
            ..DeepLife::default()
        }));
        assert_eq!(
            Kit::of(&salmon).bite,
            BASE_BITE + BITE_PER_SIZE_STEP * 3 + 3
        );
    }

    #[test]
    fn small_fish_dodge_and_big_fish_do_not() {
        let mut probe = fish(FishSpecies::Merluza);
        probe.size_category = SizeCategory::S;
        assert_eq!(Kit::of(&probe).dodge, 20);
        probe.size_category = SizeCategory::XL;
        assert_eq!(Kit::of(&probe).dodge, 0);
    }

    #[test]
    fn an_adult_has_one_more_bubble_and_a_grade_more_vigor() {
        let mut probe = fish(FishSpecies::Merluza);
        probe.weight_g = 2_500;
        let fry = Kit::of(&probe);
        probe.deep = Some(Box::new(DeepLife {
            floors_survived: Grade::Adult.floors_needed(),
            ..DeepLife::default()
        }));
        let adult = Kit::of(&probe);
        assert_eq!(adult.max_mp, fry.max_mp + 1);
        assert_eq!(adult.vigor, fry.vigor * 130 / 100);
    }

    #[test]
    fn a_hungry_fish_bites_softer_and_a_full_one_starts_with_a_bubble() {
        let mut probe = fish(FishSpecies::Merluza);
        probe.size_category = SizeCategory::M;
        probe.weight_g = probe.species.config().weight_base[SizeCategory::M as usize];
        let hungry = Kit::of(&probe);
        assert!(hungry.hungry);
        probe.weight_g = probe.species.config().weight_cap[SizeCategory::M as usize];
        let full = Kit::of(&probe);
        assert!(full.full);
        assert_eq!(full.bite, hungry.bite + HUNGRY_BITE);
        assert_eq!(full.start_mp, hungry.start_mp + 1);
    }
}
