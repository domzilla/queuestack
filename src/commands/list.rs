//! # List Command
//!
//! Lists queuestack items with filtering and sorting options.
//! Also supports listing labels, categories, attachments, and item metadata.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::cmp::Reverse;
use std::path::PathBuf;

use anyhow::{Context, Result};
use owo_colors::OwoColorize;

use crate::{
    commands,
    config::Config,
    item::{matches_filter, FilterCriteria, Item},
    storage,
    tui::screens::ItemAction,
    ui,
    ui::InteractiveArgs,
};

/// Sort order for listing
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum SortBy {
    #[default]
    Id,
    Date,
    Title,
}

/// Status filter for item listing
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StatusFilter {
    /// Show only open/active items (default)
    #[default]
    Open,
    /// Show only closed/archived items
    Closed,
    /// Show all items regardless of status
    All,
}

/// Special list modes
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ListMode {
    /// Standard item listing (default)
    #[default]
    Items,
    /// List unique labels across all items
    Labels,
    /// List unique categories across all items
    Categories,
    /// List attachments for a specific item
    Attachments,
    /// Show metadata/frontmatter for a specific item
    Meta,
    /// List templates
    Templates,
}

/// Filter options for listing
pub struct ListOptions {
    pub mode: ListMode,
    pub status: StatusFilter,
    pub labels: Vec<String>,
    pub author: Option<String>,
    pub category: Option<String>,
    pub sort: SortBy,
    pub interactive: InteractiveArgs,
    /// Item ID (required for --attachments and --meta modes)
    pub id: Option<String>,
    /// Item file path (alternative to id)
    pub file: Option<PathBuf>,
}

impl Default for ListOptions {
    fn default() -> Self {
        Self {
            mode: ListMode::default(),
            status: StatusFilter::default(),
            labels: Vec::new(),
            author: None,
            category: None,
            sort: SortBy::Id,
            interactive: InteractiveArgs::default(),
            id: None,
            file: None,
        }
    }
}

/// Collects and filters items from storage.
///
/// If `include_archived` is true, collects from archive directory,
/// otherwise collects from the main stack directory.
pub fn collect_items(
    config: &Config,
    include_archived: bool,
    filter: &FilterCriteria,
) -> Vec<Item> {
    let paths: Vec<_> = if include_archived {
        storage::walk_archived(config).collect()
    } else {
        storage::walk_items(config).collect()
    };

    paths
        .into_iter()
        .filter_map(|path| Item::load(&path).ok())
        .filter(|item| {
            let category = item
                .path
                .as_ref()
                .and_then(|p| storage::derive_category(config, p));
            matches_filter(item, filter, category.as_deref())
        })
        .collect()
}

/// Sorts items in place by the given sort order.
pub fn sort_items(items: &mut [Item], sort: SortBy) {
    match sort {
        SortBy::Id => items.sort_by(|a, b| a.id().cmp(b.id())),
        SortBy::Date => items.sort_by_key(|item| Reverse(item.created_at())),
        SortBy::Title => items.sort_by_key(|item| item.title().to_lowercase()),
    }
}

/// Collect unique labels from items, sorted alphabetically.
fn collect_unique_labels(items: &[Item]) -> Vec<String> {
    let mut labels: Vec<String> = items
        .iter()
        .flat_map(|item| item.labels().to_vec())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    labels.sort();
    labels
}

/// Collect unique categories from items, sorted alphabetically.
fn collect_unique_categories(items: &[Item], config: &Config) -> Vec<String> {
    let mut categories: Vec<String> = items
        .iter()
        .filter_map(|item| {
            item.path
                .as_ref()
                .and_then(|p| storage::derive_category(config, p))
        })
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    categories.sort();
    categories
}

/// Executes the list command.
pub fn execute(filter: &ListOptions) -> Result<()> {
    let config = Config::load()?;

    match filter.mode {
        ListMode::Items => execute_items(filter, &config),
        ListMode::Labels => execute_labels(filter, &config),
        ListMode::Categories => execute_categories(filter, &config),
        ListMode::Attachments => execute_attachments(filter, &config),
        ListMode::Meta => execute_meta(filter, &config),
        ListMode::Templates => execute_templates(filter, &config),
    }
}

/// Lists items (default mode).
fn execute_items(filter: &ListOptions, config: &Config) -> Result<()> {
    // Collect items based on status filter
    let item_filter = FilterCriteria {
        labels: filter.labels.clone(),
        author: filter.author.clone(),
        category: filter.category.clone(),
        ..FilterCriteria::default()
    };

    let mut items = match filter.status {
        StatusFilter::Open => collect_items(config, false, &item_filter),
        StatusFilter::Closed => collect_items(config, true, &item_filter),
        StatusFilter::All => {
            let mut open = collect_items(config, false, &item_filter);
            let closed = collect_items(config, true, &item_filter);
            open.extend(closed);
            open
        }
    };

    // Sort items
    sort_items(&mut items, filter.sort);

    // Display
    if items.is_empty() {
        println!("No items found.");
        return Ok(());
    }

    // Check interactive mode
    if !filter.interactive.should_run(config) {
        // Non-interactive: print file paths
        for item in &items {
            if let Some(ref path) = item.path {
                println!("{}", config.relative_path(path).display());
            }
        }
        return Ok(());
    }

    // Collect available labels and categories for filter overlay
    let available_labels = collect_unique_labels(&items);
    let available_categories = collect_unique_categories(&items, config);

    // Interactive: TUI selection with actions
    let Some(action) = ui::select_item_with_actions(
        "Select an item",
        &items,
        config,
        available_labels,
        available_categories,
    )?
    else {
        return Ok(()); // User cancelled
    };

    handle_item_action(action, config)?;

    Ok(())
}

/// Handle an action selected from the item action popup.
fn handle_item_action(action: ItemAction, config: &Config) -> Result<()> {
    match action {
        ItemAction::View(path) => {
            // Open in editor
            let item = Item::load(&path)?;
            ui::open_item_in_editor(&item, config)?;
        }
        ItemAction::Edit(path) => {
            // Launch edit wizard
            execute_edit_wizard(&path, config)?;
        }
        ItemAction::Close(path) => {
            commands::execute_close(None, Some(path))?;
        }
        ItemAction::Reopen(path) => {
            commands::execute_reopen(None, Some(path))?;
        }
        ItemAction::Delete(path) => {
            // Show confirmation dialog
            let item = Item::load(&path)?;
            let message = format!("Delete '{}'?", item.title());
            if ui::confirm(&message)? == Some(true) {
                storage::delete_item(&path)?;
                println!(
                    "{} Deleted: {}",
                    "✓".green(),
                    config.relative_path(&path).display()
                );
            }
        }
    }

    Ok(())
}

/// Execute the edit wizard for an existing item.
fn execute_edit_wizard(path: &std::path::Path, config: &Config) -> Result<()> {
    use crate::tui::{self, screens::NewItemWizard};

    // Load the item
    let item = Item::load(path)?;

    // Collect existing metadata
    let (existing_categories, existing_labels) = commands::new::collect_existing_metadata(config);

    // Get current category from path
    let current_category = storage::derive_category(config, path);

    // Create pre-populated wizard
    let wizard = NewItemWizard::new(existing_categories, existing_labels)
        .with_title(item.title())
        .with_attachments(item.attachments().to_vec())
        .with_category(current_category.clone())
        .with_labels(item.labels())
        .with_item_id(item.id())
        .for_editing();

    // Run wizard
    let Some(output) = tui::run(wizard)? else {
        println!("{}", "Cancelled.".dimmed());
        return Ok(());
    };

    // Apply changes (body is edited in external editor, not in wizard)
    let mut updated = item;
    updated.set_title(output.title);
    updated.frontmatter.labels = output.labels;

    // Handle new attachments
    if !output.attachments.is_empty() {
        // For new attachments, we need to process them
        for source in &output.attachments {
            // Skip existing attachments
            if updated.attachments().contains(source) {
                continue;
            }
            // Process new attachment
            if let Ok(result) = storage::process_attachment(source, &mut updated, path) {
                match result {
                    storage::AttachmentResult::UrlAdded(url) => {
                        println!("  {} {}", "+".green(), url);
                    }
                    storage::AttachmentResult::FileCopied { original, new_name } => {
                        println!("  {} {} -> {}", "+".green(), original, new_name);
                    }
                    storage::AttachmentResult::FileNotFound(p) => {
                        eprintln!("  {} File not found: {}", "!".yellow(), p);
                    }
                }
            }
        }
    }

    // Save the item
    updated.save(path)?;

    // Handle category change - need to move file
    let final_path = if output.category == current_category {
        ui::print_success("Updated", config, path);
        path.to_path_buf()
    } else {
        // Move to new category
        let (new_path, warnings) =
            storage::move_to_category(config, path, output.category.as_deref())?;
        ui::print_warnings(&warnings);
        ui::print_success("Updated", config, &new_path);
        new_path
    };

    // Open editor for content editing
    crate::editor::open(&final_path, config).context("Failed to open editor")?;

    Ok(())
}

/// Lists all unique labels across items.
fn execute_labels(filter: &ListOptions, config: &Config) -> Result<()> {
    let item_filter = FilterCriteria::default();

    // Load all items to get complete label vocabulary
    let all_items = storage::load_all_items(config);
    let all_label_counts = ui::count_by_many(&all_items, |item: &Item| item.labels().to_vec());

    if all_label_counts.is_empty() {
        println!("No labels found.");
        return Ok(());
    }

    // Count only open items per label (for display and selectability)
    let open_items = collect_items(config, false, &item_filter);
    let open_label_counts = ui::count_by_many(&open_items, |item: &Item| item.labels().to_vec());

    // Build label list: all labels with their open counts
    let mut labels: Vec<(String, usize)> = all_label_counts
        .keys()
        .map(|label| {
            let open_count = open_label_counts.get(label).copied().unwrap_or(0);
            (label.clone(), open_count)
        })
        .collect();

    // Sort by open count (descending), then alphabetically
    labels.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    // Check interactive mode
    if !filter.interactive.should_run(config) {
        // Non-interactive: print labels with open count, one per line
        for (label, count) in &labels {
            println!("{label} ({count})");
        }
        return Ok(());
    }

    // Interactive selection - show all labels but only allow selecting ones with open items
    let selectable_indices: Vec<usize> = labels
        .iter()
        .enumerate()
        .filter(|(_, (_, count))| *count > 0)
        .map(|(i, _)| i)
        .collect();

    // Build display options
    let options: Vec<String> = labels
        .iter()
        .map(|(label, count)| format!("{label} ({count})"))
        .collect();

    // Show TUI even if all items are disabled (user can view and ESC to exit)
    let Some(selection) = ui::select_from_list_filtered(
        "Select a label to filter by",
        &options,
        &selectable_indices,
    )?
    else {
        return Ok(()); // User cancelled
    };
    let selected_label = &labels[selection].0;

    // Filter open items with selected label
    let filtered: Vec<&Item> = open_items
        .iter()
        .filter(|item| item.labels().iter().any(|l| l == selected_label))
        .collect();

    if filtered.is_empty() {
        println!("No items found.");
        return Ok(());
    }

    // Interactive: TUI selection for items
    let Some(item_selection) = ui::select_item("Select an item to open", &filtered, config)? else {
        return Ok(()); // User cancelled
    };
    let item = filtered[item_selection];
    ui::open_item_in_editor(item, config)?;

    Ok(())
}

/// Lists all unique categories across items.
fn execute_categories(filter: &ListOptions, config: &Config) -> Result<()> {
    let item_filter = FilterCriteria::default();

    // Load all items to get complete category vocabulary
    let all_items = storage::load_all_items(config);
    let all_category_counts = ui::count_by(&all_items, |item: &Item| {
        item.path
            .as_ref()
            .and_then(|p| storage::derive_category(config, p))
    });

    if all_category_counts.is_empty() {
        println!("No categories found.");
        return Ok(());
    }

    // Count only open items per category (for display and selectability)
    let open_items = collect_items(config, false, &item_filter);
    let open_category_counts = ui::count_by(&open_items, |item: &Item| {
        item.path
            .as_ref()
            .and_then(|p| storage::derive_category(config, p))
    });

    // Build category list: all categories with their open counts
    let mut categories: Vec<(Option<String>, usize)> = all_category_counts
        .keys()
        .map(|category| {
            let open_count = open_category_counts.get(category).copied().unwrap_or(0);
            (category.clone(), open_count)
        })
        .collect();

    // Sort by count (descending), then alphabetically (None/"Uncategorized" last)
    categories.sort_by(|a, b| {
        b.1.cmp(&a.1).then_with(|| match (&a.0, &b.0) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(_), None) => std::cmp::Ordering::Less,
            (Some(a), Some(b)) => a.cmp(b),
        })
    });

    // Check interactive mode
    if !filter.interactive.should_run(config) {
        // Non-interactive: print categories with count, one per line
        for (category, count) in &categories {
            let name = category.as_deref().unwrap_or("Uncategorized");
            println!("{name} ({count})");
        }
        return Ok(());
    }

    // Interactive selection - show all categories but only allow selecting ones with open items
    let selectable_indices: Vec<usize> = categories
        .iter()
        .enumerate()
        .filter(|(_, (_, count))| *count > 0)
        .map(|(i, _)| i)
        .collect();

    // Build display options
    let options: Vec<String> = categories
        .iter()
        .map(|(cat, count)| {
            let name = cat.as_deref().unwrap_or("Uncategorized");
            format!("{name} ({count})")
        })
        .collect();

    // Show TUI even if all items are disabled (user can view and ESC to exit)
    let Some(selection) = ui::select_from_list_filtered(
        "Select a category to filter by",
        &options,
        &selectable_indices,
    )?
    else {
        return Ok(()); // User cancelled
    };
    let selected_category = &categories[selection].0;

    // Filter open items in selected category
    let filtered: Vec<&Item> = open_items
        .iter()
        .filter(|item| {
            let cat = item
                .path
                .as_ref()
                .and_then(|p| storage::derive_category(config, p));
            cat == *selected_category
        })
        .collect();

    if filtered.is_empty() {
        println!("No items found.");
        return Ok(());
    }

    // Interactive: TUI selection for items
    let Some(item_selection) = ui::select_item("Select an item to open", &filtered, config)? else {
        return Ok(()); // User cancelled
    };
    let item = filtered[item_selection];
    ui::open_item_in_editor(item, config)?;

    Ok(())
}

/// Lists attachments for a specific item.
fn execute_attachments(filter: &ListOptions, config: &Config) -> Result<()> {
    let item_ref = storage::ItemRef::from_options(filter.id.clone(), filter.file.clone())?;

    // Find and load the item
    let storage::LoadedItem { item, .. } = item_ref.resolve(config)?;

    let attachments = item.attachments();

    if attachments.is_empty() {
        println!("No attachments.");
        return Ok(());
    }

    // Print attachments one per line
    for attachment in attachments {
        println!("{attachment}");
    }

    Ok(())
}

/// Shows metadata/frontmatter for a specific item.
fn execute_meta(filter: &ListOptions, config: &Config) -> Result<()> {
    let item_ref = storage::ItemRef::from_options(filter.id.clone(), filter.file.clone())?;

    // Find and load the item
    let storage::LoadedItem { path, item } = item_ref.resolve(config)?;

    // Print frontmatter fields
    println!("id: {}", item.id());
    println!("title: {}", item.title());
    println!("author: {}", item.author());
    println!("created_at: {}", item.created_at());
    println!("status: {}", item.status());

    let labels = item.labels();
    if !labels.is_empty() {
        println!("labels: {}", labels.join(", "));
    }

    // Derive category from path
    if let Some(category) = storage::derive_category(config, &path) {
        println!("category: {category}");
    }

    let attachments = item.attachments();
    if !attachments.is_empty() {
        println!("attachments:");
        for attachment in attachments {
            println!("  - {attachment}");
        }
    }

    Ok(())
}

/// Lists all templates.
fn execute_templates(filter: &ListOptions, config: &Config) -> Result<()> {
    // Collect all templates
    let mut templates: Vec<Item> = storage::walk_templates(config)
        .filter_map(|path| Item::load(&path).ok())
        .collect();

    if templates.is_empty() {
        println!("No templates found.");
        return Ok(());
    }

    // Sort templates by ID (default)
    sort_items(&mut templates, filter.sort);

    // Check interactive mode
    if !filter.interactive.should_run(config) {
        // Non-interactive: print file paths
        for template in &templates {
            if let Some(ref path) = template.path {
                println!("{}", config.relative_path(path).display());
            }
        }
        return Ok(());
    }

    // Interactive: show selection with tabular layout matching item list
    let header = format!(
        "{:<15}  {:<40}  {:<20}  {}",
        "ID", "Title", "Labels", "Category"
    );

    let options: Vec<String> = templates
        .iter()
        .map(|t| {
            let labels_str = ui::truncate(&t.labels().join(", "), 20);
            let title_truncated = ui::truncate(t.title(), 40);
            let category = t
                .path
                .as_ref()
                .and_then(|p| storage::derive_category(config, p))
                .unwrap_or_default();

            format!(
                "{:<15}  {}  {}  {}",
                t.id(),
                ui::pad_to_width(&title_truncated, 40),
                ui::pad_to_width(&labels_str, 20),
                category
            )
        })
        .collect();

    let Some(selection) = ui::select_from_list_with_header("Select a template", &header, &options)?
    else {
        return Ok(()); // User cancelled
    };

    // Open selected template in editor
    let template = &templates[selection];
    ui::open_item_in_editor(template, config)?;

    Ok(())
}
