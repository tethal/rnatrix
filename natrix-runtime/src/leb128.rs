//! LEB128 (Little Endian Base 128) encoding and decoding.
//!
//! This module provides efficient variable-length integer encoding used by the bytecode format.
//! Functions are generic over the byte-fetching mechanism to allow both safe and unsafe usage.

/// Decodes an unsigned LEB128 value by repeatedly calling the provided closure to fetch bytes.
///
/// The closure is called once per byte until a byte with the high bit unset is encountered.
/// The caller is responsible for advancing any offset/pointer within the closure.
///
/// # Example
/// ```
/// # use natrix_runtime::leb128::decode_uleb128;
/// let bytes = [0xe5, 0x8e, 0x26]; // 624485 in ULEB128
/// let mut offset = 0;
/// let value = decode_uleb128(|| {
///     let b = bytes[offset];
///     offset += 1;
///     b
/// });
/// assert_eq!(value, 624485);
/// assert_eq!(offset, 3);
/// ```
#[inline(always)]
pub fn decode_uleb128<F>(mut fetch: F) -> usize
where
    F: FnMut() -> u8,
{
    let mut result = 0usize;
    let mut shift = 0;
    loop {
        let byte = fetch();
        result |= ((byte & 0x7f) as usize) << shift;
        shift += 7;
        if (byte & 0x80) == 0 {
            break;
        }
    }
    result
}

/// Decodes a signed LEB128 value by repeatedly calling the provided closure to fetch bytes.
///
/// The closure is called once per byte until a byte with the high bit unset is encountered.
/// Sign extension is performed if the final byte has bit 6 set.
/// The caller is responsible for advancing any offset/pointer within the closure.
///
/// # Example
/// ```
/// # use natrix_runtime::leb128::decode_sleb128;
/// let bytes = [0x9b, 0xf1, 0x59]; // -624485 in SLEB128
/// let mut offset = 0;
/// let value = decode_sleb128(|| {
///     let b = bytes[offset];
///     offset += 1;
///     b
/// });
/// assert_eq!(value, -624485);
/// assert_eq!(offset, 3);
/// ```
#[inline(always)]
pub fn decode_sleb128<F>(mut fetch: F) -> i64
where
    F: FnMut() -> u8,
{
    let mut result = 0i64;
    let mut shift = 0;
    let mut byte;
    loop {
        byte = fetch();
        result |= ((byte & 0x7f) as i64) << shift;
        shift += 7;
        if (byte & 0x80) == 0 {
            break;
        }
    }
    // Sign extend if the sign bit (bit 6) is set in the last byte
    if shift < 64 && (byte & 0x40) != 0 {
        result |= !0i64 << shift;
    }
    result
}

/// Encodes an unsigned value as ULEB128 by repeatedly calling the provided closure with each byte.
///
/// The closure is called once per encoded byte (1-10 bytes for usize on 64-bit platforms).
/// The caller is responsible for storing the bytes (e.g., pushing to a Vec).
///
/// # Example
/// ```
/// # use natrix_runtime::leb128::encode_uleb128;
/// let mut output = Vec::new();
/// encode_uleb128(624485, |byte| output.push(byte));
/// assert_eq!(output, [0xe5, 0x8e, 0x26]);
/// ```
#[inline(always)]
pub fn encode_uleb128<F>(mut value: usize, mut emit: F)
where
    F: FnMut(u8),
{
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80; // Set continuation bit
        }
        emit(byte);
        if value == 0 {
            break;
        }
    }
}

/// Encodes a signed value as SLEB128 by repeatedly calling the provided closure with each byte.
///
/// The closure is called once per encoded byte (1-10 bytes for i64).
/// The caller is responsible for storing the bytes (e.g., pushing to a Vec).
///
/// # Example
/// ```
/// # use natrix_runtime::leb128::encode_sleb128;
/// let mut output = Vec::new();
/// encode_sleb128(-624485, |byte| output.push(byte));
/// assert_eq!(output, [0x9b, 0xf1, 0x59]);
/// ```
#[inline(always)]
pub fn encode_sleb128<F>(mut value: i64, mut emit: F)
where
    F: FnMut(u8),
{
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;

        // Check if we're done: remaining bits must all match the sign bit of this byte
        let sign_bit = (byte & 0x40) != 0;
        let done = (value == 0 && !sign_bit) || (value == -1 && sign_bit);

        if !done {
            byte |= 0x80; // Set continuation bit
        }
        emit(byte);

        if done {
            break;
        }
    }
}

/// Computes the number of bytes required to encode a value as ULEB128.
///
/// # Example
/// ```
/// # use natrix_runtime::leb128::uleb128_len;
/// assert_eq!(uleb128_len(0), 1);
/// assert_eq!(uleb128_len(127), 1);
/// assert_eq!(uleb128_len(128), 2);
/// assert_eq!(uleb128_len(624485), 3);
/// ```
#[inline]
pub const fn uleb128_len(mut value: usize) -> usize {
    if value == 0 {
        return 1;
    }
    let mut len = 0;
    while value != 0 {
        value >>= 7;
        len += 1;
    }
    len
}

/// Computes the number of bytes required to encode a value as SLEB128.
///
/// # Example
/// ```
/// # use natrix_runtime::leb128::sleb128_len;
/// assert_eq!(sleb128_len(0), 1);
/// assert_eq!(sleb128_len(-1), 1);
/// assert_eq!(sleb128_len(63), 1);
/// assert_eq!(sleb128_len(-64), 1);
/// assert_eq!(sleb128_len(64), 2);
/// assert_eq!(sleb128_len(-65), 2);
/// assert_eq!(sleb128_len(624485), 3);
/// assert_eq!(sleb128_len(-624485), 3);
/// ```
#[inline]
pub const fn sleb128_len(value: i64) -> usize {
    let mut len = 0;
    let mut val = value;

    loop {
        let byte = (val & 0x7f) as u8;
        val >>= 7;
        len += 1;

        // Check if we're done: remaining bits must all match the sign bit of this byte
        let sign_bit = (byte & 0x40) != 0;
        let done = (val == 0 && !sign_bit) || (val == -1 && sign_bit);

        if done {
            break;
        }
    }

    len
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to decode from a byte slice
    fn decode_uleb_from_slice(bytes: &[u8]) -> (usize, usize) {
        let mut offset = 0;
        let value = decode_uleb128(|| {
            let b = bytes[offset];
            offset += 1;
            b
        });
        (value, offset)
    }

    fn decode_sleb_from_slice(bytes: &[u8]) -> (i64, usize) {
        let mut offset = 0;
        let value = decode_sleb128(|| {
            let b = bytes[offset];
            offset += 1;
            b
        });
        (value, offset)
    }

    // Helper to encode to a Vec
    fn encode_uleb_to_vec(value: usize) -> Vec<u8> {
        let mut output = Vec::new();
        encode_uleb128(value, |byte| output.push(byte));
        output
    }

    fn encode_sleb_to_vec(value: i64) -> Vec<u8> {
        let mut output = Vec::new();
        encode_sleb128(value, |byte| output.push(byte));
        output
    }

    #[test]
    fn test_uleb128_small_values() {
        // Single byte encoding (0-127)
        assert_eq!(encode_uleb_to_vec(0), [0x00]);
        assert_eq!(encode_uleb_to_vec(1), [0x01]);
        assert_eq!(encode_uleb_to_vec(127), [0x7f]);

        assert_eq!(decode_uleb_from_slice(&[0x00]), (0, 1));
        assert_eq!(decode_uleb_from_slice(&[0x01]), (1, 1));
        assert_eq!(decode_uleb_from_slice(&[0x7f]), (127, 1));
    }

    #[test]
    fn test_uleb128_multi_byte() {
        // Two byte encoding
        assert_eq!(encode_uleb_to_vec(128), [0x80, 0x01]);
        assert_eq!(encode_uleb_to_vec(300), [0xac, 0x02]);

        assert_eq!(decode_uleb_from_slice(&[0x80, 0x01]), (128, 2));
        assert_eq!(decode_uleb_from_slice(&[0xac, 0x02]), (300, 2));

        // Three byte encoding
        assert_eq!(encode_uleb_to_vec(624485), [0xe5, 0x8e, 0x26]);
        assert_eq!(decode_uleb_from_slice(&[0xe5, 0x8e, 0x26]), (624485, 3));
    }

    #[test]
    fn test_uleb128_roundtrip() {
        let test_values = [0, 1, 127, 128, 255, 256, 16383, 16384, 624485, usize::MAX];

        for &value in &test_values {
            let encoded = encode_uleb_to_vec(value);
            let (decoded, bytes_read) = decode_uleb_from_slice(&encoded);
            assert_eq!(decoded, value, "Failed roundtrip for {}", value);
            assert_eq!(bytes_read, encoded.len());
        }
    }

    #[test]
    fn test_sleb128_positive_values() {
        // Small positive values
        assert_eq!(encode_sleb_to_vec(0), [0x00]);
        assert_eq!(encode_sleb_to_vec(1), [0x01]);
        assert_eq!(encode_sleb_to_vec(63), [0x3f]);

        assert_eq!(decode_sleb_from_slice(&[0x00]), (0, 1));
        assert_eq!(decode_sleb_from_slice(&[0x01]), (1, 1));
        assert_eq!(decode_sleb_from_slice(&[0x3f]), (63, 1));

        // Two byte positive
        assert_eq!(encode_sleb_to_vec(64), [0xc0, 0x00]);
        assert_eq!(decode_sleb_from_slice(&[0xc0, 0x00]), (64, 2));
    }

    #[test]
    fn test_sleb128_negative_values() {
        // Small negative values
        assert_eq!(encode_sleb_to_vec(-1), [0x7f]);
        assert_eq!(encode_sleb_to_vec(-64), [0x40]);

        assert_eq!(decode_sleb_from_slice(&[0x7f]), (-1, 1));
        assert_eq!(decode_sleb_from_slice(&[0x40]), (-64, 1));

        // Two byte negative
        assert_eq!(encode_sleb_to_vec(-65), [0xbf, 0x7f]);
        assert_eq!(decode_sleb_from_slice(&[0xbf, 0x7f]), (-65, 2));

        // Three byte negative
        assert_eq!(encode_sleb_to_vec(-624485), [0x9b, 0xf1, 0x59]);
        assert_eq!(decode_sleb_from_slice(&[0x9b, 0xf1, 0x59]), (-624485, 3));
    }

    #[test]
    fn test_sleb128_roundtrip() {
        let test_values = [
            0, 1, -1, 63, -64, 64, -65, 127, -128,
            624485, -624485,
            i64::MAX, i64::MIN,
        ];

        for &value in &test_values {
            let encoded = encode_sleb_to_vec(value);
            let (decoded, bytes_read) = decode_sleb_from_slice(&encoded);
            assert_eq!(decoded, value, "Failed roundtrip for {}", value);
            assert_eq!(bytes_read, encoded.len());
        }
    }

    #[test]
    fn test_uleb128_len() {
        assert_eq!(uleb128_len(0), 1);
        assert_eq!(uleb128_len(127), 1);
        assert_eq!(uleb128_len(128), 2);
        assert_eq!(uleb128_len(255), 2);
        assert_eq!(uleb128_len(256), 2);
        assert_eq!(uleb128_len(16383), 2);
        assert_eq!(uleb128_len(16384), 3);
        assert_eq!(uleb128_len(624485), 3);

        // Verify length matches actual encoding
        for value in [0, 1, 127, 128, 255, 16383, 16384, 624485, usize::MAX] {
            let encoded = encode_uleb_to_vec(value);
            assert_eq!(uleb128_len(value), encoded.len(), "Length mismatch for {}", value);
        }
    }

    #[test]
    fn test_sleb128_len() {
        assert_eq!(sleb128_len(0), 1);
        assert_eq!(sleb128_len(-1), 1);
        assert_eq!(sleb128_len(63), 1);
        assert_eq!(sleb128_len(-64), 1);
        assert_eq!(sleb128_len(64), 2);
        assert_eq!(sleb128_len(-65), 2);
        assert_eq!(sleb128_len(624485), 3);
        assert_eq!(sleb128_len(-624485), 3);

        // Verify length matches actual encoding
        let test_values = [
            0, 1, -1, 63, -64, 64, -65, 127, -128,
            624485, -624485,
            i64::MAX, i64::MIN,
        ];

        for value in test_values {
            let encoded = encode_sleb_to_vec(value);
            assert_eq!(sleb128_len(value), encoded.len(), "Length mismatch for {}", value);
        }
    }

    #[test]
    fn test_decode_with_trailing_bytes() {
        // Ensure decoder stops at the right place and doesn't consume extra bytes
        let bytes = [0xe5, 0x8e, 0x26, 0xff, 0xff];
        let (value, offset) = decode_uleb_from_slice(&bytes);
        assert_eq!(value, 624485);
        assert_eq!(offset, 3); // Should stop after 3 bytes, not read the trailing 0xff
    }

    #[test]
    fn test_encode_decode_boundary_values() {
        // Test boundary cases for 7-bit chunks
        let boundaries = [
            0x7f,           // Max 1-byte unsigned
            0x80,           // Min 2-byte unsigned
            0x3fff,         // Max 2-byte unsigned
            0x4000,         // Min 3-byte unsigned
        ];

        for &value in &boundaries {
            let encoded = encode_uleb_to_vec(value);
            let (decoded, _) = decode_uleb_from_slice(&encoded);
            assert_eq!(decoded, value);
        }

        let signed_boundaries = [
            63,             // Max 1-byte positive
            64,             // Min 2-byte positive
            -64,            // Min 1-byte negative
            -65,            // Max 2-byte negative
        ];

        for &value in &signed_boundaries {
            let encoded = encode_sleb_to_vec(value);
            let (decoded, _) = decode_sleb_from_slice(&encoded);
            assert_eq!(decoded, value);
        }
    }
}