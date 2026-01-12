//! # Attachments Command
//!
//! Lists attachments for a specific item.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::{config::Config, storage};

/// Arguments for the attachments command
pub struct AttachmentsArgs {
    pub id: String,
}

/// Executes the attachments command.
pub fn execute(args: &AttachmentsArgs) -> Result<()> {
    let config = Config::load()?;

    // Find and load the item
    let storage::LoadedItem { item, .. } = storage::find_and_load(&config, &args.id)?;

    let attachments = item.attachments();

    if attachments.is_empty() {
        println!("{}", "No attachments.".dimmed());
        return Ok(());
    }

    // Print attachments one per line
    for attachment in attachments {
        println!("{attachment}");
    }

    Ok(())
}
