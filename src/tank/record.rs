use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{Arrival, ChannelRegistry, Tank, TankKind, WorldSignal};
use crate::economy::Money;
use crate::entities::cow::Cow;
use crate::fishes::fish::Fish;
use crate::fishes::parts::Part;

#[derive(Serialize, Deserialize)]
pub enum Cargo {
    Fish(Fish),
    Cow(Cow),
}

#[derive(Serialize, Deserialize)]
pub struct TankRecord {
    name: String,
    kind: TankKind,
    seed: u64,
    fish: Vec<Fish>,
    cows: Vec<Cow>,
    cargo: Vec<Cargo>,
    channels: ChannelRegistry,
    extra_capacity: u32,
    boundless: bool,
    cow_abduction_count: u32,
    mutation_timer: f32,
    rad_weight_timer: f32,
    void_spawn_timer: f32,
    ufo_timer: f32,
    pending_star_cash: Money,
    pending_graveyard: Vec<Fish>,
    pending_loose_parts: Vec<Part>,
    #[serde(alias = "pending_exiles")]
    pending_arrivals: Vec<Arrival>,
    pending_signals: BTreeSet<WorldSignal>,
}

impl TankRecord {
    pub fn capture(tank: &Tank) -> Self {
        let Tank {
            name,
            kind,
            seed,
            scenery: _,
            fish,
            cows,
            food: _,
            background: _,
            channels,
            bubbles: _,
            width: _,
            height: _,
            used_names: _,
            used_cow_names: _,
            pending_star_cash,
            pending_graveyard,
            pending_loose_parts,
            pending_arrivals,
            pending_signals,
            extra_capacity,
            boundless,
            cow_abduction_count,
            bubble_spawner: _,
            mutation_timer,
            rad_weight_timer,
            void_spawn_timer,
            ufo_timer,
            ufos,
            candy_tick: _,
            candy_scan: _,
            milk_clock: _,
            sky: _,
            trails: _,
            inks: _,
            sheddings: _,
            watched: _,
        } = tank;
        let cargo = ufos
            .iter()
            .filter_map(|ufo| {
                ufo.carried_fish()
                    .map(|fish| Cargo::Fish(fish.clone()))
                    .or_else(|| ufo.carried_cow().map(|cow| Cargo::Cow(cow.clone())))
            })
            .collect();
        Self {
            name: name.clone(),
            kind: *kind,
            seed: *seed,
            fish: fish.clone(),
            cows: cows.clone(),
            cargo,
            channels: channels.clone(),
            extra_capacity: *extra_capacity,
            boundless: *boundless,
            cow_abduction_count: *cow_abduction_count,
            mutation_timer: *mutation_timer,
            rad_weight_timer: *rad_weight_timer,
            void_spawn_timer: *void_spawn_timer,
            ufo_timer: *ufo_timer,
            pending_star_cash: *pending_star_cash,
            pending_graveyard: pending_graveyard.clone(),
            pending_loose_parts: pending_loose_parts.clone(),
            pending_arrivals: pending_arrivals.clone(),
            pending_signals: pending_signals.clone(),
        }
    }

    pub fn food_in_the_water(tank: &Tank) -> u32 {
        tank.food.iter().filter(|food| !food.eaten).count() as u32
    }

    pub fn restore(self, width: u16, height: u16, dead_names: &[String]) -> Tank {
        let mut tank = Tank::seeded(self.name, self.kind, dead_names, self.seed);
        tank.resize(width, height, dead_names);
        let mut rng = rand::rng();
        for mut fish in self.fish {
            if !fish.remembers_its_place() {
                fish.position = tank.spawn_point(&mut rng);
            }
            tank.used_names.insert(fish.name.clone());
            tank.fish.push(fish);
        }
        for mut cow in self.cows {
            cow.position.x = tank.cow_spot(&cow, &mut rng);
            tank.used_cow_names.insert(cow.name.clone());
            tank.cows.push(cow);
        }
        tank.resize(width, height, dead_names);
        for cargo in self.cargo {
            match cargo {
                Cargo::Fish(fish) => {
                    let name = fish.name.clone();
                    tank.place_fish(fish, name, &mut rng);
                }
                Cargo::Cow(mut cow) => {
                    cow.position.x = tank.cow_spot(&cow, &mut rng);
                    tank.land_cow_delivery(cow);
                }
            }
        }
        tank.channels = self.channels;
        tank.extra_capacity = self.extra_capacity;
        tank.boundless = self.boundless;
        tank.cow_abduction_count += self.cow_abduction_count;
        tank.mutation_timer = self.mutation_timer;
        tank.rad_weight_timer = self.rad_weight_timer;
        tank.void_spawn_timer = self.void_spawn_timer;
        tank.ufo_timer = self.ufo_timer;
        tank.pending_star_cash = self.pending_star_cash;
        tank.pending_graveyard = self.pending_graveyard;
        tank.pending_loose_parts = self.pending_loose_parts;
        tank.pending_arrivals.extend(self.pending_arrivals);
        tank.pending_signals.extend(self.pending_signals);
        tank
    }
}
