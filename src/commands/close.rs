//! # Close/Reopen Commands
//!
//! Closes or reopens qstack items, moving them to/from the archive.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::{
    config::Config,
    item::{Item, Status},
    storage,
};

/// Executes the close command.
pub fn execute_close(id: &str) -> Result<()> {
    let config = Config::load()?;

    // Find the item
    let path = storage::find_by_id(&config, id)?;
    let mut item = Item::load(&path)?;

    // Check if already closed
    if item.status() == Status::Closed {
        anyhow::bail!("Item '{}' is already closed", item.id());
    }

    // Update status
    item.set_status(Status::Closed);
    item.save(&path)?;

    // Move to archive
    let (new_path, warnings) = storage::archive_item(&config, &path)?;

    // Print any attachment move warnings
    for warning in warnings {
        eprintln!("{} {}", "warning:".yellow(), warning);
    }

    println!(
        "{} Closed item: {}",
        "✓".green(),
        config.relative_path(&new_path).display()
    );

    Ok(())
}

/// Executes the reopen command.
pub fn execute_reopen(id: &str) -> Result<()> {
    let config = Config::load()?;

    // Find the item (likely in archive)
    let path = storage::find_by_id(&config, id)?;
    let mut item = Item::load(&path)?;

    // Check if already open
    if item.status() == Status::Open {
        anyhow::bail!("Item '{}' is already open", item.id());
    }

    // Update status
    item.set_status(Status::Open);
    item.save(&path)?;

    // Move back from archive to original category (or root)
    let (new_path, warnings) = storage::unarchive_item(&config, &path, item.category())?;

    // Print any attachment move warnings
    for warning in warnings {
        eprintln!("{} {}", "warning:".yellow(), warning);
    }

    println!(
        "{} Reopened item: {}",
        "✓".green(),
        config.relative_path(&new_path).display()
    );

    Ok(())
}
