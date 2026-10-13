use super::*;

const RUNS: u64 = 30;
const FLOOR_BAND: f64 = 0.20;
const ALL_TEN_BAND: f64 = 0.25;
const HOURLY_FACTOR: f64 = 1.75;

#[test]
fn survival_matches_the_targets() {
    let mut misses = Vec::new();
    for tier in Tier::ALL {
        let report = Report::of(tier, Difficulty::Normal, RUNS);
        for (index, target) in tier.targets().into_iter().enumerate() {
            let got = report.floors[index];
            if let Some(target) = target
                && (got - target).abs() > FLOOR_BAND
            {
                misses.push(format!(
                    "{} F{}: {got:.2} against {target:.2}",
                    tier.name(),
                    index + 1
                ));
            }
        }
        if (report.all_ten() - tier.all_ten()).abs() > ALL_TEN_BAND {
            misses.push(format!(
                "{} all ten: {:.2} against {:.2}",
                tier.name(),
                report.all_ten(),
                tier.all_ten()
            ));
        }
        let (_, hourly) = report.best_hourly(Difficulty::Normal);
        let ratio = hourly / tier.hourly();
        if !(1.0 / HOURLY_FACTOR..=HOURLY_FACTOR).contains(&ratio) {
            misses.push(format!(
                "{} earns ${hourly:.0} an hour against ${:.0}",
                tier.name(),
                tier.hourly()
            ));
        }
    }
    assert!(misses.is_empty(), "{misses:#?}");
}

#[test]
fn a_starter_party_clears_the_shallows() {
    assert!(survival(Tier::Starter, 1, Difficulty::Normal, RUNS) > 0.5);
}

#[test]
fn a_bred_party_can_clear_the_eye() {
    assert!(survival(Tier::Bred, LAST_FLOOR, Difficulty::Normal, RUNS) > 0.5);
}

#[test]
fn hardcore_pays_twice_for_the_same_fights() {
    let report = Report::of(Tier::Mid, Difficulty::Normal, 1);
    let normal = report.best_hourly(Difficulty::Normal);
    let hardcore = report.best_hourly(Difficulty::Hardcore);
    assert_eq!(normal.0, hardcore.0);
    assert!((hardcore.1 - normal.1 * floor::HARDCORE_PAY as f64).abs() < 1.0);
}

#[test]
fn a_starter_party_can_clear_the_deep() {
    let cleared =
        (0..STARTER_RUNS).find(|&seed| dive(Tier::Starter, Difficulty::Normal, seed) >= LAST_FLOOR);
    assert!(
        cleared.is_some(),
        "no starter dive of {STARTER_RUNS} cleared all ten floors"
    );
}
