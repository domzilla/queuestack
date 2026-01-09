//! # Configuration
//!
//! Merged configuration system combining global (~/.qstack) and project (.qstack) settings.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

pub mod global;
pub mod project;

use std::path::{Path, PathBuf};

use anyhow::Result;

pub use self::{global::GlobalConfig, project::ProjectConfig};
use crate::id::DEFAULT_PATTERN;

/// Merged configuration with project settings overriding global
#[derive(Debug, Clone)]
pub struct Config {
    /// Global configuration
    pub global: GlobalConfig,

    /// Project configuration
    pub project: ProjectConfig,

    /// Resolved project root path
    pub project_root: PathBuf,
}

impl Config {
    /// Loads configuration from both global and project sources
    pub fn load() -> Result<Self> {
        let global = GlobalConfig::load()?;

        let project_root = ProjectConfig::find_project_root()
            .ok_or_else(|| anyhow::anyhow!("Not in a qstack project (no .qstack file found)"))?;

        let project = ProjectConfig::load(&project_root)?;

        Ok(Self {
            global,
            project,
            project_root,
        })
    }

    /// Creates a config for initialization (no existing project required)
    pub fn for_init() -> Result<Self> {
        let global = GlobalConfig::load()?;
        let project_root = std::env::current_dir()
            .map_err(|e| anyhow::anyhow!("Cannot get current directory: {e}"))?;
        let project = ProjectConfig::default();

        Ok(Self {
            global,
            project,
            project_root,
        })
    }

    /// Returns the effective ID pattern (project overrides global)
    pub fn id_pattern(&self) -> &str {
        self.project
            .id_pattern
            .as_deref()
            .unwrap_or(&self.global.default_id_pattern)
    }

    /// Returns the effective user name
    pub fn user_name(&self) -> Option<String> {
        self.global.resolve_user_name()
    }

    /// Returns the effective user name, prompting if not available.
    /// Falls back to: config `user_name` -> git `user.name` -> prompt -> error
    pub fn user_name_or_prompt(&mut self) -> Result<String> {
        // Try existing sources first
        if let Some(name) = self.global.resolve_user_name() {
            return Ok(name);
        }

        // Prompt user for name
        if let Some(name) = self.global.prompt_and_save_user_name()? {
            return Ok(name);
        }

        anyhow::bail!(
            "No user name available. Set user_name in ~/.qstack or configure git user.name"
        )
    }

    /// Returns the effective editor command
    pub fn editor(&self) -> Option<String> {
        self.global.editor.clone().or_else(|| {
            std::env::var("VISUAL")
                .ok()
                .or_else(|| std::env::var("EDITOR").ok())
        })
    }

    /// Whether to auto-open editor
    pub const fn auto_open(&self) -> bool {
        self.global.auto_open
    }

    /// Returns the stack directory path
    pub fn stack_path(&self) -> PathBuf {
        self.project.stack_path(&self.project_root)
    }

    /// Returns the archive directory path
    pub fn archive_path(&self) -> PathBuf {
        self.project.archive_path(&self.project_root)
    }

    /// Returns path to a category subdirectory within the stack
    pub fn category_path(&self, category: &str) -> PathBuf {
        self.stack_path().join(category)
    }

    /// Returns path relative to project root
    pub fn relative_path(&self, path: &Path) -> PathBuf {
        path.strip_prefix(&self.project_root)
            .map_or_else(|_| path.to_path_buf(), Path::to_path_buf)
    }
}

/// Default ID pattern constant re-export for convenience
pub const DEFAULT_ID_PATTERN: &str = DEFAULT_PATTERN;
