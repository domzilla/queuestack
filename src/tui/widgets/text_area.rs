//! Multi-line text area widget.
//!
//! Wraps edtui for multi-line editing with line wrapping support.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use edtui::{EditorEventHandler, EditorState, EditorTheme, EditorView, Lines};
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
        Self {
            state: EditorState::default(),
            event_handler: EditorEventHandler::default(),
            label: label.into(),
        }
    }

    /// Set initial content.
    #[must_use]
    pub fn with_initial(mut self, content: &str) -> Self {
        self.state = EditorState::new(Lines::from(content));
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

        self.event_handler.on_key_event(key, &mut self.state);
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
            .block(block)
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
