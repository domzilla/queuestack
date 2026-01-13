//! Terminal User Interface module for qstack.
//!
//! Provides interactive TUI components using ratatui.

pub mod event;
pub mod screens;
pub mod terminal;
pub mod widgets;

use anyhow::Result;
use ratatui::Frame;

use crate::tui::event::{EventHandler, TuiEvent};
use crate::tui::terminal::TerminalGuard;

/// Result of a TUI application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppResult<T> {
    /// Application completed with a value
    Done(T),
    /// Application was cancelled by user
    Cancelled,
}

/// Trait for TUI applications.
///
/// Implement this trait to create interactive TUI screens.
pub trait TuiApp {
    /// The output type when the application completes.
    type Output;

    /// Handle an event and optionally return a result.
    ///
    /// Return `Some(AppResult)` to exit the application,
    /// or `None` to continue running.
    fn handle_event(&mut self, event: &TuiEvent) -> Option<AppResult<Self::Output>>;

    /// Render the application to the frame.
    fn render(&mut self, frame: &mut Frame);
}

/// Run a TUI application to completion.
///
/// Returns `Ok(Some(output))` if completed successfully,
/// `Ok(None)` if cancelled, or an error.
pub fn run<A: TuiApp>(mut app: A) -> Result<Option<A::Output>> {
    let mut terminal = TerminalGuard::new()?;
    let events = EventHandler::default();

    loop {
        terminal.terminal().draw(|frame| app.render(frame))?;

        let event = events.next()?;
        if let Some(result) = app.handle_event(&event) {
            return match result {
                AppResult::Done(output) => Ok(Some(output)),
                AppResult::Cancelled => Ok(None),
            };
        }
    }
}
