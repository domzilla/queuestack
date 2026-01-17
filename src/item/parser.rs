//! # YAML Frontmatter Parser
//!
//! Parses and serializes Markdown files with YAML frontmatter.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use anyhow::{Context, Result};

use super::Frontmatter;
use crate::constants::FRONTMATTER_DELIMITER;

/// Parses a Markdown file with YAML frontmatter.
///
/// # Arguments
/// * `content` - The full file content
///
/// # Returns
/// A tuple of (Frontmatter, body markdown)
pub fn parse(content: &str) -> Result<(Frontmatter, String)> {
    let content = content.trim_start();

    // Check for frontmatter start
    if !content.starts_with(FRONTMATTER_DELIMITER) {
        anyhow::bail!("File does not start with YAML frontmatter (---)");
    }

    // Find the closing delimiter
    let after_start = &content[FRONTMATTER_DELIMITER.len()..];
    let end_pos = after_start
        .find(&format!("\n{FRONTMATTER_DELIMITER}"))
        .ok_or_else(|| anyhow::anyhow!("No closing frontmatter delimiter found"))?;

    let yaml_content = &after_start[..end_pos];
    let body_start = end_pos + 1 + FRONTMATTER_DELIMITER.len();
    let body = after_start
        .get(body_start..)
        .unwrap_or("")
        .trim_start_matches(['\n', '\r'])
        .to_string();

    let frontmatter: Frontmatter =
        serde_yml::from_str(yaml_content).context("Failed to parse YAML frontmatter")?;

    Ok((frontmatter, body))
}

/// Serializes frontmatter and body back to Markdown format.
///
/// # Arguments
/// * `frontmatter` - The YAML frontmatter data
/// * `body` - The Markdown body content
///
/// # Returns
/// The complete file content as a String
pub fn serialize(frontmatter: &Frontmatter, body: &str) -> Result<String> {
    let yaml = serde_yml::to_string(frontmatter).context("Failed to serialize frontmatter")?;

    let mut result = String::new();
    result.push_str(FRONTMATTER_DELIMITER);
    result.push('\n');
    result.push_str(&yaml);
    result.push_str(FRONTMATTER_DELIMITER);
    result.push_str("\n\n\n"); // Two empty lines after frontmatter

    if !body.is_empty() {
        result.push_str(body);
        if !body.ends_with('\n') {
            result.push('\n');
        }
    }

    Ok(result)
}

/// Creates a new item file content with minimal template.
pub fn create_template(frontmatter: &Frontmatter) -> Result<String> {
    serialize(frontmatter, "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_frontmatter() -> Frontmatter {
        Frontmatter {
            id: "260109-02F7K9M".to_string(),
            title: "Test Item".to_string(),
            author: "Test Author".to_string(),
            created_at: Utc::now(),
            status: super::super::Status::Open,
            labels: vec!["bug".to_string()],
            attachments: vec![],
        }
    }

    #[test]
    fn test_roundtrip() {
        let fm = sample_frontmatter();
        let body = "This is the description.\n\nWith multiple paragraphs.";

        let serialized = serialize(&fm, body).unwrap();
        let (parsed_fm, parsed_body) = parse(&serialized).unwrap();

        assert_eq!(parsed_fm.id, fm.id);
        assert_eq!(parsed_fm.title, fm.title);
        assert_eq!(parsed_body.trim(), body.trim());
    }

    #[test]
    fn test_parse_empty_body() {
        let fm = sample_frontmatter();
        let serialized = serialize(&fm, "").unwrap();
        let (_, body) = parse(&serialized).unwrap();
        assert!(body.is_empty());
    }

    #[test]
    fn test_missing_frontmatter() {
        let result = parse("No frontmatter here");
        assert!(result.is_err());
    }

    #[test]
    fn test_unclosed_frontmatter() {
        let result = parse("---\nid: test\n");
        assert!(result.is_err());
    }
}
