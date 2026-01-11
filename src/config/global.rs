//! # Global Configuration
//!
//! Handles the global user configuration stored at `~/.qstack`.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::{
    cell::RefCell,
    fs,
    io::{self, IsTerminal, Write},
    path::PathBuf,
    process::Command,
};

use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

use crate::id::DEFAULT_PATTERN;

thread_local! {
    /// Thread-local override for the home directory path.
    /// Used by integration tests to redirect config to a temp directory
    /// without modifying environment variables.
    static HOME_OVERRIDE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// Sets a thread-local override for the home directory.
/// This is used by tests to redirect global config without modifying env vars.
pub fn set_home_override(path: Option<PathBuf>) {
    HOME_OVERRIDE.with(|cell| {
        *cell.borrow_mut() = path;
    });
}

/// Gets the current home directory override, if set.
fn get_home_override() -> Option<PathBuf> {
    HOME_OVERRIDE.with(|cell| cell.borrow().clone())
}

/// Global configuration file name
const GLOBAL_CONFIG_FILE: &str = ".qstack";

/// Default stack directory name
const DEFAULT_STACK_DIR: &str = "qstack";

/// Default archive directory name
const DEFAULT_ARCHIVE_DIR: &str = "archive";

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

    /// ID pattern for generating unique identifiers
    #[serde(default = "default_id_pattern", alias = "default_id_pattern")]
    pub id_pattern: String,

    /// Directory name for storing items (default: "qstack")
    #[serde(default)]
    pub stack_dir: Option<String>,

    /// Directory name for archived items (default: "archive")
    #[serde(default)]
    pub archive_dir: Option<String>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            user_name: None,
            use_git_user: true,
            editor: None,
            auto_open: true,
            id_pattern: DEFAULT_PATTERN.to_string(),
            stack_dir: None,
            archive_dir: None,
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
    ///
    /// Checks for a thread-local home override first (used by tests),
    /// then falls back to the actual home directory.
    pub fn path() -> Option<PathBuf> {
        // Check for thread-local test override first (no env var modification)
        if let Some(home) = get_home_override() {
            return Some(home.join(GLOBAL_CONFIG_FILE));
        }
        dirs::home_dir().map(|home| home.join(GLOBAL_CONFIG_FILE))
    }

    /// Loads the global config from ~/.qstack, creating it if it doesn't exist
    pub fn load() -> Result<Self> {
        let Some(path) = Self::path() else {
            return Ok(Self::default());
        };

        if !path.exists() {
            // Auto-generate default config with comments
            let config = Self::default();
            Self::save_with_comments(&path, &config)?;
            eprintln!("{} Created global config: {}", "✓".green(), path.display());
            return Ok(config);
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

        // When saving after user input, just update the values without regenerating comments
        let content = toml::to_string_pretty(self).context("Failed to serialize global config")?;

        fs::write(&path, content)
            .with_context(|| format!("Failed to write global config: {}", path.display()))
    }

    /// Saves config with detailed comments for all options
    fn save_with_comments(path: &PathBuf, config: &Self) -> Result<()> {
        let content = format!(
            r#"# qstack Global Configuration
# This file configures qstack behavior across all projects.
# Location: ~/.qstack

# Your display name used as the author when creating new items.
# If not set, falls back to git user.name (if use_git_user is true).
# user_name = "Your Name"

# Whether to use `git config user.name` as a fallback when user_name is not set.
# Default: true
use_git_user = {use_git_user}

# Editor command to open when creating new items.
# Supports commands with arguments (e.g., "code --wait", "nvim").
# If not set, falls back to $VISUAL, then $EDITOR, then "vi".
# editor = "nvim"

# Whether to automatically open the editor when creating a new item.
# Set to false if you prefer to edit files manually or use qstack in scripts.
# Default: true
auto_open = {auto_open}

# Pattern for generating unique item IDs.
# Default: "%y%m%d-%T%RRR" (e.g., "260109-0A2BK4M")
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
# Base32 uses Crockford's alphabet: 0-9, A-Z excluding I, L, O, U
# This ensures IDs are human-readable and avoid ambiguous characters.
#
# Examples:
#   "%y%m%d-%T%RRR"  -> "260109-0A2BK4M" (default, 14 chars)
#   "%y%j-%T%RR"     -> "26009-0A2BK4"   (day-of-year variant, 12 chars)
#   "%T%RRRR"        -> "0A2BK4MN"       (compact, 8 chars)
# id_pattern = "{id_pattern}"

# Default directory name for storing items (relative to project root).
# Used when initializing new projects. Can be overridden per-project.
# Default: "qstack"
# stack_dir = "qstack"

# Default subdirectory name for archived (closed) items within the stack directory.
# Used when initializing new projects. Can be overridden per-project.
# Default: "archive"
# archive_dir = "archive"
"#,
            use_git_user = config.use_git_user,
            auto_open = config.auto_open,
            id_pattern = config.id_pattern,
        );

        fs::write(path, content)
            .with_context(|| format!("Failed to write global config: {}", path.display()))
    }

    /// Returns the effective stack directory name
    pub fn stack_dir(&self) -> &str {
        self.stack_dir.as_deref().unwrap_or(DEFAULT_STACK_DIR)
    }

    /// Returns the effective archive directory name
    pub fn archive_dir(&self) -> &str {
        self.archive_dir.as_deref().unwrap_or(DEFAULT_ARCHIVE_DIR)
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

    /// Prompts the user for their name and saves it to the config.
    /// Only prompts if running in a terminal.
    pub fn prompt_and_save_user_name(&mut self) -> Result<Option<String>> {
        // Only prompt if we're in a terminal
        if !io::stdin().is_terminal() {
            return Ok(None);
        }

        eprint!("{}", "Enter your name for item authorship: ".bold());
        io::stderr().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let name = input.trim().to_string();
        if name.is_empty() {
            return Ok(None);
        }

        // Save to config
        self.user_name = Some(name.clone());
        self.save()?;

        eprintln!(
            "{} Saved user name to {}",
            "✓".green(),
            Self::path().map_or_else(|| "~/.qstack".to_string(), |p| p.display().to_string())
        );

        Ok(Some(name))
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
        assert_eq!(config.id_pattern, DEFAULT_PATTERN);
        assert_eq!(config.stack_dir(), "qstack");
        assert_eq!(config.archive_dir(), "archive");
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

    #[test]
    fn test_parse_old_field_name() {
        // Test backwards compatibility with old field name
        let toml = r#"
default_id_pattern = "%y%j-%RRR"
"#;
        let config: GlobalConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.id_pattern, "%y%j-%RRR");
    }

    #[test]
    fn test_home_override() {
        use tempfile::tempdir;

        let temp = tempdir().unwrap();

        // Set thread-local override
        set_home_override(Some(temp.path().to_path_buf()));
        let path = GlobalConfig::path().unwrap();
        assert_eq!(path, temp.path().join(".qstack"));

        // Clear override - should fall back to real home dir
        set_home_override(None);
        let path = GlobalConfig::path();
        assert!(path.is_some());
        assert_ne!(path.unwrap(), temp.path().join(".qstack"));
    }
}
