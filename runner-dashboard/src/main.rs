mod app;
mod runner;
mod ui;

use anyhow::Result;
use app::{App, AppMode};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::{Duration, Instant};

fn main() -> Result<()> {
    // Setup panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new()?;

    // Run the app
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {}", err);
    }

    Ok(())
}

const REFRESH_INTERVAL_MS: u64 = 1000;

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    let refresh_rate = Duration::from_millis(REFRESH_INTERVAL_MS);
    let mut last_refresh = Instant::now();

    loop {
        // Draw UI
        terminal.draw(|f| ui::draw(f, app))?;

        // Handle events with timeout for periodic refresh
        let time_until_refresh = refresh_rate
            .checked_sub(last_refresh.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));

        if event::poll(time_until_refresh)? {
            if let Event::Key(key) = event::read()? {
                // Clear status message on any key press
                app.status_message = None;

                match app.mode {
                    AppMode::Help => {
                        // Any key exits help
                        app.mode = AppMode::Normal;
                    }
                    AppMode::Logs => {
                        handle_logs_mode(app, key.code);
                    }
                    AppMode::Normal => {
                        handle_normal_mode(app, key.code, key.modifiers);
                    }
                }

                if app.should_quit {
                    break;
                }
            }
        }

        // Periodic refresh
        if last_refresh.elapsed() >= refresh_rate {
            app.refresh();
            last_refresh = Instant::now();
        }
    }

    Ok(())
}

fn handle_normal_mode(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    match key {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => app.should_quit = true,

        // Navigation
        KeyCode::Up | KeyCode::Char('k') => app.select_previous(),
        KeyCode::Down | KeyCode::Char('j') => app.select_next(),

        // Actions
        KeyCode::Char('s') => app.start_selected(),
        KeyCode::Char('x') => app.stop_selected(),
        KeyCode::Char('r') => app.restart_selected(),
        KeyCode::Char('l') => app.toggle_logs(),

        // Help
        KeyCode::Char('?') | KeyCode::Char('h') => app.toggle_help(),

        _ => {}
    }
}

fn handle_logs_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('l') | KeyCode::Esc => app.toggle_logs(),

        // Scroll
        KeyCode::Up | KeyCode::Char('k') => app.scroll_logs_up(),
        KeyCode::Down | KeyCode::Char('j') => app.scroll_logs_down(),

        // Help
        KeyCode::Char('?') | KeyCode::Char('h') => app.toggle_help(),

        _ => {}
    }
}
