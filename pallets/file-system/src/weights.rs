


//! Autogenerated weights for `pallet_file_system`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 43.0.0
//! DATE: 2024-12-17, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `snowmead-msl.local`, CPU: `<UNKNOWN>`
//! WASM-EXECUTION: `Compiled`, CHAIN: `None`, DB CACHE: `1024`

// Executed Command:
// frame-omni-bencher
// v1
// benchmark
// pallet
// --runtime
// target/production/wbuild/storage-hub-runtime/storage_hub_runtime.wasm
// --pallet
// pallet-file-system
// --extrinsic
// 
// --output
// pallets/file-system/src/weights.rs
// --template
// ./frame-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for `pallet_file_system`.
pub trait WeightInfo {
    fn create_bucket() -> Weight;
    fn issue_storage_request() -> Weight;
}

/// Weights for `pallet_file_system` using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    /// Storage: `Providers::MainStorageProviders` (r:1 w:0)
    /// Proof: `Providers::MainStorageProviders` (`max_values`: None, `max_size`: Some(647), added: 3122, mode: `MaxEncodedLen`)
    /// Storage: `Nfts::NextCollectionId` (r:1 w:1)
    /// Proof: `Nfts::NextCollectionId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
    /// Storage: `Nfts::Collection` (r:1 w:1)
    /// Proof: `Nfts::Collection` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
    /// Storage: `System::Account` (r:1 w:1)
    /// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
    /// Storage: `Providers::Buckets` (r:1 w:1)
    /// Proof: `Providers::Buckets` (`max_values`: None, `max_size`: Some(192), added: 2667, mode: `MaxEncodedLen`)
    /// Storage: `Providers::MainStorageProviderIdsToValuePropositions` (r:1 w:0)
    /// Proof: `Providers::MainStorageProviderIdsToValuePropositions` (`max_values`: None, `max_size`: Some(1123), added: 3598, mode: `MaxEncodedLen`)
    /// Storage: `Balances::Holds` (r:1 w:1)
    /// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
    /// Storage: `PaymentStreams::UsersWithoutFunds` (r:1 w:0)
    /// Proof: `PaymentStreams::UsersWithoutFunds` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
    /// Storage: `PaymentStreams::FixedRatePaymentStreams` (r:1 w:1)
    /// Proof: `PaymentStreams::FixedRatePaymentStreams` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
    /// Storage: `Parameters::Parameters` (r:1 w:0)
    /// Proof: `Parameters::Parameters` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
    /// Storage: `Providers::BackupStorageProviders` (r:1 w:0)
    /// Proof: `Providers::BackupStorageProviders` (`max_values`: None, `max_size`: Some(683), added: 3158, mode: `MaxEncodedLen`)
    /// Storage: `PaymentStreams::RegisteredUsers` (r:1 w:1)
    /// Proof: `PaymentStreams::RegisteredUsers` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
    /// Storage: `PaymentStreams::OnPollTicker` (r:1 w:0)
    /// Proof: `PaymentStreams::OnPollTicker` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
    /// Storage: `Providers::MainStorageProviderIdsToBuckets` (r:0 w:1)
    /// Proof: `Providers::MainStorageProviderIdsToBuckets` (`max_values`: None, `max_size`: Some(96), added: 2571, mode: `MaxEncodedLen`)
    /// Storage: `Nfts::CollectionRoleOf` (r:0 w:1)
    /// Proof: `Nfts::CollectionRoleOf` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
    /// Storage: `Nfts::CollectionConfigOf` (r:0 w:1)
    /// Proof: `Nfts::CollectionConfigOf` (`max_values`: None, `max_size`: Some(73), added: 2548, mode: `MaxEncodedLen`)
    /// Storage: `Nfts::CollectionAccount` (r:0 w:1)
    /// Proof: `Nfts::CollectionAccount` (`max_values`: None, `max_size`: Some(68), added: 2543, mode: `MaxEncodedLen`)
    fn create_bucket() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `1568`
        //  Estimated: `4588`
        // Minimum execution time: 131_000_000 picoseconds.
        Weight::from_parts(133_000_000, 4588)
            .saturating_add(T::DbWeight::get().reads(13_u64))
            .saturating_add(T::DbWeight::get().writes(11_u64))
    }
    /// Storage: `Providers::Buckets` (r:1 w:0)
    /// Proof: `Providers::Buckets` (`max_values`: None, `max_size`: Some(192), added: 2667, mode: `MaxEncodedLen`)
    /// Storage: `FileSystem::PendingBucketsToMove` (r:1 w:0)
    /// Proof: `FileSystem::PendingBucketsToMove` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
    /// Storage: `System::Account` (r:1 w:1)
    /// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
    /// Storage: `Balances::Holds` (r:1 w:1)
    /// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
    /// Storage: `Providers::MainStorageProviders` (r:1 w:0)
    /// Proof: `Providers::MainStorageProviders` (`max_values`: None, `max_size`: Some(647), added: 3122, mode: `MaxEncodedLen`)
    /// Storage: `FileSystem::ReplicationTarget` (r:1 w:0)
    /// Proof: `FileSystem::ReplicationTarget` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
    /// Storage: `ProofsDealer::ChallengesTicker` (r:1 w:0)
    /// Proof: `ProofsDealer::ChallengesTicker` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
    /// Storage: `FileSystem::StorageRequests` (r:1 w:1)
    /// Proof: `FileSystem::StorageRequests` (`max_values`: None, `max_size`: Some(1227), added: 3702, mode: `MaxEncodedLen`)
    /// Storage: `FileSystem::NextAvailableStorageRequestExpirationBlock` (r:1 w:1)
    /// Proof: `FileSystem::NextAvailableStorageRequestExpirationBlock` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
    /// Storage: `FileSystem::StorageRequestExpirations` (r:1 w:1)
    /// Proof: `FileSystem::StorageRequestExpirations` (`max_values`: None, `max_size`: Some(3222), added: 5697, mode: `MaxEncodedLen`)
    /// Storage: `FileSystem::BucketsWithStorageRequests` (r:0 w:1)
    /// Proof: `FileSystem::BucketsWithStorageRequests` (`max_values`: None, `max_size`: Some(96), added: 2571, mode: `MaxEncodedLen`)
    fn issue_storage_request() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `865`
        //  Estimated: `6687`
        // Minimum execution time: 74_000_000 picoseconds.
        Weight::from_parts(76_000_000, 6687)
            .saturating_add(T::DbWeight::get().reads(10_u64))
            .saturating_add(T::DbWeight::get().writes(6_u64))
    }
}

// For backwards compatibility and tests.
impl WeightInfo for () {
    /// Storage: `Providers::MainStorageProviders` (r:1 w:0)
    /// Proof: `Providers::MainStorageProviders` (`max_values`: None, `max_size`: Some(647), added: 3122, mode: `MaxEncodedLen`)
    /// Storage: `Nfts::NextCollectionId` (r:1 w:1)
    /// Proof: `Nfts::NextCollectionId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
    /// Storage: `Nfts::Collection` (r:1 w:1)
    /// Proof: `Nfts::Collection` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
    /// Storage: `System::Account` (r:1 w:1)
    /// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
    /// Storage: `Providers::Buckets` (r:1 w:1)
    /// Proof: `Providers::Buckets` (`max_values`: None, `max_size`: Some(192), added: 2667, mode: `MaxEncodedLen`)
    /// Storage: `Providers::MainStorageProviderIdsToValuePropositions` (r:1 w:0)
    /// Proof: `Providers::MainStorageProviderIdsToValuePropositions` (`max_values`: None, `max_size`: Some(1123), added: 3598, mode: `MaxEncodedLen`)
    /// Storage: `Balances::Holds` (r:1 w:1)
    /// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
    /// Storage: `PaymentStreams::UsersWithoutFunds` (r:1 w:0)
    /// Proof: `PaymentStreams::UsersWithoutFunds` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
    /// Storage: `PaymentStreams::FixedRatePaymentStreams` (r:1 w:1)
    /// Proof: `PaymentStreams::FixedRatePaymentStreams` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
    /// Storage: `Parameters::Parameters` (r:1 w:0)
    /// Proof: `Parameters::Parameters` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
    /// Storage: `Providers::BackupStorageProviders` (r:1 w:0)
    /// Proof: `Providers::BackupStorageProviders` (`max_values`: None, `max_size`: Some(683), added: 3158, mode: `MaxEncodedLen`)
    /// Storage: `PaymentStreams::RegisteredUsers` (r:1 w:1)
    /// Proof: `PaymentStreams::RegisteredUsers` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
    /// Storage: `PaymentStreams::OnPollTicker` (r:1 w:0)
    /// Proof: `PaymentStreams::OnPollTicker` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
    /// Storage: `Providers::MainStorageProviderIdsToBuckets` (r:0 w:1)
    /// Proof: `Providers::MainStorageProviderIdsToBuckets` (`max_values`: None, `max_size`: Some(96), added: 2571, mode: `MaxEncodedLen`)
    /// Storage: `Nfts::CollectionRoleOf` (r:0 w:1)
    /// Proof: `Nfts::CollectionRoleOf` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
    /// Storage: `Nfts::CollectionConfigOf` (r:0 w:1)
    /// Proof: `Nfts::CollectionConfigOf` (`max_values`: None, `max_size`: Some(73), added: 2548, mode: `MaxEncodedLen`)
    /// Storage: `Nfts::CollectionAccount` (r:0 w:1)
    /// Proof: `Nfts::CollectionAccount` (`max_values`: None, `max_size`: Some(68), added: 2543, mode: `MaxEncodedLen`)
    fn create_bucket() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `1568`
        //  Estimated: `4588`
        // Minimum execution time: 131_000_000 picoseconds.
        Weight::from_parts(133_000_000, 4588)
            .saturating_add(RocksDbWeight::get().reads(13_u64))
            .saturating_add(RocksDbWeight::get().writes(11_u64))
    }
    /// Storage: `Providers::Buckets` (r:1 w:0)
    /// Proof: `Providers::Buckets` (`max_values`: None, `max_size`: Some(192), added: 2667, mode: `MaxEncodedLen`)
    /// Storage: `FileSystem::PendingBucketsToMove` (r:1 w:0)
    /// Proof: `FileSystem::PendingBucketsToMove` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
    /// Storage: `System::Account` (r:1 w:1)
    /// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
    /// Storage: `Balances::Holds` (r:1 w:1)
    /// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
    /// Storage: `Providers::MainStorageProviders` (r:1 w:0)
    /// Proof: `Providers::MainStorageProviders` (`max_values`: None, `max_size`: Some(647), added: 3122, mode: `MaxEncodedLen`)
    /// Storage: `FileSystem::ReplicationTarget` (r:1 w:0)
    /// Proof: `FileSystem::ReplicationTarget` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
    /// Storage: `ProofsDealer::ChallengesTicker` (r:1 w:0)
    /// Proof: `ProofsDealer::ChallengesTicker` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
    /// Storage: `FileSystem::StorageRequests` (r:1 w:1)
    /// Proof: `FileSystem::StorageRequests` (`max_values`: None, `max_size`: Some(1227), added: 3702, mode: `MaxEncodedLen`)
    /// Storage: `FileSystem::NextAvailableStorageRequestExpirationBlock` (r:1 w:1)
    /// Proof: `FileSystem::NextAvailableStorageRequestExpirationBlock` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
    /// Storage: `FileSystem::StorageRequestExpirations` (r:1 w:1)
    /// Proof: `FileSystem::StorageRequestExpirations` (`max_values`: None, `max_size`: Some(3222), added: 5697, mode: `MaxEncodedLen`)
    /// Storage: `FileSystem::BucketsWithStorageRequests` (r:0 w:1)
    /// Proof: `FileSystem::BucketsWithStorageRequests` (`max_values`: None, `max_size`: Some(96), added: 2571, mode: `MaxEncodedLen`)
    fn issue_storage_request() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `865`
        //  Estimated: `6687`
        // Minimum execution time: 74_000_000 picoseconds.
        Weight::from_parts(76_000_000, 6687)
            .saturating_add(RocksDbWeight::get().reads(10_u64))
            .saturating_add(RocksDbWeight::get().writes(6_u64))
    }
}
