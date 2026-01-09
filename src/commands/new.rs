//! # New Command
//!
//! Creates a new qstack item with the given title.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use anyhow::{Context, Result};
use chrono::Utc;

use crate::{
    config::Config,
    editor, id,
    item::{Frontmatter, Item, Status},
    storage,
};

/// Arguments for the new command
pub struct NewArgs {
    pub title: String,
    pub labels: Vec<String>,
    pub category: Option<String>,
}

/// Executes the new command.
pub fn execute(args: NewArgs) -> Result<()> {
    let config = Config::load()?;

    // Get author name
    let author = config.user_name().ok_or_else(|| {
        anyhow::anyhow!(
            "No user name configured. Set user_name in ~/.qstack or enable use_git_user"
        )
    })?;

    // Generate ID
    let id = id::generate(config.id_pattern());

    // Create frontmatter
    let frontmatter = Frontmatter {
        id,
        title: args.title,
        author,
        created_at: Utc::now(),
        status: Status::Open,
        labels: args.labels,
        category: args.category,
    };

    // Create item
    let item = Item::new(frontmatter);

    // Save to disk
    let path = storage::create_item(&config, &item)?;

    // Open editor if configured and TTY
    if config.auto_open() {
        editor::open(&path, &config).context("Failed to open editor")?;
    }

    // Output the path (for scripting)
    println!("{}", config.relative_path(&path).display());

    Ok(())
}
