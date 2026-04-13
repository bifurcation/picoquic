//! Hash functions for picoquic hash tables.
//!
//! This module provides hash functions used by picoquic's hash tables.
//! The hash table structure itself remains in C due to its use of
//! function pointers and intrusive items.
//!
//! Translated from picoquic/picohash.c

use crate::siphash;

// =============================================================================
// Hash functions
// =============================================================================

/// Simple hash function for byte arrays.
///
/// Uses XOR and rotation with a seed-based hash.
pub fn hash_bytes(bytes: &[u8], hash_seed: &[u8; 16]) -> u64 {
    let mut hash: u64 = u64::from(hash_seed[8])
        | (u64::from(hash_seed[9]) << 8)
        | (u64::from(hash_seed[10]) << 16)
        | (u64::from(hash_seed[11]) << 24)
        | (u64::from(hash_seed[12]) << 32)
        | (u64::from(hash_seed[13]) << 40)
        | (u64::from(hash_seed[14]) << 48)
        | (u64::from(hash_seed[15]) << 56);

    let mut rotate: u32 = 11;

    for (i, &byte) in bytes.iter().enumerate() {
        hash ^= u64::from(byte);
        hash ^= u64::from(hash_seed[i & 15]);
        hash ^= hash << 8;
        hash = hash.wrapping_add(hash >> rotate);
        rotate = (hash as u32 & 31) + 11;
    }

    hash ^= hash >> rotate;
    hash
}

/// SipHash-based hash function for byte arrays.
///
/// Uses SipHash-2-4 for cryptographic security against hash flooding.
pub fn hash_siphash(bytes: &[u8], hash_seed: &[u8; 16]) -> u64 {
    siphash::hash_u64(bytes, hash_seed)
}

// =============================================================================
// FFI exports
// =============================================================================

/// FFI export: Simple byte hash function.
///
/// # Safety
/// - `bytes` must point to valid data of `length` bytes (or be non-null if length is 0).
/// - `hash_seed` must point to a valid 16-byte array.
#[no_mangle]
pub unsafe extern "C" fn picohash_bytes(
    bytes: *const u8,
    length: usize,
    hash_seed: *const u8,
) -> u64 {
    let bytes_slice = if length == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(bytes, length)
    };
    let seed_slice = std::slice::from_raw_parts(hash_seed, 16);
    let seed_array: &[u8; 16] = seed_slice.try_into().unwrap();
    hash_bytes(bytes_slice, seed_array)
}

/// FFI export: SipHash-based hash function.
///
/// # Safety
/// - `bytes` must point to valid data of `length` bytes (or be non-null if length is 0).
/// - `hash_seed` must point to a valid 16-byte array.
#[no_mangle]
pub unsafe extern "C" fn picohash_siphash(
    bytes: *const u8,
    length: usize,
    hash_seed: *const u8,
) -> u64 {
    let bytes_slice = if length == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(bytes, length)
    };
    let seed_slice = std::slice::from_raw_parts(hash_seed, 16);
    let seed_array: &[u8; 16] = seed_slice.try_into().unwrap();
    hash_siphash(bytes_slice, seed_array)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SEED: [u8; 16] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0x0f,
    ];

    #[test]
    fn test_hash_bytes_deterministic() {
        let data = b"hello world";
        let h1 = hash_bytes(data, &TEST_SEED);
        let h2 = hash_bytes(data, &TEST_SEED);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_bytes_different_data() {
        let h1 = hash_bytes(b"hello", &TEST_SEED);
        let h2 = hash_bytes(b"world", &TEST_SEED);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_bytes_empty() {
        // Should not panic on empty input
        let h = hash_bytes(&[], &TEST_SEED);
        // Value depends on seed
        assert_ne!(h, 0); // Seed-based initial value
    }

    #[test]
    fn test_hash_siphash_deterministic() {
        let data = b"hello world";
        let h1 = hash_siphash(data, &TEST_SEED);
        let h2 = hash_siphash(data, &TEST_SEED);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_siphash_matches_siphash_module() {
        let data = b"test data";
        let h1 = hash_siphash(data, &TEST_SEED);
        let h2 = siphash::hash_u64(data, &TEST_SEED);
        assert_eq!(h1, h2);
    }
}
