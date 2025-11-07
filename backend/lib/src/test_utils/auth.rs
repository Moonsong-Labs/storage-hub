//! Test utilities for authentication
//!
//! This module provides common utilities for testing authentication functionality

use alloy_core::primitives::{eip191_hash_message, Address};
use alloy_signer::{k256::ecdsa::SigningKey, utils::public_key_to_address};

/// Generate a random ETH wallet
///
/// Returns the corresponding address and signing key
pub fn eth_wallet() -> (Address, SigningKey) {
    let signing_key = SigningKey::random(&mut rand::thread_rng());
    let verifying_key = signing_key.verifying_key();
    let address = public_key_to_address(verifying_key);

    (address, signing_key)
}

/// Sign a message using EIP-191 personal_sign format
pub fn sign_message(signing_key: &SigningKey, message: &str) -> String {
    let message_hash = eip191_hash_message(message.as_bytes());
    let (sig, recovery_id) = signing_key
        .sign_prehash_recoverable(&message_hash.0)
        .unwrap();

    let mut sig_bytes = [0u8; 65];
    sig_bytes[..64].copy_from_slice(&sig.to_bytes());
    sig_bytes[64] = recovery_id.to_byte();

    format!("0x{}", hex::encode(sig_bytes))
}
