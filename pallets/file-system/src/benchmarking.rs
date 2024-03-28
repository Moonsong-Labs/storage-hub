//! Benchmarking setup for pallet-file-system

use super::*;

#[allow(unused)]
use crate::Pallet as FileSystem;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;

use crate::types::{FileLocation, Fingerprint, MultiAddresses, StorageData};

benchmarks! {
    issue_storage_request {
        let s in 0 .. 100;
        let caller: T::AccountId = whitelisted_caller();
        let location: FileLocation<T> = Default::default();
        let fingerprint: Fingerprint<T> = Default::default();
        let size: StorageData<T> = Default::default();
        let multiaddresses: MultiAddresses<T> = Default::default();
    }: _(RawOrigin::Signed(caller), location.clone(), fingerprint, size, multiaddresses)
    verify {
        assert!(FileSystem::<T>::storage_requests(location).is_some());
    }

    // TODO: add benchmarking for `on_idle`
}

impl_benchmark_test_suite!(FileSystem, crate::mock::new_test_ext(), crate::mock::Test);
