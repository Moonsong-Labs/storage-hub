//! Benchmarking setup for pallet-file-system

use super::*;

#[allow(unused)]
use crate::Pallet as FileSystem;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;

benchmarks! {
<<<<<<<< HEAD:storage-hub-node/pallets/file-system/src/benchmarking.rs
	request_storage {
		let s in 0 .. 100;
		let caller: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Signed(caller), s)
	verify {
		assert_eq!(Something::<T>::get(), Some(s));
	}
========
    do_something {
        let s in 0 .. 100;
        let caller: T::AccountId = whitelisted_caller();
    }: _(RawOrigin::Signed(caller), s)
    verify {
        assert_eq!(Something::<T>::get(), Some(s));
    }
>>>>>>>> main:pallets/template/src/benchmarking.rs
}

impl_benchmark_test_suite!(FileSystem, crate::mock::new_test_ext(), crate::mock::Test,);
