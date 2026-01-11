//! # Attach Command
//!
//! Add or remove attachments from items.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::path::Path;

use anyhow::{bail, Result};
use owo_colors::OwoColorize;

use crate::{
    config::Config,
    item::{is_url, Item},
    storage,
};

/// Arguments for the attach add subcommand
pub struct AttachAddArgs {
    pub id: String,
    pub sources: Vec<String>,
}

/// Arguments for the attach remove subcommand
pub struct AttachRemoveArgs {
    pub id: String,
    pub indices: Vec<usize>,
}

/// Executes the attach add command.
pub fn execute_add(args: &AttachAddArgs) -> Result<()> {
    if args.sources.is_empty() {
        bail!("No files or URLs specified");
    }

    let config = Config::load()?;

    // Find the item by ID
    let path = storage::find_by_id(&config, &args.id)?;
    let mut item = Item::load(&path)?;

    // Check item is not closed
    if item.status() == crate::item::Status::Closed {
        bail!("Cannot attach to a closed item. Use 'qstack reopen' first.");
    }

    // Get item directory
    let item_dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid item path"))?;

    // Clone ID to avoid borrow conflicts when mutating item
    let item_id = item.id().to_string();
    let mut added_count = 0;

    for source in &args.sources {
        let added = if is_url(source) {
            // URL attachment - just add to frontmatter
            item.add_attachment(source.clone());
            println!("  {} {}", "+".green(), source);
            true
        } else {
            // File attachment - copy to item directory
            let source_path = Path::new(source);

            // Handle relative paths
            let source_path = if source_path.is_relative() {
                std::env::current_dir()?.join(source_path)
            } else {
                source_path.to_path_buf()
            };

            if source_path.exists() {
                let counter = item.next_attachment_counter();
                let new_filename =
                    storage::copy_attachment(&source_path, item_dir, &item_id, counter)?;

                item.add_attachment(new_filename.clone());
                println!("  {} {} -> {}", "+".green(), source, new_filename);
                true
            } else {
                eprintln!(
                    "  {} File not found: {}",
                    "!".yellow(),
                    source_path.display()
                );
                false
            }
        };

        if added {
            added_count += 1;
        }
    }

    // Save updated item
    item.save(&path)?;

    println!(
        "\n{} Added {} attachment(s) to {}",
        "✓".green(),
        added_count,
        config.relative_path(&path).display()
    );

    Ok(())
}

/// Executes the attach remove command.
pub fn execute_remove(args: &AttachRemoveArgs) -> Result<()> {
    if args.indices.is_empty() {
        bail!("No attachment indices specified");
    }

    let config = Config::load()?;

    // Find the item by ID
    let path = storage::find_by_id(&config, &args.id)?;
    let mut item = Item::load(&path)?;

    let attachment_count = item.attachments().len();
    if attachment_count == 0 {
        bail!("Item has no attachments");
    }

    // Get item directory
    let item_dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid item path"))?;

    // Validate all indices first (1-based from user)
    for &idx in &args.indices {
        if idx == 0 || idx > attachment_count {
            bail!(
                "Invalid attachment index: {}. Item has {} attachment(s). Use 'qstack attachments --id {}' to see the list.",
                idx,
                attachment_count,
                args.id
            );
        }
    }

    // Sort indices in descending order to remove from end first (preserves indices)
    let mut indices: Vec<usize> = args.indices.clone();
    indices.sort_unstable();
    indices.reverse();
    indices.dedup();

    let mut removed_count = 0;

    for idx in indices {
        // Convert from 1-based to 0-based
        let idx_0 = idx - 1;

        if let Some(removed) = item.remove_attachment(idx_0) {
            // If it's a file (not URL), delete from disk
            if !is_url(&removed) {
                if let Err(e) = storage::delete_attachment(item_dir, &removed) {
                    eprintln!(
                        "  {} Failed to delete file {}: {}",
                        "!".yellow(),
                        removed,
                        e
                    );
                }
            }
            println!("  {} [{}] {}", "-".red(), idx, removed);
            removed_count += 1;
        }
    }

    // Save updated item
    item.save(&path)?;

    println!(
        "\n{} Removed {} attachment(s) from {}",
        "✓".green(),
        removed_count,
        config.relative_path(&path).display()
    );

    Ok(())
}
