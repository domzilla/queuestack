//! Filter overlay widget for interactive list filtering.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};
use unicode_width::UnicodeWidthStr;

use super::{MultiSelect, SelectList, TextInput};

/// Filter state that can be applied to a list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilterState {
    /// Search query (matches title, ID, body)
    pub search: String,
    /// Selected labels (AND logic)
    pub labels: Vec<String>,
    /// Selected category (None = all categories)
    pub category: Option<String>,
}

impl FilterState {
    /// Check if the filter is empty (no filtering applied).
    pub fn is_empty(&self) -> bool {
        self.search.is_empty() && self.labels.is_empty() && self.category.is_none()
    }

    /// Clear all filters.
    pub fn clear(&mut self) {
        self.search.clear();
        self.labels.clear();
        self.category = None;
    }
}

/// Result of filter overlay interaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterOverlayResult {
    /// User applied the filter
    Applied(FilterState),
    /// User cancelled (keeps previous filter)
    Cancelled,
}

/// Which section of the filter overlay has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum FilterFocus {
    #[default]
    Search,
    Labels,
    Category,
}

impl FilterFocus {
    const fn next(self) -> Self {
        match self {
            Self::Search => Self::Labels,
            Self::Labels => Self::Category,
            Self::Category => Self::Search,
        }
    }

    const fn prev(self) -> Self {
        match self {
            Self::Search => Self::Category,
            Self::Labels => Self::Search,
            Self::Category => Self::Labels,
        }
    }
}

/// Filter overlay widget combining search, labels, and category filters.
pub struct FilterOverlay {
    search_input: TextInput,
    labels_select: MultiSelect,
    category_select: SelectList,
    focus: FilterFocus,
    /// Available labels for reference
    available_labels: Vec<String>,
    /// Available categories for reference
    available_categories: Vec<String>,
}

impl FilterOverlay {
    /// Create a new filter overlay with available options.
    pub fn new(
        available_labels: Vec<String>,
        available_categories: Vec<String>,
        initial_state: &FilterState,
    ) -> Self {
        // Build search input
        let search_input = TextInput::new("Search").with_initial(&initial_state.search);

        // Build labels multi-select
        let labels_select = if available_labels.is_empty() {
            MultiSelect::new(vec!["(no labels)".to_string()])
        } else {
            MultiSelect::new(available_labels.clone()).with_selected(&initial_state.labels)
        };

        // Build category select with "(all)" option
        let mut category_items = vec!["(all)".to_string()];
        category_items.extend(available_categories.clone());
        let category_select = SelectList::new(category_items);

        // Select the current category if set
        let mut overlay = Self {
            search_input,
            labels_select,
            category_select,
            focus: FilterFocus::Search,
            available_labels,
            available_categories,
        };

        // Set category selection based on initial state
        if let Some(ref cat) = initial_state.category {
            // Find the category index (+1 because "(all)" is at index 0)
            if let Some(idx) = overlay.available_categories.iter().position(|c| c == cat) {
                for _ in 0..=idx {
                    overlay.category_select.select_next();
                }
            }
        }

        overlay
    }

    /// Get the current filter state from the overlay.
    pub fn state(&self) -> FilterState {
        let search = self.search_input.content().to_string();

        let labels: Vec<String> = if self.available_labels.is_empty() {
            vec![]
        } else {
            self.labels_select
                .selected_items()
                .into_iter()
                .map(String::from)
                .collect()
        };

        let category = self.category_select.selected_index().and_then(|idx| {
            if idx == 0 {
                None // "(all)" selected
            } else {
                self.available_categories.get(idx - 1).cloned()
            }
        });

        FilterState {
            search,
            labels,
            category,
        }
    }

    /// Insert text into the search input (for paste support).
    pub fn insert_search_text(&mut self, text: &str) {
        self.search_input.insert_text(text);
    }

    /// Handle a key event.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<FilterOverlayResult> {
        match key.code {
            KeyCode::Tab => {
                self.focus = self.focus.next();
                None
            }
            KeyCode::BackTab => {
                self.focus = self.focus.prev();
                None
            }
            KeyCode::Enter => Some(FilterOverlayResult::Applied(self.state())),
            KeyCode::Esc => Some(FilterOverlayResult::Cancelled),
            _ => {
                // Delegate to focused widget
                match self.focus {
                    FilterFocus::Search => {
                        self.search_input.handle_key(key);
                    }
                    FilterFocus::Labels => {
                        if !self.available_labels.is_empty() {
                            self.labels_select.handle_key(key);
                        }
                    }
                    FilterFocus::Category => {
                        // Only allow navigation, not confirm/cancel
                        match key.code {
                            KeyCode::Up | KeyCode::Char('k') => {
                                self.category_select.select_previous();
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                self.category_select.select_next();
                            }
                            _ => {}
                        }
                    }
                }
                None
            }
        }
    }

    /// Calculate the overlay dimensions.
    #[allow(clippy::cast_possible_truncation)]
    fn overlay_size(&self) -> (u16, u16) {
        // Width: enough for labels and categories side by side
        let max_label_width = self
            .available_labels
            .iter()
            .map(|l| l.width())
            .max()
            .unwrap_or(10);
        let max_cat_width = self
            .available_categories
            .iter()
            .map(|c| c.width())
            .max()
            .unwrap_or(10);

        // Width: labels + categories + padding (prefix + checkbox/radio + borders)
        // Each side needs: 2 ("> ") + 4 ("[ ] " or "(•) ") + label + 2 (borders) + 2 (padding)
        let content_width = (max_label_width + 12) + (max_cat_width + 12) + 4;
        let width = content_width.clamp(50, 100) as u16;

        // Height: search (3) + labels/categories (max 10 rows) + help (2) + borders
        let list_height = self
            .available_labels
            .len()
            .max(self.available_categories.len() + 1)
            .min(8);
        let height = (3 + list_height + 3 + 2) as u16;

        (width, height.max(12))
    }

    /// Render the overlay.
    #[allow(clippy::cast_possible_truncation)]
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let (width, height) = self.overlay_size();

        // Center the overlay
        let overlay_area = centered_rect(width, height, area);

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

        // Clear the overlay area
        Clear.render(overlay_area, buf);

        // Render the main block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Filter ");

        let inner = block.inner(overlay_area);
        block.render(overlay_area, buf);

        // Layout: search, lists side by side, help
        let chunks = Layout::vertical([
            Constraint::Length(3), // Search input
            Constraint::Min(4),    // Labels and categories
            Constraint::Length(1), // Help text
        ])
        .split(inner);

        // Render search input
        self.search_input
            .render(chunks[0], buf, self.focus == FilterFocus::Search);

        // Split middle area for labels and categories
        let list_chunks =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);

        // Render labels (left)
        self.render_labels_section(list_chunks[0], buf);

        // Render categories (right)
        self.render_category_section(list_chunks[1], buf);

        // Render help text
        self.render_help(chunks[2], buf);
    }

    #[allow(clippy::cast_possible_truncation)]
    fn render_labels_section(&self, area: Rect, buf: &mut Buffer) {
        let focused = self.focus == FilterFocus::Labels;
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Labels ");

        let inner = block.inner(area);
        block.render(area, buf);

        if self.available_labels.is_empty() {
            let text = Paragraph::new("(no labels)").style(Style::default().fg(Color::DarkGray));
            text.render(inner, buf);
        } else {
            // Render each label with checkbox
            for (i, label) in self.available_labels.iter().enumerate() {
                if i >= inner.height as usize {
                    break;
                }
                let y = inner.y + i as u16;
                let is_selected = self.labels_select.selected_index() == Some(i);
                let is_checked = self
                    .labels_select
                    .selected_items()
                    .contains(&label.as_str());

                let checkbox = if is_checked { "[x] " } else { "[ ] " };
                let prefix = if is_selected && focused { "> " } else { "  " };

                let style = if is_selected && focused {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let line = Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(checkbox, style),
                    Span::styled(label, style),
                ]);
                buf.set_line(inner.x, y, &line, inner.width);
            }
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn render_category_section(&self, area: Rect, buf: &mut Buffer) {
        let focused = self.focus == FilterFocus::Category;
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Category ");

        let inner = block.inner(area);
        block.render(area, buf);

        // Render each category with radio button style
        let items: Vec<&str> = std::iter::once("(all)")
            .chain(self.available_categories.iter().map(String::as_str))
            .collect();

        let selected_idx = self.category_select.selected_index();

        for (i, item) in items.iter().enumerate() {
            if i >= inner.height as usize {
                break;
            }
            let y = inner.y + i as u16;
            let is_selected = selected_idx == Some(i);

            let radio = if is_selected { "(•) " } else { "( ) " };
            let prefix = if is_selected && focused { "> " } else { "  " };

            let style = if is_selected && focused {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(radio, style),
                Span::styled(*item, style),
            ]);
            buf.set_line(inner.x, y, &line, inner.width);
        }
    }

    #[allow(clippy::unused_self, clippy::cast_possible_truncation)]
    fn render_help(&self, area: Rect, buf: &mut Buffer) {
        // Left side: Enter Apply, Esc Cancel
        let left = Line::from(vec![
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::raw(" Apply  "),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(" Cancel"),
        ]);
        buf.set_line(area.x, area.y, &left, area.width);

        // Right side: Tab Switch, Space Toggle
        let right_text = "Tab Switch  Space Toggle";
        let right = Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Cyan)),
            Span::raw(" Switch  "),
            Span::styled("Space", Style::default().fg(Color::Cyan)),
            Span::raw(" Toggle"),
        ]);
        let right_x = area.x + area.width.saturating_sub(right_text.len() as u16);
        buf.set_line(right_x, area.y, &right, area.width);
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

    #[test]
    fn test_filter_state_is_empty() {
        let state = FilterState::default();
        assert!(state.is_empty());

        let state = FilterState {
            search: "test".to_string(),
            ..Default::default()
        };
        assert!(!state.is_empty());
    }

    #[test]
    fn test_filter_state_clear() {
        let mut state = FilterState {
            search: "test".to_string(),
            labels: vec!["bug".to_string()],
            category: Some("features".to_string()),
        };
        state.clear();
        assert!(state.is_empty());
    }

    #[test]
    fn test_focus_navigation() {
        assert_eq!(FilterFocus::Search.next(), FilterFocus::Labels);
        assert_eq!(FilterFocus::Labels.next(), FilterFocus::Category);
        assert_eq!(FilterFocus::Category.next(), FilterFocus::Search);

        assert_eq!(FilterFocus::Search.prev(), FilterFocus::Category);
        assert_eq!(FilterFocus::Labels.prev(), FilterFocus::Search);
        assert_eq!(FilterFocus::Category.prev(), FilterFocus::Labels);
    }
}
