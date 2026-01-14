//! Multi-line text area widget.
//!
//! Wraps tui-textarea for multi-line editing.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders},
    Frame,
};
use tui_textarea::TextArea;

/// Multi-line text area widget.
pub struct TextAreaWidget<'a> {
    textarea: TextArea<'a>,
    label: String,
}

impl TextAreaWidget<'_> {
    /// Create a new text area with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(Style::default());
        Self {
            textarea,
            label: label.into(),
        }
    }

    /// Set initial content.
    #[must_use]
    pub fn with_initial(mut self, content: &str) -> Self {
        let lines: Vec<String> = content.lines().map(String::from).collect();
        self.textarea = TextArea::new(lines);
        self.textarea.set_cursor_line_style(Style::default());
        self
    }

    /// Get the current content as a string.
    pub fn content(&self) -> String {
        self.textarea.lines().join("\n")
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.textarea.is_empty()
    }

    /// Insert text at the current cursor position.
    ///
    /// Used for paste operations. Supports multi-line text.
    pub fn insert_text(&mut self, text: &str) {
        self.textarea.insert_str(text);
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

        self.textarea.input(key);
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

        self.textarea.set_block(block);

        if focused {
            self.textarea
                .set_cursor_style(Style::default().bg(Color::White).fg(Color::Black));
        } else {
            self.textarea.set_cursor_style(Style::default());
        }

        frame.render_widget(&self.textarea, area);
    }
}

impl Clone for TextAreaWidget<'_> {
    fn clone(&self) -> Self {
        let content = self.content();
        Self::new(self.label.clone()).with_initial(&content)
    }
}
