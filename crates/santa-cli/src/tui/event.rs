//! Event handling for the TUI.
//!
//! This module handles keyboard input and maps it to actions.

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use std::time::Duration;

/// Actions that can be triggered by user input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Quit the application
    Quit,
    /// Refresh package data
    Refresh,
    /// Move selection up
    Up,
    /// Move selection down
    Down,
    /// Move to previous panel or collapse
    Left,
    /// Move to next panel or expand
    Right,
    /// Toggle expand/collapse of current item
    Toggle,
    /// Switch focus between panels
    SwitchFocus,
    /// Show help overlay
    Help,
}

/// Poll for terminal events with a timeout.
///
/// Returns `Some(Event)` if an event occurred within the timeout,
/// or `None` if the timeout expired.
pub fn poll_event(timeout: Duration) -> std::io::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

/// Process a key event and return the corresponding action.
///
/// Supports both arrow keys and vim-style navigation (h/j/k/l).
pub fn handle_key_event(key: KeyEvent) -> Option<Action> {
    // Only handle key press events, not release or repeat
    if key.kind != KeyEventKind::Press {
        return None;
    }

    match key.code {
        // Quit
        KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),

        // Refresh
        KeyCode::Char('r') => Some(Action::Refresh),

        // Navigation - up
        KeyCode::Up | KeyCode::Char('k') => Some(Action::Up),

        // Navigation - down
        KeyCode::Down | KeyCode::Char('j') => Some(Action::Down),

        // Navigation - left / collapse
        KeyCode::Left | KeyCode::Char('h') => Some(Action::Left),

        // Navigation - right / expand
        KeyCode::Right | KeyCode::Char('l') => Some(Action::Right),

        // Toggle expand/collapse
        KeyCode::Enter | KeyCode::Char(' ') => Some(Action::Toggle),

        // Switch focus between panels
        KeyCode::Tab => Some(Action::SwitchFocus),

        // Help
        KeyCode::Char('?') => Some(Action::Help),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};

    fn make_key_event(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn test_quit_keys() {
        assert_eq!(
            handle_key_event(make_key_event(KeyCode::Char('q'))),
            Some(Action::Quit)
        );
        assert_eq!(
            handle_key_event(make_key_event(KeyCode::Esc)),
            Some(Action::Quit)
        );
    }

    #[test]
    fn test_navigation_keys() {
        assert_eq!(
            handle_key_event(make_key_event(KeyCode::Up)),
            Some(Action::Up)
        );
        assert_eq!(
            handle_key_event(make_key_event(KeyCode::Char('k'))),
            Some(Action::Up)
        );
        assert_eq!(
            handle_key_event(make_key_event(KeyCode::Down)),
            Some(Action::Down)
        );
        assert_eq!(
            handle_key_event(make_key_event(KeyCode::Char('j'))),
            Some(Action::Down)
        );
    }

    #[test]
    fn test_toggle_and_focus() {
        assert_eq!(
            handle_key_event(make_key_event(KeyCode::Enter)),
            Some(Action::Toggle)
        );
        assert_eq!(
            handle_key_event(make_key_event(KeyCode::Tab)),
            Some(Action::SwitchFocus)
        );
    }

    #[test]
    fn test_unknown_key_returns_none() {
        assert_eq!(handle_key_event(make_key_event(KeyCode::Char('z'))), None);
    }
}
