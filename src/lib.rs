//! # qstack
//!
//! A CLI-based task and issue tracker following a "documentation as code" philosophy.
//!
//! Items are stored as Markdown files with YAML frontmatter, organized in a directory
//! structure within a Git repository. Designed to be human-readable, grep-friendly,
//! and fully integrated with standard developer workflows.
//!
//! ## Features
//!
//! - **Markdown Storage**: Items are plain Markdown files with YAML frontmatter
//! - **Git Integration**: Automatic `git mv` to preserve history
//! - **Categorization**: Organize items in subdirectories
//! - **Flexible IDs**: Customizable ID patterns using Crockford's Base32
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

pub mod commands;
pub mod config;
pub mod editor;
pub mod id;
pub mod item;
pub mod storage;

pub use config::{set_home_override, Config};
pub use item::{is_url, Frontmatter, Item, Status};
