use std::time::{Duration, Instant};

use super::{Tank, Transmission, WorldView};

pub const STAGE_BUDGET: Duration = Duration::from_millis(5);

pub struct StageBudget {
    deadline: Instant,
}

impl Default for StageBudget {
    fn default() -> Self {
        Self::new()
    }
}

impl StageBudget {
    pub fn new() -> Self {
        Self {
            deadline: Instant::now() + STAGE_BUDGET,
        }
    }

    pub fn spent(&self) -> bool {
        Instant::now() >= self.deadline
    }
}

impl Tank {
    pub fn advance_stage(&mut self, world: &mut WorldView) {
        Tank::advance_together(std::slice::from_mut(self), std::slice::from_mut(world));
    }

    pub fn advance_together(tanks: &mut [Tank], worlds: &mut [WorldView]) {
        for world in worlds.iter_mut() {
            world.tune_in(tanks);
        }
        let mut sent = Vec::new();
        for (tank, world) in tanks.iter_mut().zip(worlds.iter()) {
            sent.extend(tank.step_fish(world));
        }
        for transmission in &sent {
            transmission.land(tanks);
        }
        for (tank, world) in tanks.iter_mut().zip(worlds.iter_mut()) {
            tank.channels.commit();
            world.settle();
        }
    }

    fn step_fish(&mut self, world: &WorldView) -> Vec<Transmission> {
        let mut sent = Vec::new();
        for fish in &mut self.fish {
            if let Some(bot) = fish.script_mut() {
                sent.extend(bot.step(&mut self.channels, world));
            }
            if let Some(level) = fish.broadcast() {
                self.channels.drive(&fish.name, level);
            }
        }
        sent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fishes::species::FishSpecies;
    use crate::tank::TankKind;

    const STAGES_WATCHED: usize = 6;

    fn tank_with(names: &[&str]) -> Tank {
        let mut tank = Tank::new("Board".to_string(), TankKind::Matrix, &[]);
        let mut rng = rand::rng();
        for name in names {
            tank.spawn_fish(FishSpecies::Botfish, name.to_string(), &mut rng);
        }
        tank
    }

    fn step(tank: &mut Tank) {
        let mut world = tank.observe(0, 0);
        tank.advance_stage(&mut world);
    }

    fn wire(tank: &mut Tank, name: &str, listens: &[&str], drives: &str) {
        let bot = tank
            .fish
            .iter_mut()
            .find(|f| f.name == name)
            .expect("the fish exists")
            .script_mut()
            .expect("the fish is programmable");
        for channel in listens {
            bot.listen(channel);
        }
        bot.drive(drives);
    }

    #[test]
    fn a_fish_drives_the_or_of_everything_it_listens_to() {
        let mut tank = tank_with(&["Or"]);
        wire(&mut tank, "Or", &["a", "b"], "q");

        step(&mut tank);
        assert!(!tank.channels.level("q"), "both inputs are low");

        tank.channels.set_level("b", true);
        step(&mut tank);
        assert!(tank.channels.level("q"), "one high input is enough");
    }

    #[test]
    fn a_signal_walks_one_fish_per_stage() {
        let mut tank = tank_with(&["A", "B", "C"]);
        wire(&mut tank, "A", &["x"], "a");
        wire(&mut tank, "B", &["a"], "b");
        wire(&mut tank, "C", &["b"], "c");
        tank.channels.set_level("x", true);

        step(&mut tank);
        assert!(tank.channels.level("a"));
        assert!(!tank.channels.level("b"), "the fabric never settles ahead");
        assert!(!tank.channels.level("c"));

        step(&mut tank);
        assert!(tank.channels.level("b"));
        assert!(!tank.channels.level("c"));

        step(&mut tank);
        assert!(tank.channels.level("c"), "three fish, three stages");
    }

    #[test]
    fn two_fish_on_one_wire_merge_as_or() {
        let mut tank = tank_with(&["L", "R"]);
        wire(&mut tank, "L", &["l"], "bus");
        wire(&mut tank, "R", &["r"], "bus");

        tank.channels.set_level("l", true);
        step(&mut tank);
        assert!(tank.channels.level("bus"), "one driver pulls the bus high");

        tank.channels.set_level("l", false);
        tank.channels.set_level("r", true);
        step(&mut tank);
        assert!(tank.channels.level("bus"), "the other driver holds it up");

        tank.channels.set_level("r", false);
        step(&mut tank);
        assert!(!tank.channels.level("bus"), "no driver left, the bus falls");
    }

    #[test]
    fn a_channel_nobody_drives_holds_its_level_forever() {
        let mut tank = tank_with(&["A"]);
        wire(&mut tank, "A", &["x"], "a");
        tank.channels.set_level("x", true);

        for _ in 0..STAGES_WATCHED {
            step(&mut tank);
        }

        assert!(tank.channels.level("x"), "an input is not consumed");
        assert!(tank.channels.level("a"));
    }

    #[test]
    fn a_fish_that_drives_nothing_changes_nothing() {
        let mut tank = tank_with(&["Deaf"]);
        tank.fish[0]
            .script_mut()
            .expect("the fish is programmable")
            .listen("x");
        tank.channels.set_level("x", true);

        step(&mut tank);

        assert_eq!(tank.channels.len(), 1, "only the wire the test made");
    }

    #[test]
    fn an_ordinary_fish_is_not_part_of_the_fabric() {
        let mut tank = Tank::new("Board".to_string(), TankKind::Matrix, &[]);
        tank.spawn_fish(FishSpecies::Merluza, "Mer".to_string(), &mut rand::rng());

        step(&mut tank);

        assert!(tank.channels.is_empty());
    }

    use crate::fishes::parts::Part;

    fn tank_named(name: &str, fish: &[&str]) -> Tank {
        let mut tank = tank_with(fish);
        tank.name = name.to_string();
        tank
    }

    fn mast(tank: &mut Tank, fish: &str, far: (&str, &str), wires: (Option<&str>, Option<&str>)) {
        let bot = tank
            .fish
            .iter_mut()
            .find(|f| f.name == fish)
            .expect("the fish exists")
            .script_mut()
            .expect("the fish is programmable");
        bot.install(Part::RelayMast);
        let config = Part::RelayMast.config();
        bot.configure(Part::RelayMast, &config[0], far.0);
        bot.configure(Part::RelayMast, &config[1], far.1);
        if let Some(channel) = wires.0 {
            bot.wire(Part::RelayMast, "in", channel);
        }
        if let Some(channel) = wires.1 {
            bot.wire(Part::RelayMast, "out", channel);
        }
    }

    fn step_all(tanks: &mut [Tank]) {
        let mut worlds: Vec<WorldView> = tanks.iter_mut().map(|t| t.observe(0, 0)).collect();
        Tank::advance_together(tanks, &mut worlds);
    }

    #[test]
    fn a_mast_carries_its_in_wire_to_one_channel_of_one_far_tank_in_one_stage() {
        let mut tanks = vec![
            tank_named("Home", &["Tx"]),
            tank_named("Zion", &[]),
            tank_named("Eden", &[]),
        ];
        mast(&mut tanks[0], "Tx", ("Zion", "y"), (Some("x"), None));
        tanks[0].channels.set_level("x", true);

        step_all(&mut tanks);

        assert!(tanks[1].channels.level("y"), "one stage, like any fish");
        assert_eq!(tanks[1].channels.len(), 1, "exactly one channel is bridged");
        assert!(!tanks[2].channels.level("y"), "only the configured tank");
        assert!(
            !tanks[0].channels.level("y"),
            "the home tank's y is its own wire"
        );

        tanks[0].channels.set_level("x", false);
        step_all(&mut tanks);
        assert!(
            !tanks[1].channels.level("y"),
            "the far wire follows the near one down"
        );
    }

    #[test]
    fn a_masts_out_reads_the_far_channel_one_stage_later_like_any_listener() {
        let mut tanks = vec![tank_named("Home", &["Rx"]), tank_named("Zion", &[])];
        mast(&mut tanks[0], "Rx", ("Zion", "y"), (None, Some("heard")));
        tanks[1].channels.set_level("y", true);

        step_all(&mut tanks);
        assert!(tanks[0].channels.level("heard"));
        assert_eq!(
            tanks[1].channels.len(),
            1,
            "a mast whose in is unwired puts nothing on the far wire"
        );
    }

    #[test]
    fn a_crossing_costs_the_same_stage_whichever_way_the_tanks_are_listed() {
        for (home, far) in [(0, 1), (1, 0)] {
            let mut tanks = vec![tank_named("A", &["Fish"]), tank_named("B", &["Fish"])];
            let far_name = tanks[far].name.clone();
            mast(
                &mut tanks[home],
                "Fish",
                (&far_name, "y"),
                (Some("x"), Some("echo")),
            );
            tanks[home].channels.set_level("x", true);

            step_all(&mut tanks);
            assert!(tanks[far].channels.level("y"), "sent {home} → {far}");
            assert!(
                !tanks[home].channels.level("echo"),
                "the read-back sees the far wire as it was before the stage"
            );

            step_all(&mut tanks);
            assert!(
                tanks[home].channels.level("echo"),
                "read back {far} → {home}"
            );
        }
    }

    #[test]
    fn the_far_wire_merges_with_its_own_drivers_as_or() {
        let mut tanks = vec![tank_named("Home", &["Tx"]), tank_named("Zion", &["Or"])];
        mast(&mut tanks[0], "Tx", ("Zion", "y"), (Some("x"), None));
        wire(&mut tanks[1], "Or", &["local"], "y");

        tanks[1].channels.set_level("local", true);
        step_all(&mut tanks);
        assert!(
            tanks[1].channels.level("y"),
            "the far tank's own driver holds it up"
        );

        tanks[1].channels.set_level("local", false);
        tanks[0].channels.set_level("x", true);
        step_all(&mut tanks);
        assert!(tanks[1].channels.level("y"), "the mast holds it up");

        tanks[0].channels.set_level("x", false);
        step_all(&mut tanks);
        assert!(
            !tanks[1].channels.level("y"),
            "no driver left on either side"
        );
    }

    #[test]
    fn a_mast_aimed_at_a_tank_that_is_not_there_bridges_nothing() {
        let mut tanks = vec![tank_named("Home", &["Tx"]), tank_named("Zion", &[])];
        mast(
            &mut tanks[0],
            "Tx",
            ("Atlantis", "y"),
            (Some("x"), Some("echo")),
        );
        tanks[0].channels.set_level("x", true);

        step_all(&mut tanks);
        step_all(&mut tanks);

        assert!(tanks[1].channels.is_empty());
        assert!(!tanks[0].channels.level("echo"));
    }

    #[test]
    fn a_mast_aimed_at_its_own_tank_is_a_buffer() {
        let mut tank = tank_named("Home", &["Tx"]);
        mast(&mut tank, "Tx", ("Home", "y"), (Some("x"), None));
        tank.channels.set_level("x", true);

        step(&mut tank);

        assert!(tank.channels.level("y"), "one stage, like a bare fish");
    }
}
