use std::env;

use fishtank::dungeon::Difficulty;
use fishtank::dungeon::fighter::Scale;
use fishtank::dungeon::floor::LAST_FLOOR;
use fishtank::dungeon::sim::{Report, Tier, autopsy, dived, survival_at};

const DEFAULT_RUNS: u64 = 200;
const PERCENT: f64 = 100.0;
const AUTOPSY_LINES: usize = 3;
const SWEEP: [u32; 14] = [
    25, 35, 45, 55, 65, 75, 85, 100, 115, 130, 150, 175, 200, 250,
];

fn main() {
    let runs = env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(DEFAULT_RUNS);
    let autopsies = env::args().any(|arg| arg == "--autopsy");
    let args: Vec<String> = env::args().collect();
    let forced: Option<u32> = args
        .iter()
        .position(|arg| arg == "--scale")
        .and_then(|at| args.get(at + 1))
        .and_then(|percent| percent.parse().ok());
    if env::args().any(|arg| arg == "--dive") {
        dives(runs);
        return;
    }
    if env::args().any(|arg| arg == "--sweep") {
        let only: Option<u32> = args
            .iter()
            .position(|arg| arg == "--floor")
            .and_then(|at| args.get(at + 1))
            .and_then(|depth| depth.parse().ok());
        sweep(runs, only);
        return;
    }
    println!("The Deep, {runs} runs a floor");
    for tier in Tier::ALL {
        let report = Report::of(tier, Difficulty::Normal, runs);
        let floors: Vec<String> = report
            .floors
            .iter()
            .zip(tier.targets())
            .map(|(got, target)| match target {
                Some(target) => format!("{:>3.0}/{:<3.0}", got * PERCENT, target * PERCENT),
                None => format!("{:>3.0}    ", got * PERCENT),
            })
            .collect();
        let (floor, hourly) = report.best_hourly(Difficulty::Normal);
        println!(
            "{:<8} {}  all {:>5.1}%  best ${:.0}/h riding up after F{floor} (of {LAST_FLOOR})",
            tier.name(),
            floors.join(" "),
            report.all_ten() * PERCENT,
            hourly
        );
        if autopsies {
            for depth in 1..=LAST_FLOOR {
                let scale = match forced {
                    Some(percent) => Scale {
                        hp_percent: percent,
                        bite_percent: percent,
                    },
                    None => fishtank::dungeon::floor::scale_for(depth),
                };
                let causes = autopsy(tier, depth, Difficulty::Normal, scale, runs);
                let worst: Vec<String> = causes
                    .iter()
                    .take(AUTOPSY_LINES)
                    .map(|(cause, count)| format!("{count}× {cause}"))
                    .collect();
                if !worst.is_empty() {
                    println!("    F{depth}: {}", worst.join(" | "));
                }
            }
        }
    }
}

fn sweep(runs: u64, only: Option<u32>) {
    println!("Survival by enemy scale, {runs} runs; each cell is Starter/Mid/Strong/Bred");
    for depth in (1..=LAST_FLOOR).filter(|depth| only.is_none_or(|only| only == *depth)) {
        let cells: Vec<String> = SWEEP
            .iter()
            .map(|&percent| {
                let scale = Scale {
                    hp_percent: percent,
                    bite_percent: percent,
                };
                let tiers: Vec<String> = Tier::ALL
                    .iter()
                    .map(|&tier| {
                        let got = survival_at(tier, depth, Difficulty::Normal, scale, runs);
                        format!("{:.0}", got * PERCENT)
                    })
                    .collect();
                format!("{percent}%:{}", tiers.join("/"))
            })
            .collect();
        println!("F{depth:<2} {}", cells.join("  "));
    }
}

fn dives(runs: u64) {
    println!("Whole dives from the Shallows, {runs} runs; floors cleared");
    for tier in Tier::ALL {
        let mut cleared = vec![0u64; LAST_FLOOR as usize + 1];
        let mut joined = vec![0u64; LAST_FLOOR as usize + 1];
        let mut first = None;
        for seed in 0..runs {
            let result = dived(tier, Difficulty::Normal, seed);
            let floors = result.floors as usize;
            cleared[floors] += 1;
            joined[floors] += u64::from(result.joined);
            if floors == LAST_FLOOR as usize && first.is_none() {
                first = Some(seed);
            }
        }
        let cells: Vec<String> = cleared
            .iter()
            .zip(&joined)
            .map(|(n, j)| format!("{n}(+{:.1})", *j as f64 / (*n).max(1) as f64))
            .collect();
        println!(
            "{:<8} {}  first clear at seed {:?}",
            tier.name(),
            cells.join(" "),
            first
        );
    }
}
