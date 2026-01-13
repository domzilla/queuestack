//! # Search Command
//!
//! Search for items and interactively select one to open.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::io::IsTerminal;

use anyhow::Result;

use super::list::{collect_items, sort_items, SortBy};
use crate::item::FilterCriteria;
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
    // Empty query = no matches
    if args.query.trim().is_empty() {
        anyhow::bail!("No items found matching \"{}\"", args.query);
    }

    let config = Config::load()?;

    // Collect all items (no pre-filtering, search applied after)
    let mut items = collect_items(&config, args.closed, &FilterCriteria::default());

    // Filter by search query
    items.retain(|item| matches_query(item, &args.query, args.full_text));

    // Sort by ID for consistent ordering
    sort_items(&mut items, SortBy::Id);

    if items.is_empty() {
        anyhow::bail!("No items found matching \"{}\"", args.query);
    }

    // Resolve interactive mode (without terminal check - handled separately)
    let interactive = args.interactive.is_enabled(&config);

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

    let Some(selection) = ui::select_item("Select an item", &items, &config)? else {
        return Ok(()); // User cancelled
    };
    ui::open_item_in_editor(&items[selection], &config)?;

    Ok(())
}
