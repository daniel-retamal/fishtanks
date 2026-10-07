use std::collections::VecDeque;

use rand::RngExt;

use crate::economy::Money;

const DEAL_SECS: f32 = 0.3;
const DEALER_SECS: f32 = 0.6;
const BUST: u32 = 21;
const DEALER_STANDS: u32 = 17;
const ACE_BONUS: u32 = 10;
const FACE_VALUE: u32 = 10;
const BLACKJACK_CARDS: usize = 2;
const MAX_HANDS: usize = 2;
pub const SAFE_TOTAL: u32 = 11;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rank {
    Ace,
    Number(u8),
    Jack,
    Queen,
    King,
}

impl Rank {
    const ALL: [Rank; 13] = [
        Rank::Ace,
        Rank::Number(2),
        Rank::Number(3),
        Rank::Number(4),
        Rank::Number(5),
        Rank::Number(6),
        Rank::Number(7),
        Rank::Number(8),
        Rank::Number(9),
        Rank::Number(10),
        Rank::Jack,
        Rank::Queen,
        Rank::King,
    ];

    pub fn value(self) -> u32 {
        match self {
            Rank::Ace => 1,
            Rank::Number(n) => u32::from(n),
            Rank::Jack | Rank::Queen | Rank::King => FACE_VALUE,
        }
    }

    pub fn label(self) -> String {
        match self {
            Rank::Ace => "A".to_string(),
            Rank::Number(n) => n.to_string(),
            Rank::Jack => "J".to_string(),
            Rank::Queen => "Q".to_string(),
            Rank::King => "K".to_string(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Suit {
    Spades,
    Hearts,
    Diamonds,
    Clubs,
}

impl Suit {
    const ALL: [Suit; 4] = [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs];

    pub fn glyph(self) -> char {
        match self {
            Suit::Spades => '♠',
            Suit::Hearts => '♥',
            Suit::Diamonds => '♦',
            Suit::Clubs => '♣',
        }
    }

    pub fn is_red(self) -> bool {
        matches!(self, Suit::Hearts | Suit::Diamonds)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
    pub face_up: bool,
    pub age: f32,
}

impl Card {
    fn draw(face_up: bool, rng: &mut impl RngExt) -> Card {
        Card {
            rank: Rank::ALL[rng.random_range(0..Rank::ALL.len())],
            suit: Suit::ALL[rng.random_range(0..Suit::ALL.len())],
            face_up,
            age: 0.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Total {
    pub points: u32,
    pub soft: bool,
}

pub fn total(cards: &[Card], hidden_too: bool) -> Total {
    let shown = cards.iter().filter(|card| hidden_too || card.face_up);
    let mut points = 0;
    let mut ace = false;
    for card in shown {
        points += card.rank.value();
        ace |= card.rank == Rank::Ace;
    }
    if ace && points + ACE_BONUS <= BUST {
        return Total {
            points: points + ACE_BONUS,
            soft: true,
        };
    }
    Total {
        points,
        soft: false,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HandOutcome {
    Blackjack,
    Win,
    Push,
    Lose,
    Bust,
}

impl HandOutcome {
    fn returned(self, bet: Money) -> Money {
        match self {
            HandOutcome::Blackjack => bet.saturating_mul(5) / 2,
            HandOutcome::Win => bet.saturating_mul(2),
            HandOutcome::Push => bet,
            HandOutcome::Lose | HandOutcome::Bust => 0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            HandOutcome::Blackjack => "Blackjack",
            HandOutcome::Win => "You win",
            HandOutcome::Push => "Push",
            HandOutcome::Lose => "Tollomind wins",
            HandOutcome::Bust => "Bust",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Hand {
    pub cards: Vec<Card>,
    pub bet: Money,
    pub done: bool,
    pub outcome: Option<HandOutcome>,
    split: bool,
}

impl Hand {
    fn new(bet: Money) -> Self {
        Self {
            cards: Vec::new(),
            bet,
            done: false,
            outcome: None,
            split: false,
        }
    }

    pub fn total(&self) -> Total {
        total(&self.cards, true)
    }

    fn is_blackjack(&self) -> bool {
        !self.split && self.cards.len() == BLACKJACK_CARDS && self.total().points == BUST
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Dealing,
    Player,
    Dealer,
    Finished { returned: Money },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Step {
    ToHand(usize),
    ToDealer { face_up: bool },
    Peek,
    Reveal,
    DealerPlays,
    Finish,
}

impl Step {
    fn pause(self) -> f32 {
        match self {
            Step::ToHand(_) | Step::ToDealer { .. } | Step::Peek => DEAL_SECS,
            Step::Reveal | Step::DealerPlays | Step::Finish => DEALER_SECS,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Move {
    Hit,
    Stand,
    Double,
    Split,
}

#[derive(Clone, Debug)]
pub struct Blackjack {
    pub dealer: Vec<Card>,
    pub hands: Vec<Hand>,
    pub active: usize,
    pub phase: Phase,
    queue: VecDeque<Step>,
    wait: f32,
}

impl Blackjack {
    pub fn deal(bet: Money) -> Self {
        let queue = VecDeque::from([
            Step::ToHand(0),
            Step::ToDealer { face_up: true },
            Step::ToHand(0),
            Step::ToDealer { face_up: false },
            Step::Peek,
        ]);
        Self {
            dealer: Vec::new(),
            hands: vec![Hand::new(bet)],
            active: 0,
            phase: Phase::Dealing,
            queue,
            wait: DEAL_SECS,
        }
    }

    pub fn tick(&mut self, dt: f32, rng: &mut impl RngExt) {
        for card in self
            .dealer
            .iter_mut()
            .chain(self.hands.iter_mut().flat_map(|h| h.cards.iter_mut()))
        {
            card.age += dt;
        }
        self.wait -= dt;
        while self.wait <= 0.0 {
            let Some(step) = self.queue.pop_front() else {
                self.wait = 0.0;
                return;
            };
            self.run(step, rng);
            self.wait += step.pause();
        }
    }

    pub fn finish_now(&mut self, rng: &mut impl RngExt) {
        while !self.queue.is_empty() {
            self.tick(DEALER_SECS, rng);
        }
    }

    fn run(&mut self, step: Step, rng: &mut impl RngExt) {
        match step {
            Step::ToHand(index) => self.hands[index].cards.push(Card::draw(true, rng)),
            Step::ToDealer { face_up } => self.dealer.push(Card::draw(face_up, rng)),
            Step::Peek => self.peek(),
            Step::Reveal => {
                if let Some(hole) = self.dealer.get_mut(1) {
                    hole.face_up = true;
                    hole.age = 0.0;
                }
            }
            Step::DealerPlays => {
                let everyone_bust = self.hands.iter().all(|h| h.total().points > BUST);
                if !everyone_bust && total(&self.dealer, true).points < DEALER_STANDS {
                    self.dealer.push(Card::draw(true, rng));
                    self.queue.push_back(Step::DealerPlays);
                } else {
                    self.queue.push_back(Step::Finish);
                }
            }
            Step::Finish => self.finish(),
        }
    }

    fn peek(&mut self) {
        let dealer_blackjack = total(&self.dealer, true).points == BUST;
        if dealer_blackjack || self.hands[0].is_blackjack() {
            self.phase = Phase::Dealer;
            self.queue.extend([Step::Reveal, Step::Finish]);
            return;
        }
        self.phase = Phase::Player;
    }

    fn finish(&mut self) {
        let dealer = total(&self.dealer, true).points;
        let dealer_blackjack = dealer == BUST && self.dealer.len() == BLACKJACK_CARDS;
        let mut returned: Money = 0;
        for hand in &mut self.hands {
            let points = hand.total().points;
            let outcome = if points > BUST {
                HandOutcome::Bust
            } else if hand.is_blackjack() && !dealer_blackjack {
                HandOutcome::Blackjack
            } else if dealer_blackjack && !hand.is_blackjack() {
                HandOutcome::Lose
            } else if dealer > BUST || points > dealer {
                HandOutcome::Win
            } else if points == dealer {
                HandOutcome::Push
            } else {
                HandOutcome::Lose
            };
            hand.outcome = Some(outcome);
            hand.done = true;
            returned = returned.saturating_add(outcome.returned(hand.bet));
        }
        self.phase = Phase::Finished { returned };
    }

    pub fn returned(&self) -> Option<Money> {
        match self.phase {
            Phase::Finished { returned } => Some(returned),
            _ => None,
        }
    }

    pub fn label(&self) -> String {
        if self.hands.len() == 1 {
            let outcome = self.hands[0].outcome.unwrap_or(HandOutcome::Push);
            if outcome == HandOutcome::Win && total(&self.dealer, true).points > BUST {
                return "Tollomind busts".to_string();
            }
            return outcome.label().to_string();
        }
        let won = self
            .hands
            .iter()
            .filter(|h| matches!(h.outcome, Some(HandOutcome::Win | HandOutcome::Blackjack)))
            .count();
        format!("{won} of {} hands won", self.hands.len())
    }

    pub fn active_hand(&self) -> &Hand {
        &self.hands[self.active]
    }

    pub fn can(&self, play: Move) -> bool {
        if self.phase != Phase::Player {
            return false;
        }
        let hand = self.active_hand();
        match play {
            Move::Hit | Move::Stand => true,
            Move::Double => hand.cards.len() == BLACKJACK_CARDS,
            Move::Split => {
                self.hands.len() < MAX_HANDS
                    && hand.cards.len() == BLACKJACK_CARDS
                    && hand.cards[0].rank.value() == hand.cards[1].rank.value()
            }
        }
    }

    pub fn extra_for(&self, play: Move) -> Money {
        match play {
            Move::Double | Move::Split => self.active_hand().bet,
            Move::Hit | Move::Stand => 0,
        }
    }

    pub fn play(&mut self, play: Move, rng: &mut impl RngExt) {
        if !self.can(play) {
            return;
        }
        let active = self.active;
        match play {
            Move::Hit => {
                self.hands[active].cards.push(Card::draw(true, rng));
                if self.hands[active].total().points >= BUST {
                    self.next_hand();
                }
            }
            Move::Stand => self.next_hand(),
            Move::Double => {
                let hand = &mut self.hands[active];
                hand.bet = hand.bet.saturating_mul(2);
                hand.cards.push(Card::draw(true, rng));
                self.next_hand();
            }
            Move::Split => {
                let hand = &mut self.hands[active];
                let moved = hand.cards.pop().expect("a pair has two cards");
                hand.split = true;
                hand.cards.push(Card::draw(true, rng));
                let mut second = Hand::new(hand.bet);
                second.split = true;
                second.cards.push(moved);
                second.cards.push(Card::draw(true, rng));
                self.hands.push(second);
            }
        }
    }

    fn next_hand(&mut self) {
        self.hands[self.active].done = true;
        if self.active + 1 < self.hands.len() {
            self.active += 1;
            return;
        }
        self.phase = Phase::Dealer;
        self.queue.extend([Step::Reveal, Step::DealerPlays]);
        self.wait = self.wait.max(0.0);
    }

    pub fn basic_move(&self) -> Move {
        let hand = self.active_hand();
        let up = self.dealer.first().map_or(FACE_VALUE, |c| c.rank.value());
        let up = if up == 1 { 11 } else { up };
        let Total { points, soft } = hand.total();
        if self.can(Move::Split) {
            let pair = hand.cards[0].rank.value();
            let split = match pair {
                1 | 8 => true,
                9 => ![7, 10, 11].contains(&up),
                2 | 3 | 7 => up <= 7,
                6 => up <= 6,
                4 => up == 5 || up == 6,
                _ => false,
            };
            if split {
                return Move::Split;
            }
        }
        let double = self.can(Move::Double);
        if soft {
            return match points {
                19.. => Move::Stand,
                18 if double && (3..=6).contains(&up) => Move::Double,
                18 if up <= 8 => Move::Stand,
                17 if double && (3..=6).contains(&up) => Move::Double,
                15 | 16 if double && (4..=6).contains(&up) => Move::Double,
                13 | 14 if double && (5..=6).contains(&up) => Move::Double,
                _ => Move::Hit,
            };
        }
        match points {
            17.. => Move::Stand,
            13..=16 if up <= 6 => Move::Stand,
            12 if (4..=6).contains(&up) => Move::Stand,
            11 if double => Move::Double,
            10 if double && up <= 9 => Move::Double,
            9 if double && (3..=6).contains(&up) => Move::Double,
            _ => Move::Hit,
        }
    }

    pub fn total_bet(&self) -> Money {
        self.hands.iter().map(|h| h.bet).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::SmallRng};

    const HANDS: usize = 200_000;

    fn play_basic(rng: &mut SmallRng) -> (Money, Money) {
        let mut game = Blackjack::deal(100);
        game.finish_now(rng);
        while game.phase == Phase::Player {
            let play = game.basic_move();
            game.play(play, rng);
            game.finish_now(rng);
        }
        (game.returned().expect("a hand ends"), game.total_bet())
    }

    #[test]
    fn basic_strategy_loses_about_half_a_percent() {
        let mut rng = SmallRng::seed_from_u64(7);
        let (mut returned, mut bet) = (0, 0);
        for _ in 0..HANDS {
            let (r, b) = play_basic(&mut rng);
            returned += r;
            bet += b;
        }
        let rtp = returned as f64 / bet as f64;
        assert!((0.985..1.0).contains(&rtp), "{rtp}");
    }

    #[test]
    fn an_ace_and_a_ten_count_twenty_one_soft() {
        let card = |rank| Card {
            rank,
            suit: Suit::Spades,
            face_up: true,
            age: 0.0,
        };
        let hand = [card(Rank::Ace), card(Rank::King)];
        assert_eq!(
            total(&hand, true),
            Total {
                points: 21,
                soft: true
            }
        );
        let busting = [card(Rank::Ace), card(Rank::King), card(Rank::Number(5))];
        assert_eq!(total(&busting, true).points, 16);
    }
}
