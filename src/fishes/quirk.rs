use rand::RngExt;
use serde::{Deserialize, Serialize};

use super::mutant::MutationRecord;
use super::species::{
    ALL_SPECIES, BodyChars, BodyTemplate, EYE_CIRCLE_SHUT, EyeAt, FishSpecies, SizeCategory,
    TailKind,
};
use super::unfish::{UnfishKind, UnfishState};
use crate::cheats::Cheat;
use crate::util::sample_exponential;

pub const LEECH_BITE_MEAN_SECS: f32 = 20.0;
pub const LEECH_BODY_SIZE: usize = 0;
pub const BONES_RIBS_MIN: usize = 3;
pub const BONES_RIBS_MAX: usize = 8;
pub const BONES_RIBS_CAP: usize = 12;
pub const BONES_ZOOMIE_STRETCH: f32 = 3.0;
pub const SIGNAL_GAP_BITS: u64 = 16;
pub const SIGNAL_BODY_SIZE: usize = 10;
const SIGNAL_START_BIT: bool = true;
const SIGNAL_STOP_BIT: bool = false;
const BITS_PER_BYTE: u64 = 8;
const FRAME_BITS: u64 = BITS_PER_BYTE + 2;
pub const SIGNAL_HIGH: char = '‾';
pub const SIGNAL_LOW: char = '_';
pub const FORGET_TURN_MEAN_SECS: f32 = 3.0;
pub const FORGET_SHAPE_MEAN_SECS: f32 = 40.0;
pub const FORGET_MUTATION_MEAN_SECS: f32 = 20.0;
pub const FORGETTING_EYE: char = '?';
pub const FAULT_SLIP_MEAN_SECS: f32 = 25.0;
pub const FAULT_SLIP_SECS: f32 = 4.0;
pub const FAULT_MAX_LAG: f32 = 3.0;
pub const ANAGRAM_SHUFFLE_MEAN_SECS: f32 = 20.0;
pub const REFLECTION_DISAGREE_MEAN_SECS: f32 = 25.0;
const REFLECTION_MOOD_MIN_SECS: f32 = 3.0;
const REFLECTION_MOOD_MAX_SECS: f32 = 5.0;
const REFLECTION_MAX_OFFSET: f32 = 14.0;
const REFLECTION_CATCH_UP: f32 = 2.5;
pub const MOLT_MEAN_SECS: f32 = 45.0;
pub const NEGATIVE_REST_MEAN_SECS: f32 = 60.0;
pub const GRAEAE_PASS_REST_SECS: f32 = 3.0;
pub const RING_CELLS_PER_SEC: f32 = 4.0;
pub const BLIND_EYE: char = EYE_CIRCLE_SHUT;
const SIDE_SIZE_MIN: usize = 1;
const SIDE_SIZE_MAX: usize = 9;
const SIDE_EYES_MAX: usize = 2;

pub const BONES_CHARS: BodyChars = BodyChars {
    mouth_left: '<',
    mouth_right: '>',
    eye_left: '°',
    eye_right: '°',
    body_left: '}',
    wave_left: ')',
    body_right: '{',
    wave_right: '(',
    tail: TailKind::Wide,
};

pub const LEECH_CHARS: BodyChars = BodyChars {
    mouth_left: 'c',
    mouth_right: 'ɔ',
    eye_left: '~',
    eye_right: '~',
    body_left: '~',
    wave_left: '≈',
    body_right: '~',
    wave_right: '≈',
    tail: TailKind::None,
};

pub const FACE_OPEN: [char; 5] = ['(', '°', '‿', '°', ')'];
pub const FACE_DOWN: [char; 5] = ['(', '.', '_', '.', ')'];
pub const FACE_SHUT: [char; 5] = ['(', '-', '‿', '-', ')'];
pub const FACE_EYES: [usize; 2] = [1, 3];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Cling {
    Above,
    Below,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Leech {
    pub host: Option<String>,
    pub cling: Option<Cling>,
    pub drunk_g: u32,
    pub bite_clock: f32,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Bones {
    pub scattered: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Signal {
    pub cheat: Cheat,
    pub sent: u64,
}

impl Signal {
    fn message(&self) -> &'static str {
        self.cheat.code()
    }

    pub fn period(&self) -> u64 {
        self.message().len() as u64 * FRAME_BITS + SIGNAL_GAP_BITS
    }

    pub fn bit(&self, index: u64) -> bool {
        let at = index % self.period();
        let byte_index = at / FRAME_BITS;
        let Some(&byte) = self.message().as_bytes().get(byte_index as usize) else {
            return false;
        };
        match at % FRAME_BITS {
            0 => SIGNAL_START_BIT,
            slot if slot <= BITS_PER_BYTE => (byte >> (slot - 1)) & 1 == 1,
            _ => SIGNAL_STOP_BIT,
        }
    }

    pub fn send(&mut self) -> bool {
        let bit = self.bit(self.sent);
        self.sent = self.sent.wrapping_add(1);
        bit
    }

    pub fn glyph(&self, cells_from_head: usize) -> char {
        let back = cells_from_head as u64 + 1;
        let index = self.sent.wrapping_sub(back);
        if self.bit(index) {
            SIGNAL_HIGH
        } else {
            SIGNAL_LOW
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Forgetting {
    pub body: FishSpecies,
    pub turn_clock: f32,
    pub shape_clock: f32,
    pub forget_clock: f32,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Side {
    pub body: FishSpecies,
    pub size: usize,
    pub eyes: usize,
}

impl Side {
    fn roll(rng: &mut impl RngExt) -> Self {
        Side {
            body: borrowed_body(rng),
            size: rng.random_range(SIDE_SIZE_MIN..=SIDE_SIZE_MAX),
            eyes: rng.random_range(1..=SIDE_EYES_MAX),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Verso {
    pub facing_left: bool,
    pub side: Side,
    pub other: Side,
    pub other_look: Option<Box<UnfishState>>,
    pub other_record: Option<Box<MutationRecord>>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Fault {
    pub clock: f32,
    pub slip: f32,
}

impl Fault {
    pub fn lag(&self) -> Option<i32> {
        if self.slip <= 0.0 {
            return None;
        }
        let progress = 1.0 - self.slip / FAULT_SLIP_SECS;
        Some((progress * FAULT_MAX_LAG).round() as i32)
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Anagram {
    pub glyphs: Vec<char>,
    pub eye: Option<usize>,
    pub clock: f32,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mood {
    #[default]
    Agrees,
    Stops,
    Strays,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Reflection {
    pub clock: f32,
    pub mood: Mood,
    pub mood_secs: f32,
    pub offset: f32,
}

impl Reflection {
    pub fn follow(&mut self, moved_dx: f32, dt: f32) {
        match self.mood {
            Mood::Agrees => self.offset -= self.offset * (REFLECTION_CATCH_UP * dt).min(1.0),
            Mood::Stops => self.offset -= moved_dx,
            Mood::Strays => self.offset -= 2.0 * moved_dx,
        }
        self.offset = self
            .offset
            .clamp(-REFLECTION_MAX_OFFSET, REFLECTION_MAX_OFFSET);
    }

    pub fn faces_away(&self) -> bool {
        self.mood == Mood::Strays
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Molt {
    pub clock: f32,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Graeae {
    pub sighted: bool,
    pub rest: f32,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Negative {
    pub rests: Vec<(String, f32)>,
}

impl Negative {
    pub fn rested(&self, name: &str) -> bool {
        !self.rests.iter().any(|(n, _)| n == name)
    }

    pub fn rest(&mut self, name: String, rng: &mut impl RngExt) {
        self.rests
            .push((name, sample_exponential(rng, NEGATIVE_REST_MEAN_SECS)));
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Ring {
    pub turn: f32,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub enum Quirk {
    #[default]
    Plain,
    Leech(Leech),
    Bones(Bones),
    Signal(Signal),
    Forgetting(Forgetting),
    Verso(Box<Verso>),
    Fault(Fault),
    Anagram(Anagram),
    Reflection(Reflection),
    Molt(Molt),
    Graeae(Graeae),
    Negative(Negative),
    Ring(Ring),
}

pub fn borrowable_bodies() -> Vec<FishSpecies> {
    ALL_SPECIES
        .iter()
        .copied()
        .filter(|species| standard_chars(*species).is_some())
        .collect()
}

fn standard_chars(species: FishSpecies) -> Option<BodyChars> {
    let config = species.config();
    if config.eye_at != EyeAt::Head {
        return None;
    }
    let (BodyTemplate::Standard(chars) | BodyTemplate::Alternating(chars, _)) = config.body else {
        return None;
    };
    (!matches!(chars.tail, TailKind::Swaying { .. })).then_some(chars)
}

pub fn borrowed_body(rng: &mut impl RngExt) -> FishSpecies {
    let bodies = borrowable_bodies();
    bodies[rng.random_range(0..bodies.len())]
}

pub fn borrowed_size(species: FishSpecies) -> usize {
    species.config().sizes[SizeCategory::M as usize].clamp(SIDE_SIZE_MIN, SIDE_SIZE_MAX)
}

pub fn chars_of(species: FishSpecies, eye: char) -> BodyChars {
    let chars = standard_chars(species).unwrap_or(BONES_CHARS);
    BodyChars {
        eye_left: eye,
        eye_right: eye,
        ..chars
    }
}

impl Quirk {
    pub fn new(kind: UnfishKind, rng: &mut impl RngExt) -> Self {
        match kind {
            UnfishKind::Leech => Quirk::Leech(Leech {
                bite_clock: sample_exponential(rng, LEECH_BITE_MEAN_SECS),
                ..Leech::default()
            }),
            UnfishKind::Bones => Quirk::Bones(Bones::default()),
            UnfishKind::Signal => Quirk::Signal(Signal {
                cheat: Cheat::ALL[rng.random_range(0..Cheat::ALL.len())],
                sent: 0,
            }),
            UnfishKind::Forgetting => Quirk::Forgetting(Forgetting {
                body: borrowed_body(rng),
                turn_clock: sample_exponential(rng, FORGET_TURN_MEAN_SECS),
                shape_clock: sample_exponential(rng, FORGET_SHAPE_MEAN_SECS),
                forget_clock: sample_exponential(rng, FORGET_MUTATION_MEAN_SECS),
            }),
            UnfishKind::Verso => Quirk::Verso(Box::new(Verso {
                facing_left: true,
                side: Side::roll(rng),
                other: Side::roll(rng),
                other_look: None,
                other_record: None,
            })),
            UnfishKind::Fault => Quirk::Fault(Fault {
                clock: sample_exponential(rng, FAULT_SLIP_MEAN_SECS),
                slip: 0.0,
            }),
            UnfishKind::Anagram => Quirk::Anagram(Anagram::default()),
            UnfishKind::Reflection => Quirk::Reflection(Reflection {
                clock: sample_exponential(rng, REFLECTION_DISAGREE_MEAN_SECS),
                ..Reflection::default()
            }),
            UnfishKind::Molt => Quirk::Molt(Molt {
                clock: sample_exponential(rng, MOLT_MEAN_SECS),
            }),
            UnfishKind::Graeae => Quirk::Graeae(Graeae::default()),
            UnfishKind::Negative => Quirk::Negative(Negative::default()),
            UnfishKind::Ouroboros => Quirk::Ring(Ring::default()),
            UnfishKind::Reversed
            | UnfishKind::Doppleganger
            | UnfishKind::Phantom
            | UnfishKind::Blinker
            | UnfishKind::Ball
            | UnfishKind::Skull
            | UnfishKind::Worm
            | UnfishKind::Absence
            | UnfishKind::Face
            | UnfishKind::Still => Quirk::Plain,
        }
    }

    pub fn tick(&mut self, dt: f32, rng: &mut impl RngExt) {
        match self {
            Quirk::Fault(fault) => {
                if fault.slip > 0.0 {
                    fault.slip = (fault.slip - dt).max(0.0);
                    return;
                }
                fault.clock -= dt;
                if fault.clock <= 0.0 {
                    fault.clock = sample_exponential(rng, FAULT_SLIP_MEAN_SECS);
                    fault.slip = FAULT_SLIP_SECS;
                }
            }
            Quirk::Reflection(reflection) => {
                if reflection.mood != Mood::Agrees {
                    reflection.mood_secs -= dt;
                    if reflection.mood_secs <= 0.0 {
                        reflection.mood = Mood::Agrees;
                    }
                    return;
                }
                reflection.clock -= dt;
                if reflection.clock <= 0.0 {
                    reflection.clock = sample_exponential(rng, REFLECTION_DISAGREE_MEAN_SECS);
                    reflection.mood = if rng.random::<bool>() {
                        Mood::Stops
                    } else {
                        Mood::Strays
                    };
                    reflection.mood_secs =
                        rng.random_range(REFLECTION_MOOD_MIN_SECS..REFLECTION_MOOD_MAX_SECS);
                }
            }
            Quirk::Ring(ring) => ring.turn += RING_CELLS_PER_SEC * dt,
            Quirk::Negative(negative) => {
                for (_, rest) in &mut negative.rests {
                    *rest -= dt;
                }
                negative.rests.retain(|(_, rest)| *rest > 0.0);
            }
            Quirk::Graeae(graeae) => graeae.rest = (graeae.rest - dt).max(0.0),
            Quirk::Molt(molt) => molt.clock -= dt,
            Quirk::Anagram(anagram) => anagram.clock -= dt,
            Quirk::Leech(leech) if leech.host.is_some() => leech.bite_clock -= dt,
            Quirk::Forgetting(forgetting) => {
                forgetting.turn_clock -= dt;
                forgetting.shape_clock -= dt;
                forgetting.forget_clock -= dt;
            }
            _ => {}
        }
    }
}
