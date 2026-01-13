//! # Constants
//!
//! Centralized constants for magic values used throughout qstack.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

// =============================================================================
// UI Display
// =============================================================================

/// Maximum length for title display in lists (truncated with ellipsis).
pub const UI_TITLE_TRUNCATE_LEN: usize = 40;

/// Maximum length for labels display in lists (truncated with ellipsis).
pub const UI_LABELS_TRUNCATE_LEN: usize = 20;

/// Column width for ID in list display.
pub const UI_COL_ID_WIDTH: usize = 15;

/// Column width for status in list display.
pub const UI_COL_STATUS_WIDTH: usize = 6;

// =============================================================================
// Item Format
// =============================================================================

/// Maximum slug length in characters (not bytes).
pub const MAX_SLUG_LENGTH: usize = 50;

/// YAML frontmatter delimiter.
pub const FRONTMATTER_DELIMITER: &str = "---";

/// Attachment filename infix (between item ID and counter).
pub const ATTACHMENT_INFIX: &str = "-Attachment-";

// =============================================================================
// File System
// =============================================================================

/// File extension for item files.
pub const ITEM_FILE_EXTENSION: &str = "md";

/// Default directory name for storing items.
pub const DEFAULT_STACK_DIR: &str = "qstack";

/// Default subdirectory name for archived items (inside `stack_dir`).
pub const DEFAULT_ARCHIVE_DIR: &str = ".archive";

/// Global configuration file name.
pub const GLOBAL_CONFIG_FILE: &str = ".qstack";

// =============================================================================
// Shell Completion Paths
// =============================================================================

/// Zsh custom completions directory (relative to home).
pub const ZSH_COMPLETIONS_DIR: &str = ".zfunc";

/// Zsh completion file name.
pub const ZSH_COMPLETION_FILE: &str = "_qstack";

/// Bash completions directory (relative to home).
pub const BASH_COMPLETIONS_DIR: &str = ".local/share/bash-completion/completions";

/// Bash completion file name.
pub const BASH_COMPLETION_FILE: &str = "qstack";

/// Fish completions directory (relative to home).
pub const FISH_COMPLETIONS_DIR: &str = ".config/fish/completions";

/// Fish completion file name.
pub const FISH_COMPLETION_FILE: &str = "qstack.fish";

/// Elvish completions directory (relative to home).
pub const ELVISH_COMPLETIONS_DIR: &str = ".config/elvish/lib";

/// Elvish completion file name.
pub const ELVISH_COMPLETION_FILE: &str = "qstack.elv";

// =============================================================================
// Shell RC Files
// =============================================================================

/// Zsh config file name.
pub const ZSHRC_FILE: &str = ".zshrc";

/// Bash config file name (primary).
pub const BASHRC_FILE: &str = ".bashrc";

/// Bash config file name (fallback).
pub const BASH_PROFILE_FILE: &str = ".bash_profile";

/// `PowerShell` profile directory (Unix).
pub const POWERSHELL_CONFIG_DIR_UNIX: &str = ".config/powershell";

/// `PowerShell` profile directory (Windows).
pub const POWERSHELL_CONFIG_DIR_WINDOWS: &str = "Documents/PowerShell";

/// `PowerShell` profile file name.
pub const POWERSHELL_PROFILE_FILE: &str = "Microsoft.PowerShell_profile.ps1";
