//! # Labels Command
//!
//! Lists all unique labels used across items.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::{config::Config, item::Item, storage, ui, ui::InteractiveArgs};

/// Arguments for the labels command
pub struct LabelsArgs {
    pub interactive: InteractiveArgs,
}

/// Executes the labels command.
pub fn execute(args: &LabelsArgs) -> Result<()> {
    let config = Config::load()?;

    // Collect all items and count labels
    let items = storage::load_all_items(&config);
    let label_counts = ui::count_by_many(&items, |item: &Item| item.labels().to_vec());

    if label_counts.is_empty() {
        println!("{}", "No labels found.".dimmed());
        return Ok(());
    }

    // Sort by count (descending), then alphabetically
    let mut labels: Vec<_> = label_counts.into_iter().collect();
    labels.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    // Check interactive mode
    if !args.interactive.should_run(&config) {
        // Non-interactive: print labels one per line
        for (label, _) in &labels {
            println!("{label}");
        }
        return Ok(());
    }

    // Interactive selection
    let options: Vec<String> = labels
        .iter()
        .map(|(label, count)| format!("{label} ({count})"))
        .collect();

    let selection = ui::select_from_list("Select a label to filter by", &options)?;
    let selected_label = &labels[selection].0;

    // Filter items with selected label
    let filtered: Vec<&Item> = items
        .iter()
        .filter(|item| item.labels().iter().any(|l| l == selected_label))
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
