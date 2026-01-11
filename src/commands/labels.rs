//! # Labels Command
//!
//! Lists all unique labels used across items.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::{config::Config, item::Item, storage, ui, ui::InteractiveArgs};

use super::{list, ListFilter, SortBy, StatusFilter};

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

    // Display table
    print_table(&labels);

    // Check interactive mode
    if !args.interactive.should_run(&config) {
        return Ok(());
    }

    // Interactive selection
    let options: Vec<String> = labels
        .iter()
        .map(|(label, count)| format!("{label} ({count})"))
        .collect();

    let selection = ui::select_from_list("Select a label to filter by", &options)?;
    let selected_label = &labels[selection].0;

    println!("\n{} {}\n", "Items with label:".bold(), selected_label);

    // Show items with selected label using list command (all statuses)
    list::execute(&ListFilter {
        status: StatusFilter::All,
        label: Some(selected_label.clone()),
        author: None,
        sort: SortBy::Id,
        interactive: args.interactive,
    })?;

    Ok(())
}

fn print_table(labels: &[(String, usize)]) {
    let mut table = ui::create_table();
    table.set_header(vec!["Label", "Count"]);

    for (label, count) in labels {
        table.add_row(vec![label.as_str(), &count.to_string()]);
    }

    println!("{table}");
}
