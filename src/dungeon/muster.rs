use crate::fishes::fish::Fish;
use crate::loot::{ConsumableKind, StockItem};
use crate::ui::input_action::InputAction;

use super::Difficulty;
use super::beat::{LogLine, Tone};
use super::boon::Charm;
use super::expedition::MAX_PARTY;
use super::items::{Item, PACK_STACKS};
use super::room::{Act, Card, Panel, Portrait, Row, Scene, Tab, Verb};
use super::sheet;

pub const DIFFICULTIES: [Difficulty; 2] = [Difficulty::Normal, Difficulty::Hardcore];
pub const IN_THE_PARTY: &str = "in the party";
pub const NOT_WORN: &str = "not worn";
const CHARM_LINE: &str = "A charm is worn for life, one a fish: ←→ picks who wears it.";

pub fn charm_item(charm: Charm) -> Item {
    Item::Stock(StockItem::Consumable(ConsumableKind::Treasure(
        super::treasure::Treasure::Charm(charm),
    )))
}

pub fn fish_card(fish: &Fish, home: &str, lit: bool) -> Card {
    let mut caption = sheet::caption(fish, home);
    if lit {
        caption.push(LogLine::new(IN_THE_PARTY, Tone::Focus));
    }
    Card {
        title: fish.name.clone(),
        portrait: Portrait::Fish(Box::new(fish.clone())),
        caption,
        lit,
    }
}

fn packed_note(supply: &Supply) -> String {
    format!("{} of {}", supply.packed, supply.held)
}

fn packed_tone(supply: &Supply) -> Tone {
    match supply.packed {
        0 => Tone::Quiet,
        _ => Tone::Focus,
    }
}

#[derive(Clone)]
pub struct Candidate {
    pub tank: String,
    pub fish: Fish,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Supply {
    pub item: Item,
    pub held: u32,
    pub packed: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fitting {
    pub charm: Charm,
    pub wearer: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leg {
    Party,
    Pack,
    Lift,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusterStep {
    Stay,
    Close,
    SetOut(Difficulty),
}

#[derive(Clone)]
pub struct Muster {
    pub leg: Leg,
    pub candidates: Vec<Candidate>,
    pub party: Vec<usize>,
    pub supplies: Vec<Supply>,
    pub fittings: Vec<Fitting>,
    pub at: [usize; 3],
}

impl Muster {
    pub fn new(candidates: Vec<Candidate>, supplies: Vec<Supply>, charms: Vec<Charm>) -> Muster {
        Muster {
            leg: Leg::Party,
            candidates,
            party: Vec::new(),
            supplies,
            fittings: charms
                .into_iter()
                .map(|charm| Fitting {
                    charm,
                    wearer: None,
                })
                .collect(),
            at: [0; 3],
        }
    }

    pub fn fitted(&self) -> Vec<(Charm, usize)> {
        self.fittings
            .iter()
            .filter_map(|fitting| Some((fitting.charm, self.worn_by(fitting)?)))
            .collect()
    }

    fn worn_by(&self, fitting: &Fitting) -> Option<usize> {
        fitting
            .wearer
            .filter(|&candidate| self.is_picked(candidate))
    }

    fn wearers_for(&self, fitting: usize) -> Vec<Option<usize>> {
        let taken: Vec<usize> = self
            .fittings
            .iter()
            .enumerate()
            .filter(|&(other, _)| other != fitting)
            .filter_map(|(_, other)| self.worn_by(other))
            .collect();
        let free = self
            .party
            .iter()
            .copied()
            .filter(|candidate| !taken.contains(candidate))
            .filter(|&candidate| {
                let fish = &self.candidates[candidate].fish;
                fish.deep.as_ref().is_none_or(|life| life.charm.is_none())
            })
            .map(Some);
        std::iter::once(None).chain(free).collect()
    }

    fn fit(&mut self, fitting: usize, forward: bool) {
        let Some(current) = self.fittings.get(fitting).map(|f| self.worn_by(f)) else {
            return;
        };
        let options = self.wearers_for(fitting);
        let at = options.iter().position(|&o| o == current).unwrap_or(0);
        let step = if forward { 1 } else { options.len() - 1 };
        self.fittings[fitting].wearer = options[(at + step) % options.len()];
    }

    fn wearer_note(&self, fitting: &Fitting) -> (String, Tone) {
        match self.worn_by(fitting) {
            Some(candidate) => (
                format!("worn by {}", self.candidates[candidate].fish.name),
                Tone::Focus,
            ),
            None => (NOT_WORN.to_string(), Tone::Quiet),
        }
    }

    fn fitting_card(&self, fitting: &Fitting) -> Card {
        let (note, tone) = self.wearer_note(fitting);
        let item = charm_item(fitting.charm);
        Card {
            title: item.name().to_string(),
            portrait: Portrait::Item(item),
            caption: vec![LogLine::new(note, tone)],
            lit: self.worn_by(fitting).is_some(),
        }
    }

    fn slot(&self) -> usize {
        self.leg as usize
    }

    pub fn selected(&self) -> usize {
        self.at[self.slot()]
    }

    pub fn picked(&self) -> Vec<usize> {
        self.party.clone()
    }

    pub fn is_picked(&self, candidate: usize) -> bool {
        self.party.contains(&candidate)
    }

    pub fn slots(&self) -> Vec<Option<&Fish>> {
        (0..MAX_PARTY)
            .map(|slot| {
                self.party
                    .get(slot)
                    .map(|&index| &self.candidates[index].fish)
            })
            .collect()
    }

    pub fn stacks(&self) -> usize {
        self.supplies.iter().filter(|s| s.packed > 0).count()
    }

    fn rows(&self) -> usize {
        match self.leg {
            Leg::Party => self.candidates.len(),
            Leg::Pack => self.supplies.len() + self.fittings.len(),
            Leg::Lift => DIFFICULTIES.len(),
        }
    }

    pub fn panel(&self) -> Panel {
        match self.leg {
            Leg::Party => self.party_panel(),
            Leg::Pack => self.pack_panel(),
            Leg::Lift => self.lift_panel(),
        }
    }

    fn party_panel(&self) -> Panel {
        let selected = self.selected();
        let shown = self.candidates.get(selected);
        let picked = self.party.len();
        let rows = self
            .candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                let in_the_party = self.is_picked(index);
                let note = match in_the_party {
                    true => (IN_THE_PARTY.to_string(), Tone::Focus),
                    false => (
                        candidate.fish.species.display_name().to_string(),
                        Tone::Quiet,
                    ),
                };
                let full = picked >= MAX_PARTY && !in_the_party;
                Row::new(candidate.fish.name.clone(), Act::Choice(index as u8))
                    .note(note.0, note.1)
                    .usable(!full)
                    .about(sheet::fish(&candidate.fish))
            })
            .collect();
        Panel {
            title: "The Deep · Party".to_string(),
            art: match shown {
                Some(candidate) => Scene::Card(fish_card(
                    &candidate.fish,
                    &candidate.tank,
                    self.is_picked(selected),
                )),
                None => Scene::Rows(&[]),
            },
            figures: Vec::new(),
            draw_figures: false,
            text: Vec::new(),
            rows,
            which_fish: false,
            verb: Verb::Pick,
            counts: false,
            fits: false,
            tab: Some(Tab::Pack),
            back: false,
        }
    }

    fn pack_panel(&self) -> Panel {
        let stacks = self.stacks();
        let supplies = self.supplies.iter().enumerate().map(|(index, supply)| {
            let room = supply.packed > 0 || stacks < PACK_STACKS;
            Row::new(supply.item.name(), Act::Choice(index as u8))
                .note(packed_note(supply), packed_tone(supply))
                .usable(room)
                .about(sheet::item(supply.item))
        });
        let charms = self.fittings.iter().enumerate().map(|(k, fitting)| {
            let (note, tone) = self.wearer_note(fitting);
            let item = charm_item(fitting.charm);
            Row::new(item.name(), Act::Choice((self.supplies.len() + k) as u8))
                .note(note, tone)
                .usable(self.wearers_for(k).len() > 1)
                .about(sheet::item(item))
        });
        let rows = supplies.chain(charms).collect();
        let selected = self.selected();
        let art = match self.supplies.get(selected) {
            Some(supply) => Scene::Card(Card {
                title: supply.item.name().to_string(),
                portrait: Portrait::Item(supply.item),
                caption: vec![LogLine::new(
                    format!("{} packed", packed_note(supply)),
                    packed_tone(supply),
                )],
                lit: supply.packed > 0,
            }),
            None => match self.fittings.get(selected - self.supplies.len()) {
                Some(fitting) => Scene::Card(self.fitting_card(fitting)),
                None => Scene::Rows(&[]),
            },
        };
        let mut text = vec![LogLine::new(
            "Pack up to eight stacks. Whatever is not used comes home.",
            Tone::Plain,
        )];
        if !self.fittings.is_empty() {
            text.push(LogLine::new(CHARM_LINE, Tone::Plain));
        }
        text.push(LogLine::new(String::new(), Tone::Plain));
        text.push(LogLine::new(
            format!("Stacks: {stacks} of {PACK_STACKS}"),
            match stacks {
                0 => Tone::Quiet,
                _ => Tone::Focus,
            },
        ));
        Panel {
            title: "The Deep · Items".to_string(),
            art,
            figures: Vec::new(),
            draw_figures: false,
            text,
            rows,
            which_fish: false,
            verb: Verb::Pack,
            counts: selected < self.supplies.len(),
            fits: selected >= self.supplies.len(),
            tab: None,
            back: true,
        }
    }

    fn lift_panel(&self) -> Panel {
        let names: Vec<String> = self
            .picked()
            .iter()
            .map(|&i| self.candidates[i].fish.name.clone())
            .collect();
        Panel {
            title: "Elevator".to_string(),
            art: Scene::Lift { from: 0, to: 1 },
            figures: Vec::new(),
            draw_figures: false,
            text: vec![
                LogLine::new(format!("The party: {}", names.join(", ")), Tone::Focus),
                LogLine::new(
                    format!("Items {} of {PACK_STACKS}", self.stacks()),
                    Tone::Quiet,
                ),
                LogLine::new(String::new(), Tone::Plain),
                LogLine::new("Chosen once, for the whole dive.", Tone::Quiet),
            ],
            rows: vec![
                Row::new("Normal", Act::Down).note("no fish dies", Tone::Quiet),
                Row::new("Hardcore", Act::Down).note("fish can die · twice the pay", Tone::Hurt),
            ],
            which_fish: false,
            verb: Verb::Ride,
            counts: false,
            fits: false,
            tab: None,
            back: true,
        }
    }

    pub fn key(&mut self, action: &InputAction) -> MusterStep {
        let slot = self.slot();
        let rows = self.rows();
        match action {
            InputAction::Up if rows > 0 => {
                self.at[slot] = (self.at[slot] + rows - 1) % rows;
                MusterStep::Stay
            }
            InputAction::Down if rows > 0 => {
                self.at[slot] = (self.at[slot] + 1) % rows;
                MusterStep::Stay
            }
            InputAction::Cancel => match self.leg {
                Leg::Party => MusterStep::Close,
                Leg::Pack => {
                    self.leg = Leg::Party;
                    MusterStep::Stay
                }
                Leg::Lift => {
                    self.leg = Leg::Pack;
                    MusterStep::Stay
                }
            },
            InputAction::Tab if self.leg == Leg::Party && !self.picked().is_empty() => {
                self.leg = Leg::Pack;
                MusterStep::Stay
            }
            InputAction::Left | InputAction::Right if self.leg == Leg::Pack => {
                let forward = matches!(action, InputAction::Right);
                match self.at[Leg::Pack as usize].checked_sub(self.supplies.len()) {
                    Some(fitting) => self.fit(fitting, forward),
                    None => self.count(forward),
                }
                MusterStep::Stay
            }
            InputAction::Confirm => self.confirm(),
            _ => MusterStep::Stay,
        }
    }

    fn count(&mut self, more: bool) {
        let stacks = self.stacks();
        let Some(supply) = self.supplies.get_mut(self.at[Leg::Pack as usize]) else {
            return;
        };
        if more {
            let opens_a_stack = supply.packed == 0;
            if supply.packed < supply.held && (!opens_a_stack || stacks < PACK_STACKS) {
                supply.packed += 1;
            }
            return;
        }
        supply.packed = supply.packed.saturating_sub(1);
    }

    fn confirm(&mut self) -> MusterStep {
        match self.leg {
            Leg::Party => {
                let at = self.at[Leg::Party as usize];
                if at >= self.candidates.len() {
                    return MusterStep::Stay;
                }
                match self.party.iter().position(|&index| index == at) {
                    Some(slot) => {
                        self.party.remove(slot);
                    }
                    None if self.party.len() < MAX_PARTY => self.party.push(at),
                    None => {}
                }
                MusterStep::Stay
            }
            Leg::Pack => {
                self.leg = Leg::Lift;
                MusterStep::Stay
            }
            Leg::Lift => MusterStep::SetOut(DIFFICULTIES[self.at[Leg::Lift as usize]]),
        }
    }

    pub fn packed(&self) -> Vec<(Item, u32)> {
        self.supplies
            .iter()
            .filter(|s| s.packed > 0)
            .map(|s| (s.item, s.packed))
            .collect()
    }
}
