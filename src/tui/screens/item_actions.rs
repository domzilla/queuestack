//! Item selection screen with action popup and filter overlay.
//!
//! Provides an interactive list of items with a popup menu for quick actions
//! like View, Edit, Close/Reopen, and Delete. Also supports filtering by
//! search query, labels, and category.

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
        widgets::{
            ActionMenu, ActionMenuResult, FilterOverlay, FilterOverlayResult, FilterState,
            MenuItem, SelectAction, SelectList,
        },
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
    /// For filtering
    title: String,
    id: String,
    body: String,
    labels: Vec<String>,
    category: Option<String>,
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
    /// Showing the filter overlay.
    ShowingFilter { overlay: Box<FilterOverlay> },
}

/// Item selection screen with action popup and filter support.
pub struct ItemActionScreen {
    /// All items (unfiltered)
    all_items: Vec<ItemInfo>,
    /// Indices into `all_items` that match the current filter
    filtered_indices: Vec<usize>,
    /// Current filter state
    filter_state: FilterState,
    /// Available labels for filter overlay
    available_labels: Vec<String>,
    /// Available categories for filter overlay
    available_categories: Vec<String>,
    /// The display list widget
    list: SelectList,
    header: String,
    prompt: String,
    state: ScreenState,
}

impl ItemActionScreen {
    /// Create a new item action screen.
    pub fn new<T: AsRef<Item>>(
        prompt: &str,
        items: &[T],
        config: &Config,
        available_labels: Vec<String>,
        available_categories: Vec<String>,
    ) -> Self {
        let header = format!(
            "{:<15} {:>6}  {:<40}  {:<20}  {}",
            "ID", "Status", "Title", "Labels", "Category"
        );

        let all_items: Vec<ItemInfo> = items
            .iter()
            .map(|item| {
                let item = item.as_ref();
                let status_str = match item.status() {
                    Status::Open => "open",
                    Status::Closed => "closed",
                };
                let labels_str = truncate(&item.labels().join(", "), UI_LABELS_TRUNCATE_LEN);
                let category_opt = item
                    .path
                    .as_ref()
                    .and_then(|p| storage::derive_category(config, p));
                let category = category_opt.as_deref().unwrap_or("");
                let title_truncated = truncate(item.title(), UI_TITLE_TRUNCATE_LEN);

                let display = format!(
                    "{:<15} {:>6}  {}  {}  {}",
                    item.id(),
                    status_str,
                    pad_to_width(&title_truncated, 40),
                    pad_to_width(&labels_str, 20),
                    category
                );

                ItemInfo {
                    path: item.path.clone().unwrap_or_default(),
                    status: item.status(),
                    display,
                    title: item.title().to_string(),
                    id: item.id().to_string(),
                    body: item.body.clone(),
                    labels: item.labels().to_vec(),
                    category: category_opt,
                }
            })
            .collect();

        // Initially all items are shown
        let filtered_indices: Vec<usize> = (0..all_items.len()).collect();
        let display_strings: Vec<String> = all_items.iter().map(|i| i.display.clone()).collect();
        let list = SelectList::new(display_strings);

        Self {
            all_items,
            filtered_indices,
            filter_state: FilterState::default(),
            available_labels,
            available_categories,
            list,
            header,
            prompt: prompt.to_string(),
            state: ScreenState::Browsing,
        }
    }

    /// Apply the current filter state to update `filtered_indices` and rebuild the list.
    fn apply_filter(&mut self) {
        let search_lower = self.filter_state.search.to_lowercase();

        self.filtered_indices = self
            .all_items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                // Search filter (FTS)
                let matches_search = search_lower.is_empty()
                    || item.title.to_lowercase().contains(&search_lower)
                    || item.id.to_lowercase().contains(&search_lower)
                    || item.body.to_lowercase().contains(&search_lower);

                // Label filter (OR logic - item has ANY of the selected labels)
                let matches_labels = self.filter_state.labels.is_empty()
                    || self
                        .filter_state
                        .labels
                        .iter()
                        .any(|label| item.labels.contains(label));

                // Category filter
                let matches_category = self.filter_state.category.is_none()
                    || item.category == self.filter_state.category;

                matches_search && matches_labels && matches_category
            })
            .map(|(i, _)| i)
            .collect();

        self.rebuild_display_list();
    }

    /// Rebuild the `SelectList` based on `filtered_indices`.
    fn rebuild_display_list(&mut self) {
        let display_strings: Vec<String> = self
            .filtered_indices
            .iter()
            .map(|&i| self.all_items[i].display.clone())
            .collect();

        // Create new list with filtered items
        self.list = SelectList::new(display_strings);
    }

    /// Get the actual item index from the filtered list index.
    fn actual_index(&self, filtered_idx: usize) -> Option<usize> {
        self.filtered_indices.get(filtered_idx).copied()
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
        if let Some(filtered_idx) = self.list.selected_index() {
            if let Some(actual_idx) = self.actual_index(filtered_idx) {
                let item = &self.all_items[actual_idx];
                let title = if item.status == Status::Open {
                    "Actions"
                } else {
                    "Actions (Archived)"
                };
                let (menu_items, actions) = Self::build_popup_items(item.status);
                let menu = ActionMenu::new(title, menu_items);
                self.state = ScreenState::ShowingPopup {
                    item_index: actual_idx,
                    menu,
                    actions,
                };
            }
        }
    }

    /// Open the filter overlay.
    fn open_filter(&mut self) {
        let overlay = FilterOverlay::new(
            self.available_labels.clone(),
            self.available_categories.clone(),
            &self.filter_state,
        );
        self.state = ScreenState::ShowingFilter {
            overlay: Box::new(overlay),
        };
    }

    /// Handle events while browsing the list.
    fn handle_browsing(&mut self, event: &TuiEvent) -> Option<AppResult<ItemAction>> {
        if let TuiEvent::Key(key) = event {
            // Handle Ctrl+C
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Some(AppResult::Cancelled);
            }

            // Handle 'f' to open filter
            if key.code == KeyCode::Char('f') {
                self.open_filter();
                return None;
            }

            // Handle 'c' to clear filter (only if filter is active)
            if key.code == KeyCode::Char('c') && !self.filter_state.is_empty() {
                self.filter_state.clear();
                self.apply_filter();
                return None;
            }

            match self.list.handle_key(*key) {
                SelectAction::Confirm => {
                    if !self.filtered_indices.is_empty() {
                        self.open_popup();
                    }
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
                    let item = &self.all_items[item_index];
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

    /// Handle events while showing the filter overlay.
    fn handle_filter(&mut self, event: &TuiEvent) -> Option<AppResult<ItemAction>> {
        if let TuiEvent::Key(key) = event {
            // Handle Ctrl+C
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Some(AppResult::Cancelled);
            }

            // Get mutable reference to overlay
            let ScreenState::ShowingFilter { overlay } = &mut self.state else {
                return None;
            };

            match overlay.handle_key(*key) {
                Some(FilterOverlayResult::Applied(new_state)) => {
                    self.filter_state = new_state;
                    self.apply_filter();
                    self.state = ScreenState::Browsing;
                    None
                }
                Some(FilterOverlayResult::Cancelled) => {
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

        // Prompt with filter indicator
        let filter_active = !self.filter_state.is_empty();
        let prompt_text = if filter_active {
            format!(
                "{} ({} of {} items)",
                self.prompt,
                self.filtered_indices.len(),
                self.all_items.len()
            )
        } else {
            self.prompt.clone()
        };

        let prompt_border_color = if filter_active {
            Color::Yellow
        } else {
            Color::Cyan
        };

        let prompt_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(prompt_border_color));
        let prompt = Paragraph::new(prompt_text).block(prompt_block);
        frame.render_widget(prompt, chunks[0]);

        // Header
        let header = Paragraph::new(self.header.as_str()).style(
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(header, chunks[1]);

        // List (or empty message)
        if self.filtered_indices.is_empty() {
            let empty_msg = if self.all_items.is_empty() {
                "No items."
            } else {
                "No matching items."
            };
            let empty = Paragraph::new(empty_msg)
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(empty, chunks[2]);
        } else {
            self.list.render(chunks[2], frame.buffer_mut(), true);
        }

        // Help - with filter right-aligned
        let help_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let help_inner = help_block.inner(chunks[3]);
        frame.render_widget(help_block, chunks[3]);

        // Left side: Enter Select, Esc Cancel
        let left_spans = vec![
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::raw(" Select  "),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(" Cancel"),
        ];
        let left_help = Paragraph::new(Line::from(left_spans));
        frame.render_widget(left_help, help_inner);

        // Right side: c Clear (grayed when no filter), f Filter
        let filter_active = !self.filter_state.is_empty();
        let clear_style = if filter_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let clear_text_style = if filter_active {
            Style::default()
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let right_spans = vec![
            Span::styled("c", clear_style),
            Span::styled(" Clear  ", clear_text_style),
            Span::styled("f", Style::default().fg(Color::Cyan)),
            Span::raw(" Filter"),
        ];
        let right_help =
            Paragraph::new(Line::from(right_spans)).alignment(ratatui::layout::Alignment::Right);
        frame.render_widget(right_help, help_inner);
    }
}

impl TuiApp for ItemActionScreen {
    type Output = ItemAction;

    fn handle_event(&mut self, event: &TuiEvent) -> Option<AppResult<Self::Output>> {
        match &self.state {
            ScreenState::Browsing => self.handle_browsing(event),
            ScreenState::ShowingPopup {
                item_index,
                actions,
                ..
            } => {
                let item_index = *item_index;
                let actions = actions.clone();
                self.handle_popup(event, item_index, &actions)
            }
            ScreenState::ShowingFilter { .. } => self.handle_filter(event),
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        // Always render the list first
        self.render_list(frame);

        // Render overlays on top
        match &mut self.state {
            ScreenState::ShowingPopup { menu, .. } => {
                menu.render(frame.area(), frame.buffer_mut());
            }
            ScreenState::ShowingFilter { overlay } => {
                overlay.render(frame.area(), frame.buffer_mut());
            }
            ScreenState::Browsing => {}
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
    available_labels: Vec<String>,
    available_categories: Vec<String>,
) -> anyhow::Result<Option<ItemAction>> {
    use crate::tui::run;
    let screen = ItemActionScreen::new(
        prompt,
        items,
        config,
        available_labels,
        available_categories,
    );
    run(screen)
}
