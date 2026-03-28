mod action;
mod app;
mod config;
mod state;
mod ui;

use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use action::{Action, Direction};
use app::{App, Pane};
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env();
    let mut app = App::new(config)?;

    // Begin fetching artists immediately.
    app.fetch_artists();

    // Set up terminal.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut app).await;

    // Restore terminal regardless of errors.
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run_loop(
    terminal: &mut ratatui::Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        // Drain library updates from background tokio tasks.
        while let Ok(update) = app.library_rx.try_recv() {
            app.apply_library_update(update);
        }
        // Drain player events from the audio thread.
        while let Ok(event) = app.player_rx.try_recv() {
            app.handle_player_event(event);
        }

        terminal.draw(|f| ui::render(app, f))?;

        // Poll for a key event (50 ms timeout keeps progress bar responsive).
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                let action = map_key(key.code, key.modifiers, app);
                app.dispatch(action);
            }
        }

        // One more drain after dispatch so any enqueued play commands reflect
        // immediately on the next frame.
        while let Ok(event) = app.player_rx.try_recv() {
            app.handle_player_event(event);
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn map_key(code: KeyCode, modifiers: KeyModifiers, app: &App) -> Action {
    match code {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('j') | KeyCode::Down => Action::Navigate(Direction::Down),
        KeyCode::Char('k') | KeyCode::Up => Action::Navigate(Direction::Up),
        KeyCode::Char('g') => Action::Navigate(Direction::Top),
        KeyCode::Char('G') => Action::Navigate(Direction::Bottom),
        KeyCode::Enter => Action::Select,
        KeyCode::Esc => Action::Back,
        KeyCode::Char('a') if modifiers.contains(KeyModifiers::SHIFT) => Action::AddAllToQueue,
        KeyCode::Char('a') => Action::AddToQueue,
        KeyCode::Char('A') => Action::AddAllToQueue,
        KeyCode::Char('p') => Action::PlayPause,
        KeyCode::Char('n') => Action::NextTrack,
        KeyCode::Char('N') => Action::PrevTrack,
        KeyCode::Char('+') | KeyCode::Char('=') => Action::VolumeUp,
        KeyCode::Char('-') => Action::VolumeDown,
        KeyCode::Tab => Action::SwitchPane(app.active_pane.next()),
        KeyCode::Char('1') => Action::SwitchPane(Pane::Artists),
        KeyCode::Char('2') => Action::SwitchPane(Pane::Albums),
        KeyCode::Char('3') => Action::SwitchPane(Pane::Tracks),
        KeyCode::Char('4') => Action::SwitchPane(Pane::Queue),
        _ => Action::None,
    }
}
