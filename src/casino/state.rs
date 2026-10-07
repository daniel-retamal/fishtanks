use rand::RngExt;

use crate::economy::Money;
use crate::fishes::fish::Fish;
use crate::fishes::species::FishSpecies;
use crate::ui::input_action::InputAction;
use crate::ui::text_input::TextInput;

use super::blackjack::{Blackjack, Move};
use super::bubble::{Bubble, MAX_IN_FLIGHT, Risk, SHELL_FLASH_SECS};
use super::derby::Derby;
use super::flip::{Flip, FlipEnd, Side};
use super::net::{Cast, Net, Prize};
use super::pufferfish::Pufferfish;
use super::seat::{OnTheLine, Round, RoundResult, Seat, Verdict};
use super::spins::Spins;
use super::{Entrant, House, Multiple, Teller};

pub const FLASH_SECS: f32 = 0.9;
pub const BANNER_COUNT_SECS: f32 = 1.2;
const BANNER_READY_SECS: f32 = 0.35;
pub const REVEAL_SECS: f32 = 1.2;
const PAYTABLE_ROWS: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Game {
    Blackjack,
    Spins,
    Pufferfish,
    BubbleUp,
    Derby,
    MysteryNet,
}

impl Game {
    pub const ALL: [Game; 6] = [
        Game::Blackjack,
        Game::Spins,
        Game::Pufferfish,
        Game::BubbleUp,
        Game::Derby,
        Game::MysteryNet,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Game::Blackjack => "Blackjack",
            Game::Spins => "Spins",
            Game::Pufferfish => "Pufferfish",
            Game::BubbleUp => "Bubble Up",
            Game::Derby => "Derby",
            Game::MysteryNet => "Mystery Net",
        }
    }

    pub fn tagline(self) -> &'static str {
        match self {
            Game::Blackjack => "beat Tollomind to 21",
            Game::Spins => "three reels and a Pearl Dive",
            Game::Pufferfish => "cash out before it pops",
            Game::BubbleUp => "every bubble finds a shell",
            Game::Derby => "five fish, one winner",
            Game::MysteryNet => "a fish in every net",
        }
    }

    pub fn token(self) -> String {
        self.name().replace(' ', "").to_ascii_lowercase()
    }

    pub fn parse(word: &str) -> Option<Game> {
        let word = word.trim().trim_matches('"').replace(' ', "");
        Game::ALL
            .into_iter()
            .find(|game| game.token().eq_ignore_ascii_case(&word))
    }

    pub fn takes_fish(self) -> bool {
        self != Game::MysteryNet
    }

    pub fn is_open(self, house: &dyn Teller) -> bool {
        match self {
            Game::MysteryNet => {
                house.room_for_a_prize()
                    && (house.spendable() >= Net::Small.price()
                        || house.entrants().iter().any(|e| e.stakeable))
            }
            _ => house.spendable() >= 1 || house.entrants().iter().any(|e| e.stakeable),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Flash {
    Win,
    Big,
    Loss,
}

#[derive(Clone)]
pub enum Play {
    Blackjack(Option<Blackjack>),
    Spins(Spins),
    Pufferfish(Pufferfish),
    BubbleUp {
        risk: Risk,
        bubbles: Vec<Bubble>,
        lit: Vec<(Risk, usize, f32)>,
    },
    Derby(Derby),
    Net(Cast),
}

#[derive(Clone, Debug)]
pub enum Card {
    Win(Banner),
    Catch(FishSpecies),
}

impl Card {
    fn outranks(&self, other: &Card) -> bool {
        match (self, other) {
            (Card::Win(new), Card::Win(old)) => new.multiple > old.multiple,
            _ => false,
        }
    }

    fn turn_over(self, rng: &mut impl RngExt) -> Popup {
        match self {
            Card::Win(banner) => Popup::Banner(banner),
            Card::Catch(species) => Popup::Prize(PrizeCard {
                fish: Fish::new(species, String::new(), 0.0, 0.0, rng),
                name: TextInput::new(),
            }),
        }
    }
}

#[derive(Clone)]
pub struct Table {
    pub game: Game,
    pub seat: Seat,
    pub round: Option<Round>,
    pub result: Option<RoundResult>,
    pub flash: Option<(Flash, f32)>,
    pub double: Option<OnTheLine>,
    pub play: Play,
    pub bait: Option<String>,
    pub clock: f32,
    pub face_down: Option<(Card, f32)>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Purpose {
    Stake,
    Bait,
}

#[derive(Clone)]
pub struct Picker {
    pub purpose: Purpose,
    pub rows: Vec<Entrant>,
    pub selected: usize,
}

impl Picker {
    fn new(purpose: Purpose, house: &dyn Teller) -> Option<Picker> {
        let mut rows = house.entrants();
        rows.sort_by(|a, b| {
            b.stakeable
                .cmp(&a.stakeable)
                .then(b.worth.cmp(&a.worth))
                .then(a.name.cmp(&b.name))
        });
        let selected = rows.iter().position(|e| e.stakeable)?;
        Some(Picker {
            purpose,
            rows,
            selected,
        })
    }

    pub fn is_usable(&self, row: usize) -> bool {
        self.rows[row].stakeable
    }

    fn step(&mut self, down: bool) {
        let mut i = self.selected;
        loop {
            i = if down {
                i + 1
            } else {
                match i.checked_sub(1) {
                    Some(prev) => prev,
                    None => return,
                }
            };
            if i >= self.rows.len() {
                return;
            }
            if self.is_usable(i) {
                self.selected = i;
                return;
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Banner {
    pub line: Option<OnTheLine>,
    pub amount: Money,
    pub multiple: Multiple,
    pub label: String,
    pub jackpot: bool,
    pub t: f32,
}

impl Banner {
    pub fn shown_amount(&self) -> Money {
        let share = (self.t / BANNER_COUNT_SECS).min(1.0);
        (self.amount as f64 * f64::from(share)) as Money
    }
}

pub struct PrizeCard {
    pub fish: Fish,
    pub name: TextInput,
}

pub enum Popup {
    Picker(Picker),
    Flip(Flip),
    Banner(Banner),
    Confirm,
    Paytable(usize),
    Prize(PrizeCard),
}

pub enum View {
    Lobby { selected: usize },
    Table(Box<Table>),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Leave {
    Stay,
    Close,
}

pub struct CasinoState {
    pub view: View,
    pub popup: Option<Popup>,
    pub clock: f32,
}

impl CasinoState {
    pub fn open(game: Option<Game>, house: &dyn Teller, rng: &mut impl RngExt) -> Self {
        let mut state = Self {
            view: View::Lobby { selected: 0 },
            popup: None,
            clock: 0.0,
        };
        state.settle_lobby(house);
        if let Some(game) = game {
            state.view = View::Table(Box::new(Table::new(game, rng)));
        }
        state
    }

    pub fn table(&self) -> Option<&Table> {
        match &self.view {
            View::Table(table) => Some(table),
            View::Lobby { .. } => None,
        }
    }

    pub fn at_risk(&self) -> Vec<String> {
        let mut names = Vec::new();
        if let Some(table) = self.table() {
            names.extend(table.at_risk());
        }
        if let Some(Popup::Flip(flip)) = &self.popup {
            names.extend(flip.at_risk());
        }
        names
    }

    fn settle_lobby(&mut self, house: &dyn Teller) {
        let View::Lobby { selected } = &mut self.view else {
            return;
        };
        if !Game::ALL[*selected].is_open(house)
            && let Some(open) = Game::ALL.iter().position(|g| g.is_open(house))
        {
            *selected = open;
        }
    }

    pub fn key(
        &mut self,
        action: &InputAction,
        house: &mut dyn House,
        rng: &mut impl RngExt,
    ) -> Leave {
        if self.popup.is_some() {
            self.popup_key(action, house, rng);
            return Leave::Stay;
        }
        match &mut self.view {
            View::Lobby { selected } => match action {
                InputAction::Up => step_games(selected, false, house),
                InputAction::Down => step_games(selected, true, house),
                InputAction::Confirm => {
                    let game = Game::ALL[*selected];
                    if game.is_open(house) {
                        self.view = View::Table(Box::new(Table::new(game, rng)));
                    }
                }
                InputAction::Cancel | InputAction::Char('q') => return Leave::Close,
                _ => {}
            },
            View::Table(table) => {
                if table.face_down.is_some() {
                    return Leave::Stay;
                }
                if matches!(action, InputAction::Cancel | InputAction::Char('q')) {
                    if table.game == Game::Pufferfish && table.is_live() {
                        table.cash_out(house);
                        self.view = View::Lobby { selected: 0 };
                    } else if table.is_live() {
                        self.popup = Some(Popup::Confirm);
                        return Leave::Stay;
                    } else {
                        let selected = Game::ALL.iter().position(|g| *g == table.game).unwrap_or(0);
                        self.view = View::Lobby { selected };
                    }
                    self.settle_lobby(house);
                    return Leave::Stay;
                }
                let opened = table.key(action, house, rng);
                if opened.is_some() {
                    self.popup = opened;
                }
            }
        }
        Leave::Stay
    }

    fn popup_key(&mut self, action: &InputAction, house: &mut dyn House, rng: &mut impl RngExt) {
        let Some(popup) = self.popup.as_mut() else {
            return;
        };
        let table = match &mut self.view {
            View::Table(table) => Some(table.as_mut()),
            View::Lobby { .. } => None,
        };
        let close = match popup {
            Popup::Picker(picker) => match action {
                InputAction::Up => {
                    picker.step(false);
                    false
                }
                InputAction::Down => {
                    picker.step(true);
                    false
                }
                InputAction::Confirm => {
                    if picker.is_usable(picker.selected)
                        && let Some(table) = table
                    {
                        let entrant = picker.rows[picker.selected].clone();
                        table.picked(picker.purpose, entrant);
                    }
                    true
                }
                InputAction::Cancel | InputAction::Char('q') => true,
                _ => false,
            },
            Popup::Flip(flip) => match action {
                InputAction::Left => {
                    flip.call(Side::Left, house, rng);
                    false
                }
                InputAction::Right => {
                    flip.call(Side::Right, house, rng);
                    false
                }
                InputAction::Confirm | InputAction::Cancel | InputAction::Char('q') => {
                    match flip.collect(house) {
                        Some(end) => {
                            let big = Flip::ends_big(end);
                            let line = flip.line.clone();
                            let multiple = Multiple::whole(flip.multiple());
                            if let Some(table) = table {
                                table.after_flip(&line, true);
                            }
                            if big {
                                self.popup = Some(Popup::Banner(Banner {
                                    line: None,
                                    amount: line.cash,
                                    multiple,
                                    label: "Double or Nothing".to_string(),
                                    jackpot: false,
                                    t: 0.0,
                                }));
                                return;
                            }
                            true
                        }
                        None => false,
                    }
                }
                _ => false,
            },
            Popup::Banner(banner) => {
                if banner.t < BANNER_READY_SECS {
                    return;
                }
                match action {
                    InputAction::Char('d') => {
                        let line = banner.line.take();
                        self.popup = line
                            .and_then(|line| Flip::start(line, house))
                            .map(Popup::Flip);
                        if self.popup.is_some()
                            && let Some(table) = table
                        {
                            table.double = None;
                        }
                        return;
                    }
                    InputAction::Confirm | InputAction::Cancel | InputAction::Char('q') => true,
                    _ => false,
                }
            }
            Popup::Confirm => match action {
                InputAction::Confirm => {
                    if let Some(table) = table {
                        table.forfeit(house);
                        let selected = Game::ALL.iter().position(|g| *g == table.game).unwrap_or(0);
                        self.view = View::Lobby { selected };
                    }
                    true
                }
                InputAction::Cancel | InputAction::Char('q') => true,
                _ => false,
            },
            Popup::Paytable(scroll) => match action {
                InputAction::Up => {
                    *scroll = scroll.saturating_sub(1);
                    false
                }
                InputAction::Down => {
                    *scroll = (*scroll + 1).min(PAYTABLE_ROWS);
                    false
                }
                InputAction::Confirm
                | InputAction::Cancel
                | InputAction::Char('q')
                | InputAction::Char('p') => true,
                _ => false,
            },
            Popup::Prize(card) => match action {
                InputAction::Confirm => {
                    let name = if card.name.is_empty() {
                        card.fish.species.display_name().to_string()
                    } else {
                        crate::names::title_case(card.name.as_str())
                    };
                    let fish = card.fish.clone();
                    let landed = house.land(fish, name);
                    if let Some(table) = table {
                        table.netted(landed);
                    }
                    true
                }
                _ => {
                    card.name.handle_action(action);
                    false
                }
            },
        };
        if close {
            self.popup = None;
            self.settle_lobby(house);
        }
    }

    pub fn tick(&mut self, dt: f32, house: &mut dyn House, rng: &mut impl RngExt) {
        self.clock += dt;
        if let Some(popup) = self.popup.as_mut() {
            match popup {
                Popup::Banner(banner) => banner.t += dt,
                Popup::Flip(flip) => {
                    if let Some(FlipEnd::Lost) = flip.tick(dt, house) {
                        if let View::Table(table) = &mut self.view {
                            table.after_flip(&OnTheLine::default(), false);
                        }
                        self.popup = None;
                    }
                }
                _ => {}
            }
            return;
        }
        let View::Table(table) = &mut self.view else {
            return;
        };
        if let Some(popup) = table.tick(dt, house, rng) {
            self.popup = Some(popup);
        }
    }

    pub fn wants_text(&self) -> bool {
        matches!(self.popup, Some(Popup::Prize(_)))
    }
}

fn step_games(selected: &mut usize, down: bool, house: &dyn Teller) {
    let mut i = *selected;
    loop {
        i = if down {
            i + 1
        } else {
            match i.checked_sub(1) {
                Some(prev) => prev,
                None => return,
            }
        };
        if i >= Game::ALL.len() {
            return;
        }
        if Game::ALL[i].is_open(house) {
            *selected = i;
            return;
        }
    }
}

impl Table {
    pub fn new(game: Game, rng: &mut impl RngExt) -> Self {
        let play = match game {
            Game::Blackjack => Play::Blackjack(None),
            Game::Spins => Play::Spins(Spins::default()),
            Game::Pufferfish => Play::Pufferfish(Pufferfish::default()),
            Game::BubbleUp => Play::BubbleUp {
                risk: Risk::High,
                bubbles: Vec::new(),
                lit: Vec::new(),
            },
            Game::Derby => Play::Derby(Derby::new(rng)),
            Game::MysteryNet => Play::Net(Cast::new(Net::Small, rng)),
        };
        Self {
            game,
            seat: Seat::default(),
            round: None,
            result: None,
            flash: None,
            double: None,
            play,
            bait: None,
            clock: 0.0,
            face_down: None,
        }
    }

    pub fn is_live(&self) -> bool {
        match &self.play {
            Play::BubbleUp { bubbles, .. } => !bubbles.is_empty(),
            Play::Spins(spins) => !spins.is_idle(),
            _ => self.round.is_some(),
        }
    }

    pub fn at_risk(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .round
            .iter()
            .filter_map(|r| r.staked.fish().map(str::to_string))
            .collect();
        if let Play::BubbleUp { bubbles, .. } = &self.play {
            names.extend(
                bubbles
                    .iter()
                    .filter_map(|b| b.round.staked.fish().map(str::to_string)),
            );
        }
        names
    }

    pub fn fish_in_play(&self) -> Option<&str> {
        self.round.as_ref().and_then(|r| r.staked.fish())
    }

    pub fn can_double(&self, house: &dyn Teller) -> bool {
        !self.is_live()
            && self
                .double
                .as_ref()
                .is_some_and(|line| line.can_double(house))
    }

    fn key(
        &mut self,
        action: &InputAction,
        house: &mut dyn House,
        rng: &mut impl RngExt,
    ) -> Option<Popup> {
        if self.is_live() && self.game != Game::BubbleUp {
            return self.play_key(action, house, rng);
        }
        let purse = house.spendable();
        match action {
            InputAction::Left => match &mut self.play {
                Play::Net(cast) => *cast = Cast::new(cast.net.cheaper(), rng),
                _ => self.seat.lower(purse),
            },
            InputAction::Right => match &mut self.play {
                Play::Net(cast) => *cast = Cast::new(cast.net.dearer(), rng),
                _ => self.seat.raise(purse),
            },
            InputAction::Char('a') if self.game != Game::MysteryNet => self.seat.all_in(purse),
            InputAction::Up => match &mut self.play {
                Play::Pufferfish(puffer) => puffer.raise_target(),
                Play::BubbleUp { risk, .. } => *risk = risk.riskier(),
                Play::Derby(derby) => derby.pick_up(),
                _ => {}
            },
            InputAction::Down => match &mut self.play {
                Play::Pufferfish(puffer) => puffer.lower_target(),
                Play::BubbleUp { risk, .. } => *risk = risk.safer(),
                Play::Derby(derby) => derby.pick_down(),
                Play::Spins(_) => return self.begin(house, rng),
                _ => {}
            },
            InputAction::Tab => {
                let purpose = match self.game {
                    Game::MysteryNet => Purpose::Bait,
                    _ => Purpose::Stake,
                };
                return Picker::new(purpose, house).map(Popup::Picker);
            }
            InputAction::Char('p') if matches!(self.game, Game::Spins | Game::BubbleUp) => {
                return Some(Popup::Paytable(0));
            }
            InputAction::Char('d') => {
                if self.can_double(house)
                    && let Some(line) = self.double.take()
                {
                    return Flip::start(line, house).map(Popup::Flip);
                }
            }
            InputAction::Confirm => return self.begin(house, rng),
            _ => {}
        }
        None
    }

    fn play_key(
        &mut self,
        action: &InputAction,
        house: &mut dyn House,
        rng: &mut impl RngExt,
    ) -> Option<Popup> {
        match &mut self.play {
            Play::Blackjack(Some(game)) => {
                let play = match action {
                    InputAction::Down => Move::Hit,
                    InputAction::Confirm => Move::Stand,
                    InputAction::Char('d') => Move::Double,
                    InputAction::Char('s') => Move::Split,
                    _ => return None,
                };
                if !game.can(play) {
                    return None;
                }
                let extra = game.extra_for(play);
                if extra > 0 {
                    let round = self.round.as_mut()?;
                    if !round.add(house, extra) {
                        return None;
                    }
                }
                game.play(play, rng);
                None
            }
            Play::Pufferfish(puffer) => {
                if matches!(action, InputAction::Confirm | InputAction::Down) && puffer.is_puffing()
                {
                    return self.cash_out(house);
                }
                None
            }
            _ => None,
        }
    }

    fn cash_out(&mut self, house: &mut dyn House) -> Option<Popup> {
        let Play::Pufferfish(puffer) = &mut self.play else {
            return None;
        };
        let at = puffer.cash_out()?;
        self.settle_multiple(at, format!("cashed out at {}", at.label()), house);
        None
    }

    fn begin(&mut self, house: &mut dyn House, rng: &mut impl RngExt) -> Option<Popup> {
        if let Play::Net(_) = self.play {
            return self.cast_the_net(house, rng);
        }
        if let Play::BubbleUp { bubbles, .. } = &self.play
            && bubbles.len() >= MAX_IN_FLIGHT
        {
            return None;
        }
        if let Play::Derby(derby) = &mut self.play
            && derby.winner().is_some()
        {
            derby.line_up(rng);
        }
        if !self.seat.can_stake(house) {
            return None;
        }
        let round = self.seat.take(house)?;
        let value = round.staked.value();
        self.result = None;
        self.double = None;
        match &mut self.play {
            Play::Blackjack(slot) => *slot = Some(Blackjack::deal(value)),
            Play::Spins(spins) => {
                house.casino().feed_the_pot(value);
                spins.spin(rng);
            }
            Play::Pufferfish(puffer) => puffer.puff(rng),
            Play::BubbleUp { risk, bubbles, .. } => {
                bubbles.push(Bubble::blow(*risk, round, rng));
                return None;
            }
            Play::Derby(derby) => derby.start(rng),
            Play::Net(_) => {}
        }
        self.round = Some(round);
        None
    }

    fn cast_the_net(&mut self, house: &mut dyn House, rng: &mut impl RngExt) -> Option<Popup> {
        let Play::Net(cast) = &mut self.play else {
            return None;
        };
        if !house.room_for_a_prize() {
            return None;
        }
        let price = cast.net.price();
        if let Some(bait) = self.bait.take() {
            let worth = house.entrant(&bait).map_or(0, |e| e.worth);
            if house.spendable().saturating_add(worth) < price {
                self.bait = Some(bait);
                return None;
            }
            house.sell_as_bait(&bait);
        }
        if !house.wager(price) {
            return None;
        }
        self.result = None;
        self.double = None;
        cast.cast(rng);
        self.round = Some(Round {
            staked: super::seat::Staked::Cash(price),
            extra: 0,
        });
        None
    }

    fn picked(&mut self, purpose: Purpose, entrant: Entrant) {
        match purpose {
            Purpose::Stake => self.seat.pick_fish(entrant.name),
            Purpose::Bait => self.bait = Some(entrant.name),
        }
    }

    fn tick(&mut self, dt: f32, house: &mut dyn House, rng: &mut impl RngExt) -> Option<Popup> {
        self.clock += dt;
        if let Some((_, t)) = &mut self.flash {
            *t -= dt;
            if *t <= 0.0 {
                self.flash = None;
            }
        }
        self.tick_play(dt, house, rng);
        let (_, wait) = self.face_down.as_mut()?;
        *wait -= dt;
        if *wait > 0.0 {
            return None;
        }
        let (card, _) = self.face_down.take()?;
        Some(card.turn_over(rng))
    }

    fn deal_face_down(&mut self, card: Card) {
        let keep = self
            .face_down
            .as_ref()
            .is_some_and(|(held, _)| !card.outranks(held));
        if keep {
            return;
        }
        let wait = self.face_down.take().map_or(REVEAL_SECS, |(_, wait)| wait);
        self.face_down = Some((card, wait));
    }

    fn tick_play(&mut self, dt: f32, house: &mut dyn House, rng: &mut impl RngExt) {
        match &mut self.play {
            Play::Blackjack(Some(game)) => {
                if self.round.is_none() {
                    return;
                }
                game.tick(dt, rng);
                let Some(returned) = game.returned() else {
                    return;
                };
                let label = game.label();
                self.settle_money(returned, label, house);
            }
            Play::Blackjack(None) => {}
            Play::Spins(spins) => {
                let Some(outcome) = spins.tick(dt, rng) else {
                    return;
                };
                let Some(round) = self.round.as_ref() else {
                    return;
                };
                let value = round.staked.value();
                let mut returned = outcome.multiple().of(value);
                let jackpot = outcome.full;
                if jackpot {
                    returned = returned.saturating_add(house.casino().empty_the_pot());
                }
                let label = outcome.label();
                self.settle_money(returned, label, house);
                if jackpot && let Some((Card::Win(banner), _)) = &mut self.face_down {
                    banner.jackpot = true;
                }
            }
            Play::Pufferfish(puffer) => {
                let Some(paid) = puffer.tick(dt) else {
                    return;
                };
                let label = if paid == Multiple::ZERO {
                    format!("popped at {}", puffer.now().label())
                } else {
                    format!("cashed out at {}", paid.label())
                };
                self.settle_multiple(paid, label, house);
            }
            Play::BubbleUp { bubbles, lit, .. } => {
                for (_, _, t) in lit.iter_mut() {
                    *t -= dt;
                }
                lit.retain(|(_, _, t)| *t > 0.0);
                for bubble in bubbles.iter_mut() {
                    bubble.tick(dt);
                }
                let (landed, flying): (Vec<Bubble>, Vec<Bubble>) =
                    bubbles.drain(..).partition(Bubble::has_landed);
                *bubbles = flying;
                let mut cards = Vec::new();
                for bubble in landed {
                    lit.push((bubble.risk, bubble.shell(), SHELL_FLASH_SECS));
                    let pays = bubble.pays();
                    let returned = pays.of(bubble.round.staked.value());
                    let result = bubble.round.settle(returned, pays.label(), house);
                    self.flash = Some((flash_of(&result), FLASH_SECS));
                    if result.is_big() {
                        cards.push(Card::Win(banner_of(&result)));
                    }
                    self.result = Some(result);
                }
                for card in cards {
                    self.deal_face_down(card);
                }
            }
            Play::Derby(derby) => {
                let Some(won) = derby.tick(dt) else {
                    return;
                };
                if self.round.is_none() {
                    return;
                }
                let odds = derby.picked().odds();
                let label = derby
                    .winner()
                    .map(|w| format!("{} wins", derby.lanes[w].name()))
                    .unwrap_or_default();
                let pays = if won { odds } else { Multiple::ZERO };
                self.settle_multiple(pays, label, house);
            }
            Play::Net(cast) => match cast.tick(dt) {
                None => {}
                Some(Prize::Food) => {
                    house.give_food(super::net::food_portion());
                    self.netted(None);
                }
                Some(Prize::Fish(species)) => self.deal_face_down(Card::Catch(species)),
            },
        }
    }

    fn netted(&mut self, landed: Option<String>) {
        let Some(round) = self.round.take() else {
            return;
        };
        let label = match &landed {
            Some(name) => format!("{name} in the net"),
            None => format!("{} pellets of food", super::net::food_portion()),
        };
        self.result = Some(RoundResult {
            label,
            multiple: Multiple::ZERO,
            verdict: Verdict::Netted {
                fish: landed.clone(),
                price: round.staked.value(),
            },
        });
        self.flash = Some((
            if landed.is_some() {
                Flash::Win
            } else {
                Flash::Loss
            },
            FLASH_SECS,
        ));
        self.double = landed.map(|name| OnTheLine {
            cash: 0,
            fish: vec![name],
        });
    }

    fn settle_multiple(&mut self, pays: Multiple, label: String, house: &mut dyn House) {
        let Some(round) = self.round.as_ref() else {
            return;
        };
        let value = round.on_the_line();
        self.settle_money(pays.of(value), label, house);
    }

    fn settle_money(&mut self, returned: Money, label: String, house: &mut dyn House) {
        let Some(round) = self.round.take() else {
            return;
        };
        let result = round.settle(returned, label, house);
        self.flash = Some((flash_of(&result), FLASH_SECS));
        self.double = result.on_the_line();
        if result.is_big() {
            self.double = None;
            self.deal_face_down(Card::Win(banner_of(&result)));
        }
        self.result = Some(result);
    }

    fn forfeit(&mut self, house: &mut dyn House) {
        if let Play::BubbleUp { bubbles, .. } = &mut self.play {
            for bubble in bubbles.drain(..) {
                bubble.round.settle(0, String::new(), house);
            }
        }
        if let Some(round) = self.round.take() {
            if let Play::Net(_) = self.play {
                house.casino().lost(round.staked.value());
            } else {
                round.settle(0, String::new(), house);
            }
        }
        if let Play::Spins(spins) = &mut self.play {
            *spins = Spins::default();
        }
    }

    fn after_flip(&mut self, line: &OnTheLine, collected: bool) {
        self.double = None;
        self.flash = Some((if collected { Flash::Win } else { Flash::Loss }, FLASH_SECS));
        self.result = Some(RoundResult {
            label: if collected {
                "Double or Nothing collected".to_string()
            } else {
                "Double or Nothing lost".to_string()
            },
            multiple: Multiple::ZERO,
            verdict: Verdict::Doubled {
                cash: line.cash,
                fish: line.fish.len(),
                collected,
            },
        });
    }
}

fn flash_of(result: &RoundResult) -> Flash {
    if result.is_big() {
        return Flash::Big;
    }
    if result.won() {
        Flash::Win
    } else {
        Flash::Loss
    }
}

fn banner_of(result: &RoundResult) -> Banner {
    let amount = match &result.verdict {
        Verdict::Cash { returned, .. } => *returned,
        Verdict::FishHome { winnings, .. } => *winnings,
        _ => 0,
    };
    Banner {
        line: result.on_the_line(),
        amount,
        multiple: result.multiple,
        label: result.label.clone(),
        jackpot: false,
        t: 0.0,
    }
}
