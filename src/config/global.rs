//! # Global Configuration
//!
//! Handles the global user configuration stored at `~/.qstack`.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::{fs, path::PathBuf, process::Command};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::id::DEFAULT_PATTERN;

/// Global configuration file name
const GLOBAL_CONFIG_FILE: &str = ".qstack";

/// Global configuration stored at ~/.qstack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// User's display name
    #[serde(default)]
    pub user_name: Option<String>,

    /// Whether to use git config user.name as fallback
    #[serde(default = "default_true")]
    pub use_git_user: bool,

    /// Editor command (e.g., "nvim", "code --wait")
    #[serde(default)]
    pub editor: Option<String>,

    /// Whether to auto-open editor on `new` command
    #[serde(default = "default_true")]
    pub auto_open: bool,

    /// Default ID pattern
    #[serde(default = "default_id_pattern")]
    pub default_id_pattern: String,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            user_name: None,
            use_git_user: true,
            editor: None,
            auto_open: true,
            default_id_pattern: DEFAULT_PATTERN.to_string(),
        }
    }
}

#[allow(clippy::missing_const_for_fn)] // serde default functions can't be const
fn default_true() -> bool {
    true
}

fn default_id_pattern() -> String {
    DEFAULT_PATTERN.to_string()
}

impl GlobalConfig {
    /// Returns the path to the global config file (~/.qstack)
    pub fn path() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(GLOBAL_CONFIG_FILE))
    }

    /// Loads the global config from ~/.qstack
    pub fn load() -> Result<Self> {
        let Some(path) = Self::path() else {
            return Ok(Self::default());
        };

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read global config: {}", path.display()))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse global config: {}", path.display()))
    }

    /// Saves the global config to ~/.qstack
    pub fn save(&self) -> Result<()> {
        let Some(path) = Self::path() else {
            anyhow::bail!("Could not determine home directory");
        };

        let content = toml::to_string_pretty(self).context("Failed to serialize global config")?;

        fs::write(&path, content)
            .with_context(|| format!("Failed to write global config: {}", path.display()))
    }

    /// Resolves the effective user name, checking git config if enabled
    pub fn resolve_user_name(&self) -> Option<String> {
        // First check explicit user_name
        if let Some(ref name) = self.user_name {
            return Some(name.clone());
        }

        // Then try git config if enabled
        if self.use_git_user {
            return git_user_name();
        }

        None
    }
}

/// Gets the user name from git config
fn git_user_name() -> Option<String> {
    Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|name| !name.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GlobalConfig::default();
        assert!(config.use_git_user);
        assert!(config.auto_open);
        assert_eq!(config.default_id_pattern, DEFAULT_PATTERN);
    }

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
user_name = "Test User"
"#;
        let config: GlobalConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.user_name, Some("Test User".to_string()));
        assert!(config.use_git_user); // default
    }
}
