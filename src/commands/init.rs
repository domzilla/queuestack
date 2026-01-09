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

    // Create project config
    let project_config = ProjectConfig::default();
    project_config.save(&config.project_root)?;

    // Create stack directory
    let stack_path = project_config.stack_path(&config.project_root);
    fs::create_dir_all(&stack_path)
        .with_context(|| format!("Failed to create stack directory: {}", stack_path.display()))?;

    // Create archive directory
    let archive_path = project_config.archive_path(&config.project_root);
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
