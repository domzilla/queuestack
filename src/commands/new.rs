//! # New Command
//!
//! Creates a new qstack item with the given title.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;
use owo_colors::OwoColorize;

use crate::{
    config::Config,
    editor, id,
    item::{is_url, Frontmatter, Item, Status},
    storage,
};

/// Arguments for the new command
pub struct NewArgs {
    pub title: String,
    pub labels: Vec<String>,
    pub category: Option<String>,
    pub attachments: Vec<String>,
    pub interactive: bool,
    pub no_interactive: bool,
}

/// Executes the new command.
pub fn execute(args: NewArgs) -> Result<()> {
    let mut config = Config::load()?;

    // Get author name (prompts if not available)
    let author = config.user_name_or_prompt()?;

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
        attachments: vec![],
    };

    // Create item
    let mut item = Item::new(frontmatter);

    // Save to disk
    let path = storage::create_item(&config, &item)?;

    // Process attachments if any
    if !args.attachments.is_empty() {
        let item_dir = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid item path"))?;
        let item_id = item.id().to_string();

        for source in &args.attachments {
            if is_url(source) {
                item.add_attachment(source.clone());
                println!("  {} {}", "+".green(), source);
            } else {
                let source_path = Path::new(source);
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
                } else {
                    eprintln!(
                        "  {} File not found: {}",
                        "!".yellow(),
                        source_path.display()
                    );
                }
            }
        }

        // Save updated item with attachments
        item.save(&path)?;
    }

    // Resolve interactive mode: flags override config
    let interactive = if args.interactive {
        true
    } else if args.no_interactive {
        false
    } else {
        config.interactive()
    };

    // Open editor if interactive
    if interactive {
        editor::open(&path, &config).context("Failed to open editor")?;
    }

    // Output the path (for scripting)
    println!("{}", config.relative_path(&path).display());

    Ok(())
}
