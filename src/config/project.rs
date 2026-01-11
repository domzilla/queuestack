//! # Project Configuration
//!
//! Handles the project-level configuration stored at `.qstack` in the project root.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Project configuration file name
pub const PROJECT_CONFIG_FILE: &str = ".qstack";

/// Project configuration stored at .qstack in project root
///
/// All fields are optional. When not set, values fall back to global config.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// User's display name (overrides global)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,

    /// Whether to use git config user.name as fallback (overrides global)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_git_user: Option<bool>,

    /// Editor command (overrides global)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor: Option<String>,

    /// Whether to auto-open editor (overrides global)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_open: Option<bool>,

    /// ID pattern override (overrides global)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_pattern: Option<String>,

    /// Directory name for storing items (overrides global)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack_dir: Option<String>,

    /// Directory name for archived items (overrides global)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_dir: Option<String>,
}

impl ProjectConfig {
    /// Finds the project root by searching for .qstack file upward
    pub fn find_project_root() -> Option<PathBuf> {
        let mut current = env::current_dir().ok()?;

        loop {
            if current.join(PROJECT_CONFIG_FILE).exists() {
                return Some(current);
            }

            if !current.pop() {
                return None;
            }
        }
    }

    /// Returns the path to the project config file
    pub fn path(project_root: &Path) -> PathBuf {
        project_root.join(PROJECT_CONFIG_FILE)
    }

    /// Loads the project config from .qstack in the given directory
    pub fn load(project_root: &Path) -> Result<Self> {
        let path = Self::path(project_root);

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read project config: {}", path.display()))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse project config: {}", path.display()))
    }

    /// Saves the project config to .qstack
    pub fn save(&self, project_root: &Path) -> Result<()> {
        let path = Self::path(project_root);
        let content = toml::to_string_pretty(self).context("Failed to serialize project config")?;

        fs::write(&path, content)
            .with_context(|| format!("Failed to write project config: {}", path.display()))
    }

    /// Saves project config with detailed comments for all options.
    ///
    /// The `stack_dir` and `archive_dir` parameters are written as explicit values,
    /// while all other options are commented out (falling back to global config).
    pub fn save_with_comments(
        project_root: &Path,
        stack_dir: &str,
        archive_dir: &str,
    ) -> Result<()> {
        let path = Self::path(project_root);

        let content = format!(
            r#"# qstack Project Configuration
# This file configures qstack for this specific project.
# All settings here override the global config (~/.qstack).
# Location: <project-root>/.qstack

# User's display name for item authorship.
# If not set, falls back to global config.
# user_name = "Your Name"

# Whether to use `git config user.name` as a fallback.
# If not set, falls back to global config.
# use_git_user = true

# Editor command to open when creating new items.
# If not set, falls back to global config.
# editor = "nvim"

# Whether to automatically open the editor when creating a new item.
# If not set, falls back to global config.
# auto_open = true

# Pattern for generating unique item IDs.
# If not set, falls back to global config.
#
# Available tokens:
#   %y  - Year (2 digits, e.g., "26" for 2026)
#   %m  - Month (2 digits, 01-12)
#   %d  - Day of month (2 digits, 01-31)
#   %j  - Day of year (3 digits, 001-366)
#   %T  - Time as Base32 (4 chars) - seconds since midnight UTC
#   %R  - Random Base32 character (repeat for more: %RRR = 3 chars)
#   %%  - Literal percent sign
#
# id_pattern = "%y%m%d-%T%RRR"

# Directory name for storing items (relative to project root).
stack_dir = "{stack_dir}"

# Subdirectory name for archived (closed) items within the stack directory.
archive_dir = "{archive_dir}"
"#
        );

        fs::write(&path, content)
            .with_context(|| format!("Failed to write project config: {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ProjectConfig::default();
        assert!(config.user_name.is_none());
        assert!(config.use_git_user.is_none());
        assert!(config.editor.is_none());
        assert!(config.auto_open.is_none());
        assert!(config.id_pattern.is_none());
        assert!(config.stack_dir.is_none());
        assert!(config.archive_dir.is_none());
    }

    #[test]
    fn test_parse_config() {
        let toml = r#"
stack_dir = "issues"
archive_dir = "done"
id_pattern = "%y%j-%RRR"
user_name = "Test User"
auto_open = false
"#;
        let config: ProjectConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.stack_dir, Some("issues".to_string()));
        assert_eq!(config.archive_dir, Some("done".to_string()));
        assert_eq!(config.id_pattern, Some("%y%j-%RRR".to_string()));
        assert_eq!(config.user_name, Some("Test User".to_string()));
        assert_eq!(config.auto_open, Some(false));
    }

    #[test]
    fn test_parse_minimal_config() {
        // Empty config should work - all fields are optional
        let toml = "";
        let config: ProjectConfig = toml::from_str(toml).unwrap();
        assert!(config.stack_dir.is_none());
        assert!(config.archive_dir.is_none());
    }
}
