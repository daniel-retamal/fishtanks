use rand::RngExt;

use crate::fishes::fish::Fish;
use crate::fishes::species::{ALL_SPECIES, FishSpecies};

use super::Multiple;

pub const LANES: usize = 5;
pub const CHANCES_PER_MILLE: [u32; LANES] = [340, 250, 190, 140, 80];
pub const PAYS_BACK_PER_MILLE: u32 = 920;
const PER_MILLE: u32 = 1000;
const TENTHS_PER_WHOLE: u32 = 10;
const WINNER_FINISH_SECS: f32 = 6.5;
const FINISH_SPREAD_SECS: f32 = 1.0;
const FIRST_GAP_SECS: f32 = 0.15;
const LAST_GAP_SECS: f32 = 1.6;
const SAMPLE_SECS: f32 = 0.05;
const ZOOMIES: usize = 3;
const ZOOM_SECS: f32 = 0.7;
const ZOOM_BOOST: f32 = 2.6;
const WOBBLE: f32 = 0.35;
const AFTER_FINISH_SECS: f32 = 0.6;
const CALL_SECS: f32 = 1.6;

pub fn odds(chance_per_mille: u32, pays_back_per_mille: u32) -> Multiple {
    Multiple::tenths((pays_back_per_mille * TENTHS_PER_WHOLE / chance_per_mille).into())
}

#[derive(Clone)]
pub struct Lane {
    pub species: FishSpecies,
    pub portrait: Fish,
    pub chance: u32,
    pub pos: f32,
    finish: f32,
    curve: Vec<f32>,
    zoomies: Vec<f32>,
}

impl Lane {
    pub fn name(&self) -> &'static str {
        self.species.display_name()
    }

    pub fn odds(&self) -> Multiple {
        odds(self.chance, PAYS_BACK_PER_MILLE)
    }

    pub fn is_zooming(&self, clock: f32) -> bool {
        self.zoomies
            .iter()
            .any(|&start| clock >= start && clock < start + ZOOM_SECS)
            && self.pos < 1.0
    }

    fn speed(&self, clock: f32, seed: f32) -> f32 {
        let zoom = if self
            .zoomies
            .iter()
            .any(|&s| clock >= s && clock < s + ZOOM_SECS)
        {
            ZOOM_BOOST
        } else {
            1.0
        };
        zoom + WOBBLE * (clock * 1.7 + seed).sin()
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RacePhase {
    Waiting,
    Running { clock: f32 },
    Finished { winner: usize },
}

#[derive(Clone)]
pub struct Derby {
    pub lanes: Vec<Lane>,
    pub pick: usize,
    pub phase: RacePhase,
    pub call: Option<(String, f32)>,
    leader: Option<usize>,
    winner: usize,
}

pub fn racers() -> Vec<FishSpecies> {
    ALL_SPECIES
        .iter()
        .copied()
        .filter(|s| s.config().habitat.is_fished())
        .filter(|s| {
            let fish = Fish::new_for_display(*s, &mut rand::rng());
            fish.unfish_kind().is_none() && fish.line_sprite().rows.len() == 1
        })
        .collect()
}

impl Derby {
    pub fn new(rng: &mut impl RngExt) -> Self {
        let mut derby = Self {
            lanes: Vec::new(),
            pick: 0,
            phase: RacePhase::Waiting,
            call: None,
            leader: None,
            winner: 0,
        };
        derby.line_up(rng);
        derby
    }

    pub fn line_up(&mut self, rng: &mut impl RngExt) {
        let mut pool = racers();
        let mut chances = CHANCES_PER_MILLE.to_vec();
        shuffle(&mut chances, rng);
        self.lanes = chances
            .into_iter()
            .map(|chance| {
                let species = pool.swap_remove(rng.random_range(0..pool.len()));
                let portrait = Fish::new_for_display(species, rng);
                Lane::new(species, portrait, chance)
            })
            .collect();
        self.phase = RacePhase::Waiting;
        self.call = None;
    }

    pub fn start(&mut self, rng: &mut impl RngExt) {
        let draw = rng.random_range(0..PER_MILLE);
        let mut acc = 0;
        self.winner = LANES - 1;
        for (i, lane) in self.lanes.iter().enumerate() {
            acc += lane.chance;
            if draw < acc {
                self.winner = i;
                break;
            }
        }
        let winner_time = WINNER_FINISH_SECS + rng.random_range(0.0..FINISH_SPREAD_SECS);
        for (i, lane) in self.lanes.iter_mut().enumerate() {
            lane.finish = if i == self.winner {
                winner_time
            } else {
                winner_time + rng.random_range(FIRST_GAP_SECS..LAST_GAP_SECS)
            };
            lane.zoomies = (0..ZOOMIES)
                .map(|_| rng.random_range(0.3..lane.finish - ZOOM_SECS))
                .collect();
            let seed = rng.random_range(0.0..std::f32::consts::TAU);
            let mut distance = 0.0;
            let mut curve = vec![0.0];
            let mut clock = 0.0;
            while clock < lane.finish {
                distance += lane.speed(clock, seed) * SAMPLE_SECS;
                curve.push(distance);
                clock += SAMPLE_SECS;
            }
            for d in &mut curve {
                *d /= distance;
            }
            lane.curve = curve;
            lane.pos = 0.0;
        }
        self.leader = None;
        self.call = None;
        self.phase = RacePhase::Running { clock: 0.0 };
    }

    pub fn tick(&mut self, dt: f32) -> Option<bool> {
        if let Some((_, t)) = &mut self.call {
            *t -= dt;
            if *t <= 0.0 {
                self.call = None;
            }
        }
        let RacePhase::Running { clock } = &mut self.phase else {
            return None;
        };
        *clock += dt;
        let clock = *clock;
        let mut zoomed = None;
        for (i, lane) in self.lanes.iter_mut().enumerate() {
            let was = lane.is_zooming(clock - dt);
            let k = (clock / SAMPLE_SECS) as usize;
            lane.pos = lane.curve.get(k).copied().unwrap_or(1.0).min(1.0);
            if !was && lane.is_zooming(clock) {
                zoomed = Some(i);
            }
        }
        let leader = (0..self.lanes.len())
            .max_by(|&a, &b| self.lanes[a].pos.partial_cmp(&self.lanes[b].pos).unwrap())
            .filter(|&i| self.lanes[i].pos > 0.1);
        if let Some(i) = zoomed {
            self.call = Some((format!("{} zooms!", self.lanes[i].name()), CALL_SECS));
        } else if leader != self.leader
            && let Some(i) = leader
        {
            self.call = Some((
                format!("{} takes the lead", self.lanes[i].name()),
                CALL_SECS,
            ));
        }
        self.leader = leader;
        let last = self.lanes.iter().map(|l| l.finish).fold(0.0, f32::max);
        if clock < last + AFTER_FINISH_SECS {
            return None;
        }
        self.phase = RacePhase::Finished {
            winner: self.winner,
        };
        self.call = Some((
            format!("{} wins", self.lanes[self.winner].name()),
            CALL_SECS * 2.0,
        ));
        Some(self.winner == self.pick)
    }

    pub fn clock(&self) -> f32 {
        match self.phase {
            RacePhase::Running { clock } => clock,
            _ => 0.0,
        }
    }

    pub fn winner(&self) -> Option<usize> {
        match self.phase {
            RacePhase::Finished { winner } => Some(winner),
            _ => None,
        }
    }

    pub fn picked(&self) -> &Lane {
        &self.lanes[self.pick]
    }

    pub fn pick_up(&mut self) {
        self.pick = self.pick.saturating_sub(1);
    }

    pub fn pick_down(&mut self) {
        self.pick = (self.pick + 1).min(LANES - 1);
    }
}

impl Lane {
    fn new(species: FishSpecies, portrait: Fish, chance: u32) -> Self {
        Self {
            species,
            portrait,
            chance,
            pos: 0.0,
            finish: 0.0,
            curve: Vec::new(),
            zoomies: Vec::new(),
        }
    }
}

fn shuffle<T>(items: &mut [T], rng: &mut impl RngExt) {
    for i in (1..items.len()).rev() {
        let j = rng.random_range(0..=i);
        items.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_lane_pays_back_at_most_ninety_two_percent() {
        for chance in CHANCES_PER_MILLE {
            let rtp = odds(chance, PAYS_BACK_PER_MILLE).as_f64() * f64::from(chance) / 1000.0;
            assert!((0.88..=0.92).contains(&rtp), "{chance}: {rtp}");
        }
    }

    #[test]
    fn every_winner_pays_more_than_its_stake() {
        for chance in CHANCES_PER_MILLE {
            assert!(
                odds(chance, PAYS_BACK_PER_MILLE) > Multiple::whole(1),
                "{chance}"
            );
        }
    }

    #[test]
    fn the_chances_add_up_to_one_race() {
        assert_eq!(CHANCES_PER_MILLE.iter().sum::<u32>(), PER_MILLE);
    }

    #[test]
    fn the_drawn_winner_crosses_the_line_first() {
        let mut rng = rand::rng();
        let mut derby = Derby::new(&mut rng);
        for _ in 0..20 {
            derby.start(&mut rng);
            let mut first = None;
            while derby.winner().is_none() {
                derby.tick(1.0 / 30.0);
                if first.is_none() {
                    first = derby.lanes.iter().position(|l| l.pos >= 1.0);
                }
            }
            assert_eq!(first, derby.winner());
            derby.phase = RacePhase::Waiting;
        }
    }
}
