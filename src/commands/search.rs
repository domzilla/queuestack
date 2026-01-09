//! # Search Command
//!
//! Search for items and interactively select one to open.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::io::IsTerminal;

use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Select};

use super::list::{collect_items, sort_items, ItemFilter, SortBy};
use crate::{config::Config, editor, item::Item};

/// Arguments for the search command
pub struct SearchArgs {
    pub query: String,
    pub full_text: bool,
    pub no_open: bool,
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
    let query_lower = args.query.to_lowercase();
    items.retain(|item| matches_query(item, &query_lower, args.full_text));

    // Sort by ID for consistent ordering
    sort_items(&mut items, SortBy::Id);

    if items.is_empty() {
        anyhow::bail!("No items found matching \"{}\"", args.query);
    }

    // Non-interactive mode: just print the list
    if args.no_open {
        for item in &items {
            if let Some(ref path) = item.path {
                println!("{}", config.relative_path(path).display());
            }
        }
        return Ok(());
    }

    // Single match: open directly
    if items.len() == 1 {
        let item = &items[0];
        let path = item.path.as_ref().context("Item has no path")?;
        println!("{}", config.relative_path(path).display());
        editor::open(path, &config).context("Failed to open editor")?;
        return Ok(());
    }

    // Multiple matches: interactive selection
    if !std::io::stdout().is_terminal() {
        anyhow::bail!(
            "Multiple items found ({}) but not running in a terminal. Use --no-open to list them.",
            items.len()
        );
    }

    let selection = interactive_select(&items)?;
    let item = &items[selection];
    let path = item.path.as_ref().context("Item has no path")?;

    println!("{}", config.relative_path(path).display());
    editor::open(path, &config).context("Failed to open editor")?;

    Ok(())
}

/// Check if an item matches the search query.
fn matches_query(item: &Item, query: &str, full_text: bool) -> bool {
    // Always search title
    if item.title().to_lowercase().contains(query) {
        return true;
    }

    // Always search ID
    if item.id().to_lowercase().contains(query) {
        return true;
    }

    // Optionally search body
    if full_text && item.body.to_lowercase().contains(query) {
        return true;
    }

    false
}

/// Show interactive selection dialog.
fn interactive_select(items: &[Item]) -> Result<usize> {
    let options: Vec<String> = items
        .iter()
        .map(|item| format!("{} - {}", item.id(), item.title()))
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select an item")
        .items(&options)
        .default(0)
        .interact()
        .context("Selection cancelled")?;

    Ok(selection)
}
