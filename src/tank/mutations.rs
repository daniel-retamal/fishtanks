use rand::RngExt;

use crate::entities::cow::Cow;
use crate::fishes::botfish::Strike;
use crate::fishes::fish::{Fish, compute_display_width};
use crate::fishes::mutant::{
    EXTRA_BODY_FOR_DOUBLE, EyeState, MIN_BODY_CHARS, MutantState, MutantTail, MutationRecord,
};
use crate::fishes::mutations::{
    MutantBacked, Mutatable, Mutation, MutationOutcome, apply_mutation, ensure_fish_mutant,
};
use crate::fishes::revert::{revert, revert_latest, settle};
use crate::fishes::species::BodyTemplate;
use crate::fishes::unfish::{UnfishKind, worm_display_width};
use crate::util::{exponential_event, hyperbolic_scale, sample_exponential};

use super::{
    MIN_SPLIT_BODY_SIZE, MUTATION_ALPHA, MUTATION_INTERVAL_BASE, MUTATION_MEAN_FLOOR_SECS,
    MUTATION_TIMER_UNARMED, RAD_AUTO_MUTANT_MEAN_SECS, RAD_MILK_MUTATIONS_PER_SEC,
    RAD_MUTATION_MEAN_SECS, RAD_WEIGHT_GAIN_G, RAD_WEIGHT_INTERVAL_SECS,
};
use super::{Tank, WorldSignal};

fn rad_mean_secs(fish: &Fish) -> f32 {
    if fish.auto_mutate_stacks() > 0 {
        return RAD_AUTO_MUTANT_MEAN_SECS;
    }
    RAD_MUTATION_MEAN_SECS
}

fn mutation_affects_both_halves(mutation: Mutation) -> bool {
    !mutation.fuses()
}

fn restore_from_snapshot(snapshot: &Fish, x: f32, y: f32, weight_g: u32) -> Fish {
    let mut fish = snapshot.clone();
    fish.position.x = x;
    fish.position.y = y;
    fish.weight_g = weight_g;
    fish.engulf_timer = 0.0;
    if let Some(mutant) = fish.mutant.as_mut() {
        mutant.is_double = false;
        mutant.backwards = false;
        mutant.double_head_eyes.clear();
    }
    if let Some(us) = fish.unfish_state.as_mut() {
        us.worm_is_double = false;
        us.worm_backwards = false;
    }
    fish.recompute_display_width();
    fish
}

fn restore_cow_from_snapshot(snapshot: &Cow, x: f32, y: f32) -> Cow {
    let mut cow = snapshot.clone();
    cow.position.x = x;
    cow.position.y = y;
    cow.engulf_timer = 0.0;
    cow.mutant.is_double = false;
    cow.mutant.backwards = false;
    cow.mutant.double_head_eyes.clear();
    cow.recompute_display_width();
    cow
}

impl Tank {
    pub(super) fn tick_mutations(&mut self, dt: f32) {
        if self.kind.config().auto_mutate_all {
            self.tick_rad_mutations(dt);
            return;
        }
        let mutant_count: u32 = self.fish.iter().map(|f| f.auto_mutate_stacks()).sum();
        if mutant_count == 0 {
            self.mutation_timer = MUTATION_TIMER_UNARMED;
            return;
        }
        let mut rng = rand::rng();
        let mean = hyperbolic_scale(MUTATION_INTERVAL_BASE, mutant_count, MUTATION_ALPHA)
            .max(MUTATION_MEAN_FLOOR_SECS);
        if self.mutation_timer == MUTATION_TIMER_UNARMED {
            self.mutation_timer = sample_exponential(&mut rng, mean);
        }
        self.mutation_timer -= dt;
        if self.mutation_timer <= 0.0 {
            self.mutation_timer = sample_exponential(&mut rng, mean);
            self.apply_random_mutation();
        }
    }

    fn tick_rad_mutations(&mut self, dt: f32) {
        let mut rng = rand::rng();
        let struck: Vec<usize> = (0..self.fish.len())
            .filter(|&index| exponential_event(&mut rng, rad_mean_secs(&self.fish[index]), dt))
            .collect();
        let (wired, plain): (Vec<usize>, Vec<usize>) = struck
            .into_iter()
            .partition(|&index| self.fish[index].is_wired());
        let fish_to_mutate: Vec<String> = plain
            .iter()
            .map(|&index| self.fish[index].name.clone())
            .collect();
        for index in wired {
            self.corrupt(index, &mut rng);
        }
        for name in fish_to_mutate {
            self.apply_named_mutation(&name, "");
        }
        let cows_to_mutate: Vec<String> = self
            .cows
            .iter()
            .filter(|_| exponential_event(&mut rng, RAD_MUTATION_MEAN_SECS, dt))
            .map(|c| c.name.clone())
            .collect();
        for name in cows_to_mutate {
            self.apply_named_mutation(&name, "");
        }
        self.rad_weight_timer -= dt;
        if self.rad_weight_timer <= 0.0 {
            self.rad_weight_timer += RAD_WEIGHT_INTERVAL_SECS;
            for fish in &mut self.fish {
                let stacks = fish.auto_mutate_stacks();
                if stacks > 0 {
                    fish.weight_g += RAD_WEIGHT_GAIN_G * stacks;
                }
            }
        }
    }

    fn corrupt(&mut self, index: usize, rng: &mut impl RngExt) {
        let channels: Vec<String> = self.channels.names().map(str::to_string).collect();
        let Some(bot) = self.fish[index].script_mut() else {
            return;
        };
        let Some(strike) = bot.irradiate(&channels, rng) else {
            return;
        };
        if let Strike::KnockedLoose(part) = strike {
            self.pending_loose_parts.push(part);
        }
        self.signal(WorldSignal::Mutation);
    }

    fn apply_random_mutation(&mut self) {
        let mutant_indices: Vec<usize> = self
            .fish
            .iter()
            .enumerate()
            .filter_map(|(i, f)| {
                if f.auto_mutate_stacks() > 0 {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();
        if mutant_indices.is_empty() {
            return;
        }
        let mut rng = rand::rng();
        let fish_idx = mutant_indices[rng.random_range(0..mutant_indices.len())];
        self.mutate_fish(fish_idx, "");
    }

    pub(super) fn tick_irradiated_milk_mutations(&mut self, dt: f32) {
        let beats = self
            .milk_clock
            .beats(dt, RAD_MILK_MUTATIONS_PER_SEC.recip());
        for _ in 0..beats {
            let pending: Vec<String> = self
                .fish
                .iter()
                .filter(|f| f.pending_rad_mutations > 0)
                .map(|f| f.name.clone())
                .collect();
            for name in pending {
                if let Some(fish) = self.fish.iter_mut().find(|f| f.name == name) {
                    fish.pending_rad_mutations -= 1;
                }
                self.apply_named_mutation(&name, "");
            }
        }
    }

    pub fn apply_named_mutation(&mut self, name: &str, token: &str) -> bool {
        if let Some(idx) = self
            .fish
            .iter()
            .position(|f| f.name.eq_ignore_ascii_case(name))
        {
            return self.mutate_fish(idx, token);
        }
        if let Some(idx) = self
            .cows
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(name))
        {
            return self.mutate_cow(idx, token);
        }
        false
    }

    fn resolve_mutation<M: Mutatable>(
        target: &M,
        token: &str,
        room_to_divide: bool,
        rng: &mut impl RngExt,
    ) -> Option<Mutation> {
        let mutation = if token.is_empty() {
            target.random_mutation_with_room(rng, room_to_divide)?
        } else {
            Mutation::parse(token)?
        };
        let has_room = room_to_divide || !mutation.divides();
        (has_room && target.supports_now(mutation)).then_some(mutation)
    }

    fn mutate_fish(&mut self, idx: usize, token: &str) -> bool {
        let mut rng = rand::rng();
        let room_to_divide = !self.is_full();
        let Some(mutation) =
            Self::resolve_mutation(&self.fish[idx], token, room_to_divide, &mut rng)
        else {
            return false;
        };
        if mutation == Mutation::Revert {
            return self.revert_fish(idx, &mut rng);
        }
        let was_fused = self.fish[idx].fused_render_halves().is_some();
        let outcome = apply_mutation(&mut self.fish[idx], mutation, &mut rng);
        self.propagate_mutation_to_halves(idx, mutation, &mut rng);
        if was_fused && mutation == Mutation::Endocytosis {
            self.collapse_endocytosis(idx, &mut rng);
            settle(&mut self.fish[idx]);
        }
        if matches!(outcome, MutationOutcome::SplitRequested) {
            self.split_fish(idx);
            settle(&mut self.fish[idx]);
        }
        self.signal(WorldSignal::Mutation);
        true
    }

    fn revert_fish(&mut self, idx: usize, rng: &mut impl RngExt) -> bool {
        let Some(undone) = revert(&mut self.fish[idx], rng) else {
            return false;
        };
        for component in self.fish[idx].fused_components_mut() {
            if let Some(snapshot) = component.fish_snapshot_mut() {
                revert_latest(snapshot, undone);
            }
        }
        self.fish[idx].refresh_width();
        self.signal(WorldSignal::Mutation);
        true
    }

    fn collapse_endocytosis(&mut self, idx: usize, rng: &mut impl RngExt) {
        let components = self.fish[idx].fused_components().to_vec();
        if components.len() != 2 {
            return;
        }
        let host_name = self.fish[idx].name.clone();
        let host_position = self.fish[idx].position.clone();
        let host_weight = self.fish[idx].weight_g;
        let host_mutations = self.fish[idx].mutations.clone();
        let host_devil_marked = self.fish[idx].devil_marked;

        let heavier_idx = usize::from(components[1].weight_g > components[0].weight_g);
        let lost = &components[1 - heavier_idx];
        if let Some(lost_snapshot) = lost.fish_snapshot() {
            let mut grave_fish = restore_from_snapshot(
                lost_snapshot,
                host_position.x,
                host_position.y,
                lost.weight_g,
            );
            grave_fish.name = lost.name.clone();
            self.pending_graveyard.push(grave_fish);
            self.signal(WorldSignal::Death);
        }

        let Some(heavier) = components[heavier_idx].fish_snapshot() else {
            return;
        };
        let mut merged = heavier.clone();
        merged.name = host_name;
        merged.position = host_position;
        merged.weight_g = host_weight;
        merged.mutations = host_mutations;
        merged.devil_marked = host_devil_marked;
        merged.engulf_timer = 0.0;
        let mut ledger: Vec<_> = components
            .into_iter()
            .flat_map(|component| component.flattened())
            .collect();
        if merged.botfish_state.is_some()
            && let Some(circuit) = ledger
                .iter_mut()
                .find_map(|component| component.program.take())
        {
            merged.botfish_state = Some(circuit);
        }
        if let Some(us) = merged.unfish_state.as_mut() {
            us.worm_is_double = false;
            us.worm_backwards = false;
            us.fused = ledger;
        } else {
            ensure_fish_mutant(&mut merged, rng);
            let mutant = merged.mutant.as_mut().unwrap();
            mutant.is_double = false;
            mutant.backwards = false;
            mutant.double_head_eyes.clear();
            mutant.fused = ledger;
        }
        merged.recompute_display_width();
        self.fish[idx] = merged;
    }

    fn propagate_mutation_to_halves(
        &mut self,
        idx: usize,
        mutation: Mutation,
        rng: &mut impl RngExt,
    ) {
        if !mutation_affects_both_halves(mutation) || self.fish[idx].fused_render_halves().is_none()
        {
            return;
        }
        for component in self.fish[idx].fused_components_mut() {
            if let Some(snapshot) = component.fish_snapshot_mut()
                && snapshot.supports_now(mutation)
            {
                apply_mutation(snapshot, mutation, rng);
            }
        }
        self.fish[idx].recompute_display_width();
    }

    fn mutate_cow(&mut self, idx: usize, token: &str) -> bool {
        let mut rng = rand::rng();
        let Some(mutation) = Self::resolve_mutation(&self.cows[idx], token, true, &mut rng) else {
            return false;
        };
        if mutation == Mutation::Revert {
            let Some(undone) = revert(&mut self.cows[idx], &mut rng) else {
                return false;
            };
            for component in &mut self.cows[idx].mutant.fused {
                if let Some(snapshot) = component.cow_snapshot_mut() {
                    revert_latest(snapshot, undone);
                }
            }
            self.signal(WorldSignal::Mutation);
            return true;
        }
        let outcome = apply_mutation(&mut self.cows[idx], mutation, &mut rng);
        self.propagate_cow_mutation_to_halves(idx, mutation, &mut rng);
        if matches!(outcome, MutationOutcome::SplitRequested) {
            self.split_cow(idx);
            settle(&mut self.cows[idx]);
        }
        self.signal(WorldSignal::Mutation);
        true
    }

    fn propagate_cow_mutation_to_halves(
        &mut self,
        idx: usize,
        mutation: Mutation,
        rng: &mut impl RngExt,
    ) {
        if !mutation_affects_both_halves(mutation) {
            return;
        }
        let mutant = &mut self.cows[idx].mutant;
        if !mutant.is_double || mutant.fused.len() != 2 {
            return;
        }
        for component in &mut mutant.fused {
            if let Some(snapshot) = component.cow_snapshot_mut()
                && snapshot.supports_now(mutation)
            {
                apply_mutation(snapshot, mutation, rng);
            }
        }
    }

    fn split_fish(&mut self, idx: usize) {
        if self.fish[idx].unfish_state.is_some() {
            self.split_worm(idx);
            return;
        }
        if matches!(
            self.fish[idx].species.config().body,
            BodyTemplate::Fixed { .. }
        ) {
            self.split_fixed_fish(idx);
            return;
        }
        self.split_standard_fish(idx);
    }

    fn try_split_engulfment_fish(&mut self, idx: usize) -> bool {
        let (c0, c1) = {
            let fish = &self.fish[idx];
            let comps = fish.fused_components();
            if !fish.is_double_now() || comps.len() != 2 {
                return false;
            }
            let (c0, c1) = (&comps[0], &comps[1]);
            if c0.name == c1.name && c0.lineage == c1.lineage {
                return false;
            }
            if c0.fish_snapshot().is_none() || c1.fish_snapshot().is_none() {
                return false;
            }
            (c0.clone(), c1.clone())
        };
        let spawn_x = self.fish[idx].position.x;
        let spawn_y = self.fish[idx].position.y;
        let parent_count = self.fish[idx].mutations.as_ref().map_or(0, |mr| mr.count);
        let mutations = self.fish[idx].mutations.clone();

        let mut host =
            restore_from_snapshot(c0.fish_snapshot().unwrap(), spawn_x, spawn_y, c0.weight_g);
        host.name = c0.name.clone();
        host.mutations = mutations;
        self.fish[idx] = host;

        let mut child =
            restore_from_snapshot(c1.fish_snapshot().unwrap(), spawn_x, spawn_y, c1.weight_g);
        child.name = c1.name.clone();
        child.mutations = Some(Box::new(MutationRecord::child_of(parent_count, &c0.name)));
        self.admit(child, c1.name.clone());
        self.fish[idx].record_mut().partners.push(c1.name);
        true
    }

    fn split_standard_fish(&mut self, idx: usize) {
        if self.try_split_engulfment_fish(idx) {
            return;
        }
        let is_double = self.fish[idx]
            .mutant
            .as_ref()
            .is_some_and(|mutant| mutant.is_double);
        if !is_double {
            return;
        }

        let parent_body_size = self.fish[idx].body_size;
        let parent_color = self.fish[idx].color;
        let parent_sway_speed = self.fish[idx].sway_speed;
        let spawn_x = self.fish[idx].position.x;
        let spawn_y = self.fish[idx].position.y;
        let parent_name = self.fish[idx].name.clone();

        let (
            parent_max_eyes,
            double_eye_count,
            half,
            other_half,
            new_patches,
            body_variant,
            glistening_mode,
            glistening_color,
            eye_color,
        ) = {
            let mutant = self.fish[idx].mutant.as_ref().unwrap();
            let max_eyes = mutant.left_eyes.len().max(mutant.right_eyes.len());
            let double_eye_count = mutant.double_head_eyes.len().max(1);

            let total_body = parent_body_size + EXTRA_BODY_FOR_DOUBLE;
            let raw_half = total_body / 2;
            let min_half = (max_eyes + MIN_BODY_CHARS).max(MIN_SPLIT_BODY_SIZE);
            let min_other = (double_eye_count + MIN_BODY_CHARS).max(MIN_SPLIT_BODY_SIZE);
            let half = raw_half.max(min_half);
            let other_half = (total_body - raw_half).max(min_other);

            let split_pos = 1 + max_eyes + half;
            let body_start_new = 1 + double_eye_count;
            let new_patches: Vec<_> = mutant
                .color_patches
                .iter()
                .filter(|&&(pos, _)| pos >= split_pos && pos < split_pos + other_half)
                .map(|&(pos, c)| (pos - split_pos + body_start_new, c))
                .collect();

            (
                max_eyes,
                double_eye_count,
                half,
                other_half,
                new_patches,
                mutant.body_variant,
                mutant.glistening_mode,
                mutant.glistening_color,
                mutant.eye_color,
            )
        };
        let parent_count = self.fish[idx].mutations.as_ref().map_or(0, |mr| mr.count);

        let mut rng = rand::rng();
        let tail_for_new = match rng.random_range(0u32..3) {
            0 => MutantTail::Wide,
            1 => MutantTail::Swaying,
            _ => MutantTail::Curly,
        };

        {
            let fish = &mut self.fish[idx];
            fish.body_size = half;
            let mutant = fish.mutant.as_mut().unwrap();
            mutant.is_double = false;
            mutant.backwards = false;
            mutant.fused.clear();
            mutant.double_head_eyes.clear();
            mutant.hydra_eyes.truncate(half.saturating_sub(1));
            let split_pos_orig = 1 + parent_max_eyes + half;
            mutant
                .color_patches
                .retain(|&(pos, _)| pos < split_pos_orig);
            fish.display_width = mutant.display_width(half);
        }

        let new_name = self.unique_name(&parent_name);
        let parent_species = self.fish[idx].species;
        let mut new_fish = Fish::new(parent_species, new_name.clone(), spawn_x, spawn_y, &mut rng);
        ensure_fish_mutant(&mut new_fish, &mut rng);
        new_fish.color = parent_color;
        new_fish.sway_speed = parent_sway_speed;
        new_fish.body_size = other_half;
        {
            let mutant = new_fish.mutant.as_mut().unwrap();
            mutant.body_variant = body_variant;
            mutant.glistening_mode = glistening_mode;
            mutant.glistening_color = glistening_color;
            mutant.eye_color = eye_color;
            mutant.tail_variant = tail_for_new;
            mutant.color_patches = new_patches;
            mutant.is_double = false;
            mutant.double_head_eyes.clear();
            mutant.left_eyes.clear();
            mutant.right_eyes.clear();
            for _ in 0..double_eye_count {
                mutant.left_eyes.push(EyeState::new(&mut rng));
                mutant.right_eyes.push(EyeState::new(&mut rng));
            }
        }
        new_fish.display_width = new_fish.mutant.as_ref().unwrap().display_width(other_half);
        new_fish.mutations = Some(Box::new(MutationRecord::child_of(
            parent_count,
            &parent_name,
        )));
        self.admit(new_fish, new_name.clone());
        self.fish[idx].record_mut().partners.push(new_name);
    }

    fn split_fixed_fish(&mut self, idx: usize) {
        let is_double = self.fish[idx]
            .mutant
            .as_ref()
            .is_some_and(|mutant| mutant.is_double);
        if !is_double {
            return;
        }
        let species = self.fish[idx].species;
        let parent_name = self.fish[idx].name.clone();
        let spawn_x = self.fish[idx].position.x;
        let spawn_y = self.fish[idx].position.y;
        let parent_color = self.fish[idx].color;
        let parent_sway_speed = self.fish[idx].sway_speed;
        let (glistening_mode, glistening_color, eye_color) = {
            let mutant = self.fish[idx].mutant.as_ref().unwrap();
            (
                mutant.glistening_mode,
                mutant.glistening_color,
                mutant.eye_color,
            )
        };
        let parent_count = self.fish[idx].mutations.as_ref().map_or(0, |mr| mr.count);
        let ear_count = self.fish[idx].mutant.as_ref().map_or(0, |m| m.ear_count);
        {
            let mutant = self.fish[idx].mutant.as_mut().unwrap();
            mutant.is_double = false;
            mutant.backwards = false;
            mutant.fused.clear();
            mutant.double_head_eyes.clear();
        }
        self.fish[idx].display_width =
            compute_display_width(species, self.fish[idx].body_size) + ear_count;
        let mut rng = rand::rng();
        let new_name = self.unique_name(&parent_name);
        let mut new_fish = Fish::new(species, new_name.clone(), spawn_x, spawn_y, &mut rng);
        new_fish.color = parent_color;
        new_fish.sway_speed = parent_sway_speed;
        new_fish.display_width = compute_display_width(species, new_fish.body_size);
        let mut new_mutant = MutantState::new_for_standard(MutantTail::Wide, &mut rng);
        new_mutant.glistening_mode = glistening_mode;
        new_mutant.glistening_color = glistening_color;
        new_mutant.eye_color = eye_color;
        new_fish.mutant = Some(Box::new(new_mutant));
        new_fish.mutations = Some(Box::new(MutationRecord::child_of(
            parent_count,
            &parent_name,
        )));
        let printed = self.fish[idx].script().is_some_and(|bot| bot.is_printed());
        if printed && let Some(twin) = new_fish.script_mut() {
            twin.mark_printed();
        }
        self.admit(new_fish, new_name.clone());
        self.fish[idx].record_mut().partners.push(new_name);
    }

    fn split_worm(&mut self, idx: usize) {
        if self.try_split_engulfment_fish(idx) {
            return;
        }
        let is_double = self.fish[idx]
            .unfish_state
            .as_ref()
            .is_some_and(|unfish_state| unfish_state.worm_is_double);
        if !is_double {
            return;
        }
        let parent_name = self.fish[idx].name.clone();
        let spawn_x = self.fish[idx].position.x;
        let spawn_y = self.fish[idx].position.y;
        let (
            segments,
            extra_eyes,
            body_color,
            glisten_enabled,
            glisten_color,
            glisten_mode,
            glisten_speed,
            ear_count,
            ear_color,
            hydra_count,
        ) = {
            let unfish_state = self.fish[idx].unfish_state.as_ref().unwrap();
            (
                unfish_state.worm_segments,
                unfish_state.worm_extra_eyes,
                unfish_state.slime_body_color,
                unfish_state.slime_glisten_enabled,
                unfish_state.slime_glisten_color,
                unfish_state.slime_glisten_mode,
                unfish_state.slime_glisten_speed,
                unfish_state.ear_count,
                unfish_state.ear_color,
                unfish_state.hydra_count,
            )
        };
        let parent_count = self.fish[idx].mutations.as_ref().map_or(0, |mr| mr.count);
        let half = (segments / 2).max(1);
        let other_half = (segments - half).max(1);
        let parent_hydra = hydra_count.min(half.saturating_sub(1));
        let child_hydra = hydra_count.min(other_half.saturating_sub(1));
        {
            let unfish_state = self.fish[idx].unfish_state.as_mut().unwrap();
            unfish_state.worm_segments = half;
            unfish_state.worm_is_double = false;
            unfish_state.worm_backwards = false;
            unfish_state.fused.clear();
            unfish_state.hydra_count = parent_hydra;
        }
        self.fish[idx].display_width =
            worm_display_width(half, extra_eyes, false, ear_count, parent_hydra);
        let mut rng = rand::rng();
        let new_name = self.unique_name(&parent_name);
        let mut new_fish = Fish::new_unfish(
            UnfishKind::Worm,
            new_name.clone(),
            spawn_x,
            spawn_y,
            &mut rng,
        );
        if let Some(ref mut unfish_state) = new_fish.unfish_state {
            unfish_state.worm_segments = other_half;
            unfish_state.worm_extra_eyes = extra_eyes;
            unfish_state.worm_is_double = false;
            unfish_state.slime_body_color = body_color;
            unfish_state.slime_glisten_enabled = glisten_enabled;
            unfish_state.slime_glisten_color = glisten_color;
            unfish_state.slime_glisten_mode = glisten_mode;
            unfish_state.slime_glisten_speed = glisten_speed;
            unfish_state.ear_count = ear_count;
            unfish_state.ear_color = ear_color;
            unfish_state.hydra_count = child_hydra;
        }
        new_fish.display_width =
            worm_display_width(other_half, extra_eyes, false, ear_count, child_hydra);
        new_fish.mutations = Some(Box::new(MutationRecord::child_of(
            parent_count,
            &parent_name,
        )));
        self.admit(new_fish, new_name.clone());
        self.fish[idx].record_mut().partners.push(new_name);
    }

    fn try_split_engulfment_cow(&mut self, idx: usize) -> bool {
        let (c0, c1) = {
            let mutant = &self.cows[idx].mutant;
            if !mutant.is_double || mutant.fused.len() != 2 {
                return false;
            }
            let c0 = &mutant.fused[0];
            let c1 = &mutant.fused[1];
            if c0.name == c1.name {
                return false;
            }
            if c0.cow_snapshot().is_none() || c1.cow_snapshot().is_none() {
                return false;
            }
            (c0.clone(), c1.clone())
        };
        let parent_x = self.cows[idx].position.x;
        let parent_y = self.cows[idx].position.y;
        let parent_count = self.cows[idx].mutations.as_ref().map_or(0, |mr| mr.count);
        let parent_mutations = self.cows[idx].mutations.clone();

        let mut parent = restore_cow_from_snapshot(c0.cow_snapshot().unwrap(), parent_x, parent_y);
        parent.name = c0.name.clone();
        parent.mutations = parent_mutations;
        let parent_w = parent.display_width;
        self.cows[idx] = parent;

        let mut child = restore_cow_from_snapshot(
            c1.cow_snapshot().unwrap(),
            parent_x + parent_w as f32 + 1.0,
            parent_y,
        );
        child.name = c1.name.clone();
        child.mutations = Some(Box::new(MutationRecord::child_of(parent_count, &c0.name)));
        let max_x = (self.width as i32 - child.display_width as i32).max(0) as f32;
        if child.position.x > max_x {
            child.position.x = max_x;
        }
        self.used_cow_names.insert(c1.name.clone());
        self.cows[idx].record_mut().partners.push(c1.name);
        self.cows.push(child);
        true
    }

    fn split_cow(&mut self, idx: usize) {
        use crate::entities::cow::Cow;
        if self.try_split_engulfment_cow(idx) {
            return;
        }
        if !self.cows[idx].mutant.is_double {
            return;
        }
        let parent_name = self.cows[idx].name.clone();
        let parent_x = self.cows[idx].position.x;
        let parent_y = self.cows[idx].position.y;
        let parent_w = self.cows[idx].display_width;
        let variant = self.cows[idx].variant;
        let parent_color = self.cows[idx].color;
        let parent_sway_speed = self.cows[idx].sway_speed;
        let (right_eyes_count, body_variant, glistening_mode, glistening_color, eye_color) = {
            let m = &self.cows[idx].mutant;
            (
                m.double_head_eyes.len().max(2),
                m.body_variant,
                m.glistening_mode,
                m.glistening_color,
                m.eye_color,
            )
        };
        let parent_count = self.cows[idx].mutations.as_ref().map_or(0, |mr| mr.count);

        {
            let p = &mut self.cows[idx];
            p.mutant.is_double = false;
            p.mutant.backwards = false;
            p.mutant.fused.clear();
            p.mutant.double_head_eyes.clear();
            p.recompute_display_width();
        }

        let mut rng = rand::rng();
        let new_name = self.unique_cow_name(&parent_name);
        let mut new_cow = Cow::new(
            new_name.clone(),
            variant,
            parent_x + parent_w as f32 + 1.0,
            parent_y,
            &mut rng,
        );
        new_cow.color = parent_color;
        new_cow.sway_speed = parent_sway_speed;
        new_cow.mutant.left_eyes.clear();
        for _ in 0..right_eyes_count {
            new_cow.mutant.left_eyes.push(EyeState::new(&mut rng));
        }
        new_cow.mutant.eye_color = eye_color;
        new_cow.mutant.glistening_mode = glistening_mode;
        new_cow.mutant.glistening_color = glistening_color;
        new_cow.mutant.body_variant = body_variant;
        new_cow.mutant.is_double = false;
        new_cow.mutant.double_head_eyes.clear();
        new_cow.recompute_display_width();

        let max_x = (self.width as i32 - new_cow.display_width as i32).max(0) as f32;
        if new_cow.position.x > max_x {
            new_cow.position.x = max_x;
        }

        new_cow.mutations = Some(Box::new(MutationRecord::child_of(
            parent_count,
            &parent_name,
        )));
        self.used_cow_names.insert(new_name.clone());
        self.cows[idx].record_mut().partners.push(new_name);
        self.cows.push(new_cow);
    }
}
