//! # UI Utilities
//!
//! Shared user interface utilities for interactive dialogs, table formatting,
//! and common UI patterns used across commands.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::io::IsTerminal;

use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, ContentArrangement, Table};

use crate::{
    config::Config,
    constants::{UI_LABELS_TRUNCATE_LEN, UI_TITLE_TRUNCATE_LEN},
    editor, id,
    item::{Item, Status},
    tui::screens::select_from_list as tui_select,
};

// =============================================================================
// Aggregation Utilities
// =============================================================================

use std::collections::HashMap;
use std::hash::Hash;

/// Counts occurrences by a single key extracted from each item.
///
/// For items that map to exactly one key (e.g., category).
pub fn count_by<T, K, F>(items: &[T], key_fn: F) -> HashMap<K, usize>
where
    K: Eq + Hash,
    F: Fn(&T) -> K,
{
    let mut counts = HashMap::new();
    for item in items {
        *counts.entry(key_fn(item)).or_insert(0) += 1;
    }
    counts
}

/// Counts occurrences by multiple keys extracted from each item.
///
/// For items that map to multiple keys (e.g., labels).
pub fn count_by_many<T, K, I, F>(items: &[T], keys_fn: F) -> HashMap<K, usize>
where
    K: Eq + Hash,
    I: IntoIterator<Item = K>,
    F: Fn(&T) -> I,
{
    let mut counts = HashMap::new();
    for item in items {
        for key in keys_fn(item) {
            *counts.entry(key).or_insert(0) += 1;
        }
    }
    counts
}

// =============================================================================
// Interactive Mode Resolution
// =============================================================================

/// Common interactive mode flags used across commands.
///
/// Consolidates the `--interactive` / `--no-interactive` flag pattern.
#[derive(Debug, Clone, Copy, Default)]
pub struct InteractiveArgs {
    /// Force interactive mode
    pub interactive: bool,
    /// Force non-interactive mode
    pub no_interactive: bool,
}

impl InteractiveArgs {
    /// Resolves interactive mode from flags and config.
    ///
    /// Priority: explicit `--interactive` > explicit `--no-interactive` > config default
    pub const fn resolve(&self, config_default: bool) -> bool {
        if self.interactive {
            true
        } else if self.no_interactive {
            false
        } else {
            config_default
        }
    }

    /// Checks if we should run interactive mode (combines flag resolution with terminal check).
    pub fn should_run(&self, config: &Config) -> bool {
        self.resolve(config.interactive()) && std::io::stdout().is_terminal()
    }
}

// =============================================================================
// Interactive Selection
// =============================================================================

/// Generic interactive selection dialog.
///
/// Displays a list of options and returns the index of the selected item.
pub fn select_from_list<T: ToString>(prompt: &str, options: &[T]) -> Result<usize> {
    tui_select(prompt, options)
}

/// Interactive selection for items - returns index.
///
/// Formats items as "{id} - {title}" for display.
pub fn select_item(prompt: &str, items: &[Item]) -> Result<usize> {
    let options: Vec<String> = items
        .iter()
        .map(|item| format!("{} - {}", item.id(), item.title()))
        .collect();

    select_from_list(prompt, &options)
}

/// Interactive selection for item references - returns index.
pub fn select_item_ref(prompt: &str, items: &[&Item]) -> Result<usize> {
    let options: Vec<String> = items
        .iter()
        .map(|item| format!("{} - {}", item.id(), item.title()))
        .collect();

    select_from_list(prompt, &options)
}

/// Opens an item in the editor and prints its relative path.
pub fn open_item_in_editor(item: &Item, config: &Config) -> Result<()> {
    let path = item.path.as_ref().context("Item has no path")?;
    println!("{}", config.relative_path(path).display());
    editor::open(path, config).context("Failed to open editor")
}

// =============================================================================
// String Utilities
// =============================================================================

/// Truncates a string to the specified maximum length, adding ellipsis if truncated.
pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

// =============================================================================
// Table Building
// =============================================================================

/// Creates a new table with default styling.
pub fn create_table() -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table
}

/// Prints an item table with standard columns: ID, Status, Title, Labels, Category.
pub fn print_items_table(items: &[Item]) {
    print_items_table_ref(&items.iter().collect::<Vec<_>>());
}

/// Prints an item table from references.
pub fn print_items_table_ref(items: &[&Item]) {
    let mut table = create_table();
    table.set_header(vec!["ID", "Status", "Title", "Labels", "Category"]);

    for item in items {
        let status_cell = status_cell(item.status());
        let labels = item.labels().join(", ");
        let category = item.category().unwrap_or("-");
        let short_id = id::short_form(item.id());

        table.add_row(vec![
            Cell::new(short_id),
            status_cell,
            Cell::new(truncate(item.title(), UI_TITLE_TRUNCATE_LEN)),
            Cell::new(truncate(&labels, UI_LABELS_TRUNCATE_LEN)),
            Cell::new(category),
        ]);
    }

    println!("{table}");
}

/// Prints a compact item table without the category column.
pub fn print_items_table_compact(items: &[&Item]) {
    let mut table = create_table();
    table.set_header(vec!["ID", "Status", "Title", "Labels"]);

    for item in items {
        let status_cell = status_cell(item.status());
        let labels = item.labels().join(", ");
        let short_id = id::short_form(item.id());

        table.add_row(vec![
            Cell::new(short_id),
            status_cell,
            Cell::new(truncate(item.title(), UI_TITLE_TRUNCATE_LEN)),
            Cell::new(truncate(&labels, UI_LABELS_TRUNCATE_LEN)),
        ]);
    }

    println!("{table}");
}

/// Creates a colored status cell.
fn status_cell(status: Status) -> Cell {
    match status {
        Status::Open => Cell::new("open").fg(Color::Green),
        Status::Closed => Cell::new("closed").fg(Color::Red),
    }
}

/// Extracts the short ID for display.
///
/// Re-export of `id::short_form` for convenience.
pub fn short_id(id: &str) -> &str {
    id::short_form(id)
}
