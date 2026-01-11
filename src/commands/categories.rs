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

    // Display table
    print_table(&categories);

    // Check interactive mode
    if !args.interactive.should_run(&config) {
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

    let category_name = selected_category.as_deref().unwrap_or("(uncategorized)");
    println!("\n{} {}\n", "Items in category:".bold(), category_name);

    // Filter and display items in selected category
    let filtered: Vec<&Item> = items
        .iter()
        .filter(|item| item.category().map(String::from) == *selected_category)
        .collect();

    if filtered.is_empty() {
        println!("{}", "No items found.".dimmed());
        return Ok(());
    }

    ui::print_items_table_compact(&filtered);

    // Second interactive selection for items (check again since we printed a new table)
    if !args.interactive.should_run(&config) {
        return Ok(());
    }

    let item_selection = ui::select_item_ref("Select an item to open", &filtered)?;
    let item = filtered[item_selection];
    ui::open_item_in_editor(item, &config)?;

    Ok(())
}

fn print_table(categories: &[(Option<String>, usize)]) {
    let mut table = ui::create_table();
    table.set_header(vec!["Category", "Count"]);

    for (category, count) in categories {
        let name = category.as_deref().unwrap_or("(uncategorized)");
        table.add_row(vec![name, &count.to_string()]);
    }

    println!("{table}");
}
