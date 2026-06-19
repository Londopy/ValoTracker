use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

mod app;
mod events;
mod ui;

use app::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Set up tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(io::stderr)
        .init();

    // Enter alternate screen / raw mode
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app
    let result = run(&mut terminal).await;

    // Restore terminal on exit (even on error)
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // if we grabbed an installer, run it now that the terminals back to normal,
    // then were done
    if let Ok(Some(installer)) = &result {
        let _ = valotracker_core::updater::spawn_installer(installer);
    }

    result.map(|_| ())
}

async fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<Option<std::path::PathBuf>> {
    let mut app = App::new().await;

    loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        if events::handle_events(&mut app).await? {
            break; // quit signal
        }

        app.tick().await;

        // installer is ready, bounce so it can swap out the exe
        if app.install_pending.is_some() {
            break;
        }
    }

    Ok(app.install_pending.take())
}
