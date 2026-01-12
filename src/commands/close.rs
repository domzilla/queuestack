//! # Close/Reopen Commands
//!
//! Closes or reopens qstack items, moving them to/from the archive.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::path::PathBuf;

use anyhow::Result;

use crate::{config::Config, item::Status, storage, ui};

/// Executes the close command.
pub fn execute_close(id: Option<String>, file: Option<PathBuf>) -> Result<()> {
    execute_status_change(id, file, StatusChange::Close)
}

/// Executes the reopen command.
pub fn execute_reopen(id: Option<String>, file: Option<PathBuf>) -> Result<()> {
    execute_status_change(id, file, StatusChange::Reopen)
}

/// Specifies the type of status change operation.
#[derive(Clone, Copy)]
enum StatusChange {
    Close,
    Reopen,
}

/// Unified implementation for close/reopen operations.
fn execute_status_change(
    id: Option<String>,
    file: Option<PathBuf>,
    operation: StatusChange,
) -> Result<()> {
    let config = Config::load()?;

    // Resolve item from --id or --file
    let item_ref = storage::ItemRef::from_options(id, file)?;
    let storage::LoadedItem { path, mut item } = item_ref.resolve(&config)?;

    // Determine operation parameters
    let (target_status, verb, state_name) = match operation {
        StatusChange::Close => (Status::Closed, "Closed", "closed"),
        StatusChange::Reopen => (Status::Open, "Reopened", "open"),
    };

    // Check if already in target state
    if item.status() == target_status {
        anyhow::bail!("Item '{}' is already {}", item.id(), state_name);
    }

    // Update status and save
    item.set_status(target_status);
    item.save(&path)?;

    // Move to/from archive
    let (new_path, warnings) = match operation {
        StatusChange::Close => storage::archive_item(&config, &path)?,
        StatusChange::Reopen => storage::unarchive_item(&config, &path, item.category())?,
    };

    // Print any attachment move warnings
    ui::print_warnings(&warnings);

    // Print success message
    ui::print_success(verb, &config, &new_path);

    Ok(())
}
