use std::env;
use std::path::Path;
use std::process::ExitCode;

use crossterm::event::KeyCode;
use fishtank::casino::Multiple;
use fishtank::casino::claw::{Goods, Motion, Phase};
use fishtank::casino::flip::{FlipPhase, Landing, Side};
use fishtank::casino::pufferfish::PuffPhase;
use fishtank::casino::spins::{PEARLS_TO_DIVE, Spins, pearls_in, window};
use fishtank::casino::state::{CasinoState, Play, Popup, View};
use fishtank::fishes::fish::Fish;
use fishtank::fishes::species::SizeCategory;
use fishtank::fishes::toy::{FittedPart, Material, Paint, Signature, ToyColor, ToyPart, ToyState};
use fishtank::testing::Tui;
use rand::{SeedableRng, rngs::SmallRng};

const USAGE: &str = "usage: cargo run --example casino -- <out-dir>

Films the wiki's casino pictures, one reel per scene (<out>/reels/casino-<scene>.html), on a big
screen so every hint bar fits on one line: the lobby, a hand of blackjack with a seahorse on the
line, a spin, a spin that lands three pearls and dives, a Pufferfish that puffs to a pop, a sky of
bubbles on the Stupid board, a Derby with a fish on the line, a toy won at the Claw, a fish doubled at double
or nothing until Tollomind eats it, and a Stupid Win. The rare moments are set up by hand;
everything after the setup plays out on its own.";
const TOYBOX_SEED: u64 = 8;
const COLS: u16 = 120;
const ROWS: u16 = 40;

fn table(tui: &mut Tui, line: &str) {
    tui.stake();
    tui.run("/spawn seahorsefish \"Pegaso\"");
    tui.run("/spawn merluza \"Adam\"");
    tui.run(line);
}

fn state(tui: &mut Tui) -> &mut CasinoState {
    tui.app.casino_state_mut().expect("the casino is open")
}

fn play(tui: &mut Tui) -> &mut Play {
    match &mut state(tui).view {
        View::Table(table) => &mut table.play,
        View::Lobby { .. } => panic!("a table is open"),
    }
}

fn scene(dir: &Path, name: &str) -> Tui {
    let mut tui = Tui::with_size(COLS, ROWS);
    tui.clear_tank();
    tui.film(dir, &format!("casino-{name}"));
    tui
}

fn lobby(dir: &Path) {
    let mut tui = scene(dir, "lobby");
    table(&mut tui, "/casino");
    tui.app.casino.pot = 48_210;
    tui.record(40, 2, "lobby");
}

fn blackjack(dir: &Path) {
    let mut tui = scene(dir, "blackjack");
    table(&mut tui, "/casino blackjack");
    tui.key(KeyCode::Tab);
    tui.select("Pegaso");
    tui.record(20, 2, "picker");
    tui.key(KeyCode::Enter);
    tui.record(20, 2, "seated");
    tui.key(KeyCode::Enter);
    tui.record(70, 2, "deal");
    tui.key(KeyCode::Enter);
    tui.record(160, 2, "dealer");
}

fn spins(dir: &Path) {
    let mut tui = scene(dir, "spins");
    table(&mut tui, "/casino spins");
    for _ in 0..3 {
        tui.key(KeyCode::Right);
    }
    tui.record(10, 2, "rest");
    tui.key(KeyCode::Down);
    tui.record(110, 2, "spin");
}

fn three_pearls() -> Spins {
    (0..)
        .map(|seed| {
            let mut spins = Spins::default();
            spins.spin(&mut SmallRng::seed_from_u64(seed));
            spins
        })
        .find(|spins| pearls_in(&window(spins.reels.map(|r| r.target))) >= PEARLS_TO_DIVE)
        .expect("some seed lands three pearls")
}

fn pearl_dive(dir: &Path) {
    let mut tui = scene(dir, "pearl-dive");
    table(&mut tui, "/casino spins");
    tui.record(9, 3, "rest");
    tui.key(KeyCode::Down);
    if let Play::Spins(spins) = play(&mut tui) {
        *spins = three_pearls();
    }
    tui.record(660, 3, "dive");
}

fn pufferfish(dir: &Path) {
    let mut tui = scene(dir, "pufferfish");
    table(&mut tui, "/casino pufferfish");
    tui.key(KeyCode::Enter);
    if let Play::Pufferfish(puffer) = play(&mut tui) {
        puffer.phase = PuffPhase::Puffing {
            clock: 0.0,
            pops_at: Multiple::hundredths(1_240),
        };
    }
    tui.record(30 * 27, 4, "puff");
}

fn bubbles(dir: &Path) {
    let mut tui = scene(dir, "bubble-up");
    table(&mut tui, "/casino bubbleup");
    tui.key(KeyCode::Up);
    for _ in 0..10 {
        tui.key(KeyCode::Enter);
        tui.record(8, 2, "blow");
    }
    tui.record(80, 2, "rise");
    if state(&mut tui).popup.is_some() {
        tui.record(40, 2, "card");
    }
}

fn derby(dir: &Path) {
    let mut tui = scene(dir, "derby");
    table(&mut tui, "/casino derby");
    tui.key(KeyCode::Tab);
    tui.select("Adam");
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Down);
    tui.record(20, 2, "line-up");
    tui.key(KeyCode::Enter);
    tui.record(300, 3, "race");
}

fn claw(dir: &Path) {
    let mut tui = scene(dir, "claw");
    table(&mut tui, "/casino claw");
    let target = match play(&mut tui) {
        Play::Claw(glass) => {
            for prize in &mut glass.prizes {
                prize.speed = 0.0;
            }
            glass
                .prizes
                .iter()
                .filter(|p| matches!(p.goods, Goods::Toy(_)) && p.motion == Motion::Resting)
                .map(|p| p.left() + p.width() / 2)
                .next()
        }
        _ => None,
    };
    tui.record(20, 2, "glass");
    tui.key(KeyCode::Enter);
    if let Some(target) = target {
        let start = match play(&mut tui) {
            Play::Claw(glass) => glass.hand.rail,
            _ => target,
        };
        for _ in start..target {
            tui.key(KeyCode::Right);
        }
    }
    tui.record(40, 2, "steer");
    tui.key(KeyCode::Down);
    for _ in 0..400 {
        tui.tick_n(1);
        if let Play::Claw(glass) = play(&mut tui)
            && glass.phase == Phase::Grabbed
        {
            glass.hold_fast();
            break;
        }
    }
    tui.record(260, 3, "won");
    tui.typewrite("Pip", 3);
    tui.record(10, 2, "named");
}

fn toybox(dir: &Path) {
    let mut tui = scene(dir, "toybox");
    let mut rng = SmallRng::seed_from_u64(TOYBOX_SEED);
    for (name, toy, size) in [
        (
            "Pip",
            ToyState::plain(ToyColor::Galaxy, Material::Metallic),
            SizeCategory::L,
        ),
        (
            "Mecha",
            ToyState::signature(Signature::Mecha, false),
            ToyState::signature_size(),
        ),
        (
            "Bolt",
            ToyState::plain(ToyColor::Coral, Material::Plastic),
            SizeCategory::M,
        ),
    ] {
        let fish = Fish::new_toy(toy, size, &mut rng);
        tui.app.tanks[0].place_fish(fish, name.to_string(), &mut rng);
    }
    for (part, paint) in [
        (ToyPart::Rotor, Paint::Mint),
        (ToyPart::Antenna, Paint::Lemon),
        (ToyPart::SmallBoots, Paint::Lemon),
        (ToyPart::Boots, Paint::Cocoa),
        (ToyPart::Wheels, Paint::Charcoal),
        (ToyPart::Treads, Paint::Lime),
        (ToyPart::Rocket, Paint::Cherry),
    ] {
        tui.app.casino.toybox.add(FittedPart { part, paint });
    }
    tui.run("/toybox");
    tui.record(20, 2, "toys");
    tui.select("Pip");
    tui.key(KeyCode::Enter);
    tui.record(10, 2, "edit");
    tui.key(KeyCode::Down);
    tui.key(KeyCode::Right);
    tui.record(16, 2, "part");
    tui.key(KeyCode::Down);
    for _ in 0..4 {
        tui.key(KeyCode::Right);
        tui.record(16, 2, "part");
    }
    tui.key(KeyCode::Down);
    tui.key(KeyCode::Right);
    tui.record(16, 2, "part");
    tui.key(KeyCode::Enter);
    tui.record(20, 2, "saved");
}

fn double(dir: &Path) {
    let mut tui = scene(dir, "double");
    table(&mut tui, "/casino pufferfish");
    tui.key(KeyCode::Tab);
    tui.select("Adam");
    tui.key(KeyCode::Enter);
    tui.key(KeyCode::Enter);
    if let Play::Pufferfish(puffer) = play(&mut tui) {
        puffer.phase = PuffPhase::Puffing {
            clock: 0.0,
            pops_at: Multiple::whole(50),
        };
    }
    tui.record(80, 4, "puff");
    tui.key(KeyCode::Enter);
    tui.record(20, 2, "home");
    tui.key(KeyCode::Char('d'));
    tui.record(20, 2, "call");
    for (call, landing) in [
        (Side::Left, Landing::Facing(Side::Left)),
        (Side::Right, Landing::Facing(Side::Right)),
        (Side::Left, Landing::Facing(Side::Right)),
    ] {
        if let Some(Popup::Flip(flip)) = &mut state(&mut tui).popup {
            flip.phase = FlipPhase::Flying {
                call,
                landing,
                t: 0.0,
            };
        }
        tui.record(60, 2, "flip");
    }
    tui.record(80, 2, "eaten");
}

fn stupid_win(dir: &Path) {
    let mut tui = scene(dir, "stupid-win");
    table(&mut tui, "/casino bubbleup");
    tui.key(KeyCode::Up);
    tui.key(KeyCode::Up);
    for _ in 0..4 {
        tui.key(KeyCode::Right);
    }
    tui.key(KeyCode::Enter);
    if let Play::BubbleUp { bubbles, .. } = play(&mut tui) {
        let rows = bubbles[0].path.len();
        bubbles[0].path = vec![true; rows];
    }
    tui.record(260, 2, "climb");
}

fn main() -> ExitCode {
    let Some(out) = env::args().nth(1) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let dir = Path::new(&out);
    lobby(dir);
    blackjack(dir);
    spins(dir);
    pearl_dive(dir);
    pufferfish(dir);
    bubbles(dir);
    derby(dir);
    claw(dir);
    toybox(dir);
    double(dir);
    stupid_win(dir);
    ExitCode::SUCCESS
}
