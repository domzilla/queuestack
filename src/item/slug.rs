//! # Slugification
//!
//! Converts titles into URL-safe, filesystem-friendly slugs.
//!
//! ## Rules
//! 1. Convert to lowercase
//! 2. Replace non-alphanumeric characters with hyphens
//! 3. Collapse multiple hyphens
//! 4. Trim hyphens from start/end
//! 5. Truncate to 50 characters
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

/// Maximum slug length
const MAX_SLUG_LENGTH: usize = 50;

/// Converts a title string into a URL-safe slug.
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
/// ```
pub fn slugify(title: &str) -> String {
    let mut result = String::with_capacity(title.len());
    let mut prev_was_hyphen = true; // Start true to trim leading hyphens

    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            result.push(c.to_ascii_lowercase());
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

    // Truncate to max length, but don't cut in middle of word if possible
    if result.len() > MAX_SLUG_LENGTH {
        result.truncate(MAX_SLUG_LENGTH);
        // If we cut in the middle of something, try to find last hyphen
        if let Some(last_hyphen) = result.rfind('-') {
            if last_hyphen > MAX_SLUG_LENGTH / 2 {
                result.truncate(last_hyphen);
            }
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
    fn test_unicode() {
        assert_eq!(slugify("Café résumé"), "caf-r-sum");
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
