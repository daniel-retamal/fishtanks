use crate::fishes::fish::Fish;

use super::beat::{LogLine, Tone};
use super::expedition::Expedition;
use super::items::{Aim, Item};
use super::kit::Kit;
use super::muster::fish_card;
use super::room::{Act, Card, Panel, Portrait, Row, Scene, Verb};
use super::skill::{bubbles, cost_label};

pub const SEPARATOR: &str = " · ";
pub const NO_SKILLS: &str = "Bites, guards and flees.";
pub const SKILLS: &str = "Skills";
pub const JOINED: &str = "joined on the way";
pub const BELLY_UP: &str = "belly up";
pub const NOTHING_PACKED: &str = "Nothing is packed.";
pub const A_COW: &str = "a cow";
pub const COMES_HOME: &str = "comes home with the party";

fn browse(title: &str, art: Scene, text: Vec<LogLine>, rows: Vec<Row>) -> Panel {
    Panel {
        title: title.to_string(),
        art,
        figures: Vec::new(),
        draw_figures: false,
        text,
        rows,
        which_fish: false,
        verb: Verb::Browse,
        counts: false,
        fits: false,
        tab: None,
        back: true,
    }
}

impl Expedition {
    fn health(&self, member: usize) -> LogLine {
        let kit = Kit::of(&self.party[member]);
        match self.wound(member).belly_up {
            true => LogLine::new(BELLY_UP, Tone::Hurt),
            false => LogLine::new(
                format!("{}/{} HP", self.hp(member, &kit), kit.vigor),
                Tone::Heal,
            ),
        }
    }

    pub fn party_sheet(&self, at: usize) -> Panel {
        let fish_count = self.party.len();
        let at = at.min((fish_count + self.cows.len()).saturating_sub(1));
        let mut rows: Vec<Row> = (0..fish_count)
            .map(|member| {
                let health = self.health(member);
                Row::new(self.party[member].name.clone(), Act::Choice(member as u8))
                    .note(health.text, health.tone)
                    .about(fish(&self.party[member]))
            })
            .collect();
        for (herd, cow) in self.cows.iter().enumerate() {
            rows.push(
                Row::new(cow.name.clone(), Act::Choice((fish_count + herd) as u8))
                    .note(A_COW, Tone::Quiet)
                    .about(kit_lines(&Kit::of_cow(cow))),
            );
        }
        let art = match (
            self.party.get(at),
            self.cows.get(at.saturating_sub(fish_count)),
        ) {
            (Some(member), _) => {
                let home = self.homes.get(at).map_or(JOINED, String::as_str);
                let mut card = fish_card(member, home, false);
                card.caption.push(self.health(at));
                Scene::Card(card)
            }
            (None, Some(cow)) => Scene::Card(Card {
                title: cow.name.clone(),
                portrait: Portrait::Cow(Box::new(cow.clone())),
                caption: vec![
                    LogLine::new(A_COW, Tone::Plain),
                    LogLine::new(COMES_HOME, Tone::Quiet),
                ],
                lit: false,
            }),
            (None, None) => Scene::Rows(&[]),
        };
        browse("Party", art, Vec::new(), rows)
    }

    pub fn item_sheet(&self, at: usize) -> Panel {
        let packed: Vec<_> = self.pack.iter().filter(|p| p.count > 0).collect();
        let rows = packed
            .iter()
            .enumerate()
            .map(|(index, pack)| {
                Row::new(pack.item.name(), Act::Choice(index as u8))
                    .note(format!("×{}", pack.count), Tone::Focus)
                    .about(item(pack.item))
            })
            .collect();
        let art = match packed.get(at.min(packed.len().saturating_sub(1))) {
            Some(pack) => Scene::Card(Card {
                title: pack.item.name().to_string(),
                portrait: Portrait::Item(pack.item),
                caption: vec![LogLine::new(format!("×{}", pack.count), Tone::Focus)],
                lit: true,
            }),
            None => Scene::Rows(&[]),
        };
        let text = match packed.is_empty() {
            true => vec![LogLine::new(NOTHING_PACKED, Tone::Quiet)],
            false => Vec::new(),
        };
        browse("Items", art, text, rows)
    }
}

fn blank() -> LogLine {
    LogLine::new(String::new(), Tone::Plain)
}

pub fn caption(fish: &Fish, home: &str) -> Vec<LogLine> {
    let kit = Kit::of(fish);
    vec![
        LogLine::new(
            format!(
                "{}{SEPARATOR}{}",
                fish.species.display_name(),
                kit.grade.name()
            ),
            Tone::Plain,
        ),
        LogLine::new(home.to_string(), Tone::Quiet),
    ]
}

pub fn fish(fish: &Fish) -> Vec<LogLine> {
    let mut lines = kit_lines(&Kit::of(fish));
    if let Some(life) = fish.deep.as_ref() {
        if !life.boons.is_empty() || !life.curses.is_empty() {
            lines.push(blank());
        }
        for boon in &life.boons {
            lines.push(LogLine::new(
                format!("{}: {}", boon.name(), boon.description()),
                Tone::Heal,
            ));
        }
        for curse in &life.curses {
            lines.push(LogLine::new(
                format!("{}: {}", curse.name(), curse.description()),
                Tone::Hurt,
            ));
        }
    }
    lines
}

pub fn kit_lines(kit: &Kit) -> Vec<LogLine> {
    let mut lines = vec![
        LogLine::new(
            format!(
                "Vigor {}{SEPARATOR}Bite {}{SEPARATOR}Speed {}",
                kit.vigor, kit.bite, kit.speed
            ),
            Tone::Plain,
        ),
        LogLine::new(
            format!(
                "Dodge {}%{SEPARATOR}Crit {}%{SEPARATOR}MP {}",
                kit.dodge,
                kit.crit,
                bubbles(kit.start_mp.min(kit.max_mp), kit.max_mp)
            ),
            Tone::Plain,
        ),
    ];
    if kit.armour > 0 {
        lines.push(LogLine::new(format!("Armour {}", kit.armour), Tone::Plain));
    }
    lines.push(blank());
    lines.push(LogLine::new(SKILLS, Tone::Quiet));
    if kit.skills.is_empty() {
        lines.push(LogLine::new(NO_SKILLS, Tone::Plain));
    }
    for skill in &kit.skills {
        lines.push(LogLine::new(
            format!("{} {}", skill.name, cost_label(skill.price)),
            Tone::Focus,
        ));
        if !skill.about.is_empty() {
            lines.push(LogLine::new(skill.about.clone(), Tone::Plain));
        }
    }
    lines
}

pub fn aim(item: Item) -> Option<&'static str> {
    if !item.usable_in_a_fight() {
        return None;
    }
    Some(match item.aim() {
        Aim::Ally => "Used on one of your fish.",
        Aim::Foe => "Thrown at one enemy.",
        Aim::Nobody => "Works on everyone at once.",
    })
}

pub fn item(item: Item) -> Vec<LogLine> {
    let mut lines = vec![LogLine::new(item.about(), Tone::Plain)];
    if let Some(aim) = aim(item) {
        lines.push(LogLine::new(aim, Tone::Quiet));
    }
    lines
}
