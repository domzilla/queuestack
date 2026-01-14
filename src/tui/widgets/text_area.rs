//! Multi-line text area widget.
//!
//! Wraps edtui for multi-line editing with line wrapping support.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use edtui::{
    actions::{Execute, MoveWordBackward, MoveWordForward},
    EditorEventHandler, EditorMode, EditorState, EditorTheme, EditorView, Lines,
};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders},
    Frame,
};

/// Multi-line text area widget with line wrapping.
pub struct TextAreaWidget {
    state: EditorState,
    event_handler: EditorEventHandler,
    label: String,
}

impl TextAreaWidget {
    /// Create a new text area with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        let mut state = EditorState::default();
        // Always use insert mode - disable vim-style modal editing
        state.mode = EditorMode::Insert;
        Self {
            state,
            event_handler: EditorEventHandler::default(),
            label: label.into(),
        }
    }

    /// Set initial content.
    #[must_use]
    pub fn with_initial(mut self, content: &str) -> Self {
        self.state = EditorState::new(Lines::from(content));
        // Always use insert mode - disable vim-style modal editing
        self.state.mode = EditorMode::Insert;
        self
    }

    /// Get the current content as a string.
    pub fn content(&self) -> String {
        self.state.lines.to_string()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.state.lines.is_empty()
    }

    /// Insert text at the current cursor position.
    ///
    /// Used for paste operations. Supports multi-line text.
    pub fn insert_text(&mut self, text: &str) {
        self.event_handler
            .on_paste_event(text.to_string(), &mut self.state);
    }

    /// Handle a key event.
    ///
    /// Returns `true` if the event was consumed by the text area.
    /// Navigation keys (Ctrl+N, Ctrl+P) are NOT consumed.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Don't consume Ctrl+N - used for navigation
        if key.code == KeyCode::Char('n') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return false;
        }
        // Don't consume Ctrl+P - used for navigation
        if key.code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return false;
        }
        // Don't consume Esc
        if key.code == KeyCode::Esc {
            return false;
        }
        // Don't consume Ctrl+C
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return false;
        }

        // Custom navigation (standard editor behavior)
        match key.code {
            // Option+Left: word backward
            KeyCode::Left if key.modifiers.contains(KeyModifiers::ALT) => {
                MoveWordBackward(1).execute(&mut self.state);
                return true;
            }
            // Option+Right: word forward
            KeyCode::Right if key.modifiers.contains(KeyModifiers::ALT) => {
                MoveWordForward(1).execute(&mut self.state);
                return true;
            }
            // Left at start of line: wrap to end of previous line
            KeyCode::Left if self.state.cursor.col == 0 && self.state.cursor.row > 0 => {
                self.state.cursor.row -= 1;
                self.state.cursor.col =
                    self.state.lines.len_col(self.state.cursor.row).unwrap_or(0);
                return true;
            }
            // Right at end of line: wrap to start of next line
            KeyCode::Right => {
                let line_len = self.state.lines.len_col(self.state.cursor.row).unwrap_or(0);
                let last_row = self.state.lines.len().saturating_sub(1);
                if self.state.cursor.col >= line_len && self.state.cursor.row < last_row {
                    self.state.cursor.row += 1;
                    self.state.cursor.col = 0;
                    return true;
                }
            }
            _ => {}
        }

        self.event_handler.on_key_event(key, &mut self.state);
        // Force insert mode - prevent vim-style mode switching
        self.state.mode = EditorMode::Insert;
        true
    }

    /// Render the widget.
    pub fn render(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" {} ", self.label));

        let theme = EditorTheme::default()
            .base(Style::default()) // Transparent background
            .block(block)
            .hide_status_line() // No vim mode indicator
            .cursor_style(if focused {
                Style::default().bg(Color::White).fg(Color::Black)
            } else {
                Style::default()
            });

        let view = EditorView::new(&mut self.state).theme(theme).wrap(true);

        frame.render_widget(view, area);
    }
}

impl Clone for TextAreaWidget {
    fn clone(&self) -> Self {
        let content = self.content();
        Self::new(self.label.clone()).with_initial(&content)
    }
}
