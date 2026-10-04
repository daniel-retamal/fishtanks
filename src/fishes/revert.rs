use rand::{RngExt, SeedableRng, rngs::SmallRng};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use super::fish::Fish;
use super::fused::FusedComponent;
use super::mutant::{EyeState, Mark, MutantState};
use super::mutations::{Mutatable, Mutation, ensure_fish_mutant};
use super::unfish::UnfishState;
use crate::entities::cow::Cow;

#[derive(Clone, Serialize, Deserialize)]
pub enum Look {
    Fish {
        mutant: Option<Box<MutantState>>,
        unfish: Option<Box<UnfishState>>,
        body_size: usize,
        color: Color,
        sway_speed: f32,
        display_width: usize,
    },
    Cow {
        mutant: Box<MutantState>,
        color: Color,
        body_length: usize,
        sway_speed: f32,
        display_width: usize,
    },
}

struct Fusion {
    is_double: bool,
    backwards: bool,
    fused: Vec<FusedComponent>,
    double_head_eyes: Vec<EyeState>,
}

impl Fusion {
    fn take(mutant: &mut MutantState) -> Self {
        Fusion {
            is_double: std::mem::take(&mut mutant.is_double),
            backwards: std::mem::take(&mut mutant.backwards),
            fused: std::mem::take(&mut mutant.fused),
            double_head_eyes: std::mem::take(&mut mutant.double_head_eyes),
        }
    }

    fn is_single(&self) -> bool {
        !self.is_double && self.fused.is_empty()
    }

    fn put_on(self, mutant: &mut MutantState) {
        mutant.is_double = self.is_double;
        mutant.backwards = self.backwards;
        mutant.fused = self.fused;
        mutant.double_head_eyes = self.double_head_eyes;
    }
}

fn without_fusion(mutant: &MutantState) -> Box<MutantState> {
    let mut bare = mutant.clone();
    Fusion::take(&mut bare);
    Box::new(bare)
}

impl Fish {
    pub(crate) fn look(&self) -> Look {
        Look::Fish {
            mutant: self.mutant.as_deref().map(without_fusion),
            unfish: self.unfish_state.as_ref().map(|us| {
                let mut bare = us.clone();
                bare.fused.clear();
                bare
            }),
            body_size: self.body_size,
            color: self.color,
            sway_speed: self.sway_speed,
            display_width: self.display_width,
        }
    }

    pub(crate) fn wear(&mut self, look: &Look) {
        let Look::Fish {
            mutant,
            unfish,
            body_size,
            color,
            sway_speed,
            display_width,
        } = look
        else {
            return;
        };
        let fusion = self
            .mutant
            .as_deref_mut()
            .map(Fusion::take)
            .filter(|fusion| !fusion.is_single());
        self.mutant = mutant.clone();
        self.body_size = *body_size;
        self.color = *color;
        self.sway_speed = *sway_speed;
        self.display_width = *display_width;
        if let (Some(current), Some(original)) = (self.unfish_state.as_mut(), unfish.as_deref()) {
            current.wear(original);
        }
        if let Some(fusion) = fusion {
            ensure_fish_mutant(self, &mut SmallRng::seed_from_u64(self.pattern_seed));
            if let Some(mutant) = self.mutant.as_mut() {
                fusion.put_on(mutant);
            }
        }
    }
}

impl Cow {
    pub(crate) fn look(&self) -> Look {
        Look::Cow {
            mutant: without_fusion(&self.mutant),
            color: self.color,
            body_length: self.body_length,
            sway_speed: self.sway_speed,
            display_width: self.display_width,
        }
    }

    pub(crate) fn wear(&mut self, look: &Look) {
        let Look::Cow {
            mutant,
            color,
            body_length,
            sway_speed,
            display_width,
        } = look
        else {
            return;
        };
        let fusion = Fusion::take(&mut self.mutant);
        self.mutant = mutant.clone();
        fusion.put_on(&mut self.mutant);
        self.color = *color;
        self.body_length = *body_length;
        self.sway_speed = *sway_speed;
        self.display_width = *display_width;
    }
}

pub fn settle_old_record<M: Mutatable>(target: &mut M) {
    if target.record().is_some_and(|record| record.is_legacy()) {
        settle(target);
    }
}

pub fn settle<M: Mutatable>(target: &mut M) {
    if target.record().is_none() {
        return;
    }
    let look = target.look();
    let record = target.record_mut();
    record.marks.resize(record.history.len(), Mark::default());
    record.settle(look);
}

pub fn can_revert<M: Mutatable + ?Sized>(target: &M) -> bool {
    target
        .record()
        .is_some_and(|record| !record.is_legacy() && !record.revertible().is_empty())
}

pub fn revert<M: Mutatable>(target: &mut M, rng: &mut impl RngExt) -> Option<Mutation> {
    settle_old_record(target);
    let candidates = target.record()?.revertible();
    if candidates.is_empty() {
        return None;
    }
    let index = candidates[rng.random_range(0..candidates.len())];
    undo(target, index)
}

pub fn revert_latest<M: Mutatable>(target: &mut M, mutation: Mutation) -> bool {
    settle_old_record(target);
    let Some(record) = target.record() else {
        return false;
    };
    let latest = record
        .revertible()
        .into_iter()
        .rev()
        .find(|&index| record.history[index] == mutation.token());
    latest.is_some_and(|index| undo(target, index).is_some())
}

fn undo<M: Mutatable>(target: &mut M, index: usize) -> Option<Mutation> {
    let record = target.record_mut();
    let undone = Mutation::parse(&record.history.remove(index))?;
    let mark = record.marks.remove(index);
    let settled = record.settled.min(record.history.len());
    let origin = record.origin.clone()?;
    let replay: Vec<(String, Mark)> = record
        .history
        .drain(settled..)
        .zip(record.marks.drain(settled..))
        .collect();
    let (mass_g, bonus_pct) = target.worth();
    target.wear(&origin);
    for (token, entry) in replay {
        let Some(mutation) = Mutation::parse(&token) else {
            continue;
        };
        if !mutation.fuses() {
            if !target.supports_now(mutation) {
                continue;
            }
            target.apply_one(mutation, &mut SmallRng::seed_from_u64(entry.seed));
        }
        let record = target.record_mut();
        record.history.push(token);
        record.marks.push(entry);
    }
    target.set_worth((
        mass_g.saturating_sub(mark.mass_g),
        bonus_pct.saturating_sub(mark.bonus_pct),
    ));
    target.refresh_width();
    target.record_mut().count += 1;
    Some(undone)
}
