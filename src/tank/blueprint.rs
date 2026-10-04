use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use rand::RngExt;

use super::Tank;
use crate::economy::{Money, Sellable};
use crate::entities::components::Position;
use crate::fishes::botfish::BotfishState;
use crate::fishes::chip::Chip;
use crate::fishes::fish::{Direction, Fish};
use crate::fishes::parts::Part;
use crate::fishes::species::FishSpecies;
use crate::loot::{
    CIRCUIT_BLUEPRINT_DESCRIPTION, CIRCUIT_BLUEPRINT_SELL_PRICE, ConsumableKind, StockItem,
};
use crate::names;
use crate::tank::{ChannelRegistry, TankKind};

const EXPORT_SEPARATOR: char = ',';
const EXPORT_JOIN: &str = ", ";
const NAMESPACE_SEPARATOR: char = '.';
const SLUG_JOIN: &str = "-";
const FIRST_INSTANCE: u32 = 1;
const PRINT_WAFERS_PER_FISH: u32 = 1;
const ETCH_WAFERS_PER_FISH: u32 = 2;
const FABRICATORS_PER_RUN: u32 = 1;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fabrication {
    Print,
    Etch,
}

impl Fabrication {
    fn wafers_per_fish(self) -> u32 {
        match self {
            Fabrication::Print => PRINT_WAFERS_PER_FISH,
            Fabrication::Etch => ETCH_WAFERS_PER_FISH,
        }
    }
}

fn etched_fish(design: &BotfishState) -> u32 {
    design.chip().map_or(0, |chip| {
        chip.board()
            .iter()
            .map(|inner| 1 + etched_fish(inner))
            .sum()
    })
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BlueprintFish {
    pub name: String,
    pub design: BotfishState,
    pub offset: Position,
    pub facing: Direction,
    pub frozen: bool,
}

impl BlueprintFish {
    fn of(fish: &Fish, origin: &Position) -> Option<Self> {
        let design = fish.script()?.design();
        Some(Self {
            name: fish.name.clone(),
            design,
            offset: Position {
                x: fish.position.x - origin.x,
                y: fish.position.y - origin.y,
            },
            facing: fish.facing,
            frozen: fish.frozen,
        })
    }
}

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct BlueprintPins {
    pub inputs: BTreeSet<String>,
    pub outputs: BTreeSet<String>,
    pub internals: BTreeSet<String>,
}

impl BlueprintPins {
    pub fn of<'a>(designs: impl IntoIterator<Item = &'a BotfishState>) -> Self {
        let mut driven = BTreeSet::new();
        let mut fed = BTreeSet::new();
        for design in designs {
            driven.extend(design.driven_lines());
            fed.extend(design.fed_lines());
        }
        Self {
            inputs: fed.difference(&driven).cloned().collect(),
            outputs: driven.difference(&fed).cloned().collect(),
            internals: driven.intersection(&fed).cloned().collect(),
        }
    }

    fn exporting(mut self, exports: &BTreeSet<String>) -> Self {
        for channel in exports {
            if self.internals.remove(channel) {
                self.outputs.insert(channel.clone());
            }
        }
        self
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub name: String,
    pub fish: Vec<BlueprintFish>,
    pub exports: BTreeSet<String>,
}

impl Blueprint {
    pub fn capture(name: String, fish: &[Fish]) -> Option<Self> {
        let wired: Vec<&Fish> = fish.iter().filter(|f| f.is_wired()).collect();
        if wired.is_empty() {
            return None;
        }
        let origin = Position {
            x: wired
                .iter()
                .map(|f| f.position.x)
                .fold(f32::INFINITY, f32::min),
            y: wired
                .iter()
                .map(|f| f.position.y)
                .fold(f32::INFINITY, f32::min),
        };
        Some(Self {
            name,
            fish: wired
                .into_iter()
                .filter_map(|f| BlueprintFish::of(f, &origin))
                .collect(),
            exports: BTreeSet::new(),
        })
    }

    fn inferred_pins(&self) -> BlueprintPins {
        BlueprintPins::of(self.fish.iter().map(|f| &f.design))
    }

    pub fn pins(&self) -> BlueprintPins {
        self.inferred_pins().exporting(&self.exports)
    }

    pub fn export(&mut self, channels: &str) {
        let internals = self.inferred_pins().internals;
        self.exports = channels
            .split(EXPORT_SEPARATOR)
            .filter_map(ChannelRegistry::normalize)
            .map(|channel| channel.into_owned())
            .filter(|channel| internals.contains(channel))
            .collect();
    }

    pub fn exports_text(&self) -> String {
        self.exports
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(EXPORT_JOIN)
    }

    pub fn part_totals(&self) -> Vec<(Part, u32)> {
        let mut totals: BTreeMap<Part, u32> = BTreeMap::new();
        for (part, count) in self.fish.iter().flat_map(|f| f.design.hardware()) {
            *totals.entry(part).or_insert(0) += count;
        }
        totals.into_iter().collect()
    }

    pub fn description(&self) -> String {
        format!(
            "{} fish. {}",
            self.fish.len(),
            CIRCUIT_BLUEPRINT_DESCRIPTION
        )
    }

    pub fn slug(&self) -> String {
        self.name
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(SLUG_JOIN)
            .to_ascii_lowercase()
    }

    fn prefix(&self, instance: u32) -> String {
        format!("{}{instance}{NAMESPACE_SEPARATOR}", self.slug())
    }

    pub fn free_instance(&self, wires: &BTreeSet<String>) -> u32 {
        (FIRST_INSTANCE..)
            .find(|&instance| {
                let prefix = self.prefix(instance);
                !wires.iter().any(|wire| wire.starts_with(&prefix))
            })
            .unwrap_or(FIRST_INSTANCE)
    }

    pub fn instance_name(&self, instance: u32, fish_name: &str) -> String {
        format!("{}{instance} {fish_name}", self.name)
    }

    pub fn instance(&self, instance: u32, names: &[String]) -> Vec<BlueprintFish> {
        let internals = self.pins().internals;
        let prefix = self.prefix(instance);
        let renames: Vec<(&str, &str)> = self
            .fish
            .iter()
            .map(|fish| fish.name.as_str())
            .zip(names.iter().map(String::as_str))
            .collect();
        self.fish
            .iter()
            .zip(names)
            .map(|(fish, name)| {
                let mut design = fish.design.clone();
                design.namespace(&internals, |channel| format!("{prefix}{channel}"));
                design.rename_references(&renames);
                design.mark_printed();
                BlueprintFish {
                    name: name.clone(),
                    design,
                    offset: fish.offset.clone(),
                    facing: fish.facing,
                    frozen: fish.frozen,
                }
            })
            .collect()
    }

    pub fn chip(&self, host: &str) -> Chip {
        let pins = self.pins();
        let renames: Vec<(&str, &str)> = self
            .fish
            .iter()
            .map(|fish| (fish.name.as_str(), host))
            .collect();
        let board = self
            .fish
            .iter()
            .map(|fish| {
                let mut design = fish.design.clone();
                design.rename_references(&renames);
                design
            })
            .collect();
        Chip::new(self.name.clone(), board, pins.inputs, pins.outputs)
    }

    pub fn bill_of_materials(&self, fabrication: Fabrication) -> Vec<(ConsumableKind, u32)> {
        let burned: u32 = self.fish.iter().map(|fish| etched_fish(&fish.design)).sum();
        let wafers = self.fish.len() as u32 * fabrication.wafers_per_fish()
            + burned * Fabrication::Etch.wafers_per_fish();
        std::iter::once((ConsumableKind::BlankWafer, wafers))
            .chain(
                self.part_totals()
                    .into_iter()
                    .map(|(part, count)| (ConsumableKind::Part(part), count)),
            )
            .collect()
    }

    pub fn quote(
        &self,
        fabrication: Fabrication,
        workshop: &Workshop,
    ) -> Result<FabricationQuote, FabricationRefusal> {
        if workshop.held(ConsumableKind::Fabricator) < FABRICATORS_PER_RUN {
            return Err(FabricationRefusal::NeedsFabricator);
        }
        match fabrication {
            Fabrication::Print if workshop.room < self.fish.len() => {
                return Err(FabricationRefusal::TankFull {
                    needed: self.fish.len(),
                });
            }
            Fabrication::Etch if workshop.hosts == 0 => {
                return Err(FabricationRefusal::NoHost);
            }
            _ => {}
        }
        let materials: Vec<Material> = self
            .bill_of_materials(fabrication)
            .into_iter()
            .map(|(kind, needed)| Material {
                kind,
                needed,
                in_stock: workshop.held(kind),
            })
            .collect();
        if !workshop.connected && materials.iter().any(|material| material.to_buy() > 0) {
            return Err(FabricationRefusal::NotConnected);
        }
        let cost = materials.iter().map(Material::cost).sum();
        if Money::from(cost) > workshop.cash {
            return Err(FabricationRefusal::Unaffordable { cost });
        }
        Ok(FabricationQuote { materials, cost })
    }
}

pub struct Workshop<'a> {
    pub stock: &'a HashMap<StockItem, u32>,
    pub cash: Money,
    pub room: usize,
    pub hosts: usize,
    pub connected: bool,
}

impl Workshop<'_> {
    fn held(&self, kind: ConsumableKind) -> u32 {
        self.stock
            .get(&StockItem::Consumable(kind))
            .copied()
            .unwrap_or(0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Material {
    pub kind: ConsumableKind,
    pub needed: u32,
    pub in_stock: u32,
}

impl Material {
    pub fn from_stock(&self) -> u32 {
        self.needed.min(self.in_stock)
    }

    pub fn to_buy(&self) -> u32 {
        self.needed - self.from_stock()
    }

    fn cost(&self) -> u32 {
        self.to_buy() * self.kind.buy_price()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct FabricationQuote {
    pub materials: Vec<Material>,
    pub cost: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FabricationRefusal {
    NeedsFabricator,
    TankFull { needed: usize },
    NoHost,
    NotConnected,
    Unaffordable { cost: u32 },
}

impl FabricationRefusal {
    pub fn reason(self) -> String {
        match self {
            FabricationRefusal::NeedsFabricator => "needs a Fabricator".to_string(),
            FabricationRefusal::TankFull { needed } => {
                format!("tank full: needs room for {needed} fish")
            }
            FabricationRefusal::NoHost => "needs a botfish to etch into".to_string(),
            FabricationRefusal::NotConnected => format!(
                "needs a {} to buy what is missing",
                TankKind::Matrix.display_name()
            ),
            FabricationRefusal::Unaffordable { cost } => {
                format!("needs ${cost} to buy what is missing")
            }
        }
    }
}

impl Tank {
    fn wires(&self) -> BTreeSet<String> {
        self.channels
            .names()
            .map(str::to_string)
            .chain(
                self.fish
                    .iter()
                    .filter_map(Fish::script)
                    .flat_map(|bot| bot.driven_lines().into_iter().chain(bot.fed_lines())),
            )
            .collect()
    }

    fn printed_names(&self, blueprint: &Blueprint, instance: u32) -> Vec<String> {
        let mut taken = self.used_names.clone();
        blueprint
            .fish
            .iter()
            .map(|fish| {
                let name =
                    names::unique_name_in(&taken, &blueprint.instance_name(instance, &fish.name));
                taken.insert(name.clone());
                name
            })
            .collect()
    }

    pub fn print(&mut self, blueprint: &Blueprint, rng: &mut impl RngExt) -> bool {
        if self.room() < blueprint.fish.len() {
            return false;
        }
        let instance = blueprint.free_instance(&self.wires());
        let names = self.printed_names(blueprint, instance);
        let board: Vec<(Fish, Position)> = blueprint
            .instance(instance, &names)
            .into_iter()
            .map(|printed| hatch(printed, rng))
            .collect();
        let origin = landing_spot(&board, self.width, self.height, rng);
        let (width, height) = (self.width, self.height);
        for (mut fish, offset) in board {
            fish.position = Position {
                x: origin.x + offset.x,
                y: origin.y + offset.y,
            };
            fish.nudge(0, 0, width, height);
            let name = fish.name.clone();
            self.admit(fish, name);
        }
        true
    }
}

fn landing_spot(
    board: &[(Fish, Position)],
    width: u16,
    height: u16,
    rng: &mut impl RngExt,
) -> Position {
    let fits = board.iter().fold(
        (f32::MIN, f32::MAX, f32::MIN, f32::MAX),
        |(lo_x, hi_x, lo_y, hi_y), (fish, offset)| {
            let (min_x, max_x, min_y, max_y) = fish.position_bounds(width, height);
            (
                lo_x.max(min_x - offset.x),
                hi_x.min(max_x - offset.x),
                lo_y.max(min_y - offset.y),
                hi_y.min(max_y - offset.y),
            )
        },
    );
    Position {
        x: random_between(rng, fits.0, fits.1),
        y: random_between(rng, fits.2, fits.3),
    }
}

fn random_between(rng: &mut impl RngExt, low: f32, high: f32) -> f32 {
    if high <= low {
        return low.max(0.0);
    }
    rng.random_range(low..=high)
}

fn hatch(printed: BlueprintFish, rng: &mut impl RngExt) -> (Fish, Position) {
    let mut fish = Fish::new(FishSpecies::Botfish, printed.name, 0.0, 0.0, rng);
    fish.botfish_state = Some(Box::new(printed.design));
    fish.frozen = printed.frozen;
    if fish.facing != printed.facing {
        fish.flip();
    }
    (fish, printed.offset)
}

impl Sellable for Blueprint {
    fn sell_price(&self) -> u32 {
        CIRCUIT_BLUEPRINT_SELL_PRICE
    }

    fn display_name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fishes::parts::{Part, PinDirection};

    fn gate(listens: &[&str], drives: &str, parts: &[Part]) -> BotfishState {
        let mut bot = BotfishState::new();
        for channel in listens {
            bot.listen(channel);
        }
        bot.drive(drives);
        for &part in parts {
            bot.install(part);
        }
        bot
    }

    fn set(lines: &[&str]) -> BTreeSet<String> {
        lines.iter().map(|line| line.to_string()).collect()
    }

    const COIL: &[Part] = &[Part::InverterCoil];

    #[test]
    fn a_not_gate_hears_its_input_and_drives_its_output_out_of_the_board() {
        let pins = BlueprintPins::of(&[gate(&["x"], "nx", COIL)]);

        assert_eq!(pins.inputs, set(&["x"]));
        assert_eq!(pins.outputs, set(&["nx"]));
        assert!(pins.internals.is_empty());
    }

    #[test]
    fn a_ring_oscillator_is_all_inside_and_has_no_pins() {
        let pins = BlueprintPins::of(&[gate(&["ring"], "ring", COIL)]);

        assert!(
            pins.inputs.is_empty(),
            "nothing outside feeds a self-fed coil"
        );
        assert!(
            pins.outputs.is_empty(),
            "and nothing it drives goes unheard"
        );
        assert_eq!(pins.internals, set(&["ring"]));
    }

    #[test]
    fn an_and_gate_exposes_its_two_inputs_and_hides_its_inverted_middle() {
        let board = [
            gate(&["a"], "na", COIL),
            gate(&["b"], "nb", COIL),
            gate(&["na", "nb"], "and", COIL),
        ];

        let pins = BlueprintPins::of(&board);

        assert_eq!(pins.inputs, set(&["a", "b"]));
        assert_eq!(pins.outputs, set(&["and"]));
        assert_eq!(pins.internals, set(&["na", "nb"]));
    }

    #[test]
    fn a_fan_in_wire_is_one_output_however_many_fish_drive_it() {
        let board = [
            gate(&["na", "b"], "xor", COIL),
            gate(&["a", "nb"], "xor", COIL),
        ];

        let pins = BlueprintPins::of(&board);

        assert_eq!(pins.outputs, set(&["xor"]));
        assert_eq!(pins.inputs, set(&["a", "b", "na", "nb"]));
    }

    #[test]
    fn an_unheard_bus_is_a_dangling_output_on_every_one_of_its_lines() {
        let mut counter = BotfishState::new();
        counter.install(Part::ShoalCounter);
        counter.wire(Part::ShoalCounter, "count", "shoal");

        let pins = BlueprintPins::of(&[counter]);

        let lines: Vec<String> = (0..8).map(|bit| format!("shoal{bit}")).collect();
        assert_eq!(pins.outputs, lines.into_iter().collect());
        assert!(pins.inputs.is_empty());
    }

    #[test]
    fn a_bus_line_heard_inside_the_board_stops_being_a_pin() {
        let mut counter = BotfishState::new();
        counter.install(Part::ShoalCounter);
        counter.wire(Part::ShoalCounter, "count", "shoal");
        let reader = gate(&["shoal7"], "big", COIL);

        let pins = BlueprintPins::of(&[counter, reader]);

        assert!(pins.internals.contains("shoal7"));
        assert!(!pins.outputs.contains("shoal7"));
        assert!(pins.outputs.contains("shoal0"));
        assert!(pins.outputs.contains("big"));
    }

    #[test]
    fn a_command_modules_fire_pin_is_an_input_when_nothing_on_the_board_drives_it() {
        let mut module = BotfishState::new();
        module.install(Part::CommandModule);
        module.wire(Part::CommandModule, "fire", "harvest");

        let alone = BlueprintPins::of(std::slice::from_ref(&module));
        assert_eq!(alone.inputs, set(&["harvest"]));

        let with_gate = BlueprintPins::of(&[module, gate(&["ripe"], "harvest", &[])]);
        assert_eq!(with_gate.internals, set(&["harvest"]));
        assert_eq!(with_gate.inputs, set(&["ripe"]));
        assert!(with_gate.outputs.is_empty());
    }

    fn blueprint_of(designs: Vec<BotfishState>) -> Blueprint {
        Blueprint {
            name: "Board".to_string(),
            fish: designs
                .into_iter()
                .enumerate()
                .map(|(index, design)| BlueprintFish {
                    name: format!("F{index}"),
                    design,
                    offset: Position { x: 0.0, y: 0.0 },
                    facing: Direction::Right,
                    frozen: true,
                })
                .collect(),
            exports: BTreeSet::new(),
        }
    }

    fn latch() -> Blueprint {
        blueprint_of(vec![
            gate(&["r", "qn"], "q", COIL),
            gate(&["s", "q"], "qn", COIL),
        ])
    }

    #[test]
    fn an_exported_latch_output_becomes_a_pin_and_stops_being_internal() {
        let mut blueprint = latch();

        blueprint.export("q");

        let pins = blueprint.pins();
        assert_eq!(pins.outputs, set(&["q"]));
        assert_eq!(pins.internals, set(&["qn"]));
        assert_eq!(
            pins.inputs,
            set(&["r", "s"]),
            "exporting never touches the inputs"
        );
    }

    #[test]
    fn only_an_internal_channel_can_be_exported() {
        let mut blueprint = latch();

        blueprint.export("r, nowhere, q");

        assert_eq!(
            blueprint.exports,
            set(&["q"]),
            "an input is already visible and an unknown channel is not on the board"
        );
    }

    #[test]
    fn exports_are_spelled_like_every_other_channel_and_replace_the_old_ones() {
        let mut blueprint = latch();
        blueprint.export("q");

        blueprint.export("  QN ,, ");

        assert_eq!(blueprint.exports, set(&["qn"]));
        assert_eq!(blueprint.exports_text(), "qn");
        blueprint.export("");
        assert!(
            blueprint.exports.is_empty(),
            "a blank edit clears every export"
        );
    }

    #[test]
    fn part_totals_add_up_every_fish_on_the_board() {
        let mut spooled = gate(&["x"], "y", COIL);
        spooled.install(Part::DelaySpool);
        spooled.install(Part::DelaySpool);
        let blueprint = blueprint_of(vec![spooled, gate(&["y"], "z", COIL)]);

        assert_eq!(
            blueprint.part_totals(),
            vec![(Part::InverterCoil, 2), (Part::DelaySpool, 2)]
        );
    }

    #[test]
    fn a_blueprint_sells_for_exactly_what_a_blank_resells_for() {
        assert_eq!(
            latch().sell_price(),
            crate::loot::ConsumableKind::BlankBlueprint.sell_price(),
            "capturing is free, so a design must never out-sell the blank it was drawn on"
        );
    }

    fn named_latch() -> Blueprint {
        let mut blueprint = latch();
        blueprint.name = "Ring Latch".to_string();
        blueprint.fish[0].name = "Q".to_string();
        blueprint.fish[1].name = "Qn".to_string();
        blueprint
    }

    fn printed(blueprint: &Blueprint, instance: u32) -> Vec<BlueprintFish> {
        let names: Vec<String> = blueprint
            .fish
            .iter()
            .map(|fish| blueprint.instance_name(instance, &fish.name))
            .collect();
        blueprint.instance(instance, &names)
    }

    fn wires_of(board: &[BlueprintFish]) -> BTreeSet<String> {
        board
            .iter()
            .flat_map(|fish| {
                fish.design
                    .driven_lines()
                    .into_iter()
                    .chain(fish.design.fed_lines())
            })
            .collect()
    }

    #[test]
    fn a_print_prefixes_its_internals_with_the_slugged_name_and_instance() {
        let blueprint = named_latch();
        assert_eq!(blueprint.slug(), "ring-latch");

        let board = printed(&blueprint, 1);

        assert_eq!(board[0].name, "Ring Latch1 Q");
        assert_eq!(board[0].design.drives(), Some("ring-latch1.q"));
        assert_eq!(
            wires_of(&board),
            set(&["r", "s", "ring-latch1.q", "ring-latch1.qn"]),
            "the inputs stay outside, everything heard inside is namespaced"
        );
    }

    #[test]
    fn two_prints_of_one_blueprint_never_share_an_internal_wire() {
        let blueprint = named_latch();
        let internals = |board: &[BlueprintFish]| -> BTreeSet<String> {
            wires_of(board)
                .into_iter()
                .filter(|wire| !blueprint.pins().inputs.contains(wire))
                .collect()
        };

        let first = internals(&printed(&blueprint, 1));
        let second = internals(&printed(&blueprint, 2));

        assert!(!first.is_empty());
        assert!(first.is_disjoint(&second), "{first:?} vs {second:?}");
    }

    #[test]
    fn an_exported_channel_keeps_its_name_so_every_print_shares_it() {
        let mut blueprint = named_latch();
        blueprint.export("q");

        let first = printed(&blueprint, 1);
        let second = printed(&blueprint, 2);

        assert_eq!(first[0].design.drives(), Some("q"));
        assert_eq!(second[0].design.drives(), Some("q"));
        assert_eq!(first[1].design.drives(), Some("ring-latch1.qn"));
        assert!(second[1].design.hears("q"), "Qn still hears the shared q");
    }

    #[test]
    fn a_bus_with_a_line_heard_inside_is_namespaced_whole_by_its_base_channel() {
        let mut counter = BotfishState::new();
        counter.install(Part::ShoalCounter);
        counter.wire(Part::ShoalCounter, "count", "shoal");
        let blueprint = blueprint_of(vec![counter, gate(&["shoal7"], "big", COIL)]);

        let board = printed(&blueprint, 1);

        assert_eq!(
            board[0].design.pins().channel(Part::ShoalCounter, "count"),
            Some("board1.shoal")
        );
        assert!(board[1].design.hears("board1.shoal7"));
        assert_eq!(board[1].design.drives(), Some("big"));
    }

    #[test]
    fn a_free_instance_is_the_lowest_number_no_wire_in_the_tank_uses() {
        let blueprint = named_latch();
        assert_eq!(blueprint.free_instance(&BTreeSet::new()), 1);
        assert_eq!(
            blueprint.free_instance(&set(&["ring-latch1.q", "ring-latch3.q", "q"])),
            2
        );
    }

    #[test]
    fn a_print_rewrites_the_names_its_trigger_and_script_point_at() {
        let mut blueprint = named_latch();
        blueprint.fish[0].design.program(
            "/run Q".to_string(),
            vec![
                "/nudge \"Qn\" 1 0".to_string(),
                "/say \"Q is \"Neo\"".to_string(),
                "/freeze \"q".to_string(),
            ],
        );

        let board = printed(&blueprint, 2);

        let design = &board[0].design;
        assert_eq!(design.trigger, "/run Ring Latch2 Q");
        assert_eq!(design.script[0], "/nudge \"Ring Latch2 Qn\" 1 0");
        assert_eq!(
            design.script[1], "/say \"Q is \"Neo\"",
            "only a whole quoted board name is rewritten"
        );
        assert_eq!(
            design.script[2], "/freeze \"q",
            "an unclosed quote is left alone"
        );
    }

    #[test]
    fn a_printed_design_is_marked_so_it_never_sells() {
        let blueprint = named_latch();
        assert!(!blueprint.fish[0].design.is_printed());
        assert!(
            printed(&blueprint, 1)
                .iter()
                .all(|fish| fish.design.is_printed())
        );
    }

    fn stock(items: &[(ConsumableKind, u32)]) -> HashMap<StockItem, u32> {
        items
            .iter()
            .map(|&(kind, qty)| (StockItem::Consumable(kind), qty))
            .collect()
    }

    const PLENTY: Money = 10_000;
    const ROOMY: usize = 50;

    #[test]
    fn a_print_bills_a_wafer_per_fish_and_every_installed_part() {
        assert!(
            named_latch().bill_of_materials(Fabrication::Print)
                == vec![
                    (ConsumableKind::BlankWafer, 2),
                    (ConsumableKind::Part(Part::InverterCoil), 2)
                ]
        );
    }

    #[test]
    fn a_print_spends_what_is_held_first_and_quick_buys_the_rest() {
        let held = stock(&[
            (ConsumableKind::Fabricator, 1),
            (ConsumableKind::BlankWafer, 1),
            (ConsumableKind::Part(Part::InverterCoil), 5),
        ]);
        let workshop = Workshop {
            stock: &held,
            cash: PLENTY,
            room: ROOMY,
            hosts: ROOMY,
            connected: true,
        };

        let Ok(quote) = named_latch().quote(Fabrication::Print, &workshop) else {
            panic!("a stocked workshop can print");
        };

        assert_eq!(quote.cost, ConsumableKind::BlankWafer.buy_price());
        assert_eq!(quote.materials[0].from_stock(), 1);
        assert_eq!(quote.materials[0].to_buy(), 1);
        assert_eq!(quote.materials[1].from_stock(), 2);
        assert_eq!(quote.materials[1].to_buy(), 0);
    }

    #[test]
    fn a_print_is_refused_with_a_reason_for_each_thing_missing() {
        let blueprint = named_latch();
        let nothing = stock(&[]);
        let fabricator = stock(&[(ConsumableKind::Fabricator, 1)]);
        let quote = |held: &HashMap<StockItem, u32>, cash, room| {
            blueprint
                .quote(
                    Fabrication::Print,
                    &Workshop {
                        stock: held,
                        cash,
                        room,
                        hosts: ROOMY,
                        connected: true,
                    },
                )
                .err()
        };

        assert_eq!(
            quote(&nothing, PLENTY, ROOMY),
            Some(FabricationRefusal::NeedsFabricator)
        );
        assert_eq!(
            quote(&fabricator, PLENTY, 1),
            Some(FabricationRefusal::TankFull { needed: 2 })
        );
        let full_price = 2 * ConsumableKind::BlankWafer.buy_price()
            + 2 * ConsumableKind::Part(Part::InverterCoil).buy_price();
        assert_eq!(
            quote(&fabricator, Money::from(full_price) - 1, ROOMY),
            Some(FabricationRefusal::Unaffordable { cost: full_price })
        );
        assert_eq!(quote(&fabricator, Money::from(full_price), ROOMY), None);
        assert_eq!(
            FabricationRefusal::TankFull { needed: 2 }.reason(),
            "tank full: needs room for 2 fish"
        );
    }

    #[test]
    fn an_etch_bills_twice_the_wafers_of_a_print_and_the_same_parts() {
        assert!(
            named_latch().bill_of_materials(Fabrication::Etch)
                == vec![
                    (ConsumableKind::BlankWafer, 4),
                    (ConsumableKind::Part(Part::InverterCoil), 2)
                ]
        );
    }

    #[test]
    fn a_board_holding_a_chip_bills_everything_burned_into_it() {
        let latch = named_latch();
        let mut host = BotfishState::new();
        host.install(Part::DelaySpool);
        host.etch(latch.chip("Host"));
        let board = blueprint_of(vec![host]);

        assert!(
            board.bill_of_materials(Fabrication::Print)
                == vec![
                    (ConsumableKind::BlankWafer, 1 + 4),
                    (ConsumableKind::Part(Part::InverterCoil), 2)
                ],
            "a copied chip costs what etching it cost, or chips would duplicate parts for free"
        );
    }

    #[test]
    fn a_chip_is_the_blueprint_with_its_names_pointed_at_the_host() {
        let mut blueprint = named_latch();
        blueprint.export("q");
        blueprint.fish[0]
            .design
            .program("/run Q".to_string(), vec!["/nudge \"Qn\" 1 0".to_string()]);

        let chip = blueprint.chip("Neo");

        assert_eq!(chip.name(), "Ring Latch");
        let pins: Vec<(String, PinDirection)> = chip
            .pins()
            .map(|pin| (pin.name.into_owned(), pin.direction))
            .collect();
        assert_eq!(
            pins,
            vec![
                ("r".to_string(), PinDirection::In),
                ("s".to_string(), PinDirection::In),
                ("q".to_string(), PinDirection::Out),
            ]
        );
        let inner = &chip.board()[0];
        assert_eq!(inner.trigger, "/run Neo");
        assert_eq!(inner.script[0], "/nudge \"Neo\" 1 0");
        assert_eq!(inner.drives(), Some("q"), "insides are never namespaced");
        assert!(!inner.is_printed());
    }

    #[test]
    fn an_unconnected_workshop_prints_from_stock_but_never_quick_buys() {
        let blueprint = named_latch();
        let quote = |held: &HashMap<StockItem, u32>| {
            blueprint
                .quote(
                    Fabrication::Print,
                    &Workshop {
                        stock: held,
                        cash: PLENTY,
                        room: ROOMY,
                        hosts: ROOMY,
                        connected: false,
                    },
                )
                .err()
        };

        let short = stock(&[
            (ConsumableKind::Fabricator, 1),
            (ConsumableKind::BlankWafer, 1),
            (ConsumableKind::Part(Part::InverterCoil), 2),
        ]);
        assert_eq!(
            quote(&short),
            Some(FabricationRefusal::NotConnected),
            "a missing wafer cannot be quick-bought off the bench"
        );

        let stocked = stock(&[
            (ConsumableKind::Fabricator, 1),
            (ConsumableKind::BlankWafer, 2),
            (ConsumableKind::Part(Part::InverterCoil), 2),
        ]);
        assert_eq!(
            quote(&stocked),
            None,
            "held stock alone still prints unconnected"
        );
    }

    #[test]
    fn an_etch_is_refused_when_no_botfish_can_take_it() {
        let held = stock(&[(ConsumableKind::Fabricator, 1)]);
        let workshop = Workshop {
            stock: &held,
            cash: PLENTY,
            room: 0,
            hosts: 0,
            connected: true,
        };

        assert_eq!(
            named_latch().quote(Fabrication::Etch, &workshop).err(),
            Some(FabricationRefusal::NoHost)
        );
        assert!(
            named_latch()
                .quote(
                    Fabrication::Etch,
                    &Workshop {
                        hosts: 1,
                        ..workshop
                    }
                )
                .is_ok(),
            "an etch needs a botfish, never tank room"
        );
    }

    #[test]
    fn a_channel_heard_under_two_spellings_is_still_one_wire() {
        let board = [
            gate(&[], "Carry  In", &[]),
            gate(&["carry in"], "sum", COIL),
        ];

        let pins = BlueprintPins::of(&board);

        assert_eq!(pins.internals, set(&["carry in"]));
        assert_eq!(pins.outputs, set(&["sum"]));
    }
}
