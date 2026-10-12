use ratatui::style::Color;

use crate::colors::rgb_of;
use crate::fishes::fish::Fish;

use super::kit::Kit;
use super::life::DeepLife;

pub mod flavour;
pub mod marks;
pub mod mutations;
pub mod parts;
pub mod species;
pub mod toys;
pub mod unfish;

pub fn fit(kit: &mut Kit, fish: &Fish, life: &DeepLife) {
    mutations::fit(kit, fish);
    species::fit(kit, fish);
    parts::fit(kit, fish);
    flavour::fit(kit, fish);
    marks::fit(kit, fish, life);
    kit.settle();
}

const DARK_BELOW: u32 = 128;
const CHANNELS: u32 = 3;

pub fn is_dark(colour: Color) -> bool {
    let (r, g, b) = rgb_of(colour);
    (u32::from(r) + u32::from(g) + u32::from(b)) / CHANNELS < DARK_BELOW
}
