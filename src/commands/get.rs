//! # Get Command
//!
//! Retrieves and opens the first matching item.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use anyhow::{Context, Result};

use super::list::{collect_items, sort_items, ItemFilter, SortBy};
use crate::{config::Config, editor};

/// Arguments for the get command
pub struct GetArgs {
    pub label: Option<String>,
    pub author: Option<String>,
    pub sort: SortBy,
    pub no_open: bool,
    pub closed: bool,
}

impl Default for GetArgs {
    fn default() -> Self {
        Self {
            label: None,
            author: None,
            sort: SortBy::Id,
            no_open: false,
            closed: false,
        }
    }
}

/// Executes the get command.
///
/// Retrieves the first item matching the filters (after sorting) and
/// optionally opens it in the editor.
pub fn execute(args: &GetArgs) -> Result<()> {
    let config = Config::load()?;

    // Collect items
    let item_filter = ItemFilter {
        label: args.label.clone(),
        author: args.author.clone(),
    };

    let mut items = collect_items(&config, args.closed, &item_filter);

    if items.is_empty() {
        anyhow::bail!("No items found matching the criteria");
    }

    // Sort items
    sort_items(&mut items, args.sort);

    // Get first item
    let item = items.into_iter().next().expect("items not empty");
    let path = item.path.as_ref().context("Item has no path")?;

    // Output the path
    println!("{}", config.relative_path(path).display());

    // Open in editor if auto_open is enabled and not suppressed
    if config.auto_open() && !args.no_open {
        editor::open(path, &config).context("Failed to open editor")?;
    }

    Ok(())
}
