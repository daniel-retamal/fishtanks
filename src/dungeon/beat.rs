use super::effect::StatusKind;
use super::fighter::Fighter;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tone {
    Plain,
    Hit,
    Hurt,
    Heal,
    Gold,
    Status,
    Tell,
    Round,
    Down,
    Quiet,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogLine {
    pub text: String,
    pub tone: Tone,
}

impl LogLine {
    pub fn new(text: impl Into<String>, tone: Tone) -> LogLine {
        LogLine {
            text: text.into(),
            tone,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pause {
    Think,
    Result,
    Round,
    None,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub hp: u32,
    pub max_hp: u32,
    pub mp: u32,
    pub max_mp: u32,
    pub rolling: u32,
    pub belly_up: bool,
    pub gone: bool,
    pub shield: u32,
    pub statuses: Vec<StatusKind>,
}

impl View {
    pub fn of(fighter: &Fighter) -> View {
        View {
            hp: fighter.hp,
            max_hp: fighter.max_hp,
            mp: fighter.mp,
            max_mp: fighter.max_mp,
            rolling: fighter.rolling,
            belly_up: fighter.belly_up,
            gone: fighter.gone,
            shield: fighter.shield,
            statuses: fighter.statuses.iter().map(|s| s.kind).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Beat {
    pub line: Option<LogLine>,
    pub actor: Option<usize>,
    pub pause: Pause,
    pub views: Vec<View>,
    pub order: Vec<usize>,
}
