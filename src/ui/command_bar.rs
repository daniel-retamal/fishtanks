use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

use crate::colors::{BLACK, DARK_GRAY, WHITE};
use unicode_width::UnicodeWidthStr;

use crate::consumable::{ActiveConsumable, ActiveMilkStatus, Buff, Measure};
use crate::economy::Money;
use crate::names::to_roman;
use crate::ui::hint_bar::HintBar;
use crate::ui::table::{INFINITY, ellipsize, visual_width};

const RULE_ROWS: u16 = 2;
const EDITOR_ROWS: u16 = 1;
const ITEM_GAP: &str = "  ";
const TINY_GAP: &str = " ";
const METRIC_UNITS: [(Money, &str); 6] = [
    (1_000_000_000_000_000_000, "Qi"),
    (1_000_000_000_000_000, "Qa"),
    (1_000_000_000_000, "T"),
    (1_000_000_000, "B"),
    (1_000_000, "M"),
    (1_000, "k"),
];
const TENTHS: Money = 10;
const SCIENTIFIC_FROM: Money = 1_000_000_000_000_000_000_000;
const SECS_PER_MINUTE: u32 = 60;
const NAME_KEPT_W: usize = 16;
const TINY_FOOD: &str = "•";
const TINY_FISH: &str = "><>";
const WORD_JOINER: char = '-';
const ONE_CAST: &str = "cast";
const MANY_CASTS: &str = "casts";
const TINY_CASTS: &str = "c";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Density {
    Full,
    Short,
    Tiny,
}

impl Density {
    const ALL: [Density; 3] = [Density::Full, Density::Short, Density::Tiny];

    fn gap(self) -> &'static str {
        match self {
            Density::Tiny => TINY_GAP,
            Density::Full | Density::Short => ITEM_GAP,
        }
    }
}

pub struct StatsBar<'a> {
    pub active_consumables: &'a [ActiveConsumable],
    pub active_statuses: &'a [ActiveMilkStatus],
    pub cash: Option<Money>,
    pub food_supply: u32,
    pub fish_count: usize,
    pub fish_capacity: Option<usize>,
    pub modes: &'a [&'static str],
    pub tank_name: &'a str,
    pub devils_luck: u32,
    pub cajetans_grace: u32,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct StatusRow {
    pub left: String,
    pub right: String,
}

impl StatsBar<'_> {
    fn stats(&self, density: Density) -> String {
        let capacity = boundless(self.fish_capacity.map(|capacity| capacity.to_string()));
        let (cash, food, fish) = match density {
            Density::Full => (
                format!("cash: {}", boundless(self.cash.map(metric))),
                format!("food: {}", metric(self.food_supply.into())),
                format!("fishes: {}/{capacity}", self.fish_count),
            ),
            Density::Short => (
                format!("${}", boundless(self.cash.map(metric))),
                format!("food {}", metric(self.food_supply.into())),
                format!("fish {}/{capacity}", self.fish_count),
            ),
            Density::Tiny => (
                format!("${}", boundless(self.cash.map(rounded_metric))),
                format!("{TINY_FOOD}{}", rounded_metric(self.food_supply.into())),
                format!("{TINY_FISH}{}/{capacity}", self.fish_count),
            ),
        };
        [cash, food, fish].join(density.gap())
    }

    fn statuses(&self, density: Density) -> Vec<String> {
        let mut items: Vec<String> = self
            .modes
            .iter()
            .map(|&mode| untimed(mode, density))
            .collect();
        for (label, stacks, left) in self.buffs() {
            items.push(match left {
                Measure::Seconds(secs) => timed(label, stacks, secs, density),
                Measure::Casts(casts) => counted(label, stacks, casts, density),
            });
        }
        if self.devils_luck > 0 {
            items.push(stacked("devil's luck", self.devils_luck, density));
        }
        if self.cajetans_grace > 0 {
            items.push(stacked("cajetan's grace", self.cajetans_grace, density));
        }
        items
    }

    fn buffs(&self) -> Vec<(&'static str, u32, Measure)> {
        let every = self
            .active_consumables
            .iter()
            .map(|buff| buff as &dyn Buff)
            .chain(self.active_statuses.iter().map(|buff| buff as &dyn Buff));
        let mut grouped: Vec<(&'static str, u32, Measure)> = Vec::new();
        for buff in every {
            let label = buff.label();
            if label.is_empty() {
                continue;
            }
            match grouped.iter_mut().find(|(seen, ..)| *seen == label) {
                Some((_, stacks, soonest)) => {
                    *stacks += buff.stacks();
                    *soonest = soonest.sooner(buff.left());
                }
                None => grouped.push((label, buff.stacks(), buff.left())),
            }
        }
        grouped
    }

    pub fn rows(&self, width: u16) -> Vec<StatusRow> {
        let width = width as usize;
        let name_w = visual_width(self.tank_name).min(NAME_KEPT_W);
        let density = Density::ALL
            .into_iter()
            .find(|&density| name_w + ITEM_GAP.len() + visual_width(&self.stats(density)) <= width)
            .unwrap_or(Density::Tiny);
        let stats = self.stats(density);
        let statuses = self.statuses(density);
        let gap = density.gap();
        let joined = statuses.join(gap);
        let name_room = width.saturating_sub(visual_width(&stats) + ITEM_GAP.len());
        if name_room == 0 {
            let mut rows = vec![StatusRow {
                left: ellipsize(self.tank_name, width),
                right: String::new(),
            }];
            rows.extend(flow(
                stats.split(gap).map(str::to_string).collect(),
                width,
                gap,
            ));
            rows.extend(flow(statuses, width, gap));
            return rows;
        }
        let full_name_w = visual_width(self.tank_name);
        let inline = !joined.is_empty()
            && full_name_w
                + ITEM_GAP.len()
                + visual_width(&joined)
                + ITEM_GAP.len()
                + visual_width(&stats)
                <= width;
        let mut first = StatusRow {
            left: ellipsize(self.tank_name, name_room),
            right: stats,
        };
        if inline {
            first.left = format!("{}{ITEM_GAP}{joined}", self.tank_name);
            return vec![first];
        }
        let mut rows = vec![first];
        rows.extend(flow(statuses, width, gap));
        rows
    }
}

fn flow(items: Vec<String>, width: usize, gap: &str) -> Vec<StatusRow> {
    let mut rows: Vec<StatusRow> = Vec::new();
    let mut current = String::new();
    for item in items {
        let item = ellipsize(&item, width);
        let needed = if current.is_empty() {
            visual_width(&item)
        } else {
            visual_width(&current) + gap.len() + visual_width(&item)
        };
        if !current.is_empty() && needed > width {
            rows.push(StatusRow {
                left: std::mem::take(&mut current),
                right: String::new(),
            });
        }
        if !current.is_empty() {
            current.push_str(gap);
        }
        current.push_str(&item);
    }
    if !current.is_empty() {
        rows.push(StatusRow {
            left: current,
            right: String::new(),
        });
    }
    rows
}

fn boundless(amount: Option<String>) -> String {
    amount.unwrap_or_else(|| INFINITY.to_string())
}

fn untimed(label: &str, density: Density) -> String {
    match density {
        Density::Tiny => initials(label),
        Density::Full | Density::Short => label.to_string(),
    }
}

fn initials(label: &str) -> String {
    label
        .split(|ch: char| ch.is_whitespace() || ch == WORD_JOINER)
        .filter_map(|word| word.chars().find(|ch| ch.is_alphabetic()))
        .map(|ch| ch.to_ascii_uppercase())
        .collect()
}

fn timed(label: &str, stacks: u32, secs: f32, density: Density) -> String {
    let total = secs.ceil().max(0.0) as u32;
    let (minutes, seconds) = (total / SECS_PER_MINUTE, total % SECS_PER_MINUTE);
    match density {
        Density::Full => format!("{label} {}: {minutes:02}:{seconds:02}", to_roman(stacks)),
        Density::Short => format!("{label} {} {minutes}:{seconds:02}", to_roman(stacks)),
        Density::Tiny if total < SECS_PER_MINUTE => format!("{}{stacks} {total}s", initials(label)),
        Density::Tiny => format!(
            "{}{stacks} {}m",
            initials(label),
            total.div_ceil(SECS_PER_MINUTE)
        ),
    }
}

fn counted(label: &str, stacks: u32, casts: u32, density: Density) -> String {
    let unit = if casts == 1 { ONE_CAST } else { MANY_CASTS };
    match density {
        Density::Full => format!("{label} {}: {casts} {unit}", to_roman(stacks)),
        Density::Short => format!("{label} {} {casts} {unit}", to_roman(stacks)),
        Density::Tiny => format!("{}{stacks} {casts}{TINY_CASTS}", initials(label)),
    }
}

fn stacked(label: &str, stacks: u32, density: Density) -> String {
    match density {
        Density::Full => format!("{label} {}", to_roman(stacks)),
        Density::Short => format!("{label} {stacks}"),
        Density::Tiny => format!("{}{stacks}", initials(label)),
    }
}

pub fn metric(n: Money) -> String {
    if n >= SCIENTIFIC_FROM {
        return scientific(n);
    }
    for (unit, suffix) in METRIC_UNITS {
        if n >= unit {
            return format!("{}.{}{suffix}", n / unit, (n % unit) / (unit / TENTHS));
        }
    }
    n.to_string()
}

pub fn scientific(n: Money) -> String {
    format!("{:.1e}", n as f64)
}

fn rounded_metric(n: Money) -> String {
    if n >= SCIENTIFIC_FROM {
        return scientific(n);
    }
    let tenth = TENTHS;
    for (unit, suffix) in METRIC_UNITS {
        if n < unit {
            continue;
        }
        let tenths = (n * tenth + unit / 2) / unit;
        if tenths < tenth * tenth && !tenths.is_multiple_of(tenth) {
            return format!("{}.{}{suffix}", tenths / tenth, tenths % tenth);
        }
        return format!("{}{suffix}", (tenths + tenth / 2) / tenth);
    }
    n.to_string()
}

fn prompt_rows(width: u16, console: Option<&HintBar>) -> u16 {
    console.map_or(EDITOR_ROWS, |bar| bar.height(width))
}

pub fn height(show_stats: bool, width: u16, stats: &StatsBar, console: Option<&HintBar>) -> u16 {
    let prompt = RULE_ROWS + prompt_rows(width, console);
    if !show_stats {
        return prompt;
    }
    prompt + stats.rows(width).len() as u16
}

pub struct CommandBar<'a> {
    pub input: &'a str,
    pub cursor_pos: usize,
    pub cursor_visible: bool,
    pub ghost: &'a str,
    pub show_stats: bool,
    pub stats: StatsBar<'a>,
    pub console: Option<HintBar>,
}

impl CommandBar<'_> {
    fn render_editor(&self, area: Rect, buf: &mut Buffer) {
        let white = Style::default().fg(WHITE);
        let gray = Style::default().fg(DARK_GRAY);
        let row = area.y + 1;
        buf.set_string(area.x, row, "> ", white);

        let base_x = area.x + 2;
        let display_col = UnicodeWidthStr::width(&self.input[..self.cursor_pos]) as u16;
        let cursor_col = base_x + display_col;
        let at_end = self.cursor_pos == self.input.len();

        if self.cursor_pos > 0 && base_x < area.right() {
            let clip = (area.right() - base_x) as usize;
            buf.set_string(base_x, row, &self.input[..self.cursor_pos.min(clip)], white);
        }

        if cursor_col < area.right() {
            let ch = if at_end {
                self.ghost.chars().next().unwrap_or(' ')
            } else {
                self.input[self.cursor_pos..].chars().next().unwrap_or(' ')
            };
            if self.cursor_visible {
                buf[(cursor_col, row)]
                    .set_char(ch)
                    .set_fg(BLACK)
                    .set_bg(WHITE);
            } else if at_end {
                buf[(cursor_col, row)].set_char(ch).set_fg(DARK_GRAY);
            } else {
                buf[(cursor_col, row)].set_char(ch).set_fg(WHITE);
            }
        }

        if at_end {
            let ghost_first_len = self.ghost.chars().next().map(|c| c.len_utf8()).unwrap_or(0);
            let ghost_rest = &self.ghost[ghost_first_len..];
            let after_x = cursor_col + 1;
            if !ghost_rest.is_empty() && after_x < area.right() {
                let clip = (area.right() - after_x) as usize;
                buf.set_string(
                    after_x,
                    row,
                    &ghost_rest[..ghost_rest.len().min(clip)],
                    gray,
                );
            }
        } else {
            let char_len = self.input[self.cursor_pos..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            let after = &self.input[self.cursor_pos + char_len..];
            let after_x = cursor_col + 1;
            if !after.is_empty() && after_x < area.right() {
                let clip = (area.right() - after_x) as usize;
                buf.set_string(after_x, row, &after[..after.len().min(clip)], white);
            }
            let ghost_x = base_x + UnicodeWidthStr::width(self.input) as u16;
            if !self.ghost.is_empty() && ghost_x < area.right() {
                let clip = (area.right() - ghost_x) as usize;
                buf.set_string(
                    ghost_x,
                    row,
                    &self.ghost[..self.ghost.len().min(clip)],
                    gray,
                );
            }
        }
    }
}

impl Widget for CommandBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let sep = "─".repeat(area.width as usize);
        let sep_style = Style::default().fg(DARK_GRAY);
        let white = Style::default().fg(WHITE);

        buf.set_string(area.x, area.y, &sep, sep_style);

        let prompt = prompt_rows(area.width, self.console.as_ref());
        match &self.console {
            Some(bar) => bar.draw(
                buf,
                Rect::new(area.x, area.y + 1, area.width, prompt).intersection(area),
                Color::Reset,
            ),
            None => self.render_editor(area, buf),
        }

        let lower_rule = area.y + 1 + prompt;
        if lower_rule < area.bottom() {
            buf.set_string(area.x, lower_rule, &sep, sep_style);
        }

        if !self.show_stats {
            return;
        }
        for (index, status) in self.stats.rows(area.width).iter().enumerate() {
            let y = lower_rule + 1 + index as u16;
            if y >= area.bottom() {
                return;
            }
            buf.set_stringn(area.x, y, &status.left, area.width as usize, white);
            let right_w = visual_width(&status.right) as u16;
            if right_w > 0 {
                let x = area.right().saturating_sub(right_w);
                buf.set_stringn(x, y, &status.right, area.width as usize, white);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(tank_name: &str) -> StatsBar<'_> {
        StatsBar {
            active_consumables: &[],
            active_statuses: &[],
            cash: Some(40_000),
            food_supply: 1_250,
            fish_count: 3,
            fish_capacity: Some(50),
            modes: &[],
            tank_name,
            devils_luck: 2,
            cajetans_grace: 0,
        }
    }

    #[test]
    fn an_endless_purse_and_a_boundless_tank_read_as_infinity() {
        let stats = StatsBar {
            cash: None,
            fish_capacity: None,
            ..bar("Fishtank")
        };
        assert_eq!(stats.rows(100)[0].right, "cash: ∞  food: 1.2k  fishes: 3/∞");
    }

    #[test]
    fn a_mode_is_named_among_the_statuses_and_abbreviated_when_tiny() {
        let stats = StatsBar {
            modes: &["debug mode"],
            ..bar("Fishtank")
        };
        assert!(stats.rows(100)[0].left.contains("debug mode"));
        assert!(stats.statuses(Density::Tiny).contains(&"DM".to_string()));
    }

    #[test]
    fn a_wide_bar_says_everything_in_words_on_one_row() {
        let rows = bar("Fishtank").rows(100);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].right, "cash: 40.0k  food: 1.2k  fishes: 3/50");
        assert!(rows[0].left.contains("devil's luck II"), "{rows:?}");
    }

    #[test]
    fn a_narrower_bar_switches_to_symbols_before_it_hides_anything() {
        let rows = bar("Fishtank").rows(40);
        assert_eq!(rows[0].left, "Fishtank");
        assert_eq!(rows[0].right, "$40.0k  food 1.2k  fish 3/50");
    }

    #[test]
    fn a_tiny_bar_rounds_its_numbers_and_keeps_the_name() {
        let rows = bar("Fishtank").rows(26);
        assert_eq!(rows[0].right, "$40k •1.3k ><>3/50");
        assert!(rows[0].left.starts_with("Fis"), "{rows:?}");
        assert!(
            rows.iter().any(|row| row.left.contains("DL2")),
            "the status wraps to its own row as initials: {rows:?}"
        );
    }

    #[test]
    fn a_bar_narrower_than_its_stats_stacks_them_instead_of_dropping_them() {
        let rows = bar("Fishtank").rows(12);
        let text: String = rows
            .iter()
            .map(|row| format!("{}{}", row.left, row.right))
            .collect();
        assert!(text.contains("$40k"), "{rows:?}");
        assert!(text.contains("><>3/50"), "{rows:?}");
    }

    #[test]
    fn rounding_keeps_one_decimal_only_while_it_says_something() {
        assert_eq!(rounded_metric(999), "999");
        assert_eq!(rounded_metric(1_250), "1.3k");
        assert_eq!(rounded_metric(40_000), "40k");
        assert_eq!(rounded_metric(2_000_000), "2M");
    }

    #[test]
    fn a_fortune_past_a_billion_keeps_its_compact_form() {
        assert_eq!(metric(1_230_000_000_000), "1.2T");
        assert_eq!(metric(Money::from(u32::MAX) * 1_000), "4.2T");
        assert_eq!(rounded_metric(7_500_000_000_000_000), "7.5Qa");
        assert_eq!(metric(Money::from(u64::MAX)), "18.4Qi");
        assert_eq!(rounded_metric(Money::from(u64::MAX)), "18Qi");
        assert_eq!(metric(Money::MAX), "3.4e38");
    }

    fn bait(casts_left: u32) -> ActiveConsumable {
        ActiveConsumable {
            casts_left,
            ..ActiveConsumable::fresh(crate::loot::ConsumableKind::Bait).expect("bait is a buff")
        }
    }

    #[test]
    fn stacks_of_one_buff_show_as_one_status_with_the_soonest_to_run_out() {
        let buffs = [bait(5), bait(2), bait(4)];
        let stats = StatsBar {
            active_consumables: &buffs,
            ..bar("Fishtank")
        };
        let statuses = stats.statuses(Density::Full);
        assert!(
            statuses.contains(&"baiting III: 2 casts".to_string()),
            "{statuses:?}"
        );
        assert!(stats.statuses(Density::Tiny).contains(&"B3 2c".to_string()));
    }

    #[test]
    fn a_last_cast_is_said_in_the_singular() {
        assert_eq!(counted("baiting", 1, 1, Density::Short), "baiting I 1 cast");
    }

    #[test]
    fn a_status_timer_is_minutes_in_tiny_form() {
        assert_eq!(timed("caffeinated", 2, 299.0, Density::Tiny), "C2 5m");
        assert_eq!(timed("caffeinated", 2, 42.0, Density::Tiny), "C2 42s");
        assert_eq!(
            timed("caffeinated", 2, 299.0, Density::Full),
            "caffeinated II: 04:59"
        );
    }
}
