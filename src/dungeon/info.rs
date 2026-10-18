use super::battle::{Battle, CRIT_TIMES, HARSH_CRIT_TIMES};
use super::effect::{DamageKind, Effect, Power, Reach, StatusKind};
use super::fighter::Side;
use super::items::{Aim, Pack};
use super::skill::{Price, Skill, cost_label};
use super::traits::Trait;

pub const UNKNOWN: &str = "???";
const DEVIL_EXTRA_PERCENT: u32 = super::battle::DEVIL_PERCENT - PERCENT;
const PERCENT: u32 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    Plain,
    Quiet,
    Bubbles,
    Gold,
    Warning,
    Danger,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfoRow {
    pub label: &'static str,
    pub value: String,
    pub ink: Ink,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Info {
    pub title: String,
    pub about: String,
    pub rows: Vec<InfoRow>,
}

impl Info {
    fn new(title: impl Into<String>, about: impl Into<String>) -> Info {
        Info {
            title: title.into(),
            about: about.into(),
            rows: Vec::new(),
        }
    }

    fn row(mut self, label: &'static str, value: impl Into<String>, ink: Ink) -> Info {
        self.rows.push(InfoRow {
            label,
            value: value.into(),
            ink,
        });
        self
    }

    fn cost(self, price: Price) -> Info {
        let ink = match price {
            Price::Bubbles(_) => Ink::Bubbles,
            Price::Purse { .. } => Ink::Gold,
            Price::Free => Ink::Quiet,
        };
        self.row("Cost", cost_label(price), ink)
    }

    pub fn bite(battle: &Battle, actor: usize, target: Option<usize>) -> Info {
        let name = &battle.fighters[actor].name;
        let (hits, power) = super::battle::bite_hits(&battle.fighters[actor]);
        let reach =
            match battle.fighters[actor].has(Trait::Signature(crate::fishes::toy::Line::Kaiju)) {
                true => Reach::AllFoes,
                false => Reach::OneFoe,
            };
        let about = match reach {
            Reach::AllFoes => format!("{name} stomps on every enemy."),
            _ => format!("{name} bites one enemy."),
        };
        Info::new(super::battle::BITE, about)
            .cost(Price::Free)
            .damage(battle, actor, power, u32::from(hits))
            .target(battle, reach, target)
            .row("Accuracy", percent(battle.accuracy(actor)), Ink::Plain)
            .crit(battle, actor, target)
            .modifiers(battle, actor, target, DamageKind::Bite)
    }

    pub fn skill(battle: &Battle, actor: usize, skill: &Skill, target: Option<usize>) -> Info {
        let mut info = Info::new(skill.name, skill.about.clone()).cost(skill.price);
        if skill.once_a_fight {
            info = info.row("Uses", "once a fight", Ink::Warning);
        }
        if random(skill) {
            info = info.row("Effect", UNKNOWN, Ink::Plain);
        }
        let hit = skill.effects.iter().find_map(|effect| match effect {
            Effect::Hit { power, kind, hits } => Some((*power, *kind, *hits)),
            _ => None,
        });
        if let Some((power, _, hits)) = hit {
            info = info.damage(battle, actor, power, u32::from(hits));
        }
        let special = skill.effects.iter().find_map(|effect| match effect {
            Effect::Special(special) => super::specials::strikes(battle, actor, *special),
            _ => None,
        });
        if let Some((power, hits)) = special {
            info = info.damage(battle, actor, power, hits);
        }
        if let Some(amount) = skill.effects.iter().find_map(|effect| match effect {
            Effect::Heal(amount) => Some(battle.fighters[actor].scale.hp(*amount).to_string()),
            Effect::HealPercent(percent) => Some(format!("{percent}% of its HP")),
            _ => None,
        }) {
            info = info.row("Heals", amount, Ink::Plain);
        }
        if let Some(amount) = skill.effects.iter().find_map(|effect| match effect {
            Effect::Drain(amount) => Some(battle.fighters[actor].scale.bite(*amount)),
            _ => None,
        }) {
            info = info.row("Drains", amount.to_string(), Ink::Plain);
        }
        info = info.target(battle, skill.reach, target);
        if skill.deals_damage() {
            let accuracy = match skill.accuracy >= super::skill::SURE_ACCURACY {
                true => skill.accuracy,
                false => battle.accuracy(actor),
            };
            info = info.row("Accuracy", percent(accuracy), Ink::Plain);
        }
        let kind = match (hit, special) {
            (Some((_, kind, _)), _) => kind,
            (None, Some((Power::Bite { .. }, _))) => DamageKind::Bite,
            _ => return info,
        };
        info.crit(battle, actor, target)
            .modifiers(battle, actor, target, kind)
    }

    pub fn guard(battle: &Battle, actor: usize) -> Info {
        let name = &battle.fighters[actor].name;
        let mut info = Info::new("Guard", format!("{name} braces until its next turn."))
            .cost(Price::Free)
            .row("Effect", "half damage taken", Ink::Plain)
            .row("Rolling", "stops", Ink::Plain)
            .row(
                "MP",
                format!("+{} now", super::skill::FULL_BUBBLE),
                Ink::Bubbles,
            );
        if battle.fighters[actor].has(Trait::Pride) || battle.fighters[actor].has(Trait::PrideSin) {
            info = info.row("Pride", format!("{name} will not guard"), Ink::Danger);
        }
        info
    }

    pub fn pray(battle: &Battle) -> Info {
        let prayer = &battle.prayer;
        Info::new(
            "Pray",
            "Say the line it waits for. While it listens, it can be hurt.",
        )
        .cost(Price::Free)
        .row("Turn", "kept", Ink::Plain)
        .row(
            "Line",
            format!("{} of {}", prayer.said + 1, prayer.lines.len()),
            Ink::Plain,
        )
    }

    pub fn flee(battle: &Battle) -> Info {
        match battle.can_flee() {
            true => Info::new("Flee", "The party tries to swim away from this fight.")
                .row("Chance", "the party's fastest against theirs", Ink::Plain)
                .row("If it fails", "the turn is spent", Ink::Warning),
            false => Info::new("Flee", "This room has no way out until the fight ends."),
        }
    }

    pub fn skills(battle: &Battle, actor: usize) -> Info {
        let name = &battle.fighters[actor].name;
        let skills = &battle.fighters[actor].skills;
        let about = match skills.is_empty() {
            true => format!("{name} has nothing to do but bite."),
            false => format!("What {name} can do besides biting."),
        };
        skills
            .iter()
            .fold(Info::new("Skills", about), |info, skill| {
                info.row(skill.name, cost_label(skill.price), price_ink(skill.price))
            })
    }

    pub fn items(battle: &Battle) -> Info {
        let packed: Vec<&Pack> = battle.items.iter().filter(|pack| pack.count > 0).collect();
        let about = match packed.is_empty() {
            true => "Nothing was packed for this dive.",
            false => "From the Items you packed.",
        };
        packed.iter().fold(Info::new("Items", about), |info, pack| {
            info.row(pack.item.name(), format!("×{}", pack.count), Ink::Plain)
        })
    }

    pub fn item(battle: &Battle, pack: &Pack, target: Option<usize>) -> Info {
        let aim = match pack.item.aim() {
            Aim::Ally => Reach::OneAlly,
            Aim::Foe => Reach::OneFoe,
            Aim::Nobody => Reach::Everyone,
        };
        Info::new(pack.item.name(), pack.item.about())
            .target(battle, aim, target)
            .row("Left", format!("×{}", pack.count), Ink::Plain)
    }

    pub fn offer(battle: &Battle, devil: usize) -> Info {
        let name = &battle.fighters[devil].name;
        Info::new(
            "Contract",
            format!("{name} offers a contract: sign, and its HP and your haul both double."),
        )
        .row("Sign", "its HP doubles, the haul doubles", Ink::Gold)
        .row("Refuse", "the fight goes on", Ink::Plain)
    }

    fn damage(self, battle: &Battle, actor: usize, power: Power, hits: u32) -> Info {
        let (low, high) = battle.damage_range(actor, power);
        let each = match low == high {
            true => low.to_string(),
            false => format!("{low} to {high}"),
        };
        let value = match hits {
            0 | 1 => each,
            n => format!("{each} a hit, {n} hits"),
        };
        self.row("Damage", value, Ink::Plain)
    }

    fn target(self, battle: &Battle, reach: Reach, target: Option<usize>) -> Info {
        let value = match target {
            Some(target) => battle.fighters[target].name.clone(),
            None => reach.label().to_string(),
        };
        self.row("Target", value, Ink::Plain)
    }

    fn crit(self, battle: &Battle, actor: usize, target: Option<usize>) -> Info {
        let (chance, times) = battle.crit_odds(actor, target);
        let how = match times {
            CRIT_TIMES => "double damage",
            HARSH_CRIT_TIMES => "triple damage",
            _ => "more damage",
        };
        self.row("Crit", format!("{}, {how}", percent(chance)), Ink::Plain)
    }

    fn modifiers(
        mut self,
        battle: &Battle,
        actor: usize,
        target: Option<usize>,
        kind: DamageKind,
    ) -> Info {
        let attacker = &battle.fighters[actor];
        let sees = attacker.has(Trait::SeesGhosts);
        if kind.halved_by_ether() && !sees {
            match target {
                Some(t) if battle.fighters[t].has(Trait::Ethereal) => {
                    let name = &battle.fighters[t].name;
                    self = self.row("Ethereal", format!("{name} takes half"), Ink::Warning);
                }
                None if foes_of(battle, actor).any(|i| battle.fighters[i].has(Trait::Ethereal)) => {
                    self = self.row("Ethereal", "ghosts take half", Ink::Warning);
                }
                _ => {}
            }
        }
        if battle.is_marked(actor) && attacker.has(Trait::DevilMarked) {
            let name = &attacker.name;
            self = self.row(
                "Damned",
                format!("+{DEVIL_EXTRA_PERCENT}% ({name} is marked)"),
                Ink::Danger,
            );
        }
        let Some(t) = target else {
            return self;
        };
        let foe = &battle.fighters[t];
        if foe.side != attacker.side && super::reveal::assayed(battle) {
            let (low, high) = battle.damage_range(t, Power::BITE);
            self = self
                .row("Its Bite", format!("{low} to {high}"), Ink::Quiet)
                .row("Its Speed", foe.speed.to_string(), Ink::Quiet);
        }
        if foe.guarding && kind.guarded() {
            self = self.row("Guarding", format!("{} takes half", foe.name), Ink::Warning);
        }
        if foe.shield > 0 && kind.shielded() {
            self = self.row(
                "Shield",
                format!("{} absorbs {}", foe.name, foe.shield),
                Ink::Warning,
            );
        }
        if foe.armour > 0 && kind.armoured() {
            self = self.row("Armour", format!("{} less a hit", foe.armour), Ink::Warning);
        }
        if foe.has_status(StatusKind::Contract) {
            self = self.row("Contract", "it takes more", Ink::Gold);
        }
        self
    }
}

fn foes_of(battle: &Battle, actor: usize) -> impl Iterator<Item = usize> + '_ {
    let side: Side = battle.fighters[actor].side.other();
    battle.standing(side).into_iter()
}

fn random(skill: &Skill) -> bool {
    skill.effects.iter().any(|effect| match effect {
        Effect::Special(special) => special.is_random(),
        Effect::Mutate(_) => true,
        _ => false,
    })
}

fn price_ink(price: Price) -> Ink {
    match price {
        Price::Bubbles(_) => Ink::Bubbles,
        Price::Purse { .. } => Ink::Gold,
        Price::Free => Ink::Quiet,
    }
}

fn percent(value: u32) -> String {
    format!("{value}%")
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    use super::*;
    use crate::dungeon::Difficulty;
    use crate::dungeon::battle::{BattleSetup, Stakes};
    use crate::dungeon::fighter::{Fighter, Scale};
    use crate::dungeon::floor;
    use crate::dungeon::moves;
    use crate::fishes::fish::Fish;
    use crate::fishes::species::FishSpecies;

    const SEED: u64 = 3;

    fn battle() -> Battle {
        let mut rng = SmallRng::seed_from_u64(SEED);
        let party = vec![Fish::new(
            FishSpecies::Salmon,
            "Lilith".to_string(),
            0.0,
            0.0,
            &mut rng,
        )];
        let foes: Vec<Fighter> = floor::encounter(1, floor::scale_for(1), &[], &mut rng);
        let setup = BattleSetup {
            floor: 1,
            difficulty: Difficulty::Normal,
            stakes: Stakes::Encounter,
            purse: 0,
            items: Vec::new(),
            seed: SEED,
            ambushed: false,
            haunts: Vec::new(),
            scale: Scale::PLAIN,
            known: Vec::new(),
            every_graeae: false,
        };
        Battle::new(party, foes, setup)
    }

    fn value(info: &Info, label: &str) -> Option<String> {
        info.rows
            .iter()
            .find(|row| row.label == label)
            .map(|row| row.value.clone())
    }

    #[test]
    fn the_bite_box_prints_the_range_the_engine_rolls() {
        let battle = battle();
        let (low, high) = battle.damage_range(0, Power::BITE);
        let info = Info::bite(&battle, 0, None);
        assert_eq!(value(&info, "Damage"), Some(format!("{low} to {high}")));
        assert_eq!(
            value(&info, "Cost"),
            Some(super::super::skill::FREE.to_string())
        );
        assert_eq!(value(&info, "Accuracy"), Some("95%".to_string()));
    }

    #[test]
    fn a_random_skill_says_so() {
        let battle = battle();
        let info = Info::skill(&battle, 0, &moves::miracle(), None);
        assert_eq!(value(&info, "Effect"), Some(UNKNOWN.to_string()));
    }

    #[test]
    fn a_special_that_strikes_shows_its_hits() {
        let battle = battle();
        let info = Info::skill(&battle, 0, &moves::roll(), None);
        let damage = value(&info, "Damage").expect("a damage row");
        assert!(damage.contains("hits"), "{damage}");
    }

    #[test]
    fn mp_costs_are_bubbles_in_the_box_too() {
        let battle = battle();
        let info = Info::skill(&battle, 0, &moves::roll(), None);
        assert_eq!(
            value(&info, "Cost"),
            Some(super::super::skill::FULL_BUBBLE.to_string())
        );
    }
}
