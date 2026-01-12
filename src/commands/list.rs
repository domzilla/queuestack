//! # List Command
//!
//! Lists qstack items with filtering and sorting options.
//! Also supports listing labels, categories, attachments, and item metadata.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::cmp::Reverse;

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::{config::Config, item::Item, storage, ui, ui::InteractiveArgs};

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
}

/// Filter options for listing
pub struct ListFilter {
    pub mode: ListMode,
    pub status: StatusFilter,
    pub label: Option<String>,
    pub author: Option<String>,
    pub sort: SortBy,
    pub interactive: InteractiveArgs,
    /// Item ID (required for --attachments and --meta modes)
    pub id: Option<String>,
}

impl Default for ListFilter {
    fn default() -> Self {
        Self {
            mode: ListMode::default(),
            status: StatusFilter::default(),
            label: None,
            author: None,
            sort: SortBy::Id,
            interactive: InteractiveArgs::default(),
            id: None,
        }
    }
}

/// Common filter options for item queries
pub struct ItemFilter {
    pub label: Option<String>,
    pub author: Option<String>,
}

/// Collects and filters items from storage.
///
/// If `include_archived` is true, collects from archive directory,
/// otherwise collects from the main stack directory.
pub fn collect_items(config: &Config, include_archived: bool, filter: &ItemFilter) -> Vec<Item> {
    let paths: Vec<_> = if include_archived {
        storage::walk_archived(config).collect()
    } else {
        storage::walk_items(config).collect()
    };

    paths
        .into_iter()
        .filter_map(|path| Item::load(&path).ok())
        .filter(|item| apply_item_filter(item, filter))
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

fn apply_item_filter(item: &Item, filter: &ItemFilter) -> bool {
    // Label filter
    if let Some(ref label) = filter.label {
        if !item.labels().iter().any(|l| l.eq_ignore_ascii_case(label)) {
            return false;
        }
    }

    // Author filter
    if let Some(ref author) = filter.author {
        if !item.author().eq_ignore_ascii_case(author) {
            return false;
        }
    }

    true
}

/// Executes the list command.
pub fn execute(filter: &ListFilter) -> Result<()> {
    let config = Config::load()?;

    match filter.mode {
        ListMode::Items => execute_items(filter, &config),
        ListMode::Labels => execute_labels(filter, &config),
        ListMode::Categories => execute_categories(filter, &config),
        ListMode::Attachments => execute_attachments(filter, &config),
        ListMode::Meta => execute_meta(filter, &config),
    }
}

/// Lists items (default mode).
fn execute_items(filter: &ListFilter, config: &Config) -> Result<()> {
    // Collect items based on status filter
    let item_filter = ItemFilter {
        label: filter.label.clone(),
        author: filter.author.clone(),
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
        println!("{}", "No items found.".dimmed());
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

    // Interactive: TUI selection
    let Some(selection) = ui::select_item("Select an item to open", &items)? else {
        return Ok(()); // User cancelled
    };
    let item = &items[selection];
    ui::open_item_in_editor(item, config)?;

    Ok(())
}

/// Lists all unique labels across items.
fn execute_labels(filter: &ListFilter, config: &Config) -> Result<()> {
    let item_filter = ItemFilter {
        label: None,
        author: None,
    };

    // Load all items to get complete label vocabulary
    let all_items = storage::load_all_items(config);
    let all_label_counts = ui::count_by_many(&all_items, |item: &Item| item.labels().to_vec());

    if all_label_counts.is_empty() {
        println!("{}", "No labels found.".dimmed());
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
        // Non-interactive: print labels one per line (all labels, including those with 0 open)
        for (label, _) in &labels {
            println!("{label}");
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

    if selectable_indices.is_empty() {
        println!("{}", "No labels with open items.".dimmed());
        return Ok(());
    }

    // Build display options with visual distinction for non-selectable items
    let options: Vec<String> = labels
        .iter()
        .map(|(label, count)| {
            if *count > 0 {
                format!("{label} ({count})")
            } else {
                format!("{label} (0)") // Will be shown dimmed in TUI
            }
        })
        .collect();

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
        println!("{}", "No items found.".dimmed());
        return Ok(());
    }

    // Interactive: TUI selection for items
    let Some(item_selection) = ui::select_item("Select an item to open", &filtered)? else {
        return Ok(()); // User cancelled
    };
    let item = filtered[item_selection];
    ui::open_item_in_editor(item, config)?;

    Ok(())
}

/// Lists all unique categories across items.
fn execute_categories(filter: &ListFilter, config: &Config) -> Result<()> {
    let item_filter = ItemFilter {
        label: None,
        author: None,
    };

    // Load all items to get complete category vocabulary
    let all_items = storage::load_all_items(config);
    let all_category_counts =
        ui::count_by(&all_items, |item: &Item| item.category().map(String::from));

    if all_category_counts.is_empty() {
        println!("{}", "No items found.".dimmed());
        return Ok(());
    }

    // Count only open items per category (for display and selectability)
    let open_items = collect_items(config, false, &item_filter);
    let open_category_counts =
        ui::count_by(&open_items, |item: &Item| item.category().map(String::from));

    // Build category list: all categories with their open counts
    let mut categories: Vec<(Option<String>, usize)> = all_category_counts
        .keys()
        .map(|cat| {
            let open_count = open_category_counts.get(cat).copied().unwrap_or(0);
            (cat.clone(), open_count)
        })
        .collect();

    // Sort by open count (descending), then alphabetically (None last)
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
        // Non-interactive: print categories one per line (all, including those with 0 open)
        for (category, _) in &categories {
            let name = category.as_deref().unwrap_or("(uncategorized)");
            println!("{name}");
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

    if selectable_indices.is_empty() {
        println!("{}", "No categories with open items.".dimmed());
        return Ok(());
    }

    // Build display options
    let options: Vec<String> = categories
        .iter()
        .map(|(cat, count)| {
            let name = cat.as_deref().unwrap_or("(uncategorized)");
            format!("{name} ({count})")
        })
        .collect();

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
        .filter(|item| item.category().map(String::from) == *selected_category)
        .collect();

    if filtered.is_empty() {
        println!("{}", "No items found.".dimmed());
        return Ok(());
    }

    // Interactive: TUI selection for items
    let Some(item_selection) = ui::select_item("Select an item to open", &filtered)? else {
        return Ok(()); // User cancelled
    };
    let item = filtered[item_selection];
    ui::open_item_in_editor(item, config)?;

    Ok(())
}

/// Lists attachments for a specific item.
fn execute_attachments(filter: &ListFilter, config: &Config) -> Result<()> {
    let id = filter
        .id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("--id is required with --attachments"))?;

    // Find and load the item
    let storage::LoadedItem { item, .. } = storage::find_and_load(config, id)?;

    let attachments = item.attachments();

    if attachments.is_empty() {
        println!("{}", "No attachments.".dimmed());
        return Ok(());
    }

    // Print attachments one per line
    for attachment in attachments {
        println!("{attachment}");
    }

    Ok(())
}

/// Shows metadata/frontmatter for a specific item.
fn execute_meta(filter: &ListFilter, config: &Config) -> Result<()> {
    let id = filter
        .id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("--id is required with --meta"))?;

    // Find and load the item
    let storage::LoadedItem { item, .. } = storage::find_and_load(config, id)?;

    // Print frontmatter fields
    println!("{}: {}", "id".bold(), item.id());
    println!("{}: {}", "title".bold(), item.title());
    println!("{}: {}", "author".bold(), item.author());
    println!("{}: {}", "created_at".bold(), item.created_at());
    println!("{}: {}", "status".bold(), item.status());

    let labels = item.labels();
    if labels.is_empty() {
        println!("{}: []", "labels".bold());
    } else {
        println!("{}: {}", "labels".bold(), labels.join(", "));
    }

    if let Some(category) = item.category() {
        println!("{}: {}", "category".bold(), category);
    }

    let attachments = item.attachments();
    if !attachments.is_empty() {
        println!("{}:", "attachments".bold());
        for attachment in attachments {
            println!("  - {attachment}");
        }
    }

    Ok(())
}
