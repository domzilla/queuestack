//! # qstack
//!
//! A minimal, scriptable task and issue tracker for agent-driven workflows.
//!
//! Items are stored as plain Markdown files, making them human-readable, grep-friendly,
//! and easy to integrate into any workflow.
//!
//! ## Features
//!
//! - **Markdown Storage**: Items are plain Markdown files with YAML frontmatter
//! - **Git Integration**: Automatic `git mv` to preserve history
//! - **Categorization**: Organize items in subdirectories
//! - **Flexible IDs**: Customizable ID patterns
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

pub mod commands;
pub mod config;
pub mod constants;
pub mod editor;
pub mod id;
pub mod item;
pub mod storage;
pub mod tui;
pub mod ui;

pub use config::{set_home_override, Config};
pub use item::{is_url, Frontmatter, Item, Status};
