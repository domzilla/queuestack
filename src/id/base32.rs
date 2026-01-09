//! # Base32 Encoding
//!
//! Crockford's Base32 encoding for human-readable, URL-safe identifiers.
//! Uses alphabet: 0-9, A-Z excluding I, L, O, U (32 characters).
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

/// Crockford's Base32 alphabet (excludes I, L, O, U for readability)
const CROCKFORD_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Encodes a u64 value into a fixed-width Crockford Base32 string.
///
/// # Arguments
/// * `value` - The number to encode
/// * `width` - The desired output width (left-padded with '0')
///
/// # Returns
/// A `String` of exactly `width` characters
pub fn encode(mut value: u64, width: usize) -> String {
    let mut result = Vec::with_capacity(width);

    for _ in 0..width {
        let idx = (value % 32) as usize;
        result.push(CROCKFORD_ALPHABET[idx]);
        value /= 32;
    }

    result.reverse();
    // SAFETY: CROCKFORD_ALPHABET contains only ASCII characters
    String::from_utf8(result).expect("Base32 alphabet is valid UTF-8")
}

/// Encodes random bytes into a Crockford Base32 string.
///
/// # Arguments
/// * `bytes` - Random bytes to encode
/// * `width` - The desired output width
///
/// # Returns
/// A `String` of exactly `width` characters
pub fn encode_bytes(bytes: &[u8], width: usize) -> String {
    let mut result = String::with_capacity(width);

    for byte in bytes.iter().take(width) {
        let idx = (*byte as usize) % 32;
        result.push(CROCKFORD_ALPHABET[idx] as char);
    }

    // Pad if needed
    while result.len() < width {
        result.push('0');
    }

    result.truncate(width);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_zero() {
        assert_eq!(encode(0, 4), "0000");
    }

    #[test]
    fn test_encode_max_seconds() {
        // 86399 seconds (23:59:59) should fit in 4 chars
        let result = encode(86399, 4);
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_encode_width() {
        assert_eq!(encode(1, 1), "1");
        assert_eq!(encode(1, 4), "0001");
        assert_eq!(encode(32, 4), "0010");
    }

    #[test]
    fn test_alphabet_excludes_confusing_chars() {
        let alphabet = String::from_utf8_lossy(CROCKFORD_ALPHABET);
        assert!(!alphabet.contains('I'));
        assert!(!alphabet.contains('L'));
        assert!(!alphabet.contains('O'));
        assert!(!alphabet.contains('U'));
    }

    #[test]
    fn test_encode_bytes() {
        let bytes = [0u8, 1, 2, 31];
        let result = encode_bytes(&bytes, 4);
        assert_eq!(result.len(), 4);
        assert_eq!(&result[0..1], "0");
        assert_eq!(&result[1..2], "1");
    }
}
