use std::env;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Result, bail};
use learn_git_tui::app::App;
use learn_git_tui::content::Content;
use learn_git_tui::editor;
use learn_git_tui::sandbox::Sandbox;
use learn_git_tui::ui;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    // Subcomando oculto: git lo invoca como GIT_EDITOR / GIT_SEQUENCE_EDITOR.
    if args.next().is_some_and(|a| a == "__editor") {
        let Some(file) = args.next() else {
            return ExitCode::from(2);
        };
        return match editor::run_client(Path::new(&file)) {
            Ok(code) => ExitCode::from(code as u8),
            Err(e) => {
                eprintln!("editor: {e:#}");
                ExitCode::from(2)
            }
        };
    }
    match run_tui() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run_tui() -> Result<()> {
    if !Sandbox::git_available() {
        bail!("git was not found in PATH. Install git (2.34 or newer) and try again.");
    }
    let sandbox = Sandbox::new(Sandbox::default_root()?, env::current_exe()?)?;
    let mut app = App::new(sandbox, Content::load()?);
    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut app);
    app.shutdown();
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    while !app.quit {
        terminal.draw(|frame| ui::draw(frame, app))?;
        if event::poll(Duration::from_millis(50))? {
            // En Windows crossterm también emite Release/Repeat; solo nos importa Press.
            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                app.on_key(key);
            }
        }
        app.tick();
    }
    Ok(())
}
