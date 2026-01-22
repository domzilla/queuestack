//! # Global Configuration
//!
//! Handles the global user configuration stored at `~/.config/queuestack/config`.
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
    constants::{
        DEFAULT_ARCHIVE_DIR, DEFAULT_STACK_DIR, DEFAULT_TEMPLATE_DIR, GLOBAL_CONFIG_DIR,
        GLOBAL_CONFIG_FILENAME,
    },
    id::DEFAULT_PATTERN,
};

/// Valid field names in the global config file.
/// Used for validation to detect unknown/invalid fields.
const VALID_FIELDS: &[&str] = &[
    "user_name",
    "use_git_user",
    "editor",
    "interactive",
    "id_pattern",
    "stack_dir",
    "archive_dir",
    "template_dir",
];

/// Fields that should be present with actual values (have meaningful defaults).
/// Other valid fields are optional personalization (`user_name`, `editor`) that
/// remain commented when not set.
const REQUIRED_FIELDS: &[&str] = &[
    "use_git_user",
    "interactive",
    "id_pattern",
    "stack_dir",
    "archive_dir",
    "template_dir",
];

/// Legacy field names that should be migrated to their new names.
/// Format: (`old_name`, `new_name`)
const LEGACY_ALIASES: &[(&str, &str)] = &[("default_id_pattern", "id_pattern")];

/// Result of validating a config file.
#[derive(Debug, Default)]
pub struct ConfigValidation {
    /// Fields that were missing and have been added with defaults
    pub missing: Vec<String>,
    /// Fields that were unrecognized and have been removed
    pub invalid: Vec<String>,
    /// Fields that were migrated from old names (`old_name`, `new_name`)
    pub migrated: Vec<(String, String)>,
}

impl ConfigValidation {
    /// Returns true if any changes were made to the config.
    pub fn has_changes(&self) -> bool {
        !self.missing.is_empty() || !self.invalid.is_empty() || !self.migrated.is_empty()
    }
}

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

/// Global configuration stored at ~/.config/queuestack/config
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

    /// Directory name for storing items (default: "queuestack")
    #[serde(default)]
    pub stack_dir: Option<String>,

    /// Directory name for archived items (default: "archive")
    #[serde(default)]
    pub archive_dir: Option<String>,

    /// Directory name for templates (default: ".templates")
    #[serde(default)]
    pub template_dir: Option<String>,
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
            template_dir: None,
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
    /// Returns the path to the global config file (~/.config/queuestack/config)
    ///
    /// Checks for a thread-local home override first (used by tests),
    /// then falls back to $HOME/.config (XDG Base Directory).
    pub fn path() -> Option<PathBuf> {
        // Check for thread-local test override first (no env var modification)
        if let Some(home) = get_home_override() {
            return Some(
                home.join(".config")
                    .join(GLOBAL_CONFIG_DIR)
                    .join(GLOBAL_CONFIG_FILENAME),
            );
        }
        // Use $HOME/.config for XDG compliance (not dirs::config_dir which varies by OS)
        dirs::home_dir().map(|home| {
            home.join(".config")
                .join(GLOBAL_CONFIG_DIR)
                .join(GLOBAL_CONFIG_FILENAME)
        })
    }

    /// Returns the path to the global config directory (~/.config/queuestack)
    pub fn dir() -> Option<PathBuf> {
        if let Some(home) = get_home_override() {
            return Some(home.join(".config").join(GLOBAL_CONFIG_DIR));
        }
        dirs::home_dir().map(|home| home.join(".config").join(GLOBAL_CONFIG_DIR))
    }

    /// Loads the global config from ~/.config/queuestack/config.
    /// Fails if the config doesn't exist — user must run `qs setup` first.
    pub fn load() -> Result<Self> {
        let Some(path) = Self::path() else {
            anyhow::bail!("Could not determine home directory");
        };

        if !path.exists() {
            anyhow::bail!("Global config not found. Run {} first.", "qs setup".green());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read global config: {}", path.display()))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse global config: {}", path.display()))
    }

    /// Creates the global config with default values and comments.
    /// Used by `qs setup`. Returns true if created, false if already exists.
    pub fn create_default_if_missing() -> Result<bool> {
        let Some(path) = Self::path() else {
            anyhow::bail!("Could not determine config directory");
        };

        if path.exists() {
            return Ok(false);
        }

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let config = Self::default();
        Self::save_with_comments(&path, &config)?;
        Ok(true)
    }

    /// Saves the global config to ~/.config/queuestack/config
    pub fn save(&self) -> Result<()> {
        let Some(path) = Self::path() else {
            anyhow::bail!("Could not determine config directory");
        };

        // When saving after user input, just update the values without regenerating comments
        let content = toml::to_string_pretty(self).context("Failed to serialize global config")?;

        fs::write(&path, content)
            .with_context(|| format!("Failed to write global config: {}", path.display()))
    }

    /// Saves config with detailed comments for all options.
    ///
    /// Required fields are always written with actual values.
    /// Optional personalization fields (`user_name`, `editor`) are shown as
    /// commented examples when not set.
    fn save_with_comments(path: &PathBuf, config: &Self) -> Result<()> {
        // Helper to format optional personalization fields (commented when not set)
        let format_personalization = |value: &Option<String>, key: &str, example: &str| {
            value.as_ref().map_or_else(
                || format!("# {key} = \"{example}\""),
                |v| format!("{key} = \"{v}\""),
            )
        };

        // Personalization fields: commented when not set
        let user_name_line = format_personalization(&config.user_name, "user_name", "Your Name");
        let editor_line = format_personalization(&config.editor, "editor", "nvim");

        // Required fields: always written with effective values
        let stack_dir_line = format!("stack_dir = \"{}\"", config.stack_dir());
        let archive_dir_line = format!("archive_dir = \"{}\"", config.archive_dir());
        let template_dir_line = format!("template_dir = \"{}\"", config.template_dir());

        // id_pattern always has a value (has default), so we always write it
        // but check if it's the default to decide on commenting
        let id_pattern_line = format!("id_pattern = \"{}\"", config.id_pattern);

        let content = format!(
            r#"# queuestack Global Configuration
# This file configures queuestack behavior across all projects.
# Location: ~/.config/queuestack/config

# Your display name used as the author when creating new items.
# If not set, falls back to git user.name (if use_git_user is true).
{user_name_line}

# Whether to use `git config user.name` as a fallback when user_name is not set.
# Default: true
use_git_user = {use_git_user}

# Editor command to open when creating new items.
# Supports commands with arguments (e.g., "code --wait", "nvim").
# If not set, falls back to $VISUAL, then $EDITOR, then "vi".
{editor_line}

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
{id_pattern_line}

# Default directory name for storing items (relative to project root).
# Used when initializing new projects. Can be overridden per-project.
# Default: "queuestack"
{stack_dir_line}

# Default subdirectory name for archived (closed) items within the queuestack directory.
# Used when initializing new projects. Can be overridden per-project.
# Default: ".archive"
{archive_dir_line}

# Default subdirectory name for templates within the queuestack directory.
# Used when initializing new projects. Can be overridden per-project.
# Default: ".templates"
{template_dir_line}
"#,
            user_name_line = user_name_line,
            use_git_user = config.use_git_user,
            editor_line = editor_line,
            interactive = config.interactive,
            id_pattern_line = id_pattern_line,
            stack_dir_line = stack_dir_line,
            archive_dir_line = archive_dir_line,
            template_dir_line = template_dir_line,
        );

        fs::write(path, content)
            .with_context(|| format!("Failed to write global config: {}", path.display()))
    }

    /// Returns the effective queuestack directory name
    pub fn stack_dir(&self) -> &str {
        self.stack_dir.as_deref().unwrap_or(DEFAULT_STACK_DIR)
    }

    /// Returns the effective archive directory name
    pub fn archive_dir(&self) -> &str {
        self.archive_dir.as_deref().unwrap_or(DEFAULT_ARCHIVE_DIR)
    }

    /// Returns the effective template directory name
    pub fn template_dir(&self) -> &str {
        self.template_dir.as_deref().unwrap_or(DEFAULT_TEMPLATE_DIR)
    }

    /// Validates the global config file and returns any issues found.
    ///
    /// This parses the raw TOML to detect:
    /// - Unknown fields that should be removed
    /// - Legacy field names that should be migrated
    /// - Missing fields (by comparing against what serde would produce)
    pub fn validate() -> Result<ConfigValidation> {
        let Some(path) = Self::path() else {
            anyhow::bail!("Could not determine home directory");
        };

        if !path.exists() {
            anyhow::bail!("Global config not found");
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read global config: {}", path.display()))?;

        let table: toml::Table = toml::from_str(&content)
            .with_context(|| format!("Failed to parse global config: {}", path.display()))?;

        let mut validation = ConfigValidation::default();

        // Check for invalid/unknown fields
        for key in table.keys() {
            // Check if it's a legacy alias
            if let Some((old, new)) = LEGACY_ALIASES.iter().find(|(old, _)| old == key) {
                validation
                    .migrated
                    .push(((*old).to_string(), (*new).to_string()));
            } else if !VALID_FIELDS.contains(&key.as_str()) {
                validation.invalid.push(key.clone());
            }
        }

        // Check for missing required fields (fields that should always be present)
        // Optional personalization fields (user_name, editor) are not reported as missing
        for &field in REQUIRED_FIELDS {
            if !table.contains_key(field) {
                // Check if it's covered by a legacy alias
                let covered_by_alias = LEGACY_ALIASES
                    .iter()
                    .any(|(old, new)| *new == field && table.contains_key(*old));
                if !covered_by_alias {
                    validation.missing.push(field.to_string());
                }
            }
        }

        Ok(validation)
    }

    /// Validates and updates the global config file if needed.
    ///
    /// This will:
    /// 1. Check for missing, invalid, or legacy fields
    /// 2. If changes are needed, load the config (serde fills defaults), then re-save
    /// 3. Return a validation report of what changed
    pub fn update_if_needed() -> Result<ConfigValidation> {
        let validation = Self::validate()?;

        if !validation.has_changes() {
            return Ok(validation);
        }

        let Some(path) = Self::path() else {
            anyhow::bail!("Could not determine home directory");
        };

        // Load the config - serde will:
        // - Use defaults for missing fields
        // - Use alias values for legacy fields (default_id_pattern -> id_pattern)
        // - Ignore unknown fields (they won't be in the struct)
        let config = Self::load()?;

        // Re-save with comments - this will:
        // - Write all valid fields with proper values
        // - Exclude unknown/invalid fields (they're not in the struct)
        // - Use the canonical field names (not legacy aliases)
        Self::save_with_comments(&path, &config)?;

        Ok(validation)
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
            Self::path().map_or_else(
                || "~/.config/queuestack/config".to_string(),
                |p| p.display().to_string()
            )
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
        assert_eq!(config.stack_dir(), "queuestack");
        assert_eq!(config.archive_dir(), ".archive");
        assert_eq!(config.template_dir(), ".templates");
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
        let expected_path = temp
            .path()
            .join(".config")
            .join("queuestack")
            .join("config");

        // Set thread-local override
        set_home_override(Some(temp.path().to_path_buf()));
        let path = GlobalConfig::path().unwrap();
        assert_eq!(path, expected_path);

        // Clear override - should fall back to real config dir
        set_home_override(None);
        let path = GlobalConfig::path();
        assert!(path.is_some());
        assert_ne!(path.unwrap(), expected_path);
    }
}
