//! Item selection screen with action popup.
//!
//! Provides an interactive list of items with a popup menu for quick actions
//! like View, Edit, Close/Reopen, Attachments, and Delete.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{
    config::Config,
    constants::{UI_LABELS_TRUNCATE_LEN, UI_TITLE_TRUNCATE_LEN},
    item::{Item, Status},
    storage,
    tui::{
        event::TuiEvent,
        widgets::{ActionMenu, ActionMenuResult, MenuItem, SelectAction, SelectList},
        AppResult, TuiApp,
    },
    ui::{pad_to_width, truncate},
};

/// Actions that can be performed on an item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemAction {
    /// Open item in editor (view-only)
    View(PathBuf),
    /// Edit item via wizard
    Edit(PathBuf),
    /// Close an open item
    Close(PathBuf),
    /// Reopen a closed item
    Reopen(PathBuf),
    /// Delete item (move to trash)
    Delete(PathBuf),
}

/// Internal action kinds for the popup menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionKind {
    View,
    Edit,
    Close,
    Reopen,
    Delete,
    Cancel,
}

/// Information about an item in the list.
struct ItemInfo {
    path: PathBuf,
    status: Status,
    display: String,
}

/// Screen state.
enum ScreenState {
    /// Browsing the item list.
    Browsing,
    /// Showing the action popup for an item.
    ShowingPopup {
        item_index: usize,
        menu: ActionMenu,
        actions: Vec<ActionKind>,
    },
}

/// Item selection screen with action popup.
pub struct ItemActionScreen {
    items: Vec<ItemInfo>,
    list: SelectList,
    header: String,
    prompt: String,
    state: ScreenState,
}

impl ItemActionScreen {
    /// Create a new item action screen.
    pub fn new<T: AsRef<Item>>(prompt: &str, items: &[T], config: &Config) -> Self {
        let header = format!(
            "{:<15} {:>6}  {:<40}  {:<20}  {}",
            "ID", "Status", "Title", "Labels", "Category"
        );

        let item_infos: Vec<ItemInfo> = items
            .iter()
            .map(|item| {
                let item = item.as_ref();
                let status_str = match item.status() {
                    Status::Open => "open",
                    Status::Closed => "closed",
                };
                let labels = truncate(&item.labels().join(", "), UI_LABELS_TRUNCATE_LEN);
                let category_opt = item
                    .path
                    .as_ref()
                    .and_then(|p| storage::derive_category(config, p));
                let category = category_opt.as_deref().unwrap_or("");
                let title = truncate(item.title(), UI_TITLE_TRUNCATE_LEN);

                let display = format!(
                    "{:<15} {:>6}  {}  {}  {}",
                    item.id(),
                    status_str,
                    pad_to_width(&title, 40),
                    pad_to_width(&labels, 20),
                    category
                );

                ItemInfo {
                    path: item.path.clone().unwrap_or_default(),
                    status: item.status(),
                    display,
                }
            })
            .collect();

        let display_strings: Vec<String> = item_infos.iter().map(|i| i.display.clone()).collect();
        let list = SelectList::new(display_strings);

        Self {
            items: item_infos,
            list,
            header,
            prompt: prompt.to_string(),
            state: ScreenState::Browsing,
        }
    }

    /// Build popup menu items based on item status.
    fn build_popup_items(status: Status) -> (Vec<MenuItem>, Vec<ActionKind>) {
        let mut items = Vec::new();
        let mut actions = Vec::new();

        // Section 1: View/Edit actions
        items.push(MenuItem::action("View...", "open in editor", actions.len()));
        actions.push(ActionKind::View);

        if status == Status::Open {
            items.push(MenuItem::action(
                "Edit...",
                "modify via wizard",
                actions.len(),
            ));
            actions.push(ActionKind::Edit);
        }

        // Separator
        items.push(MenuItem::separator());

        // Section 2: Status actions
        if status == Status::Open {
            items.push(MenuItem::action_colored(
                "Close",
                "archive item",
                Color::Yellow,
                actions.len(),
            ));
            actions.push(ActionKind::Close);
        } else {
            items.push(MenuItem::action_colored(
                "Reopen",
                "restore to active",
                Color::Green,
                actions.len(),
            ));
            actions.push(ActionKind::Reopen);
        }

        items.push(MenuItem::action_colored(
            "Delete",
            "move to trash",
            Color::Red,
            actions.len(),
        ));
        actions.push(ActionKind::Delete);

        // Separator
        items.push(MenuItem::separator());

        // Section 3: Cancel
        items.push(MenuItem::action("Cancel", "ESC", actions.len()));
        actions.push(ActionKind::Cancel);

        (items, actions)
    }

    /// Open the popup for the currently selected item.
    fn open_popup(&mut self) {
        if let Some(idx) = self.list.selected_index() {
            let item = &self.items[idx];
            let title = if item.status == Status::Open {
                "Actions"
            } else {
                "Actions (Archived)"
            };
            let (menu_items, actions) = Self::build_popup_items(item.status);
            let menu = ActionMenu::new(title, menu_items);
            self.state = ScreenState::ShowingPopup {
                item_index: idx,
                menu,
                actions,
            };
        }
    }

    /// Handle events while browsing the list.
    fn handle_browsing(&mut self, event: &TuiEvent) -> Option<AppResult<ItemAction>> {
        if let TuiEvent::Key(key) = event {
            // Handle Ctrl+C
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Some(AppResult::Cancelled);
            }

            match self.list.handle_key(*key) {
                SelectAction::Confirm => {
                    self.open_popup();
                    None
                }
                SelectAction::Cancel => Some(AppResult::Cancelled),
                SelectAction::None => None,
            }
        } else {
            None
        }
    }

    /// Handle events while showing the popup.
    fn handle_popup(
        &mut self,
        event: &TuiEvent,
        item_index: usize,
        actions: &[ActionKind],
    ) -> Option<AppResult<ItemAction>> {
        if let TuiEvent::Key(key) = event {
            // Handle Ctrl+C
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Some(AppResult::Cancelled);
            }

            // Get mutable reference to menu
            let ScreenState::ShowingPopup { menu, .. } = &mut self.state else {
                return None;
            };

            match menu.handle_key(*key) {
                Some(ActionMenuResult::Selected(action_idx)) => {
                    let item = &self.items[item_index];
                    let path = item.path.clone();
                    match actions[action_idx] {
                        ActionKind::View => Some(AppResult::Done(ItemAction::View(path))),
                        ActionKind::Edit => Some(AppResult::Done(ItemAction::Edit(path))),
                        ActionKind::Close => Some(AppResult::Done(ItemAction::Close(path))),
                        ActionKind::Reopen => Some(AppResult::Done(ItemAction::Reopen(path))),
                        ActionKind::Delete => Some(AppResult::Done(ItemAction::Delete(path))),
                        ActionKind::Cancel => {
                            // Close popup, return to browsing
                            self.state = ScreenState::Browsing;
                            None
                        }
                    }
                }
                Some(ActionMenuResult::Cancelled) => {
                    // Close popup, return to browsing
                    self.state = ScreenState::Browsing;
                    None
                }
                None => None,
            }
        } else {
            None
        }
    }

    fn render_list(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Layout: prompt, header, list, help
        let chunks = Layout::vertical([
            Constraint::Length(3), // Prompt
            Constraint::Length(1), // Header
            Constraint::Min(5),    // List
            Constraint::Length(3), // Help
        ])
        .split(area);

        // Prompt
        let prompt_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let prompt = Paragraph::new(self.prompt.as_str()).block(prompt_block);
        frame.render_widget(prompt, chunks[0]);

        // Header
        let header = Paragraph::new(self.header.as_str()).style(
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(header, chunks[1]);

        // List
        self.list.render(chunks[2], frame.buffer_mut(), true);

        // Help
        let help_spans = vec![
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::raw(" Select  "),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(" Cancel"),
        ];
        let help = Paragraph::new(Line::from(help_spans)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(help, chunks[3]);
    }
}

impl TuiApp for ItemActionScreen {
    type Output = ItemAction;

    fn handle_event(&mut self, event: &TuiEvent) -> Option<AppResult<Self::Output>> {
        // Clone state info we need for handling
        let (item_index, actions) = match &self.state {
            ScreenState::Browsing => return self.handle_browsing(event),
            ScreenState::ShowingPopup {
                item_index,
                actions,
                ..
            } => (*item_index, actions.clone()),
        };

        self.handle_popup(event, item_index, &actions)
    }

    fn render(&mut self, frame: &mut Frame) {
        // Always render the list first
        self.render_list(frame);

        // If popup is active, render it on top
        if let ScreenState::ShowingPopup { menu, .. } = &mut self.state {
            menu.render(frame.area(), frame.buffer_mut());
        }
    }
}

/// Run the item action screen.
///
/// Returns the selected action, or `Ok(None)` if cancelled.
pub fn select_item_with_actions<T: AsRef<Item>>(
    prompt: &str,
    items: &[T],
    config: &Config,
) -> anyhow::Result<Option<ItemAction>> {
    use crate::tui::run;
    let screen = ItemActionScreen::new(prompt, items, config);
    run(screen)
}
