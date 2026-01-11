//! # Attachments Command
//!
//! Lists attachments for a specific item.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use anyhow::Result;
use comfy_table::{presets::UTF8_FULL_CONDENSED, ContentArrangement, Table};
use owo_colors::OwoColorize;

use crate::{
    config::Config,
    item::{is_url, Item},
    storage,
};

/// Arguments for the attachments command
pub struct AttachmentsArgs {
    pub id: String,
}

/// Executes the attachments command.
pub fn execute(args: &AttachmentsArgs) -> Result<()> {
    let config = Config::load()?;

    // Find the item by ID
    let path = storage::find_by_id(&config, &args.id)?;
    let item = Item::load(&path)?;

    let attachments = item.attachments();

    if attachments.is_empty() {
        println!("{}", "No attachments.".dimmed());
        return Ok(());
    }

    // Build and print table
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["#", "Type", "Attachment"]);

    for (i, attachment) in attachments.iter().enumerate() {
        let type_str = if is_url(attachment) { "url" } else { "file" };
        table.add_row(vec![&(i + 1).to_string(), type_str, attachment]);
    }

    println!("{table}");

    Ok(())
}
