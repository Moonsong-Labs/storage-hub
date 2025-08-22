#![cfg_attr(not(feature = "std"), no_std)]

use sp_core::H256;
use sp_runtime::traits::BlakeTwo256;
use sp_trie::LayoutV1;

/// A hash of some data used by the chain.
pub type Hash = H256;

/// The hashing algorithm used.
pub type Hashing = BlakeTwo256;

/// The layout of the storage proofs merkle trie.
pub type StorageProofsMerkleTrieLayout = LayoutV1<BlakeTwo256>;

/// Type representing the storage data units in StorageHub.
pub type StorageDataUnit = u64;
