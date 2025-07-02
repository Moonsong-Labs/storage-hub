use crate::traits::StorageEnableRuntimeConfig;
use frame_system::{self, pallet_prelude::RuntimeCallFor, Provider};
use pallet_bucket_nfts;
use pallet_file_system;
use pallet_payment_streams;
use pallet_proofs_dealer;
use pallet_randomness;
use pallet_storage_providers;

#[derive(Debug)]
pub enum EventsStorageEnable<Runtime: StorageEnableRuntimeConfig> {
    FileSystem(pallet_file_system::Event<Runtime>),
    Providers(pallet_storage_providers::Event<Runtime>),
    PaymentStreams(pallet_payment_streams::Event<Runtime>),
    ProofsDealer(pallet_proofs_dealer::Event<Runtime>),
    BucketNfts(pallet_bucket_nfts::Event<Runtime>),
    System(frame_system::Event<Runtime>),
    Randomness(pallet_randomness::Event<Runtime>),
    Others,
}

// storage_hub_runtime::RuntimeEvent -> EventsStorageEnable
// impl Into<EventsStorageEnable<storage_hub_runtime::Runtime>> for storage_hub_runtime::RuntimeEvent {
//     into {

//         match

//     }
// }

// struct StorageEnableRuntimeCall;
// impl<Runtime: StorageEnableRuntimeConfig> Into<<Runtime as frame_system::Config>::RuntimeCall>
//     for StorageEnableRuntimeCall
// {
//     fn into(self) -> <Runtime as frame_system::Config>::RuntimeCall {}
// }

// impl<Runtime: StorageEnableRuntimeConfig> Into<<Runtime as frame_system::Config>::RuntimeCall>
//     for pallet_storage_providers::Call<Runtime>
// {
//     fn into(self) -> <Runtime as frame_system::Config>::RuntimeCall {
//         <Runtime as frame_system::Config>::RuntimeCall::from(self)
//     }
// }
