//! Cryptographic utilities and implementations for StorageHub client.
//!
//! This module provides implementations of the `KeyTypeOperations` trait for different
//! cryptographic schemes used in the StorageHub client.

use crate::traits::KeyTypeOperations;
use sp_core::sr25519;
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    KeyTypeId, MultiSignature,
};

/// Implementation of KeyTypeOperations for MultiSignature with AccountId32.
///
/// This implementation assumes sr25519 as the underlying signature scheme.
/// While MultiSignature can represent multiple signature types (Sr25519, Ed25519, ECDSA),
/// this implementation specifically uses Sr25519 for all operations.
impl KeyTypeOperations for MultiSignature {
    type Public = sr25519::Public;
    type AccountId = <<MultiSignature as Verify>::Signer as IdentifyAccount>::AccountId;

    fn public_keys(keystore: &KeystorePtr, key_type: KeyTypeId) -> Vec<Self::Public> {
        keystore.sr25519_public_keys(key_type)
    }

    fn sign(
        keystore: &KeystorePtr,
        key_type: KeyTypeId,
        public: &Self::Public,
        msg: &[u8],
    ) -> Option<Self> {
        keystore
            .sr25519_sign(key_type, public, msg)
            .ok()
            .flatten()
            .map(MultiSignature::Sr25519)
    }

    fn to_runtime_signature(self) -> polkadot_primitives::Signature {
        match self {
            MultiSignature::Sr25519(sig) => polkadot_primitives::Signature::Sr25519(sig),
            MultiSignature::Ed25519(sig) => polkadot_primitives::Signature::Ed25519(sig),
            MultiSignature::Ecdsa(sig) => polkadot_primitives::Signature::Ecdsa(sig),
        }
    }

    fn public_to_account_id(public: &Self::Public) -> Self::AccountId {
        (*public).into()
    }
}
