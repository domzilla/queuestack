//! Single-line text input widget.
//!
//! Fully supports UTF-8 input including multi-byte characters.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

/// Single-line text input with cursor.
///
/// The cursor position is tracked as a character index (not byte index)
/// to properly handle UTF-8 multi-byte characters.
#[derive(Debug, Clone)]
pub struct TextInput {
    content: String,
    /// Cursor position as character index (0 = before first char)
    cursor: usize,
    label: String,
}

impl TextInput {
    /// Create a new text input with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            content: String::new(),
            cursor: 0,
            label: label.into(),
        }
    }

    /// Set initial content.
    #[must_use]
    pub fn with_initial(mut self, value: impl Into<String>) -> Self {
        self.content = value.into();
        self.cursor = self.content.chars().count();
        self
    }

    /// Get the current content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Check if the input is empty.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Returns the byte index for the current character cursor position.
    fn cursor_byte_index(&self) -> usize {
        self.content
            .char_indices()
            .nth(self.cursor)
            .map_or(self.content.len(), |(i, _)| i)
    }

    /// Returns the character count of the content.
    fn char_count(&self) -> usize {
        self.content.chars().count()
    }

    /// Insert text at the current cursor position.
    ///
    /// Used for paste operations. Multi-line content is flattened
    /// (newlines replaced with spaces) since this is a single-line input.
    pub fn insert_text(&mut self, text: &str) {
        // Flatten to single line - replace newlines with spaces
        let flattened: String = text
            .chars()
            .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
            .collect();

        let byte_idx = self.cursor_byte_index();
        self.content.insert_str(byte_idx, &flattened);
        self.cursor += flattened.chars().count();
    }

    /// Handle a key event.
    ///
    /// Returns `true` if the event was handled.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(c) => {
                // Handle Ctrl+key combinations first
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match c {
                        'u' => {
                            // Ctrl+U: Clear line
                            self.content.clear();
                            self.cursor = 0;
                            return true;
                        }
                        'w' => {
                            // Ctrl+W: Delete word backward
                            while self.cursor > 0 && self.char_at(self.cursor - 1) == Some(' ') {
                                self.delete_char_before_cursor();
                            }
                            while self.cursor > 0 && self.char_at(self.cursor - 1) != Some(' ') {
                                self.delete_char_before_cursor();
                            }
                            return true;
                        }
                        _ => return false, // Let other Ctrl combinations bubble up
                    }
                }
                // Regular character input
                let byte_idx = self.cursor_byte_index();
                self.content.insert(byte_idx, c);
                self.cursor += 1;
                true
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.delete_char_before_cursor();
                }
                true
            }
            KeyCode::Delete => {
                if self.cursor < self.char_count() {
                    let byte_idx = self.cursor_byte_index();
                    self.content.remove(byte_idx);
                }
                true
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                true
            }
            KeyCode::Right => {
                if self.cursor < self.char_count() {
                    self.cursor += 1;
                }
                true
            }
            KeyCode::Home => {
                self.cursor = 0;
                true
            }
            KeyCode::End => {
                self.cursor = self.char_count();
                true
            }
            _ => false,
        }
    }

    /// Returns the character at the given character index.
    fn char_at(&self, char_idx: usize) -> Option<char> {
        self.content.chars().nth(char_idx)
    }

    /// Deletes the character before the cursor and moves cursor back.
    fn delete_char_before_cursor(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            let byte_idx = self.cursor_byte_index();
            self.content.remove(byte_idx);
        }
    }

    /// Render the widget.
    pub fn render(&self, area: Rect, buf: &mut Buffer, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" {} ", self.label));

        let inner = block.inner(area);
        block.render(area, buf);

        // Render content with cursor
        if focused {
            let byte_idx = self.cursor_byte_index();
            let (before, after) = self.content.split_at(byte_idx);
            let cursor_char = after.chars().next().unwrap_or(' ');
            let after_cursor: String = after.chars().skip(1).collect();

            let line = Line::from(vec![
                Span::raw(before),
                Span::styled(
                    cursor_char.to_string(),
                    Style::default()
                        .bg(Color::White)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(after_cursor),
            ]);

            Paragraph::new(line).render(inner, buf);
        } else {
            Paragraph::new(self.content.as_str()).render(inner, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_utf8_initial_content() {
        let input = TextInput::new("Test").with_initial("日本語");
        assert_eq!(input.content(), "日本語");
        assert_eq!(input.cursor, 3); // 3 characters, not 9 bytes
    }

    #[test]
    fn test_utf8_cursor_movement() {
        let mut input = TextInput::new("Test").with_initial("über");
        assert_eq!(input.cursor, 4); // 4 characters

        // Move left
        input.handle_key(key(KeyCode::Left));
        assert_eq!(input.cursor, 3);

        // Move to start
        input.handle_key(key(KeyCode::Home));
        assert_eq!(input.cursor, 0);

        // Move right through multi-byte char 'ü'
        input.handle_key(key(KeyCode::Right));
        assert_eq!(input.cursor, 1);
        assert_eq!(input.content(), "über"); // Content unchanged
    }

    #[test]
    fn test_utf8_insert() {
        let mut input = TextInput::new("Test").with_initial("ab");
        input.handle_key(key(KeyCode::Home));
        input.handle_key(key(KeyCode::Right)); // After 'a'
        input.handle_key(key(KeyCode::Char('ü')));
        assert_eq!(input.content(), "aüb");
    }

    #[test]
    fn test_utf8_backspace() {
        let mut input = TextInput::new("Test").with_initial("日本語");
        // Cursor at end (position 3)
        input.handle_key(key(KeyCode::Backspace));
        assert_eq!(input.content(), "日本"); // Removed '語'
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn test_utf8_delete() {
        let mut input = TextInput::new("Test").with_initial("日本語");
        input.handle_key(key(KeyCode::Home)); // Move to start
        input.handle_key(key(KeyCode::Delete));
        assert_eq!(input.content(), "本語"); // Removed '日'
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn test_mixed_ascii_utf8() {
        let mut input = TextInput::new("Test").with_initial("a日b");
        assert_eq!(input.cursor, 3); // 3 characters

        input.handle_key(key(KeyCode::Home));
        input.handle_key(key(KeyCode::Right)); // After 'a'
        input.handle_key(key(KeyCode::Delete)); // Delete '日'
        assert_eq!(input.content(), "ab");
    }
}
