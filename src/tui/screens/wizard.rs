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
    labels: Vec<String>,
    labels_list: MultiSelect,
    label_input: TextInput,
    label_input_mode: bool,
}

impl NewItemWizard<'_> {
    /// Create a new wizard with existing categories and labels.
    pub fn new(existing_categories: Vec<String>, existing_labels: Vec<String>) -> Self {
        // Build category list with "Create new..." option
        let mut category_items = vec!["(none)".to_string(), "+ Create new...".to_string()];
        category_items.extend(existing_categories.iter().cloned());

        // Build labels list with "Add new..." option
        let mut label_items = vec!["+ Add new...".to_string()];
        label_items.extend(existing_labels);

        Self {
            step: WizardStep::Title,
            title_input: TextInput::new("Title"),
            content_area: TextAreaWidget::new("Content"),
            attachments: Vec::new(),
            attachment_input: TextInput::new("Add attachment (path or URL)"),
            category: None,
            existing_categories,
            category_list: SelectList::new(category_items).with_title("Category"),
            category_input: TextInput::new("New category name"),
            category_input_mode: false,
            labels: Vec::new(),
            labels_list: MultiSelect::new(label_items).with_title("Labels (Space to toggle)"),
            label_input: TextInput::new("New label"),
            label_input_mode: false,
        }
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
        // Collect selected labels (skip "Add new..." at index 0)
        let selected = self.labels_list.selected_items();
        let mut labels: Vec<String> = selected
            .into_iter()
            .filter(|s| *s != "+ Add new...")
            .map(String::from)
            .collect();
        labels.extend(self.labels.clone());

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
            KeyCode::Tab => {
                self.try_advance();
                None
            }
            KeyCode::BackTab => {
                self.go_back();
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
            KeyCode::Tab => {
                self.try_advance();
                None
            }
            KeyCode::BackTab => {
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
            KeyCode::Tab => {
                self.try_advance();
                None
            }
            KeyCode::BackTab => {
                self.go_back();
                None
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
                    self.attachment_input = TextInput::new("Add attachment (path or URL)");
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
                    self.try_advance();
                    None
                }
                KeyCode::BackTab => {
                    self.go_back();
                    None
                }
                KeyCode::Esc => Some(AppResult::Cancelled),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(AppResult::Cancelled)
                }
                KeyCode::Enter => {
                    if let Some(idx) = self.category_list.selected_index() {
                        if idx == 0 {
                            // (none)
                            self.category = None;
                        } else if idx == 1 {
                            // Create new...
                            self.category_input_mode = true;
                        } else {
                            // Existing category (offset by 2 for the special items)
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
        // Index 0 = "(none)", 1 = "+ Create new...", 2+ = actual categories
        match idx {
            0 | 1 => None, // "(none)" or "+ Create new..."
            _ => self.existing_categories.get(idx - 2).cloned(),
        }
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
                        self.labels.push(content.clone());
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
                KeyCode::Tab | KeyCode::Enter => {
                    // Check if "Add new..." is selected and Enter pressed
                    if self.labels_list.selected_items().is_empty()
                        && self.labels_list.selected_index() == Some(0)
                        && key.code == KeyCode::Enter
                    {
                        self.label_input_mode = true;
                        return None;
                    }
                    // On last step, Enter completes
                    if key.code == KeyCode::Enter && self.step.is_last() {
                        return Some(AppResult::Done(self.complete()));
                    }
                    // Tab on labels step doesn't advance (it's the last step)
                    None
                }
                KeyCode::BackTab => {
                    self.go_back();
                    None
                }
                KeyCode::Esc => Some(AppResult::Cancelled),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(AppResult::Cancelled)
                }
                KeyCode::Char(' ') => {
                    // Check if on "Add new..."
                    if self.labels_list.selected_index() == Some(0) {
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
        let TuiEvent::Key(key) = event else {
            return None;
        };

        match self.step {
            WizardStep::Title => self.handle_title_key(*key),
            WizardStep::Content => self.handle_content_key(*key),
            WizardStep::Attachments => self.handle_attachments_key(*key),
            WizardStep::Category => self.handle_category_key(*key),
            WizardStep::Labels => self.handle_labels_key(*key),
        }
    }

    fn render(&self, frame: &mut Frame) {
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

        let header = Paragraph::new(Line::from(indicators)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!(" New Item - Step {step_num} of {total} ")),
        );

        frame.render_widget(header, area);
    }

    fn render_step(&self, frame: &mut Frame, area: Rect) {
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

    fn render_content_step(&self, frame: &mut Frame, area: Rect) {
        let mut content = self.content_area.clone();
        content.render(area, frame, true);
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
            // Show selected labels
            let selected: Vec<&str> = self.labels_list.selected_items();
            let custom: Vec<&str> = self.labels.iter().map(String::as_str).collect();
            let all_labels: Vec<&str> = selected
                .into_iter()
                .chain(custom)
                .filter(|l| *l != "+ Add new...")
                .collect();

            let chunks = Layout::vertical([
                Constraint::Length(3), // Current
                Constraint::Min(5),    // List
            ])
            .split(area);

            let current = if all_labels.is_empty() {
                "(none)".to_string()
            } else {
                all_labels.join(", ")
            };

            let current_display = Paragraph::new(format!("Selected: {current}"))
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(current_display, chunks[0]);

            let mut list = self.labels_list.clone();
            list.render(chunks[1], frame.buffer_mut(), true);
        }
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = match self.step {
            WizardStep::Title => "Tab: Next  Esc: Cancel",
            WizardStep::Content => "Tab: Next  Shift+Tab: Back  Esc: Cancel",
            WizardStep::Attachments => "Enter: Add (multiple paths separated by space)  Backspace: Remove last  Tab: Next  Esc: Cancel",
            WizardStep::Category => {
                if self.category_input_mode {
                    "Enter: Confirm  Esc: Cancel input"
                } else {
                    "Enter: Select  Tab: Next  Esc: Cancel"
                }
            }
            WizardStep::Labels => {
                if self.label_input_mode {
                    "Enter: Add label  Esc: Cancel input"
                } else {
                    "Space: Toggle  Enter: Complete  Shift+Tab: Back  Esc: Cancel"
                }
            }
        };

        let help = Paragraph::new(help_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

        frame.render_widget(help, area);
    }
}

/// Parse a string containing shell-escaped paths separated by unescaped spaces.
///
/// Paths can contain escaped spaces (e.g., `/path/to\ file.png`) and multiple
/// paths are separated by unescaped spaces. Returns individual paths with
/// escape sequences removed.
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
            ' ' => {
                // Unescaped space - this is a separator
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
}
