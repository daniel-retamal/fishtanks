use crate::consumable::ConsumeTarget;
use crate::dungeon::life::DeepLife;
use crate::dungeon::treasure::{RADIANT_SECS, SHARDS_PER_WISH, Treasure, Use};
use crate::fishes::fish::Fish;
use crate::loot::{ConsumableKind, StockItem};
use crate::ui::consume_picker::{ConsumePickerEntry, ConsumePickerSource};
use crate::void_ritual::{self, VoidRitualState};

use super::{App, Overlay};

const DEAD: &str = "graveyard";

impl App {
    pub(super) fn use_treasure(&mut self, treasure: Treasure, source: ConsumePickerSource) -> bool {
        let stock = StockItem::Consumable(ConsumableKind::Treasure(treasure));
        match treasure.using() {
            Use::Casts { .. } => {
                self.consume_item(ConsumableKind::Treasure(treasure));
                self.spend_one(stock);
                true
            }
            Use::OnFish | Use::OnTheDead => {
                self.open_consume_picker(ConsumeTarget::Treasure(treasure), source)
            }
            Use::OnTheTank => {
                if !self.set_in_the_tank(treasure) {
                    return false;
                }
                self.spend_one(stock);
                true
            }
            Use::Wish => self.make_a_wish(stock),
            Use::AtTheTable | Use::OnTheShelf | Use::InTheDeep => false,
        }
    }

    pub(super) fn spend_one(&mut self, stock: StockItem) {
        let entry = self.inventory.entry(stock).or_insert(0);
        *entry = entry.saturating_sub(1);
        self.inventory.retain(|_, v| *v > 0);
        let now_empty = if let Some(Overlay::Inventory(state)) = &mut self.active_overlay {
            state.update_from(&self.inventory, &self.blueprints, &mut rand::rng());
            state.items.is_empty()
        } else {
            false
        };
        if now_empty {
            self.close_overlay();
        }
        let bh = self.bar_height();
        let tw = self.terminal_width;
        let th = self.terminal_height.saturating_sub(bh);
        let dead_names = self.graveyard_names();
        self.tanks[self.current_tank].resize(tw, th, &dead_names);
    }

    fn set_in_the_tank(&mut self, treasure: Treasure) -> bool {
        let tank = self.current_tank;
        match treasure {
            Treasure::RadiantCore => {
                self.tanks[tank].radiance += RADIANT_SECS;
                true
            }
            Treasure::Cowbell(variant) => {
                if !self.tanks[tank].welcomes_cows() {
                    return false;
                }
                self.plan_cow_delivery(tank, variant, &mut rand::rng());
                true
            }
            Treasure::MammonsCoin => {
                self.dungeon.mammon += 1;
                true
            }
            _ => false,
        }
    }

    fn make_a_wish(&mut self, stock: StockItem) -> bool {
        let held = self.inventory.get(&stock).copied().unwrap_or(0);
        if held < SHARDS_PER_WISH || !matches!(self.void_ritual, VoidRitualState::Idle { .. }) {
            return false;
        }
        for _ in 0..SHARDS_PER_WISH {
            self.spend_one(stock);
        }
        self.close_overlay();
        self.editor.clear();
        self.void_ritual = VoidRitualState::Wish {
            retries_left: void_ritual::MAX_WISH_RETRIES,
        };
        true
    }

    pub(super) fn the_dead_for(&self, treasure: Treasure) -> Vec<ConsumePickerEntry> {
        if treasure.using() != Use::OnTheDead {
            return Vec::new();
        }
        self.graveyard
            .iter()
            .enumerate()
            .map(|(grave, fish)| ConsumePickerEntry {
                fish_name: fish.name.clone(),
                species_display: fish.species.display_name().to_string(),
                tank_name: fish
                    .fate
                    .map_or(DEAD.to_string(), |fate| fate.epitaph().to_string()),
                fed: None,
                tank_idx: self.current_tank,
                fish_idx: grave,
            })
            .collect()
    }

    pub(super) fn give_treasure(
        &mut self,
        treasure: Treasure,
        tank: usize,
        entry: usize,
        name: &str,
    ) -> bool {
        if treasure.using() == Use::OnTheDead {
            return self
                .graveyard
                .get(entry)
                .is_some_and(|fish| fish.name == name)
                && self.raise_from_grave(entry, tank);
        }
        if let Treasure::Vial(mutation) = treasure {
            return self.tanks[tank].apply_named_mutation(name, mutation.token());
        }
        let Some(at) = self.tanks[tank].fish.iter().position(|f| f.name == name) else {
            return false;
        };
        let fish = &mut self.tanks[tank].fish[at];
        match treasure {
            Treasure::Chip(salvage) => {
                let Some(bot) = fish.script_mut() else {
                    return false;
                };
                let salvaged = bot.etch(salvage.chip());
                for (part, count) in salvaged {
                    self.stock_up(StockItem::Consumable(ConsumableKind::Part(part)), count);
                }
                true
            }
            Treasure::Feather => {
                deep_life(fish).feather = true;
                true
            }
            _ => false,
        }
    }
}

pub(super) fn wear_charm(fish: &mut Fish, charm: crate::dungeon::boon::Charm) -> bool {
    let life = deep_life(fish);
    if life.charm.is_some() {
        return false;
    }
    life.charm = Some(charm);
    true
}

fn deep_life(fish: &mut Fish) -> &mut DeepLife {
    fish.deep
        .get_or_insert_with(|| Box::new(DeepLife::default()))
}

#[cfg(test)]
mod tests;
