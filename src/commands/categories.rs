//! # Categories Command
//!
//! Lists all unique categories used across items.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::{config::Config, item::Item, storage, ui, ui::InteractiveArgs};

/// Arguments for the categories command
pub struct CategoriesArgs {
    pub interactive: InteractiveArgs,
}

/// Executes the categories command.
pub fn execute(args: &CategoriesArgs) -> Result<()> {
    let config = Config::load()?;

    // Collect all items and count categories
    let items = storage::load_all_items(&config);
    let category_counts = ui::count_by(&items, |item: &Item| item.category().map(String::from));

    if category_counts.is_empty() {
        println!("{}", "No items found.".dimmed());
        return Ok(());
    }

    // Sort by count (descending), then alphabetically (None last)
    let mut categories: Vec<_> = category_counts.into_iter().collect();
    categories.sort_by(|a, b| {
        b.1.cmp(&a.1).then_with(|| match (&a.0, &b.0) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(_), None) => std::cmp::Ordering::Less,
            (Some(a), Some(b)) => a.cmp(b),
        })
    });

    // Check interactive mode
    if !args.interactive.should_run(&config) {
        // Non-interactive: print categories one per line
        for (category, _) in &categories {
            let name = category.as_deref().unwrap_or("(uncategorized)");
            println!("{name}");
        }
        return Ok(());
    }

    // Interactive selection
    let options: Vec<String> = categories
        .iter()
        .map(|(cat, count)| {
            let name = cat.as_deref().unwrap_or("(uncategorized)");
            format!("{name} ({count})")
        })
        .collect();

    let selection = ui::select_from_list("Select a category to filter by", &options)?;
    let selected_category = &categories[selection].0;

    // Filter items in selected category
    let filtered: Vec<&Item> = items
        .iter()
        .filter(|item| item.category().map(String::from) == *selected_category)
        .collect();

    if filtered.is_empty() {
        println!("{}", "No items found.".dimmed());
        return Ok(());
    }

    // Interactive: TUI selection for items
    let item_selection = ui::select_item("Select an item to open", &filtered)?;
    let item = filtered[item_selection];
    ui::open_item_in_editor(item, &config)?;

    Ok(())
}
