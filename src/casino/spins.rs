use std::sync::OnceLock;

use rand::RngExt;

use super::Multiple;

pub const STOPS: usize = 100;
pub const REELS: usize = 3;
pub const ROWS: usize = 3;
pub const CELLS: usize = REELS * ROWS;
const MIDDLE: usize = 1;
const PEARL_STRIDE: usize = STOPS / PEARL_STOPS;
const PEARL_STOPS: usize = 5;
pub const PEARLS_TO_DIVE: usize = 3;
pub const RESPINS: u8 = 3;
pub const NEW_PEARL_PER_MILLE: u32 = 100;
const PER_MILLE: u32 = 1000;
const REEL_SPEED: f32 = 26.0;
const REEL_SPEED_STEP: f32 = 2.0;
const FIRST_STOP_SECS: f32 = 0.75;
const STOP_GAP_SECS: f32 = 0.4;
const SUSPENSE_SECS: f32 = 1.3;
const SUSPENSE_FROM_SECS: f32 = 1.2;
const SUSPENSE_SLOWDOWN: f32 = 0.55;
const SETTLE_SECS: f32 = 0.38;
const OVERSHOOT: f32 = 1.8;
const LANDING_STOPS: f32 = 4.0;
pub const RESPIN_SECS: f32 = 0.9;
const DIVE_CLOSE_SECS: f32 = 1.2;
const PEARL_GLOW_SECS: f32 = 1.4;
const PEARL_OPEN_SECS: f32 = 0.5;
const DIVE_RULES_SECS: f32 = 1.8;
const SHOAL: Multiple = Multiple::whole(3);
const TWO_ANCHOVETAS: Multiple = Multiple::whole(2);
const ONE_ANCHOVETA: Multiple = Multiple::whole(1);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Symbol {
    Bubbles,
    Anchoveta,
    Merluza,
    Pufferfish,
    Coffee,
    Cash,
    Pearl,
}

impl Symbol {
    pub const ALL: [Symbol; 7] = [
        Symbol::Bubbles,
        Symbol::Anchoveta,
        Symbol::Merluza,
        Symbol::Pufferfish,
        Symbol::Coffee,
        Symbol::Cash,
        Symbol::Pearl,
    ];

    pub fn stops(self) -> usize {
        match self {
            Symbol::Bubbles => 36,
            Symbol::Anchoveta => 8,
            Symbol::Merluza => 18,
            Symbol::Pufferfish => 14,
            Symbol::Coffee => 12,
            Symbol::Cash => 7,
            Symbol::Pearl => PEARL_STOPS,
        }
    }

    pub fn three(self) -> Option<Multiple> {
        match self {
            Symbol::Bubbles => Some(Multiple::whole(5)),
            Symbol::Anchoveta => Some(Multiple::whole(25)),
            Symbol::Merluza => Some(Multiple::whole(10)),
            Symbol::Pufferfish => Some(Multiple::whole(20)),
            Symbol::Coffee => Some(Multiple::whole(40)),
            Symbol::Cash => Some(Multiple::whole(100)),
            Symbol::Pearl => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Symbol::Bubbles => "Bubbles",
            Symbol::Anchoveta => "Anchoveta",
            Symbol::Merluza => "Merluza",
            Symbol::Pufferfish => "Pufferfish",
            Symbol::Coffee => "Coffee",
            Symbol::Cash => "Cash",
            Symbol::Pearl => "Golden Pearl",
        }
    }

    pub fn is_fish(self) -> bool {
        matches!(
            self,
            Symbol::Anchoveta | Symbol::Merluza | Symbol::Pufferfish
        )
    }
}

pub fn strip() -> &'static [Symbol; STOPS] {
    static STRIP: OnceLock<[Symbol; STOPS]> = OnceLock::new();
    STRIP.get_or_init(build_strip)
}

fn build_strip() -> [Symbol; STOPS] {
    let mut strip = [Symbol::Bubbles; STOPS];
    let open: Vec<usize> = (0..STOPS).filter(|i| i % PEARL_STRIDE != 0).collect();
    let mut placed: Vec<(f32, usize, Symbol)> = Vec::new();
    for (order, symbol) in Symbol::ALL.iter().enumerate() {
        if *symbol == Symbol::Pearl {
            continue;
        }
        let count = symbol.stops();
        for k in 0..count {
            let spot = (k as f32 + 0.5 + order as f32 * 0.13) / count as f32;
            placed.push((spot, order, *symbol));
        }
    }
    placed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));
    for (slot, (_, _, symbol)) in open.iter().zip(placed) {
        strip[*slot] = symbol;
    }
    for i in (0..STOPS).step_by(PEARL_STRIDE) {
        strip[i] = Symbol::Pearl;
    }
    strip
}

pub fn at(stop: isize) -> Symbol {
    strip()[stop.rem_euclid(STOPS as isize) as usize]
}

pub fn window(stops: [usize; REELS]) -> [[Symbol; ROWS]; REELS] {
    stops.map(|stop| {
        let s = stop as isize;
        [at(s - 1), at(s), at(s + 1)]
    })
}

pub fn line_pay(line: [Symbol; REELS]) -> (Multiple, String) {
    if line.contains(&Symbol::Pearl) {
        return (Multiple::ZERO, String::new());
    }
    let [a, b, c] = line;
    if a == b
        && b == c
        && let Some(pays) = a.three()
    {
        return (pays, format!("3 {}", a.name()));
    }
    if line.iter().all(|s| s.is_fish()) {
        return (SHOAL, "a shoal".to_string());
    }
    match line.iter().filter(|s| **s == Symbol::Anchoveta).count() {
        2 => (TWO_ANCHOVETAS, "2 Anchoveta".to_string()),
        1 => (ONE_ANCHOVETA, "Anchoveta".to_string()),
        _ => (Multiple::ZERO, String::new()),
    }
}

pub fn pearls_in(window: &[[Symbol; ROWS]; REELS]) -> usize {
    window
        .iter()
        .flatten()
        .filter(|s| **s == Symbol::Pearl)
        .count()
}

pub const PEARL_VALUES: [(u32, u32); 7] = [
    (1, 30),
    (2, 25),
    (3, 18),
    (5, 12),
    (10, 8),
    (25, 5),
    (50, 2),
];

pub fn pearl_value(rng: &mut impl RngExt) -> Multiple {
    let total: u32 = PEARL_VALUES.iter().map(|(_, w)| w).sum();
    let mut draw = rng.random_range(0..total);
    for (value, weight) in PEARL_VALUES {
        if draw < weight {
            return Multiple::whole(value.into());
        }
        draw -= weight;
    }
    Multiple::ONE
}

pub fn mean_pearl() -> f64 {
    let total: u32 = PEARL_VALUES.iter().map(|(_, w)| w).sum();
    PEARL_VALUES
        .iter()
        .map(|(v, w)| f64::from(*v) * f64::from(*w))
        .sum::<f64>()
        / f64::from(total)
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Reel {
    pub pos: f32,
    pub spinning: bool,
    stop_at: f32,
    settling: Option<(f32, f32, f32)>,
    speed: f32,
    pub target: usize,
}

impl Reel {
    fn resting(stop: usize) -> Self {
        Self {
            pos: stop as f32,
            spinning: false,
            stop_at: 0.0,
            settling: None,
            speed: REEL_SPEED,
            target: stop,
        }
    }

    pub fn is_blurred(&self) -> bool {
        self.spinning && self.settling.is_none()
    }

    pub fn stop(&self) -> isize {
        (self.pos + 0.5).floor() as isize
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiveStage {
    Surfacing,
    Opening,
    Rules,
    Respins,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dive {
    pub cells: [Option<Multiple>; CELLS],
    pub respins: u8,
    pub timer: f32,
    pub fresh: [bool; CELLS],
    pub over: bool,
    pub window: [[Symbol; ROWS]; REELS],
    pub intro: f32,
}

impl Dive {
    pub fn start(window: &[[Symbol; ROWS]; REELS], rng: &mut impl RngExt) -> Self {
        let mut cells = [None; CELLS];
        for (reel, column) in window.iter().enumerate() {
            for (row, symbol) in column.iter().enumerate() {
                if *symbol == Symbol::Pearl {
                    cells[cell(reel, row)] = Some(pearl_value(rng));
                }
            }
        }
        Self {
            cells,
            respins: RESPINS,
            timer: 0.0,
            fresh: [false; CELLS],
            over: false,
            window: *window,
            intro: 0.0,
        }
    }

    fn opening_secs(&self) -> f32 {
        PEARL_OPEN_SECS * self.filled() as f32
    }

    fn intro_secs(&self) -> f32 {
        PEARL_GLOW_SECS + self.opening_secs() + DIVE_RULES_SECS
    }

    pub fn stage(&self) -> DiveStage {
        if self.intro < PEARL_GLOW_SECS {
            DiveStage::Surfacing
        } else if self.intro < PEARL_GLOW_SECS + self.opening_secs() {
            DiveStage::Opening
        } else if self.intro < self.intro_secs() {
            DiveStage::Rules
        } else {
            DiveStage::Respins
        }
    }

    pub fn is_open(&self, index: usize) -> bool {
        match self.stage() {
            DiveStage::Surfacing => false,
            DiveStage::Opening => {
                let opened = ((self.intro - PEARL_GLOW_SECS) / PEARL_OPEN_SECS) as usize + 1;
                let rank = self.cells[..index].iter().flatten().count();
                self.cells[index].is_some() && rank < opened
            }
            DiveStage::Rules | DiveStage::Respins => self.cells[index].is_some(),
        }
    }

    pub fn filled(&self) -> usize {
        self.cells.iter().flatten().count()
    }

    pub fn is_full(&self) -> bool {
        self.filled() == CELLS
    }

    pub fn total(&self) -> Multiple {
        Multiple::hundredths(self.cells.iter().flatten().map(|m| m.raw()).sum())
    }

    fn respin(&mut self, rng: &mut impl RngExt) {
        self.fresh = [false; CELLS];
        let mut landed = false;
        for (i, slot) in self.cells.iter_mut().enumerate() {
            if slot.is_none() && rng.random_range(0..PER_MILLE) < NEW_PEARL_PER_MILLE {
                *slot = Some(pearl_value(rng));
                self.fresh[i] = true;
                landed = true;
            }
        }
        if landed {
            self.respins = RESPINS;
        } else {
            self.respins = self.respins.saturating_sub(1);
        }
        if self.respins == 0 || self.is_full() {
            self.over = true;
        }
    }
}

pub fn cell(reel: usize, row: usize) -> usize {
    row * REELS + reel
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpinPhase {
    Idle,
    Spinning,
    Diving(Box<Dive>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpinOutcome {
    pub line: Multiple,
    pub line_label: String,
    pub dive: Option<Multiple>,
    pub full: bool,
}

impl SpinOutcome {
    pub fn multiple(&self) -> Multiple {
        Multiple::hundredths(self.line.raw() + self.dive.map_or(0, Multiple::raw))
    }

    pub fn label(&self) -> String {
        match (self.dive, self.full) {
            (Some(_), true) => "Pearl Dive, a full grid".to_string(),
            (Some(total), false) => format!("Pearl Dive {}", total.label()),
            (None, _) if self.line_label.is_empty() => String::new(),
            (None, _) => format!("{}  {}", self.line_label, self.line.label()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Spins {
    pub reels: [Reel; REELS],
    pub phase: SpinPhase,
    pub suspense: bool,
    clock: f32,
    line: (Multiple, String),
    pub lever: f32,
}

impl Default for Spins {
    fn default() -> Self {
        Self {
            reels: [Reel::resting(3), Reel::resting(41), Reel::resting(77)],
            phase: SpinPhase::Idle,
            suspense: false,
            clock: 0.0,
            line: (Multiple::ZERO, String::new()),
            lever: 0.0,
        }
    }
}

pub const LEVER_SECS: f32 = 0.5;

impl Spins {
    pub fn is_idle(&self) -> bool {
        self.phase == SpinPhase::Idle
    }

    pub fn spin(&mut self, rng: &mut impl RngExt) {
        let targets: [usize; REELS] = std::array::from_fn(|_| rng.random_range(0..STOPS));
        let shown = window(targets);
        let pearls_first_two = pearls_in(&[shown[0], shown[1], [Symbol::Bubbles; ROWS]]);
        let line_first_two = [shown[0][MIDDLE], shown[1][MIDDLE]];
        self.suspense = pearls_first_two == PEARLS_TO_DIVE - 1
            || (line_first_two[0] == line_first_two[1]
                && matches!(line_first_two[0], Symbol::Cash | Symbol::Coffee));
        for (i, reel) in self.reels.iter_mut().enumerate() {
            reel.spinning = true;
            reel.settling = None;
            reel.target = targets[i];
            reel.speed = REEL_SPEED + REEL_SPEED_STEP * i as f32;
            let last = i == REELS - 1;
            reel.stop_at = FIRST_STOP_SECS
                + STOP_GAP_SECS * i as f32
                + if last && self.suspense {
                    SUSPENSE_SECS
                } else {
                    0.0
                };
        }
        self.clock = 0.0;
        self.lever = LEVER_SECS;
        self.phase = SpinPhase::Spinning;
    }

    pub fn shown(&self) -> [[Symbol; ROWS]; REELS] {
        self.reels.map(|reel| {
            let s = reel.stop();
            [at(s - 1), at(s), at(s + 1)]
        })
    }

    pub fn in_suspense(&self) -> bool {
        self.suspense
            && self.phase == SpinPhase::Spinning
            && self.reels[REELS - 1].is_blurred()
            && self.clock > SUSPENSE_FROM_SECS
    }

    pub fn tick(&mut self, dt: f32, rng: &mut impl RngExt) -> Option<SpinOutcome> {
        self.lever = (self.lever - dt).max(0.0);
        match &mut self.phase {
            SpinPhase::Idle => None,
            SpinPhase::Spinning => {
                self.clock += dt;
                let clock = self.clock;
                let slow = self.suspense && clock > SUSPENSE_FROM_SECS;
                for (i, reel) in self.reels.iter_mut().enumerate() {
                    turn(reel, i == REELS - 1 && slow, clock, dt);
                }
                if self.reels.iter().any(|r| r.spinning) {
                    return None;
                }
                self.land(rng)
            }
            SpinPhase::Diving(dive) => {
                if dive.intro < dive.intro_secs() {
                    dive.intro += dt;
                    return None;
                }
                dive.timer += dt;
                if dive.over {
                    if dive.timer < DIVE_CLOSE_SECS {
                        return None;
                    }
                    let outcome = SpinOutcome {
                        line: self.line.0,
                        line_label: self.line.1.clone(),
                        dive: Some(dive.total()),
                        full: dive.is_full(),
                    };
                    self.phase = SpinPhase::Idle;
                    return Some(outcome);
                }
                if dive.timer >= RESPIN_SECS {
                    dive.timer = 0.0;
                    dive.respin(rng);
                }
                None
            }
        }
    }

    fn land(&mut self, rng: &mut impl RngExt) -> Option<SpinOutcome> {
        let shown = self.shown();
        let line = [shown[0][MIDDLE], shown[1][MIDDLE], shown[2][MIDDLE]];
        self.line = line_pay(line);
        if pearls_in(&shown) >= PEARLS_TO_DIVE {
            self.phase = SpinPhase::Diving(Box::new(Dive::start(&shown, rng)));
            return None;
        }
        self.phase = SpinPhase::Idle;
        Some(SpinOutcome {
            line: self.line.0,
            line_label: self.line.1.clone(),
            dive: None,
            full: false,
        })
    }

    pub fn dive(&self) -> Option<&Dive> {
        match &self.phase {
            SpinPhase::Diving(dive) => Some(dive),
            _ => None,
        }
    }
}

fn turn(reel: &mut Reel, slow: bool, clock: f32, dt: f32) {
    if !reel.spinning {
        return;
    }
    match reel.settling.as_mut() {
        None => {
            reel.pos += reel.speed * dt * if slow { SUSPENSE_SLOWDOWN } else { 1.0 };
            if clock >= reel.stop_at {
                let mut to = (reel.pos + LANDING_STOPS).floor();
                while (to as isize).rem_euclid(STOPS as isize) as usize != reel.target {
                    to += 1.0;
                }
                reel.settling = Some((reel.pos, to, 0.0));
            }
        }
        Some((from, to, t)) => {
            *t += dt / SETTLE_SECS;
            let e = t.min(1.0) - 1.0;
            let eased = 1.0 + (OVERSHOOT + 1.0) * e.powi(3) + OVERSHOOT * e.powi(2);
            reel.pos = *from + (*to - *from) * eased;
            if *t >= 1.0 {
                reel.pos = *to;
                reel.spinning = false;
                reel.settling = None;
            }
        }
    }
}

pub fn line_pays_back() -> f64 {
    let s = strip();
    let mut total = 0.0;
    for a in s {
        for b in s {
            for c in s {
                total += line_pay([*a, *b, *c]).0.as_f64();
            }
        }
    }
    total / (STOPS as f64).powi(3)
}

pub fn line_beyond_the_stake() -> (f64, f64) {
    let s = strip();
    let (mut chance, mut pays) = (0.0, 0.0);
    for a in s {
        for b in s {
            for c in s {
                let m = line_pay([*a, *b, *c]).0;
                if m > Multiple::ONE {
                    chance += 1.0;
                    pays += m.as_f64();
                }
            }
        }
    }
    let n = (STOPS as f64).powi(3);
    (chance / n, pays / n)
}

pub fn dive_chance() -> f64 {
    let with_pearl = (0..STOPS)
        .filter(|&stop| window([stop, 0, 0])[0].contains(&Symbol::Pearl))
        .count() as f64
        / STOPS as f64;
    with_pearl.powi(REELS as i32)
}

pub fn dive_odds() -> (f64, f64) {
    let q = f64::from(NEW_PEARL_PER_MILLE) / f64::from(PER_MILLE);
    let mut memo = std::collections::HashMap::new();
    fn binom(n: usize, k: usize) -> f64 {
        (1..=k).fold(1.0, |acc, i| acc * (n - k + i) as f64 / i as f64)
    }
    fn walk(
        filled: usize,
        respins: u8,
        q: f64,
        memo: &mut std::collections::HashMap<(usize, u8), (f64, f64)>,
    ) -> (f64, f64) {
        if filled == CELLS {
            return (CELLS as f64, 1.0);
        }
        if respins == 0 {
            return (filled as f64, 0.0);
        }
        if let Some(known) = memo.get(&(filled, respins)) {
            return *known;
        }
        let empty = CELLS - filled;
        let (mut pearls, mut full) = (0.0, 0.0);
        for new in 0..=empty {
            let p = binom(empty, new) * q.powi(new as i32) * (1.0 - q).powi((empty - new) as i32);
            let (n, f) = if new > 0 {
                walk(filled + new, RESPINS, q, memo)
            } else {
                walk(filled, respins - 1, q, memo)
            };
            pearls += p * n;
            full += p * f;
        }
        memo.insert((filled, respins), (pearls, full));
        (pearls, full)
    }
    walk(PEARLS_TO_DIVE, RESPINS, q, &mut memo)
}

pub fn pays_back() -> f64 {
    let (pearls, _) = dive_odds();
    line_pays_back() + dive_chance() * pearls * mean_pearl()
}

pub fn full_grid_chance() -> f64 {
    dive_chance() * dive_odds().1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_strip_holds_every_symbol_as_often_as_it_says() {
        for symbol in Symbol::ALL {
            let count = strip().iter().filter(|s| **s == symbol).count();
            assert_eq!(count, symbol.stops(), "{symbol:?}");
        }
        assert_eq!(Symbol::ALL.iter().map(|s| s.stops()).sum::<usize>(), STOPS);
    }

    #[test]
    fn two_golden_pearls_never_share_a_reel_window() {
        for stop in 0..STOPS {
            let column = window([stop, 0, 0])[0];
            assert!(column.iter().filter(|s| **s == Symbol::Pearl).count() <= 1);
        }
    }

    #[test]
    fn the_reels_pay_back_about_ninety_one_percent_before_the_pot() {
        let rtp = pays_back();
        assert!((0.88..0.93).contains(&rtp), "{rtp}");
    }

    #[test]
    fn a_pearl_dive_starts_about_once_in_three_hundred_spins() {
        let chance = dive_chance();
        assert!((1.0 / 400.0..1.0 / 200.0).contains(&chance), "{chance}");
    }

    #[test]
    fn a_spin_lands_where_it_was_aimed() {
        let mut rng = rand::rng();
        let mut spins = Spins::default();
        for _ in 0..50 {
            spins.spin(&mut rng);
            let aimed = spins.reels.map(|r| r.target);
            for _ in 0..400 {
                if spins.tick(1.0 / 30.0, &mut rng).is_some() || spins.dive().is_some() {
                    break;
                }
            }
            let landed = spins
                .reels
                .map(|r| r.stop().rem_euclid(STOPS as isize) as usize);
            assert_eq!(landed, aimed);
            spins.phase = SpinPhase::Idle;
        }
    }

    #[test]
    fn a_dive_ends_when_its_respins_run_out_or_the_grid_fills() {
        let mut rng = rand::rng();
        let window = [[Symbol::Pearl, Symbol::Bubbles, Symbol::Bubbles]; REELS];
        let mut dive = Dive::start(&window, &mut rng);
        assert_eq!(dive.filled(), PEARLS_TO_DIVE);
        for _ in 0..200 {
            if dive.over {
                break;
            }
            dive.respin(&mut rng);
        }
        assert!(dive.over);
        assert!(dive.respins == 0 || dive.is_full());
    }

    #[test]
    fn a_dive_surfaces_opens_its_pearls_one_by_one_and_explains_itself_before_a_respin() {
        let mut rng = rand::rng();
        let window = [[Symbol::Pearl, Symbol::Bubbles, Symbol::Bubbles]; REELS];
        let mut spins = Spins {
            phase: SpinPhase::Diving(Box::new(Dive::start(&window, &mut rng))),
            ..Spins::default()
        };
        let mut stages = vec![DiveStage::Surfacing];
        let mut opened = 0;
        while let Some(dive) = spins.dive()
            && dive.stage() != DiveStage::Respins
        {
            let now = (0..CELLS).filter(|&i| dive.is_open(i)).count();
            assert!(now == opened || now == opened + 1, "one pearl at a time");
            opened = now;
            assert_eq!(dive.timer, 0.0, "no respin before the rules are shown");
            if stages.last() != Some(&dive.stage()) {
                stages.push(dive.stage());
            }
            spins.tick(1.0 / 30.0, &mut rng);
        }
        assert_eq!(
            stages,
            [DiveStage::Surfacing, DiveStage::Opening, DiveStage::Rules]
        );
        assert_eq!(opened, PEARLS_TO_DIVE);
    }
}
