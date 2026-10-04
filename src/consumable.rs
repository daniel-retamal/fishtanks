use rand::RngExt;
use serde::{Deserialize, Serialize};

use crate::fishes::fish::Fish;
use crate::fishes::mutations::{Mutation, apply_mutation_to_fish};
use crate::fishes::parts::Part;
use crate::loot::{ConsumableKind, MilkVariant, StockItem};
use crate::restore::Restorable;
use crate::ui::hints::{HINT_ENTER_CONSUME, HINT_ENTER_ETCH, HINT_ENTER_INSTALL};

pub const COFFEE_DURATION: f32 = 60.0;
pub const BAIT_DURATION: f32 = 60.0;
pub const CASTS_PER_BUFF: u32 = 5;

pub const COFFEE_SPEED_MULT: f32 = 0.8;
pub const COFFEE_SWAY_MULT: f32 = 0.7;
pub const COFFEE_ZOOMIE_DT_MULT: f32 = 0.3;

const IRRADIATED_MILK_MUTATIONS_MIN: u32 = 10;
const IRRADIATED_MILK_MUTATIONS_MAX: u32 = 15;

pub const VISUAL_CALCULUS_ALPHA: f32 = 0.175;
pub const VOLITION_ALPHA: f32 = 0.9;
pub const PHYSICAL_INSTRUMENT_ALPHA: f32 = 0.9;
pub const REACTION_SPEED_ALPHA: f32 = 0.9;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum MilkStatus {
    VisualCalculus,
    Volition,
    PhysicalInstrument,
    ReactionSpeed,
}

impl MilkStatus {
    pub const ALL: &'static [MilkStatus] = &[
        MilkStatus::VisualCalculus,
        MilkStatus::Volition,
        MilkStatus::PhysicalInstrument,
        MilkStatus::ReactionSpeed,
    ];

    pub fn display_name(self) -> &'static str {
        match self {
            MilkStatus::VisualCalculus => "visual-calculus",
            MilkStatus::Volition => "volition",
            MilkStatus::PhysicalInstrument => "physical-instrument",
            MilkStatus::ReactionSpeed => "reaction-speed",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Measure {
    Seconds(f32),
    Casts(u32),
}

impl Measure {
    pub fn sooner(self, other: Measure) -> Measure {
        match (self, other) {
            (Measure::Seconds(a), Measure::Seconds(b)) => Measure::Seconds(a.min(b)),
            (Measure::Casts(a), Measure::Casts(b)) => Measure::Casts(a.min(b)),
            (mine, _) => mine,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Caster {
    Angler,
    Rig,
}

pub fn full_casts() -> u32 {
    CASTS_PER_BUFF
}

pub trait Buff {
    fn label(&self) -> &'static str;
    fn stacks(&self) -> u32;
    fn full_measure(&self) -> Measure;
    fn improves(&self, caster: Caster) -> bool;
    fn clocks(&self) -> (f32, u32);
    fn clocks_mut(&mut self) -> (&mut f32, &mut u32);

    fn left(&self) -> Measure {
        let (secs, casts) = self.clocks();
        match self.full_measure() {
            Measure::Seconds(_) => Measure::Seconds(secs),
            Measure::Casts(_) => Measure::Casts(casts),
        }
    }

    fn tick(&mut self, dt: f32) {
        if let Measure::Seconds(_) = self.full_measure() {
            *self.clocks_mut().0 -= dt;
        }
    }

    fn spend_cast(&mut self, caster: Caster) {
        if !matches!(self.full_measure(), Measure::Casts(_)) || !self.improves(caster) {
            return;
        }
        let casts = self.clocks_mut().1;
        *casts = casts.saturating_sub(1);
    }

    fn is_spent(&self) -> bool {
        match self.left() {
            Measure::Seconds(secs) => secs <= 0.0,
            Measure::Casts(casts) => casts == 0,
        }
    }
}

fn clocks_for(measure: Measure) -> (f32, u32) {
    match measure {
        Measure::Seconds(secs) => (secs, 0),
        Measure::Casts(casts) => (0.0, casts),
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ActiveMilkStatus {
    pub kind: MilkStatus,
    pub stacks: u32,
    pub time_remaining: f32,
    #[serde(default = "full_casts")]
    pub casts_left: u32,
}

impl ActiveMilkStatus {
    pub fn fresh(kind: MilkStatus) -> Self {
        let (time_remaining, casts_left) = clocks_for(Measure::Casts(CASTS_PER_BUFF));
        Self {
            kind,
            stacks: 1,
            time_remaining,
            casts_left,
        }
    }
}

impl Buff for ActiveMilkStatus {
    fn label(&self) -> &'static str {
        self.kind.display_name()
    }

    fn stacks(&self) -> u32 {
        self.stacks
    }

    fn full_measure(&self) -> Measure {
        Measure::Casts(CASTS_PER_BUFF)
    }

    fn improves(&self, caster: Caster) -> bool {
        caster == Caster::Angler
    }

    fn clocks(&self) -> (f32, u32) {
        (self.time_remaining, self.casts_left)
    }

    fn clocks_mut(&mut self) -> (&mut f32, &mut u32) {
        (&mut self.time_remaining, &mut self.casts_left)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ActiveConsumable {
    pub kind: ConsumableKind,
    pub stacks: u32,
    pub time_remaining: f32,
    #[serde(default = "full_casts")]
    pub casts_left: u32,
}

impl ActiveConsumable {
    pub fn fresh(kind: ConsumableKind) -> Option<Self> {
        let (time_remaining, casts_left) = clocks_for(kind.active_measure()?);
        Some(Self {
            kind,
            stacks: 1,
            time_remaining,
            casts_left,
        })
    }
}

impl Buff for ActiveConsumable {
    fn label(&self) -> &'static str {
        self.kind.active_label().unwrap_or_default()
    }

    fn stacks(&self) -> u32 {
        self.stacks
    }

    fn full_measure(&self) -> Measure {
        self.kind.active_measure().unwrap_or(Measure::Seconds(0.0))
    }

    fn improves(&self, _caster: Caster) -> bool {
        true
    }

    fn clocks(&self) -> (f32, u32) {
        (self.time_remaining, self.casts_left)
    }

    fn clocks_mut(&mut self) -> (&mut f32, &mut u32) {
        (&mut self.time_remaining, &mut self.casts_left)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConsumeTarget {
    Milk(MilkVariant),
    Part(Part),
    Etch(usize),
}

impl ConsumeTarget {
    pub fn display_name(self) -> &'static str {
        match self {
            ConsumeTarget::Milk(variant) => variant.display_name(),
            ConsumeTarget::Part(part) => part.display_name(),
            ConsumeTarget::Etch(_) => ConsumableKind::Fabricator.display_name(),
        }
    }

    pub fn stock(self) -> StockItem {
        match self {
            ConsumeTarget::Milk(variant) => StockItem::Consumable(ConsumableKind::Milk(variant)),
            ConsumeTarget::Part(part) => StockItem::Consumable(ConsumableKind::Part(part)),
            ConsumeTarget::Etch(_) => StockItem::Consumable(ConsumableKind::Fabricator),
        }
    }

    pub fn header(self, name: &str, remaining: u32) -> String {
        match self {
            ConsumeTarget::Milk(_) => format!(" {name} to the fishes ({remaining}) "),
            ConsumeTarget::Part(_) | ConsumeTarget::Etch(_) => {
                format!(" {name} to the botfishes ({remaining}) ")
            }
        }
    }

    pub fn confirm_hint(self) -> &'static str {
        match self {
            ConsumeTarget::Milk(_) => HINT_ENTER_CONSUME,
            ConsumeTarget::Part(_) => HINT_ENTER_INSTALL,
            ConsumeTarget::Etch(_) => HINT_ENTER_ETCH,
        }
    }

    pub fn fattens(self) -> bool {
        match self {
            ConsumeTarget::Milk(variant) => variant.weight_gain_g() > 0,
            ConsumeTarget::Part(_) | ConsumeTarget::Etch(_) => false,
        }
    }

    pub fn accepts(self, fish: &Fish) -> bool {
        if fish.unfish_state.is_some() {
            return false;
        }
        match self {
            ConsumeTarget::Milk(_) => true,
            ConsumeTarget::Part(_) | ConsumeTarget::Etch(_) => fish.is_programmable(),
        }
    }

    pub fn apply_to(self, fish: &mut Fish, rng: &mut impl RngExt) -> bool {
        match self {
            ConsumeTarget::Milk(variant) => {
                apply_milk_to_fish(variant, fish, rng);
                true
            }
            ConsumeTarget::Part(part) => fish.script_mut().is_some_and(|bot| bot.install(part)),
            ConsumeTarget::Etch(_) => false,
        }
    }
}

pub fn apply_milk_to_fish(variant: MilkVariant, fish: &mut Fish, rng: &mut impl RngExt) {
    match variant {
        MilkVariant::Plain | MilkVariant::Blueberry | MilkVariant::Honey | MilkVariant::Matcha => {}
        MilkVariant::Chocolate => {
            fish.weight_g = fish.weight_g.saturating_add(variant.weight_gain_g());
            apply_mutation_to_fish(fish, Mutation::SizeIncrease, rng);
        }
        MilkVariant::Strawberry => {
            apply_mutation_to_fish(fish, Mutation::Strawberry, rng);
        }
        MilkVariant::Vanilla => fish.restore(),
        MilkVariant::Alien => {
            apply_mutation_to_fish(fish, Mutation::Alienation, rng);
        }
        MilkVariant::Irradiated => {
            fish.pending_rad_mutations +=
                rng.random_range(IRRADIATED_MILK_MUTATIONS_MIN..=IRRADIATED_MILK_MUTATIONS_MAX);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_old_timed_bait_or_milk_stack_loads_as_that_many_stacks_of_full_casts() {
        let bait: ActiveConsumable =
            ron::from_str("(kind: Bait, stacks: 3, time_remaining: 40.0)").expect("an old bait");
        assert_eq!(
            (bait.stacks, bait.left()),
            (3, Measure::Casts(CASTS_PER_BUFF))
        );
        let milk: ActiveMilkStatus =
            ron::from_str("(kind: Volition, stacks: 2, time_remaining: 12.5)")
                .expect("an old glass");
        assert_eq!(
            (milk.stacks, milk.left()),
            (2, Measure::Casts(CASTS_PER_BUFF))
        );
        let coffee: ActiveConsumable =
            ron::from_str("(kind: Coffee, stacks: 1, time_remaining: 60.0)").expect("a cup");
        assert_eq!(coffee.left(), Measure::Seconds(60.0));
    }

    #[test]
    fn a_cast_spends_a_milk_only_for_the_angler_who_drank_it() {
        let mut glass = ActiveMilkStatus::fresh(MilkStatus::Volition);
        glass.spend_cast(Caster::Rig);
        assert_eq!(glass.left(), Measure::Casts(CASTS_PER_BUFF));
        glass.spend_cast(Caster::Angler);
        assert_eq!(glass.left(), Measure::Casts(CASTS_PER_BUFF - 1));
    }

    #[test]
    fn coffee_is_spent_by_time_and_never_by_a_cast() {
        let mut cup = ActiveConsumable::fresh(ConsumableKind::Coffee).expect("coffee is a buff");
        cup.spend_cast(Caster::Angler);
        assert_eq!(cup.left(), Measure::Seconds(COFFEE_DURATION));
        cup.tick(COFFEE_DURATION);
        assert!(cup.is_spent());
    }
}
