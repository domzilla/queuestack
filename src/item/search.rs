//! # Search and Filter
//!
//! Item search and filtering logic. This module provides the single source of truth
//! for all item filtering operations, used by both CLI commands and TUI.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use super::Item;

// =============================================================================
// Filter Criteria
// =============================================================================

/// Unified filter criteria for item filtering.
///
/// Used by both CLI commands (`qstack list`) and TUI filter overlay.
/// All fields are optional - empty/None means "match all".
#[derive(Debug, Clone, Default)]
pub struct FilterCriteria {
    /// Text search query (matches title, ID, and optionally body).
    pub search: String,
    /// Labels to filter by (OR logic - item must have ANY of these).
    pub labels: Vec<String>,
    /// Category to filter by (exact match, case-insensitive).
    pub category: Option<String>,
    /// Author to filter by (substring match, case-insensitive).
    pub author: Option<String>,
}

impl FilterCriteria {
    /// Creates empty filter criteria (matches everything).
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if no filters are active.
    pub fn is_empty(&self) -> bool {
        self.search.is_empty()
            && self.labels.is_empty()
            && self.category.is_none()
            && self.author.is_none()
    }
}

// =============================================================================
// Filter Matching
// =============================================================================

/// Checks if an item matches the filter criteria.
///
/// This is the canonical filtering logic used by both CLI and TUI.
///
/// # Arguments
/// * `item` - The item to check
/// * `criteria` - Filter criteria to match against
/// * `item_category` - The item's category (derived from path, not stored in item)
pub fn matches_filter(item: &Item, criteria: &FilterCriteria, item_category: Option<&str>) -> bool {
    // Search filter (always full-text in filter context)
    if !criteria.search.is_empty()
        && !matches_search_text(item.title(), item.id(), &item.body, &criteria.search)
    {
        return false;
    }

    // Label filter (OR logic - item must have ANY of the specified labels)
    if !criteria.labels.is_empty() && !matches_labels(item, &criteria.labels) {
        return false;
    }

    // Category filter (case-insensitive, with "uncategorized" special case)
    if let Some(ref filter_cat) = criteria.category {
        if !matches_category(item_category, filter_cat) {
            return false;
        }
    }

    // Author filter (case-insensitive substring match)
    if let Some(ref filter_author) = criteria.author {
        if !matches_author(item, filter_author) {
            return false;
        }
    }

    true
}

// =============================================================================
// Individual Filter Predicates (public for TUI reuse)
// =============================================================================

/// Checks if text fields match search query (case-insensitive).
///
/// Returns true if any of title, ID, or body contain the query.
pub fn matches_search_text(title: &str, id: &str, body: &str, query: &str) -> bool {
    let query_lower = query.to_lowercase();

    title.to_lowercase().contains(&query_lower)
        || id.to_lowercase().contains(&query_lower)
        || body.to_lowercase().contains(&query_lower)
}

/// Checks if item has ANY of the specified labels (OR logic, case-insensitive).
pub fn matches_any_label(item_labels: &[String], filter_labels: &[String]) -> bool {
    filter_labels
        .iter()
        .any(|filter| item_labels.iter().any(|l| l.eq_ignore_ascii_case(filter)))
}

/// Checks if item's category matches the filter (case-insensitive).
///
/// Handles "uncategorized" as a special case for items with no category.
pub fn matches_category_filter(item_category: Option<&str>, filter_category: &str) -> bool {
    item_category.map_or_else(
        || filter_category.eq_ignore_ascii_case("uncategorized"),
        |cat| cat.eq_ignore_ascii_case(filter_category),
    )
}

/// Checks if author matches filter (case-insensitive substring).
pub fn matches_author_filter(item_author: &str, filter_author: &str) -> bool {
    item_author
        .to_lowercase()
        .contains(&filter_author.to_lowercase())
}

// Internal wrappers for Item
fn matches_labels(item: &Item, labels: &[String]) -> bool {
    matches_any_label(item.labels(), labels)
}

fn matches_category(item_category: Option<&str>, filter_category: &str) -> bool {
    matches_category_filter(item_category, filter_category)
}

fn matches_author(item: &Item, filter_author: &str) -> bool {
    matches_author_filter(item.author(), filter_author)
}

// =============================================================================
// Simple Query Matching (for search command)
// =============================================================================

/// Check if an item matches the search query (case-insensitive).
///
/// Searches the item's title and ID. When `full_text` is true,
/// also searches the body content.
///
/// This is a simpler interface for the search command. For full filtering
/// with labels/category/author, use `matches_filter()` instead.
pub fn matches_query(item: &Item, query: &str, full_text: bool) -> bool {
    let query_lower = query.to_lowercase();

    // Always search title
    if item.title().to_lowercase().contains(&query_lower) {
        return true;
    }

    // Always search ID
    if item.id().to_lowercase().contains(&query_lower) {
        return true;
    }

    // Optionally search body
    if full_text && item.body.to_lowercase().contains(&query_lower) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::item::{Frontmatter, Status};

    fn sample_item(title: &str, body: &str) -> Item {
        let frontmatter = Frontmatter {
            id: "260109-02F7K9M".to_string(),
            title: title.to_string(),
            author: "Test".to_string(),
            created_at: Utc::now(),
            status: Status::Open,
            labels: vec![],
            attachments: vec![],
        };
        let mut item = Item::new(frontmatter);
        item.body = body.to_string();
        item
    }

    #[test]
    fn test_matches_title() {
        let item = sample_item("Fix Login Bug", "");
        assert!(matches_query(&item, "login", false));
        assert!(matches_query(&item, "LOGIN", false)); // case insensitive
        assert!(matches_query(&item, "LoGiN", false)); // mixed case
        assert!(matches_query(&item, "bug", false));
    }

    #[test]
    fn test_matches_id() {
        let item = sample_item("Some Title", "");
        assert!(matches_query(&item, "260109", false));
        assert!(matches_query(&item, "02f7k9m", false)); // case insensitive
    }

    #[test]
    fn test_no_match() {
        let item = sample_item("Fix Login Bug", "Some body content");
        assert!(!matches_query(&item, "xyz", false));
        assert!(!matches_query(&item, "body", false)); // not full_text
    }

    #[test]
    fn test_full_text_matches_body() {
        let item = sample_item("Title", "The error occurs in production");
        assert!(!matches_query(&item, "production", false));
        assert!(matches_query(&item, "production", true));
    }

    #[test]
    fn test_full_text_still_matches_title_and_id() {
        let item = sample_item("Important Task", "Body text");
        assert!(matches_query(&item, "important", true));
        assert!(matches_query(&item, "260109", true));
    }
}
