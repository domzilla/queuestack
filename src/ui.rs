//! # UI Utilities
//!
//! Shared user interface utilities for interactive dialogs, table formatting,
//! and common UI patterns used across commands.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::io::IsTerminal;

use anyhow::{Context, Result};

use std::path::Path;

use owo_colors::OwoColorize;

use crate::{
    config::Config,
    constants::{UI_LABELS_TRUNCATE_LEN, UI_TITLE_TRUNCATE_LEN},
    editor,
    item::{Item, Status},
    storage::AttachmentResult,
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
/// Formats items as columns: ID | Status | Title | Labels | Category
/// Works with both `&[Item]` and `&[&Item]` via `AsRef<Item>`.
pub fn select_item<T: AsRef<Item>>(prompt: &str, items: &[T]) -> Result<usize> {
    let options: Vec<String> = items
        .iter()
        .map(|item| {
            let item = item.as_ref();
            let status = match item.status() {
                Status::Open => "open",
                Status::Closed => "closed",
            };
            let labels = truncate(&item.labels().join(", "), UI_LABELS_TRUNCATE_LEN);
            let category = item.category().unwrap_or("-");
            let title = truncate(item.title(), UI_TITLE_TRUNCATE_LEN);
            format!(
                "{:<15} {:>6}  {:<40}  {:<20}  {}",
                item.id(),
                status,
                title,
                labels,
                category
            )
        })
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
// Success Messages
// =============================================================================

/// Prints a success message with an item path.
///
/// Format: `✓ {verb} item: {relative_path}`
pub fn print_success(verb: &str, config: &Config, path: &Path) {
    println!(
        "{} {} item: {}",
        "✓".green(),
        verb,
        config.relative_path(path).display()
    );
}

/// Prints warnings with yellow prefix.
pub fn print_warnings(warnings: &[String]) {
    for warning in warnings {
        eprintln!("{} {}", "warning:".yellow(), warning);
    }
}

// =============================================================================
// Attachment Processing
// =============================================================================

/// Processes attachments and prints results.
///
/// This is a shared utility for `new` and `attach` commands that handles:
/// - Setting up the item's attachment directory
/// - Processing each attachment source
/// - Printing colored output for each result
/// - Saving the updated item
///
/// Returns the number of successfully added attachments.
pub fn process_and_save_attachments(
    item: &mut Item,
    path: &Path,
    sources: &[String],
) -> Result<usize> {
    use crate::storage;

    // Set path so attachment_dir() works
    item.path = Some(path.to_path_buf());

    let item_dir = item
        .attachment_dir()
        .ok_or_else(|| anyhow::anyhow!("Invalid item path"))?
        .to_path_buf();
    let item_id = item.id().to_string();

    let mut added_count = 0;

    for source in sources {
        match storage::process_attachment(source, item, &item_dir, &item_id)? {
            AttachmentResult::UrlAdded(url) => {
                println!("  {} {}", "+".green(), url);
                added_count += 1;
            }
            AttachmentResult::FileCopied { original, new_name } => {
                println!("  {} {} -> {}", "+".green(), original, new_name);
                added_count += 1;
            }
            AttachmentResult::FileNotFound(p) => {
                eprintln!("  {} File not found: {}", "!".yellow(), p);
            }
        }
    }

    // Save updated item with attachments
    item.save(path)?;

    Ok(added_count)
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
