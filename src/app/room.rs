use crate::entities::cow::Cow;
use crate::fishes::fish::Fish;
use crate::fishes::species::FishSpecies;
use crate::tank::Arrival;

use super::App;

impl App {
    pub(super) fn landing_tank(&self, from: usize, fish: &Fish) -> Option<usize> {
        self.tanks_from(from)
            .find(|&index| self.tanks[index].has_room_for(fish))
    }

    pub(super) fn has_room_for_a_new(&self, species: FishSpecies) -> bool {
        let newborn = Fish::new_for_display(species, &mut rand::rng());
        self.landing_tank(self.current_tank, &newborn).is_some()
    }

    pub(super) fn land_fish(
        &mut self,
        from: usize,
        fish: Fish,
        name: String,
    ) -> Result<usize, Box<Fish>> {
        let Some(to) = self.landing_tank(from, &fish) else {
            return Err(Box::new(fish));
        };
        self.tanks[to].place_fish(fish, name, &mut rand::rng());
        Ok(to)
    }

    pub(super) fn land_or_wait(&mut self, from: usize, fish: Fish, name: String) -> Option<usize> {
        match self.land_fish(from, fish, name.clone()) {
            Ok(to) => Some(to),
            Err(mut waiting) => {
                waiting.name = name;
                self.tanks[from]
                    .pending_arrivals
                    .push(Arrival::Fish(waiting));
                None
            }
        }
    }

    pub(super) fn settle_arrivals(&mut self) {
        for from in 0..self.tanks.len() {
            for arrival in std::mem::take(&mut self.tanks[from].pending_arrivals) {
                match arrival {
                    Arrival::Fish(fish) => {
                        let name = fish.name.clone();
                        self.land_or_wait(from, *fish, name);
                    }
                    Arrival::Cow(cow) => self.land_cow(from, *cow),
                }
            }
        }
    }

    pub(super) fn land_cow(&mut self, from: usize, cow: Cow) {
        let pasture = self
            .tanks_from(from)
            .find(|&index| self.tanks[index].welcomes_cows());
        match pasture {
            Some(to) => self.tanks[to].place_cow(cow, &mut rand::rng()),
            None => self.tanks[from]
                .pending_arrivals
                .push(Arrival::Cow(Box::new(cow))),
        }
    }

    fn tanks_from(&self, from: usize) -> impl Iterator<Item = usize> + use<> {
        let count = self.tanks.len();
        (0..count).map(move |step| (from + step) % count)
    }
}
