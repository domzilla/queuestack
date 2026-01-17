//! New item wizard screen.
//!
//! Two-panel wizard for creating new items with Tab navigation.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::{
    event::TuiEvent,
    widgets::{MultiSelect, SelectList, TextInput},
    AppResult, TuiApp,
};

/// Wizard panels for breadcrumb display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardPanel {
    Meta,
    Attachments,
}

impl WizardPanel {
    const fn name(self) -> &'static str {
        match self {
            Self::Meta => "Meta",
            Self::Attachments => "Attachments",
        }
    }
}

/// Currently focused widget for Tab navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedWidget {
    Title,
    Category,
    Labels,
    Attachments,
}

impl FocusedWidget {
    /// Cycle to next widget (Tab).
    const fn next(self) -> Self {
        match self {
            Self::Title => Self::Category,
            Self::Category => Self::Labels,
            Self::Labels => Self::Attachments,
            Self::Attachments => Self::Title,
        }
    }

    /// Cycle to previous widget (Shift+Tab).
    const fn prev(self) -> Self {
        match self {
            Self::Title => Self::Attachments,
            Self::Category => Self::Title,
            Self::Labels => Self::Category,
            Self::Attachments => Self::Labels,
        }
    }

    /// Get the panel this widget belongs to.
    const fn panel(self) -> WizardPanel {
        match self {
            Self::Title | Self::Category | Self::Labels => WizardPanel::Meta,
            Self::Attachments => WizardPanel::Attachments,
        }
    }
}

/// Output from the wizard.
#[derive(Debug, Clone)]
pub struct WizardOutput {
    pub title: String,
    pub attachments: Vec<String>,
    pub category: Option<String>,
    pub labels: Vec<String>,
}

/// New item wizard application.
pub struct NewItemWizard {
    focused: FocusedWidget,
    title_input: TextInput,
    attachments: Vec<String>,
    attachment_input: TextInput,
    category: Option<String>,
    existing_categories: Vec<String>,
    category_list: SelectList,
    category_input: TextInput,
    category_input_mode: bool,
    labels_list: MultiSelect,
    label_input: TextInput,
    label_input_mode: bool,
    /// Whether this wizard is editing an existing item (changes header).
    is_editing: bool,
    /// The ID of the item being edited (shown in header when editing).
    item_id: Option<String>,
}

impl NewItemWizard {
    /// Create a new wizard with existing categories and labels.
    pub fn new(existing_categories: Vec<String>, existing_labels: Vec<String>) -> Self {
        // Build category list: (none), existing categories, Create new...
        let mut category_items = vec!["(none)".to_string()];
        category_items.extend(existing_categories.iter().cloned());
        category_items.push("Create new...".to_string());

        // Build labels list: existing labels, Create new... (at end, as action item)
        let mut label_items = existing_labels;
        label_items.push("Create new...".to_string());

        Self {
            focused: FocusedWidget::Title,
            title_input: TextInput::new("Title"),
            attachments: Vec::new(),
            attachment_input: TextInput::new("Add attachments (Space or Newline separated)"),
            category: None,
            existing_categories,
            category_list: SelectList::new(category_items).with_title("Category"),
            category_input: TextInput::new("New category name"),
            category_input_mode: false,
            labels_list: MultiSelect::new(label_items)
                .with_title("Labels")
                .with_action_item_last(),
            label_input: TextInput::new("New label"),
            label_input_mode: false,
            is_editing: false,
            item_id: None,
        }
    }

    /// Pre-populate the title field.
    #[must_use]
    pub fn with_title(mut self, title: &str) -> Self {
        self.title_input = self.title_input.with_initial(title);
        self
    }

    /// Pre-populate the attachments list.
    #[must_use]
    pub fn with_attachments(mut self, attachments: Vec<String>) -> Self {
        self.attachments = attachments;
        self
    }

    /// Pre-populate the category and select it in the list.
    #[must_use]
    #[allow(clippy::needless_pass_by_value, clippy::assigning_clones)]
    pub fn with_category(mut self, category: Option<String>) -> Self {
        self.category = category.clone();
        // Select the matching item in the category list
        // Index 0 = "(none)", 1..n = existing categories, last = "Create new..."
        let select_idx = match &category {
            None => 0, // "(none)"
            Some(cat) => {
                // Find the category in existing_categories
                self.existing_categories
                    .iter()
                    .position(|c| c == cat)
                    .map_or(0, |pos| pos + 1) // +1 because "(none)" is at index 0
            }
        };
        // Rebuild category list with selection
        let mut category_items = vec!["(none)".to_string()];
        category_items.extend(self.existing_categories.iter().cloned());
        category_items.push("Create new...".to_string());
        self.category_list = SelectList::new(category_items).with_title("Category");
        // Select the appropriate index
        for _ in 0..select_idx {
            self.category_list.select_next();
        }
        self
    }

    /// Pre-select labels.
    #[must_use]
    pub fn with_labels(mut self, labels: &[String]) -> Self {
        self.labels_list = self.labels_list.with_selected(labels);
        self
    }

    /// Mark this wizard as editing mode (changes header text).
    #[must_use]
    pub const fn for_editing(mut self) -> Self {
        self.is_editing = true;
        self
    }

    /// Set the item ID to display in the header when editing.
    #[must_use]
    pub fn with_item_id(mut self, id: impl Into<String>) -> Self {
        self.item_id = Some(id.into());
        self
    }

    /// Check if saving is allowed (title must not be empty).
    fn can_save(&self) -> bool {
        !self.title_input.content().trim().is_empty()
    }

    /// Check if we're in any input mode (category or label creation).
    const fn is_input_mode(&self) -> bool {
        self.category_input_mode || self.label_input_mode
    }

    fn complete(&self) -> WizardOutput {
        // Collect selected labels (action item is automatically excluded by MultiSelect)
        let labels: Vec<String> = self
            .labels_list
            .selected_items()
            .into_iter()
            .map(String::from)
            .collect();

        WizardOutput {
            title: self.title_input.content().trim().to_string(),
            attachments: self.attachments.clone(),
            category: self.category.clone(),
            labels,
        }
    }

    fn handle_title_key(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Option<AppResult<WizardOutput>> {
        match key.code {
            KeyCode::Tab => {
                self.focused = self.focused.next();
                None
            }
            KeyCode::BackTab => {
                self.focused = self.focused.prev();
                None
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.can_save() {
                    Some(AppResult::Done(self.complete()))
                } else {
                    None
                }
            }
            KeyCode::Esc => Some(AppResult::Cancelled),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(AppResult::Cancelled)
            }
            _ => {
                self.title_input.handle_key(key);
                None
            }
        }
    }

    fn handle_attachments_key(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Option<AppResult<WizardOutput>> {
        match key.code {
            KeyCode::Tab => {
                self.focused = self.focused.next();
                None
            }
            KeyCode::BackTab => {
                self.focused = self.focused.prev();
                None
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.can_save() {
                    Some(AppResult::Done(self.complete()))
                } else {
                    None
                }
            }
            KeyCode::Esc => Some(AppResult::Cancelled),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(AppResult::Cancelled)
            }
            KeyCode::Enter => {
                let content = self.attachment_input.content().trim().to_string();
                if !content.is_empty() {
                    let paths = parse_shell_escaped_paths(&content);
                    self.attachments.extend(paths);
                    self.attachment_input =
                        TextInput::new("Add attachments (Space or Newline separated)");
                }
                None
            }
            KeyCode::Backspace if self.attachment_input.is_empty() => {
                self.attachments.pop();
                None
            }
            _ => {
                self.attachment_input.handle_key(key);
                None
            }
        }
    }

    fn handle_category_key(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Option<AppResult<WizardOutput>> {
        if self.category_input_mode {
            match key.code {
                KeyCode::Enter => {
                    let content = self.category_input.content().trim().to_string();
                    if !content.is_empty() {
                        self.category = Some(content);
                    }
                    self.category_input_mode = false;
                    self.category_input = TextInput::new("New category name");
                    None
                }
                KeyCode::Esc => {
                    self.category_input_mode = false;
                    self.category_input = TextInput::new("New category name");
                    None
                }
                _ => {
                    self.category_input.handle_key(key);
                    None
                }
            }
        } else {
            match key.code {
                KeyCode::Tab => {
                    self.focused = self.focused.next();
                    None
                }
                KeyCode::BackTab => {
                    self.focused = self.focused.prev();
                    None
                }
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if self.can_save() {
                        Some(AppResult::Done(self.complete()))
                    } else {
                        None
                    }
                }
                KeyCode::Esc => Some(AppResult::Cancelled),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(AppResult::Cancelled)
                }
                KeyCode::Enter => {
                    if let Some(idx) = self.category_list.selected_index() {
                        if self.is_create_new_category(idx) {
                            // Create new...
                            self.category_input_mode = true;
                        } else {
                            // (none) or existing category - just select it
                            self.category = self.get_category_at_index(idx);
                        }
                    }
                    None
                }
                _ => {
                    self.category_list.handle_key(key);
                    None
                }
            }
        }
    }

    fn get_category_at_index(&self, idx: usize) -> Option<String> {
        // Index 0 = "(none)", 1..n-1 = actual categories, last = "+ Create new..."
        if idx == 0 {
            None // "(none)"
        } else {
            self.existing_categories.get(idx - 1).cloned()
        }
    }

    fn is_create_new_category(&self, idx: usize) -> bool {
        // Last index is "+ Create new..."
        idx == self.existing_categories.len() + 1
    }

    fn is_add_new_label(&self) -> bool {
        // Last index is "+ Add new..." (action item)
        self.labels_list.selected_index() == Some(self.labels_list.len() - 1)
    }

    fn handle_labels_key(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Option<AppResult<WizardOutput>> {
        if self.label_input_mode {
            match key.code {
                KeyCode::Enter => {
                    let content = self.label_input.content().trim().to_string();
                    if !content.is_empty() {
                        // add_item adds as pre-selected and handles duplicates
                        self.labels_list.add_item(&content);
                    }
                    self.label_input_mode = false;
                    self.label_input = TextInput::new("New label");
                    None
                }
                KeyCode::Esc => {
                    self.label_input_mode = false;
                    self.label_input = TextInput::new("New label");
                    None
                }
                _ => {
                    self.label_input.handle_key(key);
                    None
                }
            }
        } else {
            match key.code {
                KeyCode::Tab => {
                    self.focused = self.focused.next();
                    None
                }
                KeyCode::BackTab => {
                    self.focused = self.focused.prev();
                    None
                }
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if self.can_save() {
                        Some(AppResult::Done(self.complete()))
                    } else {
                        None
                    }
                }
                KeyCode::Esc => Some(AppResult::Cancelled),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(AppResult::Cancelled)
                }
                KeyCode::Enter => {
                    // Enter on "+ Add new..." opens input mode, otherwise toggles selection
                    if self.is_add_new_label() {
                        self.label_input_mode = true;
                    } else {
                        self.labels_list.toggle_current();
                    }
                    None
                }
                _ => {
                    self.labels_list.handle_key(key);
                    None
                }
            }
        }
    }
}

impl TuiApp for NewItemWizard {
    type Output = WizardOutput;

    fn handle_event(&mut self, event: &TuiEvent) -> Option<AppResult<Self::Output>> {
        match event {
            TuiEvent::Paste(content) => {
                // Handle paste based on current focus/context
                match self.focused {
                    FocusedWidget::Title => {
                        self.title_input.insert_text(content);
                    }
                    FocusedWidget::Attachments => {
                        // Parse as file paths
                        let paths = parse_shell_escaped_paths(content);
                        self.attachments.extend(paths);
                    }
                    FocusedWidget::Category if self.category_input_mode => {
                        self.category_input.insert_text(content);
                    }
                    FocusedWidget::Labels if self.label_input_mode => {
                        self.label_input.insert_text(content);
                    }
                    _ => {}
                }
                None
            }
            TuiEvent::Key(key) => match self.focused {
                FocusedWidget::Title => self.handle_title_key(*key),
                FocusedWidget::Attachments => self.handle_attachments_key(*key),
                FocusedWidget::Category => self.handle_category_key(*key),
                FocusedWidget::Labels => self.handle_labels_key(*key),
            },
            _ => None,
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Main layout: Header, Content, Help
        let chunks = Layout::vertical([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Content (one panel at a time)
            Constraint::Length(3), // Help
        ])
        .split(area);

        // Header
        self.render_header(frame, chunks[0]);

        // Content area - show panel based on current focus
        match self.focused.panel() {
            WizardPanel::Meta => self.render_meta_panel(frame, chunks[1]),
            WizardPanel::Attachments => self.render_attachments_panel(frame, chunks[1]),
        }

        // Help bar
        self.render_help(frame, chunks[2]);
    }
}

impl NewItemWizard {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let current_panel = self.focused.panel();

        // Panel indicators: Meta > Attachments
        let panels = [WizardPanel::Meta, WizardPanel::Attachments];
        let indicators: Vec<Span> = panels
            .iter()
            .enumerate()
            .flat_map(|(i, panel)| {
                let style = if *panel == current_panel {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let sep = if i < panels.len() - 1 { " > " } else { "" };
                vec![Span::styled(panel.name(), style), Span::raw(sep)]
            })
            .collect();

        let mode = if self.is_editing {
            self.item_id
                .as_ref()
                .map_or_else(|| "Edit Item".to_string(), |id| format!("Edit {id}"))
        } else {
            "New Item".to_string()
        };
        let header = Paragraph::new(Line::from(indicators)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!(" {mode} ")),
        );

        frame.render_widget(header, area);
    }

    fn render_meta_panel(&self, frame: &mut Frame, area: Rect) {
        // Meta panel layout:
        // - Title input (full width, 3 rows)
        // - Category (left 50%) | Labels (right 50%)
        let chunks = Layout::vertical([
            Constraint::Length(3), // Title input
            Constraint::Min(4),    // Category/Labels split
        ])
        .split(area);

        // Title input
        let title_focused = self.focused == FocusedWidget::Title;
        self.render_title_widget(frame, chunks[0], title_focused);

        // Category/Labels horizontal split (50/50)
        let split_chunks =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);

        // Category (left)
        let category_focused = self.focused == FocusedWidget::Category;
        self.render_category_widget(frame, split_chunks[0], category_focused);

        // Labels (right)
        let labels_focused = self.focused == FocusedWidget::Labels;
        self.render_labels_widget(frame, split_chunks[1], labels_focused);
    }

    fn render_title_widget(&self, frame: &mut Frame, area: Rect, focused: bool) {
        // Add warning if title is empty
        let input = if self.title_input.content().trim().is_empty() {
            self.title_input.clone().with_warning("required")
        } else {
            self.title_input.clone()
        };
        input.render(area, frame.buffer_mut(), focused);
    }

    fn render_category_widget(&self, frame: &mut Frame, area: Rect, focused: bool) {
        // Build title with current selection
        let current = self.category.as_deref().unwrap_or("(none)");
        let title = format!("Category: {current}");

        if self.category_input_mode {
            // Show input overlay for creating new category
            let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).split(area);

            let mut list = self.category_list.clone().with_title(&title);
            list.render(chunks[0], frame.buffer_mut(), false);

            self.category_input
                .render(chunks[1], frame.buffer_mut(), true);
        } else {
            // Category list (SelectList renders its own border)
            let mut list = self.category_list.clone().with_title(&title);
            list.render(area, frame.buffer_mut(), focused);
        }
    }

    fn render_labels_widget(&self, frame: &mut Frame, area: Rect, focused: bool) {
        // Build title with selected labels
        let selected: Vec<&str> = self.labels_list.selected_items();
        let selection = if selected.is_empty() {
            "(none)".to_string()
        } else {
            selected.join(", ")
        };
        let title = format!("Labels: {selection}");

        if self.label_input_mode {
            // Show input overlay for creating new label
            let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).split(area);

            let mut list = self.labels_list.clone().with_title(&title);
            list.render(chunks[0], frame.buffer_mut(), false);

            self.label_input.render(chunks[1], frame.buffer_mut(), true);
        } else {
            // Labels list (MultiSelect renders its own border)
            let mut list = self.labels_list.clone().with_title(&title);
            list.render(area, frame.buffer_mut(), focused);
        }
    }

    fn render_attachments_panel(&self, frame: &mut Frame, area: Rect) {
        let focused = self.focused == FocusedWidget::Attachments;
        let border_color = if focused {
            Color::Cyan
        } else {
            Color::DarkGray
        };
        let text_style = if focused {
            Style::default()
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(" Attachments ");

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split inner area: list + input
        let chunks = Layout::vertical([
            Constraint::Min(2),    // List
            Constraint::Length(3), // Input
        ])
        .split(inner);

        // Attachments list
        let items: Vec<ListItem> = self
            .attachments
            .iter()
            .enumerate()
            .map(|(i, a)| ListItem::new(format!("{}. {}", i + 1, a)).style(text_style))
            .collect();

        let list = List::new(items);
        frame.render_widget(list, chunks[0]);

        // Input
        self.attachment_input
            .render(chunks[1], frame.buffer_mut(), focused);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let key_on = Style::default().fg(Color::Cyan);
        let key_off = Style::default().fg(Color::DarkGray);
        let txt_on = Style::default();
        let txt_off = Style::default().fg(Color::DarkGray);

        let can_save = self.can_save();
        let is_input = self.is_input_mode();

        // Build context-specific hints
        let context_spans: Vec<Span> = if is_input {
            vec![
                Span::styled("Enter", key_on),
                Span::styled(" Confirm  ", txt_on),
                Span::styled("Esc", key_on),
                Span::styled(" Cancel", txt_on),
            ]
        } else {
            match self.focused {
                FocusedWidget::Title => vec![],
                FocusedWidget::Category => vec![
                    Span::styled("Enter", key_on),
                    Span::styled(" Select", txt_on),
                ],
                FocusedWidget::Labels => vec![
                    Span::styled("Enter", key_on),
                    Span::styled(" Toggle", txt_on),
                ],
                FocusedWidget::Attachments => vec![
                    Span::styled("Enter", key_on),
                    Span::styled(" Add  ", txt_on),
                    Span::styled("Backspace", key_on),
                    Span::styled(" Remove", txt_on),
                ],
            }
        };

        // Navigation hints (disabled during input mode)
        let nav_spans: Vec<Span> = vec![
            Span::styled("Tab", if is_input { key_off } else { key_on }),
            Span::styled(" Next  ", if is_input { txt_off } else { txt_on }),
            Span::styled(
                "Ctrl+S",
                if can_save && !is_input {
                    key_on
                } else {
                    key_off
                },
            ),
            Span::styled(
                " Save  ",
                if can_save && !is_input {
                    txt_on
                } else {
                    txt_off
                },
            ),
            Span::styled("Esc", key_on),
            Span::styled(" Cancel", txt_on),
        ];

        // Calculate widths
        let nav_width: usize = nav_spans.iter().map(|s| s.content.len()).sum();
        let context_width: usize = context_spans.iter().map(|s| s.content.len()).sum();
        let inner_width = area.width.saturating_sub(2) as usize;

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        // Check if we need to stack vertically
        let needs_stacking = nav_width + context_width + 2 > inner_width;

        if needs_stacking && !context_spans.is_empty() {
            let lines = vec![Line::from(nav_spans), Line::from(context_spans)];
            let help = Paragraph::new(lines).block(block);
            frame.render_widget(help, area);
        } else {
            let mut spans = nav_spans;
            if !context_spans.is_empty() {
                let padding = inner_width.saturating_sub(nav_width + context_width);
                spans.push(Span::raw(" ".repeat(padding)));
                spans.extend(context_spans);
            }
            let help = Paragraph::new(Line::from(spans)).block(block);
            frame.render_widget(help, area);
        }
    }
}

/// Parse a string containing shell-escaped paths separated by unescaped spaces or newlines.
///
/// Paths can contain escaped spaces (e.g., `/path/to\ file.png`) and multiple
/// paths are separated by unescaped spaces or newlines (\n, \r, \r\n).
/// Returns individual paths with escape sequences removed.
fn parse_shell_escaped_paths(input: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                // Check if this is an escape sequence
                if let Some(&next) = chars.peek() {
                    // Escaped character - consume it and add the literal character
                    chars.next();
                    current.push(next);
                } else {
                    // Trailing backslash - keep it
                    current.push(ch);
                }
            }
            ' ' | '\n' | '\r' => {
                // Unescaped space or newline - this is a separator
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    paths.push(trimmed);
                }
                current.clear();
            }
            _ => {
                current.push(ch);
            }
        }
    }

    // Don't forget the last path
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        paths.push(trimmed);
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shell_escaped_paths_single() {
        let input = "/path/to/file.png";
        let result = parse_shell_escaped_paths(input);
        assert_eq!(result, vec!["/path/to/file.png"]);
    }

    #[test]
    fn test_parse_shell_escaped_paths_with_escaped_spaces() {
        let input = r"/path/to\ file.png";
        let result = parse_shell_escaped_paths(input);
        assert_eq!(result, vec!["/path/to file.png"]);
    }

    #[test]
    fn test_parse_shell_escaped_paths_multiple() {
        let input = "/path/one.png /path/two.png";
        let result = parse_shell_escaped_paths(input);
        assert_eq!(result, vec!["/path/one.png", "/path/two.png"]);
    }

    #[test]
    fn test_parse_shell_escaped_paths_multiple_with_escaped_spaces() {
        let input = r"/Users/dom/Desktop/Screenshot\ 2026-01-11\ at\ 11.58.43.png /Users/dom/Desktop/Screenshot\ 2026-01-11\ at\ 11.58.52.png";
        let result = parse_shell_escaped_paths(input);
        assert_eq!(
            result,
            vec![
                "/Users/dom/Desktop/Screenshot 2026-01-11 at 11.58.43.png",
                "/Users/dom/Desktop/Screenshot 2026-01-11 at 11.58.52.png"
            ]
        );
    }

    #[test]
    fn test_parse_shell_escaped_paths_empty() {
        let input = "";
        let result = parse_shell_escaped_paths(input);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_shell_escaped_paths_only_spaces() {
        let input = "   ";
        let result = parse_shell_escaped_paths(input);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_shell_escaped_paths_newline_separated() {
        let input = "/path/one.png\n/path/two.png";
        let result = parse_shell_escaped_paths(input);
        assert_eq!(result, vec!["/path/one.png", "/path/two.png"]);
    }

    #[test]
    fn test_parse_shell_escaped_paths_crlf_separated() {
        let input = "/path/one.png\r\n/path/two.png";
        let result = parse_shell_escaped_paths(input);
        assert_eq!(result, vec!["/path/one.png", "/path/two.png"]);
    }

    #[test]
    fn test_parse_shell_escaped_paths_mixed_separators() {
        let input = "/path/one.png /path/two.png\n/path/three.png";
        let result = parse_shell_escaped_paths(input);
        assert_eq!(
            result,
            vec!["/path/one.png", "/path/two.png", "/path/three.png"]
        );
    }

    #[test]
    fn test_focused_widget_navigation() {
        assert_eq!(FocusedWidget::Title.next(), FocusedWidget::Category);
        assert_eq!(FocusedWidget::Category.next(), FocusedWidget::Labels);
        assert_eq!(FocusedWidget::Labels.next(), FocusedWidget::Attachments);
        assert_eq!(FocusedWidget::Attachments.next(), FocusedWidget::Title);

        assert_eq!(FocusedWidget::Title.prev(), FocusedWidget::Attachments);
        assert_eq!(FocusedWidget::Category.prev(), FocusedWidget::Title);
        assert_eq!(FocusedWidget::Labels.prev(), FocusedWidget::Category);
        assert_eq!(FocusedWidget::Attachments.prev(), FocusedWidget::Labels);
    }

    #[test]
    fn test_focused_widget_panel() {
        assert_eq!(FocusedWidget::Title.panel(), WizardPanel::Meta);
        assert_eq!(FocusedWidget::Category.panel(), WizardPanel::Meta);
        assert_eq!(FocusedWidget::Labels.panel(), WizardPanel::Meta);
        assert_eq!(FocusedWidget::Attachments.panel(), WizardPanel::Attachments);
    }
}
