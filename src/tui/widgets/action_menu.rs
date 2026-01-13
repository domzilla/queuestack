//! Centered modal popup menu widget.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Widget},
};
use unicode_width::UnicodeWidthStr;

/// Result of an action menu interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionMenuResult {
    /// User selected an option (returns the action index)
    Selected(usize),
    /// User cancelled (Esc)
    Cancelled,
}

/// A menu item entry.
#[derive(Debug, Clone)]
pub enum MenuItem {
    /// A selectable action with label, description, and optional color.
    Action {
        label: String,
        description: String,
        color: Option<Color>,
        /// Index into the actions array (for mapping back to action kind)
        action_index: usize,
    },
    /// A visual separator line.
    Separator,
}

impl MenuItem {
    /// Create a new action item.
    pub fn action(
        label: impl Into<String>,
        description: impl Into<String>,
        action_index: usize,
    ) -> Self {
        Self::Action {
            label: label.into(),
            description: description.into(),
            color: None,
            action_index,
        }
    }

    /// Create an action item with a custom color.
    pub fn action_colored(
        label: impl Into<String>,
        description: impl Into<String>,
        color: Color,
        action_index: usize,
    ) -> Self {
        Self::Action {
            label: label.into(),
            description: description.into(),
            color: Some(color),
            action_index,
        }
    }

    /// Create a separator.
    pub const fn separator() -> Self {
        Self::Separator
    }

    /// Check if this item is selectable.
    const fn is_selectable(&self) -> bool {
        matches!(self, Self::Action { .. })
    }

    /// Get the display width of this item.
    fn width(&self) -> usize {
        match self {
            Self::Action {
                label, description, ..
            } => {
                if description.is_empty() {
                    label.width()
                } else {
                    label.width() + 3 + description.width() // "label - description"
                }
            }
            Self::Separator => 3, // "───"
        }
    }
}

/// A centered modal popup menu.
///
/// Renders as an overlay with a dimmed background and a bordered
/// menu box in the center of the screen.
pub struct ActionMenu {
    items: Vec<MenuItem>,
    /// Index into items (only selectable items)
    selected: usize,
    /// Indices of selectable items
    selectable_indices: Vec<usize>,
    title: String,
}

impl ActionMenu {
    /// Create a new action menu with the given items.
    pub fn new(title: impl Into<String>, items: Vec<MenuItem>) -> Self {
        let selectable_indices: Vec<usize> = items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.is_selectable())
            .map(|(i, _)| i)
            .collect();

        Self {
            items,
            selected: 0,
            selectable_indices,
            title: title.into(),
        }
    }

    /// Get the action index of the currently selected item.
    pub fn selected_action_index(&self) -> Option<usize> {
        self.selectable_indices.get(self.selected).and_then(|&idx| {
            if let MenuItem::Action { action_index, .. } = &self.items[idx] {
                Some(*action_index)
            } else {
                None
            }
        })
    }

    /// Move selection up (wraps around).
    pub fn select_previous(&mut self) {
        if self.selectable_indices.is_empty() {
            return;
        }
        let len = self.selectable_indices.len();
        self.selected = (self.selected + len - 1) % len;
    }

    /// Move selection down (wraps around).
    pub fn select_next(&mut self) {
        if self.selectable_indices.is_empty() {
            return;
        }
        let len = self.selectable_indices.len();
        self.selected = (self.selected + 1) % len;
    }

    /// Handle a key event, returning a result if the interaction is complete.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<ActionMenuResult> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_previous();
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
                None
            }
            KeyCode::Enter => self.selected_action_index().map(ActionMenuResult::Selected),
            KeyCode::Esc => Some(ActionMenuResult::Cancelled),
            _ => None,
        }
    }

    /// Calculate the popup dimensions based on content.
    #[allow(clippy::cast_possible_truncation)]
    fn popup_size(&self) -> (u16, u16) {
        // Width: max item width + borders + padding + prefix
        let max_item_width = self.items.iter().map(MenuItem::width).max().unwrap_or(10);
        let title_width = self.title.width() + 4; // " title " with padding
        let content_width = max_item_width + 4; // "> " prefix + padding
        let width = content_width.max(title_width) as u16 + 2; // +2 for borders

        // Height: items + borders
        let height = self.items.len() as u16 + 2; // +2 for borders

        (width.max(24), height.max(4)) // Minimum size
    }

    /// Render the popup as a centered modal overlay.
    #[allow(clippy::cast_possible_truncation)]
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Calculate popup dimensions
        let (width, height) = self.popup_size();

        // Center the popup
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
            .border_style(Style::default().fg(Color::Cyan))
            .title(format!(" {} ", self.title));

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        // Get the currently selected item index
        let selected_item_idx = self.selectable_indices.get(self.selected).copied();

        // Render each item
        for (i, item) in self.items.iter().enumerate() {
            let y = inner.y + i as u16;
            if y >= inner.y + inner.height {
                break;
            }

            match item {
                MenuItem::Action {
                    label,
                    description,
                    color,
                    ..
                } => {
                    let is_selected = selected_item_idx == Some(i);

                    // Determine styles
                    let (prefix, label_style, desc_style) = if is_selected {
                        let base = Style::default().add_modifier(Modifier::BOLD);
                        let label_style = color
                            .as_ref()
                            .map_or_else(|| base.fg(Color::Cyan), |c| base.fg(*c));
                        ("> ", label_style, Style::default().fg(Color::DarkGray))
                    } else {
                        let label_style = color
                            .as_ref()
                            .map_or_else(Style::default, |c| Style::default().fg(*c));
                        ("  ", label_style, Style::default().fg(Color::DarkGray))
                    };

                    // Build the line with right-aligned description
                    let prefix_width = prefix.width();
                    let label_width = label.width();
                    let desc_width = description.width();

                    let mut spans = vec![
                        Span::styled(prefix, label_style),
                        Span::styled(label, label_style),
                    ];

                    if !description.is_empty() {
                        // Calculate padding to right-align the description
                        let used = prefix_width + label_width + desc_width;
                        let padding = (inner.width as usize).saturating_sub(used);
                        spans.push(Span::raw(" ".repeat(padding)));
                        spans.push(Span::styled(description, desc_style));
                    }

                    let line = Line::from(spans);
                    buf.set_line(inner.x, y, &line, inner.width);
                }
                MenuItem::Separator => {
                    // Render a dim separator line
                    let sep = "─".repeat(inner.width as usize);
                    let line = Line::from(Span::styled(sep, Style::default().fg(Color::DarkGray)));
                    buf.set_line(inner.x, y, &line, inner.width);
                }
            }
        }
    }
}

/// Calculate a centered rectangle within the given area.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key_event(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn test_items() -> Vec<MenuItem> {
        vec![
            MenuItem::action("A", "desc a", 0),
            MenuItem::action("B", "desc b", 1),
            MenuItem::action("C", "desc c", 2),
        ]
    }

    #[test]
    fn test_navigation_down() {
        let mut menu = ActionMenu::new("Test", test_items());
        assert_eq!(menu.selected_action_index(), Some(0));

        menu.handle_key(key_event(KeyCode::Down));
        assert_eq!(menu.selected_action_index(), Some(1));

        menu.handle_key(key_event(KeyCode::Down));
        assert_eq!(menu.selected_action_index(), Some(2));

        // Wrap around
        menu.handle_key(key_event(KeyCode::Down));
        assert_eq!(menu.selected_action_index(), Some(0));
    }

    #[test]
    fn test_navigation_up() {
        let mut menu = ActionMenu::new("Test", test_items());

        // Wrap around from first item
        menu.handle_key(key_event(KeyCode::Up));
        assert_eq!(menu.selected_action_index(), Some(2));

        menu.handle_key(key_event(KeyCode::Up));
        assert_eq!(menu.selected_action_index(), Some(1));
    }

    #[test]
    fn test_separator_skipped() {
        let items = vec![
            MenuItem::action("A", "", 0),
            MenuItem::separator(),
            MenuItem::action("B", "", 1),
        ];
        let mut menu = ActionMenu::new("Test", items);

        assert_eq!(menu.selected_action_index(), Some(0));
        menu.handle_key(key_event(KeyCode::Down));
        // Should skip separator and go to B
        assert_eq!(menu.selected_action_index(), Some(1));
    }

    #[test]
    fn test_confirm() {
        let mut menu = ActionMenu::new("Test", test_items());
        menu.handle_key(key_event(KeyCode::Down));

        let result = menu.handle_key(key_event(KeyCode::Enter));
        assert_eq!(result, Some(ActionMenuResult::Selected(1)));
    }

    #[test]
    fn test_cancel() {
        let menu_items = vec![MenuItem::action("A", "", 0)];
        let mut menu = ActionMenu::new("Test", menu_items);
        let result = menu.handle_key(key_event(KeyCode::Esc));
        assert_eq!(result, Some(ActionMenuResult::Cancelled));
    }
}
