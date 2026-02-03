//! StorageHub client trait implementations for the solochain-evm runtime.
//!
//! This module implements the [`StorageEnableRuntime`] and related traits for the concrete
//! solochain-evm runtime types. It is only compiled for native (std) builds to avoid pulling
//! `shc-common` into the no_std Wasm runtime.

use shc_common::{
    traits::{ExtensionOperations, StorageEnableRuntime, TransactionHashProvider},
    types::{MinimalExtension, StorageEnableErrors, StorageEnableEvents, StorageHubEventsVec},
};
use sp_core::H256;

/// Implementation of [`StorageEnableRuntime`] for the solochain-evm runtime.
impl StorageEnableRuntime for crate::Runtime {
    type Address = crate::Address;
    type Call = crate::RuntimeCall;
    type Signature = crate::Signature;
    type Extension = crate::TxExtension;
    type RuntimeApi = crate::RuntimeApi;
    type RuntimeError = crate::RuntimeError;
}

// Implement the transaction extension helpers for the concrete runtime's TxExtension.
impl ExtensionOperations<crate::RuntimeCall, crate::Runtime> for crate::TxExtension {
    type Hash = shp_types::Hash;

    fn from_minimal_extension(minimal: MinimalExtension) -> Self {
        let inner = (
            frame_system::CheckNonZeroSender::<crate::Runtime>::new(),
            frame_system::CheckSpecVersion::<crate::Runtime>::new(),
            frame_system::CheckTxVersion::<crate::Runtime>::new(),
            frame_system::CheckGenesis::<crate::Runtime>::new(),
            frame_system::CheckEra::<crate::Runtime>::from(minimal.era),
            frame_system::CheckNonce::<crate::Runtime>::from(minimal.nonce),
            frame_system::CheckWeight::<crate::Runtime>::new(),
            pallet_transaction_payment::ChargeTransactionPayment::<crate::Runtime>::from(
                minimal.tip,
            ),
            frame_metadata_hash_extension::CheckMetadataHash::new(false),
        );
        cumulus_pallet_weight_reclaim::StorageWeightReclaim::new(inner)
    }
}

// Map the runtime event into the client-facing storage events enum.
impl Into<StorageEnableEvents<crate::Runtime>> for crate::RuntimeEvent {
    fn into(self) -> StorageEnableEvents<crate::Runtime> {
        match self {
            crate::RuntimeEvent::System(event) => StorageEnableEvents::System(event),
            crate::RuntimeEvent::Providers(event) => StorageEnableEvents::StorageProviders(event),
            crate::RuntimeEvent::ProofsDealer(event) => StorageEnableEvents::ProofsDealer(event),
            crate::RuntimeEvent::PaymentStreams(event) => {
                StorageEnableEvents::PaymentStreams(event)
            }
            crate::RuntimeEvent::FileSystem(event) => StorageEnableEvents::FileSystem(event),
            crate::RuntimeEvent::TransactionPayment(event) => {
                StorageEnableEvents::TransactionPayment(event)
            }
            crate::RuntimeEvent::Balances(event) => StorageEnableEvents::Balances(event),
            crate::RuntimeEvent::BucketNfts(event) => StorageEnableEvents::BucketNfts(event),
            crate::RuntimeEvent::Randomness(event) => StorageEnableEvents::Randomness(event),
            _ => StorageEnableEvents::Other(self),
        }
    }
}

// Implement transaction hash extraction for EVM runtime.
impl TransactionHashProvider for crate::Runtime {
    fn build_transaction_hash_map(
        all_events: &StorageHubEventsVec<Self>,
    ) -> std::collections::HashMap<u32, H256> {
        let mut tx_map = std::collections::HashMap::new();

        for ev in all_events {
            if let frame_system::Phase::ApplyExtrinsic(extrinsic_index) = ev.phase {
                // Convert to StorageEnableEvents
                let storage_event: StorageEnableEvents<Self> = ev.event.clone().into();

                // Check if it's an `Executed` Ethereum event in the `Other` variant
                if let StorageEnableEvents::Other(runtime_event) = storage_event {
                    // If it is, extract the Ethereum transaction hash from it
                    if let crate::RuntimeEvent::Ethereum(pallet_ethereum::Event::Executed {
                        transaction_hash,
                        ..
                    }) = runtime_event
                    {
                        tx_map.insert(extrinsic_index, transaction_hash);
                    }
                }
            }
        }

        tx_map
    }
}

// Map the runtime error into the client-facing storage errors enum.
impl Into<StorageEnableErrors<crate::Runtime>> for crate::RuntimeError {
    fn into(self) -> StorageEnableErrors<crate::Runtime> {
        match self {
            crate::RuntimeError::System(error) => StorageEnableErrors::System(error),
            crate::RuntimeError::Providers(error) => StorageEnableErrors::StorageProviders(error),
            crate::RuntimeError::ProofsDealer(error) => StorageEnableErrors::ProofsDealer(error),
            crate::RuntimeError::PaymentStreams(error) => {
                StorageEnableErrors::PaymentStreams(error)
            }
            crate::RuntimeError::FileSystem(error) => StorageEnableErrors::FileSystem(error),
            crate::RuntimeError::Balances(error) => StorageEnableErrors::Balances(error),
            crate::RuntimeError::BucketNfts(error) => StorageEnableErrors::BucketNfts(error),
            other => StorageEnableErrors::Other(format!("{:?}", other)),
        }
    }
}
