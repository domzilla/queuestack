//! # ID Generation
//!
//! Generates unique, sortable identifiers using a configurable pattern.
//! Default pattern: `%y%m%d-%T%RRR` (e.g., `260109-02F7K9M`)
//!
//! ## Tokens
//! - `%y`, `%m`, `%d`: Year, Month, Day (2 digits)
//! - `%j`: Day of year (001-366)
//! - `%T`: Base32 time (4 chars, seconds since midnight UTC)
//! - `%R`: Base32 random (count of R determines length)
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

pub mod base32;

use std::fmt::Write;

use chrono::{Datelike, Timelike, Utc};
use rand::Rng;

/// Default ID pattern: YYMMDD-TTTTRRR
pub const DEFAULT_PATTERN: &str = "%y%m%d-%T%RRR";

/// Generates a unique ID based on the given pattern.
///
/// # Arguments
/// * `pattern` - The pattern string with tokens to expand
///
/// # Returns
/// A `String` containing the generated ID
pub fn generate(pattern: &str) -> String {
    let now = Utc::now();
    let mut result = String::with_capacity(pattern.len() + 8);
    let mut chars = pattern.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            match chars.next() {
                Some('y') => {
                    // 2-digit year
                    let _ = write!(result, "{:02}", now.year() % 100);
                }
                Some('m') => {
                    // 2-digit month
                    let _ = write!(result, "{:02}", now.month());
                }
                Some('d') => {
                    // 2-digit day
                    let _ = write!(result, "{:02}", now.day());
                }
                Some('j') => {
                    // Day of year (001-366)
                    let _ = write!(result, "{:03}", now.ordinal());
                }
                Some('T') => {
                    // Base32 time: seconds since midnight UTC (4 chars)
                    let seconds_since_midnight = u64::from(now.hour()) * 3600
                        + u64::from(now.minute()) * 60
                        + u64::from(now.second());
                    result.push_str(&base32::encode(seconds_since_midnight, 4));
                }
                Some('R') => {
                    // Count consecutive R's to determine random length
                    let mut count = 1;
                    while chars.peek() == Some(&'R') {
                        chars.next();
                        count += 1;
                    }
                    // Generate random bytes and encode
                    let mut rng = rand::rng();
                    let random_bytes: Vec<u8> = (0..count).map(|_| rng.random()).collect();
                    result.push_str(&base32::encode_bytes(&random_bytes, count));
                }
                Some('%') | None => {
                    result.push('%');
                }
                Some(other) => {
                    // Unknown token, keep as-is
                    result.push('%');
                    result.push(other);
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_pattern_format() {
        let id = generate(DEFAULT_PATTERN);
        // Format: YYMMDD-TTTTRRR (14 chars: 6 date + 1 hyphen + 4 time + 3 random)
        assert_eq!(id.len(), 14);
        assert_eq!(&id[6..7], "-");
    }

    #[test]
    fn test_date_tokens() {
        let id = generate("%y%m%d");
        assert_eq!(id.len(), 6);
        // Should be numeric
        assert!(id.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_random_length() {
        let id1 = generate("%R");
        let id2 = generate("%RR");
        let id3 = generate("%RRR");
        assert_eq!(id1.len(), 1);
        assert_eq!(id2.len(), 2);
        assert_eq!(id3.len(), 3);
    }

    #[test]
    fn test_literal_passthrough() {
        let id = generate("prefix-%y-suffix");
        assert!(id.starts_with("prefix-"));
        assert!(id.ends_with("-suffix"));
    }

    #[test]
    fn test_escaped_percent() {
        let id = generate("100%%");
        assert_eq!(id, "100%");
    }

    #[test]
    fn test_day_of_year() {
        let id = generate("%j");
        assert_eq!(id.len(), 3);
        let day: u32 = id.parse().unwrap();
        assert!((1..=366).contains(&day));
    }
}
