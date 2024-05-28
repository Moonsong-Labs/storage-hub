//! Benchmarking setup for pallet-file-system

use super::*;

#[allow(unused)]
use crate::Pallet as BucketNfts;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;

benchmarks! {
    share_access {
        let s in 0 .. 100;
        let caller: T::AccountId = whitelisted_caller();
    }: _(RawOrigin::Signed(caller), account)
    verify {
        // assert!(BucketNfts::<T>::share_access().is_some());
    }
}

impl_benchmark_test_suite!(BucketNfts, crate::mock::new_test_ext(), crate::mock::Test);
