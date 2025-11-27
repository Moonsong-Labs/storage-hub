// This module implements the StorageHub client traits for the runtime types.
// It is only compiled for native (std) builds to avoid pulling `shc-common` into the
// no_std Wasm runtime.
use shc_common::{
    traits::{ExtensionOperations, StorageEnableRuntime},
    types::{MinimalExtension, StorageEnableEvents},
};

// Implement the client-facing runtime trait for the concrete runtime.
impl StorageEnableRuntime for crate::Runtime {
    type Address = crate::Address;
    type Call = crate::RuntimeCall;
    type Signature = crate::Signature;
    type Extension = crate::TxExtension;
    type RuntimeApi = crate::RuntimeApi;
}

// Implement the transaction extension helpers for the concrete runtime's TxExtension.
impl ExtensionOperations<crate::RuntimeCall, crate::Runtime> for crate::TxExtension {
    type Hash = shp_types::Hash;

    fn from_minimal_extension(minimal: MinimalExtension) -> Self {
        (
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
        )
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
impl shc_common::traits::TransactionHashProvider for crate::Runtime {
    fn build_transaction_hash_map(
        all_events: &shc_common::types::StorageHubEventsVec<Self>,
    ) -> std::collections::HashMap<u32, sp_core::H256> {
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
