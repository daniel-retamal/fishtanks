use crate::fishes::fish::Fish;
use crate::ui::input_action::InputAction;

use super::Difficulty;
use super::beat::{LogLine, Tone};
use super::expedition::MAX_PARTY;
use super::items::{Item, PACK_STACKS};
use super::room::{Act, Card, Panel, Portrait, Row, Scene, Tab, Verb};
use super::sheet;

pub const DIFFICULTIES: [Difficulty; 2] = [Difficulty::Normal, Difficulty::Hardcore];
pub const IN_THE_PARTY: &str = "in the party";

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
    pub picked: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Supply {
    pub item: Item,
    pub held: u32,
    pub packed: u32,
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
    pub supplies: Vec<Supply>,
    pub at: [usize; 3],
}

impl Muster {
    pub fn new(candidates: Vec<Candidate>, supplies: Vec<Supply>) -> Muster {
        Muster {
            leg: Leg::Party,
            candidates,
            supplies,
            at: [0; 3],
        }
    }

    fn slot(&self) -> usize {
        self.leg as usize
    }

    pub fn selected(&self) -> usize {
        self.at[self.slot()]
    }

    pub fn picked(&self) -> Vec<usize> {
        (0..self.candidates.len())
            .filter(|&i| self.candidates[i].picked)
            .collect()
    }

    pub fn stacks(&self) -> usize {
        self.supplies.iter().filter(|s| s.packed > 0).count()
    }

    fn rows(&self) -> usize {
        match self.leg {
            Leg::Party => self.candidates.len(),
            Leg::Pack => self.supplies.len(),
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

    fn party_line(&self) -> LogLine {
        let names: Vec<&str> = self
            .picked()
            .into_iter()
            .map(|i| self.candidates[i].fish.name.as_str())
            .collect();
        let count = format!("{} of {MAX_PARTY}", names.len());
        match names.is_empty() {
            true => LogLine::new(format!("Party: nobody yet ({count})"), Tone::Quiet),
            false => LogLine::new(
                format!("Party: {} ({count})", names.join(", ")),
                Tone::Focus,
            ),
        }
    }

    fn party_panel(&self) -> Panel {
        let shown = self.candidates.get(self.selected());
        let picked = self.picked().len();
        let rows = self
            .candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                let note = match candidate.picked {
                    true => (IN_THE_PARTY.to_string(), Tone::Focus),
                    false => (
                        candidate.fish.species.display_name().to_string(),
                        Tone::Quiet,
                    ),
                };
                let full = picked >= MAX_PARTY && !candidate.picked;
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
                    candidate.picked,
                )),
                None => Scene::Rows(&[]),
            },
            figures: Vec::new(),
            draw_figures: false,
            text: vec![
                LogLine::new(
                    "Take one to four fish down. Whoever joins on the way has no limit.",
                    Tone::Plain,
                ),
                LogLine::new(String::new(), Tone::Plain),
                self.party_line(),
            ],
            rows,
            which_fish: false,
            verb: Verb::Pick,
            counts: false,
            tab: Some(Tab::Pack),
            back: false,
        }
    }

    fn pack_panel(&self) -> Panel {
        let stacks = self.stacks();
        let rows = self
            .supplies
            .iter()
            .enumerate()
            .map(|(index, supply)| {
                let room = supply.packed > 0 || stacks < PACK_STACKS;
                Row::new(supply.item.name(), Act::Choice(index as u8))
                    .note(packed_note(supply), packed_tone(supply))
                    .usable(room)
                    .about(sheet::item(supply.item))
            })
            .collect();
        let shown = self.supplies.get(self.selected());
        Panel {
            title: "The Deep · Items".to_string(),
            art: match shown {
                Some(supply) => Scene::Card(Card {
                    title: supply.item.name().to_string(),
                    portrait: Portrait::Item(supply.item),
                    caption: vec![LogLine::new(
                        format!("{} packed", packed_note(supply)),
                        packed_tone(supply),
                    )],
                    lit: supply.packed > 0,
                }),
                None => Scene::Rows(&[]),
            },
            figures: Vec::new(),
            draw_figures: false,
            text: vec![
                LogLine::new(
                    "Pack up to eight stacks. Whatever is not used comes home.",
                    Tone::Plain,
                ),
                LogLine::new(String::new(), Tone::Plain),
                LogLine::new(
                    format!("Stacks: {stacks} of {PACK_STACKS}"),
                    match stacks {
                        0 => Tone::Quiet,
                        _ => Tone::Focus,
                    },
                ),
            ],
            rows,
            which_fish: false,
            verb: Verb::Pack,
            counts: true,
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
                self.count(matches!(action, InputAction::Right));
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
                let picked = self.picked().len();
                if let Some(candidate) = self.candidates.get_mut(at)
                    && (candidate.picked || picked < MAX_PARTY)
                {
                    candidate.picked = !candidate.picked;
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
