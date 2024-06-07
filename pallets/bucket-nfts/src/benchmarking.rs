//! Benchmarking setup for pallet-file-system

use super::*;

#[allow(unused)]
use crate::Pallet as BucketNfts;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use pallet_nfts::BenchmarkHelper as NftsBenchmarkHelper;
use sp_core::H256;
use sp_runtime::traits::StaticLookup;
use types::{BucketIdFor, ReadAccessRegex};

use crate::pallet::BenchmarkHelper as BucketNftsBenchmarkHelper;

benchmarks! {
    share_access {
        let s in 0 .. 100;
        let caller: T::AccountId = whitelisted_caller();
        let recipient = T::Lookup::unlookup(caller.clone());
        let bucket: BucketIdFor<T> = <T as crate::Config>::Helper::bucket(H256::default());
        let item_id: T::ItemId = <T as pallet_nfts::Config>::Helper::item(0);
        let read_access: ReadAccessRegex<T> = sp_runtime::BoundedVec::<u8, <T as pallet_nfts::Config>::StringLimit>::default();
    }: _(RawOrigin::Signed(caller), recipient, bucket, item_id, Some(read_access))
    verify {
        // assert!(BucketNfts::<T>::read_access((bucket, item_id)).is_some());
    }
}

impl_benchmark_test_suite!(BucketNfts, crate::mock::new_test_ext(), crate::mock::Test);
