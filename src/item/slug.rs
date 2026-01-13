//! # Slugification
//!
//! Converts titles into URL-safe, filesystem-friendly slugs.
//!
//! ## Rules
//! 1. Convert to lowercase (Unicode-aware)
//! 2. Replace non-alphanumeric characters with hyphens
//! 3. Collapse multiple hyphens
//! 4. Trim hyphens from start/end
//! 5. Truncate to 50 characters
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use crate::constants::MAX_SLUG_LENGTH;

/// Converts a title string into a URL-safe slug.
///
/// Supports full UTF-8: umlauts, CJK characters, and other Unicode are preserved.
///
/// # Arguments
/// * `title` - The original title string
///
/// # Returns
/// A slugified `String` suitable for filenames
///
/// # Example
/// ```
/// use qstack::item::slug::slugify;
/// assert_eq!(slugify("Fix Login Bug!"), "fix-login-bug");
/// assert_eq!(slugify("Über Änderung"), "über-änderung");
/// ```
pub fn slugify(title: &str) -> String {
    let mut result = String::with_capacity(title.len());
    let mut prev_was_hyphen = true; // Start true to trim leading hyphens

    for c in title.chars() {
        if c.is_alphanumeric() {
            // Unicode-aware lowercase (handles umlauts, etc.)
            for lower in c.to_lowercase() {
                result.push(lower);
            }
            prev_was_hyphen = false;
        } else if !prev_was_hyphen {
            result.push('-');
            prev_was_hyphen = true;
        }
    }

    // Trim trailing hyphen
    if result.ends_with('-') {
        result.pop();
    }

    // Truncate to max length (character count, not bytes)
    let char_count = result.chars().count();
    if char_count > MAX_SLUG_LENGTH {
        // Find a good truncation point (prefer word boundary)
        let truncate_at = result
            .char_indices()
            .take(MAX_SLUG_LENGTH)
            .collect::<Vec<_>>();

        // Try to find last hyphen within the range
        let byte_end = truncate_at.last().map_or(0, |(i, c)| i + c.len_utf8());
        let truncated = &result[..byte_end];

        if let Some(last_hyphen) = truncated.rfind('-') {
            if last_hyphen > byte_end / 2 {
                result.truncate(last_hyphen);
            } else {
                result.truncate(byte_end);
            }
        } else {
            result.truncate(byte_end);
        }

        // Trim any trailing hyphen from truncation
        if result.ends_with('-') {
            result.pop();
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_slug() {
        assert_eq!(slugify("Fix Login Bug"), "fix-login-bug");
    }

    #[test]
    fn test_special_characters() {
        assert_eq!(slugify("Add dark-mode!!!"), "add-dark-mode");
    }

    #[test]
    fn test_multiple_spaces() {
        assert_eq!(slugify("This   has   spaces"), "this-has-spaces");
    }

    #[test]
    fn test_leading_trailing() {
        assert_eq!(slugify("  --Title--  "), "title");
    }

    #[test]
    fn test_unicode_preserved() {
        // UTF-8 characters are now preserved
        assert_eq!(slugify("Café résumé"), "café-résumé");
        assert_eq!(slugify("Über Änderung"), "über-änderung");
        assert_eq!(slugify("日本語タイトル"), "日本語タイトル");
        assert_eq!(slugify("한글 제목"), "한글-제목");
        assert_eq!(slugify("العربية"), "العربية");
    }

    #[test]
    fn test_numbers() {
        assert_eq!(slugify("Bug #123 in v2.0"), "bug-123-in-v2-0");
    }

    #[test]
    fn test_truncation() {
        let long_title = "This is a very long title that should be truncated to fifty characters";
        let slug = slugify(long_title);
        assert!(slug.len() <= MAX_SLUG_LENGTH);
        assert!(!slug.ends_with('-'));
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(slugify(""), "");
    }

    #[test]
    fn test_only_special_chars() {
        assert_eq!(slugify("!@#$%"), "");
    }
}
