use crate::fishes::fish::Fish;
use crate::fishes::fused::Lineage;
use crate::fishes::unfish::UnfishKind;

use crate::dungeon::kit::Kit;
use crate::dungeon::moves;
use crate::dungeon::traits::Trait;

pub fn kinds(fish: &Fish) -> Vec<UnfishKind> {
    let own = fish.unfish_state.as_deref().map(|state| state.kind);
    let fused = fish
        .fused_components()
        .iter()
        .filter_map(|component| match component.lineage {
            Lineage::Unfish(kind) => Some(kind),
            Lineage::Fish(_) | Lineage::Cow(_) => None,
        });
    let mut kinds: Vec<UnfishKind> = own.into_iter().chain(fused).collect();
    kinds.dedup();
    kinds
}

pub fn fit(kit: &mut Kit, fish: &Fish) {
    for kind in kinds(fish) {
        wear(kit, kind);
    }
}

pub fn wear(kit: &mut Kit, kind: UnfishKind) {
    match kind {
        UnfishKind::Reversed => {
            kit.add(Trait::AlwaysLast);
            kit.skills.push(moves::reverse());
        }
        UnfishKind::Doppleganger => {
            kit.add(Trait::Envy);
            kit.skills.push(moves::wear());
        }
        UnfishKind::Phantom => {
            kit.add(Trait::HalfThere);
            kit.skills.push(moves::cross_over());
        }
        UnfishKind::Blinker => kit.skills.push(moves::blink()),
        UnfishKind::Ball => kit.skills.push(moves::roll()),
        UnfishKind::Skull => {
            kit.add(Trait::Telepathy);
            kit.skills.push(moves::mind_spike());
            kit.skills.push(moves::dominate());
        }
        UnfishKind::Worm => kit.add(Trait::Segments),
        UnfishKind::Absence => {
            kit.add(Trait::Absent);
            kit.skills.push(moves::erase());
        }
        UnfishKind::Leech => kit.skills.push(moves::cling()),
        UnfishKind::Bones => kit.add(Trait::Scatter),
        UnfishKind::Signal => kit.skills.push(moves::static_burst()),
        UnfishKind::Ouroboros => kit.add(Trait::TheLap),
        UnfishKind::Negative => kit.skills.push(moves::touch()),
        UnfishKind::Forgetting => {
            kit.add(Trait::HalfRemembered);
            kit.skills.push(moves::amnesia());
        }
        UnfishKind::Verso => kit.skills.push(moves::turn()),
        UnfishKind::Fault => kit.add(Trait::Lag),
        UnfishKind::Anagram => kit.skills.push(moves::shuffle()),
        UnfishKind::Reflection => kit.add(Trait::Reflection),
        UnfishKind::Molt => kit.skills.push(moves::shed()),
        UnfishKind::Face => kit.add(Trait::Stare),
        UnfishKind::Graeae => kit.add(Trait::Graeae),
        UnfishKind::Still => kit.add(Trait::Unwatched),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fishes::unfish::SPAWNABLE_UNFISH;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    #[test]
    fn every_unfish_kind_changes_the_kit() {
        let mut rng = SmallRng::seed_from_u64(5);
        for &kind in SPAWNABLE_UNFISH {
            let fish = Fish::new_unfish(kind, "Probe".into(), 0.0, 0.0, &mut rng);
            let mut kit = Kit::of(&fish);
            kit.traits.clear();
            kit.skills.clear();
            let before = kit.clone();
            wear(&mut kit, kind);
            assert_ne!(before, kit, "{kind:?} does nothing in the Deep");
        }
    }

    #[test]
    fn an_unfish_wears_its_own_kind() {
        let mut rng = SmallRng::seed_from_u64(6);
        let skull = Fish::new_unfish(UnfishKind::Skull, "Skull".into(), 0.0, 0.0, &mut rng);
        assert_eq!(kinds(&skull), vec![UnfishKind::Skull]);
        assert!(Kit::of(&skull).has(Trait::Telepathy));
    }
}
