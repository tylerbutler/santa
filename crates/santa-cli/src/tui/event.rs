//! Terminal event handling for the TUI.
//!
//! Wraps crossterm events into a channel-based async event stream
//! that integrates with the ratatui rendering loop.

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::Duration;

/// Application events produced by the terminal or internal timers.
#[derive(Debug)]
pub enum Event {
    /// A key was pressed.
    Key(KeyEvent),
    /// The terminal was resized.
    Resize(u16, u16),
    /// A periodic tick for UI refresh.
    Tick,
}

/// Polls for terminal events with a given tick rate.
///
/// Returns `Some(Event)` if an event is available, `None` on timeout.
pub fn poll_event(tick_rate: Duration) -> Option<Event> {
    if event::poll(tick_rate).ok()? {
        match event::read().ok()? {
            CrosstermEvent::Key(key) => Some(Event::Key(key)),
            CrosstermEvent::Resize(w, h) => Some(Event::Resize(w, h)),
            _ => None,
        }
    } else {
        Some(Event::Tick)
    }
}
