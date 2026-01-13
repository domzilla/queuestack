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
};

use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

use crate::{
    constants::{DEFAULT_ARCHIVE_DIR, DEFAULT_STACK_DIR, GLOBAL_CONFIG_FILE},
    id::DEFAULT_PATTERN,
};

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

    /// Whether to enable interactive mode (open editor, show selectors)
    #[serde(default = "default_true")]
    pub interactive: bool,

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
            interactive: true,
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

    /// Loads the global config from ~/.qstack.
    /// Fails if the config doesn't exist — user must run `qstack setup` first.
    pub fn load() -> Result<Self> {
        let Some(path) = Self::path() else {
            anyhow::bail!("Could not determine home directory");
        };

        if !path.exists() {
            anyhow::bail!(
                "Global config not found. Run {} first.",
                "qstack setup".green()
            );
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read global config: {}", path.display()))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse global config: {}", path.display()))
    }

    /// Creates the global config with default values and comments.
    /// Used by `qstack setup`. Returns true if created, false if already exists.
    pub fn create_default_if_missing() -> Result<bool> {
        let Some(path) = Self::path() else {
            anyhow::bail!("Could not determine home directory");
        };

        if path.exists() {
            return Ok(false);
        }

        let config = Self::default();
        Self::save_with_comments(&path, &config)?;
        Ok(true)
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

# Whether to enable interactive mode (opens editor, shows selection dialogs).
# Set to false for scripting or if you prefer to edit files manually.
# Default: true
interactive = {interactive}

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

# Default subdirectory name for archived (closed) items within the qstack directory.
# Used when initializing new projects. Can be overridden per-project.
# Default: ".archive"
# archive_dir = ".archive"
"#,
            use_git_user = config.use_git_user,
            interactive = config.interactive,
            id_pattern = config.id_pattern,
        );

        fs::write(path, content)
            .with_context(|| format!("Failed to write global config: {}", path.display()))
    }

    /// Returns the effective qstack directory name
    pub fn stack_dir(&self) -> &str {
        self.stack_dir.as_deref().unwrap_or(DEFAULT_STACK_DIR)
    }

    /// Returns the effective archive directory name
    pub fn archive_dir(&self) -> &str {
        self.archive_dir.as_deref().unwrap_or(DEFAULT_ARCHIVE_DIR)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GlobalConfig::default();
        assert!(config.use_git_user);
        assert!(config.interactive);
        assert_eq!(config.id_pattern, DEFAULT_PATTERN);
        assert_eq!(config.stack_dir(), "qstack");
        assert_eq!(config.archive_dir(), ".archive");
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
