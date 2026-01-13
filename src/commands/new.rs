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
    item::{normalize_identifier, Frontmatter, Item, Status},
    storage,
    tui::{self, screens::NewItemWizard},
    ui::{self, InteractiveArgs},
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

    // Validate title is not empty
    if title.trim().is_empty() {
        anyhow::bail!("Title cannot be empty");
    }

    // Validate labels are not empty
    for label in &args.labels {
        if label.trim().is_empty() {
            anyhow::bail!("Label cannot be empty");
        }
    }

    // Validate category is not empty
    if let Some(ref cat) = args.category {
        if cat.trim().is_empty() {
            anyhow::bail!("Category cannot be empty");
        }
    }

    // Get author name (prompts if not available)
    let author = config.user_name_or_prompt()?;

    // Generate ID
    let id = id::generate(config.id_pattern());

    // Normalize labels and category (spaces -> hyphens)
    let labels: Vec<String> = args
        .labels
        .iter()
        .map(|l| normalize_identifier(l))
        .collect();
    let category = args.category.as_deref().map(normalize_identifier);

    // Create frontmatter
    let frontmatter = Frontmatter {
        id,
        title,
        author,
        created_at: Utc::now(),
        status: Status::Open,
        labels,
        attachments: vec![],
    };

    // Create item
    let mut item = Item::new(frontmatter);

    // Save to disk (category determines folder placement)
    let path = storage::create_item(&config, &item, category.as_deref())?;

    // Process attachments if any
    if !args.attachments.is_empty() {
        ui::process_and_save_attachments(&mut item, &path, &args.attachments)?;
    }

    // Resolve interactive mode (editor doesn't require terminal check)
    let interactive = args.interactive.is_enabled(&config);

    // Open editor if interactive
    if interactive {
        editor::open(&path, &config).context("Failed to open editor")?;
    }

    // Output the path (for scripting)
    println!("{}", config.relative_path(&path).display());

    Ok(())
}

/// Collect existing categories and labels from all items.
pub fn collect_existing_metadata(config: &Config) -> (Vec<String>, Vec<String>) {
    let mut categories: HashSet<String> = HashSet::new();
    let mut labels: HashSet<String> = HashSet::new();

    let paths: Vec<_> = storage::walk_all(config).collect();

    for path in paths {
        if let Ok(item) = Item::load(&path) {
            // Derive category from path
            if let Some(cat) = storage::derive_category(config, &path) {
                categories.insert(cat);
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

    // Normalize labels and category (spaces -> hyphens)
    let labels: Vec<String> = output
        .labels
        .iter()
        .map(|l| normalize_identifier(l))
        .collect();
    let category = output.category.as_deref().map(normalize_identifier);

    // Create frontmatter from wizard output
    let frontmatter = Frontmatter {
        id,
        title: output.title,
        author,
        created_at: Utc::now(),
        status: Status::Open,
        labels,
        attachments: vec![],
    };

    // Create item with content
    let mut item = Item::new(frontmatter);
    item.body = output.content;

    // Save to disk (category determines folder placement)
    let path = storage::create_item(&config, &item, category.as_deref())?;

    // Process attachments
    if !output.attachments.is_empty() {
        ui::process_and_save_attachments(&mut item, &path, &output.attachments)?;
    }

    // Output the path
    println!("{}", config.relative_path(&path).display());

    Ok(())
}
