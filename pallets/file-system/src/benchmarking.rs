//! Benchmarking setup for pallet-file-system

// use super::*;

// #[allow(unused)]
// use crate::Pallet as FileSystem;
// use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
// use frame_system::RawOrigin;
// use sp_runtime::testing::H256;

// use crate::types::{FileLocation, Fingerprint, PeerIds, StorageData};

// benchmarks! {
//     issue_storage_request {
//         let s in 0 .. 100;
//         let caller: T::AccountId = whitelisted_caller();
//         let location: FileLocation<T> = Default::default();
//         let fingerprint: Fingerprint<T> = Default::default();
//         let size: StorageData<T> = Default::default();
//         let msp = H256::default();
//         let peer_ids: PeerIds<T> = Default::default();
//     }: _(RawOrigin::Signed(caller), location.clone(), fingerprint, size, msp, peer_ids)
//     verify {
//         assert!(FileSystem::<T>::storage_requests(location).is_some());
//     }

//     // TODO: add benchmarking for `on_idle`
// }

// impl_benchmark_test_suite!(FileSystem, crate::mock::new_test_ext(), crate::mock::Test);
