use crate::fishes::fish::Fish;
use crate::names::NameBook;
use crate::tank::Tank;

use super::App;

impl App {
    fn name_books(&self) -> Option<(NameBook, NameBook)> {
        self.tanks
            .first()
            .map(|tank| (tank.used_names.clone(), tank.used_cow_names.clone()))
    }

    pub(super) fn enrol(&self, tank: &mut Tank) {
        if let Some((fish, cows)) = self.name_books() {
            tank.join_names(&fish, &cows);
        }
    }

    pub(super) fn enrol_every_tank(&mut self) {
        let fish = NameBook::default();
        let cows = NameBook::default();
        for tank in &mut self.tanks {
            tank.join_names(&fish, &cows);
        }
        let Some(expedition) = self.dungeon.expedition.as_mut() else {
            return;
        };
        for member in &mut expedition.party {
            if fish.contains(&member.name) {
                member.name = fish.renamed(&member.name);
            }
            fish.insert(member.name.clone());
        }
    }

    pub(super) fn hold_name(&self, name: &str) {
        if let Some((fish, _)) = self.name_books() {
            fish.insert(name.to_string());
        }
    }

    pub(super) fn free_name(&self, name: &str) {
        if let Some((fish, _)) = self.name_books() {
            fish.remove(name);
        }
    }

    pub(super) fn name_a_newcomer(&self, fish: &mut Fish) {
        if let Some((book, _)) = self.name_books() {
            fish.name = book.unique(&fish.name);
        }
    }
}
