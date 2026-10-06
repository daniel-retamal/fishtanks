use std::cmp::Ordering;

use crate::ui::fields::FieldKind;
use crate::ui::table::NOTHING;

pub const SORT_WORD: &str = "sort";
pub const ALL_WORD: &str = "all";
pub const ALIVE: &str = "Alive";
pub const DEAD: &str = "Dead";
pub const ASCENDING_MARK: &str = "▲";
pub const DESCENDING_MARK: &str = "▼";
const KEY_SEPARATOR: char = ':';
const DESCENDING_PREFIX: char = '-';
const QUOTE: char = '"';
const TANK_ALIAS: &str = "tank";
const GRAMS_PER_KILO: f64 = 1000.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Name,
    Status,
    Species,
    Display,
    Weight,
    Worth,
    Fed,
    Fishtank,
    Field(FieldKind),
}

impl Column {
    pub const FIXED: [Column; 8] = [
        Column::Name,
        Column::Status,
        Column::Species,
        Column::Display,
        Column::Weight,
        Column::Worth,
        Column::Fed,
        Column::Fishtank,
    ];

    pub fn header(self) -> &'static str {
        match self {
            Column::Name => "Name",
            Column::Status => "Status",
            Column::Species => "Species",
            Column::Display => "Display",
            Column::Weight => "Weight",
            Column::Worth => crate::ui::fields::WORTH_HEADER,
            Column::Fed => crate::ui::fields::FED_HEADER,
            Column::Fishtank => "Fishtank",
            Column::Field(kind) => kind.header(),
        }
    }

    pub fn key(self) -> String {
        self.header()
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .map(|ch| ch.to_ascii_lowercase())
            .collect()
    }

    pub fn is_queryable(self) -> bool {
        self != Column::Display
    }

    pub fn every() -> impl Iterator<Item = Column> {
        Column::FIXED
            .into_iter()
            .chain(FieldKind::all().iter().map(|&kind| Column::Field(kind)))
            .filter(|column| column.is_queryable())
    }

    pub fn keys() -> Vec<String> {
        let mut keys: Vec<String> = Vec::new();
        for column in Column::every() {
            let key = column.key();
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        keys
    }

    pub fn parse(word: &str) -> Option<Column> {
        let word = word.to_ascii_lowercase();
        if word == TANK_ALIAS {
            return Some(Column::Fishtank);
        }
        Column::every().find(|column| column.key() == word)
    }
}

#[derive(Clone, PartialEq)]
pub struct Cell {
    pub text: String,
    pub number: Option<f64>,
}

impl Cell {
    pub fn text(text: impl Into<String>) -> Self {
        let text = text.into();
        let number = amount(&text);
        Self { text, number }
    }

    pub fn counted(text: impl Into<String>, number: Option<f64>) -> Self {
        Self {
            text: text.into(),
            number,
        }
    }

    pub fn ranked(&self, other: &Cell, descending: bool) -> Ordering {
        let order = self.order(other);
        if descending && !self.is_missing() && !other.is_missing() {
            return order.reverse();
        }
        order
    }

    fn is_missing(&self) -> bool {
        self.number.is_none() && (self.text.is_empty() || self.text == NOTHING)
    }

    pub fn order(&self, other: &Cell) -> Ordering {
        match (self.is_missing(), other.is_missing()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Greater,
            (false, true) => return Ordering::Less,
            (false, false) => {}
        }
        match (self.number, other.number) {
            (Some(a), Some(b)) => a.total_cmp(&b),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => self.text.to_lowercase().cmp(&other.text.to_lowercase()),
        }
    }
}

pub fn amount(text: &str) -> Option<f64> {
    let cleaned: String = text
        .trim()
        .trim_start_matches('$')
        .chars()
        .filter(|&ch| ch != ',')
        .collect();
    let lower = cleaned.to_ascii_lowercase();
    if let Some(kilos) = lower.strip_suffix("kg") {
        return kilos
            .trim()
            .parse::<f64>()
            .ok()
            .map(|kg| kg * GRAMS_PER_KILO);
    }
    let bare = lower
        .strip_suffix('g')
        .or_else(|| lower.strip_suffix('%'))
        .unwrap_or(&lower);
    bare.trim().parse::<f64>().ok().filter(|n| n.is_finite())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Comparison {
    AtLeast,
    AtMost,
    Above,
    Below,
}

impl Comparison {
    const ALL: [(&'static str, Comparison); 4] = [
        (">=", Comparison::AtLeast),
        ("<=", Comparison::AtMost),
        (">", Comparison::Above),
        ("<", Comparison::Below),
    ];

    fn split(text: &str) -> Option<(Comparison, f64)> {
        let (sign, comparison) = Comparison::ALL
            .iter()
            .find(|(sign, _)| text.starts_with(sign))?;
        Some((*comparison, amount(&text[sign.len()..])?))
    }

    fn holds(self, value: f64, bound: f64) -> bool {
        match self {
            Comparison::AtLeast => value >= bound,
            Comparison::AtMost => value <= bound,
            Comparison::Above => value > bound,
            Comparison::Below => value < bound,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Filter {
    pub column: Column,
    pub text: String,
}

impl Filter {
    pub fn new(column: Column, text: impl Into<String>) -> Self {
        Self {
            column,
            text: text.into(),
        }
    }

    pub fn exactly(column: Column, text: &str) -> Self {
        Self::new(column, format!("{QUOTE}{text}{QUOTE}"))
    }

    fn exact(&self) -> Option<&str> {
        self.text
            .strip_prefix(QUOTE)
            .and_then(|inner| inner.strip_suffix(QUOTE))
    }

    pub fn matches(&self, cell: &Cell) -> bool {
        if let Some(exact) = self.exact() {
            return cell.text.to_lowercase() == exact.to_lowercase();
        }
        if let Some((comparison, bound)) = Comparison::split(self.text.trim()) {
            return cell
                .number
                .is_some_and(|value| comparison.holds(value, bound));
        }
        cell.text
            .to_lowercase()
            .contains(&self.text.trim().to_lowercase())
    }

    fn word(&self) -> String {
        format!("{}{KEY_SEPARATOR}{}", self.column.key(), self.text)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Sort {
    pub column: Column,
    pub descending: bool,
}

impl Sort {
    pub fn mark(self) -> &'static str {
        if self.descending {
            DESCENDING_MARK
        } else {
            ASCENDING_MARK
        }
    }

    fn word(self) -> String {
        let sign = if self.descending {
            DESCENDING_PREFIX.to_string()
        } else {
            String::new()
        };
        format!("{SORT_WORD}{KEY_SEPARATOR}{sign}{}", self.column.key())
    }

    fn parse(text: &str) -> Option<Sort> {
        let (descending, key) = match text.strip_prefix(DESCENDING_PREFIX) {
            Some(key) => (true, key),
            None => (false, text),
        };
        Some(Sort {
            column: Column::parse(key)?,
            descending,
        })
    }
}

#[derive(Clone, Default, PartialEq)]
pub struct IndexQuery {
    pub all: bool,
    pub filters: Vec<Filter>,
    pub sort: Option<Sort>,
}

pub enum Term {
    All,
    Filter(Filter),
    Sort(Sort),
    Name(String),
}

impl Term {
    pub fn parse(word: &str) -> Option<Term> {
        if word.eq_ignore_ascii_case(ALL_WORD) {
            return Some(Term::All);
        }
        if let Some(name) = word
            .strip_prefix(QUOTE)
            .and_then(|inner| inner.strip_suffix(QUOTE))
        {
            return (!name.is_empty()).then(|| Term::Name(name.to_string()));
        }
        let Some((key, value)) = word.split_once(KEY_SEPARATOR) else {
            return Some(Term::Name(word.to_string()));
        };
        if value.is_empty() {
            return None;
        }
        if key.eq_ignore_ascii_case(SORT_WORD) {
            return Sort::parse(value).map(Term::Sort);
        }
        Some(Term::Filter(Filter::new(Column::parse(key)?, value)))
    }
}

pub fn words(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    for ch in line.chars() {
        if ch == QUOTE {
            quoted = !quoted;
        }
        if ch.is_whitespace() && !quoted {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
            continue;
        }
        word.push(ch);
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

impl IndexQuery {
    pub fn add(&mut self, term: Term) {
        match term {
            Term::All => self.all = true,
            Term::Filter(filter) => self.set_filter(filter.column, filter.text),
            Term::Sort(sort) => self.sort = Some(sort),
            Term::Name(name) => self.set_filter(
                Column::Fishtank,
                Filter::exactly(Column::Fishtank, &name).text,
            ),
        }
    }

    pub fn shows_the_dead(&self) -> bool {
        self.filters
            .iter()
            .any(|filter| matches!(filter.column, Column::Fishtank | Column::Status))
    }

    pub fn mentions(&self, column: Column) -> bool {
        self.columns().any(|mentioned| mentioned == column)
    }

    pub fn filter_text(&self, column: Column) -> Option<&str> {
        self.filters
            .iter()
            .find(|filter| filter.column == column)
            .map(|filter| filter.text.as_str())
    }

    pub fn set_filter(&mut self, column: Column, text: impl Into<String>) {
        let text = text.into();
        let at = self
            .filters
            .iter()
            .position(|filter| filter.column == column);
        match (at, text.trim().is_empty()) {
            (Some(at), true) => {
                self.filters.remove(at);
            }
            (Some(at), false) => self.filters[at].text = text,
            (None, true) => {}
            (None, false) => self.filters.push(Filter::new(column, text)),
        }
    }

    pub fn cycle_sort(&mut self, column: Column) {
        self.sort = match self.sort {
            Some(sort) if sort.column == column && !sort.descending => Some(Sort {
                column,
                descending: true,
            }),
            Some(sort) if sort.column == column => None,
            _ => Some(Sort {
                column,
                descending: false,
            }),
        };
    }

    pub fn is_narrowed(&self) -> bool {
        !self.filters.is_empty() || self.sort.is_some()
    }

    pub fn clear(&mut self) {
        self.filters.clear();
        self.sort = None;
    }

    pub fn columns(&self) -> impl Iterator<Item = Column> + '_ {
        self.filters
            .iter()
            .map(|filter| filter.column)
            .chain(self.sort.map(|sort| sort.column))
    }

    pub fn words(&self) -> Vec<String> {
        let all = self.all.then(|| ALL_WORD.to_string());
        all.into_iter()
            .chain(self.filters.iter().map(Filter::word))
            .chain(self.sort.map(Sort::word))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter(column: Column, text: &str) -> Filter {
        Filter::new(column, text)
    }

    #[test]
    fn a_filter_finds_any_part_of_a_cell_whatever_its_case() {
        assert!(filter(Column::Species, "SAL").matches(&Cell::text("Salmon")));
        assert!(!filter(Column::Species, "koi").matches(&Cell::text("Salmon")));
    }

    #[test]
    fn a_quoted_filter_wants_the_whole_cell() {
        let coral = Filter::exactly(Column::Fishtank, "Coral");
        assert!(coral.matches(&Cell::text("coral")));
        assert!(!coral.matches(&Cell::text("Coral II")));
    }

    #[test]
    fn a_comparison_reads_the_cell_as_the_amount_it_shows() {
        let heavy = filter(Column::Weight, ">1kg");
        assert!(heavy.matches(&Cell::counted("1.2kg", Some(1200.0))));
        assert!(!heavy.matches(&Cell::counted("900g", Some(900.0))));
        assert!(filter(Column::Worth, "<=$1,000").matches(&Cell::text("$1,000")));
        assert!(!filter(Column::Worth, ">5").matches(&Cell::text("-")));
    }

    #[test]
    fn an_empty_cell_sorts_last_whichever_way() {
        let empty = Cell::text(NOTHING);
        let cheap = Cell::text("$5");
        assert_eq!(empty.order(&cheap), Ordering::Greater);
        assert_eq!(cheap.order(&empty), Ordering::Less);
        assert_eq!(Cell::text("$5").order(&Cell::text("$40")), Ordering::Less);
        assert_eq!(
            Cell::text("koi").order(&Cell::text("Betta")),
            Ordering::Greater
        );
    }

    #[test]
    fn a_sort_turns_up_then_down_then_off() {
        let mut query = IndexQuery::default();
        query.cycle_sort(Column::Worth);
        assert_eq!(query.words(), ["sort:worth"]);
        query.cycle_sort(Column::Worth);
        assert_eq!(query.words(), ["sort:-worth"]);
        query.cycle_sort(Column::Worth);
        assert!(query.words().is_empty());
    }

    #[test]
    fn a_query_reads_back_as_the_words_that_make_it() {
        let mut query = IndexQuery::default();
        for word in words(r#"all species:sal "Big Coral" sort:-weight"#) {
            query.add(Term::parse(&word).expect("a term"));
        }
        assert_eq!(
            query.words(),
            [
                "all",
                "species:sal",
                r#"fishtank:"Big Coral""#,
                "sort:-weight"
            ]
        );
    }

    #[test]
    fn only_an_index_that_names_a_place_or_a_status_shows_the_dead() {
        let mut query = IndexQuery::default();
        query.add(Term::Filter(Filter::new(Column::Species, "sal")));
        assert!(!query.shows_the_dead());
        query.add(Term::Name("Heaven".to_string()));
        assert!(query.shows_the_dead());
        let mut query = IndexQuery::default();
        query.add(Term::Filter(Filter::new(Column::Status, DEAD)));
        assert!(query.shows_the_dead());
    }

    #[test]
    fn every_queryable_column_has_a_key_that_finds_it() {
        for column in Column::FIXED.into_iter().filter(|c| c.is_queryable()) {
            assert!(
                Column::parse(&column.key()) == Some(column),
                "{}",
                column.header()
            );
        }
        assert!(Column::parse("tank") == Some(Column::Fishtank));
        assert!(Column::parse("display").is_none());
    }
}
