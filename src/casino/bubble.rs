use rand::RngExt;

use super::Multiple;
use super::seat::Round;

pub const STEP_SECS: f32 = 0.11;
pub const MAX_IN_FLIGHT: usize = 12;
pub const SHELL_FLASH_SECS: f32 = 0.8;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Risk {
    Low,
    High,
    Stupid,
}

impl Risk {
    pub const ALL: [Risk; 3] = [Risk::Low, Risk::High, Risk::Stupid];

    pub fn rows(self) -> usize {
        match self {
            Risk::Low | Risk::High => 10,
            Risk::Stupid => 16,
        }
    }

    fn half(self) -> &'static [u32] {
        match self {
            Risk::Low => &[89, 30, 14, 11, 9, 6],
            Risk::High => &[700, 100, 30, 9, 3, 2],
            Risk::Stupid => &[10_000, 1_300, 260, 90, 40, 19, 2, 2, 2],
        }
    }

    pub fn shells(self) -> Vec<Multiple> {
        let half = self.half();
        half.iter()
            .chain(half.iter().rev().skip(1))
            .map(|&tenths| Multiple::tenths(tenths.into()))
            .collect()
    }

    pub fn name(self) -> &'static str {
        match self {
            Risk::Low => "Low",
            Risk::High => "High",
            Risk::Stupid => "Stupid",
        }
    }

    pub fn riskier(self) -> Risk {
        match self {
            Risk::Low => Risk::High,
            Risk::High | Risk::Stupid => Risk::Stupid,
        }
    }

    pub fn safer(self) -> Risk {
        match self {
            Risk::Stupid => Risk::High,
            Risk::High | Risk::Low => Risk::Low,
        }
    }

    pub fn chance(self, shell: usize) -> f64 {
        let n = self.rows();
        let ways = (1..=shell).fold(1.0, |acc, i| acc * (n - shell + i) as f64 / i as f64);
        ways / 2f64.powi(n as i32)
    }

    pub fn pays_back(self) -> f64 {
        self.shells()
            .iter()
            .enumerate()
            .map(|(k, m)| self.chance(k) * m.as_f64())
            .sum()
    }

    pub fn beyond_the_stake(self) -> (f64, f64) {
        let mut chance = 0.0;
        let mut pays = 0.0;
        for (k, m) in self.shells().iter().enumerate() {
            if *m > Multiple::ONE {
                chance += self.chance(k);
                pays += self.chance(k) * m.as_f64();
            }
        }
        (chance, pays)
    }
}

#[derive(Clone, Debug)]
pub struct Bubble {
    pub path: Vec<bool>,
    pub step: usize,
    pub t: f32,
    pub risk: Risk,
    pub round: Round,
}

impl Bubble {
    pub fn blow(risk: Risk, round: Round, rng: &mut impl RngExt) -> Self {
        Self {
            path: (0..risk.rows()).map(|_| rng.random::<bool>()).collect(),
            step: 0,
            t: 0.0,
            risk,
            round,
        }
    }

    pub fn offset(&self) -> isize {
        self.path
            .iter()
            .take(self.step.min(self.path.len()))
            .map(|&right| if right { 1 } else { -1 })
            .sum()
    }

    pub fn shell(&self) -> usize {
        self.path.iter().filter(|&&right| right).count()
    }

    pub fn has_landed(&self) -> bool {
        self.step > self.path.len()
    }

    pub fn tick(&mut self, dt: f32) {
        self.t += dt;
        while self.t >= STEP_SECS && !self.has_landed() {
            self.t -= STEP_SECS;
            self.step += 1;
        }
    }

    pub fn pays(&self) -> Multiple {
        self.risk.shells()[self.shell()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_board_pays_back_about_ninety_seven_percent() {
        for risk in Risk::ALL {
            let rtp = risk.pays_back();
            assert!((0.95..0.99).contains(&rtp), "{risk:?}: {rtp}");
        }
    }

    #[test]
    fn every_board_has_one_shell_more_than_rows() {
        for risk in Risk::ALL {
            assert_eq!(risk.shells().len(), risk.rows() + 1);
        }
    }

    #[test]
    fn the_stupid_board_hides_a_thousand_at_each_edge() {
        let shells = Risk::Stupid.shells();
        assert_eq!(shells[0], Multiple::whole(1000));
        assert_eq!(*shells.last().unwrap(), Multiple::whole(1000));
    }
}
