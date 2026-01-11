//! # Search Command
//!
//! Search for items and interactively select one to open.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::io::IsTerminal;

use anyhow::Result;

use super::list::{collect_items, sort_items, ItemFilter, SortBy};
use crate::{config::Config, item::search::matches_query, ui, ui::InteractiveArgs};

/// Arguments for the search command
pub struct SearchArgs {
    pub query: String,
    pub full_text: bool,
    pub interactive: InteractiveArgs,
    pub closed: bool,
}

/// Executes the search command.
pub fn execute(args: &SearchArgs) -> Result<()> {
    let config = Config::load()?;

    // Collect all items
    let item_filter = ItemFilter {
        label: None,
        author: None,
    };

    let mut items = collect_items(&config, args.closed, &item_filter);

    // Filter by search query
    items.retain(|item| matches_query(item, &args.query, args.full_text));

    // Sort by ID for consistent ordering
    sort_items(&mut items, SortBy::Id);

    if items.is_empty() {
        anyhow::bail!("No items found matching \"{}\"", args.query);
    }

    // Resolve interactive mode
    let interactive = args.interactive.resolve(config.interactive());

    // Non-interactive mode: just print the list
    if !interactive {
        for item in &items {
            if let Some(ref path) = item.path {
                println!("{}", config.relative_path(path).display());
            }
        }
        return Ok(());
    }

    // Single match: open directly
    if items.len() == 1 {
        ui::open_item_in_editor(&items[0], &config)?;
        return Ok(());
    }

    // Multiple matches: interactive selection
    if !std::io::stdout().is_terminal() {
        anyhow::bail!(
            "Multiple items found ({}) but not running in a terminal. Use --no-interactive to list them.",
            items.len()
        );
    }

    let selection = ui::select_item("Select an item", &items)?;
    ui::open_item_in_editor(&items[selection], &config)?;

    Ok(())
}
