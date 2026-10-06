use rand::RngExt;

use crate::colors::{LIGHT_YELLOW, WHITE};
use crate::entities::bubble::{Bubble, BubblePhase};
use crate::entities::components::Position;
use crate::entities::cow::Cow;
use crate::fishes::fish::{
    BLESSING_GLOW_SECS, BLESSING_INTERVAL_SECS, Direction, EATING_DURATION, Fish, FishState,
};
use crate::fishes::mutations::{MutantBacked, Mutatable, Mutation, apply_mutation};
use crate::fishes::species::{FishSpecies, Habit};
use crate::fishes::unfish::{
    PHANTOM_CROSS_TANK_CHANCE, PHANTOM_TELEPORT_MEAN, SPAWNABLE_UNFISH, UnfishKind,
    VOID_SPAWN_MEAN_SECS, is_multi_row,
};
use crate::names;
use crate::settings::Settings;
use crate::util::{events_in, sample_exponential};

use super::{BLIND_SMELL_RADIUS, Tank, TankEvent};
use super::{
    SEEK_BOOST_GROWTH, SEEK_BOOST_INITIAL_MAX, SEEK_DX_DEADZONE, SEEK_DY_MULTIPLIER, SEEK_NORM_MIN,
    ZOOMIE_BUBBLES_PER_SEC,
};
use crate::entities::food::{CANDY_GAIN_MULT, FOOD_WEIGHT_GAIN_G};

fn sq(x: f32) -> f32 {
    x * x
}

const CANDYFISH_SCANS_PER_MINUTE: f32 = 18.0;
const SECS_PER_MINUTE: f32 = 60.0;
const CASHFISH_ZOOMIE_CASH: u32 = 100;
const CANDYFISH_TOUCH_WEIGHT_G: u32 = 1;
const ENGULF_REACH: f32 = 3.0;
const ENGULF_FISH_Y_TOLERANCE: f32 = 2.0;
const ENGULF_COW_Y_TOLERANCE: f32 = 5.0;

fn horizontally_near(ax: f32, aw: usize, bx: f32, bw: usize) -> bool {
    let ax2 = ax + aw as f32;
    let bx2 = bx + bw as f32;
    ax <= bx2 + ENGULF_REACH && bx <= ax2 + ENGULF_REACH
}

fn is_worm(fish: &Fish) -> bool {
    fish.unfish_state
        .as_ref()
        .is_some_and(|us| us.kind == UnfishKind::Worm)
}

fn engulf_compatible(receiver: &Fish, candidate: &Fish) -> bool {
    let receiver_is_worm = is_worm(receiver);
    match candidate.unfish_state.as_ref() {
        Some(us) if is_multi_row(us.kind) => false,
        Some(us) if us.kind == UnfishKind::Worm => receiver_is_worm,
        _ => !receiver_is_worm,
    }
}

pub(super) fn fish_near(a: &Fish, b: &Fish) -> bool {
    horizontally_near(a.position.x, a.display_width, b.position.x, b.display_width)
        && (a.position.y - b.position.y).abs() <= ENGULF_FISH_Y_TOLERANCE
}

fn cow_near(a: &Cow, b: &Cow) -> bool {
    horizontally_near(a.position.x, a.display_width, b.position.x, b.display_width)
        && (a.position.y - b.position.y).abs() <= ENGULF_COW_Y_TOLERANCE
}

impl Tank {
    pub(super) fn spawn_bubbles(&mut self, dt: f32) {
        if self.kind.config().bubble_rate_mult <= 0.0 {
            return;
        }
        if self.width == 0 || self.height == 0 {
            return;
        }
        let mut rng = rand::rng();

        let bubble_color = self
            .background
            .bubble_color_override()
            .unwrap_or(self.kind.config().bubble_color);

        let spawn_dt = dt * self.kind.config().bubble_rate_mult;
        let new_bubbles =
            self.bubble_spawner
                .tick(spawn_dt, self.width, self.height, bubble_color, &mut rng);
        self.bubbles.extend(new_bubbles);

        for fish in &self.fish {
            for _ in 0..events_in(&mut rng, ZOOMIE_BUBBLES_PER_SEC, fish.zoomed_secs()) {
                let tail_x = match fish.facing {
                    Direction::Left => fish.position.x + fish.display_width as f32 - 1.0,
                    Direction::Right => fish.position.x,
                };
                let cash_stacks = fish.ability_stacks(FishSpecies::Cashfish);
                let mut bubble = if cash_stacks > 0 {
                    let mut b = Bubble::new(
                        tail_x,
                        fish.position.y,
                        BubblePhase::rising(&mut rng),
                        LIGHT_YELLOW,
                        Some('$'),
                        &mut rng,
                    );
                    b.cash_value = Some(CASHFISH_ZOOMIE_CASH * cash_stacks);
                    b.poppable = false;
                    b
                } else if fish.ability_stacks(FishSpecies::Holyfish) > 0 {
                    let mut b = Bubble::new(
                        tail_x,
                        fish.position.y,
                        BubblePhase::rising(&mut rng),
                        WHITE,
                        None,
                        &mut rng,
                    );
                    b.poppable = false;
                    b
                } else {
                    Bubble::new(
                        tail_x,
                        fish.position.y,
                        BubblePhase::rising(&mut rng),
                        fish.species
                            .config()
                            .zoomie_bubble_color
                            .resolve(bubble_color, fish.color),
                        None,
                        &mut rng,
                    )
                };
                if let Some(c) = fish.wake_color() {
                    bubble.color = c;
                }
                self.bubbles.push(bubble);
            }
        }
    }

    pub(super) fn steer_seeking_fish(&mut self) {
        for i in 0..self.fish.len() {
            let (idx, approach_right) = match self.fish[i].state {
                FishState::SeekingFood {
                    food_idx,
                    approach_right,
                } => (food_idx, approach_right),
                _ => continue,
            };
            if idx >= self.food.len() {
                self.fish[i].cancel_seek();
                continue;
            }

            self.fish[i].seek_boost += SEEK_BOOST_GROWTH / (1.0 + self.fish[i].seek_boost);

            let food_x = self.food[idx].position.x;
            let food_y = self.food[idx].position.y;
            let fish_len = self.fish[i].display_width as f32;
            let fish_y = self.fish[i].position.y;

            let target_x = if approach_right {
                food_x - (fish_len - 1.0)
            } else {
                food_x
            };

            let raw_dx = target_x - self.fish[i].position.x;
            let dx = if raw_dx.abs() < SEEK_DX_DEADZONE {
                0.0
            } else {
                raw_dx
            };
            let raw_dy = food_y - fish_y;
            let dy = if self.food[idx].settled && fish_y as i32 >= food_y as i32 {
                0.0
            } else {
                raw_dy
            };
            let norm = (dx * dx + dy * dy).sqrt().max(SEEK_NORM_MIN);
            let seek_speed = self.fish[i].speed + self.fish[i].seek_boost;
            self.fish[i].velocity.dx = (dx / norm) * seek_speed;
            self.fish[i].velocity.dy = (dy / norm) * seek_speed * SEEK_DY_MULTIPLIER;
            self.fish[i].facing = if approach_right {
                Direction::Right
            } else {
                Direction::Left
            };
        }
    }

    pub(super) fn tick_fish(&mut self, settings: &Settings, coffee: u32) {
        for fish in &mut self.fish {
            fish.tick(settings, self.width, self.height, coffee);
        }
    }

    pub(super) fn check_eating_collisions(&mut self) {
        for i in 0..self.fish.len() {
            let idx = match self.fish[i].state {
                FishState::SeekingFood { food_idx, .. } => food_idx,
                _ => continue,
            };
            if idx >= self.food.len() || self.food[idx].eaten {
                continue;
            }
            let head_x = self.fish[i].head_x();
            let head_y = self.fish[i].position.y as i32;
            let fx = self.food[idx].position.x as i32;
            let fy = self.food[idx].position.y as i32;
            if (head_x - fx).abs() <= 1 && head_y == fy {
                self.bite(i, idx);
            }
        }
    }

    pub(super) fn bite(&mut self, i: usize, idx: usize) {
        self.fish[i].state = FishState::Eating {
            time_remaining: EATING_DURATION,
        };
        if self.fish[i].ability_stacks(FishSpecies::Candyfish) > 0 {
            self.food[idx].is_candy = true;
            return;
        }
        self.food[idx].eaten = true;
        let gain = if self.food[idx].is_candy {
            FOOD_WEIGHT_GAIN_G * CANDY_GAIN_MULT
        } else {
            FOOD_WEIGHT_GAIN_G
        };
        self.fish[i].eat(gain);
    }

    pub(super) fn assign_food_to_idle_fish(&mut self) {
        if self.food.is_empty() {
            return;
        }
        for i in 0..self.fish.len() {
            if !matches!(
                self.fish[i].state,
                FishState::Idle | FishState::Zoomie { .. }
            ) {
                continue;
            }
            let fish = &self.fish[i];
            if fish.is_wired()
                || fish.is_pinned()
                || fish.is_asleep()
                || !fish.locomotion().swims()
                || !fish.seeks_food()
            {
                continue;
            }
            let fish_len = self.fish[i].display_width as f32;
            let head_x = self.fish[i].head_x() as f32;
            let fish_y = self.fish[i].position.y;
            let smell = if self.fish[i].habit() == Some(Habit::Blind) {
                BLIND_SMELL_RADIUS
            } else {
                f32::INFINITY
            };
            let nearest_idx = self
                .food
                .iter()
                .enumerate()
                .filter(|(_, food)| {
                    sq(head_x - food.position.x) + sq(fish_y - food.position.y) <= sq(smell)
                })
                .min_by(|(_, a), (_, b)| {
                    let da = sq(head_x - a.position.x) + sq(fish_y - a.position.y);
                    let db = sq(head_x - b.position.x) + sq(fish_y - b.position.y);
                    da.partial_cmp(&db).unwrap()
                });
            if let Some((idx, _)) = nearest_idx {
                let mut rng = rand::rng();
                let center_x = self.fish[i].position.x + (fish_len - 1.0) / 2.0;
                let food_x = self.food[idx].position.x;
                let lead = self.fish[i].lead() as f32;
                let mouth_reach_facing_left = (self.width as f32 - fish_len).max(0.0) + lead + 1.0;
                let mouth_reach_facing_right = fish_len - 1.0 - lead - 1.0;
                let approach_right = if food_x > mouth_reach_facing_left {
                    true
                } else if food_x < mouth_reach_facing_right {
                    false
                } else {
                    center_x <= food_x
                };
                self.fish[i].seek_boost = rng.random_range(0.0_f32..SEEK_BOOST_INITIAL_MAX);
                self.fish[i].facing = if approach_right {
                    Direction::Right
                } else {
                    Direction::Left
                };
                self.fish[i].state = FishState::SeekingFood {
                    food_idx: idx,
                    approach_right,
                };
            }
        }
    }

    pub(super) fn tick_candyfish_effects(&mut self, dt: f32) {
        let candyfish_bounds: Vec<(f32, f32, f32, u32)> = self
            .fish
            .iter()
            .filter(|f| f.ability_stacks(FishSpecies::Candyfish) > 0)
            .map(|cf| {
                (
                    cf.position.x,
                    cf.position.x + cf.display_width as f32,
                    cf.position.y,
                    cf.ability_stacks(FishSpecies::Candyfish),
                )
            })
            .collect();
        if candyfish_bounds.is_empty() {
            return;
        }
        for food in &mut self.food {
            if food.is_candy {
                continue;
            }
            let fx = food.position.x;
            let fy = food.position.y;
            for &(cx1, cx2, cy, _) in &candyfish_bounds {
                if fx >= cx1 - 1.0 && fx <= cx2 + 1.0 && (fy - cy).abs() <= 1.0 {
                    food.is_candy = true;
                    break;
                }
            }
        }
        let scans = self
            .candy_scan
            .beats(dt, SECS_PER_MINUTE / CANDYFISH_SCANS_PER_MINUTE);
        if scans == 0 {
            return;
        }
        for i in 0..self.fish.len() {
            if self.fish[i].ability_stacks(FishSpecies::Candyfish) > 0 {
                continue;
            }
            let fx1 = self.fish[i].position.x;
            let fx2 = fx1 + self.fish[i].display_width as f32;
            let fy = self.fish[i].position.y;
            for &(cx1, cx2, cy, stacks) in &candyfish_bounds {
                if fx1 < cx2 && fx2 > cx1 && (fy - cy).abs() <= 1.0 {
                    self.fish[i].weight_g += CANDYFISH_TOUCH_WEIGHT_G * stacks * scans;
                    break;
                }
            }
        }
    }

    pub(super) fn tick_engulfment(&mut self) {
        if let Some((receiver, engulfed)) = self.find_engulf_pair_fish() {
            self.fuse_fish(receiver, engulfed);
        }
        if let Some((receiver, engulfed)) = self.find_engulf_pair_cow() {
            self.fuse_cow(receiver, engulfed);
        }
    }

    fn find_engulf_pair_fish(&self) -> Option<(usize, usize)> {
        for r in 0..self.fish.len() {
            if self.fish[r].engulf_timer <= 0.0 || self.fish[r].is_double() {
                continue;
            }
            for e in 0..self.fish.len() {
                if e == r || self.fish[e].is_double() {
                    continue;
                }
                if engulf_compatible(&self.fish[r], &self.fish[e])
                    && fish_near(&self.fish[r], &self.fish[e])
                {
                    return Some((r, e));
                }
            }
        }
        None
    }

    fn find_engulf_pair_cow(&self) -> Option<(usize, usize)> {
        for r in 0..self.cows.len() {
            if self.cows[r].engulf_timer <= 0.0 || self.cows[r].mutant.is_double {
                continue;
            }
            for e in 0..self.cows.len() {
                if e == r || self.cows[e].mutant.is_double {
                    continue;
                }
                if cow_near(&self.cows[r], &self.cows[e]) {
                    return Some((r, e));
                }
            }
        }
        None
    }

    fn fuse_fish(&mut self, receiver: usize, engulfed: usize) {
        let mut rng = rand::rng();
        let receiver_snapshot = self.fish[receiver].clone();
        let engulfed_snapshot = self.fish[engulfed].clone();
        let mut receiver_component = self.fish[receiver]
            .fused_self_component()
            .with_snapshot(receiver_snapshot);
        receiver_component.program = self.fish[receiver].fused_program(&self.fish[engulfed]);
        let mut engulfed_component = self.fish[engulfed]
            .fused_self_component()
            .with_snapshot(engulfed_snapshot);
        engulfed_component.persona = self.fish[engulfed].capture_persona();
        let engulfed_name = self.fish[engulfed].name.clone();
        let receiver_weight = self.fish[receiver].weight_g;
        let engulfed_weight = self.fish[engulfed].weight_g;

        apply_mutation(&mut self.fish[receiver], Mutation::Telophase, &mut rng);
        {
            let fish = &mut self.fish[receiver];
            fish.set_fused(vec![receiver_component, engulfed_component]);
            fish.name = format!("{} / {}", fish.name, engulfed_name);
            fish.weight_g = receiver_weight.saturating_add(engulfed_weight);
            fish.engulf_timer = 0.0;
            fish.recompute_display_width();
        }
        self.fish.remove(engulfed);
    }

    fn fuse_cow(&mut self, receiver: usize, engulfed: usize) {
        let mut rng = rand::rng();
        let receiver_component = self.cows[receiver]
            .self_component()
            .with_cow_snapshot(self.cows[receiver].clone());
        let engulfed_component = self.cows[engulfed]
            .self_component()
            .with_cow_snapshot(self.cows[engulfed].clone());
        let engulfed_name = self.cows[engulfed].name.clone();

        apply_mutation(&mut self.cows[receiver], Mutation::Telophase, &mut rng);
        {
            let cow = &mut self.cows[receiver];
            cow.mutant.fused = vec![receiver_component, engulfed_component];
            cow.name = format!("{} / {}", cow.name, engulfed_name);
            cow.engulf_timer = 0.0;
            cow.recompute_display_width();
        }
        self.cows.remove(engulfed);
    }

    pub fn spawn_unfish(&mut self, rng: &mut impl RngExt) -> bool {
        let kind = SPAWNABLE_UNFISH[rng.random_range(0..SPAWNABLE_UNFISH.len())];
        self.spawn_unfish_of(kind, rng)
    }

    pub fn spawn_unfish_of(&mut self, kind: UnfishKind, rng: &mut impl RngExt) -> bool {
        if kind.takes_a_seat() && self.is_full() {
            return false;
        }
        let actual_name = if kind == crate::fishes::unfish::UnfishKind::Doppleganger {
            names::unique_name_in(&self.used_names, "Doppleganger")
        } else {
            names::unique_roman_in(&self.used_names)
        };
        let Position { x, y } = self.spawn_point(rng);
        let fish = crate::fishes::fish::Fish::new_unfish(kind, actual_name.clone(), x, y, rng);
        self.admit(fish, actual_name);
        true
    }

    pub(super) fn tick_void_spawn(&mut self, dt: f32, rng: &mut impl RngExt) {
        self.void_spawn_timer -= dt;
        if self.void_spawn_timer <= 0.0 {
            self.void_spawn_timer = sample_exponential(rng, VOID_SPAWN_MEAN_SECS);
            self.spawn_unfish(rng);
        }
    }

    pub(super) fn tick_dopplegangers(&mut self) {
        self.tick_standalone_doppleganger();
        self.tick_fused_doppleganger();
    }

    fn tick_standalone_doppleganger(&mut self) {
        let doppleganger_idx = self.fish.iter().position(|f| {
            f.unfish_state
                .as_ref()
                .is_some_and(|us| us.kind == UnfishKind::Doppleganger && !us.doppleganger_cloned)
        });
        let Some(d_idx) = doppleganger_idx else {
            return;
        };

        let target_idx = self.fish.iter().position(|f| !f.is_unfish());
        let Some(t_idx) = target_idx else { return };

        let target_name = self.fish[t_idx].name.clone();
        let target_clone = self.fish[t_idx].clone();

        let old_unfish_state = self.fish[d_idx].unfish_state.take();
        let old_weight = self.fish[d_idx].weight_g;

        self.fish[d_idx] = target_clone;
        self.fish[d_idx].weight_g = old_weight;
        self.fish[d_idx].unfish_state = old_unfish_state;
        if let Some(ref mut us) = self.fish[d_idx].unfish_state {
            us.doppleganger_cloned = true;
        }

        let new_name = names::unique_name_in(&self.used_names, &format!("{}?", target_name));
        self.used_names.insert(new_name.clone());
        self.fish[d_idx].name = new_name;
    }

    fn tick_fused_doppleganger(&mut self) {
        let host_idx = self.fish.iter().position(|f| {
            !f.is_unfish()
                && f.fused_components().iter().any(|c| {
                    c.persona.as_deref().is_some_and(|us| {
                        us.kind == UnfishKind::Doppleganger && !us.doppleganger_cloned
                    })
                })
        });
        let Some(h_idx) = host_idx else { return };

        let target_name = self
            .fish
            .iter()
            .enumerate()
            .find(|(i, f)| *i != h_idx && !f.is_unfish())
            .map(|(_, f)| f.name.clone());
        let Some(target_name) = target_name else {
            return;
        };

        let new_name = names::unique_name_in(&self.used_names, &format!("{target_name}?"));
        self.used_names.insert(new_name.clone());

        let Some(mutant) = self.fish[h_idx].mutant.as_mut() else {
            return;
        };
        for component in &mut mutant.fused {
            let Some(persona) = component.persona.as_deref_mut() else {
                continue;
            };
            if persona.kind == UnfishKind::Doppleganger && !persona.doppleganger_cloned {
                persona.doppleganger_cloned = true;
                component.name = new_name;
                break;
            }
        }
    }

    pub(super) fn tick_blessings(&mut self, dt: f32) -> Vec<TankEvent> {
        let mut events = Vec::new();
        for fish in &mut self.fish {
            let stacks = fish.ability_stacks(FishSpecies::Holyfish);
            if stacks == 0 {
                continue;
            }
            fish.blessing_timer -= dt;
            if fish.blessing_timer <= 0.0 {
                fish.blessing_timer = BLESSING_INTERVAL_SECS;
                fish.blessing_glow = BLESSING_GLOW_SECS;
                events.extend((0..stacks).map(|_| TankEvent::Blessing));
            }
        }
        events
    }

    pub(super) fn tick_phantoms(&mut self, dt: f32, rng: &mut impl RngExt) -> Vec<TankEvent> {
        let mut events = Vec::new();
        for fish in &mut self.fish {
            let mut teleport = false;
            let mut cross_tank = false;
            for us in fish.personas_mut() {
                if us.kind != UnfishKind::Phantom {
                    continue;
                }
                us.phantom_timer -= dt;
                if us.phantom_timer <= 0.0 {
                    us.phantom_timer = sample_exponential(rng, PHANTOM_TELEPORT_MEAN);
                    if rng.random::<f32>() < PHANTOM_CROSS_TANK_CHANCE {
                        cross_tank = true;
                    } else {
                        teleport = true;
                    }
                }
            }
            if cross_tank {
                events.push(TankEvent::Wander {
                    fish_name: fish.name.clone(),
                });
            } else if teleport {
                let max_x = (self.width as f32 - fish.display_width as f32).max(0.0);
                let max_y = (self.height as f32 - 1.0).max(0.0);
                fish.position.x = rng.random_range(0.0..=max_x);
                fish.position.y = rng.random_range(0.0..=max_y);
            }
        }
        events
    }
}

#[cfg(test)]
mod engulfment_tests {
    use super::*;
    use crate::entities::cow::CowVariant;
    use crate::tank::TankKind;
    use ratatui::style::Color;

    fn tank_with_two_merluza() -> Tank {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        let mut rng = rand::rng();
        let mut a = Fish::new(FishSpecies::Merluza, "Ann".to_string(), 10.0, 5.0, &mut rng);
        let b = Fish::new(FishSpecies::Merluza, "Bob".to_string(), 12.0, 5.0, &mut rng);
        a.weight_g = 100;
        tank.used_names.insert("Ann".to_string());
        tank.used_names.insert("Bob".to_string());
        tank.fish.push(a);
        tank.fish.push(b);
        tank
    }

    #[test]
    fn engulfment_fuses_nearby_same_species() {
        let mut tank = tank_with_two_merluza();
        tank.fish[1].weight_g = 300;
        tank.fish[0].engulf_timer = 5.0;
        tank.tick_engulfment();
        assert_eq!(tank.fish.len(), 1, "the engulfed body is absorbed");
        let survivor = &tank.fish[0];
        assert!(survivor.is_double(), "the receiver becomes a telophase");
        assert_eq!(survivor.fused_components().len(), 2);
        assert_eq!(survivor.name, "Ann / Bob", "combined name");
        assert_eq!(
            survivor.weight_g, 400,
            "masses are summed, so no food is thrown away"
        );
        assert!(
            survivor.engulf_timer <= 0.0,
            "the window closes after fusing"
        );
    }

    #[test]
    fn engulfment_resplit_restores_both_identities() {
        let mut tank = tank_with_two_merluza();
        tank.fish[0].engulf_timer = 5.0;
        tank.tick_engulfment();
        assert!(tank.apply_named_mutation("Ann / Bob", "cytokinesis"));
        assert_eq!(tank.fish.len(), 2);
        let names: Vec<&str> = tank.fish.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"Ann"), "receiver keeps its identity");
        assert!(names.contains(&"Bob"), "engulfed regains its identity");
        for f in &tank.fish {
            assert!(!f.is_double(), "each half is single again");
        }
    }

    #[test]
    fn engulfment_ignores_far_entities() {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        let mut rng = rand::rng();
        let mut a = Fish::new(FishSpecies::Merluza, "Ann".to_string(), 10.0, 5.0, &mut rng);
        let far = Fish::new(FishSpecies::Salmon, "Sal".to_string(), 80.0, 5.0, &mut rng);
        a.engulf_timer = 5.0;
        tank.fish.push(a);
        tank.fish.push(far);
        tank.tick_engulfment();
        assert_eq!(tank.fish.len(), 2, "a far entity is out of reach");
    }

    #[test]
    fn engulfment_fuses_across_species_and_stacks_both_abilities() {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        let mut rng = rand::rng();
        let mut gold = Fish::new(FishSpecies::Cashfish, "Au".to_string(), 10.0, 5.0, &mut rng);
        let mutant = Fish::new(
            FishSpecies::Mutantfish,
            "Goo".to_string(),
            12.0,
            5.0,
            &mut rng,
        );
        gold.engulf_timer = 5.0;
        tank.fish.push(gold);
        tank.fish.push(mutant);
        tank.tick_engulfment();
        assert_eq!(tank.fish.len(), 1, "different species fuse");
        let s = &tank.fish[0];
        assert_eq!(
            s.ability_stacks(FishSpecies::Cashfish),
            1,
            "keeps the money zoomies"
        );
        assert_eq!(s.auto_mutate_stacks(), 1, "gains the auto-mutation");
    }

    #[test]
    fn a_fused_candyfish_candies_the_food_it_bites() {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        let mut rng = rand::rng();
        let mut host = Fish::new(FishSpecies::Merluza, "Ann".to_string(), 10.0, 5.0, &mut rng);
        host.engulf_timer = 5.0;
        let candy = Fish::new(
            FishSpecies::Candyfish,
            "Cara".to_string(),
            12.0,
            5.0,
            &mut rng,
        );
        tank.fish.push(host);
        tank.fish.push(candy);
        tank.tick_engulfment();
        assert_eq!(tank.fish.len(), 1, "the candyfish is engulfed");
        assert_ne!(tank.fish[0].species, FishSpecies::Candyfish);
        assert_eq!(tank.fish[0].ability_stacks(FishSpecies::Candyfish), 1);
        let mut food = crate::entities::food::Food::new(tank.fish[0].head_x() as f32);
        food.position.y = tank.fish[0].position.y;
        tank.food.push(food);
        tank.fish[0].state = FishState::SeekingFood {
            food_idx: 0,
            approach_right: true,
        };
        tank.check_eating_collisions();
        assert!(
            tank.food[0].is_candy,
            "the fused candyfish candies the food"
        );
        assert!(!tank.food[0].eaten, "candied food is left for the others");
    }

    #[test]
    fn endocytosis_after_cross_fusion_keeps_both_abilities_on_one_body() {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        let mut rng = rand::rng();
        let mut gold = Fish::new(FishSpecies::Cashfish, "Au".to_string(), 10.0, 5.0, &mut rng);
        let mutant = Fish::new(
            FishSpecies::Mutantfish,
            "Goo".to_string(),
            12.0,
            5.0,
            &mut rng,
        );
        gold.engulf_timer = 5.0;
        tank.fish.push(gold);
        tank.fish.push(mutant);
        tank.tick_engulfment();
        assert!(tank.apply_named_mutation("Au / Goo", "endocytosis"));
        assert_eq!(tank.fish.len(), 1, "one body remains");
        let s = &tank.fish[0];
        assert!(!s.is_double(), "collapsed to a single body");
        assert_eq!(s.ability_stacks(FishSpecies::Cashfish), 1);
        assert_eq!(s.auto_mutate_stacks(), 1, "both abilities live on one fish");
    }

    #[test]
    fn cross_species_resplit_restores_each_species() {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        let mut rng = rand::rng();
        let mut gold = Fish::new(FishSpecies::Cashfish, "Au".to_string(), 10.0, 5.0, &mut rng);
        let mutant = Fish::new(
            FishSpecies::Mutantfish,
            "Goo".to_string(),
            12.0,
            5.0,
            &mut rng,
        );
        gold.engulf_timer = 5.0;
        tank.fish.push(gold);
        tank.fish.push(mutant);
        tank.tick_engulfment();
        assert!(tank.apply_named_mutation("Au / Goo", "cytokinesis"));
        assert_eq!(tank.fish.len(), 2);
        let species: Vec<FishSpecies> = tank.fish.iter().map(|f| f.species).collect();
        assert!(species.contains(&FishSpecies::Cashfish));
        assert!(species.contains(&FishSpecies::Mutantfish));
    }

    fn fuse_host_with_botfish() -> Tank {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        let mut rng = rand::rng();
        let mut host = Fish::new(FishSpecies::Cashfish, "Au".to_string(), 10.0, 5.0, &mut rng);
        host.weight_g = 1000;
        host.engulf_timer = 5.0;
        let mut bot = Fish::new(FishSpecies::Botfish, "Neo".to_string(), 12.0, 5.0, &mut rng);
        bot.weight_g = 1;
        bot.botfish_state
            .as_mut()
            .unwrap()
            .program("wake".to_string(), vec!["/feed 1".to_string()]);
        tank.fish.push(host);
        tank.fish.push(bot);
        tank.tick_engulfment();
        assert_eq!(tank.fish.len(), 1, "the botfish is engulfed");
        tank
    }

    fn assert_carries_script(fish: &Fish) {
        assert!(fish.is_programmable(), "programmability rides the fusion");
        let script = fish.script().expect("the merged fish carries a script");
        assert!(script.responds_to("wake"), "the trigger survives");
        assert_eq!(
            script.script,
            vec!["/feed 1".to_string()],
            "the lines survive"
        );
    }

    #[test]
    fn an_engulfed_botfish_makes_its_host_programmable() {
        let tank = fuse_host_with_botfish();
        assert_carries_script(&tank.fish[0]);
    }

    #[test]
    fn endocytosis_with_a_botfish_keeps_the_script_on_one_body() {
        let mut tank = fuse_host_with_botfish();
        assert!(tank.apply_named_mutation("Au / Neo", "endocytosis"));
        assert_eq!(tank.fish.len(), 1, "one body remains");
        let survivor = &tank.fish[0];
        assert!(!survivor.is_double(), "collapsed to a single body");
        assert_eq!(
            survivor.species,
            FishSpecies::Cashfish,
            "the heavier host body wins"
        );
        assert_carries_script(survivor);
    }

    fn engulf_two(a_body: usize, b_body: usize) -> Tank {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        let mut rng = rand::rng();
        let mut a = Fish::new(FishSpecies::Merluza, "Ann".to_string(), 10.0, 5.0, &mut rng);
        a.body_size = a_body;
        a.recompute_display_width();
        let mut b = Fish::new(FishSpecies::Salmon, "Bob".to_string(), 12.0, 5.0, &mut rng);
        b.body_size = b_body;
        b.recompute_display_width();
        a.engulf_timer = 5.0;
        tank.fish.push(a);
        tank.fish.push(b);
        tank.tick_engulfment();
        assert_eq!(tank.fish.len(), 1, "the two fish fuse into one body");
        tank
    }

    #[test]
    fn fused_body_renders_both_halves_at_their_own_widths() {
        let tank = engulf_two(7, 3);
        let fused = &tank.fish[0];
        let sprite = fused.line_sprite();
        assert_eq!(
            sprite.rows[sprite.body_row].len(),
            fused.display_width,
            "the drawn fused body is exactly display_width wide"
        );
        let solo = Fish::new(
            FishSpecies::Merluza,
            "x".to_string(),
            0.0,
            0.0,
            &mut rand::rng(),
        );
        assert!(
            fused.display_width > solo.display_width,
            "a two-fish merge is wider than a single fish"
        );
    }

    #[test]
    fn cytokinesis_restores_each_halfs_visual_state() {
        let mut tank = engulf_two(7, 3);
        assert!(tank.apply_named_mutation("Ann / Bob", "cytokinesis"));
        assert_eq!(tank.fish.len(), 2);
        let ann = tank.fish.iter().find(|f| f.name == "Ann").unwrap();
        let bob = tank.fish.iter().find(|f| f.name == "Bob").unwrap();
        assert_eq!(ann.body_size, 7, "the receiver keeps its own body");
        assert_eq!(bob.body_size, 3, "the engulfed fish keeps its own body");
    }

    #[test]
    fn a_mutation_while_merged_lands_on_both_halves() {
        let mut tank = engulf_two(6, 6);
        assert!(tank.apply_named_mutation("Ann / Bob", "feet"));
        let comps = tank.fish[0].fused_components();
        assert!(
            comps[0].fish_snapshot().unwrap().feet().is_some(),
            "the receiver half grows feet"
        );
        assert!(
            comps[1].fish_snapshot().unwrap().feet().is_some(),
            "the engulfed half grows feet too"
        );
    }

    #[test]
    fn endocytosis_collapses_to_the_heavier_halfs_body() {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        let mut rng = rand::rng();
        let mut a = Fish::new(FishSpecies::Merluza, "Ann".to_string(), 10.0, 5.0, &mut rng);
        a.weight_g = 10;
        let mut b = Fish::new(FishSpecies::Salmon, "Bob".to_string(), 12.0, 5.0, &mut rng);
        b.weight_g = 40;
        a.engulf_timer = 5.0;
        tank.fish.push(a);
        tank.fish.push(b);
        tank.tick_engulfment();
        assert!(tank.apply_named_mutation("Ann / Bob", "endocytosis"));
        assert_eq!(tank.fish.len(), 1);
        let merged = &tank.fish[0];
        assert!(!merged.is_double(), "endocytosis collapses to one body");
        assert_eq!(
            merged.species,
            FishSpecies::Salmon,
            "the heavier half's body wins the conflict"
        );
        assert_eq!(
            merged.fused_components().len(),
            2,
            "both abilities still stack on the single body"
        );
    }

    #[test]
    fn cows_engulf_across_variants_and_stack_milk() {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        let mut rng = rand::rng();
        let a_name = tank.spawn_cow(CowVariant::Pink, &mut rng);
        let b_name = tank.spawn_cow(CowVariant::LightYellow, &mut rng);
        tank.cows[0].position.x = 5.0;
        tank.cows[0].position.y = 5.0;
        tank.cows[1].position.x = 6.0;
        tank.cows[1].position.y = 5.0;
        tank.cows[0].engulf_timer = 5.0;
        tank.tick_engulfment();
        assert_eq!(tank.cows.len(), 1, "one cow absorbs the other");
        let survivor = &tank.cows[0];
        assert!(survivor.mutant.is_double);
        assert_eq!(survivor.name, format!("{a_name} / {b_name}"));
        let milks = survivor.milk_components();
        assert!(milks.contains(&CowVariant::Pink), "keeps strawberry milk");
        assert!(
            milks.contains(&CowVariant::LightYellow),
            "gains vanilla milk"
        );
    }

    #[test]
    fn standard_fish_engulfs_a_slime_unfish() {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        let mut rng = rand::rng();
        let mut salmon = Fish::new(FishSpecies::Salmon, "Sal".to_string(), 10.0, 5.0, &mut rng);
        let mut phantom =
            Fish::new_unfish(UnfishKind::Phantom, "Ph".to_string(), 12.0, 5.0, &mut rng);
        phantom.position.y = 5.0;
        salmon.engulf_timer = 5.0;
        tank.fish.push(salmon);
        tank.fish.push(phantom);
        tank.tick_engulfment();
        assert_eq!(tank.fish.len(), 1, "a salmon can engulf a phantom");
        assert_eq!(tank.fish[0].name, "Sal / Ph");
        assert!(tank.apply_named_mutation("Sal / Ph", "cytokinesis"));
        assert!(
            tank.fish.iter().any(|f| f
                .unfish_state
                .as_ref()
                .is_some_and(|us| us.kind == UnfishKind::Phantom)),
            "the phantom is restored as a real phantom on re-split"
        );
    }

    fn salmon_engulfing(kind: UnfishKind, engulfed_name: &str) -> Tank {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        let mut rng = rand::rng();
        let mut salmon = Fish::new(FishSpecies::Salmon, "Sal".to_string(), 10.0, 5.0, &mut rng);
        let mut unfish = Fish::new_unfish(kind, engulfed_name.to_string(), 12.0, 5.0, &mut rng);
        unfish.position.y = 5.0;
        salmon.engulf_timer = 5.0;
        tank.used_names.insert("Sal".to_string());
        tank.used_names.insert(engulfed_name.to_string());
        tank.fish.push(salmon);
        tank.fish.push(unfish);
        tank.tick_engulfment();
        tank
    }

    #[test]
    fn engulfed_phantom_teleports_its_host() {
        let mut tank = salmon_engulfing(UnfishKind::Phantom, "Ph");
        assert_eq!(tank.fish.len(), 1);
        for persona in tank.fish[0].personas_mut() {
            persona.phantom_timer = 0.0;
        }
        let mut rng = rand::rng();
        tank.tick_phantoms(1.0 / 60.0, &mut rng);
        let fired = tank.fish[0]
            .personas_mut()
            .iter()
            .any(|us| us.phantom_timer > 0.0);
        assert!(fired, "the fused phantom rearms after teleporting its host");
    }

    #[test]
    fn engulfed_blinker_makes_its_host_invisible() {
        let mut tank = salmon_engulfing(UnfishKind::Blinker, "Bl");
        assert!(!tank.fish[0].is_invisible(), "starts visible");
        for persona in tank.fish[0].personas_mut() {
            persona.blinker_phase = crate::fishes::unfish::BlinkerPhase::Invisible;
        }
        assert!(
            tank.fish[0].is_invisible(),
            "the host vanishes while its blinker passenger is invisible"
        );
    }

    #[test]
    fn engulfed_doppleganger_impersonates_on_resplit() {
        let mut tank = salmon_engulfing(UnfishKind::Doppleganger, "Dop");
        let mut rng = rand::rng();
        let victim = Fish::new(FishSpecies::Merluza, "Vic".to_string(), 40.0, 5.0, &mut rng);
        tank.used_names.insert("Vic".to_string());
        tank.fish.push(victim);
        tank.tick_dopplegangers();
        assert!(tank.apply_named_mutation("Sal / Dop", "cytokinesis"));
        assert!(
            tank.fish.iter().any(|f| f.name == "Vic?"),
            "the fused doppleganger emerges impersonating its victim"
        );
    }

    #[test]
    fn worms_engulf_and_resplit() {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        let mut rng = rand::rng();
        let mut a = Fish::new_unfish(UnfishKind::Worm, "Wa".to_string(), 10.0, 5.0, &mut rng);
        let mut b = Fish::new_unfish(UnfishKind::Worm, "Wb".to_string(), 11.0, 5.0, &mut rng);
        a.position.y = 5.0;
        b.position.y = 5.0;
        a.weight_g = 100;
        b.weight_g = 300;
        a.engulf_timer = 5.0;
        tank.used_names.insert("Wa".to_string());
        tank.used_names.insert("Wb".to_string());
        tank.fish.push(a);
        tank.fish.push(b);
        tank.tick_engulfment();
        assert_eq!(tank.fish.len(), 1, "one worm absorbs the other");
        assert!(
            tank.fish[0].is_double(),
            "the receiver becomes a double worm"
        );
        assert_eq!(tank.fish[0].name, "Wa / Wb");
        assert_eq!(tank.fish[0].weight_g, 400, "weights are summed");
        assert!(tank.apply_named_mutation("Wa / Wb", "cytokinesis"));
        assert_eq!(tank.fish.len(), 2);
        let names: Vec<&str> = tank.fish.iter().map(|f| f.name.as_str()).collect();
        assert!(
            names.contains(&"Wa") && names.contains(&"Wb"),
            "both worms return"
        );
    }

    #[test]
    fn worms_do_not_engulf_standard_fish() {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        let mut rng = rand::rng();
        let mut worm = Fish::new_unfish(UnfishKind::Worm, "Wa".to_string(), 10.0, 5.0, &mut rng);
        let mut salmon = Fish::new(FishSpecies::Salmon, "Sal".to_string(), 11.0, 5.0, &mut rng);
        worm.position.y = 5.0;
        salmon.position.y = 5.0;
        worm.engulf_timer = 5.0;
        tank.fish.push(worm);
        tank.fish.push(salmon);
        tank.tick_engulfment();
        assert_eq!(tank.fish.len(), 2, "a worm only fuses with another worm");
    }

    fn engulf_two_worms(a_color: Color, b_color: Color) -> Tank {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        let mut rng = rand::rng();
        let mut a = Fish::new_unfish(UnfishKind::Worm, "Wa".to_string(), 10.0, 5.0, &mut rng);
        let mut b = Fish::new_unfish(UnfishKind::Worm, "Wb".to_string(), 11.0, 5.0, &mut rng);
        a.position.y = 5.0;
        b.position.y = 5.0;
        a.unfish_state.as_mut().unwrap().slime_body_color = Some(a_color);
        b.unfish_state.as_mut().unwrap().slime_body_color = Some(b_color);
        a.engulf_timer = 5.0;
        tank.used_names.insert("Wa".to_string());
        tank.used_names.insert("Wb".to_string());
        tank.fish.push(a);
        tank.fish.push(b);
        tank.tick_engulfment();
        assert_eq!(tank.fish.len(), 1, "the two worms fuse into one body");
        tank
    }

    #[test]
    fn fused_worm_renders_each_half_in_its_own_color() {
        let tank = engulf_two_worms(Color::Red, Color::Blue);
        let cells = tank.fish[0]
            .fused_worm_cells()
            .expect("a fused worm renders per-half, not as a uniform telophase");
        let colors: std::collections::HashSet<Color> = cells.iter().map(|c| c.1).collect();
        assert!(
            colors.contains(&Color::Red),
            "the left half keeps its color"
        );
        assert!(
            colors.contains(&Color::Blue),
            "the right half keeps its color"
        );
    }

    #[test]
    fn worm_carries_mutations_through_fusion_and_resplit() {
        let mut tank = engulf_two_worms(Color::Red, Color::Blue);
        assert!(tank.apply_named_mutation("Wa / Wb", "ear"));
        assert!(tank.apply_named_mutation("Wa / Wb", "cytokinesis"));
        assert_eq!(tank.fish.len(), 2);
        for f in &tank.fish {
            assert!(
                f.unfish_state.as_ref().unwrap().ear_count > 0,
                "each separated worm keeps the ears it grew while fused"
            );
        }
    }

    fn engulf_two_cows(a: CowVariant, b: CowVariant) -> (Tank, String) {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        let mut rng = rand::rng();
        let a_name = tank.spawn_cow(a, &mut rng);
        let b_name = tank.spawn_cow(b, &mut rng);
        tank.cows[0].position.x = 5.0;
        tank.cows[0].position.y = 5.0;
        tank.cows[1].position.x = 6.0;
        tank.cows[1].position.y = 5.0;
        tank.cows[0].engulf_timer = 5.0;
        tank.tick_engulfment();
        assert_eq!(tank.cows.len(), 1, "one cow absorbs the other");
        (tank, format!("{a_name} / {b_name}"))
    }

    #[test]
    fn fused_cow_renders_each_half_in_its_own_color() {
        let (tank, _) = engulf_two_cows(CowVariant::Pink, CowVariant::LightYellow);
        let rows = crate::entities::cow::cow_sprite(&tank.cows[0]);
        let colors: std::collections::HashSet<Color> = rows.iter().flatten().map(|c| c.1).collect();
        assert!(
            colors.contains(&CowVariant::Pink.body_color()),
            "the left half keeps strawberry color"
        );
        assert!(
            colors.contains(&CowVariant::LightYellow.body_color()),
            "the right half keeps vanilla color"
        );
    }

    #[test]
    fn cow_carries_mutations_through_fusion_and_resplit() {
        let (mut tank, fused_name) = engulf_two_cows(CowVariant::Pink, CowVariant::LightYellow);
        let mut rng = rand::rng();
        let baseline =
            crate::entities::cow::Cow::new("Ref".to_string(), CowVariant::Pink, 0.0, 0.0, &mut rng)
                .eye_count();
        assert!(tank.apply_named_mutation(&fused_name, "eyeincrease"));
        assert!(tank.apply_named_mutation(&fused_name, "cytokinesis"));
        assert_eq!(tank.cows.len(), 2);
        for cow in &tank.cows {
            assert!(
                cow.eye_count() > baseline,
                "each separated cow keeps the eye it grew while fused"
            );
        }
    }

    #[test]
    fn endocytosis_sends_the_lost_identity_to_the_graveyard() {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        let mut rng = rand::rng();
        let mut a = Fish::new(FishSpecies::Merluza, "Ann".to_string(), 10.0, 5.0, &mut rng);
        a.weight_g = 10;
        let mut b = Fish::new(FishSpecies::Salmon, "Bob".to_string(), 12.0, 5.0, &mut rng);
        b.weight_g = 40;
        a.engulf_timer = 5.0;
        tank.fish.push(a);
        tank.fish.push(b);
        tank.tick_engulfment();
        assert!(tank.apply_named_mutation("Ann / Bob", "endocytosis"));
        assert_eq!(tank.fish.len(), 1, "the merged body remains");
        assert_eq!(
            tank.pending_graveyard.len(),
            1,
            "the absorbed identity becomes revivable"
        );
        assert_eq!(
            tank.pending_graveyard[0].name, "Ann",
            "the lighter half is the lost identity"
        );
        assert!(!tank.pending_graveyard[0].is_double());
    }

    #[test]
    fn a_holyfish_emits_a_blessing_when_its_timer_elapses() {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        let mut rng = rand::rng();
        let mut holy = Fish::new(
            FishSpecies::Holyfish,
            "Saint".to_string(),
            10.0,
            5.0,
            &mut rng,
        );
        holy.blessing_timer = 0.0001;
        tank.fish.push(holy);
        let events = tank.tick_blessings(1.0);
        assert!(events.iter().any(|e| matches!(e, TankEvent::Blessing)));
        assert!(
            tank.fish[0].blessing_glow > 0.0,
            "the holyfish glistens while it blesses"
        );
        assert!(
            tank.fish[0].blessing_timer > 1.0,
            "the timer rearms for the next blessing"
        );
    }

    #[test]
    fn a_plain_fish_never_blesses() {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        let mut rng = rand::rng();
        let mut merluza = Fish::new(FishSpecies::Merluza, "Mer".to_string(), 10.0, 5.0, &mut rng);
        merluza.blessing_timer = 0.0001;
        tank.fish.push(merluza);
        assert!(tank.tick_blessings(1.0).is_empty());
    }
}

#[cfg(test)]
mod wiring_tests {
    use super::*;
    use crate::entities::food::Food;
    use crate::fishes::botfish::BotfishState;
    use crate::fishes::parts::{Part, PinOwner};
    use crate::tank::TankKind;
    use crate::tank::{ChannelRegistry, WorldSignal, WorldView};

    const TANK_W: u16 = 60;
    const TANK_H: u16 = 20;
    const BOT_X: f32 = 10.0;
    const BOT_Y: f32 = 5.0;
    const FOOD_X: f32 = 12.0;

    fn tank_with_botfish(wired: bool) -> Tank {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        tank.width = TANK_W;
        tank.height = TANK_H;
        let mut bot = Fish::new(
            FishSpecies::Botfish,
            "Neo".to_string(),
            BOT_X,
            BOT_Y,
            &mut rand::rng(),
        );
        if wired {
            bot.botfish_state
                .as_mut()
                .unwrap()
                .program("wake".to_string(), vec!["/feed 1".to_string()]);
        }
        assert_eq!(bot.is_wired(), wired);
        tank.fish.push(bot);
        tank
    }

    fn seek_nearby_food(tank: &mut Tank) {
        tank.food.push(Food::new(FOOD_X));
        tank.food[0].position.y = BOT_Y;
        tank.assign_food_to_idle_fish();
    }

    const RAD_WATCH_TICKS: usize = 600;
    const RAD_TICK_SECS: f32 = 1.0;

    fn mutations_in_a_radtank(wired: bool) -> u32 {
        let mut tank = tank_with_botfish(wired);
        tank.kind = TankKind::Rad;
        for _ in 0..RAD_WATCH_TICKS {
            tank.tick_mutations(RAD_TICK_SECS);
        }
        tank.fish
            .iter()
            .filter_map(|f| f.mutations.as_ref())
            .map(|record| record.count)
            .sum()
    }

    #[test]
    fn radiation_never_mutates_a_wired_botfish() {
        assert_eq!(
            mutations_in_a_radtank(true),
            0,
            "a circuit must survive the Radtank it farms"
        );
    }

    #[test]
    fn radiation_still_mutates_a_botfish_nobody_wired() {
        assert!(
            mutations_in_a_radtank(false) > 0,
            "ten minutes at a thirty-second mean is never quiet for an unwired fish"
        );
    }

    #[test]
    fn a_wired_botfish_never_seeks_food() {
        let mut tank = tank_with_botfish(true);
        seek_nearby_food(&mut tank);
        assert!(
            matches!(tank.fish[0].state, FishState::Idle),
            "a wired fish ignores food"
        );
    }

    #[test]
    fn an_unwired_botfish_seeks_food_normally() {
        let mut tank = tank_with_botfish(false);
        seek_nearby_food(&mut tank);
        assert!(
            matches!(tank.fish[0].state, FishState::SeekingFood { .. }),
            "an unwired botfish still eats"
        );
    }

    #[test]
    fn every_tank_is_its_own_board() {
        let mut a = Tank::new("A".to_string(), TankKind::Base, &[]);
        let mut b = Tank::new("B".to_string(), TankKind::Base, &[]);

        a.channels.set_level("clk", true);
        b.channels.register("clk");

        assert!(a.channels.level("clk"), "the driven board is high");
        assert!(
            !b.channels.level("clk"),
            "the same channel name in another tank is a different wire"
        );
        assert!(
            !b.channels.contains("harvest"),
            "a channel never leaks between boards"
        );
    }

    #[test]
    fn a_fresh_tank_has_no_wiring() {
        let tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        assert!(tank.channels.is_empty());
    }

    const DRIFT_TICKS: usize = 30;

    fn plain_fish_tank() -> Tank {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        tank.width = TANK_W;
        tank.height = TANK_H;
        let mut fish = Fish::new(
            FishSpecies::Merluza,
            "Ann".to_string(),
            BOT_X,
            BOT_Y,
            &mut rand::rng(),
        );
        fish.randomize_direction();
        tank.fish.push(fish);
        tank
    }

    fn drift(tank: &mut Tank) -> (f32, f32) {
        let settings = Settings::default();
        let (start_x, start_y) = (tank.fish[0].position.x, tank.fish[0].position.y);
        for _ in 0..DRIFT_TICKS {
            tank.fish[0].tick(&settings, TANK_W, TANK_H, 0);
        }
        (
            (tank.fish[0].position.x - start_x).abs(),
            (tank.fish[0].position.y - start_y).abs(),
        )
    }

    #[test]
    fn a_frozen_fish_never_moves() {
        let mut tank = plain_fish_tank();
        tank.fish[0].frozen = true;
        assert_eq!(drift(&mut tank), (0.0, 0.0), "a frozen fish is pinned");
    }

    #[test]
    fn an_unfrozen_fish_drifts() {
        let mut tank = plain_fish_tank();
        let (dx, dy) = drift(&mut tank);
        assert!(dx + dy > 0.0, "an unfrozen fish keeps swimming");
    }

    #[test]
    fn unfreezing_returns_the_fish_to_motion() {
        let mut tank = plain_fish_tank();
        tank.fish[0].frozen = true;
        assert_eq!(drift(&mut tank), (0.0, 0.0));
        tank.fish[0].frozen = false;
        let (dx, dy) = drift(&mut tank);
        assert!(dx + dy > 0.0, "unfreezing restores movement");
    }

    #[test]
    fn a_frozen_fish_never_seeks_food() {
        let mut tank = plain_fish_tank();
        tank.fish[0].frozen = true;
        seek_nearby_food(&mut tank);
        assert!(
            matches!(tank.fish[0].state, FishState::Idle),
            "a pinned fish is never sent after food it cannot reach"
        );
    }

    const ZOOMIE_WINDOW_TICKS: usize = 200;
    const ZOOMIE_WATCH_FPS: f32 = 1.0;
    const HOST_WEIGHT_G: u32 = 1000;
    const BOT_WEIGHT_G: u32 = 1;

    fn host_carrying_a_bot(wired: bool) -> Tank {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        tank.width = TANK_W;
        tank.height = TANK_H;
        let mut rng = rand::rng();
        let mut host = Fish::new(
            FishSpecies::Merluza,
            "Ann".to_string(),
            BOT_X,
            BOT_Y,
            &mut rng,
        );
        host.weight_g = HOST_WEIGHT_G;
        host.engulf_timer = 5.0;
        let mut bot = Fish::new(
            FishSpecies::Botfish,
            "Neo".to_string(),
            FOOD_X,
            BOT_Y,
            &mut rng,
        );
        bot.weight_g = BOT_WEIGHT_G;
        if wired {
            bot.botfish_state
                .as_mut()
                .unwrap()
                .program("wake".to_string(), vec!["/feed 1".to_string()]);
        }
        tank.fish.push(host);
        tank.fish.push(bot);
        tank.tick_engulfment();
        assert_eq!(tank.fish.len(), 1, "the botfish is engulfed");
        assert!(tank.fish[0].is_programmable());
        assert_eq!(tank.fish[0].is_wired(), wired);
        tank
    }

    fn zoomies_within_window(tank: &mut Tank) -> bool {
        let settings = Settings {
            fps: ZOOMIE_WATCH_FPS,
            ..Settings::default()
        };
        for _ in 0..ZOOMIE_WINDOW_TICKS {
            tank.fish[0].tick(&settings, TANK_W, TANK_H, 0);
            if matches!(tank.fish[0].state, FishState::Zoomie { .. }) {
                return true;
            }
        }
        false
    }

    #[test]
    fn a_wired_host_never_zoomies() {
        let mut tank = host_carrying_a_bot(true);
        assert!(!zoomies_within_window(&mut tank), "a wired fish stays calm");
    }

    #[test]
    fn a_host_carrying_an_unwired_bot_still_zoomies() {
        let mut tank = host_carrying_a_bot(false);
        assert!(zoomies_within_window(&mut tank), "an unwired fish plays");
    }

    #[test]
    fn an_unwired_botfish_zoomies_so_that_wiring_one_is_visible() {
        let mut tank = tank_with_botfish(false);
        assert!(
            FishSpecies::Botfish.config().zoomie.zooms(),
            "a botfish must be able to zoomie, or wiring it changes nothing you can watch"
        );
        assert!(zoomies_within_window(&mut tank), "a loose botfish plays");
    }

    #[test]
    fn a_wired_botfish_never_zoomies() {
        let mut tank = tank_with_botfish(true);
        assert!(
            !zoomies_within_window(&mut tank),
            "wiring a botfish calms it — this is the Phase 1 demo"
        );
    }

    const ENGULF_SECS: f32 = 5.0;
    const HEAVY_SENSE_WEIGHT_G: u32 = 5000;
    const CARRIER: &str = "Ann / Cm";
    const FUSED: &str = "Ann / Cm / Ear";
    const RIPE: &str = "ripe";
    const NOON: u32 = 12;

    fn botfish(name: &str, weight_g: u32, wire: fn(&mut BotfishState)) -> Fish {
        let mut bot = Fish::new(
            FishSpecies::Botfish,
            name.to_string(),
            FOOD_X,
            BOT_Y,
            &mut rand::rng(),
        );
        bot.weight_g = weight_g;
        wire(bot.botfish_state.as_mut().unwrap());
        bot
    }

    fn module_wiring(bot: &mut BotfishState) {
        bot.program(String::new(), vec!["/feed 1".to_string()]);
        bot.install(Part::CommandModule);
        bot.wire(Part::CommandModule, "fire", RIPE);
        bot.install(Part::InverterCoil);
        bot.install(Part::DelaySpool);
        bot.listen("a");
        bot.drive("q");
    }

    fn sense_wiring(bot: &mut BotfishState) {
        bot.install(Part::StartleNerve);
        bot.wire(Part::StartleNerve, "birth", RIPE);
        bot.install(Part::InverterCoil);
        bot.install(Part::DelaySpool);
        bot.listen("b");
        bot.drive("nq");
    }

    fn engulfing_host() -> Fish {
        let mut host = Fish::new(
            FishSpecies::Merluza,
            "Ann".to_string(),
            BOT_X,
            BOT_Y,
            &mut rand::rng(),
        );
        host.weight_g = HOST_WEIGHT_G;
        host.engulf_timer = ENGULF_SECS;
        host
    }

    fn carrier_of_a_module() -> Tank {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        tank.width = TANK_W;
        tank.height = TANK_H;
        tank.admit(engulfing_host(), "Ann".to_string());
        tank.admit(botfish("Cm", BOT_WEIGHT_G, module_wiring), "Cm".to_string());
        tank.tick_engulfment();
        assert!(tank.apply_named_mutation(CARRIER, "endocytosis"));
        assert!(!tank.fish[0].is_double(), "the carrier is one body again");
        tank
    }

    fn carrier_engulfs_a_sense(sense_weight_g: u32) -> Tank {
        let mut tank = carrier_of_a_module();
        tank.fish[0].engulf_timer = ENGULF_SECS;
        tank.admit(
            botfish("Ear", sense_weight_g, sense_wiring),
            "Ear".to_string(),
        );
        tank.tick_engulfment();
        assert_eq!(tank.fish.len(), 1, "the sense fish is engulfed");
        assert_eq!(tank.fish[0].name, FUSED);
        tank
    }

    fn named<'a>(tank: &'a Tank, name: &str) -> &'a BotfishState {
        tank.fish
            .iter()
            .find(|f| f.name == name)
            .and_then(Fish::script)
            .unwrap_or_else(|| panic!("{name} carries a circuit"))
    }

    fn assert_module_fish(bot: &BotfishState) {
        assert_eq!(
            bot.parts().iter().collect::<Vec<_>>(),
            vec![
                (Part::InverterCoil, 1),
                (Part::DelaySpool, 1),
                (Part::CommandModule, 1)
            ]
        );
        assert!(bot.hears("a") && bot.drives() == Some("q"));
    }

    fn assert_fused_circuit(bot: &BotfishState) {
        assert_eq!(
            bot.parts().iter().collect::<Vec<_>>(),
            vec![
                (Part::InverterCoil, 1),
                (Part::DelaySpool, 2),
                (Part::StartleNerve, 1),
                (Part::CommandModule, 1)
            ],
            "the parts are a union: spools add up, a second coil does nothing"
        );
        assert!(bot.hears("a") && !bot.hears("b"), "the receiver listens");
        assert_eq!(bot.drives(), Some("q"), "the receiver drives");
        assert_eq!(bot.script, vec!["/feed 1".to_string()]);
        assert_eq!(bot.pins().channel(Part::StartleNerve, "birth"), Some(RIPE));
    }

    #[test]
    fn a_host_carrying_a_circuit_keeps_it_when_it_engulfs_another_botfish() {
        let tank = carrier_engulfs_a_sense(BOT_WEIGHT_G);
        let fused = &tank.fish[0];
        assert!(fused.is_double());
        assert!(fused.is_programmable());
        assert_fused_circuit(named(&tank, FUSED));
    }

    #[test]
    fn a_sense_fused_into_a_command_module_fish_senses_and_acts() {
        let mut tank = carrier_engulfs_a_sense(BOT_WEIGHT_G);
        tank.signal(WorldSignal::Birth);
        let mut world = tank.observe(0, NOON);
        tank.advance_stage(&mut world);
        assert!(tank.channels.level(RIPE), "the fused sense drove the wire");
        let mut world = tank.observe(0, NOON);
        tank.advance_stage(&mut world);
        assert!(
            named(&tank, FUSED).is_processing(),
            "and the fused module ran the program"
        );
    }

    const PANEL_FISH: &str = "Lcd";
    const PANEL_FUSED: &str = "Ann / Cm / Lcd";
    const LETTER_A_LINES: [&str; 2] = ["k0", "k6"];

    fn lit_panel_wiring(bot: &mut BotfishState) {
        bot.install(Part::GlyphPanel);
        bot.wire(Part::GlyphPanel, "char", "k");
        bot.wire(Part::GlyphPanel, "write", "w");
        let mut channels = ChannelRegistry::new();
        for line in LETTER_A_LINES.into_iter().chain(["w"]) {
            channels.set_level(line, true);
        }
        bot.react(&channels);
    }

    fn first_panel_row(bot: &BotfishState) -> Option<String> {
        let displays = bot.displays();
        let rows = displays.first()?.rows();
        Some(rows[0].trim_end().to_string())
    }

    #[test]
    fn a_glyph_panel_keeps_what_it_shows_through_a_fusion() {
        let mut tank = carrier_of_a_module();
        let lit = botfish(PANEL_FISH, BOT_WEIGHT_G, lit_panel_wiring);
        assert_eq!(lit.script().and_then(first_panel_row).as_deref(), Some("A"));
        tank.fish[0].engulf_timer = ENGULF_SECS;
        tank.admit(lit, PANEL_FISH.to_string());
        tank.tick_engulfment();

        assert_eq!(tank.fish.len(), 1, "the panel fish is engulfed");
        assert_eq!(
            first_panel_row(named(&tank, PANEL_FUSED)).as_deref(),
            Some("A"),
            "the bubble survives a fusion"
        );
    }

    const MEMORY_FISH: &str = "Ram";
    const MEMORY_FUSED: &str = "Ann / Cm / Ram";

    fn stored_letter_a(bot: &mut BotfishState) {
        bot.install(Part::CoreStack);
        bot.wire(Part::CoreStack, "data_in", "k");
        bot.wire(Part::CoreStack, "write", "w");
        let mut channels = ChannelRegistry::new();
        for line in LETTER_A_LINES.into_iter().chain(["w"]) {
            channels.set_level(line, true);
        }
        bot.react(&channels);
    }

    fn word_zero(bot: &BotfishState) -> Option<u32> {
        let mut word = None;
        bot.parts().report(
            |_, _| 0,
            &WorldView::default(),
            |_, readings| word = readings.get("data_out"),
        );
        word
    }

    #[test]
    fn a_core_stack_keeps_its_words_through_a_fusion() {
        let mut tank = carrier_of_a_module();
        let memory = botfish(MEMORY_FISH, BOT_WEIGHT_G, stored_letter_a);
        assert_eq!(memory.script().and_then(word_zero), Some(u32::from(b'A')));
        tank.fish[0].engulf_timer = ENGULF_SECS;
        tank.admit(memory, MEMORY_FISH.to_string());
        tank.tick_engulfment();

        assert_eq!(tank.fish.len(), 1, "the memory fish is engulfed");
        assert_eq!(
            word_zero(named(&tank, MEMORY_FUSED)),
            Some(u32::from(b'A')),
            "word 0 survives a fusion"
        );
    }

    const SCREEN_FISH: &str = "Crt";
    const SCREEN_FUSED: &str = "Ann / Cm / Crt";

    fn drawn_letter_a(bot: &mut BotfishState) {
        bot.install(Part::CathodeArray);
        bot.wire(Part::CathodeArray, "byte", "k");
        bot.wire(Part::CathodeArray, "write_byte", "w");
        let mut channels = ChannelRegistry::new();
        for line in LETTER_A_LINES.into_iter().chain(["w"]) {
            channels.set_level(line, true);
        }
        bot.react(&channels);
    }

    fn surface_start(bot: &BotfishState) -> Option<String> {
        let displays = bot.displays();
        let rows = displays.first()?.rows();
        Some(rows[0].chars().take(4).collect())
    }

    #[test]
    fn a_cathode_keeps_its_picture_through_a_fusion() {
        let mut tank = carrier_of_a_module();
        let screen = botfish(SCREEN_FISH, BOT_WEIGHT_G, drawn_letter_a);
        assert_eq!(
            screen.script().and_then(surface_start).as_deref(),
            Some("⠁⠀⠀⠁"),
            "byte 0 is 0x41: dots 0 and 6 of the top row"
        );
        tank.fish[0].engulf_timer = ENGULF_SECS;
        tank.admit(screen, SCREEN_FISH.to_string());
        tank.tick_engulfment();

        assert_eq!(tank.fish.len(), 1, "the screen fish is engulfed");
        assert_eq!(
            surface_start(named(&tank, SCREEN_FUSED)).as_deref(),
            Some("⠁⠀⠀⠁"),
            "the surface survives a fusion"
        );
    }

    #[test]
    fn cytokinesis_gives_each_half_back_its_own_parts_and_wiring() {
        let mut tank = carrier_engulfs_a_sense(BOT_WEIGHT_G);
        assert!(tank.apply_named_mutation(FUSED, "cytokinesis"));
        assert_eq!(tank.fish.len(), 2);

        let carrier = tank.fish.iter().find(|f| f.name == CARRIER).unwrap();
        assert!(carrier.is_programmable(), "the carrier is still a carrier");
        assert_module_fish(named(&tank, CARRIER));

        let ear = named(&tank, "Ear");
        assert_eq!(
            ear.parts().iter().collect::<Vec<_>>(),
            vec![
                (Part::InverterCoil, 1),
                (Part::DelaySpool, 1),
                (Part::StartleNerve, 1)
            ]
        );
        assert!(ear.hears("b") && ear.drives() == Some("nq"));
    }

    #[test]
    fn endocytosis_onto_the_heavier_botfish_body_keeps_the_receivers_circuit() {
        let mut tank = carrier_engulfs_a_sense(HEAVY_SENSE_WEIGHT_G);
        assert!(tank.apply_named_mutation(FUSED, "endocytosis"));

        let survivor = &tank.fish[0];
        assert_eq!(survivor.species, FishSpecies::Botfish, "the heavier body");
        assert!(survivor.is_programmable());
        assert_fused_circuit(named(&tank, FUSED));

        let buried = tank
            .pending_graveyard
            .iter()
            .find(|f| f.name == CARRIER)
            .and_then(Fish::script)
            .expect("the lost half is buried with its own circuit");
        assert_module_fish(buried);
    }

    fn printed_neo() -> Tank {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        tank.admit(
            botfish("Neo", BOT_WEIGHT_G, module_wiring),
            "Neo".to_string(),
        );
        let bot = tank.fish[0].script_mut().unwrap();
        bot.install(Part::InverterCoil);
        bot.install(Part::DelaySpool);
        bot.mark_printed();
        tank
    }

    #[test]
    fn a_botfish_split_by_telophase_never_copies_its_parts_or_sheds_its_print_mark() {
        let mut tank = printed_neo();
        assert!(tank.apply_named_mutation("Neo", "telophase"));
        assert!(tank.apply_named_mutation("Neo", "cytokinesis"));
        assert_eq!(tank.fish.len(), 2);

        let neo = named(&tank, "Neo");
        assert!(neo.parts().has(Part::InverterCoil) && neo.parts().has(Part::DelaySpool));

        let twin = tank.fish.iter().find(|f| f.name != "Neo").unwrap();
        let twin_circuit = twin.script().expect("a botfish twin is programmable");
        assert!(twin_circuit.parts().is_empty(), "no part is duplicated");
        assert!(
            twin_circuit.is_printed(),
            "a printed fish cannot split into one worth money"
        );
        assert_eq!(twin.sell_value(), 0);
    }

    #[test]
    fn a_doubled_fish_is_still_refused_as_engulf_receiver_and_prey() {
        let mut tank = Tank::new("T".to_string(), TankKind::Base, &[]);
        tank.admit(engulfing_host(), "Ann".to_string());
        tank.admit(botfish("Cm", BOT_WEIGHT_G, module_wiring), "Cm".to_string());
        assert!(tank.apply_named_mutation("Cm", "telophase"));
        tank.tick_engulfment();
        assert_eq!(tank.fish.len(), 2, "a doubled botfish is never prey");

        assert!(tank.apply_named_mutation("Cm", "cytokinesis"));
        tank.fish.retain(|f| f.name == "Ann" || f.name == "Cm");
        assert!(tank.apply_named_mutation("Ann", "telophase"));
        tank.fish[0].engulf_timer = ENGULF_SECS;
        tank.tick_engulfment();
        assert_eq!(tank.fish.len(), 2, "a doubled host never engulfs");
    }

    fn fitted_botfish(kind: TankKind) -> Tank {
        let mut tank = tank_with_botfish(false);
        tank.kind = kind;
        let bot = tank.fish[0].script_mut().expect("a botfish");
        bot.listen("a");
        bot.drive("q");
        bot.install(Part::InverterCoil);
        for channel in ["a", "q", "z"] {
            tank.channels.register(channel);
        }
        tank
    }

    fn fitting(tank: &Tank) -> (Vec<String>, Option<String>, u32) {
        let bot = tank.fish[0].script().expect("a botfish");
        (
            bot.listens().map(str::to_string).collect(),
            bot.drives().map(str::to_string),
            bot.parts().count(Part::InverterCoil),
        )
    }

    fn first_strike(tank: &mut Tank) -> bool {
        let before = fitting(tank);
        for _ in 0..RAD_WATCH_TICKS {
            tank.tick_mutations(RAD_TICK_SECS);
            if fitting(tank) != before {
                return true;
            }
        }
        false
    }

    #[test]
    fn a_radtank_corrupts_a_wired_botfish_instead_of_mutating_it() {
        let mut tank = fitted_botfish(TankKind::Rad);
        assert!(
            first_strike(&mut tank),
            "ten minutes at a thirty-second mean always lands a strike"
        );
        assert!(
            tank.fish[0]
                .mutations
                .as_ref()
                .is_none_or(|record| record.count == 0),
            "radiation still never mutates a wired fish"
        );
        assert!(
            tank.observe(0, 0).pulsed(WorldSignal::Mutation),
            "a Geiger hears the strike as the tank's mutation pulse"
        );
    }

    #[test]
    fn a_part_knocked_loose_is_handed_up_to_the_player_never_destroyed() {
        let mut tank = fitted_botfish(TankKind::Rad);
        tank.channels = ChannelRegistry::new();
        let bot = tank.fish[0].script_mut().expect("a botfish");
        bot.unlisten("a");
        bot.unwire(PinOwner::Body, "out");

        assert!(first_strike(&mut tank));

        assert_eq!(tank.pending_loose_parts, vec![Part::InverterCoil]);
        assert_eq!(fitting(&tank).2, 0);
    }

    #[test]
    fn outside_a_radtank_a_circuit_is_never_touched() {
        let mut tank = fitted_botfish(TankKind::Base);
        assert!(!first_strike(&mut tank));
        assert!(tank.pending_loose_parts.is_empty());
    }
}
