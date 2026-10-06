use rand::RngExt;

use super::{Fish, FishState, LineSprite};
use crate::colors::{RED_PALE, blended, inverted};
use crate::fishes::quirk::{
    FORGET_SHAPE_MEAN_SECS, FORGET_TURN_MEAN_SECS, Quirk, borrowed_body, borrowed_size,
};
use crate::fishes::species::{
    EYE_CIRCLE, EYE_CIRCLE_SHUT, EYE_DEAD, EYE_ROUND, EYE_ROUND_SHUT, FishSpecies, Zoomie,
};
use crate::fishes::unfish::{UNFISH_EYE_COLOR, UnfishKind, UnfishState};
use crate::sprite::TRANSPARENT;
use crate::tank::ChannelRegistry;
use crate::util::sample_exponential;

const FLESH_MIX: f32 = 0.65;

fn is_eye(glyph: char) -> bool {
    matches!(
        glyph,
        EYE_ROUND | EYE_CIRCLE | EYE_ROUND_SHUT | EYE_CIRCLE_SHUT | EYE_DEAD
    )
}

pub struct Reflected {
    pub offset: f32,
    pub faces_away: bool,
}

impl Fish {
    pub fn is_unfish(&self) -> bool {
        self.unfish_state.is_some() || self.leeched
    }

    pub fn unfish_kind(&self) -> Option<UnfishKind> {
        self.unfish_state.as_deref().map(|us| us.kind)
    }

    pub fn takes_a_seat(&self) -> bool {
        self.unfish_kind().is_none_or(UnfishKind::takes_a_seat)
    }

    pub fn quirk(&self) -> Option<&Quirk> {
        self.unfish_state.as_deref().map(|us| &us.quirk)
    }

    pub fn quirk_mut(&mut self) -> Option<&mut Quirk> {
        self.unfish_state.as_deref_mut().map(|us| &mut us.quirk)
    }

    pub fn kind_name(&self) -> &'static str {
        if self.leeched {
            return FishSpecies::Unfish.display_name();
        }
        self.species.display_name()
    }

    pub fn carries(&self, kind: UnfishKind) -> bool {
        self.unfish_kind() == Some(kind)
            || self
                .fused_components()
                .iter()
                .filter_map(|component| component.persona.as_deref())
                .any(|persona| persona.kind == kind)
    }

    pub fn scatter_progress(&self) -> Option<f32> {
        let FishState::Zoomie {
            time_remaining,
            total_duration,
            ..
        } = self.state
        else {
            return None;
        };
        let scatters = self.zoomie() == Zoomie::Scatter || self.carries(UnfishKind::Bones);
        (scatters && total_duration > 0.0).then(|| 1.0 - time_remaining / total_duration)
    }

    pub fn fault_lag(&self) -> Option<i32> {
        match self.quirk()? {
            Quirk::Fault(fault) => fault.lag(),
            _ => None,
        }
    }

    pub fn reflected(&self) -> Option<Reflected> {
        match self.quirk()? {
            Quirk::Reflection(reflection) => Some(Reflected {
                offset: reflection.offset,
                faces_away: reflection.faces_away(),
            }),
            _ => None,
        }
    }

    pub fn is_reverse_video(&self) -> bool {
        self.unfish_kind() == Some(UnfishKind::Negative)
    }

    pub fn is_negative(&self) -> bool {
        self.mutant.as_ref().is_some_and(|mutant| mutant.negative)
    }

    pub fn broadcast(&mut self) -> Option<bool> {
        match self.quirk_mut()? {
            Quirk::Signal(signal) => Some(signal.send()),
            _ => None,
        }
    }

    pub fn broadcast_channel(&self) -> Option<String> {
        matches!(self.quirk()?, Quirk::Signal(_))
            .then(|| ChannelRegistry::normalize(&self.name).map(|name| name.into_owned()))
            .flatten()
    }

    pub fn forgets_it_died(&self) -> bool {
        self.unfish_kind() == Some(UnfishKind::Forgetting)
    }

    pub fn holds_still(&self) -> bool {
        self.carries(UnfishKind::Still)
    }

    pub(super) fn tint(&self, sprite: &mut LineSprite) {
        if !self.leeched && !self.is_negative() {
            return;
        }
        for cell in sprite.rows.iter_mut().flatten() {
            if cell.0 == TRANSPARENT || cell.0 == ' ' {
                continue;
            }
            if self.leeched {
                cell.1 = if is_eye(cell.0) {
                    UNFISH_EYE_COLOR
                } else {
                    blended(cell.1, RED_PALE, FLESH_MIX)
                };
            }
            if self.is_negative() {
                cell.1 = inverted(cell.1);
            }
        }
    }

    pub(super) fn tick_oddities(&mut self, dt: f32, moved_dx: f32, rng: &mut impl RngExt) {
        if let Some(Quirk::Reflection(reflection)) = self.quirk_mut() {
            reflection.follow(moved_dx, dt);
        }
        self.forget_the_way(rng);
        self.forget_the_shape(rng);
        self.settle_verso(rng);
    }

    fn forget_the_way(&mut self, rng: &mut impl RngExt) {
        let mut due = false;
        for persona in self.personas_mut() {
            if let Quirk::Forgetting(forgetting) = &mut persona.quirk
                && forgetting.turn_clock <= 0.0
            {
                forgetting.turn_clock = sample_exponential(rng, FORGET_TURN_MEAN_SECS);
                due = true;
            }
        }
        if due && matches!(self.state, FishState::Idle) {
            self.velocity.dx = -self.velocity.dx;
            self.facing = self.facing.flip();
        }
    }

    fn forget_the_shape(&mut self, rng: &mut impl RngExt) {
        let Some(Quirk::Forgetting(forgetting)) = self.quirk_mut() else {
            return;
        };
        if forgetting.shape_clock > 0.0 {
            return;
        }
        forgetting.shape_clock = sample_exponential(rng, FORGET_SHAPE_MEAN_SECS);
        forgetting.body = borrowed_body(rng);
        let body = forgetting.body;
        self.body_size = borrowed_size(body);
        self.display_width = self.unfish_line_width();
    }

    fn settle_verso(&mut self, rng: &mut impl RngExt) {
        let facing_left = self.facing_left();
        let turned =
            matches!(self.quirk(), Some(Quirk::Verso(verso)) if verso.facing_left != facing_left);
        if !turned {
            return;
        }
        let width = self.display_width as f32;
        let x = self.position.x;
        let record = self.mutations.take();
        let Some(us) = self.unfish_state.as_deref_mut() else {
            return;
        };
        let Quirk::Verso(mut verso) = std::mem::take(&mut us.quirk) else {
            return;
        };
        let head = if verso.facing_left {
            x
        } else {
            x + width - 1.0
        };
        let worn = us.clone();
        let other = verso
            .other_look
            .take()
            .unwrap_or_else(|| Box::new(UnfishState::new(us.kind, rng)));
        us.wear(&other);
        verso.other_look = Some(Box::new(worn));
        let other_record = verso.other_record.take();
        verso.other_record = record;
        std::mem::swap(&mut verso.side, &mut verso.other);
        verso.facing_left = facing_left;
        let size = verso.side.size;
        us.quirk = Quirk::Verso(verso);
        self.mutations = other_record;
        self.body_size = size;
        self.display_width = self.unfish_line_width();
        self.position.x = if facing_left {
            head
        } else {
            head - self.display_width as f32 + 1.0
        };
    }
}
