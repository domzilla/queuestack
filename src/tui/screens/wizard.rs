//! New item wizard screen.
//!
//! Multi-step wizard for creating new items.

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
    widgets::{MultiSelect, SelectList, TextAreaWidget, TextInput},
    AppResult, TuiApp,
};

/// Wizard steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardStep {
    Title,
    Content,
    Attachments,
    Category,
    Labels,
}

impl WizardStep {
    const ALL: [Self; 5] = [
        Self::Title,
        Self::Content,
        Self::Attachments,
        Self::Category,
        Self::Labels,
    ];

    const fn index(self) -> usize {
        match self {
            Self::Title => 0,
            Self::Content => 1,
            Self::Attachments => 2,
            Self::Category => 3,
            Self::Labels => 4,
        }
    }

    fn from_index(index: usize) -> Self {
        Self::ALL[index.min(Self::ALL.len() - 1)]
    }

    fn next(self) -> Self {
        Self::from_index(self.index() + 1)
    }

    fn prev(self) -> Self {
        if self.index() == 0 {
            self
        } else {
            Self::from_index(self.index() - 1)
        }
    }

    const fn is_last(self) -> bool {
        self.index() == Self::ALL.len() - 1
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Title => "Title",
            Self::Content => "Content",
            Self::Attachments => "Attachments",
            Self::Category => "Category",
            Self::Labels => "Labels",
        }
    }
}

/// Output from the wizard.
#[derive(Debug, Clone)]
pub struct WizardOutput {
    pub title: String,
    pub content: String,
    pub attachments: Vec<String>,
    pub category: Option<String>,
    pub labels: Vec<String>,
}

/// New item wizard application.
pub struct NewItemWizard<'a> {
    step: WizardStep,
    title_input: TextInput,
    content_area: TextAreaWidget<'a>,
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
}

impl NewItemWizard<'_> {
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
            step: WizardStep::Title,
            title_input: TextInput::new("Title"),
            content_area: TextAreaWidget::new("Content"),
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
        }
    }

    /// Pre-populate the title field.
    #[must_use]
    pub fn with_title(mut self, title: &str) -> Self {
        self.title_input = self.title_input.with_initial(title);
        self
    }

    /// Pre-populate the content field.
    #[must_use]
    pub fn with_content(mut self, content: &str) -> Self {
        self.content_area = self.content_area.with_initial(content);
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

    fn can_advance(&self) -> bool {
        match self.step {
            WizardStep::Title => !self.title_input.content().trim().is_empty(),
            _ => true,
        }
    }

    fn try_advance(&mut self) {
        if self.can_advance() && !self.step.is_last() {
            self.step = self.step.next();
        }
    }

    fn go_back(&mut self) {
        self.step = self.step.prev();
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
            content: self.content_area.content(),
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
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.try_advance();
                None
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.go_back();
                None
            }
            KeyCode::Enter => {
                // Enter also advances to next step
                self.try_advance();
                None
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

    fn handle_content_key(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Option<AppResult<WizardOutput>> {
        match key.code {
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.try_advance();
                None
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.go_back();
                None
            }
            KeyCode::Esc => Some(AppResult::Cancelled),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(AppResult::Cancelled)
            }
            _ => {
                self.content_area.handle_key(key);
                None
            }
        }
    }

    fn handle_attachments_key(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Option<AppResult<WizardOutput>> {
        match key.code {
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.try_advance();
                None
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.go_back();
                None
            }
            KeyCode::Esc => Some(AppResult::Cancelled),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(AppResult::Cancelled)
            }
            KeyCode::Enter => {
                let content = self.attachment_input.content().trim().to_string();
                if content.is_empty() {
                    // Empty input: Enter acts as Next
                    self.try_advance();
                } else {
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
                KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.try_advance();
                    None
                }
                KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.go_back();
                    None
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
                            // (none) or existing category - select and advance
                            self.category = self.get_category_at_index(idx);
                            self.try_advance();
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
                KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    // Ctrl+N on last step completes the wizard
                    Some(AppResult::Done(self.complete()))
                }
                KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.go_back();
                    None
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

impl TuiApp for NewItemWizard<'_> {
    type Output = WizardOutput;

    fn handle_event(&mut self, event: &TuiEvent) -> Option<AppResult<Self::Output>> {
        match event {
            TuiEvent::Paste(content) => {
                // Handle paste based on current step/context
                match self.step {
                    WizardStep::Title => {
                        self.title_input.insert_text(content);
                    }
                    WizardStep::Content => {
                        self.content_area.insert_text(content);
                    }
                    WizardStep::Attachments => {
                        // Parse as file paths
                        let paths = parse_shell_escaped_paths(content);
                        self.attachments.extend(paths);
                    }
                    WizardStep::Category if self.category_input_mode => {
                        self.category_input.insert_text(content);
                    }
                    WizardStep::Labels if self.label_input_mode => {
                        self.label_input.insert_text(content);
                    }
                    _ => {}
                }
                None
            }
            TuiEvent::Key(key) => match self.step {
                WizardStep::Title => self.handle_title_key(*key),
                WizardStep::Content => self.handle_content_key(*key),
                WizardStep::Attachments => self.handle_attachments_key(*key),
                WizardStep::Category => self.handle_category_key(*key),
                WizardStep::Labels => self.handle_labels_key(*key),
            },
            _ => None,
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Main layout
        let chunks = Layout::vertical([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Content
            Constraint::Length(3), // Help
        ])
        .split(area);

        // Header
        self.render_header(frame, chunks[0]);

        // Content area
        self.render_step(frame, chunks[1]);

        // Help bar
        self.render_help(frame, chunks[2]);
    }
}

impl NewItemWizard<'_> {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let step_num = self.step.index() + 1;
        let total = WizardStep::ALL.len();

        // Step indicators
        let indicators: Vec<Span> = WizardStep::ALL
            .iter()
            .enumerate()
            .flat_map(|(i, step)| {
                let style = match i.cmp(&self.step.index()) {
                    std::cmp::Ordering::Equal => Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                    std::cmp::Ordering::Less => Style::default().fg(Color::Green),
                    std::cmp::Ordering::Greater => Style::default().fg(Color::DarkGray),
                };
                let sep = if i < WizardStep::ALL.len() - 1 {
                    " > "
                } else {
                    ""
                };
                vec![Span::styled(step.name(), style), Span::raw(sep)]
            })
            .collect();

        let mode = if self.is_editing {
            "Edit Item"
        } else {
            "New Item"
        };
        let header = Paragraph::new(Line::from(indicators)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!(" {mode} - Step {step_num} of {total} ")),
        );

        frame.render_widget(header, area);
    }

    fn render_step(&mut self, frame: &mut Frame, area: Rect) {
        match self.step {
            WizardStep::Title => self.render_title_step(frame, area),
            WizardStep::Content => self.render_content_step(frame, area),
            WizardStep::Attachments => self.render_attachments_step(frame, area),
            WizardStep::Category => self.render_category_step(frame, area),
            WizardStep::Labels => self.render_labels_step(frame, area),
        }
    }

    fn render_title_step(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(area);

        self.title_input.render(chunks[0], frame.buffer_mut(), true);

        // Validation message
        if self.title_input.content().trim().is_empty() {
            let msg = Paragraph::new("Title is required").style(Style::default().fg(Color::Yellow));
            frame.render_widget(msg, chunks[1]);
        }
    }

    fn render_content_step(&mut self, frame: &mut Frame, area: Rect) {
        self.content_area.render(area, frame, true);
    }

    fn render_attachments_step(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Min(5),    // List
            Constraint::Length(3), // Input
        ])
        .split(area);

        // Attachments list
        let items: Vec<ListItem> = self
            .attachments
            .iter()
            .enumerate()
            .map(|(i, a)| ListItem::new(format!("{}. {}", i + 1, a)))
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Attachments "),
        );
        frame.render_widget(list, chunks[0]);

        // Input
        self.attachment_input
            .render(chunks[1], frame.buffer_mut(), true);
    }

    fn render_category_step(&self, frame: &mut Frame, area: Rect) {
        if self.category_input_mode {
            // Show input overlay
            let chunks = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(area);

            let mut list = self.category_list.clone();
            list.render(chunks[0], frame.buffer_mut(), false);

            self.category_input
                .render(chunks[1], frame.buffer_mut(), true);
        } else {
            // Current selection display
            let current = self.category.as_deref().unwrap_or("(none)");

            let chunks = Layout::vertical([
                Constraint::Length(3), // Current
                Constraint::Min(5),    // List
            ])
            .split(area);

            let current_display = Paragraph::new(format!("Current: {current}"))
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(current_display, chunks[0]);

            let mut list = self.category_list.clone();
            list.render(chunks[1], frame.buffer_mut(), true);
        }
    }

    fn render_labels_step(&self, frame: &mut Frame, area: Rect) {
        if self.label_input_mode {
            let chunks = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(area);

            let mut list = self.labels_list.clone();
            list.render(chunks[0], frame.buffer_mut(), false);

            self.label_input.render(chunks[1], frame.buffer_mut(), true);
        } else {
            // Show selected labels (action item is auto-excluded by MultiSelect)
            let selected: Vec<&str> = self.labels_list.selected_items();

            let chunks = Layout::vertical([
                Constraint::Length(3), // Current
                Constraint::Min(5),    // List
            ])
            .split(area);

            let current = if selected.is_empty() {
                "(none)".to_string()
            } else {
                selected.join(", ")
            };

            let current_display = Paragraph::new(format!("Selected: {current}"))
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(current_display, chunks[0]);

            let mut list = self.labels_list.clone();
            list.render(chunks[1], frame.buffer_mut(), true);
        }
    }

    fn help_panel_spans(&self) -> Vec<Span<'static>> {
        let key = Style::default().fg(Color::Cyan);
        let text = Style::default();

        match self.step {
            WizardStep::Title => vec![Span::styled("Enter", key), Span::styled(" Confirm", text)],
            WizardStep::Content => vec![],
            WizardStep::Attachments => vec![
                Span::styled("Enter", key),
                Span::styled(" Add/Next  ", text),
                Span::styled("Backspace", key),
                Span::styled(" Remove  ", text),
                Span::styled("Drop", key),
                Span::styled(" Add", text),
            ],
            WizardStep::Category => {
                if self.category_input_mode {
                    vec![
                        Span::styled("Enter", key),
                        Span::styled(" Confirm  ", text),
                        Span::styled("Esc", key),
                        Span::styled(" Cancel", text),
                    ]
                } else {
                    vec![
                        Span::styled("Enter", key),
                        Span::styled(" Select/Next", text),
                    ]
                }
            }
            WizardStep::Labels => {
                if self.label_input_mode {
                    vec![
                        Span::styled("Enter", key),
                        Span::styled(" Add  ", text),
                        Span::styled("Esc", key),
                        Span::styled(" Cancel", text),
                    ]
                } else {
                    vec![Span::styled("Enter", key), Span::styled(" Toggle", text)]
                }
            }
        }
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let is_input_mode = self.category_input_mode || self.label_input_mode;

        // Navigation: Ctrl+P (Back), Ctrl+N (Next/Finish), Esc (Cancel)
        let can_go_back = self.step.index() > 0 && !is_input_mode;
        let can_go_next = !is_input_mode;
        let next_label = if self.step.is_last() {
            "Finish"
        } else {
            "Next"
        };

        let key_on = Style::default().fg(Color::Cyan);
        let key_off = Style::default().fg(Color::DarkGray);
        let txt_on = Style::default();
        let txt_off = Style::default().fg(Color::DarkGray);

        let nav_spans = vec![
            Span::styled("Ctrl+P", if can_go_back { key_on } else { key_off }),
            Span::styled(" Back  ", if can_go_back { txt_on } else { txt_off }),
            Span::styled("Ctrl+N", if can_go_next { key_on } else { key_off }),
            Span::styled(
                format!(" {next_label}  "),
                if can_go_next { txt_on } else { txt_off },
            ),
            Span::styled("Esc", key_on),
            Span::styled(" Cancel", txt_on),
        ];

        let panel_spans = self.help_panel_spans();

        // Calculate widths
        let nav_width: usize = nav_spans.iter().map(|s| s.content.len()).sum();
        let panel_width: usize = panel_spans.iter().map(|s| s.content.len()).sum();
        let inner_width = area.width.saturating_sub(2) as usize; // Account for borders

        // Check if we need to stack vertically
        let needs_stacking = nav_width + panel_width + 2 > inner_width;

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        if needs_stacking && !panel_spans.is_empty() {
            // Vertical layout: nav on top, panel below
            let lines = vec![Line::from(nav_spans), Line::from(panel_spans)];
            let help = Paragraph::new(lines).block(block);
            frame.render_widget(help, area);
        } else {
            // Horizontal layout: nav left, panel right
            let mut spans = nav_spans;
            if !panel_spans.is_empty() {
                let padding = inner_width.saturating_sub(nav_width + panel_width);
                spans.push(Span::raw(" ".repeat(padding)));
                spans.extend(panel_spans);
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
}
