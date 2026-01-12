//! # Update Command
//!
//! Updates an existing qstack item.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::path::PathBuf;

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::{config::Config, storage, ui};

/// Arguments for the update command
pub struct UpdateArgs {
    pub id: Option<String>,
    pub file: Option<PathBuf>,
    pub title: Option<String>,
    pub labels: Vec<String>,
    pub category: Option<String>,
    pub clear_category: bool,
}

/// Executes the update command.
pub fn execute(args: UpdateArgs) -> Result<()> {
    let config = Config::load()?;

    // Resolve item from --id or --file
    let item_ref = storage::ItemRef::from_options(args.id, args.file)?;
    let storage::LoadedItem { mut path, mut item } = item_ref.resolve(&config)?;

    let mut changed = false;
    let old_filename = item.filename();

    // Update title
    if let Some(new_title) = args.title {
        if new_title != item.title() {
            item.set_title(new_title);
            changed = true;
        }
    }

    // Add labels
    for label in args.labels {
        item.add_label(label);
        changed = true;
    }

    // Update category
    if args.clear_category {
        if item.category().is_some() {
            item.set_category(None);
            changed = true;
        }
    } else if let Some(ref new_category) = args.category {
        if item.category() != Some(new_category.as_str()) {
            item.set_category(Some(new_category.clone()));
            changed = true;
        }
    }

    if !changed {
        println!("{}", "No changes to apply.".dimmed());
        return Ok(());
    }

    // Save updated frontmatter
    item.save(&path)?;

    // Handle filename change (title changed)
    let new_filename = item.filename();
    if old_filename != new_filename {
        path = storage::rename_item(&path, &new_filename)?;
    }

    // Handle category change (move to different directory)
    if args.clear_category || args.category.is_some() {
        let category = if args.clear_category {
            None
        } else {
            args.category.as_deref()
        };
        let (new_path, warnings) = storage::move_to_category(&config, &path, category)?;
        path = new_path;

        // Print any attachment move warnings
        ui::print_warnings(&warnings);
    }

    ui::print_success("Updated", &config, &path);

    Ok(())
}
