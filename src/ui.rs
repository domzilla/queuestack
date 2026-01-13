//! # UI Utilities
//!
//! Shared user interface utilities for interactive dialogs, list formatting,
//! and common UI patterns used across commands.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::io::IsTerminal;

use anyhow::{Context, Result};

use std::path::Path;

use owo_colors::OwoColorize;
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    constants::{
        UI_COL_ID_WIDTH, UI_COL_STATUS_WIDTH, UI_LABELS_TRUNCATE_LEN, UI_TITLE_TRUNCATE_LEN,
    },
    editor,
    item::{Item, Status},
    storage::{self, AttachmentResult},
    tui::screens::{
        confirm as tui_confirm, select_from_list as tui_select,
        select_from_list_filtered as tui_select_filtered, select_from_list_with_header,
        select_item_with_actions as tui_select_item_with_actions, ItemAction,
    },
};

// Re-export ItemAction for commands
pub use crate::tui::screens::ItemAction as ItemActionKind;

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
    /// Resolves interactive mode from flags and config default.
    ///
    /// Priority: explicit `--interactive` > explicit `--no-interactive` > config default
    ///
    /// Use this when you need fine-grained control (e.g., different behavior for
    /// terminal vs. non-terminal). For the common case, use `should_run()` instead.
    pub const fn resolve(&self, config_default: bool) -> bool {
        if self.interactive {
            true
        } else if self.no_interactive {
            false
        } else {
            config_default
        }
    }

    /// Returns whether interactive mode is enabled based on flags and config.
    ///
    /// Equivalent to `resolve(config.interactive())`. Use this when you only need
    /// the resolved boolean without a terminal check.
    pub fn is_enabled(&self, config: &Config) -> bool {
        self.resolve(config.interactive())
    }

    /// Checks if interactive mode should run (enabled AND in a terminal).
    ///
    /// Use this for TUI/interactive features that require a terminal.
    pub fn should_run(&self, config: &Config) -> bool {
        self.is_enabled(config) && std::io::stdout().is_terminal()
    }
}

// =============================================================================
// Interactive Selection
// =============================================================================

/// Generic interactive selection dialog.
///
/// Returns `Some(index)` if an item was selected, `None` if cancelled.
pub fn select_from_list<T: ToString>(prompt: &str, options: &[T]) -> Result<Option<usize>> {
    tui_select(prompt, options)
}

/// Interactive selection with some items disabled.
///
/// Shows all options but only allows selecting items at `selectable_indices`.
/// Disabled items are shown dimmed and cannot be navigated to.
/// Returns `Some(index)` if an item was selected, `None` if cancelled.
pub fn select_from_list_filtered<T: ToString>(
    prompt: &str,
    options: &[T],
    selectable_indices: &[usize],
) -> Result<Option<usize>> {
    tui_select_filtered(prompt, options, selectable_indices)
}

/// Interactive selection for items - returns index.
///
/// Formats items as columns: ID | Status | Title | Labels | Category
/// Works with both `&[Item]` and `&[&Item]` via `AsRef<Item>`.
/// Returns `Some(index)` if an item was selected, `None` if cancelled.
pub fn select_item<T: AsRef<Item>>(
    prompt: &str,
    items: &[T],
    config: &Config,
) -> Result<Option<usize>> {
    let header = format!(
        "{:<id_w$} {:>status_w$}  {:<title_w$}  {:<labels_w$}  {}",
        "ID",
        "Status",
        "Title",
        "Labels",
        "Category",
        id_w = UI_COL_ID_WIDTH,
        status_w = UI_COL_STATUS_WIDTH,
        title_w = UI_TITLE_TRUNCATE_LEN,
        labels_w = UI_LABELS_TRUNCATE_LEN,
    );

    let options: Vec<String> = items
        .iter()
        .map(|item| {
            let item = item.as_ref();
            let status = match item.status() {
                Status::Open => "open",
                Status::Closed => "closed",
            };
            let labels = truncate(&item.labels().join(", "), UI_LABELS_TRUNCATE_LEN);
            let category_opt = item
                .path
                .as_ref()
                .and_then(|p| storage::derive_category(config, p));
            let category = category_opt.as_deref().unwrap_or("");
            let title = truncate(item.title(), UI_TITLE_TRUNCATE_LEN);
            // Use display-width-aware padding for proper alignment with CJK/emoji
            format!(
                "{:<id_w$} {:>status_w$}  {}  {}  {}",
                item.id(),
                status,
                pad_to_width(&title, UI_TITLE_TRUNCATE_LEN),
                pad_to_width(&labels, UI_LABELS_TRUNCATE_LEN),
                category,
                id_w = UI_COL_ID_WIDTH,
                status_w = UI_COL_STATUS_WIDTH,
            )
        })
        .collect();

    select_from_list_with_header(prompt, &header, &options)
}

/// Interactive item selection with action popup.
///
/// Shows items in a list and when an item is selected, shows a popup menu
/// with actions (View, Edit, Close/Reopen, Delete).
/// Returns the selected action, or `Ok(None)` if cancelled.
pub fn select_item_with_actions<T: AsRef<Item>>(
    prompt: &str,
    items: &[T],
    config: &Config,
    available_labels: Vec<String>,
    available_categories: Vec<String>,
) -> Result<Option<ItemAction>> {
    tui_select_item_with_actions(
        prompt,
        items,
        config,
        available_labels,
        available_categories,
    )
}

/// Show a confirmation dialog.
///
/// Returns `Ok(Some(true))` if confirmed, `Ok(Some(false))` if declined,
/// or `Ok(None)` if cancelled (Esc/Ctrl+C).
pub fn confirm(message: &str) -> Result<Option<bool>> {
    tui_confirm(message)
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

/// Truncates a string to the specified maximum display width, adding ellipsis if truncated.
///
/// Uses Unicode display width (accounts for wide CJK characters and emojis)
/// rather than character count for proper terminal column alignment.
pub fn truncate(s: &str, max_width: usize) -> String {
    use unicode_width::UnicodeWidthChar;

    let display_width = s.width();
    if display_width <= max_width {
        s.to_string()
    } else {
        // Find the position where we need to truncate (leaving room for ellipsis)
        let target_width = max_width.saturating_sub(1); // Reserve 1 column for '…'
        let mut current_width = 0;
        let mut byte_index = 0;

        for (i, c) in s.char_indices() {
            let char_width = c.width().unwrap_or(0);
            if current_width + char_width > target_width {
                byte_index = i;
                break;
            }
            current_width += char_width;
            byte_index = i + c.len_utf8();
        }

        format!("{}…", &s[..byte_index])
    }
}

/// Pads a string to the specified display width using spaces.
///
/// Uses Unicode display width (accounts for wide CJK characters and emojis)
/// rather than character count for proper terminal column alignment.
pub fn pad_to_width(s: &str, width: usize) -> String {
    let display_width = s.width();
    if display_width >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - display_width))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Truncate Tests (display width based)
    // ==========================================================================

    #[test]
    fn test_truncate_ascii() {
        // ASCII chars are 1 display column each
        assert_eq!(truncate("hello", 10), "hello"); // 5 cols ≤ 10
        assert_eq!(truncate("hello world", 5), "hell…"); // 4 cols + ellipsis
        assert_eq!(truncate("abc", 3), "abc"); // 3 cols ≤ 3
        assert_eq!(truncate("abcd", 3), "ab…"); // 2 cols + ellipsis
    }

    #[test]
    fn test_truncate_cjk() {
        // CJK characters are 2 display columns each
        // "日本語" = 6 columns, "日本語中文" = 10 columns
        assert_eq!(truncate("日本語", 6), "日本語"); // 6 cols ≤ 6
        assert_eq!(truncate("日本語", 5), "日本…"); // 4 cols + ellipsis = 5
        assert_eq!(truncate("日本語中文", 5), "日本…"); // 4 cols + ellipsis = 5
        assert_eq!(truncate("日本語中文", 4), "日…"); // 2 cols + ellipsis = 3 (can't fit 4)
    }

    #[test]
    fn test_truncate_mixed_width() {
        // "Test: 日本語" = 6 (ASCII) + 6 (CJK) = 12 display columns
        assert_eq!(truncate("Test: 日本語", 12), "Test: 日本語"); // Exact fit
        assert_eq!(truncate("Test: 日本語", 11), "Test: 日本…"); // 10 cols + ellipsis
        assert_eq!(truncate("Test: 日本語", 9), "Test: 日…"); // 8 cols + ellipsis

        // "über" = 4 columns (ü is 1 column)
        assert_eq!(truncate("über", 4), "über");
        assert_eq!(truncate("über", 3), "üb…");
    }

    #[test]
    fn test_truncate_edge_cases() {
        assert_eq!(truncate("", 5), "");
        assert_eq!(truncate("a", 1), "a");
        // CJK char is 2 cols, can't fit in 1 col, so truncate to just ellipsis
        assert_eq!(truncate("日", 1), "…");
        assert_eq!(truncate("日", 2), "日"); // Exact fit
        assert_eq!(truncate("日", 3), "日"); // 2 cols ≤ 3
    }

    // ==========================================================================
    // Pad to Width Tests
    // ==========================================================================

    #[test]
    fn test_pad_to_width_ascii() {
        assert_eq!(pad_to_width("hello", 10), "hello     ");
        assert_eq!(pad_to_width("hello", 5), "hello");
        assert_eq!(pad_to_width("hello", 3), "hello"); // No truncation, just returns as-is
    }

    #[test]
    fn test_pad_to_width_cjk() {
        // "日本" = 4 display columns
        assert_eq!(pad_to_width("日本", 6), "日本  "); // 4 cols + 2 spaces = 6
        assert_eq!(pad_to_width("日本", 4), "日本"); // Exact fit
    }

    #[test]
    fn test_pad_to_width_mixed() {
        // "a日b" = 1 + 2 + 1 = 4 display columns
        assert_eq!(pad_to_width("a日b", 6), "a日b  ");
    }
}
