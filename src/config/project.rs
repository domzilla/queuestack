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

/// Default stack directory name
const DEFAULT_STACK_DIR: &str = "qstack";

/// Default archive directory name
const DEFAULT_ARCHIVE_DIR: &str = "archive";

/// Project configuration stored at .qstack in project root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Directory name for storing items (default: "qstack")
    #[serde(default = "default_stack_dir")]
    pub stack_dir: String,

    /// Directory name for archived items (default: "archive")
    #[serde(default = "default_archive_dir")]
    pub archive_dir: String,

    /// ID pattern override (uses global default if not set)
    #[serde(default)]
    pub id_pattern: Option<String>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            stack_dir: DEFAULT_STACK_DIR.to_string(),
            archive_dir: DEFAULT_ARCHIVE_DIR.to_string(),
            id_pattern: None,
        }
    }
}

fn default_stack_dir() -> String {
    DEFAULT_STACK_DIR.to_string()
}

fn default_archive_dir() -> String {
    DEFAULT_ARCHIVE_DIR.to_string()
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

    /// Returns the full path to the stack directory
    pub fn stack_path(&self, project_root: &Path) -> PathBuf {
        project_root.join(&self.stack_dir)
    }

    /// Returns the full path to the archive directory
    pub fn archive_path(&self, project_root: &Path) -> PathBuf {
        self.stack_path(project_root).join(&self.archive_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ProjectConfig::default();
        assert_eq!(config.stack_dir, DEFAULT_STACK_DIR);
        assert_eq!(config.archive_dir, DEFAULT_ARCHIVE_DIR);
        assert!(config.id_pattern.is_none());
    }

    #[test]
    fn test_parse_config() {
        let toml = r#"
stack_dir = "issues"
archive_dir = "done"
id_pattern = "%y%j-%RRR"
"#;
        let config: ProjectConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.stack_dir, "issues");
        assert_eq!(config.archive_dir, "done");
        assert_eq!(config.id_pattern, Some("%y%j-%RRR".to_string()));
    }

    #[test]
    fn test_paths() {
        let config = ProjectConfig::default();
        let root = PathBuf::from("/project");

        assert_eq!(config.stack_path(&root), PathBuf::from("/project/qstack"));
        assert_eq!(
            config.archive_path(&root),
            PathBuf::from("/project/qstack/archive")
        );
    }
}
