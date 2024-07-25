#![cfg_attr(not(feature = "std"), no_std)]

use sp_core::Hasher;
use sp_runtime::traits::BlakeTwo256;

/// The size of the hash output in bytes.
pub const H_LENGTH: usize = BlakeTwo256::LENGTH;

/// The file chunk size in bytes. This is the size of the leaf nodes in the Merkle
/// Patricia Trie that is constructed for each file.
/// Each chunk is 1 kB.
pub const FILE_CHUNK_SIZE: u64 = 2u64.pow(10);

/// The number of challenges for a file, depending on the size of the file.
/// For every 512 kB, there is a challenge.
pub const FILE_SIZE_TO_CHALLENGES: u64 = 2u64.pow(25);
