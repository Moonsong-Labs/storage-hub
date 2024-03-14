//! Benchmarking setup for pallet-file-system

use super::*;

#[allow(unused)]
use crate::Pallet as FileSystem;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;

use crate::types::{FileLocation, Fingerprint, MultiAddress, StorageUnit};

benchmarks! {
    request_storage {
        let s in 0 .. 100;
        let caller: T::AccountId = whitelisted_caller();
        let location: FileLocation<T> = Default::default();
        let fingerprint: Fingerprint<T> = Default::default();
        let size: StorageUnit<T> = Default::default();
        let user_multiaddr: MultiAddress<T> = Default::default();

        let _ = FileSystem::<T>::create_bucket(RawOrigin::Signed(caller.clone()).into());
    }: _(RawOrigin::Signed(caller), location.clone(), fingerprint, size, user_multiaddr, overwrite)
    verify {
        assert!(FileSystem::<T>::storage_requests(location).is_some());
    }
}

impl_benchmark_test_suite!(FileSystem, crate::mock::new_test_ext(), crate::mock::Test,);
