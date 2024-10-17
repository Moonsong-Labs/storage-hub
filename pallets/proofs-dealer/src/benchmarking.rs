//! Benchmarking setup for pallet-proofs-dealer
use frame_benchmarking::v2::*;

#[benchmarks]
mod benchmarks {
    use frame_system::RawOrigin;
    use sp_runtime::traits::Hash;

    use super::*;
    use crate::{
        // mock::{new_test_ext, Test},
        types::MerkleTrieHashingFor,
        Call,
        Config,
        Pallet,
    };

    // Benchmark `some_extrinsic` extrinsic with the worst possible conditions:
    // * Worst possible condition 1.
    // * Worst possible condition 2.
    #[benchmark]
    fn challenge_some_case() {
        // Setup initial conditions.
        let caller: T::AccountId = whitelisted_caller();
        let file_key = MerkleTrieHashingFor::<T>::hash(b"file_key");

        // Call some extrinsic.
        #[extrinsic_call]
        Pallet::challenge(RawOrigin::Signed(caller), file_key);

        // Verify the result.
        // assert_eq!(value, expected);
    }

    impl_benchmark_test_suite! {
            Pallet,
            crate::mock::new_test_ext(),
            crate::mock::Test,
    }
}
