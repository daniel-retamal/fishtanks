use rand::RngExt;

pub fn sample_exponential(rng: &mut impl RngExt, mean: f32) -> f32 {
    let u: f32 = rng.random_range(f32::MIN_POSITIVE..1.0);
    -mean * u.ln()
}

pub fn hyperbolic_scale(base: f32, count: u32, alpha: f32) -> f32 {
    base / (1.0 + alpha * count as f32)
}

pub fn exponential_event(rng: &mut impl RngExt, mean: f32, dt: f32) -> bool {
    if mean <= 0.0 {
        return true;
    }
    rng.random::<f32>() < 1.0 - (-dt / mean).exp()
}

pub fn events_in(rng: &mut impl RngExt, per_sec: f32, dt: f32) -> u32 {
    let expected = (per_sec * dt).max(0.0);
    let whole = expected.floor();
    whole as u32 + u32::from(rng.random::<f32>() < expected - whole)
}

const BEAT_TOLERANCE: f32 = 1e-4;

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Metronome {
    elapsed: f32,
}

impl Metronome {
    pub fn beats(&mut self, dt: f32, interval: f32) -> u32 {
        self.elapsed += dt;
        let beats = (self.elapsed / interval + BEAT_TOLERANCE).floor();
        if beats < 1.0 {
            return 0;
        }
        self.elapsed = (self.elapsed - beats * interval).max(0.0);
        beats as u32
    }
}

pub fn even_indices(avail: usize, count: usize) -> Vec<usize> {
    let mut out = Vec::with_capacity(count);
    let mut last: Option<usize> = None;
    for j in 0..count {
        let mut idx = (((2 * j + 1) * avail) / (2 * count)).min(avail - 1);
        if let Some(prev) = last
            && idx <= prev
        {
            idx = prev + 1;
        }
        out.push(idx);
        last = Some(idx);
    }
    out
}

pub const SECS_A_DAY: u64 = 86_400;
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const DAYS_FROM_YEAR_ZERO: i64 = 719_468;
const DAYS_AN_ERA: i64 = 146_097;
const MONTH_DAYS_NUMERATOR: i64 = 153;
const MONTH_SHIFT: i64 = 2;
const MARCH_BASED: i64 = 3;
const DAYS_A_YEAR: i64 = 365;
const LEAP_EVERY: i64 = 4;
const CENTURY: i64 = 100;
const DAYS_IN_FOUR_YEARS: i64 = 1_460;
const DAYS_IN_A_CENTURY: i64 = 36_524;
const DAYS_IN_AN_ERA_LESS_ONE: i64 = 146_096;
const MONTHS_IN_A_YEAR: i64 = 12;
const MONTH_LENGTH_DIVISOR: i64 = 5;

pub fn day_and_month(days_since_epoch: i64) -> String {
    let z = days_since_epoch + DAYS_FROM_YEAR_ZERO;
    let era = z.div_euclid(DAYS_AN_ERA);
    let doe = z - era * DAYS_AN_ERA;
    let yoe = (doe - doe / DAYS_IN_FOUR_YEARS + doe / DAYS_IN_A_CENTURY
        - doe / DAYS_IN_AN_ERA_LESS_ONE)
        / DAYS_A_YEAR;
    let doy = doe - (DAYS_A_YEAR * yoe + yoe / LEAP_EVERY - yoe / CENTURY);
    let mp = (MONTH_LENGTH_DIVISOR * doy + MONTH_SHIFT) / MONTH_DAYS_NUMERATOR;
    let day = doy - (MONTH_DAYS_NUMERATOR * mp + MONTH_SHIFT) / MONTH_LENGTH_DIVISOR + 1;
    let month = match mp < MONTHS_IN_A_YEAR - MARCH_BASED + 1 {
        true => mp + MARCH_BASED,
        false => mp - (MONTHS_IN_A_YEAR - MARCH_BASED + 1) + 1,
    };
    format!("{day} {}", MONTHS[(month - 1) as usize])
}

pub fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    day_and_month((secs / SECS_A_DAY) as i64)
}

#[cfg(test)]
mod tests {
    use rand::{SeedableRng, rngs::SmallRng};

    use super::*;

    const DEFAULT_FRAME: f32 = 1.0 / 30.0;
    const FAST_FRAME: f32 = 1.0 / 120.0;

    #[test]
    fn a_day_is_named_by_its_date() {
        assert_eq!(day_and_month(0), "1 Jan");
        assert_eq!(day_and_month(20_738), "12 Oct");
        assert_eq!(day_and_month(19_782), "29 Feb");
    }

    #[test]
    fn a_metronome_counts_whole_intervals_and_keeps_the_rest() {
        let mut clock = Metronome::default();
        assert_eq!(clock.beats(0.25, 1.0), 0);
        assert_eq!(clock.beats(0.25, 1.0), 0);
        assert_eq!(clock.beats(0.75, 1.0), 1);
        assert_eq!(clock.beats(2.0, 1.0), 2);
        assert_eq!(clock.beats(0.25, 1.0), 0);
        assert_eq!(clock.beats(0.5, 1.0), 1);
    }

    #[test]
    fn a_metronome_never_loses_a_beat_to_rounding() {
        let mut clock = Metronome::default();
        assert_eq!(clock.beats(1.0, DEFAULT_FRAME), 30);
    }

    #[test]
    fn a_metronome_at_the_default_frame_rate_beats_once_a_frame() {
        let mut clock = Metronome::default();
        for _ in 0..90 {
            assert_eq!(clock.beats(DEFAULT_FRAME, DEFAULT_FRAME), 1);
        }
    }

    #[test]
    fn a_metronome_beats_as_often_per_second_whatever_the_frame() {
        let mut slow = Metronome::default();
        let mut fast = Metronome::default();
        let slow_beats: u32 = (0..30).map(|_| slow.beats(DEFAULT_FRAME, 0.1)).sum();
        let fast_beats: u32 = (0..120).map(|_| fast.beats(FAST_FRAME, 0.1)).sum();
        assert!(slow_beats.abs_diff(10) <= 1, "{slow_beats}");
        assert!(fast_beats.abs_diff(10) <= 1, "{fast_beats}");
    }

    #[test]
    fn events_in_a_second_average_their_rate_at_any_frame() {
        let mut rng = SmallRng::seed_from_u64(7);
        let rate = 13.5;
        let seconds = 400;
        for frame in [1.0, DEFAULT_FRAME, FAST_FRAME] {
            let frames = (seconds as f32 / frame).round() as usize;
            let total: u32 = (0..frames).map(|_| events_in(&mut rng, rate, frame)).sum();
            let mean = total as f32 / seconds as f32;
            assert!(
                (mean - rate).abs() < 0.5,
                "{mean} events a second at {frame}"
            );
        }
    }

    #[test]
    fn no_time_is_no_event() {
        let mut rng = SmallRng::seed_from_u64(1);
        assert_eq!(events_in(&mut rng, 13.5, 0.0), 0);
    }
}
