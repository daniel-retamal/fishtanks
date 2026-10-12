use super::bestiary::{EnemySpec, Rank};
use super::effect::{Status, StatusKind};
use super::kit::Kit;
use super::skill::Skill;
use super::traits::{Haunt, Trait};

pub const ENEMY_START_BUBBLES: u32 = 1;
const SCALE_PERCENT: u32 = 100;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ghost {
    pub name: String,
    pub haunt: Haunt,
    pub vigor: u32,
    pub bite: u32,
    pub speed: u32,
}

impl Ghost {
    pub fn of(fish: &crate::fishes::fish::Fish, haunt: Haunt) -> Ghost {
        let kit = Kit::of(fish);
        Ghost {
            name: fish.name.clone(),
            haunt,
            vigor: kit.vigor,
            bite: kit.bite,
            speed: kit.speed,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    Party,
    Foes,
}

impl Side {
    pub fn other(self) -> Side {
        match self {
            Side::Party => Side::Foes,
            Side::Foes => Side::Party,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Origin {
    Fish { member: usize },
    Enemy { spec: &'static EnemySpec },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fighter {
    pub name: String,
    pub side: Side,
    pub origin: Origin,
    pub rank: Option<Rank>,
    pub hp: u32,
    pub max_hp: u32,
    pub mp: u32,
    pub max_mp: u32,
    pub bite: u32,
    pub speed: u32,
    pub dodge: u32,
    pub crit: u32,
    pub armour: u32,
    pub traits: Vec<Trait>,
    pub skills: Vec<Skill>,
    pub spent: Vec<&'static str>,
    pub statuses: Vec<Status>,
    pub shield: u32,
    pub rolling: u32,
    pub guarding: bool,
    pub belly_up: bool,
    pub gone: bool,
    pub ready_at: u64,
    pub turns: u32,
    pub hits_taken: u32,
    pub came_back: bool,
    pub tell: Option<usize>,
    pub learned: Option<&'static str>,
    pub last_hp: u32,
    pub eaten_gold: u64,
    pub stolen: Vec<usize>,
    pub shares: u64,
    pub parts: u32,
    pub phase: u8,
    pub body: Option<usize>,
    pub summoned_by: Option<usize>,
    pub damage_taken: u32,
    pub scale: Scale,
    pub once: Vec<Trait>,
    pub extra: u8,
    pub lap: Vec<u32>,
    pub gathered: u32,
    pub hid_in_round: u32,
}

impl Fighter {
    pub fn of_kit(kit: &Kit, member: usize) -> Fighter {
        Fighter {
            name: kit.name.clone(),
            side: Side::Party,
            origin: Origin::Fish { member },
            rank: None,
            hp: kit.vigor,
            max_hp: kit.vigor,
            mp: kit.start_mp.min(kit.max_mp),
            max_mp: kit.max_mp,
            bite: kit.bite,
            speed: kit.speed,
            dodge: kit.dodge,
            crit: kit.crit,
            armour: kit.armour,
            traits: kit.traits.clone(),
            skills: kit.skills.clone(),
            spent: Vec::new(),
            statuses: Vec::new(),
            shield: 0,
            rolling: 0,
            guarding: false,
            belly_up: false,
            gone: false,
            ready_at: 0,
            turns: 0,
            hits_taken: 0,
            came_back: false,
            tell: None,
            learned: None,
            last_hp: kit.vigor,
            eaten_gold: 0,
            stolen: Vec::new(),
            shares: 0,
            parts: 0,
            phase: 0,
            body: None,
            summoned_by: None,
            damage_taken: 0,
            scale: Scale::PLAIN,
            once: Vec::new(),
            extra: 0,
            lap: Vec::new(),
            gathered: 0,
            hid_in_round: 0,
        }
    }

    pub fn of_enemy(spec: &'static EnemySpec, floor: u32, scale: Scale) -> Fighter {
        let (base_hp, base_bite) = scaled_stats(spec, floor);
        let hp = scale.hp(base_hp).max(1);
        Fighter {
            name: spec.name.to_string(),
            side: Side::Foes,
            origin: Origin::Enemy { spec },
            rank: Some(spec.rank),
            hp,
            max_hp: hp,
            mp: ENEMY_START_BUBBLES.min(spec.mp),
            max_mp: spec.mp,
            bite: scale.bite(base_bite),
            speed: spec.speed.max(1),
            dodge: 0,
            crit: super::kit::CRIT_PER_EYE,
            armour: armour_of(spec.traits),
            traits: spec.traits.to_vec(),
            skills: spec.skills.iter().map(|skill| skill.skill()).collect(),
            spent: Vec::new(),
            statuses: Vec::new(),
            shield: 0,
            rolling: 0,
            guarding: false,
            belly_up: false,
            gone: false,
            ready_at: 0,
            turns: 0,
            hits_taken: 0,
            came_back: false,
            tell: None,
            learned: None,
            last_hp: hp,
            eaten_gold: 0,
            stolen: Vec::new(),
            shares: spec.rank.shares(),
            parts: 0,
            phase: 0,
            body: None,
            summoned_by: None,
            damage_taken: 0,
            scale,
            once: Vec::new(),
            extra: 0,
            lap: Vec::new(),
            gathered: 0,
            hid_in_round: 0,
        }
    }

    pub fn of_ghost(spec: &'static EnemySpec, ghost: &Ghost, floor: u32, scale: Scale) -> Fighter {
        let mut fighter = Fighter::of_enemy(spec, floor, scale);
        let hp = scale.hp(ghost.vigor).max(1);
        fighter.name = format!("{} {}", spec.name, ghost.name);
        fighter.hp = hp;
        fighter.max_hp = hp;
        fighter.last_hp = hp;
        fighter.bite = scale.bite(ghost.bite);
        fighter.speed = ghost.speed.max(1);
        fighter
    }

    pub fn spec(&self) -> Option<&'static EnemySpec> {
        match self.origin {
            Origin::Enemy { spec } => Some(spec),
            Origin::Fish { .. } => None,
        }
    }

    pub fn member(&self) -> Option<usize> {
        match self.origin {
            Origin::Fish { member } => Some(member),
            Origin::Enemy { .. } => None,
        }
    }

    pub fn is_boss(&self) -> bool {
        self.rank == Some(Rank::Boss)
    }

    pub fn in_the_fight(&self) -> bool {
        !self.gone
    }

    pub fn standing(&self) -> bool {
        !self.gone && !self.belly_up && !self.has_status(StatusKind::Away)
    }

    pub fn has(&self, wanted: Trait) -> bool {
        self.traits.contains(&wanted)
    }

    pub fn trait_value(&self, pick: impl Fn(&Trait) -> Option<u32>) -> Option<u32> {
        self.traits.iter().find_map(pick)
    }

    pub fn has_status(&self, kind: StatusKind) -> bool {
        self.statuses.iter().any(|status| status.kind == kind)
    }

    pub fn status(&self, kind: StatusKind) -> Option<&Status> {
        self.statuses.iter().find(|status| status.kind == kind)
    }

    pub fn status_amount(&self, kind: StatusKind) -> u32 {
        self.statuses
            .iter()
            .filter(|status| status.kind == kind)
            .map(|status| status.amount)
            .sum()
    }

    pub fn clear_status(&mut self, kind: StatusKind) {
        self.statuses.retain(|status| status.kind != kind);
    }

    pub fn inflict(&mut self, status: Status) {
        if let Some(existing) = self.statuses.iter_mut().find(|s| s.kind == status.kind) {
            existing.turns = existing.turns.max(status.turns);
            existing.amount = existing.amount.max(status.amount);
            return;
        }
        self.statuses.push(status);
    }

    pub fn hp_percent(&self) -> u32 {
        if self.max_hp == 0 {
            return 0;
        }
        self.hp * 100 / self.max_hp
    }

    pub fn effective_bite(&self) -> u32 {
        let up = self.status_amount(StatusKind::BiteUp);
        let down = self.status_amount(StatusKind::BiteDown);
        (self.bite + up).saturating_sub(down)
    }

    pub fn gain_bubbles(&mut self, amount: u32) {
        self.mp = (self.mp + amount).min(self.max_mp);
    }

    pub fn lose_bubbles(&mut self, amount: u32) {
        self.mp = self.mp.saturating_sub(amount);
    }

    pub fn heal(&mut self, amount: u32) -> u32 {
        self.rolling = 0;
        let before = self.hp;
        self.hp = (self.hp + amount).min(self.max_hp);
        self.hp - before
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scale {
    pub hp_percent: u32,
    pub bite_percent: u32,
}

impl Scale {
    pub const PLAIN: Scale = Scale {
        hp_percent: 100,
        bite_percent: 100,
    };

    pub fn hp(self, base: u32) -> u32 {
        scaled(base, self.hp_percent)
    }

    pub fn bite(self, base: u32) -> u32 {
        scaled(base, self.bite_percent)
    }
}

fn scaled(base: u32, percent: u32) -> u32 {
    match base {
        0 => 0,
        n => (n * percent / SCALE_PERCENT).max(1),
    }
}

fn armour_of(traits: &[Trait]) -> u32 {
    traits
        .iter()
        .map(|t| match t {
            Trait::Armour(n) | Trait::Wrapped(n) => *n,
            _ => 0,
        })
        .sum()
}

pub fn scaled_stats(spec: &EnemySpec, floor: u32) -> (u32, u32) {
    spec.traits
        .iter()
        .find_map(|t| match t {
            Trait::Scaled {
                hp,
                hp_per_floor,
                bite,
                bite_per_floor,
            } => Some((hp + hp_per_floor * floor, bite + bite_per_floor * floor)),
            _ => None,
        })
        .unwrap_or((spec.hp, spec.bite))
}
