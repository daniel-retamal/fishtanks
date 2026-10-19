use super::*;
use crate::app::Launch;
use crate::dungeon::boon::Charm;
use crate::dungeon::treasure::{INK_CASTS, SHELL_CASTS, Salvage};
use crate::economy::Rarity;
use crate::fishes::mutations::Mutation;
use crate::fishes::parts::Part;
use crate::fishes::species::FishSpecies;
use crate::loot::LootKind;
use crate::restore::Restorable;
use crate::tank::{Tank, TankKind};
use crate::ui::consume_picker::ConsumePickerSource;

const ROLLS: usize = 400;

fn holding(treasure: Treasure, count: u32) -> App {
    let mut app = App::launch(Launch::Debug);
    app.stock_up(
        StockItem::Consumable(ConsumableKind::Treasure(treasure)),
        count,
    );
    app
}

fn held(app: &App, treasure: Treasure) -> u32 {
    app.inventory
        .get(&StockItem::Consumable(ConsumableKind::Treasure(treasure)))
        .copied()
        .unwrap_or(0)
}

fn spend(app: &mut App, treasure: Treasure) -> bool {
    app.try_consume_kind(
        ConsumableKind::Treasure(treasure),
        ConsumePickerSource::FromInventory,
    )
}

#[test]
fn pincoyas_shell_makes_the_next_ten_casts_bite_rare_or_better() {
    let mut app = holding(Treasure::PincoyasShell, 1);
    assert!(spend(&mut app, Treasure::PincoyasShell));
    assert_eq!(held(&app, Treasure::PincoyasShell), 0);
    let mut rng = rand::rng();
    for _ in 0..ROLLS {
        let loot = app.roll_catch(0, &mut rng);
        assert_ne!(loot.rarity(), Rarity::Common);
    }
    for _ in 0..SHELL_CASTS {
        app.spend_a_cast(crate::consumable::Caster::Angler);
    }
    let common = (0..ROLLS).any(|_| app.roll_catch(0, &mut rng).rarity() == Rarity::Common);
    assert!(common, "ten casts later the shell is spent");
}

#[test]
fn kraken_ink_lands_a_legendary_in_any_tank() {
    let mut app = holding(Treasure::KrakenInk, 1);
    assert!(spend(&mut app, Treasure::KrakenInk));
    let mut rng = rand::rng();
    for _ in 0..ROLLS {
        match app.roll_catch(0, &mut rng) {
            LootKind::Fish(species) => assert_eq!(species.config().rarity, Rarity::Legendary),
            _ => panic!("the ink bit something that is not a fish"),
        }
    }
    for _ in 0..INK_CASTS {
        app.spend_a_cast(crate::consumable::Caster::Angler);
    }
    assert!(app.catch_charm().is_none());
}

#[test]
fn a_charm_is_never_lent_to_a_rig() {
    let mut app = holding(Treasure::KrakenInk, 1);
    spend(&mut app, Treasure::KrakenInk);
    app.spend_a_cast(crate::consumable::Caster::Rig);
    assert!(app.catch_charm().is_some(), "only the angler spends it");
}

#[test]
fn a_vial_mutates_the_fish_you_choose_through_the_one_entry() {
    let vial = Treasure::Vial(Mutation::NightOwl);
    let mut app = holding(vial, 1);
    let at = app.tanks[0]
        .fish
        .iter()
        .position(|fish| ConsumeTarget::Treasure(vial).accepts(fish))
        .expect("a fish that can take it");
    let name = app.tanks[0].fish[at].name.clone();
    let before = app.tanks[0].fish[at].mutation_count();
    assert!(app.give_treasure(vial, 0, at, &name));
    assert_eq!(app.tanks[0].fish[at].mutation_count(), before + 1);
}

#[test]
fn shears_only_split_a_double() {
    let shears = Treasure::Vial(Mutation::Cytokinesis);
    let app = holding(shears, 1);
    assert!(!ConsumeTarget::Treasure(shears).accepts(&app.tanks[0].fish[0]));
    assert!(
        ConsumeTarget::Treasure(Treasure::Vial(Mutation::Telophase)).accepts(&app.tanks[0].fish[0])
    );
}

#[test]
fn a_salvaged_chip_seals_onto_a_botfish_and_hands_back_its_old_parts() {
    let chip = Treasure::Chip(Salvage::Feeder);
    let mut app = holding(chip, 1);
    let mut rng = rand::rng();
    app.tanks[0].spawn_fish(FishSpecies::Botfish, "Wall-e".to_string(), &mut rng);
    let at = app.tanks[0].fish.len() - 1;
    app.tanks[0].fish[at]
        .script_mut()
        .expect("a botfish")
        .install(Part::InverterCoil);
    assert!(app.give_treasure(chip, 0, at, "Wall-e"));
    let bot = app.tanks[0].fish[at].script().expect("still a botfish");
    assert!(bot.chip().is_some_and(|chip| chip.is_sealed()));
    let coil = StockItem::Consumable(ConsumableKind::Part(Part::InverterCoil));
    assert_eq!(app.inventory.get(&coil).copied(), Some(1));
}

#[test]
fn a_charm_is_worn_for_life_and_only_one() {
    let lantern = Treasure::Charm(Charm::CaleuchesLantern);
    let mut app = holding(lantern, 2);
    let name = app.tanks[0].fish[0].name.clone();
    assert!(app.give_treasure(lantern, 0, 0, &name));
    assert!(!ConsumeTarget::Treasure(lantern).accepts(&app.tanks[0].fish[0]));
    assert!(!app.give_treasure(Treasure::Charm(Charm::Halo), 0, 0, &name));
    app.tanks[0].fish[0].restore();
    let life = app.tanks[0].fish[0].deep.as_ref().expect("a deep life");
    assert_eq!(
        life.charm,
        Some(Charm::CaleuchesLantern),
        "restore keeps the charm"
    );
}

#[test]
fn a_feather_makes_a_fish_holy_enough_for_heaven() {
    let mut app = holding(Treasure::Feather, 1);
    let name = app.tanks[0].fish[0].name.clone();
    let heaven = Tank::new("Heaven".to_string(), TankKind::Heaven, &[]);
    assert!(!heaven.welcomes(&app.tanks[0].fish[0]));
    assert!(app.give_treasure(Treasure::Feather, 0, 0, &name));
    assert!(app.tanks[0].fish[0].is_holy());
    assert!(heaven.welcomes(&app.tanks[0].fish[0]));
}

#[test]
fn a_radiant_core_makes_any_tank_radioactive_for_an_hour() {
    let mut app = holding(Treasure::RadiantCore, 1);
    assert!(!app.tanks[0].is_radioactive());
    assert!(spend(&mut app, Treasure::RadiantCore));
    assert!(app.tanks[0].is_radioactive());
    assert!(app.tanks[0].rads() > 0);
    app.tanks[0].radiance = 0.0;
    assert!(!app.tanks[0].is_radioactive());
}

#[test]
fn mammons_coin_binds_a_soul_to_the_first_hell_wall_for_ever() {
    let mut app = holding(Treasure::MammonsCoin, 1);
    let hell = app.found_tank(Tank::new("Hell".to_string(), TankKind::Hell, &[]));
    let luck = app.devils_luck_at(hell);
    assert!(spend(&mut app, Treasure::MammonsCoin));
    assert_eq!(app.devils_luck_at(hell), luck + 1);
    let second = app.found_tank(Tank::new("Hell II".to_string(), TankKind::Hell, &[]));
    assert_eq!(app.devils_luck_at(second), 0, "one wall holds it");
}

#[test]
fn three_shards_make_a_wish() {
    let mut app = holding(Treasure::WishShard, 2);
    assert!(
        !spend(&mut app, Treasure::WishShard),
        "two shards are not a wish"
    );
    app.stock_up(
        StockItem::Consumable(ConsumableKind::Treasure(Treasure::WishShard)),
        1,
    );
    assert!(spend(&mut app, Treasure::WishShard));
    assert_eq!(held(&app, Treasure::WishShard), 0);
    assert!(matches!(app.void_ritual, VoidRitualState::Wish { .. }));
}

#[test]
fn phoenix_roe_raises_a_marked_fish_from_the_graveyard() {
    let mut app = holding(Treasure::PhoenixRoe, 1);
    let mut fish = app.tanks[0].take_fish(0);
    fish.devil_marked = true;
    let name = fish.name.clone();
    app.bury(fish, crate::fishes::graveyard::Fate::Killed);
    let dead = app.the_dead_for(Treasure::PhoenixRoe);
    let entry = dead
        .iter()
        .find(|e| e.fish_name == name)
        .expect("it lies there");
    assert!(app.give_treasure(Treasure::PhoenixRoe, entry.tank_idx, entry.fish_idx, &name));
    assert!(
        app.tanks
            .iter()
            .any(|t| t.fish.iter().any(|f| f.name == name))
    );
}

#[test]
fn a_tuned_cowbell_calls_a_cow_of_its_colour() {
    let bell = Treasure::Cowbell(crate::entities::cow::CowVariant::Blue);
    let mut app = holding(bell, 1);
    let ufos = app.tanks[0].ufos.len();
    assert!(spend(&mut app, bell));
    assert_eq!(
        app.tanks[0].ufos.len(),
        ufos + 1,
        "the mothership is on its way"
    );
}

#[test]
fn the_marked_card_and_a_deep_toy_part_are_not_used_from_the_inventory() {
    let mut app = holding(Treasure::MarkedCard, 1);
    assert!(!spend(&mut app, Treasure::MarkedCard));
    assert_eq!(held(&app, Treasure::MarkedCard), 1);
}
