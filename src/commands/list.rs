//! # List Command
//!
//! Lists qstack items with filtering and sorting options.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::cmp::Reverse;

use anyhow::Result;
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, ContentArrangement, Table};
use owo_colors::OwoColorize;

use crate::{
    config::Config,
    item::{Item, Status},
    storage,
};

/// Sort order for listing
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum SortBy {
    #[default]
    Id,
    Date,
    Title,
}

/// Filter options for listing
pub struct ListFilter {
    pub open: bool,
    pub closed: bool,
    pub id: Option<String>,
    pub label: Option<String>,
    pub author: Option<String>,
    pub sort: SortBy,
}

impl Default for ListFilter {
    fn default() -> Self {
        Self {
            open: false,
            closed: false,
            id: None,
            label: None,
            author: None,
            sort: SortBy::Id,
        }
    }
}

/// Executes the list command.
pub fn execute(filter: &ListFilter) -> Result<()> {
    let config = Config::load()?;

    // Collect items based on status filter
    let paths: Vec<_> = if filter.closed {
        storage::walk_archived(&config).collect()
    } else if filter.open {
        storage::walk_items(&config).collect()
    } else {
        // Default: show open items only
        storage::walk_items(&config).collect()
    };

    // Load and filter items
    let mut items: Vec<Item> = paths
        .into_iter()
        .filter_map(|path| Item::load(&path).ok())
        .filter(|item| apply_filters(item, filter))
        .collect();

    // Handle single item detail view
    if let Some(ref partial_id) = filter.id {
        let path = storage::find_by_id(&config, partial_id)?;
        let item = Item::load(&path)?;
        print_item_detail(&item, &config);
        return Ok(());
    }

    // Sort items
    sort_items(&mut items, filter.sort);

    // Display
    if items.is_empty() {
        println!("{}", "No items found.".dimmed());
        return Ok(());
    }

    print_table(&items);

    Ok(())
}

fn apply_filters(item: &Item, filter: &ListFilter) -> bool {
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

fn sort_items(items: &mut [Item], sort: SortBy) {
    match sort {
        SortBy::Id => items.sort_by(|a, b| a.id().cmp(b.id())),
        SortBy::Date => items.sort_by_key(|item| Reverse(item.created_at())),
        SortBy::Title => items.sort_by_key(|item| item.title().to_lowercase()),
    }
}

fn print_table(items: &[Item]) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["ID", "Status", "Title", "Labels", "Category"]);

    for item in items {
        let status_cell = match item.status() {
            Status::Open => Cell::new("open").fg(Color::Green),
            Status::Closed => Cell::new("closed").fg(Color::Red),
        };

        let labels = item.labels().join(", ");
        let category = item.category().unwrap_or("-");

        // Truncate ID to first part for display
        let short_id = item.id().split('-').next().unwrap_or_else(|| item.id());

        table.add_row(vec![
            Cell::new(short_id),
            status_cell,
            Cell::new(truncate(item.title(), 40)),
            Cell::new(truncate(&labels, 20)),
            Cell::new(category),
        ]);
    }

    println!("{table}");
}

fn print_item_detail(item: &Item, config: &Config) {
    println!("{}: {}", "ID".bold(), item.id());
    println!("{}: {}", "Title".bold(), item.title());
    println!("{}: {}", "Author".bold(), item.author());
    println!(
        "{}: {}",
        "Created".bold(),
        item.created_at().format("%Y-%m-%d %H:%M:%S UTC")
    );

    let status = match item.status() {
        Status::Open => "open".green().to_string(),
        Status::Closed => "closed".red().to_string(),
    };
    println!("{}: {}", "Status".bold(), status);

    if !item.labels().is_empty() {
        println!("{}: {}", "Labels".bold(), item.labels().join(", "));
    }

    if let Some(category) = item.category() {
        println!("{}: {}", "Category".bold(), category);
    }

    if let Some(ref path) = item.path {
        println!(
            "{}: {}",
            "File".bold(),
            config.relative_path(path).display()
        );
    }

    if !item.body.is_empty() {
        println!("\n{}", "---".dimmed());
        println!("{}", item.body);
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
