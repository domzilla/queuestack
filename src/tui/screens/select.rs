//! Generic selection screen.
//!
//! Replaces dialoguer's Select for item selection.

use std::collections::HashSet;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::{
    event::TuiEvent,
    run,
    widgets::{SelectAction, SelectList},
    AppResult, TuiApp,
};

/// Selection screen application.
struct SelectScreen {
    list: SelectList,
    prompt: String,
    header: Option<String>,
}

impl SelectScreen {
    fn new(prompt: impl Into<String>, items: Vec<String>) -> Self {
        let list = SelectList::new(items).with_title("Select");
        Self {
            list,
            prompt: prompt.into(),
            header: None,
        }
    }

    fn with_header(mut self, header: impl Into<String>) -> Self {
        self.header = Some(header.into());
        self
    }

    fn with_disabled(mut self, disabled: HashSet<usize>) -> Self {
        self.list = self.list.with_disabled(disabled);
        self
    }
}

impl TuiApp for SelectScreen {
    type Output = usize;

    fn handle_event(&mut self, event: &TuiEvent) -> Option<AppResult<Self::Output>> {
        match event {
            TuiEvent::Key(key) => {
                // Handle Ctrl+C
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Some(AppResult::Cancelled);
                }

                match self.list.handle_key(*key) {
                    SelectAction::Confirm => self.list.selected_index().map(AppResult::Done),
                    SelectAction::Cancel => Some(AppResult::Cancelled),
                    SelectAction::None => None,
                }
            }
            _ => None,
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Layout: prompt at top, optional header, list below, help at bottom
        let has_header = self.header.is_some();
        let chunks = if has_header {
            Layout::vertical([
                Constraint::Length(3), // Prompt
                Constraint::Length(1), // Header
                Constraint::Min(5),    // List
                Constraint::Length(3), // Help
            ])
            .split(area)
        } else {
            Layout::vertical([
                Constraint::Length(3), // Prompt
                Constraint::Min(5),    // List
                Constraint::Length(3), // Help
            ])
            .split(area)
        };

        // Prompt
        let prompt_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let prompt = Paragraph::new(self.prompt.as_str()).block(prompt_block);
        frame.render_widget(prompt, chunks[0]);

        // Header (if present)
        let (list_chunk, help_chunk) = if has_header {
            if let Some(ref header) = self.header {
                let header_style = Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD);
                // Add prefix spacing to align with list items:
                // 1 char for list block border + 2 chars for "> " prefix = 3 spaces
                let header_text = format!("   {header}");
                let header_widget = Paragraph::new(header_text).style(header_style);
                frame.render_widget(header_widget, chunks[1]);
            }
            (chunks[2], chunks[3])
        } else {
            (chunks[1], chunks[2])
        };

        // List
        let mut list_clone = self.list.clone();
        list_clone.render(list_chunk, frame.buffer_mut(), true);

        // Help
        let help = Paragraph::new(Line::from(vec![
            ratatui::text::Span::styled("Enter", Style::default().fg(Color::Cyan)),
            ratatui::text::Span::raw(" Select  "),
            ratatui::text::Span::styled("Esc", Style::default().fg(Color::Cyan)),
            ratatui::text::Span::raw(" Cancel"),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

        frame.render_widget(help, help_chunk);
    }
}

/// Select from a list of options.
///
/// Returns `Some(index)` if an item was selected, `None` if cancelled.
pub fn select_from_list<T: ToString>(prompt: &str, options: &[T]) -> Result<Option<usize>> {
    let items: Vec<String> = options.iter().map(ToString::to_string).collect();

    if items.is_empty() {
        anyhow::bail!("No items to select from");
    }

    let app = SelectScreen::new(prompt, items);
    run(app)
}

/// Select from a list of options with a header row.
///
/// The header is displayed above the list items to label columns.
/// Returns `Some(index)` if an item was selected, `None` if cancelled.
pub fn select_from_list_with_header<T: ToString>(
    prompt: &str,
    header: &str,
    options: &[T],
) -> Result<Option<usize>> {
    let items: Vec<String> = options.iter().map(ToString::to_string).collect();

    if items.is_empty() {
        anyhow::bail!("No items to select from");
    }

    let app = SelectScreen::new(prompt, items).with_header(header);
    run(app)
}

/// Select from a list with some items disabled (visible but not selectable).
///
/// `selectable_indices` contains the indices that CAN be selected.
/// Items not in this list are shown dimmed and cannot be navigated to.
/// Returns `Some(index)` if an item was selected, `None` if cancelled.
pub fn select_from_list_filtered<T: ToString>(
    prompt: &str,
    options: &[T],
    selectable_indices: &[usize],
) -> Result<Option<usize>> {
    let items: Vec<String> = options.iter().map(ToString::to_string).collect();

    if items.is_empty() {
        anyhow::bail!("No items to select from");
    }

    // Build disabled set (all indices not in selectable_indices)
    let selectable: HashSet<usize> = selectable_indices.iter().copied().collect();
    let disabled: HashSet<usize> = (0..items.len())
        .filter(|i| !selectable.contains(i))
        .collect();

    let app = SelectScreen::new(prompt, items).with_disabled(disabled);
    run(app)
}
