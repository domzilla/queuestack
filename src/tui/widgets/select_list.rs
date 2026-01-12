//! Single-select scrollable list widget.

use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget},
};

/// Actions from list interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectAction {
    /// No action, continue
    None,
    /// User confirmed selection
    Confirm,
    /// User cancelled
    Cancel,
}

/// Single-select scrollable list.
pub struct SelectList {
    items: Vec<String>,
    state: ListState,
    title: String,
    /// Indices of items that are disabled (shown but not selectable)
    disabled: HashSet<usize>,
}

impl SelectList {
    /// Create a new select list.
    pub fn new<T: ToString>(items: Vec<T>) -> Self {
        let items: Vec<String> = items.into_iter().map(|i| i.to_string()).collect();
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self {
            items,
            state,
            title: String::new(),
            disabled: HashSet::new(),
        }
    }

    /// Set the title/prompt.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set which indices are disabled (visible but not selectable).
    #[must_use]
    pub fn with_disabled(mut self, disabled: HashSet<usize>) -> Self {
        self.disabled = disabled;
        // If current selection is disabled, move to first enabled item
        if let Some(selected) = self.state.selected() {
            if self.disabled.contains(&selected) {
                self.select_first_enabled();
            }
        }
        self
    }

    /// Select the first enabled item.
    fn select_first_enabled(&mut self) {
        for i in 0..self.items.len() {
            if !self.disabled.contains(&i) {
                self.state.select(Some(i));
                return;
            }
        }
        // All items disabled - select nothing
        self.state.select(None);
    }

    /// Check if an index is enabled (not disabled).
    fn is_enabled(&self, index: usize) -> bool {
        !self.disabled.contains(&index)
    }

    /// Get the currently selected index.
    pub const fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    /// Check if list is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Move selection up, skipping disabled items.
    pub fn select_previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let len = self.items.len();

        // Try to find an enabled item going backwards
        for offset in 1..=len {
            let i = (current + len - offset) % len;
            if self.is_enabled(i) {
                self.state.select(Some(i));
                return;
            }
        }
        // No enabled items found, keep current
    }

    /// Move selection down, skipping disabled items.
    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let len = self.items.len();

        // Try to find an enabled item going forwards
        for offset in 1..=len {
            let i = (current + offset) % len;
            if self.is_enabled(i) {
                self.state.select(Some(i));
                return;
            }
        }
        // No enabled items found, keep current
    }

    /// Handle a key event.
    pub fn handle_key(&mut self, key: KeyEvent) -> SelectAction {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_previous();
                SelectAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
                SelectAction::None
            }
            KeyCode::Enter => {
                // Only confirm if selection is enabled
                if let Some(idx) = self.state.selected() {
                    if self.is_enabled(idx) {
                        return SelectAction::Confirm;
                    }
                }
                SelectAction::None
            }
            KeyCode::Esc => SelectAction::Cancel,
            _ => SelectAction::None,
        }
    }

    /// Render the widget.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(if self.title.is_empty() {
                String::new()
            } else {
                format!(" {} ", self.title)
            });

        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let is_selected = Some(i) == self.state.selected();
                let is_disabled = self.disabled.contains(&i);

                let style = if is_disabled {
                    // Disabled items shown dimmed
                    Style::default().fg(Color::DarkGray)
                } else if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let prefix = if is_selected && !is_disabled {
                    "> "
                } else {
                    "  "
                };
                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(item, style),
                ]))
            })
            .collect();

        let list = List::new(items).block(block).highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

        StatefulWidget::render(list, area, buf, &mut self.state);
    }
}

impl Clone for SelectList {
    fn clone(&self) -> Self {
        let mut new_list = Self::new(self.items.clone());
        new_list.title.clone_from(&self.title);
        new_list.disabled.clone_from(&self.disabled);
        if let Some(idx) = self.state.selected() {
            new_list.state.select(Some(idx));
        }
        new_list
    }
}
