//! Terminal User Interface for Santa Package Manager.
//!
//! This module provides an interactive TUI dashboard for viewing package status
//! across configured sources. It uses ratatui with a crossterm backend.
//!
//! # Features
//!
//! - Interactive navigation between sources and packages
//! - Real-time package status display (installed/missing)
//! - Keyboard-driven interface with vim-style navigation
//! - Manual refresh of package data
//!
//! # Usage
//!
//! The TUI can be launched via:
//! - `santa tui` - Direct TUI subcommand
//! - `santa dashboard` - Alias for TUI subcommand
//! - `santa status --tui` - TUI flag on status command

mod app;
mod event;
mod ui;

pub use app::App;

use crate::configuration::SantaConfig;
use crate::data::SantaData;
use crate::sources::PackageCache;
use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use event::{handle_key_event, poll_event, Action};
use ratatui::prelude::*;
use std::io::{self, stdout};
use std::time::Duration;

/// RAII guard for terminal state cleanup.
///
/// Ensures the terminal is restored to normal mode even on panic.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
    }
}

/// Run the TUI dashboard.
///
/// This function takes ownership of the terminal and runs the interactive
/// dashboard until the user quits.
///
/// # Arguments
///
/// * `config` - Santa configuration
/// * `data` - Santa data containing source definitions
/// * `cache` - Package cache for performance
///
/// # Returns
///
/// Returns `Ok(())` on normal exit, or an error if terminal operations fail.
pub async fn run_tui(config: SantaConfig, data: SantaData, cache: PackageCache) -> Result<()> {
    // Setup terminal with guard for cleanup on drop/panic
    let _guard = TerminalGuard;
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and perform initial data refresh
    let mut app = App::new(config, data, cache);
    app.refresh_data().await?;

    // Run the main event loop
    run_app(&mut terminal, &mut app).await
}

/// Main event loop for the TUI.
async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        // Draw the UI
        terminal.draw(|frame| ui::draw(frame, app))?;

        // Poll for events with a short timeout
        if let Some(crossterm::event::Event::Key(key)) = poll_event(Duration::from_millis(100))? {
            if let Some(action) = handle_key_event(key) {
                match action {
                    Action::Quit => break,
                    Action::Refresh => {
                        app.refresh_data().await?;
                    }
                    Action::Up => app.handle_up(),
                    Action::Down => app.handle_down(),
                    Action::Left => app.handle_left(),
                    Action::Right => app.handle_right(),
                    Action::Toggle => app.toggle_expand(),
                    Action::SwitchFocus => app.switch_focus(),
                    Action::Help => app.toggle_help(),
                }
            }
        }

        // Check if we should quit
        if app.should_quit {
            break;
        }
    }

    Ok(())
}
