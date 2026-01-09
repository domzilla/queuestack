//! # Item
//!
//! Represents a qstack item (task/issue) with YAML frontmatter metadata.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

pub mod parser;
pub mod slug;

use std::{
    fmt,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use self::slug::slugify;

/// Item status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Open,
    Closed,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::Closed => write!(f, "closed"),
        }
    }
}

/// YAML frontmatter for an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    /// Unique sortable ID
    pub id: String,

    /// Original full title
    pub title: String,

    /// Creator's name
    pub author: String,

    /// Creation timestamp (UTC)
    pub created_at: DateTime<Utc>,

    /// Item status
    #[serde(default)]
    pub status: Status,

    /// Metadata labels/tags
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,

    /// Category (matches parent subdirectory)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

/// A complete item with frontmatter and body
#[derive(Debug, Clone)]
pub struct Item {
    /// YAML frontmatter
    pub frontmatter: Frontmatter,

    /// Markdown body content
    pub body: String,

    /// File path (if loaded from disk)
    pub path: Option<PathBuf>,
}

impl Item {
    /// Creates a new item with the given frontmatter
    #[allow(clippy::missing_const_for_fn)] // String::new() is not const in stable Rust
    pub fn new(frontmatter: Frontmatter) -> Self {
        Self {
            frontmatter,
            body: String::new(),
            path: None,
        }
    }

    /// Loads an item from a file path
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read item: {}", path.display()))?;

        let (frontmatter, body) = parser::parse(&content)
            .with_context(|| format!("Failed to parse item: {}", path.display()))?;

        Ok(Self {
            frontmatter,
            body,
            path: Some(path.to_path_buf()),
        })
    }

    /// Saves the item to disk
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = parser::serialize(&self.frontmatter, &self.body)?;
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write item: {}", path.display()))
    }

    /// Returns the filename for this item: `{id}-{slug}.md`
    pub fn filename(&self) -> String {
        let slug = slugify(&self.frontmatter.title);
        if slug.is_empty() {
            format!("{}.md", self.frontmatter.id)
        } else {
            format!("{}-{slug}.md", self.frontmatter.id)
        }
    }

    /// Returns the ID
    pub fn id(&self) -> &str {
        &self.frontmatter.id
    }

    /// Returns the title
    pub fn title(&self) -> &str {
        &self.frontmatter.title
    }

    /// Returns the status
    pub const fn status(&self) -> Status {
        self.frontmatter.status
    }

    /// Returns the author
    pub fn author(&self) -> &str {
        &self.frontmatter.author
    }

    /// Returns the labels
    pub fn labels(&self) -> &[String] {
        &self.frontmatter.labels
    }

    /// Returns the category
    pub fn category(&self) -> Option<&str> {
        self.frontmatter.category.as_deref()
    }

    /// Returns the creation timestamp
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.frontmatter.created_at
    }

    /// Sets the status
    pub fn set_status(&mut self, status: Status) {
        self.frontmatter.status = status;
    }

    /// Sets the title
    pub fn set_title(&mut self, title: String) {
        self.frontmatter.title = title;
    }

    /// Sets the category
    pub fn set_category(&mut self, category: Option<String>) {
        self.frontmatter.category = category;
    }

    /// Adds a label
    pub fn add_label(&mut self, label: String) {
        if !self.frontmatter.labels.contains(&label) {
            self.frontmatter.labels.push(label);
        }
    }

    /// Removes a label
    pub fn remove_label(&mut self, label: &str) {
        self.frontmatter.labels.retain(|l| l != label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filename_generation() {
        let fm = Frontmatter {
            id: "260109-02F7K9M".to_string(),
            title: "Fix Login Bug".to_string(),
            author: "Test".to_string(),
            created_at: Utc::now(),
            status: Status::Open,
            labels: vec![],
            category: None,
        };
        let item = Item::new(fm);
        assert_eq!(item.filename(), "260109-02F7K9M-fix-login-bug.md");
    }

    #[test]
    fn test_filename_empty_title() {
        let fm = Frontmatter {
            id: "260109-02F7K9M".to_string(),
            title: "!!!".to_string(), // Results in empty slug
            author: "Test".to_string(),
            created_at: Utc::now(),
            status: Status::Open,
            labels: vec![],
            category: None,
        };
        let item = Item::new(fm);
        assert_eq!(item.filename(), "260109-02F7K9M.md");
    }

    #[test]
    fn test_status_display() {
        assert_eq!(Status::Open.to_string(), "open");
        assert_eq!(Status::Closed.to_string(), "closed");
    }
}
