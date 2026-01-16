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
    item::{is_url, normalize_identifier, Frontmatter, Item, Status},
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
    pub as_template: bool,
    #[allow(clippy::option_option)]
    pub from_template: Option<Option<String>>,
}

/// Executes the new command.
pub fn execute(args: NewArgs) -> Result<()> {
    let mut config = Config::load()?;

    // Handle --from-template
    if let Some(ref template_ref) = args.from_template {
        return execute_from_template(&mut config, &args, template_ref.as_deref());
    }

    // If no title provided and we're in a terminal, launch the wizard
    if args.title.is_none() {
        if !std::io::stdout().is_terminal() {
            anyhow::bail!("Title is required in non-interactive mode");
        }
        return execute_wizard(&config, args.as_template);
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

    // Determine status based on --as-template flag
    let status = if args.as_template {
        Status::Template
    } else {
        Status::Open
    };

    // Create frontmatter
    let frontmatter = Frontmatter {
        id,
        title,
        author,
        created_at: Utc::now(),
        status,
        labels,
        attachments: vec![],
    };

    // Create item
    let mut item = Item::new(frontmatter);

    // Save to disk (category determines folder placement)
    let path = if args.as_template {
        storage::create_template(&config, &item, category.as_deref())?
    } else {
        storage::create_item(&config, &item, category.as_deref())?
    };

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
fn execute_wizard(config: &Config, as_template: bool) -> Result<()> {
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

    // Determine status based on --as-template flag
    let status = if as_template {
        Status::Template
    } else {
        Status::Open
    };

    // Create frontmatter from wizard output
    let frontmatter = Frontmatter {
        id,
        title: output.title,
        author,
        created_at: Utc::now(),
        status,
        labels,
        attachments: vec![],
    };

    // Create item with content
    let mut item = Item::new(frontmatter);
    item.body = output.content;

    // Save to disk (category determines folder placement)
    let path = if as_template {
        storage::create_template(&config, &item, category.as_deref())?
    } else {
        storage::create_item(&config, &item, category.as_deref())?
    };

    // Process attachments
    if !output.attachments.is_empty() {
        ui::process_and_save_attachments(&mut item, &path, &output.attachments)?;
    }

    // Output the path
    println!("{}", config.relative_path(&path).display());

    Ok(())
}

/// Execute the from-template flow.
///
/// If `template_ref` is `None`, shows template selection TUI.
/// Otherwise, loads the template by ID/title reference.
fn execute_from_template(
    config: &mut Config,
    args: &NewArgs,
    template_ref: Option<&str>,
) -> Result<()> {
    // Load template
    let template = if let Some(reference) = template_ref {
        // Direct reference - find by ID or title
        let template_path = storage::find_template(config, reference)?;
        Item::load(&template_path)?
    } else {
        // No reference - show template selection TUI
        if !std::io::stdout().is_terminal() {
            anyhow::bail!("Template reference required in non-interactive mode");
        }
        let Some(selected) = select_template(config)? else {
            println!("{}", "Cancelled.".dimmed());
            return Ok(());
        };
        selected
    };

    // Get template's category (may be inherited)
    let template_category = template
        .path
        .as_ref()
        .and_then(|p| storage::derive_category(config, p));

    // Use CLI category if specified, otherwise inherit from template
    let category = args
        .category
        .as_deref()
        .map(normalize_identifier)
        .or(template_category);

    // Merge labels: template labels + CLI labels
    let mut labels: Vec<String> = template.labels().to_vec();
    for label in &args.labels {
        let normalized = normalize_identifier(label);
        if !labels.contains(&normalized) {
            labels.push(normalized);
        }
    }

    // If no title provided, launch wizard with template data pre-filled
    if args.title.is_none() {
        if !std::io::stdout().is_terminal() {
            anyhow::bail!("Title is required in non-interactive mode");
        }
        return execute_wizard_from_template(config, &template, category.as_deref(), &labels);
    }

    let title = args.title.clone().unwrap();

    // Validate title is not empty
    if title.trim().is_empty() {
        anyhow::bail!("Title cannot be empty");
    }

    // Get author name
    let author = config.user_name_or_prompt()?;

    // Generate new ID
    let id = id::generate(config.id_pattern());

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

    // Create item with template's body content
    let mut item = Item::new(frontmatter);
    item.body.clone_from(&template.body);

    // Save to disk
    let path = storage::create_item(config, &item, category.as_deref())?;

    // Copy template attachments (files are copied from template dir, URLs are added directly)
    copy_template_attachments(&template, &mut item, &path)?;

    // Process CLI attachments (if any)
    if !args.attachments.is_empty() {
        ui::process_and_save_attachments(&mut item, &path, &args.attachments)?;
    }

    // Resolve interactive mode
    let interactive = args.interactive.is_enabled(config);

    // Open editor if interactive
    if interactive {
        editor::open(&path, config).context("Failed to open editor")?;
    }

    // Output the path
    println!("{}", config.relative_path(&path).display());

    Ok(())
}

/// Execute wizard flow with template data pre-filled.
fn execute_wizard_from_template(
    config: &Config,
    template: &Item,
    category: Option<&str>,
    labels: &[String],
) -> Result<()> {
    // Collect existing metadata for autocomplete
    let (existing_categories, existing_labels) = collect_existing_metadata(config);

    // Resolve template attachments to full paths for pre-population
    // URLs are kept as-is, file attachments are converted to full paths
    let template_attachments: Vec<String> = resolve_template_attachments(template);

    // Create pre-populated wizard
    let wizard = NewItemWizard::new(existing_categories, existing_labels)
        .with_title(template.title())
        .with_content(&template.body)
        .with_attachments(template_attachments)
        .with_category(category.map(String::from))
        .with_labels(labels);

    let Some(output) = tui::run(wizard)? else {
        println!("{}", "Cancelled.".dimmed());
        return Ok(());
    };

    // Get author name
    let mut config = Config::load()?;
    let author = config.user_name_or_prompt()?;

    // Generate ID
    let id = id::generate(config.id_pattern());

    // Normalize category
    let category = output.category.as_deref().map(normalize_identifier);

    // Create frontmatter
    let frontmatter = Frontmatter {
        id,
        title: output.title,
        author,
        created_at: Utc::now(),
        status: Status::Open,
        labels: output.labels,
        attachments: vec![],
    };

    // Create item
    let mut item = Item::new(frontmatter);
    item.body = output.content;

    // Save to disk
    let path = storage::create_item(&config, &item, category.as_deref())?;

    // Process attachments
    if !output.attachments.is_empty() {
        ui::process_and_save_attachments(&mut item, &path, &output.attachments)?;
    }

    // Output the path
    println!("{}", config.relative_path(&path).display());

    Ok(())
}

/// Resolves template attachments to full paths.
///
/// URLs are kept as-is. File attachments are converted to full paths
/// by combining the template's directory with the attachment filename.
fn resolve_template_attachments(template: &Item) -> Vec<String> {
    let template_dir = template
        .path
        .as_ref()
        .and_then(|p| p.parent())
        .map(std::path::Path::to_path_buf);

    template
        .attachments()
        .iter()
        .map(|attachment| {
            if is_url(attachment) {
                // URLs are kept as-is
                attachment.clone()
            } else if let Some(ref dir) = template_dir {
                // File attachments: convert to full path
                dir.join(attachment).display().to_string()
            } else {
                // No template path, keep as-is (will fail gracefully)
                attachment.clone()
            }
        })
        .collect()
}

/// Copies template attachments to a new item.
///
/// - URL attachments are added directly to the item's frontmatter
/// - File attachments are copied from the template's directory to the item's directory
fn copy_template_attachments(
    template: &Item,
    item: &mut Item,
    item_path: &std::path::Path,
) -> Result<()> {
    let template_dir = template.path.as_ref().and_then(|p| p.parent());

    let Some(template_dir) = template_dir else {
        return Ok(()); // No template path, skip attachment copying
    };

    let item_dir = item_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid item path"))?;

    let item_id = item.id().to_string();

    for attachment in template.attachments() {
        if is_url(attachment) {
            // URL: add directly to item
            item.add_attachment(attachment.clone());
            println!("  {} {}", "+".green(), attachment);
        } else {
            // File: copy from template directory
            let source_path = template_dir.join(attachment);
            if source_path.exists() {
                let counter = item.next_attachment_counter();
                let new_filename =
                    storage::copy_attachment(&source_path, item_dir, &item_id, counter)?;
                item.add_attachment(new_filename.clone());
                println!("  {} {} -> {}", "+".green(), attachment, new_filename);
            } else {
                eprintln!(
                    "  {} Template attachment not found: {}",
                    "!".yellow(),
                    attachment
                );
            }
        }
    }

    // Save updated item with attachments
    if !template.attachments().is_empty() {
        item.save(item_path)?;
    }

    Ok(())
}

/// Show template selection TUI and return selected template.
fn select_template(config: &Config) -> Result<Option<Item>> {
    let templates: Vec<Item> = storage::walk_templates(config)
        .filter_map(|path| Item::load(&path).ok())
        .collect();

    if templates.is_empty() {
        anyhow::bail!(
            "No templates found. Create one with: qstack new --as-template \"Template Name\""
        );
    }

    // Format templates in tabular layout matching the item list
    let header = format!(
        "{:<15}  {:<40}  {:<20}  {}",
        "ID", "Title", "Labels", "Category"
    );

    let options: Vec<String> = templates
        .iter()
        .map(|t| {
            let labels_str = ui::truncate(&t.labels().join(", "), 20);
            let title_truncated = ui::truncate(t.title(), 40);
            let category = t
                .path
                .as_ref()
                .and_then(|p| storage::derive_category(config, p))
                .unwrap_or_default();

            format!(
                "{:<15}  {}  {}  {}",
                t.id(),
                ui::pad_to_width(&title_truncated, 40),
                ui::pad_to_width(&labels_str, 20),
                category
            )
        })
        .collect();

    let Some(selection) = ui::select_from_list_with_header("Select a template", &header, &options)?
    else {
        return Ok(None);
    };

    Ok(Some(templates.into_iter().nth(selection).unwrap()))
}
