//! Test utilities for generating random IDs and test data

use rand::Rng;
use shp_types::Hash;

/// Generate a random 32-byte Hash for use in tests
pub fn random_hash() -> Hash {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes);
    Hash::from(bytes)
}

/// Generate a random 32-byte array for use in tests
pub fn random_bytes_32() -> [u8; 32] {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes);
    bytes
}
