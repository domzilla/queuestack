//! Multi-select list widget with checkboxes.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget},
};

/// Actions from multi-select interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiSelectAction {
    /// No action, continue
    None,
    /// User confirmed selection
    Confirm,
    /// User cancelled
    Cancel,
}

/// Multi-select list with checkboxes.
pub struct MultiSelect {
    items: Vec<(String, bool)>,
    state: ListState,
    title: String,
    /// Index of an "action" item (like "+ Add new...") that has no checkbox.
    action_item_index: Option<usize>,
}

impl MultiSelect {
    /// Create a new multi-select list.
    pub fn new<T: ToString>(items: Vec<T>) -> Self {
        let items: Vec<(String, bool)> =
            items.into_iter().map(|i| (i.to_string(), false)).collect();
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self {
            items,
            state,
            title: String::new(),
            action_item_index: None,
        }
    }

    /// Mark the last item as an action item (no checkbox).
    #[must_use]
    pub fn with_action_item_last(mut self) -> Self {
        if !self.items.is_empty() {
            self.action_item_index = Some(self.items.len() - 1);
        }
        self
    }

    /// Check if an index is the action item.
    fn is_action_item(&self, index: usize) -> bool {
        self.action_item_index == Some(index)
    }

    /// Set the title.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Pre-select items by their labels.
    #[must_use]
    pub fn with_selected(mut self, labels: &[String]) -> Self {
        for (item, selected) in &mut self.items {
            *selected = labels.contains(item);
        }
        self
    }

    /// Get the selected item labels (excludes action items).
    pub fn selected_items(&self) -> Vec<&str> {
        self.items
            .iter()
            .enumerate()
            .filter(|(i, (_, selected))| *selected && !self.is_action_item(*i))
            .map(|(_, (item, _))| item.as_str())
            .collect()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get the number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Get the currently highlighted index (cursor position).
    pub const fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    /// Toggle the currently selected item (unless it's an action item).
    pub fn toggle_current(&mut self) {
        if let Some(i) = self.state.selected() {
            // Don't toggle action items
            if self.is_action_item(i) {
                return;
            }
            if let Some((_, selected)) = self.items.get_mut(i) {
                *selected = !*selected;
            }
        }
    }

    /// Move selection up.
    pub fn select_previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// Add a new item to the list (inserted before the action item if present).
    pub fn add_item(&mut self, item: impl Into<String>) {
        let item = item.into();
        // Don't add duplicates
        if !self.items.iter().any(|(i, _)| i == &item) {
            // Insert before action item, or append if no action item
            let insert_pos = self.action_item_index.unwrap_or(self.items.len());
            self.items.insert(insert_pos, (item, true)); // New items are selected by default

            // Update action item index since we inserted before it
            if let Some(ref mut idx) = self.action_item_index {
                *idx += 1;
            }

            // Select the new item
            self.state.select(Some(insert_pos));
        }
    }

    /// Handle a key event.
    pub fn handle_key(&mut self, key: KeyEvent) -> MultiSelectAction {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_previous();
                MultiSelectAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
                MultiSelectAction::None
            }
            KeyCode::Char(' ') => {
                self.toggle_current();
                MultiSelectAction::None
            }
            KeyCode::Enter => MultiSelectAction::Confirm,
            KeyCode::Esc => MultiSelectAction::Cancel,
            _ => MultiSelectAction::None,
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

        let action_idx = self.action_item_index;
        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, (item, selected))| {
                let is_cursor = Some(i) == self.state.selected();
                let is_action = action_idx == Some(i);
                let style = if !focused {
                    // Unfocused: all content muted
                    Style::default().fg(Color::DarkGray)
                } else if is_cursor {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                // Action items don't show a checkbox
                let checkbox = if is_action {
                    "    "
                } else if *selected {
                    "[x] "
                } else {
                    "[ ] "
                };
                let cursor = if is_cursor && focused { "> " } else { "  " };

                ListItem::new(Line::from(vec![
                    Span::styled(cursor, style),
                    Span::styled(checkbox, style),
                    Span::styled(item, style),
                ]))
            })
            .collect();

        let highlight_style = if focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        };

        let list = List::new(items)
            .block(block)
            .highlight_style(highlight_style);

        StatefulWidget::render(list, area, buf, &mut self.state);
    }
}

impl Clone for MultiSelect {
    fn clone(&self) -> Self {
        let mut new_ms = Self {
            items: self.items.clone(),
            state: ListState::default(),
            title: self.title.clone(),
            action_item_index: self.action_item_index,
        };
        if let Some(idx) = self.state.selected() {
            new_ms.state.select(Some(idx));
        }
        new_ms
    }
}
