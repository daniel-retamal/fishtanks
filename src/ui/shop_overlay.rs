use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};
use std::collections::HashMap;

use crate::colors::{DARK_GRAY, WHITE};
use crate::{
    economy::{Money, grouped},
    entities::food::FOOD_BUY_PRICE,
    fishes::{
        fish::{Fed, Fish},
        parts::PartTier,
        species::FishSpecies,
    },
    ledger::Flow,
    loot::{
        CIRCUIT_BLUEPRINT_NAME, CIRCUIT_BLUEPRINT_SELL_PRICE, ConsumableKind, JUNK_PER_JUNKFISH,
        MilkVariant, StockItem,
    },
    tank::TankKind,
    ui::{
        draw_fish_centred, fields, fish_art_height,
        hint_bar::HintBar,
        hints::{
            HINT_BACK, HINT_CANCEL, HINT_CLOSE, HINT_ENTER_ASSEMBLE, HINT_ENTER_BUY,
            HINT_ENTER_SELL, HINT_NAV, HINT_SCROLL,
        },
        layout::{Screen, Scroll, Scrollbar},
        modal::{Frame, Modal, ModalSpec},
        panels::{PanelSpec, Panels, Reach},
        table,
        text_input::{TextInput, draw_text_cursor},
    },
};

const BACKGROUND: Color = Color::Reset;
const PENGUIN_WIDTH: u16 = 7;
const PENGUIN_HEIGHT: u16 = 4;
const RIGHT_INNER_WIDTH: u16 = 34;
const MIN_BODY_WIDTH: u16 = 16;
const MAIN_LEAD_ROWS: u16 = 1;
const MAIN_ROWS: u16 = MAIN_LEAD_ROWS + Counter::ALL.len() as u16;
const COLUMN_HEADER_ROWS: u16 = 2;
const SELECTED_PREFIX: &str = "> ";
const UNSELECTED_PREFIX: &str = "  ";
const PRICE_HEADER: &str = "Price/unit";
const PRICE_GAP: u16 = 1;
const TEXT_PAD: u16 = 1;
const RULE_LINE: char = '─';
const RULE_JOINS_LEFT: char = '├';
const RULE_JOINS_RIGHT: char = '┤';

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ShopAccess {
    pub cash: Money,
    pub connected: bool,
    pub room: bool,
    pub sellable: bool,
}

pub struct Shelf {
    order: Vec<usize>,
    available: Vec<bool>,
}

impl Shelf {
    fn of<K: Ord>(available: Vec<bool>, key: impl Fn(usize) -> K) -> Self {
        let mut order: Vec<usize> = (0..available.len()).collect();
        order.sort_by_key(|&item| (!available[item], key(item)));
        Self { order, available }
    }

    pub fn first(&self) -> usize {
        self.order.first().copied().unwrap_or(0)
    }

    pub fn position(&self, item: usize) -> usize {
        self.order
            .iter()
            .position(|&shelved| shelved == item)
            .unwrap_or(0)
    }

    pub fn is_available(&self, item: usize) -> bool {
        self.available.get(item).copied().unwrap_or(false)
    }

    pub fn available_count(&self) -> usize {
        self.available.iter().filter(|&&open| open).count()
    }

    pub fn step(&self, item: usize, down: bool) -> usize {
        let at = self.position(item);
        let open = |shelved: &&usize| self.is_available(**shelved);
        let found = if down {
            self.order.iter().skip(at + 1).find(open)
        } else {
            self.order[..at].iter().rev().find(open)
        };
        found.copied().unwrap_or(item)
    }

    pub fn settle(&self, item: usize) -> usize {
        if self.is_available(item) || self.available_count() == 0 {
            item
        } else {
            self.first()
        }
    }

    fn rank(&self, item: usize) -> usize {
        let at = self.position(item);
        self.order[..=at]
            .iter()
            .filter(|&&shelved| self.is_available(shelved))
            .count()
    }

    fn rows(&self, row: impl Fn(usize, bool) -> PricedRow) -> Vec<PricedRow> {
        self.order
            .iter()
            .map(|&item| row(item, self.is_available(item)))
            .collect()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Counter {
    Buy,
    Sell,
}

impl Counter {
    pub const ALL: [Counter; 2] = [Counter::Buy, Counter::Sell];

    pub fn display_name(self) -> &'static str {
        match self {
            Counter::Buy => "Buy",
            Counter::Sell => "Sell",
        }
    }

    pub fn index(self) -> usize {
        Counter::ALL
            .iter()
            .position(|&counter| counter == self)
            .unwrap_or(0)
    }

    pub fn is_available(self, access: ShopAccess) -> bool {
        match self {
            Counter::Buy => BuyCategory::ALL.iter().any(|cat| cat.is_available(access)),
            Counter::Sell => access.sellable,
        }
    }

    pub fn shelf(access: ShopAccess) -> Shelf {
        Shelf::of(
            Counter::ALL
                .iter()
                .map(|counter| counter.is_available(access))
                .collect(),
            |item| item,
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BuyList {
    Fishes,
    Tanks,
    Bench(PartTier),
}

impl BuyList {
    fn is_open(self, access: ShopAccess) -> bool {
        match self {
            BuyList::Bench(_) => access.connected,
            BuyList::Fishes => access.room,
            BuyList::Tanks => true,
        }
    }

    pub fn shelf(self, access: ShopAccess) -> Shelf {
        let catalogue = self.catalogue();
        let open = self.is_open(access);
        let available = catalogue
            .iter()
            .map(|&(_, price)| open && affords(price, access.cash))
            .collect();
        Shelf::of(available, |item| (catalogue[item].1, catalogue[item].0))
    }

    pub fn has_anything_available(self, access: ShopAccess) -> bool {
        self.shelf(access).available_count() > 0
    }

    fn catalogue(self) -> Vec<(&'static str, u32)> {
        match self {
            BuyList::Fishes => FishSpecies::all_buyable()
                .iter()
                .map(|species| (species.display_name(), species.buy_price()))
                .collect(),
            BuyList::Tanks => TankKind::all_buyable()
                .into_iter()
                .map(|kind| (kind.display_name(), kind.buy_price()))
                .collect(),
            BuyList::Bench(_) => self
                .stock()
                .into_iter()
                .map(|kind| (kind.display_name(), kind.buy_price()))
                .collect(),
        }
    }

    fn stock(self) -> Vec<ConsumableKind> {
        let BuyList::Bench(tier) = self else {
            return Vec::new();
        };
        ConsumableKind::bench_stock()
            .into_iter()
            .filter(|kind| kind.bench_tier() == tier)
            .collect()
    }

    fn rows(self, access: ShopAccess) -> Vec<PricedRow> {
        let catalogue = self.catalogue();
        self.shelf(access).rows(|item, available| {
            let (name, price) = catalogue[item];
            PricedRow::plain(name.to_string(), format!("${price}"), available)
        })
    }

    fn height(self) -> usize {
        self.catalogue().len()
    }

    fn header(self) -> &'static str {
        match self {
            BuyList::Bench(_) => ITEM_HEADER,
            BuyList::Fishes | BuyList::Tanks => NAME_HEADER,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum QtyTarget {
    Stock(StockItem),
    Food,
}

impl QtyTarget {
    pub fn display_name(self) -> &'static str {
        match self {
            QtyTarget::Stock(item) => item.display_name(),
            QtyTarget::Food => "Food",
        }
    }

    pub fn flow(self) -> Flow {
        match self {
            QtyTarget::Food => Flow::Food,
            QtyTarget::Stock(item) if item == StockItem::COFFEE => Flow::Coffee,
            QtyTarget::Stock(item) if item == StockItem::BAIT => Flow::Bait,
            QtyTarget::Stock(_) => Flow::Robotics,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Purchase {
    Browse(BuyList),
    Tiers,
    Counted { target: QtyTarget, unit_price: u32 },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BuyCategory {
    Fishes,
    Coffee,
    Bait,
    Food,
    Fishtank,
    Robotics,
}

impl BuyCategory {
    pub const ALL: &'static [BuyCategory] = &[
        BuyCategory::Fishes,
        BuyCategory::Coffee,
        BuyCategory::Bait,
        BuyCategory::Food,
        BuyCategory::Fishtank,
        BuyCategory::Robotics,
    ];

    pub fn display_name(self) -> &'static str {
        match self {
            BuyCategory::Fishes => "Fishes",
            BuyCategory::Coffee => ConsumableKind::Coffee.display_name(),
            BuyCategory::Bait => ConsumableKind::Bait.display_name(),
            BuyCategory::Food => "Food",
            BuyCategory::Fishtank => "Fishtank",
            BuyCategory::Robotics => "Robotics",
        }
    }

    pub fn purchase(self) -> Purchase {
        match self {
            BuyCategory::Fishes => Purchase::Browse(BuyList::Fishes),
            BuyCategory::Fishtank => Purchase::Browse(BuyList::Tanks),
            BuyCategory::Robotics => Purchase::Tiers,
            BuyCategory::Coffee => Purchase::Counted {
                target: QtyTarget::Stock(StockItem::COFFEE),
                unit_price: ConsumableKind::Coffee.buy_price(),
            },
            BuyCategory::Bait => Purchase::Counted {
                target: QtyTarget::Stock(StockItem::BAIT),
                unit_price: ConsumableKind::Bait.buy_price(),
            },
            BuyCategory::Food => Purchase::Counted {
                target: QtyTarget::Food,
                unit_price: FOOD_BUY_PRICE,
            },
        }
    }

    pub fn index(self) -> usize {
        BuyCategory::ALL
            .iter()
            .position(|&cat| cat == self)
            .unwrap_or(0)
    }

    pub fn is_available(self, access: ShopAccess) -> bool {
        match self.purchase() {
            Purchase::Tiers => tier_shelf(access).available_count() > 0,
            Purchase::Browse(list) => list.has_anything_available(access),
            Purchase::Counted { unit_price, .. } => affords(unit_price, access.cash),
        }
    }

    pub fn shelf(access: ShopAccess) -> Shelf {
        Shelf::of(
            BuyCategory::ALL
                .iter()
                .map(|cat| cat.is_available(access))
                .collect(),
            |item| item,
        )
    }
}

const PENGUIN_LINES: &[&str] = &["  __   ", " ( o>  ", " ///\\  ", " \\V_/_ "];

pub const JUNK_SELL_PRICE: u32 = 1;

pub struct QtyPopup {
    pub target: QtyTarget,
    pub unit_price: u32,
    pub qty: u32,
    pub max_qty: u32,
}

impl QtyPopup {
    pub fn new(target: QtyTarget, unit_price: u32, cash: Money) -> Option<Self> {
        if unit_price == 0 || !affords(unit_price, cash) {
            return None;
        }
        Some(Self {
            target,
            unit_price,
            qty: 1,
            max_qty: u32::try_from(cash / Money::from(unit_price)).unwrap_or(u32::MAX),
        })
    }

    pub fn cost(&self) -> Money {
        Money::from(self.qty) * Money::from(self.unit_price)
    }
}

pub struct BuyTankPopup {
    pub kind: TankKind,
    pub name_input: TextInput,
}

pub fn buy_cat_first_available(access: ShopAccess) -> usize {
    BuyCategory::shelf(access).first()
}

pub fn buy_cat_next(current: usize, down: bool, access: ShopAccess) -> usize {
    BuyCategory::shelf(access).step(current, down)
}

pub struct FishNamePopup {
    pub catalog_idx: usize,
    pub fish: Fish,
    pub name_input: TextInput,
}

pub enum SellEntry {
    Fish {
        name: String,
        species: FishSpecies,
        sell_value: Money,
        fed: Fed,
    },
    Junk {
        qty: u32,
    },
    Coffee {
        qty: u32,
    },
    Bait {
        qty: u32,
    },
    Milk {
        variant: MilkVariant,
        qty: u32,
    },
    Tank {
        name: String,
        sell_price: u32,
    },
    Seed {
        kind: ConsumableKind,
        qty: u32,
    },
    Robotics {
        kind: ConsumableKind,
        qty: u32,
    },
    Blueprint {
        name: String,
    },
}

impl SellEntry {
    pub fn label(&self) -> String {
        match self {
            SellEntry::Fish { name, species, .. } => {
                format!("{} ({})", name, species.display_name())
            }
            SellEntry::Junk { qty } => format!("Junk ({})", qty),
            SellEntry::Coffee { qty } => format!("Coffee ({})", qty),
            SellEntry::Bait { qty } => format!("Bait ({})", qty),
            SellEntry::Milk { variant, qty } => {
                format!(
                    "{} ({})",
                    ConsumableKind::display_name(ConsumableKind::Milk(*variant)),
                    qty
                )
            }
            SellEntry::Tank { name, .. } => format!("{} (Fishtank)", name),
            SellEntry::Seed { kind, qty } | SellEntry::Robotics { kind, qty } => {
                format!("{} ({})", kind.display_name(), qty)
            }
            SellEntry::Blueprint { name } => format!("{} ({})", name, CIRCUIT_BLUEPRINT_NAME),
        }
    }

    pub fn price_label(&self) -> String {
        format!("${}", self.unit_price())
    }

    pub fn fed(&self) -> Option<Fed> {
        match self {
            SellEntry::Fish { fed, .. } => Some(*fed),
            _ => None,
        }
    }

    fn fed_label(&self) -> Option<String> {
        self.fed().map(fields::format_fed)
    }

    pub fn unit_price(&self) -> Money {
        let price = match self {
            SellEntry::Fish { sell_value, .. } => return *sell_value,
            SellEntry::Junk { .. } => JUNK_SELL_PRICE,
            SellEntry::Coffee { .. } => ConsumableKind::Coffee.sell_price(),
            SellEntry::Bait { .. } => ConsumableKind::Bait.sell_price(),
            SellEntry::Milk { .. } => {
                ConsumableKind::Milk(crate::loot::MilkVariant::Plain).sell_price()
            }
            SellEntry::Tank { sell_price, .. } => *sell_price,
            SellEntry::Seed { kind, .. } | SellEntry::Robotics { kind, .. } => kind.sell_price(),
            SellEntry::Blueprint { .. } => CIRCUIT_BLUEPRINT_SELL_PRICE,
        };
        Money::from(price)
    }

    pub fn flow(&self) -> Flow {
        match self {
            SellEntry::Fish { .. } => Flow::FishSales,
            SellEntry::Tank { .. } => Flow::TankSales,
            SellEntry::Junk { .. }
            | SellEntry::Coffee { .. }
            | SellEntry::Bait { .. }
            | SellEntry::Milk { .. }
            | SellEntry::Seed { .. }
            | SellEntry::Robotics { .. }
            | SellEntry::Blueprint { .. } => Flow::StockSales,
        }
    }

    pub fn max_qty(&self) -> u32 {
        match self {
            SellEntry::Fish { .. } | SellEntry::Tank { .. } | SellEntry::Blueprint { .. } => 1,
            SellEntry::Junk { qty }
            | SellEntry::Coffee { qty }
            | SellEntry::Bait { qty }
            | SellEntry::Milk { qty, .. }
            | SellEntry::Seed { qty, .. }
            | SellEntry::Robotics { qty, .. } => *qty,
        }
    }
}

pub struct SellConfirm {
    pub item_idx: usize,
    pub sell_qty: u32,
}

impl SellConfirm {
    pub fn qty_up(&mut self, max: u32) {
        if self.sell_qty < max {
            self.sell_qty += 1;
        }
    }

    pub fn qty_down(&mut self) {
        if self.sell_qty > 1 {
            self.sell_qty -= 1;
        }
    }
}

pub struct SellMenuState {
    pub items: Vec<SellEntry>,
    pub selected: usize,
    scroll: Scroll,
    pub confirm: Option<SellConfirm>,
}

impl SellMenuState {
    pub fn new(
        tank_fish: &[(String, FishSpecies, Money, Fed)],
        inventory: &HashMap<StockItem, u32>,
        sellable_tanks: &[(String, u32)],
        blueprint_names: &[String],
    ) -> Option<Self> {
        let mut items: Vec<SellEntry> = Vec::new();
        let qty_of = |stock: StockItem| inventory.get(&stock).copied().unwrap_or(0);
        let junk = qty_of(StockItem::Junk);
        if junk > 0 {
            items.push(SellEntry::Junk { qty: junk });
        }
        let coffee = qty_of(StockItem::COFFEE);
        if coffee > 0 {
            items.push(SellEntry::Coffee { qty: coffee });
        }
        let bait = qty_of(StockItem::BAIT);
        if bait > 0 {
            items.push(SellEntry::Bait { qty: bait });
        }
        for kind in ConsumableKind::seeds() {
            let qty = qty_of(StockItem::Consumable(kind));
            if qty > 0 {
                items.push(SellEntry::Seed { kind, qty });
            }
        }
        for &variant in MilkVariant::ALL {
            let q = qty_of(StockItem::Consumable(ConsumableKind::Milk(variant)));
            if q > 0 {
                items.push(SellEntry::Milk { variant, qty: q });
            }
        }
        for kind in ConsumableKind::robotics_stock() {
            let qty = qty_of(StockItem::Consumable(kind));
            if qty > 0 {
                items.push(SellEntry::Robotics { kind, qty });
            }
        }
        for name in blueprint_names {
            items.push(SellEntry::Blueprint { name: name.clone() });
        }
        for (tank_name, sell_price) in sellable_tanks {
            items.push(SellEntry::Tank {
                name: tank_name.clone(),
                sell_price: *sell_price,
            });
        }
        items.extend(tank_fish.iter().map(|(n, s, sv, fed)| SellEntry::Fish {
            name: n.clone(),
            species: *s,
            sell_value: *sv,
            fed: *fed,
        }));
        if items.is_empty() {
            return None;
        }
        items.sort_by_cached_key(|entry| (entry.unit_price(), entry.label()));
        Some(Self {
            items,
            selected: 0,
            scroll: Scroll::default(),
            confirm: None,
        })
    }

    fn has_fish(&self) -> bool {
        self.items.iter().any(|entry| entry.fed().is_some())
    }

    fn fed_room(&self) -> u16 {
        if !self.has_fish() {
            return 0;
        }
        let notes = self.items.iter().filter_map(SellEntry::fed_label);
        note_width(
            fields::FED_HEADER,
            notes.map(|note| table::visual_width(&note)),
        ) + PRICE_GAP
    }

    pub fn scroll_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        if self.selected + 1 < self.items.len() {
            self.selected += 1;
        }
    }
}

fn affords(price: u32, cash: Money) -> bool {
    Money::from(price) <= cash
}

pub struct BuyListState<P> {
    list: BuyList,
    pub selected: usize,
    scroll: Scroll,
    pub popup: Option<P>,
}

impl<P> BuyListState<P> {
    pub fn list(&self) -> BuyList {
        self.list
    }

    pub fn new(list: BuyList, access: ShopAccess) -> Self {
        Self {
            list,
            selected: list.shelf(access).first(),
            scroll: Scroll::default(),
            popup: None,
        }
    }

    pub fn scroll_up(&mut self, access: ShopAccess) {
        self.selected = self.list.shelf(access).step(self.selected, false);
    }

    pub fn scroll_down(&mut self, access: ShopAccess) {
        self.selected = self.list.shelf(access).step(self.selected, true);
    }

    pub fn settle(&mut self, access: ShopAccess) {
        self.selected = self.list.shelf(access).settle(self.selected);
    }

    fn is_on_sale(&self, access: ShopAccess) -> bool {
        self.list.shelf(access).is_available(self.selected)
    }

    fn body_rows(&self) -> u16 {
        COLUMN_HEADER_ROWS + self.list.height() as u16
    }

    fn hints(&self, access: ShopAccess, overflowing: bool) -> HintBar {
        let shelf = self.list.shelf(access);
        scroll_hints(
            shelf.rank(self.selected),
            shelf.available_count(),
            overflowing,
        )
    }

    fn draw(
        &self,
        buf: &mut Buffer,
        panels: &Panels,
        access: ShopAccess,
        cursor_vis: bool,
        dim: bool,
    ) {
        draw_priced_list(
            buf,
            panels,
            self.list.header(),
            &ListView {
                rows: &self.list.rows(access),
                selected: self.list.shelf(access).position(self.selected),
                scroll: &self.scroll,
                cursor_vis,
                dim,
                note_header: None,
            },
        );
    }
}

impl BuyListState<FishNamePopup> {
    pub fn purchase(&self, access: ShopAccess) -> Option<FishNamePopup> {
        let species = *FishSpecies::all_buyable().get(self.selected)?;
        self.is_on_sale(access).then(|| FishNamePopup {
            catalog_idx: self.selected,
            fish: Fish::new_for_display(species, &mut rand::rng()),
            name_input: TextInput::new(),
        })
    }
}

impl BuyListState<BuyTankPopup> {
    pub fn purchase(&self, access: ShopAccess) -> Option<BuyTankPopup> {
        let kind = *TankKind::all_buyable().get(self.selected)?;
        self.is_on_sale(access).then(|| BuyTankPopup {
            kind,
            name_input: TextInput::new(),
        })
    }
}

impl BuyListState<QtyPopup> {
    pub fn purchase(&self, access: ShopAccess) -> Option<QtyPopup> {
        let kind = *self.list.stock().get(self.selected)?;
        if !self.is_on_sale(access) {
            return None;
        }
        QtyPopup::new(
            QtyTarget::Stock(StockItem::Consumable(kind)),
            kind.buy_price(),
            access.cash,
        )
    }
}

pub enum ShopPage {
    Main {
        selected: usize,
    },
    BuyCategory {
        selected: usize,
        buy_popup: Option<QtyPopup>,
    },
    BuyBenchTiers {
        selected: usize,
    },
    BuyFishList(Box<BuyListState<FishNamePopup>>),
    BuyTankList(BuyListState<BuyTankPopup>),
    BuyBenchList(BuyListState<QtyPopup>),
    Sell(SellMenuState),
}

pub struct ShopState {
    pub page: ShopPage,
    pub cursor_visible: bool,
    blink_timer: f32,
    category_scroll: Scroll,
}

impl Default for ShopState {
    fn default() -> Self {
        Self::new()
    }
}

impl ShopState {
    pub fn open(access: ShopAccess) -> Self {
        Self {
            page: ShopPage::Main {
                selected: Counter::shelf(access).first(),
            },
            ..Self::new()
        }
    }

    pub fn new() -> Self {
        Self {
            page: ShopPage::Main { selected: 0 },
            cursor_visible: true,
            blink_timer: 0.0,
            category_scroll: Scroll::default(),
        }
    }

    pub fn tick(&mut self, fps: f32) {
        self.blink_timer += 1.0;
        let half = (fps * 0.5).max(1.0);
        if self.blink_timer >= half {
            self.blink_timer = 0.0;
            self.cursor_visible = !self.cursor_visible;
        }
        if let ShopPage::BuyFishList(ref mut fl) = self.page
            && let Some(ref mut p) = fl.popup
        {
            p.fish.tick_animation(1.0 / fps);
        }
    }

    pub fn reset_blink(&mut self) {
        self.cursor_visible = true;
        self.blink_timer = 0.0;
    }
}

impl ShopPage {
    fn title(&self) -> &'static str {
        match self {
            ShopPage::Main { .. } => " Shop ",
            ShopPage::BuyCategory { .. }
            | ShopPage::BuyBenchTiers { .. }
            | ShopPage::BuyFishList(_)
            | ShopPage::BuyTankList(_)
            | ShopPage::BuyBenchList(_) => " Buy ",
            ShopPage::Sell(_) => " Sell ",
        }
    }

    fn has_popup(&self) -> bool {
        match self {
            ShopPage::Main { .. } => false,
            ShopPage::BuyCategory { buy_popup, .. } => buy_popup.is_some(),
            ShopPage::BuyBenchTiers { .. } => false,
            ShopPage::BuyFishList(fl) => fl.popup.is_some(),
            ShopPage::BuyTankList(tl) => tl.popup.is_some(),
            ShopPage::BuyBenchList(pl) => pl.popup.is_some(),
            ShopPage::Sell(sm) => sm.confirm.is_some(),
        }
    }

    fn body_rows(&self) -> u16 {
        match self {
            ShopPage::Main { .. } => MAIN_ROWS,
            ShopPage::BuyCategory { .. } => BuyCategory::ALL.len() as u16,
            ShopPage::BuyBenchTiers { .. } => bench_tiers().len() as u16,
            ShopPage::BuyFishList(fl) => fl.body_rows(),
            ShopPage::BuyTankList(tl) => tl.body_rows(),
            ShopPage::BuyBenchList(pl) => pl.body_rows(),
            ShopPage::Sell(sm) => COLUMN_HEADER_ROWS + sm.items.len() as u16,
        }
    }

    fn body_w(&self) -> u16 {
        match self {
            ShopPage::Sell(sm) => RIGHT_INNER_WIDTH + sm.fed_room(),
            _ => RIGHT_INNER_WIDTH,
        }
    }
}

pub struct ShopOverlay<'a> {
    pub state: &'a ShopState,
    pub access: ShopAccess,
    pub screen: Screen,
}

impl<'a> ShopOverlay<'a> {
    pub fn new(state: &'a ShopState, access: ShopAccess, screen: Screen) -> Self {
        Self {
            state,
            access,
            screen,
        }
    }

    fn hints(&self, overflowing: bool) -> HintBar {
        let access = self.access;
        match &self.state.page {
            ShopPage::Main { .. } => HintBar::new(HINT_CLOSE).action(HINT_NAV),
            ShopPage::BuyCategory { selected, .. } => scroll_hints(
                BuyCategory::shelf(access).position(*selected) + 1,
                BuyCategory::ALL.len(),
                overflowing,
            ),
            ShopPage::BuyBenchTiers { selected } => scroll_hints(
                tier_shelf(access).position(*selected) + 1,
                bench_tiers().len(),
                overflowing,
            ),
            ShopPage::BuyFishList(fl) => fl.hints(access, overflowing),
            ShopPage::BuyTankList(tl) => tl.hints(access, overflowing),
            ShopPage::BuyBenchList(pl) => pl.hints(access, overflowing),
            ShopPage::Sell(sm) => scroll_hints(sm.selected + 1, sm.items.len(), overflowing),
        }
    }

    fn open(&self, buf: &mut Buffer) -> Panels {
        self.with_spec(|spec| Panels::open(buf, self.screen, spec))
    }

    pub fn measure(&self) -> Panels {
        self.with_spec(|spec| Panels::measure(self.screen, spec))
    }

    fn with_spec<R>(&self, run: impl FnOnce(&PanelSpec) -> R) -> R {
        let page = &self.state.page;
        let fg = if page.has_popup() { DARK_GRAY } else { WHITE };
        let rows = page.body_rows();
        let rows_for = |_: u16| rows;
        let spec = |hints| PanelSpec {
            title: page.title(),
            title_style: Style::default()
                .fg(fg)
                .add_modifier(Modifier::BOLD)
                .bg(BACKGROUND),
            border: Style::default().fg(fg).bg(BACKGROUND),
            background: BACKGROUND,
            side: (PENGUIN_WIDTH, PENGUIN_HEIGHT),
            body_w: page.body_w(),
            body_min_w: MIN_BODY_WIDTH,
            body_rows: &rows_for,
            hints,
            reach: Reach::Tab,
        };
        let calm = self.hints(false);
        let overflowing = Panels::measure(self.screen, &spec(&calm)).body.height < rows;
        let hints = self.hints(overflowing);
        run(&spec(&hints))
    }
}

fn scroll_hints(position: usize, total: usize, overflowing: bool) -> HintBar {
    let nav = if overflowing { HINT_SCROLL } else { HINT_NAV };
    HintBar::new(HINT_BACK).counted(nav, overflowing.then_some((position, total)))
}

impl Widget for ShopOverlay<'_> {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        let state = self.state;
        let dim = state.page.has_popup();
        let cursor_vis = dim || state.cursor_visible;
        let access = self.access;
        let panels = self.open(buf);
        draw_penguin(buf, panels.side, dim);

        match &state.page {
            ShopPage::Main { selected } => {
                draw_main(buf, panels.body, *selected, access, cursor_vis, dim)
            }
            ShopPage::BuyCategory {
                selected,
                buy_popup,
            } => {
                let shelf = BuyCategory::shelf(access);
                let rows = shelf.rows(|item, available| {
                    let name = BuyCategory::ALL[item].display_name().to_string();
                    PricedRow::plain(name, String::new(), available)
                });
                draw_rows(
                    buf,
                    &panels,
                    panels.body,
                    &ListView {
                        rows: &rows,
                        selected: shelf.position(*selected),
                        scroll: &state.category_scroll,
                        cursor_vis,
                        dim,
                        note_header: None,
                    },
                );
                if let Some(popup) = buy_popup {
                    draw_qty_buy_popup(buf, popup, self.screen);
                }
            }
            ShopPage::BuyBenchTiers { selected } => {
                let shelf = tier_shelf(access);
                let tiers = bench_tiers();
                let rows = shelf.rows(|item, available| {
                    PricedRow::plain(
                        tiers[item].display_name().to_string(),
                        String::new(),
                        available,
                    )
                });
                draw_rows(
                    buf,
                    &panels,
                    panels.body,
                    &ListView {
                        rows: &rows,
                        selected: shelf.position(*selected),
                        scroll: &state.category_scroll,
                        cursor_vis,
                        dim,
                        note_header: None,
                    },
                );
            }
            ShopPage::BuyFishList(fl) => {
                fl.draw(buf, &panels, access, cursor_vis, dim);
                if let Some(ref popup) = fl.popup {
                    draw_fish_name_popup(buf, popup, state.cursor_visible, self.screen);
                }
            }
            ShopPage::BuyTankList(tl) => {
                tl.draw(buf, &panels, access, cursor_vis, dim);
                if let Some(ref popup) = tl.popup {
                    draw_buy_tank_name_popup(buf, popup, state.cursor_visible, self.screen);
                }
            }
            ShopPage::BuyBenchList(pl) => {
                pl.draw(buf, &panels, access, cursor_vis, dim);
                if let Some(ref popup) = pl.popup {
                    draw_qty_buy_popup(buf, popup, self.screen);
                }
            }
            ShopPage::Sell(sm) => {
                let rows: Vec<PricedRow> = sm
                    .items
                    .iter()
                    .map(|entry| {
                        PricedRow::plain(entry.label(), entry.price_label(), true)
                            .noted(entry.fed_label())
                    })
                    .collect();
                let note_header = sm.has_fish().then_some(fields::FED_HEADER);
                draw_priced_list(
                    buf,
                    &panels,
                    ITEM_HEADER,
                    &ListView {
                        rows: &rows,
                        selected: sm.selected,
                        scroll: &sm.scroll,
                        cursor_vis,
                        dim,
                        note_header,
                    },
                );
                if let Some(ref confirm) = sm.confirm {
                    draw_sell_confirm_popup(buf, confirm, &sm.items[confirm.item_idx], self.screen);
                }
            }
        }
    }
}

const NAME_HEADER: &str = "Name";
const ITEM_HEADER: &str = "Item";

struct PricedRow {
    label: String,
    note: Option<String>,
    price: String,
    bright: bool,
}

impl PricedRow {
    fn plain(label: String, price: String, bright: bool) -> Self {
        Self {
            label,
            note: None,
            price,
            bright,
        }
    }

    fn noted(self, note: Option<String>) -> Self {
        Self { note, ..self }
    }
}

struct ListView<'a> {
    rows: &'a [PricedRow],
    selected: usize,
    scroll: &'a Scroll,
    cursor_vis: bool,
    dim: bool,
    note_header: Option<&'static str>,
}

struct NoteColumn {
    header: &'static str,
    x: u16,
    width: u16,
}

fn note_width(header: &str, notes: impl Iterator<Item = usize>) -> u16 {
    notes
        .chain([table::visual_width(header)])
        .max()
        .unwrap_or(0) as u16
}

impl ListView<'_> {
    fn note_column(&self, area: Rect) -> Option<NoteColumn> {
        let header = self.note_header?;
        let notes = self.rows.iter().filter_map(|row| row.note.as_deref());
        let width = note_width(header, notes.map(table::visual_width));
        let x = area.right().saturating_sub(TEXT_PAD + width);
        Some(NoteColumn { header, x, width })
    }

    fn price_edge(&self, area: Rect) -> u16 {
        match self.note_column(area) {
            Some(note) => note.x.saturating_sub(PRICE_GAP),
            None => area.right().saturating_sub(TEXT_PAD),
        }
    }
}

pub fn bench_tiers() -> Vec<PartTier> {
    let mut tiers: Vec<PartTier> = Vec::new();
    for kind in ConsumableKind::bench_stock() {
        if !tiers.contains(&kind.bench_tier()) {
            tiers.push(kind.bench_tier());
        }
    }
    tiers
}

pub fn tier_shelf(access: ShopAccess) -> Shelf {
    Shelf::of(
        bench_tiers()
            .into_iter()
            .map(|tier| BuyList::Bench(tier).has_anything_available(access))
            .collect(),
        |item| item,
    )
}

fn draw_penguin(buf: &mut Buffer, side: Rect, dim: bool) {
    let fg = if dim { DARK_GRAY } else { WHITE };
    let style = Style::default().fg(fg).bg(BACKGROUND);
    let x = side.x + side.width.saturating_sub(PENGUIN_WIDTH) / 2;
    for (row, line) in PENGUIN_LINES.iter().enumerate().take(side.height as usize) {
        buf.set_stringn(
            x,
            side.y + row as u16,
            line,
            side.right().saturating_sub(x) as usize,
            style,
        );
    }
}

fn draw_main(
    buf: &mut Buffer,
    body: Rect,
    selected: usize,
    access: ShopAccess,
    cursor_vis: bool,
    dim: bool,
) {
    let lead = MAIN_LEAD_ROWS.min(body.height.saturating_sub(Counter::ALL.len() as u16));
    let start_y = body.y + lead;
    let shelf = Counter::shelf(access);
    for (row, &item) in shelf.order.iter().enumerate() {
        let row_y = start_y + row as u16;
        if row_y >= body.bottom() {
            return;
        }
        let prefix = if item == selected && cursor_vis {
            SELECTED_PREFIX
        } else {
            UNSELECTED_PREFIX
        };
        let fg = if dim || !shelf.is_available(item) {
            DARK_GRAY
        } else {
            WHITE
        };
        buf.set_stringn(
            body.x,
            row_y,
            format!("{prefix}{}", Counter::ALL[item].display_name()),
            body.width as usize,
            Style::default().fg(fg).bg(BACKGROUND),
        );
    }
}

fn draw_priced_list(buf: &mut Buffer, panels: &Panels, title: &str, view: &ListView) {
    let body = panels.body;
    if body.height <= COLUMN_HEADER_ROWS {
        draw_rows(buf, panels, body, view);
        return;
    }
    let header_fg = if view.dim { DARK_GRAY } else { WHITE };
    let bold = Style::default()
        .fg(header_fg)
        .add_modifier(Modifier::BOLD)
        .bg(BACKGROUND);
    let prefix_w = table::visual_width(SELECTED_PREFIX) as u16;
    let price_w = table::visual_width(PRICE_HEADER) as u16;
    let price_x = view.price_edge(body).saturating_sub(price_w);
    let title_room = price_x.saturating_sub(body.x + prefix_w + PRICE_GAP) as usize;
    buf.set_stringn(body.x + prefix_w, body.y, title, title_room, bold);
    if price_x > body.x + prefix_w {
        buf.set_stringn(price_x, body.y, PRICE_HEADER, price_w as usize, bold);
    }
    if let Some(note) = view.note_column(body) {
        buf.set_stringn(note.x, body.y, note.header, note.width as usize, bold);
    }
    Rule {
        label: None,
        fill: Style::default().fg(header_fg).bg(BACKGROUND),
        junction: Style::default().fg(header_fg).bg(BACKGROUND),
    }
    .draw(buf, panels, body.y + 1);
    let data = Rect::new(
        body.x,
        body.y + COLUMN_HEADER_ROWS,
        body.width,
        body.height - COLUMN_HEADER_ROWS,
    );
    draw_rows(buf, panels, data, view);
}

struct Rule<'a> {
    label: Option<&'a str>,
    fill: Style,
    junction: Style,
}

impl Rule<'_> {
    fn draw(&self, buf: &mut Buffer, panels: &Panels, y: u16) {
        let (left, right) = (panels.divider_x(), panels.right_x());
        for x in left..=right {
            buf[(x, y)].set_char(RULE_LINE).set_style(self.fill);
        }
        buf[(left, y)]
            .set_char(RULE_JOINS_LEFT)
            .set_style(self.junction);
        buf[(right, y)]
            .set_char(RULE_JOINS_RIGHT)
            .set_style(self.junction);
        let Some(label) = self.label else {
            return;
        };
        let text = format!("{RULE_LINE}{RULE_LINE} {label} ");
        buf.set_stringn(
            left + 1,
            y,
            &text,
            right.saturating_sub(left + 1) as usize,
            self.fill,
        );
    }
}

fn draw_rows(buf: &mut Buffer, panels: &Panels, area: Rect, view: &ListView) {
    let n = view.rows.len();
    let heights = vec![1usize; n];
    let note = view.note_column(area);
    let shown = view
        .scroll
        .follow(&heights, view.selected, area.height as usize);
    for (y, index) in (area.y..area.bottom()).zip(shown.clone()) {
        let entry = &view.rows[index];
        let is_sel = index == view.selected;
        let fg = if view.dim || !entry.bright {
            DARK_GRAY
        } else {
            WHITE
        };
        let style = Style::default().fg(fg).bg(BACKGROUND);
        let prefix = if is_sel && view.cursor_vis {
            SELECTED_PREFIX
        } else {
            UNSELECTED_PREFIX
        };
        let price_w = table::visual_width(&entry.price) as u16;
        let price_x = view.price_edge(area).saturating_sub(price_w);
        let label_room = if price_w == 0 && note.is_none() {
            area.width as usize
        } else {
            price_x.saturating_sub(area.x + PRICE_GAP) as usize
        };
        let label = table::ellipsize(&format!("{prefix}{}", entry.label), label_room);
        buf.set_stringn(area.x, y, &label, label_room, style);
        if let Some(note) = &note {
            let text = entry.note.as_deref().unwrap_or(table::NOTHING);
            buf.set_stringn(note.x, y, text, note.width as usize, style);
        }
        if price_w > 0 {
            buf.set_stringn(price_x, y, &entry.price, price_w as usize, style);
        }
    }
    Scrollbar {
        x: panels.right_x(),
        top: area.y,
        height: area.height,
    }
    .draw(buf, shown, n, if view.dim { DARK_GRAY } else { WHITE });
}

const QTY_POP_MIN_W: u16 = 26;

fn draw_qty_popup(
    buf: &mut Buffer,
    title: &str,
    qty: u32,
    max_qty: u32,
    unit_price: Money,
    confirm_hint: &str,
    screen: Screen,
) {
    let qty_str = format!("< {} >", qty);
    let total_str = format!("  Total: ${}", Money::from(qty) * unit_price);
    let row_w = table::visual_width(&qty_str) + table::visual_width(&total_str);
    let hints = HintBar::new(confirm_hint).action(HINT_CANCEL);
    let modal = Modal::open(
        buf,
        screen,
        &ModalSpec {
            title,
            border: WHITE,
            background: BACKGROUND,
            content_w: QTY_POP_MIN_W.max(row_w as u16 + TEXT_PAD * 2),
            content_h: 1,
            hints: &hints,
        },
    );
    if modal.body.height == 0 {
        return;
    }
    let s_white = Style::default().fg(WHITE).bg(BACKGROUND);
    let s_dim = Style::default().fg(DARK_GRAY).bg(BACKGROUND);
    let x = modal.body.x + TEXT_PAD;
    let room = modal.body.width.saturating_sub(TEXT_PAD * 2) as usize;
    let left = if qty <= 1 { s_dim } else { s_white };
    let right = if qty >= max_qty { s_dim } else { s_white };
    let arrows = qty_str.chars().count();
    for (i, ch) in qty_str.chars().enumerate().take(room) {
        let style = match i {
            0 => left,
            last if last + 1 == arrows => right,
            _ => s_white,
        };
        buf[(x + i as u16, modal.body.y)]
            .set_char(ch)
            .set_style(style);
    }
    let rest = room.saturating_sub(arrows);
    buf.set_stringn(x + arrows as u16, modal.body.y, &total_str, rest, s_white);
}

fn draw_qty_buy_popup(buf: &mut Buffer, popup: &QtyPopup, screen: Screen) {
    draw_qty_popup(
        buf,
        &format!(" Buy {} ", popup.target.display_name()),
        popup.qty,
        popup.max_qty,
        Money::from(popup.unit_price),
        HINT_ENTER_BUY,
        screen,
    );
}

const NAME_POPUP_BODY_W: u16 = 30;
const FISH_POPUP_SIDE_H: u16 = 5;
const FISH_POPUP_SIDE_PAD: u16 = 3;
const NAME_LABEL: &str = "Name it";
const NAME_CANCEL: &str = "ESC cancel";

const NAME_ROWS_AFTER_HEADER: u16 = 3;

fn header_lines(header: &str, body_w: u16) -> Vec<String> {
    table::wrap_words(header, body_w.saturating_sub(TEXT_PAD * 2) as usize)
}

fn draw_name_rows(
    buf: &mut Buffer,
    body: Rect,
    lines: &[String],
    input: &TextInput,
    cursor_vis: bool,
) {
    if body.height == 0 {
        return;
    }
    let bold = Style::default()
        .fg(WHITE)
        .add_modifier(Modifier::BOLD)
        .bg(BACKGROUND);
    let white = Style::default().fg(WHITE).bg(BACKGROUND);
    let x = body.x + TEXT_PAD;
    let room = body.width.saturating_sub(TEXT_PAD * 2);
    let input_y = body.bottom() - 1;
    draw_text_cursor(buf, input, cursor_vis, x, input_y, room, BACKGROUND);
    let Some(label_y) = input_y.checked_sub(1).filter(|&y| y >= body.y) else {
        return;
    };
    buf.set_stringn(x, label_y, NAME_LABEL, room as usize, white);
    let fits_gap = body.height >= lines.len() as u16 + NAME_ROWS_AFTER_HEADER;
    let header_end = if fits_gap { label_y - 1 } else { label_y };
    for (row, line) in lines.iter().enumerate() {
        let y = body.y + row as u16;
        if y >= header_end {
            return;
        }
        buf.set_stringn(x, y, line, room as usize, bold);
    }
}

fn open_wrapped(
    buf: &mut Buffer,
    screen: Screen,
    frame: &Frame,
    message: &str,
    (min_w, rows_after): (u16, u16),
    hints: &HintBar,
) -> (Modal, Vec<String>) {
    let content_w = min_w.max(table::visual_width(message) as u16 + TEXT_PAD * 2);
    let inner_w = Modal::inner_width(screen, frame.title, content_w, hints);
    let lines = header_lines(message, inner_w);
    let modal = Modal::open(
        buf,
        screen,
        &ModalSpec {
            title: frame.title,
            border: frame.border,
            background: frame.background,
            content_w,
            content_h: lines.len() as u16 + rows_after,
            hints,
        },
    );
    (modal, lines)
}

struct FishNaming<'a> {
    fish: &'a Fish,
    header: String,
    title: String,
    confirm: &'static str,
    input: &'a TextInput,
}

fn draw_fish_name_popup(buf: &mut Buffer, popup: &FishNamePopup, cursor_vis: bool, screen: Screen) {
    let species = FishSpecies::all_buyable()[popup.catalog_idx];
    let naming = FishNaming {
        fish: &popup.fish,
        header: format!(
            "{} for sale! Only ${}",
            species.display_name(),
            species.buy_price()
        ),
        title: format!(" Buy {} ", species.display_name()),
        confirm: HINT_ENTER_BUY,
        input: &popup.name_input,
    };
    draw_fish_naming(buf, &naming, cursor_vis, screen);
}

fn draw_fish_naming(buf: &mut Buffer, naming: &FishNaming, cursor_vis: bool, screen: Screen) {
    let FishNaming {
        fish,
        header,
        title,
        confirm,
        input,
    } = naming;
    let hints = HintBar::new(confirm).action(NAME_CANCEL);
    let rows_for = |body_w: u16| header_lines(header, body_w).len() as u16 + NAME_ROWS_AFTER_HEADER;
    let spec = PanelSpec {
        title,
        title_style: Style::default()
            .fg(WHITE)
            .add_modifier(Modifier::BOLD)
            .bg(BACKGROUND),
        border: Style::default().fg(WHITE).bg(BACKGROUND),
        background: BACKGROUND,
        side: (
            fish.display_width as u16 + FISH_POPUP_SIDE_PAD,
            fish_art_height(fish, FISH_POPUP_SIDE_H),
        ),
        body_w: NAME_POPUP_BODY_W.max(table::visual_width(header) as u16 + TEXT_PAD * 2),
        body_min_w: MIN_BODY_WIDTH,
        body_rows: &rows_for,
        hints: &hints,
        reach: Reach::Full,
    };
    let panels = Panels::open(buf, screen, &spec);
    draw_fish_centred(buf, fish, panels.side, BACKGROUND);
    let lines = header_lines(header, panels.body.width);
    draw_name_rows(buf, panels.body, &lines, input, cursor_vis);
}

pub struct JunkfishPopup {
    pub fish: Fish,
    pub name_input: TextInput,
}

pub struct JunkfishPopupWidget<'a> {
    pub popup: &'a JunkfishPopup,
    pub cursor_visible: bool,
    pub screen: Screen,
}

impl Widget for JunkfishPopupWidget<'_> {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        let fish = &self.popup.fish;
        let species = fish.species.display_name();
        let naming = FishNaming {
            fish,
            header: format!(
                "Worth ${}: a {species} made of {JUNK_PER_JUNKFISH} Junk!",
                grouped(u128::from(fish.sell_value()))
            ),
            title: format!(" Junk to the {species}! "),
            confirm: HINT_ENTER_ASSEMBLE,
            input: &self.popup.name_input,
        };
        draw_fish_naming(buf, &naming, self.cursor_visible, self.screen);
    }
}

const CONFIRM_MIN_W: u16 = 38;

fn draw_sell_confirm_popup(
    buf: &mut Buffer,
    confirm: &SellConfirm,
    entry: &SellEntry,
    screen: Screen,
) {
    let counted = match entry {
        SellEntry::Junk { qty } => Some((" Sell Junk ".to_string(), *qty)),
        SellEntry::Coffee { qty } => Some((" Sell Coffee ".to_string(), *qty)),
        SellEntry::Bait { qty } => Some((" Sell Bait ".to_string(), *qty)),
        SellEntry::Seed { kind, qty } | SellEntry::Robotics { kind, qty } => {
            Some((format!(" Sell {} ", kind.display_name()), *qty))
        }
        SellEntry::Milk { .. }
        | SellEntry::Fish { .. }
        | SellEntry::Tank { .. }
        | SellEntry::Blueprint { .. } => None,
    };
    if let Some((title, qty)) = counted {
        draw_qty_popup(
            buf,
            &title,
            confirm.sell_qty,
            qty,
            entry.unit_price(),
            HINT_ENTER_SELL,
            screen,
        );
        return;
    }
    let (title, name) = match entry {
        SellEntry::Fish { name, species, .. } => {
            (format!(" Sell {} ", species.display_name()), name.as_str())
        }
        SellEntry::Tank { name, .. } => (" Sell Fishtank ".to_string(), name.as_str()),
        SellEntry::Blueprint { name } => {
            (format!(" Sell {CIRCUIT_BLUEPRINT_NAME} "), name.as_str())
        }
        SellEntry::Milk { variant, .. } => (
            format!(" Sell {} ", ConsumableKind::Milk(*variant).display_name()),
            ConsumableKind::Milk(*variant).display_name(),
        ),
        _ => return,
    };
    let message = format!(
        "Sell {} for ${}?",
        name,
        Money::from(confirm.sell_qty) * entry.unit_price()
    );
    let hints = HintBar::new(HINT_ENTER_SELL).action(HINT_CANCEL);
    let frame = Frame {
        title: &title,
        border: WHITE,
        background: BACKGROUND,
    };
    let (modal, lines) = open_wrapped(buf, screen, &frame, &message, (CONFIRM_MIN_W, 0), &hints);
    draw_lines(
        buf,
        modal.body,
        &lines,
        Style::default().fg(WHITE).bg(BACKGROUND),
    );
}

fn draw_lines(buf: &mut Buffer, body: Rect, lines: &[String], style: Style) {
    let room = body.width.saturating_sub(TEXT_PAD * 2) as usize;
    for (row, line) in lines.iter().enumerate().take(body.height as usize) {
        buf.set_stringn(body.x + TEXT_PAD, body.y + row as u16, line, room, style);
    }
}

fn draw_buy_tank_name_popup(
    buf: &mut Buffer,
    popup: &BuyTankPopup,
    cursor_vis: bool,
    screen: Screen,
) {
    let kind = popup.kind;
    let header = format!(
        "{} for sale! Only ${}",
        kind.display_name(),
        kind.buy_price()
    );
    let title = format!(" Buy {} ", kind.display_name());
    let hints = HintBar::new(HINT_ENTER_BUY).action(NAME_CANCEL);
    let frame = Frame {
        title: &title,
        border: WHITE,
        background: BACKGROUND,
    };
    let (modal, lines) = open_wrapped(
        buf,
        screen,
        &frame,
        &header,
        (NAME_POPUP_BODY_W, NAME_ROWS_AFTER_HEADER),
        &hints,
    );
    draw_name_rows(buf, modal.body, &lines, &popup.name_input, cursor_vis);
}

pub struct NamingPopupWidget<'a> {
    pub input: &'a TextInput,
    pub kind: ConsumableKind,
    pub cursor_visible: bool,
    pub screen: Screen,
}

const SUMMON_INPUT_ROWS: u16 = 1;

impl Widget for NamingPopupWidget<'_> {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        let Some(target) = self.kind.name_target() else {
            return;
        };
        let border = self
            .kind
            .summon_border_override()
            .unwrap_or(target.border());
        let header = target.header();
        let title = format!(" {} ", self.kind.display_name());
        let hints = HintBar::new(self.kind.summon_hint()).action(NAME_CANCEL);
        let frame = Frame {
            title: &title,
            border,
            background: BACKGROUND,
        };
        let (modal, lines) = open_wrapped(
            buf,
            self.screen,
            &frame,
            &header,
            (0, SUMMON_INPUT_ROWS),
            &hints,
        );
        if modal.body.height == 0 {
            return;
        }
        let input_y = modal.body.bottom() - 1;
        let x = modal.body.x + TEXT_PAD;
        let room = modal.body.width.saturating_sub(TEXT_PAD * 2);
        draw_text_cursor(
            buf,
            self.input,
            self.cursor_visible,
            x,
            input_y,
            room,
            BACKGROUND,
        );
        let bold = Style::default()
            .fg(WHITE)
            .add_modifier(Modifier::BOLD)
            .bg(BACKGROUND);
        let header_rows = Rect::new(
            modal.body.x,
            modal.body.y,
            modal.body.width,
            modal.body.height - 1,
        );
        draw_lines(buf, header_rows, &lines, bold);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fishes::parts::{Part, PartTier};

    const ROOMY_COLS: u16 = 100;
    const ROOMY_ROWS: u16 = 40;
    const TINY_COLS: u16 = 24;
    const TINY_ROWS: u16 = 10;

    fn roomy() -> Screen {
        Screen::only(Rect::new(0, 0, ROOMY_COLS, ROOMY_ROWS))
    }

    fn buy_category_state() -> ShopState {
        let mut state = ShopState::new();
        state.page = ShopPage::BuyCategory {
            selected: 0,
            buy_popup: None,
        };
        state
    }

    fn measured(state: &ShopState, screen: Screen) -> Panels {
        ShopOverlay::new(state, connected_rich(), screen).measure()
    }

    #[test]
    fn the_buy_category_page_keeps_exactly_one_padding_row() {
        let panels = measured(&buy_category_state(), roomy());
        assert_eq!(panels.body.height as usize, BuyCategory::ALL.len());
        assert_eq!(
            panels.hints.y,
            panels.body.bottom() + 1,
            "one blank padding row between the list and the hint row"
        );
    }

    #[test]
    fn the_buy_category_page_draws_every_category_without_scrolling() {
        assert_eq!(
            measured(&buy_category_state(), roomy()).body.height as usize,
            BuyCategory::ALL.len(),
            "a new category must grow the page, not scroll inside it"
        );
    }

    #[test]
    fn a_narrow_screen_still_opens_the_shop_with_the_penguin_above_the_list() {
        let tiny = Screen::only(Rect::new(0, 0, TINY_COLS, TINY_ROWS));
        let panels = measured(&buy_category_state(), tiny);
        assert!(panels.stacked, "the penguin wraps above the list");
        assert!(panels.body.height >= 1);
        assert!(panels.hints.height >= 1);
    }

    #[test]
    fn every_buy_category_row_is_reachable() {
        assert_eq!(
            buy_cat_next(BuyCategory::ALL.len() - 1, true, connected_rich()),
            BuyCategory::ALL.len() - 1,
            "navigation stops at the last category instead of running past the list"
        );
    }

    fn bench(access: ShopAccess) -> BuyListState<QtyPopup> {
        BuyListState::new(BuyList::Bench(PartTier::Fabric), access)
    }

    fn holding(cash: Money) -> ShopAccess {
        ShopAccess {
            cash,
            ..connected_rich()
        }
    }

    fn connected_rich() -> ShopAccess {
        ShopAccess {
            cash: Money::MAX,
            connected: true,
            room: true,
            sellable: true,
        }
    }

    fn unconnected_rich() -> ShopAccess {
        ShopAccess {
            cash: Money::MAX,
            connected: false,
            room: true,
            sellable: true,
        }
    }

    #[test]
    fn robotics_is_the_one_category_a_matrixtank_unlocks() {
        for &cat in BuyCategory::ALL {
            assert_eq!(
                cat.is_available(unconnected_rich()),
                cat != BuyCategory::Robotics,
                "{} answered the wrong way with no Matrixtank",
                cat.display_name()
            );
        }
        assert!(BuyCategory::Robotics.is_available(connected_rich()));
    }

    #[test]
    fn a_connected_pauper_still_cannot_reach_the_robotics_bench() {
        let cheapest = bench_tiers()
            .into_iter()
            .flat_map(|tier| BuyList::Bench(tier).catalogue())
            .map(|(_, price)| price)
            .min()
            .expect("the bench stocks something");
        let broke = ShopAccess {
            cash: Money::from(cheapest) - 1,
            connected: true,
            room: true,
            sellable: true,
        };
        assert!(!BuyCategory::Robotics.is_available(broke));
        assert!(BuyCategory::Robotics.is_available(ShopAccess {
            cash: Money::from(cheapest),
            connected: true,
            room: true,
            sellable: true,
        }));
    }

    #[test]
    fn navigation_skips_a_locked_robotics_row_from_both_sides() {
        let last = BuyCategory::ALL.len() - 1;
        assert_eq!(
            buy_cat_next(BuyCategory::Fishtank.index(), true, unconnected_rich()),
            BuyCategory::Fishtank.index(),
            "with no Matrixtank there is nothing below the Fishtank row"
        );
        assert_eq!(
            buy_cat_next(BuyCategory::Fishtank.index(), true, connected_rich()),
            last
        );
    }

    #[test]
    fn every_category_names_itself_exactly_once() {
        let mut names: Vec<&str> = BuyCategory::ALL
            .iter()
            .map(|cat| cat.display_name())
            .collect();
        assert_eq!(names.len(), BuyCategory::ALL.len());
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), BuyCategory::ALL.len(), "two rows share a name");
        for (idx, &cat) in BuyCategory::ALL.iter().enumerate() {
            assert_eq!(cat.index(), idx, "index() disagrees with ALL");
        }
    }

    #[test]
    fn the_robotics_bench_walks_down_to_its_last_row_and_back() {
        let stock = BuyList::Bench(PartTier::Fabric).catalogue().len();
        let shelf = BuyList::Bench(PartTier::Fabric).shelf(connected_rich());
        let mut list = bench(connected_rich());
        assert_eq!(
            shelf.position(list.selected),
            0,
            "the bench opens on its top row"
        );
        for _ in 0..stock {
            list.scroll_down(connected_rich());
        }
        assert_eq!(shelf.position(list.selected), stock - 1);
        for _ in 0..stock {
            list.scroll_up(connected_rich());
        }
        assert_eq!(shelf.position(list.selected), 0);
    }

    #[test]
    fn the_bench_sells_every_part_by_tier_then_foundry_stock_ending_with_the_blank_blueprint() {
        let mut expected: Vec<ConsumableKind> = Part::ALL
            .iter()
            .map(|&part| ConsumableKind::Part(part))
            .collect();
        expected.sort_by_key(|kind| kind.bench_tier().index());
        expected.push(ConsumableKind::BlankWafer);
        expected.push(ConsumableKind::Fabricator);
        expected.push(ConsumableKind::BlankBlueprint);
        assert!(
            ConsumableKind::bench_stock() == expected,
            "the bench follows the tier table, the pipeline follows Part::ALL"
        );
    }

    #[test]
    fn the_bench_menu_lists_every_tier_in_the_tier_tables_order() {
        assert_eq!(
            bench_tiers(),
            vec![
                PartTier::Fabric,
                PartTier::Senses,
                PartTier::Hands,
                PartTier::Peripherals,
                PartTier::Materials,
            ],
            "Robotics opens on the tiers, in the order of the tier table"
        );
    }

    #[test]
    fn every_piece_of_bench_stock_sits_on_exactly_one_tier_page() {
        for kind in ConsumableKind::bench_stock() {
            let pages: Vec<PartTier> = bench_tiers()
                .into_iter()
                .filter(|&tier| {
                    BuyList::Bench(tier)
                        .catalogue()
                        .iter()
                        .any(|(name, _)| *name == kind.display_name())
                })
                .collect();
            assert_eq!(
                pages,
                vec![kind.bench_tier()],
                "{} must be on its own tier's page and nowhere else",
                kind.display_name()
            );
        }
    }

    #[test]
    fn a_tier_page_is_as_tall_as_its_own_rows() {
        for tier in bench_tiers() {
            let list = BuyList::Bench(tier);
            assert_eq!(list.height(), list.rows(connected_rich()).len());
            assert!(
                list.rows(connected_rich()).len() < ConsumableKind::bench_stock().len(),
                "a tier page is a page, not the whole bench"
            );
        }
    }

    #[test]
    fn a_tier_page_buys_the_row_its_own_cursor_is_on() {
        for tier in bench_tiers() {
            let stock = BuyList::Bench(tier).stock();
            for (index, kind) in stock.iter().enumerate() {
                let mut list: BuyListState<QtyPopup> =
                    BuyListState::new(BuyList::Bench(tier), connected_rich());
                list.selected = index;
                let popup = list
                    .purchase(connected_rich())
                    .expect("a part row opens a counter");
                assert_eq!(
                    popup.target.display_name(),
                    kind.display_name(),
                    "the {tier:?} page counted out the wrong item"
                );
            }
        }
    }

    #[test]
    fn walking_down_a_tier_page_stops_at_its_last_part() {
        let stock = BuyList::Bench(PartTier::Fabric).catalogue().len();
        let shelf = BuyList::Bench(PartTier::Fabric).shelf(connected_rich());
        let mut list = bench(connected_rich());
        for _ in 0..stock * 2 {
            list.scroll_down(connected_rich());
        }
        assert_eq!(shelf.position(list.selected), stock - 1);
        assert!(list.purchase(connected_rich()).is_some());
    }

    #[test]
    fn enter_on_the_blank_blueprint_row_counts_out_blank_blueprints() {
        let mut list: BuyListState<QtyPopup> =
            BuyListState::new(BuyList::Bench(PartTier::Materials), connected_rich());
        list.selected = BuyList::Bench(PartTier::Materials)
            .stock()
            .iter()
            .position(|&kind| kind == ConsumableKind::BlankBlueprint)
            .expect("the Materials page sells blank blueprints");
        let popup = list
            .purchase(connected_rich())
            .expect("a rich player can buy one");
        assert_eq!(
            popup.target.display_name(),
            ConsumableKind::BlankBlueprint.display_name()
        );
        assert_eq!(popup.unit_price, ConsumableKind::BlankBlueprint.buy_price());
    }

    #[test]
    fn stepping_skips_what_the_player_cannot_afford() {
        let prices = [10, 500, 20, 900];
        let shelf = Shelf::of(
            prices.iter().map(|&price| affords(price, 100)).collect(),
            |item| prices[item],
        );
        assert_eq!(
            shelf.order,
            vec![0, 2, 1, 3],
            "affordable first, then by price"
        );
        assert_eq!(shelf.step(0, true), 2);
        assert_eq!(shelf.step(2, true), 2, "nothing further is affordable");
        assert_eq!(shelf.step(2, false), 0);
    }

    #[test]
    fn every_priced_page_lists_what_is_for_sale_first_then_by_price_then_name() {
        let mut lists = vec![BuyList::Fishes, BuyList::Tanks];
        lists.extend(bench_tiers().into_iter().map(BuyList::Bench));
        let cheap_fish = FishSpecies::all_buyable()
            .iter()
            .map(|species| species.buy_price())
            .min()
            .expect("the shop sells fish");
        for access in [
            connected_rich(),
            holding(Money::from(cheap_fish)),
            holding(0),
        ] {
            for &list in &lists {
                let rows = list.rows(access);
                let for_sale = rows.iter().take_while(|row| row.bright).count();
                assert!(
                    rows[for_sale..].iter().all(|row| !row.bright),
                    "a row for sale sank below one that is not"
                );
                for group in [&rows[..for_sale], &rows[for_sale..]] {
                    let keys: Vec<(u32, &str)> = group
                        .iter()
                        .map(|row| (row.price[1..].parse().expect("a price"), row.label.as_str()))
                        .collect();
                    assert!(keys.is_sorted(), "not by price, then name: {keys:?}");
                }
            }
        }
    }

    #[test]
    fn a_category_or_tier_for_sale_is_listed_above_one_that_is_not() {
        let order = BuyCategory::shelf(unconnected_rich()).order;
        assert_eq!(
            order.last(),
            Some(&BuyCategory::Robotics.index()),
            "with no Matrixtank the locked bench sinks to the bottom"
        );
        let broke = holding(0);
        assert_eq!(
            Counter::shelf(broke).order,
            vec![Counter::Sell.index(), Counter::Buy.index()],
            "a pauper with something to sell sees Sell first"
        );
        let nothing_to_sell = ShopAccess {
            sellable: false,
            ..connected_rich()
        };
        assert!(!Counter::Sell.is_available(nothing_to_sell));
        assert!(Counter::Buy.is_available(nothing_to_sell));
    }

    #[test]
    fn the_fish_shelf_closes_when_no_tank_has_room() {
        let full = ShopAccess {
            room: false,
            ..connected_rich()
        };
        assert!(!BuyList::Fishes.has_anything_available(full));
        assert!(!BuyCategory::Fishes.is_available(full));
        let list: BuyListState<FishNamePopup> = BuyListState::new(BuyList::Fishes, full);
        assert!(list.purchase(full).is_none());
    }

    #[test]
    fn the_robotics_bench_opens_on_a_part_the_player_can_afford() {
        let coil = Part::InverterCoil.price();
        let list = bench(holding(Money::from(coil)));
        assert!(
            ConsumableKind::bench_stock()[list.selected].buy_price() <= coil,
            "the cursor never starts on something unaffordable"
        );
    }

    #[test]
    fn a_part_costs_its_rarity_and_the_till_agrees() {
        for &part in Part::ALL {
            let price = part.rarity().part_price();
            let kind = ConsumableKind::Part(part);
            assert_eq!(
                kind.buy_price(),
                price,
                "{part:?} reaches the bench at a price of its own"
            );
            let popup = QtyPopup::new(
                QtyTarget::Stock(StockItem::Consumable(kind)),
                kind.buy_price(),
                Money::from(price) * 3,
            )
            .expect("three of them is affordable");
            assert_eq!(popup.max_qty, 3);
            assert_eq!(popup.cost(), Money::from(price), "the popup opens at one");
            assert_eq!(popup.target.display_name(), part.display_name());
        }
    }

    #[test]
    fn the_sell_menu_is_ordered_by_price_then_name() {
        let inventory = HashMap::from([
            (StockItem::Consumable(ConsumableKind::BlankWafer), 3),
            (
                StockItem::Consumable(ConsumableKind::Part(Part::DelaySpool)),
                2,
            ),
        ]);
        let menu = SellMenuState::new(&[], &inventory, &[], &["Latch".to_string()])
            .expect("there is something to sell");
        let keys: Vec<(Money, String)> = menu
            .items
            .iter()
            .map(|item| (item.unit_price(), item.label()))
            .collect();
        assert!(keys.is_sorted(), "not by price, then name: {keys:?}");
        let latch = menu
            .items
            .iter()
            .find(|item| item.label() == "Latch (Circuit Blueprint)")
            .expect("the blueprint is for sale");
        assert_eq!(latch.max_qty(), 1, "a design is one of a kind");
        assert_eq!(
            latch.unit_price(),
            Money::from(CIRCUIT_BLUEPRINT_SELL_PRICE)
        );
        assert!(
            menu.items
                .iter()
                .any(|item| item.label() == "Delay Spool (2)")
        );
        assert!(
            menu.items
                .iter()
                .any(|item| item.label() == "Blank Wafer (3)")
        );
    }

    fn tank_list_on(kind: TankKind) -> BuyListState<BuyTankPopup> {
        let mut list = BuyListState::new(BuyList::Tanks, connected_rich());
        list.selected = TankKind::all_buyable()
            .iter()
            .position(|&sold| sold == kind)
            .expect("the catalogue sells this tank");
        list
    }

    #[test]
    fn every_tank_the_catalogue_sells_is_named_on_the_spot() {
        for kind in TankKind::all_buyable() {
            let popup = tank_list_on(kind)
                .purchase(connected_rich())
                .expect("a tank row opens the naming popup");
            assert_eq!(popup.kind, kind);
        }
    }

    #[test]
    fn a_tank_row_a_player_cannot_pay_for_opens_nothing() {
        let list = tank_list_on(TankKind::Base);
        assert!(
            list.purchase(holding(Money::from(TankKind::Base.buy_price()) - 1))
                .is_none()
        );
    }

    #[test]
    fn the_tank_catalogue_holds_no_item_and_no_tank_you_are_meant_to_find() {
        let labels: Vec<&str> = BuyList::Tanks
            .catalogue()
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert!(labels.contains(&TankKind::Base.display_name()));
        for seed in ConsumableKind::seeds() {
            assert!(
                !labels.contains(&seed.display_name()),
                "{} only grows a tank, so it is found, never bought",
                seed.display_name()
            );
            let kind = seed.summons_tank().expect("a seed grows a tank");
            assert!(!labels.contains(&kind.display_name()));
        }
        assert!(!labels.contains(&TankKind::Alien.display_name()));
    }

    #[test]
    fn every_seed_a_player_holds_is_on_the_sell_menu() {
        let inventory: HashMap<StockItem, u32> = ConsumableKind::seeds()
            .into_iter()
            .map(|seed| (StockItem::Consumable(seed), 1))
            .collect();
        let menu = SellMenuState::new(&[], &inventory, &[], &[]).expect("seeds are sellable");
        let labels: Vec<String> = menu.items.iter().map(SellEntry::label).collect();
        for seed in ConsumableKind::seeds() {
            let row = format!("{} (1)", seed.display_name());
            assert!(labels.contains(&row), "the sell menu lost {row}");
        }
        for seed in ConsumableKind::seeds() {
            let row = format!("{} (1)", seed.display_name());
            let item = menu
                .items
                .iter()
                .find(|item| item.label() == row)
                .expect("listed above");
            assert_eq!(item.unit_price(), Money::from(seed.sell_price()));
        }
    }

    #[test]
    fn a_qty_popup_never_opens_on_something_unaffordable() {
        assert!(
            QtyPopup::new(QtyTarget::Food, FOOD_BUY_PRICE, 0).is_none(),
            "no cash, no counter"
        );
    }
}
