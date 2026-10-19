use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use super::botfish::{BotfishState, DueCast, DueLine};
use super::parts::{Display, KeyBinding, Part, Pin, PinDirection, PinMap, PinOwner};
use crate::tank::{Link, Netlist, Transmission, Wires, WorldView};

mod record;

use record::ChipRecord;

const CHIP_SUFFIX: &str = " chip";

#[derive(Clone)]
struct Tap {
    line: String,
    slot: usize,
    settled_this_stage: bool,
}

#[derive(Clone)]
enum Cell {
    Gate {
        fish: usize,
        reads: Vec<Tap>,
        drives: Option<usize>,
    },
    Fish {
        fish: usize,
        taps: Vec<Tap>,
    },
}

#[derive(Clone)]
struct Terminal {
    line: String,
    slot: usize,
}

#[derive(Clone, Default)]
struct Levels {
    held: Vec<bool>,
    settling: Vec<bool>,
    driven: Vec<bool>,
}

impl Levels {
    fn with_slots(slots: usize) -> Self {
        Self {
            held: vec![false; slots],
            settling: vec![false; slots],
            driven: vec![false; slots],
        }
    }

    fn fresh(&self) -> Self {
        Self::with_slots(self.held.len())
    }

    fn hold(&mut self, slot: usize, level: bool) {
        self.held[slot] = level;
    }

    fn read(&self, tap: &Tap) -> bool {
        if tap.settled_this_stage {
            return self.current(tap.slot);
        }
        self.held[tap.slot]
    }

    fn current(&self, slot: usize) -> bool {
        if self.driven[slot] {
            return self.settling[slot];
        }
        self.held[slot]
    }

    fn drive(&mut self, slot: usize, level: bool) {
        if self.driven[slot] {
            self.settling[slot] |= level;
            return;
        }
        self.settling[slot] = level;
        self.driven[slot] = true;
    }

    fn commit(&mut self) {
        for slot in 0..self.held.len() {
            if !self.driven[slot] {
                continue;
            }
            self.held[slot] = self.settling[slot];
            self.driven[slot] = false;
        }
    }
}

struct CellWires<'a> {
    taps: &'a [Tap],
    levels: &'a mut Levels,
}

impl CellWires<'_> {
    fn tap(&self, channel: &str) -> Option<&Tap> {
        self.taps.iter().find(|tap| tap.line == channel)
    }
}

impl Wires for CellWires<'_> {
    fn level(&self, channel: &str) -> bool {
        self.tap(channel).is_some_and(|tap| self.levels.read(tap))
    }

    fn drive(&mut self, channel: &str, level: bool) {
        let Some(slot) = self.tap(channel).map(|tap| tap.slot) else {
            return;
        };
        self.levels.drive(slot, level);
    }
}

#[derive(Clone)]
struct Compiled {
    cells: Vec<Cell>,
    inputs: Vec<Terminal>,
    outputs: Vec<Terminal>,
    levels: Levels,
}

impl Compiled {
    fn of(board: &[BotfishState], inputs: BTreeSet<String>, outputs: BTreeSet<String>) -> Self {
        let produces: Vec<Vec<String>> = board.iter().map(BotfishState::driven_lines).collect();
        let consumes: Vec<Vec<String>> = board.iter().map(BotfishState::fed_lines).collect();
        let order = Netlist::of(&produces, &consumes).settle().order;
        let mut position = vec![0; board.len()];
        for (place, &fish) in order.iter().enumerate() {
            position[fish] = place;
        }
        let mut slots: HashMap<String, usize> = HashMap::new();
        let mut drivers: HashMap<String, Vec<usize>> = HashMap::new();
        for (fish, lines) in produces.iter().enumerate() {
            for line in lines {
                drivers.entry(line.clone()).or_default().push(fish);
            }
        }
        let mut slot_of = |line: &str| {
            let next = slots.len();
            *slots.entry(line.to_string()).or_insert(next)
        };
        let cells = order
            .iter()
            .map(|&fish| {
                let settled = |line: &str| {
                    drivers.get(line).is_none_or(|driving| {
                        driving
                            .iter()
                            .all(|&driver| position[driver] < position[fish])
                    })
                };
                let mut taps: Vec<Tap> = Vec::new();
                for line in consumes[fish].iter().chain(&produces[fish]) {
                    if taps.iter().any(|tap| &tap.line == line) {
                        continue;
                    }
                    taps.push(Tap {
                        line: line.clone(),
                        slot: slot_of(line),
                        settled_this_stage: settled(line),
                    });
                }
                if !board[fish].is_gate() {
                    return Cell::Fish { fish, taps };
                }
                let drives = produces[fish].first().and_then(|line| {
                    taps.iter()
                        .find(|tap| &tap.line == line)
                        .map(|tap| tap.slot)
                });
                taps.truncate(consumes[fish].len());
                Cell::Gate {
                    fish,
                    reads: taps,
                    drives,
                }
            })
            .collect();
        let mut terminals = |lines: BTreeSet<String>| {
            lines
                .into_iter()
                .map(|line| Terminal {
                    slot: slot_of(&line),
                    line,
                })
                .collect::<Vec<_>>()
        };
        let inputs = terminals(inputs);
        let outputs = terminals(outputs);
        Self {
            cells,
            inputs,
            outputs,
            levels: Levels::with_slots(slots.len()),
        }
    }

    fn design(&self) -> Self {
        Self {
            levels: self.levels.fresh(),
            ..self.clone()
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(into = "ChipRecord", from = "ChipRecord")]
pub struct Chip {
    name: String,
    board: Vec<BotfishState>,
    compiled: Compiled,
    heard: bool,
    sealed: bool,
}

impl Chip {
    pub fn new(
        name: String,
        board: Vec<BotfishState>,
        inputs: BTreeSet<String>,
        outputs: BTreeSet<String>,
    ) -> Self {
        let compiled = Compiled::of(&board, inputs, outputs);
        Self {
            name,
            board,
            compiled,
            heard: false,
            sealed: false,
        }
    }

    pub fn sealed(mut self) -> Self {
        self.sealed = true;
        self
    }

    pub fn is_sealed(&self) -> bool {
        self.sealed
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn display_name(&self) -> String {
        format!("{}{CHIP_SUFFIX}", self.name)
    }

    pub fn board(&self) -> &[BotfishState] {
        &self.board
    }

    pub fn pins(&self) -> impl Iterator<Item = Pin> + '_ {
        let inputs = self
            .compiled
            .inputs
            .iter()
            .map(|input| Pin::line(input.line.clone(), PinDirection::In));
        let outputs = self
            .compiled
            .outputs
            .iter()
            .map(|output| Pin::line(output.line.clone(), PinDirection::Out));
        inputs.chain(outputs)
    }

    pub fn design(&self) -> Self {
        Self {
            name: self.name.clone(),
            board: self.board.iter().map(BotfishState::design).collect(),
            compiled: self.compiled.design(),
            heard: false,
            sealed: self.sealed,
        }
    }

    pub fn step<W: Wires>(
        &mut self,
        pins: &PinMap,
        outer: &mut W,
        world: &WorldView,
    ) -> Vec<Transmission> {
        let compiled = &mut self.compiled;
        let mut sent = Vec::new();
        self.heard = false;
        for input in &compiled.inputs {
            let level = pins
                .channel(PinOwner::Body, &input.line)
                .is_some_and(|channel| outer.level(channel));
            self.heard |= level;
            compiled.levels.hold(input.slot, level);
        }
        for cell in &compiled.cells {
            match cell {
                Cell::Gate {
                    fish,
                    reads,
                    drives,
                } => {
                    let sensed = reads.iter().any(|tap| compiled.levels.read(tap));
                    let level = self.board[*fish].respond(sensed);
                    if let Some(slot) = drives {
                        compiled.levels.drive(*slot, level);
                    }
                }
                Cell::Fish { fish, taps } => {
                    let mut wires = CellWires {
                        taps,
                        levels: &mut compiled.levels,
                    };
                    sent.extend(self.board[*fish].step(&mut wires, world));
                }
            }
        }
        for output in &compiled.outputs {
            let Some(channel) = pins.channel(PinOwner::Body, &output.line) else {
                continue;
            };
            outer.drive(channel, compiled.levels.current(output.slot));
        }
        compiled.levels.commit();
        sent
    }

    pub fn links(&self) -> impl Iterator<Item = Link> + '_ {
        self.board.iter().flat_map(BotfishState::links)
    }

    pub fn hears_high(&self) -> bool {
        self.heard
    }

    pub fn is_active(&self) -> bool {
        self.board.iter().any(BotfishState::output_level)
    }

    pub fn reads_world(&self) -> bool {
        self.board.iter().any(BotfishState::reads_world)
    }

    pub fn is_processing(&self) -> bool {
        self.board.iter().any(BotfishState::is_processing)
    }

    pub fn hear(&mut self, speech: &str) {
        for fish in &mut self.board {
            fish.hear(speech);
        }
    }

    pub fn key(&mut self, key: &str, down: bool) {
        for fish in &mut self.board {
            fish.key(key, down);
        }
    }

    pub fn bindings(&self) -> impl Iterator<Item = KeyBinding> + '_ {
        self.board.iter().flat_map(BotfishState::bindings)
    }

    pub fn due_lines(&mut self, coffee_stacks: u32) -> Vec<DueLine> {
        let mut due = Vec::new();
        for (index, fish) in self.board.iter_mut().enumerate() {
            for mut line in fish.due_lines(coffee_stacks) {
                line.path.insert(0, index);
                due.push(line);
            }
        }
        due
    }

    pub fn due_casts(&mut self, dt: f32) -> Vec<DueCast> {
        let mut due = Vec::new();
        for (index, fish) in self.board.iter_mut().enumerate() {
            for mut cast in fish.due_casts(dt) {
                cast.path.insert(0, index);
                due.push(cast);
            }
        }
        due
    }

    pub fn lower_line(&mut self, index: usize, rest: &[usize], secs: f32) {
        if let Some(fish) = self.board.get_mut(index) {
            fish.lower_line(rest, secs);
        }
    }

    pub fn reel_in(&mut self, index: usize, rest: &[usize], landed: bool) {
        if let Some(fish) = self.board.get_mut(index) {
            fish.reel_in(rest, landed);
        }
    }

    pub fn report_line(&mut self, index: usize, rest: &[usize], success: bool) {
        if let Some(fish) = self.board.get_mut(index) {
            fish.report_line(rest, success);
        }
    }

    pub fn rename_references(&mut self, renames: &[(&str, &str)]) {
        for fish in &mut self.board {
            fish.rename_references(renames);
        }
    }

    pub fn hardware(&self) -> impl Iterator<Item = (Part, u32)> + '_ {
        self.board
            .iter()
            .filter(|_| !self.sealed)
            .flat_map(BotfishState::hardware)
    }

    pub fn displays(&self) -> impl Iterator<Item = Display> + '_ {
        self.board.iter().flat_map(BotfishState::displays)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tank::ChannelRegistry;

    fn lines(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    fn gate(listens: &[&str], drives: &str, parts: &[Part]) -> BotfishState {
        let mut fish = BotfishState::new();
        for channel in listens {
            fish.listen(channel);
        }
        fish.drive(drives);
        for &part in parts {
            fish.install(part);
        }
        fish
    }

    fn order(chip: &Chip) -> Vec<usize> {
        chip.compiled
            .cells
            .iter()
            .map(|cell| match cell {
                Cell::Gate { fish, .. } | Cell::Fish { fish, .. } => *fish,
            })
            .collect()
    }

    fn step(chip: &mut Chip, channels: &mut ChannelRegistry) {
        let mut pins = PinMap::new();
        for pin in chip.pins().collect::<Vec<_>>() {
            pins.wire(pin.owner, &pin.name, &pin.name);
        }
        chip.step(&pins, channels, &WorldView::default());
        channels.commit();
    }

    #[test]
    fn a_board_compiles_drivers_first_whatever_order_it_was_drawn_in() {
        let board = vec![
            gate(&["b"], "c", &[]),
            gate(&["a"], "b", &[]),
            gate(&["x"], "a", &[]),
        ];

        let chip = Chip::new("Chain".to_string(), board, lines(&["x"]), lines(&["c"]));

        assert_eq!(order(&chip), vec![2, 1, 0]);
    }

    #[test]
    fn a_fish_with_only_its_own_output_compiles_to_a_gate() {
        let mut module = gate(&[], "", &[Part::CommandModule]);
        module.wire(Part::CommandModule, "fire", "go");
        let board = vec![
            gate(&["x"], "go", &[Part::InverterCoil, Part::DelaySpool]),
            module,
        ];

        let chip = Chip::new("Mixed".to_string(), board, lines(&["x"]), BTreeSet::new());

        assert!(matches!(chip.compiled.cells[0], Cell::Gate { .. }));
        assert!(
            matches!(chip.compiled.cells[1], Cell::Fish { .. }),
            "a pin is read through the fish's own hooks"
        );
    }

    #[test]
    fn a_line_heard_before_its_driver_speaks_is_read_from_the_last_stage() {
        let board = vec![
            gate(&["r", "qn"], "q", &[Part::InverterCoil]),
            gate(&["s", "q"], "qn", &[Part::InverterCoil]),
        ];
        let chip = Chip::new(
            "Latch".to_string(),
            board,
            lines(&["r", "s"]),
            lines(&["q"]),
        );

        let settled: Vec<(String, bool)> = chip
            .compiled
            .cells
            .iter()
            .flat_map(|cell| match cell {
                Cell::Gate { reads, .. } => reads.clone(),
                Cell::Fish { taps, .. } => taps.clone(),
            })
            .map(|tap| (tap.line, tap.settled_this_stage))
            .collect();

        assert_eq!(
            settled,
            vec![
                ("qn".to_string(), false),
                ("r".to_string(), true),
                ("q".to_string(), true),
                ("s".to_string(), true),
            ]
        );
    }

    #[test]
    fn a_compiled_chip_settles_a_chain_within_one_stage() {
        let board = vec![
            gate(&["b"], "c", &[Part::InverterCoil]),
            gate(&["a"], "b", &[Part::InverterCoil]),
            gate(&["x"], "a", &[Part::InverterCoil]),
        ];
        let mut chip = Chip::new("Chain".to_string(), board, lines(&["x"]), lines(&["c"]));
        let mut channels = ChannelRegistry::new();

        channels.set_level("x", true);
        step(&mut chip, &mut channels);

        assert!(!channels.level("c"));
        assert!(chip.hears_high());
    }

    #[test]
    fn a_design_keeps_the_compiled_order_and_forgets_the_levels() {
        let mut chip = Chip::new(
            "Not".to_string(),
            vec![gate(&["x"], "nx", &[Part::InverterCoil])],
            lines(&["x"]),
            lines(&["nx"]),
        );
        let mut channels = ChannelRegistry::new();
        step(&mut chip, &mut channels);
        assert!(chip.compiled.levels.held.iter().any(|&level| level));

        let design = chip.design();

        assert_eq!(order(&design), order(&chip));
        assert!(design.compiled.levels.held.iter().all(|&level| !level));
        assert!(!design.is_active());
    }

    #[test]
    fn a_command_module_inside_a_chip_fires_the_stage_its_wire_rises() {
        let mut module = gate(&[], "", &[Part::CommandModule]);
        module.wire(Part::CommandModule, "fire", "go");
        module.program(String::new(), vec!["/feed 1".to_string()]);
        let board = vec![module, gate(&["x"], "go", &[])];
        let mut chip = Chip::new("Feeder".to_string(), board, lines(&["x"]), BTreeSet::new());
        let mut channels = ChannelRegistry::new();

        channels.set_level("x", true);
        step(&mut chip, &mut channels);

        assert!(chip.is_processing());
    }
}
