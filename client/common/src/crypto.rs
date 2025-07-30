//! Cryptographic utilities and implementations for StorageHub client.
//!
//! This module provides implementations of the `KeyTypeOperations` trait for different
//! cryptographic schemes used in the StorageHub client.

use crate::traits::KeyTypeOperations;
use sp_core::{sr25519, ecdsa, Hasher};
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::{AccountId32, KeyTypeId};

/// Implementation of KeyTypeOperations for sr25519 cryptographic scheme.
///
/// Sr25519 is the default signature scheme used in Substrate/Polkadot ecosystems.
/// Public keys are 32 bytes and can be directly converted to AccountId32.
impl KeyTypeOperations for sr25519::Pair {
    type Public = sr25519::Public;
    type Signature = sr25519::Signature;

    fn public_keys(keystore: &KeystorePtr, key_type: KeyTypeId) -> Vec<Self::Public> {
        keystore.sr25519_public_keys(key_type)
    }

    fn sign(
        keystore: &KeystorePtr,
        key_type: KeyTypeId,
        public: &Self::Public,
        msg: &[u8],
    ) -> Option<Self::Signature> {
        keystore.sr25519_sign(key_type, public, msg).ok().flatten()
    }

    fn to_runtime_signature(signature: Self::Signature) -> polkadot_primitives::Signature {
        polkadot_primitives::Signature::Sr25519(signature)
    }

    fn public_to_account_id(public: &Self::Public) -> AccountId32 {
        (*public).into()
    }
}

/// Implementation of KeyTypeOperations for ecdsa cryptographic scheme.
///
/// ECDSA public keys are 33 bytes (compressed format) which doesn't directly map to
/// AccountId32's 32 bytes. Therefore, we hash the public key to derive the account ID.
impl KeyTypeOperations for ecdsa::Pair {
    type Public = ecdsa::Public;
    type Signature = ecdsa::Signature;

    fn public_keys(keystore: &KeystorePtr, key_type: KeyTypeId) -> Vec<Self::Public> {
        keystore.ecdsa_public_keys(key_type)
    }

    fn sign(
        keystore: &KeystorePtr,
        key_type: KeyTypeId,
        public: &Self::Public,
        msg: &[u8],
    ) -> Option<Self::Signature> {
        keystore.ecdsa_sign(key_type, public, msg).ok().flatten()
    }

    fn to_runtime_signature(signature: Self::Signature) -> polkadot_primitives::Signature {
        polkadot_primitives::Signature::Ecdsa(signature)
    }

    fn public_to_account_id(public: &Self::Public) -> AccountId32 {
        // ECDSA public keys are 33 bytes (compressed), but AccountId32 expects 32 bytes
        // We need to hash the public key to get a 32-byte account ID
        let hash = sp_runtime::traits::BlakeTwo256::hash(&public.0);
        AccountId32::new(hash.into())
    }
}