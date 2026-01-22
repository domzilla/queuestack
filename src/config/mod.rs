//! # Configuration
//!
//! Merged configuration system combining global (~/.config/queuestack/config) and project
//! (.queuestack) settings. Project settings override global settings when specified.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

pub mod global;
pub mod project;

use std::path::{Path, PathBuf};

use anyhow::Result;

pub use self::{
    global::{set_home_override, ConfigValidation, GlobalConfig},
    project::ProjectConfig,
};
use crate::{id::DEFAULT_PATTERN, storage::git};

/// Merged configuration with project settings overriding global
#[derive(Debug, Clone)]
pub struct Config {
    /// Global configuration (private - use resolution methods)
    global: GlobalConfig,

    /// Project configuration (private - use resolution methods)
    project: ProjectConfig,

    /// Resolved project root path
    project_root: PathBuf,
}

impl Config {
    /// Loads configuration from both global and project sources
    pub fn load() -> Result<Self> {
        let global = GlobalConfig::load()?;

        let project_root = ProjectConfig::find_project_root().ok_or_else(|| {
            anyhow::anyhow!("Not in a queuestack project (no .queuestack file found)")
        })?;

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

    // -------------------------------------------------------------------------
    // Resolution methods: project overrides global
    // -------------------------------------------------------------------------

    /// Returns the effective ID pattern (project overrides global)
    pub fn id_pattern(&self) -> &str {
        self.project
            .id_pattern
            .as_deref()
            .unwrap_or(&self.global.id_pattern)
    }

    /// Returns the effective queuestack directory name (project overrides global)
    pub fn stack_dir(&self) -> &str {
        self.project
            .stack_dir
            .as_deref()
            .unwrap_or_else(|| self.global.stack_dir())
    }

    /// Returns the effective archive directory name (project overrides global)
    pub fn archive_dir(&self) -> &str {
        self.project
            .archive_dir
            .as_deref()
            .unwrap_or_else(|| self.global.archive_dir())
    }

    /// Returns the effective template directory name (project overrides global)
    pub fn template_dir(&self) -> &str {
        self.project
            .template_dir
            .as_deref()
            .unwrap_or_else(|| self.global.template_dir())
    }

    /// Returns the effective `use_git_user` setting (project overrides global)
    pub fn use_git_user(&self) -> bool {
        self.project
            .use_git_user
            .unwrap_or(self.global.use_git_user)
    }

    /// Whether interactive mode is enabled (project overrides global)
    pub fn interactive(&self) -> bool {
        self.project.interactive.unwrap_or(self.global.interactive)
    }

    /// Returns the effective user name (project overrides global)
    pub fn user_name(&self) -> Option<String> {
        // First check project-level user_name
        if let Some(ref name) = self.project.user_name {
            return Some(name.clone());
        }

        // Then check global user_name
        if let Some(ref name) = self.global.user_name {
            return Some(name.clone());
        }

        // Then try git config if enabled
        if self.use_git_user() {
            return git::user_name();
        }

        None
    }

    /// Returns the effective user name, prompting if not available.
    /// Falls back to: config `user_name` -> git `user.name` -> prompt -> error
    pub fn user_name_or_prompt(&mut self) -> Result<String> {
        // Try existing sources first
        if let Some(name) = self.user_name() {
            return Ok(name);
        }

        // Prompt user for name (saves to global config)
        if let Some(name) = self.global.prompt_and_save_user_name()? {
            return Ok(name);
        }

        anyhow::bail!(
            "No user name available. Set user_name in ~/.config/queuestack/config or configure git user.name"
        )
    }

    /// Returns the effective editor command (project overrides global)
    pub fn editor(&self) -> Option<String> {
        self.project
            .editor
            .clone()
            .or_else(|| self.global.editor.clone())
            .or_else(|| std::env::var("VISUAL").ok())
            .or_else(|| std::env::var("EDITOR").ok())
    }

    // -------------------------------------------------------------------------
    // Path helpers
    // -------------------------------------------------------------------------

    /// Returns the project root path
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Returns the queuestack directory path
    pub fn stack_path(&self) -> PathBuf {
        self.project_root.join(self.stack_dir())
    }

    /// Returns the archive directory path
    pub fn archive_path(&self) -> PathBuf {
        self.stack_path().join(self.archive_dir())
    }

    /// Returns the template directory path
    pub fn template_path(&self) -> PathBuf {
        self.stack_path().join(self.template_dir())
    }

    /// Returns path to a category subdirectory within queuestack
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
