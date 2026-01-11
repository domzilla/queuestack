//! # Item
//!
//! Represents a qstack item (task/issue) with YAML frontmatter metadata.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

pub mod parser;
pub mod search;
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

    /// Attached files (relative paths) and URLs
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<String>,
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

    /// Returns the directory containing this item (and its attachments).
    ///
    /// Returns `None` if the item has no path set.
    pub fn attachment_dir(&self) -> Option<&Path> {
        self.path.as_ref().and_then(|p| p.parent())
    }

    /// Returns the attachments
    pub fn attachments(&self) -> &[String] {
        &self.frontmatter.attachments
    }

    /// Adds an attachment
    pub fn add_attachment(&mut self, attachment: String) {
        self.frontmatter.attachments.push(attachment);
    }

    /// Removes an attachment by index (0-based)
    ///
    /// Returns the removed attachment, or None if index out of bounds
    pub fn remove_attachment(&mut self, index: usize) -> Option<String> {
        if index < self.frontmatter.attachments.len() {
            Some(self.frontmatter.attachments.remove(index))
        } else {
            None
        }
    }

    /// Returns the next attachment counter for this item
    ///
    /// Parses existing attachment filenames to find the highest counter and returns max + 1.
    /// Uses `AttachmentFileName::parse()` as the single source of truth for the naming convention.
    pub fn next_attachment_counter(&self) -> u32 {
        use crate::storage::AttachmentFileName;

        self.frontmatter
            .attachments
            .iter()
            .filter(|a| !is_url(a))
            .filter_map(|a| AttachmentFileName::parse(a))
            .map(|af| af.counter)
            .max()
            .map_or(1, |n| n + 1)
    }
}

/// Checks if a string is a URL (starts with http:// or https://)
pub fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_frontmatter(id: &str) -> Frontmatter {
        Frontmatter {
            id: id.to_string(),
            title: "Test Item".to_string(),
            author: "Test".to_string(),
            created_at: Utc::now(),
            status: Status::Open,
            labels: vec![],
            category: None,
            attachments: vec![],
        }
    }

    #[test]
    fn test_filename_generation() {
        let mut fm = sample_frontmatter("260109-02F7K9M");
        fm.title = "Fix Login Bug".to_string();
        let item = Item::new(fm);
        assert_eq!(item.filename(), "260109-02F7K9M-fix-login-bug.md");
    }

    #[test]
    fn test_filename_empty_title() {
        let mut fm = sample_frontmatter("260109-02F7K9M");
        fm.title = "!!!".to_string(); // Results in empty slug
        let item = Item::new(fm);
        assert_eq!(item.filename(), "260109-02F7K9M.md");
    }

    #[test]
    fn test_status_display() {
        assert_eq!(Status::Open.to_string(), "open");
        assert_eq!(Status::Closed.to_string(), "closed");
    }

    // ==========================================================================
    // Attachment Tests
    // ==========================================================================

    #[test]
    fn test_is_url_http() {
        assert!(is_url("http://example.com"));
        assert!(is_url("http://example.com/path?query=1"));
    }

    #[test]
    fn test_is_url_https() {
        assert!(is_url("https://example.com"));
        assert!(is_url("https://github.com/user/repo/issues/42"));
    }

    #[test]
    fn test_is_url_file_path_returns_false() {
        assert!(!is_url("260109-XXX-Attachment-1-screenshot.png"));
        assert!(!is_url("/path/to/file.txt"));
        assert!(!is_url("relative/path.md"));
        assert!(!is_url("ftp://example.com")); // Only http(s) are URLs
    }

    #[test]
    fn test_attachments_getter() {
        let mut fm = sample_frontmatter("260109-AAA");
        fm.attachments = vec!["file.txt".to_string(), "https://example.com".to_string()];
        let item = Item::new(fm);
        assert_eq!(item.attachments().len(), 2);
        assert_eq!(item.attachments()[0], "file.txt");
    }

    #[test]
    fn test_add_attachment() {
        let fm = sample_frontmatter("260109-AAA");
        let mut item = Item::new(fm);
        assert!(item.attachments().is_empty());

        item.add_attachment("260109-AAA-Attachment-1-test.txt".to_string());
        assert_eq!(item.attachments().len(), 1);

        item.add_attachment("https://example.com".to_string());
        assert_eq!(item.attachments().len(), 2);
    }

    #[test]
    fn test_remove_attachment_valid_index() {
        let mut fm = sample_frontmatter("260109-AAA");
        fm.attachments = vec![
            "file1.txt".to_string(),
            "file2.txt".to_string(),
            "file3.txt".to_string(),
        ];
        let mut item = Item::new(fm);

        let removed = item.remove_attachment(1);
        assert_eq!(removed, Some("file2.txt".to_string()));
        assert_eq!(item.attachments().len(), 2);
        assert_eq!(item.attachments()[0], "file1.txt");
        assert_eq!(item.attachments()[1], "file3.txt");
    }

    #[test]
    fn test_remove_attachment_invalid_index() {
        let fm = sample_frontmatter("260109-AAA");
        let mut item = Item::new(fm);
        assert_eq!(item.remove_attachment(0), None);
        assert_eq!(item.remove_attachment(100), None);
    }

    #[test]
    fn test_next_counter_empty_attachments() {
        let fm = sample_frontmatter("260109-AAA");
        let item = Item::new(fm);
        assert_eq!(item.next_attachment_counter(), 1);
    }

    #[test]
    fn test_next_counter_with_existing() {
        let mut fm = sample_frontmatter("260109-AAA");
        fm.attachments = vec![
            "260109-AAA-Attachment-1-file.txt".to_string(),
            "260109-AAA-Attachment-2-image.png".to_string(),
        ];
        let item = Item::new(fm);
        assert_eq!(item.next_attachment_counter(), 3);
    }

    #[test]
    fn test_next_counter_with_gaps() {
        let mut fm = sample_frontmatter("260109-AAA");
        fm.attachments = vec![
            "260109-AAA-Attachment-1-file.txt".to_string(),
            "260109-AAA-Attachment-5-image.png".to_string(), // Gap: 2,3,4 missing
        ];
        let item = Item::new(fm);
        assert_eq!(item.next_attachment_counter(), 6);
    }

    #[test]
    fn test_next_counter_ignores_urls() {
        let mut fm = sample_frontmatter("260109-AAA");
        fm.attachments = vec![
            "260109-AAA-Attachment-2-file.txt".to_string(),
            "https://example.com".to_string(),
            "http://test.com/page".to_string(),
        ];
        let item = Item::new(fm);
        assert_eq!(item.next_attachment_counter(), 3);
    }

    #[test]
    fn test_next_counter_urls_only() {
        let mut fm = sample_frontmatter("260109-AAA");
        fm.attachments = vec![
            "https://example.com".to_string(),
            "http://test.com".to_string(),
        ];
        let item = Item::new(fm);
        assert_eq!(item.next_attachment_counter(), 1);
    }
}
