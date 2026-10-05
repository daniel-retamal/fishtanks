use rand::RngExt;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use super::simulation::fish_near;
use super::{Tank, WorldSignal};
use crate::colors::GRAY;
use crate::entities::components::Position;
use crate::entities::food::FOOD_WEIGHT_GAIN_G;
use crate::fishes::botfish::BotfishState;
use crate::fishes::fish::{Direction, Fish};
use crate::fishes::mutations::{Mutatable, Mutation};
use crate::fishes::quirk::{
    ANAGRAM_SHUFFLE_MEAN_SECS, BONES_RIBS_CAP, Cling, FORGET_MUTATION_MEAN_SECS,
    GRAEAE_PASS_REST_SECS, LEECH_BITE_MEAN_SECS, MOLT_MEAN_SECS, Quirk,
};
use crate::fishes::species::{EYE_CIRCLE, EYE_ROUND, SizeCategory};
use crate::fishes::unfish::{RING_HOLE_COLS, RING_HOLE_ROWS, UnfishKind};
use crate::names;
use crate::util::sample_exponential;

const LEECH_REACH_X: f32 = 2.0;
const LEECH_REACH_Y: f32 = 1.5;
const LEECH_SEEK_SPEED: f32 = 3.0;
const STILL_GAP: f32 = 1.0;
const BONES_SPACING: f32 = 3.0;
const SHED_SINK_PER_SEC: f32 = 0.5;
const SHED_SECS: f32 = 30.0;
const SHED_GLYPH: char = '·';
const SHED_COLOR: Color = GRAY;
const NEGATIVE: &str = "negative";
const REVERT: &str = "revert";

#[derive(Clone, Serialize, Deserialize)]
pub struct Shedding {
    pub x: f32,
    pub y: f32,
    pub cells: Vec<(char, Color)>,
    pub age: f32,
}

impl Shedding {
    fn of(fish: &Fish) -> Self {
        let sprite = fish.line_sprite();
        let body = sprite
            .rows
            .get(sprite.body_row)
            .cloned()
            .unwrap_or_default();
        let cells = body
            .into_iter()
            .map(|(glyph, _)| {
                let glyph = if matches!(glyph, EYE_ROUND | EYE_CIRCLE) {
                    SHED_GLYPH
                } else {
                    glyph
                };
                (glyph, SHED_COLOR)
            })
            .collect();
        Shedding {
            x: fish.position.x,
            y: fish.position.y,
            cells,
            age: 0.0,
        }
    }
}

fn smallest_weight(fish: &Fish) -> u32 {
    let bases = fish.species.config().weight_base;
    bases
        .iter()
        .copied()
        .filter(|&base| base > 0)
        .min()
        .unwrap_or(FOOD_WEIGHT_GAIN_G)
}

fn catch_weight(fish: &Fish) -> u32 {
    fish.species.config().weight_base[fish.size_category as usize]
}

fn can_be_drunk(fish: &Fish) -> bool {
    !fish.is_unfish() && fish.weight_g >= catch_weight(fish) + FOOD_WEIGHT_GAIN_G
}

fn center(fish: &Fish) -> f32 {
    fish.position.x + fish.display_width as f32 / 2.0
}

fn distance(a: &Fish, b: &Fish) -> f32 {
    (center(a) - center(b)).abs() + (a.position.y - b.position.y).abs()
}

fn leech_of(fish: &Fish) -> Option<(Option<String>, Option<Cling>)> {
    match fish.quirk()? {
        Quirk::Leech(leech) => Some((leech.host.clone(), leech.cling)),
        _ => None,
    }
}

impl Tank {
    pub fn seated(&self) -> usize {
        self.fish.iter().filter(|fish| fish.takes_a_seat()).count()
    }

    pub(super) fn hold_the_still(&self) -> Vec<(usize, Position)> {
        self.fish
            .iter()
            .enumerate()
            .filter(|(_, fish)| fish.holds_still())
            .map(|(i, fish)| (i, fish.position.clone()))
            .collect()
    }

    pub(super) fn tick_oddities(
        &mut self,
        dt: f32,
        held: Vec<(usize, Position)>,
        rng: &mut impl RngExt,
    ) {
        self.tick_still(dt, held);
        self.tick_leeches(rng);
        self.tick_bones(rng);
        self.tick_negatives(rng);
        self.tick_forgetting(rng);
        self.tick_graeae();
        self.tick_rings();
        self.tick_molts(dt, rng);
        self.tick_anagrams(rng);
    }

    fn tick_still(&mut self, dt: f32, held: Vec<(usize, Position)>) {
        for (i, position) in held {
            let Some(still) = self.fish.get(i) else {
                continue;
            };
            let step = still.speed * dt;
            let target = if self.watched {
                None
            } else {
                self.fish
                    .iter()
                    .enumerate()
                    .filter(|(j, other)| *j != i && !other.is_unfish())
                    .min_by(|(_, a), (_, b)| {
                        distance(still, a).partial_cmp(&distance(still, b)).unwrap()
                    })
                    .map(|(_, other)| {
                        let beside_left = center(still) < center(other);
                        let x = if beside_left {
                            other.position.x - still.display_width as f32 - STILL_GAP
                        } else {
                            other.position.x + other.display_width as f32 + STILL_GAP
                        };
                        (x, other.position.y, beside_left)
                    })
            };
            let fish = &mut self.fish[i];
            fish.position = position;
            let Some((x, y, beside_left)) = target else {
                continue;
            };
            let (dx, dy) = (x - fish.position.x, y - fish.position.y);
            let reach = (dx * dx + dy * dy).sqrt();
            if reach > step {
                fish.position.x += dx / reach * step;
                fish.position.y += dy / reach * step;
            } else {
                fish.position.x = x;
                fish.position.y = y;
            }
            fish.facing = if beside_left {
                Direction::Right
            } else {
                Direction::Left
            };
        }
    }

    fn tick_leeches(&mut self, rng: &mut impl RngExt) {
        for i in 0..self.fish.len() {
            let Some((host, cling)) = leech_of(&self.fish[i]) else {
                continue;
            };
            let host_idx = host
                .as_ref()
                .and_then(|name| self.fish.iter().position(|f| &f.name == name))
                .filter(|&h| can_be_drunk(&self.fish[h]));
            match (host_idx, cling) {
                (Some(h), Some(cling)) => self.cling(i, h, cling, rng),
                _ => self.seek_a_host(i),
            }
        }
    }

    fn taken_clings(&self, host: &str) -> Vec<Cling> {
        self.fish
            .iter()
            .filter_map(leech_of)
            .filter(|(name, _)| name.as_deref() == Some(host))
            .filter_map(|(_, cling)| cling)
            .collect()
    }

    fn seek_a_host(&mut self, i: usize) {
        let leech = &self.fish[i];
        let target = self
            .fish
            .iter()
            .enumerate()
            .filter(|(j, fish)| *j != i && can_be_drunk(fish))
            .filter(|(_, fish)| self.taken_clings(&fish.name).len() < 2)
            .min_by(|(_, a), (_, b)| distance(leech, a).partial_cmp(&distance(leech, b)).unwrap())
            .map(|(j, _)| j);
        let Some(h) = target else {
            if let Some(Quirk::Leech(state)) = self.fish[i].quirk_mut() {
                state.host = None;
                state.cling = None;
            }
            return;
        };
        let (host_x, host_y) = (center(&self.fish[h]), self.fish[h].position.y);
        let name = self.fish[h].name.clone();
        let taken = self.taken_clings(&name);
        let leech = &mut self.fish[i];
        let (dx, dy) = (host_x - center(leech), host_y - leech.position.y);
        if dx.abs() <= LEECH_REACH_X && dy.abs() <= LEECH_REACH_Y {
            let cling = if taken.contains(&Cling::Below) {
                Cling::Above
            } else {
                Cling::Below
            };
            if let Some(Quirk::Leech(state)) = leech.quirk_mut() {
                state.host = Some(name);
                state.cling = Some(cling);
            }
            return;
        }
        let reach = (dx * dx + dy * dy).sqrt().max(f32::MIN_POSITIVE);
        leech.velocity.dx = dx / reach * LEECH_SEEK_SPEED * leech.speed;
        leech.velocity.dy = dy / reach * LEECH_SEEK_SPEED * leech.speed;
        leech.facing = if dx < 0.0 {
            Direction::Left
        } else {
            Direction::Right
        };
    }

    fn cling(&mut self, i: usize, h: usize, cling: Cling, rng: &mut impl RngExt) {
        let floor = self.height.saturating_sub(1) as f32;
        let (host_x, host_y, facing) = (
            center(&self.fish[h]),
            self.fish[h].position.y,
            self.fish[h].facing,
        );
        let leech = &mut self.fish[i];
        leech.position.x = host_x - leech.display_width as f32 / 2.0;
        leech.position.y = match cling {
            Cling::Below => (host_y + 1.0).min(floor),
            Cling::Above => (host_y - 1.0).max(0.0),
        };
        leech.facing = facing;
        leech.velocity.dx = 0.0;
        leech.velocity.dy = 0.0;
        let Some(Quirk::Leech(state)) = leech.quirk_mut() else {
            return;
        };
        if state.bite_clock > 0.0 {
            return;
        }
        state.bite_clock = sample_exponential(rng, LEECH_BITE_MEAN_SECS);
        state.drunk_g += FOOD_WEIGHT_GAIN_G;
        let drunk = state.drunk_g;
        let host = &mut self.fish[h];
        host.weight_g = host.weight_g.saturating_sub(FOOD_WEIGHT_GAIN_G);
        if drunk >= smallest_weight(host) {
            self.leechling(i, h);
        }
    }

    fn leechling(&mut self, i: usize, h: usize) {
        let leech = &self.fish[i];
        let mut child = self.fish[h].clone();
        child.name = leech.name.clone();
        child.position = leech.position.clone();
        child.facing = leech.facing;
        child.devil_marked = leech.devil_marked;
        child.weight_g = smallest_weight(&child);
        child.size_category = SizeCategory::S;
        child.leeched = true;
        child.frozen = false;
        child.abduction_lock = false;
        child.speech = None;
        child.habits = Box::default();
        child.botfish_state = child
            .botfish_state
            .as_ref()
            .map(|_| Box::new(BotfishState::new()));
        for component in child.fused_components_mut() {
            component.program = None;
        }
        child.cancel_seek();
        self.fish[i] = child;
        self.signal(WorldSignal::Birth);
    }

    fn tick_bones(&mut self, rng: &mut impl RngExt) {
        let bones: Vec<usize> = (0..self.fish.len())
            .filter(|&i| matches!(self.fish[i].quirk(), Some(Quirk::Bones(_))))
            .collect();
        let leader = bones.iter().copied().find(|&i| {
            let fish = &self.fish[i];
            fish.is_zooming() && matches!(fish.quirk(), Some(Quirk::Bones(b)) if !b.scattered)
        });
        if let Some(leader) = leader {
            let state = self.fish[leader].state;
            for &i in &bones {
                if !self.fish[i].is_zooming() && !self.fish[i].is_pinned() {
                    self.fish[i].state = state;
                }
            }
        }
        let mut ended = Vec::new();
        for &i in &bones {
            let zooming = self.fish[i].is_zooming();
            let Some(Quirk::Bones(state)) = self.fish[i].quirk_mut() else {
                continue;
            };
            if zooming {
                state.scattered = true;
            } else if state.scattered {
                state.scattered = false;
                ended.push(i);
            }
        }
        if !ended.is_empty() {
            self.reassemble(ended, rng);
        }
    }

    fn reassemble(&mut self, ended: Vec<usize>, rng: &mut impl RngExt) {
        let total: usize = ended.iter().map(|&i| self.fish[i].body_size).sum();
        let kept = ended.len();
        let fewest = total.div_ceil(BONES_RIBS_CAP).max(1);
        let most = total.min(kept + self.room()).max(fewest);
        let count = rng.random_range(fewest..=most);
        let mut ribs = vec![1usize; count];
        for _ in count..total {
            let open: Vec<usize> = (0..count).filter(|&k| ribs[k] < BONES_RIBS_CAP).collect();
            let Some(&k) = open.get(rng.random_range(0..open.len().max(1))) else {
                break;
            };
            ribs[k] += 1;
        }
        let x = ended.iter().map(|&i| center(&self.fish[i])).sum::<f32>() / kept as f32;
        let y = ended.iter().map(|&i| self.fish[i].position.y).sum::<f32>() / kept as f32;
        let spot = |k: usize| x + (k as f32 - count as f32 / 2.0) * BONES_SPACING;
        for (k, &i) in ended.iter().enumerate().take(count) {
            let fish = &mut self.fish[i];
            fish.body_size = ribs[k];
            fish.display_width = fish.unfish_line_width();
            fish.position.x = spot(k);
            fish.position.y = y;
        }
        let mut gone: Vec<usize> = ended.iter().copied().skip(count).collect();
        gone.sort_unstable_by(|a, b| b.cmp(a));
        for i in gone {
            self.take_fish(i);
        }
        for (k, &size) in ribs.iter().enumerate().skip(kept) {
            let name = names::unique_roman_in(&self.used_names);
            let mut fish = Fish::new_unfish(UnfishKind::Bones, name.clone(), spot(k), y, rng);
            fish.body_size = size;
            fish.display_width = fish.unfish_line_width();
            self.admit(fish, name);
        }
    }

    fn tick_negatives(&mut self, rng: &mut impl RngExt) {
        for i in 0..self.fish.len() {
            if !matches!(self.fish[i].quirk(), Some(Quirk::Negative(_))) {
                continue;
            }
            let touched: Vec<String> = self
                .fish
                .iter()
                .enumerate()
                .filter(|(j, fish)| {
                    *j != i
                        && !fish.is_unfish()
                        && fish.supports_now(Mutation::Negative)
                        && fish_near(&self.fish[i], fish)
                })
                .map(|(_, fish)| fish.name.clone())
                .filter(|name| {
                    matches!(self.fish[i].quirk(), Some(Quirk::Negative(n)) if n.rested(name))
                })
                .collect();
            for name in touched {
                if let Some(Quirk::Negative(negative)) = self.fish[i].quirk_mut() {
                    negative.rest(name.clone(), rng);
                }
                self.apply_named_mutation(&name, NEGATIVE);
            }
        }
    }

    fn tick_forgetting(&mut self, rng: &mut impl RngExt) {
        let due: Vec<String> = self
            .fish
            .iter_mut()
            .filter_map(|fish| {
                let name = fish.name.clone();
                match fish.quirk_mut()? {
                    Quirk::Forgetting(forgetting) if forgetting.forget_clock <= 0.0 => {
                        forgetting.forget_clock =
                            sample_exponential(rng, FORGET_MUTATION_MEAN_SECS);
                        Some(name)
                    }
                    _ => None,
                }
            })
            .collect();
        for name in due {
            self.apply_named_mutation(&name, REVERT);
        }
    }

    fn tick_graeae(&mut self) {
        let sisters: Vec<usize> = (0..self.fish.len())
            .filter(|&i| matches!(self.fish[i].quirk(), Some(Quirk::Graeae(_))))
            .collect();
        let rested = |fish: &Fish| matches!(fish.quirk(), Some(Quirk::Graeae(g)) if g.rest <= 0.0);
        let sighted = |fish: &Fish| matches!(fish.quirk(), Some(Quirk::Graeae(g)) if g.sighted);
        let Some(&holder) = sisters.iter().find(|&&i| sighted(&self.fish[i])) else {
            return;
        };
        if !rested(&self.fish[holder]) {
            return;
        }
        let Some(&blind) = sisters.iter().find(|&&i| {
            i != holder && rested(&self.fish[i]) && fish_near(&self.fish[holder], &self.fish[i])
        }) else {
            return;
        };
        for (i, sight) in [(holder, false), (blind, true)] {
            if let Some(Quirk::Graeae(graeae)) = self.fish[i].quirk_mut() {
                graeae.sighted = sight;
                graeae.rest = GRAEAE_PASS_REST_SECS;
            }
        }
    }

    fn tick_rings(&mut self) {
        let width = self.width as f32;
        let holes: Vec<(f32, f32, f32, f32)> = self
            .fish
            .iter()
            .filter(|fish| fish.unfish_kind() == Some(UnfishKind::Ouroboros))
            .filter_map(|ring| {
                let grid = UnfishKind::Ouroboros.grid()?;
                let top = ring.position.y - grid.center as f32;
                Some((
                    ring.position.x + RING_HOLE_COLS.0 as f32,
                    ring.position.x + RING_HOLE_COLS.1 as f32,
                    top + RING_HOLE_ROWS.0 as f32,
                    top + RING_HOLE_ROWS.1 as f32,
                ))
            })
            .collect();
        if holes.is_empty() {
            return;
        }
        for fish in &mut self.fish {
            if fish.is_unfish() || fish.is_pinned() {
                continue;
            }
            let head = fish.head_x() as f32;
            let y = fish.position.y.round();
            let inside = holes
                .iter()
                .any(|&(x0, x1, y0, y1)| head >= x0 && head <= x1 && y >= y0 && y <= y1);
            if !inside {
                continue;
            }
            fish.position.x = if fish.velocity.dx < 0.0 {
                (width - fish.display_width as f32).max(0.0)
            } else {
                0.0
            };
        }
    }

    pub(super) fn shed_if_molting(&mut self, idx: usize) {
        let Some(fish) = self.fish.get(idx) else {
            return;
        };
        if matches!(fish.quirk(), Some(Quirk::Molt(_))) {
            self.sheddings.push(Shedding::of(fish));
        }
    }

    fn tick_molts(&mut self, dt: f32, rng: &mut impl RngExt) {
        let floor = self.height.saturating_sub(1) as f32;
        for shed in &mut self.sheddings {
            shed.age += dt;
            shed.y = (shed.y + SHED_SINK_PER_SEC * dt).min(floor);
        }
        self.sheddings
            .retain(|shed| shed.age < SHED_SECS && shed.y < floor);
        for i in 0..self.fish.len() {
            let due = match self.fish[i].quirk_mut() {
                Some(Quirk::Molt(molt)) if molt.clock <= 0.0 => {
                    molt.clock = sample_exponential(rng, MOLT_MEAN_SECS);
                    true
                }
                _ => false,
            };
            if due {
                self.shed_if_molting(i);
            }
        }
    }

    fn tick_anagrams(&mut self, rng: &mut impl RngExt) {
        for i in 0..self.fish.len() {
            let due = matches!(self.fish[i].quirk(), Some(Quirk::Anagram(a)) if a.clock <= 0.0);
            if !due {
                continue;
            }
            let source = self
                .fish
                .iter()
                .enumerate()
                .filter(|(j, fish)| {
                    *j != i
                        && fish.unfish_kind() != Some(UnfishKind::Anagram)
                        && !fish.is_invisible()
                })
                .min_by(|(_, a), (_, b)| {
                    distance(&self.fish[i], a)
                        .partial_cmp(&distance(&self.fish[i], b))
                        .unwrap()
                })
                .map(|(j, _)| j)
                .unwrap_or(i);
            let mut glyphs: Vec<char> = if source == i {
                "<º(((((><".chars().collect()
            } else {
                self.fish[source]
                    .static_left_segments()
                    .into_iter()
                    .map(|(c, _)| c)
                    .filter(|&c| c != crate::sprite::TRANSPARENT && c != ' ')
                    .collect()
            };
            if glyphs.len() < 2 {
                continue;
            }
            for k in (1..glyphs.len()).rev() {
                glyphs.swap(k, rng.random_range(0..=k));
            }
            let eye = glyphs
                .iter()
                .position(|&c| matches!(c, EYE_ROUND | EYE_CIRCLE | '°' | '0'));
            let fish = &mut self.fish[i];
            if let Some(Quirk::Anagram(anagram)) = fish.quirk_mut() {
                anagram.glyphs = glyphs;
                anagram.eye = eye;
                anagram.clock = sample_exponential(rng, ANAGRAM_SHUFFLE_MEAN_SECS);
            }
            fish.display_width = fish.unfish_line_width();
        }
    }
}
