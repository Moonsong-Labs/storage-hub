//! StorageHub client trait implementations for the solochain-evm runtime.
//!
//! This module implements the [`StorageEnableRuntime`] and related traits for the concrete
//! solochain-evm runtime types. It is only compiled for native (std) builds to avoid pulling
//! `shc-common` into the no_std Wasm runtime.

use codec::Decode;
use frame_support::traits::PalletInfoAccess;
use shc_common::{
    traits::{ExtensionOperations, StorageEnableRuntime},
    types::{MinimalExtension, StorageEnableErrors, StorageEnableEvents},
};

/// Implementation of [`StorageEnableRuntime`] for the solochain-evm runtime.
impl StorageEnableRuntime for crate::Runtime {
    type Address = crate::Address;
    type Call = crate::RuntimeCall;
    type Signature = crate::Signature;
    type Extension = crate::TxExtension;
    type RuntimeApi = crate::RuntimeApi;
    type ModuleError = RuntimeModuleError;
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

/// Wrapper type for converting [`sp_runtime::ModuleError`] into [`StorageEnableErrors`].
pub struct RuntimeModuleError(pub sp_runtime::ModuleError);

impl From<sp_runtime::ModuleError> for RuntimeModuleError {
    fn from(err: sp_runtime::ModuleError) -> Self {
        Self(err)
    }
}

impl Into<StorageEnableErrors<crate::Runtime>> for RuntimeModuleError {
    fn into(self) -> StorageEnableErrors<crate::Runtime> {
        let module_error = self.0;
        let system_index = frame_system::Pallet::<crate::Runtime>::index() as u8;
        let providers_index = pallet_storage_providers::Pallet::<crate::Runtime>::index() as u8;
        let proofs_dealer_index = pallet_proofs_dealer::Pallet::<crate::Runtime>::index() as u8;
        let payment_streams_index = pallet_payment_streams::Pallet::<crate::Runtime>::index() as u8;
        let file_system_index = pallet_file_system::Pallet::<crate::Runtime>::index() as u8;
        let balances_index = pallet_balances::Pallet::<crate::Runtime>::index() as u8;
        let bucket_nfts_index = pallet_bucket_nfts::Pallet::<crate::Runtime>::index() as u8;

        match module_error.index {
            i if i == system_index => {
                frame_system::Error::<crate::Runtime>::decode(&mut &module_error.error[..])
                    .map(StorageEnableErrors::System)
                    .unwrap_or_else(|_| StorageEnableErrors::Other(module_error))
            }
            i if i == providers_index => pallet_storage_providers::Error::<crate::Runtime>::decode(
                &mut &module_error.error[..],
            )
            .map(StorageEnableErrors::StorageProviders)
            .unwrap_or_else(|_| StorageEnableErrors::Other(module_error)),
            i if i == proofs_dealer_index => {
                pallet_proofs_dealer::Error::<crate::Runtime>::decode(&mut &module_error.error[..])
                    .map(StorageEnableErrors::ProofsDealer)
                    .unwrap_or_else(|_| StorageEnableErrors::Other(module_error))
            }
            i if i == payment_streams_index => {
                pallet_payment_streams::Error::<crate::Runtime>::decode(
                    &mut &module_error.error[..],
                )
                .map(StorageEnableErrors::PaymentStreams)
                .unwrap_or_else(|_| StorageEnableErrors::Other(module_error))
            }
            i if i == file_system_index => {
                pallet_file_system::Error::<crate::Runtime>::decode(&mut &module_error.error[..])
                    .map(StorageEnableErrors::FileSystem)
                    .unwrap_or_else(|_| StorageEnableErrors::Other(module_error))
            }
            i if i == balances_index => {
                pallet_balances::Error::<crate::Runtime>::decode(&mut &module_error.error[..])
                    .map(StorageEnableErrors::Balances)
                    .unwrap_or_else(|_| StorageEnableErrors::Other(module_error))
            }
            i if i == bucket_nfts_index => {
                pallet_bucket_nfts::Error::<crate::Runtime>::decode(&mut &module_error.error[..])
                    .map(StorageEnableErrors::BucketNfts)
                    .unwrap_or_else(|_| StorageEnableErrors::Other(module_error))
            }
            _ => StorageEnableErrors::Other(module_error),
        }
    }
}
