//! Confirmation dialog screen.
//!
//! A simple yes/no modal dialog for confirming destructive actions.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::tui::{event::TuiEvent, run, AppResult, TuiApp};

/// Confirmation dialog state.
pub struct ConfirmDialog {
    message: String,
    selected: bool, // true = Yes, false = No
}

impl ConfirmDialog {
    /// Create a new confirmation dialog.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            selected: false, // Default to "No" for safety
        }
    }

    /// Calculate the popup dimensions based on content.
    #[allow(clippy::cast_possible_truncation)]
    fn popup_size(&self) -> (u16, u16) {
        let msg_width = self.message.width() as u16;
        // Width: message width + padding, minimum for buttons
        let width = msg_width.max(20) + 4;
        // Height: message + buttons + borders
        let height = 5;
        (width, height)
    }

    fn render_popup(&self, area: Rect, buf: &mut Buffer) {
        let (width, height) = self.popup_size();
        let popup_area = centered_rect(width, height, area);

        // Dim the background
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_style(
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::DIM),
                    );
                }
            }
        }

        // Clear the popup area
        Clear.render(popup_area, buf);

        // Render the popup block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Confirm ");

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        // Render message
        let msg = Paragraph::new(self.message.as_str());
        let msg_area = Rect::new(inner.x, inner.y, inner.width, 1);
        msg.render(msg_area, buf);

        // Render buttons
        let yes_style = if self.selected {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let no_style = if self.selected {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        };

        let buttons = Line::from(vec![
            Span::raw("  "),
            Span::styled(if self.selected { "[Yes]" } else { " Yes " }, yes_style),
            Span::raw("   "),
            Span::styled(if self.selected { " No " } else { "[No]" }, no_style),
        ]);

        let buttons_para = Paragraph::new(buttons);
        let buttons_area = Rect::new(inner.x, inner.y + 2, inner.width, 1);
        buttons_para.render(buttons_area, buf);
    }
}

impl TuiApp for ConfirmDialog {
    type Output = bool;

    fn handle_event(&mut self, event: &TuiEvent) -> Option<AppResult<Self::Output>> {
        match event {
            TuiEvent::Key(key) => {
                // Handle Ctrl+C
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Some(AppResult::Cancelled);
                }

                match key.code {
                    // Quick keys for Yes/No
                    KeyCode::Char('y' | 'Y') => Some(AppResult::Done(true)),
                    KeyCode::Char('n' | 'N') => Some(AppResult::Done(false)),

                    // Arrow navigation
                    KeyCode::Left | KeyCode::Char('h') => {
                        self.selected = true; // Yes is on left
                        None
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        self.selected = false; // No is on right
                        None
                    }
                    KeyCode::Tab => {
                        self.selected = !self.selected;
                        None
                    }

                    KeyCode::Enter => Some(AppResult::Done(self.selected)),
                    KeyCode::Esc => Some(AppResult::Cancelled),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        self.render_popup(frame.area(), frame.buffer_mut());
    }
}

/// Calculate a centered rectangle within the given area.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

/// Show a confirmation dialog.
///
/// Returns `Ok(Some(true))` if confirmed, `Ok(Some(false))` if declined,
/// or `Ok(None)` if cancelled (Esc/Ctrl+C).
pub fn confirm(message: &str) -> Result<Option<bool>> {
    let dialog = ConfirmDialog::new(message);
    run(dialog)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyModifiers};

    fn key_event(code: KeyCode) -> TuiEvent {
        TuiEvent::Key(KeyEvent::new(code, KeyModifiers::empty()))
    }

    #[test]
    fn test_default_is_no() {
        let dialog = ConfirmDialog::new("Test?");
        assert!(!dialog.selected);
    }

    #[test]
    fn test_quick_yes() {
        let mut dialog = ConfirmDialog::new("Test?");
        let result = dialog.handle_event(&key_event(KeyCode::Char('y')));
        assert_eq!(result, Some(AppResult::Done(true)));
    }

    #[test]
    fn test_quick_no() {
        let mut dialog = ConfirmDialog::new("Test?");
        let result = dialog.handle_event(&key_event(KeyCode::Char('n')));
        assert_eq!(result, Some(AppResult::Done(false)));
    }

    #[test]
    fn test_arrow_navigation() {
        let mut dialog = ConfirmDialog::new("Test?");
        assert!(!dialog.selected); // Default No

        dialog.handle_event(&key_event(KeyCode::Left));
        assert!(dialog.selected); // Now Yes

        dialog.handle_event(&key_event(KeyCode::Right));
        assert!(!dialog.selected); // Back to No
    }

    #[test]
    fn test_tab_toggle() {
        let mut dialog = ConfirmDialog::new("Test?");
        assert!(!dialog.selected);

        dialog.handle_event(&key_event(KeyCode::Tab));
        assert!(dialog.selected);

        dialog.handle_event(&key_event(KeyCode::Tab));
        assert!(!dialog.selected);
    }

    #[test]
    fn test_enter_confirms_selection() {
        let mut dialog = ConfirmDialog::new("Test?");
        dialog.selected = true;
        let result = dialog.handle_event(&key_event(KeyCode::Enter));
        assert_eq!(result, Some(AppResult::Done(true)));
    }

    #[test]
    fn test_escape_cancels() {
        let mut dialog = ConfirmDialog::new("Test?");
        let result = dialog.handle_event(&key_event(KeyCode::Esc));
        assert_eq!(result, Some(AppResult::Cancelled));
    }
}
