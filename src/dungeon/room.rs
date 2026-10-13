use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::economy::{Rarity, grouped};
use crate::entities::food::FOOD_BUY_PRICE;
use crate::fishes::fish::Fish;
use crate::fishes::mutations::{Mutation, apply_mutation};
use crate::fishes::revert::revert_latest;
use crate::fishes::species::{FishSpecies, SizeCategory};
use crate::loot::{ConsumableKind, LootPool, StockItem};

use super::Wreck;
use super::battle::{
    FLEE_BASE_PERCENT, FLEE_MAX_PERCENT, FLEE_MIN_PERCENT, FLEE_PER_SPEED, Stakes,
};
use super::beat::{LogLine, Tone};
use super::bestiary::SpriteRow;
use super::boon::{Boon, Curse};
use super::event::{self, Check, EventSpec, FISH, Need, Outcome};
use super::expedition::{Expedition, Foe, Ground, Next, Wound};
use super::fighter::Fighter;
use super::floor::{self, CHEST_SHARES, FloorSpec, Light};
use super::haul::{self, Haul, counted};
use super::items::{HEAL_PER_PELLET, Item, MEAL_PELLETS, PACK_STACKS, Pack};
use super::kit::Kit;
use super::life::DeepLife;
use super::map::{Room, RoomKind};
use super::room_art;
use super::specials::wearable_mutations;
use super::traits::Trait;

pub const CLEAN_SHARES: u64 = 2;
pub const BRING_BACK_SHARES: u64 = 6;
pub const HIRE_COMMON_SHARES: u64 = 16;
pub const HIRE_RARE_SHARES: u64 = 32;
pub const DOSE_SHARES: u64 = 4;
pub const UNDO_SHARES: u64 = 3;
pub const SHRINE_GOLD_SHARES: u64 = 2;
pub const SCHOOL_COMMONS: usize = 3;
pub const SCHOOL_RARES: usize = 1;
pub const DOSES_OFFERED: usize = 3;
pub const BLESSINGS_OFFERED: usize = 2;
pub const MIMIC_PERCENT: i32 = 20;
pub const LURKER_PERCENT: i32 = 20;
pub const DARK_AMBUSH_PERCENT: u32 = 25;
pub const REST_PERCENT: u32 = 25;
pub const FOOD_BUNDLE: u32 = 30;
pub const GUARDS: usize = 2;
pub const CLONE_SUFFIX: &str = "'s Clone";
const PERCENT: u32 = 100;
const PERCENT_SIGNED: i32 = 100;
const LUCK_PIVOT: i32 = 50;
const MIMIC_SALT: u64 = 0x31_4D1C;
const LURKER_SALT: u64 = 0x1A_2CE2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Blessing {
    Revive,
    FeedBurst,
    Cash,
    Gift,
    Clone,
}

impl Blessing {
    pub const ALL: [Blessing; 5] = [
        Blessing::Revive,
        Blessing::FeedBurst,
        Blessing::Cash,
        Blessing::Gift,
        Blessing::Clone,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Blessing::Revive => "Revive",
            Blessing::FeedBurst => "Feed Burst",
            Blessing::Cash => "Cash",
            Blessing::Gift => "Gift",
            Blessing::Clone => "Clone",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Act {
    Fight,
    Flee,
    Leave,
    MoveOn,
    Open,
    OpenWithBait,
    LookFirst,
    Break,
    Hire(u8),
    Bless(u8),
    Sign(Boon),
    Dose,
    Choose,
    Pick(u8),
    Undo,
    Revert(u8),
    Lift(Curse),
    Back,
    Feed,
    Rest,
    Clean,
    BringBack(u8),
    Buy(u8),
    Kick,
    Choice(u8),
    Up,
    Down,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage {
    #[default]
    Open,
    Looked(bool),
    Sprung,
    Doses(Vec<String>),
    Undoing,
    Spoils,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Offer {
    Hire(FishSpecies),
    Blessing(Blessing),
    Stock { item: Item, count: u32, price: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Visit {
    pub row: usize,
    pub index: u8,
    pub kind: RoomKind,
    pub seed: u64,
    #[serde(default)]
    pub foes: Vec<Foe>,
    #[serde(default)]
    pub stage: Stage,
    #[serde(default)]
    pub said: Vec<LogLine>,
    #[serde(default)]
    pub spent: Vec<Act>,
    #[serde(default)]
    pub rolls: u32,
    #[serde(default)]
    pub offers: Vec<Offer>,
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub ambush: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub label: String,
    pub note: String,
    pub note_tone: Tone,
    pub tag: Option<String>,
    pub usable: bool,
    pub act: Act,
    pub about: Vec<LogLine>,
}

impl Row {
    pub fn new(label: impl Into<String>, act: Act) -> Row {
        Row {
            label: label.into(),
            note: String::new(),
            note_tone: Tone::Quiet,
            tag: None,
            usable: true,
            act,
            about: Vec::new(),
        }
    }

    pub fn about(mut self, about: Vec<LogLine>) -> Row {
        self.about = about;
        self
    }

    pub fn note(mut self, note: impl Into<String>, tone: Tone) -> Row {
        self.note = note.into();
        self.note_tone = tone;
        self
    }

    fn tag(mut self, tag: impl Into<String>) -> Row {
        self.tag = Some(tag.into());
        self
    }

    pub fn usable(mut self, usable: bool) -> Row {
        self.usable = usable;
        self
    }

    fn price(self, price: u64, haul: &Haul) -> Row {
        let affordable = haul.gold >= price;
        self.note(format!("${}", grouped(u128::from(price))), Tone::Gold)
            .usable(affordable)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Figure {
    pub name: String,
    pub hp: u32,
    pub max_hp: u32,
    pub mp: u32,
    pub max_mp: u32,
    pub sprite: &'static [SpriteRow],
}

impl Figure {
    pub fn of(fighter: &Fighter) -> Figure {
        Figure {
            name: fighter.name.clone(),
            hp: fighter.hp,
            max_hp: fighter.max_hp,
            mp: fighter.mp,
            max_mp: fighter.max_mp,
            sprite: fighter.spec().map_or(&[], |spec| spec.sprite),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Select,
    Fight,
    Hire,
    Pray,
    Sign,
    Pay,
    Buy,
    Ride,
    Pick,
    Pack,
}

#[derive(Clone)]
pub enum Scene {
    Rows(&'static [SpriteRow]),
    Fish(Box<Fish>),
    Item(Item),
    Vending(Vec<(Item, u64)>),
    Lift { from: u32, to: u32 },
}

#[derive(Clone)]
pub struct Panel {
    pub title: String,
    pub art: Scene,
    pub figures: Vec<Figure>,
    pub draw_figures: bool,
    pub text: Vec<LogLine>,
    pub rows: Vec<Row>,
    pub which_fish: bool,
    pub verb: Verb,
    pub counts: bool,
    pub tab: Option<Tab>,
    pub back: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    ReadBoth,
    Pack,
}

fn line(text: impl Into<String>) -> LogLine {
    LogLine::new(text, Tone::Plain)
}

fn named(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => format!("A {one}"),
        [first, second] => format!("A {first} and a {second}"),
        [rest @ .., last] => {
            let list: Vec<String> = rest.iter().map(|n| format!("a {n}")).collect();
            let list = list.join(", ");
            let mut list = list;
            list.replace_range(..1, "A");
            format!("{list} and a {last}")
        }
    }
}

pub fn spoils_lines(found: &Haul) -> Vec<LogLine> {
    let mut lines = Vec::new();
    if found.gold > 0 {
        lines.push(LogLine::new(
            format!("${} goes into the haul.", grouped(u128::from(found.gold))),
            Tone::Gold,
        ));
    }
    for treasure in &found.treasures {
        let what = match treasure.count {
            1 => treasure.stock.display_name().to_string(),
            n => format!("{} ×{n}", treasure.stock.display_name()),
        };
        lines.push(LogLine::new(format!("Found: {what}."), Tone::Gold));
    }
    if lines.is_empty() {
        lines.push(LogLine::new("Nothing else is here.", Tone::Quiet));
    }
    lines
}

fn mimic_chance(kits: &[Kit], base: i32) -> i32 {
    let luck = event::luck(kits);
    (base * (PERCENT_SIGNED + LUCK_PIVOT - luck) / PERCENT_SIGNED).max(0)
}

pub fn hidden(room: Room, kits: &[Kit]) -> bool {
    let (salt, base) = match room.kind {
        RoomKind::Chest => (MIMIC_SALT, MIMIC_PERCENT),
        RoomKind::BellJar => (LURKER_SALT, LURKER_PERCENT),
        _ => return false,
    };
    let mut rng = SmallRng::seed_from_u64(room.seed ^ salt);
    rng.random_range(0..PERCENT_SIGNED) < mimic_chance(kits, base)
}

fn fighters(foes: &[Foe], expedition: &Expedition) -> Vec<Fighter> {
    foes.iter()
        .filter_map(|foe| foe.fighter(expedition.floor, expedition.scale()))
        .collect()
}

fn foes_of(fighters: &[Fighter]) -> Vec<Foe> {
    fighters.iter().filter_map(Foe::of).collect()
}

fn hire_shares(rarity: Rarity) -> u64 {
    match rarity {
        Rarity::Common => HIRE_COMMON_SHARES,
        Rarity::Rare | Rarity::Legendary => HIRE_RARE_SHARES,
    }
}

fn school(depth: u32, rng: &mut SmallRng) -> Vec<Offer> {
    let pool = LootPool::default_pool().with_native(FloorSpec::of(depth).tank);
    let of = |rarity: Rarity| -> Vec<FishSpecies> {
        pool.species()
            .into_iter()
            .filter(|s| s.config().rarity == rarity && s.config().buyable)
            .collect()
    };
    let mut offers = Vec::new();
    for (rarity, count) in [
        (Rarity::Common, SCHOOL_COMMONS),
        (Rarity::Rare, SCHOOL_RARES),
    ] {
        let mut shelf = of(rarity);
        for _ in 0..count {
            if shelf.is_empty() {
                break;
            }
            let species = shelf.remove(rng.random_range(0..shelf.len()));
            offers.push(Offer::Hire(species));
        }
    }
    offers
}

fn vending() -> Vec<Offer> {
    let mut offers = vec![Offer::Stock {
        item: Item::Food,
        count: FOOD_BUNDLE / MEAL_PELLETS,
        price: u64::from(FOOD_BUNDLE * FOOD_BUY_PRICE),
    }];
    for kind in [ConsumableKind::Coffee, ConsumableKind::Bait] {
        offers.push(Offer::Stock {
            item: Item::Stock(StockItem::Consumable(kind)),
            count: 1,
            price: u64::from(kind.buy_price()),
        });
    }
    offers
}

fn blessings(expedition: &Expedition, rng: &mut SmallRng) -> Vec<Offer> {
    let mut table: Vec<Blessing> = Blessing::ALL
        .into_iter()
        .filter(|&b| b != Blessing::Revive)
        .collect();
    let mut offers = Vec::new();
    if !fallen(expedition).is_empty() {
        offers.push(Offer::Blessing(Blessing::Revive));
    }
    while offers.len() < BLESSINGS_OFFERED && !table.is_empty() {
        let blessing = table.remove(rng.random_range(0..table.len()));
        offers.push(Offer::Blessing(blessing));
    }
    offers
}

pub fn fallen(expedition: &Expedition) -> Vec<String> {
    let mut names: Vec<String> = (0..expedition.party.len())
        .filter(|&member| expedition.wound(member).belly_up)
        .map(|member| expedition.party[member].name.clone())
        .collect();
    names.extend(expedition.fallen.iter().cloned());
    names
}

fn guards(depth: u32, scale: super::fighter::Scale, rng: &mut SmallRng) -> Vec<Foe> {
    let one = floor::encounter(depth, scale, &[], rng);
    let Some(guard) = one.first().and_then(Foe::of) else {
        return Vec::new();
    };
    vec![guard; GUARDS]
}

fn flee_chance(expedition: &Expedition, foes: &[Fighter]) -> u32 {
    let kits = expedition.kits();
    let fastest = expedition
        .standing()
        .iter()
        .map(|&m| kits[m].speed)
        .max()
        .unwrap_or(0) as i32;
    let theirs = foes.iter().map(|f| f.speed).max().unwrap_or(0) as i32;
    (FLEE_BASE_PERCENT + FLEE_PER_SPEED * (fastest - theirs))
        .clamp(FLEE_MIN_PERCENT, FLEE_MAX_PERCENT) as u32
}

fn always_flees(kits: &[Kit]) -> bool {
    kits.iter()
        .any(|kit| kit.traits.contains(&Trait::Walks) || kit.traits.contains(&Trait::Hover))
}

impl Visit {
    pub fn enter(
        expedition: &Expedition,
        row: usize,
        index: u8,
        room: Room,
        ground: &Ground,
    ) -> Visit {
        let mut rng = SmallRng::seed_from_u64(room.seed);
        let depth = expedition.floor;
        let scale = expedition.scale();
        let mut visit = Visit {
            row,
            index,
            kind: room.kind,
            seed: room.seed,
            foes: Vec::new(),
            stage: Stage::Open,
            said: Vec::new(),
            spent: Vec::new(),
            rolls: 0,
            offers: Vec::new(),
            event: None,
            ambush: false,
        };
        match room.kind {
            RoomKind::Encounter => {
                visit.foes = foes_of(&floor::encounter(depth, scale, &ground.haunts, &mut rng));
                let dark = FloorSpec::of(depth).light == Light::Dark;
                let lit = expedition.carries(Trait::Lantern).is_some();
                visit.ambush = dark && !lit && rng.random_range(0..PERCENT) < DARK_AMBUSH_PERCENT;
            }
            RoomKind::Den => visit.foes = foes_of(&floor::den(depth, scale, &mut rng)),
            RoomKind::Boss => visit.foes = foes_of(&floor::lair(depth, scale, expedition.dive)),
            RoomKind::Wreck => visit.foes = guards(depth, scale, &mut rng),
            RoomKind::School => visit.offers = school(depth, &mut rng),
            RoomKind::Shrine => visit.offers = blessings(expedition, &mut rng),
            RoomKind::Vending => visit.offers = vending(),
            RoomKind::Event => {
                let events = event::of_floor(depth);
                if !events.is_empty() {
                    visit.event = Some(events[rng.random_range(0..events.len())].name.to_string());
                }
            }
            RoomKind::Chest
            | RoomKind::BellJar
            | RoomKind::Altar
            | RoomKind::Laboratory
            | RoomKind::Vent
            | RoomKind::Cleaning
            | RoomKind::Elevator => {}
        }
        visit
    }

    pub fn elevator(row: usize, seed: u64) -> Visit {
        Visit {
            row,
            index: 0,
            kind: RoomKind::Elevator,
            seed,
            foes: Vec::new(),
            stage: Stage::Open,
            said: Vec::new(),
            spent: Vec::new(),
            rolls: 0,
            offers: Vec::new(),
            event: None,
            ambush: false,
        }
    }

    pub fn event(&self) -> Option<&'static EventSpec> {
        event::find(self.event.as_deref()?)
    }

    fn rng(&self) -> SmallRng {
        SmallRng::seed_from_u64(
            self.seed ^ u64::from(self.rolls).wrapping_mul(super::expedition::SEED_STRIDE),
        )
    }

    fn spend(&mut self, act: Act) {
        self.spent.push(act);
    }

    fn spent(&self, act: Act) -> bool {
        self.spent.contains(&act)
    }

    pub fn prize(&self, expedition: &mut Expedition, rng: &mut SmallRng) -> Haul {
        let mut found = Haul::default();
        match (self.kind, &self.stage) {
            (RoomKind::Chest, Stage::Sprung) => {
                let tier = haul::richer(expedition.chest_tier());
                found = chest(expedition, tier, rng);
            }
            (RoomKind::Wreck, _) => found.absorb(expedition.wreck_found.take().unwrap_or_default()),
            _ => {}
        }
        found
    }
}

fn chest(expedition: &mut Expedition, tier: Rarity, rng: &mut SmallRng) -> Haul {
    let mut found = Haul {
        gold: expedition.price(CHEST_SHARES),
        treasures: Vec::new(),
    };
    let twice = expedition.carries(Trait::Lucky).is_some() && !expedition.lucky_spent;
    let times = 1 + u32::from(twice);
    expedition.lucky_spent |= twice;
    found.gold *= u64::from(times);
    for _ in 0..times {
        found.add(haul::treasure(expedition.floor, tier, rng), 1);
    }
    found
}

fn packed(expedition: &Expedition, item: Item) -> Option<usize> {
    expedition
        .pack
        .iter()
        .position(|pack| pack.item == item && pack.count > 0)
}

pub fn pack_room(pack: &[Pack], item: Item) -> bool {
    pack.iter().any(|p| p.item == item) || pack.iter().filter(|p| p.count > 0).count() < PACK_STACKS
}

pub fn put_in(pack: &mut Vec<Pack>, item: Item, count: u32) -> bool {
    if let Some(stack) = pack.iter_mut().find(|p| p.item == item) {
        stack.count += count;
        return true;
    }
    pack.retain(|p| p.count > 0);
    if pack.len() >= PACK_STACKS {
        return false;
    }
    pack.push(Pack::of(item, count));
    true
}

fn meal_heal() -> u32 {
    MEAL_PELLETS * HEAL_PER_PELLET
}

fn heal(expedition: &mut Expedition, member: usize, amount: u32) {
    let kit = Kit::of(&expedition.party[member]);
    let hp = expedition.hp(member, &kit);
    let hp = (hp + amount).min(kit.vigor).max(1);
    expedition.wounds[member] = match hp >= kit.vigor {
        true => Wound::default(),
        false => Wound {
            hp: Some(hp),
            belly_up: false,
        },
    };
}

fn life(fish: &mut Fish) -> &mut DeepLife {
    fish.deep
        .get_or_insert_with(|| Box::new(DeepLife::default()))
}

impl Expedition {
    pub fn panel(&self, fish: usize, wrecks: &[Wreck]) -> Option<Panel> {
        let visit = self.visit.as_ref()?;
        let fish = fish.min(self.party.len().saturating_sub(1));
        let mut panel = Panel {
            title: visit.kind.title().to_string(),
            art: room_art::of(visit.kind, &visit.stage),
            figures: Vec::new(),
            draw_figures: false,
            text: Vec::new(),
            rows: Vec::new(),
            which_fish: false,
            verb: Verb::Select,
            counts: false,
            tab: None,
            back: false,
        };
        if visit.stage == Stage::Spoils {
            panel.text = visit.said.clone();
            panel.rows = vec![Row::new("Move on", Act::MoveOn)];
            return Some(panel);
        }
        match visit.kind {
            RoomKind::Encounter => self.encounter_panel(visit, &mut panel),
            RoomKind::Den => self.den_panel(visit, &mut panel),
            RoomKind::Boss => self.boss_panel(visit, &mut panel),
            RoomKind::Wreck => self.wreck_panel(visit, &mut panel, wrecks),
            RoomKind::Chest => self.chest_panel(visit, &mut panel),
            RoomKind::BellJar => self.jar_panel(visit, &mut panel),
            RoomKind::Event => self.event_panel(visit, &mut panel),
            RoomKind::School => self.school_panel(visit, &mut panel),
            RoomKind::Shrine => self.shrine_panel(visit, &mut panel),
            RoomKind::Altar => self.altar_panel(visit, &mut panel, fish),
            RoomKind::Laboratory => self.lab_panel(visit, &mut panel, fish),
            RoomKind::Vent => self.vent_panel(visit, &mut panel, fish),
            RoomKind::Cleaning => self.cleaning_panel(visit, &mut panel),
            RoomKind::Vending => self.vending_panel(visit, &mut panel),
            RoomKind::Elevator => self.elevator_panel(&mut panel),
        }
        Some(panel)
    }

    fn figures(&self, visit: &Visit) -> Vec<Figure> {
        fighters(&visit.foes, self).iter().map(Figure::of).collect()
    }

    fn sprung(&self, visit: &Visit, panel: &mut Panel) {
        panel.figures = self.figures(visit);
        panel.text = visit.said.clone();
        panel.rows = vec![Row::new("Fight", Act::Fight)];
        panel.verb = Verb::Fight;
    }

    fn encounter_panel(&self, visit: &Visit, panel: &mut Panel) {
        panel.figures = self.figures(visit);
        panel.draw_figures = true;
        let names: Vec<String> = panel.figures.iter().map(|f| f.name.clone()).collect();
        panel.text = vec![line(format!("{} block the way.", named(&names)))];
        if visit.ambush {
            panel
                .text
                .push(LogLine::new("They were waiting in the dark.", Tone::Hurt));
            panel.rows = vec![Row::new("Fight", Act::Fight)];
            return;
        }
        panel.rows = vec![Row::new("Fight", Act::Fight), Row::new("Flee", Act::Flee)];
    }

    fn den_panel(&self, visit: &Visit, panel: &mut Panel) {
        panel.text = vec![line(
            "Two eyes in the dark, too far apart for anything small.",
        )];
        for figure in &self.figures(visit) {
            panel.text.push(LogLine::new(String::new(), Tone::Plain));
            panel
                .text
                .push(LogLine::new(figure.name.clone(), Tone::Hurt));
            panel.text.push(LogLine::new(
                format!("Elite · {}/{}", figure.hp, figure.max_hp),
                Tone::Heal,
            ));
        }
        panel.rows = vec![
            Row::new("Fight", Act::Fight).note("a Rare treasure", Tone::Gold),
            Row::new("Leave", Act::Leave),
        ];
    }

    fn boss_panel(&self, visit: &Visit, panel: &mut Panel) {
        panel.figures = self.figures(visit);
        panel.draw_figures = true;
        let name = panel
            .figures
            .first()
            .map_or(String::new(), |f| f.name.clone());
        panel.title = name.clone();
        panel.text = vec![line(format!("{name} guards the elevator."))];
        panel.rows = vec![Row::new("Fight", Act::Fight)];
        panel.verb = Verb::Fight;
    }

    fn wreck_panel(&self, visit: &Visit, panel: &mut Panel, wrecks: &[Wreck]) {
        let figures = self.figures(visit);
        let Some(wreck) = wrecks.iter().find(|wreck| wreck.floor == self.floor) else {
            panel.text = vec![line("Only the bones of a boat. Nothing is left.")];
            panel.rows = vec![Row::new("Leave it", Act::Leave)];
            return;
        };
        let guards: Vec<String> = figures.iter().map(|f| f.name.clone()).collect();
        let guarded = match guards.as_slice() {
            [one, two] if one == two => format!("Guarded by two {one}s"),
            _ => format!("Guarded by {}", named(&guards).to_lowercase()),
        };
        panel.text = vec![
            line(format!("Your party fell here on {}.", wreck.day)),
            line(String::new()),
            LogLine::new(format!("Haul {}", wreck.haul.line()), Tone::Gold),
            LogLine::new(guarded, Tone::Hurt),
        ];
        panel.rows = vec![
            Row::new("Fight for it", Act::Fight),
            Row::new("Leave it", Act::Leave),
        ];
    }

    fn chest_panel(&self, visit: &Visit, panel: &mut Panel) {
        if visit.stage == Stage::Sprung {
            return self.sprung(visit, panel);
        }
        let mut text = vec![line("A chest lies half in the sand, its lid shut tight.")];
        if let Stage::Looked(mimic) = visit.stage {
            let seer = self
                .carries(Trait::LooksUp)
                .map_or(String::new(), |m| self.party[m].name.clone());
            text.push(line(String::new()));
            text.push(match mimic {
                true => LogLine::new(
                    format!("{seer} looks through the lid: it has teeth."),
                    Tone::Hurt,
                ),
                false => LogLine::new(
                    format!("{seer} looks through the lid: it is a chest."),
                    Tone::Heal,
                ),
            });
        }
        panel.text = text;
        panel.rows.push(Row::new("Open it", Act::Open));
        if packed(self, Item::Stock(StockItem::BAIT)).is_some() {
            panel
                .rows
                .push(Row::new("Open it with Bait", Act::OpenWithBait).tag("Bait"));
        }
        if let (Some(seer), Stage::Open) = (self.carries(Trait::LooksUp), &visit.stage) {
            let species = self.party[seer].species.display_name();
            panel
                .rows
                .push(Row::new(format!("Have {species} look first"), Act::LookFirst).tag(species));
        }
        panel.rows.push(Row::new("Leave it", Act::Leave));
    }

    fn jar_panel(&self, visit: &Visit, panel: &mut Panel) {
        if visit.stage == Stage::Sprung {
            return self.sprung(visit, panel);
        }
        panel.text = vec![line(
            "A bell jar stands on the sand. Something inside presses against the glass.",
        )];
        panel.rows = vec![
            Row::new("Break it", Act::Break),
            Row::new("Leave it", Act::Leave),
        ];
    }

    fn event_panel(&self, visit: &Visit, panel: &mut Panel) {
        let Some(event) = visit.event() else {
            panel.text = vec![line("The water is still.")];
            panel.rows = vec![Row::new("Move on", Act::MoveOn)];
            return;
        };
        panel.title = event.name.to_string();
        panel.art = Scene::Rows(event.art);
        if visit.stage == Stage::Sprung {
            return self.sprung(visit, panel);
        }
        panel.text = vec![line(event.text)];
        for (index, choice) in event.choices.iter().enumerate() {
            let tag = match choice.need {
                Need::Nothing => None,
                Need::Trait(wanted) => match self.carries(wanted) {
                    Some(member) => Some(self.party[member].species.display_name().to_string()),
                    None => continue,
                },
                Need::Item(item) => match packed(self, item) {
                    Some(_) => Some(item.name().to_string()),
                    None => continue,
                },
            };
            let mut row = Row::new(choice.label, Act::Choice(index as u8));
            if let Some(tag) = tag {
                row = row.tag(tag);
            }
            panel.rows.push(row);
        }
    }

    fn school_panel(&self, visit: &Visit, panel: &mut Panel) {
        panel.text = vec![line(
            "A school that is going your way, for a price. Whoever is alive at the elevator stays yours.",
        )];
        panel.verb = Verb::Hire;
        for (index, offer) in visit.offers.iter().enumerate() {
            let Offer::Hire(species) = offer else {
                continue;
            };
            let act = Act::Hire(index as u8);
            let price = self.price(hire_shares(species.config().rarity));
            let row = Row::new(format!("Hire {}", species.display_name()), act);
            panel.rows.push(match visit.spent(act) {
                true => row.note("hired", Tone::Quiet).usable(false),
                false => row.price(price, &self.haul),
            });
        }
        panel.rows.push(Row::new("Leave", Act::Leave));
    }

    fn shrine_panel(&self, visit: &Visit, panel: &mut Panel) {
        let fallen = fallen(self);
        panel.text = vec![line(match fallen.first() {
            Some(name) => format!("{name} fell on this dive."),
            None => "Candles burn in a shell. Two blessings wait to be asked for.".to_string(),
        })];
        panel.verb = Verb::Pray;
        for (index, offer) in visit.offers.iter().enumerate() {
            let Offer::Blessing(blessing) = offer else {
                continue;
            };
            let note = match blessing {
                Blessing::Revive => {
                    format!("{} comes back", fallen.first().cloned().unwrap_or_default())
                }
                Blessing::FeedBurst => format!("every fish +{} HP", meal_heal()),
                Blessing::Cash => {
                    format!("${}", grouped(u128::from(self.price(SHRINE_GOLD_SHARES))))
                }
                Blessing::Gift => "a Rare treasure".to_string(),
                Blessing::Clone => format!("a second {}", self.party[self.lead()].name),
            };
            panel
                .rows
                .push(Row::new(blessing.name(), Act::Bless(index as u8)).note(note, Tone::Quiet));
        }
        panel.rows.push(Row::new("Leave", Act::Leave));
    }

    fn altar_panel(&self, _visit: &Visit, panel: &mut Panel, fish: usize) {
        panel.which_fish = true;
        panel.tab = Some(Tab::ReadBoth);
        panel.verb = Verb::Sign;
        let name = &self.party[fish].name;
        panel.text = vec![
            line("Each Boon is written on top. The Curse it comes with is written underneath."),
            line(String::new()),
            LogLine::new(format!("Signing: {name}"), Tone::Status),
        ];
        let worn = self.party[fish]
            .deep
            .as_ref()
            .map_or(Vec::new(), |life| life.boons.clone());
        for boon in Boon::ALL {
            let about = vec![
                LogLine::new(
                    format!("{}: {}", boon.name(), boon.description()),
                    Tone::Heal,
                ),
                LogLine::new(
                    format!("{}: {}", boon.curse().name(), boon.curse().description()),
                    Tone::Hurt,
                ),
            ];
            let row = Row::new(boon.name(), Act::Sign(boon))
                .note(boon.curse().name(), Tone::Hurt)
                .about(about);
            panel.rows.push(row.usable(!worn.contains(&boon)));
        }
        panel.rows.push(Row::new("Walk away", Act::Leave));
    }

    fn lab_panel(&self, visit: &Visit, panel: &mut Panel, fish: usize) {
        panel.which_fish = true;
        let subject = &self.party[fish];
        let name = &subject.name;
        match &visit.stage {
            Stage::Doses(tokens) => {
                panel.text = vec![
                    line("Three doses on the tray, each one labelled."),
                    line(String::new()),
                    LogLine::new(format!("For: {name}"), Tone::Status),
                ];
                for (index, token) in tokens.iter().enumerate() {
                    let Some(mutation) = Mutation::parse(token) else {
                        continue;
                    };
                    let kiss = super::tables::mutations::row(mutation);
                    let about = vec![
                        LogLine::new(kiss.gift, Tone::Heal),
                        LogLine::new(kiss.quirk, Tone::Hurt),
                    ];
                    panel.tab = Some(Tab::ReadBoth);
                    panel.rows.push(
                        Row::new(token.clone(), Act::Pick(index as u8))
                            .price(self.price(DOSE_SHARES), &self.haul)
                            .about(about),
                    );
                }
                panel.rows.push(Row::new("Back", Act::Back));
            }
            Stage::Undoing => {
                panel.text = vec![
                    line("A centrifuge spins one change back out."),
                    line(String::new()),
                    LogLine::new(format!("For: {name}"), Tone::Status),
                ];
                for (index, mutation) in undoable(subject).iter().enumerate() {
                    panel
                        .rows
                        .push(Row::new(mutation.token(), Act::Revert(index as u8)));
                }
                for curse in subject
                    .deep
                    .as_ref()
                    .map_or(Vec::new(), |life| life.curses.clone())
                {
                    panel
                        .rows
                        .push(Row::new(curse.name(), Act::Lift(curse)).note("curse", Tone::Hurt));
                }
                panel.rows.push(Row::new("Back", Act::Back));
            }
            _ => {
                panel.text = vec![
                    line("Glass tubes with things in them, and a tray of doses."),
                    line(String::new()),
                    LogLine::new(format!("For: {name}"), Tone::Status),
                ];
                let wearable = !wearable_mutations(subject).is_empty();
                let undo = !undoable(subject).is_empty()
                    || subject
                        .deep
                        .as_ref()
                        .is_some_and(|life| !life.curses.is_empty());
                panel.rows.push(match visit.spent(Act::Dose) {
                    true => Row::new("Free dose", Act::Dose)
                        .note("taken", Tone::Quiet)
                        .usable(false),
                    false => Row::new("Free dose", Act::Dose)
                        .note("???", Tone::Quiet)
                        .usable(wearable),
                });
                let choose = Row::new("Choose one of three", Act::Choose)
                    .price(self.price(DOSE_SHARES), &self.haul);
                panel
                    .rows
                    .push(choose.clone().usable(choose.usable && wearable));
                let undo_row = Row::new("Undo one mutation", Act::Undo)
                    .price(self.price(UNDO_SHARES), &self.haul);
                panel
                    .rows
                    .push(undo_row.clone().usable(undo_row.usable && undo));
                panel.rows.push(Row::new("Leave", Act::Leave));
            }
        }
    }

    fn vent_panel(&self, visit: &Visit, panel: &mut Panel, fish: usize) {
        panel.which_fish = true;
        let name = &self.party[fish].name;
        let meals = packed(self, Item::Food).map_or(0, |at| self.pack[at].count);
        panel.text = vec![
            line("Warm water. The party can rest here."),
            line(String::new()),
            LogLine::new(
                format!("Feeding: {name} · {} packed", counted(meals, haul::MEAL)),
                Tone::Status,
            ),
        ];
        let feed = Row::new("Feed", Act::Feed)
            .note(format!("+{HEAL_PER_PELLET} HP a pellet"), Tone::Quiet);
        panel.rows.push(feed.usable(meals > 0));
        let rest =
            Row::new("Rest", Act::Rest).note(format!("every fish +{REST_PERCENT}%"), Tone::Quiet);
        panel.rows.push(rest.usable(!visit.spent(Act::Rest)));
        panel.rows.push(Row::new("Move on", Act::Leave));
    }

    fn cleaning_panel(&self, visit: &Visit, panel: &mut Panel) {
        panel.text = vec![line("Cleaner shrimp, the way they work on a real reef.")];
        panel.verb = Verb::Pay;
        let anyone_hurt = (0..self.party.len()).any(|m| {
            let kit = Kit::of(&self.party[m]);
            !self.wound(m).belly_up && self.hp(m, &kit) < kit.vigor
        });
        let clean =
            Row::new("Clean every fish", Act::Clean).price(self.price(CLEAN_SHARES), &self.haul);
        panel.rows.push(
            clean
                .clone()
                .usable(clean.usable && anyone_hurt && !visit.spent(Act::Clean)),
        );
        for (index, name) in fallen(self).iter().enumerate() {
            let act = Act::BringBack(index as u8);
            panel.rows.push(
                Row::new(format!("Bring back {name}"), act)
                    .price(self.price(BRING_BACK_SHARES), &self.haul),
            );
        }
        panel.rows.push(Row::new("Leave", Act::Leave));
    }

    fn vending_panel(&self, visit: &Visit, panel: &mut Panel) {
        panel.text = vec![line("Shop prices.")];
        panel.verb = Verb::Buy;
        panel.art = Scene::Vending(
            visit
                .offers
                .iter()
                .filter_map(|offer| match offer {
                    Offer::Stock { item, price, .. } => Some((*item, *price)),
                    _ => None,
                })
                .collect(),
        );
        for (index, offer) in visit.offers.iter().enumerate() {
            let Offer::Stock { item, count, price } = offer else {
                continue;
            };
            let label = match (item, count) {
                (Item::Food, _) => format!("{} {} ×{FOOD_BUNDLE}", slot(index), item.name()),
                (_, 1) => format!("{} {}", slot(index), item.name()),
                (_, n) => format!("{} {} ×{n}", slot(index), item.name()),
            };
            let row = Row::new(label, Act::Buy(index as u8)).price(*price, &self.haul);
            let fits = pack_room(&self.pack, *item);
            panel.rows.push(row.clone().usable(row.usable && fits));
        }
        panel
            .rows
            .push(Row::new("Kick it", Act::Kick).usable(!visit.spent(Act::Kick)));
        panel.rows.push(Row::new("Leave", Act::Leave));
    }

    fn elevator_panel(&self, panel: &mut Panel) {
        let next = self.floor + 1;
        panel.text = vec![LogLine::new(
            format!(
                "Haul {} · {}",
                self.haul.line(),
                counted(self.joined, haul::FISH)
            ),
            Tone::Gold,
        )];
        panel.verb = Verb::Ride;
        panel.art = Scene::Lift {
            from: self.floor,
            to: next,
        };
        panel.rows = vec![
            Row::new("Up", Act::Up).note("bank it all, go home", Tone::Quiet),
            Row::new("Down", Act::Down).note(
                format!("Floor {next} · {}", FloorSpec::title(next)),
                Tone::Quiet,
            ),
        ];
    }

    pub fn act(&mut self, act: Act, fish: usize, ground: &Ground) -> Next {
        let Some(mut visit) = self.visit.take() else {
            return Next::Map;
        };
        let fish = fish.min(self.party.len().saturating_sub(1));
        let rows = {
            self.visit = Some(visit.clone());
            let rows = self
                .panel(fish, &ground.wrecks)
                .map(|p| p.rows)
                .unwrap_or_default();
            self.visit = None;
            rows
        };
        let usable = rows.iter().any(|row| row.act == act && row.usable);
        if !usable {
            self.visit = Some(visit);
            return Next::Stay;
        }
        visit.rolls += 1;
        let next = self.perform(&mut visit, act, fish, ground);
        if self.visit.is_none() && matches!(next, Next::Stay | Next::Fight | Next::Raise(_)) {
            self.visit = Some(visit);
        }
        next
    }

    fn perform(&mut self, visit: &mut Visit, act: Act, fish: usize, ground: &Ground) -> Next {
        let mut rng = visit.rng();
        match act {
            Act::Leave | Act::MoveOn => {
                self.visit = Some(visit.clone());
                self.leave()
            }
            Act::Fight => {
                let stakes = match visit.kind {
                    RoomKind::Den | RoomKind::Wreck => Stakes::Den,
                    RoomKind::Boss => Stakes::Boss,
                    _ => Stakes::Encounter,
                };
                if visit.kind == RoomKind::Wreck {
                    self.wreck_found = ground.wreck(self.floor).map(|wreck| wreck.haul.clone());
                }
                let ambushed = visit.ambush;
                self.visit = Some(visit.clone());
                self.start_fight(visit.foes.clone(), stakes, ambushed, ground)
            }
            Act::Flee => {
                let foes = fighters(&visit.foes, self);
                let kits = self.kits();
                let away =
                    always_flees(&kits) || rng.random_range(0..PERCENT) < flee_chance(self, &foes);
                if away {
                    visit.stage = Stage::Spoils;
                    visit.said = vec![LogLine::new("The party slips past.", Tone::Quiet)];
                    return Next::Stay;
                }
                self.visit = Some(visit.clone());
                self.start_fight(visit.foes.clone(), Stakes::Encounter, true, ground)
            }
            Act::Open | Act::OpenWithBait => self.open_chest(visit, act, &mut rng),
            Act::LookFirst => {
                let map = self.map();
                let room = map.room(visit.row, usize::from(visit.index));
                let mimic = room.is_some_and(|room| hidden(room, &self.kits()));
                visit.stage = Stage::Looked(mimic);
                Next::Stay
            }
            Act::Break => self.break_jar(visit, &mut rng),
            Act::Choice(index) => self.choose(visit, index, &mut rng),
            Act::Hire(index) => {
                let Some(Offer::Hire(species)) = visit.offers.get(usize::from(index)).cloned()
                else {
                    return Next::Stay;
                };
                if !self
                    .haul
                    .spend(self.price(hire_shares(species.config().rarity)))
                {
                    return Next::Stay;
                }
                let fish = Fish::new(
                    species,
                    species.display_name().to_string(),
                    0.0,
                    0.0,
                    &mut rng,
                );
                self.join(fish);
                visit.spend(act);
                Next::Stay
            }
            Act::Bless(index) => {
                let Some(Offer::Blessing(blessing)) = visit.offers.get(usize::from(index)).cloned()
                else {
                    return Next::Stay;
                };
                self.bless(visit, blessing, &mut rng)
            }
            Act::Sign(boon) => {
                let name = self.party[fish].name.clone();
                let life = life(&mut self.party[fish]);
                life.boons.push(boon);
                life.curses.push(boon.curse());
                visit.stage = Stage::Spoils;
                visit.said = vec![
                    LogLine::new(format!("{name} signs for {}.", boon.name()), Tone::Status),
                    LogLine::new(boon.description(), Tone::Heal),
                    LogLine::new(format!("Underneath: {}.", boon.curse().name()), Tone::Hurt),
                    LogLine::new(boon.curse().description(), Tone::Hurt),
                ];
                Next::Stay
            }
            Act::Dose => {
                let name = self.party[fish].name.clone();
                let choices = wearable_mutations(&self.party[fish]);
                if choices.is_empty() {
                    return Next::Stay;
                }
                let pick = choices[rng.random_range(0..choices.len())];
                apply_mutation(&mut self.party[fish], pick, &mut rng);
                visit.spend(act);
                visit.said = vec![LogLine::new(
                    format!("{name} takes the dose: {}.", pick.token()),
                    Tone::Status,
                )];
                Next::Stay
            }
            Act::Choose => {
                let mut choices = wearable_mutations(&self.party[fish]);
                let mut tokens = Vec::new();
                while tokens.len() < DOSES_OFFERED && !choices.is_empty() {
                    let pick = choices.remove(rng.random_range(0..choices.len()));
                    tokens.push(pick.token().to_string());
                }
                visit.stage = Stage::Doses(tokens);
                Next::Stay
            }
            Act::Pick(index) => {
                let Stage::Doses(tokens) = &visit.stage else {
                    return Next::Stay;
                };
                let Some(mutation) = tokens
                    .get(usize::from(index))
                    .and_then(|t| Mutation::parse(t))
                else {
                    return Next::Stay;
                };
                if !self.haul.spend(self.price(DOSE_SHARES)) {
                    return Next::Stay;
                }
                let name = self.party[fish].name.clone();
                apply_mutation(&mut self.party[fish], mutation, &mut rng);
                visit.stage = Stage::Open;
                visit.said = vec![LogLine::new(
                    format!("{name} takes {}.", mutation.token()),
                    Tone::Status,
                )];
                Next::Stay
            }
            Act::Undo => {
                visit.stage = Stage::Undoing;
                Next::Stay
            }
            Act::Revert(index) => {
                let Some(mutation) = undoable(&self.party[fish]).get(usize::from(index)).copied()
                else {
                    return Next::Stay;
                };
                if !self.haul.spend(self.price(UNDO_SHARES)) {
                    return Next::Stay;
                }
                revert_latest(&mut self.party[fish], mutation);
                visit.stage = Stage::Open;
                let name = self.party[fish].name.clone();
                visit.said = vec![LogLine::new(
                    format!("{name} loses {}.", mutation.token()),
                    Tone::Status,
                )];
                Next::Stay
            }
            Act::Lift(curse) => {
                if !self.haul.spend(self.price(UNDO_SHARES)) {
                    return Next::Stay;
                }
                let name = self.party[fish].name.clone();
                life(&mut self.party[fish]).curses.retain(|c| *c != curse);
                visit.stage = Stage::Open;
                visit.said = vec![LogLine::new(
                    format!("{name} is free of {}.", curse.name()),
                    Tone::Status,
                )];
                Next::Stay
            }
            Act::Back => {
                visit.stage = Stage::Open;
                Next::Stay
            }
            Act::Feed => {
                let Some(at) = packed(self, Item::Food) else {
                    return Next::Stay;
                };
                self.pack[at].count -= 1;
                self.party[fish].eat(crate::entities::food::FOOD_WEIGHT_GAIN_G * MEAL_PELLETS);
                heal(self, fish, meal_heal());
                Next::Stay
            }
            Act::Rest => {
                for member in 0..self.party.len() {
                    let vigor = Kit::of(&self.party[member]).vigor;
                    heal(self, member, vigor * REST_PERCENT / PERCENT);
                }
                visit.spend(act);
                Next::Stay
            }
            Act::Clean => {
                if !self.haul.spend(self.price(CLEAN_SHARES)) {
                    return Next::Stay;
                }
                for member in 0..self.party.len() {
                    if !self.wound(member).belly_up {
                        self.wounds[member] = Wound::default();
                    }
                }
                visit.spend(act);
                Next::Stay
            }
            Act::BringBack(index) => {
                let Some(name) = fallen(self).get(usize::from(index)).cloned() else {
                    return Next::Stay;
                };
                if !self.haul.spend(self.price(BRING_BACK_SHARES)) {
                    return Next::Stay;
                }
                self.bring_back(&name)
            }
            Act::Buy(index) => {
                let Some(Offer::Stock { item, count, price }) =
                    visit.offers.get(usize::from(index)).cloned()
                else {
                    return Next::Stay;
                };
                if !pack_room(&self.pack, item) || !self.haul.spend(price) {
                    return Next::Stay;
                }
                put_in(&mut self.pack, item, count);
                Next::Stay
            }
            Act::Kick => {
                visit.spend(act);
                let stock: Vec<(Item, u32)> = visit
                    .offers
                    .iter()
                    .filter_map(|offer| match offer {
                        Offer::Stock { item, count, .. } => Some((*item, *count)),
                        _ => None,
                    })
                    .filter(|(item, _)| pack_room(&self.pack, *item))
                    .collect();
                let falls = !stock.is_empty() && event::lucky(&self.kits(), &mut rng);
                visit.said = match falls {
                    true => {
                        let (item, count) = stock[rng.random_range(0..stock.len())];
                        put_in(&mut self.pack, item, count);
                        vec![LogLine::new(
                            format!("A {} falls out.", item.name()),
                            Tone::Gold,
                        )]
                    }
                    false => vec![LogLine::new("Nothing falls.", Tone::Quiet)],
                };
                Next::Stay
            }
            Act::Up => {
                self.visit = Some(visit.clone());
                Next::Home
            }
            Act::Down => {
                self.descend(ground);
                Next::Map
            }
        }
    }

    pub fn bring_back(&mut self, name: &str) -> Next {
        let belly_up = self
            .party
            .iter()
            .position(|fish| fish.name == name)
            .filter(|&member| self.wound(member).belly_up);
        match belly_up {
            Some(member) => {
                self.wounds[member] = Wound::default();
                Next::Stay
            }
            None => Next::Raise(name.to_string()),
        }
    }

    fn open_chest(&mut self, visit: &mut Visit, act: Act, rng: &mut SmallRng) -> Next {
        let map = self.map();
        let room = map.room(visit.row, usize::from(visit.index));
        let mimic = room.is_some_and(|room| hidden(room, &self.kits()));
        if mimic {
            visit.stage = Stage::Sprung;
            visit.foes = vec![Foe {
                enemy: MIMIC.to_string(),
                ghost: None,
            }];
            visit.said = vec![
                LogLine::new("The lid was a lip.", Tone::Plain),
                line(String::new()),
                LogLine::new("The Mimic moves first.", Tone::Hurt),
            ];
            return Next::Stay;
        }
        let mut tier = self.chest_tier();
        if act == Act::OpenWithBait
            && let Some(at) = packed(self, Item::Stock(StockItem::BAIT))
        {
            self.pack[at].count -= 1;
            tier = haul::richer(tier);
        }
        let found = chest(self, tier, rng);
        visit.stage = Stage::Spoils;
        visit.said = spoils_lines(&found);
        self.haul.absorb(found);
        Next::Stay
    }

    fn break_jar(&mut self, visit: &mut Visit, rng: &mut SmallRng) -> Next {
        let map = self.map();
        let room = map.room(visit.row, usize::from(visit.index));
        if room.is_some_and(|room| hidden(room, &self.kits())) {
            visit.stage = Stage::Sprung;
            visit.foes = vec![Foe {
                enemy: LURKER.to_string(),
                ghost: None,
            }];
            visit.said = vec![
                LogLine::new(
                    "The glass gives. What was inside was never a fish.",
                    Tone::Plain,
                ),
                line(String::new()),
                LogLine::new("The Lurker moves first.", Tone::Hurt),
            ];
            return Next::Stay;
        }
        let pool = LootPool::default_pool().with_native(FloorSpec::of(self.floor).tank);
        let species = pool.roll_species(rng).unwrap_or(FishSpecies::Merluza);
        let fish = Fish::new(species, species.display_name().to_string(), 0.0, 0.0, rng);
        let name = self.join(fish);
        visit.stage = Stage::Spoils;
        visit.said = vec![LogLine::new(
            format!("The glass gives. {name} swims out and joins the party."),
            Tone::Heal,
        )];
        Next::Stay
    }

    fn choose(&mut self, visit: &mut Visit, index: u8, rng: &mut SmallRng) -> Next {
        let Some(event) = visit.event() else {
            return Next::Stay;
        };
        let Some(choice) = event.choices.get(usize::from(index)) else {
            return Next::Stay;
        };
        let kits = self.kits();
        let subject = match choice.check {
            Some(Check::Size) => self.smallest(),
            _ => self.lead(),
        };
        let passed = match choice.check {
            None => true,
            Some(check) => event::passes(
                check,
                &kits,
                subject,
                self.party[subject].size_category,
                rng,
            ),
        };
        let turn = if passed { choice.pass } else { choice.fail };
        let name = self.party[subject].name.clone();
        let said = LogLine::new(turn.text.replace(FISH, &name), Tone::Plain);
        let mut found = Haul::default();
        match turn.outcome {
            Outcome::Nothing => {}
            Outcome::Gold(shares) => found.gold += self.price(shares),
            Outcome::Treasure(tier) => found.add(haul::treasure(self.floor, tier, rng), 1),
            Outcome::Keep(stock) => found.add(stock, 1),
            Outcome::Omen(omen) => self.omens.push(omen),
            Outcome::Hurt(percent) => {
                let kit = Kit::of(&self.party[subject]);
                let hp = self.hp(subject, &kit);
                let left = hp.saturating_sub(kit.vigor * percent / PERCENT).max(1);
                self.wounds[subject] = Wound {
                    hp: Some(left),
                    belly_up: false,
                };
            }
            Outcome::Fight { enemy, count } => {
                visit.stage = Stage::Sprung;
                visit.foes = (0..count)
                    .map(|_| Foe {
                        enemy: enemy.to_string(),
                        ghost: None,
                    })
                    .collect();
                visit.said = vec![LogLine::new(said.text, Tone::Hurt)];
                return Next::Stay;
            }
        }
        visit.stage = Stage::Spoils;
        visit.said = vec![said];
        if !found.is_empty() {
            visit.said.extend(spoils_lines(&found));
        }
        self.haul.absorb(found);
        Next::Stay
    }

    fn smallest(&self) -> usize {
        self.standing()
            .into_iter()
            .min_by_key(|&member| {
                (
                    size_rank(self.party[member].size_category),
                    self.party[member].weight_g,
                )
            })
            .unwrap_or(0)
    }

    fn bless(&mut self, visit: &mut Visit, blessing: Blessing, rng: &mut SmallRng) -> Next {
        visit.stage = Stage::Spoils;
        visit.said = vec![LogLine::new(
            format!("The shrine grants {}.", blessing.name()),
            Tone::Status,
        )];
        match blessing {
            Blessing::Revive => {
                let Some(name) = fallen(self).first().cloned() else {
                    return Next::Stay;
                };
                visit
                    .said
                    .push(LogLine::new(format!("{name} comes back."), Tone::Heal));
                self.bring_back(&name)
            }
            Blessing::FeedBurst => {
                for member in 0..self.party.len() {
                    heal(self, member, meal_heal());
                }
                Next::Stay
            }
            Blessing::Cash => {
                let gold = self.price(SHRINE_GOLD_SHARES);
                self.haul.gold += gold;
                visit.said.extend(spoils_lines(&Haul {
                    gold,
                    treasures: Vec::new(),
                }));
                Next::Stay
            }
            Blessing::Gift => {
                let mut found = Haul::default();
                found.add(haul::treasure(self.floor, Rarity::Rare, rng), 1);
                visit.said.extend(spoils_lines(&found));
                self.haul.absorb(found);
                Next::Stay
            }
            Blessing::Clone => {
                let lead = self.lead();
                let mut twin = self.party[lead].clone();
                twin.name = format!("{}{CLONE_SUFFIX}", twin.name);
                let name = self.join(twin);
                visit
                    .said
                    .push(LogLine::new(format!("{name} joins the party."), Tone::Heal));
                Next::Stay
            }
        }
    }
}

pub const MIMIC: &str = "Mimic";
pub const LURKER: &str = "Lurker";

fn size_rank(size: SizeCategory) -> u8 {
    match size {
        SizeCategory::S => 0,
        SizeCategory::M => 1,
        SizeCategory::L => 2,
        SizeCategory::XL => 3,
    }
}

pub fn slot(index: usize) -> String {
    let rows = ['A', 'B', 'C', 'D'];
    let row = rows[(index / VEND_COLUMNS).min(rows.len() - 1)];
    format!("{row}{}", index % VEND_COLUMNS + 1)
}

pub const VEND_COLUMNS: usize = 3;

pub fn undoable(fish: &Fish) -> Vec<Mutation> {
    let Some(record) = crate::fishes::mutations::Mutatable::record(fish) else {
        return Vec::new();
    };
    let mut seen = Vec::new();
    for index in record.revertible() {
        if let Some(mutation) = Mutation::parse(&record.history[index])
            && !seen.contains(&mutation)
        {
            seen.push(mutation);
        }
    }
    seen
}

#[cfg(test)]
mod tests;
