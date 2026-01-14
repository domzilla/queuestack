//! Text prompt screen.
//!
//! Replaces standard I/O prompts with a TUI interface.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::{event::TuiEvent, run, widgets::TextInput, AppResult, TuiApp};

/// Text prompt screen application.
struct PromptScreen {
    input: TextInput,
    prompt: String,
}

impl PromptScreen {
    fn new(prompt: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            input: TextInput::new(label),
            prompt: prompt.into(),
        }
    }

    fn with_initial(mut self, value: impl Into<String>) -> Self {
        self.input = self.input.with_initial(value);
        self
    }
}

impl TuiApp for PromptScreen {
    type Output = String;

    fn handle_event(&mut self, event: &TuiEvent) -> Option<AppResult<Self::Output>> {
        match event {
            TuiEvent::Paste(content) => {
                self.input.insert_text(content);
                None
            }
            TuiEvent::Key(key) => {
                // Handle Ctrl+C
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Some(AppResult::Cancelled);
                }

                match key.code {
                    KeyCode::Enter => {
                        let content = self.input.content().to_string();
                        if content.is_empty() {
                            None // Don't allow empty input
                        } else {
                            Some(AppResult::Done(content))
                        }
                    }
                    KeyCode::Esc => Some(AppResult::Cancelled),
                    _ => {
                        self.input.handle_key(*key);
                        None
                    }
                }
            }
            _ => None,
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Layout: prompt, input, help
        let chunks = Layout::vertical([
            Constraint::Length(3), // Prompt
            Constraint::Length(3), // Input
            Constraint::Min(1),    // Spacer
            Constraint::Length(3), // Help
        ])
        .split(area);

        // Prompt text
        let prompt_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let prompt = Paragraph::new(self.prompt.as_str()).block(prompt_block);
        frame.render_widget(prompt, chunks[0]);

        // Input
        self.input.render(chunks[1], frame.buffer_mut(), true);

        // Help
        let help = Paragraph::new(Line::from(vec![
            ratatui::text::Span::styled("Enter", Style::default().fg(Color::Cyan)),
            ratatui::text::Span::raw(" Confirm  "),
            ratatui::text::Span::styled("Esc", Style::default().fg(Color::Cyan)),
            ratatui::text::Span::raw(" Cancel"),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

        frame.render_widget(help, chunks[3]);
    }
}

/// Prompt for text input.
///
/// Returns the entered text, or `Ok(None)` if cancelled.
pub fn prompt_text(prompt: &str, label: &str) -> Result<Option<String>> {
    let app = PromptScreen::new(prompt, label);
    run(app)
}

/// Prompt for text input with an initial value.
///
/// Returns the entered text, or `Ok(None)` if cancelled.
#[allow(dead_code)]
pub fn prompt_text_with_initial(
    prompt: &str,
    label: &str,
    initial: &str,
) -> Result<Option<String>> {
    let app = PromptScreen::new(prompt, label).with_initial(initial);
    run(app)
}
