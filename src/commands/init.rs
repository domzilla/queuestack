//! # Init Command
//!
//! Initializes a new qstack project in the current directory.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::fs;

use anyhow::{Context, Result};
use owo_colors::OwoColorize;

use crate::config::{project::PROJECT_CONFIG_FILE, Config, ProjectConfig};

/// Executes the init command.
pub fn execute() -> Result<()> {
    let config = Config::for_init()?;

    // Check if already initialized
    let config_path = config.project_root.join(PROJECT_CONFIG_FILE);
    if config_path.exists() {
        anyhow::bail!(
            "Project already initialized (found {})",
            config_path.display()
        );
    }

    // Get directory names from global config (with defaults)
    let stack_dir = config.stack_dir();
    let archive_dir = config.archive_dir();

    // Create project config with comments
    // stack_dir and archive_dir are set explicitly, other options are commented out
    ProjectConfig::save_with_comments(&config.project_root, stack_dir, archive_dir)?;

    // Create stack directory
    let stack_path = config.project_root.join(stack_dir);
    fs::create_dir_all(&stack_path)
        .with_context(|| format!("Failed to create stack directory: {}", stack_path.display()))?;

    // Create archive directory
    let archive_path = stack_path.join(archive_dir);
    fs::create_dir_all(&archive_path).with_context(|| {
        format!(
            "Failed to create archive directory: {}",
            archive_path.display()
        )
    })?;

    println!("{} Initialized qstack project", "✓".green());
    println!("  {} {}", "Config:".dimmed(), config_path.display());
    println!("  {} {}", "Stack:".dimmed(), stack_path.display());

    Ok(())
}
