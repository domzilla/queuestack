//! # New Command
//!
//! Creates a new qstack item with the given title.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::collections::HashSet;
use std::io::IsTerminal;

use anyhow::{Context, Result};
use chrono::Utc;
use owo_colors::OwoColorize;

use crate::{
    config::Config,
    editor, id,
    item::{Frontmatter, Item, Status},
    storage::{self, AttachmentResult},
    tui::{self, screens::NewItemWizard},
    ui::InteractiveArgs,
};

/// Arguments for the new command
pub struct NewArgs {
    pub title: Option<String>,
    pub labels: Vec<String>,
    pub category: Option<String>,
    pub attachments: Vec<String>,
    pub interactive: InteractiveArgs,
}

/// Executes the new command.
pub fn execute(args: NewArgs) -> Result<()> {
    let mut config = Config::load()?;

    // If no title provided and we're in a terminal, launch the wizard
    if args.title.is_none() {
        if !std::io::stdout().is_terminal() {
            anyhow::bail!("Title is required in non-interactive mode");
        }
        return execute_wizard(&config);
    }

    let title = args.title.unwrap();

    // Get author name (prompts if not available)
    let author = config.user_name_or_prompt()?;

    // Generate ID
    let id = id::generate(config.id_pattern());

    // Create frontmatter
    let frontmatter = Frontmatter {
        id,
        title,
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
        // Set path so attachment_dir() works
        item.path = Some(path.clone());
        let item_dir = item
            .attachment_dir()
            .ok_or_else(|| anyhow::anyhow!("Invalid item path"))?
            .to_path_buf();
        let item_id = item.id().to_string();

        for source in &args.attachments {
            match storage::process_attachment(source, &mut item, &item_dir, &item_id)? {
                AttachmentResult::UrlAdded(url) => {
                    println!("  {} {}", "+".green(), url);
                }
                AttachmentResult::FileCopied { original, new_name } => {
                    println!("  {} {} -> {}", "+".green(), original, new_name);
                }
                AttachmentResult::FileNotFound(path) => {
                    eprintln!("  {} File not found: {}", "!".yellow(), path);
                }
            }
        }

        // Save updated item with attachments
        item.save(&path)?;
    }

    // Resolve interactive mode
    let interactive = args.interactive.resolve(config.interactive());

    // Open editor if interactive
    if interactive {
        editor::open(&path, &config).context("Failed to open editor")?;
    }

    // Output the path (for scripting)
    println!("{}", config.relative_path(&path).display());

    Ok(())
}

/// Collect existing categories and labels from all items.
fn collect_existing_metadata(config: &Config) -> (Vec<String>, Vec<String>) {
    let mut categories: HashSet<String> = HashSet::new();
    let mut labels: HashSet<String> = HashSet::new();

    let paths: Vec<_> = storage::walk_all(config).collect();

    for path in paths {
        if let Ok(item) = Item::load(&path) {
            if let Some(cat) = item.category() {
                categories.insert(cat.to_string());
            }
            for label in item.labels() {
                labels.insert(label.clone());
            }
        }
    }

    let mut categories: Vec<_> = categories.into_iter().collect();
    let mut labels: Vec<_> = labels.into_iter().collect();
    categories.sort();
    labels.sort();

    (categories, labels)
}

/// Execute the wizard flow for creating a new item.
fn execute_wizard(config: &Config) -> Result<()> {
    // Collect existing metadata
    let (existing_categories, existing_labels) = collect_existing_metadata(config);

    // Run the wizard
    let wizard = NewItemWizard::new(existing_categories, existing_labels);
    let Some(output) = tui::run(wizard)? else {
        println!("{}", "Cancelled.".dimmed());
        return Ok(());
    };

    // Get author name
    let mut config = Config::load()?;
    let author = config.user_name_or_prompt()?;

    // Generate ID
    let id = id::generate(config.id_pattern());

    // Create frontmatter from wizard output
    let frontmatter = Frontmatter {
        id,
        title: output.title,
        author,
        created_at: Utc::now(),
        status: Status::Open,
        labels: output.labels,
        category: output.category,
        attachments: vec![],
    };

    // Create item with content
    let mut item = Item::new(frontmatter);
    item.body = output.content;

    // Save to disk
    let path = storage::create_item(&config, &item)?;

    // Process attachments
    if !output.attachments.is_empty() {
        item.path = Some(path.clone());
        let item_dir = item
            .attachment_dir()
            .ok_or_else(|| anyhow::anyhow!("Invalid item path"))?
            .to_path_buf();
        let item_id = item.id().to_string();

        for source in &output.attachments {
            match storage::process_attachment(source, &mut item, &item_dir, &item_id)? {
                AttachmentResult::UrlAdded(url) => {
                    println!("  {} {}", "+".green(), url);
                }
                AttachmentResult::FileCopied { original, new_name } => {
                    println!("  {} {} -> {}", "+".green(), original, new_name);
                }
                AttachmentResult::FileNotFound(p) => {
                    eprintln!("  {} File not found: {}", "!".yellow(), p);
                }
            }
        }

        // Save updated item with attachments
        item.save(&path)?;
    }

    // Output the path
    println!("{}", config.relative_path(&path).display());

    Ok(())
}
