use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crossterm::event::KeyCode;

use super::{Angler, Angling, Tui};
use crate::ui::input_action::key_code;

const NOTE_PREFIX: char = '#';
const FAILED_STILL_PREFIX: &str = "FAILED";
const INCLUDE_VERB: &str = "include";
const ARGUMENT_SIGIL: char = '$';
const QUOTE: char = '"';

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Cue {
    Size {
        cols: u16,
        rows: u16,
    },
    Clear,
    Run(String),
    Type(String),
    Press {
        key: KeyCode,
        times: usize,
    },
    Release(KeyCode),
    Tick(usize),
    Snap(String),
    Select(String),
    Expect(String),
    Absent(String),
    Record {
        ticks: usize,
        every: usize,
        label: String,
    },
    Angle {
        style: Angling,
        every: usize,
        label: String,
    },
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CueError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for CueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for CueError {}

#[derive(Clone, PartialEq, Eq, Debug)]
struct Beat {
    line: usize,
    scene: Option<String>,
    cue: Cue,
}

impl Beat {
    fn failure(&self, message: String) -> CueError {
        let message = match &self.scene {
            Some(scene) => format!("{scene}: {message}"),
            None => message,
        };
        CueError {
            line: self.line,
            message,
        }
    }

    fn within(self, line: usize, scene: &str) -> Beat {
        let place = match self.scene {
            Some(inner) => format!("{scene} line {} › {inner}", self.line),
            None => format!("{scene} line {}", self.line),
        };
        Beat {
            line,
            scene: Some(place),
            cue: self.cue,
        }
    }
}

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Screenplay {
    beats: Vec<Beat>,
}

#[derive(Default)]
struct Includes {
    open: Vec<PathBuf>,
}

impl Includes {
    fn load(&mut self, path: &Path, arguments: Option<&[String]>) -> Result<Screenplay, String> {
        let located = |error: String| format!("{}: {error}", path.display());
        let identity = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if self.open.contains(&identity) {
            return Err(located("the scene includes itself".to_string()));
        }
        let source = fs::read_to_string(path).map_err(|error| located(error.to_string()))?;
        let source = match arguments {
            Some(arguments) => substitute(&source, arguments).map_err(located)?,
            None => source,
        };
        let dir = path.parent().unwrap_or(Path::new(""));
        self.open.push(identity);
        let play = Screenplay::compose(&source, Some((dir, self)));
        self.open.pop();
        play.map_err(|error| located(error.to_string()))
    }
}

impl Screenplay {
    pub fn parse(source: &str) -> Result<Self, CueError> {
        Self::compose(source, None)
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        Includes::default().load(path, None)
    }

    fn compose(
        source: &str,
        mut includes: Option<(&Path, &mut Includes)>,
    ) -> Result<Self, CueError> {
        let mut beats = Vec::new();
        for (index, raw) in source.lines().enumerate() {
            let line = index + 1;
            let text = raw.trim();
            if text.is_empty() || text.starts_with(NOTE_PREFIX) {
                continue;
            }
            let failure = |message: String| CueError { line, message };
            let (verb, rest) = split_verb(text);
            if !verb.eq_ignore_ascii_case(INCLUDE_VERB) {
                let cue = Cue::parse(verb, rest).map_err(failure)?;
                beats.push(Beat {
                    line,
                    scene: None,
                    cue,
                });
                continue;
            }
            let Some((dir, loader)) = includes.as_mut() else {
                return Err(failure(
                    "include needs a play read from a file (Screenplay::load)".to_string(),
                ));
            };
            let mut words = words(rest).map_err(failure)?.into_iter();
            let Some(scene) = words.next() else {
                return Err(failure("include needs a scene path".to_string()));
            };
            let arguments: Vec<String> = words.collect();
            let included = loader
                .load(&dir.join(&scene), Some(&arguments))
                .map_err(failure)?;
            beats.extend(
                included
                    .beats
                    .into_iter()
                    .map(|beat| beat.within(line, &scene)),
            );
        }
        Ok(Self { beats })
    }

    pub fn len(&self) -> usize {
        self.beats.len()
    }

    pub fn is_empty(&self) -> bool {
        self.beats.is_empty()
    }

    pub fn cues(&self) -> impl Iterator<Item = &Cue> {
        self.beats.iter().map(|beat| &beat.cue)
    }

    pub fn perform(&self, tui: &mut Tui) -> Result<(), CueError> {
        for beat in &self.beats {
            beat.cue
                .perform(tui)
                .map_err(|message| beat.failure(message))
                .inspect_err(|failure| {
                    tui.snap(&format!("{FAILED_STILL_PREFIX} · {failure}"));
                })?;
        }
        Ok(())
    }
}

fn split_verb(text: &str) -> (&str, &str) {
    let (verb, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
    (verb, rest.trim())
}

fn words(text: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut rest = text.trim_start();
    while !rest.is_empty() {
        let (word, after) = match rest.strip_prefix(QUOTE) {
            Some(quoted) => quoted
                .split_once(QUOTE)
                .ok_or_else(|| format!("unclosed {QUOTE} in {text:?}"))?,
            None => rest.split_once(char::is_whitespace).unwrap_or((rest, "")),
        };
        words.push(word.to_string());
        rest = after.trim_start();
    }
    Ok(words)
}

fn substitute(source: &str, arguments: &[String]) -> Result<String, String> {
    let mut used = vec![false; arguments.len()];
    let mut text = String::with_capacity(source.len());
    for (index, line) in source.split_inclusive('\n').enumerate() {
        let mut chars = line.chars().peekable();
        while let Some(ch) = chars.next() {
            let slot = chars
                .peek()
                .and_then(|next| next.to_digit(10))
                .filter(|&number| number > 0);
            let (ARGUMENT_SIGIL, Some(number)) = (ch, slot) else {
                text.push(ch);
                continue;
            };
            chars.next();
            let slot = number as usize - 1;
            let Some(argument) = arguments.get(slot) else {
                return Err(format!(
                    "line {}: uses {ARGUMENT_SIGIL}{number} but was given {} argument(s)",
                    index + 1,
                    arguments.len()
                ));
            };
            used[slot] = true;
            text.push_str(argument);
        }
    }
    if let Some(unused) = used.iter().position(|&was| !was) {
        return Err(format!(
            "was given {ARGUMENT_SIGIL}{} but never uses it",
            unused + 1
        ));
    }
    Ok(text)
}

impl Cue {
    fn parse(verb: &str, rest: &str) -> Result<Cue, String> {
        match verb.to_ascii_lowercase().as_str() {
            "size" => parse_size(rest),
            "clear" => Ok(Cue::Clear),
            "run" => required(rest, "run needs a command line").map(Cue::Run),
            "type" => required(rest, "type needs text").map(Cue::Type),
            "key" => parse_key(rest),
            "release" => parse_release(rest),
            "tick" => parse_count(rest).map(Cue::Tick),
            "snap" => required(rest, "snap needs a label").map(Cue::Snap),
            "select" => required(rest, "select needs a row label").map(Cue::Select),
            "expect" => required(rest, "expect needs text").map(Cue::Expect),
            "absent" => required(rest, "absent needs text").map(Cue::Absent),
            "record" => parse_record(rest),
            "angle" => parse_angle(rest),
            other => Err(format!(
                "unknown cue {other:?} (size, clear, run, type, key, release, tick, snap, select, expect, absent, record, angle, include)"
            )),
        }
    }

    fn perform(&self, tui: &mut Tui) -> Result<(), String> {
        match self {
            Cue::Size { cols, rows } => {
                tui.resize(*cols, *rows);
            }
            Cue::Clear => {
                tui.clear_tank();
            }
            Cue::Run(line) => {
                tui.run(line);
            }
            Cue::Type(text) => {
                tui.type_text(text);
            }
            Cue::Press { key, times } => {
                for _ in 0..*times {
                    tui.key(*key);
                }
            }
            Cue::Release(key) => {
                tui.release(*key);
            }
            Cue::Tick(ticks) => {
                tui.tick_n(*ticks);
            }
            Cue::Snap(label) => {
                tui.snap(label);
            }
            Cue::Select(label) => {
                tui.try_select(label)?;
            }
            Cue::Expect(needle) => {
                if !tui.screen().contains(needle) {
                    return Err(format!("expected {needle:?} on screen"));
                }
            }
            Cue::Absent(needle) => {
                if tui.screen().contains(needle) {
                    return Err(format!("expected {needle:?} to be absent"));
                }
            }
            Cue::Record {
                ticks,
                every,
                label,
            } => {
                tui.record(*ticks, *every, label);
            }
            Cue::Angle {
                style,
                every,
                label,
            } => {
                Angler::new(*style, *every, label).cast(tui)?;
            }
        }
        Ok(())
    }
}

fn required(rest: &str, message: &str) -> Result<String, String> {
    if rest.is_empty() {
        return Err(message.to_string());
    }
    Ok(rest.to_string())
}

fn parse_count(text: &str) -> Result<usize, String> {
    text.parse()
        .map_err(|_| format!("{text:?} is not a whole number"))
}

fn parse_size(rest: &str) -> Result<Cue, String> {
    let mut parts = rest.split_whitespace();
    let (Some(cols), Some(rows)) = (parts.next(), parts.next()) else {
        return Err("size needs <cols> <rows>".to_string());
    };
    let cols = cols
        .parse()
        .map_err(|_| format!("{cols:?} is not a column count"))?;
    let rows = rows
        .parse()
        .map_err(|_| format!("{rows:?} is not a row count"))?;
    Ok(Cue::Size { cols, rows })
}

fn parse_key(rest: &str) -> Result<Cue, String> {
    let mut parts = rest.split_whitespace();
    let Some(name) = parts.next() else {
        return Err("key needs a key name".to_string());
    };
    let times = match parts.next() {
        Some(count) => parse_count(count)?,
        None => 1,
    };
    let key = key_code(name).ok_or_else(|| format!("unknown key {name:?}"))?;
    Ok(Cue::Press { key, times })
}

fn parse_record(rest: &str) -> Result<Cue, String> {
    let mut parts = rest.splitn(3, char::is_whitespace);
    let (Some(ticks), Some(every), Some(label)) = (parts.next(), parts.next(), parts.next()) else {
        return Err("record needs <ticks> <every> <label>".to_string());
    };
    Ok(Cue::Record {
        ticks: parse_count(ticks)?,
        every: parse_count(every)?,
        label: label.trim().to_string(),
    })
}

fn parse_angle(rest: &str) -> Result<Cue, String> {
    let mut parts = rest.splitn(3, char::is_whitespace);
    let (Some(style), Some(every), Some(label)) = (parts.next(), parts.next(), parts.next()) else {
        return Err("angle needs steer|hold <every> <label>".to_string());
    };
    let style = Angling::parse(style).ok_or_else(|| format!("{style:?} is not steer or hold"))?;
    Ok(Cue::Angle {
        style,
        every: parse_count(every)?,
        label: label.trim().to_string(),
    })
}

fn parse_release(rest: &str) -> Result<Cue, String> {
    let name = required(rest, "release needs a key name")?;
    key_code(&name)
        .map(Cue::Release)
        .ok_or_else(|| format!("unknown key {name:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_and_blank_lines_are_not_cues() {
        let play = Screenplay::parse("# a note\n\n   \nclear\n").expect("parses");
        assert_eq!(play.len(), 1);
    }

    #[test]
    fn every_cue_parses_the_way_it_reads() {
        let play = Screenplay::parse(
            "size 80 24\nrun /spawn botfish \"neo\"\ntype hello world\nkey down 3\nkey q\n\
             key Enter\nrelease space\ntick 40\nsnap The panel\nexpect ENTER\nabsent too big",
        )
        .expect("parses");
        let cues: Vec<Cue> = play.beats.into_iter().map(|beat| beat.cue).collect();
        assert_eq!(
            cues,
            vec![
                Cue::Size { cols: 80, rows: 24 },
                Cue::Run("/spawn botfish \"neo\"".to_string()),
                Cue::Type("hello world".to_string()),
                Cue::Press {
                    key: KeyCode::Down,
                    times: 3
                },
                Cue::Press {
                    key: KeyCode::Char('q'),
                    times: 1
                },
                Cue::Press {
                    key: KeyCode::Enter,
                    times: 1
                },
                Cue::Release(KeyCode::Char(' ')),
                Cue::Tick(40),
                Cue::Snap("The panel".to_string()),
                Cue::Expect("ENTER".to_string()),
                Cue::Absent("too big".to_string()),
            ]
        );
    }

    #[test]
    fn record_and_angle_parse_their_counts_and_labels() {
        let play = Screenplay::parse("record 90 3 the engulfment\nangle steer 2 a steady reel")
            .expect("parses");
        let cues: Vec<Cue> = play.beats.into_iter().map(|beat| beat.cue).collect();
        assert_eq!(
            cues,
            vec![
                Cue::Record {
                    ticks: 90,
                    every: 3,
                    label: "the engulfment".to_string()
                },
                Cue::Angle {
                    style: Angling::Steer,
                    every: 2,
                    label: "a steady reel".to_string()
                },
            ]
        );
    }

    #[test]
    fn record_films_one_still_every_few_ticks() {
        let play = Screenplay::parse("record 9 3 water").expect("parses");
        let mut tui = Tui::new();
        play.perform(&mut tui).expect("performs");
        assert_eq!(tui.reel().stills().len(), 3);
    }

    #[test]
    fn an_angler_needs_the_fishing_window_open() {
        let play = Screenplay::parse("angle steer 1 nothing").expect("parses");
        let error = play.perform(&mut Tui::new()).expect_err("refuses");
        assert!(error.message.contains("/fish"), "{error}");
    }

    #[test]
    fn a_bad_cue_names_its_line() {
        let error = Screenplay::parse("clear\n\nwiggle the fish").expect_err("rejects");
        assert_eq!(error.line, 3);
        assert!(error.message.contains("wiggle"), "{error}");
    }

    #[test]
    fn a_failed_expectation_stops_the_play_and_leaves_the_screen_on_the_reel() {
        let play =
            Screenplay::parse("clear\nexpect no such words anywhere\nclear").expect("parses");
        let mut tui = Tui::new();
        let error = play.perform(&mut tui).expect_err("the expectation fails");
        assert_eq!(error.line, 2);
        let stills = tui.reel().stills();
        assert_eq!(stills.len(), 1, "the failing screen is kept for inspection");
        assert!(stills[0].label.starts_with(FAILED_STILL_PREFIX));
    }

    fn stage_dir(test: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("fishtank-screenplay-{}-{test}", std::process::id()));
        fs::create_dir_all(dir.join("scenes")).expect("the stage directory is writable");
        dir
    }

    fn write(dir: &Path, file: &str, source: &str) -> PathBuf {
        let path = dir.join(file);
        fs::write(&path, source).expect("the play is writable");
        path
    }

    #[test]
    fn an_included_scene_plays_in_place_with_its_arguments_filled_in() {
        let dir = stage_dir("in-place");
        write(
            &dir,
            "scenes/pick.play",
            "# $1 is a row\nselect $1\nkey down $2\n",
        );
        let main = write(
            &dir,
            "main.play",
            "clear\ninclude scenes/pick.play \"Inverter Coil\" 3\ntick 2\n",
        );
        let play = Screenplay::load(&main).expect("loads");
        let cues: Vec<&Cue> = play.cues().collect();
        assert_eq!(
            cues,
            vec![
                &Cue::Clear,
                &Cue::Select("Inverter Coil".to_string()),
                &Cue::Press {
                    key: KeyCode::Down,
                    times: 3
                },
                &Cue::Tick(2),
            ]
        );
    }

    #[test]
    fn a_scene_that_includes_itself_is_refused() {
        let dir = stage_dir("loop");
        write(&dir, "scenes/echo.play", "clear\ninclude echo.play\n");
        let main = write(&dir, "main.play", "include scenes/echo.play\n");
        let error = Screenplay::load(&main).expect_err("refuses the loop");
        assert!(error.contains("includes itself"), "{error}");
    }

    #[test]
    fn a_scene_is_given_exactly_the_arguments_it_uses() {
        let dir = stage_dir("arguments");
        write(&dir, "scenes/pick.play", "select $1\nkey down $2\n");
        let short = write(&dir, "short.play", "include scenes/pick.play Coil\n");
        let error = Screenplay::load(&short).expect_err("refuses a missing argument");
        assert!(error.contains("line 2: uses $2"), "{error}");
        let long = write(&dir, "long.play", "include scenes/pick.play Coil 1 2\n");
        let error = Screenplay::load(&long).expect_err("refuses an unused argument");
        assert!(error.contains("$3 but never uses it"), "{error}");
    }

    #[test]
    fn a_play_parsed_from_text_has_no_directory_to_include_from() {
        let error = Screenplay::parse("clear\ninclude scenes/pick.play").expect_err("refuses");
        assert_eq!(error.line, 2);
        assert!(error.message.contains("include"), "{error}");
    }

    #[test]
    fn a_failure_inside_a_scene_names_the_including_line_and_the_scene_line() {
        let dir = stage_dir("failure");
        write(
            &dir,
            "scenes/check.play",
            "clear\nexpect no such words anywhere\n",
        );
        let main = write(&dir, "main.play", "clear\ninclude scenes/check.play\n");
        let play = Screenplay::load(&main).expect("loads");
        let error = play.perform(&mut Tui::new()).expect_err("the scene fails");
        assert_eq!(error.line, 2, "the line is the journey's own include");
        assert!(
            error
                .message
                .starts_with("scenes/check.play line 2: expected"),
            "{error}"
        );
    }

    #[test]
    fn a_resize_redraws_the_app_at_the_new_size() {
        let play = Screenplay::parse("size 60 20\nsnap small").expect("parses");
        let mut tui = Tui::new();
        play.perform(&mut tui).expect("performs");
        let still = &tui.reel().stills()[0];
        assert_eq!((still.width(), still.height()), (60, 20));
    }
}
