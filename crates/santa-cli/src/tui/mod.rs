//! Interactive Terminal User Interface for Santa.
//!
//! Provides a full-screen TUI that displays configured sources and packages,
//! shows installation status, and allows selecting and installing missing packages.
//!
//! # Usage
//!
//! ```bash
//! santa tui                    # Safe mode (generates scripts)
//! santa tui -x                 # Execute mode (direct install)
//! ```
//!
//! # Keybindings
//!
//! - `↑`/`↓` or `k`/`j`: Navigate packages
//! - `Space`: Toggle selection on a missing package
//! - `a`: Select all missing packages
//! - `d`: Deselect all
//! - `i`: Install selected packages
//! - `r`: Refresh package data
//! - `q` or `Esc`: Quit

pub mod app;
pub mod event;
pub mod ui;

use app::{App, AppPhase};
use crossterm::event::{KeyCode, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::time::Duration;

use crate::configuration::SantaConfig;
use crate::data::SantaData;
use crate::script_generator::{ExecutionMode, ScriptFormat};

/// Run the interactive TUI.
///
/// This takes ownership of the terminal, renders the UI, and returns
/// control when the user quits.
pub async fn run_tui(
    config: SantaConfig,
    data: SantaData,
    execution_mode: ExecutionMode,
    script_format: ScriptFormat,
    output_dir: std::path::PathBuf,
) -> anyhow::Result<()> {
    let mut app = App::new(config, data, execution_mode, script_format, output_dir);

    // Set up terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Draw loading state, then load data
    terminal.draw(|f| ui::render(f, &app))?;
    app.load_data().await?;

    let tick_rate = Duration::from_millis(100);

    // Main event loop
    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        if let Some(evt) = event::poll_event(tick_rate) {
            match evt {
                event::Event::Key(key) => {
                    // Dismiss message popup on any key
                    if matches!(app.phase, AppPhase::Message(_)) {
                        app.phase = AppPhase::Ready;
                        continue;
                    }

                    if app.phase != AppPhase::Ready {
                        continue;
                    }

                    match (key.code, key.modifiers) {
                        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => {
                            app.should_quit = true;
                        }
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            app.should_quit = true;
                        }
                        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
                            app.cursor_up();
                        }
                        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
                            app.cursor_down();
                        }
                        (KeyCode::Char(' '), _) | (KeyCode::Enter, _) => {
                            app.toggle_selection();
                        }
                        (KeyCode::Char('a'), _) => {
                            app.select_all_missing();
                        }
                        (KeyCode::Char('d'), _) => {
                            app.deselect_all();
                        }
                        (KeyCode::Char('i'), _) => {
                            if app.selected_count() > 0 {
                                app.phase = AppPhase::Installing;
                                // We need to leave the alternate screen for script
                                // generation output, then re-enter.
                                disable_raw_mode()?;
                                stdout().execute(LeaveAlternateScreen)?;

                                let result = app.install_selected();

                                // Re-enter TUI mode
                                enable_raw_mode()?;
                                stdout().execute(EnterAlternateScreen)?;
                                terminal.clear()?;

                                match result {
                                    Ok(msg) => {
                                        // Reload data to reflect changes
                                        app.phase = AppPhase::Loading;
                                        terminal.draw(|f| ui::render(f, &app))?;
                                        let _ = app.load_data().await;
                                        app.phase = AppPhase::Message(msg);
                                    }
                                    Err(e) => {
                                        app.phase =
                                            AppPhase::Message(format!("Error: {e}"));
                                    }
                                }
                            }
                        }
                        (KeyCode::Char('r'), _) => {
                            app.phase = AppPhase::Loading;
                            terminal.draw(|f| ui::render(f, &app))?;
                            let _ = app.load_data().await;
                        }
                        _ => {}
                    }
                }
                event::Event::Resize(_, _) => {
                    // Terminal will redraw on next iteration
                }
                event::Event::Tick => {}
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
