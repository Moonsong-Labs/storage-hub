use trie_db::{Hasher, TrieLayout};

/// The hash type of trie node keys
pub type HashT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;
