//! Cryptographic utilities and implementations for StorageHub client.
//!
//! This module provides implementations of the `KeyTypeOperations` trait for different
//! cryptographic schemes used in the StorageHub client.

use codec::{Decode, Encode};
use fp_account::{AccountId20, EthereumSignature};
use sp_core::{ecdsa, keccak_256, sr25519, H160, H256};
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    KeyTypeId, MultiSignature,
};

use crate::traits::KeyTypeOperations;

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

/// Implementation of KeyTypeOperations for EthereumSignature with AccountId20.
///
/// This implementation uses ECDSA keys and signatures in the Ethereum format.
/// The AccountId is the same as the Ethereum address. That is, the last 20 bytes
/// of the keccak256 hash of the public key.
impl KeyTypeOperations for EthereumSignature {
    type Public = ecdsa::Public;
    type AccountId = <<EthereumSignature as Verify>::Signer as IdentifyAccount>::AccountId;

    fn public_keys(keystore: &KeystorePtr, key_type: KeyTypeId) -> Vec<Self::Public> {
        keystore.ecdsa_public_keys(key_type)
    }

    fn sign(
        keystore: &KeystorePtr,
        key_type: KeyTypeId,
        public: &Self::Public,
        msg: &[u8],
    ) -> Option<Self> {
        keystore
            .ecdsa_sign(key_type, public, msg)
            .ok()
            .flatten()
            .map(|ecdsa_sig| EthereumSignature::new(ecdsa_sig))
    }

    fn to_runtime_signature(self) -> polkadot_primitives::Signature {
        //! WARNING: This is a workaround to convert the `EthereumSignature` to a `ecdsa::Signature`.,
        //! by encoding and decoding it. This takes advantage of the fact that the `EthereumSignature`
        //! is just a wrapper around the `ecdsa::Signature`, and SCALE-encoding of a wrapper type is
        //! the same as the SCALE-encoding of the wrapped type.
        //!
        //! This is NOT safe, as it bypasses the type system. A proper solution would be to add a `.inner()`
        //! method to the `EthereumSignature` type, and use that instead.
        let encoded = self.encode();
        let ecdsa_sig = ecdsa::Signature::decode(&mut &encoded[..]).expect(
            "The encoded `EthereumSignature` is just a wrapper around the `ecdsa::Signature`, so decoding it should always succeed",
        );
        polkadot_primitives::Signature::Ecdsa(ecdsa_sig)
    }

    fn public_to_account_id(public: &Self::Public) -> Self::AccountId {
        AccountId20(H160::from(H256::from(keccak_256(&public))).0)
    }
}
