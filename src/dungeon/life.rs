use serde::{Deserialize, Serialize};

use super::boon::{Boon, Charm, Curse};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Grade {
    Fry,
    Fingerling,
    Juvenile,
    Adult,
    Elder,
    Ancient,
    Leviathan,
}

impl Grade {
    pub const ALL: [Grade; 7] = [
        Grade::Fry,
        Grade::Fingerling,
        Grade::Juvenile,
        Grade::Adult,
        Grade::Elder,
        Grade::Ancient,
        Grade::Leviathan,
    ];

    pub fn floors_needed(self) -> u32 {
        match self {
            Grade::Fry => 0,
            Grade::Fingerling => 1,
            Grade::Juvenile => 3,
            Grade::Adult => 6,
            Grade::Elder => 10,
            Grade::Ancient => 15,
            Grade::Leviathan => 21,
        }
    }

    pub fn of(floors_survived: u32) -> Grade {
        Grade::ALL
            .into_iter()
            .rev()
            .find(|grade| floors_survived >= grade.floors_needed())
            .unwrap_or(Grade::Fry)
    }

    pub fn step(self) -> u32 {
        self as u32
    }

    pub fn name(self) -> &'static str {
        match self {
            Grade::Fry => "Fry",
            Grade::Fingerling => "Fingerling",
            Grade::Juvenile => "Juvenile",
            Grade::Adult => "Adult",
            Grade::Elder => "Elder",
            Grade::Ancient => "Ancient",
            Grade::Leviathan => "Leviathan",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeepLife {
    #[serde(default)]
    pub floors_survived: u32,
    #[serde(default)]
    pub dives: u32,
    #[serde(default)]
    pub deepest: u32,
    #[serde(default)]
    pub beaten: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boons: Vec<Boon>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub curses: Vec<Curse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charm: Option<Charm>,
    #[serde(default)]
    pub lives_spent: u32,
}

impl DeepLife {
    pub fn grade(&self) -> Grade {
        Grade::of(self.floors_survived)
    }

    pub fn wipe_marks(&mut self) {
        self.boons.clear();
        self.curses.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grade_is_earned_by_floors_survived() {
        assert_eq!(Grade::of(0), Grade::Fry);
        assert_eq!(Grade::of(1), Grade::Fingerling);
        assert_eq!(Grade::of(2), Grade::Fingerling);
        assert_eq!(Grade::of(3), Grade::Juvenile);
        assert_eq!(Grade::of(6), Grade::Adult);
        assert_eq!(Grade::of(10), Grade::Elder);
        assert_eq!(Grade::of(15), Grade::Ancient);
        assert_eq!(Grade::of(21), Grade::Leviathan);
        assert_eq!(Grade::of(1_000), Grade::Leviathan);
    }

    #[test]
    fn a_restore_wipes_marks_and_keeps_the_grade_and_the_charm() {
        let mut life = DeepLife {
            floors_survived: 11,
            boons: vec![Boon::SecondWind],
            curses: vec![Curse::HungryGhost],
            charm: Some(Charm::CaleuchesLantern),
            ..DeepLife::default()
        };
        life.wipe_marks();
        assert!(life.boons.is_empty());
        assert!(life.curses.is_empty());
        assert_eq!(life.grade(), Grade::Elder);
        assert_eq!(life.charm, Some(Charm::CaleuchesLantern));
    }
}
