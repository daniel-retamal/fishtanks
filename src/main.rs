use std::fmt::Display;
use std::io::{self, Write};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::terminal;
use fishtank::app::App;
use fishtank::cli::Invocation;
use fishtank::closing;
use fishtank::terminal_events::{self, Heard, TerminalEvents};
use fishtank::update::{self, Watch};
use fishtank::vault::Vault;

const FALLBACK_SIZE: (u16, u16) = (80, 24);

fn main() -> ExitCode {
    let launch = match Invocation::from_args(std::env::args()) {
        Invocation::Play(launch) => launch,
        Invocation::Help => {
            println!("{}", Invocation::help());
            return ExitCode::SUCCESS;
        }
        Invocation::Version => {
            println!("{}", Invocation::version());
            return ExitCode::SUCCESS;
        }
        Invocation::Update => return update::run(),
    };
    update::sweep();
    let vault = match Vault::open(launch) {
        Ok(vault) => vault,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let mut watch = Watch::start(&vault.path());
    let (width, height) = terminal::size().unwrap_or(FALLBACK_SIZE);
    let mut app = match App::open(launch, vault, width, height) {
        Ok(app) => app,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    closing::listen();
    let mut terminal = ratatui::init();
    let asked = terminal_events::ask_for_key_releases();
    let result = run(&mut terminal, &mut app, &mut watch);
    let saved = app.persist_and_wait();
    terminal_events::stop_asking_for_key_releases(asked);
    if let Err(error) = ratatui::try_restore() {
        warn(format!("fishtank could not tidy the terminal: {error}"));
    }
    if let Err(error) = saved {
        warn(format!("fishtank could not write its water down: {error}"));
    }
    if let Some(news) = watch.news() {
        let _ = writeln!(io::stdout(), "{news}");
    }
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            warn(error);
            ExitCode::FAILURE
        }
    }
}

fn warn(message: impl Display) {
    let _ = writeln!(io::stderr(), "{message}");
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App, watch: &mut Watch) -> Result<()> {
    let events = TerminalEvents::listen();
    let mut last_tick = Instant::now();

    loop {
        let tick_duration = Duration::from_secs_f32(1.0 / app.settings.fps);
        let timeout = tick_duration.saturating_sub(last_tick.elapsed());

        let mut heard = Some(events.wait(timeout));
        while let Some(news) = heard {
            match news {
                Heard::Event(event) => app.handle_input(event),
                Heard::Quiet => break,
                Heard::Gone(None) => return Ok(()),
                Heard::Gone(Some(error)) => return Err(error.into()),
            }
            heard = events.waiting();
        }

        if last_tick.elapsed() >= tick_duration {
            app.tick();
            last_tick = Instant::now();
        }

        if watch.poll() {
            app.announce_update();
        }

        terminal.draw(|f| app.draw(f))?;
        app.persist_if_due();

        if !app.running || closing::requested() {
            break;
        }
    }

    Ok(())
}
