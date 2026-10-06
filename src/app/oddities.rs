use crate::fishes::quirk::Quirk;

use super::App;

impl App {
    pub(super) fn watch_the_current_tank(&mut self) {
        let current = self.current_tank;
        for (i, tank) in self.tanks.iter_mut().enumerate() {
            tank.watched = i == current;
        }
    }

    pub(super) fn settle_graeae(&mut self) {
        let mut seen = false;
        let mut first = None;
        for (t, tank) in self.tanks.iter_mut().enumerate() {
            for (f, fish) in tank.fish.iter_mut().enumerate() {
                let Some(Quirk::Graeae(graeae)) = fish.quirk_mut() else {
                    continue;
                };
                first.get_or_insert((t, f));
                if graeae.sighted && seen {
                    graeae.sighted = false;
                }
                seen |= graeae.sighted;
            }
        }
        if seen {
            return;
        }
        let Some((t, f)) = first else {
            return;
        };
        if let Some(Quirk::Graeae(graeae)) = self.tanks[t].fish[f].quirk_mut() {
            graeae.sighted = true;
        }
    }
}
