use rand::RngExt;

use super::Multiple;

pub const PAYS_BACK: f64 = 0.97;
pub const DOUBLING_SECS: f32 = 7.0;
pub const AUTO_TARGETS: [Option<Multiple>; 9] = [
    None,
    Some(Multiple::tenths(15)),
    Some(Multiple::whole(2)),
    Some(Multiple::whole(3)),
    Some(Multiple::whole(5)),
    Some(Multiple::whole(10)),
    Some(Multiple::whole(25)),
    Some(Multiple::whole(100)),
    Some(Multiple::whole(1000)),
];
pub const HISTORY: usize = 8;
pub const STUPID_FROM: Multiple = Multiple::whole(100);

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PuffPhase {
    Waiting,
    Puffing { clock: f32, pops_at: Multiple },
    Popped { at: Multiple, t: f32 },
    Cashed { at: Multiple },
}

#[derive(Clone, Debug)]
pub struct Pufferfish {
    pub phase: PuffPhase,
    pub auto: usize,
    pub history: Vec<Multiple>,
}

impl Default for Pufferfish {
    fn default() -> Self {
        Self {
            phase: PuffPhase::Waiting,
            auto: 0,
            history: Vec::new(),
        }
    }
}

pub fn pop_point(rng: &mut impl RngExt) -> Multiple {
    let draw: f64 = 1.0 - rng.random::<f64>();
    let point = PAYS_BACK / draw;
    if point < 1.0 {
        return Multiple::ONE;
    }
    Multiple::hundredths((point * 100.0).floor().min(u128::MAX as f64) as u128)
}

pub fn multiple_at(clock: f32) -> Multiple {
    let x = 2f64.powf(f64::from(clock / DOUBLING_SECS));
    Multiple::hundredths((x * 100.0).floor() as u128)
}

pub fn reach_chance(target: f64) -> f64 {
    if target <= 1.0 {
        return PAYS_BACK.min(1.0);
    }
    (PAYS_BACK / target).min(1.0)
}

impl Pufferfish {
    pub fn target(&self) -> Option<Multiple> {
        AUTO_TARGETS[self.auto]
    }

    pub fn raise_target(&mut self) {
        self.auto = (self.auto + 1).min(AUTO_TARGETS.len() - 1);
    }

    pub fn lower_target(&mut self) {
        self.auto = self.auto.saturating_sub(1);
    }

    pub fn puff(&mut self, rng: &mut impl RngExt) {
        self.phase = PuffPhase::Puffing {
            clock: 0.0,
            pops_at: pop_point(rng),
        };
    }

    pub fn now(&self) -> Multiple {
        match self.phase {
            PuffPhase::Waiting => Multiple::ONE,
            PuffPhase::Puffing { clock, pops_at } => multiple_at(clock).min(pops_at),
            PuffPhase::Popped { at, .. } | PuffPhase::Cashed { at } => at,
        }
    }

    pub fn is_puffing(&self) -> bool {
        matches!(self.phase, PuffPhase::Puffing { .. })
    }

    pub fn cash_out(&mut self) -> Option<Multiple> {
        let PuffPhase::Puffing { pops_at, .. } = self.phase else {
            return None;
        };
        let at = self.now();
        self.remember(pops_at);
        self.phase = PuffPhase::Cashed { at };
        Some(at)
    }

    fn remember(&mut self, pop: Multiple) {
        self.history.insert(0, pop);
        self.history.truncate(HISTORY);
    }

    pub fn tick(&mut self, dt: f32) -> Option<Multiple> {
        match &mut self.phase {
            PuffPhase::Puffing { clock, pops_at } => {
                *clock += dt;
                let (now, pops_at) = (multiple_at(*clock), *pops_at);
                if now >= pops_at {
                    self.remember(pops_at);
                    self.phase = PuffPhase::Popped {
                        at: pops_at,
                        t: 0.0,
                    };
                    return Some(Multiple::ZERO);
                }
                if let Some(target) = self.target()
                    && now >= target
                {
                    self.remember(pops_at);
                    self.phase = PuffPhase::Cashed { at: target };
                    return Some(target);
                }
                None
            }
            PuffPhase::Popped { t, .. } => {
                *t += dt;
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_cash_out_target_pays_back_ninety_seven_percent() {
        for target in [1.5, 2.0, 10.0, 1000.0] {
            let rtp = reach_chance(target) * target;
            assert!((rtp - PAYS_BACK).abs() < 1e-9, "{target}: {rtp}");
        }
    }

    #[test]
    fn a_puffer_reaches_its_targets_as_often_as_promised() {
        let mut rng = rand::rng();
        let rolls = 40_000;
        let reached = (0..rolls)
            .filter(|_| pop_point(&mut rng) >= Multiple::whole(2))
            .count();
        let share = reached as f64 / rolls as f64;
        assert!((share - reach_chance(2.0)).abs() < 0.015, "{share}");
    }

    #[test]
    fn the_puffer_doubles_every_seven_seconds() {
        assert_eq!(multiple_at(0.0), Multiple::ONE);
        assert_eq!(multiple_at(DOUBLING_SECS), Multiple::whole(2));
    }

    #[test]
    fn an_auto_target_cashes_out_by_itself() {
        let mut puffer = Pufferfish {
            auto: 2,
            ..Pufferfish::default()
        };
        puffer.phase = PuffPhase::Puffing {
            clock: 0.0,
            pops_at: Multiple::whole(50),
        };
        let mut paid = None;
        for _ in 0..1000 {
            if let Some(m) = puffer.tick(0.05) {
                paid = Some(m);
                break;
            }
        }
        assert_eq!(paid, Some(Multiple::whole(2)));
    }
}
