use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{BotfishState, Chip};

#[derive(Serialize, Deserialize)]
pub struct ChipRecord {
    name: String,
    board: Vec<BotfishState>,
    inputs: BTreeSet<String>,
    outputs: BTreeSet<String>,
    held: Vec<bool>,
    heard: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    sealed: bool,
}

impl From<Chip> for ChipRecord {
    fn from(chip: Chip) -> Self {
        let lines = |terminals: &[super::Terminal]| {
            terminals
                .iter()
                .map(|terminal| terminal.line.clone())
                .collect()
        };
        Self {
            inputs: lines(&chip.compiled.inputs),
            outputs: lines(&chip.compiled.outputs),
            held: chip.compiled.levels.held,
            heard: chip.heard,
            sealed: chip.sealed,
            name: chip.name,
            board: chip.board,
        }
    }
}

impl From<ChipRecord> for Chip {
    fn from(record: ChipRecord) -> Self {
        let mut chip = Chip::new(record.name, record.board, record.inputs, record.outputs);
        if record.held.len() == chip.compiled.levels.held.len() {
            chip.compiled.levels.held = record.held;
        }
        chip.heard = record.heard;
        chip.sealed = record.sealed;
        chip
    }
}
