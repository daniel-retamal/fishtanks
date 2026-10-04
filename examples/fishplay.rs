use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fishtank::app::Launch;
use fishtank::testing::{DEFAULT_COLS, DEFAULT_ROWS, Screenplay, Tui};

const USAGE: &str = "usage: cargo run --example fishplay -- <play-file> [--out <dir>] [--size <cols> <rows>] [--print] [--player]

Performs a screenplay against the real App and films every `snap` cue.
--size starts the terminal at that size, which is how a journey is checked on a small screen.
--player plays as a player, the way every journey runs; without it the play is the debug lab bench.
Cues, one per line (# starts a note):
  size <cols> <rows>     resize the terminal (starts at 100 30)
  clear                  empty the current tank of fish, cows and food
  run <command line>     type a line into the command bar and press ENTER
  type <text>            type text into whatever has focus
  typewrite <k> <text>   type text one letter at a time, k ticks apart, filming each letter
  key <name> [times]     press a key: enter esc tab backtab backspace delete
                         up down left right home end pageup pagedown space, or one character
  tick <n>               advance the simulation n ticks
  snap <label>           film the screen as a still
  select <label>         press down until the row reads `> <label>` (downward only)
  expect <text>          fail unless the text is on screen
  absent <text>          fail if the text is on screen
  record <n> <k> <label> advance n ticks, filming a still every k ticks (for a GIF)
  angle steer|hold|watch <k> <label>
                         play the open /fish window to its end like a player: steer
                         against the fish and reel on green, hold down and nothing
                         else, or press nothing; a still every k ticks (0 films nothing)
  include <scene> [args] perform another play in place, path relative to this file;
                         $1..$9 in the scene are its arguments (quote one with spaces)

The reel is written as <out>/reels/<play-name>.txt (ruled text) and .html (colour).";
const OUT_FLAG: &str = "--out";
const SIZE_FLAG: &str = "--size";
const PRINT_FLAG: &str = "--print";
const PLAYER_FLAG: &str = "--player";
const TARGET_DIR_VAR: &str = "CARGO_TARGET_DIR";
const DEFAULT_TARGET_DIR: &str = "target";
const PLAY_DIR: &str = "fishplay";
const TEXT_EXTENSION: &str = "txt";

fn default_out() -> PathBuf {
    let target = env::var(TARGET_DIR_VAR).unwrap_or_else(|_| DEFAULT_TARGET_DIR.to_string());
    Path::new(&target).join(PLAY_DIR)
}

fn flag_value(args: &[String], flag: &str) -> Option<PathBuf> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| PathBuf::from(&pair[1]))
}

fn size_value(args: &[String]) -> Option<(u16, u16)> {
    let triple = args.windows(3).find(|triple| triple[0] == SIZE_FLAG)?;
    Some((triple[1].parse().ok()?, triple[2].parse().ok()?))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(script) = args.first().filter(|first| !first.starts_with("--")) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let out = flag_value(&args, OUT_FLAG).unwrap_or_else(default_out);
    let print = args.iter().any(|arg| arg == PRINT_FLAG);
    let path = Path::new(script);
    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(PLAY_DIR);

    let play = match Screenplay::load(path) {
        Ok(play) => play,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };

    let (cols, rows) = size_value(&args).unwrap_or((DEFAULT_COLS, DEFAULT_ROWS));
    let launch = if args.iter().any(|arg| arg == PLAYER_FLAG) {
        Launch::Player
    } else {
        Launch::Debug
    };
    let mut tui = Tui::launched(launch, cols, rows);
    tui.film(&out, name);
    let outcome = play.perform(&mut tui);
    let reel = tui.reel();
    let html = match reel.save(&out, name) {
        Ok(html) => html,
        Err(error) => {
            eprintln!("cannot write the reel under {}: {error}", out.display());
            return ExitCode::FAILURE;
        }
    };

    if print {
        print!("{}", reel.text(name));
    }
    println!(
        "{} cue(s), {} still(s)\n  text: {}\n  html: {}",
        play.len(),
        reel.stills().len(),
        html.with_extension(TEXT_EXTENSION).display(),
        html.display()
    );

    let flaws = reel.flaw_report();
    if !flaws.is_empty() {
        println!("broken borders:\n{flaws}");
    }
    if let Err(failure) = &outcome {
        println!("the play stopped at {failure}");
    }
    if outcome.is_err() || !flaws.is_empty() {
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
