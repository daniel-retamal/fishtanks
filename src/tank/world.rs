use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use super::{Link, Tank};
use crate::economy;
use crate::fishes::botfish::BotfishState;
use crate::fishes::fish::{Fed, Fish};
use crate::fishes::species::FishSpecies;

pub const HOURS_PER_DAY: u32 = 24;
pub const DAY_LENGTH_SECS: f32 = 1440.0;
pub const HOUR_SECS: f32 = DAY_LENGTH_SECS / HOURS_PER_DAY as f32;
pub const DAWN_HOUR: u32 = 6;
pub const DUSK_HOUR: u32 = 20;
pub const LUNAR_MONTH_DAYS: u32 = 8;
pub const FULL_MOON: u32 = LUNAR_MONTH_DAYS / 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sky {
    pub daylight: bool,
    pub moon: u32,
    pub calm: bool,
}

impl Default for Sky {
    fn default() -> Self {
        Self {
            daylight: true,
            moon: FULL_MOON,
            calm: false,
        }
    }
}

impl Sky {
    pub fn is_night(self) -> bool {
        !self.daylight
    }

    pub fn lights(self, cell: usize, width: usize) -> bool {
        let phase = self.moon % LUNAR_MONTH_DAYS;
        if width == 0 {
            return false;
        }
        let waxing = phase <= FULL_MOON;
        let days_lit = if waxing {
            phase
        } else {
            LUNAR_MONTH_DAYS - phase
        };
        let lit = (width as u32 * days_lit).div_ceil(FULL_MOON) as usize;
        if waxing {
            cell >= width.saturating_sub(lit)
        } else {
            cell < lit
        }
    }
}
pub const RAD_TANK_RADS: u32 = 10;
pub const SELECTOR_OPEN: char = '{';
pub const SELECTOR_CLOSE: char = '}';

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum WorldSignal {
    Death,
    Birth,
    Sale,
    Catch,
    Abduction,
    Mutation,
    Dawn,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Superlative {
    Heaviest,
    Lightest,
    Richest,
    Cheapest,
}

impl Superlative {
    const ALL: &'static [Superlative] = &[
        Superlative::Heaviest,
        Superlative::Lightest,
        Superlative::Richest,
        Superlative::Cheapest,
    ];

    pub fn token(self) -> &'static str {
        match self {
            Superlative::Heaviest => "heaviest",
            Superlative::Lightest => "lightest",
            Superlative::Richest => "richest",
            Superlative::Cheapest => "cheapest",
        }
    }

    pub fn parse(word: &str) -> Option<Superlative> {
        let lower = word.to_ascii_lowercase();
        Superlative::ALL
            .iter()
            .copied()
            .find(|s| s.token() == lower)
    }

    fn ranks_above(self, candidate: &SensedFish, best: &SensedFish) -> bool {
        match self {
            Superlative::Heaviest => candidate.weight_g > best.weight_g,
            Superlative::Lightest => candidate.weight_g < best.weight_g,
            Superlative::Richest => candidate.value > best.value,
            Superlative::Cheapest => candidate.value < best.value,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SensedFish {
    pub name: String,
    pub species: FishSpecies,
    pub weight_g: u32,
    pub value: u32,
    pub full: bool,
}

impl SensedFish {
    pub fn of(fish: &Fish) -> Self {
        Self {
            name: fish.name.clone(),
            species: fish.species,
            weight_g: fish.weight_g,
            value: economy::reading(fish.sell_value()),
            full: fish.fed() == Fed::Full,
        }
    }
}

fn quoted_if_spaced(name: &str) -> String {
    if name.chars().any(char::is_whitespace) {
        return format!("\"{name}\"");
    }
    name.to_string()
}

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Selector {
    superlative: Option<Superlative>,
    species: Option<FishSpecies>,
    name: Option<String>,
}

impl Selector {
    pub fn parse(text: &str) -> Option<Selector> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        let superlative = Superlative::parse(words[0]);
        let rest = if superlative.is_some() {
            words[1..].join(" ")
        } else {
            words.join(" ")
        };
        if rest.is_empty() {
            return Some(Selector {
                superlative,
                species: None,
                name: None,
            });
        }
        if let Some(species) = FishSpecies::parse(&rest) {
            return Some(Selector {
                superlative,
                species: Some(species),
                name: None,
            });
        }
        if superlative.is_some() {
            return None;
        }
        Some(Selector {
            superlative: None,
            species: None,
            name: Some(rest),
        })
    }

    pub fn admits(&self, fish: &SensedFish) -> bool {
        if let Some(species) = self.species
            && fish.species != species
        {
            return false;
        }
        let Some(name) = self.name.as_deref() else {
            return true;
        };
        fish.name.eq_ignore_ascii_case(name)
    }

    pub fn count(&self, shoal: &[SensedFish]) -> u32 {
        shoal.iter().filter(|fish| self.admits(fish)).count() as u32
    }

    pub fn expand(command: &str, shoal: &[SensedFish]) -> Option<String> {
        let mut expanded = String::new();
        let mut rest = command;
        while let Some(open) = rest.find(SELECTOR_OPEN) {
            let close = open + rest[open..].find(SELECTOR_CLOSE)?;
            expanded.push_str(&rest[..open]);
            let picked = Selector::parse(&rest[open + 1..close])?.resolve(shoal)?;
            expanded.push_str(&quoted_if_spaced(&picked.name));
            rest = &rest[close + 1..];
        }
        expanded.push_str(rest);
        Some(expanded)
    }

    pub fn resolve<'a>(&self, shoal: &'a [SensedFish]) -> Option<&'a SensedFish> {
        let mut admitted = shoal.iter().filter(|fish| self.admits(fish));
        let first = admitted.next()?;
        let Some(superlative) = self.superlative else {
            return Some(first);
        };
        Some(admitted.fold(first, |best, fish| {
            if superlative.ranks_above(fish, best) {
                return fish;
            }
            best
        }))
    }
}

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct WorldView {
    shoal: Vec<SensedFish>,
    capacity: usize,
    cash: u32,
    tank_value: u32,
    rads: u32,
    hour: u32,
    signals: BTreeSet<WorldSignal>,
    relayed: BTreeMap<Link, bool>,
    bait: u32,
}

impl WorldView {
    pub fn new(
        shoal: Vec<SensedFish>,
        capacity: usize,
        cash: u32,
        rads: u32,
        hour: u32,
        signals: BTreeSet<WorldSignal>,
    ) -> Self {
        let tank_value = shoal.iter().map(|fish| fish.value).sum();
        Self {
            shoal,
            capacity,
            cash,
            tank_value,
            rads,
            hour,
            signals,
            relayed: BTreeMap::new(),
            bait: 0,
        }
    }

    pub fn with_bait(mut self, bait: u32) -> Self {
        self.bait = bait;
        self
    }

    pub fn bait(&self) -> u32 {
        self.bait
    }

    pub fn tuned_to(mut self, links: impl IntoIterator<Item = Link>) -> Self {
        for link in links {
            self.relayed.insert(link, false);
        }
        self
    }

    pub fn tune_in(&mut self, tanks: &[Tank]) {
        for (link, level) in &mut self.relayed {
            *level = link.level_in(tanks);
        }
    }

    pub fn relayed(&self, link: &Link) -> bool {
        self.relayed.get(link).copied().unwrap_or(false)
    }

    pub fn unobserved(signals: BTreeSet<WorldSignal>) -> Self {
        Self {
            signals,
            ..Self::default()
        }
    }

    pub fn shoal(&self) -> &[SensedFish] {
        &self.shoal
    }

    pub fn is_full(&self) -> bool {
        self.shoal.len() >= self.capacity
    }

    pub fn cash(&self) -> u32 {
        self.cash
    }

    pub fn tank_value(&self) -> u32 {
        self.tank_value
    }

    pub fn rads(&self) -> u32 {
        self.rads
    }

    pub fn hour(&self) -> u32 {
        self.hour
    }

    pub fn is_night(&self) -> bool {
        self.hour >= DUSK_HOUR || self.hour < DAWN_HOUR
    }

    pub fn pulsed(&self, signal: WorldSignal) -> bool {
        self.signals.contains(&signal)
    }

    pub fn settle(&mut self) {
        self.signals.clear();
    }
}

impl Tank {
    pub fn signal(&mut self, signal: WorldSignal) {
        self.pending_signals.insert(signal);
    }

    pub fn rads(&self) -> u32 {
        let pressure: u32 = self.fish.iter().map(Fish::auto_mutate_stacks).sum();
        if self.kind.config().auto_mutate_all {
            return pressure + RAD_TANK_RADS;
        }
        pressure
    }

    pub fn reads_world(&self) -> bool {
        self.fish
            .iter()
            .any(|fish| fish.script().is_some_and(BotfishState::reads_world))
    }

    pub fn shoal(&self) -> Vec<SensedFish> {
        self.fish.iter().map(SensedFish::of).collect()
    }

    pub fn tunings(&self) -> Vec<Link> {
        self.fish
            .iter()
            .filter_map(Fish::script)
            .flat_map(BotfishState::links)
            .collect()
    }

    pub fn observe(&mut self, cash: u32, hour: u32) -> WorldView {
        let signals = std::mem::take(&mut self.pending_signals);
        if !self.reads_world() {
            return WorldView::unobserved(signals);
        }
        WorldView::new(
            self.shoal(),
            self.capacity(),
            cash,
            self.rads(),
            hour,
            signals,
        )
        .tuned_to(self.tunings())
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct DayClock {
    elapsed: f32,
    #[serde(default)]
    days: u32,
}

impl DayClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn hour(&self) -> u32 {
        (self.elapsed / HOUR_SECS) as u32 % HOURS_PER_DAY
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        let was = self.hour();
        let reached = self.elapsed + dt;
        if reached >= DAY_LENGTH_SECS {
            self.days = self.days.wrapping_add((reached / DAY_LENGTH_SECS) as u32);
        }
        self.elapsed = reached.rem_euclid(DAY_LENGTH_SECS);
        let now = self.hour();
        now != was && now == DAWN_HOUR
    }

    pub fn is_daylight(&self) -> bool {
        (DAWN_HOUR..DUSK_HOUR).contains(&self.hour())
    }

    pub fn sky(&self, calm: bool) -> Sky {
        Sky {
            daylight: self.is_daylight(),
            moon: self.days % LUNAR_MONTH_DAYS,
            calm,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sensed(name: &str, species: FishSpecies, weight_g: u32, value: u32) -> SensedFish {
        SensedFish {
            name: name.to_string(),
            species,
            weight_g,
            value,
            full: false,
        }
    }

    fn shoal() -> Vec<SensedFish> {
        vec![
            sensed("Ann", FishSpecies::Merluza, 400, 90),
            sensed("Bob", FishSpecies::Betta, 900, 20),
            sensed("Cid", FishSpecies::Merluza, 100, 500),
        ]
    }

    #[test]
    fn a_blank_target_selects_nothing() {
        assert_eq!(Selector::parse("   "), None);
    }

    #[test]
    fn a_bare_superlative_ranges_over_the_whole_tank() {
        let selector = Selector::parse("heaviest").expect("a superlative is a target");
        assert_eq!(
            selector.resolve(&shoal()).map(|f| f.name.as_str()),
            Some("Bob")
        );
        assert_eq!(
            Selector::parse("lightest")
                .unwrap()
                .resolve(&shoal())
                .map(|f| f.name.as_str()),
            Some("Cid")
        );
    }

    #[test]
    fn worth_and_weight_rank_differently() {
        assert_eq!(
            Selector::parse("richest")
                .unwrap()
                .resolve(&shoal())
                .map(|f| f.name.as_str()),
            Some("Cid")
        );
        assert_eq!(
            Selector::parse("cheapest")
                .unwrap()
                .resolve(&shoal())
                .map(|f| f.name.as_str()),
            Some("Bob")
        );
    }

    #[test]
    fn a_species_narrows_the_field_the_superlative_ranges_over() {
        let selector = Selector::parse("heaviest merluza").expect("a species narrows a target");
        assert_eq!(
            selector.resolve(&shoal()).map(|f| f.name.as_str()),
            Some("Ann"),
            "Bob is heavier but is not a merluza"
        );
        assert_eq!(selector.count(&shoal()), 2);
    }

    #[test]
    fn a_name_selects_exactly_that_fish_however_it_is_typed() {
        let selector = Selector::parse("cid").expect("a name is a target");
        assert_eq!(
            selector.resolve(&shoal()).map(|f| f.name.as_str()),
            Some("Cid")
        );
        assert_eq!(selector.count(&shoal()), 1);
    }

    #[test]
    fn a_superlative_over_a_name_is_not_a_target() {
        assert_eq!(Selector::parse("heaviest Cid"), None);
    }

    #[test]
    fn a_target_that_matches_nothing_resolves_to_nothing() {
        let selector = Selector::parse("ghost").expect("a name is a target");
        assert_eq!(selector.resolve(&shoal()), None);
        assert_eq!(selector.count(&shoal()), 0);
    }

    #[test]
    fn a_species_alone_counts_without_ranking() {
        let selector = Selector::parse("merluza").expect("a species is a target");
        assert_eq!(selector.count(&shoal()), 2);
        assert_eq!(
            selector.resolve(&shoal()).map(|f| f.name.as_str()),
            Some("Ann"),
            "with no superlative the first one admitted wins"
        );
    }

    #[test]
    fn a_line_with_no_selector_is_handed_through_word_for_word() {
        assert_eq!(
            Selector::expand("/sell \"Cid\"", &shoal()),
            Some("/sell \"Cid\"".to_string())
        );
    }

    #[test]
    fn a_selector_becomes_the_name_of_the_fish_it_picks() {
        assert_eq!(
            Selector::expand("/sell {richest merluza}", &shoal()),
            Some("/sell Cid".to_string())
        );
        assert_eq!(
            Selector::expand("/move {heaviest} \"Annex\"", &shoal()),
            Some("/move Bob \"Annex\"".to_string())
        );
    }

    #[test]
    fn two_selectors_in_one_line_both_resolve() {
        assert_eq!(
            Selector::expand("/mutate {lightest} {heaviest}", &shoal()),
            Some("/mutate Cid Bob".to_string())
        );
    }

    #[test]
    fn a_name_with_a_space_comes_back_quoted_so_it_stays_one_argument() {
        let crowd = vec![sensed("Big Bob", FishSpecies::Betta, 900, 20)];
        assert_eq!(
            Selector::expand("/clone {heaviest}", &crowd),
            Some("/clone \"Big Bob\"".to_string())
        );
    }

    #[test]
    fn a_selector_that_matches_nothing_leaves_no_line_to_run() {
        assert_eq!(Selector::expand("/sell {richest snapper}", &shoal()), None);
        assert_eq!(Selector::expand("/sell {heaviest}", &[]), None);
    }

    #[test]
    fn an_empty_selector_leaves_no_line_to_run() {
        assert_eq!(Selector::expand("/sell {}", &shoal()), None);
        assert_eq!(Selector::expand("/sell {   }", &shoal()), None);
    }

    #[test]
    fn a_selector_nobody_closed_leaves_no_line_to_run() {
        assert_eq!(Selector::expand("/sell {richest", &shoal()), None);
    }

    #[test]
    fn a_mistyped_superlative_poisons_the_line_instead_of_picking_a_fish() {
        assert_eq!(Selector::expand("/sell {heaviest Cid}", &shoal()), None);
    }

    #[test]
    fn a_view_adds_up_what_the_tank_is_worth() {
        let view = WorldView::new(shoal(), 50, 700, 0, 0, BTreeSet::new());
        assert_eq!(view.tank_value(), 610);
        assert_eq!(view.cash(), 700);
        assert!(!view.is_full());
    }

    #[test]
    fn a_view_is_full_when_the_shoal_reaches_capacity() {
        let view = WorldView::new(shoal(), 3, 0, 0, 0, BTreeSet::new());
        assert!(view.is_full());
    }

    #[test]
    fn a_pulse_is_gone_once_the_stage_settles() {
        let mut view = WorldView::new(
            Vec::new(),
            1,
            0,
            0,
            0,
            BTreeSet::from([WorldSignal::Death, WorldSignal::Sale]),
        );
        assert!(view.pulsed(WorldSignal::Death));
        assert!(!view.pulsed(WorldSignal::Birth));

        view.settle();

        assert!(!view.pulsed(WorldSignal::Death), "a pulse lasts one stage");
        assert!(!view.pulsed(WorldSignal::Sale));
    }

    #[test]
    fn the_night_runs_from_dusk_to_dawn() {
        let night_at = |hour| WorldView::new(Vec::new(), 1, 0, 0, hour, BTreeSet::new()).is_night();
        assert!(night_at(DUSK_HOUR));
        assert!(night_at(23));
        assert!(night_at(0));
        assert!(night_at(DAWN_HOUR - 1));
        assert!(!night_at(DAWN_HOUR));
        assert!(!night_at(12));
        assert!(!night_at(DUSK_HOUR - 1));
    }

    #[test]
    fn a_day_walks_through_every_hour_and_comes_back_round() {
        let mut clock = DayClock::new();
        assert_eq!(clock.hour(), 0);
        for hour in 1..HOURS_PER_DAY {
            clock.tick(HOUR_SECS);
            assert_eq!(clock.hour(), hour);
        }
        clock.tick(HOUR_SECS);
        assert_eq!(clock.hour(), 0, "midnight comes round again");
    }

    #[test]
    fn dawn_breaks_once_and_only_on_the_turn() {
        let mut clock = DayClock::new();
        let mut breaks = 0;
        for _ in 0..HOURS_PER_DAY {
            if clock.tick(HOUR_SECS) {
                assert_eq!(clock.hour(), DAWN_HOUR);
                breaks += 1;
            }
        }
        assert_eq!(breaks, 1, "one dawn a day");
    }

    #[test]
    fn an_hour_that_does_not_turn_never_breaks_dawn() {
        let mut clock = DayClock::new();
        let stride = HOUR_SECS / 4.0;
        for _ in 0..(HOURS_PER_DAY * 4) {
            if clock.tick(stride) {
                assert_eq!(clock.hour(), DAWN_HOUR, "only the turn itself pulses");
            }
        }
    }
}
