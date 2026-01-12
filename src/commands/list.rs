//! # List Command
//!
//! Lists qstack items with filtering and sorting options.
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

/// Filter options for listing
pub struct ListFilter {
    pub status: StatusFilter,
    pub label: Option<String>,
    pub author: Option<String>,
    pub sort: SortBy,
    pub interactive: InteractiveArgs,
}

impl Default for ListFilter {
    fn default() -> Self {
        Self {
            status: StatusFilter::default(),
            label: None,
            author: None,
            sort: SortBy::Id,
            interactive: InteractiveArgs::default(),
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

    // Collect items based on status filter
    let item_filter = ItemFilter {
        label: filter.label.clone(),
        author: filter.author.clone(),
    };

    let mut items = match filter.status {
        StatusFilter::Open => collect_items(&config, false, &item_filter),
        StatusFilter::Closed => collect_items(&config, true, &item_filter),
        StatusFilter::All => {
            let mut open = collect_items(&config, false, &item_filter);
            let closed = collect_items(&config, true, &item_filter);
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
    if !filter.interactive.should_run(&config) {
        // Non-interactive: print file paths
        for item in &items {
            if let Some(ref path) = item.path {
                println!("{}", config.relative_path(path).display());
            }
        }
        return Ok(());
    }

    // Interactive: TUI selection
    let selection = ui::select_item("Select an item to open", &items)?;
    let item = &items[selection];
    ui::open_item_in_editor(item, &config)?;

    Ok(())
}
