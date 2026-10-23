use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

pub fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    upper + chars.as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn unique_roman_in(used: &HashSet<String>) -> String {
    let mut i = 1u32;
    loop {
        let candidate = to_roman(i);
        if !used.contains(&candidate) {
            return candidate;
        }
        i += 1;
    }
}

pub fn unique_name_in(used: &HashSet<String>, requested: &str) -> String {
    if !used.contains(requested) {
        return requested.to_string();
    }
    let (root, n) = parse_name_numeral(requested);
    let mut i = n + 1;
    loop {
        let candidate = format!("{} {}", root, to_roman(i));
        if !used.contains(&candidate) {
            return candidate;
        }
        i += 1;
    }
}

fn parse_name_numeral(name: &str) -> (&str, u32) {
    if let Some(pos) = name.rfind(' ') {
        let suffix = &name[pos + 1..];
        if let Some(n) = roman_to_u32(suffix) {
            return (&name[..pos], n);
        }
    }
    (name, 1)
}

pub fn to_roman(mut n: u32) -> String {
    const VALS: &[(u32, &str)] = &[
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut s = String::new();
    for &(val, sym) in VALS {
        while n >= val {
            s.push_str(sym);
            n -= val;
        }
    }
    s
}

fn roman_to_u32(s: &str) -> Option<u32> {
    if s.is_empty() {
        return None;
    }
    let digit = |c: char| match c {
        'I' => Some(1u32),
        'V' => Some(5),
        'X' => Some(10),
        'L' => Some(50),
        'C' => Some(100),
        'D' => Some(500),
        'M' => Some(1000),
        _ => None,
    };
    let vals: Option<Vec<u32>> = s.chars().map(digit).collect();
    let vals = vals?;
    let mut total = 0u32;
    let mut prev = 0u32;
    for &v in vals.iter().rev() {
        if v < prev {
            total = total.saturating_sub(v);
        } else {
            total = total.saturating_add(v);
        }
        prev = v;
    }
    if total == 0 { None } else { Some(total) }
}

#[derive(Clone, Default)]
pub struct NameBook(Rc<RefCell<HashSet<String>>>);

impl NameBook {
    pub fn contains(&self, name: &str) -> bool {
        self.0.borrow().contains(name)
    }

    pub fn insert(&self, name: String) -> bool {
        self.0.borrow_mut().insert(name)
    }

    pub fn remove(&self, name: &str) -> bool {
        self.0.borrow_mut().remove(name)
    }

    pub fn clear(&self) {
        self.0.borrow_mut().clear();
    }

    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }

    pub fn names(&self) -> HashSet<String> {
        self.0.borrow().clone()
    }

    pub fn unique(&self, requested: &str) -> String {
        unique_name_in(&self.0.borrow(), requested)
    }

    pub fn unique_roman(&self) -> String {
        unique_roman_in(&self.0.borrow())
    }

    pub fn renamed(&self, name: &str) -> String {
        match roman_to_u32(name) {
            Some(_) => self.unique_roman(),
            None => self.unique(name),
        }
    }

    pub fn shares(&self, other: &NameBook) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_case_single_word() {
        assert_eq!(title_case("nemo"), "Nemo");
    }

    #[test]
    fn title_case_multiple_words() {
        assert_eq!(title_case("blue tang"), "Blue Tang");
    }

    #[test]
    fn title_case_already_correct() {
        assert_eq!(title_case("Nemo"), "Nemo");
    }

    #[test]
    fn title_case_all_caps() {
        assert_eq!(title_case("NEMO"), "NEMO");
    }

    #[test]
    fn title_case_empty_string() {
        assert_eq!(title_case(""), "");
    }

    #[test]
    fn title_case_extra_spaces_are_collapsed() {
        assert_eq!(title_case("blue  tang"), "Blue Tang");
    }

    #[test]
    fn to_roman_one() {
        assert_eq!(to_roman(1), "I");
    }

    #[test]
    fn to_roman_two() {
        assert_eq!(to_roman(2), "II");
    }

    #[test]
    fn to_roman_three() {
        assert_eq!(to_roman(3), "III");
    }

    #[test]
    fn to_roman_four() {
        assert_eq!(to_roman(4), "IV");
    }

    #[test]
    fn to_roman_nine() {
        assert_eq!(to_roman(9), "IX");
    }

    #[test]
    fn to_roman_ten() {
        assert_eq!(to_roman(10), "X");
    }

    #[test]
    fn to_roman_fourteen() {
        assert_eq!(to_roman(14), "XIV");
    }

    #[test]
    fn to_roman_forty() {
        assert_eq!(to_roman(40), "XL");
    }

    #[test]
    fn to_roman_fifty() {
        assert_eq!(to_roman(50), "L");
    }

    #[test]
    fn to_roman_ninety() {
        assert_eq!(to_roman(90), "XC");
    }

    #[test]
    fn to_roman_one_hundred() {
        assert_eq!(to_roman(100), "C");
    }

    #[test]
    fn to_roman_four_hundred() {
        assert_eq!(to_roman(400), "CD");
    }

    #[test]
    fn to_roman_nine_hundred() {
        assert_eq!(to_roman(900), "CM");
    }

    #[test]
    fn to_roman_one_thousand() {
        assert_eq!(to_roman(1000), "M");
    }

    #[test]
    fn unique_name_no_collision() {
        let used = HashSet::new();
        assert_eq!(unique_name_in(&used, "Nemo"), "Nemo");
    }

    #[test]
    fn unique_name_first_collision_yields_roman_ii() {
        let mut used = HashSet::new();
        used.insert("Nemo".to_string());
        assert_eq!(unique_name_in(&used, "Nemo"), "Nemo II");
    }

    #[test]
    fn unique_name_chain_collision() {
        let mut used = HashSet::new();
        used.insert("Nemo".to_string());
        used.insert("Nemo II".to_string());
        assert_eq!(unique_name_in(&used, "Nemo"), "Nemo III");
    }

    #[test]
    fn unique_name_long_chain() {
        let mut used = HashSet::new();
        used.insert("Nemo".to_string());
        used.insert("Nemo II".to_string());
        used.insert("Nemo III".to_string());
        assert_eq!(unique_name_in(&used, "Nemo"), "Nemo IV");
    }

    #[test]
    fn unique_name_request_already_roman_skips_ahead() {
        let mut used = HashSet::new();
        used.insert("Nemo II".to_string());
        assert_eq!(unique_name_in(&used, "Nemo II"), "Nemo III");
    }

    #[test]
    fn unique_name_multi_word_name() {
        let mut used = HashSet::new();
        used.insert("Blue Tang".to_string());
        assert_eq!(unique_name_in(&used, "Blue Tang"), "Blue Tang II");
    }
}
